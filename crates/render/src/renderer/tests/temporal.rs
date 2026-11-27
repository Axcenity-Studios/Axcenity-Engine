//! Motion vectors, TAA and motion blur: the motion target holds how far each
//! pixel moved (moved parts, posed meshes and the camera alike), TAA smooths
//! edges and thin shapes over frames without leaving ghosts behind a moving
//! part, motion blur streaks only what moves relative to the camera, and
//! their targets exist only while they're used.

use super::skinning::{skin, square};
use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, MotionBlurQuality, Quality};
use crate::{GraphicsSettings, LensEffects, Shading};
use glam::Vec2;

/// The frame size, square.
const SIZE: u32 = 128;

/// High with `anti_aliasing` and `motion_blur`, and nothing else that keeps
/// the prepass alive.
fn settings(anti_aliasing: AntiAliasing, motion_blur: MotionBlurQuality) -> GraphicsSettings {
    GraphicsSettings {
        anti_aliasing: Some(anti_aliasing),
        motion_blur: Some(motion_blur),
        ambient_occlusion: Some(AmbientOcclusion::Off),
        contact_shadows: Some(false),
        volumetric_fog: Some(false),
        ..GraphicsSettings::preset(Quality::High)
    }
}

fn renderer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    settings: &GraphicsSettings,
) -> TextureRenderer {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(settings);
    renderer
}

fn draw(renderer: &mut TextureRenderer, list: &DrawList) -> Vec<u8> {
    renderer.render(list).unwrap();
    renderer.read_pixels()
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 3] {
    pixels[((y * SIZE + x) * 4) as usize..][..3]
        .try_into()
        .unwrap()
}

/// An unlit box of `color` (sRGB), `size` studs, at `center`, known as `id`.
fn card(color: [f32; 3], center: Vec3, size: Vec3, id: u64) -> DrawItem {
    DrawItem {
        transform: Mat4::from_scale_rotation_translation(size, glam::Quat::IDENTITY, center),
        material: Material {
            base_color: color,
            shading: Shading::Unlit,
            ..Material::default()
        },
        id,
        ..big_block()
    }
}

/// A grey wall across the back of the view.
fn wall(z: f32) -> DrawItem {
    card(
        [0.5; 3],
        Vec3::new(0.0, 0.0, z),
        Vec3::new(60.0, 60.0, 0.1),
        1,
    )
}

/// A white cube `size` studs wide at `center`.
fn cube(center: Vec3, size: f32, id: u64) -> DrawItem {
    card([1.0; 3], center, Vec3::splat(size), id)
}

/// Where `point` is on screen for `list`'s camera, in texture coordinates.
fn uv_of(list: &DrawList, point: Vec3) -> Vec2 {
    let clip = view_projection(list, 1.0) * point.extend(1.0);
    Vec2::new(clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5)
}

fn pixel_of(uv: Vec2) -> (u32, u32) {
    let at = (uv * SIZE as f32).floor();
    (at.x as u32, at.y as u32)
}

/// The pixels around everything `items` cover on screen, `margin` pixels
/// wider each way: left, top, right and bottom.
fn screen_box(list: &DrawList, items: &[&DrawItem], margin: f32) -> [u32; 4] {
    let corners = items.iter().flat_map(|item| {
        [0, 1, 2, 3, 4, 5, 6, 7].map(|bits| {
            let corner = Vec3::new(
                if bits & 1 == 0 { -0.5 } else { 0.5 },
                if bits & 2 == 0 { -0.5 } else { 0.5 },
                if bits & 4 == 0 { -0.5 } else { 0.5 },
            );
            uv_of(list, item.transform.transform_point3(corner)) * SIZE as f32
        })
    });
    let (low, high) = corners.fold((Vec2::splat(1e9), Vec2::splat(-1e9)), |(low, high), at| {
        (low.min(at), high.max(at))
    });
    let low = (low - margin).max(Vec2::ZERO);
    let high = (high + margin).min(Vec2::splat(SIZE as f32 - 1.0));
    [low.x as u32, low.y as u32, high.x as u32, high.y as u32]
}

