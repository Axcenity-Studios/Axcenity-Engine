//! A garment's pose correctives, baked from cloth simulation (wave 5,
//! stage 3; `docs/development/character-studio/refinement/wave5/
//! pose-correctives.md`).
//!
//! The fit binds a garment to the skin and skins it with the body's
//! weights, so in a pose it moves as the skin under it does. Loose cloth
//! doesn't: between a hanging arm and the side the bridge of cloth is
//! squeezed and folds over itself, and the skin comes through where it
//! does. A pose corrective puts back what linear skinning loses there, as
//! the body's own swing correctives do for its skin: a shape of the
//! garment, driven by a joint's turn by the same rule
//! (`anim::correctives`; architecture.md section 7).
//!
//! The bake, in three steps (the intake's `correctives.py` runs them):
//!
//! 1. [`write_bake_input`]: the body and the fitted garment on the base
//!    body, and [`cloth_runs`]: for each run, its frames from rest into
//!    the key poses ([`cloth_rules`]: shoulder lowered, swung forward and
//!    back, raised to level and above; elbow bent; hip forward and back;
//!    knee bent), the skin posed in each frame (with the body's own
//!    correctives) and the garment as skinning poses it today.
//! 2. Blender's cloth simulation, with the body as collider, pulls the
//!    garment from that skinned result towards where cloth would hang
//!    (`correctives.py`), and writes it at each run's reads.
//! 3. [`solve`]: at each key pose, the difference between the simulated
//!    cloth and the skinned one, taken back to rest through each corner's
//!    own skinning, less what the simulation moved at rest, is the pose's
//!    shape ([`PoseShapes`]); a shape made on top of another (the swing
//!    on the hanging arm) has that one's share taken off first. Half way
//!    to each key pose the blended shapes are checked against the
//!    simulation (smoothness between poses).
//!
//! [`add_pose_shapes`] then writes the shapes into the garment's `.glb`,
//! as morph targets with their rules in the mesh's
//! `extras.axcenity_correctives`, and [`add_to_item`] into an item of a
//! library (its model and its manifest's `correctives`). A garment without
//! them poses as it did before.

use std::collections::BTreeMap;
use std::path::Path;

use axcenity_assets::{BoneTransform, Rig};
use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::pose::{CORRECTIVE_RULES, FinishPose, corrective_weights, skin, skin_matrices};
use super::surface::Surface;
use super::{BodyMesh, FitError};
use crate::PoseCorrective;

/// Every cloth corrective's name starts with this (and so with the body
/// correctives' `Corrective_`: a live shape, never identity).
pub const CLOTH_PREFIX: &str = "Corrective_Cloth_";

/// The sidecar's format version.
pub const POSE_SHAPES_VERSION: u32 = 1;

/// A file's axes (glTF): the character faces +Z.
const FORWARD: Vec3 = Vec3::Z;

/// Corners closer than this to the body's middle (along X) take their
/// shape from both sides' runs, blended; beyond it, from their own side's.
const SIDE_BLEND: f32 = 0.04;

/// How many rounds a solved shape is smoothed over the garment's surface:
/// the simulation's small crinkles (a coarse mesh squeezed into folds a
/// triangle wide) are left out, the cloth's larger move kept.
const SMOOTH_ROUNDS: usize = 12;

/// The most a key pose's shape moves a corner, in the pose (metres). The
/// simulation lets the bridge of cloth under the arm move 5 to 10 cm, and
/// folds it differently in each key pose, so that blends of whole moves
/// are far from the cloth between the key poses (checked half way: up to
/// 9 cm) and a shape the body's own shapes and other bodies' proportions
/// change opens or shuts the armpit (the gate's T-pose and arms-raised
/// cases). Measured on the test tee (pose-correctives.md): moves held to
/// 4, 6 and 10 mm, and whole ones; 6 mm did best in the stride.
const MOST_MOVE: f32 = 0.006;

/// A shape that moves no corner further than this is dropped (metres).
const LEAST_SHAPE: f32 = 0.0005;

/// A corner a shape moves less than this is left out of it (metres).
const LEAST_MOVE: f32 = 0.00005;

