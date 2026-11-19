//! The first licensed joggers (wave 5, Paul's decision 13: "clean cuffs,
//! consistent geometry and proper leg deformation"): BlenderKit's Grey
//! sweat pants (CC0), refined and fitted by the intake
//! (`assets/characters/realistic/import/`), worn over the underwear by
//! every realistic cast and body benchmark, as a game poses them. Held to
//! `docs/development/character-studio/asset-quality.md` §2 and §3 with
//! **no records**: what a production item must pass.
//!
//! - **Poke-through** (the fit gate's rule, `fit_gate_tests`): no body
//!   corner more than 1 mm outside the joggers where they cover it at
//!   rest, and no underwear through them, at rest, in Idle, in every held
//!   pose (the hips flexed too), a walk frame and the whole stride.
//! - **Levels of detail**: the same at rest for each simpler copy.
//! - **Stability in motion**: Idle, Walk and Run (`garment_checks`).
//! - **The knee bends without collapse**: the joggers' section through
//!   the knee keeps at least the body's own limit of its area (the
//!   body's `knee_135` ring area ratio, 0.6) with the knees bent.
//! - **The waistband stays closed**: its top edge stays near the skin all
//!   round in the walk and with the hips flexed (§3 "Sleeves and waist").

use axcenity_avatar::fit::Surface;
use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, Value};
use axcenity_render::glam::Vec3;

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{
    Failure, Posed, WALK_FRAMES, bodies, check, cover_at_rest, measure, measure_walk, poke_through,
    pose_name, posed_at, undressed, worn,
};
use super::garment_checks::check_motion;

pub(super) const JOGGERS: &str = "axcenity/realistic-fleece-joggers";
/// The part the joggers draw as.
const PART: &str = "Pants_FleeceJoggers";

/// `description` in the joggers, over the underwear its frame wears.
fn dressed(description: &Description) -> Description {
    let mut out = description.clone();
    out.items.push(WornItem::new(JOGGERS));
    out
}

/// A failure that is the joggers' (or a layer through them).
fn ours(failure: &Failure) -> bool {
    failure.garment.contains(PART)
}

/// The gate for `stance` on every realistic cast and body benchmark in
/// the joggers, on the shipped library. Nothing is recorded: any failure
/// fails.
fn joggers_gate(stance: Stance) {
    let label = pose_name(stance);
    let stage =
        Stage::new(&format!("joggers-gate-{label}"), &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&PART), "{}: {names:?}", description.name);
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let cover = cover_at_rest(&body, &garments, &bare);
        let (body, garments) = if stance == Stance::Rest {
            (body, garments)
        } else {
            worn(&stage.assets, &dressed, stance)
        };
        checked += measure(
            &description.name,
            stance,
            &body,
            &garments,
            &cover,
            &mut failures,
        );
    }
    failures.retain(ours);
    println!(
        "joggers gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&label, &failures, &[]);
}

/// The walking fit test (decision 13) through the walk's `frames`.
fn joggers_gate_walk(frames: &[u32]) {
    let label = format!("Walk {frames:?}");
    let stage = Stage::new(
        &format!("joggers-gate-walk-{}", frames[0]),
        &repository_library(),
    )
    .expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        // A walk without the joggers would pass: they must be worn.
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        assert!(names.contains(&PART), "{}: {names:?}", description.name);
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let cover = cover_at_rest(&body, &garments, &bare);
        checked += measure_walk(
            &stage.assets,
            &description.name,
            &dressed,
            &cover,
            frames,
            &mut failures,
        );
    }
    failures.retain(ours);
    println!(
        "joggers gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&label, &failures, &[]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_at_rest() {
    joggers_gate(Stance::Rest);
}

#[test]
fn the_fleece_joggers_cover_the_cast_in_idle() {
    joggers_gate(Stance::Idle);
}

#[test]
fn the_fleece_joggers_cover_the_cast_walking() {
    joggers_gate(Stance::Clip {
        name: "Walk",
        frames: 18,
    });
}

