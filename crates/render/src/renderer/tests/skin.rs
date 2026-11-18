//! Skin shading (`Shading::Skin`, R5): its binding and uniform; light
//! spreading past the line where it ends, redder than green and blue; a
//! bumpy normal map's pores left out of its diffuse light; the defined
//! normal map blended by `normal_blend`; and no other material drawn any
//! differently for it.

use super::*;
use crate::graphics::{AmbientOcclusion, AntiAliasing, Quality};
use crate::{GraphicsSettings, Shading, SkinShading, Toon};
use glam::Vec2;

/// The frame size, square.
const SIZE: u32 = 128;

/// A texture of its own key (the shared test textures share theirs).
struct Map {
    key: u64,
    size: u32,
    rgba: Vec<u8>,
}

impl Map {
    fn shared(key: u64, size: u32, texel: impl Fn(u32, u32) -> [u8; 4]) -> Arc<dyn TextureSource> {
        let rgba = (0..size)
            .flat_map(|y| (0..size).map(move |x| (x, y)))
            .flat_map(|(x, y)| texel(x, y))
            .collect();
        Arc::new(Map { key, size, rgba })
    }
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

/// A normal map leaning every texel `degrees` toward +X (negative: -X).
fn leaning(key: u64, degrees: f32) -> Arc<dyn TextureSource> {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0).round() as u8;
    Map::shared(key, 4, move |_, _| [byte(sin), byte(0.0), byte(cos), 255])
}

/// A normal map of bumps leaning every way up to about 35 degrees, a
/// different way on each texel (as `shading.rs`'s).
fn bumpy(key: u64) -> Arc<dyn TextureSource> {
    let mut seed = 0x2545_f491_u32;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let texels: Vec<[u8; 4]> = (0..32 * 32)
        .map(|_| {
            let (x, y) = (next() * 0.6, next() * 0.6);
            let z = (1.0 - x * x - y * y).sqrt();
            let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0).round() as u8;
            [byte(x), byte(y), byte(z), 255]
        })
        .collect();
    Map::shared(key, 32, move |x, y| texels[(y * 32 + x) as usize])
}

fn normal(texture: Arc<dyn TextureSource>) -> Option<NormalMap> {
    Some(NormalMap {
        texture,
        scale: 1.0,
        flip_green: false,
    })
}

fn skin(defined: Option<Arc<dyn TextureSource>>, blend: f32) -> Shading {
    Shading::Skin(SkinShading {
        defined_normal: defined,
        normal_blend: blend,
        ..SkinShading::default()
    })
}

fn ball(radius: f32, x: f32, shading: Shading) -> DrawItem {
    DrawItem {
        transform: Mat4::from_scale_rotation_translation(
            Vec3::splat(radius * 2.0),
            glam::Quat::IDENTITY,
            Vec3::new(x, 0.0, 0.0),
        ),
        material: Material {
            base_color: [0.8; 3],
            roughness: 0.6,
            shading,
            ..Material::default()
        },
        ..item(Shape::Sphere, Vec3::ZERO, 1.0)
    }
}

/// `items` in the dark (no sky, no ambient light, no shadows), seen from
/// `distance` along +Z through a lens `fov` degrees high, lit by a sun from
/// straight right.
fn scene(items: Vec<DrawItem>, distance: f32, fov: f32) -> DrawList {
    let mut list = list_of(items);
    list.camera = Camera {
        position: Vec3::new(0.0, 0.0, distance),
        look_at: Vec3::ZERO,
        fov_y_degrees: fov,
    };
    list.sun.direction = Vec3::X;
    list.sun.intensity = 2.0;
    list.shadows.enabled = false;
    list
}

fn renderer(device: &wgpu::Device, queue: &wgpu::Queue) -> TextureRenderer {
    let mut renderer = TextureRenderer::new(device, queue, SIZE, SIZE);
    renderer.set_graphics(&GraphicsSettings {
        anti_aliasing: Some(AntiAliasing::Off),
        ambient_occlusion: Some(AmbientOcclusion::Off),
        ..GraphicsSettings::preset(Quality::High)
    });
    renderer
}

