//! Hair shading (`Shading::Hair`, CO7): its binding and uniform; the
//! primary highlight following the strand's direction and lying toward the
//! root where the model puts it, the secondary toward the tip, tinted by
//! the hair's colour; light through hair from behind; and melanin,
//! redness, the root and tip gradient and streaks changing the pixels live,
//! with nothing rebuilt.

use glam::{Vec2, Vec3};

use super::card_set::{ball, card, dark_scene, flow, hair, hair_data, on_screen, rgb_at, sum};
use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};
use crate::{GraphicsSettings, HairShading, Shading};

/// The frame size, square.
const SIZE: u32 = 256;

fn renderer(device: &wgpu::Device, queue: &wgpu::Queue) -> TextureRenderer {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(AntiAliasing::Off),
        ambient_occlusion: Some(AmbientOcclusion::Off),
        ..GraphicsSettings::preset(Quality::High)
    });
    renderer
}

fn draw(renderer: &mut TextureRenderer, list: &DrawList) -> Vec<u8> {
    renderer.render(list).unwrap();
    renderer.read_pixels()
}

// -- The model, as hair.wgsl has it (Karis 2016), for where its lobes lie.

/// The cuticle's tilt, in radians (Blender's Principled Hair default, 2°).
const TILT: f32 = 0.0349;
/// The lobes' shifts, as multiples of the tilt: R toward the root, TT and
/// TRT toward the tip (d'Eon and others, 2011).
const SHIFTS: [f32; 3] = [-2.0, 1.0, 4.0];

fn gaussian(width: f32, x: f32) -> f32 {
    (-0.5 * x * x / (width * width)).exp() / ((2.0 * std::f32::consts::PI).sqrt() * width)
}

/// The R lobe's longitudinal part for strand direction `t` (toward the
/// root), light `l`, view `v`, `roughness` and `shift` (the tilt's scale).
fn r_lobe(t: Vec3, l: Vec3, v: Vec3, roughness: f32, shift: f32) -> f32 {
    let (sin_l, sin_v) = (t.dot(l), t.dot(v));
    let lp = l - sin_l * t;
    let vp = v - sin_v * t;
    let cos_phi = lp.dot(vp) / (lp.length_squared() * vp.length_squared() + 1e-4).sqrt();
    let cos_half_phi = (0.5 + 0.5 * cos_phi).clamp(0.0, 1.0).sqrt();
    let alpha = SHIFTS[0] * TILT * shift;
    let (sa, ca) = alpha.sin_cos();
    let offset =
        2.0 * sa * (ca * cos_half_phi * (1.0 - sin_v * sin_v).max(0.0).sqrt() + sa * sin_v);
    let b = roughness * roughness;
    gaussian(b * 2f32.sqrt() * cos_half_phi, sin_l + sin_v - offset) * 0.25 * cos_half_phi
}

/// The TRT lobe's longitudinal part, as [`r_lobe`].
fn trt_lobe(t: Vec3, l: Vec3, v: Vec3, roughness: f32, shift: f32) -> f32 {
    let (sin_l, sin_v) = (t.dot(l), t.dot(v));
    let b = roughness * roughness;
    gaussian(2.0 * b, sin_l + sin_v - SHIFTS[2] * TILT * shift)
}

/// The latitude (radians, up positive) on the front meridian of a ball of
/// radius 1 at the origin where `lobe` is brightest, seen from `camera`,
/// lit from `sun`; the strands run down the meridians from a root at the
/// top.
fn brightest_latitude(camera: Vec3, sun: Vec3, lobe: impl Fn(Vec3, Vec3, Vec3) -> f32) -> f32 {
    let mut best = (0.0, f32::MIN);
    for step in -6000..=6000 {
        let latitude = step as f32 * 1e-4;
        let point = Vec3::new(0.0, latitude.sin(), latitude.cos());
        let toward_root = Vec3::new(0.0, latitude.cos(), -latitude.sin());
        let value = lobe(toward_root, sun.normalize(), (camera - point).normalize());
        if value > best.1 {
            best = (latitude, value);
        }
    }
    best.0
}

