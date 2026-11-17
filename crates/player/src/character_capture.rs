//! The in-engine half of the character visual QA harness (Character
//! Studio overhaul CO2, `docs/development/character-studio/visual-qa.md`):
//! draws a character from a **description file** with the player's own
//! renderer, from fixed cameras under fixed lights and settings, so the
//! same character can be looked at before and after a change. The same
//! input gives the same pictures.
//!
//! A capture makes a project of its own in a scratch folder (the default
//! character and a character library), builds a world with the default
//! character at the origin, applies the description to it
//! (`gameplay`'s applier, as a game does), poses it and draws every
//! [`VIEWS`] camera under every [`LIGHTS`] preset. **Don't change the
//! cameras, the lights, the stage or [`Settings::default`]**: sets
//! captured before and after would stop being comparable. Add a view or a
//! preset instead.
//!
//! `capture_characters` is the tool:
//!
//! ```text
//! AXCENITY_CAPTURE_DESCRIPTIONS=assets/characters/benchmarks AXCENITY_CAPTURE_DIR=<folder> \
//!   cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
//! ```
//!
//! `AXCENITY_CAPTURE_DESCRIPTIONS` is a `*.character.json` file or a
//! folder of them (several, separated by `;`). For each character it
//! writes `<name>/<light>-<view>.png` at 1280 × 720 and a contact sheet
//! `<name>.png` (every view under every light, at half size), and
//! `captures.json` listing what was drawn and the level of detail each
//! part was drawn at in each view (in the full-body views small parts are
//! drawn from simpler copies, as in a game; the face view is full
//! detail). Optional:
//! `AXCENITY_CAPTURE_LIBRARY` (the character library to use; the
//! repository's `assets/characters` otherwise), `AXCENITY_CAPTURE_SIZE`
//! (`1920x1080`), `AXCENITY_CAPTURE_POSE` (`rest`, the A-pose, `idle`, or
//! one of the default character's clips at a time, such as `Cheer@0.6`),
//! `AXCENITY_CAPTURE_FACE` (what the face shows: face controls and
//! expressions with their strengths, such as `Blink_Left:1,Jaw_Open:0.5`
//! or `Happy:1`; blinking, gaze and idle motion are off with it),
//! `AXCENITY_CAPTURE_ONLY` (names containing this text) and
//! `AXCENITY_CAPTURE_SHEETS=only` (no single pictures).
//!
//! Added beside the fixed set for the refinement's benchmarks (plan 11.6,
//! brief §17.2), never changing what a run without them draws:
//!
//! - `AXCENITY_CAPTURE_LIGHTS` and `AXCENITY_CAPTURE_VIEWS`: which presets
//!   and cameras, by name, separated by `,` (from [`LIGHTS`] and
//!   [`MORE_LIGHTS`]: `studio`, `hard`, `warm`, `cool`, `outdoor`; from
//!   [`VIEWS`] and [`MORE_VIEWS`]: the six above and `face-three-quarter`,
//!   `face-side`, `skin-close`). The sheet has a row per light and a
//!   column per view, as before.
//! - `AXCENITY_CAPTURE_POSE` also takes the held poses of the brief's body
//!   tests ([`HeldPose`]): `t-pose`, `arms-raised`, `elbows-bent`,
//!   `knees-bent`, both sides, at the Blender harness's angles.
//! - `AXCENITY_CAPTURE_SUITE=face` or `body` ([`Suite`]): a fixed list of
//!   faces or poses, each drawn from the suite's views, written as one
//!   sheet per character, `<name>-<suite>.png` (a row per state and light,
//!   a column per view), and the pictures as
//!   `<name>/<suite>/<state>/<light>-<view>.png`. The face suite draws
//!   neutral, smile (`Happy`), frown (`Sad`), eyes closed (both `Blink`
//!   controls) and mouth open (`Jaw_Open`); the body suite rest, the four
//!   held poses and a walk frame (`Walk@0.3`). Lights default to `studio`;
//!   `AXCENITY_CAPTURE_LIGHTS` and `_VIEWS` change them.

use std::path::{Path, PathBuf};

use axcenity_assets::{BoneTransform, Rig};
use axcenity_avatar::{Description, Library};
use axcenity_core::{Color3, PartType, Value, Vector3};
use axcenity_render::glam::{Mat4, Quat, Vec3};
use axcenity_render::{AntiAliasing, Camera, GraphicsSettings, Quality, TextureRenderer, wgpu};
use axcenity_scene::TerrainCache;

use super::*;

/// A camera of the set: its name, where it is and what it looks at
/// (studs; the character stands at the origin, 5 studs tall, facing -Z)
/// and its vertical field of view in degrees.
pub(crate) struct View {
    pub name: &'static str,
    eye: [f32; 3],
    target: [f32; 3],
    fov: f32,
}

/// The cameras, in the order the sheet shows them.
pub(crate) const VIEWS: [View; 6] = [
    View {
        name: "front",
        eye: [0.0, 2.6, -9.5],
        target: [0.0, 2.6, 0.0],
        fov: 35.0,
    },
    View {
        name: "three-quarter",
        eye: [-6.7, 2.9, -6.7],
        target: [0.0, 2.6, 0.0],
        fov: 35.0,
    },
    View {
        name: "side",
        eye: [-9.5, 2.6, 0.0],
        target: [0.0, 2.6, 0.0],
        fov: 35.0,
    },
    View {
        name: "back",
        eye: [0.0, 2.6, 9.5],
        target: [0.0, 2.6, 0.0],
        fov: 35.0,
    },
    View {
        name: "face",
        eye: [-0.35, 4.62, -2.3],
        target: [0.0, 4.58, 0.0],
        fov: 30.0,
    },
    // Straight on, where `face` stands a little to one side: a mouth or a
    // brow that is higher on one side shows here and nowhere else (CO6).
    View {
        name: "face-front",
        eye: [0.0, 4.6, -2.3],
        target: [0.0, 4.58, 0.0],
        fov: 30.0,
    },
];

/// Cameras beside [`VIEWS`] (refinement R1, brief §17.2's face and skin
/// tests), drawn only when named (`AXCENITY_CAPTURE_VIEWS`) or by a suite,
/// so the default run and its sheet stay as they were.
pub(crate) const MORE_VIEWS: [View; 3] = [
    // The face at 45 degrees and in profile, at `face`'s distance.
    View {
        name: "face-three-quarter",
        eye: [-1.63, 4.62, -1.63],
        target: [0.0, 4.58, 0.0],
        fov: 30.0,
    },
    View {
        name: "face-side",
        eye: [-2.3, 4.6, 0.0],
        target: [0.0, 4.58, 0.0],
        fov: 30.0,
    },
    // A close crop of the left cheek and the nose at about 3/4, for the
    // skin's grain, gloss and shadow edges. Aimed between the two realistic
    // frames' faces (the masculine eyes sit about 0.2 studs above the
    // feminine ones), so both show the nose from bridge to tip and the
    // cheek below the eye.
    View {
        name: "skin-close",
        eye: [-0.62, 4.57, -0.84],
        target: [-0.05, 4.54, -0.31],
        fov: 30.0,
    },
];

/// A lighting preset: the time of day (which places the sun), the ambient
/// light, and the backdrop's and floor's colours.
pub(crate) struct Light {
    pub name: &'static str,
    clock_time: f64,
    ambient: f32,
    /// The backdrop's top, horizon and ground colours (sRGB).
    backdrop: [[f32; 3]; 3],
    floor: [f32; 3],
}

