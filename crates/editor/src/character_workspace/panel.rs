//! The contextual panel on the right (brief §3.2): only what the selected
//! category, or the feature picked in it, can change. Its header holds the
//! detail level (Quick, Detailed, Advanced: brief §3.3) and Symmetry. Every
//! page returns [`Intent`]s; nothing here changes the workspace.

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

/// What a category is for, under its title and on its tooltip.
pub(super) fn about(category: Category) -> &'static str {
    match category {
        Category::Body => "Pick the body type, then shape the body. Changes show as you drag.",
        Category::Face => "Shape the face, one feature at a time.",
        Category::Hair => "Pick a hairstyle: point at one to see it on the character.",
        Category::Clothing => {
            "Dress the character one piece at a time: point at a piece to see it on."
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
    // What a side's reset puts back (the asymmetry tools), for the pages
    // with sliders.
    let defaults = if has_sliders(category) {
        axcenity_gameplay::description::default_description(library, &base.id)
    } else {
        workspace.description().clone()
    };
    egui::ScrollArea::vertical()
        .id_salt("character workspace panel")
        .auto_shrink(false)
        .show(ui, |ui| {
            let description = workspace.description();
            intents.extend(match category {
                Category::Body => {
                    let mut asked = Vec::new();
                    if selection.feature.is_none() {
                        asked.extend(base_picker(ui, workspace, library));
                        asked.extend(preset_chips(ui, workspace, library, PresetKind::Body));
                    }
                    asked.extend(parameters::show(
                        ui,
                        base,
                        description,
                        &defaults,
                        selection,
                        category,
                        symmetry,
                        &workspace.locks,
                    ));
                    asked
                }
                Category::Face => {
                    let mut asked = Vec::new();
                    if selection.feature.is_none() {
                        asked.extend(preset_chips(ui, workspace, library, PresetKind::Face));
                    }
                    asked.extend(parameters::show(
                        ui,
                        base,
                        description,
                        &defaults,
                        selection,
                        category,
                        symmetry,
                        &workspace.locks,
                    ));
                    asked
                }
                Category::SkinAndMakeup => {
                    let mut asked = materials::skin(ui, workspace, base);
                    if base
                        .groups
                        .iter()
                        .any(|group| group.category == axcenity_avatar::Category::Skin)
                    {
                        asked.extend(parameters::show(
                            ui,
                            base,
                            description,
                            &defaults,
                            selection,
                            category,
                            symmetry,
                            &workspace.locks,
                        ));
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
    intents
}

/// A kind's presets as "Start from" chips at the top of the Body or Face
/// page (ux patterns P1): a good start first. A body preset tagged for a
/// body type shows only on that body type. "Custom" is lit when none of
/// them is what the character is.
fn preset_chips(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
    kind: PresetKind,
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
        .collect();
    if listed.is_empty() {
        return intents;
    }
    widgets::section_label(ui, "Start from");
    let mut any = false;
    ui.horizontal_wrapped(|ui| {
        for listed in &listed {
            let matched = matches_preset(description, listed.preset, library);
            any |= matched;
            if widgets::chip(ui, listed.label(), matched)
                .on_hover_text(format!(
                    "{} Sets the {} only; you can undo it.",
                    listed.info.about,
                    kind.name()
                ))
                .clicked()
            {
                intents.push(Intent::ApplyPreset(listed.id.to_owned()));
            }
        }
        // Not a button: it says the character is its own.
        widgets::chip(ui, "Custom", !any).on_hover_text(format!(
            "Lit when the {} is none of these presets",
            kind.name()
        ));
    });
    ui.add_space(8.0);
    intents
}

/// The body families (plan section 0, decision 7): realistic first, the
/// default, and stylized. The one the character is on is lit; another
/// starts again on that body as one undo step.
fn base_picker(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    library: &Library,
) -> Vec<Intent> {
    let mut intents = Vec::new();
    let current = &workspace.description().base;
    let families = presets::families(library);
    widgets::section_label(ui, "Family");
    ui.horizontal_wrapped(|ui| {
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
        let columns = 3;
        let gap = 6.0;
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
            PresetKind::Other => false,
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
