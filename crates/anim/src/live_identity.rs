//! A character's identity worn live (Character Studio refinement R2a,
//! ADR-0041): a MeshPart draws its base model's own mesh, and the
//! identity a host registered in `Assets`
//! ([`Assets::register_identity`](axcenity_assets::Assets::register_identity))
//! is put on each of its poses: the identity's bone changes after the clips
//! (in both posing paths, as `Model::settle` was), its shape weights and
//! joint corrections last (`LiveIdentity::dress`). The MeshPart names the
//! identity by its key, in the attribute [`IDENTITY`].
//!
//! How a model wears an identity is worked out once per model and identity
//! ([`axcenity_assets::Model::live`]) and kept here, so a frame costs a
//! lookup per part; a new identity (a slider dragged) costs one
//! `Model::live` per part, with no vertex copied.

use std::collections::HashMap;
use std::sync::Arc;

use axcenity_assets::{Assets, LiveIdentity, Model};
use axcenity_core::{InstanceId, Value, World};

/// The attribute on a MeshPart naming the identity it wears live: the
/// identity's key ([`axcenity_assets::Identity::key`]) as 16 hex digits.
/// Written by the Character Runtime's applier; a key this run hasn't
/// registered (a saved world before its description is applied again)
/// wears nothing.
pub const IDENTITY: &str = "Identity";

/// The identity key `part` names in its [`IDENTITY`] attribute, if any.
pub fn identity_key(world: &World, part: InstanceId) -> Option<u64> {
    let text = world.get(part)?.attribute(IDENTITY)?.as_str()?;
    u64::from_str_radix(text, 16).ok()
}

/// Sets `part`'s [`IDENTITY`] attribute to `key` (`None` removes it),
/// changing nothing when it's already so.
pub fn set_identity_key(world: &mut World, part: InstanceId, key: Option<u64>) {
    let value = key.map(|key| Value::String(format!("{key:016x}")));
    let now = world.get(part).and_then(|held| held.attribute(IDENTITY));
    if now != value.as_ref() {
        let _ = world.set_attribute(part, IDENTITY, value);
    }
}

/// How each model wears each identity, by the model's key and the
/// identity's.
#[derive(Default)]
pub(crate) struct LiveIdentities {
    /// Each one made, with the frame it was last asked for.
    worn: HashMap<(u64, u64), (Arc<LiveIdentity>, u64)>,
    /// Counts `Animators::update`s ([`Self::next_frame`]).
    frame: u64,
    /// When `worn` reaches this many, the ones not asked for this frame or
    /// the last are let go (0 means [`KEPT`]).
    limit: usize,
    /// How many `LiveIdentity`s have been made (cache misses), for tests.
    made: usize,
}

/// How many are kept before the ones no part has worn for two frames are
/// let go (a drag makes one per part per step; each is a few kilobytes).
/// A crowd wearing more than this keeps them all: the limit grows to twice
/// what's in use, so letting go stays rare and never drops one in use.
const KEPT: usize = 1024;

impl LiveIdentities {
    /// How `model` wears the identity `part` names, or `None` when it
    /// names none this run knows, or one that changes nothing on `model`.
    pub(crate) fn of(
        &mut self,
        world: &World,
        assets: &Assets,
        part: InstanceId,
        model: &Model,
    ) -> Option<Arc<LiveIdentity>> {
        let key = identity_key(world, part)?;
        if let Some((live, used)) = self.worn.get_mut(&(model.key(), key)) {
            *used = self.frame;
            return (!live.is_empty()).then(|| Arc::clone(live));
        }
        let identity = assets.identity(key)?;
        if self.worn.len() >= self.limit.max(KEPT) {
            // Only the ones not worn this frame or the last go, so a part
            // visited later this frame still finds its own.
            let recent = self.frame.saturating_sub(1);
            self.worn.retain(|_, (_, used)| *used >= recent);
            self.limit = (2 * self.worn.len()).max(KEPT);
        }
        let live = Arc::new(model.live(&identity));
        self.made += 1;
        self.worn
            .insert((model.key(), key), (Arc::clone(&live), self.frame));
        (!live.is_empty()).then_some(live)
    }

    /// Starts a new frame (called once per `Animators::update`).
    pub(crate) fn next_frame(&mut self) {
        self.frame += 1;
    }

    pub(crate) fn clear(&mut self) {
        self.worn.clear();
        self.limit = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use axcenity_assets::{Identity, IdentityLayer, ModelMaterial, MorphTarget, Primitive};
    use axcenity_core::ClassName;

    use super::*;

    /// A triangle with one blend shape, `Build`.
    fn model() -> Model {
        let triangle = Primitive::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0.0, 0.0, 1.0]; 3],
            Vec::new(),
            vec![0, 1, 2],
            ModelMaterial::default(),
        );
        let mut model = Model::from_primitives(vec![triangle]).unwrap();
        model.morph_targets = vec![MorphTarget {
            name: "Build".into(),
            default_weight: 0.0,
        }];
        model
    }

    /// `count` MeshParts, each naming its own registered identity (a
    /// distinct `Build` weight), so each is a distinct (model, identity)
    /// pair with `model`.
    fn crowd(count: usize) -> (World, Assets, Vec<InstanceId>) {
        let assets = Assets::new(Path::new(env!("CARGO_MANIFEST_DIR")));
        let mut world = World::new();
        let mut parts = Vec::new();
        for index in 0..count {
            let identity = Arc::new(Identity {
                layers: vec![IdentityLayer {
                    shapes: vec![("Build".into(), (index + 1) as f32 / count as f32)],
                    bones: Vec::new(),
                }],
            });
            let key = assets.register_identity(&identity).unwrap();
            let part = world.create(ClassName::MeshPart).unwrap();
            set_identity_key(&mut world, part, Some(key));
            parts.push(part);
        }
        (world, assets, parts)
    }

    #[test]
    fn a_crowd_larger_than_the_cache_limit_is_not_remade_every_frame() {
        let (world, assets, parts) = crowd(2000);
        let model = model();
        let mut identities = LiveIdentities::default();
        for frame in 0..3 {
            identities.next_frame();
            let before = identities.made;
            for &part in &parts {
                assert!(identities.of(&world, &assets, part, &model).is_some());
            }
            let made = identities.made - before;
            if frame == 0 {
                assert_eq!(made, 2000, "the first frame makes each one once");
            } else {
                assert_eq!(made, 0, "frame {frame} remade {made} live identities");
            }
        }
    }

    #[test]
    fn identities_no_part_wears_any_more_are_let_go() {
        // A drag: each frame a new identity on the same character, far past
        // the limit. The cache must not keep them all.
        let (world, assets, parts) = crowd(3 * KEPT);
        let model = model();
        let mut identities = LiveIdentities::default();
        for &part in &parts {
            identities.next_frame();
            identities.of(&world, &assets, part, &model);
            assert!(
                identities.worn.len() <= KEPT,
                "kept {}",
                identities.worn.len()
            );
        }
        // The one worn this frame is still there.
        let before = identities.made;
        identities.of(&world, &assets, *parts.last().unwrap(), &model);
        assert_eq!(identities.made, before);
    }
}
