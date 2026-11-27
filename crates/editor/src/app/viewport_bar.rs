//! The viewport bar: a translucent pill at the top left of the Scene View
//! with the toggles people reach for while building, where their eyes
//! already are: the flying speed, Move snapping to the grid, Shaded or
//! Wireframe, and how fast the viewport draws (frames a second and draw
//! calls). Viewport > Viewport Bar hides it.

use std::time::{Duration, Instant};

use eframe::egui;

use super::EditorApp;
use crate::profiler::RenderCost;
use crate::theme::{self, icons};
use crate::viewport::ViewMode;
use axcenity_render::{CpuSection, DebugView};

/// How tall the bar is, in points; the legend and tool panels under it start below.
pub(crate) const HEIGHT: f32 = 30.0;
/// Its distance from the viewport's edges.
const MARGIN: f32 = 8.0;
/// Narrower than this, the viewport is left clear.
const MIN_VIEWPORT_WIDTH: f32 = 340.0;
/// Smoothing weight for frame timings (exponential moving average).
const SMOOTHING: f32 = 0.12;
/// Frames further apart than this weren't drawn one after another: the
/// editor rested between them, and the gap says nothing about its speed.
const REST: Duration = Duration::from_millis(250);
/// How often the frame log prints a line (`AXCENITY_FRAME_LOG`).
const LOG_EVERY: Duration = Duration::from_secs(2);

/// Why the last frame came when it did, which the frame rate alone can't
/// say: a rate the editor holds isn't the scene's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Pace {
    /// As soon as it was ready.
    #[default]
    Free,
    /// Ready sooner, and held to the frame rate limit someone chose, this
    /// many a second (`EditorApp::pace_frame`).
    Held(u32),
    /// Nothing asked for it at once: it came on a timer (scenery that
    /// moves on its own, 30 times a second) or after a rest.
    OnDemand,
}

/// What the bar measures from each rendered viewport frame.
///
/// The frame rate is the rate frames are shown at: the time from one
/// viewport frame to the next while they come one after another. It used
/// to be worked out from the viewport's CPU and GPU times
/// (`1000 / max(cpu, gpu)`), which is what a renderer whose CPU and GPU
/// work fully overlap could reach and leaves out the rest of the editor's
/// frame: it read 41 where 24 frames a second were shown. Those times are
/// in the tooltip, as what the frame is made of.
#[derive(Default)]
pub(super) struct ViewportBar {
    /// When the last viewport frame was recorded.
    last_frame: Option<Instant>,
    /// Smoothed time from one frame to the next, in milliseconds.
    shown_ms: Option<f32>,
    /// Smoothed CPU time (viewport draw list + render submission + Play step), in ms.
    cpu_ms: f32,
    /// Smoothed GPU render time from hardware timestamps, in ms, when available.
    gpu_ms: Option<f32>,
    /// Smoothed CPU time of the whole editor frame (panels and viewport), in ms.
    editor_ms: Option<f32>,
    /// True once at least one rendered frame has been recorded.
    has_sample: bool,
    /// The frames since the frame log's last line.
    log: Option<LogWindow>,
    /// How long the viewport's last frame waited for an earlier one's GPU
    /// work, until [`ViewportBar::take_gpu_wait`] asks.
    gpu_wait: Duration,
    /// The viewport's size in pixels, as last drawn, for the frame log.
    pixels: (u32, u32),
    /// Why the last frame came when it did.
    pace: Pace,
    /// The share of late frames that were held or on demand, smoothed, so
    /// the bar's note doesn't flicker while frames straddle the grid.
    paced_share: f32,
}

/// What the frame log adds up between two of its lines.
struct LogWindow {
    since: Instant,
    frames: u32,
    cpu_ms: f32,
    play_ms: f32,
    gpu_ms: f32,
    gpu_frames: u32,
    /// The viewport's drawing, section by section (`CpuSection::ALL`).
    sections_ms: [f32; CpuSection::COUNT],
    /// The newest frame's counts: items drawn, draw calls, and the items
    /// in the ray traced scene.
    counts: (u32, u32, u32),
}

