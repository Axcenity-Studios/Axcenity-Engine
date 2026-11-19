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
/// files it names (models, a groom's cards, strands and mesh, thumbnails,
/// variants' pictures), and each preset file (`*.preset.json`) and its
/// picture. A file is copied when it's missing or differs from the shipped
/// one (in size or in its bytes), so a newer engine's package replaces an
/// older copy, file by file, and an unchanged one isn't copied again
/// (audit D2: a package the folder already had used to be skipped, so an
/// old `Realistic.base.json` stayed for good). The project's own files,
/// which the shipped packages haven't, are never touched, and neither is
/// an ID the project has at a path of its own (its entry wins). The
/// catalogue is written again with what the project's library had and
/// what was copied. Returns the IDs of the base models, items and presets
/// copied in or brought up to date; what `from` can't read is skipped.
///
/// Character Studio's workspace calls it for its own folder, so it offers
/// every family the engine ships, not only the one built in (the realistic
/// body's model is 32 MB, too big to build into the program).
pub fn add_character_packages(project: &Path, from: &Path) -> Result<Vec<String>, SaveError> {
    let (shipped, _, _) = Library::read(from);
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
        if copied || library.base(id) != Some(base) {
            library.add_base(manifest, base.clone());
            added.push(id.clone());
        }
    }
    for (id, item) in &shipped.items {
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
            .chain(item.variants.iter().filter_map(|v| v.thumbnail.as_deref()))
            // An imported item's provenance record travels with it, so a
            // game made from the project credits it (decision 11).
            .chain(item.source.as_ref().and_then(|s| s.provenance.as_deref()));
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
    for (id, preset) in &shipped.presets {
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
        let folder = axcenity_test_dirs::TestDir::adopt(std::env::temp_dir().join(format!(
            "axcenity-character-library-test-{}",
            std::process::id()
        )));
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
        let folder = axcenity_test_dirs::TestDir::adopt(std::env::temp_dir().join(format!(
            "axcenity-character-packages-test-{}",
            std::process::id()
        )));
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
        // The presets come too, file for file, and the library is sound.
        assert!(!repository.presets.is_empty());
        assert_eq!(library.presets, repository.presets);
        for id in repository.presets.keys() {
            let file = repository.preset_path(id).unwrap();
            assert_eq!(library.preset_path(id), Some(file));
            assert_eq!(
                fs::read(root.join(file)).unwrap(),
                fs::read(shipped.join(file)).unwrap()
            );
        }
        assert!(library.validate().is_empty(), "{:?}", library.validate());

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
        let _ = fs::remove_dir_all(&folder);
    }

    /// Audit D2: a folder that already has a shipped package (an older
    /// copy) gets the shipped files again where they differ, even at the
    /// same size; a project's own files, which the shipped package hasn't,
    /// are left alone, and so are its own catalogue entries.
    #[test]
    fn an_older_copy_of_a_shipped_package_is_brought_up_to_date() {
        let folder = axcenity_test_dirs::TestDir::adopt(std::env::temp_dir().join(format!(
            "axcenity-character-packages-update-{}",
            std::process::id()
        )));
        let _ = fs::remove_dir_all(&folder);
        add_character_library(&folder).unwrap();
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters");
        add_character_packages(&folder, &shipped).unwrap();
        let root = folder.join(CHARACTER_LIBRARY_PATH);
        let (repository, _, _) = Library::read(&shipped);
        let manifest = repository.manifest_path("axcenity/realistic").unwrap();
        let preset_id = repository.presets.keys().next().unwrap().clone();
        let preset_file = repository.preset_path(&preset_id).unwrap();

        // An older engine's copies: the base manifest with its label
        // changed at the same size (one letter), and a preset's about.
        let shipped_text = fs::read_to_string(shipped.join(manifest)).unwrap();
        let older = shipped_text.replacen("\"Realistic\"", "\"Realistix\"", 1);
        assert_ne!(older, shipped_text, "the label is in the manifest");
        assert_eq!(older.len(), shipped_text.len());
        fs::write(root.join(manifest), &older).unwrap();
        let preset_text = fs::read_to_string(shipped.join(preset_file)).unwrap();
        fs::write(
            root.join(preset_file),
            preset_text.replacen("\"about\": \"", "\"about\": \"Older. ", 1),
        )
        .unwrap();
        // The project's own files: an item of its own, listed in its
        // catalogue, and a note beside the package.
        let briefs = repository
            .manifest_path("axcenity/realistic-briefs")
            .unwrap();
        let mut item: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(shipped.join(briefs)).unwrap()).unwrap();
        item["id"] = "studio-x/briefs".into();
        item["label"] = "Our briefs".into();
        // Beside the shipped item, so its paths still resolve.
        let own_path = briefs.replace(".item.json", "-ours.item.json");
        let own_text = serde_json::to_string_pretty(&item).unwrap();
        fs::write(root.join(&own_path), &own_text).unwrap();
        let (mut library, _, _) = Library::read(&root);
        library.add_item(&own_path, ItemManifest::from_json(&own_text).unwrap());
        fs::write(
            root.join("catalogue.json"),
            Catalogue::of(&library).to_json(),
        )
        .unwrap();
        fs::write(root.join("realistic/NOTES.txt"), "ours").unwrap();

        let updated = add_character_packages(&folder, &shipped).unwrap();
        assert!(
            updated.iter().any(|id| id == "axcenity/realistic"),
            "{updated:?}"
        );
        assert!(updated.contains(&preset_id), "{updated:?}");
        assert_eq!(
            fs::read_to_string(root.join(manifest)).unwrap(),
            shipped_text,
            "the shipped manifest replaced the older one"
        );
        assert_eq!(
            fs::read(root.join(preset_file)).unwrap(),
            fs::read(shipped.join(preset_file)).unwrap()
        );
        let (library, _, problems) = Library::read(&root);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            library.base("axcenity/realistic").unwrap().label,
            "Realistic",
            "the library reads the new manifest"
        );
        assert_eq!(
            library.preset(&preset_id),
            repository.preset(&preset_id),
            "and the new preset"
        );
        assert!(
            library.item("studio-x/briefs").is_some(),
            "the project's own item stays in the catalogue"
        );
        assert_eq!(fs::read_to_string(root.join(&own_path)).unwrap(), own_text);
        assert_eq!(
            fs::read_to_string(root.join("realistic/NOTES.txt")).unwrap(),
            "ours"
        );
        // Up to date now: nothing to copy.
        assert!(
            add_character_packages(&folder, &shipped)
                .unwrap()
                .is_empty()
        );
        let _ = fs::remove_dir_all(&folder);
    }
    /// Paul's decision 11: an imported item's provenance record travels
    /// with it into a project, so the game's credits can be made there.
    #[test]
    fn an_imported_items_provenance_record_is_copied_with_it() {
        let root = axcenity_test_dirs::TestDir::adopt(std::env::temp_dir().join(format!(
            "axcenity-character-packages-provenance-{}",
            std::process::id()
        )));
        let _ = fs::remove_dir_all(&root);
        let shipped = root.join("shipped");
        fs::create_dir_all(shipped.join("hats")).unwrap();
        let item = r#"{ "format_version": 1, "id": "imported/hat", "label": "Hat",
            "kind": "accessory", "slot": "hair", "bases": ["axcenity/realistic"],
            "model": { "file": "Hat.glb" }, "fit": { "method": "binding" },
            "source": { "author": "A. Modeller", "licence": "CC0-1.0", "origin": "imported",
                        "provenance": "Hat.provenance.json" } }"#;
        fs::write(shipped.join("hats/Hat.item.json"), item).unwrap();
        fs::write(shipped.join("hats/Hat.glb"), b"model").unwrap();
        fs::write(
            shipped.join("hats/Hat.provenance.json"),
            b"{ \"record\": 1 }",
        )
        .unwrap();
        let mut library = Library::new();
        library.add_item("hats/Hat.item.json", ItemManifest::from_json(item).unwrap());
        fs::write(
            shipped.join("catalogue.json"),
            Catalogue::of(&library).to_json(),
        )
        .unwrap();
        let project = root.join("project");
        let added = add_character_packages(&project, &shipped).unwrap();
        assert_eq!(added, ["imported/hat"]);
        let copied = project.join(CHARACTER_LIBRARY_PATH).join("hats");
        assert!(copied.join("Hat.glb").is_file());
        assert_eq!(
            fs::read(copied.join("Hat.provenance.json")).unwrap(),
            fs::read(shipped.join("hats/Hat.provenance.json")).unwrap()
        );
        let _ = fs::remove_dir_all(&root);
    }
}
