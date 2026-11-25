//! A character's material values (`Mat_…` attributes) as `modulate` draws
//! them: neutral settings change nothing, and each drawn value moves the
//! material the way `avatar`'s `material.rs` says (Character Studio
//! refinement R5, CO7's first slice).

use std::sync::Arc;

use axcenity_core::{ClassName, Color3, InstanceId, Value, World, material_value_attribute};
use axcenity_render::{Material, NormalMap, Shading, SkinShading, TextureSource};

use crate::material::{colour_map, modulate};

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

/// The realistic skin as a model marked skin draws it: Skin shading, with
/// its defined normal map beside its own (smooth) one.
fn skin_shaded() -> Material {
    struct Defined;
    impl TextureSource for Defined {
        fn key(&self) -> u64 {
            8
        }
        fn size(&self) -> (u32, u32) {
            (1, 1)
        }
        fn rgba(&self) -> &[u8] {
            &[150, 128, 250, 255]
        }
    }
    Material {
        shading: Shading::Skin(SkinShading {
            defined_normal: Some(Arc::new(Defined)),
            ..SkinShading::default()
        }),
        ..skin()
    }
}

/// `material`'s skin shading: its scatter and normal blend.
fn skin_of(material: &Material) -> (f32, f32) {
    match &material.shading {
        Shading::Skin(skin) => (skin.scatter, skin.normal_blend),
        other => panic!("not skin: {other:?}"),
    }
}

#[test]
fn neutral_values_leave_skin_shading_as_the_model_made_it() {
    let authored = skin_shaded();
    let neutral = drawn(
        &authored,
        "Skin",
        &[
            number("Skin", "scatter_scale", 1.0),
            number("Skin", "definition", 0.0),
            number("Skin", "softness", 0.0),
            number("Skin", "detail_strength", 1.0),
        ],
    );
    assert_eq!(skin_of(&neutral), skin_of(&authored));
    assert_eq!(skin_of(&neutral), (1.0, 0.0));
    let Shading::Skin(skin) = &neutral.shading else {
        unreachable!()
    };
    assert_eq!(
        skin.defined_normal.as_ref().map(|map| map.key()),
        Some(8),
        "the defined map is kept"
    );
}

#[test]
fn scatter_scale_multiplies_how_far_light_spreads_under_the_skin() {
    let authored = skin_shaded();
    let wide = drawn(&authored, "Skin", &[number("Skin", "scatter_scale", 1.6)]);
    let none = drawn(&authored, "Skin", &[number("Skin", "scatter_scale", 0.0)]);
    assert_eq!(skin_of(&wide).0, 1.6);
    assert_eq!(skin_of(&none).0, 0.0);
    // Held to the value's range (0 to 2).
    let past = drawn(&authored, "Skin", &[number("Skin", "scatter_scale", 7.0)]);
    assert_eq!(skin_of(&past).0, 2.0);
    // A standard material has nothing to scatter: it is left as it is.
    let standard = drawn(&skin(), "Skin", &[number("Skin", "scatter_scale", 1.6)]);
    assert!(matches!(standard.shading, Shading::Standard));
}

#[test]
fn definition_shows_the_defined_normal_map_unless_softness_hides_it() {
    let authored = skin_shaded();
    let blend = |definition: f64, softness: f64| {
        skin_of(&drawn(
            &authored,
            "Skin",
            &[
                number("Skin", "definition", definition),
                number("Skin", "softness", softness),
            ],
        ))
        .1
    };
    assert_eq!(blend(1.0, 0.0), 1.0, "lean and muscular: the defined map");
    assert_eq!(blend(0.0, 0.0), 0.0);
    assert_eq!(blend(1.0, 1.0), 0.0, "fat covers the definition");
    assert!((blend(0.8, 0.25) - 0.6).abs() < 1e-6);
    // Each held to 0 to 1.
    assert_eq!(blend(3.0, -2.0), 1.0);
    // Definition alone, softness alone.
    let alone = drawn(&authored, "Skin", &[number("Skin", "definition", 0.5)]);
    assert_eq!(skin_of(&alone).1, 0.5);
    let soft = drawn(&authored, "Skin", &[number("Skin", "softness", 0.5)]);
    assert_eq!(skin_of(&soft).1, 0.0);
}

