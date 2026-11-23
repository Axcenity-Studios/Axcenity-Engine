//! Weather on surfaces: wet ones darker and glossier, snow on the tops of
//! things but not their walls, and nothing changed or added while it's
//! clear.

use super::*;
use crate::Weather;

/// The frame size [`pixel_at`] reads.
const SIZE: u32 = 64;

/// A 4-stud green block standing on a wide grey floor, under a high sun
/// beyond it, placed so the floor between the camera and the block
/// mirrors it (at (4, 0, 4)). Seen from above and to the side, so the
/// block's top and its +X wall both show.
fn block_on_floor() -> DrawList {
    let block = |size: Vec3, position: Vec3, color: [f32; 3]| DrawItem {
        transform: Mat4::from_scale_rotation_translation(size, glam::Quat::IDENTITY, position),
        material: Material {
            base_color: color,
            ..Material::default()
        },
        ..item(Shape::Block, Vec3::ZERO, 1.0)
    };
    let mut list = list_of(vec![
        block(
            Vec3::new(200.0, 1.0, 200.0),
            Vec3::new(0.0, -0.5, 0.0),
            [0.5; 3],
        ),
        block(
            Vec3::splat(4.0),
            Vec3::new(0.0, 2.0, 0.0),
            [0.15, 0.45, 0.2],
        ),
    ]);
    list.camera = Camera {
        position: Vec3::new(9.0, 9.0, 9.0),
        look_at: Vec3::new(0.0, 1.0, 0.0),
        fov_y_degrees: 70.0,
    };
    list.sun.direction = Vec3::new(-5.0, 9.0, -5.0).normalize();
    list.sun.intensity = 2.0;
    list.ambient.intensity = 0.3;
    list
}

/// Where the block's top and +X wall are drawn.
const TOP: Vec3 = Vec3::new(0.0, 4.0, 0.0);
const WALL: Vec3 = Vec3::new(2.0, 2.0, 0.5);

fn brightness(pixel: [u8; 4]) -> u32 {
    pixel[..3].iter().map(|&c| u32::from(c)).sum()
}

/// The floor's pixels (not the black sky, nor the block, which is much
/// greener than it is red), as brightness.
fn floor_pixels(pixels: &[u8]) -> Vec<u32> {
    pixels
        .chunks(4)
        .filter(|pixel| pixel[0] > 4 && u32::from(pixel[1]) < u32::from(pixel[0]) + 8)
        .map(|pixel| brightness([pixel[0], pixel[1], pixel[2], 255]))
        .collect()
}

fn median(mut values: Vec<u32>) -> u32 {
    values.sort_unstable();
    values[values.len() / 2]
}

#[test]
fn wet_surfaces_are_darker_and_glossier() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let dry_wall = brightness(pixel_at(&mut renderer, &list, WALL));
    let dry_floor = floor_pixels(&renderer.read_pixels());

    list.weather = Weather {
        wetness: 1.0,
        snow: 0.0,
        ..Weather::default()
    };
    let wet_wall = brightness(pixel_at(&mut renderer, &list, WALL));
    let wet_floor = floor_pixels(&renderer.read_pixels());
    assert!(wet_wall < dry_wall, "wall {dry_wall} dry, {wet_wall} wet");
    assert!(
        median(wet_floor.clone()) < median(dry_floor.clone()),
        "most of the floor is darker"
    );
    let peak = |pixels: &[u32]| pixels.iter().copied().max().unwrap();
    assert!(
        peak(&wet_floor) > peak(&dry_floor),
        "the sun's reflection is sharper and brighter: {} dry, {} wet",
        peak(&dry_floor),
        peak(&wet_floor)
    );
}