fn on_screen(list: &DrawList, point: Vec3) -> Vec2 {
    let clip = view_projection(list, 1.0) * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    Vec2::new(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * SIZE as f32
}

fn pixel(pixels: &[u8], at: Vec2) -> [u8; 4] {
    let (x, y) = (at.x as usize, at.y as usize);
    pixels[(y * SIZE as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}

fn brightness(pixel: [u8; 4]) -> i32 {
    pixel[..3].iter().map(|&c| i32::from(c)).sum()
}

fn draw(renderer: &mut TextureRenderer, list: &DrawList) -> Vec<u8> {
    renderer.render(list).unwrap();
    renderer.read_pixels()
}

#[test]
fn a_skin_materials_shading_picks_its_binding_and_its_defined_map_takes_the_specular_maps_place() {
    let plain = item(Shape::Block, Vec3::ZERO, 1.0);
    let shaded = |shading: Shading| {
        let mut item = plain.clone();
        item.material.shading = shading;
        item.material.maps.specular = Some(Arc::new(TestTexture(9)));
        item
    };
    let key = material_key(&shaded(skin(None, 0.7)));
    assert_ne!(key, MaterialKey::default());
    let uniform = maps_uniform_of(&key);
    assert_eq!(
        uniform.shading,
        [SHADING_SKIN, 0.0, 1.0, 0.0],
        "without a defined map the blend is 0"
    );
    assert_eq!(uniform.specular[0], 0.0, "skin reads no specular map");
    assert_eq!(key.textures[7], None, "nor binds one");

    let defined = leaning(0x5e_01, 20.0);
    let key = material_key(&shaded(Shading::Skin(SkinShading {
        scatter: 1.5,
        defined_normal: Some(defined.clone()),
        normal_blend: 0.4,
        layers: None,
    })));
    assert_eq!(key.textures[7], Some(defined.key()), "the defined map");
    assert_eq!(maps_uniform_of(&key).shading, [SHADING_SKIN, 0.4, 1.5, 0.0]);
    // Held to 0 to 1 and 0 to 4.
    let key = material_key(&shaded(Shading::Skin(SkinShading {
        scatter: -1.0,
        defined_normal: Some(defined),
        normal_blend: 3.0,
        layers: None,
    })));
    assert_eq!(maps_uniform_of(&key).shading, [SHADING_SKIN, 1.0, 0.0, 0.0]);
}

/// The red and green of the pixels across the middle of a 12 mm ball (a
/// nose's tip) lit from straight right, from its lit side to its dark one.
fn across_a_small_ball(shading: Shading) -> Vec<[u8; 4]> {
    across_a_ball(0.006, shading, |_| {})
}

/// [`across_a_small_ball`] for a ball of `radius_m` metres, filling the
/// frame as much as the 6 mm one does, its list changed by `change` first.
fn across_a_ball(radius_m: f32, shading: Shading, change: impl Fn(&mut DrawList)) -> Vec<[u8; 4]> {
    let Some((device, queue)) = test_device() else {
        return Vec::new();
    };
    // In studs (a stud is 0.35 m).
    let radius = radius_m / 0.35;
    let mut list = scene(
        vec![ball(radius, 0.0, shading)],
        0.3,
        8.0 * radius_m / 0.006,
    );
    change(&mut list);
    let mut renderer = renderer(&device, &queue);
    let pixels = draw(&mut renderer, &list);
    let (start, end) = (
        on_screen(&list, Vec3::new(radius * 0.9, 0.0, 0.0)),
        on_screen(&list, Vec3::new(-radius * 0.9, 0.0, 0.0)),
    );
    let steps = (end - start).abs().max_element().ceil() as u32;
    (0..=steps)
        .map(|step| pixel(&pixels, start.lerp(end, step as f32 / steps as f32)))
        .collect()
}

/// The mean red over green of the pixels whose green is between 5% and 35%
/// of the brightest green: the band where the light ends.
fn red_where_light_ends(row: &[[u8; 4]]) -> f32 {
    let top = row.iter().map(|p| p[1]).max().unwrap_or(0) as f32;
    let band: Vec<f32> = row
        .iter()
        .filter(|p| (top * 0.05..=top * 0.35).contains(&(p[1] as f32)))
        .map(|p| p[0] as f32 / p[1].max(1) as f32)
        .collect();
    assert!(band.len() >= 3, "no band where the light ends: {row:?}");
    band.iter().sum::<f32>() / band.len() as f32
}

#[test]
fn light_spreads_past_where_it_ends_on_skin_further_in_red() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let standard = across_a_small_ball(Shading::Standard);
    let skinned = across_a_small_ball(skin(None, 0.0));
    let (plain, red) = (
        red_where_light_ends(&standard),
        red_where_light_ends(&skinned),
    );
    eprintln!("red over green where the light ends: standard {plain}, skin {red}");
    assert!(plain < 1.04, "a grey standard ball stays grey: {plain}");
    // Measured 1.20 on a 6 mm radius: the profile itself (red spreads
    // about three times as far as green), not a tint.
    assert!(red > 1.12, "skin is redder where the light ends: {red}");
    // And it reaches further round the ball: more of the row is lit.
    let lit = |row: &[[u8; 4]]| row.iter().filter(|p| p[0] > 4).count();
    assert!(
        lit(&skinned) > lit(&standard),
        "lit pixels: standard {}, skin {}",
        lit(&standard),
        lit(&skinned)
    );
    // With no scatter, nothing spreads.
    let none = across_a_small_ball(Shading::Skin(SkinShading {
        scatter: 0.0,
        ..SkinShading::default()
    }));
    let unspread = red_where_light_ends(&none);
    assert!(unspread < 1.04, "no scatter, no red: {unspread}");
}

/// The mean red over green of the lit pixels (green above half the
/// brightest's).
fn red_where_lit(row: &[[u8; 4]]) -> f32 {
    let top = row.iter().map(|p| p[1]).max().unwrap_or(0) as f32;
    let lit: Vec<f32> = row
        .iter()
        .filter(|p| p[1] as f32 > top * 0.5)
        .map(|p| p[0] as f32 / p[1].max(1) as f32)
        .collect();
    assert!(lit.len() >= 3, "nothing lit: {row:?}");
    lit.iter().sum::<f32>() / lit.len() as f32
}

/// The mean of red, green and blue over the lit pixels of `row`.
fn lit_brightness(row: &[[u8; 4]]) -> f32 {
    let lit: Vec<f32> = row
        .iter()
        .filter(|p| p[1] > 8)
        .map(|&p| brightness(p) as f32)
        .collect();
    lit.iter().sum::<f32>() / lit.len().max(1) as f32
}

/// The sun a third of the way up from straight right (so clouds can
/// shade it), at `intensity`, under an overcast sky of thin cloud: about
/// half the sun's light gets through, the same everywhere on a small ball.
fn under_clouds(clouded: bool, intensity: f32) -> impl Fn(&mut DrawList) {
    move |list: &mut DrawList| {
        list.sun.direction = Vec3::new(1.0, 0.35, 0.0).normalize();
        list.sun.intensity = intensity;
        if clouded {
            list.clouds = crate::Clouds {
                cover: 1.0,
                density: CLOUD_DENSITY,
                ..crate::Clouds::default()
            };
        }
    }
}

/// Thin enough that about half the sun gets through (`CLOUD_LIGHT`).
const CLOUD_DENSITY: f32 = 0.0165;
/// How much of the sun's light gets through [`CLOUD_DENSITY`] at full
/// cover (clouds.wgsl's `cloud_shadow`: an optical depth of 0.05 a stud at
/// density 1, through the 400-stud layer, over the sun's height 0.33).
fn cloud_light() -> f32 {
    let up = Vec3::new(1.0, 0.35, 0.0).normalize().y.max(0.25);
    let depth = 0.05 * CLOUD_DENSITY * 400.0 / up;
    0.7 * (-depth).exp() + 0.3 * (-0.25 * depth).exp()
}

#[test]
fn a_cloud_shadow_dims_skin_without_reddening_it() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let through = cloud_light();
    // The clouds really take about half the sun away (a grey ball).
    let open = across_a_ball(0.006, Shading::Standard, under_clouds(false, 2.0));
    let shaded = across_a_ball(0.006, Shading::Standard, under_clouds(true, 2.0));
    let (open_light, shaded_light) = (lit_brightness(&open), lit_brightness(&shaded));
    assert!(
        shaded_light < 0.8 * open_light,
        "the clouds shade the ball: open {open_light}, under clouds {shaded_light}"
    );
    // Skin in the open, and under the clouds with the sun made brighter
    // by as much as they take: the same light arrives, so the same
    // picture. Clouds are grey, not a shadow's edge: they mustn't redden
    // the skin (it would turn redder all over when a cloud passes).
    let skin = || skin(None, 0.0);
    let open = across_a_ball(0.006, skin(), under_clouds(false, 2.0));
    let shaded = across_a_ball(0.006, skin(), under_clouds(true, 2.0 / through));
    let (lit_open, lit_shaded) = (red_where_lit(&open), red_where_lit(&shaded));
    let (edge_open, edge_shaded) = (red_where_light_ends(&open), red_where_light_ends(&shaded));
    eprintln!(
        "{through} of the sun through the clouds; red over green lit: open {lit_open}, under clouds {lit_shaded}; where the light ends: open {edge_open}, under clouds {edge_shaded}"
    );
    assert!(
        (lit_shaded - lit_open).abs() < 0.03,
        "lit: open {lit_open}, under clouds {lit_shaded}"
    );
    assert!(
        (edge_shaded - edge_open).abs() < 0.03,
        "where the light ends: open {edge_open}, under clouds {edge_shaded}"
    );
}

#[test]
fn a_crease_sharper_than_skins_smallest_radius_is_no_redder_than_it() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    // A 1 mm ball (a nostril's rim, a crease) is drawn as skin's smallest
    // radius, 4 mm: past it the table reddens a crease's shading
    // as no real skin does at that size.
    let tiny = red_where_light_ends(&across_a_ball(0.001, skin(None, 0.0), |_| {}));
    let smallest = red_where_light_ends(&across_a_ball(0.004, skin(None, 0.0), |_| {}));
    eprintln!("red over green where the light ends: 1 mm {tiny}, 4 mm {smallest}");
    assert!(tiny < smallest + 0.03, "1 mm {tiny}, 4 mm {smallest}");
}

