//! Surface and part-restricted proxy binding (ADR-0035, ADR-0046).
//!
//! [`bind_proxy`] binds a garment to a smooth [`FitProxy`] (Body or Skirt) with:
//! 1. **Part restriction:** garment vertices are partitioned into the 5 semantic
//!    [`BodyPart`]s (`Trunk`, `RightArm`, `LeftArm`, `RightLeg`, `LeftLeg`) by
//!    diffusing clear proximity votes over the welded garment graph, so a sleeve
//!    never binds to the trunk and one leg never binds to the other.
//! 2. **Confidence gating (Abdrashitov et al., SIGGRAPH Asia 2023):** a vertex
//!    is marked [`VertexBind::Confident`] only when its distance to its part's
//!    proxy surface is within `0.05 * bbox_diag` and its normal agrees within
//!    `35 degrees`. All other vertices (collars, lapels, loose folds, wide hems)
//!    remain [`VertexBind::Unbound`] and are solved smoothly via biharmonic
//!    inpainting ([`super::inpaint`]).

use std::collections::BTreeMap;

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use super::BodyMesh;
use super::proxy::{BodyPart, FitProxy, ProxyKind};
use super::surface::Surface;

/// How far from the body a raw-surface bound vertex is counted as near.
pub const BIND_REACH: f32 = 0.25;
/// Maximum distance ratio (of garment bounding-box diagonal) for a confident
/// proxy match (tuned for folded/wrinkled real-world garments).
pub const CONFIDENT_DIAG_RATIO: f32 = 0.08;
/// Maximum angle (degrees) between garment normal and proxy normal for a
/// confident match (accepts fabric folds/ribs while rejecting back-facing lapels).
pub const CONFIDENT_NORMAL_MAX_DEG: f32 = 65.0;

/// One garment vertex's place on a surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bound {
    /// Triangle index in the bound surface.
    pub triangle: u32,
    /// Barycentric weights of the triangle's three corners.
    pub bary: [f32; 3],
    /// Vertex offset from that point in the triangle's local frame:
    /// along the first edge, across it, and along the normal (metres).
    pub local: [f32; 3],
}

/// Every vertex of a garment bound to a body: what the item manifest's
/// `fit.data` file holds (format 1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub format_version: u32,
    /// The body model it was bound to (a `MeshId` in the library).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
    pub vertices: Vec<Bound>,
}

/// The format version of a binding file.
pub const BINDING_VERSION: u32 = 1;

impl Binding {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Result<Binding, String> {
        serde_json::from_str(text).map_err(|err| err.to_string())
    }
}

/// Result of part-restricted confident proxy binding for one garment vertex.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VertexBind {
    /// Confident match on the vertex's semantic [`BodyPart`] proxy surface.
    Confident(Bound),
    /// Unbound (collar, lapel, loose fold, wide hem): skin weights and morph
    /// deltas are solved smoothly via biharmonic inpainting.
    Unbound,
}

/// Part-restricted confident proxy binding for an entire garment.
#[derive(Clone, Debug)]
pub struct ProxyBinding {
    pub proxy_kind: ProxyKind,
    pub vertices: Vec<VertexBind>,
    /// Assigned semantic [`BodyPart`] per garment corner.
    pub parts: Vec<BodyPart>,
    /// Nearest proxy triangle + barycentric coords on the vertex's assigned
    /// part (unfiltered by distance/normal), used to compute smooth local base
    /// displacements for frame-relative morph inpainting.
    pub nearest_proxy: Vec<(u32, [f32; 3])>,
    pub confident_count: usize,
    pub unbound_count: usize,
}

impl ProxyBinding {
    /// Boolean mask of confident corners for [`super::inpaint::BiharmonicSolver`].
    pub fn confident_mask(&self) -> Vec<bool> {
        self.vertices
            .iter()
            .map(|v| matches!(v, VertexBind::Confident(_)))
            .collect()
    }

    /// Converts this proxy binding into a body-space [`Binding`] (for
    /// `_AXCENITY_SKIN_UV` cover maps and posed-fit diagnostics).
    pub fn to_body_binding(&self, body: &BodyMesh, garment_positions: &[Vec3]) -> Binding {
        let body_surf = Surface::new(
            body.positions.clone(),
            body.normals.clone(),
            body.triangles.clone(),
        );
        bind(&body_surf, garment_positions, &[])
    }
}

/// The frame of a triangle at `bary`: the point, and the columns first edge,
/// across, normal (one unit each, orthonormal).
pub fn frame(positions: &[Vec3], normals: &[Vec3], triangle: [u32; 3], bary: Vec3) -> (Vec3, Mat3) {
    let [i, j, k] = triangle.map(|v| v as usize);
    let point = positions[i] * bary.x + positions[j] * bary.y + positions[k] * bary.z;
    let face = (positions[j] - positions[i]).cross(positions[k] - positions[i]);
    let mixed = normals[i] * bary.x + normals[j] * bary.y + normals[k] * bary.z;
    let normal = mixed
        .try_normalize()
        .or_else(|| face.try_normalize())
        .unwrap_or(Vec3::Y);
    let edge = positions[j] - positions[i];
    let edge = (edge - normal * edge.dot(normal))
        .try_normalize()
        .unwrap_or_else(|| normal.any_orthonormal_vector());
    let across = normal.cross(edge);
    (point, Mat3::from_cols(edge, across, normal))
}

