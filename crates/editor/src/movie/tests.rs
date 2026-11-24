use std::ffi::OsString;
use std::path::{Path, PathBuf};

use axcenity_anim::sequence::{CameraCut, FadeSection, LetterboxSection, SequenceFile, Transition};
use axcenity_assets::Assets;
use axcenity_core::{ClassName, Color3, Value, Vector3, World};
use axcenity_render::glam::Vec3;
use axcenity_render::{Camera, Capabilities, Quality, wgpu};

use super::encode::{
    Encoders, Ffmpeg, Sink, bit_rate, blend, ffmpeg_args, find_in, png_frame_path, program_name,
};
use super::window::thumbnail;
use super::*;

// ----- The settings and what they work out to -----

#[test]
fn frames_are_counted_and_timed_from_the_frame_rate_alone() {
    assert_eq!(frame_count(0.0, 10.0, 24), 240);
    assert_eq!(frame_count(0.0, 10.0, 60), 600);
    // A range: only its seconds.
    assert_eq!(frame_count(1.2, 3.4, 30), 66);
    // Thirds don't lose a frame to rounding.
    assert_eq!(frame_count(0.0, 0.1 + 0.2, 10), 3);
    // Nothing at all is still one frame.
    assert_eq!(frame_count(2.0, 2.0, 30), 1);
    assert_eq!(frame_count(5.0, 1.0, 30), 1);

    assert_eq!(frame_time(0.0, 24, 0), 0.0);
    assert_eq!(frame_time(0.0, 24, 24), 1.0);
    assert_eq!(frame_time(1.5, 30, 15), 2.0);
    // The last frame is one frame's time before the end.
    let last = frame_count(0.0, 10.0, 25) - 1;
    assert!((frame_time(0.0, 25, last) - (10.0 - 1.0 / 25.0)).abs() < 1e-9);
    // No error builds up: frame 86 400 of an hour at 24 fps is second 3600.
    assert_eq!(frame_time(0.0, 24, 86_400), 3600.0);
}

#[test]
fn the_picture_is_the_preset_cut_to_its_shape_in_even_pixels() {
    let size = |resolution, aspect| {
        MovieSettings {
            resolution,
            aspect,
            ..MovieSettings::default()
        }
        .size(8192)
    };
    assert_eq!(size(Resolution::Hd720, Aspect::Wide), (1280, 720));
    assert_eq!(size(Resolution::Hd1080, Aspect::Wide), (1920, 1080));
    assert_eq!(size(Resolution::Qhd1440, Aspect::Wide), (2560, 1440));
    assert_eq!(size(Resolution::Uhd2160, Aspect::Wide), (3840, 2160));
    // The letterboxed look as the whole picture.
    assert_eq!(size(Resolution::Uhd2160, Aspect::Scope), (3840, 1608));
    assert_eq!(size(Resolution::Hd1080, Aspect::Scope), (1920, 804));
    assert_eq!(size(Resolution::Hd1080, Aspect::Classic), (1440, 1080));
    assert_eq!(size(Resolution::Hd1080, Aspect::Square), (1080, 1080));
    assert_eq!(size(Resolution::Hd1080, Aspect::Tall), (1080, 1920));

    // A custom size is made even (video encoders need it) and kept inside
    // what the GPU can draw.
    let custom = |width, height, most| {
        MovieSettings {
            resolution: Resolution::Custom,
            custom: (width, height),
            ..MovieSettings::default()
        }
        .size(most)
    };
    assert_eq!(custom(641, 361, 8192), (640, 360));
    assert_eq!(custom(3, 100_000, 4096), (16, 4096));
    assert_eq!(custom(9000, 9000, 16_384), (MAX_SIDE, MAX_SIDE));
}

#[test]
fn the_range_is_used_only_when_it_is_one() {
    let mut settings = MovieSettings::default();
    assert_eq!(settings.span(10.0), (0.0, 10.0));
    settings.range = Some((2.0, 6.5));
    assert_eq!(settings.span(10.0), (2.0, 6.5));
    // Kept inside the sequence.
    settings.range = Some((4.0, 60.0));
    assert_eq!(settings.span(10.0), (4.0, 10.0));
    // Backwards: the whole sequence.
    settings.range = Some((6.0, 2.0));
    assert_eq!(settings.span(10.0), (0.0, 10.0));
    assert_eq!(cut_warm_up(48), 12);
    assert_eq!(cut_warm_up(400), 16);
    assert_eq!(cut_warm_up(0), 0);
}

#[test]
fn ray_tracing_is_on_only_where_the_device_traces_rays() {
    let settings = MovieSettings::default();
    assert_eq!(settings.quality, Quality::Ultra);
    let plain = settings.graphics(&Capabilities::default());
    assert_eq!(plain.quality, Quality::Ultra);
    assert_eq!(plain.ray_traced_shadows, None);
    assert_eq!(plain.ray_traced_reflections, None);
    // The world's MotionBlurEffect is drawn as the preset says.
    assert_eq!(plain.motion_blur, None);

    let rays = Capabilities {
        ray_queries: true,
        ..Capabilities::default()
    };
    let traced = settings.graphics(&rays);
    assert!(traced.ray_traced_shadows.is_some());
    assert!(traced.ray_traced_ambient_occlusion.is_some());
    assert!(traced.ray_traced_global_illumination.is_some());
    assert!(traced.ray_traced_reflections.is_some());
    let off = MovieSettings {
        ray_tracing: false,
        ..settings.clone()
    };
    assert_eq!(off.graphics(&rays).ray_traced_shadows, None);

    // A strength set in the window is drawn even at Low.
    let blurred = MovieSettings {
        quality: Quality::Low,
        motion_blur: Some(0.5),
        ..settings
    };
    assert_eq!(
        blurred.graphics(&rays).motion_blur,
        Some(axcenity_render::MotionBlurQuality::High)
    );
    let software = Capabilities {
        software: true,
        ..Capabilities::default()
    };
    assert_eq!(MovieSettings::best_quality(&software), Quality::Low);
    assert_eq!(MovieSettings::best_quality(&rays), Quality::Ultra);
}

