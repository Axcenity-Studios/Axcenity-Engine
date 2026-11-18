//! The Expressions page's state (refinement R9, brief §10): the built-in
//! expression shown, the face controls held over it, and the face preview
//! mode. All of it is editor state: it never enters the description, undo
//! or a saved file (the character's identity stays apart from what its
//! face does).
//!
//! The preview's face is a FaceControls of its own on the body
//! ([`PREVIEW_FACE`](super::PREVIEW_FACE)), as the capture tool's: the
//! expression plays on its Expression layer, and the controls a person
//! moves are gameplay overrides (`FaceState::overrides`, the Override
//! layer, which goes over everything under it), so a slider shows exactly
//! the value it says. A slider not moved shows what the expression gives
//! its control.
//!
//! The face preview frames the face straight on and close, holds the body
//! still (the rest pose), turns the face's blinking and idle motion off
//! (each can be asked for) and lights it with the neutral light; leaving
//! it puts back the camera, view, light and motion it found.

use std::collections::BTreeMap;

use axcenity_core::face::face_control;
use axcenity_core::{ClassName, InstanceId, Value};
use axcenity_gameplay::description as applier;
use axcenity_render::glam::Vec3;

use super::motion::Motion;
use super::stage::{Lighting, Orbit, ViewPreset};
use super::{CharacterWorkspace, PREVIEW_FACE};

/// The face's parts, as the page groups its controls (brief §10's facial
/// controls: smile, brows, squint, mouth opening, jaw, blink).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceGroup {
    Brows,
    Eyes,
    Cheeks,
    Nose,
    Mouth,
    Jaw,
}

impl FaceGroup {
    pub const ALL: [FaceGroup; 6] = [
        FaceGroup::Brows,
        FaceGroup::Eyes,
        FaceGroup::Cheeks,
        FaceGroup::Nose,
        FaceGroup::Mouth,
        FaceGroup::Jaw,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FaceGroup::Brows => "Brows",
            FaceGroup::Eyes => "Eyes",
            FaceGroup::Cheeks => "Cheeks",
            FaceGroup::Nose => "Nose",
            FaceGroup::Mouth => "Mouth",
            FaceGroup::Jaw => "Jaw",
        }
    }
}

