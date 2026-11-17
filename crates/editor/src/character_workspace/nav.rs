//! The navigation down the left (brief §3.1, the lead's DIS-06): the
//! categories in the brief's order and, from the Detailed level, the open
//! category's features (the manifest's groups today: Eyes, Nose, Jaw…).
//! Each shows how many of its values differ from the default.

use axcenity_avatar::{BaseManifest, Description, Library};
use eframe::egui;

use super::intent::Intent;
use super::selection::{Level, parameter_feature, parameter_level};
use super::{Category, CharacterWorkspace, value_of};
use crate::theme::{self, icons};
use crate::widgets;

pub(super) fn label(category: Category) -> &'static str {
    match category {
        Category::Body => "Body",
        Category::Face => "Face",
        Category::Hair => "Hair",
        Category::Clothing => "Clothing",
        Category::SkinAndMakeup => "Skin and Makeup",
        Category::Accessories => "Accessories",
        Category::Expressions => "Expressions",
        Category::Presets => "Presets",
    }
}

pub(super) fn icon(category: Category) -> &'static str {
    match category {
        Category::Body => icons::PERSON,
        Category::Face => icons::SMILEY,
        Category::Hair => icons::SCISSORS,
        Category::Clothing => icons::T_SHIRT,
        Category::SkinAndMakeup => icons::DROP,
        Category::Accessories => icons::EYEGLASSES,
        Category::Expressions => icons::MASK_HAPPY,
        Category::Presets => icons::SPARKLE,
    }
}

/// A feature of a category: a manifest group with something to show at
/// `level`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Feature {
    pub id: String,
    pub label: String,
    /// How many of its values differ from the default.
    pub changed: usize,
}

/// The features of `category` on `base` shown at `level` (none at Quick:
/// the level shows categories only).
pub(super) fn features(
    base: &BaseManifest,
    description: &Description,
    category: Category,
    level: Level,
) -> Vec<Feature> {
    let Some(data) = category.data() else {
        return Vec::new();
    };
    if level == Level::Quick {
        return Vec::new();
    }
    base.groups
        .iter()
        .filter(|group| group.category == data)
        .filter_map(|group| {
            let parameters: Vec<_> = base
                .parameters
                .iter()
                .filter(|p| parameter_feature(p) == group.id)
                .collect();
            parameters
                .iter()
                .any(|p| level.shows(parameter_level(p)))
                .then(|| Feature {
                    id: group.id.clone(),
                    label: group.label.clone(),
                    changed: parameters
                        .iter()
                        .filter(|p| value_of(description, Some(base), &p.id) != p.default)
                        .count(),
                })
        })
        .collect()
}

/// How many of `category`'s values differ from the default (its sliders;
/// for Skin and Makeup also the skin's material values).
pub(super) fn changed(base: &BaseManifest, description: &Description, category: Category) -> usize {
    let Some(data) = category.data() else {
        return 0;
    };
    let sliders = base
        .parameters
        .iter()
        .filter(|p| {
            base.groups
                .iter()
                .any(|group| group.id == p.group && group.category == data)
        })
        .filter(|p| p.choices.is_empty())
        .filter(|p| value_of(description, Some(base), &p.id) != p.default)
        .count();
    let materials = if data == axcenity_avatar::Category::Skin {
        base.materials
            .iter()
            .filter(|slot| slot.shading == axcenity_avatar::ShadingModel::Skin)
            .filter_map(|slot| description.materials.get(&slot.name))
            .map(|values| values.len())
            .sum()
    } else {
        0
    };
    sliders + materials
}

/// Draws the navigation and returns what it asks for.
pub(super) fn show(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let description = workspace.description();
    let base = library.base(&description.base);
    let selection = &workspace.selection;
    widgets::section_label(ui, "Customize");
    for category in Category::ALL {
        let count = base
            .map(|base| changed(base, description, category))
            .filter(|count| *count > 0);
        let open = selection.category == category;
        let response = widgets::nav_item(ui, icon(category), label(category), count, open)
            .on_hover_text(super::panel::about(category));
        if response.clicked() {
            if open {
                // A second click on the open category shows all of it.
                intents.push(Intent::PickFeature(None));
            } else {
                intents.push(Intent::PickCategory(category));
            }
        }
        if !open {
            continue;
        }
        let Some(base) = base else {
            continue;
        };
        for feature in features(base, description, category, selection.level) {
            let picked = selection.feature.as_deref() == Some(feature.id.as_str());
            let text = if feature.changed > 0 {
                format!("{}  ·  {}", feature.label, feature.changed)
            } else {
                feature.label.clone()
            };
            let response = ui
                .horizontal(|ui| {
                    ui.add_space(28.0);
                    ui.add(egui::Button::selectable(
                        picked,
                        egui::RichText::new(text).color(if picked {
                            palette.text_strong
                        } else {
                            palette.text_weak
                        }),
                    ))
                })
                .inner;
            if response.clicked() {
                intents.push(Intent::PickFeature((!picked).then(|| feature.id.clone())));
            }
        }
    }
    intents
}
