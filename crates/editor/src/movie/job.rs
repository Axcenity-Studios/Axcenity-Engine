//! A movie being rendered: [`MovieJob::step`] draws one frame.
//!
//! Each kept frame goes like this, at its own time `start + n / fps`:
//!
//! 1. The sequence puts the job's world where it has it at that time
//!    (`SequenceRun::apply`: keyed properties, clips on Animators, the
//!    overlay with its fades, bars and subtitles, the camera cut).
//! 2. The Animators, the particles and the weather move on by exactly one
//!    frame's time, rain and snow stopping at roofs as in the game.
//! 3. The world is drawn from the cut's camera with the UI over it, read
//!    back and handed to the [`Sink`].
//!
//! Nothing reads a clock, so two runs draw the same frames.
//!
//! **Warm-up.** Ray traced light, light probes, TAA and the textures and
//! meshes on their way to the GPU all take some frames to settle. Before
//! the first kept frame the job draws `warm_up` frames it throws away, the
//! world held at the first frame while the shader clock runs up to it and
//! the particles run ahead (so rain is already falling). A step that finds
//! textures still being read in the background draws nothing and says
//! [`Progress::Loading`].
//!
//! **Camera cuts.** What the renderer keeps from the frame before (TAA's
//! picture, where things were for motion blur) shows somewhere else after
//! a cut. So at a cut the job has it forget (`forget_history`) and draws a
//! few thrown-away frames from the new camera ([`cut_warm_up`]).
//!
//! **Dissolves.** While a cut dissolves in, the sequence gives both shots
//! (`Applied::dissolve`). The job draws the world twice, the outgoing shot
//! with its renderer and the incoming one with a second renderer made for
//! it, and mixes the two pictures ([`blend`]). When the dissolve ends the
//! second renderer carries on as the only one, its history unbroken.

use std::path::PathBuf;
use std::time::Instant;

use axcenity_anim::Animators;
use axcenity_anim::sequence::{OVERLAY_NAME, SequenceFile, SequenceRun, Shot};
use axcenity_assets::Assets;
use axcenity_core::{ClassName, Value, World};
use axcenity_render::{Camera, GraphicsSettings, TextureRenderer, wgpu};
use axcenity_scene::{Particles, TerrainCache};
use axcenity_ui::{Stylesheet, UiInput, UiInteraction};

use super::encode::{Ffmpeg, Sink, blend, ffmpeg_args};
use super::{Format, MovieSettings, cut_warm_up, frame_count, frame_time};

/// How long the particles and the weather run before the first frame, in
/// seconds: enough for rain to reach the ground and smoke to rise.
const PARTICLE_RUN_UP: f32 = 5.0;
/// The time step of that run-up.
const RUN_UP_STEP: f32 = 1.0 / 30.0;
/// The most steps in a row that wait for textures before the job goes on
/// with what it has (a file that never arrives mustn't hang the movie).
const MOST_LOADING_STEPS: u32 = 3000;

/// The picture height the overlay's subtitle size is right for, in pixels.
const SUBTITLE_HEIGHT: f64 = 1080.0;

/// How many steps the particles' run-up takes.
fn run_up_updates() -> u32 {
    (PARTICLE_RUN_UP / RUN_UP_STEP).ceil() as u32
}

/// What a movie is made from.
pub struct MovieSource {
    /// The world as it is before the sequence plays: the job changes its
    /// own copy.
    pub world: World,
    pub sequence: SequenceFile,
    /// The project's files. A clone shares what's already loaded.
    pub assets: Assets,
    pub stylesheets: Vec<Stylesheet>,
    /// The view while no camera cut is in charge (before the first cut,
    /// or a sequence without any): the Scene view's camera.
    pub camera: Camera,
}

/// How far a job has got, from [`MovieJob::step`].
#[derive(Clone, Debug, PartialEq)]
pub enum Progress {
    /// Textures are still being read: nothing was drawn. Step again in a
    /// moment.
    Loading,
    /// A frame was drawn and thrown away; `left` more before the next
    /// kept frame.
    Warming {
        left: u32,
    },
    /// Frame `index` (from 0) of `total` was drawn and handed over.
    Frame {
        index: u32,
        total: u32,
    },
    Finished(Finished),
}

