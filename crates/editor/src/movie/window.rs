//! The Render Movie window: what to render, the picture, the quality, where
//! it goes, then Render and its progress. It draws and returns what was
//! asked for ([`MovieAction`]); `app/movie.rs` starts and steps the job.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use axcenity_render::{Capabilities, Quality};
use eframe::egui;

use super::encode::{self, Ffmpeg};
use super::{
    Aspect, FRAME_RATES, Finished, Format, MAX_SIDE, MIN_SIDE, MovieJob, MovieSettings, Resolution,
    clock, default_output, frame_count, with_format,
};
use crate::theme::{self, icons};
use crate::widgets::{self, Role};

/// How big the window opens (it can be resized; its content scrolls).
const WINDOW_SIZE: egui::Vec2 = egui::vec2(500.0, 740.0);
/// The widest the frame shown while rendering is, in pixels.
const PREVIEW_WIDTH: usize = 400;
/// Seconds between two frames shown while rendering.
const PREVIEW_EVERY: f64 = 0.4;

/// What the window is shown with.
pub struct MovieScene<'a> {
    /// The open sequence's name, if one is open.
    pub sequence: Option<&'a str>,
    /// Its length in seconds.
    pub duration: f64,
    /// The Sequencer's play range (In and Out), if one is marked.
    pub range: Option<(f64, f64)>,
    /// The open project's folder.
    pub project: Option<&'a Path>,
    /// What the editor's GPU device can do; `None` before it has drawn.
    pub capabilities: Option<&'a Capabilities>,
    /// The game is running in the editor (Play): no movie then.
    pub playing: bool,
}

/// What the window asks the editor for.
#[derive(Clone, Debug, PartialEq)]
pub enum MovieAction {
    Render,
    Cancel,
    /// Pick the output with the system's Save dialog.
    Browse,
    /// Show this folder in the system's file manager.
    OpenFolder(PathBuf),
}

/// The `ffmpeg` on the machine, looked for on a thread.
#[derive(Default)]
enum Probe {
    #[default]
    NotAsked,
    Asking(Receiver<Option<Ffmpeg>>),
    Known(Option<Ffmpeg>),
}

#[derive(Default)]
pub struct MovieWindow {
    pub open: bool,
    pub settings: MovieSettings,
    /// Render the play range, not the whole sequence, while one is marked.
    pub use_range: bool,
    /// The output was typed or picked: it no longer follows the sequence's
    /// name and the format.
    output_set: bool,
    /// The quality was picked: it no longer follows the machine's best.
    quality_set: bool,
    /// The sequence and project the output was last worked out for.
    made_for: Option<(String, PathBuf)>,
    probe: Probe,
    pub job: Option<MovieJob>,
    pub done: Option<Finished>,
    pub error: Option<String>,
    /// The last frame shown, and when (the job's elapsed seconds).
    preview: Option<(egui::TextureHandle, f64)>,
}

impl MovieWindow {
    /// Opens the window, and looks for `ffmpeg` the first time.
    pub fn show_window(&mut self) {
        self.open = true;
        if matches!(self.probe, Probe::NotAsked) {
            self.check_ffmpeg();
        }
    }

    /// Looks for `ffmpeg` again, on a thread.
    pub fn check_ffmpeg(&mut self) {
        let (found, waiting) = channel();
        let asked = std::thread::Builder::new()
            .name("ffmpeg probe".to_owned())
            .spawn(move || {
                let _ = found.send(encode::probe());
            });
        self.probe = match asked {
            Ok(_) => Probe::Asking(waiting),
            Err(_) => Probe::Known(None),
        };
    }

    /// Sets what `ffmpeg` was found, as if the probe had answered: for
    /// tests, which mustn't depend on the machine.
    #[cfg(test)]
    pub fn set_ffmpeg(&mut self, ffmpeg: Option<Ffmpeg>) {
        self.probe = Probe::Known(ffmpeg);
    }

    /// The `ffmpeg` found, once the probe has answered.
    pub fn ffmpeg(&self) -> Option<&Ffmpeg> {
        match &self.probe {
            Probe::Known(ffmpeg) => ffmpeg.as_ref(),
            _ => None,
        }
    }

