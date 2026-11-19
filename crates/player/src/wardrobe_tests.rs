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
    Failure, HOLES, Known, Posed, WALK, WALK_FRAMES, bodies, check, cover_at_rest, hidden_at_rest,
    measure, measure_walk, poke_through, pose_name, posed_at, undressed, worn,
};
use super::garment_checks::{Record, check_motion_with, footwear, frames_of, stale, unrecorded};

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

/// A failure that is one of the three items' (or a layer through one),
/// or hidden skin showing (wave 5: the tee and the joggers hide what they
/// enclose).
fn ours(failure: &Failure) -> bool {
    PARTS.iter().any(|p| failure.garment.contains(p)) || failure.garment == HOLES
}

/// Wave 5, hiding under clothing: wearing the crew tee and the joggers
/// leaves out the skin they enclose, the underwear under them and the
/// joggers' waistband under the tee's hem where it is enclosed; taking
/// them off puts it back.
#[test]
fn the_wardrobe_hides_the_skin_and_underwear_it_encloses() {
    let stage = Stage::new("wardrobe-hiding", &repository_library()).expect("the stage");
    for (name, skin_least, under_least) in [
        (
            "realistic-masculine-cast-average",
            HIDES_SKIN.0,
            HIDES_UNDER.0,
        ),
        (
            "realistic-feminine-cast-average",
            HIDES_SKIN.1,
            HIDES_UNDER.1,
        ),
    ] {
        let description = bodies().into_iter().find(|d| d.name == name).unwrap();
        let (skin, parts) = hidden_at_rest(&stage.assets, &description, &dressed(&description));
        println!("{name}: {skin} body triangles hidden; parts {parts:?}");
        assert!(
            skin >= skin_least,
            "{name}: {skin} hidden, at least {skin_least}"
        );
        let under: usize = parts
            .iter()
            .filter(|(part, _)| part.starts_with("Underwear_"))
            .map(|(_, n)| *n)
            .sum();
        assert!(
            under >= under_least,
            "{name}: underwear {under} hidden, at least {under_least}"
        );
        let (again, parts) = hidden_at_rest(&stage.assets, &description, &description);
        assert_eq!(again, 0);
        assert!(parts.iter().all(|(_, n)| *n == 0), "{parts:?}");
    }
}

/// How many body triangles the crew tee and joggers hide at least
/// (masculine, feminine average), and of the underwear: what the masks hid
/// when made and trimmed with the fine views (wave 5, review fixes 1,
/// `refinement/wave5/body-hiding.md`). They change only
/// when the masks are made again.
const HIDES_SKIN: (usize, usize) = (709, 709);
const HIDES_UNDER: (usize, usize) = (149, 104);

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

/// The gate through the walk's `frames` ([`WALK_FRAMES`]): the walking
/// fit test (Paul's decision 13), on every realistic cast and body
/// benchmark dressed in the three.
fn wardrobe_gate_walk(frames: &[u32], known: &[Known]) {
    let label = format!("{WALK} {frames:?}");
    let stage = Stage::new(
        &format!("wardrobe-gate-walk-{}", frames[0]),
        &repository_library(),
    )
    .expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let mut cover = cover_at_rest(&body, &garments, &bare);
        // Holes in the walk are `the_wardrobe_shows_no_hidden_place_through_the_walk_*`.
        cover.holes = false;
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
        &WALK_18,
    );
}

#[test]
fn the_wardrobe_covers_the_cast_through_the_walk_first_third() {
    wardrobe_gate_walk(WALK_FRAMES[0], &WALK_THROUGH_FIRST);
}

#[test]
fn the_wardrobe_covers_the_cast_through_the_walk_second_third() {
    wardrobe_gate_walk(WALK_FRAMES[1], &WALK_THROUGH_SECOND);
}

#[test]
fn the_wardrobe_covers_the_cast_through_the_walk_last_third() {
    wardrobe_gate_walk(WALK_FRAMES[2], &WALK_THROUGH_LAST);
}

#[test]
fn the_wardrobe_covers_the_cast_with_the_arms_forward() {
    wardrobe_gate(Stance::Held(HeldPose::ArmsForward), &ARMS_FORWARD);
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
    check_motion_with(&stage, &cast(&stage), "Idle", &MOTION_IDLE, false);
}

#[test]
fn the_wardrobe_stays_steady_in_the_walk() {
    let stage = Stage::new("wardrobe-motion-walk", &repository_library()).expect("the stage");
    check_motion_with(&stage, &cast(&stage), "Walk", &MOTION_WALK, false);
}

