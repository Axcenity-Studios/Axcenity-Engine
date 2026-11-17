//! The realistic character's Skin and Eye material values, drawn by the
//! player's own renderer (Character Studio refinement R5, CO7's first
//! slice): a description with every value at its neutral setting draws the
//! same pixels as one with none, and moving each drawn value off neutral
//! changes the pixels of an in-engine capture the way the value's meaning
//! says (`avatar`'s `material.rs`). Skipped on machines without a GPU.

use axcenity_avatar::{Colour, Description, MaterialValue};

use super::character_capture::{Settings, Shot, Stage, capture, repository_library};

/// Small and quick: the tests compare averages, not detail.
fn settings() -> Settings {
    Settings {
        size: (320, 180),
        frames: 8,
        ..Settings::default()
    }
}

/// The masculine default of the realistic family (a benchmark file).
fn realistic() -> Description {
    let file =
        repository_library().join("benchmarks/realistic-masculine-body-default.character.json");
    Description::from_json(&std::fs::read_to_string(file).unwrap()).unwrap()
}

/// `description` with `values` set on its material `slot`.
fn with(description: &Description, slot: &str, values: &[(&str, MaterialValue)]) -> Description {
    let mut changed = description.clone();
    let slot = changed.materials.entry(slot.to_owned()).or_default();
    for (name, value) in values {
        slot.insert((*name).to_owned(), *value);
    }
    changed
}

fn number(value: f32) -> MaterialValue {
    MaterialValue::Number(value)
}

fn colour(text: &str) -> MaterialValue {
    MaterialValue::Colour(Colour::parse(text).unwrap())
}

/// The shot of `view` under the studio light.
fn studio<'a>(shots: &'a [Shot], view: &str) -> &'a [u8] {
    &shots
        .iter()
        .find(|shot| shot.light == "studio" && shot.view == view)
        .unwrap()
        .pixels
}

fn linear(byte: u8) -> f32 {
    let c = f32::from(byte) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// What changed between two pictures: the mean linear colour of each over
/// the pixels that differ (by more than 2 in a channel), and their share.
struct Change {
    before: [f32; 3],
    after: [f32; 3],
    share: f32,
    /// The pixel indexes that differ.
    pixels: Vec<usize>,
}

fn change(before: &[u8], after: &[u8]) -> Change {
    let pixels: Vec<usize> = (0..before.len() / 4)
        .filter(|&i| (0..3).any(|c| before[i * 4 + c].abs_diff(after[i * 4 + c]) > 2))
        .collect();
    let mean = |picture: &[u8]| -> [f32; 3] {
        let mut sum = [0.0; 3];
        for &i in &pixels {
            for (c, total) in sum.iter_mut().enumerate() {
                *total += linear(picture[i * 4 + c]);
            }
        }
        sum.map(|total| total / pixels.len().max(1) as f32)
    };
    Change {
        before: mean(before),
        after: mean(after),
        share: pixels.len() as f32 / (before.len() / 4) as f32,
        pixels,
    }
}

fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// How rough a picture is over `pixels`: the mean absolute difference of
/// each pixel's luminance from its right and lower neighbours'.
fn texture(picture: &[u8], width: usize, pixels: &[usize]) -> f32 {
    let lum = |i: usize| {
        luminance([
            linear(picture[i * 4]),
            linear(picture[i * 4 + 1]),
            linear(picture[i * 4 + 2]),
        ])
    };
    let total = picture.len() / 4;
    let mut sum = 0.0;
    let mut count = 0;
    for &i in pixels {
        if i + width + 1 < total {
            sum += (lum(i) - lum(i + 1)).abs() + (lum(i) - lum(i + width)).abs();
            count += 1;
        }
    }
    sum / count.max(1) as f32
}

/// The brightest share of `pixels` (their mean luminance): highlights.
fn highlights(picture: &[u8], pixels: &[usize]) -> f32 {
    let mut lums: Vec<f32> = pixels
        .iter()
        .map(|&i| {
            luminance([
                linear(picture[i * 4]),
                linear(picture[i * 4 + 1]),
                linear(picture[i * 4 + 2]),
            ])
        })
        .collect();
    lums.sort_by(f32::total_cmp);
    let top = &lums[lums.len() - (lums.len() / 50).max(1)..];
    top.iter().sum::<f32>() / top.len() as f32
}

#[test]
fn neutral_skin_and_eye_values_draw_the_same_pixels_as_none() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("skin-neutral", &repository_library()).expect("the stage");
    let plain = realistic();
    let white = colour("#ffffff");
    let neutral = with(
        &plain,
        "Skin",
        &[
            ("tint", white),
            ("melanin", number(0.0)),
            ("redness", number(0.0)),
            ("undertone", number(0.0)),
            ("roughness_scale", number(1.0)),
            ("detail_strength", number(1.0)),
            ("scatter_scale", number(1.0)),
        ],
    );
    let neutral = with(&neutral, "Sclera", &[("sclera_tint", white)]);
    let neutral = with(&neutral, "Iris", &[("iris_tint", white)]);
    let first = capture(&device, &queue, &stage.assets, &plain, settings()).unwrap();
    let second = capture(&device, &queue, &stage.assets, &neutral, settings()).unwrap();
    for (one, two) in first.iter().zip(&second) {
        assert!(
            one.pixels == two.pixels,
            "{} {}: neutral values changed the picture",
            one.light,
            one.view
        );
    }
}

