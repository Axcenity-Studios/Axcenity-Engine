//! Presets in the studio (refinement plan 11.3, architecture §15a): the
//! library's `*.preset.json` files and a person's own, which are the same
//! files in the workspace's `presets/` folder. They are addressed by their
//! `preset` block's ID; applying one is `avatar::apply_preset` (one rule
//! for the studio, scripts and Randomize) and "Custom" is no preset of a
//! kind matching (`avatar::matches_preset`). Favourites are editor state,
//! a small file beside the Recent list, never in a character or the
//! library.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use axcenity_avatar::{
    BaseManifest, Description, Library, PRESET_SUFFIX, PresetInfo, PresetKind, check_preset,
    chosen_slots,
};
use axcenity_gameplay::description as applier;
use serde::{Deserialize, Serialize};

use super::{DEFAULT_BASE, value_of};

/// A person's own presets, in the workspace's folder.
pub const OWN_PRESETS_FOLDER: &str = "presets";
/// The namespace of a person's own preset IDs (`my/round-face`).
pub const OWN_NAMESPACE: &str = "my";
/// The favourites, in the workspace's folder.
pub const FAVOURITES_FILE: &str = "favourites.json";
const FAVOURITES_VERSION: u32 = 1;

/// The kinds in the order the studio lists them.
pub const KINDS: [PresetKind; 6] = [
    PresetKind::Character,
    PresetKind::Body,
    PresetKind::Face,
    PresetKind::Hair,
    PresetKind::Outfit,
    PresetKind::Skin,
];

/// A kind's heading.
pub fn kind_label(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::Character => "Characters",
        PresetKind::Body => "Bodies",
        PresetKind::Face => "Faces",
        PresetKind::Hair => "Hairstyles",
        PresetKind::Outfit => "Outfits",
        PresetKind::Skin => "Skin",
        PresetKind::Other => "Other",
    }
}

/// A kind's name in "Save as preset".
pub fn kind_name(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::Character => "Whole character",
        PresetKind::Body => "Body",
        PresetKind::Face => "Face",
        PresetKind::Hair => "Hair",
        PresetKind::Outfit => "Outfit",
        PresetKind::Skin => "Skin",
        PresetKind::Other => "Other",
    }
}

/// One preset, as the studio lists it.
#[derive(Clone, Debug)]
pub struct Listed<'a> {
    pub id: &'a str,
    pub info: PresetInfo,
    /// The preset file: a description with its block.
    pub preset: &'a Description,
    /// A person's own (in the workspace's `presets/`), not the library's.
    pub own: bool,
}

impl Listed<'_> {
    /// Its label: the preset file's `name`.
    pub fn label(&self) -> &str {
        &self.preset.name
    }

    /// What the picture of it shows: a character preset as it is; a part
    /// preset on its base model's default character (on the body type its
    /// tags name, for a body preset).
    pub fn pictured(&self, library: &Library) -> Description {
        let mut shown = applier::default_description(library, &self.preset.base);
        shown.name = self.label().to_owned();
        if let Some(base) = library.base(&self.preset.base) {
            for parameter in base.parameters.iter().filter(|p| !p.choices.is_empty()) {
                if let Some(choice) = parameter.choices.iter().find(|choice| {
                    self.info
                        .tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case(&choice.label))
                }) {
                    shown.parameters.insert(parameter.id.clone(), choice.value);
                }
            }
        }
        let _ = axcenity_avatar::apply_preset(&mut shown, self.preset, library);
        shown.extra.clear();
        shown
    }
}

