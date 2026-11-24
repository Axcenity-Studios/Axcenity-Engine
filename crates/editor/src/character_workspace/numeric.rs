//! The studio's one numeric-drag helper (RA1; Paul's drag rule of
//! 2026-11-18). Every numeric customization control uses it: the body's
//! and the face's slider rows, the Skin page's values, makeup and
//! markings (`panel/row.rs`), and the handles on the character
//! (`direct.rs`). Paul's rule is **consistency**: the same drag does the
//! same to any of them.
//!
//! - **A whole range takes [`FULL_RANGE`] points of drag** (110, inside
//!   Paul's 100 to 120): horizontally for a slider row or its value,
//!   however wide the row is; along the handle's direction on the screen
//!   for a handle, however far the camera is ([`points_per_unit`]). The
//!   drag is relative: a press on a track doesn't move the value to the
//!   pointer, and the value stops at the range's ends while the pointer
//!   goes on.
//! - **Shift is fine** ([`FINE`], a quarter) and **Ctrl coarse**
//!   ([`COARSE`], double); both held, the product (half). Ctrl is egui's
//!   `command`: Ctrl on Windows and Linux, Cmd on macOS (where Ctrl with a
//!   click is the right button). The studio's own Ctrl shortcuts (Z, Y,
//!   Shift+Z, S) are keys, not drags, so they don't clash. A modifier
//!   pressed partway changes the speed from there on ([`Travel`]): the
//!   value never jumps.
//! - **A drag is one undo step**: the row sends [`Gesture::Drag`] while it
//!   moves and [`Gesture::Commit`] when let go. **Esc cancels it** back to
//!   where it started (the shell's [`Intent::CancelEdit`], and the row's
//!   drag does nothing more until let go).
//! - **Exact values are typed** ([`field`]): a click or a double click on
//!   a row's value opens it for typing; Enter (or clicking away) sets it as
//!   one undo step, Esc leaves it as it was. A value outside the range is
//!   kept to the range and the row says so for a few seconds; text that
//!   isn't a number changes nothing and says so.
//!
//! A control added later (the Expressions page's, a browser's, an item's
//! own parameters) draws its number with [`track`] and [`field`] (or the
//! panel's slider row, which uses them), and a drag elsewhere goes through
//! [`NumberDrag`] or [`Travel`] and [`points_per_unit`].
//!
//! [`Intent::CancelEdit`]: super::intent::Intent::CancelEdit

use std::ops::{Add, Mul};

use eframe::egui;

use super::intent::Gesture;
use crate::theme;

/// Points of drag for a whole range, at normal speed.
pub const FULL_RANGE: f32 = 110.0;
/// Shift: a quarter of the speed.
pub const FINE: f32 = 0.25;
/// Ctrl (Cmd on macOS): twice the speed.
pub const COARSE: f32 = 2.0;
/// How long a row's note ("kept to the range") stays, in seconds.
const NOTE_SECONDS: f64 = 4.0;
/// How wide a row's value is drawn ([`field`]), shown or typed in.
pub const FIELD_WIDTH: f32 = 48.0;

/// The speed `modifiers` ask for: 1, a quarter with Shift, double with
/// Ctrl (egui's `command`, Cmd on macOS).
pub fn speed(modifiers: egui::Modifiers) -> f32 {
    let mut speed = 1.0;
    if modifiers.shift {
        speed *= FINE;
    }
    if modifiers.command {
        speed *= COARSE;
    }
    speed
}

/// Points of drag per unit of value for a parameter of `range`, so its
/// whole range takes [`FULL_RANGE`] points.
pub fn points_per_unit(range: [f32; 2]) -> f32 {
    FULL_RANGE / span(range).max(1.0e-6)
}

fn span(range: [f32; 2]) -> f32 {
    (range[1] - range[0]).abs()
}

/// `range`'s ends, lower first.
fn ends(range: [f32; 2]) -> [f32; 2] {
    [range[0].min(range[1]), range[0].max(range[1])]
}

fn clamp(value: f32, range: [f32; 2]) -> f32 {
    let [low, high] = ends(range);
    value.clamp(low, high)
}