/// The rules of a garment on `rig`: the body's shoulder rules under cloth
/// names (lowered, swung forward and back, level, raised), and the elbow
/// bent, the thigh forward and back, and the knee bent, each about the
/// axis across its limb. A rig without a limb gets the rules its bones
/// allow.
pub fn cloth_rules(rig: &Rig) -> Vec<PoseCorrective> {
    let mut rules: Vec<PoseCorrective> = Vec::new();
    for &(shape, bone, axis, start, full) in &CORRECTIVE_RULES {
        if rig.bone(bone).is_none() {
            continue;
        }
        let shape = shape.replacen("Corrective_", CLOTH_PREFIX, 1);
        // A swing is the walk's: it spends most of a stride in the first
        // half of its range, where cloth doesn't follow a straight blend
        // to the full swing. So it has a shape full half way, and the
        // full one on top of it from there.
        if shape.contains("_Forward_") || shape.contains("_Back_") {
            let half = (start + full) / 2.0;
            rules.push(PoseCorrective {
                shape: shape.replacen("_Forward_", "_ForwardHalf_", 1).replacen(
                    "_Back_",
                    "_BackHalf_",
                    1,
                ),
                bone: bone.to_owned(),
                axis,
                angles: [start, half],
            });
            rules.push(PoseCorrective {
                shape,
                bone: bone.to_owned(),
                axis,
                angles: [half, full],
            });
            continue;
        }
        rules.push(PoseCorrective {
            shape,
            bone: bone.to_owned(),
            axis,
            angles: [start, full],
        });
    }
    let world = rig.world(&rig.rest_pose());
    let head = |bone: usize| world[bone].w_axis.truncate();
    // A world axis in the space of `bone`'s parent at rest (the rule's).
    let local = |bone: usize, axis: Vec3| -> [f32; 3] {
        let parent = rig.bones[bone].parent.map_or(Quat::IDENTITY, |p| {
            world[p].to_scale_rotation_translation().1
        });
        let a = (parent.inverse() * axis).normalize();
        // Rounded as the build data's rules are.
        a.to_array().map(|v| (v * 1e5).round() / 1e5)
    };
    // The axis across `bone` (its limb from `upper`) that turns `tail`'s
    // head `toward`.
    let across = |upper: usize, bone: usize, tail: usize, toward: Vec3| -> Vec3 {
        let limb = (head(bone) - head(upper)).normalize_or_zero();
        let axis = limb.cross(FORWARD).normalize_or_zero();
        let reach = head(tail) - head(bone);
        let turned = Quat::from_axis_angle(axis, 0.3) * reach;
        if (turned - reach).dot(toward) >= 0.0 {
            axis
        } else {
            -axis
        }
    };
    for side in ["Left", "Right"] {
        let bone = |name: &str| rig.bone(&format!("{side}{name}"));
        if let (Some(upper), Some(lower), Some(hand)) =
            (bone("UpperArm"), bone("LowerArm"), bone("Hand"))
        {
            rules.push(PoseCorrective {
                shape: format!("{CLOTH_PREFIX}Elbow_Bent_{side}"),
                bone: format!("{side}LowerArm"),
                axis: local(lower, across(upper, lower, hand, FORWARD)),
                angles: [0.0, 90.0],
            });
        }
        if let (Some(thigh), Some(shin), Some(foot)) =
            (bone("UpperLeg"), bone("LowerLeg"), bone("Foot"))
        {
            // The thigh swings about the axis across the body.
            let reach = head(shin) - head(thigh);
            let forward = if (Quat::from_axis_angle(Vec3::X, 0.3) * reach - reach).z >= 0.0 {
                Vec3::X
            } else {
                -Vec3::X
            };
            for (kind, axis, full) in [("Forward", forward, 30.0), ("Back", -forward, 20.0)] {
                rules.push(PoseCorrective {
                    shape: format!("{CLOTH_PREFIX}Hip_{kind}_{side}"),
                    bone: format!("{side}UpperLeg"),
                    axis: local(thigh, axis),
                    angles: [0.0, full],
                });
            }
            rules.push(PoseCorrective {
                shape: format!("{CLOTH_PREFIX}Knee_Bent_{side}"),
                bone: format!("{side}LowerLeg"),
                axis: local(shin, across(thigh, shin, foot, -FORWARD)),
                angles: [0.0, 90.0],
            });
        }
    }
    rules
}

