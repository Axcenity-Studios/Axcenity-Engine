//! Alpha cards (CO7): a hair card's hashed alpha settling under TAA with
//! little frame-to-frame variance and soft edges; its coverage graded under
//! MSAA; a cut-out card on a skinned, morphed part showing the skin behind
//! its clear pixels (OPEN-19); and a striped card's shadow striped in the
//! shadow maps and in ray traced shadows, on posed parts too.

use std::sync::Arc;

use glam::{Mat4, Vec3};

use super::card_set::{card, evidence, flat_alpha, hair, ramp, rgb_at, striped, sum};
use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, MotionBlurQuality, Quality};
use crate::{GraphicsSettings, Shading, Skin};

/// The frame size, square.
const SIZE: u32 = 128;

/// How much a hair card's pixels may vary from frame to frame once TAA
/// has settled: the root mean square of each pixel's standard deviation
/// over 16 frames, in steps of 255, over a card whose opacity ramps from
/// clear to whole and its edges (`docs/rendering/hair.md`). Measured 3.38
/// on an RX 9070 XT (a Standard cut-out, hard-edged: 0.6). Provisional:
/// asset-quality.md §4 has Paul set it at the first hairstyle's sign-off.
pub(super) const TAA_VARIANCE_LIMIT: f32 = 4.5;

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

fn make_renderer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    settings: &GraphicsSettings,
) -> TextureRenderer {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(settings);
    renderer
}

/// A card of light hair whose alpha ramps from clear (left) to whole
/// (right), filling most of the view, in front of the dark, lit from the
/// front; `standard` draws it as a Standard cut-out instead.
fn ramp_card(standard: bool) -> DrawList {
    let mut material = hair([0.7, 0.55, 0.4], 0.4, |_| {});
    if standard {
        material.shading = Shading::Standard;
    }
    material.base_color_texture = Some(ramp(0x7a_3b));
    material.alpha_cutoff = 0.5;
    let mut list = list_of(vec![card(Vec3::new(0.0, 0.0, -3.0), [3.0, 3.0], material)]);
    list.sun.direction = Vec3::new(0.2, 0.3, 1.0).normalize();
    list.sun.intensity = 2.0;
    list.shadows.enabled = false;
    list
}

/// The red channel along the middle row.
fn middle_row(pixels: &[u8]) -> Vec<f32> {
    (0..SIZE)
        .map(|x| rgb_at(pixels, SIZE, x, SIZE / 2)[0])
        .collect()
}

/// How many pixels of `row` are neither the dark around the card (under
/// 12) nor its full brightness (within 12 of the row's brightest).
fn in_between(row: &[f32]) -> usize {
    let brightest = row.iter().copied().fold(0.0, f32::max);
    row.iter()
        .filter(|&&value| value > 12.0 && value < brightest - 12.0)
        .count()
}