/// `value` written as a row shows it: two decimals, or three where the
/// range's ends need them (the waist's 1.175), so an end reads as it is.
pub fn shown(value: f32, range: [f32; 2]) -> String {
    let exact = |end: f32| ((end * 100.0).round() / 100.0 - end).abs() < 1.0e-5;
    if range.into_iter().all(exact) {
        format!("{value:.2}")
    } else {
        format!("{value:.3}")
    }
}

/// How far a drag has gone, in points: `raw` as the pointer went (for
/// thresholds such as a handle's dead zone) and `scaled`, each step times
/// the speed its modifiers asked for then (what moves the value).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Travel<V> {
    pub raw: V,
    pub scaled: V,
}

// `V` is a number (a slider's horizontal drag) or a 2D vector (a handle's):
// anything that adds and scales.
impl<V: Copy + Add<Output = V> + Mul<f32, Output = V>> Travel<V> {
    /// The pointer moved `step` points with `modifiers` held.
    pub fn add(&mut self, step: V, modifiers: egui::Modifiers) {
        self.raw = self.raw + step;
        self.scaled = self.scaled + step * speed(modifiers);
    }
}

/// A drag of one number: a slider row's track or value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberDrag {
    start: f32,
    range: [f32; 2],
    travel: Travel<f32>,
    last: f32,
    cancelled: bool,
}

impl NumberDrag {
    pub fn new(start: f32, range: [f32; 2]) -> Self {
        Self {
            start,
            range,
            travel: Travel::default(),
            last: start,
            cancelled: false,
        }
    }

    /// Moves it on by `points` with `modifiers` held: the new value when
    /// it changed (nothing once cancelled).
    pub fn step(&mut self, points: f32, modifiers: egui::Modifiers) -> Option<f32> {
        self.travel.add(points, modifiers);
        if self.cancelled {
            return None;
        }
        let value = clamp(
            self.start + self.travel.scaled / points_per_unit(self.range),
            self.range,
        );
        (value != self.last).then(|| {
            self.last = value;
            value
        })
    }

    /// Esc: the rest of the drag does nothing.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Let go: the value to commit, when the drag changed it and wasn't
    /// cancelled.
    pub fn finish(&self) -> Option<f32> {
        (!self.cancelled && self.last != self.start).then_some(self.last)
    }
}

/// What typed text sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Typed {
    /// `value`, kept to the range (`clamped` when it had to be).
    Value {
        value: f32,
        clamped: bool,
    },
    NotANumber,
}

/// Reads typed `text` for a value in `range`. A comma is read as a
/// decimal point; infinities and NaN aren't numbers here.
pub fn parse_typed(text: &str, range: [f32; 2]) -> Typed {
    match text.trim().replace(',', ".").parse::<f32>() {
        Ok(typed) if typed.is_finite() => {
            let value = clamp(typed, range);
            Typed::Value {
                value,
                clamped: value != typed,
            }
        }
        _ => Typed::NotANumber,
    }
}

/// A drag going on in a track or a value field: the number and where the
/// pointer was last frame.
#[derive(Clone, Copy, Debug)]
struct Held {
    drag: NumberDrag,
    x: f32,
}

/// A value field being typed in.
#[derive(Clone, Debug)]
struct Draft {
    text: String,
    /// The value as the field showed it when opened: left unchanged, it
    /// sets nothing (so a value isn't rounded to what's shown).
    shown: String,
    /// Just opened: the field takes the focus and selects its text.
    opened: bool,
}

/// What a row said last ("kept to the range"), and until when.
#[derive(Clone, Debug)]
struct Note {
    text: String,
    until: f64,
}

fn note_id(row: &str) -> egui::Id {
    egui::Id::new(("numeric note", row))
}

fn draft_id(row: &str) -> egui::Id {
    egui::Id::new(("numeric draft", row))
}

/// Where the shell records the frame Esc cancelled a drag in (it takes
/// the key before the panel is drawn).
fn cancel_id() -> egui::Id {
    egui::Id::new("numeric drag cancelled")
}

/// The shell took Esc to cancel the drag going on: the row's drag does
/// nothing more until it's let go.
pub fn cancelled_by_shell(ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| data.insert_temp(cancel_id(), pass));
}

