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
    // Lit under the pointer, and not up at the chest.
    let strength_at = |studio: &Studio, at: Pos2| {
        let (_, texels) = studio.workspace.highlight();
        texels
            .iter()
            .find(|(texel, _)| (texel.x - at.x).abs() <= 1.0 && (texel.y - at.y).abs() <= 1.0)
            .map_or(0.0, |(_, strength)| *strength)
    };
    let [_, hover] = CharacterWorkspace::highlight_strengths();
    assert!(
        strength_at(&studio, hip) >= hover - 0.01,
        "the hips are lit"
    );
    let chest = studio.workspace.handle_point("shoulder_left").unwrap();
    let chest = egui::pos2(middle, chest.y + 40.0);
    assert_eq!(strength_at(&studio, chest), 0.0, "the chest isn't");
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
    assert!(
        Lit::of(&studio).texels.is_empty(),
        "the selection isn't lit"
    );
    studio.hover(at);
    assert_eq!(studio.workspace.direct.hovered, None);
    assert!(Lit::of(&studio).texels.is_empty(), "nor what's hovered");
    let orbit = studio.workspace.orbit;
    studio.drag(at, vec2(40.0, 0.0));
    assert_ne!(studio.workspace.orbit, orbit);
    assert_eq!(studio.workspace.description(), &before);
    studio.workspace.apply(Intent::SetHandles(true));
    studio.workspace.focus(ViewPreset::Face);
    studio.settle();
    assert!(studio.workspace.handle_on_screen("eye_left").is_some());
    let lit = Lit::of(&studio);
    assert!(lit.over(0.05).contains(&true), "the eyes are lit again");
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

/// The highlight drawn last frame, for looking at its texels.
struct Lit {
    /// Texels across.
    width: usize,
    /// Each texel's middle on the screen and its strength, row by row.
    texels: Vec<(Pos2, f32)>,
}

impl Lit {
    fn of(studio: &Studio) -> Self {
        let (width, texels) = studio.workspace.highlight();
        Self { width, texels }
    }

    /// The texels at least `strength` strong, as a grid of yes and no.
    fn over(&self, strength: f32) -> Vec<bool> {
        self.texels.iter().map(|(_, s)| *s >= strength).collect()
    }

    /// The sizes of the groups of texels of `grid` that are `lit` and
    /// touch one another side to side, and whether each touches the
    /// grid's edge.
    fn groups(&self, grid: &[bool], lit: bool) -> Vec<(usize, bool)> {
        let (width, height) = (self.width, grid.len() / self.width.max(1));
        let mut seen = vec![false; grid.len()];
        let mut groups = Vec::new();
        for start in 0..grid.len() {
            if seen[start] || grid[start] != lit {
                continue;
            }
            let (mut size, mut at_edge, mut open) = (0, false, vec![start]);
            seen[start] = true;
            while let Some(at) = open.pop() {
                size += 1;
                let (x, y) = (at % width, at / width);
                at_edge |= x == 0 || y == 0 || x + 1 == width || y + 1 == height;
                let mut visit = |next: usize| {
                    if !seen[next] && grid[next] == lit {
                        seen[next] = true;
                        open.push(next);
                    }
                };
                if x > 0 {
                    visit(at - 1);
                }
                if x + 1 < width {
                    visit(at + 1);
                }
                if y > 0 {
                    visit(at - width);
                }
                if y + 1 < height {
                    visit(at + width);
                }
            }
            groups.push((size, at_edge));
        }
        groups
    }
}

/// The screen points round `at` exact rays are cast through: itself and
/// `reach` points each way.
fn round(at: Pos2, reach: f32) -> [Pos2; 5] {
    [
        at,
        at + vec2(reach, 0.0),
        at - vec2(reach, 0.0),
        at + vec2(0.0, reach),
        at - vec2(0.0, reach),
    ]
}

/// The screen points within 6 points of `at` each way, a texel (2 points)
/// apart, `at` itself first: a sliver of a feature one texel wide (the
/// far shoulder past the neck) is still met.
fn within_six(at: Pos2) -> impl Iterator<Item = Pos2> {
    let steps = [0, -1, 1, -2, 2, -3, 3];
    steps
        .into_iter()
        .flat_map(move |y| steps.map(move |x| at + vec2(x as f32 * 2.0, y as f32 * 2.0)))
}

