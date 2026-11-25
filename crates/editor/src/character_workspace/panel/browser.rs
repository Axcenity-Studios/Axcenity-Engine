//! The Hair and Clothing pages as browsers (R6 CO25, R7 CO26; brief §7.1,
//! §8.2): a search field with a "Filters" menu beside it (the kind,
//! "Recolourable", "Fits this body" and favourites; the button says how
//! many are changed), who the items are made for as a row of its own
//! (the character's body type's, the other's, or all), the page's slots
//! as one row of categories, the worn items folded to a line each (`worn.rs`), a row
//! of the items worn last and picture tiles of the character wearing each
//! item, as many across as fit the panel. Resting the pointer on a tile
//! (`browser::REST`) shows it on the character without wearing it, once
//! its models are loaded in the background; a click wears it as one undo
//! step, and a click on the worn tile takes it off. What is listed is `browser::list`;
//! this only draws it and returns intents.

use axcenity_avatar::{BaseManifest, Library};
use eframe::egui::{self, Sense, vec2};

use super::super::browser::{self, Browse, Browser, Cut, Entry, Filters, Listing, Pick, Shelf};
use super::super::intent::Intent;
use super::super::{CharacterWorkspace, nav};
use super::{wardrobe, worn};
use crate::theme::{self, icons};
use crate::widgets;

/// The room between tiles.
pub(super) const GAP: f32 = 6.0;

/// What a tile shows and was asked.
struct Tile<'a> {
    pick: Pick,
    label: &'a str,
    /// The picture's key in the workspace (an item ID, or `""` + a slot).
    picture: String,
    draft: bool,
    /// The item is worn (the "nothing" tile: nothing is worn there).
    worn: bool,
    favourite: Option<bool>,
    /// The body type it is made for, when that isn't the character's.
    made_for: Option<&'a str>,
    /// The tile previewed on the character (the pointer is on it).
    trying: bool,
    /// What a click on it wears (`None`: nothing in its slot), when a
    /// click changes anything.
    click: Option<Option<String>>,
    tip: String,
}

/// What a tile's pointer did.
#[derive(Default)]
struct Touched {
    hovered: bool,
    clicked: bool,
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
    let mut intents = Vec::new();
    let description = workspace.description();
    let state = &workspace.browser;
    let filters = state.filters(shelf).clone();
    let listing = browser::list(
        library,
        description,
        shelf,
        &filters,
        state.show_drafts,
        &workspace.item_favourites,
        &workspace.recent_items,
    );
    let set = |change: &dyn Fn(&mut Filters)| {
        let mut changed = filters.clone();
        change(&mut changed);
        (changed != filters).then_some(Intent::Browse(Browse::Filters(shelf, changed)))
    };

    if listing.slots.is_empty() {
        nothing_offered(ui, shelf);
        return intents;
    }
    // The Clothing page shows the library's clothes, or the online
    // wardrobe's to download (`wardrobe.rs`): one or the other, so the
    // wardrobe is never under a long list of tiles.
    let online = shelf == Shelf::Clothing && state.online;
    if shelf == Shelf::Clothing {
        let mut shown = online;
        let options = [
            (false, Some(icons::T_SHIRT), "Your library"),
            (true, Some(icons::CLOUD_ARROW_DOWN), "Online wardrobe"),
        ];
        let tips = [
            "The clothes in your library",
            "More clothes to download, from the Axcenity Wardrobe",
        ];
        if widgets::segmented_tips(ui, &mut shown, &options, &tips).0 {
            intents.push(Intent::Browse(Browse::Online(shown)));
        }
        ui.add_space(2.0);
    }
    if !online && listing.offered == 0 && listing.entries.is_empty() {
        nothing_offered(ui, shelf);
        return intents;
    }