/// Every preset of the library and `own`, sorted by kind, then `order`,
/// then label (the library's first where both have one).
pub fn all<'a>(library: &'a Library, own: &'a BTreeMap<String, Description>) -> Vec<Listed<'a>> {
    let listed = |(id, preset): (&'a String, &'a Description), own: bool| {
        preset.preset().map(|info| Listed {
            id: id.as_str(),
            info,
            preset,
            own,
        })
    };
    let mut all: Vec<Listed> = library
        .presets
        .iter()
        .filter_map(|entry| listed(entry, false))
        .chain(own.iter().filter_map(|entry| listed(entry, true)))
        .collect();
    let rank = |kind: PresetKind| KINDS.iter().position(|k| *k == kind).unwrap_or(KINDS.len());
    all.sort_by(|a, b| {
        (rank(a.info.kind), a.own, a.info.order.unwrap_or(i32::MAX))
            .cmp(&(rank(b.info.kind), b.own, b.info.order.unwrap_or(i32::MAX)))
            .then_with(|| a.label().cmp(b.label()))
    });
    all
}

/// The presets of `kind` for base model `base`.
pub fn of_kind<'a>(
    library: &'a Library,
    own: &'a BTreeMap<String, Description>,
    base: &str,
    kind: PresetKind,
) -> Vec<Listed<'a>> {
    all(library, own)
        .into_iter()
        .filter(|listed| listed.info.kind == kind && listed.preset.base == base)
        .collect()
}

/// The preset `id`, the library's or a person's own.
pub fn find<'a>(
    library: &'a Library,
    own: &'a BTreeMap<String, Description>,
    id: &str,
) -> Option<&'a Description> {
    library.preset(id).or_else(|| own.get(id))
}

/// Whether a body preset suits `description`'s body type: one tagged with
/// a choice's label ("male", "female") suits that choice only; one tagged
/// with none suits every body.
pub fn suits(listed: &Listed, description: &Description, base: &BaseManifest) -> bool {
    base.parameters
        .iter()
        .filter(|parameter| !parameter.choices.is_empty())
        .all(|parameter| {
            let tagged: Vec<_> = parameter
                .choices
                .iter()
                .filter(|choice| {
                    listed
                        .info
                        .tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case(&choice.label))
                })
                .collect();
            let value = value_of(description, Some(base), &parameter.id);
            let picked = parameter.nearest_choice(value).map(|choice| choice.value);
            tagged.is_empty() || tagged.iter().any(|choice| Some(choice.value) == picked)
        })
}

/// The bases in the order New and the Body page offer them: the realistic
/// family first (the default, plan section 0 decision 7), then by label.
pub fn families(library: &Library) -> Vec<&BaseManifest> {
    let mut bases: Vec<&BaseManifest> = library
        .bases
        .values()
        .filter(|base| super::offers_base(library, &base.id))
        .collect();
    bases.sort_by(|a, b| (a.id != DEFAULT_BASE, &a.label).cmp(&(b.id != DEFAULT_BASE, &b.label)));
    bases
}

/// A person's own presets in `folder`'s `presets/`, by ID: files that
/// don't read or have no block are passed by.
pub fn read_own(folder: &Path) -> BTreeMap<String, Description> {
    let mut own = BTreeMap::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(folder.join(OWN_PRESETS_FOLDER))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(PRESET_SUFFIX))
        })
        .collect();
    files.sort();
    for file in files {
        let Some(preset) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| Description::from_json(&text).ok())
        else {
            continue;
        };
        if let Some(info) = preset.preset() {
            own.entry(info.id).or_insert(preset);
        }
    }
    own
}

/// A file and ID name from `name`: lower-case letters and digits joined by
/// `-` ("Round face!" is `round-face`).
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_owned();
    if slug.is_empty() {
        "preset".to_owned()
    } else {
        slug
    }
}

