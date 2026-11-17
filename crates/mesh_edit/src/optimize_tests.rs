//! Group as Mesh's optimizer: what each pass takes away, and that it never
//! takes away what can be seen.

use glam::{Vec2, Vec3};

use crate::optimize::{LookInfo, PartInfo, optimize, used_vertices};
use crate::{Mesh, Surface, Triangle};

/// Parts put together for the optimizer: one mesh, each triangle's part,
/// and whether each part is solid.
#[derive(Default)]
struct Build {
    mesh: Mesh,
    part_of: Vec<u32>,
    parts: Vec<PartInfo>,
}

impl Build {
    /// Adds `mesh` moved by `offset` as a part, every triangle in `look`.
    fn add(&mut self, mesh: Mesh, offset: Vec3, look: u32, solid: bool) -> &mut Self {
        let first = self.mesh.positions.len() as u32;
        for k in 0..mesh.positions.len() {
            self.mesh
                .push_vertex(mesh.positions[k] + offset, mesh.normals[k], mesh.uvs[k]);
        }
        let part = self.parts.len() as u32;
        for triangle in &mesh.triangles {
            self.mesh.triangles.push(Triangle {
                corners: triangle.corners.map(|c| c + first),
                surface: Surface::Outer(look),
            });
            self.part_of.push(part);
        }
        self.parts.push(PartInfo { solid });
        self
    }

    fn block(&mut self, size: [f32; 3], at: [f32; 3], look: u32) -> &mut Self {
        self.add(Mesh::block(Vec3::from(size)), Vec3::from(at), look, true)
    }

    fn optimized(&self, looks: &[LookInfo]) -> Mesh {
        optimize(&self.mesh, &self.part_of, &self.parts, looks).0
    }
}

const PLAIN: LookInfo = LookInfo {
    plain: true,
    double_sided: false,
};
const TEXTURED: LookInfo = LookInfo {
    plain: false,
    double_sided: false,
};

/// A flat square of side `size` in the XZ plane facing up (or down), its
/// own open part, with uvs `uv(corner)` at its corners.
fn square(size: f32, up: bool, uv: impl Fn(Vec2) -> Vec2) -> Mesh {
    let h = size / 2.0;
    let corners = [(-h, -h), (h, -h), (h, h), (-h, h)];
    let mut mesh = Mesh::default();
    let normal = if up { Vec3::Y } else { Vec3::NEG_Y };
    for (x, z) in corners {
        mesh.push_vertex(Vec3::new(x, 0.0, z), normal, uv(Vec2::new(x, z)));
    }
    let triangles = if up {
        [[0, 3, 2], [0, 2, 1]]
    } else {
        [[0, 1, 2], [0, 2, 3]]
    };
    for corners in triangles {
        mesh.triangles.push(Triangle {
            corners,
            surface: Surface::Outer(0),
        });
    }
    mesh
}

fn assert_box(mesh: &Mesh, min: [f32; 3], max: [f32; 3]) {
    let (low, high) = mesh.bounds();
    assert!((low - Vec3::from(min)).abs().max_element() < 1e-5, "{low}");
    assert!(
        (high - Vec3::from(max)).abs().max_element() < 1e-5,
        "{high}"
    );
}

#[test]
fn two_cubes_sharing_a_face_become_one_box_of_12_triangles() {
    let mut build = Build::default();
    build
        .block([1.0; 3], [-0.5, 0.0, 0.0], 0)
        .block([1.0; 3], [0.5, 0.0, 0.0], 0);
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN]);
    assert_eq!(mesh.triangles.len(), 12);
    assert_eq!(report.pressed, 4, "the two faces pressed together");
    assert_eq!(mesh.open_edges(), 0, "no crack");
    assert!((mesh.volume() - 2.0).abs() < 1e-4);
    assert_box(&mesh, [-1.0, -0.5, -0.5], [1.0, 0.5, 0.5]);
}

#[test]
fn a_cube_inside_a_cube_is_gone() {
    let mut build = Build::default();
    build
        .block([4.0; 3], [0.0; 3], 0)
        .block([1.0; 3], [0.3, -0.2, 0.1], 1);
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    assert_eq!(report.hidden, 12);
    assert_eq!(mesh.triangles.len(), 12);
    assert_box(&mesh, [-2.0; 3], [2.0; 3]);
    assert!(
        mesh.triangles
            .iter()
            .all(|t| t.surface == Surface::Outer(0)),
        "only the outer cube's look is left"
    );
}

