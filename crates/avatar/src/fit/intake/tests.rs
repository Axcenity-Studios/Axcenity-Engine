//! The intake on made-up garments: hollow tubes of cloth with a thickness
//! (an outer wall, an inner wall and the rims that join them), in their
//! own size and place, on the fit tests' tube body.

use std::f32::consts::TAU;

use glam::Vec3;

use super::slices::{Welded, slices};
use super::structure::structure;
use super::*;
use crate::fit::tests::tube_body;

const SIDES: usize = 40;
const RINGS: usize = 24;

/// A hollow tube of cloth about the upright line through (`x`, `z`),
/// from height `low` to `high`: outer wall `outer` from the line, inner
/// wall `inner`, joined at both ends. Wound outward (or inside out).
fn tube(
    x: f32,
    z: f32,
    low: f32,
    high: f32,
    outer: f32,
    inner: f32,
    inside_out: bool,
) -> GarmentPiece {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for (radius, faces) in [(outer, 1.0), (inner, -1.0)] {
        for ring in 0..=RINGS {
            let y = low + (high - low) * ring as f32 / RINGS as f32;
            for side in 0..SIDES {
                let angle = side as f32 / SIDES as f32 * TAU;
                let out = Vec3::new(angle.cos(), 0.0, angle.sin());
                positions.push(Vec3::new(x, y, z) + out * radius);
                normals.push(out * faces);
            }
        }
    }
    let at = |wall: usize, ring: usize, side: usize| {
        (wall * (RINGS + 1) * SIDES + ring * SIDES + side % SIDES) as u32
    };
    let mut indices = Vec::new();
    let mut quad = |a: u32, b: u32, c: u32, d: u32| indices.extend([a, b, c, a, c, d]);
    for ring in 0..RINGS {
        for side in 0..SIDES {
            // The outer wall faces out, the inner wall faces the line.
            quad(
                at(0, ring, side),
                at(0, ring + 1, side),
                at(0, ring + 1, side + 1),
                at(0, ring, side + 1),
            );
            quad(
                at(1, ring, side),
                at(1, ring, side + 1),
                at(1, ring + 1, side + 1),
                at(1, ring + 1, side),
            );
        }
    }
    for side in 0..SIDES {
        // The rims: down at the bottom, up at the top.
        quad(
            at(0, 0, side),
            at(0, 0, side + 1),
            at(1, 0, side + 1),
            at(1, 0, side),
        );
        quad(
            at(0, RINGS, side),
            at(1, RINGS, side),
            at(1, RINGS, side + 1),
            at(0, RINGS, side + 1),
        );
    }
    if inside_out {
        for t in indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
        for n in &mut normals {
            *n = -*n;
        }
    }
    GarmentPiece {
        material: "Cloth".to_owned(),
        colour: [1.0; 4],
        roughness: 0.8,
        positions,
        normals,
        uvs: Vec::new(),
        indices,
    }
}

fn read(garment: &GarmentMesh) -> (Welded, Structure) {
    let mesh = Welded::new(&garment.positions(), &garment.triangles());
    let made = structure(&slices(&mesh, PLANES), &mesh.points);
    (mesh, made)
}

#[test]
fn a_plane_cuts_a_tube_of_cloth_in_two_loops_one_inside_the_other() {
    let garment = GarmentMesh {
        pieces: vec![tube(0.0, 0.0, -1.0, 1.0, 0.5, 0.45, false)],
    };
    let mesh = Welded::new(&garment.positions(), &garment.triangles());
    assert!(mesh.signed_volume() > 0.0);
    let cuts = slices(&mesh, 20);
    assert_eq!(cuts.len(), 20);
    for cut in &cuts {
        assert_eq!(cut.loops.len(), 2, "at {}", cut.y);
        let outer = cut
            .loops
            .iter()
            .find(|l| l.depth == 0)
            .expect("an outer loop");
        let inner = cut
            .loops
            .iter()
            .find(|l| l.depth == 1)
            .expect("an inner loop");
        assert!(outer.closed && inner.closed);
        // Each loop is as long as its wall is round (a 40-sided ring).
        assert!((outer.length - TAU * 0.5).abs() < 0.02, "{}", outer.length);
        assert!((inner.length - TAU * 0.45).abs() < 0.02, "{}", inner.length);
    }
    let thick = shell::thickness(&cuts).expect("a hollow tube has a thickness");
    assert!((thick - 0.05).abs() < 0.003, "{thick}");
}

