//! The Assets panel's image thumbnails. Decoding a 2K–8K texture takes long
//! enough to stall a frame, so a few background threads read and shrink
//! them; a tile shows its icon until its thumbnail arrives.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use eframe::egui;

/// Thumbnails are scaled down to at most this many pixels across: enough
/// for the hover preview, which is larger than the tile.
pub const THUMBNAIL_PIXELS: u32 = 192;
/// Size of procedural shaded sphere swatches for untextured materials.
const SWATCH_PIXELS: u32 = 128;
/// At most this many images are decoded at once, leaving cores for the
/// editor itself.
const WORKERS: usize = 3;

/// What kind of file a thumbnail request points to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThumbnailKind {
    Image,
    TextureSet,
    Material,
}

type Request = (String, ThumbnailKind);
type Done = (String, Option<egui::ColorImage>);

#[derive(Default)]
pub struct Thumbnails {
    folder: Option<PathBuf>,
    /// By asset path, `None` for an image that can't be read.
    done: HashMap<String, Option<egui::TextureHandle>>,
    loading: HashSet<String>,
    /// Started on the first request. Dropping them stops the workers.
    channels: Option<(Sender<Request>, Receiver<Done>)>,
}

impl Thumbnails {
    /// Forgets every thumbnail, for a newly opened (or closed) project.
    pub fn open(&mut self, folder: Option<&Path>) {
        *self = Self {
            folder: folder.map(Path::to_owned),
            ..Self::default()
        };
    }

    /// Drops the thumbnail of `path`, so it's read again, after the file
    /// changed.
    pub fn forget(&mut self, path: &str) {
        self.done.remove(path);
    }

    /// Drops the thumbnails of files that are gone. It looks at the disk,
    /// not the panel's tiles: a texture set's images aren't tiles, but a
    /// terrain layer still shows them.
    pub fn forget_missing(&mut self) {
        let Some(folder) = &self.folder else {
            return;
        };
        self.done.retain(|path, _| folder.join(path).is_file());
    }

    /// Turns finished thumbnails into textures. Call once a frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some((_, results)) = &self.channels else {
            return;
        };
        for (path, image) in results.try_iter() {
            self.loading.remove(&path);
            let handle = image.map(|image| {
                ctx.load_texture(
                    format!("asset thumbnail {path}"),
                    image,
                    egui::TextureOptions::LINEAR,
                )
            });
            self.done.insert(path, handle);
        }
    }

    /// The thumbnail of `path`, asking for it the first time. A texture set
    /// shows its color map; a material shows its color map (tinted by its
    /// color) or a shaded swatch when it has no color texture.
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        path: &str,
        kind: ThumbnailKind,
    ) -> Option<egui::TextureHandle> {
        if let Some(done) = self.done.get(path) {
            return done.clone();
        }
        if self.loading.contains(path) {
            return None;
        }
        let folder = self.folder.clone()?;
        let (requests, _) = self
            .channels
            .get_or_insert_with(|| start_workers(ctx, folder));
        if requests.send((path.to_owned(), kind)).is_ok() {
            self.loading.insert(path.to_owned());
        }
        None
    }
}

fn start_workers(ctx: &egui::Context, folder: PathBuf) -> (Sender<Request>, Receiver<Done>) {
    let (request_tx, request_rx) = mpsc::channel::<Request>();
    let (done_tx, done_rx) = mpsc::channel();
    // One queue shared by the workers: whichever is free takes the next.
    let requests = Arc::new(Mutex::new(request_rx));
    for _ in 0..WORKERS {
        let requests = Arc::clone(&requests);
        let done = done_tx.clone();
        let folder = folder.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            loop {
                // The lock is only held while waiting, not while decoding.
                let next = requests.lock().map(|requests| requests.recv());
                let Ok(Ok((path, kind))) = next else {
                    return;
                };
                let image = load(&folder, &path, kind);
                if done.send((path, image)).is_err() {
                    return;
                }
                ctx.request_repaint();
            }
        });
    }
    (request_tx, done_rx)
}

fn load(folder: &Path, path: &str, kind: ThumbnailKind) -> Option<egui::ColorImage> {
    match kind {
        ThumbnailKind::Image => load_image(folder, path),
        ThumbnailKind::TextureSet => {
            let relative = axcenity_assets::safe_path(path)?;
            let set = axcenity_assets::read_texture_set(&folder.join(relative)).ok()?;
            let color_path = set.path(axcenity_assets::MapSlot::BaseColor)?;
            load_image(folder, color_path)
        }
        ThumbnailKind::Material => load_material(folder, path),
    }
}

