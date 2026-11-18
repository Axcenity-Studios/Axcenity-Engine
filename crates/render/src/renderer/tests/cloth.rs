//! Cloth shading (`Shading::Cloth`, CO7): its uniform; the sheen
//! brightening the cloth at grazing angles and not head-on; anisotropy
//! stretching the highlight along its direction; and Standard drawing as
//! before (the probe in `skin.rs` holds it for every other model).

use glam::{Vec2, Vec3};

use super::card_set::{ball, cloth, dark_scene, on_screen, rgb_at, sum};
use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};
use crate::{ClothShading, GraphicsSettings, SURFACE_PARAMETERS, Shading};

/// The frame size, square.
const SIZE: u32 = 192;

fn renderer(device: &wgpu::Device, queue: &wgpu::Queue) -> TextureRenderer {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(AntiAliasing::Off),
        ambient_occlusion: Some(AmbientOcclusion::Off),
        ..GraphicsSettings::preset(Quality::High)
    });
    renderer
}

fn draw(renderer: &mut TextureRenderer, list: &DrawList) -> Vec<u8> {
    renderer.render(list).unwrap();
    renderer.read_pixels()
}

#[test]
fn a_cloth_materials_sheen_and_anisotropy_ride_its_uniform() {
    let mut item = item(Shape::Block, Vec3::ZERO, 1.0);
    item.material.shading = Shading::Cloth(ClothShading {
        sheen_colour: [1.0, 0.5, 0.0],
        sheen_roughness: 0.3,
        anisotropy: 0.6,
        anisotropy_rotation: 0.5,
    });
    let maps = maps_uniform_of(&material_key(&item));
    assert_eq!(maps.shading, [SHADING_CLOTH, 0.3, 0.6, 0.5]);
    // The sheen's colour in linear light, in the toon row (cloth has none).
    assert_eq!(maps.toon[0], 1.0);
    assert!((maps.toon[1] - 0.214).abs() < 0.01, "{:?}", maps.toon);
    assert_eq!(maps.toon[2], 0.0);
    // The surface shader's values are left to a surface shader.
    assert_eq!(maps.params, [[0.0; 4]; SURFACE_PARAMETERS]);
}

/// Cloth minus the same ball drawn Standard, along the middle row from
/// the ball's middle out to its rim, lit from the camera's side.
fn sheen_along_the_middle(renderer: &mut TextureRenderer, sheen: [f32; 3]) -> Vec<f32> {
    let lit = |renderer: &mut TextureRenderer, material: crate::Material| {
        let list = dark_scene(
            vec![ball(1.0, Vec3::ZERO, material)],
            6.0,
            30.0,
            Vec3::new(0.0, 0.0, 1.0),
        );
        (draw(renderer, &list), list)
    };
    let (standard, list) = lit(renderer, cloth(|_| {}).with_shading(Shading::Standard));
    let (sheened, _) = lit(
        renderer,
        cloth(|cloth| {
            cloth.sheen_colour = sheen;
            cloth.sheen_roughness = 0.5;
        }),
    );
    let middle = on_screen(&list, Vec3::ZERO, SIZE);
    (0..10)
        .map(|step| {
            // From the middle out to 95% of the radius.
            let reach = 0.95 * step as f32 / 9.0;
            let p = on_screen(
                &list,
                Vec3::new(reach, 0.0, (1.0 - reach * reach).sqrt()),
                SIZE,
            );
            let at = Vec2::new(p.x, middle.y);
            sum(rgb_at(&sheened, SIZE, at.x as u32, at.y as u32))
                - sum(rgb_at(&standard, SIZE, at.x as u32, at.y as u32))
        })
        .collect()
}

#[test]
fn sheen_brightens_cloth_at_grazing_angles_and_not_head_on() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    let added = sheen_along_the_middle(&mut renderer, [1.0; 3]);
    eprintln!("sheen's light from the middle out: {added:?}");
    let (head_on, grazing) = (added[0], added[9]);
    assert!(head_on.abs() <= 12.0, "little head-on: {added:?}");
    assert!(
        grazing > head_on + 25.0,
        "brighter at grazing angles: {added:?}"
    );
    // No sheen draws the cloth as Standard.
    let none = sheen_along_the_middle(&mut renderer, [0.0; 3]);
    assert!(none.iter().all(|v| v.abs() <= 3.0), "{none:?}");
}

