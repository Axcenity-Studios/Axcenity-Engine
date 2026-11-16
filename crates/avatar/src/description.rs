//! The character description (`*.character.json`, format 2): one
//! character, as Character Studio saves it and the Character Runtime
//! applies it. It names its base model and its items by ID and holds only
//! values: no meshes, no editor state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::manifest::{check_version, not_empty};
use crate::material::{MaterialValues, check_values};
use crate::{FormatError, Library, Problem, format_version, pretty};

/// The character description's format version. Format 1 was Character
/// Studio CS2's `Look`; it migrates on load ([`Description::from_json`]).
pub const DESCRIPTION_VERSION: u32 = 2;

/// One character.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Description {
    pub format_version: u32,
    pub name: String,
    /// The base model's ID, such as `axcenity/stylized`.
    pub base: String,
    /// Parameter values by the base manifest's parameter IDs. A parameter
    /// left out takes its default.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, f32>,
    /// What it wears, each item in its slot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<WornItem>,
    /// Values for the base model's materials (skin, eyes, brows, …), by
    /// material slot name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub materials: BTreeMap<String, MaterialValues>,
    /// Fields this version doesn't know, kept as they were: a field a newer
    /// minor addition writes survives a round trip through this version.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// An item worn: its ID, the variant chosen and values for its materials
/// (over the variant's).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WornItem {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub materials: BTreeMap<String, MaterialValues>,
}

impl WornItem {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            variant: None,
            materials: BTreeMap::new(),
        }
    }
}

impl Description {
    /// A character on `base` with every parameter at its default and
    /// nothing worn.
    pub fn new(name: impl Into<String>, base: impl Into<String>) -> Self {
        Self {
            format_version: DESCRIPTION_VERSION,
            name: name.into(),
            base: base.into(),
            parameters: BTreeMap::new(),
            items: Vec::new(),
            materials: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    /// Reads a character file of any format up to this version's,
    /// migrating older ones; a newer one is refused with a message.
    pub fn from_json(text: &str) -> Result<Self, FormatError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|err| FormatError::Damaged(err.to_string()))?;
        if !value.is_object() {
            return Err(FormatError::Damaged(
                "a character file is a JSON object".into(),
            ));
        }
        match format_version(&value)? {
            1 => crate::migrate::from_v1(value),
            DESCRIPTION_VERSION => {
                serde_json::from_value(value).map_err(|err| FormatError::Damaged(err.to_string()))
            }
            found => Err(FormatError::Newer {
                what: "character",
                found,
                supported: DESCRIPTION_VERSION,
            }),
        }
    }

    /// The file's text: pretty JSON, keys in a fixed order.
    pub fn to_json(&self) -> String {
        pretty(self)
    }

    /// Checks the description against `library`: its base model exists;
    /// every parameter is the base's, finite and in range; every item
    /// exists, fits the base, sits in one of its slots (one per slot unless
    /// the slot takes several) and names a variant and materials it has;
    /// every material value suits its slot's shading model.
    pub fn validate(&self, library: &Library) -> Vec<Problem> {
        let mut problems = Vec::new();
        let p = &mut problems;
        check_version(p, self.format_version, DESCRIPTION_VERSION);
        not_empty(p, "name", &self.name);
        let Some(base) = library.base(&self.base) else {
            p.push(Problem::new(
                "base",
                format!("the library has no base model `{}`", self.base),
            ));
            return problems;
        };
        for (id, value) in &self.parameters {
            let at = format!("parameters.{id}");
            let Some(parameter) = base.parameter(id) else {
                p.push(Problem::new(
                    at,
                    format!("`{}` has no such parameter", base.id),
                ));
                continue;
            };
            let [min, max] = parameter.range;
            if !value.is_finite() {
                p.push(Problem::new(at, "isn't a number"));
            } else if *value < min || *value > max {
                p.push(Problem::new(
                    at,
                    format!("{value} is outside its range, {min} to {max}"),
                ));
            }
        }
        let mut filled: BTreeMap<&str, &str> = BTreeMap::new();
        for (index, worn) in self.items.iter().enumerate() {
            let at = format!("items[{index}]");
            let Some(item) = library.item(&worn.id) else {
                p.push(Problem::new(
                    at,
                    format!("the library has no item `{}`", worn.id),
                ));
                continue;
            };
            if !item.bases.contains(&base.id) {
                p.push(Problem::new(
                    &at,
                    format!("`{}` doesn't fit `{}`", item.id, base.id),
                ));
                continue;
            }
            match base.slot(&item.slot) {
                None => p.push(Problem::new(
                    &at,
                    format!("`{}` has no slot `{}`", base.id, item.slot),
                )),
                Some(slot) if !slot.multiple => {
                    if let Some(first) = filled.insert(slot.id.as_str(), item.id.as_str()) {
                        p.push(Problem::new(
                            &at,
                            format!(
                                "the {} slot holds one item and `{first}` is already in it",
                                slot.id
                            ),
                        ));
                    }
                }
                Some(_) => {}
            }
            if let Some(variant) = &worn.variant
                && item.variant(variant).is_none()
            {
                p.push(Problem::new(
                    format!("{at}.variant"),
                    format!("`{}` has no variant `{variant}`", item.id),
                ));
            }
            for (name, values) in &worn.materials {
                let at = format!("{at}.materials.{name}");
                match item.material(name) {
                    None => p.push(Problem::new(
                        at,
                        format!("`{}` has no such material", item.id),
                    )),
                    Some(slot) => p.extend(check_values(&at, slot.shading, values)),
                }
            }
        }
        for (name, values) in &self.materials {
            let at = format!("materials.{name}");
            match base.material(name) {
                None => p.push(Problem::new(
                    at,
                    format!("`{}` has no such material", base.id),
                )),
                Some(slot) => p.extend(check_values(&at, slot.shading, values)),
            }
        }
        problems
    }
}