/// The drag `response` belongs to, for a number `value` in `range`: its
/// value and gesture when it changes. `id` keeps its state.
fn drag(
    ui: &egui::Ui,
    response: &egui::Response,
    id: egui::Id,
    value: f32,
    range: [f32; 2],
) -> Option<(f32, Gesture)> {
    let ctx = ui.ctx();
    let (origin, at, modifiers, escape) = ui.input(|input| {
        (
            input.pointer.press_origin(),
            input.pointer.interact_pos(),
            input.modifiers,
            input.key_pressed(egui::Key::Escape),
        )
    });
    let pass = ctx.cumulative_pass_nr();
    let cancelled = escape || ctx.data(|data| data.get_temp::<u64>(cancel_id())) == Some(pass);
    let mut out = None;
    if response.drag_started_by(egui::PointerButton::Primary) {
        // From where it was pressed: egui starts a drag only past its
        // click distance, which would otherwise be lost.
        let x = origin.or(at).map_or(0.0, |point| point.x);
        ctx.data_mut(|data| {
            data.insert_temp(
                id,
                Held {
                    drag: NumberDrag::new(value, range),
                    x,
                },
            )
        });
    }
    let mut held = ctx.data(|data| data.get_temp::<Held>(id))?;
    if cancelled {
        held.drag.cancel();
    }
    if let Some(at) = at {
        if let Some(value) = held.drag.step(at.x - held.x, modifiers) {
            out = Some((value, Gesture::Drag));
        }
        held.x = at.x;
    }
    if response.drag_stopped() || !ui.input(|input| input.pointer.primary_down()) {
        ctx.data_mut(|data| data.remove::<Held>(id));
        // Let go: the last value again, closing the undo step.
        return held.drag.finish().map(|value| (value, Gesture::Commit));
    }
    ctx.data_mut(|data| data.insert_temp(id, held));
    out
}

/// A slider's track, `width` points wide (the middle of a one-line row,
/// between its label and its value), for `value` in `range`: a relative
/// drag by this module's rule, the same whatever the width, and the arrow
/// keys while it has the focus (a hundred and tenth of the range a press,
/// with Shift and Ctrl as a drag). Returns the track's response (a double
/// click is the row's reset) and the value asked for.
pub fn track(
    ui: &mut egui::Ui,
    row: &str,
    value: f32,
    range: [f32; 2],
    about: &str,
    width: f32,
) -> (egui::Response, Option<(f32, Gesture)>) {
    let height = ui
        .text_style_height(&egui::TextStyle::Body)
        .max(ui.spacing().interact_size.y);
    let size = egui::vec2(width, height);
    // The track's ID is its row's, not egui's next automatic one: a page
    // that shows something above the row once its value changes (the
    // Expressions page) would otherwise give the track a new ID mid-drag,
    // and egui would drop the drag after its first step.
    let (_, rect) = ui.allocate_space(size);
    let id = egui::Id::new(("numeric track", row));
    let response = ui
        .interact(rect, id, egui::Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
    let mut out = drag(ui, &response, id, value, range);
    if response.has_focus() {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            )
        });
        let (presses, modifiers) = ui.input(|input| {
            (
                input.num_presses(egui::Key::ArrowRight) as f32
                    - input.num_presses(egui::Key::ArrowLeft) as f32,
                input.modifiers,
            )
        });
        if presses != 0.0 {
            let mut keys = NumberDrag::new(value, range);
            out = keys
                .step(presses, modifiers)
                .map(|value| (value, Gesture::Commit));
        }
    }
    let shown = out.map_or(value, |(value, _)| value);
    response.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), f64::from(shown), about));
    paint_track(ui, &response, rect, shown, range);
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("numeric track", row)), rect));
    #[cfg(not(test))]
    let _ = row;
    (response, out)
}

