//! The Hair and Cloth models' evidence (CO7, wave 4), run by hand:
//!
//! - `hair_and_cloth_evidence` draws the test card set and cloth swatch at
//!   the cameras and lights of the Blender reference renders
//!   (`docs/development/character-studio/refinement/wave4/co7-hair-cloth/`)
//!   into the folder `AXCENITY_RENDER_EVIDENCE` names;
//! - `bench_hair_and_cloth_models` times the scene pass (`FramePass`) with
//!   a ball of each shading model filling a 1080p view, and a stack of
//!   hair cards, on this machine's GPU.
//!
//! `cargo test -p axcenity-render --release -- --ignored
//! character_models_evidence --nocapture`

use glam::{Mat4, Vec3};

use super::card_set::{ball, card, cloth, dark_scene, evidence, hair, hair_data, ramp};
use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};
use crate::{FramePass, GraphicsSettings, HairShading, Shading, SkinShading};

/// A device that can time its passes, as the player's benchmarks open it.
fn timed_device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = crate::adapters::instance();
    let adapter = pollster::block_on(crate::adapters::adapter(&instance, None)).ok()?;
    let info = adapter.get_info();
    println!(
        "adapter: {} ({:?}, {})",
        info.name, info.backend, info.driver_info
    );
    let required_features = adapter.features()
        & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TEXTURE_COMPRESSION_BC);
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("evidence device"),
        required_features,
        ..Default::default()
    };
    pollster::block_on(adapter.request_device(&descriptor)).ok()
}

/// The comparison scenes, each with the Blender reference's camera (6
/// studs in front of a ball of radius 1, a 30° lens) and sun.
pub(super) fn scenes() -> Vec<(&'static str, DrawList)> {
    let brown = [0.45, 0.3, 0.18];
    let blonde = [0.75, 0.6, 0.4];
    let front = Vec3::new(0.0, 0.15, 1.0);
    let hair_ball = |colour, change: &dyn Fn(&mut HairShading)| {
        ball(1.0, Vec3::ZERO, hair(colour, 0.35, change))
    };
    let mut satin = cloth(|cloth| cloth.anisotropy = 0.8);
    satin.base_color = [0.05; 3];
    satin.roughness = 0.3;
    vec![
        (
            "hair-brown",
            dark_scene(vec![hair_ball(brown, &|_| {})], 6.0, 30.0, front),
        ),
        (
            "hair-blonde",
            dark_scene(vec![hair_ball(blonde, &|_| {})], 6.0, 30.0, front),
        ),
        (
            "hair-brown-melanin-0.6",
            dark_scene(
                vec![hair_ball(brown, &|h| h.melanin = 0.6)],
                6.0,
                30.0,
                front,
            ),
        ),
        (
            "hair-brown-redness-1",
            dark_scene(
                vec![hair_ball(brown, &|h| h.redness = 1.0)],
                6.0,
                30.0,
                front,
            ),
        ),
        ("hair-blonde-backlit", {
            // The ball shadows the strands facing away from the sun, as
            // the Blender scene's does.
            let mut list = dark_scene(
                vec![hair_ball(blonde, &|_| {})],
                6.0,
                30.0,
                Vec3::new(0.0, 0.5, -1.0),
            );
            list.shadows.enabled = true;
            list
        }),
        (
            "cloth-standard",
            dark_scene(
                vec![ball(1.0, Vec3::ZERO, cloth(|_| {}))],
                6.0,
                30.0,
                Vec3::new(0.0, 0.0, 1.0),
            ),
        ),
        (
            "cloth-sheen",
            dark_scene(
                vec![ball(
                    1.0,
                    Vec3::ZERO,
                    cloth(|cloth| {
                        cloth.sheen_colour = [1.0; 3];
                        cloth.sheen_roughness = 0.5;
                    }),
                )],
                6.0,
                30.0,
                Vec3::new(0.0, 0.0, 1.0),
            ),
        ),
        (
            "cloth-satin",
            dark_scene(
                vec![ball(1.0, Vec3::ZERO, satin)],
                6.0,
                30.0,
                Vec3::new(0.0, 0.0, 1.0),
            ),
        ),
    ]
}

