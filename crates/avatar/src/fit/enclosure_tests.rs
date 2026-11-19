//! The enclosure rule and the hole check on synthetic shapes: a tube of
//! cloth over a cylinder of skin, its openings, a gap that opens in a
//! pose, a fold flipped towards the skin, and skin out through the cloth.

use super::*;

/// A tube round the Z axis: `radius`, from `z0` to `z1`, open at both
/// ends, `around` by `along` quads, its normals out (in, and wound to face
/// in, when `inward`).
fn tube(radius: f32, z0: f32, z1: f32, along: u32, inward: bool) -> (Placed, Vec<[u32; 3]>) {
    let around = 32u32;
    let mut placed = Placed::default();
    for j in 0..=along {
        let z = z0 + (z1 - z0) * j as f32 / along as f32;
        for i in 0..around {
            let a = std::f32::consts::TAU * i as f32 / around as f32;
            let n = Vec3::new(a.cos(), a.sin(), 0.0);
            placed.positions.push(n * radius + Vec3::Z * z);
            placed.normals.push(if inward { -n } else { n });
        }
    }
    let mut triangles = Vec::new();
    for j in 0..along {
        for i in 0..around {
            let a = j * around + i;
            let b = j * around + (i + 1) % around;
            let (c, d) = (a + around, b + around);
            // Counter-clockwise seen from outside: a, b, d then a, d, c.
            if inward {
                triangles.push([a, d, b]);
                triangles.push([a, c, d]);
            } else {
                triangles.push([a, b, d]);
                triangles.push([a, d, c]);
            }
        }
    }
    (placed, triangles)
}

/// Two meshes as one: `b`'s corners after `a`'s.
fn joined(a: (Placed, Vec<[u32; 3]>), b: (Placed, Vec<[u32; 3]>)) -> (Placed, Vec<[u32; 3]>) {
    let (mut placed, mut triangles) = a;
    let start = placed.positions.len() as u32;
    placed.positions.extend(b.0.positions);
    placed.normals.extend(b.0.normals);
    triangles.extend(b.1.iter().map(|t| t.map(|v| v + start)));
    (placed, triangles)
}

/// `placed` moved by `by`.
fn moved(placed: &Placed, by: Vec3) -> Placed {
    Placed {
        positions: placed.positions.iter().map(|p| *p + by).collect(),
        normals: placed.normals.clone(),
    }
}

/// The skin: a cylinder 10 cm across, a metre long.
fn arm() -> (Placed, Vec<[u32; 3]>) {
    tube(0.10, -0.5, 0.5, 100, false)
}

/// The middle height of each triangle.
fn heights(placed: &Placed, triangles: &[[u32; 3]]) -> Vec<f32> {
    triangles
        .iter()
        .map(|t| {
            t.iter()
                .map(|v| placed.positions[*v as usize].z)
                .sum::<f32>()
                / 3.0
        })
        .collect()
}

/// The proof: no hidden place shows in `case` from any view direction.
fn holes_in(case: &HideCase, under: &[[u32; 3]], garment: &[[u32; 3]], hidden: &[bool]) -> usize {
    let cloth = garment_surface(&case.garment, garment);
    let skin = drawn_surface(&case.under[0], under, hidden);
    let openings = Openings::new(&case.garment.positions, garment);
    let hidden_triangles: Vec<[u32; 3]> = under
        .iter()
        .zip(hidden)
        .filter(|(_, h)| **h)
        .map(|(t, _)| *t)
        .collect();
    holes_seen(
        &hidden_triangles,
        &case.under[0].positions,
        &case.under[0].normals,
        &[&skin, &cloth],
        &[&cloth],
        &Views::new(&[&openings]),
    )
    .len()
}

/// A tube of cloth 1.5 cm off the skin, 40 cm long: the skin it encloses
/// is hidden, but not within the rule's distance of its openings, nor
/// where it can be seen up or down the tube; past its ends nothing is.
#[test]
fn a_tube_over_a_cylinder_hides_the_skin_it_encloses_away_from_its_openings() {
    let (skin, under) = arm();
    let (cloth, garment) = tube(0.115, -0.2, 0.2, 40, false);
    let case = HideCase {
        under: vec![skin.clone()],
        garment: cloth,
        rest: true,
    };
    let (hidden, report) = hidden_under(&[&under], &garment, 1, |_| case.clone(), HIDE_RULE);
    let z = heights(&skin, &under);
    // How far along the tube from an opening the rule's distance is: the
    // opening's rim is 1.5 cm out from the skin.
    let clear = 0.2 - (HIDE_RULE.opening.powi(2) - 0.015f32.powi(2)).sqrt();
    for (t, z) in z.iter().enumerate() {
        if z.abs() < 0.1 {
            assert!(hidden[0][t], "z {z} is drawn: {report:?}");
        }
        if z.abs() > clear {
            assert!(!hidden[0][t], "z {z} is hidden, past {clear}: {report:?}");
        }
    }
    assert!(report.opening > 0, "{report:?}");
    assert_eq!(holes_in(&case, &under, &garment, &hidden[0]), 0);
}

