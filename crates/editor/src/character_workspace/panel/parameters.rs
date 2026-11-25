//! A category's parameters by group, as compact sections: a one-line
//! slider row each (a chip row for a choice such as the body type),
//! filtered by the detail level and the picked feature. A category with
//! many features (the face) lists them folded until one is opened or
//! picked. With Symmetry on, a left and right pair shows as one row while
//! its sides agree; a pair whose sides differ shows both, so nothing is
//! flattened unseen (audit D-11). A group with left and right pairs has a
//! "Left and right" menu of the asymmetry tools (R4: mirror a side onto
//! the other, reset a side) for the pairs it shows, and each paired row a
//! ⇄ menu of the same tools for its own pair.
//!
//! While the panel's "Search controls" field has text, the page lists
//! instead every parameter of the category that matches it ([`matches`]:
//! every word typed, in the label, the ID or `about`), whatever feature is
//! picked and at every level: a row of a level above the one shown says
//! which ("Advanced").

use std::collections::BTreeSet;

use axcenity_avatar::{BaseManifest, Description, Parameter};
use eframe::egui;

use super::super::intent::Intent;
use super::super::selection::{
    Level, Selection, parameter_about, parameter_feature, parameter_level,
};
use super::super::sides::{self, SideTool, SidesOf};
use super::super::{Category, symmetric_label, value_of};
use super::row::{self, Asked, Row, Sides};
use crate::theme::{self, icons};
use crate::widgets;

/// A row to show: a parameter, alone or with its mirror.
#[derive(Debug, PartialEq)]
pub(in super::super) struct Shown<'a> {
    pub parameter: &'a Parameter,
    /// It edits its mirror too (Symmetry, sides equal).
    pub mirrored: bool,
}

/// The rows of group `group` shown for `selection` with `symmetry`.
pub(in super::super) fn rows<'a>(
    base: &'a BaseManifest,
    description: &Description,
    group: &str,
    level: Level,
    feature: Option<&str>,
    symmetry: bool,
) -> Vec<Shown<'a>> {
    let value = |id: &str| value_of(description, Some(base), id);
    let index = |id: &str| base.parameters.iter().position(|p| p.id == id);
    base.parameters
        .iter()
        .filter(|p| p.group == group)
        .filter(|p| feature.is_none_or(|feature| parameter_feature(p) == feature))
        .filter(|p| level.shows(parameter_level(p)))
        .filter_map(|p| {
            let mirror = p
                .mirror
                .as_deref()
                .filter(|_| symmetry)
                .filter(|mirror| (value(&p.id) - value(mirror)).abs() < 1.0e-4);
            match mirror {
                // A pair shows once, as the first of the two.
                Some(mirror) if index(&p.id) > index(mirror) => None,
                Some(_) => Some(Shown {
                    parameter: p,
                    mirrored: true,
                }),
                None => Some(Shown {
                    parameter: p,
                    mirrored: false,
                }),
            }
        })
        .collect()
}

/// Whether every word of `search` is in `parameter`'s label, ID or
/// `about` (case aside). An empty search matches everything.
pub(in super::super) fn matches(parameter: &Parameter, search: &str) -> bool {
    super::has_words(
        &[
            &parameter.label,
            &parameter.id,
            parameter.about.as_deref().unwrap_or(""),
        ],
        search,
    )
}

/// The rows of group `group` that match `search`, at every level and for
/// every feature. A pair shown as one row (Symmetry) matches by either
/// side.
pub(in super::super) fn found<'a>(
    base: &'a BaseManifest,
    description: &Description,
    group: &str,
    search: &str,
    symmetry: bool,
) -> Vec<Shown<'a>> {
    rows(base, description, group, Level::Advanced, None, symmetry)
        .into_iter()
        .filter(|row| {
            let mirror = row
                .parameter
                .mirror
                .as_deref()
                .filter(|_| row.mirrored)
                .and_then(|mirror| base.parameter(mirror));
            matches(row.parameter, search) || mirror.is_some_and(|mirror| matches(mirror, search))
        })
        .collect()
}

/// Values of `category` that differ from the default but are hidden at
/// `level`, and the level that shows them all.
pub(in super::super) fn hidden_changes(
    base: &BaseManifest,
    description: &Description,
    category: axcenity_avatar::Category,
    level: Level,
) -> Option<(usize, Level)> {
    let hidden: Vec<Level> = base
        .parameters
        .iter()
        .filter(|p| {
            base.groups
                .iter()
                .any(|group| group.id == p.group && group.category == category)
        })
        .filter(|p| !level.shows(parameter_level(p)))
        .filter(|p| (value_of(description, Some(base), &p.id) - p.default).abs() > 1.0e-4)
        .map(parameter_level)
        .collect();
    let needed = hidden.iter().copied().max()?;
    Some((hidden.len(), needed))
}

