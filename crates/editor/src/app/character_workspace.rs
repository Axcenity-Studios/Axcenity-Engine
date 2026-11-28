//! The editor's side of the Character Studio workspace
//! (`crate::character_workspace`): opening it from the start screen,
//! drawing it in place of the start screen (or of a project's panels,
//! for the round trip: `character_round_trip.rs`), and leaving it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

use axcenity_render::TextureRenderer;

use eframe::egui;
use eframe::egui_wgpu::RenderState;

use super::EditorApp;
use crate::character_workspace::{CharacterWorkspace, Start, WorkspaceAction};
use crate::dialog;
use crate::theme::{self, icons};

/// What the opening thread hands back: the workspace, and the preview's
/// renderer with its shaders ready.
type Opened = Result<(Box<CharacterWorkspace>, TextureRenderer), String>;

/// Character Studio opening on its own thread (`begin_character_workspace`).
pub(super) struct CharacterOpening {
    /// What the loading screen calls it.
    name: String,
    /// What the thread is doing now: an index into [`CharacterOpening::STEPS`].
    step: Arc<AtomicUsize>,
    done: Receiver<Opened>,
    started: Instant,
}

impl CharacterOpening {
    /// What the loading screen says for each step, and why it may take a
    /// while.
    const STEPS: [(&'static str, Option<&'static str>); 2] = [
        (
            "Checking the character library and building the body",
            Some("The first open copies the starter library, about 300 MB."),
        ),
        (
            "Preparing the graphics card",
            Some(
                "The first open after an update or a driver change prepares shaders, which can take several seconds; later opens are quicker.",
            ),
        ),
    ];

    fn start(
        ctx: &egui::Context,
        folder: PathBuf,
        start: Option<Start>,
        file: Option<PathBuf>,
        device: &axcenity_render::wgpu::Device,
        queue: &axcenity_render::wgpu::Queue,
    ) -> Option<CharacterOpening> {
        let name = match &file {
            Some(file) => file
                .file_stem()
                .map(|stem| {
                    stem.to_string_lossy()
                        .trim_end_matches(".character")
                        .to_owned()
                })
                .unwrap_or_else(|| "a character".to_owned()),
            None => "Character Studio".to_owned(),
        };
        let step = Arc::new(AtomicUsize::new(0));
        let (send, done) = mpsc::channel();
        let (device, queue) = (device.clone(), queue.clone());
        let (ctx, on_thread) = (ctx.clone(), step.clone());
        std::thread::Builder::new()
            .name("character studio opening".to_owned())
            .spawn(move || {
                let opened = match &file {
                    Some(file) => CharacterWorkspace::open_file(&folder, file),
                    None => CharacterWorkspace::open_new(&folder, &start.unwrap_or_default()),
                };
                let result = opened.map(|workspace| {
                    on_thread.store(1, Ordering::Relaxed);
                    ctx.request_repaint();
                    // The size is a stand-in: the first frame resizes it.
                    let renderer = TextureRenderer::new(&device, &queue, 640, 360);
                    (Box::new(workspace), renderer)
                });
                let _ = send.send(result);
                ctx.request_repaint();
            })
            .ok()?;
        Some(CharacterOpening {
            name,
            step,
            done,
            started: Instant::now(),
        })
    }

    fn poll(&self) -> Option<Opened> {
        match self.done.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err("Character Studio stopped while opening.".to_owned()))
            }
        }
    }

    /// What the loading screen says now.
    fn doing(&self) -> (String, Option<&'static str>) {
        let (doing, note) =
            Self::STEPS[self.step.load(Ordering::Relaxed).min(Self::STEPS.len() - 1)];
        (doing.to_owned(), note)
    }
}

impl EditorApp {
    /// Where Character Studio keeps its files; the start screen lists the
    /// saved characters there.
    pub fn set_characters_folder(&mut self, folder: Option<PathBuf>) {
        self.start.set_characters_folder(folder.clone());
        self.characters_folder = folder;
    }

