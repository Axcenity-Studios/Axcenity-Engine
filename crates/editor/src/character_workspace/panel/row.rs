//! A slider row: the label (its tooltip says what the control does), the
//! value (typed at the Advanced level), a reset to the default when it
//! differs, and the slider across the panel. A changed row gets the
//! inspectors' accent bar in the gutter.

use eframe::egui;

use super::super::intent::Gesture;
use super::super::shell::gesture;
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
}

/// Draws `row`; returns a new value and where it is in its gesture.
pub(super) fn slider(ui: &mut egui::Ui, row: &Row) -> Option<(f32, Gesture)> {
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
                        out = Some((value, gesture(&response)));
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
                    out = Some((row.default, Gesture::Commit));
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
            out = Some((row.default, Gesture::Commit));
        } else if response.changed() || response.drag_stopped() {
            out = Some((value, gesture(&response)));
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