/// A rule's weight in `bones` (0 when `rig` hasn't its bone): the body
/// correctives' rule, `anim::correctives::weight`.
pub fn rule_weight(rig: &Rig, bones: &[BoneTransform], rule: &PoseCorrective) -> f32 {
    let Some(index) = rig.bone(&rule.bone) else {
        return 0.0;
    };
    let rest = rig.bones[index].rest.rotation;
    let now = bones.get(index).map_or(rest, |b| b.rotation);
    let axis = Vec3::from(rule.axis).normalize_or_zero();
    let angle = super::pose::twist_angle(now * rest.inverse(), axis).to_degrees();
    let [start, full] = rule.angles;
    let t = ((angle - start) / (full - start)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What the bake does at one frame of a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClothRead {
    /// The cloth at rest, settled: what the simulation moves with no pose
    /// is the fit's business, so it is taken off every shape.
    Rest { frame: usize },
    /// A key pose: these shapes are solved here.
    Solve { frame: usize, shapes: Vec<String> },
    /// Half way to a key pose: the shapes solved, blended at their
    /// weights here, are checked against the simulation.
    Check { frame: usize },
}

impl ClothRead {
    pub fn frame(&self) -> usize {
        match self {
            ClothRead::Rest { frame }
            | ClothRead::Solve { frame, .. }
            | ClothRead::Check { frame } => *frame,
        }
    }
}

/// One simulation of the bake: the skeleton's pose in each of its frames
/// (every bone, relative to its parent, finished as the runtime finishes
/// a pose) and what is read when.
#[derive(Clone, Debug)]
pub struct ClothRun {
    pub name: String,
    pub frames: Vec<Vec<BoneTransform>>,
    pub reads: Vec<ClothRead>,
}

/// A key frame of a run: the rules' bones turned by these degrees about
/// their axes (applied in order on each bone, from rest), reached over
/// `moving` frames and held for `held`.
struct Key<'a> {
    turns: Vec<(&'a str, f32)>,
    moving: usize,
    held: usize,
    read: Option<ClothRead>,
}

/// A key frame as planned: the turns (shape, degrees), the frames held
/// and what is read there.
type Planned = (Vec<(String, f32)>, usize, Option<ClothRead>);

/// The runs that bake `rules` on `rig`: from rest, each run moves to half
/// a key pose (checked), then to the key pose (solved), with both sides
/// at once (the walk's swings and the hips one forward and one back, as
/// people walk). Swings are made on the hanging arm, after the hanging
/// arm's own shapes; raising past level after level's. `finish` is what
/// the runtime does to a pose before skinning (its twist bones).
pub fn cloth_runs(
    rig: &Rig,
    rules: &[PoseCorrective],
    finish: Option<FinishPose>,
) -> Vec<ClothRun> {
    let name = |kind: &str, side: &str| format!("{CLOTH_PREFIX}{kind}_{side}");
    let has = |shape: &str| rules.iter().any(|r| r.shape == shape);
    let rest = || Key {
        turns: Vec::new(),
        moving: 0,
        held: 10,
        read: Some(ClothRead::Rest { frame: 0 }),
    };
    let key = |turns: Vec<(String, f32)>, held: usize, read: Option<ClothRead>| (turns, held, read);
    let mut runs = Vec::new();
    let mut plan: Vec<(String, Vec<Planned>)> = Vec::new();
    let both = |kind: &str, degrees: f32| -> Vec<(String, f32)> {
        ["Left", "Right"]
            .iter()
            .map(|side| (name(kind, side), degrees))
            .filter(|(shape, _)| has(shape))
            .collect()
    };
    let solve = |shapes: Vec<String>| {
        Some(ClothRead::Solve {
            frame: 0,
            shapes: shapes.into_iter().filter(|s| has(s)).collect(),
        })
    };
    let check = || Some(ClothRead::Check { frame: 0 });
    let sides = |kind: &str| vec![name(kind, "Left"), name(kind, "Right")];
    // The arms let down to hang.
    plan.push((
        "hang".to_owned(),
        vec![
            key(both("Shoulder_Lowered", 17.5), 8, check()),
            key(
                both("Shoulder_Lowered", 35.0),
                14,
                solve(sides("Shoulder_Lowered")),
            ),
        ],
    ));
    // The walk's swings on the hanging arms, either way round.
    for (ahead, behind) in [("Left", "Right"), ("Right", "Left")] {
        let swung = |share: f32| {
            let mut turns = both("Shoulder_Lowered", 35.0);
            turns.push((name("Shoulder_Forward", ahead), 40.0 * share));
            turns.push((name("Shoulder_Back", behind), 50.0 * share));
            turns.retain(|(shape, _)| has(shape));
            turns
        };
        plan.push((
            format!("swing-{}", ahead.to_lowercase()),
            vec![
                key(swung(0.0), 6, None),
                key(swung(0.25), 8, check()),
                key(
                    swung(0.5),
                    14,
                    solve(vec![
                        name("Shoulder_ForwardHalf", ahead),
                        name("Shoulder_BackHalf", behind),
                    ]),
                ),
                key(swung(0.75), 8, check()),
                key(
                    swung(1.0),
                    14,
                    solve(vec![
                        name("Shoulder_Forward", ahead),
                        name("Shoulder_Back", behind),
                    ]),
                ),
            ],
        ));
    }
    // Raised to level, then on (the same axis: past 45 degrees the level
    // shape is full and the raised one leans in).
    plan.push((
        "level".to_owned(),
        vec![
            key(both("Shoulder_Level", 22.5), 8, check()),
            key(
                both("Shoulder_Level", 45.0),
                14,
                solve(sides("Shoulder_Level")),
            ),
        ],
    ));
    plan.push((
        "raised".to_owned(),
        vec![
            key(both("Shoulder_Level", 45.0), 4, None),
            key(both("Shoulder_Level", 75.0), 8, check()),
            key(
                both("Shoulder_Level", 105.0),
                14,
                solve(sides("Shoulder_Raised")),
            ),
        ],
    ));
    plan.push((
        "elbows".to_owned(),
        vec![
            key(both("Elbow_Bent", 45.0), 8, check()),
            key(both("Elbow_Bent", 90.0), 14, solve(sides("Elbow_Bent"))),
        ],
    ));
    for (ahead, behind) in [("Left", "Right"), ("Right", "Left")] {
        let strides = |share: f32| {
            let turns = vec![
                (name("Hip_Forward", ahead), 30.0 * share),
                (name("Hip_Back", behind), 20.0 * share),
            ];
            turns
                .into_iter()
                .filter(|(shape, _)| has(shape))
                .collect::<Vec<_>>()
        };
        plan.push((
            format!("hips-{}", ahead.to_lowercase()),
            vec![
                key(strides(0.5), 8, check()),
                key(
                    strides(1.0),
                    14,
                    solve(vec![name("Hip_Forward", ahead), name("Hip_Back", behind)]),
                ),
            ],
        ));
    }
    plan.push((
        "knees".to_owned(),
        vec![
            key(both("Knee_Bent", 45.0), 8, check()),
            key(both("Knee_Bent", 90.0), 14, solve(sides("Knee_Bent"))),
        ],
    ));

    for (run_name, keys) in plan {
        // A run whose key pose turns nothing on this rig is left out.
        let solves_any = keys.iter().any(|(_, _, read)| {
            matches!(read, Some(ClothRead::Solve { shapes, .. }) if !shapes.is_empty())
        });
        if !solves_any {
            continue;
        }
        let mut all = vec![rest()];
        all.extend(keys.iter().map(|(turns, held, read)| Key {
            turns: turns.iter().map(|(s, d)| (s.as_str(), *d)).collect(),
            moving: 16,
            held: *held,
            read: read.clone(),
        }));
        runs.push(expand(rig, rules, &run_name, &all, finish));
    }
    runs
}

/// The run's frames: each key's pose reached from the last by turning each
/// bone the short way (eased at both ends), then held; its read at the
/// last frame held.
fn expand(
    rig: &Rig,
    rules: &[PoseCorrective],
    name: &str,
    keys: &[Key],
    finish: Option<FinishPose>,
) -> ClothRun {
    let pose_of = |turns: &[(&str, f32)]| -> Vec<BoneTransform> {
        let mut bones = rig.rest_pose();
        for (shape, degrees) in turns {
            let Some(rule) = rules.iter().find(|r| r.shape == *shape) else {
                continue;
            };
            let Some(bone) = rig.bone(&rule.bone) else {
                continue;
            };
            let axis = Vec3::from(rule.axis).normalize_or_zero();
            bones[bone].rotation =
                Quat::from_axis_angle(axis, degrees.to_radians()) * bones[bone].rotation;
        }
        bones
    };
    let mut frames: Vec<Vec<BoneTransform>> = Vec::new();
    let mut reads = Vec::new();
    let mut last = rig.rest_pose();
    for key in keys {
        let target = pose_of(&key.turns);
        for step in 1..=key.moving {
            let t = step as f32 / key.moving as f32;
            let t = t * t * (3.0 - 2.0 * t);
            let bones = last
                .iter()
                .zip(&target)
                .map(|(a, b)| BoneTransform {
                    rotation: a.rotation.slerp(b.rotation, t),
                    ..*b
                })
                .collect();
            frames.push(bones);
        }
        for _ in 0..key.held.max(1) {
            frames.push(target.clone());
        }
        if let Some(read) = &key.read {
            let frame = frames.len() - 1;
            reads.push(match read {
                ClothRead::Rest { .. } => ClothRead::Rest { frame },
                ClothRead::Check { .. } => ClothRead::Check { frame },
                ClothRead::Solve { shapes, .. } => ClothRead::Solve {
                    frame,
                    shapes: shapes.clone(),
                },
            });
        }
        last = target;
    }
    if let Some(finish) = finish {
        for bones in &mut frames {
            finish(rig, bones);
        }
    }
    ClothRun {
        name: name.to_owned(),
        frames,
        reads,
    }
}

/// The body's skin posed in `bones`, with its corrective shapes at their
/// weights there (as the runtime poses it): positions and normals.
pub fn posed_body(body: &BodyMesh, bones: &[BoneTransform]) -> (Vec<Vec3>, Vec<Vec3>) {
    let weights = corrective_weights(&body.rig, bones);
    let (positions, normals) = body.shaped(&weights);
    let matrices = skin_matrices(&body.rig, bones);
    let (posed, turned, _) = skin(&matrices, &body.joints, &body.weights, &positions, &normals);
    (posed, turned)
}

/// The garment at rest with its shapes at the weights the runtime gives
/// them in `bones` (its copies of the body's correctives, and the cloth
/// correctives `solved` so far), and posed: (rest, posed, each corner's
/// linear part).
fn posed_garment(
    garment: &BodyMesh,
    bones: &[BoneTransform],
    rules: &[PoseCorrective],
    solved: &BTreeMap<String, Vec<Vec3>>,
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Mat3>) {
    let weights = corrective_weights(&garment.rig, bones);
    let (mut rest, _) = garment.shaped(&weights);
    for (shape, offsets) in solved {
        let Some(rule) = rules.iter().find(|r| r.shape == *shape) else {
            continue;
        };
        let w = rule_weight(&garment.rig, bones, rule);
        if w > 0.0 {
            for (p, o) in rest.iter_mut().zip(offsets) {
                *p += *o * w;
            }
        }
    }
    let matrices = skin_matrices(&garment.rig, bones);
    let empty = vec![Vec3::ZERO; rest.len()];
    let (posed, _, linear) = skin(&matrices, &garment.joints, &garment.weights, &rest, &empty);
    (rest, posed, linear)
}

/// A fingerprint of a garment's corners at rest: their count, the mean
/// of their places and their spread about it (metres, to a micrometre).
/// The bake is for this garment only: a fit that moved its corners by a
/// tenth of a millimetre or more changes it ([`same_garment`]), while a
/// file's own rounding (read through the engine's fitted space, or
/// straight from the file) doesn't.
pub fn fingerprint(positions: &[Vec3]) -> String {
    let n = positions.len().max(1) as f64;
    let mean = positions
        .iter()
        .fold([0.0f64; 3], |a, p| {
            [
                a[0] + f64::from(p.x),
                a[1] + f64::from(p.y),
                a[2] + f64::from(p.z),
            ]
        })
        .map(|v| v / n);
    let spread = positions
        .iter()
        .fold([0.0f64; 3], |a, p| {
            let d = [
                f64::from(p.x) - mean[0],
                f64::from(p.y) - mean[1],
                f64::from(p.z) - mean[2],
            ];
            [a[0] + d[0].abs(), a[1] + d[1].abs(), a[2] + d[2].abs()]
        })
        .map(|v| v / n);
    format!(
        "{}:{:.6},{:.6},{:.6}:{:.6},{:.6},{:.6}",
        positions.len(),
        mean[0],
        mean[1],
        mean[2],
        spread[0],
        spread[1],
        spread[2]
    )
}

/// Whether two [`fingerprint`]s are of one garment: the same count, and
/// means and spreads within 5 micrometres.
pub fn same_garment(a: &str, b: &str) -> bool {
    let parse = |text: &str| -> Option<(usize, Vec<f64>)> {
        let mut parts = text.split(':');
        let count = parts.next()?.parse().ok()?;
        let numbers: Option<Vec<f64>> = parts
            .flat_map(|part| part.split(','))
            .map(|v| v.parse().ok())
            .collect();
        Some((count, numbers?))
    };
    match (parse(a), parse(b)) {
        (Some((n, x)), Some((m, y))) => {
            n == m
                && x.len() == 6
                && y.len() == 6
                && x.iter().zip(&y).all(|(a, b)| (a - b).abs() < 5e-6)
        }
        _ => false,
    }
}

fn write_floats(path: &Path, values: impl Iterator<Item = Vec3>) -> Result<(), FitError> {
    let bytes: Vec<u8> = values
        .flat_map(|v| v.to_array())
        .flat_map(f32::to_le_bytes)
        .collect();
    write_file(path, &bytes)
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), FitError> {
    std::fs::write(path, bytes).map_err(|e| FitError::Write {
        path: path.display().to_string(),
        message: e.to_string(),
    })
}

/// Writes what the simulation needs into `work`: the body and garment at
/// rest (`body.rest.f32`, `body.triangles.u32`, `garment.rest.f32`,
/// `garment.triangles.u32`: little-endian, three to a corner or
/// triangle); per run, the skin posed in each frame (`<run>.body.f32`)
/// and the garment as skinning poses it there (`<run>.goal.f32`); and
/// `bake.json`, which lists them with their frames and reads, the rules,
/// the side each corner's shapes come from, and the garment's
/// [`fingerprint`].
pub fn write_bake_input(
    work: &Path,
    body: &BodyMesh,
    garment: &BodyMesh,
    rules: &[PoseCorrective],
    runs: &[ClothRun],
) -> Result<(), FitError> {
    std::fs::create_dir_all(work).map_err(|e| FitError::Write {
        path: work.display().to_string(),
        message: e.to_string(),
    })?;
    write_floats(&work.join("body.rest.f32"), body.positions.iter().copied())?;
    let triangles = |mesh: &BodyMesh| -> Vec<u8> {
        mesh.triangles
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    };
    write_file(&work.join("body.triangles.u32"), &triangles(body))?;
    write_floats(
        &work.join("garment.rest.f32"),
        garment.positions.iter().copied(),
    )?;
    write_file(&work.join("garment.triangles.u32"), &triangles(garment))?;
    let empty = BTreeMap::new();
    for run in runs {
        let mut skin_frames = Vec::new();
        let mut goals = Vec::new();
        for bones in &run.frames {
            let (posed, _) = posed_body(body, bones);
            skin_frames.extend(posed);
            let (_, cloth, _) = posed_garment(garment, bones, rules, &empty);
            goals.extend(cloth);
        }
        write_floats(
            &work.join(format!("{}.body.f32", run.name)),
            skin_frames.into_iter(),
        )?;
        write_floats(
            &work.join(format!("{}.goal.f32", run.name)),
            goals.into_iter(),
        )?;
    }
    let document = json!({
        "format_version": POSE_SHAPES_VERSION,
        "body": {"corners": body.positions.len(), "triangles": body.triangles.len()},
        "garment": {
            "corners": garment.positions.len(),
            "triangles": garment.triangles.len(),
            "fingerprint": fingerprint(&garment.positions),
        },
        "rules": rules,
        "runs": runs.iter().map(|run| json!({
            "name": run.name,
            "frames": run.frames.len(),
            "reads": run.reads,
        })).collect::<Vec<_>>(),
    });
    write_file(
        &work.join("bake.json"),
        serde_json::to_string_pretty(&document)
            .unwrap_or_default()
            .as_bytes(),
    )
}

/// One pose corrective's offsets, in the file's space at rest (metres):
/// the corners it moves, in rising order, and how far.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoseShape {
    pub name: String,
    pub corners: Vec<u32>,
    pub offsets: Vec<[f32; 3]>,
}

impl PoseShape {
    /// Its offsets as one per corner of a mesh of `count` corners.
    pub fn dense(&self, count: usize) -> Vec<Vec3> {
        let mut out = vec![Vec3::ZERO; count];
        for (&corner, offset) in self.corners.iter().zip(&self.offsets) {
            if let Some(slot) = out.get_mut(corner as usize) {
                *slot = Vec3::from(*offset);
            }
        }
        out
    }

    /// The farthest it moves a corner.
    pub fn reach(&self) -> f32 {
        self.offsets
            .iter()
            .map(|o| Vec3::from(*o).length())
            .fold(0.0, f32::max)
    }
}

/// A garment's baked pose correctives (the sidecar
/// `<Object>.correctives.json`): for the garment whose full-detail corners
/// at rest have `fingerprint`, its rules and shapes, and what the bake
/// measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoseShapes {
    pub format_version: u32,
    pub fingerprint: String,
    pub rules: Vec<PoseCorrective>,
    pub shapes: Vec<PoseShape>,
    /// What the bake measured (per shape and per check), for the
    /// hand-back; not read by anything.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub report: Value,
}