#[test]
fn skin_tone_values_change_the_picture_the_way_they_mean() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("skin-tone", &repository_library()).expect("the stage");
    let plain = realistic();
    let shots = |description: &Description| {
        capture(&device, &queue, &stage.assets, description, settings()).unwrap()
    };
    let base = shots(&plain);
    let skin = |values: &[(&str, MaterialValue)]| shots(&with(&plain, "Skin", values));
    for view in ["front", "face-front"] {
        let before = studio(&base, view);

        let deeper = skin(&[("melanin", number(0.8))]);
        let darker = change(before, studio(&deeper, view));
        assert!(
            darker.share > 0.05,
            "{view}: melanin 0.8 changed {}",
            darker.share
        );
        assert!(
            luminance(darker.after) < luminance(darker.before) * 0.6,
            "{view}: melanin 0.8 isn't darker: {:?} to {:?}",
            darker.before,
            darker.after
        );
        let lighter = change(before, studio(&skin(&[("melanin", number(-0.8))]), view));
        assert!(
            luminance(lighter.after) > luminance(lighter.before) * 1.15,
            "{view}: melanin -0.8 isn't lighter: {:?} to {:?}",
            lighter.before,
            lighter.after
        );

        let flushed = change(before, studio(&skin(&[("redness", number(0.6))]), view));
        let red = |[r, g, _]: [f32; 3]| r / g;
        assert!(
            red(flushed.after) > red(flushed.before) * 1.02,
            "{view}: redness 0.6 isn't redder: {:?} to {:?}",
            flushed.before,
            flushed.after
        );

        let warmth = |[r, g, b]: [f32; 3]| (r + g) / (2.0 * b);
        let warm = change(before, studio(&skin(&[("undertone", number(0.7))]), view));
        assert!(
            warmth(warm.after) > warmth(warm.before) * 1.02,
            "{view}: undertone 0.7 isn't warmer: {:?} to {:?}",
            warm.before,
            warm.after
        );
        let cool = change(before, studio(&skin(&[("undertone", number(-0.7))]), view));
        assert!(
            warmth(cool.after) < warmth(cool.before) / 1.02,
            "{view}: undertone -0.7 isn't cooler: {:?} to {:?}",
            cool.before,
            cool.after
        );
    }
}

#[test]
fn skin_surface_and_eye_values_change_the_picture_the_way_they_mean() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("skin-surface", &repository_library()).expect("the stage");
    let plain = realistic();
    let shots = |description: &Description| {
        capture(&device, &queue, &stage.assets, description, settings()).unwrap()
    };
    let (width, _) = settings().size;
    let width = width as usize;
    let base = shots(&plain);
    let before = studio(&base, "face-front");

    // Oilier skin (roughness down) has brighter highlights; drier, dimmer.
    let oily = shots(&with(&plain, "Skin", &[("roughness_scale", number(0.5))]));
    let oily = studio(&oily, "face-front");
    let shine = change(before, oily);
    assert!(shine.share > 0.02, "roughness 0.5 changed {}", shine.share);
    assert!(
        highlights(oily, &shine.pixels) > highlights(before, &shine.pixels) * 1.05,
        "roughness 0.5 didn't brighten the highlights"
    );

    // Detail: a flat normal map smooths the skin, a doubled one roughens it.
    let smooth = shots(&with(&plain, "Skin", &[("detail_strength", number(0.0))]));
    let smooth = studio(&smooth, "face-front");
    let flattened = change(before, smooth);
    assert!(
        flattened.share > 0.02,
        "detail 0 changed {}",
        flattened.share
    );
    let grain = texture(before, width, &flattened.pixels);
    assert!(
        texture(smooth, width, &flattened.pixels) < grain,
        "detail 0 isn't smoother"
    );
    let rough = shots(&with(&plain, "Skin", &[("detail_strength", number(2.0))]));
    let rough = studio(&rough, "face-front");
    let sharpened = change(before, rough);
    assert!(
        texture(rough, width, &sharpened.pixels) > texture(before, width, &sharpened.pixels),
        "detail 2 isn't rougher"
    );

    // A pink sclera tint makes the whites of the eyes pinker.
    let pink = shots(&with(
        &plain,
        "Sclera",
        &[("sclera_tint", colour("#ff9090"))],
    ));
    let whites = change(before, studio(&pink, "face-front"));
    assert!(
        whites.share > 0.0005,
        "the sclera tint changed {}",
        whites.share
    );
    assert!(
        whites.after[0] / whites.after[1] > whites.before[0] / whites.before[1] * 1.1,
        "the sclera tint isn't pinker: {:?} to {:?}",
        whites.before,
        whites.after
    );
}

