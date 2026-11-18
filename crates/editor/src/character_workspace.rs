//! The Character Studio workspace (ADR-0031; Character Studio overhaul
//! CO12, `docs/development/character-studio/architecture.md` sections 1,
//! 6, 7 and 15): a character editor of its own, opened from the start
//! screen's Character Studio page with no project open.
//!
//! It edits a character [`Description`] (format 2; a format 1 file, CS2's
//! `Look`, migrates when it's opened) on a base model of the character
//! library: the realistic family (ADR-0032), the default, or the stylized
//! one (plan section 0, decision 7: both are offered, [`offers_base`]).
//! Its controls come from the base model's manifest (the
//! parameters by group and category, the slots) and the catalogue (the
//! items that fit), so a new body or item shows up without code. The
//! preview is the Character Runtime's: the default character in a World
//! of its own with the description applied by
//! `gameplay::description::apply_description`, as a game applies it, posed
//! by the engine's `Animators` (the character idles) and drawn by the
//! scene renderer.
//!
//! Its files live in the editor's data folder (`Character Studio/`): the
//! default humanoid's files, the character library (the built-in package
//! and the packages the engine ships, [`find_character_library`]) and the
//! saved characters, `characters/<name>.character.json`; beside them the
//! start page's Recent list (`recent.rs`), character packs (`packs.rs`),
//! the cached pictures of characters and presets (`thumbnails.rs`), a
//! person's own presets (`presets/`) and favourites (`presets.rs`) and the
//! autosave's recovery file (`autosave.rs`).
//!
//! The studio is split by area (audit-studio-ui.md section 3), each
//! drawing from `&CharacterWorkspace` and returning [`Intent`]s that only
//! the shell applies: `shell.rs` (the frame, shortcuts, notices, applying
//! intents), `nav.rs` (categories and features down the left),
//! `panel.rs` (the contextual panel on the right), `toolbar.rs` and
//! `bottom_bar.rs` (camera, lighting and backdrop under the viewport).
//! `intent.rs`, `selection.rs` and `edit.rs` are the contracts they share
//! (what a panel can ask, what is shown, undo and reset). `random.rs` is
//! Randomize and its locks, `compare.rs` the held versions (before and
//! after, side by side). `stage.rs` is the camera, lighting, backdrop and
//! floor; `shelf.rs` lists the saved characters for the start page and
//! deletes them.

pub mod autosave;
mod bottom_bar;
pub mod compare;
pub mod edit;
pub mod intent;
mod nav;
pub mod packs;
mod panel;
pub mod presets;
pub mod random;
pub mod recent;
pub mod selection;
pub mod shelf;
mod shell;
mod stage;
pub mod thumbnails;
mod toolbar;

#[cfg(test)]
mod feature_tests;
#[cfg(test)]
mod shelf_tests;
#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timing_tests;
#[cfg(test)]
mod walkthrough_tests;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axcenity_anim::Animators;
use axcenity_assets::Assets;
pub use axcenity_avatar::chosen_slots;
use axcenity_avatar::{
    BaseManifest, Description, Library, MaterialValue, Parameter, PresetKind, WornItem,
    wear_choice, wear_choices,
};
use axcenity_core::{ClassName, InstanceId, Vector3, World};
use axcenity_editor_tools::preview::Canvas;
use axcenity_gameplay::description as applier;
use axcenity_render::glam::Vec3;

use autosave::{Autosave, Recovery};
use compare::Comparison;
use edit::{History, Scope};
pub use intent::{Gesture, Intent};
use selection::Selection;
pub use shell::WorkspaceAction;
pub use stage::{Backdrop, Lighting, Orbit, ViewPreset};

/// The command-line flag that opens the editor straight into the workspace.
pub const FLAG: &str = "--character-studio";

/// The base model a new character starts on: the semi-realistic family is
/// the default (ADR-0032).
pub const DEFAULT_BASE: &str = "axcenity/realistic";

/// Where a workspace's files live, in the editor's data folder.
pub fn default_folder() -> Option<PathBuf> {
    eframe::storage_dir(crate::APP_ID).map(|data| data.join("Character Studio"))
}

/// The folder of saved characters, inside the workspace's folder.
pub const CHARACTERS_FOLDER: &str = "characters";

/// The character library the engine ships (`assets/characters/`, with its
/// `catalogue.json`): looked for in the folder the editor was started
/// from, then in each folder above the editor's program (an installed
/// editor has it beside itself; a build from source under the engine's
/// folder), then in the source tree a debug build came from.
pub fn find_character_library() -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(folder) = std::env::current_dir() {
        roots.push(folder);
    }
    if let Ok(program) = std::env::current_exe() {
        roots.extend(program.ancestors().skip(1).map(Path::to_path_buf));
    }
    roots.extend(crate::rebuild::EngineSource::find().map(|source| source.workspace));
    roots
        .into_iter()
        .map(|root| root.join(axcenity_avatar::LIBRARY_FOLDER))
        .find(|folder| folder.join("catalogue.json").is_file())
}

