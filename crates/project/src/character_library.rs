//! The character library a project ships (Character Studio overhaul CO3,
//! `docs/development/character-studio/architecture.md` section 15): the
//! folder `assets/characters/` with a `catalogue.json`, base model packages
//! and their items, which a character description names by ID and
//! `Humanoid:ApplyDescription` resolves against.
//!
//! The engine ships one family, the human base (MakeHuman's body,
//! `assets/characters/human`), in a folder beside the program
//! ([`shipped_character_library`]): its model and wardrobe are too big to
//! build into the program, and a project gets only what its characters
//! name ([`add_character_packages_for`]). Character Studio's own folder
//! takes all of it ([`add_character_packages`]).

use std::path::Path;

use axcenity_avatar::{Catalogue, Description, Library};

use super::*;

/// The character library's folder in a project.
pub const CHARACTER_LIBRARY_PATH: &str = axcenity_avatar::LIBRARY_FOLDER;

/// Gives `project` the default game character's family from the engine's
/// shipped library ([`add_game_character_library`]), when that is found;
/// without it the project's library stays as it is.
pub fn add_character_library(project: &Path) -> Result<(), SaveError> {
    add_game_character_library(project, shipped_character_library().as_deref()).map(|_| ())
}

/// Copies what `descriptions` name from the library in `from` into
/// `project`'s library: each one's base model (with its region map and
/// the colour maps its materials can draw) and the items it wears, and
/// the base's default items. Nothing else of `from` is copied (no other
/// item, no preset), so a project holds what its characters need and not
/// the whole wardrobe. Files are compared and kept as
/// [`add_character_packages`] does. Returns the IDs copied in or brought
/// up to date.
pub fn add_character_packages_for(
    project: &Path,
    from: &Path,
    descriptions: &[&Description],
) -> Result<Vec<String>, SaveError> {
    let (shipped, _, _) = Library::read(from);
    let mut wanted = std::collections::BTreeSet::new();
    for description in descriptions {
        wanted.insert(description.base.clone());
        wanted.extend(description.items.iter().map(|worn| worn.id.clone()));
        if let Some(base) = shipped.base(&description.base) {
            wanted.extend(base.defaults.items.iter().cloned());
            // What a parameter's choices put on (the underwear that
            // follows the body type).
            for parameter in &base.parameters {
                for choice in &parameter.choices {
                    wanted.extend(choice.items.iter().cloned());
                }
            }
        }
    }
    copy_packages(project, from, &shipped, |id| wanted.contains(id), false)
}

/// Copies the character packages of the library in `from` (a folder with
/// a `catalogue.json`, such as the engine's shipped `assets/characters/`)
/// into `project`'s library: each base model's and item's manifest and the
/// files it names (models, a groom's cards, strands and mesh, thumbnails,
/// variants' pictures), and each preset file (`*.preset.json`) and its
/// picture. A file is copied when it's missing or differs from the shipped
/// one (in size or in its bytes), so a newer engine's package replaces an
/// older copy, file by file, and an unchanged one isn't copied again
/// (audit D2: a package the folder already had used to be skipped, so an
/// old base manifest stayed for good). The project's own files,
/// which the shipped packages haven't, are never touched, and neither is
/// an ID the project has at a path of its own (its entry wins). The
/// catalogue is written again with what the project's library had and
/// what was copied. Returns the IDs of the base models, items and presets
/// copied in or brought up to date; what `from` can't read is skipped.
///
/// Character Studio's workspace calls it for its own folder, so it offers
/// everything the engine ships.
pub fn add_character_packages(project: &Path, from: &Path) -> Result<Vec<String>, SaveError> {
    let (shipped, _, _) = Library::read(from);
    copy_packages(project, from, &shipped, |_| true, true)
}

