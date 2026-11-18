//! What the workspace is showing (refinement plan section 11.3, "Editor-only
//! state"): the category, the feature inside it and the detail level. It is
//! editor state only: never in a description, never saved, never an undo
//! step. Panels read it; only the shell changes it, through an
//! [`Intent`](super::intent::Intent).
//!
//! The three functions at the end read a parameter's `level`, `feature`
//! and `about` from the manifest (refinement plan 11.3), with fallbacks
//! for a base model whose manifest doesn't carry them (the stylized one).

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
    /// Fine and one-sided controls.
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
            Level::Advanced => "Fine controls, each side on its own",
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

/// The level a parameter is shown from: its manifest `level`
/// ([`Parameter::level`], which falls back to `advanced`). A parameter with
/// no `level` but with choices (a body type) is Quick.
pub fn parameter_level(parameter: &Parameter) -> Level {
    use axcenity_avatar::Level as Data;
    if parameter.level.is_none() && !parameter.choices.is_empty() && !parameter.advanced {
        return Level::Quick;
    }
    match parameter.level() {
        Data::Quick => Level::Quick,
        Data::Advanced => Level::Advanced,
        Data::Detailed | Data::Other => Level::Detailed,
    }
}

/// The feature a parameter belongs to: its manifest `feature`, or its
/// group where the manifest has none.
pub fn parameter_feature(parameter: &Parameter) -> &str {
    parameter.feature.as_deref().unwrap_or(&parameter.group)
}

/// What a parameter's tooltip says: its manifest `about`, or its label.
pub fn parameter_about(parameter: &Parameter) -> String {
    parameter
        .about
        .clone()
        .unwrap_or_else(|| parameter.label.clone())
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
    fn without_metadata_a_parameter_falls_back_to_what_it_has() {
        assert_eq!(parameter_level(&parameter(true, false)), Level::Advanced);
        assert_eq!(parameter_level(&parameter(false, false)), Level::Detailed);
        assert_eq!(parameter_level(&parameter(false, true)), Level::Quick);
        let nose = parameter(false, false);
        assert_eq!(parameter_feature(&nose), "nose");
        assert_eq!(parameter_about(&nose), "Nose length");
    }

    #[test]
    fn the_manifest_s_level_feature_and_about_win() {
        let tip: Parameter = serde_json::from_str(
            r#"{"id": "nose_tip", "label": "Nose tip", "group": "nose", "range": [-1, 1],
                "level": "quick", "feature": "nose_tip_area",
                "about": "How far the tip turns up or down."}"#,
        )
        .unwrap();
        assert_eq!(parameter_level(&tip), Level::Quick);
        assert_eq!(parameter_feature(&tip), "nose_tip_area");
        assert_eq!(parameter_about(&tip), "How far the tip turns up or down.");
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