fn inside([left, top, right, bottom]: [u32; 4], x: u32, y: u32) -> bool {
    (left..=right).contains(&x) && (top..=bottom).contains(&y)
}

/// The motion target the last frame drew: how far each pixel moved since
/// the frame before, in texture coordinates.
fn read_motion(renderer: &TextureRenderer) -> Vec<Vec2> {
    let scene = &renderer.scene;
    let texture = scene
        .frame_targets
        .temporal
        .as_ref()
        .expect("motion targets")
        .last_motion(&scene.history);
    let (width, height) = (texture.width(), texture.height());
    let row_bytes = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = scene.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("motion read back"),
        size: u64::from(row_bytes * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = scene.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    scene.queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |result| {
        result.expect("could not read the motion back");
    });
    scene
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .expect("the buffer is mapped");
    mapped
        .chunks(row_bytes as usize)
        .flat_map(|row| row[..(width * 4) as usize].chunks(4))
        .map(|texel| {
            let half = |at: usize| half_to_f32(u16::from_le_bytes([texel[at], texel[at + 1]]));
            Vec2::new(half(0), half(2))
        })
        .collect()
}

/// A 16-bit float's value.
fn half_to_f32(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x3ff) / 1024.0;
    sign * match exponent {
        0 => fraction * 2f32.powi(-14),
        31 => f32::INFINITY,
        _ => (1.0 + fraction) * 2f32.powi(exponent - 15),
    }
}

fn motion_at(motion: &[Vec2], (x, y): (u32, u32)) -> Vec2 {
    motion[(y * SIZE + x) as usize]
}

#[test]
fn motion_vectors_hold_how_far_moved_parts_posed_meshes_and_the_camera_went() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Taa, MotionBlurQuality::Off),
    );
    // A part moved half a stud to the right; the wall stays.
    let at = |x: f32| list_of(vec![wall(-10.0), cube(Vec3::new(x, 0.0, -5.0), 1.0, 2)]);
    let (before, now) = (at(-1.0), at(-0.5));
    renderer.render(&before).unwrap();
    renderer.render(&now).unwrap();
    let motion = read_motion(&renderer);
    // The middle of its face toward the camera, half a stud in front.
    let center = Vec3::new(-0.5, 0.0, -4.5);
    let expected = uv_of(&now, center) - uv_of(&before, Vec3::new(-1.0, 0.0, -4.5));
    let moved = motion_at(&motion, pixel_of(uv_of(&now, center)));
    assert!(expected.x > 0.02, "{expected}");
    assert!(moved.distance(expected) < 0.002, "{moved} vs {expected}");
    let still = motion_at(&motion, pixel_of(uv_of(&now, Vec3::new(3.0, 3.0, -10.0))));
    assert!(still.length() < 1e-4, "the wall didn't move: {still}");

    // The camera and the part both move another half stud right: the part
    // is still on screen, and the wall moves left.
    let mut follow = at(0.0);
    follow.camera.position.x += 0.5;
    follow.camera.look_at.x += 0.5;
    renderer.render(&follow).unwrap();
    let motion = read_motion(&renderer);
    let moved = motion_at(&motion, pixel_of(uv_of(&follow, Vec3::new(0.0, 0.0, -4.5))));
    assert!(
        moved.length() < 0.001,
        "the part kept its place on screen: {moved}"
    );
    let point = Vec3::new(3.0, 3.0, -10.0);
    let expected = uv_of(&follow, point) - uv_of(&now, point);
    let wall_moved = motion_at(&motion, pixel_of(uv_of(&follow, point)));
    assert!(expected.x < -0.01, "{expected}");
    assert!(
        wall_moved.distance(expected) < 0.001,
        "{wall_moved} vs {expected}"
    );

    // A posed mesh whose joint moves (its transform stays) moves with it.
    let posed = |x: f32| {
        let mut item = square(0, Some(skin(1, Vec3::new(x - 30.0, 0.0, 0.0))));
        item.id = 3;
        item.material.shading = Shading::Unlit;
        list_of(vec![wall(-10.0), item])
    };
    let (before, now) = (posed(0.0), posed(0.25));
    renderer.render(&before).unwrap();
    renderer.render(&now).unwrap();
    let motion = read_motion(&renderer);
    let center = Vec3::new(0.25, 0.0, -3.0);
    let expected = uv_of(&now, center) - uv_of(&before, Vec3::new(0.0, 0.0, -3.0));
    let moved = motion_at(&motion, pixel_of(uv_of(&now, center)));
    assert!(expected.x > 0.02, "{expected}");
    assert!(
        moved.distance(expected) < 0.002,
        "posed: {moved} vs {expected}"
    );

    // A mesh moved by its blend shape alone (joint and transform still)
    // moves with it too: last frame's posed corners are kept (CO8).
    let shaped = |x: f32| {
        // The shape moves the square 30 studs left at weight 1.
        let mut item = super::skinning::morphed_skinned_square(1.0 - x / 30.0);
        item.id = 4;
        item.material.shading = Shading::Unlit;
        list_of(vec![wall(-10.0), item])
    };
    let (before, now) = (shaped(0.0), shaped(0.25));
    renderer.render(&before).unwrap();
    renderer.render(&now).unwrap();
    let motion = read_motion(&renderer);
    let moved = motion_at(&motion, pixel_of(uv_of(&now, center)));
    assert!(
        moved.distance(expected) < 0.002,
        "shaped: {moved} vs {expected}"
    );
}

