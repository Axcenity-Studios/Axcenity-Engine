//! The Expressions page (refinement R9, brief §10): the body's pose and
//! motion, the face preview, the 15 built-in expressions as tiles (Neutral
//! first: it puts the face back) and the face controls grouped by feature.
//! Everything here is a preview: no intent it returns changes the
//! description or undo.

use eframe::egui::{self, Sense, vec2};

use super::super::CharacterWorkspace;
use super::super::expressions::{FaceGroup, ROWS};
use super::super::intent::Intent;
use super::super::stage::Lighting;
use super::row::{self, Asked, Row};
use crate::theme::{self, icons};
use crate::widgets;

/// A tile's icon for each built-in expression.
fn icon(name: &str) -> &'static str {
    match name {
        "Neutral" => icons::SMILEY_BLANK,
        "Happy" | "Excited" | "Laughing" => icons::SMILEY,
        "Sad" | "Crying" => icons::SMILEY_SAD,
        "Angry" | "Frustrated" | "Determined" => icons::SMILEY_ANGRY,
        "Surprised" | "Fearful" => icons::SMILEY_NERVOUS,
        "Confused" | "Worried" => icons::SMILEY_MEH,
        "Disgusted" => icons::SMILEY_X_EYES,
        "Sleepy" => icons::SMILEY_MELTING,
        _ => icons::SMILEY_WINK,
    }
}

/// Draws the page.
pub(in super::super) fn page(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    widgets::section_label(ui, "Pose and motion");
    intents.extend(super::motion_chips(ui, workspace));
    ui.label(
        egui::RichText::new(
            "Walk and Run play at their own pace, in place. The camera follows the head.",
        )
        .small()
        .color(palette.text_weak),
    );
    ui.add_space(8.0);
    intents.extend(face_preview(ui, workspace));
    ui.add_space(8.0);
    intents.extend(tiles(ui, workspace));
    ui.add_space(4.0);
    intents.extend(controls(ui, workspace));
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "A preview only: nothing here is saved with the character. Expressions are made \
             and edited in the Game Editor's Face Rig tab.",
        )
        .small()
        .color(palette.text_weak),
    );
    intents
}

/// The face preview's switch and, while it's on, blinking, idle motion
/// and the neutral light.
fn face_preview(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let on = workspace.face_view.is_some();
    ui.horizontal(|ui| {
        let mut shown = on;
        if ui
            .toggle_value(
                &mut shown,
                theme::with_icon(icons::USER_FOCUS, "Face preview"),
            )
            .on_hover_text(
                "Close on the face, the body still, no blinking: the expression is held to be \
                 looked at. Off puts the view back.",
            )
            .changed()
        {
            intents.push(Intent::FacePreview(shown));
        }
    });
    if let Some(view) = &workspace.face_view {
        ui.horizontal_wrapped(|ui| {
            let (mut blink, mut idle) = (view.blink, view.idle);
            let blink_changed = ui
                .checkbox(&mut blink, "Blink")
                .on_hover_text("Let the eyes blink now and then")
                .changed();
            let idle_changed = ui
                .checkbox(&mut idle, "Idle motion")
                .on_hover_text("Let the face and the body move a little, as when standing")
                .changed();
            if blink_changed || idle_changed {
                intents.push(Intent::FacePreviewMotion { blink, idle });
            }
            let neutral = workspace.lighting == Lighting::Neutral;
            if widgets::chip(ui, "Neutral light", neutral)
                .on_hover_text(
                    "Soft, even, white light for judging a face (the bottom bar has the others)",
                )
                .clicked()
                && !neutral
            {
                intents.push(Intent::SetLighting(Lighting::Neutral));
            }
        });
    } else {
        ui.label(
            egui::RichText::new("Frames the face and holds it still to look at.")
                .small()
                .color(palette.text_weak),
        );
    }
    intents
}

