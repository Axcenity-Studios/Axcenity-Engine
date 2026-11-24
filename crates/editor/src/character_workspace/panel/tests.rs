//! The panel's and the navigation's choices of what to show, against the
//! realistic body's manifest.

use std::sync::Arc;

use axcenity_avatar::{BaseManifest, Description, Library};

use super::super::edit::Scope;
use super::super::selection::{Level, Selection};
use super::super::{
    Category, CharacterWorkspace, DEFAULT_BASE, Start, find_character_library, nav, toolbar,
};
use super::materials::{DRAWN_TODAY, value_label};
use super::parameters::{found, hidden_changes, matches, rows};
use crate::character_studio::test_folder;

fn realistic() -> (Arc<Library>, BaseManifest) {
    let shipped = find_character_library().expect("the repository's library");
    let (library, _, problems) = Library::read(&shipped);
    assert!(problems.is_empty(), "{problems:?}");
    let base = library.base(DEFAULT_BASE).unwrap().clone();
    (Arc::new(library), base)
}

fn ids(shown: &[super::parameters::Shown]) -> Vec<String> {
    shown.iter().map(|row| row.parameter.id.clone()).collect()
}

#[test]
fn the_level_filters_rows_and_symmetry_merges_only_equal_pairs() {
    let (_, base) = realistic();
    let mut description = Description::new("x", DEFAULT_BASE);
    let eyes = |description: &Description, level, symmetry| {
        rows(&base, description, "eyes", level, None, symmetry)
    };
    // Detailed with Symmetry: one row per pair, the depth (advanced) left out.
    let shown = eyes(&description, Level::Detailed, true);
    assert!(shown.iter().all(|row| row.mirrored));
    assert_eq!(
        ids(&shown),
        [
            "eye_height_left",
            "eye_spacing_left",
            "eye_size_left",
            "eye_tilt_left",
            // R4's eye controls.
            "eye_corner_left",
            "upper_lid_fold_left",
            "lower_lid_left",
            // The finer face controls (2026-11-24); the openness and the
            // upper lid's fullness are advanced.
            "inner_corner_fold_left",
            "under_eye_left"
        ]
    );
    // Advanced adds the depth; Symmetry off shows both sides.
    assert!(ids(&eyes(&description, Level::Advanced, true)).contains(&"eye_depth_left".into()));
    assert_eq!(eyes(&description, Level::Detailed, false).len(), 18);
    // Audit D-11: a pair whose sides differ shows both, even with Symmetry.
    description.parameters.insert("eye_size_left".into(), 0.3);
    let shown = eyes(&description, Level::Detailed, true);
    let sizes: Vec<_> = shown
        .iter()
        .filter(|row| row.parameter.id.starts_with("eye_size"))
        .map(|row| (row.parameter.id.as_str(), row.mirrored))
        .collect();
    assert_eq!(sizes, [("eye_size_left", false), ("eye_size_right", false)]);
    // Quick shows the manifest's quick controls: the body type and the
    // whole body's build, not the regions.
    let frame = rows(&base, &description, "frame", Level::Quick, None, true);
    assert_eq!(ids(&frame), ["frame"]);
    let build = rows(&base, &description, "build", Level::Quick, None, true);
    assert_eq!(ids(&build), ["muscle", "fat"]);
    assert!(
        rows(
            &base,
            &description,
            "build_regions",
            Level::Quick,
            None,
            true
        )
        .is_empty()
    );
    assert!(
        !rows(
            &base,
            &description,
            "build_regions",
            Level::Detailed,
            None,
            true
        )
        .is_empty()
    );
    // A feature filters to its own group.
    assert!(
        rows(
            &base,
            &description,
            "eyes",
            Level::Detailed,
            Some("nose"),
            true
        )
        .is_empty()
    );
}

