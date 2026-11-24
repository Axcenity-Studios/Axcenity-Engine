//! Smooth normals and merged corners for a garment that came as a
//! generated mesh.
//!
//! Such a mesh gives nearly every triangle corners of its own (the same
//! place and texture coordinates, a normal each): three times the corners
//! the cloth needs. Each one is skinned, carries every body shape and is
//! morphed every frame, so the garment is three times as heavy to draw
//! and to load as it looks, and its faces show as facets. Here each
//! corner's normal is made again from the faces round its place (those
//! within [`CREASE`] of its own faces, so a collar's fold stays a fold),
//! and corners at one place with the same texture coordinates and normal
//! become one.

use std::collections::{BTreeMap, HashMap};

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::{GarmentMesh, GarmentPiece};

/// Faces turned more than this from a corner's own don't count toward its
/// normal (degrees): a sharper turn is a fold of the cloth.
const CREASE: f32 = 80.0;

/// Corners at one place and texture coordinates whose normals are within
/// this of each other become one (degrees).
const SAME: f32 = 12.0;

/// What [`weld`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welding {
    pub corners_before: usize,
    pub corners_after: usize,
}

/// Smooths `garment`'s normals and merges its corners (see the module
/// documentation). Its triangles and their order stay as they are, but
/// one wound against its corners' normals is turned over.
pub(super) fn weld(garment: &mut GarmentMesh) -> Welding {
    let mut welding = Welding::default();
    for piece in &mut garment.pieces {
        welding.corners_before += piece.positions.len();
        weld_piece(piece);
        welding.corners_after += piece.positions.len();
    }
    welding
}

