//! The default game character (refinement R10, OPEN-22; since the
//! consolidation of 2026-11-25 the engine has one family, the human base).
//!
//! A new project, and Insert > Character in any project, get the human
//! base in their library, copied from the packages the engine ships
//! (`assets/characters/`, found by [`shipped_character_library`]: the
//! body's model is too big to build into the program) with what the
//! default character wears and nothing else of the wardrobe. The default
//! character is the default humanoid's Model ([`default_character`]) with
//! a description kept in its Humanoid ([`described_character`]), which the
//! Character Runtime applies wherever the world is loaded
//! (`gameplay::description::DescriptionSync`), in the editor's edit mode
//! too. Without the shipped packages (a program run away from its files)
//! the default character is the default humanoid's own model, the same
//! body in its underwear, with no description to change it by.
//!
//! A project's own files are never written over: an older project keeps
//! its `Character` prefab (its players stay what they were) and its
//! library's own packages.

use std::path::{Path, PathBuf};

use axcenity_avatar::{Description, LIBRARY_FOLDER, Library, WornItem};
use axcenity_core::{ClassName, InstanceId, Value, Vector3};

use super::default_humanoid::write_missing;
use super::*;

/// The base model of the default game character: the human base.
pub const DEFAULT_GAME_BASE: &str = axcenity_avatar::DEFAULT_BASE;

/// What the default game character wears over its base's default items,
/// where the library has them: MakeHuman's shirt and jeans, its brown
/// shoes and a short hairstyle.
pub const DEFAULT_GAME_OUTFIT: [&str; 3] = [
    "makehuman/male-casualsuit01",
    "makehuman/shoes01",
    "makehuman/hair-short02",
];

/// The name of the description kept in a described character's Humanoid.
pub const KEPT_DESCRIPTION: &str = "HumanoidDescription";

/// The character library the engine ships (`assets/characters/` with a
/// `catalogue.json`): looked for in the working folder, then beside the
/// program and in each folder above it (an installed engine keeps it
/// next to the program; a build in the repository finds the
/// repository's).
pub fn shipped_character_library() -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(folder) = std::env::current_dir() {
        roots.push(folder);
    }
    if let Ok(program) = std::env::current_exe() {
        roots.extend(program.ancestors().skip(1).map(Path::to_path_buf));
    }
    roots
        .into_iter()
        .map(|root| root.join(LIBRARY_FOLDER))
        .find(|folder| folder.join("catalogue.json").is_file())
}

/// Gives `project` the default game character's family from the library
/// in `shipped`: the human base and what the default character wears
/// ([`add_character_packages_for`]: a project's own files and IDs are
/// kept). Returns the project's library as it is now.
pub fn add_game_character_library(
    project: &Path,
    shipped: Option<&Path>,
) -> Result<Library, SaveError> {
    if let Some(shipped) = shipped {
        let (library, _, _) = Library::read(shipped);
        if let Some(description) = game_character_description(&library) {
            add_character_packages_for(project, shipped, &[&description])?;
        }
    }
    Ok(Library::read(&project.join(LIBRARY_FOLDER)).0)
}

/// The default game character's description: the human base's default
/// character (its default items, every parameter at its default) in
/// [`DEFAULT_GAME_OUTFIT`], named `Character`. `None` when `library`
/// hasn't the base.
pub fn game_character_description(library: &Library) -> Option<Description> {
    let base = library.base(DEFAULT_GAME_BASE)?;
    let mut description = Description::new("Character", DEFAULT_GAME_BASE);
    description.items = base
        .defaults
        .items
        .iter()
        .map(String::as_str)
        .chain(
            DEFAULT_GAME_OUTFIT
                .into_iter()
                .filter(|id| library.item(id).is_some()),
        )
        .map(WornItem::new)
        .collect();
    Some(description)
}