#[test]
#[ignore = "writes the evidence pictures; run by hand"]
fn hair_and_cloth_evidence() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let size = 512;
    for (name, list) in scenes() {
        let mut renderer = TextureRenderer::new(&device, &queue, size, size);
        renderer.set_graphics(&GraphicsSettings {
            anti_aliasing: Some(AntiAliasing::Msaa4),
            ambient_occlusion: Some(AmbientOcclusion::Off),
            ..GraphicsSettings::preset(Quality::High)
        });
        renderer.render(&list).unwrap();
        evidence(
            &format!("engine-{name}"),
            &renderer.read_pixels(),
            size,
            size,
        );
    }
    // A card of strands over skin under each anti-aliasing, lit from the
    // front, and its striped shadow on a wall.
    let data = hair_data(0x0e71);
    for (mode, frames) in [
        (AntiAliasing::Off, 1),
        (AntiAliasing::Msaa4, 1),
        (AntiAliasing::Taa, 40),
    ] {
        let mut material = hair([0.55, 0.4, 0.28], 0.4, |hair| {
            hair.data = Some(data.clone())
        });
        material.base_color_texture = Some(ramp(0x0e72));
        material.alpha_cutoff = 0.5;
        let skin = DrawItem {
            transform: Mat4::from_scale_rotation_translation(
                Vec3::new(6.0, 6.0, 0.1),
                glam::Quat::IDENTITY,
                Vec3::new(0.0, 0.0, -1.0),
            ),
            material: crate::Material {
                base_color: [0.8, 0.55, 0.45],
                shading: Shading::Skin(SkinShading::default()),
                ..crate::Material::default()
            },
            ..item(Shape::Block, Vec3::ZERO, 1.0)
        };
        let mut list = dark_scene(
            vec![skin, card(Vec3::ZERO, [2.0, 2.0], material)],
            3.0,
            50.0,
            Vec3::new(0.4, 0.5, 1.0),
        );
        list.shadows.enabled = true;
        list.ambient.intensity = 0.3;
        let mut renderer = TextureRenderer::new(&device, &queue, size, size);
        renderer.set_graphics(&GraphicsSettings {
            anti_aliasing: Some(mode),
            ..GraphicsSettings::preset(Quality::High)
        });
        for _ in 0..frames {
            renderer.render(&list).unwrap();
        }
        evidence(
            &format!(
                "engine-card-over-skin-{}",
                format!("{mode:?}").to_lowercase()
            ),
            &renderer.read_pixels(),
            size,
            size,
        );
    }
}

#[test]
#[ignore = "times the GPU; run by hand on the reference PC"]
fn bench_hair_and_cloth_models() {
    let Some((device, queue)) = timed_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let models: Vec<(&str, crate::Material)> = vec![
        ("standard", cloth(|_| {}).with(Shading::Standard)),
        (
            "skin",
            cloth(|_| {}).with(Shading::Skin(SkinShading::default())),
        ),
        ("hair", hair([0.45, 0.3, 0.18], 0.35, |_| {})),
        (
            "cloth (sheen)",
            cloth(|cloth| {
                cloth.sheen_colour = [1.0; 3];
            }),
        ),
        (
            "cloth (sheen, anisotropy)",
            cloth(|cloth| {
                cloth.sheen_colour = [1.0; 3];
                cloth.anisotropy = 0.8;
            }),
        ),
    ];
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied().unwrap_or(f64::NAN)
    };
    let time = |list: &DrawList, mode: AntiAliasing| {
        let mut renderer = TextureRenderer::new(&device, &queue, 1920, 1080);
        renderer.set_graphics(&GraphicsSettings {
            anti_aliasing: Some(mode),
            ..GraphicsSettings::preset(Quality::High)
        });
        let mut scene = Vec::new();
        let mut shadows = Vec::new();
        let mut depth = Vec::new();
        for frame in 0..240 {
            renderer.render(list).unwrap();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            if frame < 40 {
                continue;
            }
            let stats = renderer.stats();
            let ms = |pass| stats.pass_time(pass).map(|t| t.as_secs_f64() * 1000.0);
            scene.extend(ms(FramePass::Scene));
            shadows.extend(ms(FramePass::Shadows));
            depth.extend(ms(FramePass::Prepass));
        }
        (median(scene), median(shadows), median(depth))
    };
    // A ball of each model filling the view, under the sun with shadows.
    for (name, material) in &models {
        let mut list = dark_scene(
            vec![ball(1.0, Vec3::ZERO, material.clone())],
            2.0,
            60.0,
            Vec3::new(0.4, 0.5, 1.0),
        );
        list.shadows.enabled = true;
        list.ambient.intensity = 0.3;
        let (scene, shadows, prepass) = time(&list, AntiAliasing::Taa);
        println!(
            "ball, {name}: scene {scene:.3} ms, shadows {shadows:.3} ms, prepass {prepass:.3} ms"
        );
    }
    // Hair cards: 24 cut-out cards in layers over the view, as a card
    // hairstyle seen close, and the same cards drawn Standard.
    let data = hair_data(0x0be1);
    for (name, shading) in [("hair cards", true), ("standard cut-out cards", false)] {
        let mut items = Vec::new();
        for layer in 0..24 {
            let mut material = hair([0.45, 0.3, 0.18], 0.35, |hair| {
                hair.data = Some(data.clone())
            });
            if !shading {
                material.shading = Shading::Standard;
            }
            material.base_color_texture = Some(super::card_set::striped(0x0be2, 16));
            material.alpha_cutoff = 0.5;
            let at = layer as f32;
            items.push(card(
                Vec3::new((at * 0.37).sin() * 0.3, (at * 0.53).cos() * 0.3, -0.02 * at),
                [3.0, 3.0],
                material,
            ));
        }
        let mut list = dark_scene(items, 2.0, 60.0, Vec3::new(0.4, 0.5, 1.0));
        list.shadows.enabled = true;
        list.ambient.intensity = 0.3;
        for mode in [AntiAliasing::Taa, AntiAliasing::Msaa4] {
            let (scene, shadows, prepass) = time(&list, mode);
            println!(
                "{name}, {mode:?}: scene {scene:.3} ms, shadows {shadows:.3} ms, prepass {prepass:.3} ms"
            );
        }
    }
}

trait With {
    fn with(self, shading: Shading) -> Self;
}

impl With for crate::Material {
    fn with(self, shading: Shading) -> Self {
        Self { shading, ..self }
    }
}
