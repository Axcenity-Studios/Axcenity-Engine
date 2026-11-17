//! The Skin and Makeup page's material values: every value the base's skin
//! materials take (`avatar::ShadingModel::Skin`; the realistic body's slot
//! is "Skin"), a slider for a number and a colour button for a colour. A
//! value left at its neutral setting is not written, so the material looks
//! as made.
//!
//! The renderer draws only some of them until the skin shading model lands
//! (CO7): the page says which.

use axcenity_avatar::{BaseManifest, Colour, MaterialValue, ShadingModel, ValueKind};
use eframe::egui;

use super::super::CharacterWorkspace;
use super::super::intent::{Gesture, Intent};
use super::super::selection::Level;
use super::row::{self, Row};
use crate::color_palette;
use crate::theme;
use crate::widgets;

/// The values `scene::material::modulate` draws today for every shading
/// model (until CO7): the others are kept in the description and drawn
/// once the skin shading model reads them.
pub(in super::super) const DRAWN_TODAY: &[&str] = &["tint", "iris_tint", "roughness_scale"];

/// A value's name for people: `roughness_scale` is "Roughness scale".
pub(in super::super) fn value_label(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Draws the skin materials' values and returns what they ask for.
pub(super) fn skin(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    base: &BaseManifest,
    level: Level,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let slots: Vec<_> = base
        .materials
        .iter()
        .filter(|slot| slot.shading == ShadingModel::Skin)
        .collect();
    if slots.is_empty() {
        widgets::empty_state(
            ui,
            crate::theme::icons::DROP,
            "No skin material",
            "This body has no material with skin shading to adjust.",
            None,
        );
        return intents;
    }
    for slot in slots {
        widgets::section(
            crate::theme::icons::DROP,
            &format!("{} material", slot.name),
        )
        .id_salt(&slot.name)
        .show(ui, |ui| {
            for spec in slot.shading.values() {
                let drawn = DRAWN_TODAY.contains(&spec.name);
                let about = if drawn {
                    spec.meaning.to_owned()
                } else {
                    format!(
                        "{} Saved with the character; the renderer draws it once skin \
                             shading lands (CO7).",
                        spec.meaning
                    )
                };
                let label = value_label(spec.name);
                let note = (!drawn).then_some("not drawn yet");
                let set = workspace.material_value(&slot.name, spec.name);
                let change = |to: Option<MaterialValue>, gesture| Intent::SetMaterial {
                    slot: slot.name.clone(),
                    value: spec.name.to_owned(),
                    to,
                    gesture,
                };
                match spec.kind {
                    ValueKind::Number { min, max, neutral } => {
                        let value = match set {
                            Some(MaterialValue::Number(value)) => value,
                            _ => neutral,
                        };
                        let id = format!("{}.{}", slot.name, spec.name);
                        if let Some(row::Asked::Set(value, gesture)) = row::slider(
                            ui,
                            &Row {
                                id: &id,
                                label: &label,
                                note,
                                about: &about,
                                value,
                                range: [min, max],
                                default: neutral,
                                typed: level == Level::Advanced,
                                // Material values aren't randomized.
                                locked: None,
                            },
                        ) {
                            let to = ((value - neutral).abs() > 1.0e-4)
                                .then_some(MaterialValue::Number(value));
                            intents.push(change(to, gesture));
                        }
                    }
                    ValueKind::Colour { neutral } => {
                        let colour = match set {
                            Some(MaterialValue::Colour(colour)) => colour,
                            _ => neutral,
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&label).color(palette.text_secondary))
                                .on_hover_text(&about);
                            if let Some(note) = note {
                                ui.label(
                                    egui::RichText::new(note).small().color(palette.text_muted),
                                );
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let mut rgb = colour.0;
                                    let response = color_palette::edit_srgb(ui, &mut rgb);
                                    if response.changed() {
                                        let to = (Colour(rgb) != neutral)
                                            .then_some(MaterialValue::Colour(Colour(rgb)));
                                        // The shell closes the step when the
                                        // pointer is let go.
                                        intents.push(change(to, Gesture::Drag));
                                    }
                                    if colour != neutral
                                        && widgets::reset_button(ui, "Back to as made (white)")
                                    {
                                        intents.push(change(None, Gesture::Commit));
                                    }
                                },
                            );
                        });
                    }
                }
            }
        });
    }
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Until skin shading lands, the preview draws the tint and roughness only; the \
             other values are saved and take effect then.",
        )
        .small()
        .color(palette.text_weak),
    );
    intents
}