/// Everything the overlay draws is what the camera sees, checked with
/// exact rays against the posed character from the studio's views where
/// far-side skin lies behind near skin: the jaw from the Face view, the
/// eyes turned 70 degrees, the shoulders and a hovered waist from
/// three-quarter, the waist from the front (behind the arms) and the hips
/// turned 45 degrees. Each handle is on skin that can be seen. The
/// highlight is lit only within 6 points of where a ray first meets a
/// surface of the selected or the hovered feature (never the far shoulder
/// over the chest), and wherever rays meet the hovered feature well inside
/// it, facing the camera, the highlight is at its full strength: it covers
/// the feature.
#[test]
fn the_overlay_is_drawn_only_where_the_feature_shows() {
    let cases: [(&str, ViewPreset, Option<f32>, &str); 6] = [
        ("jaw", ViewPreset::Face, None, "jaw"),
        ("eyes", ViewPreset::Face, Some(70.0), "eyes"),
        ("shoulders", ViewPreset::ThreeQuarter, None, "waist"),
        ("shoulders", ViewPreset::ThreeQuarter, None, "shoulders"),
        ("waist", ViewPreset::Front, None, "waist"),
        ("hips", ViewPreset::Front, Some(45.0), "hips"),
    ];
    let [_, hover] = CharacterWorkspace::highlight_strengths();
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
        // What's under the pointer there (an anchor can be inside the
        // head: the skin in front of it may be another feature's).
        let hovered = studio.workspace.direct.hovered.clone();
        let hovered = hovered.as_deref().unwrap_or(selected);
        let case = format!("{selected} in {view:?} at {yaw:?} hovering {hovered}");
        let handles = studio.workspace.handles_drawn();
        let unseen = handles
            .iter()
            .filter(|point| !studio.workspace.in_sight(**point, 6.0, 0.01))
            .count();
        if unseen > 0 {
            faults.push(format!(
                "{case}: {unseen} of {} handles behind the skin",
                handles.len()
            ));
        }
        let lit = Lit::of(&studio);
        let shows: Vec<Pos2> = lit
            .texels
            .iter()
            .filter(|(_, strength)| *strength > 0.02)
            .map(|(at, _)| *at)
            .collect();
        assert!(!shows.is_empty(), "{case}: something is lit");
        // About 150 of the lit texels, spread out: one of the lit
        // features shows within 6 points of each.
        let mut weight = |feature: &str, at: Pos2| studio.workspace.weight_in_sight(feature, at);
        let step = (shows.len() / 150).max(1);
        let sampled: Vec<Pos2> = shows.iter().copied().step_by(step).collect();
        let astray = sampled
            .iter()
            .filter(|at| {
                !within_six(**at).any(|near| {
                    [selected, hovered].into_iter().any(|feature| {
                        weight(feature, near).is_some_and(|(weight, _, _)| weight >= 0.4)
                    })
                })
            })
            .count();
        if astray > 0 {
            faults.push(format!(
                "{case}: {astray} of {} sampled lit texels are not on a lit feature",
                sampled.len()
            ));
        }
        // And about 400 texels of the view: where the hovered feature
        // shows well inside it, it's lit in full (at least; where the
        // selected one is too, more).
        let step = (lit.texels.len() / 400).max(1);
        let (mut inside, mut dim) = (0, 0);
        for (at, strength) in lit.texels.iter().step_by(step) {
            let within = round(*at, 6.0).into_iter().all(|near| {
                weight(hovered, near)
                    .is_some_and(|(weight, _, facing)| weight >= 0.9 && facing >= 0.5)
            });
            if within {
                inside += 1;
                if *strength < hover - 0.01 {
                    dim += 1;
                }
            }
        }
        if dim > 0 {
            faults.push(format!(
                "{case}: {dim} of {inside} sampled texels well inside {hovered} are not lit in full"
            ));
        }
    }
    assert!(faults.is_empty(), "{faults:#?}");
}

/// Wears the realistic base's tee and joggers.
fn dress(studio: &mut Studio) {
    for (slot, item) in [
        ("top", "axcenity/realistic-crew-tee"),
        ("bottom", "axcenity/realistic-joggers"),
    ] {
        studio.workspace.apply(Intent::Wear {
            slot: slot.to_owned(),
            item: Some(item.to_owned()),
        });
    }
    studio.settle();
}

