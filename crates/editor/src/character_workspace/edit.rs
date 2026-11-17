//! The workspace's edit model (refinement plan section 11.3): undo and
//! redo as whole-description snapshots, transactions so a drag is one
//! step, and reset by scope.
//!
//! A transaction is opened by the first change of a gesture (a slider
//! drag, a typed name) and closed by [`History::commit`], which records
//! one step only if the description changed: a click that changes nothing
//! keeps redo (audit D-05). [`History::cancel`] closes it and hands back
//! the description it started from (Esc during a drag).
//!
//! It is separate from the Game Editor's `History`; editor-only state
//! (selection, camera, stage) is never in it.

use axcenity_avatar::{BaseManifest, Category, Description, Library, WornItem};

/// What a reset puts back to the default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The whole character: the body's default character, keeping the
    /// name.
    All,
    /// One manifest category: its sliders (body type, a choice, stays),
    /// the items of its slots, or (Skin) the skin materials' values.
    Category(Category),
    /// One manifest group's sliders: a feature, such as the nose.
    Group(String),
}

/// Undo and redo for one description.
#[derive(Clone, Debug, Default)]
pub struct History {
    past: Vec<Description>,
    future: Vec<Description>,
    /// The description an open transaction started from.
    open: Option<Description>,
}

impl History {
    /// Records a finished change from `before` to `after` as one step;
    /// nothing when they're the same. Returns whether a step was made.
    pub fn record(&mut self, before: Description, after: &Description) -> bool {
        if before == *after {
            return false;
        }
        self.past.push(before);
        self.future.clear();
        true
    }

    /// Opens a transaction on `current`, unless one is open.
    pub fn begin(&mut self, current: &Description) {
        if self.open.is_none() {
            self.open = Some(current.clone());
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The description the open transaction started from.
    pub fn start(&self) -> Option<&Description> {
        self.open.as_ref()
    }

    /// Closes the open transaction, recording it as one step if `current`
    /// differs from where it started. Returns whether a step was made.
    pub fn commit(&mut self, current: &Description) -> bool {
        match self.open.take() {
            Some(start) => self.record(start, current),
            None => false,
        }
    }

    /// Closes the open transaction without a step and returns the
    /// description it started from, to go back to.
    pub fn cancel(&mut self) -> Option<Description> {
        self.open.take()
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty() || self.open.is_some()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty() && self.open.is_none()
    }

    /// The description before the last step, given the `current` one. An
    /// open transaction is closed first, so its change is what's undone.
    pub fn undo(&mut self, current: Description) -> Option<Description> {
        self.commit(&current);
        let before = self.past.pop()?;
        self.future.push(current);
        Some(before)
    }

    /// The description after the step last undone, given the `current` one.
    pub fn redo(&mut self, current: Description) -> Option<Description> {
        if self.open.is_some() {
            return None;
        }
        let after = self.future.pop()?;
        self.past.push(current);
        Some(after)
    }
}

/// Whether `a` and `b` are the same character: equal once parameters at
/// their default are left out (a parameter left out takes its default, so
/// a slider clicked where it was changes nothing).
pub fn same_character(a: &Description, b: &Description, base: Option<&BaseManifest>) -> bool {
    if a == b {
        return true;
    }
    let plain = |description: &Description| {
        let mut plain = description.clone();
        plain.parameters.retain(|id, value| {
            base.and_then(|base| base.parameter(id))
                .is_none_or(|parameter| *value != parameter.default)
        });
        plain
    };
    plain(a) == plain(b)
}

/// `description` with `scope` put back to the base's defaults (`defaults`
/// is the base's default character). The name is kept. The caller tidies
/// the result (a choice's items are worn again).
pub fn reset(
    description: &Description,
    defaults: &Description,
    base: &BaseManifest,
    library: &Library,
    scope: &Scope,
) -> Description {
    let mut reset = description.clone();
    let mut sliders_where = |keep: &dyn Fn(&axcenity_avatar::Parameter) -> bool| {
        for parameter in base.parameters.iter().filter(|p| !keep(p)) {
            match defaults.parameters.get(&parameter.id) {
                Some(value) => {
                    reset.parameters.insert(parameter.id.clone(), *value);
                }
                None => {
                    reset.parameters.remove(&parameter.id);
                }
            }
        }
    };
    let category_of = |group: &str| {
        base.groups
            .iter()
            .find(|g| g.id == group)
            .map(|g| g.category)
    };
    match scope {
        Scope::All => {
            reset = defaults.clone();
            reset.name = description.name.clone();
            return reset;
        }
        Scope::Group(group) => sliders_where(&|p| p.group != *group || !p.choices.is_empty()),
        Scope::Category(category) => {
            sliders_where(&|p| category_of(&p.group) != Some(*category) || !p.choices.is_empty());
            // The items worn in the category's slots go back to the base's
            // defaults for those slots.
            let slot_of = |id: &str| {
                library
                    .item(id)
                    .and_then(|item| base.slot(&item.slot))
                    .map(|slot| slot.category)
            };
            reset
                .items
                .retain(|worn| slot_of(&worn.id) != Some(*category));
            for item in &base.defaults.items {
                if slot_of(item) == Some(*category) && !reset.items.iter().any(|w| w.id == *item) {
                    reset.items.push(WornItem::new(item.clone()));
                }
            }
            if *category == Category::Skin {
                for slot in &base.materials {
                    if slot.shading == axcenity_avatar::ShadingModel::Skin {
                        reset.materials.remove(&slot.name);
                    }
                }
            }
        }
    }
    reset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> Description {
        Description::new(name, "axcenity/realistic")
    }

    #[test]
    fn a_transaction_is_one_step_and_a_click_that_changes_nothing_keeps_redo() {
        let mut history = History::default();
        let (a, b, c) = (named("a"), named("b"), named("c"));
        assert!(history.record(a.clone(), &b));
        let back = history.undo(b.clone()).unwrap();
        assert_eq!(back, a);
        assert!(history.can_redo());

        // A drag that ends where it started (a click on a slider): no step,
        // and redo survives (audit D-05).
        history.begin(&a);
        history.begin(&c);
        assert!(history.is_open());
        assert!(!history.commit(&a));
        assert!(history.can_redo());
        assert_eq!(history.redo(a.clone()), Some(b.clone()));

        // A drag through several values is one step back to its start.
        history.begin(&b);
        assert!(history.commit(&c));
        assert_eq!(history.undo(c.clone()), Some(b.clone()));
        assert_eq!(history.undo(b.clone()), Some(a.clone()));
        assert!(!history.can_undo());
    }

    #[test]
    fn cancel_goes_back_to_where_the_transaction_started_without_a_step() {
        let mut history = History::default();
        history.begin(&named("start"));
        assert!(history.can_undo(), "an open drag can be undone");
        assert!(!history.can_redo());
        assert_eq!(history.cancel(), Some(named("start")));
        assert!(!history.is_open());
        assert!(!history.can_undo());
        assert_eq!(history.cancel(), None);
    }

    #[test]
    fn undo_during_a_drag_undoes_the_drag() {
        let mut history = History::default();
        history.begin(&named("before"));
        assert_eq!(history.undo(named("during")), Some(named("before")));
        assert!(!history.is_open());
        assert_eq!(history.redo(named("before")), Some(named("during")));
    }
}