/// Each side tool for the pairs `of` names, and whether it would change
/// anything (`defaults` is the body's default character).
pub(in super::super) fn side_choices(
    description: &Description,
    defaults: &Description,
    base: &BaseManifest,
    of: &SidesOf,
) -> Vec<(SideTool, bool)> {
    let pairs = sides::pairs(base, of);
    SideTool::ALL
        .into_iter()
        .map(|tool| {
            (
                tool,
                sides::would_change(description, defaults, base, tool, &pairs),
            )
        })
        .collect()
}

/// The group's "Left and right" menu: the side tools for the pairs of the
/// feature its rows show at this level.
fn sides_row(
    ui: &mut egui::Ui,
    base: &BaseManifest,
    description: &Description,
    defaults: &Description,
    shown: &[Shown],
    level: Level,
) -> Option<Intent> {
    let feature = shown
        .iter()
        .find(|row| row.parameter.mirror.is_some())
        .map(|row| parameter_feature(row.parameter).to_owned())?;
    let of = SidesOf::Feature { feature, level };
    let count = sides::pairs(base, &of).len();
    if count == 0 {
        return None;
    }
    let tools = side_choices(description, defaults, base, &of);
    let tip = format!(
        "Mirror one side onto the other, or reset a side, for the {count} left and right {} \
         shown here (one undo step)",
        if count == 1 { "pair" } else { "pairs" }
    );
    let tool = super::row::sides_menu(ui, &tools, Some("Left and right"), &tip);
    ui.add_space(4.0);
    tool.map(|tool| Intent::Sides { tool, of })
}

/// Draws `category`'s groups and returns what they ask for, and whether
/// any row was drawn. `defaults` is the body's default character (what a
/// side's reset puts back); `search` is the panel's "Search controls" text
/// (empty: the level's and the feature's rows).
#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    ui: &mut egui::Ui,
    base: &BaseManifest,
    description: &Description,
    defaults: &Description,
    selection: &Selection,
    category: Category,
    symmetry: bool,
    locks: &BTreeSet<String>,
    search: &str,
) -> (Vec<Intent>, bool) {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let Some(data) = category.data() else {
        return (intents, false);
    };
    let searching = !search.trim().is_empty();
    let mut any = false;
    // Many features (the face's nine) start folded, so the page is a list
    // of features to open rather than a wall of sliders.
    let groups = base
        .groups
        .iter()
        .filter(|group| group.category == data)
        .count();
    for group in base.groups.iter().filter(|group| group.category == data) {
        let shown = if searching {
            found(base, description, &group.id, search, symmetry)
        } else {
            rows(
                base,
                description,
                &group.id,
                selection.level,
                selection.feature.as_deref(),
                symmetry,
            )
        };
        if shown.is_empty() {
            continue;
        }
        any = true;
        // One column for the ⇄ marks where any row of the group is a
        // pair, so the group's tracks line up.
        let paired = shown.iter().any(|row| row.parameter.mirror.is_some());
        let draw = |ui: &mut egui::Ui, intents: &mut Vec<Intent>| {
            if !searching {
                intents.extend(sides_row(
                    ui,
                    base,
                    description,
                    defaults,
                    &shown,
                    selection.level,
                ));
            }
            for row in &shown {
                // What a search finds above the level shown says where
                // it is.
                let level = parameter_level(row.parameter);
                let note = (searching && !selection.level.shows(level)).then(|| level.label());
                intents.extend(parameter(
                    ui,
                    base,
                    description,
                    defaults,
                    row,
                    locks,
                    paired,
                    note,
                ));
            }
        };
        if selection.feature.is_some() && !searching {
            // One feature: its rows without a header (the panel's title
            // names it).
            draw(ui, &mut intents);
        } else {
            widgets::section(icon(&group.id), &group.label)
                .id_salt(&group.id)
                .default_open(groups <= 4)
                // What was searched for is never folded away.
                .forced(searching.then_some(true))
                .show(ui, |ui| draw(ui, &mut intents));
        }
    }
    if searching {
        // The panel says when nothing on the page matches.
        return (intents, any);
    }
    if !any {
        ui.label(
            egui::RichText::new(match selection.level {
                Level::Quick => "Its sliders are at the Detailed level.",
                _ => "Nothing to change here on this body.",
            })
            .color(palette.text_weak),
        );
    } else if selection.level == Level::Quick
        && base
            .parameters
            .iter()
            .filter(|p| base.group(&p.group).is_some_and(|g| g.category == data))
            .any(|p| !Level::Quick.shows(parameter_level(p)))
    {
        // Quick shows the broad controls only: say where the rest are.
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("More sliders at the Detailed level.")
                .small()
                .color(palette.text_weak),
        );
    }
    if let Some((count, needed)) = hidden_changes(base, description, data, selection.level) {
        ui.add_space(6.0);
        let text = format!(
            "{count} more {} in {}",
            if count == 1 { "change" } else { "changes" },
            needed.label()
        );
        if ui
            .link(egui::RichText::new(text).small())
            .on_hover_text("Show them: nothing changed is ever hidden")
            .clicked()
        {
            intents.push(Intent::PickLevel(needed));
        }
    }
    (intents, any)
}

