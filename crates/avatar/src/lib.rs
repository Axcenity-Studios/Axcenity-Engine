//! Character Data: the formats shared by Character Studio (the authoring
//! workspace, editor only) and the Character Runtime (which loads, applies
//! and changes characters in the player and server). Character Studio
//! overhaul, stage CO1; every schema and rule is in
//! `docs/development/character-studio/architecture.md`.
//!
//! - A [`Description`] (`*.character.json`, format 2) is one character:
//!   its base model, parameter values, the items it wears and its material
//!   values. Format 1 (Character Studio CS2's `Look`) migrates on load.
//! - A [`BaseManifest`] (`*.base.json`) ships with a base model: what each
//!   parameter drives (shapes, bones, attributes, material values), the
//!   body's regions, slots, sockets, materials, skeleton and face.
//! - An [`ItemManifest`] (`*.item.json`) is one garment, accessory or
//!   groom: its slot, layer, the regions it hides, its materials and
//!   variants, how it fits and where its model is.
//! - A [`Catalogue`] (`catalogue.json`) lists a library's base models,
//!   items and presets, and a [`Library`] holds them read.
//! - A preset (`*.preset.json`) is a description with a `preset` block;
//!   [`apply_preset`] copies what its kind owns. [`randomize`] draws a
//!   character from a seed.
//!
//! Nothing here draws, holds Instances or knows the editor: no camera,
//! lighting, layout or undo is ever written into these files.

mod catalogue;
mod choices;
mod description;
pub mod fit;
mod ids;
mod item;
mod library;
mod manifest;
mod material;
mod migrate;
mod model_check;
mod preset;
mod randomize;
mod resolve;
mod skin;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;

pub use catalogue::{CATALOGUE_VERSION, Catalogue, CatalogueBase, CatalogueItem, CataloguePreset};
pub use choices::{chosen_slots, wear_choice, wear_choices};
pub use description::{DESCRIPTION_VERSION, Description, Transform, WornItem};
pub use ids::{is_valid_id, is_valid_path, resolve_path};
pub use item::{
    Adjust, ContentTier, Fit, Groom, ITEM_VERSION, ItemKind, ItemManifest, ItemParameter,
    Limitation, Quality, Source, Variant,
};
pub use library::{LIBRARY_FOLDER, Library};
pub use manifest::{
    AddedBone, Anchor, AttributeDriver, BASE_VERSION, BaseManifest, BaseModel, BoneDriver,
    Category, Choice, Constraint, Corner, Defaults, Drive, Face, Factor, Feature, Group, Handle,
    Level, MaterialDriver, Parameter, Region, ShapeDriver, Skeleton, Slot, Socket,
};
pub use material::{
    Colour, MaterialRole, MaterialSlot, MaterialValue, MaterialValues, ShadingModel, ValueKind,
    ValueSpec,
};
pub use migrate::{V1_BASE, V1_ITEMS};
pub use model_check::{ModelContents, ModelRef};
pub use preset::{
    PRESET_SUFFIX, PresetInfo, PresetKind, apply_preset, check_preset, matches_preset,
};
pub use randomize::{Scope, random_span, randomize};
pub use resolve::{
    PartFit, Resolved, ResolvedPart, body_shape_weights, parameter_values, resolve, span,
};
pub use skin::{Skin, SkinLayer, SkinLayerKind, SkinLayerSpec, skin_layer_value};

use std::fmt;

/// Something wrong in a file or between files, found by a check: where it
/// is (a path into the file, such as `parameters[3].shapes[0]`, or a file
/// name) and what is wrong. A check returns every problem it finds; none
/// means the file is sound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub at: String,
    pub message: String,
}

impl Problem {
    pub fn new(at: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            at: at.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.message)
    }
}

/// Why a file couldn't be read at all.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    /// Not JSON, or not this kind of file.
    #[error("{0}")]
    Damaged(String),
    /// Saved by a newer Axcenity.
    #[error(
        "It was saved by a newer Axcenity ({what} format {found}; this version reads up to {supported})."
    )]
    Newer {
        what: &'static str,
        found: u32,
        supported: u32,
    },
}

/// A file's `format_version`: 1 when it has none (format 1 character files
/// always wrote one, but older hand-made files may not).
pub(crate) fn format_version(value: &serde_json::Value) -> Result<u32, FormatError> {
    match value.get("format_version") {
        None => Ok(1),
        Some(version) => version
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| FormatError::Damaged("`format_version` isn't a whole number".into())),
    }
}

/// Reads a manifest of a format that has had one version so far: refuses
/// a newer one, reads the rest with serde.
pub(crate) fn read_versioned<T: serde::de::DeserializeOwned>(
    text: &str,
    what: &'static str,
    supported: u32,
) -> Result<T, FormatError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|err| FormatError::Damaged(err.to_string()))?;
    if !value.is_object() {
        return Err(FormatError::Damaged(format!("a {what} is a JSON object")));
    }
    let found = format_version(&value)?;
    if found > supported {
        return Err(FormatError::Newer {
            what,
            found,
            supported,
        });
    }
    serde_json::from_value(value).map_err(|err| FormatError::Damaged(err.to_string()))
}

/// Pretty JSON with a final newline: how every file here is written
/// (maps are `BTreeMap`s, so the output is deterministic).
pub(crate) fn pretty<T: serde::Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

pub(crate) fn is_false(value: &bool) -> bool {
    !*value
}