#[test]
fn the_output_follows_the_sequences_name_and_the_format() {
    let project = Path::new("/games/Desert");
    assert_eq!(
        default_output(project, "Intro", Format::H264),
        project.join("movies/Intro.mp4")
    );
    assert_eq!(
        default_output(project, "Intro", Format::Ffv1),
        project.join("movies/Intro.mkv")
    );
    // A PNG sequence is a folder.
    assert_eq!(
        default_output(project, "Intro", Format::Png),
        project.join("movies/Intro")
    );
    // A change of format changes only the extension.
    let mp4 = Path::new("/films/Take 2.mp4");
    assert_eq!(
        with_format(mp4, Format::Ffv1),
        Path::new("/films/Take 2.mkv")
    );
    assert_eq!(with_format(mp4, Format::Av1), mp4);
    assert_eq!(with_format(mp4, Format::Png), Path::new("/films/Take 2"));
    assert_eq!(
        with_format(Path::new("/films/Take 2"), Format::H264),
        Path::new("/films/Take 2.mp4")
    );
    // A dot in the name isn't an extension to throw away.
    assert_eq!(
        with_format(Path::new("/films/Intro.v2"), Format::H264),
        Path::new("/films/Intro.v2.mp4")
    );
    assert_eq!(Format::of_path(Path::new("a/b.MP4")), Format::H264);
    assert_eq!(Format::of_path(Path::new("a/b.mkv")), Format::Ffv1);
    assert_eq!(Format::of_path(Path::new("a/frames")), Format::Png);
    assert_eq!(clock(0.4), "0:00");
    assert_eq!(clock(83.0), "1:23");
}

// ----- ffmpeg -----

/// The start of a real `ffmpeg -hide_banner -encoders`.
const LISTING: &str = "Encoders:
 V..... = Video
 A..... = Audio
 S..... = Subtitle
 .F.... = Frame-level multithreading
 ..S... = Slice-level multithreading
 ...X.. = Codec is experimental
 ....B. = Supports draw_horiz_band
 .....D = Supports direct rendering method 1
 ------
 V....D a64multi             Multicolor charset for Commodore 64 (codec a64_multi)
 V..... libsvtav1            SVT-AV1(Scalable Video Technology for AV1) encoder (codec av1)
 V.S..D ffv1                 FFmpeg video codec #1
 V....D libopenh264          OpenH264 H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10 (codec h264)
 V....D h264_nvenc           NVIDIA NVENC H.264 encoder (codec h264)
 VF...D png                  PNG (Portable Network Graphics) image
 A....D libx264              Not a video encoder: an audio one with a misleading name
 A....D aac                  AAC (Advanced Audio Coding)
";

#[test]
fn the_formats_offered_come_from_what_ffmpeg_lists() {
    let encoders = Encoders::parse(LISTING);
    assert!(encoders.has("ffv1") && encoders.has("libsvtav1") && encoders.has("png"));
    // Not the legend's lines, and not audio encoders.
    assert!(!encoders.has("="));
    assert!(!encoders.has("aac") && !encoders.has("libx264"));
    // No libx264 here: the other software encoder, before the hardware one.
    assert_eq!(encoders.encoder(Format::H264), Some("libopenh264"));
    assert_eq!(encoders.encoder(Format::Av1), Some("libsvtav1"));
    assert_eq!(encoders.encoder(Format::Ffv1), Some("ffv1"));
    assert_eq!(encoders.encoder(Format::Png), None);
    assert_eq!(encoders.formats(), Format::ALL);

    // libx264 wins when it's there.
    let with_x264 = LISTING.replace(" A....D libx264", " V....D libx264");
    assert_eq!(
        Encoders::parse(&with_x264).encoder(Format::H264),
        Some("libx264")
    );
    // Only a graphics card's encoder: that one.
    let hardware = LISTING.replace("libopenh264", "something_else");
    assert_eq!(
        Encoders::parse(&hardware).encoder(Format::H264),
        Some("h264_nvenc")
    );
    // An ffmpeg built without any of them still gets a PNG sequence, and
    // so does a listing that makes no sense.
    let bare = " ------\n V....D rawvideo             raw video\n";
    assert_eq!(Encoders::parse(bare).formats(), [Format::Png]);
    assert_eq!(Encoders::parse("").formats(), [Format::Png]);
    assert_eq!(Encoders::default().formats(), [Format::Png]);
}

