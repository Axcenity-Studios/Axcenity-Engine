//! Every file round-trips; newer files are refused; damaged ones are
//! errors, never panics.

use std::collections::BTreeMap;

use serde_json::json;

use crate::fixtures::{self, BASE, HAIR, TEE, WATCH};
use crate::{
    BaseManifest, Catalogue, Colour, Description, FormatError, ItemManifest, MaterialValue,
    WornItem,
};

fn full_description() -> Description {
    let mut description = Description::new("Ada", "test/body");
    description.parameters = BTreeMap::from([
        ("frame".to_owned(), 0.6),
        ("build".to_owned(), -0.3),
        ("height".to_owned(), 1.04),
    ]);
    let mut tee = WornItem::new("test/tee");
    tee.variant = Some("navy".to_owned());
    tee.materials.insert(
        "Shirt".to_owned(),
        BTreeMap::from([(
            "tint".to_owned(),
            MaterialValue::Colour(Colour([16, 32, 48])),
        )]),
    );
    description.items = vec![tee, WornItem::new("test/hair")];
    description.materials.insert(
        "Skin".to_owned(),
        BTreeMap::from([
            ("melanin".to_owned(), MaterialValue::Number(0.4)),
            (
                "tint".to_owned(),
                MaterialValue::Colour(Colour([240, 224, 208])),
            ),
        ]),
    );
    description.extra.insert(
        "skin_layers".to_owned(),
        json!([{ "kind": "freckles", "amount": 0.3 }]),
    );
    description
}

#[test]
fn a_description_round_trips_with_every_field() {
    let description = full_description();
    let text = description.to_json();
    let back = Description::from_json(&text).unwrap();
    assert_eq!(back, description);
    assert_eq!(back.to_json(), text, "written the same way again");
    assert!(text.contains("\"format_version\": 2"), "{text}");
    assert!(text.ends_with('\n'));
}

#[test]
fn fields_a_newer_minor_version_adds_survive_a_round_trip() {
    let text = r##"{
        "format_version": 2, "name": "Bo", "base": "test/body",
        "makeup": { "lipstick": "#aa2233" }, "future_flag": true
    }"##;
    let description = Description::from_json(text).unwrap();
    assert_eq!(description.extra["future_flag"], json!(true));
    let again = Description::from_json(&description.to_json()).unwrap();
    assert_eq!(again.extra["makeup"], json!({ "lipstick": "#aa2233" }));
}

#[test]
fn a_newer_description_is_refused_with_a_message() {
    let err =
        Description::from_json(r#"{"format_version": 3, "name": "X", "base": "a/b"}"#).unwrap_err();
    assert_eq!(
        err,
        FormatError::Newer {
            what: "character",
            found: 3,
            supported: 2
        }
    );
    assert!(err.to_string().contains("newer Axcenity"), "{err}");
}

#[test]
fn damaged_files_are_errors_not_panics() {
    for text in [
        "",
        "{",
        "[1, 2]",
        r#"{"format_version": "two"}"#,
        r#"{"format_version": 2}"#,
        r#"{"format_version": 2, "name": "X", "base": "a/b", "parameters": {"x": "big"}}"#,
        r##"{"format_version": 2, "name": "X", "base": "a/b", "materials": {"Skin": {"tint": "#zz0000"}}}"##,
        r#"{"format_version": -1}"#,
    ] {
        assert!(
            matches!(Description::from_json(text), Err(FormatError::Damaged(_))),
            "{text}"
        );
    }
    for text in ["", "{}", "[]", r#"{"format_version": 1, "id": 4}"#] {
        assert!(BaseManifest::from_json(text).is_err(), "{text}");
        assert!(ItemManifest::from_json(text).is_err(), "{text}");
    }
    for text in ["", "[]", r#"{"format_version": 1, "items": 4}"#] {
        assert!(Catalogue::from_json(text).is_err(), "{text}");
    }
}

#[test]
fn manifests_round_trip() {
    let base = fixtures::base();
    assert_eq!(BaseManifest::from_json(&base.to_json()).unwrap(), base);
    // What the fixture says arrives: defaults filled in, rules typed.
    assert_eq!(base.parameter("height").unwrap().default, 1.0);
    assert!(base.parameter("eye_size_left").unwrap().advanced);
    assert_eq!(base.constraints.len(), 2);
    for text in [TEE, HAIR, WATCH] {
        let item = fixtures::item(text);
        assert_eq!(ItemManifest::from_json(&item.to_json()).unwrap(), item);
    }
    let watch = fixtures::item(WATCH);
    assert_eq!(watch.tier, crate::ContentTier::BuiltIn, "the default tier");
    assert_eq!(fixtures::item(HAIR).tier, crate::ContentTier::Premium);
    let catalogue = Catalogue::of(&fixtures::library());
    assert_eq!(
        Catalogue::from_json(&catalogue.to_json()).unwrap(),
        catalogue
    );
}

#[test]
fn newer_manifests_and_catalogues_are_refused() {
    let newer = BASE.replacen("\"format_version\": 1", "\"format_version\": 2", 1);
    assert!(matches!(
        BaseManifest::from_json(&newer),
        Err(FormatError::Newer { found: 2, .. })
    ));
    let newer = TEE.replacen("\"format_version\": 1", "\"format_version\": 9", 1);
    assert!(matches!(
        ItemManifest::from_json(&newer),
        Err(FormatError::Newer { found: 9, .. })
    ));
    assert!(matches!(
        Catalogue::from_json(r#"{"format_version": 5}"#),
        Err(FormatError::Newer { found: 5, .. })
    ));
}

#[test]
fn the_catalogue_lists_each_manifest_with_its_browsing_fields() {
    let catalogue = Catalogue::of(&fixtures::library());
    let ids: Vec<&str> = catalogue
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(ids, ["test/hair", "test/tee", "test/watch"], "by ID");
    let tee = &catalogue.items[1];
    assert_eq!(tee.manifest, "test/items/tee.item.json");
    assert_eq!(tee.slot, "top");
    assert_eq!(tee.thumbnail.as_deref(), Some("test/items/tee.png"));
    assert_eq!(catalogue.bases[0].manifest, "test/Body.base.json");
}