/// What a new character starts from: a base model's default character (a
/// family), or a preset by its ID (the library's or a person's own).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Start {
    Family(String),
    Preset(String),
}

impl Start {
    /// A shipped realistic preset by its short name (`female`,
    /// `athletic-male`…), for tests.
    #[cfg(test)]
    pub fn shipped(name: &str) -> Self {
        Start::Preset(format!("axcenity/realistic-{name}"))
    }
}

impl Default for Start {
    /// The realistic family's default character.
    fn default() -> Self {
        Start::Family(DEFAULT_BASE.to_owned())
    }
}

/// Whether Character Studio offers base model `id` for new characters:
/// every family the library has (plan section 0, decision 7: both the
/// realistic and the stylized family are selectable; realistic is the
/// default and listed first, [`presets::families`]).
pub fn offers_base(library: &Library, id: &str) -> bool {
    library.base(id).is_some()
}

/// Sets `parameter` to `value` in `description` and, if the parameter has
/// choices, wears what the chosen one names ([`wear_choice`]).
fn choose(description: &mut Description, library: &Library, parameter: &Parameter, value: f32) {
    description.parameters.insert(parameter.id.clone(), value);
    wear_choice(description, library, parameter, value);
}

/// A parameter's label for a slider that edits it and its mirror together:
/// "Left ear protrusion" is "Ear protrusion".
pub fn symmetric_label(label: &str) -> String {
    for side in ["Left ", "Right ", "left ", "right "] {
        if let Some(rest) = label.strip_prefix(side) {
            let mut chars = rest.chars();
            return chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default();
        }
    }
    label
        .replace(" (left)", "")
        .replace(" (right)", "")
        .replace(" left", "")
        .replace(" right", "")
}

/// The categories down the left of the workspace, in the brief's order
/// (§3.1). The view (camera, lighting, backdrop) is in the bottom bar, not
/// a category: it is about looking, not about the character.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Category {
    #[default]
    Body,
    Face,
    Hair,
    Clothing,
    SkinAndMakeup,
    Accessories,
    Expressions,
    Presets,
}

impl Category {
    pub const ALL: [Category; 8] = [
        Category::Body,
        Category::Face,
        Category::Hair,
        Category::Clothing,
        Category::SkinAndMakeup,
        Category::Accessories,
        Category::Expressions,
        Category::Presets,
    ];

    /// The manifest's category this one shows, if it shows one.
    pub fn data(self) -> Option<axcenity_avatar::Category> {
        use axcenity_avatar::Category as Data;
        match self {
            Category::Body => Some(Data::Body),
            Category::Face => Some(Data::Face),
            Category::Hair => Some(Data::Hair),
            Category::Clothing => Some(Data::Clothing),
            Category::SkinAndMakeup => Some(Data::Skin),
            Category::Accessories => Some(Data::Accessories),
            Category::Expressions | Category::Presets => None,
        }
    }

    /// Whether its page looks at the face (the camera frames it).
    fn frames_the_face(self) -> bool {
        matches!(self, Category::Face | Category::Expressions)
    }
}

/// Why a save went where it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saved {
    /// Written to the character's own file.
    Written(PathBuf),
    /// Another character had the name: saved under a free one (audit
    /// D-01), which the character now has.
    Renamed { path: PathBuf, from: String },
}

/// The workspace: the character being edited, its preview and its undo.
pub struct CharacterWorkspace {
    folder: PathBuf,
    assets: Assets,
    world: World,
    character: Option<InstanceId>,
    animators: Animators,
    description: Description,
    /// The description as last saved or opened (`None`: never saved).
    saved: Option<Description>,
    /// The file it was opened from or last saved to: a save after a rename
    /// moves it instead of leaving a copy (audit D-03).
    file: Option<PathBuf>,
    history: History,
    /// What the preview shows, and when it was applied.
    applied: Option<(Description, Instant)>,
    /// Sliders with a mirror move it too.
    pub symmetry: bool,
    /// The category, feature and detail level shown (editor state).
    pub selection: Selection,
    pub orbit: Orbit,
    /// The view preset the camera is at, until it's moved.
    pub view: Option<ViewPreset>,
    pub lighting: Lighting,
    pub backdrop: Backdrop,
    /// The built-in expression shown on the preview's face: preview only,
    /// never in the description (brief §10).
    pub expression: Option<String>,
    canvas: Canvas,
    last_frame: Option<Instant>,
    /// A line for the status bar: what was saved, what failed.
    pub notice: Option<String>,
    /// The notice shown and since when: it clears itself (audit D-13).
    notice_shown: Option<(String, Instant)>,
    /// Home was asked for with unsaved changes: the editor asks to save.
    pub leaving: bool,
    /// New or Open was asked for with unsaved changes: the shell asks to
    /// save first, then does it.
    pub asking: Option<Intent>,
    /// The files saved since the editor last looked (it draws their
    /// pictures again).
    pub saved_files: Vec<PathBuf>,
    /// A person's own presets (`presets/`), by ID.
    own_presets: BTreeMap<String, Description>,
    /// The favourite presets' IDs (editor state, `favourites.json`).
    pub favourites: BTreeSet<String>,
    /// Parameters Randomize leaves alone (editor state).
    pub locks: BTreeSet<String>,
    /// How many times Randomize was pressed: part of its seed.
    rolls: u64,
    /// The held versions and what is shown (editor state, never undone or
    /// saved).
    pub comparison: Comparison,
    /// The second character, side by side, its own floor, and what it
    /// shows.
    other: Option<InstanceId>,
    other_floor: Option<InstanceId>,
    other_applied: Option<Description>,
    other_canvas: Canvas,
    autosave: Autosave,
    /// A character a crash left, offered when the studio opened.
    pub recovery: Option<Recovery>,
    /// The presets' pictures (the Presets page's tiles).
    pictures: thumbnails::Thumbnails,
    /// Each preset's picture file and its texture once loaded, by ID.
    preset_pictures: HashMap<String, (PathBuf, Option<eframe::egui::TextureHandle>)>,
}

