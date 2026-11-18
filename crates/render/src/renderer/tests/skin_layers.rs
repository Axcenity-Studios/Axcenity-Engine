//! Skin's makeup and markings (ADR-0042, R5): their masks bound in the
//! metalness, emissive and opacity maps' places with their values in the
//! uniform; each layer changing the skin where its mask is and nowhere
//! else; layers off drawing the skin bit for bit; a marking's seed and
//! amount choosing its spots; a layer's roughness changing the highlight.

use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};
use crate::{
    GraphicsSettings, Makeup, SKIN_MAKEUP_LAYERS, Shading, SkinLayers, SkinShading, Spots,
};

/// The frame size, square.
const SIZE: u32 = 96;

/// A texture of its own key.
struct Map {
    key: u64,
    size: u32,
    rgba: Vec<u8>,
}

fn map(key: u64, size: u32, texel: impl Fn(u32, u32) -> [u8; 4]) -> Arc<dyn TextureSource> {
    let rgba = (0..size)
        .flat_map(|y| (0..size).map(move |x| (x, y)))
        .flat_map(|(x, y)| texel(x, y))
        .collect();
    Arc::new(Map { key, size, rgba })
}

impl TextureSource for Map {
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

/// Masks 16 texels square: every channel whole over the right half of the
/// square and empty over the left, so a layer changes only the right
/// half. The ranks (green and alpha of the third) run 0 to 1 from the top
/// down, so a marking's amount and seed pick rows.
fn masks() -> [Arc<dyn TextureSource>; 3] {
    let half = |x: u32| if x >= 8 { 255 } else { 0 };
    let rank = |y: u32| (y * 255 / 15) as u8;
    [
        map(0x5e_a1, 16, move |x, _| [half(x); 4]),
        map(0x5e_a2, 16, move |x, _| [half(x), half(x), half(x), 0]),
        map(0x5e_a3, 16, move |x, y| {
            [half(x), rank(y), half(x), rank(y)]
        }),
    ]
}

/// [`masks`] with only one coverage channel whole (0 to 11, the masks'
/// channels in order; the ranks stay), so a layer reading another
/// layer's channel draws nothing.
fn only(channel: usize) -> [Arc<dyn TextureSource>; 3] {
    let rank = |y: u32| (y * 255 / 15) as u8;
    std::array::from_fn(|mask| {
        map(0x5e_b0 + (channel * 3 + mask) as u64, 16, move |x, y| {
            let mut texel = [0u8; 4];
            for (c, value) in texel.iter_mut().enumerate() {
                if mask * 4 + c == channel && x >= 8 {
                    *value = 255;
                }
            }
            if mask == 2 {
                texel[1] = rank(y);
                texel[3] = rank(y);
            }
            texel
        })
    })
}

fn layers() -> SkinLayers {
    SkinLayers {
        masks: masks(),
        makeup: [Makeup::default(); SKIN_MAKEUP_LAYERS],
        freckles: Spots::default(),
        moles: Spots::default(),
        scar: 0.0,
    }
}

fn skin(layers: Option<SkinLayers>) -> Shading {
    Shading::Skin(SkinShading {
        layers: layers.map(Box::new),
        ..SkinShading::default()
    })
}

/// A square facing the camera, skin-coloured, with a metalness map of its
/// own (the realistic skin's occlusion-roughness image), under a sun from
/// the front right; or the uniform and binding of that item.
fn square(shading: Shading) -> DrawItem {
    let mut square = item(Shape::Block, Vec3::ZERO, 1.0);
    square.transform = Mat4::from_scale(Vec3::new(4.0, 4.0, 0.1));
    square.material.base_color = [0.74, 0.53, 0.43];
    square.material.roughness = 0.5;
    square.material.maps.metalness = Some(ChannelMap {
        texture: map(0x5e_a0, 4, |_, _| [255, 128, 0, 255]),
        channel: Channel::Blue,
        invert: false,
    });
    square.material.shading = shading;
    square
}

fn draw(shading: Shading, sun: Vec3) -> Vec<u8> {
    let Some((device, queue)) = test_device() else {
        return Vec::new();
    };
    let mut list = list_of(vec![square(shading)]);
    list.camera = Camera {
        position: Vec3::new(0.0, 0.0, 5.0),
        look_at: Vec3::ZERO,
        fov_y_degrees: 40.0,
    };
    list.sun.direction = sun.normalize();
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

fn lit(shading: Shading) -> Vec<u8> {
    draw(shading, Vec3::new(0.5, 0.3, 1.0))
}

/// The mean colour (0 to 255 each) of a box of the frame: the left half's
/// middle or the right half's.
fn mean(pixels: &[u8], right: bool) -> [f32; 3] {
    let (x0, x1) = if right {
        (SIZE * 5 / 8, SIZE * 6 / 8)
    } else {
        (SIZE * 2 / 8, SIZE * 3 / 8)
    };
    let (mut sum, mut count) = ([0.0; 3], 0.0);
    for y in SIZE * 3 / 8..SIZE * 5 / 8 {
        for x in x0..x1 {
            let at = ((y * SIZE + x) * 4) as usize;
            for c in 0..3 {
                sum[c] += f32::from(pixels[at + c]);
            }
            count += 1.0;
        }
    }
    sum.map(|s| s / count)
}

fn apart(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|c| (a[c] - b[c]).abs()).fold(0.0, f32::max)
}

#[test]
fn skin_layers_bind_their_masks_in_place_of_metalness_emissive_and_opacity() {
    let plain = material_key(&square(skin(None)));
    let metalness = plain.textures[2].expect("the square's own metalness map");
    let mut with = layers();
    with.makeup[1] = Makeup {
        colour: Some([1.0, 0.5, 0.0]),
        opacity: 0.4,
        roughness: Some(0.3),
    };
    with.makeup[0] = Makeup {
        colour: None,
        opacity: 0.7,
        roughness: None,
    };
    with.freckles = Spots {
        amount: 0.6,
        opacity: 0.8,
        seed: 0.25,
    };
    with.moles = Spots {
        amount: 0.1,
        opacity: 0.9,
        seed: 0.5,
    };
    with.scar = 0.75;
    let masks = with.masks.clone();
    let key = material_key(&square(skin(Some(with))));
    assert_eq!(key.textures[2], Some(masks[0].key()));
    assert_eq!(key.textures[5], Some(masks[1].key()));
    assert_eq!(key.textures[6], Some(masks[2].key()));
    assert_ne!(key.textures[2], Some(metalness));
    let uniform = maps_uniform_of(&key);
    assert_eq!(uniform.specular[2], 1.0, "the masks are bound");
    assert_eq!(uniform.settings[3], 0.0, "no opacity map");
    // Blush: its colour in linear light and its opacity; foundation in
    // the skin's own tone (red below 0).
    let blush = uniform.params[1];
    assert!((blush[0] - 1.0).abs() < 1e-6 && (blush[1] - 0.2140).abs() < 1e-3);
    assert_eq!(blush[3], 0.4);
    assert!(uniform.params[0][0] < 0.0 && uniform.params[0][3] == 0.7);
    // Roughness: the blush's 0.3, the foundation's kept (below 0).
    assert_eq!(uniform.params[6][1], 0.3);
    assert!(uniform.params[6][0] < 0.0);
    assert_eq!(&uniform.params[7][2..], &[0.6, 0.8]);
    assert_eq!(uniform.toon, [0.1, 0.9, 0.75, 0.25]);
    assert_eq!(uniform.specular[3], 0.5);
    // Without layers the slots and uniform are skin's as before.
    assert_eq!(plain.textures[5], None);
    assert_eq!(maps_uniform_of(&plain).specular[2], 0.0);
}

/// Each layer alone at full opacity, in a colour far from the skin's: the
/// right half (its mask) changes, the left doesn't.
#[test]
fn each_makeup_layer_and_marking_changes_the_skin_where_its_mask_is() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let before = lit(skin(None));
    let (left, right) = (mean(&before, false), mean(&before, true));
    eprintln!("bare: left {left:?}, right {right:?}");
    let names = [
        "foundation",
        "blush",
        "eyeshadow",
        "eyeliner",
        "lips",
        "brow fill",
    ];
    for (index, name) in names.iter().enumerate() {
        let mut with = layers();
        with.masks = only(index);
        with.makeup[index] = Makeup {
            colour: Some([0.2, 0.6, 0.9]),
            opacity: 1.0,
            roughness: None,
        };
        let after = lit(skin(Some(with)));
        let moved = apart(mean(&after, true), right);
        eprintln!("{name}: right {:?} -> {:?}", right, mean(&after, true));
        assert!(moved > 20.0, "{name} changed its mask by {moved}");
        assert!(
            apart(mean(&after, false), left) < 1.0,
            "{name} changed the skin outside its mask"
        );
    }
    let darker = |shading: Shading, what: &str| {
        let after = lit(shading);
        let (now, was) = (mean(&after, true), right);
        eprintln!("{what}: right {was:?} -> {now:?}");
        assert!(
            apart(mean(&after, false), left) < 1.0,
            "{what} changed the skin outside its mask"
        );
        now.iter().sum::<f32>() - was.iter().sum::<f32>()
    };
    let spots = Spots {
        amount: 1.0,
        opacity: 1.0,
        seed: 0.0,
    };
    let mut freckled = layers();
    freckled.masks = only(8);
    freckled.freckles = spots;
    assert!(darker(skin(Some(freckled)), "freckles") < -40.0);
    let mut moled = layers();
    moled.masks = only(10);
    moled.moles = spots;
    assert!(darker(skin(Some(moled)), "moles") < -80.0);
    let mut scarred = layers();
    scarred.masks = only(6);
    scarred.scar = 1.0;
    let scar = lit(skin(Some(scarred)));
    let now = mean(&scar, true);
    eprintln!("scar: right {right:?} -> {now:?}");
    assert!(now[0] > right[0] + 3.0, "a scar is paler: {now:?}");
}

