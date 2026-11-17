//! Refinement R2 (direct manipulation, brief §3.4, §4.3 and §5.3; the
//! plan's 11.5 "Done when"): the feature under the pointer lights up, a
//! click selects it and frames it, and its handles, drawn on the skin,
//! write the same parameters through the same intents as the sliders. Each
//! is driven with pointer events through the whole workspace, as a person
//! drives it (no GPU: the preview isn't drawn, the rest is).

use std::path::PathBuf;

use eframe::egui::{self, Pos2, vec2};

use super::intent::{Gesture, Intent};
use super::selection::Level;
use super::shell_tests::input;
use super::stage::ViewPreset;
use super::*;
use crate::character_studio::test_folder;
use crate::theme;

/// The seven features of the brief, in its order: a handle of each, the
/// view it's dragged in, and the drag (points: x right, y down).
const FEATURES: [(&str, &str, ViewPreset, [f32; 2]); 7] = [
    ("nose", "nose_tip", ViewPreset::Face, [0.0, 40.0]),
    ("eyes", "eye_left", ViewPreset::Face, [36.0, 0.0]),
    ("jaw", "jaw_left", ViewPreset::Face, [40.0, 0.0]),
    ("lips", "mouth_corner_left", ViewPreset::Face, [30.0, 0.0]),
    ("shoulders", "shoulder_left", ViewPreset::Front, [24.0, 0.0]),
    ("waist", "waist_left", ViewPreset::Front, [10.0, 0.0]),
    ("hips", "hip_left", ViewPreset::Front, [-12.0, 0.0]),
];

struct Studio {
    ctx: egui::Context,
    workspace: CharacterWorkspace,
    folder: PathBuf,
}

impl Studio {
    fn new(name: &str) -> Self {
        let folder = test_folder(name);
        let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
        Self {
            ctx: theme::test_context(),
            workspace,
            folder,
        }
    }

    /// One frame with `events`.
    fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let workspace = &mut self.workspace;
        theme::run(&self.ctx, input(events), |ui| {
            workspace.ui(ui, None);
        })
    }

    /// A few frames with nothing happening (the layout settles, the pose
    /// and the handles are worked out).
    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(Vec::new());
        }
    }

    /// Shows `feature` selected in `view`.
    fn select(&mut self, feature: &str, view: ViewPreset) {
        self.workspace
            .apply(Intent::PickFeature(Some(feature.to_owned())));
        self.workspace.apply(Intent::FocusView(view));
        self.settle();
    }

    fn handle(&self, id: &str) -> Pos2 {
        self.workspace
            .handle_on_screen(id)
            .unwrap_or_else(|| panic!("{id} is shown"))
    }

    /// A primary-button drag from `from` by `by`, in four steps, with
    /// `modifiers` held.
    fn drag_with(&mut self, from: Pos2, by: egui::Vec2, modifiers: egui::Modifiers) {
        let button = |pos: Pos2, pressed: bool| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        self.frame(vec![
            egui::Event::ModifiersChanged(modifiers),
            egui::Event::PointerMoved(from),
        ]);
        self.frame(vec![
            egui::Event::ModifiersChanged(modifiers),
            button(from, true),
        ]);
        for step in 1..=4 {
            let at = from + by * (step as f32 / 4.0);
            self.frame(vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(at),
            ]);
        }
        self.frame(vec![
            egui::Event::ModifiersChanged(modifiers),
            button(from + by, false),
        ]);
        self.settle();
    }

    fn drag(&mut self, from: Pos2, by: egui::Vec2) {
        self.drag_with(from, by, egui::Modifiers::NONE);
    }

    /// Moves the pointer to `at` (no button down).
    fn hover(&mut self, at: Pos2) {
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![egui::Event::PointerMoved(at)]);
    }

    fn click(&mut self, at: Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.hover(at);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
        self.settle();
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

/// A drag through feature `index`'s handle (of [`FEATURES`]) gives the
/// same description as the sliders set to where the drag left them, as
/// one undo step that undoes the whole drag.
fn a_drag_through_the_handle_gives_what_the_sliders_give(index: usize) {
    {
        let (feature, handle, view, by) = FEATURES[index];
        let mut studio = Studio::new(&format!("handles-{feature}"));
        let before = studio.workspace.description().clone();
        studio.select(feature, view);
        let at = studio.handle(handle);
        studio.drag(at, vec2(by[0], by[1]));
        let dragged = studio.workspace.description().clone();
        assert_ne!(dragged, before, "{handle}: the drag changed the character");
        assert_eq!(studio.undo_depth(), 1, "{handle}: one drag, one step");
        assert!(!studio.workspace.dragging(), "{handle}: the drag ended");

        // The sliders: the handle's parameters set to the dragged values,
        // through the slider's intent (Symmetry on, as the studio opens).
        let base = studio.workspace.base().unwrap();
        let drives = &base.handle(handle).unwrap().drives;
        let mut sliders = Studio::new(&format!("handles-{feature}-sliders"));
        for drive in drives {
            let Some(value) = dragged.parameters.get(&drive.parameter) else {
                continue;
            };
            sliders.workspace.apply(Intent::SetMirrored {
                id: drive.parameter.clone(),
                value: *value,
                gesture: Gesture::Commit,
            });
        }
        assert_eq!(
            sliders.workspace.description(),
            &dragged,
            "{handle}: the handle and the sliders agree"
        );

        // One undo takes the whole drag back.
        studio.workspace.apply(Intent::Undo);
        assert_eq!(studio.workspace.description(), &before, "{handle}");
    }
}

// The seven features of the brief, each its own test (they run side by
// side).
#[test]
fn the_nose_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(0);
}

