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

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use glam::Vec3;
use rustc_hash::FxHashMap;

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

/// A mesh's skirt made on a worker: its indices with the skirt's after
/// them, for the mesh at `key` as it was at `version`.
pub(super) struct MadeSkirt {
    pub(super) key: u64,
    pub(super) version: u64,
    pub(super) indices: Vec<u32>,
}

/// Skirts made away from the frame (a renderer whose frames are shown):
/// a mesh goes up at once without its skirt, which follows a few frames
/// later. Snow is all it's for, so nothing else waits on it; made in the
/// frame, a mesh of 190,000 triangles held its frame up 30 ms.
pub(super) struct Skirts {
    send: Sender<MadeSkirt>,
    made: Receiver<MadeSkirt>,
}

impl Default for Skirts {
    fn default() -> Self {
        let (send, made) = mpsc::channel();
        Self { send, made }
    }
}

impl Skirts {
    /// Makes `source`'s skirt on rayon's pool. A mesh that needs none
    /// sends nothing.
    pub(super) fn request(&self, source: Arc<dyn crate::MeshSource>) {
        let send = self.send.clone();
        rayon::spawn(move || {
            let Some(vertices) = super::gpu::custom_vertices(source.as_ref()) else {
                return;
            };
            let indices = source.indices();
            let skirt = skirt(&vertices, indices);
            if skirt.is_empty() {
                return;
            }
            let mut all = Vec::with_capacity(indices.len() + skirt.len());
            all.extend_from_slice(indices);
            all.extend_from_slice(&skirt);
            let _ = send.send(MadeSkirt {
                key: source.key(),
                version: source.version(),
                indices: all,
            });
        });
    }

    /// The skirts made since last asked.
    pub(super) fn made(&self) -> Vec<MadeSkirt> {
        self.made.try_iter().collect()
    }
}

/// One edge's corner pairs, each kept once in the order first met. Most
/// edges have one or two, held without an allocation.
#[derive(Default)]
struct Pairs {
    inline: [(u32, u32); 2],
    len: u8,
    more: Vec<(u32, u32)>,
}

impl Pairs {
    fn add(&mut self, pair: (u32, u32)) {
        if self.iter().any(|&seen| seen == pair) {
            return;
        }
        if usize::from(self.len) < self.inline.len() {
            self.inline[usize::from(self.len)] = pair;
            self.len += 1;
        } else {
            self.more.push(pair);
        }
    }

    fn iter(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.inline[..usize::from(self.len)]
            .iter()
            .chain(self.more.iter())
    }
}