#[test]
fn ffmpeg_is_given_raw_frames_on_its_input_and_one_argument_each() {
    let output = Path::new("/films/My Movie/Intro.mp4");
    let args = ffmpeg_args("libx264", (3840, 1608), 24, output);
    let has = |words: &[&str]| args.windows(words.len()).any(|window| window == words);
    assert!(has(&["-f", "rawvideo", "-pix_fmt", "rgba"]));
    assert!(has(&["-s", "3840x1608"]));
    assert!(has(&["-r", "24"]));
    assert!(has(&["-i", "-"]), "frames come in on its input");
    assert!(has(&["-c:v", "libx264"]));
    assert!(has(&["-pix_fmt", "yuv420p"]), "plays everywhere");
    assert!(has(&["-movflags", "+faststart"]));
    assert!(has(&["-y"]) && has(&["-nostdin"]));
    // The path is one argument, spaces and all: nothing is quoted because
    // no shell reads it.
    assert_eq!(args.last().unwrap(), "/films/My Movie/Intro.mp4");
    // Input options come before the input, output options after.
    let input = args.iter().position(|arg| arg == "-i").unwrap();
    let size = args.iter().position(|arg| arg == "-s").unwrap();
    let codec = args.iter().position(|arg| arg == "-c:v").unwrap();
    assert!(size < input && input < codec);

    // Lossless: every frame whole, no change of colors, no MP4 flags.
    let lossless = ffmpeg_args("ffv1", (1920, 1080), 30, Path::new("a.mkv"));
    let has = |words: &[&str]| lossless.windows(words.len()).any(|w| w == words);
    assert!(has(&["-c:v", "ffv1"]) && has(&["-g", "1"]) && has(&["-pix_fmt", "bgr0"]));
    assert!(
        !lossless
            .iter()
            .any(|arg| arg == "-movflags" || arg == "-vf")
    );

    // An encoder with no quality setting is given a bit rate by the size.
    let open = ffmpeg_args("libopenh264", (1920, 1080), 30, output);
    let rate = open.iter().position(|arg| arg == "-b:v").unwrap();
    assert_eq!(open[rate + 1], bit_rate((1920, 1080), 30).to_string());
    assert!(bit_rate((3840, 2160), 24) > bit_rate((1920, 1080), 24));
    assert_eq!(bit_rate((160, 90), 10), 2_000_000);
    let av1 = ffmpeg_args("libsvtav1", (1280, 720), 60, output);
    assert!(av1.windows(2).any(|w| w == ["-c:v", "libsvtav1"]));
}