/// Held while a workspace writes the library into its folder: the start
/// page's pictures open a workspace of their own on another thread.
static WRITING_LIBRARY: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The name of the FaceControls the preview adds to show an expression.
const PREVIEW_FACE: &str = "Studio Expression Preview";

/// How long a notice stays in the status line.
const NOTICE_FOR: Duration = Duration::from_secs(8);

impl CharacterWorkspace {
    /// Opens the workspace on `description`, its files in `folder` (written
    /// where missing: the humanoid's files and the character library). A
    /// description whose base model the library hasn't is shown on the
    /// default character, with a notice.
    pub fn open(folder: &Path, description: Description, saved: bool) -> Result<Self, String> {
        std::fs::create_dir_all(folder.join(CHARACTERS_FOLDER))
            .map_err(|err| format!("Could not make the Character Studio folder: {err}"))?;
        let writing = WRITING_LIBRARY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        axcenity_project::add_default_humanoid(folder)
            .map_err(|err| format!("Could not write the humanoid's files: {err}"))?;
        axcenity_project::add_character_library(folder)
            .map_err(|err| format!("Could not write the character library: {err}"))?;
        let mut notice = None;
        match find_character_library() {
            Some(shipped) => {
                if let Err(err) = axcenity_project::add_character_packages(folder, &shipped) {
                    notice = Some(format!("Could not copy the character library: {err}"));
                }
            }
            None => {
                notice = Some(
                    "Only the stylized characters are here: the engine's character library \
                     (assets/characters) wasn't found."
                        .to_owned(),
                )
            }
        }
        drop(writing);
        let assets = Assets::new(folder);
        let library = applier::library(&assets);
        let (description, fell_back) = adopt(description, &library);
        let mut pictures = thumbnails::Thumbnails::default();
        pictures.set_folder(Some(folder.to_owned()));
        let mut workspace = Self {
            folder: folder.to_owned(),
            assets,
            world: World::new(),
            character: None,
            animators: Animators::new(),
            saved: saved.then(|| description.clone()),
            description,
            file: None,
            history: History::default(),
            applied: None,
            symmetry: true,
            selection: Selection::default(),
            orbit: Orbit::default(),
            view: Some(ViewPreset::FullBody),
            lighting: Lighting::default(),
            backdrop: Backdrop::default(),
            expression: None,
            canvas: Canvas::default(),
            last_frame: None,
            notice: fell_back.or(notice),
            notice_shown: None,
            leaving: false,
            asking: None,
            saved_files: Vec::new(),
            own_presets: presets::read_own(folder),
            favourites: presets::read_favourites(folder),
            locks: random::default_locks(),
            rolls: 0,
            comparison: Comparison::default(),
            other: None,
            other_floor: None,
            other_applied: None,
            other_canvas: Canvas::default(),
            autosave: Autosave {
                on: true,
                ..Autosave::default()
            },
            recovery: autosave::read_recovery(folder),
            pictures,
            preset_pictures: HashMap::new(),
        };
        workspace.build_world();
        Ok(workspace)
    }

    /// Opens a saved character file (any format up to this version's).
    pub fn open_file(folder: &Path, file: &Path) -> Result<Self, String> {
        let description = read_character(file)?;
        let mut workspace = Self::open(folder, description, true)?;
        workspace.file = Some(file.to_owned());
        recent::remember(folder, file);
        Ok(workspace)
    }

    /// Opens the workspace on a new character from `start` (the default
    /// character when the library hasn't what it names).
    pub fn open_new(folder: &Path, start: &Start) -> Result<Self, String> {
        // The library is written into the folder by `open`; a preset's
        // items need it, so it opens first on a bare description.
        let mut workspace = Self::open(
            folder,
            Description::new("New Character", DEFAULT_BASE),
            false,
        )?;
        let description = workspace.starting(start);
        workspace.description = description.clone();
        workspace.autosave.untouched = Some(description);
        workspace.applied = None;
        workspace.refresh();
        Ok(workspace)
    }

