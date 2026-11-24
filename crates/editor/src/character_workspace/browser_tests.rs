//! The Hair and Clothing browsers (R6 CO25, R7 CO26): what is offered,
//! drafts (listed without asking where a page has nothing else), search
//! and filters, favourites and recently used, a hovered tile previewed
//! without committing, a click that wears it (and one on the worn tile
//! that takes it off) as one undo step, and the worn item's controls, only
//! what it declares. A worn garment stays on while the body is changed.
//! With `AXCENITY_SNAPSHOTS`, both pages are drawn for real
//! (`the_browsers_are_drawn`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axcenity_avatar::{
    Catalogue, Description, ItemManifest, LIBRARY_FOLDER, Library, MaterialRole, MaterialValue,
    V1_BASE,
};
use eframe::egui;

use super::browser::{self, Browse, Declared, Filters, Pick, Shelf};
use super::intent::{Gesture, Intent};
use super::shell_tests::{drawn_texts, input};
use super::{Category, CharacterWorkspace, DEFAULT_BASE, Start, find_character_library};
use crate::character_studio::test_folder;
use crate::theme;

/// An item manifest from its fields beyond the required ones.
fn item(id: &str, label: &str, kind: &str, slot: &str, bases: &[&str], more: &str) -> ItemManifest {
    let bases: Vec<String> = bases.iter().map(|b| format!("\"{b}\"")).collect();
    let layer = if kind == "garment" && !more.contains("\"layer\"") {
        "\"layer\": 1,"
    } else {
        ""
    };
    let fit = if more.contains("\"fit\"") {
        String::new()
    } else {
        r#""fit": { "method": "shapes", "follows": [] },"#.to_owned()
    };
    let text = format!(
        r#"{{ "format_version": 1, "id": "{id}", "label": "{label}", "kind": "{kind}",
             "slot": "{slot}", {layer} {fit} "bases": [{}] {more} }}"#,
        bases.join(", ")
    );
    ItemManifest::from_json(&text).unwrap_or_else(|err| panic!("{id}: {err}\n{text}"))
}

/// The items the listing tests use, on the realistic body.
fn fixtures() -> Vec<ItemManifest> {
    let r = &[DEFAULT_BASE];
    vec![
        item(
            "test/tee",
            "Crew tee",
            "garment",
            "top",
            r,
            r##", "quality": "production", "tags": ["casual", "cotton"],
               "materials": [{ "name": "Shirt", "shading": "cloth", "role": "primary" },
                             { "name": "Trim", "shading": "cloth", "role": "accent" }],
               "variants": [{ "id": "navy", "label": "Navy",
                              "materials": { "Shirt": { "tint": "#1f2a44" } } }]"##,
        ),
        item(
            "test/hoodie",
            "Zip hoodie",
            "garment",
            "outerwear",
            r,
            r#", "quality": "production", "tags": ["casual", "hood"], "layer": 3,
               "about": "A loose zip hoodie in fleece.",
               "materials": [{ "name": "Fleece", "shading": "cloth" }],
               "limitations": [{ "note": "Made for up to half the body fat.",
                                 "parameter": "fat", "range": [-1, 0.5] }]"#,
        ),
        item(
            "test/jeans",
            "Straight jeans",
            "garment",
            "bottom",
            r,
            r#", "quality": "draft", "tags": ["denim"]"#,
        ),
        item(
            "test/stylized-top",
            "Stylized top",
            "garment",
            "top",
            &[V1_BASE],
            r#", "quality": "production""#,
        ),
        item(
            "test/slip-dress",
            "Slip dress",
            "garment",
            "top",
            r,
            r#", "quality": "production", "also_fills": ["underwear_bottom"]"#,
        ),
        item(
            "test/vest",
            "Vest",
            "garment",
            "top",
            r,
            r#", "quality": "production", "tags": ["cotton"], "excludes": ["test/hoodie"]"#,
        ),
        item(
            "test/bob",
            "Plain bob",
            "groom",
            "hair",
            r,
            r#", "quality": "production", "tags": ["medium", "straight"],
               "groom": { "mesh": { "file": "x.glb" } },
               "materials": [{ "name": "Hair", "shading": "hair", "role": "primary" }]"#,
        ),
        item(
            "test/streaked",
            "Streaked waves",
            "groom",
            "hair",
            r,
            r#", "quality": "production", "tags": ["long", "wavy"],
               "groom": { "mesh": { "file": "x.glb" } },
               "materials": [{ "name": "Hair", "shading": "hair", "role": "primary" },
                             { "name": "Streaks", "shading": "hair", "role": "secondary" },
                             { "name": "Roots", "shading": "hair", "role": "accent",
                               "values": ["melanin", "redness"] }],
               "parameters": [{ "id": "length", "label": "Length", "range": [0, 1],
                                "default": 0.5 }]"#,
        ),
    ]
}

/// The repository's library with the fixtures added.
fn library() -> Arc<Library> {
    let shipped = find_character_library().expect("the repository's library");
    let (mut library, _, problems) = Library::read(&shipped);
    assert!(problems.is_empty(), "{problems:?}");
    for item in fixtures() {
        library.add_item(
            format!("test/{}.item.json", item.id.replace('/', "-")),
            item,
        );
    }
    Arc::new(library)
}

fn realistic(library: &Library) -> Description {
    axcenity_gameplay::description::default_description(library, DEFAULT_BASE)
}

/// The fixtures' IDs in a listing, in order: the shipped library's own
/// items (the realistic wardrobe since wave 4) come and go with the
/// content, so the listing tests look at the fixtures only.
fn ids(listing: &browser::Listing) -> Vec<String> {
    listing
        .entries
        .iter()
        .map(|entry| entry.item.id.clone())
        .filter(|id| id.starts_with("test/"))
        .collect()
}

fn listed(
    library: &Library,
    description: &Description,
    shelf: Shelf,
    filters: &Filters,
    drafts: bool,
) -> Vec<String> {
    ids(&browser::list(
        library,
        description,
        shelf,
        filters,
        drafts,
        &BTreeSet::new(),
        &[],
    ))
}

#[test]
fn only_items_for_this_base_in_a_slot_the_page_takes_are_offered() {
    let library = library();
    let description = realistic(&library);
    let listing = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        true,
        &BTreeSet::new(),
        &[],
    );
    let offered = ids(&listing);
    // The stylized top is for another base; the slip dress also fills the
    // underwear the body type wears, a slot nobody picks in.
    assert!(!offered.contains(&"test/stylized-top".to_owned()));
    assert!(!offered.contains(&"test/slip-dress".to_owned()));
    // No underwear tile: it comes with the body type and is said so.
    assert!(offered.iter().all(|id| !id.contains("briefs")));
    let wears = browser::body_type_wears(&library, &description, Shelf::Clothing);
    assert!(
        wears.iter().any(|(slot, _)| slot == "Underwear"),
        "{wears:?}"
    );
    // The categories are the page's slots a person picks in, in order.
    let slots: Vec<&str> = listing.slots.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(slots, ["top", "outerwear", "bottom", "shoes"]);
    // Grouped by slot in that order.
    assert_eq!(
        offered,
        ["test/tee", "test/vest", "test/hoodie", "test/jeans"]
    );
    // Hair lists grooms only.
    assert_eq!(
        listed(
            &library,
            &description,
            Shelf::Hair,
            &Filters::default(),
            true
        ),
        ["test/bob", "test/streaked"]
    );
}

#[test]
fn drafts_are_hidden_until_show_drafts_and_marked_when_shown() {
    let library = library();
    let mut description = realistic(&library);
    let off = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        false,
        &BTreeSet::new(),
        &[],
    );
    assert!(!ids(&off).contains(&"test/jeans".to_owned()));
    // The jeans and the shipped realistic drafts (the first garments).
    let shipped_drafts = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        true,
        &BTreeSet::new(),
        &[],
    )
    .entries
    .iter()
    .filter(|e| !e.item.id.starts_with("test/") && e.draft)
    .count();
    assert_eq!(off.hidden_drafts, 1 + shipped_drafts);
    let on = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        true,
        &BTreeSet::new(),
        &[],
    );
    let jeans = on
        .entries
        .iter()
        .find(|e| e.item.id == "test/jeans")
        .unwrap();
    assert!(jeans.draft);
    assert!(
        on.entries
            .iter()
            .filter(|e| e.item.id.starts_with("test/") && e.item.id != "test/jeans")
            .all(|e| !e.draft)
    );
    // A draft that is worn is always listed (and still marked).
    description
        .items
        .push(axcenity_avatar::WornItem::new("test/jeans"));
    let worn = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        false,
        &BTreeSet::new(),
        &[],
    );
    let jeans = worn.entries.iter().find(|e| e.item.id == "test/jeans");
    assert!(jeans.is_some_and(|e| e.draft && e.worn));
}