    /// The formats that can be written now: a PNG sequence, and what the
    /// `ffmpeg` found encodes.
    pub fn formats(&self) -> Vec<Format> {
        self.ffmpeg()
            .map_or_else(|| vec![Format::Png], |ffmpeg| ffmpeg.encoders.formats())
    }

    /// Whether a movie is being rendered.
    pub fn rendering(&self) -> bool {
        self.job.is_some()
    }

    /// The settings a job is started with: the play range only while it's
    /// chosen and marked.
    pub fn job_settings(&self, range: Option<(f64, f64)>) -> MovieSettings {
        MovieSettings {
            range: range.filter(|_| self.use_range),
            ..self.settings.clone()
        }
    }

    /// Takes the output picked in the Save dialog.
    pub fn set_output(&mut self, path: PathBuf) {
        self.settings.format = Format::of_path(&path);
        self.settings.output = path;
        self.output_set = true;
    }

    /// Brings the settings up to date with what the probe found, the
    /// machine and the open sequence, before they're shown or used.
    pub fn settle(&mut self, scene: &MovieScene) {
        if let Probe::Asking(waiting) = &self.probe
            && let Ok(found) = waiting.try_recv()
        {
            // The first answer picks the format: a video if one can be
            // made, as most people want a file that plays.
            let first = !self.output_set && self.settings.format == Format::Png;
            self.probe = Probe::Known(found);
            if first && let Some(best) = self.formats().first() {
                self.settings.format = *best;
            }
        }
        if !self.formats().contains(&self.settings.format) && !self.rendering() {
            self.settings.format = Format::Png;
        }
        if let Some(capabilities) = scene.capabilities {
            if !self.quality_set {
                self.settings.quality = MovieSettings::best_quality(capabilities);
            }
            if !capabilities.ray_queries {
                self.settings.ray_tracing = false;
            }
        }
        if scene.range.is_none() {
            self.use_range = false;
        }
        if let (Some(name), Some(project)) = (scene.sequence, scene.project) {
            let made_for = (name.to_owned(), project.to_path_buf());
            if self.made_for.as_ref() != Some(&made_for) {
                self.made_for = Some(made_for);
                self.output_set = false;
            }
            if !self.output_set {
                self.settings.output = default_output(project, name, self.settings.format);
            }
        }
    }

