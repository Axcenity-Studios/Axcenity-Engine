//! The Console's log: messages from the editor, such as which project opened
//! and why a file couldn't load, during Play what scripts `print`, their
//! errors, and engine crashes, and the commands typed into the Console
//! (`console.rs`) with what they printed.
//!
//! Script errors start with `path:line:`. That part is underlined, and
//! clicking the line opens the script at that line. Engine compile errors
//! (`crates/…/x.rs:line:column:`) link the same way, to the Rust file. Under a script error is
//! its stack trace: the calls that led to it, innermost first, each a link
//! to its own script and line.
//!
//! Each line has the time it came at its left (the local clock), and a
//! Copy button while hovered (the line and its stack trace). Above the
//! lines: the kinds' filter buttons, Filter logs (only lines with that
//! text), Collapse (each message once, at its newest, with how many times
//! it came) and Clear.

use std::collections::HashMap;

use eframe::egui;

use crate::theme::{self, icons};
use crate::widgets;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutputKind {
    Info,
    Warning,
    Error,
    /// A command typed into the Console, echoed. Always shown.
    Command,
}

impl OutputKind {
    /// Its place in [`Output`]'s counts.
    fn index(self) -> usize {
        self as usize
    }
}

/// The most lines the Console keeps: older ones are dropped, so a script
/// that prints every frame can't slow the editor down over a session.
pub const MAX_LINES: usize = 10_000;
/// Dropped lines leave memory this many at a time, so a full Console
/// doesn't move 10,000 lines along for each new one.
const DROP_BATCH: usize = 1_000;
/// The room between lines.
const LINE_GAP: f32 = 2.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputLine {
    pub kind: OutputKind,
    pub text: String,
    /// Where in which script an error happened, if the line starts with
    /// `path:line:`.
    pub location: Option<Location>,
    /// For a script error, the calls that led to it, innermost first.
    pub trace: Vec<TraceLine>,
    /// When it came, in seconds since local midnight.
    pub time: u32,
}

impl OutputLine {
    /// The line and its stack trace, as copied.
    pub fn copy_text(&self) -> String {
        let mut text = format!("[{}] {}", clock(self.time), self.text);
        for frame in &self.trace {
            text.push_str("\n    ");
            text.push_str(&frame.text);
        }
        text
    }

    fn matches(&self, filter: &str) -> bool {
        contains_ignoring_case(&self.text, filter)
            || self
                .trace
                .iter()
                .any(|frame| contains_ignoring_case(&frame.text, filter))
    }
}

/// Whether `text` has `lower` (already lower case) in it, whatever its case.
fn contains_ignoring_case(text: &str, lower: &str) -> bool {
    text.to_lowercase().contains(lower)
}

/// The local time now, in seconds since midnight.
fn now_of_day() -> u32 {
    use chrono::Timelike;
    chrono::Local::now().num_seconds_from_midnight()
}

/// `seconds` since midnight as `21:12:04`.
fn clock(seconds: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// What the rows shown were worked out for: the lines kept (the first
/// one's number and how many), the kinds hidden, the filter and Collapse.
#[derive(Clone, PartialEq, Eq)]
struct RowsKey {
    gone: usize,
    len: usize,
    hidden: Vec<OutputKind>,
    filter: String,
    collapse: bool,
}

/// One call in a script error's stack trace, such as
/// `scripts/util.module.lua:5: in field 'explode'`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceLine {
    pub text: String,
    pub location: Option<Location>,
}

/// A line in a script file, or in an engine source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    /// The script's path, such as `scripts/pad.server.lua`, or the engine
    /// file's in its workspace, such as `crates/editor/src/app.rs`.
    pub path: String,
    /// Counting from 1.
    pub line: usize,
}

