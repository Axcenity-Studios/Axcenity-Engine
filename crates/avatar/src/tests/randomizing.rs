//! Randomize (plan section 11.3): deterministic from its seed, inside each
//! parameter's `random` share, only what the scope names, never what is
//! locked, and always a character that validates and resolves.

use std::collections::BTreeSet;

use super::texts;
use crate::fixtures;
use crate::{Category, Description, Library, Scope, WornItem, randomize, resolve};

fn start() -> Description {
    let mut description = Description::new("Ada", "test/body");
    description.parameters.insert("fat".to_owned(), 0.3);
    description.parameters.insert("height".to_owned(), 1.1);
    description.items = vec![WornItem::new("test/tee")];
    description
}

fn locks(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

#[track_caller]
fn sound(description: &Description, library: &Library, seed: u64) {
    assert_eq!(
        texts(&description.validate(library)),
        Vec::<String>::new(),
        "seed {seed}"
    );
    let resolved = resolve(library, description).unwrap();
    assert_eq!(
        texts(&resolved.problems),
        Vec::<String>::new(),
        "seed {seed}"
    );
}

#[test]
fn the_same_seed_gives_the_same_character() {
    let library = fixtures::rich_library();
    let none = BTreeSet::new();
    let a = randomize(&start(), &Scope::All, &none, 7, &library);
    let b = randomize(&start(), &Scope::All, &none, 7, &library);
    assert_eq!(a, b);
    let different = (0..20)
        .filter(|seed| randomize(&start(), &Scope::All, &none, *seed, &library) != a)
        .count();
    assert!(
        different >= 18,
        "seeds give different characters: {different}"
    );
    assert_eq!(a.name, "Ada", "the name is kept");
}

#[test]
fn a_thousand_random_characters_are_all_sound() {
    for library in [fixtures::rich_library(), fixtures::library()] {
        let none = BTreeSet::new();
        let mut worn: BTreeSet<String> = BTreeSet::new();
        for seed in 0..1000 {
            let description = randomize(&start(), &Scope::All, &none, seed, &library);
            sound(&description, &library, seed);
            worn.extend(description.items.iter().map(|item| item.id.clone()));
        }
        // Every item that fits gets worn by some seed.
        for (id, item) in &library.items {
            assert!(
                worn.contains(id) || !item.bases.iter().any(|b| b == "test/body"),
                "{id} never drawn"
            );
        }
    }
}

#[test]
fn values_stay_inside_the_random_share() {
    let library = fixtures::rich_library();
    let none = BTreeSet::new();
    for seed in 0..300 {
        let d = randomize(&start(), &Scope::All, &none, seed, &library);
        // `fat` declares `random: [0, 0.6]`.
        let fat = d.parameters["fat"];
        assert!((0.0..=0.6).contains(&fat), "fat {fat} at seed {seed}");
        // `height` (0.5 to 1.5) has none: the middle half of its range.
        let height = d.parameters["height"];
        assert!((0.75..=1.25).contains(&height), "height {height}");
        // A parameter with choices takes one of them.
        let frame = d.parameters.get("frame").copied().unwrap_or(0.0);
        assert!(frame == 0.0 || frame == 1.0, "frame {frame}");
        // `fat + muscle` keep to their `max_sum`.
        let muscle = d.parameters.get("muscle").copied().unwrap_or(0.0);
        assert!(fat + muscle <= 1.5 + 1e-4);
        // An item's own parameters are drawn too.
        if let Some(bob) = d.items.iter().find(|item| item.id == "test/bob") {
            let length = bob.parameters["length"];
            assert!((0.2..=0.8).contains(&length), "length {length}");
        }
    }
}

#[test]
fn locked_parameters_and_slots_never_change() {
    let library = fixtures::rich_library();
    let locked = locks(&["fat", "eye_size_left", "top"]);
    let mut start = start();
    start.parameters.insert("eye_size_left".to_owned(), -0.8);
    for seed in 0..1000 {
        let d = randomize(&start, &Scope::All, &locked, seed, &library);
        assert_eq!(d.parameters["fat"], 0.3, "seed {seed}");
        assert_eq!(d.parameters["eye_size_left"], -0.8, "seed {seed}");
        assert!(
            d.items.iter().any(|item| item.id == "test/tee"),
            "seed {seed}"
        );
        assert!(
            !d.items.iter().any(|item| item.id == "test/dress"),
            "seed {seed}"
        );
        sound(&d, &library, seed);
    }
}

#[test]
fn a_scope_limits_what_changes() {
    let library = fixtures::rich_library();
    let none = BTreeSet::new();
    let start = start();
    let changed = |d: &Description| -> BTreeSet<String> {
        let ids: BTreeSet<&String> = d.parameters.keys().chain(start.parameters.keys()).collect();
        ids.into_iter()
            .filter(|id| d.parameters.get(*id) != start.parameters.get(*id))
            .cloned()
            .collect()
    };
    for seed in 0..200 {
        // The face: only face parameters, no items.
        let d = randomize(
            &start,
            &Scope::Category(Category::Face),
            &none,
            seed,
            &library,
        );
        assert!(
            changed(&d).iter().all(|id| id.starts_with("eye_size")),
            "{seed}"
        );
        assert_eq!(d.items, start.items);
        // A feature: its parameters (the eyes are the eye group's).
        let d = randomize(
            &start,
            &Scope::Feature("torso".to_owned()),
            &none,
            seed,
            &library,
        );
        assert!(
            changed(&d).iter().all(|id| id == "fat" || id == "muscle"),
            "{seed}"
        );
        // A list of parameters.
        let d = randomize(
            &start,
            &Scope::Parameters(vec!["height".to_owned()]),
            &none,
            seed,
            &library,
        );
        assert!(changed(&d).iter().all(|id| id == "height"), "{seed}");
        // The clothing category changes items only, and never the hair.
        let mut hair = start.clone();
        hair.items.push(WornItem::new("test/hair"));
        let d = randomize(
            &hair,
            &Scope::Category(Category::Clothing),
            &none,
            seed,
            &library,
        );
        assert!(changed(&d).is_empty(), "{seed}");
        assert!(d.items.iter().any(|item| item.id == "test/hair"), "{seed}");
        sound(&d, &library, seed);
    }
    // Over many seeds a scope does change what it names.
    let any_face = (0..50).any(|seed| {
        let d = randomize(
            &start,
            &Scope::Category(Category::Face),
            &none,
            seed,
            &library,
        );
        !changed(&d).is_empty()
    });
    assert!(any_face);
    let any_items = (0..50).any(|seed| {
        randomize(
            &start,
            &Scope::Category(Category::Clothing),
            &none,
            seed,
            &library,
        )
        .items
            != start.items
    });
    assert!(any_items);
}

#[test]
fn a_description_on_a_base_the_library_lacks_comes_back_unchanged() {
    let mut lost = start();
    lost.base = "test/nobody".to_owned();
    let d = randomize(
        &lost,
        &Scope::All,
        &BTreeSet::new(),
        1,
        &fixtures::rich_library(),
    );
    assert_eq!(d, lost);
}
