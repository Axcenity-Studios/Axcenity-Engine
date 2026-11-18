//! The first realistic wardrobe (R7, CO10) on the cast, as a game poses
//! it: the crew-neck tee, the joggers and the sneakers worn together over
//! the underwear by every realistic cast and body benchmark, held to
//! `docs/development/character-studio/asset-quality.md` §3:
//!
//! - **Poke-through** (the fit gate's rule, `fit_gate_tests`): no body
//!   corner more than 1 mm outside a garment where it covers the body at
//!   rest, and no lower layer through an upper one (the joggers' waistband
//!   under the tee's hem, the underwear under each), at rest, in Idle, in
//!   the body suite's held poses and a walk frame.
//! - **Levels of detail**: the same at rest for each garment's simpler
//!   copies (§2 "LODs": no skin through a simpler garment).
//! - **Footwear** and **stability in motion**: `garment_checks`.
//!
//! What a draft still fails is recorded below as an open defect, with its
//! numbers; a production item has no record.

use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, Value};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{
    Failure, Known, Posed, bodies, check, cover_at_rest, measure, poke_through, pose_name,
    posed_at, undressed, worn,
};
use super::garment_checks::{Record, check_motion, footwear, stale, unrecorded};

pub(super) const TEE: &str = "axcenity/realistic-crew-tee";
pub(super) const JOGGERS: &str = "axcenity/realistic-joggers";
pub(super) const SNEAKERS: &str = "axcenity/realistic-sneakers";
/// The parts the three items draw as.
const PARTS: [&str; 3] = ["Shirt_CrewTee", "Pants_Joggers", "Shoes_CourtSneakers"];

/// `description` wearing the three items (over the underwear its frame
/// wears by default).
fn dressed(description: &Description) -> Description {
    let mut out = description.clone();
    for id in [TEE, JOGGERS, SNEAKERS] {
        out.items.push(WornItem::new(id));
    }
    out
}

/// A failure that is one of the three items' (or a layer through one).
fn ours(failure: &Failure) -> bool {
    PARTS.iter().any(|p| failure.garment.contains(p))
}

/// Runs the gate for `stance` on every realistic cast and body benchmark
/// dressed in the three, on the shipped library.
fn wardrobe_gate(stance: Stance, known: &[Known]) {
    let label = pose_name(stance);
    let stage =
        Stage::new(&format!("wardrobe-gate-{label}"), &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        for part in PARTS {
            assert!(names.contains(&part), "{}: {names:?}", description.name);
        }
        // The bare body at rest, to find the skin the items hide (the
        // gate's hole check, CO9 part 2).
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
        "wardrobe gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&label, &failures, known);
}

// What the gate still finds on the three drafts (open defects, not
// accepted behaviour: `refinement/wave4/r7-wardrobe-first.md`). Lower these
// as the items improve; never raise one, never list a production item.

#[test]
fn the_wardrobe_covers_the_cast_at_rest() {
    wardrobe_gate(Stance::Rest, &REST);
}

#[test]
fn the_wardrobe_covers_the_cast_in_idle() {
    wardrobe_gate(Stance::Idle, &IDLE);
}

#[test]
fn the_wardrobe_covers_the_cast_walking() {
    wardrobe_gate(
        Stance::Clip {
            name: "Walk",
            frames: 18,
        },
        &WALK,
    );
}

#[test]
fn the_wardrobe_covers_the_cast_with_the_arms_out() {
    wardrobe_gate(Stance::Held(HeldPose::TPose), &T_POSE);
}

#[test]
fn the_wardrobe_covers_the_cast_with_the_arms_raised() {
    wardrobe_gate(Stance::Held(HeldPose::ArmsRaised), &ARMS_RAISED);
}

#[test]
fn the_wardrobe_covers_the_cast_with_the_elbows_bent() {
    wardrobe_gate(Stance::Held(HeldPose::ElbowsBent), &ELBOWS_BENT);
}

#[test]
fn the_wardrobe_covers_the_cast_with_the_knees_bent() {
    wardrobe_gate(Stance::Held(HeldPose::KneesBent), &KNEES_BENT);
}

/// The body and the three garments' parts at rest, the garments at their
/// level of detail `level`.
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
    let garments = PARTS
        .iter()
        .filter_map(|name| named(name))
        .map(|part| posed_at(&world, &stage.assets, &animators, part, level))
        .collect();
    (body, garments)
}