/// The built-in expressions as tiles, three a row; the one shown is lit
/// (Neutral when none is). A click shows one, starting the controls from
/// its values; Neutral puts every control back.
fn tiles(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    widgets::section_label(ui, "Expressions");
    let columns = 3;
    let gap = 6.0;
    let width = ((ui.available_width() - gap * (columns as f32 - 1.0)) / columns as f32).floor();
    let held = !workspace.face_controls.is_empty();
    for names in axcenity_anim::BUILT_IN_EXPRESSIONS.chunks(columns) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for name in names {
                let shown = match &workspace.expression {
                    Some(shown) => shown.eq_ignore_ascii_case(name),
                    None => *name == "Neutral" && !held,
                };
                let (rect, response) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click());
                let painter = ui.painter_at(rect);
                painter.rect_filled(
                    rect,
                    theme::radius::CARD,
                    if shown {
                        palette.control_active
                    } else {
                        palette.inset
                    },
                );
                if shown || response.hovered() {
                    painter.rect_stroke(
                        rect,
                        theme::radius::CARD,
                        egui::Stroke::new(
                            if shown { 1.5 } else { 1.0 },
                            if shown {
                                palette.accent
                            } else {
                                palette.text_muted
                            },
                        ),
                        egui::StrokeKind::Inside,
                    );
                }
                let color = if shown {
                    palette.text_strong
                } else {
                    palette.text_secondary
                };
                painter.text(
                    rect.left_center() + vec2(8.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    icon(name),
                    egui::FontId::proportional(14.0),
                    color,
                );
                painter.text(
                    rect.left_center() + vec2(27.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    egui::FontId::proportional(12.5),
                    color,
                );
                let tip = if *name == "Neutral" {
                    "The face at rest: no expression, every control back".to_owned()
                } else {
                    format!("Show {name} on the face (a preview only)")
                };
                if response.on_hover_text(tip).clicked() {
                    intents.push(Intent::PreviewExpression(Some((*name).to_owned())));
                }
            }
        });
        ui.add_space(gap);
    }
    if held {
        ui.label(
            egui::RichText::new(match &workspace.expression {
                Some(name) => format!("{name}, changed below. Pick it again to start over."),
                None => "Changed below. Neutral puts every control back.".to_owned(),
            })
            .small()
            .color(palette.text_weak),
        );
    }
    intents
}

/// The face controls, a section per feature, each slider starting from
/// what the expression shown gives it. Its reset (and a double click) goes
/// back to the expression's value.
///
/// The sliders are the panel's shared slider rows ([`row::slider`]), so
/// they follow the studio's drag rule (`numeric.rs`: a full range in 110
/// points of drag, Shift a quarter, Ctrl double, a value can be typed):
/// nothing here sets a drag speed of its own (`numeric_tests.rs` checks
/// these rows).
fn controls(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let mut intents = Vec::new();
    for group in FaceGroup::ALL {
        let icon = match group {
            FaceGroup::Brows => icons::EYES,
            FaceGroup::Eyes => icons::EYE,
            FaceGroup::Cheeks | FaceGroup::Nose => icons::SMILEY,
            FaceGroup::Mouth => icons::SMILEY_WINK,
            FaceGroup::Jaw => icons::SMILEY_NERVOUS,
        };
        let salt = format!("character studio face controls {}", group.label());
        widgets::section(icon, group.label())
            .id_salt(&salt)
            .show(ui, |ui| {
                for face_row in ROWS.iter().filter(|row| row.group == group) {
                    let id = format!("face control {} {}", group.label(), face_row.label);
                    let value = workspace.row_value(face_row.controls, false);
                    let default = workspace.row_value(face_row.controls, true);
                    let row = Row {
                        id: &id,
                        label: face_row.label,
                        note: None,
                        about: face_row.about,
                        value,
                        range: face_row.controls.range(),
                        default,
                        locked: None,
                    };
                    if let Some(Asked::Set(value, _)) = row::slider(ui, &row) {
                        // Back at the expression's own value: let the
                        // controls go, so the expression shows through.
                        let back = (value - default).abs() < 1e-4;
                        for (name, value) in face_row.controls.values(value) {
                            intents.push(Intent::PreviewFaceControl {
                                name,
                                value: (!back).then_some(value),
                            });
                        }
                    }
                }
            });
    }
    intents
}
