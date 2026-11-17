//! A character's material values (`Mat_…` attributes) as `modulate` draws
//! them: neutral settings change nothing, and each drawn value moves the
//! material the way `avatar`'s `material.rs` says (Character Studio
//! refinement R5, CO7's first slice).

use std::sync::Arc;

use axcenity_core::{ClassName, Color3, InstanceId, Value, World, material_value_attribute};
use axcenity_render::{Material, NormalMap, TextureSource};

use crate::material::modulate;

struct Flat;

impl TextureSource for Flat {
    fn key(&self) -> u64 {
        7
    }
    fn size(&self) -> (u32, u32) {
        (1, 1)
    }
    fn rgba(&self) -> &[u8] {
        &[128, 128, 255, 255]
    }
}

/// The realistic skin as `scene` draws it before values: its albedo map's
/// tone as the base colour (so the tests can read the toning off it), a
/// roughness and a normal map.
fn skin() -> Material {
    Material {
        base_color: [0.74, 0.53, 0.43],
        roughness: 0.6,
        maps: axcenity_render::TextureMaps {
            normal: Some(NormalMap {
                texture: Arc::new(Flat),
                scale: 1.0,
                flip_green: false,
            }),
            ..Default::default()
        },
        ..Material::default()
    }
}

fn part(values: &[(&str, &str, Value)]) -> (World, InstanceId) {
    let mut world = World::new();
    let part = world.create(ClassName::MeshPart).unwrap();
    for (material, name, value) in values {
        world
            .set_attribute(
                part,
                &material_value_attribute(material, name),
                Some(value.clone()),
            )
            .unwrap();
    }
    (world, part)
}

/// `material` (named `name`) drawn with `values` on its part.
fn drawn(material: &Material, name: &str, values: &[(&str, &str, Value)]) -> Material {
    let (world, part) = part(values);
    let mut material = material.clone();
    modulate(&mut material, world.get(part).unwrap(), name);
    material
}

fn number(
    material: &'static str,
    name: &'static str,
    value: f64,
) -> (&'static str, &'static str, Value) {
    (material, name, Value::Number(value))
}

fn linear(srgb: [f32; 3]) -> [f32; 3] {
    srgb.map(crate::skin_tone::srgb_to_linear)
}

fn luminance(srgb: [f32; 3]) -> f32 {
    let [r, g, b] = linear(srgb);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

#[test]
fn neutral_values_draw_the_material_exactly_as_authored() {
    let authored = skin();
    let white = Value::Color3(Color3::new(1.0, 1.0, 1.0));
    let neutral = drawn(
        &authored,
        "Skin",
        &[
            ("Skin", "tint", white.clone()),
            number("Skin", "melanin", 0.0),
            number("Skin", "redness", 0.0),
            number("Skin", "undertone", 0.0),
            number("Skin", "roughness_scale", 1.0),
            number("Skin", "detail_strength", 1.0),
            number("Skin", "scatter_scale", 1.0),
            ("Skin", "sclera_tint", white),
        ],
    );
    assert_eq!(neutral.base_color, authored.base_color);
    assert_eq!(neutral.roughness, authored.roughness);
    assert_eq!(neutral.maps.normal.as_ref().map(|map| map.scale), Some(1.0));
}

#[test]
fn melanin_darkens_and_lightens_the_skin_along_a_pigment_curve() {
    let authored = skin();
    let deeper = drawn(&authored, "Skin", &[number("Skin", "melanin", 0.8)]);
    let lighter = drawn(&authored, "Skin", &[number("Skin", "melanin", -0.8)]);
    let base = luminance(authored.base_color);
    assert!(
        luminance(deeper.base_color) < base * 0.5,
        "{:?}",
        deeper.base_color
    );
    assert!(
        luminance(lighter.base_color) > base * 1.2,
        "{:?}",
        lighter.base_color
    );
    // Not a plain darkening: from the authored tone a little deeper, blue
    // loses more than red (melanin absorbs short wavelengths most).
    let [r0, _, b0] = linear(authored.base_color);
    let a_little = drawn(&authored, "Skin", &[number("Skin", "melanin", 0.3)]);
    let [r, _, b] = linear(a_little.base_color);
    assert!(b / b0 < r / r0 * 0.95, "{:?}", a_little.base_color);
    // It is the value of the material it names, not another's.
    let other = drawn(&authored, "Skin", &[number("Body", "melanin", 0.8)]);
    assert_eq!(other.base_color, authored.base_color);
}

#[test]
fn redness_flushes_and_undertone_warms_the_skin() {
    let authored = skin();
    let [r0, g0, b0] = linear(authored.base_color);
    let [r, g, _] = linear(drawn(&authored, "Skin", &[number("Skin", "redness", 0.6)]).base_color);
    assert!(r / g > r0 / g0 * 1.02, "redness up isn't redder");
    let warmth = |[r, g, b]: [f32; 3]| (r + g) / (2.0 * b);
    let warm = drawn(&authored, "Skin", &[number("Skin", "undertone", 0.7)]);
    let cool = drawn(&authored, "Skin", &[number("Skin", "undertone", -0.7)]);
    assert!(warmth(linear(warm.base_color)) > warmth([r0, g0, b0]) * 1.02);
    assert!(warmth(linear(cool.base_color)) < warmth([r0, g0, b0]) / 1.02);
}

#[test]
fn a_tint_still_multiplies_after_toning() {
    let authored = skin();
    let toned = drawn(&authored, "Skin", &[number("Skin", "melanin", 0.4)]);
    let both = drawn(
        &authored,
        "Skin",
        &[
            number("Skin", "melanin", 0.4),
            ("Skin", "tint", Value::Color3(Color3::new(0.5, 1.0, 1.0))),
        ],
    );
    assert!((both.base_color[0] - toned.base_color[0] * 0.5).abs() < 1e-5);
    assert!((both.base_color[1] - toned.base_color[1]).abs() < 1e-5);
}

#[test]
fn detail_strength_scales_the_normal_map_and_keeps_it() {
    let authored = skin();
    let flat = drawn(&authored, "Skin", &[number("Skin", "detail_strength", 0.0)]);
    let strong = drawn(&authored, "Skin", &[number("Skin", "detail_strength", 2.0)]);
    let scale = |material: &Material| material.maps.normal.as_ref().map(|map| map.scale);
    assert_eq!(scale(&flat), Some(0.0));
    assert_eq!(scale(&strong), Some(2.0));
    assert_eq!(
        strong.maps.normal.as_ref().map(|map| map.texture.key()),
        Some(7)
    );
    // Held to the value's range (0 to 2).
    let past = drawn(&authored, "Skin", &[number("Skin", "detail_strength", 9.0)]);
    assert_eq!(scale(&past), Some(2.0));
}

#[test]
fn the_sclera_tint_multiplies_the_white_of_the_eye() {
    let sclera = Material {
        base_color: [1.0, 1.0, 1.0],
        ..Material::default()
    };
    let tinted = drawn(
        &sclera,
        "Sclera",
        &[(
            "Sclera",
            "sclera_tint",
            Value::Color3(Color3::new(1.0, 0.8, 0.8)),
        )],
    );
    assert_eq!(tinted.base_color, [1.0, 0.8, 0.8]);
}
