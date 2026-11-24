//! The Clothing page's online wardrobe (`wardrobe.rs`), shown in place of
//! the library's tiles when the page's switch is on "Online wardrobe":
//! what the Axcenity Wardrobe has for the character's body, as picture
//! tiles with a download button each. It follows the page's search and
//! category. Nothing is fetched until the list is asked for; a download
//! shows its progress and can be cancelled. This only draws the state and
//! returns intents.

use axcenity_avatar::BaseManifest;
use axcenity_toolbox::character_pack::PackItem;
use axcenity_toolbox::license;
use eframe::egui::{self, Sense, vec2};

use super::super::CharacterWorkspace;
use super::super::browser::Filters;
use super::super::intent::Intent;
use super::super::wardrobe::{Ask, State, Wardrobe};
use crate::theme::{self, icons};
use crate::toolbox_panel::size_text;
use crate::widgets;

/// The room between tiles, as the browser's.
const GAP: f32 = super::browser::GAP;
/// A tile's label and its button, under its picture.
const LABEL: f32 = 20.0;
const BUTTON: f32 = 22.0;

/// Draws the section and returns what it asks for.
pub(super) fn section(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    base: &BaseManifest,
    filters: &Filters,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let wardrobe = &workspace.wardrobe;
    let mut intents = Vec::new();
    ui.add_space(4.0);

    let Some(pack) = &wardrobe.pack else {
        ui.label(
            egui::RichText::new(
                "More clothes can be downloaded from the Axcenity Wardrobe. Nothing is fetched \
                 until you ask.",
            )
            .color(palette.text_weak),
        );
        ui.add_space(4.0);
        if wardrobe.asking() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Asking the wardrobe…");
            });
        } else if ui
            .add(widgets::button(
                widgets::Role::Secondary,
                Some(icons::CLOUD_ARROW_DOWN),
                "Show the online wardrobe",
                &palette,
            ))
            .on_hover_text("Asks the Axcenity Wardrobe what it has (needs the internet)")
            .clicked()
        {
            intents.push(Intent::Wardrobe(Ask::List));
        }
        problem(ui, wardrobe);
        return intents;
    };

    // The pack, and asking again.
    ui.horizontal(|ui| {
        let title = format!("{} {}", pack.name, pack.version);
        ui.label(egui::RichText::new(title.trim()).color(palette.text_strong));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if wardrobe.asking() {
                ui.spinner();
            } else if widgets::icon_button(
                ui,
                icons::ARROWS_CLOCKWISE,
                "Ask the wardrobe again for what it has",
                false,
            )
            .clicked()
            {
                intents.push(Intent::Wardrobe(Ask::List));
            }
        });
    });
    let mut items: Vec<&PackItem> = wardrobe.items_for(&base.id).collect();
    let licence = license::normalize(&pack.licence);
    ui.label(
        egui::RichText::new(format!(
            "{} for this body · {}. {}",
            counted(items.len(), "garment"),
            pack.licence.replace('-', " "),
            license::plain_words(licence)
        ))
        .color(palette.text_weak),
    );
    if !pack.credit.is_empty()
        && ui
            .add(widgets::button(
                widgets::Role::Ghost,
                Some(icons::COPY),
                "Copy credit line",
                &palette,
            ))
            .on_hover_text(&pack.credit)
            .clicked()
    {
        ui.ctx().copy_text(pack.credit.clone());
    }
    if !wardrobe.fresh && !wardrobe.asking() {
        ui.label(
            egui::RichText::new("As listed the last time you asked.")
                .small()
                .color(palette.text_muted),
        );
    }
    if pack.refused > 0 {
        ui.label(
            egui::RichText::new(format!(
                "{} left out: this editor can't take them. A newer editor may.",
                counted(pack.refused, "item")
            ))
            .small()
            .color(palette.text_muted),
        );
    }
    problem(ui, wardrobe);
    ui.add_space(4.0);

    // The download under way, or the way to get everything.
    if let Some(job) = &wardrobe.job {
        let fraction = job.done() as f32 / job.bytes.max(1) as f32;
        ui.add(
            egui::ProgressBar::new(fraction)
                .text(format!(
                    "{}: {} of {}",
                    job.label,
                    size_text(job.done()),
                    size_text(job.bytes)
                ))
                .desired_width(ui.available_width()),
        );
        ui.horizontal(|ui| {
            if !wardrobe.queue.is_empty() {
                ui.label(
                    egui::RichText::new(format!("{} waiting", wardrobe.queue.len()))
                        .color(palette.text_weak),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(widgets::button(
                        widgets::Role::Secondary,
                        Some(icons::X),
                        "Cancel",
                        &palette,
                    ))
                    .on_hover_text("Stop this download and forget the ones waiting")
                    .clicked()
                {
                    intents.push(Intent::Wardrobe(Ask::Cancel));
                }
            });
        });
        ui.add_space(4.0);
    } else {
        let (missing, bytes) = wardrobe.missing(&base.id);
        if missing > 1
            && ui
                .add(widgets::button(
                    widgets::Role::Secondary,
                    Some(icons::DOWNLOAD_SIMPLE),
                    &format!("Download all {missing} ({})", size_text(bytes)),
                    &palette,
                ))
                .on_hover_text("Downloads every garment for this body, one after another")
                .clicked()
        {
            intents.push(Intent::Wardrobe(Ask::GetAll));
        }
        ui.add_space(4.0);
    }

    if items.is_empty() {
        ui.label(
            egui::RichText::new("Nothing in the wardrobe is made for this body yet.")
                .color(palette.text_weak),
        );
        return intents;
    }
    // The page's category, cut and search narrow these too.
    let feminine = super::super::browser::feminine_body(&workspace.description, base);
    items.retain(|item| filters.cut.lists(&item.catalogue.tags, feminine));
    items.retain(|item| filters.slot.as_ref().is_none_or(|slot| item.slot == *slot));
    items.retain(|item| matches(item, slot_label(base, item), &filters.search));
    let order = |item: &PackItem| base.slots.iter().position(|slot| slot.id == item.slot);
    items.sort_by(|a, b| order(a).cmp(&order(b)).then_with(|| a.label.cmp(&b.label)));
    if items.is_empty() {
        ui.label(
            egui::RichText::new("Nothing in the wardrobe matches the search or the category.")
                .color(palette.text_weak),
        );
        return intents;
    }
    // As many across as the browser's tiles.
    let columns = super::tile_columns(ui.available_width(), GAP);
    let width = ((ui.available_width() - GAP * (columns as f32 - 1.0)) / columns as f32).floor();
    for row in items.chunks(columns) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for item in row {
                if let Some(ask) = tile(ui, wardrobe, item, slot_label(base, item), width) {
                    intents.push(Intent::Wardrobe(ask));
                }
            }
        });
        ui.add_space(GAP);
    }
    intents
}

