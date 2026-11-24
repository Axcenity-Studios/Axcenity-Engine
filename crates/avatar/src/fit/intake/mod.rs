//! Intake: a garment that was **not** made on the body (a generated or
//! downloaded one, in its own size, place and pose, and as a thick closed
//! shell) made ready for [`fit_garment`](super::fit_garment), which wants
//! one layer of cloth in the body's rest pose and file space.
//!
//! [`prepare`] does, in order:
//!
//! 1. **Winding**: a shell wound inside out is turned.
//! 2. **Reading** ([`slices`], [`structure`]): level cross-sections give
//!    its trunk, sleeves and legs, the underarm and crotch heights, and
//!    whether it is hollow.
//! 3. **One wall** ([`shell`]): the inner wall and the floors of the pits
//!    at its openings are taken off (a hollow garment keeps a turned edge).
//! 4. **Pose**: the body's arms and legs are turned to reach the garment's
//!    cuffs and hems.
//! 5. **Size and place**, by the garment's [`GarmentKind`]: a top hangs
//!    from the shoulders, sized by its sleeves reaching the wrists or by
//!    its chest; trousers sit at the crotch, sized by their legs reaching
//!    the ankles or by the seat; a skirt sits at the waist; a one-piece
//!    hangs from the shoulders, sized so its hems reach the ankles (or its
//!    crotch the body's).
//! 6. **Rest pose**: each sleeve and leg is turned back about its joint,
//!    each corner by its share of the limb; then the garment is carried
//!    from the body it was cut for (`cut_for`, say the feminine frame) to
//!    the default body.
//! 7. **Conform** ([`conform`]): drawn in round the body by how the kind
//!    of garment is worn ([`Conform`]), its folds kept.
//! 8. **Budget**: thinned to a number of triangles.
//! 9. **Tidy** ([`conform::tidy`]): scraps, teeth along the cut edges and
//!    the edges' zigzag.
//!
//! Nothing here is exact: a garment's size is a guess from its shape, and
//! [`IntakeOptions`] lets a person correct it (the kind, the ease, a scale
//! and a nudge). The report says which rule set each number.

mod centre;
mod conform;
mod shell;
mod slices;
mod structure;
mod weld;

#[cfg(test)]
mod tests;

use std::f32::consts::TAU;

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

pub use self::conform::{Conform, Openings, Tidied};
use self::slices::Welded;
use self::structure::{Ring, Structure};
pub use self::weld::Welding;
use super::bind::{bind, place};
use super::pose::{FinishPose, Posing, corrective_weights, skin, skin_matrices};
use super::surface::Surface;
use super::transfer::Welds;
use super::{BodyMesh, FitError, GarmentMesh, GarmentPiece};

/// How many level planes a mesh is read by.
const PLANES: usize = 240;
/// How far under the body's crotch a garment's crotch hangs (metres).
const CROTCH_EASE: f32 = 0.015;
/// How much longer round than the body a waistband is.
const WAIST_EASE: f32 = 0.03;
/// How much longer round than the body's seat short trousers are; and
/// the least (cloth that stretches) and most that long ones sized by
/// their legs may be.
const SEAT_EASE: f32 = 0.06;
const SEAT_LEAST: f32 = -0.08;
const SEAT_MOST: f32 = 0.6;
/// How far past the ankle a long leg's hem reaches (metres).
const HEM_PAST: f32 = 0.03;
/// The clearance a garment resting on the shoulders keeps (metres).
const REST_CLEARANCE: f32 = 0.002;
/// Skin a garment rests on faces up at least this much (its normal's
/// height, one unit long); and the share (per cent) of the cloth over
/// such skin that stays clear of it.
const TOP_FACING: f32 = 0.6;
const RESTS: usize = 90;
/// A sleeve or leg at least this many times longer than thick shows which
/// way it points; a sleeve this slender reaches the wrist.
const POINTS: f32 = 1.5;
const LONG_SLEEVE: f32 = 2.6;
/// A long-sleeved top sized by its sleeves is still at least this much,
/// and at most this much, longer round the chest than the body.
const CHEST_LEAST: f32 = 0.02;
const CHEST_MOST: f32 = 0.7;
/// How many rounds a limb's share of each corner settles over, when the
/// garment is taken back to the rest pose.
const UNBEND_ROUNDS: usize = 400;
/// Turning a limb back to rest, a triangle whose edge grows by more than
/// this many times (and half a centimetre) joined the limb to something
/// it only touched.
const BRIDGE: f32 = 2.5;
/// Thinning a garment to its budget never moves its surface by more than
/// this share of its size.
const THIN_ERROR: f32 = 0.02;
/// How many rounds the moves from the body a garment was cut for to the
/// default one are smoothed over it.
const UNFRAME_ROUNDS: usize = 16;
/// The clearance a solid garment's surface keeps from the skin in place
/// of a thickness it doesn't have (metres).
const SOLID_CLOTH: f32 = 0.003;

/// How a garment is worn, which decides how it is sized and placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GarmentKind {
    /// Hangs from the shoulders: shirts, sweaters, jackets, coats and
    /// dresses.
    Top,
    /// Two legs from a waistband: trousers, shorts, leggings.
    Trousers,
    /// One tube from a waistband.
    Skirt,
    /// Shoulders and legs in one: jumpsuits, overalls, suits.
    OnePiece,
}

/// How loosely a top is cut, when nothing better sizes it (its chest's
/// length round against the body's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    Snug,
    #[default]
    Regular,
    Loose,
    Oversized,
}

impl Ease {
    /// How much longer round the chest than the body.
    pub fn share(self) -> f32 {
        match self {
            Ease::Snug => 0.04,
            Ease::Regular => 0.10,
            Ease::Loose => 0.22,
            Ease::Oversized => 0.38,
        }
    }
}

