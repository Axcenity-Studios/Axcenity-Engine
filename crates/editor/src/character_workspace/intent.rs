//! Everything a panel can ask of the workspace (refinement plan section
//! 11.3). A panel draws from `&CharacterWorkspace` and returns
//! `Vec<Intent>`; only the shell ([`CharacterWorkspace::apply`]) changes
//! the workspace. So a walkthrough test drives the studio exactly as the
//! panels do, and no panel calls another.
//!
//! Comparison (brief §12), Randomize, presets by ID and the recovery
//! offer came with R1's second part. Comparison, locks and favourites are
//! editor state: their intents never change the description or undo.
//! The asymmetry tools (R4) are edits, one undo step each; the preview's
//! pose and motion (R9) is editor state, as an expression is, and so are
//! the Expressions page's face controls and face preview (R9).
//!
//! The Hair and Clothing browsers (R6, R7; `browser.rs`) add `Browse`
//! (search, filters, "Show drafts", the tile hovered or picked: editor
//! state, the preview shown in the character's place), `EndPreview`,
//! item favourites, and the worn item's own controls (`SetItemVariant`,
//! `SetItemMaterial`, `SetItemParameter`), which are edits. `Wear` stays
//! the one way an item goes on: one undo step. The Clothing page's online
//! wardrobe adds `Wardrobe` (`wardrobe.rs`): its list and its downloads
//! into the library, which are editor state.
//!
//! [`CharacterWorkspace::apply`]: super::CharacterWorkspace::apply

use std::path::PathBuf;

use axcenity_avatar::{MaterialValue, PresetKind, SkinLayer};

use super::browser::Browse;
use super::edit::Scope;
use super::motion::Motion;
use super::random::RandomScope;
use super::selection::Level;
use super::sides::{SideTool, SidesOf};
use super::stage::{Backdrop, Lighting, ViewPreset};
use super::wardrobe::Ask;
use super::{Category, Start};