#[test]
fn the_inner_wall_comes_off_and_the_outer_wall_stays_whole() {
    let garment = GarmentMesh {
        pieces: vec![tube(0.0, 0.0, -1.0, 1.0, 0.5, 0.45, false)],
    };
    let (mesh, made) = read(&garment);
    let cuts = slices(&mesh, PLANES);
    let normals = point_normals(&mesh);
    let inner = shell::inner_wall(&mesh, &cuts, &normals, &made, 0.02);
    for (point, is_inner) in mesh.points.iter().zip(&inner) {
        let from_line = Vec3::new(point.x, 0.0, point.z).length();
        assert_eq!(
            *is_inner,
            from_line < 0.47,
            "a point {from_line} from the line"
        );
    }
    // Without a turned edge only the outer wall's triangles are left.
    let kept = shell::outer_triangles(&mesh, &inner, 0);
    assert_eq!(kept.len(), RINGS * SIDES * 2);
    // With one, the rims too.
    let kept = shell::outer_triangles(&mesh, &inner, 1);
    assert_eq!(kept.len(), RINGS * SIDES * 2 + 2 * SIDES * 2);
}

#[test]
fn trousers_are_a_trunk_over_two_legs_and_a_skirt_is_a_trunk_alone() {
    let trousers = GarmentMesh {
        pieces: vec![
            tube(0.0, 0.0, 0.0, 0.6, 0.5, 0.47, false),
            tube(-0.25, 0.0, -1.2, 0.0, 0.22, 0.2, false),
            tube(0.25, 0.0, -1.2, 0.0, 0.22, 0.2, false),
        ],
    };
    let (_, made) = read(&trousers);
    assert_eq!(guess(&made), GarmentKind::Trousers);
    let crotch = made.crotch().expect("legs part at a crotch");
    assert!(crotch.abs() < 0.02, "{crotch}");
    let [Some(right), Some(left)] = &made.legs else {
        panic!("two legs");
    };
    assert!(right.start.x < 0.0 && left.start.x > 0.0);
    for leg in [right, left] {
        assert!(leg.axis().y < -0.99, "{:?}", leg.axis());
        assert!((leg.length() - 1.2).abs() < 0.06, "{}", leg.length());
        assert!((leg.width - 0.42).abs() < 0.06, "{}", leg.width);
    }
    assert!(made.sleeves.iter().all(Option::is_none));
    // The trunk's rings measure its outside, and its hollow.
    let ring = made.trunk_at(0.3).expect("a trunk ring");
    assert!((ring.length - TAU * 0.5).abs() < 0.03, "{}", ring.length);
    let inside = ring.inside.expect("a hollow trunk");
    assert!((inside - TAU * 0.47).abs() < 0.03, "{inside}");

    let skirt = GarmentMesh {
        pieces: vec![tube(0.0, 0.0, -0.4, 0.4, 0.5, 0.47, false)],
    };
    let (_, made) = read(&skirt);
    assert_eq!(guess(&made), GarmentKind::Skirt);
    assert!(made.crotch().is_none());
}