/// The Console's lines. Each line has a number, counted from the first
/// line ever added; it stays the same as older lines are dropped, so
/// `seen`, `reveal` and `marked` keep pointing at the same line.
#[derive(Default)]
pub struct Output {
    /// The lines kept, oldest first: the newest [`MAX_LINES`] of these
    /// (see [`lines`](Self::lines)), plus up to [`DROP_BATCH`] dropped ones
    /// waiting to leave memory.
    lines: Vec<OutputLine>,
    /// How many lines have left `lines`: the first one's number.
    gone: usize,
    /// Lines dropped for being over [`MAX_LINES`] since the last Clear.
    dropped: usize,
    /// How many kept lines there are of each kind, for the filter buttons.
    counts: [usize; 4],
    /// Each line's height (beside `lines`) at the width and font size in
    /// `heights_for`: as last drawn, or worked out until it's drawn; `None`
    /// until it's needed.
    heights: Vec<Option<f32>>,
    heights_for: (u32, u32, u64),
    /// Kinds of line switched off with the filter buttons. Only what's
    /// shown changes; every line is kept.
    hidden: Vec<OutputKind>,
    /// The number of the next line when the panel was last drawn: errors
    /// from there on haven't been seen.
    seen: usize,
    /// The frame (`theme::frame_number`) the panel was last drawn in.
    drawn_pass: Option<u64>,
    /// A line (by number) to scroll to the next time the panel is drawn.
    reveal: Option<usize>,
    /// The line (by number) last scrolled to, outlined so the eye finds it.
    marked: Option<usize>,
    /// Filter logs: only lines with this in them (or their trace), in any
    /// case.
    pub filter: String,
    /// Each message once, at its newest, with how many times it came.
    pub collapse: bool,
    /// The rows shown (an index into `lines` and how many lines it stands
    /// for), worked out again only when their key changes.
    rows: Option<(RowsKey, Vec<(usize, usize)>)>,
}

