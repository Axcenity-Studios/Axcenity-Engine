use super::*;

impl AnimationStudio {
    /// The open state machine: its toolbar, graph, inspector and live
    /// preview. Returns where double-clicking a state went (its clip).
    #[allow(clippy::too_many_arguments)] // Each is a different input to the studio.
    pub(super) fn machine_body(
        &mut self,
        ui: &mut egui::Ui,
        project: &Path,
        assets: &Assets,
        render_state: Option<&RenderState>,
        world: &mut World,
        output: &mut Output,
        actions: &mut Vec<StudioAction>,
    ) -> Option<Go> {
        let stacked = ui.available_width() < STACK_BELOW;
        let pick = egui::Panel::top("state machine toolbar")
            .show(ui, |ui| self.controller_toolbar(ui, stacked))
            .inner;
        if let Some(pick) = pick {
            self.act(pick, project, assets, world, output, actions);
        }

        let dt = f64::from(ui.input(|input| input.unstable_dt)).min(MAX_FRAME_SECONDS);
        let mut go = None;
        if stacked {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("state machine stacked")
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        let size = egui::vec2(ui.available_width(), STACKED_GRAPH);
                        ui.allocate_ui(size, |ui| go = self.controller_graph(ui));
                        ui.add_space(6.0);
                        self.controller_live(ui, assets, render_state, dt);
                        ui.separator();
                        egui::CollapsingHeader::new(format!("{} Settings", icons::SLIDERS))
                            .id_salt("state machine inspector stacked")
                            .default_open(true)
                            .icon(widgets::chevron)
                            .show(ui, |ui| self.controller_inspector(ui));
                    });
            });
        } else {
            if !self.machine.hide_inspector {
                let room = ui.available_width();
                egui::Panel::right("state machine inspector")
                    .resizable(true)
                    .default_size(320.0)
                    .max_size(room * 0.5)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("state machine inspector scroll")
                            .show(ui, |ui| self.controller_inspector(ui))
                    });
            }
            let mut graph_rect = egui::Rect::NOTHING;
            egui::CentralPanel::default().show(ui, |ui| {
                graph_rect = ui.max_rect();
                go = self.controller_graph(ui);
            });
            // Dock the 3D Preview cleanly in the bottom-left corner of the graph,
            // or undock it into a floating window.
            self.corner_preview(ui.ctx(), graph_rect, assets, render_state, dt);
        }
        if self.machine.doc.as_ref().is_some_and(|doc| doc.playing) {
            ui.ctx().request_repaint();
        }
        go
    }

    /// What's open's preview model and playback, the button that shows or
    /// hides the inspector, and the ⋯ menu. Returns what the ⋯ menu asked for.
    pub(super) fn controller_toolbar(&mut self, ui: &mut egui::Ui, stacked: bool) -> Option<Pick> {
        let open = self.controller_path();
        responsive::toolbar(ui, |ui| {
            let mut pick = None;
            let Some(doc) = &mut self.machine.doc else {
                return pick;
            };
            let shown = doc
                .preview_model
                .as_deref()
                .map_or("(pick a model)", file_name);
            let mut picked = doc.preview_model.clone();
            responsive::room_for(ui, MODEL_PICKER_ROOM);
            egui::ComboBox::from_id_salt("state machine preview model")
                .selected_text(format!("{} Preview on {shown}", icons::CUBE))
                .width(160.0)
                .show_ui(ui, |ui| {
                    for model in &self.files.models {
                        ui.selectable_value(&mut picked, Some(model.clone()), file_name(model));
                    }
                })
                .response
                .on_hover_text("The model the live preview shows");
            if picked != doc.preview_model {
                doc.preview_model = picked;
                doc.restart();
            }
            let (icon, hover) = if doc.playing {
                (icons::PAUSE, "Pause the preview")
            } else {
                (icons::PLAY, "Play the preview")
            };
            if ui.button(icon).on_hover_text(hover).clicked() {
                doc.playing = !doc.playing;
            }
            if ui
                .button(icons::ARROW_COUNTER_CLOCKWISE)
                .on_hover_text("Start again from the default state")
                .clicked()
            {
                doc.restart();
            }
            ui.separator();
            let (wires_icon, wires_hover) = if doc.view.show_all_wires {
                (
                    icons::EYE,
                    "Showing all transition wires (click to show only active/hovered)",
                )
            } else {
                (
                    icons::EYE_SLASH,
                    "Showing only active/hovered node wires (click to show all)",
                )
            };
            if widgets::icon_button(ui, wires_icon, wires_hover, doc.view.show_all_wires).clicked()
            {
                doc.view.show_all_wires = !doc.view.show_all_wires;
            }
            if !stacked
                && widgets::icon_button(
                    ui,
                    icons::SLIDERS,
                    "Show or hide the inspector",
                    !self.machine.hide_inspector,
                )
                .clicked()
            {
                self.machine.hide_inspector = !self.machine.hide_inspector;
            }
            if let Some(path) = &open {
                ui.menu_button(icons::DOTS_THREE, |ui| asset_menu(ui, path, &mut pick));
            }
            pick
        })
    }

    /// The open state machine's graph, filling `ui`. Returns where
    /// double-clicking a state went: the clip it plays (a blend tree's
    /// first).
    pub(super) fn controller_graph(&mut self, ui: &mut egui::Ui) -> Option<Go> {
        let controller = self.machine.doc.as_mut()?;
        let size = ui.available_size();
        let open = ui
            .allocate_ui(size, |ui| controller.graph_ui(ui, &self.files.clips))
            .inner?;
        let state = controller.file.states.get(open)?;
        let clip = motion_clips(&state.motion).into_iter().next()?;
        Some(Go::State(state.name.clone(), clip))
    }

    /// The 3D live preview docked in the bottom-left corner of the graph, or
    /// undocked into a floating window.
    fn corner_preview(
        &mut self,
        ctx: &egui::Context,
        graph_rect: egui::Rect,
        assets: &Assets,
        render_state: Option<&RenderState>,
        dt: f64,
    ) {
        let Some(controller) = &mut self.machine.doc else {
            return;
        };
        if graph_rect.width() < 120.0 || graph_rect.height() < 120.0 {
            return;
        }
        let preview = &mut self.machine.preview;
        let palette = theme::palette(ctx);
        let playing_label = controller.live_state_name().unwrap_or("Preview").to_owned();

        if self.machine.preview_undocked {
            let mut open = true;
            crate::widgets::tool_window(icons::CUBE, &format!("3D Preview — {playing_label}"))
                .id("state machine preview undocked")
                .size(egui::vec2(280.0, 260.0))
                .show(ctx, &mut open, |ui| {
                    show_model(preview, controller.preview_model.as_deref(), assets);
                    let posed = match controller.tick(assets, dt) {
                        Ok(posed) => Some(posed),
                        Err(problem) => {
                            ui.colored_label(ui.visuals().warn_fg_color, problem);
                            None
                        }
                    };
                    let h = ui.available_height().max(120.0);
                    preview.show(ui, render_state, assets, posed, h);
                });
            if !open {
                self.machine.preview_undocked = false;
            }
            return;
        }

        let card_w = (graph_rect.width() * 0.30).clamp(180.0, 260.0);
        let card_h = if self.machine.preview_collapsed {
            28.0
        } else {
            (graph_rect.height() * 0.36).clamp(150.0, 220.0)
        };
        let pos = egui::pos2(
            graph_rect.left() + 10.0,
            graph_rect.bottom() - card_h - 10.0,
        );
        egui::Area::new(egui::Id::new("state machine corner preview"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.panel.gamma_multiply(0.95))
                    .stroke(egui::Stroke::new(1.0_f32, palette.border))
                    .corner_radius(theme::radius::CARD)
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.set_width(card_w);
                        ui.horizontal(|ui| {
                            let chev = if self.machine.preview_collapsed {
                                icons::CARET_RIGHT
                            } else {
                                icons::CARET_DOWN
                            };
                            if ui
                                .small_button(chev)
                                .on_hover_text("Collapse or expand 3D preview")
                                .clicked()
                            {
                                self.machine.preview_collapsed = !self.machine.preview_collapsed;
                            }
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!(
                                        "{} {playing_label}",
                                        icons::PLAY_CIRCLE
                                    ))
                                    .size(theme::type_scale::SMALL)
                                    .strong(),
                                )
                                .truncate(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .small_button(icons::ARROW_SQUARE_OUT)
                                        .on_hover_text("Undock 3D Preview to a floating window")
                                        .clicked()
                                    {
                                        self.machine.preview_undocked = true;
                                    }
                                },
                            );
                        });
                        if !self.machine.preview_collapsed {
                            ui.add_space(4.0);
                            show_model(preview, controller.preview_model.as_deref(), assets);
                            let posed = match controller.tick(assets, dt) {
                                Ok(posed) => Some(posed),
                                Err(problem) => {
                                    ui.colored_label(ui.visuals().warn_fg_color, problem);
                                    None
                                }
                            };
                            preview.show(
                                ui,
                                render_state,
                                assets,
                                posed,
                                (card_h - 36.0).max(80.0),
                            );
                        }
                    });
            });
    }

    /// The live preview (used in stacked narrow layout).
    pub(super) fn controller_live(
        &mut self,
        ui: &mut egui::Ui,
        assets: &Assets,
        render_state: Option<&RenderState>,
        dt: f64,
    ) {
        let Some(controller) = &mut self.machine.doc else {
            return;
        };
        let preview = &mut self.machine.preview;
        let height = ui.available_height().min(CONTROLLER_PREVIEW);
        let size = egui::vec2(ui.available_width(), height);
        ui.allocate_ui(size, |ui| {
            show_model(preview, controller.preview_model.as_deref(), assets);
            let posed = match controller.tick(assets, dt) {
                Ok(posed) => Some(posed),
                Err(problem) => {
                    ui.colored_label(ui.visuals().warn_fg_color, problem);
                    None
                }
            };
            let height = ui.available_height().clamp(80.0, CONTROLLER_PREVIEW);
            preview.show(ui, render_state, assets, posed, height);
        });
    }

    pub(super) fn controller_inspector(&mut self, ui: &mut egui::Ui) {
        if let Some(controller) = &mut self.machine.doc {
            controller.inspector(ui, &self.files.clips);
        }
    }
}
