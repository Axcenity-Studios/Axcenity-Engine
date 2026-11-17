//! Pictures of characters and presets for the start page: the workspace's
//! own preview (the Character Runtime's applied description on the
//! default character, idling, on the stage) rendered offscreen by the
//! scene renderer.
//!
//! They're cached as PNGs in the workspace's folder, `thumbnails/`:
//! `characters/<file>.<key>.png` for a saved character and
//! `presets/<preset ID>.<key>.png` for a preset (its ID with `/` as `-`),
//! where the key is a hash of what's drawn (the file's text, or the
//! character the preset makes), the picture's version and the engine's.
//! The Presets page's tiles in the workspace come from the same cache. A changed file has a new key, so it's drawn
//! again; the older pictures of that file are removed when the new one is
//! written. Deleting a character removes its pictures.
//!
//! [`Thumbnails`] draws missing ones on a thread of its own, on the
//! editor's GPU device, one at a time, so the start page never waits: a
//! card shows a placeholder until its picture is there. The thread ends
//! when it has had nothing to do for a while.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use axcenity_avatar::{Description, Library, PresetKind};
use axcenity_render::{TextureRenderer, wgpu};
use eframe::egui;

use super::stage::{self, Backdrop, Lighting, Orbit, ViewPreset};
use super::{CharacterWorkspace, presets};

/// The pictures' folder, in the workspace's folder.
pub const THUMBNAILS_FOLDER: &str = "thumbnails";
/// A picture's size in pixels: portrait, 4 by 5.
pub const PICTURE_SIZE: (u32, u32) = (360, 450);
/// Bump to draw every picture again (the framing or stage changed).
const PICTURE_VERSION: u32 = 1;
/// The thread stops after this long with nothing to draw.
const IDLE: Duration = Duration::from_secs(20);
/// At most this many pictures are read from disk in one frame.
const LOADS_PER_FRAME: usize = 4;

/// A 64-bit FNV-1a hash: stable across builds and Rust versions, unlike
/// the standard library's hasher, so the cache keeps its keys.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The cache key of a picture of `what`.
fn key(what: &str) -> String {
    let keyed = format!("{PICTURE_VERSION}\n{}\n{what}", env!("CARGO_PKG_VERSION"));
    format!("{:016x}", fnv1a(keyed.as_bytes()))
}

/// A saved character's file name without `.character.json`.
fn stem(file: &Path) -> Option<&str> {
    file.file_name()?.to_str()?.strip_suffix(".character.json")
}

/// Where the picture of the saved character `file`, whose text is `text`,
/// is cached.
pub fn character_picture(folder: &Path, file: &Path, text: &str) -> Option<PathBuf> {
    let stem = stem(file)?;
    Some(
        folder
            .join(THUMBNAILS_FOLDER)
            .join("characters")
            .join(format!("{stem}.{}.png", key(text))),
    )
}

/// Where the picture of preset `id`, showing `subject`, is cached.
pub fn preset_picture(folder: &Path, id: &str, subject: &Subject) -> PathBuf {
    let stem: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let (framing, drawn) = match subject {
        Subject::Character(description) => ("whole", description),
        Subject::Body(description) => ("body", description),
        Subject::Face(description) => ("face", description),
    };
    folder.join(THUMBNAILS_FOLDER).join("presets").join(format!(
        "{stem}.{}.png",
        key(&format!("{framing}\n{}", drawn.to_json()))
    ))
}

/// What the picture of `listed` shows: the character it makes, a face
/// preset's framed on the face, a body preset's on the body.
pub fn preset_subject(listed: &presets::Listed, library: &Library) -> Subject {
    let shown = listed.pictured(library);
    match listed.info.kind {
        PresetKind::Face => Subject::Face(shown),
        PresetKind::Body => Subject::Body(shown),
        _ => Subject::Character(shown),
    }
}

/// Whether `name` is a picture of `stem` (`<stem>.<16 hex digits>.png`).
fn picture_of(name: &str, stem: &str) -> bool {
    name.strip_suffix(".png")
        .and_then(|rest| rest.rsplit_once('.'))
        .is_some_and(|(of, key)| {
            of == stem && key.len() == 16 && key.chars().all(|c| c.is_ascii_hexdigit())
        })
}