/// The IDs a row's lock covers: the parameter and, on a mirrored row,
/// its mirror.
fn lock_ids(shown: &Shown) -> Vec<String> {
    let parameter = shown.parameter;
    let mut ids = vec![parameter.id.clone()];
    if shown.mirrored
        && let Some(mirror) = &parameter.mirror
    {
        ids.push(mirror.clone());
    }
    ids
}

/// One parameter's row: a slider, or chips for a choice; each with a lock
/// for Randomize, and a paired row with its pair's side tools (`paired`:
/// a row of its group has them, so this one keeps their column). `note`
/// is a quiet word after its label (the level a search found it at).
#[allow(clippy::too_many_arguments)]
fn parameter(
    ui: &mut egui::Ui,
    base: &BaseManifest,
    description: &Description,
    defaults: &Description,
    shown: &Shown,
    locks: &BTreeSet<String>,
    paired: bool,
    note: Option<&str>,
) -> Option<Intent> {
    let parameter = shown.parameter;
    let value = value_of(description, Some(base), &parameter.id);
    let ids = lock_ids(shown);
    let locked = ids.iter().all(|id| locks.contains(id));
    if !parameter.choices.is_empty() {
        let picked = parameter.nearest_choice(value).map(|choice| choice.value);
        let mut out = None;
        // One line, as a slider row is: the label in its share of the
        // width, the choices, the lock at the end.
        ui.horizontal(|ui| {
            let width = row::label_width(ui.available_width());
            row::label(
                ui,
                &parameter.label,
                note,
                &parameter_about(parameter),
                width,
            );
            ui.spacing_mut().item_spacing.x = 2.0;
            // The choices take the width up to the lock's column and go
            // on to more lines when there are more than fit (a body's
            // five skins): none is ever past the panel's edge.
            let room = (ui.available_width() - 2.0 * row::MARK - 8.0).max(0.0);
            ui.allocate_ui_with_layout(
                egui::vec2(room, 0.0),
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    for choice in &parameter.choices {
                        let chip = widgets::chip(ui, &choice.label, picked == Some(choice.value));
                        if chip.clicked() {
                            out = Some(Intent::Choose {
                                id: parameter.id.clone(),
                                value: choice.value,
                            });
                        }
                    }
                },
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Under the slider rows' locks: their reset's column is
                // the last.
                ui.add_space(row::MARK);
                if let Some(locked) = row::lock_toggle(ui, locked) {
                    out = Some(Intent::SetLock {
                        ids: ids.clone(),
                        locked,
                    });
                }
            });
        });
        return out;
    }
    let label = if shown.mirrored {
        symmetric_label(&parameter.label)
    } else {
        parameter.label.clone()
    };
    let about = parameter_about(parameter);
    let tools = parameter.mirror.is_some().then(|| {
        side_choices(
            description,
            defaults,
            base,
            &SidesOf::Pair(parameter.id.clone()),
        )
    });
    let sides = match &tools {
        Some(tools) => Sides::Tools(tools),
        None if paired => Sides::Column,
        None => Sides::None,
    };
    let asked = row::slider_with_sides(
        ui,
        &Row {
            id: &parameter.id,
            label: &label,
            note,
            about: &about,
            value,
            range: parameter.range,
            default: parameter.default,
            locked: Some(locked),
        },
        sides,
    )?;
    let (value, gesture) = match asked {
        Asked::Set(value, gesture) => (value, gesture),
        Asked::Lock(locked) => return Some(Intent::SetLock { ids, locked }),
        Asked::Sides(tool) => {
            return Some(Intent::Sides {
                tool,
                of: SidesOf::Pair(parameter.id.clone()),
            });
        }
    };

    let id = parameter.id.clone();
    Some(if shown.mirrored {
        Intent::SetMirrored { id, value, gesture }
    } else {
        Intent::SetParameter { id, value, gesture }
    })
}

/// A group's icon by its ID, for the groups the shipped bodies have.
fn icon(group: &str) -> &'static str {
    match group {
        "frame" => icons::PERSON,
        "build" | "build_regions" => icons::BARBELL,
        "proportions" => icons::RULER,
        "eyes" => icons::EYE,
        "ears" => icons::EAR,
        "lips" => icons::SMILEY,
        _ => icons::SLIDERS_HORIZONTAL,
    }
}
