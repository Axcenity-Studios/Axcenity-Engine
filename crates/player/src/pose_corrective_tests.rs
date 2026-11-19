//! A garment's pose correctives baked from cloth simulation, in the
//! engine (wave 5, `docs/development/character-studio/refinement/wave5/
//! pose-correctives.md`): the runtime drives them from the joints on every
//! frame, and the fit gate measures the test tee with them
//! ([`gate_library_with_correctives`]: `TestTee.correctives.json`) beside
//! the gate's own test tee without them ([`gate_library`], whose records
//! in `fit_gate_tests` stand: a garment without correctives poses as
//! before).
//!
//! The records here are the test tee's with its correctives, exact and
//! only lowered, as the gate's are. They are not the gate's: where the
//! correctives do worse than the gate's records (the bra through the tee
//! in the stride's first third, one body more at Walk@18 and with the
//! arms raised), taking them into the gate would raise a record, which is
//! the lead's call (`pose-correctives.md`, open issues).

use axcenity_assets::Model;
use axcenity_avatar::WornItem;
use axcenity_core::Value;

use super::character_capture::{HeldPose, Stage, Stance, stage_world};
use super::fit_gate_tests::{
    BRA_THROUGH_TEE, Known, TEE, WALK_FRAMES, bodies, cover_at_rest, gate_in, gate_library,
    gate_library_with_correctives, gate_walk_in, under_flipped_folds, undressed, walk_records,
    worn, worn_in,
};
use super::garment_checks::{Record, check_motion, realistic_cast};

/// The test tee's part and the model it draws.
fn tee_part(
    world: &axcenity_core::World,
    assets: &axcenity_assets::Assets,
) -> (axcenity_core::InstanceId, std::sync::Arc<Model>) {
    let part = world
        .all_below(world.workspace(), |class| {
            class == axcenity_core::ClassName::MeshPart
        })
        .into_iter()
        .find(|part| world.get(*part).is_some_and(|p| p.name() == "TestTee"))
        .expect("the test tee is worn");
    let Some(Value::String(mesh_id)) = world.get(part).and_then(|p| p.property("MeshId")) else {
        panic!("the test tee has no MeshId");
    };
    (part, assets.model_blocking(mesh_id).unwrap())
}

/// In the walk's widest swing (frame 18: the right arm back, the left
/// forward), the runtime leans the test tee on its cloth correctives for
/// those swings, by its own joints (the body has no shapes of those
/// names), and on none for the other way round; the gate's tee without
/// them has no driven shape.
#[test]
fn the_test_tee_leans_on_its_cloth_correctives_in_the_walk() {
    let walk = Stance::Clip {
        name: "Walk",
        frames: 18,
    };
    let mut dressed = bodies().remove(0);
    dressed.items.push(WornItem::new(TEE));
    let stage =
        Stage::new("pose-correctives-walk", gate_library_with_correctives()).expect("the stage");
    let (world, animators) = stage_world(&stage.assets, &dressed, walk, &[]).unwrap();
    let (part, model) = tee_part(&world, &stage.assets);
    let driven = model
        .morph_targets
        .iter()
        .filter(|t| t.driver.is_some())
        .count();
    assert!(driven >= 10, "{driven} driven shapes");
    let posed = &animators.poses()[&part];
    let weight = |name: &str| {
        let index = model
            .morph_targets
            .iter()
            .position(|t| t.name == format!("Corrective_Cloth_{name}"))
            .unwrap_or_else(|| panic!("no {name}"));
        posed.morph_weights().get(index).copied().unwrap_or(0.0)
    };
    let back = weight("Shoulder_BackHalf_Right");
    let forward = weight("Shoulder_ForwardHalf_Left");
    println!(
        "Walk@18: BackHalf_Right {back:.2}, Back_Right {:.2}, ForwardHalf_Left {forward:.2}, Forward_Left {:.2}, Lowered {:.2} {:.2}",
        weight("Shoulder_Back_Right"),
        weight("Shoulder_Forward_Left"),
        weight("Shoulder_Lowered_Left"),
        weight("Shoulder_Lowered_Right")
    );
    assert!(back > 0.5, "{back}");
    assert!(forward > 0.5, "{forward}");
    assert_eq!(weight("Shoulder_BackHalf_Left"), 0.0);
    assert_eq!(weight("Shoulder_ForwardHalf_Right"), 0.0);
    assert!(weight("Shoulder_Lowered_Left") > 0.5);
    assert_eq!(weight("Shoulder_Raised_Left"), 0.0);

    let plain = Stage::new("pose-correctives-walk-plain", gate_library()).expect("the stage");
    let (world, _) = stage_world(&plain.assets, &dressed, walk, &[]).unwrap();
    let (_, model) = tee_part(&world, &plain.assets);
    assert!(model.morph_targets.iter().all(|t| t.driver.is_none()));
}

