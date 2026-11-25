//! Authored and derived Fit Proxies and Layer Envelopes (ADR-0046).
//!
//! Binding loose garments directly to raw body skin fails across high-curvature
//! concavities (armpit crease, cleavage, crotch, buttock cleft): neighbouring
//! vertices bind to opposite sides of a crease, and triangle-frame rotations
//! amplify lever-arm offsets into multi-centimetre spikes.
//!
//! A [`FitProxy`] is a smooth, fixed-topology stand-in surface whose vertices
//! are fixed barycentric mixes of [`BodyMesh`] vertices plus a fixed outward
//! concavity-bridging offset. Because every proxy vertex is a linear
//! combination of body vertices, the proxy inherits every identity morph and
//! pose corrective of the body automatically and deterministically.
//!
//! Two proxy surfaces are supported:
//! - [`ProxyKind::Body`]: bridges armpits, cleavage, crotch, and buttock cleft.
//! - [`ProxyKind::Skirt`]: a smooth hip-to-ankle tube wrapping both legs so
//!   skirts, dresses, and long coats never split between the thighs.
//!
//! Deterministic radial [`LAYER_ENVELOPE_OFFSETS`] (Layers 0–3) define the
//! target surfaces for the non-destructive `Shrink_Under` morph target.

use std::collections::BTreeMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::BodyMesh;
use super::surface::{Surface, vertex_normals};

/// Deterministic radial clearances (metres) for Layer Envelopes 0..=3.
///
/// - Layer 0 (underwear / tucked base): 2 mm
/// - Layer 1 (base tops & bottoms): 6 mm
/// - Layer 2 (mid layers: sweaters, hoodies, overalls): 12 mm
/// - Layer 3 (outerwear: jackets, coats, suits): 20 mm
pub const LAYER_ENVELOPE_OFFSETS: [f32; 4] = [0.002, 0.006, 0.012, 0.020];

/// Which stand-in surface a garment binds to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyKind {
    /// Smooth body proxy bridging armpits, cleavage, crotch, and buttock cleft.
    #[default]
    Body,
    /// Smooth hip-to-ankle tube wrapping both legs for skirts, dresses, and coats.
    Skirt,
}

/// Semantic body partition for part-restricted binding (prevents sleeve-to-torso
/// and leg-to-leg cross-binding).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyPart {
    Trunk = 0,
    RightArm = 1,
    LeftArm = 2,
    RightLeg = 3,
    LeftLeg = 4,
}

impl BodyPart {
    pub const ALL: [BodyPart; 5] = [
        BodyPart::Trunk,
        BodyPart::RightArm,
        BodyPart::LeftArm,
        BodyPart::RightLeg,
        BodyPart::LeftLeg,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(index: usize) -> Self {
        match index {
            1 => BodyPart::RightArm,
            2 => BodyPart::LeftArm,
            3 => BodyPart::RightLeg,
            4 => BodyPart::LeftLeg,
            _ => BodyPart::Trunk,
        }
    }
}

/// Limb root bones used to partition body triangles into [`BodyPart`]s.
const LIMB_ROOTS: [&str; 4] = [
    "RightUpperArm",
    "LeftUpperArm",
    "RightUpperLeg",
    "LeftUpperLeg",
];

/// Outward concavity-bridging rounds on the body proxy.
const BRIDGE_ROUNDS: usize = 10;
/// Maximum outward bridging lift in a concavity (metres).
const BRIDGE_MAX: f32 = 0.028;

/// Fixed barycentric reference to a triangle on the base [`BodyMesh`], so the
/// proxy inherits all body morph targets and pose correctives with zero drift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProxyVertex {
    pub body_triangle: u32,
    pub bary: [f32; 3],
    /// Fixed outward normal offset (metres) used to bridge concavities.
    pub normal_offset: f32,
    pub part: BodyPart,
    /// Optional symmetric mirror anchor `(body_triangle, bary, weight)` so
    /// skirt-tube vertices near the centerline average both legs under morphs.
    pub mirror_anchor: Option<(u32, [f32; 3], f32)>,
    /// Optional smooth skin-weight override (used by `ProxyKind::Skirt` tube
    /// vertices so skirts blend smoothly across `Hips` + both legs instead of
    /// hard-snapping to a single leg).
    pub skin_override: Option<([u16; 4], [f32; 4])>,
}

/// Smooth, fixed-topology stand-in surface driven directly by [`BodyMesh`].
#[derive(Clone, Debug)]
pub struct FitProxy {
    pub kind: ProxyKind,
    pub vertices: Vec<ProxyVertex>,
    pub triangles: Vec<[u32; 3]>,
    pub triangle_parts: Vec<BodyPart>,
}

