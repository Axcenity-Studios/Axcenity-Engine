//! Two-stage biharmonic skin-weight and frame-relative morph-delta inpainting
//! (ADR-0046).
//!
//! Implements Abdrashitov, Raichstat, Monsen, and Hill, *"Robust Skin Weights
//! Transfer via Weight Inpainting"* (SIGGRAPH Asia 2023; MIT sample code) in
//! pure safe Rust using `faer` sparse simplicial Cholesky (`LLT`).
//!
//! For a welded garment mesh with cotangent Laplacian $\mathbf{L}$ (symmetric
//! positive semi-definite form: $L_{ij} = -w_{ij} \le 0$ for $i \neq j$,
//! $L_{ii} = \sum_j w_{ij}$) and lumped vertex area mass matrix $\mathbf{M}$,
//! the biharmonic + Dirichlet energy matrix is:
//!
//! $$\mathbf{Q} = \mathbf{L} + \mathbf{L}\mathbf{M}^{-1}\mathbf{L}$$
//!
//! Partitioning vertices into confident ($c$) and free ($u$), we factor
//! $\mathbf{Q}_{uu} + \epsilon\mathbf{I}$ **once** via `faer` sparse Cholesky
//! and solve:
//!
//! $$\mathbf{Q}_{uu}\mathbf{X}_u = -\mathbf{Q}_{uc}\mathbf{X}_c$$
//!
//! for both:
//! 1. **Skin weights** (one RHS column per active bone, clamped $\ge 0$,
//!    renormalized, truncated to top-4 joints, with cuff/hem joint holding);
//! 2. **Frame-relative morph deltas** (3 RHS columns for residual $x, y, z$
//!    relative to a smooth local base displacement $\mathbf{b}$, so free-hanging
//!    skirt hems, open coats, and lapels expand naturally with hip/thigh/chest
//!    morphs without flattening).

use std::collections::{BTreeMap, HashMap};

use faer::Side;
use faer::linalg::solvers::Solve;
use faer::mat::Mat;
use faer::sparse::linalg::solvers::{Llt, SymbolicLlt};
use faer::sparse::{SparseColMat, Triplet};
use glam::Vec3;

use super::bind::{ProxyBinding, VertexBind};
use super::proxy::FitProxy;
use super::{BodyMesh, FitError};

/// Tiny diagonal regularization so isolated or flat unbound components remain
/// strictly positive definite during Cholesky factorization.
const REGULARIZATION: f64 = 1e-8;
/// Floor for lumped vertex mass (square metres) to prevent division by zero on
/// tiny triangles.
const MIN_MASS: f64 = 1e-10;
/// Light post-solve Laplacian smoothing rounds applied only to solved (free)
/// skin weights, as recommended in Abdrashitov et al. (2023).
const POST_SMOOTH_ROUNDS: usize = 6;
const POST_SMOOTH_ALPHA: f32 = 0.15;

/// Pre-factored biharmonic energy solver over a welded garment mesh.
pub struct BiharmonicSolver {
    welded_count: usize,
    of_corner: Vec<u32>,
    /// Confident welded vertex indices (sorted).
    confident: Vec<usize>,
    /// Free (unbound) welded vertex indices (sorted).
    free: Vec<usize>,
    /// Maps welded index -> Some(free_slot) or None (confident).
    free_slot: Vec<Option<usize>>,
    /// Sparse off-diagonal block `Q_uc` (rows = free, cols = confident), stored
    /// as `(free_row, confident_col, value)` entries for fast RHS assembly.
    q_uc: Vec<(usize, usize, f64)>,
    /// Sparse Cholesky factorization of `Q_uu + eps * I` (`None` when `free` is empty).
    llt: Option<Llt<usize, f64>>,
    /// Welded vertex adjacency for optional light post-smoothing on free weights.
    neighbours: Vec<Vec<usize>>,
}

