//! Makeup and markings on the realistic character, drawn by the player's
//! own renderer (Character Studio R5, ADR-0042): each layer the base
//! offers changes the pixels of a capture against the bare skin, where it
//! is meant to and not elsewhere; a layer turned off draws the skin as
//! made; a colour change re-applies live, with no bake; and `softness` on
//! its own hides the definition the body drives. Skipped on machines
//! without a GPU.

use std::time::Instant;

use axcenity_avatar::{Colour, Description, MaterialValue, SkinLayer};

use super::character_capture::{
    Settings, Stage, Stance, capture_in, lights_named, repository_library, stage_world, views_named,
};

fn settings() -> Settings {
    Settings {
        size: (480, 270),
        frames: 8,
        ..Settings::default()
    }
}

fn character(file: &str) -> Description {
    let path = repository_library().join(format!("benchmarks/{file}.character.json"));
    Description::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The pixels of `description` in each of `views` under the studio light.
fn shots(stage: &Stage, description: &Description, views: &str) -> Vec<Vec<u8>> {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return Vec::new();
    };
    let lights = lights_named("studio").unwrap();
    let views = views_named(views).unwrap();
    capture_in(
        &device,
        &queue,
        &stage.assets,
        description,
        settings(),
        &lights,
        &views,
    )
    .unwrap()
    .into_iter()
    .map(|shot| shot.pixels)
    .collect()
}

/// The share of pixels any channel of which moved by more than `by`.
fn changed(before: &[u8], after: &[u8], by: u8) -> f32 {
    let count = before.len() / 4;
    (0..count)
        .filter(|&i| (0..3).any(|c| before[i * 4 + c].abs_diff(after[i * 4 + c]) > by))
        .count() as f32
        / count as f32
}

/// The mean of each channel over the pixels that moved by more than 2.
fn moved_means(before: &[u8], after: &[u8]) -> ([f32; 3], [f32; 3]) {
    let mut sums = ([0.0; 3], [0.0; 3]);
    let mut count = 0.0;
    for i in 0..before.len() / 4 {
        if (0..3).any(|c| before[i * 4 + c].abs_diff(after[i * 4 + c]) > 2) {
            for c in 0..3 {
                sums.0[c] += f32::from(before[i * 4 + c]);
                sums.1[c] += f32::from(after[i * 4 + c]);
            }
            count += 1.0;
        }
    }
    let mean = |sum: [f32; 3]| sum.map(|s| s / f32::max(count, 1.0));
    (mean(sums.0), mean(sums.1))
}

fn with_layer(description: &Description, layer: SkinLayer, marking: bool) -> Description {
    let mut changed = description.clone();
    if marking {
        changed.skin.markings.push(layer);
    } else {
        changed.skin.makeup.push(layer);
    }
    changed
}

/// One layer against bare skin: at full opacity in a colour far from skin
/// (makeup) or at its full amount (a marking), the face-front capture
/// changes by at least `least` of the picture and at most `most` (the
/// layer stays where it belongs: lips don't paint the face), the way the
/// layer means; turned off, or at opacity 0, it draws the bare picture
/// exactly. One test per layer (R5's "a test per value or layer").
fn layer_changes_the_face(id: &str, marking: bool, least: f32, most: f32) {
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new(&format!("makeup-{id}"), &repository_library()).expect("the stage");
    let bare = character("realistic-feminine-cast-average");
    let before = shots(&stage, &bare, "face-front");
    let mut layer = SkinLayer::new(id);
    if marking {
        layer.amount = Some(1.0);
    } else {
        layer.colour = Some(Colour([40, 90, 200]));
    }
    let after = shots(
        &stage,
        &with_layer(&bare, layer.clone(), marking),
        "face-front",
    );
    let share = changed(&before[0], &after[0], 4);
    let (was, now) = moved_means(&before[0], &after[0]);
    eprintln!("{id}: {share:.4} of the picture; mean {was:?} -> {now:?}");
    assert!(
        share > least && share < most,
        "{id} changed {share} of the face-front picture ({least} to {most})"
    );
    if !marking {
        // Blue makeup makes what it covers bluer.
        assert!(
            now[2] - now[0] > was[2] - was[0],
            "{id}: {was:?} -> {now:?}"
        );
    } else if id != "scar" {
        // Spots are darker; a scar is paler.
        assert!(now.iter().sum::<f32>() < was.iter().sum::<f32>(), "{id}");
    } else {
        assert!(now.iter().sum::<f32>() > was.iter().sum::<f32>(), "{id}");
    }
    let mut off = layer.clone();
    off.enabled = false;
    let mut clear = layer;
    clear.opacity = 0.0;
    for quiet in [off, clear] {
        let drawn = shots(&stage, &with_layer(&bare, quiet, marking), "face-front");
        assert!(drawn[0] == before[0], "{id} turned off changed the picture");
    }
}