impl FitProxy {
    /// Builds a smooth [`FitProxy`] (`Body` or `Skirt`) from `body`.
    pub fn from_body(body: &BodyMesh, kind: ProxyKind) -> Self {
        match kind {
            ProxyKind::Body => build_body_proxy(body),
            ProxyKind::Skirt => build_skirt_proxy(body),
        }
    }

    /// Evaluates proxy positions and unit normals at the given body shape
    /// `weights`, plus an optional radial `envelope_offset` (metres) for Layer
    /// Envelopes 0–3.
    pub fn evaluate(
        &self,
        body: &BodyMesh,
        weights: &[(String, f32)],
        envelope_offset: f32,
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let (body_pos, body_norm) = body.shaped(weights);
        let mut positions = Vec::with_capacity(self.vertices.len());
        for v in &self.vertices {
            let [a, b, c] = body.triangles[v.body_triangle as usize].map(|i| i as usize);
            let p0 = body_pos[a] * v.bary[0] + body_pos[b] * v.bary[1] + body_pos[c] * v.bary[2];
            let n0 =
                (body_norm[a] * v.bary[0] + body_norm[b] * v.bary[1] + body_norm[c] * v.bary[2])
                    .normalize_or_zero();
            let pos = if let Some((m_tri, m_bary, m_w)) = v.mirror_anchor {
                let [ma, mb, mc] = body.triangles[m_tri as usize].map(|i| i as usize);
                let pm =
                    body_pos[ma] * m_bary[0] + body_pos[mb] * m_bary[1] + body_pos[mc] * m_bary[2];
                // Keep rest X from p0 while averaging symmetric morph expansion
                // from both legs so the centerline never tears on fat/hip morphs.
                let r0 = body.positions[a] * v.bary[0]
                    + body.positions[b] * v.bary[1]
                    + body.positions[c] * v.bary[2];
                let rm = body.positions[ma] * m_bary[0]
                    + body.positions[mb] * m_bary[1]
                    + body.positions[mc] * m_bary[2];
                let delta = (p0 - r0) * (1.0 - m_w) + (pm - rm) * m_w;
                r0 + delta + n0 * (v.normal_offset + envelope_offset)
            } else {
                p0 + n0 * (v.normal_offset + envelope_offset)
            };
            positions.push(pos);
        }
        let normals = vertex_normals(&positions, &self.triangles);
        (positions, normals)
    }

    /// Evaluates the body's interpolated skin weights at each proxy vertex.
    pub fn vertex_skin_weights(&self, body: &BodyMesh) -> (Vec<[u16; 4]>, Vec<[f32; 4]>) {
        let mut joints = Vec::with_capacity(self.vertices.len());
        let mut weights = Vec::with_capacity(self.vertices.len());
        for v in &self.vertices {
            if let Some((j, w)) = v.skin_override {
                joints.push(j);
                weights.push(w);
                continue;
            }
            let tri = body.triangles[v.body_triangle as usize];
            let mut mix: BTreeMap<u16, f32> = BTreeMap::new();
            for (&corner, &share) in tri.iter().zip(&v.bary) {
                let c = corner as usize;
                for (&joint, &w) in body.joints[c].iter().zip(&body.weights[c]) {
                    if w > 0.0 && share > 0.0 {
                        *mix.entry(joint).or_default() += w * share;
                    }
                }
            }
            let mut sorted: Vec<(u16, f32)> = mix.into_iter().collect();
            sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            sorted.truncate(4);
            let total: f32 = sorted.iter().map(|(_, w)| *w).sum();
            let mut j = [0u16; 4];
            let mut w = [0.0f32; 4];
            if total > 0.0 {
                for (i, (joint, weight)) in sorted.into_iter().enumerate() {
                    j[i] = joint;
                    w[i] = weight / total;
                }
            }
            joints.push(j);
            weights.push(w);
        }
        (joints, weights)
    }

    /// Builds per-part [`Surface`] acceleration structures at `weights` and
    /// `envelope_offset`. Parts with no triangles yield `None`.
    pub fn part_surfaces(
        &self,
        body: &BodyMesh,
        weights: &[(String, f32)],
        envelope_offset: f32,
    ) -> [Option<Surface>; 5] {
        let (positions, normals) = self.evaluate(body, weights, envelope_offset);
        let mut by_part: [Vec<[u32; 3]>; 5] = Default::default();
        for (&tri, &part) in self.triangles.iter().zip(&self.triangle_parts) {
            by_part[part.index()].push(tri);
        }
        by_part.map(|tris| {
            (!tris.is_empty()).then(|| Surface::new(positions.clone(), normals.clone(), tris))
        })
    }