impl BiharmonicSolver {
    /// Builds cotangent Laplacian $\mathbf{L}$, lumped mass $\mathbf{M}$, and
    /// factors $\mathbf{Q}_{uu} = (\mathbf{L} + \mathbf{L}\mathbf{M}^{-1}\mathbf{L})_{uu}$
    /// using `faer` sparse Cholesky.
    pub fn new(
        positions: &[Vec3],
        triangles: &[[u32; 3]],
        confident_corners: &[bool],
    ) -> Result<Self, FitError> {
        let (welded_pos, of_corner) = weld_positions(positions);
        let n = welded_pos.len();
        if n == 0 {
            return Err(FitError::Empty);
        }

        // A welded vertex is confident if any of its UV-split corners is confident.
        let mut is_confident = vec![false; n];
        for (&w, &c) in of_corner.iter().zip(confident_corners) {
            if c {
                is_confident[w as usize] = true;
            }
        }

        let mut confident = Vec::new();
        let mut free = Vec::new();
        let mut conf_slot = vec![None; n];
        let mut free_slot = vec![None; n];
        for i in 0..n {
            if is_confident[i] {
                conf_slot[i] = Some(confident.len());
                confident.push(i);
            } else {
                free_slot[i] = Some(free.len());
                free.push(i);
            }
        }

        // Build welded cotangent Laplacian L (PSD) and lumped mass M.
        let mut rows: Vec<HashMap<usize, f64>> = vec![HashMap::new(); n];
        let mut mass = vec![0.0f64; n];
        let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); n];

        for tri in triangles {
            let [a, b, c] = tri.map(|v| of_corner[v as usize] as usize);
            if a == b || b == c || a == c {
                continue;
            }
            let pa = welded_pos[a].as_dvec3();
            let pb = welded_pos[b].as_dvec3();
            let pc = welded_pos[c].as_dvec3();
            let area = 0.5 * (pb - pa).cross(pc - pa).length();
            if area <= 1e-16 {
                continue;
            }
            let share = area / 3.0;
            mass[a] += share;
            mass[b] += share;
            mass[c] += share;

            // Clamped cotangent weights (>= 0) keep L strictly symmetric PSD.
            let cot_a = cotangent(pb - pa, pc - pa);
            let cot_b = cotangent(pc - pb, pa - pb);
            let cot_c = cotangent(pa - pc, pb - pc);

            for (u, v, w) in [
                (b, c, 0.5 * cot_a),
                (c, a, 0.5 * cot_b),
                (a, b, 0.5 * cot_c),
            ] {
                let w = w.max(1e-6);
                *rows[u].entry(v).or_default() -= w;
                *rows[v].entry(u).or_default() -= w;
                *rows[u].entry(u).or_default() += w;
                *rows[v].entry(v).or_default() += w;
                neighbours[u].push(v);
                neighbours[v].push(u);
            }
        }
        for list in &mut neighbours {
            list.sort_unstable();
            list.dedup();
        }

        if free.is_empty() || confident.is_empty() {
            return Ok(Self {
                welded_count: n,
                of_corner,
                confident,
                free,
                free_slot,
                q_uc: Vec::new(),
                llt: None,
                neighbours,
            });
        }

        // Normalize lumped mass by mean vertex area so `L M^-1 L` stays O(1)–O(1e3)
        // regardless of mesh resolution (preventing float64 cancellation on dense meshes).
        let mean_mass = (mass.iter().sum::<f64>() / n as f64).max(MIN_MASS);
        let inv_mass: Vec<f64> = mass
            .iter()
            .map(|&m| (mean_mass / m.max(mean_mass * 1e-3)).min(1e3))
            .collect();

        // Assemble Q = L + L * M^-1 * L for free rows `i in free`:
        // Q[i, j] = L[i, j] + sum_k L[i, k] * inv_mass[k] * L[k, j].
        let nf = free.len();
        let mut q_uu_triplets: Vec<Triplet<usize, usize, f64>> = Vec::new();
        let mut q_uc: Vec<(usize, usize, f64)> = Vec::new();

        for (fi, &i) in free.iter().enumerate() {
            let mut acc: HashMap<usize, f64> = HashMap::new();
            for (&k, &lik) in &rows[i] {
                *acc.entry(k).or_default() += lik;
                let scale = lik * inv_mass[k];
                for (&j, &lkj) in &rows[k] {
                    *acc.entry(j).or_default() += scale * lkj;
                }
            }
            // Add tiny regularization on the diagonal of Q_uu.
            *acc.entry(i).or_default() += REGULARIZATION;

            for (j, val) in acc {
                if val.abs() <= 1e-18 {
                    continue;
                }
                if let Some(fj) = free_slot[j] {
                    // Store lower triangle (row >= col) for faer Side::Lower LLT.
                    if fi >= fj {
                        q_uu_triplets.push(Triplet::new(fi, fj, val));
                    }
                } else if let Some(cj) = conf_slot[j] {
                    q_uc.push((fi, cj, val));
                }
            }
        }

        let mut q_uu = SparseColMat::<usize, f64>::try_new_from_triplets(nf, nf, &q_uu_triplets)
            .map_err(|err| FitError::Model(format!("failed to assemble sparse Q_uu: {err:?}")))?;
        let symbolic = SymbolicLlt::try_new(q_uu.symbolic(), Side::Lower)
            .map_err(|err| FitError::Model(format!("failed symbolic Cholesky on Q_uu: {err:?}")))?;

        let llt = match Llt::try_new_with_symbolic(symbolic.clone(), q_uu.as_ref(), Side::Lower) {
            Ok(llt) => llt,
            Err(_) => {
                // Retry with progressively larger diagonal Tikhonov shifts if ill-conditioned.
                let mut solved = None;
                let mut last_err = None;
                for shift in [1e-6_f64, 1e-4, 1e-2, 1e-1] {
                    let mut shifted = q_uu_triplets.clone();
                    for fi in 0..nf {
                        shifted.push(Triplet::new(fi, fi, shift));
                    }
                    q_uu = SparseColMat::<usize, f64>::try_new_from_triplets(nf, nf, &shifted)
                        .map_err(|err| {
                            FitError::Model(format!("failed to assemble shifted Q_uu: {err:?}"))
                        })?;
                    match Llt::try_new_with_symbolic(symbolic.clone(), q_uu.as_ref(), Side::Lower) {
                        Ok(llt) => {
                            solved = Some(llt);
                            break;
                        }
                        Err(err) => last_err = Some(err),
                    }
                }
                solved.ok_or_else(|| {
                    FitError::Model(format!("failed numeric Cholesky on Q_uu: {last_err:?}"))
                })?
            }
        };

        Ok(Self {
            welded_count: n,
            of_corner,
            confident,
            free,
            free_slot,
            q_uc,
            llt: Some(llt),
            neighbours,
        })
    }

    /// Number of confident welded vertices.
    pub fn confident_welded(&self) -> usize {
        self.confident.len()
    }

    /// Number of free (inpainted) welded vertices.
    pub fn free_welded(&self) -> usize {
        self.free.len()
    }

    /// Solves scalar fields on free welded vertices from known values on
    /// confident welded vertices (`known_columns[col][welded_idx]`).
    pub fn solve_welded_scalars(&self, known_columns: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let ncols = known_columns.len();
        if ncols == 0 {
            return Vec::new();
        }
        let mut out = known_columns.to_vec();
        let Some(llt) = &self.llt else {
            return out;
        };
        let nf = self.free.len();
        let mut rhs = Mat::<f64>::zeros(nf, ncols);
        for &(fi, cj, qval) in &self.q_uc {
            let c_welded = self.confident[cj];
            for (col, known) in known_columns.iter().enumerate() {
                rhs[(fi, col)] -= qval * known[c_welded];
            }
        }
        let sol = llt.solve(&rhs);
        for (fi, &w) in self.free.iter().enumerate() {
            for (col, out_col) in out.iter_mut().enumerate() {
                out_col[w] = sol[(fi, col)];
            }
        }
        out
    }

    /// Solves 3D morph-target deltas on free corners **relative to a smooth
    /// local base displacement** `base_deltas` (ADR-0046 Refinement #1).
    ///
    /// On confident corners, `known_deltas` is preserved exactly. On free
    /// corners (wide skirt hems, open coats, lapels), the residual
    /// `known_deltas - base_deltas` is biharmonically inpainted and added back
    /// to `base_deltas`, so free-hanging hems expand naturally with local
    /// hip/thigh/chest morphs instead of flattening.
    pub fn solve_deltas_relative(&self, known_deltas: &[Vec3], base_deltas: &[Vec3]) -> Vec<Vec3> {
        let count = known_deltas.len();
        if self.llt.is_none() {
            return known_deltas.to_vec();
        }

        // Average confident residuals per welded vertex.
        let mut sum = vec![Vec3::ZERO; self.welded_count];
        let mut hits = vec![0u32; self.welded_count];
        for i in 0..count {
            let w = self.of_corner[i] as usize;
            if self.free_slot[w].is_none() {
                sum[w] += known_deltas[i] - base_deltas[i];
                hits[w] += 1;
            }
        }
        let mut cols = vec![vec![0.0f64; self.welded_count]; 3];
        for w in 0..self.welded_count {
            if hits[w] > 0 {
                let avg = sum[w] / hits[w] as f32;
                cols[0][w] = f64::from(avg.x);
                cols[1][w] = f64::from(avg.y);
                cols[2][w] = f64::from(avg.z);
            }
        }
        let solved = self.solve_welded_scalars(&cols);

        (0..count)
            .map(|i| {
                let w = self.of_corner[i] as usize;
                if self.free_slot[w].is_none() {
                    known_deltas[i]
                } else {
                    let residual = Vec3::new(
                        solved[0][w] as f32,
                        solved[1][w] as f32,
                        solved[2][w] as f32,
                    );
                    base_deltas[i] + residual
                }
            })
            .collect()
    }
}

