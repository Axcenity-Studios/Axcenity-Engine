//! The bar under the viewport (brief §3.1, "Bottom viewport controls"; the
//! lead's DIS-06): camera presets, the pose and motion (R9: Still, Idle,
//! Walk, Run, gestures; the Expressions page has the same as chips),
//! lighting and backdrop, which were the Stage page. All of it is about
//! looking, not the character: none of it is saved or undone. Screenshot
//! and turntable come later. The bar fits the viewport it is under: its
//! rows of chips fold into menus where there isn't room for them.

use eframe::egui;

use super::CharacterWorkspace;
use super::intent::Intent;
use super::motion::Motion;
use super::stage::{Backdrop, Lighting, ViewPreset};
use crate::theme::{self, icons};
use crate::widgets;

/// The room between the bar's controls.
const GAP: f32 = 4.0;

/// How wide a chip saying `label` is drawn ([`widgets::chip`]).
fn chip_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::FontId::new(theme::type_scale::SMALL + 0.5, theme::medium_family());
    ui.painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER)
        .size()
        .x
        + 16.0
}

/// How wide a row of chips saying `labels` is, with the room after each.
fn chips_width<'a>(ui: &egui::Ui, labels: impl Iterator<Item = &'a str>) -> f32 {
    labels.map(|label| chip_width(ui, label) + GAP).sum()
}

/// How wide a menu of the bar is at its widest: its icon, the longest of
/// `labels` and its caret (so the bar doesn't change shape with what is
/// picked), with the room after it.
fn menu_width<'a>(ui: &egui::Ui, icon: &str, labels: impl Iterator<Item = &'a str>) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widest = labels
        .map(|label| {
            ui.painter()
                .layout_no_wrap(
                    format!("{icon} {label} {}", icons::CARET_DOWN),
                    font.clone(),
                    egui::Color32::PLACEHOLDER,
                )
                .size()
                .x
        })
        .fold(0.0, f32::max);
    widest + 2.0 * ui.spacing().button_padding.x + GAP
}

/// Which of the bar's choices are rows of chips; the others are menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shape {
    views: bool,
    lighting: bool,
    backdrop: bool,
}

/// The most chips that fit `ui`'s width: every choice a chip where there
/// is room for them all; narrower (a wide panel, a small window), first
/// the backdrop is a menu that says what is on, then the lighting, then
/// the views, so nothing is ever drawn over anything else.
fn shape(ui: &egui::Ui) -> Shape {
    let room = ui.available_width();
    let motions = Motion::all();
    let motion = menu_width(
        ui,
        icons::PERSON_SIMPLE_WALK,
        motions.iter().map(|motion| motion.label()),
    );
    // A row of chips after its quiet icon, and the same as a menu.
    let quiet = 22.0;
    let view_chips = chips_width(ui, ViewPreset::ALL.iter().map(|view| view.label()));
    let lighting = || Lighting::ALL.iter().map(|lighting| lighting.label());
    let backdrop = || Backdrop::ALL.iter().map(|backdrop| backdrop.label());
    let lighting_chips = chips_width(ui, lighting()) + quiet + 8.0;
    let backdrop_chips = chips_width(ui, backdrop()) + quiet;
    // (The face preview's Neutral light is no chip, but the menu says it.)
    let lighting_menu = menu_width(ui, icons::SUN, lighting().chain(["Neutral"]));
    let backdrop_menu = menu_width(ui, icons::IMAGE, backdrop());
    let left = view_chips + motion;
    [
        (true, true, true),
        (true, true, false),
        (true, false, false),
    ]
    .into_iter()
    .find(|(_, lighting, backdrop)| {
        let looks = if *lighting {
            lighting_chips
        } else {
            lighting_menu
        } + if *backdrop {
            backdrop_chips
        } else {
            backdrop_menu
        };
        left + looks <= room
    })
    .map_or(
        Shape {
            views: false,
            lighting: false,
            backdrop: false,
        },
        |(views, lighting, backdrop)| Shape {
            views,
            lighting,
            backdrop,
        },
    )
}