#[test]
fn a_skirt_in_its_own_size_and_place_is_put_round_the_bodys_waist() {
    let body = tube_body();
    // The body is a tube 0.15 m in radius and 1 m tall. The skirt's file
    // is 20 times too big, off to one side, and wound inside out.
    let skirt = GarmentMesh {
        pieces: vec![tube(7.0, -3.0, 40.0, 48.0, 4.4, 4.0, true)],
    };
    let prepared = prepare(&skirt, &body, &IntakeOptions::default()).expect("a skirt prepares");
    let report = &prepared.report;
    assert_eq!(report.kind, Some(GarmentKind::Skirt));
    assert!(report.guessed && report.flipped);
    // Its hollow goes round the body with the waistband's ease.
    let wanted = 0.15 * (1.0 + WAIST_EASE) / 4.0;
    assert!(
        (report.scale - wanted).abs() < 0.03 * wanted,
        "{}",
        report.scale
    );
    // Only its outer wall and turned edges are left, about the body's
    // middle line, outside the body.
    assert!(report.triangles_after < report.triangles_before * 6 / 10);
    let (lo, hi) = prepared.garment.bounds();
    assert!(
        (lo.x + hi.x).abs() < 0.01 && (lo.z + hi.z).abs() < 0.01,
        "{lo} {hi}"
    );
    assert!(lo.y > 0.0 && hi.y < 1.0, "{lo} {hi}");
    for p in prepared.garment.positions() {
        assert!(Vec3::new(p.x, 0.0, p.z).length() > 0.15, "{p}");
    }
    // A given size is kept.
    let options = IntakeOptions {
        scale: Some(0.05),
        ..IntakeOptions::default()
    };
    let prepared = prepare(&skirt, &body, &options).expect("a skirt prepares");
    assert_eq!(prepared.report.scale, 0.05);
    assert_eq!(prepared.report.sized_by, "given");
}

#[test]
fn a_budget_thins_a_garment() {
    let body = tube_body();
    let skirt = GarmentMesh {
        pieces: vec![tube(0.0, 0.0, 0.4, 0.6, 0.2, 0.19, false)],
    };
    let options = IntakeOptions {
        triangles: Some(400),
        ..IntakeOptions::default()
    };
    let prepared = prepare(&skirt, &body, &options).expect("a skirt prepares");
    let left = prepared.garment.triangles().len();
    assert!(left <= 440, "{left}");
    assert_eq!(prepared.report.triangles_after, left);
    assert_eq!(prepared.garment.normals().len(), prepared.garment.len());
}

#[test]
fn a_garment_with_no_loop_round_its_middle_is_refused() {
    let body = tube_body();
    // Two tubes side by side, nothing between them.
    let garment = GarmentMesh {
        pieces: vec![
            tube(-0.5, 0.0, 0.0, 1.0, 0.2, 0.18, false),
            tube(0.5, 0.0, 0.0, 1.0, 0.2, 0.18, false),
        ],
    };
    assert!(matches!(
        prepare(&garment, &body, &IntakeOptions::default()),
        Err(FitError::Model(_))
    ));
    assert!(matches!(
        prepare(&GarmentMesh::default(), &body, &IntakeOptions::default()),
        Err(FitError::Empty)
    ));
}

/// One wall of cloth round the upright line through the origin, from
/// height `low` to `high`, `radius` from the line, open at both ends.
fn wall(low: f32, high: f32, radius: f32) -> GarmentMesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for ring in 0..=RINGS {
        let y = low + (high - low) * ring as f32 / RINGS as f32;
        for side in 0..SIDES {
            let angle = side as f32 / SIDES as f32 * TAU;
            let out = Vec3::new(angle.cos(), 0.0, angle.sin());
            positions.push(out * radius + Vec3::Y * y);
            normals.push(out);
        }
    }
    let at = |ring: usize, side: usize| (ring * SIDES + side % SIDES) as u32;
    let mut indices = Vec::new();
    for ring in 0..RINGS {
        for side in 0..SIDES {
            let (a, b, c, d) = (
                at(ring, side),
                at(ring + 1, side),
                at(ring + 1, side + 1),
                at(ring, side + 1),
            );
            indices.extend([a, b, c, a, c, d]);
        }
    }
    GarmentMesh {
        pieces: vec![GarmentPiece {
            material: "Cloth".to_owned(),
            colour: [1.0; 4],
            roughness: 0.8,
            positions,
            normals,
            uvs: Vec::new(),
            indices,
        }],
    }
}

fn radii(garment: &GarmentMesh) -> (f32, f32) {
    garment
        .positions()
        .iter()
        .map(|p| Vec3::new(p.x, 0.0, p.z).length())
        .fold((f32::MAX, 0.0), |(lo, hi), r| (lo.min(r), hi.max(r)))
}