/// A page with nothing but drafts lists them without asking (it would
/// open on "Nothing to show" otherwise), still marked; the page offers
/// "Show drafts" only where it has finished items and drafts both.
#[test]
fn a_page_of_drafts_alone_lists_them_without_asking() {
    let mut library = (*library()).clone();
    // Only drafts in the bottom slot and on the Hair page, whatever the
    // repository ships there.
    library
        .items
        .retain(|_, item| item.is_draft() || !matches!(item.slot.as_str(), "hair" | "bottom"));
    library.add_item(
        "test/test-draft-crop.item.json",
        item(
            "test/draft-crop",
            "Draft crop",
            "groom",
            "hair",
            &[DEFAULT_BASE],
            r#", "quality": "draft", "groom": { "mesh": { "file": "x.glb" } }"#,
        ),
    );
    let description = realistic(&library);
    let list = |shelf, filters: &Filters, drafts| {
        browser::list(
            &library,
            &description,
            shelf,
            filters,
            drafts,
            &BTreeSet::new(),
            &[],
        )
    };
    // Hair: drafts only. Listed with "Show drafts" off, marked, none hidden.
    let hair = list(Shelf::Hair, &Filters::default(), false);
    assert_eq!(hair.finished, 0);
    assert!(hair.drafts >= 1);
    assert_eq!(hair.hidden_drafts, 0);
    let crop = hair
        .entries
        .iter()
        .find(|entry| entry.item.id == "test/draft-crop")
        .expect("the draft is listed");
    assert!(crop.draft, "and still marked a draft");
    assert_eq!(hair.entries.len(), hair.drafts);
    // Clothing has finished items (the fixtures' tops), so its drafts
    // wait for "Show drafts", the jeans among them.
    let clothing = list(Shelf::Clothing, &Filters::default(), false);
    assert!(clothing.finished > 0 && clothing.drafts > 0);
    assert!(clothing.hidden_drafts > 0);
    assert!(!ids(&clothing).contains(&"test/jeans".to_owned()));
    // Narrowed to a slot with drafts only, the page still has finished
    // items: the switch stays the way to its drafts.
    let bottoms = Filters {
        slot: Some("bottom".to_owned()),
        ..Filters::default()
    };
    assert!(ids(&list(Shelf::Clothing, &bottoms, false)).is_empty());
    assert_eq!(ids(&list(Shelf::Clothing, &bottoms, true)), ["test/jeans"]);
}

#[test]
fn search_matches_every_word_against_name_tags_and_about() {
    let library = library();
    let description = realistic(&library);
    let search = |text: &str| {
        let filters = Filters {
            search: text.to_owned(),
            ..Filters::default()
        };
        listed(&library, &description, Shelf::Clothing, &filters, true)
    };
    assert_eq!(search("hoodie"), ["test/hoodie"]);
    assert_eq!(search("FLEECE"), ["test/hoodie"], "the about");
    assert_eq!(search("denim"), ["test/jeans"], "a tag");
    assert_eq!(search("cotton crew"), ["test/tee"], "every word");
    assert!(search("velvet").is_empty());
}

#[test]
fn filters_narrow_by_slot_kind_colour_fit_and_favourites() {
    let library = library();
    let mut description = realistic(&library);
    let with = |filters: Filters, description: &Description| {
        listed(&library, description, Shelf::Clothing, &filters, true)
    };
    let slot = Filters {
        slot: Some("top".to_owned()),
        ..Filters::default()
    };
    assert_eq!(with(slot, &description), ["test/tee", "test/vest"]);
    let kind = Filters {
        kind: Some("cotton".to_owned()),
        ..Filters::default()
    };
    assert_eq!(with(kind, &description), ["test/tee", "test/vest"]);
    let colour = Filters {
        recolourable: true,
        ..Filters::default()
    };
    assert_eq!(with(colour, &description), ["test/tee"]);
    // The kinds offered are the listed items' tags.
    let listing = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        true,
        &BTreeSet::new(),
        &[],
    );
    // (The shipped realistic garments add their own tags.)
    for tag in ["casual", "cotton", "denim", "hood"] {
        assert!(
            listing.kinds.iter().any(|k| k == tag),
            "{:?}",
            listing.kinds
        );
    }
    let mut sorted = listing.kinds.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(listing.kinds, sorted, "listed once each, in order");
    // Fits this body: the hoodie is made for up to half the body fat.
    description.parameters.insert("fat".to_owned(), 0.8);
    assert!(!with(Filters::default(), &description).contains(&"test/hoodie".to_owned()));
    let all = Filters {
        fits: false,
        ..Filters::default()
    };
    assert!(with(all, &description).contains(&"test/hoodie".to_owned()));
    let listing = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters {
            fits: false,
            ..Filters::default()
        },
        true,
        &BTreeSet::new(),
        &[],
    );
    let hoodie = listing
        .entries
        .iter()
        .find(|e| e.item.id == "test/hoodie")
        .unwrap();
    assert_eq!(
        hoodie.misfit.as_deref(),
        Some("Made for up to half the body fat.")
    );
    // Favourites only, and favourites first on the whole list.
    let favourites = BTreeSet::from(["test/jeans".to_owned()]);
    let only = browser::list(
        &library,
        &realistic(&library),
        Shelf::Clothing,
        &Filters {
            favourites: true,
            ..Filters::default()
        },
        true,
        &favourites,
        &[],
    );
    assert_eq!(ids(&only), ["test/jeans"]);
    let first = browser::list(
        &library,
        &realistic(&library),
        Shelf::Clothing,
        &Filters::default(),
        true,
        &favourites,
        &[],
    );
    assert_eq!(ids(&first)[0], "test/jeans");
}

#[test]
fn recently_used_lists_the_items_last_worn_newest_first() {
    let library = library();
    let description = realistic(&library);
    let mut recent = Vec::new();
    for id in ["test/tee", "test/jeans", "test/hoodie", "test/tee"] {
        browser::remember(&mut recent, id);
    }
    assert_eq!(recent, ["test/tee", "test/hoodie", "test/jeans"]);
    let shown = |filters: &Filters, drafts| {
        browser::list(
            &library,
            &description,
            Shelf::Clothing,
            filters,
            drafts,
            &BTreeSet::new(),
            &recent,
        )
        .recent
        .iter()
        .map(|e| e.item.id.clone())
        .collect::<Vec<_>>()
    };
    // A hidden draft isn't in the row either.
    assert_eq!(
        shown(&Filters::default(), false),
        ["test/tee", "test/hoodie"]
    );
    assert_eq!(
        shown(&Filters::default(), true),
        ["test/tee", "test/hoodie", "test/jeans"]
    );
    // Only on the whole list: a search shows its own results.
    let searching = Filters {
        search: "tee".to_owned(),
        ..Filters::default()
    };
    assert!(shown(&searching, true).is_empty());
    for n in 0..20 {
        browser::remember(&mut recent, &format!("x/{n}"));
    }
    assert_eq!(recent.len(), browser::MAX_RECENT_ITEMS);
}

#[test]
fn wearing_takes_off_what_conflicts_and_what_its_slots_held() {
    let library = library();
    let mut description = realistic(&library);
    let worn = |d: &Description| d.items.iter().map(|w| w.id.clone()).collect::<Vec<_>>();
    description = browser::dressed(&description, &library, "outerwear", Some("test/hoodie"));
    description = browser::dressed(&description, &library, "top", Some("test/tee"));
    assert!(worn(&description).contains(&"test/hoodie".to_owned()));
    // The vest can't be worn with the hoodie: it goes, and the tee too.
    let vest = browser::dressed(&description, &library, "top", Some("test/vest"));
    let ids = worn(&vest);
    assert!(ids.contains(&"test/vest".to_owned()));
    assert!(!ids.contains(&"test/hoodie".to_owned()));
    assert!(!ids.contains(&"test/tee".to_owned()));
    // The underwear the body type wears stays.
    assert!(ids.iter().any(|id| id.contains("briefs")));
    // The vest's tile says what it takes off.
    let listing = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        true,
        &BTreeSet::new(),
        &[],
    );
    let entry = listing
        .entries
        .iter()
        .find(|e| e.item.id == "test/vest")
        .unwrap();
    assert_eq!(entry.takes_off, ["Zip hoodie"]);
    // Nothing: the slot is emptied, nothing else changes.
    let bare = browser::dressed(&description, &library, "top", None);
    assert_eq!(
        worn(&bare).len(),
        worn(&description).len() - 1,
        "{:?}",
        worn(&bare)
    );
}

