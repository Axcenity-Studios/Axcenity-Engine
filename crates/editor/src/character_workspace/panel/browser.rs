//! The Hair and Clothing pages as browsers (R6 CO25, R7 CO26; brief §7.1,
//! §8.2): a search field, the page's slots as categories, filters (kind,
//! "Recolourable", "Fits this body", favourites, "Legacy items"), a row of
//! the items worn last and picture tiles of the character wearing each
//! item. Hovering a tile, or clicking it, shows it on the character
//! without wearing it; "Wear" (or a double click) wears it as one undo
//! step. Above the tiles, the worn items' own controls (`worn.rs`). What
//! is listed is `browser::list`; this only draws it and returns intents.
//! Drafts show only for content review (the toolbar's View menu,
//! ADR-0043); the page then says so.

use axcenity_avatar::{BaseManifest, Library, Quality};
use eframe::egui::{self, Sense, vec2};

use super::super::browser::{self, Browse, Entry, Filters, Pick, Shelf};
use super::super::intent::Intent;
use super::super::{CharacterWorkspace, nav};
use super::worn;
use crate::theme::{self, icons};
use crate::widgets;

/// Tiles across the panel.
const COLUMNS: usize = 3;
const GAP: f32 = 6.0;

/// What a tile shows and was asked.
struct Tile<'a> {
    pick: Pick,
    label: &'a str,
    /// The picture's key in the workspace (an item ID, or `""` + a slot).
    picture: String,
    /// Marked "Draft" or "Legacy" unless Production.
    state: Quality,
    worn: bool,
    favourite: Option<bool>,
    trying: bool,
    tip: String,
}

/// What a tile's pointer did.
#[derive(Default)]
struct Touched {
    hovered: bool,
    clicked: bool,
    double_clicked: bool,
    star: bool,
}

/// The ID of the "nothing worn" tile's picture for `slot`.
pub(in super::super) fn none_picture(slot: &str) -> String {
    format!("none:{slot}")
}