#[test]
fn a_brick_on_a_plate_loses_only_its_bottom_even_flush_with_the_plate_edge() {
    let mut build = Build::default();
    // The brick's right side is flush with the plate's.
    build
        .block([4.0, 1.0, 4.0], [0.0, 0.5, 0.0], 0)
        .block([1.0, 1.0, 1.0], [1.5, 1.5, 0.0], 1);
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    assert_eq!(report.hidden + report.pressed, 2, "the brick's bottom");
    assert_eq!(mesh.triangles.len(), 22);
    // The plate's top is still whole under it: the brick's sides meet it.
    assert_box(&mesh, [-2.0, 0.0, -2.0], [2.0, 2.0, 2.0]);
}

#[test]
fn a_part_overhanging_a_plate_keeps_its_bottom() {
    let mut build = Build::default();
    build
        .block([4.0, 1.0, 4.0], [0.0, 0.5, 0.0], 0)
        .block([1.0, 1.0, 1.0], [1.75, 1.5, 0.0], 1);
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    assert_eq!(report.hidden + report.pressed, 0, "a quarter of it shows");
    assert_eq!(mesh.triangles.len(), 24);
}

#[test]
fn an_open_plane_touching_a_cube_is_kept_and_never_hides_anything() {
    let mut build = Build::default();
    build.block([2.0; 3], [0.0; 3], 0);
    // On the cube's top, facing up: it can be seen.
    build.add(square(1.0, true, |p| p), Vec3::new(0.0, 1.0, 0.0), 1, false);
    // A cube under a plane lying on another plane: open parts hide nothing.
    build.add(
        square(6.0, true, |p| p),
        Vec3::new(10.0, 0.0, 0.0),
        1,
        false,
    );
    build.block([1.0; 3], [10.0, -0.5, 0.0], 0);
    let (mesh, report) = optimize(
        &build.mesh,
        &build.part_of,
        &build.parts,
        &[PLAIN, TEXTURED],
    );
    assert_eq!(report.hidden + report.pressed, 0);
    assert_eq!(mesh.triangles.len(), 12 + 2 + 2 + 12);
}

#[test]
fn an_open_plane_inside_a_cube_or_facing_into_it_is_hidden() {
    let mut build = Build::default();
    build.block([2.0; 3], [0.0; 3], 0);
    build.add(square(1.0, true, |p| p), Vec3::ZERO, 1, false);
    // On the cube's top but facing into it: pressed against it.
    build.add(
        square(1.0, false, |p| p),
        Vec3::new(0.0, 1.0, 0.0),
        1,
        false,
    );
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    assert_eq!(report.hidden + report.pressed, 4);
    assert_eq!(mesh.triangles.len(), 12);
}

#[test]
fn a_two_sided_card_is_kept_and_a_two_sided_plane_inside_a_cube_is_not() {
    // Two planes back to back: a card seen from both sides. Neither is
    // solid, so neither hides the other.
    let mut build = Build::default();
    build.add(square(1.0, true, |p| p), Vec3::ZERO, 0, false);
    build.add(square(1.0, false, |p| p), Vec3::ZERO, 0, false);
    assert_eq!(build.optimized(&[PLAIN]).triangles.len(), 4);

    let two_sided = LookInfo {
        plain: true,
        double_sided: true,
    };
    let mut build = Build::default();
    build.block([2.0; 3], [0.0; 3], 0);
    build.add(square(1.0, true, |p| p), Vec3::ZERO, 1, false);
    assert_eq!(build.optimized(&[PLAIN, two_sided]).triangles.len(), 12);
    // Poking out of the top, one of its sides shows.
    let mut build = Build::default();
    build.block([2.0; 3], [0.0; 3], 0);
    let mut leaf = square(1.0, true, |p| p);
    for position in &mut leaf.positions {
        *position = Vec3::new(position.x, position.z, 0.0);
    }
    build.add(leaf, Vec3::new(0.0, 1.2, 0.0), 1, false);
    assert_eq!(build.optimized(&[PLAIN, two_sided]).triangles.len(), 14);
}

#[test]
fn same_look_coplanar_squares_merge_and_different_looks_never_do() {
    // A 3 by 3 grid of squares in one look: one square of 2 triangles.
    let mut build = Build::default();
    for x in -1..=1 {
        for z in -1..=1 {
            let at = Vec3::new(x as f32, 0.0, z as f32);
            build.add(square(1.0, true, |p| p), at, 0, false);
        }
    }
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN]);
    assert_eq!(mesh.triangles.len(), 2);
    assert_eq!(report.merged, 16);
    assert_eq!(used_vertices(&mesh), 4);

    // Every other square another look: nothing joins.
    let mut build = Build::default();
    for x in 0..2 {
        build.add(
            square(1.0, true, |p| p),
            Vec3::new(x as f32, 0.0, 0.0),
            x,
            false,
        );
    }
    assert_eq!(build.optimized(&[PLAIN, PLAIN]).triangles.len(), 4);
}