impl LogWindow {
    fn starting(since: Instant) -> Self {
        Self {
            since,
            frames: 0,
            cpu_ms: 0.0,
            play_ms: 0.0,
            gpu_ms: 0.0,
            gpu_frames: 0,
            sections_ms: [0.0; CpuSection::COUNT],
            counts: (0, 0, 0),
        }
    }
}

fn smooth(value: &mut f32, new: f32) {
    *value += (new - *value) * SMOOTHING;
}

impl ViewportBar {
    /// Records a viewport frame drawn at `now`: when it came, and the CPU
    /// and GPU work in it.
    fn record(&mut self, render: Option<RenderCost>, play_cpu: Duration, now: Instant) {
        let Some(render) = render else {
            return;
        };
        if let Some(last) = self.last_frame.replace(now) {
            let between = now.saturating_duration_since(last);
            if between <= REST {
                let ms = between.as_secs_f32() * 1000.0;
                match &mut self.shown_ms {
                    Some(shown) => smooth(shown, ms),
                    None => self.shown_ms = Some(ms),
                }
            }
        }
        self.gpu_wait = render.stats.gpu_wait;
        let render_ms = render.cpu.as_secs_f32() * 1000.0;
        let play_ms = play_cpu.as_secs_f32() * 1000.0;
        let cpu_ms = render_ms + play_ms;
        let gpu_ms = render.stats.gpu_time.map(|t| t.as_secs_f32() * 1000.0);
        match self.log.as_mut() {
            Some(log) => {
                log.frames += 1;
                log.cpu_ms += render_ms;
                log.play_ms += play_ms;
                if let Some(gpu) = gpu_ms {
                    log.gpu_ms += gpu;
                    log.gpu_frames += 1;
                }
                log.counts = (
                    render.stats.items_drawn,
                    render.stats.draw_calls,
                    render.stats.ray_tracing.instances,
                );
                for (sum, time) in log.sections_ms.iter_mut().zip(render.stats.cpu_times) {
                    *sum += time.as_secs_f32() * 1000.0;
                }
            }
            None => self.log = Some(LogWindow::starting(now)),
        }
        if !self.has_sample {
            self.cpu_ms = cpu_ms;
            self.gpu_ms = gpu_ms;
            self.has_sample = true;
            return;
        }
        smooth(&mut self.cpu_ms, cpu_ms);
        if let Some(gpu) = gpu_ms {
            match &mut self.gpu_ms {
                Some(smoothed) => smooth(smoothed, gpu),
                None => self.gpu_ms = Some(gpu),
            }
        }
    }

    /// Notes how long the whole editor frame took on the CPU: every panel
    /// and the viewport, without the wait to present it.
    pub(super) fn note_editor_frame(&mut self, cpu: Duration) {
        let ms = cpu.as_secs_f32() * 1000.0;
        match &mut self.editor_ms {
            Some(smoothed) => smooth(smoothed, ms),
            None => self.editor_ms = Some(ms),
        }
    }

    /// How long the viewport frame recorded since the last call waited for
    /// an earlier frame's GPU work: zero if none was drawn.
    pub(super) fn take_gpu_wait(&mut self) -> Duration {
        std::mem::take(&mut self.gpu_wait)
    }

    /// Notes why the frame just built comes when it does.
    pub(super) fn note_pace(&mut self, pace: Pace) {
        let paced = if pace == Pace::Free { 0.0 } else { 1.0 };
        smooth(&mut self.paced_share, paced);
        // The kind of the last frame that wasn't free: what the note says.
        if pace != Pace::Free || self.paced_share < 0.05 {
            self.pace = pace;
        }
    }

    /// What holds the frame rate, if most late frames were held.
    pub(super) fn held_by(&self) -> Pace {
        if self.paced_share > 0.5 {
            self.pace
        } else {
            Pace::Free
        }
    }