/// The skirt of the mesh with these corners and `indices` (three a
/// triangle): three indices a triangle again, to draw after its own.
///
/// Each corner's place is numbered once and each edge found by its two
/// places' numbers in one `u64`, so a mesh of 190,000 triangles takes a few
/// milliseconds: keyed by the places themselves in a SipHash map with a
/// list allocated per edge, it took 380 ms, in the frame that first drew
/// the mesh (2026-11-27, a hitch each time a near level of detail came in).
pub(super) fn skirt(vertices: &[Vertex], indices: &[u32]) -> Vec<u32> {
    if indices
        .iter()
        .any(|&index| index as usize >= vertices.len())
    {
        return Vec::new();
    }
    let places: Vec<[u32; 3]> = vertices.iter().map(place).collect();
    let mut numbers: FxHashMap<[u32; 3], u32> =
        FxHashMap::with_capacity_and_hasher(vertices.len(), Default::default());
    let number: Vec<u32> = places
        .iter()
        .map(|place| {
            let next = numbers.len() as u32;
            *numbers.entry(*place).or_insert(next)
        })
        .collect();
    // Each edge's corner pairs, by its two ends' places, in the order the
    // edges are first met. The pair is kept in the places' order, so two
    // faces' pairs line up end to end.
    let mut edge_of: FxHashMap<u64, u32> =
        FxHashMap::with_capacity_and_hasher(indices.len(), Default::default());
    let mut edges: Vec<Pairs> = Vec::with_capacity(indices.len() / 2);
    for triangle in indices.chunks_exact(3) {
        for corner in 0..3 {
            let (a, b) = (triangle[corner], triangle[(corner + 1) % 3]);
            let (number_a, number_b) = (number[a as usize], number[b as usize]);
            if number_a == number_b {
                continue;
            }
            let (low, high, pair) = if places[a as usize] < places[b as usize] {
                (number_a, number_b, (a, b))
            } else {
                (number_b, number_a, (b, a))
            };
            let key = (u64::from(low) << 32) | u64::from(high);
            let next = edges.len() as u32;
            let edge = *edge_of.entry(key).or_insert(next);
            if edge == next {
                edges.push(Pairs::default());
            }
            edges[edge as usize].add(pair);
        }
    }
    let normal = |index: u32| Vec3::from(vertices[index as usize].normal).normalize_or_zero();
    let position = |index: u32| Vec3::from(vertices[index as usize].position);
    let most = indices.len() * MOST;
    let mut skirt = Vec::new();
    // In the order the mesh's own indices first meet each edge, so the same
    // mesh always gets the same skirt.
    for pairs in &edges {
        for (first, second) in pairs.iter().zip(pairs.iter().skip(1)) {
            let ((a1, b1), (a2, b2)) = (*first, *second);
            let (n1, n2) = (normal(a1) + normal(b1), normal(a2) + normal(b2));
            let together =
                normal(a1).dot(normal(a2)) > SAME_WAY && normal(b1).dot(normal(b2)) > SAME_WAY;
            if together {
                continue;
            }
            // Wound to face out from the face that stays, whichever of the
            // two rises (the same winding serves both).
            let along = position(b1) - position(a1);
            let quad = if n1.cross(along).dot(n2) >= 0.0 {
                [a1, b1, b2, a1, b2, a2]
            } else {
                [a1, b2, b1, a1, a2, b2]
            };
            for corners in quad.chunks_exact(3) {
                // Where the two faces do share a corner, half the quad is
                // a triangle of no area whatever rises.
                if corners[0] != corners[1] && corners[1] != corners[2] && corners[0] != corners[2]
                {
                    skirt.extend_from_slice(corners);
                }
            }
            if skirt.len() > most {
                return Vec::new();
            }
        }
    }
    skirt
}

#[cfg(test)]
mod tests {

    /// The skirt as it was first written (2026-11), kept to check the faster
    /// one gives exactly the same.
    /// The skirt of the mesh with these corners and `indices` (three a
    /// triangle): three indices a triangle again, to draw after its own.
    fn reference_skirt(vertices: &[Vertex], indices: &[u32]) -> Vec<u32> {
        // Each edge's corner pairs, by where its two ends are. The pair is
        // kept in the key's order, so two faces' pairs line up end to end.
        type Edge = ([u32; 3], [u32; 3]);
        let mut edges: std::collections::HashMap<Edge, Vec<(u32, u32)>> =
            std::collections::HashMap::with_capacity(indices.len());
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
                let (place_a, place_b) =
                    (place(&vertices[a as usize]), place(&vertices[b as usize]));
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
                    let together = normal(a1).dot(normal(a2)) > SAME_WAY
                        && normal(b1).dot(normal(b2)) > SAME_WAY;
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

    #[test]
    fn the_skirt_is_exactly_what_it_was() {
        // A cube, a mesh of random faceted triangles sharing places, and a
        // grid with some corners split and some normals bent.
        let mut cases = vec![cube()];
        let mut state = 0x9e37_79b9_u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let grid = 12;
        let mut vertices = Vec::new();
        for z in 0..grid {
            for x in 0..grid {
                let tilt = (next() % 5) as f32 * 0.2;
                vertices.push(vertex(
                    [x as f32, (next() % 3) as f32 * 0.1, z as f32],
                    [tilt, 1.0, 0.0],
                ));
                if next() % 3 == 0 {
                    vertices.push(vertex([x as f32, 0.0, z as f32], [0.0, 0.0, 1.0]));
                }
            }
        }
        let mut indices = Vec::new();
        for _ in 0..600 {
            let pick = |n: u32| n % vertices.len() as u32;
            let (a, b, c) = (pick(next()), pick(next()), pick(next()));
            indices.extend_from_slice(&[a, b, c]);
        }
        cases.push((vertices, indices));
        for (vertices, indices) in &cases {
            let reference = reference_skirt(vertices, indices);
            assert!(!reference.is_empty(), "each case has a skirt to compare");
            assert_eq!(skirt(vertices, indices), reference);
        }
    }

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
