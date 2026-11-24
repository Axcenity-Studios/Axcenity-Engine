//! The Properties panel's **Environment presets** section (shown with
//! nothing or Lighting selected): a button for each environment preset
//! (`anim::environment`: the sky and the weather under one name), which
//! puts the world there as one undo step, and **Save current as…**, which
//! writes the world's look as `assets/environments/<name>.environment.json`
//! for the Sequencer's Environment track and `Lighting:TransitionToPreset`.
//! The actions are `app/environment.rs`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use axcenity_anim::environment::{BUILT_IN, preset_names};
use axcenity_assets::Assets;
use eframe::egui;

use crate::theme::{self, icons};

/// How long the list of the project's presets is kept before the folder is
/// read again (a file may come from outside: a sync, a checkout).
const KEPT: Duration = Duration::from_secs(1);

/// The presets' names and the name being typed for a new one.
#[derive(Default)]
pub struct EnvironmentPresets {
    names: Vec<String>,
    read: Option<(Instant, Option<PathBuf>)>,
    /// The name typed beside Save current as.
    pub new_name: String,
}

impl EnvironmentPresets {
    /// Every preset's name: the built-ins, then the project's own.
    pub fn names(&mut self, assets: &Assets) -> &[String] {
        let folder = assets.folder();
        let fresh = self
            .read
            .as_ref()
            .is_some_and(|(at, of)| *of == folder && at.elapsed() < KEPT);
        if !fresh {
            self.names = preset_names(assets);
            self.read = Some((Instant::now(), folder));
        }
        &self.names
    }

    /// Reads the project's presets again the next time they're asked for.
    pub fn refresh(&mut self) {
        self.read = None;
    }
}

/// What the section asks the editor for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentAction {
    /// Put the world where this preset has it.
    Apply(String),
    /// Save the world's look as a preset with this name.
    Save(String),
}

/// Draws the section. `names` is every preset, the built-ins first.
pub fn show(
    ui: &mut egui::Ui,
    state: &mut EnvironmentPresets,
    names: &[String],
    has_project: bool,
) -> Option<EnvironmentAction> {
    let palette = theme::palette(ui.ctx());
    let mut action = None;
    ui.add_space(theme::space::M);
    egui::CollapsingHeader::new(
        egui::RichText::new(theme::with_icon(icons::CLOUD_SUN, "Environment presets"))
            .family(theme::heading_family())
            .color(palette.text_strong),
    )
    .id_salt("environment_presets")
    .default_open(true)
    .show(ui, |ui| {
        ui.label(
            egui::RichText::new(
                "The time of day, the air, the clouds, the weather and the wind, set together.",
            )
            .color(palette.text_weak),
        );
        ui.add_space(theme::space::XS);
        ui.horizontal_wrapped(|ui| {
            for name in names {
                let own = !BUILT_IN.contains(&name.as_str());
                let hint = if own {
                    format!(
                        "Apply this project's preset `{name}` to the world now (one undo step)."
                    )
                } else {
                    format!(
                        "Apply the built-in preset `{name}` to the world now (one undo step). \
                         Clouds and a Weather are added to Lighting if it has none."
                    )
                };
                if ui.button(name).on_hover_text(hint).clicked() {
                    action = Some(EnvironmentAction::Apply(name.clone()));
                }
            }
        });
        ui.add_space(theme::space::XS);
        crate::responsive::toolbar(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.new_name)
                    .hint_text("New preset's name")
                    .desired_width(120.0),
            );
            let name = state.new_name.trim().to_owned();
            if ui
                .add_enabled(
                    has_project && !name.is_empty(),
                    egui::Button::new(theme::with_icon(icons::FLOPPY_DISK, "Save current as")),
                )
                .on_hover_text(
                    "Save how the sky and the weather are now as a preset of this project \
                     (assets/environments), for the Sequencer's Environment track and \
                     Lighting:TransitionToPreset. One undo step.",
                )
                .on_disabled_hover_text(if has_project {
                    "Type a name for the preset first."
                } else {
                    "Open a project to save presets in."
                })
                .clicked()
            {
                action = Some(EnvironmentAction::Save(name));
                state.new_name.clear();
            }
        });
    });
    action
}