/// The inside of half a cylinder standing upright, open toward +Z: a
/// groove (a crease, an ear's fold), in the unit cube. Its normals face in,
/// toward its axis.
struct Groove {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Groove {
    fn new() -> Self {
        let (around, up) = (48u32, 8u32);
        let mut groove = Groove {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        };
        for j in 0..=up {
            for i in 0..=around {
                // From the left edge (x -0.5) round the back to the right.
                let angle = (i as f32 / around as f32 - 0.5) * std::f32::consts::PI;
                let (sin, cos) = angle.sin_cos();
                let y = j as f32 / up as f32 - 0.5;
                groove.positions.push([0.5 * sin, y, -0.5 * cos]);
                groove.normals.push([-sin, 0.0, cos]);
                groove
                    .uvs
                    .push([i as f32 / around as f32, j as f32 / up as f32]);
            }
        }
        let row = around + 1;
        for j in 0..up {
            for i in 0..around {
                let at = j * row + i;
                // Counter-clockwise seen from +Z.
                groove
                    .indices
                    .extend([at, at + 1, at + row + 1, at, at + row + 1, at + row]);
            }
        }
        groove
    }
}

impl MeshSource for Groove {
    fn key(&self) -> u64 {
        0x5e_9700
    }
    fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
    fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &self.uvs
    }
    fn indices(&self) -> &[u32] {
        &self.indices
    }
}

