//! Drawing the workspace: a toolbar across the top (Home, the name, undo
//! and redo, Reset, Save), the categories down the left, the viewport in
//! the middle and the category's controls on the right. The Body, Face,
//! Hair and Clothing pages are drawn from the base model's manifest (its
//! groups, parameters and slots) and the catalogue (CO12).

use eframe::egui::{self, Rect, Sense, vec2};
use eframe::egui_wgpu::RenderState;

use super::stage::{self, Backdrop, Lighting, ViewPreset};
use super::{Category, CharacterWorkspace, presets, symmetric_label};
use crate::theme::{self, icons};
use crate::widgets::{self, Role};

/// What the workspace asks of the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceAction {
    /// Back to the start screen (after any Save it asked for).
    Home,
}

const TOOLBAR: f32 = 48.0;
const STATUS: f32 = 26.0;

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Presets => "Presets",
            Category::Body => "Body",
            Category::Face => "Face",
            Category::Hair => "Hair",
            Category::Clothing => "Clothing",
            Category::Stage => "Stage",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Category::Presets => icons::SPARKLE,
            Category::Body => icons::PERSON,
            Category::Face => icons::SMILEY,
            Category::Hair => icons::SCISSORS,
            Category::Clothing => icons::T_SHIRT,
            Category::Stage => icons::LIGHTBULB,
        }
    }

    fn about(self) -> &'static str {
        match self {
            Category::Presets => "Start from a ready-made character, then change anything.",
            Category::Body => "Pick male or female, then shape the body. Changes show as you drag.",
            Category::Face => "Shape the face. Changes show on the character as you drag.",
            Category::Hair => "Pick a hairstyle.",
            Category::Clothing => "Dress the character one piece at a time.",
            Category::Stage => {
                "Light and backdrop for looking at the character. Not saved with it."
            }
        }
    }
}

