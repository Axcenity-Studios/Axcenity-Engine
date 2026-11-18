//! The asymmetry tools (refinement R4, brief §5.4): for a feature's left
//! and right pairs, or for one pair, **Mirror left to right**, **Mirror
//! right to left**, **Reset left** and **Reset right**. Each is one
//! [`Intent::Sides`](super::intent::Intent::Sides) and one undo step.
//! Symmetry is untouched: it still moves both sides of a slider together
//! while they agree, and a pair whose sides differ still shows both.
//!
//! A pair is two parameters that name each other as `mirror` in the
//! manifest. Each side's parameter is already written for its own side
//! (a left eye's tilt and a right eye's tilt turn the same way when their
//! values are equal), so mirroring a side is copying its value across.
//! Resetting a side puts its parameter back to the body's default
//! character's value.

use axcenity_avatar::{BaseManifest, Description, Parameter};

use super::selection::{Level, parameter_feature, parameter_level};
use super::value_of;

/// One of the four tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SideTool {
    /// The right side takes the left side's values.
    MirrorLeftToRight,
    /// The left side takes the right side's values.
    MirrorRightToLeft,
    /// The left side back to the default.
    ResetLeft,
    /// The right side back to the default.
    ResetRight,
}

impl SideTool {
    pub const ALL: [SideTool; 4] = [
        SideTool::MirrorLeftToRight,
        SideTool::MirrorRightToLeft,
        SideTool::ResetLeft,
        SideTool::ResetRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SideTool::MirrorLeftToRight => "Mirror left to right",
            SideTool::MirrorRightToLeft => "Mirror right to left",
            SideTool::ResetLeft => "Reset left side",
            SideTool::ResetRight => "Reset right side",
        }
    }

    /// The tooltip. Left and right are the character's own.
    pub fn about(self) -> &'static str {
        match self {
            SideTool::MirrorLeftToRight => {
                "The character's right side takes its left side's values (one undo step)"
            }
            SideTool::MirrorRightToLeft => {
                "The character's left side takes its right side's values (one undo step)"
            }
            SideTool::ResetLeft => {
                "The character's left side back to the default; the right stays (one undo step)"
            }
            SideTool::ResetRight => {
                "The character's right side back to the default; the left stays (one undo step)"
            }
        }
    }
}

/// What a tool works on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidesOf {
    /// Every pair of a feature (a manifest `feature`, or a group where the
    /// manifest has none) that `level` shows: the tool changes what can
    /// be seen, and a pair the level hides stays as it is (the panel's
    /// "N more changes in Advanced" still counts it).
    Feature { feature: String, level: Level },
    /// One pair, named by either side's parameter.
    Pair(String),
}

/// A left and right pair of parameters.
#[derive(Clone, Copy, Debug)]
pub struct Pair<'a> {
    pub left: &'a Parameter,
    pub right: &'a Parameter,
}

/// Which side a parameter is on, by its ID (`_left`, `_right`, `left_`…),
/// then by its label ("Left ear", "(right)").
fn is_left(parameter: &Parameter) -> Option<bool> {
    let id = parameter.id.to_ascii_lowercase();
    for (left, right) in [("_left", "_right"), ("_l", "_r")] {
        if id.ends_with(left) {
            return Some(true);
        }
        if id.ends_with(right) {
            return Some(false);
        }
    }
    if id.starts_with("left_") {
        return Some(true);
    }
    if id.starts_with("right_") {
        return Some(false);
    }
    let label = parameter.label.to_ascii_lowercase();
    let words: Vec<&str> = label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    match (words.contains(&"left"), words.contains(&"right")) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}

/// The pair parameter `id` belongs to, if it has a mirror. A pair whose
/// sides can't be told apart by ID or label takes the one the manifest
/// lists first as the left.
pub fn pair_of<'a>(base: &'a BaseManifest, id: &str) -> Option<Pair<'a>> {
    let parameter = base.parameter(id)?;
    let mirror = base.parameter(parameter.mirror.as_deref()?)?;
    if mirror.id == parameter.id {
        return None;
    }
    let first = |a: &Parameter| base.parameters.iter().position(|p| p.id == a.id);
    let left_first = match (is_left(parameter), is_left(mirror)) {
        (Some(left), _) => left,
        (None, Some(mirror_left)) => !mirror_left,
        (None, None) => first(parameter) < first(mirror),
    };
    Some(if left_first {
        Pair {
            left: parameter,
            right: mirror,
        }
    } else {
        Pair {
            left: mirror,
            right: parameter,
        }
    })
}