#[test]
fn a_hair_card_under_taa_holds_still_with_soft_edges() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let list = ramp_card(false);
    let mut renderer = make_renderer(&device, &queue, &settings(AntiAliasing::Taa));
    for _ in 0..32 {
        renderer.render(&list).unwrap();
    }
    let frames: Vec<Vec<u8>> = (0..16)
        .map(|_| {
            renderer.render(&list).unwrap();
            renderer.read_pixels()
        })
        .collect();
    // Each pixel's spread over the frames, where the card is.
    let mut squares = 0.0;
    let mut counted = 0;
    let mut mean_row = vec![0.0; SIZE as usize];
    let mut std_row = vec![0.0f32; SIZE as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let values: Vec<f32> = frames
                .iter()
                .map(|f| sum(rgb_at(f, SIZE, x, y)) / 3.0)
                .collect();
            let mean = values.iter().sum::<f32>() / values.len() as f32;
            if y == SIZE / 2 {
                mean_row[x as usize] = mean;
            }
            if mean < 4.0 {
                continue;
            }
            let variance =
                values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / values.len() as f32;
            if y == SIZE / 2 {
                std_row[x as usize] = variance.sqrt();
            }
            squares += variance;
            counted += 1;
        }
    }
    for (index, frame) in frames.iter().take(4).enumerate() {
        evidence(&format!("taa-hair-ramp-frame-{index}"), frame, SIZE, SIZE);
    }
    let rms = (squares / counted as f32).sqrt();
    eprintln!("TAA: {counted} card pixels, rms frame-to-frame deviation {rms:.2}");
    eprintln!(
        "middle row, averaged: {:?}",
        mean_row.iter().map(|v| v.round()).collect::<Vec<_>>()
    );
    eprintln!(
        "middle row, deviation: {:?}",
        std_row.iter().map(|v| v.round()).collect::<Vec<_>>()
    );
    assert!(counted > 2000, "the card is drawn: {counted}");
    assert!(rms < TAA_VARIANCE_LIMIT, "hair pixels shimmer: {rms:.2}");
    // A soft edge: the ramp shows as many steps, not a cut.
    let soft = in_between(&mean_row);
    assert!(soft >= 20, "a soft edge over {soft} pixels: {mean_row:?}");
    // A Standard cut-out under TAA stays a cut: hashing is the hair's.
    let list = ramp_card(true);
    let mut renderer = make_renderer(&device, &queue, &settings(AntiAliasing::Taa));
    for _ in 0..32 {
        renderer.render(&list).unwrap();
    }
    renderer.render(&list).unwrap();
    let hard = in_between(&middle_row(&renderer.read_pixels()));
    assert!(hard <= 4, "a Standard cut-out is cut: {hard}");
}

#[test]
fn under_msaa_a_hair_cards_coverage_is_graded() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let draw = |list: &DrawList| {
        let mut renderer = make_renderer(&device, &queue, &settings(AntiAliasing::Msaa4));
        renderer.render(list).unwrap();
        middle_row(&renderer.read_pixels())
    };
    let row = draw(&ramp_card(false));
    eprintln!(
        "MSAA middle row: {:?}",
        row.iter().map(|v| v.round()).collect::<Vec<_>>()
    );
    let soft = in_between(&row);
    assert!(soft >= 20, "graded over {soft} pixels: {row:?}");
    // Its levels: a quarter, half and three quarters covered at least.
    let brightest = row.iter().copied().fold(0.0, f32::max);
    let mut levels: Vec<i32> = row
        .iter()
        .filter(|&&v| v > 12.0 && v < brightest - 12.0)
        .map(|v| (v / 16.0).round() as i32)
        .collect();
    levels.sort_unstable();
    levels.dedup();
    assert!(levels.len() >= 3, "graded coverage: {levels:?}");
    // A Standard cut-out under MSAA is smoothed over a pixel or two.
    let hard = in_between(&draw(&ramp_card(true)));
    assert!(hard <= 4, "a Standard cut-out: {hard}");
}

// -- Posed cards (OPEN-19) ---------------------------------------------------

/// A unit square facing +Z, `rest_x` studs to the right of where it's
/// drawn until its one joint moves it back, with a blend shape that moves
/// it `shape_z` toward the camera: a skinned, morphed card (a lash, a
/// brow, hair on a head).
struct PosedQuad {
    key: u64,
    rest: [[f32; 3]; 4],
}

const POSED_SHAPE_CORNERS: [u32; 4] = [0, 1, 2, 3];
const POSED_SHAPE_MOVES: [[f32; 3]; 4] = [[0.0, 0.0, 0.5]; 4];

impl PosedQuad {
    fn new(key: u64, rest_x: f32) -> Self {
        let (l, r) = (rest_x - 0.5, rest_x + 0.5);
        Self {
            key,
            rest: [[l, 0.5, 0.0], [r, 0.5, 0.0], [r, -0.5, 0.0], [l, -0.5, 0.0]],
        }
    }
}