/// The realistic skin with its makeup and marking masks, as a model whose
/// file names them draws it, before values: every layer off.
fn skin_with_masks() -> Material {
    let mut material = skin_shaded();
    if let Shading::Skin(skin) = &mut material.shading {
        skin.layers = Some(Box::new(axcenity_render::SkinLayers {
            masks: std::array::from_fn(|_| Arc::new(Flat) as Arc<dyn TextureSource>),
            makeup: Default::default(),
            freckles: Default::default(),
            moles: Default::default(),
            scar: 0.0,
        }));
    }
    material
}

fn layers_of(material: &Material) -> Option<&axcenity_render::SkinLayers> {
    match &material.shading {
        Shading::Skin(skin) => skin.layers.as_deref(),
        other => panic!("not skin: {other:?}"),
    }
}

fn colour(
    material: &'static str,
    name: &'static str,
    rgb: [f32; 3],
) -> (&'static str, &'static str, Value) {
    (
        material,
        name,
        Value::Color3(Color3::new(rgb[0], rgb[1], rgb[2])),
    )
}

#[test]
fn skin_layers_draw_from_their_values_and_none_on_draws_no_layers() {
    let authored = skin_with_masks();
    // No layer on: the masks aren't bound at all (nothing to draw).
    assert!(layers_of(&drawn(&authored, "Skin", &[])).is_none());
    let drawn = drawn(
        &authored,
        "Skin",
        &[
            number("Skin", "layer_blush_opacity", 0.4),
            colour("Skin", "layer_blush_colour", [0.8, 0.3, 0.3]),
            number("Skin", "layer_blush_roughness", 0.45),
            number("Skin", "layer_foundation_opacity", 0.7),
            number("Skin", "layer_brow_fill_opacity", 0.5),
            number("Skin", "layer_freckles_opacity", 0.9),
            number("Skin", "layer_freckles_amount", 0.6),
            number("Skin", "layer_freckles_seed", 7.0),
            number("Skin", "layer_moles_opacity", 1.0),
            number("Skin", "layer_scar_opacity", 0.8),
            // Another material's values and an unknown layer change nothing.
            number("Mouth", "layer_lips_opacity", 1.0),
            number("Skin", "layer_glitter_opacity", 1.0),
        ],
    );
    let layers = layers_of(&drawn).expect("layers on");
    let blush = layers.makeup[1];
    assert_eq!(blush.opacity, 0.4);
    assert_eq!(blush.colour, Some([0.8, 0.3, 0.3]));
    assert_eq!(blush.roughness, Some(0.45));
    // Foundation without a colour: the skin's own tone.
    assert_eq!(layers.makeup[0].colour, None);
    assert_eq!(layers.makeup[0].opacity, 0.7);
    assert_eq!(layers.makeup[5].opacity, 0.5, "brow fill");
    assert_eq!(layers.makeup[4].opacity, 0.0, "lips are the Mouth's here");
    assert_eq!(layers.freckles.amount, 0.6);
    assert_eq!(layers.freckles.opacity, 0.9);
    // The seed's golden-ratio offset: 7 × 0.618… = 4.3262…
    assert!((layers.freckles.seed - 0.326_237).abs() < 1e-4);
    // A marking without an amount shows all its spots.
    assert_eq!(layers.moles.amount, 1.0);
    assert_eq!(layers.moles.seed, 0.0);
    assert_eq!(layers.scar, 0.8);
    // Held to 0 to 1.
    let past = drawn_one(&authored, number("Skin", "layer_scar_opacity", 3.0));
    assert_eq!(layers_of(&past).unwrap().scar, 1.0);
}

