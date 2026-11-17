//! The bar under the viewport (brief §3.1, "Bottom viewport controls"; the
//! lead's DIS-06): camera presets, lighting and backdrop, which were the
//! Stage page. All of it is about looking, not the character: none of it
//! is saved or undone. Pose, clips, screenshot and turntable come later.

use eframe::egui;

use super::CharacterWorkspace;
use super::intent::Intent;
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
        quiet(ui, icons::CAMERA, "Camera");
        for view in ViewPreset::ALL {
            // The chip of the view the camera is at is lit (audit D-12).
            if widgets::chip(ui, view.label(), workspace.view == Some(view))
                .on_hover_text(format!("{} view", view.label()))
                .clicked()
            {
                intents.push(Intent::FocusView(view));
            }
        }
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
