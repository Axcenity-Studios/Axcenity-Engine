//! A slider row, on one line: the label on the left in a fixed share of
//! the width (cut short with "…" when it doesn't fit; its tooltip says the
//! whole label and what the control does), the track in the middle, the
//! value (click to type it, at every level) and, after it, the row's small
//! marks, each in a column of its own so the tracks of a section line up:
//! a left and right pair's side tools (⇄, R4) where the row is one, a lock
//! for Randomize where it has one, and a reset to the default, drawn only
//! while the value differs. The track and the value drag and type by the
//! studio's one rule (`numeric.rs`: 110 points for the whole range, Shift
//! a quarter, Ctrl double). A changed row gets the inspectors' accent bar
//! in the gutter. What a row says about a typed value ("kept to the
//! range") goes under it while it lasts.

use crate::theme::icons;

use eframe::egui::{self, vec2};

use super::super::intent::Gesture;
use super::super::numeric;
use super::super::sides::SideTool;
use crate::responsive;
use crate::theme;
use crate::widgets;

/// How wide a mark's column is.
pub(super) const MARK: f32 = 16.0;
/// The room between a row's parts.
const GAP: f32 = 6.0;
/// The least a track is drawn at: narrower, the label gives way.
const MIN_TRACK: f32 = 44.0;

/// One row's data.
pub(super) struct Row<'a> {
    /// Keeps the widgets' state apart from other rows'.
    pub id: &'a str,
    pub label: &'a str,
    /// A quiet word after the label ("not drawn yet", "Advanced").
    pub note: Option<&'a str>,
    pub about: &'a str,
    pub value: f32,
    pub range: [f32; 2],
    pub default: f32,
    /// Whether Randomize leaves it alone, for a row with a lock.
    pub locked: Option<bool>,
}

/// What a row asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Asked {
    /// A new value, and where it is in its gesture.
    Set(f32, Gesture),
    /// Locked (or unlocked) for Randomize.
    Lock(bool),
    /// A side tool for the row's left and right pair.
    Sides(SideTool),
}

/// The side tools of a row's left and right pair, for [`slider_with_sides`].
#[derive(Clone, Copy)]
pub(super) enum Sides<'a> {
    /// The row is no pair, and no row drawn with it is: no column.
    None,
    /// The row is no pair, but rows drawn with it are: their column is
    /// kept empty, so the tracks line up.
    Column,
    /// Each tool, and whether it would change anything.
    Tools(&'a [(SideTool, bool)]),
}

/// The share of a row `width` wide its label takes ([`label`] draws it):
/// what a row of chips (a choice) lines up with too.
pub(super) fn label_width(width: f32) -> f32 {
    responsive::name_width(width)
}

/// A row's label in `width` points: cut short with "…" where it doesn't
/// fit, a quiet `note` after it, the whole label and `about` on hover.
pub(super) fn label(
    ui: &mut egui::Ui,
    text: &str,
    note: Option<&str>,
    about: &str,
    width: f32,
) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let height = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        // The note is said whole: the label gives way to it.
        let note = note.map(|note| {
            ui.painter().layout_no_wrap(
                note.to_owned(),
                egui::TextStyle::Small.resolve(ui.style()),
                palette.text_muted,
            )
        });
        let taken = note.as_ref().map_or(0.0, |note| note.size().x + 5.0);
        let galley = widgets::elided(
            ui,
            text,
            egui::TextStyle::Body.resolve(ui.style()),
            palette.text_secondary,
            (width - taken).max(16.0),
            1,
        );
        let at = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        let end = at.x + galley.size().x;
        ui.painter().galley(at, galley, palette.text_secondary);
        if let Some(note) = note {
            let at = egui::pos2(end + 5.0, rect.center().y - note.size().y / 2.0 + 0.5);
            ui.painter().galley(at, note, palette.text_muted);
        }
    }
    // The whole label is said too: it may be cut short.
    let tip = match (about == text || about.is_empty(), note) {
        (true, None) => text.to_owned(),
        (true, Some(note)) => format!("{text} ({note})"),
        (false, None) => format!("{text}\n{about}"),
        (false, Some(note)) => format!("{text} ({note})\n{about}"),
    };
    response.on_hover_text(tip)
}

/// A row's small mark: an icon in a column [`MARK`] wide. Returns its
/// response; `lit` draws it in the accent colour.
fn mark(ui: &mut egui::Ui, icon: &str, tip: &str, lit: bool) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let height = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(vec2(MARK, height), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), lit, tip)
    });
    if ui.is_rect_visible(rect) {
        let colour = if lit {
            palette.accent
        } else if response.hovered() {
            palette.text_strong
        } else {
            palette.text_muted
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(12.0),
            colour,
        );
    }
    response.on_hover_text(tip)
}