/// What the body calls the slot `item` goes in ("Outerwear"), else the
/// slot's ID.
fn slot_label<'a>(base: &'a BaseManifest, item: &'a PackItem) -> &'a str {
    base.slot(&item.slot)
        .map_or(item.slot.as_str(), |slot| slot.label.as_str())
}

/// Why the last ask failed, if it did.
fn problem(ui: &mut egui::Ui, wardrobe: &Wardrobe) {
    if let Some(problem) = &wardrobe.problem {
        let palette = theme::palette(ui.ctx());
        ui.label(egui::RichText::new(problem).color(palette.error));
    }
}

/// "1 garment", "30 garments".
fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// Whether every word of `search` is in the item's label, ID or slot.
fn matches(item: &PackItem, slot: &str, search: &str) -> bool {
    let haystack = format!("{} {} {slot}", item.label, item.id).to_lowercase();
    search
        .split_whitespace()
        .all(|word| haystack.contains(&word.to_lowercase()))
}

/// One garment of the wardrobe: its picture, its label and, under them,
/// what can be done with it (download, its progress, remove).
fn tile(
    ui: &mut egui::Ui,
    wardrobe: &Wardrobe,
    item: &PackItem,
    slot: &str,
    width: f32,
) -> Option<Ask> {
    let palette = theme::palette(ui.ctx());
    let state = wardrobe.state(&item.id);
    let job = wardrobe.job.as_ref().filter(|job| job.id == item.id);
    let waiting = wardrobe.waiting(&item.id);
    let picture_height = (width * 1.25).round();
    let size = vec2(width, picture_height + LABEL + BUTTON);
    let mut ask = None;
    // The tile's own rectangle first, and its parts in a child that takes
    // no room of its own, so a row's tiles line up whatever each holds.
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let layout = egui::Layout::top_down(egui::Align::Min);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    {
        let ui = &mut child;
        ui.spacing_mut().item_spacing.y = 0.0;
        let (image, response) = ui.allocate_exact_size(vec2(width, picture_height), Sense::hover());
        let painter = ui.painter_at(image);
        painter.rect_filled(image, theme::radius::CARD, palette.inset);
        match wardrobe.picture(&item.id) {
            Some(texture) => {
                painter.image(
                    texture.id(),
                    image,
                    cropped(texture.size_vec2(), image.size()),
                    egui::Color32::WHITE,
                );
            }
            None => {
                painter.text(
                    image.center(),
                    egui::Align2::CENTER_CENTER,
                    icons::T_SHIRT,
                    egui::FontId::proportional(22.0),
                    palette.text_muted,
                );
            }
        }
        if state == State::Installed {
            painter.rect_stroke(
                image,
                theme::radius::CARD,
                egui::Stroke::new(1.0, palette.accent),
                egui::StrokeKind::Inside,
            );
        }
        // Badges along the picture's foot, as a browser tile's.
        let mut x = image.left() + 4.0;
        let mut badge = |text: &str, fill: egui::Color32, colour: egui::Color32| {
            let galley =
                painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(10.5), colour);
            let at = egui::pos2(x + 4.0, image.bottom() - 4.0 - galley.size().y - 2.0);
            let back = egui::Rect::from_min_size(at, galley.size()).expand2(vec2(4.0, 2.0));
            painter.rect_filled(back, 3.0, fill);
            x = back.right() + 3.0;
            painter.galley(at, galley, colour);
        };
        // One badge: a tile is too narrow for two (the tooltip says both).
        if state == State::Installed {
            badge(
                &format!("{} In library", icons::CHECK),
                palette.accent_solid,
                palette.on_accent,
            );
        } else if item.is_draft() {
            badge("Draft", palette.elevated, palette.warning);
        }
        let mut tip = format!("{} ({slot}), {}", item.label, size_text(item.bytes()));
        if item.is_draft() {
            tip.push_str("\nA draft: it hasn't passed the quality checks yet.");
        }
        tip.push_str(match state {
            State::Installed => "\nIn your library: wear it from Your library.",
            State::Outdated => "\nIn your library, in an older version.",
            State::Taken => "\nYour library has an item of its own with this ID.",
            State::Available => "\nNot in your library yet.",
        });
        response.on_hover_text(tip);

        let (label_rect, _) = ui.allocate_exact_size(vec2(width, LABEL), Sense::hover());
        let label = widgets::elided(
            ui,
            &item.label,
            egui::FontId::proportional(12.0),
            palette.text_secondary,
            width - 4.0,
            1,
        );
        ui.painter().galley(
            egui::pos2(label_rect.left() + 2.0, label_rect.top() + 3.0),
            label,
            palette.text_secondary,
        );

        let button_size = vec2(width, BUTTON);
        let small = |text: String| egui::RichText::new(text).size(11.5);
        let button = |text: String| {
            egui::Button::new(small(text))
                .corner_radius(theme::radius::CONTROL)
                .min_size(button_size)
        };
        if let Some(job) = job {
            let fraction = job.done() as f32 / job.bytes.max(1) as f32;
            ui.add(
                egui::ProgressBar::new(fraction)
                    .text(small(format!("{:.0}%", fraction * 100.0)))
                    .desired_width(width)
                    .desired_height(BUTTON),
            )
            .on_hover_text(format!(
                "Downloading: {} of {}",
                size_text(job.done()),
                size_text(job.bytes)
            ));
        } else if waiting {
            ui.add_enabled(false, button("Waiting".to_owned()))
                .on_disabled_hover_text("It downloads after the one under way");
        } else {
            match state {
                State::Available | State::Outdated => {
                    let (text, tip) = if state == State::Outdated {
                        (
                            format!("{} Update", icons::DOWNLOAD_SIMPLE),
                            format!(
                                "The wardrobe has a newer {}: download it ({})",
                                item.label,
                                size_text(item.bytes())
                            ),
                        )
                    } else {
                        (
                            format!("{} {}", icons::DOWNLOAD_SIMPLE, size_text(item.bytes())),
                            format!(
                                "Download {} into your library ({})",
                                item.label,
                                size_text(item.bytes())
                            ),
                        )
                    };
                    if ui.add(button(text)).on_hover_text(tip).clicked() {
                        ask = Some(Ask::Get(item.id.clone()));
                    }
                }
                State::Installed => {
                    if ui
                        .add(button(format!("{} Remove", icons::TRASH)))
                        .on_hover_text(format!(
                            "Take {} out of your library. It can be downloaded again.",
                            item.label
                        ))
                        .clicked()
                    {
                        ask = Some(Ask::Remove(item.id.clone()));
                    }
                }
                State::Taken => {
                    ui.add_enabled(false, button("Yours".to_owned()))
                        .on_disabled_hover_text(
                            "Your library has an item of its own with this ID, so the \
                             wardrobe's isn't downloaded over it",
                        );
                }
            }
        }
    }
    ask
}