#[test]
fn textured_squares_merge_only_where_the_texture_runs_on_the_same() {
    // The texture runs on across both: they join.
    let mut build = Build::default();
    for x in 0..2 {
        let at = Vec3::new(x as f32, 0.0, 0.0);
        build.add(square(1.0, true, move |p| p + at.truncate()), at, 0, false);
    }
    let merged = build.optimized(&[TEXTURED]);
    assert_eq!(merged.triangles.len(), 2);
    // Its corners keep their uvs: the texture lands where it did.
    for triangle in &merged.triangles {
        for corner in triangle.corners {
            let p = merged.positions[corner as usize];
            let uv = merged.uvs[corner as usize];
            assert!((uv - Vec2::new(p.x, p.z)).length() < 1e-5, "{p} {uv}");
        }
    }

    // A seam: each square has the whole texture. The uvs don't meet, so
    // the seam and both squares stay, with their vertices apart.
    let mut build = Build::default();
    for x in 0..2 {
        let at = Vec3::new(x as f32, 0.0, 0.0);
        build.add(square(1.0, true, |p| p + 0.5), at, 0, false);
    }
    let seam = build.optimized(&[TEXTURED]);
    assert_eq!(seam.triangles.len(), 4);
    assert_eq!(used_vertices(&seam), 8);

    // The uvs meet, but the second square stretches the texture twice as
    // much: filling them again would move it.
    let mut build = Build::default();
    build.add(square(1.0, true, |p| p), Vec3::ZERO, 0, false);
    build.add(
        square(1.0, true, |p| Vec2::new(0.5 + (p.x + 0.5) * 2.0, p.y)),
        Vec3::X,
        0,
        false,
    );
    assert_eq!(build.optimized(&[TEXTURED]).triangles.len(), 4);
}

#[test]
fn hard_edges_and_smooth_faces_keep_their_normals() {
    // A block alone: each face keeps its own corners (a hard edge).
    let mut build = Build::default();
    build.block([1.0; 3], [0.0; 3], 0);
    let mesh = build.optimized(&[PLAIN]);
    assert_eq!(mesh.triangles.len(), 12);
    assert_eq!(used_vertices(&mesh), 24);

    // Two squares side by side with smooth (tilted) normals: never filled
    // again, so their shading can't change.
    let mut build = Build::default();
    for x in 0..2 {
        let mut smooth = square(1.0, true, |p| p);
        for (k, normal) in smooth.normals.iter_mut().enumerate() {
            *normal = Vec3::new(k as f32 * 0.1, 1.0, 0.0).normalize();
        }
        build.add(smooth, Vec3::new(x as f32, 0.0, 0.0), 0, false);
    }
    assert_eq!(build.optimized(&[PLAIN]).triangles.len(), 4);
}

#[test]
fn degenerate_and_duplicate_triangles_go_and_coincident_corners_weld() {
    let mut build = Build::default();
    build.add(square(1.0, true, |p| p), Vec3::ZERO, 0, false);
    // The same square again, a rounding away: its corners weld onto the
    // first's, and its triangles are copies.
    build.add(square(1.0, true, |p| p), Vec3::splat(1e-8), 0, false);
    // A sliver with no area.
    let mut sliver = Mesh::default();
    for x in [0.0, 1.0, 2.0] {
        sliver.push_vertex(Vec3::new(x, 5.0, 0.0), Vec3::Y, Vec2::ZERO);
    }
    sliver.triangles.push(Triangle {
        corners: [0, 1, 2],
        surface: Surface::Outer(0),
    });
    build.add(sliver, Vec3::ZERO, 0, false);
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN]);
    assert_eq!(report.degenerate, 1);
    assert_eq!(report.duplicates, 2);
    assert_eq!(report.welded, 4, "the second square's corners");
    assert_eq!(mesh.triangles.len(), 2);
}

#[test]
fn optimizing_again_changes_nothing() {
    let mut build = Build::default();
    build
        .block([4.0, 1.0, 4.0], [0.0, 0.5, 0.0], 0)
        .block([1.0, 1.0, 1.0], [1.5, 1.5, 0.0], 0)
        .block([1.0, 1.0, 1.0], [-1.5, 1.5, 1.5], 1)
        .block([1.0, 1.0, 1.0], [-1.5, 2.5, 1.5], 1);
    for x in 0..3 {
        let at = Vec3::new(x as f32 + 5.0, 0.0, 0.0);
        build.add(square(1.0, true, move |p| p + at.truncate()), at, 2, false);
    }
    let looks = [PLAIN, PLAIN, TEXTURED];
    let once = build.optimized(&looks);
    let part_of = vec![0; once.triangles.len()];
    let (twice, report) = optimize(&once, &part_of, &[PartInfo { solid: false }], &looks);
    assert_eq!(report, Default::default(), "{report:?}");
    assert_eq!(twice.triangles.len(), once.triangles.len());
    assert_eq!(used_vertices(&twice), used_vertices(&once));
    assert_eq!(twice.bounds(), once.bounds());
    assert!((twice.volume() - once.volume()).abs() < 1e-4);
}