#[test]
fn the_fleece_joggers_cover_the_cast_through_the_walk_first_third() {
    joggers_gate_walk(WALK_FRAMES[0]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_through_the_walk_second_third() {
    joggers_gate_walk(WALK_FRAMES[1]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_through_the_walk_last_third() {
    joggers_gate_walk(WALK_FRAMES[2]);
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_knees_bent() {
    joggers_gate(Stance::Held(HeldPose::KneesBent));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_hips_flexed() {
    joggers_gate(Stance::Held(HeldPose::HipFlex));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_arms_raised() {
    joggers_gate(Stance::Held(HeldPose::ArmsRaised));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_arms_out() {
    joggers_gate(Stance::Held(HeldPose::TPose));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_arms_forward() {
    joggers_gate(Stance::Held(HeldPose::ArmsForward));
}

#[test]
fn the_fleece_joggers_cover_the_cast_with_the_elbows_bent() {
    joggers_gate(Stance::Held(HeldPose::ElbowsBent));
}

/// The body and the joggers at rest, the joggers at level of detail
/// `level`.
fn worn_at_level(stage: &Stage, description: &Description, level: usize) -> (Posed, Vec<Posed>) {
    let (world, animators) =
        stage_world(&stage.assets, description, Stance::Rest, &[]).expect("the stage world");
    let parts = world.all_below(world.workspace(), |class| class == ClassName::MeshPart);
    let named = |name: &str| {
        parts.iter().copied().find(|part| {
            world.get(*part).is_some_and(|p| p.name() == name)
                && matches!(
                    world.get(*part).and_then(|p| p.property("MeshId")),
                    Some(Value::String(id)) if !id.is_empty()
                )
        })
    };
    let body = posed_at(&world, &stage.assets, &animators, named("Body").unwrap(), 0);
    let garments = named(PART)
        .map(|part| posed_at(&world, &stage.assets, &animators, part, level))
        .into_iter()
        .collect();
    (body, garments)
}

/// §2 "LODs": each simpler copy stays clear of the body where the full
/// one covers it, at rest, on every cast and body benchmark.
#[test]
fn the_fleece_joggers_levels_of_detail_stay_clear_of_the_body() {
    let stage = Stage::new("joggers-lods", &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, full) = worn_at_level(&stage, &dressed, 0);
        assert_eq!(full.len(), 1, "{}", description.name);
        let (bare, _) = worn_at_level(&stage, &undressed(&description), 0);
        let cover = cover_at_rest(&body, &full, &bare);
        let covered = &cover.body.iter().find(|(n, _)| n == PART).unwrap().1;
        for level in 1..=2 {
            let (_, simpler) = worn_at_level(&stage, &dressed, level);
            let label = format!("{} LOD{level}", description.name);
            let (count, worst) = poke_through(&label, &body, &simpler[0], covered);
            println!("{label}: {count} out, worst {:.2} mm", worst * 1000.0);
            if count > 0 {
                failures.push(Failure {
                    garment: format!("{PART} LOD{level}"),
                    pose: "rest".to_owned(),
                    body: description.name.clone(),
                    count,
                    worst,
                });
            }
        }
    }
    check("levels of detail", &failures, &[]);
}

/// The realistic cast (not the extremes) in the joggers.
fn cast() -> Vec<Description> {
    bodies()
        .into_iter()
        .filter(|d| d.name.contains("-cast-"))
        .map(|d| dressed(&d))
        .collect()
}

#[test]
fn the_fleece_joggers_stay_steady_in_the_idle_walk_and_run() {
    let stage = Stage::new("joggers-motion", &repository_library()).expect("the stage");
    let cast = cast();
    for description in &cast {
        let (_, garments) = worn(&stage.assets, description, Stance::Rest);
        assert!(
            garments.iter().any(|g| g.name == PART),
            "{}: no joggers",
            description.name
        );
    }
    for clip in ["Idle", "Walk", "Run"] {
        check_motion(&stage, &cast, clip, &[]);
    }
}

/// The body's own limit at the knee (`assets/characters/realistic/qa/
/// thresholds.json`, `knee_135/ring_area_ratio`): what a section through
/// the bent knee keeps of its area at rest.
const KNEE_RING_RATIO: f32 = 0.6;
/// The rest pose's knee joint height (m, the masculine and feminine frames
/// share the skeleton: `LeftLowerLeg`'s head).
const KNEE_HEIGHT: f32 = 0.50;

/// The area of the convex hull of `points` (in a plane through `centre`
/// with normal `normal`).
fn hull_area(points: &[Vec3], centre: Vec3, normal: Vec3) -> f32 {
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    let mut flat: Vec<(f32, f32)> = points
        .iter()
        .map(|p| ((*p - centre).dot(u), (*p - centre).dot(v)))
        .collect();
    flat.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f32, f32)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let points: Vec<(f32, f32)> = if pass == 0 {
            flat.clone()
        } else {
            flat.iter().rev().copied().collect()
        };
        for p in points {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    let mut area = 0.0;
    for k in 0..hull.len() {
        let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
        area += a.0 * b.1 - b.0 * a.1;
    }
    area.abs() / 2.0
}

/// Where the plane through `centre` with `normal` cuts `mesh`'s
/// triangles, keeping the crossings within `reach` of `centre`.
fn section(mesh: &Posed, centre: Vec3, normal: Vec3, reach: f32) -> Vec<Vec3> {
    let mut out = Vec::new();
    for t in &mesh.triangles {
        let p = t.map(|v| mesh.positions[v as usize]);
        for k in 0..3 {
            let (a, b) = (p[k], p[(k + 1) % 3]);
            let (da, db) = ((a - centre).dot(normal), (b - centre).dot(normal));
            if da * db < 0.0 {
                let at = a + (b - a) * (da / (da - db));
                if at.distance(centre) < reach {
                    out.push(at);
                }
            }
        }
    }
    out
}

/// The knee's section on each side, from the body's own corners: the skin
/// corners within 1 cm of the knee's height at rest (one leg), as posed;
/// the plane through them (its normal the leg's direction there: the
/// least spread of the corners) and their middle.
fn knee_plane(rest: &Posed, posed: &Posed, side: f32) -> (Vec3, Vec3) {
    let ring: Vec<usize> = rest
        .positions
        .iter()
        .enumerate()
        .filter(|(_, p)| (p.y - KNEE_HEIGHT).abs() < 0.01 && p.x * side > 0.03)
        .map(|(k, _)| k)
        .collect();
    assert!(ring.len() > 8, "the knee's skin ring");
    let points: Vec<Vec3> = ring.iter().map(|k| posed.positions[*k]).collect();
    let centre = points.iter().copied().sum::<Vec3>() / points.len() as f32;
    // The plane's normal: the direction of least spread (the smallest
    // eigenvector of the covariance), by power iteration on its inverse's
    // stand-in: the largest of (trace - covariance).
    let mut c = [[0.0f32; 3]; 3];
    for p in &points {
        let d = *p - centre;
        let d = [d.x, d.y, d.z];
        for i in 0..3 {
            for j in 0..3 {
                c[i][j] += d[i] * d[j];
            }
        }
    }
    let trace = c[0][0] + c[1][1] + c[2][2];
    let mut n = Vec3::new(0.3, 1.0, 0.2).normalize();
    for _ in 0..64 {
        let m = |r: usize| {
            Vec3::new(
                if r == 0 { trace } else { 0.0 } - c[r][0],
                if r == 1 { trace } else { 0.0 } - c[r][1],
                if r == 2 { trace } else { 0.0 } - c[r][2],
            )
        };
        n = Vec3::new(m(0).dot(n), m(1).dot(n), m(2).dot(n)).normalize();
    }
    (centre, n)
}

/// "Proper leg deformation" (decision 13): with the knees bent 135 degrees
/// (the body suite's pose), the joggers' section through each knee keeps
/// at least [`KNEE_RING_RATIO`] of its area at rest, as the body must, on
/// every cast body.
#[test]
fn the_fleece_joggers_bend_at_the_knee_without_collapsing() {
    let stage = Stage::new("joggers-knee", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in bodies().into_iter().filter(|d| d.name.contains("-cast-")) {
        let dressed = dressed(&description);
        let (rest_body, rest_garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bent_body, bent_garments) =
            worn(&stage.assets, &dressed, Stance::Held(HeldPose::KneesBent));
        let rest = rest_garments.iter().find(|g| g.name == PART).unwrap();
        let bent = bent_garments.iter().find(|g| g.name == PART).unwrap();
        for side in [1.0, -1.0] {
            let (c0, n0) = knee_plane(&rest_body, &rest_body, side);
            let (c1, n1) = knee_plane(&rest_body, &bent_body, side);
            let skin = |b: &Posed, c, n| hull_area(&section(b, c, n, 0.12), c, n);
            let cloth = |g: &Posed, c, n| hull_area(&section(g, c, n, 0.15), c, n);
            let skin_ratio = skin(&bent_body, c1, n1) / skin(&rest_body, c0, n0);
            let ratio = cloth(bent, c1, n1) / cloth(rest, c0, n0);
            println!(
                "{} side {side}: the joggers' knee section keeps {:.3} of its area (the skin's {:.3})",
                description.name, ratio, skin_ratio
            );
            if ratio < KNEE_RING_RATIO {
                problems.push(format!("{} side {side}: {ratio:.3}", description.name));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// How far the waistband's top edge may stand off the skin in a pose
/// (§3 "Sleeves and waist": the band stays closed round the waist, no gap
/// to skin): the boxer briefs' band under it (up to 9 mm off the skin) and
/// the band's own ease, and no more.
const WAIST_GAP: f32 = 0.012;

/// The waistband's top edge: the joggers' corners within 4 mm of their
/// highest at rest.
fn waist_edge(stage: &Stage, description: &Description) -> Vec<usize> {
    let (_, rest) = worn(&stage.assets, description, Stance::Rest);
    let rest = rest.iter().find(|g| g.name == PART).unwrap();
    let top = rest.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    (0..rest.positions.len())
        .filter(|k| rest.positions[*k].y > top - 0.004)
        .collect()
}

/// How far the farthest corner of `edge` is from the skin in `stance`,
/// and where.
fn waist_gap(
    stage: &Stage,
    description: &Description,
    edge: &[usize],
    stance: Stance,
) -> (f32, Vec3) {
    let (body, garments) = worn(&stage.assets, description, stance);
    let posed = garments.iter().find(|g| g.name == PART).unwrap();
    let skin = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let mut worst = (0.0f32, Vec3::ZERO);
    for &k in edge {
        let p = posed.positions[k];
        let gap = skin.closest(p, 0.05).map_or(0.05, |hit| hit.distance);
        if gap > worst.0 {
            worst = (gap, p);
        }
    }
    worst
}

/// The waistband stays closed round the waist (§3 "Sleeves and waist"):
/// in the walk's frames and with the hips flexed, on every cast body, its
/// top edge is never more than [`WAIST_GAP`] off the skin.
#[test]
fn the_fleece_joggers_waistband_stays_closed_in_the_walk_and_the_hip_flex() {
    let stage = Stage::new("joggers-waist", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in bodies().into_iter().filter(|d| d.name.contains("-cast-")) {
        let dressed = dressed(&description);
        let mut stances: Vec<Stance> = WALK_FRAMES
            .iter()
            .flat_map(|frames| frames.iter())
            .map(|frames| Stance::Clip {
                name: "Walk",
                frames: *frames,
            })
            .collect();
        stances.push(Stance::Rest);
        stances.push(Stance::Held(HeldPose::HipFlex));
        let edge = waist_edge(&stage, &dressed);
        let mut worst = (0.0f32, String::new(), Vec3::ZERO);
        for stance in stances {
            let (gap, at) = waist_gap(&stage, &dressed, &edge, stance);
            if gap > worst.0 {
                worst = (gap, pose_name(stance), at);
            }
        }
        println!(
            "{}: the waistband's top edge is at most {:.1} mm off the skin ({} at {})",
            description.name,
            worst.0 * 1000.0,
            worst.1,
            worst.2
        );
        if worst.0 > WAIST_GAP {
            problems.push(format!(
                "{}: {:.1} mm in {}",
                description.name,
                worst.0 * 1000.0,
                worst.1
            ));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