impl CharacterWorkspace {
    /// Draws the workspace in `ui`'s whole space.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        render_state: Option<&RenderState>,
    ) -> Option<WorkspaceAction> {
        let mut action = None;
        self.shortcuts(ui);
        let full = ui.available_rect_before_wrap();
        let palette = theme::palette(ui.ctx());
        ui.painter().rect_filled(full, 0.0, palette.app_bg);
        let toolbar = Rect::from_min_size(full.min, vec2(full.width(), TOOLBAR));
        let status = Rect::from_min_max(egui::pos2(full.left(), full.bottom() - STATUS), full.max);
        let body = Rect::from_min_max(
            egui::pos2(full.left(), toolbar.bottom()),
            egui::pos2(full.right(), status.top()),
        );
        // Narrow windows keep the viewport: the side panels get narrower.
        let left_w = (body.width() * 0.16).clamp(150.0, 210.0);
        let right_w = (body.width() * 0.26).clamp(260.0, 340.0);
        let left = Rect::from_min_size(body.min, vec2(left_w, body.height()));
        let right = Rect::from_min_max(egui::pos2(body.right() - right_w, body.top()), body.max);
        let view = Rect::from_min_max(
            egui::pos2(left.right(), body.top()),
            egui::pos2(right.left(), body.bottom()),
        );

        if self.toolbar(
            &mut ui.new_child(egui::UiBuilder::new().max_rect(toolbar.shrink2(vec2(10.0, 6.0)))),
        ) {
            action = Some(WorkspaceAction::Home);
        }
        ui.painter().rect_filled(left, 0.0, palette.panel);
        ui.painter().rect_filled(right, 0.0, palette.panel);
        self.categories(&mut ui.new_child(egui::UiBuilder::new().max_rect(left.shrink(10.0))));
        self.viewport(ui, view.shrink(6.0), render_state);
        self.properties(&mut ui.new_child(egui::UiBuilder::new().max_rect(right.shrink(12.0))));
        self.status(
            &mut ui.new_child(egui::UiBuilder::new().max_rect(status.shrink2(vec2(12.0, 4.0)))),
        );
        ui.advance_cursor_after_rect(full);
        action
    }

    fn shortcuts(&mut self, ui: &mut egui::Ui) {
        let (undo, redo, save) = ui.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
                    || input.consume_key(
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                        egui::Key::Z,
                    ),
                input.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
            )
        });
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if save {
            self.save_now();
        }
    }

    fn save_now(&mut self) {
        self.notice = Some(match self.save() {
            Ok(path) => format!("Saved {}", path.display()),
            Err(error) => error,
        });
    }

    /// Returns true when Home was clicked.
    fn toolbar(&mut self, ui: &mut egui::Ui) -> bool {
        let palette = theme::palette(ui.ctx());
        let mut home = false;
        ui.horizontal_centered(|ui| {
            if widgets::icon_button(ui, icons::HOUSE, "Back to the start screen", false).clicked() {
                home = true;
            }
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(theme::with_icon(icons::USER_FOCUS, "Character Studio"))
                    .family(theme::medium_family())
                    .color(palette.accent),
            );
            ui.add_space(8.0);
            let mut name = self.description().name.clone();
            let field = ui.add(
                egui::TextEdit::singleline(&mut name)
                    .desired_width(180.0)
                    .hint_text("Character name"),
            );
            if field.changed() {
                self.drag(|look| look.name = name.clone(), false);
            }
            if field.lost_focus() {
                self.drag(|_| {}, true);
            }
            if self.dirty() {
                ui.label(
                    egui::RichText::new("Not saved")
                        .color(palette.text_weak)
                        .small(),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(widgets::button(
                        Role::Primary,
                        Some(icons::FLOPPY_DISK),
                        "Save",
                        &palette,
                    ))
                    .on_hover_text("Save the character (Ctrl+S)")
                    .clicked()
                {
                    self.save_now();
                }
                if ui
                    .add(widgets::button(
                        Role::Secondary,
                        Some(icons::ARROW_COUNTER_CLOCKWISE),
                        "Reset",
                        &palette,
                    ))
                    .on_hover_text("Back to the default character (you can undo it)")
                    .clicked()
                {
                    self.reset();
                }
                ui.add_space(8.0);
                if ui
                    .add_enabled(
                        self.can_redo(),
                        egui::Button::new(icons::ARROW_CLOCKWISE).frame(false),
                    )
                    .on_hover_text("Redo (Ctrl+Y)")
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .add_enabled(
                        self.can_undo(),
                        egui::Button::new(icons::ARROW_COUNTER_CLOCKWISE).frame(false),
                    )
                    .on_hover_text("Undo (Ctrl+Z)")
                    .clicked()
                {
                    self.undo();
                }
            });
        });
        home
    }

    fn categories(&mut self, ui: &mut egui::Ui) {
        widgets::section_label(ui, "Customize");
        for category in Category::ALL {
            if widgets::nav_item(
                ui,
                category.icon(),
                category.label(),
                None,
                self.category == category,
            )
            .clicked()
            {
                self.pick_category(category);
            }
        }
    }

    /// Shows `category`: the Face page frames the face, the Body page the
    /// whole character.
    pub(super) fn pick_category(&mut self, category: Category) {
        if category == self.category {
            return;
        }
        match category {
            Category::Face => self.orbit = ViewPreset::Face.orbit(),
            Category::Body if self.category == Category::Face => {
                self.orbit = ViewPreset::FullBody.orbit()
            }
            _ => {}
        }
        self.category = category;
    }

    fn viewport(&mut self, ui: &mut egui::Ui, rect: Rect, render_state: Option<&RenderState>) {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.orbit.turn(delta.x, delta.y);
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            let delta = response.drag_delta();
            self.orbit.pan(delta.x, delta.y);
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.orbit.zoom(scroll);
            }
        }
        if response.double_clicked() {
            self.orbit = ViewPreset::FullBody.orbit();
        }
        self.step();
        stage::apply(&mut self.world, self.lighting, self.backdrop);
        let painter = ui.painter_at(rect);
        let palette = theme::palette(ui.ctx());
        match render_state {
            Some(render_state) => {
                let mut list = axcenity_scene::build_draw_list_posed(
                    &self.world,
                    &self.assets,
                    self.orbit.camera(),
                    self.poses(),
                    &mut axcenity_scene::TerrainCache::new(),
                );
                stage::backdrop(&mut list, self.backdrop);
                self.canvas.paint(ui, rect, render_state, &list);
                ui.ctx().request_repaint();
            }
            None => {
                painter.rect_filled(rect, theme::radius::CARD, palette.inset);
            }
        }
        // The view buttons over the top of the viewport.
        let bar = Rect::from_min_size(rect.min + vec2(10.0, 10.0), vec2(rect.width() - 20.0, 32.0));
        let mut bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(bar));
        egui::Frame::new()
            .fill(palette.panel.gamma_multiply(0.92))
            .corner_radius(theme::radius::CARD)
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(&mut bar_ui, |ui| {
                ui.horizontal(|ui| {
                    for preset in ViewPreset::ALL {
                        if widgets::chip(ui, preset.label(), false)
                            .on_hover_text(format!("{} view", preset.label()))
                            .clicked()
                        {
                            self.orbit = preset.orbit();
                        }
                    }
                });
            });
        painter.text(
            rect.left_bottom() + vec2(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag to turn · right-drag to pan · scroll to zoom · double-click to reset",
            egui::FontId::proportional(12.0),
            palette.text_weak,
        );
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        ui.label(
            egui::RichText::new(self.category.label())
                .family(theme::display_family())
                .size(20.0)
                .color(palette.text_strong),
        );
        ui.label(egui::RichText::new(self.category.about()).color(palette.text_weak));
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("character workspace properties")
            .auto_shrink(false)
            .show(ui, |ui| match self.category {
                Category::Presets => self.presets_page(ui),
                Category::Body => {
                    self.base_picker(ui);
                    self.parameters_page(ui, Category::Body);
                }
                Category::Face => self.parameters_page(ui, Category::Face),
                Category::Hair | Category::Clothing => self.slots_page(ui, self.category),
                Category::Stage => self.stage_page(ui),
            });
    }

    fn presets_page(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        let library = self.library();
        for preset in presets() {
            if library.base(preset.base).is_none() {
                continue;
            }
            let selected = self.is_preset(&preset);
            let frame = egui::Frame::new()
                .fill(if selected {
                    palette.accent_soft
                } else {
                    palette.surface
                })
                .corner_radius(theme::radius::CARD)
                .inner_margin(10.0);
            let response = frame
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        egui::RichText::new(preset.name)
                            .family(theme::medium_family())
                            .color(palette.text_strong),
                    );
                    ui.label(
                        egui::RichText::new(preset.about)
                            .color(palette.text_weak)
                            .small(),
                    );
                })
                .response
                .interact(Sense::click());
            if response.clicked() {
                self.use_preset(&preset);
            }
            ui.add_space(6.0);
        }
    }

    /// The body families Character Studio offers: one chip each, shown
    /// only when there's a choice (or the character is on a body no longer
    /// offered, such as a saved stylized one, so it can move).
    fn base_picker(&mut self, ui: &mut egui::Ui) {
        let library = self.library();
        let current = self.description().base.clone();
        let offered: Vec<(&String, &axcenity_avatar::BaseManifest)> = library
            .bases
            .iter()
            .filter(|(id, _)| super::offers_base(&library, id))
            .collect();
        if offered.len() < 2 && offered.iter().any(|(id, _)| **id == current) {
            return;
        }
        widgets::section_label(ui, "Body");
        ui.horizontal_wrapped(|ui| {
            for (id, base) in offered {
                if widgets::chip(ui, &base.label, *id == current)
                    .on_hover_text(format!(
                        "Start again on the {} body",
                        base.label.to_lowercase()
                    ))
                    .clicked()
                {
                    self.set_base(id);
                }
            }
        });
        ui.add_space(8.0);
    }

    /// The manifest's parameters of `category`, by group: a slider each,
    /// a mirrored pair as one slider while Symmetry is on.
    fn parameters_page(&mut self, ui: &mut egui::Ui, category: Category) {
        let Some(base) = self.base() else {
            ui.label("This body isn't in the library.");
            return;
        };
        let data = category.data();
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.symmetry, "Symmetry")
                .on_hover_text("Move a left and right pair together");
            ui.checkbox(&mut self.advanced, "Advanced")
                .on_hover_text("Show every slider the body has");
        });
        ui.add_space(4.0);
        for group in base
            .groups
            .iter()
            .filter(|group| Some(group.category) == data)
        {
            let shown: Vec<&axcenity_avatar::Parameter> = base
                .parameters
                .iter()
                .filter(|parameter| parameter.group == group.id)
                .filter(|parameter| self.advanced || !parameter.advanced)
                .filter(|parameter| {
                    // With symmetry, a pair shows once: as the first of the two.
                    let Some(mirror) = parameter.mirror.as_deref().filter(|_| self.symmetry) else {
                        return true;
                    };
                    let index = |id: &str| base.parameters.iter().position(|p| p.id == id);
                    index(&parameter.id) < index(mirror)
                })
                .collect();
            if shown.is_empty() {
                continue;
            }
            egui::CollapsingHeader::new(&group.label)
                .id_salt(("character parameters", &group.id))
                .default_open(true)
                .show(ui, |ui| {
                    for parameter in shown {
                        if parameter.choices.is_empty() {
                            self.slider(ui, parameter);
                        } else {
                            self.choices(ui, parameter);
                        }
                    }
                });
        }
    }

    fn slider(&mut self, ui: &mut egui::Ui, parameter: &axcenity_avatar::Parameter) {
        let label = if self.symmetry && parameter.mirror.is_some() {
            symmetric_label(&parameter.label)
        } else {
            parameter.label.clone()
        };
        let [low, high] = parameter.range;
        let mut value = self.parameter(&parameter.id);
        ui.label(&label);
        let response = ui.add(
            egui::Slider::new(&mut value, low.min(high)..=low.max(high))
                .fixed_decimals(2)
                .clamping(egui::SliderClamping::Always),
        );
        if response.double_clicked() {
            value = parameter.default;
        }
        if response.changed() || response.drag_stopped() || response.double_clicked() {
            self.set_parameter(parameter, value, !response.dragged());
        }
    }

    /// A parameter with choices (Male or Female): a chip each.
    fn choices(&mut self, ui: &mut egui::Ui, parameter: &axcenity_avatar::Parameter) {
        let value = self.parameter(&parameter.id);
        let picked = parameter.nearest_choice(value).map(|choice| choice.value);
        ui.label(&parameter.label);
        ui.horizontal_wrapped(|ui| {
            for choice in &parameter.choices {
                if widgets::chip(ui, &choice.label, picked == Some(choice.value)).clicked() {
                    self.choose(parameter, choice.value);
                }
            }
        });
        ui.add_space(4.0);
    }

    /// The manifest's slots of `category`: a chip per item that fits this
    /// body, and one for wearing nothing.
    fn slots_page(&mut self, ui: &mut egui::Ui, category: Category) {
        let Some(base) = self.base() else {
            return;
        };
        let data = category.data();
        let mut any = false;
        for slot in base.slots.iter().filter(|slot| Some(slot.category) == data) {
            let items = self.items_for(&slot.id);
            let worn = self.worn_in(&slot.id);
            if items.is_empty() && worn.is_none() {
                continue;
            }
            any = true;
            widgets::section_label(ui, &slot.label);
            ui.horizontal_wrapped(|ui| {
                let none = slot.empty_label.as_deref().unwrap_or("None");
                if widgets::chip(ui, none, worn.is_none()).clicked() {
                    self.wear(&slot.id, None);
                }
                for (id, label) in &items {
                    if widgets::chip(ui, label, worn.as_deref() == Some(id.as_str())).clicked() {
                        self.wear(&slot.id, Some(id));
                    }
                }
            });
            ui.add_space(10.0);
        }
        if !any {
            let palette = theme::palette(ui.ctx());
            ui.label(
                egui::RichText::new(format!(
                    "Nothing in the library fits the {} body here yet.",
                    base.label.to_lowercase()
                ))
                .color(palette.text_weak),
            );
        }
    }

    fn stage_page(&mut self, ui: &mut egui::Ui) {
        widgets::section_label(ui, "Lighting");
        ui.horizontal_wrapped(|ui| {
            for lighting in Lighting::ALL {
                if widgets::chip(ui, lighting.label(), self.lighting == lighting).clicked() {
                    self.lighting = lighting;
                }
            }
        });
        ui.add_space(8.0);
        widgets::section_label(ui, "Backdrop");
        ui.horizontal_wrapped(|ui| {
            for backdrop in Backdrop::ALL {
                if widgets::chip(ui, backdrop.label(), self.backdrop == backdrop).clicked() {
                    self.backdrop = backdrop;
                }
            }
        });
        ui.add_space(8.0);
        widgets::section_label(ui, "View");
        ui.horizontal_wrapped(|ui| {
            for preset in ViewPreset::ALL {
                if widgets::chip(ui, preset.label(), false).clicked() {
                    self.orbit = preset.orbit();
                }
            }
        });
    }

    fn status(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        ui.horizontal_centered(|ui| {
            let text = self.notice.clone().unwrap_or_else(|| {
                format!(
                    "Characters are saved in {}",
                    self.folder().join(super::CHARACTERS_FOLDER).display()
                )
            });
            ui.label(egui::RichText::new(text).small().color(palette.text_weak));
        });
    }
}