#[test]
fn foundation_changes_the_face_against_bare_skin() {
    layer_changes_the_face("foundation", false, 0.02, 0.6);
}

#[test]
fn blush_changes_the_face_against_bare_skin() {
    layer_changes_the_face("blush", false, 0.005, 0.2);
}

#[test]
fn eyeshadow_changes_the_face_against_bare_skin() {
    layer_changes_the_face("eyeshadow", false, 0.001, 0.05);
}

#[test]
fn eyeliner_changes_the_face_against_bare_skin() {
    layer_changes_the_face("eyeliner", false, 0.0001, 0.03);
}

#[test]
fn lip_colour_changes_the_face_against_bare_skin() {
    layer_changes_the_face("lips", false, 0.002, 0.05);
}

#[test]
fn brow_fill_changes_the_face_against_bare_skin() {
    layer_changes_the_face("brow_fill", false, 0.0003, 0.03);
}

#[test]
fn freckles_change_the_face_against_bare_skin() {
    layer_changes_the_face("freckles", true, 0.0002, 0.2);
}

#[test]
fn moles_change_the_face_against_bare_skin() {
    // (Since the review the face's moles keep off its middle, where one
    // read as a bindi: about a dozen pixels of this small picture.)
    layer_changes_the_face("moles", true, 0.00004, 0.02);
}

#[test]
fn the_scar_changes_the_face_against_bare_skin() {
    layer_changes_the_face("scar", true, 0.0001, 0.02);
}

/// A layer's roughness changes its highlights: glossy lips against matte
/// ones, both in the skin's own colour so only the roughness differs.
#[test]
fn a_makeup_layers_roughness_changes_the_lips_highlight() {
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new("makeup-roughness", &repository_library()).expect("the stage");
    let bare = character("realistic-feminine-cast-average");
    let lips = |roughness: f32| {
        let mut layer = SkinLayer::new("lips");
        layer.colour = Some(Colour([168, 67, 75]));
        layer.roughness = Some(roughness);
        shots(&stage, &with_layer(&bare, layer, false), "face-front")
    };
    let (glossy, matte) = (lips(0.08), lips(0.9));
    let share = changed(&matte[0], &glossy[0], 2);
    let brightest = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]))
            .max()
            .unwrap_or(0)
    };
    let (was, now) = moved_means(&matte[0], &glossy[0]);
    eprintln!("glossy against matte lips: {share:.4} changed, mean {was:?} -> {now:?}");
    assert!(share > 0.0005, "roughness changed {share} of the picture");
    assert!(brightest(&glossy[0]) >= brightest(&matte[0]));
}

