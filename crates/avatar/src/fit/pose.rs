//! Poses a garment is fitted in (CO9 review fixes 2): the body suite's
//! held poses and walk-like swings of the arms and legs, built on the
//! body's own skeleton, with the body's pose correctives' weights there.
//!
//! A garment skinned with the body's weights follows the skin closely
//! where it lies on it, but not where it stands off it across a joint
//! (a T-shirt's underarm, its shoulder by the neck): there linear skinning
//! moves the cloth and the skin under it differently, and the skin comes
//! through. [`fit_garment`](super::fit_garment) poses the garment and the
//! body in each of these poses and pushes the cloth out where it must be,
//! into its copies of the correctives the pose leans on (so the runtime
//! drives the push with the body's own live weights) or, where no
//! corrective is leaning, into its rest.

use axcenity_assets::{BoneTransform, Rig};
use glam::{Mat3, Mat4, Quat, Vec3};

/// A model file's axes (glTF): Y up, the character facing +Z.
const UP: Vec3 = Vec3::Y;
const FORWARD: Vec3 = Vec3::Z;

/// A pose the fit checks a garment in: every bone (relative to its
/// parent) and the body's corrective shapes' weights there.
#[derive(Clone, Debug)]
pub struct FitPose {
    pub name: String,
    pub bones: Vec<BoneTransform>,
    pub correctives: Vec<(String, f32)>,
}

/// What the runtime does to a finished pose before it skins with it: on
/// the realistic body, turning the twist and helper bones that follow the
/// limbs (`axcenity_anim::twist::drive`, which `avatar` can't call: it has
/// no animation code). The fit's poses turn only the limbs; with this they
/// are the runtime's.
pub type FinishPose = fn(&Rig, &mut Vec<BoneTransform>);

/// A corrective shape's rule, as `anim::correctives` drives it at run
/// time: (shape, bone, axis in the space of the bone's parent at rest,
/// degrees at which it starts, degrees at which it is full). The same
/// numbers as `anim`'s `CORRECTIVES` and the realistic build data's
/// `correctives` (a test holds them to the build data).
pub type CorrectiveRule = (&'static str, &'static str, [f32; 3], f32, f32);

/// The realistic body's corrective rules (its shoulders: raised, level,
/// lowered, and the hanging arm swung forward and back).
pub const CORRECTIVE_RULES: [CorrectiveRule; 10] = [
    (
        "Corrective_Shoulder_Level_Left",
        "LeftUpperArm",
        [0.0, -0.37768, 0.92594],
        0.0,
        45.0,
    ),
    (
        "Corrective_Shoulder_Level_Right",
        "RightUpperArm",
        [0.0, 0.37698, -0.92622],
        0.0,
        45.0,
    ),
    (
        "Corrective_Shoulder_Raised_Left",
        "LeftUpperArm",
        [0.0, -0.37768, 0.92594],
        45.0,
        105.0,
    ),
    (
        "Corrective_Shoulder_Raised_Right",
        "RightUpperArm",
        [0.0, 0.37698, -0.92622],
        45.0,
        105.0,
    ),
    (
        "Corrective_Shoulder_Lowered_Left",
        "LeftUpperArm",
        [0.0, 0.37768, -0.92594],
        0.0,
        35.0,
    ),
    (
        "Corrective_Shoulder_Lowered_Right",
        "RightUpperArm",
        [0.0, -0.37698, 0.92622],
        0.0,
        35.0,
    ),
    (
        "Corrective_Shoulder_Forward_Left",
        "LeftUpperArm",
        [0.32884, -0.87444, -0.35668],
        0.0,
        40.0,
    ),
    (
        "Corrective_Shoulder_Forward_Right",
        "RightUpperArm",
        [0.32888, 0.8747, 0.35601],
        0.0,
        40.0,
    ),
    (
        "Corrective_Shoulder_Back_Left",
        "LeftUpperArm",
        [-0.32884, 0.87444, 0.35668],
        0.0,
        50.0,
    ),
    (
        "Corrective_Shoulder_Back_Right",
        "RightUpperArm",
        [-0.32888, -0.8747, -0.35601],
        0.0,
        50.0,
    ),
];

/// A rotation's twist about `axis` (radians, the short way round): how
/// far it turns about that axis alone.
fn twist_angle(rotation: Quat, axis: Vec3) -> f32 {
    let along = Vec3::new(rotation.x, rotation.y, rotation.z).dot(axis);
    let angle = 2.0 * along.atan2(rotation.w);
    if angle > std::f32::consts::PI {
        angle - std::f32::consts::TAU
    } else if angle < -std::f32::consts::PI {
        angle + std::f32::consts::TAU
    } else {
        angle
    }
}