/// The pairs `of` names, each once, in the manifest's order.
pub fn pairs<'a>(base: &'a BaseManifest, of: &SidesOf) -> Vec<Pair<'a>> {
    match of {
        SidesOf::Pair(id) => pair_of(base, id).into_iter().collect(),
        SidesOf::Feature { feature, level } => {
            let mut found: Vec<Pair> = Vec::new();
            for parameter in base.parameters.iter().filter(|p| {
                p.mirror.is_some()
                    && parameter_feature(p) == feature
                    && level.shows(parameter_level(p))
            }) {
                let Some(pair) = pair_of(base, &parameter.id) else {
                    continue;
                };
                // Both sides must be shown: a side the level hides is left
                // alone, and so is its pair.
                if !level.shows(parameter_level(pair.left))
                    || !level.shows(parameter_level(pair.right))
                {
                    continue;
                }
                if !found.iter().any(|seen| seen.left.id == pair.left.id) {
                    found.push(pair);
                }
            }
            found
        }
    }
}

/// `description` with `tool` used on `pairs`. `defaults` is the body's
/// default character (what Reset puts back, as the toolbar's Reset does).
pub fn apply(
    description: &Description,
    defaults: &Description,
    base: &BaseManifest,
    tool: SideTool,
    pairs: &[Pair],
) -> Description {
    let mut changed = description.clone();
    let value = |id: &str| value_of(description, Some(base), id);
    let mut set = |id: &str, to: Option<f32>| match to {
        Some(value) => {
            changed.parameters.insert(id.to_owned(), value);
        }
        None => {
            changed.parameters.remove(id);
        }
    };
    for pair in pairs {
        let (left, right) = (&pair.left.id, &pair.right.id);
        match tool {
            SideTool::MirrorLeftToRight => set(right, Some(value(left))),
            SideTool::MirrorRightToLeft => set(left, Some(value(right))),
            SideTool::ResetLeft => set(left, defaults.parameters.get(left).copied()),
            SideTool::ResetRight => set(right, defaults.parameters.get(right).copied()),
        }
    }
    changed
}

/// Whether `tool` on `pairs` would change anything (the menu greys out
/// what wouldn't).
pub fn would_change(
    description: &Description,
    defaults: &Description,
    base: &BaseManifest,
    tool: SideTool,
    pairs: &[Pair],
) -> bool {
    let after = apply(description, defaults, base, tool, pairs);
    pairs.iter().any(|pair| {
        [&pair.left.id, &pair.right.id].into_iter().any(|id| {
            (value_of(description, Some(base), id) - value_of(&after, Some(base), id)).abs()
                > 1.0e-6
        })
    })
}

impl super::CharacterWorkspace {
    /// Uses `tool` on the pairs `of` names, as one undo step; the status
    /// line says what it did. Nothing to change makes no step.
    pub fn use_sides(&mut self, tool: SideTool, of: &SidesOf) {
        let library = self.library();
        let Some(base) = library.base(&self.description.base) else {
            return;
        };
        let found = pairs(base, of);
        if found.is_empty() {
            self.notice = Some("There are no left and right pairs here.".to_owned());
            return;
        }
        let defaults =
            axcenity_gameplay::description::default_description(&library, &self.description.base);
        if !would_change(&self.description, &defaults, base, tool, &found) {
            self.notice = Some(format!("{}: nothing to change.", tool.label()));
            return;
        }
        let changed = apply(&self.description, &defaults, base, tool, &found);
        let count = found.len();
        self.edit(|description| *description = changed);
        self.notice = Some(format!(
            "{} ({count} {}). Undo puts it back.",
            tool.label(),
            if count == 1 { "pair" } else { "pairs" }
        ));
    }
}
