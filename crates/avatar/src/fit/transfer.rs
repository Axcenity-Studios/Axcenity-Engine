//! The transfer: a garment takes the body's skin weights and shapes
//! through its binding, and is pushed clear of the body and the layers
//! under it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::bind::{BIND_REACH, Binding, bind, frames};
use super::surface::Surface;
use super::{BodyMesh, FitError, GarmentMesh, is_mask_shape};
use crate::BaseManifest;

/// How far a garment's push spreads to its neighbours: each ring keeps
/// this share of the ring before.
const SPREAD: f32 = 0.7;
/// How many rings it spreads over.
const SPREAD_RINGS: usize = 6;
/// Body triangles farther than this from the garment's box are left out
/// of the push-out checks (they can't be close to it).
const NEAR: f32 = 0.10;
/// How many rounds the transferred weights are smoothed over the
/// garment.
const WEIGHT_ROUNDS: usize = 2;
/// A shape that moves no bound point farther than this moves the garment
/// by nothing worth checking.
const LEAST: f32 = 1e-5;

/// What [`fit_garment`] does besides binding.
#[derive(Clone, Debug)]
pub struct FitOptions {
    /// The clearance kept from the body and from every lower layer
    /// (metres; an item's `fit.margin`).
    pub margin: f32,
    /// The body shapes the garment carries (usually [`carried_shapes`]).
    pub shapes: Vec<String>,
    /// The layers under it, to stay outside of.
    pub under: Vec<Layer>,
    /// Combinations of body shape weights the garment must also clear the
    /// body and the lower layers at (the parameter extremes): anything
    /// still inside at one lifts the garment's rest a little there.
    pub extremes: Vec<Vec<(String, f32)>>,
    /// How many times the push-out is checked again after moving.
    pub rounds: usize,
}

impl Default for FitOptions {
    fn default() -> Self {
        FitOptions {
            margin: 0.002,
            shapes: Vec::new(),
            under: Vec::new(),
            extremes: Vec::new(),
            rounds: 4,
        }
    }
}

/// A lower layer a garment stays outside of: its surface at rest and its
/// shapes (offsets per corner, by the body shape's name).
#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    pub shapes: BTreeMap<String, Vec<Vec3>>,
}

impl Layer {
    /// A fitted garment as a layer for the next one out.
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
        }
    }

    /// A garment model (as [`BodyMesh::from_model`] reads one: a skinned
    /// item with shapes) as a layer.
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
        }
    }

    fn shaped(&self, weights: &[(String, f32)]) -> Vec<Vec3> {
        let mut out = self.positions.clone();
        for (name, weight) in weights {
            if let Some(offsets) = self.shapes.get(name) {
                for (p, o) in out.iter_mut().zip(offsets) {
                    *p += *o * *weight;
                }
            }
        }
        out
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

/// What one layer (the body, or a garment under this one) pushed out.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerOffset {
    /// `body`, or the lower layer's name.
    pub layer: String,
    /// Corners pushed at rest.
    pub pushed: usize,
    /// The farthest push at rest (metres).
    pub most: f32,
    /// Shapes at whose full weight it pushed anything.
    pub shapes_pushed: usize,
    /// The farthest push at any shape's full weight (metres).
    pub shape_most: f32,
}

/// What fitting did, for the item's records and the checks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FitReport {
    pub vertices: usize,
    pub triangles: usize,
    /// The body shapes carried (every name in [`FitOptions::shapes`] the
    /// body has), and how many of them move the garment at all.
    pub shapes_carried: usize,
    pub shapes_moving: usize,
    /// Corners bound farther than [`BIND_REACH`] from the body.
    pub far: usize,
    /// The largest offset from the body along its normal (metres).
    pub farthest: f32,
    /// What the body and each lower layer pushed.
    pub layers: Vec<LayerOffset>,
    /// Extreme combinations checked, how many lifted the garment, and the
    /// most it was lifted (metres).
    pub extremes: usize,
    pub extremes_lifting: usize,
    pub lift_most: f32,
    /// After everything: the deepest a garment corner is inside the body
    /// or a lower layer's surface at rest, at any carried shape's full
    /// weight or at any extreme (metres; 0 when none is).
    pub inside_most: f32,
}