/// The pixels across a groove 12 mm wide lit from straight right, from
/// its lit (left) side to its dark one.
fn across_a_groove(shading: Shading) -> Vec<[u8; 4]> {
    let Some((device, queue)) = test_device() else {
        return Vec::new();
    };
    let radius = 0.006 / 0.35;
    let mut groove = ball(radius, 0.0, shading);
    groove.shape = Shape::Mesh(Arc::new(Groove::new()));
    let list = scene(vec![groove], 0.3, 8.0);
    let mut renderer = renderer(&device, &queue);
    let pixels = draw(&mut renderer, &list);
    let back = -radius * (1.0f32 - 0.81).sqrt();
    let (start, end) = (
        on_screen(&list, Vec3::new(-radius * 0.9, 0.0, back)),
        on_screen(&list, Vec3::new(radius * 0.9, 0.0, back)),
    );
    let steps = (end - start).abs().max_element().ceil() as u32;
    (0..=steps)
        .map(|step| pixel(&pixels, start.lerp(end, step as f32 / steps as f32)))
        .collect()
}

#[test]
fn light_ending_in_a_groove_isnt_reddened_as_on_a_ball() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    // The table is pre-integrated round a ball's curve: light reaches past
    // where it ends because the surface curves away from it. A crease
    // curves the other way (an ear's fold, a nostril, where the nose meets
    // the lip), and drawn as a ball it took a band of saturated red.
    let standard = across_a_groove(Shading::Standard);
    let skinned = across_a_groove(skin(None, 0.0));
    assert!(
        standard.iter().any(|p| p[1] > 40),
        "the groove is drawn: {standard:?}"
    );
    let (plain, red) = (
        red_where_light_ends(&standard),
        red_where_light_ends(&skinned),
    );
    eprintln!("red over green where the light ends in a groove: standard {plain}, skin {red}");
    assert!(plain < 1.04, "a grey standard groove stays grey: {plain}");
    assert!(red < 1.04, "skin in a groove isn't reddened: {red}");
}

