//! Pose-driven corrective shapes (Character Studio overhaul, CO5): blend
//! shapes a body leans on as a bone turns, which put back what skinning
//! loses at a joint. The realistic base body had ten, for its shoulders:
//! raised, level, lowered, and the hanging arm swung forward and back as a
//! walk swings it. That base was retired (ADR-0050) and the human base has
//! none yet, so no shipped body leans on these rules now; a body that
//! carries shapes of these names does.
//!
//! **The rule.** A shape's weight is how far its bone has turned from rest
//! about an axis, from 0 at `start` degrees to 1 at `full` and beyond. The
//! turn is the bone's own, in its parent's space (its rotation now times
//! the inverse of its rest rotation), and its angle about the axis is its
//! twist about it ([`crate::twist::twist_angle`]): an arm swung forward
//! leans on a shape made for raising it only as far as it is also raised.
//! The axis is in the space of the bone's parent at rest.
//!
//! These are **live** shapes, unlike the ones a character's description
//! drives (identity, baked into its model, ADR-0038): they are set on every
//! pose, after the face's. A character's parts take the body's shape
//! weights by name, so a garment that has a shape of the same name follows.
//!
//! The rules are constants here because `anim` doesn't read manifests; a
//! test holds them to the build data's. Architecture.md section 7 keeps
//! the same form for a manifest field (`correctives`), which would replace
//! the constants.

use axcenity_assets::{Model, Pose, Posed, Rig};
use glam::{Quat, Vec3};

use crate::twist::twist_angle;

/// A corrective shape's rule: (shape, bone, axis in the space of the
/// bone's parent at rest, degrees at which it starts, degrees at which it
/// is full).
pub type Rule = (&'static str, &'static str, [f32; 3], f32, f32);

/// The corrective shapes of a body built as the realistic one was: the shoulder's cap, held as the
/// arm is raised to level (45° up from its rest, which is 45° below
/// level) and on to 150° from hanging, and as it is lowered to hang (35°
/// down from rest: about the raise axis reversed, so the rule's form is
/// the same); and the hanging arm swung forward (full at 40°) and back
/// (full at 50°) about the axis across the body, which keeps the skin
/// where the arm meets the chest or the back from folding out in a walk
/// (CO9 part 2). A swing has no part about the raise axis, nor a raise
/// about the swing's, so the two kinds lean independently.
pub const CORRECTIVES: [Rule; 10] = [
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

/// Every corrective shape's name starts with this.
pub const PREFIX: &str = "Corrective_";

/// A rule found on a model: which of its blend shapes, which bone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Found {
    pub morph: usize,
    pub bone: usize,
    pub axis: Vec3,
    pub start: f32,
    pub full: f32,
}

/// The corrective shapes of one model on its skeleton.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Correctives(pub Vec<Found>);

impl Correctives {
    /// The rules of [`CORRECTIVES`] whose shape `model` has and whose bone
    /// `rig` has. Nearly every model has none, which costs a look at each
    /// of its blend shapes' names.
    pub fn detect(model: &Model, rig: &Rig) -> Self {
        let mut found = Vec::new();
        for (morph, target) in model.morph_targets.iter().enumerate() {
            if !target.name.starts_with(PREFIX) {
                continue;
            }
            let Some(&(_, bone, axis, start, full)) =
                CORRECTIVES.iter().find(|rule| rule.0 == target.name)
            else {
                continue;
            };
            if let Some(bone) = rig.bone(bone).filter(|_| start < full) {
                found.push(Found {
                    morph,
                    bone,
                    axis: Vec3::from(axis).normalize_or_zero(),
                    start,
                    full,
                });
            }
        }
        Self(found)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Each found shape's index and weight for `pose`.
    pub fn weights<'a>(
        &'a self,
        rig: &'a Rig,
        pose: &'a Pose,
    ) -> impl Iterator<Item = (usize, f32)> + 'a {
        self.0.iter().map(|rule| {
            let rest = rig.bones[rule.bone].rest.rotation;
            let now = pose[rule.bone].rotation;
            (
                rule.morph,
                weight(rest, now, rule.axis, rule.start, rule.full),
            )
        })
    }
}

/// A corrective shape's weight (0 to 1) for a bone whose rotation in its
/// parent is `posed` and was `rest`: how far it has turned about `axis`
/// (in the parent's space), from 0 at `start` degrees to 1 at `full`.
///
/// The ramp between is smooth (`ramp`): it starts and ends level, so a
/// joint turning steadily through `start` or `full` (an arm swinging
/// through hanging, from its Back shape to its Forward one, in every step
/// of a walk) eases the shape in and out instead of starting it at full
/// speed, which made cloth carrying a large copy of the shape jolt in that
/// frame (CO9 part 2, the motion check).
pub fn weight(rest: Quat, posed: Quat, axis: Vec3, start: f32, full: f32) -> f32 {
    // The bone's turn from rest, in its parent's axes.
    let turn = posed * rest.inverse();
    let angle = twist_angle(turn, axis).to_degrees();
    ramp((angle - start) / (full - start))
}