/// A folder of this test's own under the system's temporary one, empty.
fn scratch(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("axcenity-movie-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn ffmpeg_is_looked_for_in_the_folders_of_path_by_its_name_on_this_system() {
    let folder = scratch("find");
    let (empty, tools) = (folder.join("empty"), folder.join("tools"));
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::create_dir_all(&tools).unwrap();
    let program = tools.join(program_name());
    assert!(program_name().starts_with("ffmpeg"));
    assert_eq!(cfg!(windows), program_name().ends_with(".exe"));
    let path = std::env::join_paths([&empty, &tools]).unwrap();
    assert_eq!(find_in(None, Some(&path)), None, "not there yet");
    std::fs::write(&program, b"").unwrap();
    assert_eq!(find_in(None, Some(&path)), Some(program.clone()));
    assert_eq!(find_in(None, None), None, "no PATH at all");
    // A folder called ffmpeg isn't the program.
    std::fs::create_dir_all(empty.join(program_name())).unwrap();
    assert_eq!(find_in(None, Some(&path)), Some(program.clone()));
    // The variable names one outright; a wrong one finds nothing, so a
    // typo is noticed.
    let own = folder.join("my-ffmpeg");
    std::fs::write(&own, b"").unwrap();
    assert_eq!(find_in(Some(own.clone()), Some(&path)), Some(own));
    assert_eq!(find_in(Some(folder.join("missing")), Some(&path)), None);
    assert_eq!(find_in(Some(PathBuf::new()), Some(&path)), Some(program));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_dissolve_mixes_two_frames_by_its_weight() {
    let from = [10, 200, 0, 255, 0, 0, 0, 7];
    let to = [110, 100, 255, 0, 255, 255, 255, 99];
    assert_eq!(blend(&from, &to, 0.0), [10, 200, 0, 255, 0, 0, 0, 255]);
    assert_eq!(
        blend(&from, &to, 1.0),
        [110, 100, 255, 255, 255, 255, 255, 255]
    );
    assert_eq!(
        blend(&from, &to, 0.5),
        [60, 150, 128, 255, 128, 128, 128, 255]
    );
    let quarter = blend(&from, &to, 0.25);
    assert_eq!(&quarter[..3], [35, 175, 64]);
    // Out of range weights are held to the ends.
    assert_eq!(blend(&from, &to, -3.0), blend(&from, &to, 0.0));
    assert_eq!(blend(&from, &to, 7.0), blend(&from, &to, 1.0));
}

#[test]
fn a_frame_is_shown_small_while_rendering() {
    // 8 × 4 pixels, each its own shade.
    let frame: Vec<u8> = (0..32u8).flat_map(|n| [n, n, n, 255]).collect();
    let small = thumbnail(&frame, (8, 4), 4);
    assert_eq!(small.size, [4, 2]);
    assert_eq!(small.pixels[0], eframe::egui::Color32::from_rgb(0, 0, 0));
    assert_eq!(small.pixels[1], eframe::egui::Color32::from_rgb(2, 2, 2));
    assert_eq!(small.pixels[4], eframe::egui::Color32::from_rgb(16, 16, 16));
    // Already small enough: as it is.
    assert_eq!(thumbnail(&frame, (8, 4), 400).size, [8, 4]);
}

// ----- The command line -----

fn args(text: &str) -> impl Iterator<Item = OsString> {
    text.split_whitespace()
        .map(OsString::from)
        .collect::<Vec<_>>()
        .into_iter()
}

#[test]
fn the_command_line_sets_what_the_window_does() {
    let options = cli::parse(args(
        "games/Desert --sequence Intro --out out/Intro.mp4 --size 3840x1608 --fps 24 \
         --quality ultra --format av1 --range 1.5:4 --warm-up 20 --motion-blur 0.4 \
         --no-letterbox --no-ui --no-ray-tracing",
    ))
    .unwrap();
    assert_eq!(options.project, PathBuf::from("games/Desert"));
    assert_eq!(options.sequence, "Intro");
    let settings = &options.settings;
    assert_eq!(settings.size(8192), (3840, 1608));
    assert_eq!(settings.fps, 24);
    assert_eq!(settings.quality, Quality::Ultra);
    assert_eq!(settings.format, Format::Av1);
    assert_eq!(settings.range, Some((1.5, 4.0)));
    assert_eq!(settings.warm_up, 20);
    assert_eq!(settings.motion_blur, Some(0.4));
    assert!(!settings.letterbox && !settings.game_ui && !settings.ray_tracing);

    // The least: the format comes from the file's name, the rest are the
    // window's own defaults.
    let least = cli::parse(args("games/Desert --sequence Intro --out Intro.mkv")).unwrap();
    assert_eq!(least.settings.format, Format::Ffv1);
    assert_eq!(least.settings.size(8192), (1920, 1080));
    assert_eq!(least.settings.fps, 30);
    assert_eq!(least.settings.warm_up, DEFAULT_WARM_UP);
    assert!(least.settings.ray_tracing && least.settings.letterbox);
    let folder = cli::parse(args("games/Desert --sequence Intro --out frames")).unwrap();
    assert_eq!(folder.settings.format, Format::Png);

    for (wrong, why) in [
        ("", "project folder is missing"),
        ("games/Desert --out a.mp4", "--sequence is missing"),
        ("games/Desert --sequence Intro", "--out is missing"),
        (
            "games/Desert --sequence Intro --out a.mp4 --size big",
            "--size can't be `big`",
        ),
        (
            "games/Desert --sequence Intro --out a.mp4 --fps 0",
            "--fps can't be `0`",
        ),
        (
            "games/Desert --sequence Intro --out a.mp4 --format gif",
            "--format can't be `gif`",
        ),
        (
            "games/Desert --sequence Intro --out a.mp4 --range 4:1",
            "--range can't be `4:1`",
        ),
        (
            "games/Desert --sequence Intro --out a.mp4 --fps",
            "--fps needs a value",
        ),
        (
            "games/Desert --sequence Intro --out a.mp4 --loud yes",
            "unknown option --loud",
        ),
    ] {
        let error = cli::parse(args(wrong)).unwrap_err();
        assert!(error.contains(why), "`{wrong}`: {error}");
    }
}

// ----- Rendering -----

fn test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = axcenity_render::adapters::instance();
    let adapter = pollster::block_on(axcenity_render::adapters::adapter(&instance, None)).ok()?;
    pollster::block_on(adapter.request_device(&Default::default())).ok()
}

/// A floor with a red block on the left and a blue one on the right, a
/// Camera named Left on the red one and a Camera named Right on the blue.
fn tiny_world() -> World {
    let mut world = World::new();
    let workspace = world.workspace();
    let vector = |x: f32, y: f32, z: f32| Value::Vector3(Vector3::new(x, y, z));
    let mut part = |name: &str, position: Value, size: Value, color: [f32; 3]| {
        let id = world.create(ClassName::Part).unwrap();
        world.set_name(id, name).unwrap();
        world.set_property(id, "Position", position).unwrap();
        world.set_property(id, "Size", size).unwrap();
        let [r, g, b] = color;
        world
            .set_property(id, "Color", Value::Color3(Color3::new(r, g, b)))
            .unwrap();
        world
            .set_property(id, "Anchored", Value::Bool(true))
            .unwrap();
        world.set_parent(id, Some(workspace)).unwrap();
    };
    part(
        "Floor",
        vector(0.0, -0.5, 0.0),
        vector(200.0, 1.0, 200.0),
        [0.5, 0.5, 0.5],
    );
    part(
        "Red",
        vector(-40.0, 3.0, 0.0),
        vector(6.0, 6.0, 6.0),
        [0.9, 0.05, 0.05],
    );
    part(
        "Blue",
        vector(40.0, 3.0, 0.0),
        vector(6.0, 6.0, 6.0),
        [0.05, 0.05, 0.9],
    );
    for (name, x) in [("Left", -40.0), ("Right", 40.0)] {
        let camera = world.create(ClassName::Camera).unwrap();
        world.set_name(camera, name).unwrap();
        // Straight ahead is -Z: from in front of the block, level.
        world
            .set_property(camera, "Position", vector(x, 3.0, 12.0))
            .unwrap();
        world.set_parent(camera, Some(workspace)).unwrap();
    }
    world
}

fn cut(id: &str, time: f64, camera: &str, transition: Transition) -> CameraCut {
    CameraCut {
        id: id.to_owned(),
        time,
        camera: format!("Workspace/{camera}"),
        transition,
    }
}

/// One second: Left until half way, then Right.
fn two_shots(transition: Transition) -> SequenceFile {
    let mut file = SequenceFile::new(1.0);
    file.camera_cuts = vec![
        cut("left", 0.0, "Left", Transition::Cut),
        cut("right", 0.5, "Right", transition),
    ];
    file
}

fn settings(output: &Path) -> MovieSettings {
    MovieSettings {
        resolution: Resolution::Custom,
        custom: (160, 90),
        fps: 10,
        quality: Quality::High,
        ray_tracing: false,
        warm_up: 2,
        format: Format::Png,
        output: output.to_path_buf(),
        ..MovieSettings::default()
    }
}

/// Renders `sequence` in the tiny world to the end; the finished movie.
fn render(
    (device, queue): &(wgpu::Device, wgpu::Queue),
    sequence: SequenceFile,
    settings: &MovieSettings,
    ffmpeg: Option<&Ffmpeg>,
) -> Finished {
    let source = MovieSource {
        world: tiny_world(),
        sequence,
        assets: Assets::default(),
        stylesheets: Vec::new(),
        camera: Camera {
            position: Vec3::new(0.0, 40.0, 0.1),
            look_at: Vec3::ZERO,
            fov_y_degrees: 50.0,
        },
    };
    let mut job = MovieJob::start(device, queue, source, settings, ffmpeg).unwrap();
    let mut kept = 0;
    let mut steps = 0;
    loop {
        steps += 1;
        assert!(steps < 10_000, "the movie never ends");
        match job.step().unwrap() {
            Progress::Loading | Progress::Warming { .. } => {}
            Progress::Frame { index, total } => {
                assert_eq!(index, kept, "frames come in order");
                assert_eq!(total, job.frames().1);
                assert_eq!(job.last_frame().len(), 160 * 90 * 4);
                kept += 1;
            }
            Progress::Finished(finished) => {
                assert_eq!(finished.frames, kept);
                return finished;
            }
        }
    }
}

/// The frames of a PNG sequence in `folder`, each as RGB bytes.
fn frames(folder: &Path, count: u32) -> Vec<Vec<u8>> {
    let stem = folder.file_name().unwrap().to_string_lossy().into_owned();
    (0..count)
        .map(|index| {
            let path = png_frame_path(folder, &stem, index);
            let image = image::open(&path)
                .unwrap_or_else(|err| panic!("{}: {err}", path.display()))
                .to_rgb8();
            assert_eq!(image.dimensions(), (160, 90), "{}", path.display());
            image.into_raw()
        })
        .collect()
}

/// The mean of each channel over a frame.
fn mean(frame: &[u8]) -> [f32; 3] {
    let mut sum = [0.0_f32; 3];
    for pixel in frame.chunks_exact(3) {
        for channel in 0..3 {
            sum[channel] += f32::from(pixel[channel]);
        }
    }
    sum.map(|total| total / (frame.len() / 3) as f32)
}

/// The biggest difference between two frames' bytes.
fn most_different(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap()
}

#[test]
fn a_sequence_renders_to_numbered_pngs_with_its_cut_fade_and_bars() {
    let Some(gpu) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let folder = scratch("cut").join("Short");
    let mut sequence = two_shots(Transition::Cut);
    // Black from 0.8 s on.
    sequence.fades.push(FadeSection {
        id: "out".into(),
        start: 0.7,
        end: 0.8,
        from: 0.0,
        to: 1.0,
        color: [0.0, 0.0, 0.0],
    });
    // Bars a fifth of the height, top and bottom, all the way through.
    sequence.letterbox.push(LetterboxSection {
        id: "bars".into(),
        start: 0.0,
        end: 1.0,
        size: 0.2,
        ramp: 0.0,
    });
    let finished = render(&gpu, sequence.clone(), &settings(&folder), None);
    // One second at 10 frames a second: 10 frames, each the movie's size.
    assert_eq!(finished.frames, 10);
    assert_eq!(finished.size, (160, 90));
    assert_eq!(finished.output, folder);
    assert_eq!(finished.missing, Vec::<String>::new());
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 10);
    assert!(folder.join("Short_0001.png").is_file());
    assert!(folder.join("Short_0010.png").is_file());
    let shown = frames(&folder, 10);

    // The first shot shows the red block, the second the blue one: the
    // cut at 0.5 s is between frames 4 and 5.
    let middle = |frame: &[u8]| {
        let at = (45 * 160 + 80) * 3;
        [frame[at], frame[at + 1], frame[at + 2]]
    };
    for (index, frame) in shown.iter().enumerate().take(7) {
        let [r, _, b] = middle(frame);
        if index < 5 {
            assert!(r > 100 && b < 80, "frame {index} shows red: {r} {b}");
        } else {
            assert!(b > 100 && r < 80, "frame {index} shows blue: {r} {b}");
        }
    }
    assert!(most_different(&shown[4], &shown[5]) > 100, "the cut");
    // Nothing moves within a shot.
    assert!(most_different(&shown[0], &shown[4]) <= 2);

    // The bars: the top and bottom fifth are black, the middle isn't.
    let row = |frame: &[u8], y: usize| mean(&frame[y * 160 * 3..(y + 1) * 160 * 3]);
    assert_eq!(row(&shown[0], 4), [0.0; 3]);
    assert_eq!(row(&shown[0], 85), [0.0; 3]);
    assert!(row(&shown[0], 45).iter().sum::<f32>() > 60.0);

    // The fade: covered from 0.8 s (frame 8) on.
    assert!(mean(&shown[6]).iter().sum::<f32>() > 60.0);
    assert_eq!(mean(&shown[8]), [0.0; 3], "the fade covers the picture");
    assert_eq!(mean(&shown[9]), [0.0; 3]);

    // A second run draws the very same frames.
    let again = scratch("cut-again").join("Short");
    render(&gpu, sequence.clone(), &settings(&again), None);
    assert_eq!(frames(&again, 10), shown, "two runs give identical frames");

    // Only the play range, without the bars: its frames alone, and the
    // top of the picture is the world again.
    let part = scratch("cut-range").join("Short");
    let ranged = MovieSettings {
        range: Some((0.5, 0.7)),
        letterbox: false,
        ..settings(&part)
    };
    let finished = render(&gpu, sequence, &ranged, None);
    assert_eq!(finished.frames, 2);
    let ranged = frames(&part, 2);
    let [r, _, b] = middle(&ranged[0]);
    assert!(b > 100 && r < 80, "the range starts in the second shot");
    assert!(row(&ranged[0], 4).iter().sum::<f32>() > 60.0, "no bars");
    for folder in [folder, again, part] {
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
    }
}