    /// The slower of the editor's CPU frame and the GPU's: what one frame
    /// takes when nothing holds it and the two work at once.
    fn work_ms(&self) -> f32 {
        let cpu = self.editor_ms.unwrap_or(self.cpu_ms).max(self.cpu_ms);
        cpu.max(self.gpu_ms.unwrap_or(0.0))
    }

    /// Frames a second as shown, once two frames came one after another.
    fn fps(&self) -> Option<f32> {
        self.shown_ms.filter(|ms| *ms > 0.0).map(|ms| 1000.0 / ms)
    }

    /// The frame rate as the bar shows it.
    fn fps_text(&self) -> String {
        let note = match self.held_by() {
            Pace::Free => "",
            Pace::Held(_) => " (held)",
            Pace::OnDemand => " (idle)",
        };
        match self.fps() {
            Some(fps) => format!("{fps:.0} FPS{note}"),
            None => "– FPS".to_owned(),
        }
    }

    /// Hover tooltip: the rate, what the frame is made of, and the
    /// editor's pacing.
    fn fps_tooltip(&self, steady_hz: u32) -> String {
        let pace = if steady_hz > 0 {
            format!(
                " While you work, the editor holds frames to the {steady_hz} Hz limit you chose \
                 (Settings > Accessibility > Frame rate limit); scenery that moves on its own is \
                 drawn 30 times a second while you don't."
            )
        } else {
            " No frame rate limit is set: this is the real rate, at most your display's \
             refresh. Scenery that moves on its own is drawn 30 times a second while you \
             don't touch the editor."
                .to_owned()
        };
        let Some(shown) = self.shown_ms else {
            return format!(
                "Frames a second as the viewport shows them, and the GPU draw calls of the \
                 last frame (shadows included).{pace} The Profiler has the full figures."
            );
        };
        let gpu = match self.gpu_ms {
            Some(gpu) => format!("GPU {gpu:.1} ms"),
            None => "GPU time unavailable on this adapter".to_owned(),
        };
        let editor = match self.editor_ms {
            Some(editor) => format!(", the whole editor frame {editor:.1} ms on the CPU"),
            None => String::new(),
        };
        let without = 1000.0 / self.work_ms().max(0.05);
        // What holds the rate goes first, and replaces the general note.
        let (held, pace) = match self.held_by() {
            Pace::Free => (String::new(), pace),
            Pace::Held(hz) => (
                format!(
                    " This rate is held to the {hz} Hz limit you chose, not the scene's: \
                     each frame is ready sooner (about {without:.0} a second without it). \
                     Settings > Accessibility > Frame rate limit changes it."
                ),
                String::new(),
            ),
            Pace::OnDemand => (
                format!(
                    " This is the editor resting, not the scene's rate: scenery that moves on \
                     its own is drawn 30 times a second while you don't touch the editor \
                     (about {without:.0} a second when you do)."
                ),
                String::new(),
            ),
        };
        format!(
            "Frames a second as shown: {shown:.1} ms between frames.{held} The viewport's work \
             in one: CPU {:.1} ms, {gpu}{editor}. The draw calls are the last frame's (shadows \
             included).{pace} The Profiler has the full figures.",
            self.cpu_ms
        )
    }