    /// Opens the workspace from the start screen: on a thread, with a
    /// loading screen meanwhile (`character_opening_ui`). Opening reads
    /// and checks the character library and builds the body (a fraction of
    /// a second), and the preview's renderer prepares its shaders, which
    /// takes seconds the first time after an update. All of it used to
    /// happen in the click's frame, and the start screen froze with nothing
    /// to say why. Without a graphics card to hand (tests), it opens at once.
    pub(super) fn begin_character_workspace(
        &mut self,
        ctx: &egui::Context,
        start: Option<Start>,
        file: Option<&Path>,
        render_state: Option<&RenderState>,
    ) {
        let Some(render_state) = render_state else {
            self.open_character_workspace(start, file);
            return;
        };
        let Some(folder) = self.characters_folder.clone() else {
            self.start.set_notice(
                "Character Studio has nowhere to keep its files on this computer.".into(),
            );
            return;
        };
        if self.character_opening.is_some() {
            return;
        }
        self.character_opening = CharacterOpening::start(
            ctx,
            folder,
            start,
            file.map(Path::to_owned),
            &render_state.device,
            &render_state.queue,
        );
    }

    /// While the workspace opens on its thread: the loading screen, then
    /// the workspace once it's ready. True while it's still opening (the
    /// start screen isn't drawn then).
    pub(super) fn character_opening_ui(
        &mut self,
        ui: &mut egui::Ui,
        render_state: Option<&RenderState>,
    ) -> bool {
        let Some(opening) = &self.character_opening else {
            return false;
        };
        match opening.poll() {
            None => {
                let rect = ui.max_rect();
                let (doing, note) = opening.doing();
                let mark = self.brand.as_ref().map(|brand| brand.mark());
                let cancelled = crate::loading::show_step(
                    ui,
                    rect,
                    &opening.name,
                    &doing,
                    note,
                    opening.started,
                    mark,
                );
                if cancelled {
                    // The thread finishes on its own; what it made is let go.
                    self.character_opening = None;
                    return false;
                }
                true
            }
            Some(result) => {
                self.character_opening = None;
                match result {
                    Ok((mut workspace, renderer)) => {
                        if let Some(render_state) = render_state {
                            workspace.adopt_renderer(renderer, render_state);
                        }
                        self.character_workspace = Some(workspace);
                    }
                    Err(error) => self.start.set_notice(error),
                }
                ui.ctx().request_repaint();
                false
            }
        }
    }

    /// Opens the workspace on a new character from `start` or on a saved
    /// `file`, in this frame. A problem is told on the start screen.
    pub(super) fn open_character_workspace(&mut self, start: Option<Start>, file: Option<&Path>) {
        let Some(folder) = self.characters_folder.clone() else {
            self.start.set_notice(
                "Character Studio has nowhere to keep its files on this computer.".into(),
            );
            return;
        };
        let opened = match file {
            Some(file) => CharacterWorkspace::open_file(&folder, file),
            None => CharacterWorkspace::open_new(&folder, &start.unwrap_or_default()),
        };
        match opened {
            Ok(workspace) => self.character_workspace = Some(Box::new(workspace)),
            Err(error) => self.start.set_notice(error),
        }
    }

    /// Opens the workspace on the default character (`--character-studio`).
    pub fn open_character_studio(&mut self) {
        self.open_character_workspace(Some(Start::default()), None);
    }

    /// Saves the workspace's character if it has unsaved changes (for
    /// "Save changes?" on closing the window). False when that failed,
    /// which the workspace's status line says.
    pub(super) fn save_character_workspace(&mut self) -> bool {
        let Some(workspace) = self.character_workspace.as_mut() else {
            return true;
        };
        if !workspace.dirty() {
            return true;
        }
        match workspace.save() {
            Ok(file) => {
                self.start.character_saved(&file);
                true
            }
            Err(error) => {
                workspace.notice = Some(error);
                false
            }
        }
    }

