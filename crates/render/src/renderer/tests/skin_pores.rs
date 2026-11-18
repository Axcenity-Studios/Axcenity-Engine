//! Skin's pores and fine lines (R5 review): detail finer than the skin's
//! maps hold, drawn by the Skin model itself in the shading normal and a
//! little in the albedo, where a pore spans a few pixels; gone, bit for bit
//! within a level, where it would be smaller than a pixel; and as strong as
//! the normal map's scale (`detail_strength`).

use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};
use crate::{GraphicsSettings, Shading, SkinShading};

/// The frame size, square.
const SIZE: u32 = 192;

/// A normal map flat everywhere: what it adds is nothing, so what changes
/// is the Skin model's own detail.
struct Flat(Vec<u8>);

impl TextureSource for Flat {
    fn key(&self) -> u64 {
        0x5e_f1a7
    }
    fn size(&self) -> (u32, u32) {
        (4, 4)
    }
    fn rgba(&self) -> &[u8] {
        &self.0
    }
}

/// A skin square 4 studs wide facing the camera, its flat normal map at
/// `strength`, its texture coordinates scaled by `uv_scale` (how many of
/// the skin's texture spaces it spans), under a sun from the upper right.
fn square(strength: f32, uv_scale: f32) -> Vec<u8> {
    let Some((device, queue)) = test_device() else {
        return Vec::new();
    };
    let mut square = item(Shape::Block, Vec3::ZERO, 1.0);
    square.transform = Mat4::from_scale(Vec3::new(4.0, 4.0, 0.1));
    square.material.base_color = [0.74, 0.53, 0.43];
    square.material.roughness = 0.45;
    square.material.uv_scale = [uv_scale, uv_scale];
    square.material.maps.normal = Some(NormalMap {
        texture: Arc::new(Flat([128, 128, 255, 255].repeat(16))),
        scale: strength,
        flip_green: false,
    });
    square.material.shading = Shading::Skin(SkinShading::default());
    let mut list = list_of(vec![square]);
    list.camera = Camera {
        position: Vec3::new(0.0, 0.0, 5.0),
        look_at: Vec3::ZERO,
        fov_y_degrees: 40.0,
    };
    list.sun.direction = Vec3::new(0.7, 0.4, 0.6).normalize();
    list.sun.intensity = 2.0;
    list.shadows.enabled = false;
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(AntiAliasing::Off),
        ambient_occlusion: Some(AmbientOcclusion::Off),
        ..GraphicsSettings::preset(Quality::High)
    });
    renderer.render(&list).unwrap();
    renderer.read_pixels()
}

/// Each pixel's brightness (the sum of its channels) over the middle of
/// the frame, `half` pixels each way, row by row.
fn middle(pixels: &[u8], half: usize) -> Vec<Vec<f32>> {
    let size = SIZE as usize;
    let (from, to) = (size / 2 - half, size / 2 + half);
    (from..to)
        .map(|y| {
            (from..to)
                .map(|x| {
                    let p = &pixels[(y * size + x) * 4..][..3];
                    p.iter().map(|&c| f32::from(c)).sum()
                })
                .collect()
        })
        .collect()
}

/// How much `rows` vary from pixel to pixel: the root mean square of each
/// pixel's difference from its 7 × 7 neighbourhood's mean (the square's
/// own gradual shading left out).
fn spread(rows: &[Vec<f32>]) -> f32 {
    let n = rows.len();
    let mut sum = 0.0;
    let mut count = 0.0;
    for y in 3..n - 3 {
        for x in 3..n - 3 {
            let mut mean = 0.0;
            for dy in 0..7 {
                for dx in 0..7 {
                    mean += rows[y + dy - 3][x + dx - 3];
                }
            }
            sum += (rows[y][x] - mean / 49.0).powi(2);
            count += 1.0;
        }
    }
    (sum / count).sqrt()
}

/// How many pixels are pits: darker than each of their eight neighbours
/// and than their 7 × 7 neighbourhood's mean by more than `by`.
fn pits(rows: &[Vec<f32>], by: f32) -> usize {
    let n = rows.len();
    let mut count = 0;
    for y in 3..n - 3 {
        for x in 3..n - 3 {
            let v = rows[y][x];
            let mut sum = 0.0;
            let mut lowest = true;
            for dy in -3i32..=3 {
                for dx in -3i32..=3 {
                    let w = rows[(y as i32 + dy) as usize][(x as i32 + dx) as usize];
                    sum += w;
                    if dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0) && w <= v {
                        lowest = false;
                    }
                }
            }
            if lowest && v < sum / 49.0 - by {
                count += 1;
            }
        }
    }
    count
}

/// Close up (a pore cell about 6 pixels across: the square spans 0.032 of
/// the skin's texture space), skin with its detail at full strength shows
/// many small dark pits; with the normal map's strength at 0 (the Skin
/// value `detail_strength` 0) it is as even as the square is flat.
#[test]
fn skin_shows_pores_close_up_as_strong_as_its_detail() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let none = middle(&square(0.0, 0.032), 60);
    let full = middle(&square(1.0, 0.032), 60);
    let double = middle(&square(2.0, 0.032), 60);
    let (none_spread, full_spread, double_spread) = (spread(&none), spread(&full), spread(&double));
    let (none_pits, full_pits) = (pits(&none, 3.0), pits(&full, 3.0));
    eprintln!(
        "spread: none {none_spread:.2}, full {full_spread:.2}, double {double_spread:.2}; pits: none {none_pits}, full {full_pits}"
    );
    assert!(none_spread < 0.5, "flat skin varies by {none_spread}");
    assert!(
        full_spread > 1.5,
        "skin close up varies by only {full_spread}: no pores"
    );
    assert!(
        double_spread > full_spread * 1.3,
        "detail_strength 2 isn't stronger"
    );
    // About 120 × 120 pixels of 6-pixel cells (400), a pore in half of them.
    assert!(none_pits < 5, "flat skin has {none_pits} pits");
    assert!(
        (60..=400).contains(&full_pits),
        "{full_pits} pits in 120 × 120 pixels"
    );
}

/// Far away (the square spans the whole texture space: a pore cell is a
/// tenth of a pixel), the detail is gone, so it can't alias: full strength
/// draws what strength 0 draws, within a level.
#[test]
fn skin_pores_fade_out_where_they_would_be_smaller_than_a_pixel() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let none = square(0.0, 1.0);
    let full = square(1.0, 1.0);
    let most = none
        .iter()
        .zip(&full)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    assert!(
        most <= 1,
        "far away the detail still changes a pixel by {most}"
    );
}