/// Part-restricted confident binding of a garment onto a [`FitProxy`].
/// Never mutates garment geometry.
pub fn bind_proxy(
    proxy: &FitProxy,
    body: &BodyMesh,
    home_weights: &[(String, f32)],
    garment_positions: &[Vec3],
    garment_normals: &[Vec3],
    garment_triangles: &[[u32; 3]],
    exclude_arms: bool,
) -> ProxyBinding {
    let count = garment_positions.len();
    let (lo, hi) = garment_positions
        .iter()
        .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(a, b), p| {
            (a.min(*p), b.max(*p))
        });
    let bbox_diag = (hi - lo).length().max(0.1);
    let max_dist = CONFIDENT_DIAG_RATIO * bbox_diag;
    let min_cos = CONFIDENT_NORMAL_MAX_DEG.to_radians().cos();

    let (proxy_pos, proxy_norm) = proxy.evaluate(body, home_weights, 0.0);
    // Build per-part proxy surfaces while tracking each part triangle's global
    // index in `proxy.triangles`.
    let mut part_tris: [Vec<[u32; 3]>; 5] = Default::default();
    let mut part_global: [Vec<u32>; 5] = Default::default();
    for (g_idx, (&tri, &part)) in proxy
        .triangles
        .iter()
        .zip(&proxy.triangle_parts)
        .enumerate()
    {
        if exclude_arms && matches!(part, BodyPart::RightArm | BodyPart::LeftArm) {
            continue;
        }
        let pi = part.index();
        part_tris[pi].push(tri);
        part_global[pi].push(g_idx as u32);
    }
    let part_surfaces: [Option<Surface>; 5] = std::array::from_fn(|pi| {
        (!part_tris[pi].is_empty())
            .then(|| Surface::new(proxy_pos.clone(), proxy_norm.clone(), part_tris[pi].clone()))
    });
    let whole_surface = Surface::new(
        proxy_pos.clone(),
        proxy_norm.clone(),
        proxy.triangles.clone(),
    );

    // Assign each garment vertex a semantic `BodyPart` by diffusing clear
    // proximity votes over the welded garment graph.
    let parts = assign_garment_parts(garment_positions, garment_triangles, &part_surfaces);

    let mut vertices = Vec::with_capacity(count);
    let mut nearest_proxy = Vec::with_capacity(count);
    let mut confident_count = 0usize;
    let mut unbound_count = 0usize;

    for (k, &p) in garment_positions.iter().enumerate() {
        let part = parts[k];
        let pi = part.index();
        let (global_tri, hit) = match &part_surfaces[pi] {
            Some(surf) => {
                let h = surf
                    .closest(p, 0.6)
                    .or_else(|| surf.closest(p, f32::MAX))
                    .expect("non-empty part surface");
                (part_global[pi][h.triangle as usize], h)
            }
            None => {
                let h = whole_surface
                    .closest(p, f32::MAX)
                    .expect("non-empty proxy surface");
                (h.triangle, h)
            }
        };

        nearest_proxy.push((global_tri, hit.bary.to_array()));

        let normal_ok = garment_normals
            .get(k)
            .and_then(|n| n.try_normalize())
            .is_none_or(|n| n.dot(hit.normal) >= min_cos);

        if hit.distance <= max_dist && normal_ok {
            let tri = proxy.triangles[global_tri as usize];
            let (point, axes) = frame(&proxy_pos, &proxy_norm, tri, hit.bary);
            let local = axes.transpose() * (p - point);
            vertices.push(VertexBind::Confident(Bound {
                triangle: global_tri,
                bary: hit.bary.to_array(),
                local: local.to_array(),
            }));
            confident_count += 1;
        } else {
            vertices.push(VertexBind::Unbound);
            unbound_count += 1;
        }
    }

    ProxyBinding {
        proxy_kind: proxy.kind,
        vertices,
        parts,
        nearest_proxy,
        confident_count,
        unbound_count,
    }
}