/// The part of a picture `texture` in size that fills a rectangle
/// `shown` in size without stretching: its middle, cut at the sides or at
/// the top and bottom.
pub(super) fn cropped(texture: egui::Vec2, shown: egui::Vec2) -> egui::Rect {
    let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if texture.x <= 0.0 || texture.y <= 0.0 || shown.x <= 0.0 || shown.y <= 0.0 {
        return whole;
    }
    let (have, want) = (texture.x / texture.y, shown.x / shown.y);
    if have > want {
        let half = want / have / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.5 - half, 0.0), egui::pos2(0.5 + half, 1.0))
    } else {
        let half = have / want / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, 0.5 - half), egui::pos2(1.0, 0.5 + half))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_picture_is_cut_at_the_sides_for_a_tall_tile() {
        let uv = cropped(vec2(256.0, 256.0), vec2(80.0, 100.0));
        assert!((uv.min.x - 0.1).abs() < 1e-5 && (uv.max.x - 0.9).abs() < 1e-5);
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
        let uv = cropped(vec2(100.0, 200.0), vec2(100.0, 100.0));
        assert_eq!((uv.min.x, uv.max.x), (0.0, 1.0));
        assert!((uv.min.y - 0.25).abs() < 1e-5 && (uv.max.y - 0.75).abs() < 1e-5);
    }

    #[test]
    fn counts_read_as_words() {
        assert_eq!(counted(1, "garment"), "1 garment");
        assert_eq!(counted(30, "garment"), "30 garments");
    }
}
