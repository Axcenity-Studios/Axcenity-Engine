//! The first licensed T-shirt (wave 5, Paul's decision 13;
//! `docs/development/character-studio/refinement/wave5/tee-licensed.md`):
//! `axcenity/realistic-tee-crew`, BlenderKit's Plain T-shirt (CC0) through
//! the licensed-asset intake, worn over the underwear by every realistic
//! cast and body benchmark, as a game poses it. It is held to
//! `docs/development/character-studio/asset-quality.md` §1 to §3. It is a
//! draft that fails them, so each case carries a record of what it fails
//! now (below); `check` fails a case that gets worse or better than its
//! record, and a production item has none.
//!
//! - **Poke-through** (the fit gate's rule, `fit_gate_tests`): no body
//!   corner more than 1 mm outside the tee where the tee covers it at rest,
//!   no hidden skin showing, and no underwear through the tee: at rest, in
//!   Idle, in the body suite's held poses, with the arms raised forward,
//!   and through the walk's whole stride (every third frame).
//! - **Levels of detail**: each simpler copy clear of the body at rest.
//! - **Stability in motion** (`garment_checks::motion`): Idle, Walk, Run.

use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, Value};

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{
    Failure, Known, Posed, WALK, WALK_FRAMES, bodies, check, cover_at_rest, measure, measure_walk,
    poke_through, pose_name, posed_at, undressed, worn,
};
use super::garment_checks::{Record, check_motion};

pub(super) const TEE: &str = "axcenity/realistic-tee-crew";
/// The part the tee draws as.
pub(super) const PART: &str = "Shirt_TeeCrew";

/// `description` wearing the tee (over the underwear its frame wears by
/// default).
fn dressed(description: &Description) -> Description {
    let mut out = description.clone();
    out.items.push(WornItem::new(TEE));
    out
}

/// A failure that is the tee's, a layer through it, or hidden skin
/// showing (only the tee hides anything here).
fn ours(failure: &Failure) -> bool {
    failure.garment.contains(PART) || failure.garment == super::fit_gate_tests::HOLES
}

/// The gate for `stance` on every realistic cast and body benchmark wearing
/// the tee, on the shipped library, against its records.
fn tee_gate(stance: Stance, known: &[Known]) {
    let label = pose_name(stance);
    let stage =
        Stage::new(&format!("tee-crew-gate-{label}"), &repository_library()).expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
        assert!(
            garments.iter().any(|g| g.name == PART),
            "{}: no {PART}",
            description.name
        );
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
        "tee gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&format!("tee {label}"), &failures, known);
}

/// The walking fit test (decision 13) through the walk's `frames`
/// ([`WALK_FRAMES`]) on every realistic cast and body benchmark wearing
/// the tee, against its records.
fn tee_gate_walk(frames: &[u32], known: &[Known]) {
    let label = format!("{WALK} {frames:?}");
    let stage = Stage::new(
        &format!("tee-crew-gate-walk-{}", frames[0]),
        &repository_library(),
    )
    .expect("the stage");
    let mut failures = Vec::new();
    let mut checked = 0;
    for description in bodies() {
        let dressed = dressed(&description);
        let (body, garments) = worn(&stage.assets, &dressed, Stance::Rest);
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
        "tee gate {label}: {checked} garment checks, {} failing",
        failures.len()
    );
    check(&format!("tee {label}"), &failures, known);
}

#[test]
fn the_licensed_tee_covers_the_cast_at_rest() {
    tee_gate(Stance::Rest, &REST);
}

#[test]
fn the_licensed_tee_covers_the_cast_in_idle() {
    tee_gate(Stance::Idle, &IDLE);
}

#[test]
fn the_licensed_tee_covers_the_cast_through_the_walk_first_third() {
    tee_gate_walk(WALK_FRAMES[0], &WALK_FIRST);
}

#[test]
fn the_licensed_tee_covers_the_cast_through_the_walk_second_third() {
    tee_gate_walk(WALK_FRAMES[1], &WALK_SECOND);
}

#[test]
fn the_licensed_tee_covers_the_cast_through_the_walk_last_third() {
    tee_gate_walk(WALK_FRAMES[2], &WALK_LAST);
}

#[test]
fn the_licensed_tee_covers_the_cast_with_the_arms_raised() {
    tee_gate(Stance::Held(HeldPose::ArmsRaised), &ARMS_RAISED);
}

#[test]
fn the_licensed_tee_covers_the_cast_with_the_arms_forward() {
    tee_gate(Stance::Held(HeldPose::ArmsForward), &ARMS_FORWARD);
}

#[test]
fn the_licensed_tee_covers_the_cast_with_the_arms_out() {
    tee_gate(Stance::Held(HeldPose::TPose), &T_POSE);
}

