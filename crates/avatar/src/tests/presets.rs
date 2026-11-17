//! Presets as data (plan section 11.3): a `*.preset.json` is a format 2
//! description with a `preset` block, and applying one copies only what
//! its kind owns.

use std::collections::BTreeMap;

use serde_json::json;

use super::{assert_problem, texts};
use crate::fixtures::{self, preset};
use crate::{
    Catalogue, CataloguePreset, Colour, Description, Library, MaterialValue, PresetKind, WornItem,
    apply_preset, check_preset, matches_preset,
};

/// A character with something set in every part a preset can own.
fn character() -> Description {
    let mut description = Description::new("Ada", "test/body");
    description.parameters = BTreeMap::from([
        ("frame".to_owned(), 1.0),
        ("fat".to_owned(), 0.1),
        ("height".to_owned(), 1.2),
        ("eye_size_left".to_owned(), -0.4),
        ("eye_size_right".to_owned(), -0.4),
        ("tone".to_owned(), 0.3),
    ]);
    description.items = vec![
        WornItem::new("test/hair"),
        WornItem::new("test/tee"),
        WornItem::new("test/watch"),
    ];
    description.materials.insert(
        "Skin".to_owned(),
        BTreeMap::from([("melanin".to_owned(), MaterialValue::Number(0.2))]),
    );
    description
        .extra
        .insert("note".to_owned(), json!("mine, kept"));
    description
}

#[test]
fn a_preset_is_a_description_with_a_preset_block() {
    let text = r#"{
      "format_version": 2, "name": "Athletic", "base": "test/body",
      "parameters": { "muscle": 0.7, "fat": 0.2 },
      "preset": { "id": "test/body-athletic", "kind": "body", "about": "Lean and muscular.",
                  "tags": ["build"], "thumbnail": "athletic.png", "order": 30 }
    }"#;
    let description = Description::from_json(text).unwrap();
    let block = description.preset().unwrap();
    assert_eq!(block.id, "test/body-athletic");
    assert_eq!(block.kind, PresetKind::Body);
    assert_eq!(block.about, "Lean and muscular.");
    assert_eq!(block.tags, ["build"]);
    assert_eq!(block.thumbnail.as_deref(), Some("athletic.png"));
    assert_eq!(block.order, Some(30));
    // It survives a round trip as any unknown field does.
    let again = Description::from_json(&description.to_json()).unwrap();
    assert_eq!(again.preset(), Some(block));
    assert_eq!(character().preset(), None);
    assert_eq!(
        texts(&check_preset(&description, &fixtures::rich_library())),
        Vec::<String>::new()
    );
}

