//! Checks a content item must pass in motion and on the feet
//! (`docs/development/character-studio/asset-quality.md` §3 "Stability in
//! motion" and "Footwear", §4 "Motion"), as helpers a content track calls
//! on its own items, and the tests that hold the shipped items to them.
//!
//! - [`motion`]: a clip played on a character, consecutive frames sampled
//!   as a game poses them (`character_capture`'s stage). No **popping**:
//!   a garment corner's jolt (the change in its frame-to-frame step, its
//!   second difference) is no more than [`LEVER`] times the most any skin
//!   corner within [`NEAR`] of it jolts that frame, plus [`POP`]. Cloth
//!   that moves with the body jolts as the body near it does, however hard
//!   the clip moves it (a clip's keys turn a limb faster or slower from one
//!   frame to the next, and cloth standing off the skin, a sole, the
//!   bridge between the legs, swings a little wider than the skin); a pop
//!   (a shape weight that jumps, a twist that wraps round) is a jolt no
//!   skin near it has.
//!   No **tearing**: corners made at one place (a seam
//!   split for its normals or texture) stay within [`TEAR`]. And no hair
//!   corner goes more than [`HAIR_INSIDE`] inside the head.
//! - [`footwear`]: a shoe on a character in a pose: no covered foot corner
//!   through the sole or the upper (a corner the shoe covers at rest, whose
//!   ray out along its normal meets the shoe then, meets no shoe in the
//!   pose: it is outside), and at rest the sole's tread flat on the floor
//!   within [`SOLE_FLAT`].
//!
//! A content specialist checks an item with these on a stage of their own
//! library: `motion(&stage.assets, &description, "Walk", 2..44)` and
//! `footwear(&stage.assets, &description, stance)`, then asserts the
//! report's [`MotionReport::problems`] or [`FootReport::problems`] is
//! empty.

use std::collections::HashMap;
use std::ops::Range;

use axcenity_assets::Assets;
use axcenity_avatar::fit::Surface;
use axcenity_avatar::{Description, WornItem};
use axcenity_render::glam::Vec3;

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{Posed, TEE, benchmarks, gate_library, hem_corners, worn, worn_in};

/// How much more a garment corner may jolt in a frame (its second
/// difference, metres, at 60 frames a second) than the skin near it does,
/// times [`LEVER`]. A pop moves a corner by its whole size in a frame.
pub(crate) const POP: f32 = 0.002;
/// How much wider than the skin near it cloth may swing in a turn: cloth
/// standing a few centimetres off the skin is a little farther from the
/// joint than the skin is.
pub(crate) const LEVER: f32 = 1.25;
/// How near the skin that sets a garment corner's allowance is: about the
/// reach of the bones that move a piece of cloth.
pub(crate) const NEAR: f32 = 0.10;
/// The most corners made at one place may come apart (a split seam, at
/// rest one point): skinned by the same weights they never do.
pub(crate) const TEAR: f32 = 0.0002;
/// The deepest a hair corner may be inside the head (§4 "Motion").
pub(crate) const HAIR_INSIDE: f32 = 0.001;
/// How far from the head's skin a hair corner is looked for: far enough
/// to find one deep inside the head (about 20 cm across).
const HEAD_REACH: f32 = 0.15;
/// How far from flat the sole's tread may be at rest, and from the floor.
pub(crate) const SOLE_FLAT: f32 = 0.002;
/// The deepest a foot corner may be outside its shoe (as the fit gate's
/// poke-through, §3 "Poke-through").
pub(crate) const FOOT_OUT: f32 = 0.001;

/// What one garment did over a clip.
#[derive(Clone, Debug, Default)]
pub(crate) struct GarmentMotion {
    pub name: String,
    /// The most a corner jolted past what the skin near it allows (m), and
    /// the frame it was at.
    pub pop_most: f32,
    pub pop_frame: u32,
    /// Where that corner was then (the file's space), to find it.
    pub pop_at: Vec3,
    /// The farthest corners made at one place came apart (m).
    pub tear_most: f32,
}

/// What a clip did to a character's garments and hair.
#[derive(Clone, Debug, Default)]
pub(crate) struct MotionReport {
    pub character: String,
    pub clip: String,
    pub frames: usize,
    pub garments: Vec<GarmentMotion>,
    /// Hair corners more than [`HAIR_INSIDE`] inside the head (summed over
    /// the frames), and the deepest (m, positive inside).
    pub hair_inside: usize,
    pub hair_deepest: f32,
}

/// A limit an item broke: what (a garment's name and the check, `hair`),
/// in which clip or pose, by how much (metres), and a line to read.
#[derive(Clone, Debug)]
pub(crate) struct Problem {
    pub what: String,
    pub case: String,
    pub amount: f32,
    pub line: String,
}