/// Skin near an opening the distance rule lets through is still kept
/// drawn where a viewer looking up the tube sees it: with no distance
/// from the openings at all, the hole check alone keeps the tube's ends
/// drawn, and nothing hidden shows.
#[test]
fn skin_seen_up_an_opening_stays_drawn() {
    let (skin, under) = arm();
    let (cloth, garment) = tube(0.115, -0.2, 0.2, 40, false);
    let case = HideCase {
        under: vec![skin.clone()],
        garment: cloth,
        rest: true,
    };
    let rule = HideRule {
        opening: 0.0,
        ..HIDE_RULE
    };
    let (hidden, report) = hidden_under(&[&under], &garment, 1, |_| case.clone(), rule);
    let z = heights(&skin, &under);
    // Right at an end, skin 1.5 cm under the rim is seen at a slant.
    for (t, z) in z.iter().enumerate() {
        if z.abs() > 0.195 {
            assert!(!hidden[0][t], "z {z} is hidden: {report:?}");
        }
    }
    assert!(report.seen > 0, "{report:?}");
    assert_eq!(holes_in(&case, &under, &garment, &hidden[0]), 0);
}

/// Two halves of a sleeve overlap at rest; in a pose the lower one slides
/// 5 cm down and a 3 cm ring of skin between them is bare. The skin round
/// the gap (and as far up the upper half as can be seen through it) stays
/// drawn (both with the rule's distances, and with the hole
/// check alone); the skin the halves keep covered in both cases is
/// hidden; and nothing hidden shows in either case.
#[test]
fn a_gap_that_opens_in_a_pose_keeps_the_skin_round_it_drawn() {
    let (skin, under) = arm();
    let upper = tube(0.118, -0.01, 0.4, 41, false);
    let lower = tube(0.113, -0.2, 0.01, 21, false);
    let lower_count = lower.0.positions.len();
    let (cloth, garment) = joined(lower, upper);
    let mut posed = cloth.clone();
    for p in &mut posed.positions[..lower_count] {
        *p -= Vec3::Z * 0.05;
    }
    let cases = [
        HideCase {
            under: vec![skin.clone()],
            garment: cloth,
            rest: true,
        },
        HideCase {
            under: vec![skin.clone()],
            garment: posed,
            rest: false,
        },
    ];
    let z = heights(&skin, &under);
    for rule in [
        HIDE_RULE,
        HideRule {
            opening: 0.0,
            ..HIDE_RULE
        },
    ] {
        let (hidden, report) = hidden_under(&[&under], &garment, 2, |i| cases[i].clone(), rule);
        for (t, z) in z.iter().enumerate() {
            if (-0.045..=-0.005).contains(z) {
                assert!(!hidden[0][t], "z {z} in the gap is hidden: {report:?}");
            }
            // Up a sleeve, skin 1.8 cm under the cloth is seen through
            // its end 9 cm in at a slant: hidden only past that.
            if (0.1..0.3).contains(z) {
                assert!(
                    hidden[0][t],
                    "z {z} under the upper half is drawn: {report:?}"
                );
            }
        }
        for case in &cases {
            assert_eq!(holes_in(case, &under, &garment, &hidden[0]), 0, "{rule:?}");
        }
    }
    // Hiding the gap's skin anyway is caught: the pose shows it.
    let all: Vec<bool> = z.iter().map(|z| (-0.12..0.25).contains(z)).collect();
    assert!(holes_in(&cases[1], &under, &garment, &all) > 0);
    assert_eq!(holes_in(&cases[0], &under, &garment, &all), 0);
}

/// Cloth whose triangles face the skin (a fold flipped in a pose) isn't
/// drawn from outside: it encloses nothing, and skin hidden under it
/// shows.
#[test]
fn cloth_flipped_towards_the_skin_hides_nothing() {
    let (skin, under) = arm();
    let (cloth, garment) = tube(0.115, -0.2, 0.2, 40, true);
    let case = HideCase {
        under: vec![skin.clone()],
        garment: cloth,
        rest: true,
    };
    let (hidden, _) = hidden_under(&[&under], &garment, 1, |_| case.clone(), HIDE_RULE);
    assert!(hidden[0].iter().all(|h| !h));
    let z = heights(&skin, &under);
    let middle: Vec<bool> = z.iter().map(|z| z.abs() < 0.1).collect();
    assert!(holes_in(&case, &under, &garment, &middle) > 0);
}

