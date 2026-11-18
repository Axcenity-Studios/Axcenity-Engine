//! The Game Editor round trip (refinement R10, MAP-S10-08, brief §18
//! Stage 10 "Complete Game Editor integration"): a character in a game's
//! world opens in the studio with **Edit in Character Studio**, and
//! **Done** takes the edited description back to that character (one undo
//! step in the Game Editor); **Cancel** goes back with nothing changed.
//!
//! The workspace only edits the description: the editor's
//! `app/character_round_trip.rs` reads it from the character and applies
//! it back. While it edits a game's character the studio's own autosave
//! and recovery file are left alone (the work goes back to the game, not
//! into `characters/`), and Save still saves a copy into the studio's
//! characters for reuse.

use std::path::Path;

use axcenity_avatar::Description;
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_gameplay::description as applier;

use super::CharacterWorkspace;

/// Which game character the workspace edits, for Done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameLink {
    /// The character's Model in the Game Editor's world.
    pub character: InstanceId,
    /// Its name there, for the toolbar.
    pub name: String,
    /// The project's name, for the toolbar.
    pub project: String,
}

impl CharacterWorkspace {
    /// Opens the workspace on a game character's `description`, linked to
    /// it by `link`: not dirty until edited, and without the studio's
    /// autosave or its recovery offer (a crash's file stays for the studio
    /// opened on its own).
    pub fn open_for_game(
        folder: &Path,
        description: Description,
        link: GameLink,
    ) -> Result<Self, String> {
        let mut workspace = Self::open(folder, description, true)?;
        workspace.game = Some(link);
        workspace.autosave.on = false;
        workspace.recovery = None;
        Ok(workspace)
    }

    /// The game character this workspace edits, if it was opened from the
    /// Game Editor.
    pub fn game(&self) -> Option<&GameLink> {
        self.game.as_ref()
    }
}

/// The character `id` is or is part of that Character Studio can edit:
/// a Model with a skinned body and a Humanoid, which either keeps a
/// description or has the default humanoid's body (a character made
/// before descriptions). A character of other models (an imported one)
/// isn't offered: applying a description would put the engine's body in
/// place of its own.
pub fn editable_character(world: &World, id: InstanceId) -> Option<InstanceId> {
    let mut at = Some(id);
    while let Some(candidate) = at {
        let instance = world.get(candidate)?;
        if instance.class() == ClassName::Model
            && has_humanoid(world, candidate)
            && let Some(body) = applier::body_of(world, candidate)
        {
            let described = applier::applied_instance(world, candidate).is_some();
            return (described || engine_body(world, body)).then_some(candidate);
        }
        at = instance.parent();
    }
    None
}

fn has_humanoid(world: &World, character: InstanceId) -> bool {
    world.get(character).is_some_and(|instance| {
        instance.children().iter().any(|child| {
            world
                .get(*child)
                .is_some_and(|child| child.class() == ClassName::Humanoid)
        })
    })
}

/// Whether `body` draws one of the engine's character models: the default
/// humanoid (whole or its parts file) or a model of the character library.
fn engine_body(world: &World, body: InstanceId) -> bool {
    let mesh = world
        .get(body)
        .and_then(|part| part.property("MeshId"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    mesh == axcenity_project::DEFAULT_HUMANOID_MODEL_PATH
        || mesh.starts_with(axcenity_project::DEFAULT_HUMANOID_PARTS_PATH)
        || mesh.starts_with(&format!("{}/", axcenity_avatar::LIBRARY_FOLDER))
}

/// The description `character` (see [`editable_character`]) has, read
/// against `library` (the project's): the kept one, or the stylized
/// default character for one made before descriptions, named after the
/// character.
pub fn game_description(
    world: &World,
    character: InstanceId,
    library: &axcenity_avatar::Library,
) -> Result<Description, String> {
    match applier::applied_instance(world, character) {
        Some(kept) => applier::description_of(world, kept, library),
        None => {
            let mut description = applier::default_description(library, axcenity_avatar::V1_BASE);
            description.name = world
                .get(character)
                .map_or("Character", |instance| instance.name())
                .to_owned();
            Ok(description)
        }
    }
}
