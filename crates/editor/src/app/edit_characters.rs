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
//!
//! Two things keep that showing honest under the Game Editor's own edits:
//!
//! - **Undo, Revert and other whole-world steps.** A recorded step can put
//!   back a character as it was before edit mode showed it (its base body,
//!   no parts) with the same Instance IDs and the same `Base` and `Data`,
//!   which the sync alone would count as already applied. So what each
//!   character showed after the last step (its body, the body's model and
//!   identity, its parts) is kept, and a character whose showing changed
//!   is forgotten by the sync and applied again.
//! - **Prefab copies.** A copy's overrides are what differs from its
//!   prefab. A prefab file keeps a described character unapplied (the
//!   project crate writes the default `Character` prefab without applying
//!   it; Play applies it where it spawns), so what edit mode derives in a
//!   copy (the body's model, size and identity, its parts and their
//!   placement) would show as the person's overrides. So the source a
//!   copy is compared with is shown the same way: applied and posed at
//!   rest in memory (never written to the file) and set on the copy's
//!   link, unrecorded. A copy nobody changed then has no overrides, Revert
//!   all puts back the shown prefab, and saving writes only what the
//!   person changed; opening the project shows it again.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use axcenity_anim::{Animators, Poses};
use axcenity_assets::{Assets, Slot};
use axcenity_core::prefab::PrefabSource;
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
    /// What each kept description's character showed after the last step,
    /// by the description (see the module documentation).
    shown: HashMap<InstanceId, Showing>,
    /// Prefab sources shown as edit mode shows their copies: each as read
    /// from its file, and shown. `Arc`s, compared by address: a copy's
    /// link holds one or the other.
    sources: Vec<(Arc<PrefabSource>, Arc<PrefabSource>)>,
}

/// What applying a description put on a character: its body, the body's
/// model and identity, and the parts it wears.
#[derive(PartialEq)]
struct Showing {
    body: Option<InstanceId>,
    mesh: Option<Value>,
    identity: Option<Value>,
    parts: Vec<InstanceId>,
}

impl EditCharacters {
    /// Applies what isn't applied yet and poses the described characters.
    /// Returns what couldn't be applied, each said once.
    pub(crate) fn step(&mut self, world: &mut World, assets: &Assets) -> Vec<String> {
        // Applying doesn't add or take away kept descriptions, so the
        // characters found now are the ones after it.
        let characters = described_characters(world);
        // A character whose showing a recorded step took away (or changed)
        // since the last step is applied again.
        for &character in &characters {
            if let Some(description) = applier::applied_instance(world, character)
                && let Some(was) = self.shown.get(&description)
                && *was != showing(world, character)
            {
                self.sync.forget(description);
            }
        }
        // What edit mode puts on is the editor's own showing of a kept
        // description, which every computer makes for itself: Collaborate
        // doesn't send it (`World::unrecorded`).
        let sync = &mut self.sync;
        let errors = world.unrecorded(|world| sync.step(world, assets));
        self.shown = characters
            .iter()
            .filter_map(|&character| {
                let description = applier::applied_instance(world, character)?;
                Some((description, showing(world, character)))
            })
            .collect();
        if characters.is_empty() {
            self.poses.clear();
            self.made_from = 0;
            self.complete = true;
            self.sources.clear();
            return errors;
        }
        self.show_sources(world, assets, &characters);
        if self.complete && stamp(world, &characters) == self.made_from {
            return errors;
        }
        let (poses, complete) = pose_at_rest(world, assets, &characters);
        self.poses = poses;
        self.complete = complete;
        // Taken after the parts were placed, so placing them doesn't
        // count as a change next frame.
        self.made_from = stamp(world, &characters);
        errors
    }

    /// The edit-mode poses, by Instance, for the viewport.
    pub(crate) fn poses(&self) -> &Poses {
        &self.poses
    }

    /// Sets the link of each prefab copy that holds (or is) one of
    /// `characters` to its source as edit mode shows it (see the module
    /// documentation), made once per source. A source whose models are
    /// still loading is tried again next step.
    fn show_sources(&mut self, world: &mut World, assets: &Assets, characters: &[InstanceId]) {
        let mut roots = Vec::new();
        for &character in characters {
            let mut at = Some(character);
            while let Some(id) = at {
                let instance = world.get(id);
                if instance.is_some_and(|i| i.prefab().is_some()) && !roots.contains(&id) {
                    roots.push(id);
                }
                at = instance.and_then(|i| i.parent());
            }
        }
        for root in roots {
            let Some(mut link) = world.get(root).and_then(|i| i.prefab()).cloned() else {
                continue;
            };
            let Some(source) = link.source.clone() else {
                continue;
            };
            if self
                .sources
                .iter()
                .any(|(_, shown)| Arc::ptr_eq(shown, &source))
            {
                continue;
            }
            let shown = match self
                .sources
                .iter()
                .find(|(read, _)| Arc::ptr_eq(read, &source))
            {
                Some((_, shown)) => Arc::clone(shown),
                None => {
                    let Some(shown) = shown_source(&source, assets) else {
                        continue;
                    };
                    self.sources.push((source, Arc::clone(&shown)));
                    shown
                }
            };
            link.source = Some(shown);
            // A link isn't a property: setting it isn't recorded, sent or
            // saved (a copy's file keeps its prefab's path and overrides).
            let _ = world.set_prefab(root, Some(link));
        }
        // Kept while a copy (or an undo step) still holds the shown one.
        self.sources
            .retain(|(_, shown)| Arc::strong_count(shown) > 1);
    }
}

/// What applying a description put on `character` (see [`Showing`]).
fn showing(world: &World, character: InstanceId) -> Showing {
    let body = applier::body_of(world, character);
    let held = body.and_then(|body| world.get(body));
    Showing {
        body,
        mesh: held.and_then(|b| b.property("MeshId")).cloned(),
        identity: held
            .and_then(|b| b.attribute(axcenity_anim::IDENTITY))
            .cloned(),
        parts: held
            .map(|b| b.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .filter(|id| {
                world
                    .get(*id)
                    .is_some_and(|part| part.attribute(applier::DESCRIPTION_PART).is_some())
            })
            .collect(),
    }
}

/// `source` as edit mode shows a copy of it: its described characters
/// applied and posed at rest, in memory. `None` while a model it needs is
/// still loading.
fn shown_source(source: &Arc<PrefabSource>, assets: &Assets) -> Option<Arc<PrefabSource>> {
    let characters = described_characters(&source.world);
    if characters.is_empty() {
        return Some(Arc::clone(source));
    }
    let mut shown = (**source).clone();
    // Errors are the copy's to say (it's applied from the same description).
    let _ = DescriptionSync::default().step(&mut shown.world, assets);
    let (_, complete) = pose_at_rest(&mut shown.world, assets, &characters);
    complete.then(|| Arc::new(shown))
}

/// Poses `characters` at rest in a World of their own and takes the
/// placement of the parts that follow each body back to `world`'s
/// Instances. Returns the poses, by `world`'s Instances, and whether every
/// body was posed (or its model can't be read).
fn pose_at_rest(world: &mut World, assets: &Assets, characters: &[InstanceId]) -> (Poses, bool) {
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
    let poses: Poses = animators
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
    let complete = characters.iter().all(|&character| {
        let Some(body) = applier::body_of(world, character) else {
            return true;
        };
        let mesh = world
            .get(body)
            .and_then(|part| part.property("MeshId"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        poses.contains_key(&body) || matches!(assets.model_slot(mesh), Slot::Failed)
    });
    (poses, complete)
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
