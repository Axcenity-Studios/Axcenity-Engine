//! Content states (Paul's decision 12, ADR-0043): how a manifest says its
//! state, what unset and an unknown state read as, the check every shipped
//! item passes, and which slots Randomize may draw in.

use super::texts;
use crate::{ItemManifest, Quality, check_declared_state, fixtures, production_slots};

/// The fixture tee with `quality` set to `state` (raw JSON), or left out.
fn tee(state: Option<&str>) -> Result<ItemManifest, crate::FormatError> {
    let text = match state {
        Some(state) => fixtures::TEE.replacen(
            "\"kind\": \"garment\",",
            &format!("\"kind\": \"garment\", \"quality\": \"{state}\","),
            1,
        ),
        None => fixtures::TEE.to_owned(),
    };
    ItemManifest::from_json(&text)
}

#[test]
fn the_three_states_read_and_write_and_unset_is_legacy() {
    for (text, state) in [
        (Some("production"), Quality::Production),
        (Some("legacy"), Quality::Legacy),
        (Some("draft"), Quality::Draft),
        (None, Quality::Legacy),
    ] {
        let item = tee(text).expect("reads");
        assert_eq!(item.state(), state, "{text:?}");
        assert_eq!(item.is_production(), state == Quality::Production);
        assert_eq!(item.is_legacy(), state == Quality::Legacy);
        assert_eq!(item.is_draft(), state == Quality::Draft);
        // Written as read: unset stays unset (an older project's file is
        // never given a state it didn't say).
        let again = ItemManifest::from_json(&item.to_json()).unwrap();
        assert_eq!(again.quality, item.quality, "{text:?}");
    }
    assert!(
        tee(Some("legacy"))
            .unwrap()
            .to_json()
            .contains("\"quality\": \"legacy\"")
    );
}

/// A newer engine's state doesn't stop the manifest loading: it reads as a
/// draft (loads and draws, never offered or drawn at random).
#[test]
fn an_unknown_state_loads_as_a_draft() {
    let item = tee(Some("approved")).expect("an unknown state still reads");
    assert_eq!(item.quality, Some(Quality::Other));
    assert_eq!(item.state(), Quality::Draft);
    assert!(item.validate().is_empty(), "{:?}", item.validate());
}

/// Every shipped item says its state, one of the three
/// (`check_declared_state`, which `tests/content_states.rs` runs over the
/// repository's library).
#[test]
fn a_shipped_item_must_declare_one_of_the_three_states() {
    for (text, problems) in [
        (Some("production"), 0),
        (Some("legacy"), 0),
        (Some("draft"), 0),
        (None, 1),
        (Some("approved"), 1),
    ] {
        let found = check_declared_state(&tee(text).unwrap());
        assert_eq!(found.len(), problems, "{text:?}: {:?}", texts(&found));
    }
    let unset = check_declared_state(&tee(None).unwrap());
    assert!(texts(&unset)[0].contains("quality"), "{:?}", texts(&unset));
}

/// The slots Randomize may draw in: those with a Production item for the
/// base, and only those.
#[test]
fn production_slots_are_the_slots_with_a_production_item_for_the_base() {
    let mut library = fixtures::rich_library();
    let all: Vec<String> = production_slots(&library, "test/body")
        .into_iter()
        .collect();
    assert_eq!(all, ["bottom", "hair", "outerwear", "top", "wrist"]);
    for id in ["test/hair", "test/bob"] {
        library.items.get_mut(id).unwrap().quality = Some(Quality::Legacy);
    }
    library.items.get_mut("test/hood").unwrap().quality = Some(Quality::Draft);
    let some: Vec<String> = production_slots(&library, "test/body")
        .into_iter()
        .collect();
    assert_eq!(some, ["bottom", "top", "wrist"]);
    assert!(production_slots(&library, "test/other").is_empty());
}