#[test]
fn the_eye_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(1);
}

#[test]
fn the_jaw_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(2);
}

#[test]
fn the_mouth_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(3);
}

#[test]
fn the_shoulder_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(4);
}

#[test]
fn the_waist_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(5);
}

#[test]
fn the_hip_handle_gives_what_its_sliders_give() {
    a_drag_through_the_handle_gives_what_the_sliders_give(6);
}

/// A drag that doesn't start on a handle turns the camera (or, with the
/// right button, pans it): the description never changes. Alt with a drag
/// on a handle turns the camera too.
#[test]
fn a_camera_drag_never_changes_the_description() {
    let mut studio = Studio::new("handles-camera");
    studio.select("eyes", ViewPreset::Face);
    let before = studio.workspace.description().clone();
    let handle = studio.handle("eye_left");
    let rect = studio.workspace.direct.view_rect.expect("a viewport");

    // On the skin beside the handle (the cheek), then on the backdrop.
    for from in [handle + vec2(0.0, 60.0), rect.left_top() + vec2(30.0, 60.0)] {
        let orbit = studio.workspace.orbit;
        studio.drag(from, vec2(80.0, 10.0));
        assert_ne!(studio.workspace.orbit, orbit, "the camera turned");
        assert_eq!(studio.workspace.description(), &before);
    }
    // Alt and a drag that starts on the handle.
    studio.workspace.focus(ViewPreset::Face);
    studio.settle();
    let orbit = studio.workspace.orbit;
    let handle = studio.handle("eye_left");
    studio.drag_with(handle, vec2(-60.0, 0.0), egui::Modifiers::ALT);
    assert_ne!(studio.workspace.orbit, orbit, "Alt turns the camera");
    assert_eq!(studio.workspace.description(), &before);
    // The right button pans, wherever it starts.
    studio.workspace.focus(ViewPreset::Face);
    studio.settle();
    let orbit = studio.workspace.orbit;
    let handle = studio.handle("eye_left");
    let right = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    studio.frame(vec![egui::Event::PointerMoved(handle)]);
    studio.frame(vec![right(handle, true)]);
    for step in 1..=3 {
        studio.frame(vec![egui::Event::PointerMoved(
            handle + vec2(20.0 * step as f32, 0.0),
        )]);
    }
    studio.frame(vec![right(handle + vec2(60.0, 0.0), false)]);
    assert_ne!(studio.workspace.orbit.target, orbit.target, "panned");
    assert_eq!(studio.workspace.description(), &before);
    assert_eq!(studio.undo_depth(), 0);
}

