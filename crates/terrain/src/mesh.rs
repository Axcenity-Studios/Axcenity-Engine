//! Chunk meshes: what the renderer draws for each chunk of a terrain.
//!
//! Each chunk is built on its own (so an edit rebuilds only its chunks) at
//! one of [`LOD_LEVELS`] levels of detail: level `n` uses every `2^n`th
//! sample. Where a chunk meets a neighbour drawn in more detail, its side
//! takes the neighbour's samples (the cells along it are fanned out to
//! them), so the two meet exactly, with no crack and no T-junction: the
//! surface is watertight across every seam. The terrain's outer border is
//! the surface's own edge; nothing hangs from it.
//!
//! Corners are relative to the terrain's `Position`. Normals come from the
//! whole heightfield, so they match across chunk edges. Each cell is split
//! along the same diagonal as parry's heightfield (from (x, z+1) to
//! (x+1, z)), so at full detail the drawn surface is exactly the one
//! physics collides with. A chunk at full detail is never stitched: only
//! the coarser side of a seam changes.

use axcenity_core::{CHUNK_CELLS, TerrainData};
use glam::Vec3;

use crate::Placement;

/// Full detail, then 1/2, 1/4, 1/8 and 1/16 of the samples.
pub const LOD_LEVELS: u32 = 5;

