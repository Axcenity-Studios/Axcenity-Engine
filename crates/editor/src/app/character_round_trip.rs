//! The Game Editor's side of the round trip to Character Studio
//! (refinement R10, `character_workspace/game.rs`): **Edit in Character
//! Studio** (the Explorer's menu on a character) opens a character of the
//! world in the studio, drawn in place of the Game Editor's panels; **Done**
//! applies the edited description to it as one undo step, **Cancel**
//! leaves it as it was. Play then shows it, since the description is kept
//! in the character. Also the edit-mode step that shows described
//! characters (`edit_characters.rs`, OPEN-21).

use axcenity_gameplay::description as applier;

use super::*;
use crate::character_workspace::CharacterWorkspace;
use crate::character_workspace::game::{self, GameLink};

impl EditorApp {
    /// Opens the character `id` is (or is part of) in Character Studio.
    /// What went wrong is also said in the Output.
    pub(super) fn edit_in_character_studio(&mut self, id: InstanceId) -> Result<(), String> {
        let opened = self.open_round_trip(id);
        if let Err(why) = &opened {
            self.output
                .error(format!("Could not open it in Character Studio: {why}"));
        }
        opened
    }

    fn open_round_trip(&mut self, id: InstanceId) -> Result<(), String> {
        if self.playing() {
            return Err("stop the game first".to_owned());
        }
        let project = self
            .project
            .as_ref()
            .map(|project| project.name.clone())
            .ok_or("no project is open")?;
        let folder = self
            .characters_folder
            .clone()
            .ok_or("Character Studio has nowhere to keep its files on this computer")?;
        let character = game::editable_character(&self.world, id).ok_or(
            "it isn't a character Character Studio can edit (a Model with a Humanoid and the \
             engine's body, or a description of its own)",
        )?;
        let library = applier::library(&self.assets);
        let description = game::game_description(&self.world, character, &library)?;
        let name = self
            .world
            .get(character)
            .map_or_else(String::new, |instance| instance.name().to_owned());
        let workspace = CharacterWorkspace::open_for_game(
            &folder,
            description,
            GameLink {
                character,
                name,
                project,
            },
        )?;
        // A gesture still open in the world ends here.
        self.history.seal();
        self.character_workspace = Some(Box::new(workspace));
        Ok(())
    }

    /// Leaves the studio opened from the Game Editor: with `done`, the
    /// edited description is applied to the game's character (one undo
    /// step); otherwise nothing changes. When applying fails the studio
    /// stays open and says why.
    pub(super) fn finish_character_round_trip(&mut self, done: bool) {
        let Some(mut workspace) = self.character_workspace.take() else {
            return;
        };
        let Some(link) = workspace.game().cloned() else {
            self.character_workspace = Some(workspace);
            return;
        };
        if !done {
            return;
        }
        let description = workspace.description().clone();
        let studio = workspace.folder().join(axcenity_avatar::LIBRARY_FOLDER);
        match self.apply_studio_character(&link, &description, &studio) {
            Ok(()) => self.select_from_viewport(Some(link.character)),
            Err(why) => {
                workspace.notice = Some(format!("Could not apply it to {}: {why}", link.name));
                self.character_workspace = Some(workspace);
            }
        }
    }

    /// Applies `description` to the game character `link` names, as one
    /// undo step, after giving the project's library what it wears (from
    /// the studio's library, `studio`: both families and a person's own
    /// items).
    fn apply_studio_character(
        &mut self,
        link: &GameLink,
        description: &axcenity_avatar::Description,
        studio: &Path,
    ) -> Result<(), String> {
        let project = self
            .project
            .as_ref()
            .map(|project| project.folder.clone())
            .ok_or("no project is open")?;
        if self
            .world
            .get(link.character)
            .and_then(|c| c.parent())
            .is_none()
        {
            return Err("it isn't in the world any more".to_owned());
        }
        axcenity_project::add_character_library(&project)
            .and_then(|()| axcenity_project::add_character_packages(&project, studio))
            .map_err(|err| format!("could not write the project's character library: {err}"))?;
        let before = self.world.clone();
        match applier::apply_description(&mut self.world, &self.assets, link.character, description)
        {
            Ok(applied) => {
                for problem in applied.problems {
                    self.output
                        .warning(format!("{}: {}", link.name, problem.message));
                }
                self.history.record_world(
                    &mut self.world,
                    format!("Edit {} in Character Studio", link.name),
                    before,
                );
                Ok(())
            }
            Err(why) => {
                self.world = before;
                Err(why)
            }
        }
    }

    /// Edit mode's step for described characters (OPEN-21): applies what
    /// isn't applied in this run and poses them at rest. Nothing during
    /// Play, which applies and poses its own.
    pub(super) fn edit_characters_frame(&mut self) {
        if self.playing() || self.project.is_none() {
            return;
        }
        for error in self.edit_characters.step(&mut self.world, &self.assets) {
            self.output.warning(error);
        }
    }

    /// The default game character at `position` for Insert > Character,
    /// named `name`: the realistic one, applied now so it shows at once
    /// (OPEN-21, OPEN-22), when the project's library has it; else the
    /// stylized default character.
    pub(super) fn default_game_character(
        &mut self,
        position: axcenity_core::Vector3,
        name: &str,
    ) -> InstanceId {
        let library = applier::library(&self.assets);
        let Some(mut description) = axcenity_project::game_character_description(&library) else {
            return axcenity_project::default_character(&mut self.world, position);
        };
        description.name = name.to_owned();
        let character =
            axcenity_project::described_character(&mut self.world, position, &description);
        if let Err(why) =
            applier::apply_description(&mut self.world, &self.assets, character, &description)
        {
            self.output.warning(format!(
                "The character shows its description once it can be applied: {why}."
            ));
        }
        character
    }
}