fn load_image(folder: &Path, path: &str) -> Option<egui::ColorImage> {
    if axcenity_assets::is_hdr_sky_file(path) {
        let sky = axcenity_assets::read_sky_image(folder, path).ok()?;
        return shrink(sky.width, sky.height, &sky_rgba(&sky.rgb), THUMBNAIL_PIXELS);
    }
    let image = axcenity_assets::read_texture(folder, path).ok()?;
    shrink(image.width, image.height, image.rgba(), THUMBNAIL_PIXELS)
}

/// Resolves a `.material.json` file (including its `base` chain) and renders
/// its color texture (tinted by the material's color) or, when it has no
/// color texture, a shaded sphere swatch matching its PBR properties.
fn load_material(folder: &Path, path: &str) -> Option<egui::ColorImage> {
    let material = axcenity_assets::read_resolved_material(folder, path)?;
    if let Some(color_map) = &material.maps.color {
        let resolved_path = resolve_texture_ref(folder, &color_map.path);
        if let Some(mut image) = load_image(folder, &resolved_path) {
            if material.color != [1.0, 1.0, 1.0] {
                tint_image(&mut image, material.color);
            }
            return Some(image);
        }
    }
    Some(material_swatch(&material, SWATCH_PIXELS))
}

/// Resolves an `ax://...` asset ID to its project-relative path; plain paths
/// pass through unchanged.
fn resolve_texture_ref(folder: &Path, reference: &str) -> String {
    if let Ok(wanted) = reference.parse::<axcenity_assets::AssetId>() {
        let ids = axcenity_assets::asset_ids(folder);
        if let Some((path, _)) = ids.into_iter().find(|(_, id)| *id == wanted) {
            return path;
        }
    }
    reference.to_owned()
}

fn tint_image(image: &mut egui::ColorImage, tint: [f32; 3]) {
    let [tr, tg, tb] = tint.map(|c| c.clamp(0.0, 1.0));
    for pixel in &mut image.pixels {
        let [r, g, b, a] = pixel.to_srgba_unmultiplied();
        *pixel = egui::Color32::from_rgba_unmultiplied(
            (f32::from(r) * tr).round() as u8,
            (f32::from(g) * tg).round() as u8,
            (f32::from(b) * tb).round() as u8,
            a,
        );
    }
}