/// "Search controls": every word typed, in a parameter's label, ID or
/// `about`, capitals aside; in any feature and at any level; a pair shown
/// as one row is found by either side.
#[test]
fn a_search_finds_a_groups_rows_at_every_level_by_label_id_or_about() {
    let (_, base) = realistic();
    let mut description = Description::new("x", DEFAULT_BASE);
    // (Of the jaw's first five parameters: the manifest may have more.)
    let jaw = |description: &Description, search: &str, symmetry| {
        let mut ids = ids(&found(&base, description, "jaw", search, symmetry));
        ids.retain(|id| {
            [
                "jaw_width_left",
                "jaw_width_right",
                "jaw_angle_left",
                "jaw_angle_right",
                "jaw_shift",
            ]
            .contains(&id.as_str())
        });
        ids
    };
    // The jaw's rows at every level: its width is Quick, its angle
    // Detailed, its shift Advanced. With Symmetry a pair is one row.
    assert_eq!(
        jaw(&description, "jaw", true),
        ["jaw_width_left", "jaw_angle_left", "jaw_shift"]
    );
    assert_eq!(jaw(&description, "jaw", false).len(), 5, "both sides");
    // Every word must be there; capitals don't matter.
    assert_eq!(jaw(&description, "JAW angle", true), ["jaw_angle_left"]);
    assert!(jaw(&description, "jaw nostril", true).is_empty());
    // The ID is searched, and the side a merged row doesn't show.
    assert_eq!(jaw(&description, "jaw_shift", true), ["jaw_shift"]);
    assert_eq!(
        jaw(&description, "right jaw width", true),
        ["jaw_width_left"],
        "the pair's one row"
    );
    // A pair whose sides differ shows both: only the side asked for.
    description.parameters.insert("jaw_width_left".into(), 0.3);
    assert_eq!(
        jaw(&description, "right jaw width", true),
        ["jaw_width_right"]
    );
    // Another group has its own: the eyes' depth is an Advanced control.
    assert!(found(&base, &description, "nose", "jaw", true).is_empty());
    assert!(
        ids(&found(&base, &description, "eyes", "depth", true)).contains(&"eye_depth_left".into())
    );
    // What a control does (its `about`) is searched too.
    let told = base
        .parameters
        .iter()
        .find(|p| {
            p.about
                .as_deref()
                .is_some_and(|about| about.split_whitespace().count() > 2)
        })
        .expect("a parameter that says what it does");
    let word = told
        .about
        .as_deref()
        .unwrap()
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .max_by_key(|word| word.len())
        .unwrap();
    assert!(matches(told, word), "{word} in {:?}", told.about);
    assert!(matches(told, ""), "no search matches everything");
    assert!(!matches(told, "zzzz"));
}

/// Picture tiles: as many across as fit at about 120 points each (three
/// in the panel at its default width), never fewer than two.
#[test]
fn tiles_are_as_many_across_as_fit_the_panel() {
    let gap = super::browser::GAP;
    // The panel's width less its margins (12 points a side).
    for (panel, columns) in [(320.0, 2), (400.0, 3), (520.0, 4), (640.0, 5), (270.0, 2)] {
        let width = panel - 24.0;
        assert_eq!(super::tile_columns(width, gap), columns, "{panel}");
        let tile = (width - gap * (columns as f32 - 1.0)) / columns as f32;
        assert!(
            (100.0..=150.0).contains(&tile),
            "{panel}: tiles {tile} wide"
        );
    }
}

#[test]
fn a_change_hidden_by_the_level_is_counted_with_the_level_that_shows_it() {
    let (_, base) = realistic();
    let mut description = Description::new("x", DEFAULT_BASE);
    let face = axcenity_avatar::Category::Face;
    assert_eq!(
        hidden_changes(&base, &description, face, Level::Detailed),
        None
    );
    // An advanced slider and a detailed one (nose_length is quick: always shown).
    description.parameters.insert("jaw_shift".into(), 0.4);
    description.parameters.insert("nose_tip".into(), 0.4);
    assert_eq!(
        hidden_changes(&base, &description, face, Level::Detailed),
        Some((1, Level::Advanced))
    );
    assert_eq!(
        hidden_changes(&base, &description, face, Level::Quick),
        Some((2, Level::Advanced))
    );
    assert_eq!(
        hidden_changes(&base, &description, face, Level::Advanced),
        None
    );
}