impl PoseShapes {
    pub fn from_json(text: &str) -> Result<Self, FitError> {
        let shapes: PoseShapes =
            serde_json::from_str(text).map_err(|e| FitError::Model(e.to_string()))?;
        if shapes.format_version != POSE_SHAPES_VERSION {
            return Err(FitError::Model(format!(
                "pose correctives of format {}; this build reads {POSE_SHAPES_VERSION}",
                shapes.format_version
            )));
        }
        Ok(shapes)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Its rules that have a shape (the rest moved nothing worth keeping).
    pub fn kept_rules(&self) -> Vec<PoseCorrective> {
        self.rules
            .iter()
            .filter(|rule| self.shapes.iter().any(|s| s.name == rule.shape))
            .cloned()
            .collect()
    }
}

/// How smooth `x`'s share of a side's shape is: 1 well on `side` (+1 or
/// -1 along X), 0 well on the other, eased across the middle.
fn side_share(x: f32, side: f32) -> f32 {
    if side == 0.0 {
        return 1.0;
    }
    let t = ((x * side + SIDE_BLEND) / (2.0 * SIDE_BLEND)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Solves the shapes from the simulation's results: `simulated(run, read)`
/// is the cloth (file space) at each of a run's reads in order. Shapes
/// are solved in the runs' order, each on top of the ones before.
pub fn solve(
    garment: &BodyMesh,
    rules: &[PoseCorrective],
    runs: &[ClothRun],
    simulated: &dyn Fn(usize, usize) -> Result<Vec<Vec3>, FitError>,
) -> Result<PoseShapes, FitError> {
    let count = garment.positions.len();
    let world = garment.rig.world(&garment.rig.rest_pose());
    let side_of = |rule: &PoseCorrective| -> f32 {
        garment
            .rig
            .bone(&rule.bone)
            .map_or(0.0, |b| world[b].w_axis.x.signum())
    };
    let neighbours = neighbours(garment);
    let mut solved: BTreeMap<String, Vec<Vec3>> = BTreeMap::new();
    let mut checks = Vec::new();
    for (r, run) in runs.iter().enumerate() {
        let mut drift = vec![Vec3::ZERO; count];
        for (k, read) in run.reads.iter().enumerate() {
            let cloth = simulated(r, k)?;
            if cloth.len() != count {
                return Err(FitError::Model(format!(
                    "{}: the simulation has {} corners, the garment {count}",
                    run.name,
                    cloth.len()
                )));
            }
            let bones = &run.frames[read.frame()];
            let (_, posed, linear) = posed_garment(garment, bones, rules, &solved);
            // The simulation's move here, less its move at rest; a key
            // pose's at most `MOST_MOVE` a corner (posed space). A check
            // compares the blend with the whole move.
            let moves: Vec<Vec3> = cloth
                .iter()
                .zip(&posed)
                .zip(&linear)
                .zip(&drift)
                .map(|(((c, p), l), d)| {
                    let m = *c - *p - *l * *d;
                    if matches!(read, ClothRead::Solve { .. }) {
                        m.clamp_length_max(MOST_MOVE)
                    } else {
                        m
                    }
                })
                .collect();
            let back: Vec<Vec3> = moves
                .iter()
                .zip(&linear)
                .map(|(m, l)| invert(*l) * *m)
                .collect();
            match read {
                ClothRead::Rest { .. } => drift = back,
                ClothRead::Check { .. } => {
                    let mut errors: Vec<f32> = back
                        .iter()
                        .zip(&linear)
                        .map(|(b, l)| (*l * *b).length())
                        .collect();
                    errors.sort_by(f32::total_cmp);
                    let at = |q: f32| errors[((errors.len() - 1) as f32 * q) as usize] * 1000.0;
                    checks.push(json!({
                        "run": run.name,
                        "frame": read.frame(),
                        "median_mm": at(0.5),
                        "p95_mm": at(0.95),
                        "most_mm": at(1.0),
                    }));
                }
                ClothRead::Solve { shapes, .. } => {
                    for shape in shapes {
                        let Some(rule) = rules.iter().find(|r| r.shape == *shape) else {
                            continue;
                        };
                        let w = rule_weight(&garment.rig, bones, rule);
                        if w < 0.5 {
                            return Err(FitError::Model(format!(
                                "{}: {shape} leans only {w:.2} in its key pose",
                                run.name
                            )));
                        }
                        let side = side_of(rule);
                        let offsets: Vec<Vec3> = back
                            .iter()
                            .zip(&garment.positions)
                            .map(|(b, rest)| *b * (side_share(rest.x, side) / w))
                            .collect();
                        solved.insert(shape.clone(), smooth(&neighbours, offsets));
                    }
                }
            }
        }
    }
    let mut shapes = Vec::new();
    let mut per_shape = Vec::new();
    for rule in rules {
        let Some(offsets) = solved.get(&rule.shape) else {
            continue;
        };
        let reach = offsets.iter().map(|o| o.length()).fold(0.0, f32::max);
        let corners: Vec<u32> = (0..count as u32)
            .filter(|&k| offsets[k as usize].length() > LEAST_MOVE)
            .collect();
        per_shape.push(json!({
            "shape": rule.shape,
            "most_mm": reach * 1000.0,
            "corners": corners.len(),
            "kept": reach >= LEAST_SHAPE,
        }));
        if reach < LEAST_SHAPE {
            continue;
        }
        shapes.push(PoseShape {
            name: rule.shape.clone(),
            offsets: corners
                .iter()
                .map(|&k| {
                    offsets[k as usize]
                        .to_array()
                        .map(|v| (v * 1e5).round() / 1e5)
                })
                .collect(),
            corners,
        });
    }
    Ok(PoseShapes {
        format_version: POSE_SHAPES_VERSION,
        fingerprint: fingerprint(&garment.positions),
        rules: rules.to_vec(),
        shapes,
        report: json!({"shapes": per_shape, "checks": checks}),
    })
}

/// Each corner's neighbours along the garment's edges, with corners at
/// one place (split for seams) taken as one: they share their neighbours
/// and so end with one offset, and the cloth doesn't part there.
fn neighbours(garment: &BodyMesh) -> Vec<Vec<u32>> {
    let mut place: std::collections::HashMap<[i64; 3], u32> = std::collections::HashMap::new();
    let welded: Vec<u32> = garment
        .positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let key = p.to_array().map(|v| (v * 1e6).round() as i64);
            *place.entry(key).or_insert(k as u32)
        })
        .collect();
    let mut around: Vec<Vec<u32>> = vec![Vec::new(); garment.positions.len()];
    for t in &garment.triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (a, b) = (welded[a as usize], welded[b as usize]);
            if a != b {
                around[a as usize].push(b);
                around[b as usize].push(a);
            }
        }
    }
    for list in &mut around {
        list.sort_unstable();
        list.dedup();
    }
    // Every corner reads its welded corner's list.
    welded.iter().map(|&w| around[w as usize].clone()).collect()
}

