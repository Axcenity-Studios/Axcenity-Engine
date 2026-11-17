//! The toolbar (brief §3.1): Home, the character's name and whether it's
//! saved, New, Open, Save, undo and redo, and Reset with a scope.
//! Randomize, comparison and viewport settings come with R1's second part.

use axcenity_avatar::Library;
use eframe::egui;

use super::edit::Scope;
use super::intent::Intent;
use super::selection::Selection;
use super::{CharacterWorkspace, presets, shelf};
use crate::theme::{self, icons};
use crate::widgets::{self, Role};

/// Draws the toolbar and returns what it asks for.
pub(super) fn show(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    ui.horizontal_centered(|ui| {
        if widgets::icon_button(ui, icons::HOUSE, "Back to the start screen", false).clicked() {
            intents.push(Intent::Home);
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(theme::with_icon(icons::USER_FOCUS, "Character Studio"))
                .family(theme::medium_family())
                .color(palette.accent),
        );
        ui.add_space(8.0);
        intents.extend(name_field(ui, workspace));
        if workspace.dirty() {
            ui.label(
                egui::RichText::new("Not saved")
                    .color(palette.text_weak)
                    .small(),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(widgets::button(
                    Role::Primary,
                    Some(icons::FLOPPY_DISK),
                    "Save",
                    &palette,
                ))
                .on_hover_text("Save the character (Ctrl+S)")
                .clicked()
            {
                intents.push(Intent::Save);
            }
            widgets::menu_button(ui, Some(icons::ARROW_COUNTER_CLOCKWISE), "Reset", |ui| {
                for (label, about, scope) in reset_choices(&workspace.selection, workspace, library)
                {
                    if ui.button(label).on_hover_text(about).clicked() {
                        intents.push(Intent::Reset(scope));
                        ui.close();
                    }
                }
            });
            widgets::menu_button(ui, Some(icons::FOLDER_OPEN), "Open", |ui| {
                let saved = shelf::list(workspace.folder());
                if saved.is_empty() {
                    ui.label(
                        egui::RichText::new("No saved characters yet.").color(palette.text_weak),
                    );
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for character in saved {
                            let own = workspace.file() == Some(character.path.as_path());
                            if ui
                                .add_enabled(!own, egui::Button::new(&character.name))
                                .clicked()
                            {
                                intents.push(Intent::Open(character.path.clone()));
                                ui.close();
                            }
                        }
                    });
            });
            widgets::menu_button(ui, Some(icons::FILE_PLUS), "New", |ui| {
                for (index, preset) in presets().iter().enumerate() {
                    if library.base(preset.base).is_none() {
                        continue;
                    }
                    if ui.button(preset.name).on_hover_text(preset.about).clicked() {
                        intents.push(Intent::New(index));
                        ui.close();
                    }
                }
            });
            ui.add_space(8.0);
            if ui
                .add_enabled(
                    workspace.can_redo(),
                    egui::Button::new(icons::ARROW_CLOCKWISE).frame(false),
                )
                .on_hover_text("Redo (Ctrl+Y or Ctrl+Shift+Z)")
                .clicked()
            {
                intents.push(Intent::Redo);
            }
            if ui
                .add_enabled(
                    workspace.can_undo(),
                    egui::Button::new(icons::ARROW_COUNTER_CLOCKWISE).frame(false),
                )
                .on_hover_text("Undo (Ctrl+Z)")
                .clicked()
            {
                intents.push(Intent::Undo);
            }
        });
    });
    intents
}

/// The character's name. What's typed is kept as a draft and becomes one
/// rename when the field is left (Enter, a click elsewhere), so the field
/// can be emptied while typing (audit D-04) and a visit that changes
/// nothing makes no undo step (D-05).
fn name_field(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Option<Intent> {
    let id = egui::Id::new("character studio name");
    let mut text = ui
        .data(|data| data.get_temp::<String>(id))
        .unwrap_or_else(|| workspace.description().name.clone());
    let field = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id.with("field"))
            .desired_width(180.0)
            .hint_text("Character name"),
    );
    if field.has_focus() {
        ui.data_mut(|data| data.insert_temp(id, text.clone()));
    }
    if field.lost_focus() {
        ui.data_mut(|data| data.remove::<String>(id));
        if text != workspace.description().name {
            return Some(Intent::Rename(text));
        }
    }
    None
}

/// What Reset offers for `selection`: everything, the category shown, and
/// the feature picked in it.
pub(super) fn reset_choices(
    selection: &Selection,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<(String, &'static str, Scope)> {
    let mut choices = vec![(
        "Everything".to_owned(),
        "Back to the body's default character, keeping the name (you can undo it)",
        Scope::All,
    )];
    if let Some(data) = selection.category.data() {
        choices.push((
            format!("{} only", super::nav::label(selection.category)),
            "This category back to its defaults; the body type stays (you can undo it)",
            Scope::Category(data),
        ));
    }
    if let Some(feature) = &selection.feature
        && let Some(group) = library
            .base(&workspace.description().base)
            .and_then(|base| base.groups.iter().find(|group| group.id == *feature))
    {
        choices.push((
            format!("{} only", group.label),
            "This feature's sliders back to their defaults (you can undo it)",
            Scope::Group(group.id.clone()),
        ));
    }
    choices
}
