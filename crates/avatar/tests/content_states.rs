//! Content states on the repository's library (Paul's decision 12 of
//! 2026-11-18, ADR-0043, `asset-quality.md` §5): every shipped item says
//! its state; the stylized family's older items and the realistic
//! underwear are Legacy, the four wave-4 items Draft, none Production yet
//! (decision 13); Randomize draws Production only and never strips a
//! character; a character wearing Legacy or Draft items still resolves,
//! and so does an older project's library, whose manifests say no state.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use axcenity_avatar::{
    Description, ItemManifest, Library, Problem, Quality, Scope, V1_BASE, WornItem,
    check_declared_state, randomize, resolve,
};

const REALISTIC: &str = "axcenity/realistic";

/// The four wave-4 items (decision 13: none becomes Production).
const WAVE_4_DRAFTS: [&str; 4] = [
    "axcenity/realistic-hair-short-crop",
    "axcenity/realistic-crew-tee",
    "axcenity/realistic-joggers",
    "axcenity/realistic-sneakers",
];

/// Wave 5's licensed items, Draft until the lead's review and Paul's
/// sign-off of the first of their kind (`refinement/wave5/`).
const WAVE_5_DRAFTS: [&str; 1] = ["axcenity/realistic-fleece-joggers"];

/// The realistic underwear, made before the standard (Legacy).
const REALISTIC_UNDERWEAR: [&str; 3] = [
    "axcenity/realistic-boxer-briefs",
    "axcenity/realistic-briefs",
    "axcenity/realistic-sports-bra",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/characters")
}

fn read() -> Library {
    let (library, catalogue, problems) = Library::read(&root());
    assert!(problems.is_empty(), "{}", lines(&problems));
    assert!(catalogue.is_some());
    library
}

fn lines(problems: &[Problem]) -> String {
    problems
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `*.item.json` under `folder`.
fn item_files(folder: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            item_files(&path, out);
        } else if path.to_string_lossy().ends_with(".item.json") {
            out.push(path);
        }
    }
}

