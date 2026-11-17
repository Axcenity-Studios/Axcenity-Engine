use std::collections::HashMap;

use super::*;

fn bumpy_of(chunks_x: usize, chunks_z: usize) -> (TerrainData, Placement) {
    let mut data =
        TerrainData::flat(CHUNK_CELLS * chunks_x + 1, CHUNK_CELLS * chunks_z + 1, 0.0).unwrap();
    let (size_x, _) = data.size();
    let all = data.bounds();
    for (i, h) in data.heights_mut_in(all).iter_mut().enumerate() {
        let (x, z) = ((i % size_x) as f32, (i / size_x) as f32);
        *h = 0.5 + 0.3 * (x * 0.4).sin() * (z * 0.3).cos();
    }
    let placement = Placement {
        position: Vec3::ZERO,
        size: Vec3::new(64.0 * chunks_x as f32, 40.0, 64.0 * chunks_z as f32),
    };
    (data, placement)
}

fn bumpy() -> (TerrainData, Placement) {
    bumpy_of(2, 1)
}

/// Every chunk of `data` at its level in `lods` (by chunk, row by row),
/// stitched to its neighbours as the scene does.
fn all_chunks(data: &TerrainData, placement: &Placement, lods: &[u32]) -> Vec<ChunkMesh> {
    let (chunks_x, chunks_z) = data.chunk_counts();
    let drawn = |cx: isize, cz: isize| {
        if cx < 0 || cz < 0 || cx as usize >= chunks_x || cz as usize >= chunks_z {
            return None;
        }
        let (cx, cz) = (cx as usize, cz as usize);
        Some(drawn_lod(data, cx, cz, lods[cz * chunks_x + cx]))
    };
    let mut meshes = Vec::new();
    for cz in 0..chunks_z {
        for cx in 0..chunks_x {
            let lod = drawn(cx as isize, cz as isize).unwrap();
            let neighbours = SIDES.map(|(dx, dz)| drawn(cx as isize + dx, cz as isize + dz));
            meshes.push(chunk_mesh(
                data,
                placement,
                cx,
                cz,
                lod,
                side_lods(lod, neighbours),
            ));
        }
    }
    meshes
}

/// Every triangle edge of `meshes` (by its two corners' exact positions)
/// with how many triangles use it.
fn edge_uses(meshes: &[ChunkMesh]) -> HashMap<[[u32; 3]; 2], usize> {
    let bits = |p: [f32; 3]| p.map(f32::to_bits);
    let mut uses = HashMap::new();
    for mesh in meshes {
        for triangle in mesh.indices.chunks(3) {
            for k in 0..3 {
                let a = bits(mesh.positions[triangle[k] as usize]);
                let b = bits(mesh.positions[triangle[(k + 1) % 3] as usize]);
                *uses.entry(if a < b { [a, b] } else { [b, a] }).or_insert(0) += 1;
            }
        }
    }
    uses
}

/// Whether `edge` runs along the border of a terrain `half` its size across.
fn along_border(edge: [[u32; 3]; 2], half: Vec3) -> bool {
    let [a, b] = edge.map(|p| p.map(f32::from_bits));
    let same = |k: usize, limit: f32| a[k] == b[k] && a[k].abs() == limit;
    same(0, half.x) || same(2, half.z)
}

fn faces_up(mesh: &ChunkMesh) -> bool {
    mesh.indices.chunks(3).all(|triangle| {
        let [a, b, c] = [0, 1, 2].map(|k| Vec3::from(mesh.positions[triangle[k] as usize]));
        (b - a).cross(c - a).y > 0.0
    })
}

#[test]
fn full_detail_has_a_corner_per_sample_and_two_triangles_per_cell() {
    let (data, placement) = bumpy();
    let mesh = chunk_mesh(&data, &placement, 0, 0, 0, [0; 4]);
    let samples = (CHUNK_CELLS + 1) * (CHUNK_CELLS + 1);
    let cells = CHUNK_CELLS * CHUNK_CELLS;
    // Nothing hangs from the edges: the surface is all there is.
    assert_eq!(mesh.positions.len(), samples);
    assert_eq!(mesh.indices.len(), cells * 6);
    assert!(
        mesh.indices
            .iter()
            .all(|&i| (i as usize) < mesh.positions.len())
    );
    // The first corner is the terrain's corner, at its height.
    assert_eq!(mesh.positions[0], [-64.0, data.height(0, 0) * 40.0, -32.0]);
    assert_eq!(mesh.uvs[0], [0.5 / 33.0, 0.5 / 33.0]);
}

#[test]
fn surface_triangles_face_up() {
    let (data, placement) = bumpy();
    assert!(faces_up(&chunk_mesh(&data, &placement, 1, 0, 0, [0; 4])));
    // Stitched to finer neighbours on every side, at every level.
    for lod in 1..LOD_LEVELS {
        for sides in [[0; 4], [0, lod, lod, lod], [lod, 0, 0, lod], [0, 0, lod, 0]] {
            let mesh = chunk_mesh(&data, &placement, 1, 0, lod, sides);
            assert!(faces_up(&mesh), "level {lod}, sides {sides:?}");
        }
    }
}

#[test]
fn neighbouring_chunks_share_their_edge_exactly() {
    let (data, placement) = bumpy();
    let left = chunk_mesh(&data, &placement, 0, 0, 0, [0; 4]);
    let right = chunk_mesh(&data, &placement, 1, 0, 0, [0; 4]);
    let columns = CHUNK_CELLS + 1;
    for row in 0..columns {
        let a = row * columns + CHUNK_CELLS;
        let b = row * columns;
        assert_eq!(left.positions[a], right.positions[b]);
        assert_eq!(left.normals[a], right.normals[b]);
    }
}