/// [`add_character_packages`] for the bases and items `wanted` names, with
/// the presets or without.
fn copy_packages(
    project: &Path,
    from: &Path,
    shipped: &Library,
    wanted: impl Fn(&str) -> bool,
    presets: bool,
) -> Result<Vec<String>, SaveError> {
    let root = project.join(CHARACTER_LIBRARY_PATH);
    let (mut library, _, _) = Library::read(&root);
    // Copies `path` when the project's copy is missing or differs; says
    // whether it did.
    let copy = |path: &str| -> Result<bool, SaveError> {
        let source = from.join(path);
        let target = root.join(path);
        if !source.is_file() || same_contents(&source, &target) {
            return Ok(false);
        }
        if let Some(folder) = target.parent() {
            fs::create_dir_all(folder).map_err(SaveError::new(folder))?;
        }
        fs::copy(&source, &target)
            .map(|_| true)
            .map_err(SaveError::new(&target))
    };
    // A shipped ID the project has at a path of its own is the project's.
    let own = |listed: Option<&str>, shipped: &str| listed.is_some_and(|path| path != shipped);
    let mut added = Vec::new();
    for (id, base) in &shipped.bases {
        if !wanted(id) {
            continue;
        }
        let Some(manifest) = shipped.manifest_path(id) else {
            continue;
        };
        if own(library.manifest_path(id), manifest) {
            continue;
        }
        let mut copied = copy(manifest)?;
        if let Some(model) = shipped.path_of(id, &base.model.file) {
            copied |= copy(&model)?;
        }
        // The region map hides the skin a worn item covers (CO9).
        let map = base.model.regions_map.as_deref();
        if let Some(map) = map.and_then(|map| shipped.path_of(id, map)) {
            copied |= copy(&map)?;
        }
        // The colour maps its materials can draw (its skins: ADR-0048).
        let mut maps: Vec<&str> = base
            .material_maps
            .iter()
            .map(|map| map.colour_map.as_str())
            .collect();
        maps.sort_unstable();
        maps.dedup();
        for map in maps {
            if let Some(map) = shipped.path_of(id, map) {
                copied |= copy(&map)?;
            }
        }
        if copied || library.base(id) != Some(base) {
            library.add_base(manifest, base.clone());
            added.push(id.clone());
        }
    }
    for (id, item) in &shipped.items {
        if !wanted(id) {
            continue;
        }
        let Some(manifest) = shipped.manifest_path(id) else {
            continue;
        };
        if own(library.manifest_path(id), manifest) {
            continue;
        }
        let mut copied = copy(manifest)?;
        let groom = item.groom.as_ref();
        let files = item
            .model
            .iter()
            .chain(groom.and_then(|groom| groom.cards.as_ref()))
            .chain(groom.and_then(|groom| groom.mesh.as_ref()))
            .map(|model| model.file.as_str())
            .chain(groom.and_then(|groom| groom.strands.as_deref()))
            .chain(item.thumbnail.as_deref())
            // What it hides when worn (ADR-0045).
            .chain(item.cover_map.as_deref())
            .chain(item.variants.iter().filter_map(|v| v.thumbnail.as_deref()));
        for file in files {
            if let Some(path) = shipped.path_of(id, file) {
                copied |= copy(&path)?;
            }
        }
        if copied || library.item(id) != Some(item) {
            library.add_item(manifest, item.clone());
            added.push(id.clone());
        }
    }
    for (id, preset) in shipped.presets.iter().filter(|_| presets) {
        let Some(file) = shipped.preset_path(id) else {
            continue;
        };
        if own(library.preset_path(id), file) {
            continue;
        }
        let mut copied = copy(file)?;
        let picture = preset.preset().and_then(|block| block.thumbnail);
        if let Some(path) = picture.and_then(|p| axcenity_avatar::resolve_path(file, &p)) {
            copied |= copy(&path)?;
        }
        if (copied || library.preset(id) != Some(preset))
            && library.add_preset(file, preset.clone()).is_ok()
        {
            added.push(id.clone());
        }
    }
    if !added.is_empty() {
        let catalogue = root.join("catalogue.json");
        fs::create_dir_all(&root).map_err(SaveError::new(&root))?;
        fs::write(&catalogue, Catalogue::of(&library).to_json())
            .map_err(SaveError::new(&catalogue))?;
    }
    Ok(added)
}

