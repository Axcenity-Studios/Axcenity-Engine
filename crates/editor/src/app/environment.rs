//! Environment presets in the editor (`environment_presets.rs` is the
//! Properties panel's section): applying one to the world being edited and
//! saving the world's look as a new one, each one undo step.

use axcenity_anim::environment::{
    EnvironmentPreset, EnvironmentTarget, apply_preset, capture, find_preset, is_preset_name,
    preset_path,
};

use super::*;
use crate::environment_presets::EnvironmentAction;

impl EditorApp {
    pub(in crate::app) fn environment_action(&mut self, action: EnvironmentAction) {
        match action {
            EnvironmentAction::Apply(name) => self.apply_environment_preset(&name),
            EnvironmentAction::Save(name) => self.save_environment_preset(&name),
        }
    }

    /// Puts the world being edited where the preset `name` has it, as one
    /// undo step. Lighting gets Clouds and a Weather if the preset sets
    /// theirs and it has none; an Atmosphere and a ColorCorrectionEffect
    /// change more than the preset says, so they aren't added.
    pub(in crate::app) fn apply_environment_preset(&mut self, name: &str) {
        if self.playing() {
            return;
        }
        let preset = match find_preset(&self.assets, name) {
            Ok(preset) => preset,
            Err(why) => {
                self.output
                    .error(format!("Could not apply the preset: {why}."));
                return;
            }
        };
        let before = self.world.clone();
        let lighting = self.world.lighting();
        for target in [EnvironmentTarget::Clouds, EnvironmentTarget::Weather] {
            if !preset.values(target).is_empty()
                && target.find(&self.world).is_none()
                && let Ok(id) = self.world.create(target.class())
            {
                let _ = self.world.set_parent(id, Some(lighting));
            }
        }
        let missing = apply_preset(&mut self.world, &preset, |_, _| false);
        self.history
            .record_world(&mut self.world, format!("Apply {name}"), before);
        if missing.contains(&EnvironmentTarget::Atmosphere) {
            self.output.info(format!(
                "Applied the preset `{name}`. Its haze needs an Atmosphere in Lighting \
                 (Lights & Sky > Sky > Atmosphere) to show."
            ));
        }
    }

    /// Saves the world's look as the project's preset `name`
    /// (`assets/environments/<name>.environment.json`), as one undo step.
    /// A preset of that name is replaced.
    pub(in crate::app) fn save_environment_preset(&mut self, name: &str) {
        let Some(folder) = self.project.as_ref().map(|project| project.folder.clone()) else {
            return;
        };
        // Letters, digits, spaces, `-` and `_`, as other asset names.
        if !name.chars().any(char::is_alphanumeric) {
            self.output
                .error("A preset needs a name, such as Dusk.".to_owned());
            return;
        }
        let name = axcenity_anim::files::file_safe(name);
        if !is_preset_name(&name) {
            return;
        }
        let preset: EnvironmentPreset = capture(&self.world);
        let path = preset_path(&name);
        let before = std::fs::read_to_string(folder.join(&path)).ok();
        let text = preset.to_json();
        if let Err(err) = axcenity_anim::files::write_file(&folder.join(&path), &text) {
            self.output.error(format!("Could not save `{path}`: {err}"));
            return;
        }
        let replaced = before.is_some();
        self.record_asset_files(
            format!("Save preset {name}"),
            vec![crate::animation::store::FileDiff {
                path: path.clone(),
                before,
                after: Some(text),
            }],
            None,
        );
        self.environment_presets_changed();
        self.output.info(if replaced {
            format!("Saved the environment preset `{name}` again ({path}).")
        } else {
            format!(
                "Saved the environment preset `{name}` ({path}). Key it on the Sequencer's \
                 Environment track, or call Lighting:TransitionToPreset(\"{name}\", seconds)."
            )
        });
    }

    /// A preset's file was written, removed or put back: the list is read
    /// again and the Sequencer's preview made again with it.
    pub(in crate::app) fn environment_presets_changed(&mut self) {
        self.environment_presets.refresh();
        self.sequencer.presets_changed();
    }
}