/// `description` as a preset of `kind` named `name` with ID `id`: only what
/// the kind owns (architecture §15a), so applying it sets just that. A body
/// preset leaves out a choice (the body type) and is tagged with it, as
/// the shipped ones are; a character preset is everything but the name.
pub fn make(
    description: &Description,
    kind: PresetKind,
    name: &str,
    id: &str,
    library: &Library,
) -> Description {
    let mut preset = Description::new(name, &description.base);
    let mut tags = Vec::new();
    let base = library.base(&description.base);
    if kind == PresetKind::Character {
        preset.parameters = description.parameters.clone();
        preset.items = description.items.clone();
        preset.materials = description.materials.clone();
    } else if let Some(base) = base {
        for parameter in &base.parameters {
            let owned = base.category_of(parameter).is_some_and(|c| kind.owns(c));
            if !owned {
                continue;
            }
            let value = value_of(description, Some(base), &parameter.id);
            if !parameter.choices.is_empty() {
                if kind == PresetKind::Body
                    && let Some(choice) = parameter.nearest_choice(value)
                {
                    tags.push(choice.label.to_lowercase());
                }
                continue;
            }
            if let Some(value) = description.parameters.get(&parameter.id) {
                preset.parameters.insert(parameter.id.clone(), *value);
            }
        }
        let chosen = chosen_slots(library, &base.id);
        preset.items = description
            .items
            .iter()
            .filter(|worn| {
                library.item(&worn.id).is_some_and(|item| {
                    !chosen.contains(&item.slot)
                        && base
                            .slot(&item.slot)
                            .is_some_and(|slot| kind.owns(slot.category))
                })
            })
            .cloned()
            .collect();
        if kind.owns_materials() {
            preset.materials = description.materials.clone();
        }
    }
    let info = PresetInfo {
        id: id.to_owned(),
        kind,
        about: format!("Your own {} preset.", kind.name()),
        tags,
        thumbnail: None,
        order: None,
    };
    preset.extra.insert(
        "preset".to_owned(),
        serde_json::to_value(&info).expect("plain data"),
    );
    preset
}

/// Saves `description` as a person's own preset of `kind` named `name` in
/// `folder`'s `presets/` (a new file: an existing preset is never written
/// over). Returns its ID and the preset.
pub fn save_own(
    folder: &Path,
    description: &Description,
    kind: PresetKind,
    name: &str,
    library: &Library,
    own: &BTreeMap<String, Description>,
) -> Result<(String, Description), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Give the preset a name to save it.".to_owned());
    }
    let dir = folder.join(OWN_PRESETS_FOLDER);
    let stem = slug(name);
    let mut n = 1;
    let (id, file) = loop {
        let candidate = if n == 1 {
            stem.clone()
        } else {
            format!("{stem}-{n}")
        };
        let id = format!("{OWN_NAMESPACE}/{candidate}");
        let file = dir.join(format!("{candidate}{PRESET_SUFFIX}"));
        if !file.exists() && !own.contains_key(&id) && library.preset(&id).is_none() {
            break (id, file);
        }
        n += 1;
    };
    let preset = make(description, kind, name, &id, library);
    if let Some(problem) = check_preset(&preset, library).first() {
        return Err(format!(
            "Could not save the preset: {} {}",
            problem.at, problem.message
        ));
    }
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not make {}: {err}", dir.display()))?;
    std::fs::write(&file, preset.to_json())
        .map_err(|err| format!("Could not save {}: {err}", file.display()))?;
    Ok((id, preset))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FavouritesFile {
    format_version: u32,
    presets: Vec<String>,
}

/// The favourite presets' IDs in `folder` (none when there's no file).
pub fn read_favourites(folder: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(folder.join(FAVOURITES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<FavouritesFile>(&text).ok())
        .filter(|file| file.format_version <= FAVOURITES_VERSION)
        .map(|file| file.presets.into_iter().collect())
        .unwrap_or_default()
}

/// Writes the favourites of `folder`.
pub fn write_favourites(folder: &Path, favourites: &BTreeSet<String>) {
    let file = FavouritesFile {
        format_version: FAVOURITES_VERSION,
        presets: favourites.iter().cloned().collect(),
    };
    let text = serde_json::to_string_pretty(&file).expect("plain data");
    if let Err(err) = std::fs::write(folder.join(FAVOURITES_FILE), text) {
        log::warn!("could not write Character Studio's favourites: {err}");
    }
}
