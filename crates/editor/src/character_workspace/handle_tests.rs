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
    // The right jaw corner: the Face view is turned to the character's
    // right, so its left jaw angle is behind the cheek (hidden).
    ("jaw", "jaw_right", ViewPreset::Face, [-40.0, 0.0]),
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

    /// Turns the camera to `yaw` degrees round the character (positive:
    /// toward its right), keeping the rest of the framing.
    fn turn(&mut self, yaw: f32) {
        self.workspace.orbit.yaw = yaw;
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
        // The right handle's place against the right eye's anchor, which
        // the head's bone carries and no eye parameter moves (the head
        // idles, so the screen alone would move it; the nose, further off,
        // turned with the idle by up to 1.3 points under load).
        let right_of_eye = |studio: &mut Studio| {
            studio.handle("eye_right") - studio.workspace.anchor_point("eyes", 1).unwrap()
        };
        let left = studio.handle("eye_left");
        let right = right_of_eye(&mut studio);
        studio.drag(left, vec2(120.0, 0.0));
        let moved = right_of_eye(&mut studio) - right;
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

/// Esc during a handle drag takes it back: the description is as it was
/// before the press, nothing is left to undo, and the rest of the drag
/// does nothing.
#[test]
fn esc_cancels_a_handle_drag() {
    let mut studio = Studio::new("handles-esc");
    studio.select("eyes", ViewPreset::Face);
    let start = studio.workspace.description().clone();
    let from = studio.handle("eye_left");
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
            from + vec2(30.0 * step as f32, 0.0),
        )]);
    }
    assert!(
        studio.workspace.parameter("eye_spacing_left") > 0.0,
        "the drag moved the eye"
    );
    let escape = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    studio.frame(vec![escape]);
    studio.frame(vec![egui::Event::PointerMoved(from + vec2(150.0, 0.0))]);
    studio.frame(vec![button(from + vec2(150.0, 0.0), false)]);
    studio.settle();
    assert_eq!(
        studio.workspace.description(),
        &start,
        "as before the press"
    );
    assert_eq!(studio.undo_depth(), 0, "nothing to undo");
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

