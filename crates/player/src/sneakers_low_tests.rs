//! The first sneakers made from scratch (wave 5 stage 3, Paul's decisions
//! 13 and 14; `docs/development/character-studio/refinement/wave5/
//! sneakers-scratch.md`): `axcenity/realistic-sneakers-low`, a plain
//! low-top modelled in Blender over a last made from the realistic foot,
//! worn by every realistic cast body of both frames, as a game poses it.
//! It is held to `docs/development/character-studio/asset-quality.md` §3
//! "Footwear" and "Stability in motion". It is a draft: what it still
//! fails is recorded below ([`WALK_RECORDS`], [`RUN_RECORDS`], and
//! [`EXTREME_RECORDS`] for the body extremes) as open defects with their
//! numbers, which can only go down; everything else (the cast at rest, in
//! Idle and the held poses, the simpler copies, motion) has none.
//!
//! - **Footwear** (`garment_checks::foot_report`): no foot corner through
//!   the sole or the upper (the cover rule, and the shoe's outside under
//!   an uncovered corner, `exposed`), the tread flat on the floor within
//!   2 mm at rest, and the toe box bending only at the toe joint: at rest,
//!   in Idle, in the body suite's held poses, through the walk's whole
//!   stride and the run's, every third frame. Each simpler copy at rest.
//! - **Stability in motion** (`garment_checks::motion`): Idle, Walk, Run.

use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, Value};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{WALK_FRAMES, bodies, posed_at, worn};
use super::garment_checks::{
    FootReport, Problem, Record, check_motion, foot_report, footwear_through, stale, unrecorded,
};

pub(super) const SNEAKERS: &str = "axcenity/realistic-sneakers-low";
/// The part the pair draws as.
pub(super) const PART: &str = "Shoes_LowSneakers";

/// The realistic cast of both frames, each wearing the sneakers over its
/// underwear.
fn cast() -> Vec<Description> {
    let cast = wearing(|name| name.contains("-cast-"));
    assert_eq!(cast.len(), 18, "nine cast bodies a frame");
    cast
}

/// The body extremes of the three frames (masculine, feminine and the
/// blend: tallest, shortest, heaviest and the rest, each body parameter at
/// its ends), each wearing the sneakers. The tallest and shortest change
/// the foot's size, so the shoe's shapes are tested at their ends here.
fn extremes() -> Vec<Description> {
    let extremes = wearing(|name| name.contains("-body-"));
    assert_eq!(extremes.len(), 36, "twelve body extremes a frame");
    extremes
}

/// The realistic bodies whose name `pick` accepts, wearing the sneakers.
fn wearing(pick: impl Fn(&str) -> bool) -> Vec<Description> {
    bodies()
        .into_iter()
        .filter(|d| pick(&d.name))
        .map(|d| {
            let mut out = d.clone();
            out.items.push(WornItem::new(SNEAKERS));
            out
        })
        .collect()
}

/// Prints a report and adds its problems (each under its clip's name, a
/// stride's frames together); the sneakers must be among the reports and
/// cover the feet.
fn take(reports: &[FootReport], who: &str, problems: &mut Vec<Problem>) {
    let ours: Vec<&FootReport> = reports.iter().filter(|r| r.shoe == PART).collect();
    assert!(!ours.is_empty(), "{who}: no {PART}");
    for report in ours {
        println!(
            "{} {}: {} covered, {} through ({:.2} mm), {} exposed ({:.2} mm), toe box {:.2} mm, tread {:.2} to {:.2} mm",
            report.character,
            report.pose,
            report.covered,
            report.through,
            report.through_most * 1000.0,
            report.exposed,
            report.exposed_most * 1000.0,
            report.toe_box_bend * 1000.0,
            report.tread_low * 1000.0,
            report.tread_high * 1000.0
        );
        problems.extend(report.problems().into_iter().map(|p| Problem {
            case: p.case.split('@').next().unwrap_or_default().to_owned(),
            ..p
        }));
    }
}

/// The footwear check in held stances (`Rest`, `Idle`, held poses) on the
/// cast: no problem at all.
fn hold_the_feet(label: &str, stances: &[Stance]) {
    hold_the_feet_of(label, cast(), stances, &[]);
}