/// The weight of each corrective shape of [`CORRECTIVE_RULES`] whose bone
/// `rig` has, in `bones` (as `anim` sets them at run time).
pub fn corrective_weights(rig: &Rig, bones: &[BoneTransform]) -> Vec<(String, f32)> {
    CORRECTIVE_RULES
        .iter()
        .filter_map(|&(shape, bone, axis, start, full)| {
            let index = rig.bone(bone)?;
            let rest = rig.bones[index].rest.rotation;
            let now = bones.get(index).map_or(rest, |b| b.rotation);
            let angle =
                twist_angle(now * rest.inverse(), Vec3::from(axis).normalize()).to_degrees();
            // Eased at both ends, as `anim::correctives::ramp`.
            let t = ((angle - start) / (full - start)).clamp(0.0, 1.0);
            let weight = t * t * (3.0 - 2.0 * t);
            Some((shape.to_owned(), weight))
        })
        .collect()
}

/// A pose being built: bones turned one after another, each about its
/// head in the file's space.
struct Posing<'a> {
    rig: &'a Rig,
    bones: Vec<BoneTransform>,
    finish: Option<FinishPose>,
}

impl<'a> Posing<'a> {
    fn new(rig: &'a Rig, finish: Option<FinishPose>) -> Self {
        Posing {
            rig,
            bones: rig.rest_pose(),
            finish,
        }
    }

    fn world(&self) -> Vec<Mat4> {
        self.rig.world(&self.bones)
    }

    fn head(&self, bone: usize) -> Vec3 {
        self.world()[bone].w_axis.truncate()
    }

    /// Turns `bone` by `turn` (in the file's space) about its head.
    fn turn(&mut self, bone: usize, turn: Quat) {
        let world = self.world();
        let head = world[bone].w_axis.truncate();
        let turned = Mat4::from_translation(head)
            * Mat4::from_quat(turn)
            * Mat4::from_translation(-head)
            * world[bone];
        let parent = self.rig.bones[bone]
            .parent
            .map_or(Mat4::IDENTITY, |parent| world[parent]);
        let (scale, rotation, translation) =
            (parent.inverse() * turned).to_scale_rotation_translation();
        self.bones[bone] = BoneTransform {
            translation,
            rotation,
            scale,
        };
    }

    /// Turns `bone` by `degrees` about `axis`, the way that moves the head
    /// of `tail` (the bone below it) further `toward`.
    fn bend(&mut self, bone: usize, tail: usize, axis: Vec3, degrees: f32, toward: Vec3) {
        let (head, end) = (self.head(bone), self.head(tail));
        let turn = |degrees: f32| Quat::from_axis_angle(axis.normalize(), degrees.to_radians());
        let moves = |turn: Quat| (turn * (end - head) - (end - head)).dot(toward);
        let turn = if moves(turn(degrees)) >= moves(turn(-degrees)) {
            turn(degrees)
        } else {
            turn(-degrees)
        };
        self.turn(bone, turn);
    }

    /// Raises `side`'s upper arm (in the body's front plane) to `degrees`
    /// from hanging straight down, as the body suite's held poses do.
    fn arm_out(&mut self, side: &str, degrees: f32) {
        let (Some(upper), Some(lower)) = (
            self.rig.bone(&format!("{side}UpperArm")),
            self.rig.bone(&format!("{side}LowerArm")),
        ) else {
            return;
        };
        let arm = (self.head(lower) - self.head(upper)).normalize();
        let turn = degrees - arm.angle_between(-UP).to_degrees();
        let toward = if turn >= 0.0 { UP } else { -UP };
        self.bend(upper, lower, FORWARD, turn.abs(), toward);
    }

    /// Swings `side`'s upper arm forward (or back, negative) by `degrees`
    /// about the axis across the body.
    fn arm_swing(&mut self, side: &str, degrees: f32) {
        self.swing(
            &format!("{side}UpperArm"),
            &format!("{side}LowerArm"),
            degrees,
        );
    }

    /// Swings `side`'s thigh forward (or back, negative).
    fn leg_swing(&mut self, side: &str, degrees: f32) {
        self.swing(
            &format!("{side}UpperLeg"),
            &format!("{side}LowerLeg"),
            degrees,
        );
    }

    fn swing(&mut self, bone: &str, tail: &str, degrees: f32) {
        let (Some(bone), Some(tail)) = (self.rig.bone(bone), self.rig.bone(tail)) else {
            return;
        };
        let toward = if degrees >= 0.0 { FORWARD } else { -FORWARD };
        self.bend(bone, tail, Vec3::X, degrees.abs(), toward);
    }

    /// Bends `side`'s elbow (forearm forward) or knee (shin back) by
    /// `degrees`, about the axis across the limb, as the held poses do.
    fn bend_limb(&mut self, side: &str, elbow: bool, degrees: f32) {
        let (upper, lower, end, toward) = if elbow {
            ("UpperArm", "LowerArm", "Hand", FORWARD)
        } else {
            ("UpperLeg", "LowerLeg", "Foot", -FORWARD)
        };
        let bone = |name: &str| self.rig.bone(&format!("{side}{name}"));
        let (Some(upper), Some(lower), Some(end)) = (bone(upper), bone(lower), bone(end)) else {
            return;
        };
        let limb = (self.head(lower) - self.head(upper)).normalize();
        self.bend(lower, end, limb.cross(FORWARD), degrees, toward);
    }