    /// Builds one whole-proxy [`Surface`] at `weights` and `envelope_offset`.
    pub fn surface(
        &self,
        body: &BodyMesh,
        weights: &[(String, f32)],
        envelope_offset: f32,
    ) -> Surface {
        let (positions, normals) = self.evaluate(body, weights, envelope_offset);
        Surface::new(positions, normals, self.triangles.clone())
    }
}

/// Returns the radial clearance (metres) for `layer` (clamped to `0..=3`).
pub fn envelope_offset(layer: u8) -> f32 {
    LAYER_ENVELOPE_OFFSETS[usize::from(layer).min(LAYER_ENVELOPE_OFFSETS.len() - 1)]
}

/// Classifies each body vertex into one of the 5 semantic [`BodyPart`]s by
/// summing its skin weights per limb subtree.
pub(crate) fn classify_body_vertices(body: &BodyMesh) -> Vec<BodyPart> {
    let rig = &body.rig;
    let mut of_bone = vec![0usize; rig.bones.len()];
    for (index, bone) in rig.bones.iter().enumerate() {
        of_bone[index] = match LIMB_ROOTS.iter().position(|limb| bone.name == *limb) {
            Some(limb) => 1 + limb,
            None => bone.parent.map_or(0, |parent| of_bone[parent]),
        };
    }
    body.joints
        .iter()
        .zip(&body.weights)
        .map(|(joints, weights)| {
            let mut totals = [0.0f32; 5];
            for (&joint, &weight) in joints.iter().zip(weights) {
                if let Some(j) = rig.joints.get(usize::from(joint)) {
                    totals[of_bone[j.bone]] += weight;
                }
            }
            let best = (0..5)
                .max_by(|a, b| totals[*a].total_cmp(&totals[*b]))
                .unwrap_or(0);
            BodyPart::from_index(best)
        })
        .collect()
}

/// Builds the smooth body proxy by applying outward-only concavity filling
/// over the welded body mesh so armpits, cleavage, crotch, and buttock cleft
/// are bridged smoothly while convex areas stay flush with the skin.
fn build_body_proxy(body: &BodyMesh) -> FitProxy {
    let count = body.positions.len();
    let vertex_parts = classify_body_vertices(body);

    // Map each body vertex to one incident triangle + barycentric slot so it
    // can be evaluated identically through `ProxyVertex`.
    let mut incident: Vec<(u32, [f32; 3])> = vec![(0, [1.0, 0.0, 0.0]); count];
    let mut has_incident = vec![false; count];
    for (t_idx, tri) in body.triangles.iter().enumerate() {
        for (slot, &v) in tri.iter().enumerate() {
            let vi = v as usize;
            if !has_incident[vi] {
                let mut bary = [0.0f32; 3];
                bary[slot] = 1.0;
                incident[vi] = (t_idx as u32, bary);
                has_incident[vi] = true;
            }
        }
    }

    // Weld coincident vertices for seamless concavity smoothing across UV seams.
    let mut place_map: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let mut of_corner = Vec::with_capacity(count);
    for p in &body.positions {
        let key = (*p * 1e5).round().as_i64vec3().to_array();
        let next = place_map.len() as u32;
        of_corner.push(*place_map.entry(key).or_insert(next));
    }
    let w_count = place_map.len();
    let mut w_pos = vec![Vec3::ZERO; w_count];
    let mut w_norm = vec![Vec3::ZERO; w_count];
    let mut w_hits = vec![0u32; w_count];
    for (i, &w) in of_corner.iter().enumerate() {
        let wi = w as usize;
        w_pos[wi] += body.positions[i];
        w_norm[wi] += body.normals[i];
        w_hits[wi] += 1;
    }
    for wi in 0..w_count {
        if w_hits[wi] > 0 {
            w_pos[wi] /= w_hits[wi] as f32;
            w_norm[wi] = w_norm[wi].normalize_or_zero();
        }
    }
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); w_count];
    for tri in &body.triangles {
        let [a, b, c] = tri.map(|v| of_corner[v as usize]);
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

    // Outward-only concavity fill: if a vertex's neighbour centroid lies
    // outside its normal plane (a crease/hollow), lift it outward along its
    // rest normal, capped at `BRIDGE_MAX`. Convex peaks never shrink.
    let rest_w = w_pos.clone();
    let mut cur = w_pos;
    for _ in 0..BRIDGE_ROUNDS {
        let prev = cur.clone();
        for wi in 0..w_count {
            let list = &neighbours[wi];
            if list.is_empty() || w_norm[wi] == Vec3::ZERO {
                continue;
            }
            let avg = list.iter().map(|&n| prev[n as usize]).sum::<Vec3>() / list.len() as f32;
            let outward = (avg - prev[wi]).dot(w_norm[wi]);
            if outward > 0.0 {
                let candidate = prev[wi] + w_norm[wi] * (outward * 0.5);
                let total = (candidate - rest_w[wi])
                    .dot(w_norm[wi])
                    .clamp(0.0, BRIDGE_MAX);
                cur[wi] = rest_w[wi] + w_norm[wi] * total;
            }
        }
    }

    let vertices: Vec<ProxyVertex> = (0..count)
        .map(|i| {
            let wi = of_corner[i] as usize;
            let offset = (cur[wi] - rest_w[wi])
                .dot(w_norm[wi])
                .clamp(0.0, BRIDGE_MAX);
            let (body_triangle, bary) = incident[i];
            ProxyVertex {
                body_triangle,
                bary,
                normal_offset: offset,
                part: vertex_parts[i],
                mirror_anchor: None,
                skin_override: None,
            }
        })
        .collect();

    let triangle_parts: Vec<BodyPart> = body
        .triangles
        .iter()
        .map(|tri| {
            let mut votes = [0u8; 5];
            for &v in tri {
                votes[vertex_parts[v as usize].index()] += 1;
            }
            let best = (0..5).max_by_key(|&k| votes[k]).unwrap_or(0);
            BodyPart::from_index(best)
        })
        .collect();

    FitProxy {
        kind: ProxyKind::Body,
        vertices,
        triangles: body.triangles.clone(),
        triangle_parts,
    }
}