/// Whether files `a` and `b` hold the same bytes (a missing one doesn't).
/// The sizes are compared first, so a changed model is usually told
/// without reading it.
fn same_contents(a: &Path, b: &Path) -> bool {
    use std::io::Read;
    let size = |file: &Path| fs::metadata(file).ok().map(|meta| meta.len());
    if size(a).is_none() || size(a) != size(b) {
        return false;
    }
    let (Ok(mut a), Ok(mut b)) = (fs::File::open(a), fs::File::open(b)) else {
        return false;
    };
    let (mut left, mut right) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    loop {
        // Whatever `a` gives, exactly as much is read from `b`, so the two
        // stay in step.
        let n = match a.read(&mut left) {
            Ok(0) => return b.read(&mut right).is_ok_and(|m| m == 0),
            Ok(n) => n,
            Err(_) => return false,
        };
        if b.read_exact(&mut right[..n]).is_err() || left[..n] != right[..n] {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const HUMAN: &str = "axcenity/human";

    fn shipped() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
    }

    fn temp(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-character-library-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A character's packages: its base with the files it draws, what it
    /// wears, and nothing else of the library.
    #[test]
    fn a_project_gets_what_its_characters_name_and_no_more() {
        let project = temp("for");
        let (whole, _, problems) = Library::read(&shipped());
        assert!(problems.is_empty(), "{problems:?}");
        let mut description = Description::new("Ada", HUMAN);
        description
            .items
            .push(axcenity_avatar::WornItem::new("makehuman/shoes01"));
        let added = add_character_packages_for(&project, &shipped(), &[&description]).unwrap();
        assert!(added.iter().any(|id| id == HUMAN), "{added:?}");
        assert!(added.iter().any(|id| id == "makehuman/shoes01"));

        let root = project.join(CHARACTER_LIBRARY_PATH);
        let (library, catalogue, problems) = Library::read(&root);
        assert!(problems.is_empty(), "{problems:?}");
        assert!(catalogue.is_some());
        assert!(library.validate().is_empty(), "{:?}", library.validate());
        let base = library.base(HUMAN).expect("copied");
        let model = library.path_of(HUMAN, &base.model.file).unwrap();
        assert!(root.join(&model).is_file(), "{model}");
        // Its skins come with it (ADR-0048), and the underwear its body
        // types put on.
        for map in &base.material_maps {
            let path = library.path_of(HUMAN, &map.colour_map).unwrap();
            assert!(root.join(&path).is_file(), "{path}");
        }
        for choice in base.parameters.iter().flat_map(|p| &p.choices) {
            for item in &choice.items {
                assert!(library.item(item).is_some(), "{item}");
            }
        }
        assert!(library.item("makehuman/male-casualsuit01").is_none());
        assert!(library.items.len() < whole.items.len() / 4);
        assert!(library.presets.is_empty());

        // A second time changes nothing.
        let again = add_character_packages_for(&project, &shipped(), &[&description]).unwrap();
        assert!(again.is_empty(), "{again:?}");
        let _ = fs::remove_dir_all(&project);
    }

    /// The whole library, as Character Studio's own folder takes it; an
    /// older copy of a file is brought up to date and the folder's own
    /// files are left alone.
    #[test]
    fn the_whole_library_is_copied_and_an_older_copy_brought_up_to_date() {
        let project = temp("whole");
        let (whole, _, _) = Library::read(&shipped());
        let added = add_character_packages(&project, &shipped()).unwrap();
        assert_eq!(
            added.len(),
            whole.bases.len() + whole.items.len() + whole.presets.len()
        );
        let root = project.join(CHARACTER_LIBRARY_PATH);
        let (library, _, problems) = Library::read(&root);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(library, whole);
        assert!(
            add_character_packages(&project, &shipped())
                .unwrap()
                .is_empty()
        );

        // An older manifest of the base, and a file of the project's own.
        let manifest = root.join(whole.manifest_path(HUMAN).unwrap());
        let text = fs::read_to_string(&manifest).unwrap();
        let label = &whole.base(HUMAN).unwrap().label;
        let older = text.replacen(
            &format!("\"label\": \"{label}\""),
            "\"label\": \"Older\"",
            1,
        );
        assert_ne!(older, text);
        fs::write(&manifest, older).unwrap();
        let notes = root.join("human/NOTES.txt");
        fs::write(&notes, "ours").unwrap();
        let updated = add_character_packages(&project, &shipped()).unwrap();
        assert_eq!(updated, [HUMAN]);
        let (library, _, _) = Library::read(&root);
        assert_eq!(&library.base(HUMAN).unwrap().label, label);
        assert_eq!(fs::read_to_string(&notes).unwrap(), "ours");
        let _ = fs::remove_dir_all(&project);
    }
}