/// A chunk's sides, in the order [`chunk_mesh`] and [`side_lods`] take
/// them: the step to the neighbour across each, toward -Z, +X, +Z and -X.
pub const SIDES: [(isize, isize); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// One chunk's triangles, with texture coordinates into its splat
/// textures (one texel per sample, texel centres on the samples).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// The level of detail for a chunk whose middle is `distance` studs from
/// the camera: full detail nearby, halving every time the distance
/// doubles past `detail_distance`.
pub fn lod_for_distance(distance: f32, detail_distance: f32) -> u32 {
    if !distance.is_finite() || distance <= detail_distance {
        return 0;
    }
    let level = (distance / detail_distance.max(1.0)).log2().floor() as u32 + 1;
    level.min(LOD_LEVELS - 1)
}

/// The level chunk (`cx`, `cz`) is drawn at when `lod` is asked for: full
/// detail if it has holes, so they keep their shape far away.
pub fn drawn_lod(data: &TerrainData, cx: usize, cz: usize, lod: u32) -> u32 {
    if data.chunk_has_holes(cx, cz) {
        0
    } else {
        lod.min(LOD_LEVELS - 1)
    }
}

/// The level each side of a chunk drawn at `lod` is drawn at, in
/// [`SIDES`] order, from the levels its neighbours are drawn at (`None`
/// past the terrain's border, or for a neighbour that isn't drawn): the
/// finer of the two, so both chunks put the same samples along their seam.
pub fn side_lods(lod: u32, neighbours: [Option<u32>; 4]) -> [u32; 4] {
    neighbours.map(|neighbour| neighbour.map_or(lod, |n| n.min(lod)))
}

/// The mesh of chunk (`cx`, `cz`) at level of detail `lod`, with its
/// sides at `sides` ([`side_lods`]; a side is never coarser than the chunk).
pub fn chunk_mesh(
    data: &TerrainData,
    placement: &Placement,
    cx: usize,
    cz: usize,
    lod: u32,
    sides: [u32; 4],
) -> ChunkMesh {
    let rect = data.chunk_rect(cx, cz);
    let holes = data.chunk_has_holes(cx, cz);
    let lod = drawn_lod(data, cx, cz, lod);
    let step = 1usize << lod;
    let side_steps = sides.map(|side| 1usize << side.min(lod));
    // The samples used along each side: every `step`th, and always the last.
    let along = |start: usize, end: usize| {
        let mut samples: Vec<usize> = (start..end).step_by(step).collect();
        if samples.last() != Some(&(end - 1)) {
            samples.push(end - 1);
        }
        samples
    };
    let xs = along(rect.x0, rect.x1);
    let zs = along(rect.z0, rect.z1);
    let (texels_x, texels_z) = (rect.width() as f32, rect.depth() as f32);
    let cell = placement.cell_size(data);
    let half = placement.size / 2.0;
    let point = |x: usize, z: usize| {
        Vec3::new(
            -half.x + x as f32 * cell.x,
            data.height(x as isize, z as isize) * placement.size.y,
            -half.z + z as f32 * cell.y,
        )
    };
    let uv = |x: usize, z: usize| {
        [
            ((x - rect.x0) as f32 + 0.5) / texels_x,
            ((z - rect.z0) as f32 + 0.5) / texels_z,
        ]
    };
    let add = |mesh: &mut ChunkMesh, x: usize, z: usize| {
        mesh.positions.push(point(x, z).to_array());
        mesh.normals.push(placement.normal(data, x, z).to_array());
        mesh.uvs.push(uv(x, z));
        mesh.positions.len() as u32 - 1
    };

    let mut mesh = ChunkMesh::default();
    for &z in &zs {
        for &x in &xs {
            add(&mut mesh, x, z);
        }
    }
    let (columns, rows) = (xs.len(), zs.len());
    let corner = |i: usize, j: usize| (j * columns + i) as u32;
    // The samples strictly between `from` and `to` on a side drawn every
    // `side_step`th sample (none on a side at the chunk's own step). Cell
    // corners are a multiple of the chunk's step from its start, so of the
    // side's step too.
    let between = |from: usize, to: usize, side_step: usize| -> Vec<usize> {
        if side_step >= step {
            return Vec::new();
        }
        (from + side_step..to).step_by(side_step).collect()
    };
    for j in 0..rows - 1 {
        for i in 0..columns - 1 {
            let (x0, x1, z0, z1) = (xs[i], xs[i + 1], zs[j], zs[j + 1]);
            let (a, b, c, d) = (
                corner(i, j),
                corner(i, j + 1),
                corner(i + 1, j),
                corner(i + 1, j + 1),
            );
            // A hole's cell isn't drawn (at full detail, a corner is a cell).
            if holes && data.is_hole(x0, z0) {
                continue;
            }
            // The samples a finer neighbour adds along this cell's sides.
            let on = |edge: bool, from: usize, to: usize, side: usize| {
                if edge {
                    between(from, to, side_steps[side])
                } else {
                    Vec::new()
                }
            };
            let near = on(j == 0, x0, x1, 0);
            let right = on(i == columns - 2, z0, z1, 1);
            let far = on(j == rows - 2, x0, x1, 2);
            let left = on(i == 0, z0, z1, 3);
            if near.is_empty() && right.is_empty() && far.is_empty() && left.is_empty() {
                // Counter-clockwise from above, split from (x, z+1) to (x+1, z).
                mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
                continue;
            }
            // The cell's outline the same way round as its two triangles
            // (a, b, d, c), with the added samples on its sides.
            let mut outline = vec![a];
            for &z in &left {
                outline.push(add(&mut mesh, x0, z));
            }
            let at_b = outline.len();
            outline.push(b);
            for &x in &far {
                outline.push(add(&mut mesh, x, z1));
            }
            let at_d = outline.len();
            outline.push(d);
            for &z in right.iter().rev() {
                outline.push(add(&mut mesh, x1, z));
            }
            let at_c = outline.len();
            outline.push(c);
            for &x in near.iter().rev() {
                outline.push(add(&mut mesh, x, z0));
            }
            // Fanned from a corner neither of whose sides has samples
            // added (b or c first, keeping the usual diagonal), or else
            // from the cell's middle (a cell with samples added on
            // opposite sides, in a chunk one cell across).
            let apex = [
                (at_b, left.is_empty() && far.is_empty()),
                (at_c, right.is_empty() && near.is_empty()),
                (0, left.is_empty() && near.is_empty()),
                (at_d, far.is_empty() && right.is_empty()),
            ]
            .into_iter()
            .find_map(|(at, free)| free.then_some(at));
            let n = outline.len();
            match apex {
                Some(at) => {
                    for k in 1..n - 1 {
                        mesh.indices.extend_from_slice(&[
                            outline[at],
                            outline[(at + k) % n],
                            outline[(at + k + 1) % n],
                        ]);
                    }
                }
                None => {
                    let corners = [a, b, c, d].map(|k| k as usize);
                    let average = |values: &dyn Fn(usize) -> Vec3| {
                        corners.iter().map(|&k| values(k)).sum::<Vec3>() / 4.0
                    };
                    let middle = average(&|k| Vec3::from(mesh.positions[k]));
                    let normal = average(&|k| Vec3::from(mesh.normals[k])).normalize_or(Vec3::Y);
                    let uv = average(&|k| Vec3::new(mesh.uvs[k][0], mesh.uvs[k][1], 0.0));
                    mesh.positions.push(middle.to_array());
                    mesh.normals.push(normal.to_array());
                    mesh.uvs.push([uv.x, uv.y]);
                    let centre = mesh.positions.len() as u32 - 1;
                    for k in 0..n {
                        mesh.indices
                            .extend_from_slice(&[centre, outline[k], outline[(k + 1) % n]]);
                    }
                }
            }
        }
    }
    mesh
}

/// How many chunk meshes cover a terrain of `samples` samples a side, for tests.
pub fn chunks_along(samples: usize) -> usize {
    (samples - 1).div_ceil(CHUNK_CELLS).max(1)
}

#[cfg(test)]
#[path = "mesh_tests.rs"]
mod tests;