#[test]
fn seams_between_levels_of_detail_are_watertight() {
    let (data, placement) = bumpy_of(4, 4);
    // Neighbours one, two and four levels apart, and the same level.
    #[rustfmt::skip]
    let lods = [
        0, 1, 2, 3,
        1, 1, 2, 4,
        2, 3, 3, 4,
        0, 4, 4, 4,
    ];
    let meshes = all_chunks(&data, &placement, &lods);
    for (edge, uses) in edge_uses(&meshes) {
        let border = along_border(edge, placement.size / 2.0);
        // Inside, every edge has a triangle on each side: no crack and no
        // T-junction. Only the terrain's own border has one side open.
        let expected = if border { 1 } else { 2 };
        assert_eq!(
            uses,
            expected,
            "edge {:?} to {:?}",
            edge[0].map(f32::from_bits),
            edge[1].map(f32::from_bits)
        );
    }
    for mesh in &meshes {
        assert!(faces_up(mesh));
    }
}

#[test]
fn the_border_ends_at_the_surface() {
    let (data, placement) = bumpy_of(2, 2);
    let lowest = data.heights().iter().copied().fold(f32::MAX, f32::min);
    for lod in 0..LOD_LEVELS {
        for mesh in all_chunks(&data, &placement, &[lod; 4]) {
            // Nothing hangs below the ground, at the border or anywhere.
            assert!(
                mesh.positions
                    .iter()
                    .all(|p| p[1] >= lowest * placement.size.y - 1e-4)
            );
            assert!(faces_up(&mesh));
        }
    }
}

#[test]
fn a_full_detail_chunk_matches_the_collider_whatever_its_neighbours() {
    let (data, placement) = bumpy();
    let own = chunk_mesh(&data, &placement, 0, 0, 0, [0; 4]);
    // A side is never coarser than its chunk: the coarser neighbour adds
    // the samples instead.
    assert_eq!(own, chunk_mesh(&data, &placement, 0, 0, 0, [3, 2, 1, 4]));
    assert_eq!(side_lods(0, [Some(3), None, Some(0), Some(1)]), [0; 4]);
    assert_eq!(
        side_lods(2, [Some(3), None, Some(0), Some(1)]),
        [2, 2, 0, 1]
    );
}

#[test]
fn a_chunk_one_cell_across_stitches_from_its_middle() {
    // 5 cells past the last whole chunk each way: drawn at 1/16, chunk
    // (1, 1) is one cell.
    let size = CHUNK_CELLS + 6;
    let mut data = TerrainData::flat(size, size, 0.3).unwrap();
    let all = data.bounds();
    for (i, h) in data.heights_mut_in(all).iter_mut().enumerate() {
        *h = 0.3 + (i % 7) as f32 * 0.01;
    }
    let placement = Placement {
        position: Vec3::ZERO,
        size: Vec3::new(74.0, 20.0, 74.0),
    };
    let meshes = all_chunks(&data, &placement, &[0, 0, 0, 4]);
    assert!(meshes.iter().all(faces_up));
    for (edge, uses) in edge_uses(&meshes) {
        let border = along_border(edge, placement.size / 2.0);
        assert_eq!(uses, if border { 1 } else { 2 });
    }
    // Finer on every side, so on opposite sides of its one cell.
    let middle = chunk_mesh(&data, &placement, 1, 1, 4, [0; 4]);
    assert!(faces_up(&middle));
    assert_eq!(middle.indices.len() / 3, 4 * 5);
}

#[test]
fn a_hole_leaves_its_cells_out_at_full_detail_even_far_away() {
    let (mut data, placement) = bumpy();
    let (size_x, _) = data.size();
    let rect = axcenity_core::SampleRect::new(40, 10, 43, 12);
    let holes = data.holes_mut_in(rect);
    for z in 10..12 {
        for x in 40..43 {
            holes[z * size_x + x] = true;
        }
    }
    let cells = CHUNK_CELLS * CHUNK_CELLS;
    // Chunk (1, 0) holds cells 32 to 63 along X: six are cut out, and
    // asked for at a quarter of the detail it's still full.
    assert_eq!(drawn_lod(&data, 1, 0, 2), 0);
    let cut = chunk_mesh(&data, &placement, 1, 0, 2, [2; 4]);
    assert_eq!(cut.indices.len(), (cells - 6) * 6);
    // The chunk beside it is as it was.
    let whole = chunk_mesh(&data, &placement, 0, 0, 0, [0; 4]);
    assert_eq!(whole.indices.len(), cells * 6);
}

#[test]
fn lower_detail_uses_fewer_corners_and_keeps_the_edges() {
    let (data, placement) = bumpy();
    let full = chunk_mesh(&data, &placement, 0, 0, 0, [0; 4]);
    let quarter = chunk_mesh(&data, &placement, 0, 0, 2, [2; 4]);
    assert!(quarter.positions.len() * 8 < full.positions.len());
    // The far corner is still the chunk's corner.
    let columns = CHUNK_CELLS / 4 + 1;
    let last_of_quarter = quarter.positions[columns * columns - 1];
    let last_of_full = full.positions[(CHUNK_CELLS + 1) * (CHUNK_CELLS + 1) - 1];
    assert_eq!(last_of_quarter, last_of_full);
    assert_eq!(lod_for_distance(10.0, 64.0), 0);
    assert_eq!(lod_for_distance(100.0, 64.0), 1);
    assert_eq!(lod_for_distance(300.0, 64.0), 3);
    assert_eq!(lod_for_distance(1e9, 64.0), LOD_LEVELS - 1);
    assert_eq!(chunks_along(CHUNK_CELLS * 2 + 1), 2);
}