/// Symmetry on: dragging the left eye's handle moves the right eye too
/// (both parameters, and the right handle on the screen). Off: only the
/// left side moves.
#[test]
fn symmetry_moves_the_mirror() {
    for symmetry in [true, false] {
        let mut studio = Studio::new(&format!("handles-symmetry-{symmetry}"));
        studio.workspace.apply(Intent::SetSymmetry(symmetry));
        studio.select("eyes", ViewPreset::Face);
        // The right handle's place against the nose's (the head idles,
        // so the screen alone would move it a little).
        let right_of_nose = |studio: &mut Studio| {
            studio.handle("eye_right") - studio.workspace.handle_point("nose_tip").unwrap()
        };
        let left = studio.handle("eye_left");
        let right = right_of_nose(&mut studio);
        studio.drag(left, vec2(120.0, 0.0));
        let moved = right_of_nose(&mut studio) - right;
        let spacing = |side: &str| studio.workspace.parameter(&format!("eye_spacing_{side}"));
        assert!(spacing("left") > 0.4, "the left eye moved out");
        if symmetry {
            assert_eq!(spacing("right"), spacing("left"), "the mirror moved alike");
            // Out on both sides: the right handle went the other way.
            assert!(moved.x < -2.0, "the right handle moved {moved:?}");
        } else {
            assert_eq!(spacing("right"), 0.0, "the right eye stayed");
            // Across: the eyes' spacing is side to side (the head's idle
            // nods it up and down a little).
            assert!(moved.x.abs() < 0.75, "the right handle stayed ({moved:?})");
        }
    }
}

/// Hovering lights the feature under the pointer and names it; a click
/// selects it (its category and its controls), frames it when it's small
/// on the screen, and shows its handles. A click on the backdrop drops it.
#[test]
fn hover_highlights_and_a_click_selects_and_frames_the_feature() {
    let mut studio = Studio::new("handles-hover");
    studio.workspace.apply(Intent::PickCategory(Category::Body));
    studio.workspace.apply(Intent::FocusView(ViewPreset::Front));
    studio.settle();
    assert_eq!(studio.workspace.selection.feature, None);
    // The left hip, from the front: a little in from its outline.
    let hip = studio
        .workspace
        .handle_point("hip_left")
        .expect("on screen");
    let middle = studio.workspace.direct.view_rect.unwrap().center().x;
    let hip = hip + vec2(6.0 * (middle - hip.x).signum(), 0.0);
    studio.hover(hip);
    assert_eq!(studio.workspace.direct.hovered.as_deref(), Some("hips"));
    let texts =
        super::shell_tests::drawn_texts(&studio.frame(vec![egui::Event::PointerMoved(hip)]));
    assert!(texts.iter().any(|t| t.contains("Hips")), "{texts:?}");
    studio.click(hip);
    assert_eq!(studio.workspace.selection.feature.as_deref(), Some("hips"));
    assert_eq!(studio.workspace.selection.category, Category::Body);
    assert!(studio.workspace.handle_on_screen("hip_left").is_some());
    assert!(studio.workspace.handle_on_screen("hip_right").is_some());
    assert!(
        studio.workspace.handle_on_screen("waist_left").is_none(),
        "only the hips'"
    );

    // The left eye from the whole body's view: the Face page, the eyes,
    // and the camera comes close to them.
    let eye = studio.workspace.anchor_point("eyes", 0).expect("on screen");
    let far = studio.workspace.orbit.distance;
    studio.click(eye);
    assert_eq!(studio.workspace.selection.category, Category::Face);
    assert_eq!(studio.workspace.selection.feature.as_deref(), Some("eyes"));
    assert!(studio.workspace.selection.level >= Level::Detailed);
    // The Face page frames the face; framing goes on from there only if
    // the eyes are still small on the screen.
    let goal = studio
        .workspace
        .direct
        .focus_goal()
        .unwrap_or(studio.workspace.orbit);
    assert!(
        goal.distance < far / 2.0,
        "closer: {} from {far}",
        goal.distance
    );
    let eyes = studio.workspace.feature_centre("eyes").unwrap();
    assert!(goal.target.distance(eyes) < 0.5, "aimed at the eyes");

    // The backdrop: nothing is selected, and the camera stays.
    let rect = studio.workspace.direct.view_rect.unwrap();
    studio.click(rect.left_top() + vec2(20.0, 40.0));
    assert_eq!(studio.workspace.selection.feature, None);
    assert_eq!(studio.workspace.direct.hovered, None);
}

