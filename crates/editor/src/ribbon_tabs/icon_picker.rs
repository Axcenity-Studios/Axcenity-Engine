//! Custom tabs' icons: any Phosphor icon, saved by name, and the picker in
//! the tab editor (a search over every icon, suggestions before anything's
//! typed).

use eframe::egui;

use super::editor::TabEditor;
use crate::theme::{self, icons, type_scale};
use crate::widgets;

/// One icon in the picker's grid.
const ICON_CELL: f32 = 32.0;

/// The icon a custom tab has until another is picked.
pub const DEFAULT_ICON: &str = "STAR";

/// The glyph of the Phosphor icon called `name`, or the star if there's
/// none by that name.
pub fn icon_glyph(name: &str) -> &'static str {
    icons::ICONS
        .iter()
        .find(|(icon, _)| *icon == name)
        .map_or(icons::STAR, |(_, glyph)| glyph)
}

/// An icon's name as people read it: `GAME_CONTROLLER` is "Game controller".
pub fn icon_label(name: &str) -> String {
    let words = name.to_lowercase().replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The icons the picker offers before anything is typed: one for most
/// things a tab gathers. Typing searches every Phosphor icon.
pub const SUGGESTED_ICONS: &[&str] = &[
    "STAR",
    "HEART",
    "LIGHTNING",
    "FIRE",
    "SPARKLE",
    "ROCKET",
    "CUBE",
    "SHAPES",
    "HAMMER",
    "WRENCH",
    "PAINT_BRUSH",
    "PALETTE",
    "MAGIC_WAND",
    "TREE",
    "MOUNTAINS",
    "SUN",
    "MOON",
    "LIGHTBULB",
    "CAMERA",
    "FILM_SLATE",
    "MUSIC_NOTES",
    "SPEAKER_HIGH",
    "GAME_CONTROLLER",
    "SWORD",
    "SHIELD",
    "CROWN",
    "TROPHY",
    "FLAG",
    "MAP_PIN",
    "GLOBE_HEMISPHERE_WEST",
    "HOUSE",
    "BUILDINGS",
    "CAR",
    "PERSON",
    "USERS_THREE",
    "ROBOT",
    "BUG",
    "FLASK",
    "CODE",
    "TERMINAL_WINDOW",
    "PUZZLE_PIECE",
    "LAYOUT",
    "SQUARES_FOUR",
    "DIAMOND",
    "LEAF",
    "DROP",
    "SNOWFLAKE",
    "PLANET",
    "ATOM",
    "BOOKMARK_SIMPLE",
    "PUSH_PIN",
    "CHART_BAR",
];

/// The Phosphor icons whose names have every word of `query` in them, in
/// the font's (alphabetical) order. An empty query gives the suggestions.
pub fn find_icons(query: &str) -> Vec<&'static str> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return SUGGESTED_ICONS.to_vec();
    }
    let words: Vec<&str> = query.split([' ', '_']).filter(|w| !w.is_empty()).collect();
    icons::ICONS
        .iter()
        .filter(|(name, _)| {
            let name = name.to_lowercase();
            words.iter().all(|word| name.contains(word))
        })
        .map(|(name, _)| *name)
        .collect()
}

/// A search field over every Phosphor icon and a grid of what matches
/// (the suggestions when nothing's typed). Picking one closes it.
pub(super) fn icon_picker(ui: &mut egui::Ui, editor: &mut TabEditor, palette: &theme::Palette) {
    let mut picked = None;
    egui::Frame::new()
        .fill(palette.inset)
        .stroke(egui::Stroke::new(1.0_f32, palette.border))
        .corner_radius(theme::radius::CARD)
        .inner_margin(theme::space::margin(theme::space::S, theme::space::S))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let count = icons::ICONS.len();
                let hint = format!("Search {},{:03} icons", count / 1000, count % 1000);
                widgets::search_field(ui, &mut editor.icon_query, &hint, 220.0);
                let found = find_icons(&editor.icon_query).len();
                let note = if editor.icon_query.trim().is_empty() {
                    "Suggested. Type to search every icon.".to_owned()
                } else if found == 0 {
                    "No icon's name has that in it.".to_owned()
                } else {
                    format!("{found} found")
                };
                ui.label(
                    egui::RichText::new(note)
                        .size(type_scale::CAPTION)
                        .color(palette.text_weak),
                );
            });
            ui.add_space(theme::space::XS);
            let found = find_icons(&editor.icon_query);
            let gap = 2.0;
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
            let columns = ((ui.available_width() + gap) / (ICON_CELL + gap))
                .floor()
                .max(1.0) as usize;
            let rows = found.len().div_ceil(columns);
            egui::ScrollArea::vertical()
                .id_salt("icon grid")
                .max_height(4.0 * (ICON_CELL + gap))
                .auto_shrink([false, true])
                .show_rows(ui, ICON_CELL, rows, |ui, range| {
                    for row in range {
                        ui.horizontal(|ui| {
                            for &name in found.iter().skip(row * columns).take(columns) {
                                if icon_cell(ui, name, name == editor.icon, palette).clicked() {
                                    picked = Some(name);
                                }
                            }
                        });
                    }
                });
        });
    if let Some(name) = picked {
        editor.icon = name.to_owned();
        editor.picking_icon = false;
    }
}

/// One icon in the picker: highlighted when hovered, filled when it's the
/// tab's.
fn icon_cell(
    ui: &mut egui::Ui,
    name: &str,
    selected: bool,
    palette: &theme::Palette,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ICON_CELL, ICON_CELL), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, icon_label(name))
    });
    let hovered = response.hovered();
    if selected {
        ui.painter().rect(
            rect,
            theme::radius::CONTROL,
            palette.accent_soft,
            egui::Stroke::new(1.0_f32, palette.accent),
            egui::StrokeKind::Inside,
        );
    } else if hovered {
        ui.painter()
            .rect_filled(rect, theme::radius::CONTROL, palette.control_hover);
    }
    let ink = if selected {
        palette.accent
    } else if hovered {
        palette.text_strong
    } else {
        palette.text_secondary
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon_glyph(name),
        egui::FontId::proportional(18.0),
        ink,
    );
    response.on_hover_text(icon_label(name))
}
