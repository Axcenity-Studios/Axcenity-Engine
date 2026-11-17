//! Everything a panel can ask of the workspace (refinement plan section
//! 11.3). A panel draws from `&CharacterWorkspace` and returns
//! `Vec<Intent>`; only the shell ([`CharacterWorkspace::apply`]) changes
//! the workspace. So a walkthrough test drives the studio exactly as the
//! panels do, and no panel calls another.
//!
//! Capture and comparison (brief §12) come with R1's second part, as new
//! variants here.
//!
//! [`CharacterWorkspace::apply`]: super::CharacterWorkspace::apply

use std::path::PathBuf;

use axcenity_avatar::MaterialValue;

use super::Category;
use super::edit::Scope;
use super::selection::Level;
use super::stage::{Backdrop, Lighting, ViewPreset};

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
    /// Starts again from preset `index` of `presets()`, keeping the name.
    ApplyPreset(usize),
    /// A new character from preset `index`, not saved yet.
    New(usize),
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
    /// Back to the start screen (the editor asks to save first).
    Home,
}