/// Builds the skirt proxy: starts from the body proxy for the upper body and
/// adds a smooth hip-to-ankle cylindrical skirt tube wrapping both legs, with
/// each tube vertex anchored to the nearest hip/leg body triangle so it
/// expands smoothly with hip, thigh, and fat morphs.
fn build_skirt_proxy(body: &BodyMesh) -> FitProxy {
    let mut proxy = build_body_proxy(body);
    proxy.kind = ProxyKind::Skirt;

    // Locate waist/hip and ankle heights from the rig (fallback to body bounds).
    let rest_world = body.rig.world(&body.rig.rest_pose());
    let bone_y = |name: &str| body.rig.bone(name).map(|b| rest_world[b].w_axis.y);
    let joint_of = |name: &str| -> Option<u16> {
        let bone = body.rig.bone(name)?;
        body.rig
            .joints
            .iter()
            .position(|j| j.bone == bone)
            .map(|i| i as u16)
    };
    let (lo, hi) = body
        .positions
        .iter()
        .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(a, b), p| {
            (a.min(*p), b.max(*p))
        });
    let height = (hi.y - lo.y).max(0.1);
    let top_y = bone_y("Hips").unwrap_or(lo.y + 0.55 * height);
    let bot_y = bone_y("RightFoot")
        .or_else(|| bone_y("LeftFoot"))
        .unwrap_or(lo.y + 0.05 * height);
    if top_y <= bot_y + 0.05 {
        return proxy;
    }

    // Strip all individual leg triangles (LeftLeg / RightLeg) and any lower-body
    // triangles below `top_y` so skirt/dress vertices ONLY see the continuous
    // hip-to-ankle skirt tube below the waist and never split between legs.
    let mut kept_tris = Vec::with_capacity(proxy.triangles.len());
    let mut kept_parts = Vec::with_capacity(proxy.triangle_parts.len());
    for (&tri, &part) in proxy.triangles.iter().zip(&proxy.triangle_parts) {
        if matches!(part, BodyPart::LeftLeg | BodyPart::RightLeg) {
            continue;
        }
        let avg_y = (body.positions[tri[0] as usize].y
            + body.positions[tri[1] as usize].y
            + body.positions[tri[2] as usize].y)
            / 3.0;
        if avg_y < top_y - 0.02 {
            continue;
        }
        kept_tris.push(tri);
        kept_parts.push(part);
    }
    proxy.triangles = kept_tris;
    proxy.triangle_parts = kept_parts;

    let hips_j = joint_of("Hips").unwrap_or(0);
    let l_leg_j = joint_of("LeftUpperLeg").unwrap_or(hips_j);
    let r_leg_j = joint_of("RightUpperLeg").unwrap_or(hips_j);
    let left_sign = body
        .rig
        .bone("LeftUpperLeg")
        .map(|b| (rest_world[b].w_axis.x - (lo.x + hi.x) * 0.5).signum())
        .filter(|s| *s != 0.0)
        .unwrap_or(1.0);

    let body_surf = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );

    const RINGS: usize = 16;
    const SIDES: usize = 32;
    let base_v = proxy.vertices.len() as u32;
    let mid_x = (lo.x + hi.x) * 0.5;
    let mid_z = (lo.z + hi.z) * 0.5;

    // Measure radial envelope around both legs at each height ring.
    let mut ring_radii = vec![[0.12f32; SIDES]; RINGS];
    for (r, radii) in ring_radii.iter_mut().enumerate() {
        let t = r as f32 / (RINGS - 1) as f32;
        let y = top_y + (bot_y - top_y) * t;
        let band = (top_y - bot_y) / RINGS as f32;
        for p in &body.positions {
            if (p.y - y).abs() > band {
                continue;
            }
            let dx = p.x - mid_x;
            let dz = p.z - mid_z;
            let dist = (dx * dx + dz * dz).sqrt();
            let angle = dz.atan2(dx).rem_euclid(std::f32::consts::TAU);
            let sector = ((angle / std::f32::consts::TAU) * SIDES as f32).floor() as usize % SIDES;
            radii[sector] = radii[sector].max(dist + 0.015);
        }
        // Smooth angularly so the skirt tube bridges the gap between legs.
        for _ in 0..3 {
            let prev = *radii;
            for (s, slot) in radii.iter_mut().enumerate() {
                let a = prev[(s + SIDES - 1) % SIDES];
                let b = prev[s];
                let c = prev[(s + 1) % SIDES];
                *slot = (a + 2.0 * b + c) * 0.25;
            }
        }
    }

    for (r, radii) in ring_radii.iter().enumerate() {
        let t = r as f32 / (RINGS - 1) as f32;
        let y = top_y + (bot_y - top_y) * t;
        for (s, &rad) in radii.iter().enumerate() {
            let angle = (s as f32 / SIDES as f32) * std::f32::consts::TAU;
            let p = Vec3::new(mid_x + rad * angle.cos(), y, mid_z + rad * angle.sin());
            let hit = body_surf
                .closest(p, 0.5)
                .or_else(|| body_surf.closest(p, f32::MAX))
                .expect("non-empty body surface");
            let offset = (p - hit.point).dot(hit.normal).max(0.0);

            // Normalized horizontal coordinate: +1 on outer left, 0 at centerline, -1 on outer right.
            let u = ((p.x - mid_x) * left_sign / rad.max(0.05)).clamp(-1.0, 1.0);
            // Mirror anchor for symmetric morph expansion near the centerline.
            let mirror_p = Vec3::new(2.0 * mid_x - p.x, p.y, p.z);
            let mirror_hit = body_surf
                .closest(mirror_p, 0.5)
                .or_else(|| body_surf.closest(mirror_p, f32::MAX))
                .expect("non-empty body surface");
            let mirror_w = 0.5 * (1.0 - u.abs());

            // Smooth symmetric skirt skin weights: Hips + blended Left/Right UpperLeg.
            // At centerline (u == 0) Left == Right (exact 50/50); max single-leg share <= 0.60.
            let hips_w = (0.70 * (1.0 - t).powf(1.5) + 0.20).clamp(0.20, 0.90);
            let leg_total = 1.0 - hips_w;
            let l_share = 0.5 + 0.25 * u;
            let l_w = leg_total * l_share;
            let r_w = leg_total * (1.0 - l_share);
            let skin_override = if l_leg_j == r_leg_j {
                Some(([hips_j, l_leg_j, 0, 0], [hips_w, leg_total, 0.0, 0.0]))
            } else {
                Some(([hips_j, l_leg_j, r_leg_j, 0], [hips_w, l_w, r_w, 0.0]))
            };

            proxy.vertices.push(ProxyVertex {
                body_triangle: hit.triangle,
                bary: hit.bary.to_array(),
                normal_offset: offset,
                part: BodyPart::Trunk,
                mirror_anchor: Some((mirror_hit.triangle, mirror_hit.bary.to_array(), mirror_w)),
                skin_override,
            });
        }
    }

    for r in 0..(RINGS - 1) {
        for s in 0..SIDES {
            let s_next = (s + 1) % SIDES;
            let a = base_v + (r * SIDES + s) as u32;
            let b = base_v + (r * SIDES + s_next) as u32;
            let c = base_v + ((r + 1) * SIDES + s) as u32;
            let d = base_v + ((r + 1) * SIDES + s_next) as u32;
            proxy.triangles.push([a, c, b]);
            proxy.triangles.push([b, c, d]);
            proxy.triangle_parts.push(BodyPart::Trunk);
            proxy.triangle_parts.push(BodyPart::Trunk);
        }
    }

    proxy
}
