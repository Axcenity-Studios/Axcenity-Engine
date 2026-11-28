//! Screen-space reflections (renderer/screen_reflections.rs): a smooth
//! floor shows a glowing block that's on the screen, from the second frame
//! (they read last frame's picture), a floor under a roof doesn't mirror
//! the bright sky the roof hides, a rough floor is left as it was, and
//! turning them off frees the pass and draws as before.

use super::*;

/// The frame size, square.
const SIZE: u32 = 96;

fn screen_reflections(on: bool) -> crate::GraphicsSettings {
    crate::GraphicsSettings {
        anti_aliasing: Some(crate::AntiAliasing::Off),
        screen_space_reflections: Some(on),
        ..crate::GraphicsSettings::default()
    }
}

fn block(size: Vec3, position: Vec3, material: Material) -> DrawItem {
    DrawItem {
        transform: Mat4::from_scale_rotation_translation(size, glam::Quat::IDENTITY, position),
        material,
        ..item(Shape::Block, Vec3::ZERO, 1.0)
    }
}

/// A black floor of `roughness` under a black sky, and a glowing red block
/// standing on it ahead of the camera, which looks along the floor at it:
/// the floor in front of the block mirrors it on the screen.
fn floor_and_red_block(roughness: f32) -> DrawList {
    let floor = block(
        Vec3::new(60.0, 1.0, 60.0),
        Vec3::new(0.0, -0.5, 0.0),
        Material {
            base_color: [0.02; 3],
            roughness,
            ..Material::default()
        },
    );
    let red = block(
        Vec3::new(8.0, 3.0, 1.0),
        Vec3::new(0.0, 1.5, -8.0),
        Material {
            base_color: [0.0; 3],
            emissive: [1.0, 0.0, 0.0],
            ..Material::default()
        },
    );
    let mut list = list_of(vec![floor, red]);
    list.camera = Camera {
        position: Vec3::new(0.0, 2.0, 6.0),
        look_at: Vec3::new(0.0, 0.0, -6.0),
        fov_y_degrees: 60.0,
    };
    list.sun.intensity = 0.0;
    list.ambient.intensity = 0.0;
    list.shadows.enabled = false;
    list
}

/// Where `point` is drawn in a `SIZE`-square frame of `list`, in pixels.
fn on_screen(list: &DrawList, point: Vec3) -> (usize, usize) {
    let clip = view_projection(list, 1.0) * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    (
        ((ndc.x * 0.5 + 0.5) * SIZE as f32) as usize,
        ((0.5 - ndc.y * 0.5) * SIZE as f32) as usize,
    )
}

fn pixel(pixels: &[u8], (x, y): (usize, usize)) -> [u8; 4] {
    pixels[(y * SIZE as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}

/// Draws `list` `frames` times and reads the last.
fn frames(renderer: &mut TextureRenderer, list: &DrawList, frames: usize) -> Vec<u8> {
    for _ in 0..frames {
        renderer.render(list).unwrap();
    }
    renderer.read_pixels()
}

/// Where the floor mirrors the red block's middle: the camera's line to
/// the block's mirror image (below the floor) crosses the floor there.
fn mirror_point(list: &DrawList) -> Vec3 {
    let eye = list.camera.position;
    let image = Vec3::new(0.0, -1.5, -8.0);
    eye + (image - eye) * (eye.y / (eye.y - image.y))
}

#[test]
fn a_smooth_floor_mirrors_a_block_on_the_screen_from_the_second_frame() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let list = floor_and_red_block(0.05);
    let at = on_screen(&list, mirror_point(&list));
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    renderer.set_graphics(&screen_reflections(false));
    let off = pixel(&frames(&mut renderer, &list, 2), at);
    renderer.set_graphics(&screen_reflections(true));
    // The first frame has no last frame to read.
    let first = pixel(&frames(&mut renderer, &list, 1), at);
    assert_eq!(first, off, "nothing on the first frame");
    let on = pixel(&frames(&mut renderer, &list, 2), at);
    assert!(renderer.scene.screen_reflections.is_some());
    assert!(
        on[0] > off[0] + 40 && on[0] > on[1] + 40,
        "the floor shows the red block: {off:?} -> {on:?}"
    );
    // Its reflection, not the block: the floor just beside the image's
    // sides stays dark.
    let beside = on_screen(&list, mirror_point(&list) + Vec3::new(3.2, 0.0, 0.0));
    let aside = pixel(&renderer.read_pixels(), beside);
    assert!(aside[0] < 20, "beside the reflection: {aside:?}");
}

#[test]
fn a_rough_floor_draws_as_with_them_off() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // Rougher than the cutoff, and nothing shades the sky: no sun.
    let list = floor_and_red_block(0.8);
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    renderer.set_graphics(&screen_reflections(false));
    let off = frames(&mut renderer, &list, 2);
    renderer.set_graphics(&screen_reflections(true));
    let on = frames(&mut renderer, &list, 3);
    assert!(renderer.scene.screen_reflections.is_some());
    assert_eq!(on, off);
}

