//! The catalogue (`catalogue.json` at a library's root): every base model
//! and item in the library with what a browser shows without opening each
//! manifest (label, kind, slot, tags, thumbnail), and every preset. It is an index made from
//! the manifests (`assets/characters/catalogue.py` for the repository's
//! library); [`Catalogue::of`] makes the same one, so a check can tell a
//! stale catalogue.

use serde::{Deserialize, Serialize};

use crate::item::{ContentTier, ItemKind};
use crate::{
    BaseManifest, Description, FormatError, ItemManifest, Library, PresetKind, pretty,
    read_versioned,
};

/// The catalogue's format version.
pub const CATALOGUE_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Catalogue {
    pub format_version: u32,
    /// By ID.
    #[serde(default)]
    pub bases: Vec<CatalogueBase>,
    /// By ID.
    #[serde(default)]
    pub items: Vec<CatalogueItem>,
    /// Presets (`*.preset.json`), by ID. Left out when there are none,
    /// so a catalogue without presets reads and writes as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<CataloguePreset>,
}

/// A preset: its ID, file (relative to the library), label (the preset's
/// `name`), kind, base model, tags and thumbnail (relative to the
/// library).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CataloguePreset {
    pub id: String,
    pub file: String,
    pub label: String,
    pub kind: PresetKind,
    pub base: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

impl CataloguePreset {
    /// The entry of `preset`, whose file is at `path`; `None` when it has
    /// no `preset` block.
    pub fn of(path: &str, preset: &Description) -> Option<Self> {
        let block = preset.preset()?;
        Some(Self {
            id: block.id,
            file: path.to_owned(),
            label: preset.name.clone(),
            kind: block.kind,
            base: preset.base.clone(),
            tags: block.tags,
            thumbnail: block
                .thumbnail
                .as_deref()
                .and_then(|thumbnail| crate::resolve_path(path, thumbnail)),
        })
    }
}

/// A base model: its ID, manifest (relative to the library), label and
/// family.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogueBase {
    pub id: String,
    pub manifest: String,
    pub label: String,
    pub family: String,
}

/// An item: its ID, manifest (relative to the library), and its manifest's
/// label, kind, slot, tier, bases and tags, and its thumbnail relative to
/// the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogueItem {
    pub id: String,
    pub manifest: String,
    pub label: String,
    pub kind: ItemKind,
    pub slot: String,
    pub tier: ContentTier,
    pub bases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
}

impl CatalogueBase {
    pub fn of(manifest_path: &str, base: &BaseManifest) -> Self {
        Self {
            id: base.id.clone(),
            manifest: manifest_path.to_owned(),
            label: base.label.clone(),
            family: base.family.clone(),
        }
    }
}

impl CatalogueItem {
    pub fn of(manifest_path: &str, item: &ItemManifest) -> Self {
        Self {
            id: item.id.clone(),
            manifest: manifest_path.to_owned(),
            label: item.label.clone(),
            kind: item.kind,
            slot: item.slot.clone(),
            tier: item.tier,
            bases: item.bases.clone(),
            tags: item.tags.clone(),
            thumbnail: item
                .thumbnail
                .as_deref()
                .and_then(|thumbnail| crate::resolve_path(manifest_path, thumbnail)),
        }
    }
}

impl Catalogue {
    pub fn from_json(text: &str) -> Result<Self, FormatError> {
        read_versioned(text, "catalogue", CATALOGUE_VERSION)
    }

    pub fn to_json(&self) -> String {
        pretty(self)
    }

    /// The catalogue of `library`'s manifests, by ID.
    pub fn of(library: &Library) -> Self {
        let path = |id: &str| library.manifest_path(id).unwrap_or_default().to_owned();
        Self {
            format_version: CATALOGUE_VERSION,
            bases: library
                .bases
                .values()
                .map(|base| CatalogueBase::of(&path(&base.id), base))
                .collect(),
            items: library
                .items
                .values()
                .map(|item| CatalogueItem::of(&path(&item.id), item))
                .collect(),
            presets: library
                .presets
                .iter()
                .filter_map(|(id, preset)| {
                    CataloguePreset::of(library.preset_path(id).unwrap_or_default(), preset)
                })
                .collect(),
        }
    }
}
