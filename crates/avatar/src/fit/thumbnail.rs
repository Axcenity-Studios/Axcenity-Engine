//! A garment's thumbnail for the wardrobe browser: its pieces drawn from
//! the front on a clear background, lit from the upper left. A small
//! software rasterizer (a depth buffer and flat triangles), so importing
//! needs no GPU.

use std::path::Path;

use glam::{Vec2, Vec3};

use super::{FitError, GarmentMesh};

/// The garment drawn `size` pixels square, as RGBA bytes (rows from the
/// top). Looks at its front (the character faces +Z) and fills about nine
/// tenths of the picture.
pub fn thumbnail(garment: &GarmentMesh, size: u32) -> Vec<u8> {
    let size = size.max(8);
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let mut depth = vec![f32::NEG_INFINITY; (size * size) as usize];
    let (lo, hi) = garment.bounds();
    if !lo.is_finite() || !hi.is_finite() {
        return rgba;
    }
    let middle = (lo + hi) / 2.0;
    let extent = (hi - lo).truncate().max_element().max(1e-6);
    let scale = 0.9 * size as f32 / extent;
    let to_screen = |p: Vec3| {
        let x = (p.x - middle.x) * scale + size as f32 / 2.0;
        let y = size as f32 / 2.0 - (p.y - middle.y) * scale;
        Vec3::new(x, y, p.z)
    };
    let light = Vec3::new(-0.4, 0.6, 0.7).normalize();
    for piece in &garment.pieces {
        let colour = Vec3::new(piece.colour[0], piece.colour[1], piece.colour[2]);
        for t in piece.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|v| piece.positions[v as usize]);
            let normal = (b - a).cross(c - a).normalize_or_zero();
            // Two-sided: the inside of a sleeve is lit as its outside.
            let lit = 0.25 + 0.75 * normal.dot(light).abs();
            let shade = to_srgb(colour * lit);
            fill(
                [to_screen(a), to_screen(b), to_screen(c)],
                size,
                &mut depth,
                &mut rgba,
                shade,
            );
        }
    }
    rgba
}

fn to_srgb(linear: Vec3) -> [u8; 3] {
    linear.to_array().map(|c| {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.003_130_8 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    })
}

/// Fills a screen triangle (x, y in pixels, z the depth: nearer is
/// larger) where it is nearer than what's drawn.
fn fill(corners: [Vec3; 3], size: u32, depth: &mut [f32], rgba: &mut [u8], shade: [u8; 3]) {
    let [a, b, c] = corners;
    let area = edge(a.truncate(), b.truncate(), c.truncate());
    if area.abs() < 1e-9 {
        return;
    }
    let lo = a.min(b).min(c).truncate().floor().max(Vec2::ZERO);
    let hi = a
        .max(b)
        .max(c)
        .truncate()
        .ceil()
        .min(Vec2::splat(size as f32 - 1.0));
    for y in lo.y as u32..=hi.y.max(0.0) as u32 {
        for x in lo.x as u32..=hi.x.max(0.0) as u32 {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let wa = edge(b.truncate(), c.truncate(), p) / area;
            let wb = edge(c.truncate(), a.truncate(), p) / area;
            let wc = 1.0 - wa - wb;
            if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                continue;
            }
            let z = a.z * wa + b.z * wb + c.z * wc;
            let at = (y * size + x) as usize;
            if z <= depth[at] {
                continue;
            }
            depth[at] = z;
            rgba[at * 4..at * 4 + 3].copy_from_slice(&shade);
            rgba[at * 4 + 3] = 255;
        }
    }
}

fn edge(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    (b - a).perp_dot(p - a)
}

/// Writes RGBA bytes as a PNG file.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), FitError> {
    let write = || -> Result<(), String> {
        let image = image::RgbaImage::from_raw(width, height, rgba.to_vec())
            .ok_or("the picture's size doesn't match its pixels")?;
        image.save(path).map_err(|err| err.to_string())
    };
    write().map_err(|message| FitError::Write {
        path: path.display().to_string(),
        message,
    })
}