#[test]
fn a_dissolves_frames_are_between_its_two_shots() {
    let Some(gpu) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let folder = scratch("dissolve").join("Mix");
    // 0.6 s centred on the cut at 0.5 s: from 0.25 s (it can't start
    // before the middle of the first shot) to 0.8 s.
    let sequence = two_shots(Transition::Dissolve { duration: 0.6 });
    assert_eq!(sequence.transition_span(1), Some((0.25, 0.8)));
    render(&gpu, sequence.clone(), &settings(&folder), None);
    let shown = frames(&folder, 10);
    // Frames 0 to 2 are the first shot alone, 8 and 9 the second.
    let (left, right) = (&shown[0], &shown[9]);
    assert!(most_different(left, &shown[2]) <= 2);
    assert!(most_different(right, &shown[8]) <= 2);
    assert!(most_different(left, right) > 100);
    // Each frame between is the two mixed by how far the dissolve is.
    let mut last_red = f32::INFINITY;
    for index in 3..8 {
        let time = f64::from(index) / 10.0;
        let (_, weight) = sequence.dissolve_at(time).unwrap();
        let rgba = |frame: &[u8]| -> Vec<u8> {
            frame
                .chunks_exact(3)
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
                .collect()
        };
        let wanted = blend(&rgba(left), &rgba(right), weight);
        let got = rgba(&shown[index as usize]);
        assert!(
            most_different(&got, &wanted) <= 3,
            "frame {index} is {weight} of the way from the first shot to the second"
        );
        // Less and less of the first shot's red block.
        let red = mean(&shown[index as usize])[0];
        assert!(red < last_red, "frame {index}");
        last_red = red;
    }
    // The middle frame really is between: neither shot.
    assert!(most_different(&shown[5], left) > 40);
    assert!(most_different(&shown[5], right) > 40);

    // The same sequence with a plain cut has no such frames.
    let plain = scratch("dissolve-cut").join("Mix");
    render(&gpu, two_shots(Transition::Cut), &settings(&plain), None);
    let cut = frames(&plain, 10);
    assert!(most_different(&cut[4], left) <= 2);
    assert!(most_different(&cut[5], right) <= 2);
    for folder in [folder, plain] {
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
    }
}

