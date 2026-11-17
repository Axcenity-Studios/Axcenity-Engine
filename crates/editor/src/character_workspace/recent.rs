//! Character Studio's Recent list: the saved characters last opened or
//! saved, newest first. It's kept in `recent.json` in the workspace's
//! folder (beside `characters/`), by file name in `characters/`, so it is
//! per user and moves with the folder. Files that are gone drop out.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::CHARACTERS_FOLDER;

/// The file, in the workspace's folder.
pub const RECENT_FILE: &str = "recent.json";
/// The list's format.
const RECENT_VERSION: u32 = 1;
/// How many characters it keeps.
pub const MAX_RECENT: usize = 24;

#[derive(Debug, Default, Serialize, Deserialize)]
struct RecentFile {
    format_version: u32,
    /// File names in `characters/`, newest first.
    characters: Vec<String>,
}

/// The file name of `file` if it's a saved character in `folder`'s
/// `characters/` (others aren't tracked).
fn tracked_name(folder: &Path, file: &Path) -> Option<String> {
    let name = file.file_name()?.to_str()?;
    let inside = file
        .parent()
        .is_some_and(|parent| same_place(parent, &folder.join(CHARACTERS_FOLDER)));
    (inside && name.ends_with(".character.json")).then(|| name.to_owned())
}

fn same_place(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn read(folder: &Path) -> Vec<String> {
    std::fs::read_to_string(folder.join(RECENT_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<RecentFile>(&text).ok())
        .filter(|file| file.format_version <= RECENT_VERSION)
        .map(|file| file.characters)
        .unwrap_or_default()
}

fn write(folder: &Path, characters: Vec<String>) {
    let file = RecentFile {
        format_version: RECENT_VERSION,
        characters,
    };
    let text = serde_json::to_string_pretty(&file).expect("plain data");
    if let Err(err) = std::fs::write(folder.join(RECENT_FILE), text) {
        log::warn!("could not write Character Studio's recent list: {err}");
    }
}

/// Puts `file` (a saved character in `folder`) first in Recent.
pub fn remember(folder: &Path, file: &Path) {
    let Some(name) = tracked_name(folder, file) else {
        return;
    };
    let mut characters = read(folder);
    characters.retain(|other| *other != name);
    characters.insert(0, name);
    characters.truncate(MAX_RECENT);
    write(folder, characters);
}

/// Takes `file` out of Recent (it was deleted).
pub fn forget(folder: &Path, file: &Path) {
    let Some(name) = file.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    let mut characters = read(folder);
    let before = characters.len();
    characters.retain(|other| other != name);
    if characters.len() != before {
        write(folder, characters);
    }
}

/// The recent characters' files, newest first: only those still there.
pub fn recent(folder: &Path) -> Vec<PathBuf> {
    let characters = folder.join(CHARACTERS_FOLDER);
    read(folder)
        .into_iter()
        .map(|name| characters.join(name))
        .filter(|path| path.is_file())
        .collect()
}
