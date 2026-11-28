//! A cut-out's coverage kept from near to far under TAA (leaves, fences):
//! its mip levels average a leaf atlas's hard alpha into a grey under the
//! cutoff, so a far bush tested against the cutoff alone thinned to nothing
//! (or, over it, filled in solid). Under TAA its averaged alpha is kept as
//! coverage by hashed thresholds that TAA averages (`cut_out_blur` in
//! shader.wgsl): the same card drawn small must cover about the share of its
//! square it covers drawn large. Without TAA the cutoff stays as it was:
//! MSAA's alpha to coverage and a still hash dithered every far canopy.

use std::sync::Arc;

use glam::Vec3;

use super::card_set::{Made, card, dark_scene, evidence, on_screen, rgb_at, sum};
use super::*;
use crate::GraphicsSettings;
use crate::graphics::{AmbientOcclusion, AntiAliasing, MotionBlurQuality, Quality};

/// The frame size, square.
const SIZE: u32 = 128;
/// The lens, degrees high.
const FOV: f32 = 40.0;

/// A leaf atlas's alpha: 128² texels, `count` round leaves (discs of
/// radius 3 to 6 texels, a texel of soft edge) scattered over it (80 cover
/// about a quarter, 150 about half), whole inside and clear outside.
fn leaves(key: u64, count: usize) -> Arc<dyn TextureSource> {
    let size = 128u32;
    let mut state = 0x2545_f491_u32;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state as f32 / u32::MAX as f32
    };
    let discs: Vec<(f32, f32, f32)> = (0..count)
        .map(|_| {
            (
                next() * size as f32,
                next() * size as f32,
                3.0 + 3.0 * next(),
            )
        })
        .collect();
    Made::shared(key, size, move |x, y| {
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        let inside = discs
            .iter()
            .map(|&(cx, cy, r)| {
                // Wrapped, so the atlas tiles like the sampler repeats it.
                let dx = (px - cx).abs().min(size as f32 - (px - cx).abs());
                let dy = (py - cy).abs().min(size as f32 - (py - cy).abs());
                (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0)
            })
            .fold(0.0, f32::max);
        [255, 255, 255, (inside * 255.0).round() as u8]
    })
}

fn settings(anti_aliasing: AntiAliasing) -> GraphicsSettings {
    GraphicsSettings {
        anti_aliasing: Some(anti_aliasing),
        motion_blur: Some(MotionBlurQuality::Off),
        ambient_occlusion: Some(AmbientOcclusion::Off),
        contact_shadows: Some(false),
        volumetric_fog: Some(false),
        ..GraphicsSettings::preset(Quality::High)
    }
}

/// The card seen from `distance`: cut out by `count` leaves at `cutoff`,
/// or whole when `cutoff` is `None`.
fn scene(distance: f32, count: usize, cutoff: Option<f32>) -> DrawList {
    let mut material = Material {
        base_color: [0.6; 3],
        roughness: 0.8,
        ..Material::default()
    };
    if let Some(cutoff) = cutoff {
        material.base_color_texture = Some(leaves(0x1eaf5 + count as u64, count));
        material.alpha_cutoff = cutoff;
    }
    dark_scene(
        vec![card(Vec3::ZERO, [1.0, 1.0], material)],
        distance,
        FOV,
        Vec3::new(0.1, 0.2, 1.0),
    )
}

/// An sRGB byte as linear light: a pixel a quarter covered is a quarter as
/// bright in light, not in its bytes (and the card is lit under the Classic
/// curve's 0.6, which it leaves as it is).
fn linear(byte: f32) -> f32 {
    let value = byte / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// The light over the card's square (a pixel round it), averaged over
/// the last 16 of 48 frames (TAA's settled picture).
fn brightness(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anti_aliasing: AntiAliasing,
    list: &DrawList,
    name: &str,
) -> f32 {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(&settings(anti_aliasing));
    let corner_a = on_screen(list, Vec3::new(-0.5, 0.5, 0.0), SIZE);
    let corner_b = on_screen(list, Vec3::new(0.5, -0.5, 0.0), SIZE);
    let (x0, y0) = (corner_a.x.floor() as u32 - 1, corner_a.y.floor() as u32 - 1);
    let (x1, y1) = (corner_b.x.ceil() as u32 + 1, corner_b.y.ceil() as u32 + 1);
    let mut total = 0.0;
    for frame in 0..48 {
        renderer.render(list).unwrap();
        if frame < 32 {
            continue;
        }
        let pixels = renderer.read_pixels();
        if frame == 47 {
            evidence(name, &pixels, SIZE, SIZE);
        }
        for y in y0..=y1.min(SIZE - 1) {
            for x in x0..=x1.min(SIZE - 1) {
                total += sum(rgb_at(&pixels, SIZE, x, y).map(linear));
            }
        }
    }
    total / 16.0
}

/// The share of the card's square the leaves cover from `distance`: its
/// brightness cut out against its brightness whole.
fn coverage(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anti_aliasing: AntiAliasing,
    distance: f32,
    (count, cutoff): (usize, f32),
) -> f32 {
    let name = format!("cut-out-coverage-{anti_aliasing:?}-{count}-{cutoff}-{distance}");
    let cut = brightness(
        device,
        queue,
        anti_aliasing,
        &scene(distance, count, Some(cutoff)),
        &name,
    );
    let whole = brightness(
        device,
        queue,
        anti_aliasing,
        &scene(distance, count, None),
        &format!("{name}-whole"),
    );
    cut / whole.max(1.0)
}

/// Near, the 128-texel card spans about 120 pixels (read at full size);
/// far, about 6 (4 to 5 mip levels down, where every texel is the
/// atlas's average).
const NEAR: f32 = 1.46;
const FAR: f32 = 29.0;

fn assert_kept(anti_aliasing: AntiAliasing, (count, cutoff): (usize, f32)) {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let near = coverage(&device, &queue, anti_aliasing, NEAR, (count, cutoff));
    let far = coverage(&device, &queue, anti_aliasing, FAR, (count, cutoff));
    eprintln!("{anti_aliasing:?}, {count} leaves, cutoff {cutoff}: near {near:.3}, far {far:.3}");
    assert!(
        near > 0.15 && near < 0.65,
        "the leaves cover a share of the card near: {near:.3}"
    );
    assert!(
        (far / near - 1.0).abs() < 0.35,
        "{anti_aliasing:?}: far off the leaves cover {far:.3} of the square, near {near:.3}"
    );
}

/// A sparse atlas (a quarter covered) at glTF's default cutoff: its mips
/// average under the cutoff, so far off it thinned out.
const SPARSE: (usize, f32) = (80, 0.5);
/// A dense atlas (about half covered) at a low cutoff: its mips average over
/// it, so far off it filled in solid.
const DENSE: (usize, f32) = (150, 0.3);

#[test]
fn a_far_sparse_cut_out_keeps_its_coverage_under_taa() {
    assert_kept(AntiAliasing::Taa, SPARSE);
}

#[test]
fn a_far_dense_cut_out_keeps_its_coverage_under_taa() {
    assert_kept(AntiAliasing::Taa, DENSE);
}