/// The default character ([`default_character`]) wearing `description`:
/// a HumanoidDescription named [`KEPT_DESCRIPTION`] inside its Humanoid
/// holds it (`Base` and `Data`), which the Character Runtime applies
/// where the world is loaded. Until then its `Body` is the default
/// humanoid's model.
pub fn described_character(
    world: &mut World,
    position: Vector3,
    description: &Description,
) -> InstanceId {
    let character = default_character(world, position);
    let humanoid = world
        .find_first_child(character, "Humanoid")
        .expect("default_character makes it");
    let kept = world
        .create(ClassName::HumanoidDescription)
        .expect("not a service");
    world.set_name(kept, KEPT_DESCRIPTION).expect("just made");
    for (property, text) in [
        ("Base", description.base.clone()),
        ("Data", description.to_json()),
    ] {
        world
            .set_property(kept, property, Value::String(text))
            .expect("a HumanoidDescription has it");
    }
    world.set_parent(kept, Some(humanoid)).expect("a Humanoid");
    character
}

/// Writes the default humanoid's files into `project` (see
/// [`add_default_humanoid`]) with the human base from `shipped`
/// ([`add_game_character_library`]), and a `Character` prefab of the
/// default game character: the described one when the library has the
/// base, else the default humanoid as it is. Files already there are
/// kept, so an older project's prefab stays as it is. Returns the
/// prefab's path.
pub fn add_default_game_character(
    project: &Path,
    shipped: Option<&Path>,
) -> Result<&'static str, SaveError> {
    let library = add_game_character_library(project, shipped)?;
    // The prefab first: `add_default_humanoid` keeps a file already there,
    // so it writes the bare one only when the described one isn't.
    if let Some(description) = game_character_description(&library) {
        let mut world = World::new();
        let character = described_character(&mut world, Vector3::default(), &description);
        write_missing(
            project,
            DEFAULT_CHARACTER_PREFAB_PATH,
            world_file::write_prefab(&world, &[character]).as_bytes(),
        )?;
    }
    add_default_humanoid(project)
}

