//! Presets as data (plan section 11.3): a preset file (`*.preset.json`) is
//! a format 2 description with a `preset` block, `{id, kind, about, tags?,
//! thumbnail?, order?}`, which rides in the description's `extra`, so any
//! version reads it. Its `kind` says what applying it changes; everything
//! else in the character is left alone. [`apply_preset`] is the one rule,
//! shared by Character Studio, scripts and Randomize.

use serde::{Deserialize, Serialize};

use crate::manifest::{check_id, check_path, not_empty, unique};
use crate::{Category, Description, Library, Problem, chosen_slots, wear_choices};

/// The top-level field a preset's block is in.
pub(crate) const PRESET_FIELD: &str = "preset";

/// What a preset file ends with, so the saved-characters shelf and the
/// manifest scans pass it by.
pub const PRESET_SUFFIX: &str = ".preset.json";

/// What applying a preset sets. A kind a newer version adds reads as
/// `Other`, which this version lists but can't apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetKind {
    /// Every parameter in a `body` group.
    Body,
    /// Every parameter in a `face` group: a complete face.
    Face,
    /// The items in `hair` slots (and parameters in `hair` groups).
    Hair,
    /// The items in `clothing` and `accessories` slots, except the slots a
    /// parameter's choices fill.
    Outfit,
    /// The base model's material values and parameters in `skin` groups.
    Skin,
    /// Everything but the name: a whole character.
    Character,
    #[serde(other)]
    Other,
}

impl PresetKind {
    /// Its name in files and messages.
    pub fn name(self) -> &'static str {
        match self {
            PresetKind::Body => "body",
            PresetKind::Face => "face",
            PresetKind::Hair => "hair",
            PresetKind::Outfit => "outfit",
            PresetKind::Skin => "skin",
            PresetKind::Character => "character",
            PresetKind::Other => "other",
        }
    }

    /// Whether a preset of this kind sets the parameters of groups, and
    /// the items of slots, in `category`.
    pub fn owns(self, category: Category) -> bool {
        match self {
            PresetKind::Body => category == Category::Body,
            PresetKind::Face => category == Category::Face,
            PresetKind::Hair => category == Category::Hair,
            PresetKind::Outfit => matches!(category, Category::Clothing | Category::Accessories),
            PresetKind::Skin => category == Category::Skin,
            PresetKind::Character => true,
            PresetKind::Other => false,
        }
    }

    /// Whether a preset of this kind sets the base model's material values.
    pub fn owns_materials(self) -> bool {
        matches!(self, PresetKind::Skin | PresetKind::Character)
    }
}

/// A preset file's `preset` block.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetInfo {
    /// Its ID in the library, such as `axcenity/realistic-face-round`.
    pub id: String,
    pub kind: PresetKind,
    /// One sentence for the preset's card.
    #[serde(default)]
    pub about: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// A picture, relative to the preset file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Where it goes in a list of its kind (lower first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
}

/// Applies `preset` to `description`: sets what the preset's kind owns
/// (a parameter or slot the preset leaves out goes back to its default or
/// empty, but a parameter with choices, such as the frame, changes only
/// when the preset names it) and leaves the rest, the name and every
/// field it doesn't know alone; the `preset` block never comes along.
/// What a parameter's choices wear is put right after. Changes nothing
/// and says why when `preset` isn't a preset, is of a kind this version
/// doesn't know, or (but for a `character` preset) is for another base
/// model.
pub fn apply_preset(
    description: &mut Description,
    preset: &Description,
    library: &Library,
) -> Result<(), Problem> {
    let Some(block) = preset.preset() else {
        return Err(Problem::new(PRESET_FIELD, "it isn't a preset"));
    };
    let kind = block.kind;
    if kind == PresetKind::Other {
        return Err(Problem::new(
            "preset.kind",
            format!("`{}` is of a kind this version can't apply", block.id),
        ));
    }
    let base_id = if kind == PresetKind::Character {
        &preset.base
    } else {
        &description.base
    };
    let Some(base) = library.base(base_id) else {
        return Err(Problem::new(
            "base",
            format!("the library has no base model `{base_id}`"),
        ));
    };
    if kind == PresetKind::Character {
        description.base = preset.base.clone();
        description.parameters = preset.parameters.clone();
        description.items = preset.items.clone();
        description.materials = preset.materials.clone();
        wear_choices(description, library);
        return Ok(());
    }
    if preset.base != description.base {
        return Err(Problem::new(
            "base",
            format!(
                "`{}` is for `{}`; the character is on `{}`",
                block.id, preset.base, description.base
            ),
        ));
    }
    for parameter in &base.parameters {
        if !base.category_of(parameter).is_some_and(|c| kind.owns(c)) {
            continue;
        }
        match preset.parameters.get(&parameter.id) {
            Some(value) => {
                description.parameters.insert(parameter.id.clone(), *value);
            }
            None if !parameter.choices.is_empty() => {}
            None => {
                description.parameters.remove(&parameter.id);
            }
        }
    }
    let chosen = chosen_slots(library, &base.id);
    let owned = |item_id: &str| {
        library.item(item_id).is_some_and(|item| {
            !chosen.contains(&item.slot)
                && base
                    .slot(&item.slot)
                    .is_some_and(|slot| kind.owns(slot.category))
        })
    };
    description.items.retain(|worn| !owned(&worn.id));
    description
        .items
        .extend(preset.items.iter().filter(|worn| owned(&worn.id)).cloned());
    if kind.owns_materials() {
        description.materials = preset.materials.clone();
    }
    wear_choices(description, library);
    Ok(())
}

