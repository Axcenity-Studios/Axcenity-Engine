//! RA1, Paul's drag rule of 2026-11-18, driven with pointer and key events
//! through the whole studio, as a person drives it: every numeric control
//! (the body's and the face's slider rows, the skin's values, makeup and
//! markings, and the handles on the character) takes the same 110 points
//! of drag for its whole range, Shift a quarter as far, Ctrl twice; a drag
//! or a typed value is one undo step; Esc takes a drag or a draft back;
//! a typed value outside the range is kept to it and the row says so.

use std::path::PathBuf;

use axcenity_avatar::{ShadingModel, SkinLayer, ValueKind};
use eframe::egui::{self, Pos2, Rect, Vec2, vec2};

use super::intent::{Gesture, Intent};
use super::numeric;
use super::selection::Level;
use super::shell_tests::drawn_texts;
use super::stage::ViewPreset;
use super::*;
use crate::character_studio::test_folder;
use crate::theme;

/// The waist's width: a Body row (range 0.8 to 1.175, default 1).
const WAIST: &str = "waist_width";
const WAIST_RANGE: [f32; 2] = [0.8, 1.175];
const CTRL: egui::Modifiers = egui::Modifiers {
    ctrl: true,
    command: true,
    ..egui::Modifiers::NONE
};

struct Studio {
    ctx: egui::Context,
    workspace: CharacterWorkspace,
    folder: PathBuf,
    size: Vec2,
}

impl Studio {
    fn new(name: &str, size: Vec2) -> Self {
        let folder = test_folder(name);
        let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
        Self {
            ctx: theme::test_context(),
            workspace,
            folder,
            size,
        }
    }

    fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let workspace = &mut self.workspace;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            events,
            ..Default::default()
        };
        theme::run(&self.ctx, input, |ui| {
            workspace.ui(ui, None);
        })
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(Vec::new());
        }
    }

    /// The Body page at Detailed, the waist picked: its rows alone.
    fn waist(name: &str, size: Vec2) -> Self {
        let mut studio = Self::new(name, size);
        studio.workspace.apply(Intent::PickCategory(Category::Body));
        studio.workspace.apply(Intent::PickLevel(Level::Detailed));
        studio
            .workspace
            .apply(Intent::PickFeature(Some("waist".into())));
        studio.settle();
        studio
    }

    fn track(&self, row: &str) -> Rect {
        numeric::track_rect(&self.ctx, row).unwrap_or_else(|| panic!("{row}'s track is drawn"))
    }

    fn field(&self, row: &str) -> Rect {
        numeric::field_rect(&self.ctx, row).unwrap_or_else(|| panic!("{row}'s value is drawn"))
    }

    /// A primary-button drag from `from` by `by`, in four steps, with
    /// `modifiers` held.
    fn drag_with(&mut self, from: Pos2, by: Vec2, modifiers: egui::Modifiers) {
        let button = |pos: Pos2, pressed: bool| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let held = egui::Event::ModifiersChanged(modifiers);
        self.frame(vec![held.clone(), egui::Event::PointerMoved(from)]);
        self.frame(vec![held.clone(), button(from, true)]);
        for step in 1..=4 {
            let at = from + by * (step as f32 / 4.0);
            self.frame(vec![held.clone(), egui::Event::PointerMoved(at)]);
        }
        self.frame(vec![held, button(from + by, false)]);
        self.settle();
    }

    fn drag(&mut self, from: Pos2, by: Vec2) {
        self.drag_with(from, by, egui::Modifiers::NONE);
    }

    fn click(&mut self, at: Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
        self.settle();
    }

    /// A key pressed and let go.
    fn key(&mut self, key: egui::Key) -> egui::FullOutput {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let output = self.frame(vec![event(true)]);
        self.frame(vec![event(false)]);
        output
    }

    /// Clicks row `row`'s value, types `text` and ends with `end` (Enter
    /// or Esc). Returns what was drawn the frame after.
    fn type_into(&mut self, row: &str, text: &str, end: egui::Key) -> Vec<String> {
        let field = self.field(row);
        self.click(field.center());
        self.frame(vec![egui::Event::Text(text.into())]);
        self.key(end);
        drawn_texts(&self.frame(Vec::new()))
    }

    fn value(&self, id: &str) -> f32 {
        self.workspace.parameter(id)
    }

    fn undo_depth(&self) -> usize {
        let mut history = self.workspace.history.clone();
        let mut current = self.workspace.description().clone();
        let mut depth = 0;
        while let Some(before) = history.undo(current) {
            current = before;
            depth += 1;
        }
        depth
    }
}

