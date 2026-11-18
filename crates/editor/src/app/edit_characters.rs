//! Described characters in edit mode (refinement R10, OPEN-21): a
//! character whose description is kept in it (a HumanoidDescription in
//! its Humanoid) shows that description in the Game Editor's viewport, its
//! identity, items and materials, not its base meshes until Play.
//!
//! No game system runs. Each edit frame:
//!
//! 1. the Character Runtime's [`DescriptionSync`] applies each kept
//!    description that isn't applied in this run yet (a world just opened)
//!    or has changed (an undo, the Properties panel): it registers the
//!    identity in the editor's `Assets` and puts on the parts. For a world
//!    saved after its description was applied this changes nothing in the
//!    world, so nothing becomes unsaved. Like the parts' placement below
//!    it's done unrecorded, so Collaborate doesn't send it: each computer
//!    shows a kept description itself;
//! 2. the described characters are posed **at rest** (the A-pose, as the
//!    studio's Still), with their identity worn live (ADR-0041): a copy of
//!    each in a World of its own, its Animators emptied so nothing plays,
//!    is posed by `anim`'s Animators, and the poses are drawn on the
//!    editor's own Instances. The parts that follow the body are placed on
//!    it as Play would place them. Posing runs again only when one of
//!    those characters changes (or a model is still loading).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use axcenity_anim::{Animators, Poses};
use axcenity_assets::{Assets, Slot};
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_gameplay::description::{self as applier, APPLIED_DESCRIPTION, DescriptionSync};

/// See the module documentation. One per open project (its `Assets`).
#[derive(Default)]
pub(crate) struct EditCharacters {
    sync: DescriptionSync,
    /// The edit-mode poses, by the editor's Instances.
    poses: Poses,
    /// What [`Self::poses`] were made from: a hash of each described
    /// character's Instances and their revisions.
    made_from: u64,
    /// Whether every described body was posed (a model still loading
    /// isn't yet, so posing runs again next frame).
    complete: bool,
}

impl EditCharacters {
    /// Applies what isn't applied yet and poses the described characters.
    /// Returns what couldn't be applied, each said once.
    pub(crate) fn step(&mut self, world: &mut World, assets: &Assets) -> Vec<String> {
        // What edit mode puts on is the editor's own showing of a kept
        // description, which every computer makes for itself: Collaborate
        // doesn't send it (`World::unrecorded`).
        let sync = &mut self.sync;
        let errors = world.unrecorded(|world| sync.step(world, assets));
        let characters = described_characters(world);
        if characters.is_empty() {
            self.poses.clear();
            self.made_from = 0;
            self.complete = true;
            return errors;
        }
        if self.complete && stamp(world, &characters) == self.made_from {
            return errors;
        }
        self.pose(world, assets, &characters);
        // Taken after the parts were placed, so placing them doesn't
        // count as a change next frame.
        self.made_from = stamp(world, &characters);
        errors
    }

    /// The edit-mode poses, by Instance, for the viewport.
    pub(crate) fn poses(&self) -> &Poses {
        &self.poses
    }

