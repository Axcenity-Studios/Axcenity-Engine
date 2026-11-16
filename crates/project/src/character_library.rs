//! The character library a project ships (Character Studio overhaul CO3,
//! `docs/development/character-studio/architecture.md` section 15): the
//! folder `assets/characters/` with a `catalogue.json`, base model packages
//! and their items, which a character description names by ID and
//! `Humanoid:ApplyDescription` resolves against.
//!
//! The default character's package (the stylized humanoid) is built into
//! the engine, like the default humanoid's model: [`add_character_library`]
//! writes it into a project, files already there kept. Other packages are
//! copied in beside it (the repository's are in `assets/characters/`).

use std::path::Path;

use axcenity_avatar::{BaseManifest, Catalogue, ItemManifest, Library};

use super::default_humanoid::write_missing;
use super::*;

/// The character library's folder in a project.
pub const CHARACTER_LIBRARY_PATH: &str = axcenity_avatar::LIBRARY_FOLDER;

/// The built-in package's manifests, by their path in the library.
const MANIFESTS: [(&str, &str); 13] = {
    macro_rules! file {
        ($path:literal) => {
            (
                $path,
                include_str!(concat!("../../../assets/characters/", $path)),
            )
        };
    }
    [
        file!("humanoid/Humanoid.base.json"),
        file!("humanoid/items/briefs.item.json"),
        file!("humanoid/items/cargo-pants.item.json"),
        file!("humanoid/items/cropped-jacket.item.json"),
        file!("humanoid/items/hair-bob.item.json"),
        file!("humanoid/items/hair-buzz.item.json"),
        file!("humanoid/items/hair-crop.item.json"),
        file!("humanoid/items/hair-long.item.json"),
        file!("humanoid/items/hair-ponytail.item.json"),
        file!("humanoid/items/hair-swept.item.json"),
        file!("humanoid/items/sneakers.item.json"),
        file!("humanoid/items/sports-top.item.json"),
        file!("humanoid/items/tee.item.json"),
    ]
};

/// The built-in package's model, by its path in the library.
const MODEL: &str = "humanoid/HumanoidParts.glb";

/// The character library built into the engine: the default character's
/// base model and its items.
pub fn built_in_character_library() -> Library {
    let mut library = Library::new();
    for (path, text) in MANIFESTS {
        if path.ends_with(".base.json") {
            if let Ok(base) = BaseManifest::from_json(text) {
                library.add_base(path, base);
            }
        } else if let Ok(item) = ItemManifest::from_json(text) {
            library.add_item(path, item);
        }
    }
    library
}

/// Writes the built-in character library into `project`
/// ([`CHARACTER_LIBRARY_PATH`]): its catalogue, manifests and model. Files
/// already there are kept, so a project's own library is never written
/// over; a project that has a catalogue keeps it (add packages to it with
/// Character Studio).
pub fn add_character_library(project: &Path) -> Result<(), SaveError> {
    let write = |path: &str, bytes: &[u8]| {
        write_missing(project, &format!("{CHARACTER_LIBRARY_PATH}/{path}"), bytes)
    };
    for (path, text) in MANIFESTS {
        write(path, text.as_bytes())?;
    }
    write(MODEL, DEFAULT_HUMANOID_PARTS)?;
    let catalogue = Catalogue::of(&built_in_character_library());
    write("catalogue.json", catalogue.to_json().as_bytes())
}