fn drawn_one(material: &Material, value: (&'static str, &'static str, Value)) -> Material {
    drawn(material, "Skin", &[value])
}

#[test]
fn skin_without_masks_ignores_layer_values() {
    let authored = skin_shaded();
    let drawn = drawn_one(&authored, number("Skin", "layer_blush_opacity", 1.0));
    assert!(layers_of(&drawn).is_none());
}

// -- Hair and Cloth (CO7, wave 4) ------------------------------------------

fn hair_shaded() -> Material {
    Material {
        base_color: [0.45, 0.3, 0.18],
        roughness: 0.4,
        shading: Shading::Hair(axcenity_render::HairShading {
            data: Some(Arc::new(Flat)),
            ..Default::default()
        }),
        ..Material::default()
    }
}

fn hair_of(material: &Material) -> &axcenity_render::HairShading {
    match &material.shading {
        Shading::Hair(hair) => hair,
        other => panic!("not hair: {other:?}"),
    }
}

#[test]
fn hair_values_go_to_the_hair_model_not_the_skin_curve() {
    let made = hair_shaded();
    // Neutral: the colour as made, the model's values neutral.
    let neutral = drawn(&made, "Hair", &[number("Hair", "melanin", 0.0)]);
    assert_eq!(neutral.base_color, made.base_color);
    let defaults = axcenity_render::HairShading::default();
    let hair = hair_of(&neutral);
    assert_eq!(
        (
            hair.melanin,
            hair.redness,
            hair.secondary,
            hair.shift,
            hair.tint
        ),
        (0.0, 0.0, defaults.secondary, defaults.shift, [1.0; 3])
    );
    assert!(hair.data.is_some(), "its atlas's data kept");
    // Every value lands on the model; the base colour stays as made (the
    // renderer moves it along the pigment curve, live).
    let red = Value::Color3(Color3::new(1.0, 0.0, 0.0));
    let blue = Value::Color3(Color3::new(0.0, 0.0, 1.0));
    let set = drawn(
        &made,
        "Hair",
        &[
            number("Hair", "melanin", 0.5),
            number("Hair", "redness", -0.25),
            ("Hair", "dye", blue),
            number("Hair", "dye_amount", 0.3),
            number("Hair", "tip_lightening", 0.4),
            number("Hair", "root_melanin", 0.7),
            number("Hair", "root_length", 0.2),
            number("Hair", "streaks", 0.6),
            number("Hair", "streak_melanin", -0.8),
            number("Hair", "highlight", 1.5),
            number("Hair", "shift_scale", 0.5),
            ("Hair", "tint", red),
            number("Hair", "roughness_scale", 1.25),
        ],
    );
    assert_eq!(set.base_color, made.base_color);
    assert!((set.roughness - 0.5).abs() < 1e-6, "{}", set.roughness);
    let hair = hair_of(&set);
    assert_eq!((hair.melanin, hair.redness), (0.5, -0.25));
    assert_eq!((hair.dye, hair.dye_amount), ([0.0, 0.0, 1.0], 0.3));
    assert_eq!(hair.tip_lightening, 0.4);
    assert_eq!((hair.root_melanin, hair.root_length), (0.7, 0.2));
    assert_eq!((hair.streaks, hair.streak_melanin), (0.6, -0.8));
    assert_eq!((hair.secondary, hair.shift), (1.5, 0.5));
    assert_eq!(hair.tint, [1.0, 0.0, 0.0]);
}

fn cloth_shaded() -> Material {
    Material {
        base_color: [0.4, 0.4, 0.5],
        roughness: 0.8,
        shading: Shading::Cloth(axcenity_render::ClothShading {
            sheen_colour: [0.6, 0.6, 0.6],
            sheen_roughness: 0.5,
            anisotropy: 0.4,
            anisotropy_rotation: 0.0,
        }),
        ..Material::default()
    }
}

