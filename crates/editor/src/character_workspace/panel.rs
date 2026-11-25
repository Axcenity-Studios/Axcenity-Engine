//! The contextual panel on the right (brief §3.2): only what the selected
//! category, or the feature picked in it, can change. Its header is one
//! line: the title, the detail level (Quick, Detailed, Advanced: brief
//! §3.3) and Symmetry. Under it, on the pages that are lists of sliders
//! (Body, Face, Skin and Makeup), a "Search controls" field finds a slider
//! by its name in every feature and at every level. Every page returns
//! [`Intent`]s; nothing here changes the workspace.

mod browser;
mod expressions;
mod materials;
mod parameters;
mod row;
#[cfg(test)]
mod tests;
mod wardrobe;
mod worn;

pub(super) use browser::none_picture;

#[cfg(test)]
pub(super) use parameters::{Shown, hidden_changes, rows, side_choices};

use axcenity_avatar::{BaseManifest, Library, PresetKind, matches_preset};
use eframe::egui::{self, Sense, vec2};

use super::intent::Intent;
use super::motion::Motion;
use super::presets::{self, Listed};
use super::selection::Level;
use super::{Category, CharacterWorkspace, chosen_slots, items_for, worn_in};
use crate::theme::{self, icons};
use crate::widgets;

/// What a category is for: its title's tooltip, and its tooltip in the
/// navigation.
pub(super) fn about(category: Category) -> &'static str {
    match category {
        Category::Body => "Pick the body type, then shape the body. Changes show as you drag.",
        Category::Face => "Shape the face, one feature at a time.",
        Category::Hair => {
            "Pick a hairstyle: point at one to see it on the character, click to wear it."
        }
        Category::Clothing => {
            "Dress the character one piece at a time: point at a piece to see it on, click to \
             wear it, click it again to take it off."
        }
        Category::SkinAndMakeup => "The skin's colour and surface.",
        Category::Accessories => "Glasses, jewellery, hats and bags.",
        Category::Expressions => {
            "See the face and body move. A preview only: never saved with the character."
        }
        Category::Presets => {
            "Start from a ready-made character, body or face; save your own; star favourites."
        }
    }
}

/// Whether `category`'s page has sliders (and so a level and Symmetry;
/// the browsers' sliders are the worn items' own).
fn has_sliders(category: Category) -> bool {
    matches!(
        category,
        Category::Body
            | Category::Face
            | Category::SkinAndMakeup
            | Category::Hair
            | Category::Clothing
    )
}

/// Whether `category`'s page has the "Search controls" field: the pages
/// that are lists of sliders.
fn has_search(category: Category) -> bool {
    matches!(
        category,
        Category::Body | Category::Face | Category::SkinAndMakeup
    )
}

/// Whether every word of `search` is in one of `texts` (case aside). An
/// empty search matches everything.
pub(super) fn has_words(texts: &[&str], search: &str) -> bool {
    let haystack = texts.join(" ").to_lowercase();
    search
        .split_whitespace()
        .all(|word| haystack.contains(&word.to_lowercase()))
}

/// How many picture tiles go across `width`: as many as fit at about 120
/// points each, two at least.
pub(super) fn tile_columns(width: f32, gap: f32) -> usize {
    (((width + gap) / (TILE + gap)).round() as usize).max(2)
}

/// How wide a picture tile is about (the Hair and Clothing browsers', the
/// online wardrobe's, the presets').
const TILE: f32 = 120.0;

