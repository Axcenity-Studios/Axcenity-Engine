//! Content states (Paul's decision 12 of 2026-11-18, ADR-0043,
//! `docs/development/character-studio/asset-quality.md` §5): every item
//! is Production, Legacy or Draft ([`Quality`], read through
//! [`ItemManifest::state`]). This file holds the rules that aren't the
//! manifest's own: what a shipped item must declare and which slots
//! Randomize may draw in. Kept apart from the other checks so the
//! licensed-asset intake's provenance check merges beside it.

use std::collections::BTreeSet;

use crate::{ItemManifest, Library, Problem, Quality};

/// The problems with `item`'s declared state, for an item the engine
/// ships (its library, `assets/characters/`): it must say its state
/// (`quality`) and say one of the three. A project's own older manifests
/// may leave it out (read as Legacy), so this is not part of
/// [`ItemManifest::validate`].
pub fn check_declared_state(item: &ItemManifest) -> Vec<Problem> {
    let problem = match item.quality {
        Some(Quality::Production | Quality::Legacy | Quality::Draft) => return Vec::new(),
        None => "doesn't say its content state: set `quality` to `production`, `legacy` or `draft`",
        Some(Quality::Other) => {
            "says a content state this engine doesn't know: use `production`, `legacy` or `draft`"
        }
    };
    vec![Problem::new(format!("{}: quality", item.id), problem)]
}

/// The slots of base `base` that have at least one Production item made
/// for it: the only slots Randomize draws again. A slot without one keeps
/// what the character wore there.
pub fn production_slots(library: &Library, base: &str) -> BTreeSet<String> {
    library
        .items
        .values()
        .filter(|item| item.is_production() && item.bases.iter().any(|b| b == base))
        .map(|item| item.slot.clone())
        .collect()
}