/// Draws page `shelf` and returns what it asks for.
pub(super) fn page(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
    base: &BaseManifest,
    shelf: Shelf,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let description = workspace.description();
    let state = &workspace.browser;
    let filters = state.filters(shelf).clone();
    let listing = browser::list(
        library,
        description,
        shelf,
        &filters,
        state.shown(),
        &workspace.item_favourites,
        &workspace.recent_items,
    );
    let set = |change: &dyn Fn(&mut Filters)| {
        let mut changed = filters.clone();
        change(&mut changed);
        (changed != filters).then_some(Intent::Browse(Browse::Filters(shelf, changed)))
    };

    for (slot, item) in &listing.body_type_wears {
        ui.label(
            egui::RichText::new(format!("{slot}: {item}, which comes with the body type."))
                .color(palette.text_weak),
        );
    }
    if !listing.body_type_wears.is_empty() {
        ui.add_space(6.0);
    }
    if listing.slots.is_empty() || (listing.offered == 0 && listing.entries.is_empty()) {
        nothing_offered(ui, shelf);
        return intents;
    }

    // Search.
    let mut search = filters.search.clone();
    let width = (ui.available_width() - widgets::SEARCH_FIELD_MARGINS).max(60.0);
    let hint = match shelf {
        Shelf::Hair => "Search hairstyles",
        Shelf::Clothing => "Search clothes",
    };
    widgets::search_field(ui, &mut search, hint, width);
    if search != filters.search {
        intents.extend(set(&|f| f.search = search.clone()));
    }
    ui.add_space(4.0);

    // The page's slots, as categories.
    if listing.slots.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            if widgets::chip(ui, "All", filters.slot.is_none()).clicked() {
                intents.extend(set(&|f| f.slot = None));
            }
            for slot in &listing.slots {
                let on = filters.slot.as_deref() == Some(slot.id.as_str());
                let response = widgets::chip(ui, &slot.label, on);
                let response = match &slot.about {
                    Some(about) => response.on_hover_text(about),
                    None => response,
                };
                if response.clicked() {
                    let id = slot.id.clone();
                    intents.extend(set(&|f| f.slot = (!on).then(|| id.clone())));
                }
            }
        });
        ui.add_space(2.0);
    }

    // Filters.
    ui.horizontal_wrapped(|ui| {
        if !listing.kinds.is_empty() {
            let current = filters
                .kind
                .clone()
                .unwrap_or_else(|| "Any kind".to_owned());
            egui::ComboBox::from_id_salt(("browser kind", shelf as u8))
                .selected_text(capitalised(&current))
                .width(96.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(filters.kind.is_none(), "Any kind")
                        .clicked()
                    {
                        intents.extend(set(&|f| f.kind = None));
                    }
                    for kind in &listing.kinds {
                        let on = filters.kind.as_deref() == Some(kind.as_str());
                        if ui.selectable_label(on, capitalised(kind)).clicked() {
                            let kind = kind.clone();
                            intents.extend(set(&|f| f.kind = Some(kind.clone())));
                        }
                    }
                })
                .response
                .on_hover_text("Show one kind only (the items' tags)");
        }
        if widgets::chip(ui, "Recolourable", filters.recolourable)
            .on_hover_text("Only items whose colours you can change")
            .clicked()
        {
            intents.extend(set(&|f| f.recolourable = !f.recolourable));
        }
        if widgets::chip(ui, "Fits this body", filters.fits)
            .on_hover_text("Hide items whose maker says they don't fit this body's shape")
            .clicked()
        {
            intents.extend(set(&|f| f.fits = !f.fits));
        }
        if widgets::chip(
            ui,
            &format!("{} Favourites", theme::filled_icons::STAR),
            filters.favourites,
        )
        .on_hover_text("Only your favourites")
        .clicked()
        {
            intents.extend(set(&|f| f.favourites = !f.favourites));
        }
        // The optional content filter (ADR-0043): only where there is
        // legacy content to show.
        if listing.hidden_legacy > 0 || state.show_legacy {
            let tip = if state.show_legacy {
                "Hide the older items kept for compatibility".to_owned()
            } else {
                format!(
                    "Show {} older items kept for compatibility, marked \"Legacy\". \
                     Randomize never uses them",
                    listing.hidden_legacy
                )
            };
            if widgets::chip(ui, "Legacy items", state.show_legacy)
                .on_hover_text(tip)
                .clicked()
            {
                intents.push(Intent::Browse(Browse::ShowLegacy(!state.show_legacy)));
            }
        }
    });
    if state.show_drafts {
        ui.label(
            egui::RichText::new("Content review: drafts are shown, marked \"Draft\" (View menu).")
                .small()
                .color(palette.warning),
        );
    }
    ui.add_space(4.0);

    // What is being tried on, and the way to wear it. The bar is always
    // there (a hint when nothing is picked), so picking a tile never moves
    // the tiles under a still pointer: the tile above would be hovered.
    // A page with no tile at all (nothing listed, no slot filter giving a
    // "nothing worn" tile) has nothing to point at, so no bar.
    let no_tiles =
        listing.entries.is_empty() && listing.recent.is_empty() && filters.slot.is_none();
    if !no_tiles || state.selected.is_some() {
        intents.extend(trying_bar(ui, library, base, state.selected.as_ref()));
    }

    // The worn items' own controls (colours, variants, parameters), above
    // the tiles so they're seen without scrolling; each folds away.
    for worn in &description.items {
        let Some(item) = library.item(&worn.id) else {
            continue;
        };
        let on_page = listing.slots.iter().any(|slot| slot.id == item.slot);
        let in_filter = filters.slot.as_ref().is_none_or(|slot| *slot == item.slot);
        // An item with nothing to change and nothing to say has no section:
        // its tile says it's worn.
        let says = !browser::Declared::of(item).is_empty() || !item.limitations.is_empty();
        if on_page && in_filter && says {
            intents.extend(worn::controls(ui, item, worn, workspace.selection.level));
            ui.add_space(6.0);
        }
    }

    let mut hovered: Option<Pick> = None;
    if !listing.recent.is_empty() {
        widgets::section_label(ui, "Recently used");
        let tiles: Vec<Tile> = listing
            .recent
            .iter()
            .map(|entry| item_tile(entry, state.trying()))
            .collect();
        intents.extend(grid(ui, workspace, "recent", &tiles, &mut hovered));
        ui.add_space(6.0);
    }

    if let Some((title, sentence)) = browser::empty_message(&listing, shelf, &filters) {
        widgets::empty_state(ui, icons::MAGNIFYING_GLASS, &title, &sentence, None);
    }
    // By slot, each with its "nothing worn" tile first (on the whole list).
    for slot in &listing.slots {
        let entries: Vec<&Entry> = listing
            .entries
            .iter()
            .filter(|entry| entry.item.slot == slot.id)
            .collect();
        let whole = !filters.narrowed() || filters.slot.as_deref() == Some(slot.id.as_str());
        if entries.is_empty() && !(whole && filters.slot.is_some()) {
            continue;
        }
        if listing.slots.len() > 1 || entries.is_empty() {
            widgets::section_label(ui, &slot.label);
        }
        let mut tiles = Vec::new();
        if whole && filters.search.trim().is_empty() && !filters.favourites {
            let none = slot.empty_label.as_deref().unwrap_or("None");
            let pick = Pick {
                slot: slot.id.clone(),
                item: None,
            };
            let worn_here = description.items.iter().any(|worn| {
                library
                    .item(&worn.id)
                    .is_some_and(|item| item.slots().any(|s| *s == slot.id))
            });
            tiles.push(Tile {
                trying: state.trying() == Some(&pick),
                pick,
                label: none,
                picture: none_picture(&slot.id),
                state: Quality::Production,
                worn: !worn_here,
                favourite: None,
                tip: format!("Wear nothing here ({none})"),
            });
        }
        tiles.extend(entries.iter().map(|entry| item_tile(entry, state.trying())));
        intents.extend(grid(ui, workspace, &slot.id, &tiles, &mut hovered));
        ui.add_space(8.0);
    }
    // After Esc or a click the tiles can shift under a still pointer (the
    // worn controls change), so a muted tile waits for the pointer to move.
    let moved = ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO);
    if state.hover_changed(&hovered, moved) {
        intents.push(Intent::Browse(Browse::Hover(hovered)));
    }

    intents
}

