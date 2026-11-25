//! The Skin and Makeup page's material values: every value the base's skin
//! materials take (`avatar::ShadingModel::Skin`; the realistic body's slot
//! is "Skin"), a slider for a number and a colour button for a colour. A
//! value left at its neutral setting is not written, so the material looks
//! as made.
//!
//! The renderer draws only some of them until the skin shading model lands
//! (CO7): the page says which.

use axcenity_avatar::{
    BaseManifest, Colour, MaterialValue, ShadingModel, SkinLayer, SkinLayerKind, ValueKind,
};
use eframe::egui;

use super::super::CharacterWorkspace;
use super::super::intent::{Gesture, Intent};
use super::row::{self, Row};
use crate::color_palette;
use crate::theme;
use crate::widgets;

/// The values the renderer draws today: `scene`'s material values (CO7's
/// first slice), the Skin shading model's (R5) and the Hair and Cloth
/// models' (CO7, wave 4: on a material its model file marks hair or
/// cloth). The Eye model's own are kept in the description and drawn once
/// its shading model reads them.
pub(in super::super) const DRAWN_TODAY: &[&str] = &[
    "tint",
    "iris_tint",
    "sclera_tint",
    "roughness_scale",
    "detail_strength",
    "melanin",
    "undertone",
    "redness",
    "scatter_scale",
    "definition",
    "softness",
    "dye",
    "dye_amount",
    "tip_lightening",
    "highlight",
    "root_melanin",
    "root_length",
    "streaks",
    "streak_melanin",
    "shift_scale",
    "sheen_tint",
    "sheen_roughness_scale",
    "anisotropy_scale",
    "anisotropy_rotation",
];

