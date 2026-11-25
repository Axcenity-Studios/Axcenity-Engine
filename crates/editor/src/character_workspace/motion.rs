//! Pose and motion in the preview (refinement R9, brief §10 "Animation
//! preview"): **Still** (the rest pose, the A-pose), **Idle**, **Walk**,
//! **Run** and gestures from the starter animations (talking, a thumbs
//! up, a shake of the head, a dance, reaching out). Like an expression,
//! it is editor state: it never enters the description, undo or a saved
//! file, and an expression still shows on top of it.
//!
//! The preview's Animator plays a state machine of the preview's own
//! ([`PREVIEW_MACHINE`], written into the workspace's folder): a state for
//! Idle (the default character's Idle clip) and one for each take, each
//! looped at its own speed. Picking a motion asks the Animator for its
//! state with a blend of [`BLEND`] seconds, as a game's Humanoid does, so
//! the body **eases from one motion into the next** where it used to
//! snap. Still clears the state machine: the rest pose is a reference, and
//! it shows at once. **Walk is the Walk take at its own speed** (the
//! library's walk, 1.33 s a cycle), in place: the game's Walk state plays
//! the Run take faster, because the controller's default `WalkSpeed` (16
//! studs/s, 5.6 m/s) is a jog (decided at the R1 merge), so the true walk
//! is only seen here. Run is the Run take (the library's jog) at its own
//! speed. The takes are in place (their hips end where they start), so
//! the character stays on its floor.
//!
//! The clips are written into the workspace's folder
//! ([`PREVIEW_CLIPS_FOLDER`]), each linked to its take in the default
//! humanoid's animation file. While a take plays, the camera follows the
//! head ([`CharacterWorkspace::follow_motion`]), smoothed so a step's bob
//! doesn't shake the view: a close view follows it whole, a far one only
//! sideways, so the floor doesn't bounce.

use std::path::Path;
use std::sync::OnceLock;

use axcenity_core::{InstanceId, Value, World};
use axcenity_gameplay::description as applier;
#[cfg(test)]
use axcenity_render::glam::Vec3;

use super::CharacterWorkspace;

/// The gestures the preview offers, as named by the starter animations'
/// takes (Quaternius's library): talking, a thumbs up (the library's
/// `Yes`), a shake of the head, a dance and reaching out to something.
pub const GESTURES: [&str; 5] = ["IdleTalking", "Yes", "No", "Dance", "Interact"];

/// Where the preview's clips are written, in the workspace's folder.
pub const PREVIEW_CLIPS_FOLDER: &str = "assets/animations/Humanoid/Preview";

/// The preview's state machine, in the workspace's folder: Idle and a
/// state for each take the preview offers, named as [`Motion::label`].
pub const PREVIEW_MACHINE: &str = "assets/animations/Humanoid/Preview/Preview.statemachine.json";

/// How long one motion takes to ease into the next, in seconds: the
/// blend a game's Animator gives a state change it isn't told one for.
pub const BLEND: f64 = axcenity_anim::DEFAULT_BLEND;

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
            Motion::Gesture(name) => {
                let what = match name.as_str() {
                    "IdleTalking" => "Talking with the hands",
                    "Yes" => "A thumbs up",
                    "No" => "A shake of the head",
                    "Dance" => "A dance",
                    "Interact" => "Reaching out to something",
                    other => other,
                };
                format!("{what}, over and over (a preview only)")
            }
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

/// The preview's state machine as a file's text: Idle (the default
/// character's own Idle clip) and each take's preview clip, every state
/// looped at its own speed and reached only by asking for it.
pub fn preview_machine() -> String {
    use axcenity_anim::{Motion as Plays, StateDef, StateMachineFile};
    let states = Motion::all()
        .into_iter()
        .filter(|motion| *motion != Motion::Still)
        .enumerate()
        .map(|(index, motion)| {
            let clip = match motion.take() {
                Some(take) => format!("{PREVIEW_CLIPS_FOLDER}/{take}.anim.json"),
                None => format!(
                    "{}/Idle.anim.json",
                    axcenity_project::DEFAULT_HUMANOID_CLIPS_FOLDER
                ),
            };
            StateDef::new(
                motion.label(),
                Plays::Clip(clip),
                [index as f32 * 180.0, 0.0],
            )
        })
        .collect();
    StateMachineFile {
        default_state: Motion::Idle.label().to_owned(),
        states,
        ..StateMachineFile::default()
    }
    .to_json()
}

/// Writes `text` to `path` in `folder` unless the file there holds it.
fn write_file(folder: &Path, path: &str, text: &str) -> std::io::Result<()> {
    let file = folder.join(path);
    if std::fs::read_to_string(&file).ok().as_deref() == Some(text) {
        return Ok(());
    }
    file.parent().map_or(Ok(()), std::fs::create_dir_all)?;
    std::fs::write(&file, text)
}