#[test]
fn the_wardrobe_stays_steady_in_the_run() {
    let stage = Stage::new("wardrobe-motion-run", &repository_library()).expect("the stage");
    check_motion_with(&stage, &cast(&stage), "Run", &MOTION_RUN, false);
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
//
// Lowered in wave 5 by the gate's corrected measure (`enclosed`: skin the
// garment encloses is inside it), not a refit: the bra through the tee in
// Idle 15.8 -> 5.3 mm, the boxer briefs through the joggers in the walk
// frame 8.9 -> 5.2 mm, the joggers with the knees bent 39.3 -> 31.1 mm.
// That last is the value from before the wave-4 merge by coincidence: the
// knees-bent case is the ankle's, 31.0 mm in every pose, and the 39.3 mm
// one was enclosed skin; nothing was refitted. Tee-walk review fixes 2
// (the hit triangle's posed face must agree, `from_inside`) measured
// every record here again: no count or worst moved, so none changed.

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
    known("Underwear_Bra through Shirt_CrewTee", "idle", 21, 0.0053),
    known("Underwear_Briefs through Pants_Joggers", "idle", 1, 0.0011),
];
const WALK_18: [Known; 6] = [
    known("Pants_Joggers", "Walk@18", 53, 0.0222),
    known("Pants_Joggers through Shirt_CrewTee", "Walk@18", 54, 0.0172),
    known("Shirt_CrewTee", "Walk@18", 54, 0.0345),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "Walk@18",
        33,
        0.0052,
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
    known("Pants_Joggers", "KneesBent", 54, 0.0311),
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
// The walking fit test and the arms forward (wave 5, `refinement/wave5/
// tee-walk.md`), from their first run: the drafts' open defects. The tee
// fails the walk on every body, up to 38.7 mm: round the waist as at
// rest, and in the arm, the armpit and the chest as the test tee; the
// joggers at the ankle as at rest; the underwear through both as in the
// walk frame.
const WALK_THROUGH_FIRST: [Known; 6] = [
    known("Pants_Joggers", WALK, 53, 0.0283),
    known("Pants_Joggers through Shirt_CrewTee", WALK, 54, 0.0233),
    known("Shirt_CrewTee", WALK, 54, 0.0385),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        WALK,
        33,
        0.0076,
    ),
    known("Underwear_Bra through Shirt_CrewTee", WALK, 21, 0.0055),
    known("Underwear_Briefs through Pants_Joggers", WALK, 20, 0.0097),
];
const WALK_THROUGH_SECOND: [Known; 6] = [
    known("Pants_Joggers", WALK, 54, 0.0306),
    known("Pants_Joggers through Shirt_CrewTee", WALK, 54, 0.0172),
    known("Shirt_CrewTee", WALK, 54, 0.0350),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        WALK,
        33,
        0.0055,
    ),
    known("Underwear_Bra through Shirt_CrewTee", WALK, 21, 0.0055),
    known("Underwear_Briefs through Pants_Joggers", WALK, 11, 0.0019),
];
const WALK_THROUGH_LAST: [Known; 6] = [
    known("Pants_Joggers", WALK, 54, 0.0280),
    known("Pants_Joggers through Shirt_CrewTee", WALK, 54, 0.0243),
    known("Shirt_CrewTee", WALK, 54, 0.0387),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        WALK,
        33,
        0.0070,
    ),
    known("Underwear_Bra through Shirt_CrewTee", WALK, 21, 0.0055),
    known("Underwear_Briefs through Pants_Joggers", WALK, 18, 0.0096),
];
const ARMS_FORWARD: [Known; 6] = [
    known("Pants_Joggers", "ArmsForward", 53, 0.0311),
    known(
        "Pants_Joggers through Shirt_CrewTee",
        "ArmsForward",
        54,
        0.0192,
    ),
    known("Shirt_CrewTee", "ArmsForward", 54, 0.0407),
    known(
        "Underwear_BoxerBriefs through Pants_Joggers",
        "ArmsForward",
        33,
        0.0043,
    ),
    known(
        "Underwear_Bra through Shirt_CrewTee",
        "ArmsForward",
        21,
        0.0060,
    ),
    known(
        "Underwear_Briefs through Pants_Joggers",
        "ArmsForward",
        8,
        0.0013,
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

/// Wave 5, hiding under clothing: no place the three hide shows (the
/// gate's hole check, `fit_gate_tests::holes`), through the walk's whole
/// stride (every third frame) on every gate body, and every third frame
/// of the motion check's idle and run on the cast. Split by bodies, so
/// each test stays inside the runner's two minutes: the joggers hide a
/// thousand triangles of the legs, and each place is looked at from
/// about 150 directions. The gate and motion tests above leave holes to
/// these.
fn wardrobe_holes(clip: &'static str, frames: &[u32], cast_only: bool, part: usize, parts: usize) {
    use super::fit_gate_tests::{HOLES, hidden_triangles, holes, worn_in};
    let label = format!("wardrobe holes {clip} {part}/{parts}");
    let stage = Stage::new(
        &format!("wardrobe-holes-{clip}-{part}"),
        &repository_library(),
    )
    .expect("the stage");
    let chosen: Vec<Description> = bodies()
        .into_iter()
        .filter(|d| !cast_only || d.name.contains("-cast-"))
        .collect();
    let size = chosen.len().div_ceil(parts);
    let mut failures = Vec::new();
    for description in chosen.iter().skip(part * size).take(size) {
        let dressed = dressed(description);
        let (body, _) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bare, _) = worn(&stage.assets, &undressed(description), Stance::Rest);
        let hidden = hidden_triangles(&bare, &body);
        let first = frames[0];
        let last = *frames.last().unwrap();
        let (mut world, mut animators) = stage_world(
            &stage.assets,
            &dressed,
            Stance::Clip {
                name: clip,
                frames: first,
            },
            &[],
        )
        .expect("the clip");
        let mut worst: Option<Failure> = None;
        for frame in first..=last {
            if frames.contains(&frame) {
                let (body, garments, _) = worn_in(&world, &stage.assets, &animators);
                let at = format!("{} {clip}@{frame}", description.name);
                let (count, most) = holes(&at, &body, &hidden, &garments);
                if count > 0 && worst.as_ref().is_none_or(|w| most > w.worst) {
                    worst = Some(Failure {
                        garment: HOLES.to_owned(),
                        pose: clip.to_owned(),
                        body: at,
                        count,
                        worst: most,
                    });
                }
            }
            animators.update(&mut world, &stage.assets, 1.0 / 60.0);
        }
        failures.extend(worst);
    }
    check(&label, &failures, &[]);
}

/// Every third frame of the walk's stride (frames 2 to 44).
fn walk_frames() -> Vec<u32> {
    WALK_FRAMES.iter().flat_map(|f| f.iter().copied()).collect()
}

/// Every third frame (2, 5, 8, ...) of a clip as the motion check plays it.
fn motion_frames(clip: &str) -> Vec<u32> {
    frames_of(clip).filter(|f| f % 3 == 2).collect()
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_1() {
    wardrobe_holes("Walk", &walk_frames(), false, 0, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_2() {
    wardrobe_holes("Walk", &walk_frames(), false, 1, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_3() {
    wardrobe_holes("Walk", &walk_frames(), false, 2, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_4() {
    wardrobe_holes("Walk", &walk_frames(), false, 3, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_5() {
    wardrobe_holes("Walk", &walk_frames(), false, 4, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_6() {
    wardrobe_holes("Walk", &walk_frames(), false, 5, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_7() {
    wardrobe_holes("Walk", &walk_frames(), false, 6, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_8() {
    wardrobe_holes("Walk", &walk_frames(), false, 7, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_9() {
    wardrobe_holes("Walk", &walk_frames(), false, 8, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_10() {
    wardrobe_holes("Walk", &walk_frames(), false, 9, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_11() {
    wardrobe_holes("Walk", &walk_frames(), false, 10, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_12() {
    wardrobe_holes("Walk", &walk_frames(), false, 11, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_13() {
    wardrobe_holes("Walk", &walk_frames(), false, 12, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_14() {
    wardrobe_holes("Walk", &walk_frames(), false, 13, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_15() {
    wardrobe_holes("Walk", &walk_frames(), false, 14, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_16() {
    wardrobe_holes("Walk", &walk_frames(), false, 15, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_17() {
    wardrobe_holes("Walk", &walk_frames(), false, 16, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_through_the_walk_18() {
    wardrobe_holes("Walk", &walk_frames(), false, 17, 18);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_idle_1() {
    wardrobe_holes("Idle", &motion_frames("Idle"), true, 0, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_idle_2() {
    wardrobe_holes("Idle", &motion_frames("Idle"), true, 1, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_idle_3() {
    wardrobe_holes("Idle", &motion_frames("Idle"), true, 2, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_idle_4() {
    wardrobe_holes("Idle", &motion_frames("Idle"), true, 3, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_idle_5() {
    wardrobe_holes("Idle", &motion_frames("Idle"), true, 4, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_idle_6() {
    wardrobe_holes("Idle", &motion_frames("Idle"), true, 5, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_run_1() {
    wardrobe_holes("Run", &motion_frames("Run"), true, 0, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_run_2() {
    wardrobe_holes("Run", &motion_frames("Run"), true, 1, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_run_3() {
    wardrobe_holes("Run", &motion_frames("Run"), true, 2, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_run_4() {
    wardrobe_holes("Run", &motion_frames("Run"), true, 3, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_run_5() {
    wardrobe_holes("Run", &motion_frames("Run"), true, 4, 6);
}

#[test]
fn the_wardrobe_shows_no_hidden_place_in_the_run_6() {
    wardrobe_holes("Run", &motion_frames("Run"), true, 5, 6);
}
