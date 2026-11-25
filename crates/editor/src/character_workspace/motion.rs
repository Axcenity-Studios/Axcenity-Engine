//! Pose and motion in the preview (refinement R9, brief §10 "Animation
//! preview"): **Still** (the rest pose, the A-pose), **Idle** (the default
//! character's state machine, which the preview has always played),
//! **Walk**, **Run** and the default character's gestures (Wave, Point,
//! Cheer, Clap, Shrug). Like an expression, it is editor state: it never
//! enters the description, undo or a saved file, and an expression still
//! shows on top of it.
//!
//! A take plays as the capture tool plays a clip: the Animator's state
//! machine is cleared and an Animation of the preview's own, looped, plays
//! the take alone. **Walk is the Walk take at its own speed** (1.07 s a
//! cycle), in place: the game's Walk state plays the Run take faster,
//! because the controller's default `WalkSpeed` (16 studs/s, 5.6 m/s) is a
//! jog (decided at the R1 merge), so the true walk is only seen here. Run
//! is the Run take at its own speed. The takes are in place (their hips
//! end where they start), so the character stays on its floor.
//!
//! The clips are written once into the workspace's folder
//! ([`PREVIEW_CLIPS_FOLDER`]), each linked to its take in the default
//! humanoid's animation file. While a take plays, the camera follows the
//! head ([`CharacterWorkspace::follow_motion`]), smoothed so a step's bob
//! doesn't shake the view: a close view follows it whole, a far one only
//! sideways, so the floor doesn't bounce.

use std::path::Path;
use std::sync::OnceLock;

use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_gameplay::description as applier;
#[cfg(test)]
use axcenity_render::glam::Vec3;

use super::CharacterWorkspace;

/// The gestures the preview offers, as named by the starter animations'
/// takes (Quaternius's library): talking, a nod, a shake of the head, a
/// dance and reaching out to something.
pub const GESTURES: [&str; 5] = ["IdleTalking", "Yes", "No", "Dance", "Interact"];

/// Where the preview's clips are written, in the workspace's folder.
pub const PREVIEW_CLIPS_FOLDER: &str = "assets/animations/Humanoid/Preview";

/// The name of the Animation the preview adds to play a take.
pub const PREVIEW_MOTION: &str = "Studio Motion Preview";

/// A close view (the face) follows the head up and down too.
const CLOSE: f32 = 6.0;

/// How fast the camera catches up with the head, per second.
const FOLLOW_RATE: f32 = 6.0;

/// What the preview's body does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    /// The rest pose (the A-pose): nothing plays.
    Still,
    /// The state machine's Idle: what the preview always played.
    #[default]
    Idle,
    /// The Walk take at its own speed, in place.
    Walk,
    /// The Run take at its own speed, in place.
    Run,
    /// One of [`GESTURES`], looped.
    Gesture(String),
}

impl Motion {
    /// Every choice, in the order the controls list them.
    pub fn all() -> Vec<Motion> {
        let mut all = vec![Motion::Still, Motion::Idle, Motion::Walk, Motion::Run];
        all.extend(
            GESTURES
                .iter()
                .map(|name| Motion::Gesture((*name).to_owned())),
        );
        all
    }

    pub fn label(&self) -> &str {
        match self {
            Motion::Still => "Still",
            Motion::Idle => "Idle",
            Motion::Walk => "Walk",
            Motion::Run => "Run",
            Motion::Gesture(name) => name,
        }
    }

    pub fn about(&self) -> String {
        match self {
            Motion::Still => "The rest pose (the A-pose), with nothing playing".to_owned(),
            Motion::Idle => "Standing, breathing and shifting a little".to_owned(),
            Motion::Walk => "A walk at its own pace, in place (a preview only)".to_owned(),
            Motion::Run => "A run at its own pace, in place (a preview only)".to_owned(),
            Motion::Gesture(name) => format!("{name}, over and over (a preview only)"),
        }
    }

    /// The take it plays, if it plays one.
    pub fn take(&self) -> Option<&str> {
        match self {
            Motion::Still | Motion::Idle => None,
            Motion::Walk => Some("Walk"),
            Motion::Run => Some("Run"),
            Motion::Gesture(name) => Some(name),
        }
    }

    /// Whether the body moves about (the camera follows it).
    pub fn moves(&self) -> bool {
        self.take().is_some()
    }
}

/// Each take's length in the default humanoid's animation file, read once.
fn take_lengths() -> &'static [(String, f64)] {
    static LENGTHS: OnceLock<Vec<(String, f64)>> = OnceLock::new();
    LENGTHS.get_or_init(|| {
        axcenity_assets::load_takes(
            axcenity_project::DEFAULT_HUMANOID_ANIMATIONS,
            axcenity_assets::ModelFormat::Gltf,
        )
        .unwrap_or_default()
        .into_iter()
        .map(|take| (take.name, f64::from(take.duration)))
        .collect()
    })
}

/// The clip that plays `take` once round at its own speed, looped, as a
/// file's text.
pub fn preview_clip(take: &str) -> Option<String> {
    let (_, length) = take_lengths().iter().find(|(name, _)| name == take)?;
    let mut clip = axcenity_anim::ClipFile::new(*length);
    clip.looping = true;
    clip.link = Some(axcenity_anim::TakeLink {
        source: axcenity_project::DEFAULT_HUMANOID_ANIMATIONS_PATH.to_owned(),
        take: take.to_owned(),
        start: 0.0,
        end: *length,
        // In place: the take's hips don't travel (only the `_RootMotion`
        // takes do), and the preview stands on its floor.
        root_motion: false,
        retarget: None,
    });
    Some(clip.to_json())
}