/// The presets: Character Studio's Studio stage (its default lighting on
/// its grey backdrop), and a hard key light with little fill on a dark
/// backdrop, which shows shading, seams and clipping the soft one hides.
pub(crate) const LIGHTS: [Light; 2] = [
    Light {
        name: "studio",
        clock_time: 14.5,
        ambient: 0.5,
        backdrop: [[0.42, 0.44, 0.47], [0.55, 0.56, 0.58], [0.30, 0.30, 0.31]],
        floor: [0.46, 0.47, 0.49],
    },
    Light {
        name: "hard",
        clock_time: 16.7,
        ambient: 0.1,
        backdrop: [[0.08, 0.09, 0.10], [0.15, 0.16, 0.17], [0.06, 0.06, 0.07]],
        floor: [0.16, 0.16, 0.17],
    },
];

/// What a preset beside [`LIGHTS`] sets as well as a [`Light`]'s values:
/// the sun itself (the direction toward it, in the stage's space, its sRGB
/// colour and strength) and the ambient light's colour (times the Light's
/// `ambient`). The fixed presets have none: the time of day places their
/// sun and the ambient is grey.
pub(crate) struct Tint {
    sun_direction: [f32; 3],
    sun_color: [f32; 3],
    sun_intensity: f32,
    ambient: [f32; 3],
}

/// A preset beside [`LIGHTS`]: a [`Light`] and its [`Tint`].
pub(crate) struct MoreLight {
    pub light: Light,
    tint: Tint,
}

/// Presets beside [`LIGHTS`] (refinement R1, brief §17.2's rendering
/// tests), drawn only when named (`AXCENITY_CAPTURE_LIGHTS`): a low warm
/// sun from the front left at the end of the day; an overcast cool light,
/// soft and bluish; and midday outdoors, a high white sun under a blue sky
/// that lights the character from above. Each sun stands on the camera's
/// side, so the face is lit.
pub(crate) const MORE_LIGHTS: [MoreLight; 3] = [
    MoreLight {
        light: Light {
            name: "warm",
            clock_time: 17.5,
            ambient: 0.3,
            backdrop: [[0.30, 0.22, 0.18], [0.62, 0.42, 0.28], [0.22, 0.16, 0.12]],
            floor: [0.40, 0.33, 0.27],
        },
        tint: Tint {
            sun_direction: [-0.75, 0.3, -0.6],
            sun_color: [1.0, 0.7, 0.42],
            sun_intensity: 2.6,
            ambient: [1.0, 0.82, 0.65],
        },
    },
    MoreLight {
        light: Light {
            name: "cool",
            clock_time: 14.5,
            ambient: 0.6,
            backdrop: [[0.42, 0.48, 0.56], [0.58, 0.63, 0.70], [0.28, 0.31, 0.35]],
            floor: [0.42, 0.45, 0.50],
        },
        tint: Tint {
            sun_direction: [0.3, 0.85, -0.45],
            sun_color: [0.8, 0.88, 1.0],
            sun_intensity: 1.4,
            ambient: [0.78, 0.88, 1.05],
        },
    },
    MoreLight {
        light: Light {
            name: "outdoor",
            clock_time: 12.0,
            ambient: 0.35,
            backdrop: [[0.25, 0.5, 0.9], [0.63, 0.8, 0.95], [0.35, 0.33, 0.3]],
            floor: [0.48, 0.47, 0.44],
        },
        tint: Tint {
            sun_direction: [-0.35, 0.85, -0.4],
            sun_color: [1.0, 0.97, 0.92],
            sun_intensity: 3.0,
            ambient: [0.72, 0.83, 1.0],
        },
    },
];

/// A light to draw under: one of [`LIGHTS`] (no tint) or [`MORE_LIGHTS`].
#[derive(Clone, Copy)]
pub(crate) struct Lit {
    pub light: &'static Light,
    tint: Option<&'static Tint>,
}

/// Every one of [`LIGHTS`], in order: what a run draws unless told.
pub(crate) fn fixed_lights() -> Vec<Lit> {
    LIGHTS
        .iter()
        .map(|light| Lit { light, tint: None })
        .collect()
}

/// The lights `names` lists (separated by `,`), from [`LIGHTS`] and
/// [`MORE_LIGHTS`], in its order.
pub(crate) fn lights_named(names: &str) -> Result<Vec<Lit>, String> {
    names_in(names)
        .map(|name| {
            let fixed = LIGHTS
                .iter()
                .find(|light| light.name == name)
                .map(|light| Lit { light, tint: None });
            let more = MORE_LIGHTS
                .iter()
                .find(|more| more.light.name == name)
                .map(|more| Lit {
                    light: &more.light,
                    tint: Some(&more.tint),
                });
            fixed.or(more).ok_or_else(|| {
                let known: Vec<&str> = LIGHTS
                    .iter()
                    .chain(MORE_LIGHTS.iter().map(|more| &more.light))
                    .map(|light| light.name)
                    .collect();
                format!("no light `{name}` (there are {})", known.join(", "))
            })
        })
        .collect()
}

/// The cameras `names` lists (separated by `,`), from [`VIEWS`] and
/// [`MORE_VIEWS`], in its order.
pub(crate) fn views_named(names: &str) -> Result<Vec<&'static View>, String> {
    names_in(names)
        .map(|name| {
            VIEWS
                .iter()
                .chain(&MORE_VIEWS)
                .find(|view| view.name == name)
                .ok_or_else(|| {
                    let known: Vec<&str> =
                        VIEWS.iter().chain(&MORE_VIEWS).map(|v| v.name).collect();
                    format!("no view `{name}` (there are {})", known.join(", "))
                })
        })
        .collect()
}

/// The names in a `,`-separated list, trimmed, empty ones left out.
fn names_in(list: &str) -> impl Iterator<Item = &str> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

/// A pose held still for the brief's body tests (§17.2), both sides at
/// once, at the angles of the Blender harness's `qa_scene.POSES`: each
/// turns one joint per side from the rest pose (the A-pose).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldPose {
    /// The upper arms out level (90 degrees from hanging).
    TPose,
    /// The upper arms raised to 150 degrees from hanging.
    ArmsRaised,
    /// The elbows bent 135 degrees, the forearms coming forward.
    ElbowsBent,
    /// The knees bent 135 degrees, the shins going back.
    KneesBent,
}

impl HeldPose {
    pub const ALL: [HeldPose; 4] = [
        HeldPose::TPose,
        HeldPose::ArmsRaised,
        HeldPose::ElbowsBent,
        HeldPose::KneesBent,
    ];

    pub fn name(self) -> &'static str {
        match self {
            HeldPose::TPose => "t-pose",
            HeldPose::ArmsRaised => "arms-raised",
            HeldPose::ElbowsBent => "elbows-bent",
            HeldPose::KneesBent => "knees-bent",
        }
    }
}