/// The switch turns direct manipulation off: no highlight, no handles,
/// and a drag where a handle was turns the camera.
#[test]
fn handles_off_leaves_the_camera_only() {
    let mut studio = Studio::new("handles-off");
    studio.select("eyes", ViewPreset::Face);
    let at = studio.handle("eye_left");
    let before = studio.workspace.description().clone();
    studio.workspace.apply(Intent::SetHandles(false));
    studio.settle();
    assert!(studio.workspace.handle_on_screen("eye_left").is_none());
    studio.hover(at);
    assert_eq!(studio.workspace.direct.hovered, None);
    let orbit = studio.workspace.orbit;
    studio.drag(at, vec2(40.0, 0.0));
    assert_ne!(studio.workspace.orbit, orbit);
    assert_eq!(studio.workspace.description(), &before);
    studio.workspace.apply(Intent::SetHandles(true));
    studio.workspace.focus(ViewPreset::Face);
    studio.settle();
    assert!(studio.workspace.handle_on_screen("eye_left").is_some());
}

/// Drawn for real with `AXCENITY_SNAPSHOTS`: the hovered nose, the eyes
/// selected with their handles, and a drag of the left eye under way.
#[test]
fn handles_are_drawn_for_hover_selection_and_a_drag() {
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let mut studio = Studio::new("handles-snapshots");
    studio.workspace.apply(Intent::PickCategory(Category::Face));
    studio.settle();
    let nose = studio.workspace.handle_point("nose_bridge").unwrap();
    let snap = |studio: &mut Studio, name: &str, events: Vec<Vec<egui::Event>>| {
        let workspace = &mut studio.workspace;
        workspace.canvas = Canvas::default();
        workspace.other_canvas = Canvas::default();
        crate::snapshot::snapshot_rendered(
            &snapshots.join(format!("character-studio-handles-{name}.png")),
            egui::vec2(1440.0, 900.0),
            events,
            |ui, render_state| {
                workspace.ui(ui, Some(render_state));
            },
        );
    };
    let still = |at: Pos2| vec![vec![egui::Event::PointerMoved(at)]; 4];
    snap(&mut studio, "hover", still(nose));
    studio.select("eyes", ViewPreset::Face);
    let corner = studio.handle("eye_corner_left");
    snap(&mut studio, "selected", still(corner + vec2(0.0, 80.0)));
    let left = studio.handle("eye_left");
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    snap(
        &mut studio,
        "dragging",
        vec![
            vec![egui::Event::PointerMoved(left)],
            vec![button(left, true)],
            vec![egui::Event::PointerMoved(left + vec2(12.0, -6.0))],
            vec![egui::Event::PointerMoved(left + vec2(24.0, -12.0))],
            vec![egui::Event::PointerMoved(left + vec2(30.0, -14.0))],
        ],
    );
    studio.select("hips", ViewPreset::Front);
    let hip = studio.handle("hip_left");
    snap(&mut studio, "body", still(hip));
}

/// What hovering costs a frame: the studio's frame (no GPU) with the
/// pointer off the viewport and over the face, the eyes selected. Ignored;
/// run it alone in a release build:
/// `cargo test -p axcenity-editor --release -- --ignored hover_costs --nocapture`.
#[test]
#[ignore = "timing: run alone with --release -- --ignored hover_costs"]
fn hover_costs_per_frame() {
    let mut studio = Studio::new("handles-timing");
    studio.select("eyes", ViewPreset::Face);
    let face = studio.workspace.anchor_point("eyes", 0).unwrap();
    let mut median = |at: Pos2| {
        let mut times: Vec<f64> = (0..40)
            .map(|_| {
                let start = std::time::Instant::now();
                studio.frame(vec![egui::Event::PointerMoved(at)]);
                start.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        times[times.len() / 2]
    };
    let away = median(Pos2::new(5.0, 5.0));
    let over = median(face);
    println!("frame, pointer away: {away:.2} ms; over the face: {over:.2} ms");
}