/// Draws the bar and returns what it asks for. It fits the viewport it is
/// under ([`shape`]).
pub(super) fn show(ui: &mut egui::Ui, workspace: &CharacterWorkspace) -> Vec<Intent> {
    let palette = theme::palette(ui.ctx());
    let mut intents = Vec::new();
    let quiet = |ui: &mut egui::Ui, icon: &str, tip: &str| {
        ui.label(egui::RichText::new(icon).color(palette.text_muted))
            .on_hover_text(tip);
    };
    let shape = shape(ui);
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        if shape.views {
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
        } else {
            // "View" while the camera is where the person moved it.
            let options: Vec<(ViewPreset, &str)> = ViewPreset::ALL
                .iter()
                .map(|view| (*view, view.label()))
                .collect();
            let label = workspace.view.map_or("View", ViewPreset::label);
            let picked = choice_menu(
                ui,
                &palette,
                icons::VIDEO_CAMERA,
                label,
                "The camera's view",
                &options,
                workspace.view,
            );
            intents.extend(picked.map(Intent::FocusView));
        }
        intents.extend(motion_menu(ui, workspace, &palette));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if shape.backdrop {
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
            } else {
                let options: Vec<(Backdrop, &str)> = Backdrop::ALL
                    .iter()
                    .map(|backdrop| (*backdrop, backdrop.label()))
                    .collect();
                let picked = choice_menu(
                    ui,
                    &palette,
                    icons::IMAGE,
                    workspace.backdrop.label(),
                    "Backdrop (not saved with the character)",
                    &options,
                    Some(workspace.backdrop),
                );
                intents.extend(picked.map(Intent::SetBackdrop));
            }
            if shape.lighting {
                for lighting in Lighting::ALL.iter().rev() {
                    if widgets::chip(ui, lighting.label(), workspace.lighting == *lighting)
                        .on_hover_text("Lighting (not saved with the character)")
                        .clicked()
                    {
                        intents.push(Intent::SetLighting(*lighting));
                    }
                }
                quiet(ui, icons::SUN, "Lighting");
            } else {
                let options: Vec<(Lighting, &str)> = Lighting::ALL
                    .iter()
                    .map(|lighting| (*lighting, lighting.label()))
                    .collect();
                let picked = choice_menu(
                    ui,
                    &palette,
                    icons::SUN,
                    workspace.lighting.label(),
                    "Lighting (not saved with the character)",
                    &options,
                    Some(workspace.lighting),
                );
                intents.extend(picked.map(Intent::SetLighting));
            }
        });
    });
    intents
}

/// A row of chips as one compact menu, where the bar hasn't room for
/// them: `icon`, what is on (`label`) and a caret; the menu lists
/// `options`, the one that is on (`current`) lit. Returns the one picked.
fn choice_menu<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    palette: &theme::Palette,
    icon: &str,
    label: &str,
    tip: &str,
    options: &[(T, &str)],
    current: Option<T>,
) -> Option<T> {
    use egui::containers::menu;
    let mut out = None;
    let button = egui::Button::new(
        egui::RichText::new(format!("{icon} {label} {}", icons::CARET_DOWN))
            .color(palette.text_secondary),
    )
    .frame(false);
    let (response, _) = menu::MenuButton::from_button(button).ui(ui, |ui| {
        for (option, label) in options {
            let on = current == Some(*option);
            if ui.add(egui::Button::selectable(on, *label)).clicked() {
                if !on {
                    out = Some(*option);
                }
                ui.close();
            }
        }
    });
    response.on_hover_text(tip);
    out
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
            "{} {} {}",
            icons::PERSON_SIMPLE_WALK,
            workspace.motion.label(),
            icons::CARET_DOWN
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