/// The panel's header, on one line: the page's title (its tooltip says
/// what the page is for), then from the right the detail level (a menu
/// that says what each level shows), Symmetry and, with a feature picked,
/// the way back to the whole category.
fn header(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    base: Option<&BaseManifest>,
    title: &str,
    feature: bool,
) -> Vec<Intent> {
    use egui::containers::menu;
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let selection = &workspace.selection;
    let category = selection.category;
    let (row, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), theme::size::BUTTON_HEIGHT),
        Sense::hover(),
    );
    let font = egui::FontId::new(18.0, theme::display_family());
    let natural = ui
        .painter()
        .layout_no_wrap(title.to_owned(), font.clone(), palette.text_strong)
        .size()
        .x;
    // The controls, from the right; the title has what they leave.
    let mut right = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(row)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    {
        let ui = &mut right;
        ui.spacing_mut().item_spacing.x = 6.0;
        if has_sliders(category) {
            let level = selection.level;
            let button = egui::Button::new(
                egui::RichText::new(format!("{}  {}", level.label(), icons::CARET_DOWN))
                    .color(palette.text_secondary),
            )
            .corner_radius(theme::radius::CONTROL);
            let (response, _) = menu::MenuButton::from_button(button).ui(ui, |ui| {
                ui.set_min_width(230.0);
                for option in Level::ALL {
                    if ui
                        .selectable_label(option == level, option.label())
                        .clicked()
                    {
                        if option != level {
                            intents.push(Intent::PickLevel(option));
                        }
                        ui.close();
                    }
                    ui.label(
                        egui::RichText::new(option.about())
                            .small()
                            .color(palette.text_weak),
                    );
                    ui.add_space(2.0);
                }
            });
            response.on_hover_text(format!(
                "How much is shown. {}: {}",
                level.label(),
                level.about().to_lowercase()
            ));
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
                let tip = "Symmetry: move a left and right pair together";
                // Its word where the title leaves room for it.
                let changed = if natural + 250.0 <= row.width() {
                    ui.checkbox(&mut symmetry, "Symmetry")
                        .on_hover_text(tip)
                        .changed()
                } else {
                    let clicked =
                        widgets::icon_button(ui, icons::ARROWS_LEFT_RIGHT, tip, symmetry).clicked();
                    symmetry ^= clicked;
                    clicked
                };
                if changed {
                    intents.push(Intent::SetSymmetry(symmetry));
                }
            }
        }
    }
    // The way back to the whole category sits right after the title.
    let close = if feature {
        ui.spacing().interact_size.y + 4.0
    } else {
        0.0
    };
    let room = (right.min_rect().left() - row.left() - 8.0 - close).max(40.0);
    let galley = widgets::elided(ui, title, font, palette.text_strong, room, 1);
    let at = egui::pos2(row.left(), row.center().y - galley.size().y / 2.0);
    let drawn = egui::Rect::from_min_size(at, galley.size());
    ui.painter().galley(at, galley, palette.text_strong);
    ui.interact(drawn, ui.id().with("panel title"), Sense::hover())
        .on_hover_text(format!("{title}\n{}", about(category)));
    if feature {
        let after = egui::Rect::from_min_max(
            egui::pos2(drawn.right() + 4.0, row.top()),
            egui::pos2(drawn.right() + 4.0 + close, row.bottom()),
        );
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(after)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        if widgets::icon_button(&mut ui, icons::X, "Show the whole category", false).clicked() {
            intents.push(Intent::PickFeature(None));
        }
    }
    intents
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
    // A manifest feature's label (the waist, the nose), or a group's.
    let feature = selection.feature.as_deref().and_then(|feature| {
        let base = base?;
        base.feature(feature)
            .map(|feature| feature.label.clone())
            .or_else(|| {
                base.groups
                    .iter()
                    .find(|group| group.id == feature)
                    .map(|group| group.label.clone())
            })
    });
    let title = match &feature {
        Some(feature) => format!("{} › {feature}", super::nav::label(category)),
        None => super::nav::label(category).to_owned(),
    };
    intents.extend(header(ui, workspace, base, &title, feature.is_some()));
    let Some(base) = base else {
        ui.label("This body isn't in the library.");
        return intents;
    };
    // The page's sliders by name (label, ID, what they do): editor state.
    let search = if has_search(category) {
        let mut text = workspace.control_search.clone();
        let width = ui.available_width();
        widgets::search_field(ui, &mut text, "Search controls", width).on_hover_text(
            "Find a control of this page by its name or what it does, in every feature and at \
             every level",
        );
        if text != workspace.control_search {
            intents.push(Intent::SearchControls(text));
        }
        workspace.control_search.trim()
    } else {
        ""
    };
    let searching = !search.is_empty();
    ui.add_space(2.0);
    // Quick always moves both sides together.
    let symmetry = workspace.symmetry || selection.level == Level::Quick;
    // What a side's reset puts back (the asymmetry tools), for the pages
    // with sliders.
    let defaults = if has_sliders(category) {
        axcenity_gameplay::description::default_description(library, &base.id)
    } else {
        workspace.description().clone()
    };
    // What the body type wears (the underwear) is said in one line under
    // a browser's tiles, always in view.
    let footer = super::browser::Shelf::of(category)
        .map(|shelf| super::browser::body_type_wears(library, workspace.description(), shelf))
        .filter(|wears| !wears.is_empty());
    let footer_height = if footer.is_some() { 22.0 } else { 0.0 };
    egui::ScrollArea::vertical()
        .id_salt("character workspace panel")
        .auto_shrink(false)
        .max_height((ui.available_height() - footer_height).max(40.0))
        .show(ui, |ui| {
            let description = workspace.description();
            // The page's sliders: every match while searching.
            let parameters = |ui: &mut egui::Ui| {
                parameters::show(
                    ui,
                    base,
                    description,
                    &defaults,
                    selection,
                    category,
                    symmetry,
                    &workspace.locks,
                    search,
                )
            };
            let nothing_found = |ui: &mut egui::Ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("No control matches \"{search}\"."))
                        .color(palette.text_weak),
                );
            };
            intents.extend(match category {
                Category::Body | Category::Face => {
                    let mut asked = Vec::new();
                    if selection.feature.is_none() && !searching {
                        if category == Category::Body {
                            asked.extend(base_picker(ui, workspace, library));
                        }
                        let kind = if category == Category::Body {
                            PresetKind::Body
                        } else {
                            PresetKind::Face
                        };
                        asked.extend(preset_chips(ui, workspace, library, kind, None));
                    } else if let (Some(feature), false) = (&selection.feature, searching) {
                        // The picked feature's own shapes (a nose, a face
                        // shape): the rest of the face is left alone.
                        asked.extend(preset_chips(
                            ui,
                            workspace,
                            library,
                            PresetKind::Feature,
                            Some(feature),
                        ));
                    }
                    let (sliders, found) = parameters(ui);
                    asked.extend(sliders);
                    if searching && !found {
                        nothing_found(ui);
                    }
                    asked
                }
                Category::SkinAndMakeup => {
                    let (mut asked, mut found) = materials::skin(ui, workspace, base, search);
                    if base
                        .groups
                        .iter()
                        .any(|group| group.category == axcenity_avatar::Category::Skin)
                    {
                        let (sliders, more) = parameters(ui);
                        asked.extend(sliders);
                        found |= more;
                    }
                    if searching && !found {
                        nothing_found(ui);
                    }
                    asked
                }
                Category::Hair => {
                    browser::page(ui, workspace, library, base, super::browser::Shelf::Hair)
                }
                Category::Clothing => browser::page(
                    ui,
                    workspace,
                    library,
                    base,
                    super::browser::Shelf::Clothing,
                ),
                Category::Accessories => slots_page(ui, workspace, library, base, category),
                Category::Expressions => expressions::page(ui, workspace),
                Category::Presets => presets_page(ui, workspace, library),
            });
        });
    if let Some(wears) = footer {
        let text = wears
            .iter()
            .map(|(slot, item)| format!("{slot}: {item}, which comes with the body type."))
            .collect::<Vec<_>>()
            .join(" ");
        ui.add(
            egui::Label::new(egui::RichText::new(&text).small().color(palette.text_muted))
                .truncate(),
        )
        .on_hover_text(format!(
            "{text}\nIt changes with the body type (Body page), and can't be taken off here."
        ));
    }
    intents
}