    /// The frame log's line for the frames since its last one, once
    /// [`LOG_EVERY`] has passed (`AXCENITY_FRAME_LOG`): how many frames a
    /// second were shown and the mean work in one.
    fn log_line(&mut self, now: Instant) -> Option<String> {
        let log = self.log.as_ref()?;
        let elapsed = now.saturating_duration_since(log.since);
        if elapsed < LOG_EVERY || log.frames == 0 {
            return None;
        }
        let frames = log.frames as f32;
        let gpu = if log.gpu_frames > 0 {
            format!("{:.1} ms", log.gpu_ms / log.gpu_frames as f32)
        } else {
            "untimed".to_owned()
        };
        let editor = self
            .editor_ms
            .map_or("untimed".to_owned(), |ms| format!("{ms:.1} ms"));
        let line = format!(
            "frames: {:.1} a second ({:.1} ms between frames); viewport CPU {:.1} ms, Play \
             {:.1} ms, the whole editor frame {editor}, GPU {gpu}",
            frames / elapsed.as_secs_f32(),
            elapsed.as_secs_f32() * 1000.0 / frames,
            log.cpu_ms / frames,
            log.play_ms / frames,
        );
        // Then where the viewport's drawing went: every section over a
        // twentieth of a millisecond a frame, in the order they run.
        let sections: Vec<String> = CpuSection::ALL
            .into_iter()
            .zip(log.sections_ms)
            .map(|(section, sum)| (section, sum / frames))
            .filter(|(_, ms)| *ms >= 0.05)
            .map(|(section, ms)| format!("{} {ms:.2}", section.name()))
            .collect();
        let line = if sections.is_empty() {
            line
        } else {
            let (items, draws, traced) = log.counts;
            let (width, height) = self.pixels;
            format!(
                "{line}\n  drawing, ms: {}\n  {items} items drawn in {draws} draw calls, {traced} traced, at {width} x {height}",
                sections.join(", ")
            )
        };
        self.log = Some(LogWindow::starting(now));
        Some(line)
    }
}

/// Whether `AXCENITY_FRAME_LOG` asks for the frame log: frames drawn one
/// after another whether or not anything changed, and a line every two
/// seconds on standard error with the frame rate as shown and the work in
/// a frame. For timing the editor itself (`docs/benchmarks.md`).
pub(super) fn frame_log_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("AXCENITY_FRAME_LOG").is_some())
}

/// The bar's height and margin, if it's shown over a viewport `width`
/// points wide: what's drawn at the top left under it moves down this far.
pub(crate) fn top_inset(shown: bool, width: f32) -> f32 {
    if shown && width >= MIN_VIEWPORT_WIDTH {
        HEIGHT + MARGIN
    } else {
        0.0
    }
}

/// The viewport area below the viewport bar, where top-left overlays (the
/// view-mode legend, building tool panels) sit without covering the bar.
pub(crate) fn below_bar(rect: egui::Rect, shown: bool) -> egui::Rect {
    let inset = top_inset(shown, rect.width());
    egui::Rect::from_min_max(rect.min + egui::vec2(0.0, inset), rect.max)
}