/// Two-stage biharmonic skin-weight transfer (Abdrashitov et al. 2023):
/// 1. Copy barycentric skin weights from the proxy on confident vertices (with
///    cuff/hem joint holding: hands -> forearm, feet -> shin, forearm twist -> forearm).
/// 2. Solve free vertices via the pre-factored biharmonic operator (`faer` LLT).
/// 3. Clamp negatives to 0, apply light Laplacian smoothing to solved vertices,
///    renormalize, and truncate to top-4 joints.
pub fn inpaint_weights(
    solver: &BiharmonicSolver,
    body: &BodyMesh,
    proxy: &FitProxy,
    binding: &ProxyBinding,
) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
    let corner_count = binding.vertices.len();
    let (proxy_joints, proxy_weights) = proxy.vertex_skin_weights(body);
    let held = held_joints(body);

    // Accumulate confident weights per welded vertex.
    let mut welded_mix: Vec<BTreeMap<u16, f64>> = vec![BTreeMap::new(); solver.welded_count];
    let mut welded_hits = vec![0u32; solver.welded_count];
    let mut active_bones: BTreeMap<u16, usize> = BTreeMap::new();

    for (corner, vb) in binding.vertices.iter().enumerate() {
        let VertexBind::Confident(bound) = vb else {
            continue;
        };
        let w = solver.of_corner[corner] as usize;
        welded_hits[w] += 1;
        let tri = proxy.triangles[bound.triangle as usize];
        for (&pv, &bary) in tri.iter().zip(&bound.bary) {
            if bary <= 0.0 {
                continue;
            }
            let pvi = pv as usize;
            for (&joint, &jw) in proxy_joints[pvi].iter().zip(&proxy_weights[pvi]) {
                if jw > 0.0 {
                    let j = held.get(usize::from(joint)).copied().unwrap_or(joint);
                    *welded_mix[w].entry(j).or_default() += f64::from(jw * bary);
                    let next = active_bones.len();
                    active_bones.entry(j).or_insert(next);
                }
            }
        }
    }

    if active_bones.is_empty() {
        // Fallback: zero-confident garment takes root bone 0.
        return (
            vec![[0u16; 4]; corner_count],
            vec![[1.0, 0.0, 0.0, 0.0]; corner_count],
        );
    }

    let bone_list: Vec<u16> = {
        let mut list = vec![0u16; active_bones.len()];
        for (&joint, &col) in &active_bones {
            list[col] = joint;
        }
        list
    };
    let mut columns = vec![vec![0.0f64; solver.welded_count]; bone_list.len()];
    for w in 0..solver.welded_count {
        if welded_hits[w] == 0 {
            continue;
        }
        let inv = 1.0 / f64::from(welded_hits[w]);
        for (&joint, &val) in &welded_mix[w] {
            columns[active_bones[&joint]][w] = val * inv;
        }
    }

    let mut solved = solver.solve_welded_scalars(&columns);

    // Clamp slight biharmonic undershoots (< 0) to 0.
    for col in &mut solved {
        for v in col.iter_mut() {
            if !v.is_finite() || *v < 0.0 {
                *v = 0.0;
            }
        }
    }

    // Light Laplacian smoothing on solved (free) vertices only.
    if !solver.free.is_empty() {
        for col in &mut solved {
            for _ in 0..POST_SMOOTH_ROUNDS {
                let prev = col.clone();
                for &w in &solver.free {
                    let list = &solver.neighbours[w];
                    if list.is_empty() {
                        continue;
                    }
                    let avg = list.iter().map(|&n| prev[n]).sum::<f64>() / list.len() as f64;
                    col[w] = prev[w] * (1.0 - f64::from(POST_SMOOTH_ALPHA))
                        + avg * f64::from(POST_SMOOTH_ALPHA);
                }
            }
        }
    }

    // Extract top-4 normalized influences per welded vertex, then fan out to corners.
    let mut w_joints = vec![[0u16; 4]; solver.welded_count];
    let mut w_weights = vec![[0.0f32; 4]; solver.welded_count];
    for w in 0..solver.welded_count {
        let mut pairs: Vec<(u16, f32)> = bone_list
            .iter()
            .enumerate()
            .filter_map(|(col, &joint)| {
                let val = solved[col][w] as f32;
                (val > 1e-6).then_some((joint, val))
            })
            .collect();
        pairs.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        pairs.truncate(4);
        let total: f32 = pairs.iter().map(|(_, v)| *v).sum();
        if total > 0.0 {
            for (k, (joint, val)) in pairs.into_iter().enumerate() {
                w_joints[w][k] = joint;
                w_weights[w][k] = val / total;
            }
        } else {
            w_joints[w][0] = bone_list[0];
            w_weights[w][0] = 1.0;
        }
    }

    let joints = solver
        .of_corner
        .iter()
        .map(|&w| w_joints[w as usize])
        .collect();
    let weights = solver
        .of_corner
        .iter()
        .map(|&w| w_weights[w as usize])
        .collect();
    (joints, weights)
}