/// §2 "LODs": each simpler copy of a garment stays clear of the body
/// where the full one covers it (the body drawn in full: the strictest
/// pairing), at rest on every cast and body benchmark.
#[test]
fn the_wardrobe_levels_of_detail_stay_clear_of_the_body() {
    let stage = Stage::new("wardrobe-lods", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, full) = worn_at_level(&stage, &dressed, 0);
        assert_eq!(full.len(), PARTS.len(), "{}", description.name);
        let (bare, _) = worn_at_level(&stage, &undressed(&description), 0);
        let cover = cover_at_rest(&body, &full, &bare);
        for level in 1..=2 {
            let (_, simpler) = worn_at_level(&stage, &dressed, level);
            for garment in &simpler {
                let Some((_, covered)) = cover_at_rest_of(&cover, &garment.name) else {
                    continue;
                };
                let label = format!("{} LOD{level}", description.name);
                let (count, worst) = poke_through(&label, &body, garment, covered);
                if count > 0 {
                    problems.push(Problem {
                        what: format!("{} LOD{level}", garment.name),
                        body: description.name.clone(),
                        count,
                        worst,
                    });
                }
            }
        }
    }
    for p in &problems {
        println!(
            "  {} {}: {} vertices out, worst {:.2} mm",
            p.body,
            p.what,
            p.count,
            p.worst * 1000.0
        );
    }
    let failures: Vec<Failure> = problems
        .into_iter()
        .map(|p| Failure {
            garment: p.what,
            pose: "rest".to_owned(),
            body: p.body,
            count: p.count,
            worst: p.worst,
        })
        .collect();
    check("levels of detail", &failures, &LODS);
}

struct Problem {
    what: String,
    body: String,
    count: usize,
    worst: f32,
}

/// The body corners `name` covers at rest, from a [`cover_at_rest`].
fn cover_at_rest_of<'a>(
    cover: &'a super::fit_gate_tests::Cover,
    name: &str,
) -> Option<&'a (String, Vec<bool>)> {
    cover.body.iter().find(|(n, _)| n == name)
}

/// The realistic cast (not the extremes) dressed in the three, each
/// checked to wear all three (a motion check of a character missing one
/// would pass it).
fn cast(stage: &Stage) -> Vec<Description> {
    let cast: Vec<Description> = bodies()
        .into_iter()
        .filter(|d| d.name.contains("-cast-"))
        .map(|d| dressed(&d))
        .collect();
    for description in &cast {
        let (_, garments) = worn(&stage.assets, description, Stance::Rest);
        let names: Vec<&str> = garments.iter().map(|g| g.name.as_str()).collect();
        for part in PARTS {
            assert!(names.contains(&part), "{}: {names:?}", description.name);
        }
    }
    cast
}

#[test]
fn the_wardrobe_stays_steady_in_the_idle() {
    let stage = Stage::new("wardrobe-motion-idle", &repository_library()).expect("the stage");
    check_motion(&stage, &cast(&stage), "Idle", &MOTION_IDLE);
}

#[test]
fn the_wardrobe_stays_steady_in_the_walk() {
    let stage = Stage::new("wardrobe-motion-walk", &repository_library()).expect("the stage");
    check_motion(&stage, &cast(&stage), "Walk", &MOTION_WALK);
}

#[test]
fn the_wardrobe_stays_steady_in_the_run() {
    let stage = Stage::new("wardrobe-motion-run", &repository_library()).expect("the stage");
    check_motion(&stage, &cast(&stage), "Run", &MOTION_RUN);
}