#[test]
fn layers_turned_off_draw_the_skin_bit_for_bit() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let mut off = layers();
    off.makeup[1] = Makeup {
        colour: Some([1.0, 0.0, 0.0]),
        opacity: 0.0,
        roughness: Some(0.1),
    };
    off.freckles = Spots {
        amount: 0.0,
        opacity: 1.0,
        seed: 0.3,
    };
    assert!(
        lit(skin(None)) == lit(skin(Some(off))),
        "layers off changed the skin"
    );
}

/// The rows of the right half a marking darkens, top to bottom.
fn spotted_rows(amount: f32, seed: f32) -> Vec<bool> {
    let bare = lit(skin(None));
    let mut with = layers();
    with.freckles = Spots {
        amount,
        opacity: 1.0,
        seed,
    };
    let after = lit(skin(Some(with)));
    // The square spans the middle of the frame; its rows of masks, 16
    // texels over its height.
    (0..16)
        .map(|row| {
            let y = SIZE / 2 - SIZE * 3 / 10 + row * SIZE * 6 / 10 / 16 + 1;
            let x = SIZE * 11 / 16;
            let at = ((y * SIZE + x) * 4) as usize;
            let (a, b) = (&after[at..at + 3], &bare[at..at + 3]);
            a.iter()
                .zip(b)
                .map(|(a, b)| i32::from(*b) - i32::from(*a))
                .sum::<i32>()
                > 20
        })
        .collect()
}