impl Drop for Studio {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 2.0e-4
}

/// 110 points of horizontal drag move a slider row over its whole range,
/// however wide the row (the panel as wide as the person made it, in a
/// wide window and a narrower one); half that moves half. The drag is
/// relative: it starts anywhere on the track and the value doesn't jump to
/// the pointer.
#[test]
fn a_slider_row_takes_110_points_for_its_whole_range_at_any_width() {
    let mut widths = Vec::new();
    for (width, panel) in [(1440.0, 600.0), (1100.0, 330.0)] {
        let mut studio = Studio::waist(&format!("numeric-row-{width}"), vec2(width, 900.0));
        studio.workspace.apply(Intent::SetPanelWidth {
            width: panel,
            gesture: Gesture::Drag,
        });
        studio.workspace.apply(Intent::SetParameter {
            id: WAIST.into(),
            value: WAIST_RANGE[0],
            gesture: Gesture::Commit,
        });
        studio.settle();
        let track = studio.track(WAIST);
        widths.push(track.width());
        // A press on the track changes nothing until it drags.
        studio.click(track.right_center() - vec2(8.0, 0.0));
        assert_eq!(studio.value(WAIST), WAIST_RANGE[0], "{width}: a click");
        studio.drag(track.center(), vec2(110.0, 0.0));
        let whole = studio.value(WAIST);
        assert!(close(whole, WAIST_RANGE[1]), "{width}: 110 points: {whole}");
        studio.drag(track.center(), vec2(-55.0, 0.0));
        let half = studio.value(WAIST);
        assert!(close(half, 0.9875), "{width}: back 55 points: {half}");
    }
    assert!(
        widths[0] - widths[1] > 40.0,
        "the rows differ in width: {widths:?}"
    );
}

/// A row is one line: its label, its track and its value side by side,
/// in a row no taller than 32 points, at the panel's default width and at
/// its narrowest; a value that leaves its default changes neither the
/// track nor the value's place (the reset mark has a column of its own),
/// so a drag never moves what is dragged.
#[test]
fn a_slider_row_is_one_line_and_keeps_its_shape() {
    // The Build section's two rows: muscle, and body fat under it.
    const ROW: &str = "muscle";
    for panel in [super::shell::PANEL_WIDTH, super::shell::PANEL_MIN] {
        let mut studio = Studio::new(&format!("numeric-line-{panel}"), vec2(1440.0, 900.0));
        studio.workspace.apply(Intent::PickCategory(Category::Body));
        studio.workspace.apply(Intent::PickLevel(Level::Detailed));
        studio.workspace.apply(Intent::SetPanelWidth {
            width: panel,
            gesture: Gesture::Drag,
        });
        studio.settle();
        let (track, field) = (studio.track(ROW), studio.field(ROW));
        // Side by side, on one line, inside the panel.
        assert!(track.right() < field.left(), "{panel}: {track:?} {field:?}");
        assert!(
            (track.center().y - field.center().y).abs() < 1.0,
            "{panel}: on one line"
        );
        assert!(track.height() <= 32.0 && field.height() <= 32.0);
        assert!(track.width() >= 44.0, "{panel}: a track to see: {track:?}");
        assert!(track.left() > 1440.0 - panel, "{panel}: {track:?}");
        assert!(field.right() <= 1440.0 - 12.0, "{panel}: {field:?}");
        // The next row starts within 32 points of this one.
        let next = studio.track("fat");
        let pitch = next.top() - track.top();
        assert!(
            (20.0..=32.0).contains(&pitch),
            "{panel}: a row every {pitch} points"
        );
        // Off its default: the same places (and a reset mark beside it).
        studio.workspace.apply(Intent::SetParameter {
            id: ROW.into(),
            value: 0.4,
            gesture: Gesture::Commit,
        });
        studio.settle();
        assert_eq!(studio.track(ROW), track, "{panel}: the track stays");
        assert_eq!(studio.field(ROW), field, "{panel}: the value stays");
        let drawn = drawn_texts(&studio.frame(Vec::new()));
        assert!(
            drawn
                .iter()
                .any(|t| t == crate::theme::icons::ARROW_COUNTER_CLOCKWISE),
            "{panel}: the reset mark is drawn"
        );
    }
}

