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
    worn: HashMap<(u64, u64), Arc<LiveIdentity>>,
}

/// More than this many kept and they're all let go (a drag makes one per
/// part per step; each is a few kilobytes).
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
        if let Some(live) = self.worn.get(&(model.key(), key)) {
            return (!live.is_empty()).then(|| Arc::clone(live));
        }
        let identity = assets.identity(key)?;
        if self.worn.len() >= KEPT {
            self.worn.clear();
        }
        let live = Arc::new(model.live(&identity));
        self.worn.insert((model.key(), key), Arc::clone(&live));
        (!live.is_empty()).then_some(live)
    }

    pub(crate) fn clear(&mut self) {
        self.worn.clear();
    }
}