#[test]
fn turned_parts_still_weld_and_press_together() {
    // Two blocks turned 90° about Y: their corners land a rounding apart.
    let turn = glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let mut build = Build::default();
    for x in [-0.5f32, 0.5] {
        let mut block = Mesh::block(Vec3::ONE);
        for position in &mut block.positions {
            *position = turn * *position;
        }
        for normal in &mut block.normals {
            *normal = turn * *normal;
        }
        build.add(block, Vec3::new(x, 0.0, 0.0), 0, true);
    }
    let mesh = build.optimized(&[PLAIN]);
    assert_eq!(mesh.triangles.len(), 12);
    assert_eq!(mesh.open_edges(), 0);
}

/// Grouping a few hundred parts of about 100k triangles: balls in a grid,
/// half sunk into a slab. Prints how long it took; run with `--release
/// --ignored --nocapture` to measure.
#[test]
#[ignore = "a measurement"]
fn a_hundred_thousand_triangles_optimize_quickly() {
    let mut build = Build::default();
    build.block([60.0, 1.0, 60.0], [0.0, -0.5, 0.0], 0);
    for x in 0..17 {
        for z in 0..17 {
            let at = Vec3::new(x as f32 * 3.0 - 24.0, 0.0, z as f32 * 3.0 - 24.0);
            build.add(Mesh::ball(Vec3::splat(2.0)), at, 1, true);
        }
    }
    let triangles = build.mesh.triangles.len();
    let started = std::time::Instant::now();
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    println!(
        "{} parts, {triangles} → {} triangles in {:?}: {report:?}",
        build.parts.len(),
        mesh.triangles.len(),
        started.elapsed()
    );
    assert!(mesh.triangles.len() < triangles);
}

/// A wall of 900 bricks laid flush in two looks: lots of faces pressed
/// together and flat faces to merge. Run like the one above.
#[test]
#[ignore = "a measurement"]
fn a_wall_of_flush_bricks_optimizes_quickly() {
    let mut build = Build::default();
    for x in 0..30 {
        for y in 0..30 {
            let at = [x as f32 * 2.0, y as f32, 0.0];
            build.block([2.0, 1.0, 1.0], at, ((x + y) % 2) as u32);
        }
    }
    let triangles = build.mesh.triangles.len();
    let started = std::time::Instant::now();
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    println!(
        "{} parts, {triangles} → {} triangles in {:?}: {report:?}",
        build.parts.len(),
        mesh.triangles.len(),
        started.elapsed()
    );
    assert_eq!(mesh.open_edges(), 0);
}

#[test]
fn a_wall_laid_in_running_bond_becomes_one_box() {
    // Every other row starts with a half brick, so each brick's corner is
    // halfway along the top of a brick below: T-junctions.
    let mut build = Build::default();
    for row in 0..4 {
        let y = row as f32 + 0.5;
        let widths: &[f32] = if row % 2 == 0 {
            &[2.0, 2.0, 2.0, 2.0]
        } else {
            &[1.0, 2.0, 2.0, 2.0, 1.0]
        };
        let mut x = 0.0;
        for &width in widths {
            build.block([width, 1.0, 1.0], [x + width / 2.0, y, 0.0], 0);
            x += width;
        }
    }
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN]);
    assert_eq!(mesh.triangles.len(), 12, "{report:?}");
    assert_eq!(mesh.open_edges(), 0, "no crack");
    assert!((mesh.volume() - 32.0).abs() < 1e-3);
    assert_box(&mesh, [0.0, 0.0, -0.5], [8.0, 4.0, 0.5]);
}

/// 2,500 bricks standing on 2,500 floor tiles in one plane: the contact
/// test must only look near each triangle. Run like the ones above.
#[test]
#[ignore = "a measurement"]
fn bricks_on_a_tiled_floor_optimize_quickly() {
    let mut build = Build::default();
    for x in 0..50 {
        for z in 0..50 {
            let (x, z) = (x as f32 * 2.0, z as f32 * 2.0);
            build.block([2.0, 0.2, 2.0], [x, -0.1, z], 0);
            build.block([1.0, 1.0, 1.0], [x, 0.5, z], 1);
        }
    }
    let triangles = build.mesh.triangles.len();
    let started = std::time::Instant::now();
    let (mesh, report) = optimize(&build.mesh, &build.part_of, &build.parts, &[PLAIN, PLAIN]);
    println!(
        "{} parts, {triangles} → {} triangles in {:?}: {report:?}",
        build.parts.len(),
        mesh.triangles.len(),
        started.elapsed()
    );
}