#[test]
fn taa_smooths_edges_and_thin_geometry_over_frames() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let after = |list: &DrawList, mode, frames| {
        let mut renderer = renderer(&device, &queue, &settings(mode, MotionBlurQuality::Off));
        assert_eq!(renderer.graphics().anti_aliasing, mode);
        for _ in 1..frames {
            renderer.render(list).unwrap();
        }
        draw(&mut renderer, list)
    };
    let in_between = |pixels: &[u8]| {
        pixels
            .chunks(4)
            .filter(|pixel| (10..=245).contains(&pixel[0]))
            .count()
    };
    // A tilted block's edge over the sky.
    let block = tilted_block();
    let off = after(&block, AntiAliasing::Off, 1);
    assert_eq!(in_between(&off), 0, "hard steps");
    let first = after(&block, AntiAliasing::Taa, 1);
    assert_eq!(in_between(&first), 0, "one frame has no history yet");
    let smoothed = after(&block, AntiAliasing::Taa, 16);
    let inside = smoothed.chunks(4).filter(|pixel| pixel[0] > 245).count();
    assert!(inside > 1000, "the block is still drawn whole: {inside}");
    let smoothed = in_between(&smoothed);
    assert!(smoothed >= 60, "TAA blends edge pixels: {smoothed}");

    // A glowing bar about a pixel thick, a little tilted: without
    // anti-aliasing it's a staircase of whole pixels. With TAA each column
    // shares its light between the pixels it crosses, and keeps about as
    // much of it (none is clamped away).
    let bar = {
        let item = DrawItem {
            transform: Mat4::from_scale_rotation_translation(
                Vec3::new(5.0, 0.066, 0.05),
                glam::Quat::from_rotation_z(4f32.to_radians()),
                Vec3::new(0.0, 0.0, -5.0),
            ),
            material: Material {
                base_color: [0.0; 3],
                emissive: [1.0; 3],
                ..Material::default()
            },
            ..big_block()
        };
        let mut list = list_of(vec![item]);
        list.shadows.enabled = false;
        list
    };
    let linear = |value: u8| linear_color([f32::from(value) / 255.0; 3]).x;
    let columns = |pixels: &[u8]| {
        (25..SIZE - 25)
            .map(|x| {
                (0..SIZE)
                    .map(|y| linear(pixel(pixels, x, y)[0]))
                    .sum::<f32>()
            })
            .collect::<Vec<_>>()
    };
    let stepped = after(&bar, AntiAliasing::Off, 1);
    assert_eq!(in_between(&stepped), 0, "whole pixels only");
    let thin = after(&bar, AntiAliasing::Taa, 24);
    let shared = in_between(&thin);
    assert!(shared >= 60, "the bar's pixels share its light: {shared}");
    for (x, (theirs, ours)) in columns(&stepped).iter().zip(columns(&thin)).enumerate() {
        assert!(*theirs >= 0.99, "column {x}: the bar crosses every column");
        assert!(
            (0.5..=2.2).contains(&ours),
            "column {x}: TAA keeps the bar's light: {ours} (without, {theirs})"
        );
    }
}

