//! The bar under the viewport (brief §3.1, "Bottom viewport controls"; the
//! lead's DIS-06): camera presets, the pose and motion (R9: Still, Idle,
//! Walk, Run, gestures; the Expressions page has the same as chips),
//! lighting and backdrop, which were the Stage page. All of it is about
//! looking, not the character: none of it is saved or undone. Screenshot
//! and turntable come later.

use eframe::egui;

use super::CharacterWorkspace;
use super::intent::Intent;
use super::motion::Motion;
use super::stage::{Backdrop, Lighting, ViewPreset};
use crate::theme::{self, icons};
use crate::widgets;

/// Draws the bar and returns what it asks for.
pub(super) fn show(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let quiet = |ui: &mut egui::Ui, icon: &str, tip: &str| {
        ui.label(egui::RichText::new(icon).color(palette.text_muted))
            .on_hover_text(tip);
    };
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // The view chips name themselves; the bar's room goes to the
        // motion menu after them.
        for view in ViewPreset::ALL {
            // The chip of the view the camera is at is lit (audit D-12).
            if widgets::chip(ui, view.label(), workspace.view == Some(view))
                .on_hover_text(format!("{} view", view.label()))
                .clicked()
            {
                intents.push(Intent::FocusView(view));
            }
        }
        intents.extend(motion_menu(ui, workspace, &palette));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for backdrop in Backdrop::ALL.iter().rev() {
                if widgets::chip(ui, backdrop.label(), workspace.backdrop == *backdrop)
                    .on_hover_text("Backdrop (not saved with the character)")
                    .clicked()
                {
                    intents.push(Intent::SetBackdrop(*backdrop));
                }
            }
            quiet(ui, icons::IMAGE, "Backdrop");
            ui.add_space(8.0);
            for lighting in Lighting::ALL.iter().rev() {
                if widgets::chip(ui, lighting.label(), workspace.lighting == *lighting)
                    .on_hover_text("Lighting (not saved with the character)")
                    .clicked()
                {
                    intents.push(Intent::SetLighting(*lighting));
                }
            }
            quiet(ui, icons::SUN, "Lighting");
        });
    });
    intents
}

/// The pose and motion as a compact menu (the bar has no room for nine
/// chips): the walking figure and what plays, in the accent colour while
/// the body moves.
fn motion_menu(
    ui: &mut egui::Ui,
    workspace: &CharacterWorkspace,
    palette: &theme::Palette,
) -> Option<Intent> {
    use egui::containers::menu;
    let mut out = None;
    let color = if workspace.motion.moves() {
        palette.accent
    } else {
        palette.text_secondary
    };
    let button = egui::Button::new(
        egui::RichText::new(format!(
            "{} {}",
            icons::PERSON_SIMPLE_WALK,
            workspace.motion.label()
        ))
        .color(color),
    )
    .frame(false);
    let (response, _) = menu::MenuButton::from_button(button).ui(ui, |ui| {
        for motion in Motion::all() {
            let on = workspace.motion == motion;
            if ui
                .add(egui::Button::selectable(on, motion.label()))
                .on_hover_text(motion.about())
                .clicked()
            {
                if !on {
                    out = Some(Intent::PreviewMotion(motion));
                }
                ui.close();
            }
        }
    });
    response.on_hover_text("Pose and motion (a preview only, not saved with the character)");
    out
}