/// A finished movie.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    /// The file written, or the folder of PNGs.
    pub output: PathBuf,
    pub frames: u32,
    pub size: (u32, u32),
    pub fps: u32,
    /// How long rendering took, in seconds.
    pub seconds: f64,
    /// Bindings the sequence couldn't find, each once.
    pub missing: Vec<String>,
}

/// One camera's renderer, with what it keeps between frames.
struct View {
    renderer: TextureRenderer,
    terrain: TerrainCache,
    /// Thrown-away frames to draw before its next kept one.
    warm: u32,
}

impl View {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        graphics: &GraphicsSettings,
        warm: u32,
    ) -> View {
        let mut renderer = TextureRenderer::new(device, queue, size.0, size.1);
        renderer.set_graphics(graphics);
        View {
            renderer,
            terrain: TerrainCache::new(),
            warm,
        }
    }
}

/// Which shot a view draws: a cut's place in the sequence's `camera_cuts`,
/// or `None` for the host's own camera.
type ShotId = Option<usize>;

pub struct MovieJob {
    device: wgpu::Device,
    queue: wgpu::Queue,
    graphics: GraphicsSettings,
    size: (u32, u32),
    fps: u32,
    start: f64,
    total: u32,
    warm_up: u32,
    motion_blur: Option<f32>,
    output: PathBuf,

    world: World,
    run: SequenceRun,
    assets: Assets,
    stylesheets: Vec<Stylesheet>,
    fallback: Camera,
    animators: Animators,
    particles: Particles,
    ui: UiInteraction,

    /// The shot in charge, or the outgoing one while a cut dissolves in.
    main: View,
    main_shot: Option<ShotId>,
    /// The incoming shot of a dissolve.
    incoming: Option<(usize, View)>,

    sink: Option<Sink>,
    /// The next frame to keep.
    frame: u32,
    /// The frame the world was last moved on to.
    posed: Option<u32>,
    /// What the sequence gave for that frame.
    shots: Shots,
    loading_steps: u32,
    /// Whether the overlay's subtitle has been sized for the picture.
    subtitle_sized: bool,
    started: Instant,
    last: Vec<u8>,
}

/// The views of one frame.
#[derive(Clone, Copy, Default)]
struct Shots {
    main: (ShotId, Option<Shot>),
    /// The dissolve's incoming shot and its share of the picture.
    incoming: Option<(usize, Option<Shot>, f32)>,
}

impl MovieJob {
    /// Starts a movie of `source` with `settings` on a GPU device. `ffmpeg`
    /// is the one found on the machine, needed for every format but a PNG
    /// sequence. Nothing is drawn until [`step`](Self::step).
    pub fn start(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: MovieSource,
        settings: &MovieSettings,
        ffmpeg: Option<&Ffmpeg>,
    ) -> Result<MovieJob, String> {
        let MovieSource {
            mut world,
            mut sequence,
            assets,
            stylesheets,
            camera,
        } = source;
        if settings.output.as_os_str().is_empty() {
            return Err("Choose where to save the movie.".to_owned());
        }
        if !settings.letterbox {
            sequence.letterbox.clear();
        }
        if !settings.game_ui {
            // The sequence's own overlay is made later, so it stays.
            for gui in world.all_below(world.root(), |class| class == ClassName::ScreenGui) {
                let _ = world.set_property(gui, "Enabled", Value::Bool(false));
            }
        }
        let (start, end) = settings.span(sequence.duration);
        let fps = settings.fps.max(1);
        let total = frame_count(start, end, fps);
        // With the project's own environment presets, for the sequence's
        // Environment track.
        let run = SequenceRun::new(sequence)
            .map_err(|err| err.0)?
            .with_presets(&assets);

        // What the device can do says how big the picture may be and
        // whether rays are traced (as a `TextureRenderer` reads it).
        let capabilities = axcenity_render::read_capabilities(device, None);
        let size = settings.size(capabilities.max_texture_size);
        let graphics = settings.graphics(&capabilities);
        let main = View::new(device, queue, size, &graphics, settings.warm_up);

        let sink = match settings.format {
            Format::Png => {
                let stem = settings
                    .output
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "frame".to_owned());
                Sink::png(&settings.output, &stem, size)?
            }
            format => {
                let ffmpeg = ffmpeg.ok_or(
                    "A video needs ffmpeg, which wasn't found on this machine. Render a PNG \
                     image sequence, or install ffmpeg.",
                )?;
                let encoder = ffmpeg.encoders.encoder(format).ok_or_else(|| {
                    format!(
                        "This machine's ffmpeg has no encoder for {}.",
                        format.label()
                    )
                })?;
                let args = ffmpeg_args(encoder, size, fps, &settings.output);
                Sink::video(&ffmpeg.program, &args, &settings.output)?
            }
        };