fn cloth_of(material: &Material) -> &axcenity_render::ClothShading {
    match &material.shading {
        Shading::Cloth(cloth) => cloth,
        other => panic!("not cloth: {other:?}"),
    }
}

#[test]
fn cloth_values_modulate_the_authored_sheen_and_anisotropy() {
    let made = cloth_shaded();
    let neutral = drawn(&made, "Shirt", &[number("Shirt", "anisotropy_scale", 1.0)]);
    let cloth = cloth_of(&neutral);
    assert_eq!(
        (
            cloth.sheen_colour,
            cloth.sheen_roughness,
            cloth.anisotropy,
            cloth.anisotropy_rotation
        ),
        ([0.6; 3], 0.5, 0.4, 0.0)
    );
    let set = drawn(
        &made,
        "Shirt",
        &[
            (
                "Shirt",
                "sheen_tint",
                Value::Color3(Color3::new(1.0, 0.5, 0.0)),
            ),
            number("Shirt", "sheen_roughness_scale", 1.5),
            number("Shirt", "anisotropy_scale", 2.0),
            number("Shirt", "anisotropy_rotation", 90.0),
            ("Shirt", "tint", Value::Color3(Color3::new(0.5, 1.0, 1.0))),
        ],
    );
    let cloth = cloth_of(&set);
    assert_eq!(cloth.sheen_colour, [0.6, 0.3, 0.0]);
    assert!((cloth.sheen_roughness - 0.75).abs() < 1e-6);
    assert!((cloth.anisotropy - 0.8).abs() < 1e-6);
    assert!((cloth.anisotropy_rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    // The generic tint still multiplies the base colour.
    assert!((set.base_color[0] - 0.2).abs() < 1e-6);
    // An anisotropic lobe repeats every half turn, so the turn is held to
    // -90 to 90 degrees (review fixes 2).
    let turned = drawn(
        &made,
        "Shirt",
        &[number("Shirt", "anisotropy_rotation", 135.0)],
    );
    assert!(
        (cloth_of(&turned).anisotropy_rotation - std::f32::consts::FRAC_PI_2).abs() < 1e-6,
        "{}",
        cloth_of(&turned).anisotropy_rotation
    );
}

/// ADR-0048: a character's colour map (`Mat_<material>_colour_map`, a
/// texture file in the project) is drawn in place of the material's own
/// and nothing else of the material changes, so its values still modulate
/// it. A file that can't be read leaves the material as authored.
#[test]
fn a_colour_map_takes_the_place_of_the_materials_own_and_keeps_the_rest() {
    let project =
        std::env::temp_dir().join(format!("axcenity-scene-colour-map-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(project.join("assets/characters/skins")).unwrap();
    let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([90, 60, 40, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    let path = "assets/characters/skins/old.png";
    std::fs::write(project.join(path), png.into_inner()).unwrap();
    let assets = axcenity_assets::Assets::new(&project);

    let mut authored = skin();
    authored.base_color_texture = Some(Arc::new(Flat));
    let draw = |values: &[(&str, &str, Value)]| {
        let (world, part) = part(values);
        let mut material = authored.clone();
        colour_map(&mut material, world.get(part).unwrap(), "Skin", &assets);
        modulate(&mut material, world.get(part).unwrap(), "Skin");
        material
    };
    let key = |material: &Material| material.base_color_texture.as_ref().map(|t| t.key());
    let map = |file: &str| {
        (
            "Skin",
            axcenity_core::MATERIAL_COLOUR_MAP,
            Value::String(file.into()),
        )
    };

    // No map, a map of another material and an empty one: as authored.
    assert_eq!(key(&draw(&[])), Some(7));
    assert_eq!(
        key(&draw(&[(
            "Iris",
            axcenity_core::MATERIAL_COLOUR_MAP,
            Value::String(path.into())
        )])),
        Some(7)
    );
    assert_eq!(key(&draw(&[map("")])), Some(7));

    let drawn = draw(&[map(path), number("Skin", "roughness_scale", 0.5)]);
    let texture = drawn.base_color_texture.as_ref().expect("a colour map");
    assert_ne!(texture.key(), 7, "the file in place of the material's own");
    assert_eq!(texture.size(), (2, 2));
    assert_eq!(&texture.rgba()[..4], &[90, 60, 40, 255]);
    assert_eq!(drawn.base_color, authored.base_color);
    assert!(drawn.maps.normal.is_some(), "the normal map stays");
    assert!(
        (drawn.roughness - 0.3).abs() < 1e-6,
        "values still modulate"
    );

    // A file that isn't there: the material's own, and one error.
    assert!(assets.take_errors().is_empty());
    assert_eq!(
        key(&draw(&[map("assets/characters/skins/none.png")])),
        Some(7)
    );
    assert_eq!(assets.take_errors().len(), 1);
    let _ = std::fs::remove_dir_all(&project);
}

/// ADR-0048: a skin between two colour maps. The map its own fades toward
/// (`Mat_<material>_colour_map_toward`) and how far
/// (`Mat_<material>_colour_map_amount`) become the Skin model's second
/// colour map; a material that isn't skin, no amount, or a file that
/// can't be read draws the first map alone.
#[test]
fn a_skin_between_two_colour_maps_draws_the_second_by_its_amount() {
    use axcenity_core::{MATERIAL_COLOUR_AMOUNT, MATERIAL_COLOUR_MAP, MATERIAL_COLOUR_TOWARD};
    let project = std::env::temp_dir().join(format!(
        "axcenity-scene-colour-blend-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&project);
    std::fs::create_dir_all(project.join("skins")).unwrap();
    for (file, pixel) in [
        ("young.png", [200, 150, 130, 255]),
        ("old.png", [150, 110, 90, 255]),
    ] {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba(pixel));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(project.join("skins").join(file), png.into_inner()).unwrap();
    }
    let assets = axcenity_assets::Assets::new(&project);
    let draw = |authored: &Material, toward: &str, amount: f64| {
        let (world, part) = part(&[
            (
                "Skin",
                MATERIAL_COLOUR_MAP,
                Value::String("skins/young.png".into()),
            ),
            ("Skin", MATERIAL_COLOUR_TOWARD, Value::String(toward.into())),
            ("Skin", MATERIAL_COLOUR_AMOUNT, Value::Number(amount)),
        ]);
        let mut material = authored.clone();
        colour_map(&mut material, world.get(part).unwrap(), "Skin", &assets);
        material
    };
    let second = |material: &Material| match &material.shading {
        Shading::Skin(skin) => skin
            .second_colour
            .as_ref()
            .map(|(map, amount)| (map.rgba()[0], *amount)),
        _ => None,
    };
    let skin_material = Material {
        shading: Shading::Skin(SkinShading::default()),
        ..skin()
    };
    let drawn = draw(&skin_material, "skins/old.png", 0.3);
    assert_eq!(drawn.base_color_texture.as_ref().unwrap().rgba()[0], 200);
    assert_eq!(second(&drawn), Some((150, 0.3)));
    assert_eq!(
        second(&draw(&skin_material, "skins/old.png", 4.0)),
        Some((150, 1.0))
    );
    assert_eq!(second(&draw(&skin_material, "skins/old.png", 0.0)), None);
    assert_eq!(second(&draw(&skin_material, "", 0.3)), None);
    assert_eq!(second(&draw(&skin_material, "skins/none.png", 0.3)), None);
    // A material that isn't skin draws the first map alone.
    let standard = draw(&skin(), "skins/old.png", 0.3);
    assert!(matches!(standard.shading, Shading::Standard));
    assert_eq!(standard.base_color_texture.as_ref().unwrap().rgba()[0], 200);
    let _ = std::fs::remove_dir_all(&project);
}
