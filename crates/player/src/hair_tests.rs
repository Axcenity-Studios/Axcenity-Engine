//! The realistic family's first hairstyle in the player's own renderer
//! (Character Studio R6, CO11): it draws on both frames, over the head
//! and not elsewhere, and follows a head parameter (the head's width), so
//! a wider head wears wider hair. Skipped on machines without a GPU.

use axcenity_avatar::{Description, WornItem};

use super::character_capture::{
    Settings, Stage, capture_in, lights_named, repository_library, views_named,
};

const STYLE: &str = "axcenity/realistic-hair-short-crop";
/// The style made from scratch against references (wave 5, decision 14).
const CROP_TAPER: &str = "axcenity/realistic-hair-crop-taper";

fn character(file: &str) -> Description {
    let path = repository_library().join(format!("benchmarks/{file}.character.json"));
    Description::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The style the ignored measurements and the benchmark wear:
/// `AXCENITY_HAIR_STYLE` (an item id), or the first style.
fn measured_style() -> String {
    std::env::var("AXCENITY_HAIR_STYLE").unwrap_or_else(|_| STYLE.to_owned())
}

fn wearing(description: &Description, style: &str) -> Description {
    let mut dressed = description.clone();
    dressed.items.retain(|item| !item.id.contains("hair"));
    let mut hair = WornItem::new(style);
    hair.variant = Some("brown".to_owned());
    dressed.items.push(hair);
    dressed
}

/// The pictures of `description` in `views` under the studio light, and
/// the level of detail the hair was drawn at in each.
fn shots(
    stage: &Stage,
    description: &Description,
    views: &str,
    size: (u32, u32),
) -> Vec<(Vec<u8>, Option<usize>)> {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return Vec::new();
    };
    let settings = Settings {
        size,
        frames: 8,
        ..Settings::default()
    };
    capture_in(
        &device,
        &queue,
        &stage.assets,
        description,
        settings,
        &lights_named("studio").unwrap(),
        &views_named(views).unwrap(),
    )
    .unwrap()
    .into_iter()
    .map(|shot| {
        let level = shot
            .levels
            .iter()
            .find(|(name, _)| name.starts_with("Hair_"))
            .map(|(_, level)| *level);
        (shot.pixels, level)
    })
    .collect()
}

/// Which pixels differ by more than `by` in any channel.
fn differ(before: &[u8], after: &[u8], by: u8) -> Vec<bool> {
    before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .map(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > by))
        .collect()
}

fn share(mask: &[bool]) -> f32 {
    mask.iter().filter(|&&m| m).count() as f32 / mask.len() as f32
}

/// The rows a mask has any pixel in, top and bottom.
fn rows(mask: &[bool], width: usize) -> Option<(usize, usize)> {
    let lit: Vec<usize> = mask
        .chunks_exact(width)
        .enumerate()
        .filter(|(_, row)| row.iter().any(|&m| m))
        .map(|(y, _)| y)
        .collect();
    Some((*lit.first()?, *lit.last()?))
}

/// The style is drawn on both frames: from the back and from the front it
/// changes the head (a good share of the picture), only above the
/// shoulders, and its cards are drawn (a level of detail is reported).
#[test]
fn the_first_hairstyle_draws_on_both_frames() {
    draws_on_both_frames(STYLE, "hair-frames");
}

/// The crop made from scratch draws on both frames, as the first style.
#[test]
fn the_crop_taper_draws_on_both_frames() {
    draws_on_both_frames(CROP_TAPER, "hair-frames-crop-taper");
}

fn draws_on_both_frames(style: &str, name: &str) {
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new(name, &repository_library()).expect("the stage");
    let size = (480, 270);
    for file in [
        "realistic-masculine-cast-average",
        "realistic-feminine-cast-average",
    ] {
        let bald = character(file);
        let before = shots(&stage, &bald, "hair-back,face-front", size);
        let after = shots(&stage, &wearing(&bald, style), "hair-back,face-front", size);
        for (index, view) in ["hair-back", "face-front"].iter().enumerate() {
            let mask = differ(&before[index].0, &after[index].0, 12);
            let changed = share(&mask);
            let (top, bottom) = rows(&mask, size.0 as usize).unwrap_or((0, 0));
            eprintln!(
                "{file} {view}: {changed:.4} of the picture, rows {top} to {bottom}, drawn at level {:?}",
                after[index].1
            );
            assert!(after[index].1.is_some(), "{file}: the hair is drawn");
            // From the back the hair covers most of the head; from the
            // front it shows over the forehead and at the sides.
            let least = if index == 0 { 0.06 } else { 0.03 };
            assert!(
                changed > least,
                "{file} {view}: the hair changed {changed} of the picture"
            );
            // Below the head (the bottom fifth of these crops: the neck
            // and shoulders) only the hair's shadow changes anything: a
            // small share, not hair hanging there.
            let width = size.0 as usize;
            let low = &mask[width * (size.1 as usize * 4 / 5)..];
            let below = share(low);
            eprintln!("{file} {view}: {below:.4} of the bottom fifth changed (to row {bottom})");
            assert!(
                below < 0.02,
                "{file} {view}: {below} of the bottom fifth changed"
            );
        }
    }
}