/// A subtitle takes the same share of the picture at any size: twice the
/// pixels on a picture twice as tall.
#[test]
fn subtitles_keep_their_share_of_the_picture() {
    let Some(gpu) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut plain = SequenceFile::new(0.2);
    plain.camera_cuts = vec![cut("left", 0.0, "Left", Transition::Cut)];
    let mut spoken = plain.clone();
    spoken
        .subtitles
        .push(axcenity_anim::sequence::SubtitleSection {
            id: "line".into(),
            start: 0.0,
            end: 0.2,
            key: String::new(),
            text: "MMMMMMMM".into(),
        });
    let first_frame = |sequence: &SequenceFile, height: u32| {
        let width = height * 16 / 9;
        let folder = scratch(&format!("subtitle-{height}")).join("Line");
        let sized = MovieSettings {
            custom: (width, height),
            warm_up: 0,
            ..settings(&folder)
        };
        let source = MovieSource {
            world: tiny_world(),
            sequence: sequence.clone(),
            assets: Assets::default(),
            stylesheets: Vec::new(),
            camera: Camera {
                position: Vec3::ONE,
                look_at: Vec3::ZERO,
                fov_y_degrees: 50.0,
            },
        };
        let mut job = MovieJob::start(&gpu.0, &gpu.1, source, &sized, None).unwrap();
        while !matches!(job.step().unwrap(), Progress::Finished(_)) {}
        let image = image::open(png_frame_path(&folder, "Line", 0))
            .unwrap()
            .to_rgb8();
        assert_eq!(image.dimensions(), (width, height));
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
        image
    };
    // The share of the picture's rows the text changes.
    let rows_of_text = |height: u32| {
        let (with, without) = (first_frame(&spoken, height), first_frame(&plain, height));
        let rows = with
            .rows()
            .zip(without.rows())
            .filter(|(a, b)| {
                a.clone()
                    .zip(b.clone())
                    .any(|(a, b)| (0..3).any(|c| a.0[c].abs_diff(b.0[c]) > 60))
            })
            .count();
        rows as f32 / height as f32
    };
    let (small, big) = (rows_of_text(270), rows_of_text(540));
    assert!(small > 0.01, "the subtitle is drawn: {small}");
    assert!(
        (small - big).abs() < 0.012,
        "the same share of a 270 and a 540 pixel picture: {small} and {big}"
    );
}

/// Particles and rain move by the frame's time alone: the same frames
/// whether the machine draws them at once or with a wait between.
#[test]
fn particles_and_weather_move_on_a_fixed_step_whatever_the_machines_pace() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // The tiny world with sparks rising from the red block, and rain.
    let stormy = || {
        let mut world = tiny_world();
        let red = world.find_first_child(world.workspace(), "Red").unwrap();
        let sparks = world.create(ClassName::ParticleEmitter).unwrap();
        world
            .set_property(sparks, "Rate", Value::Number(80.0))
            .unwrap();
        world
            .set_property(sparks, "Speed", Value::Number(12.0))
            .unwrap();
        world.set_parent(sparks, Some(red)).unwrap();
        let weather = world.create(ClassName::Weather).unwrap();
        world
            .set_property(weather, "Intensity", Value::Number(1.0))
            .unwrap();
        world.set_parent(weather, Some(world.lighting())).unwrap();
        world
    };
    let mut sequence = SequenceFile::new(0.6);
    sequence.camera_cuts = vec![cut("left", 0.0, "Left", Transition::Cut)];
    let run = |name: &str, wait: std::time::Duration| {
        let folder = scratch(name).join("Storm");
        let source = MovieSource {
            world: stormy(),
            sequence: sequence.clone(),
            assets: Assets::default(),
            stylesheets: Vec::new(),
            camera: Camera {
                position: Vec3::ONE,
                look_at: Vec3::ZERO,
                fov_y_degrees: 50.0,
            },
        };
        let mut job = MovieJob::start(&device, &queue, source, &settings(&folder), None).unwrap();
        loop {
            match job.step().unwrap() {
                Progress::Finished(finished) => {
                    assert_eq!(finished.frames, 6);
                    break;
                }
                _ => std::thread::sleep(wait),
            }
        }
        let shown = frames(&folder, 6);
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
        shown
    };
    let quick = run("storm-quick", std::time::Duration::ZERO);
    let slow = run("storm-slow", std::time::Duration::from_millis(40));
    assert_eq!(quick, slow, "the pace of drawing changes nothing");
    // And they do move: every frame differs from the one before.
    for pair in quick.windows(2) {
        assert!(
            most_different(&pair[0], &pair[1]) > 30,
            "sparks and rain move"
        );
    }
    // The run-up had them out before the first frame: it isn't the still
    // world's.
    let folder = scratch("storm-none").join("Storm");
    render(&(device, queue), sequence, &settings(&folder), None);
    let calm = frames(&folder, 1);
    assert!(
        most_different(&calm[0], &quick[0]) > 30,
        "falling from frame 1"
    );
    let _ = std::fs::remove_dir_all(folder.parent().unwrap());
}