        Ok(MovieJob {
            device: device.clone(),
            queue: queue.clone(),
            graphics,
            size,
            fps,
            start,
            total,
            warm_up: settings.warm_up,
            motion_blur: settings.motion_blur,
            output: settings.output.clone(),
            world,
            run,
            assets,
            stylesheets,
            fallback: camera,
            animators: Animators::new(),
            particles: Particles::new(),
            ui: UiInteraction::default(),
            main,
            main_shot: None,
            incoming: None,
            sink: Some(sink),
            frame: 0,
            posed: None,
            shots: Shots::default(),
            loading_steps: 0,
            subtitle_sized: false,
            started: Instant::now(),
            last: Vec::new(),
        })
    }

    /// Frames kept so far, and how many there will be.
    pub fn frames(&self) -> (u32, u32) {
        (self.frame, self.total)
    }

    /// The picture's size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn fps(&self) -> u32 {
        self.fps
    }

    /// Seconds since the job started.
    pub fn elapsed(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Thrown-away frames still to draw before the next kept one.
    pub fn warming(&self) -> u32 {
        self.main.warm + self.incoming.as_ref().map_or(0, |(_, view)| view.warm)
    }

    /// The last frame kept, as rows of RGBA bytes ([`size`](Self::size)).
    /// Empty before the first.
    pub fn last_frame(&self) -> &[u8] {
        &self.last
    }

    /// The graphics values the movie is drawn with, and why any were
    /// lowered.
    pub fn graphics(&self) -> &axcenity_render::EffectiveGraphics {
        self.main.renderer.graphics()
    }

    /// Stops the movie; a half-written video file is taken away.
    pub fn cancel(mut self) {
        if let Some(sink) = self.sink.take() {
            sink.cancel();
        }
    }

    /// Draws one frame: a thrown-away one while warming up, else the next
    /// kept one (two views of it while a cut dissolves in). An error ends
    /// the movie.
    pub fn step(&mut self) -> Result<Progress, String> {
        if self.frame >= self.total {
            return self.finish();
        }
        let step = 1.0 / self.fps as f32;
        let time = frame_time(self.start, self.fps, self.frame);
        if self.posed != Some(self.frame) {
            self.pose(time, if self.frame == 0 { 0.0 } else { step });
            self.posed = Some(self.frame);
        }

        // Textures on their way: wait, drawing nothing, so no frame is
        // kept with a surface still blank.
        if self.assets.texture_progress().is_some() && self.loading_steps < MOST_LOADING_STEPS {
            self.loading_steps += 1;
            // Asking for this frame's textures is what starts them loading.
            self.request_assets();
            return Ok(Progress::Loading);
        }

        let warming = self.warming();
        if warming > 0 {
            self.warm_frame(time, step, warming)?;
            return Ok(Progress::Warming { left: warming - 1 });
        }
        self.loading_steps = 0;

        let shots = self.shots;
        let clock = time as f32;
        let (_, main_shot) = shots.main;
        let mut picture = self.draw(None, main_shot, clock, step, true)?;
        if let Some((_, shot, weight)) = shots.incoming {
            let incoming = self.draw(Some(()), shot, clock, 0.0, true)?;
            picture = blend(&picture, &incoming, weight);
        }
        self.last.clone_from(&picture);
        match &mut self.sink {
            Some(sink) => sink.send(picture)?,
            None => return Err("the movie has already ended".to_owned()),
        }
        let index = self.frame;
        self.frame += 1;
        Ok(Progress::Frame {
            index,
            total: self.total,
        })
    }

    fn finish(&mut self) -> Result<Progress, String> {
        if let Some(sink) = self.sink.take() {
            sink.finish()?;
        }
        Ok(Progress::Finished(Finished {
            output: self.output.clone(),
            frames: self.total,
            size: self.size,
            fps: self.fps,
            seconds: self.elapsed(),
            missing: self.run.missing.iter().cloned().collect(),
        }))
    }

    /// Moves the world on to `time`, `seconds` after the frame before, and
    /// works out which shots this frame draws and with which renderer.
    fn pose(&mut self, time: f64, seconds: f32) {
        let applied = self.run.apply(&mut self.world, time);
        self.size_subtitle();
        self.animators
            .update(&mut self.world, &self.assets, f64::from(seconds));
        self.move_particles(seconds);
        // With no warm-up frames to spread it over, the particles' run-up
        // is done at once.
        if self.frame == 0 && self.warm_up == 0 {
            for _ in 0..run_up_updates() {
                self.move_particles(RUN_UP_STEP);
            }
        }

        let shots = match applied.dissolve {
            Some(dissolve) => {
                let incoming = self
                    .run
                    .file
                    .dissolve_at(time)
                    .map_or(0, |(index, _)| index);
                Shots {
                    main: (incoming.checked_sub(1), dissolve.from),
                    incoming: Some((incoming, dissolve.to, dissolve.weight)),
                }
            }
            None => Shots {
                main: (self.run.file.cut_index_at(time), applied.camera),
                incoming: None,
            },
        };
        self.shots = shots;

        // The main view's shot changed: the dissolve's second renderer
        // takes over if it was drawing that shot, else it's a cut.
        let (wanted, _) = shots.main;
        if self.main_shot != Some(wanted) {
            let taken_over = match self.incoming.take() {
                Some((index, view)) if Some(index) == wanted => {
                    self.main = view;
                    true
                }
                _ => false,
            };
            if !taken_over && self.main_shot.is_some() {
                self.main.renderer.forget_history();
                self.main.warm = self.main.warm.max(cut_warm_up(self.warm_up));
            }
            self.main_shot = Some(wanted);
        }
        match shots.incoming {
            Some((index, _, _)) => {
                if self.incoming.as_ref().is_none_or(|(own, _)| *own != index) {
                    let warm = cut_warm_up(self.warm_up);
                    let view =
                        View::new(&self.device, &self.queue, self.size, &self.graphics, warm);
                    self.incoming = Some((index, view));
                }
            }
            None => self.incoming = None,
        }
    }

    /// The overlay's subtitle is a size in pixels, right on a picture 1080
    /// pixels tall. A movie keeps it that share of the picture at any
    /// size, so a 4K movie's subtitles aren't half as big as a Full HD
    /// one's. Done once, when the sequence has made its overlay.
    fn size_subtitle(&mut self) {
        if self.subtitle_sized {
            return;
        }
        let label = self
            .world
            .find_first_child(self.world.workspace(), OVERLAY_NAME)
            .and_then(|overlay| self.world.find_first_child(overlay, "Subtitle"));
        let Some(label) = label else {
            return;
        };
        self.subtitle_sized = true;
        let size = self
            .world
            .get(label)
            .and_then(|label| label.property("TextSize"))
            .and_then(Value::as_number);
        if let Some(size) = size {
            let scaled = (size * f64::from(self.size.1) / SUBTITLE_HEIGHT)
                .round()
                .max(1.0);
            let _ = self
                .world
                .set_property(label, "TextSize", Value::Number(scaled));
        }
    }

    /// The particles and the weather move on by `seconds`, rain, snow and
    /// colliding particles landing on what's in the world.
    fn move_particles(&mut self, seconds: f32) {
        let (world, assets) = (&self.world, &self.assets);
        self.particles
            .update_against(world, seconds, &mut |skip, paths| {
                crate::play::particle_surfaces(world, assets, skip, paths)
            });
    }

    /// Draws one frame to throw away, for each view still warming. Before
    /// the movie's first frame the particles run ahead and the shader
    /// clock runs up to the frame; at a cut later on both are held, so
    /// what the frame shows doesn't move.
    fn warm_frame(&mut self, time: f64, step: f32, left: u32) -> Result<(), String> {
        let first = self.frame == 0;
        let clock = if first {
            (time - f64::from(left) * f64::from(step)) as f32
        } else {
            time as f32
        };
        if first && self.main.warm > 0 {
            // The run-up, spread over the warm-up frames so no single
            // step takes long.
            for _ in 0..run_up_updates().div_ceil(self.warm_up.max(1)) {
                self.move_particles(RUN_UP_STEP);
            }
        }
        let shots = self.shots;
        if self.main.warm > 0 {
            self.draw(None, shots.main.1, clock, 0.0, false)?;
            self.main.warm -= 1;
        } else if let Some((_, shot, _)) = shots.incoming {
            self.draw(Some(()), shot, clock, 0.0, false)?;
            if let Some((_, view)) = &mut self.incoming {
                view.warm = view.warm.saturating_sub(1);
            }
        }
        Ok(())
    }

    /// Builds this frame's draw list once without drawing it, which asks
    /// `Assets` for every texture and model it shows.
    fn request_assets(&mut self) {
        let camera = self.camera_of(self.shots.main.1);
        let _ = axcenity_scene::build_draw_list_posed(
            &self.world,
            &self.assets,
            camera,
            self.animators.poses(),
            &mut self.main.terrain,
        );
    }

    fn camera_of(&self, shot: Option<Shot>) -> Camera {
        match shot {
            Some(shot) => axcenity_script::CameraView {
                position: shot.position,
                orientation: shot.orientation,
                field_of_view: shot.field_of_view,
            }
            .render_camera(),
            None => self.fallback,
        }
    }

    /// Draws the world from `shot` with the main view's renderer, or the
    /// incoming one's for `Some(())`, the UI moved on by `ui_seconds`.
    /// Returns the picture when it's `kept` (reading it back is slow).
    fn draw(
        &mut self,
        incoming: Option<()>,
        shot: Option<Shot>,
        clock: f32,
        ui_seconds: f32,
        kept: bool,
    ) -> Result<Vec<u8>, String> {
        let camera = self.camera_of(shot);
        let view = match (incoming, &mut self.incoming) {
            (Some(()), Some((_, view))) => view,
            _ => &mut self.main,
        };
        let mut list = axcenity_scene::build_draw_list_posed(
            &self.world,
            &self.assets,
            camera,
            self.animators.poses(),
            &mut view.terrain,
        );
        let layout = self.ui.layout(
            &self.world,
            self.size,
            &self.stylesheets,
            UiInput::default(),
            Some(&camera),
            ui_seconds,
        );
        list.ui = layout.draw_items;
        axcenity_scene::attach_ui_textures(&mut list.ui, &self.assets);
        self.particles.draw(&mut list, &self.world, &self.assets);
        self.particles.add_weather(&mut list);
        list.time = clock;
        if let Some(strength) = self.motion_blur {
            list.lens.motion_blur = (strength > 0.0).then_some(strength.min(1.0));
        }
        view.renderer
            .render(&list)
            .map_err(|err| format!("could not draw a frame: {err:#}"))?;
        // Wait for the GPU: frames are made one at a time, in order.
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        Ok(if kept {
            view.renderer.read_pixels()
        } else {
            Vec::new()
        })
    }
}