/// The row (with a fraction, from a parabola through the brightest pixel
/// and its neighbours) where column `x` of `pixels` is brightest between
/// rows `from` and `to`, by `value` of a pixel.
fn brightest_row(
    pixels: &[u8],
    x: u32,
    (from, to): (u32, u32),
    value: impl Fn([f32; 3]) -> f32,
) -> f32 {
    let at = |y: u32| value(rgb_at(pixels, SIZE, x, y));
    let peak = (from..to).max_by(|&a, &b| at(a).total_cmp(&at(b))).unwrap();
    let (a, b, c) = (at(peak - 1), at(peak), at(peak + 1));
    let bend = a - 2.0 * b + c;
    let fraction = if bend.abs() > 1e-6 {
        0.5 * (a - c) / bend
    } else {
        0.0
    };
    peak as f32 + 0.5 + fraction
}

/// A ball of hair of `colour` seen from 6 studs in front, lit from the
/// camera's side and a little above it.
fn hair_ball(colour: [f32; 3], change: impl FnOnce(&mut HairShading)) -> DrawList {
    let item = ball(1.0, Vec3::ZERO, hair(colour, 0.25, change));
    dark_scene(vec![item], 6.0, 30.0, Vec3::new(0.0, 0.15, 1.0))
}

#[test]
fn a_hair_materials_data_and_flow_maps_take_the_metalness_and_specular_places() {
    let mut item = item(Shape::Block, Vec3::ZERO, 1.0);
    // A card: double-sided (single-sided is a shell, below).
    item.material.double_sided = true;
    item.material.maps.metalness = Some(crate::ChannelMap {
        texture: Arc::new(TestTexture(3)),
        channel: crate::Channel::Blue,
        invert: false,
    });
    item.material.maps.specular = Some(Arc::new(TestTexture(4)));
    item.material.shading = Shading::Hair(HairShading {
        data: Some(Arc::new(TestTexture(5))),
        flow: Some(Arc::new(TestTexture(6))),
        melanin: 0.5,
        redness: -0.25,
        ..HairShading::default()
    });
    let textures = material_textures(&item);
    assert_eq!(
        textures[2].map(|t| t.key()),
        Some(5),
        "data in the metalness place"
    );
    assert_eq!(
        textures[7].map(|t| t.key()),
        Some(6),
        "flow in the specular place"
    );
    let key = material_key(&item);
    let maps = maps_uniform_of(&key);
    assert_eq!(maps.shading[0], SHADING_HAIR);
    // Both maps bound (1 + 2), the tilt's scale, the secondary's strength.
    assert_eq!(maps.shading[1], 3.0);
    assert_eq!(maps.params[0][0], 0.5, "melanin");
    assert_eq!(maps.params[0][1], -0.25, "redness");
    // A hair material without maps still has a binding of its own.
    item.material.shading = Shading::Hair(HairShading::default());
    item.material.maps = Default::default();
    assert_ne!(material_key(&item), MaterialKey::default());
    assert_eq!(maps_uniform_of(&material_key(&item)).shading[1], 0.0);
    // Single-sided: a shell (4), whose light fades over its terminator.
    item.material.double_sided = false;
    assert_eq!(maps_uniform_of(&material_key(&item)).shading[1], 4.0);
}

