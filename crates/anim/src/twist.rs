//! Twist and helper bones (Character Studio overhaul CO4; architecture
//! section 12): bones a skeleton has for its skin's sake, which no clip
//! keys. They're turned here, from the pose the clips, IK and physics
//! made, just before the skin is posed, so every clip plays unchanged.
//!
//! A skeleton is driven by its bones' names: `Left` or `Right`, then one
//! of the names below. A skeleton with none of them (the stylized
//! humanoid) is left alone.
//!
//! **Twist bones** lie inside a limb bone and spread a turn about the
//! limb's length along it, where two bones alone wind the skin between
//! them up like a sweet wrapper:
//!
//! | Bone | Inside | Reads | Share |
//! |---|---|---|---|
//! | `UpperArmTwist` | `UpperArm` | `UpperArm` | −0.5 |
//! | `LowerArmTwist` | `LowerArm` | `Hand` | 0.5 |
//! | `UpperLegTwist` | `UpperLeg` | `UpperLeg` | −0.5 |
//! | `LowerLegTwist` | `LowerLeg` | `Foot` | 0.5 |
//!
//! A forearm's twist bone takes half of the hand's twist about the
//! forearm (so the skin turns a little at the elbow, half way along the
//! forearm, and wholly at the wrist); a shin's does the same from the
//! foot. An upper arm's twist bone holds the half of the arm nearer the
//! shoulder and takes half of the arm's own twist back out (a negative
//! share), so the skin by the shoulder turns half as far as the arm; a
//! thigh's does the same by the hip. Only the twist is read: bending a
//! wrist or raising an arm turns no twist bone.
//!
//! **Helper bones** sit on a hinge and turn half as far as it does, so
//! skin shared between the joint's two bones goes round it on an arc and
//! keeps its width, where a plain blend of the two cuts the corner and
//! the joint collapses:
//!
//! | Bone | Parent | Reads |
//! |---|---|---|
//! | `ShoulderHelper` | `Shoulder` | `UpperArm` |
//! | `ElbowHelper` | `UpperArm` | `LowerArm` |
//! | `HipHelper` | `Hips` | `UpperLeg` |
//! | `KneeHelper` | `UpperLeg` | `LowerLeg` |
//!
//! A helper has the parent of the bone it reads and stays on that bone's
//! joint. The human base has the four twist bones a side
//! (`assets/characters/human/build.py` `TWISTS`) and no helpers; the
//! realistic base, retired by ADR-0050, was built and weighted for both.
//!
//! Body sliders ([`crate::BodyShape`]) scale a bone's own vertices; a
//! twist bone takes its limb's scale and a helper its joint's, so skin
//! weighted to them stretches with the limb.

use axcenity_assets::{Pose, Rig};
use glam::{Quat, Vec3};

/// The twist bones: (name after the side, the bone it's inside, the bone
/// whose twist it reads, the share of it it takes).
pub const TWISTS: [(&str, &str, &str, f32); 4] = [
    ("UpperArmTwist", "UpperArm", "UpperArm", -0.5),
    ("LowerArmTwist", "LowerArm", "Hand", 0.5),
    ("UpperLegTwist", "UpperLeg", "UpperLeg", -0.5),
    ("LowerLegTwist", "LowerLeg", "Foot", 0.5),
];

/// The helper bones: (name after the side, the bone whose turn it reads).
/// Its parent is that bone's.
pub const HELPERS: [(&str, &str); 4] = [
    ("ShoulderHelper", "UpperArm"),
    ("ElbowHelper", "LowerArm"),
    ("HipHelper", "UpperLeg"),
    ("KneeHelper", "LowerLeg"),
];

/// How far a helper turns with its joint.
pub const HELPER_SHARE: f32 = 0.5;

const SIDES: [&str; 2] = ["Left", "Right"];

/// One bone this module turns, found in a skeleton.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Driven {
    /// `bone` takes `share` of the twist of `reads` about the limb `inside`
    /// (its parent). `reads` is `inside` itself, or a child of it.
    Twist {
        bone: usize,
        inside: usize,
        reads: usize,
        share: f32,
    },
    /// `bone` takes [`HELPER_SHARE`] of the turn of `reads` (which has
    /// the same parent) and stays on its joint.
    Helper { bone: usize, reads: usize },
}

/// The bones of a skeleton this module turns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrivenBones(pub Vec<Driven>);