/// Body corners through the test tee under a flipped fold in `frames` of
/// the walk, on every gate body, in `library`: each body's most over the
/// frames, added up.
fn flipped_in_walk(library: &std::path::Path, frames: &[u32]) -> usize {
    let stage =
        Stage::new(&format!("pose-correctives-flipped-{}", frames[0]), library).expect("the stage");
    let first = frames.iter().copied().min().unwrap_or(2).max(2);
    let last = frames.iter().copied().max().unwrap_or(first);
    let mut total = 0;
    for description in bodies() {
        let mut dressed = description.clone();
        dressed.items.push(WornItem::new(TEE));
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        let (bare, _) = worn(&stage.assets, &undressed(&description), Stance::Rest);
        let cover = cover_at_rest(&body, &garments, &bare);
        let covered = &cover
            .body
            .iter()
            .find(|(name, _)| name == "TestTee")
            .expect("the tee covers")
            .1;
        let (mut world, mut animators) = stage_world(
            &stage.assets,
            &dressed,
            Stance::Clip {
                name: "Walk",
                frames: first,
            },
            &[],
        )
        .unwrap();
        let mut most = 0;
        for frame in first..=last {
            if frames.contains(&frame) {
                let (body, garments, _) = worn_in(&world, &stage.assets, &animators);
                let tee = garments.iter().find(|g| g.name == "TestTee").unwrap();
                most = most.max(under_flipped_folds(&body, tee, covered));
            }
            animators.update(&mut world, &stage.assets, 1.0 / 60.0);
        }
        total += most;
    }
    total
}

/// A third of the stride, with the correctives and without: each count
/// exact against its record (lower a record when its count comes down),
/// and fewer with them.
fn flipped_third(third: usize) {
    let with = flipped_in_walk(gate_library_with_correctives(), WALK_FRAMES[third]);
    let without = flipped_in_walk(gate_library(), WALK_FRAMES[third]);
    println!(
        "walk third {third}: {with} body corners through the tee under a flipped fold with its correctives, {without} without"
    );
    for (what, measured, record) in [
        ("with", with, FLIPPED[third]),
        ("without", without, BEFORE[third]),
    ] {
        assert!(
            measured <= record,
            "{what}: {measured} under a flipped fold, recorded {record}"
        );
        assert!(
            measured == record,
            "{what}: {measured} under a flipped fold, recorded {record}: lower the record"
        );
    }
    assert!(with < without, "{with} with, {without} without");
}

// The stride's armpit folds (`tee-walk.md`): body corners the gate counts
// through the test tee whose way out meets a fold flipped towards the
// skin, each body's most over the third's frames, added over the 54
// bodies. Exact counts: lower them as the tee improves, never raise them.
const BEFORE: [usize; 3] = [147, 199, 161];
const FLIPPED: [usize; 3] = [122, 136, 139];

#[test]
fn the_strides_armpit_folds_flip_less_with_correctives_first_third() {
    flipped_third(0);
}

#[test]
fn the_strides_armpit_folds_flip_less_with_correctives_second_third() {
    flipped_third(1);
}

#[test]
fn the_strides_armpit_folds_flip_less_with_correctives_last_third() {
    flipped_third(2);
}

// The fit gate on the test tee with its correctives: what it still finds,
// per garment and pose, as the gate's records hold (open defects, exact,
// only lowered). Against the gate's records for the tee without them
// (`fit_gate_tests`), with the correctives:
//
// - lower: the stride's second and last thirds (16.6 to 14.1 mm, 14.7 to
//   14.5 mm; the bra through it 17.1 to 10.4 mm, 13.1 to 11.5 mm), its
//   first third (13.6 to 13.4 mm), Walk@18's worst (10.3 to 9.0 mm; the
//   bra 15.2 to 10.2 mm), arms forward (44 bodies, 9.4 mm to 43, 7.2 mm;
//   the bra 15 bodies, 3.6 mm to 6, 2.9 mm), arms raised (2.9 to 2.5 mm);
// - higher: the bra through the tee in the first third (20 bodies, 10.8
//   mm to 21, 14.9 mm), Walk@18 one body more for the tee (39 from 38) and
//   for the bra (7 from 6), and the bra through the tee with the arms
//   raised (none to 1 body, 2.3 mm: feminine heavy-soft).