/// The hair's widest span, in pixels, from the back: the columns of the
/// rows a third of the way down the hair whose pixels differ from the
/// bald head's.
fn hair_width(mask: &[bool], width: usize) -> usize {
    let Some((top, bottom)) = rows(mask, width) else {
        return 0;
    };
    let band = top + (bottom - top) / 4..top + (bottom - top) / 2;
    band.map(|y| {
        let row = &mask[y * width..(y + 1) * width];
        match (row.iter().position(|&m| m), row.iter().rposition(|&m| m)) {
            (Some(left), Some(right)) => right - left + 1,
            _ => 0,
        }
    })
    .max()
    .unwrap_or(0)
}

/// A head parameter moves the hair with the head: at `temple_width` 1 the
/// hair seen from the back is wider than at -1, by about what the head
/// widens (`Face_Head_Width` moves the skin up to 5 mm a side: at 960 ×
/// 540 a pixel is about 0.8 mm there).
#[test]
fn the_first_hairstyle_follows_the_heads_width() {
    follows_the_heads_width(STYLE, "hair-follows");
}

/// The crop made from scratch follows the head's width too, by the
/// head's identity shapes it carries.
#[test]
fn the_crop_taper_follows_the_heads_width() {
    follows_the_heads_width(CROP_TAPER, "hair-follows-crop-taper");
}

fn follows_the_heads_width(style: &str, name: &str) {
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new(name, &repository_library()).expect("the stage");
    let size = (960, 540);
    let bald = character("realistic-masculine-cast-average");
    let mut widths = Vec::new();
    for temple in [-1.0, 1.0] {
        let mut head = bald.clone();
        head.parameters.insert("temple_width".to_owned(), temple);
        let before = shots(&stage, &head, "hair-back", size);
        let after = shots(&stage, &wearing(&head, style), "hair-back", size);
        let mask = differ(&before[0].0, &after[0].0, 12);
        widths.push(hair_width(&mask, size.0 as usize));
    }
    eprintln!(
        "hair width from the back: temple_width -1 {} px, 1 {} px",
        widths[0], widths[1]
    );
    assert!(widths[0] > 100, "the hair is drawn ({} px)", widths[0]);
    assert!(
        widths[1] >= widths[0] + 4,
        "the hair widens with the head: {} -> {} px",
        widths[0],
        widths[1]
    );
}

/// What the style costs the GPU (asset-quality.md §2, plan section 7): a
/// frame of `count` realistic characters, each bald and then each wearing
/// the style, idling, from `camera`. Ignored; run in a release build, one
/// at a time:
///
/// ```text
/// cargo test -p axcenity-player --release -- --ignored bench_hair --nocapture --test-threads 1
/// ```
fn hair_bench(count: usize, hair: bool, eye: [f32; 3], target: [f32; 3]) {
    use axcenity_core::Vector3;
    use axcenity_gameplay::description::{apply_description, default_description, library};
    use axcenity_render::glam::Vec3;
    use axcenity_scene::TerrainCache;

    use super::Animators;
    use super::bench_tests::{camera, run};

    let name = format!(
        "bench_hair: {count} realistic, {}",
        if hair { "short crop" } else { "bald" }
    );
    let stage = Stage::new(&format!("bench-hair-{count}-{hair}"), &repository_library())
        .expect("the stage");
    let shipped = library(&stage.assets);
    let mut description = default_description(&shipped, "axcenity/realistic");
    if hair {
        description = wearing(&description, &measured_style());
    }
    let mut world = axcenity_core::World::new();
    let workspace = world.workspace();
    let left = if count > 1 { 13.5 } else { 0.0 };
    for index in 0..count {
        let (column, row) = ((index % 10) as f32, (index / 10) as f32);
        let at = Vector3::new(column * 3.0 - left, 0.0, -row * 3.0);
        let character = axcenity_project::default_character(&mut world, at);
        world.set_parent(character, Some(workspace)).unwrap();
        apply_description(&mut world, &stage.assets, character, &description).expect("applied");
    }
    let mut animators = Animators::new();
    let camera = camera(Vec3::from(eye), Vec3::from(target));
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
    assert!(stage.assets.take_errors().is_empty());
}

#[test]
#[ignore = "a benchmark: run by hand in a release build"]
fn bench_hair() {
    // One character, its head filling the view (the hero copy), then
    // twenty at gameplay distance.
    for hair in [false, true] {
        hair_bench(1, hair, [0.0, 4.75, -1.6], [0.0, 4.7, 0.0]);
    }
    for hair in [false, true] {
        hair_bench(20, hair, [0.0, 9.0, 24.0], [0.0, 2.5, -6.0]);
    }
}