/// Welds vertices within `1e-5` m so UV-split seams share topological edges.
fn weld_positions(positions: &[Vec3]) -> (Vec<Vec3>, Vec<u32>) {
    let mut map: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let mut welded = Vec::new();
    let mut of_corner = Vec::with_capacity(positions.len());
    for &p in positions {
        let key = (p * 1e5).round().as_i64vec3().to_array();
        let id = *map.entry(key).or_insert_with(|| {
            let next = welded.len() as u32;
            welded.push(p);
            next
        });
        of_corner.push(id);
    }
    (welded, of_corner)
}

/// Clamped cotangent of the angle between `u` and `v` (`>= 0`).
fn cotangent(u: glam::DVec3, v: glam::DVec3) -> f64 {
    let cross = u.cross(v).length();
    if cross <= 1e-12 {
        return 0.0;
    }
    (u.dot(v) / cross).max(0.0)
}

/// Remaps hand/finger/forearm-twist joints to `LowerArm` and foot/toe joints to
/// `LowerLeg` so cuffs and hems follow the limb cleanly without collapsing.
fn held_joints(body: &BodyMesh) -> Vec<u16> {
    let rig = &body.rig;
    let joint_of = |name: &str| -> Option<u16> {
        let bone = rig.bone(name)?;
        rig.joints
            .iter()
            .position(|joint| joint.bone == bone)
            .map(|joint| joint as u16)
    };
    (0..rig.joints.len())
        .map(|own| {
            let mut at = Some(rig.joints[own].bone);
            let mut first = true;
            while let Some(bone) = at {
                let name = rig.bones[bone].name.as_str();
                for side in ["Left", "Right"] {
                    let Some(part) = name.strip_prefix(side) else {
                        continue;
                    };
                    let onto = match part {
                        "Hand" => Some("LowerArm"),
                        "LowerArmTwist" if first => Some("LowerArm"),
                        "Foot" => Some("LowerLeg"),
                        _ => None,
                    };
                    if let Some(joint) = onto.and_then(|onto| joint_of(&format!("{side}{onto}"))) {
                        return joint;
                    }
                }
                first = false;
                at = rig.bones[bone].parent;
            }
            own as u16
        })
        .collect()
}
