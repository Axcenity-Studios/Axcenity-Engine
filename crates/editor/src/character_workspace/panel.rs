//! The contextual panel on the right (brief §3.2): only what the selected
//! category, or the feature picked in it, can change. Its header holds the
//! detail level (Quick, Detailed, Advanced: brief §3.3) and Symmetry. Every
//! page returns [`Intent`]s; nothing here changes the workspace.

mod materials;
mod parameters;
mod row;
#[cfg(test)]
mod tests;

use axcenity_avatar::{BaseManifest, Library};
use eframe::egui::{self, Sense};

use super::intent::Intent;
use super::selection::Level;
use super::{Category, CharacterWorkspace, chosen_slots, items_for, presets, worn_in};
use crate::theme::{self, icons};
use crate::widgets;

/// What a category is for, under its title and on its tooltip.
pub(super) fn about(category: Category) -> &'static str {
    match category {
        Category::Body => "Pick the body type, then shape the body. Changes show as you drag.",
        Category::Face => "Shape the face, one feature at a time.",
        Category::Hair => "Pick a hairstyle.",
        Category::Clothing => "Dress the character one piece at a time.",
        Category::SkinAndMakeup => "The skin's colour and surface.",
        Category::Accessories => "Glasses, jewellery, hats and bags.",
        Category::Expressions => {
            "See the face move. A preview only: never saved with the character."
        }
        Category::Presets => "Start again from a ready-made character, then change anything.",
    }
}

/// Whether `category`'s page has sliders (and so a level and Symmetry).
fn has_sliders(category: Category) -> bool {
    matches!(
        category,
        Category::Body | Category::Face | Category::SkinAndMakeup
    )
}

/// Draws the panel and returns what it asks for.
pub(super) fn show(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let selection = &workspace.selection;
    let category = selection.category;
    let base = library.base(&workspace.description().base);
    let feature = selection.feature.as_deref().and_then(|feature| {
        base?
            .groups
            .iter()
            .find(|group| group.id == feature)
            .map(|group| group.label.clone())
    });
    let title = match &feature {
        Some(feature) => format!("{} › {feature}", super::nav::label(category)),
        None => super::nav::label(category).to_owned(),
    };
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .family(theme::display_family())
                .size(20.0)
                .color(palette.text_strong),
        );
        if feature.is_some() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::icon_button(ui, icons::X, "Show the whole category", false).clicked() {
                    intents.push(Intent::PickFeature(None));
                }
            });
        }
    });
    ui.label(egui::RichText::new(about(category)).color(palette.text_weak));
    ui.add_space(6.0);
    if has_sliders(category) {
        ui.horizontal_wrapped(|ui| {
            let mut level = selection.level;
            let options: Vec<(Level, Option<&str>, &str)> = Level::ALL
                .iter()
                .map(|level| (*level, None, level.label()))
                .collect();
            if widgets::segmented(ui, &mut level, &options) {
                intents.push(Intent::PickLevel(level));
            }
            // Quick keeps Symmetry on and doesn't show it (ux patterns,
            // section 2); a page without left and right pairs hasn't it.
            let pairs = base.is_some_and(|base| {
                let data = category.data();
                base.parameters.iter().any(|p| {
                    p.mirror.is_some()
                        && base
                            .groups
                            .iter()
                            .any(|g| g.id == p.group && Some(g.category) == data)
                })
            });
            if selection.level != Level::Quick && pairs {
                let mut symmetry = workspace.symmetry;
                if ui
                    .checkbox(&mut symmetry, "Symmetry")
                    .on_hover_text("Move a left and right pair together")
                    .changed()
                {
                    intents.push(Intent::SetSymmetry(symmetry));
                }
            }
        });
        ui.label(
            egui::RichText::new(selection.level.about())
                .small()
                .color(palette.text_muted),
        );
        ui.add_space(4.0);
    }
    let Some(base) = base else {
        ui.label("This body isn't in the library.");
        return intents;
    };
    // Quick always moves both sides together.
    let symmetry = workspace.symmetry || selection.level == Level::Quick;
    egui::ScrollArea::vertical()
        .id_salt("character workspace panel")
        .auto_shrink(false)
        .show(ui, |ui| {
            let description = workspace.description();
            intents.extend(match category {
                Category::Body => {
                    let mut asked = Vec::new();
                    if selection.feature.is_none() {
                        asked.extend(preset_chips(ui, workspace, library));
                        asked.extend(base_picker(ui, workspace, library));
                    }
                    asked.extend(parameters::show(
                        ui,
                        base,
                        description,
                        selection,
                        category,
                        symmetry,
                    ));
                    asked
                }
                Category::Face => {
                    parameters::show(ui, base, description, selection, category, symmetry)
                }
                Category::SkinAndMakeup => {
                    let mut asked = materials::skin(ui, workspace, base, selection.level);
                    if base
                        .groups
                        .iter()
                        .any(|group| group.category == axcenity_avatar::Category::Skin)
                    {
                        asked.extend(parameters::show(
                            ui,
                            base,
                            description,
                            selection,
                            category,
                            symmetry,
                        ));
                    }
                    asked
                }
                Category::Hair | Category::Clothing | Category::Accessories => {
                    slots_page(ui, workspace, library, base, category)
                }
                Category::Expressions => expressions_page(ui, workspace),
                Category::Presets => presets_page(ui, workspace, library),
            });
        });
    intents
}

