//! The scene pass's depth starting from the prepass's
//! (`renderer/depth_start.rs`): the picture must be the one the scene
//! pass draws from a cleared depth, pixel for pixel, and the scene pass
//! must be left alone wherever the prepass's depth isn't the scene's.

use std::sync::Arc;

use glam::{Mat4, Vec3};

use super::card_set::{card, evidence, hair, striped};
use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, MotionBlurQuality, Quality};
use crate::{GraphicsSettings, Outline, SurfaceShader};

const SIZE: u32 = 160;

/// A surface with holes of its own making: stripes across U.
const HOLED: &str = "\
fn surface(input: SurfaceInput) -> Surface {
    var out = standard_surface(input);
    out.opacity = step(0.5, fract(input.uv.x * 6.0));
    return out;
}
";

fn settings(anti_aliasing: AntiAliasing, occlusion: AmbientOcclusion) -> GraphicsSettings {
    GraphicsSettings {
        anti_aliasing: Some(anti_aliasing),
        motion_blur: Some(MotionBlurQuality::Off),
        ambient_occlusion: Some(occlusion),
        contact_shadows: Some(occlusion != AmbientOcclusion::Off),
        volumetric_fog: Some(false),
        ..GraphicsSettings::preset(Quality::High)
    }
}

/// A thicket over a terrain: leaf cards with holes in layers one behind
/// another, solid blocks and balls among them (two of them in exactly the
/// same place), a hair card, a block whose surface shader cuts its own
/// holes, a plant fading out at its cull distance, an outlined ball and a
/// see-through pane in front of some of it.
fn thicket() -> DrawList {
    let layers = Arc::new(crate::TerrainLayers {
        id: 71,
        key: 71,
        layers: vec![crate::TerrainLayer {
            color: [0.5, 0.4, 0.3],
            ..terrain::tests::layer()
        }],
        view: crate::TerrainView::Shaded,
        height_range: [0.0, 1.0],
        grid_origin: [-10.0, -10.0],
        grid_cell: [1.0, 1.0],
    });
    let mut items = vec![DrawItem {
        transform: Mat4::from_translation(Vec3::new(0.0, -2.0, -8.0)),
        material: Material {
            terrain: Some(crate::TerrainSurface {
                layers,
                weights: [
                    Arc::new(Weights(7100, [255, 0, 0, 0])),
                    Arc::new(Weights(7101, [0; 4])),
                ],
            }),
            ..Material::default()
        },
        ..item(Shape::Mesh(Arc::new(Ground)), Vec3::ZERO, 1.0)
    }];
    // Leaf cards: three layers, each offset so holes show the one behind.
    for layer in 0..3 {
        for column in 0..4 {
            let center = Vec3::new(
                column as f32 * 1.9 - 3.0 + layer as f32 * 0.35,
                0.2 * layer as f32 - 0.6,
                -6.0 - layer as f32 * 1.3,
            );
            let material = Material {
                base_color: [0.2 + 0.2 * layer as f32, 0.7, 0.2 + 0.15 * column as f32],
                base_color_texture: Some(striped(7200 + layer, 5 + layer as u32)),
                alpha_cutoff: 0.5,
                ..Material::default()
            };
            let mut leaf = card(center, [1.8, 2.4], material);
            leaf.cull_distance = 400.0;
            items.push(leaf);
        }
    }
    // Solid things among them.
    for i in 0..6 {
        let shape = if i % 2 == 0 {
            Shape::Block
        } else {
            Shape::Sphere
        };
        let mut solid = item(
            shape,
            Vec3::new(i as f32 * 1.3 - 3.2, -1.2, -5.0 - (i % 3) as f32 * 1.6),
            1.0,
        );
        solid.material.base_color = [0.8, 0.3 + 0.1 * i as f32, 0.2];
        solid.material.roughness = 0.4;
        items.push(solid);
    }
    // Things in exactly the same place, differing in color, of which one
    // shows: two blocks drawn in one batch, two more of which the first
    // listed is drawn in a later batch (it is double-sided), and two leaf
    // cards.
    for color in [[0.1, 0.1, 0.9], [0.9, 0.1, 0.1]] {
        let mut twin = item(Shape::Block, Vec3::new(-0.4, 0.9, -4.5), 1.0);
        twin.material.base_color = color;
        items.push(twin);
    }
    for (color, double_sided) in [([0.9, 0.5, 0.0], true), ([0.0, 0.5, 0.9], false)] {
        let mut twin = item(Shape::Block, Vec3::new(0.5, 2.0, -5.0), 1.0);
        twin.material.base_color = color;
        twin.material.double_sided = double_sided;
        items.push(twin);
    }
    for color in [[0.9, 0.9, 0.9], [0.9, 0.1, 0.6]] {
        let material = Material {
            base_color: color,
            base_color_texture: Some(striped(7400, 4)),
            alpha_cutoff: 0.5,
            ..Material::default()
        };
        items.push(card(Vec3::new(-2.0, 2.2, -5.0), [1.4, 1.0], material));
    }
    // A hair card, hashed under TAA.
    let mut strands = hair([0.7, 0.55, 0.4], 0.4, |_| {});
    strands.base_color_texture = Some(striped(7300, 9));
    strands.alpha_cutoff = 0.5;
    items.push(card(Vec3::new(2.6, 1.0, -4.2), [1.2, 1.6], strands));
    // A surface shader that cuts holes itself.
    let mut holed = item(Shape::Block, Vec3::new(1.2, 0.8, -4.6), 1.0);
    holed.material.alpha_cutoff = 0.5;
    holed.material.double_sided = true;
    holed.material.surface_shader = Some(SurfaceShader::new("holed", HOLED));
    items.push(holed);
    // A plant most of the way through its fade.
    let mut fading = item(Shape::Sphere, Vec3::new(-2.4, 1.0, -9.6), 1.0);
    fading.transform *= Mat4::from_scale(Vec3::splat(2.0));
    fading.cull_distance = 10.5;
    fading.material.base_color = [0.9, 0.9, 0.2];
    items.push(fading);
    // An outlined ball.
    let mut outlined = item(Shape::Sphere, Vec3::new(3.0, -0.9, -4.0), 1.0);
    outlined.material.outline = Some(Outline {
        width: 6.0,
        color: [1.0, 1.0, 1.0],
    });
    items.push(outlined);
    // A see-through pane.
    let mut pane = item(Shape::Block, Vec3::new(-2.6, -0.4, -3.4), 0.5);
    pane.material.base_color = [0.2, 0.9, 0.9];
    items.push(pane);

    let mut list = list_of(items);
    list.sky.top_color = [0.2, 0.3, 0.6];
    list.sky.horizon_color = [0.6, 0.6, 0.7];
    list.sun.direction = Vec3::new(0.4, 0.8, 0.5).normalize();
    list.sun.intensity = 2.0;
    list.ambient.intensity = 0.4;
    list.shadows.enabled = true;
    list
}