/// The footwear check in held stances on `bodies`, against `records`.
fn hold_the_feet_of(label: &str, bodies: Vec<Description>, stances: &[Stance], records: &[Record]) {
    let stage =
        Stage::new(&format!("sneakers-low-{label}"), &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in bodies {
        let (body_rest, garments_rest) = worn(&stage.assets, &description, Stance::Rest);
        for stance in stances {
            let (body, garments) = if *stance == Stance::Rest {
                (body_rest.clone(), garments_rest.clone())
            } else {
                worn(&stage.assets, &description, *stance)
            };
            let pose = super::fit_gate_tests::pose_name(*stance);
            if *stance == Stance::Rest {
                let lowest = body.positions.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                println!(
                    "{} rest: the body's lowest point {:.2} mm",
                    description.name,
                    lowest * 1000.0
                );
            }
            let reports: Vec<FootReport> = garments_rest
                .iter()
                .zip(&garments)
                .filter(|(g, _)| g.name.starts_with("Shoes_"))
                .map(|(shoe_rest, shoe)| {
                    foot_report(&description.name, &pose, &body_rest, shoe_rest, &body, shoe)
                })
                .collect();
            take(&reports, &description.name, &mut problems);
        }
    }
    let left = unrecorded(&problems, records);
    assert!(left.is_empty(), "{left:#?}");
    let old = stale(&problems, records);
    assert!(old.is_empty(), "{old:#?}");
}

/// The footwear check through `frames` of `clip` on the cast, against the
/// clip's records.
fn hold_the_feet_through(clip: &'static str, frames: &[u32], records: &[Record]) {
    let stage = Stage::new(
        &format!("sneakers-low-{clip}-{}", frames[0]),
        &repository_library(),
    )
    .expect("the stage");
    let mut problems = Vec::new();
    for description in cast() {
        let reports = footwear_through(&stage.assets, &description, clip, frames);
        take(&reports, &description.name, &mut problems);
    }
    let left = unrecorded(&problems, records);
    assert!(left.is_empty(), "{left:#?}");
    let old = stale(&problems, records);
    assert!(old.is_empty(), "{old:#?}");
}

#[test]
fn the_sneakers_hold_the_feet_at_rest_and_in_idle() {
    hold_the_feet("rest", &[Stance::Rest, Stance::Idle]);
}

#[test]
fn the_sneakers_hold_the_feet_in_the_held_poses() {
    hold_the_feet(
        "held",
        &[
            Stance::Held(HeldPose::TPose),
            Stance::Held(HeldPose::ArmsRaised),
            Stance::Held(HeldPose::ArmsForward),
            Stance::Held(HeldPose::ElbowsBent),
            Stance::Held(HeldPose::KneesBent),
        ],
    );
}

/// The body extremes at rest, in Idle and in the held poses, against
/// [`EXTREME_RECORDS`] (review fixes 1: the cast alone left them unmeasured).
#[test]
fn the_sneakers_hold_the_feet_of_the_body_extremes() {
    hold_the_feet_of(
        "extremes",
        extremes(),
        &[
            Stance::Rest,
            Stance::Idle,
            Stance::Held(HeldPose::TPose),
            Stance::Held(HeldPose::ArmsRaised),
            Stance::Held(HeldPose::ArmsForward),
            Stance::Held(HeldPose::ElbowsBent),
            Stance::Held(HeldPose::KneesBent),
        ],
        &EXTREME_RECORDS,
    );
}

#[test]
fn the_sneakers_hold_the_feet_through_the_walk_first_third() {
    hold_the_feet_through("Walk", WALK_FRAMES[0], &WALK_RECORDS);
}

#[test]
fn the_sneakers_hold_the_feet_through_the_walk_second_third() {
    hold_the_feet_through("Walk", WALK_FRAMES[1], &WALK_RECORDS_SECOND);
}

#[test]
fn the_sneakers_hold_the_feet_through_the_walk_last_third() {
    hold_the_feet_through("Walk", WALK_FRAMES[2], &WALK_RECORDS_LAST);
}

/// The run's stride (frames 2 to 35, as the motion check plays it), every
/// third frame.
#[test]
fn the_sneakers_hold_the_feet_through_the_run() {
    hold_the_feet_through(
        "Run",
        &[2, 5, 8, 11, 14, 17, 20, 23, 26, 29, 32, 35],
        &RUN_RECORDS,
    );
}

/// §2 "LODs": each simpler copy of the pair covers the feet at rest as the
/// full one does: no foot corner out through it, its tread flat.
#[test]
fn the_sneakers_levels_of_detail_hold_the_feet_at_rest() {
    let stage = Stage::new("sneakers-low-lods", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in cast() {
        let (world, animators) =
            stage_world(&stage.assets, &description, Stance::Rest, &[]).expect("the stage world");
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
        let shoes = named(PART).unwrap_or_else(|| panic!("{}: no {PART}", description.name));
        for level in 1..=2 {
            let shoe = posed_at(&world, &stage.assets, &animators, shoes, level);
            let report = foot_report(&description.name, "rest", &body, &shoe, &body, &shoe);
            let mut report = report;
            report.pose = format!("rest LOD{level}");
            let mut found = Vec::new();
            take(&[report.clone()], &description.name, &mut found);
            problems.extend(found.into_iter().map(|p| p.line));
            // The tread is judged at rest only (`FootReport::problems`):
            // judge each level's here.
            let off = report
                .tread_low
                .abs()
                .max(report.tread_high - report.tread_low);
            if off > super::garment_checks::SOLE_FLAT {
                problems.push(format!(
                    "{} LOD{level}: the tread from {:.2} to {:.2} mm",
                    description.name,
                    report.tread_low * 1000.0,
                    report.tread_high * 1000.0
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// The cast, each checked to wear the pair (a motion check of a character
/// without it would pass).
fn worn_cast(stage: &Stage) -> Vec<Description> {
    let cast = cast();
    for description in &cast {
        let (_, garments) = worn(&stage.assets, description, Stance::Rest);
        assert!(
            garments.iter().any(|g| g.name == PART),
            "{}: no {PART}",
            description.name
        );
    }
    cast
}

#[test]
fn the_sneakers_stay_steady_in_the_idle() {
    let stage = Stage::new("sneakers-low-motion-idle", &repository_library()).expect("the stage");
    check_motion(&stage, &worn_cast(&stage), "Idle", &[]);
}

#[test]
fn the_sneakers_stay_steady_in_the_walk() {
    let stage = Stage::new("sneakers-low-motion-walk", &repository_library()).expect("the stage");
    check_motion(&stage, &worn_cast(&stage), "Walk", &[]);
}

#[test]
fn the_sneakers_stay_steady_in_the_run() {
    let stage = Stage::new("sneakers-low-motion-run", &repository_library()).expect("the stage");
    check_motion(&stage, &worn_cast(&stage), "Run", &[]);
}

// The records: what the draft still fails (open defects, not accepted
// behaviour; `refinement/wave5/sneakers-scratch.md` says where each is).
// Lower them as the item improves; never raise one; a production item has
// none. Through the stride, on all 18 cast bodies, foot corners the shoe
// covers at rest leave it: on the feminine bodies most (the feminine foot
// is narrower and shorter than the last, and the shoe follows it by one
// smooth map per body shape, so the side of the ball and the inner heel
// stand nearer the skin than on the masculine foot), on the masculine
// bodies at the inner heel and the side of the ankle; and skin just above
// the collar that the cover rule counts as covered at rest (its way out
// meets the collar's roll) moves away from the collar as the ankle turns.
// The worst, 9.0 mm, is on the heavyset feminine body, in every third of
// the stride and in the run. Review fixes 2 corrected this: a one-off dump
// on the shipped build found every one of these readings (1,378 over the
// cast, every third frame) still inside the shoe, 5 to 9 mm off its inner
// faces, its way out no longer meeting the shoe as the ankle turns (the
// worst is skin at the outer front of the ankle, 8.1 mm inside the
// collar at rest and in the walk alike). The records hold what the shared
// cover rule reads; whether it should read them is the lead's ruling.
const WALK_RECORDS: [Record; 1] = [Record {
    what: "Shoes_LowSneakers through",
    case: "Walk",
    most: 0.0091,
}];
/// The stride's second and last thirds (frames 17 to 29, 32 to 44): the
/// same worst as the first.
const WALK_RECORDS_SECOND: [Record; 1] = [Record {
    what: "Shoes_LowSneakers through",
    case: "Walk",
    most: 0.0091,
}];
const WALK_RECORDS_LAST: [Record; 1] = [Record {
    what: "Shoes_LowSneakers through",
    case: "Walk",
    most: 0.0091,
}];
const RUN_RECORDS: [Record; 1] = [Record {
    what: "Shoes_LowSneakers through",
    case: "Run",
    most: 0.0091,
}];
/// On the body extremes (measured first in review fixes 1): the shoe
/// follows the foot's size by one smooth map per body shape, which on the
/// shortest and tallest bodies of each frame tilts and sinks the tread
/// (from 9.10 mm under the floor to 5.63 mm over it, the worst on the
/// shortest feminine body; on every other extreme it is flat); and on the
/// slim feminine body one foot corner leaves the shoe in Idle and with
/// the knees bent, 5.69 mm.
const EXTREME_RECORDS: [Record; 3] = [
    Record {
        what: "Shoes_LowSneakers tread",
        case: "rest",
        most: 0.0092,
    },
    Record {
        what: "Shoes_LowSneakers through",
        case: "idle",
        most: 0.0058,
    },
    Record {
        what: "Shoes_LowSneakers through",
        case: "KneesBent",
        most: 0.0058,
    },
];
