//! The binding: each garment vertex tied to a point of the body's surface.

use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};

use super::surface::Surface;

/// How far from the body a garment vertex is still bound. Farther ones
/// (none, for a garment made on the body) are bound to the closest point
/// anyway; the report counts them.
pub const BIND_REACH: f32 = 0.25;

/// One garment vertex's place on the body.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bound {
    /// The body triangle.
    pub triangle: u32,
    /// The point on it, as weights of its corners.
    pub bary: [f32; 3],
    /// The vertex's offset from that point in the triangle's frame there:
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

/// The frame of `surface`'s triangle at `bary`: the point, and the
/// columns first edge, across, normal (one unit each, at right angles).
/// The normal is the corners' normals mixed, so neighbouring triangles'
/// frames turn smoothly into each other.
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

/// Binds every point of `garment` to `body`. A point is bound to where
/// a ray from it inward (against its normal in `normals`, when given) meets
/// the body, if that is not much farther than the closest point: a loose
/// sleeve's underside, nearer the torso than the arm inside it, belongs to
/// the arm. Otherwise, and without normals, to the closest point.
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

/// Where `binding`'s vertices are on a body whose corners are at
/// `positions` with `normals` (the body bound to, or it with shapes).
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

/// Each bound vertex's frame on a body at `positions` with `normals`.
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