/// The sneakers on the realistic cast (the body extremes too would take
/// the check past the test runner's two minutes: each pose is applied
/// twice): no foot through the sole or the upper at rest, with the knees
/// bent or in the walk, and the tread flat on the floor at rest.
#[test]
fn the_sneakers_hold_the_feet() {
    let stage = Stage::new("wardrobe-footwear", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in bodies().into_iter().filter(|d| d.name.contains("-cast-")) {
        let description = dressed(&description);
        for stance in [
            Stance::Rest,
            Stance::Held(HeldPose::KneesBent),
            Stance::Clip {
                name: "Walk",
                frames: 18,
            },
        ] {
            let reports = footwear(&stage.assets, &description, stance);
            let ours: Vec<_> = reports
                .iter()
                .filter(|r| r.shoe == "Shoes_CourtSneakers")
                .collect();
            assert!(!ours.is_empty(), "{}: no sneakers", description.name);
            for report in ours {
                println!(
                    "{} {} {}: {} covered, {} out ({:.2} mm), tread {:.2} to {:.2} mm",
                    report.character,
                    report.pose,
                    report.shoe,
                    report.covered,
                    report.through,
                    report.through_most * 1000.0,
                    report.tread_low * 1000.0,
                    report.tread_high * 1000.0
                );
                problems.extend(report.problems());
            }
        }
    }
    let left = unrecorded(&problems, &FOOTWEAR);
    assert!(left.is_empty(), "{left:#?}");
    let old = stale(&problems, &FOOTWEAR);
    assert!(old.is_empty(), "{old:#?}");
}

// The records: what the three drafts still fail (open defects, not
// accepted behaviour; `refinement/wave4/r7-wardrobe-first.md` says where
// each is and why). Lower them as the items improve; never raise one;
// a production item has none.
//
// - The joggers, at rest and in every pose: skin at the back of the ankle
//   comes out under the cuff (the cuff's inner layer, moved clear of the
//   heel column by column, leaves the skin between columns outside it):
//   12.5 mm on the default body, 31 mm at the extremes.
// - The tee, at rest and in every pose: round the waist, where its hem's
//   turn-up lies over the joggers' waistband, the tee's inner layer is
//   inside the skin by 1 to 8.5 mm; with the arms raised or in the walk, the
//   sleeve's end and the side at the armpit, as the test tee (CO9 part 2).
// - The joggers through the tee (every body, about 18 mm): the drawcord
//   and the waistband's top under the tee's hem, which the import pushed
//   the tee clear of at rest but not of the cord's ends.
// - The boxer briefs through the joggers (33 bodies, 9 mm) and the briefs
//   (1 to 6 bodies, 1.6 mm): at the gusset and the hip, where the
//   joggers' rise is close.
// - The bra through the tee (21 feminine bodies, 5.4 mm, 15.8 mm in Idle):
//   the bra's side band under the armpit, as with the test tee.
// - Every simpler copy, at rest: the joggers' LOD1 and LOD2 (31, 19 mm)
//   and the tee's (31, 30 mm) cut inside the skin where the full copy
//   folds over it (the ankles, the waist and the armpits).
//
// Re-recorded once at the wave-4 merge, because what the body shows
// changed, not to pass an item: CO9 part 2's pose-safe hiding (merged
// after this branch was made) stopped the tee and the joggers hiding the
// torso and arm regions, which had cut holes in the walk, so skin the
// hiding took out of the count is counted again. The tee at rest and with
// the elbows or knees bent 38 -> 51 bodies (28.2 mm unchanged), in the
// T-pose 24.7 -> 25.8 mm; the joggers with the knees bent 53 -> 54 bodies,
// 31.1 -> 39.3 mm, in the walk 51 -> 53 bodies. The rule above holds from
// here.

const fn known(garment: &'static str, pose: &'static str, cases: usize, worst: f32) -> Known {
    Known {
        garment,
        pose,
        cases,
        worst,
    }
}

const REST: [Known; 6] = [
    known("Pants_Joggers", "rest", 53, 0.0311),
    known("Pants_Joggers through Shirt_CrewTee", "rest", 54, 0.0185),
    known("Shirt_CrewTee", "rest", 51, 0.0282),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "rest",
        33,
        0.0090,
    ),
    known("Underwear_Bra through Shirt_CrewTee", "rest", 21, 0.0054),
    known("Underwear_Briefs through Pants_Joggers", "rest", 1, 0.0011),
];
const IDLE: [Known; 6] = [
    known("Pants_Joggers", "idle", 53, 0.0311),
    known("Pants_Joggers through Shirt_CrewTee", "idle", 54, 0.0226),
    known("Shirt_CrewTee", "idle", 54, 0.0422),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "idle",
        32,
        0.0094,
    ),
    known("Underwear_Bra through Shirt_CrewTee", "idle", 21, 0.0158),
    known("Underwear_Briefs through Pants_Joggers", "idle", 1, 0.0011),
];
const WALK: [Known; 6] = [
    known("Pants_Joggers", "Walk@18", 53, 0.0222),
    known("Pants_Joggers through Shirt_CrewTee", "Walk@18", 54, 0.0172),
    known("Shirt_CrewTee", "Walk@18", 54, 0.0345),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "Walk@18",
        33,
        0.0089,
    ),
    known("Underwear_Bra through Shirt_CrewTee", "Walk@18", 21, 0.0055),
    known(
        "Underwear_Briefs through Pants_Joggers",
        "Walk@18",
        6,
        0.0017,
    ),
];
const T_POSE: [Known; 6] = [
    known("Pants_Joggers", "TPose", 53, 0.0311),
    known("Pants_Joggers through Shirt_CrewTee", "TPose", 54, 0.0185),
    known("Shirt_CrewTee", "TPose", 54, 0.0258),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "TPose",
        33,
        0.0090,
    ),
    known("Underwear_Bra through Shirt_CrewTee", "TPose", 21, 0.0043),
    known("Underwear_Briefs through Pants_Joggers", "TPose", 1, 0.0011),
];
const ARMS_RAISED: [Known; 6] = [
    known("Pants_Joggers", "ArmsRaised", 53, 0.0311),
    known(
        "Pants_Joggers through Shirt_CrewTee",
        "ArmsRaised",
        54,
        0.0185,
    ),
    known("Shirt_CrewTee", "ArmsRaised", 54, 0.0372),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "ArmsRaised",
        33,
        0.0090,
    ),
    known(
        "Underwear_Bra through Shirt_CrewTee",
        "ArmsRaised",
        21,
        0.0038,
    ),
    known(
        "Underwear_Briefs through Pants_Joggers",
        "ArmsRaised",
        1,
        0.0011,
    ),
];
const ELBOWS_BENT: [Known; 6] = [
    known("Pants_Joggers", "ElbowsBent", 53, 0.0311),
    known(
        "Pants_Joggers through Shirt_CrewTee",
        "ElbowsBent",
        54,
        0.0185,
    ),
    known("Shirt_CrewTee", "ElbowsBent", 51, 0.0282),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "ElbowsBent",
        33,
        0.0090,
    ),
    known(
        "Underwear_Bra through Shirt_CrewTee",
        "ElbowsBent",
        21,
        0.0054,
    ),
    known(
        "Underwear_Briefs through Pants_Joggers",
        "ElbowsBent",
        1,
        0.0011,
    ),
];
const KNEES_BENT: [Known; 6] = [
    known("Pants_Joggers", "KneesBent", 54, 0.0393),
    known(
        "Pants_Joggers through Shirt_CrewTee",
        "KneesBent",
        54,
        0.0185,
    ),
    known("Shirt_CrewTee", "KneesBent", 51, 0.0282),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "KneesBent",
        33,
        0.0090,
    ),
    known(
        "Underwear_Bra through Shirt_CrewTee",
        "KneesBent",
        21,
        0.0054,
    ),
    known(
        "Underwear_Briefs through Pants_Joggers",
        "KneesBent",
        1,
        0.0011,
    ),
];
const LODS: [Known; 4] = [
    known("Pants_Joggers LOD1", "rest", 54, 0.0309),
    known("Pants_Joggers LOD2", "rest", 54, 0.0188),
    known("Shirt_CrewTee LOD1", "rest", 54, 0.0315),
    known("Shirt_CrewTee LOD2", "rest", 54, 0.0299),
];
const MOTION_IDLE: [Record; 0] = [];
/// The tee jolts at the armpit as the arm swings through hanging (the
/// swing correctives' copies, as the test tee's: CO9 part 2).
const MOTION_WALK: [Record; 1] = [Record {
    what: "Shirt_CrewTee pop",
    case: "Walk",
    most: 0.0025,
}];
const MOTION_RUN: [Record; 1] = [Record {
    what: "Shirt_CrewTee pop",
    case: "Run",
    most: 0.0051,
}];
/// The sneakers: in the walk's stride the heel and the toes come out of
/// the shoe (3 to 12 corners, up to 11.3 mm), and at rest the tread isn't
/// flat on the floor: the fit carries the body's shapes onto the sole, so
/// it tilts with the foot (masculine 3.9 mm under the floor to 5.7 mm over
/// it; feminine 4 to 5 mm under it).
const FOOTWEAR: [Record; 2] = [
    Record {
        what: "Shoes_CourtSneakers through",
        case: "Walk@18",
        most: 0.0113,
    },
    Record {
        what: "Shoes_CourtSneakers tread",
        case: "rest",
        most: 0.0099,
    },
];