impl DrivenBones {
    /// Finds `rig`'s twist and helper bones by name. One whose limb or
    /// joint the skeleton lacks, or that hangs from another bone than the
    /// tables say, is left out (it stays where the pose has it).
    pub fn detect(rig: &Rig) -> Self {
        let mut found = Vec::new();
        for (bone, own) in rig.bones.iter().enumerate() {
            // Nearly every bone of nearly every skeleton stops here.
            if !(own.name.ends_with("Twist") || own.name.ends_with("Helper")) {
                continue;
            }
            let Some((side, rest)) = SIDES
                .iter()
                .find_map(|side| Some((*side, own.name.strip_prefix(side)?)))
            else {
                continue;
            };
            let named = |name: &str| rig.bone(&format!("{side}{name}"));
            if let Some(&(_, inside, reads, share)) = TWISTS.iter().find(|twist| twist.0 == rest) {
                let (Some(inside), Some(reads)) = (named(inside), named(reads)) else {
                    continue;
                };
                // Inside its limb, reading the limb or the next bone along.
                let along = reads == inside || rig.bones[reads].parent == Some(inside);
                if own.parent == Some(inside) && along {
                    found.push(Driven::Twist {
                        bone,
                        inside,
                        reads,
                        share,
                    });
                }
            } else if let Some(&(_, reads)) = HELPERS.iter().find(|helper| helper.0 == rest) {
                let Some(reads) = named(reads) else { continue };
                if own.parent.is_some() && own.parent == rig.bones[reads].parent {
                    found.push(Driven::Helper { bone, reads });
                }
            }
        }
        Self(found)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Turns the bones in `pose` (of `rig`, each bone relative to its
    /// parent) for the rest of the pose, replacing whatever they had. With
    /// body slider scales in `shape` (one per bone, or empty), gives each
    /// its limb's or joint's.
    pub fn apply(&self, rig: &Rig, pose: &mut Pose, shape: &mut [Vec3]) {
        if pose.len() < rig.bones.len() {
            pose.extend(rig.bones[pose.len()..].iter().map(|bone| bone.rest));
        }
        for &one in &self.0 {
            match one {
                Driven::Twist {
                    bone,
                    inside,
                    reads,
                    share,
                } => {
                    let rest = rig.bones[bone].rest;
                    // The limb's length: the way to the twist bone, which
                    // lies along it (a humanoid's bones point along +Y).
                    let axis = rest.translation.try_normalize().unwrap_or(Vec3::Y);
                    let read = rig.bones[reads].rest.rotation;
                    // The turn from rest, in the limb's own axes: its own
                    // (after its rest), or the next bone's (before its rest).
                    let turn = if reads == inside {
                        read.inverse() * pose[reads].rotation
                    } else {
                        pose[reads].rotation * read.inverse()
                    };
                    let angle = twist_angle(turn, axis) * share;
                    pose[bone].rotation = Quat::from_axis_angle(axis, angle) * rest.rotation;
                    if let (Some(&limb), true) = (shape.get(inside), bone < shape.len()) {
                        shape[bone] = limb;
                    }
                }
                Driven::Helper { bone, reads } => {
                    let (rest, read) = (rig.bones[bone].rest, rig.bones[reads].rest);
                    let turn = pose[reads].rotation * read.rotation.inverse();
                    let part = Quat::IDENTITY.slerp(turn, HELPER_SHARE);
                    pose[bone].rotation = part * rest.rotation;
                    // On the joint, wherever a slider or an offset put it.
                    pose[bone].translation =
                        pose[reads].translation + (rest.translation - read.translation);
                    if let (Some(&joint), true) = (shape.get(reads), bone < shape.len()) {
                        shape[bone] = joint;
                    }
                }
            }
        }
    }
}

/// Turns `rig`'s twist and helper bones in `pose` ([`DrivenBones`]). Call
/// it on a finished pose, after clips, IK and physics; it costs a look at
/// each bone's name on a skeleton that has none.
pub fn drive(rig: &Rig, pose: &mut Pose, shape: &mut [Vec3]) {
    let driven = DrivenBones::detect(rig);
    if !driven.is_empty() {
        driven.apply(rig, pose, shape);
    }
}

/// The angle, in radians (−π to π), of the part of `rotation` that turns
/// about `axis` (a unit vector): its twist, with the swing taken out.
/// `rotation` is the swing after the twist (`swing * twist`).
pub fn twist_angle(rotation: Quat, axis: Vec3) -> f32 {
    // A rotation's vector part along the axis and its scalar part are the
    // twist's sine and cosine of half the angle (times the same factor).
    let along = Vec3::new(rotation.x, rotation.y, rotation.z).dot(axis);
    let angle = 2.0 * along.atan2(rotation.w);
    // The short way round.
    if angle > std::f32::consts::PI {
        angle - std::f32::consts::TAU
    } else if angle < -std::f32::consts::PI {
        angle + std::f32::consts::TAU
    } else {
        angle
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use axcenity_assets::{Bone, BoneTransform};
    use glam::Mat4;

    use super::*;

    fn rig(bones: &[(&str, Option<usize>, Vec3)]) -> Rig {
        Rig {
            bones: bones
                .iter()
                .map(|&(name, parent, at)| Bone {
                    name: name.to_owned(),
                    parent,
                    rest: BoneTransform {
                        translation: at,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::ONE,
                    },
                })
                .collect(),
            joints: Vec::new(),
            fit: Mat4::IDENTITY,
        }
    }

    /// A left arm along +Y, as glTF humanoids point their bones: shoulder,
    /// upper arm (0.3 m), forearm (0.25 m), hand, with both twist bones
    /// and the shoulder's and the elbow's helpers.
    fn arm() -> Rig {
        rig(&[
            ("LeftShoulder", None, Vec3::ZERO),
            ("LeftUpperArm", Some(0), Vec3::new(0.0, 0.15, 0.0)),
            ("LeftLowerArm", Some(1), Vec3::new(0.0, 0.3, 0.0)),
            ("LeftHand", Some(2), Vec3::new(0.0, 0.25, 0.0)),
            ("LeftUpperArmTwist", Some(1), Vec3::new(0.0, 0.024, 0.0)),
            ("LeftLowerArmTwist", Some(2), Vec3::new(0.0, 0.125, 0.0)),
            ("LeftShoulderHelper", Some(0), Vec3::new(0.0, 0.15, 0.0)),
            ("LeftElbowHelper", Some(1), Vec3::new(0.0, 0.3, 0.0)),
        ])
    }

    const UPPER: usize = 1;
    const LOWER: usize = 2;
    const HAND: usize = 3;
    const UPPER_TWIST: usize = 4;
    const LOWER_TWIST: usize = 5;
    const SHOULDER_HELPER: usize = 6;
    const ELBOW_HELPER: usize = 7;

    fn close(a: Quat, b: Quat) -> bool {
        a.dot(b).abs() > 1.0 - 1e-5
    }

    fn driven(rig: &Rig, pose: &mut Pose) {
        drive(rig, pose, &mut []);
    }

    #[test]
    fn the_twist_of_a_turn_is_its_part_about_the_axis() {
        let twist = Quat::from_axis_angle(Vec3::Y, 0.7);
        let swing = Quat::from_axis_angle(Vec3::X, 1.1);
        assert!((twist_angle(twist, Vec3::Y) - 0.7).abs() < 1e-5);
        assert!((twist_angle(swing * twist, Vec3::Y) - 0.7).abs() < 1e-5);
        assert!(twist_angle(swing, Vec3::Y).abs() < 1e-5);
        // The short way round, and the same turn whichever way it's written.
        let far = Quat::from_axis_angle(Vec3::Y, PI + 0.4);
        assert!((twist_angle(far, Vec3::Y) - (0.4 - PI)).abs() < 1e-4);
        assert!((twist_angle(-twist, Vec3::Y) - 0.7).abs() < 1e-5);
    }

    #[test]
    fn a_forearms_twist_bone_takes_half_of_the_hands_twist() {
        let rig = arm();
        let mut pose = rig.rest_pose();
        pose[HAND].rotation = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2);
        driven(&rig, &mut pose);
        let half = Quat::from_axis_angle(Vec3::Y, FRAC_PI_2 / 2.0);
        assert!(close(pose[LOWER_TWIST].rotation, half));
        // The other way too.
        pose[HAND].rotation = Quat::from_axis_angle(Vec3::Y, -1.2);
        driven(&rig, &mut pose);
        assert!(close(
            pose[LOWER_TWIST].rotation,
            Quat::from_axis_angle(Vec3::Y, -0.6)
        ));
        // Nothing else moved.
        assert_eq!(pose[LOWER_TWIST].translation, Vec3::new(0.0, 0.125, 0.0));
        assert!(close(pose[UPPER_TWIST].rotation, Quat::IDENTITY));
        assert_eq!(pose[LOWER], rig.bones[LOWER].rest);
    }

    #[test]
    fn bending_a_wrist_or_raising_an_arm_turns_no_twist_bone() {
        let rig = arm();
        let mut pose = rig.rest_pose();
        pose[HAND].rotation = Quat::from_axis_angle(Vec3::X, 1.2);
        pose[UPPER].rotation = Quat::from_axis_angle(Vec3::Z, -1.8);
        driven(&rig, &mut pose);
        assert!(close(pose[LOWER_TWIST].rotation, Quat::IDENTITY));
        assert!(close(pose[UPPER_TWIST].rotation, Quat::IDENTITY));
        // A bent and twisted wrist gives only its twist.
        let bend = Quat::from_axis_angle(Vec3::X, 0.9);
        pose[HAND].rotation = bend * Quat::from_axis_angle(Vec3::Y, 1.0);
        driven(&rig, &mut pose);
        assert!(close(
            pose[LOWER_TWIST].rotation,
            Quat::from_axis_angle(Vec3::Y, 0.5)
        ));
    }

    #[test]
    fn an_upper_arms_twist_bone_takes_half_of_the_arms_own_twist_back_out() {
        let rig = arm();
        let mut pose = rig.rest_pose();
        let swing = Quat::from_axis_angle(Vec3::Z, -1.0);
        pose[UPPER].rotation = swing * Quat::from_axis_angle(Vec3::Y, 1.4);
        driven(&rig, &mut pose);
        assert!(close(
            pose[UPPER_TWIST].rotation,
            Quat::from_axis_angle(Vec3::Y, -0.7)
        ));
        // In the shoulder's space the twist bone has the arm's swing and
        // half its twist: the skin by the shoulder turns half as far.
        let world = pose[UPPER].rotation * pose[UPPER_TWIST].rotation;
        assert!(close(world, swing * Quat::from_axis_angle(Vec3::Y, 0.7)));
    }

    #[test]
    fn a_helper_turns_half_as_far_as_its_joint_and_stays_on_it() {
        let rig = arm();
        let mut pose = rig.rest_pose();
        let bend = 135.0_f32.to_radians();
        pose[LOWER].rotation = Quat::from_axis_angle(Vec3::X, bend);
        // A body slider moved the elbow out.
        pose[LOWER].translation = Vec3::new(0.0, 0.33, 0.0);
        pose[UPPER].rotation = Quat::from_axis_angle(Vec3::Z, -1.2);
        driven(&rig, &mut pose);
        assert!(close(
            pose[ELBOW_HELPER].rotation,
            Quat::from_axis_angle(Vec3::X, bend / 2.0)
        ));
        assert_eq!(pose[ELBOW_HELPER].translation, pose[LOWER].translation);
        assert!(close(
            pose[SHOULDER_HELPER].rotation,
            Quat::from_axis_angle(Vec3::Z, -0.6)
        ));
        assert_eq!(pose[SHOULDER_HELPER].translation, pose[UPPER].translation);
    }

    #[test]
    fn what_a_clip_keyed_on_a_driven_bone_is_replaced() {
        let rig = arm();
        let mut pose = rig.rest_pose();
        pose[LOWER_TWIST].rotation = Quat::from_axis_angle(Vec3::X, 1.0);
        pose[ELBOW_HELPER].rotation = Quat::from_axis_angle(Vec3::Z, 2.0);
        driven(&rig, &mut pose);
        assert!(close(pose[LOWER_TWIST].rotation, Quat::IDENTITY));
        assert!(close(pose[ELBOW_HELPER].rotation, Quat::IDENTITY));
    }

    #[test]
    fn a_rest_turned_twist_bone_keeps_its_rest_under_the_twist() {
        // A twist bone that isn't lined up with its limb at rest (an
        // imported skeleton): the twist goes over its rest turn, about
        // the limb's length (the way to the twist bone).
        let mut rig = arm();
        let rest = Quat::from_axis_angle(Vec3::Z, 0.3);
        rig.bones[LOWER_TWIST].rest.rotation = rest;
        let mut pose = rig.rest_pose();
        pose[HAND].rotation = Quat::from_axis_angle(Vec3::Y, 1.0);
        driven(&rig, &mut pose);
        assert!(close(
            pose[LOWER_TWIST].rotation,
            Quat::from_axis_angle(Vec3::Y, 0.5) * rest
        ));
    }

    #[test]
    fn a_skeleton_without_such_bones_is_left_alone() {
        let rig = rig(&[
            ("LeftUpperArm", None, Vec3::ZERO),
            ("LeftLowerArm", Some(0), Vec3::new(0.0, 0.3, 0.0)),
            ("LeftHand", Some(1), Vec3::new(0.0, 0.25, 0.0)),
            // Named like one, but not hanging where the tables say.
            ("LeftLowerArmTwist", Some(0), Vec3::new(0.0, 0.1, 0.0)),
            ("RightKneeHelper", Some(0), Vec3::ZERO),
        ]);
        assert!(DrivenBones::detect(&rig).is_empty());
        let mut pose = rig.rest_pose();
        pose[2].rotation = Quat::from_axis_angle(Vec3::Y, 1.0);
        let before = pose.clone();
        driven(&rig, &mut pose);
        assert_eq!(pose, before);
    }

    #[test]
    fn driven_bones_take_their_limbs_slider_scale() {
        let rig = arm();
        let mut pose = rig.rest_pose();
        let mut shape = vec![Vec3::ONE; rig.bones.len()];
        shape[UPPER] = Vec3::new(1.0, 1.2, 1.0);
        shape[LOWER] = Vec3::new(1.0, 1.1, 1.0);
        drive(&rig, &mut pose, &mut shape);
        assert_eq!(shape[UPPER_TWIST], shape[UPPER]);
        assert_eq!(shape[LOWER_TWIST], shape[LOWER]);
        assert_eq!(shape[ELBOW_HELPER], shape[LOWER]);
        assert_eq!(shape[SHOULDER_HELPER], shape[UPPER]);
    }

    #[test]
    fn a_short_pose_is_filled_in_before_its_bones_are_turned() {
        let rig = arm();
        let mut pose: Pose = rig.rest_pose()[..=HAND].to_vec();
        pose[HAND].rotation = Quat::from_axis_angle(Vec3::Y, 1.0);
        driven(&rig, &mut pose);
        assert_eq!(pose.len(), rig.bones.len());
        assert!(close(
            pose[LOWER_TWIST].rotation,
            Quat::from_axis_angle(Vec3::Y, 0.5)
        ));
    }

    /// The pose path turns the shipped body's twist bones before it poses
    /// the skin, in the pose its parts then follow.
    #[test]
    fn the_pose_path_drives_the_shipped_body_before_it_is_skinned() {
        use axcenity_assets::{ModelFormat, Pick, load_models};
        use std::path::Path;

        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/characters/human/HumanParts.glb");
        let bytes = std::fs::read(file).unwrap();
        let picks = [Pick::Objects(vec!["AxcenityHuman/Body".to_owned()])];
        let models = load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks).unwrap();
        let model = &models[0];
        let rig = model.rig.clone().unwrap();
        let bone = |name: &str| rig.bone(name).unwrap();
        let hand = bone("LeftHand");
        let mut pose = rig.rest_pose();
        // The hand turned a quarter about the forearm.
        let forearm = rig.bones[hand].rest.translation.normalize();
        pose[hand].rotation = Quat::from_axis_angle(forearm, FRAC_PI_2) * pose[hand].rotation;
        let at_rest = model.pose_shaped(&pose, &[]).unwrap();
        let posed = crate::animators::skinned_for_test(model, &rig, &mut pose, &mut []).unwrap();
        let twist = bone("LeftLowerArmTwist");
        let turned = pose[twist].rotation * rig.bones[twist].rest.rotation.inverse();
        assert!((twist_angle(turned, forearm) - FRAC_PI_2 / 2.0).abs() < 1e-3);
        // And the skin was posed with them: those bones' joints moved.
        let moved = |index: usize| {
            let joint = rig
                .joints
                .iter()
                .position(|joint| joint.bone == index)
                .unwrap();
            !posed.joints()[joint].abs_diff_eq(at_rest.joints()[joint], 1e-5)
        };
        assert!(moved(twist));
    }

    /// The human base's skeleton, as shipped, has its eight twist bones
    /// found, each lying along its limb and not turned against it at rest
    /// (`assets/characters/human/build.py` `TWISTS`).
    #[test]
    fn the_shipped_skeletons_twist_bones_are_found_along_their_limbs() {
        use axcenity_assets::{ModelFormat, Pick, load_models};
        use std::path::Path;

        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/characters/human/HumanParts.glb");
        let bytes = std::fs::read(file).unwrap();
        let picks = [Pick::Objects(vec!["AxcenityHuman/Body".to_owned()])];
        let models = load_models(&bytes, Path::new("."), ModelFormat::Gltf, &picks).unwrap();
        let rig = models[0].rig.clone().unwrap();
        let driven = DrivenBones::detect(&rig);
        assert_eq!(driven.0.len(), 8);
        for one in &driven.0 {
            let Driven::Twist { bone, inside, .. } = *one else {
                panic!("the human base has no helper bones");
            };
            let name = rig.bones[bone].name.as_str();
            assert_eq!(rig.bones[bone].parent, Some(inside), "{name}");
            let at = rig.bones[bone].rest.translation;
            assert!(
                at.x.abs() < 1e-4 && at.z.abs() < 1e-4 && at.y > 0.0,
                "{name}: {at:?}"
            );
            assert!(
                close(rig.bones[bone].rest.rotation, Quat::IDENTITY),
                "{name}"
            );
        }
    }
}