/// A garment fitted to a body: its rest (pushed out), skin weights and
/// carried shapes, the binding and the report.
#[derive(Clone, Debug)]
pub struct Fitted {
    pub garment: GarmentMesh,
    /// Joint indices into the body's [`Rig::joints`](axcenity_assets::Rig),
    /// one set per corner.
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

/// The body's skin weights at each bound point: its corners' influences
/// mixed by the point's weights, the four largest kept, adding up to 1.
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

/// Skin weights from the skin under the garment: each body corner near
/// the garment casts a ray out along its normal, and the garment corners
/// of the triangle it meets (within 10 cm) take its weights, by how near
/// they are to where it meets. A corner that no skin reaches keeps the
/// weights it had (`joints` and `weights`, from its bound point).
///
/// Where the garment bridges a crease (a T-shirt's underarm between the
/// arm and the side), its own inward ray meets one side or the other, and
/// it would follow that limb alone; the skin of both sides faces it, so it
/// takes both, as the skin there does.
fn projected_weights(
    body: &BodyMesh,
    near: &[[u32; 3]],
    positions: &[Vec3],
    normals: &[Vec3],
    triangles: &[[u32; 3]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
    let garment = Surface::new(positions.to_vec(), normals.to_vec(), triangles.to_vec());
    let mut corners: BTreeSet<u32> = BTreeSet::new();
    for t in near {
        corners.extend(t.iter().copied());
    }
    let mut mixes: Vec<BTreeMap<u16, f32>> = vec![BTreeMap::new(); positions.len()];
    for corner in corners {
        let c = corner as usize;
        let out = body.normals[c];
        if out == Vec3::ZERO {
            continue;
        }
        let Some(hit) = garment.cast(body.positions[c], out, 0.10) else {
            continue;
        };
        // Only the garment's side that faces away from the skin: a ray
        // that meets the garment from outside is another limb's.
        let triangle = triangles[hit.triangle as usize];
        if hit.normal.dot(out) <= 0.0 {
            continue;
        }
        for (k, share) in triangle.iter().zip(hit.bary.to_array()) {
            if share <= 0.0 {
                continue;
            }
            let mix = &mut mixes[*k as usize];
            for (joint, weight) in body.joints[c].iter().zip(body.weights[c]) {
                if weight > 0.0 {
                    *mix.entry(*joint).or_default() += weight * share;
                }
            }
        }
    }
    let mut out_joints = joints.to_vec();
    let mut out_weights = weights.to_vec();
    for (k, mix) in mixes.into_iter().enumerate() {
        let mut sorted: Vec<(u16, f32)> = mix.into_iter().collect();
        let total: f32 = sorted.iter().map(|(_, w)| w).sum();
        // Less than a quarter of a corner's worth of skin: too little to go
        // by.
        if total < 0.25 {
            continue;
        }
        sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        sorted.truncate(4);
        let kept: f32 = sorted.iter().map(|(_, w)| w).sum();
        let mut j = [0u16; 4];
        let mut w = [0.0f32; 4];
        for (i, (joint, weight)) in sorted.into_iter().enumerate() {
            j[i] = joint;
            w[i] = weight / kept;
        }
        out_joints[k] = j;
        out_weights[k] = w;
    }
    (out_joints, out_weights)
}

/// The surfaces a garment is pushed out of in one state: the body (near
/// the garment) and each lower layer.
struct Obstacles {
    surfaces: Vec<(usize, Surface, bool)>,
}

/// Fits `garment` (in `body`'s rest pose and file space) to `body`.
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
    let mut rest = garment.positions();
    let rest_normals = garment.normals();
    let welds = Welds::new(&rest, &triangles);
    let near = near_triangles(body, garment);
    let mut layers: Vec<LayerOffset> = std::iter::once("body".to_owned())
        .chain(options.under.iter().map(|layer| layer.name.clone()))
        .map(|layer| LayerOffset {
            layer,
            ..LayerOffset::default()
        })
        .collect();
    let margin = options.margin;

    // 1. At rest: out of the body and the lower layers.
    let at_rest = obstacles(body, &near, &options.under, &[]);
    for _ in 0..options.rounds.max(1) {
        let (need, by) = needs(&at_rest, &rest, margin);
        if need.iter().all(|n| *n == Vec3::ZERO) {
            break;
        }
        record(&mut layers, &need, &by, false);
        for (p, a) in rest.iter_mut().zip(welds.spread(&need)) {
            *p += a;
        }
    }

    // 2. The binding, from where the garment now rests.
    let full = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let mut binding = bind(&full, &rest, &rest_normals);
    let rest_frames = frames(&binding, &body.positions, &body.normals, &body.triangles);

    // 3. Skin weights: the skin's under each corner (see
    // `projected_weights`), or the bound point's where no skin is.
    let (joints, weights) = transfer_weights(body, &binding);
    let (joints, weights) = projected_weights(
        body,
        &near,
        &rest,
        &rest_normals,
        &triangles,
        &joints,
        &weights,
    );
    // Smoothed over the garment, so where neighbouring corners were bound
    // to different limbs (a sleeve meeting the torso) the cloth stretches
    // over a few corners instead of tearing at one edge.
    let (joints, weights) = welds.smooth_weights(&joints, &weights, WEIGHT_ROUNDS);

    // 4. Every carried shape, pushed out at its full weight (each on its
    // own, so they are worked out side by side).
    let carried: Vec<(usize, &String)> = options
        .shapes
        .iter()
        .filter_map(|name| Some((body.shape_index(name)?, name)))
        .collect();
    let worked = par_map(&carried, |&(index, name)| {
        let weights = [(name.clone(), 1.0)];
        let (positions, normals) = body.shaped(&weights);
        let shaped_frames = frames(&binding, &positions, &normals, &body.triangles);
        let mut offsets = Vec::with_capacity(count);
        let mut turns = Vec::with_capacity(count);
        for (k, bound) in binding.vertices.iter().enumerate() {
            let (point, axes) = shaped_frames[k];
            let (_, rest_axes) = rest_frames[k];
            offsets.push(point + axes * Vec3::from(bound.local) - rest[k]);
            // How the frame turns: the garment's normal turns with it.
            turns.push(axes * rest_axes.transpose());
        }
        let moves =
            body.shapes[index].reach() > LEAST && offsets.iter().any(|o| o.length() > LEAST);
        let mut pushes = Vec::new();
        if moves {
            let state = obstacles(body, &near, &options.under, &weights);
            for _ in 0..options.rounds.max(1) {
                let now: Vec<Vec3> = rest.iter().zip(&offsets).map(|(p, o)| *p + *o).collect();
                let (need, by) = needs(&state, &now, margin);
                if need.iter().all(|n| *n == Vec3::ZERO) {
                    break;
                }
                for (o, a) in offsets.iter_mut().zip(welds.spread(&need)) {
                    *o += a;
                }
                pushes.push((need, by));
            }
        } else {
            offsets.iter_mut().for_each(|o| *o = Vec3::ZERO);
            turns.clear();
        }
        (name.clone(), offsets, turns, pushes)
    });
    let mut shapes = Vec::new();
    let mut shape_turns = Vec::new();
    let mut moving = 0;
    for (name, offsets, turns, pushes) in worked {
        if !turns.is_empty() {
            moving += 1;
        }
        for (need, by) in &pushes {
            record(&mut layers, need, by, true);
        }
        shapes.push(CarriedShape {
            name,
            offsets,
            normals: Vec::new(),
        });
        shape_turns.push(turns);
    }

    // 5. The extremes: what is still inside at one lifts the rest there,
    // checked again after each lift. A lift only moves the garment out, so
    // each round after the first checks again only the extremes that
    // lifted (step 6 checks them all once more).
    let mut lifted = vec![0.0f32; count];
    let mut lifting = 0;
    let mut checking: Vec<&Vec<(String, f32)>> = options.extremes.iter().collect();
    for round in 0..options.rounds.max(1) {
        let mut lift = vec![Vec3::ZERO; count];
        let mut any = 0;
        let wanted = par_map(&checking, |extreme| {
            extreme_need(body, &near, options, &rest, &shapes, extreme)
        });
        let mut again = Vec::new();
        for (need, extreme) in wanted.into_iter().zip(checking) {
            if need.iter().any(|n| *n != Vec3::ZERO) {
                any += 1;
                again.push(extreme);
                for (l, n) in lift.iter_mut().zip(&need) {
                    if n.length() > l.length() {
                        *l = *n;
                    }
                }
            }
        }
        if round == 0 {
            lifting = any;
        }
        checking = again;
        if any == 0 {
            break;
        }
        for ((p, a), total) in rest
            .iter_mut()
            .zip(welds.spread(&lift))
            .zip(lifted.iter_mut())
        {
            *p += a;
            *total += a.length();
        }
    }
    if lifted.iter().any(|l| *l > 0.0) {
        // The binding's offsets follow the lifted rest.
        for (bound, (k, (point, axes))) in binding
            .vertices
            .iter_mut()
            .zip(rest_frames.iter().enumerate())
        {
            bound.local = (axes.transpose() * (rest[k] - *point)).to_array();
        }
    }
    let lift_most = lifted.into_iter().fold(0.0, f32::max);

    // 6. What is left: the deepest anything is still inside.
    let mut inside_most = deepest(&obstacles(body, &near, &options.under, &[]), &rest);
    for shape in &shapes {
        if shape.reach() <= LEAST {
            continue;
        }
        let weights = [(shape.name.clone(), 1.0)];
        let now: Vec<Vec3> = rest
            .iter()
            .zip(&shape.offsets)
            .map(|(p, o)| *p + *o)
            .collect();
        inside_most = inside_most.max(deepest(
            &obstacles(body, &near, &options.under, &weights),
            &now,
        ));
    }
    let deepest_at = par_map(&options.extremes, |extreme| {
        let now = garment_at(&rest, &shapes, extreme);
        deepest(&obstacles(body, &near, &options.under, extreme), &now)
    });
    inside_most = deepest_at.into_iter().fold(inside_most, f32::max);

    // 7. Normals. Each of the garment's own normals is turned as far as
    // its surface turned where the push-out and the lift moved it (from the
    // welded surface's normal as it came to the one now): a corner nothing
    // moved keeps its normal exactly, hard edges and all.
    let made = welds.normals(&garment.positions(), &triangles);
    let now = welds.normals(&rest, &triangles);
    let fitted_normals: Vec<Vec3> = rest_normals
        .iter()
        .zip(made.iter().zip(&now))
        .map(|(n, (was, is))| {
            if was.distance_squared(*is) < 1e-12 || *was == Vec3::ZERO || *is == Vec3::ZERO {
                *n
            } else {
                (Quat::from_rotation_arc(*was, *is) * *n).normalize_or_zero()
            }
        })
        .collect();
    // Each shape turns those normals with the body's frame.
    for (shape, turns) in shapes.iter_mut().zip(&shape_turns) {
        shape.normals = if turns.is_empty() {
            vec![Vec3::ZERO; count]
        } else {
            turns
                .iter()
                .zip(&fitted_normals)
                .map(|(turn, n)| *turn * *n - *n)
                .collect()
        };
    }

    let mut fitted_garment = garment.clone();
    fitted_garment.set_positions(&rest);
    fitted_garment.set_normals(&fitted_normals);
    let farthest = binding
        .vertices
        .iter()
        .map(|b| b.local[2].abs())
        .fold(0.0, f32::max);
    let far = binding
        .vertices
        .iter()
        .filter(|b| Vec3::from(b.local).length() > BIND_REACH)
        .count();
    let report = FitReport {
        vertices: count,
        triangles: triangles.len(),
        shapes_carried: shapes.len(),
        shapes_moving: moving,
        far,
        farthest,
        layers,
        extremes: options.extremes.len(),
        extremes_lifting: lifting,
        lift_most,
        inside_most,
    };
    Ok(Fitted {
        garment: fitted_garment,
        joints,
        weights,
        shapes,
        binding,
        report,
    })
}

/// `extremes` as fitting `garment` needs them: each with only the weights
/// of the body shapes that move the body near the garment (within 10 cm of
/// its box; the others can't reach it, nor move what it is bound to), the
/// ones that become the same or rest left out.
pub fn extremes_near(
    body: &BodyMesh,
    garment: &GarmentMesh,
    extremes: &[Vec<(String, f32)>],
) -> Vec<Vec<(String, f32)>> {
    let mut near = vec![false; body.positions.len()];
    for triangle in near_triangles(body, garment) {
        for v in triangle {
            near[v as usize] = true;
        }
    }
    let reaching: BTreeSet<&str> = body
        .shapes
        .iter()
        .filter(|shape| {
            shape
                .corners
                .iter()
                .zip(&shape.offsets)
                .any(|(corner, offset)| near[*corner as usize] && offset.length() > LEAST)
        })
        .map(|shape| shape.name.as_str())
        .collect();
    let mut out: Vec<Vec<(String, f32)>> = extremes
        .iter()
        .map(|extreme| {
            extreme
                .iter()
                .filter(|(name, _)| reaching.contains(name.as_str()))
                .cloned()
                .collect::<Vec<_>>()
        })
        .filter(|extreme| !extreme.is_empty())
        .collect();
    out.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    out.dedup();
    out
}

/// The garment's corners with `weights` of its carried shapes.
pub(crate) fn garment_at(
    rest: &[Vec3],
    shapes: &[CarriedShape],
    weights: &[(String, f32)],
) -> Vec<Vec3> {
    let mut out = rest.to_vec();
    for (name, weight) in weights {
        if let Some(shape) = shapes.iter().find(|s| &s.name == name) {
            for (p, o) in out.iter_mut().zip(&shape.offsets) {
                *p += *o * *weight;
            }
        }
    }
    out
}

fn extreme_need(
    body: &BodyMesh,
    near: &[[u32; 3]],
    options: &FitOptions,
    rest: &[Vec3],
    shapes: &[CarriedShape],
    extreme: &[(String, f32)],
) -> Vec<Vec3> {
    let now = garment_at(rest, shapes, extreme);
    let state = obstacles(body, near, &options.under, extreme);
    needs(&state, &now, options.margin).0
}

/// The body's triangles near the garment's box (the rest are too far to
/// matter for push-out).
fn near_triangles(body: &BodyMesh, garment: &GarmentMesh) -> Vec<[u32; 3]> {
    let (lo, hi) = garment.bounds();
    let (lo, hi) = (lo - Vec3::splat(NEAR), hi + Vec3::splat(NEAR));
    body.triangles
        .iter()
        .copied()
        .filter(|t| {
            t.iter().any(|&v| {
                let p = body.positions[v as usize];
                p.cmpge(lo).all() && p.cmple(hi).all()
            })
        })
        .collect()
}

fn obstacles(
    body: &BodyMesh,
    near: &[[u32; 3]],
    under: &[Layer],
    weights: &[(String, f32)],
) -> Obstacles {
    let (positions, normals) = body.shaped(weights);
    let mut surfaces = vec![(0, Surface::new(positions, normals, near.to_vec()), false)];
    for (k, layer) in under.iter().enumerate() {
        let positions = layer.shaped(weights);
        // A lower layer's normals barely turn with a shape: its rest ones.
        surfaces.push((
            k + 1,
            Surface::new(positions, layer.normals.clone(), layer.triangles.clone()),
            true,
        ));
    }
    Obstacles { surfaces }
}

/// How each of `points` must move to be `margin` clear of every
/// obstacle: out along the obstacle's normal at its closest point (moving
/// along the garment's own normal could push it into the next obstacle:
/// a sleeve's underside, pushed out of the arm, into the torso), the
/// biggest move wanted, and which obstacle wants it.
fn needs(state: &Obstacles, points: &[Vec3], margin: f32) -> (Vec<Vec3>, Vec<usize>) {
    let mut need = vec![Vec3::ZERO; points.len()];
    let mut by = vec![0usize; points.len()];
    for (k, &p) in points.iter().enumerate() {
        for (layer, surface, only_over) in &state.surfaces {
            let Some(hit) = surface.closest(p, margin + 0.03) else {
                continue;
            };
            if *only_over && !hit.is_over(p) {
                continue;
            }
            let signed = hit.signed(p);
            if signed >= margin {
                continue;
            }
            let push = hit.normal * (margin - signed);
            if push.length() > need[k].length() {
                need[k] = push;
                by[k] = *layer;
            }
        }
    }
    (need, by)
}

/// The deepest any of `points` is inside an obstacle (0 when none is).
fn deepest(state: &Obstacles, points: &[Vec3]) -> f32 {
    let mut most = 0.0f32;
    for &p in points {
        for (_, surface, only_over) in &state.surfaces {
            let Some(hit) = surface.closest(p, 0.03) else {
                continue;
            };
            if *only_over && !hit.is_over(p) {
                continue;
            }
            most = most.max(-hit.signed(p));
        }
    }
    most
}

fn record(layers: &mut [LayerOffset], need: &[Vec3], by: &[usize], shape: bool) {
    let mut pushed_any = vec![false; layers.len()];
    for (n, b) in need.iter().zip(by) {
        let n = &n.length();
        if *n <= 0.0 {
            continue;
        }
        let layer = &mut layers[*b];
        pushed_any[*b] = true;
        if shape {
            layer.shape_most = layer.shape_most.max(*n);
        } else {
            layer.pushed += 1;
            layer.most = layer.most.max(*n);
        }
    }
    if shape {
        for (layer, any) in layers.iter_mut().zip(pushed_any) {
            if any {
                layer.shapes_pushed += 1;
            }
        }
    }
}

/// Corners at the same place (a seam between pieces or texture islands)
/// are one, so a push never opens a seam; and which welded corners are
/// neighbours, so a push spreads smoothly.
struct Welds {
    /// Each corner's welded group.
    group: Vec<usize>,
    /// Each group's neighbouring groups.
    neighbours: Vec<Vec<usize>>,
}

impl Welds {
    fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Welds {
        let mut by_place: HashMap<[i64; 3], usize> = HashMap::new();
        let group: Vec<usize> = positions
            .iter()
            .map(|p| {
                // Within a hundredth of a millimetre is the same place.
                let key = (*p * 1e5).round().as_i64vec3().to_array();
                let next = by_place.len();
                *by_place.entry(key).or_insert(next)
            })
            .collect();
        let mut neighbours = vec![Vec::new(); by_place.len()];
        for t in triangles {
            let g = t.map(|v| group[v as usize]);
            for (a, b) in [(g[0], g[1]), (g[1], g[2]), (g[2], g[0])] {
                if a != b {
                    neighbours[a].push(b);
                    neighbours[b].push(a);
                }
            }
        }
        for list in &mut neighbours {
            list.sort_unstable();
            list.dedup();
        }
        Welds { group, neighbours }
    }

