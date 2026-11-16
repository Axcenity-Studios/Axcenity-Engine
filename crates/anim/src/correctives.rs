//! Pose-driven corrective shapes (Character Studio overhaul, CO5): blend
//! shapes a body leans on as a bone turns, which put back what skinning
//! loses at a joint. The realistic base body has four, for its shoulders
//! (`assets/characters/realistic/correctives.py` makes them and states the
//! rule in `Realistic.build.json`).
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

/// The realistic body's corrective shapes: the shoulder's cap, held as the
/// arm is raised to level (45° up from its rest, which is 45° below
/// level) and on to 150° from hanging, and as it is lowered to hang (35°
/// down from rest: about the raise axis reversed, so the rule's form is
/// the same).
pub const CORRECTIVES: [Rule; 6] = [
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
pub fn weight(rest: Quat, posed: Quat, axis: Vec3, start: f32, full: f32) -> f32 {
    // The bone's turn from rest, in its parent's axes.
    let turn = posed * rest.inverse();
    let angle = twist_angle(turn, axis).to_degrees();
    ((angle - start) / (full - start)).clamp(0.0, 1.0)
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
            .join("../../assets/characters/realistic/RealisticParts.glb");
        let bytes = std::fs::read(file).unwrap();
        let picks = [Pick::Objects(vec![format!("AxcenityRealistic/{object}")])];
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

    /// The constants are the rules the realistic body's build wrote beside
    /// its shapes.
    #[test]
    fn the_rules_are_the_ones_the_build_states() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/characters/realistic/Realistic.build.json");
        let data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let built = data["correctives"].as_array().unwrap();
        assert_eq!(built.len(), CORRECTIVES.len());
        for rule in built {
            let shape = rule["shape"].as_str().unwrap();
            let ours = CORRECTIVES
                .iter()
                .find(|ours| ours.0 == shape)
                .unwrap_or_else(|| panic!("no rule for {shape}"));
            assert!(shape.starts_with(PREFIX));
            assert_eq!(ours.1, rule["bone"].as_str().unwrap(), "{shape}");
            let number = |value: &serde_json::Value| value.as_f64().unwrap() as f32;
            for k in 0..3 {
                assert!(
                    (ours.2[k] - number(&rule["axis"][k])).abs() < 1e-4,
                    "{shape}"
                );
            }
            assert_eq!(ours.3, number(&rule["angles"][0]), "{shape}");
            assert_eq!(ours.4, number(&rule["angles"][1]), "{shape}");
        }
    }

    /// Every rule is found on the shipped body.
    #[test]
    fn the_shipped_body_has_every_corrective() {
        let model = shipped("Body");
        let rig = model.rig.clone().unwrap();
        let found = Correctives::detect(&model, &rig);
        assert_eq!(found.0.len(), CORRECTIVES.len());
        for rule in &found.0 {
            let name = &model.morph_targets[rule.morph].name;
            assert!(name.starts_with(PREFIX), "{name}");
            assert!(rig.bones[rule.bone].name.ends_with("UpperArm"));
        }
    }

    /// An arm lowered from the A-pose to hang, as the starter Idle holds
    /// it, leans on its side's Lowered shape and on none of the raising
    /// ones; half way down, half; and the skin moves with it.
    #[test]
    fn a_lowered_arm_leans_on_its_lowered_corrective_only() {
        let model = shipped("Body");
        let rig = model.rig.clone().unwrap();
        let arm = rig.bone("LeftUpperArm").unwrap();
        let rest = rig.bones[arm].rest.rotation;
        let lowered_axis = -AXIS;
        let weights_at = |degrees: f32| {
            let mut pose = rig.rest_pose();
            pose[arm].rotation = turned(rest, lowered_axis, degrees);
            let posed =
                crate::animators::skinned_for_test(&model, &rig, &mut pose, &mut []).unwrap();
            let plain = model.pose_shaped(&pose, &[]).unwrap();
            let of = |name: &str| {
                let index = model
                    .morph_targets
                    .iter()
                    .position(|target| target.name == name)
                    .unwrap();
                posed.morph_weights()[index]
            };
            let (a, b) = (posed.primitive(&model, 0, 0), plain.primitive(&model, 0, 0));
            let moved = a
                .positions
                .iter()
                .zip(&b.positions)
                .map(|(p, q)| Vec3::from(*p).distance(Vec3::from(*q)))
                .fold(0.0_f32, f32::max);
            (
                of("Corrective_Shoulder_Lowered_Left"),
                of("Corrective_Shoulder_Lowered_Right"),
                of("Corrective_Shoulder_Level_Left"),
                of("Corrective_Shoulder_Raised_Left"),
                moved,
            )
        };
        let (lowered, right, level, raised, moved) = weights_at(35.0);
        assert!((lowered - 1.0).abs() < 1e-3, "{lowered}");
        assert_eq!((right, level, raised), (0.0, 0.0, 0.0));
        assert!(moved > 0.005, "the cap moves {moved} m");
        assert!((weights_at(17.5).0 - 0.5).abs() < 0.02);
        assert_eq!(weights_at(0.0).0, 0.0);
    }

    /// The pose path leans the shipped body on its shapes: the left arm
    /// raised overhead brings both of the left shoulder's in and neither
    /// of the right's, and the skin moves with them, on the body and on
    /// both of its simpler copies (levels of detail 1 and 2).
    #[test]
    fn the_pose_path_leans_the_shipped_body_on_its_correctives() {
        let model = shipped("Body");
        let rig = model.rig.clone().unwrap();
        let arm = rig.bone("LeftUpperArm").unwrap();
        let mut pose = rig.rest_pose();
        pose[arm].rotation = turned(rig.bones[arm].rest.rotation, AXIS, 105.0);
        let plain = model.pose_shaped(&pose, &[]).unwrap();
        let posed = crate::animators::skinned_for_test(&model, &rig, &mut pose, &mut []).unwrap();
        let weight_of = |name: &str| {
            let index = model
                .morph_targets
                .iter()
                .position(|target| target.name == name)
                .unwrap();
            posed.morph_weights()[index]
        };
        assert!((weight_of("Corrective_Shoulder_Level_Left") - 1.0).abs() < 1e-3);
        assert!((weight_of("Corrective_Shoulder_Raised_Left") - 1.0).abs() < 1e-3);
        assert_eq!(weight_of("Corrective_Shoulder_Level_Right"), 0.0);
        assert_eq!(weight_of("Corrective_Shoulder_Raised_Right"), 0.0);
        assert_eq!(weight_of("Corrective_Shoulder_Lowered_Left"), 0.0);
        // Nothing else was leaned on.
        let others: f32 = model
            .morph_targets
            .iter()
            .zip(posed.morph_weights())
            .filter(|(target, _)| !target.name.starts_with(PREFIX))
            .map(|(_, weight)| weight.abs())
            .sum();
        assert_eq!(others, 0.0);
        assert_eq!(model.primitives[0].lods.len(), 2);
        for level in 0..3 {
            let (a, b) = (
                posed.primitive(&model, 0, level),
                plain.primitive(&model, 0, level),
            );
            let far = a
                .positions
                .iter()
                .zip(&b.positions)
                .map(|(p, q)| Vec3::from(*p).distance(Vec3::from(*q)))
                .fold(0.0_f32, f32::max);
            assert!(far > 0.01, "level {level}: the shoulder's skin moved {far}");
        }
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