    /// The new character `start` makes, tidied; the default character
    /// when the library hasn't what it names.
    fn starting(&self, start: &Start) -> Description {
        let library = self.library();
        let made = match start {
            Start::Family(base) => library
                .base(base)
                .map(|_| applier::default_description(&library, base)),
            Start::Preset(id) => {
                presets::find(&library, &self.own_presets, id).and_then(|preset| {
                    library.base(&preset.base)?;
                    let mut made = applier::default_description(&library, &preset.base);
                    axcenity_avatar::apply_preset(&mut made, preset, &library).ok()?;
                    Some(made)
                })
            }
        };
        let made = made.unwrap_or_else(|| {
            let base = if library.base(DEFAULT_BASE).is_some() {
                DEFAULT_BASE
            } else {
                axcenity_avatar::V1_BASE
            };
            applier::default_description(&library, base)
        });
        let mut made = tidy(made, &library);
        made.name = "New Character".to_owned();
        made
    }

    /// Starts a new character from `start` in place of this one: not
    /// saved, with an undo of its own.
    pub fn new_character(&mut self, start: &Start) {
        let description = self.starting(start);
        self.start_over(description.clone(), None);
        self.autosave.untouched = Some(description);
    }

    /// Opens the character saved in `file` in place of this one.
    pub fn open_character(&mut self, file: &Path) -> Result<(), String> {
        let description = read_character(file)?;
        let (description, fell_back) = adopt(description, &self.library());
        self.start_over(description, Some(file.to_owned()));
        if fell_back.is_some() {
            self.notice = fell_back;
        }
        recent::remember(&self.folder, file);
        Ok(())
    }

    /// Edits `description` from scratch: a fresh undo, saved when `file`
    /// holds it.
    fn start_over(&mut self, description: Description, file: Option<PathBuf>) {
        self.saved = file.is_some().then(|| description.clone());
        self.description = description;
        self.file = file;
        self.history = History::default();
        self.expression = None;
        self.leaving = false;
        self.asking = None;
        self.autosave.untouched = None;
        self.animators.clear();
        self.applied = None;
        // Held versions belong to the character they were held of.
        self.clear_comparison();
    }

