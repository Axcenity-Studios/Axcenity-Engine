//! The test card set and cloth swatch the Hair and Cloth models and the
//! alpha cards are tested with (CO7, RA7): small textures made in code,
//! laid out as `docs/rendering/hair.md` and `cloth.md` say an atlas is.
//! Not a hairstyle or a garment: those are content
//! (`assets/characters/realistic/items`).

use std::sync::Arc;

use glam::{Mat4, Vec3};

use super::*;
use crate::{ClothShading, HairShading, Shading};

/// A square texture made by a function of its texel, under a key of its
/// own.
pub(super) struct Made {
    key: u64,
    size: u32,
    rgba: Vec<u8>,
}

impl Made {
    /// The texture, shared.
    pub(super) fn shared(
        key: u64,
        size: u32,
        texel: impl Fn(u32, u32) -> [u8; 4],
    ) -> Arc<dyn TextureSource> {
        let rgba = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .flat_map(|(x, y)| texel(x, y))
            .collect();
        Arc::new(Made { key, size, rgba })
    }
}

impl TextureSource for Made {
    fn key(&self) -> u64 {
        self.key
    }
    fn size(&self) -> (u32, u32) {
        (self.size, self.size)
    }
    fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// A white texture whose alpha is `stripes` vertical stripes across U,
/// clear and whole in turn (a card of a few broad strands): what a
/// cut-out-aware shadow shows as stripes and a slab doesn't.
pub(super) fn striped(key: u64, stripes: u32) -> Arc<dyn TextureSource> {
    let size = 64;
    Made::shared(key, size, move |x, _| {
        let whole = (x * stripes * 2 / size).is_multiple_of(2);
        [255, 255, 255, if whole { 255 } else { 0 }]
    })
}

/// A white texture whose alpha ramps from clear on the left to whole on
/// the right: a card's soft edge, whose coverage must come out graded.
pub(super) fn ramp(key: u64) -> Arc<dyn TextureSource> {
    let size = 64;
    Made::shared(key, size, move |x, _| {
        let alpha = (x as f32 + 0.5) / size as f32;
        [255, 255, 255, (alpha * 255.0).round() as u8]
    })
}

/// A white texture with every texel `alpha`.
pub(super) fn flat_alpha(key: u64, alpha: u8) -> Arc<dyn TextureSource> {
    Made::shared(key, 4, move |_, _| [255, 255, 255, alpha])
}

/// The hair atlas's data map for a card whose strands run down it (+V):
/// red root (top, 0) to tip (bottom, 1), green a value per strand
/// (columns of 4 texels), blue the streak mask (the left half), alpha the
/// strands' occlusion (none).
pub(super) fn hair_data(key: u64) -> Arc<dyn TextureSource> {
    let size = 64;
    Made::shared(key, size, move |x, y| {
        let along = (y as f32 + 0.5) / size as f32;
        let strand = ((x / 4) * 97 % 16) as f32 / 15.0;
        let streak = if x < size / 2 { 255 } else { 0 };
        [
            (along * 255.0).round() as u8,
            (strand * 255.0).round() as u8,
            streak,
            255,
        ]
    })
}

/// A flow map pointing every texel along `direction` (U, V) in the card's
/// tangent space.
pub(super) fn flow(key: u64, direction: [f32; 2]) -> Arc<dyn TextureSource> {
    let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0).round() as u8;
    Made::shared(key, 4, move |_, _| {
        [byte(direction[0]), byte(direction[1]), 128, 255]
    })
}

/// Hair as made: `colour` (sRGB) and `roughness`, every value neutral but
/// `change`'s.
pub(super) fn hair(
    colour: [f32; 3],
    roughness: f32,
    change: impl FnOnce(&mut HairShading),
) -> Material {
    let mut shading = HairShading::default();
    change(&mut shading);
    Material {
        base_color: colour,
        roughness,
        shading: Shading::Hair(shading),
        ..Material::default()
    }
}

/// The cloth swatch: a mid-grey fabric (sRGB 0.45) of roughness 0.8 with
/// `change`'s sheen and anisotropy.
pub(super) fn cloth(change: impl FnOnce(&mut ClothShading)) -> Material {
    let mut shading = ClothShading::default();
    change(&mut shading);
    Material {
        base_color: [0.45; 3],
        roughness: 0.8,
        shading: Shading::Cloth(shading),
        ..Material::default()
    }
}