/// What [`prepare`] is told; everything has a default.
#[derive(Clone, Debug, Default)]
pub struct IntakeOptions {
    /// How it is worn (`None`: guessed from its shape).
    pub kind: Option<GarmentKind>,
    pub ease: Ease,
    /// Its size, as metres per unit of its file (`None`: worked out).
    pub scale: Option<f32>,
    /// A move after placing (metres).
    pub nudge: Vec3,
    /// The body it was cut for, as weights of the body's shapes (the
    /// feminine frame: `[("Frame_Feminine", 1.0)]`). It is sized and
    /// placed on that body, then carried to the default one.
    pub cut_for: Vec<(String, f32)>,
    /// What the runtime does to a pose before skinning
    /// ([`FinishPose`]).
    pub finish_pose: Option<FinishPose>,
    /// Keeps both walls of a shell.
    pub keep_inner_wall: bool,
    /// How closely it follows the body (`None`: [`Conform::Moderate`]).
    pub conform: Option<Conform>,
    /// How many triangles it is thinned to (`None`: as many as it has).
    /// Generated garments have five times a game's budget, spread evenly
    /// rather than where the shape needs them.
    pub triangles: Option<usize>,
}

/// What [`prepare`] found and did.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IntakeReport {
    pub kind: Option<GarmentKind>,
    /// Whether the kind was guessed.
    pub guessed: bool,
    /// Whether the shell was wound inside out.
    pub flipped: bool,
    /// The cloth's thickness (metres, after sizing; 0 for one layer).
    pub thickness: f32,
    pub triangles_before: usize,
    pub triangles_after: usize,
    /// Metres per unit of the file, and the move after it (metres).
    pub scale: f32,
    pub offset: [f32; 3],
    /// Which rule sized it.
    pub sized_by: String,
    /// How much longer round the chest (a top) or the waist than the
    /// body it came out (a share; 0 when not measured).
    pub ease: f32,
    /// How far each arm and leg of the body was turned to lie along the
    /// garment's (degrees; right then left).
    pub arms_turned: [f32; 2],
    pub legs_turned: [f32; 2],
    /// Whether its sleeves reach the wrists.
    pub long_sleeves: bool,
    /// The farthest a corner was drawn in round the body (metres;
    /// [`Conform`]).
    #[serde(default)]
    pub conformed: f32,
    /// The farthest a corner of a sleeve or a leg was moved to bring it
    /// round its limb (metres).
    #[serde(default)]
    pub centred: f32,
    /// What was cleaned off its cut edges.
    #[serde(default)]
    pub tidied: Tidied,
    /// For a garment without legs: the body's crotch height, under which
    /// its cloth is a skirt's (kept as it hangs by the conforming, and
    /// hung from the hips by the fit's weights).
    #[serde(default)]
    pub skirt_from: Option<f32>,
    /// Its corners before and after they were merged
    /// ([`Welding`](weld::Welding)).
    #[serde(default)]
    pub welded: weld::Welding,
    /// What closing the cuts did, where a hanging sleeve's joins to the
    /// side were taken out ([`Mended`](super::Mended)).
    #[serde(default)]
    pub cuts_closed: super::Mended,
    /// The holes filled at the end ([`Mended`](super::Mended)).
    #[serde(default)]
    pub mended: super::Mended,
    /// How open the cloth was after each step (its open loops and
    /// edges): a step that adds loops tore it.
    #[serde(default)]
    pub openings: Vec<(String, conform::Openings)>,
    /// Anything a person should look at.
    pub notes: Vec<String>,
}

/// A garment ready to fit, and what was done to it.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub garment: GarmentMesh,
    pub report: IntakeReport,
}