/// Every item manifest file the engine ships, catalogued or not, says its
/// state explicitly, one of the three (`check_declared_state`).
#[test]
fn every_shipped_item_declares_its_state() {
    let mut files = Vec::new();
    item_files(&root(), &mut files);
    assert!(files.len() >= 19, "{} item manifests", files.len());
    let mut problems = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let item = ItemManifest::from_json(&text)
            .unwrap_or_else(|err| panic!("{}: {err}", file.display()));
        for problem in check_declared_state(&item) {
            problems.push(format!("{}: {problem}", file.display()));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The classification (ADR-0043): the stylized family's items are Legacy;
/// the realistic underwear is Legacy (made before the standard, never
/// passed it); the four wave-4 items stay Draft; nothing is Production.
#[test]
fn the_shipped_items_are_classified() {
    let library = read();
    for (id, item) in &library.items {
        let expected =
            if WAVE_4_DRAFTS.contains(&id.as_str()) || WAVE_5_DRAFTS.contains(&id.as_str()) {
                Quality::Draft
            } else if REALISTIC_UNDERWEAR.contains(&id.as_str())
                || item.bases.iter().any(|b| b == V1_BASE)
            {
                Quality::Legacy
            } else {
                panic!("`{id}` isn't classified here: add it");
            };
        assert_eq!(item.state(), expected, "{id}");
    }
    for id in WAVE_4_DRAFTS.iter().chain(&REALISTIC_UNDERWEAR) {
        assert!(library.item(id).is_some(), "{id} ships");
    }
    let production = library.items.values().filter(|i| i.is_production()).count();
    assert_eq!(production, 0, "nothing is Production before the review");
}

/// The starts Randomize is tried from on `family`: its default character
/// and every one of its presets applied to it.
fn starts(library: &Library, family: &str) -> Vec<Description> {
    let base = library.base(family).unwrap();
    let mut start = Description::new("Npc", family);
    start.items = base.defaults.items.iter().map(WornItem::new).collect();
    axcenity_avatar::wear_choices(&mut start, library);
    let mut starts = vec![start.clone()];
    for preset in library.presets.values().filter(|p| p.base == family) {
        let mut from = start.clone();
        if axcenity_avatar::apply_preset(&mut from, preset, library).is_ok() {
            starts.push(from);
        }
    }
    starts
}

/// Checks a randomized character against where it started: every item it
/// wears is Production, or what it wore before (the very same worn
/// item), or what a parameter's choice wears (the body type's
/// underwear). Returns the items Randomize drew.
#[track_caller]
fn only_production_drawn(
    library: &Library,
    start: &Description,
    drawn: &Description,
    at: &str,
) -> Vec<String> {
    let chosen = axcenity_avatar::chosen_slots(library, &drawn.base);
    let mut new = Vec::new();
    for worn in &drawn.items {
        let item = library.item(&worn.id).expect("a library item");
        if start.items.contains(worn) || chosen.contains(&item.slot) {
            continue;
        }
        assert!(
            item.is_production(),
            "{at}: drew `{}`, which is {:?}",
            worn.id,
            item.state()
        );
        new.push(worn.id.clone());
    }
    new
}

/// 1,000 seeded randomizations on each family, from its default and its
/// presets, with every scope that draws items: never a Legacy or Draft
/// item. With no Production item in the library (today), every item slot
/// keeps what it wore, so nothing is taken off (the stylized underwear
/// included), and the realistic body keeps its type's underwear.
#[test]
fn randomize_on_both_families_draws_no_legacy_or_draft_item() {
    let library = read();
    for family in [V1_BASE, REALISTIC] {
        let starts = starts(&library, family);
        let scopes = [
            Scope::All,
            Scope::Category(axcenity_avatar::Category::Hair),
            Scope::Category(axcenity_avatar::Category::Clothing),
        ];
        for seed in 0..1000u64 {
            let start = &starts[seed as usize % starts.len()];
            let scope = &scopes[seed as usize % scopes.len()];
            let drawn = randomize(start, scope, &BTreeSet::new(), seed, &library);
            let at = format!("{family} {scope:?} seed {seed}");
            let new = only_production_drawn(&library, start, &drawn, &at);
            assert!(new.is_empty(), "{at}: {new:?}");
            // Nothing taken off but what a choice swaps (the underwear when
            // the body type changes).
            let chosen = axcenity_avatar::chosen_slots(&library, family);
            for worn in &start.items {
                let slot = &library.item(&worn.id).unwrap().slot;
                assert!(
                    chosen.contains(slot) || drawn.items.contains(worn),
                    "{at}: took off `{}`",
                    worn.id
                );
            }
            // Never undressed: the realistic body wears its type's
            // underwear, the stylized one what it wore.
            if family == REALISTIC {
                assert!(
                    drawn
                        .items
                        .iter()
                        .any(|w| REALISTIC_UNDERWEAR.contains(&w.id.as_str())),
                    "{at}: no underwear"
                );
            }
            let resolved = resolve(&library, &drawn).unwrap();
            assert!(resolved.problems.is_empty(), "{at}");
        }
    }
}

/// With a Production item added to each family (a copy of a shipped item,
/// marked production), Randomize draws it, and still never a Legacy or
/// Draft one.
#[test]
fn randomize_draws_a_production_item_once_there_is_one() {
    let mut library = read();
    let copies = [
        ("axcenity/hair-bob", "test/production-bob"),
        ("axcenity/realistic-hair-short-crop", "test/production-crop"),
    ];
    for (from, id) in copies {
        let mut item = library.item(from).unwrap().clone();
        item.id = id.to_owned();
        item.quality = Some(Quality::Production);
        let path = library.manifest_path(from).unwrap().to_owned();
        library.add_item(path.replace(".item.json", "-production.item.json"), item);
    }
    for (family, (_, id)) in [V1_BASE, REALISTIC].into_iter().zip(copies) {
        let starts = starts(&library, family);
        let mut drawn_ids = BTreeSet::new();
        for seed in 0..1000u64 {
            let start = &starts[seed as usize % starts.len()];
            let drawn = randomize(start, &Scope::All, &BTreeSet::new(), seed, &library);
            let at = format!("{family} seed {seed}");
            drawn_ids.extend(only_production_drawn(&library, start, &drawn, &at));
        }
        assert!(drawn_ids.contains(id), "{family}: never drew {id}");
    }
}

/// A saved character wearing Legacy and Draft items validates and
/// resolves on both families, each item a part (ADR-0043: old projects
/// keep loading).
#[test]
fn a_character_wearing_legacy_and_draft_items_resolves() {
    let mut library = read();
    // A stylized draft: no stylized item is one, so a copy of the tee.
    let mut draft = library.item("axcenity/tee").unwrap().clone();
    draft.id = "test/stylized-draft-tee".to_owned();
    draft.quality = Some(Quality::Draft);
    library.add_item("humanoid/items/test-draft-tee.item.json", draft);

    let mut realistic = Description::new("Old save", REALISTIC);
    realistic.items = [
        "axcenity/realistic-boxer-briefs",
        "axcenity/realistic-hair-short-crop",
        "axcenity/realistic-crew-tee",
        "axcenity/realistic-joggers",
        "axcenity/realistic-sneakers",
    ]
    .map(WornItem::new)
    .to_vec();
    let mut stylized = Description::new("Old save", V1_BASE);
    stylized.items = [
        "axcenity/hair-long",
        "test/stylized-draft-tee",
        "axcenity/cargo-pants",
        "axcenity/sneakers",
        "axcenity/briefs",
    ]
    .map(WornItem::new)
    .to_vec();
    for description in [realistic, stylized] {
        let problems = description.validate(&library);
        assert!(problems.is_empty(), "{}", lines(&problems));
        let resolved = resolve(&library, &description).unwrap();
        assert!(
            resolved.problems.is_empty(),
            "{}",
            lines(&resolved.problems)
        );
        for worn in &description.items {
            assert!(
                resolved
                    .parts
                    .iter()
                    .any(|part| part.item.as_deref() == Some(&worn.id)),
                "{} draws `{}`",
                description.base,
                worn.id
            );
        }
    }
}

/// An older project's copy of the library says no state (its manifests
/// were written before ADR-0043): it still reads, every item is Legacy,
/// and its default characters resolve.
#[test]
fn an_older_projects_library_without_states_still_loads() {
    let library = read();
    let mut old = Library::new();
    for (id, base) in &library.bases {
        old.add_base(library.manifest_path(id).unwrap(), base.clone());
    }
    for (id, item) in &library.items {
        // The file as an older engine wrote it: no `quality` at all.
        let mut json: serde_json::Value = serde_json::from_str(&item.to_json()).unwrap();
        json.as_object_mut().unwrap().remove("quality");
        let text = serde_json::to_string(&json).unwrap();
        let read = ItemManifest::from_json(&text).expect("an older manifest reads");
        assert_eq!(read.state(), Quality::Legacy, "{id}");
        old.add_item(library.manifest_path(id).unwrap(), read);
    }
    for family in [V1_BASE, REALISTIC] {
        for start in starts(&library, family) {
            let resolved = resolve(&old, &start).unwrap();
            assert!(resolved.problems.is_empty(), "{family}");
            let drawn: BTreeSet<&str> = resolved
                .parts
                .iter()
                .filter_map(|p| p.item.as_deref())
                .collect();
            assert_eq!(
                drawn.len(),
                start.items.len(),
                "{family}: every worn item draws"
            );
        }
    }
}

/// Default outfits and presets (asset-quality.md §5): no default and no
/// shipped preset wears a Draft; the stylized default outfit is Legacy (a
/// compatibility case: the family has no Production item yet) and the
/// realistic default wears only its underwear (Legacy, the modesty layer).
#[test]
fn default_outfits_keep_their_legacy_items_and_wear_no_draft() {
    let library = read();
    let stylized = library.base(V1_BASE).unwrap();
    assert!(!stylized.defaults.items.is_empty());
    for id in &stylized.defaults.items {
        assert_eq!(library.item(id).unwrap().state(), Quality::Legacy, "{id}");
    }
    let realistic = library.base(REALISTIC).unwrap();
    for id in &realistic.defaults.items {
        assert!(REALISTIC_UNDERWEAR.contains(&id.as_str()), "{id}");
    }
    for (id, preset) in &library.presets {
        for worn in &preset.items {
            let item = library.item(&worn.id).unwrap();
            assert!(
                !item.is_draft(),
                "the preset `{id}` wears the draft `{}`",
                worn.id
            );
            // Legacy only for compatibility: the stylized family, or the
            // realistic body type's underwear.
            if item.is_legacy() {
                assert!(
                    preset.base == V1_BASE || REALISTIC_UNDERWEAR.contains(&worn.id.as_str()),
                    "the preset `{id}` wears the legacy `{}`",
                    worn.id
                );
            }
        }
    }
}