    /// Smooth normals of `positions` over `triangles`, the same for welded
    /// corners (a seam between pieces or texture islands doesn't show).
    fn normals(&self, positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<Vec3> {
        let mut sums = vec![Vec3::ZERO; self.neighbours.len()];
        for &[a, b, c] in triangles {
            let (pa, pb, pc) = (
                positions[a as usize],
                positions[b as usize],
                positions[c as usize],
            );
            // Twice the area: bigger triangles count for more.
            let face = (pb - pa).cross(pc - pa);
            for v in [a, b, c] {
                sums[self.group[v as usize]] += face;
            }
        }
        self.group
            .iter()
            .map(|g| sums[*g].normalize_or_zero())
            .collect()
    }

    /// Skin weights (per corner) made the same for welded corners and
    /// each round mixed half and half with the mean of the neighbours',
    /// the four largest kept, adding up to 1.
    fn smooth_weights(
        &self,
        joints: &[[u16; 4]],
        weights: &[[f32; 4]],
        rounds: usize,
    ) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
        let mut groups: Vec<BTreeMap<u16, f32>> = vec![BTreeMap::new(); self.neighbours.len()];
        let mut counts = vec![0usize; self.neighbours.len()];
        for (corner, (j, w)) in joints.iter().zip(weights).enumerate() {
            let g = self.group[corner];
            counts[g] += 1;
            for (joint, weight) in j.iter().zip(w) {
                if *weight > 0.0 {
                    *groups[g].entry(*joint).or_default() += *weight;
                }
            }
        }
        for (map, count) in groups.iter_mut().zip(&counts) {
            for weight in map.values_mut() {
                *weight /= (*count).max(1) as f32;
            }
        }
        for _ in 0..rounds {
            let before = groups.clone();
            for (g, list) in self.neighbours.iter().enumerate() {
                if list.is_empty() {
                    continue;
                }
                let mut mixed: BTreeMap<u16, f32> = BTreeMap::new();
                for (joint, weight) in &before[g] {
                    *mixed.entry(*joint).or_default() += 0.5 * weight;
                }
                let share = 0.5 / list.len() as f32;
                for other in list {
                    for (joint, weight) in &before[*other] {
                        *mixed.entry(*joint).or_default() += share * weight;
                    }
                }
                groups[g] = mixed;
            }
        }
        let limited: Vec<([u16; 4], [f32; 4])> = groups
            .iter()
            .map(|map| {
                let mut sorted: Vec<(u16, f32)> = map.iter().map(|(j, w)| (*j, *w)).collect();
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
                (j, w)
            })
            .collect();
        self.group.iter().map(|g| limited[*g]).unzip()
    }