/// A double click on a row's track puts its value back to the default, as
/// one undo step; so does its reset mark.
#[test]
fn a_double_click_on_the_track_resets_the_row() {
    let mut studio = Studio::waist("numeric-reset", vec2(1440.0, 900.0));
    studio.workspace.apply(Intent::SetParameter {
        id: WAIST.into(),
        value: 1.1,
        gesture: Gesture::Commit,
    });
    studio.settle();
    let at = studio.track(WAIST).center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    studio.frame(vec![egui::Event::PointerMoved(at)]);
    for _ in 0..2 {
        studio.frame(vec![button(true)]);
        studio.frame(vec![button(false)]);
    }
    studio.settle();
    assert_eq!(studio.value(WAIST), 1.0, "back to the default");
    assert_eq!(studio.undo_depth(), 2, "one step more");
}

/// Shift moves a quarter as far, Ctrl twice as far; each drag is one undo
/// step.
#[test]
fn shift_is_a_quarter_and_ctrl_double_on_a_row() {
    for (name, modifiers, change) in [
        ("plain", egui::Modifiers::NONE, 0.075),
        ("shift", egui::Modifiers::SHIFT, 0.01875),
        ("ctrl", CTRL, 0.15),
    ] {
        let mut studio = Studio::waist(&format!("numeric-{name}"), vec2(1440.0, 900.0));
        let track = studio.track(WAIST);
        studio.drag_with(track.center(), vec2(22.0, 0.0), modifiers);
        let value = studio.value(WAIST);
        assert!(close(value, 1.0 + change), "{name}: {value}");
        assert_eq!(studio.undo_depth(), 1, "{name}: one drag, one step");
        assert!(!studio.workspace.dragging(), "{name}: the drag ended");
    }
}

/// Esc during a row's drag puts the value back where the drag started,
/// leaves nothing to undo, and the rest of the drag does nothing.
#[test]
fn esc_cancels_a_row_drag_back_to_its_start() {
    let mut studio = Studio::waist("numeric-esc", vec2(1440.0, 900.0));
    let start = studio.workspace.description().clone();
    let from = studio.track(WAIST).center();
    let button = |pos: Pos2, pressed: bool| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    studio.frame(vec![egui::Event::PointerMoved(from)]);
    studio.frame(vec![button(from, true)]);
    for step in 1..=3 {
        studio.frame(vec![egui::Event::PointerMoved(
            from + vec2(10.0 * step as f32, 0.0),
        )]);
    }
    assert!(studio.value(WAIST) > 1.05, "the drag moved the waist");
    studio.key(egui::Key::Escape);
    assert_eq!(studio.workspace.description(), &start, "back at once");
    studio.frame(vec![egui::Event::PointerMoved(from + vec2(60.0, 0.0))]);
    studio.frame(vec![button(from + vec2(60.0, 0.0), false)]);
    studio.settle();
    assert_eq!(
        studio.workspace.description(),
        &start,
        "as before the press"
    );
    assert_eq!(studio.undo_depth(), 0, "nothing to undo");
}

