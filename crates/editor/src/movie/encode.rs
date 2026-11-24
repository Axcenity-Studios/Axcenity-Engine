//! Writing a movie's frames: PNG files, or a video through the `ffmpeg`
//! program on the machine.
//!
//! The engine has no video encoder of its own and takes on none: a video
//! is made by running `ffmpeg` and writing each frame's raw pixels to its
//! input. Which formats are offered comes from what that `ffmpeg` says it
//! can encode (`ffmpeg -hide_banner -encoders`); a PNG sequence needs no
//! `ffmpeg` and is always offered.
//!
//! Frames are written on a thread ([`Sink`]), so encoding one runs beside
//! drawing the next. The thread takes at most two frames ahead: a 4K frame
//! is 33 MB, and a slow encoder must slow the renderer, not fill memory.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;

/// The environment variable that names the `ffmpeg` to use, in place of
/// the one on `PATH`.
pub const FFMPEG_VARIABLE: &str = "AXCENITY_FFMPEG";

/// What a movie is written as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// H.264 in an MP4: plays everywhere.
    H264,
    /// AV1 in an MP4: smaller files, newer players.
    Av1,
    /// FFV1 in a Matroska file: every pixel exactly as drawn, for editing.
    Ffv1,
    /// A folder of numbered PNG files. Needs no `ffmpeg`.
    Png,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::H264, Format::Av1, Format::Ffv1, Format::Png];

    pub fn label(self) -> &'static str {
        match self {
            Format::H264 => "H.264 video (.mp4)",
            Format::Av1 => "AV1 video (.mp4)",
            Format::Ffv1 => "Lossless FFV1 video (.mkv)",
            Format::Png => "PNG image sequence (a folder)",
        }
    }

    /// What it's good for, for a tooltip.
    pub fn hint(self) -> &'static str {
        match self {
            Format::H264 => "Plays on every device and uploads anywhere.",
            Format::Av1 => "Smaller than H.264 at the same quality. Needs a recent player.",
            Format::Ffv1 => {
                "Every pixel exactly as drawn, in a big file: for a video editor, not for sharing."
            }
            Format::Png => {
                "One PNG file a frame, in a folder. Every pixel exactly as drawn; needs no ffmpeg."
            }
        }
    }

    /// The file's extension, or `None` for a PNG sequence (a folder).
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Format::H264 | Format::Av1 => Some("mp4"),
            Format::Ffv1 => Some("mkv"),
            Format::Png => None,
        }
    }

    /// The name `--format` takes on the command line.
    pub fn flag(self) -> &'static str {
        match self {
            Format::H264 => "h264",
            Format::Av1 => "av1",
            Format::Ffv1 => "ffv1",
            Format::Png => "png",
        }
    }

    pub fn from_flag(name: &str) -> Option<Format> {
        Format::ALL
            .into_iter()
            .find(|format| format.flag().eq_ignore_ascii_case(name))
    }

    /// The format a file called `path` is, by its extension: `.mp4` is
    /// H.264, `.mkv` FFV1, and anything else a folder of PNGs.
    pub fn of_path(path: &Path) -> Format {
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
        match extension.as_deref() {
            Some("mp4") => Format::H264,
            Some("mkv") => Format::Ffv1,
            _ => Format::Png,
        }
    }
}

/// The H.264 encoders used, best first: the two software ones, then the
/// graphics cards' and the systems' own.
pub const H264_ENCODERS: [&str; 7] = [
    "libx264",
    "libopenh264",
    "h264_nvenc",
    "h264_qsv",
    "h264_amf",
    "h264_videotoolbox",
    "h264_mf",
];

/// What an `ffmpeg` can encode, from its `-encoders` listing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Encoders {
    names: Vec<String>,
}

impl Encoders {
    /// Reads `ffmpeg -hide_banner -encoders`' output: after a line of
    /// dashes, each line is flags (`V....D`), the encoder's name and what
    /// it is. Only video encoders are kept.
    pub fn parse(listing: &str) -> Encoders {
        let names = listing
            .lines()
            .skip_while(|line| !line.trim_start().starts_with("---"))
            .skip(1)
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                let flags = words.next()?;
                let name = words.next()?;
                flags.starts_with('V').then(|| name.to_owned())
            })
            .collect();
        Encoders { names }
    }

    pub fn has(&self, name: &str) -> bool {
        self.names.iter().any(|own| own == name)
    }

    /// The encoder `format` is written with, or `None` if this `ffmpeg`
    /// has none for it (a PNG sequence has none: it needs no `ffmpeg`).
    pub fn encoder(&self, format: Format) -> Option<&'static str> {
        match format {
            Format::H264 => H264_ENCODERS.into_iter().find(|name| self.has(name)),
            Format::Av1 => self.has("libsvtav1").then_some("libsvtav1"),
            Format::Ffv1 => self.has("ffv1").then_some("ffv1"),
            Format::Png => None,
        }
    }

    /// The formats this `ffmpeg` can write, and a PNG sequence.
    pub fn formats(&self) -> Vec<Format> {
        Format::ALL
            .into_iter()
            .filter(|&format| format == Format::Png || self.encoder(format).is_some())
            .collect()
    }
}