    /// The moves `need` (per corner) made the same for welded corners
    /// (the biggest) and spread over the neighbours, fading by [`SPREAD`] a
    /// ring: a corner takes a neighbour's move, faded, when that is bigger
    /// than its own, so nowhere moves less than asked.
    fn spread(&self, need: &[Vec3]) -> Vec<Vec3> {
        let mut amount = vec![Vec3::ZERO; self.neighbours.len()];
        for (corner, n) in need.iter().enumerate() {
            let g = self.group[corner];
            if n.length() > amount[g].length() {
                amount[g] = *n;
            }
        }
        for _ in 0..SPREAD_RINGS {
            let before = amount.clone();
            for (g, list) in self.neighbours.iter().enumerate() {
                for &other in list {
                    let faded = before[other] * SPREAD;
                    if faded.length() > amount[g].length() {
                        amount[g] = faded;
                    }
                }
            }
        }
        self.group.iter().map(|g| amount[*g]).collect()
    }
}

/// `each` of `items`, worked out on several threads (the machine's, at most
/// eight), in order.
fn par_map<T: Sync, R: Send>(items: &[T], each: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8);
    if threads == 1 || items.len() < 2 {
        return items.iter().map(&each).collect();
    }
    let chunk = items.len().div_ceil(threads);
    let each = &each;
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(each).collect::<Vec<R>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a fitting thread panicked"))
            .collect()
    })
}
