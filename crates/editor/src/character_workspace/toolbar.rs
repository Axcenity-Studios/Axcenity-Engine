//! The toolbar (brief §3.1): Home (Cancel and Done for a game's
//! character, the round trip), the character's name and whether it's
//! saved, New (a family or a preset), Open, Save, undo and redo, Reset
//! with a scope and Randomize with a scope (brief §11), View (content
//! review: drafts shown, ADR-0043), and the switch for
//! editing on the character (R2: hover, click to select, handles;
//! `direct.rs`). Comparison is the strip over the viewport (`compare.rs`).

use axcenity_avatar::{Library, PresetKind};
use eframe::egui;

use super::edit::Scope;
use super::intent::Intent;
use super::random::{RandomScope, scope_for};
use super::selection::Selection;
use super::{CharacterWorkspace, Start, presets, shelf};
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
        // Opened from the Game Editor (the round trip, `game.rs`): Cancel
        // in Home's place, and Done beside Save.
        let game = workspace.game();
        match game {
            Some(game) => {
                if ui
                    .button(theme::with_icon(icons::X, "Cancel"))
                    .on_hover_text(format!(
                        "Back to {} without changing {}",
                        game.project, game.name
                    ))
                    .clicked()
                {
                    intents.push(Intent::Cancel);
                }
            }
            None => {
                if widgets::icon_button(ui, icons::HOUSE, "Back to the start screen", false)
                    .clicked()
                {
                    intents.push(Intent::Home);
                }
            }
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
            if let Some(game) = game
                && ui
                    .add(widgets::button(
                        Role::Primary,
                        Some(icons::CHECK),
                        "Done",
                        &palette,
                    ))
                    .on_hover_text(format!(
                        "Back to {} with {} looking like this (one undo step there)",
                        game.project, game.name
                    ))
                    .clicked()
            {
                intents.push(Intent::Done);
            }
            let save = if game.is_some() {
                Role::Secondary
            } else {
                Role::Primary
            };
            let about = if game.is_some() {
                "Save a copy in Character Studio's characters (Ctrl+S)"
            } else {
                "Save the character (Ctrl+S)"
            };
            if ui
                .add(widgets::button(
                    save,
                    Some(icons::FLOPPY_DISK),
                    "Save",
                    &palette,
                ))
                .on_hover_text(about)
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
                for (label, about, start) in new_choices(library, workspace) {
                    match start {
                        None => {
                            widgets::section_label(ui, &label);
                        }
                        Some(start) => {
                            if ui.button(label).on_hover_text(about).clicked() {
                                intents.push(Intent::New(start));
                                ui.close();
                            }
                        }
                    }
                }
            });
            widgets::menu_button(ui, Some(icons::DICE_FIVE), "Randomize", |ui| {
                for (label, about, scope) in
                    random_choices(&workspace.selection, workspace, library)
                {
                    if ui.button(label).on_hover_text(about).clicked() {
                        intents.push(Intent::Randomize(scope));
                        ui.close();
                    }
                }
            });
            widgets::menu_button(ui, Some(icons::EYE), "View", |ui| {
                for option in view_options(workspace) {
                    let mut on = option.on;
                    if ui
                        .checkbox(&mut on, option.label)
                        .on_hover_text(option.about)
                        .changed()
                    {
                        intents.push(option.toggle);
                        ui.close();
                    }
                }
            });
            ui.add_space(8.0);
            let on = workspace.direct.on;
            let tip = if on {
                "Edit on character is on: hover, click a feature and drag its handles. Click to \
                 turn it off and only move the camera"
            } else {
                "Edit on character is off: the viewport only moves the camera. Click to turn it on"
            };
            if widgets::icon_button(ui, icons::CURSOR_CLICK, tip, on).clicked() {
                intents.push(Intent::SetHandles(!on));
            }
            ui.add_space(4.0);
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

/// An option of the View menu: shown checked when `on`; picking it
/// asks for `toggle`.
#[derive(Clone, Debug)]
pub(super) struct ViewOption {
    pub label: &'static str,
    pub about: &'static str,
    pub on: bool,
    pub toggle: Intent,
}

/// What the View menu offers: drafts for content review (ADR-0043), off
/// by default, editor state.
pub(super) fn view_options(workspace: &CharacterWorkspace) -> Vec<ViewOption> {
    let on = workspace.browser.show_drafts;
    vec![ViewOption {
        label: "Show drafts for content review",
        about: "Lists items that haven't passed the asset quality review on the Hair and \
                Clothing pages, marked \"Draft\". Drafts are never in presets or Randomize. \
                For reviewing content; not saved in a character.",
        on,
        toggle: Intent::Browse(super::browser::Browse::ShowDrafts(!on)),
    }]
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

/// What New offers: for each family (realistic first), a heading
/// (`None`), then its whole-character presets, or its default character
/// when it has none.
pub(super) fn new_choices(
    library: &Library,
    workspace: &CharacterWorkspace,
) -> Vec<(String, String, Option<Start>)> {
    let mut choices = Vec::new();
    for base in presets::families(library) {
        choices.push((base.label.clone(), String::new(), None));
        let made = presets::of_kind(
            library,
            workspace.own_presets(),
            &base.id,
            PresetKind::Character,
        );
        if made.is_empty() {
            choices.push((
                format!("{} character", base.label),
                format!("The {} body as it's built", base.label.to_lowercase()),
                Some(Start::Family(base.id.clone())),
            ));
        }
        for listed in made {
            choices.push((
                listed.label().to_owned(),
                listed.info.about.clone(),
                Some(Start::Preset(listed.id.to_owned())),
            ));
        }
    }
    choices
}

/// What Randomize offers for `selection`: everything, the category shown
/// and the feature picked in it, each saying how many locks it keeps.
pub(super) fn random_choices(
    selection: &Selection,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<(String, String, RandomScope)> {
    let Some(base) = library.base(&workspace.description().base) else {
        return Vec::new();
    };
    let locked = |scope: &axcenity_avatar::Scope| match workspace.locked_in(base, scope) {
        0 => String::new(),
        1 => " 1 locked value is kept.".to_owned(),
        n => format!(" {n} locked values are kept."),
    };
    let feature = selection.feature.as_deref();
    let mut choices = Vec::new();
    for (scope, label) in [
        (RandomScope::Everything, "Everything".to_owned()),
        (
            RandomScope::Category,
            super::nav::label(selection.category).to_owned(),
        ),
        (
            RandomScope::Feature,
            super::nav::features(
                base,
                workspace.description(),
                selection.category,
                selection.level,
            )
            .into_iter()
            .find(|found| Some(found.id.as_str()) == feature)
            .map(|found| found.label)
            .unwrap_or_default(),
        ),
    ] {
        let Some(reach) = scope_for(scope, selection.category, feature, base) else {
            continue;
        };
        let about = match scope {
            RandomScope::Everything => "A new character from this one's body",
            RandomScope::Category => "This category drawn again",
            RandomScope::Feature => "This feature drawn again",
        };
        // Randomize draws Production items only (ADR-0043): say when the
        // scope's slots have none, so what's worn stays.
        let items = if super::random::no_items_to_draw(library, base, &reach) {
            super::random::NO_ITEMS
        } else {
            ""
        };
        choices.push((
            label,
            format!("{about}, as one undo step.{}{items}", locked(&reach)),
            scope,
        ));
    }
    choices
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