/// Every row's value can be typed, at every level: a click on it, the
/// number, Enter: one undo step. Out of the range it's kept to the range
/// and the row says so; Esc keeps the value; text that isn't a number
/// changes nothing and says so.
#[test]
fn a_typed_value_is_one_undo_step_and_clamps_to_the_range() {
    let mut studio = Studio::waist("numeric-typed", vec2(1440.0, 900.0));
    // Quick and Detailed type too, not only Advanced.
    assert_eq!(studio.workspace.selection.level, Level::Detailed);
    studio.type_into(WAIST, "1.1", egui::Key::Enter);
    assert!(close(studio.value(WAIST), 1.1), "{}", studio.value(WAIST));
    assert_eq!(studio.undo_depth(), 1, "one step");
    studio.workspace.apply(Intent::Undo);
    assert_eq!(studio.value(WAIST), 1.0, "one undo takes it back");

    let drawn = studio.type_into(WAIST, "5", egui::Key::Enter);
    assert_eq!(studio.value(WAIST), WAIST_RANGE[1], "kept to the range");
    assert!(
        drawn
            .iter()
            .any(|text| text.contains("outside 0.800 to 1.175: set to 1.175")),
        "the row says so: {drawn:?}"
    );
    assert_eq!(studio.undo_depth(), 1);

    let before = studio.workspace.description().clone();
    studio.type_into(WAIST, "0.9", egui::Key::Escape);
    assert_eq!(studio.workspace.description(), &before, "Esc keeps it");
    assert_eq!(studio.undo_depth(), 1, "and makes no step");

    let drawn = studio.type_into(WAIST, "wide", egui::Key::Enter);
    assert_eq!(studio.workspace.description(), &before, "not a number");
    assert!(
        drawn.iter().any(|text| text.contains("isn't a number")),
        "{drawn:?}"
    );
    assert_eq!(studio.undo_depth(), 1);

    // Opened and left as it was: nothing is set (a value between the
    // shown decimals isn't rounded).
    studio.workspace.apply(Intent::SetParameter {
        id: WAIST.into(),
        value: 1.0004,
        gesture: Gesture::Commit,
    });
    studio.settle();
    let field = studio.field(WAIST);
    studio.click(field.center());
    studio.key(egui::Key::Enter);
    studio.settle();
    assert_eq!(studio.value(WAIST), 1.0004, "not rounded to 1.000");
    assert_eq!(studio.undo_depth(), 2);

    // The value field drags as the track does.
    let field = studio.field(WAIST);
    studio.drag(field.center(), vec2(-55.0, 0.0));
    let dragged = studio.value(WAIST);
    assert!(close(dragged, 1.0004 - 0.1875), "{dragged}");
}

/// The Face page's rows too (a face row at Quick, where it used to be only
/// shown): typed and dragged by the same rule.
#[test]
fn a_face_row_follows_the_same_rule() {
    let mut studio = Studio::new("numeric-face", vec2(1440.0, 900.0));
    studio.workspace.apply(Intent::PickCategory(Category::Face));
    studio.workspace.apply(Intent::PickLevel(Level::Quick));
    studio
        .workspace
        .apply(Intent::PickFeature(Some("nose".into())));
    studio.settle();
    let track = studio.track("nose_length");
    studio.drag(track.center(), vec2(22.0, 0.0));
    let value = studio.value("nose_length");
    assert!(close(value, 0.4), "a fifth of -1 to 1: {value}");
    studio.type_into("nose_length", "-0.25", egui::Key::Enter);
    assert!(close(studio.value("nose_length"), -0.25));
    assert_eq!(studio.undo_depth(), 2);
}

/// The Expressions page's face controls are the shared rows too: 110
/// points for a whole range (0 to 1, or -1 to 1 for a two-way row), Shift
/// a quarter, and none of it is an undo step (the page is a preview).
#[test]
fn the_expression_rows_follow_the_same_rule() {
    use super::expressions::ROWS;
    let mut studio = Studio::new("numeric-expressions", vec2(1440.0, 1400.0));
    studio
        .workspace
        .apply(Intent::PickCategory(Category::Expressions));
    studio.settle();
    let raise = ROWS.iter().find(|row| row.label == "Raise").unwrap();
    let look = ROWS
        .iter()
        .find(|row| row.label == "Look down or up")
        .unwrap();
    let id =
        |row: &super::expressions::Row| format!("face control {} {}", row.group.label(), row.label);
    let track = studio.track(&id(raise));
    studio.drag(track.center(), vec2(22.0, 0.0));
    let value = studio.workspace.row_value(raise.controls, false);
    assert!(close(value, 0.2), "a fifth of 0 to 1: {value}");
    let track = studio.track(&id(look));
    studio.drag_with(track.center(), vec2(44.0, 0.0), egui::Modifiers::SHIFT);
    let value = studio.workspace.row_value(look.controls, false);
    assert!(
        close(value, 0.2),
        "a quarter of two fifths of -1 to 1: {value}"
    );
    assert_eq!(
        studio.undo_depth(),
        0,
        "the page never edits the description"
    );
}

