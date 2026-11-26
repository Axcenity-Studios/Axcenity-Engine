//! A frame recorded in parts (`renderer/encoders.rs`): the same commands
//! in the same order as from one encoder, so the same picture.

use super::*;
use crate::{RayTracedAmbientOcclusion, RayTracedReflections, RayTracedShadows};

/// Blocks and balls over a floor, under a sun that casts shadows: shadow
/// views, the depth prepass and the scene each have draws of their own.
fn scene() -> DrawList {
    let mut floor = item(Shape::Block, Vec3::new(0.0, -3.5, -9.0), 1.0);
    floor.transform = Mat4::from_translation(Vec3::new(0.0, -3.5, -9.0))
        * Mat4::from_scale(Vec3::new(30.0, 1.0, 30.0));
    let mut items = vec![floor];
    items.extend((0..24).map(|i| {
        let shape = if i % 3 == 0 {
            Shape::Sphere
        } else {
            Shape::Block
        };
        let position = Vec3::new((i % 6) as f32 * 1.6 - 4.0, (i / 6) as f32 - 2.0, -9.0);
        let mut item = item(shape, position, if i % 5 == 2 { 0.5 } else { 1.0 });
        item.material.base_color = [(i % 4) as f32 / 3.0, (i % 5) as f32 / 4.0, 0.6];
        item
    }));
    let mut list = list_of(items);
    list.shadows.enabled = true;
    list
}

/// `list` drawn a few frames, as pixels, and how many command buffers the
/// last frame was.
fn drawn(renderer: &mut TextureRenderer, list: &DrawList) -> (Vec<u8>, u32) {
    for _ in 0..3 {
        renderer.render(list).unwrap();
    }
    (renderer.read_pixels(), renderer.stats().command_buffers)
}

#[test]
fn a_frame_recorded_in_parts_is_the_picture_one_encoder_draws() {
    let Some((device, queue)) = test_device() else {
        return;
    };
    let settings = with_occlusion(crate::AmbientOcclusion::High);
    let picture_of = |in_parts: bool| {
        let mut renderer = TextureRenderer::new(&device, &queue, 128, 96);
        renderer.set_graphics(&settings);
        renderer.set_frame_in_parts(in_parts);
        drawn(&mut renderer, &scene())
    };
    let (one, buffers) = picture_of(false);
    assert_eq!(buffers, 1);
    let colors: std::collections::HashSet<&[u8]> = one.chunks(4).collect();
    assert!(colors.len() > 20, "{} colors", colors.len());
    let (parts, buffers) = picture_of(true);
    // The shadow views, the prepass and the scene are parts of their own.
    assert!(buffers >= 3, "{buffers} command buffers");
    assert!(one == parts, "the pictures differ");
}

#[test]
fn a_ray_traced_frame_recorded_in_parts_is_the_picture_one_encoder_draws() {
    let Some((device, queue)) = ray_test_device() else {
        return;
    };
    let settings = crate::GraphicsSettings {
        ray_traced_shadows: Some(RayTracedShadows::High),
        ray_traced_ambient_occlusion: Some(RayTracedAmbientOcclusion::High),
        ray_traced_reflections: Some(RayTracedReflections::High),
        ..crate::GraphicsSettings::default()
    };
    let picture_of = |in_parts: bool| {
        let mut renderer = TextureRenderer::new(&device, &queue, 128, 96);
        renderer.set_graphics(&settings);
        renderer.set_frame_in_parts(in_parts);
        let picture = drawn(&mut renderer, &scene());
        assert!(renderer.stats().ray_tracing.instances > 0);
        picture
    };
    let (one, _) = picture_of(false);
    let (parts, buffers) = picture_of(true);
    assert!(buffers >= 3, "{buffers} command buffers");
    assert!(one == parts, "the pictures differ");
}