    /// Poses `characters` at rest in a World of their own and takes the
    /// poses (and the placement of the parts that follow each body) back
    /// to `world`'s Instances.
    fn pose(&mut self, world: &mut World, assets: &Assets, characters: &[InstanceId]) {
        let mut shadow = World::new();
        let shadow_workspace = shadow.workspace();
        // The shadow's Instance → the editor's.
        let mut back: HashMap<InstanceId, InstanceId> = HashMap::new();
        for &character in characters {
            let Ok(copy) = shadow.copy_from(world, character) else {
                continue;
            };
            let _ = shadow.set_parent(copy, Some(shadow_workspace));
            pair(world, character, &shadow, copy, &mut back);
        }
        // Nothing plays in edit mode: an Animator with no state machine
        // poses its body at rest, and nothing else animates.
        let animated = shadow.all_below(shadow_workspace, |class| {
            matches!(
                class,
                ClassName::Animator
                    | ClassName::Animation
                    | ClassName::IKControl
                    | ClassName::Ragdoll
                    | ClassName::FaceControls
            )
        });
        for id in animated {
            if shadow.get(id).map(|i| i.class()) == Some(ClassName::Animator) {
                let _ = shadow.set_property(id, "StateMachine", Value::String(String::new()));
            } else {
                let _ = shadow.destroy(id);
            }
        }
        // A fresh Animators each time: the shadow's IDs are new.
        let mut animators = Animators::new();
        animators.update(&mut shadow, assets, 0.0);
        self.poses = animators
            .poses()
            .iter()
            .filter_map(|(id, posed)| back.get(id).map(|own| (*own, posed.clone())))
            .collect();
        // The parts that follow a body, placed on it as Play places them.
        for (&copy, &own) in &back {
            let follows = shadow.get(copy).is_some_and(|part| {
                part.class() == ClassName::MeshPart
                    && part
                        .parent()
                        .and_then(|parent| shadow.get(parent))
                        .is_some_and(|parent| parent.class() == ClassName::MeshPart)
            });
            if !follows {
                continue;
            }
            for name in ["Size", "Position", "Orientation"] {
                let wanted = shadow
                    .get(copy)
                    .and_then(|part| part.property(name))
                    .cloned();
                let now = world.get(own).and_then(|part| part.property(name));
                if let Some(wanted) = wanted
                    && now != Some(&wanted)
                {
                    world.unrecorded(|world| {
                        let _ = world.set_property(own, name, wanted);
                    });
                }
            }
        }
        // Done when every body is posed, or its model can't be read.
        self.complete = characters.iter().all(|&character| {
            let Some(body) = applier::body_of(world, character) else {
                return true;
            };
            let mesh = world
                .get(body)
                .and_then(|part| part.property("MeshId"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            self.poses.contains_key(&body) || matches!(assets.model_slot(mesh), Slot::Failed)
        });
    }
}

/// Every character in the Workspace that keeps a description (as
/// `DescriptionSync` finds them), each once, in tree order.
fn described_characters(world: &World) -> Vec<InstanceId> {
    let mut found = Vec::new();
    let kept = world.all_below(world.workspace(), |class| {
        class == ClassName::HumanoidDescription
    });
    for description in kept {
        let Some(holder) = world.get(description).and_then(|held| held.parent()) else {
            continue;
        };
        let Some(holder_instance) = world.get(holder) else {
            continue;
        };
        let character = if holder_instance.class() == ClassName::Humanoid {
            holder_instance.parent()
        } else if world.get(description).map(|held| held.name()) == Some(APPLIED_DESCRIPTION) {
            Some(holder)
        } else {
            None
        };
        if let Some(character) = character
            && !found.contains(&character)
        {
            found.push(character);
        }
    }
    found
}

/// A hash of `characters` and everything in them, with each Instance's
/// revision: the same number means nothing in them changed.
fn stamp(world: &World, characters: &[InstanceId]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for &character in characters {
        for id in std::iter::once(character).chain(world.descendants(character)) {
            id.hash(&mut hasher);
            world.get(id).map(|i| i.revision()).hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Notes that `copy` in `shadow` is `own` in `world`, and so on down
/// their trees. `World::copy_from` keeps the children's order and leaves
/// out scripts without a file (and what's in them), so they're skipped
/// here too.
fn pair(
    world: &World,
    own: InstanceId,
    shadow: &World,
    copy: InstanceId,
    back: &mut HashMap<InstanceId, InstanceId>,
) {
    back.insert(copy, own);
    let kept: Vec<InstanceId> = world
        .get(own)
        .map(|instance| instance.children().to_vec())
        .unwrap_or_default()
        .into_iter()
        .filter(|child| {
            world
                .get(*child)
                .is_some_and(|child| !child.class().is_script() || child.source_file().is_some())
        })
        .collect();
    let copies = shadow
        .get(copy)
        .map(|instance| instance.children().to_vec())
        .unwrap_or_default();
    for (own, copy) in kept.into_iter().zip(copies) {
        pair(world, own, shadow, copy, back);
    }
}