#[test]
fn taa_leaves_no_ghost_behind_a_moving_part() {
    leaves_no_ghost(1.0, 2.0, |_| 0.0);
}

/// G9 stage 2: TAA that upscales (from half the size) leaves none either.
/// Its edges are softer, so more round the part is left out.
#[test]
fn upscaling_taa_leaves_no_ghost_behind_a_moving_part() {
    leaves_no_ghost(0.5, 4.0, |_| 0.0);
}

/// Nor while the camera turns by a different amount each frame: the
/// history is kept through the camera's own changes of pace
/// (`taa_keeps_its_history_while_the_camera_turns_unevenly`), and still
/// dropped where the part uncovered the wall.
#[test]
fn taa_leaves_no_ghost_behind_a_moving_part_while_the_camera_turns_unevenly() {
    leaves_no_ghost(1.0, 2.0, uneven_yaw);
    leaves_no_ghost(0.5, 4.0, uneven_yaw);
}

/// What a moving part uncovers starts again from the frame as drawn. A
/// grey part crosses a wall of dark and light stripes four pixels wide:
/// grey is among the colors round every pixel at a stripe's edge, so
/// holding the history to those colors lets the part's grey through, and
/// only the history's being dropped where the motion differs from last
/// frame's keeps the stripes it uncovers from being washed grey. (The
/// plain wall of `leaves_no_ghost` shows no ghost either way.) With the
/// camera still, and turning by a different amount each frame, where the
/// camera's own change of pace must not be taken for the part's.
#[test]
fn taa_drops_the_history_of_what_a_moving_part_uncovers() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let stripes = super::card_set::Made::shared(0x057a_19e5, 64, |x, _| {
        let value = if (x / 2).is_multiple_of(2) { 235 } else { 40 };
        [value, value, value, 255]
    });
    let mut board = card(
        [1.0; 3],
        Vec3::new(0.0, 0.0, -8.0),
        Vec3::new(12.0, 12.0, 0.1),
        1,
    );
    board.material.base_color_texture = Some(stripes);
    let graphics = settings(AntiAliasing::Taa, MotionBlurQuality::Off);
    for (name, yaw) in [
        ("still", (|_| 0.0) as fn(u32) -> f32),
        ("uneven", uneven_yaw),
    ] {
        // The part moves a third of a stud (about 6 pixels) a frame.
        let part = |frame: u32| {
            let x = -3.0 + frame as f32 / 3.0;
            card(
                [0.54; 3],
                Vec3::new(x, 0.0, -5.0),
                Vec3::new(1.5, 1.5, 0.1),
                2,
            )
        };
        let at = |frame: u32, with_part: bool| {
            let mut items = vec![board.clone()];
            if with_part {
                items.push(part(frame));
            }
            let mut list = list_of(items);
            let yaw = yaw(frame);
            list.camera.look_at = list.camera.position + Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
            list
        };
        let frames = 13;
        let mut taa = renderer(&device, &queue, &graphics);
        let mut ours = Vec::new();
        for frame in 0..frames {
            ours = draw(&mut taa, &at(frame, true));
        }
        // The stripes alone from the same view, settled.
        let last = at(frames - 1, true);
        let mut alone = renderer(&device, &queue, &graphics);
        let mut theirs = Vec::new();
        for _ in 0..48 {
            theirs = draw(&mut alone, &at(frames - 1, false));
        }
        // Where the part was two frames ago, clear of where it is now.
        let now = screen_box(&last, &[&last.items[1]], 3.0);
        let was = at(frames - 3, true);
        let mut trail = screen_box(&last, &[&was.items[1]], 0.0);
        trail[2] = trail[2].min(now[0] - 1);
        assert!(trail[0] + 6 < trail[2], "{name}: a trail to look at");
        let mut apart = 0.0;
        let mut pixels = 0.0;
        for y in trail[1] + 2..trail[3] - 2 {
            for x in trail[0]..=trail[2] {
                apart += f32::from(pixel(&ours, x, y)[0].abs_diff(pixel(&theirs, x, y)[0]));
                pixels += 1.0;
            }
        }
        let apart = apart / pixels;
        eprintln!("{name}: the uncovered stripes are {apart:.1} levels from the stripes alone");
        assert!(
            apart < 22.0,
            "{name}: the stripes the part uncovered are {apart} levels off, washed by its grey"
        );
    }
}

