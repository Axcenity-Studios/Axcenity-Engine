//! Non-destructive garment fitting (ADR-0046):
//!
//! 1. **Immutable rest geometry:** [`fit_garment`] never mutates the canonical
//!    garment's rest vertex positions or topology (no rest push-out, no extreme
//!    rest-lift, no web-cutting).
//! 2. **Fit/Skirt Proxy binding:** binds the garment to a smooth fixed-topology
//!    [`FitProxy`] on its [`HomeFrame`] with part restriction and confidence
//!    gating (`<= 0.05 * bbox_diag`, `<= 35 deg`).
//! 3. **Two-stage biharmonic skin-weight inpainting:** copies confident proxy
//!    weights and solves unbound regions (armpits, collars, hems, loose cloth)
//!    via `faer` sparse Cholesky ([`super::inpaint`]).
//! 4. **Frame-relative biharmonic morph transfer:** transfers body identity and
//!    corrective shapes from the proxy on confident vertices, and inpaints
//!    unbound residuals relative to each vertex's smooth local proxy
//!    displacement so free-hanging hems expand naturally without flattening or
//!    lever-arm spikes.
//! 5. **Non-destructive `Shrink_Under` layering morph:** generates one optional
//!    morph target pulling vertices inward toward Layer Envelope `layer - 1`,
//!    activated live by [`crate::resolve`] only when an overlapping higher-layer
//!    garment is worn.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::bind::{BIND_REACH, Binding, VertexBind, bind_proxy};
use super::inpaint::{BiharmonicSolver, inpaint_weights};
use super::pose::FitPose;
use super::posed::{self, PoseReport};
use super::proxy::{FitProxy, ProxyKind, envelope_offset};
use super::validate::{ValidationReport, validate_garment};
use super::{BodyMesh, FitError, GarmentMesh, is_mask_shape};
use crate::BaseManifest;

/// Name of the non-destructive inner-layer shrink morph target activated by
/// [`crate::resolve`] when an overlapping higher-layer garment is equipped.
pub const SHRINK_UNDER_SHAPE: &str = "Shrink_Under";
/// Default minimum morph reach (metres) retained in the fitted garment (0.5 mm).
pub const DEFAULT_PRUNE_BELOW: f32 = 0.0005;
/// Maximum inward displacement for `Shrink_Under` (metres) so thin garments are
/// not crushed.
const SHRINK_UNDER_MAX: f32 = 0.025;
/// Smoothing rounds applied to `Shrink_Under` so layer transitions are seamless.
const SHRINK_SMOOTH_ROUNDS: usize = 6;
/// Shape threshold below which a body shape is skipped.
const LEAST: f32 = 1e-5;

/// Home frame a garment was authored for (no cross-frame rest warping).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HomeFrame {
    #[default]
    Masculine,
    Feminine,
    /// Only for body-topology items authored natively on both frames.
    Both,
}

impl HomeFrame {
    /// Body shape weights representing this home frame's rest state.
    pub fn rest_weights(self) -> Vec<(String, f32)> {
        match self {
            HomeFrame::Feminine => vec![("Frame_Feminine".to_owned(), 1.0)],
            HomeFrame::Masculine | HomeFrame::Both => Vec::new(),
        }
    }
}

/// Options for non-destructive [`fit_garment`].
#[derive(Clone, Debug)]
pub struct FitOptions {
    /// Target clearance margin (metres) used for opening chord-sag validation.
    pub margin: f32,
    /// Garment wearing layer (`0..=3`), used for Layer Envelope `Shrink_Under`.
    pub layer: u8,
    /// Home frame the garment was authored on (no cross-frame rest warping).
    pub home_frame: HomeFrame,
    /// Proxy surface to bind against (`Body` or `Skirt`).
    pub proxy_kind: ProxyKind,
    /// When `true`, excludes `LeftArm` and `RightArm` during part assignment so
    /// sleeveless garments bind strictly to `Trunk` and never flare into wings.
    pub sleeveless: bool,
    /// Body shapes to transfer (usually [`carried_shapes`]).
    pub shapes: Vec<String>,
    /// Poses for posed corrective refinement (`posed::solve`). Empty: skip.
    pub poses: Vec<FitPose>,
    pub pose_states: Vec<Vec<(String, f32)>>,
    /// Generates non-destructive `Shrink_Under` morph toward Layer Envelope
    /// `layer - 1` (when `layer > 0`).
    pub generate_shrink_under: bool,
    /// Prunes carried morph targets whose maximum displacement is below this
    /// threshold (metres; default 0.5 mm). `0.0` keeps all shapes.
    pub prune_below: f32,
}

