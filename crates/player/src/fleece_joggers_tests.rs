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
//! - **The knee wraps the bent knee's front**: with the knees bent, no
//!   point of the cloth's surface over the front of the knee lies more
//!   than 1 mm inside the skin (sampled inside the triangles, not only at
//!   corners). The measure fails the wave-4 joggers, so it can fail. It
//!   doesn't judge the back of the knee (see `knee_front_failures`).
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

/// The rest pose's knee joint height (m, the masculine and feminine frames
/// share the skeleton: `LeftLowerLeg`'s head).
const KNEE_HEIGHT: f32 = 0.50;
/// The knee's band: cloth whose rest height is within this of the knee.
const KNEE_BAND: f32 = 0.08;
/// How far inside the skin the cloth's surface may lie: the fit gate's
/// poke-through limit (`fit_gate_tests`).
const KNEE_CUT: f32 = 0.001;

/// Points across a triangle, as barycentric weights: its middle, its
/// edges' middles, and one between the middle and each corner. The gate
/// checks corners; a long triangle across a bend cuts the corner of the
/// bend between its corners, so this samples inside the triangles.
const SAMPLES: [[f32; 3]; 7] = [
    [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
    [0.5, 0.5, 0.0],
    [0.0, 0.5, 0.5],
    [0.5, 0.0, 0.5],
    [0.6, 0.2, 0.2],
    [0.2, 0.6, 0.2],
    [0.2, 0.2, 0.6],
];

/// Over the front of one knee (`side` +1 or -1: the cloth whose rest
/// height is within [`KNEE_BAND`] of the knee and whose rest face looks
/// forward), the cloth's surface as `posed` against the posed `body`:
/// how many of its sample points lie more than [`KNEE_CUT`] inside the
/// skin, of how many, and the deepest (m).
fn knee_front_cut(rest: &Posed, posed: &Posed, body: &Posed, side: f32) -> (usize, usize, f32) {
    let skin = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let (mut total, mut inside, mut deepest) = (0, 0, 0.0f32);
    for t in &rest.triangles {
        let r = t.map(|k| rest.positions[k as usize]);
        let middle = (r[0] + r[1] + r[2]) / 3.0;
        let forward = (r[1] - r[0]).cross(r[2] - r[0]).z > 0.0;
        if (middle.y - KNEE_HEIGHT).abs() > KNEE_BAND || middle.x * side < 0.03 || !forward {
            continue;
        }
        let p = t.map(|k| posed.positions[k as usize]);
        for [a, b, c] in SAMPLES {
            let q = p[0] * a + p[1] * b + p[2] * c;
            total += 1;
            if let Some(hit) = skin.closest(q, 0.05) {
                let depth = -hit.signed(q);
                if depth > KNEE_CUT {
                    inside += 1;
                    deepest = deepest.max(depth);
                }
            }
        }
    }
    (total, inside, deepest)
}

/// "Proper leg deformation" at the knee for `item` (drawn as `part`): with
/// the knees bent 135 degrees (the body suite's pose), on every cast body,
/// the cloth wraps the front of each knee: no sample of its surface lies
/// more than [`KNEE_CUT`] inside the skin. Returns the knees that fail.
///
/// This is the side a bend stretches, where too few rings across the knee
/// show as cloth cutting the kneecap's corner. The back of the bent knee
/// is not judged: there the calf meets the thigh and the body's own skin
/// closes the crease, so any cloth in it lies inside the skin (up to 15 mm
/// for these joggers and the wave-4 ones alike) and none of it shows.
fn knee_front_failures(item: &str, part: &str) -> Vec<String> {
    let stage =
        Stage::new(&format!("knee-front-{part}"), &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    for description in bodies().into_iter().filter(|d| d.name.contains("-cast-")) {
        let mut dressed = description.clone();
        dressed.items.push(WornItem::new(item));
        let (_, rest) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bent_body, bent) = worn(&stage.assets, &dressed, Stance::Held(HeldPose::KneesBent));
        let rest = rest.iter().find(|g| g.name == part).unwrap();
        let bent = bent.iter().find(|g| g.name == part).unwrap();
        for side in [1.0f32, -1.0] {
            let (total, inside, deepest) = knee_front_cut(rest, bent, &bent_body, side);
            println!(
                "{part} on {} side {side}: {inside} of {total} samples over the knee's front more than 1 mm inside the skin, deepest {:.1} mm",
                description.name,
                deepest * 1000.0
            );
            assert!(total > 100, "the knee's front has cloth");
            if inside > 0 {
                failures.push(format!(
                    "{} side {side}: {inside} samples, {:.1} mm",
                    description.name,
                    deepest * 1000.0
                ));
            }
        }
    }
    failures
}

#[test]
fn the_fleece_joggers_wrap_the_front_of_the_bent_knee() {
    let failures = knee_front_failures(JOGGERS, PART);
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The knee measure can fail: the wave-4 joggers (`axcenity/realistic-
/// joggers`, a Draft kept as it is), whose knee isn't ringed, cut the
/// front of the bent right knee. If they are ever remade, point this at
/// another garment without knee loops rather than deleting it.
#[test]
fn the_knee_measure_catches_the_wave_4_joggers() {
    let failures = knee_front_failures("axcenity/realistic-joggers", "Pants_Joggers");
    assert!(
        !failures.is_empty(),
        "the measure passed a garment without knee loops"
    );
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