    /// Shows the window while it's open. Returns what was asked for.
    pub fn show(&mut self, ctx: &egui::Context, scene: &MovieScene) -> Vec<MovieAction> {
        let mut actions = Vec::new();
        if self.open {
            self.settle(scene);
            if matches!(self.probe, Probe::Asking(_)) || self.rendering() {
                ctx.request_repaint();
            }
        }
        let mut open = self.open;
        widgets::tool_window(icons::FILM_STRIP, "Render Movie")
            .size(WINDOW_SIZE)
            .show(ctx, &mut open, |ui| {
                let Some(name) = scene.sequence else {
                    ui.label("Open a sequence in the Sequencer tab to render it as a movie.");
                    return;
                };
                // Render and the progress stay in view; the settings above
                // them scroll.
                egui::Panel::bottom("movie footer")
                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 6)))
                    .show(ui, |ui| self.footer(ui, scene, &mut actions));
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.settings(ui, name, scene, &mut actions));
                    });
            });
        self.open = open;
        actions
    }

    /// What to render, the picture, the quality and the output: not to be
    /// changed while a movie is rendering.
    fn settings(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        scene: &MovieScene,
        actions: &mut Vec<MovieAction>,
    ) {
        let rendering = self.rendering();
        ui.add_enabled_ui(!rendering, |ui| {
            self.what(ui, name, scene);
            ui.add_space(6.0);
            self.picture(ui, scene);
            ui.add_space(6.0);
            self.quality(ui, scene);
            ui.add_space(6.0);
            self.output(ui, actions);
        });
    }

    /// Render, or while rendering the progress, the last frame and Cancel.
    fn footer(&mut self, ui: &mut egui::Ui, scene: &MovieScene, actions: &mut Vec<MovieAction>) {
        let palette = theme::palette(ui.ctx());
        if let Some(job) = &self.job {
            let (frame, total) = job.frames();
            let (elapsed, warming) = (job.elapsed(), job.warming());
            ui.add(
                egui::ProgressBar::new(frame as f32 / total.max(1) as f32)
                    .desired_width(ui.available_width().min(WINDOW_SIZE.x - 40.0))
                    .text(format!("Frame {frame} of {total}")),
            );
            let state = if frame == 0 && warming > 0 {
                format!("Warming up: {warming} frames to go")
            } else if frame == 0 {
                "Starting".to_owned()
            } else {
                // From the frames kept so far; the first ones include the
                // warm-up, so it starts a little high and settles.
                let left = elapsed / f64::from(frame) * f64::from(total - frame);
                format!("about {} left", clock(left))
            };
            ui.label(format!("{} elapsed · {state}", clock(elapsed)));
            self.show_preview(ui);
            if ui
                .add(widgets::button(
                    Role::Secondary,
                    Some(icons::X),
                    "Cancel",
                    &palette,
                ))
                .on_hover_text("Stop rendering. A half-written video file is removed.")
                .clicked()
            {
                actions.push(MovieAction::Cancel);
            }
            return;
        }

        if let Some(error) = &self.error {
            ui.colored_label(palette.error, theme::with_icon(icons::WARNING, error));
        }
        if let Some(done) = &self.done {
            ui.label(theme::with_icon(
                icons::CHECK_CIRCLE,
                &format!(
                    "Saved {} ({} frames, {} × {}, in {}).",
                    done.output.display(),
                    done.frames,
                    done.size.0,
                    done.size.1,
                    clock(done.seconds)
                ),
            ));
            self.show_preview(ui);
        }
        let blocked = if scene.playing {
            Some("Stop the game first: a movie is rendered from the edited world.")
        } else if scene.capabilities.is_none() {
            Some("The editor's graphics device isn't ready yet.")
        } else if self.settings.output.as_os_str().is_empty() {
            Some("Choose where to save the movie.")
        } else {
            None
        };
        ui.horizontal_wrapped(|ui| {
            let render = ui.add_enabled(
                blocked.is_none(),
                widgets::button(Role::Primary, Some(icons::FILM_STRIP), "Render", &palette),
            );
            let render = match blocked {
                Some(why) => render.on_disabled_hover_text(why),
                None => render.on_hover_text(
                    "Draw every frame and save the movie. The editor stays usable meanwhile.",
                ),
            };
            if render.clicked() {
                actions.push(MovieAction::Render);
            }
            if let Some(done) = &self.done {
                let folder = if done.output.is_dir() {
                    done.output.clone()
                } else {
                    done.output
                        .parent()
                        .map_or_else(|| done.output.clone(), Path::to_path_buf)
                };
                if ui
                    .add(widgets::button(
                        Role::Secondary,
                        Some(icons::FOLDER_OPEN),
                        "Open folder",
                        &palette,
                    ))
                    .on_hover_text("Show the movie in the system's file manager")
                    .clicked()
                {
                    actions.push(MovieAction::OpenFolder(folder));
                }
            }
        });
    }

    /// The whole sequence or the play range.
    fn what(&mut self, ui: &mut egui::Ui, name: &str, scene: &MovieScene) {
        widgets::section_label(ui, "What to render");
        let fps = self.settings.fps;
        let frames = |from: f64, to: f64| frame_count(from, to, fps);
        ui.radio_value(
            &mut self.use_range,
            false,
            format!(
                "All of {name} ({}, {} frames)",
                clock(scene.duration),
                frames(0.0, scene.duration)
            ),
        );
        let range = scene.range;
        let label = match range {
            Some((from, to)) => format!(
                "The play range, {:.2} s to {:.2} s ({} frames)",
                from,
                to,
                frames(from, to)
            ),
            None => "The play range".to_owned(),
        };
        ui.add_enabled_ui(range.is_some(), |ui| {
            ui.radio_value(&mut self.use_range, true, label)
                .on_disabled_hover_text(
                    "Mark a range with In and Out (I and O) in the Sequencer to render only \
                     that part.",
                );
        });
    }

    /// Resolution, shape and frame rate.
    fn picture(&mut self, ui: &mut egui::Ui, scene: &MovieScene) {
        widgets::section_label(ui, "Picture");
        let most = scene
            .capabilities
            .map_or(MAX_SIDE, |capabilities| capabilities.max_texture_size);
        let settings = &mut self.settings;
        crate::responsive::form(ui, "movie picture", |ui| {
            ui.label("Resolution");
            egui::ComboBox::from_id_salt("movie resolution")
                .selected_text(settings.resolution.label())
                .show_ui(ui, |ui| {
                    for resolution in Resolution::ALL {
                        ui.selectable_value(
                            &mut settings.resolution,
                            resolution,
                            resolution.label(),
                        );
                    }
                });
            ui.end_row();
            if settings.resolution == Resolution::Custom {
                ui.label("Size");
                ui.horizontal(|ui| {
                    let side = |ui: &mut egui::Ui, value: &mut u32| {
                        ui.add(
                            egui::DragValue::new(value)
                                .speed(2.0)
                                .range(MIN_SIDE..=most.min(MAX_SIDE)),
                        );
                    };
                    side(ui, &mut settings.custom.0);
                    ui.label("×");
                    side(ui, &mut settings.custom.1);
                });
                ui.end_row();
            } else {
                ui.label("Shape");
                egui::ComboBox::from_id_salt("movie aspect")
                    .selected_text(settings.aspect.label())
                    .show_ui(ui, |ui| {
                        for aspect in Aspect::ALL {
                            if ui
                                .selectable_value(&mut settings.aspect, aspect, aspect.label())
                                .clicked()
                            {
                                // A scope picture is already the shape the
                                // bars would leave: don't cut it twice.
                                settings.letterbox = aspect != Aspect::Scope;
                            }
                        }
                    })
                    .response
                    .on_hover_text(
                        "2.39:1 is the cinema's wide picture: what a sequence's letterbox bars \
                         leave of a 16:9 frame, as the whole movie.",
                    );
                ui.end_row();
            }
            ui.label("Frame rate");
            egui::ComboBox::from_id_salt("movie fps")
                .selected_text(format!("{} fps", settings.fps))
                .show_ui(ui, |ui| {
                    for fps in FRAME_RATES {
                        ui.selectable_value(&mut settings.fps, fps, format!("{fps} fps"));
                    }
                })
                .response
                .on_hover_text(
                    "Frames a second. The world moves on by exactly one frame's time for each \
                     frame, however long it takes to draw, so the movie is the same on every run.",
                );
            ui.end_row();
        });
        let (width, height) = settings.size(most);
        ui.weak(format!("The movie is {width} × {height} pixels."));
    }

    /// The graphics preset, ray tracing, motion blur and warm-up.
    fn quality(&mut self, ui: &mut egui::Ui, scene: &MovieScene) {
        widgets::section_label(ui, "Quality");
        let capabilities = scene.capabilities.copied().unwrap_or_default();
        let settings = &mut self.settings;
        let quality_set = &mut self.quality_set;
        crate::responsive::form(ui, "movie quality", |ui| {
            ui.label("Graphics");
            egui::ComboBox::from_id_salt("movie quality")
                .selected_text(settings.quality.label())
                .show_ui(ui, |ui| {
                    for quality in Quality::ALL {
                        if ui
                            .selectable_value(&mut settings.quality, quality, quality.label())
                            .clicked()
                        {
                            *quality_set = true;
                        }
                    }
                })
                .response
                .on_hover_text(
                    "The graphics preset the movie is drawn with, whatever the editor's own \
                     is. A movie isn't drawn in real time, so the best the machine can do is \
                     the usual choice.",
                );
            ui.end_row();

            ui.label("Ray tracing");
            let why_not = if capabilities.ray_tracing_offered {
                "Turn on Settings > Graphics > Hardware ray tracing and restart the editor to \
                 ray trace movies."
            } else {
                "This machine's graphics card can't trace rays."
            };
            ui.add_enabled(
                capabilities.ray_queries,
                egui::Checkbox::new(&mut settings.ray_tracing, "Every ray traced effect"),
            )
            .on_hover_text(
                "Ray traced shadows, ambient occlusion, bounced light (global illumination) \
                 and reflections, each at its highest setting.",
            )
            .on_disabled_hover_text(why_not);
            ui.end_row();

            ui.label("Motion blur");
            ui.horizontal(|ui| {
                let mut own = settings.motion_blur.is_some();
                if ui
                    .checkbox(&mut own, "Set here")
                    .on_hover_text(
                        "Off: moving things streak as the world's MotionBlurEffect says (not \
                         at all without one). On: by the strength set here, whatever the world \
                         has.",
                    )
                    .changed()
                {
                    settings.motion_blur = own.then_some(0.5);
                }
                if let Some(strength) = &mut settings.motion_blur {
                    ui.add(egui::Slider::new(strength, 0.0..=1.0))
                        .on_hover_text(
                            "How much of the way something moved in a frame it streaks over: \
                             0.5 is a film camera's shutter, 0 none.",
                        );
                }
            });
            ui.end_row();

            ui.label("Warm-up");
            ui.add(
                egui::DragValue::new(&mut settings.warm_up)
                    .speed(1.0)
                    .range(0..=600)
                    .suffix(" frames"),
            )
            .on_hover_text(
                "Frames drawn and thrown away before the first one is kept, and a quarter as \
                 many (16 at most) after each camera cut. Ray traced light, light probes, \
                 anti-aliasing and textures on their way to the graphics card all take some \
                 frames to settle; without a warm-up the movie's first frames flicker or show \
                 blank surfaces.",
            );
            ui.end_row();
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut settings.letterbox, "Letterbox bars")
                .on_hover_text(
                    "Off: the bars of the sequence's Letterbox sections are left out. Turn it \
                     off when the movie's shape is already 2.39:1, so the picture isn't cut \
                     twice.",
                );
            ui.checkbox(&mut settings.game_ui, "The game's own UI")
                .on_hover_text(
                    "Off: the world's ScreenGuis are hidden. The sequence's fades, bars and \
                     subtitles are always drawn.",
                );
        });
        ui.weak("Depth of field and the other lens effects come from the world, as in the game.")
            .on_hover_text(
                "Lighting's effects (DepthOfFieldEffect, BloomEffect, ColorCorrectionEffect \
                 and the rest) and GraphicsVolumes are drawn as the game draws them. Key \
                 their numbers in the sequence to pull focus or change the grade.",
            );
    }

    /// The format and the file.
    fn output(&mut self, ui: &mut egui::Ui, actions: &mut Vec<MovieAction>) {
        let palette = theme::palette(ui.ctx());
        widgets::section_label(ui, "Output");
        let available = self.formats();
        let asking = matches!(self.probe, Probe::Asking(_));
        let encoder_of = |format: Format| {
            self.ffmpeg()
                .and_then(|ffmpeg| ffmpeg.encoders.encoder(format))
        };
        let before = self.settings.format;
        let mut format = before;
        crate::responsive::form(ui, "movie output", |ui| {
            ui.label("Format");
            egui::ComboBox::from_id_salt("movie format")
                .selected_text(format.label())
                .show_ui(ui, |ui| {
                    for choice in Format::ALL {
                        let offered = available.contains(&choice);
                        let hint = match encoder_of(choice) {
                            Some(encoder) => {
                                format!("{} Encoded with ffmpeg's {encoder}.", choice.hint())
                            }
                            None => choice.hint().to_owned(),
                        };
                        ui.add_enabled_ui(offered, |ui| {
                            ui.selectable_value(&mut format, choice, choice.label())
                                .on_hover_text(hint)
                                .on_disabled_hover_text(
                                    "This machine's ffmpeg has no encoder for it.",
                                );
                        });
                    }
                });
            ui.end_row();
        });
        if format != before {
            self.settings.format = format;
            self.settings.output = with_format(&self.settings.output, format);
        }
        // Whether there's an ffmpeg, said plainly.
        match &self.probe {
            Probe::Known(Some(ffmpeg)) => {
                ui.weak(format!(
                    "Videos are encoded by {}.",
                    ffmpeg.program.display()
                ));
            }
            Probe::Known(None) => {
                ui.colored_label(
                    palette.warning,
                    theme::with_icon(
                        icons::WARNING,
                        "ffmpeg wasn't found on this machine, so only a PNG image sequence can \
                         be rendered. Videos (MP4, MKV) are encoded by ffmpeg.",
                    ),
                );
                ui.weak(encode::install_hint());
                if ui.small_button("Check again").clicked() {
                    self.check_ffmpeg();
                }
            }
            _ => {
                ui.weak(if asking {
                    "Looking for ffmpeg…"
                } else {
                    "ffmpeg hasn't been looked for yet."
                });
            }
        }
        ui.add_space(2.0);
        ui.label(if self.settings.format == Format::Png {
            "Save in the folder"
        } else {
            "Save as"
        });
        // The button first, from the right, so the field takes exactly
        // what's left: a field sized by guessing the button's width makes
        // the row a little too wide, and the window grows every frame.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(widgets::button(
                    Role::Secondary,
                    Some(icons::FOLDER_OPEN),
                    "Browse…",
                    &palette,
                ))
                .on_hover_text("Choose the file with the system's Save dialog")
                .clicked()
            {
                actions.push(MovieAction::Browse);
            }
            let mut text = self.settings.output.to_string_lossy().into_owned();
            if ui
                .add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY))
                .changed()
            {
                self.settings.output = PathBuf::from(text);
                self.output_set = true;
            }
        });
        if self.settings.output.exists() {
            ui.weak(if self.settings.format == Format::Png {
                "The folder is there already: frames with the same names are replaced."
            } else {
                "The file is there already: it will be replaced."
            });
        }
    }

    /// Shows the job's last frame, small: a few times a second, or `now`
    /// (the movie's last frame, when it ends).
    pub fn update_preview(&mut self, ctx: &egui::Context, now: bool) {
        let Some(job) = &self.job else {
            return;
        };
        let elapsed = job.elapsed();
        let due = now
            || self
                .preview
                .as_ref()
                .is_none_or(|(_, shown)| elapsed - shown >= PREVIEW_EVERY || elapsed < *shown);
        let frame = job.last_frame();
        if !due || frame.is_empty() {
            return;
        }
        let image = thumbnail(frame, job.size(), PREVIEW_WIDTH);
        let texture = ctx.load_texture("movie preview", image, egui::TextureOptions::LINEAR);
        self.preview = Some((texture, elapsed));
    }

    /// Forgets the frame shown, when a new movie starts.
    pub fn clear_preview(&mut self) {
        self.preview = None;
    }

    fn show_preview(&self, ui: &mut egui::Ui) {
        let Some((texture, _)) = &self.preview else {
            return;
        };
        let size = texture.size_vec2();
        let width = ui.available_width().min(size.x);
        let shown = egui::vec2(width, width * size.y / size.x.max(1.0));
        ui.add(egui::Image::new((texture.id(), shown)).corner_radius(4.0));
    }
}

/// A frame (rows of RGBA bytes, `size` pixels) made at most `most` pixels
/// wide by taking every nth pixel: quick, and good enough to see how the
/// movie is going.
pub fn thumbnail(frame: &[u8], (width, height): (u32, u32), most: usize) -> egui::ColorImage {
    let (width, height) = (width as usize, height as usize);
    let step = width.div_ceil(most.max(1)).max(1);
    let (small_width, small_height) = (width.div_ceil(step).max(1), height.div_ceil(step).max(1));
    let mut pixels = Vec::with_capacity(small_width * small_height);
    for y in 0..small_height {
        for x in 0..small_width {
            let at = ((y * step) * width + x * step) * 4;
            let pixel = frame.get(at..at + 3).unwrap_or(&[0, 0, 0]);
            pixels.push(egui::Color32::from_rgb(pixel[0], pixel[1], pixel[2]));
        }
    }
    egui::ColorImage::new([small_width, small_height], pixels)
}
