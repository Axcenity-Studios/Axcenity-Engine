//! Color grading: brightness, contrast, saturation, tint, split toning, the
//! S-curve, a lookup table, posterizing and a palette each change the
//! finished frame as they say, a neutral table and the new controls at
//! their defaults change nothing, and the table's texture lives only while
//! one is used.

use super::*;
use crate::graphics::{AntiAliasing, Quality};
use crate::{ColorGrading, ColorLookup, GraphicsSettings, Shading};

const SIZE: u32 = 64;

fn renderer(device: &wgpu::Device, queue: &wgpu::Queue, aa: AntiAliasing) -> TextureRenderer {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(aa),
        ..GraphicsSettings::preset(Quality::High)
    });
    renderer
}

/// The middle pixel of an unlit card of `color` (sRGB) filling the view,
/// with post-processing off, so with no grading it shows `color` itself.
fn shown(renderer: &mut TextureRenderer, color: [f32; 3], grading: ColorGrading) -> [u8; 3] {
    let mut card = big_block();
    card.material = Material {
        base_color: color,
        shading: Shading::Unlit,
        ..Material::default()
    };
    let mut list = list_of(vec![card]);
    list.grading = Some(grading);
    renderer.render(&list).unwrap();
    let pixels = renderer.read_pixels();
    let at = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

#[track_caller]
fn near(drawn: [u8; 3], expected: [u8; 3], what: &str) {
    for (channel, wanted) in drawn.iter().zip(expected) {
        assert!(
            channel.abs_diff(wanted) <= 2,
            "{what}: {drawn:?}, not {expected:?}"
        );
    }
}

/// A table `size` entries a side whose entries are `map` of where they are.
fn table(key: u64, size: u32, map: impl Fn([f32; 3]) -> [f32; 3]) -> ColorLookup {
    let step = |i: u32| i as f32 / (size - 1) as f32;
    let mut rgb = Vec::new();
    for blue in 0..size {
        for green in 0..size {
            for red in 0..size {
                rgb.extend(map([step(red), step(green), step(blue)]));
            }
        }
    }
    ColorLookup {
        key,
        size,
        rgb: Arc::new(rgb),
    }
}

#[test]
fn each_control_changes_the_frame_as_it_says() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue, AntiAliasing::Off);
    let mut shown = |color, grading| shown(&mut renderer, color, grading);
    let none = ColorGrading::default();
    near(shown([0.5; 3], none.clone()), [128; 3], "the default");

    let brightness = |brightness| ColorGrading {
        brightness,
        ..ColorGrading::default()
    };
    near(shown([0.5; 3], brightness(0.2)), [179; 3], "brighter");
    near(shown([0.5; 3], brightness(-1.0)), [0; 3], "brightness -1");

    let contrast = |contrast| ColorGrading {
        contrast,
        ..ColorGrading::default()
    };
    near(
        shown([0.25; 3], contrast(0.5)),
        [32; 3],
        "dark, more contrast",
    );
    near(
        shown([0.75; 3], contrast(0.5)),
        [223; 3],
        "light, more contrast",
    );
    near(shown([0.25; 3], contrast(-1.0)), [128; 3], "contrast -1");

    let saturation = |saturation| ColorGrading {
        saturation,
        ..ColorGrading::default()
    };
    // Grey at the color's brightness (Rec. 709 weights): 0.342.
    near(
        shown([0.8, 0.2, 0.4], saturation(-1.0)),
        [87; 3],
        "saturation -1",
    );
    // Twice as far from that grey (0.450).
    near(
        shown([0.6, 0.4, 0.5], saturation(1.0)),
        [191, 89, 140],
        "saturation 1",
    );

    let tint = ColorGrading {
        tint: [1.0, 0.5, 0.0],
        ..ColorGrading::default()
    };
    near(shown([1.0; 3], tint), [255, 128, 0], "tinted orange");

    let inverted = ColorGrading {
        lookup: Some(table(1, 2, |c| c.map(|c| 1.0 - c))),
        ..ColorGrading::default()
    };
    near(
        shown([0.2, 0.6, 0.9], inverted),
        [204, 102, 26],
        "an inverting table",
    );

    let posterize = |posterize| ColorGrading {
        posterize,
        ..ColorGrading::default()
    };
    near(shown([0.3; 3], posterize(2)), [0; 3], "two levels, dark");
    near(shown([0.7; 3], posterize(2)), [255; 3], "two levels, light");
    near(shown([0.3; 3], posterize(4)), [85; 3], "four levels");
    near(shown([0.3; 3], posterize(1)), [77; 3], "one level is off");

    let palette = ColorGrading {
        palette: vec![[0.0; 3], [1.0; 3], [1.0, 0.0, 0.0]],
        ..ColorGrading::default()
    };
    near(
        shown([0.9, 0.3, 0.2], palette.clone()),
        [255, 0, 0],
        "nearest is red",
    );
    near(
        shown([0.2, 0.2, 0.25], palette.clone()),
        [0; 3],
        "nearest is black",
    );
    near(shown([0.8; 3], palette), [255; 3], "nearest is white");
}