/// How much each pixel's brightness differs from its right neighbour's,
/// on the lit half of a bumpy ball 8 studs wide lit from the right, and
/// across the line where its light ends: (lit, terminator).
fn grain(shading: Shading, bumpy_map: bool) -> (f32, f32) {
    let Some((device, queue)) = test_device() else {
        return (0.0, 0.0);
    };
    let radius = 4.0;
    let mut ball = ball(radius, 0.0, shading);
    // Matte, so what's measured is the diffuse light (skin's specular
    // light keeps the pores, as real skin's highlights do).
    ball.material.roughness = 1.0;
    if bumpy_map {
        ball.material.maps.normal = normal(bumpy(0x5e_b0));
    }
    // About 3 pixels a texel.
    ball.material.uv_scale = [16.0, 16.0];
    let list = scene(vec![ball], 18.0, 30.0);
    let mut renderer = renderer(&device, &queue);
    let pixels = draw(&mut renderer, &list);
    let at = |side: f32, row: f32| {
        let (x, y) = (side * radius, row * radius);
        let z = (radius * radius - x * x - y * y).max(0.0).sqrt();
        brightness(pixel(&pixels, on_screen(&list, Vec3::new(x, y, z))))
    };
    let noise = |sides: std::ops::Range<i32>| {
        let mut steps = Vec::new();
        for row in -15..=15 {
            let row = row as f32 * 0.02;
            let band: Vec<i32> = sides.clone().map(|i| at(0.015 * i as f32, row)).collect();
            steps.extend(band.windows(2).map(|pair| (pair[1] - pair[0]).abs()));
        }
        steps.iter().sum::<i32>() as f32 / steps.len() as f32
    };
    (noise(20..40), noise(-6..5))
}

#[test]
fn a_bumpy_normal_maps_pores_dont_grain_skins_diffuse_light() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    // The ball's own shading changes from pixel to pixel too: a smooth
    // ball's is the floor. Skin must take off most of what the bumps add.
    let (smooth_lit, smooth_edge) = grain(Shading::Standard, false);
    let (standard_lit, standard_edge) = grain(Shading::Standard, true);
    let (skin_lit, skin_edge) = grain(skin(None, 0.0), true);
    eprintln!(
        "grain lit: smooth {smooth_lit}, standard {standard_lit}, skin {skin_lit}; where the light ends: smooth {smooth_edge}, standard {standard_edge}, skin {skin_edge}"
    );
    let left = |skin: f32, standard: f32, smooth: f32| (skin - smooth) / (standard - smooth);
    assert!(
        left(skin_lit, standard_lit, smooth_lit) < 0.3,
        "lit: smooth {smooth_lit}, standard {standard_lit}, skin {skin_lit}"
    );
    assert!(
        left(skin_edge, standard_edge, smooth_edge) < 0.3,
        "where the light ends: smooth {smooth_edge}, standard {standard_edge}, skin {skin_edge}"
    );
}

/// A square facing the camera with normal map `own`, lit from the right.
fn square(own: Arc<dyn TextureSource>, shading: Shading) -> Vec<u8> {
    square_under(own, shading, Vec3::new(1.0, 0.0, 0.6))
}

/// A square facing the camera (+Z) with normal map `own`, under a sun
/// toward `sun`.
fn square_under(own: Arc<dyn TextureSource>, shading: Shading, sun: Vec3) -> Vec<u8> {
    let Some((device, queue)) = test_device() else {
        return Vec::new();
    };
    let mut square = item(Shape::Block, Vec3::ZERO, 1.0);
    square.transform = Mat4::from_scale(Vec3::new(4.0, 4.0, 0.1));
    square.material.base_color = [0.8; 3];
    square.material.maps.normal = normal(own);
    square.material.shading = shading;
    let mut list = scene(vec![square], 5.0, 40.0);
    list.sun.direction = sun.normalize();
    let mut renderer = renderer(&device, &queue);
    draw(&mut renderer, &list)
}