/// `rig`'s bones that `pose` turns, each with its new transform relative
/// to its parent (what a clip's bone key holds).
pub(crate) fn held_pose(rig: &Rig, pose: HeldPose) -> Vec<(usize, BoneTransform)> {
    // A model file's axes (glTF): Y up, the character facing +Z.
    const UP: Vec3 = Vec3::Y;
    const FORWARD: Vec3 = Vec3::Z;
    let world = rig.world(&rig.rest_pose());
    let mut turned = Vec::new();
    for side in ["Left", "Right"] {
        let bone = |name: &str| rig.bone(&format!("{side}{name}"));
        let head = |bone: usize| world[bone].w_axis.truncate();
        // (the bone turned, the bone at its tail, the axis, degrees, which
        // way the tail should go)
        let joint = match pose {
            HeldPose::TPose | HeldPose::ArmsRaised => {
                let (Some(upper), Some(lower)) = (bone("UpperArm"), bone("LowerArm")) else {
                    continue;
                };
                // Abduction: the arm's angle from hanging straight down, in
                // the body's front plane (about the forward axis).
                let wanted: f32 = if pose == HeldPose::TPose { 90.0 } else { 150.0 };
                let arm = (head(lower) - head(upper)).normalize();
                let turn = wanted - arm.angle_between(-UP).to_degrees();
                let toward = if turn >= 0.0 { UP } else { -UP };
                (upper, lower, FORWARD, turn.abs(), toward)
            }
            HeldPose::ElbowsBent | HeldPose::KneesBent => {
                let (upper, lower, end, toward) = if pose == HeldPose::ElbowsBent {
                    ("UpperArm", "LowerArm", "Hand", FORWARD)
                } else {
                    ("UpperLeg", "LowerLeg", "Foot", -FORWARD)
                };
                let (Some(upper), Some(lower), Some(end)) = (bone(upper), bone(lower), bone(end))
                else {
                    continue;
                };
                // Bent about the axis across the limb, as the Blender
                // harness's `_elbow` and `_knee` bend it.
                let limb = (head(lower) - head(upper)).normalize();
                (lower, end, limb.cross(FORWARD), 135.0, toward)
            }
        };
        let (bone, tail, axis, degrees, toward) = joint;
        turned.push((
            bone,
            bend(rig, &world, bone, head(tail), axis, degrees, toward),
        ));
    }
    turned
}

/// `bone`'s transform relative to its parent once it is turned by
/// `degrees` about `axis` (the file's space) through its head, the way
/// that moves `tail` (the head of the bone below it) further `toward`.
/// `world` is every bone at rest in the file's space.
fn bend(
    rig: &Rig,
    world: &[Mat4],
    bone: usize,
    tail: Vec3,
    axis: Vec3,
    degrees: f32,
    toward: Vec3,
) -> BoneTransform {
    let head = world[bone].w_axis.truncate();
    let turn = |degrees: f32| Quat::from_axis_angle(axis.normalize(), degrees.to_radians());
    let moves = |turn: Quat| (turn * (tail - head) - (tail - head)).dot(toward);
    let turn = if moves(turn(degrees)) >= moves(turn(-degrees)) {
        turn(degrees)
    } else {
        turn(-degrees)
    };
    // Turned in place: the bone's rest in the file's space, turned about
    // its head; then taken back into its parent's space.
    let turned = Mat4::from_translation(head)
        * Mat4::from_quat(turn)
        * Mat4::from_translation(-head)
        * world[bone];
    let parent = rig.bones[bone]
        .parent
        .map_or(Mat4::IDENTITY, |parent| world[parent]);
    let (scale, rotation, translation) =
        (parent.inverse() * turned).to_scale_rotation_translation();
    BoneTransform {
        translation,
        rotation,
        scale,
    }
}

/// How the character stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stance {
    /// The rest pose (the A-pose), with nothing animating it.
    Rest,
    /// One second into the default state machine's Idle.
    Idle,
    /// `frames` sixtieths of a second into one of the default character's
    /// clips (`Cheer`, `Wave`, ...), played alone: joints away from rest
    /// (a raised arm, a bent knee) as a game's animation leaves them.
    Clip { name: &'static str, frames: u32 },
    /// A pose held still ([`HeldPose`]), played as a clip of one key per
    /// turned bone, the way `Clip` plays a starter clip.
    Held(HeldPose),
}

impl Stance {
    fn label(self) -> String {
        match self {
            Stance::Rest => "rest".to_owned(),
            Stance::Idle => "idle".to_owned(),
            Stance::Clip { name, frames } => format!("{name}@{:.2}", f64::from(frames) / 60.0),
            Stance::Held(pose) => pose.name().to_owned(),
        }
    }
}

/// `AXCENITY_CAPTURE_POSE`'s value as a stance: `rest`, `idle`, a held
/// pose's name, or a starter clip at a time (`Walk@0.5`; 0.5 s when left
/// out).
pub(crate) fn stance_named(text: &str) -> Result<Stance, String> {
    let text = text.trim();
    if let Some(pose) = HeldPose::ALL.into_iter().find(|pose| pose.name() == text) {
        return Ok(Stance::Held(pose));
    }
    match text {
        "rest" => Ok(Stance::Rest),
        "idle" => Ok(Stance::Idle),
        _ => {
            let (name, seconds) = text.split_once('@').unwrap_or((text, "0.5"));
            let seconds: f64 = seconds.parse().map_err(|_| {
                format!("`{text}`: a pose is `rest`, `idle`, a held pose or `Clip@seconds`")
            })?;
            Ok(Stance::Clip {
                // Kept for the run: a capture is one process.
                name: Box::leak(name.to_owned().into_boxed_str()),
                frames: (seconds * 60.0).round() as u32,
            })
        }
    }
}

/// One row of a suite: a name, how the character stands and what its face
/// shows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SuiteState {
    pub name: &'static str,
    pub stance: Stance,
    pub face: &'static [(&'static str, f64)],
}

/// A fixed set of states drawn for each character as one sheet (brief
/// §17.2's facial and body tests).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Suite {
    Face,
    Body,
}

impl Suite {
    pub fn named(name: &str) -> Option<Suite> {
        match name {
            "face" => Some(Suite::Face),
            "body" => Some(Suite::Body),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Suite::Face => "face",
            Suite::Body => "body",
        }
    }

    /// Its rows, in order.
    pub fn states(self) -> Vec<SuiteState> {
        let state = |name, stance, face| SuiteState { name, stance, face };
        match self {
            // Brief §17.2's facial tests: neutral, smile, frown, eyes
            // closed, mouth open, each whole.
            Suite::Face => vec![
                state("neutral", Stance::Rest, &[]),
                state("smile", Stance::Rest, &[("Happy", 1.0)]),
                state("frown", Stance::Rest, &[("Sad", 1.0)]),
                state(
                    "eyes-closed",
                    Stance::Rest,
                    &[("Blink_Left", 1.0), ("Blink_Right", 1.0)],
                ),
                state("mouth-open", Stance::Rest, &[("Jaw_Open", 1.0)]),
            ],
            // Its body tests: the A-pose (rest), the T-pose, arms raised,
            // bent elbows and knees, and a frame of the walk with the
            // legs apart.
            Suite::Body => std::iter::once(state("rest", Stance::Rest, &[]))
                .chain(
                    HeldPose::ALL
                        .into_iter()
                        .map(|pose| state(pose.name(), Stance::Held(pose), &[])),
                )
                .chain([state(
                    "walk",
                    Stance::Clip {
                        name: "Walk",
                        frames: 18,
                    },
                    &[],
                )])
                .collect(),
        }
    }

    /// Its columns, unless `AXCENITY_CAPTURE_VIEWS` names others.
    pub fn views(self) -> Vec<&'static View> {
        let names = match self {
            Suite::Face => "face-front,face,face-three-quarter,face-side",
            Suite::Body => "front,three-quarter,side,back",
        };
        views_named(names).expect("the suites name views that exist")
    }
}

/// What a capture is drawn with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Settings {
    pub size: (u32, u32),
    pub stance: Stance,
    /// Frames drawn before the picture is read, the same every time, so
    /// temporal anti-aliasing has settled the same way.
    pub frames: u32,
    /// What the face shows: face controls and expressions by name, each
    /// with its strength. Empty leaves the face alone.
    pub face: &'static [(&'static str, f64)],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            size: (1280, 720),
            stance: Stance::Rest,
            frames: 48,
            face: &[],
        }
    }
}

/// One picture: RGBA, `size.0 × size.1`.
pub(crate) struct Shot {
    pub light: &'static str,
    pub view: &'static str,
    pub pixels: Vec<u8>,
    /// The level of detail each MeshPart of the character was drawn at
    /// (0 is its full mesh), by the part's name.
    pub levels: Vec<(String, usize)>,
}