    // Search, and the filters in a menu beside it.
    let mut search = filters.search.clone();
    let hint = match shelf {
        Shelf::Hair => "Search hairstyles",
        Shelf::Clothing if online => "Search the wardrobe",
        Shelf::Clothing => "Search clothes",
    };
    // One row: the button at its end, the field in what it leaves.
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), theme::size::BUTTON_HEIGHT),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            intents.extend(filters_menu(ui, shelf, &filters, &listing, online));
            let width = ui.available_width();
            widgets::search_field_sized(ui, &mut search, hint, width, theme::size::BUTTON_HEIGHT);
        },
    );
    if search != filters.search {
        intents.extend(set(&|f| f.search = search.clone()));
    }

    // Who the items are made for: this body type's (with the unisex
    // ones), the other's, or everything. Lit on the character's own until
    // another is picked, so it follows a change of body type.
    let labels = browser::cut_labels(base);
    let feminine = browser::feminine_body(description, base);
    if (listing.cuts || online) && feminine.is_some() {
        let mut shown = filters.cut.shown(feminine);
        let options = [
            (Cut::Masculine, Some(icons::GENDER_MALE), labels.0.as_str()),
            (Cut::Feminine, Some(icons::GENDER_FEMALE), labels.1.as_str()),
            (Cut::All, None, "All"),
        ];
        let what = match shelf {
            Shelf::Hair => "Hairstyles",
            Shelf::Clothing => "Clothes",
        };
        let tips = [
            format!(
                "{what} made for the {} body, and unisex ones",
                labels.0.to_lowercase()
            ),
            format!(
                "{what} made for the {} body, and unisex ones",
                labels.1.to_lowercase()
            ),
            format!(
                "Every one, whoever it is made for ({} last)",
                match feminine {
                    Some(true) => format!("the {} ones", labels.0.to_lowercase()),
                    _ => format!("the {} ones", labels.1.to_lowercase()),
                }
            ),
        ];
        let tips: Vec<&str> = tips.iter().map(String::as_str).collect();
        if widgets::segmented_tips(ui, &mut shown, &options, &tips).0 {
            let cut = Cut::picked(shown, feminine);
            intents.extend(set(&|f| f.cut = cut));
        }
        ui.add_space(2.0);
    }

    // The page's slots, as categories.
    if listing.slots.len() > 1 {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
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
    }

    if online {
        // The search, the category and the cut above narrow the wardrobe
        // too.
        intents.extend(wardrobe::section(ui, workspace, base, &filters));
        return intents;
    }

    // "Show drafts" is a choice only where there are finished items to
    // show without them: a page of drafts alone lists them.
    if listing.finished > 0 && listing.drafts > 0 {
        let mut drafts = state.show_drafts;
        let label = if listing.hidden_drafts > 0 {
            format!("Show drafts ({} hidden)", listing.hidden_drafts)
        } else {
            "Show drafts".to_owned()
        };
        if ui
            .checkbox(&mut drafts, label)
            .on_hover_text(
                "Drafts haven't passed the quality checks yet. They're marked \"Draft\" and \
                 never used by presets or Randomize.",
            )
            .changed()
        {
            intents.push(Intent::Browse(Browse::ShowDrafts(drafts)));
        }
    }

    // The worn items, a line each above the tiles so they're seen without
    // scrolling: its name, the way to take it off and, unfolded, its own
    // controls (colours, variants, parameters).
    let worn_here: Vec<_> = description
        .items
        .iter()
        .filter_map(|worn| Some((library.item(&worn.id)?, worn)))
        .filter(|(item, _)| listing.slots.iter().any(|slot| slot.id == item.slot))
        .filter(|(item, _)| filters.slot.as_ref().is_none_or(|slot| *slot == item.slot))
        .collect();
    if !worn_here.is_empty() {
        ui.add_space(2.0);
        widgets::section_label(ui, "Wearing");
        for (item, worn) in worn_here {
            let slot = listing
                .slots
                .iter()
                .find(|slot| slot.id == item.slot)
                .map_or(item.slot.as_str(), |slot| slot.label.as_str());
            intents.extend(worn::controls(
                ui,
                item,
                worn,
                slot,
                workspace.selection.level,
                state.unfolded.contains(&item.id),
            ));
        }
        ui.add_space(4.0);
    }

    let mut hovered: Option<Pick> = None;
    if !listing.recent.is_empty() {
        widgets::section_label(ui, "Recently used");
        let tiles: Vec<Tile> = listing
            .recent
            .iter()
            .map(|entry| item_tile(entry, state, &labels))
            .collect();
        intents.extend(grid(ui, workspace, "recent", &tiles, &mut hovered));
        ui.add_space(4.0);
    }

    if listing.entries.is_empty() {
        let sentence = if filters.narrowed() {
            "Nothing matches. Clear the search or a filter."
        } else if listing.hidden_drafts > 0 {
            "What fits here is still a draft. Turn on Show drafts to see it."
        } else {
            "Nothing in the library fits this body here yet."
        };
        let clear = filters
            .narrowed()
            .then_some((icons::X, "Clear the search and filters"));
        if widgets::empty_state(
            ui,
            icons::MAGNIFYING_GLASS,
            "Nothing to show",
            sentence,
            clear,
        ) {
            intents.extend(set(&|f| *f = Filters::default()));
        }
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
                draft: false,
                made_for: None,
                worn: !worn_here,
                favourite: None,
                // Nothing to take off where nothing is worn.
                click: worn_here.then_some(None),
                tip: if worn_here {
                    format!("Click to wear nothing here ({none})")
                } else {
                    format!("Nothing is worn here ({none})")
                },
            });
        }
        tiles.extend(entries.iter().map(|entry| item_tile(entry, state, &labels)));
        intents.extend(grid(ui, workspace, &slot.id, &tiles, &mut hovered));
        ui.add_space(4.0);
    }
    // The tile under the pointer: said when it changes, so what it would
    // show is loaded in the background while the pointer rests there.
    if hovered != state.over {
        intents.push(Intent::Browse(Browse::Over(hovered.clone())));
    }
    // It is previewed once the pointer has rested on it, without the page
    // scrolling, and its models are in: a tile crossed on the way to
    // another, or scrolled past, is never shown on the character.
    let (now, scrolled, moved) = ui.input(|input| {
        (
            input.time,
            input.smooth_scroll_delta != egui::Vec2::ZERO
                || input
                    .raw
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::MouseWheel { .. })),
            input.pointer.delta() != egui::Vec2::ZERO,
        )
    });
    let rest = egui::Id::new("browser tile the pointer rests on");
    let since = ui.data_mut(|data| {
        let kept: Option<(Option<Pick>, f64)> = data.get_temp(rest);
        match kept {
            Some((tile, since)) if tile == hovered && !scrolled => since,
            _ => {
                data.insert_temp(rest, (hovered.clone(), now));
                now
            }
        }
    });
    let shown = match &hovered {
        // The tile previewed stays so while the pointer is on it.
        Some(_) if hovered == state.hovered => hovered.clone(),
        Some(_) if now - since >= browser::REST && hovered == state.over && state.ready => {
            hovered.clone()
        }
        Some(_) => {
            ui.ctx().request_repaint();
            None
        }
        None => None,
    };
    // After Esc or a click the tiles can shift under a still pointer (the
    // worn items' lines change), so a muted tile waits for the pointer to
    // move.
    if state.hover_changed(&hovered, &shown, moved) {
        intents.push(Intent::Browse(Browse::Hover(shown)));
    }

    intents
}