impl EditorApp {
    /// Draws the bar over the viewport at `rect`, with the frame timings and
    /// draw calls of the frame just drawn.
    pub(super) fn viewport_bar(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        order: egui::Order,
        render: Option<RenderCost>,
        play_cpu: Duration,
    ) {
        let now = Instant::now();
        let pixels = rect.size() * ctx.pixels_per_point();
        self.viewport_bar_state.pixels = (pixels.x.round() as u32, pixels.y.round() as u32);
        self.viewport_bar_state.record(render, play_cpu, now);
        if frame_log_on()
            && let Some(line) = self.viewport_bar_state.log_line(now)
        {
            eprintln!("{line}");
        }
        let draw_calls = render.map(|cost| cost.stats.draw_calls);
        if top_inset(self.viewport_bar, rect.width()) == 0.0 {
            return;
        }
        let palette = theme::palette(ctx);
        // An area of its own above the viewport, so the viewport doesn't
        // also take the clicks, on the viewport's own order (Background when
        // docked) so tool windows such as Settings always paint over it.
        egui::Area::new(egui::Id::new("viewport bar"))
            .order(order)
            .fixed_pos(rect.left_top() + egui::vec2(MARGIN, MARGIN))
            .constrain_to(rect)
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.elevated.gamma_multiply(0.86))
                    .stroke(egui::Stroke::new(1.0_f32, palette.border))
                    .corner_radius((HEIGHT / 2.0) as u8)
                    .inner_margin(egui::Margin::symmetric(8, 2))
                    .show(ui, |ui| {
                        ui.set_height(HEIGHT - 6.0);
                        ui.horizontal_centered(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.spacing_mut().interact_size.y = HEIGHT - 8.0;
                            self.viewport_bar_items(ui, draw_calls);
                        });
                    });
            });
    }

    fn viewport_bar_items(&mut self, ui: &mut egui::Ui, draw_calls: Option<u32>) {
        let palette = theme::palette(ui.ctx());
        let divider = |ui: &mut egui::Ui| {
            ui.add(egui::Separator::default().vertical().spacing(8.0));
        };

        // The flying speed, which Shift+wheel also changes.
        ui.label(egui::RichText::new(icons::VIDEO_CAMERA).color(palette.text_secondary))
            .on_hover_text("Camera speed");
        let mut speed = self.viewport.camera_speed();
        let drag = ui
            .add(
                egui::DragValue::new(&mut speed)
                    .range(1.0..=500.0)
                    .speed(0.5)
                    .max_decimals(1)
                    .suffix(" studs/s"),
            )
            .on_hover_text(
                "How fast the camera flies, in studs a second. Drag or type; Shift+wheel in \
                 the viewport changes it too.",
            );
        if drag.changed() {
            self.viewport.set_camera_speed(speed);
        }
        divider(ui);

        // Move snapping, the Snapping group's Move.
        let step = self.snap.move_step;
        let tip = format!(
            "Grid snap: move and scale in steps of {step} studs (the Home tab's Snapping \
             sets the step)."
        );
        if crate::widgets::icon_button(ui, icons::GRID_FOUR, &tip, self.snap.move_on).clicked() {
            self.snap.move_on = !self.snap.move_on;
        }
        divider(ui);

        // Shaded or Wireframe; the other modes are in Viewport > View Mode.
        let wireframe = ViewMode::Debug(DebugView::Wireframe);
        let shaded = crate::widgets::icon_button(
            ui,
            icons::CUBE,
            "Shaded: the scene as the game draws it",
            self.view_mode == ViewMode::Lit,
        );
        if shaded.clicked() {
            self.view_mode = ViewMode::Lit;
        }
        let lines = crate::widgets::icon_button(
            ui,
            icons::CUBE_TRANSPARENT,
            "Wireframe: every triangle's edges",
            self.view_mode == wireframe,
        );
        if lines.clicked() {
            self.view_mode = if self.view_mode == wireframe {
                ViewMode::Lit
            } else {
                wireframe
            };
        }
        divider(ui);

        // How fast it draws.
        let calls = draw_calls.map_or("–".to_owned(), |calls| calls.to_string());
        let text = format!("{}  ·  {calls} draws", self.viewport_bar_state.fps_text());
        ui.label(
            egui::RichText::new(text)
                .size(theme::type_scale::SMALL)
                .color(palette.text_secondary),
        )
        .on_hover_text(
            self.viewport_bar_state
                .fps_tooltip(self.appearance.steady_hz),
        );
    }
}

#[cfg(test)]
mod tests {
    use axcenity_render::RenderStats;

    use super::*;

    fn cost(cpu_ms: f32, gpu_ms: Option<f32>) -> RenderCost {
        RenderCost {
            cpu: Duration::from_secs_f32(cpu_ms / 1000.0),
            lights: 0,
            stats: RenderStats {
                gpu_time: gpu_ms.map(|ms| Duration::from_secs_f32(ms / 1000.0)),
                gpu_timing: gpu_ms.is_some(),
                ..RenderStats::default()
            },
        }
    }

    /// Records `count` frames `apart_ms` after one another, each with
    /// `cost`, and returns when the last one was.
    fn frames(
        bar: &mut ViewportBar,
        from: Instant,
        count: u32,
        apart_ms: u64,
        cost: RenderCost,
    ) -> Instant {
        let mut at = from;
        for _ in 0..count {
            at += Duration::from_millis(apart_ms);
            bar.record(Some(cost), Duration::ZERO, at);
        }
        at
    }