/// A tile for `entry`.
fn item_tile<'a>(entry: &'a Entry, trying: Option<&Pick>) -> Tile<'a> {
    let pick = Pick {
        slot: entry.item.slot.clone(),
        item: Some(entry.item.id.clone()),
    };
    let mut tip = format!(
        "{} ({})",
        entry
            .item
            .about
            .clone()
            .unwrap_or_else(|| entry.item.label.clone()),
        entry.slot
    );
    match entry.state {
        Quality::Draft => tip.push_str("\nA draft: it hasn't passed the quality review yet."),
        Quality::Legacy => tip
            .push_str("\nLegacy: older content kept for compatibility. Randomize doesn't use it."),
        _ => {}
    }
    if let Some(note) = &entry.misfit {
        tip.push_str(&format!("\nMay not fit this body: {note}"));
    }
    if !entry.takes_off.is_empty() {
        tip.push_str(&format!(
            "\nWearing it takes off: {}.",
            entry.takes_off.join(", ")
        ));
    }
    tip.push_str(if entry.worn {
        "\nWorn."
    } else {
        "\nClick to try it on; double-click to wear it."
    });
    Tile {
        trying: trying == Some(&pick),
        pick,
        label: &entry.item.label,
        picture: entry.item.id.clone(),
        state: entry.state,
        worn: entry.worn,
        favourite: Some(entry.favourite),
        tip,
    }
}