/// How many of a page's filters (the "Filters" menu's: not the search,
/// the category or who the items are made for, which have rows of their
/// own) differ from what the page opens with.
fn filters_changed(filters: &Filters) -> usize {
    let default = Filters::default();
    usize::from(filters.kind.is_some())
        + usize::from(filters.recolourable != default.recolourable)
        + usize::from(filters.fits != default.fits)
        + usize::from(filters.favourites != default.favourites)
}

/// The "Filters" menu beside the search field: "Recolourable", "Fits this
/// body", favourites and the kind (one of the listed items' tags). Its
/// button says how many are changed; the menu stays open while they're
/// picked. The online wardrobe has none of them, so no menu.
fn filters_menu(
    ui: &mut egui::Ui,
    shelf: Shelf,
    filters: &Filters,
    listing: &Listing,
    online: bool,
) -> Vec<Intent> {
    if online {
        return Vec::new();
    }
    use egui::containers::menu;
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let mut changed = filters.clone();
    let count = filters_changed(filters);
    let label = if count == 0 {
        "Filters".to_owned()
    } else {
        format!("Filters · {count}")
    };
    let button = widgets::button(
        widgets::Role::Secondary,
        Some(icons::FUNNEL),
        &label,
        &palette,
    );
    let config =
        menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    let (response, _) = menu::MenuButton::from_button(button)
        .config(config)
        .ui(ui, |ui| {
            ui.set_min_width(190.0);
            if !online {
                ui.checkbox(&mut changed.fits, "Fits this body")
                    .on_hover_text("Hide items whose maker says they don't fit this body's shape");
                ui.checkbox(&mut changed.recolourable, "Recolourable")
                    .on_hover_text("Only items whose colours you can change");
                ui.checkbox(
                    &mut changed.favourites,
                    format!("{} Favourites", theme::filled_icons::STAR),
                )
                .on_hover_text("Only your favourites");
            }
            if !online && !listing.kinds.is_empty() {
                ui.add_space(4.0);
                widgets::section_label(ui, "Kind");
                egui::ScrollArea::vertical()
                    .id_salt(("browser kinds", shelf as u8))
                    .max_height(180.0)
                    .show(ui, |ui| {
                        ui.radio_value(&mut changed.kind, None, "Any kind")
                            .on_hover_text("Every kind (the items' tags)");
                        for kind in &listing.kinds {
                            ui.radio_value(
                                &mut changed.kind,
                                Some(kind.clone()),
                                capitalised(kind),
                            );
                        }
                    });
            }
            if count > 0 {
                ui.add_space(4.0);
                if ui
                    .button("Reset filters")
                    .on_hover_text("Back to what the page opens with")
                    .clicked()
                {
                    let default = Filters::default();
                    changed = Filters {
                        search: changed.search.clone(),
                        slot: changed.slot.clone(),
                        cut: changed.cut,
                        ..default
                    };
                }
            }
        });
    response.on_hover_text(if count == 0 {
        "Narrow the list: what fits, what can be recoloured, favourites, kinds".to_owned()
    } else if count == 1 {
        "1 filter is changed from what the page opens with".to_owned()
    } else {
        format!("{count} filters are changed from what the page opens with")
    });
    if changed != *filters {
        intents.push(Intent::Browse(Browse::Filters(shelf, changed)));
    }
    intents
}

