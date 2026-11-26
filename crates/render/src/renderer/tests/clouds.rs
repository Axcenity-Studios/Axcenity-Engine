//! Clouds: drawn in the sky, shadowing the ground beneath, carried by the
//! wind, and costing nothing without them.

use super::*;
use crate::{CloudQuality, Clouds, Quality};

/// The frame size these tests draw at.
const SIZE: u32 = 128;

/// A place where the clouds' noise has edges (cloud and clear sky) close
/// together, at the ground.
const CLOUDY: Vec3 = Vec3::new(-1750.0, 0.0, 1150.0);

/// Half the sky cloudy, drifting `wind` studs a second.
fn cloudy(wind: Vec3) -> Clouds {
    Clouds {
        cover: 0.5,
        wind,
        ..Clouds::default()
    }
}

/// A wide white floor under a sun straight overhead, seen from `height`
/// studs up, looking down and a little ahead. The sky is the default
/// gradient.
fn floor_from(height: f32) -> DrawList {
    let mut floor = item(Shape::Block, Vec3::new(0.0, -0.5, 0.0), 1.0);
    floor.transform = Mat4::from_scale_rotation_translation(
        Vec3::new(20_000.0, 1.0, 20_000.0),
        glam::Quat::IDENTITY,
        Vec3::new(0.0, -0.5, 0.0),
    );
    let mut list = list_of(vec![floor]);
    list.camera = Camera {
        position: CLOUDY + Vec3::new(0.0, height, 0.0),
        look_at: CLOUDY + Vec3::new(0.0, 0.0, -0.2 * height),
        fov_y_degrees: 70.0,
    };
    list.sun.intensity = 3.0;
    list
}

/// The sky straight above, seen from the ground, with the default sky's
/// colors.
fn sky_overhead() -> DrawList {
    let mut list = list_of(Vec::new());
    list.sky = Sky::default();
    list.camera = Camera {
        position: CLOUDY,
        look_at: CLOUDY + Vec3::new(0.0, 100.0, -20.0),
        fov_y_degrees: 70.0,
    };
    list.sun.intensity = 3.0;
    list.sun.direction = Vec3::new(0.3, 1.0, 0.2).normalize();
    list
}

/// How bright each pixel of the middle row is (the sum of its channels).
fn middle_row(pixels: &[u8]) -> Vec<f32> {
    let row = SIZE as usize / 2;
    pixels[row * SIZE as usize * 4..][..SIZE as usize * 4]
        .chunks(4)
        .map(|pixel| pixel[..3].iter().map(|&c| f32::from(c)).sum())
        .collect()
}

/// The number of pixels `after` is `before` moved along by: the shift that
/// lines the two up best, from `-most` to `most`.
fn best_shift(before: &[f32], after: &[f32], most: i32) -> i32 {
    let difference = |shift: i32| {
        let pairs = (0..before.len() as i32)
            .filter_map(|x| {
                let from = usize::try_from(x - shift).ok()?;
                Some((after[x as usize], *before.get(from)?))
            })
            .collect::<Vec<_>>();
        pairs.iter().map(|(a, b)| (a - b).abs()).sum::<f32>() / pairs.len() as f32
    };
    (-most..=most)
        .min_by(|&a, &b| difference(a).total_cmp(&difference(b)))
        .unwrap()
}

/// How many pixels along the middle row `studs` along X are at the
/// floor, in `list`'s view.
fn pixels_per_stud(list: &DrawList, at: Vec3) -> f32 {
    let screen_x = |point: Vec3| {
        let clip = view_projection(list, 1.0) * point.extend(1.0);
        (clip.x / clip.w * 0.5 + 0.5) * SIZE as f32
    };
    screen_x(at + Vec3::X) - screen_x(at)
}

fn spread(row: &[f32]) -> f32 {
    let most = row.iter().copied().fold(f32::MIN, f32::max);
    let least = row.iter().copied().fold(f32::MAX, f32::min);
    most - least
}

#[test]
fn clouds_and_their_shadows_move_with_the_wind() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let wind = Vec3::new(20.0, 0.0, 0.0);
    for (what, mut list, at) in [
        (
            "the clouds",
            sky_overhead(),
            CLOUDY + Vec3::new(0.0, Clouds::BOTTOM, -160.0),
        ),
        (
            "their shadows",
            floor_from(700.0),
            CLOUDY + Vec3::new(0.0, 0.0, -140.0),
        ),
    ] {
        let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
        list.clouds = cloudy(wind);
        list.time = 0.0;
        renderer.render(&list).unwrap();
        let before = middle_row(&renderer.read_pixels());
        assert!(spread(&before) > 40.0, "{what} show: {before:?}");
        // Five seconds on, the wind has carried them 100 studs along X.
        list.time = 5.0;
        renderer.render(&list).unwrap();
        let after = middle_row(&renderer.read_pixels());
        let expected = (100.0 * pixels_per_stud(&list, at)).round() as i32;
        assert!(expected >= 4, "{what}: {expected} pixels");
        let shift = best_shift(&before, &after, 2 * expected);
        assert!(
            (shift - expected).abs() <= 1,
            "{what} moved {shift} pixels, not {expected}"
        );

        // Without wind, they stay where they are.
        list.clouds.wind = Vec3::ZERO;
        list.time = 10.0;
        renderer.render(&list).unwrap();
        let still = renderer.read_pixels();
        list.time = 15.0;
        renderer.render(&list).unwrap();
        assert!(renderer.read_pixels() == still, "{what} stay still");
    }
}

