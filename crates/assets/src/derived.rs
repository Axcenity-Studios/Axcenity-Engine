//! Derived models in `Assets`: a base model with a character's identity
//! baked in (`model/identity.rs`), found by its derived `MeshId` like any
//! other model. `Assets` keeps the identities hosts register with
//! [`Assets::derive`] and bakes a model the first time it's asked for;
//! characters with the same identity share the result. A derived model is
//! made from its base as it is now, so a reloaded base is baked again.

use std::collections::HashMap;
use std::sync::Arc;

use crate::model::{Identity, Model, derived_mesh_id, split_derived};
use crate::{Assets, Slot};

/// How many derived models are kept before the ones nothing else holds
/// are let go (a slider dragged in Character Studio makes one per step).
const KEPT: usize = 96;

/// The identities registered and the models baked from them.
#[derive(Default)]
pub(crate) struct Derived {
    identities: HashMap<u64, Arc<Identity>>,
    /// By derived `MeshId`: the base model's key when it was baked, and
    /// the baked model.
    models: HashMap<String, (u64, Arc<Model>)>,
}

impl Assets {
    /// The `MeshId` of the model at `mesh_id` with `identity` baked in,
    /// remembering `identity` so the model can be made when it's asked
    /// for. An identity that changes nothing gives the base `MeshId`.
    pub fn derive(&self, mesh_id: &str, identity: &Arc<Identity>) -> String {
        let (base, _) = split_derived(mesh_id);
        if identity.is_empty() {
            return base.to_owned();
        }
        let key = identity.key();
        self.lock()
            .derived
            .identities
            .entry(key)
            .or_insert_with(|| Arc::clone(identity));
        derived_mesh_id(base, key)
    }

    /// The identity a derived `MeshId` names, if this run registered it.
    pub fn identity_of(&self, mesh_id: &str) -> Option<Arc<Identity>> {
        let (_, key) = split_derived(mesh_id);
        self.lock().derived.identities.get(&key?).cloned()
    }

    /// The derived model `mesh_id` (whose identity is `key`) of `base`,
    /// baked now unless it's kept. With no such identity registered, the
    /// base model itself.
    fn derived_of(&self, mesh_id: &str, key: u64, base: Arc<Model>) -> Arc<Model> {
        let identity = {
            let shared = self.lock();
            if let Some((base_key, model)) = shared.derived.models.get(mesh_id)
                && *base_key == base.key()
            {
                return Arc::clone(model);
            }
            match shared.derived.identities.get(&key) {
                Some(identity) => Arc::clone(identity),
                None => return base,
            }
        };
        // Baked with the lock released: it copies the model's vertices.
        let model = Arc::new(base.bake(&identity));
        let mut shared = self.lock();
        let derived = &mut shared.derived;
        if derived.models.len() >= KEPT {
            derived
                .models
                .retain(|_, (_, model)| Arc::strong_count(model) > 1);
        }
        derived
            .models
            .insert(mesh_id.to_owned(), (base.key(), Arc::clone(&model)));
        shared.generation += 1;
        model
    }

    /// [`model_slot`](Self::model_slot) for a derived `MeshId`, or `None`
    /// for any other.
    pub(crate) fn derived_slot(&self, mesh_id: &str) -> Option<Slot<Model>> {
        let (base, key) = split_derived(mesh_id);
        let key = key?;
        Some(match self.model_slot(base) {
            Slot::Ready(model) => Slot::Ready(self.derived_of(mesh_id, key, model)),
            other => other,
        })
    }

    /// [`model_ready`](Self::model_ready) for a derived `MeshId`.
    pub(crate) fn derived_ready(&self, mesh_id: &str) -> Option<Option<Arc<Model>>> {
        let (base, key) = split_derived(mesh_id);
        let key = key?;
        Some(
            self.model_ready(base)
                .map(|model| self.derived_of(mesh_id, key, model)),
        )
    }

    /// [`model_blocking`](Self::model_blocking) for a derived `MeshId`.
    pub(crate) fn derived_blocking(&self, mesh_id: &str) -> Option<Option<Arc<Model>>> {
        let (base, key) = split_derived(mesh_id);
        let key = key?;
        Some(
            self.model_blocking(base)
                .map(|model| self.derived_of(mesh_id, key, model)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BoneChange, IdentityLayer};

    fn examples() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-humanoid")
    }

    const BODY: &str = "assets/models/Humanoid/Humanoid.glb";

    fn identity(weight: f32) -> Arc<Identity> {
        Arc::new(Identity {
            layers: vec![IdentityLayer {
                shapes: vec![("Frame_Feminine".to_owned(), weight)],
                bones: vec![BoneChange {
                    bone: "Hips".to_owned(),
                    offset: [0.0, 0.05, 0.0],
                    scale: 1.0,
                }],
            }],
        })
    }

    #[test]
    fn a_derived_model_is_found_by_its_mesh_id_and_shared() {
        let assets = Assets::new(examples());
        let base = assets.model_blocking(BODY).expect("the humanoid example");
        let id = assets.derive(BODY, &identity(1.0));
        assert_ne!(id, BODY);
        let derived = assets.model_blocking(&id).expect("baked from the base");
        assert_ne!(derived.key(), base.key());
        assert!(derived.settles());
        // Asked for again, by any of the lookups, it is the same model.
        let again = assets.model_ready(&id).expect("kept");
        assert!(Arc::ptr_eq(&derived, &again));
        let Slot::Ready(slot) = assets.model_slot(&id) else {
            panic!("ready");
        };
        assert!(Arc::ptr_eq(&derived, &slot));
        // The same identity registered again names the same model.
        assert_eq!(assets.derive(BODY, &identity(1.0)), id);
        assert_ne!(assets.derive(BODY, &identity(0.5)), id);
        assert!(assets.identity_of(&id).is_some());
    }

    #[test]
    fn an_identity_that_changes_nothing_is_the_base_model() {
        let assets = Assets::new(examples());
        assert_eq!(assets.derive(BODY, &Arc::new(Identity::default())), BODY);
    }

    #[test]
    fn an_identity_this_run_never_registered_is_the_base_model() {
        let assets = Assets::new(examples());
        let base = assets.model_blocking(BODY).expect("the humanoid example");
        let unknown = derived_mesh_id(BODY, 0x1234);
        let model = assets.model_blocking(&unknown).expect("its base");
        assert!(Arc::ptr_eq(&model, &base));
        assert!(assets.take_errors().is_empty());
    }

    #[test]
    fn a_reloaded_base_is_baked_again() {
        let assets = Assets::new(examples());
        let id = assets.derive(BODY, &identity(1.0));
        let first = assets.model_blocking(&id).expect("baked");
        assets.forget(BODY);
        let second = assets.model_blocking(&id).expect("baked again");
        assert_ne!(first.key(), second.key());
    }

    #[test]
    fn a_derived_model_of_a_missing_file_fails_like_its_base() {
        let assets = Assets::new(examples());
        let id = assets.derive("assets/models/none.glb", &identity(1.0));
        assert!(assets.model_blocking(&id).is_none());
        assert!(matches!(assets.model_slot(&id), Slot::Failed));
    }
}