/// Removes the pictures in `picture`'s folder of the same thing, but for
/// `picture` itself.
fn prune(picture: &Path) {
    let (Some(dir), Some(name)) = (
        picture.parent(),
        picture.file_name().and_then(|name| name.to_str()),
    ) else {
        return;
    };
    let Some(stem) = name
        .strip_suffix(".png")
        .and_then(|rest| rest.rsplit_once('.'))
        .map(|(stem, _)| stem)
    else {
        return;
    };
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let other = entry.file_name();
        let other = other.to_string_lossy();
        if other != name && picture_of(&other, stem) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Removes every cached picture of the saved character `file`.
pub fn remove_character_pictures(folder: &Path, file: &Path) {
    let Some(stem) = stem(file) else {
        return;
    };
    let dir = folder.join(THUMBNAILS_FOLDER).join("characters");
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if picture_of(&entry.file_name().to_string_lossy(), stem) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// What a picture shows.
#[derive(Clone, Debug)]
pub enum Subject {
    /// A character, the whole body.
    Character(Description),
    /// A body preset's character, framed on the body ([`body_orbit`]).
    Body(Description),
    /// A character's face, close up.
    Face(Description),
}

/// A picture to draw, and where it goes.
#[derive(Clone, Debug)]
pub struct Job {
    pub subject: Subject,
    pub out: PathBuf,
}

/// How a picture frames the character: the whole body, a little turned,
/// filling the portrait.
pub fn picture_orbit() -> Orbit {
    Orbit {
        yaw: 22.0,
        pitch: 5.0,
        distance: 10.3,
        target: axcenity_render::glam::Vec3::new(0.0, 2.5, 0.0),
    }
}

/// How a body preset's picture frames the character: from the front and a
/// little turned, neck to knees, so the build fills the tile's width (a
/// whole figure is small in a tall picture, and slim, average and athletic
/// looked alike at a tile's size). The crop is below the chin: a body
/// preset doesn't change the face, and a frame cut through the mouth read
/// badly.
pub fn body_orbit() -> Orbit {
    Orbit {
        yaw: 15.0,
        pitch: 4.0,
        distance: 5.4,
        target: axcenity_render::glam::Vec3::new(0.0, 2.8, 0.0),
    }
}

/// How a face picture frames the character: the face, a little turned.
pub fn face_orbit() -> Orbit {
    let mut orbit = ViewPreset::Face.orbit();
    orbit.yaw = 18.0;
    orbit.distance *= 1.15;
    orbit
}

/// Draws pictures: a workspace of its own (the preview World and the
/// library) and a renderer the picture's size.
pub struct Painter {
    workspace: CharacterWorkspace,
    renderer: TextureRenderer,
}

impl Painter {
    /// A painter for the workspace in `folder`, on `device`.
    pub fn new(folder: &Path, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, String> {
        let mut workspace = CharacterWorkspace::open(
            folder,
            Description::new("Picture", super::DEFAULT_BASE),
            true,
        )?;
        // It shows characters, never edits one: no recovery file.
        workspace.autosave.on = false;
        let (width, height) = PICTURE_SIZE;
        Ok(Self {
            workspace,
            renderer: TextureRenderer::new(device, queue, width, height),
        })
    }

    /// The picture of `subject` as rows of RGBA bytes, [`PICTURE_SIZE`].
    pub fn paint(&mut self, subject: &Subject) -> Vec<u8> {
        let (description, orbit) = match subject {
            Subject::Character(description) => (description.clone(), picture_orbit()),
            Subject::Body(description) => (description.clone(), body_orbit()),
            Subject::Face(description) => (description.clone(), face_orbit()),
        };
        let workspace = &mut self.workspace;
        workspace.show_only(description);
        for _ in 0..10 {
            workspace.step_by(1.0 / 30.0);
        }
        stage::apply(&mut workspace.world, Lighting::Studio, Backdrop::Grey);
        let mut list = axcenity_scene::build_draw_list_posed(
            &workspace.world,
            &workspace.assets,
            orbit.camera(),
            workspace.poses(),
            &mut axcenity_scene::TerrainCache::new(),
        );
        stage::backdrop(&mut list, Backdrop::Grey);
        // Models load the first time they're drawn.
        for _ in 0..2 {
            if let Err(error) = self.renderer.render(&list) {
                log::warn!("could not draw a character's picture: {error:#}");
            }
        }
        self.renderer.take_shader_errors();
        self.renderer.read_pixels()
    }

    /// Draws `job`'s picture and writes it, removing older pictures of the
    /// same thing.
    pub fn paint_to(&mut self, job: &Job) -> Result<(), String> {
        let pixels = self.paint(&job.subject);
        write_png(&job.out, PICTURE_SIZE, &pixels)?;
        prune(&job.out);
        Ok(())
    }
}

/// Writes RGBA `pixels` as a PNG at `path`, whole or not at all.
fn write_png(path: &Path, (width, height): (u32, u32), pixels: &[u8]) -> Result<(), String> {
    let dir = path.parent().ok_or("no folder")?;
    std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(pixels))
        .map_err(|err| err.to_string())?;
    let partial = path.with_extension("png.partial");
    std::fs::write(&partial, bytes).map_err(|err| err.to_string())?;
    std::fs::rename(&partial, path).map_err(|err| err.to_string())
}

/// What the thread sends back: the picture written, or why not.
type Done = (PathBuf, Result<(), String>);

struct Worker {
    jobs: mpsc::Sender<Job>,
    done: mpsc::Receiver<Done>,
    thread: std::thread::JoinHandle<()>,
}

/// The start page's pictures: which are being drawn, and those loaded.
#[derive(Default)]
pub struct Thumbnails {
    folder: Option<PathBuf>,
    gpu: Option<(wgpu::Device, wgpu::Queue)>,
    repaint: Option<egui::Context>,
    worker: Option<Worker>,
    /// Pictures asked for and not yet written, with how many threads were
    /// given them (a thread that ended without drawing one gets it again
    /// once; then it's given up on).
    pending: HashMap<PathBuf, (Job, u32)>,
    failed: HashSet<PathBuf>,
    textures: HashMap<PathBuf, egui::TextureHandle>,
}

impl Thumbnails {
    /// The workspace's folder, where the pictures are kept.
    pub fn set_folder(&mut self, folder: Option<PathBuf>) {
        if self.folder != folder {
            *self = Self {
                gpu: self.gpu.take(),
                repaint: self.repaint.take(),
                folder,
                ..Default::default()
            };
        }
    }

    /// The GPU device pictures are drawn on (the editor's).
    pub fn set_gpu(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.gpu.is_none() {
            self.gpu = Some((device.clone(), queue.clone()));
        }
    }

    /// Asks for a picture of `subject` at `out` unless it's there already.
    pub fn request(&mut self, subject: Subject, out: PathBuf) {
        if out.is_file() || self.pending.contains_key(&out) || self.failed.contains(&out) {
            return;
        }
        let job = Job { subject, out };
        self.pending.insert(job.out.clone(), (job.clone(), 0));
        self.send(job);
    }

    /// Gives `job` to the thread, starting one if there's none.
    fn send(&mut self, job: Job) {
        if let Some(worker) = &self.worker
            && !worker.thread.is_finished()
            && worker.jobs.send(job.clone()).is_ok()
        {
            if let Some((_, given)) = self.pending.get_mut(&job.out) {
                *given += 1;
            }
            return;
        }
        self.start_thread();
    }

    /// Starts a thread and gives it every pending picture. Without a GPU
    /// (yet) the pictures wait.
    fn start_thread(&mut self) {
        self.worker = None;
        let (Some(folder), Some((device, queue))) = (self.folder.clone(), self.gpu.clone()) else {
            return;
        };
        let (jobs, take) = mpsc::channel::<Job>();
        let (give, done) = mpsc::channel::<Done>();
        let repaint = self.repaint.clone();
        let spawned = std::thread::Builder::new()
            .name("character pictures".into())
            .spawn(move || draw_pictures(&folder, &device, &queue, &take, &give, repaint));
        let Ok(thread) = spawned else {
            return;
        };
        for (pending, given) in self.pending.values_mut() {
            *given += 1;
            let _ = jobs.send(pending.clone());
        }
        self.worker = Some(Worker { jobs, done, thread });
    }

    /// Takes in what the thread finished; starts another for pictures
    /// still to draw if it has ended. Call each frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        if self.repaint.is_none() {
            self.repaint = Some(ctx.clone());
        }
        if let Some(worker) = &self.worker {
            while let Ok((out, result)) = worker.done.try_recv() {
                self.pending.remove(&out);
                self.textures.remove(&out);
                if let Err(error) = result {
                    log::warn!("could not draw {}: {error}", out.display());
                    self.failed.insert(out);
                }
            }
        }
        let ended = self
            .worker
            .as_ref()
            .is_none_or(|worker| worker.thread.is_finished());
        if ended && !self.pending.is_empty() {
            self.pending.retain(|out, (_, given)| {
                let keep = *given < 2;
                if !keep {
                    self.failed.insert(out.clone());
                }
                keep
            });
            if !self.pending.is_empty() {
                self.start_thread();
            }
        }
    }

    /// Whether the picture at `out` is still being drawn.
    pub fn pending(&self, out: &Path) -> bool {
        self.pending.contains_key(out)
    }

    /// The picture at `out` as a texture, once it's written (a few are read
    /// from disk each frame).
    pub fn texture(&mut self, ctx: &egui::Context, out: &Path) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.textures.get(out) {
            return Some(texture.clone());
        }
        if self.pending.contains_key(out) || !out.is_file() {
            return None;
        }
        // (No `ctx` calls inside `data_mut`: its lock isn't re-entrant.)
        let frame = ctx.cumulative_frame_nr();
        let budget = ctx.data_mut(|data| {
            let id = egui::Id::new("character pictures read");
            let (at, used) = data.get_temp::<(u64, usize)>(id).unwrap_or((frame, 0));
            let used = if at == frame { used } else { 0 };
            data.insert_temp(id, (frame, used + 1));
            used < LOADS_PER_FRAME
        });
        if !budget {
            ctx.request_repaint();
            return None;
        }
        let image = std::fs::read(out)
            .ok()
            .and_then(|bytes| image::load_from_memory(&bytes).ok())?
            .to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let texture = ctx.load_texture(
            out.to_string_lossy(),
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        self.textures.insert(out.to_owned(), texture.clone());
        Some(texture)
    }

    /// Drops the loaded picture at `out` (its file was removed).
    pub fn forget(&mut self, out: &Path) {
        self.textures.remove(out);
        self.failed.remove(out);
    }
}