    /// Draws the open workspace and does what it asks.
    pub(super) fn character_workspace_ui(
        &mut self,
        ui: &mut egui::Ui,
        render_state: Option<&RenderState>,
    ) {
        if let Some(render_state) = render_state {
            // A saved character's picture is drawn on it.
            self.start
                .set_gpu(&render_state.device, &render_state.queue);
        }
        let Some(workspace) = self.character_workspace.as_mut() else {
            return;
        };
        let palette = theme::palette(ui.ctx());
        let action = egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(palette.app_bg))
            .show(ui, |ui| workspace.ui(ui, render_state))
            .inner;
        // What it saved gets a fresh picture on the start page.
        for file in std::mem::take(&mut workspace.saved_files) {
            self.start.character_saved(&file);
        }
        // Opened from the Game Editor: Done and Cancel go back to it, and
        // so does Home (as Cancel).
        let from_game = self
            .character_workspace
            .as_ref()
            .is_some_and(|workspace| workspace.game().is_some());
        match action {
            Some(WorkspaceAction::Done) if from_game => {
                self.finish_character_round_trip(true);
                return;
            }
            Some(WorkspaceAction::Cancel | WorkspaceAction::Home) if from_game => {
                self.finish_character_round_trip(false);
                return;
            }
            _ => {}
        }
        let Some(workspace) = self.character_workspace.as_mut() else {
            return;
        };
        if action == Some(WorkspaceAction::Home) {
            if workspace.dirty() {
                workspace.leaving = true;
            } else {
                self.character_workspace = None;
                return;
            }
        }
        if workspace.leaving {
            let (icon, tone) = dialog::unsaved_style();
            let message = format!(
                "\"{}\" has unsaved changes. Save them before going back to the start screen?",
                workspace.description().name
            );
            let outcome = dialog::ask(
                ui.ctx(),
                &dialog::Question {
                    id: "leave character studio",
                    icon,
                    tone,
                    title: "Save changes?",
                    message: &message,
                    items: Vec::new(),
                },
                &[
                    dialog::Button::new(icons::X, "Cancel", dialog::Kind::Plain),
                    dialog::Button::new(icons::TRASH, "Don't Save", dialog::Kind::Plain),
                    dialog::Button::new(icons::FLOPPY_DISK, "Save", dialog::Kind::Main),
                ],
            );
            match outcome {
                Some(dialog::Outcome::Picked(2)) => match workspace.save() {
                    Ok(file) => {
                        self.start.character_saved(&file);
                        self.character_workspace = None;
                    }
                    Err(error) => {
                        workspace.notice = Some(error);
                        workspace.leaving = false;
                    }
                },
                Some(dialog::Outcome::Picked(1)) => self.character_workspace = None,
                Some(_) => workspace.leaving = false,
                None => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::drawn_texts;
    use super::*;

    fn frame(app: &mut EditorApp, ctx: &egui::Context) -> Vec<String> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            ..Default::default()
        };
        let output = crate::theme::run(ctx, input, |ui| app.frame(ui, None));
        drawn_texts(&output)
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }

