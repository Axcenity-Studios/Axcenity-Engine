//! The Character Studio workspace (ADR-0031; Character Studio overhaul
//! CO12, `docs/development/character-studio/architecture.md` sections 1,
//! 6, 7 and 15): a character editor of its own, opened from the start
//! screen's Character Studio page with no project open.
//!
//! It edits a character [`Description`] (format 2; a format 1 file, CS2's
//! `Look`, migrates when it's opened) on a base model of the character
//! library: the realistic family (ADR-0032). The stylized body isn't
//! offered for new characters ([`offers_base`]); a saved character on it
//! still opens. Its controls come from the base model's manifest (the
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
//! saved characters, `characters/<name>.character.json`.
//!
//! `ui.rs` draws it; `stage.rs` is the viewport (camera, lighting,
//! backdrop and drawing).

mod stage;
mod ui;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axcenity_anim::Animators;
use axcenity_assets::Assets;
use axcenity_avatar::{BaseManifest, Description, Library, Parameter, WornItem};
use axcenity_core::{InstanceId, Vector3, World};
use axcenity_editor_tools::preview::Canvas;
use axcenity_gameplay::description as applier;

pub use stage::{Backdrop, Lighting, Orbit};
pub use ui::WorkspaceAction;

/// The command-line flag that opens the editor straight into the workspace.
pub const FLAG: &str = "--character-studio";

/// The base model a new character starts on: the semi-realistic family is
/// the default (ADR-0032).
pub const DEFAULT_BASE: &str = "axcenity/realistic";

/// While a slider is dragged, the preview is applied again at most this
/// often (each new identity is baked into the character's meshes); it is
/// applied once more when the drag ends.
const DRAG_APPLY_EVERY: Duration = Duration::from_millis(120);

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

/// A starting point on the Presets page and the start screen: plain data,
/// so the start screen can list them without a library.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    pub about: &'static str,
    pub base: &'static str,
    pub parameters: &'static [(&'static str, f32)],
    /// The items worn; `None` wears the base model's defaults.
    pub items: Option<&'static [&'static str]>,
}

impl Preset {
    /// The preset as a description named `name`.
    pub fn description(&self, name: &str, library: &Library) -> Description {
        let mut description = applier::default_description(library, self.base);
        description.name = name.to_owned();
        let base = library.base(self.base);
        for (id, value) in self.parameters {
            match base.and_then(|base| base.parameter(id)) {
                Some(parameter) if self.items.is_none() => {
                    choose(&mut description, library, parameter, *value)
                }
                _ => {
                    description.parameters.insert((*id).to_owned(), *value);
                }
            }
        }
        if let Some(items) = self.items {
            description.items = items.iter().map(|id| WornItem::new(*id)).collect();
        }
        description
    }

    /// Whether `description` is this preset (whatever its name).
    fn matches(&self, description: &Description, library: &Library) -> bool {
        let mut mine = description.clone();
        mine.name = String::new();
        mine == self.description("", library)
    }
}

/// The presets, all on the realistic body (the stylized one is no longer
/// offered for new characters). The first is the default character. A
/// preset that sets a parameter with choices (Male or Female) wears what
/// that choice wears unless it names its own items.
pub fn presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "Male",
            about: "The realistic male body, as it's built: a start for any character.",
            base: DEFAULT_BASE,
            parameters: &[],
            items: None,
        },
        Preset {
            name: "Female",
            about: "The realistic female body, as it's built.",
            base: DEFAULT_BASE,
            parameters: &[("frame", 1.0)],
            items: None,
        },
        Preset {
            name: "Athletic male",
            about: "Lean and muscular.",
            base: DEFAULT_BASE,
            parameters: &[("muscle", 0.7), ("fat", -0.4)],
            items: None,
        },
        Preset {
            name: "Athletic female",
            about: "Lean and toned.",
            base: DEFAULT_BASE,
            parameters: &[("frame", 1.0), ("muscle", 0.6), ("fat", -0.4)],
            items: None,
        },
        Preset {
            name: "Heavyset male",
            about: "A heavier build.",
            base: DEFAULT_BASE,
            parameters: &[("fat", 0.8), ("muscle", 0.2)],
            items: None,
        },
        Preset {
            name: "Heavyset female",
            about: "A heavier build.",
            base: DEFAULT_BASE,
            parameters: &[("frame", 1.0), ("fat", 0.8), ("muscle", 0.2)],
            items: None,
        },
    ]
}