#[test]
fn the_primary_highlight_lies_toward_the_root_where_the_model_puts_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    // Black hair: its light is all the white primary (R) lobe.
    let mut peak_row = |shift: f32| {
        let list = hair_ball([0.0; 3], |hair| {
            hair.shift = shift;
            hair.secondary = 0.0;
        });
        let pixels = draw(&mut renderer, &list);
        let middle = on_screen(&list, Vec3::ZERO, SIZE);
        let top = on_screen(&list, Vec3::new(0.0, 0.8, 0.6), SIZE).y as u32;
        let bottom = on_screen(&list, Vec3::new(0.0, -0.8, 0.6), SIZE).y as u32;
        let row = brightest_row(&pixels, middle.x as u32, (top, bottom), sum);
        let peak = rgb_at(&pixels, SIZE, middle.x as u32, row as u32);
        (list, row, peak)
    };
    for shift in [0.0, 1.0, 2.0] {
        let (list, row, peak) = peak_row(shift);
        assert!(
            sum(peak) > 60.0,
            "shift {shift}: a highlight is drawn: {peak:?}"
        );
        // White: the primary lobe isn't tinted (black hair, or any hair).
        assert!(peak[0] - peak[2] < 12.0, "shift {shift}: {peak:?}");
        let latitude = brightest_latitude(list.camera.position, list.sun.direction, |t, l, v| {
            r_lobe(t, l, v, 0.25, shift)
        });
        let expected = on_screen(&list, Vec3::new(0.0, latitude.sin(), latitude.cos()), SIZE).y;
        eprintln!(
            "shift {shift}: row {row:.2}, the model's {expected:.2} ({:.2}°)",
            latitude.to_degrees()
        );
        assert!(
            (row - expected).abs() < 1.5,
            "shift {shift}: the primary is at row {row:.2}; the model puts it at {expected:.2}"
        );
    }
    // Toward the root (up the ball, up the picture) as the tilt grows.
    let (_, unshifted, _) = peak_row(0.0);
    let (_, shifted, _) = peak_row(1.0);
    assert!(shifted < unshifted - 2.0, "{shifted} vs {unshifted}");
}

#[test]
fn the_secondary_highlight_lies_toward_the_tip_tinted_by_the_hairs_colour() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    // The secondary alone: hair drawn with it twice as strong, less hair
    // drawn without it.
    let mut secondary = |colour: [f32; 3], melanin: f32| {
        let with = hair_ball(colour, |hair| {
            hair.secondary = 2.0;
            hair.melanin = melanin;
        });
        let without = hair_ball(colour, |hair| {
            hair.secondary = 0.0;
            hair.melanin = melanin;
        });
        let a = draw(&mut renderer, &with);
        let b = draw(&mut renderer, &without);
        let difference: Vec<u8> = a
            .iter()
            .zip(&b)
            .map(|(a, b)| a.saturating_sub(*b))
            .collect();
        (with, difference)
    };
    let brown = [0.45, 0.3, 0.18];
    let (list, difference) = secondary(brown, 0.0);
    let middle = on_screen(&list, Vec3::ZERO, SIZE);
    let top = on_screen(&list, Vec3::new(0.0, 0.8, 0.6), SIZE).y as u32;
    let bottom = on_screen(&list, Vec3::new(0.0, -0.8, 0.6), SIZE).y as u32;
    let row = brightest_row(&difference, middle.x as u32, (top, bottom), sum);
    let latitude = brightest_latitude(list.camera.position, list.sun.direction, |t, l, v| {
        trt_lobe(t, l, v, 0.25, 1.0)
    });
    let expected = on_screen(&list, Vec3::new(0.0, latitude.sin(), latitude.cos()), SIZE).y;
    eprintln!(
        "secondary: row {row:.2}, the model's {expected:.2} ({:.2}°)",
        latitude.to_degrees()
    );
    assert!(
        (row - expected).abs() < 2.0,
        "the secondary is at row {row:.2}; the model puts it at {expected:.2}"
    );
    // Below the primary (toward the tips), which lies above the mirror
    // point (toward the root).
    let primary = brightest_latitude(list.camera.position, list.sun.direction, |t, l, v| {
        r_lobe(t, l, v, 0.25, 1.0)
    });
    let primary = on_screen(&list, Vec3::new(0.0, primary.sin(), primary.cos()), SIZE).y;
    assert!(
        row > primary + 4.0,
        "below the primary, toward the tips: {row} vs {primary}"
    );
    // Tinted by the hair: redder than it is blue, unlike the white primary.
    let tint = rgb_at(&difference, SIZE, middle.x as u32, row as u32);
    eprintln!("secondary's colour on brown hair: {tint:?}");
    assert!(sum(tint) > 20.0, "the secondary shows: {tint:?}");
    assert!(
        tint[0] > 1.4 * tint[2] + 2.0,
        "tinted by the hair: {tint:?}"
    );
    // Darker hair (more melanin) has a darker, more saturated secondary.
    let (_, darker) = secondary(brown, 0.8);
    let dark = rgb_at(&darker, SIZE, middle.x as u32, row as u32);
    eprintln!("secondary at melanin 0.8: {dark:?}");
    assert!(sum(dark) < 0.6 * sum(tint), "{dark:?} vs {tint:?}");
}