#[test]
fn the_normal_blend_draws_the_own_map_at_0_and_the_defined_one_at_1() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let toward = || leaning(0x5e_0a, 30.0);
    let away = || leaning(0x5e_0b, -30.0);
    let own = square(toward(), skin(None, 0.0));
    let defined = square(away(), skin(None, 0.0));
    let middle = (SIZE as usize / 2 * SIZE as usize + SIZE as usize / 2) * 4;
    assert!(
        own[middle] > defined[middle] + 20,
        "the maps light differently: {} and {}",
        own[middle],
        defined[middle]
    );
    assert!(
        square(toward(), skin(Some(away()), 0.0)) == own,
        "blend 0 draws the own map"
    );
    assert!(
        square(toward(), skin(Some(away()), 1.0)) == defined,
        "blend 1 draws the defined map"
    );
    let half = square(toward(), skin(Some(away()), 0.5));
    assert!(
        half[middle] < own[middle] && half[middle] > defined[middle],
        "half way: {} between {} and {}",
        half[middle],
        own[middle],
        defined[middle]
    );
}

/// A normal map 64 texels square, flat but for a fold: a band 12 texels
/// wide (columns 26 to 37) whose texels lean 50 degrees toward -X, away from
/// a light on the right (an ear's fold, a nostril's rim, the lid's crease
/// as the sculpt's map carries them).
fn folded(key: u64) -> Arc<dyn TextureSource> {
    let (sin, cos) = (-50.0f32).to_radians().sin_cos();
    let byte = |v: f32| ((v * 0.5 + 0.5) * 255.0).round() as u8;
    Map::shared(key, 64, move |x, _| {
        if (26..38).contains(&x) {
            [byte(sin), byte(0.0), byte(cos), 255]
        } else {
            [128, 128, 255, 255]
        }
    })
}

#[test]
fn red_light_doesnt_fill_a_normal_maps_fold_that_green_leaves_dark() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    // Red reads the normal map blurrier than green (it spreads further
    // under the skin), but only a little further: in a fold the sculpt's
    // map turns away from the light, a blurrier red normal faces it again,
    // and drawn so the fold filled with saturated red (an ear's helix, a
    // nostril's corner, the crease under the eye).
    let pixels = square(folded(0x5e_f0), skin(None, 0.0));
    let flat = square(leaning(0x5e_f1, 0.0), skin(None, 0.0));
    let row = SIZE as usize / 2;
    let at = |pixels: &[u8], x: usize| -> [u8; 4] {
        pixels[(row * SIZE as usize + x) * 4..][..4]
            .try_into()
            .unwrap()
    };
    let ratio = |p: [u8; 4]| p[0] as f32 / p[1].max(1) as f32;
    let middle = at(&flat, SIZE as usize / 2);
    // The most red over green across the row, where there is any red.
    let reddest = (0..SIZE as usize)
        .map(|x| at(&pixels, x))
        .filter(|p| p[0] >= 8)
        .map(ratio)
        .fold(0.0f32, f32::max);
    let dark = (0..SIZE as usize)
        .map(|x| at(&pixels, x))
        .filter(|p| p[1] < middle[1] / 2)
        .count();
    eprintln!(
        "flat red over green {}; the folded square's reddest {reddest}, {dark} pixels darker than half the flat's",
        ratio(middle)
    );
    assert!(dark >= 4, "the fold is drawn dark: {dark} pixels");
    assert!(
        ratio(middle) < 1.04,
        "flat skin under a sun is grey: {middle:?}"
    );
    // Measured 82 before red was held, 1.375 after: red at most half as
    // bright again as green's normal gives it (`SKIN_RED_FILL`), which near
    // black stays about 1.5 in 8-bit sRGB too.
    assert!(
        reddest < 1.5,
        "red over green in the fold: {reddest} (flat {})",
        ratio(middle)
    );
}

/// The mean brightness (0 to 765) of the middle half of a frame.
fn middle_brightness(pixels: &[u8]) -> f32 {
    let quarter = SIZE / 4;
    let (mut sum, mut count) = (0.0, 0.0);
    for y in quarter..SIZE - quarter {
        for x in quarter..SIZE - quarter {
            let at = ((y * SIZE + x) * 4) as usize;
            sum += brightness(pixels[at..][..4].try_into().unwrap()) as f32;
            count += 1.0;
        }
    }
    sum / count
}