/// A scratch project for captures: the default character's files and a
/// copy of the character library at `library` (its catalogue, manifests
/// and every file they name). Removed when dropped.
pub(crate) struct Stage {
    folder: PathBuf,
    pub assets: Assets,
}

impl Stage {
    pub fn new(name: &str, library: &Path) -> Result<Self, String> {
        let folder = std::env::temp_dir().join(format!(
            "axcenity-character-capture-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).map_err(|err| err.to_string())?;
        axcenity_project::add_default_humanoid(&folder).map_err(|err| err.to_string())?;
        copy_library(library, &folder.join(axcenity_avatar::LIBRARY_FOLDER))?;
        Ok(Self {
            assets: Assets::new(&folder),
            folder,
        })
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// Copies the library at `from` to `to`: its catalogue, each manifest and
/// the files the manifests name (models, groom files), not its sources.
fn copy_library(from: &Path, to: &Path) -> Result<(), String> {
    let (library, catalogue, problems) = Library::read(from);
    if catalogue.is_none() || !problems.is_empty() {
        return Err(format!(
            "the character library `{}` doesn't read: {problems:?}",
            from.display()
        ));
    }
    let mut files = vec!["catalogue.json".to_owned()];
    for (id, base) in &library.bases {
        files.extend(library.manifest_path(id).map(str::to_owned));
        files.extend(library.path_of(id, &base.model.file));
    }
    for (id, item) in &library.items {
        files.extend(library.manifest_path(id).map(str::to_owned));
        for model in item.models() {
            files.extend(library.path_of(id, &model.file));
        }
        if let Some(strands) = item.groom.as_ref().and_then(|g| g.strands.as_deref()) {
            files.extend(library.path_of(id, strands));
        }
    }
    files.sort();
    files.dedup();
    for file in files {
        let (source, target) = (from.join(&file), to.join(&file));
        if !source.is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        std::fs::copy(&source, &target)
            .map_err(|err| format!("could not copy {}: {err}", source.display()))?;
    }
    // A model's loose textures sit beside it: take each model folder's
    // images too.
    for base in library.bases.keys() {
        let Some(manifest) = library.manifest_path(base) else {
            continue;
        };
        let folder = manifest.rsplit_once('/').map_or("", |(folder, _)| folder);
        let Ok(entries) = std::fs::read_dir(from.join(folder)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let image = path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|e| matches!(e, "png" | "jpg" | "jpeg" | "ktx2"));
            if image && let Some(name) = path.file_name() {
                let _ = std::fs::copy(&path, to.join(folder).join(name));
            }
        }
    }
    Ok(())
}

/// The world a capture draws: the default character at the origin with
/// `description` applied, on Character Studio's round floor.
pub(crate) fn stage_world(
    assets: &Assets,
    description: &Description,
    stance: Stance,
    face: &[(&str, f64)],
) -> Result<(World, Animators), String> {
    let mut world = World::new();
    let workspace = world.workspace();
    let character = axcenity_project::default_character(&mut world, Vector3::default());
    world
        .set_parent(character, Some(workspace))
        .map_err(|err| err.to_string())?;
    let applied = axcenity_gameplay::description::apply_description(
        &mut world,
        assets,
        character,
        description,
    )?;
    for problem in &applied.problems {
        eprintln!("  left out: {problem}");
    }
    if stance != Stance::Idle
        && let Some(animator) = world.find_first_child(applied.body, "Animator")
    {
        // No state machine: the Animator poses the skeleton at rest, with
        // the character's identity, and leads its parts.
        let _ = world.set_property(animator, "StateMachine", Value::String(String::new()));
    }
    if !face.is_empty() {
        // A FaceControls of the body's own, showing only what is asked:
        // no blinks, gaze or idle motion, so the picture is the same
        // every time.
        let controls = world
            .create(ClassName::FaceControls)
            .map_err(|err| err.to_string())?;
        let _ = world.set_property(controls, "ProceduralEnabled", Value::Bool(false));
        let _ = world.set_parent(controls, Some(applied.body));
        for (order, &(name, strength)) in face.iter().enumerate() {
            if axcenity_core::face::face_control(name).is_some() {
                let _ = world.set_property(controls, name, Value::Number(strength));
            } else if let Some(state) = world.face_mut(controls) {
                // Not a control: an expression, shown whole at once.
                state.expressions.push(axcenity_core::face::ExpressionPlay {
                    name: name.to_owned(),
                    intensity: strength,
                    target_intensity: strength,
                    presence: 1.0,
                    target_presence: 1.0,
                    transition: Some(0.0),
                    weight: 1.0,
                    priority: 0,
                    asymmetry: 0.0,
                    order: order as u64,
                });
            }
        }
    }
    if let Stance::Clip { name, .. } = stance {
        // The clip alone, beside the Animator, started with the game.
        let clip = format!(
            "{}/{name}.anim.json",
            axcenity_project::DEFAULT_HUMANOID_CLIPS_FOLDER
        );
        let animation = world
            .create(ClassName::Animation)
            .map_err(|err| err.to_string())?;
        let _ = world.set_property(animation, "Clip", Value::String(clip));
        let _ = world.set_property(animation, "AutoPlay", Value::Bool(true));
        let _ = world.set_parent(animation, Some(applied.body));
    }
    if let Stance::Held(pose) = stance {
        // The pose as a clip of the body's own skeleton (one key per
        // turned bone), written into the stage and played as `Clip` plays
        // a starter clip: the Animator poses, then skins, as in a game.
        let clip = held_pose_clip(&world, assets, applied.body, pose)?;
        let animation = world
            .create(ClassName::Animation)
            .map_err(|err| err.to_string())?;
        let _ = world.set_property(animation, "Clip", Value::String(clip));
        let _ = world.set_property(animation, "AutoPlay", Value::Bool(true));
        let _ = world.set_parent(animation, Some(applied.body));
    }
    let floor = world
        .create(ClassName::Part)
        .map_err(|err| err.to_string())?;
    let _ = world.set_name(floor, "Floor");
    for (name, value) in [
        ("Shape", Value::PartType(PartType::Cylinder)),
        ("Size", Value::Vector3(Vector3::new(0.4, 14.0, 14.0))),
        ("Position", Value::Vector3(Vector3::new(0.0, -0.2, 0.0))),
        ("Orientation", Value::Vector3(Vector3::new(0.0, 0.0, 90.0))),
        ("Anchored", Value::Bool(true)),
    ] {
        let _ = world.set_property(floor, name, value);
    }
    let _ = world.set_parent(floor, Some(workspace));
    let mut animators = Animators::new();
    let steps = match stance {
        Stance::Rest => 2,
        Stance::Idle => 60,
        Stance::Clip { frames, .. } => frames.max(2),
        // Long enough for the clip's fade in to finish.
        Stance::Held(_) => 30,
    };
    for _ in 0..steps {
        animators.update(&mut world, assets, 1.0 / 60.0);
    }
    let mut errors = animators.take_errors();
    errors.extend(assets.take_errors());
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok((world, animators))
}

/// Writes `pose` on `body`'s skeleton as a clip in the stage's project
/// (one constant key per turned bone, the rest of the skeleton left to its
/// rest pose) and returns its path there. Named after its content, so
/// another skeleton's pose is another file.
fn held_pose_clip(
    world: &World,
    assets: &Assets,
    body: InstanceId,
    pose: HeldPose,
) -> Result<String, String> {
    use std::hash::{Hash, Hasher};
    let mesh_id = match world.get(body).and_then(|body| body.property("MeshId")) {
        Some(Value::String(mesh_id)) => mesh_id.clone(),
        _ => return Err("the body has no MeshId".to_owned()),
    };
    let rig = assets
        .model_blocking(&mesh_id)
        .and_then(|model| model.rig.clone())
        .ok_or_else(|| format!("`{mesh_id}` has no skeleton to pose"))?;
    let paths = axcenity_anim::bone_paths(&rig);
    let mut clip = axcenity_anim::ClipFile::new(1.0);
    clip.looping = true;
    for (index, (bone, transform)) in held_pose(&rig, pose).into_iter().enumerate() {
        let value = axcenity_anim::bone_numbers(&transform);
        clip.tracks.push(axcenity_anim::PropertyTrack {
            id: format!("bone{}", index + 1),
            target: paths[bone].clone(),
            property: axcenity_anim::PropertyName::Bone,
            keys: vec![
                axcenity_anim::PropertyKeyframe::new("k1", 0.0, value.clone()),
                axcenity_anim::PropertyKeyframe::new("k2", 1.0, value),
            ],
        });
    }
    if clip.tracks.is_empty() {
        return Err(format!("the skeleton has no bones for {}", pose.name()));
    }
    let text = clip.to_json();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    let path = format!(
        "assets/animations/capture/{}-{:016x}.anim.json",
        pose.name(),
        hasher.finish()
    );
    let folder = assets.folder().ok_or("the stage has no folder")?;
    let file = folder.join(&path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::write(&file, text).map_err(|err| err.to_string())?;
    Ok(path)
}

/// Draws `description` from every view under every light: `LIGHTS.len() ×
/// VIEWS.len()` pictures, lights outermost.
pub(crate) fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    assets: &Assets,
    description: &Description,
    settings: Settings,
) -> Result<Vec<Shot>, String> {
    let views: Vec<&View> = VIEWS.iter().collect();
    capture_in(
        device,
        queue,
        assets,
        description,
        settings,
        &fixed_lights(),
        &views,
    )
}

/// Draws `description` from each of `views` under each of `lights`
/// (lights outermost), as [`capture`] draws every view under every light.
pub(crate) fn capture_in(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    assets: &Assets,
    description: &Description,
    settings: Settings,
    lights: &[Lit],
    views: &[&View],
) -> Result<Vec<Shot>, String> {
    let (mut world, animators) = stage_world(assets, description, settings.stance, settings.face)?;
    let (width, height) = settings.size;
    let mut graphics = GraphicsSettings::preset(Quality::High);
    graphics.anti_aliasing = Some(AntiAliasing::Taa);
    let mut shots = Vec::with_capacity(lights.len() * views.len());
    for lit in lights {
        let light = lit.light;
        let lighting = world.lighting();
        let _ = world.set_property(lighting, "ClockTime", Value::Number(light.clock_time));
        let ambient = light.ambient;
        let ambient = match lit.tint {
            None => Color3::new(ambient, ambient, ambient * 1.05),
            Some(tint) => {
                let [r, g, b] = tint.ambient;
                Color3::new(ambient * r, ambient * g, ambient * b)
            }
        };
        let _ = world.set_property(lighting, "Ambient", Value::Color3(ambient));
        if let Some(floor) = world.find_first_child(world.workspace(), "Floor") {
            let [r, g, b] = light.floor;
            let _ = world.set_property(floor, "Color", Value::Color3(Color3::new(r, g, b)));
        }
        for view in views {
            // A renderer of its own per picture, so nothing a view before
            // it left behind (temporal history, exposure) reaches it.
            let mut renderer = TextureRenderer::new(device, queue, width, height);
            renderer.set_graphics(&graphics);
            let mut cache = TerrainCache::new();
            let camera = Camera {
                position: Vec3::from(view.eye),
                look_at: Vec3::from(view.target),
                fov_y_degrees: view.fov,
            };
            for _ in 0..settings.frames.max(1) {
                let mut list = axcenity_scene::build_draw_list_posed(
                    &world,
                    assets,
                    camera,
                    animators.poses(),
                    &mut cache,
                );
                let [top, horizon, ground] = light.backdrop;
                list.sky.top_color = top;
                list.sky.horizon_color = horizon;
                list.sky.ground_color = ground;
                list.sky.image = None;
                list.sky.atmosphere = None;
                list.fog = Default::default();
                list.time = 0.0;
                if let Some(tint) = lit.tint {
                    list.sun.direction = Vec3::from(tint.sun_direction).normalize();
                    list.sun.color = tint.sun_color;
                    list.sun.intensity = tint.sun_intensity;
                }
                renderer.render(&list).map_err(|err| err.to_string())?;
                device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .map_err(|err| err.to_string())?;
            }
            // Which level of detail each part was drawn at: each picks its
            // own by its size on screen, and a simpler copy of a garment
            // doesn't keep its clearance from the skin, so a reviewer must
            // know which pictures show full detail.
            let levels = world
                .all_below(world.workspace(), |class| class == ClassName::MeshPart)
                .into_iter()
                .filter_map(|part| {
                    let level = renderer.lod_level_of(axcenity_scene::item_id(part))?;
                    Some((world.get(part)?.name().to_owned(), level))
                })
                .collect();
            shots.push(Shot {
                light: light.name,
                view: view.name,
                pixels: renderer.read_pixels(),
                levels,
            });
        }
    }
    let errors = assets.take_errors();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(shots)
}

/// Draws each of `suite`'s states from each of `views` under each of
/// `lights`: one row of shots per state, the lights outermost in it.
#[allow(clippy::too_many_arguments)] // The capture's own inputs and the suite's three.
pub(crate) fn capture_suite(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    assets: &Assets,
    description: &Description,
    settings: Settings,
    suite: Suite,
    lights: &[Lit],
    views: &[&View],
) -> Result<Vec<(SuiteState, Vec<Shot>)>, String> {
    suite
        .states()
        .into_iter()
        .map(|state| {
            let settings = Settings {
                stance: state.stance,
                face: state.face,
                ..settings
            };
            let shots = capture_in(device, queue, assets, description, settings, lights, views)
                .map_err(|why| format!("{}: {why}", state.name))?;
            Ok((state, shots))
        })
        .collect()
}

/// `shots` on one picture at half size, `columns` to a row, rows in order.
/// Returns its width, height and RGBA pixels.
pub(crate) fn contact_grid(
    shots: &[Shot],
    size: (u32, u32),
    columns: usize,
) -> (u32, u32, Vec<u8>) {
    let columns = columns.max(1);
    let rows = shots.len().div_ceil(columns);
    sheet_of(shots, size, columns, rows)
}

/// Every view under every light on one picture at half size: a row per
/// light, a column per view. Returns its width, height and RGBA pixels.
pub(crate) fn contact_sheet(shots: &[Shot], size: (u32, u32)) -> (u32, u32, Vec<u8>) {
    sheet_of(shots, size, VIEWS.len(), LIGHTS.len())
}

/// The first `columns × rows` of `shots` at half size, a row at a time.
fn sheet_of(shots: &[Shot], size: (u32, u32), columns: usize, rows: usize) -> (u32, u32, Vec<u8>) {
    let (width, height) = (size.0 as usize, size.1 as usize);
    let (cell_w, cell_h) = (width / 2, height / 2);
    let (sheet_w, sheet_h) = (cell_w * columns, cell_h * rows);
    let mut sheet = vec![0u8; sheet_w * sheet_h * 4];
    for (index, shot) in shots.iter().enumerate().take(columns * rows) {
        let (column, row) = (index % columns, index / columns);
        for y in 0..cell_h {
            for x in 0..cell_w {
                // Each sheet pixel is the average of the four under it.
                let mut sum = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let at = ((y * 2 + dy) * width + x * 2 + dx) * 4;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(shot.pixels.get(at + channel).copied().unwrap_or(0));
                    }
                }
                let at = ((row * cell_h + y) * sheet_w + column * cell_w + x) * 4;
                for (channel, total) in sum.iter().enumerate() {
                    sheet[at + channel] = ((total + 2) / 4) as u8;
                }
            }
        }
    }
    (sheet_w as u32, sheet_h as u32, sheet)
}

pub(crate) fn save_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let file = std::fs::File::create(path).map_err(|err| err.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().map_err(|err| err.to_string())?;
    writer
        .write_image_data(pixels)
        .map_err(|err| err.to_string())?;
    writer.finish().map_err(|err| err.to_string())
}

/// The repository's character library.
pub(crate) fn repository_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

/// The `*.character.json` files `paths` names (files, or folders of them,
/// separated by `;`), sorted by name. A relative path is from where the
/// tool is run, or else from the repository's top folder.
fn description_files(paths: &str) -> Vec<PathBuf> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for path in paths.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let mut path = PathBuf::from(path);
        if path.is_relative() && !path.exists() {
            path = repository.join(path);
        }
        if path.is_dir() {
            files.extend(
                std::fs::read_dir(&path)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|entry| entry.path())
                    .filter(|file| {
                        file.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.ends_with(".character.json"))
                    }),
            );
        } else {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// The tool: see the module documentation.
#[test]
#[ignore = "tool: run with --release -- --ignored capture_characters"]
fn capture_characters() {
    let (Ok(descriptions), Some(out)) = (
        std::env::var("AXCENITY_CAPTURE_DESCRIPTIONS"),
        std::env::var_os("AXCENITY_CAPTURE_DIR"),
    ) else {
        eprintln!("skipped capture_characters: set AXCENITY_CAPTURE_DESCRIPTIONS and _DIR");
        return;
    };
    let mut out = PathBuf::from(out);
    if out.is_relative() {
        out = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(out);
    }
    let library =
        std::env::var_os("AXCENITY_CAPTURE_LIBRARY").map_or_else(repository_library, PathBuf::from);
    let mut settings = Settings::default();
    if let Some((width, height)) = std::env::var("AXCENITY_CAPTURE_SIZE")
        .ok()
        .and_then(|size| {
            let (width, height) = size.split_once('x')?;
            Some((width.parse().ok()?, height.parse().ok()?))
        })
    {
        settings.size = (width, height);
    }
    if let Ok(pose) = std::env::var("AXCENITY_CAPTURE_POSE") {
        settings.stance =
            stance_named(&pose).unwrap_or_else(|why| panic!("AXCENITY_CAPTURE_POSE: {why}"));
    }
    let suite = std::env::var("AXCENITY_CAPTURE_SUITE").ok().map(|name| {
        Suite::named(name.trim())
            .unwrap_or_else(|| panic!("AXCENITY_CAPTURE_SUITE is `face` or `body`, not `{name}`"))
    });
    // Without these, a run draws what it always drew: every fixed view
    // under every fixed light (a suite: its own views, under `studio`).
    let lights = match std::env::var("AXCENITY_CAPTURE_LIGHTS") {
        Ok(names) => lights_named(&names),
        Err(_) if suite.is_some() => lights_named("studio"),
        Err(_) => Ok(fixed_lights()),
    }
    .unwrap_or_else(|why| panic!("AXCENITY_CAPTURE_LIGHTS: {why}"));
    let views = match (std::env::var("AXCENITY_CAPTURE_VIEWS"), suite) {
        (Ok(names), _) => views_named(&names),
        (Err(_), Some(suite)) => Ok(suite.views()),
        (Err(_), None) => Ok(VIEWS.iter().collect()),
    }
    .unwrap_or_else(|why| panic!("AXCENITY_CAPTURE_VIEWS: {why}"));
    if let Ok(face) = std::env::var("AXCENITY_CAPTURE_FACE") {
        let shown: Vec<(&'static str, f64)> = face
            .split(',')
            .filter(|entry| !entry.trim().is_empty())
            .map(|entry| {
                let (name, strength) = entry.trim().split_once(':').unwrap_or((entry.trim(), "1"));
                let strength = strength
                    .parse()
                    .expect("AXCENITY_CAPTURE_FACE is `Name:strength,...`");
                // Kept for the run: a capture is one process.
                (&*Box::leak(name.to_owned().into_boxed_str()), strength)
            })
            .collect();
        settings.face = Box::leak(shown.into_boxed_slice());
    }
    let only = std::env::var("AXCENITY_CAPTURE_ONLY").ok();
    let sheets_only = std::env::var("AXCENITY_CAPTURE_SHEETS").as_deref() == Ok("only");
    let (device, queue) = super::bench_tests::bench_gpu().expect("no graphics adapter");
    let stage = Stage::new("tool", &library).expect("the stage");
    let mut listed = Vec::new();
    for file in description_files(&descriptions) {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".character.json"))
            .unwrap_or("character")
            .to_owned();
        if only.as_deref().is_some_and(|only| !name.contains(only)) {
            continue;
        }
        let read = std::fs::read_to_string(&file)
            .map_err(|err| err.to_string())
            .and_then(|text| Description::from_json(&text).map_err(|err| err.to_string()));
        let description = match read {
            Ok(description) => description,
            Err(why) => {
                eprintln!("{name}: not captured: {why}");
                continue;
            }
        };
        let started = std::time::Instant::now();
        // Each picture with the folder it goes in: the character's own, or
        // a suite's row in it.
        let drawn = match suite {
            None => capture_in(
                &device,
                &queue,
                &stage.assets,
                &description,
                settings,
                &lights,
                &views,
            )
            .map(|shots| {
                let folder = out.join(&name);
                shots
                    .into_iter()
                    .map(|shot| (folder.clone(), shot))
                    .collect::<Vec<_>>()
            }),
            Some(suite) => capture_suite(
                &device,
                &queue,
                &stage.assets,
                &description,
                settings,
                suite,
                &lights,
                &views,
            )
            .map(|rows| {
                rows.into_iter()
                    .flat_map(|(state, shots)| {
                        let folder = out.join(&name).join(suite.name()).join(state.name);
                        shots.into_iter().map(move |shot| (folder.clone(), shot))
                    })
                    .collect()
            }),
        };
        let (folders, shots): (Vec<PathBuf>, Vec<Shot>) = match drawn {
            Ok(drawn) => drawn.into_iter().unzip(),
            Err(why) => {
                eprintln!("{name}: not captured: {why}");
                continue;
            }
        };
        let (width, height) = settings.size;
        if !sheets_only {
            for (folder, shot) in folders.iter().zip(&shots) {
                let path = folder.join(format!("{}-{}.png", shot.light, shot.view));
                save_png(&path, width, height, &shot.pixels).unwrap();
            }
        }
        // A row per light (a suite: per state and light), a column per view.
        let (sheet_w, sheet_h, sheet) = contact_grid(&shots, settings.size, views.len());
        let sheet_name = match suite {
            Some(suite) => format!("{name}-{}.png", suite.name()),
            None => format!("{name}.png"),
        };
        save_png(&out.join(sheet_name), sheet_w, sheet_h, &sheet).unwrap();
        println!(
            "{name}: {} pictures in {:.1} s",
            shots.len(),
            started.elapsed().as_secs_f64()
        );
        // The levels of detail by view (the lights don't change them).
        let levels: serde_json::Map<String, serde_json::Value> = shots
            .iter()
            .take(views.len())
            .map(|shot| {
                let parts: serde_json::Map<String, serde_json::Value> = shot
                    .levels
                    .iter()
                    .map(|(part, level)| (part.clone(), serde_json::json!(level)))
                    .collect();
                (shot.view.to_owned(), parts.into())
            })
            .collect();
        listed.push(serde_json::json!({
            "name": name,
            "description": description.name,
            "base": description.base,
            "levels_of_detail": levels,
        }));
    }
    let mut record = serde_json::json!({
        "size": [settings.size.0, settings.size.1],
        "stance": settings.stance.label(),
        "face": settings.face.iter().map(|(name, strength)| format!("{name}:{strength}")).collect::<Vec<_>>(),
        "frames": settings.frames,
        "quality": "High",
        "anti_aliasing": "TAA",
        "lights": lights.iter().map(|lit| lit.light.name).collect::<Vec<_>>(),
        "views": views.iter().map(|view| view.name).collect::<Vec<_>>(),
        "characters": listed,
    });
    // A suite's rows, in the sheet's order (a run without one writes the
    // same record as before).
    if let Some(suite) = suite {
        let states: Vec<serde_json::Value> = suite
            .states()
            .iter()
            .map(|state| {
                serde_json::json!({
                    "name": state.name,
                    "stance": state.stance.label(),
                    "face": state.face.iter().map(|(name, strength)| format!("{name}:{strength}")).collect::<Vec<_>>(),
                })
            })
            .collect();
        record["suite"] = serde_json::json!(suite.name());
        record["states"] = serde_json::json!(states);
    }
    let mut text = serde_json::to_string_pretty(&record).unwrap();
    text.push('\n');
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("captures.json"), text).unwrap();
}

