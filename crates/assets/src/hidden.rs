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
//!
//! **Cover maps** (ADR-0045) hide more finely than regions: a garment's
//! cover map is an image over the *body's* texture coordinates, lit where
//! the garment lies over the skin (set back from its openings). A hiding
//! lists the cover maps of what is worn ([`RegionHiding::covers`]): the
//! body leaves out the triangles whose middle is lit in any of them, and
//! a **garment** worn under another ([`RegionHiding::garment`]) leaves
//! out the triangles whose corners all lie over lit skin
//! ([`Primitive::skin_uvs`](crate::model::Primitive::skin_uvs)), so
//! neither comes through the cloth over it.

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
    /// Cover maps (image files in the project, over the body's texture
    /// coordinates): a triangle under a pixel lit in any of them (at
    /// least [`COVERED`]: in red for the body, in green for a garment)
    /// is left out.
    pub covers: Vec<String>,
    /// Those of `covers` a garment is hidden under **whole**: read in
    /// blue, which is lit wherever the other garment's cloth is over the
    /// skin, with no setback from its openings (a top tucked into a
    /// bottom: nothing is seen under a waistband as it is under a hem).
    /// Garments only.
    pub whole: Vec<String>,
    /// The model is a garment fitted to the body, not the body: its
    /// triangles are placed on the cover maps by their corners' skin
    /// coordinates, and regions don't apply.
    pub garment: bool,
    /// For a garment whose file has no skin coordinates (one made on the
    /// body by its package, such as the underwear): the body's `MeshId`.
    /// Each of its corners then takes the coordinates of the body's
    /// corner nearest it at rest, so it too is hidden under what is worn
    /// over it. Empty: only a garment with coordinates of its own is.
    pub skin: String,
}

/// A cover map's pixel is lit from this value up.
pub const COVERED: u8 = 128;

/// How far (metres) the covered corners of a garment's triangles that
/// reach out from under the garment over it sink toward the body
/// (`Model::without_covered`): past the thickness of a waistband or a
/// collar, so what is left of them goes under the outer garment's edge.
const SINK: f32 = 0.015;

/// How far round a point of the skin (in texture coordinates) a cover
/// map is read to tell a pinhole in it from its edge, for a garment that
/// lies on the skin.
const PINHOLE: f32 = 0.01;

impl RegionHiding {
    /// Only regions of the body's region map `map`.
    pub fn regions(map: impl Into<String>, hidden: Vec<u8>) -> Self {
        Self {
            map: map.into(),
            hidden,
            covers: Vec::new(),
            whole: Vec::new(),
            garment: false,
            skin: String::new(),
        }
    }

    /// Whether it leaves anything out.
    pub fn is_empty(&self) -> bool {
        (self.garment || self.hidden.is_empty()) && self.covers.is_empty()
    }