impl MeshSource for PosedQuad {
    fn key(&self) -> u64 {
        self.key
    }
    fn positions(&self) -> &[[f32; 3]] {
        &self.rest
    }
    fn normals(&self) -> &[[f32; 3]] {
        &[[0.0, 0.0, 1.0]; 4]
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    }
    fn tangents(&self) -> &[[f32; 4]] {
        &[[1.0, 0.0, 0.0, -1.0]; 4]
    }
    fn indices(&self) -> &[u32] {
        &[0, 3, 2, 0, 2, 1]
    }
    fn skin_joints(&self) -> &[[u16; 4]] {
        &[[0; 4]; 4]
    }
    fn skin_weights(&self) -> &[[f32; 4]] {
        &[[1.0, 0.0, 0.0, 0.0]; 4]
    }
    fn morph_count(&self) -> usize {
        1
    }
    fn morph(&self, index: usize) -> Option<crate::MorphView<'_>> {
        (index == 0).then_some(crate::MorphView {
            target: 0,
            corners: &POSED_SHAPE_CORNERS,
            positions: &POSED_SHAPE_MOVES,
            normals: &[],
        })
    }
}

/// A posed quad of `size` at `center`: at rest 30 studs to the side, its
/// joint moving it back and its shape at weight 1 bringing it half a stud
/// forward, so it is drawn only if posed.
fn posed_quad(key: u64, center: Vec3, size: Vec3, material: crate::Material) -> DrawItem {
    let mut skin = Skin::new(vec![Mat4::from_translation(Vec3::new(-30.0, 0.0, 0.0))]);
    skin.set_morph_weights(&[1.0]);
    DrawItem {
        shape: Shape::Mesh(Arc::new(PosedQuad::new(key, 30.0))),
        transform: Mat4::from_scale_rotation_translation(
            size,
            glam::Quat::IDENTITY,
            center - Vec3::new(0.0, 0.0, 0.5 * size.z),
        ),
        material: crate::Material {
            double_sided: true,
            ..material
        },
        skin: Some(Arc::new(skin)),
        ..item(Shape::Block, Vec3::ZERO, 1.0)
    }
}

/// Skin (a red, unlit, skinned and morphed quad) with a cut-out card of
/// four broad strands on a skinned, morphed part in front of it, over a
/// blue sky; `hair` draws the card with the Hair model.
fn card_over_skin(hair_model: bool) -> DrawList {
    let skin = posed_quad(
        0x05c1,
        Vec3::new(0.0, 0.0, -6.0),
        Vec3::new(6.0, 6.0, 1.0),
        crate::Material {
            base_color: [0.8, 0.1, 0.1],
            shading: Shading::Unlit,
            ..crate::Material::default()
        },
    );
    let mut material = if hair_model {
        hair([0.9, 0.9, 0.9], 0.4, |_| {})
    } else {
        crate::Material {
            base_color: [0.9; 3],
            ..crate::Material::default()
        }
    };
    material.base_color_texture = Some(striped(0x57_21, 4));
    material.alpha_cutoff = 0.5;
    let card = posed_quad(
        0x0ca1,
        Vec3::new(0.0, 0.0, -4.0),
        Vec3::new(3.0, 3.0, 1.0),
        material,
    );
    let mut list = list_of(vec![skin, card]);
    list.sky.top_color = [0.0, 0.0, 1.0];
    list.sky.horizon_color = [0.0, 0.0, 1.0];
    list.sun.direction = Vec3::new(0.0, 0.0, 1.0);
    list
}