#[test]
fn the_primary_highlight_follows_the_strands_direction() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    // How far the highlight's bright pixels spread across and down the
    // picture: strands down the meridians draw a band across the ball,
    // strands around it (flow along U) a band down it.
    let mut spread = |strands: Option<[f32; 2]>| {
        let list = hair_ball([0.0; 3], |hair| {
            hair.secondary = 0.0;
            hair.flow = strands.map(|direction| flow(0x0f10 + direction[0] as u64, direction));
        });
        let pixels = draw(&mut renderer, &list);
        let brightest = (0..SIZE * SIZE)
            .map(|at| sum(rgb_at(&pixels, SIZE, at % SIZE, at / SIZE)))
            .fold(0.0, f32::max);
        let mut points = Vec::new();
        for y in 0..SIZE {
            for x in 0..SIZE {
                if sum(rgb_at(&pixels, SIZE, x, y)) > 0.6 * brightest {
                    points.push(Vec2::new(x as f32, y as f32));
                }
            }
        }
        let mean = points.iter().copied().sum::<Vec2>() / points.len() as f32;
        let spread = points
            .iter()
            .map(|p| (*p - mean) * (*p - mean))
            .sum::<Vec2>()
            / points.len() as f32;
        (spread.x.sqrt(), spread.y.sqrt())
    };
    let (across, down) = spread(None);
    eprintln!("strands down: spread {across:.1} across, {down:.1} down");
    assert!(
        across > 2.0 * down,
        "a band across the ball: {across} vs {down}"
    );
    let (across, down) = spread(Some([1.0, 0.0]));
    eprintln!("strands around: spread {across:.1} across, {down:.1} down");
    assert!(
        down > 2.0 * across,
        "a band down the ball: {across} vs {down}"
    );
}

#[test]
fn light_from_behind_comes_through_hair() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    // A card of strands running up and down between the camera and a sun
    // behind it, off to the side: across the strands, so the light
    // through them (TT, narrow along the strand, wide round it) reaches
    // the camera.
    let mut lit = |material: crate::Material| {
        let list = dark_scene(
            vec![card(Vec3::ZERO, [2.0, 2.0], material)],
            4.0,
            40.0,
            Vec3::new(0.4, 0.0, -1.0),
        );
        // A dim sun: what comes through is concentrated, and mustn't clip.
        let mut list = list;
        list.sun.intensity = 0.1;
        let pixels = draw(&mut renderer, &list);
        rgb_at(&pixels, SIZE, SIZE / 2, SIZE / 2)
    };
    let blonde = [0.75, 0.6, 0.4];
    let standard = lit(crate::Material {
        base_color: blonde,
        roughness: 0.25,
        ..crate::Material::default()
    });
    let through = lit(hair(blonde, 0.25, |_| {}));
    eprintln!("lit from behind: standard {standard:?}, hair {through:?}");
    assert!(
        sum(through) > sum(standard) + 40.0,
        "{through:?} vs {standard:?}"
    );
    // What comes through is coloured by the hair.
    assert!(through[0] > through[2] + 10.0, "{through:?}");
}

