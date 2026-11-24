//! `axcenity-editor --render-movie`: a movie rendered without the window,
//! for build scripts.
//!
//! ```text
//! axcenity-editor --render-movie <project folder> --sequence <path or name>
//!     --out <file or folder> [--size WxH] [--fps N] [--quality Low|Medium|High|Ultra]
//!     [--format h264|av1|ffv1|png] [--range FROM:TO] [--warm-up N]
//!     [--motion-blur 0..1] [--no-ray-tracing] [--no-letterbox] [--no-ui]
//! ```
//!
//! It loads the project as the editor does, opens a GPU device of its own
//! (with ray queries where the card has them, unless `--no-ray-tracing`),
//! runs a [`MovieJob`] to its end and prints how far it is. The frames are
//! the ones the Render Movie window would make with the same settings.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use axcenity_assets::Assets;
use axcenity_render::glam::Vec3;
use axcenity_render::{Camera, Quality, wgpu};

use super::job::{MovieJob, MovieSource, Progress};
use super::{DEFAULT_WARM_UP, Format, MovieSettings, Resolution, clock, encode};
use crate::app::mcp::block_on;

/// The flag that starts it, first on the command line.
pub const FLAG: &str = "--render-movie";

pub const USAGE: &str = "usage: axcenity-editor --render-movie <project folder> --sequence <path \
     or name> --out <file or folder> [--size WxH] [--fps N] [--quality Low|Medium|High|Ultra] \
     [--format h264|av1|ffv1|png] [--range FROM:TO] [--warm-up N] [--motion-blur 0..1] \
     [--no-ray-tracing] [--no-letterbox] [--no-ui]";

/// What the command line asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub project: PathBuf,
    /// A path from the project folder, or a sequence's name.
    pub sequence: String,
    pub settings: MovieSettings,
}

/// Reads the arguments after the flag. An error says what's wrong.
pub fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Options, String> {
    let project = PathBuf::from(args.next().ok_or("the project folder is missing")?);
    let mut sequence = None;
    let mut format = None;
    let mut settings = MovieSettings {
        warm_up: DEFAULT_WARM_UP,
        ..MovieSettings::default()
    };
    while let Some(flag) = args.next() {
        let flag = flag.to_string_lossy().into_owned();
        // The flags that stand alone.
        match flag.as_str() {
            "--no-ray-tracing" => {
                settings.ray_tracing = false;
                continue;
            }
            "--no-letterbox" => {
                settings.letterbox = false;
                continue;
            }
            "--no-ui" => {
                settings.game_ui = false;
                continue;
            }
            _ => {}
        }
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        let text = value.to_string_lossy().into_owned();
        let wrong = || format!("{flag} can't be `{text}`");
        match flag.as_str() {
            "--sequence" => sequence = Some(text),
            "--out" => settings.output = PathBuf::from(value),
            "--size" => {
                let (width, height) = text
                    .split_once(['x', 'X', '×'])
                    .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))
                    .ok_or_else(wrong)?;
                settings.resolution = Resolution::Custom;
                settings.custom = (width, height);
            }
            "--fps" => {
                settings.fps = text
                    .parse()
                    .ok()
                    .filter(|fps| (1..=240).contains(fps))
                    .ok_or_else(wrong)?;
            }
            "--quality" => {
                settings.quality = Quality::ALL
                    .into_iter()
                    .find(|quality| quality.label().eq_ignore_ascii_case(&text))
                    .ok_or_else(wrong)?;
            }
            "--format" => format = Some(Format::from_flag(&text).ok_or_else(wrong)?),
            "--range" => {
                let (from, to) = text
                    .split_once(':')
                    .and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?)))
                    .filter(|(from, to)| from.is_finite() && to > from && *from >= 0.0)
                    .ok_or_else(wrong)?;
                settings.range = Some((from, to));
            }
            "--warm-up" => settings.warm_up = text.parse().map_err(|_| wrong())?,
            "--motion-blur" => {
                let strength: f32 = text.parse().map_err(|_| wrong())?;
                if !(0.0..=1.0).contains(&strength) {
                    return Err(wrong());
                }
                settings.motion_blur = Some(strength);
            }
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if settings.output.as_os_str().is_empty() {
        return Err("--out is missing".to_owned());
    }
    // Without --format, the file's name says: .mp4, .mkv, or a folder.
    settings.format = format.unwrap_or_else(|| Format::of_path(&settings.output));
    Ok(Options {
        project,
        sequence: sequence.ok_or("--sequence is missing")?,
        settings,
    })
}