#[test]
fn a_cut_out_card_on_a_skinned_morphed_part_shows_the_skin_behind_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    // The clear stripes' middles, along the middle row: the card spans
    // the middle 3 / (2 × 4 × tan 35°) of the view.
    let width = 3.0 / (2.0 * 4.0 * 35f32.to_radians().tan()) * SIZE as f32;
    let left = SIZE as f32 / 2.0 - width / 2.0;
    let clear: Vec<u32> = (0..4)
        .map(|stripe| (left + width * (2.0 * stripe as f32 + 1.5) / 8.0) as u32)
        .collect();
    let whole: Vec<u32> = (0..4)
        .map(|stripe| (left + width * (2.0 * stripe as f32 + 0.5) / 8.0) as u32)
        .collect();
    let mut every = vec![settings(AntiAliasing::Off), settings(AntiAliasing::Msaa4)];
    every.push(settings(AntiAliasing::Taa));
    every.push(GraphicsSettings {
        ambient_occlusion: Some(AmbientOcclusion::High),
        ..settings(AntiAliasing::Taa)
    });
    for hair_model in [false, true] {
        let list = card_over_skin(hair_model);
        for settings in &every {
            let mut renderer = make_renderer(&device, &queue, settings);
            for _ in 0..8 {
                renderer.render(&list).unwrap();
            }
            let pixels = renderer.read_pixels();
            // The card is drawn: its strands are grey, not the skin.
            for &x in &whole {
                let seen = rgb_at(&pixels, SIZE, x, SIZE / 2);
                assert!(
                    seen[2] > 60.0 && (seen[0] - seen[2]).abs() < 30.0,
                    "hair {hair_model}, {:?}: the card's strand at x {x}: {seen:?}",
                    settings.anti_aliasing,
                );
            }
            for &x in &clear {
                let seen = rgb_at(&pixels, SIZE, x, SIZE / 2);
                assert!(
                    seen[0] > 120.0 && seen[2] < 60.0,
                    "hair {hair_model}, {:?}, AO {:?}: the skin, not the sky, through the card at x {x}: {seen:?}",
                    settings.anti_aliasing,
                    settings.ambient_occlusion,
                );
            }
        }
    }
}

// -- Cut-out-aware shadows ---------------------------------------------------

/// A wall 10 studs away facing the camera, lit from the front right, with
/// a card of `stripes` broad strands (4 studs square) 4 studs in front of
/// it, whose shadow falls on the wall left of the middle; `posed` makes
/// the card a skinned, morphed part.
fn striped_shadow(hair_model: bool, posed: bool) -> DrawList {
    let mut material = if hair_model {
        hair([0.6, 0.5, 0.4], 0.4, |_| {})
    } else {
        crate::Material::default()
    };
    material.base_color_texture = Some(striped(0x57_22, 4));
    material.alpha_cutoff = 0.5;
    let center = Vec3::new(2.5, 0.0, -6.0);
    let card = if posed {
        posed_quad(0x0ca2, center, Vec3::new(4.0, 4.0, 1.0), material)
    } else {
        card(center, [4.0, 4.0], material)
    };
    let wall = DrawItem {
        transform: Mat4::from_scale_rotation_translation(
            Vec3::new(40.0, 30.0, 1.0),
            glam::Quat::IDENTITY,
            Vec3::new(0.0, 0.0, -10.5),
        ),
        ..big_block()
    };
    let mut list = list_of(vec![wall, card]);
    list.sun.direction = Vec3::new(1.0, 0.0, 1.0).normalize();
    list.shadows.softness = 0.0;
    list
}

/// How many times the wall's brightness turns between lit and shadowed
/// along the middle row, across where the card's shadow falls (the left
/// half), and how dark its darkest pixel is against its brightest.
fn stripes_in(pixels: &[u8], size: u32) -> (usize, f32, f32) {
    let row: Vec<f32> = (0..size / 2)
        .map(|x| rgb_at(pixels, size, x, size / 2)[0])
        .collect();
    let (low, high) = row.iter().fold((f32::MAX, 0.0f32), |(low, high), &v| {
        (low.min(v), high.max(v))
    });
    let middle = 0.5 * (low + high);
    let mut turns = 0;
    let mut above = row[0] > middle;
    for &value in &row {
        if (value > middle) != above && (value - middle).abs() > 0.15 * (high - low) {
            turns += 1;
            above = value > middle;
        }
    }
    (turns, low, high)
}

