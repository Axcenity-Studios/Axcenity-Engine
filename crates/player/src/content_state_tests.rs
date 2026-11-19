//! Content states in the player (Paul's decision 12 of 2026-11-18,
//! ADR-0043): the state decides what Character Studio offers and what
//! Randomize draws, never what a game draws. A saved character wearing
//! Legacy and Draft items applies and draws every one of them on both
//! families, and an older project, whose library says no state at all,
//! still draws its default character.

use std::path::Path;

use axcenity_assets::Assets;
use axcenity_avatar::{Description, Quality, V1_BASE, WornItem};
use axcenity_core::{InstanceId, Vector3, World};

use super::character_capture::{Stage, repository_library};

/// Applies `description` to a default character in a new World; returns
/// the names of the parts it wears and what was left out.
fn applied(assets: &Assets, description: &Description) -> (Vec<String>, Vec<String>) {
    let mut world = World::new();
    let character = axcenity_project::default_character(&mut world, Vector3::default());
    let workspace = world.workspace();
    world.set_parent(character, Some(workspace)).unwrap();
    let applied = axcenity_gameplay::description::apply_description(
        &mut world,
        assets,
        character,
        description,
    )
    .expect("the description applies");
    let name = |part: &InstanceId| world.get(*part).map(|p| p.name().to_owned());
    let parts = applied.parts.iter().filter_map(name).collect();
    let problems = applied.problems.iter().map(ToString::to_string).collect();
    (parts, problems)
}

/// The parts `description`'s items resolve to (what must be drawn).
fn item_parts(assets: &Assets, description: &Description) -> Vec<String> {
    let library = axcenity_gameplay::description::library(assets);
    let resolved = axcenity_avatar::resolve(&library, description).unwrap();
    resolved
        .parts
        .into_iter()
        .filter(|part| part.item.is_some())
        .map(|part| part.name)
        .collect()
}

/// A realistic and a stylized character saved wearing Legacy and Draft
/// items: every item applies as a part, nothing is left out.
#[test]
fn a_saved_character_wearing_legacy_and_draft_items_draws_on_both_families() {
    let stage = Stage::new("content-states", &repository_library()).expect("the stage");
    let library = axcenity_gameplay::description::library(&stage.assets);
    let mut realistic = Description::new("Saved", "axcenity/realistic");
    realistic.items = [
        "axcenity/realistic-boxer-briefs",
        "axcenity/realistic-hair-short-crop",
        "axcenity/realistic-crew-tee",
        "axcenity/realistic-joggers",
        "axcenity/realistic-sneakers",
    ]
    .map(WornItem::new)
    .to_vec();
    let mut stylized = Description::new("Saved", V1_BASE);
    stylized.items = [
        "axcenity/hair-long",
        "axcenity/tee",
        "axcenity/cargo-pants",
        "axcenity/sneakers",
        "axcenity/briefs",
    ]
    .map(WornItem::new)
    .to_vec();
    let mut states = Vec::new();
    for description in [&realistic, &stylized] {
        for worn in &description.items {
            states.push(library.item(&worn.id).unwrap().state());
        }
        let expected = item_parts(&stage.assets, description);
        assert_eq!(expected.len(), description.items.len(), "{expected:?}");
        let (parts, problems) = applied(&stage.assets, description);
        assert!(problems.is_empty(), "{}: {problems:?}", description.base);
        for part in &expected {
            assert!(
                parts.contains(part),
                "{}: {part} in {parts:?}",
                description.base
            );
        }
    }
    // The test is about both states.
    assert!(states.contains(&Quality::Legacy) && states.contains(&Quality::Draft));
}

/// An older project: its library (the built-in stylized package) was
/// written before the states, so no manifest says one. It reads, every
/// item is Legacy, and the default character draws its whole outfit.
#[test]
fn an_older_projects_library_without_states_still_draws() {
    let folder = axcenity_test_dirs::TestDir::adopt(std::env::temp_dir().join(format!(
        "axcenity-content-states-old-project-{}",
        std::process::id()
    )));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    axcenity_project::add_default_humanoid(&folder).unwrap();
    axcenity_project::add_character_library(&folder).unwrap();
    let root = folder.join(axcenity_avatar::LIBRARY_FOLDER);
    let mut stripped = 0;
    strip_states(&root, &mut stripped);
    assert_eq!(stripped, 12, "the stylized package's items");
    let assets = Assets::new(&folder);
    let library = axcenity_gameplay::description::library(&assets);
    assert!(library.items.values().all(|item| item.quality.is_none()));
    assert!(library.items.values().all(|item| item.is_legacy()));
    let description = axcenity_gameplay::description::default_description(&library, V1_BASE);
    assert!(!description.items.is_empty());
    let expected = item_parts(&assets, &description);
    let (parts, problems) = applied(&assets, &description);
    assert!(problems.is_empty(), "{problems:?}");
    for part in &expected {
        assert!(parts.contains(part), "{part} in {parts:?}");
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// Takes `quality` out of every item manifest under `folder`, as an older
/// engine wrote them.
fn strip_states(folder: &Path, stripped: &mut usize) {
    for entry in std::fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            strip_states(&path, stripped);
        } else if path.to_string_lossy().ends_with(".item.json") {
            let text = std::fs::read_to_string(&path).unwrap();
            let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
            if json.as_object_mut().unwrap().remove("quality").is_some() {
                *stripped += 1;
            }
            std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()).unwrap();
        }
    }
}