    /// A key that names the hiding the same way in every run (FNV-1a of
    /// the map's path, the sorted indices, the sorted cover maps and
    /// whether it is a garment's). A hiding without cover maps has the
    /// key it had before there were any.
    pub fn key(&self) -> u64 {
        let mut hidden = self.hidden.clone();
        hidden.sort_unstable();
        hidden.dedup();
        let mut covers = self.covers.clone();
        covers.sort_unstable();
        covers.dedup();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut feed = |byte: u8| {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        };
        for byte in self
            .map
            .bytes()
            .chain(std::iter::once(0))
            .chain(hidden.iter().copied())
        {
            feed(byte);
        }
        for cover in &covers {
            feed(0);
            cover.bytes().for_each(&mut feed);
        }
        if self.garment {
            feed(0);
            feed(b'g');
            if !self.skin.is_empty() {
                feed(0);
                self.skin.bytes().for_each(&mut feed);
            }
            let mut whole = self.whole.clone();
            whole.sort_unstable();
            whole.dedup();
            for cover in &whole {
                feed(0);
                feed(b'w');
                cover.bytes().for_each(&mut feed);
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
        if hiding.is_empty() {
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
        // Cover maps that can't be read cover nothing.
        let covers: Vec<Arc<crate::Image>> = hiding
            .covers
            .iter()
            .filter_map(|cover| self.texture_pixels_blocking(cover))
            .collect();
        // A pixel's channel: red is skin to leave out, green is skin
        // under which cloth worn beneath is left out.
        let lit = |image: &crate::Image, uv: [f32; 2], channel: usize| -> u8 {
            let (width, height) = (image.width.max(1), image.height.max(1));
            let x = ((uv[0].rem_euclid(1.0)) * width as f32) as u32;
            let y = ((uv[1].rem_euclid(1.0)) * height as f32) as u32;
            let at = (y.min(height - 1) * width + x.min(width - 1)) as usize * 4;
            image.rgba().get(at + channel).copied().unwrap_or(0)
        };
        let channel = usize::from(hiding.garment);
        // A garment is under some of them whole: those are read in blue.
        let whole: Vec<bool> = hiding
            .covers
            .iter()
            .filter(|cover| self.texture_pixels_blocking(cover).is_some())
            .map(|cover| hiding.garment && hiding.whole.contains(cover))
            .collect();
        let covered = |uv: [f32; 2]| {
            covers
                .iter()
                .zip(&whole)
                .any(|(cover, whole)| lit(cover, uv, if *whole { 2 } else { channel }) >= COVERED)
        };
        let model = if hiding.garment {
            if covers.is_empty() {
                return base;
            }
            // A garment without skin coordinates of its own borrows the
            // body's.
            let borrowed = (!hiding.skin.is_empty() && !base.has_skin_uvs())
                .then(|| self.model_blocking(&hiding.skin))
                .flatten()
                .map(|body| base.with_skin_from(&body));
            let garment = borrowed.as_ref().unwrap_or(&*base);
            // One made on the body lies on the skin, under the edge of
            // what is worn over it: its corners at the cover's edge stay
            // where they are (sunk, with the skin under them not drawn,
            // they showed as shards round a neckline).
            // and they go only where they are well under it.
            match &borrowed {
                Some(on_skin) => {
                    // A pinhole in the cover (a patch of skin the garment
                    // over it touches) is no edge of it: covered is what
                    // most of the skin round a point is.
                    let mostly = |uv: [f32; 2]| {
                        let mut lit = 0;
                        for dx in [-PINHOLE, 0.0, PINHOLE] {
                            for dy in [-PINHOLE, 0.0, PINHOLE] {
                                lit += usize::from(covered([uv[0] + dx, uv[1] + dy]));
                            }
                        }
                        lit >= 5
                    };
                    Arc::new(on_skin.without_well_covered(&mostly))
                }
                None => Arc::new(garment.without_covered(&covered, SINK)),
            }
        } else {
            let regions = if hiding.hidden.is_empty() {
                None
            } else {
                self.texture_pixels_blocking(&hiding.map)
            };
            if regions.is_none() && covers.is_empty() {
                return base;
            }
            let mut hidden = [false; 256];
            for index in &hiding.hidden {
                hidden[usize::from(*index)] = true;
            }
            Arc::new(base.without_triangles(&|uv| {
                regions
                    .as_ref()
                    .is_some_and(|map| hidden[usize::from(lit(map, uv, 0))])
                    || covered(uv)
            }))
        };
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

    /// A hiding of regions alone keeps the key it had before cover maps,
    /// so the `MeshId`s kept in saved worlds still name the same model;
    /// cover maps and a garment's hiding each have keys of their own.
    #[test]
    fn a_hiding_s_key_tells_regions_covers_and_garments_apart() {
        let regions = RegionHiding::regions("map.png", vec![3, 1]);
        assert_eq!(
            regions.key(),
            RegionHiding::regions("map.png", vec![1, 3, 3]).key()
        );
        let covered = RegionHiding {
            covers: vec!["b.png".to_owned(), "a.png".to_owned()],
            ..regions.clone()
        };
        assert_ne!(covered.key(), regions.key());
        // The order the garments were put on in doesn't matter.
        let other_order = RegionHiding {
            covers: vec!["a.png".to_owned(), "b.png".to_owned()],
            ..regions.clone()
        };
        assert_eq!(covered.key(), other_order.key());
        let garment = RegionHiding {
            garment: true,
            ..covered.clone()
        };
        assert_ne!(garment.key(), covered.key());
        // Hidden whole under one of them (tucked into it) is another model.
        let tucked = RegionHiding {
            whole: vec!["a.png".to_owned()],
            ..garment.clone()
        };
        assert_ne!(tucked.key(), garment.key());
    }

    #[test]
    fn a_hiding_with_nothing_to_hide_is_empty() {
        assert!(RegionHiding::regions("map.png", Vec::new()).is_empty());
        assert!(!RegionHiding::regions("map.png", vec![2]).is_empty());
        let garment = |covers: Vec<String>| RegionHiding {
            map: String::new(),
            // A garment has no regions of its own to hide.
            hidden: vec![2],
            covers,
            whole: Vec::new(),
            garment: true,
            skin: String::new(),
        };
        assert!(garment(Vec::new()).is_empty());
        assert!(!garment(vec!["a.png".to_owned()]).is_empty());
    }
}
