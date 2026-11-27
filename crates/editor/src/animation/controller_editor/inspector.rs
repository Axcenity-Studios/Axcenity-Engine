use super::*;
use crate::animation::graph::words;

/// A combo box picking one of the project's `clips` (or none) for `current`.
pub(super) fn clip_picker(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    current: &mut String,
    clips: &[String],
) {
    let shown = if current.is_empty() {
        "(none)".to_owned()
    } else {
        axcenity_anim::files::clip_name(current).to_owned()
    };
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            ui.selectable_value(current, String::new(), "(none)");
            for clip in clips {
                let label = format!(
                    "{}  ({})",
                    axcenity_anim::files::clip_name(clip),
                    super::super::store::group_of(clip)
                );
                ui.selectable_value(current, clip.clone(), label);
            }
        });
}

/// The clip in `clips` whose name has `word` in it, preferring ones in
/// `folder` (a clip's group).
pub(super) fn clip_named(clips: &[String], word: &str, folder: Option<&str>) -> Option<String> {
    let matching: Vec<&String> = clips
        .iter()
        .filter(|clip| {
            axcenity_anim::files::clip_name(clip)
                .to_lowercase()
                .contains(word)
        })
        .collect();
    matching
        .iter()
        .find(|clip| folder.is_some_and(|folder| super::super::store::group_of(clip) == folder))
        .or(matching.first())
        .map(|clip| (*clip).clone())
}

/// The blend tree's thresholds on a line, each clip's name over its tick,
/// with the parameter's value now as a marker.
pub(super) fn threshold_line(
    ui: &mut egui::Ui,
    tree: &BlendTree,
    value: Option<f64>,
    low: f64,
    high: f64,
) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    let line_y = rect.bottom() - 10.0;
    let x_of = |v: f64| {
        let t = ((v - low) / (high - low).max(1e-9)).clamp(0.0, 1.0) as f32;
        egui::lerp(rect.left() + 8.0..=rect.right() - 8.0, t)
    };
    painter.line_segment(
        [
            egui::pos2(rect.left() + 8.0, line_y),
            egui::pos2(rect.right() - 8.0, line_y),
        ],
        egui::Stroke::new(2.0_f32, visuals.weak_text_color()),
    );
    for child in &tree.children {
        let x = x_of(child.threshold);
        painter.circle_filled(egui::pos2(x, line_y), 4.0, visuals.selection.bg_fill);
        let name = if child.clip.is_empty() {
            "?".to_owned()
        } else {
            axcenity_anim::files::clip_name(&child.clip).to_owned()
        };
        painter.text(
            egui::pos2(x, line_y - 8.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{name} {}", child.threshold),
            egui::FontId::proportional(10.0),
            visuals.text_color(),
        );
    }
    if let Some(value) = value {
        let x = x_of(value);
        painter.line_segment(
            [
                egui::pos2(x, rect.top() + 2.0),
                egui::pos2(x, rect.bottom()),
            ],
            egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 120, 80)),
        );
        painter.text(
            egui::pos2(x + 4.0, rect.bottom()),
            egui::Align2::LEFT_BOTTOM,
            format!("{value:.2}"),
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(255, 120, 80),
        );
    }
}

/// A slider range for each Number parameter: from 0 (or lower) to past
/// the biggest value its conditions test and its blend trees' thresholds.
pub(super) fn number_ranges(file: &StateMachineFile) -> BTreeMap<String, (f64, f64)> {
    let mut ranges = BTreeMap::new();
    for def in &file.parameters {
        if let ParameterDef::Number { name, default } = def {
            let mut low: f64 = default.min(0.0);
            let mut high: f64 = default.max(1.0);
            for condition in file.transitions.iter().flat_map(|t| &t.conditions) {
                if condition.parameter == *name {
                    low = low.min(condition.value);
                    high = high.max(condition.value * 1.5);
                }
            }
            for state in &file.states {
                if let Motion::BlendTree(tree) = &state.motion
                    && tree.parameter == *name
                {
                    for child in &tree.children {
                        low = low.min(child.threshold);
                        high = high.max(child.threshold);
                    }
                }
            }
            ranges.insert(name.clone(), (low, high.max(low + 1.0)));
        }
    }
    ranges
}

/// A subtle `(?)` hover icon that replaces verbose inline explanatory paragraphs.
fn help_icon(ui: &mut egui::Ui, tip: &str) {
    let color = crate::theme::palette(ui.ctx()).text_muted;
    ui.add(
        egui::Label::new(egui::RichText::new(icons::QUESTION).color(color))
            .sense(egui::Sense::hover()),
    )
    .on_hover_text(tip);
}

