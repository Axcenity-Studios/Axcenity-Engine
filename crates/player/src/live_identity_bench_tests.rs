//! Refinement R2a (ADR-0041): what a frame costs with N realistic
//! characters of distinct descriptions, each wearing its identity live
//! (the base meshes, shape weights and bone changes on every pose) and,
//! for comparison, baked (`gameplay::description::bake`: one derived model
//! per part per character, what every character was before R2a). Ignored
//! by default; run it in a release build, one at a time:
//!
//! ```text
//! cargo test -p axcenity-player --release -- --ignored bench_distinct_characters --nocapture --test-threads 1
//! ```
//!
//! The numbers are in
//! `docs/development/character-studio/refinement/r2/r2a-live-identity.md`.

use std::collections::BTreeSet;

use axcenity_avatar::{Scope, randomize};
use axcenity_core::Vector3;
use axcenity_gameplay::description::{apply_description, bake, default_description, library};
use axcenity_render::glam::Vec3;
use axcenity_scene::TerrainCache;

use super::bench_tests::{camera, run};
use super::character_capture::{Stage, repository_library};
use super::*;

/// `count` realistic characters in a grid, each with its own randomized
/// description, idling; baked when `baked`.
fn distinct_characters(count: usize, baked: bool) {
    distinct_characters_in(count, baked, &repository_library(), None);
}

/// [`distinct_characters`] from the character library `library`, each
/// also wearing `item` when one is given (wave 5's pose correctives: the
/// fit gate's test tee with and without them, `tee` and
/// `tee-correctives`).
fn distinct_characters_in(
    count: usize,
    baked: bool,
    library_root: &std::path::Path,
    item: Option<(&str, &str)>,
) {
    let name = format!(
        "bench_distinct_characters: {count} {}{}",
        if baked { "baked" } else { "live" },
        item.map(|(_, label)| format!(" {label}"))
            .unwrap_or_default()
    );
    let stage = Stage::new(
        &format!(
            "bench-{count}-{baked}{}",
            item.map(|(_, label)| format!("-{label}"))
                .unwrap_or_default()
        ),
        library_root,
    )
    .expect("the stage");
    let shipped = library(&stage.assets);
    let default = default_description(&shipped, "axcenity/realistic");
    let mut world = World::new();
    let workspace = world.workspace();
    let mut identities = BTreeSet::new();
    for index in 0..count {
        let (column, row) = ((index % 10) as f32, (index / 10) as f32);
        let at = Vector3::new(column * 3.0 - 13.5, 0.0, -row * 3.0);
        let character = axcenity_project::default_character(&mut world, at);
        world.set_parent(character, Some(workspace)).unwrap();
        let mut description = randomize(
            &default,
            &Scope::All,
            &BTreeSet::new(),
            index as u64 + 1,
            &shipped,
        );
        if let Some((id, _)) = item {
            description.items.push(axcenity_avatar::WornItem::new(id));
        }
        let applied =
            apply_description(&mut world, &stage.assets, character, &description).expect("applied");
        identities.extend(axcenity_anim::identity_key(&world, applied.body));
        if baked {
            bake(&mut world, &stage.assets, character).expect("baked");
        }
    }
    if !baked {
        assert_eq!(identities.len(), count, "every description distinct");
    }
    let mut animators = Animators::new();
    let camera = camera(Vec3::new(0.0, 9.0, 24.0), Vec3::new(0.0, 2.5, -6.0));
    let mut cache = TerrainCache::new();
    run(&name, |steps| {
        steps.time("Animators::update (posing)", || {
            animators.update(&mut world, &stage.assets, 1.0 / 60.0)
        });
        steps.time("build_draw_list", || {
            axcenity_scene::build_draw_list_posed(
                &world,
                &stage.assets,
                camera,
                animators.poses(),
                &mut cache,
            )
        })
    });
    println!(
        "  {count} characters, {} derived models baked",
        stage.assets.derived_count()
    );
    assert!(animators.take_errors().is_empty());
    assert!(stage.assets.take_errors().is_empty());
}

/// `count` default (stylized) characters in the same grid, idling: the
/// body CO8's "today with 41 shapes" names, for comparison.
fn stylized_characters(count: usize) {
    let name = format!("bench_distinct_characters: {count} stylized");
    let stage =
        Stage::new(&format!("bench-{count}-stylized"), &repository_library()).expect("the stage");
    let mut world = World::new();
    let workspace = world.workspace();
    for index in 0..count {
        let (column, row) = ((index % 10) as f32, (index / 10) as f32);
        let at = Vector3::new(column * 3.0 - 13.5, 0.0, -row * 3.0);
        let character = axcenity_project::default_character(&mut world, at);
        world.set_parent(character, Some(workspace)).unwrap();
    }
    let mut animators = Animators::new();
    let camera = camera(Vec3::new(0.0, 9.0, 24.0), Vec3::new(0.0, 2.5, -6.0));
    let mut cache = TerrainCache::new();
    run(&name, |steps| {
        steps.time("Animators::update (posing)", || {
            animators.update(&mut world, &stage.assets, 1.0 / 60.0)
        });
        steps.time("build_draw_list", || {
            axcenity_scene::build_draw_list_posed(
                &world,
                &stage.assets,
                camera,
                animators.poses(),
                &mut cache,
            )
        })
    });
    assert!(animators.take_errors().is_empty());
}

/// The counts and modes to run: `AXCENITY_BENCH_COUNTS` (default
/// `1,10,20,50`) and `AXCENITY_BENCH_MODES` (default `live,baked,stylized`;
/// also `tee` and `tee-correctives`, wave 5's pose correctives).
fn bench_plan() -> (Vec<usize>, Vec<String>) {
    let counts = std::env::var("AXCENITY_BENCH_COUNTS").unwrap_or_else(|_| "1,10,20,50".into());
    let modes =
        std::env::var("AXCENITY_BENCH_MODES").unwrap_or_else(|_| "live,baked,stylized".into());
    (
        counts
            .split(',')
            .filter_map(|count| count.trim().parse().ok())
            .collect(),
        modes
            .split(',')
            .map(|mode| mode.trim().to_owned())
            .collect(),
    )
}

#[test]
#[ignore = "benchmark: run with --release -- --ignored bench_"]
fn bench_distinct_characters() {
    let (counts, modes) = bench_plan();
    for count in counts {
        for mode in &modes {
            match mode.as_str() {
                "live" => distinct_characters(count, false),
                "baked" => distinct_characters(count, true),
                "stylized" => stylized_characters(count),
                "tee" => distinct_characters_in(
                    count,
                    false,
                    super::fit_gate_tests::gate_library(),
                    Some((super::fit_gate_tests::TEE, "tee")),
                ),
                "tee-correctives" => distinct_characters_in(
                    count,
                    false,
                    super::fit_gate_tests::gate_library_with_correctives(),
                    Some((super::fit_gate_tests::TEE, "tee-correctives")),
                ),
                other => panic!("unknown bench mode {other}"),
            }
        }
    }
}
