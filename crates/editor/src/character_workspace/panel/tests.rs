//! The panel's and the navigation's choices of what to show, against the
//! realistic body's manifest.

use std::sync::Arc;

use axcenity_avatar::{BaseManifest, Description, Library};

use super::super::edit::Scope;
use super::super::selection::{Level, Selection};
use super::super::{
    Category, CharacterWorkspace, DEFAULT_BASE, find_character_library, nav, toolbar,
};
use super::materials::{DRAWN_TODAY, value_label};
use super::parameters::{hidden_changes, rows};
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
            "eye_tilt_left"
        ]
    );
    // Advanced adds the depth; Symmetry off shows both sides.
    assert!(ids(&eyes(&description, Level::Advanced, true)).contains(&"eye_depth_left".into()));
    assert_eq!(eyes(&description, Level::Detailed, false).len(), 8);
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
            "eyes", "brows", "nose", "cheeks", "jaw", "chin", "lips", "forehead", "ears"
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
    let workspace = CharacterWorkspace::open_preset(&folder, 0).unwrap();
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
    assert!(!DRAWN_TODAY.contains(&"melanin"));
    assert_eq!(value_label("roughness_scale"), "Roughness scale");
}