    pub fn description(&self) -> &Description {
        &self.description
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The file the character was opened from or last saved to.
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// The character library in the workspace's folder.
    pub fn library(&self) -> Arc<Library> {
        applier::library(&self.assets)
    }

    #[cfg(test)]
    /// The base model the character is on.
    pub fn base(&self) -> Option<BaseManifest> {
        self.library().base(&self.description.base).cloned()
    }

    /// Whether the description changed since it was saved (a new one always
    /// has).
    pub fn dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.description)
    }

    /// Changes the description as one undo step and shows it. A drag that
    /// was open is closed first.
    pub fn edit(&mut self, change: impl FnOnce(&mut Description)) {
        self.history.commit(&self.description);
        let before = self.description.clone();
        let mut changed = before.clone();
        change(&mut changed);
        let library = self.library();
        self.description = tidy(changed, &library);
        if edit::same_character(&before, &self.description, library.base(&before.base)) {
            // A change of nothing makes no step and keeps redo (audit D-05).
            self.description = before;
        } else if self.history.record(before, &self.description) {
            self.refresh();
        }
    }

    /// A change that is part of a gesture: a drag's changes are one undo
    /// step, closed by its [`Gesture::Commit`]; nothing changed, no step.
    pub fn transact(&mut self, change: impl FnOnce(&mut Description), gesture: Gesture) {
        self.history.begin(&self.description);
        let mut changed = self.description.clone();
        change(&mut changed);
        let library = self.library();
        self.description = tidy(changed, &library);
        let done = gesture == Gesture::Commit;
        if done {
            // A drag that ends where it started changes nothing (D-05).
            if let Some(start) = self.history.start()
                && edit::same_character(start, &self.description, library.base(&start.base))
            {
                self.description = start.clone();
            }
            self.history.commit(&self.description);
        }
        self.refresh();
    }

    /// Whether a drag is going on (its undo step is open).
    pub fn dragging(&self) -> bool {
        self.history.is_open()
    }

    /// Ends a drag going back to where it started.
    pub fn cancel_edit(&mut self) {
        if let Some(start) = self.history.cancel() {
            self.description = start;
            self.refresh();
        }
    }

    #[cfg(test)]
    /// The value of parameter `id` (its default when the description leaves
    /// it out).
    pub fn parameter(&self, id: &str) -> f32 {
        let library = self.library();
        let base = library.base(&self.description.base);
        value_of(&self.description, base, id)
    }

    #[cfg(test)]
    /// Sets parameter `parameter` (and, with symmetry on, its mirror) while
    /// a slider is dragged; `done` ends the drag.
    pub fn set_parameter(&mut self, parameter: &Parameter, value: f32, done: bool) {
        let gesture = if done { Gesture::Commit } else { Gesture::Drag };
        self.set_value(&parameter.id, value, self.symmetry, gesture);
    }

    /// Sets parameter `id` (and its mirror when `mirrored`).
    pub fn set_value(&mut self, id: &str, value: f32, mirrored: bool, gesture: Gesture) {
        let mirror = mirrored
            .then(|| {
                self.library()
                    .base(&self.description.base)?
                    .parameter(id)?
                    .mirror
                    .clone()
            })
            .flatten();
        let id = id.to_owned();
        self.transact(
            |description| {
                description.parameters.insert(id, value);
                if let Some(mirror) = mirror {
                    description.parameters.insert(mirror, value);
                }
            },
            gesture,
        );
    }

    /// Sets value `name` of material slot `slot` (`None`: back to the
    /// material as made).
    pub fn set_material(
        &mut self,
        slot: &str,
        name: &str,
        to: Option<MaterialValue>,
        gesture: Gesture,
    ) {
        let (slot, name) = (slot.to_owned(), name.to_owned());
        self.transact(
            |description| {
                let values = description.materials.entry(slot.clone()).or_default();
                match to {
                    Some(value) => {
                        values.insert(name, value);
                    }
                    None => {
                        values.remove(&name);
                    }
                }
                if values.is_empty() {
                    description.materials.remove(&slot);
                }
            },
            gesture,
        );
    }

    /// Sets makeup layer or marking `id` to `to` (or takes it out), in the
    /// list `marking` says, as one undo step or part of a drag's.
    pub fn set_skin_layer(
        &mut self,
        id: &str,
        marking: bool,
        to: Option<axcenity_avatar::SkinLayer>,
        gesture: Gesture,
    ) {
        let id = id.to_owned();
        self.transact(
            |description| {
                let list = if marking {
                    &mut description.skin.markings
                } else {
                    &mut description.skin.makeup
                };
                match to {
                    Some(layer) => match list.iter_mut().find(|held| held.layer == id) {
                        Some(held) => *held = layer,
                        None => list.push(layer),
                    },
                    None => list.retain(|held| held.layer != id),
                }
            },
            gesture,
        );
    }

    /// Makeup layer or marking `id`, if the description has it.
    pub fn skin_layer(&self, id: &str) -> Option<&axcenity_avatar::SkinLayer> {
        self.description.skin.layer(id)
    }

    /// Value `name` of material slot `slot`, if the description sets it.
    pub fn material_value(&self, slot: &str, name: &str) -> Option<MaterialValue> {
        self.description
            .materials
            .get(slot)
            .and_then(|values| values.get(name))
            .copied()
    }

    #[cfg(test)]
    /// The items the library has for `slot` on this character's base model.
    pub fn items_for(&self, slot: &str) -> Vec<(String, String)> {
        items_for(&self.library(), &self.description.base, slot)
    }

    #[cfg(test)]
    /// The item worn in `slot`, if any.
    pub fn worn_in(&self, slot: &str) -> Option<String> {
        worn_in(&self.library(), &self.description, slot)
    }

    /// Wears `item` in `slot` (taking off what was there), or nothing.
    pub fn wear(&mut self, slot: &str, item: Option<&str>) {
        let library = self.library();
        let slot = slot.to_owned();
        let item = item.map(str::to_owned);
        self.edit(|description| {
            description
                .items
                .retain(|worn| library.item(&worn.id).is_none_or(|it| it.slot != slot));
            if let Some(item) = item {
                description.items.push(WornItem::new(item));
            }
        });
    }

    /// Picks `value` of a parameter with choices (Male or Female) as one
    /// undo step, wearing what the choice wears.
    pub fn choose(&mut self, parameter: &Parameter, value: f32) {
        let library = self.library();
        let parameter = parameter.clone();
        self.edit(|description| choose(description, &library, &parameter, value));
    }

    /// Renames the character, as one undo step (an empty name is "New
    /// Character").
    pub fn rename(&mut self, name: &str) {
        let name = name.trim().to_owned();
        self.edit(|description| description.name = name);
    }

    /// Moves the character onto base model `base`: that body's default
    /// character, keeping the name.
    pub fn set_base(&mut self, base: &str) {
        if base == self.description.base {
            return;
        }
        let library = self.library();
        let name = self.description.name.clone();
        let base = base.to_owned();
        self.edit(|description| {
            *description = applier::default_description(&library, &base);
            description.name = name;
        });
    }

    /// Applies preset `id` (`avatar::apply_preset`: what its kind owns,
    /// the name kept), as one undo step.
    pub fn use_preset(&mut self, id: &str) {
        let library = self.library();
        let Some(preset) = presets::find(&library, &self.own_presets, id).cloned() else {
            self.notice = Some(format!("There's no preset \"{id}\" in this library."));
            return;
        };
        let mut applied = self.description.clone();
        if let Err(problem) = axcenity_avatar::apply_preset(&mut applied, &preset, &library) {
            self.notice = Some(format!(
                "\"{}\" can't be applied: {}",
                preset.name, problem.message
            ));
            return;
        }
        self.edit(|description| *description = applied);
    }

    /// The presets of `kind` for the character's base model, the
    /// library's (`library`, the caller's for the frame) and a person's
    /// own.
    pub fn presets_of<'a>(
        &'a self,
        library: &'a Library,
        kind: PresetKind,
    ) -> Vec<presets::Listed<'a>> {
        presets::of_kind(library, &self.own_presets, &self.description.base, kind)
    }

    /// A person's own presets, by ID.
    pub fn own_presets(&self) -> &BTreeMap<String, Description> {
        &self.own_presets
    }

    /// Saves the character as a person's own preset of `kind` named
    /// `name` (in `presets/`); the status line says how it went.
    pub fn save_as_preset(&mut self, kind: PresetKind, name: &str) {
        let library = self.library();
        match presets::save_own(
            &self.folder,
            &self.description,
            kind,
            name,
            &library,
            &self.own_presets,
        ) {
            Ok((id, preset)) => {
                self.notice = Some(format!(
                    "Saved \"{}\" as a {} preset.",
                    preset.name,
                    kind.name()
                ));
                self.own_presets.insert(id, preset);
            }
            Err(error) => self.notice = Some(error),
        }
    }

    /// Marks preset `id` a favourite, or not.
    pub fn toggle_favourite(&mut self, id: &str) {
        if !self.favourites.remove(id) {
            self.favourites.insert(id.to_owned());
        }
        presets::write_favourites(&self.folder, &self.favourites);
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn undo(&mut self) {
        if let Some(before) = self.history.undo(self.description.clone()) {
            self.description = before;
            self.refresh();
        }
    }

    pub fn redo(&mut self) {
        if let Some(after) = self.history.redo(self.description.clone()) {
            self.description = after;
            self.refresh();
        }
    }

    #[cfg(test)]
    /// Back to the default character of the current body (keeping the
    /// name), as an undo step.
    pub fn reset(&mut self) {
        self.reset_scope(&Scope::All);
    }

    /// Puts `scope` back to the body's defaults, as an undo step.
    pub fn reset_scope(&mut self, scope: &Scope) {
        let library = self.library();
        let Some(base) = library.base(&self.description.base) else {
            return;
        };
        let defaults = applier::default_description(&library, &self.description.base);
        let reset = edit::reset(&self.description, &defaults, base, &library, scope);
        self.edit(|description| *description = reset);
    }

    /// The saved file's path for the character's name.
    pub fn file_path(&self) -> Option<PathBuf> {
        self.file_for(&self.description.name)
    }

    fn file_for(&self, name: &str) -> Option<PathBuf> {
        let name = crate::character_studio::file_name(name)?;
        Some(
            self.folder
                .join(CHARACTERS_FOLDER)
                .join(format!("{name}.character.json")),
        )
    }

    /// Whether `path` is this character's own file.
    fn is_own_file(&self, path: &Path) -> bool {
        self.file.as_deref().is_some_and(|own| same_file(own, path))
    }

    /// Saves the description as `characters/<name>.character.json`
    /// (format 2). Another character's file is never written over: the
    /// character is saved under the first free "<name> 2", "<name> 3"…
    /// (audit D-01). After a rename the old file is moved, not kept beside
    /// the new one (D-03): its Recent entry, pictures and pack places go
    /// with it.
    pub fn save(&mut self) -> Result<PathBuf, String> {
        self.save_reporting().map(|saved| match saved {
            Saved::Written(path) | Saved::Renamed { path, .. } => path,
        })
    }

    /// [`save`](Self::save), saying whether the name had to change.
    pub fn save_reporting(&mut self) -> Result<Saved, String> {
        let wanted = self
            .file_path()
            .ok_or_else(|| "Give the character a name to save it.".to_owned())?;
        let taken = |path: &Path| path.exists() && !self.is_own_file(path);
        let mut renamed_from = None;
        let path = if taken(&wanted) {
            let name = packs::free_name(&self.description.name, |name| {
                self.file_for(name).is_none_or(|path| taken(&path))
            });
            renamed_from = Some(std::mem::replace(&mut self.description.name, name));
            self.file_path()
                .ok_or_else(|| "Give the character a name to save it.".to_owned())?
        } else {
            wanted
        };
        std::fs::write(&path, self.description.to_json())
            .map_err(|err| format!("Could not save {}: {err}", path.display()))?;
        if let Some(old) = self.file.take()
            && !same_file(&old, &path)
            && old.parent() == path.parent()
        {
            self.move_away(&old, &path);
        }
        self.file = Some(path.clone());
        self.saved = Some(self.description.clone());
        // Saved: nothing left to recover.
        self.autosave.written = None;
        autosave::remove_recovery(&self.folder);
        recent::remember(&self.folder, &path);
        self.saved_files.push(path.clone());
        Ok(match renamed_from {
            Some(from) => Saved::Renamed { path, from },
            None => Saved::Written(path),
        })
    }

    /// The character was saved as `new` after a rename: `old` goes, and
    /// its places in packs follow it.
    fn move_away(&self, old: &Path, new: &Path) {
        if let (Some(old_name), Some(new_name)) = (
            old.file_name().and_then(|n| n.to_str()),
            new.file_name().and_then(|n| n.to_str()),
        ) {
            packs::rename_character(&self.folder, old_name, new_name);
        }
        thumbnails::remove_character_pictures(&self.folder, old);
        recent::forget(&self.folder, old);
        if let Err(err) = std::fs::remove_file(old)
            && err.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!("could not remove {}: {err}", old.display());
        }
    }

    /// Shows `description` in the preview in place of the character being
    /// edited, as the workspace would open it (for the start page's
    /// pictures): its undo is left alone.
    pub(crate) fn show_only(&mut self, description: Description) {
        let library = self.library();
        self.description = adopt(description, &library).0;
        self.animators.clear();
        self.applied = None;
        self.refresh();
    }

    /// Builds the preview World: a floor and the default character, with
    /// the description applied.
    fn build_world(&mut self) {
        let mut world = World::new();
        let workspace = world.workspace();
        let character = axcenity_project::default_character(&mut world, Vector3::default());
        let _ = world.set_parent(character, Some(workspace));
        stage::add_floor(&mut world);
        self.world = world;
        self.character = Some(character);
        self.other = None;
        self.other_floor = None;
        self.other_applied = None;
        self.animators.clear();
        self.applied = None;
        self.refresh_previews();
    }

    /// Applies the description to the preview if it changed, every step of
    /// a drag too: the identity is worn live (refinement R2a, ADR-0041), so
    /// a change is new weights, measured at well under a millisecond
    /// (`timing_tests.rs`), and the 120 ms throttle is gone.
    fn refresh(&mut self) {
        let Some(character) = self.character else {
            return;
        };
        // A held version shown in the character's place (comparison).
        let shown = self.shown();
        if let Some((applied, _)) = &self.applied
            && applied == shown
        {
            return;
        }
        let shown = shown.clone();
        match applier::apply_description(&mut self.world, &self.assets, character, &shown) {
            Ok(applied) => {
                if let Some(problem) = applied.problems.first() {
                    let more = applied.problems.len() - 1;
                    self.notice = Some(if more == 0 {
                        problem.message.clone()
                    } else {
                        format!("{} (and {more} more)", problem.message)
                    });
                }
            }
            Err(error) => self.notice = Some(format!("Could not show the character: {error}")),
        }
        self.applied = Some((shown, Instant::now()));
    }

    /// Frames `view`. The Face view aims at the character's head bone as
    /// it's posed, so a tall or short character's face is framed too
    /// (audit D-06).
    pub fn focus(&mut self, view: ViewPreset) {
        self.orbit = view.orbit();
        if view == ViewPreset::Face
            && let Some(head) = self.head()
        {
            // The fixed view looked at the default character's head bone
            // (0.905 of its height) from 0.05 in front of it.
            self.orbit.target = head + Vec3::new(0.0, 0.0, -0.05);
        }
        self.view = Some(view);
    }

    /// Where the preview's head bone is in the world, as it's posed (or at
    /// rest before the first frame).
    pub(crate) fn head(&self) -> Option<Vec3> {
        let body = applier::body_of(&self.world, self.character?)?;
        let instance = self.world.get(body)?;
        let at = crate::gizmo::Pose::of(instance)?;
        let mesh = instance.property("MeshId")?.as_str()?;
        let model = self.assets.model_blocking(mesh)?;
        let rig = model.rig.as_deref()?;
        let bone = rig.bone("Head")?;
        let position = match self.animators.poses().get(&body) {
            Some(posed) => posed.bones().get(bone)?.position,
            None => {
                let mut pose = rig.rest_pose();
                model.settle(&mut pose);
                rig.skeleton(&pose).get(bone)?.position
            }
        };
        let place = axcenity_render::glam::Mat4::from_scale_rotation_translation(
            at.size,
            at.rotation,
            at.position,
        );
        Some(place.transform_point3(position)).filter(|point| point.is_finite())
    }

    /// Shows `name` (a built-in expression) on the preview's face, or none:
    /// a FaceControls of the preview's own on the body, as the capture tool
    /// does. The description is not touched.
    pub fn preview_expression(&mut self, name: Option<String>) {
        let name = name.filter(|name| !name.eq_ignore_ascii_case("Neutral"));
        self.expression = name;
        let Some(body) = self
            .character
            .and_then(|character| applier::body_of(&self.world, character))
        else {
            return;
        };
        let existing = self.world.get(body).and_then(|body| {
            body.children().iter().copied().find(|child| {
                self.world.get(*child).is_some_and(|child| {
                    child.class() == ClassName::FaceControls && child.name() == PREVIEW_FACE
                })
            })
        });
        let Some(name) = self.expression.clone() else {
            if let Some(face) = existing {
                let _ = self.world.destroy(face);
            }
            return;
        };
        let face = match existing {
            Some(face) => face,
            None => {
                let Ok(face) = self.world.create(ClassName::FaceControls) else {
                    return;
                };
                let _ = self.world.set_name(face, PREVIEW_FACE);
                let _ = self.world.set_parent(face, Some(body));
                face
            }
        };
        if let Some(state) = self.world.face_mut(face) {
            let order = state.next_order;
            state.next_order += 1;
            state.expressions = vec![axcenity_core::face::ExpressionPlay {
                name,
                intensity: 1.0,
                target_intensity: 1.0,
                presence: 0.0,
                target_presence: 1.0,
                transition: None,
                weight: 1.0,
                priority: 0,
                asymmetry: 0.0,
                order,
            }];
        }
    }

    /// The MeshParts the preview's body wears, by name, with the body's own
    /// `MeshId` first.
    #[cfg(test)]
    pub fn worn(&self) -> Vec<String> {
        let Some(body) = self
            .character
            .and_then(|character| applier::body_of(&self.world, character))
        else {
            return Vec::new();
        };
        let mesh = |id: InstanceId| {
            self.world
                .get(id)
                .and_then(|part| part.property("MeshId"))
                .and_then(axcenity_core::Value::as_str)
                .map(str::to_owned)
        };
        let mut names: Vec<String> = self
            .world
            .get(body)
            .map(|body| body.children().to_vec())
            .unwrap_or_default()
            .into_iter()
            .filter(|child| mesh(*child).is_some())
            .filter_map(|child| self.world.get(child).map(|part| part.name().to_owned()))
            .collect();
        names.sort();
        names.insert(0, mesh(body).unwrap_or_default());
        names
    }

    /// Moves the preview on by the time since the last frame: the
    /// character idles, and a dragged slider's change is shown.
    pub fn step(&mut self) -> f64 {
        let now = Instant::now();
        let seconds = self
            .last_frame
            .map_or(0.0, |last| now.duration_since(last).as_secs_f64())
            .min(0.1);
        self.last_frame = Some(now);
        self.refresh();
        self.step_by(seconds);
        seconds
    }

    pub(crate) fn step_by(&mut self, seconds: f64) {
        self.animators
            .update(&mut self.world, &self.assets, seconds);
    }

    /// The preview's poses.
    pub fn poses(&self) -> &axcenity_scene::Poses {
        self.animators.poses()
    }
}

