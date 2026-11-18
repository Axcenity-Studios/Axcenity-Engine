//! The Hair and Clothing browsers (R6 CO25, R7 CO26): what is offered,
//! drafts, search and filters, favourites and recently used, a tile
//! previewed without committing, "Wear" as one undo step and the worn
//! item's controls, only what it declares. With `AXCENITY_SNAPSHOTS`, both
//! pages are drawn for real (`the_browsers_are_drawn`).

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
            r#", "tags": ["denim"]"#,
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

fn ids(listing: &browser::Listing) -> Vec<String> {
    listing
        .entries
        .iter()
        .map(|entry| entry.item.id.clone())
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
    assert!(
        listing
            .body_type_wears
            .iter()
            .any(|(slot, _)| slot == "Underwear"),
        "{:?}",
        listing.body_type_wears
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
    assert_eq!(off.hidden_drafts, 1);
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
            .filter(|e| e.item.id != "test/jeans")
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
    assert_eq!(listing.kinds, ["casual", "cotton", "denim", "hood"]);
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
fn a_tile_is_previewed_without_committing_until_wear() {
    let (folder, mut workspace) = stylized_workspace("browser-preview");
    let before = workspace.description().clone();
    let dirty = workspace.dirty();
    assert!(!wears(&before, "test/streaked-bob"));
    let parts = workspace.worn();
    // Picking a tile shows it on the character only.
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
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
    // Hovering another shows that one while hovered, then the pick again.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert!(wears(workspace.shown(), "test/plain-crop"));
    workspace.apply(Intent::Browse(Browse::Hover(None)));
    assert!(wears(workspace.shown(), "test/streaked-bob"));
    // Esc puts the character back.
    let texts = draw(&mut workspace, Vec::new());
    assert!(
        texts.iter().any(|t| t == "Trying on Streaked bob"),
        "{texts:?}"
    );
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
    assert!(workspace.browser.selected.is_none());
    // Leaving the page puts it back too.
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
        "test/streaked-bob",
    )))));
    workspace.apply(Intent::PickCategory(Category::Body));
    assert_eq!(*workspace.shown(), before);
    assert!(!workspace.can_undo());
    // Wear: one undo step, the preview over, recently used.
    workspace.apply(Intent::PickCategory(Category::Hair));
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
        "test/streaked-bob",
    )))));
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("test/streaked-bob".to_owned()),
    });
    assert!(wears(workspace.description(), "test/streaked-bob"));
    assert!(!workspace.browser.previewing());
    assert_eq!(workspace.recent_items[0], "test/streaked-bob");
    workspace.apply(Intent::Undo);
    assert_eq!(*workspace.description(), before);
    assert!(!workspace.can_undo(), "exactly one step");
    // Any edit ends a preview first.
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
        "test/plain-crop",
    )))));
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
    // The shipped stylized hairstyles are drafts (no `quality`).
    let library = workspace.library();
    let draft = library.item("axcenity/hair-long").unwrap();
    assert!(draft.is_draft());
    let texts = draw(&mut workspace, Vec::new());
    assert!(!texts.iter().any(|t| t == "Long"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Streaked bob"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Show drafts (") && t.ends_with(" hidden)")),
        "{texts:?}"
    );
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    let texts = draw(&mut workspace, Vec::new());
    assert!(texts.iter().any(|t| t == "Long"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Draft"), "{texts:?}");
    // Turning them off again drops a draft being tried on.
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
        "axcenity/hair-long",
    )))));
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
    let texts = draw(&mut workspace, Vec::new());
    assert!(has(&texts, "Hair colour"), "{texts:?}");
    for absent in ["Highlights", "Roots", "Length", "Variant"] {
        assert!(!has(&texts, absent), "{absent}: {texts:?}");
    }
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("test/streaked-bob".to_owned()),
    });
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
    workspace.apply(Intent::PickCategory(Category::Hair));
    let texts = draw(&mut workspace, Vec::new());
    assert!(texts.iter().any(|t| t == "No hairstyles yet"), "{texts:?}");
    workspace.apply(Intent::PickCategory(Category::Clothing));
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
                Intent::Browse(Browse::Select(Some(pick("axcenity/hair-long")))),
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
    });
    assert!(imported.is_ok(), "{:?}", imported.err());
    workspace.apply(Intent::PickCategory(Category::Clothing));
    snapshot(
        &snapshots,
        "realistic-clothing-drafts-off",
        &folder,
        &mut workspace,
    );
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    workspace.apply(Intent::Browse(Browse::Select(Some(Pick {
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
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
        "test/plain-crop",
    )))));
    assert!(workspace.browser.previewing());
    workspace.apply(Intent::ShowHeld(Some(0)));
    workspace.apply(Intent::KeepHeld(0));
    assert!(!workspace.browser.previewing(), "the try-on ended");
    assert_eq!(
        *workspace.shown(),
        *workspace.description(),
        "what is edited is what is seen"
    );
    // Undo and redo change the character too: a try-on gives way.
    workspace.apply(Intent::Browse(Browse::Select(Some(pick(
        "test/plain-crop",
    )))));
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
fn esc_takes_a_try_on_off_while_the_pointer_stays_on_its_tile() {
    // Review fix 1: after a click the pointer is still on the tile; Esc
    // must not be undone by the panel hovering that tile again next frame.
    let (folder, mut workspace) = stylized_workspace("browser-esc-hover");
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    let frame = |workspace: &mut CharacterWorkspace, events: Vec<egui::Event>| {
        theme::run(&ctx, input(events), |ui| {
            assert_eq!(workspace.ui(ui, None), None);
        })
    };
    frame(&mut workspace, Vec::new());
    let output = frame(&mut workspace, Vec::new());
    let tile = text_at(&output, "Plain crop").expect("the Plain crop tile is drawn");
    let button = |pressed| egui::Event::PointerButton {
        pos: tile,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut workspace, vec![egui::Event::PointerMoved(tile)]);
    frame(&mut workspace, vec![egui::Event::PointerMoved(tile)]);
    assert!(wears(workspace.shown(), "test/plain-crop"), "hovered");
    frame(&mut workspace, vec![button(true)]);
    frame(&mut workspace, vec![button(false)]);
    frame(&mut workspace, Vec::new());
    frame(&mut workspace, Vec::new());
    assert!(wears(workspace.shown(), "test/plain-crop"), "picked");
    let escape = egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&mut workspace, vec![escape]);
    // The pointer hasn't moved: the try-on stays off, though the tiles
    // shift up under it when the "Trying on" bar goes.
    for _ in 0..3 {
        frame(&mut workspace, Vec::new());
    }
    assert_eq!(*workspace.shown(), before, "Esc took the try-on off");
    assert!(!workspace.browser.previewing());
    // The same Hover again (what the panel would send) changes nothing.
    workspace.apply(Intent::Browse(Browse::Hover(Some(pick("test/plain-crop")))));
    assert_eq!(*workspace.shown(), before);
    // Off the tile and back: hovering previews it again.
    frame(
        &mut workspace,
        vec![egui::Event::PointerMoved(egui::pos2(5.0, 5.0))],
    );
    frame(&mut workspace, Vec::new());
    frame(&mut workspace, vec![egui::Event::PointerMoved(tile)]);
    frame(&mut workspace, Vec::new());
    assert!(
        wears(workspace.shown(), "test/plain-crop"),
        "hovering the tile again previews it"
    );
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