/// A tile for `entry`; `labels` are the body types' names
/// (`browser::cut_labels`).
fn item_tile<'a>(entry: &'a Entry, state: &Browser, labels: &'a (String, String)) -> Tile<'a> {
    let made_for = entry.other_cut.map(|feminine| {
        if feminine {
            labels.1.as_str()
        } else {
            labels.0.as_str()
        }
    });
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
    match (browser::cut_for(&entry.item.tags), made_for) {
        (_, Some(body)) => tip.push_str(&format!(
            "\nMade for the {} body, not this character's.",
            body.to_lowercase()
        )),
        (Some(feminine), None) => tip.push_str(&format!(
            "\nMade for the {} body.",
            if feminine { &labels.1 } else { &labels.0 }.to_lowercase()
        )),
        (None, None) if entry.item.tags.iter().any(|tag| tag == "unisex") => {
            tip.push_str("\nUnisex.");
        }
        (None, None) => {}
    }
    if entry.draft {
        tip.push_str("\nA draft: it hasn't passed the quality checks yet.");
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
        "\nWorn. Click to take it off."
    } else {
        "\nClick to wear it."
    });
    Tile {
        trying: state.trying() == Some(&pick),
        pick,
        label: &entry.item.label,
        picture: entry.item.id.clone(),
        draft: entry.draft,
        made_for,
        worn: entry.worn,
        favourite: Some(entry.favourite),
        // A click wears it; a click on what is worn takes it off (one
        // undo step either way).
        click: Some((!entry.worn).then(|| entry.item.id.clone())),
        tip,
    }
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
    let columns = super::tile_columns(ui.available_width(), GAP);
    let width = ((ui.available_width() - GAP * (columns as f32 - 1.0)) / columns as f32).floor();
    for row in tiles.chunks(columns) {
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
                } else if touched.clicked
                    && let Some(item) = &tile.click
                {
                    // The clicked tile is under the pointer still: it
                    // isn't previewed again until the pointer moves off.
                    intents.push(Intent::Browse(Browse::Clicked(tile.pick.clone())));
                    intents.push(Intent::Wear {
                        slot: tile.pick.slot.clone(),
                        item: item.clone(),
                    });
                }
            }
        });
        ui.add_space(GAP);
    }
    intents
}

/// One tile: the character wearing it (a placeholder until drawn), its
/// label, "Draft", an accent border and "Worn" when worn, a star for
/// favourites.
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
            // The picture fills the tile, whatever shape it was drawn in.
            let uv = wardrobe::cropped(texture.size_vec2(), image.size());
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
    // Badges along the picture's foot: Draft, Worn.
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
    if tile.draft {
        badge("Draft", palette.elevated, palette.warning);
    }
    // Made for the other body type: said where both are listed.
    if let Some(body) = tile.made_for {
        badge(body, palette.elevated, palette.text_secondary);
    }
    // (The "nothing" tile of an empty slot is lit, not "worn".)
    if tile.worn && tile.pick.item.is_some() {
        badge(
            &format!("{} Worn", icons::CHECK),
            palette.accent_solid,
            palette.on_accent,
        );
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