impl Default for FitOptions {
    fn default() -> Self {
        FitOptions {
            margin: 0.002,
            layer: 1,
            home_frame: HomeFrame::Masculine,
            proxy_kind: ProxyKind::Body,
            sleeveless: false,
            shapes: Vec::new(),
            poses: Vec::new(),
            pose_states: Vec::new(),
            generate_shrink_under: true,
            prune_below: DEFAULT_PRUNE_BELOW,
        }
    }
}

/// A lower layer reference (retained for API compatibility).
#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    pub shapes: BTreeMap<String, Vec<Vec3>>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
}

impl Layer {
    pub fn of_fitted(name: &str, fitted: &Fitted) -> Layer {
        Layer {
            name: name.to_owned(),
            positions: fitted.garment.positions(),
            normals: fitted.garment.normals(),
            triangles: fitted.garment.triangles(),
            shapes: fitted
                .shapes
                .iter()
                .map(|shape| (shape.name.clone(), shape.offsets.clone()))
                .collect(),
            joints: fitted.joints.clone(),
            weights: fitted.weights.clone(),
        }
    }

    pub fn of_body_mesh(name: &str, mesh: &BodyMesh) -> Layer {
        Layer {
            name: name.to_owned(),
            positions: mesh.positions.clone(),
            normals: mesh.normals.clone(),
            triangles: mesh.triangles.clone(),
            shapes: mesh
                .shapes
                .iter()
                .map(|shape| (shape.name.clone(), shape.dense(mesh.positions.len())))
                .collect(),
            joints: mesh.joints.clone(),
            weights: mesh.weights.clone(),
        }
    }
}

/// One shape a garment carries: its offsets and normal changes, one per
/// corner (as [`GarmentMesh::positions`] orders them), at full weight.
#[derive(Clone, Debug, PartialEq)]
pub struct CarriedShape {
    pub name: String,
    pub offsets: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

impl CarriedShape {
    pub fn reach(&self) -> f32 {
        self.offsets.iter().map(|o| o.length()).fold(0.0, f32::max)
    }
}

/// What fitting measured and produced.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FitReport {
    pub vertices: usize,
    pub triangles: usize,
    pub shapes_carried: usize,
    pub shapes_moving: usize,
    pub shapes_pruned: usize,
    /// Confident vertices bound directly to the FitProxy.
    pub confident_vertices: usize,
    /// Unbound vertices solved via biharmonic inpainting.
    pub inpainted_vertices: usize,
    /// Maximum single-morph displacement across all carried shapes (metres).
    pub max_morph_displacement: f32,
    /// Maximum displacement of `Shrink_Under` (metres; 0 if not generated).
    pub shrink_under_reach: f32,
    /// Read-only canonical source validation metrics.
    pub validation: ValidationReport,
    /// Corners farther than [`BIND_REACH`] from the body.
    pub far: usize,
    /// Largest normal offset from the body (metres).
    pub farthest: f32,
    /// What posing it with the body did ([`FitOptions::poses`]).
    #[serde(default)]
    pub posed: PoseReport,
}

/// A garment fitted to a body: its **immutable** canonical rest mesh, inpainted
/// skin weights, carried shapes (+ optional `Shrink_Under`), binding, and report.
#[derive(Clone, Debug)]
pub struct Fitted {
    pub garment: GarmentMesh,
    /// Joint indices into the body's [`Rig::joints`](axcenity_assets::Rig).
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub shapes: Vec<CarriedShape>,
    pub binding: Binding,
    pub report: FitReport,
}

/// The body shapes a garment on `base` carries: every identity shape (a
/// parameter's or a corner's) and every pose corrective (`Corrective_`),
/// in the body's order, never a mask shape.
pub fn carried_shapes(base: &BaseManifest, body: &BodyMesh) -> Vec<String> {
    let identity: BTreeSet<&str> = base
        .parameters
        .iter()
        .flat_map(|p| p.shapes.iter().map(|d| d.shape.as_str()))
        .chain(base.corners.iter().map(|c| c.shape.as_str()))
        .collect();
    body.shapes
        .iter()
        .map(|shape| shape.name.as_str())
        .filter(|name| !is_mask_shape(name))
        .filter(|name| identity.contains(name) || name.starts_with("Corrective_"))
        .map(str::to_owned)
        .collect()
}