/// Makes `garment` ready to fit on `body`: see the module documentation.
pub fn prepare(
    garment: &GarmentMesh,
    body: &BodyMesh,
    options: &IntakeOptions,
) -> Result<Prepared, FitError> {
    let mut garment = garment.clone();
    let mut report = IntakeReport::default();
    let triangles = garment.triangles();
    if triangles.is_empty() {
        return Err(FitError::Empty);
    }
    report.triangles_before = triangles.len();
    report
        .openings
        .push(("as it came".to_owned(), conform::openings(&garment)));

    // 1. Winding.
    let mut mesh = Welded::new(&garment.positions(), &triangles);
    if mesh.signed_volume() < 0.0 {
        flip(&mut garment);
        mesh = Welded::new(&garment.positions(), &garment.triangles());
        report.flipped = true;
    }

    // 2. Reading.
    let cuts = slices::slices(&mesh, PLANES);
    let made = structure::structure(&cuts, &mesh.points);
    if made.trunk.is_empty() {
        return Err(FitError::Model(
            "the garment has no trunk to place it by (no closed loop round its middle)".to_owned(),
        ));
    }
    // Cloth with a hollow for the body all the way up its trunk has a
    // thickness; a solid block with a pit at each opening has none that
    // can be read.
    let nested = shell::thickness(&cuts).unwrap_or(0.0);
    let hollow_rings = made.trunk.iter().filter(|r| r.inside.is_some()).count();
    let hollow = hollow_rings * 10 >= made.trunk.len() * 6;
    let thick = if hollow { nested } else { 0.0 };

    // 3. One wall: the inner wall and the floors of the pits come off.
    if !options.keep_inner_wall {
        let normals = point_normals(&mesh);
        // Two walls at one place still have sides.
        let band = nested.clamp(0.004 * made.height(), 0.012 * made.height());
        let inner = shell::inner_wall(&mesh, &cuts, &normals, &made, band);
        // A solid's pits are no turned edges: nothing of them is kept.
        let rim = if hollow { 1 } else { 0 };
        let kept = shell::outer_triangles(&mesh, &inner, rim);
        // Less than a third left is a shell read wrongly: keep it whole.
        if kept.len() * 3 >= mesh.triangles.len() {
            let source: Vec<u32> = kept.iter().map(|k| mesh.source[*k as usize]).collect();
            garment = with_triangles(&garment, &source);
        } else {
            report.notes.push(
                "its walls could not be told apart: both are kept, so it may show through itself"
                    .to_owned(),
            );
        }
    }
    report.triangles_after = garment.triangles().len();
    report
        .openings
        .push(("one wall".to_owned(), conform::openings(&garment)));

    let (kind, guessed) = match options.kind {
        Some(kind) => (kind, false),
        None => (guess(&made), true),
    };
    report.kind = Some(kind);
    report.guessed = guessed;

    // The body as the garment was cut for it, at rest: what it is
    // measured by (its arms stand clear of it there, so its trunk reads
    // cleanly whatever pose the garment is in).
    let rig = &body.rig;
    let (shaped, _) = body.shaped(&options.cut_for);
    let rest_welded = Welded::new(&shaped, &body.triangles);
    let body_made =
        structure::structure(&slices::slices(&rest_welded, PLANES), &rest_welded.points);
    if body_made.trunk.is_empty() {
        return Err(FitError::Model(
            "the body has no trunk to place a garment by".to_owned(),
        ));
    }
    let rest_world = rig.world(&rig.rest_pose());
    let joint = |name: &str| {
        rig.bone(name)
            .map(|bone| rest_world[bone].w_axis.truncate())
    };
    let wrists = [joint("RightHand"), joint("LeftHand")];
    let ankles = [joint("RightFoot"), joint("LeftFoot")];
    // The body's own limbs are its bones': from the shoulder to the wrist,
    // from the hip to the ankle.
    let mut body_made = body_made;
    let bone_line = |from: &str, to: Option<Vec3>| {
        Some(structure::Limb {
            start: joint(from)?,
            end: to?,
            width: 0.0,
        })
    };
    body_made.sleeves = [
        bone_line("RightUpperArm", wrists[0]),
        bone_line("LeftUpperArm", wrists[1]),
    ];
    body_made.legs = [
        bone_line("RightUpperLeg", ankles[0]),
        bone_line("LeftUpperLeg", ankles[1]),
    ];

    // 4. Pose: the body's limbs along the garment's. `aims` are where each
    // limb's end goes once the garment is placed (the cuffs and hems);
    // before that only the limbs' directions are known. Gives the posed
    // body and each limb's turn: sleeves right and left, then legs.
    let with_legs = matches!(kind, GarmentKind::Trousers | GarmentKind::OnePiece);
    let pose = |aims: Option<(f32, Vec3)>, report: &mut IntakeReport| {
        let mut posing = Posing::new(rig, options.finish_pose);
        let mut turns = [None; 4];
        for (side, name) in ["Right", "Left"].into_iter().enumerate() {
            let bones = |a: &str, b: &str| {
                Some((
                    rig.bone(&format!("{name}{a}"))?,
                    rig.bone(&format!("{name}{b}"))?,
                ))
            };
            let mut aim =
                |limb: &Option<structure::Limb>, upper: &str, end: &str| -> Option<Turn> {
                    let (limb, (upper, end)) = (limb.as_ref()?, bones(upper, end)?);
                    if limb.slenderness() < POINTS {
                        return None;
                    }
                    let about = posing.head(upper);
                    let was = posing.head(end) - about;
                    let toward = match aims {
                        Some((scale, offset)) => limb.end * scale + offset - about,
                        None => limb.axis(),
                    };
                    let degrees = posing.aim(upper, end, toward);
                    let by = Quat::from_rotation_arc(
                        was.normalize_or_zero(),
                        (posing.head(end) - about).normalize_or_zero(),
                    );
                    Some(Turn { about, by, degrees })
                };
            turns[side] = aim(&made.sleeves[side], "UpperArm", "Hand");
            if with_legs {
                turns[2 + side] = aim(&made.legs[side], "UpperLeg", "Foot");
            }
        }
        let degrees = |turn: &Option<Turn>| turn.map_or(0.0, |t| t.degrees);
        report.arms_turned = [degrees(&turns[0]), degrees(&turns[1])];
        report.legs_turned = [degrees(&turns[2]), degrees(&turns[3])];
        let bones = posing.finished();
        let mut shape_weights = options.cut_for.clone();
        if turns.iter().flatten().any(|t| t.degrees > 0.05) {
            shape_weights.extend(corrective_weights(rig, &bones));
        }
        let (shaped, shaped_normals) = body.shaped(&shape_weights);
        let matrices = skin_matrices(rig, &bones);
        let (posed, posed_normals, _) = skin(
            &matrices,
            &body.joints,
            &body.weights,
            &shaped,
            &shaped_normals,
        );
        (posed, posed_normals, turns)
    };

    // 5. Size and place: first with the limbs only pointing the garment's
    // way, then again at that size with them reaching its cuffs and hems.
    let (posed, posed_normals, _) = pose(None, &mut report);
    let floor = floor_of(body, &posed, &posed_normals);
    let fitting = Fitting {
        made: &made,
        body: &body_made,
        thick,
        garment: &garment,
        floor: &floor,
        wrists,
        ankles,
        ease: options.ease,
    };
    let mut placed = match kind {
        GarmentKind::Top => fitting.top(),
        GarmentKind::Trousers => fitting.trousers(),
        GarmentKind::Skirt => fitting.skirt(),
        GarmentKind::OnePiece => fitting.one_piece(),
    }?;
    if let Some(scale) = options.scale {
        placed.scale = scale;
        placed.sized_by = "given".to_owned();
    }
    placed = fitting.placed_at(kind, placed.scale, placed);
    let (posed, posed_normals, turns) = pose(Some((placed.scale, placed.offset)), &mut report);
    let floor = floor_of(body, &posed, &posed_normals);
    let fitting = Fitting {
        floor: &floor,
        ..fitting
    };
    placed = fitting.placed_at(kind, placed.scale, placed);
    placed.offset += options.nudge;
    garment.transform(placed.scale, placed.offset);
    report.scale = placed.scale;
    report.offset = placed.offset.to_array();
    report.sized_by = placed.sized_by;
    report.ease = placed.ease;
    report.long_sleeves = placed.long_sleeves;
    report.thickness = thick * placed.scale;
    report.notes.extend(placed.notes);

    // 6. Back to the rest pose: each sleeve and leg is turned back about
    // its joint, as far as each corner belongs to it (wholly on the limb,
    // not at all on the trunk, and eased between them over the shoulder or
    // the seat).
    let garment_triangles = garment.triangles();
    if turns.iter().flatten().any(|t| t.degrees > 0.05) {
        let mut positions = garment.positions();
        let mut normals = garment.normals();
        let welded = Welded::new(&positions, &garment_triangles);
        // The reading's points, found again by where they are.
        let place = |p: Vec3| {
            (p * placed.scale + placed.offset)
                .to_array()
                .map(f32::to_bits)
        };
        let at: std::collections::HashMap<[u32; 3], u32> = welded
            .points
            .iter()
            .enumerate()
            .map(|(k, p)| (p.to_array().map(f32::to_bits), k as u32))
            .collect();
        let found = |ids: &[u32]| -> Vec<u32> {
            ids.iter()
                .filter_map(|id| at.get(&place(mesh.points[*id as usize])).copied())
                .collect()
        };
        // A limb's points: the ones the reading found on its loops, and
        // every point round its middle line that faces away from it (where
        // a hanging sleeve touches the garment's side the two are cut as
        // one loop, but the side faces the sleeve's middle line and the
        // sleeve faces away from it).
        let welded_normals = point_normals(&welded);
        let limb_points = |read: &[u32], limb: Option<&structure::Limb>| -> Vec<u32> {
            let mut own = found(read);
            let Some(limb) = limb else {
                return own;
            };
            let start = limb.start * placed.scale + placed.offset;
            let end = limb.end * placed.scale + placed.offset;
            let (axis, length) = ((end - start).normalize_or_zero(), start.distance(end));
            let radius = 0.9 * limb.width * placed.scale;
            for (k, p) in welded.points.iter().enumerate() {
                let along = (*p - start).dot(axis);
                if along < 0.05 * length || along > length + radius {
                    continue;
                }
                let out = *p - (start + axis * along);
                if out.length() < radius && out.dot(welded_normals[k]) > 0.0 {
                    own.push(k as u32);
                }
            }
            own.sort_unstable();
            own.dedup();
            own
        };
        let limbs = [
            (
                limb_points(&made.sleeve_points[0], made.sleeves[0].as_ref()),
                made.sleeves[0].as_ref(),
            ),
            (
                limb_points(&made.sleeve_points[1], made.sleeves[1].as_ref()),
                made.sleeves[1].as_ref(),
            ),
            (
                limb_points(&made.leg_points[0], made.legs[0].as_ref()),
                made.legs[0].as_ref(),
            ),
            (
                limb_points(&made.leg_points[1], made.legs[1].as_ref()),
                made.legs[1].as_ref(),
            ),
        ];
        let on_limb: std::collections::HashSet<u32> = limbs
            .iter()
            .flat_map(|(points, _)| points.iter().copied())
            .collect();
        let trunk: Vec<u32> = found(&made.trunk_points)
            .into_iter()
            .filter(|k| !on_limb.contains(k))
            .collect();
        let axis_x = made.axis.x * placed.scale + placed.offset.x;
        let neighbours = welded.neighbours();
        // Each point's share of each limb that is turned (for closing the
        // cuts below).
        let mut limb_shares = vec![[0.0f32; 4]; welded.points.len()];
        for (which, turn) in turns.iter().enumerate() {
            let (Some(turn), (own, Some(limb))) = (turn, &limbs[which]) else {
                continue;
            };
            // Held at 1 on the limb and at 0 on the trunk, on the other
            // limbs and (for a sleeve) over the middle half of the
            // shoulders; in between each corner settles at the mean of its
            // neighbours.
            let mut share = vec![0.0f32; welded.points.len()];
            let mut held = vec![false; welded.points.len()];
            for &k in own {
                share[k as usize] = 1.0;
                held[k as usize] = true;
            }
            let others = limbs
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != which)
                .flat_map(|(_, (points, _))| points.iter());
            for &k in trunk.iter().chain(others) {
                share[k as usize] = 0.0;
                held[k as usize] = true;
            }
            let start_x = limb.start.x * placed.scale + placed.offset.x;
            let side = (start_x - axis_x).signum();
            for (k, p) in welded.points.iter().enumerate() {
                let out_by = (p.x - axis_x) * side;
                let middle = which < 2 && out_by < 0.5 * (start_x - axis_x).abs();
                if !held[k] && (middle || out_by < 0.0) {
                    held[k] = true;
                }
            }
            for _ in 0..UNBEND_ROUNDS {
                let before = share.clone();
                for (k, list) in neighbours.iter().enumerate() {
                    if !held[k] && !list.is_empty() {
                        share[k] = list.iter().map(|n| before[*n as usize]).sum::<f32>()
                            / list.len() as f32;
                    }
                }
            }
            for (own, share) in limb_shares.iter_mut().zip(&share) {
                own[which] = *share;
            }
            let back = turn.by.inverse();
            for (corner, &point) in welded.of_corner.iter().enumerate() {
                let by = Quat::IDENTITY.slerp(back, share[point as usize]);
                positions[corner] = turn.about + by * (positions[corner] - turn.about);
                normals[corner] = (by * normals[corner]).normalize_or_zero();
            }
        }
        // Where a hanging sleeve touched the garment's side the mesh is
        // often joined; turned back, those joins are stretched into
        // strings, and come out.
        let before = garment.positions();
        let stretched = |t: &[u32; 3]| {
            let [a, b, c] = t.map(|v| v as usize);
            [(a, b), (b, c), (c, a)].into_iter().any(|(i, j)| {
                positions[i].distance(positions[j]) > BRIDGE * before[i].distance(before[j]) + 0.005
            })
        };
        let kept: Vec<u32> = garment_triangles
            .iter()
            .enumerate()
            .filter(|(_, t)| !stretched(t))
            .map(|(k, _)| k as u32)
            .collect();
        garment.set_positions(&positions);
        garment.set_normals(&normals);
        if kept.len() < garment_triangles.len() {
            // The cut's rim: the corners of what came out, each with its
            // shares of the limbs, found again by where they are.
            let mut is_kept = vec![false; garment_triangles.len()];
            for k in &kept {
                is_kept[*k as usize] = true;
            }
            let bits = |p: Vec3| p.to_array().map(f32::to_bits);
            let mut rim: std::collections::HashMap<[u32; 3], [f32; 4]> =
                std::collections::HashMap::new();
            for (t, kept) in garment_triangles.iter().zip(&is_kept) {
                if *kept {
                    continue;
                }
                for &corner in t {
                    let corner = corner as usize;
                    rim.insert(
                        bits(positions[corner]),
                        limb_shares[welded.of_corner[corner] as usize],
                    );
                }
            }
            garment = with_triangles(&garment, &kept);
            // Closed again: the sleeve's underside to itself, the side
            // to itself.
            let now = garment.positions();
            report.cuts_closed = super::mend::close_cuts(&mut garment, |corner| {
                rim.get(&bits(now[corner as usize])).copied()
            });
        }
    }
    report
        .openings
        .push(("back to rest".to_owned(), conform::openings(&garment)));
    let garment_triangles = garment.triangles();
    // And from the body it was cut for to the default one: each corner
    // moves with the skin nearest it, the moves smoothed over the cloth.
    if !options.cut_for.is_empty() {
        let (cut, cut_normals) = body.shaped(&options.cut_for);
        let surface = Surface::new(cut, cut_normals, body.triangles.clone());
        let positions = garment.positions();
        let normals = garment.normals();
        let binding = bind(&surface, &positions, &[]);
        let moved = place(&binding, &body.positions, &body.normals, &body.triangles);
        let welds = Welds::new(&positions, &garment_triangles);
        let moves: Vec<Vec3> = moved.iter().zip(&positions).map(|(m, p)| *m - *p).collect();
        let loose: Vec<f32> = binding
            .vertices
            .iter()
            .map(|b| 0.5 + 0.5 * (Vec3::from(b.local).length() / 0.05).min(1.0))
            .collect();
        let moves = welds.smooth(&moves, &loose, UNFRAME_ROUNDS);
        let moved: Vec<Vec3> = positions.iter().zip(&moves).map(|(p, m)| *p + *m).collect();
        let was = welds.normals(&positions, &garment_triangles);
        let is = welds.normals(&moved, &garment_triangles);
        let turned: Vec<Vec3> = normals
            .iter()
            .zip(was.iter().zip(&is))
            .map(|(n, (was, is))| {
                if *was == Vec3::ZERO || *is == Vec3::ZERO {
                    *n
                } else {
                    (Quat::from_rotation_arc(*was, *is) * *n).normalize_or_zero()
                }
            })
            .collect();
        garment.set_positions(&moved);
        garment.set_normals(&turned);
    }
    // 7. Each sleeve and leg centred on its limb, then the cloth drawn in
    // round the body. Cloth under the crotch that no leg fills (a
    // skirt's, a dress's, a coat's) keeps its shape.
    report.centred = centre::centre_limbs(&mut garment, body, with_legs);
    let skirt_from = (!with_legs).then(|| body_made.trunk_low());
    report.skirt_from = skirt_from;
    report.conformed = conform::conform(
        &mut garment,
        body,
        options.conform.unwrap_or_default(),
        skirt_from,
    );
    // 8. Thinned to its budget, each piece by its share.
    if let Some(budget) = options.triangles {
        let total = garment.triangles().len();
        if total > budget {
            for piece in &mut garment.pieces {
                let own = piece.indices.len() / 3;
                let target = (own as f64 * budget as f64 / total as f64).round() as usize;
                let array = |list: &[Vec3]| list.iter().map(|v| v.to_array()).collect::<Vec<_>>();
                let indices = axcenity_assets::simplify_triangles(
                    &array(&piece.positions),
                    &array(&piece.normals),
                    &piece.uvs,
                    &piece.indices,
                    target.max(64),
                    THIN_ERROR,
                );
                piece.indices = indices;
            }
            let all: Vec<u32> = (0..garment.triangles().len() as u32).collect();
            garment = with_triangles(&garment, &all);
        }
    }
    report
        .openings
        .push(("thinned".to_owned(), conform::openings(&garment)));
    // 9. Its cut edges cleaned.
    report.tidied = conform::tidy(&mut garment);
    report
        .openings
        .push(("tidied".to_owned(), conform::openings(&garment)));
    // 10. Its holes filled: every open loop the body doesn't come through.
    report.mended = super::mend::mend(&mut garment, body, |_| true);
    report
        .openings
        .push(("mended".to_owned(), conform::openings(&garment)));
    report.triangles_after = garment.triangles().len();
    // 11. Its normals made smooth and its corners merged.
    report.welded = weld::weld(&mut garment);
    report.triangles_after = garment.triangles().len();
    Ok(Prepared { garment, report })
}