impl Output {
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(OutputKind::Info, text.into());
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(OutputKind::Error, text.into());
    }

    /// A script error, with the stack trace of calls that led to it.
    pub fn script_error(&mut self, text: impl Into<String>, trace: Vec<String>) {
        self.push_with_trace(OutputKind::Error, text.into(), trace);
    }

    pub fn warning(&mut self, text: impl Into<String>) {
        self.push(OutputKind::Warning, text.into());
    }

    pub fn command(&mut self, text: impl Into<String>) {
        self.push(OutputKind::Command, text.into());
    }

    /// Removes every line.
    pub fn clear(&mut self) {
        self.gone += self.lines.len();
        self.lines.clear();
        self.heights.clear();
        self.counts = [0; 4];
        self.dropped = 0;
        self.seen = self.gone;
        self.reveal = None;
        self.marked = None;
        self.rows = None;
    }

    /// Where the kept lines start in `lines`.
    fn start(&self) -> usize {
        self.lines.len().saturating_sub(MAX_LINES)
    }

    /// The number the next line will get: how many lines have been added
    /// since the editor started.
    pub fn total(&self) -> usize {
        self.gone + self.lines.len()
    }

    /// The kept lines numbered `from` or later (see [`total`](Self::total)).
    pub fn since(&self, from: usize) -> &[OutputLine] {
        let first = from.max(self.gone + self.start()) - self.gone;
        &self.lines[first.min(self.lines.len())..]
    }

    /// Errors added since the panel was last drawn, or 0 while it's
    /// showing (drawn last frame), for a count on its tab.
    pub fn unseen_errors(&self, ctx: &egui::Context) -> usize {
        let showing = self
            .drawn_pass
            .is_some_and(|pass| pass + 1 >= theme::frame_number(ctx));
        if showing {
            return 0;
        }
        self.since(self.seen)
            .iter()
            .filter(|line| line.kind == OutputKind::Error)
            .count()
    }

    /// Scrolls to the newest error (showing errors if they were filtered
    /// out) the next time the panel is drawn.
    pub fn reveal_last_error(&mut self) {
        let start = self.start();
        self.reveal = self.lines[start..]
            .iter()
            .rposition(|line| line.kind == OutputKind::Error)
            .map(|index| self.gone + start + index);
        if self.reveal.is_some() {
            self.hidden.retain(|kind| *kind != OutputKind::Error);
            self.filter.clear();
        }
    }

    /// Works out the rows to show, if what they depend on changed: each an
    /// index into `lines` and how many lines it stands for (more than 1
    /// only with Collapse, where a message shows once, at its newest).
    fn update_rows(&mut self) {
        let key = RowsKey {
            gone: self.gone,
            len: self.lines.len(),
            hidden: self.hidden.clone(),
            filter: self.filter.trim().to_lowercase(),
            collapse: self.collapse,
        };
        if self.rows.as_ref().is_some_and(|(old, _)| *old == key) {
            return;
        }
        let start = self.start();
        let lines = &self.lines;
        let shown = (start..lines.len()).filter(|&index| {
            let line = &lines[index];
            !key.hidden.contains(&line.kind) && (key.filter.is_empty() || line.matches(&key.filter))
        });
        let rows = if key.collapse {
            // How many of each message, and where its newest is.
            let mut newest: HashMap<(OutputKind, &str), (usize, usize)> = HashMap::new();
            for index in shown.clone() {
                let line = &lines[index];
                let entry = newest.entry((line.kind, &line.text)).or_insert((index, 0));
                *entry = (index, entry.1 + 1);
            }
            shown
                .filter_map(|index| {
                    let line = &lines[index];
                    let (last, count) = newest[&(line.kind, line.text.as_str())];
                    (last == index).then_some((index, count))
                })
                .collect()
        } else {
            shown.map(|index| (index, 1)).collect()
        };
        self.rows = Some((key, rows));
    }

    fn push(&mut self, kind: OutputKind, text: String) {
        self.push_with_trace(kind, text, Vec::new());
    }

    fn push_with_trace(&mut self, kind: OutputKind, text: String, trace: Vec<String>) {
        // Also log it, so messages show up in the terminal when the editor
        // was started from one (handy when the window won't open).
        match kind {
            OutputKind::Info | OutputKind::Command => log::info!("{text}"),
            OutputKind::Warning => log::warn!("{text}"),
            OutputKind::Error => {
                log::error!("{text}");
                for frame in &trace {
                    log::error!("    {frame}");
                }
            }
        }
        // Only errors link to scripts: a `print` that happens to look like
        // `x.lua:3:` shouldn't.
        let location = match kind {
            OutputKind::Error => script_location(&text),
            OutputKind::Info | OutputKind::Warning | OutputKind::Command => None,
        };
        let trace = trace
            .into_iter()
            .map(|text| TraceLine {
                location: script_location(&text),
                text,
            })
            .collect();
        self.counts[kind.index()] += 1;
        self.lines.push(OutputLine {
            kind,
            text,
            location,
            trace,
            time: now_of_day(),
        });
        self.heights.push(None);
        if self.lines.len() > MAX_LINES {
            let leaving = self.lines[self.lines.len() - MAX_LINES - 1].kind;
            self.counts[leaving.index()] -= 1;
            self.dropped += 1;
        }
        if self.lines.len() >= MAX_LINES + DROP_BATCH {
            let leaving = self.lines.len() - MAX_LINES;
            self.lines.drain(..leaving);
            self.heights.drain(..leaving);
            self.gone += leaving;
        }
    }

    /// The kept lines, oldest first: at most [`MAX_LINES`].
    pub fn lines(&self) -> &[OutputLine] {
        &self.lines[self.start()..]
    }

    /// Draws the panel. Returns the location of a script error that was
    /// clicked, to open the script there.
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<Location> {
        let palette = theme::palette(ui.ctx());
        self.seen = self.total();
        self.drawn_pass = Some(theme::frame_number(ui.ctx()));
        let reveal = self.reveal.take();
        if reveal.is_some() {
            self.marked = reveal;
        }
        let marked = self.marked;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (kind, icon, color, name) in [
                (
                    OutputKind::Info,
                    icons::INFO,
                    palette.text_secondary,
                    "messages",
                ),
                (
                    OutputKind::Warning,
                    icons::WARNING,
                    palette.warning,
                    "warnings",
                ),
                (OutputKind::Error, icons::X_CIRCLE, palette.error, "errors"),
            ] {
                let count = self.counts[kind.index()];
                let shown = !self.hidden.contains(&kind);
                if filter_toggle(ui, icon, color, count, shown)
                    .on_hover_text(if shown {
                        format!("Hide {name}")
                    } else {
                        format!("Show {name}")
                    })
                    .clicked()
                {
                    if shown {
                        self.hidden.push(kind);
                    } else {
                        self.hidden.retain(|hidden| *hidden != kind);
                    }
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if ui
                    .add_enabled(
                        !self.lines().is_empty(),
                        widgets::button(
                            widgets::Role::Ghost,
                            Some(icons::BROOM),
                            "Clear",
                            &palette,
                        )
                        .min_size(egui::vec2(0.0, 22.0)),
                    )
                    .on_hover_text("Remove every line")
                    .clicked()
                {
                    self.clear();
                }
                if widgets::icon_button(
                    ui,
                    icons::STACK,
                    "Collapse: show each message once, at its newest, with how many times \
                     it came",
                    self.collapse,
                )
                .clicked()
                {
                    self.collapse = !self.collapse;
                }
                let width =
                    (ui.available_width() - widgets::SEARCH_FIELD_MARGINS - 4.0).clamp(40.0, 220.0);
                widgets::search_field(ui, &mut self.filter, "Filter logs…", width);
            });
        });
        ui.add_space(2.0);
        widgets::divider(ui);

        let mut clicked = None;
        let font = egui::TextStyle::Monospace.resolve(ui.style());
        let start = self.start();
        let (gone, dropped) = (self.gone, self.dropped);
        let filtering = !self.filter.trim().is_empty();
        self.update_rows();
        let rows = self
            .rows
            .as_ref()
            .map_or(&[][..], |(_, rows)| rows.as_slice());
        let lines = &self.lines;
        let heights = &mut self.heights;
        let heights_for = &mut self.heights_for;
        let avail_h = ui.available_height().max(0.0);
        let font_gen = theme::font_generation(ui.ctx());
        egui::ScrollArea::vertical()
            // Keep showing the newest line, unless the user scrolled up to
            // read, or a line is being revealed (sticking would pull back).
            .stick_to_bottom(reveal.is_none())
            .min_scrolled_height(0.0)
            .max_height(avail_h)
            .auto_shrink(false)
            .show_viewport(ui, |ui, viewport| {
                let origin = ui.max_rect().top();
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing.y = LINE_GAP;
                if dropped > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "{dropped} older lines were dropped: the Console keeps the newest \
                             {MAX_LINES}."
                        ))
                        .color(palette.text_weak),
                    );
                }
                if lines.len() == start {
                    ui.label(
                        egui::RichText::new(
                            "Nothing yet. Editor messages, and what scripts print during Play, \
                             show here. Type Lua or a command (try help) below.",
                        )
                        .color(palette.text_weak),
                    );
                } else if rows.is_empty() && filtering {
                    ui.label(
                        egui::RichText::new("No line has that in it.").color(palette.text_weak),
                    );
                }
                // Lines wrap, so each has its own height: as last drawn, or
                // worked out from its length until it has been.
                let width = ui.available_width();
                let key = (width.to_bits(), font.size.to_bits(), font_gen);
                if *heights_for != key {
                    heights.fill(None);
                    *heights_for = key;
                }
                let measure = Measure::new(ui, &font, width);
                let top = ui.cursor().top() - origin;
                let (mut above, mut below, mut y) = (0.0, 0.0, top);
                let mut visible = Vec::new();
                let mut reveal_at = None;
                for &(index, count) in rows {
                    let line = &lines[index];
                    // Worked out once; drawing replaces it with the real one.
                    let height = *heights[index].get_or_insert_with(|| measure.height(line));
                    if reveal.is_some_and(|number| number == gone + index) {
                        reveal_at = Some((y, height));
                    }
                    if y + height < viewport.min.y {
                        above += height + LINE_GAP;
                    } else if y > viewport.max.y {
                        below += height + LINE_GAP;
                    } else {
                        visible.push((index, count));
                    }
                    y += height + LINE_GAP;
                }
                if above > 0.0 {
                    ui.add_space(above);
                }
                for (index, count) in visible {
                    let line = &lines[index];
                    let (row, link) = draw_line(ui, line, count, &font, &palette);
                    if link.is_some() {
                        clicked = link;
                    }
                    heights[index] = Some(row.height());
                    if marked == Some(gone + index) {
                        ui.painter().rect_stroke(
                            row.expand(2.0),
                            theme::radius::CONTROL,
                            egui::Stroke::new(1.0_f32, palette.error),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
                if below > 0.0 {
                    ui.add_space(below);
                }
                if let Some((y, height)) = reveal_at {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(ui.max_rect().left(), origin + y),
                        egui::vec2(width, height),
                    );
                    // At once: an animated scroll would end up stuck to the bottom.
                    ui.scroll_to_rect_animation(
                        rect,
                        Some(egui::Align::Center),
                        egui::style::ScrollAnimation::none(),
                    );
                }
            });
        clicked
    }
}