/// Draws the track as egui draws a slider (its rail, the trailing fill
/// where the theme has one, the round handle), so the rows look as they
/// did.
fn paint_track(
    ui: &egui::Ui,
    response: &egui::Response,
    rect: egui::Rect,
    value: f32,
    range: [f32; 2],
) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let visuals = ui.style().interact(response);
    let widgets = &ui.visuals().widgets;
    let rail = (ui.spacing().slider_rail_height / 2.0).max(0.0);
    let rail = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.center().y - rail),
        egui::pos2(rect.right(), rect.center().y + rail),
    );
    let corner = widgets.inactive.corner_radius;
    ui.painter()
        .rect_filled(rail, corner, widgets.inactive.bg_fill);
    let radius = (rect.height() / 2.5).min(8.0);
    let [low, high] = range;
    let share = if (high - low).abs() > 1.0e-9 {
        ((value - low) / (high - low)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let x = egui::lerp(rect.left() + radius..=rect.right() - radius, share);
    let centre = egui::pos2(x, rail.center().y);
    if ui.visuals().slider_trailing_fill {
        let mut fill = rail;
        fill.max.x = centre.x + f32::from(corner.nw);
        ui.painter()
            .rect_filled(fill, corner, ui.visuals().selection.bg_fill);
    }
    ui.painter().add(egui::epaint::CircleShape {
        center: centre,
        radius: radius + visuals.expansion,
        fill: visuals.bg_fill,
        stroke: visuals.fg_stroke,
    });
}

/// A row's value: shown with two decimals; a click or a double click
/// opens it for typing (Enter or clicking away sets it, Esc leaves it), and
/// it drags as the track does. Returns the value asked for. A note the
/// row has ("kept to the range") is drawn by [`note`].
pub fn field(ui: &mut egui::Ui, row: &str, value: f32, range: [f32; 2]) -> Option<(f32, Gesture)> {
    let palette = theme::palette(ui.ctx());
    let id = draft_id(row);
    let draft = ui.data(|data| data.get_temp::<Draft>(id));
    let width = FIELD_WIDTH;
    let mut out = None;
    let rect;
    match draft {
        Some(mut draft) => {
            let text_id = id.with("text");
            // As wide as the value it's typed over, margins and all, so a
            // one-line row keeps its shape while it's typed in.
            let margin = egui::Margin::symmetric(4, 2);
            let response = ui.add(
                egui::TextEdit::singleline(&mut draft.text)
                    .id(text_id)
                    .desired_width(width - f32::from(margin.left + margin.right))
                    .margin(margin)
                    .horizontal_align(egui::Align::RIGHT)
                    .font(egui::TextStyle::Small),
            );
            rect = response.rect;
            if std::mem::take(&mut draft.opened) {
                response.request_focus();
                // Selected, so what's typed replaces it.
                let mut state = egui::TextEdit::load_state(ui.ctx(), text_id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::default(),
                        egui::text::CCursor::new(draft.text.chars().count()),
                    )));
                state.store(ui.ctx(), text_id);
                ui.data_mut(|data| data.insert_temp(id, draft));
            } else if response.lost_focus() || !response.has_focus() {
                ui.data_mut(|data| data.remove::<Draft>(id));
                if !ui.input(|input| input.key_pressed(egui::Key::Escape))
                    && draft.text.trim() != draft.shown
                {
                    out = typed(ui.ctx(), row, &draft.text, value, range);
                }
            } else {
                ui.data_mut(|data| data.insert_temp(id, draft));
            }
        }
        None => {
            let text = shown(value, range);
            let [low, high] = ends(range);
            // Its ID is its row's, as the track's is: what a page draws
            // above the row may change while the value is dragged.
            let height = ui.spacing().interact_size.y;
            let (_, space) = ui.allocate_space(egui::vec2(width, height));
            rect = space;
            let response = ui
                .interact(
                    rect,
                    egui::Id::new(("numeric value", row)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
                .on_hover_text(format!(
                    "Click to type a value ({} to {}), or drag. Shift: finer; Ctrl: coarser",
                    shown(low, range),
                    shown(high, range)
                ));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &text)
            });
            if ui.is_rect_visible(rect) {
                // Drawn as a button is: the value reads as a field.
                let visuals = ui.style().interact(&response);
                ui.painter().rect(
                    rect.expand(visuals.expansion),
                    visuals.corner_radius,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &text,
                    egui::TextStyle::Small.resolve(ui.style()),
                    palette.text_secondary,
                );
            }
            out = drag(ui, &response, response.id, value, range);
            if response.clicked() || response.double_clicked() {
                begin_typing(ui.ctx(), row, value, range);
            }
        }
    }
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(("numeric field", row)), rect));
    #[cfg(not(test))]
    let _ = rect;
    out
}