/// Writes the preview's state machine and the clip of every take it plays
/// into `folder`, where they aren't there as they should be.
fn write_preview(folder: &Path) -> Result<(), String> {
    for motion in Motion::all() {
        let Some(take) = motion.take() else {
            continue;
        };
        let text = preview_clip(take)
            .ok_or_else(|| format!("The default character has no \"{take}\" animation."))?;
        write_file(
            folder,
            &format!("{PREVIEW_CLIPS_FOLDER}/{take}.anim.json"),
            &text,
        )
        .map_err(|err| format!("Could not write the {take} preview: {err}"))?;
    }
    write_file(folder, PREVIEW_MACHINE, &preview_machine())
        .map_err(|err| format!("Could not write the motion preview: {err}"))
}

/// The state machine `body`'s Animator plays, by its path.
fn machine_of(world: &World, body: InstanceId) -> Option<(InstanceId, String)> {
    let animator = world.find_first_child(body, "Animator")?;
    let path = world
        .get(animator)?
        .property("StateMachine")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    Some((animator, path))
}

/// The state machine `motion` plays on: the preview's, or none for Still.
fn machine_for(motion: &Motion) -> &'static str {
    if *motion == Motion::Still {
        ""
    } else {
        PREVIEW_MACHINE
    }
}

/// Poses `body` for `motion`: Still clears its Animator's state machine;
/// anything else plays the preview's, eased into that motion's state over
/// `blend` seconds.
fn pose_body(world: &mut World, body: InstanceId, motion: &Motion, blend: f64) {
    let Some((animator, playing)) = machine_of(world, body) else {
        return;
    };
    let machine = machine_for(motion);
    if playing != machine {
        let _ = world.set_property(animator, "StateMachine", Value::String(machine.to_owned()));
    }
    if let Some(live) = world.animator_mut(animator) {
        live.play = (!machine.is_empty()).then(|| axcenity_core::PlayRequest {
            state: motion.label().to_owned(),
            blend: Some(blend),
        });
    }
}

impl CharacterWorkspace {
    /// Plays `motion` on the preview (and on the held version beside it):
    /// editor state, never in the description or undo.
    pub fn preview_motion(&mut self, motion: Motion) {
        if motion != Motion::Still
            && !self.preview_written
            && let Err(error) = write_preview(&self.folder)
        {
            self.notice = Some(error);
            return;
        }
        self.preview_written |= motion != Motion::Still;
        let was_moving = self.motion.moves();
        self.motion = motion;
        self.pose_previews(BLEND);
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

    /// The bodies the preview shows: the character's and the one beside it.
    fn preview_bodies(&self) -> Vec<InstanceId> {
        [self.character, self.other]
            .into_iter()
            .flatten()
            .filter_map(|character| applier::body_of(&self.world, character))
            .collect()
    }

    /// Poses the main preview and the one beside it for the motion shown,
    /// both from its start (so the two move together), eased in over
    /// `blend` seconds.
    pub(super) fn pose_previews(&mut self, blend: f64) {
        for body in self.preview_bodies() {
            pose_body(&mut self.world, body, &self.motion, blend);
        }
    }

    /// Puts a body that isn't playing the motion shown onto it, at once: a
    /// character built since the motion was picked (a new world, a new
    /// one beside it) starts on the default character's own state machine.
    /// Idle on that one is left alone: it is the same Idle, and nothing
    /// is written into the folder before a motion is first picked.
    fn keep_previews_posed(&mut self) {
        let machine = machine_for(&self.motion);
        let own = axcenity_project::DEFAULT_HUMANOID_STATE_MACHINE_PATH;
        let stray: Vec<InstanceId> = self
            .preview_bodies()
            .into_iter()
            .filter(|body| {
                machine_of(&self.world, *body).is_some_and(|(_, playing)| {
                    playing != machine && !(self.motion == Motion::Idle && playing == own)
                })
            })
            .collect();
        if stray.is_empty() {
            return;
        }
        // The motion may have come with the workspace (kept from the one
        // before it) and not through `preview_motion`: its files are
        // wanted all the same.
        if !machine.is_empty() && !self.preview_written {
            if let Err(error) = write_preview(&self.folder) {
                self.notice = Some(error);
                return;
            }
            self.preview_written = true;
        }
        for body in stray {
            pose_body(&mut self.world, body, &self.motion, 0.0);
        }
    }

    /// What the preview's Animator plays: its state machine, the state, how
    /// far through it and the blend that is bringing it there, if a state
    /// machine plays (tests).
    #[cfg(test)]
    pub(super) fn motion_state(&self) -> Option<axcenity_anim::MachineView> {
        let body = applier::body_of(&self.world, self.character?)?;
        self.animators.inspect(&self.world, body)?.machine
    }

    /// Keeps the camera on a moving character: the orbit's target moves
    /// with the head, smoothed over about a sixth of a second. A close
    /// view follows it whole; a far one only across the floor.
    pub(super) fn follow_motion(&mut self, seconds: f64) {
        self.keep_previews_posed();
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