#[test]
fn snow_whitens_the_tops_of_things_but_not_their_walls() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let bare_top = pixel_at(&mut renderer, &list, TOP);
    let bare_wall = pixel_at(&mut renderer, &list, WALL);

    let mut tops = Vec::new();
    for cover in [0.3, 1.0] {
        list.weather = Weather {
            wetness: 0.0,
            snow: cover,
            ..Weather::default()
        };
        tops.push(pixel_at(&mut renderer, &list, TOP));
        assert_eq!(
            pixel_at(&mut renderer, &list, WALL),
            bare_wall,
            "the wall stays bare at cover {cover}"
        );
    }
    let [light, deep] = [tops[0], tops[1]];
    // Snow is white: the green top turns as red and blue as it is green.
    let whiteness = |pixel: [u8; 4]| i32::from(pixel[0]) - i32::from(pixel[1]);
    assert!(
        whiteness(bare_top) < whiteness(light) && whiteness(light) < whiteness(deep),
        "{bare_top:?} bare, {light:?} light cover, {deep:?} deep"
    );
    assert!(brightness(deep) > brightness(bare_top) + 150, "{deep:?}");
}

#[test]
fn clear_weather_draws_the_same_frame_and_weather_adds_no_cost() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    renderer.render(&list).unwrap();
    let clear = renderer.read_pixels();
    let clear_stats = renderer.stats();

    list.weather = Weather {
        wetness: 0.8,
        snow: 0.6,
        ..Weather::default()
    };
    renderer.render(&list).unwrap();
    assert!(renderer.read_pixels() != clear, "the weather shows");
    let stats = renderer.stats();
    assert_eq!(stats.draw_calls, clear_stats.draw_calls);
    assert_eq!(stats.target_bytes, clear_stats.target_bytes);

    list.weather = Weather::default();
    renderer.render(&list).unwrap();
    assert!(renderer.read_pixels() == clear, "dry again, as before");
}

#[test]
fn weather_field_shelters_roof_and_manual_interior_from_snow() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let bare_top = pixel_at(&mut renderer, &list, TOP);

    // Snow with no weather field whitens the block top.
    list.weather = Weather {
        wetness: 0.0,
        snow: 1.0,
        snow_thickness: 0.5,
    };
    let snowy_top = pixel_at(&mut renderer, &list, TOP);
    assert!(brightness(snowy_top) > brightness(bare_top) + 150);

    // A roof at y = 12 above the block top (y = 4) shelters it back to bare.
    let size = 16u32;
    let roof_cells = vec![[12.0, 0.0, 0.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: roof_cells,
    }));
    let sheltered_top = pixel_at(&mut renderer, &list, TOP);
    assert_eq!(
        sheltered_top, bare_top,
        "a roof above the block keeps its top completely bare"
    );

    // Manual exterior override (+1.0) forces snow even under the roof.
    let forced_exterior = vec![[12.0, 1.0, 0.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: forced_exterior,
    }));
    let forced_top = pixel_at(&mut renderer, &list, TOP);
    assert!(
        brightness(forced_top) > brightness(bare_top) + 150,
        "manual exterior override restores snow: {forced_top:?}"
    );

    // Manual interior override (-1.0) shelters even with open sky (-1e9).
    let forced_interior = vec![[-1.0e9, -1.0, 0.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: forced_interior,
    }));
    let interior_top = pixel_at(&mut renderer, &list, TOP);
    assert_eq!(
        interior_top, bare_top,
        "manual interior paint keeps the surface bare"
    );
}

#[test]
fn snow_footprints_visibly_compact_snow() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    list.weather = Weather {
        wetness: 0.0,
        snow: 1.0,
        snow_thickness: 0.6,
    };
    let size = 16u32;
    let open = vec![[-1.0e9, 0.0, 0.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: open,
    }));
    let untouched = pixel_at(&mut renderer, &list, TOP);

    // Stamp a deep footprint across the top of the block.
    let stamped = vec![[-1.0e9, 0.0, 1.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: stamped,
    }));
    let compressed = pixel_at(&mut renderer, &list, TOP);
    assert!(
        brightness(compressed) < brightness(untouched),
        "footprints compact and darken snow: untouched {untouched:?}, compressed {compressed:?}"
    );
}