/// A limb's turn from the rest pose: about a point, by a rotation.
#[derive(Clone, Copy, Debug)]
struct Turn {
    about: Vec3,
    by: Quat,
    degrees: f32,
}

/// A garment's kind from its shape: two legs make trousers, or a
/// one-piece when it goes on above the trunk; without legs it is a top
/// when it goes on above the trunk (shoulders, straps, a collar) and a
/// skirt when the trunk is all there is.
fn guess(made: &Structure) -> GarmentKind {
    match (made.crotch().is_some(), made.has_upper()) {
        (true, true) => GarmentKind::OnePiece,
        (true, false) => GarmentKind::Trousers,
        (false, true) => GarmentKind::Top,
        (false, false) => GarmentKind::Skirt,
    }
}

/// A size and place: each corner `p` becomes `p * scale + offset`.
#[derive(Clone, Debug)]
struct Placed {
    scale: f32,
    offset: Vec3,
    sized_by: String,
    ease: f32,
    long_sleeves: bool,
    notes: Vec<String>,
}

/// What the sizing rules read: the garment's and the posed body's
/// structures, the cloth's thickness (the garment's units), the garment
/// itself and what it rests on.
struct Fitting<'a> {
    made: &'a Structure,
    body: &'a Structure,
    thick: f32,
    garment: &'a GarmentMesh,
    floor: &'a Surface,
    wrists: [Option<Vec3>; 2],
    ankles: [Option<Vec3>; 2],
    ease: Ease,
}

