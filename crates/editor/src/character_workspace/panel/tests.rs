//! The panel's and the navigation's choices of what to show, against the
//! human base's manifest.

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

fn human() -> (Arc<Library>, BaseManifest) {
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
    let (_, base) = human();
    let mut description = Description::new("x", DEFAULT_BASE);
    let eyes = |description: &Description, level, symmetry| {
        rows(&base, description, "eyes", level, None, symmetry)
    };
    // Detailed with Symmetry: one row per pair, the lid gaps and the other
    // advanced controls left out.
    let shown = eyes(&description, Level::Detailed, true);
    assert!(shown.iter().all(|row| row.mirrored));
    assert_eq!(
        ids(&shown),
        [
            // Quick.
            "eyes_scale_left",
            "eyes_trans_in_out_left",
            // Detailed.
            "eyes_trans_down_up_left",
            "eyes_height2_left",
            "eyes_corner1_down_up_left",
            "eyes_corner2_down_up_left",
            "eyes_eyefold_down_up_left",
            "eyes_eyefold_concave_convex_left",
            "eyes_epicanthus_in_out_left",
            "eyes_bag_left"
        ]
    );
    // Advanced adds the inner lid gap; Symmetry off shows both sides.
    assert!(ids(&eyes(&description, Level::Advanced, true)).contains(&"eyes_height1_left".into()));
    assert_eq!(eyes(&description, Level::Detailed, false).len(), 20);
    // Audit D-11: a pair whose sides differ shows both, even with Symmetry.
    description.parameters.insert("eyes_scale_left".into(), 0.3);
    let shown = eyes(&description, Level::Detailed, true);
    let sizes: Vec<_> = shown
        .iter()
        .filter(|row| row.parameter.id.starts_with("eyes_scale"))
        .map(|row| (row.parameter.id.as_str(), row.mirrored))
        .collect();
    assert_eq!(
        sizes,
        [("eyes_scale_left", false), ("eyes_scale_right", false)]
    );
    // Quick shows the manifest's quick controls: the body type, the whole
    // body's build and its height, not the other proportions.
    let frame = rows(&base, &description, "frame", Level::Quick, None, true);
    assert_eq!(ids(&frame), ["frame"]);
    let build = rows(&base, &description, "build", Level::Quick, None, true);
    assert_eq!(ids(&build), ["muscle", "fat", "age"]);
    let proportions = |level| ids(&rows(&base, &description, "proportions", level, None, true));
    assert_eq!(proportions(Level::Quick), ["height"]);
    assert_eq!(
        proportions(Level::Detailed),
        [
            "height",
            "leg_length",
            "arm_length",
            "shoulder_width",
            "waist_width",
            "hip_width",
            "head_size"
        ]
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
    let (_, base) = human();
    let mut description = Description::new("x", DEFAULT_BASE);
    // (Of three of the ears' pairs: the manifest has more.)
    let ears = |description: &Description, search: &str, symmetry| {
        let mut ids = ids(&found(&base, description, "ears", search, symmetry));
        ids.retain(|id| {
            [
                "ears_scale_left",
                "ears_scale_right",
                "ears_scale_vert_left",
                "ears_scale_vert_right",
                "ears_flap_left",
                "ears_flap_right",
            ]
            .contains(&id.as_str())
        });
        ids
    };
    // The ears' rows at every level: their size is Quick, their height
    // Detailed, their rim Advanced. With Symmetry a pair is one row.
    assert_eq!(
        ears(&description, "ear", true),
        ["ears_scale_left", "ears_scale_vert_left", "ears_flap_left"]
    );
    assert_eq!(ears(&description, "ear", false).len(), 6, "both sides");
    // Every word must be there; capitals don't matter.
    assert_eq!(
        ears(&description, "EAR height", true),
        ["ears_scale_vert_left"]
    );
    assert!(ears(&description, "ear nostril", true).is_empty());
    // The ID is searched, and the side a merged row doesn't show.
    assert_eq!(ears(&description, "ears_flap", true), ["ears_flap_left"]);
    assert_eq!(
        ears(&description, "right ear size", true),
        ["ears_scale_left"],
        "the pair's one row"
    );
    // A pair whose sides differ shows both: only the side asked for.
    description.parameters.insert("ears_scale_left".into(), 0.3);
    assert_eq!(
        ears(&description, "right ear size", true),
        ["ears_scale_right"]
    );
    // Another group has its own: the nose's rows have no side, and its
    // width is found at every level (the nose's is Quick, the tip's
    // Detailed, the upper bridge's Advanced); the eyes' inner lid gap is an
    // Advanced control.
    assert!(found(&base, &description, "nose", "ear", true).is_empty());
    let widths = ids(&found(&base, &description, "nose", "width", true));
    for id in ["nose_scale_horiz", "nose_point_width", "nose_width1"] {
        assert!(widths.contains(&id.to_owned()), "{id} in {widths:?}");
    }
    assert!(
        ids(&found(&base, &description, "eyes", "lid gap", true))
            .contains(&"eyes_height1_left".into())
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
    let (_, base) = human();
    let mut description = Description::new("x", DEFAULT_BASE);
    let face = axcenity_avatar::Category::Face;
    assert_eq!(
        hidden_changes(&base, &description, face, Level::Detailed),
        None
    );
    // An advanced slider and a detailed one (a quick one, such as
    // nose_scale_vert, is always shown).
    description.parameters.insert("nose_width1".into(), 0.4);
    description.parameters.insert("nose_hump".into(), 0.4);
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
    let (_, base) = human();
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
    description.parameters.insert("nose_scale_vert".into(), 0.4);
    description.parameters.insert("nose_hump".into(), 0.2);
    let face = nav::features(&base, &description, Category::Face, Level::Detailed);
    let face_ids: Vec<&str> = face.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        face_ids,
        [
            "face_shape",
            "forehead",
            "brows",
            "eyes",
            "nose",
            "cheeks",
            "lips",
            "chin",
            "ears",
            "neck"
        ]
    );
    assert_eq!(face.iter().find(|f| f.id == "nose").unwrap().changed, 2);
    assert!(nav::features(&base, &description, Category::Face, Level::Quick).is_empty());
    assert!(nav::features(&base, &description, Category::Hair, Level::Advanced).is_empty());
    // The human base's manifest has no features for the body, so the
    // Body page lists its groups, from Detailed.
    let body = |level| -> Vec<String> {
        nav::features(&base, &description, Category::Body, level)
            .into_iter()
            .map(|f| f.id)
            .collect()
    };
    assert_eq!(body(Level::Detailed), ["frame", "build", "proportions"]);
    assert!(body(Level::Quick).is_empty());
    assert_eq!(body(Level::Advanced), body(Level::Detailed));
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
    let (_, base) = human();
    let skin = base.material("Skin").expect("the human base's skin slot");
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
