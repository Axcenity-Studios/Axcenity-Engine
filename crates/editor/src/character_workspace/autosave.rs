//! Autosave (CO20): while a character has unsaved work, a recovery copy is
//! written to `recovery/character.json` in the workspace's folder, outside
//! `characters/` so it's never listed as a saved character. It is written
//! when the character changes, at most every [`AUTOSAVE_EVERY`] and never
//! during a drag; it is removed on Save and when the workspace closes
//! cleanly (Home, Don't Save, quitting), so it is only left by a crash.
//! The next time the studio opens it offers it: "Recover the unsaved
//! character?".
//!
//! The clock is passed in ([`CharacterWorkspace::autosave_step`]), so tests
//! drive it without waiting.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axcenity_avatar::Description;
use serde::{Deserialize, Serialize};

use super::{CHARACTERS_FOLDER, CharacterWorkspace, read_character};

/// The recovery file, in the workspace's folder.
pub const RECOVERY_FILE: &str = "recovery/character.json";
/// Unsaved work is written at most this often.
pub const AUTOSAVE_EVERY: Duration = Duration::from_secs(3);
const RECOVERY_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct RecoveryFile {
    format_version: u32,
    /// The saved file the character was opened from or last saved to, by
    /// its name in `characters/` (none for a new one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    /// The character as it was, a format 2 description.
    character: serde_json::Value,
}

/// A character left by a crash, offered when the studio opens.
#[derive(Clone, Debug, PartialEq)]
pub struct Recovery {
    pub description: Description,
    /// Its saved file, if it had one that's still there.
    pub file: Option<PathBuf>,
}

/// The workspace's autosave state (editor state, never in a description).
#[derive(Clone, Debug, Default)]
pub(super) struct Autosave {
    /// Off for the start page's picture painter, which shows characters
    /// without editing them.
    pub on: bool,
    /// What the recovery file holds, written by this workspace.
    pub written: Option<Description>,
    /// When it was last written.
    pub at: Option<Instant>,
    /// A new character as it started: not work until it changes.
    pub untouched: Option<Description>,
}

/// The recovery file of the workspace in `folder`, if a crash left one.
pub fn read_recovery(folder: &Path) -> Option<Recovery> {
    let text = std::fs::read_to_string(folder.join(RECOVERY_FILE)).ok()?;
    let file: RecoveryFile = serde_json::from_str(&text).ok()?;
    if file.format_version > RECOVERY_VERSION {
        return None;
    }
    let description = Description::from_json(&file.character.to_string()).ok()?;
    let saved = file
        .file
        .filter(|name| !name.contains(['/', '\\']))
        .map(|name| folder.join(CHARACTERS_FOLDER).join(name))
        .filter(|path| path.is_file());
    Some(Recovery {
        description,
        file: saved,
    })
}

fn write_recovery(folder: &Path, description: &Description, file: Option<&Path>) {
    let path = folder.join(RECOVERY_FILE);
    let Ok(character) = serde_json::from_str(&description.to_json()) else {
        return;
    };
    let recovery = RecoveryFile {
        format_version: RECOVERY_VERSION,
        file: file
            .and_then(|file| file.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_owned),
        character,
    };
    let text = serde_json::to_string_pretty(&recovery).expect("plain data");
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            // Whole or not at all: a crash while writing keeps the last one.
            let partial = path.with_extension("json.partial");
            std::fs::write(&partial, text)?;
            std::fs::rename(&partial, &path)
        });
    if let Err(err) = written {
        log::warn!("could not write Character Studio's recovery file: {err}");
    }
}

/// Removes the recovery file of the workspace in `folder`.
pub fn remove_recovery(folder: &Path) {
    if let Err(err) = std::fs::remove_file(folder.join(RECOVERY_FILE))
        && err.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("could not remove Character Studio's recovery file: {err}");
    }
}

impl CharacterWorkspace {
    /// Whether the character holds work a crash would lose: it differs
    /// from its saved file, and a new one has changed since it started.
    pub fn unsaved_work(&self) -> bool {
        self.dirty() && self.autosave.untouched.as_ref() != Some(&self.description)
    }

    /// The autosave's step at `now` (each frame): writes the recovery copy
    /// when the work changed and the last write is old enough, removes it
    /// when there's no work left. Nothing while a recovery is offered or a
    /// drag is going on.
    pub fn autosave_step(&mut self, now: Instant) {
        if !self.autosave.on || self.recovery.is_some() || self.dragging() {
            return;
        }
        if !self.unsaved_work() {
            if self.autosave.written.take().is_some() {
                remove_recovery(&self.folder);
            }
            return;
        }
        let due = self
            .autosave
            .at
            .is_none_or(|at| now.saturating_duration_since(at) >= AUTOSAVE_EVERY);
        if self.autosave.written.as_ref() != Some(&self.description) && due {
            write_recovery(&self.folder, &self.description, self.file.as_deref());
            self.autosave.written = Some(self.description.clone());
            self.autosave.at = Some(now);
        }
    }

    /// The answer to "Recover the unsaved character?": `true` takes the
    /// recovered character up (unsaved, to its own file if it had one),
    /// `false` throws it away.
    pub fn recover(&mut self, take: bool) {
        let Some(recovery) = self.recovery.take() else {
            return;
        };
        if !take {
            remove_recovery(&self.folder);
            return;
        }
        let library = self.library();
        let (description, fell_back) = super::adopt(recovery.description, &library);
        let saved = recovery
            .file
            .as_deref()
            .and_then(|file| read_character(file).ok())
            .map(|saved| super::adopt(saved, &library).0);
        self.start_over(description, recovery.file);
        self.saved = saved;
        self.autosave.untouched = None;
        self.notice = Some(fell_back.unwrap_or_else(|| {
            format!(
                "Recovered \"{}\". Save it to keep it.",
                self.description.name
            )
        }));
    }
}

impl Drop for CharacterWorkspace {
    /// A clean close removes the recovery file this workspace wrote; a
    /// panic (a crash) leaves it. A crash's file that was offered and not
    /// answered is left for next time.
    fn drop(&mut self) {
        if self.autosave.on && self.autosave.written.is_some() && !std::thread::panicking() {
            remove_recovery(&self.folder);
        }
    }
}