/// The HumanoidDescription kept in `character`'s Humanoid, if any.
#[cfg(test)]
fn kept_description(world: &World, character: InstanceId) -> Option<InstanceId> {
    let humanoid = world.find_first_child(character, "Humanoid")?;
    world.get(humanoid)?.children().iter().copied().find(|id| {
        world
            .get(*id)
            .is_some_and(|held| held.class() == ClassName::HumanoidDescription)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-game-character-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn shipped() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
    }

    /// The `Character` prefab's Model, read back from the project.
    fn prefab(folder: &Path) -> (World, InstanceId) {
        let mut world = World::new();
        let workspace = world.workspace();
        let roots = crate::Prefabs::in_folder(folder)
            .instantiate(DEFAULT_CHARACTER_PREFAB_PATH, &mut world, workspace)
            .unwrap();
        (world, roots[0])
    }

    fn base_kept(world: &World, character: InstanceId) -> Option<String> {
        let kept = kept_description(world, character)?;
        world
            .get(kept)?
            .property("Base")
            .and_then(Value::as_str)
            .map(str::to_owned)
    }

    #[test]
    fn the_engines_library_is_found_from_a_test() {
        let found = shipped_character_library().expect("found beside the test program");
        assert!(found.join("human").is_dir(), "{}", found.display());
    }

    /// A new project has the human base and what its players wear, not
    /// the whole wardrobe, and its players are that character.
    #[test]
    fn a_new_project_gets_the_human_base_and_the_default_characters_clothes() {
        let parent = temp("new-project");
        let folder = create_project_from(&parent, "Human Game", Template::Baseplate).unwrap();
        let (library, _, problems) = Library::read(&folder.join(LIBRARY_FOLDER));
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(library.bases.len(), 1);
        assert!(library.base(DEFAULT_GAME_BASE).is_some(), "human");
        assert!(library.validate().is_empty(), "{:?}", library.validate());
        for item in DEFAULT_GAME_OUTFIT {
            assert!(library.item(item).is_some(), "{item}");
        }
        let (whole, _, _) = Library::read(&shipped());
        assert!(
            library.items.len() < whole.items.len() / 4,
            "{} of {} items",
            library.items.len(),
            whole.items.len()
        );
        assert!(library.presets.is_empty());

        let (world, character) = prefab(&folder);
        assert_eq!(
            base_kept(&world, character).as_deref(),
            Some(DEFAULT_GAME_BASE)
        );
        let kept = kept_description(&world, character).unwrap();
        let data = world.get(kept).unwrap().property("Data").unwrap();
        let description = Description::from_json(data.as_str().unwrap()).unwrap();
        assert_eq!(description.base, DEFAULT_GAME_BASE);
        assert!(
            description.validate(&library).is_empty(),
            "{:?}",
            description.validate(&library)
        );
        for item in DEFAULT_GAME_OUTFIT {
            assert!(description.items.iter().any(|worn| worn.id == item));
        }
        // Still the default character underneath: its controller, Health,
        // Animator and sockets.
        for name in [
            "HumanoidRootPart",
            "Body",
            "Humanoid",
            "ThirdPersonController",
        ] {
            assert!(world.find_first_child(character, name).is_some(), "{name}");
        }
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn without_the_engines_library_the_default_character_is_the_bare_humanoid() {
        let folder = temp("no-shipped");
        add_default_game_character(&folder, None).unwrap();
        let (world, character) = prefab(&folder);
        assert_eq!(base_kept(&world, character), None);
        let body = world.find_first_child(character, "Body").unwrap();
        assert_eq!(
            world.get(body).unwrap().property("MeshId"),
            Some(&Value::String(DEFAULT_HUMANOID_MODEL_PATH.to_owned()))
        );
        let _ = fs::remove_dir_all(&folder);
    }

    /// An older project keeps its players' prefab and its library's own
    /// entries, and gains the human base.
    #[test]
    fn an_older_project_keeps_its_prefab_and_gains_the_human_base() {
        let folder = temp("older");
        add_default_game_character(&folder, None).unwrap();
        let prefab_file = folder.join(DEFAULT_CHARACTER_PREFAB_PATH);
        let old = fs::read_to_string(&prefab_file).unwrap();
        // A package of its own, as an older project has the stylized one.
        let own = folder.join(LIBRARY_FOLDER).join("own/Own.base.json");
        fs::create_dir_all(own.parent().unwrap()).unwrap();
        fs::write(&own, "{}").unwrap();

        add_default_game_character(&folder, Some(&shipped())).unwrap();
        assert_eq!(fs::read_to_string(&prefab_file).unwrap(), old);
        assert_eq!(fs::read_to_string(&own).unwrap(), "{}");
        let (library, _, _) = Library::read(&folder.join(LIBRARY_FOLDER));
        assert!(library.base(DEFAULT_GAME_BASE).is_some());
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_described_character_keeps_its_description_in_its_humanoid() {
        let (library, _, _) = Library::read(&shipped());
        let description = game_character_description(&library).expect("human");
        assert_eq!(description.name, "Character");
        let base = library.base(DEFAULT_GAME_BASE).unwrap();
        for item in &base.defaults.items {
            assert!(description.items.iter().any(|worn| &worn.id == item));
        }
        let mut world = World::new();
        let character = described_character(&mut world, Vector3::default(), &description);
        let kept = kept_description(&world, character).expect("kept");
        assert_eq!(world.get(kept).unwrap().name(), KEPT_DESCRIPTION);
        assert_eq!(
            world.get(kept).unwrap().property("Data"),
            Some(&Value::String(description.to_json()))
        );
        assert_eq!(
            base_kept(&world, character).as_deref(),
            Some(DEFAULT_GAME_BASE)
        );
    }
}