/// The `ffmpeg` found on the machine and what it encodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ffmpeg {
    pub program: PathBuf,
    pub encoders: Encoders,
}

/// The program's file name on this system: `ffmpeg`, or `ffmpeg.exe`.
pub fn program_name() -> String {
    format!("ffmpeg{}", std::env::consts::EXE_SUFFIX)
}

/// The `ffmpeg` to run: the file [`FFMPEG_VARIABLE`] names if there is
/// one, else the first `ffmpeg` (`ffmpeg.exe` on Windows) in the folders
/// of `PATH`. No shell is asked: the folders are looked in directly.
pub fn find_ffmpeg() -> Option<PathBuf> {
    find_in(
        std::env::var_os(FFMPEG_VARIABLE).map(PathBuf::from),
        std::env::var_os("PATH").as_deref(),
    )
}

/// [`find_ffmpeg`], given the variable's value and `PATH`'s.
pub fn find_in(named: Option<PathBuf>, path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    if let Some(named) = named.filter(|named| !named.as_os_str().is_empty()) {
        return named.is_file().then_some(named);
    }
    let name = program_name();
    std::env::split_paths(path?)
        .filter(|folder| !folder.as_os_str().is_empty())
        .map(|folder| folder.join(&name))
        .find(|candidate| candidate.is_file())
}