#[test]
fn a_normal_maps_slope_toward_a_light_behind_the_surface_stays_dark_on_skin() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    // The sun is just behind the square's plane (its cosine to the
    // triangles' normal is about -0.15), and the normal map leans 50
    // degrees toward it: a navel's inner wall, past where the belly's light
    // ends. The surface itself faces away, so no light reaches the wall;
    // Standard draws it dark (`bump_shadowing`). Skin drew it lit through
    // the rim: an orange spot in the body's shadowed half.
    let behind = Vec3::new(1.0, 0.0, -0.15);
    let toward = || leaning(0x5e_b1, 50.0);
    let flat = || leaning(0x5e_b2, 0.0);
    let standard = middle_brightness(&square_under(toward(), Shading::Standard, behind));
    let plain = middle_brightness(&square_under(flat(), skin(None, 0.0), behind));
    let leaned = middle_brightness(&square_under(toward(), skin(None, 0.0), behind));
    // With the sun in front of the plane (cosine about 0.29) the same
    // slope does light skin more than the plain square: the map leans
    // toward the light, and relief on the lit side survives.
    let in_front = Vec3::new(1.0, 0.0, 0.3);
    let lit_flat = middle_brightness(&square_under(flat(), skin(None, 0.0), in_front));
    let lit_leaned = middle_brightness(&square_under(toward(), skin(None, 0.0), in_front));
    eprintln!(
        "behind the plane: standard {standard}, skin flat {plain}, skin leaned {leaned}; \
         in front: skin flat {lit_flat}, leaned {lit_leaned}"
    );
    assert!(
        lit_leaned > lit_flat + 20.0,
        "the slope faces the light: {lit_leaned} vs {lit_flat}"
    );
    assert!(standard < 3.0, "Standard draws the slope dark: {standard}");
    assert!(
        leaned < plain + 3.0,
        "skin draws the slope as dark as the plain square: {leaned} vs {plain}"
    );
}

/// The mean color of each of five balls in a row: Standard, Standard with
/// a bumpy normal map, Toon, Unlit and a translucent leaf-like card, under
/// a sun from the front right, in the dark.
fn probe() -> Vec<[f32; 3]> {
    let Some((device, queue)) = test_device() else {
        return Vec::new();
    };
    let mut bumpy_ball = ball(1.0, -2.5, Shading::Standard);
    bumpy_ball.material.maps.normal = normal(bumpy(0x5e_b1));
    bumpy_ball.material.uv_scale = [4.0, 4.0];
    let mut card = item(Shape::Block, Vec3::ZERO, 1.0);
    card.transform = Mat4::from_scale_rotation_translation(
        Vec3::new(1.8, 1.8, 0.05),
        glam::Quat::IDENTITY,
        Vec3::new(5.0, 0.0, 0.0),
    );
    card.material.base_color = [0.3, 0.6, 0.2];
    card.material.double_sided = true;
    card.material.translucency = 0.5;
    let items = vec![
        ball(1.0, -5.0, Shading::Standard),
        bumpy_ball,
        ball(1.0, 0.0, Shading::Toon(Toon::default())),
        ball(1.0, 2.5, Shading::Unlit),
        card,
    ];
    let mut list = scene(items, 14.0, 40.0);
    list.sun.direction = Vec3::new(0.6, 0.4, 0.7).normalize();
    let mut renderer = renderer(&device, &queue);
    let pixels = draw(&mut renderer, &list);
    [-5.0, -2.5, 0.0, 2.5, 5.0]
        .iter()
        .map(|&x| {
            let middle = on_screen(&list, Vec3::new(x, 0.0, 0.0));
            let mut sum = [0.0; 3];
            let mut count = 0.0;
            for dy in -6..=6 {
                for dx in -6..=6 {
                    let p = pixel(&pixels, middle + Vec2::new(dx as f32, dy as f32));
                    for c in 0..3 {
                        sum[c] += f32::from(p[c]);
                    }
                    count += 1.0;
                }
            }
            sum.map(|s| s / count)
        })
        .collect()
}

