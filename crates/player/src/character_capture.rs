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

use std::path::{Path, PathBuf};

use axcenity_avatar::{Description, Library};
use axcenity_core::{Color3, PartType, Value, Vector3};
use axcenity_render::glam::Vec3;
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
}

impl Stance {
    fn label(self) -> String {
        match self {
            Stance::Rest => "rest".to_owned(),
            Stance::Idle => "idle".to_owned(),
            Stance::Clip { name, frames } => format!("{name}@{:.2}", f64::from(frames) / 60.0),
        }
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
fn stage_world(
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

/// Draws `description` from every view under every light: `LIGHTS.len() ×
/// VIEWS.len()` pictures, lights outermost.
pub(crate) fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    assets: &Assets,
    description: &Description,
    settings: Settings,
) -> Result<Vec<Shot>, String> {
    let (mut world, animators) = stage_world(assets, description, settings.stance, settings.face)?;
    let (width, height) = settings.size;
    let mut graphics = GraphicsSettings::preset(Quality::High);
    graphics.anti_aliasing = Some(AntiAliasing::Taa);
    let mut shots = Vec::with_capacity(LIGHTS.len() * VIEWS.len());
    for light in &LIGHTS {
        let lighting = world.lighting();
        let _ = world.set_property(lighting, "ClockTime", Value::Number(light.clock_time));
        let ambient = light.ambient;
        let _ = world.set_property(
            lighting,
            "Ambient",
            Value::Color3(Color3::new(ambient, ambient, ambient * 1.05)),
        );
        if let Some(floor) = world.find_first_child(world.workspace(), "Floor") {
            let [r, g, b] = light.floor;
            let _ = world.set_property(floor, "Color", Value::Color3(Color3::new(r, g, b)));
        }
        for view in &VIEWS {
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

/// Every view under every light on one picture at half size: a row per
/// light, a column per view. Returns its width, height and RGBA pixels.
pub(crate) fn contact_sheet(shots: &[Shot], size: (u32, u32)) -> (u32, u32, Vec<u8>) {
    let (width, height) = (size.0 as usize, size.1 as usize);
    let (cell_w, cell_h) = (width / 2, height / 2);
    let (columns, rows) = (VIEWS.len(), LIGHTS.len());
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
    match std::env::var("AXCENITY_CAPTURE_POSE").as_deref() {
        Ok("idle") => settings.stance = Stance::Idle,
        Ok(pose) if pose != "rest" => {
            let (name, seconds) = pose.split_once('@').unwrap_or((pose, "0.5"));
            let seconds: f64 = seconds
                .parse()
                .expect("AXCENITY_CAPTURE_POSE is `rest`, `idle` or `Clip@seconds`");
            settings.stance = Stance::Clip {
                // Kept for the run: a capture is one process.
                name: Box::leak(name.to_owned().into_boxed_str()),
                frames: (seconds * 60.0).round() as u32,
            };
        }
        _ => {}
    }
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
        let shots = match capture(&device, &queue, &stage.assets, &description, settings) {
            Ok(shots) => shots,
            Err(why) => {
                eprintln!("{name}: not captured: {why}");
                continue;
            }
        };
        let (width, height) = settings.size;
        if !sheets_only {
            for shot in &shots {
                let path = out
                    .join(&name)
                    .join(format!("{}-{}.png", shot.light, shot.view));
                save_png(&path, width, height, &shot.pixels).unwrap();
            }
        }
        let (sheet_w, sheet_h, sheet) = contact_sheet(&shots, settings.size);
        save_png(&out.join(format!("{name}.png")), sheet_w, sheet_h, &sheet).unwrap();
        println!(
            "{name}: {} pictures in {:.1} s",
            shots.len(),
            started.elapsed().as_secs_f64()
        );
        // The levels of detail by view (the lights don't change them).
        let levels: serde_json::Map<String, serde_json::Value> = shots
            .iter()
            .take(VIEWS.len())
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
    let record = serde_json::json!({
        "size": [settings.size.0, settings.size.1],
        "stance": settings.stance.label(),
        "face": settings.face.iter().map(|(name, strength)| format!("{name}:{strength}")).collect::<Vec<_>>(),
        "frames": settings.frames,
        "quality": "High",
        "anti_aliasing": "TAA",
        "lights": LIGHTS.iter().map(|light| light.name).collect::<Vec<_>>(),
        "views": VIEWS.iter().map(|view| view.name).collect::<Vec<_>>(),
        "characters": listed,
    });
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
