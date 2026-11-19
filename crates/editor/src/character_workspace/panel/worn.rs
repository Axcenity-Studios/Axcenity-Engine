//! A worn item's own controls under the browser's tiles (brief §7.2,
//! §8.3, §14.1): only what the item declares ([`Declared`]). Its variants
//! as chips; for a hairstyle, natural colours (melanin and redness) for
//! its colour, and for its highlights and roots only where the style has
//! slots with those roles; for a garment, a colour button per colour role
//! (primary, secondary, accent); its own parameters as sliders. A style
//! that declares no highlights shows no highlight control.

use axcenity_avatar::{
    Colour, ItemKind, ItemManifest, MaterialRole, MaterialValue, ValueKind, WornItem,
};
use eframe::egui;

use super::super::browser::{COLOUR_VALUE, Declared, HAIR_COLOURS, role_label};
use super::super::intent::{Gesture, Intent};
use super::super::selection::Level;
use super::row::{self, Row};
use crate::color_palette;
use crate::theme;
use crate::widgets;

/// The value `value` of material slot `slot` as the worn item shows it:
/// the person's own, else its variant's (not known here: `None`).
fn held(worn: &WornItem, slot: &str, value: &str) -> Option<MaterialValue> {
    worn.materials
        .get(slot)
        .and_then(|values| values.get(value))
        .copied()
}

/// The value `value` of `slot` on `item` worn as `worn`: the person's own,
/// else its variant's, else neutral (`None`).
fn current(item: &ItemManifest, worn: &WornItem, slot: &str, value: &str) -> Option<MaterialValue> {
    held(worn, slot, value).or_else(|| {
        let variant = item.variant(worn.variant.as_deref()?)?;
        variant.materials.get(slot)?.get(value).copied()
    })
}

fn number(value: Option<MaterialValue>) -> f32 {
    match value {
        Some(MaterialValue::Number(number)) => number,
        _ => 0.0,
    }
}

/// The studio's level of an item parameter (`detailed` unless it says).
fn item_level(parameter: &axcenity_avatar::ItemParameter) -> Level {
    match parameter.level() {
        axcenity_avatar::Level::Quick => Level::Quick,
        axcenity_avatar::Level::Advanced => Level::Advanced,
        _ => Level::Detailed,
    }
}

/// Draws the controls of `item`, worn as `worn`, at `level`.
pub(super) fn controls(
    ui: &mut egui::Ui,
    item: &ItemManifest,
    worn: &WornItem,
    level: Level,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let declared = Declared::of(item);
    let groom = item.kind == ItemKind::Groom;
    // Worn whatever its state (ADR-0043), and said so.
    let title = super::super::browser::marked_label(item);
    widgets::section(theme::icons::SLIDERS_HORIZONTAL, &title)
        .id_salt(&format!("worn item {}", item.id))
        .show(ui, |ui| {
            if let Some(about) = &item.about {
                ui.label(egui::RichText::new(about).small().color(palette.text_weak));
            }
            if !item.variants.is_empty() {
                widgets::section_label(ui, "Variant");
                ui.horizontal_wrapped(|ui| {
                    if widgets::chip(ui, "As made", worn.variant.is_none()).clicked()
                        && worn.variant.is_some()
                    {
                        intents.push(Intent::SetItemVariant {
                            item: item.id.clone(),
                            variant: None,
                        });
                    }
                    for variant in &item.variants {
                        let on = worn.variant.as_deref() == Some(variant.id.as_str());
                        if widgets::chip(ui, &variant.label, on).clicked() && !on {
                            intents.push(Intent::SetItemVariant {
                                item: item.id.clone(),
                                variant: Some(variant.id.clone()),
                            });
                        }
                    }
                });
                ui.add_space(4.0);
            }
            for (role, slots) in &declared.natural {
                intents.extend(natural_colours(ui, item, worn, *role, slots));
            }
            // A hairstyle's natural colours come first; a tint over them is
            // a fine control (ux patterns P4: coarse first).
            if !groom || level == Level::Advanced {
                for (role, slots) in &declared.roles {
                    intents.extend(colour_button(ui, item, worn, *role, slots, groom));
                }
            }
            for parameter in item
                .parameters
                .iter()
                .filter(|p| level.shows(item_level(p)))
            {
                let value = worn.parameters.get(&parameter.id).copied().or_else(|| {
                    item.variant(worn.variant.as_deref()?)?
                        .parameters
                        .get(&parameter.id)
                        .copied()
                });
                let value = value.unwrap_or(parameter.default);
                let id = format!("{}.{}", item.id, parameter.id);
                let about = parameter
                    .about
                    .clone()
                    .unwrap_or_else(|| parameter.label.clone());
                // The slider row is shared with Body and Face, so it follows
                // the drag rule (`numeric.rs`: a full range in 110 points,
                // Shift 25%, Ctrl 2x, a typed value).
                if let Some(row::Asked::Set(value, gesture)) = row::slider(
                    ui,
                    &Row {
                        id: &id,
                        label: &parameter.label,
                        note: None,
                        about: &about,
                        value,
                        range: parameter.range,
                        default: parameter.default,
                        locked: None,
                    },
                ) {
                    intents.push(Intent::SetItemParameter {
                        item: item.id.clone(),
                        id: parameter.id.clone(),
                        value,
                        gesture,
                    });
                }
            }
            for limitation in &item.limitations {
                ui.label(
                    egui::RichText::new(format!("{} {}", theme::icons::INFO, limitation.note))
                        .small()
                        .color(palette.text_weak),
                );
            }
        });
    intents
}