/// One line: the time it came, its icon, how many times it came when that's
/// more than once (Collapse), then its text (wrapping beside the icon) with
/// the stack trace under it, and while hovered a Copy button at its right.
/// Returns where it was drawn, and its script location if it was clicked.
fn draw_line(
    ui: &mut egui::Ui,
    line: &OutputLine,
    count: usize,
    font: &egui::FontId,
    palette: &theme::Palette,
) -> (egui::Rect, Option<Location>) {
    let (icon, icon_color, color) = line_style(line.kind, palette);
    let mut clicked = None;
    let row = ui.horizontal_top(|ui| {
        ui.add(
            egui::Label::new(
                egui::RichText::new(clock(line.time))
                    .font(stamp_font(font))
                    .color(palette.text_muted),
            )
            .extend(),
        );
        ui.add(
            egui::Label::new(
                egui::RichText::new(icon)
                    .font(egui::FontId::proportional(font.size))
                    .color(icon_color),
            )
            .extend(),
        );
        if count > 1 {
            count_badge(ui, count, icon_color, palette);
        }
        let text_width = ui.available_width().max(40.0);
        ui.vertical(|ui| {
            ui.set_max_width(text_width);
            if let Some(location) =
                text_or_link(ui, &line.text, &line.location, font.clone(), color)
            {
                clicked = Some(location);
            }
            for frame in &line.trace {
                ui.horizontal_top(|ui| {
                    ui.add_space(TRACE_INDENT);
                    let trace_w = ui.available_width().max(40.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(trace_w);
                        if let Some(location) = text_or_link(
                            ui,
                            &frame.text,
                            &frame.location,
                            font.clone(),
                            palette.text_weak,
                        ) {
                            clicked = Some(location);
                        }
                    });
                });
            }
        });
    });
    let rect = row.response.rect;
    if ui.rect_contains_pointer(rect) {
        let button = egui::Rect::from_min_size(
            egui::pos2(rect.right() - 22.0, rect.top()),
            egui::vec2(22.0, 20.0),
        );
        ui.painter()
            .rect_filled(button, theme::radius::CONTROL, palette.panel);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(button)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let tip = if line.trace.is_empty() {
            "Copy this line"
        } else {
            "Copy this line and its stack trace"
        };
        if widgets::icon_button(&mut child, icons::COPY, tip, false).clicked() {
            ui.ctx().copy_text(line.copy_text());
        }
    }
    (rect, clicked)
}