/// Direct barycentric skin-weight copy from a raw [`Binding`] (retained for
/// tests and diagnostics).
pub fn transfer_weights(body: &BodyMesh, binding: &Binding) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
    let mut joints = Vec::with_capacity(binding.vertices.len());
    let mut weights = Vec::with_capacity(binding.vertices.len());
    for bound in &binding.vertices {
        let corners = body.triangles[bound.triangle as usize];
        let mut mix: BTreeMap<u16, f32> = BTreeMap::new();
        for (corner, share) in corners.iter().zip(bound.bary) {
            let corner = *corner as usize;
            for (joint, weight) in body.joints[corner].iter().zip(body.weights[corner]) {
                if weight > 0.0 {
                    *mix.entry(*joint).or_default() += weight * share;
                }
            }
        }
        let mut sorted: Vec<(u16, f32)> = mix.into_iter().collect();
        sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        sorted.truncate(4);
        let total: f32 = sorted.iter().map(|(_, w)| w).sum();
        let mut j = [0u16; 4];
        let mut w = [0.0f32; 4];
        if total > 0.0 {
            for (k, (joint, weight)) in sorted.into_iter().enumerate() {
                j[k] = joint;
                w[k] = weight / total;
            }
        }
        joints.push(j);
        weights.push(w);
    }
    (joints, weights)
}