/// "Trying on X": Wear and Cancel.
fn trying_bar(
    ui: &mut egui::Ui,
    library: &Library,
    base: &BaseManifest,
    pick: Option<&Pick>,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let label = pick.map(|pick| match &pick.item {
        Some(id) => library
            .item(id)
            .map_or_else(|| id.clone(), |item| item.label.clone()),
        None => base
            .slot(&pick.slot)
            .and_then(|slot| slot.empty_label.clone())
            .unwrap_or_else(|| "Nothing".to_owned()),
    });
    egui::Frame::new()
        .fill(palette.inset)
        .corner_radius(theme::radius::CARD)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // The same widgets either way, so the bar keeps its height.
                match &label {
                    Some(label) => ui
                        .label(
                            egui::RichText::new(format!("Trying on {label}"))
                                .color(palette.text_strong),
                        )
                        .on_hover_text(
                            "Shown on the character only: nothing is changed until you wear it",
                        ),
                    None => ui.label(
                        egui::RichText::new("Point at a tile to try it on")
                            .color(palette.text_muted),
                    ),
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wear = ui
                        .add_enabled(
                            pick.is_some(),
                            widgets::button(
                                widgets::Role::Primary,
                                Some(icons::CHECK),
                                "Wear",
                                &palette,
                            ),
                        )
                        .on_hover_text("Wear it (you can undo it)")
                        .on_disabled_hover_text("Click a tile first");
                    if let Some(pick) = pick.filter(|_| wear.clicked()) {
                        intents.push(Intent::Wear {
                            slot: pick.slot.clone(),
                            item: pick.item.clone(),
                        });
                    }
                    let back = ui.add_enabled_ui(pick.is_some(), |ui| {
                        widgets::icon_button(ui, icons::X, "Put it back (Esc)", false)
                    });
                    if back.inner.clicked() {
                        intents.push(Intent::EndPreview);
                    }
                });
            });
        });
    ui.add_space(6.0);
    intents
}

/// Lays `tiles` out in rows; notes the tile hovered in `hovered`.
fn grid(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    group: &str,
    tiles: &[Tile],
    hovered: &mut Option<Pick>,
) -> Vec<Intent> {
    // A tile can be in two grids (recently used and its slot's).
    ui.push_id(("browser grid", group), |ui| {
        grid_rows(ui, workspace, tiles, hovered)
    })
    .inner
}

fn grid_rows(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    tiles: &[Tile],
    hovered: &mut Option<Pick>,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let width = ((ui.available_width() - GAP * (COLUMNS as f32 - 1.0)) / COLUMNS as f32).floor();
    for row in tiles.chunks(COLUMNS) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for tile in row {
                let touched = draw_tile(ui, workspace.item_texture(&tile.picture), tile, width);
                if touched.hovered {
                    *hovered = Some(tile.pick.clone());
                }
                if touched.star {
                    if let Some(id) = &tile.pick.item {
                        intents.push(Intent::ToggleItemFavourite(id.clone()));
                    }
                } else if touched.double_clicked {
                    intents.push(Intent::Wear {
                        slot: tile.pick.slot.clone(),
                        item: tile.pick.item.clone(),
                    });
                } else if touched.clicked {
                    // What's worn has nothing to try on.
                    let pick = (!tile.worn).then(|| tile.pick.clone());
                    intents.push(Intent::Browse(Browse::Select(pick)));
                }
            }
        });
        ui.add_space(GAP);
    }
    intents
}