#[test]
fn a_markings_amount_shows_a_share_of_its_spots_and_its_seed_picks_which() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let half = spotted_rows(0.5, 0.0);
    let other = spotted_rows(0.5, 0.5);
    let none = spotted_rows(0.0, 0.0);
    eprintln!("amount 0.5 seed 0: {half:?}\nseed 0.5: {other:?}");
    let shown = |rows: &[bool]| rows.iter().filter(|&&on| on).count();
    assert!((5..=11).contains(&shown(&half)), "about half: {half:?}");
    assert!((5..=11).contains(&shown(&other)), "about half: {other:?}");
    assert_ne!(half, other, "another seed, other spots");
    assert_eq!(shown(&none), 0, "amount 0 shows none");
}

#[test]
fn a_layers_roughness_changes_the_skins_highlight() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    // The sun behind the camera: the highlight is in the middle.
    let toward = Vec3::new(0.15, 0.1, 1.0);
    let with = |roughness: Option<f32>| {
        let mut layered = layers();
        // In the skin's own tone, so only its roughness differs.
        layered.makeup[4] = Makeup {
            colour: None,
            opacity: 1.0,
            roughness,
        };
        mean(&draw(skin(Some(layered)), toward), true)
    };
    let (kept, glossy, matte) = (with(None), with(Some(0.15)), with(Some(0.95)));
    eprintln!("kept {kept:?}, glossy {glossy:?}, matte {matte:?}");
    let sum = |c: [f32; 3]| c.iter().sum::<f32>();
    assert!(
        sum(glossy) > sum(kept) + 6.0,
        "gloss: {glossy:?} vs {kept:?}"
    );
    assert!(sum(matte) < sum(kept) - 2.0, "matte: {matte:?} vs {kept:?}");
}