/// The chest: the ring a fifth of the trunk's width under the underarm
/// (right under it the ring takes in the folds of the armpits, on a body
/// and on a garment).
fn chest(made: &Structure) -> Option<Ring> {
    let top = made.trunk.last()?;
    made.trunk_at(top.y - 0.2 * top.width())
}

impl Fitting<'_> {
    /// A ring's length round on the inside of the cloth: round its hollow
    /// where it has one, else the tape's less the cloth's thickness.
    fn inside(&self, ring: &Ring) -> f32 {
        ring.inside
            .unwrap_or(ring.length - TAU * self.thick)
            .max(ring.length * 0.5)
    }

    /// How far the garment's outside is from the skin where it lies on
    /// it, at `scale` (metres).
    fn cloth(&self, scale: f32) -> f32 {
        if self.thick > 0.0 {
            self.thick * scale
        } else {
            SOLID_CLOTH
        }
    }

    fn missing(what: &str) -> FitError {
        FitError::Model(format!("the garment has no {what} to size it by"))
    }

    /// The move that puts the garment's middle on the body's across, and
    /// `ring` of it over `over` of the body front to back.
    fn across(&self, scale: f32, ring: &Ring, over: &Ring) -> Vec3 {
        Vec3::new(
            self.body.axis.x - scale * self.made.axis.x,
            0.0,
            over.centre.y - scale * ring.centre.y,
        )
    }

    /// How far the garment at `scale` and `offset` must rise to rest on
    /// the body: its upward-facing cloth above the underarm is held
    /// [`REST_CLEARANCE`] and the cloth's thickness over the skin that
    /// faces up under it (the shoulders, the tops of the arms by them),
    /// all but the [`RESTS`] share nearest it, which is what it rests on
    /// (the fit pushes that out). Skin above a corner doesn't count: the
    /// rim of an armhole lies under the arm. `None` when nothing of it is
    /// over the body.
    fn rise(&self, scale: f32, offset: Vec3) -> Option<f32> {
        let underarm = self.made.underarm();
        let clearance = REST_CLEARANCE + self.cloth(scale);
        let normals = self.garment.normals();
        let mut needs: Vec<f32> = self
            .garment
            .positions()
            .iter()
            .zip(&normals)
            .filter(|(p, n)| p.y > underarm && n.y > 0.3)
            .filter_map(|(p, _)| {
                let p = *p * scale + offset;
                // In the skin, or just over it: out along the way up.
                if let Some(hit) = self.floor.closest(p, 0.06)
                    && hit.normal.y > TOP_FACING
                    && hit.is_over(p)
                {
                    return Some((clearance - hit.signed(p)) / hit.normal.y);
                }
                let hit = self.floor.cast(p, -Vec3::Y, 0.6)?;
                (hit.normal.y > TOP_FACING).then_some(clearance - hit.distance)
            })
            .collect();
        if needs.len() < 8 {
            return None;
        }
        needs.sort_by(f32::total_cmp);
        Some(needs[(needs.len() - 1) * RESTS / 100])
    }

    /// A top at `scale`: its middle on the body's, its chest over the
    /// body's, resting on the shoulders (or, when nothing of it is over
    /// them, its underarm a little under the body's).
    fn hang(&self, scale: f32) -> Result<Vec3, FitError> {
        let ring = chest(self.made).ok_or_else(|| Self::missing("chest"))?;
        let over = chest(self.body).ok_or_else(|| Self::missing("body chest"))?;
        let mut offset = self.across(scale, &ring, &over);
        offset.y = self.body.underarm() - 0.02 - scale * self.made.underarm();
        // A few rounds: what is under each corner changes as it moves.
        for _ in 0..4 {
            match self.rise(scale, offset) {
                Some(rise) if rise.abs() > 1e-4 => offset.y += rise,
                _ => break,
            }
        }
        Ok(offset)
    }

    /// For each side whose limb of the garment is long ([`LONG_SLEEVE`]):
    /// how many metres a unit make it end at the body's joint there (the
    /// wrist, the ankle), `past` metres beyond it. Both limbs are measured
    /// from where they leave the trunk.
    fn reach(
        &self,
        limbs: &[Option<structure::Limb>; 2],
        on_body: &[Option<structure::Limb>; 2],
        joints: &[Option<Vec3>; 2],
        past: f32,
        legs: bool,
    ) -> Vec<f32> {
        let mut ratios = Vec::new();
        for side in 0..2 {
            let (Some(limb), Some(own), Some(joint)) = (&limbs[side], &on_body[side], joints[side])
            else {
                continue;
            };
            if limb.slenderness() < LONG_SLEEVE {
                continue;
            }
            // Each from where its middle line passes the height it leaves
            // the trunk at, to its end.
            let leaves = |limb: &structure::Limb, at: f32| {
                let axis = limb.axis();
                if axis.y.abs() < 0.2 {
                    return limb.start;
                }
                limb.start + axis * ((at - limb.start.y) / axis.y)
            };
            let (at, on) = if legs {
                (self.made.trunk_low(), self.body.trunk_low())
            } else {
                (self.made.underarm(), self.body.underarm())
            };
            let length = (joint - leaves(own, on)).dot(own.axis()) + past;
            let own_length = (limb.end - leaves(limb, at)).dot(limb.axis());
            if length > 0.0 && own_length > 0.0 {
                ratios.push(length / own_length);
            }
        }
        ratios
    }

    /// When both legs are long ([`LONG_SLEEVE`]): the height their hems
    /// reach on the body, a little past the ankles.
    fn long_legs(&self) -> Option<f32> {
        let long = |limb: &Option<structure::Limb>| {
            limb.as_ref()
                .is_some_and(|l| l.slenderness() >= LONG_SLEEVE)
        };
        match self.ankles {
            [Some(right), Some(left)] if self.made.legs.iter().all(long) => {
                Some((right.y + left.y) / 2.0 - HEM_PAST)
            }
            _ => None,
        }
    }

    /// The seat: the trunk's longest ring in its lower part (the garment's
    /// lower half, the body's lower third).
    fn seats(&self) -> Result<(Ring, Ring), FitError> {
        let (low, high) = (self.made.trunk_low(), self.made.underarm());
        let ring = self
            .made
            .widest(low, low + 0.5 * (high - low))
            .ok_or_else(|| Self::missing("seat"))?;
        let (low, high) = (self.body.trunk_low(), self.body.underarm());
        let over = self
            .body
            .widest(low, low + 0.33 * (high - low))
            .ok_or_else(|| Self::missing("body seat"))?;
        Ok((ring, over))
    }

    /// The move that seats trousers at `scale`: the crotch a little under
    /// the body's, the seat over the body's front to back.
    fn seated(&self, scale: f32) -> Result<Vec3, FitError> {
        let (ring, over) = self.seats()?;
        let mut offset = self.across(scale, &ring, &over);
        offset.y =
            self.body.trunk_low() - CROTCH_EASE - self.cloth(scale) - scale * self.made.trunk_low();
        Ok(offset)
    }

    fn top(&self) -> Result<Placed, FitError> {
        let ring = chest(self.made).ok_or_else(|| Self::missing("chest"))?;
        let over = chest(self.body).ok_or_else(|| Self::missing("body chest"))?;
        let round = self.inside(&ring);
        let by_chest = |ease: f32| over.length * (1.0 + ease) / round;
        let mut scale = by_chest(self.ease.share());
        let mut sized_by = format!("chest ({:?} ease)", self.ease);
        let mut long_sleeves = false;
        let mut notes = Vec::new();

        // Sleeves that reach the wrist size it without guessing its ease.
        let ratios = self.reach(
            &self.made.sleeves,
            &self.body.sleeves,
            &self.wrists,
            0.0,
            false,
        );
        if ratios.len() == 2 {
            long_sleeves = true;
            // The longer sleeve (the smaller size): what cuts a sleeve's
            // reading short makes it shorter, never longer.
            let by_sleeves = ratios[0].min(ratios[1]);
            let (least, most) = (by_chest(CHEST_LEAST), by_chest(CHEST_MOST));
            scale = by_sleeves.clamp(least, most);
            sized_by = if scale == by_sleeves {
                "sleeve length".to_owned()
            } else {
                notes.push(format!(
                    "its sleeves ask for {by_sleeves:.3} m a unit, which its chest doesn't allow"
                ));
                "chest (the sleeves' size was out of range)".to_owned()
            };
        }
        let offset = self.hang(scale)?;
        Ok(Placed {
            scale,
            offset,
            sized_by,
            ease: scale * round / over.length - 1.0,
            long_sleeves,
            notes,
        })
    }

    /// The garment's waistband: the trunk's shortest ring in its upper
    /// part.
    fn waistband(&self, share: f32) -> Result<Ring, FitError> {
        let (low, high) = (self.made.trunk_low(), self.made.underarm());
        let span = high - low;
        // Not its very top: an uneven edge is cut as part of a ring.
        self.made
            .narrowest(high - share * span, high - 0.05 * span)
            .ok_or_else(|| Self::missing("waistband"))
    }

    fn trousers(&self) -> Result<Placed, FitError> {
        let (ring, over) = self.seats()?;
        let round = self.inside(&ring);
        let by_seat = |ease: f32| over.length * (1.0 + ease) / round;
        let mut scale = by_seat(SEAT_EASE);
        let mut sized_by = "seat".to_owned();
        let mut notes = Vec::new();
        // Legs that reach the ankle size it without guessing its ease:
        // from the crotch straight down to its lowest point, against the
        // body's crotch to a little past the ankle.
        if let Some(ankle) = self.long_legs() {
            let by_legs = (self.body.trunk_low() - CROTCH_EASE - ankle)
                / (self.made.trunk_low() - self.made.bottom).max(1e-6);
            scale = by_legs.clamp(by_seat(SEAT_LEAST), by_seat(SEAT_MOST));
            sized_by = if scale == by_legs {
                "leg length".to_owned()
            } else {
                notes.push(format!(
                    "its legs ask for {by_legs:.3} m a unit, which its seat doesn't allow"
                ));
                "seat (the legs' size was out of range)".to_owned()
            };
        }
        Ok(Placed {
            scale,
            offset: self.seated(scale)?,
            sized_by,
            ease: scale * round / over.length - 1.0,
            long_sleeves: false,
            notes,
        })
    }

    /// The body's natural waist: its trunk's shortest ring round the
    /// middle of it.
    fn body_waist(&self) -> Result<Ring, FitError> {
        let (low, high) = (self.body.trunk_low(), self.body.underarm());
        let span = high - low;
        self.body
            .narrowest(low + 0.4 * span, low + 0.7 * span)
            .ok_or_else(|| Self::missing("body waist"))
    }

    fn skirt(&self) -> Result<Placed, FitError> {
        let band = self.waistband(0.3)?;
        let over = self.body_waist()?;
        let scale = over.length * (1.0 + WAIST_EASE) / self.inside(&band);
        let mut offset = self.across(scale, &band, &over);
        offset.y = over.y - scale * band.y;
        Ok(Placed {
            scale,
            offset,
            sized_by: "waistband, at the waist".to_owned(),
            ease: WAIST_EASE,
            long_sleeves: false,
            notes: Vec::new(),
        })
    }

    fn one_piece(&self) -> Result<Placed, FitError> {
        let crotch = self.made.trunk_low();
        let seat = self.body.trunk_low() - CROTCH_EASE;
        // Hanging from the shoulders, a bigger garment reaches lower: the
        // size is the one where its legs' ends meet the ankles when they
        // are long (a suit's jacket hides its crotch), else the one where
        // its crotch meets the body's.
        let hems = self.long_legs().map(|ankle| (self.made.bottom, ankle));
        let gap = |scale: f32| -> Result<f32, FitError> {
            let offset = self.hang(scale)?;
            Ok(match hems {
                Some((hem, ankle)) => scale * hem + offset.y - ankle,
                None => scale * crotch + offset.y - (seat - self.cloth(scale)),
            })
        };
        let guess = (self.body.underarm() - self.body.trunk_low())
            / (self.made.underarm() - crotch).max(1e-6);
        // The gap falls as the size grows, but not evenly (what the
        // garment rests on changes): the sizes are tried in steps, and the
        // first step the gap changes sign over is halved down.
        let mut notes = Vec::new();
        let steps: Vec<f32> = (0..=22).map(|k| guess * (0.5 + 0.05 * k as f32)).collect();
        let mut gaps = Vec::with_capacity(steps.len());
        for scale in &steps {
            gaps.push(gap(*scale)?);
        }
        let crossing = (1..steps.len()).find(|&k| gaps[k - 1] > 0.0 && gaps[k] <= 0.0);
        let scale = if let Some(k) = crossing {
            let (mut low, mut high) = (steps[k - 1], steps[k]);
            for _ in 0..16 {
                let middle = (low + high) / 2.0;
                if gap(middle)? > 0.0 {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            (low + high) / 2.0
        } else {
            notes.push(
                "its shoulders and legs could not both be met: the nearest size is taken"
                    .to_owned(),
            );
            let nearest = (0..steps.len())
                .min_by(|a, b| gaps[*a].abs().total_cmp(&gaps[*b].abs()))
                .unwrap_or(0);
            steps[nearest]
        };
        let offset = self.hang(scale)?;
        let ease = match (chest(self.made), chest(self.body)) {
            (Some(ring), Some(over)) => scale * self.inside(&ring) / over.length - 1.0,
            _ => 0.0,
        };
        Ok(Placed {
            scale,
            offset,
            sized_by: if hems.is_some() {
                "shoulders to ankles".to_owned()
            } else {
                "shoulders to crotch".to_owned()
            },
            ease,
            long_sleeves: false,
            notes,
        })
    }

    /// `kind`'s place at a given `scale` (what a rule gave is kept where
    /// the place can't be worked out again).
    fn placed_at(&self, kind: GarmentKind, scale: f32, was: Placed) -> Placed {
        let offset = match kind {
            GarmentKind::Top | GarmentKind::OnePiece => self.hang(scale).ok(),
            GarmentKind::Trousers => self.seated(scale).ok(),
            GarmentKind::Skirt => self.waistband(0.3).ok().and_then(|band| {
                let over = self.body_waist().ok()?;
                let mut offset = self.across(scale, &band, &over);
                offset.y = over.y - scale * band.y;
                Some(offset)
            }),
        };
        Placed {
            scale,
            offset: offset.unwrap_or(was.offset),
            ..was
        }
    }
}

/// What a garment rests on: the posed body without its head (a collar
/// goes round the neck, under the head).
fn floor_of(body: &BodyMesh, posed: &[Vec3], normals: &[Vec3]) -> Surface {
    let head: Vec<bool> = body
        .rig
        .joints
        .iter()
        .map(|joint| {
            let name = body.rig.bones[joint.bone].name.as_str();
            matches!(name, "Head" | "Jaw" | "LeftEye" | "RightEye")
        })
        .collect();
    let on_head = |corner: u32| -> f32 {
        let c = corner as usize;
        body.joints[c]
            .iter()
            .zip(body.weights[c])
            .filter(|(joint, _)| head.get(**joint as usize).copied().unwrap_or(false))
            .map(|(_, weight)| weight)
            .sum()
    };
    let triangles = body
        .triangles
        .iter()
        .copied()
        .filter(|t| t.iter().map(|v| on_head(*v)).sum::<f32>() < 1.5)
        .collect();
    Surface::new(posed.to_vec(), normals.to_vec(), triangles)
}

/// Turns a garment wound inside out: every triangle the other way round,
/// every normal reversed.
fn flip(garment: &mut GarmentMesh) {
    for piece in &mut garment.pieces {
        for t in piece.indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
        for n in &mut piece.normals {
            *n = -*n;
        }
    }
}

/// Smooth normals of a welded mesh's points (bigger triangles count for
/// more).
fn point_normals(mesh: &Welded) -> Vec<Vec3> {
    let mut sums = vec![Vec3::ZERO; mesh.points.len()];
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|v| mesh.points[v as usize]);
        let face = (b - a).cross(c - a);
        for v in t {
            sums[*v as usize] += face;
        }
    }
    sums.into_iter().map(Vec3::normalize_or_zero).collect()
}