#[test]
fn clicking_near_a_tiles_top_tries_on_that_tile() {
    // Review fix 2: picking a tile must not move the grid under a still
    // pointer, or the tile above it would be previewed in its place.
    let (folder, mut workspace) = stylized_workspace("browser-click-top");
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
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
        "the point is on the Long tile"
    );
    click_and_hold_still(&ctx, &mut workspace, at);
    assert_eq!(
        workspace
            .browser
            .selected
            .as_ref()
            .and_then(|p| p.item.as_deref()),
        Some("axcenity/hair-long")
    );
    assert_eq!(
        hair_of(&workspace, workspace.shown()).as_deref(),
        Some("axcenity/hair-long"),
        "clicked Long, so the character shows Long (hovered: {:?})",
        workspace.browser.hovered
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn clicking_the_worn_tile_while_trying_one_on_previews_nothing() {
    // Review fix 2: clicking the worn tile ends the pick (Select(None)); the
    // tiles must not move under the still pointer and preview another.
    let (folder, mut workspace) = stylized_workspace("browser-click-worn");
    workspace.apply(Intent::Browse(Browse::ShowDrafts(true)));
    // Bob is worn: its tile is in the first row, with Long below it.
    workspace.apply(Intent::Wear {
        slot: "hair".to_owned(),
        item: Some("axcenity/hair-bob".to_owned()),
    });
    let before = workspace.description().clone();
    let ctx = theme::test_context();
    frame_in(&ctx, &mut workspace, Vec::new());
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let long = text_at(&output, "Long").expect("the Long tile is drawn");
    click_and_hold_still(&ctx, &mut workspace, long);
    assert!(workspace.browser.selected.is_some(), "Long is picked");
    let output = frame_in(&ctx, &mut workspace, Vec::new());
    let bob = text_at(&output, "Bob").expect("the worn Bob tile is drawn");
    assert!(long.y > bob.y + 50.0, "Long is below Bob");
    click_and_hold_still(&ctx, &mut workspace, bob);
    assert!(workspace.browser.selected.is_none(), "the pick is over");
    assert_eq!(
        *workspace.shown(),
        before,
        "nothing is tried on (hovered: {:?})",
        workspace.browser.hovered
    );
    let _ = std::fs::remove_dir_all(&folder);
}