/// Skin out through the cloth (a corner 5 mm past it) is hidden by the
/// rule when the cloth encloses it at rest, and its place doesn't show in
/// the pose: the cloth facing the viewer just behind it is what is seen.
/// Skin 6 cm past the cloth over it shows.
#[test]
fn skin_out_through_the_cloth_is_hidden_and_the_cloth_behind_it_is_seen() {
    let (skin, under) = arm();
    let (cloth, garment) = tube(0.115, -0.2, 0.2, 40, false);
    let z = heights(&skin, &under);
    let middle: Vec<bool> = z.iter().map(|z| z.abs() < 0.1).collect();
    // The skin swells to 12 cm in the pose (the cloth at 11.5).
    let swollen = Placed {
        positions: skin
            .positions
            .iter()
            .map(|p| Vec3::new(p.x * 1.2, p.y * 1.2, p.z))
            .collect(),
        normals: skin.normals.clone(),
    };
    let cases = [
        HideCase {
            under: vec![skin.clone()],
            garment: cloth.clone(),
            rest: true,
        },
        HideCase {
            under: vec![swollen.clone()],
            garment: cloth.clone(),
            rest: false,
        },
    ];
    let (hidden, report) = hidden_under(&[&under], &garment, 2, |i| cases[i].clone(), HIDE_RULE);
    for (t, z) in z.iter().enumerate() {
        if z.abs() < 0.1 {
            assert!(hidden[0][t], "z {z} drawn: {report:?}");
        }
    }
    assert_eq!(holes_in(&cases[1], &under, &garment, &middle), 0);
    // Far out: nothing of the cloth is within `BEHIND` of it.
    let far = HideCase {
        under: vec![moved(&swollen, Vec3::ZERO)],
        garment: Placed {
            positions: cloth.positions.iter().map(|p| *p * 0.6).collect(),
            normals: cloth.normals.clone(),
        },
        rest: false,
    };
    assert!(holes_in(&far, &under, &garment, &middle) > 0);
}

/// A tube's two open ends are its openings, along its axis; the view
/// directions take in each opening's axis both ways, are unit long, and
/// look through each end at a place inside.
#[test]
fn a_tubes_openings_are_its_ends() {
    let (cloth, garment) = tube(0.115, -0.2, 0.2, 40, false);
    let openings = Openings::new(&cloth.positions, &garment);
    // Both ends look the same way: one axis.
    assert_eq!(openings.axes.len(), 1, "{:?}", openings.axes);
    for axis in &openings.axes {
        assert!(axis.z.abs() > 0.999, "{axis}");
    }
    assert!(openings.within(Vec3::new(0.1, 0.0, 0.19), 0.02));
    assert!(!openings.within(Vec3::new(0.1, 0.0, 0.0), 0.04));
    let directions = view_directions(&openings.axes);
    assert!(directions.iter().all(|d| (d.length() - 1.0).abs() < 1e-4));
    assert!(directions.iter().any(|d| d.z > 0.999));
    assert!(directions.iter().any(|d| d.z < -0.999));
    // Through each end at a place inside: towards its middle, among
    // others, and nothing from more than `APERTURE_REACH` away.
    let views = Views::new(&[&openings]);
    let inside = Vec3::new(0.1, 0.0, 0.1);
    let through: Vec<Vec3> = views.from(inside).skip(views.directions.len()).collect();
    assert!(!through.is_empty());
    assert!(
        through
            .iter()
            .any(|d| d.dot((Vec3::Z * 0.2 - inside).normalize()) > 0.9999)
    );
    let far = Vec3::new(0.1, 0.0, 1.5);
    assert_eq!(views.from(far).count(), views.directions.len());
}

/// A mask: each triangle's middle reads its own value as the engine reads
/// it, and a drawn triangle's texels win along a shared edge.
#[test]
fn a_masks_middles_read_each_triangles_own_value() {
    // Two triangles over the unit square, and a tiny one in a corner.
    let uvs = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.001, 0.001],
        [0.002, 0.001],
        [0.001, 0.002],
    ];
    let triangles = [[0, 1, 2], [0, 2, 3], [4, 5, 6]];
    let hidden = [true, false, true];
    let size = 64;
    let texels = mask_texels(size, &uvs, &triangles, &hidden);
    let at = |uv: [f32; 2]| {
        let x = (uv[0] * size as f32) as u32;
        let y = (uv[1] * size as f32) as u32;
        texels[(y.min(size - 1) * size + x.min(size - 1)) as usize]
    };
    for (t, h) in triangles.iter().zip(hidden) {
        let uv = t.iter().fold([0.0f32; 2], |sum, &v| {
            [
                sum[0] + uvs[v as usize][0] / 3.0,
                sum[1] + uvs[v as usize][1] / 3.0,
            ]
        });
        assert_eq!(at(uv) >= 128, h, "{t:?}");
    }
    // Along the diagonal both share, the drawn one's texels.
    assert_eq!(at([0.5, 0.5]), 0);
    // Deep in the hidden one.
    assert_eq!(at([0.9, 0.1]), 255);
}
