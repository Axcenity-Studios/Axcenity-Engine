//! What the workspace is showing (refinement plan section 11.3, "Editor-only
//! state"): the category, the feature inside it and the detail level. It is
//! editor state only: never in a description, never saved, never an undo
//! step. Panels read it; only the shell changes it, through an
//! [`Intent`](super::intent::Intent).
//!
//! The three functions at the end are **shims** until the parameter
//! manifest carries `level`, `feature` and `about` (the lead's contract
//! change in `avatar`): each reads what the manifest has today. At merge
//! the lead swaps their bodies for the new fields; nothing else changes.

use axcenity_avatar::Parameter;

use super::Category;

/// How much of a category is shown (brief §3.3). One flat filter, not three
/// screens: each level shows everything the level below shows, plus more.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Presets, body type and the few broad controls.
    Quick,
    /// Every feature's own sliders.
    #[default]
    Detailed,
    /// Fine and one-sided controls, numbers typed in.
    Advanced,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Quick, Level::Detailed, Level::Advanced];

    pub fn label(self) -> &'static str {
        match self {
            Level::Quick => "Quick",
            Level::Detailed => "Detailed",
            Level::Advanced => "Advanced",
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Level::Quick => "Presets, body type and the broad controls",
            Level::Detailed => "Every feature's own sliders",
            Level::Advanced => "Fine controls, each side on its own, typed numbers",
        }
    }

    /// Whether something of level `level` is shown at this level.
    pub fn shows(self, level: Level) -> bool {
        level <= self
    }
}

/// The workspace's selection: a category, optionally one feature in it
/// (a manifest group's ID today, the manifest's `feature` once it has
/// one), and the detail level.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub category: Category,
    pub feature: Option<String>,
    pub level: Level,
}

impl Selection {
    /// Picks `category`; the feature is dropped, as it belonged to the
    /// category left.
    pub fn pick_category(&mut self, category: Category) {
        if self.category != category {
            self.category = category;
            self.feature = None;
        }
    }
}

/// Shim (lead swaps at merge for `Parameter.level`): the level a
/// parameter is shown from. `advanced` is Advanced; a choice (body type) is
/// Quick; everything else is Detailed.
pub fn parameter_level(parameter: &Parameter) -> Level {
    if parameter.advanced {
        Level::Advanced
    } else if !parameter.choices.is_empty() {
        Level::Quick
    } else {
        Level::Detailed
    }
}

/// Shim (lead swaps at merge for `Parameter.feature`): the feature a
/// parameter belongs to. Today a feature is the parameter's group.
pub fn parameter_feature(parameter: &Parameter) -> &str {
    &parameter.group
}

/// Shim (lead swaps at merge for `Parameter.about`): what a parameter's
/// tooltip says. Today only its label is known.
pub fn parameter_about(parameter: &Parameter) -> String {
    parameter.label.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter(advanced: bool, choices: bool) -> Parameter {
        let mut text = format!(
            r#"{{"id": "p", "label": "Nose length", "group": "nose", "range": [-1, 1], "advanced": {advanced}"#
        );
        if choices {
            text.push_str(
                r#", "choices": [{"value": 0, "label": "A"}, {"value": 1, "label": "B"}]"#,
            );
        }
        text.push('}');
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn levels_are_one_flat_filter() {
        assert!(Level::Advanced.shows(Level::Quick));
        assert!(Level::Advanced.shows(Level::Detailed));
        assert!(Level::Detailed.shows(Level::Quick));
        assert!(!Level::Detailed.shows(Level::Advanced));
        assert!(!Level::Quick.shows(Level::Detailed));
        assert_eq!(Level::default(), Level::Detailed, "today's default view");
    }

    #[test]
    fn the_shims_read_what_the_manifest_has_today() {
        assert_eq!(parameter_level(&parameter(true, false)), Level::Advanced);
        assert_eq!(parameter_level(&parameter(false, false)), Level::Detailed);
        assert_eq!(parameter_level(&parameter(false, true)), Level::Quick);
        let nose = parameter(false, false);
        assert_eq!(parameter_feature(&nose), "nose");
        assert_eq!(parameter_about(&nose), "Nose length");
    }

    #[test]
    fn picking_another_category_drops_the_feature() {
        let mut selection = Selection {
            category: Category::Face,
            feature: Some("nose".into()),
            level: Level::Advanced,
        };
        selection.pick_category(Category::Face);
        assert_eq!(selection.feature.as_deref(), Some("nose"));
        selection.pick_category(Category::Body);
        assert_eq!(selection.feature, None);
        assert_eq!(selection.level, Level::Advanced, "the level stays");
    }
}