#[test]
fn a_clothing_tile_shows_its_item_over_what_is_under_it_only() {
    let library = library();
    let mut description = realistic(&library);
    description = browser::dressed(&description, &library, "outerwear", Some("test/hoodie"));
    let worn = |d: &Description| d.items.iter().map(|w| w.id.clone()).collect::<Vec<_>>();
    // The tee's picture leaves the hoodie (layer 3 over its 1) off, and
    // keeps the underwear (layer 0).
    let tee = browser::dressed(&description, &library, "top", Some("test/tee"));
    let shown = worn(&browser::pictured(tee, &library, "top", Some("test/tee")));
    assert!(shown.contains(&"test/tee".to_owned()));
    assert!(!shown.contains(&"test/hoodie".to_owned()));
    assert!(shown.iter().any(|id| id.contains("briefs")));
    // The hoodie's own picture keeps what is under it.
    let shown = browser::pictured(
        description.clone(),
        &library,
        "outerwear",
        Some("test/hoodie"),
    );
    assert!(worn(&shown).contains(&"test/hoodie".to_owned()));
    // An empty top's tile: the slot's lowest layer, 1, so the hoodie goes.
    let none = browser::dressed(&description, &library, "top", None);
    let shown = worn(&browser::pictured(none, &library, "top", None));
    assert!(!shown.contains(&"test/hoodie".to_owned()));
}

#[test]
fn every_style_declares_only_what_it_supports() {
    let library = library();
    // A bob with one colour: its colour, no highlights, no roots, no
    // variants, no parameters.
    let bob = Declared::of(library.item("test/bob").unwrap());
    let roles = |list: &[(MaterialRole, Vec<String>)]| -> Vec<MaterialRole> {
        list.iter().map(|(role, _)| *role).collect()
    };
    assert_eq!(roles(&bob.natural), [MaterialRole::Primary]);
    assert_eq!(roles(&bob.roles), [MaterialRole::Primary]);
    assert!(bob.variants.is_empty() && bob.parameters.is_empty());
    // Streaked waves: highlights and roots, and a length.
    let streaked = Declared::of(library.item("test/streaked").unwrap());
    assert_eq!(
        roles(&streaked.natural),
        [
            MaterialRole::Primary,
            MaterialRole::Secondary,
            MaterialRole::Accent
        ]
    );
    // Its roots offer only melanin and redness: no tint button for them.
    assert_eq!(
        roles(&streaked.roles),
        [MaterialRole::Primary, MaterialRole::Secondary]
    );
    assert_eq!(streaked.parameters, ["length"]);
    // A garment: a colour per declared role, its variants; no hair colours.
    let tee = Declared::of(library.item("test/tee").unwrap());
    assert_eq!(
        roles(&tee.roles),
        [MaterialRole::Primary, MaterialRole::Accent]
    );
    assert!(tee.natural.is_empty());
    assert_eq!(tee.variants, ["navy"]);
    // A slot with no role is no colour control.
    assert!(Declared::of(library.item("test/hoodie").unwrap()).is_empty());
}

/// Writes `items` into the workspace's own library (beside the copy of
/// the repository's) and lists them in its catalogue.
fn add_items(folder: &Path, items: &[ItemManifest]) {
    let root = folder.join(LIBRARY_FOLDER);
    let (mut library, _, _) = Library::read(&root);
    for item in items {
        let path = format!("test-items/{}.item.json", item.id.replace('/', "-"));
        let file = root.join(&path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, item.to_json()).unwrap();
        library.add_item(path, item.clone());
    }
    std::fs::write(
        root.join("catalogue.json"),
        Catalogue::of(&library).to_json(),
    )
    .unwrap();
}

/// Items for the stylized body that use its shipped models: a bob with
/// highlights, variants and a length, and one with a single colour.
fn stylized_items() -> Vec<ItemManifest> {
    let s = &[V1_BASE];
    vec![
        item(
            "test/streaked-bob",
            "Streaked bob",
            "groom",
            "hair",
            s,
            r#", "quality": "production", "tags": ["medium"],
               "groom": { "mesh": { "file": "../humanoid/HumanoidParts.glb",
                                    "object": "AxcenityHumanoid/Hair_Bob" } },
               "fit": { "method": "shapes", "follows": ["Frame_Feminine"] },
               "materials": [{ "name": "Hair", "shading": "hair", "role": "primary" },
                             { "name": "HairSecondary", "shading": "hair", "role": "secondary",
                               "label": "Highlights" }],
               "variants": [{ "id": "sun", "label": "Sun-lightened",
                              "materials": { "HairSecondary": { "melanin": -0.8 } } }],
               "parameters": [{ "id": "length", "label": "Length", "range": [0, 1],
                                "default": 0.5 }]"#,
        ),
        item(
            "test/draft-crop",
            "Draft crop",
            "groom",
            "hair",
            s,
            r#", "quality": "draft", "tags": ["short"],
               "groom": { "mesh": { "file": "../humanoid/HumanoidParts.glb",
                                    "object": "AxcenityHumanoid/Hair_Crop" } },
               "materials": [{ "name": "Hair", "shading": "hair", "role": "primary" }]"#,
        ),
        item(
            "test/plain-crop",
            "Plain crop",
            "groom",
            "hair",
            s,
            r#", "quality": "production", "tags": ["short"],
               "groom": { "mesh": { "file": "../humanoid/HumanoidParts.glb",
                                    "object": "AxcenityHumanoid/Hair_Crop" } },
               "materials": [{ "name": "Hair", "shading": "hair", "role": "primary" }]"#,
        ),
    ]
}

/// A workspace on the stylized body with [`stylized_items`] in its library.
fn stylized_workspace(name: &str) -> (PathBuf, CharacterWorkspace) {
    let folder = test_folder(name);
    let mut workspace =
        CharacterWorkspace::open_new(&folder, &Start::Family(V1_BASE.to_owned())).unwrap();
    add_items(&folder, &stylized_items());
    assert!(workspace.library().item("test/streaked-bob").is_some());
    workspace.apply(Intent::PickCategory(Category::Hair));
    (folder, workspace)
}

fn draw(workspace: &mut CharacterWorkspace, events: Vec<egui::Event>) -> Vec<String> {
    let ctx = theme::test_context();
    let mut texts = Vec::new();
    for events in [events, Vec::new()] {
        let output = theme::run(&ctx, input(events), |ui| {
            assert_eq!(workspace.ui(ui, None), None);
        });
        texts = drawn_texts(&output);
    }
    texts
}

fn wears(description: &Description, id: &str) -> bool {
    description.items.iter().any(|worn| worn.id == id)
}

fn pick(id: &str) -> Pick {
    Pick {
        slot: "hair".to_owned(),
        item: Some(id.to_owned()),
    }
}