/// `softness` alone (wave 2 tested it only with `definition`): on the
/// athletic body, whose muscle drives `definition` up, softness 1 hides
/// the defined normal map and the belly's relief goes; the front capture
/// changes and is smoother there.
#[test]
fn softness_alone_hides_the_definition_the_body_drives() {
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new("skin-softness", &repository_library()).expect("the stage");
    let athletic = character("realistic-masculine-cast-athletic");
    let mut soft = athletic.clone();
    soft.materials
        .entry("Skin".to_owned())
        .or_default()
        .insert("softness".to_owned(), MaterialValue::Number(1.0));
    let before = shots(&stage, &athletic, "front");
    let after = shots(&stage, &soft, "front");
    let share = changed(&before[0], &after[0], 2);
    // Relief: the mean luminance step between neighbours over the pixels
    // that changed.
    let (width, _) = settings().size;
    let width = width as usize;
    let relief = |pixels: &[u8]| {
        let lum = |i: usize| {
            0.2126 * f32::from(pixels[i * 4])
                + 0.7152 * f32::from(pixels[i * 4 + 1])
                + 0.0722 * f32::from(pixels[i * 4 + 2])
        };
        let moved: Vec<usize> = (0..pixels.len() / 4 - width - 1)
            .filter(|&i| (0..3).any(|c| before[0][i * 4 + c].abs_diff(after[0][i * 4 + c]) > 2))
            .collect();
        moved
            .iter()
            .map(|&i| (lum(i) - lum(i + 1)).abs() + (lum(i) - lum(i + width)).abs())
            .sum::<f32>()
            / moved.len().max(1) as f32
    };
    let (was, now) = (relief(&before[0]), relief(&after[0]));
    eprintln!("softness 1 on the athletic body: {share:.4} changed; relief {was:.3} -> {now:.3}");
    assert!(share > 0.002, "softness changed {share} of the picture");
    assert!(now < was, "softness smooths the relief: {was} -> {now}");
}