/// Where a change is in its gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gesture {
    /// Part of a drag that goes on: the drag's first change opens one undo
    /// step, and the preview may lag a little (each apply bakes).
    Drag,
    /// The last change of a drag, or a change on its own (a click, a typed
    /// number): closes the step and shows it at once.
    Commit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// Sets one parameter.
    SetParameter {
        id: String,
        value: f32,
        gesture: Gesture,
    },
    /// Sets a parameter and its mirror to the same value (Symmetry).
    SetMirrored {
        id: String,
        value: f32,
        gesture: Gesture,
    },
    /// Picks one of a parameter's choices (Male or Female), wearing what
    /// it wears.
    Choose {
        id: String,
        value: f32,
    },
    /// Wears `item` in `slot`, or nothing.
    Wear {
        slot: String,
        item: Option<String>,
    },
    /// Sets (or, with `None`, clears) value `value` of material `slot`.
    SetMaterial {
        slot: String,
        value: String,
        to: Option<MaterialValue>,
        gesture: Gesture,
    },
    /// Sets makeup layer or marking `id` (`marking`: which list of the
    /// description's `skin` block) to `to` whole, or with `None` takes it
    /// out (R5, ADR-0042). Turning a layer off is `to` with `enabled`
    /// false, which keeps its settings.
    SetSkinLayer {
        id: String,
        marking: bool,
        to: Option<SkinLayer>,
        gesture: Gesture,
    },
    /// Moves the character onto another base model.
    SetBase(String),
    /// Renames the character (one undo step).
    Rename(String),
    /// Applies the preset with this ID (what its kind owns; the name is
    /// kept), as one undo step.
    ApplyPreset(String),
    /// Saves the character as a person's own preset of a kind.
    SaveAsPreset {
        kind: PresetKind,
        name: String,
    },
    /// Marks a preset a favourite, or not (editor state).
    ToggleFavourite(String),
    /// Marks an item (a hairstyle, a garment) a favourite, or not
    /// (editor state, beside the preset favourites).
    ToggleItemFavourite(String),
    /// A browser's search, filters, drafts, or the tile hovered or picked
    /// (editor state: a tile is previewed on the character, never worn).
    Browse(Browse),
    /// Takes a tile's preview off: the character as it is (Esc, leaving
    /// the page).
    EndPreview,
    /// The online wardrobe (`wardrobe.rs`): asks for its list, downloads
    /// an item into the library or takes one out again (editor state and
    /// the library's files: never the character, never an undo step).
    Wardrobe(Ask),
    /// Picks variant `variant` of the worn item `item` (`None`: as made),
    /// as one undo step. The colours and parameters the variant sets are
    /// the variant's again.
    SetItemVariant {
        item: String,
        variant: Option<String>,
    },
    /// Sets (or with `None` clears) values of the worn item `item`'s
    /// material slots `slots`, together (a colour role's slots, a hair
    /// colour's melanin and redness): one undo step, or one per drag.
    SetItemMaterial {
        item: String,
        slots: Vec<String>,
        values: Vec<(String, Option<MaterialValue>)>,
        gesture: Gesture,
    },
    /// Sets the worn item `item`'s own parameter `id` (a hairstyle's
    /// length).
    SetItemParameter {
        item: String,
        id: String,
        value: f32,
        gesture: Gesture,
    },
    /// A new character, not saved yet: a family's default or a preset.
    New(Start),
    /// Opens a saved character file in place of this one.
    Open(PathBuf),
    Save,
    Undo,
    Redo,
    /// Ends a drag going back to where it started (Esc).
    CancelEdit,
    /// Puts a scope back to the default.
    Reset(Scope),
    PickCategory(Category),
    /// Picks a feature of the category (`None`: the whole category).
    PickFeature(Option<String>),
    PickLevel(Level),
    SetSymmetry(bool),
    /// Turns editing on the character in the viewport on or off (R2): the
    /// hover highlight, picking by click and the handles. Off, the
    /// viewport is the camera only (editor state).
    SetHandles(bool),
    /// Frames a feature (R2: a feature clicked on the character, when it's
    /// small on the screen or partly off it).
    FocusFeature(String),
    FocusView(ViewPreset),
    SetLighting(Lighting),
    SetBackdrop(Backdrop),
    /// Shows a built-in expression on the preview's face (`None` or
    /// `Neutral`: none), starting the face controls from its own values.
    /// Preview only: never in the description.
    PreviewExpression(Option<String>),
    /// Plays a pose or motion on the preview (Still, Idle, Walk, Run, a
    /// gesture). Preview only: never in the description.
    PreviewMotion(Motion),
    /// Holds face control `name` (a standard control, `Jaw_Open`) at
    /// `value` (0 to 1) on the preview's face, over the expression shown;
    /// `None` lets the expression's own value show again (R9). Preview
    /// only: never in the description.
    PreviewFaceControl {
        name: String,
        value: Option<f32>,
    },
    /// Turns the face preview on or off (R9): the camera close on the
    /// face, the body still, blinking and idle motion off, the neutral
    /// light, the expression held. Off puts back the view, light and
    /// motion it found (editor state).
    FacePreview(bool),
    /// In the face preview: blinking, and idle motion (the face's and the
    /// body's), each off unless asked for.
    FacePreviewMotion {
        blink: bool,
        idle: bool,
    },
    /// Mirrors a side onto the other, or resets a side, for a feature's
    /// left and right pairs or one pair (one undo step).
    Sides {
        tool: SideTool,
        of: SidesOf,
    },
    /// Draws the scope again, keeping what's locked (one undo step).
    Randomize(RandomScope),
    /// Locks or unlocks parameters for Randomize (editor state).
    SetLock {
        ids: Vec<String>,
        locked: bool,
    },
    /// Holds the current character as the next of A, B and C.
    Capture,
    /// Shows a held version (`None`: the current character).
    ShowHeld(Option<usize>),
    /// A held version beside the current one, or not.
    SideBySide(bool),
    /// Makes a held version the character (one undo step).
    KeepHeld(usize),
    /// Drops a held version.
    DiscardHeld(usize),
    /// The answer to "Recover the unsaved character?".
    Recover(bool),
    /// Back to the start screen (the editor asks to save first).
    Home,
    /// Back to the Game Editor, the edited character applied to the game
    /// character the workspace was opened on (the round trip, `game.rs`).
    Done,
    /// Back to the Game Editor, the game character left as it was.
    Cancel,
}

impl Intent {
    /// Whether it changes the character (so a held version shown in its
    /// place gives way to it first: an edit is always seen).
    pub fn edits(&self) -> bool {
        matches!(
            self,
            Intent::SetParameter { .. }
                | Intent::SetMirrored { .. }
                | Intent::Choose { .. }
                | Intent::Wear { .. }
                | Intent::SetItemVariant { .. }
                | Intent::SetItemMaterial { .. }
                | Intent::SetItemParameter { .. }
                | Intent::SetMaterial { .. }
                | Intent::SetSkinLayer { .. }
                | Intent::SetBase(_)
                | Intent::Rename(_)
                | Intent::ApplyPreset(_)
                | Intent::Undo
                | Intent::Redo
                | Intent::Reset(_)
                | Intent::Randomize(_)
                | Intent::Sides { .. }
        )
    }
}