/// A kind's presets as "Start from" chips at the top of the Body or Face
/// page (ux patterns P1): a good start first. A body preset tagged for a
/// body type shows only on that body type. "Custom" is lit when none of
/// them is what the character is. With `feature`, the feature presets of
/// that feature (`PresetKind::Feature`: the nose shapes on the Nose page,
/// the face shapes on Face shape), as "Shape" chips.
fn preset_chips(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
    kind: PresetKind,
    feature: Option<&String>,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let description = workspace.description();
    let Some(base) = library.base(&description.base) else {
        return intents;
    };
    let listed: Vec<Listed> = workspace
        .presets_of(library, kind)
        .into_iter()
        .filter(|listed| kind != PresetKind::Body || presets::suits(listed, description, base))
        .filter(|listed| kind != PresetKind::Feature || listed.info.feature.as_ref() == feature)
        .collect();
    if listed.is_empty() {
        return intents;
    }
    let mut any = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let what = if kind == PresetKind::Feature {
            "feature"
        } else {
            kind.name()
        };
        widgets::section_label(
            ui,
            if kind == PresetKind::Feature {
                "Shape"
            } else {
                "Start from"
            },
        );
        for listed in &listed {
            let matched = matches_preset(description, listed.preset, library);
            any |= matched;
            if widgets::chip(ui, listed.label(), matched)
                .on_hover_text(format!(
                    "{} Sets this {what} only; you can undo it.",
                    listed.info.about,
                ))
                .clicked()
            {
                intents.push(Intent::ApplyPreset(listed.id.to_owned()));
            }
        }
        // Not a button: it says the character is its own.
        widgets::chip(ui, "Custom", !any)
            .on_hover_text(format!("Lit when the {what} is none of these presets"));
    });
    ui.add_space(2.0);
    intents
}