/// CO2's "done when": the same description, cameras and settings give
/// identical pictures twice, the capture draws every view under every
/// light, and what it draws is the character (a different description is
/// a different picture; the character is in frame). Skipped on machines
/// without a GPU.
#[test]
fn the_same_description_captures_identical_pictures_and_every_view_is_drawn() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("test", &repository_library()).expect("the stage");
    let settings = Settings {
        size: (320, 180),
        frames: 12,
        ..Settings::default()
    };
    let default = axcenity_gameplay::description::default_description(
        &axcenity_gameplay::description::library(&stage.assets),
        axcenity_avatar::V1_BASE,
    );
    let first = capture(&device, &queue, &stage.assets, &default, settings).unwrap();
    let again = capture(&device, &queue, &stage.assets, &default, settings).unwrap();
    assert_eq!(first.len(), LIGHTS.len() * VIEWS.len());
    for (one, two) in first.iter().zip(&again) {
        assert_eq!((one.light, one.view), (two.light, two.view));
        assert_eq!(one.pixels.len(), 320 * 180 * 4);
        assert!(
            one.pixels == two.pixels,
            "{} {}: the same input drew another picture",
            one.light,
            one.view
        );
    }
    // The character is in every picture: against the flat backdrop, a
    // share of each picture's pixels differ from its corner's (the least
    // is the side view in hard light: a narrow figure, half in shadow).
    for shot in &first {
        let corner = &shot.pixels[..4];
        let different = shot
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel.iter().zip(corner).any(|(a, b)| a.abs_diff(*b) > 24))
            .count();
        let share = different as f32 / (320.0 * 180.0);
        assert!(
            share > 0.015,
            "{} {}: only {share} of the picture isn't backdrop",
            shot.light,
            shot.view
        );
    }
    // Views and lights differ from each other, and so does another
    // character from the same view.
    assert!(
        first[0].pixels != first[1].pixels,
        "front and three-quarter"
    );
    assert!(
        first[0].pixels != first[VIEWS.len()].pixels,
        "studio and hard"
    );
    let mut bald = default.clone();
    bald.items.clear();
    bald.parameters.insert("build".to_owned(), 1.0);
    let other = capture(&device, &queue, &stage.assets, &bald, settings).unwrap();
    assert!(first[0].pixels != other[0].pixels, "another character");
    // Each picture says which level of detail the parts were drawn at.
    assert!(
        first[0].levels.iter().any(|(part, _)| part == "Body"),
        "{:?}",
        first[0].levels
    );
    // The sheet holds them all, at half size.
    let (width, height, sheet) = contact_sheet(&first, settings.size);
    assert_eq!(
        (width, height),
        (160 * VIEWS.len() as u32, 90 * LIGHTS.len() as u32)
    );
    assert_eq!(sheet.len(), (width * height * 4) as usize);
}