/// A recorded open defect of a draft item: `what` in `case` may break its
/// limit by up to `most` metres. Lower these as items improve; never
/// raise one to pass an item, and never record a `production` item.
pub(crate) struct Record {
    pub what: &'static str,
    pub case: &'static str,
    pub most: f32,
}

/// The problems no record allows, one line each.
pub(crate) fn unrecorded(problems: &[Problem], records: &[Record]) -> Vec<String> {
    problems
        .iter()
        .filter(|p| {
            !records
                .iter()
                .any(|r| r.what == p.what && r.case == p.case && p.amount <= r.most + 1e-5)
        })
        .map(|p| p.line.clone())
        .collect()
}

/// Records no longer earned: one whose defect no problem shows any more
/// (remove it), or that the worst problem it allows is now more than
/// [`SLACK`] under (lower it). Records are ceilings that follow the item
/// down, and a check that stopped finding a recorded defect is caught.
pub(crate) fn stale(problems: &[Problem], records: &[Record]) -> Vec<String> {
    records
        .iter()
        .filter_map(|r| {
            let worst = problems
                .iter()
                .filter(|p| p.what == r.what && p.case == r.case)
                .map(|p| p.amount)
                .reduce(f32::max);
            match worst {
                None => Some(format!(
                    "{} in {}: recorded {:.2} mm, now none: remove the record",
                    r.what,
                    r.case,
                    r.most * 1000.0
                )),
                Some(worst) if worst < r.most - SLACK => Some(format!(
                    "{} in {}: recorded {:.2} mm, now {:.2} mm: lower the record",
                    r.what,
                    r.case,
                    r.most * 1000.0,
                    worst * 1000.0
                )),
                Some(_) => None,
            }
        })
        .collect()
}

/// How far under its record a defect may fall before the record must come
/// down to it.
pub(crate) const SLACK: f32 = 0.001;

impl MotionReport {
    /// Each limit broken.
    pub fn problems(&self) -> Vec<Problem> {
        let mut out = Vec::new();
        let mut add = |what: String, amount: f32, line: String| {
            out.push(Problem {
                what,
                case: self.clip.clone(),
                amount,
                line: format!("{} {} {line}", self.character, self.clip),
            });
        };
        for garment in &self.garments {
            if garment.pop_most > POP {
                add(
                    format!("{} pop", garment.name),
                    garment.pop_most,
                    format!(
                        "{}: a corner jolted {:.2} mm past the skin near it at frame {}",
                        garment.name,
                        garment.pop_most * 1000.0,
                        garment.pop_frame
                    ),
                );
            }
            if garment.tear_most > TEAR {
                add(
                    format!("{} tear", garment.name),
                    garment.tear_most,
                    format!(
                        "{}: a seam opened {:.2} mm",
                        garment.name,
                        garment.tear_most * 1000.0
                    ),
                );
            }
        }
        if self.hair_inside > 0 {
            add(
                "hair".to_owned(),
                self.hair_deepest,
                format!(
                    "{} hair corners inside the head, the deepest {:.2} mm",
                    self.hair_inside,
                    self.hair_deepest * 1000.0
                ),
            );
        }
        out
    }
}

/// For each corner of `mesh`, the skin corners within [`NEAR`] of it (as
/// both are now).
fn near_skin(skin: &Posed, mesh: &Posed) -> Vec<Vec<u32>> {
    let cell = |p: Vec3| (p / NEAR).floor().as_ivec3().to_array();
    let mut grid: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
    for (k, p) in skin.positions.iter().enumerate() {
        grid.entry(cell(*p)).or_default().push(k as u32);
    }
    mesh.positions
        .iter()
        .map(|p| {
            let [x, y, z] = cell(*p);
            let mut out = Vec::new();
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        for k in grid.get(&[x + dx, y + dy, z + dz]).into_iter().flatten() {
                            if skin.positions[*k as usize].distance(*p) <= NEAR {
                                out.push(*k);
                            }
                        }
                    }
                }
            }
            out
        })
        .collect()
}

/// Each point's jolt (the change in its step, its second difference) over
/// three frames.
fn jolts(before: &[Vec3], then: &[Vec3], now: &[Vec3]) -> Vec<f32> {
    before
        .iter()
        .zip(then)
        .zip(now)
        .map(|((a, b), c)| (*c - *b * 2.0 + *a).length())
        .collect()
}

