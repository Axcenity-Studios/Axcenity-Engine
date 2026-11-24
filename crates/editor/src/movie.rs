//! Render Movie: the open sequence drawn frame by frame into a video file
//! or a folder of PNGs, from the Sequencer tab or `--render-movie`
//! (`docs/editor/movie.md`).
//!
//! A movie isn't a recording of the screen. A [`MovieJob`] has its own copy
//! of the world, its own [`SequenceRun`](axcenity_anim::sequence::SequenceRun)
//! and its own renderer at the movie's size, and moves everything on by
//! exactly one frame's time (`1 / fps`) for each frame it draws, however
//! long drawing takes. So a movie is the same on every run and on every
//! machine that draws the same picture, and the editor's viewport keeps
//! drawing meanwhile.
//!
//! - `movie.rs` (here): the settings and what they work out to (the size,
//!   the frames and their times, the graphics settings).
//! - `movie/encode.rs`: finding `ffmpeg`, what it can encode, its
//!   arguments, the thread that writes frames, PNG files, the dissolve's
//!   blend.
//! - `movie/job.rs`: the job, one frame a [`MovieJob::step`].
//! - `movie/window.rs`: the Render Movie window.
//! - `movie/cli.rs`: `axcenity-editor --render-movie`.
//! - `app/movie.rs`: the editor's side: starting a job on its GPU device,
//!   stepping it a little each frame, the Output panel's line.

use std::path::{Path, PathBuf};

use axcenity_render::{
    Capabilities, GraphicsSettings, MotionBlurQuality, Quality, RayTracedAmbientOcclusion,
    RayTracedGlobalIllumination, RayTracedReflections, RayTracedShadows,
};

pub mod cli;
pub mod encode;
pub mod job;
pub mod window;

pub use encode::Format;
pub use job::{Finished, MovieJob, MovieSource, Progress};
pub use window::{MovieAction, MovieScene, MovieWindow};

/// Where movies go, inside the project folder.
pub const MOVIES_FOLDER: &str = "movies";
/// The frame rates offered.
pub const FRAME_RATES: [u32; 5] = [24, 25, 30, 50, 60];
/// Frames drawn and thrown away before the first one is kept, unless set.
pub const DEFAULT_WARM_UP: u32 = 48;
/// The widest and tallest a movie can be, whatever the GPU allows.
pub const MAX_SIDE: u32 = 7680;
/// The smallest side.
pub const MIN_SIDE: u32 = 16;

/// How big the picture is before its shape is chosen: the 16:9 frame the
/// other shapes are cut from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Hd720,
    Hd1080,
    Qhd1440,
    Uhd2160,
    /// The width and height in [`MovieSettings::custom`].
    Custom,
}

impl Resolution {
    pub const ALL: [Resolution; 5] = [
        Resolution::Hd720,
        Resolution::Hd1080,
        Resolution::Qhd1440,
        Resolution::Uhd2160,
        Resolution::Custom,
    ];

    /// Its 16:9 frame, or `None` for Custom.
    pub fn frame(self) -> Option<(u32, u32)> {
        match self {
            Resolution::Hd720 => Some((1280, 720)),
            Resolution::Hd1080 => Some((1920, 1080)),
            Resolution::Qhd1440 => Some((2560, 1440)),
            Resolution::Uhd2160 => Some((3840, 2160)),
            Resolution::Custom => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Resolution::Hd720 => "1280 × 720 (HD)",
            Resolution::Hd1080 => "1920 × 1080 (Full HD)",
            Resolution::Qhd1440 => "2560 × 1440 (QHD)",
            Resolution::Uhd2160 => "3840 × 2160 (4K)",
            Resolution::Custom => "Custom",
        }
    }
}

/// The picture's shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aspect {
    /// 16:9, a screen's.
    Wide,
    /// 2.39:1, the cinema's wide screen: the shape a sequence's letterbox
    /// bars leave of a 16:9 picture, as the whole picture.
    Scope,
    /// 4:3.
    Classic,
    /// 1:1.
    Square,
    /// 9:16, a phone held upright.
    Tall,
}

impl Aspect {
    pub const ALL: [Aspect; 5] = [
        Aspect::Wide,
        Aspect::Scope,
        Aspect::Classic,
        Aspect::Square,
        Aspect::Tall,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Aspect::Wide => "16:9",
            Aspect::Scope => "2.39:1 (scope)",
            Aspect::Classic => "4:3",
            Aspect::Square => "1:1",
            Aspect::Tall => "9:16 (upright)",
        }
    }

    /// The picture of this shape cut from (or stood up in) a 16:9 frame
    /// `width` by `height`: a scope picture keeps the width (3840 × 1608
    /// from 4K), the others the height.
    pub fn of(self, (width, height): (u32, u32)) -> (u32, u32) {
        let by_four = |value: f64| ((value / 4.0).round() as u32).max(1) * 4;
        match self {
            Aspect::Wide => (width, height),
            Aspect::Scope => (width, by_four(f64::from(width) / 2.39)),
            Aspect::Classic => (by_four(f64::from(height) * 4.0 / 3.0), height),
            Aspect::Square => (height, height),
            Aspect::Tall => (height, width),
        }
    }
}