#[test]
fn every_other_material_draws_as_it_did_before_the_skin_model() {
    if test_device().is_none() {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    let drawn = probe();
    eprintln!("probe: {drawn:?}");
    // Measured on the commit before the Skin model (r5-skin-shading's
    // base), on an RX 9070 XT. Within 1.5 of 255 for other GPUs' rounding.
    let before: [[f32; 3]; 5] = PROBE_BEFORE;
    for (i, (now, was)) in drawn.iter().zip(before).enumerate() {
        for c in 0..3 {
            assert!(
                (now[c] - was[c]).abs() <= 1.5,
                "item {i} channel {c}: {now:?}, before {was:?}"
            );
        }
    }
}

/// What [`probe`] read before the Skin model (see the test).
const PROBE_BEFORE: [[f32; 3]; 5] = [
    [152.497, 152.497, 152.497],
    [139.763, 139.763, 139.763],
    [150.704, 150.704, 150.704],
    [204.0, 204.0, 204.0],
    [80.142, 100.923, 73.941],
];

/// Human skin's diffusion profile: d'Eon and Luebke's sum of six Gaussians
/// (GPU Gems 3, chapter 14), each a variance in square millimetres and its
/// red, green and blue weights.
const PROFILE: [(f64, [f64; 3]); 6] = [
    (0.0064, [0.233, 0.455, 0.649]),
    (0.0484, [0.100, 0.336, 0.344]),
    (0.187, [0.118, 0.198, 0.0]),
    (0.567, [0.113, 0.007, 0.007]),
    (1.99, [0.358, 0.004, 0.0]),
    (7.41, [0.078, 0.0, 0.0]),
];

/// How much light comes out at `distance` mm from where it went in.
fn profile(distance: f64) -> [f64; 3] {
    let mut out = [0.0; 3];
    for (variance, weights) in PROFILE {
        let gaussian = (-distance * distance / (2.0 * variance)).exp()
            / (2.0 * std::f64::consts::PI * variance);
        for c in 0..3 {
            out[c] += weights[c] * gaussian;
        }
    }
    out
}

/// Penner's pre-integration: the light at a point of a ring of `radius`
/// mm lit at cosine `n_dot_l`, gathered from round the ring through the
/// profile, as a share of what a flat surface lit head-on gets.
fn pre_integrated(n_dot_l: f64, radius: f64) -> [f64; 3] {
    use std::f64::consts::PI;
    let theta = n_dot_l.clamp(-1.0, 1.0).acos();
    // The profile is nothing past 30 mm.
    let reach = (30.0 / radius).min(PI);
    let steps = 40_000;
    let (mut lit, mut total) = ([0.0; 3], [0.0; 3]);
    for i in 0..=steps {
        let x = -reach + 2.0 * reach * f64::from(i) / f64::from(steps);
        let weight = profile(2.0 * radius * (x / 2.0).sin().abs());
        let light = (theta + x).cos().max(0.0);
        for c in 0..3 {
            lit[c] += light * weight[c];
            total[c] += weight[c];
        }
    }
    std::array::from_fn(|c| lit[c] / total[c])
}

/// The table skin.wgsl holds: 7 rows (2 to 128 mm) of 33 columns.
fn skin_table() -> Vec<[f64; 3]> {
    (0..7)
        .flat_map(|row| {
            let radius = 2.0 * 2f64.powi(row);
            (0..33).map(move |column| pre_integrated(-1.0 + f64::from(column) / 16.0, radius))
        })
        .collect()
}

#[test]
fn the_skin_table_is_skins_profile_pre_integrated() {
    let source = include_str!("../../skin.wgsl");
    let start = source.rfind("array<vec3<f32>, 231>(").expect("the table");
    let body = &source[start..source[start..].find(");").unwrap() + start];
    let written: Vec<[f64; 3]> = body
        .split("vec3<f32>(")
        .skip(1)
        .map(|entry| {
            let numbers: Vec<f64> = entry
                .split(')')
                .next()
                .unwrap()
                .split(',')
                .map(|n| n.trim().parse().unwrap())
                .collect();
            [numbers[0], numbers[1], numbers[2]]
        })
        .collect();
    let table = skin_table();
    let right = written.len() == table.len()
        && written
            .iter()
            .zip(&table)
            .all(|(a, b)| (0..3).all(|c| (a[c] - b[c]).abs() < 6e-4));
    if !right {
        let mut text = String::new();
        for row in 0..7 {
            text += &format!("    // {} mm\n", 2 << row);
            for chunk in table[row * 33..row * 33 + 33].chunks(3) {
                text += "   ";
                for [r, g, b] in chunk {
                    text += &format!(" vec3<f32>({r:.4}, {g:.4}, {b:.4}),");
                }
                text += "\n";
            }
        }
        panic!("skin.wgsl's table is out of date; it should be:\n{text}");
    }
    // Lit head-on, a gentle curve keeps nearly all its light, and every
    // channel reaches past the line where light ends, red furthest.
    let last_row = &table[6 * 33..];
    assert!(last_row[32].iter().all(|&c| c > 0.99), "{:?}", last_row[32]);
    let past = &table[15];
    assert!(past[0] > past[1] && past[1] >= past[2], "{past:?}");
}