/// Groups of corners made at one place (more than one each).
fn welds(mesh: &Posed) -> Vec<Vec<usize>> {
    let mut groups: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (k, p) in mesh.positions.iter().enumerate() {
        groups
            .entry((*p * 1e5).round().as_i64vec3().to_array())
            .or_default()
            .push(k);
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
}

/// One frame of a garment against the skin under it, into `out`: each
/// corner's jolt over its last two places (`last`) and `now`, against
/// [`LEVER`] times the most the skin near it (`near`, by corner) jolted
/// (`skin_jolts`; none in the first two frames), and how far apart the
/// corners of each weld group are now.
fn step(
    out: &mut GarmentMotion,
    frame: u32,
    skin_jolts: Option<&[f32]>,
    near: &[Vec<u32>],
    last: &[Vec<Vec3>],
    now: &[Vec3],
    welds: &[Vec<usize>],
) {
    if let (Some(skin), [a, b]) = (skin_jolts, last) {
        let cloth = jolts(a, b, now);
        for (k, (jolt, near)) in cloth.iter().zip(near).enumerate() {
            let skin_most = near
                .iter()
                .map(|k| skin[*k as usize])
                .fold(0.0f32, f32::max);
            let past = jolt - LEVER * skin_most;
            if past > out.pop_most {
                out.pop_most = past;
                out.pop_frame = frame;
                out.pop_at = now[k];
            }
        }
    }
    for group in welds {
        let a = now[group[0]];
        for k in &group[1..] {
            out.tear_most = out.tear_most.max(a.distance(now[*k]));
        }
    }
}

/// Plays `clip` (a starter clip, `Walk`, `Run`, `Idle`) on `description`
/// and samples `frames` (sixtieths of a second from its start, at least
/// 2): see the module's notes.
pub(crate) fn motion(
    assets: &Assets,
    description: &Description,
    clip: &'static str,
    frames: Range<u32>,
) -> MotionReport {
    let first = frames.start.max(2);
    let (mut world, mut animators) = stage_world(
        assets,
        description,
        Stance::Clip {
            name: clip,
            frames: first,
        },
        &[],
    )
    .unwrap();
    let mut report = MotionReport {
        character: description.name.clone(),
        clip: clip.to_owned(),
        ..MotionReport::default()
    };
    // The seams are found at rest, where corners made at one place are
    // still at one place (a seam already split in the first frame sampled
    // would be two places there, and never be seen to tear).
    let (_, rest_garments) = worn(assets, description, Stance::Rest);
    // Per garment: the skin near each corner, its welds and its last two
    // places; the skin's last two places.
    struct Track {
        near: Vec<Vec<u32>>,
        welds: Vec<Vec<usize>>,
        last: Vec<Vec<Vec3>>,
    }
    let mut tracks: Vec<Track> = Vec::new();
    let mut skin_last: Vec<Vec<Vec3>> = Vec::new();
    for frame in first..frames.end.max(first + 1) {
        let (body, garments, hair) = worn_in(&world, assets, &animators);
        if tracks.is_empty() {
            for garment in &garments {
                let rest = rest_garments
                    .iter()
                    .find(|g| {
                        g.name == garment.name && g.positions.len() == garment.positions.len()
                    })
                    .unwrap_or(garment);
                tracks.push(Track {
                    near: near_skin(&body, garment),
                    welds: welds(rest),
                    last: Vec::new(),
                });
                report.garments.push(GarmentMotion {
                    name: garment.name.clone(),
                    ..GarmentMotion::default()
                });
            }
        }
        let skin_jolts =
            (skin_last.len() == 2).then(|| jolts(&skin_last[0], &skin_last[1], &body.positions));
        for ((garment, track), out) in garments.iter().zip(&mut tracks).zip(&mut report.garments) {
            step(
                out,
                frame,
                skin_jolts.as_deref(),
                &track.near,
                &track.last,
                &garment.positions,
                &track.welds,
            );
            track.last.push(garment.positions.clone());
            if track.last.len() > 2 {
                track.last.remove(0);
            }
        }
        skin_last.push(body.positions.clone());
        if skin_last.len() > 2 {
            skin_last.remove(0);
        }
        if !hair.is_empty() {
            let (count, deepest) = hair_in_head(&body, &hair);
            report.hair_inside += count;
            report.hair_deepest = report.hair_deepest.max(deepest);
        }
        report.frames += 1;
        animators.update(&mut world, assets, 1.0 / 60.0);
    }
    report
}

/// Hair corners more than [`HAIR_INSIDE`] inside the body's skin (over the
/// head: the skin above the neck's middle), and the deepest (positive
/// inside).
fn hair_in_head(body: &Posed, hair: &[Posed]) -> (usize, f32) {
    let top = body.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
    // The head and the top of the neck: the top 15% of the body's height
    // above its lowest point is far more than a head.
    let low = body.positions.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    let from = top - 0.15 * (top - low);
    let head: Vec<[u32; 3]> = body
        .triangles
        .iter()
        .copied()
        .filter(|t| t.iter().all(|v| body.positions[*v as usize].y > from))
        .collect();
    let surface = Surface::new(body.positions.clone(), body.normals.clone(), head);
    let mut count = 0;
    let mut deepest = 0.0f32;
    for strand in hair {
        for p in &strand.positions {
            if p.y < from {
                continue;
            }
            let Some(hit) = surface.closest(*p, HEAD_REACH) else {
                continue;
            };
            let inside = -hit.signed(*p);
            if inside > HAIR_INSIDE && hit.is_over(*p) {
                count += 1;
            }
            deepest = deepest.max(inside.max(0.0) * f32::from(hit.is_over(*p)));
        }
    }
    (count, deepest)
}

/// What a shoe did on a character in a pose.
#[derive(Clone, Debug, Default)]
pub(crate) struct FootReport {
    pub character: String,
    pub pose: String,
    pub shoe: String,
    /// Foot corners the shoe covers at rest, and those outside it now (by
    /// more than [`FOOT_OUT`]), and the farthest (m).
    pub covered: usize,
    pub through: usize,
    pub through_most: f32,
    /// At rest: the tread's lowest and highest point above the floor (m;
    /// the floor is the file's `y = 0`, where the body stands).
    pub tread_low: f32,
    pub tread_high: f32,
}

impl FootReport {
    /// Each limit broken.
    pub fn problems(&self) -> Vec<Problem> {
        let mut out = Vec::new();
        let mut add = |what: &str, amount: f32, line: String| {
            out.push(Problem {
                what: format!("{} {what}", self.shoe),
                case: self.pose.clone(),
                amount,
                line: format!("{} {} {} {line}", self.character, self.pose, self.shoe),
            });
        };
        if self.covered == 0 {
            add("cover", f32::INFINITY, "covers no foot corner".to_owned());
        }
        if self.through > 0 {
            add(
                "through",
                self.through_most,
                format!(
                    "{} foot corners out of the shoe, the farthest {:.2} mm",
                    self.through,
                    self.through_most * 1000.0
                ),
            );
        }
        let off = self.tread_low.abs().max(self.tread_high - self.tread_low);
        if self.pose == "rest" && off > SOLE_FLAT {
            add(
                "tread",
                off,
                format!(
                    "the tread from {:.2} to {:.2} mm above the floor",
                    self.tread_low * 1000.0,
                    self.tread_high * 1000.0
                ),
            );
        }
        out
    }
}

/// Which body corners the shoe covers, posed as `body` and `shoe` are: a
/// ray from the corner out along its normal meets the shoe within 4 cm,
/// away from its hems (the collar and the opening). `None` for a corner
/// it doesn't.
fn foot_cover(body: &Posed, shoe: &Posed) -> Vec<bool> {
    let surface = Surface::new(
        shoe.positions.clone(),
        shoe.normals.clone(),
        shoe.triangles.clone(),
    );
    let hem = hem_corners(shoe);
    let mut drawn = vec![false; body.positions.len()];
    for t in &body.triangles {
        for v in t {
            drawn[*v as usize] = true;
        }
    }
    body.positions
        .iter()
        .zip(&body.normals)
        .enumerate()
        .map(|(k, (p, n))| {
            drawn[k]
                && *n != Vec3::ZERO
                && surface.cast(*p, *n, 0.04).is_some_and(|hit| {
                    let t = shoe.triangles[hit.triangle as usize];
                    !t.iter().any(|v| hem[*v as usize])
                })
        })
        .collect()
}

/// The footwear check on `description` (wearing its shoes) in `stance`:
/// see the module's notes. One report per shoe part.
pub(crate) fn footwear(
    assets: &Assets,
    description: &Description,
    stance: Stance,
) -> Vec<FootReport> {
    let (body_rest, garments_rest) = worn(assets, description, Stance::Rest);
    let (body, garments) = if stance == Stance::Rest {
        (body_rest.clone(), garments_rest.clone())
    } else {
        worn(assets, description, stance)
    };
    let pose = match stance {
        Stance::Rest => "rest".to_owned(),
        Stance::Held(pose) => format!("{pose:?}"),
        Stance::Clip { name, frames } => format!("{name}@{frames}"),
        Stance::Idle => "idle".to_owned(),
    };
    garments_rest
        .iter()
        .zip(&garments)
        .filter(|(g, _)| g.name.starts_with("Shoes_"))
        .map(|(shoe_rest, shoe)| {
            foot_report(&description.name, &pose, &body_rest, shoe_rest, &body, shoe)
        })
        .collect()
}

/// The footwear check of one shoe: `body_rest` and `shoe_rest` at rest,
/// `body` and `shoe` in the pose named `pose` (`rest` for rest itself).
fn foot_report(
    character: &str,
    pose: &str,
    body_rest: &Posed,
    shoe_rest: &Posed,
    body: &Posed,
    shoe: &Posed,
) -> FootReport {
    let covered = foot_cover(body_rest, shoe_rest);
    let still = foot_cover(body, shoe);
    let surface = Surface::new(
        shoe.positions.clone(),
        shoe.normals.clone(),
        shoe.triangles.clone(),
    );
    let mut report = FootReport {
        character: character.to_owned(),
        pose: pose.to_owned(),
        shoe: shoe.name.clone(),
        covered: covered.iter().filter(|c| **c).count(),
        ..FootReport::default()
    };
    for (k, p) in body.positions.iter().enumerate() {
        if !covered[k] || still[k] {
            continue;
        }
        // Out of the shoe: how far from it.
        let far = surface.closest(*p, 0.05).map_or(0.05, |hit| hit.distance);
        if far > FOOT_OUT {
            report.through += 1;
            report.through_most = report.through_most.max(far);
        }
    }
    // The tread: the shoe's faces that face the floor (within 25 degrees
    // of straight down) under the foot, at rest.
    let mut low = f32::MAX;
    let mut high = f32::MIN;
    let lowest = shoe_rest
        .positions
        .iter()
        .map(|p| p.y)
        .fold(f32::MAX, f32::min);
    for t in &shoe_rest.triangles {
        let [a, b, c] = t.map(|v| shoe_rest.positions[v as usize]);
        let n = (b - a).cross(c - a).normalize_or_zero();
        // The tread is the bottom: faces down and within 1 cm of the
        // lowest point (not the inner floor above it, nor a toe cap's
        // underside that curves up).
        if n.y < -0.9 && [a, b, c].iter().all(|p| p.y < lowest + 0.01) {
            for p in [a, b, c] {
                low = low.min(p.y);
                high = high.max(p.y);
            }
        }
    }
    if low <= high {
        report.tread_low = low;
        report.tread_high = high;
    }
    report
}

/// The realistic cast (both frames) in the test tee over the underwear.
fn realistic_cast() -> Vec<Description> {
    super::fit_gate_tests::bodies()
        .into_iter()
        .filter(|d| d.name.contains("-cast-"))
        .map(|mut d| {
            d.items.push(WornItem::new(TEE));
            d
        })
        .collect()
}

/// The stylized cast in the tee, cargo pants and sneakers, each with a
/// hairstyle (a different one for each body, in turn).
fn stylized_cast() -> Vec<Description> {
    let hair = [
        "axcenity/hair-bob",
        "axcenity/hair-buzz",
        "axcenity/hair-crop",
        "axcenity/hair-long",
        "axcenity/hair-ponytail",
        "axcenity/hair-swept",
    ];
    benchmarks("stylized-")
        .into_iter()
        .filter(|d| d.name.contains("-cast-"))
        .enumerate()
        .map(|(k, mut d)| {
            for id in [
                "axcenity/tee",
                "axcenity/cargo-pants",
                "axcenity/sneakers",
                hair[k % hair.len()],
            ] {
                if !d.items.iter().any(|worn| worn.id == id) {
                    d.items.push(WornItem::new(id));
                }
            }
            d
        })
        .collect()
}

/// A clip's frames for the checks: one stride of the walk and the run
/// from their start (the clip played alone holds its last frame from
/// about frame 44 on), and a second of the idle.
fn frames_of(clip: &str) -> Range<u32> {
    match clip {
        "Idle" => 2..62,
        "Run" => 2..35,
        _ => 2..44,
    }
}

pub(super) fn check_motion(
    stage: &Stage,
    cast: &[Description],
    clip: &'static str,
    records: &[Record],
) {
    let mut problems = Vec::new();
    for description in cast {
        let report = motion(&stage.assets, description, clip, frames_of(clip));
        println!(
            "{} {clip}: {} frames; {}; hair inside {} (deepest {:.2} mm)",
            report.character,
            report.frames,
            report
                .garments
                .iter()
                .map(|g| format!(
                    "{} pop {:.2} mm @{} at {:.3} tear {:.3} mm",
                    g.name,
                    g.pop_most * 1000.0,
                    g.pop_frame,
                    g.pop_at,
                    g.tear_most * 1000.0
                ))
                .collect::<Vec<_>>()
                .join(", "),
            report.hair_inside,
            report.hair_deepest * 1000.0
        );
        assert!(
            !report.garments.is_empty(),
            "{}: no garments",
            description.name
        );
        problems.extend(report.problems());
    }
    let left = unrecorded(&problems, records);
    assert!(left.is_empty(), "{left:#?}");
    let old = stale(&problems, records);
    assert!(old.is_empty(), "{old:#?}");
}

// What the motion and footwear checks still find (wave 4, CO9 part 2), as
// records of open defects of draft items (`co9-fit-2.md`, open issues):
// not accepted behaviour. Lower them as the items improve; never raise
// one. The underwear has none: it moves with the skin in every clip.

/// The test tee's armpit, where the cloth bridging the arm and the side
/// carries large copies of the swing correctives (the posed fit's
/// following), jolts as the arm swings through hanging.
const WALK_RECORDS: [Record; 1] = [Record {
    what: "TestTee pop",
    case: "Walk",
    most: 0.0041,
}];
const RUN_RECORDS: [Record; 1] = [Record {
    what: "TestTee pop",
    case: "Run",
    most: 0.0117,
}];
/// The stylized cargo pants between the legs (the bridge under the
/// crotch) jolt past the thighs as a stride turns; four of the six
/// stylized hairstyles (bob, long, ponytail, swept) have corners up to
/// 11 mm inside the head, still or moving (the cards' roots).
const STYLIZED_WALK_RECORDS: [Record; 2] = [
    Record {
        what: "Pants_Cargo pop",
        case: "Walk",
        most: 0.0056,
    },
    Record {
        what: "hair",
        case: "Walk",
        most: 0.0114,
    },
];
const STYLIZED_RUN_RECORDS: [Record; 2] = [
    Record {
        what: "Pants_Cargo pop",
        case: "Run",
        most: 0.0032,
    },
    Record {
        what: "hair",
        case: "Run",
        most: 0.0112,
    },
];
/// The stylized sneakers: in the walk's stride a few corners of the foot
/// (1 to 4) come out of the shoe by up to 18 mm; at rest the tread is not
/// flat (from 3.6 mm under the floor to 4.1 mm over it, up to 8.4 mm from
/// end to end on some bodies).
const SNEAKER_RECORDS: [Record; 2] = [
    Record {
        what: "Shoes_Sneakers through",
        case: "Walk@18",
        most: 0.0183,
    },
    Record {
        what: "Shoes_Sneakers tread",
        case: "rest",
        most: 0.0084,
    },
];

#[test]
fn realistic_garments_stay_steady_in_the_idle() {
    let stage = Stage::new("motion-realistic-idle", gate_library()).expect("the stage");
    check_motion(&stage, &realistic_cast(), "Idle", &[]);
}

#[test]
fn realistic_garments_stay_steady_in_the_walk() {
    let stage = Stage::new("motion-realistic-walk", gate_library()).expect("the stage");
    check_motion(&stage, &realistic_cast(), "Walk", &WALK_RECORDS);
}

#[test]
fn realistic_garments_stay_steady_in_the_run() {
    let stage = Stage::new("motion-realistic-run", gate_library()).expect("the stage");
    check_motion(&stage, &realistic_cast(), "Run", &RUN_RECORDS);
}

#[test]
fn stylized_garments_and_hair_stay_steady_in_the_walk() {
    let stage = Stage::new("motion-stylized-walk", &repository_library()).expect("the stage");
    check_motion(&stage, &stylized_cast(), "Walk", &STYLIZED_WALK_RECORDS);
}

#[test]
fn stylized_garments_and_hair_stay_steady_in_the_run() {
    let stage = Stage::new("motion-stylized-run", &repository_library()).expect("the stage");
    check_motion(&stage, &stylized_cast(), "Run", &STYLIZED_RUN_RECORDS);
}

/// The stylized sneakers on the stylized cast: no foot through the sole
/// or the upper at rest or in the held poses that move the legs, and the
/// tread flat on the floor at rest.
#[test]
fn the_stylized_sneakers_hold_the_feet() {
    let stage = Stage::new("footwear-stylized", &repository_library()).expect("the stage");
    let mut problems = Vec::new();
    for description in stylized_cast() {
        for stance in [
            Stance::Rest,
            Stance::Held(HeldPose::KneesBent),
            Stance::Clip {
                name: "Walk",
                frames: 18,
            },
        ] {
            let reports = footwear(&stage.assets, &description, stance);
            assert!(!reports.is_empty(), "{}: no shoes", description.name);
            for report in &reports {
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
    let left = unrecorded(&problems, &SNEAKER_RECORDS);
    assert!(left.is_empty(), "{left:#?}");
    // The recorded defects are still found: a check that stopped finding
    // them would pass any shoe.
    let old = stale(&problems, &SNEAKER_RECORDS);
    assert!(old.is_empty(), "{old:#?}");
}

/// The checks catch what they are for: a garment corner moved 1 cm for
/// one frame is a pop; corners made at one place moved apart are a tear;
/// a hair corner pushed into the head is inside it.
#[test]
fn the_motion_checks_catch_a_pop_a_tear_and_hair_in_the_head() {
    // A unit square of skin and a garment over it.
    let skin = Posed {
        name: "Body".to_owned(),
        positions: vec![
            Vec3::new(0.0, 1.6, 0.0),
            Vec3::new(0.08, 1.6, 0.0),
            Vec3::new(0.0, 1.68, 0.0),
            Vec3::new(0.08, 1.68, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ],
        normals: vec![Vec3::Z; 5],
        triangles: vec![[0, 1, 2], [1, 3, 2]],
    };
    let garment = Posed {
        name: "Cloth".to_owned(),
        positions: vec![Vec3::new(0.04, 1.64, 0.003), Vec3::new(0.04, 1.64, 0.003)],
        normals: vec![Vec3::Z; 2],
        triangles: vec![],
    };
    // The skin near the cloth: the square's four corners, not the far one
    // (1.64 m below it).
    assert_eq!(near_skin(&skin, &garment)[0].len(), 4);
    assert_eq!(welds(&garment), vec![vec![0, 1]]);
    // A corner that steps 1 cm out of line in one frame jolts 1 cm; one
    // moving steadily doesn't.
    let at = |x: f32| vec![Vec3::new(x, 0.0, 0.0)];
    assert!((jolts(&at(0.0), &at(0.01), &at(0.02))[0]).abs() < 1e-6);
    assert!((jolts(&at(0.0), &at(0.01), &at(0.03))[0] - 0.01).abs() < 1e-6);
    // Hair 2 mm inside the skin (behind it) over the head.
    let hair = Posed {
        positions: vec![Vec3::new(0.04, 1.64, -0.002), Vec3::new(0.04, 1.64, 0.004)],
        ..garment.clone()
    };
    let (count, deepest) = hair_in_head(&skin, &[hair]);
    assert_eq!(count, 1);
    assert!((deepest - 0.002).abs() < 1e-4, "{deepest}");
    // A report with a pop and a tear names both.
    let report = MotionReport {
        character: "c".to_owned(),
        clip: "Walk".to_owned(),
        frames: 3,
        garments: vec![GarmentMotion {
            name: "Cloth".to_owned(),
            pop_most: 0.01,
            pop_frame: 2,
            pop_at: Vec3::ZERO,
            tear_most: 0.001,
        }],
        hair_inside: 1,
        hair_deepest: 0.002,
    };
    assert_eq!(report.problems().len(), 3, "{:?}", report.problems());
}

/// A garment that moves with the skin passes a frame; one corner thrown
/// 1 cm out of line for one frame is a pop; corners made at one place
/// coming 1 mm apart are a tear (`step`, the frame-by-frame core of
/// [`motion`]).
#[test]
fn a_frame_with_a_corner_thrown_out_of_line_is_a_pop_and_a_split_seam_a_tear() {
    // Skin and cloth both speeding up the same way: they jolt alike.
    let speeding = |k: f32| Vec3::new(0.01 * k + 0.002 * k * k, 0.0, 0.0);
    let skin_jolts = jolts(&[speeding(0.0)], &[speeding(1.0)], &[speeding(2.0)]);
    let near = vec![vec![0u32]; 3];
    // Three cloth corners; the last two made at one place.
    let cloth = |k: f32| {
        let at = speeding(k);
        vec![
            at + Vec3::Y * 0.01,
            at + Vec3::Z * 0.01,
            at + Vec3::Z * 0.01,
        ]
    };
    let welds = vec![vec![1usize, 2]];
    let mut steady = GarmentMotion::default();
    step(
        &mut steady,
        2,
        Some(&skin_jolts),
        &near,
        &[cloth(0.0), cloth(1.0)],
        &cloth(2.0),
        &welds,
    );
    assert!(steady.pop_most <= 1e-6, "{steady:?}");
    assert!(steady.tear_most <= 1e-6, "{steady:?}");
    // The first corner thrown 1 cm out of line in the middle frame: its
    // jolt over the next frame is 2 cm the skin doesn't have.
    let mut thrown = cloth(1.0);
    thrown[0] += Vec3::X * 0.01;
    let mut popped = GarmentMotion::default();
    step(
        &mut popped,
        2,
        Some(&skin_jolts),
        &near,
        &[cloth(0.0), thrown],
        &cloth(2.0),
        &welds,
    );
    assert!(popped.pop_most > POP, "{popped:?}");
    assert_eq!(popped.pop_frame, 2);
    // The welded corners 1 mm apart now.
    let mut split = cloth(2.0);
    split[2] += Vec3::Y * 0.001;
    let mut torn = GarmentMotion::default();
    step(
        &mut torn,
        2,
        Some(&skin_jolts),
        &near,
        &[cloth(0.0), cloth(1.0)],
        &split,
        &welds,
    );
    assert!(torn.tear_most > TEAR, "{torn:?}");
    let report = MotionReport {
        character: "c".to_owned(),
        clip: "Walk".to_owned(),
        frames: 3,
        garments: vec![
            GarmentMotion {
                name: "Popped".to_owned(),
                ..popped
            },
            GarmentMotion {
                name: "Torn".to_owned(),
                ..torn
            },
            GarmentMotion {
                name: "Steady".to_owned(),
                ..steady
            },
        ],
        ..MotionReport::default()
    };
    let what: Vec<String> = report.problems().into_iter().map(|p| p.what).collect();
    assert_eq!(what, ["Popped pop", "Torn tear"]);
}

/// A box shoe (outside faces out, 10 × 8 × 24 cm, its sole on the floor;
/// the bottom raised by `tilt` at its front) as one closed mesh.
fn box_shoe(tilt: f32) -> Posed {
    let (x, y, z) = (0.05, 0.08, 0.12);
    let mut positions = Vec::new();
    for k in 0..8 {
        let at = Vec3::new(
            if k & 1 == 0 { -x } else { x },
            if k & 2 == 0 { 0.0 } else { y },
            if k & 4 == 0 { -z } else { z },
        );
        // The front of the sole raised: a tread that isn't flat.
        let raised = if at.y == 0.0 && at.z > 0.0 { tilt } else { 0.0 };
        positions.push(at + Vec3::Y * raised);
    }
    // Each side's corners counter-clockwise seen from outside.
    let quads = [
        [0, 1, 5, 4], // bottom (-Y)
        [2, 6, 7, 3], // top (+Y)
        [0, 2, 3, 1], // back (-Z)
        [4, 5, 7, 6], // front (+Z)
        [0, 4, 6, 2], // left (-X)
        [1, 3, 7, 5], // right (+X)
    ];
    let triangles: Vec<[u32; 3]> = quads
        .iter()
        .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
        .collect();
    let normals = axcenity_avatar::fit::vertex_normals(&positions, &triangles);
    Posed {
        name: "Shoes_Box".to_owned(),
        positions,
        normals,
        triangles,
    }
}

/// A foot inside [`box_shoe`]: a sole patch 1 cm over the floor facing
/// down and an instep patch 2 cm under the box's top facing up, `drop`
/// lower.
fn box_foot(drop: f32) -> Posed {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut triangles = Vec::new();
    for (y, normal) in [(0.01, -Vec3::Y), (0.06, Vec3::Y)] {
        let first = positions.len() as u32;
        for i in 0..5 {
            for j in 0..9 {
                positions.push(Vec3::new(
                    -0.03 + 0.015 * i as f32,
                    y - drop,
                    -0.08 + 0.02 * j as f32,
                ));
                normals.push(normal);
            }
        }
        for i in 0..4u32 {
            for j in 0..8u32 {
                let a = first + i * 9 + j;
                triangles.push([a, a + 1, a + 9]);
                triangles.push([a + 1, a + 10, a + 9]);
            }
        }
    }
    Posed {
        name: "Body".to_owned(),
        positions,
        normals,
        triangles,
    }
}

/// The footwear check finds a foot pushed 5 mm through the sole, and a
/// tread 5 mm off flat at rest; a foot inside a flat shoe passes.
#[test]
fn the_footwear_check_finds_a_foot_through_the_sole_and_a_tread_off_flat() {
    let (foot, shoe) = (box_foot(0.0), box_shoe(0.0));
    let fits = foot_report("c", "rest", &foot, &shoe, &foot, &shoe);
    assert_eq!(fits.covered, foot.positions.len(), "{fits:?}");
    assert!(fits.problems().is_empty(), "{:?}", fits.problems());
    // The foot 1.5 cm lower in a pose: its sole 5 mm under the shoe's.
    let through = foot_report("c", "Walk@18", &foot, &shoe, &box_foot(0.015), &shoe);
    assert_eq!(through.through, 45, "{through:?}");
    assert!((through.through_most - 0.005).abs() < 1e-4, "{through:?}");
    let what: Vec<String> = through.problems().into_iter().map(|p| p.what).collect();
    assert_eq!(what, ["Shoes_Box through"]);
    // The tread's front 5 mm up at rest.
    let tilted = box_shoe(0.005);
    let tread = foot_report("c", "rest", &foot, &tilted, &foot, &tilted);
    assert!(
        (tread.tread_high - tread.tread_low - 0.005).abs() < 1e-4,
        "{tread:?}"
    );
    let what: Vec<String> = tread.problems().into_iter().map(|p| p.what).collect();
    assert_eq!(what, ["Shoes_Box tread"]);
}

/// A record whose defect is gone, or which is well over it, is stale.
#[test]
fn a_record_no_longer_earned_is_stale() {
    let problem = |amount| Problem {
        what: "Draft pop".to_owned(),
        case: "Walk".to_owned(),
        amount,
        line: String::new(),
    };
    let records = [Record {
        what: "Draft pop",
        case: "Walk",
        most: 0.005,
    }];
    assert!(stale(&[problem(0.0045)], &records).is_empty());
    assert_eq!(stale(&[problem(0.003)], &records).len(), 1);
    assert_eq!(stale(&[], &records).len(), 1);
}