/// The body families, when the library has more than the engine's one
/// (packages of a person's own, or an older library's): the default
/// first. The one the character is on is lit; another starts again on
/// that body as one undo step. With one family nothing is drawn.
fn base_picker(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let current = &workspace.description().base;
    let families = presets::families(library);
    if families.len() < 2 {
        return intents;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        widgets::section_label(ui, "Family");
        for base in families {
            let on = base.id == *current;
            let response = widgets::chip(ui, &base.label, on);
            if on {
                response.on_hover_text(format!("The character is on the {} body", base.label));
            } else if response
                .on_hover_text(format!(
                    "Start again on the {} body (you can undo it)",
                    base.label.to_lowercase()
                ))
                .clicked()
            {
                intents.push(Intent::SetBase(base.id.clone()));
            }
        }
    });
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

/// The preview's pose and motion as chips (R9): Still, Idle, Walk, Run
/// and the gestures. Preview only, like an expression.
pub(super) fn motion_chips(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let mut intents = Vec::new();
    ui.horizontal_wrapped(|ui| {
        for motion in Motion::all() {
            let on = workspace.motion == motion;
            if widgets::chip(ui, motion.label(), on)
                .on_hover_text(motion.about())
                .clicked()
                && !on
            {
                intents.push(Intent::PreviewMotion(motion));
            }
        }
    });
    intents
}

/// The presets of the character's base model, grouped by kind (whole
/// characters, bodies, faces…), as picture tiles: the favourites first,
/// the ones the character is lit, "Custom" said where none is. Then "Save
/// as preset".
fn presets_page(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let description = workspace.description();
    for kind in presets::KINDS {
        let mut listed = workspace.presets_of(library, kind);
        // Body presets for this body type only (a female build on a male
        // body isn't what its picture shows).
        if kind == PresetKind::Body
            && let Some(base) = library.base(&description.base)
        {
            listed.retain(|listed| presets::suits(listed, description, base));
        }
        if listed.is_empty() {
            continue;
        }
        // Favourites first, otherwise in the library's order (a stable sort).
        listed.sort_by_key(|listed| !workspace.favourites.contains(listed.id));
        let matched = listed
            .iter()
            .any(|listed| matches_preset(description, listed.preset, library));
        ui.horizontal(|ui| {
            widgets::section_label(ui, presets::kind_label(kind));
            if !matched {
                ui.label(
                    egui::RichText::new("Custom")
                        .small()
                        .color(palette.text_weak),
                )
                .on_hover_text("The character is none of these");
            }
        });
        let gap = 6.0;
        let columns = tile_columns(ui.available_width(), gap);
        let width =
            ((ui.available_width() - gap * (columns as f32 - 1.0)) / columns as f32).floor();
        for row in listed.chunks(columns) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for listed in row {
                    let selected = matches_preset(description, listed.preset, library);
                    let favourite = workspace.favourites.contains(listed.id);
                    intents.extend(tile(
                        ui,
                        workspace.preset_texture(listed.id),
                        listed,
                        selected,
                        favourite,
                        width,
                    ));
                }
            });
            ui.add_space(gap);
        }
        ui.add_space(8.0);
    }
    intents.extend(save_as_preset(ui, workspace, library));
    intents
}