/// Shaded sphere preview for an untextured material (color, metalness,
/// roughness, emission, opacity, water).
fn material_swatch(material: &axcenity_assets::StandardMaterial, size: u32) -> egui::ColorImage {
    let srgb_to_lin = |c: f32| {
        let c = c.clamp(0.0, 1.0);
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let lin_to_srgb = |c: f32| {
        let c = c.max(0.0);
        // Simple Reinhard roll-off so bright emissive/specular highlights stay pleasant.
        let mapped = c / (1.0 + c * 0.25);
        let mapped = mapped.min(1.0);
        let s = if mapped <= 0.0031308 {
            mapped * 12.92
        } else {
            1.055 * mapped.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };

    let (base_srgb, metalness, roughness, opacity) = if let Some(water) = material.water {
        (water.deep_color, 0.05, 0.06, 0.72)
    } else {
        (
            material.color,
            material.metalness.clamp(0.0, 1.0),
            material.roughness.clamp(0.04, 1.0),
            material.opacity.clamp(0.0, 1.0),
        )
    };
    let base = base_srgb.map(srgb_to_lin);
    let emissive = material
        .emissive_color
        .map(|c| srgb_to_lin(c) * material.emissive_strength.max(0.0));

    // Normalized studio key + fill light directions.
    let key = {
        let (x, y, z) = (-0.52_f32, 0.62_f32, 0.58_f32);
        let len = (x * x + y * y + z * z).sqrt();
        [x / len, y / len, z / len]
    };
    let fill = {
        let (x, y, z) = (0.65_f32, -0.35_f32, 0.67_f32);
        let len = (x * x + y * y + z * z).sqrt();
        [x / len, y / len, z / len]
    };
    let view = [0.0_f32, 0.0, 1.0];
    let half = {
        let h = [key[0] + view[0], key[1] + view[1], key[2] + view[2]];
        let len = (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
        [h[0] / len, h[1] / len, h[2] / len]
    };

    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    let radius = 0.78_f32;
    let feather = 2.0 / size as f32;
    let transparent = opacity < 0.99;

    for py in 0..size {
        let ny = 1.0 - (py as f32 + 0.5) / size as f32 * 2.0;
        for px in 0..size {
            let nx = (px as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let r2 = nx * nx + ny * ny;
            let dist = r2.sqrt();
            let coverage = ((radius - dist) / feather + 0.5).clamp(0.0, 1.0);

            // Subtle backdrop; faint checkerboard when transparent so opacity reads clearly.
            let bg_val = if transparent {
                let checker = ((px / 12) + (py / 12)) % 2 == 0;
                if checker { 0.025 } else { 0.055 }
            } else {
                0.018 + (ny * 0.5 + 0.5) * 0.008
            };
            let bg = [bg_val, bg_val, bg_val * 1.06];

            if coverage <= 0.0 {
                pixels.extend(bg.map(lin_to_srgb));
                pixels.push(255);
                continue;
            }

            let sx = (nx / radius).clamp(-1.0, 1.0);
            let sy = (ny / radius).clamp(-1.0, 1.0);
            let sz = (1.0 - sx * sx - sy * sy).max(0.0).sqrt();
            let n = [sx, sy, sz];

            let ndl = (n[0] * key[0] + n[1] * key[1] + n[2] * key[2]).max(0.0);
            let ndf = (n[0] * fill[0] + n[1] * fill[1] + n[2] * fill[2]).max(0.0);
            let ndh = (n[0] * half[0] + n[1] * half[1] + n[2] * half[2]).max(0.0);
            let ndv = n[2].max(0.0);

            let diffuse_weight = 1.0 - metalness;
            let f0 = [
                0.04 + (base[0] - 0.04) * metalness,
                0.04 + (base[1] - 0.04) * metalness,
                0.04 + (base[2] - 0.04) * metalness,
            ];
            let fresnel = (1.0 - ndv).powi(5);
            let shininess =
                (2.0 / (roughness * roughness * roughness * roughness) - 2.0).clamp(4.0, 256.0);
            let spec_norm = (shininess + 8.0) / 28.0;
            let spec = ndh.powf(shininess) * spec_norm * (1.0 - roughness * 0.55);
            let env_grad = (n[1] * 0.5 + 0.5).clamp(0.0, 1.0);
            let env = 0.14 + env_grad * 0.34;
            let rim = (1.0 - ndv).powf(3.0) * 0.22;

            let sphere: [f32; 3] = std::array::from_fn(|c| {
                let f = f0[c] + (1.0 - f0[c]) * fresnel;
                let diff = base[c] * diffuse_weight * (ndl * 0.95 + ndf * 0.25 + 0.14);
                let refl = f * (spec * 1.15 + env * (1.0 - roughness * 0.65) + rim);
                diff + refl + emissive[c]
            });

            let alpha = coverage * opacity.max(0.15);
            let out: [f32; 3] = std::array::from_fn(|c| bg[c] * (1.0 - alpha) + sphere[c] * alpha);
            pixels.extend(out.map(lin_to_srgb));
            pixels.push(255);
        }
    }

    egui::ColorImage::from_rgba_unmultiplied([size as usize, size as usize], &pixels)
}

/// A sky image's linear light (red, green, blue, brighter than 1 where the
/// sun is) as sRGB bytes for a thumbnail: each channel rolled off toward
/// white (Reinhard), so the sun doesn't wash the rest out.
fn sky_rgba(rgb: &[f32]) -> Vec<u8> {
    let byte = |light: f32| {
        let light = (light.max(0.0) / (1.0 + light.max(0.0))).min(1.0);
        let srgb = if light <= 0.0031308 {
            light * 12.92
        } else {
            1.055 * light.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round() as u8
    };
    rgb.as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[red, green, blue]| [byte(red), byte(green), byte(blue), 255])
        .collect()
}

/// `rgba` (`width` × `height`) scaled down to at most `most` pixels across.
/// Each pixel is the average of the block it covers, so a detailed texture
/// still reads as a whole instead of as noise. `None` for an empty or short
/// image.
pub fn shrink(width: u32, height: u32, rgba: &[u8], most: u32) -> Option<egui::ColorImage> {
    if width == 0 || height == 0 || rgba.len() < (width * height * 4) as usize {
        return None;
    }
    let scale = (most as f32 / width.max(height) as f32).min(1.0);
    let (w, h) = (
        ((width as f32 * scale).round() as u32).max(1),
        ((height as f32 * scale).round() as u32).max(1),
    );
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let (y0, y1) = span(y, h, height);
        for x in 0..w {
            let (x0, x1) = span(x, w, width);
            let mut sum = [0u64; 4];
            for sy in y0..y1 {
                let row = (sy * width) as usize * 4;
                for pixel in rgba[row + x0 as usize * 4..row + x1 as usize * 4]
                    .as_chunks::<4>()
                    .0
                {
                    for (total, &channel) in sum.iter_mut().zip(pixel) {
                        *total += u64::from(channel);
                    }
                }
            }
            let count = u64::from((x1 - x0) * (y1 - y0));
            pixels.extend(sum.map(|total| (total / count) as u8));
        }
    }
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w as usize, h as usize],
        &pixels,
    ))
}