/// What a run draws without `AXCENITY_CAPTURE_LIGHTS` and `_VIEWS` is
/// what it always drew: every fixed view under every fixed light, in
/// order, none of the presets and cameras added beside them.
#[test]
fn the_default_run_is_every_fixed_view_under_every_fixed_light() {
    let names = |lights: &[Lit]| lights.iter().map(|lit| lit.light.name).collect::<Vec<_>>();
    assert_eq!(names(&fixed_lights()), ["studio", "hard"]);
    assert!(fixed_lights().iter().all(|lit| lit.tint.is_none()));
    let fixed = lights_named("studio,hard").unwrap();
    assert_eq!(names(&fixed), ["studio", "hard"]);
    assert!(fixed.iter().all(|lit| lit.tint.is_none()));
    let views: Vec<&str> = views_named("front,three-quarter,side,back,face,face-front")
        .unwrap()
        .iter()
        .map(|view| view.name)
        .collect();
    assert_eq!(views, VIEWS.map(|view| view.name));
    for added in MORE_VIEWS.iter().map(|view| view.name) {
        assert!(VIEWS.iter().all(|view| view.name != added), "{added}");
    }
    for added in MORE_LIGHTS.iter().map(|more| more.light.name) {
        assert!(LIGHTS.iter().all(|light| light.name != added), "{added}");
    }
}