#[test]
fn the_licensed_tee_covers_the_cast_with_the_elbows_bent() {
    tee_gate(Stance::Held(HeldPose::ElbowsBent), &ELBOWS_BENT);
}

#[test]
fn the_licensed_tee_covers_the_cast_with_the_knees_bent() {
    tee_gate(Stance::Held(HeldPose::KneesBent), &KNEES_BENT);
}

/// The body and the tee's part at rest, the tee at its level of detail
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
    let tee = named(PART)
        .map(|part| posed_at(&world, &stage.assets, &animators, part, level))
        .into_iter()
        .collect();
    (body, tee)
}

/// §2 "LODs": each simpler copy stays clear of the body where the full
/// one covers it, at rest on every cast and body benchmark.
#[test]
fn the_licensed_tee_levels_of_detail_stay_clear_of_the_body() {
    let stage = Stage::new("tee-crew-lods", &repository_library()).expect("the stage");
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
    check("tee levels of detail", &failures, &LODS);
}

/// The realistic cast (not the extremes) wearing the tee, each checked to
/// wear it.
fn cast(stage: &Stage) -> Vec<Description> {
    let cast: Vec<Description> = bodies()
        .into_iter()
        .filter(|d| d.name.contains("-cast-"))
        .map(|d| dressed(&d))
        .collect();
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
fn the_licensed_tee_stays_steady_in_the_idle() {
    let stage = Stage::new("tee-crew-motion-idle", &repository_library()).expect("the stage");
    check_motion(&stage, &cast(&stage), "Idle", &[]);
}

#[test]
fn the_licensed_tee_stays_steady_in_the_walk() {
    let stage = Stage::new("tee-crew-motion-walk", &repository_library()).expect("the stage");
    check_motion(&stage, &cast(&stage), "Walk", &[]);
}

#[test]
fn the_licensed_tee_stays_steady_in_the_run() {
    let stage = Stage::new("tee-crew-motion-run", &repository_library()).expect("the stage");
    check_motion(&stage, &cast(&stage), "Run", &MOTION_RUN);
}

// The records: what the licensed tee, a draft, still fails (open
// defects, not accepted behaviour; `refinement/wave5/tee-licensed.md`
// says where each is and why). Lower them as the item improves; never
// raise one; a production item has none. From the first run of the item
// as committed (wave 5):
//
// - The walk, Idle and the arms forward (every body): the armpit, as
//   every tee before it. The cloth bridging the arm and the side, made
//   with the arm at 45 degrees, folds over itself and flips when the arm
//   hangs and swings, and the skin of the inner arm and the side comes
//   through; on the feminine bodies also the side of the breast by the
//   front of the armpit. With the arms raised forward, the top of the
//   shoulder by the neck, and the sleeve's end riding up the arm.
// - The hems' edges (rest and every pose, 7 to 14 bodies, 28.2 mm):
//   skin and underwear just past the turned hems (below the waist hem,
//   below a sleeve's end). The gate takes two rings of triangles in from
//   an open edge as a hem; a turned hem's open edge is inside, at the end
//   of its turn-up, and its rings are 1.5 to 18 mm wide, so the fold and
//   the cloth just above it count as covering the skin below the fold.
//   Without the turned hems the tee passed rest and the elbows and knees
//   bent (`tee-licensed.md`, experiment E1). A gate rule for turned hems
//   is the lead's decision (measured there, not committed).
// - The underwear through the tee: the briefs and boxer briefs at the
//   same hem edge; the sports bra at the side of the chest in the walk,
//   as with every tee.
// - The simpler copies: LOD2's 1,020 triangles cut inside the skin.
// - The run: the cloth at the front of the armpit jolts 6 mm in one frame
//   as the arm swings through hanging (feminine slim, frame 10).
//
// Seen in the captures and **not counted by any record** (cloth-only
// defects, or skin the gate's records already hold elsewhere on the same
// body; review fixes 1, `tee-licensed.md` section 5):
//
// - T-pose, every cast body: detached strips about 15 to 20 cm long hang
//   diagonally from both underarms (mesh separation). They are the sleeve
//   hems' fold and turn-up triangles under the arm (rest edges 1.2 to
//   4.8 mm), stretched 3 to 111 times between corners that follow the arm
//   and corners that stay with the side (167 mm on masculine average,
//   200 mm on feminine average, from the gate's dumps).
// - Arms forward, every cast body's side view: the same triangles stick
//   out as a white spike at the side of the armpit (177 mm on masculine
//   average, 149 mm on feminine average).
// - Rest and the walk, every cast body: a dark lip about 4 cm long shows
//   above the rib at the back of the neck, right of the spine seen from
//   behind (the character's right), with a light tick at its end; with
//   the right arm forward in the walk it stands up as a light tab of about
//   1 cm. Not a clean back.
// - The walk with the right arm back (Walk@0.367), masculine average,
//   heavyset and muscular: a skin speck of 7 to 11 mm on the back of the
//   right sleeve just above its hem.
// - Arms raised, every cast body (front, side and back): flaps about 10
//   to 15 cm long hang from both sleeves' undersides, the sleeve hems'
//   turn-up stretched (323 to 374 triangles over twice their rest length,
//   the longest edge 299 to 313 mm, on the ten cast bodies' dumps).
// - Rest, the T-pose, the elbows bent, the arms raised and forward, every
//   cast body (review fixes 2): the waist hem's centre back hangs in a point
//   55 to 63 mm below the back hem beside the buttocks, into the cleft and
//   free of the body below them; from the side it reads as a strip hanging
//   behind the thighs (up to 26 to 34 mm behind the body's back edge). In
//   the walk it is 11 to 54 mm, and it stands free of every cast body in 5
//   of the 15 sampled frames (in 2 more on two). The engine fit made it: the
//   refined hem is level (the point is 4 mm without the push-out's
//   between-corners pass), and that pass pushes the hem's lower edge down along the
//   cloth's normal there, round after round: 24 mm at rest, 67 mm after the
//   extremes' lifts (`tee-licensed.md`, review fixes 2).

const fn known(garment: &'static str, pose: &'static str, cases: usize, worst: f32) -> Known {
    Known {
        garment,
        pose,
        cases,
        worst,
    }
}

const BOXERS: &str = "Underwear_BoxerBriefs through Shirt_TeeCrew";
const BRA: &str = "Underwear_Bra through Shirt_TeeCrew";
const BRIEFS: &str = "Underwear_Briefs through Shirt_TeeCrew";

/// The hems' edges, in the poses that leave them where they are at rest.
const fn at_the_hems(pose: &'static str, tee: usize) -> [Known; 3] {
    [
        known(PART, pose, tee, 0.0282),
        known(BOXERS, pose, 1, 0.0151),
        known(BRIEFS, pose, 11, 0.0179),
    ]
}

const REST: [Known; 3] = at_the_hems("rest", 7);
const T_POSE: [Known; 3] = at_the_hems("TPose", 7);
const ELBOWS_BENT: [Known; 3] = at_the_hems("ElbowsBent", 8);
const KNEES_BENT: [Known; 3] = at_the_hems("KneesBent", 7);
const ARMS_RAISED: [Known; 4] = [
    known(PART, "ArmsRaised", 14, 0.0282),
    known(BOXERS, "ArmsRaised", 1, 0.0151),
    known(BRA, "ArmsRaised", 1, 0.0031),
    known(BRIEFS, "ArmsRaised", 11, 0.0179),
];
const IDLE: [Known; 4] = [
    known(PART, "idle", 53, 0.0380),
    known(BOXERS, "idle", 1, 0.0145),
    known(BRA, "idle", 17, 0.0041),
    known(BRIEFS, "idle", 11, 0.0180),
];
const WALK_FIRST: [Known; 4] = [
    known(PART, WALK, 54, 0.0325),
    known(BOXERS, WALK, 2, 0.0141),
    known(BRA, WALK, 17, 0.0057),
    known(BRIEFS, WALK, 12, 0.0185),
];
const WALK_SECOND: [Known; 4] = [
    known(PART, WALK, 54, 0.0313),
    known(BOXERS, WALK, 26, 0.0024),
    known(BRA, WALK, 20, 0.0092),
    known(BRIEFS, WALK, 13, 0.0227),
];
const WALK_LAST: [Known; 4] = [
    known(PART, WALK, 54, 0.0320),
    known(BOXERS, WALK, 2, 0.0142),
    known(BRA, WALK, 10, 0.0074),
    known(BRIEFS, WALK, 12, 0.0183),
];
const ARMS_FORWARD: [Known; 4] = [
    known(PART, "ArmsForward", 54, 0.0302),
    known(BOXERS, "ArmsForward", 1, 0.0140),
    known(BRA, "ArmsForward", 19, 0.0032),
    known(BRIEFS, "ArmsForward", 3, 0.0173),
];
const LODS: [Known; 2] = [
    known("Shirt_TeeCrew LOD1", "rest", 10, 0.0120),
    known("Shirt_TeeCrew LOD2", "rest", 54, 0.0307),
];
const MOTION_RUN: [Record; 1] = [Record {
    what: "Shirt_TeeCrew pop",
    case: "Run",
    most: 0.0061,
}];