/// Everything the Render Movie window sets.
#[derive(Clone, Debug, PartialEq)]
pub struct MovieSettings {
    /// The seconds of the sequence to render, or `None` for all of it.
    pub range: Option<(f64, f64)>,
    pub resolution: Resolution,
    pub aspect: Aspect,
    /// The size while `resolution` is Custom.
    pub custom: (u32, u32),
    /// Frames a second.
    pub fps: u32,
    pub quality: Quality,
    /// Ray traced shadows, ambient occlusion, global illumination and
    /// reflections, where the GPU device has ray queries.
    pub ray_tracing: bool,
    /// How much of each frame's motion streaks (0 to 1), in place of the
    /// world's MotionBlurEffect; `None` leaves it to the world.
    pub motion_blur: Option<f32>,
    /// Frames drawn and thrown away before the first is kept.
    pub warm_up: u32,
    /// Whether the sequence's letterbox bars are drawn.
    pub letterbox: bool,
    /// Whether the game's own ScreenGuis are drawn.
    pub game_ui: bool,
    pub format: Format,
    /// The file to write, or for a PNG sequence the folder.
    pub output: PathBuf,
}

impl Default for MovieSettings {
    fn default() -> Self {
        Self {
            range: None,
            resolution: Resolution::Hd1080,
            aspect: Aspect::Wide,
            custom: (1920, 1080),
            fps: 30,
            quality: Quality::Ultra,
            ray_tracing: true,
            motion_blur: None,
            warm_up: DEFAULT_WARM_UP,
            letterbox: true,
            game_ui: true,
            format: Format::Png,
            output: PathBuf::new(),
        }
    }
}

impl MovieSettings {
    /// The picture's size in pixels: even numbers (video encoders halve
    /// the color's size), no bigger than `max_side` (the GPU's largest
    /// texture) or [`MAX_SIDE`].
    pub fn size(&self, max_side: u32) -> (u32, u32) {
        let (width, height) = match self.resolution.frame() {
            Some(frame) => self.aspect.of(frame),
            None => self.custom,
        };
        let most = max_side.clamp(MIN_SIDE, MAX_SIDE);
        let even = |side: u32| (side.clamp(MIN_SIDE, most) / 2) * 2;
        (even(width), even(height))
    }

    /// The seconds rendered, inside a sequence `duration` seconds long.
    pub fn span(&self, duration: f64) -> (f64, f64) {
        let duration = duration.max(0.0);
        match self.range {
            Some((from, to)) if to > from => (from.clamp(0.0, duration), to.clamp(0.0, duration)),
            _ => (0.0, duration),
        }
    }

    /// The graphics settings the movie is drawn with on a GPU that can do
    /// `capabilities`: the preset, motion blur at its best, and the ray
    /// traced effects at their best where the device traces rays.
    pub fn graphics(&self, capabilities: &Capabilities) -> GraphicsSettings {
        let mut graphics = GraphicsSettings::preset(self.quality);
        // A strength set here is drawn whatever the preset would do.
        if self.motion_blur.is_some_and(|strength| strength > 0.0) {
            graphics.motion_blur = Some(MotionBlurQuality::High);
        }
        if self.ray_tracing && capabilities.ray_queries {
            graphics.ray_traced_shadows = Some(RayTracedShadows::High);
            graphics.ray_traced_ambient_occlusion = Some(RayTracedAmbientOcclusion::High);
            graphics.ray_traced_global_illumination = Some(RayTracedGlobalIllumination::High);
            graphics.ray_traced_reflections = Some(RayTracedReflections::High);
        }
        graphics
    }

    /// The best preset for the machine: Ultra, or Low where the adapter
    /// draws on the CPU.
    pub fn best_quality(capabilities: &Capabilities) -> Quality {
        if capabilities.software {
            Quality::Low
        } else {
            Quality::Ultra
        }
    }
}

/// How many frames `start` to `end` seconds are at `fps`: at least one.
/// The last frame is one frame's time before `end`, as a video's is.
pub fn frame_count(start: f64, end: f64, fps: u32) -> u32 {
    let seconds = (end - start).max(0.0);
    // A hair under a whole frame (2.9999999) still counts as that frame.
    ((seconds * f64::from(fps.max(1)) + 1e-6).floor() as u32).max(1)
}

/// The sequence time of frame `index` (from 0) of a movie that starts at
/// `start` seconds: worked out from the index each time, so no error
/// builds up over a long movie.
pub fn frame_time(start: f64, fps: u32, index: u32) -> f64 {
    start + f64::from(index) / f64::from(fps.max(1))
}

/// Frames drawn and thrown away after a camera cut (and when a dissolve's
/// second view starts), for a movie that warms up `warm_up` frames at its
/// start: a quarter as many, at most 16.
pub fn cut_warm_up(warm_up: u32) -> u32 {
    (warm_up / 4).min(16)
}

/// Where the movie of the sequence called `name` goes unless another place
/// is picked: `<project>/movies/<name>` with the format's extension (none
/// for a PNG sequence, which is a folder).
pub fn default_output(project: &Path, name: &str, format: Format) -> PathBuf {
    let name = axcenity_anim::files::file_safe(name);
    with_format(&project.join(MOVIES_FOLDER).join(name), format)
}

/// `path` with `format`'s extension in place of its own movie extension.
pub fn with_format(path: &Path, format: Format) -> PathBuf {
    let known = Format::ALL
        .iter()
        .filter_map(|format| format.extension())
        .any(|extension| {
            path.extension()
                .is_some_and(|own| own.eq_ignore_ascii_case(extension))
        });
    let mut path = path.to_path_buf();
    if known {
        path.set_extension("");
    }
    if let Some(extension) = format.extension() {
        // `set_extension` would cut a name such as `Intro.v2` short.
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".");
        name.push(extension);
        path.set_file_name(name);
    }
    path
}

/// Seconds as `m:ss`, for the progress line.
pub fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests;