/// The Skin page's values and a makeup layer's sliders take the same drag.
#[test]
fn skin_values_and_makeup_take_the_same_drag() {
    // Tall, so the makeup section is on the screen without scrolling.
    let mut studio = Studio::new("numeric-skin", vec2(1440.0, 2400.0));
    let base = studio.workspace.base().unwrap();
    let spec = base
        .skin_layers
        .iter()
        .find(|layer| layer.id == "blush")
        .unwrap()
        .clone();
    let mut blush = SkinLayer::from_spec(&spec);
    blush.enabled = true;
    studio.workspace.apply(Intent::SetSkinLayer {
        id: "blush".into(),
        marking: false,
        to: Some(blush.clone()),
        gesture: Gesture::Commit,
    });
    studio
        .workspace
        .apply(Intent::PickCategory(Category::SkinAndMakeup));
    studio.settle();
    let (min, max, neutral) = ShadingModel::Skin
        .values()
        .iter()
        .find(|value| value.name == "melanin")
        .and_then(|value| match value.kind {
            ValueKind::Number { min, max, neutral } => Some((min, max, neutral)),
            _ => None,
        })
        .unwrap();
    let track = studio.track("Skin.melanin");
    let toward = if neutral - min > max - neutral {
        -1.0
    } else {
        1.0
    };
    studio.drag(track.center(), vec2(22.0 * toward, 0.0));
    let melanin = match studio.workspace.material_value("Skin", "melanin") {
        Some(axcenity_avatar::MaterialValue::Number(value)) => value,
        other => panic!("melanin: {other:?}"),
    };
    assert!(
        close(melanin, neutral + toward * (max - min) / 5.0),
        "melanin {melanin} from {neutral} in {min}..{max}"
    );

    let track = studio.track("skin.blush.opacity");
    studio.drag(track.center(), vec2(-22.0, 0.0));
    let opacity = studio.workspace.skin_layer("blush").unwrap().opacity;
    assert!(
        close(opacity, blush.opacity - 0.2),
        "{opacity} from {}",
        blush.opacity
    );
}