#[test]
fn cover_zero_is_a_clear_sky_and_no_clouds_cost_nothing() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    for mut list in [sky_overhead(), floor_from(700.0)] {
        let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
        renderer.render(&list).unwrap();
        let clear = renderer.read_pixels();
        let clear_stats = renderer.stats();

        list.clouds = Clouds {
            cover: 0.0,
            ..cloudy(Vec3::X)
        };
        renderer.render(&list).unwrap();
        assert!(renderer.read_pixels() == clear, "cover 0 is a clear sky");

        list.clouds = cloudy(Vec3::X);
        renderer.render(&list).unwrap();
        assert!(renderer.read_pixels() != clear, "clouds show");
        // Traced, they keep their shapes' texture and two targets;
        // drawn by the sky pass, nothing.
        let cloudy_stats = renderer.stats();
        assert_eq!(cloudy_stats.draw_calls, clear_stats.draw_calls);
        let shapes = 4096 * 4096 * 2;
        let (frame_size, half_size) = (u64::from(SIZE).pow(2) * 8, u64::from(SIZE / 2).pow(2) * 8);
        assert_eq!(
            cloudy_stats.target_bytes,
            clear_stats.target_bytes + shapes + frame_size + half_size
        );
        renderer.set_trace_clouds(false);
        renderer.render(&list).unwrap();
        assert_eq!(renderer.stats().target_bytes, clear_stats.target_bytes);
        renderer.set_trace_clouds(true);
        renderer.render(&list).unwrap();

        list.clouds = Clouds::default();
        renderer.render(&list).unwrap();
        assert!(renderer.read_pixels() == clear, "removed, they're gone");
        assert_eq!(renderer.stats().target_bytes, clear_stats.target_bytes);
    }
}

#[test]
fn every_cloud_quality_draws_clouds_and_casts_the_same_shadows() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    renderer.render(&sky_overhead()).unwrap();
    let clear_sky = renderer.read_pixels();
    let mut shadows = Vec::new();
    for quality in CloudQuality::ALL {
        renderer.set_graphics(&GraphicsSettings {
            cloud_quality: Some(quality),
            ..GraphicsSettings::preset(Quality::High)
        });
        let mut sky = sky_overhead();
        sky.clouds = cloudy(Vec3::ZERO);
        renderer.render(&sky).unwrap();
        let clouded = renderer.read_pixels();
        let changed = clouded
            .chunks(4)
            .zip(clear_sky.chunks(4))
            .filter(|(a, b)| {
                a[..3]
                    .iter()
                    .zip(b.iter())
                    .any(|(x, y)| x.abs_diff(*y) > 20)
            })
            .count();
        assert!(
            changed > clouded.len() / 4 / 10,
            "{quality:?}: clouds over {changed} pixels"
        );
        let mut floor = floor_from(700.0);
        floor.clouds = cloudy(Vec3::ZERO);
        renderer.render(&floor).unwrap();
        shadows.push(renderer.read_pixels());
    }
    assert!(
        shadows.windows(2).all(|pair| pair[0] == pair[1]),
        "every quality shades the ground the same"
    );
}

#[test]
fn an_overcast_turns_the_skys_light_grey() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 64, 64);
    // A white floor lit by a blue sky alone.
    let mut list = floor_from(40.0);
    list.sun.intensity = 0.0;
    list.ambient.intensity = 0.0;
    list.sky.top_color = [0.1, 0.3, 1.0];
    list.sky.horizon_color = [0.2, 0.4, 1.0];
    list.sky.environment_intensity = 1.0;
    let blueness = |renderer: &mut TextureRenderer, list: &DrawList| {
        renderer.render(list).unwrap();
        let pixels = renderer.read_pixels();
        let middle = &pixels[(32 * 64 + 32) * 4..][..4];
        (
            i32::from(middle[2]) - i32::from(middle[0]),
            i32::from(middle[2]),
        )
    };
    let (clear, clear_blue) = blueness(&mut renderer, &list);
    assert!(clear > 40, "the floor takes the sky's blue: {clear}");

    // Fair-weather cloud leaves the sky's light as it is.
    list.clouds = Clouds {
        cover: 0.3,
        density: 0.7,
        ..Clouds::default()
    };
    assert_eq!(blueness(&mut renderer, &list), (clear, clear_blue));

    // Under a thick overcast it's grey, and a little dimmer.
    list.clouds.cover = 1.0;
    let (overcast, overcast_blue) = blueness(&mut renderer, &list);
    assert!(
        overcast.abs() < clear / 4,
        "{clear} clear, {overcast} overcast"
    );
    assert!(overcast_blue < clear_blue, "the layer takes some of it");

    // A veil of no depth changes nothing, however much of the sky it covers.
    list.clouds.density = 0.0;
    assert_eq!(blueness(&mut renderer, &list), (clear, clear_blue));
}