/// A sequence's Environment track is in the movie: the same blend the
/// game and the Sequencer's preview show, with the project's own presets.
#[test]
fn a_movie_renders_a_sequences_environment_track() {
    use axcenity_anim::environment::{
        ClockTurn, EnvironmentKey, EnvironmentPreset, EnvironmentTarget, PresetValue, preset_path,
    };
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // A project whose own preset, Midnight, is only an hour of the day.
    let project = scratch("environment-project");
    let mut midnight = EnvironmentPreset::new();
    midnight.set(
        EnvironmentTarget::Lighting,
        "ClockTime",
        PresetValue::Number(0.0),
    );
    let file = project.join(preset_path("Midnight"));
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, midnight.to_json()).unwrap();
    // One second from Clear (13.4 h) on to Midnight, at an even pace: the
    // sun sets a little before half way.
    let mut sequence = SequenceFile::new(1.0);
    sequence.camera_cuts = vec![cut("left", 0.0, "Left", Transition::Cut)];
    let key = |id: &str, time: f64, preset: &str| EnvironmentKey {
        interp: axcenity_anim::Interp::Linear,
        easing: None,
        clock: ClockTurn::Forward,
        ..EnvironmentKey::new(id, time, preset)
    };
    sequence.environment = vec![key("day", 0.0, "Clear"), key("night", 1.0, "Midnight")];
    let render_with = |name: &str, sequence: SequenceFile| {
        let folder = scratch(name).join("Sky");
        let mut world = tiny_world();
        let lighting = world.lighting();
        for class in [ClassName::Clouds, ClassName::Weather] {
            let id = world.create(class).unwrap();
            world.set_parent(id, Some(lighting)).unwrap();
        }
        let source = MovieSource {
            world,
            sequence,
            assets: Assets::new(&project),
            stylesheets: Vec::new(),
            camera: Camera {
                position: Vec3::ONE,
                look_at: Vec3::ZERO,
                fov_y_degrees: 50.0,
            },
        };
        let mut job = MovieJob::start(&device, &queue, source, &settings(&folder), None).unwrap();
        loop {
            if let Progress::Finished(finished) = job.step().unwrap() {
                assert_eq!(finished.frames, 10);
                break;
            }
        }
        let shown = frames(&folder, 10);
        let _ = std::fs::remove_dir_all(folder.parent().unwrap());
        shown
    };
    let light = |frame: &Vec<u8>| mean(frame).iter().sum::<f32>() / 3.0;
    let keyed = render_with("environment-keyed", sequence.clone());
    let lights: Vec<f32> = keyed.iter().map(light).collect();
    // Day at the start, night at the end, and dusk on the way.
    assert!(
        lights[0] > lights[9] + 40.0,
        "day is far brighter than night: {lights:?}"
    );
    assert!(
        lights
            .iter()
            .any(|l| *l < lights[0] - 8.0 && *l > lights[9] + 8.0),
        "the light goes down over frames, not at once: {lights:?}"
    );
    assert!(
        lights.windows(2).all(|pair| pair[1] <= pair[0] + 4.0),
        "it never gets lighter on the way to midnight: {lights:?}"
    );
    // The first frame is the Clear preset's world, not the tiny world's own
    // morning with half the sky in cloud.
    let mut plain = sequence.clone();
    plain.environment.clear();
    let unkeyed = render_with("environment-unkeyed", plain);
    assert!(
        most_different(&keyed[0], &unkeyed[0]) > 8,
        "the preset shows from the first frame"
    );
    let steady: Vec<f32> = unkeyed.iter().map(light).collect();
    assert!(
        (steady[0] - steady[9]).abs() < 6.0,
        "without the track the light holds: {steady:?}"
    );
    // A preset the project hasn't got is skipped, and the movie still
    // renders: the key that's left holds its look.
    let mut missing = sequence;
    missing.environment[1].preset = "Monsoon".into();
    let held = render_with("environment-missing", missing);
    let held: Vec<f32> = held.iter().map(light).collect();
    assert!((held[0] - held[9]).abs() < 6.0, "{held:?}");
    let _ = std::fs::remove_dir_all(&project);
}