/// The font of the time at each line's left: the code font, a size smaller.
fn stamp_font(font: &egui::FontId) -> egui::FontId {
    egui::FontId::monospace((font.size - 1.5).max(8.0))
}

/// How many times a collapsed message came, in a small pill.
fn count_badge(ui: &mut egui::Ui, count: usize, color: egui::Color32, palette: &theme::Palette) {
    let galley = ui.painter().layout_no_wrap(
        count.to_string(),
        egui::FontId::new(theme::type_scale::SMALL, theme::medium_family()),
        palette.text_strong,
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 10.0, galley.size().y + 2.0),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, rect.height() / 2.0, color.gamma_multiply(0.22));
    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        palette.text_strong,
    );
}

/// How far a stack trace is indented under its error.
const TRACE_INDENT: f32 = 12.0;

/// A line's icon, the icon's colour and the text's colour.
fn line_style(
    kind: OutputKind,
    palette: &theme::Palette,
) -> (&'static str, egui::Color32, egui::Color32) {
    match kind {
        OutputKind::Info => (icons::INFO, palette.text_weak, palette.text),
        OutputKind::Warning => (icons::WARNING, palette.warning, palette.warning),
        OutputKind::Error => (icons::X_CIRCLE, palette.error, palette.error),
        OutputKind::Command => (icons::CARET_RIGHT, palette.accent_text, palette.text_strong),
    }
}

/// Works out a line's height before it's drawn, from how many characters
/// fit on a row of the code font. Words wrap, so a drawn line can be a row
/// taller; its drawn height replaces this once it's on screen.
struct Measure {
    row: f32,
    char_width: f32,
    /// The text's width beside the icon, and a trace line's.
    text_width: f32,
    trace_width: f32,
}

impl Measure {
    fn new(ui: &egui::Ui, font: &egui::FontId, width: f32) -> Self {
        let (row, char_width, icon, digit) = ui.fonts_mut(|fonts| {
            (
                fonts.row_height(font),
                fonts.glyph_width(font, 'M'),
                fonts.glyph_width(&egui::FontId::proportional(font.size), '\u{e000}'),
                fonts.glyph_width(&stamp_font(font), '0'),
            )
        });
        let spacing = ui.spacing().item_spacing.x;
        // The time ("00:00:00") and the icon, each with a gap after it.
        let stamp = digit * 8.0 + spacing;
        let text_width = (width - stamp - icon.max(font.size) - spacing).max(char_width);
        Self {
            row,
            char_width: char_width.max(1.0),
            text_width,
            trace_width: (text_width - TRACE_INDENT - spacing).max(char_width),
        }
    }

    fn rows(&self, text: &str, width: f32) -> f32 {
        let per_row = (width / self.char_width).floor().max(1.0);
        text.split('\n')
            .map(|part| (part.chars().count() as f32 / per_row).ceil().max(1.0))
            .sum()
    }

    fn height(&self, line: &OutputLine) -> f32 {
        let mut height = self.rows(&line.text, self.text_width) * self.row;
        for frame in &line.trace {
            height += LINE_GAP + self.rows(&frame.text, self.trace_width) * self.row;
        }
        height.max(self.row)
    }
}