/// `t` clamped to 0 to 1 and eased at both ends (smoothstep): 0, 0.5 and 1
/// where a straight ramp is, with no change of slope at the ends.
pub fn ramp(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Leans `posed` (the skin of `model` in `pose`) on the model's corrective
/// shapes. The other shapes' weights (the face's) are kept. A model
/// without corrective shapes is left alone.
pub fn drive(model: &Model, rig: &Rig, pose: &Pose, posed: &mut Posed) {
    let found = Correctives::detect(model, rig);
    if found.is_empty() {
        return;
    }
    let mut weights = posed.morph_weights().to_vec();
    weights.resize(model.morph_targets.len(), 0.0);
    for (morph, value) in found.weights(rig, pose) {
        weights[morph] = value;
    }
    posed.set_morph_weights(&weights);
}

#[cfg(test)]
mod tests {
    use super::*;
    use axcenity_assets::{ModelFormat, Pick, load_models};
    use std::path::Path;

    const AXIS: Vec3 = Vec3::new(0.0, -0.37768, 0.92594);

    fn turned(rest: Quat, axis: Vec3, degrees: f32) -> Quat {
        Quat::from_axis_angle(axis.normalize(), degrees.to_radians()) * rest
    }

    fn shipped(object: &str) -> Model {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/characters/human/HumanParts.glb");
        let bytes = std::fs::read(file).unwrap();
        let picks = [Pick::Objects(vec![format!("AxcenityHuman/{object}")])];
        load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks)
            .unwrap()
            .remove(0)
    }

    #[test]
    fn a_shape_leans_in_as_its_bone_turns_about_the_axis() {
        let rest = Quat::from_rotation_z(0.4) * Quat::from_rotation_x(-0.2);
        let at = |degrees: f32| weight(rest, turned(rest, AXIS, degrees), AXIS, 45.0, 105.0);
        assert_eq!(at(0.0), 0.0);
        // (Floats: the axis is a rounded unit vector.)
        assert!(at(45.0) < 1e-4);
        assert!((at(75.0) - 0.5).abs() < 1e-4);
        assert!((at(105.0) - 1.0).abs() < 1e-4);
        // Past full it stays full; turned the other way it stays out.
        assert_eq!(at(140.0), 1.0);
        assert_eq!(at(-60.0), 0.0);
    }

    /// The ramp is level at both ends: a joint turning steadily through a
    /// shape's start or full angle doesn't start or stop the shape at
    /// speed (the motion check's pops), and it is half way at half way.
    #[test]
    fn the_ramp_eases_in_and_out() {
        assert_eq!(
            (ramp(-1.0), ramp(0.0), ramp(1.0), ramp(2.0)),
            (0.0, 0.0, 1.0, 1.0)
        );
        assert!((ramp(0.5) - 0.5).abs() < 1e-6);
        // The slope by each end is near none (a straight ramp's is 1).
        let slope = |t: f32| (ramp(t + 1e-3) - ramp(t)) / 1e-3;
        assert!(slope(0.0) < 0.01 && slope(1.0 - 1e-3) < 0.01);
        assert!((slope(0.5) - 1.5).abs() < 0.01);
    }

    #[test]
    fn a_turn_about_another_axis_doesnt_lean() {
        let rest = Quat::from_rotation_y(0.3);
        let other = AXIS.normalize().any_orthonormal_vector();
        let swung = turned(rest, other, 90.0);
        assert!(weight(rest, swung, AXIS, 0.0, 45.0) < 1e-4);
        // Swung and raised: only the raise counts.
        let both = Quat::from_axis_angle(other, 0.5) * turned(rest, AXIS, 22.5);
        assert!((weight(rest, both, AXIS, 0.0, 45.0) - 0.5).abs() < 0.02);
    }

    #[test]
    fn a_model_without_corrective_shapes_is_left_alone() {
        let model = shipped("Eyes");
        let rig = model.rig.clone().unwrap();
        assert!(Correctives::detect(&model, &rig).is_empty());
        let pose = rig.rest_pose();
        let mut posed = model.pose_shaped(&pose, &[]).unwrap();
        let version = posed.version;
        drive(&model, &rig, &pose, &mut posed);
        assert_eq!(posed.version, version);
    }
}