/// The handles on the character take the same helper's 110 points for a
/// parameter's whole range along their direction on the screen, whatever
/// the zoom (no 240-point fallback, no one-to-one skin following): every
/// live drive of every feature's handle, in its view.
#[test]
fn every_handle_takes_110_points_for_its_whole_range() {
    const HANDLES: [(&str, &str, ViewPreset); 7] = [
        ("nose", "nose_tip", ViewPreset::Face),
        ("eyes", "eye_left", ViewPreset::Face),
        ("jaw", "jaw_right", ViewPreset::Face),
        ("lips", "mouth_corner_left", ViewPreset::Face),
        ("shoulders", "shoulder_left", ViewPreset::Front),
        ("waist", "waist_left", ViewPreset::Front),
        ("hips", "hip_left", ViewPreset::Front),
    ];
    let mut studio = Studio::new("numeric-handles", vec2(1440.0, 900.0));
    let base = studio.workspace.base().unwrap();
    let mut wrong = Vec::new();
    for (feature, handle, view) in HANDLES {
        studio
            .workspace
            .apply(Intent::PickFeature(Some(feature.into())));
        studio.workspace.apply(Intent::FocusView(view));
        for zoom in [1.0, 0.5] {
            studio.workspace.orbit.distance *= zoom;
            studio.settle();
            let live = studio.workspace.handle_drives_on_screen(handle);
            if live.is_empty() {
                wrong.push(format!("{handle} at {zoom}: no live drive"));
            }
            for (parameter, screen) in live {
                let range = base.parameter(&parameter).unwrap().range;
                let whole = screen.length() * (range[1] - range[0]).abs();
                if (whole - 110.0).abs() > 0.01 {
                    wrong.push(format!("{handle} {parameter} at {zoom}: {whole:.1} points"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A drag of 22 points along the nose tip's direction moves its parameter
/// a fifth of its range, zoomed out or in; Shift a quarter of that, Ctrl
/// twice. One undo step each.
#[test]
fn a_handle_drag_is_the_same_at_any_zoom_with_shift_and_ctrl() {
    for (name, zoom, modifiers, share) in [
        ("plain", 1.0, egui::Modifiers::NONE, 0.2),
        ("zoomed", 0.5, egui::Modifiers::NONE, 0.2),
        ("shift", 1.0, egui::Modifiers::SHIFT, 0.05),
        ("ctrl", 1.0, CTRL, 0.4),
    ] {
        let mut studio = Studio::new(&format!("numeric-handle-{name}"), vec2(1440.0, 900.0));
        studio
            .workspace
            .apply(Intent::PickFeature(Some("nose".into())));
        studio.workspace.apply(Intent::FocusView(ViewPreset::Face));
        studio.workspace.orbit.distance *= zoom;
        studio.settle();
        let live = studio.workspace.handle_drives_on_screen("nose_tip");
        let (parameter, screen) = live.first().expect("a live drive").clone();
        let range = studio
            .workspace
            .base()
            .unwrap()
            .parameter(&parameter)
            .unwrap()
            .range;
        let start = studio.value(&parameter);
        let at = studio.workspace.handle_on_screen("nose_tip").unwrap();
        let along = screen.normalize();
        studio.drag_with(at, vec2(along.x, along.y) * 22.0, modifiers);
        let change = studio.value(&parameter) - start;
        let expected = share * (range[1] - range[0]);
        assert!(
            (change - expected).abs() < 2.0e-3,
            "{name}: {parameter} moved {change}, not {expected}"
        );
        assert_eq!(studio.undo_depth(), 1, "{name}: one step");
    }
}

/// With `AXCENITY_SNAPSHOTS`, the waist's row drawn for real while a value
/// is typed into it (`character-studio-numeric-typing.png`) and after an
/// out-of-range value was kept to the range
/// (`character-studio-numeric-clamped.png`).
#[test]
fn snapshots_of_a_row_typed_into() {
    let Some(folder) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let text = |text: &str| egui::Event::Text(text.into());
    let key = |key, pressed| egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    for (name, typed) in [
        ("typing", vec![vec![text("1.12")]]),
        (
            "clamped",
            vec![
                vec![text("1.5")],
                vec![key(egui::Key::Enter, true)],
                vec![key(egui::Key::Enter, false)],
            ],
        ),
    ] {
        let mut studio = Studio::waist(&format!("numeric-snap-{name}"), vec2(1440.0, 900.0));
        studio.workspace.canvas = Canvas::default();
        studio.workspace.other_canvas = Canvas::default();
        // Settle, open the field, then type; a 60th of a second a frame,
        // so the note is still up in the last.
        let mut frames: Vec<(f64, Vec<egui::Event>)> = Vec::new();
        for index in 0..4 {
            frames.push((index as f64 / 60.0, Vec::new()));
        }
        for events in typed.into_iter().chain([Vec::new(), Vec::new()]) {
            frames.push((frames.len() as f64 / 60.0, events));
        }
        let workspace = &mut studio.workspace;
        let mut first = true;
        crate::snapshot::snapshot_timed(
            &folder.join(format!("character-studio-numeric-{name}.png")),
            vec2(1440.0, 900.0),
            frames,
            |ui, render_state| {
                if std::mem::take(&mut first) {
                    numeric::begin_typing(ui.ctx(), WAIST, 1.0, WAIST_RANGE);
                }
                workspace.ui(ui, Some(render_state));
            },
        );
    }
}