    fn done(mut self, name: &str) -> FitPose {
        if let Some(finish) = self.finish {
            finish(self.rig, &mut self.bones);
        }
        FitPose {
            name: name.to_owned(),
            correctives: corrective_weights(self.rig, &self.bones),
            bones: self.bones,
        }
    }
}

/// The poses a garment on `rig` is fitted in: the body suite's held poses
/// (the arms level and raised to 150°, the elbows and knees bent 135°),
/// the arms hanging, and a walk's swings either way round (arms hanging
/// and swung 40° forward and back, thighs 30°, elbows a little bent). A
/// rig without the humanoid's limbs gets the poses its bones allow.
/// `finish` is what the runtime does to a pose before skinning
/// ([`FinishPose`]).
pub fn fit_poses(rig: &Rig, finish: Option<FinishPose>) -> Vec<FitPose> {
    let mut poses = Vec::new();
    for (name, degrees) in [
        ("arms-hanging", 5.0),
        ("t-pose", 90.0),
        ("arms-raised", 150.0),
    ] {
        let mut posing = Posing::new(rig, finish);
        for side in ["Left", "Right"] {
            posing.arm_out(side, degrees);
        }
        poses.push(posing.done(name));
    }
    for (name, elbow) in [("elbows-bent", true), ("knees-bent", false)] {
        let mut posing = Posing::new(rig, finish);
        for side in ["Left", "Right"] {
            posing.bend_limb(side, elbow, 135.0);
        }
        poses.push(posing.done(name));
    }
    // A walk's swings, both ways round: one arm forward with the other
    // leg, as people walk.
    for (name, ahead) in [("walk-left", "Left"), ("walk-right", "Right")] {
        let behind = if ahead == "Left" { "Right" } else { "Left" };
        for (arm_degrees, forward, back) in [(15.0, 15.0, 25.0), (40.0, 30.0, 48.0)] {
            let mut posing = Posing::new(rig, finish);
            for side in ["Left", "Right"] {
                posing.arm_out(side, 14.0);
            }
            posing.arm_swing(ahead, forward);
            posing.arm_swing(behind, -back);
            posing.leg_swing(behind, 30.0);
            posing.leg_swing(ahead, -20.0);
            for side in ["Left", "Right"] {
                posing.bend_limb(side, true, 30.0);
            }
            posing.bend_limb(ahead, false, 40.0);
            poses.push(posing.done(&format!("{name}-{arm_degrees}")));
        }
    }
    poses
}

/// Each joint's matrix in `bones`: from a vertex as bound (the file's
/// space) to where the pose puts it (the file's space).
pub fn skin_matrices(rig: &Rig, bones: &[BoneTransform]) -> Vec<Mat4> {
    let world = rig.world(bones);
    rig.joints
        .iter()
        .map(|joint| world.get(joint.bone).copied().unwrap_or(Mat4::IDENTITY) * joint.inverse_bind)
        .collect()
}

/// One corner's skinning: its joints' matrices mixed by its weights.
pub fn blend(matrices: &[Mat4], joints: &[u16; 4], weights: &[f32; 4]) -> Mat4 {
    let mut out = Mat4::ZERO;
    let mut total = 0.0;
    for (joint, weight) in joints.iter().zip(weights) {
        if *weight > 0.0
            && let Some(m) = matrices.get(*joint as usize)
        {
            out += *m * *weight;
            total += *weight;
        }
    }
    if total <= 0.0 {
        Mat4::IDENTITY
    } else {
        out * (1.0 / total)
    }
}

/// `positions` and `normals` skinned by `matrices` with each corner's
/// `joints` and `weights`; and each corner's linear part (to take a move
/// in the pose back to rest).
pub fn skin(
    matrices: &[Mat4],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    positions: &[Vec3],
    normals: &[Vec3],
) -> (Vec<Vec3>, Vec<Vec3>, Vec<Mat3>) {
    let mut posed = Vec::with_capacity(positions.len());
    let mut turned = Vec::with_capacity(positions.len());
    let mut linear = Vec::with_capacity(positions.len());
    for (k, p) in positions.iter().enumerate() {
        let m = blend(matrices, &joints[k], &weights[k]);
        let l = Mat3::from_mat4(m);
        posed.push(m.transform_point3(*p));
        let n = normals.get(k).copied().unwrap_or(Vec3::ZERO);
        // Normals turn by the inverse transpose.
        turned.push((l.inverse().transpose() * n).normalize_or_zero());
        linear.push(l);
    }
    (posed, turned, linear)
}
