//! The sides of the snow that lies on an item (`docs/rendering/weather.md`,
//! Snow on what stands in it).
//!
//! Snow with a thickness lifts every corner of a solid item that faces up
//! (shader.wgsl's `item_vertex`, lighting.wgsl's `snow_cap`). A face is
//! lifted whole, but where two faces meet at a hard edge each has its own
//! corners there, with its own normal: the top of a crate rises and its
//! sides stay, which would leave a slit all round, open to what's behind.
//!
//! So every mesh gets a skirt: for each edge whose two faces don't share
//! their corners, two more triangles between the one face's pair of
//! corners and the other's. They use the mesh's own corners, so they add
//! nothing but indices, and while nothing is lifted they have no area and
//! aren't drawn. When one face rises they open into the wall of snow
//! between it and the other.
//!
//! The skirt's indices follow the mesh's own in the same buffer
//! ([`GpuMesh::cap_index_count`](super::types::GpuMesh)), and only the
//! passes that lift corners draw them, and only while snow lies: shadows,
//! ray tracing and the triangle counts never see them.

use std::collections::HashMap;

use glam::Vec3;

use crate::mesh::Vertex;

/// A mesh with more skirt than this many times its own indices gets none:
/// every edge of a faceted mesh is hard, and a skirt three times the mesh
/// is the most a closed one can need.
const MOST: usize = 4;

/// Two corners count as facing the same way from this cosine up, and need
/// no skirt between them: they rise together.
const SAME_WAY: f32 = 0.9995;

/// A corner's place, exactly: split corners are copies of one position.
fn place(vertex: &Vertex) -> [u32; 3] {
    // -0.0 and 0.0 are the same place.
    vertex.position.map(|value| (value + 0.0).to_bits())
}