/// ADR-0045: a garment standing well off the body is drawn in round it,
/// more the closer its kind is worn, never onto the skin; one that lies
/// within its kind's clearance isn't moved.
#[test]
fn conforming_draws_loose_cloth_in_by_its_kind_and_leaves_snug_cloth() {
    let body = tube_body();
    // The body is 0.15 m in radius: cloth 10 cm off it.
    let loose = wall(0.3, 0.7, 0.25);
    let mut shirt = loose.clone();
    let moved = conform::conform(&mut shirt, &body, Conform::Moderate, None);
    let (lo, hi) = radii(&shirt);
    assert!(moved > 0.03, "{moved}");
    assert!(hi < 0.25 - 0.03, "drawn in: {lo} to {hi}");
    // Not nearer than the clearance a shirt keeps (1.5 cm).
    assert!(lo > 0.15 + 0.014, "{lo}");
    // A jacket keeps more of its build, leggings almost none.
    let mut jacket = loose.clone();
    conform::conform(&mut jacket, &body, Conform::Structured, None);
    let mut leggings = loose.clone();
    conform::conform(&mut leggings, &body, Conform::Strong, None);
    assert!(radii(&jacket).0 > hi, "{:?} against {hi}", radii(&jacket));
    assert!(
        radii(&leggings).1 < lo,
        "{:?} against {lo}",
        radii(&leggings)
    );
    assert!(radii(&leggings).0 > 0.15 + 0.003);
    // As made when asked.
    let mut kept = loose.clone();
    assert_eq!(conform::conform(&mut kept, &body, Conform::None, None), 0.0);
    assert_eq!(kept, loose);
    // Cloth a centimetre off the skin is inside a shirt's clearance.
    let snug = wall(0.3, 0.7, 0.16);
    let mut same = snug.clone();
    assert!(conform::conform(&mut same, &body, Conform::Moderate, None) < 1e-4);
    // A skirt under the crotch keeps its shape, and is drawn in over it.
    let mut skirt = wall(0.2, 0.8, 0.25);
    conform::conform(&mut skirt, &body, Conform::Moderate, Some(0.5));
    for p in skirt.positions() {
        let radius = Vec3::new(p.x, 0.0, p.z).length();
        // (The moves are smoothed over the cloth, so the skirt eases
        // into the part drawn in rather than stepping.)
        if p.y < 0.35 {
            assert!(radius > 0.245, "under the crotch: {p}");
        } else if p.y > 0.62 {
            assert!(radius < 0.24, "over it: {p}");
        }
    }
}

/// Tidying takes off the teeth a cut leaves along a hem and scraps of
/// cloth joined to nothing, and a hem keeps its size.
#[test]
fn tidying_takes_off_teeth_and_scraps_and_keeps_a_hem_its_size() {
    let mut garment = wall(0.3, 0.7, 0.2);
    let whole = garment.triangles().len();
    {
        let piece = &mut garment.pieces[0];
        let first = piece.positions.len() as u32;
        // A tooth: a triangle hanging off the bottom edge by one side.
        piece.positions.push(Vec3::new(0.2, 0.25, 0.01));
        piece.normals.push(Vec3::X);
        piece.indices.extend([0, first, 1]);
        // A scrap: a triangle off on its own.
        for at in [
            Vec3::new(0.5, 0.5, 0.0),
            Vec3::new(0.52, 0.5, 0.0),
            Vec3::new(0.5, 0.52, 0.0),
        ] {
            piece.positions.push(at);
            piece.normals.push(Vec3::Z);
        }
        piece.indices.extend([first + 1, first + 2, first + 3]);
    }
    let tidied = conform::tidy(&mut garment);
    assert_eq!(tidied.scraps, 1);
    assert_eq!(tidied.teeth, 1);
    assert_eq!(garment.triangles().len(), whole);
    let (lo, hi) = radii(&garment);
    assert!(lo > 0.199 && hi < 0.201, "{lo} to {hi}");
}