/// Reads a character file (any format up to this version's).
pub(crate) fn read_character(file: &Path) -> Result<Description, String> {
    let text = std::fs::read_to_string(file)
        .map_err(|err| format!("Could not read {}: {err}", file.display()))?;
    Description::from_json(&text).map_err(|err| err.to_string())
}

/// `description` ready to edit: tidied, or the default character keeping
/// its name when the library hasn't its body (with a notice saying so).
fn adopt(description: Description, library: &Library) -> (Description, Option<String>) {
    if library.base(&description.base).is_some() {
        return (tidy(description, library), None);
    }
    let base = if library.base(DEFAULT_BASE).is_some() {
        DEFAULT_BASE
    } else {
        axcenity_avatar::V1_BASE
    };
    let notice = format!(
        "\"{}\" is on a body this library hasn't ({}); showing the default character.",
        description.name, description.base
    );
    let mut fallback = applier::default_description(library, base);
    fallback.name = description.name;
    (tidy(fallback, library), Some(notice))
}

/// The value of parameter `id` in `description` (its default when left
/// out), without cloning the manifest: what panels call per slider.
pub(crate) fn value_of(description: &Description, base: Option<&BaseManifest>, id: &str) -> f32 {
    description.parameters.get(id).copied().unwrap_or_else(|| {
        base.and_then(|base| base.parameter(id))
            .map_or(0.0, |parameter| parameter.default)
    })
}

