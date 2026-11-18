//! Comparison (CO19, brief §12; ux patterns section 6): held states. A
//! held state is a whole description kept in the workspace's memory, up to
//! three (A, B, C), outside undo and never written to the character's
//! file. One can be shown in place of the current character (before and
//! after: same viewport, camera and light), or beside it (side by side: a
//! second character in the preview World, applied with the same applier,
//! drawn with the same camera moved over to it). "Keep this" makes a held
//! state the character as one undo step; "Discard" drops it. Starting
//! another character drops them all.

use axcenity_avatar::Description;
use axcenity_core::Vector3;
use axcenity_gameplay::description as applier;
use axcenity_render::glam::Vec3;
use eframe::egui;

use super::intent::Intent;
use super::{CharacterWorkspace, stage, tidy};
use crate::theme::{self, icons};
use crate::widgets;

/// How many states can be held.
pub const MAX_HELD: usize = 3;

/// Where the second character stands for side by side, 30 studs along -X.
/// It shares the preview World, so a camera looking along the X axis (the
/// Side view) has it 30 studs behind the main character: mostly hidden by
/// it, but its floor can show near the horizon when zoomed out.
pub const OTHER_AT: Vec3 = Vec3::new(-30.0, 0.0, 0.0);

/// The card the comparison's controls and labels sit on over the viewport:
/// the panel colour, nearly opaque, so the panel's text colours read on it
/// whatever the backdrop (Grey, Light, Dark or Sky).
pub(super) fn backing(palette: &theme::Palette) -> egui::Color32 {
    palette.panel.gamma_multiply(0.92)
}

/// The held states and what is shown (editor state only).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Comparison {
    /// A, B, C.
    pub held: Vec<Description>,
    /// The held state shown: in place of the current character, or (side
    /// by side) beside it.
    pub showing: Option<usize>,
    pub side_by_side: bool,
}

impl Comparison {
    /// A held state's letter.
    pub fn letter(index: usize) -> char {
        (b'A' + index.min(25) as u8) as char
    }

    /// The held state shown beside the current one, side by side (the
    /// picked one, or A).
    pub fn beside(&self) -> Option<usize> {
        (self.side_by_side && !self.held.is_empty())
            .then(|| self.showing.filter(|i| *i < self.held.len()).unwrap_or(0))
    }

    /// The held state shown in place of the current character.
    pub fn instead(&self) -> Option<usize> {
        (!self.side_by_side)
            .then_some(self.showing)
            .flatten()
            .filter(|i| *i < self.held.len())
    }
}

impl CharacterWorkspace {
    /// What the main preview shows: a held state in its place, a browser
    /// tile tried on (the character wearing it, not yet worn), or the
    /// character being edited.
    pub fn shown(&self) -> &Description {
        match self.comparison.instead() {
            Some(index) => &self.comparison.held[index],
            None => self.browser.preview.as_ref().unwrap_or(&self.description),
        }
    }

    /// Holds the current character as the next of A, B and C.
    pub fn capture(&mut self) {
        if self.comparison.held.len() >= MAX_HELD {
            self.notice = Some(format!(
                "{MAX_HELD} versions are held: discard one to hold another."
            ));
            return;
        }
        self.comparison.held.push(self.description.clone());
        let letter = Comparison::letter(self.comparison.held.len() - 1);
        self.notice = Some(format!(
            "Held as {letter}. Change the character, then compare it with {letter}."
        ));
    }

    /// Shows held state `index` (or, with `None`, the current character).
    pub fn show_held(&mut self, index: Option<usize>) {
        let was = self.comparison.instead();
        self.comparison.showing = index.filter(|i| *i < self.comparison.held.len());
        self.say_what_is_shown(was);
        self.refresh_previews();
    }

    /// Side by side on or off.
    pub fn set_side_by_side(&mut self, on: bool) {
        let was = self.comparison.beside();
        self.comparison.side_by_side = on && !self.comparison.held.is_empty();
        self.say_what_is_shown(was);
        self.refresh_previews();
    }

    /// The status line says what the preview shows now, in place of the
    /// hint left from holding ("Held as A…"); `was` is the held version
    /// shown before, if one was.
    fn say_what_is_shown(&mut self, was: Option<usize>) {
        let comparison = &self.comparison;
        self.notice = if let Some(index) = comparison.beside() {
            let letter = Comparison::letter(index);
            Some(format!(
                "Side by side: {letter} beside the current character, one camera for both."
            ))
        } else if let Some(index) = comparison.instead() {
            let letter = Comparison::letter(index);
            Some(format!(
                "Showing {letter} (held). Editing is off until you go back to Current."
            ))
        } else if was.is_some() {
            Some("Showing the current character.".to_owned())
        } else {
            self.notice.take()
        };
    }

    /// Makes held state `index` the character, as one undo step; the name
    /// stays the current one (a rename isn't a version).
    pub fn keep_held(&mut self, index: usize) {
        let Some(mut kept) = self.comparison.held.get(index).cloned() else {
            return;
        };
        kept.name = self.description.name.clone();
        self.comparison.showing = None;
        self.edit(|description| *description = kept);
        self.refresh_previews();
    }

    /// Drops held state `index`.
    pub fn discard_held(&mut self, index: usize) {
        if index >= self.comparison.held.len() {
            return;
        }
        let was = (self.comparison.instead(), self.comparison.beside());
        self.comparison.held.remove(index);
        let comparison = &mut self.comparison;
        comparison.showing = match comparison.showing {
            Some(shown) if shown == index => None,
            Some(shown) if shown > index => Some(shown - 1),
            other => other,
        };
        if comparison.held.is_empty() {
            comparison.side_by_side = false;
        }
        if was.0 == Some(index) || was.1 == Some(index) {
            // What the status line said is shown is gone.
            self.say_what_is_shown(Some(index));
        }
        self.refresh_previews();
    }