#[test]
fn the_curve_deepens_shadows_and_lifts_highlights_about_its_pivot_without_clipping() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue, AntiAliasing::Off);
    let mut shown = |grey: f32, curve, curve_pivot| {
        shown(
            &mut renderer,
            [grey; 3],
            ColorGrading {
                curve,
                curve_pivot,
                ..ColorGrading::default()
            },
        )
    };
    // A power of 1.5 each side of 0.5: 0.5 × 0.5^1.5 = 0.177 below it, its
    // mirror above.
    near(shown(0.25, 0.5, 0.5), [45; 3], "a shadow, deeper");
    near(shown(0.75, 0.5, 0.5), [210; 3], "a highlight, lifted");
    near(shown(0.5, 0.5, 0.5), [128; 3], "the pivot stays");
    // It eases into black and white: nothing near them is clipped.
    near(shown(0.05, 0.5, 0.5), [4; 3], "near black");
    near(shown(0.95, 0.5, 0.5), [251; 3], "near white");
    near(shown(0.0, 0.5, 0.5), [0; 3], "black");
    near(shown(1.0, 0.5, 0.5), [255; 3], "white");
    // Below 0 it flattens toward the pivot (a power of 2/3).
    near(shown(0.25, -0.5, 0.5), [80; 3], "a shadow, lifted");
    // About a lower pivot, mid-grey is a highlight: 1 − 0.7 × (0.5 / 0.7)^1.5.
    near(shown(0.3, 0.5, 0.3), [77; 3], "a lower pivot stays");
    near(shown(0.5, 0.5, 0.3), [147; 3], "mid-grey above it, lifted");
}

#[test]
fn split_toning_tints_shadows_and_highlights_apart_keeping_their_brightness() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue, AntiAliasing::Off);
    let mut shown = |grey: f32, grading| shown(&mut renderer, [grey; 3], grading);
    let cool = |balance| ColorGrading {
        shadow_tint: [0.8, 0.9, 1.0],
        balance,
        ..ColorGrading::default()
    };
    // The tint scaled to white's brightness (0.903, 1.016, 1.129), mostly
    // (0.896) on a dark grey of 0.2, which keeps its brightness.
    near(shown(0.2, cool(0.0)), [47, 52, 57], "cool shadows");
    near(
        shown(0.9, cool(0.0)),
        [229, 230, 230],
        "light colors barely",
    );
    let warm = ColorGrading {
        highlight_tint: [1.0, 0.95, 0.85],
        ..ColorGrading::default()
    };
    near(shown(0.9, warm.clone()), [240, 229, 205], "warm highlights");
    near(shown(0.2, warm), [51, 51, 50], "dark colors barely");
    // The balance moves where shadows end: above 0, the dark grey is more a
    // highlight and so less cooled; below, all shadow.
    let spread = |[r, _, b]: [u8; 3]| i16::from(b) - i16::from(r);
    let (less, even, more) = (
        spread(shown(0.2, cool(1.0))),
        spread(shown(0.2, cool(0.0))),
        spread(shown(0.2, cool(-1.0))),
    );
    assert!(
        less + 5 < even && even <= more,
        "balance 1, 0, -1: {less} {even} {more}"
    );
}

#[test]
fn the_new_controls_at_their_defaults_give_back_the_picture_exactly() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    for aa in [AntiAliasing::Off, AntiAliasing::Msaa4] {
        let mut renderer = renderer(&device, &queue, aa);
        let mut list = colorful();
        list.grading = Some(ColorGrading::default());
        renderer.render(&list).unwrap();
        let plain = renderer.read_pixels();
        // White tints and no curve change nothing, whatever the balance and
        // pivot say.
        list.grading = Some(ColorGrading {
            shadow_tint: [1.0; 3],
            highlight_tint: [1.0; 3],
            balance: 0.8,
            curve: 0.0,
            curve_pivot: 0.3,
            ..ColorGrading::default()
        });
        renderer.render(&list).unwrap();
        assert!(
            renderer.read_pixels() == plain,
            "neutral split toning and curve changed the frame ({aa:?})"
        );
        // And grading the default is as near as 8 bits allow to none.
        list.grading = None;
        renderer.render(&list).unwrap();
        let ungraded = renderer.read_pixels();
        let most = plain
            .iter()
            .zip(&ungraded)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(most <= 1, "default grading changed a channel by {most}");
    }
}