#[test]
fn melanin_redness_the_root_and_tip_and_streaks_change_the_pixels_live() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    // A card of light brown hair with the atlas's data (root at the top,
    // a streak down the left half), lit from the front.
    let data = hair_data(0xda_7a);
    let draw_with = |renderer: &mut TextureRenderer, change: &dyn Fn(&mut HairShading)| {
        let material = hair([0.55, 0.4, 0.28], 0.4, |hair| {
            hair.data = Some(data.clone());
            change(hair);
        });
        let list = dark_scene(
            vec![card(Vec3::ZERO, [2.0, 2.0], material)],
            3.0,
            50.0,
            Vec3::new(0.3, 0.2, 1.0),
        );
        let pixels = draw(renderer, &list);
        let at = |x: f32, y: f32| {
            let p = on_screen(&list, Vec3::new(x, y, 0.0), SIZE);
            rgb_at(&pixels, SIZE, p.x as u32, p.y as u32)
        };
        // Root and tip, on the streak's side (left) and off it (right).
        [at(-0.5, 0.9), at(0.5, 0.9), at(-0.5, -0.9), at(0.5, -0.9)]
    };
    let neutral = draw_with(&mut renderer, &|_| {});
    let meshes = renderer.stats().mesh_bytes;
    let changed = |a: [f32; 3], b: [f32; 3]| (sum(a) - sum(b)).abs() > 6.0;
    let unchanged = |a: [f32; 3], b: [f32; 3]| (sum(a) - sum(b)).abs() <= 3.0;

    let darker = draw_with(&mut renderer, &|hair| hair.melanin = 0.6);
    for (now, was) in darker.iter().zip(&neutral) {
        assert!(
            sum(*now) < sum(*was) - 6.0,
            "melanin darkens: {now:?} vs {was:?}"
        );
    }
    // Redness moves the melanin toward all pheomelanin (Blender's
    // "melanin redness"): warmer, and lighter, as pheomelanin absorbs less.
    let redder = draw_with(&mut renderer, &|hair| hair.redness = 1.0);
    let ratio = |p: [f32; 3]| p[0] / p[2].max(1.0);
    assert!(
        ratio(redder[1]) > ratio(neutral[1]) * 1.04,
        "redness: {:?} vs {:?}",
        redder[1],
        neutral[1]
    );
    let ashen = draw_with(&mut renderer, &|hair| hair.redness = -1.0);
    // Less red takes away the little pheomelanin light brown has.
    assert!(
        ratio(ashen[1]) < ratio(neutral[1]) * 0.995,
        "less red: {:?}",
        ashen[1]
    );
    // Dark roots over the top fifth of the strands: the root only.
    let roots = draw_with(&mut renderer, &|hair| {
        hair.root_melanin = 1.0;
        hair.root_length = 0.2;
    });
    assert!(
        sum(roots[1]) < sum(neutral[1]) - 6.0,
        "the root darkens: {:?}",
        roots[1]
    );
    assert!(
        unchanged(roots[3], neutral[3]),
        "the tip doesn't: {:?} vs {:?}",
        roots[3],
        neutral[3]
    );
    // Lighter tips: the tip only.
    let tips = draw_with(&mut renderer, &|hair| hair.tip_lightening = 1.0);
    assert!(
        sum(tips[3]) > sum(neutral[3]) + 6.0,
        "the tip lightens: {:?}",
        tips[3]
    );
    assert!(
        unchanged(tips[1], neutral[1]),
        "the root doesn't: {:?}",
        tips[1]
    );
    // Light streaks: where the mask is (left), not elsewhere.
    let streaks = draw_with(&mut renderer, &|hair| {
        hair.streaks = 1.0;
        hair.streak_melanin = -1.0;
    });
    assert!(
        changed(streaks[0], neutral[0]),
        "a streak: {:?} vs {:?}",
        streaks[0],
        neutral[0]
    );
    assert!(sum(streaks[0]) > sum(neutral[0]), "lighter");
    assert!(
        unchanged(streaks[1], neutral[1]),
        "off the streak: {:?}",
        streaks[1]
    );
    // A dye replaces the natural colour.
    let dyed = draw_with(&mut renderer, &|hair| {
        hair.dye = [0.1, 0.2, 0.9];
        hair.dye_amount = 1.0;
    });
    assert!(dyed[1][2] > dyed[1][0] + 10.0, "dyed blue: {:?}", dyed[1]);
    // Nothing was rebuilt: the mesh is the one uploaded first.
    assert_eq!(renderer.stats().mesh_bytes, meshes);
    // And back to neutral draws what neutral drew.
    assert_eq!(draw_with(&mut renderer, &|_| {}), neutral);
}