fn with(stance: Stance, known: &[Known]) {
    gate_in(gate_library_with_correctives(), stance, known);
}

const fn tee(pose: &'static str, cases: usize, worst: f32) -> Known {
    Known {
        garment: "TestTee",
        pose,
        cases,
        worst,
    }
}

const fn bra(pose: &'static str, cases: usize, worst: f32) -> Known {
    Known {
        garment: BRA_THROUGH_TEE,
        pose,
        cases,
        worst,
    }
}

#[test]
fn with_correctives_the_tee_covers_the_cast_at_rest() {
    with(Stance::Rest, &[]);
}

#[test]
fn with_correctives_the_tee_covers_the_cast_walking() {
    with(
        Stance::Clip {
            name: "Walk",
            frames: 18,
        },
        &[tee("Walk@18", 39, 0.0090), bra("Walk@18", 7, 0.0102)],
    );
}

#[test]
fn with_correctives_the_tee_covers_the_cast_through_the_walk_first_third() {
    gate_walk_in(
        gate_library_with_correctives(),
        WALK_FRAMES[0],
        &walk_records((54, 0.0134), (21, 0.0149)),
    );
}

#[test]
fn with_correctives_the_tee_covers_the_cast_through_the_walk_second_third() {
    gate_walk_in(
        gate_library_with_correctives(),
        WALK_FRAMES[1],
        &walk_records((54, 0.0141), (20, 0.0104)),
    );
}

#[test]
fn with_correctives_the_tee_covers_the_cast_through_the_walk_last_third() {
    gate_walk_in(
        gate_library_with_correctives(),
        WALK_FRAMES[2],
        &walk_records((54, 0.0145), (20, 0.0116)),
    );
}

#[test]
fn with_correctives_the_tee_covers_the_cast_with_the_arms_forward() {
    with(
        Stance::Held(HeldPose::ArmsForward),
        &[
            tee("ArmsForward", 43, 0.0072),
            bra("ArmsForward", 6, 0.0030),
        ],
    );
}

#[test]
fn with_correctives_the_tee_covers_the_cast_with_the_arms_out() {
    with(Stance::Held(HeldPose::TPose), &[]);
}

#[test]
fn with_correctives_the_tee_covers_the_cast_with_the_arms_raised() {
    with(
        Stance::Held(HeldPose::ArmsRaised),
        &[tee("ArmsRaised", 1, 0.0025), bra("ArmsRaised", 1, 0.0023)],
    );
}

#[test]
fn with_correctives_the_tee_covers_the_cast_with_the_elbows_bent() {
    with(Stance::Held(HeldPose::ElbowsBent), &[]);
}

#[test]
fn with_correctives_the_tee_covers_the_cast_with_the_knees_bent() {
    with(Stance::Held(HeldPose::KneesBent), &[]);
}

// The motion check with the correctives (asset-quality §3, stability in
// motion): no garment corner jolts more than the skin near it does, as
// the correctives blend in and out through the stride. With them the
// test tee jolts as it does without them (`garment_checks`' records): at
// most 4.06 mm at its armpit in the walk (frame 34) and 11.65 mm in the
// run (frame 28); the correctives add none.

#[test]
fn with_correctives_the_tee_stays_steady_in_the_idle() {
    let stage =
        Stage::new("pose-correctives-idle", gate_library_with_correctives()).expect("the stage");
    check_motion(&stage, &realistic_cast(), "Idle", &[]);
}

#[test]
fn with_correctives_the_tee_stays_steady_in_the_walk() {
    let stage = Stage::new(
        "pose-correctives-motion-walk",
        gate_library_with_correctives(),
    )
    .expect("the stage");
    check_motion(
        &stage,
        &realistic_cast(),
        "Walk",
        &[Record {
            what: "TestTee pop",
            case: "Walk",
            most: 0.0041,
        }],
    );
}

#[test]
fn with_correctives_the_tee_stays_steady_in_the_run() {
    let stage = Stage::new(
        "pose-correctives-motion-run",
        gate_library_with_correctives(),
    )
    .expect("the stage");
    check_motion(
        &stage,
        &realistic_cast(),
        "Run",
        &[Record {
            what: "TestTee pop",
            case: "Run",
            most: 0.0117,
        }],
    );
}