/// A colour change re-applies live: the applier writes the new value as
/// the body's material value attribute, and the body keeps its base model
/// (nothing is baked, no texture is made). Timed (debug builds are slow;
/// the figure for the hand-back is release). (That the draw list draws
/// from those attributes is scene's test,
/// `skin_layers_draw_from_their_values_and_none_on_draws_no_layers`.)
#[test]
fn a_makeup_colour_change_re_applies_live_without_a_bake() {
    let stage = Stage::new("makeup-live", &repository_library()).expect("the stage");
    let mut description = character("realistic-feminine-cast-average");
    let mut blush = SkinLayer::new("blush");
    blush.colour = Some(Colour([196, 103, 95]));
    description.skin.makeup.push(blush);
    let (mut world, _) =
        stage_world(&stage.assets, &description, Stance::Rest, &[]).expect("the world");
    let character = world
        .get(world.workspace())
        .expect("the workspace")
        .children()
        .iter()
        .copied()
        .find(|&id| {
            world
                .get(id)
                .is_some_and(|i| i.class() == axcenity_core::ClassName::Model)
        })
        .expect("the character");
    let body = axcenity_gameplay::description::body_of(&world, character).unwrap();
    let mesh = |world: &axcenity_core::World| {
        world
            .get(body)
            .and_then(|i| i.property("MeshId"))
            .and_then(axcenity_core::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let before = mesh(&world);
    let colour = |world: &axcenity_core::World| {
        world
            .get(body)
            .and_then(|i| {
                i.attribute(&axcenity_core::material_value_attribute(
                    "Skin",
                    "layer_blush_colour",
                ))
            })
            .cloned()
    };
    let mut times = Vec::new();
    for step in 0..20u8 {
        description.skin.makeup[0].colour = Some(Colour([196, 103, 95 + step * 5]));
        let start = Instant::now();
        axcenity_gameplay::description::apply_description(
            &mut world,
            &stage.assets,
            character,
            &description,
        )
        .unwrap();
        times.push(start.elapsed());
    }
    times.sort();
    let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
    eprintln!(
        "{} build: a blush colour change re-applied in median {:.3} ms (min {:.3}, max {:.3})",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        ms(times[times.len() / 2]),
        ms(times[0]),
        ms(times[times.len() - 1])
    );
    assert_eq!(mesh(&world), before, "the body keeps its model");
    assert!(
        axcenity_assets::split_derived(&mesh(&world)).1.is_none(),
        "no derived (baked) model"
    );
    assert!(!axcenity_gameplay::description::is_baked(&world, character));
    let expected = axcenity_core::Value::Color3(axcenity_core::Color3::new(
        196.0 / 255.0,
        103.0 / 255.0,
        190.0 / 255.0,
    ));
    assert_eq!(
        colour(&world),
        Some(expected),
        "the last colour is on the body"
    );
    assert!(
        ms(times[times.len() / 2]) < 250.0,
        "a colour change took {} ms",
        ms(times[times.len() / 2])
    );
}

/// The Skin `tint` (the one Skin value without a capture test of its own
/// before this branch): a green tint makes the face greener.
#[test]
fn a_skin_tint_changes_the_face_the_way_it_means() {
    if super::bench_tests::bench_gpu().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let stage = Stage::new("skin-tint", &repository_library()).expect("the stage");
    let bare = character("realistic-feminine-cast-average");
    let mut tinted = bare.clone();
    tinted
        .materials
        .entry("Skin".to_owned())
        .or_default()
        .insert(
            "tint".to_owned(),
            MaterialValue::Colour(Colour::parse("#b0ffb0").unwrap()),
        );
    let (before, after) = (
        shots(&stage, &bare, "face-front"),
        shots(&stage, &tinted, "face-front"),
    );
    let share = changed(&before[0], &after[0], 4);
    let (was, now) = moved_means(&before[0], &after[0]);
    eprintln!("tint: {share:.4} of the picture; mean {was:?} -> {now:?}");
    assert!(share > 0.05, "a tint changed {share} of the picture");
    assert!(
        now[1] / now[0] > was[1] / was[0] * 1.1,
        "{was:?} -> {now:?}"
    );
}

/// Close up, the cheek reads as pores (R5 review: the grain without its
/// crackle read as smooth, waxy skin with a few pale specks). In the
/// `skin-close` view (about 0.19 mm a pixel on the cheek) under the hard
/// light, with temporal anti-aliasing, a crop of the cheek below the eye
/// has many small dark pits: pixels darker than their eight neighbours and
/// than their 7 × 7 neighbourhood by more than 2 levels of luminance. Real
/// cheek skin has tens to hundreds of pores to a square centimetre.
#[test]
fn the_cheek_close_up_shows_pores() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let stage = Stage::new("skin-pores", &repository_library()).expect("the stage");
    let bare = character("realistic-feminine-cast-average");
    let settings = Settings {
        frames: 8,
        ..Settings::default()
    };
    let (width, _) = settings.size;
    let shot = capture_in(
        &device,
        &queue,
        &stage.assets,
        &bare,
        settings,
        &lights_named("hard").unwrap(),
        &views_named("skin-close").unwrap(),
    )
    .unwrap()
    .remove(0);
    let width = width as usize;
    let lum = |x: usize, y: usize| {
        let p = &shot.pixels[(y * width + x) * 4..][..3];
        0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])
    };
    // The cheek below the eye's outer corner and in front of the ear.
    let (xs, ys) = (700..980, 440..660);
    let mut pits = 0;
    for y in ys.clone() {
        for x in xs.clone() {
            let v = lum(x, y);
            let mut sum = 0.0;
            let mut lowest = true;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let w = lum((x as i32 + dx) as usize, (y as i32 + dy) as usize);
                    sum += w;
                    if dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0) && w <= v {
                        lowest = false;
                    }
                }
            }
            if lowest && v < sum / 49.0 - 2.0 {
                pits += 1;
            }
        }
    }
    let square_cm = (xs.len() * ys.len()) as f32 * 0.019 * 0.019;
    let per_cm2 = pits as f32 / square_cm;
    eprintln!(
        "the cheek close up: {pits} pits in {square_cm:.1} cm2, {per_cm2:.0} a square centimetre"
    );
    assert!(
        (25.0..=600.0).contains(&per_cm2),
        "{per_cm2} pits a square centimetre on the cheek"
    );
}