/// Non-destructively fits `garment` (canonical A-pose single-shell mesh in
/// `body`'s file space) to `body` via [`FitProxy`] and `faer` biharmonic
/// inpainting. Never mutates `garment` rest vertex positions.
pub fn fit_garment(
    body: &BodyMesh,
    garment: &GarmentMesh,
    options: &FitOptions,
) -> Result<Fitted, FitError> {
    let triangles = garment.triangles();
    if triangles.is_empty() {
        return Err(FitError::Empty);
    }
    let count = garment.len();
    let rest = garment.positions();
    let rest_normals = garment.normals();
    let validation = validate_garment(garment, options.margin);
    let welds = Welds::new(&rest, &triangles);
    let rest_welded_normals = welds.normals(&rest, &triangles);

    // 1. Build smooth FitProxy (Body or Skirt) and evaluate on home frame.
    let home_weights = options.home_frame.rest_weights();
    let proxy = FitProxy::from_body(body, options.proxy_kind);
    let (proxy_rest_pos, _) = proxy.evaluate(body, &home_weights, 0.0);

    // 2. Part-restricted confident proxy binding.
    let proxy_binding = bind_proxy(
        &proxy,
        body,
        &home_weights,
        &rest,
        &rest_normals,
        &triangles,
        options.sleeveless,
    );
    let confident_mask = proxy_binding.confident_mask();

    // 3. Build & factor the biharmonic operator Q_uu once via `faer` Cholesky.
    let solver = BiharmonicSolver::new(&rest, &triangles, &confident_mask)?;

    // 4. Two-stage biharmonic skin-weight inpainting.
    let (mut joints, mut weights) = inpaint_weights(&solver, body, &proxy, &proxy_binding);
    if options.sleeveless {
        strip_arm_weights(body, &mut joints, &mut weights);
    }

    // 5. Frame-relative biharmonic morph-target transfer from the FitProxy.
    //    For each carried body shape:
    //    - Skip `Frame_Feminine` when `home_frame == HomeFrame::Feminine` because
    //      the garment's rest mesh is ALREADY authored at `Frame_Feminine = 1.0`
    //      (carrying it would double-apply `Frame_Feminine` at runtime!).
    //    - Compute pure proxy vertex displacements `proxy_shaped - proxy_rest`.
    //    - Compute smooth local `base_deltas` for every garment vertex from its
    //      nearest same-part proxy triangle (pure barycentric translation, no
    //      tangent-frame lever-arm rotation!).
    //    - Confident vertices take exact proxy displacement at their bound point;
    //      unbound vertices (collars, lapels, wide hems) solve the residual
    //      `confident - base` biharmonically and add `base_deltas` back.
    let carried: Vec<(usize, &String)> = options
        .shapes
        .iter()
        .filter(|name| !is_mask_shape(name))
        .filter(|name| {
            !(options.home_frame == HomeFrame::Feminine && name.as_str() == "Frame_Feminine")
        })
        .filter_map(|name| Some((body.shape_index(name)?, name)))
        .collect();

    let worked = par_map(&carried, |&(index, name)| {
        if body.shapes[index].reach() <= LEAST {
            return (name.clone(), Vec::new(), Vec::new());
        }
        let mut shaped_weights = home_weights.clone();
        if let Some(entry) = shaped_weights.iter_mut().find(|(n, _)| n == name) {
            entry.1 += 1.0;
        } else {
            shaped_weights.push((name.clone(), 1.0));
        }
        let (proxy_shaped_pos, _) = proxy.evaluate(body, &shaped_weights, 0.0);
        let proxy_deltas: Vec<Vec3> = proxy_shaped_pos
            .iter()
            .zip(&proxy_rest_pos)
            .map(|(s, r)| *s - *r)
            .collect();

        let mut base_deltas = Vec::with_capacity(count);
        let mut known_deltas = Vec::with_capacity(count);
        for k in 0..count {
            let (ntri, nbary) = proxy_binding.nearest_proxy[k];
            let base = sample_delta(&proxy.triangles, &proxy_deltas, ntri, nbary);
            base_deltas.push(base);
            match proxy_binding.vertices[k] {
                VertexBind::Confident(bound) => {
                    known_deltas.push(sample_delta(
                        &proxy.triangles,
                        &proxy_deltas,
                        bound.triangle,
                        bound.bary,
                    ));
                }
                VertexBind::Unbound => {
                    known_deltas.push(base);
                }
            }
        }

        let offsets = solver.solve_deltas_relative(&known_deltas, &base_deltas);
        let reach = offsets.iter().map(|o| o.length()).fold(0.0f32, f32::max);
        if reach < options.prune_below || reach <= LEAST {
            return (name.clone(), Vec::new(), Vec::new());
        }

        let moved: Vec<Vec3> = rest.iter().zip(&offsets).map(|(p, o)| *p + *o).collect();
        let moved_normals = welds.normals(&moved, &triangles);
        let shape_normals = rotate_normals(&rest_normals, &rest_welded_normals, &moved_normals);
        (name.clone(), offsets, shape_normals)
    });

    let mut shapes = Vec::new();
    let mut shapes_pruned = 0usize;
    let mut max_morph_displacement = 0.0f32;
    for (name, offsets, normals) in worked {
        if offsets.is_empty() {
            shapes_pruned += 1;
            continue;
        }
        let reach = offsets.iter().map(|o| o.length()).fold(0.0f32, f32::max);
        max_morph_displacement = max_morph_displacement.max(reach);
        shapes.push(CarriedShape {
            name,
            offsets,
            normals,
        });
    }

    // 6. Body-space binding for cover maps (`_AXCENITY_SKIN_UV`) and posed solve.
    let body_binding = proxy_binding.to_body_binding(body, &rest);

    // 7. Optional posed corrective refinement (clamped & seam-welded).
    let posed_report = posed::solve(
        body,
        &posed::Garment {
            binding: &body_binding,
            joints: &joints,
            weights: &weights,
            welds: &welds,
            loose: true,
        },
        &options.poses,
        &options.pose_states,
        &rest,
        &mut shapes,
    );

    // 8. Non-destructive `Shrink_Under` morph target toward Layer Envelope `layer - 1`
    //    (or 1 mm skin clearance for Layer 0 tucked tops so they tuck inside Layer 1 bottoms).
    let mut shrink_under_reach = 0.0f32;
    if options.generate_shrink_under {
        let target_offset = if options.layer == 0 {
            0.001
        } else {
            envelope_offset(options.layer - 1)
        };
        let lower_env = proxy.surface(body, &home_weights, target_offset);
        let shrink_offsets = build_shrink_under(&rest, &welds, &lower_env);
        shrink_under_reach = shrink_offsets
            .iter()
            .map(|o| o.length())
            .fold(0.0f32, f32::max);
        if shrink_under_reach > LEAST {
            let moved: Vec<Vec3> = rest
                .iter()
                .zip(&shrink_offsets)
                .map(|(p, o)| *p + *o)
                .collect();
            let moved_normals = welds.normals(&moved, &triangles);
            let shrink_normals =
                rotate_normals(&rest_normals, &rest_welded_normals, &moved_normals);
            shapes.push(CarriedShape {
                name: SHRINK_UNDER_SHAPE.to_owned(),
                offsets: shrink_offsets,
                normals: shrink_normals,
            });
        }
    }

    let farthest = body_binding
        .vertices
        .iter()
        .map(|b| b.local[2].abs())
        .fold(0.0, f32::max);
    let far = body_binding
        .vertices
        .iter()
        .filter(|b| Vec3::from(b.local).length() > BIND_REACH)
        .count();

    let shapes_moving = shapes.len();
    let report = FitReport {
        vertices: count,
        triangles: triangles.len(),
        shapes_carried: shapes.len(),
        shapes_moving,
        shapes_pruned,
        confident_vertices: proxy_binding.confident_count,
        inpainted_vertices: proxy_binding.unbound_count,
        max_morph_displacement,
        shrink_under_reach,
        validation,
        far,
        farthest,
        posed: posed_report,
    };

    // Rest mesh is strictly immutable: clone verbatim without touching positions.
    Ok(Fitted {
        garment: garment.clone(),
        joints,
        weights,
        shapes,
        binding: body_binding,
        report,
    })
}