/// The camera's yaw at `frame`, in radians, turning there and back by a
/// different amount each frame: 2.9 pixels, 0.2, then both back (91 pixels
/// a radian here), so each frame's motion is about 3 pixels from the last
/// one's and the view stays within 3 pixels of straight ahead.
fn uneven_yaw(frame: u32) -> f32 {
    const STEPS: [f32; 4] = [2.9, 0.2, -2.9, -0.2];
    let turned: f32 = (0..frame).map(|at| STEPS[at as usize % 4]).sum();
    turned / 91.4
}

/// A part crossing a wall with TAA at `render_scale`, against the wall drawn
/// without TAA at full size, away from `margin` pixels round the part. The
/// camera is turned `yaw(frame)` radians at each frame.
fn leaves_no_ghost(render_scale: f32, margin: f32, yaw: fn(u32) -> f32) {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // A white part crossing a grey wall, a third of a stud (about 6 pixels)
    // a frame.
    let at = |frame: u32| {
        let x = -3.0 + frame as f32 / 3.0;
        let mut list = list_of(vec![wall(-6.0), cube(Vec3::new(x, 0.0, -5.0), 1.5, 2)]);
        let yaw = yaw(frame);
        list.camera.look_at = list.camera.position + Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        list
    };
    let mut taa = renderer(
        &device,
        &queue,
        &GraphicsSettings {
            render_scale: Some(render_scale),
            ..settings(AntiAliasing::Taa, MotionBlurQuality::Off)
        },
    );

    let frames = 12;
    for frame in 0..frames {
        taa.render(&at(frame)).unwrap();
    }
    let last = at(frames - 1);
    let ours = draw(&mut taa, &last);
    let mut off = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Off, MotionBlurQuality::Off),
    );
    let theirs = draw(&mut off, &last);
    // Away from the part's edges (where the anti-aliasing differs), and
    // above all where it was in the frames before, TAA shows the wall.
    let part = screen_box(&last, &[&last.items[1]], margin);
    let trail = screen_box(&at(frames - 6), &[&at(frames - 6).items[1]], 0.0);
    assert!(trail[0] < part[0], "the part was to the left");
    let mut trail_pixels = 0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            if inside(part, x, y) {
                continue;
            }
            let (a, b) = (pixel(&ours, x, y), pixel(&theirs, x, y));
            let difference = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap();
            assert!(
                difference <= 6,
                "({x}, {y}): {a:?} vs {b:?}, a ghost where the part was"
            );
            trail_pixels += usize::from(inside(trail, x, y));
        }
    }
    assert!(trail_pixels > 200, "the trail was checked: {trail_pixels}");
    // The part itself kept up with its motion: its middle is white.
    let middle = pixel_of(uv_of(&last, last.items[1].transform.w_axis.truncate()));
    assert_eq!(pixel(&ours, middle.0, middle.1), [255; 3]);
}