/// `offsets` smoothed [`SMOOTH_ROUNDS`] times: each round a corner moves
/// half way to its neighbours' mean.
fn smooth(neighbours: &[Vec<u32>], mut offsets: Vec<Vec3>) -> Vec<Vec3> {
    for _ in 0..SMOOTH_ROUNDS {
        offsets = neighbours
            .iter()
            .zip(&offsets)
            .map(|(around, own)| {
                if around.is_empty() {
                    return *own;
                }
                let mean =
                    around.iter().map(|&n| offsets[n as usize]).sum::<Vec3>() / around.len() as f32;
                (*own + mean) * 0.5
            })
            .collect();
    }
    offsets
}

fn invert(m: Mat3) -> Mat3 {
    if m.determinant().abs() < 1e-8 {
        Mat3::IDENTITY
    } else {
        m.inverse()
    }
}

/// A `.glb`'s JSON and binary chunks.
fn read_glb(bytes: &[u8]) -> Result<(Value, Vec<u8>), FitError> {
    let bad = || FitError::Model("not a binary glTF file".to_owned());
    let word = |at: usize| -> Result<u32, FitError> {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(bad)
    };
    if bytes.get(0..4) != Some(b"glTF") {
        return Err(bad());
    }
    let length = word(12)? as usize;
    let text = bytes.get(20..20 + length).ok_or_else(bad)?;
    let document: Value =
        serde_json::from_slice(text).map_err(|e| FitError::Model(e.to_string()))?;
    let at = 20 + length;
    let bin = if bytes.len() >= at + 8 {
        let size = word(at)? as usize;
        bytes.get(at + 8..at + 8 + size).ok_or_else(bad)?.to_vec()
    } else {
        Vec::new()
    };
    Ok((document, bin))
}