/// The side tools as a small menu (⇄, with `label` after it where given):
/// each tool, greyed out where it would change nothing. Returns the tool
/// picked.
pub(super) fn sides_menu(
    ui: &mut egui::Ui,
    tools: &[(SideTool, bool)],
    label: Option<&str>,
    tip: &str,
) -> Option<SideTool> {
    use egui::containers::menu;
    let palette = theme::palette(ui.ctx());
    let mut picked = None;
    let button = match label {
        Some(label) => egui::Button::new(
            egui::RichText::new(format!(
                "{}  {label}  {}",
                icons::ARROWS_LEFT_RIGHT,
                icons::CARET_DOWN
            ))
            .small()
            .color(palette.text_secondary),
        ),
        // A row's own: as wide as any other mark.
        None => egui::Button::new(
            egui::RichText::new(icons::ARROWS_LEFT_RIGHT)
                .size(12.0)
                .color(palette.text_muted),
        )
        .frame(false)
        .min_size(vec2(MARK, ui.spacing().interact_size.y)),
    };
    let (response, _) = ui
        .scope(|ui| {
            if label.is_none() {
                ui.spacing_mut().button_padding = vec2(0.0, 0.0);
            }
            menu::MenuButton::from_button(button).ui(ui, |ui| {
                for (tool, enabled) in tools {
                    if ui
                        .add_enabled(*enabled, egui::Button::new(tool.label()))
                        .on_hover_text(tool.about())
                        .on_disabled_hover_text("Nothing to change")
                        .clicked()
                    {
                        picked = Some(*tool);
                        ui.close();
                    }
                }
            })
        })
        .inner;
    response.on_hover_text(tip);
    picked
}

/// A lock's small toggle: lit while locked. Returns the new state when
/// clicked.
pub(super) fn lock_toggle(ui: &mut egui::Ui, locked: bool) -> Option<bool> {
    let (icon, tip) = if locked {
        (
            icons::LOCK_SIMPLE,
            "Locked: Randomize leaves it alone (click to unlock)",
        )
    } else {
        (
            icons::LOCK_SIMPLE_OPEN,
            "Lock it so Randomize leaves it alone",
        )
    };
    mark(ui, icon, tip, locked).clicked().then_some(!locked)
}

/// Draws `row`; returns what it asks for.
pub(super) fn slider(ui: &mut egui::Ui, row: &Row) -> Option<Asked> {
    slider_with_sides(ui, row, Sides::None)
}

/// [`slider`], with the side tools of the left and right pair it edits.
pub(super) fn slider_with_sides(ui: &mut egui::Ui, row: &Row, sides: Sides) -> Option<Asked> {
    let changed = (row.value - row.default).abs() > 1.0e-4;
    let mut out = None;
    let top = ui.cursor().min;
    // The marks' columns: the reset's always (the track mustn't move when
    // a drag takes the value off its default), the others where the row,
    // or a row drawn with it, has them.
    let marks = 1 + usize::from(row.locked.is_some()) + usize::from(!matches!(sides, Sides::None));
    ui.push_id(row.id, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            let width = ui.available_width();
            let after = numeric::FIELD_WIDTH + GAP + marks as f32 * MARK;
            // In a very narrow panel the label gives way to the track.
            let label_width = label_width(width)
                .min(width - after - MIN_TRACK - 2.0 * GAP)
                .max(40.0);
            let track = (width - label_width - after - 2.0 * GAP).max(MIN_TRACK);
            label(ui, row.label, row.note, row.about, label_width);
            let (response, asked) =
                numeric::track(ui, row.id, row.value, row.range, row.about, track);
            if response.double_clicked() {
                out = Some(Asked::Set(row.default, Gesture::Commit));
            } else if let Some((value, gesture)) = asked {
                out = Some(Asked::Set(value, gesture));
            }
            if let Some((value, gesture)) = numeric::field(ui, row.id, row.value, row.range) {
                out = Some(Asked::Set(value, gesture));
            }
            // The marks sit together, close after the value.
            ui.spacing_mut().item_spacing.x = 0.0;
            match sides {
                Sides::None => {}
                Sides::Column => {
                    ui.add_space(MARK);
                }
                Sides::Tools(tools) => {
                    if let Some(tool) =
                        sides_menu(ui, tools, None, "Left and right: mirror or reset a side")
                    {
                        out = Some(Asked::Sides(tool));
                    }
                }
            }
            if let Some(locked) = row.locked
                && let Some(locked) = lock_toggle(ui, locked)
            {
                out = Some(Asked::Lock(locked));
            }
            if changed {
                let tip = format!(
                    "Back to the default ({})",
                    numeric::shown(row.default, row.range)
                );
                if mark(ui, icons::ARROW_COUNTER_CLOCKWISE, &tip, false).clicked() {
                    out = Some(Asked::Set(row.default, Gesture::Commit));
                }
            } else {
                ui.add_space(MARK);
            }
        });
        numeric::note(ui, row.id);
    });
    if changed {
        let rect =
            egui::Rect::from_min_max(top, egui::pos2(ui.max_rect().right(), ui.cursor().min.y));
        widgets::changed_bar(ui, rect);
    }
    out
}