#[test]
fn motion_blur_streaks_only_what_moves_relative_to_the_camera() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // A part that stays on the left, and one moving right on the right.
    let at = |moving: f32, camera: f32| {
        let mut list = list_of(vec![
            wall(-8.0),
            cube(Vec3::new(-2.5, 0.0, -5.0), 1.2, 2),
            cube(Vec3::new(moving, 0.0, -5.0), 1.2, 3),
        ]);
        list.camera.position.x += camera;
        list.camera.look_at.x += camera;
        list.lens = LensEffects {
            motion_blur: Some(1.0),
            ..LensEffects::default()
        };
        list
    };
    let frames = [at(1.2, 0.0), at(1.8, 0.0), at(2.4, 0.6)];
    let mut blurred = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Off, MotionBlurQuality::High),
    );
    let mut sharp = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Off, MotionBlurQuality::Off),
    );
    let changed = |ours: &[u8], theirs: &[u8], area: [u32; 4]| {
        let mut changed = 0;
        let mut most = 0;
        for y in area[1]..=area[3] {
            for x in area[0]..=area[2] {
                let (a, b) = (pixel(ours, x, y), pixel(theirs, x, y));
                let difference = (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap();
                most = most.max(difference);
                changed += usize::from(difference > 20);
            }
        }
        (changed, most)
    };
    let mut drawn = Vec::new();
    for list in &frames {
        drawn.push((draw(&mut blurred, list), draw(&mut sharp, list)));
    }
    // The first frame has nothing to compare with: nothing moves.
    assert_eq!(drawn[0].0, drawn[0].1);

    // With the camera still, the moving part streaks along its motion and
    // the still one stays exactly as it was.
    let list = &frames[1];
    let (ours, theirs) = &drawn[1];
    let still = screen_box(list, &[&list.items[1]], 8.0);
    let moving = screen_box(list, &[&list.items[2]], 8.0);
    assert_eq!(
        changed(ours, theirs, still),
        (0, 0),
        "the still part is sharp"
    );
    let (streaked, _) = changed(ours, theirs, moving);
    assert!(streaked > 60, "the moving part streaks: {streaked} pixels");
    // Not up and down: the rows above and below it are untouched.
    let [left, top, right, bottom] = moving;
    assert_eq!(changed(ours, theirs, [left, top, right, top + 3]).0, 0);
    assert_eq!(
        changed(ours, theirs, [left, bottom - 3, right, bottom]).0,
        0
    );

    // The camera follows the moving part: now it's the one that stays
    // sharp, and the still part (and the wall's edges, which it has none
    // of) streaks.
    let list = &frames[2];
    let (ours, theirs) = &drawn[2];
    let still = screen_box(list, &[&list.items[1]], 8.0);
    let moving = screen_box(list, &[&list.items[2]], 8.0);
    assert!(
        changed(ours, theirs, moving).1 <= 2,
        "the followed part is sharp"
    );
    let (streaked, _) = changed(ours, theirs, still);
    assert!(
        streaked > 60,
        "the part left behind streaks: {streaked} pixels"
    );
}

#[test]
fn motion_targets_exist_only_while_taa_or_motion_blur_use_them() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let plain = list_of(vec![wall(-8.0), cube(Vec3::new(0.0, 0.0, -5.0), 1.0, 2)]);
    let streaked = DrawList {
        lens: LensEffects {
            motion_blur: Some(0.5),
            ..LensEffects::default()
        },
        ..plain.clone()
    };
    let mut renderer = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Off, MotionBlurQuality::High),
    );
    // The draws of a frame without the prepass.
    let mut without = None;
    let mut check = |settings: GraphicsSettings, list: &DrawList, alive, prepass: bool| {
        renderer.set_graphics(&settings);
        renderer.render(list).unwrap();
        renderer.render(list).unwrap();
        let scene = &renderer.scene;
        assert_eq!(scene.temporal_targets_alive(), alive, "{settings:?}");
        assert_eq!(
            scene.frame_targets.occlusion.is_some(),
            prepass,
            "{settings:?}"
        );
        assert_eq!(scene.targets_alive(), 1);
        // Nothing moved, so the prepass's draws are the only extra ones:
        // there's no moving items' pass.
        let stats = renderer.stats();
        let base = *without.get_or_insert(stats.draw_calls);
        let expected = base + if prepass { stats.batches } else { 0 };
        assert_eq!(stats.draw_calls, expected, "{settings:?}");
    };
    use AntiAliasing::{Off, Taa};
    use MotionBlurQuality as Blur;
    // Nothing to streak and no TAA: nothing kept, no prepass.
    check(settings(Off, Blur::High), &plain, (0, 0, 0), false);
    // Motion blur asked for but turned off in the settings: the same.
    check(settings(Off, Blur::Off), &streaked, (0, 0, 0), false);
    check(settings(Off, Blur::Low), &streaked, (1, 0, 1), true);
    check(settings(Taa, Blur::Off), &plain, (1, 1, 0), true);
    check(settings(Taa, Blur::High), &streaked, (1, 1, 1), true);
    check(settings(Off, Blur::High), &plain, (0, 0, 0), false);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
}