/// A command for `program` that opens no console window on Windows (the
/// editor is a GUI program there).
fn quiet_command(program: &Path) -> Command {
    #[allow(unused_mut)] // Only Windows changes it.
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Finds `ffmpeg` and asks it what it encodes. `None` when there's none,
/// or it couldn't be run. Takes a moment: call it off the UI thread.
pub fn probe() -> Option<Ffmpeg> {
    let program = find_ffmpeg()?;
    let output = quiet_command(&program)
        .args(["-hide_banner", "-encoders"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let encoders = Encoders::parse(&String::from_utf8_lossy(&output.stdout));
    Some(Ffmpeg { program, encoders })
}

/// What to tell someone with no `ffmpeg`, for this system.
pub fn install_hint() -> &'static str {
    if cfg!(windows) {
        "Install it with `winget install ffmpeg` (or from ffmpeg.org, adding its bin folder to \
         PATH), then press Check again."
    } else if cfg!(target_os = "macos") {
        "Install it with `brew install ffmpeg`, then press Check again."
    } else {
        "Install it with your package manager (`sudo dnf install ffmpeg`, `sudo apt install \
         ffmpeg`), then press Check again."
    }
}

/// A video's bit rate for encoders that are given one and not a quality:
/// about 0.17 bits a pixel a frame (27 Mbit/s at 4K 24, 11 at 1080p 30),
/// at least 2 Mbit/s.
pub fn bit_rate((width, height): (u32, u32), fps: u32) -> u64 {
    let pixels = u64::from(width) * u64::from(height) * u64::from(fps.max(1));
    (pixels * 17 / 100).max(2_000_000)
}

/// `ffmpeg`'s arguments to read raw RGBA frames of `size` at `fps` from
/// its input and write them to `output` with `encoder` (one of
/// [`Encoders::encoder`]'s). Each is its own argument, so no shell and no
/// quoting is involved.
pub fn ffmpeg_args(encoder: &str, size: (u32, u32), fps: u32, output: &Path) -> Vec<String> {
    let text = |words: &[&str]| {
        words
            .iter()
            .map(|word| (*word).to_owned())
            .collect::<Vec<_>>()
    };
    let mut args = text(&["-hide_banner", "-loglevel", "error", "-nostdin", "-y"]);
    args.extend(text(&["-f", "rawvideo", "-pix_fmt", "rgba"]));
    args.extend(["-s".to_owned(), format!("{}x{}", size.0, size.1)]);
    args.extend(["-r".to_owned(), fps.to_string()]);
    args.extend(text(&["-i", "-", "-an", "-c:v", encoder]));
    let rate = bit_rate(size, fps).to_string();
    // The picture is sRGB: say so, and turn it into the video's colors by
    // the HD rule (BT.709), which players assume, not ffmpeg's default.
    let video_colors = [
        "-vf",
        "scale=out_color_matrix=bt709:out_range=tv",
        "-pix_fmt",
        "yuv420p",
        "-colorspace",
        "bt709",
        "-color_primaries",
        "bt709",
        "-color_trc",
        "iec61966-2-1",
        "-color_range",
        "tv",
    ];
    match encoder {
        "ffv1" => {
            // Lossless, every frame on its own, checked slices.
            args.extend(text(&[
                "-level",
                "3",
                "-g",
                "1",
                "-slices",
                "24",
                "-slicecrc",
                "1",
                "-pix_fmt",
                "bgr0",
            ]));
        }
        "libx264" => {
            args.extend(text(&["-preset", "slow", "-crf", "16"]));
            args.extend(text(&video_colors));
        }
        "libsvtav1" => {
            args.extend(text(&["-preset", "6", "-crf", "24"]));
            args.extend(text(&video_colors));
        }
        "h264_nvenc" => {
            args.extend(text(&[
                "-preset", "p6", "-rc", "vbr", "-cq", "18", "-b:v", "0",
            ]));
            args.extend(text(&video_colors));
        }
        // libopenh264 and the other hardware encoders take a bit rate.
        _ => {
            args.extend(["-b:v".to_owned(), rate]);
            args.extend(text(&video_colors));
        }
    }
    if output
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
    {
        // The index at the front, so the file starts playing while it loads.
        args.extend(text(&["-movflags", "+faststart"]));
    }
    args.push(output.to_string_lossy().into_owned());
    args
}

/// The file of frame `index` (from 0) of a PNG sequence in `folder`:
/// `<stem>_0001.png` for the first.
pub fn png_frame_path(folder: &Path, stem: &str, index: u32) -> PathBuf {
    folder.join(format!("{stem}_{:04}.png", index + 1))
}

/// Writes rows of RGBA bytes as an RGB PNG (a movie frame is opaque).
pub fn write_png(path: &Path, (width, height): (u32, u32), rgba: &[u8]) -> Result<(), String> {
    let failed = |err: &dyn std::fmt::Display| format!("could not write {}: {err}", path.display());
    let file = std::fs::File::create(path).map_err(|err| failed(&err))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    // A frame a file: the fast setting is a few percent bigger and several
    // times quicker.
    encoder.set_compression(png::Compression::Fast);
    let rgb: Vec<u8> = rgba
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    let mut writer = encoder.write_header().map_err(|err| failed(&err))?;
    writer.write_image_data(&rgb).map_err(|err| failed(&err))?;
    writer.finish().map_err(|err| failed(&err))
}

/// Two frames mixed for a dissolve: `weight` of `to` over `from` (0 is all
/// `from`, 1 all `to`), byte by byte as the pictures are stored (sRGB), the
/// way video editors cross-dissolve. The result is opaque.
pub fn blend(from: &[u8], to: &[u8], weight: f32) -> Vec<u8> {
    // 8.8 fixed point: exact at both ends and quick over 33 MB.
    let weight = (weight.clamp(0.0, 1.0) * 256.0).round() as u32;
    let mut mixed = Vec::with_capacity(from.len());
    for (a, b) in from.chunks_exact(4).zip(to.chunks_exact(4)) {
        for channel in 0..3 {
            let (a, b) = (u32::from(a[channel]), u32::from(b[channel]));
            mixed.push(((a * (256 - weight) + b * weight + 128) >> 8) as u8);
        }
        mixed.push(255);
    }
    mixed
}

/// Where a movie's frames go, written on a thread of its own.
pub struct Sink {
    frames: Option<SyncSender<Vec<u8>>>,
    writer: Option<JoinHandle<Result<(), String>>>,
    video: Option<Video>,
}

/// A running `ffmpeg` and what it says on its error output.
struct Video {
    child: Child,
    said: Option<JoinHandle<String>>,
    output: PathBuf,
}

/// Frames waiting to be written, at most.
const AHEAD: usize = 2;

impl Sink {
    /// A PNG a frame in `folder` (made if it isn't there), named after
    /// `stem` ([`png_frame_path`]).
    pub fn png(folder: &Path, stem: &str, size: (u32, u32)) -> Result<Sink, String> {
        std::fs::create_dir_all(folder)
            .map_err(|err| format!("could not make {}: {err}", folder.display()))?;
        let (frames, waiting) = sync_channel::<Vec<u8>>(AHEAD);
        let (folder, stem) = (folder.to_path_buf(), stem.to_owned());
        let writer = spawn("movie frames", move || {
            for (index, frame) in waiting.into_iter().enumerate() {
                write_png(&png_frame_path(&folder, &stem, index as u32), size, &frame)?;
            }
            Ok(())
        })?;
        Ok(Sink {
            frames: Some(frames),
            writer: Some(writer),
            video: None,
        })
    }

    /// Runs `program` (an `ffmpeg`) with `args` ([`ffmpeg_args`]) and
    /// writes each frame to its input. `output` is the file it writes,
    /// taken away again if the movie is cancelled or fails.
    pub fn video(program: &Path, args: &[String], output: &Path) -> Result<Sink, String> {
        if let Some(folder) = output.parent().filter(|f| !f.as_os_str().is_empty()) {
            std::fs::create_dir_all(folder)
                .map_err(|err| format!("could not make {}: {err}", folder.display()))?;
        }
        let mut child = quiet_command(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("could not run {}: {err}", program.display()))?;
        let mut input = child.stdin.take().ok_or("ffmpeg has no input")?;
        // Read as it's said, so a long complaint can't fill the pipe and
        // stop ffmpeg.
        let said = child.stderr.take().and_then(|mut errors| {
            spawn("movie encoder errors", move || {
                let mut text = String::new();
                let _ = errors.read_to_string(&mut text);
                text
            })
            .ok()
        });
        let (frames, waiting): (_, Receiver<Vec<u8>>) = sync_channel(AHEAD);
        let writer = spawn("movie frames", move || {
            for frame in waiting {
                input
                    .write_all(&frame)
                    .map_err(|err| format!("ffmpeg stopped taking frames: {err}"))?;
            }
            // Dropping `input` here closes it: ffmpeg finishes the file.
            Ok(())
        })?;
        Ok(Sink {
            frames: Some(frames),
            writer: Some(writer),
            video: Some(Video {
                child,
                said,
                output: output.to_path_buf(),
            }),
        })
    }

    /// Hands over one frame (rows of RGBA bytes), waiting while two are
    /// already queued. An error is why the writer stopped.
    pub fn send(&mut self, frame: Vec<u8>) -> Result<(), String> {
        let sent = self
            .frames
            .as_ref()
            .is_some_and(|frames| frames.send(frame).is_ok());
        if sent { Ok(()) } else { Err(self.stop(false)) }
    }

    /// Waits for every frame to be written and the file to be finished.
    pub fn finish(mut self) -> Result<(), String> {
        self.frames = None;
        let wrote = match self.writer.take().map(JoinHandle::join) {
            Some(Ok(result)) => result,
            Some(Err(_)) => Err("the frame writer stopped unexpectedly".to_owned()),
            None => Ok(()),
        };
        let Some(mut video) = self.video.take() else {
            return wrote;
        };
        let status = video.child.wait();
        let said = video.said.take().and_then(|said| said.join().ok());
        match (wrote, status) {
            (Ok(()), Ok(status)) if status.success() => Ok(()),
            (wrote, status) => {
                let _ = std::fs::remove_file(&video.output);
                Err(encoder_error(wrote.err(), status.ok(), said))
            }
        }
    }

    /// Stops writing and throws a half-written video away (PNG files
    /// already written stay).
    pub fn cancel(mut self) {
        self.stop(true);
    }

    /// Stops everything; returns why it had stopped by itself.
    fn stop(&mut self, cancelled: bool) -> String {
        self.frames = None;
        let video = self.video.take();
        let mut status = None;
        let mut said = None;
        if let Some(mut video) = video {
            if cancelled {
                let _ = video.child.kill();
            }
            // With its input closed (or killed), ffmpeg ends by itself.
            let wrote = self.writer.take().map(JoinHandle::join);
            status = video.child.wait().ok();
            said = video.said.take().and_then(|said| said.join().ok());
            let _ = std::fs::remove_file(&video.output);
            let wrote = wrote.and_then(Result::ok).and_then(Result::err);
            return encoder_error(wrote, status, said);
        }
        let wrote = self
            .writer
            .take()
            .map(JoinHandle::join)
            .and_then(Result::ok)
            .and_then(Result::err);
        encoder_error(wrote, status, said)
    }
}

impl Drop for Sink {
    /// A sink dropped without [`finish`](Sink::finish) (the job was
    /// dropped, the editor closed): don't leave ffmpeg running.
    fn drop(&mut self) {
        if self.frames.is_some() || self.video.is_some() {
            self.stop(true);
        }
    }
}

fn spawn<T: Send + 'static>(
    name: &str,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<JoinHandle<T>, String> {
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(work)
        .map_err(|err| format!("could not start a thread: {err}"))
}

/// One line saying why encoding failed: what ffmpeg said if it said
/// anything, else how it ended, else the writer's own reason.
fn encoder_error(
    wrote: Option<String>,
    status: Option<std::process::ExitStatus>,
    said: Option<String>,
) -> String {
    let said = said.unwrap_or_default();
    let said = said.trim();
    if !said.is_empty() {
        // The last lines are the reason; the first can be many warnings.
        let lines: Vec<&str> = said.lines().rev().take(4).collect();
        let lines: Vec<&str> = lines.into_iter().rev().collect();
        return format!("ffmpeg: {}", lines.join(" / "));
    }
    match (wrote, status) {
        (_, Some(status)) if !status.success() => format!("ffmpeg ended with {status}"),
        (Some(why), _) => why,
        _ => "the movie was stopped".to_owned(),
    }
}