/// Partitions garment corners into the 5 semantic [`BodyPart`]s by seeding
/// clearly-closest vertices and diffusing over the welded garment graph.
fn assign_garment_parts(
    positions: &[Vec3],
    triangles: &[[u32; 3]],
    part_surfaces: &[Option<Surface>; 5],
) -> Vec<BodyPart> {
    let count = positions.len();
    // Weld coincident vertices for seamless diffusion across UV seams.
    let mut place_map: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let mut of_corner = Vec::with_capacity(count);
    for &p in positions {
        let key = (p * 1e5).round().as_i64vec3().to_array();
        let next = place_map.len() as u32;
        of_corner.push(*place_map.entry(key).or_insert(next));
    }
    let w_count = place_map.len();
    let mut w_pos = vec![Vec3::ZERO; w_count];
    for (i, &w) in of_corner.iter().enumerate() {
        w_pos[w as usize] = positions[i];
    }
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); w_count];
    for tri in triangles {
        let [a, b, c] = tri.map(|v| of_corner[v as usize] as usize);
        for (u, v) in [(a, b), (b, c), (c, a)] {
            if u != v {
                neighbours[u].push(v);
                neighbours[v].push(u);
            }
        }
    }
    for list in &mut neighbours {
        list.sort_unstable();
        list.dedup();
    }

    let mut shares = vec![[0.0f32; 5]; w_count];
    let mut pinned = vec![false; w_count];
    for (wi, &p) in w_pos.iter().enumerate() {
        let mut dists: Vec<(usize, f32)> = part_surfaces
            .iter()
            .enumerate()
            .filter_map(|(pi, surf)| {
                let hit = surf.as_ref()?.closest(p, 0.6)?;
                Some((pi, hit.distance))
            })
            .collect();
        dists.sort_by(|a, b| a.1.total_cmp(&b.1));
        match dists.as_slice() {
            [] => {
                shares[wi][0] = 1.0;
            }
            [(best, _)] => {
                shares[wi][*best] = 1.0;
                pinned[wi] = true;
            }
            [(best, d0), (_, d1), ..] => {
                shares[wi][*best] = 1.0;
                if *d0 < 0.5 * *d1 {
                    pinned[wi] = true;
                }
            }
        }
    }

    // Diffuse unpinned part shares over the welded graph.
    const DIFFUSE_ROUNDS: usize = 80;
    for _ in 0..DIFFUSE_ROUNDS {
        let prev = shares.clone();
        for wi in 0..w_count {
            if pinned[wi] || neighbours[wi].is_empty() {
                continue;
            }
            let inv = 1.0 / neighbours[wi].len() as f32;
            let mut acc = [0.0f32; 5];
            for &n in &neighbours[wi] {
                for p in 0..5 {
                    acc[p] += prev[n][p] * inv;
                }
            }
            for p in 0..5 {
                shares[wi][p] = prev[wi][p] * 0.3 + acc[p] * 0.7;
            }
        }
    }

    of_corner
        .iter()
        .map(|&w| {
            let s = shares[w as usize];
            let best = (0..5).max_by(|a, b| s[*a].total_cmp(&s[*b])).unwrap_or(0);
            BodyPart::from_index(best)
        })
        .collect()
}

/// Raw nearest-surface binding (retained for body-space `_AXCENITY_SKIN_UV`
/// lookup and test compatibility).
pub fn bind(body: &Surface, garment: &[Vec3], normals: &[Vec3]) -> Binding {
    let vertices = garment
        .iter()
        .enumerate()
        .map(|(k, &p)| {
            let closest = body
                .closest(p, BIND_REACH)
                .or_else(|| body.closest(p, f32::MAX))
                .expect("a surface with triangles has a closest point");
            let inward = normals
                .get(k)
                .and_then(|n| n.try_normalize())
                .and_then(|n| body.cast(p, -n, BIND_REACH))
                .filter(|hit| {
                    hit.distance <= (3.0 * closest.distance).max(closest.distance + 0.03)
                });
            let hit = inward.unwrap_or(closest);
            let triangle = body.triangles[hit.triangle as usize];
            let (point, axes) = frame(&body.positions, &body.normals, triangle, hit.bary);
            let local = axes.transpose() * (p - point);
            Bound {
                triangle: hit.triangle,
                bary: hit.bary.to_array(),
                local: local.to_array(),
            }
        })
        .collect();
    Binding {
        format_version: BINDING_VERSION,
        body: String::new(),
        vertices,
    }
}

/// Where `binding`'s vertices are on a surface whose corners are at `positions`
/// with `normals`.
pub fn place(
    binding: &Binding,
    positions: &[Vec3],
    normals: &[Vec3],
    triangles: &[[u32; 3]],
) -> Vec<Vec3> {
    binding
        .vertices
        .iter()
        .map(|bound| {
            let (point, axes) = frame(
                positions,
                normals,
                triangles[bound.triangle as usize],
                Vec3::from(bound.bary),
            );
            point + axes * Vec3::from(bound.local)
        })
        .collect()
}

/// Each bound vertex's frame on a surface at `positions` with `normals`.
pub(super) fn frames(
    binding: &Binding,
    positions: &[Vec3],
    normals: &[Vec3],
    triangles: &[[u32; 3]],
) -> Vec<(Vec3, Mat3)> {
    binding
        .vertices
        .iter()
        .map(|bound| {
            frame(
                positions,
                normals,
                triangles[bound.triangle as usize],
                Vec3::from(bound.bary),
            )
        })
        .collect()
}
