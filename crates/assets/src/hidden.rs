//! Models with hidden regions (Character Studio overhaul CO9, ADR-0035):
//! a body with the skin a worn garment covers left out, so it is never
//! drawn under the cloth (and never pokes through it).
//!
//! A body says which region each of its triangles is in with a **region
//! map**: an image over its first texture coordinates whose red value is
//! the region's index. A [`RegionHiding`] names the map and the indices to
//! hide; [`Assets::hide_regions`] registers it and gives the `MeshId` of
//! the model with those triangles left out: its base's with a tag,
//! `…#h<16 hex>` (the hiding's key). Like a derived model
//! (`derived.rs`), it is made from its base the first time it is asked
//! for and shared by every character that hides the same regions. Only
//! the triangle lists change: the corners, blend shapes and skeleton are
//! the base's, so a character's live identity and pose apply as they are.

use std::sync::Arc;

use crate::model::{Model, last_tag};
use crate::{Assets, Slot};

/// What marks hidden regions in a `MeshId`.
pub(crate) const TAG: char = 'h';

/// Which regions of a model to leave out.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegionHiding {
    /// The region map: an image file in the project.
    pub map: String,
    /// The region indices whose triangles are left out.
    pub hidden: Vec<u8>,
}

impl RegionHiding {
    /// A key that names the hiding the same way in every run (FNV-1a of
    /// the map's path and the sorted indices).
    pub fn key(&self) -> u64 {
        let mut hidden = self.hidden.clone();
        hidden.sort_unstable();
        hidden.dedup();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in self
            .map
            .bytes()
            .chain(std::iter::once(0))
            .chain(hidden.iter().copied())
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }
}

/// The `MeshId` of `base` with the hiding `key`'s regions left out.
pub fn hidden_mesh_id(base: &str, key: u64) -> String {
    let base = split_hidden(base).0;
    if base.contains('#') {
        format!("{base}#{TAG}{key:016x}")
    } else {
        format!("{base}##{TAG}{key:016x}")
    }
}

/// A `MeshId`'s base model and, for one with hidden regions (and no
/// identity baked on top), its hiding's key.
pub fn split_hidden(mesh_id: &str) -> (&str, Option<u64>) {
    match last_tag(mesh_id) {
        Some((base, tag, Some(key))) if tag == TAG => (base, Some(key)),
        _ => (mesh_id, None),
    }
}

impl Assets {
    /// Remembers `hiding` and returns the `MeshId` of the model at
    /// `mesh_id` with its regions left out. Hiding nothing gives
    /// `mesh_id` without any hiding it had.
    pub fn hide_regions(&self, mesh_id: &str, hiding: &Arc<RegionHiding>) -> String {
        let base = split_hidden(mesh_id).0;
        if hiding.hidden.is_empty() {
            return base.to_owned();
        }
        let key = hiding.key();
        self.lock()
            .derived
            .hidings
            .entry(key)
            .or_insert_with(|| Arc::clone(hiding));
        hidden_mesh_id(base, key)
    }

    /// The hiding a `MeshId` names, if this run registered it.
    pub fn hiding_of(&self, mesh_id: &str) -> Option<Arc<RegionHiding>> {
        let key = split_hidden(mesh_id).1?;
        self.lock().derived.hidings.get(&key).cloned()
    }

    /// The model `mesh_id` (with the hiding `key`) made from `base`, now
    /// unless it's kept. Without the hiding registered, or without its
    /// map, the base itself: everything drawn rather than nothing.
    fn hidden_of(&self, mesh_id: &str, key: u64, base: Arc<Model>) -> Arc<Model> {
        let hiding = {
            let shared = self.lock();
            if let Some((base_key, model)) = shared.derived.hidden_models.get(mesh_id)
                && *base_key == base.key()
            {
                return Arc::clone(model);
            }
            match shared.derived.hidings.get(&key) {
                Some(hiding) => Arc::clone(hiding),
                None => return base,
            }
        };
        let Some(map) = self.texture_pixels_blocking(&hiding.map) else {
            return base;
        };
        let (width, height) = (map.width.max(1), map.height.max(1));
        let pixels = map.rgba();
        let mut hidden = [false; 256];
        for index in &hiding.hidden {
            hidden[usize::from(*index)] = true;
        }
        let region = |uv: [f32; 2]| -> u8 {
            let x = ((uv[0].rem_euclid(1.0)) * width as f32) as u32;
            let y = ((uv[1].rem_euclid(1.0)) * height as f32) as u32;
            let at = (y.min(height - 1) * width + x.min(width - 1)) as usize * 4;
            pixels.get(at).copied().unwrap_or(0)
        };
        let model = Arc::new(base.without_triangles(&|uv| hidden[usize::from(region(uv))]));
        let mut shared = self.lock();
        shared
            .derived
            .hidden_models
            .insert(mesh_id.to_owned(), (base.key(), Arc::clone(&model)));
        shared.generation += 1;
        model
    }

    /// [`model_slot`](Self::model_slot) for a `MeshId` with hidden
    /// regions, or `None` for any other.
    pub(crate) fn hidden_slot(&self, mesh_id: &str) -> Option<Slot<Model>> {
        let (base, key) = split_hidden(mesh_id);
        let key = key?;
        Some(match self.model_slot(base) {
            Slot::Ready(model) => Slot::Ready(self.hidden_of(mesh_id, key, model)),
            other => other,
        })
    }

    /// [`model_ready`](Self::model_ready) for a `MeshId` with hidden
    /// regions.
    pub(crate) fn hidden_ready(&self, mesh_id: &str) -> Option<Option<Arc<Model>>> {
        let (base, key) = split_hidden(mesh_id);
        let key = key?;
        Some(
            self.model_ready(base)
                .map(|model| self.hidden_of(mesh_id, key, model)),
        )
    }

    /// [`model_blocking`](Self::model_blocking) for a `MeshId` with hidden
    /// regions.
    pub(crate) fn hidden_blocking(&self, mesh_id: &str) -> Option<Option<Arc<Model>>> {
        let (base, key) = split_hidden(mesh_id);
        let key = key?;
        Some(
            self.model_blocking(base)
                .map(|model| self.hidden_of(mesh_id, key, model)),
        )
    }
}