/// A float VEC3 accessor's values (tightly packed or strided; not sparse).
fn read_vec3s(document: &Value, bin: &[u8], index: usize) -> Result<Vec<Vec3>, FitError> {
    let bad = |why: &str| FitError::Model(format!("accessor {index}: {why}"));
    let accessor = &document["accessors"][index];
    if accessor["componentType"] != json!(5126) || accessor["type"] != json!("VEC3") {
        return Err(bad("not three floats"));
    }
    let count = accessor["count"].as_u64().ok_or_else(|| bad("no count"))? as usize;
    let view = &document["bufferViews"][accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| bad("no buffer view"))? as usize];
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().unwrap_or(12) as usize;
    let float = |at: usize| -> Result<f32, FitError> {
        bin.get(at..at + 4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| bad("past the buffer"))
    };
    (0..count)
        .map(|k| {
            let at = start + k * stride;
            Ok(Vec3::new(float(at)?, float(at + 4)?, float(at + 8)?))
        })
        .collect()
}

/// A garment mesh of a `.glb`: its index and each primitive's corners at
/// rest.
struct GlbMesh {
    index: usize,
    primitives: Vec<Vec<Vec3>>,
}

impl GlbMesh {
    fn positions(&self) -> Vec<Vec3> {
        self.primitives.iter().flatten().copied().collect()
    }
}