#[test]
fn a_hovered_tile_is_previewed_without_committing_and_a_click_wears_it() {
    let (folder, mut workspace) = stylized_workspace("browser-preview");
    let before = workspace.description().clone();
    let dirty = workspace.dirty();
    assert!(!wears(&before, "test/streaked-bob"));
    let parts = workspace.worn();
    // Hovering a tile shows it on the character only.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick(
        "test/streaked-bob",
    )))));
    assert!(wears(workspace.shown(), "test/streaked-bob"));
    assert_eq!(
        *workspace.description(),
        before,
        "the character is untouched"
    );
    assert!(!workspace.can_undo(), "no undo step");
    assert_eq!(workspace.dirty(), dirty);
    assert_ne!(workspace.worn(), parts, "the preview wears it");
    // Another tile under the pointer shows that one; off the tiles, the
    // character as it is.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert!(wears(workspace.shown(), "test/plain-crop"));
    assert!(!wears(workspace.shown(), "test/streaked-bob"));
    workspace.apply(Intent::Browse(Browse::Hover(None)));
    assert_eq!(*workspace.shown(), before);
    assert_eq!(workspace.worn(), parts);
    // Esc puts the character back too. Nothing waits to be confirmed:
    // there is no "Trying on" bar and no "Wear" button.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick(
        "test/streaked-bob",
    )))));
    let texts = draw(&mut workspace, Vec::new());
    assert!(
        !texts
            .iter()
            .any(|t| t.starts_with("Trying on") || t.contains("Wear")),
        "{texts:?}"
    );
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick(
        "test/streaked-bob",
    )))));
    draw(
        &mut workspace,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(*workspace.shown(), before);
    assert_eq!(workspace.worn(), parts);
    assert!(workspace.browser.hovered.is_none());
    // Leaving the page puts it back too.
    workspace.apply(Intent::Browse(Browse::Hover(None)));
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick(
        "test/streaked-bob",
    )))));
    assert!(workspace.browser.previewing());
    workspace.apply(Intent::PickCategory(Category::Body));
    assert_eq!(*workspace.shown(), before);
    assert!(!workspace.can_undo());
    // A click wears it (what the panel sends): one undo step, the preview
    // over, recently used. It stays on when the page is left.
    workspace.apply(Intent::PickCategory(Category::Hair));
    workspace.apply(Intent::Browse(Browse::Hover(None)));
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick(
        "test/streaked-bob",
    )))));
    workspace.apply(Intent::Browse(Browse::Clicked(pick("test/streaked-bob"))));
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("test/streaked-bob".to_owned()),
    });
    assert!(wears(workspace.description(), "test/streaked-bob"));
    assert!(!workspace.browser.previewing());
    assert_eq!(workspace.recent_items[0], "test/streaked-bob");
    workspace.apply(Intent::PickCategory(Category::Body));
    assert!(wears(workspace.shown(), "test/streaked-bob"), "still worn");
    workspace.apply(Intent::PickCategory(Category::Hair));
    workspace.apply(Intent::Undo);
    assert_eq!(*workspace.description(), before);
    assert!(!workspace.can_undo(), "exactly one step");
    // Any edit ends a preview first.
    workspace.apply(Intent::Browse(Browse::Hover(None)));
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert!(workspace.browser.previewing());
    workspace.apply(Intent::SetParameter {
        id: "height".to_owned(),
        value: 1.1,
        gesture: Gesture::Commit,
    });
    assert!(!workspace.browser.previewing());
    assert!(!wears(workspace.description(), "test/plain-crop"));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn drafts_show_only_with_show_drafts_and_say_draft() {
    let (folder, mut workspace) = stylized_workspace("browser-drafts");
    // Only an item that says `draft` is one; the shipped stylized
    // hairstyles (no `quality`, made before the standard) are listed.
    let library = workspace.library();
    assert!(library.item("test/draft-crop").unwrap().is_draft());
    assert!(!library.item("axcenity/hair-long").unwrap().is_draft());
    let texts = draw(&mut workspace, Vec::new());
    assert!(!texts.iter().any(|t| t == "Draft crop"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Long"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Streaked bob"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Show drafts (") && t.ends_with(" hidden)")),
        "{texts:?}"
    );
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    let texts = draw(&mut workspace, Vec::new());
    assert!(texts.iter().any(|t| t == "Draft crop"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Draft"), "{texts:?}");
    // Turning them off again drops a draft being previewed.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/draft-crop")))));
    assert!(workspace.browser.previewing());
    workspace.apply(Intent::Browse(Browse::ShowDrafts(false)));
    assert!(!workspace.browser.previewing());
    assert!(!workspace.can_undo());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_worn_style_shows_only_the_controls_it_declares() {
    let (folder, mut workspace) = stylized_workspace("browser-controls");
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("test/plain-crop".to_owned()),
    });
    // Section labels are drawn in capitals.
    let has = |texts: &[String], text: &str| texts.iter().any(|t| t.eq_ignore_ascii_case(text));
    // Worn, it is one line above the tiles: its controls are folded away.
    let texts = draw(&mut workspace, Vec::new());
    assert!(has(&texts, "Wearing"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t.starts_with("Plain crop")),
        "{texts:?}"
    );
    assert!(!has(&texts, "Hair colour"), "folded: {texts:?}");
    let unfold = |id: &str| Intent::Browse(Browse::Unfold(id.to_owned(), true));
    let worn = workspace.description().clone();
    workspace.apply(unfold("test/plain-crop"));
    assert_eq!(*workspace.description(), worn, "a fold is editor state");
    let texts = draw(&mut workspace, Vec::new());
    assert!(has(&texts, "Hair colour"), "{texts:?}");
    for absent in ["Highlights", "Roots", "Length", "Variant"] {
        assert!(!has(&texts, absent), "{absent}: {texts:?}");
    }
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("test/streaked-bob".to_owned()),
    });
    workspace.apply(unfold("test/streaked-bob"));
    let texts = draw(&mut workspace, Vec::new());
    for present in [
        "Hair colour",
        "Highlights",
        "Length",
        "Variant",
        "Sun-lightened",
    ] {
        assert!(has(&texts, present), "{present}: {texts:?}");
    }
    assert!(!has(&texts, "Roots"), "{texts:?}");
    // Nothing drawn twice under one ID (egui warns in the picture).
    assert!(!texts.iter().any(|t| t.contains("widget ID")), "{texts:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_worn_items_controls_are_one_undo_step_each() {
    let (folder, mut workspace) = stylized_workspace("browser-worn");
    let id = "test/streaked-bob".to_owned();
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some(id.clone()),
    });
    let worn = |workspace: &CharacterWorkspace| {
        workspace
            .description()
            .items
            .iter()
            .find(|w| w.id == "test/streaked-bob")
            .cloned()
            .unwrap()
    };
    let steps = |workspace: &mut CharacterWorkspace| {
        let mut n = 0;
        while workspace.can_undo() {
            workspace.apply(Intent::Undo);
            n += 1;
        }
        for _ in 0..n {
            workspace.apply(Intent::Redo);
        }
        n
    };
    assert_eq!(steps(&mut workspace), 1);
    // A variant.
    workspace.apply(Intent::SetItemVariant {
        item: id.clone(),
        variant: Some("sun".to_owned()),
    });
    assert_eq!(worn(&workspace).variant.as_deref(), Some("sun"));
    assert_eq!(steps(&mut workspace), 2);
    // One the item doesn't declare changes nothing.
    workspace.apply(Intent::SetItemVariant {
        item: id.clone(),
        variant: Some("neon".to_owned()),
    });
    assert_eq!(steps(&mut workspace), 2);
    // A natural colour: melanin and redness on both slots, one step.
    workspace.apply(Intent::SetItemMaterial {
        item: id.clone(),
        slots: vec!["Hair".to_owned(), "HairSecondary".to_owned()],
        values: vec![
            ("melanin".to_owned(), Some(MaterialValue::Number(1.0))),
            ("redness".to_owned(), Some(MaterialValue::Number(0.2))),
        ],
        gesture: Gesture::Commit,
    });
    let held = worn(&workspace);
    for slot in ["Hair", "HairSecondary"] {
        assert_eq!(
            held.materials[slot]["melanin"],
            MaterialValue::Number(1.0),
            "{slot}"
        );
        assert_eq!(held.materials[slot]["redness"], MaterialValue::Number(0.2));
    }
    assert_eq!(steps(&mut workspace), 3);
    // A dragged length: one step when it's let go, held in its range.
    for (value, gesture) in [
        (0.6, Gesture::Drag),
        (0.8, Gesture::Drag),
        (1.4, Gesture::Commit),
    ] {
        workspace.apply(Intent::SetItemParameter {
            item: id.clone(),
            id: "length".to_owned(),
            value,
            gesture,
        });
    }
    assert_eq!(worn(&workspace).parameters["length"], 1.0);
    assert_eq!(steps(&mut workspace), 4);
    // Picking the variant again takes back what it sets (HairSecondary).
    workspace.apply(Intent::SetItemVariant {
        item: id.clone(),
        variant: None,
    });
    workspace.apply(Intent::SetItemVariant {
        item: id.clone(),
        variant: Some("sun".to_owned()),
    });
    let held = worn(&workspace);
    assert!(!held.materials.contains_key("HairSecondary"));
    assert!(held.materials.contains_key("Hair"));
    // The description is sound against the library.
    let problems = workspace.description().validate(&workspace.library());
    assert!(problems.is_empty(), "{problems:?}");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn favourites_and_recently_used_are_kept_beside_the_preset_favourites() {
    let (folder, mut workspace) = stylized_workspace("browser-favourites");
    let before = workspace.description().clone();
    let library = workspace.library();
    let preset = workspace
        .presets_of(&library, axcenity_avatar::PresetKind::Character)
        .first()
        .map(|listed| listed.id.to_owned());
    if let Some(preset) = &preset {
        workspace.apply(Intent::ToggleFavourite(preset.clone()));
    }
    workspace.apply(Intent::ToggleItemFavourite("test/plain-crop".to_owned()));
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("test/streaked-bob".to_owned()),
    });
    workspace.apply(Intent::Undo);
    // Editor state: the character and its undo are untouched.
    assert_eq!(*workspace.description(), before);
    assert!(!workspace.can_undo());
    // Favourites first on the page; the last worn in the row.
    let listing = browser::list(
        &library,
        workspace.description(),
        Shelf::Hair,
        &Filters::default(),
        false,
        &workspace.item_favourites,
        &workspace.recent_items,
    );
    assert_eq!(listing.entries[0].item.id, "test/plain-crop");
    assert_eq!(listing.recent[0].item.id, "test/streaked-bob");
    // Kept in the favourites file, beside the preset favourites.
    drop(workspace);
    let workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    assert!(workspace.item_favourites.contains("test/plain-crop"));
    assert_eq!(workspace.recent_items, ["test/streaked-bob"]);
    if let Some(preset) = &preset {
        assert!(workspace.favourites.contains(preset));
    }
    let text = std::fs::read_to_string(folder.join(super::presets::FAVOURITES_FILE)).unwrap();
    assert!(text.contains("\"items\"") && text.contains("\"recent_items\""));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_realistic_pages_say_what_they_have() {
    let folder = test_folder("browser-realistic");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    for (category, shelf) in [
        (Category::Hair, Shelf::Hair),
        (Category::Clothing, Shelf::Clothing),
    ] {
        workspace.apply(Intent::PickCategory(category));
        let library = workspace.library();
        let listing = browser::list(
            &library,
            workspace.description(),
            shelf,
            &Filters::default(),
            false,
            &BTreeSet::new(),
            &[],
        );
        let texts = draw(&mut workspace, Vec::new());
        // A page never opens on "Nothing to show" while it has anything:
        // with only drafts to list (the realistic hairstyles and garments
        // so far), they are listed, marked "Draft", and there is no "Show
        // drafts" to turn on first.
        if listing.finished == 0 && listing.drafts > 0 {
            assert!(!listing.entries.is_empty(), "{category:?}");
            assert!(
                texts.iter().any(|t| t == "Draft"),
                "{category:?}: {texts:?}"
            );
            assert!(
                !texts.iter().any(|t| t.starts_with("Show drafts")),
                "{category:?}: {texts:?}"
            );
            assert!(
                !texts.iter().any(|t| t == "Nothing to show"),
                "{category:?}: {texts:?}"
            );
        }
        // Nothing waits to be confirmed on either page.
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("Wear") && !t.eq_ignore_ascii_case("Wearing")),
            "{category:?}: {texts:?}"
        );
    }
    // The underwear is said in one line, under the tiles.
    let texts = draw(&mut workspace, Vec::new());
    assert!(
        texts
            .iter()
            .any(|t| t.contains("which comes with the body type")),
        "{texts:?}"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// With `AXCENITY_SNAPSHOTS` naming a folder: both pages drawn for real at
/// 1440 by 900, the tiles' pictures drawn first. The realistic Clothing
/// page has the CO9 test tee imported into the workspace's library (a
/// draft). Saved as `character-browser-*.png`.
#[test]
fn the_browsers_are_drawn() {
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let (folder, mut workspace) = stylized_workspace("browser-snapshots");
    workspace.selection.level = super::selection::Level::Detailed;
    let shots: Vec<(&str, Category, Vec<Intent>)> = vec![
        ("hair-drafts-off", Category::Hair, vec![]),
        (
            "hair-drafts-on",
            Category::Hair,
            vec![Intent::Browse(Browse::ShowDrafts(true))],
        ),
        (
            "hair-preview",
            Category::Hair,
            vec![
                Intent::Wear {
                    slot: "hair".to_owned(),
                    item: Some("test/streaked-bob".to_owned()),
                },
                Intent::Browse(Browse::Unfold("test/streaked-bob".to_owned(), true)),
                Intent::Browse(Browse::Hover(Some(pick("axcenity/hair-long")))),
            ],
        ),
        (
            "hair-search",
            Category::Hair,
            vec![
                Intent::EndPreview,
                Intent::Browse(Browse::Filters(
                    Shelf::Hair,
                    Filters {
                        search: "short".to_owned(),
                        ..Filters::default()
                    },
                )),
            ],
        ),
        (
            "clothing",
            Category::Clothing,
            vec![Intent::Wear {
                slot: "top".to_owned(),
                item: Some("axcenity/tee".to_owned()),
            }],
        ),
        (
            "clothing-filtered",
            Category::Clothing,
            vec![Intent::Browse(Browse::Filters(
                Shelf::Clothing,
                Filters {
                    slot: Some("bottom".to_owned()),
                    ..Filters::default()
                },
            ))],
        ),
    ];
    for (name, category, intents) in shots {
        workspace.apply(Intent::PickCategory(category));
        for intent in intents {
            workspace.apply(intent);
        }
        snapshot(&snapshots, name, &folder, &mut workspace);
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// With `AXCENITY_SNAPSHOTS`: the realistic Clothing page with the CO9
/// test tee imported into the workspace's library (a draft), drafts off
/// and the tee tried on. Ignored: the import takes minutes in a debug
/// build, past nextest's two; run it with `cargo test -- --ignored`.
#[test]
#[ignore = "imports the CO9 test tee: minutes in a debug build"]
fn the_realistic_clothing_page_is_drawn_with_the_test_tee() {
    let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) else {
        return;
    };
    let folder = test_folder("browser-snapshots-realistic");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let tee =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../avatar/tests/fixtures/test-tee/TestTee.obj");
    let imported = axcenity_avatar::fit::import_garment(&axcenity_avatar::fit::ImportRequest {
        source: tee,
        library: folder.join(LIBRARY_FOLDER),
        base: DEFAULT_BASE.to_owned(),
        id: "imported/test-tee".to_owned(),
        label: "Test tee".to_owned(),
        folder: "imported/test-tee".to_owned(),
        source_info: axcenity_avatar::Source {
            author: "Axcenity Engine contributors".to_owned(),
            licence: "CC0-1.0".to_owned(),
            url: None,
        },
        slot: None,
        layer: None,
        margin: 0.002,
        under: vec!["axcenity/realistic-sports-bra".to_owned()],
        about: Some("The fitting test garment (CO9): a draft.".to_owned()),
        finish_pose: None,
        also_fills: Vec::new(),
        tags: Vec::new(),
        limitations: Vec::new(),
        intake: None,
        texture_size: None,
        worn_with: None,
    });
    assert!(imported.is_ok(), "{:?}", imported.err());
    workspace.apply(Intent::PickCategory(Category::Clothing));
    snapshot(
        &snapshots,
        "realistic-clothing-drafts-off",
        &folder,
        &mut workspace,
    );
    workspace.apply(Intent::Browse(Browse::Hover(Some(Pick {
        slot: "top".to_owned(),
        item: Some("imported/test-tee".to_owned()),
    }))));
    snapshot(
        &snapshots,
        "realistic-clothing-preview",
        &folder,
        &mut workspace,
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Draws the tiles' pictures (on a GPU of its own), then the workspace.
fn snapshot(snapshots: &Path, name: &str, folder: &Path, workspace: &mut CharacterWorkspace) {
    let library = workspace.library();
    if let Some(shelf) = Shelf::of(workspace.selection.category) {
        let instance = axcenity_render::adapters::instance();
        if let Ok(adapter) = pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
        {
            let (device, queue) =
                pollster::block_on(adapter.request_device(&Default::default())).unwrap();
            let mut painter = super::thumbnails::Painter::new(folder, &device, &queue).unwrap();
            for (id, subject) in workspace.browser_tiles(&library, shelf) {
                let out = super::thumbnails::item_picture(folder, &id, &subject);
                if !out.is_file() {
                    let started = std::time::Instant::now();
                    painter
                        .paint_to(&super::thumbnails::Job { subject, out })
                        .unwrap();
                    eprintln!("tile picture {id}: {:?}", started.elapsed());
                }
            }
        }
    }
    workspace.canvas = Default::default();
    // Each snapshot has an egui context of its own: textures loaded in
    // another are no good here.
    workspace.item_pictures.clear();
    workspace.pictures = super::thumbnails::Thumbnails::default();
    workspace.pictures.set_folder(Some(folder.to_owned()));
    crate::snapshot::snapshot_rendered(
        &snapshots.join(format!("character-browser-{name}.png")),
        egui::vec2(1440.0, 900.0),
        vec![Vec::new(); 6],
        |ui, render_state| {
            workspace.ui(ui, Some(render_state));
        },
    );
}

#[test]
fn keeping_a_held_version_ends_a_tile_preview() {
    // Review fix 1: Keep A replaces the description, so a tile tried on
    // the character before it must not stay in the viewport.
    let (folder, mut workspace) = stylized_workspace("browser-keep-held");
    workspace.apply(Intent::Capture);
    workspace.apply(Intent::SetParameter {
        id: "height".to_owned(),
        value: 1.1,
        gesture: Gesture::Commit,
    });
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert!(workspace.browser.previewing());
    workspace.apply(Intent::ShowHeld(Some(0)));
    workspace.apply(Intent::KeepHeld(0));
    assert!(!workspace.browser.previewing(), "the try-on ended");
    assert_eq!(
        *workspace.shown(),
        *workspace.description(),
        "what is edited is what is seen"
    );
    // Undo and redo change the character too: a preview gives way.
    workspace.apply(Intent::Browse(Browse::Hover(None)));
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert!(workspace.browser.previewing());
    workspace.apply(Intent::Undo);
    assert!(!workspace.browser.previewing());
    assert_eq!(*workspace.shown(), *workspace.description());
    let _ = std::fs::remove_dir_all(&folder);
}

/// Where text `text` is drawn in `output` (its first appearance).
fn text_at(output: &egui::FullOutput, text: &str) -> Option<egui::Pos2> {
    fn walk(shape: &egui::Shape, text: &str) -> Option<egui::Pos2> {
        match shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.visual_bounding_rect().center())
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| walk(s, text)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|clipped| walk(&clipped.shape, text))
}

#[test]
fn esc_ends_a_hover_preview_while_the_pointer_stays_on_its_tile() {
    // Review fix 1: the pointer is still on the tile after Esc; the
    // preview must not come back with the panel hovering that tile again
    // next frame.
    let (folder, mut workspace) = stylized_workspace("browser-esc-hover");
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    frame_in(&ctx, &mut workspace, Vec::new());
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let tile = text_at(&output, "Plain crop").expect("the Plain crop tile is drawn");
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(tile)]);
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(tile)]);
    assert!(wears(workspace.shown(), "test/plain-crop"), "hovered");
    assert_eq!(*workspace.description(), before, "a preview only");
    let escape = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame_in(&ctx, &mut workspace, vec![escape]);
    // The pointer hasn't moved: the preview stays off.
    for _ in 0..3 {
        frame_in(&ctx, &mut workspace, Vec::new());
    }
    assert_eq!(*workspace.shown(), before, "Esc took the preview off");
    assert!(!workspace.browser.previewing());
    // The same Hover again (what the panel would send) changes nothing.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert_eq!(*workspace.shown(), before);
    // Off the tile, the character as it is; back on it, previewed again.
    frame_in(
        &ctx,
        &mut workspace,
        vec![egui::Event::PointerMoved(egui::pos2(5.0, 5.0))],
    );
    frame_in(&ctx, &mut workspace, Vec::new());
    assert_eq!(*workspace.shown(), before);
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(tile)]);
    frame_in(&ctx, &mut workspace, Vec::new());
    assert!(
        wears(workspace.shown(), "test/plain-crop"),
        "hovering the tile again previews it"
    );
    // Leaving the tiles ends it: nothing was ever worn.
    frame_in(
        &ctx,
        &mut workspace,
        vec![egui::Event::PointerMoved(egui::pos2(5.0, 5.0))],
    );
    frame_in(&ctx, &mut workspace, Vec::new());
    assert_eq!(*workspace.shown(), before);
    assert!(!workspace.can_undo());
    let _ = std::fs::remove_dir_all(&folder);
}

