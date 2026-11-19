//! The asymmetry tools (refinement R4, brief §5.4): Mirror left to right,
//! Mirror right to left, Reset left and Reset right, for a feature's pairs
//! and for one pair, each one undo step through an [`Intent`], with
//! Symmetry's behaviour unchanged.

use super::intent::{Gesture, Intent};
use super::panel::{Shown, hidden_changes, rows, side_choices};
use super::selection::Level;
use super::sides::{self, SideTool, SidesOf};
use super::*;
use crate::character_studio::test_folder;

fn set(id: &str, value: f32) -> Intent {
    Intent::SetParameter {
        id: id.to_owned(),
        value,
        gesture: Gesture::Commit,
    }
}

fn tool(tool: SideTool, of: SidesOf) -> Intent {
    Intent::Sides { tool, of }
}

fn eyes(level: Level) -> SidesOf {
    SidesOf::Feature {
        feature: "eyes".into(),
        level,
    }
}

/// How many steps undo goes back.
fn undo_depth(workspace: &CharacterWorkspace) -> usize {
    let mut history = workspace.history.clone();
    let mut current = workspace.description().clone();
    let mut depth = 0;
    while let Some(before) = history.undo(current) {
        current = before;
        depth += 1;
    }
    depth
}

/// A workspace on the realistic default character whose eyes differ
/// side to side: size, tilt and the eye corner set one way on the left and
/// another on the right.
fn uneven_eyes(name: &str) -> (axcenity_test_dirs::TestDir, CharacterWorkspace) {
    let folder = test_folder(name);
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    workspace.apply(Intent::PickCategory(Category::Face));
    workspace.apply(Intent::SetSymmetry(false));
    for (id, value) in [
        ("eye_size_left", 0.4),
        ("eye_size_right", -0.3),
        ("eye_tilt_left", 0.5),
        ("eye_corner_right", -0.4),
        ("nostril_width_left", 0.6),
    ] {
        workspace.apply(set(id, value));
    }
    workspace.apply(Intent::SetSymmetry(true));
    (folder, workspace)
}

fn default_of(workspace: &CharacterWorkspace, id: &str) -> f32 {
    workspace.base().unwrap().parameter(id).unwrap().default
}