/// A flat card of `size` (width, height) studs facing +Z at `center`,
/// drawn from both sides, `material` on it: U across, V down.
pub(super) fn card(center: Vec3, size: [f32; 2], material: Material) -> DrawItem {
    DrawItem {
        transform: Mat4::from_scale_rotation_translation(
            Vec3::new(size[0], size[1], 1.0),
            glam::Quat::IDENTITY,
            center,
        ),
        material: Material {
            double_sided: true,
            ..material
        },
        ..item(Shape::Mesh(Arc::new(CardMesh)), Vec3::ZERO, 1.0)
    }
}

/// A unit square in the XY plane facing +Z, its texture's U along +X and
/// V down (-Y), with tangents (the hair model reads the strand's
/// direction from them).
pub(super) struct CardMesh;

impl MeshSource for CardMesh {
    fn key(&self) -> u64 {
        0xca_4d
    }
    fn positions(&self) -> &[[f32; 3]] {
        &[
            [-0.5, 0.5, 0.0],
            [0.5, 0.5, 0.0],
            [0.5, -0.5, 0.0],
            [-0.5, -0.5, 0.0],
        ]
    }
    fn normals(&self) -> &[[f32; 3]] {
        &[[0.0, 0.0, 1.0]; 4]
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    }
    fn tangents(&self) -> &[[f32; 4]] {
        // cross(normal, +X) is +Y; V runs down, so w is -1.
        &[[1.0, 0.0, 0.0, -1.0]; 4]
    }
    fn indices(&self) -> &[u32] {
        &[0, 3, 2, 0, 2, 1]
    }
}

/// A ball of `radius` at `center` with `material`: the built-in sphere,
/// whose U runs around it and V from its top down, so a hair material's
/// strands run down its meridians from a root at the top.
pub(super) fn ball(radius: f32, center: Vec3, material: Material) -> DrawItem {
    DrawItem {
        transform: Mat4::from_scale_rotation_translation(
            Vec3::splat(radius * 2.0),
            glam::Quat::IDENTITY,
            center,
        ),
        material,
        ..item(Shape::Sphere, Vec3::ZERO, 1.0)
    }
}

/// `items` in the dark (no sky, no ambient light, no shadows), seen from
/// `distance` along +Z through a lens `fov` degrees high, lit by a sun
/// from `sun`.
pub(super) fn dark_scene(items: Vec<DrawItem>, distance: f32, fov: f32, sun: Vec3) -> DrawList {
    let mut list = list_of(items);
    list.camera = Camera {
        position: Vec3::new(0.0, 0.0, distance),
        look_at: Vec3::ZERO,
        fov_y_degrees: fov,
    };
    list.sun.direction = sun.normalize();
    list.sun.intensity = 2.0;
    list.shadows.enabled = false;
    list
}

/// Where `point` is drawn on a `size`-pixel square frame of `list`.
pub(super) fn on_screen(list: &DrawList, point: Vec3, size: u32) -> glam::Vec2 {
    let clip = view_projection(list, 1.0) * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    glam::Vec2::new(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * size as f32
}

/// The pixel at (`x`, `y`) of a `size`-pixel square frame, as linear-ish
/// floats (the bytes as they are).
pub(super) fn rgb_at(pixels: &[u8], size: u32, x: u32, y: u32) -> [f32; 3] {
    let at = ((y * size + x) * 4) as usize;
    [
        f32::from(pixels[at]),
        f32::from(pixels[at + 1]),
        f32::from(pixels[at + 2]),
    ]
}

pub(super) fn sum(rgb: [f32; 3]) -> f32 {
    rgb[0] + rgb[1] + rgb[2]
}

/// Saves `pixels` (RGBA, `width` × `height`) as `<name>.ppm` in the folder
/// `AXCENITY_RENDER_EVIDENCE` names, when it names one: the evidence
/// pictures of `docs/development/character-studio/refinement/wave4/`.
pub(super) fn evidence(name: &str, pixels: &[u8], width: u32, height: u32) {
    let Some(folder) = std::env::var_os("AXCENITY_RENDER_EVIDENCE") else {
        return;
    };
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in pixels.chunks(4) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    let path = std::path::Path::new(&folder).join(format!("{name}.ppm"));
    std::fs::write(&path, bytes).unwrap();
}