/// The sky toward the horizon from the ground, with a row of dark posts
/// standing against it and the sun low to one side.
fn sky_behind_posts() -> DrawList {
    let posts = (0..7)
        .map(|post| {
            let mut post_item = item(Shape::Block, Vec3::ZERO, 1.0);
            post_item.transform = Mat4::from_scale_rotation_translation(
                Vec3::new(1.5, 30.0 + 6.0 * post as f32, 1.5),
                glam::Quat::from_rotation_z(0.05 * post as f32),
                CLOUDY + Vec3::new(-30.0 + 10.0 * post as f32, 10.0, -60.0),
            );
            post_item
        })
        .collect();
    let mut list = list_of(posts);
    list.sky = Sky::default();
    list.camera = Camera {
        position: CLOUDY + Vec3::new(0.0, 6.0, 0.0),
        look_at: CLOUDY + Vec3::new(0.0, 30.0, -100.0),
        fov_y_degrees: 50.0,
    };
    list.sun.intensity = 3.0;
    list.sun.direction = Vec3::new(0.5, 0.35, -0.6).normalize();
    list
}

/// How far apart two pictures are: the mean difference of a channel, and
/// the share of pixels with a channel more than `much` apart (of 255).
fn apart(a: &[u8], b: &[u8], much: u8) -> (f32, f32) {
    let mut sum = 0u64;
    let mut far = 0usize;
    for (a, b) in a.chunks(4).zip(b.chunks(4)) {
        let most = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap();
        sum += (0..3).map(|c| u64::from(a[c].abs_diff(b[c]))).sum::<u64>();
        far += usize::from(most > much);
    }
    let pixels = a.len() / 4;
    (sum as f32 / (3 * pixels) as f32, far as f32 / pixels as f32)
}

#[test]
fn traced_clouds_are_the_sky_passs_clouds() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    const SIDE: u32 = 512;
    for quality in [
        CloudQuality::Medium,
        CloudQuality::High,
        CloudQuality::Ultra,
    ] {
        for (view, mut list) in [
            ("overhead", sky_overhead()),
            ("behind posts", sky_behind_posts()),
        ] {
            list.clouds = cloudy(Vec3::ZERO);
            let mut renderer = TextureRenderer::new(&device, &queue, SIDE, SIDE);
            renderer.set_graphics(&GraphicsSettings {
                cloud_quality: Some(quality),
                ..GraphicsSettings::preset(Quality::High)
            });
            renderer.set_trace_clouds(false);
            renderer.render(&list).unwrap();
            let marched = renderer.read_pixels();
            let marched_bytes = renderer.stats().target_bytes;
            renderer.set_trace_clouds(true);
            renderer.render(&list).unwrap();
            let traced = renderer.read_pixels();
            assert!(
                renderer.stats().target_bytes > marched_bytes,
                "{quality:?}, {view}: traced into a target of their own"
            );
            if let Some(folder) = std::env::var_os("AXCENITY_CLOUD_PICTURES") {
                let name = format!("{quality:?}-{}", view.replace(' ', "-"));
                let folder = std::path::Path::new(&folder);
                std::fs::write(folder.join(format!("{name}-marched.rgba")), &marched).unwrap();
                std::fs::write(folder.join(format!("{name}-traced.rgba")), &traced).unwrap();
                // The sky pass's clouds at four times the size, to judge
                // both against.
                let mut fine = TextureRenderer::new(&device, &queue, SIDE * 4, SIDE * 4);
                fine.set_graphics(&GraphicsSettings {
                    cloud_quality: Some(quality),
                    ..GraphicsSettings::preset(Quality::High)
                });
                fine.set_trace_clouds(false);
                fine.render(&list).unwrap();
                std::fs::write(folder.join(format!("{name}-fine.rgba")), fine.read_pixels())
                    .unwrap();
            }
            let (mean, far) = apart(&marched, &traced, 8);
            eprintln!(
                "{quality:?}, {view}: mean {mean:.3}, {:.3}% far",
                far * 100.0
            );
            // The two differ where the traced clouds read between half-size
            // texels that agree, and by the shapes' texture standing in for
            // the noise: a quarter of a level on average, and hardly a pixel
            // far off.
            let (most_mean, most_far) = (0.4, 0.006);
            assert!(
                mean < most_mean,
                "{quality:?}, {view}: a channel is {mean} apart on average"
            );
            assert!(
                far < most_far,
                "{quality:?}, {view}: {far} of the pixels are far apart"
            );
        }
    }
}

