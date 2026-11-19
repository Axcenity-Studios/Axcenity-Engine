//! The default game character (refinement R10, OPEN-22; plan section 0,
//! decision 7: both families are selectable, the realistic one the
//! default).
//!
//! A new project, and Insert > Character in any project, get both of the
//! engine's character families in their library: the stylized package
//! built into the engine ([`add_character_library`]) and the packages the
//! engine ships beside it (`assets/characters/`, found by
//! [`shipped_character_library`]; the realistic body's model is too big
//! to build into the program). The default character is then the
//! realistic one: the default humanoid's Model ([`default_character`])
//! with a description kept in its Humanoid ([`described_character`]),
//! which the Character Runtime applies wherever the world is loaded
//! (`gameplay::description::DescriptionSync`), in the editor's edit mode
//! too. Without the shipped packages (a program run away from its files)
//! the default character stays the stylized one.
//!
//! A project's own files are never written over: an older project keeps
//! its `Character` prefab (its players stay what they were) and only
//! gains the families it lacked.

use std::path::{Path, PathBuf};

use axcenity_avatar::{Description, LIBRARY_FOLDER, Library, WornItem};
use axcenity_core::{ClassName, InstanceId, Value, Vector3};

use super::default_humanoid::write_missing;
use super::*;

/// The base model of the default game character: the realistic family.
pub const DEFAULT_GAME_BASE: &str = "axcenity/realistic";

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

/// Gives `project` both character families: the built-in stylized
/// package, then the packages in `shipped` (see
/// [`add_character_packages`]: a project's own files and IDs are kept).
/// Returns the project's library as it is now.
pub fn add_both_character_libraries(
    project: &Path,
    shipped: Option<&Path>,
) -> Result<Library, SaveError> {
    add_character_library(project)?;
    if let Some(shipped) = shipped {
        add_character_packages(project, shipped)?;
    }
    Ok(Library::read(&project.join(LIBRARY_FOLDER)).0)
}

/// The default game character's description: the realistic family's
/// default character (its default items, every parameter at its
/// default), named `Character`. `None` when `library` hasn't the family.
pub fn game_character_description(library: &Library) -> Option<Description> {
    let base = library.base(DEFAULT_GAME_BASE)?;
    let mut description = Description::new("Character", DEFAULT_GAME_BASE);
    description.items = base.defaults.items.iter().map(WornItem::new).collect();
    Some(description)
}

/// The default character ([`default_character`]) wearing `description`:
/// a HumanoidDescription named [`KEPT_DESCRIPTION`] inside its Humanoid
/// holds it (`Base` and `Data`), which the Character Runtime applies
/// where the world is loaded. Until then its `Body` is the stylized
/// model's.
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
/// [`add_default_humanoid`]) with both character families
/// ([`add_both_character_libraries`]), and a `Character` prefab of the
/// default game character: the realistic one when the library has it,
/// else the stylized one. Files already there are kept, so an older
/// project's prefab stays as it is. Returns the prefab's path.
pub fn add_default_game_character(
    project: &Path,
    shipped: Option<&Path>,
) -> Result<&'static str, SaveError> {
    let library = add_both_character_libraries(project, shipped)?;
    // The prefab first: `add_default_humanoid` keeps a file already there,
    // so it writes the stylized one only when the realistic one isn't.
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

    fn temp(name: &str) -> axcenity_test_dirs::TestDir {
        let folder = axcenity_test_dirs::TestDir::adopt(std::env::temp_dir().join(format!(
            "axcenity-game-character-{name}-{}",
            std::process::id()
        )));
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
        assert!(found.join("realistic").is_dir(), "{}", found.display());
    }

    /// OPEN-22: a new project has both families, and its players are the
    /// realistic character (decision 7).
    #[test]
    fn a_new_project_gets_both_families_and_the_realistic_default_character() {
        let parent = temp("new-project");
        let folder = create_project_from(&parent, "Realistic Game", Template::Baseplate).unwrap();
        let (library, _, problems) = Library::read(&folder.join(LIBRARY_FOLDER));
        assert!(problems.is_empty(), "{problems:?}");
        assert!(library.base(axcenity_avatar::V1_BASE).is_some(), "stylized");
        assert!(library.base(DEFAULT_GAME_BASE).is_some(), "realistic");
        assert!(library.validate().is_empty(), "{:?}", library.validate());

        let (world, character) = prefab(&folder);
        assert_eq!(
            base_kept(&world, character).as_deref(),
            Some(DEFAULT_GAME_BASE)
        );
        let kept = kept_description(&world, character).unwrap();
        let data = world.get(kept).unwrap().property("Data").unwrap();
        let description = Description::from_json(data.as_str().unwrap()).unwrap();
        assert_eq!(description.base, DEFAULT_GAME_BASE);
        assert!(description.validate(&library).is_empty());
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
    fn without_the_engines_library_the_default_character_stays_stylized() {
        let folder = temp("no-shipped");
        add_default_game_character(&folder, None).unwrap();
        let (library, _, _) = Library::read(&folder.join(LIBRARY_FOLDER));
        assert!(library.base(DEFAULT_GAME_BASE).is_none());
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
    /// entries, and gains the realistic family.
    #[test]
    fn an_older_project_keeps_its_prefab_and_gains_the_realistic_family() {
        let folder = temp("older");
        add_default_humanoid(&folder).unwrap();
        let prefab_file = folder.join(DEFAULT_CHARACTER_PREFAB_PATH);
        let old = fs::read_to_string(&prefab_file).unwrap();
        add_default_game_character(&folder, Some(&shipped())).unwrap();
        assert_eq!(fs::read_to_string(&prefab_file).unwrap(), old);
        let (library, _, problems) = Library::read(&folder.join(LIBRARY_FOLDER));
        assert!(problems.is_empty(), "{problems:?}");
        assert!(library.base(axcenity_avatar::V1_BASE).is_some());
        assert!(library.base(DEFAULT_GAME_BASE).is_some());
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_described_character_keeps_its_description_in_its_humanoid() {
        let (library, _, _) = Library::read(&shipped());
        let description = game_character_description(&library).expect("realistic");
        assert_eq!(description.name, "Character");
        let base = library.base(DEFAULT_GAME_BASE).unwrap();
        let defaults: Vec<WornItem> = base.defaults.items.iter().map(WornItem::new).collect();
        assert_eq!(description.items, defaults);
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