/// Opens row `row`'s value for typing, starting from `value` (in
/// `range`).
pub fn begin_typing(ctx: &egui::Context, row: &str, value: f32, range: [f32; 2]) {
    ctx.data_mut(|data| {
        data.insert_temp(
            draft_id(row),
            Draft {
                text: shown(value, range),
                shown: shown(value, range),
                opened: true,
            },
        )
    });
}

/// What typed `text` sets, or nothing; a note says when it was kept to the
/// range or wasn't a number.
fn typed(
    ctx: &egui::Context,
    row: &str,
    text: &str,
    value: f32,
    range: [f32; 2],
) -> Option<(f32, Gesture)> {
    let [low, high] = ends(range).map(|end| shown(end, range));
    let (asked, said) = match parse_typed(text, range) {
        Typed::Value { value, clamped } => (
            Some((value, Gesture::Commit)),
            clamped.then(|| {
                format!(
                    "{} is outside {low} to {high}: set to {}",
                    text.trim(),
                    shown(value, range)
                )
            }),
        ),
        Typed::NotANumber => (
            None,
            Some(format!(
                "\"{}\" isn't a number: kept {}",
                text.trim(),
                shown(value, range)
            )),
        ),
    };
    let now = ctx.input(|input| input.time);
    ctx.data_mut(|data| {
        if let Some(text) = said {
            let until = now + NOTE_SECONDS;
            data.insert_temp(note_id(row), Note { text, until });
        } else {
            data.remove::<Note>(note_id(row));
        }
    });
    asked
}

/// Row `row`'s note, under it, while it lasts.
pub fn note(ui: &mut egui::Ui, row: &str) {
    let Some(note) = ui.data(|data| data.get_temp::<Note>(note_id(row))) else {
        return;
    };
    let now = ui.input(|input| input.time);
    if now >= note.until {
        ui.data_mut(|data| data.remove::<Note>(note_id(row)));
        return;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs_f64(note.until - now));
    let palette = theme::palette(ui.ctx());
    ui.label(
        egui::RichText::new(note.text)
            .small()
            .color(palette.warning),
    );
}

/// Where row `row`'s track was drawn last.
#[cfg(test)]
pub fn track_rect(ctx: &egui::Context, row: &str) -> Option<egui::Rect> {
    ctx.data(|data| data.get_temp(egui::Id::new(("numeric track", row))))
}

