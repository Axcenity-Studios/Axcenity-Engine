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

use crate::model::{Model, TriangleAt, last_tag};
use crate::{Assets, Slot};

/// What marks hidden regions in a `MeshId`.
pub(crate) const TAG: char = 'h';

/// Which regions of a model to leave out.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegionHiding {
    /// The region map: an image file in the project (unused, and may be
    /// empty, when `hidden` is).
    pub map: String,
    /// The region indices whose triangles are left out.
    pub hidden: Vec<u8>,
    /// Masks (wave 5, hiding the body under clothing): image files in the
    /// project over the same texture coordinates; a triangle whose middle
    /// falls on a texel with red at least [`MASK_HIDES`] in any of them is
    /// left out too. A model with a primitive without texture coordinates
    /// is masked by number instead: triangle `k` of the full-detail model
    /// (its primitives in order) by texel `k` in reading order (its
    /// simpler copies keep every triangle). A mask that can't be read
    /// hides nothing.
    pub masks: Vec<String>,
}

/// The red value from which a mask's texel hides the triangle over it.
pub const MASK_HIDES: u8 = 128;

impl RegionHiding {
    /// A key that names the hiding the same way in every run (FNV-1a of
    /// the map's path and the sorted indices).
    pub fn key(&self) -> u64 {
        let mut hidden = self.hidden.clone();
        hidden.sort_unstable();
        hidden.dedup();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut add = |byte: u8| {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for byte in self
            .map
            .bytes()
            .chain(std::iter::once(0))
            .chain(hidden.iter().copied())
        {
            add(byte);
        }
        // Masks after a marker byte no region index list ends with, so a
        // hiding without masks keeps the key it had before them.
        let mut masks = self.masks.clone();
        masks.sort_unstable();
        masks.dedup();
        for mask in &masks {
            add(0xff);
            for byte in mask.bytes() {
                add(byte);
            }
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
        if hiding.hidden.is_empty() && hiding.masks.is_empty() {
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
        // The region map, when regions are hidden: without it, the base
        // itself (everything drawn rather than nothing).
        let map = if hiding.hidden.is_empty() {
            None
        } else {
            match self.texture_pixels_blocking(&hiding.map) {
                Some(map) => Some(map),
                None => return base,
            }
        };
        let masks: Vec<_> = hiding
            .masks
            .iter()
            .filter_map(|mask| self.texture_pixels_blocking(mask))
            .collect();
        let mut hidden = [false; 256];
        for index in &hiding.hidden {
            hidden[usize::from(*index)] = true;
        }
        // The red value of `image` at `uv`, as the region map is read.
        let red = |image: &crate::model::Image, uv: [f32; 2]| -> u8 {
            let (width, height) = (image.width.max(1), image.height.max(1));
            let x = ((uv[0].rem_euclid(1.0)) * width as f32) as u32;
            let y = ((uv[1].rem_euclid(1.0)) * height as f32) as u32;
            let at = (y.min(height - 1) * width + x.min(width - 1)) as usize * 4;
            image.rgba().get(at).copied().unwrap_or(0)
        };
        // A model with a primitive without texture coordinates (a lower
        // layer made without them) is masked by its triangles' numbers:
        // the mask's texels in reading order, one per triangle.
        let by_number = base
            .primitives
            .iter()
            .any(|p| p.uvs.len() != p.positions.len());
        let numbered = |image: &crate::model::Image, index: u32| -> u8 {
            let width = image.width.max(1);
            let at = ((index / width) * width + index % width) as usize * 4;
            image.rgba().get(at).copied().unwrap_or(0)
        };
        let model = Arc::new(base.without_triangles_where(&|at: TriangleAt| {
            let regions = at.uv.is_some_and(|uv| {
                map.as_ref()
                    .is_some_and(|map| hidden[usize::from(red(map, uv))])
            });
            let masked = masks.iter().any(|mask| match (by_number, at.uv, at.index) {
                (true, _, Some(index)) => numbered(mask, index) >= MASK_HIDES,
                (false, Some(uv), _) => red(mask, uv) >= MASK_HIDES,
                _ => false,
            });
            regions || masked
        }));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A project with a model of two triangles over the left and right
    /// halves of its texture square, and a mask whose left half hides.
    fn project() -> axcenity_test_dirs::TestDir {
        let project = axcenity_test_dirs::TestDir::new("hidden-masks");
        // A glTF with its buffer beside it: positions, texture
        // coordinates, then the indices.
        let positions: [[f32; 3]; 6] = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ];
        let uvs: [[f32; 2]; 6] = [
            [0.1, 0.1],
            [0.3, 0.1],
            [0.1, 0.9],
            [0.7, 0.1],
            [0.9, 0.1],
            [0.7, 0.9],
        ];
        let mut bytes: Vec<u8> = Vec::new();
        for p in positions.iter().flatten().chain(uvs.iter().flatten()) {
            bytes.extend_from_slice(&p.to_le_bytes());
        }
        for i in [0u16, 1, 2, 3, 4, 5] {
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        std::fs::write(project.join("Two.bin"), &bytes).unwrap();
        let gltf = format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"mesh":0,"name":"Two"}}],
            "meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"TEXCOORD_0":1}},"indices":2}}]}}],
            "buffers":[{{"uri":"Two.bin","byteLength":{len}}}],
            "bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":72}},{{"buffer":0,"byteOffset":72,"byteLength":48}},{{"buffer":0,"byteOffset":120,"byteLength":12}}],
            "accessors":[{{"bufferView":0,"componentType":5126,"count":6,"type":"VEC3","min":[0,0,0],"max":[3,1,0]}},
            {{"bufferView":1,"componentType":5126,"count":6,"type":"VEC2"}},
            {{"bufferView":2,"componentType":5123,"count":6,"type":"SCALAR"}}]}}"#,
            len = bytes.len()
        );
        std::fs::write(project.join("Two.gltf"), gltf).unwrap();
        let mut mask = image::RgbaImage::new(8, 8);
        for (x, _, pixel) in mask.enumerate_pixels_mut() {
            let red = if x < 4 { 255 } else { 0 };
            *pixel = image::Rgba([red, red, red, 255]);
        }
        mask.save(project.join("mask.png")).unwrap();
        project
    }

    fn triangles(assets: &Assets, mesh_id: &str) -> usize {
        let model = assets.model_blocking(mesh_id).unwrap();
        model.primitives.iter().map(|p| p.indices.len() / 3).sum()
    }

    /// Wave 5, hiding under clothing: a mask leaves out the triangles
    /// whose middle is on a texel with red from `MASK_HIDES`, with no
    /// region map; a mask that can't be read hides nothing; masks change
    /// the key, and a hiding without them keeps the key it had.
    #[test]
    fn a_mask_leaves_out_the_triangles_over_its_red_texels() {
        let project = project();
        let assets = Assets::new(&project);
        assert_eq!(triangles(&assets, "Two.gltf"), 2);
        let masked = RegionHiding {
            map: String::new(),
            hidden: Vec::new(),
            masks: vec!["mask.png".to_owned()],
        };
        let id = assets.hide_regions("Two.gltf", &Arc::new(masked.clone()));
        assert_ne!(id, "Two.gltf");
        assert_eq!(triangles(&assets, &id), 1);
        let model = assets.model_blocking(&id).unwrap();
        // The one left is the right-hand triangle.
        let kept = &model.primitives[0];
        let right = kept.indices.iter().all(|v| kept.uvs[*v as usize][0] > 0.5);
        assert!(right);

        let missing = RegionHiding {
            masks: vec!["nowhere.png".to_owned()],
            ..masked.clone()
        };
        let id = assets.hide_regions("Two.gltf", &Arc::new(missing));
        assert_eq!(triangles(&assets, &id), 2);

        let regions = RegionHiding {
            map: "Regions.png".to_owned(),
            hidden: vec![3, 1],
            masks: Vec::new(),
        };
        // FNV-1a of the map, a 0 and the sorted indices, as before masks.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in b"Regions.png\0\x01\x03" {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        assert_eq!(regions.key(), hash);
        let with_mask = RegionHiding {
            masks: vec!["mask.png".to_owned()],
            ..regions.clone()
        };
        assert_ne!(with_mask.key(), regions.key());
        // Nothing hidden and no masks: the model itself.
        let nothing = RegionHiding {
            masks: Vec::new(),
            ..masked
        };
        assert_eq!(
            assets.hide_regions("Two.gltf", &Arc::new(nothing)),
            "Two.gltf"
        );
    }
}
