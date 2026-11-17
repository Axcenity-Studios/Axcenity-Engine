//! Character packs: a named set of saved characters, to keep a cast
//! together, move it to another computer and put it in a game.
//!
//! - A pack is `packs/<name>.pack.json` in the workspace's folder (format
//!   [`PACK_VERSION`]): `{format_version, name, characters}`, the
//!   characters by their file names in `characters/`. A character that's
//!   deleted leaves its packs; one that's missing is skipped.
//! - **Export** writes the pack as one file, `<name>.characterpack.json`
//!   (format [`EXPORT_VERSION`]): `{format_version, kind:
//!   "axcenity.character-pack", name, characters}`, each character its
//!   whole description (format 2, `architecture.md` section 5), so the
//!   file stands alone. **Import** saves each character into
//!   `characters/` (one already there with the same contents is used; a
//!   different one with the same name gets a number) and makes the pack.
//! - **Add to Project** writes the pack's characters into a game project's
//!   [`PROJECT_FOLDER`] (`assets/characters/descriptions/`) beside the
//!   project's character library (section 15), and makes sure the library
//!   has the bodies and items they wear (`project::add_character_library`
//!   and the engine's packages).

use std::path::{Path, PathBuf};

use axcenity_avatar::Description;
use serde::{Deserialize, Serialize};

use super::CHARACTERS_FOLDER;

/// The packs' folder, in the workspace's folder.
pub const PACKS_FOLDER: &str = "packs";
/// A pack file's format.
pub const PACK_VERSION: u32 = 1;
/// An exported pack's format, and what its `kind` says.
pub const EXPORT_VERSION: u32 = 1;
pub const EXPORT_KIND: &str = "axcenity.character-pack";
/// The end of an exported pack's file name.
pub const EXPORT_ENDING: &str = ".characterpack.json";
/// Where Add to Project puts a pack's characters, in the project.
pub const PROJECT_FOLDER: &str = "assets/characters/descriptions";

/// A pack, as saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub format_version: u32,
    pub name: String,
    /// Its characters' file names in `characters/`, in the pack's order.
    pub characters: Vec<String>,
    /// Where it's saved.
    #[serde(skip)]
    pub path: PathBuf,
}

impl Pack {
    /// The pack's characters that are still saved, by path.
    pub fn members(&self, folder: &Path) -> Vec<PathBuf> {
        let characters = folder.join(CHARACTERS_FOLDER);
        self.characters
            .iter()
            .map(|name| characters.join(name))
            .filter(|path| path.is_file())
            .collect()
    }
}

/// An exported pack: the characters in full.
#[derive(Debug, Serialize, Deserialize)]
struct Exported {
    format_version: u32,
    kind: String,
    name: String,
    characters: Vec<serde_json::Value>,
}

fn pack_path(folder: &Path, name: &str) -> Option<PathBuf> {
    let file = crate::character_studio::file_name(name)?;
    Some(folder.join(PACKS_FOLDER).join(format!("{file}.pack.json")))
}

/// Every pack in `folder`, by name.
pub fn packs(folder: &Path) -> Vec<Pack> {
    let Ok(entries) = std::fs::read_dir(folder.join(PACKS_FOLDER)) else {
        return Vec::new();
    };
    let mut found: Vec<Pack> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".pack.json"))
        })
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let mut pack: Pack = serde_json::from_str(&text).ok()?;
            if pack.format_version > PACK_VERSION {
                log::warn!("skipped {}: a newer pack format", path.display());
                return None;
            }
            pack.path = path;
            Some(pack)
        })
        .collect();
    found.sort_by_key(|pack| pack.name.to_lowercase());
    found
}

/// Saves a pack named `name` of `characters` (file names in
/// `characters/`). `previous` is the pack being edited, if it is one: a
/// new name renames it. Returns the pack as saved.
pub fn save_pack(
    folder: &Path,
    previous: Option<&Path>,
    name: &str,
    characters: Vec<String>,
) -> Result<Pack, String> {
    let name = name.trim();
    let path = pack_path(folder, name).ok_or_else(|| "Give the pack a name.".to_owned())?;
    if characters.is_empty() {
        return Err("Pick at least one character for the pack.".to_owned());
    }
    let renamed = previous.is_none_or(|previous| previous != path);
    if renamed && path.exists() {
        return Err(format!("There's already a pack called \"{name}\"."));
    }
    std::fs::create_dir_all(folder.join(PACKS_FOLDER))
        .map_err(|err| format!("Could not make the packs folder: {err}"))?;
    let pack = Pack {
        format_version: PACK_VERSION,
        name: name.to_owned(),
        characters,
        path: path.clone(),
    };
    let text = serde_json::to_string_pretty(&pack).expect("plain data");
    std::fs::write(&path, text)
        .map_err(|err| format!("Could not save {}: {err}", path.display()))?;
    if let Some(previous) = previous
        && previous != path
    {
        let _ = std::fs::remove_file(previous);
    }
    Ok(pack)
}

/// Deletes a pack (its characters are kept).
pub fn delete_pack(pack: &Pack) -> Result<(), String> {
    std::fs::remove_file(&pack.path)
        .map_err(|err| format!("Could not delete {}: {err}", pack.path.display()))
}

/// Takes the character saved as `file_name` out of every pack.
pub fn remove_character(folder: &Path, file_name: &str) {
    for mut pack in packs(folder) {
        let before = pack.characters.len();
        pack.characters.retain(|name| name != file_name);
        if pack.characters.len() != before {
            let text = serde_json::to_string_pretty(&pack).expect("plain data");
            if let Err(err) = std::fs::write(&pack.path, text) {
                log::warn!("could not update {}: {err}", pack.path.display());
            }
        }
    }
}