/// A value's name for people: `roughness_scale` is "Roughness scale".
pub(in super::super) fn value_label(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Draws the skin materials' values and returns what they ask for, and
/// whether any control was drawn. `search` is the panel's "Search
/// controls" text: with any, only the values and layers that match it
/// (their label, name and meaning) are drawn.
pub(super) fn skin(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    base: &BaseManifest,
    search: &str,
) -> (Vec<Intent>, bool) {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let searching = !search.trim().is_empty();
    let slots: Vec<_> = base
        .materials
        .iter()
        .filter(|slot| slot.shading == ShadingModel::Skin)
        .collect();
    if slots.is_empty() {
        if !searching {
            widgets::empty_state(
                ui,
                crate::theme::icons::DROP,
                "No skin material",
                "This body has no material with skin shading to adjust.",
                None,
            );
        }
        return (intents, false);
    }
    let mut any = false;
    for slot in slots {
        // What the slot offers: a skin with no detail or definition maps
        // (the human base's) doesn't show sliders that would do nothing.
        let specs: Vec<_> = slot
            .offered()
            .into_iter()
            .filter(|spec| {
                super::has_words(&[&value_label(spec.name), spec.name, spec.meaning], search)
            })
            .collect();
        if specs.is_empty() {
            continue;
        }
        any = true;
        widgets::section(
            crate::theme::icons::DROP,
            &format!("{} material", slot.name),
        )
        .id_salt(&slot.name)
        .forced(searching.then_some(true))
        .show(ui, |ui| {
            for spec in specs {
                let drawn = DRAWN_TODAY.contains(&spec.name);
                let about = if drawn {
                    spec.meaning.to_owned()
                } else {
                    format!(
                        "{} Saved with the character; the renderer draws it once its \
                             shading model does.",
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
    let (asked, found) = makeup(ui, workspace, base, search);
    intents.extend(asked);
    any |= found;
    (intents, any)
}

/// The base's makeup layers and markings (R5, ADR-0042): each turned on
/// or off by its check box; when on, its colour (makeup), opacity,
/// roughness (makeup), amount and pattern (markings). Each change asks for
/// the whole layer ([`Intent::SetSkinLayer`]): one undo step, or one per
/// slider drag.
fn makeup(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    base: &BaseManifest,
    search: &str,
) -> (Vec<Intent>, bool) {
    let mut intents = Vec::new();
    let searching = !search.trim().is_empty();
    // A layer found by a search is shown whole.
    let layers: Vec<_> = base
        .skin_layers
        .iter()
        .filter(|spec| spec.kind != SkinLayerKind::Other)
        .filter(|spec| {
            super::has_words(
                &[&spec.label, &spec.id, spec.about.as_deref().unwrap_or("")],
                search,
            )
        })
        .collect();
    if layers.is_empty() {
        return (intents, false);
    }
    let palette = theme::palette(ui.ctx());
    widgets::section(crate::theme::icons::PAINT_BRUSH, "Makeup and markings")
        .id_salt("makeup and markings")
        .forced(searching.then_some(true))
        .show(ui, |ui| {
            for spec in layers {
                let marking = spec.kind == SkinLayerKind::Marking;
                let held = workspace.skin_layer(&spec.id).cloned();
                let set = |layer: SkinLayer, gesture| Intent::SetSkinLayer {
                    id: spec.id.clone(),
                    marking,
                    to: Some(layer),
                    gesture,
                };
                let about = spec.about.as_deref().unwrap_or("");
                let mut on = held.as_ref().is_some_and(|layer| layer.enabled);
                let label = egui::RichText::new(&spec.label).color(palette.text_secondary);
                if ui.checkbox(&mut on, label).on_hover_text(about).changed() {
                    // Off keeps its settings; on starts from them, or from
                    // the base's.
                    let mut layer = held.clone().unwrap_or_else(|| SkinLayer::from_spec(spec));
                    layer.enabled = on;
                    intents.push(set(layer, Gesture::Commit));
                }
                let Some(layer) = held.filter(|layer| layer.enabled) else {
                    continue;
                };
                ui.indent(&spec.id, |ui| {
                    if !marking {
                        let colour = layer.colour.or(spec.colour);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Colour").color(palette.text_secondary));
                            if colour.is_none() {
                                ui.label(
                                    egui::RichText::new("own tone")
                                        .small()
                                        .color(palette.text_muted),
                                )
                                .on_hover_text(
                                    "Without a colour it evens the skin in its own tone.",
                                );
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let mut rgb = colour.unwrap_or(Colour([200, 160, 140])).0;
                                    if color_palette::edit_srgb(ui, &mut rgb).changed() {
                                        let mut changed = layer.clone();
                                        changed.colour = Some(Colour(rgb));
                                        // The shell closes the step when the
                                        // pointer is let go.
                                        intents.push(set(changed, Gesture::Drag));
                                    }
                                    if layer.colour.is_some()
                                        && widgets::reset_button(
                                            ui,
                                            "Back to the layer's own colour",
                                        )
                                    {
                                        let mut changed = layer.clone();
                                        changed.colour = None;
                                        intents.push(set(changed, Gesture::Commit));
                                    }
                                },
                            );
                        });
                    }
                    let mut number =
                        |ui: &mut egui::Ui,
                         field: &str,
                         words: [&str; 2],
                         value: f32,
                         default: f32,
                         apply: &dyn Fn(&mut SkinLayer, f32)| {
                            let id = format!("skin.{}.{field}", spec.id);
                            if let Some(row::Asked::Set(value, gesture)) = row::slider(
                                ui,
                                &Row {
                                    id: &id,
                                    label: words[0],
                                    note: None,
                                    about: words[1],
                                    value,
                                    range: [0.0, 1.0],
                                    default,
                                    locked: None,
                                },
                            ) {
                                let mut changed = layer.clone();
                                apply(&mut changed, value);
                                intents.push(set(changed, gesture));
                            }
                        };
                    number(
                        ui,
                        "opacity",
                        ["Opacity", "How much of it shows where it is whole."],
                        layer.opacity,
                        spec.opacity.unwrap_or(1.0),
                        &|layer, value| layer.opacity = value,
                    );
                    if !marking {
                        let own = spec.roughness.unwrap_or(0.5);
                        number(
                            ui,
                            "roughness",
                            ["Roughness", "Glossy (0) to matte (1) where it covers."],
                            layer.roughness.unwrap_or(own),
                            own,
                            &|layer, value| layer.roughness = Some(value),
                        );
                    } else if spec.id != "scar" {
                        number(
                            ui,
                            "amount",
                            ["Amount", "How many of its spots show."],
                            layer.amount.or(spec.amount).unwrap_or(1.0),
                            spec.amount.unwrap_or(1.0),
                            &|layer, value| layer.amount = Some(value),
                        );
                        let new = format!("{} New pattern", crate::theme::icons::SHUFFLE);
                        if ui
                            .button(new)
                            .on_hover_text("Other spots: a new seed.")
                            .clicked()
                        {
                            let mut changed = layer.clone();
                            changed.seed = Some(layer.seed.unwrap_or(0).wrapping_add(1));
                            intents.push(set(changed, Gesture::Commit));
                        }
                    }
                });
            }
        });
    (intents, true)
}