/// One frame of `workspace` drawn in `ctx` with `events`.
fn frame_in(
    ctx: &egui::Context,
    workspace: &mut CharacterWorkspace,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    theme::run(ctx, input(events), |ui| {
        assert_eq!(workspace.ui(ui, None), None);
    })
}

/// A press (or release) of the main button at `at`.
fn button_at(at: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// Moves to `at`, then clicks there, then draws three frames with the
/// pointer still: what a person does when they click a tile.
fn click_and_hold_still(ctx: &egui::Context, workspace: &mut CharacterWorkspace, at: egui::Pos2) {
    frame_in(ctx, workspace, vec![egui::Event::PointerMoved(at)]);
    frame_in(ctx, workspace, vec![egui::Event::PointerMoved(at)]);
    frame_in(ctx, workspace, vec![button_at(at, true)]);
    frame_in(ctx, workspace, vec![button_at(at, false)]);
    for _ in 0..3 {
        frame_in(ctx, workspace, Vec::new());
    }
}

/// The hairstyle `description` wears (its `hair` slot).
fn hair_of(workspace: &CharacterWorkspace, description: &Description) -> Option<String> {
    let library = workspace.library();
    description
        .items
        .iter()
        .find(|worn| {
            library
                .item(&worn.id)
                .is_some_and(|item| item.slot == "hair")
        })
        .map(|worn| worn.id.clone())
}

/// How many undo steps `workspace` has.
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

#[test]
fn clicking_a_tile_wears_it_and_the_still_pointer_previews_nothing_else() {
    // Paul, 2026-11-24: a click wears (it used to pick a tile to try on,
    // which any edit or page change then took off again). Review fix 2
    // still holds: the worn item's line pushes the tiles down under the
    // still pointer, and the tile that comes under it must not be
    // previewed in the clicked one's place.
    let (folder, mut workspace) = stylized_workspace("browser-click-top");
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    frame_in(&ctx, &mut workspace, Vec::new());
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let long = text_at(&output, "Long").expect("the Long tile is drawn");
    let bob = text_at(&output, "Bob").expect("the Bob tile is drawn");
    assert!(long.y > bob.y + 50.0, "Long is in a lower row than Bob");
    // A point near the top of Long's tile: a row's pitch above its label
    // is the label of the row above; the tile starts about 20 points lower.
    let at = egui::pos2(long.x, long.y - (long.y - bob.y) + 35.0);
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(at)]);
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(at)]);
    assert_eq!(
        hair_of(&workspace, workspace.shown()).as_deref(),
        Some("axcenity/hair-long"),
        "the point is on the Long tile: hovered, it is previewed"
    );
    assert_eq!(*workspace.description(), before, "and not worn yet");
    click_and_hold_still(&ctx, &mut workspace, at);
    assert_eq!(
        hair_of(&workspace, workspace.description()).as_deref(),
        Some("axcenity/hair-long"),
        "the click wore Long"
    );
    assert_eq!(undo_depth(&workspace), 1, "as one undo step");
    assert_eq!(
        workspace.shown(),
        workspace.description(),
        "nothing else is previewed (hovered: {:?})",
        workspace.browser.hovered
    );
    // It stays on through an edit and on another page: it is worn.
    workspace.apply(Intent::PickCategory(Category::Body));
    workspace.apply(Intent::SetParameter {
        id: "height".to_owned(),
        value: 1.1,
        gesture: Gesture::Commit,
    });
    assert_eq!(
        hair_of(&workspace, workspace.shown()).as_deref(),
        Some("axcenity/hair-long")
    );
    workspace.apply(Intent::Undo);
    workspace.apply(Intent::Undo);
    assert_eq!(*workspace.description(), before);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn clicking_the_worn_tile_takes_it_off_and_previews_nothing() {
    // A click on what is worn takes it off (Paul, 2026-11-24: a worn item
    // couldn't be clicked off). The tiles move up under the still pointer
    // when its line goes: nothing may be previewed, least of all the tile
    // just taken off.
    let (folder, mut workspace) = stylized_workspace("browser-click-worn");
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("axcenity/hair-bob".to_owned()),
    });
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    frame_in(&ctx, &mut workspace, Vec::new());
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let bob = text_at(&output, "Bob").expect("the worn Bob tile is drawn");
    click_and_hold_still(&ctx, &mut workspace, bob);
    assert_eq!(
        hair_of(&workspace, workspace.description()),
        None,
        "the worn style came off"
    );
    assert_eq!(
        workspace.shown(),
        workspace.description(),
        "nothing is previewed (hovered: {:?})",
        workspace.browser.hovered
    );
    // One undo step puts it back on.
    workspace.apply(Intent::Undo);
    assert_eq!(*workspace.description(), before);

    // Its line above the tiles takes it off too (the ✕ at its end), as
    // one undo step; the tiles that move up under the still pointer are
    // not previewed.
    frame_in(
        &ctx,
        &mut workspace,
        vec![egui::Event::PointerMoved(egui::pos2(600.0, 400.0))],
    );
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let line =
        text_where(&output, |t| t.starts_with("Bob") && t != "Bob").expect("the worn Bob's line");
    let take_off = egui::pos2(1440.0 - 12.0 - 3.0 - 12.0, line.y);
    click_and_hold_still(&ctx, &mut workspace, take_off);
    assert_eq!(hair_of(&workspace, workspace.description()), None);
    assert_eq!(
        workspace.shown(),
        workspace.description(),
        "nothing is previewed (hovered: {:?})",
        workspace.browser.hovered
    );
    assert_eq!(undo_depth(&workspace), 2, "worn, then taken off");
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_second_click_on_a_tile_takes_it_off_again() {
    let (folder, mut workspace) = stylized_workspace("browser-click-again");
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    frame_in(&ctx, &mut workspace, Vec::new());
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let long = text_at(&output, "Long").expect("the Long tile is drawn");
    click_and_hold_still(&ctx, &mut workspace, long);
    assert_eq!(
        hair_of(&workspace, workspace.description()).as_deref(),
        Some("axcenity/hair-long")
    );
    // Its line above the tiles says it is worn, with the way to take it
    // off there too.
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let texts = drawn_texts(&output);
    assert!(texts.iter().any(|t| t == "WEARING"), "{texts:?}");
    // A second click, well after the first, takes it off again. (Half a
    // second of frames first: the test's clock moves a sixtieth of a
    // second a frame. The tile moved down a line when it was worn.)
    for _ in 0..30 {
        frame_in(&ctx, &mut workspace, Vec::new());
    }
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let long = text_at(&output, "Long").expect("the Long tile is still drawn");
    click_and_hold_still(&ctx, &mut workspace, long);
    assert_eq!(hair_of(&workspace, workspace.description()), None);
    assert_eq!(workspace.shown(), workspace.description());
    assert_eq!(undo_depth(&workspace), 2, "a step on, a step off");
    workspace.apply(Intent::Undo);
    workspace.apply(Intent::Undo);
    assert_eq!(*workspace.description(), before);
    let _ = std::fs::remove_dir_all(&folder);
}