#[test]
fn pairs_are_found_by_side_and_by_feature() {
    let folder = test_folder("sides-pairs");
    let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let base = workspace.base().unwrap();
    // Either side names the pair, and the left is the character's left.
    for id in ["eye_size_left", "eye_size_right"] {
        let pair = sides::pair_of(&base, id).unwrap();
        assert_eq!(pair.left.id, "eye_size_left");
        assert_eq!(pair.right.id, "eye_size_right");
    }
    assert!(sides::pair_of(&base, "nose_length").is_none(), "no mirror");
    // A feature's pairs, each once, only those the level shows.
    let ids = |level| -> Vec<String> {
        sides::pairs(&base, &eyes(level))
            .iter()
            .map(|pair| pair.left.id.clone())
            .collect()
    };
    let detailed = ids(Level::Detailed);
    assert!(
        detailed.contains(&"eye_size_left".to_owned()),
        "{detailed:?}"
    );
    assert!(detailed.contains(&"eye_corner_left".to_owned()));
    assert!(
        !detailed.contains(&"eye_depth_left".to_owned()),
        "eye depth is Advanced: {detailed:?}"
    );
    assert!(ids(Level::Advanced).contains(&"eye_depth_left".to_owned()));
    assert_eq!(ids(Level::Quick), vec!["eye_size_left".to_owned()]);
    let mut unique = detailed.clone();
    unique.dedup();
    assert_eq!(unique.len(), detailed.len());
    assert!(
        sides::pairs(
            &base,
            &SidesOf::Feature {
                feature: "nose".into(),
                level: Level::Detailed,
            }
        )
        .iter()
        .all(|pair| pair.left.id == "nostril_width_left")
    );
    drop(workspace);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn mirror_left_to_right_copies_the_feature_s_left_side_in_one_step() {
    let (folder, mut workspace) = uneven_eyes("sides-mirror-lr");
    let before = workspace.description().clone();
    let depth = undo_depth(&workspace);
    workspace.apply(tool(SideTool::MirrorLeftToRight, eyes(Level::Detailed)));
    assert_eq!(workspace.parameter("eye_size_right"), 0.4);
    assert_eq!(workspace.parameter("eye_size_left"), 0.4, "the left stays");
    assert_eq!(workspace.parameter("eye_tilt_right"), 0.5);
    assert_eq!(
        workspace.parameter("eye_corner_right"),
        workspace.parameter("eye_corner_left"),
        "a right side set alone takes the left's (default) value"
    );
    assert_eq!(
        workspace.parameter("nostril_width_left"),
        0.6,
        "another feature is left alone"
    );
    assert_eq!(undo_depth(&workspace), depth + 1, "one undo step");
    assert!(
        workspace
            .notice
            .as_deref()
            .unwrap()
            .contains("Mirror left to right")
    );
    workspace.apply(Intent::Undo);
    assert_eq!(workspace.description(), &before);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn mirror_right_to_left_copies_the_feature_s_right_side_in_one_step() {
    let (folder, mut workspace) = uneven_eyes("sides-mirror-rl");
    let depth = undo_depth(&workspace);
    workspace.apply(tool(SideTool::MirrorRightToLeft, eyes(Level::Detailed)));
    assert_eq!(workspace.parameter("eye_size_left"), -0.3);
    assert_eq!(workspace.parameter("eye_size_right"), -0.3);
    assert_eq!(workspace.parameter("eye_corner_left"), -0.4);
    assert_eq!(
        workspace.parameter("eye_tilt_left"),
        default_of(&workspace, "eye_tilt_right"),
        "the right's default tilt goes across"
    );
    assert_eq!(undo_depth(&workspace), depth + 1);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn reset_left_and_reset_right_put_one_side_back() {
    let (folder, mut workspace) = uneven_eyes("sides-reset");
    let depth = undo_depth(&workspace);
    workspace.apply(tool(SideTool::ResetLeft, eyes(Level::Detailed)));
    for id in ["eye_size_left", "eye_tilt_left"] {
        assert_eq!(workspace.parameter(id), default_of(&workspace, id), "{id}");
    }
    assert_eq!(
        workspace.parameter("eye_size_right"),
        -0.3,
        "the right stays"
    );
    assert_eq!(workspace.parameter("eye_corner_right"), -0.4);
    assert_eq!(undo_depth(&workspace), depth + 1);

    workspace.apply(tool(SideTool::ResetRight, eyes(Level::Detailed)));
    for id in ["eye_size_right", "eye_corner_right"] {
        assert_eq!(workspace.parameter(id), default_of(&workspace, id), "{id}");
    }
    assert_eq!(undo_depth(&workspace), depth + 2);
    // Both sides reset: the eyes are the default character's again.
    let defaults =
        axcenity_gameplay::description::default_description(&workspace.library(), DEFAULT_BASE);
    let base = workspace.base().unwrap();
    for pair in sides::pairs(&base, &eyes(Level::Detailed)) {
        for id in [&pair.left.id, &pair.right.id] {
            assert_eq!(
                workspace.description().parameters.get(id),
                defaults.parameters.get(id),
                "{id}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_pair_s_tools_change_only_that_pair() {
    let (folder, mut workspace) = uneven_eyes("sides-pair");
    let depth = undo_depth(&workspace);
    // Either side's ID names the pair.
    workspace.apply(tool(
        SideTool::MirrorRightToLeft,
        SidesOf::Pair("eye_size_right".into()),
    ));
    assert_eq!(workspace.parameter("eye_size_left"), -0.3);
    assert_eq!(
        workspace.parameter("eye_tilt_left"),
        0.5,
        "another pair stays"
    );
    workspace.apply(tool(
        SideTool::ResetRight,
        SidesOf::Pair("eye_size_left".into()),
    ));
    assert_eq!(
        workspace.parameter("eye_size_right"),
        default_of(&workspace, "eye_size_right")
    );
    assert_eq!(workspace.parameter("eye_size_left"), -0.3);
    workspace.apply(tool(
        SideTool::MirrorLeftToRight,
        SidesOf::Pair("eye_tilt_left".into()),
    ));
    assert_eq!(workspace.parameter("eye_tilt_right"), 0.5);
    workspace.apply(tool(
        SideTool::ResetLeft,
        SidesOf::Pair("eye_tilt_right".into()),
    ));
    assert_eq!(
        workspace.parameter("eye_tilt_left"),
        default_of(&workspace, "eye_tilt_left")
    );
    assert_eq!(workspace.parameter("eye_tilt_right"), 0.5);
    assert_eq!(undo_depth(&workspace), depth + 4, "one step each");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A tool that would change nothing makes no step and says so; the menu
/// greys it out.
#[test]
fn a_tool_with_nothing_to_change_makes_no_step() {
    let folder = test_folder("sides-nothing");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let depth = undo_depth(&workspace);
    for each in SideTool::ALL {
        workspace.apply(tool(each, eyes(Level::Detailed)));
        assert_eq!(undo_depth(&workspace), depth, "{each:?}");
    }
    assert!(
        workspace
            .notice
            .as_deref()
            .unwrap()
            .contains("nothing to change")
    );
    let library = workspace.library();
    let base = library.base(DEFAULT_BASE).unwrap();
    let defaults = axcenity_gameplay::description::default_description(&library, DEFAULT_BASE);
    let choices = side_choices(
        workspace.description(),
        &defaults,
        base,
        &eyes(Level::Detailed),
    );
    assert_eq!(choices.len(), 4);
    assert!(choices.iter().all(|(_, enabled)| !enabled), "{choices:?}");
    // A pair whose left differs: both mirrors and Reset left are offered,
    // Reset right isn't (the right is at the default).
    workspace.apply(Intent::SetMirrored {
        id: "eye_size_left".into(),
        value: 0.3,
        gesture: Gesture::Commit,
    });
    workspace.apply(Intent::SetSymmetry(false));

    workspace.apply(set("eye_size_left", 0.5));
    let choices = side_choices(
        workspace.description(),
        &defaults,
        base,
        &SidesOf::Pair("eye_size_left".into()),
    );
    let enabled = |wanted: SideTool| choices.iter().any(|(t, on)| *t == wanted && *on);
    assert!(enabled(SideTool::MirrorLeftToRight));
    assert!(enabled(SideTool::MirrorRightToLeft));
    assert!(enabled(SideTool::ResetLeft));
    assert!(
        enabled(SideTool::ResetRight),
        "the right is 0.3, not the default"
    );
    // The stylized body has no pairs: the tool says so.
    workspace.apply(Intent::New(Start::Family(axcenity_avatar::V1_BASE.into())));
    workspace.answer(Some(false));
    workspace.apply(tool(SideTool::MirrorLeftToRight, eyes(Level::Detailed)));
    assert!(
        workspace
            .notice
            .as_deref()
            .unwrap()
            .contains("no left and right pairs")
    );
    assert!(!workspace.can_undo());
    let _ = std::fs::remove_dir_all(&folder);
}

/// A pair the level hides isn't changed by the feature's tools (what is
/// changed is what can be seen), and the panel's "N more changes in
/// Advanced" keeps counting it; at Advanced the tool reaches it.
#[test]
fn hidden_pairs_stay_put_and_the_hidden_changes_notice_counts_them() {
    let (folder, mut workspace) = uneven_eyes("sides-hidden");
    workspace.apply(Intent::SetSymmetry(false));
    workspace.apply(set("eye_depth_left", 0.5));
    workspace.apply(Intent::SetSymmetry(true));
    let hidden = |workspace: &CharacterWorkspace| {
        let base = workspace.base().unwrap();
        hidden_changes(
            &base,
            workspace.description(),
            axcenity_avatar::Category::Face,
            Level::Detailed,
        )
    };
    assert_eq!(hidden(&workspace), Some((1, Level::Advanced)));
    workspace.apply(tool(SideTool::MirrorLeftToRight, eyes(Level::Detailed)));
    assert_eq!(
        workspace.parameter("eye_depth_right"),
        default_of(&workspace, "eye_depth_right"),
        "hidden at Detailed: left alone"
    );
    assert_eq!(hidden(&workspace), Some((1, Level::Advanced)), "still said");
    workspace.apply(tool(SideTool::MirrorLeftToRight, eyes(Level::Advanced)));
    assert_eq!(workspace.parameter("eye_depth_right"), 0.5);
    assert_eq!(hidden(&workspace), Some((2, Level::Advanced)), "both sides");
    workspace.apply(tool(SideTool::ResetLeft, eyes(Level::Advanced)));
    workspace.apply(tool(SideTool::ResetRight, eyes(Level::Advanced)));
    assert_eq!(hidden(&workspace), None, "nothing hidden is changed now");
    let _ = std::fs::remove_dir_all(&folder);
}

/// Symmetry is as it was: a slider moves both sides while they agree; a
/// pair whose sides differ shows both rows; once a tool makes them agree,
/// it's one row again.
#[test]
fn symmetry_still_works_and_a_mirrored_pair_is_one_row_again() {
    let (folder, mut workspace) = uneven_eyes("sides-symmetry");
    let shown = |workspace: &CharacterWorkspace| -> Vec<(String, bool)> {
        let base = workspace.base().unwrap();
        rows(
            &base,
            workspace.description(),
            "eyes",
            Level::Detailed,
            None,
            workspace.symmetry,
        )
        .into_iter()
        .map(
            |Shown {
                 parameter,
                 mirrored,
             }| (parameter.id.clone(), mirrored),
        )
        .filter(|(id, _)| id.starts_with("eye_size"))
        .collect()
    };
    assert_eq!(
        shown(&workspace),
        vec![
            ("eye_size_left".to_owned(), false),
            ("eye_size_right".to_owned(), false)
        ],
        "the sides differ: both are shown"
    );
    workspace.apply(tool(
        SideTool::MirrorLeftToRight,
        SidesOf::Pair("eye_size_left".into()),
    ));
    assert_eq!(shown(&workspace), vec![("eye_size_left".to_owned(), true)]);
    workspace.apply(Intent::SetMirrored {
        id: "eye_size_left".into(),
        value: -0.2,
        gesture: Gesture::Commit,
    });
    assert_eq!(workspace.parameter("eye_size_right"), -0.2);
    assert!(workspace.symmetry, "the tools never turn Symmetry off");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The tools are on the feature's controls: a "Left and right" menu over
/// a feature with pairs, and a pair button on each paired row.
#[test]
fn the_tools_are_drawn_with_the_feature_s_controls() {
    let (folder, mut workspace) = uneven_eyes("sides-drawn");
    workspace.apply(Intent::PickFeature(Some("eyes".into())));
    let ctx = crate::theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        let output = crate::theme::run(&ctx, super::shell_tests::input(Vec::new()), |ui| {
            workspace.ui(ui, None);
        });
        texts = super::shell_tests::drawn_texts(&output);
    }
    assert!(
        texts.iter().any(|t| t.contains("Left and right")),
        "{texts:?}"
    );
    let pair_buttons = texts
        .iter()
        .filter(|t| t.contains(crate::theme::icons::ARROWS_LEFT_RIGHT))
        .count();
    assert!(pair_buttons >= 3, "a pair button per paired row: {texts:?}");
    // The nose has a pair too; the forehead hasn't.
    workspace.apply(Intent::PickFeature(Some("forehead".into())));
    let mut texts = Vec::new();
    for _ in 0..2 {
        let output = crate::theme::run(&ctx, super::shell_tests::input(Vec::new()), |ui| {
            workspace.ui(ui, None);
        });
        texts = super::shell_tests::drawn_texts(&output);
    }
    assert!(
        !texts.iter().any(|t| t.contains("Left and right")),
        "{texts:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}