#[test]
fn the_navigation_is_in_the_briefs_order_with_features_from_detailed() {
    let (_, base) = realistic();
    let labels: Vec<&str> = Category::ALL.iter().map(|c| nav::label(*c)).collect();
    assert_eq!(
        labels,
        [
            "Body",
            "Face",
            "Hair",
            "Clothing",
            "Skin and Makeup",
            "Accessories",
            "Expressions",
            "Presets"
        ]
    );
    let mut description = Description::new("x", DEFAULT_BASE);
    description.parameters.insert("nose_length".into(), 0.4);
    description.parameters.insert("nose_tip".into(), 0.2);
    let face = nav::features(&base, &description, Category::Face, Level::Detailed);
    let face_ids: Vec<&str> = face.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        face_ids,
        [
            "face_shape",
            "eyes",
            "brows",
            "nose",
            "cheeks",
            "jaw",
            "chin",
            "lips",
            "forehead",
            "ears"
        ]
    );
    assert_eq!(face.iter().find(|f| f.id == "nose").unwrap().changed, 2);
    assert!(nav::features(&base, &description, Category::Face, Level::Quick).is_empty());
    assert!(nav::features(&base, &description, Category::Hair, Level::Advanced).is_empty());
    // The body's features are its regions (the manifest's `features`),
    // listed from Detailed; hands and feet have only advanced sliders.
    let body = |level| -> Vec<String> {
        nav::features(&base, &description, Category::Body, level)
            .into_iter()
            .map(|f| f.id)
            .collect()
    };
    let detailed = body(Level::Detailed);
    for region in ["shoulders", "chest", "abdomen", "waist", "hips", "thighs"] {
        assert!(
            detailed.contains(&region.to_owned()),
            "{region} in {detailed:?}"
        );
    }
    assert!(
        !detailed.contains(&"build_regions".to_owned()),
        "groups aren't features"
    );
    assert!(!detailed.contains(&"hands".to_owned()));
    assert!(body(Level::Advanced).contains(&"hands".to_owned()));
    assert_eq!(nav::changed(&base, &description, Category::Face), 2);
    assert_eq!(nav::changed(&base, &description, Category::Body), 0);
}

#[test]
fn reset_offers_everything_the_category_and_the_picked_feature() {
    let folder = test_folder("panel-reset");
    let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let library = workspace.library();
    let mut selection = Selection {
        category: Category::Face,
        feature: Some("nose".into()),
        level: Level::Detailed,
    };
    let choices = toolbar::reset_choices(&selection, &workspace, &library);
    let scopes: Vec<(&str, &Scope)> = choices.iter().map(|(l, _, s)| (l.as_str(), s)).collect();
    assert_eq!(
        scopes,
        [
            ("Everything", &Scope::All),
            (
                "Face only",
                &Scope::Category(axcenity_avatar::Category::Face)
            ),
            ("Nose only", &Scope::Group("nose".into())),
        ]
    );
    selection.pick_category(Category::Presets);
    assert_eq!(
        toolbar::reset_choices(&selection, &workspace, &library).len(),
        1
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_skin_page_offers_the_skin_models_values_and_says_which_are_drawn() {
    let (_, base) = realistic();
    let skin = base
        .material("Skin")
        .expect("the realistic body's skin slot");
    assert_eq!(skin.shading, axcenity_avatar::ShadingModel::Skin);
    let names: Vec<&str> = skin.shading.values().iter().map(|v| v.name).collect();
    assert!(
        names.contains(&"tint") && names.contains(&"melanin"),
        "{names:?}"
    );
    assert!(DRAWN_TODAY.contains(&"tint"));
    // Since R5 the Skin model draws all of its values; the Eye model's
    // aren't drawn yet.
    assert!(DRAWN_TODAY.contains(&"melanin") && DRAWN_TODAY.contains(&"softness"));
    assert!(!DRAWN_TODAY.contains(&"pupil_size"));
    assert_eq!(value_label("roughness_scale"), "Roughness scale");
}