#[test]
fn turning_them_off_frees_the_pass_and_draws_as_before() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let list = floor_and_red_block(0.05);
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    renderer.set_graphics(&screen_reflections(false));
    let before = frames(&mut renderer, &list, 2);
    assert!(renderer.scene.screen_reflections.is_none());
    let bytes = renderer.stats().target_bytes;
    renderer.set_graphics(&screen_reflections(true));
    assert_ne!(frames(&mut renderer, &list, 3), before);
    renderer.set_graphics(&screen_reflections(false));
    assert_eq!(frames(&mut renderer, &list, 1), before);
    assert!(renderer.scene.screen_reflections.is_none());
    assert_eq!(
        renderer.stats().target_bytes,
        bytes,
        "its buffers are freed"
    );
}

/// A black, smooth floor under a bright sky and a high sun, seen from
/// above looking down, with a wide roof `roof` high over the left half of
/// what's seen (above the frame, so the screen can't find it).
fn floor_under_a_bright_sky(roof: bool) -> DrawList {
    let floor = block(
        Vec3::new(200.0, 1.0, 200.0),
        Vec3::new(0.0, -0.5, 0.0),
        Material {
            base_color: [0.0; 3],
            roughness: 0.08,
            ..Material::default()
        },
    );
    let mut items = vec![floor];
    if roof {
        items.push(block(
            Vec3::new(30.0, 0.5, 60.0),
            Vec3::new(-15.0, 9.0, -10.0),
            Material {
                base_color: [0.3; 3],
                ..Material::default()
            },
        ));
    }
    let mut list = list_of(items);
    list.sky = Sky {
        top_color: [0.9, 0.92, 0.95],
        horizon_color: [1.0; 3],
        ground_color: [0.3; 3],
        environment_intensity: 1.0,
        image: None,
        ..Sky::default()
    };
    list.camera = Camera {
        position: Vec3::new(0.0, 6.0, 8.0),
        look_at: Vec3::new(0.0, 0.0, -4.0),
        fov_y_degrees: 50.0,
    };
    // Straight overhead, so the roof's shadow lies right under it.
    list.sun.direction = Vec3::new(0.2, 1.0, 0.3).normalize();
    list.sun.intensity = 1.0;
    list.ambient.intensity = 0.0;
    list
}

#[test]
fn a_floor_under_a_roof_does_not_mirror_the_sky_the_roof_hides() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let under = |list: &DrawList| on_screen(list, Vec3::new(-3.5, 0.0, -2.0));
    let open = |list: &DrawList| on_screen(list, Vec3::new(3.5, 0.0, -2.0));
    // Linear light, summed.
    let brightness = |p: [u8; 4]| {
        let rgb = [p[0], p[1], p[2]].map(|c| f32::from(c) / 255.0);
        let linear = linear_color(rgb);
        linear.x + linear.y + linear.z
    };
    let roofed = floor_under_a_bright_sky(true);
    let clip = view_projection(&roofed, 1.0) * Vec3::new(-6.0, 8.75, -2.0).extend(1.0);
    assert!(clip.y / clip.w > 1.0, "the roof is above the frame");
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);

    // Off: the roof's shadow aside, the floor under it mirrors the sky as
    // brightly as the open floor beside it (the bright blotches).
    renderer.set_graphics(&screen_reflections(false));
    let pixels = frames(&mut renderer, &roofed, 2);
    let (off_under, off_open) = (
        pixel(&pixels, under(&roofed)),
        pixel(&pixels, open(&roofed)),
    );
    assert!(
        brightness(off_under) > brightness(off_open) * 0.8,
        "without them the sky shows under the roof: {off_under:?}, open {off_open:?}"
    );

    renderer.set_graphics(&screen_reflections(true));
    let pixels = frames(&mut renderer, &roofed, 3);
    let (on_under, on_open) = (
        pixel(&pixels, under(&roofed)),
        pixel(&pixels, open(&roofed)),
    );
    assert!(
        brightness(on_under) < brightness(off_under) * 0.5,
        "under the roof the sky's reflection is shaded: {off_under:?} -> {on_under:?}"
    );
    assert!(
        brightness(on_open) > brightness(off_open) * 0.9,
        "the open floor still mirrors the sky: {off_open:?} -> {on_open:?}"
    );
}
