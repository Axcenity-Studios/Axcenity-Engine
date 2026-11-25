//! DIS-18 (refinement plan 11.4): how long a parameter change takes to
//! apply to the preview, measured before the drag behaviour is changed.
//! Ignored by default; run it in a release build:
//!
//! ```text
//! cargo test -p axcenity-editor --release -- --ignored drag_apply_time --nocapture
//! ```
//!
//! For each parameter it takes 20 slider steps across its range, live
//! (refinement R2a: the identity worn as weights) and then baked as before
//! R2a (`gameplay::description::bake` after each apply), and times
//! (a) `apply_description` alone (with the bake when baked), and
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
        // Live (R2a: weights and bone changes, no bake), then baked as
        // every apply was before R2a (`gameplay::description::bake`: the
        // parts' derived models are baked in the step and the draw list).
        for baked in [false, true] {
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
                if baked {
                    axcenity_gameplay::description::bake(
                        &mut workspace.world,
                        &workspace.assets,
                        character,
                    )
                    .unwrap();
                }
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
            let mode = if baked { "baked" } else { "live" };
            println!("{id:>16}  {mode:>5} apply: {}", summary(&mut apply));
            println!(
                "{:>16}  {mode:>5} apply + step + draw list: {}",
                "",
                summary(&mut frame)
            );
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// What the Clothing and Hair pages cost a frame, and what a tile costs the
/// first time the pointer comes over it (Paul, 2026-11-25: "noticeable lag
/// when scrolling through or viewing clothing models"). Ignored by default;
/// run it in a release build:
///
/// ```text
/// cargo test -p axcenity-editor --release -- --ignored browser_costs --nocapture
/// ```
#[test]
#[ignore = "a measurement: run in release with --ignored"]
fn browser_costs() {
    use super::browser::{self, Browse, Pick, Shelf};
    use eframe::egui;
    let folder = test_folder("browser-costs");
    let mut workspace = CharacterWorkspace::open_new(&folder, &Start::default()).unwrap();
    for _ in 0..3 {
        workspace.step_by(1.0 / 60.0);
    }
    let ctx = crate::theme::test_context();
    // With a GPU, each hovered tile is drawn too (the upload of what it
    // loads is part of the hitch).
    let instance = axcenity_render::adapters::instance();
    let gpu = pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
        .ok()
        .and_then(|adapter| pollster::block_on(adapter.request_device(&Default::default())).ok());
    let mut renderer = gpu
        .as_ref()
        .map(|(device, queue)| axcenity_render::TextureRenderer::new(device, queue, 830, 780));
    let frame = |workspace: &mut CharacterWorkspace, events: Vec<egui::Event>| {
        let started = Instant::now();
        crate::theme::run(&ctx, super::shell_tests::input(events), |ui| {
            let _ = workspace.ui(ui, None);
        });
        started.elapsed()
    };
    println!(
        "browser costs, {} build",
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "release"
        }
    );
    for (category, shelf) in [
        (Category::Body, None),
        (Category::Clothing, Some(Shelf::Clothing)),
        (Category::Hair, Some(Shelf::Hair)),
    ] {
        workspace.apply(Intent::PickCategory(category));
        frame(&mut workspace, Vec::new());
        let mut frames: Vec<Duration> =
            (0..60).map(|_| frame(&mut workspace, Vec::new())).collect();
        println!(
            "{category:?} page, a still frame:        {}",
            summary(&mut frames)
        );
        let Some(shelf) = shelf else { continue };
        let library = workspace.library();
        let mut list: Vec<Duration> = (0..60)
            .map(|_| {
                let started = Instant::now();
                let listing = browser::list(
                    &library,
                    workspace.description(),
                    shelf,
                    workspace.browser.filters(shelf),
                    workspace.browser.show_drafts,
                    &workspace.item_favourites,
                    &workspace.recent_items,
                );
                std::hint::black_box(listing.entries.len());
                started.elapsed()
            })
            .collect();
        println!(
            "  browser::list:                      {}",
            summary(&mut list)
        );
        let mut tiles: Vec<Duration> = (0..60)
            .map(|_| {
                let started = Instant::now();
                std::hint::black_box(workspace.browser_tiles(&library, shelf).len());
                started.elapsed()
            })
            .collect();
        println!(
            "  browser_tiles:                      {}",
            summary(&mut tiles)
        );
        let mut pictures: Vec<Duration> = (0..60)
            .map(|_| {
                let wanted = workspace.browser_tiles(&library, shelf);
                let started = Instant::now();
                workspace.load_item_pictures(&ctx, wanted);
                started.elapsed()
            })
            .collect();
        println!(
            "  load_item_pictures:                 {}",
            summary(&mut pictures)
        );
        // Each tile hovered in turn: what the frame that shows it costs
        // the first time (its model is read) and the second.
        let listing = browser::list(
            &library,
            workspace.description(),
            shelf,
            workspace.browser.filters(shelf),
            workspace.browser.show_drafts,
            &workspace.item_favourites,
            &workspace.recent_items,
        );
        let picks: Vec<Pick> = listing
            .entries
            .iter()
            .map(|entry| Pick {
                slot: entry.item.slot.clone(),
                item: Some(entry.item.id.clone()),
            })
            .collect();
        drop(listing);
        for pass in ["first", "second"] {
            let mut hover = Vec::new();
            let mut parts = [Duration::ZERO; 4];
            let mut loading = Duration::ZERO;
            let mut worst = (Duration::ZERO, String::new());
            for pick in &picks {
                // As the page does it: the tile's models are read on the
                // loader's thread while the pointer rests on it, and the
                // preview starts once they are in. What is timed is the
                // interface's thread alone.
                let started = Instant::now();
                workspace.apply(Intent::Browse(Browse::Over(Some(pick.clone()))));
                let asked = started.elapsed();
                let waiting = Instant::now();
                while !workspace.browser.ready {
                    std::thread::sleep(Duration::from_millis(1));
                    workspace.warm_tile();
                }
                loading += waiting.elapsed();
                let started = Instant::now() - asked;
                workspace.apply(Intent::Browse(Browse::Hover(Some(pick.clone()))));
                let applied = started.elapsed();
                workspace.step();
                let stepped = started.elapsed();
                let list = axcenity_scene::build_draw_list_posed(
                    &workspace.world,
                    &workspace.assets,
                    workspace.orbit.camera(),
                    workspace.poses(),
                    &mut axcenity_scene::TerrainCache::new(),
                );
                let listed = started.elapsed();
                if let Some(renderer) = &mut renderer {
                    let _ = renderer.render(&list);
                    let _ = renderer.read_pixels();
                }
                let took = started.elapsed();
                parts[0] += applied;
                parts[1] += stepped - applied;
                parts[2] += listed - stepped;
                parts[3] += took - listed;
                if took > worst.0 {
                    worst = (took, pick.item.clone().unwrap_or_default());
                }
                hover.push(took);
            }
            workspace.apply(Intent::Browse(Browse::Hover(None)));
            workspace.step();
            let total: Duration = hover.iter().sum();
            let ms = |d: Duration| d.as_secs_f64() * 1000.0;
            println!(
                "  a tile hovered, {pass} time ({} tiles, {:.0} ms in all, worst {}): {}",
                hover.len(),
                ms(total),
                worst.1,
                summary(&mut hover)
            );
            println!(
                "    (read on the loader's thread meanwhile: {:.0} ms) of it: the hover's apply {:.0} ms, the step {:.0} ms, the draw list {:.0} ms, drawn and read back {:.0} ms",
                ms(loading),
                ms(parts[0]),
                ms(parts[1]),
                ms(parts[2]),
                ms(parts[3])
            );
        }
    }
    // The tiles' pictures: what the picture thread spends on each, on the
    // GPU device the viewport draws with. Every tile of a page is drawn
    // again when the character changes.
    if let Some((device, queue)) = &gpu {
        workspace.apply(Intent::PickCategory(Category::Clothing));
        let library = workspace.library();
        let tiles = workspace.browser_tiles(&library, Shelf::Clothing);
        let mut painter = thumbnails::Painter::new(&folder, device, queue).unwrap();
        for pass in ["first", "second"] {
            let mut times: Vec<Duration> = tiles
                .iter()
                .map(|(id, subject)| {
                    let started = Instant::now();
                    let pixels = painter.paint(subject);
                    let took = started.elapsed();
                    if let Some(out) = std::env::var_os("AXCENITY_TILE_PICTURES") {
                        let name = format!("{pass}-{}.png", id.replace(['/', ':'], "-"));
                        let (width, height) = thumbnails::PICTURE_SIZE;
                        let _ = image::save_buffer(
                            std::path::Path::new(&out).join(name),
                            &pixels,
                            width,
                            height,
                            image::ColorType::Rgba8,
                        );
                    }
                    took
                })
                .collect();
            let total: Duration = times.iter().sum();
            println!(
                "a tile's picture drawn, {pass} time ({} tiles, {:.0} ms in all): {}",
                times.len(),
                total.as_secs_f64() * 1000.0,
                summary(&mut times)
            );
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}