/// A filter button: the kind's icon and how many lines of it there are,
/// dimmed while that kind is hidden.
fn filter_toggle(
    ui: &mut egui::Ui,
    icon: &str,
    color: egui::Color32,
    count: usize,
    shown: bool,
) -> egui::Response {
    let palette = theme::palette(ui.ctx());
    let text = count.to_string();
    let galley = ui.painter().layout_no_wrap(
        text,
        egui::FontId::new(theme::type_scale::SMALL, theme::medium_family()),
        egui::Color32::PLACEHOLDER,
    );
    let size = egui::vec2(galley.size().x + 34.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, shown, icon));
    widgets::paint_hover(ui, rect, &response, false, theme::radius::CONTROL);
    let (icon_color, text_color) = if shown {
        (color, palette.text)
    } else {
        (palette.text_muted, palette.text_muted)
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(13.0, 0.0),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(13.0),
        icon_color,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 24.0, rect.center().y - galley.size().y / 2.0),
        galley,
        text_color,
    );
    response
}

/// `text`, as a link if it has a script location. Returns the location
/// when it's clicked.
fn text_or_link(
    ui: &mut egui::Ui,
    text: &str,
    location: &Option<Location>,
    font: egui::FontId,
    color: egui::Color32,
) -> Option<Location> {
    match location {
        Some(location) => location_link(ui, text, location, font, color).then(|| location.clone()),
        None => {
            let text = egui::RichText::new(text).font(font).color(color);
            ui.add(egui::Label::new(text).wrap());
            None
        }
    }
}

/// A script error's text, with the `path:line` it starts with underlined
/// like a link, then the message. Returns true when it's clicked.
fn location_link(
    ui: &mut egui::Ui,
    text: &str,
    location: &Location,
    font: egui::FontId,
    color: egui::Color32,
) -> bool {
    let prefix = format!("{}:{}", location.path, location.line);
    let rest = text.strip_prefix(&prefix).unwrap_or(text);
    let mut job = egui::text::LayoutJob::default();
    let mut format = egui::TextFormat::simple(font, color);
    format.underline = egui::Stroke::new(1.0_f32, color);
    job.append(&prefix, 0.0, format.clone());
    format.underline = egui::Stroke::NONE;
    job.append(rest, 0.0, format);
    ui.add(egui::Label::new(job).wrap().sense(egui::Sense::click()))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(format!("Open {} at line {}", location.path, location.line))
        .clicked()
}

/// The script and line at the start of `text`, if it starts with
/// `<path>.lua:<line>:`, as script errors do (see `crates/script/src/errors.rs`),
/// or the engine file and line if it starts with `<path>.rs:<line>:`, as
/// compile errors from a rebuild do (see `rebuild.rs`).
pub fn script_location(text: &str) -> Option<Location> {
    file_location(text, ".lua").or_else(|| file_location(text, ".rs"))
}