/// The presets as chips at the top of the Body page: a good start first
/// (ux patterns P1).
fn preset_chips(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    widgets::section_label(ui, "Start from");
    ui.horizontal_wrapped(|ui| {
        for (index, preset) in presets().iter().enumerate() {
            if library.base(preset.base).is_none() {
                continue;
            }
            if widgets::chip(
                ui,
                preset.name,
                preset.matches(workspace.description(), library),
            )
            .on_hover_text(format!("{} Keeps the name; you can undo it.", preset.about))
            .clicked()
            {
                intents.push(Intent::ApplyPreset(index));
            }
        }
    });
    ui.add_space(8.0);
    intents
}

/// The body families Character Studio offers: one chip each, shown only
/// when there's a choice (or the character is on a body no longer offered,
/// such as a saved stylized one, so it can move).
fn base_picker(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let current = &workspace.description().base;
    let offered: Vec<(&String, &BaseManifest)> = library
        .bases
        .iter()
        .filter(|(id, _)| super::offers_base(library, id))
        .collect();
    if offered.len() < 2 && offered.iter().any(|(id, _)| *id == current) {
        return intents;
    }
    widgets::section_label(ui, "Body family");
    ui.horizontal_wrapped(|ui| {
        for (id, base) in offered {
            if widgets::chip(ui, &base.label, id == current)
                .on_hover_text(format!(
                    "Start again on the {} body",
                    base.label.to_lowercase()
                ))
                .clicked()
            {
                intents.push(Intent::SetBase(id.clone()));
            }
        }
    });
    ui.add_space(8.0);
    intents
}

/// The manifest's slots of `category`: a chip per item that fits this
/// body, and one for wearing nothing.
fn slots_page(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
    base: &BaseManifest,
    category: Category,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let data = category.data();
    let description = workspace.description();
    let chosen = chosen_slots(library, &description.base);
    let palette = theme::palette(ui.ctx());
    if base
        .slots
        .iter()
        .any(|slot| Some(slot.category) == data && chosen.contains(&slot.id))
    {
        ui.label(
            egui::RichText::new("Underwear comes with the body type (Male or Female).")
                .color(palette.text_weak),
        );
        ui.add_space(8.0);
    }
    let mut any = false;
    for slot in base
        .slots
        .iter()
        .filter(|slot| Some(slot.category) == data && !chosen.contains(&slot.id))
    {
        let items = items_for(library, &description.base, &slot.id);
        let worn = worn_in(library, description, &slot.id);
        if items.is_empty() && worn.is_none() {
            continue;
        }
        any = true;
        widgets::section_label(ui, &slot.label);
        ui.horizontal_wrapped(|ui| {
            let none = slot.empty_label.as_deref().unwrap_or("None");
            if widgets::chip(ui, none, worn.is_none()).clicked() {
                intents.push(Intent::Wear {
                    slot: slot.id.clone(),
                    item: None,
                });
            }
            for (id, label) in &items {
                if widgets::chip(ui, label, worn.as_deref() == Some(id.as_str())).clicked() {
                    intents.push(Intent::Wear {
                        slot: slot.id.clone(),
                        item: Some(id.clone()),
                    });
                }
            }
        });
        ui.add_space(10.0);
    }
    if !any {
        let (title, sentence) = match category {
            Category::Hair => (
                "No hairstyles yet",
                "Nothing in the library fits this body yet. Hairstyles for it are being made.",
            ),
            Category::Accessories => (
                "No accessories yet",
                "Nothing in the library fits this body yet. Glasses, jewellery, hats and bags \
                 are being made.",
            ),
            _ => (
                "No clothes yet",
                "Nothing in the library fits this body yet beyond what the body type wears.",
            ),
        };
        widgets::empty_state(ui, super::nav::icon(category), title, sentence, None);
    }
    intents
}

/// The built-in expressions as chips: a click shows one on the preview's
/// face; the description never changes (brief §10).
fn expressions_page(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    widgets::section_label(ui, "Built-in expressions");
    ui.horizontal_wrapped(|ui| {
        for name in axcenity_anim::BUILT_IN_EXPRESSIONS {
            let shown = match &workspace.expression {
                Some(shown) => shown.eq_ignore_ascii_case(name),
                None => name.eq_ignore_ascii_case("Neutral"),
            };
            if widgets::chip(ui, name, shown).clicked() {
                intents.push(Intent::PreviewExpression(
                    (!shown).then(|| (*name).to_owned()),
                ));
            }
        }
    });
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Shown on the preview only. Expressions are made and edited in the Game \
             Editor's face tab.",
        )
        .small()
        .color(palette.text_weak),
    );
    intents
}

/// Ready-made characters as cards; the one the character is, highlighted.
fn presets_page(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    for (index, preset) in presets().iter().enumerate() {
        if library.base(preset.base).is_none() {
            continue;
        }
        let selected = preset.matches(workspace.description(), library);
        let response = egui::Frame::new()
            .fill(if selected {
                palette.accent_soft
            } else {
                palette.surface
            })
            .corner_radius(theme::radius::CARD)
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(preset.name)
                        .family(theme::medium_family())
                        .color(palette.text_strong),
                );
                ui.label(
                    egui::RichText::new(preset.about)
                        .color(palette.text_weak)
                        .small(),
                );
            })
            .response
            .interact(Sense::click());
        if response.clicked() {
            intents.push(Intent::ApplyPreset(index));
        }
        ui.add_space(6.0);
    }
    intents
}