#[test]
fn a_video_needs_ffmpeg_and_says_so() {
    let Some(gpu) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let folder = scratch("no-ffmpeg");
    let source = || MovieSource {
        world: tiny_world(),
        sequence: two_shots(Transition::Cut),
        assets: Assets::default(),
        stylesheets: Vec::new(),
        camera: Camera {
            position: Vec3::ONE,
            look_at: Vec3::ZERO,
            fov_y_degrees: 50.0,
        },
    };
    let video = MovieSettings {
        format: Format::H264,
        ..settings(&folder.join("a.mp4"))
    };
    let Err(why) = MovieJob::start(&gpu.0, &gpu.1, source(), &video, None) else {
        panic!("a video started without ffmpeg");
    };
    assert!(why.contains("ffmpeg") && why.contains("PNG"), "{why}");
    // An ffmpeg that can't encode the format: said too.
    let bare = Ffmpeg {
        program: PathBuf::from("ffmpeg"),
        encoders: Encoders::default(),
    };
    let Err(why) = MovieJob::start(&gpu.0, &gpu.1, source(), &video, Some(&bare)) else {
        panic!("a video started without an encoder");
    };
    assert!(why.contains("no encoder for H.264"), "{why}");
    let nowhere = MovieSettings {
        output: PathBuf::new(),
        ..settings(&folder)
    };
    assert!(MovieJob::start(&gpu.0, &gpu.1, source(), &nowhere, None).is_err());
    assert!(!folder.join("a.mp4").exists());
    let _ = std::fs::remove_dir_all(&folder);
}

/// With the machine's own ffmpeg: skipped where there's none.
#[test]
fn ffmpeg_encodes_every_format_it_offers() {
    let Some(ffmpeg) = encode::probe() else {
        eprintln!("skipped: no ffmpeg on this machine");
        return;
    };
    let folder = scratch("encode");
    let size = (64, 48);
    // Eight frames going from black to white.
    let frame = |shade: u8| -> Vec<u8> {
        (0..size.0 * size.1)
            .flat_map(|_| [shade, shade, shade, 255])
            .collect()
    };
    let mut made = 0;
    for format in ffmpeg.encoders.formats() {
        let Some(encoder) = ffmpeg.encoders.encoder(format) else {
            continue;
        };
        // A graphics card's encoder is listed whether or not the card is
        // there: only the software ones must work everywhere.
        let software = matches!(encoder, "libx264" | "libopenh264" | "libsvtav1" | "ffv1");
        let output = with_format(&folder.join(format.flag()), format);
        let args = ffmpeg_args(encoder, size, 10, &output);
        let mut sink = Sink::video(&ffmpeg.program, &args, &output).unwrap();
        let sent = (0..8).try_for_each(|n| sink.send(frame(n * 36)));
        let result = sent.and_then(|()| sink.finish());
        match result {
            Ok(()) => {}
            Err(why) if !software => {
                eprintln!("{encoder} isn't usable here: {why}");
                assert!(!output.exists(), "a failed video is taken away");
                continue;
            }
            Err(why) => panic!("{encoder}: {why}"),
        }
        let bytes = std::fs::read(&output).unwrap();
        assert!(bytes.len() > 200, "{encoder} wrote {} bytes", bytes.len());
        match format.extension() {
            Some("mp4") => assert_eq!(&bytes[4..8], b"ftyp", "{encoder} wrote an MP4"),
            Some("mkv") => assert_eq!(&bytes[..4], [0x1A, 0x45, 0xDF, 0xA3], "a Matroska file"),
            _ => unreachable!("a PNG sequence has no encoder"),
        }
        made += 1;
    }
    eprintln!("encoded {made} formats with {}", ffmpeg.program.display());

    // A cancelled video leaves no half-written file.
    if let Some(encoder) = ffmpeg.encoders.encoder(Format::Ffv1) {
        let output = folder.join("cancelled.mkv");
        let args = ffmpeg_args(encoder, size, 10, &output);
        let mut sink = Sink::video(&ffmpeg.program, &args, &output).unwrap();
        sink.send(frame(10)).unwrap();
        sink.cancel();
        assert!(!output.exists());
    }
    // What ffmpeg says is the error: here, a size it can't take.
    if let Some(encoder) = ffmpeg.encoders.encoder(Format::Ffv1) {
        let output = folder.join("nonsense.mkv");
        let mut args = ffmpeg_args(encoder, size, 10, &output);
        let at = args.iter().position(|arg| arg == "-s").unwrap();
        args[at + 1] = "0x0".to_owned();
        let mut sink = Sink::video(&ffmpeg.program, &args, &output).unwrap();
        let sent = (0..8).try_for_each(|n| sink.send(frame(n)));
        let why = sent.and_then(|()| sink.finish()).unwrap_err();
        assert!(why.starts_with("ffmpeg"), "{why}");
        assert!(!output.exists());
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// A whole movie through the machine's ffmpeg: skipped without one, or
/// without a graphics adapter.
#[test]
fn a_sequence_renders_to_a_video_file() {
    let (Some(gpu), Some(ffmpeg)) = (test_device(), encode::probe()) else {
        eprintln!("skipped: needs a graphics adapter and ffmpeg");
        return;
    };
    let folder = scratch("video");
    for format in [Format::Ffv1, Format::H264] {
        let Some(encoder) = ffmpeg.encoders.encoder(format) else {
            continue;
        };
        if !matches!(encoder, "libx264" | "libopenh264" | "ffv1") {
            continue;
        }
        let output = with_format(&folder.join("Movie"), format);
        let video = MovieSettings {
            format,
            ..settings(&output)
        };
        let finished = render(&gpu, two_shots(Transition::Cut), &video, Some(&ffmpeg));
        assert_eq!(finished.output, output);
        assert_eq!(finished.frames, 10);
        assert!(std::fs::metadata(&output).unwrap().len() > 500, "{encoder}");
    }
    let _ = std::fs::remove_dir_all(&folder);
}