/// The GPU time of the realistic character's frame by pass (the renderer's
/// `FramePass` timers), at 1920 × 1080, High with TAA, under the hard
/// light, in the capture's front and face cameras: the measure for
/// changes to skin shading and character shadows.
///
/// ```text
/// cargo test -p axcenity-player --release -- --ignored bench_realistic_character --nocapture
/// ```
#[test]
#[ignore = "benchmark: run with --release -- --ignored bench_realistic_character"]
fn bench_realistic_character() {
    use axcenity_render::glam::Vec3;
    use axcenity_render::{
        AntiAliasing, Camera, FramePass, GraphicsSettings, Quality, TextureRenderer,
    };

    let (device, queue) = super::bench_tests::bench_gpu().expect("no graphics adapter");
    let stage = Stage::new("bench-skin", &repository_library()).expect("the stage");
    let (mut world, animators) = super::character_capture::stage_world(
        &stage.assets,
        &realistic(),
        super::character_capture::Stance::Rest,
        &[],
    )
    .unwrap();
    let lighting = world.lighting();
    let _ = world.set_property(lighting, "ClockTime", axcenity_core::Value::Number(16.7));
    let mut graphics = GraphicsSettings::preset(Quality::High);
    graphics.anti_aliasing = Some(AntiAliasing::Taa);
    for (name, eye, target, fov) in [
        ("front", [0.0, 2.6, -9.5], [0.0, 2.6, 0.0], 35.0),
        ("face", [-0.35, 4.62, -2.3], [0.0, 4.58, 0.0], 30.0),
    ] {
        let mut renderer = TextureRenderer::new(&device, &queue, 1920, 1080);
        renderer.set_graphics(&graphics);
        let mut cache = axcenity_scene::TerrainCache::new();
        let camera = Camera {
            position: Vec3::from(eye),
            look_at: Vec3::from(target),
            fov_y_degrees: fov,
        };
        let mut times: Vec<Vec<f64>> = vec![Vec::new(); FramePass::COUNT + 1];
        for frame in 0..300 {
            let list = axcenity_scene::build_draw_list_posed(
                &world,
                &stage.assets,
                camera,
                animators.poses(),
                &mut cache,
            );
            renderer.render(&list).unwrap();
            device
                .poll(axcenity_render::wgpu::PollType::wait_indefinitely())
                .unwrap();
            if frame < 60 {
                continue;
            }
            let stats = renderer.stats();
            for (slot, pass) in FramePass::ALL.iter().enumerate() {
                if let Some(time) = stats.pass_time(*pass) {
                    times[slot].push(time.as_secs_f64() * 1000.0);
                }
            }
            if let Some(time) = stats.gpu_time {
                times[FramePass::COUNT].push(time.as_secs_f64() * 1000.0);
            }
        }
        let median = |values: &mut Vec<f64>| {
            values.sort_by(f64::total_cmp);
            values.get(values.len() / 2).copied()
        };
        println!("bench_realistic_character {name} (1920 × 1080, High, TAA, hard light)");
        if let Some(frame) = median(&mut times[FramePass::COUNT]) {
            println!("  GPU frame                      {frame:8.3} ms");
        }
        for (slot, pass) in FramePass::ALL.iter().enumerate() {
            if let Some(time) = median(&mut times[slot]) {
                println!("  GPU   {:<24} {time:8.3} ms", pass.label());
            }
        }
    }
}