/// Copies the character packages of the library in `from` (a folder with
/// a `catalogue.json`, such as the engine's shipped `assets/characters/`)
/// into `project`'s library: each base model's and item's manifest and the
/// files it names (models, a groom, a thumbnail). A file is copied when
/// it's missing or differs ([`differs`]), so a newer engine's package
/// replaces an older copy, manifests included, and an unchanged one isn't
/// copied again. A base model or item the project's library has from a
/// manifest of its own (another path) is the project's and is left alone.
/// The catalogue is written again when anything was copied. Returns the
/// IDs of the base models and items copied in or brought up to date; what
/// `from` can't read is skipped.
///
/// Character Studio's workspace calls it for its own folder, so it offers
/// every family the engine ships, not only the one built in (the realistic
/// body's model is 32 MB, too big to build into the program), and an
/// engine update reaches characters it already made.
pub fn add_character_packages(project: &Path, from: &Path) -> Result<Vec<String>, SaveError> {
    let (shipped, _, _) = Library::read(from);
    let root = project.join(CHARACTER_LIBRARY_PATH);
    let (mut library, _, _) = Library::read(&root);
    let copy = |path: &str| -> Result<bool, SaveError> {
        let source = from.join(path);
        let target = root.join(path);
        if !source.is_file() || !differs(&source, &target) {
            return Ok(false);
        }
        if let Some(folder) = target.parent() {
            fs::create_dir_all(folder).map_err(SaveError::new(folder))?;
        }
        fs::copy(&source, &target)
            .map(|_| true)
            .map_err(SaveError::new(&target))
    };
    // The project's own entry for `id`, from a manifest elsewhere than the
    // shipped one's: kept.
    let own = |library: &Library, id: &str, manifest: &str| {
        library
            .manifest_path(id)
            .is_some_and(|path| path != manifest)
    };
    let mut added = Vec::new();
    for (id, base) in &shipped.bases {
        let Some(manifest) = shipped.manifest_path(id) else {
            continue;
        };
        if own(&library, id, manifest) {
            continue;
        }
        let mut changed = copy(manifest)?;
        if let Some(model) = shipped.path_of(id, &base.model.file) {
            changed |= copy(&model)?;
        }
        if changed || library.base(id).is_none() {
            library.add_base(manifest, base.clone());
            added.push(id.clone());
        }
    }
    for (id, item) in &shipped.items {
        let Some(manifest) = shipped.manifest_path(id) else {
            continue;
        };
        if own(&library, id, manifest) {
            continue;
        }
        let mut changed = copy(manifest)?;
        let groom = item.groom.as_ref();
        let files = item
            .model
            .iter()
            .chain(groom.and_then(|groom| groom.cards.as_ref()))
            .map(|model| model.file.as_str())
            .chain(groom.and_then(|groom| groom.strands.as_deref()))
            .chain(item.thumbnail.as_deref());
        for file in files {
            if let Some(path) = shipped.path_of(id, file) {
                changed |= copy(&path)?;
            }
        }
        if changed || library.item(id).is_none() {
            library.add_item(manifest, item.clone());
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

/// Whether `target` isn't a copy of `source`: it's missing, its size
/// differs or, for a file small enough to read (a manifest), its contents
/// do. A large file (a model) is compared by size alone, so opening
/// Character Studio doesn't read 36 MB twice.
fn differs(source: &Path, target: &Path) -> bool {
    const READ_UP_TO: u64 = 4 << 20;
    let size = |file: &Path| fs::metadata(file).ok().map(|meta| meta.len());
    match (size(source), size(target)) {
        (Some(a), Some(b)) if a == b => {
            a <= READ_UP_TO && fs::read(source).ok() != fs::read(target).ok()
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository's stylized package, file for file: a manifest added
    /// there must be added to `MANIFESTS` too.
    #[test]
    fn the_built_in_library_is_the_repositorys_stylized_package() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters");
        let (repository, _, problems) = Library::read(&root);
        assert!(problems.is_empty(), "{problems:?}");
        let built_in = built_in_character_library();
        assert_eq!(built_in.bases.len(), 1);
        let base = built_in.base(axcenity_avatar::V1_BASE).expect("stylized");
        assert_eq!(repository.base(&base.id), Some(base));
        let fitting: Vec<&String> = repository
            .items
            .values()
            .filter(|item| item.bases.contains(&base.id))
            .map(|item| &item.id)
            .collect();
        let listed: Vec<&String> = built_in.items.keys().collect();
        assert_eq!(listed, fitting, "every item of the package is built in");
        for (id, item) in &built_in.items {
            assert_eq!(repository.item(id), Some(item));
            assert_eq!(built_in.manifest_path(id), repository.manifest_path(id));
        }
        assert!(built_in.validate().is_empty(), "{:?}", built_in.validate());
        assert_eq!(
            built_in.path_of(&base.id, &base.model.file).as_deref(),
            Some(MODEL)
        );
    }

    #[test]
    fn the_library_written_into_a_project_reads_back_sound() {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-character-library-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        add_character_library(&folder).unwrap();
        let root = folder.join(CHARACTER_LIBRARY_PATH);
        let (library, catalogue, problems) = Library::read(&root);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(library, built_in_character_library());
        assert_eq!(catalogue, Some(Catalogue::of(&library)));
        assert_eq!(fs::read(root.join(MODEL)).unwrap(), DEFAULT_HUMANOID_PARTS);
        // A file already there is kept.
        fs::write(root.join("catalogue.json"), "mine").unwrap();
        add_character_library(&folder).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("catalogue.json")).unwrap(),
            "mine"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_shipped_packages_are_copied_beside_the_built_in_one() {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-character-packages-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        add_character_library(&folder).unwrap();
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters");
        let added = add_character_packages(&folder, &shipped).unwrap();
        assert!(
            added.iter().any(|id| id == "axcenity/realistic"),
            "{added:?}"
        );
        assert!(
            !added.iter().any(|id| id == axcenity_avatar::V1_BASE),
            "the built-in package was already there: {added:?}"
        );

        let root = folder.join(CHARACTER_LIBRARY_PATH);
        let (library, _, problems) = Library::read(&root);
        assert!(problems.is_empty(), "{problems:?}");
        let realistic = library.base("axcenity/realistic").expect("copied");
        let model = library
            .path_of("axcenity/realistic", &realistic.model.file)
            .unwrap();
        assert_eq!(
            fs::metadata(root.join(&model)).unwrap().len(),
            fs::metadata(shipped.join(&model)).unwrap().len()
        );
        assert!(library.base(axcenity_avatar::V1_BASE).is_some());
        let (repository, _, _) = Library::read(&shipped);
        assert_eq!(library.bases.len(), repository.bases.len());
        assert_eq!(library.items.len(), repository.items.len());

        // Again: nothing new to copy, and the catalogue is left alone.
        fs::write(
            root.join("catalogue.json"),
            Catalogue::of(&library).to_json(),
        )
        .unwrap();
        assert!(
            add_character_packages(&folder, &shipped)
                .unwrap()
                .is_empty()
        );

        // An older engine's copy of a package is brought up to date,
        // manifest and all (a folder made before the realistic body had
        // its underwear kept a manifest without it).
        let manifest = library
            .manifest_path("axcenity/realistic")
            .unwrap()
            .to_owned();
        let stale = fs::read_to_string(root.join(&manifest)).unwrap().replacen(
            "\"choices\"",
            "\"old_choices\"",
            1,
        );
        fs::write(root.join(&manifest), stale).unwrap();
        let refreshed = add_character_packages(&folder, &shipped).unwrap();
        assert_eq!(refreshed, ["axcenity/realistic"]);
        assert_eq!(
            fs::read(root.join(&manifest)).unwrap(),
            fs::read(shipped.join(&manifest)).unwrap()
        );

        // A project's own manifest for a shipped ID, elsewhere, is kept.
        let mut catalogue =
            Catalogue::from_json(&fs::read_to_string(root.join("catalogue.json")).unwrap())
                .unwrap();
        let mine = "mine/Realistic.base.json";
        fs::create_dir_all(root.join("mine")).unwrap();
        fs::copy(root.join(&manifest), root.join(mine)).unwrap();
        for entry in &mut catalogue.bases {
            if entry.id == "axcenity/realistic" {
                entry.manifest = mine.to_owned();
            }
        }
        fs::write(root.join("catalogue.json"), catalogue.to_json()).unwrap();
        fs::write(root.join(&manifest), "left alone").unwrap();
        assert!(
            add_character_packages(&folder, &shipped)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            fs::read_to_string(root.join(&manifest)).unwrap(),
            "left alone"
        );
        let _ = fs::remove_dir_all(&folder);
    }
}
