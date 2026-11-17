//! Everything a panel can ask of the workspace (refinement plan section
//! 11.3). A panel draws from `&CharacterWorkspace` and returns
//! `Vec<Intent>`; only the shell ([`CharacterWorkspace::apply`]) changes
//! the workspace. So a walkthrough test drives the studio exactly as the
//! panels do, and no panel calls another.
//!
//! Comparison (brief §12), Randomize, presets by ID and the recovery
//! offer came with R1's second part. Comparison, locks and favourites are
//! editor state: their intents never change the description or undo.
//!
//! [`CharacterWorkspace::apply`]: super::CharacterWorkspace::apply

use std::path::PathBuf;

use axcenity_avatar::{MaterialValue, PresetKind};

use super::edit::Scope;
use super::random::RandomScope;
use super::selection::Level;
use super::stage::{Backdrop, Lighting, ViewPreset};
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
    FocusView(ViewPreset),
    SetLighting(Lighting),
    SetBackdrop(Backdrop),
    /// Shows a built-in expression on the preview's face (`None`: none).
    /// Preview only: never in the description.
    PreviewExpression(Option<String>),
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
                | Intent::SetMaterial { .. }
                | Intent::SetBase(_)
                | Intent::Rename(_)
                | Intent::ApplyPreset(_)
                | Intent::Undo
                | Intent::Redo
                | Intent::Reset(_)
                | Intent::Randomize(_)
        )
    }
}