/// A feature of the body under clothes is lit on the outside of the
/// clothes, as one soft region (the bug this replaced: a tint of the
/// garment's own triangles tested against a depth buffer, which showed as
/// blotches with zig-zag outlines). With a tee and joggers on and the
/// waist, then the hips, under the pointer from the front:
///
/// - the highlight is there, and nearly all of it lies on cloth;
/// - what's lit is what's picked: the pointer anywhere inside it names
///   the same feature (the cloth is picked by the weights it's lit by);
/// - it's whole: no specks of highlight apart from it, no pinholes in it
///   (every group of lit texels and every unlit gap inside it is large);
/// - its edge is soft: along a line across its border the strength is
///   even inside, rises once to the edge's rim and falls from full to
///   nothing over 4 to 14 points, never rising again on the way out.
#[test]
fn a_clothed_feature_is_lit_on_the_cloth_as_one_soft_region() {
    let [_, hover] = CharacterWorkspace::highlight_strengths();
    let mut studio = Studio::new("handles-clothed");
    dress(&mut studio);
    studio.workspace.apply(Intent::PickCategory(Category::Body));
    studio.workspace.apply(Intent::FocusView(ViewPreset::Front));
    studio.settle();
    for (feature, handle) in [("waist", "waist_left"), ("hips", "hip_left")] {
        let side = studio.workspace.handle_point(handle).expect("on screen");
        let middle = studio.workspace.direct.view_rect.unwrap().center().x;
        let at = side + vec2(8.0 * (middle - side.x).signum(), 0.0);
        studio.hover(at);
        assert_eq!(studio.workspace.direct.hovered.as_deref(), Some(feature));
        let lit = Lit::of(&studio);
        let full = lit.over(hover - 0.01);
        let count = full.iter().filter(|lit| **lit).count();
        assert!(count > 200, "{feature}: {count} texels lit in full");

        // On the cloth.
        let texels: Vec<Pos2> = lit
            .texels
            .iter()
            .zip(&full)
            .filter(|(_, lit)| **lit)
            .map(|((at, _), _)| *at)
            .collect();
        let sampled: Vec<Pos2> = texels
            .iter()
            .copied()
            .step_by((count / 150).max(1))
            .collect();
        let on_cloth = sampled
            .iter()
            .filter(|at| {
                studio
                    .workspace
                    .weight_in_sight(feature, **at)
                    .is_some_and(|(_, worn, _)| worn)
            })
            .count();
        assert!(
            on_cloth * 10 >= sampled.len() * 9,
            "{feature}: {on_cloth} of {} sampled lit texels are on cloth",
            sampled.len()
        );

        // What's lit is what's picked: the pointer anywhere well inside
        // it (4 texels from its edge or more) is over the same feature.
        let inside: Vec<Pos2> = (0..lit.texels.len())
            .filter(|at| {
                let (x, y) = ((at % lit.width) as i32, (at / lit.width) as i32);
                (-4..=4).step_by(4).all(|dy: i32| {
                    (-4..=4).step_by(4).all(|dx: i32| {
                        let (x, y) = (x + dx, y + dy);
                        x >= 0
                            && y >= 0
                            && (x as usize) < lit.width
                            && full
                                .get(y as usize * lit.width + x as usize)
                                .is_some_and(|lit| *lit)
                    })
                })
            })
            .map(|at| lit.texels[at].0)
            .collect();
        assert!(
            inside.len() > 50,
            "{feature}: {} texels inside",
            inside.len()
        );
        for point in inside.iter().step_by((inside.len() / 12).max(1)) {
            studio.hover(*point);
            assert_eq!(
                studio.workspace.direct.hovered.as_deref(),
                Some(feature),
                "under the pointer at {point:?}"
            );
        }
        studio.hover(at);

        // Whole: no specks, no pinholes.
        let shows = lit.over(hover * 0.5);
        let specks: Vec<usize> = lit
            .groups(&shows, true)
            .into_iter()
            .map(|(size, _)| size)
            .filter(|size| *size < 60)
            .collect();
        assert!(specks.is_empty(), "{feature}: specks of {specks:?} texels");
        let pinholes: Vec<usize> = lit
            .groups(&shows, false)
            .into_iter()
            .filter(|(size, at_edge)| !at_edge && *size < 60)
            .map(|(size, _)| size)
            .collect();
        assert!(
            pinholes.is_empty(),
            "{feature}: pinholes of {pinholes:?} texels"
        );

        // Soft, and falling once: up a column of texels from inside the
        // feature (half way from the pointer to the body's middle) to
        // above it.
        let from = egui::pos2((at.x + middle) / 2.0, at.y);
        let column = lit
            .texels
            .iter()
            .position(|(texel, _)| {
                (texel.x - from.x).abs() <= 1.0 && (texel.y - from.y).abs() <= 1.0
            })
            .expect("the point is in the view");
        let up: Vec<f32> = (0..=column / lit.width)
            .map(|rows| lit.texels[column - rows * lit.width].1)
            .collect();
        assert!(up[0] >= hover - 0.01, "{feature}: lit at {from:?}");
        let out = up
            .iter()
            .position(|strength| *strength == 0.0)
            .expect("the feature ends above the pointer");
        let up = &up[..=out];
        // Even inside, a little stronger along the edge (the rim), then
        // down to nothing: it never falls and rises again.
        let strongest = up.iter().copied().fold(0.0, f32::max);
        let rim = up.iter().position(|s| *s == strongest).unwrap();
        assert!(
            up[..=rim].windows(2).all(|pair| pair[1] >= pair[0] - 1e-6)
                && up[rim..].windows(2).all(|pair| pair[1] <= pair[0] + 1e-6),
            "{feature}: the edge rises to its rim and falls once: {up:?}"
        );
        let last_full = up.iter().rposition(|s| *s >= hover - 0.01).unwrap();
        let fade = out - last_full;
        assert!(
            (2..=7).contains(&fade),
            "{feature}: the edge fades over {fade} texels: {up:?}"
        );
    }
}