/// Where the first text that `wanted` picks is drawn in `output`.
fn text_where(output: &egui::FullOutput, wanted: impl Fn(&str) -> bool) -> Option<egui::Pos2> {
    fn walk(shape: &egui::Shape, wanted: &dyn Fn(&str) -> bool) -> Option<egui::Pos2> {
        match shape {
            egui::Shape::Text(shape) if wanted(shape.galley.text()) => {
                Some(shape.visual_bounding_rect().center())
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| walk(s, wanted)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|clipped| walk(&clipped.shape, &wanted))
}

/// The filters are one menu beside the search field (they were three rows
/// of chips): "Fits this body", "Recolourable", favourites and the kind.
/// The menu stays open while they are picked, and its button says how many
/// differ from what the page opens with.
#[test]
fn the_filters_are_a_menu_whose_button_counts_what_is_changed() {
    let (folder, mut workspace) = stylized_workspace("browser-filters-menu");
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    frame_in(&ctx, &mut workspace, Vec::new());
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let texts = drawn_texts(&output);
    // Folded away: none of them is a row of the page.
    for text in ["Recolourable", "Fits this body", "Any kind"] {
        assert!(!texts.iter().any(|t| t == text), "{text}: {texts:?}");
    }
    let button = text_where(&output, |t| t.ends_with("Filters")).expect("the Filters button");
    click_and_hold_still(&ctx, &mut workspace, button);
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let texts = drawn_texts(&output);
    for text in ["Recolourable", "Fits this body", "Any kind"] {
        assert!(texts.iter().any(|t| t == text), "{text}: {texts:?}");
    }
    // One picked: the list is narrowed, the menu is still there, and the
    // button counts it.
    let recolourable = text_where(&output, |t| t == "Recolourable").unwrap();
    click_and_hold_still(&ctx, &mut workspace, recolourable);
    assert!(workspace.browser.hair.recolourable);
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let texts = drawn_texts(&output);
    assert!(texts.iter().any(|t| t == "Fits this body"), "still open");
    assert!(
        texts.iter().any(|t| t.ends_with("Filters · 1")),
        "{texts:?}"
    );
    // "Fits this body" is on as the page opens: off, it is a change too.
    let fits = text_where(&output, |t| t == "Fits this body").unwrap();
    click_and_hold_still(&ctx, &mut workspace, fits);
    assert!(!workspace.browser.hair.fits);
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    assert!(
        drawn_texts(&output)
            .iter()
            .any(|t| t.ends_with("Filters · 2"))
    );
    // "Reset filters" puts them back; the search and the category aren't
    // the menu's.
    workspace.apply(Intent::Browse(Browse::Filters(
        Shelf::Hair,
        Filters {
            search: "crop".to_owned(),
            ..workspace.browser.hair.clone()
        },
    )));
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let reset = text_where(&output, |t| t == "Reset filters").unwrap();
    click_and_hold_still(&ctx, &mut workspace, reset);
    assert_eq!(
        workspace.browser.hair,
        Filters {
            search: "crop".to_owned(),
            ..Filters::default()
        }
    );
    // A click elsewhere closes it.
    click_and_hold_still(&ctx, &mut workspace, egui::pos2(600.0, 400.0));
    let texts = drawn_texts(&frame_in(&ctx, &mut workspace, Vec::new()));
    assert!(!texts.iter().any(|t| t == "Recolourable"), "{texts:?}");
    assert!(texts.iter().any(|t| t.ends_with("Filters")), "{texts:?}");
    // Editor state: none of it changed the character.
    assert_eq!(*workspace.description(), before);
    assert!(!workspace.can_undo());
    let _ = std::fs::remove_dir_all(&folder);
}

/// The MeshPart the preview's body wears for item `item` (the applier
/// marks it with the item's ID).
fn part_of(workspace: &CharacterWorkspace, item: &str) -> Option<axcenity_core::InstanceId> {
    use axcenity_gameplay::description as applier;
    let body = applier::body_of(&workspace.world, workspace.character?)?;
    workspace
        .world
        .get(body)?
        .children()
        .iter()
        .copied()
        .find(|child| {
            workspace
                .world
                .get(*child)
                .and_then(|part| part.attribute(applier::DESCRIPTION_PART))
                .and_then(axcenity_core::Value::as_str)
                == Some(item)
        })
}

/// What holds of a worn garment at every moment of a change to the body:
/// the description wears it and so does what is shown; its MeshPart is the
/// one it was put on as (never taken off and put back), a child of the
/// body, drawing the same model; it wears the body's own identity, so it
/// is shaped as the body is. Returns the identity key it wears.
#[track_caller]
fn garment_is_worn(
    workspace: &CharacterWorkspace,
    item: &str,
    part: axcenity_core::InstanceId,
    mesh: &str,
    when: &str,
) -> u64 {
    use axcenity_gameplay::description as applier;
    assert!(wears(workspace.description(), item), "{when}: worn");
    assert!(wears(workspace.shown(), item), "{when}: shown");
    assert_eq!(part_of(workspace, item), Some(part), "{when}: its part");
    let body = applier::body_of(&workspace.world, workspace.character.unwrap()).unwrap();
    let held = workspace.world.get(part).expect("the part is in the world");
    assert_eq!(held.parent(), Some(body), "{when}: on the body");
    assert_eq!(
        held.property("MeshId")
            .and_then(axcenity_core::Value::as_str),
        Some(mesh),
        "{when}: its model"
    );
    let key = axcenity_anim::identity_key(&workspace.world, part);
    assert!(key.is_some(), "{when}: it wears an identity");
    assert_eq!(
        key,
        axcenity_anim::identity_key(&workspace.world, body),
        "{when}: the body's own"
    );
    key.unwrap()
}

/// Paul, 2026-11-24: "when adjusting character proportions, the clothing
/// disappears completely rather than dynamically sizing". What went was a
/// tile picked to try on, never worn (any edit ended it). A garment that
/// is worn stays worn while a body slider is dragged, while a handle on
/// the character is dragged, and after each: the same part on the body,
/// wearing each new identity as the body does.
#[test]
fn a_worn_garment_stays_on_while_the_body_is_dragged() {
    use super::numeric;
    use super::stage::ViewPreset;
    let folder = test_folder("browser-worn-drag");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    // A garment of the realistic library, whichever it ships first (they
    // are drafts so far, listed all the same).
    let (slot, item) = {
        let library = workspace.library();
        let listing = browser::list(
            &library,
            workspace.description(),
            Shelf::Clothing,
            &Filters::default(),
            false,
            &BTreeSet::new(),
            &[],
        );
        let entry = listing
            .entries
            .first()
            .expect("the realistic library has a garment to wear");
        (entry.item.slot.clone(), entry.item.id.clone())
    };
    // Worn as the Clothing page wears it: a click on its tile.
    workspace.apply(Intent::PickCategory(Category::Clothing));
    workspace.apply(Intent::Browse(Browse::Hover(Some(Pick {
        slot: slot.clone(),
        item: Some(item.clone()),
    }))));
    workspace.apply(Intent::Browse(Browse::Clicked(Pick {
        slot: slot.clone(),
        item: Some(item.clone()),
    })));
    workspace.apply(Intent::Wear {
        slot: slot.clone(),
        item: Some(item.clone()),
    });
    assert!(workspace.notice.is_none(), "{:?}", workspace.notice);
    let part = part_of(&workspace, &item).expect("the garment is on the body");
    let mesh = workspace
        .world
        .get(part)
        .and_then(|part| part.property("MeshId"))
        .and_then(axcenity_core::Value::as_str)
        .expect("it draws a model")
        .to_owned();
    assert!(!mesh.is_empty());
    let mut keys = vec![garment_is_worn(&workspace, &item, part, &mesh, "worn")];

    // The body's page, the waist picked: its slider, driven by the
    // pointer as a person drags it.
    workspace.apply(Intent::PickCategory(Category::Body));
    workspace.apply(Intent::PickFeature(Some("waist".to_owned())));
    workspace.apply(Intent::FocusView(ViewPreset::Front));
    let ctx = theme::test_context();
    for _ in 0..3 {
        frame_in(&ctx, &mut workspace, Vec::new());
    }
    keys.push(garment_is_worn(&workspace, &item, part, &mesh, "on Body"));
    let from = numeric::track_rect(&ctx, "waist_width")
        .expect("the waist's row is drawn")
        .center();
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(from)]);
    frame_in(&ctx, &mut workspace, vec![button_at(from, true)]);
    // (A little: at its widest the waist's handles are behind the arms.)
    for step in 1..=4 {
        let at = from + egui::vec2(2.0 + 6.0 * step as f32, 0.0);
        frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(at)]);
        assert!(workspace.dragging(), "step {step}: the drag is going on");
        let key = garment_is_worn(&workspace, &item, part, &mesh, "during the slider drag");
        assert_ne!(Some(&key), keys.last(), "step {step}: a new identity");
        keys.push(key);
    }
    let end = from + egui::vec2(26.0, 0.0);
    frame_in(&ctx, &mut workspace, vec![button_at(end, false)]);
    for _ in 0..3 {
        frame_in(&ctx, &mut workspace, Vec::new());
    }
    assert!(!workspace.dragging());
    assert!(workspace.parameter("waist_width") > 1.05, "the waist moved");
    garment_is_worn(&workspace, &item, part, &mesh, "after the slider drag");
    assert_eq!(undo_depth(&workspace), 2, "the garment, then the waist");

    // A handle on the character, dragged the same way: 24 points against
    // the direction that widens what it drives (well past the handle's
    // dead zone).
    let from = workspace
        .handle_on_screen("waist_left")
        .expect("the waist's handle is shown");
    let (driven, screen) = workspace
        .handle_drives_on_screen("waist_left")
        .first()
        .cloned()
        .expect("the handle drives a parameter");
    let along = screen.normalize() * -24.0;
    let by = egui::vec2(along.x, along.y);
    let before = workspace.parameter(&driven);
    let key = garment_is_worn(&workspace, &item, part, &mesh, "before the handle drag");
    frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(from)]);
    frame_in(&ctx, &mut workspace, vec![button_at(from, true)]);
    for step in 1..=4 {
        let at = from + by * (step as f32 / 4.0);
        frame_in(&ctx, &mut workspace, vec![egui::Event::PointerMoved(at)]);
        garment_is_worn(&workspace, &item, part, &mesh, "during the handle drag");
    }
    frame_in(&ctx, &mut workspace, vec![button_at(from + by, false)]);
    for _ in 0..3 {
        frame_in(&ctx, &mut workspace, Vec::new());
    }
    assert!(
        workspace.parameter(&driven) < before - 0.02,
        "the handle moved {driven}: {} from {before}",
        workspace.parameter(&driven)
    );
    let after = garment_is_worn(&workspace, &item, part, &mesh, "after the handle drag");
    assert_ne!(after, key, "and the garment wears the new shape");
    // Worn, that identity reshapes the garment's own model (it is sized
    // with the body, not left as made), and the garment is drawn: one
    // more thing than the character without it.
    let model = workspace
        .assets
        .model_blocking(&mesh)
        .expect("the garment's model loads");
    let identity = workspace
        .assets
        .identity(after)
        .expect("the identity it wears is registered");
    assert!(
        !model.live(&identity).is_empty(),
        "the garment follows the body's shape"
    );
    let drawn = |workspace: &CharacterWorkspace| {
        axcenity_scene::build_draw_list_posed(
            &workspace.world,
            &workspace.assets,
            workspace.orbit.camera(),
            workspace.poses(),
            &mut axcenity_scene::TerrainCache::new(),
        )
        .items
        .len()
    };
    let dressed = drawn(&workspace);

    // Undone, the body is as it was and the garment still on; only the
    // last undo, of the click that wore it, takes it off.
    while undo_depth(&workspace) > 1 {
        workspace.apply(Intent::Undo);
        garment_is_worn(&workspace, &item, part, &mesh, "after an undo");
    }
    assert_eq!(drawn(&workspace), dressed, "drawn with the body as it was");
    workspace.apply(Intent::Undo);
    assert!(!wears(workspace.description(), &item));
    assert_eq!(part_of(&workspace, &item), None);
    assert!(
        drawn(&workspace) < dressed,
        "taken off, it is drawn no more ({dressed} things were)"
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Paul, 2026-11-24: clothes weren't separated by who they are cut for.
/// The Clothing page lists the garments for the character's own frame
/// (and the unisex ones), or one frame's, or all.
#[test]
fn the_clothing_page_lists_the_garments_cut_for_the_characters_frame() {
    let mut library = (*library()).clone();
    let r = &[DEFAULT_BASE];
    for item in [
        item(
            "test/gown",
            "Gown",
            "garment",
            "top",
            r,
            r#", "quality": "production", "tags": ["feminine", "formal"],
               "limitations": [{ "note": "Cut for the feminine frame",
                                 "parameter": "frame", "range": [0.5, 1] }]"#,
        ),
        item(
            "test/suit",
            "Suit",
            "garment",
            "top",
            r,
            r#", "quality": "production", "tags": ["masculine", "formal"],
               "limitations": [{ "note": "Cut for the masculine frame",
                                 "parameter": "frame", "range": [0, 0.5] }]"#,
        ),
        item(
            "test/jumper",
            "Jumper",
            "garment",
            "top",
            r,
            r#", "quality": "production", "tags": ["unisex"]"#,
        ),
    ] {
        library.add_item(
            format!("test/{}.item.json", item.id.replace('/', "-")),
            item,
        );
    }
    let tops = |description: &Description, cut: browser::Cut| -> Vec<String> {
        let filters = Filters {
            cut,
            ..Filters::default()
        };
        let mut ids = listed(&library, description, Shelf::Clothing, &filters, false);
        ids.retain(|id| {
            ["test/gown", "test/suit", "test/jumper", "test/tee"].contains(&id.as_str())
        });
        ids.sort();
        ids
    };
    // The default body is the masculine frame.
    let mut description = realistic(&library);
    assert_eq!(
        browser::feminine_body(&description, library.base(DEFAULT_BASE).unwrap()),
        Some(false)
    );
    assert_eq!(
        tops(&description, browser::Cut::Body),
        ["test/jumper", "test/suit", "test/tee"],
        "the masculine and unisex ones, and what doesn't say"
    );
    // Asked for the feminine ones, they are listed whatever the body (the
    // frame is then no misfit for "Fits this body", on by default).
    assert_eq!(
        tops(&description, browser::Cut::Feminine),
        ["test/gown", "test/jumper", "test/tee"]
    );
    assert_eq!(
        tops(&description, browser::Cut::Masculine),
        ["test/jumper", "test/suit", "test/tee"]
    );
    assert_eq!(
        tops(&description, browser::Cut::All),
        ["test/gown", "test/jumper", "test/suit", "test/tee"]
    );
    // On the feminine frame "This body" is the feminine ones.
    description.parameters.insert("frame".to_owned(), 1.0);
    assert_eq!(
        tops(&description, browser::Cut::Body),
        ["test/gown", "test/jumper", "test/tee"]
    );
    // What is worn is always listed.
    description
        .items
        .push(axcenity_avatar::WornItem::new("test/suit"));
    assert!(tops(&description, browser::Cut::Body).contains(&"test/suit".to_owned()));
    // The page offers the choice, and the kind filter leaves it alone.
    let listing = browser::list(
        &library,
        &description,
        Shelf::Clothing,
        &Filters::default(),
        false,
        &BTreeSet::new(),
        &[],
    );
    assert!(listing.cuts);
    assert!(listing.kinds.contains(&"formal".to_owned()));
    for cut in browser::CUT_TAGS {
        assert!(!listing.kinds.contains(&cut.to_owned()), "{cut}");
    }
    // A cut other than the body's own narrows the list.
    assert!(
        Filters {
            cut: browser::Cut::All,
            ..Filters::default()
        }
        .narrowed()
    );
}
