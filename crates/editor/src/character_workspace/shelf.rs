//! The saved characters as the start page shows them: listed with their
//! descriptions and a short overview (body type, hair, what they wear),
//! and deleted with everything that names them (the picture, Recent,
//! packs).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use axcenity_assets::Assets;

use axcenity_avatar::{Category, Description, Library, chosen_slots, wear_choices};

use super::{CHARACTERS_FOLDER, packs, recent, thumbnails};

/// A saved character file.
#[derive(Clone, Debug)]
pub struct SavedCharacter {
    /// The character's name (the file's when it can't be read).
    pub name: String,
    pub path: PathBuf,
    /// The file's text, which keys its picture.
    pub text: String,
    /// `None` when the file can't be read (damaged, or a newer format).
    pub description: Option<Description>,
    pub modified: Option<SystemTime>,
}

impl SavedCharacter {
    /// Reads the character file at `path`.
    pub fn read(path: &Path) -> Option<Self> {
        let file_name = path.file_name()?.to_str()?;
        let stem = file_name.strip_suffix(".character.json")?;
        let text = std::fs::read_to_string(path).ok()?;
        let description = Description::from_json(&text).ok();
        Some(Self {
            name: description
                .as_ref()
                .map_or_else(|| stem.to_owned(), |description| description.name.clone()),
            path: path.to_owned(),
            text,
            description,
            modified: std::fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok(),
        })
    }

    /// The file's name in `characters/` (what Recent and packs name).
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned()
    }
}

/// Every saved character in `folder`, newest first.
pub fn list(folder: &Path) -> Vec<SavedCharacter> {
    let Ok(entries) = std::fs::read_dir(folder.join(CHARACTERS_FOLDER)) else {
        return Vec::new();
    };
    let mut found: Vec<SavedCharacter> = entries
        .flatten()
        .filter_map(|entry| SavedCharacter::read(&entry.path()))
        .collect();
    found.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    found
}

/// The character library the start page describes characters with: the
/// workspace's (written the first time it opens), or until then the one
/// the engine ships.
pub fn library_for(folder: &Path) -> Arc<Library> {
    let library = axcenity_gameplay::description::library(&Assets::new(folder));
    if library.base(super::DEFAULT_BASE).is_some() {
        return library;
    }
    // `find_character_library` is `<root>/assets/characters`.
    super::find_character_library()
        .as_deref()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(|root| axcenity_gameplay::description::library(&Assets::new(root)))
        .filter(|shipped| shipped.base(super::DEFAULT_BASE).is_some())
        .unwrap_or(library)
}

/// A character in a few words, for its card.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Overview {
    /// Its body: "Female · Realistic · 4 values set".
    pub body: String,
    /// Its hair and what it wears: "Bald · Boxer briefs".
    pub look: String,
}

/// The overview of `description` on `library`'s base models and items: of
/// the character as it's shown, wearing what its body type wears.
pub fn overview(description: &Description, library: &Library) -> Overview {
    let mut shown = description.clone();
    wear_choices(&mut shown, library);
    let description = &shown;
    let label = |id: &str| {
        library.item(id).map_or_else(
            || id.rsplit('/').next().unwrap_or(id).replace(['-', '_'], " "),
            |item| item.label.clone(),
        )
    };
    let Some(base) = library.base(&description.base) else {
        return Overview {
            body: description.base.clone(),
            look: match description.items.len() {
                0 => "Wears nothing".to_owned(),
                1 => "1 item".to_owned(),
                n => format!("{n} items"),
            },
        };
    };
    let mut body = Vec::new();
    let mut set = 0;
    for parameter in &base.parameters {
        let value = description
            .parameters
            .get(&parameter.id)
            .copied()
            .unwrap_or(parameter.default);
        match parameter.nearest_choice(value) {
            Some(choice) => body.push(choice.label.clone()),
            None if (value - parameter.default).abs() > 1e-3 => set += 1,
            None => {}
        }
    }
    body.push(base.label.clone());
    match set {
        0 => {}
        1 => body.push("1 value set".to_owned()),
        n => body.push(format!("{n} values set")),
    }

    let chosen = chosen_slots(library, &description.base);
    let slot_of = |id: &str| {
        library
            .item(id)
            .and_then(|item| base.slot(&item.slot))
            .cloned()
    };
    let mut hair = Vec::new();
    let mut clothes = Vec::new();
    let mut underwear = Vec::new();
    for worn in &description.items {
        let slot = slot_of(&worn.id);
        match slot.as_ref().map(|slot| slot.category) {
            Some(Category::Hair) => hair.push(label(&worn.id)),
            _ if slot.is_some_and(|slot| chosen.contains(&slot.id)) => {
                underwear.push(label(&worn.id))
            }
            _ => clothes.push(label(&worn.id)),
        }
    }
    let mut look = Vec::new();
    if hair.is_empty() {
        if let Some(slot) = base.slots.iter().find(|s| s.category == Category::Hair) {
            look.push(slot.empty_label.clone().unwrap_or_else(|| "No hair".into()));
        }
    } else {
        look.push(hair.join(", "));
    }
    if !clothes.is_empty() {
        look.push(clothes.join(", "));
    } else if !underwear.is_empty() {
        look.push(underwear.join(", "));
    } else {
        look.push("Wears nothing".to_owned());
    }
    Overview {
        body: body.join(" · "),
        look: look.join(" · "),
    }
}

/// Deletes the saved character at `path` in `folder`: its file, its
/// pictures, its place in Recent and in every pack.
pub fn delete_character(folder: &Path, path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(format!("Could not delete {}: {err}", path.display())),
    }
    thumbnails::remove_character_pictures(folder, path);
    recent::forget(folder, path);
    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
        packs::remove_character(folder, name);
    }
    Ok(())
}