#[test]
fn a_plant_swaying_in_the_wind_has_motion_of_its_own() {
    // Its leaves move though it stands still and so does the camera, so TAA
    // must take them from where the wind had them last frame, or they smear.
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Taa, MotionBlurQuality::Off),
    );
    let at = |time: f32| {
        let mut plant = card(
            [0.2, 0.8, 0.2],
            Vec3::new(0.0, 1.0, -5.0),
            Vec3::new(1.0, 4.0, 0.2),
            2,
        );
        plant.sway = Some(crate::Sway {
            base: -1.0,
            height: 4.0,
            strength: 1.0,
            flutter: 0.0,
        });
        let mut list = list_of(vec![wall(-10.0), plant]);
        list.wind = Vec3::new(20.0, 0.0, 0.0);
        list.time = time;
        list
    };
    renderer.render(&at(0.0)).unwrap();
    let now = at(0.25);
    renderer.render(&now).unwrap();
    let motion = read_motion(&renderer);
    // Near its top, which sways the most; its foot stays put.
    let top = motion_at(&motion, pixel_of(uv_of(&now, Vec3::new(0.0, 2.6, -4.9))));
    assert!(top.length() > 1e-3, "the top swayed: {top}");
    let still = motion_at(&motion, pixel_of(uv_of(&now, Vec3::new(4.0, 4.0, -10.0))));
    assert!(still.length() < 1e-4, "the wall didn't move: {still}");
}

/// A camera cut in a rendered movie: without forgetting, motion blur
/// streaks the first frame after the jump and TAA mixes the old view in.
/// TAA reads last frame's picture from where each pixel was, which while
/// the camera turns is between pixels. Read there as a plain blend of the
/// four around it, the picture lost a little sharpness every frame, so a
/// moving view went soft and came back as the camera stopped (in Desert
/// Homestead a turning frame was 3.3 levels from the settled one, 2.3 read
/// through the spline upscaling already used).
#[test]
fn taa_keeps_a_turning_cameras_picture_close_to_the_settled_one() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // A checkered card, each square about four pixels wide.
    let checkers = super::card_set::Made::shared(0x7aa_c4ec, 64, |x, y| {
        let light = (x / 2 + y / 2).is_multiple_of(2);
        let value = if light { 235 } else { 40 };
        [value, value, value, 255]
    });
    let mut board = card(
        [1.0; 3],
        Vec3::new(0.0, 0.0, -8.0),
        Vec3::new(12.0, 12.0, 0.1),
        1,
    );
    board.material.base_color_texture = Some(checkers);
    // Turned a third of a pixel a frame (91 pixels a radian here).
    let at = |frame: u32| {
        let mut list = list_of(vec![board.clone()]);
        let yaw = frame as f32 * 0.37 / 91.4;
        list.camera.look_at = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        list
    };
    let mut renderer = renderer(
        &device,
        &queue,
        &settings(AntiAliasing::Taa, MotionBlurQuality::Off),
    );
    let mut moving = Vec::new();
    for frame in 0..=40 {
        moving = draw(&mut renderer, &at(frame));
    }
    let mut settled = Vec::new();
    for _ in 0..48 {
        settled = draw(&mut renderer, &at(40));
    }
    let middle = SIZE / 4..SIZE * 3 / 4;
    let apart = middle
        .clone()
        .flat_map(|y| middle.clone().map(move |x| (x, y)))
        .map(|(x, y)| f32::from(pixel(&moving, x, y)[0].abs_diff(pixel(&settled, x, y)[0])))
        .sum::<f32>()
        / (SIZE * SIZE / 4) as f32;
    eprintln!("turning against settled: {apart:.2} levels");
    assert!(
        apart < 9.0,
        "the turning picture is {apart} levels from the settled one"
    );
}