/// With a feature selected in its working view, the pointer on any of its
/// handles names that feature (a handle on the border of two body regions,
/// such as a shoulder's, or beside a nostril, is still its feature's), so a
/// click there never moves the panel to another page.
#[test]
fn hovering_a_handle_names_its_feature() {
    let mut studio = Studio::new("handles-hover-each");
    let base = studio.workspace.base().unwrap();
    let mut wrong = Vec::new();
    for (feature, _, view, _) in FEATURES {
        studio.select(feature, view);
        for handle in base.handles.iter().filter(|h| h.feature == feature) {
            let Some(at) = studio.workspace.handle_on_screen(&handle.id) else {
                continue;
            };
            studio.hover(at);
            let hovered = studio.workspace.direct.hovered.clone();
            if hovered.as_deref() != Some(feature) {
                wrong.push(format!("{}: {hovered:?}", handle.id));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
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

/// A handle is drawn only where its skin can be seen: the far jaw angle
/// behind the cheek in the default Face view (turned 12 degrees to the
/// character's right), the far eye's corner behind the nose with the face
/// turned 70 degrees either way. Every handle of the seven features, in
/// its working view and turned either way, is drawn only where exact rays
/// see its skin (within 6 points: a handle at the skin's own outline, such
/// as the side of the hips from the front, shows). One that isn't drawn
/// can't be grabbed.
#[test]
fn handles_on_the_far_side_are_hidden() {
    let mut studio = Studio::new("handles-far-side");
    studio.select("jaw", ViewPreset::Face);
    assert!(
        studio.workspace.handle_on_screen("jaw_left").is_none(),
        "the left jaw angle is behind the cheek"
    );
    assert!(studio.workspace.handle_on_screen("jaw_right").is_some());
    studio.select("eyes", ViewPreset::Face);
    for (yaw, near, far) in [(70.0, "right", "left"), (-70.0, "left", "right")] {
        studio.turn(yaw);
        assert!(
            studio
                .workspace
                .handle_on_screen(&format!("eye_corner_{far}"))
                .is_none(),
            "the {far} eye's corner is behind the nose at {yaw} degrees"
        );
        assert!(
            studio
                .workspace
                .handle_on_screen(&format!("eye_corner_{near}"))
                .is_some(),
            "the near eye's corner shows at {yaw} degrees"
        );
    }
    let base = studio.workspace.base().unwrap();
    let mut unseen = Vec::new();
    for (feature, _, view, _) in FEATURES {
        for yaw in [None, Some(45.0), Some(-70.0)] {
            studio.select(feature, view);
            if let Some(yaw) = yaw {
                studio.turn(yaw);
            }
            for handle in base.handles.iter().filter(|h| h.feature == feature) {
                if studio.workspace.handle_on_screen(&handle.id).is_none() {
                    continue;
                }
                let point = studio.workspace.handle_world(&handle.id).unwrap();
                if !studio.workspace.in_sight(point, 6.0, 0.01) {
                    unseen.push(format!("{} in {view:?} at {yaw:?}", handle.id));
                }
            }
        }
    }
    assert!(unseen.is_empty(), "drawn behind other skin: {unseen:#?}");
    // A hidden handle isn't grabbed: a press where it would be turns the
    // camera.
    studio.select("jaw", ViewPreset::Face);
    let before = studio.workspace.description().clone();
    let hidden = studio.workspace.handle_point("jaw_left").unwrap();
    studio.drag(hidden, vec2(30.0, 0.0));
    assert_eq!(studio.workspace.description(), &before);
}

/// Everything the overlay draws (the handles and the tint of the selected
/// and the hovered feature) is skin that can be seen, checked with exact
/// rays against the posed body from the studio's views where far-side
/// skin lies behind near skin: the jaw from the Face view, the eyes turned
/// 70 degrees, the shoulders and a hovered waist from three-quarter, the
/// waist from the front (behind the arms) and the hips turned 45 degrees.
#[test]
fn the_overlay_is_drawn_only_where_the_skin_shows() {
    let cases: [(&str, ViewPreset, Option<f32>, &str); 6] = [
        ("jaw", ViewPreset::Face, None, "jaw"),
        ("eyes", ViewPreset::Face, Some(70.0), "eyes"),
        ("shoulders", ViewPreset::ThreeQuarter, None, "waist"),
        ("shoulders", ViewPreset::ThreeQuarter, None, "shoulders"),
        ("waist", ViewPreset::Front, None, "waist"),
        ("hips", ViewPreset::Front, Some(45.0), "hips"),
    ];
    let mut studio = Studio::new("handles-in-sight");
    let mut faults = Vec::new();
    for (selected, view, yaw, hovered) in cases {
        studio.select(selected, view);
        if let Some(yaw) = yaw {
            studio.turn(yaw);
        }
        if let Some(at) = studio.workspace.anchor_point(hovered, 0) {
            studio.hover(at);
        }
        let shown = studio.workspace.direct.shown_count();
        let points = studio.workspace.drawn_points();
        assert!(!points.is_empty(), "{selected}: something is drawn");
        // Every handle, and about 150 of the tint's corners, spread out.
        let step = ((points.len() - shown) / 150).max(1);
        let (mut handles, mut tint) = (0, 0);
        for (index, point) in points.iter().enumerate() {
            let handle = index < shown;
            if !handle && !(index - shown).is_multiple_of(step) {
                continue;
            }
            // A handle shows within 6 points of skin that's seen (at its
            // own outline), the tint within its buffer's texel and a half.
            let slack = if handle { 6.0 } else { 3.0 };
            if !studio.workspace.in_sight(*point, slack, 0.01) {
                if handle {
                    handles += 1;
                } else {
                    tint += 1;
                }
            }
        }
        if handles + tint > 0 {
            faults.push(format!(
                "{selected} in {view:?} at {yaw:?} hovering {hovered}: {handles} of {shown} \
                 handles and {tint} of {} sampled tint corners behind the skin",
                (points.len() - shown).div_ceil(step)
            ));
        }
    }
    assert!(faults.is_empty(), "{faults:#?}");
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

    // Each of the seven features selected in the view it's worked in and
    // hovered, and the views where one side is behind the other: the face
    // turned, the body at three-quarter. Each is a name, the feature, the
    // view, and the camera's turn and distance if not the view's.
    type Shot<'a> = (&'a str, &'a str, ViewPreset, Option<f32>, Option<f32>);
    let views: [Shot; 12] = [
        ("nose", "nose", ViewPreset::Face, None, None),
        ("eyes", "eyes", ViewPreset::Face, None, None),
        ("jaw", "jaw", ViewPreset::Face, None, None),
        ("lips", "lips", ViewPreset::Face, None, Some(1.6)),
        ("shoulders", "shoulders", ViewPreset::Front, None, None),
        ("waist", "waist", ViewPreset::Front, None, None),
        ("hips", "hips", ViewPreset::Front, None, None),
        ("eyes-45", "eyes", ViewPreset::Face, Some(45.0), None),
        ("eyes-70", "eyes", ViewPreset::Face, Some(70.0), None),
        ("jaw-45", "jaw", ViewPreset::Face, Some(45.0), None),
        ("lips-45", "lips", ViewPreset::Face, Some(45.0), Some(1.6)),
        (
            "shoulders-three-quarter",
            "shoulders",
            ViewPreset::ThreeQuarter,
            None,
            None,
        ),
    ];
    for (name, feature, view, yaw, distance) in views {
        studio.select(feature, view);
        if let Some(yaw) = yaw {
            studio.workspace.orbit.yaw = yaw;
        }
        if let Some(distance) = distance {
            studio.workspace.orbit.distance = distance;
        }
        studio.settle();
        // The pointer on its first handle that shows (else its anchor).
        let base = studio.workspace.base().unwrap();
        let at = base
            .handles
            .iter()
            .filter(|handle| handle.feature == feature)
            .find_map(|handle| studio.workspace.handle_on_screen(&handle.id))
            .or_else(|| studio.workspace.anchor_point(feature, 0))
            .unwrap();
        snap(&mut studio, &format!("feature-{name}"), still(at));
    }
}

/// What direct manipulation costs a frame: the studio's frame (no GPU)
/// with it off, with the eyes selected and the pointer off the character
/// (the depth buffer, the handles and the tint), and with the pointer moving
/// over the face (a ray cast every frame, the hovered feature's tint too).
/// Ignored; run it alone in a release build:
/// `cargo test -p axcenity-editor --release -- --ignored hover_costs --nocapture`.
#[test]
#[ignore = "timing: run alone with --release -- --ignored hover_costs"]
fn hover_costs_per_frame() {
    let mut studio = Studio::new("handles-timing");
    studio.select("eyes", ViewPreset::Face);
    let face = studio.workspace.anchor_point("eyes", 0).unwrap();
    // The median of 40 frames, the pointer at `at(frame)`.
    let median = |studio: &mut Studio, at: &dyn Fn(usize) -> Pos2| {
        let mut times: Vec<f64> = (0..40)
            .map(|frame| {
                let start = std::time::Instant::now();
                studio.frame(vec![egui::Event::PointerMoved(at(frame))]);
                start.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        times[times.len() / 2]
    };
    let away = |_| Pos2::new(5.0, 5.0);
    studio.workspace.apply(Intent::SetHandles(false));
    let off = median(&mut studio, &away);
    studio.workspace.apply(Intent::SetHandles(true));
    let selected = median(&mut studio, &away);
    // A point or two each frame, so the ray is cast every time.
    let moving = median(&mut studio, &|frame| face + vec2((frame % 7) as f32, 0.0));
    println!(
        "frame: off {off:.2} ms; eyes selected, pointer away {selected:.2} ms; \
         pointer moving over the face {moving:.2} ms"
    );
}

/// A face feature's tint is its own skin: the lips' is the mouth (its
/// 5.5 cm width with a margin round it), not the muzzle from the nostrils
/// to the chin (63.8 cm2, 10.0 cm across and 5.8 cm high before the
/// mouth's height and the philtrum's length, which move the whole muzzle,
/// were left out of it); and every face feature still has one.
#[test]
fn the_lips_tint_is_the_mouth_not_the_muzzle() {
    let mut studio = Studio::new("handles-lips-tint");
    studio.settle();
    let (area, size) = studio.workspace.tint_extent("lips").expect("a tint");
    assert!(
        size.x <= 9.0 && size.y <= 4.0 && area <= 35.0,
        "the lips' tint is {area:.1} cm2, {:.1} cm across and {:.1} cm high",
        size.x,
        size.y
    );
    for feature in [
        "nose", "eyes", "brows", "cheeks", "jaw", "chin", "lips", "ears",
    ] {
        let (area, _) = studio.workspace.tint_extent(feature).expect("a tint");
        assert!(area >= 3.0, "{feature}'s tint is {area:.1} cm2");
    }
}
