//! Render Movie in the app (`movie.rs` and `movie/`): the window, starting
//! a job on the editor's GPU device with a copy of the world and the open
//! sequence, drawing a few of its frames each editor frame, and saying in
//! the Output panel where the movie went.

use std::time::{Duration, Instant};

use eframe::egui_wgpu::RenderState;

use super::*;
use crate::movie::{MovieAction, MovieJob, MovieScene, MovieSource, Progress, clock};
use crate::sequencer::sequence_name;

/// How long a movie's frames may take of each editor frame. A frame that
/// takes longer is still drawn whole: the editor just draws less often.
const FRAME_BUDGET: Duration = Duration::from_millis(30);

impl EditorApp {
    /// Opens the Render Movie window (the Sequencer's Render Movie button).
    pub(in crate::app) fn open_render_movie(&mut self) {
        self.movie.done = None;
        self.movie.error = None;
        self.movie.show_window();
    }

    /// Each frame: a few frames of a movie being rendered, then the window.
    pub(super) fn movie_frame(&mut self, ctx: &egui::Context, render_state: Option<&RenderState>) {
        self.step_movie(ctx);
        if !self.movie.open {
            return;
        }
        let name = self
            .sequencer
            .doc
            .as_ref()
            .map(|doc| sequence_name(&doc.path).to_owned());
        let duration = self
            .sequencer
            .doc
            .as_ref()
            .map_or(0.0, |doc| doc.duration());
        let range = self
            .sequencer
            .mode
            .range
            .map(|_| self.sequencer.mode.bounds(duration))
            .filter(|(from, to)| to > from);
        let folder = self.project.as_ref().map(|project| project.folder.clone());
        let scene = MovieScene {
            sequence: name.as_deref(),
            duration,
            range,
            project: folder.as_deref(),
            capabilities: self.capabilities.as_ref(),
            playing: self.playing(),
        };
        for action in self.movie.show(ctx, &scene) {
            match action {
                MovieAction::Render => self.start_movie(render_state, range),
                MovieAction::Cancel => self.cancel_movie(),
                MovieAction::Browse => self.browse_movie_output(),
                MovieAction::OpenFolder(folder) => {
                    if let Err(err) = crate::asset_inventory::open_folder(&folder) {
                        self.output
                            .error(format!("Could not open {}: {err}", folder.display()));
                    }
                }
            }
        }
    }

    /// Starts rendering the open sequence with the window's settings, on
    /// the editor's GPU device. Not during Play.
    pub(in crate::app) fn start_movie(
        &mut self,
        render_state: Option<&RenderState>,
        range: Option<(f64, f64)>,
    ) {
        if self.playing() || self.movie.rendering() {
            return;
        }
        self.movie.done = None;
        self.movie.error = None;
        let Some(render_state) = render_state else {
            self.movie.error = Some(
                "No graphics device to render with: the editor isn't drawing with wgpu.".into(),
            );
            return;
        };
        let Some(doc) = &self.sequencer.doc else {
            self.movie.error = Some("Open a sequence in the Sequencer tab first.".into());
            return;
        };
        let stylesheets = self
            .project
            .as_ref()
            .map(|project| project.stylesheets.clone())
            .unwrap_or_default();
        let source = MovieSource {
            world: self.world.clone(),
            sequence: doc.current(),
            assets: self.assets.clone(),
            stylesheets,
            camera: self.viewport.render_camera(),
        };
        let settings = self.movie.job_settings(range);
        let ffmpeg = self.movie.ffmpeg().cloned();
        match MovieJob::start(
            &render_state.device,
            &render_state.queue,
            source,
            &settings,
            ffmpeg.as_ref(),
        ) {
            Ok(job) => {
                for note in &job.graphics().notes {
                    self.output.info(format!("Render Movie: {note}"));
                }
                self.movie.clear_preview();
                self.movie.job = Some(job);
            }
            Err(why) => self.movie.error = Some(why),
        }
    }

    /// Draws frames of the movie until this editor frame's share of time
    /// is used, so the editor keeps answering while it renders.
    fn step_movie(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.movie.job else {
            return;
        };
        let started = Instant::now();
        let ended = loop {
            match job.step() {
                // Textures are still being read: look again next frame.
                Ok(Progress::Loading) => break None,
                Ok(Progress::Warming { .. } | Progress::Frame { .. }) => {
                    if started.elapsed() >= FRAME_BUDGET {
                        break None;
                    }
                }
                Ok(Progress::Finished(finished)) => break Some(Ok(finished)),
                Err(why) => break Some(Err(why)),
            }
        };
        self.movie.update_preview(ctx, matches!(ended, Some(Ok(_))));
        ctx.request_repaint();
        match ended {
            None => {}
            Some(Ok(finished)) => {
                // Dropping the job frees its renderers.
                self.movie.job = None;
                self.output.info(format!(
                    "Rendered the movie {} ({} frames at {} fps, {} × {}, in {}).",
                    finished.output.display(),
                    finished.frames,
                    finished.fps,
                    finished.size.0,
                    finished.size.1,
                    clock(finished.seconds)
                ));
                for missing in &finished.missing {
                    self.output.warning(format!(
                        "Render Movie: the sequence binds `{missing}`, which wasn't found."
                    ));
                }
                self.movie.done = Some(finished);
            }
            Some(Err(why)) => {
                self.movie.job = None;
                self.output
                    .error(format!("The movie could not be rendered: {why}"));
                self.movie.error = Some(why);
            }
        }
    }

    /// Stops the movie being rendered.
    pub(in crate::app) fn cancel_movie(&mut self) {
        if let Some(job) = self.movie.job.take() {
            let (frame, total) = job.frames();
            job.cancel();
            self.output.info(format!(
                "Render Movie: cancelled at frame {frame} of {total}."
            ));
        }
    }

    /// Picks the movie's file (or a PNG sequence's folder) with the
    /// system's dialog.
    fn browse_movie_output(&mut self) {
        let settings = &self.movie.settings;
        let mut dialog = rfd::FileDialog::new().set_title("Save the movie as");
        if let Some(folder) = settings.output.parent().filter(|folder| folder.is_dir()) {
            dialog = dialog.set_directory(folder);
        } else if let Some(project) = &self.project {
            dialog = dialog.set_directory(&project.folder);
        }
        let picked = match settings.format.extension() {
            Some(extension) => {
                if let Some(name) = settings.output.file_name() {
                    dialog = dialog.set_file_name(name.to_string_lossy());
                }
                dialog
                    .add_filter(settings.format.label(), &[extension])
                    .save_file()
                    .map(|path| crate::movie::with_format(&path, settings.format))
            }
            // A PNG sequence goes in a folder.
            None => dialog.pick_folder(),
        };
        if let Some(path) = picked {
            let format = self.movie.settings.format;
            self.movie.set_output(path);
            // The dialog's filter chose the file's kind, not its name.
            self.movie.settings.format = format;
        }
    }
}