    #[test]
    fn character_studio_opens_from_the_start_screen_as_a_workspace_of_its_own() {
        let ctx = crate::theme::test_context();
        let mut app = EditorApp::new(None);
        let folder = crate::character_studio::test_folder("workspace-app");
        app.set_characters_folder(Some(folder.clone()));
        let start = frame(&mut app, &ctx);
        assert!(
            start.iter().any(|text| text.contains("Character Studio")),
            "the start screen lists it: {start:?}"
        );
        assert!(start.iter().any(|text| text == "Debug"), "{start:?}");

        // The Female preset, from the start page: the human base's female
        // body, in the underwear that body type wears.
        app.open_character_workspace(Some(Start::shipped("female")), None);
        let workspace = app.character_workspace.as_ref().expect("opened");
        assert_eq!(
            workspace.description().base,
            crate::character_workspace::DEFAULT_BASE
        );
        assert_eq!(workspace.description().parameters.get("frame"), Some(&1.0));
        let worn: Vec<&str> = workspace
            .description()
            .items
            .iter()
            .map(|worn| worn.id.as_str())
            .collect();
        assert_eq!(
            worn,
            ["axcenity/human-boxer-briefs", "axcenity/human-bandeau"]
        );
        assert_eq!(app.title(), "New Character* - Character Studio");
        let drawn = frame(&mut app, &ctx);
        // The pages, and the Body page it opens on: "Athletic" is one of
        // the human base's body presets there.
        for text in ["Customize", "Presets", "Face", "Clothing", "Athletic"] {
            let found = drawn
                .iter()
                .any(|t| t.to_lowercase().contains(&text.to_lowercase()));
            assert!(found, "{text} in {drawn:?}");
        }
        // No Game Editor menus over it.
        assert!(!drawn.iter().any(|text| text == "Debug"), "{drawn:?}");

        // Leaving with unsaved changes asks first.
        app.character_workspace.as_mut().unwrap().leaving = true;
        // A modal is sized on its first frame and shown on the next.
        frame(&mut app, &ctx);
        let asked = frame(&mut app, &ctx);
        assert!(
            asked
                .iter()
                .any(|text| text.contains("\"New Character\" has unsaved changes")),
            "{asked:?}"
        );
        // A saved character opens from its file, not dirty.
        let workspace = app.character_workspace.as_mut().unwrap();
        workspace.leaving = false;
        let saved = workspace.save().unwrap();
        app.character_workspace = None;
        app.open_character_workspace(None, Some(&saved));
        assert!(!app.character_workspace.as_ref().unwrap().dirty());
        assert_eq!(app.title(), "New Character - Character Studio");
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Opening from the start screen happens on a thread, behind a loading
    /// screen that says what it's doing, and the workspace comes in when
    /// it's ready. It used to freeze the start screen for the whole open.
    #[test]
    fn character_studio_opens_on_a_thread_behind_a_loading_screen() {
        let instance = axcenity_render::adapters::instance();
        let Ok(adapter) = pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
        else {
            eprintln!("no GPU: skipped");
            return;
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let ctx = crate::theme::test_context();
        let mut app = EditorApp::new(None);
        let folder = crate::character_studio::test_folder("workspace-thread");
        app.set_characters_folder(Some(folder.clone()));
        frame(&mut app, &ctx);
        app.character_opening = CharacterOpening::start(
            &ctx,
            folder.clone(),
            Some(Start::default()),
            None,
            &device,
            &queue,
        );
        assert!(app.character_opening.is_some());
        // Past the loading screen's grace period, it shows if still opening.
        std::thread::sleep(std::time::Duration::from_millis(350));
        let started = Instant::now();
        let mut screen_seen = false;
        while app.character_workspace.is_none() {
            let opening = app.character_opening.is_some();
            let drawn = frame(&mut app, &ctx);
            if opening && app.character_workspace.is_none() {
                assert!(
                    drawn.iter().any(|text| text.contains("Character Studio")),
                    "the loading screen names what's opening: {drawn:?}"
                );
                assert!(
                    !drawn.iter().any(|text| text == "Projects"),
                    "the start screen isn't drawn under it: {drawn:?}"
                );
                screen_seen = true;
            }
            assert!(started.elapsed().as_secs() < 300, "it never opened");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(app.character_opening.is_none());
        eprintln!("loading screen seen: {screen_seen}");
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// D-02: closing the editor's window with unsaved changes in the
    /// workspace asks first, and Save saves the character.
    #[test]
    fn closing_the_window_with_unsaved_studio_changes_asks_and_saves_the_character() {
        let ctx = crate::theme::test_context();
        let mut app = EditorApp::new(None);
        let folder = crate::character_studio::test_folder("workspace-close");
        app.set_characters_folder(Some(folder.clone()));
        app.open_character_workspace(Some(Start::default()), None);
        assert!(app.unsaved(), "a new character isn't saved");
        let modified = app.modified_documents();
        assert!(
            modified
                .iter()
                .any(|doc| doc.name.contains("New Character")),
            "{modified:?}"
        );
        // The window's close button.
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .events
            .push(egui::ViewportEvent::Close);
        let _ = crate::theme::run(&ctx, input, |ui| app.frame(ui, None));
        assert_eq!(app.asking, Some(super::super::Discard::Quit), "it asks");
        // Save saves the character and lets the window close.
        assert_eq!(
            app.answer(super::super::Answer::Save),
            Some(super::super::Discard::Quit)
        );
        let workspace = app.character_workspace.as_ref().unwrap();
        assert!(!workspace.dirty());
        assert!(workspace.file_path().unwrap().is_file());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Quitting with "Don't Save" removes the studio's recovery file at
    /// once (`discard_unsaved`), not only when the workspace is dropped:
    /// the next open must not offer work the person threw away.
    #[test]
    fn dont_save_on_quit_removes_the_studio_recovery_file() {
        let mut app = EditorApp::new(None);
        let folder = crate::character_studio::test_folder("workspace-dont-save");
        app.set_characters_folder(Some(folder.clone()));
        app.open_character_workspace(Some(Start::default()), None);
        let workspace = app.character_workspace.as_mut().unwrap();
        workspace.apply(crate::character_workspace::Intent::SetParameter {
            id: "fat".into(),
            value: 0.4,
            gesture: crate::character_workspace::Gesture::Commit,
        });
        workspace.autosave_step(std::time::Instant::now());
        let recovery = folder.join(crate::character_workspace::autosave::RECOVERY_FILE);
        assert!(recovery.is_file(), "unsaved work is autosaved");
        app.asking = Some(super::super::Discard::Quit);
        assert_eq!(
            app.answer(super::super::Answer::DontSave),
            Some(super::super::Discard::Quit)
        );
        assert!(app.character_workspace.is_some(), "not dropped yet");
        assert!(!recovery.is_file(), "the recovery file is gone already");
        // A frame before the window closes doesn't write it again.
        app.character_workspace
            .as_mut()
            .unwrap()
            .autosave_step(std::time::Instant::now() + std::time::Duration::from_secs(10));
        assert!(!recovery.is_file());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