    #[test]
    fn the_frame_rate_is_the_rate_frames_are_shown_at() {
        let mut bar = ViewportBar::default();
        assert_eq!(bar.fps_text(), "– FPS");
        // 14 ms on the CPU and 24 ms on the GPU, one after the other and
        // then a wait for the display: 40 ms from frame to frame is 25
        // frames a second, whatever the larger of the two would allow.
        let start = Instant::now();
        frames(&mut bar, start, 60, 40, cost(14.0, Some(24.0)));
        assert_eq!(bar.fps_text(), "25 FPS");
        let tooltip = bar.fps_tooltip(0);
        assert!(tooltip.contains("40.0 ms between frames"), "{tooltip}");
        assert!(tooltip.contains("CPU 14.0 ms"), "{tooltip}");
        assert!(tooltip.contains("GPU 24.0 ms"), "{tooltip}");
    }

    #[test]
    fn one_frame_alone_has_no_rate_yet() {
        let mut bar = ViewportBar::default();
        bar.record(Some(cost(1.0, Some(5.0))), Duration::ZERO, Instant::now());
        assert_eq!(bar.fps_text(), "– FPS");
    }

    #[test]
    fn a_pause_between_frames_is_not_a_slow_frame() {
        // The editor draws when something happens: the gap before a frame
        // drawn after a rest says nothing about how fast frames come.
        let mut bar = ViewportBar::default();
        let at = frames(&mut bar, Instant::now(), 60, 10, cost(2.0, Some(3.0)));
        assert_eq!(bar.fps_text(), "100 FPS");
        let later = at + Duration::from_secs(3);
        bar.record(Some(cost(2.0, Some(3.0))), Duration::ZERO, later);
        assert_eq!(bar.fps_text(), "100 FPS");
        // Frames coming again at another pace are followed.
        frames(&mut bar, later, 80, 20, cost(2.0, Some(3.0)));
        assert_eq!(bar.fps_text(), "50 FPS");
    }

    #[test]
    fn the_tooltip_says_what_the_whole_editor_frame_costs() {
        let mut bar = ViewportBar::default();
        for _ in 0..60 {
            bar.note_editor_frame(Duration::from_millis(30));
        }
        frames(&mut bar, Instant::now(), 60, 50, cost(14.0, Some(24.0)));
        let tooltip = bar.fps_tooltip(60);
        assert!(
            tooltip.contains("the whole editor frame 30.0 ms"),
            "{tooltip}"
        );
        assert!(tooltip.contains("60 Hz"), "{tooltip}");
    }

    #[test]
    fn a_frame_rate_held_to_the_steady_grid_says_so() {
        // 9 ms of work a frame, shown every 16.7 ms because someone chose a
        // 60 Hz frame rate limit: the bar must not
        // read as if the scene could only reach 60.
        let mut bar = ViewportBar::default();
        let mut at = Instant::now();
        for _ in 0..90 {
            at += Duration::from_micros(16_667);
            bar.note_editor_frame(Duration::from_millis(10));
            bar.note_pace(Pace::Held(60));
            bar.record(Some(cost(9.0, Some(6.0))), Duration::ZERO, at);
        }
        assert_eq!(bar.fps_text(), "60 FPS (held)");
        let tooltip = bar.fps_tooltip(60);
        assert!(tooltip.contains("held to the 60 Hz limit"), "{tooltip}");
        assert!(!tooltip.contains("  "), "no runs of spaces: {tooltip}");
        assert!(tooltip.contains("Frame rate limit"), "{tooltip}");
        // The slower of the editor's CPU frame and the GPU's: 10 ms.
        assert!(tooltip.contains("about 100 a second without"), "{tooltip}");
        // Frames that stop being held stop saying so.
        for _ in 0..90 {
            at += Duration::from_millis(25);
            bar.note_pace(Pace::Free);
            bar.record(Some(cost(20.0, Some(6.0))), Duration::ZERO, at);
        }
        assert_eq!(bar.fps_text(), "40 FPS");
    }