/// `garment` with only the triangles `kept` (places in
/// [`GarmentMesh::triangles`]) and the corners they use.
fn with_triangles(garment: &GarmentMesh, kept: &[u32]) -> GarmentMesh {
    let mut keep = vec![false; garment.triangles().len()];
    for k in kept {
        keep[*k as usize] = true;
    }
    let mut first = 0;
    let mut pieces = Vec::new();
    for piece in &garment.pieces {
        const UNUSED: u32 = u32::MAX;
        let count = piece.indices.len() / 3;
        let mut remap = vec![UNUSED; piece.positions.len()];
        let mut used: Vec<usize> = Vec::new();
        let mut indices = Vec::new();
        for (t, corners) in piece.indices.chunks_exact(3).enumerate() {
            if !keep[first + t] {
                continue;
            }
            for &corner in corners {
                let slot = &mut remap[corner as usize];
                if *slot == UNUSED {
                    *slot = used.len() as u32;
                    used.push(corner as usize);
                }
                indices.push(*slot);
            }
        }
        first += count;
        if indices.is_empty() {
            continue;
        }
        let pick = |list: &[Vec3]| -> Vec<Vec3> {
            if list.len() == piece.positions.len() {
                used.iter().map(|k| list[*k]).collect()
            } else {
                Vec::new()
            }
        };
        pieces.push(GarmentPiece {
            material: piece.material.clone(),
            colour: piece.colour,
            roughness: piece.roughness,
            positions: pick(&piece.positions),
            normals: pick(&piece.normals),
            uvs: if piece.uvs.len() == piece.positions.len() {
                used.iter().map(|k| piece.uvs[*k]).collect()
            } else {
                Vec::new()
            },
            indices,
        });
    }
    GarmentMesh { pieces }
}