/// A hairstyle's natural colours for the slots of `role` (its colour, its
/// highlights or its roots): a swatch per colour, lit when the slots have
/// it, setting `melanin` and `redness` together (one undo step) and
/// clearing a tint.
fn natural_colours(
    ui: &mut egui::Ui,
    item: &ItemManifest,
    worn: &WornItem,
    role: MaterialRole,
    slots: &[String],
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let first = &slots[0];
    let melanin = number(current(item, worn, first, "melanin"));
    let redness = number(current(item, worn, first, "redness"));
    widgets::section_label(ui, role_label(role, true));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for (name, m, r, swatch) in HAIR_COLOURS {
            let on = (melanin - m).abs() < 1.0e-3 && (redness - r).abs() < 1.0e-3;
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
            let painter = ui.painter_at(rect.expand(2.0));
            painter.circle_filled(
                rect.center(),
                9.0,
                egui::Color32::from_rgb(swatch[0], swatch[1], swatch[2]),
            );
            if on || response.hovered() {
                painter.circle_stroke(
                    rect.center(),
                    10.5,
                    egui::Stroke::new(
                        if on { 2.0 } else { 1.0 },
                        if on {
                            palette.accent
                        } else {
                            palette.text_muted
                        },
                    ),
                );
            }
            let response = response.on_hover_text(name);
            if response.clicked() && !on {
                let neutral = |v: f32| (v.abs() > 1.0e-4).then_some(MaterialValue::Number(v));
                intents.push(Intent::SetItemMaterial {
                    item: item.id.clone(),
                    slots: slots.to_vec(),
                    values: vec![
                        ("melanin".to_owned(), neutral(m)),
                        ("redness".to_owned(), neutral(r)),
                        (COLOUR_VALUE.to_owned(), None),
                    ],
                    gesture: Gesture::Commit,
                });
            }
        }
    });
    ui.add_space(4.0);
    intents
}

/// A colour button for the slots of `role`: their tint.
fn colour_button(
    ui: &mut egui::Ui,
    item: &ItemManifest,
    worn: &WornItem,
    role: MaterialRole,
    slots: &[String],
    groom: bool,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let neutral = item
        .material(&slots[0])
        .and_then(|slot| slot.shading.value(COLOUR_VALUE))
        .and_then(|spec| match spec.kind {
            ValueKind::Colour { neutral } => Some(neutral),
            ValueKind::Number { .. } => None,
        })
        .unwrap_or(Colour::WHITE);
    let colour = match current(item, worn, &slots[0], COLOUR_VALUE) {
        Some(MaterialValue::Colour(colour)) => colour,
        _ => neutral,
    };
    // A hairstyle's colour is set by its natural colours above; the
    // button is a tint over them.
    let label = if groom {
        format!("{} tint", role_label(role, true))
    } else {
        role_label(role, false).to_owned()
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&label).color(palette.text_secondary))
            .on_hover_text(format!(
                "The {} of {}",
                label.to_lowercase(),
                slots.join(", ")
            ));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut rgb = colour.0;
            let change = |to: Option<MaterialValue>, gesture| Intent::SetItemMaterial {
                item: item.id.clone(),
                slots: slots.to_vec(),
                values: vec![(COLOUR_VALUE.to_owned(), to)],
                gesture,
            };
            // TODO(ux-drag): the colour picker's own HSV and value fields
            // are numeric controls too (the drag rule: a full range in
            // 100 to 120 points, Shift about 25%, Ctrl about 2x, typed
            // values); they are egui's and shared with the Skin page, so
            // the rule reaches them through `color_palette`, not here.
            if color_palette::edit_srgb(ui, &mut rgb).changed() {
                let to = (Colour(rgb) != neutral).then_some(MaterialValue::Colour(Colour(rgb)));
                // The shell closes the step when the pointer is let go.
                intents.push(change(to, Gesture::Drag));
            }
            if held(worn, &slots[0], COLOUR_VALUE).is_some()
                && widgets::reset_button(ui, "Back to as made")
            {
                intents.push(change(None, Gesture::Commit));
            }
        });
    });
    intents
}