/// [`script_location`] for files ending in `extension`.
fn file_location(text: &str, extension: &str) -> Option<Location> {
    let (path, rest) = text.split_once(&format!("{extension}:"))?;
    // A path is one relative file name; this keeps out sentences that
    // happen to contain ".lua:" further along.
    if path.is_empty() || path.contains([':', '\n', '`', '"']) {
        return None;
    }
    let (line, _) = rest.split_once(':')?;
    let line = line.parse().ok().filter(|&line| line > 0)?;
    Some(Location {
        path: format!("{path}{extension}"),
        line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_errors_link_to_their_path_and_line() {
        assert_eq!(
            script_location("scripts/doors/open door.server.lua:12: attempt to index nil"),
            Some(Location {
                path: "scripts/doors/open door.server.lua".to_string(),
                line: 12
            })
        );
        assert_eq!(
            script_location("crates/editor/src/app.rs:12:5: error[E0308]: mismatched types"),
            Some(Location {
                path: "crates/editor/src/app.rs".to_string(),
                line: 12
            })
        );
        for text in [
            "scripts/pad.server.lua won't run: could not read",
            "Could not open `scripts/a.lua:3:`",
            "scripts/pad.server.lua:x: no line",
            "scripts/pad.server.lua:0: no line zero",
            "hello",
        ] {
            assert_eq!(script_location(text), None, "{text}");
        }
    }

    #[test]
    fn warnings_are_visible_but_are_not_script_links() {
        let mut output = Output::default();
        output.warning("ui.css: unsupported CSS property `animation`");

        assert_eq!(output.lines()[0].kind, OutputKind::Warning);
        assert_eq!(output.lines()[0].location, None);
    }

    #[test]
    fn each_call_in_a_stack_trace_links_to_its_script() {
        let mut output = Output::default();
        output.script_error(
            "scripts/Fuse.module.lua:3: boom",
            vec![
                "scripts/Fuse.module.lua:3: in field 'light'".to_string(),
                "scripts/a.server.lua:2: in main chunk".to_string(),
            ],
        );
        let line = &output.lines()[0];
        assert_eq!(line.location.as_ref().unwrap().line, 3);
        let links: Vec<_> = line
            .trace
            .iter()
            .map(|frame| frame.location.clone().unwrap())
            .collect();
        assert_eq!(
            links,
            [
                Location {
                    path: "scripts/Fuse.module.lua".to_string(),
                    line: 3
                },
                Location {
                    path: "scripts/a.server.lua".to_string(),
                    line: 2
                }
            ]
        );
    }

    #[test]
    fn only_errors_are_links() {
        let mut output = Output::default();
        output.info("scripts/a.server.lua:1: printed by a script");
        output.error("scripts/a.server.lua:2: boom");
        assert_eq!(output.lines()[0].location, None);
        assert_eq!(output.lines()[1].location.as_ref().unwrap().line, 2);
    }
}

#[cfg(test)]
mod unseen_tests {
    use super::*;

    fn draw(ctx: &egui::Context, output: &mut Output) {
        let _ = crate::theme::run(ctx, Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                output.show(ui);
            });
        });
    }

    fn hidden_frame(ctx: &egui::Context) {
        let _ = crate::theme::run(ctx, Default::default(), |_ui| {});
    }

    #[test]
    fn errors_while_the_panel_is_hidden_are_counted_until_it_shows() {
        let ctx = theme::test_context();
        let mut output = Output::default();
        output.error("before");
        draw(&ctx, &mut output);
        // Showing: nothing to count, even for a new error.
        output.error("while shown");
        assert_eq!(output.unseen_errors(&ctx), 0);
        draw(&ctx, &mut output);

        // Hidden: errors (not warnings) count up.
        hidden_frame(&ctx);
        hidden_frame(&ctx);
        output.error("a");
        output.warning("w");
        output.error("b");
        assert_eq!(output.unseen_errors(&ctx), 2);
        draw(&ctx, &mut output);
        hidden_frame(&ctx);
        hidden_frame(&ctx);
        assert_eq!(output.unseen_errors(&ctx), 0);
    }

    #[test]
    fn revealing_the_last_error_shows_errors_again_and_marks_it() {
        let ctx = theme::test_context();
        let mut output = Output::default();
        output.error("first");
        output.info("note");
        output.error("last");
        output.hidden.push(OutputKind::Error);
        output.reveal_last_error();
        assert!(!output.hidden.contains(&OutputKind::Error));
        draw(&ctx, &mut output);
        assert_eq!(output.marked, Some(2));
        assert_eq!(output.reveal, None);
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::*;

    /// Draws the panel 900 by 260 and returns the text it drew.
    fn draw(ctx: &egui::Context, output: &mut Output) -> Vec<String> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 260.0),
            )),
            ..Default::default()
        };
        let drawn = crate::theme::run(ctx, input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                output.show(ui);
            });
        });
        let mut texts = Vec::new();
        for clipped in &drawn.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push(text.galley.text().to_owned());
            }
        }
        texts
    }

    #[test]
    fn the_newest_10000_lines_are_kept_and_counted() {
        let mut output = Output::default();
        output.error("first, an error");
        for index in 1..MAX_LINES {
            output.info(format!("line {index}"));
        }
        assert_eq!(output.lines().len(), MAX_LINES);
        assert_eq!(output.counts[OutputKind::Error.index()], 1);
        // The 10,001st line drops the first, and its count.
        output.warning("one too many");
        assert_eq!(output.lines().len(), MAX_LINES);
        assert_eq!(output.lines()[0].text, "line 1");
        assert_eq!(output.counts[OutputKind::Error.index()], 0);
        assert_eq!(output.counts[OutputKind::Warning.index()], 1);
        assert_eq!(output.counts[OutputKind::Info.index()], MAX_LINES - 1);
        assert_eq!(output.dropped, 1);
        // Past a batch, dropped lines leave memory; numbers stay the same.
        let seen = output.total();
        for index in 0..DROP_BATCH + 10 {
            output.info(format!("more {index}"));
        }
        assert!(output.lines.len() < MAX_LINES + DROP_BATCH);
        assert_eq!(output.lines().len(), MAX_LINES);
        assert_eq!(output.since(seen).len(), DROP_BATCH + 10);
        assert_eq!(output.since(seen)[0].text, "more 0");
        assert_eq!(output.total(), MAX_LINES + 1 + DROP_BATCH + 10);
        let counted: usize = output.counts.iter().sum();
        assert_eq!(counted, MAX_LINES);
        // Clear starts the count of dropped lines again; numbers go on.
        output.clear();
        assert!(output.lines().is_empty());
        assert_eq!(output.dropped, 0);
        output.error("after clear");
        assert_eq!(output.since(seen + DROP_BATCH + 10).len(), 1);
    }

    #[test]
    fn only_the_lines_on_screen_are_drawn_newest_at_the_bottom() {
        let ctx = theme::test_context();
        let mut output = Output::default();
        for index in 0..MAX_LINES + 5 {
            output.info(format!("line {index}"));
        }
        let mut texts = Vec::new();
        for _ in 0..3 {
            texts = draw(&ctx, &mut output);
        }
        let lines: Vec<&String> = texts
            .iter()
            .filter(|text| text.starts_with("line "))
            .collect();
        assert!(lines.len() < 40, "{} lines drawn", lines.len());
        let last = format!("line {}", MAX_LINES + 4);
        assert!(lines.iter().any(|text| **text == last), "{lines:?}");
        // With the lines filtered out, the note at the top shows.
        output.hidden.push(OutputKind::Info);
        draw(&ctx, &mut output);
        let texts = draw(&ctx, &mut output);
        assert!(
            texts
                .iter()
                .any(|text| text.starts_with("5 older lines were dropped")),
            "{texts:?}"
        );
    }

    /// Clicking an error's toast reveals it, wherever it is in a long log,
    /// and a long line wraps without the lines after it overlapping.
    #[test]
    fn a_revealed_error_far_up_is_scrolled_to_and_marked() {
        let ctx = theme::test_context();
        let mut output = Output::default();
        for index in 0..3_000 {
            if index == 1_000 {
                output.error("scripts/Spin.server.lua:4: the one to find");
            } else {
                output.info(format!("line {index} {}", "long words ".repeat(index % 30)));
            }
        }
        draw(&ctx, &mut output);
        output.reveal_last_error();
        let mut texts = Vec::new();
        for _ in 0..4 {
            texts = draw(&ctx, &mut output);
        }
        assert!(
            texts.iter().any(|text| text.contains("the one to find")),
            "{texts:?}"
        );
        assert_eq!(output.marked, Some(1_000));
    }

    #[test]
    fn console_log_clips_above_command_bar_and_never_overlaps_on_font_change() {
        let ctx = theme::test_context();
        let mut output = Output::default();
        for index in 0..25 {
            output.info(format!(
                "log entry {index}: a long message that wraps across lines when narrow"
            ));
        }
        let mut command_top = 0.0_f32;
        let mut draw_short = |ctx: &egui::Context, output: &mut Output| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(520.0, 120.0),
                )),
                ..Default::default()
            };
            crate::theme::run(ctx, input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let bottom = egui::Panel::bottom("test console command")
                        .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(0, 4)))
                        .show_separator_line(false)
                        .resizable(false)
                        .show(ui, |ui| {
                            ui.label("> Lua, or a command");
                        });
                    command_top = bottom.response.rect.top();
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show(ui, |ui| {
                            output.show(ui);
                        });
                });
            })
        };
        let _ = draw_short(&ctx, &mut output);
        let appearance = theme::Appearance {
            code_font: theme::CodeFont::Hack,
            code_font_size: 15.0,
            ..Default::default()
        };
        theme::apply(&ctx, &appearance);
        let _ = draw_short(&ctx, &mut output);
        let frame = draw_short(&ctx, &mut output);

        // Every clipped log text shape stays strictly above the command bar.
        let mut log_rows: Vec<(f32, f32)> = Vec::new();
        for clipped in &frame.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let s = text.galley.text();
                if s.starts_with("log entry ") {
                    assert!(
                        clipped.clip_rect.bottom() <= command_top + 0.5,
                        "log clip rect bottom {} ran into command bar top {}",
                        clipped.clip_rect.bottom(),
                        command_top
                    );
                    let top = text.pos.y;
                    let bottom = top + text.galley.size().y;
                    log_rows.push((top, bottom));
                }
            }
        }
        assert!(!log_rows.is_empty(), "expected visible log entries");
        log_rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in log_rows.windows(2) {
            assert!(
                pair[0].1 <= pair[1].0 + 0.5,
                "console rows overlapped: {:?} vs {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}
