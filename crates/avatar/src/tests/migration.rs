//! Format 1 (Character Studio CS2's `Look`) migrates to format 2 as the
//! character format 1 showed.

use std::collections::BTreeMap;

use crate::{DESCRIPTION_VERSION, Description, V1_BASE, V1_ITEMS, is_valid_id};

fn item_ids(description: &Description) -> Vec<&str> {
    description
        .items
        .iter()
        .map(|item| item.id.as_str())
        .collect()
}

#[test]
fn a_format_1_file_migrates_to_the_same_character() {
    // The workspace's "Casual" preset, as CS2 saved it.
    let v1 = r#"{
      "format_version": 1,
      "name": "Casual",
      "build": -0.4,
      "muscle": 0.0,
      "hair": "Hair_Bob",
      "top": "Shirt_Tee",
      "outerwear": null,
      "bottom": "Pants_Cargo",
      "shoes": "Shoes_Sneakers"
    }"#;
    let description = Description::from_json(v1).unwrap();
    assert_eq!(description.format_version, DESCRIPTION_VERSION);
    assert_eq!(description.name, "Casual");
    assert_eq!(description.base, V1_BASE);
    assert_eq!(
        description.parameters,
        BTreeMap::from([("build".to_owned(), -0.4), ("muscle".to_owned(), 0.0)])
    );
    assert_eq!(
        item_ids(&description),
        [
            "axcenity/hair-bob",
            "axcenity/tee",
            "axcenity/cargo-pants",
            "axcenity/sneakers"
        ],
        "hair, top, outerwear, bottom, shoes: format 1's slot order"
    );
    assert!(description.items.iter().all(|item| item.variant.is_none()));
    assert!(description.materials.is_empty() && description.extra.is_empty());
}

#[test]
fn format_1_defaults_ranges_and_unknown_parts_carry_over() {
    // Missing values were the default character's; values out of range
    // were brought in; an empty name was "New Character"; a part the parts
    // file doesn't have wasn't worn; no version meant format 1.
    let v1 = r#"{ "name": "  ", "build": 4.0, "muscle": -2, "top": "Shirt_Fancy", "shoes": null }"#;
    let description = Description::from_json(v1).unwrap();
    assert_eq!(description.name, "New Character");
    assert_eq!(description.parameters["build"], 1.0);
    assert_eq!(description.parameters["muscle"], 0.0);
    assert_eq!(
        item_ids(&description),
        [
            "axcenity/hair-swept",
            "axcenity/cropped-jacket",
            "axcenity/cargo-pants"
        ]
    );
}

#[test]
fn an_empty_format_1_file_is_the_default_character() {
    let description = Description::from_json(r#"{"format_version": 1}"#).unwrap();
    assert_eq!(description.name, "New Character");
    assert_eq!(
        item_ids(&description),
        [
            "axcenity/hair-swept",
            "axcenity/tee",
            "axcenity/cropped-jacket",
            "axcenity/cargo-pants",
            "axcenity/sneakers"
        ]
    );
}

#[test]
fn a_migrated_file_saves_as_format_2_and_reads_back_the_same() {
    let v1 = r#"{"format_version": 1, "name": "Athlete", "muscle": 0.9, "hair": "Hair_Buzz", "outerwear": null}"#;
    let description = Description::from_json(v1).unwrap();
    let text = description.to_json();
    assert!(text.contains("\"format_version\": 2"), "{text}");
    assert_eq!(Description::from_json(&text).unwrap(), description);
}

#[test]
fn every_format_1_part_maps_to_its_own_item() {
    let mut ids: Vec<&str> = V1_ITEMS.iter().map(|(_, id)| *id).collect();
    assert!(ids.iter().all(|id| is_valid_id(id)), "{ids:?}");
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), V1_ITEMS.len());
    assert!(is_valid_id(V1_BASE));
}

#[test]
fn a_damaged_format_1_file_is_an_error() {
    for text in [
        r#"{"format_version": 1, "build": "big"}"#,
        r#"{"format_version": 1, "hair": 3}"#,
        r#"[1]"#,
    ] {
        assert!(Description::from_json(text).is_err(), "{text}");
    }
}