/// The style's temporal stability under TAA (asset-quality.md §4: "no
/// stair-stepping or shimmer under TAA", a limit Paul sets at the first
/// sign-off): the same view drawn after 40 to 48 frames, each picture
/// against the one a frame before, over the hair's pixels (those that
/// differ from the bald head); the root mean square of the per-pixel
/// deviation (of 255), and its 99th percentile. Also the scalp's
/// show-through: the hair drawn over a green and then a red skin, so a
/// pixel whose colour follows the skin's is skin seen through the hair,
/// as a share of the hair's pixels (front, side, back and top; the
/// hairline's ramp, a declared thinning, is the pixels within 12 of the
/// hair's edge, left out of the second figure). Ignored; run in a
/// release build:
///
/// ```text
/// cargo test -p axcenity-player --release -- --ignored measure_hair --nocapture --test-threads 1
/// ```
#[test]
#[ignore = "a measurement: run by hand in a release build"]
fn measure_hair_stability_and_show_through() {
    use axcenity_avatar::{Colour, MaterialValue};
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new("hair-measure", &repository_library()).expect("the stage");
    let size = (1920, 1080);
    let (device, queue) = super::bench_tests::bench_gpu().unwrap();
    let draw = |description: &Description, view: &str, frames: u32| {
        let settings = Settings {
            size,
            frames,
            ..Settings::default()
        };
        capture_in(
            &device,
            &queue,
            &stage.assets,
            description,
            settings,
            &lights_named("studio").unwrap(),
            &views_named(view).unwrap(),
        )
        .unwrap()
        .remove(0)
        .pixels
    };
    for file in [
        "realistic-masculine-cast-average",
        "realistic-feminine-cast-average",
    ] {
        let bald = character(file);
        let hair = wearing(&bald, &measured_style());
        for view in ["face-front", "hair-back"] {
            let mask = differ(&draw(&bald, view, 48), &draw(&hair, view, 48), 12);
            let pictures: Vec<Vec<u8>> =
                (40..=48).map(|frames| draw(&hair, view, frames)).collect();
            let mut deviations = Vec::new();
            for pair in pictures.windows(2) {
                for (i, &m) in mask.iter().enumerate() {
                    if m {
                        let d: f32 = (0..3)
                            .map(|c| {
                                let x =
                                    f32::from(pair[0][i * 4 + c]) - f32::from(pair[1][i * 4 + c]);
                                x * x
                            })
                            .sum::<f32>()
                            / 3.0;
                        deviations.push(d.sqrt());
                    }
                }
            }
            let rms =
                (deviations.iter().map(|d| d * d).sum::<f32>() / deviations.len() as f32).sqrt();
            deviations.sort_by(f32::total_cmp);
            let p99 = deviations[deviations.len() * 99 / 100];
            eprintln!(
                "TAA {file} {view}: {} hair pixels, rms frame-to-frame deviation {rms:.2} of 255, 99th percentile {p99:.1}",
                share(&mask) * (size.0 * size.1) as f32
            );
        }
        let tinted = |description: &Description, colour: [u8; 3]| {
            let mut tinted = description.clone();
            tinted
                .materials
                .entry("Skin".to_owned())
                .or_default()
                .insert("tint".to_owned(), MaterialValue::Colour(Colour(colour)));
            tinted
        };
        for view in ["face-front", "face-side", "hair-back", "hair-top"] {
            let bald_green = draw(&tinted(&bald, [0, 255, 0]), view, 48);
            let green = draw(&tinted(&hair, [0, 255, 0]), view, 48);
            let red = draw(&tinted(&hair, [255, 0, 0]), view, 48);
            // The hair: pixels it changes against the bald head, where the
            // bald head is skin (its green is strong) or the backdrop.
            let hair_mask = differ(&bald_green, &green, 12);
            let skin = differ(&green, &red, 24);
            let width = size.0 as usize;
            let near_edge = |i: usize| {
                let (x, y) = ((i % width) as i64, (i / width) as i64);
                (-12i64..=12).step_by(3).any(|dy| {
                    (-12i64..=12).step_by(3).any(|dx| {
                        let (nx, ny) = (x + dx, y + dy);
                        nx < 0
                            || ny < 0
                            || nx >= width as i64
                            || ny >= size.1 as i64
                            || !hair_mask[ny as usize * width + nx as usize]
                    })
                })
            };
            let (mut hair_px, mut through, mut core_px, mut core_through) =
                (0usize, 0usize, 0usize, 0usize);
            for i in 0..hair_mask.len() {
                if !hair_mask[i] {
                    continue;
                }
                hair_px += 1;
                through += usize::from(skin[i]);
                if !near_edge(i) {
                    core_px += 1;
                    core_through += usize::from(skin[i]);
                }
            }
            eprintln!(
                "show-through {file} {view}: {:.4} of {hair_px} hair pixels; {:.4} of {core_px} away from the hair's edge",
                through as f32 / hair_px.max(1) as f32,
                core_through as f32 / core_px.max(1) as f32
            );
        }
    }
}