/// [`only`]'s masks with channel `channel` at `value` over the right half
/// (not whole), and every rank at `rank`.
fn partly(channel: usize, value: u8, rank: u8) -> [Arc<dyn TextureSource>; 3] {
    std::array::from_fn(|mask| {
        let key = 0x5e_c000
            + ((channel * 3 + mask) as u64) * 0x1_0000
            + u64::from(value) * 0x100
            + u64::from(rank);
        map(key, 16, move |x, _| {
            let mut texel = [0u8; 4];
            for (c, slot) in texel.iter_mut().enumerate() {
                if mask * 4 + c == channel && x >= 8 {
                    *slot = value;
                }
            }
            if mask == 2 {
                texel[1] = rank;
                texel[3] = rank;
            }
            texel
        })
    })
}

/// A mask's value is coverage, read as it is stored: half a mask under a
/// whole layer draws what a whole mask under a half-opaque layer draws.
/// Lips (the second mask, bound in the emissive map's place, a colour
/// slot otherwise read through its sRGB view) and blush (the first, in
/// the metalness place). Read through sRGB, 128 would cover 0.22.
#[test]
fn a_masks_middle_value_covers_half_in_every_slot() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    for (index, channel, name) in [(4, 4, "lips"), (1, 1, "blush"), (5, 5, "brow fill")] {
        let draw_with = |value: u8, opacity: f32| {
            let mut with = layers();
            with.masks = partly(channel, value, 0);
            with.makeup[index] = Makeup {
                colour: Some([0.2, 0.6, 0.9]),
                opacity,
                roughness: None,
            };
            mean(&lit(skin(Some(with))), true)
        };
        let (half_mask, half_layer) = (draw_with(128, 1.0), draw_with(255, 128.0 / 255.0));
        eprintln!("{name}: half mask {half_mask:?}, half layer {half_layer:?}");
        assert!(
            apart(half_mask, half_layer) < 2.0,
            "{name}: half its mask {half_mask:?} against half its opacity {half_layer:?}"
        );
    }
}

/// Amount 1 shows every spot, the last ranked too: a freckle ranked 0.99
/// is as dark as one ranked 0.
#[test]
fn a_markings_full_amount_shows_every_spot() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let spots = Spots {
        amount: 1.0,
        opacity: 1.0,
        seed: 0.0,
    };
    let freckled = |rank: u8| {
        let mut with = layers();
        with.masks = partly(8, 255, rank);
        with.freckles = spots;
        mean(&lit(skin(Some(with))), true)
    };
    let (first, last) = (freckled(0), freckled(253));
    eprintln!("ranked 0: {first:?}; ranked 0.99: {last:?}");
    // (Each spot's own shade comes from its rank too: within that.)
    let sum = |c: [f32; 3]| c.iter().sum::<f32>();
    let bare = sum(mean(&lit(skin(None)), true));
    assert!(
        bare - sum(last) > (bare - sum(first)) * 0.6,
        "a spot ranked 0.99 shows {} of the way",
        (bare - sum(last)) / (bare - sum(first))
    );
}