/// The pack's characters that can be read, in its order.
fn descriptions(folder: &Path, pack: &Pack) -> Vec<Description> {
    pack.members(folder)
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(|text| Description::from_json(&text).ok())
        .collect()
}

/// The exported file's name for `pack`.
pub fn export_file_name(pack: &Pack) -> String {
    let name = crate::character_studio::file_name(&pack.name).unwrap_or_else(|| "Pack".into());
    format!("{name}{EXPORT_ENDING}")
}

/// Writes `pack` with its characters in full to `to`.
pub fn export_pack(folder: &Path, pack: &Pack, to: &Path) -> Result<(), String> {
    let characters = descriptions(folder, pack)
        .iter()
        .map(|description| {
            serde_json::from_str(&description.to_json()).expect("a description is JSON")
        })
        .collect();
    let exported = Exported {
        format_version: EXPORT_VERSION,
        kind: EXPORT_KIND.to_owned(),
        name: pack.name.clone(),
        characters,
    };
    let text = serde_json::to_string_pretty(&exported).expect("plain data");
    std::fs::write(to, text).map_err(|err| format!("Could not write {}: {err}", to.display()))
}

/// The first of `name`, "`name` 2", "`name` 3"… for which `taken` is false.
fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} {n}"))
        .find(|candidate| !taken(candidate))
        .expect("some number is free")
}

/// Saves `description` into `folder`'s `characters/`, or finds it there
/// already. A different character saved under its name makes it "`name`
/// 2" (and so on). Returns its file name.
fn save_character(folder: &Path, description: Description) -> Result<String, String> {
    let characters = folder.join(CHARACTERS_FOLDER);
    let file_of = |name: &str| {
        crate::character_studio::file_name(name).map(|file| format!("{file}.character.json"))
    };
    let renamed = |name: &str| {
        let mut renamed = description.clone();
        renamed.name = name.to_owned();
        renamed
    };
    // A name is taken by a file there that's another character.
    let taken = |name: &str| {
        let Some(file) = file_of(name) else {
            return true;
        };
        let path = characters.join(file);
        path.exists()
            && std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| Description::from_json(&text).ok())
                .is_none_or(|saved| saved != renamed(name))
    };
    let name = free_name(&description.name, taken);
    let file = file_of(&name).ok_or_else(|| format!("\"{name}\" can't be a file name."))?;
    let path = characters.join(&file);
    if !path.exists() {
        std::fs::write(&path, renamed(&name).to_json())
            .map_err(|err| format!("Could not save {}: {err}", path.display()))?;
    }
    Ok(file)
}

/// Reads an exported pack from `from`, saves its characters into
/// `folder` and makes the pack (named after the file's, with a number if
/// that's taken).
pub fn import_pack(folder: &Path, from: &Path) -> Result<Pack, String> {
    let text = std::fs::read_to_string(from)
        .map_err(|err| format!("Could not read {}: {err}", from.display()))?;
    let exported: Exported = serde_json::from_str(&text)
        .map_err(|err| format!("{} isn't a character pack: {err}", from.display()))?;
    if exported.kind != EXPORT_KIND {
        return Err(format!("{} isn't a character pack.", from.display()));
    }
    if exported.format_version > EXPORT_VERSION {
        return Err(format!(
            "It was saved by a newer Axcenity (character pack format {}; this version reads up \
             to {EXPORT_VERSION}).",
            exported.format_version
        ));
    }
    let descriptions = exported
        .characters
        .iter()
        .map(|value| Description::from_json(&value.to_string()).map_err(|err| err.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    std::fs::create_dir_all(folder.join(CHARACTERS_FOLDER))
        .map_err(|err| format!("Could not make the characters folder: {err}"))?;
    let mut files = Vec::new();
    for description in descriptions {
        let file = save_character(folder, description)?;
        if !files.contains(&file) {
            files.push(file);
        }
    }
    let name = exported.name.trim();
    let name = if name.is_empty() {
        "Imported pack"
    } else {
        name
    };
    let name = free_name(name, |candidate| {
        pack_path(folder, candidate).is_none_or(|path| path.exists())
    });
    save_pack(folder, None, &name, files)
}

/// Writes `pack`'s characters into the game project in `project`
/// ([`PROJECT_FOLDER`]), replacing ones of the same name, and gives the
/// project's character library what they wear. Returns the files written.
pub fn add_to_project(folder: &Path, pack: &Pack, project: &Path) -> Result<Vec<PathBuf>, String> {
    if axcenity_project::read_manifest(project).is_none() {
        return Err(format!("{} isn't an Axcenity project.", project.display()));
    }
    let descriptions = descriptions(folder, pack);
    if descriptions.is_empty() {
        return Err(format!("\"{}\" has no characters to add.", pack.name));
    }
    axcenity_project::add_character_library(project)
        .map_err(|err| format!("Could not write the project's character library: {err}"))?;
    if let Some(shipped) = super::find_character_library() {
        axcenity_project::add_character_packages(project, &shipped)
            .map_err(|err| format!("Could not copy the character library: {err}"))?;
    }
    let target = project.join(PROJECT_FOLDER);
    std::fs::create_dir_all(&target)
        .map_err(|err| format!("Could not make {}: {err}", target.display()))?;
    let mut written = Vec::new();
    for description in descriptions {
        let Some(file) = crate::character_studio::file_name(&description.name) else {
            continue;
        };
        let path = target.join(format!("{file}.character.json"));
        std::fs::write(&path, description.to_json())
            .map_err(|err| format!("Could not write {}: {err}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}