/// Whether `description` is what applying `preset` to it gives: no
/// slider moved since (Character Studio shows "Custom" when no preset of
/// a kind matches).
pub fn matches_preset(description: &Description, preset: &Description, library: &Library) -> bool {
    let mut applied = description.clone();
    apply_preset(&mut applied, preset, library).is_ok() && applied == *description
}

/// Checks a preset: its block is sound (an ID, a sentence, a path), it is
/// a sound description of a base model in `library`, and it sets only
/// what its kind owns.
pub fn check_preset(preset: &Description, library: &Library) -> Vec<Problem> {
    let mut problems = Vec::new();
    let p = &mut problems;
    let Some(value) = preset.extra.get(PRESET_FIELD) else {
        p.push(Problem::new(PRESET_FIELD, "has no `preset` block"));
        return problems;
    };
    let block: PresetInfo = match serde_json::from_value(value.clone()) {
        Ok(block) => block,
        Err(err) => {
            p.push(Problem::new(PRESET_FIELD, err.to_string()));
            return problems;
        }
    };
    check_id(p, "preset.id", &block.id);
    not_empty(p, "preset.about", &block.about);
    if let Some(thumbnail) = &block.thumbnail {
        check_path(p, "preset.thumbnail", thumbnail);
    }
    unique(p, "preset.tags", block.tags.iter());
    if block.kind == PresetKind::Other {
        p.push(Problem::new(
            "preset.kind",
            "isn't a kind this version knows",
        ));
    }
    p.extend(preset.validate(library));
    let kind = block.kind;
    let Some(base) = library.base(&preset.base) else {
        return problems;
    };
    if kind == PresetKind::Character {
        return problems;
    }
    for id in preset.parameters.keys() {
        let Some(parameter) = base.parameter(id) else {
            continue;
        };
        let category = base.category_of(parameter);
        if !category.is_some_and(|c| kind.owns(c)) {
            p.push(Problem::new(
                format!("parameters.{id}"),
                format!(
                    "a {} preset doesn't set `{id}`, a {} parameter",
                    kind.name(),
                    category.map_or("groupless", category_name)
                ),
            ));
        }
    }
    let chosen = chosen_slots(library, &base.id);
    for (index, worn) in preset.items.iter().enumerate() {
        let Some(item) = library.item(&worn.id) else {
            continue;
        };
        let owned = !chosen.contains(&item.slot)
            && base
                .slot(&item.slot)
                .is_some_and(|slot| kind.owns(slot.category));
        if !owned {
            p.push(Problem::new(
                format!("items[{index}]"),
                format!(
                    "a {} preset doesn't wear `{}` (slot `{}`)",
                    kind.name(),
                    item.id,
                    item.slot
                ),
            ));
        }
    }
    if !kind.owns_materials() {
        for name in preset.materials.keys() {
            p.push(Problem::new(
                format!("materials.{name}"),
                format!("a {} preset doesn't set material values", kind.name()),
            ));
        }
    }
    problems
}

fn category_name(category: Category) -> &'static str {
    match category {
        Category::Body => "body",
        Category::Face => "face",
        Category::Skin => "skin",
        Category::Hair => "hair",
        Category::Clothing => "clothing",
        Category::Accessories => "accessories",
    }
}
