//! The viewport bar: a translucent pill at the top left of the Scene View
//! with the toggles people reach for while building, where their eyes
//! already are: the flying speed, Move snapping to the grid, Shaded or
//! Wireframe, and how fast the viewport draws (frames a second and draw
//! calls). Viewport > Viewport Bar hides it.

use std::time::Duration;

use eframe::egui;

use super::EditorApp;
use crate::profiler::RenderCost;
use crate::theme::{self, icons};
use crate::viewport::ViewMode;
use axcenity_render::DebugView;

/// How tall the bar is, in points; the legend and tool panels under it start below.
pub(crate) const HEIGHT: f32 = 30.0;
/// Its distance from the viewport's edges.
const MARGIN: f32 = 8.0;
/// Narrower than this, the viewport is left clear.
const MIN_VIEWPORT_WIDTH: f32 = 340.0;
/// Smoothing weight for frame timings (exponential moving average).
const SMOOTHING: f32 = 0.12;
/// Floor for frame duration in ms so synthetic zero-time frames don't divide by zero.
const MIN_FRAME_MS: f32 = 0.05;

/// What the bar measures from each rendered viewport frame.
///
/// The editor paces presentation onto a fixed grid (`steady_hz`) and only
/// repaints on demand, so wall-clock intervals between egui frames reflect
/// pacing and idle gaps rather than actual performance. Instead, this tracks
/// the real pipelined frame cost (`max(cpu, gpu)`), so the FPS reading
/// represents what the scene can actually sustain uncapped.
#[derive(Default)]
pub(super) struct ViewportBar {
    /// Smoothed bottleneck frame time (`max(cpu, gpu)`), in milliseconds.
    frame_ms: f32,
    /// Smoothed CPU time (viewport draw list + render submission + Play step), in ms.
    cpu_ms: f32,
    /// Smoothed GPU render time from hardware timestamps, in ms, when available.
    gpu_ms: Option<f32>,
    /// True once at least one rendered frame has been recorded.
    has_sample: bool,
}

impl ViewportBar {
    /// Records the actual CPU and GPU work of a rendered viewport frame.
    fn record(&mut self, render: Option<RenderCost>, play_cpu: Duration) {
        let Some(render) = render else {
            return;
        };
        let cpu_ms = (render.cpu + play_cpu).as_secs_f32() * 1000.0;
        let gpu_ms = render.stats.gpu_time.map(|t| t.as_secs_f32() * 1000.0);
        let bottleneck_ms = match gpu_ms {
            Some(gpu) => cpu_ms.max(gpu),
            None => cpu_ms,
        };
        let first_gpu = gpu_ms.is_some() && self.gpu_ms.is_none();
        if !self.has_sample {
            self.cpu_ms = cpu_ms;
            self.gpu_ms = gpu_ms;
            self.frame_ms = bottleneck_ms;
            self.has_sample = true;
            return;
        }
        self.cpu_ms += (cpu_ms - self.cpu_ms) * SMOOTHING;
        if let Some(gpu) = gpu_ms {
            self.gpu_ms = Some(match self.gpu_ms {
                Some(prev) => prev + (gpu - prev) * SMOOTHING,
                None => gpu,
            });
        }
        // Snap once when the first asynchronous GPU timestamp arrives so we
        // don't slowly decay from an earlier CPU-only reading.
        if first_gpu {
            self.frame_ms = bottleneck_ms;
        } else {
            self.frame_ms += (bottleneck_ms - self.frame_ms) * SMOOTHING;
        }
    }

    /// Uncapped frames per second implied by the smoothed frame work.
    fn fps(&self) -> Option<f32> {
        if !self.has_sample || self.frame_ms <= 0.0 {
            return None;
        }
        Some(1000.0 / self.frame_ms.max(MIN_FRAME_MS))
    }

    /// The frame rate as the bar shows it.
    fn fps_text(&self) -> String {
        match self.fps() {
            Some(fps) => format!("{fps:.0} FPS"),
            None => "– FPS".to_owned(),
        }
    }

    /// Hover tooltip detailing the CPU/GPU breakdown and editor pacing.
    fn fps_tooltip(&self, steady_hz: u32) -> String {
        let pace = if steady_hz > 0 {
            format!(
                " The editor display itself is paced at {steady_hz} Hz \
                 (Settings > Accessibility > Steady frame rate)."
            )
        } else {
            String::new()
        };
        if !self.has_sample {
            return format!(
                "Uncapped frame rate estimated from the viewport's CPU and GPU work, and the \
                 GPU draw calls of the last frame (shadows included).{pace} The Profiler has \
                 the full figures."
            );
        }
        match self.gpu_ms {
            Some(gpu) => format!(
                "Uncapped frame rate from the viewport's work ({:.1} ms/frame: CPU {:.1} ms, \
                 GPU {:.1} ms), and the GPU draw calls of the last frame (shadows \
                 included).{pace} The Profiler has the full figures.",
                self.frame_ms, self.cpu_ms, gpu
            ),
            None => format!(
                "Uncapped frame rate from the viewport's CPU work ({:.1} ms/frame; GPU \
                 timestamps unavailable on this adapter), and the GPU draw calls of the last \
                 frame (shadows included).{pace} The Profiler has the full figures.",
                self.frame_ms
            ),
        }
    }
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
        self.viewport_bar_state.record(render, play_cpu);
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

    #[test]
    fn the_frame_rate_reflects_actual_cpu_and_gpu_work_not_display_pacing() {
        let mut bar = ViewportBar::default();
        assert_eq!(bar.fps_text(), "– FPS");
        // 4 ms GPU time with 1.5 ms CPU time -> pipelined bottleneck is 4 ms -> 250 FPS,
        // even if the editor display is paced to 60 Hz.
        for _ in 0..30 {
            bar.record(Some(cost(1.5, Some(4.0))), Duration::ZERO);
        }
        assert_eq!(bar.fps_text(), "250 FPS");
        // Adding 6 ms of Play simulation makes CPU (7.5 ms) the bottleneck -> ~133 FPS.
        for _ in 0..80 {
            bar.record(
                Some(cost(1.5, Some(4.0))),
                Duration::from_secs_f32(6.0 / 1000.0),
            );
        }
        assert_eq!(bar.fps_text(), "133 FPS");
    }

    #[test]
    fn first_gpu_timestamp_snaps_frame_time_immediately() {
        let mut bar = ViewportBar::default();
        // Before the first GPU timestamp comes back, CPU-only says 1000 FPS (1 ms).
        bar.record(Some(cost(1.0, None)), Duration::ZERO);
        assert_eq!(bar.fps_text(), "1000 FPS");
        // Once the first GPU reading (5 ms) arrives, FPS reflects it right away (200 FPS).
        bar.record(Some(cost(1.0, Some(5.0))), Duration::ZERO);
        assert_eq!(bar.fps_text(), "200 FPS");
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