/// Runs `--render-movie` with the arguments after the flag. Prints
/// progress and the result; false if the movie wasn't made.
pub fn run_from_command_line(args: impl Iterator<Item = OsString>) -> bool {
    let options = match parse(args) {
        Ok(options) => options,
        Err(why) => {
            eprintln!("axcenity-editor {FLAG}: {why}\n{USAGE}");
            return false;
        }
    };
    match render(&options) {
        Ok(()) => true,
        Err(why) => {
            eprintln!("axcenity-editor {FLAG}: {why}");
            false
        }
    }
}

/// A GPU device for drawing with no window, with ray queries when asked
/// for and the card has them.
fn device(ray_tracing: bool) -> Result<(wgpu::Device, wgpu::Queue, String), String> {
    axcenity_render::adapters::want_ray_tracing(ray_tracing);
    let instance = axcenity_render::adapters::instance();
    let adapter = block_on(axcenity_render::adapters::adapter(&instance, None))
        .map_err(|err| format!("no graphics adapter: {err}"))?;
    // As the editor's own device: timestamps and BC7 textures where the
    // card has them.
    let wanted = wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::TIMESTAMP_QUERY;
    let mut descriptor = wgpu::DeviceDescriptor {
        required_features: adapter.features() & wanted,
        required_limits: adapter.limits(),
        ..Default::default()
    };
    axcenity_render::adapters::with_ray_queries(&adapter, &mut descriptor);
    let (device, queue) = block_on(adapter.request_device(&descriptor))
        .map_err(|err| format!("could not open the graphics device: {err}"))?;
    let info = adapter.get_info();
    Ok((device, queue, format!("{} ({:?})", info.name, info.backend)))
}

fn render(options: &Options) -> Result<(), String> {
    let project = axcenity_project::load(&options.project)
        .map_err(|err| format!("could not open {}: {err}", options.project.display()))?;
    for problem in &project.problems {
        eprintln!("load problem: {problem}");
    }
    let assets = Assets::new(&options.project);
    // A name (`Intro`) or a path from the project folder.
    let named = crate::sequencer::sequence_path(&options.sequence);
    let path = if options.project.join(&options.sequence).is_file() {
        options.sequence.replace('\\', "/")
    } else {
        named
    };
    let sequence = axcenity_anim::sequence::load_sequence(&assets, &path)
        .map_err(|why| format!("could not read the sequence `{path}`: {why}"))?;
    let stylesheets = project
        .stylesheets
        .iter()
        .map(|file| axcenity_ui::parse_stylesheet(&file.path, &file.source).0)
        .collect();

    let ffmpeg = if options.settings.format == Format::Png {
        None
    } else {
        Some(encode::probe().ok_or_else(|| {
            format!(
                "a video needs ffmpeg, which wasn't found. {} Or pass --format png.",
                encode::install_hint()
            )
        })?)
    };
    let (device, queue, adapter) = device(options.settings.ray_tracing)?;
    let source = MovieSource {
        world: project.world,
        sequence,
        assets,
        stylesheets,
        // Until the sequence's first camera cut: a view of the middle of
        // the world from a little way off.
        camera: Camera {
            position: Vec3::new(0.0, 12.0, 36.0),
            look_at: Vec3::new(0.0, 4.0, 0.0),
            fov_y_degrees: 50.0,
        },
    };
    let mut job = MovieJob::start(&device, &queue, source, &options.settings, ffmpeg.as_ref())?;
    let (width, height) = job.size();
    let (_, total) = job.frames();
    println!(
        "Rendering {path}: {total} frames, {width} × {height} at {} fps, {} on {adapter}",
        job.fps(),
        options.settings.quality.label(),
    );
    for note in &job.graphics().notes {
        println!("graphics: {note}");
    }
    let mut said = Instant::now();
    loop {
        match job.step()? {
            Progress::Loading => std::thread::sleep(Duration::from_millis(5)),
            Progress::Warming { left } => {
                if said.elapsed() >= Duration::from_secs(2) {
                    said = Instant::now();
                    println!("warming up, {left} frames to go");
                }
            }
            Progress::Frame { index, total } => {
                let done = index + 1;
                if said.elapsed() >= Duration::from_secs(2) || done == total {
                    said = Instant::now();
                    let elapsed = job.elapsed();
                    let left = elapsed / f64::from(done) * f64::from(total - done);
                    println!(
                        "frame {done} of {total} ({}%), {} elapsed, about {} left",
                        done * 100 / total.max(1),
                        clock(elapsed),
                        clock(left)
                    );
                }
            }
            Progress::Finished(finished) => {
                for missing in &finished.missing {
                    eprintln!("the sequence binds `{missing}`, which wasn't found");
                }
                println!(
                    "Wrote {} ({} frames in {})",
                    finished.output.display(),
                    finished.frames,
                    clock(finished.seconds)
                );
                return Ok(());
            }
        }
    }
}