/// A picture's brightness (the mean of its channels) over each of `cells`
/// by `cells` squares.
fn brightness_grid(pixels: &[u8], side: u32, cells: u32) -> Vec<f32> {
    let cell = (side / cells) as usize;
    let side = side as usize;
    (0..cells as usize * cells as usize)
        .map(|index| {
            let (cx, cy) = (index % cells as usize, index / cells as usize);
            let mut sum = 0u64;
            for y in cy * cell..(cy + 1) * cell {
                for x in cx * cell..(cx + 1) * cell {
                    let at = (y * side + x) * 4;
                    sum += pixels[at..at + 3]
                        .iter()
                        .map(|&c| u64::from(c))
                        .sum::<u64>();
                }
            }
            sum as f32 / (3 * cell * cell) as f32
        })
        .collect()
}

#[test]
fn traced_clouds_keep_their_brightness_everywhere() {
    // Square by square the traced clouds are as bright as the sky pass's:
    // nothing gained or lost, whatever the grain does pixel by pixel.
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    const SIDE: u32 = 512;
    for (view, mut list) in [
        ("overhead", sky_overhead()),
        ("behind posts", sky_behind_posts()),
    ] {
        list.clouds = cloudy(Vec3::ZERO);
        let mut renderer = TextureRenderer::new(&device, &queue, SIDE, SIDE);
        renderer.set_graphics(&GraphicsSettings::preset(Quality::Ultra));
        let mut grid = |traced| {
            renderer.set_trace_clouds(traced);
            renderer.render(&list).unwrap();
            brightness_grid(&renderer.read_pixels(), SIDE, 16)
        };
        let (marched, traced) = (grid(false), grid(true));
        let most = marched
            .iter()
            .zip(&traced)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);
        assert!(most < 2.5, "{view}: a square is {most} of 255 apart");
    }
}

#[test]
fn traced_clouds_dont_change_when_their_shapes_are_drawn_again() {
    // The shapes' texture follows the camera in whole texels, so the same
    // view is the same picture wherever the texture's middle was left: by
    // a camera that came from 1,000 studs away (the old texture is kept) or
    // from 3,000 (it's drawn again).
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut list = sky_behind_posts();
    list.clouds = cloudy(Vec3::ZERO);
    let ultra = GraphicsSettings::preset(Quality::Ultra);
    let picture = |from: f32| {
        let mut renderer = TextureRenderer::new(&device, &queue, 256, 256);
        renderer.set_graphics(&ultra);
        let mut elsewhere = list.clone();
        elsewhere.camera.position.x += from;
        elsewhere.camera.look_at.x += from;
        // Enough frames for TAA to forget where it was.
        renderer.render(&elsewhere).unwrap();
        for _ in 0..24 {
            renderer.render(&list).unwrap();
        }
        renderer.read_pixels()
    };
    let here = picture(0.0);
    for from in [1_000.0, 3_000.0] {
        let (mean, far) = apart(&here, &picture(from), 2);
        assert!(
            mean < 0.05 && far < 0.001,
            "from {from} studs away: {mean} apart, {far} of the pixels"
        );
    }
}

#[test]
fn traced_clouds_flicker_no_more_than_the_sky_passs() {
    // A still view under TAA, whose jitter moves every ray a little each
    // frame: from one settled frame to the next the traced clouds change
    // no more than the sky pass's do.
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut list = sky_behind_posts();
    list.clouds = cloudy(Vec3::ZERO);
    let flicker = |traced| {
        let mut renderer = TextureRenderer::new(&device, &queue, 256, 256);
        renderer.set_graphics(&GraphicsSettings {
            anti_aliasing: Some(crate::AntiAliasing::Taa),
            ..GraphicsSettings::preset(Quality::Ultra)
        });
        renderer.set_trace_clouds(traced);
        for _ in 0..24 {
            renderer.render(&list).unwrap();
        }
        let before = renderer.read_pixels();
        let mut most = 0.0f32;
        for _ in 0..8 {
            renderer.render(&list).unwrap();
            most = most.max(apart(&before, &renderer.read_pixels(), 8).0);
        }
        most
    };
    let (marched, traced) = (flicker(false), flicker(true));
    eprintln!("flicker: marched {marched}, traced {traced}");
    assert!(
        traced <= marched + 0.05,
        "traced {traced}, the sky pass's {marched}"
    );
}