/// Whether Character Studio offers base model `id` for new characters:
/// every base but the stylized one, unless the library has nothing else.
pub fn offers_base(library: &Library, id: &str) -> bool {
    id != axcenity_avatar::V1_BASE || library.bases.keys().all(|base| base == id)
}

/// Sets `parameter` to `value` in `description` and, if the parameter has
/// choices, wears what the chosen one names (in their slots), taking off
/// what the other choices name.
fn choose(description: &mut Description, library: &Library, parameter: &Parameter, value: f32) {
    description.parameters.insert(parameter.id.clone(), value);
    let Some(chosen) = parameter.nearest_choice(value) else {
        return;
    };
    let others: Vec<&String> = parameter
        .choices
        .iter()
        .filter(|choice| *choice != chosen)
        .flat_map(|choice| &choice.items)
        .filter(|id| !chosen.items.contains(id))
        .collect();
    description.items.retain(|worn| !others.contains(&&worn.id));
    for id in &chosen.items {
        let Some(item) = library.item(id) else {
            continue;
        };
        if !item.bases.contains(&description.base) {
            continue;
        }
        let slot = item.slot.clone();
        description
            .items
            .retain(|worn| library.item(&worn.id).is_none_or(|it| it.slot != slot));
        description.items.push(WornItem::new(id.clone()));
    }
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

/// The categories down the left of the workspace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    #[default]
    Presets,
    Body,
    Face,
    Hair,
    Clothing,
    Stage,
}

impl Category {
    pub const ALL: [Category; 6] = [
        Category::Presets,
        Category::Body,
        Category::Face,
        Category::Hair,
        Category::Clothing,
        Category::Stage,
    ];

    /// The manifest's category this one shows, if it shows one.
    pub fn data(self) -> Option<axcenity_avatar::Category> {
        use axcenity_avatar::Category as Data;
        match self {
            Category::Body => Some(Data::Body),
            Category::Face => Some(Data::Face),
            Category::Hair => Some(Data::Hair),
            Category::Clothing => Some(Data::Clothing),
            Category::Presets | Category::Stage => None,
        }
    }
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
    history: Vec<Description>,
    future: Vec<Description>,
    /// A slider being dragged: its edit is one undo step.
    dragging: bool,
    /// What the preview shows, and when it was applied.
    applied: Option<(Description, Instant)>,
    /// Sliders with a mirror move it too.
    pub symmetry: bool,
    /// Show the manifest's advanced parameters.
    pub advanced: bool,
    pub category: Category,
    pub orbit: Orbit,
    pub lighting: Lighting,
    pub backdrop: Backdrop,
    canvas: Canvas,
    last_frame: Option<Instant>,
    /// A line for the status bar: what was saved, what failed.
    pub notice: Option<String>,
    /// Home was asked for with unsaved changes: the editor asks to save.
    pub leaving: bool,
}

impl CharacterWorkspace {
    /// Opens the workspace on `description`, its files in `folder` (written
    /// where missing: the humanoid's files and the character library). A
    /// description whose base model the library hasn't is shown on the
    /// default character, with a notice.
    pub fn open(folder: &Path, description: Description, saved: bool) -> Result<Self, String> {
        std::fs::create_dir_all(folder.join(CHARACTERS_FOLDER))
            .map_err(|err| format!("Could not make the Character Studio folder: {err}"))?;
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
        let assets = Assets::new(folder);
        let library = applier::library(&assets);
        let mut description = description;
        if library.base(&description.base).is_none() {
            let base = if library.base(DEFAULT_BASE).is_some() {
                DEFAULT_BASE
            } else {
                axcenity_avatar::V1_BASE
            };
            notice = Some(format!(
                "\"{}\" is on a body this library hasn't ({}); showing the default character.",
                description.name, description.base
            ));
            let name = description.name.clone();
            description = applier::default_description(&library, base);
            description.name = name;
        }
        let description = tidy(description, &library);
        let mut workspace = Self {
            folder: folder.to_owned(),
            assets,
            world: World::new(),
            character: None,
            animators: Animators::new(),
            saved: saved.then(|| description.clone()),
            description,
            history: Vec::new(),
            future: Vec::new(),
            dragging: false,
            applied: None,
            symmetry: true,
            advanced: false,
            category: Category::default(),
            orbit: Orbit::default(),
            lighting: Lighting::default(),
            backdrop: Backdrop::default(),
            canvas: Canvas::default(),
            last_frame: None,
            notice,
            leaving: false,
        };
        workspace.build_world();
        Ok(workspace)
    }