/// Interpolates a proxy-vertex displacement field at `(triangle, bary)`.
fn sample_delta(triangles: &[[u32; 3]], deltas: &[Vec3], triangle: u32, bary: [f32; 3]) -> Vec3 {
    let [a, b, c] = triangles[triangle as usize].map(|v| v as usize);
    deltas[a] * bary[0] + deltas[b] * bary[1] + deltas[c] * bary[2]
}

/// Computes morph normal deltas from welded rest vs. moved surface normals.
fn rotate_normals(rest_normals: &[Vec3], was_surf: &[Vec3], is_surf: &[Vec3]) -> Vec<Vec3> {
    rest_normals
        .iter()
        .zip(was_surf.iter().zip(is_surf))
        .map(|(&n, (&was, &is))| {
            if was.distance_squared(is) < 1e-12 || was == Vec3::ZERO || is == Vec3::ZERO {
                Vec3::ZERO
            } else {
                let turned = (Quat::from_rotation_arc(was, is) * n).normalize_or_zero();
                turned - n
            }
        })
        .collect()
}

/// Generates the non-destructive `Shrink_Under` morph offsets pulling garment
/// vertices smoothly inward toward `lower_envelope` (capped at `SHRINK_UNDER_MAX`).
fn build_shrink_under(
    rest: &[Vec3],
    welds: &Welds,
    lower_envelope: &super::surface::Surface,
) -> Vec<Vec3> {
    let raw: Vec<Vec3> = rest
        .iter()
        .map(|&p| {
            let Some(hit) = lower_envelope.closest(p, 0.10) else {
                return Vec3::ZERO;
            };
            let clearance = hit.signed(p);
            if clearance <= 0.0 {
                return Vec3::ZERO;
            }
            let pull = clearance.min(SHRINK_UNDER_MAX);
            -hit.normal * pull
        })
        .collect();
    welds.smooth(&raw, &[0.5], SHRINK_SMOOTH_ROUNDS)
}

/// Evaluates a garment's corner positions with `weights` of its carried shapes.
pub(super) fn garment_at(
    rest: &[Vec3],
    shapes: &[CarriedShape],
    weights: &[(String, f32)],
) -> Vec<Vec3> {
    let mut out = rest.to_vec();
    for (name, weight) in weights {
        if *weight == 0.0 {
            continue;
        }
        let Some(shape) = shapes.iter().find(|s| s.name == *name) else {
            continue;
        };
        for (p, o) in out.iter_mut().zip(&shape.offsets) {
            *p += *o * *weight;
        }
    }
    out
}

/// Runs `work` over `items` across scoped OS threads (one per available core).
pub(super) fn par_map<T: Sync, R: Send>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(items.len().max(1));
    if threads <= 1 || items.len() <= 2 {
        return items.iter().map(&work).collect();
    }
    let chunk = items.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|slice| scope.spawn(|| slice.iter().map(&work).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("fit worker panicked"))
            .collect()
    })
}

/// Coincident vertices (within `1e-5` m) merged into one topological point so
/// UV seams move and smooth together without opening.
pub(super) struct Welds {
    points: usize,
    of_corner: Vec<u32>,
    neighbours: Vec<Vec<u32>>,
}