/// Where row `row`'s value was drawn last.
#[cfg(test)]
pub fn field_rect(ctx: &egui::Context, row: &str) -> Option<egui::Rect> {
    ctx.data(|data| data.get_temp(egui::Id::new(("numeric field", row))))
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Modifiers, Vec2, vec2};

    use super::*;

    const WAIST: [f32; 2] = [0.8, 1.175];
    const CTRL: Modifiers = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-4
    }

    /// Paul's numbers: a whole range in 110 points (inside his 100 to 120),
    /// Shift a quarter, Ctrl (Cmd on macOS: egui's `command`) double.
    #[test]
    fn the_numbers_are_paul_s() {
        for range in [WAIST, [-1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.82, 1.13]] {
            let span = (range[1] - range[0]).abs();
            let whole = points_per_unit(range) * span;
            assert!(close(whole, 110.0), "{range:?}: {whole} points");
            assert!((100.0..=120.0).contains(&whole));
        }
        assert_eq!(speed(Modifiers::NONE), 1.0);
        assert_eq!(speed(Modifiers::SHIFT), 0.25);
        assert_eq!(speed(CTRL), 2.0);
        // macOS: egui reports Cmd as `command` (and `mac_cmd`), not `ctrl`.
        let cmd = Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(speed(cmd), 2.0);
        // Both: the product, half speed.
        assert_eq!(speed(CTRL | Modifiers::SHIFT), 0.5);
    }

    /// A drag of 110 points moves the whole range; Shift a quarter as far,
    /// Ctrl twice; the value stays in the range.
    #[test]
    fn a_number_drag_takes_110_points_for_the_range() {
        let mut drag = NumberDrag::new(0.8, WAIST);
        let whole = drag.step(110.0, Modifiers::NONE).unwrap();
        assert!(close(whole, 1.175), "{whole}");
        let mut drag = NumberDrag::new(0.8, WAIST);
        let short = drag.step(100.0, Modifiers::NONE).unwrap();
        assert!(short < 1.175 - 0.03, "100 points is short of it: {short}");
        let mut drag = NumberDrag::new(0.8, WAIST);
        let half = drag.step(55.0, Modifiers::NONE).unwrap();
        assert!(close(half, 0.9875), "{half}");
        let mut drag = NumberDrag::new(1.0, WAIST);
        let fine = drag.step(22.0, Modifiers::SHIFT).unwrap();
        assert!(close(fine, 1.0 + 0.375 / 20.0), "{fine}");
        let mut drag = NumberDrag::new(1.0, WAIST);
        let coarse = drag.step(22.0, CTRL).unwrap();
        assert!(close(coarse, 1.15), "{coarse}");
        // Past the end the value stops; coming back starts from the end
        // only once the pointer is back over it.
        let mut drag = NumberDrag::new(1.0, WAIST);
        assert_eq!(drag.step(200.0, Modifiers::NONE), Some(1.175));
        assert_eq!(drag.step(-30.0, Modifiers::NONE), None, "still past it");
    }

    /// Pressing Shift partway changes the speed from there on: the value
    /// doesn't jump.
    #[test]
    fn a_modifier_pressed_partway_does_not_jump() {
        let mut drag = NumberDrag::new(1.0, WAIST);
        let first = drag.step(11.0, Modifiers::NONE).unwrap();
        let then = drag.step(11.0, Modifiers::SHIFT).unwrap();
        assert!(close(first, 1.0375), "{first}");
        assert!(close(then, 1.0375 + 0.375 / 40.0), "{then}");
        let mut travel = Travel::<Vec2>::default();
        travel.add(vec2(10.0, 4.0), Modifiers::NONE);
        travel.add(vec2(8.0, 0.0), Modifiers::SHIFT);
        assert_eq!(travel.raw, vec2(18.0, 4.0));
        assert_eq!(travel.scaled, vec2(12.0, 4.0));
    }

    /// A drag that ends where it started, or was cancelled (Esc), commits
    /// nothing; otherwise the release commits the last value.
    #[test]
    fn a_cancelled_drag_commits_nothing() {
        let mut drag = NumberDrag::new(1.0, WAIST);
        drag.step(20.0, Modifiers::NONE);
        assert!(drag.finish().is_some());
        drag.cancel();
        assert_eq!(drag.step(20.0, Modifiers::NONE), None);
        assert_eq!(drag.finish(), None);
        let mut drag = NumberDrag::new(1.0, WAIST);
        drag.step(20.0, Modifiers::NONE);
        drag.step(-20.0, Modifiers::NONE);
        assert_eq!(drag.finish(), None, "back where it started");
    }

    /// A typed value is kept to the range and says so; text that isn't a
    /// number changes nothing. A comma is read as a decimal point.
    #[test]
    fn typed_values_clamp_to_the_range() {
        assert_eq!(
            parse_typed(" 1.1 ", WAIST),
            Typed::Value {
                value: 1.1,
                clamped: false
            }
        );
        assert_eq!(
            parse_typed("1,1", WAIST),
            Typed::Value {
                value: 1.1,
                clamped: false
            }
        );
        assert_eq!(
            parse_typed("5", WAIST),
            Typed::Value {
                value: 1.175,
                clamped: true
            }
        );
        assert_eq!(
            parse_typed("-3", WAIST),
            Typed::Value {
                value: 0.8,
                clamped: true
            }
        );
        assert_eq!(parse_typed("abc", WAIST), Typed::NotANumber);
        assert_eq!(parse_typed("", WAIST), Typed::NotANumber);
        assert_eq!(parse_typed("NaN", WAIST), Typed::NotANumber);
        assert_eq!(parse_typed("inf", WAIST), Typed::NotANumber);
    }

    /// Two decimals, or three where the range's ends need them.
    #[test]
    fn values_show_the_decimals_their_range_needs() {
        assert_eq!(shown(1.0, [0.0, 1.0]), "1.00");
        assert_eq!(shown(0.333, [-1.0, 1.0]), "0.33");
        assert_eq!(shown(1.175, WAIST), "1.175");
        assert_eq!(shown(1.0, WAIST), "1.000");
    }
}