    /// Drops every held state (another character was started), and a
    /// status line about them.
    pub(super) fn clear_comparison(&mut self) {
        self.comparison = Comparison::default();
        if self.notice.as_deref().is_some_and(|notice| {
            ["Showing ", "Side by side:", "Held as "]
                .iter()
                .any(|start| notice.starts_with(start))
        }) {
            self.notice = None;
        }
        self.refresh_previews();
    }

    /// The main preview and, side by side, the second character, brought
    /// up to date with what they show.
    pub(super) fn refresh_previews(&mut self) {
        self.refresh();
        self.refresh_other();
    }

    /// Applies the held state shown side by side to the second character,
    /// making it the first time; removes it when side by side is off.
    pub(super) fn refresh_other(&mut self) {
        let Some(index) = self.comparison.beside() else {
            // The second character goes with its own floor.
            for id in [self.other.take(), self.other_floor.take()]
                .into_iter()
                .flatten()
            {
                let _ = self.world.destroy(id);
            }
            self.other_applied = None;
            return;
        };
        let wanted = self.comparison.held[index].clone();
        let other = match self.other {
            Some(other) => other,
            None => {
                let at = Vector3::new(OTHER_AT.x, OTHER_AT.y, OTHER_AT.z);
                let other = axcenity_project::default_character(&mut self.world, at);
                let workspace = self.world.workspace();
                let _ = self.world.set_parent(other, Some(workspace));
                self.other_floor = stage::add_floor_at(&mut self.world, OTHER_AT.x);
                self.other = Some(other);
                self.other_applied = None;
                // It moves as the current one does (Walk, a gesture).
                self.pose_previews();
                other
            }
        };
        if self.other_applied.as_ref() == Some(&wanted) {
            return;
        }
        let library = self.library();
        let wanted = tidy(wanted, &library);
        if let Err(error) =
            applier::apply_description(&mut self.world, &self.assets, other, &wanted)
        {
            self.notice = Some(format!("Could not show the held version: {error}"));
        }
        self.other_applied = Some(self.comparison.held[index].clone());
    }

    /// The second character's description as applied (tests).
    #[cfg(test)]
    pub(super) fn other_shows(&self) -> Option<&Description> {
        self.other.and(self.other_applied.as_ref())
    }
}

/// The comparison strip over the top of the viewport: hold the current
/// version, flip between held ones and the current one, side by side, and
/// keep or discard the one shown. The row sits on a card of the panel's
/// colour ([`backing`]): its chips and icon are the panel's, and drawn
/// straight on a light backdrop their quiet text could hardly be read.
pub(super) fn strip(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let comparison = &workspace.comparison;
    // A horizontal row whose one child is the card, so the card is only as
    // wide as what is on it.
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(backing(&palette))
            .corner_radius(theme::radius::CARD)
            .inner_margin(4)
            .show(ui, |ui| strip_row(ui, comparison, &palette, &mut intents));
    });
    intents
}

/// The strip's controls, in a row.
fn strip_row(
    ui: &mut egui::Ui,
    comparison: &Comparison,
    palette: &theme::Palette,
    intents: &mut Vec<Intent>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let next = Comparison::letter(comparison.held.len());
        let full = comparison.held.len() >= MAX_HELD;
        if ui
            .add_enabled(
                !full,
                widgets::button(
                    widgets::Role::Secondary,
                    Some(icons::CAMERA),
                    &format!("Hold as {next}"),
                    palette,
                ),
            )
            .on_hover_text(
                "Keep this version to compare with (not saved, not an undo step; up to three)",
            )
            .on_disabled_hover_text("Three versions are held: discard one first")
            .clicked()
        {
            intents.push(Intent::Capture);
        }
        if comparison.held.is_empty() {
            return;
        }
        ui.add_space(6.0);
        let shown = if comparison.side_by_side {
            comparison.beside()
        } else {
            comparison.showing
        };
        for index in 0..comparison.held.len() {
            let letter = Comparison::letter(index).to_string();
            if widgets::chip(ui, &letter, shown == Some(index))
                .on_hover_text(if comparison.side_by_side {
                    format!("Show {letter} beside the current version")
                } else {
                    format!("Show {letter} (the same camera and light)")
                })
                .clicked()
            {
                intents.push(Intent::ShowHeld(Some(index)));
            }
        }
        if !comparison.side_by_side
            && widgets::chip(ui, "Current", comparison.showing.is_none())
                .on_hover_text("Show the version you're editing")
                .clicked()
        {
            intents.push(Intent::ShowHeld(None));
        }
        ui.add_space(6.0);
        if widgets::chip(ui, "Side by side", comparison.side_by_side)
            .on_hover_text("A held version beside the current one, one camera for both")
            .clicked()
        {
            intents.push(Intent::SideBySide(!comparison.side_by_side));
        }
        if let Some(index) = shown {
            let letter = Comparison::letter(index);
            ui.add_space(6.0);
            if ui
                .add(widgets::button(
                    widgets::Role::Primary,
                    Some(icons::CHECK),
                    &format!("Keep {letter}"),
                    palette,
                ))
                .on_hover_text(format!(
                    "Make {letter} the character (one undo step; the name stays)"
                ))
                .clicked()
            {
                intents.push(Intent::KeepHeld(index));
            }
            if widgets::icon_button(ui, icons::TRASH, &format!("Discard {letter}"), false).clicked()
            {
                intents.push(Intent::DiscardHeld(index));
            }
        }
    });
}