/// A single-sided hair surface (a hair shell or cap; here the evidence's
/// ball) lit from behind ends its lit band at the solid's terminator with a
/// smooth edge. Without a fade there, the hair model's light past the
/// terminator showed the shadow map's self-shadow edge, torn by its
/// per-pixel filter noise, as blotches several pixels deep (review fixes 1).
/// Under Off and MSAA, which don't average frames.
#[test]
fn a_back_lit_hair_shells_lit_band_ends_smoothly_at_its_terminator() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let size = 512;
    let list = super::character_models_evidence::scenes()
        .into_iter()
        .find(|(name, _)| *name == "hair-blonde-backlit")
        .unwrap()
        .1;
    // The terminator: the great circle of the ball (radius 1) square to
    // the sun, by where it is on screen, column by column (the half facing
    // the camera).
    let l = list.sun.direction;
    let (a, b) = (Vec3::new(0.0, -l.z, l.y).normalize(), Vec3::X);
    let mut terminator = vec![f32::MAX; size as usize];
    for step in 0..4000 {
        let angle = std::f32::consts::PI * (step as f32 / 4000.0 - 0.5);
        let point = on_screen(&list, a * angle.cos() + b * angle.sin(), size);
        let x = point.x.round() as usize;
        if x < terminator.len() {
            terminator[x] = terminator[x].min(point.y);
        }
    }
    let mut failures = Vec::new();
    for mode in [AntiAliasing::Off, AntiAliasing::Msaa4] {
        let mut renderer = TextureRenderer::new(&device, &queue, size, size);
        renderer.set_graphics(&GraphicsSettings {
            anti_aliasing: Some(mode),
            ambient_occlusion: Some(AmbientOcclusion::Off),
            ..GraphicsSettings::preset(Quality::High)
        });
        let pixels = draw(&mut renderer, &list);
        let at = |x: u32, y: u32| sum(rgb_at(&pixels, size, x, y));
        // Two runs of columns either side of the middle, clear of the
        // ball's pole (its strands' directions meet there: the test ball's
        // own artefact, which Blender's render of it shows too).
        let runs = [size / 2 - 96..size / 2 - 16, size / 2 + 16..size / 2 + 96];
        let peak = runs
            .iter()
            .flat_map(|run| run.clone())
            .flat_map(|x| (0..size).map(move |y| at(x, y)))
            .fold(0.0, f32::max);
        // Per column: how far past the terminator the lit band ends (its
        // last pixel over a tenth of the brightest), and how much that end
        // jumps from column to column (a torn edge jumps).
        let (mut past, mut steps, mut counted) = (0.0f32, 0.0, 0);
        for run in &runs {
            let mut previous = None;
            for x in run.clone() {
                let last = (0..size).rev().find(|&y| at(x, y) > 0.1 * peak).unwrap();
                past = past.max(last as f32 - terminator[x as usize]);
                if let Some(previous) = previous {
                    steps += (last as f32 - previous as f32).abs();
                    counted += 1;
                }
                previous = Some(last);
            }
        }
        let steps = steps / counted as f32;
        eprintln!(
            "{mode:?}: band ends {past:.1} rows past the terminator at most, \
             {steps:.2} rows from column to column"
        );
        // Before the fade: 21.1 rows past and 0.48 a column, under Off and
        // MSAA 4x alike; with it 2.6 and 0.30.
        if past > 6.0 {
            failures.push(format!("{mode:?}: lit {past} rows past the terminator"));
        }
        if steps >= 0.4 {
            failures.push(format!("{mode:?}: a ragged edge ({steps:.2})"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