/// Writes `shapes` into a garment's `.glb` (`bytes`): in the mesh whose
/// corners at rest have the shapes' fingerprint (the full detail), each
/// shape as a morph target named as it is; in the garment's other meshes
/// (its simpler copies, `<object>_LOD<n>`), each corner taking the offsets
/// of the closest point of the full detail's surface; and the rules of the
/// shapes kept in each mesh's `extras.axcenity_correctives`, which the
/// runtime reads. Fails if no mesh is the garment the shapes were baked
/// for (the fit changed since the bake: bake again), or the garment
/// already has cloth correctives.
pub fn add_pose_shapes(bytes: &[u8], shapes: &PoseShapes) -> Result<Vec<u8>, FitError> {
    let (mut document, mut bin) = read_glb(bytes)?;
    let mut meshes = Vec::new();
    for (index, mesh) in document["meshes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let names = mesh["extras"]["targetNames"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if names
            .iter()
            .any(|n| n.as_str().is_some_and(|n| n.starts_with(CLOTH_PREFIX)))
        {
            return Err(FitError::Model(format!(
                "mesh {index} already has cloth correctives: bake from the garment as exported"
            )));
        }
        let mut primitives = Vec::new();
        for primitive in mesh["primitives"].as_array().cloned().unwrap_or_default() {
            let Some(position) = primitive["attributes"]["POSITION"].as_u64() else {
                continue;
            };
            primitives.push(read_vec3s(&document, &bin, position as usize)?);
        }
        meshes.push(GlbMesh { index, primitives });
    }
    let full = meshes
        .iter()
        .position(|mesh| same_garment(&fingerprint(&mesh.positions()), &shapes.fingerprint))
        .ok_or_else(|| {
            FitError::Model(format!(
                "no mesh is the garment these correctives were baked for ({}): the fit changed since; bake again",
                shapes.fingerprint
            ))
        })?;
    let rest = meshes[full].positions();
    let count = rest.len();
    let dense: Vec<Vec<Vec3>> = shapes.shapes.iter().map(|s| s.dense(count)).collect();
    let surface = {
        let mut triangles = Vec::new();
        let mut start = 0u32;
        let mesh = &document["meshes"][meshes[full].index];
        for (k, primitive) in mesh["primitives"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let corners = meshes[full].primitives.get(k).map_or(0, Vec::len) as u32;
            if let Some(indices) = primitive["indices"].as_u64() {
                let list = read_indices(&document, &bin, indices as usize)?;
                triangles.extend(
                    list.chunks_exact(3)
                        .map(|t| [t[0] + start, t[1] + start, t[2] + start]),
                );
            }
            start += corners;
        }
        Surface::new(rest.clone(), Vec::new(), triangles)
    };
    let rules = serde_json::to_value(shapes.kept_rules()).unwrap_or_default();
    let mut out = Out::new(&mut document, &mut bin);
    for mesh in &meshes {
        // Each corner of this mesh: the offsets of each shape there.
        let offsets: Vec<Vec<Vec3>> = if mesh.index == meshes[full].index {
            dense.clone()
        } else {
            let positions = mesh.positions();
            let places: Vec<(Vec3, [u32; 3])> = positions
                .iter()
                .map(|p| {
                    surface
                        .closest(*p, 0.05)
                        .map_or((Vec3::X, [0, 0, 0]), |hit| {
                            (hit.bary, surface.triangles[hit.triangle as usize])
                        })
                })
                .collect();
            dense
                .iter()
                .map(|d| {
                    places
                        .iter()
                        .map(|(bary, t)| {
                            d[t[0] as usize] * bary.x
                                + d[t[1] as usize] * bary.y
                                + d[t[2] as usize] * bary.z
                        })
                        .collect()
                })
                .collect()
        };
        out.add(mesh, &shapes.shapes, &offsets, &rules);
    }
    Ok(super::glb::glb(&document, &bin))
}

fn read_indices(document: &Value, bin: &[u8], index: usize) -> Result<Vec<u32>, FitError> {
    let bad = |why: &str| FitError::Model(format!("accessor {index}: {why}"));
    let accessor = &document["accessors"][index];
    let count = accessor["count"].as_u64().ok_or_else(|| bad("no count"))? as usize;
    let view = &document["bufferViews"][accessor["bufferView"]
        .as_u64()
        .ok_or_else(|| bad("no buffer view"))? as usize];
    let start = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let size = match accessor["componentType"].as_u64() {
        Some(5121) => 1,
        Some(5123) => 2,
        Some(5125) => 4,
        _ => return Err(bad("not indices")),
    };
    (0..count)
        .map(|k| {
            let at = start + k * size;
            let b = bin
                .get(at..at + size)
                .ok_or_else(|| bad("past the buffer"))?;
            Ok(match size {
                1 => u32::from(b[0]),
                2 => u32::from(u16::from_le_bytes([b[0], b[1]])),
                _ => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            })
        })
        .collect()
}

/// Appends morph targets to a `.glb`'s document and binary chunk.
struct Out<'a> {
    document: &'a mut Value,
    bin: &'a mut Vec<u8>,
}

impl<'a> Out<'a> {
    fn new(document: &'a mut Value, bin: &'a mut Vec<u8>) -> Self {
        Out { document, bin }
    }

    fn view(&mut self, data: &[u8]) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let views = self.document["bufferViews"]
            .as_array_mut()
            .expect("a glTF file's buffer views");
        views.push(json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": data.len()}));
        self.bin.extend_from_slice(data);
        views.len() - 1
    }