/// The items `library` has for `slot` on base model `base`.
pub(crate) fn items_for(library: &Library, base: &str, slot: &str) -> Vec<(String, String)> {
    library
        .items
        .values()
        .filter(|item| item.slot == slot && item.bases.iter().any(|b| b == base))
        .map(|item| (item.id.clone(), item.label.clone()))
        .collect()
}

/// The item `description` wears in `slot`, if any.
pub(crate) fn worn_in(library: &Library, description: &Description, slot: &str) -> Option<String> {
    description
        .items
        .iter()
        .find(|worn| library.item(&worn.id).is_some_and(|item| item.slot == slot))
        .map(|worn| worn.id.clone())
}

/// Whether `a` and `b` are the same file (on a file system that ignores
/// case, "sam" and "Sam" are).
fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// `description` with every parameter in its base model's range (one with
/// choices at the nearest, wearing what that choice wears), unknown
/// parameters dropped and a name.
fn tidy(mut description: Description, library: &Library) -> Description {
    if let Some(base) = library.base(&description.base) {
        description
            .parameters
            .retain(|id, _| base.parameter(id).is_some());
        for (id, value) in description.parameters.iter_mut() {
            if let Some(parameter) = base.parameter(id) {
                let [low, high] = parameter.range;
                *value = value.clamp(low.min(high), low.max(high));
                if let Some(choice) = parameter.nearest_choice(*value) {
                    *value = choice.value;
                }
            }
        }
        // What a choice wears is always on: it can't be taken off, and an
        // older file without it gets it.
        wear_choices(&mut description, library);
    }
    if description.name.trim().is_empty() {
        description.name = "New Character".to_owned();
    }
    description
}