/// A garment's cross-sections and structure as text, for looking into a
/// reading that went wrong.
#[doc(hidden)]
pub fn describe_reading(garment: &GarmentMesh, every: usize) -> String {
    use std::fmt::Write;
    let mesh = Welded::new(&garment.positions(), &garment.triangles());
    let cuts = slices::slices(&mesh, PLANES);
    let made = structure::structure(&cuts, &mesh.points);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "points {} triangles {} volume {:.4} thickness {:?}",
        mesh.points.len(),
        mesh.triangles.len(),
        mesh.signed_volume() / 6.0,
        shell::thickness(&cuts)
    );
    if let Some(thick) = shell::thickness(&cuts) {
        let normals = point_normals(&mesh);
        let band = thick.max(0.004 * made.height());
        let inner = shell::inner_wall(&mesh, &cuts, &normals, &made, band);
        let kept = shell::outer_triangles(&mesh, &inner, 1);
        let _ = writeln!(
            out,
            "inner points {} of {}; triangles kept {} of {}",
            inner.iter().filter(|i| **i).count(),
            inner.len(),
            kept.len(),
            mesh.triangles.len()
        );
    }
    for slice in cuts.iter().step_by(every.max(1)) {
        let _ = write!(out, "y {:+.3}:", slice.y);
        for item in &slice.loops {
            let _ = write!(
                out,
                " [{}{} len {:.3} area {:.4} x {:+.2}..{:+.2} z {:+.2}..{:+.2}]",
                if item.closed { "d" } else { "open" },
                item.depth,
                item.length,
                item.area,
                item.min.x,
                item.max.x,
                item.min.y,
                item.max.y
            );
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "bottom {:.3} top {:.3} trunk {:.3}..{:.3} ({} rings) upper {} crotch {:?} guess {:?}",
        made.bottom,
        made.top,
        made.trunk_low(),
        made.underarm(),
        made.trunk.len(),
        made.has_upper(),
        made.crotch(),
        guess(&made)
    );
    for (name, limbs) in [("sleeve", &made.sleeves), ("leg", &made.legs)] {
        for (side, limb) in limbs.iter().enumerate() {
            if let Some(limb) = limb {
                let _ = writeln!(
                    out,
                    "{name} {side}: start {:.3?} end {:.3?} length {:.3} width {:.3}",
                    limb.start,
                    limb.end,
                    limb.length(),
                    limb.width
                );
            }
        }
    }
    out
}