#[test]
fn a_body_preset_sets_the_body_and_nothing_else() {
    let library = fixtures::rich_library();
    let athletic = preset(
        "body",
        "athletic",
        r#""parameters": { "muscle": 0.7, "fat": 0.2 },"#,
    );
    let mut description = character();
    apply_preset(&mut description, &athletic, &library).unwrap();
    assert_eq!(description.name, "Ada", "the name is the character's");
    assert_eq!(description.parameters["muscle"], 0.7);
    assert_eq!(description.parameters["fat"], 0.2);
    // A body parameter the preset leaves out goes back to its default...
    assert!(!description.parameters.contains_key("height"));
    // ...but one with choices (the frame) stays unless the preset names it.
    assert_eq!(description.parameters["frame"], 1.0);
    // The face, skin, items and materials are the character's.
    assert_eq!(description.parameters["eye_size_left"], -0.4);
    assert_eq!(description.parameters["tone"], 0.3);
    assert_eq!(description.items, character().items);
    assert_eq!(description.materials, character().materials);
    // The preset block never comes along; the character's own extra stays.
    assert!(description.preset().is_none());
    assert_eq!(description.extra["note"], json!("mine, kept"));
    assert!(matches_preset(&description, &athletic, &library));
    description.parameters.insert("muscle".to_owned(), 0.6);
    assert!(!matches_preset(&description, &athletic, &library));
    // Naming the frame sets it.
    let male = preset("body", "male", r#""parameters": { "frame": 0 },"#);
    apply_preset(&mut description, &male, &library).unwrap();
    assert_eq!(description.parameters["frame"], 0.0);
    assert!(!description.parameters.contains_key("muscle"));
}

#[test]
fn a_face_preset_is_a_complete_face() {
    let library = fixtures::rich_library();
    let wide = preset(
        "face",
        "wide-eyes",
        r#""parameters": { "eye_size_left": 0.5 },"#,
    );
    let mut description = character();
    apply_preset(&mut description, &wide, &library).unwrap();
    assert_eq!(description.parameters["eye_size_left"], 0.5);
    assert!(!description.parameters.contains_key("eye_size_right"));
    assert_eq!(description.parameters["fat"], 0.1);
    assert_eq!(description.parameters["height"], 1.2);
    assert_eq!(description.items, character().items);
}

#[test]
fn hair_and_outfit_presets_set_their_slots() {
    let library = fixtures::rich_library();
    let bob = preset(
        "hair",
        "bob",
        r#""items": [{ "id": "test/bob", "variant": "blonde", "parameters": { "length": 0.4 } }],"#,
    );
    let mut description = character();
    apply_preset(&mut description, &bob, &library).unwrap();
    let ids: Vec<&str> = description.items.iter().map(|w| w.id.as_str()).collect();
    assert_eq!(ids, ["test/tee", "test/watch", "test/bob"]);
    assert_eq!(description.items[2].parameters["length"], 0.4);
    assert_eq!(description.parameters, character().parameters);
    // A hair preset that wears nothing is bald.
    let bald = preset("hair", "bald", "");
    apply_preset(&mut description, &bald, &library).unwrap();
    let ids: Vec<&str> = description.items.iter().map(|w| w.id.as_str()).collect();
    assert_eq!(ids, ["test/tee", "test/watch"]);

    // An outfit takes every clothing and accessory slot, not the hair.
    let summer = preset("outfit", "summer", r#""items": [{ "id": "test/dress" }],"#);
    let mut description = character();
    apply_preset(&mut description, &summer, &library).unwrap();
    let ids: Vec<&str> = description.items.iter().map(|w| w.id.as_str()).collect();
    assert_eq!(ids, ["test/hair", "test/dress"]);
}

#[test]
fn skin_and_character_presets() {
    let library = fixtures::rich_library();
    let skin = preset(
        "skin",
        "deep",
        r##""parameters": { "tone": 0.8 }, "materials": { "Skin": { "tint": "#f0e0d0" } },"##,
    );
    let mut description = character();
    apply_preset(&mut description, &skin, &library).unwrap();
    assert_eq!(description.parameters["tone"], 0.8);
    assert_eq!(
        description.materials["Skin"],
        BTreeMap::from([(
            "tint".to_owned(),
            MaterialValue::Colour(Colour([240, 224, 208]))
        )])
    );
    assert_eq!(description.parameters["fat"], 0.1);
    assert_eq!(description.items, character().items);

    // A character preset is everything but the name (and its block).
    let whole = preset(
        "character",
        "whole",
        r#""parameters": { "fat": 0.9 }, "items": [{ "id": "test/bob" }],"#,
    );
    let mut description = character();
    apply_preset(&mut description, &whole, &library).unwrap();
    assert_eq!(description.name, "Ada");
    assert_eq!(
        description.parameters,
        BTreeMap::from([("fat".to_owned(), 0.9)])
    );
    assert_eq!(description.items, vec![WornItem::new("test/bob")]);
    assert!(description.materials.is_empty());
    assert!(description.preset().is_none());
    assert_eq!(description.extra["note"], json!("mine, kept"));
}

#[test]
fn a_preset_that_cant_apply_says_why() {
    let library = fixtures::rich_library();
    let mut description = character();
    // Not a preset.
    assert!(apply_preset(&mut description, &character(), &library).is_err());
    // Another base model's body preset.
    let mut elsewhere = preset("body", "other", r#""parameters": { "fat": 0.5 },"#);
    elsewhere.base = "test/other".to_owned();
    let err = apply_preset(&mut description, &elsewhere, &library).unwrap_err();
    assert!(err.message.contains("test/other"), "{err}");
    // A kind this version doesn't know.
    let newer = preset("tattoos", "ink", "");
    assert_eq!(newer.preset().unwrap().kind, PresetKind::Other);
    assert!(apply_preset(&mut description, &newer, &library).is_err());
    assert_eq!(description, character(), "nothing changed");
}

#[test]
fn a_preset_with_mistakes_reports_each() {
    let library = fixtures::rich_library();
    // A face preset that sets the body and wears a shirt.
    let face = preset(
        "face",
        "busy",
        r#""parameters": { "eye_size_left": 0.5, "fat": 0.4 }, "items": [{ "id": "test/tee" }],"#,
    );
    let problems = check_preset(&face, &library);
    assert_problem(&problems, &["parameters.fat", "face"]);
    assert_problem(&problems, &["items[0]", "test/tee", "face"]);
    // A hair preset that wears a shirt; a body preset with materials.
    let hair = preset("hair", "shirt", r#""items": [{ "id": "test/tee" }],"#);
    assert_problem(&check_preset(&hair, &library), &["items[0]", "test/tee"]);
    let body = preset(
        "body",
        "tinted",
        r##""materials": { "Skin": { "melanin": 0.5 } },"##,
    );
    assert_problem(&check_preset(&body, &library), &["materials.Skin", "body"]);
    // A bad block, and an unsound description.
    let mut bad = preset("body", "bad", r#""parameters": { "fat": 4 },"#);
    bad.extra.insert(
        "preset".to_owned(),
        json!({ "id": "Bad Id", "kind": "body", "about": "", "thumbnail": "C:/x.png" }),
    );
    let problems = check_preset(&bad, &library);
    assert_problem(&problems, &["preset.id", "Bad Id"]);
    assert_problem(&problems, &["preset.about"]);
    assert_problem(&problems, &["preset.thumbnail"]);
    assert_problem(&problems, &["parameters.fat", "range"]);
    assert_problem(&check_preset(&character(), &library), &["preset"]);
}

#[test]
fn the_library_holds_presets_and_the_catalogue_lists_them() {
    let mut library = fixtures::rich_library();
    let mut athletic = preset("body", "athletic", r#""parameters": { "muscle": 0.7 },"#);
    athletic.extra.insert(
        "preset".to_owned(),
        json!({ "id": "test/athletic", "kind": "body", "about": "Lean.",
                "tags": ["build"], "thumbnail": "athletic.png" }),
    );
    library
        .add_preset("test/presets/athletic.preset.json", athletic.clone())
        .unwrap();
    library
        .add_preset(
            "test/presets/round.preset.json",
            preset(
                "face",
                "round",
                r#""parameters": { "eye_size_left": 0.2 },"#,
            ),
        )
        .unwrap();
    assert!(library.add_preset("x.preset.json", character()).is_err());
    assert_eq!(library.preset("test/athletic"), Some(&athletic));
    assert_eq!(
        library.preset_path("test/athletic"),
        Some("test/presets/athletic.preset.json")
    );
    assert_eq!(texts(&library.validate()), Vec::<String>::new());
    let catalogue = Catalogue::of(&library);
    assert_eq!(
        catalogue.presets,
        [
            CataloguePreset {
                id: "test/athletic".to_owned(),
                file: "test/presets/athletic.preset.json".to_owned(),
                label: "athletic".to_owned(),
                kind: PresetKind::Body,
                base: "test/body".to_owned(),
                tags: vec!["build".to_owned()],
                thumbnail: Some("test/presets/athletic.png".to_owned()),
            },
            CataloguePreset {
                id: "test/round".to_owned(),
                file: "test/presets/round.preset.json".to_owned(),
                label: "round".to_owned(),
                kind: PresetKind::Face,
                base: "test/body".to_owned(),
                tags: Vec::new(),
                thumbnail: None,
            },
        ]
    );
    assert_eq!(
        Catalogue::from_json(&catalogue.to_json()).unwrap(),
        catalogue
    );
    // A catalogue without presets writes no `presets` key.
    let plain = Catalogue::of(&fixtures::library());
    assert!(!plain.to_json().contains("presets"));
    // An unsound preset is the library's problem.
    library
        .add_preset(
            "test/presets/busy.preset.json",
            preset("face", "busy", r#""parameters": { "fat": 0.4 },"#),
        )
        .unwrap();
    assert_problem(&library.validate(), &["test/busy", "fat"]);
}

#[test]
fn a_library_folder_reads_its_presets() {
    let root = std::env::temp_dir().join(format!(
        "axcenity-avatar-presets-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("test/presets")).unwrap();
    let mut library = Library::new();
    library.add_base("test/Body.base.json", fixtures::rich_base());
    let round = preset(
        "face",
        "round",
        r#""parameters": { "eye_size_left": 0.2 },"#,
    );
    library
        .add_preset("test/presets/round.preset.json", round.clone())
        .unwrap();
    std::fs::write(root.join("test/Body.base.json"), fixtures::RICH_BASE).unwrap();
    std::fs::write(root.join("test/presets/round.preset.json"), round.to_json()).unwrap();
    let catalogue = Catalogue::of(&library);
    std::fs::write(root.join("catalogue.json"), catalogue.to_json()).unwrap();
    let (read, read_catalogue, problems) = Library::read(&root);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(read_catalogue, Some(catalogue.clone()));
    assert_eq!(read.preset("test/round"), Some(&round));
    // A listed preset whose block names another ID is a problem.
    let mut wrong = catalogue;
    wrong.presets[0].id = "test/oval".to_owned();
    std::fs::write(root.join("catalogue.json"), wrong.to_json()).unwrap();
    let (read, _, problems) = Library::read(&root);
    assert!(read.preset("test/oval").is_none());
    assert_problem(&problems, &["round.preset.json", "test/oval"]);
    let _ = std::fs::remove_dir_all(&root);
}