fn weld_piece(piece: &mut GarmentPiece) {
    let count = piece.positions.len();
    if count == 0 || piece.indices.is_empty() {
        return;
    }
    // Corners at one place are one point.
    let mut places: HashMap<[i64; 3], u32> = HashMap::new();
    let point_of: Vec<u32> = piece
        .positions
        .iter()
        .map(|p| {
            let key = (*p * 1e5).round().as_i64vec3().to_array();
            let next = places.len() as u32;
            *places.entry(key).or_insert(next)
        })
        .collect();
    // A triangle wound against its corners' own normals is wound the
    // wrong way (a generated mesh has some): turned over, so its face
    // counts the right way below and it is lit as its neighbours are.
    let had_normals = piece.normals.len() == count;
    if had_normals {
        for t in piece.indices.chunks_exact_mut(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|v| piece.positions[v as usize]);
            let given: Vec3 = t.iter().map(|v| piece.normals[*v as usize]).sum();
            if (b - a).cross(c - a).dot(given) < 0.0 {
                t.swap(1, 2);
            }
        }
    }
    // Each face's normal, twice its area long (a bigger face counts for
    // more), and the faces at each point and at each corner.
    let faces: Vec<Vec3> = piece
        .indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|v| piece.positions[v as usize]);
            (b - a).cross(c - a)
        })
        .collect();
    let mut at_point: Vec<Vec<u32>> = vec![Vec::new(); places.len()];
    let mut own = vec![Vec3::ZERO; count];
    for (face, t) in piece.indices.chunks_exact(3).enumerate() {
        for &v in t {
            at_point[point_of[v as usize] as usize].push(face as u32);
            own[v as usize] += faces[face];
        }
    }
    for list in &mut at_point {
        list.sort_unstable();
        list.dedup();
    }
    let crease = CREASE.to_radians().cos();
    let normals: Vec<Vec3> = (0..count)
        .map(|corner| {
            let toward = own[corner].normalize_or_zero();
            if toward == Vec3::ZERO {
                // No triangle uses it: it goes below.
                return if had_normals {
                    piece.normals[corner]
                } else {
                    Vec3::ZERO
                };
            }
            at_point[point_of[corner] as usize]
                .iter()
                .map(|face| faces[*face as usize])
                .filter(|face| face.normalize_or_zero().dot(toward) >= crease)
                .sum::<Vec3>()
                .normalize_or_zero()
        })
        .collect();

    // Merged: the corners a triangle uses, by place, texture coordinates
    // and normal.
    let same = SAME.to_radians().cos();
    let uv_key = |corner: usize| -> [i64; 2] {
        piece.uvs.get(corner).map_or([0, 0], |uv| {
            [
                (f64::from(uv[0]) * 1e5).round() as i64,
                (f64::from(uv[1]) * 1e5).round() as i64,
            ]
        })
    };
    let mut kept: Vec<usize> = Vec::new();
    let mut groups: BTreeMap<(u32, [i64; 2]), Vec<u32>> = BTreeMap::new();
    const UNUSED: u32 = u32::MAX;
    let mut merged = vec![UNUSED; count];
    for &v in &piece.indices {
        let corner = v as usize;
        if merged[corner] != UNUSED {
            continue;
        }
        let group = groups
            .entry((point_of[corner], uv_key(corner)))
            .or_default();
        let found = group
            .iter()
            .copied()
            .find(|other| normals[kept[*other as usize]].dot(normals[corner]) >= same);
        merged[corner] = match found {
            Some(other) => other,
            None => {
                group.push(kept.len() as u32);
                kept.push(corner);
                kept.len() as u32 - 1
            }
        };
    }
    piece.indices = piece.indices.iter().map(|v| merged[*v as usize]).collect();
    piece.positions = kept.iter().map(|k| piece.positions[*k]).collect();
    if piece.uvs.len() == count {
        piece.uvs = kept.iter().map(|k| piece.uvs[*k]).collect();
    }
    piece.normals = kept.iter().map(|k| normals[*k]).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strip of squares bent along X by `turn` degrees at each column,
    /// every triangle with three corners of its own and a flat normal, as
    /// a generated mesh comes.
    fn faceted(columns: usize, turn: f32) -> GarmentMesh {
        let mut at = Vec::new();
        let (mut x, mut z, mut angle) = (0.0f32, 0.0f32, 0.0f32);
        for _ in 0..=columns {
            at.push((x, z));
            angle += turn.to_radians();
            x += 0.02 * angle.cos();
            z -= 0.02 * angle.sin();
        }
        let mut piece = GarmentPiece {
            material: "Cloth".to_owned(),
            colour: [1.0; 4],
            roughness: 0.8,
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        };
        for column in 0..columns {
            let corner = |c: usize, y: f32| Vec3::new(at[c].0, y, at[c].1);
            let quad = [
                (corner(column, 0.0), [column as f32, 0.0]),
                (corner(column + 1, 0.0), [column as f32 + 1.0, 0.0]),
                (corner(column + 1, 0.02), [column as f32 + 1.0, 1.0]),
                (corner(column, 0.02), [column as f32, 1.0]),
            ];
            for t in [[0, 1, 2], [0, 2, 3]] {
                let [a, b, c] = t.map(|k| quad[k].0);
                let normal = (b - a).cross(c - a).normalize();
                for k in t {
                    piece.indices.push(piece.positions.len() as u32);
                    piece.positions.push(quad[k].0);
                    piece.uvs.push(quad[k].1);
                    piece.normals.push(normal);
                }
            }
        }
        GarmentMesh {
            pieces: vec![piece],
        }
    }

    #[test]
    fn a_faceted_sheet_becomes_a_smooth_one_with_a_corner_a_place() {
        let mut garment = faceted(6, 10.0);
        let before = garment.clone();
        let welding = weld(&mut garment);
        assert_eq!(welding.corners_before, 36);
        // Seven columns of two corners.
        assert_eq!(welding.corners_after, 14);
        let piece = &garment.pieces[0];
        assert_eq!(piece.indices.len(), before.pieces[0].indices.len());
        // The triangles are where they were.
        for (now, was) in piece.indices.iter().zip(&before.pieces[0].indices) {
            assert_eq!(
                piece.positions[*now as usize],
                before.pieces[0].positions[*was as usize]
            );
            assert_eq!(
                piece.uvs[*now as usize],
                before.pieces[0].uvs[*was as usize]
            );
        }
        // A corner between two columns faces between their faces.
        let between = piece
            .positions
            .iter()
            .position(|p| (p.x - before.pieces[0].positions[1].x).abs() < 1e-6 && p.y == 0.0)
            .unwrap();
        let (first, second) = (before.pieces[0].normals[0], before.pieces[0].normals[6]);
        let normal = piece.normals[between];
        assert!(normal.dot(first) > 0.99 && normal.dot(second) > 0.99);
        assert!(
            normal.dot(first) < 0.99999,
            "it kept the first face's normal"
        );
    }

    #[test]
    fn a_triangle_wound_the_wrong_way_is_turned_over() {
        let mut garment = faceted(3, 5.0);
        // The third triangle is wound backwards; its normals are right.
        garment.pieces[0].indices.swap(7, 8);
        let welding = weld(&mut garment);
        assert_eq!(welding.corners_after, 8);
        let piece = &garment.pieces[0];
        for t in piece.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|v| piece.positions[v as usize]);
            let face = (b - a).cross(c - a).normalize();
            for v in t {
                assert!(face.dot(piece.normals[*v as usize]) > 0.9);
            }
        }
    }

    #[test]
    fn a_fold_sharper_than_the_crease_keeps_a_normal_each_side() {
        let mut garment = faceted(2, 100.0);
        let welding = weld(&mut garment);
        // Three columns of two corners, and the middle column twice.
        assert_eq!(welding.corners_after, 8);
    }

    #[test]
    fn corners_with_other_texture_coordinates_stay_apart() {
        let mut garment = faceted(2, 0.0);
        // The second square is another island of the texture.
        for uv in &mut garment.pieces[0].uvs[6..] {
            uv[0] += 10.0;
        }
        let welding = weld(&mut garment);
        assert_eq!(welding.corners_after, 8);
    }
}