/// `list` drawn for `frames` frames by a new renderer with `settings`,
/// the scene pass's depth starting from the prepass's if `from_prepass`
/// and it can: its pixels, and whether it did.
fn drawn(
    (device, queue): (&wgpu::Device, &wgpu::Queue),
    list: &DrawList,
    settings: &GraphicsSettings,
    from_prepass: bool,
    frames: usize,
) -> (Vec<u8>, bool) {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(settings);
    renderer.set_depth_from_prepass(from_prepass);
    for _ in 0..frames {
        renderer.render(list).unwrap();
    }
    (renderer.read_pixels(), renderer.stats().depth_from_prepass)
}

/// How many pixel channels differ between two pictures, and by how much
/// at worst.
fn difference(a: &[u8], b: &[u8]) -> (usize, u8) {
    assert_eq!(a.len(), b.len());
    let apart = a.iter().zip(b).map(|(a, b)| a.abs_diff(*b));
    (
        apart.clone().filter(|&apart| apart > 0).count(),
        apart.max().unwrap_or(0),
    )
}

#[test]
fn a_scene_whose_depth_starts_from_the_prepasss_is_the_same_picture() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let list = thicket();
    for anti_aliasing in [AntiAliasing::Off, AntiAliasing::Fxaa, AntiAliasing::Taa] {
        let settings = settings(anti_aliasing, AmbientOcclusion::High);
        let (cleared, started) = drawn((&device, &queue), &list, &settings, false, 6);
        assert!(!started, "{anti_aliasing:?}: turned off, it still started");
        let (shared, started) = drawn((&device, &queue), &list, &settings, true, 6);
        assert!(
            started,
            "{anti_aliasing:?}: the depth didn't start from the prepass's"
        );
        evidence(
            &format!("depth-cleared-{anti_aliasing:?}"),
            &cleared,
            SIZE,
            SIZE,
        );
        evidence(
            &format!("depth-shared-{anti_aliasing:?}"),
            &shared,
            SIZE,
            SIZE,
        );
        // The picture has something in it to compare.
        let lit = cleared.chunks(4).filter(|pixel| pixel[1] > 100).count();
        assert!(lit > 2000, "{anti_aliasing:?}: only {lit} bright pixels");
        assert_eq!(
            difference(&cleared, &shared),
            (0, 0),
            "{anti_aliasing:?}: channels that differ, and the most by"
        );
    }
}

#[test]
fn where_the_prepass_isnt_the_scenes_depth_the_scene_pass_clears_its_own() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let list = thicket();
    // Under MSAA the prepass has one sample per pixel and the scene four;
    // with nothing reading the prepass there is none.
    for settings in [
        settings(AntiAliasing::Msaa4, AmbientOcclusion::High),
        settings(AntiAliasing::Off, AmbientOcclusion::Off),
    ] {
        let (cleared, _) = drawn((&device, &queue), &list, &settings, false, 3);
        let (asked, started) = drawn((&device, &queue), &list, &settings, true, 3);
        assert!(!started);
        assert_eq!(difference(&cleared, &asked), (0, 0));
    }
}

#[test]
fn turning_it_on_and_off_on_one_renderer_keeps_the_picture() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let list = thicket();
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    renderer.set_graphics(&settings(AntiAliasing::Off, AmbientOcclusion::High));
    let picture = |renderer: &mut TextureRenderer, on| {
        renderer.set_depth_from_prepass(on);
        renderer.render(&list).unwrap();
        renderer.render(&list).unwrap();
        renderer.read_pixels()
    };
    let cleared = picture(&mut renderer, false);
    let shared = picture(&mut renderer, true);
    let cleared_again = picture(&mut renderer, false);
    assert_eq!(difference(&cleared, &shared), (0, 0));
    assert_eq!(difference(&cleared, &cleared_again), (0, 0));
}