#[test]
fn a_striped_cards_shadow_is_striped_in_the_shadow_maps() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let size = 256;
    for (hair_model, posed) in [(false, false), (true, false), (true, true)] {
        let list = striped_shadow(hair_model, posed);
        let mut renderer = TextureRenderer::new(&device, &queue, size, size);
        renderer.set_graphics(&GraphicsSettings {
            shadow_quality: Some(crate::ShadowQuality::High),
            anti_aliasing: Some(AntiAliasing::Off),
            ..GraphicsSettings::default()
        });
        renderer.render(&list).unwrap();
        let pixels = renderer.read_pixels();
        evidence(
            &format!("shadow-map-stripes-hair-{hair_model}-posed-{posed}"),
            &pixels,
            size,
            size,
        );
        let (turns, low, high) = stripes_in(&pixels, size);
        eprintln!("shadow maps, hair {hair_model}, posed {posed}: {turns} turns, {low}..{high}");
        assert!(high - low > 40.0, "a shadow falls: {low}..{high}");
        // Four strands and three gaps: about seven turns, not a slab's two.
        assert!(
            turns >= 6,
            "hair {hair_model}, posed {posed}: a slab ({turns} turns)"
        );
    }
}

#[test]
fn a_striped_cards_ray_traced_shadow_is_striped_on_a_posed_part_too() {
    let Some((device, queue)) = ray_test_device() else {
        eprintln!("skipped: no GPU with hardware ray tracing on this machine");
        return;
    };
    let size = 256;
    for (hair_model, posed) in [(false, false), (false, true), (true, true)] {
        let list = striped_shadow(hair_model, posed);
        let mut renderer = TextureRenderer::new(&device, &queue, size, size);
        renderer.set_graphics(&GraphicsSettings {
            shadow_quality: Some(crate::ShadowQuality::Off),
            ray_traced_shadows: Some(crate::RayTracedShadows::Low),
            anti_aliasing: Some(AntiAliasing::Off),
            ..GraphicsSettings::default()
        });
        for _ in 0..4 {
            renderer.render(&list).unwrap();
        }
        let (turns, low, high) = stripes_in(&renderer.read_pixels(), size);
        eprintln!("ray traced, hair {hair_model}, posed {posed}: {turns} turns, {low}..{high}");
        assert!(high - low > 40.0, "a shadow falls: {low}..{high}");
        assert!(
            turns >= 6,
            "hair {hair_model}, posed {posed}: a slab ({turns} turns)"
        );
    }
}

#[test]
fn a_clear_card_on_a_posed_part_casts_no_ray_traced_shadow_or_occlusion() {
    let Some((device, queue)) = ray_test_device() else {
        eprintln!("skipped: no GPU with hardware ray tracing on this machine");
        return;
    };
    let size = 128;
    // The card wholly clear, close in front of the wall, on a posed part:
    // rays pass it, for the sun and for ambient occlusion.
    let clear = |posed: bool| {
        let mut list = striped_shadow(false, posed);
        list.items[1].material.base_color_texture = Some(flat_alpha(0xc1_ea, 0));
        list.ambient.intensity = 1.0;
        list
    };
    let mut open = clear(false);
    open.items.truncate(1);
    let draw = |list: &DrawList| {
        let mut renderer = TextureRenderer::new(&device, &queue, size, size);
        renderer.set_graphics(&GraphicsSettings {
            shadow_quality: Some(crate::ShadowQuality::Off),
            ray_traced_shadows: Some(crate::RayTracedShadows::Low),
            ambient_occlusion: Some(AmbientOcclusion::Off),
            ray_traced_ambient_occlusion: Some(crate::RayTracedAmbientOcclusion::Low),
            anti_aliasing: Some(AntiAliasing::Off),
            ..GraphicsSettings::default()
        });
        for _ in 0..4 {
            renderer.render(list).unwrap();
        }
        let pixels = renderer.read_pixels();
        (0..size / 2)
            .map(|x| sum(rgb_at(&pixels, size, x, size / 2)))
            .sum::<f32>()
            / (size / 2) as f32
    };
    let open = draw(&open);
    for posed in [false, true] {
        let through = draw(&clear(posed));
        eprintln!("posed {posed}: {through:.1} vs open {open:.1}");
        assert!(
            (through - open).abs() < 6.0,
            "posed {posed}: {through} vs {open}"
        );
    }
}
