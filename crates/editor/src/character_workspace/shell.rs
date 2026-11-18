//! The studio's frame (brief §3.1): the toolbar across the top, the
//! categories down the left, the viewport with its bottom bar in the
//! middle, the contextual panel on the right and a status line. Each area
//! draws from `&CharacterWorkspace` and returns [`Intent`]s; the shell
//! applies them ([`CharacterWorkspace::apply`]), so only it changes the
//! workspace. It also owns the keyboard shortcuts and the notices.

use std::time::Instant;

use eframe::egui::{self, Rect, Sense, vec2};
use eframe::egui_wgpu::RenderState;

use super::compare::{self, OTHER_AT};
use super::intent::{Gesture, Intent};
use super::stage::{self, ViewPreset};
use super::{Category, CharacterWorkspace, NOTICE_FOR, Saved, bottom_bar, nav, panel, toolbar};
use crate::dialog;
use crate::theme::{self, icons};

/// What the workspace asks of the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceAction {
    /// Back to the start screen (after any Save it asked for).
    Home,
}

const TOOLBAR: f32 = 48.0;
const STATUS: f32 = 26.0;
const BOTTOM_BAR: f32 = 40.0;

impl CharacterWorkspace {
    /// Draws the workspace in `ui`'s whole space.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        render_state: Option<&RenderState>,
    ) -> Option<WorkspaceAction> {
        // A drag always ends with the pointer let go, even when the widget
        // that started it missed the release (a colour picker's popup).
        if self.dragging() && !self.direct.dragging() && !ui.input(|input| input.pointer.any_down())
        {
            self.history.commit(&self.description);
            self.refresh();
        }
        let mut intents = self.shortcuts(ui);
        let full = ui.available_rect_before_wrap();
        let palette = theme::palette(ui.ctx());
        ui.painter().rect_filled(full, 0.0, palette.app_bg);
        let toolbar_rect = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR));
        let status = Rect::from_min_max(egui::pos2(full.left(), full.bottom() - STATUS), full.max);
        let body = Rect::from_min_max(
            egui::pos2(full.left(), toolbar_rect.bottom()),
            egui::pos2(full.right(), status.top()),
        );
        // Narrow windows keep the viewport: the side panels get narrower.
        let left_w = (body.width() * 0.15).clamp(150.0, 200.0);
        let right_w = (body.width() * 0.25).clamp(270.0, 340.0);
        let left = Rect::from_min_size(body.min, vec2(left_w, body.height()));
        let right = Rect::from_min_max(egui::pos2(body.right() - right_w, body.top()), body.max);
        let centre = Rect::from_min_max(
            egui::pos2(left.right(), body.top()),
            egui::pos2(right.left(), body.bottom()),
        );
        let bar = Rect::from_min_max(
            egui::pos2(centre.left(), centre.bottom() - BOTTOM_BAR),
            centre.max,
        );
        let view = Rect::from_min_max(centre.min, egui::pos2(centre.right(), bar.top()));

        // One library for the frame: each call stats the catalogue.
        let library = self.library();
        self.autosave_step(Instant::now());
        if let Some(render_state) = render_state {
            self.pictures
                .set_gpu(&render_state.device, &render_state.queue);
        }
        self.pictures.poll(ui.ctx());
        if self.selection.category == Category::Presets {
            self.load_preset_pictures(ui.ctx(), &library);
        }
        // While a held version is shown in the character's place, editing
        // is off: what's edited is always what's seen.
        let looking = self.comparison.instead().is_some();
        let child =
            |ui: &mut egui::Ui, rect: Rect| ui.new_child(egui::UiBuilder::new().max_rect(rect));
        intents.extend(toolbar::show(
            &mut child(ui, toolbar_rect.shrink2(vec2(10.0, 6.0))),
            self,
            &library,
        ));
        ui.painter().rect_filled(left, 0.0, palette.panel);
        ui.painter().rect_filled(right, 0.0, palette.panel);
        intents.extend(nav::show(&mut child(ui, left.shrink(10.0)), self, &library));
        intents.extend(self.viewport(ui, view.shrink2(vec2(6.0, 4.0)), render_state));
        intents.extend(compare::strip(
            &mut child(
                ui,
                Rect::from_min_size(view.min + vec2(14.0, 10.0), vec2(view.width() - 28.0, 30.0)),
            ),
            self,
        ));
        intents.extend(bottom_bar::show(
            &mut child(ui, bar.shrink2(vec2(10.0, 4.0))),
            self,
        ));
        let mut panel_ui = child(ui, right.shrink(12.0));
        if looking {
            panel_ui.disable();
        }
        intents.extend(panel::show(&mut panel_ui, self, &library));
        self.status(&mut child(ui, status.shrink2(vec2(12.0, 4.0))));
        if self.recovery.is_some() {
            self.offer_recovery(ui.ctx());
        } else if self.asking.is_some() {
            self.ask_to_save(ui.ctx());
        }
        ui.advance_cursor_after_rect(full);

        let mut action = None;
        for intent in intents {
            if let Some(asked) = self.apply(intent) {
                action = Some(asked);
            }
        }
        action
    }

    /// Does what a panel asked. The one place the workspace is changed from
    /// the interface.
    pub fn apply(&mut self, intent: Intent) -> Option<WorkspaceAction> {
        if intent.edits() && self.comparison.instead().is_some() {
            // Back to the character being edited before it changes.
            self.show_held(None);
        }
        match intent {
            Intent::SetParameter { id, value, gesture } => {
                self.set_value(&id, value, false, gesture)
            }
            Intent::SetMirrored { id, value, gesture } => self.set_value(&id, value, true, gesture),
            Intent::Choose { id, value } => {
                let parameter = self
                    .library()
                    .base(&self.description.base)
                    .and_then(|base| base.parameter(&id).cloned());
                if let Some(parameter) = parameter {
                    self.choose(&parameter, value);
                }
            }
            Intent::Wear { slot, item } => self.wear(&slot, item.as_deref()),
            Intent::SetMaterial {
                slot,
                value,
                to,
                gesture,
            } => self.set_material(&slot, &value, to, gesture),
            Intent::SetSkinLayer {
                id,
                marking,
                to,
                gesture,
            } => self.set_skin_layer(&id, marking, to, gesture),
            Intent::SetBase(base) => self.set_base(&base),
            Intent::Rename(name) => self.rename(&name),
            Intent::ApplyPreset(id) => self.use_preset(&id),
            Intent::SaveAsPreset { kind, name } => self.save_as_preset(kind, &name),
            Intent::ToggleFavourite(id) => self.toggle_favourite(&id),
            Intent::New(_) | Intent::Open(_) if self.dirty() => self.asking = Some(intent),
            Intent::New(start) => self.new_character(&start),
            Intent::Open(file) => {
                if let Err(error) = self.open_character(&file) {
                    self.notice = Some(error);
                }
            }
            Intent::Save => self.save_now(),
            Intent::Undo => self.undo(),
            Intent::Redo => self.redo(),
            Intent::CancelEdit => {
                self.cancel_handle_drag();
                self.cancel_edit();
            }
            Intent::Reset(scope) => self.reset_scope(&scope),
            Intent::PickCategory(category) => self.pick_category(category),
            Intent::PickFeature(feature) => self.pick_feature(feature),
            Intent::PickLevel(level) => self.selection.level = level,
            Intent::SetSymmetry(on) => self.symmetry = on,
            Intent::SetHandles(on) => {
                self.direct.on = on;
                if !on {
                    self.direct.forget_hover();
                }
            }
            Intent::FocusFeature(feature) => self.focus_feature(&feature),
            Intent::FocusView(view) => self.focus(view),
            Intent::SetLighting(lighting) => self.lighting = lighting,
            Intent::SetBackdrop(backdrop) => self.backdrop = backdrop,
            Intent::PreviewExpression(name) => self.preview_expression(name),
            Intent::PreviewMotion(motion) => self.preview_motion(motion),
            Intent::PreviewFaceControl { name, value } => self.preview_face_control(&name, value),
            Intent::FacePreview(on) => self.face_preview(on),
            Intent::FacePreviewMotion { blink, idle } => self.face_preview_motion(blink, idle),
            Intent::Sides { tool, of } => self.use_sides(tool, &of),
            Intent::Randomize(scope) => self.randomize(scope),
            Intent::SetLock { ids, locked } => self.set_lock(&ids, locked),
            Intent::Capture => self.capture(),
            Intent::ShowHeld(index) => self.show_held(index),
            Intent::SideBySide(on) => self.set_side_by_side(on),
            Intent::KeepHeld(index) => self.keep_held(index),
            Intent::DiscardHeld(index) => self.discard_held(index),
            Intent::Recover(take) => self.recover(take),
            Intent::Home => return Some(WorkspaceAction::Home),
        }
        None
    }

    /// The answer to "Save changes?" before New or Open: `Some(true)`
    /// saves first, `Some(false)` lets the changes go, `None` cancels.
    pub fn answer(&mut self, save: Option<bool>) {
        let Some(intent) = self.asking.take() else {
            return;
        };
        match save {
            None => {}
            Some(true) => {
                self.save_now();
                if !self.dirty() {
                    self.apply(intent);
                }
            }
            Some(false) => {
                // Marked saved so the intent goes ahead without asking again.
                self.saved = Some(self.description.clone());
                self.apply(intent);
            }
        }
    }

    fn ask_to_save(&mut self, ctx: &egui::Context) {
        let doing = match &self.asking {
            Some(Intent::Open(_)) => "opening another character",
            _ => "starting a new one",
        };
        let message = format!(
            "\"{}\" has unsaved changes. Save them before {doing}?",
            self.description.name
        );
        let (icon, tone) = dialog::unsaved_style();
        let outcome = dialog::ask(
            ctx,
            &dialog::Question {
                id: "character studio save first",
                icon,
                tone,
                title: "Save changes?",
                message: &message,
                items: Vec::new(),
            },
            &[
                dialog::Button::new(icons::X, "Cancel", dialog::Kind::Plain),
                dialog::Button::new(icons::TRASH, "Don't Save", dialog::Kind::Plain),
                dialog::Button::new(icons::FLOPPY_DISK, "Save", dialog::Kind::Main),
            ],
        );
        match outcome {
            Some(dialog::Outcome::Picked(2)) => self.answer(Some(true)),
            Some(dialog::Outcome::Picked(1)) => self.answer(Some(false)),
            Some(_) => self.answer(None),
            None => {}
        }
    }

    /// "Recover the unsaved character?": a crash left one (autosave).
    fn offer_recovery(&mut self, ctx: &egui::Context) {
        let Some(recovery) = &self.recovery else {
            return;
        };
        let message = format!(
            "Character Studio closed with unsaved changes to \"{}\". Recover them? Discard \
             throws them away.",
            recovery.description.name
        );
        let (icon, tone) = dialog::unsaved_style();
        let outcome = dialog::ask(
            ctx,
            &dialog::Question {
                id: "character studio recover",
                icon,
                tone,
                title: "Recover the unsaved character?",
                message: &message,
                items: Vec::new(),
            },
            &[
                dialog::Button::new(icons::TRASH, "Discard", dialog::Kind::Plain),
                dialog::Button::new(
                    icons::ARROW_COUNTER_CLOCKWISE,
                    "Recover",
                    dialog::Kind::Main,
                ),
            ],
        );
        match outcome {
            Some(dialog::Outcome::Picked(1)) => {
                self.apply(Intent::Recover(true));
            }
            Some(dialog::Outcome::Picked(0)) => {
                self.apply(Intent::Recover(false));
            }
            // Closed without an answer: the file stays, offered next time
            // (unless new work is written over it).
            Some(_) => self.recovery = None,
            None => {}
        }
    }

    /// Keyboard shortcuts. Redo is checked before undo: egui ignores an
    /// extra Shift when matching, so Ctrl+Z would take Ctrl+Shift+Z
    /// (audit D-16).
    fn shortcuts(&self, ui: &mut egui::Ui) -> Vec<Intent> {
        // A text field being typed in keeps its own keys.
        if ui.ctx().egui_wants_keyboard_input() {
            return Vec::new();
        }
        let dragging = self.dragging();
        ui.input_mut(|input| {
            let mut intents = Vec::new();
            if input.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            ) || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
            {
                intents.push(Intent::Redo);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z) {
                intents.push(Intent::Undo);
            }
            if input.consume_key(egui::Modifiers::COMMAND, egui::Key::S) {
                intents.push(Intent::Save);
            }
            if dragging && input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                intents.push(Intent::CancelEdit);
            }
            intents
        })
    }

    pub(super) fn save_now(&mut self) {
        self.notice = Some(match self.save_reporting() {
            Ok(Saved::Written(path)) => format!("Saved {}", path.display()),
            Ok(Saved::Renamed { path, from }) => format!(
                "Another character is called \"{from}\", so this one was saved as \"{}\" ({})",
                self.description.name,
                path.display()
            ),
            Err(error) => error,
        });
    }

    /// Shows `category`. The Face and Expressions pages frame the face;
    /// leaving them for another page frames the whole character again
    /// (audit D-10). A camera the person moved is left where it is.
    pub(super) fn pick_category(&mut self, category: Category) {
        if category == self.selection.category {
            return;
        }
        let was = self.selection.category;
        if was == Category::Expressions {
            // An expression, the controls held and the face preview are
            // the Expressions page's only (the face preview first puts
            // back the view it found).
            self.leave_expressions();
        }
        if category.frames_the_face() && self.view != Some(ViewPreset::Face) {
            self.focus(ViewPreset::Face);
        } else if was.frames_the_face()
            && !category.frames_the_face()
            && self.view == Some(ViewPreset::Face)
        {
            self.focus(ViewPreset::FullBody);
        }
        self.selection.pick_category(category);
    }

    /// The preview: editing on the character (hover, click to select,
    /// handles: `direct.rs`), camera input (turn, pan, zoom; a double click
    /// goes back to the whole character) and the rendered character.
    fn viewport(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        render_state: Option<&RenderState>,
    ) -> Vec<Intent> {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let looking = self.comparison.instead().is_some();
        // A primary drag that starts on a handle edits; any other turns
        // the camera (ux patterns section 3).
        let intents = self.direct_input(ui, &response, looking);
        let mut moved = false;
        if response.dragged_by(egui::PointerButton::Primary) && !self.direct.dragging() {
            let delta = response.drag_delta();
            self.orbit.turn(delta.x, delta.y);
            moved |= delta != egui::Vec2::ZERO;
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            let delta = response.drag_delta();
            self.orbit.pan(delta.x, delta.y);
            moved |= delta != egui::Vec2::ZERO;
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.orbit.zoom(scroll);
                moved = true;
            }
        }
        if moved {
            // The view chips no longer say where the camera is.
            self.view = None;
            self.direct.stop_focus();
        }
        if response.double_clicked() {
            self.direct.stop_focus();
            self.focus(ViewPreset::FullBody);
        }
        self.direct.step_focus(&mut self.orbit, Instant::now());
        self.step();
        stage::apply(&mut self.world, self.lighting, self.backdrop);
        let palette = theme::palette(ui.ctx());
        // Side by side: the held version on the left, the current one on
        // the right, one camera moved over to each, the same light.
        let beside = self.comparison.beside();
        let (left, right) = match beside {
            Some(_) => {
                let middle = rect.center().x;
                (
                    Some(Rect::from_min_max(
                        rect.min,
                        egui::pos2(middle - 2.0, rect.max.y),
                    )),
                    Rect::from_min_max(egui::pos2(middle + 2.0, rect.min.y), rect.max),
                )
            }
            None => (None, rect),
        };
        match render_state {
            Some(render_state) => {
                let camera = self.orbit.camera();
                let list = self.draw_list(camera);
                self.canvas.paint(ui, right, render_state, &list);
                if let Some(left) = left {
                    let mut moved = camera;
                    moved.position += OTHER_AT;
                    moved.look_at += OTHER_AT;
                    let list = self.draw_list(moved);
                    self.other_canvas.paint(ui, left, render_state, &list);
                }
                ui.ctx().request_repaint();
            }
            None => {
                for rect in left.into_iter().chain([right]) {
                    ui.painter_at(rect)
                        .rect_filled(rect, theme::radius::CARD, palette.inset);
                }
            }
        }
        // Over the current character: the feature under the pointer, the
        // selected one and its handles.
        self.direct_paint(ui, right, looking);
        // Say what is shown, so a held version is never taken for the
        // character being edited. Each label sits on a small card of its
        // own: the backdrop is any colour the person picks.
        let label = |rect: Rect, text: String| {
            let painter = ui.painter_at(rect);
            let galley =
                painter.layout_no_wrap(text, egui::FontId::proportional(14.0), palette.text_strong);
            let at = egui::pos2(rect.left() + 22.0, rect.bottom() - 16.0 - galley.size().y);
            let card = Rect::from_min_size(at, galley.size()).expand2(vec2(8.0, 4.0));
            painter.rect_filled(card, theme::radius::CARD, compare::backing(&palette));
            painter.galley(at, galley, palette.text_strong);
        };
        if let (Some(index), Some(left)) = (beside, left) {
            label(
                left,
                format!("{} (held)", compare::Comparison::letter(index)),
            );
            label(right, "Current".to_owned());
        } else if let Some(index) = self.comparison.instead() {
            label(
                right,
                format!(
                    "Showing {} (held): editing is off until you go back to Current",
                    compare::Comparison::letter(index)
                ),
            );
        }
        intents
    }

    /// The preview World's draw list from `camera`, on the stage.
    fn draw_list(&self, camera: axcenity_render::Camera) -> axcenity_render::DrawList {
        let mut list = axcenity_scene::build_draw_list_posed(
            &self.world,
            &self.assets,
            camera,
            self.poses(),
            &mut axcenity_scene::TerrainCache::new(),
        );
        stage::backdrop(&mut list, self.backdrop);
        list
    }

    /// Clears a notice that has been shown long enough (audit D-13).
    pub(super) fn expire_notice(&mut self, now: Instant) {
        let Some(text) = &self.notice else {
            self.notice_shown = None;
            return;
        };
        match &self.notice_shown {
            Some((shown, since)) if shown == text => {
                if now.duration_since(*since) >= NOTICE_FOR {
                    self.notice = None;
                    self.notice_shown = None;
                }
            }
            _ => self.notice_shown = Some((text.clone(), now)),
        }
    }

    fn status(&mut self, ui: &mut egui::Ui) {
        self.expire_notice(Instant::now());
        let palette = theme::palette(ui.ctx());
        ui.horizontal_centered(|ui| {
            let text = self
                .notice
                .clone()
                .or_else(|| self.direct.status.clone())
                .unwrap_or_else(|| {
                    "Click a feature to select it · drag a handle to change it · drag elsewhere \
                     to turn · right-drag to pan · scroll to zoom"
                        .to_owned()
                });
            ui.label(egui::RichText::new(text).small().color(palette.text_weak));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Saved in {}",
                        self.folder().join(super::CHARACTERS_FOLDER).display()
                    ))
                    .small()
                    .color(palette.text_muted),
                );
            });
        });
    }
}

/// A slider's gesture from its response: a drag going on, or done.
pub(super) fn gesture(response: &egui::Response) -> Gesture {
    if response.dragged() {
        Gesture::Drag
    } else {
        Gesture::Commit
    }
}