/// The highlight is made again only when what it shows changes: frames
/// with the pointer still, the camera still and the same pose draw the
/// texture as it is.
#[test]
fn a_still_view_keeps_its_highlight() {
    let mut studio = Studio::new("handles-still");
    studio.workspace.apply(Intent::PickCategory(Category::Body));
    studio.workspace.apply(Intent::FocusView(ViewPreset::Front));
    studio
        .workspace
        .apply(Intent::PreviewMotion(super::motion::Motion::Still));
    studio.settle();
    let hip = studio.workspace.handle_point("hip_left").unwrap();
    let middle = studio.workspace.direct.view_rect.unwrap().center().x;
    let hip = hip + vec2(6.0 * (middle - hip.x).signum(), 0.0);
    studio.hover(hip);
    assert_eq!(studio.workspace.direct.hovered.as_deref(), Some("hips"));
    let made = studio.workspace.highlight_rebuilds();
    assert!(made > 0, "the highlight was made");
    for _ in 0..5 {
        studio.frame(vec![egui::Event::PointerMoved(hip)]);
    }
    assert_eq!(studio.workspace.highlight_rebuilds(), made);
    assert!(
        !Lit::of(&studio).over(0.05).is_empty(),
        "and it's still drawn"
    );
    // The camera turns: made again.
    studio.turn(20.0);
    studio.hover(hip);
    assert!(studio.workspace.highlight_rebuilds() > made);
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

/// Drawn for real with `AXCENITY_SNAPSHOTS`: a dressed character (the
/// tee and joggers) with the hips, the waist and the shoulders under the
/// pointer from the front, the hips again close up and turned, and the
/// waist selected with its handles. The highlight lies on the cloth.
#[test]
fn a_clothed_character_s_highlight_is_drawn() {
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let mut studio = Studio::new("handles-clothed-snapshots");
    dress(&mut studio);
    studio.workspace.apply(Intent::PickCategory(Category::Body));
    let snap = |studio: &mut Studio, name: &str, at: Pos2| {
        let workspace = &mut studio.workspace;
        workspace.canvas = Canvas::default();
        workspace.other_canvas = Canvas::default();
        crate::snapshot::snapshot_rendered(
            &snapshots.join(format!("character-studio-handles-clothed-{name}.png")),
            egui::vec2(1440.0, 900.0),
            vec![vec![egui::Event::PointerMoved(at)]; 4],
            |ui, render_state| {
                workspace.ui(ui, Some(render_state));
            },
        );
    };
    // Each: a name, the handle the pointer is beside, the camera's turn
    // and how near it is (the view's own when `None`).
    type Shot<'a> = (&'a str, &'a str, Option<f32>, Option<f32>);
    let shots: [Shot; 6] = [
        ("hips", "hip_left", None, None),
        ("waist", "waist_left", None, None),
        ("shoulders", "shoulder_left", None, None),
        ("hips-close", "hip_left", None, Some(5.0)),
        ("hips-turned", "hip_left", Some(40.0), Some(6.0)),
        ("waist-turned", "waist_left", Some(-60.0), Some(6.0)),
    ];
    for (name, handle, yaw, distance) in shots {
        studio.workspace.apply(Intent::PickFeature(None));
        studio.workspace.apply(Intent::FocusView(ViewPreset::Front));
        studio.settle();
        if let Some(yaw) = yaw {
            studio.workspace.orbit.yaw = yaw;
        }
        if let Some(distance) = distance {
            let target = studio.workspace.handle_world(handle).unwrap();
            studio.workspace.orbit.distance = distance;
            studio.workspace.orbit.target.y = target.y;
        }
        studio.settle();
        let side = studio.workspace.handle_point(handle).unwrap();
        let middle = studio.workspace.direct.view_rect.unwrap().center().x;
        let at = side + vec2(10.0 * (middle - side.x).signum(), 0.0);
        snap(&mut studio, name, at);
    }
    studio.select("waist", ViewPreset::Front);
    let at = studio.handle("waist_left");
    snap(&mut studio, "waist-selected", at + vec2(0.0, 120.0));
}

/// What direct manipulation costs a frame: the studio's frame (no GPU)
/// with it off, with the eyes selected and the pointer off the character
/// (the buffer of what the camera sees, the handles and the highlight), and
/// with the pointer moving over the face (a ray cast every frame, the
/// hovered feature's highlight too).
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

/// What the highlight costs each time it's made, on the default realistic
/// character in a tee and joggers: the studio's frame (no GPU) with the
/// pointer over a feature against the same frame with editing on the
/// character off. The difference is looking at the character (rasterizing
/// the body and the clothes as posed into the viewport's buffer, a texel
/// per 2 points), making the mask from that (the weights, the blur, the
/// texture's update) and the ray under the pointer. With the hips under
/// the pointer from the front, with the face under it close up (the mask
/// is larger on the screen), and with the waist selected too. The idle
/// motion moves the pose every frame, so each frame makes it again. Also
/// the first frame over the clothes, which works out where they lie on
/// the body (`direct/worn.rs`). Ignored; run it alone in a release build:
/// `cargo test -p axcenity-editor --release -- --ignored highlight_costs --nocapture`.
#[test]
#[ignore = "timing: run alone with --release -- --ignored highlight_costs"]
fn highlight_costs_per_rebuild() {
    let mut studio = Studio::new("handles-highlight-timing");
    dress(&mut studio);
    studio.workspace.apply(Intent::PickCategory(Category::Body));
    studio.workspace.apply(Intent::FocusView(ViewPreset::Front));
    studio.settle();
    let rect = studio.workspace.direct.view_rect.unwrap();
    println!(
        "viewport {} by {} points: a buffer of {} by {} texels",
        rect.width(),
        rect.height(),
        (rect.width() / 2.0).ceil(),
        (rect.height() / 2.0).ceil()
    );
    let side = studio.workspace.handle_point("hip_left").unwrap();
    let hips = side + vec2(8.0 * (rect.center().x - side.x).signum(), 0.0);
    let start = std::time::Instant::now();
    studio.frame(vec![egui::Event::PointerMoved(hips)]);
    println!(
        "first frame over the clothes: {:.1} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
    // The median of 60 frames, the pointer a point to and fro at `at`.
    let median = |studio: &mut Studio, at: Pos2| {
        let mut times: Vec<f64> = (0..60)
            .map(|frame| {
                let start = std::time::Instant::now();
                studio.frame(vec![egui::Event::PointerMoved(
                    at + vec2((frame % 2) as f32, 0.0),
                )]);
                start.elapsed().as_secs_f64() * 1000.0
            })
            .collect();
        times.sort_by(f64::total_cmp);
        times[times.len() / 2]
    };
    let measure = |studio: &mut Studio, name: &str, at: Pos2| {
        let made = studio.workspace.highlight_rebuilds();
        let on = median(studio, at);
        let made = studio.workspace.highlight_rebuilds() - made;
        let hovered = studio.workspace.direct.hovered.clone();
        let lit = Lit::of(studio)
            .over(0.02)
            .iter()
            .filter(|lit| **lit)
            .count();
        studio.workspace.apply(Intent::SetHandles(false));
        let off = median(studio, at);
        studio.workspace.apply(Intent::SetHandles(true));
        studio.hover(at);
        println!(
            "{name} ({hovered:?}, {lit} texels lit, made {made} times in 60 frames): the frame \
             {on:.2} ms, {off:.2} ms with editing on the character off: {:.2} ms for it",
            on - off
        );
    };
    measure(&mut studio, "hips, front", hips);
    studio.workspace.apply(Intent::FocusView(ViewPreset::Face));
    studio.settle();
    studio.workspace.orbit.distance = 1.6;
    studio.settle();
    let lips = studio.workspace.handle_point("mouth_corner_left").unwrap();
    measure(&mut studio, "the face, close", lips);
    studio.select("waist", ViewPreset::Front);
    measure(&mut studio, "waist selected, hips hovered", hips);
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