/// Writes the preview's clip of `take` into `folder` when it isn't there
/// as it should be; returns its path in the folder.
fn write_clip(folder: &Path, take: &str) -> Result<String, String> {
    let text = preview_clip(take)
        .ok_or_else(|| format!("The default character has no \"{take}\" animation."))?;
    let path = format!("{PREVIEW_CLIPS_FOLDER}/{take}.anim.json");
    let file = folder.join(&path);
    if std::fs::read_to_string(&file).ok().as_deref() != Some(text.as_str()) {
        file.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, text))
            .map_err(|err| format!("Could not write the {take} preview: {err}"))?;
    }
    Ok(path)
}

/// Poses `body` for `motion`: its Animator's state machine on for Idle,
/// off otherwise, and the preview's Animation playing `clip` (or gone).
fn pose_body(world: &mut World, body: InstanceId, idle: bool, clip: Option<&str>) {
    if let Some(animator) = world.find_first_child(body, "Animator") {
        let machine = if idle {
            axcenity_project::DEFAULT_HUMANOID_STATE_MACHINE_PATH
        } else {
            ""
        };
        let _ = world.set_property(animator, "StateMachine", Value::String(machine.to_owned()));
    }
    let existing = world.get(body).and_then(|body| {
        body.children().iter().copied().find(|child| {
            world.get(*child).is_some_and(|child| {
                child.class() == ClassName::Animation && child.name() == PREVIEW_MOTION
            })
        })
    });
    let Some(clip) = clip else {
        if let Some(animation) = existing {
            let _ = world.destroy(animation);
        }
        return;
    };
    let animation = match existing {
        Some(animation) => animation,
        None => {
            let Ok(animation) = world.create(ClassName::Animation) else {
                return;
            };
            let _ = world.set_name(animation, PREVIEW_MOTION);
            let _ = world.set_parent(animation, Some(body));
            animation
        }
    };
    let _ = world.set_property(animation, "Clip", Value::String(clip.to_owned()));
    let _ = world.set_property(animation, "Looped", Value::Bool(true));
    let _ = world.set_property(animation, "Speed", Value::Number(1.0));
    if let Some(state) = world.property_animation_mut(animation) {
        state.play = true;
    }
}

impl CharacterWorkspace {
    /// Plays `motion` on the preview (and on the held version beside it):
    /// editor state, never in the description or undo.
    pub fn preview_motion(&mut self, motion: Motion) {
        let clip = match motion.take() {
            Some(take) => match write_clip(&self.folder, take) {
                Ok(path) => Some(path),
                Err(error) => {
                    self.notice = Some(error);
                    return;
                }
            },
            None => None,
        };
        let was_moving = self.motion.moves();
        self.motion = motion;
        self.preview_clip = clip;
        self.pose_previews();
        // The camera follows from where the head is now, as framed: a take
        // that leans the body (a run) moves the head at once, and the
        // camera catches up with it. From one take to the next it keeps
        // the point it was following: the head is a bob away from that
        // point, and starting again from the head would leave the view
        // off the framing by that much for as long as takes play.
        self.follow = match (self.motion.moves(), was_moving, self.follow) {
            (true, true, Some(following)) => Some(following),
            (true, ..) => self.head(),
            (false, ..) => None,
        };
        // Back to the view's own framing once nothing moves, after the
        // next frame has posed the body (the Face view aims at the head).
        self.reframe = was_moving && !self.motion.moves();
    }

    /// Poses the main preview and the one beside it for the motion shown.
    pub(super) fn pose_previews(&mut self) {
        let idle = self.motion == Motion::Idle;
        let bodies: Vec<InstanceId> = [self.character, self.other]
            .into_iter()
            .flatten()
            .filter_map(|character| applier::body_of(&self.world, character))
            .collect();
        for body in bodies {
            pose_body(&mut self.world, body, idle, self.preview_clip.as_deref());
        }
    }

    /// The preview's Animation playing a take, if one plays (tests).
    #[cfg(test)]
    pub(super) fn motion_animation(&self) -> Option<InstanceId> {
        let body = applier::body_of(&self.world, self.character?)?;
        self.world
            .get(body)?
            .children()
            .iter()
            .copied()
            .find(|child| {
                self.world
                    .get(*child)
                    .is_some_and(|child| child.name() == PREVIEW_MOTION)
            })
    }

    /// Keeps the camera on a moving character: the orbit's target moves
    /// with the head, smoothed over about a sixth of a second. A close
    /// view follows it whole; a far one only across the floor.
    pub(super) fn follow_motion(&mut self, seconds: f64) {
        if !self.motion.moves() {
            self.follow = None;
            if std::mem::take(&mut self.reframe)
                && let Some(view) = self.view
            {
                self.focus(view);
            }
            return;
        }

        let Some(head) = self.head() else {
            return;
        };
        let Some(previous) = self.follow else {
            self.follow = Some(head);
            return;
        };
        let catch_up = 1.0 - (-(seconds as f32) * FOLLOW_RATE).exp();
        let anchor = previous + (head - previous) * catch_up;
        let mut moved = anchor - previous;
        if self.orbit.distance >= CLOSE {
            moved.y = 0.0;
        }
        self.orbit.target += moved;
        self.follow = Some(anchor);
    }

    /// The point the camera follows while a take plays (tests).
    #[cfg(test)]
    pub(super) fn following(&self) -> Option<Vec3> {
        self.follow
    }
}