/// The source pixels `[start, end)` that output pixel `i` of `out` covers,
/// never empty.
fn span(i: u32, out: u32, source: u32) -> (u32, u32) {
    let start = i * source / out;
    let end = ((i + 1) * source / out).clamp(start + 1, source);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_shrink_and_keep_their_shape() {
        let rgba = vec![255; 200 * 100 * 4];
        let image = shrink(200, 100, &rgba, 96).unwrap();
        assert_eq!(image.size, [96, 48]);
        assert!(shrink(0, 10, &[], 96).is_none());
    }

    #[test]
    fn thumbnails_are_kept_while_their_files_exist() {
        let folder = std::env::temp_dir().join(format!("axcenity-thumbs-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("assets/set")).unwrap();
        std::fs::write(folder.join("assets/set/color.png"), b"").unwrap();
        let mut thumbnails = Thumbnails::default();
        thumbnails.open(Some(&folder));
        thumbnails.done.insert("assets/set/color.png".into(), None);
        thumbnails.done.insert("assets/gone.png".into(), None);
        thumbnails.forget_missing();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(thumbnails.done.contains_key("assets/set/color.png"));
        assert!(!thumbnails.done.contains_key("assets/gone.png"));
    }

    #[test]
    fn shrinking_averages_what_each_pixel_covers() {
        // A 2×2 checker of black and white becomes one grey pixel.
        let rgba = [
            0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
        ];
        let image = shrink(2, 2, &rgba, 1).unwrap();
        assert_eq!(image.pixels[0], egui::Color32::from_gray(127));
    }

    #[test]
    fn material_thumbnail_shows_its_color_texture_and_inherits_base_maps() {
        let folder =
            std::env::temp_dir().join(format!("axcenity-thumbs-mat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("assets/textures")).unwrap();
        std::fs::create_dir_all(folder.join("assets/materials")).unwrap();

        // Write a bright green 4x4 PNG texture.
        let png = image::RgbaImage::from_pixel(4, 4, image::Rgba([40, 220, 80, 255]));
        png.save(folder.join("assets/textures/grid.png")).unwrap();

        let base = axcenity_assets::MaterialFile {
            color: Some([1.0, 1.0, 1.0]),
            maps: axcenity_assets::MaterialMaps {
                color: Some(axcenity_assets::MapFile::of("assets/textures/grid.png")),
                ..Default::default()
            },
            ..Default::default()
        };
        axcenity_assets::write_material_file(
            &folder.join("assets/materials/base.material.json"),
            &base,
        )
        .unwrap();

        // A child material inherits the color map and tints it half as bright.
        let child = axcenity_assets::MaterialFile {
            base: Some("assets/materials/base.material.json".into()),
            color: Some([0.5, 0.5, 0.5]),
            ..Default::default()
        };
        axcenity_assets::write_material_file(
            &folder.join("assets/materials/child.material.json"),
            &child,
        )
        .unwrap();

        // Untextured red material produces a shaded swatch instead of None.
        let solid = axcenity_assets::MaterialFile {
            color: Some([0.9, 0.1, 0.1]),
            metalness: Some(0.2),
            roughness: Some(0.3),
            ..Default::default()
        };
        axcenity_assets::write_material_file(
            &folder.join("assets/materials/solid.material.json"),
            &solid,
        )
        .unwrap();

        let base_img = load(
            &folder,
            "assets/materials/base.material.json",
            ThumbnailKind::Material,
        )
        .expect("textured material produces a thumbnail");
        assert_eq!(base_img.size, [4, 4]);
        assert_eq!(
            base_img.pixels[0],
            egui::Color32::from_rgba_unmultiplied(40, 220, 80, 255)
        );

        let child_img = load(
            &folder,
            "assets/materials/child.material.json",
            ThumbnailKind::Material,
        )
        .expect("derived material inherits base color texture");
        assert_eq!(
            child_img.pixels[0],
            egui::Color32::from_rgba_unmultiplied(20, 110, 40, 255)
        );

        let solid_img = load(
            &folder,
            "assets/materials/solid.material.json",
            ThumbnailKind::Material,
        )
        .expect("untextured material produces a shaded swatch");
        assert_eq!(
            solid_img.size,
            [SWATCH_PIXELS as usize, SWATCH_PIXELS as usize]
        );
        let center = solid_img.pixels
            [(SWATCH_PIXELS as usize / 2) * SWATCH_PIXELS as usize + SWATCH_PIXELS as usize / 2];
        assert!(
            center.r() > center.g() && center.r() > center.b(),
            "center of red material swatch should be red: {center:?}"
        );

        let _ = std::fs::remove_dir_all(&folder);
    }
}
