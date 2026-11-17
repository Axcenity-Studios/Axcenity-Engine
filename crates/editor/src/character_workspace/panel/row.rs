//! A slider row: the label (its tooltip says what the control does), the
//! value (typed at the Advanced level), a reset to the default when it
//! differs, a lock for Randomize where the row has one, a left and right
//! pair's side tools (R4) where it is one, and the slider across the
//! panel. A changed row gets the inspectors' accent bar in the gutter.

use crate::theme::icons;

use eframe::egui;

use super::super::intent::Gesture;
use super::super::shell::gesture;
use super::super::sides::SideTool;
use crate::theme;
use crate::widgets;

/// One row's data.
pub(super) struct Row<'a> {
    /// Keeps the widgets' state apart from other rows'.
    pub id: &'a str,
    pub label: &'a str,
    /// A quiet word after the label ("not drawn yet").
    pub note: Option<&'a str>,
    pub about: &'a str,
    pub value: f32,
    pub range: [f32; 2],
    pub default: f32,
    /// The value can be typed (Advanced); otherwise it's only shown.
    pub typed: bool,
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
    let text = match label {
        Some(label) => egui::RichText::new(format!(
            "{}  {label}  {}",
            icons::ARROWS_LEFT_RIGHT,
            icons::CARET_DOWN
        ))
        .small()
        .color(palette.text_secondary),
        None => egui::RichText::new(icons::ARROWS_LEFT_RIGHT)
            .size(12.0)
            .color(palette.text_muted),
    };
    let button = egui::Button::new(text).frame(label.is_some());
    let (response, _) = menu::MenuButton::from_button(button).ui(ui, |ui| {
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
    });
    response.on_hover_text(tip);
    picked
}

/// A lock's small toggle: lit while locked. Returns the new state when
/// clicked.
pub(super) fn lock_toggle(ui: &mut egui::Ui, locked: bool) -> Option<bool> {
    let palette = theme::palette(ui.ctx());
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
    let response = ui
        .add(
            egui::Button::new(egui::RichText::new(icon).size(12.0).color(if locked {
                palette.accent
            } else {
                palette.text_muted
            }))
            .frame(false),
        )
        .on_hover_text(tip);
    response.clicked().then_some(!locked)
}

/// Draws `row`; returns what it asks for.
pub(super) fn slider(ui: &mut egui::Ui, row: &Row) -> Option<Asked> {
    slider_with_sides(ui, row, None)
}

/// [`slider`], with the side tools of the left and right pair it edits.
pub(super) fn slider_with_sides(
    ui: &mut egui::Ui,
    row: &Row,
    sides: Option<&[(SideTool, bool)]>,
) -> Option<Asked> {
    let palette = theme::palette(ui.ctx());
    let [low, high] = row.range;
    let range = low.min(high)..=low.max(high);
    let changed = (row.value - row.default).abs() > 1.0e-4;
    let mut value = row.value;
    let mut out = None;
    let top = ui.cursor().min;
    ui.push_id(row.id, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(row.label).color(palette.text_secondary))
                .on_hover_text(row.about);
            if let Some(note) = row.note {
                ui.label(egui::RichText::new(note).small().color(palette.text_muted))
                    .on_hover_text(row.about);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if row.typed {
                    let response = ui.add(
                        egui::DragValue::new(&mut value)
                            .range(range.clone())
                            .speed((range.end() - range.start()) / 300.0)
                            .fixed_decimals(2),
                    );
                    if response.changed() || response.drag_stopped() {
                        out = Some(Asked::Set(value, gesture(&response)));
                    }
                } else {
                    ui.label(
                        egui::RichText::new(format!("{value:.2}"))
                            .small()
                            .color(palette.text_weak),
                    );
                }
                if changed
                    && widgets::reset_button(
                        ui,
                        &format!("Back to the default ({:.2})", row.default),
                    )
                {
                    out = Some(Asked::Set(row.default, Gesture::Commit));
                }
                if let Some(locked) = row.locked
                    && let Some(locked) = lock_toggle(ui, locked)
                {
                    out = Some(Asked::Lock(locked));
                }
                if let Some(tools) = sides
                    && let Some(tool) =
                        sides_menu(ui, tools, None, "Left and right: mirror or reset a side")
                {
                    out = Some(Asked::Sides(tool));
                }
            });
        });
        ui.spacing_mut().slider_width = ui.available_width();
        let response = ui.add(
            egui::Slider::new(&mut value, range)
                .show_value(false)
                .clamping(egui::SliderClamping::Always),
        );
        let response = response.on_hover_text(row.about);
        if response.double_clicked() {
            out = Some(Asked::Set(row.default, Gesture::Commit));
        } else if response.changed() || response.drag_stopped() {
            out = Some(Asked::Set(value, gesture(&response)));
        }
    });
    if changed {
        let rect =
            egui::Rect::from_min_max(top, egui::pos2(ui.max_rect().right(), ui.cursor().min.y));
        widgets::changed_bar(ui, rect);
    }
    ui.add_space(2.0);
    out
}