/// The skirt of the mesh with these corners and `indices` (three a
/// triangle): three indices a triangle again, to draw after its own.
pub(super) fn skirt(vertices: &[Vertex], indices: &[u32]) -> Vec<u32> {
    // Each edge's corner pairs, by where its two ends are. The pair is
    // kept in the key's order, so two faces' pairs line up end to end.
    type Edge = ([u32; 3], [u32; 3]);
    let mut edges: HashMap<Edge, Vec<(u32, u32)>> = HashMap::with_capacity(indices.len());
    for triangle in indices.chunks_exact(3) {
        for corner in 0..3 {
            let (a, b) = (triangle[corner], triangle[(corner + 1) % 3]);
            let (Some(at_a), Some(at_b)) = (vertices.get(a as usize), vertices.get(b as usize))
            else {
                return Vec::new();
            };
            let (place_a, place_b) = (place(at_a), place(at_b));
            if place_a == place_b {
                continue;
            }
            let (key, pair) = if place_a < place_b {
                ((place_a, place_b), (a, b))
            } else {
                ((place_b, place_a), (b, a))
            };
            let pairs = edges.entry(key).or_default();
            if !pairs.contains(&pair) {
                pairs.push(pair);
            }
        }
    }
    let normal = |index: u32| Vec3::from(vertices[index as usize].normal).normalize_or_zero();
    let position = |index: u32| Vec3::from(vertices[index as usize].position);
    let most = indices.len() * MOST;
    let mut skirt = Vec::new();
    // In the order of the mesh's own indices, so the same mesh always
    // gets the same skirt (a map's order changes from run to run).
    let mut seen = std::collections::HashSet::new();
    for triangle in indices.chunks_exact(3) {
        for corner in 0..3 {
            let (a, b) = (triangle[corner], triangle[(corner + 1) % 3]);
            let (place_a, place_b) = (place(&vertices[a as usize]), place(&vertices[b as usize]));
            let key = if place_a < place_b {
                (place_a, place_b)
            } else {
                (place_b, place_a)
            };
            if place_a == place_b || !seen.insert(key) {
                continue;
            }
            let pairs = &edges[&key];
            for (first, second) in pairs.iter().zip(pairs.iter().skip(1)) {
                let ((a1, b1), (a2, b2)) = (*first, *second);
                let (n1, n2) = (normal(a1) + normal(b1), normal(a2) + normal(b2));
                let together =
                    normal(a1).dot(normal(a2)) > SAME_WAY && normal(b1).dot(normal(b2)) > SAME_WAY;
                if together {
                    continue;
                }
                // Wound to face out from the face that stays, whichever
                // of the two rises (the same winding serves both).
                let along = position(b1) - position(a1);
                let quad = if n1.cross(along).dot(n2) >= 0.0 {
                    [a1, b1, b2, a1, b2, a2]
                } else {
                    [a1, b2, b1, a1, a2, b2]
                };
                for corners in quad.chunks_exact(3) {
                    // Where the two faces do share a corner, half the
                    // quad is a triangle of no area whatever rises.
                    if corners[0] != corners[1]
                        && corners[1] != corners[2]
                        && corners[0] != corners[2]
                    {
                        skirt.extend_from_slice(corners);
                    }
                }
                if skirt.len() > most {
                    return Vec::new();
                }
            }
        }
    }
    skirt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::NO_TANGENT;

    fn vertex(position: [f32; 3], normal: [f32; 3]) -> Vertex {
        Vertex {
            position,
            normal,
            uv: [0.0; 2],
            tangent: NO_TANGENT,
        }
    }

    /// A unit cube with its own four corners a face, as the built-in block
    /// has: 24 corners, 12 triangles.
    fn cube() -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (normal, u, v) in [
            (Vec3::X, Vec3::Y, Vec3::Z),
            (Vec3::NEG_X, Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::Z, Vec3::X),
            (Vec3::NEG_Y, Vec3::X, Vec3::Z),
            (Vec3::Z, Vec3::X, Vec3::Y),
            (Vec3::NEG_Z, Vec3::Y, Vec3::X),
        ] {
            let first = vertices.len() as u32;
            for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let at = (normal + u * su + v * sv) * 0.5;
                vertices.push(vertex(at.to_array(), normal.to_array()));
            }
            indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
        (vertices, indices)
    }

    #[test]
    fn a_cube_gets_a_quad_along_each_of_its_twelve_edges() {
        let (vertices, indices) = cube();
        let skirt = skirt(&vertices, &indices);
        assert_eq!(skirt.len(), 12 * 6);
        // Nothing lifted, every skirt triangle has no area.
        for triangle in skirt.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(vertices[triangle[i] as usize].position));
            assert!((b - a).cross(c - a).length() < 1e-6);
        }
        // The same again: it doesn't depend on a map's order.
        assert_eq!(skirt, super::skirt(&vertices, &indices));
    }

    #[test]
    fn a_lifted_top_is_joined_to_the_sides_by_walls_that_face_out() {
        let (mut vertices, indices) = cube();
        let skirt = skirt(&vertices, &indices);
        // The top face's own four corners rise, as snow lifts them.
        for vertex in &mut vertices {
            if vertex.normal[1] > 0.5 {
                vertex.position[1] += 0.25;
            }
        }
        let mut walls = 0.0;
        for triangle in skirt.chunks_exact(3) {
            let [a, b, c] = [0, 1, 2].map(|i| Vec3::from(vertices[triangle[i] as usize].position));
            let face = (b - a).cross(c - a);
            if face.length() < 1e-6 {
                continue;
            }
            let middle = (a + b + c) / 3.0;
            // A wall of the slab: upright, between the old top and the new,
            // facing away from the cube's middle.
            assert!(face.y.abs() < 1e-6, "{face}");
            assert!(middle.y > 0.5 && middle.y < 0.75, "{middle}");
            assert!(
                face.dot(Vec3::new(middle.x, 0.0, middle.z)) > 0.0,
                "{face} at {middle}"
            );
            walls += face.length() * 0.5;
        }
        // Four walls, each as wide as the cube and as high as the lift.
        assert!((walls - 4.0 * 0.25).abs() < 1e-5, "{walls}");
    }

    #[test]
    fn a_smooth_mesh_and_a_seam_of_like_normals_get_no_skirt() {
        // Two triangles sharing their edge's corners: nothing to join.
        let shared = vec![
            vertex([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            vertex([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            vertex([1.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            vertex([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ];
        assert!(skirt(&shared, &[0, 2, 1, 0, 3, 2]).is_empty());
        // The same split along the edge (a texture seam): the copies face
        // the same way, so they rise together.
        let mut seam = shared.clone();
        seam.push(vertex([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
        seam.push(vertex([1.0, 0.0, 1.0], [0.0, 1.0, 0.0]));
        assert!(skirt(&seam, &[0, 2, 1, 4, 3, 5]).is_empty());
        // An index past the corners gives no skirt, and no panic.
        assert!(skirt(&shared, &[0, 2, 9]).is_empty());
    }

    #[test]
    fn a_mesh_whose_every_edge_is_hard_still_gets_its_skirt() {
        // A faceted mesh needs three times its own indices at most.
        let (vertices, indices) = cube();
        assert!(skirt(&vertices, &indices).len() <= indices.len() * MOST);
    }
}
