//! The viewport bar: a translucent pill at the top left of the Scene View
//! with the toggles people reach for while building, where their eyes
//! already are: the flying speed, Move snapping to the grid, Shaded or
//! Wireframe, and how fast the viewport draws (frames a second and draw
//! calls). Viewport > Viewport Bar hides it.

use eframe::egui;

use super::EditorApp;
use crate::theme::{self, icons};
use crate::viewport::ViewMode;
use axcenity_render::DebugView;

/// How tall the bar is, in points; the legend under it starts below.
pub(crate) const HEIGHT: f32 = 30.0;
/// Its distance from the viewport's edges.
const MARGIN: f32 = 8.0;
/// Narrower than this, the viewport is left clear.
const MIN_VIEWPORT_WIDTH: f32 = 340.0;

/// What the bar measures from one frame to the next.
#[derive(Default)]
pub(super) struct ViewportBar {
    /// When the Scene View was last drawn (egui's clock, in seconds).
    last: Option<f64>,
    /// Frames a second, smoothed.
    fps: f32,
    /// The gap before the last frame was long: the editor was idle, so
    /// there's no frame rate to show.
    idle: bool,
}

impl ViewportBar {
    /// Notes a frame of the Scene View drawn at `now`.
    fn frame(&mut self, now: f64) {
        if let Some(last) = self.last {
            let gap = (now - last) as f32;
            // egui draws only when something changes, so a long gap is
            // the editor waiting, not a slow frame.
            self.idle = gap > 0.25;
            if !self.idle && gap > 0.0 {
                let rate = 1.0 / gap;
                self.fps = if self.fps == 0.0 {
                    rate
                } else {
                    self.fps + (rate - self.fps) * 0.1
                };
            }
        }
        self.last = Some(now);
    }

    /// The frame rate as the bar shows it.
    fn fps_text(&self) -> String {
        if self.idle || self.fps == 0.0 {
            "– FPS".to_owned()
        } else {
            format!("{:.0} FPS", self.fps)
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

impl EditorApp {
    /// Draws the bar over the viewport at `rect`, with the draw calls of
    /// the frame just drawn.
    pub(super) fn viewport_bar(
        &mut self,
        ctx: &egui::Context,
        rect: egui::Rect,
        draw_calls: Option<u32>,
    ) {
        self.viewport_bar_state.frame(ctx.input(|input| input.time));
        if top_inset(self.viewport_bar, rect.width()) == 0.0 {
            return;
        }
        let palette = theme::palette(ctx);
        // An area of its own above the viewport, so the viewport doesn't
        // also take the clicks, as the view mode legend and the ray
        // tracing badge do.
        egui::Area::new(egui::Id::new("viewport bar"))
            .order(egui::Order::Foreground)
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
            "Frames the viewport draws a second, and the GPU draw calls of the last one \
             (shadows included). The editor draws only when something changes, so with \
             nothing moving there's no frame rate. The Profiler has the full figures.",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_rate_is_smoothed_and_idle_gaps_show_none() {
        let mut bar = ViewportBar::default();
        assert_eq!(bar.fps_text(), "– FPS");
        for frame in 0..200 {
            bar.frame(frame as f64 / 60.0);
        }
        assert_eq!(bar.fps_text(), "60 FPS");
        bar.frame(10.0);
        assert_eq!(bar.fps_text(), "– FPS");
    }

    #[test]
    fn the_bar_moves_the_legend_down_only_when_it_shows() {
        assert_eq!(top_inset(true, 800.0), HEIGHT + MARGIN);
        assert_eq!(top_inset(false, 800.0), 0.0);
        assert_eq!(top_inset(true, 200.0), 0.0);
    }
}
