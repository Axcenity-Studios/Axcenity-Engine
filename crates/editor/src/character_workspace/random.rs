//! Randomize (brief §11, architecture §15b; ux patterns P15): the
//! toolbar's Randomize draws the whole character, the open category or the
//! picked feature again with `avatar::randomize`, skipping what is locked.
//! A lock is on a slider row or the body type; the body type (`frame`) is
//! locked by default, so Randomize never swaps a male body for a female
//! one unasked. Locks are editor state: never in a description, never an
//! undo step. One Randomize is one undo step.
//!
//! The seed comes from a counter mixed with the character, not the clock,
//! so the same presses on the same character give the same results (tests
//! repeat), and pressing again gives another.

use std::collections::BTreeSet;

use axcenity_avatar::{BaseManifest, Library, Scope, chosen_slots, production_slots};

use super::CharacterWorkspace;
use super::selection::parameter_feature;

/// What Randomize draws again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomScope {
    /// The whole character.
    Everything,
    /// The open category (Body, Face…).
    Category,
    /// The feature picked in it (the nose…).
    Feature,
}

/// What the menu and the notice add when [`no_items_to_draw`]: Randomize
/// draws Production items only (ADR-0043).
pub const NO_ITEMS: &str = " Worn items stay as they are: there are no production items to draw \
                            yet.";

/// Whether `scope` reaches item slots on `base` (ones a person picks in,
/// not a body type's) and none of them has a Production item: Randomize
/// then leaves every item as it is.
pub fn no_items_to_draw(library: &Library, base: &BaseManifest, scope: &Scope) -> bool {
    let chosen = chosen_slots(library, &base.id);
    let production = production_slots(library, &base.id);
    let mut reached = base
        .slots
        .iter()
        .filter(|slot| !chosen.contains(&slot.id))
        .filter(|slot| match scope {
            Scope::All => true,
            Scope::Category(category) => slot.category == *category,
            Scope::Feature(_) | Scope::Parameters(_) => false,
        })
        .peekable();
    reached.peek().is_some() && reached.all(|slot| !production.contains(&slot.id))
}

/// The locks a new workspace starts with: the body type.
pub fn default_locks() -> BTreeSet<String> {
    BTreeSet::from(["frame".to_owned()])
}

/// `avatar`'s scope for `scope` with `category` and `feature` open, or
/// `None` when there's nothing of it to draw (a page with no sliders).
pub fn scope_for(
    scope: RandomScope,
    category: super::Category,
    feature: Option<&str>,
    base: &BaseManifest,
) -> Option<Scope> {
    match scope {
        RandomScope::Everything => Some(Scope::All),
        RandomScope::Category => category.data().map(Scope::Category),
        RandomScope::Feature => {
            // The feature as the studio shows it: a manifest feature, or a
            // group where the manifest has none (the stylized body).
            let feature = feature?;
            let ids: Vec<String> = base
                .parameters
                .iter()
                .filter(|parameter| parameter_feature(parameter) == feature)
                .map(|parameter| parameter.id.clone())
                .collect();
            (!ids.is_empty()).then_some(Scope::Parameters(ids))
        }
    }
}

/// A 64-bit FNV-1a hash, stable across builds.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

impl CharacterWorkspace {
    /// Draws `scope` again, keeping what's locked, as one undo step.
    pub fn randomize(&mut self, scope: RandomScope) {
        let library = self.library();
        let Some(base) = library.base(&self.description.base) else {
            return;
        };
        let feature = self.selection.feature.as_deref();
        let Some(scope) = scope_for(scope, self.selection.category, feature, base) else {
            self.notice = Some("Nothing here to randomize.".to_owned());
            return;
        };
        self.rolls = self.rolls.wrapping_add(1);
        // The counter, spread over the bits, mixed with the character.
        let seed = fnv1a(self.description.to_json().as_bytes())
            ^ self.rolls.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let drawn =
            axcenity_avatar::randomize(&self.description, &scope, &self.locks, seed, &library);
        if drawn == self.description {
            // Nothing to draw (no production items, everything locked): no
            // empty undo step, and say why.
            self.notice = Some(if no_items_to_draw(&library, base, &scope) {
                format!("Nothing changed.{NO_ITEMS}")
            } else {
                "Nothing changed.".to_owned()
            });
            return;
        }
        self.edit(|description| *description = drawn);
    }

    /// Locks or unlocks parameters (a slider row and its mirror).
    pub fn set_lock(&mut self, ids: &[String], locked: bool) {
        for id in ids {
            if locked {
                self.locks.insert(id.clone());
            } else {
                self.locks.remove(id);
            }
        }
    }

    /// How many of `base`'s parameters in `scope`'s reach are locked.
    pub(super) fn locked_in(&self, base: &BaseManifest, scope: &Scope) -> usize {
        base.parameters
            .iter()
            .filter(|parameter| self.locks.contains(&parameter.id))
            .filter(|parameter| match scope {
                Scope::All => true,
                Scope::Category(category) => base.category_of(parameter) == Some(*category),
                Scope::Parameters(ids) => ids.contains(&parameter.id),
                Scope::Feature(feature) => parameter_feature(parameter) == feature,
            })
            .count()
    }
}