impl ControllerDoc {
    /// The inspector: the selected state or transition, or the controller's
    /// own settings (parameters, default state).
    pub fn inspector(&mut self, ui: &mut egui::Ui, clips: &[String]) {
        match self.selected {
            Selected::Node(Node::State(index)) if index < self.file.states.len() => {
                self.state_inspector(ui, index, clips);
                ui.separator();
                self.parameters_section(ui, false);
            }
            Selected::Transition(index) if index < self.file.transitions.len() => {
                self.transition_inspector(ui, index);
                ui.separator();
                self.parameters_section(ui, true);
            }
            Selected::Node(Node::Any) => {
                ui.horizontal(|ui| {
                    ui.heading(ANY_STATE);
                    help_icon(
                        ui,
                        "Wires from Any State can fire regardless of the current state (never to the state already playing). Use for Jump, Hit, or Emotes.",
                    );
                });
                ui.separator();
                self.parameters_section(ui, true);
            }
            Selected::Node(Node::Entry) => {
                ui.horizontal(|ui| {
                    ui.heading("Entry");
                    help_icon(
                        ui,
                        "Where the Animator starts. Drag from its pin to another state to change the default state.",
                    );
                });
                ui.add_space(4.0);
                self.default_state_picker(ui);
                ui.separator();
                self.parameters_section(ui, true);
            }
            _ => self.controller_inspector(ui),
        }
    }