#[test]
fn anisotropy_stretches_the_highlight_along_its_direction() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = renderer(&device, &queue);
    // A shiny dark ball (satin) lit from beside the camera; its tangent
    // (the sphere's U) runs across the picture at its middle.
    let spread = |renderer: &mut TextureRenderer, anisotropy: f32, rotation: f32| {
        let mut material = cloth(|cloth| {
            cloth.anisotropy = anisotropy;
            cloth.anisotropy_rotation = rotation;
        });
        material.base_color = [0.05; 3];
        material.roughness = 0.3;
        let list = dark_scene(
            vec![ball(1.0, Vec3::ZERO, material)],
            6.0,
            30.0,
            Vec3::new(0.0, 0.0, 1.0),
        );
        let pixels = draw(renderer, &list);
        let brightest = (0..SIZE * SIZE)
            .map(|at| sum(rgb_at(&pixels, SIZE, at % SIZE, at / SIZE)))
            .fold(0.0, f32::max);
        let points: Vec<Vec2> = (0..SIZE * SIZE)
            .filter(|&at| sum(rgb_at(&pixels, SIZE, at % SIZE, at / SIZE)) > 0.5 * brightest)
            .map(|at| Vec2::new((at % SIZE) as f32, (at / SIZE) as f32))
            .collect();
        let mean = points.iter().copied().sum::<Vec2>() / points.len() as f32;
        let spread = points
            .iter()
            .map(|p| (*p - mean) * (*p - mean))
            .sum::<Vec2>()
            / points.len() as f32;
        (spread.x.sqrt(), spread.y.sqrt())
    };
    let (across, down) = spread(&mut renderer, 0.0, 0.0);
    eprintln!("isotropic: {across:.1} across, {down:.1} down");
    assert!(
        (across / down - 1.0).abs() < 0.25,
        "round: {across} vs {down}"
    );
    let (across, down) = spread(&mut renderer, 0.8, 0.0);
    eprintln!("along U: {across:.1} across, {down:.1} down");
    assert!(
        across > 1.6 * down,
        "stretched along the tangent: {across} vs {down}"
    );
    let (across, down) = spread(&mut renderer, 0.8, std::f32::consts::FRAC_PI_2);
    eprintln!("turned a quarter: {across:.1} across, {down:.1} down");
    assert!(
        down > 1.6 * across,
        "stretched along the turned direction: {across} vs {down}"
    );
}

trait WithShading {
    fn with_shading(self, shading: Shading) -> Self;
}

impl WithShading for crate::Material {
    fn with_shading(self, shading: Shading) -> Self {
        Self { shading, ..self }
    }
}

/// The Charlie sheen's directional albedo for a viewer at cosine `mu` and
/// sheen roughness `roughness`: its lobe with Neubelt's visibility,
/// integrated over the hemisphere of light directions (a midpoint sum of
/// `steps` by twice as many).
fn sheen_albedo(mu: f64, roughness: f64, steps: usize) -> f64 {
    let alpha = roughness * roughness;
    let mu = mu.max(0.02);
    let v = [(1.0 - mu * mu).sqrt(), 0.0, mu];
    let (d_theta, d_phi) = (
        std::f64::consts::FRAC_PI_2 / steps as f64,
        std::f64::consts::TAU / (2 * steps) as f64,
    );
    let mut total = 0.0;
    for i in 0..steps {
        let theta = (i as f64 + 0.5) * d_theta;
        let (st, ct) = theta.sin_cos();
        for j in 0..2 * steps {
            let phi = (j as f64 + 0.5) * d_phi;
            let l = [st * phi.cos(), st * phi.sin(), ct];
            let h = [v[0] + l[0], v[1] + l[1], v[2] + l[2]];
            let n_dot_h = h[2] / (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt();
            let sin2 = (1.0 - n_dot_h * n_dot_h).max(0.0);
            let charlie = (2.0 + 1.0 / alpha) * sin2.powf(0.5 / alpha) / std::f64::consts::TAU;
            let neubelt = 1.0 / (4.0 * (ct + mu - ct * mu));
            total += charlie * neubelt * ct * st * d_theta * d_phi;
        }
    }
    total
}

#[test]
fn the_sheen_albedo_table_is_charlies_integrated() {
    let (rows, columns) = (8, 16);
    let table: Vec<f64> = (0..rows)
        .flat_map(|row| {
            let roughness = 0.1 + 0.9 * row as f64 / (rows - 1) as f64;
            (0..columns).map(move |column| {
                sheen_albedo(column as f64 / (columns - 1) as f64, roughness, 96).min(1.0)
            })
        })
        .collect();
    let source = include_str!("../../cloth.wgsl");
    let start = source.find("array<f32, 128>(").unwrap() + "array<f32, 128>(".len();
    let end = start + source[start..].find(");").unwrap();
    let held: Vec<f64> = source[start..end]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.parse().unwrap())
        .collect();
    let wrong = held.len() != table.len()
        || held
            .iter()
            .zip(&table)
            .any(|(held, made)| (held - made).abs() > 0.002);
    if wrong {
        let text: Vec<String> = table
            .chunks(columns)
            .map(|row| {
                let values: Vec<String> = row.iter().map(|v| format!("{v:.4}")).collect();
                format!("    {},", values.join(", "))
            })
            .collect();
        panic!(
            "cloth.wgsl's sheen table is out of date; it should be:\n{}",
            text.join("\n")
        );
    }
}
