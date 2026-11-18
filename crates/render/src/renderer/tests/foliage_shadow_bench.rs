//! What cut-out-aware shadow maps (CO7) cost foliage, run by hand: a grove
//! of 24 trees of 1,500 cut-out leaf cards each (36,000 cards) on a ground,
//! under a low sun with its cascades, at 1080p High with TAA. It prints the median
//! Shadows, Prepass and Scene passes (`FramePass`) over 200 frames.
//!
//! Self-contained (its own leaf texture and card mesh), so the same file
//! builds on a revision before cut-out-aware shadows for a before and
//! after. `cargo test -p axcenity-render --release -- --ignored
//! bench_cut_out_foliage_shadows --nocapture`; results in
//! `docs/rendering/shadows.md` and the wiki's performance history.

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use super::*;
use crate::graphics::{AntiAliasing, Quality};
use crate::{FramePass, GraphicsSettings};

/// A leaf texture: five round leaves on a clear card, about 45% covered.
struct Leaves {
    rgba: Vec<u8>,
}

const LEAF_TEXELS: u32 = 64;

impl Leaves {
    fn shared() -> Arc<dyn TextureSource> {
        let centres = [
            (0.3, 0.3, 0.2),
            (0.7, 0.28, 0.18),
            (0.5, 0.55, 0.2),
            (0.25, 0.75, 0.17),
            (0.75, 0.75, 0.19),
        ];
        let size = LEAF_TEXELS;
        let rgba = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let (u, v) = (
                    (x as f32 + 0.5) / size as f32,
                    (y as f32 + 0.5) / size as f32,
                );
                let inside = centres
                    .iter()
                    .any(|&(cu, cv, r)| (u - cu).powi(2) + (v - cv).powi(2) < r * r);
                [60, 110, 40, if inside { 255 } else { 0 }]
            })
            .collect();
        Arc::new(Leaves { rgba })
    }
}

impl TextureSource for Leaves {
    fn key(&self) -> u64 {
        0x1eaf_5eed
    }
    fn size(&self) -> (u32, u32) {
        (LEAF_TEXELS, LEAF_TEXELS)
    }
    fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// A unit square card facing +Z.
struct LeafCard;

impl MeshSource for LeafCard {
    fn key(&self) -> u64 {
        0x1eaf_ca4d
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
        &[[1.0, 0.0, 0.0, -1.0]; 4]
    }
    fn indices(&self) -> &[u32] {
        &[0, 3, 2, 0, 2, 1]
    }
}

/// A number from 0 to 1, the same every run.
fn next(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state >> 40) as f32 / (1u64 << 24) as f32
}

/// 24 trees (six rows of four, 12 studs apart), each a crown of 1,500 leaf
/// cards (1.5 studs, turned every way) in a ball of radius 5 on a trunk,
/// over a 200-stud ground, seen along the grove, the sun about 40° up.
fn forest() -> DrawList {
    let texture = Leaves::shared();
    let mesh: Arc<dyn MeshSource> = Arc::new(LeafCard);
    let mut items = vec![DrawItem {
        shape: Shape::Block,
        transform: Mat4::from_scale_rotation_translation(
            Vec3::new(200.0, 1.0, 200.0),
            Quat::IDENTITY,
            Vec3::new(0.0, -0.5, -40.0),
        ),
        ..item(Shape::Block, Vec3::ZERO, 1.0)
    }];
    let mut state = 0x5eed;
    for tree in 0..24 {
        let base = Vec3::new(
            -18.0 + 12.0 * (tree % 4) as f32,
            0.0,
            -12.0 - 12.0 * (tree / 4) as f32,
        );
        items.push(DrawItem {
            shape: Shape::Cylinder,
            transform: Mat4::from_scale_rotation_translation(
                Vec3::new(1.0, 8.0, 1.0),
                Quat::IDENTITY,
                base + Vec3::new(0.0, 4.0, 0.0),
            ),
            ..item(Shape::Cylinder, Vec3::ZERO, 1.0)
        });
        for _ in 0..1500 {
            let direction = Vec3::new(
                next(&mut state) * 2.0 - 1.0,
                next(&mut state) * 2.0 - 1.0,
                next(&mut state) * 2.0 - 1.0,
            )
            .normalize_or_zero();
            let at = base + Vec3::new(0.0, 11.0, 0.0) + direction * 5.0 * next(&mut state).sqrt();
            let turn = Quat::from_euler(
                glam::EulerRot::YXZ,
                next(&mut state) * std::f32::consts::TAU,
                next(&mut state) * std::f32::consts::PI,
                next(&mut state) * std::f32::consts::TAU,
            );
            items.push(DrawItem {
                shape: Shape::Mesh(mesh.clone()),
                transform: Mat4::from_scale_rotation_translation(Vec3::splat(1.5), turn, at),
                material: Material {
                    base_color_texture: Some(texture.clone()),
                    alpha_cutoff: 0.5,
                    double_sided: true,
                    ..Material::default()
                },
                ..item(Shape::Block, Vec3::ZERO, 1.0)
            });
        }
    }
    for (index, item) in items.iter_mut().enumerate() {
        item.id = index as u64 + 1;
    }
    let mut list = list_of(items);
    list.camera = Camera {
        position: Vec3::new(0.0, 7.0, 10.0),
        look_at: Vec3::new(0.0, 9.0, -30.0),
        fov_y_degrees: 70.0,
    };
    list.sun.direction = Vec3::new(0.7, 0.75, 0.45).normalize();
    list.sun.intensity = 2.0;
    list.ambient.intensity = 0.3;
    list.shadows.enabled = true;
    list
}

#[test]
#[ignore = "times the GPU; run by hand on the reference PC"]
fn bench_cut_out_foliage_shadows() {
    let instance = crate::adapters::instance();
    let Ok(adapter) = pollster::block_on(crate::adapters::adapter(&instance, None)) else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let info = adapter.get_info();
    println!("adapter: {} ({:?})", info.name, info.backend);
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("foliage bench device"),
        required_features: adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TEXTURE_COMPRESSION_BC),
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).unwrap();
    let list = forest();
    let mut renderer = TextureRenderer::new(&device, &queue, 1920, 1080);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(AntiAliasing::Taa),
        ..GraphicsSettings::preset(Quality::High)
    });
    let median = |mut values: Vec<f64>| {
        values.sort_by(f64::total_cmp);
        values.get(values.len() / 2).copied().unwrap_or(f64::NAN)
    };
    let (mut shadows, mut prepass, mut scene) = (Vec::new(), Vec::new(), Vec::new());
    for frame in 0..240 {
        renderer.render(&list).unwrap();
        if frame < 40 {
            continue;
        }
        let stats = renderer.stats();
        let ms = |pass| stats.pass_time(pass).map(|t| t.as_secs_f64() * 1000.0);
        shadows.extend(ms(FramePass::Shadows));
        prepass.extend(ms(FramePass::Prepass));
        scene.extend(ms(FramePass::Scene));
    }
    // The frame, to see what was timed.
    if let Some(folder) = std::env::var_os("AXCENITY_RENDER_EVIDENCE") {
        let mut bytes = b"P6
1920 1080
255
"
        .to_vec();
        for pixel in renderer.read_pixels().chunks(4) {
            bytes.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(std::path::Path::new(&folder).join("foliage.ppm"), bytes).unwrap();
    }
    println!(
        "foliage, 36,000 cut-out leaf cards: shadows {:.3} ms, prepass {:.3} ms, scene {:.3} ms",
        median(shadows),
        median(prepass),
        median(scene)
    );
}