/// One preset's tile: its picture (a placeholder until it's drawn), its
/// label and a star for favourites.
fn tile(
    ui: &mut egui::Ui,
    picture: Option<&egui::TextureHandle>,
    listed: &Listed,
    selected: bool,
    favourite: bool,
    width: f32,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let picture_height = (width * 1.25).round();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, picture_height + 22.0), Sense::click());
    let image = egui::Rect::from_min_size(rect.min, vec2(width, picture_height));
    let painter = ui.painter_at(rect);
    painter.rect_filled(image, theme::radius::CARD, palette.inset);
    if let Some(texture) = picture {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), image, uv, egui::Color32::WHITE);
    } else {
        painter.text(
            image.center(),
            egui::Align2::CENTER_CENTER,
            icons::USER,
            egui::FontId::proportional(22.0),
            palette.text_muted,
        );
    }
    if selected || response.hovered() {
        painter.rect_stroke(
            image,
            theme::radius::CARD,
            egui::Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    palette.accent
                } else {
                    palette.text_muted
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(
        egui::pos2(rect.left() + 2.0, image.bottom() + 4.0),
        egui::Align2::LEFT_TOP,
        listed.label(),
        egui::FontId::proportional(12.0),
        if selected {
            palette.text_strong
        } else {
            palette.text_secondary
        },
    );
    let star = egui::Rect::from_min_size(image.right_top() + vec2(-24.0, 2.0), vec2(22.0, 22.0));
    let star_response = ui
        .interact(star, ui.id().with(("favourite", listed.id)), Sense::click())
        .on_hover_text(if favourite {
            "A favourite: listed first (click to take it off)"
        } else {
            "Add to favourites"
        });
    if favourite || star_response.hovered() || response.hovered() {
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
    if star_response.clicked() {
        intents.push(Intent::ToggleFavourite(listed.id.to_owned()));
    } else if response
        .on_hover_text(format!(
            "{}{}\nKeeps the name; you can undo it.",
            listed.info.about,
            if listed.own { " (your own)" } else { "" }
        ))
        .clicked()
    {
        intents.push(Intent::ApplyPreset(listed.id.to_owned()));
    }
    intents
}

/// The kinds a person can save for `base`: whole character, body and face,
/// and hair, outfit or skin where the body has them.
fn savable_kinds(base: &BaseManifest) -> Vec<PresetKind> {
    use axcenity_avatar::Category as Data;
    presets::KINDS
        .into_iter()
        .filter(|kind| match kind {
            PresetKind::Character | PresetKind::Body | PresetKind::Face => true,
            PresetKind::Skin => !base.materials.is_empty(),
            PresetKind::Hair => base.slots.iter().any(|slot| slot.category == Data::Hair),
            PresetKind::Outfit => base
                .slots
                .iter()
                .any(|slot| matches!(slot.category, Data::Clothing | Data::Accessories)),
            PresetKind::Feature | PresetKind::Other => false,
        })
        .collect()
}

/// "Save as preset": a name and a kind; the preset goes into the
/// workspace's `presets/` folder.
fn save_as_preset(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let Some(base) = library.base(&workspace.description().base) else {
        return intents;
    };
    widgets::section_label(ui, "Save as preset");
    let id = egui::Id::new("character studio save preset");
    let (mut name, mut kind) = ui
        .data(|data| data.get_temp::<(String, PresetKind)>(id))
        .unwrap_or_else(|| (String::new(), PresetKind::Character));
    let kinds = savable_kinds(base);
    let options: Vec<(PresetKind, Option<&str>, &str)> = kinds
        .iter()
        .map(|kind| (*kind, None, presets::kind_name(*kind)))
        .collect();
    ui.horizontal_wrapped(|ui| {
        widgets::segmented(ui, &mut kind, &options);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut name)
                .desired_width(ui.available_width() - 90.0)
                .hint_text("Preset name"),
        );
        if ui
            .add_enabled(
                !name.trim().is_empty(),
                widgets::button(
                    widgets::Role::Secondary,
                    Some(icons::FLOPPY_DISK),
                    "Save",
                    &palette,
                ),
            )
            .on_hover_text(format!(
                "Save the {} as a preset of your own",
                presets::kind_name(kind).to_lowercase()
            ))
            .clicked()
        {
            intents.push(Intent::SaveAsPreset {
                kind,
                name: name.trim().to_owned(),
            });
            name.clear();
        }
    });
    ui.label(
        egui::RichText::new(format!(
            "A {} preset keeps only what it's about; your presets are listed with the others.",
            presets::kind_name(kind).to_lowercase()
        ))
        .small()
        .color(palette.text_weak),
    );
    ui.data_mut(|data| data.insert_temp(id, (name, kind)));
    intents
}