    #[test]
    fn with_no_limit_the_bar_reads_the_real_rate_unmarked() {
        // Paul's report: "60 FPS (held)" with the editor focused, 120 with
        // it in the background. With no frame rate limit (the default)
        // frames are never held, so the bar reads what's shown, unmarked,
        // and the tooltip doesn't talk of a hold.
        let mut bar = ViewportBar::default();
        let mut at = Instant::now();
        for _ in 0..120 {
            at += Duration::from_micros(8_333);
            bar.note_editor_frame(Duration::from_millis(5));
            bar.note_pace(Pace::Free);
            bar.record(Some(cost(5.0, Some(4.0))), Duration::ZERO, at);
        }
        assert_eq!(bar.fps_text(), "120 FPS");
        let tooltip = bar.fps_tooltip(0);
        assert!(tooltip.contains("No frame rate limit is set"), "{tooltip}");
        assert!(!tooltip.contains("held"), "{tooltip}");
        assert!(!tooltip.contains("  "), "no runs of spaces: {tooltip}");
    }

    #[test]
    fn frames_drawn_on_a_timer_say_the_editor_is_idle() {
        // Nobody touches the editor; the wind moves the plants, drawn 30
        // times a second to spare the GPU.
        let mut bar = ViewportBar::default();
        let mut at = Instant::now();
        for _ in 0..90 {
            at += Duration::from_micros(33_333);
            bar.note_pace(Pace::OnDemand);
            bar.record(Some(cost(9.0, Some(6.0))), Duration::ZERO, at);
        }
        assert_eq!(bar.fps_text(), "30 FPS (idle)");
        assert!(
            bar.fps_tooltip(60)
                .contains("while you don't touch the editor")
        );
    }

    #[test]
    fn the_frame_log_reports_every_two_seconds() {
        let mut bar = ViewportBar::default();
        let start = Instant::now();
        let mut lines = Vec::new();
        let mut at = start;
        for _ in 0..120 {
            at += Duration::from_millis(40);
            bar.record(Some(cost(14.0, Some(24.0))), Duration::ZERO, at);
            lines.extend(bar.log_line(at));
        }
        // 4.8 seconds of frames: a line at two seconds and one at four.
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with("frames: 25.0 a second"), "{lines:?}");
        assert!(lines[0].contains("viewport CPU 14.0 ms"), "{lines:?}");
        assert!(lines[0].contains("GPU 24.0 ms"), "{lines:?}");
        assert!(!lines[0].contains("drawing, ms"), "no sections were timed");
    }

    #[test]
    fn the_frame_log_says_where_the_viewports_drawing_went() {
        let mut bar = ViewportBar::default();
        let mut with_sections = cost(14.0, Some(24.0));
        with_sections.stats.cpu_times[CpuSection::Plan as usize] = Duration::from_millis(3);
        with_sections.stats.cpu_times[CpuSection::RayScene as usize] = Duration::from_micros(2500);
        // Too small to be worth a place in the line.
        with_sections.stats.cpu_times[CpuSection::Posing as usize] = Duration::from_micros(10);
        let mut at = Instant::now();
        let mut lines = Vec::new();
        for _ in 0..60 {
            at += Duration::from_millis(40);
            bar.record(Some(with_sections), Duration::ZERO, at);
            lines.extend(bar.log_line(at));
        }
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].contains("\n  drawing, ms: Culling and batches 3.00, Ray traced scene 2.50\n"),
            "{lines:?}"
        );
    }

    #[test]
    fn the_bar_moves_the_legend_down_only_when_it_shows() {
        assert_eq!(top_inset(true, 800.0), HEIGHT + MARGIN);
        assert_eq!(top_inset(false, 800.0), 0.0);
        assert_eq!(top_inset(true, 200.0), 0.0);

        let viewport = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(800.0, 600.0));
        let bar_bottom = viewport.min.y + MARGIN + HEIGHT;
        let below = below_bar(viewport, true);
        assert!(below.min.y >= bar_bottom);
        assert_eq!(below_bar(viewport, false), viewport);
    }
}