/// Lights, views and poses are read by name, in the order given; an
/// unknown name is refused, not skipped.
#[test]
fn light_view_and_pose_names_are_read_and_unknown_ones_refused() {
    let lights: Vec<&str> = lights_named("studio, warm,cool,outdoor")
        .unwrap()
        .iter()
        .map(|lit| lit.light.name)
        .collect();
    assert_eq!(lights, ["studio", "warm", "cool", "outdoor"]);
    assert!(lights_named("warm").unwrap()[0].tint.is_some());
    assert!(lights_named("sunset").is_err());
    let views: Vec<&str> = views_named("skin-close,front,face-side,face-three-quarter")
        .unwrap()
        .iter()
        .map(|view| view.name)
        .collect();
    assert_eq!(
        views,
        ["skin-close", "front", "face-side", "face-three-quarter"]
    );
    assert!(views_named("top").is_err());
    assert_eq!(stance_named("rest"), Ok(Stance::Rest));
    assert_eq!(stance_named("idle"), Ok(Stance::Idle));
    for pose in HeldPose::ALL {
        assert_eq!(stance_named(pose.name()), Ok(Stance::Held(pose)));
    }
    assert_eq!(
        stance_named("Walk@0.25"),
        Ok(Stance::Clip {
            name: "Walk",
            frames: 15
        })
    );
    assert_eq!(
        stance_named("Cheer"),
        Ok(Stance::Clip {
            name: "Cheer",
            frames: 30
        })
    );
    assert!(stance_named("Walk@soon").is_err());
}