impl Welds {
    pub(super) fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Self {
        let mut map: HashMap<[i64; 3], u32> = HashMap::new();
        let mut of_corner = Vec::with_capacity(positions.len());
        for &p in positions {
            let key = (p * 1e5).round().as_i64vec3().to_array();
            let next = map.len() as u32;
            of_corner.push(*map.entry(key).or_insert(next));
        }
        let points = map.len();
        let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); points];
        for t in triangles {
            let [a, b, c] = t.map(|v| of_corner[v as usize]);
            for (u, v) in [(a, b), (b, c), (c, a)] {
                if u != v {
                    neighbours[u as usize].push(v);
                    neighbours[v as usize].push(u);
                }
            }
        }
        for list in &mut neighbours {
            list.sort_unstable();
            list.dedup();
        }
        Self {
            points,
            of_corner,
            neighbours,
        }
    }

    /// Area-weighted smooth vertex normals over the welded surface, fanned back
    /// out to corners.
    pub(super) fn normals(&self, positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<Vec3> {
        let mut acc = vec![Vec3::ZERO; self.points];
        for t in triangles {
            let [a, b, c] = t.map(|v| v as usize);
            let face = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
            for &v in t {
                acc[self.of_corner[v as usize] as usize] += face;
            }
        }
        for n in &mut acc {
            *n = n.normalize_or_zero();
        }
        self.of_corner.iter().map(|&w| acc[w as usize]).collect()
    }

    /// Averages seam-split corners and runs `rounds` of Laplacian smoothing
    /// weighted by `share` (one value for all corners, or one per corner).
    pub(super) fn smooth(&self, values: &[Vec3], share: &[f32], rounds: usize) -> Vec<Vec3> {
        let mut sum = vec![Vec3::ZERO; self.points];
        let mut hits = vec![0u32; self.points];
        let mut w_share = vec![0.0f32; self.points];
        for (k, &w) in self.of_corner.iter().enumerate() {
            let wi = w as usize;
            sum[wi] += values[k];
            hits[wi] += 1;
            let s = share
                .get(k)
                .copied()
                .or_else(|| share.first().copied())
                .unwrap_or(0.5);
            w_share[wi] += s;
        }
        let mut cur: Vec<Vec3> = (0..self.points)
            .map(|wi| {
                if hits[wi] > 0 {
                    sum[wi] / hits[wi] as f32
                } else {
                    Vec3::ZERO
                }
            })
            .collect();
        for wi in 0..self.points {
            if hits[wi] > 0 {
                w_share[wi] /= hits[wi] as f32;
            }
        }
        for _ in 0..rounds {
            let prev = cur.clone();
            for wi in 0..self.points {
                let list = &self.neighbours[wi];
                if list.is_empty() || w_share[wi] <= 0.0 {
                    continue;
                }
                let avg = list.iter().map(|&n| prev[n as usize]).sum::<Vec3>() / list.len() as f32;
                cur[wi] = prev[wi].lerp(avg, w_share[wi]);
            }
        }
        self.of_corner.iter().map(|&w| cur[w as usize]).collect()
    }
}

/// Zeroes out arm-limb bone weights (`LeftUpperArm`, `RightUpperArm`, and their
/// descendants) on sleeveless garments and renormalizes to `Chest`/`UpperChest`
/// so sleeveless armholes never wing out when arms are raised.
fn strip_arm_weights(body: &BodyMesh, joints: &mut [[u16; 4]], weights: &mut [[f32; 4]]) {
    let rig = &body.rig;
    let mut is_arm_bone = vec![false; rig.bones.len()];
    for (i, bone) in rig.bones.iter().enumerate() {
        is_arm_bone[i] = matches!(bone.name.as_str(), "LeftUpperArm" | "RightUpperArm")
            || bone.parent.is_some_and(|p| is_arm_bone[p]);
    }
    let is_arm_joint: Vec<bool> = rig
        .joints
        .iter()
        .map(|j| is_arm_bone.get(j.bone).copied().unwrap_or(false))
        .collect();
    let fallback_joint = ["UpperChest", "Chest", "Spine", "Hips"]
        .iter()
        .find_map(|name| {
            let b = rig.bone(name)?;
            rig.joints
                .iter()
                .position(|j| j.bone == b)
                .map(|i| i as u16)
        })
        .unwrap_or(0);

    for (j, w) in joints.iter_mut().zip(weights.iter_mut()) {
        let mut sum = 0.0f32;
        for k in 0..4 {
            if is_arm_joint
                .get(usize::from(j[k]))
                .copied()
                .unwrap_or(false)
            {
                w[k] = 0.0;
            }
            sum += w[k];
        }
        if sum > 1e-6 {
            for wk in w.iter_mut() {
                *wk /= sum;
            }
        } else {
            *j = [fallback_joint, 0, 0, 0];
            *w = [1.0, 0.0, 0.0, 0.0];
        }
    }
}