/// A camera never turns by the same amount two frames running: a hand on
/// a mouse doesn't, a frame that took longer moves it further, and it
/// starts and stops. TAA dropped its history wherever this frame's motion
/// was over half a pixel from last frame's, meant for what a moving part
/// uncovers, so each such frame showed the whole picture as drawn, with no
/// anti-aliasing and every per-pixel noise bare (in Desert Homestead a turn
/// with steps 40% uneven read twice as sharp as a steady one, every frame).
/// The camera's own change of pace is now left out of that comparison.
#[test]
fn taa_keeps_its_history_while_the_camera_turns_unevenly() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let checkers = super::card_set::Made::shared(0x7aa_c4ec, 64, |x, y| {
        let light = (x / 2 + y / 2).is_multiple_of(2);
        let value = if light { 235 } else { 40 };
        [value, value, value, 255]
    });
    let mut board = card(
        [1.0; 3],
        Vec3::new(0.0, 0.0, -8.0),
        Vec3::new(12.0, 12.0, 0.1),
        1,
    );
    board.material.base_color_texture = Some(checkers);
    let at = |frame: u32| {
        let mut list = list_of(vec![board.clone()]);
        let yaw = uneven_yaw(frame);
        list.camera.look_at = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        list
    };
    let graphics = settings(AntiAliasing::Taa, MotionBlurQuality::Off);
    let mut renderer = renderer(&device, &queue, &graphics);
    let mut moving = Vec::new();
    for frame in 0..=41 {
        moving = draw(&mut renderer, &at(frame));
    }
    let mut settled = Vec::new();
    for _ in 0..48 {
        settled = draw(&mut renderer, &at(41));
    }
    let middle = SIZE / 4..SIZE * 3 / 4;
    let apart = middle
        .clone()
        .flat_map(|y| middle.clone().map(move |x| (x, y)))
        .map(|(x, y)| f32::from(pixel(&moving, x, y)[0].abs_diff(pixel(&settled, x, y)[0])))
        .sum::<f32>()
        / (SIZE * SIZE / 4) as f32;
    eprintln!("turning unevenly against settled: {apart:.2} levels");
    assert!(
        apart < 9.0,
        "the unevenly turning picture is {apart} levels from the settled one"
    );
}

#[test]
fn forgetting_history_draws_the_frame_after_a_camera_jump_as_a_first_frame() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let at = |camera: f32| {
        let mut list = list_of(vec![
            wall(-8.0),
            cube(Vec3::new(-2.5, 0.0, -5.0), 1.2, 2),
            cube(Vec3::new(1.5, 0.0, -5.0), 1.2, 3),
        ]);
        list.camera.position.x += camera;
        list.camera.look_at.x += camera;
        list.lens = LensEffects {
            motion_blur: Some(1.0),
            ..LensEffects::default()
        };
        list
    };
    let graphics = settings(AntiAliasing::Taa, MotionBlurQuality::High);
    let (before, after) = (at(0.0), at(1.5));
    // What the view after the jump looks like with nothing before it.
    let first = draw(&mut renderer(&device, &queue, &graphics), &after);
    let differing = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .zip(first.chunks_exact(4))
            .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 20))
            .count()
    };

    let mut kept = renderer(&device, &queue, &graphics);
    let mut forgotten = renderer(&device, &queue, &graphics);
    for _ in 0..4 {
        draw(&mut kept, &before);
        draw(&mut forgotten, &before);
    }
    let smeared = differing(&draw(&mut kept, &after));
    assert!(
        smeared > 200,
        "the jump streaks and ghosts while the history is kept: {smeared} pixels"
    );
    forgotten.forget_history();
    assert_eq!(
        draw(&mut forgotten, &after),
        first,
        "forgotten, it's drawn as a first frame"
    );
}