/// A frame with many colors: lit blocks and spheres in front of the sky,
/// with bloom and tone mapping on.
fn colorful() -> DrawList {
    let colors = [
        [0.9, 0.2, 0.1],
        [0.1, 0.8, 0.3],
        [0.2, 0.3, 0.95],
        [0.95, 0.9, 0.2],
        [0.5; 3],
    ];
    let items = colors
        .iter()
        .enumerate()
        .map(|(i, &color)| {
            let shape = if i % 2 == 0 {
                Shape::Block
            } else {
                Shape::Sphere
            };
            let mut item = item(shape, Vec3::new(i as f32 * 1.4 - 2.8, 0.0, -6.0), 1.0);
            item.material.base_color = color;
            item.material.emissive = if i == 3 { [2.0; 3] } else { [0.0; 3] };
            item
        })
        .collect();
    let mut list = list_of(items);
    list.sun.direction = Vec3::new(0.4, 0.8, 0.5).normalize();
    list.sun.intensity = 2.0;
    list.ambient.intensity = 0.3;
    list.post = PostProcessing {
        tone_mapping: crate::ToneMapping::Neutral,
        ..PostProcessing::default()
    };
    list
}

#[test]
fn a_neutral_lookup_table_changes_nothing() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    for aa in [AntiAliasing::Off, AntiAliasing::Fxaa, AntiAliasing::Msaa4] {
        let mut renderer = renderer(&device, &queue, aa);
        let mut list = colorful();
        renderer.render(&list).unwrap();
        let plain = renderer.read_pixels();
        let distinct: HashSet<&[u8; 4]> = plain.as_chunks::<4>().0.iter().collect();
        assert!(distinct.len() > 200, "a colorful frame: {}", distinct.len());

        for (key, size) in [(10, 2), (11, 17), (12, 33)] {
            for grading in [
                ColorGrading::default(),
                ColorGrading {
                    lookup: Some(table(key, size, |c| c)),
                    ..ColorGrading::default()
                },
            ] {
                list.grading = Some(grading);
                renderer.render(&list).unwrap();
                let graded = renderer.read_pixels();
                let most = plain
                    .iter()
                    .zip(&graded)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert!(
                    most <= 1,
                    "a neutral table of {size} ({aa:?}) changed a channel by {most}"
                );
            }
        }
    }
}

#[test]
fn a_lookup_tables_texture_lives_only_while_one_is_used() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue, AntiAliasing::Off);
    let inverted = || ColorGrading {
        lookup: Some(table(20, 2, |c| c.map(|c| 1.0 - c))),
        ..ColorGrading::default()
    };
    assert!(!renderer.scene.post.grading.has_lookup());
    near(
        shown(&mut renderer, [0.2; 3], inverted()),
        [204; 3],
        "inverted",
    );
    assert!(renderer.scene.post.grading.has_lookup());
    // Taken away, the table goes and the frame is back as it was.
    near(
        shown(&mut renderer, [0.2; 3], ColorGrading::default()),
        [51; 3],
        "plain",
    );
    assert!(!renderer.scene.post.grading.has_lookup());
    // A broken one (the wrong number of entries) is left out.
    let broken = ColorGrading {
        lookup: Some(ColorLookup {
            key: 21,
            size: 4,
            rgb: Arc::new(vec![0.0; 3]),
        }),
        ..ColorGrading::default()
    };
    near(shown(&mut renderer, [0.2; 3], broken), [51; 3], "broken");
    assert!(!renderer.scene.post.grading.has_lookup());
}

#[test]
fn a_shade_between_two_is_drawn_as_a_mix_of_them_and_black_stays_black() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue, AntiAliasing::Off);
    // Nothing but sky, one flat color all over.
    let shades = |renderer: &mut TextureRenderer, grey: f32, post: bool| {
        let mut list = list_of(Vec::new());
        list.sky.top_color = [grey; 3];
        list.sky.horizon_color = [grey; 3];
        list.sky.ground_color = [grey; 3];
        list.post.enabled = post;
        renderer.render(&list).unwrap();
        let pixels = renderer.read_pixels();
        let distinct: HashSet<u8> = pixels.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        let mut distinct: Vec<u8> = distinct.into_iter().collect();
        distinct.sort_unstable();
        distinct
    };
    // Somewhere in a run of greys closer together than 8 bits can tell
    // apart, one falls between two shades: tone mapped, it's drawn in both
    // (which average to it), where the plain frame rounds it to one.
    let mut mixed = 0;
    for step in 0..12 {
        let grey = 0.3 + step as f32 * 0.0005;
        let plain = shades(&mut renderer, grey, false);
        assert_eq!(
            plain.len(),
            1,
            "post-processing off adds nothing: {plain:?}"
        );
        let dithered = shades(&mut renderer, grey, true);
        assert!(
            dithered.len() <= 3 && dithered[dithered.len() - 1] - dithered[0] <= 2,
            "under a shade of noise: {dithered:?}"
        );
        mixed += usize::from(dithered.len() >= 2);
    }
    assert!(mixed >= 6, "{mixed} of 12 greys were drawn in two shades");
    assert_eq!(shades(&mut renderer, 0.0, true), [0], "black stays black");
}