    /// A sparse accessor of `count` offsets, zero but at `corners` (glTF
    /// wants one value at least: a shape that moves nothing here keeps a
    /// zero at corner 0).
    fn sparse(&mut self, count: usize, corners: &[u32], values: &[Vec3]) -> usize {
        let (corners, values) = if corners.is_empty() {
            (vec![0u32], vec![Vec3::ZERO])
        } else {
            (corners.to_vec(), values.to_vec())
        };
        let indices: Vec<u8> = corners.iter().flat_map(|i| i.to_le_bytes()).collect();
        let indices = self.view(&indices);
        let data: Vec<u8> = values
            .iter()
            .flat_map(|v| v.to_array())
            .flat_map(f32::to_le_bytes)
            .collect();
        let data = self.view(&data);
        let (lo, hi) = values.iter().fold((Vec3::ZERO, Vec3::ZERO), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
        let accessors = self.document["accessors"]
            .as_array_mut()
            .expect("a glTF file's accessors");
        accessors.push(json!({
            "componentType": 5126, "count": count, "type": "VEC3",
            "min": lo.to_array(), "max": hi.to_array(),
            "sparse": {
                "count": corners.len(),
                "indices": {"bufferView": indices, "componentType": 5125},
                "values": {"bufferView": data},
            },
        }));
        accessors.len() - 1
    }

    /// Adds each of `shapes` to `mesh`, its corners moved by `offsets`
    /// (per shape, one per corner of the mesh), and the rules.
    fn add(&mut self, mesh: &GlbMesh, shapes: &[PoseShape], offsets: &[Vec<Vec3>], rules: &Value) {
        let mut start = 0;
        let mut targets: Vec<Vec<Value>> = Vec::new();
        for corners in &mesh.primitives {
            let count = corners.len();
            let mut list = Vec::new();
            for offsets in offsets {
                let here = &offsets[start..start + count];
                let moved: Vec<u32> = (0..count as u32)
                    .filter(|&k| here[k as usize].length() > LEAST_MOVE)
                    .collect();
                let values: Vec<Vec3> = moved.iter().map(|&k| here[k as usize]).collect();
                list.push(json!({"POSITION": self.sparse(count, &moved, &values)}));
            }
            targets.push(list);
            start += count;
        }
        let entry = &mut self.document["meshes"][mesh.index];
        let primitives = entry["primitives"]
            .as_array_mut()
            .expect("a mesh's primitives");
        let mut with_position = primitives
            .iter_mut()
            .filter(|p| p["attributes"]["POSITION"].is_u64());
        for list in targets {
            let Some(primitive) = with_position.next() else {
                break;
            };
            let object = primitive.as_object_mut().expect("a primitive");
            let held = object.entry("targets").or_insert_with(|| json!([]));
            held.as_array_mut().expect("targets").extend(list);
        }
        let mesh_object = entry.as_object_mut().expect("a mesh");
        let had = mesh_object
            .get("extras")
            .and_then(|e| e["targetNames"].as_array())
            .map_or(0, Vec::len);
        let extras = mesh_object.entry("extras").or_insert_with(|| json!({}));
        let names = extras
            .as_object_mut()
            .expect("extras")
            .entry("targetNames")
            .or_insert_with(|| json!([]));
        names
            .as_array_mut()
            .expect("target names")
            .extend(shapes.iter().map(|s| json!(s.name)));
        extras["axcenity_correctives"] = rules.clone();
        if let Some(weights) = mesh_object.get_mut("weights").and_then(Value::as_array_mut) {
            weights.resize(had + shapes.len(), json!(0.0));
        }
    }
}

/// Adds `shapes` to the item `id` of the library at `library`: its model
/// file ([`add_pose_shapes`]) and its manifest's `correctives` (the rules
/// of the shapes kept). Returns how many shapes it added.
pub fn add_to_item(library: &Path, id: &str, shapes: &PoseShapes) -> Result<usize, FitError> {
    let (read, _, problems) = crate::Library::read(library);
    if !problems.is_empty() {
        return Err(FitError::Library(format!("{problems:?}")));
    }
    let item = read
        .item(id)
        .ok_or_else(|| FitError::Library(format!("no item `{id}`")))?
        .clone();
    let model = item
        .model
        .as_ref()
        .ok_or_else(|| FitError::Library(format!("`{id}` has no model")))?;
    let manifest = read
        .manifest_path(id)
        .ok_or_else(|| FitError::Library(format!("`{id}` has no manifest")))?
        .to_owned();
    let model_path =
        library
            .join(read.path_of(id, &model.file).ok_or_else(|| {
                FitError::Library(format!("`{id}`'s model isn't in the library"))
            })?);
    let bytes = std::fs::read(&model_path).map_err(|e| FitError::Library(e.to_string()))?;
    let written = add_pose_shapes(&bytes, shapes)?;
    write_file(&model_path, &written)?;
    let mut item = item;
    item.correctives = shapes.kept_rules();
    write_file(&library.join(manifest), item.to_json().as_bytes())?;
    Ok(shapes.shapes.len())
}

/// The cloth correctives of `rules` a model carries, by name, if any:
/// whether every shape has a rule there and every rule a shape.
pub fn rules_match(model: &axcenity_assets::Model, rules: &[PoseCorrective]) -> Result<(), String> {
    let driven: Vec<(&str, &axcenity_assets::PoseDriver)> = model
        .morph_targets
        .iter()
        .filter_map(|t| t.driver.as_ref().map(|d| (t.name.as_str(), d)))
        .collect();
    for rule in rules {
        let Some((_, driver)) = driven.iter().find(|(name, _)| *name == rule.shape) else {
            return Err(format!("the model has no driven shape `{}`", rule.shape));
        };
        if driver.bone != rule.bone
            || driver.axis != rule.axis
            || [driver.start, driver.full] != rule.angles
        {
            return Err(format!("`{}` is driven otherwise in the model", rule.shape));
        }
    }
    if let Some((name, _)) = driven
        .iter()
        .find(|(name, _)| rules.iter().all(|r| r.shape != *name))
    {
        return Err(format!("the model drives `{name}`, the manifest doesn't"));
    }
    Ok(())
}

/// The base body and the garment of item `id` in the library at `root`,
/// as fitting sees them (the full detail, in the file's space).
pub fn item_meshes(root: &Path, id: &str) -> Result<(BodyMesh, BodyMesh), FitError> {
    let (library, _, problems) = crate::Library::read(root);
    if !problems.is_empty() {
        return Err(FitError::Library(format!("{problems:?}")));
    }
    let item = library
        .item(id)
        .ok_or_else(|| FitError::Library(format!("no item `{id}`")))?;
    let base = item
        .bases
        .first()
        .ok_or_else(|| FitError::Library(format!("`{id}` fits no base")))?;
    let (_, body_id) = super::import::base_of(&library, base)?;
    let model = item
        .model
        .as_ref()
        .ok_or_else(|| FitError::Library(format!("`{id}` has no model")))?;
    let file = library
        .path_of(id, &model.file)
        .ok_or_else(|| FitError::Library(format!("`{id}`'s model isn't in the library")))?;
    let mesh_id = match &model.object {
        Some(object) => format!("{file}#{object}"),
        None => file,
    };
    let assets = axcenity_assets::Assets::new(root);
    let load = |mesh_id: &str| {
        assets
            .model_blocking(mesh_id)
            .ok_or_else(|| FitError::Model(format!("`{mesh_id}` doesn't load")))
    };
    let body = BodyMesh::from_model(&*load(&body_id)?)?;
    let garment = BodyMesh::from_model(&*load(&mesh_id)?)?;
    Ok((body, garment))
}