impl CharacterWorkspace {
    /// Asks for the pictures of the presets of the character's base model
    /// that aren't drawn yet, and loads those that are (the Presets page's
    /// tiles; a few a frame).
    pub(super) fn load_preset_pictures(&mut self, ctx: &egui::Context, library: &Library) {
        let wanted: Vec<(String, Subject)> = presets::all(library, &self.own_presets)
            .iter()
            .filter(|listed| listed.preset.base == self.description.base)
            .filter(|listed| !self.preset_pictures.contains_key(listed.id))
            .map(|listed| (listed.id.to_owned(), preset_subject(listed, library)))
            .collect();
        for (id, subject) in wanted {
            let out = preset_picture(&self.folder, &id, &subject);
            self.pictures.request(subject, out.clone());
            self.preset_pictures.insert(id, (out, None));
        }
        for (out, texture) in self.preset_pictures.values_mut() {
            if texture.is_none() {
                *texture = self.pictures.texture(ctx, out);
            }
        }
    }

    /// The picture of preset `id`, once it's drawn and loaded.
    pub(super) fn preset_texture(&self, id: &str) -> Option<&egui::TextureHandle> {
        self.preset_pictures.get(id)?.1.as_ref()
    }
}

/// The thread: draws each job it's given, with one painter, until it has
/// had nothing to do for [`IDLE`] or the page is gone.
fn draw_pictures(
    folder: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    jobs: &mpsc::Receiver<Job>,
    done: &mpsc::Sender<Done>,
    repaint: Option<egui::Context>,
) {
    let mut painter: Option<Painter> = None;
    while let Ok(job) = jobs.recv_timeout(IDLE) {
        let result = match &mut painter {
            Some(painter) => Ok(painter),
            None => Painter::new(folder, device, queue).map(|made| painter.insert(made)),
        }
        .and_then(|painter| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| painter.paint_to(&job)))
                .unwrap_or_else(|_| Err("drawing it failed".to_owned()))
        });
        if result.is_err() {
            // Start afresh for the next one.
            painter = None;
        }
        if done.send((job.out, result)).is_err() {
            return;
        }
        if let Some(ctx) = &repaint {
            ctx.request_repaint();
        }
    }
}