/// Which standard face controls (`crates/core/src/face.rs`) a row moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowControls {
    /// `<stem>_Left` and `<stem>_Right` together, 0 to 1.
    Pair(&'static str),
    /// One control, 0 to 1.
    One(&'static str),
    /// Two opposite controls as one slider from -1 (`less` all the way)
    /// to 1 (`more` all the way).
    Signed {
        less: &'static str,
        more: &'static str,
    },
}

impl RowControls {
    /// The slider's range.
    pub fn range(self) -> [f32; 2] {
        match self {
            RowControls::Signed { .. } => [-1.0, 1.0],
            _ => [0.0, 1.0],
        }
    }

    /// Each control the row moves and the value it gets for slider value
    /// `value`.
    pub fn values(self, value: f32) -> Vec<(String, f32)> {
        match self {
            RowControls::Pair(stem) => vec![
                (format!("{stem}_Left"), value.clamp(0.0, 1.0)),
                (format!("{stem}_Right"), value.clamp(0.0, 1.0)),
            ],
            RowControls::One(name) => vec![(name.to_owned(), value.clamp(0.0, 1.0))],
            RowControls::Signed { less, more } => vec![
                (less.to_owned(), (-value).clamp(0.0, 1.0)),
                (more.to_owned(), value.clamp(0.0, 1.0)),
            ],
        }
    }
}

/// One slider of the page.
pub struct Row {
    pub group: FaceGroup,
    pub label: &'static str,
    /// Its tooltip.
    pub about: &'static str,
    pub controls: RowControls,
}

const fn row(
    group: FaceGroup,
    label: &'static str,
    about: &'static str,
    controls: RowControls,
) -> Row {
    Row {
        group,
        label,
        about,
        controls,
    }
}

use FaceGroup::{Brows, Cheeks, Eyes, Jaw, Mouth, Nose};
use RowControls::{One, Pair, Signed};

/// The page's sliders, by group. Left and right move together: the page
/// previews faces, it doesn't author them (the Game Editor's Face Rig tab
/// does, DIS-07).
pub const ROWS: &[Row] = &[
    row(
        Brows,
        "Raise",
        "Both brows up, as in surprise",
        Pair("Brow_Raise"),
    ),
    row(
        Brows,
        "Lower",
        "Both brows down and together, as in anger",
        Pair("Brow_Lower"),
    ),
    row(
        Brows,
        "Inner ends up",
        "The brows' inner ends up and in, as in worry",
        Pair("Brow_Inward"),
    ),
    row(Eyes, "Blink", "Closes both eyes", Pair("Blink")),
    row(
        Eyes,
        "Squint",
        "Narrows both eyes from below",
        Pair("Eye_Squint"),
    ),
    row(Eyes, "Wide", "Opens both eyes wide", Pair("Eye_Wide")),
    row(
        Eyes,
        "Look down or up",
        "Turns the eyes down (left of the middle) or up (right of it)",
        Signed {
            less: "Eye_Look_Down",
            more: "Eye_Look_Up",
        },
    ),
    row(
        Eyes,
        "Look sideways",
        "Turns the eyes to the character's right (left of the middle) or its left",
        Signed {
            less: "Eye_Look_Right",
            more: "Eye_Look_Left",
        },
    ),
    row(
        Cheeks,
        "Raise",
        "Lifts the cheeks, as in a broad smile",
        Pair("Cheek_Raise"),
    ),
    row(Cheeks, "Puff", "Puffs both cheeks out", One("Cheek_Puff")),
    row(
        Nose,
        "Sneer",
        "Wrinkles the nose up, as in disgust",
        Pair("Nose_Sneer"),
    ),
    row(
        Mouth,
        "Smile",
        "Pulls the corners of the mouth up and out",
        Pair("Mouth_Smile"),
    ),
    row(
        Mouth,
        "Frown",
        "Pulls the corners of the mouth down",
        Pair("Mouth_Frown"),
    ),
    row(
        Mouth,
        "Open lips",
        "Parts the lips without the jaw",
        One("Mouth_Open"),
    ),
    row(Mouth, "Wide", "Stretches the mouth wide", One("Mouth_Wide")),
    row(
        Mouth,
        "Narrow",
        "Rounds the mouth, as for \"oo\"",
        One("Mouth_Narrow"),
    ),
    row(
        Mouth,
        "Pucker",
        "Pushes the lips forward together",
        One("Mouth_Pucker"),
    ),
    row(
        Mouth,
        "Upper lip up",
        "Raises the upper lip off the teeth",
        One("Upper_Lip_Raise"),
    ),
    row(
        Mouth,
        "Lower lip down",
        "Lowers the lower lip off the teeth",
        One("Lower_Lip_Lower"),
    ),
    row(
        Mouth,
        "Corners back",
        "Draws the corners back toward the ears",
        Pair("Lip_Corner_Pull"),
    ),
    row(
        Mouth,
        "Sideways",
        "Moves the mouth to the character's right (left of the middle) or its left",
        Signed {
            less: "Mouth_Right",
            more: "Mouth_Left",
        },
    ),
    row(
        Jaw,
        "Open",
        "Drops the jaw: the mouth opens",
        One("Jaw_Open"),
    ),
    row(Jaw, "Forward", "Pushes the jaw forward", One("Jaw_Forward")),
    row(
        Jaw,
        "Sideways",
        "Moves the jaw to the character's right (left of the middle) or its left",
        Signed {
            less: "Jaw_Right",
            more: "Jaw_Left",
        },
    ),
    row(
        Jaw,
        "Tongue out",
        "Puts the tongue out over the lower lip",
        One("Tongue_Out"),
    ),
];

/// What the face preview put aside, to put back when it ends.
#[derive(Clone, Debug, PartialEq)]
struct Restore {
    orbit: Orbit,
    view: Option<ViewPreset>,
    lighting: Lighting,
    motion: Motion,
}

/// The face preview mode's state (editor state).
#[derive(Clone, Debug, PartialEq)]
pub struct FaceView {
    /// The face blinks (off unless asked for).
    pub blink: bool,
    /// The face's idle motion and the body's Idle (off unless asked for).
    pub idle: bool,
    restore: Restore,
    /// Frame the face again after the next pose (the body's pose changed).
    reframe: bool,
}

/// The face preview's camera: straight on, a little closer than the
/// capture tool's `face-front` (2.3 studs), aimed between the eyes and
/// the mouth (about 0.06 studs over the head bone).
const FACE_DISTANCE: f32 = 2.0;
const FACE_ABOVE_HEAD_BONE: f32 = 0.06;

/// The face's idle motion when it's asked for (FaceControls' default).
const IDLE_MOTION: f64 = 0.5;

impl CharacterWorkspace {
    /// The value the preview's face holds control `name` at: what the
    /// page set it to, or else what the expression shown gives it.
    pub fn face_value(&self, name: &str) -> f32 {
        match self.face_controls.get(name) {
            Some(value) => *value,
            None => self.expression_value(name),
        }
    }

    /// What the expression shown gives control `name` (0 without one).
    pub fn expression_value(&self, name: &str) -> f32 {
        self.expression
            .as_deref()
            .and_then(axcenity_anim::built_in_expression)
            .and_then(|expression| {
                expression
                    .controls
                    .get(name)
                    .map(|value| value * expression.intensity)
            })
            .unwrap_or(0.0)
    }

    /// A row's slider value from the controls it moves: as held
    /// ([`face_value`](Self::face_value)) or, with `expression_only`, as
    /// the expression alone gives them (what the row's reset goes back to).
    pub fn row_value(&self, controls: RowControls, expression_only: bool) -> f32 {
        let value = |name: &str| {
            if expression_only {
                self.expression_value(name)
            } else {
                self.face_value(name)
            }
        };
        match controls {
            RowControls::Pair(stem) => {
                value(&format!("{stem}_Left")).max(value(&format!("{stem}_Right")))
            }
            RowControls::One(name) => value(name),
            RowControls::Signed { less, more } => value(more) - value(less),
        }
    }

    /// Shows `name` (a built-in expression) on the preview's face, or none
    /// (`None` or Neutral): the controls held start again from its values.
    /// The description is not touched.
    pub fn preview_expression(&mut self, name: Option<String>) {
        let name = name.filter(|name| !name.eq_ignore_ascii_case("Neutral"));
        self.expression = name;
        self.face_controls.clear();
        self.sync_preview_face(true);
    }

    /// Holds control `name` at `value` over the expression, or lets it go.
    pub(super) fn preview_face_control(&mut self, name: &str, value: Option<f32>) {
        if face_control(name).is_none() {
            return;
        }
        match value {
            Some(value) => {
                self.face_controls
                    .insert(name.to_owned(), value.clamp(0.0, 1.0));
            }
            None => {
                self.face_controls.remove(name);
            }
        }
        self.sync_preview_face(false);
    }

    /// Turns the face preview on or off.
    pub(super) fn face_preview(&mut self, on: bool) {
        match (on, self.face_view.is_some()) {
            (true, false) => {
                self.face_view = Some(FaceView {
                    blink: false,
                    idle: false,
                    restore: Restore {
                        orbit: self.orbit,
                        view: self.view,
                        lighting: self.lighting,
                        motion: self.motion.clone(),
                    },
                    reframe: true,
                });
                self.lighting = Lighting::Neutral;
                if self.motion != Motion::Still {
                    self.preview_motion(Motion::Still);
                }
                self.frame_face();
                self.sync_preview_face(false);
            }
            (false, true) => {
                let Some(view) = self.face_view.take() else {
                    return;
                };
                let restore = view.restore;
                if self.motion != restore.motion {
                    self.preview_motion(restore.motion);
                }
                // (After the motion: stopping one reframes the view.)
                self.reframe = false;
                self.orbit = restore.orbit;
                self.view = restore.view;
                self.lighting = restore.lighting;
                self.sync_preview_face(false);
            }
            _ => {}
        }
    }

    /// In the face preview: blinking, and idle motion (the face's and the
    /// body's Idle).
    pub(super) fn face_preview_motion(&mut self, blink: bool, idle: bool) {
        let Some(view) = &mut self.face_view else {
            return;
        };
        let idle_changed = view.idle != idle;
        view.blink = blink;
        view.idle = idle;
        if idle_changed {
            view.reframe = true;
            self.preview_motion(if idle { Motion::Idle } else { Motion::Still });
        }
        self.sync_preview_face(false);
    }

    /// Frames the face straight on and close (the face preview's camera).
    fn frame_face(&mut self) {
        let Some(head) = self.head() else {
            return;
        };
        self.orbit = Orbit {
            yaw: 0.0,
            pitch: 1.0,
            distance: FACE_DISTANCE,
            target: head + Vec3::new(0.0, FACE_ABOVE_HEAD_BONE, -0.05),
        };
        self.view = None;
    }

    /// After a step: the face preview frames the face again once the body
    /// has taken its new pose.
    pub(super) fn follow_face_preview(&mut self) {
        if let Some(view) = &mut self.face_view
            && std::mem::take(&mut view.reframe)
        {
            self.frame_face();
        }
    }

    /// Ends what the page shows: no expression, no control held, no face
    /// preview (leaving the page; a new character).
    pub(super) fn leave_expressions(&mut self) {
        self.face_preview(false);
        self.preview_expression(None);
    }

    /// The preview's FaceControls, if it has one.
    fn preview_face_controls(&self) -> Option<InstanceId> {
        let body = applier::body_of(&self.world, self.character?)?;
        self.world
            .get(body)?
            .children()
            .iter()
            .copied()
            .find(|child| {
                self.world.get(*child).is_some_and(|child| {
                    child.class() == ClassName::FaceControls && child.name() == PREVIEW_FACE
                })
            })
    }

    /// Puts what the page shows onto the preview's FaceControls: made when
    /// something is shown (an expression, a control held, the face
    /// preview), taken away when nothing is. `restart` plays the
    /// expression from the start (a new one was picked).
    pub(super) fn sync_preview_face(&mut self, restart: bool) {
        let wanted =
            self.expression.is_some() || !self.face_controls.is_empty() || self.face_view.is_some();
        let existing = self.preview_face_controls();
        if !wanted {
            if let Some(face) = existing {
                let _ = self.world.destroy(face);
            }
            return;
        }
        let Some(body) = self
            .character
            .and_then(|character| applier::body_of(&self.world, character))
        else {
            return;
        };
        let face = match existing {
            Some(face) => face,
            None => {
                let Ok(face) = self.world.create(ClassName::FaceControls) else {
                    return;
                };
                let _ = self.world.set_name(face, PREVIEW_FACE);
                let _ = self.world.set_parent(face, Some(body));
                face
            }
        };
        // The face preview holds the face still unless asked; elsewhere on
        // the page the face is alive (the FaceControls' defaults).
        let (blink, idle) = match &self.face_view {
            Some(view) => (view.blink, if view.idle { IDLE_MOTION } else { 0.0 }),
            None => (true, IDLE_MOTION),
        };
        let _ = self
            .world
            .set_property(face, "BlinkEnabled", Value::Bool(blink));
        let _ = self
            .world
            .set_property(face, "IdleMotion", Value::Number(idle));
        let overrides: BTreeMap<usize, f64> = self
            .face_controls
            .iter()
            .filter_map(|(name, value)| Some((face_control(name)?, f64::from(*value))))
            .collect();
        let expression = self.expression.clone();
        if let Some(state) = self.world.face_mut(face) {
            state.overrides = overrides;
            let playing = state.expressions.first().map(|play| play.name.clone());
            if restart || playing != expression {
                state.expressions.clear();
                if let Some(name) = expression {
                    let order = state.next_order;
                    state.next_order += 1;
                    state.expressions.push(axcenity_core::face::ExpressionPlay {
                        name,
                        intensity: 1.0,
                        target_intensity: 1.0,
                        presence: 0.0,
                        target_presence: 1.0,
                        transition: None,
                        weight: 1.0,
                        priority: 0,
                        asymmetry: 0.0,
                        order,
                    });
                }
            }
        }
    }
}