/// One tile: the character wearing it (a placeholder until drawn), its
/// label, "Draft" or "Legacy", a check when worn, a star for favourites.
fn draw_tile(
    ui: &mut egui::Ui,
    picture: Option<&egui::TextureHandle>,
    tile: &Tile,
    width: f32,
) -> Touched {
    let palette = theme::palette(ui.ctx());
    let picture_height = (width * 1.25).round();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, picture_height + 22.0), Sense::click());
    let image = egui::Rect::from_min_size(rect.min, vec2(width, picture_height));
    let painter = ui.painter_at(rect);
    painter.rect_filled(image, theme::radius::CARD, palette.inset);
    match picture {
        Some(texture) => {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            painter.image(texture.id(), image, uv, egui::Color32::WHITE);
        }
        None => {
            painter.text(
                image.center(),
                egui::Align2::CENTER_CENTER,
                if tile.pick.item.is_some() {
                    icons::T_SHIRT
                } else {
                    icons::PROHIBIT
                },
                egui::FontId::proportional(22.0),
                palette.text_muted,
            );
        }
    }
    let hovered = response.hovered();
    if tile.worn || tile.trying || hovered {
        let (width, colour) = if tile.worn {
            (2.0, palette.accent)
        } else if tile.trying {
            (2.0, palette.text_secondary)
        } else {
            (1.0, palette.text_muted)
        };
        painter.rect_stroke(
            image,
            theme::radius::CARD,
            egui::Stroke::new(width, colour),
            egui::StrokeKind::Inside,
        );
    }
    // Badges along the picture's foot: Draft or Legacy, Worn.
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
    match tile.state {
        Quality::Draft => badge("Draft", palette.elevated, palette.warning),
        Quality::Legacy => badge("Legacy", palette.elevated, palette.text_secondary),
        _ => {}
    }
    if tile.worn {
        badge(
            &format!("{} Worn", icons::CHECK),
            palette.accent_solid,
            palette.on_accent,
        );
    } else if tile.trying {
        badge("Trying on", palette.elevated, palette.text_strong);
    }
    let colour = if tile.worn {
        palette.text_strong
    } else {
        palette.text_secondary
    };
    let label = widgets::elided(
        ui,
        tile.label,
        egui::FontId::proportional(12.0),
        colour,
        width - 4.0,
        1,
    );
    painter.galley(
        egui::pos2(rect.left() + 2.0, image.bottom() + 4.0),
        label,
        colour,
    );
    let mut touched = Touched {
        hovered,
        clicked: false,
        double_clicked: response.double_clicked(),
        star: false,
    };
    if let Some(favourite) = tile.favourite {
        let star =
            egui::Rect::from_min_size(image.right_top() + vec2(-24.0, 2.0), vec2(22.0, 22.0));
        let star_response = ui
            .interact(
                star,
                ui.id().with(("item favourite", &tile.picture)),
                Sense::click(),
            )
            .on_hover_text(if favourite {
                "A favourite (click to take it off)"
            } else {
                "Add to favourites"
            });
        if favourite || star_response.hovered() || hovered {
            painter.text(
                star.center(),
                egui::Align2::CENTER_CENTER,
                if favourite {
                    theme::filled_icons::STAR
                } else {
                    icons::STAR
                },
                egui::FontId::proportional(15.0),
                if favourite {
                    palette.accent
                } else {
                    palette.text_weak
                },
            );
        }
        touched.star = star_response.clicked();
        touched.hovered |= star_response.hovered();
    }
    touched.clicked = response.on_hover_text(&tile.tip).clicked();
    touched
}

/// The empty page: nothing of this kind fits the body at all.
fn nothing_offered(ui: &mut egui::Ui, shelf: Shelf) {
    let (category, title, sentence) = match shelf {
        Shelf::Hair => (
            super::super::Category::Hair,
            "No hairstyles yet",
            "Nothing in the library fits this body yet. Hairstyles for it are being made.",
        ),
        Shelf::Clothing => (
            super::super::Category::Clothing,
            "No clothes yet",
            "Nothing in the library fits this body yet beyond what the body type wears.",
        ),
    };
    widgets::empty_state(ui, nav::icon(category), title, sentence, None);
}

/// `text` with a capital first letter ("short" reads "Short").
fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