/// The suites hold brief §17.2's facial and body tests, in order.
#[test]
fn the_suites_hold_the_brief_s_facial_and_body_tests() {
    assert_eq!(Suite::named("face"), Some(Suite::Face));
    assert_eq!(Suite::named("body"), Some(Suite::Body));
    assert_eq!(Suite::named("wardrobe"), None);
    let face = Suite::Face.states();
    let names: Vec<&str> = face.iter().map(|state| state.name).collect();
    assert_eq!(
        names,
        ["neutral", "smile", "frown", "eyes-closed", "mouth-open"]
    );
    assert!(face.iter().all(|state| state.stance == Stance::Rest));
    assert!(face[0].face.is_empty());
    assert_eq!(face[1].face, [("Happy", 1.0)]);
    assert_eq!(face[2].face, [("Sad", 1.0)]);
    assert_eq!(face[3].face, [("Blink_Left", 1.0), ("Blink_Right", 1.0)]);
    assert_eq!(face[4].face, [("Jaw_Open", 1.0)]);
    for state in &face {
        for (control, _) in state.face {
            let expression = matches!(*control, "Happy" | "Sad");
            assert!(
                expression || axcenity_core::face::face_control(control).is_some(),
                "{control}"
            );
        }
    }
    let views = |suite: Suite| suite.views().iter().map(|v| v.name).collect::<Vec<_>>();
    assert_eq!(
        views(Suite::Face),
        ["face-front", "face", "face-three-quarter", "face-side"]
    );
    let body = Suite::Body.states();
    let names: Vec<&str> = body.iter().map(|state| state.name).collect();
    assert_eq!(
        names,
        [
            "rest",
            "t-pose",
            "arms-raised",
            "elbows-bent",
            "knees-bent",
            "walk"
        ]
    );
    assert!(body.iter().all(|state| state.face.is_empty()));
    assert_eq!(body[0].stance, Stance::Rest);
    for (state, pose) in body[1..5].iter().zip(HeldPose::ALL) {
        assert_eq!(state.stance, Stance::Held(pose));
    }
    assert!(matches!(body[5].stance, Stance::Clip { name: "Walk", .. }));
    assert_eq!(
        views(Suite::Body),
        ["front", "three-quarter", "side", "back"]
    );
}

/// The realistic base's skeleton, at rest.
fn realistic_rig(stage: &Stage) -> std::sync::Arc<Rig> {
    let library = axcenity_gameplay::description::library(&stage.assets);
    let description =
        axcenity_gameplay::description::default_description(&library, "axcenity/realistic");
    let resolved = axcenity_avatar::resolve(&library, &description).unwrap();
    let mesh_id = format!(
        "{}/{}",
        axcenity_avatar::LIBRARY_FOLDER,
        resolved.body.model
    );
    let model = stage.assets.model_blocking(&mesh_id).expect("the body");
    model.rig.clone().expect("a skeleton")
}

/// Each held pose turns its joints, on both sides, to the Blender
/// harness's angles, measured on the realistic skeleton in its file's
/// space (Y up, the character facing +Z: its eyes are ahead of its head).
#[test]
fn held_poses_turn_each_joint_to_its_angle_on_the_realistic_body() {
    let stage = Stage::new("held", &repository_library()).expect("the stage");
    let rig = realistic_rig(&stage);
    let rest = rig.world(&rig.rest_pose());
    let at = |world: &[Mat4], name: &str| world[rig.bone(name).unwrap()].w_axis.truncate();
    assert!(at(&rest, "LeftEye").z > at(&rest, "Head").z, "facing +Z");
    let degrees = |a: Vec3, b: Vec3| a.normalize().angle_between(b.normalize()).to_degrees();
    for pose in HeldPose::ALL {
        let turned = held_pose(&rig, pose);
        assert_eq!(turned.len(), 2, "{pose:?}: one joint a side");
        let mut posed = rig.rest_pose();
        for (bone, transform) in &turned {
            posed[*bone] = *transform;
        }
        let world = rig.world(&posed);
        for side in ["Left", "Right"] {
            let bone = |name: &str| format!("{side}{name}");
            let arm = at(&world, &bone("LowerArm")) - at(&world, &bone("UpperArm"));
            let abduction = degrees(arm, Vec3::NEG_Y);
            let (from, to) = match pose {
                HeldPose::TPose | HeldPose::ArmsRaised => {
                    let wanted = if pose == HeldPose::TPose { 90.0 } else { 150.0 };
                    assert!(
                        (abduction - wanted).abs() < 1.0,
                        "{pose:?} {side}: {abduction}"
                    );
                    continue;
                }
                HeldPose::ElbowsBent => ("LowerArm", "Hand"),
                HeldPose::KneesBent => ("LowerLeg", "Foot"),
            };
            let before = at(&rest, &bone(to)) - at(&rest, &bone(from));
            let after = at(&world, &bone(to)) - at(&world, &bone(from));
            let bend = degrees(before, after);
            assert!((bend - 135.0).abs() < 1.0, "{pose:?} {side}: {bend}");
            let forward = at(&world, &bone(to)).z - at(&rest, &bone(to)).z;
            match pose {
                HeldPose::ElbowsBent => assert!(forward > 0.0, "{side} forearm comes forward"),
                _ => assert!(forward < 0.0, "{side} shin goes back"),
            }
        }
    }
}

/// The added lights, the skin view and the held poses draw the same
/// pictures from the same input; each added light differs from `studio`,
/// and each held pose from the rest pose. Skipped without a GPU.
#[test]
fn added_lights_views_and_poses_repeat_and_differ() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("more", &repository_library()).expect("the stage");
    let library = axcenity_gameplay::description::library(&stage.assets);
    let realistic =
        axcenity_gameplay::description::default_description(&library, "axcenity/realistic");
    let settings = Settings {
        size: (320, 180),
        frames: 12,
        ..Settings::default()
    };
    let lights = lights_named("studio,warm,cool,outdoor").unwrap();
    let views = views_named("front,skin-close").unwrap();
    let draw = |settings: Settings, lights: &[Lit], views: &[&View]| {
        capture_in(
            &device,
            &queue,
            &stage.assets,
            &realistic,
            settings,
            lights,
            views,
        )
        .unwrap()
    };
    let first = draw(settings, &lights, &views);
    let again = draw(settings, &lights, &views);
    assert_eq!(first.len(), 8);
    for (one, two) in first.iter().zip(&again) {
        assert_eq!((one.light, one.view), (two.light, two.view));
        assert!(one.pixels == two.pixels, "{} {}", one.light, one.view);
    }
    // studio front, studio skin-close, warm front, ...
    for added in [2, 4, 6] {
        assert!(
            first[added].pixels != first[0].pixels,
            "{} looks like studio",
            first[added].light
        );
    }
    assert_ne!(first[1].view, first[0].view);
    assert!(first[1].pixels != first[0].pixels, "skin-close");
    let front = &views[..1];
    let studio = &lights[..1];
    let rest = draw(settings, studio, front);
    for pose in HeldPose::ALL {
        let held = Settings {
            stance: Stance::Held(pose),
            ..settings
        };
        let one = draw(held, studio, front);
        let two = draw(held, studio, front);
        assert!(one[0].pixels == two[0].pixels, "{pose:?} again");
        assert!(one[0].pixels != rest[0].pixels, "{pose:?} looks like rest");
    }
}

/// A suite draws each of its states from each of its views and lays them
/// out as one sheet, a row a state; each face differs from the neutral
/// one. Skipped without a GPU.
#[test]
fn the_face_suite_draws_each_face_on_one_sheet() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("suite", &repository_library()).expect("the stage");
    let library = axcenity_gameplay::description::library(&stage.assets);
    let realistic =
        axcenity_gameplay::description::default_description(&library, "axcenity/realistic");
    let settings = Settings {
        size: (160, 90),
        frames: 6,
        ..Settings::default()
    };
    let views = Suite::Face.views();
    let rows = capture_suite(
        &device,
        &queue,
        &stage.assets,
        &realistic,
        settings,
        Suite::Face,
        &fixed_lights()[..1],
        &views,
    )
    .unwrap();
    assert_eq!(rows.len(), 5);
    let neutral = &rows[0].1[0].pixels;
    for (state, shots) in &rows[1..] {
        assert_eq!(shots.len(), views.len());
        assert!(shots[0].pixels != *neutral, "{} looks neutral", state.name);
    }
    let shots: Vec<Shot> = rows.into_iter().flat_map(|(_, shots)| shots).collect();
    let (width, height, sheet) = contact_grid(&shots, settings.size, views.len());
    assert_eq!((width, height), (80 * 4, 45 * 5));
    assert_eq!(sheet.len(), (width * height * 4) as usize);
}