    pub(super) fn controller_inspector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(
                axcenity_anim::files::clip_name(&self.path).trim_end_matches(".statemachine.json"),
            );
            help_icon(
                ui,
                "Starts at the state Entry is wired to, and follows transitions when their conditions hold. Right-click the graph or drop a clip to add states.",
            );
        });
        ui.weak(&self.path);
        ui.add_space(4.0);
        self.default_state_picker(ui);
        ui.separator();
        self.parameters_section(ui, true);
        ui.separator();
        self.layers_section(ui);
    }

    /// Unified Parameters panel: type badge (draggable onto wires), editable name,
    /// default value, and live interactive preview control in one row.
    fn parameters_section(&mut self, ui: &mut egui::Ui, default_open: bool) {
        let mut header_tip = false;
        let collapsing = egui::CollapsingHeader::new(format!(
            "{} Parameters ({})",
            icons::SLIDERS_HORIZONTAL,
            self.file.parameters.len()
        ))
        .id_salt("unified parameters accordion")
        .default_open(default_open)
        .icon(crate::widgets::chevron)
        .show(ui, |ui| {
            self.view.chips.clear();
            let ranges = number_ranges(&self.file);
            let mut remove = None;
            let mut renamed = None;

            if let Some(run) = &self.preview {
                for problem in &run.problems {
                    ui.colored_label(ui.visuals().warn_fg_color, problem);
                }
            }

            for (index, def) in self.file.parameters.iter_mut().enumerate() {
                let kind = ParameterKind::of(def);
                let param_name = def.name().to_owned();
                let color = kind.color();
                ui.horizontal(|ui| {
                    // Draggable colored type badge (can be dropped onto wires).
                    let chip_resp = ui
                        .dnd_drag_source(
                            ui.id().with(("param badge", index, &param_name)),
                            crate::animation::graph::ParameterChip(param_name.clone()),
                            |ui| {
                                egui::Frame::new()
                                    .fill(color.gamma_multiply(0.2))
                                    .stroke(egui::Stroke::new(1.0_f32, color))
                                    .corner_radius(4)
                                    .inner_margin(egui::Margin::symmetric(4, 1))
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(kind.icon())
                                                .size(11.0)
                                                .color(color),
                                        );
                                    });
                            },
                        )
                        .response
                        .on_hover_text(format!(
                            "{} — drag onto a wire to add a condition",
                            kind.label()
                        ));
                    self.view.chips.push((param_name.clone(), chip_resp.rect));

                    let mut name = param_name.clone();
                    if ui
                        .add(egui::TextEdit::singleline(&mut name).desired_width(82.0))
                        .changed()
                    {
                        renamed = Some((param_name.clone(), name.clone()));
                        match def {
                            ParameterDef::Bool { name: n, .. }
                            | ParameterDef::Number { name: n, .. }
                            | ParameterDef::Trigger { name: n } => *n = name,
                        }
                    }

                    // Default value (compact) + live preview control right beside it.
                    match def {
                        ParameterDef::Bool { default, .. } => {
                            ui.checkbox(default, "")
                                .on_hover_text("Default value at start");
                            if let Some(run) = &mut self.preview {
                                let slot = run
                                    .parameters
                                    .entry(param_name.clone())
                                    .or_insert_with(|| Parameter::Bool(*default));
                                if let Parameter::Bool(live_val) = slot {
                                    let label = if *live_val { "On" } else { "Off" };
                                    if ui
                                        .selectable_label(*live_val, label)
                                        .on_hover_text("Live preview value")
                                        .clicked()
                                    {
                                        *live_val = !*live_val;
                                    }
                                }
                            }
                        }
                        ParameterDef::Number { default, .. } => {
                            ui.add(egui::DragValue::new(default).speed(0.05).max_decimals(2))
                                .on_hover_text("Default value at start");
                            if let Some(run) = &mut self.preview {
                                let slot = run
                                    .parameters
                                    .entry(param_name.clone())
                                    .or_insert_with(|| Parameter::Number(*default));
                                if let Parameter::Number(live_val) = slot {
                                    let (low, high) =
                                        ranges.get(&param_name).copied().unwrap_or((0.0, 10.0));
                                    ui.add(
                                        egui::Slider::new(live_val, low..=high)
                                            .show_value(false)
                                            .clamping(egui::SliderClamping::Never),
                                    )
                                    .on_hover_text(format!("Live preview: {live_val:.2}"));
                                }
                            }
                        }
                        ParameterDef::Trigger { .. } => {
                            if let Some(run) = &mut self.preview {
                                let slot = run
                                    .parameters
                                    .entry(param_name.clone())
                                    .or_insert(Parameter::Trigger(false));
                                if let Parameter::Trigger(set) = slot
                                    && ui
                                        .small_button(format!("{} Fire", icons::LIGHTNING))
                                        .on_hover_text("Trigger in live preview")
                                        .clicked()
                                {
                                    *set = true;
                                }
                            }
                        }
                    }

                    if ui.small_button(icons::X).on_hover_text("Remove").clicked() {
                        remove = Some(index);
                    }
                });
            }

            // Conditions follow a renamed parameter.
            if let Some((old, new)) = renamed {
                for transition in &mut self.file.transitions {
                    for condition in &mut transition.conditions {
                        if condition.parameter == old {
                            condition.parameter = new.clone();
                        }
                    }
                }
            }
            if let Some(index) = remove {
                self.file.parameters.remove(index);
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for kind in ParameterKind::ALL {
                    if ui
                        .small_button(format!("{} {}", icons::PLUS, kind.label()))
                        .clicked()
                    {
                        self.add_parameter(kind);
                    }
                }
            });
            let duplicates = self.file.parameters.iter().enumerate().any(|(i, a)| {
                self.file.parameters[..i]
                    .iter()
                    .any(|b| b.name() == a.name())
            });
            if duplicates {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    "Two parameters have the same name.",
                );
            }
            header_tip = true;
        });
        let _ = header_tip;
        collapsing.header_response.on_hover_text(
            "Values driven by scripts/blocks and tested by transitions. Drag a type badge onto a wire to add a condition, or use the live controls to test the preview.",
        );
    }

    /// The default state, which Entry is wired to.
    fn default_state_picker(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Default state")
                .on_hover_text("The state the Animator starts in (Entry is wired to it)");
            let names: Vec<String> = self.file.states.iter().map(|s| s.name.clone()).collect();
            egui::ComboBox::from_id_salt("default state")
                .selected_text(&self.file.default_state)
                .show_ui(ui, |ui| {
                    for name in names {
                        ui.selectable_value(&mut self.file.default_state, name.clone(), name);
                    }
                });
        });
    }

    pub(super) fn free_parameter_name(&self, wanted: &str) -> String {
        (1u32..)
            .map(|n| match n {
                1 => wanted.to_owned(),
                n => format!("{wanted}{n}"),
            })
            .find(|name| !self.file.parameters.iter().any(|p| p.name() == name))
            .expect("some number is always free")
    }

    pub(super) fn state_inspector(&mut self, ui: &mut egui::Ui, index: usize, clips: &[String]) {
        let name = self.file.states[index].name.clone();
        match &mut self.renaming {
            Some((renaming, text)) if *renaming == index => {
                let mut done = false;
                let mut text = text.clone();
                ui.horizontal_wrapped(|ui| {
                    let response = ui.text_edit_singleline(&mut text);
                    if ui.button("Rename").clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        done = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.renaming = None;
                    }
                });
                if done {
                    match self.rename_state(index, &text) {
                        Ok(()) => self.renaming = None,
                        Err(problem) => {
                            ui.colored_label(ui.visuals().error_fg_color, problem);
                            self.renaming = Some((index, text));
                        }
                    }
                } else if self.renaming.is_some() {
                    self.renaming = Some((index, text));
                }
            }
            _ => {
                ui.horizontal_wrapped(|ui| {
                    ui.heading(&name);
                    if ui
                        .small_button(icons::PENCIL_SIMPLE)
                        .on_hover_text("Rename this state")
                        .clicked()
                    {
                        self.renaming = Some((index, name.clone()));
                    }
                });
            }
        }
        ui.label(words::state_header(&self.file, &name));
        let is_default = self.file.default_state == name;
        if is_default {
            ui.weak("The default state: the animator starts here.");
        } else if ui.button("Set as default state").clicked() {
            self.file.default_state = name.clone();
        }
        ui.add_space(4.0);
        // Clip, blend tree or blend space.
        let is_tree = matches!(self.file.states[index].motion, Motion::BlendTree(_));
        let is_space = matches!(self.file.states[index].motion, Motion::BlendSpace(_));
        let is_clip = !is_tree && !is_space;
        let mut make_tree = false;
        let mut make_space = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Plays");
            if ui.selectable_label(is_clip, "A clip").clicked() && !is_clip {
                let first = self.file.states[index]
                    .motion
                    .clips()
                    .first()
                    .map(|clip| (*clip).clone())
                    .unwrap_or_default();
                self.file.states[index].motion = Motion::Clip(first);
            }
            if ui
                .selectable_label(is_tree, "A blend tree")
                .on_hover_text(
                    "Mix related clips by a Number parameter, such as Idle, Walk and Run by Speed",
                )
                .clicked()
                && !is_tree
            {
                make_tree = true;
            }
            if ui
                .selectable_label(is_space, "A blend space")
                .on_hover_text(
                    "Mix clips placed by two Number parameters, such as eight strafes by sideways and forward speed",
                )
                .clicked()
                && !is_space
            {
                make_space = true;
            }
        });
        if make_space {
            self.make_blend_space(index);
        }
        if make_tree {
            let parameter = self.number_parameter();
            let children = match &self.file.states[index].motion {
                Motion::Clip(clip) if !clip.is_empty() => vec![BlendChild {
                    clip: clip.clone(),
                    threshold: 0.0,
                    speed: 1.0,
                }],
                _ => Vec::new(),
            };
            self.file.states[index].motion = Motion::BlendTree(BlendTree {
                parameter,
                children,
            });
        }
        if matches!(self.file.states[index].motion, Motion::BlendTree(_)) {
            self.blend_tree_editor(ui, index, clips);
        }
        if matches!(self.file.states[index].motion, Motion::BlendSpace(_)) {
            self.blend_space_editor(ui, index, clips);
        }
        let group_names: Vec<String> = self.file.groups.iter().map(|g| g.name.clone()).collect();
        crate::responsive::form(ui, "state settings", |ui| {
            let state = &mut self.file.states[index];
            if let Motion::Clip(current) = &mut state.motion {
                ui.label("Clip");
                clip_picker(ui, "state clip", current, clips);
                ui.end_row();
            }
            ui.label("Speed");
            ui.add(
                egui::DragValue::new(&mut state.speed)
                    .range(-10.0..=10.0)
                    .speed(0.01)
                    .suffix("×"),
            );
            ui.end_row();
            ui.label("Group");
            let current_group = state.group.clone().unwrap_or_default();
            egui::ComboBox::from_id_salt("state group")
                .selected_text(if current_group.is_empty() {
                    "(none)"
                } else {
                    current_group.as_str()
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(state.group.is_none(), "(none)")
                        .clicked()
                    {
                        state.group = None;
                    }
                    for g in &group_names {
                        if ui
                            .selectable_label(state.group.as_deref() == Some(g.as_str()), g)
                            .clicked()
                        {
                            state.group = Some(g.clone());
                        }
                    }
                });
            ui.end_row();
        });
        self.return_editor(ui, index);
        ui.separator();
        ui.strong("Transitions");
        let outgoing: Vec<(usize, String, bool)> = self
            .file
            .transitions
            .iter()
            .enumerate()
            .filter(|(_, t)| t.from == name || t.to == name)
            .map(|(i, t)| {
                let out = t.from == name;
                (
                    i,
                    if out {
                        format!("→ {}", t.to)
                    } else {
                        format!("← {}", t.from)
                    },
                    out,
                )
            })
            .collect();
        if outgoing.is_empty() {
            ui.weak("None yet. Drag from its right pin to another state.");
        }
        for (index, label, _) in outgoing {
            if ui.selectable_label(false, label).clicked() {
                self.selected = Selected::Transition(index);
            }
        }
        ui.separator();
        if ui
            .button(format!("{} Delete state", icons::TRASH))
            .clicked()
        {
            self.delete(Selected::Node(Node::State(index)));
        }
    }

    /// A state's Auto-Return setting: quick one-click "Auto-Return to Default (on finish)"
    /// plus an expandable custom return editor.
    pub(super) fn return_editor(&mut self, ui: &mut egui::Ui, index: usize) {
        ui.separator();
        let others: Vec<String> = self
            .file
            .states
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .map(|(_, s)| s.name.clone())
            .collect();
        let parameters = self.file.parameters.clone();
        let default = self.file.default_state.clone();
        let is_default = self.file.states[index].name == default;
        let state = &mut self.file.states[index];

        let default_target = others
            .iter()
            .find(|name| **name == default)
            .or(others.first())
            .cloned()
            .unwrap_or_default();
        let mut auto_return = state
            .returns
            .as_ref()
            .is_some_and(|r| matches!(r.when, ReturnWhen::Finished) && r.to == default_target);
        if !is_default
            && !default_target.is_empty()
            && ui
                .checkbox(
                    &mut auto_return,
                    egui::RichText::new("Auto-Return to Default (on finish)").strong(),
                )
                .on_hover_text(
                    "Automatically return to the default state when this clip finishes — no manual return wire needed.",
                )
                .changed()
        {
            state.returns = auto_return.then(|| ReturnDef {
                to: default_target.clone(),
                when: ReturnWhen::Finished,
                blend: DEFAULT_BLEND,
            });
        }

        let is_custom = state.returns.is_some() && !auto_return;
        egui::CollapsingHeader::new("Custom return options")
            .id_salt(("custom return", index))
            .default_open(is_custom)
            .icon(crate::widgets::chevron)
            .show(ui, |ui| {
                let mut on = state.returns.is_some();
                if ui.checkbox(&mut on, "Enable return transition").changed() {
                    state.returns = on.then(|| ReturnDef {
                        to: default_target.clone(),
                        when: ReturnWhen::Finished,
                        blend: DEFAULT_BLEND,
                    });
                }
                let Some(returns) = &mut state.returns else {
                    return;
                };
                let names = |of: fn(&ParameterDef) -> bool| -> Vec<String> {
                    parameters
                        .iter()
                        .filter(|def| of(def))
                        .map(|def| def.name().to_owned())
                        .collect()
                };
                let bools = names(|def| matches!(def, ParameterDef::Bool { .. }));
                let triggers = names(|def| matches!(def, ParameterDef::Trigger { .. }));
                crate::responsive::form(ui, "return settings", |ui| {
                    ui.label("When");
                    let label = match &returns.when {
                        ReturnWhen::Finished => "its clip finishes",
                        ReturnWhen::BoolIs { .. } => "a Bool becomes",
                        ReturnWhen::Triggered { .. } => "a Trigger is set",
                        ReturnWhen::After { .. } => "some seconds pass",
                    };
                    egui::ComboBox::from_id_salt("return when")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    matches!(returns.when, ReturnWhen::Finished),
                                    "its clip finishes",
                                )
                                .clicked()
                            {
                                returns.when = ReturnWhen::Finished;
                            }
                            if ui
                                .selectable_label(
                                    matches!(returns.when, ReturnWhen::BoolIs { .. }),
                                    "a Bool becomes",
                                )
                                .clicked()
                            {
                                returns.when = ReturnWhen::BoolIs {
                                    parameter: bools.first().cloned().unwrap_or_default(),
                                    value: false,
                                };
                            }
                            if ui
                                .selectable_label(
                                    matches!(returns.when, ReturnWhen::Triggered { .. }),
                                    "a Trigger is set",
                                )
                                .clicked()
                            {
                                returns.when = ReturnWhen::Triggered {
                                    parameter: triggers.first().cloned().unwrap_or_default(),
                                };
                            }
                            if ui
                                .selectable_label(
                                    matches!(returns.when, ReturnWhen::After { .. }),
                                    "some seconds pass",
                                )
                                .clicked()
                            {
                                returns.when = ReturnWhen::After { seconds: 1.0 };
                            }
                        });
                    ui.end_row();
                    match &mut returns.when {
                        ReturnWhen::Finished => {}
                        ReturnWhen::BoolIs { parameter, value } => {
                            ui.label("Bool");
                            ui.horizontal_wrapped(|ui| {
                                egui::ComboBox::from_id_salt("return bool")
                                    .selected_text(parameter.as_str())
                                    .show_ui(ui, |ui| {
                                        for name in &bools {
                                            ui.selectable_value(parameter, name.clone(), name);
                                        }
                                    });
                                ui.label("is");
                                ui.selectable_value(value, true, "true");
                                ui.selectable_value(value, false, "false");
                            });
                            ui.end_row();
                        }
                        ReturnWhen::Triggered { parameter } => {
                            ui.label("Trigger");
                            egui::ComboBox::from_id_salt("return trigger")
                                .selected_text(parameter.as_str())
                                .show_ui(ui, |ui| {
                                    for name in &triggers {
                                        ui.selectable_value(parameter, name.clone(), name);
                                    }
                                });
                            ui.end_row();
                        }
                        ReturnWhen::After { seconds } => {
                            ui.label("After");
                            ui.add(
                                egui::DragValue::new(seconds)
                                    .range(0.0..=600.0)
                                    .speed(0.05)
                                    .suffix(" s"),
                            );
                            ui.end_row();
                        }
                    }
                    ui.label("Back to");
                    egui::ComboBox::from_id_salt("return to")
                        .selected_text(returns.to.as_str())
                        .show_ui(ui, |ui| {
                            for name in &others {
                                ui.selectable_value(&mut returns.to, name.clone(), name);
                            }
                        });
                    ui.end_row();
                    ui.label("Blend");
                    ui.horizontal_wrapped(|ui| {
                        ui.add(egui::Slider::new(&mut returns.blend, 0.0..=2.0).suffix(" s"));
                    });
                    ui.end_row();
                });
                let missing = match &returns.when {
                    ReturnWhen::BoolIs { parameter, .. } | ReturnWhen::Triggered { parameter } => {
                        parameter.is_empty()
                    }
                    _ => false,
                };
                if missing {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Add a parameter of that kind in Parameters to test.",
                    );
                }
            });
    }

    pub(super) fn transition_inspector(&mut self, ui: &mut egui::Ui, index: usize) {
        let parameters = self.file.parameters.clone();
        let transition = &mut self.file.transitions[index];
        ui.heading(format!("{} → {}", transition.from, transition.to));
        ui.label(words::transition_header(transition));
        if transition.generated {
            ui.label("Made by its state's Return setting, so it can't be changed here.");
            let from = transition.from.clone();
            if ui.button("Edit the Return setting").clicked()
                && let Some(state) = self.file.states.iter().position(|s| s.name == from)
            {
                self.selected = Selected::Node(Node::State(state));
            }
            if ui
                .button(format!("{} Remove it", icons::TRASH))
                .on_hover_text("Turns off its state's Return setting")
                .clicked()
            {
                self.delete(Selected::Transition(index));
            }
            return;
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong("Conditions");
            help_icon(
                ui,
                "All conditions must hold for the transition to fire. You can also drag a parameter badge onto the wire.",
            );
        });
        let mut remove = None;
        egui::Grid::new("conditions")
            .num_columns(4)
            .spacing([6.0, 4.0])
            .show(ui, |ui| {
                for (c, condition) in transition.conditions.iter_mut().enumerate() {
                    egui::ComboBox::from_id_salt(("condition parameter", c))
                        .width(100.0)
                        .selected_text(&condition.parameter)
                        .show_ui(ui, |ui| {
                            for def in &parameters {
                                ui.selectable_value(
                                    &mut condition.parameter,
                                    def.name().to_owned(),
                                    def.name(),
                                );
                            }
                        });
                    let kind = parameters
                        .iter()
                        .find(|def| def.name() == condition.parameter)
                        .map(ParameterKind::of);
                    let tests = super::wire_editor::tests_for(kind);
                    if let Some(&(first, _)) = tests.first()
                        && !tests.iter().any(|(test, _)| *test == condition.test)
                    {
                        condition.test = first;
                    }
                    let label = tests
                        .iter()
                        .find(|(test, _)| *test == condition.test)
                        .map_or("(pick a parameter)", |(_, label)| label);
                    egui::ComboBox::from_id_salt(("condition test", c))
                        .width(90.0)
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for &(test, label) in tests {
                                ui.selectable_value(&mut condition.test, test, label);
                            }
                        });
                    if matches!(condition.test, Test::Greater | Test::Less) {
                        ui.add(egui::DragValue::new(&mut condition.value).speed(0.05));
                    } else {
                        ui.label("");
                    }
                    if ui.small_button(icons::X).on_hover_text("Remove").clicked() {
                        remove = Some(c);
                    }
                    ui.end_row();
                }
            });
        if let Some(c) = remove {
            transition.conditions.remove(c);
        }
        if parameters.is_empty() {
            ui.weak("Add parameters in the state machine's settings (click the graph's background) to test them here.");
        } else if ui
            .button(format!("{} Add condition", icons::PLUS))
            .clicked()
        {
            self.add_condition(index, parameters[0].name());
        }
        let transition = &mut self.file.transitions[index];

        ui.separator();
        ui.strong("Blend");
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Slider::new(&mut transition.blend, 0.0..=2.0)
                    .suffix(" s")
                    .text("duration"),
            )
            .on_hover_text("How long the old pose takes to turn into the new one");
        });
        ui.horizontal_wrapped(|ui| {
            for (label, seconds) in BLENDS {
                if ui
                    .selectable_label((transition.blend - seconds).abs() < 1e-9, label)
                    .clicked()
                {
                    transition.blend = seconds;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Preview the blend:");
            ui.add(egui::Slider::new(&mut self.scrub, 0.0..=1.0).show_value(false));
        });
        let new = (self.scrub * 100.0).round();
        ui.weak(format!(
            "{}: {:.0}%  ·  {}: {new:.0}%{}",
            transition.from,
            100.0 - new,
            transition.to,
            if self.scrub >= 1.0 {
                "  (drag left to see the mix)"
            } else {
                ""
            }
        ));

        ui.separator();
        crate::responsive::form(ui, "transition settings", |ui| {
            ui.label("Exit time");
            ui.horizontal_wrapped(|ui| {
                let mut on = transition.exit_time.is_some();
                if ui
                    .checkbox(&mut on, "")
                    .on_hover_text(
                        "Only once the state has played this much of its clip (1 is once through)",
                    )
                    .changed()
                {
                    transition.exit_time = on.then_some(0.9);
                }
                if let Some(exit) = &mut transition.exit_time {
                    ui.add(egui::DragValue::new(exit).range(0.0..=10.0).speed(0.01));
                    ui.weak("of the clip");
                }
            });
            ui.end_row();
            ui.label("Interruptible");
            ui.checkbox(&mut transition.interruptible, "")
                .on_hover_text("On, another transition can take over while this one blends");
            ui.end_row();
            ui.label("Priority");
            ui.add(egui::DragValue::new(&mut transition.priority).range(-100..=100))
                .on_hover_text("When several transitions could happen at once, the highest goes");
            ui.end_row();
        });
        if transition.conditions.is_empty() && transition.exit_time.is_none() {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "This never happens: add a condition, or turn on Exit time to go on by itself.",
            );
        } else if transition.conditions.is_empty() {
            ui.weak("With no conditions, it happens by itself at its exit time.");
        }
        ui.separator();
        if ui
            .button(format!("{} Delete transition", icons::TRASH))
            .clicked()
        {
            self.delete(Selected::Transition(index));
        }
    }

    /// The blend tree of state `index`: its parameter, each clip with its
    /// threshold (a slider and a number) and its share now in the preview,
    /// a line of the thresholds with the value now, and Add Locomotion.
    pub(super) fn blend_tree_editor(&mut self, ui: &mut egui::Ui, index: usize, clips: &[String]) {
        // The preview's mix now, and its parameter's value.
        let state_name = self.file.states[index].name.clone();
        let (weights, value) = match &self.preview {
            Some(run) => {
                let state = run.compiled.state_index(&state_name);
                let weights = state.and_then(|s| run.compiled.tree_weights(s, &run.parameters));
                let Motion::BlendTree(tree) = &self.file.states[index].motion else {
                    return;
                };
                let value = match run.parameters.get(&tree.parameter) {
                    Some(Parameter::Number(value)) => Some(*value),
                    _ => None,
                };
                (weights, value)
            }
            None => (None, None),
        };
        let numbers: Vec<String> = self
            .file
            .parameters
            .iter()
            .filter(|def| matches!(def, ParameterDef::Number { .. }))
            .map(|def| def.name().to_owned())
            .collect();
        let mut locomotion = false;
        let Motion::BlendTree(tree) = &mut self.file.states[index].motion else {
            return;
        };
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong(format!("{} Blend tree", icons::SLIDERS_HORIZONTAL));
            ui.horizontal_wrapped(|ui| {
                ui.label("By");
                egui::ComboBox::from_id_salt("tree parameter")
                    .selected_text(&tree.parameter)
                    .show_ui(ui, |ui| {
                        for name in &numbers {
                            ui.selectable_value(&mut tree.parameter, name.clone(), name);
                        }
                    });
                ui.weak("(a Number parameter)");
            });
            let low = tree
                .children
                .iter()
                .map(|c| c.threshold)
                .fold(0.0_f64, f64::min);
            let high = tree
                .children
                .iter()
                .map(|c| c.threshold)
                .fold(6.0_f64, f64::max);
            let mut remove = None;
            egui::Grid::new("tree children")
                .num_columns(5)
                .spacing([6.0, 4.0])
                .show(ui, |ui| {
                    ui.weak("Clip");
                    ui.weak("Threshold");
                    ui.weak("Speed")
                        .on_hover_text("Plays this clip this many times as fast, so its stride keeps up with its threshold");
                    ui.weak("Share");
                    ui.end_row();
                    for (c, child) in tree.children.iter_mut().enumerate() {
                        clip_picker(ui, ("tree clip", c), &mut child.clip, clips);
                        ui.horizontal_wrapped(|ui| {
                            ui.add(
                                egui::Slider::new(&mut child.threshold, low..=high)
                                    .show_value(false),
                            );
                            ui.add(egui::DragValue::new(&mut child.threshold).speed(0.05));
                        });
                        ui.add(
                            egui::DragValue::new(&mut child.speed)
                                .speed(0.01)
                                .range(0.05..=10.0)
                                .suffix("×"),
                        );
                        let share = weights.as_ref().and_then(|w| w.get(c)).copied().unwrap_or(0.0);
                        ui.add(
                            egui::ProgressBar::new(share as f32)
                                .desired_width(60.0)
                                .text(format!("{:.0}%", share * 100.0)),
                        );
                        if ui.small_button(icons::X).on_hover_text("Remove").clicked() {
                            remove = Some(c);
                        }
                        ui.end_row();
                    }
                });
            if let Some(c) = remove {
                tree.children.remove(c);
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button(format!("{} Add clip", icons::PLUS)).clicked() {
                    let threshold = tree
                        .children
                        .iter()
                        .map(|c| c.threshold + 1.0)
                        .fold(0.0, f64::max);
                    tree.children.push(BlendChild {
                        clip: String::new(),
                        threshold,
                        speed: 1.0,
                    });
                }
                if ui
                    .button("Sort")
                    .on_hover_text("Put the clips in threshold order")
                    .clicked()
                {
                    tree.children
                        .sort_by(|a, b| a.threshold.total_cmp(&b.threshold));
                }
                if ui
                    .button("Add Locomotion")
                    .on_hover_text("Speed (0–6 m/s): Idle at 0, Walk at 2, Run at 6, with the project's clips of those names")
                    .clicked()
                {
                    locomotion = true;
                }
            });
            threshold_line(ui, tree, value, low, high);
        });
        if locomotion {
            let folder = self.file.states[index]
                .motion
                .clips()
                .into_iter()
                .find(|clip| !clip.is_empty())
                .map(|clip| super::super::store::group_of(clip).to_owned());
            let parameter = if self
                .file
                .parameters
                .iter()
                .any(|def| def.name() == "Speed" && matches!(def, ParameterDef::Number { .. }))
            {
                "Speed".to_owned()
            } else if self.file.parameters.iter().any(|def| def.name() == "Speed") {
                self.number_parameter()
            } else {
                self.file.parameters.push(ParameterDef::Number {
                    name: "Speed".to_owned(),
                    default: 0.0,
                });
                "Speed".to_owned()
            };
            let children = [("idle", 0.0), ("walk", 2.0), ("run", 6.0)]
                .into_iter()
                .map(|(word, threshold)| BlendChild {
                    clip: clip_named(clips, word, folder.as_deref()).unwrap_or_default(),
                    threshold,
                    speed: 1.0,
                })
                .collect();
            self.file.states[index].motion = Motion::BlendTree(BlendTree {
                parameter,
                children,
            });
        }
    }
}
