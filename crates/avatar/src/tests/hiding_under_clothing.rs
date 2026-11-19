//! Hiding under clothing (wave 5, stage 3): an item's `hide_maps` (masks
//! of the body's and lower layers' triangles it hides) read, check and
//! resolve.

use super::assert_problem;
use crate::{Description, HideMap, ItemManifest, WornItem, fixtures, resolve};

/// The test tee with masks for the body, the watch (worn as a lower layer
/// here) and a cape the library hasn't.
fn tee_with_masks() -> ItemManifest {
    let mut tee = fixtures::item(fixtures::TEE);
    tee.hide_maps = vec![
        HideMap {
            under: None,
            map: "tee.hide.png".to_owned(),
        },
        HideMap {
            under: Some("test/watch".to_owned()),
            map: "tee.hide.watch.png".to_owned(),
        },
        HideMap {
            under: Some("test/cape".to_owned()),
            map: "tee.hide.cape.png".to_owned(),
        },
    ];
    tee
}

#[test]
fn hide_maps_round_trip_and_an_item_without_them_reads_as_before() {
    let tee = tee_with_masks();
    let text = tee.to_json();
    assert!(text.contains("\"hide_maps\""), "{text}");
    assert_eq!(ItemManifest::from_json(&text).unwrap(), tee);
    // Optional: an item without masks writes none, and reads with none.
    let plain = fixtures::item(fixtures::TEE);
    assert!(plain.hide_maps.is_empty());
    assert!(!plain.to_json().contains("hide_maps"));
    assert!(tee.validate().is_empty(), "{:?}", tee.validate());
}

#[test]
fn a_hide_map_must_be_a_path_name_another_item_and_be_one_per_model() {
    let mut tee = tee_with_masks();
    tee.hide_maps.push(HideMap {
        under: Some("test/tee".to_owned()),
        map: "../outside\\tee.png".to_owned(),
    });
    tee.hide_maps.push(HideMap {
        under: None,
        map: "again.png".to_owned(),
    });
    let problems = tee.validate();
    assert_problem(&problems, &["hide_maps[3].map", "relative path"]);
    assert_problem(&problems, &["hide_maps[3]", "itself"]);
    assert_problem(&problems, &["hide_maps[4]", "second mask"]);
}

#[test]
fn a_worn_items_masks_resolve_for_the_body_and_the_layers_worn_under_it() {
    let mut library = fixtures::library();
    library.add_item("test/items/tee.item.json", tee_with_masks());
    let mut description = Description::new("Ada", "test/body");
    description.items = vec![WornItem::new("test/watch"), WornItem::new("test/tee")];
    let resolved = resolve(&library, &description).unwrap();
    let maps: Vec<(Option<&str>, &str, &str)> = resolved
        .hide_maps
        .iter()
        .map(|m| (m.under.as_deref(), m.map.as_str(), m.by.as_str()))
        .collect();
    // The cape isn't worn: its mask is left out. Paths are the library's.
    assert_eq!(
        maps,
        [
            (None, "test/items/tee.hide.png", "test/tee"),
            (
                Some("test/watch"),
                "test/items/tee.hide.watch.png",
                "test/tee"
            ),
        ]
    );
    // Without the tee, nothing hides anything.
    description.items = vec![WornItem::new("test/watch")];
    assert!(
        resolve(&library, &description)
            .unwrap()
            .hide_maps
            .is_empty()
    );
    // Without the watch, only the body's mask.
    description.items = vec![WornItem::new("test/tee")];
    let resolved = resolve(&library, &description).unwrap();
    assert_eq!(resolved.hide_maps.len(), 1);
    assert_eq!(resolved.hide_maps[0].under, None);
}
