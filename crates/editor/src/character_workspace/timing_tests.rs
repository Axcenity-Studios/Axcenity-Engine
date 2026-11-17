//! DIS-18 (refinement plan 11.4): how long a parameter change takes to
//! apply to the preview, measured before the drag behaviour is changed.
//! Ignored by default; run it in a release build:
//!
//! ```text
//! cargo test -p axcenity-editor --release -- --ignored drag_apply_time --nocapture
//! ```
//!
//! For each parameter it takes 20 slider steps across its range and times
//! (a) `apply_description` alone, what the 120 ms throttle guards, and
//! (b) that plus the next frame's work before drawing: the animators'
//! step (an owner whose model changed rebinds its clips) and the draw list
//! (the worn parts' derived models are baked there, on first use). The GPU
//! upload is not included. The numbers go in
//! `docs/development/character-studio/refinement/r1/drag-timing.md`.

use std::time::{Duration, Instant};

use super::*;
use crate::character_studio::test_folder;

fn summary(times: &mut [Duration]) -> String {
    times.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    format!(
        "min {:6.2} ms  median {:6.2} ms  max {:6.2} ms",
        ms(times[0]),
        ms(times[times.len() / 2]),
        ms(times[times.len() - 1])
    )
}

#[test]
#[ignore = "a measurement: run in release with --ignored"]
fn drag_apply_time() {
    let folder = test_folder("drag-timing");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    let character = workspace.character.unwrap();
    // Warm up: the first frame loads the models.
    for _ in 0..3 {
        workspace.step_by(1.0 / 60.0);
    }
    println!(
        "DIS-18 apply timing, {} build, realistic default character",
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "release"
        }
    );
    // Body: shape-driven (fat, muscle), an attribute-only proportion
    // (height), a regional shape; face: a mirrored pair side, a single.
    for id in [
        "fat",
        "muscle",
        "height",
        "fat_abdomen",
        "nose_length",
        "eye_size_left",
        "jaw_width_left",
    ] {
        let parameter = workspace.base().unwrap().parameter(id).unwrap().clone();
        let [low, high] = parameter.range;
        let mut apply = Vec::new();
        let mut frame = Vec::new();
        for step in 0..20 {
            let value = low + (high - low) * (step as f32 + 0.5) / 20.0;
            let mut description = workspace.description.clone();
            description.parameters.insert(id.to_owned(), value);
            let started = Instant::now();
            applier::apply_description(
                &mut workspace.world,
                &workspace.assets,
                character,
                &description,
            )
            .unwrap();
            apply.push(started.elapsed());
            workspace.step_by(1.0 / 60.0);
            let _list = axcenity_scene::build_draw_list_posed(
                &workspace.world,
                &workspace.assets,
                workspace.orbit.camera(),
                workspace.poses(),
                &mut axcenity_scene::TerrainCache::new(),
            );
            frame.push(started.elapsed());
        }
        println!("{id:>16}  apply: {}", summary(&mut apply));
        println!(
            "{:>16}  apply + step + draw list: {}",
            "",
            summary(&mut frame)
        );
    }
    let _ = std::fs::remove_dir_all(&folder);
}