    /// Opens a saved character file (any format up to this version's).
    pub fn open_file(folder: &Path, file: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(file)
            .map_err(|err| format!("Could not read {}: {err}", file.display()))?;
        let description = Description::from_json(&text).map_err(|err| err.to_string())?;
        Self::open(folder, description, true)
    }

    /// Opens preset `index` of [`presets`] (the default character when
    /// there's no such preset).
    pub fn open_preset(folder: &Path, index: usize) -> Result<Self, String> {
        let preset = presets().into_iter().nth(index).unwrap_or(presets()[0]);
        // The library is written into the folder by `open`; a preset's
        // items need it, so it opens first on a bare description.
        let mut workspace = Self::open(
            folder,
            Description::new("New Character", preset.base),
            false,
        )?;
        let library = workspace.library();
        if library.base(preset.base).is_some() {
            workspace.description = tidy(preset.description("New Character", &library), &library);
            workspace.applied = None;
            workspace.refresh(true);
        }
        Ok(workspace)
    }

    pub fn description(&self) -> &Description {
        &self.description
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// The character library in the workspace's folder.
    pub fn library(&self) -> Arc<Library> {
        applier::library(&self.assets)
    }

    /// The base model the character is on.
    pub fn base(&self) -> Option<BaseManifest> {
        self.library().base(&self.description.base).cloned()
    }

    /// Whether the description changed since it was saved (a new one always
    /// has).
    pub fn dirty(&self) -> bool {
        self.saved.as_ref() != Some(&self.description)
    }

    /// Changes the description as one undo step and shows it.
    pub fn edit(&mut self, change: impl FnOnce(&mut Description)) {
        let before = self.description.clone();
        let mut changed = before.clone();
        change(&mut changed);
        self.description = tidy(changed, &self.library());
        if self.description != before {
            self.history.push(before);
            self.future.clear();
            self.refresh(true);
        }
    }

    /// A slider's change: the whole drag is one undo step (`done` ends it).
    pub fn drag(&mut self, change: impl FnOnce(&mut Description), done: bool) {
        if !self.dragging {
            self.history.push(self.description.clone());
            self.future.clear();
            self.dragging = true;
        }
        let mut changed = self.description.clone();
        change(&mut changed);
        self.description = tidy(changed, &self.library());
        if done {
            self.dragging = false;
            if self.history.last() == Some(&self.description) {
                self.history.pop();
            }
        }
        self.refresh(done);
    }

    /// The value of parameter `id` (its default when the description leaves
    /// it out).
    pub fn parameter(&self, id: &str) -> f32 {
        let default = self
            .base()
            .and_then(|base| base.parameter(id).map(|parameter| parameter.default))
            .unwrap_or_default();
        self.description
            .parameters
            .get(id)
            .copied()
            .unwrap_or(default)
    }

    /// Sets parameter `parameter` (and, with symmetry on, its mirror) while
    /// a slider is dragged; `done` ends the drag.
    pub fn set_parameter(&mut self, parameter: &Parameter, value: f32, done: bool) {
        let mirror = parameter.mirror.clone().filter(|_| self.symmetry);
        let id = parameter.id.clone();
        self.drag(
            |description| {
                description.parameters.insert(id, value);
                if let Some(mirror) = mirror {
                    description.parameters.insert(mirror, value);
                }
            },
            done,
        );
    }

    /// The items the library has for `slot` on this character's base model.
    pub fn items_for(&self, slot: &str) -> Vec<(String, String)> {
        let library = self.library();
        library
            .items
            .values()
            .filter(|item| item.slot == slot && item.bases.contains(&self.description.base))
            .map(|item| (item.id.clone(), item.label.clone()))
            .collect()
    }

    /// The item worn in `slot`, if any.
    pub fn worn_in(&self, slot: &str) -> Option<String> {
        let library = self.library();
        self.description
            .items
            .iter()
            .find(|worn| library.item(&worn.id).is_some_and(|item| item.slot == slot))
            .map(|worn| worn.id.clone())
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

    /// Starts again from `preset`, keeping the name, as an undo step.
    pub fn use_preset(&mut self, preset: &Preset) {
        let library = self.library();
        if library.base(preset.base).is_none() {
            self.notice = Some(format!(
                "This library hasn't the body \"{}\" is on.",
                preset.name
            ));
            return;
        }
        let name = self.description.name.clone();
        self.edit(|description| *description = preset.description(&name, &library));
    }

    /// Whether the character is `preset` (whatever its name).
    pub fn is_preset(&self, preset: &Preset) -> bool {
        preset.matches(&self.description, &self.library())
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(before) = self.history.pop() {
            self.future
                .push(std::mem::replace(&mut self.description, before));
            self.dragging = false;
            self.refresh(true);
        }
    }

    pub fn redo(&mut self) {
        if let Some(after) = self.future.pop() {
            self.history
                .push(std::mem::replace(&mut self.description, after));
            self.refresh(true);
        }
    }

    /// Back to the default character of the current body (keeping the
    /// name), as an undo step.
    pub fn reset(&mut self) {
        let library = self.library();
        let name = self.description.name.clone();
        let base = self.description.base.clone();
        self.edit(|description| {
            *description = applier::default_description(&library, &base);
            description.name = name;
        });
    }

    /// The saved file's path for the character's name.
    pub fn file_path(&self) -> Option<PathBuf> {
        let name = crate::character_studio::file_name(&self.description.name)?;
        Some(
            self.folder
                .join(CHARACTERS_FOLDER)
                .join(format!("{name}.character.json")),
        )
    }

    /// Saves the description as `characters/<name>.character.json`
    /// (format 2).
    pub fn save(&mut self) -> Result<PathBuf, String> {
        let path = self
            .file_path()
            .ok_or_else(|| "Give the character a name to save it.".to_owned())?;
        std::fs::write(&path, self.description.to_json())
            .map_err(|err| format!("Could not save {}: {err}", path.display()))?;
        self.saved = Some(self.description.clone());
        Ok(path)
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
        self.animators.clear();
        self.applied = None;
        self.refresh(true);
    }

    /// Applies the description to the preview if it changed: at once when
    /// `now`, else at most every [`DRAG_APPLY_EVERY`] while a slider is
    /// dragged.
    fn refresh(&mut self, now: bool) {
        let Some(character) = self.character else {
            return;
        };
        if let Some((applied, at)) = &self.applied {
            if *applied == self.description {
                return;
            }
            if !now && at.elapsed() < DRAG_APPLY_EVERY {
                return;
            }
        }
        match applier::apply_description(
            &mut self.world,
            &self.assets,
            character,
            &self.description,
        ) {
            Ok(applied) => {
                if let Some(problem) = applied.problems.first() {
                    self.notice = Some(problem.message.clone());
                }
            }
            Err(error) => self.notice = Some(format!("Could not show the character: {error}")),
        }
        self.applied = Some((self.description.clone(), Instant::now()));
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
        self.refresh(!self.dragging);
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

/// `description` with every parameter in its base model's range (one with
/// choices at the nearest), unknown parameters dropped and a name.
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
    }
    if description.name.trim().is_empty() {
        description.name = "New Character".to_owned();
    }
    description
}

/// The saved characters in `folder`, newest first: (name, path).
pub fn saved_characters(folder: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(folder.join(CHARACTERS_FOLDER)) else {
        return Vec::new();
    };
    let mut found: Vec<(std::time::SystemTime, String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path
                .file_name()?
                .to_str()?
                .strip_suffix(".character.json")?;
            let modified = entry.metadata().and_then(|m| m.modified()).ok()?;
            let shown = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| Description::from_json(&text).ok())
                .map_or_else(|| name.to_owned(), |description| description.name);
            Some((modified, shown, path))
        })
        .collect();
    found.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    found
        .into_iter()
        .map(|(_, name, path)| (name, path))
        .collect()
}
