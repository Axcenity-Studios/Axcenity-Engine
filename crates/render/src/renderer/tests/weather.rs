//! Weather on surfaces: wet ones darker and glossier, snow on the tops of
//! things but not their walls, what's pressed into the snow showing as it's
//! shaped, and nothing changed or added while it's clear.

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

/// Screen-space reflections read last frame's picture, so a frame drawn
/// right after a change isn't yet settled: the tests here that compare
/// single frames draw without them (tests/screen_reflections.rs covers
/// them).
fn without_screen_reflections(renderer: &mut TextureRenderer) {
    renderer.set_graphics(&crate::GraphicsSettings {
        screen_space_reflections: Some(false),
        ..crate::GraphicsSettings::default()
    });
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
    without_screen_reflections(&mut renderer);
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
    without_screen_reflections(&mut renderer);
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
        ..Weather::default()
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
        prints: None,
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
        prints: None,
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
        prints: None,
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
        ..Weather::default()
    };
    let size = 16u32;
    let open = vec![[-1.0e9, 0.0, 0.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: open,
        prints: None,
    }));
    let untouched = pixel_at(&mut renderer, &list, TOP);

    // Stamp a deep footprint across the top of the block.
    let stamped = vec![[-1.0e9, 0.0, 1.0, 0.0]; (size * size) as usize];
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: stamped,
        prints: None,
    }));
    let compressed = pixel_at(&mut renderer, &list, TOP);
    assert!(
        brightness(compressed) < brightness(untouched),
        "footprints compact and darken snow: untouched {untouched:?}, compressed {compressed:?}"
    );
}

#[test]
fn falling_rain_rings_the_water_and_wet_surfaces_are_still_without_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    without_screen_reflections(&mut renderer);
    let mut list = block_on_floor();
    let mut frame_at = |list: &mut DrawList, time: f32| {
        list.time = time;
        renderer.render(list).unwrap();
        renderer.read_pixels()
    };
    // Wet after the rain: nothing on it moves.
    list.weather = Weather {
        wetness: 1.0,
        ..Weather::default()
    };
    let after = frame_at(&mut list, 0.0);
    assert!(after == frame_at(&mut list, 0.4), "standing water is still");
    // While it rains, drops ring the water, and the rings spread.
    list.weather.rain = 1.0;
    let ringed = frame_at(&mut list, 0.4);
    assert!(ringed != after, "falling rain rings the water");
    assert!(
        ringed != frame_at(&mut list, 0.55),
        "the rings spread as time passes"
    );
    // Rain that isn't wetting anything (sheltered, or dried at once) rings nothing.
    list.weather.wetness = 0.0;
    let dry = frame_at(&mut list, 0.4);
    assert!(dry == frame_at(&mut list, 0.55), "no water, no rings");
}

#[test]
fn snow_lies_on_a_plants_crown_though_its_leaves_face_sideways() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    // The block as a bush: its +X side near the top, which as a wall
    // stays bare (`WALL`, lower down, does either way).
    let high = Vec3::new(2.0, 3.7, 0.5);
    let snowy = Weather {
        snow: 1.0,
        ..Weather::default()
    };
    let mut whitened = Vec::new();
    for translucency in [0.0, 0.4] {
        let mut list = block_on_floor();
        list.items[1].material.translucency = translucency;
        let bare = pixel_at(&mut renderer, &list, high);
        list.weather = snowy;
        let covered = pixel_at(&mut renderer, &list, high);
        whitened.push(brightness(covered) as i32 - brightness(bare) as i32);
    }
    let [wall, crown] = [whitened[0], whitened[1]];
    // (But for the shade of the slab of snow now standing on its top.)
    assert!(
        wall.abs() <= 6,
        "a wall stays bare right up to its top: {wall}"
    );
    assert!(
        crown > 60,
        "the top of a plant's crown is snowed on: {crown} brighter"
    );
}

/// A frame of `list` from a camera level with the block's top, looking at
/// it from the side: the pixel just above the top's far edge, and the
/// pixel a little higher, clear of any snow.
fn above_the_block(renderer: &mut TextureRenderer, list: &DrawList) -> ([u8; 4], [u8; 4]) {
    let mut list = list.clone();
    list.camera = Camera {
        position: Vec3::new(14.0, 4.3, 0.0),
        look_at: Vec3::new(0.0, 4.3, 0.0),
        fov_y_degrees: 40.0,
    };
    // (The whitest of a few heights up the slab's side: its foot is ragged,
    // the thing's own side showing in places, and its depth differs from
    // place to place, from 0.6 to 1.15 of what lies on things.)
    let slab = [4.2, 4.3, 4.4, 4.5, 4.58]
        .map(|height| pixel_at(renderer, &list, Vec3::new(2.0, height, 0.0)))
        .into_iter()
        .max_by_key(|pixel| pixel[..3].iter().copied().min())
        .unwrap();
    let clear = pixel_at(renderer, &list, Vec3::new(2.0, 5.6, 0.0));
    (slab, clear)
}

#[test]
fn thick_snow_stands_on_a_block_as_a_slab_with_sides() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    // Seen from the side, level with its top: above the top is sky.
    let (bare, sky) = above_the_block(&mut renderer, &list);
    assert_eq!(bare, sky, "nothing stands on the bare block");

    // A stud of snow: 0.7 of it lies on things. The +X wall's top edge is
    // at y 4; 0.3 above it is the side of the slab, white, and not the sky
    // seen through a slit between the lifted top and the wall.
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 1.0,
        ..Weather::default()
    };
    let (slab, clear) = above_the_block(&mut renderer, &list);
    assert_eq!(clear, sky, "the slab is no deeper than the snow");
    // (It faces away from the sun, so it's snow in the shade: pale and
    // without the block's green.)
    assert!(
        slab[..3].iter().all(|&channel| channel > 90)
            && slab[1].abs_diff(slab[0]) < 12
            && slab[1].abs_diff(slab[2]) < 12,
        "the side of the slab is snow: {slab:?} (sky {sky:?})"
    );

    // Snow with no thickness is the layer of white it was: nothing rises.
    list.weather.snow_thickness = 0.0001;
    let (thin, _) = above_the_block(&mut renderer, &list);
    assert_eq!(thin, sky, "a dusting has no sides");

    // What's see-through, cut out or swaying isn't lifted.
    list.weather.snow_thickness = 1.0;
    list.items[1].sway = Some(crate::Sway {
        base: 0.0,
        height: 4.0,
        strength: 0.5,
        flutter: 0.0,
    });
    let (swaying, _) = above_the_block(&mut renderer, &list);
    assert_eq!(swaying, sky, "a plant keeps its shape");
}

#[test]
fn snow_lifts_nothing_under_shelter_and_draws_no_skirt_in_clear_weather() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let (_, sky) = above_the_block(&mut renderer, &list);
    renderer.render(&list).unwrap();
    let clear_triangles = renderer.stats().triangles;
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 1.0,
        ..Weather::default()
    };
    // The skirts are drawn, but aren't counted as the meshes' triangles.
    renderer.render(&list).unwrap();
    assert_eq!(renderer.stats().triangles, clear_triangles);

    // A roof over everything at y 30: the block's top is under it.
    let size = 8;
    list.weather_field = Some(std::sync::Arc::new(crate::WeatherField {
        origin: [-40.0, -40.0],
        cell_size: 10.0,
        size,
        cells: vec![[30.0, 0.0, 0.0, 0.0]; (size * size) as usize],
        prints: None,
    }));
    let (sheltered, _) = above_the_block(&mut renderer, &list);
    assert_eq!(sheltered, sky, "no snow, no slab under a roof");
}

/// A weather box `half_size` around `middle`, with these shares of the
/// world's wetness and snow inside it.
fn weather_box(middle: Vec3, half_size: Vec3, wetness: f32, snow: f32) -> crate::WeatherBox {
    crate::WeatherBox {
        to_box: Mat4::from_translation(middle).inverse(),
        half_size,
        blend: 1.0,
        wetness,
        snow,
    }
}

/// A box around the block and the air over it, clear of the floor's part
/// that [`OPEN_FLOOR`] is on.
fn box_over_the_block(wetness: f32, snow: f32) -> crate::WeatherBox {
    weather_box(
        Vec3::new(0.0, 4.0, 0.0),
        Vec3::new(3.5, 3.0, 3.5),
        wetness,
        snow,
    )
}

/// A place on the floor in the sun, out of every box the tests make. (The
/// floor is one wide block, and snow lifts a block by its corners, which
/// are far out in the open: don't look for shelter on it.)
const OPEN_FLOOR: Vec3 = Vec3::new(-2.0, 0.0, 6.0);

/// A roof at y = 12 over everything, as the sky-visibility field has it.
fn roof_over_everything() -> std::sync::Arc<crate::WeatherField> {
    let size = 16u32;
    std::sync::Arc::new(crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells: vec![[12.0, 0.0, 0.0, 0.0]; (size * size) as usize],
        prints: None,
    })
}

#[test]
fn a_weather_box_keeps_the_snow_off_what_is_in_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let bare_top = pixel_at(&mut renderer, &list, TOP);
    let bare_floor = pixel_at(&mut renderer, &list, OPEN_FLOOR);

    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 0.5,
        ..Weather::default()
    };
    let snowy_top = pixel_at(&mut renderer, &list, TOP);
    assert!(brightness(snowy_top) > brightness(bare_top) + 150);

    // Under open sky, a box with no snow in it: the block's top is bare
    // again, and the floor outside it is as white as it was.
    list.weather_boxes = vec![box_over_the_block(1.0, 0.0)];
    assert_eq!(
        pixel_at(&mut renderer, &list, TOP),
        bare_top,
        "no snow lies in the box"
    );
    let floor = pixel_at(&mut renderer, &list, OPEN_FLOOR);
    assert!(
        brightness(floor) > brightness(bare_floor) + 100,
        "the floor outside the box is snowed on: {bare_floor:?} bare, {floor:?} now"
    );
}

#[test]
fn a_weather_box_puts_snow_back_under_a_roof() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let bare_top = pixel_at(&mut renderer, &list, TOP);

    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 0.5,
        ..Weather::default()
    };
    list.weather_field = Some(roof_over_everything());
    assert_eq!(
        pixel_at(&mut renderer, &list, TOP),
        bare_top,
        "the roof keeps the block's top bare"
    );

    // An open-sided shed: the snow blows in under its roof.
    list.weather_boxes = vec![box_over_the_block(1.0, 1.0)];
    let top = pixel_at(&mut renderer, &list, TOP);
    assert!(
        brightness(top) > brightness(bare_top) + 150,
        "the box's snow lies under the roof: {bare_top:?} bare, {top:?} in the box"
    );
    // A box somewhere else changes nothing here.
    list.weather_boxes = vec![weather_box(
        Vec3::new(40.0, 4.0, 0.0),
        Vec3::splat(3.0),
        1.0,
        1.0,
    )];
    assert_eq!(
        pixel_at(&mut renderer, &list, TOP),
        bare_top,
        "outside a box the roof still shelters"
    );
}

#[test]
fn a_weather_box_keeps_what_is_in_it_dry() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    // Two places on the floor in the sun, either side of the block.
    let porch = Vec3::new(6.0, 0.0, -2.0);
    let dry_porch = pixel_at(&mut renderer, &list, porch);
    let dry_open = pixel_at(&mut renderer, &list, OPEN_FLOOR);

    list.weather = Weather {
        wetness: 1.0,
        ..Weather::default()
    };
    let wet_porch = pixel_at(&mut renderer, &list, porch);
    let wet_open = pixel_at(&mut renderer, &list, OPEN_FLOOR);
    assert!(
        brightness(wet_porch) < brightness(dry_porch)
            && brightness(wet_open) < brightness(dry_open),
        "wet, both are darker: {dry_porch:?} to {wet_porch:?}, {dry_open:?} to {wet_open:?}"
    );

    // A dry box over one of them.
    list.weather_boxes = vec![weather_box(porch, Vec3::new(2.0, 2.0, 2.0), 0.0, 0.0)];
    assert_eq!(
        pixel_at(&mut renderer, &list, porch),
        dry_porch,
        "the floor in the box is as it is dry"
    );
    assert_eq!(
        pixel_at(&mut renderer, &list, OPEN_FLOOR),
        wet_open,
        "the floor outside it is as wet as it was"
    );
}

#[test]
fn where_weather_boxes_overlap_the_later_one_wins() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let bare_top = pixel_at(&mut renderer, &list, TOP);
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 0.5,
        ..Weather::default()
    };
    let snowy_top = pixel_at(&mut renderer, &list, TOP);

    let (bare, snowy) = (box_over_the_block(0.0, 0.0), box_over_the_block(1.0, 1.0));
    list.weather_boxes = vec![bare, snowy];
    assert_eq!(
        pixel_at(&mut renderer, &list, TOP),
        snowy_top,
        "the later box's snow lies, as in the open"
    );
    list.weather_boxes = vec![snowy, bare];
    assert_eq!(
        pixel_at(&mut renderer, &list, TOP),
        bare_top,
        "the later box keeps it off"
    );
}

#[test]
fn a_weather_box_with_twice_the_snow_lifts_the_slab_twice_as_high() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    // The camera `above_the_block` uses, and the pixel a stud over the
    // block's far edge: over the slab a stud of snow leaves on it (0.7
    // deep), inside the one twice the snow does (1.4).
    let mut over_the_slab = |list: &DrawList| {
        let mut list = list.clone();
        list.camera = Camera {
            position: Vec3::new(14.0, 4.3, 0.0),
            look_at: Vec3::new(0.0, 4.3, 0.0),
            fov_y_degrees: 40.0,
        };
        pixel_at(&mut renderer, &list, Vec3::new(2.0, 5.0, 0.0))
    };
    let sky = over_the_slab(&list);
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 1.0,
        ..Weather::default()
    };
    assert_eq!(over_the_slab(&list), sky, "a stud of snow ends below it");
    list.weather_boxes = vec![box_over_the_block(1.0, 1.0)];
    assert_eq!(over_the_slab(&list), sky, "and so does a box's whole share");

    list.weather_boxes = vec![box_over_the_block(1.0, 2.0)];
    let slab = over_the_slab(&list);
    assert!(
        slab[..3].iter().all(|&channel| channel > 90)
            && slab[1].abs_diff(slab[0]) < 12
            && slab[1].abs_diff(slab[2]) < 12,
        "twice the snow stands that high: {slab:?} (sky {sky:?})"
    );
}

#[test]
fn the_frame_takes_the_nearest_weather_boxes_in_order_and_none_in_clear_weather() {
    let mut list = block_on_floor();
    list.camera.position = Vec3::new(0.0, 5.0, 0.0);
    // Ten boxes in a row along x, 100 studs apart, each with its place in
    // the row as its wetness; the camera is inside the first.
    let row = |count: usize| -> Vec<crate::WeatherBox> {
        (0..count)
            .map(|place| {
                weather_box(
                    Vec3::new(place as f32 * 100.0, 0.0, 0.0),
                    Vec3::splat(10.0),
                    place as f32 * 0.1,
                    2.0,
                )
            })
            .collect()
    };
    list.weather_boxes = row(10);
    let graphics = EffectiveGraphics::default();
    let clear = frame_uniform(&list, 1.0, &graphics);
    assert_eq!(clear.precipitation[2], 0.0, "no weather, no boxes");

    list.weather.snow = 1.0;
    let frame = frame_uniform(&list, 1.0, &graphics);
    assert_eq!(frame.precipitation[2], crate::MAX_WEATHER_BOXES as f32);
    let wetness: Vec<f32> = frame.weather_boxes.iter().map(|b| b.values[0]).collect();
    let expected: Vec<f32> = (0..8).map(|place| place as f32 * 0.1).collect();
    assert_eq!(wetness, expected, "the eight nearest, in the list's order");
    assert_eq!(frame.weather_boxes[0].extent, [10.0, 10.0, 10.0, 1.0]);
    assert_eq!(frame.weather_boxes[0].values, [0.0, 2.0, 0.0, 0.0]);

    // From the far end of the row it's the last eight, still in order.
    list.camera.position = Vec3::new(900.0, 5.0, 0.0);
    let frame = frame_uniform(&list, 1.0, &graphics);
    let wetness: Vec<f32> = frame.weather_boxes.iter().map(|b| b.values[0]).collect();
    let expected: Vec<f32> = (2..10).map(|place| place as f32 * 0.1).collect();
    assert_eq!(wetness, expected);

    // Numbers that aren't finite never reach the shader: a box that has no
    // place is left out, and a share or fade that isn't a number counts as
    // none; shares are held to 2.
    let mut boxes = row(3);
    boxes[0].to_box = Mat4::from_translation(Vec3::new(f32::NAN, 0.0, 0.0));
    boxes[1].half_size = Vec3::new(f32::INFINITY, 1.0, 1.0);
    boxes[2].wetness = f32::NAN;
    boxes[2].snow = 9.0;
    boxes[2].blend = f32::NAN;
    list.weather_boxes = boxes;
    let frame = frame_uniform(&list, 1.0, &graphics);
    assert_eq!(frame.precipitation[2], 1.0);
    assert_eq!(frame.weather_boxes[0].values, [0.0, 2.0, 0.0, 0.0]);
    assert_eq!(frame.weather_boxes[0].extent, [10.0, 10.0, 10.0, 0.0]);
    assert!(
        bytemuck::bytes_of(&frame)
            .chunks_exact(4)
            .skip(std::mem::offset_of!(FrameUniform, weather_boxes) / 4)
            .all(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()).is_finite())
    );
}

/// A field of open sky over the block (16 studs a side, from -8), whose
/// four cells round the origin say prints are near, and a whole print map
/// (from -32) with a square pressed right through the snow in it: a stud a
/// side, round the origin. `fill` is how far prints have filled in: the
/// cells are stamped that much higher, so the square is a whole depth deep
/// whatever it is.
fn field_with_a_square_print(fill: f32, key: u64) -> crate::WeatherField {
    let size = 16u32;
    let mut cells = vec![[-1.0e9, 0.0, 0.0, 0.0]; (size * size) as usize];
    for (x, z) in [(7, 7), (8, 7), (7, 8), (8, 8)] {
        cells[(z * size + x) as usize][2] = 0.25;
    }
    let side = crate::WEATHER_PRINT_SIZE as usize;
    let per = crate::WEATHER_PRINT_CELLS_PER_STUD;
    let mut print = vec![0.0; side * side];
    let pressed = ((1.0 + fill) * crate::WEATHER_PRINT_LEVELS) as u32;
    for z in 0..side {
        for x in 0..side {
            let at = |cell: usize| (cell as f32 + 0.5) / per - 32.0;
            if at(x).abs() < 0.5 && at(z).abs() < 0.5 {
                print[z * side + x] = crate::WeatherPrintCells::pack(pressed, 0);
            }
        }
    }
    crate::WeatherField {
        origin: [-8.0, -8.0],
        cell_size: 1.0,
        size,
        cells,
        prints: Some(crate::WeatherPrints {
            fill,
            cells: std::sync::Arc::new(crate::WeatherPrintCells {
                key,
                version: 1,
                origin: [-32.0, -32.0],
                cells_per_stud: per,
                size: side as u32,
                cells: print,
                row_versions: vec![1; side],
            }),
        }),
    }
}

#[test]
fn a_print_shows_as_its_own_shape_and_not_a_quarter_stud_outside_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    list.weather = Weather {
        wetness: 0.0,
        snow: 1.0,
        snow_thickness: 0.5,
        ..Weather::default()
    };
    // Looking down on the block's top from close by: a pixel is about a
    // twentieth of a stud. The snow stands 0.35 on it.
    list.camera = Camera {
        position: Vec3::new(0.0, 9.5, 0.6),
        look_at: Vec3::new(0.0, 4.35, 0.0),
        fov_y_degrees: 30.0,
    };
    let on_top = |x: f32, z: f32| Vec3::new(x, 4.35, z);
    // Inside the square, on its east and west walls, and a quarter stud
    // outside it on each side.
    let places = [
        on_top(0.0, 0.0),
        on_top(0.5, 0.0),
        on_top(-0.5, 0.0),
        on_top(0.75, 0.0),
        on_top(-0.75, 0.0),
        on_top(0.0, 0.75),
        on_top(0.0, -0.75),
    ];
    let mut look = |list: &DrawList| places.map(|place| pixel_at(&mut renderer, list, place));

    // Untrodden: the same field, with nothing pressed.
    let mut blank = field_with_a_square_print(0.0, 1);
    blank.prints = None;
    for cell in &mut blank.cells {
        cell[2] = 0.0;
    }
    list.weather_field = Some(std::sync::Arc::new(blank));
    let untrodden = look(&list);

    for fill in [0.0, 0.25] {
        list.weather_field = Some(std::sync::Arc::new(field_with_a_square_print(fill, 2)));
        let printed = look(&list);
        // Its floor is packed snow: darker, and bluer.
        let blue = |pixel: [u8; 4]| i32::from(pixel[2]) - i32::from(pixel[0]);
        assert!(
            brightness(printed[0]) + 20 < brightness(untrodden[0]),
            "floor {:?}, untrodden {:?}",
            printed[0],
            untrodden[0]
        );
        assert!(blue(printed[0]) > blue(untrodden[0]), "{:?}", printed[0]);
        // Its walls tip the snow: the sun is in the west (-X), so the east
        // wall, which faces it, is lit and the west wall is in its own shade.
        assert!(
            brightness(printed[1]) > brightness(printed[2]) + 40,
            "east wall {:?}, west wall {:?}",
            printed[1],
            printed[2]
        );
        assert!(brightness(printed[2]) + 20 < brightness(untrodden[2]));
        // A quarter of a stud outside it the snow is as it was. (The field's
        // one-stud cells say a print is near all the way out there.)
        for outside in 3..7 {
            let (was, is) = (untrodden[outside], printed[outside]);
            assert!(
                (0..3).all(|c| was[c].abs_diff(is[c]) <= 1),
                "{:?} beside the print, {:?} untrodden",
                is,
                was
            );
        }
    }

    // Filled in by a whole depth, nothing shows.
    list.weather_field = Some(std::sync::Arc::new(field_with_a_square_print(0.0, 3)));
    if let Some(field) = &mut list.weather_field
        && let Some(prints) = &mut std::sync::Arc::make_mut(field).prints
    {
        prints.fill = 1.0;
    }
    let filled = look(&list);
    assert!(brightness(filled[0]).abs_diff(brightness(untrodden[0])) <= 3);

    // Without the print map the field's cell is all there is: the old,
    // one-stud prints, which darken the snow a quarter stud outside too.
    let mut coarse = field_with_a_square_print(0.0, 4);
    coarse.prints = None;
    list.weather_field = Some(std::sync::Arc::new(coarse));
    let old = look(&list);
    assert!(brightness(old[3]) < brightness(untrodden[3]));
}

#[test]
fn the_print_map_is_uploaded_only_when_it_changes() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 0.5,
        ..Weather::default()
    };
    let side = u64::from(crate::WEATHER_PRINT_SIZE);
    let (row, whole) = (side * 4, side * side * 4);
    let field = field_with_a_square_print(0.0, 7);
    list.weather_field = Some(std::sync::Arc::new(field.clone()));
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, whole, "all of it, once");

    // The same cells again, and filled in further: nothing is written.
    renderer.render(&list).unwrap();
    let mut filling = field.clone();
    filling.prints.as_mut().unwrap().fill = 0.4;
    list.weather_field = Some(std::sync::Arc::new(filling));
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, whole);

    // Three rows change, two of them together: those are written.
    let mut pressed = field.clone();
    {
        let cells = std::sync::Arc::make_mut(&mut pressed.prints.as_mut().unwrap().cells);
        cells.version = 2;
        for changed in [100, 101, 300] {
            cells.row_versions[changed] = 2;
        }
    }
    list.weather_field = Some(std::sync::Arc::new(pressed.clone()));
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, whole + 3 * row);
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, whole + 3 * row);

    // Another map (another host's, or after a reset) is written whole, and
    // so is this one after a frame with none.
    list.weather_field = Some(std::sync::Arc::new(field_with_a_square_print(0.0, 8)));
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, 2 * whole + 3 * row);
    let mut none = field.clone();
    none.prints = None;
    list.weather_field = Some(std::sync::Arc::new(none));
    renderer.render(&list).unwrap();
    list.weather_field = Some(std::sync::Arc::new(field_with_a_square_print(0.0, 8)));
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, 3 * whole + 3 * row);

    // Clear weather writes nothing, whatever the field holds.
    list.weather = Weather::default();
    list.weather_field = Some(std::sync::Arc::new(field_with_a_square_print(0.0, 9)));
    renderer.render(&list).unwrap();
    assert_eq!(renderer.scene.weather_print_bytes, 3 * whole + 3 * row);
}

/// Run with `cargo test -p axcenity-render print_map_upload_cost -- --ignored
/// --nocapture`: the time the renderer takes to hand the print map's changed
/// rows to the GPU's queue (the copy into wgpu's staging buffer; the GPU's
/// own copy at the frame's submit isn't timed apart from the frame), when
/// all of it changed, when a print's rows did, and when nothing did.
#[test]
#[ignore = "a measurement, not a check"]
fn print_map_upload_cost() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 0.5,
        ..Weather::default()
    };
    let mut field = field_with_a_square_print(0.0, 20);
    list.weather_field = Some(std::sync::Arc::new(field.clone()));
    renderer.render(&list).unwrap();
    let frames = 200u64;
    let all = crate::WEATHER_PRINT_SIZE as usize;
    // How many rows change each frame.
    for rows in [all, 64, 16, 0, all, 64, 16, 0] {
        let mut spent = std::time::Duration::ZERO;
        let before = renderer.scene.weather_print_bytes;
        for _ in 0..frames {
            let cells = std::sync::Arc::make_mut(&mut field.prints.as_mut().unwrap().cells);
            if rows > 0 {
                cells.version += 1;
                let version = cells.version;
                cells.row_versions[..rows].fill(version);
            }
            let started = std::time::Instant::now();
            renderer
                .scene
                .upload_weather_prints(cells, field.cells.len());
            spent += started.elapsed();
            // The frame submits what was written (and finds nothing more
            // to write: it's given the same cells).
            list.weather_field = Some(std::sync::Arc::new(field.clone()));
            renderer.render(&list).unwrap();
        }
        println!(
            "{rows} rows a frame: {:.1} us a frame, {:.1} KiB written a frame",
            spent.as_secs_f64() * 1e6 / frames as f64,
            (renderer.scene.weather_print_bytes - before) as f64 / frames as f64 / 1024.0
        );
    }
}

#[test]
fn lightning_flashes_light_the_open_and_throw_the_suns_shadows() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    // The floor in the open, and in the block's shadow (the sun is behind
    // the block, so its shadow falls toward the camera).
    let open = Vec3::new(7.0, 0.0, -1.0);
    let shaded = Vec3::new(3.0, 0.0, 3.0);
    let look = |renderer: &mut TextureRenderer, list: &DrawList| {
        (
            brightness(pixel_at(renderer, list, open)),
            brightness(pixel_at(renderer, list, shaded)),
        )
    };
    let (calm_open, calm_shaded) = look(&mut renderer, &list);
    assert!(calm_shaded < calm_open, "the block shades the floor");

    // None by default, whatever the time.
    for time in [0.0, 1.3, 7.7] {
        list.time = time;
        assert_eq!(look(&mut renderer, &list), (calm_open, calm_shaded));
    }

    // A flash in every beat: over ten seconds some frames are lit by one
    // and some fall between two.
    list.weather = Weather {
        lightning: 1.0,
        ..Weather::default()
    };
    let (mut brightest, mut its_shadow, mut dark_frames) = (0, 0, 0);
    for step in 0..200 {
        list.time = step as f32 * 0.05;
        let (lit, shadow) = look(&mut renderer, &list);
        assert!(lit + 2 >= calm_open, "a flash only adds light");
        if lit <= calm_open + 2 {
            dark_frames += 1;
        }
        if lit > brightest {
            (brightest, its_shadow) = (lit, shadow);
        }
    }
    assert!(
        brightest > calm_open + 40,
        "a flash lights the open floor: {calm_open} calm, {brightest} at its brightest"
    );
    assert!(
        dark_frames > 40,
        "{dark_frames} of 200 frames between flashes"
    );
    // The stroke's light is shadowed as the sun's is: the shaded floor
    // gains only the cloud's share of it.
    let (gain, shadow_gain) = (
        brightest - calm_open,
        its_shadow.saturating_sub(calm_shaded),
    );
    assert!(
        shadow_gain * 3 < gain * 2,
        "the open floor gained {gain}, the block's shadow {shadow_gain}"
    );
}

#[test]
fn wind_drives_snow_onto_the_wall_that_faces_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = block_on_floor();
    let bare = pixel_at(&mut renderer, &list, WALL);
    list.weather = Weather {
        snow: 1.0,
        ..Weather::default()
    };
    let whiteness = |pixel: [u8; 4]| i32::from(pixel[0]) - i32::from(pixel[1]);
    // In still air the +X wall is bare (as
    // `snow_whitens_the_tops_of_things_but_not_their_walls` checks); a gale
    // from +X plasters it, and one from -X, which it's in the lee of, doesn't.
    list.wind = Vec3::new(-25.0, 0.0, 0.0);
    let windward = pixel_at(&mut renderer, &list, WALL);
    assert!(
        whiteness(windward) > whiteness(bare) + 12,
        "{bare:?} bare, {windward:?} facing a gale"
    );
    list.wind = Vec3::new(25.0, 0.0, 0.0);
    assert_eq!(
        pixel_at(&mut renderer, &list, WALL),
        bare,
        "the lee wall stays bare"
    );
}

/// Run with `cargo test -p axcenity-render --release weather_looks_cost --
/// --ignored --nocapture`: the GPU's time for a 1920 × 1080 frame of blocks on
/// a floor under each of the weather's looks, against the same frame in clear
/// weather: an overcast (the sky's light greyed), a storm, a storm with
/// lightning (the mean and the slowest frame, since a flash lights a few
/// frames in a hundred), snow in still air and snow in a gale.
#[test]
#[ignore = "a measurement, not a check"]
fn weather_looks_cost() {
    let instance = crate::adapters::instance();
    let Ok(adapter) = pollster::block_on(crate::adapters::adapter(&instance, None)) else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    if !adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        eprintln!("skipped: the adapter has no timestamps");
        return;
    }
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::TIMESTAMP_QUERY,
        ..Default::default()
    }))
    .unwrap();
    let mut items = vec![DrawItem {
        transform: Mat4::from_scale_rotation_translation(
            Vec3::new(400.0, 1.0, 400.0),
            glam::Quat::IDENTITY,
            Vec3::new(0.0, -0.5, 0.0),
        ),
        ..item(Shape::Block, Vec3::ZERO, 1.0)
    }];
    for x in -16..16_i32 {
        for z in 0..32 {
            let height = 1.0 + (x * 7 + z * 3).rem_euclid(5) as f32;
            items.push(DrawItem {
                transform: Mat4::from_scale_rotation_translation(
                    Vec3::new(2.0, height, 2.0),
                    glam::Quat::IDENTITY,
                    Vec3::new(x as f32 * 3.0, height * 0.5, -z as f32 * 3.0),
                ),
                ..item(Shape::Block, Vec3::ZERO, 1.0)
            });
        }
    }
    let mut list = list_of(items);
    list.camera = Camera {
        position: Vec3::new(0.0, 7.0, 14.0),
        look_at: Vec3::new(0.0, 2.0, -30.0),
        fov_y_degrees: 60.0,
    };
    list.sky = crate::Sky::default();
    list.shadows.enabled = true;
    list.post.enabled = true;
    list.sun.direction = Vec3::new(-5.0, 9.0, 4.0).normalize();
    list.ambient.intensity = 0.3;

    let overcast = crate::Clouds {
        cover: 0.9,
        density: 0.7,
        ..crate::Clouds::default()
    };
    let storm = Weather {
        wetness: 1.0,
        rain: 1.0,
        ..Weather::default()
    };
    let snow = Weather {
        snow: 1.0,
        snow_thickness: 0.5,
        snowfall: 0.6,
        ..Weather::default()
    };
    let gale = Vec3::new(20.0, 0.0, -8.0);
    let looks: [(&str, crate::Clouds, Weather, Vec3); 6] = [
        (
            "clear",
            crate::Clouds::default(),
            Weather::default(),
            Vec3::ZERO,
        ),
        ("overcast", overcast, Weather::default(), Vec3::ZERO),
        ("storm", overcast, storm, gale),
        (
            "storm with lightning",
            overcast,
            Weather {
                lightning: 1.0,
                ..storm
            },
            gale,
        ),
        ("snow, still air", overcast, snow, Vec3::ZERO),
        ("snow in a gale", overcast, snow, gale),
    ];
    println!("{}", adapter.get_info().name);
    println!("look                    median ms   mean ms   slowest ms");
    for (name, clouds, weather, wind) in looks {
        let mut renderer = TextureRenderer::new(&device, &queue, 1920, 1080);
        list.clouds = clouds;
        list.weather = weather;
        list.wind = wind;
        let mut frames = Vec::new();
        for frame in 0..330 {
            list.time = frame as f32 / 30.0;
            renderer.render(&list).unwrap();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            if let (true, Some(gpu)) = (frame >= 30, renderer.stats().gpu_time) {
                frames.push(gpu.as_secs_f64() * 1000.0);
            }
        }
        frames.sort_by(f64::total_cmp);
        let mean = frames.iter().sum::<f64>() / frames.len().max(1) as f64;
        println!(
            "{name:<22}  {:>9.3}  {:>8.3}  {:>11.3}",
            frames[frames.len() / 2],
            mean,
            frames[frames.len() * 99 / 100],
        );
    }
}

/// A unit cube with each face's own corners and normals (a caller's mesh,
/// faceted, so every edge needs a skirt under snow).
struct FacetedCube;

impl FacetedCube {
    fn corners() -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        for axis in 0..3 {
            for side in [-1.0f32, 1.0] {
                let mut normal = [0.0; 3];
                normal[axis] = side;
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                let first = positions.len() as u32;
                for (a, b) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
                    let mut p = [0.0; 3];
                    p[axis] = 0.5 * side;
                    p[u] = a;
                    p[v] = b;
                    positions.push(p);
                    normals.push(normal);
                }
                let quad = if side > 0.0 {
                    [0, 1, 2, 0, 2, 3]
                } else {
                    [0, 2, 1, 0, 3, 2]
                };
                indices.extend(quad.iter().map(|i| first + i));
            }
        }
        (positions, normals, indices)
    }
}

/// The cube's positions, normals, indices and texture coordinates.
type CubeCorners = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>, Vec<[f32; 2]>);

static FACETED_CUBE: std::sync::LazyLock<CubeCorners> = std::sync::LazyLock::new(|| {
    let (positions, normals, indices) = FacetedCube::corners();
    let uvs = vec![[0.0; 2]; positions.len()];
    (positions, normals, indices, uvs)
});

impl MeshSource for FacetedCube {
    fn key(&self) -> u64 {
        0xfac_e7ed
    }
    fn positions(&self) -> &[[f32; 3]] {
        &FACETED_CUBE.0
    }
    fn normals(&self) -> &[[f32; 3]] {
        &FACETED_CUBE.1
    }
    fn uvs(&self) -> &[[f32; 2]] {
        &FACETED_CUBE.3
    }
    fn indices(&self) -> &[u32] {
        &FACETED_CUBE.2
    }
}

#[test]
fn a_shown_renderer_adds_a_meshs_snow_skirt_a_few_frames_later() {
    // A window's renderer makes a caller's mesh's skirt on a worker (for a
    // big mesh it took tens of milliseconds of the frame that first drew
    // it), and the slab's sides come in a few frames later, as an offline
    // renderer draws them at once.
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut list = block_on_floor();
    list.items[1].shape = Shape::Mesh(Arc::new(FacetedCube));
    list.weather = Weather {
        snow: 1.0,
        snow_thickness: 1.0,
        ..Weather::default()
    };
    let mut offline = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let (slab, clear) = above_the_block(&mut offline, &list);
    assert_ne!(
        slab, clear,
        "the offline renderer draws the slab's side at once"
    );
    let mut shown = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    shown.set_one_frame_ahead(true);
    let arrived = (0..120).find(|_| above_the_block(&mut shown, &list).0 == slab);
    assert!(arrived.is_some(), "the skirt never came");
}

/// Black on the left half, white on the right (an ambient occlusion map
/// with a crack down one half), 64 texels square so its mips keep the two
/// halves apart.
struct Halves(Vec<u8>);

impl Halves {
    fn new() -> Self {
        Self(
            (0..64 * 64)
                .flat_map(|i| {
                    let shade = if i % 64 < 32 { 0 } else { 255 };
                    [shade, shade, shade, 255]
                })
                .collect(),
        )
    }
}

impl crate::TextureSource for Halves {
    fn key(&self) -> u64 {
        0x4a11_7e55
    }
    fn size(&self) -> (u32, u32) {
        (64, 64)
    }
    fn rgba(&self) -> &[u8] {
        &self.0
    }
}

/// A black, rough floor whose ambient occlusion map is dark across one
/// half, under a sky bright at the horizon and dark overhead, seen low and
/// along it, so a smooth floor mirrors the bright horizon and a rough one
/// the sky's average.
fn floor_with_a_crack(wetness: f32) -> DrawList {
    let mut material = Material {
        base_color: [0.0; 3],
        roughness: 0.85,
        ..Material::default()
    };
    material.maps.occlusion = Some(crate::ChannelMap {
        texture: Arc::new(Halves::new()),
        channel: crate::Channel::Red,
        invert: false,
    });
    material.maps.occlusion_strength = 1.0;
    // A square lying flat, its texture once across it: the crack is the
    // half toward -X.
    let floor = DrawItem {
        shape: Shape::Mesh(Arc::new(Square)),
        transform: Mat4::from_scale_rotation_translation(
            Vec3::splat(40.0),
            glam::Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
            Vec3::ZERO,
        ),
        material,
        ..item(Shape::Block, Vec3::ZERO, 1.0)
    };
    let mut list = list_of(vec![floor]);
    list.sky = Sky {
        top_color: [0.05; 3],
        horizon_color: [1.0; 3],
        ground_color: [0.05; 3],
        environment_intensity: 1.0,
        image: None,
        ..Sky::default()
    };
    list.camera = Camera {
        position: Vec3::new(0.0, 5.0, 12.0),
        look_at: Vec3::new(0.0, 0.0, 0.0),
        fov_y_degrees: 70.0,
    };
    list.sun.intensity = 0.0;
    list.ambient.intensity = 0.0;
    list.weather = Weather {
        wetness,
        snow: 0.0,
        ..Weather::default()
    };
    list
}

#[test]
fn water_gathers_in_a_surfaces_cracks_first_and_they_shine_where_the_rest_has_a_sheen() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    without_screen_reflections(&mut renderer);
    let linear = |p: [u8; 4]| {
        let rgb = linear_color([p[0], p[1], p[2]].map(|c| f32::from(c) / 255.0));
        rgb.x + rgb.y + rgb.z
    };
    let (left, right) = (Vec3::new(-6.0, 0.0, 2.0), Vec3::new(6.0, 0.0, 2.0));
    let mut seen = Vec::new();
    for wetness in [0.0, 0.5] {
        let list = floor_with_a_crack(wetness);
        seen.push([
            linear(pixel_at(&mut renderer, &list, left)),
            linear(pixel_at(&mut renderer, &list, right)),
        ]);
    }
    let [dry, wet] = [seen[0], seen[1]];
    let (crack, raised) = (0, 1);
    assert!(
        dry[crack] < dry[raised] * 0.5,
        "the crack is dark dry: {dry:?}"
    );
    let gain = |side: usize| wet[side] / dry[side].max(1e-4);
    assert!(
        gain(crack) > gain(raised) * 1.5,
        "the crack's water shines more than the raised half's film: dry {dry:?}, wet {wet:?}"
    );
    assert!(
        gain(raised) > 1.0,
        "the raised half has a sheen: dry {dry:?}, wet {wet:?}"
    );
}

#[test]
fn no_wet_pixel_is_brighter_than_the_sky_it_mirrors() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    let mut list = floor_with_a_crack(1.0);
    // Looking out over it, the sky in the frame's top half.
    list.camera.look_at = Vec3::new(0.0, 5.0, 0.0);
    for _ in 0..3 {
        renderer.render(&list).unwrap();
    }
    let pixels = renderer.read_pixels();
    let rows: Vec<&[u8]> = pixels.chunks(SIZE as usize * 4).collect();
    let brightest = |rows: &[&[u8]]| {
        rows.iter()
            .flat_map(|row| row.chunks(4))
            .map(|p| brightness([p[0], p[1], p[2], 255]))
            .max()
            .unwrap()
    };
    let half = SIZE as usize / 2;
    let sky = brightest(&rows[..half]);
    let floor = brightest(&rows[half + 2..]);
    assert!(sky > 300, "the horizon is bright: {sky}");
    assert!(floor <= sky, "floor {floor}, sky {sky}");
}

#[test]
fn wet_ground_with_no_snow_cover_has_no_snow_on_it() {
    let Some((device, queue)) = test_device() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, SIZE, SIZE);
    without_screen_reflections(&mut renderer);
    // A dark floor lit evenly, so nothing but the surface's own colour
    // shows, and snow would show plainly.
    let mut list = block_on_floor();
    list.items[0].material.base_color = [0.05; 3];
    list.sun.intensity = 0.0;
    list.ambient.intensity = 1.0;
    renderer.render(&list).unwrap();
    let dry = renderer.read_pixels();
    for wetness in [0.1, 0.5] {
        list.weather = Weather {
            wetness,
            snow: 0.0,
            ..Weather::default()
        };
        renderer.render(&list).unwrap();
        let wet = renderer.read_pixels();
        // Snow's ragged edge used to lie white patches over level ground
        // whenever any weather was on (a grey floor turned near white).
        // Wet, the far floor seen at a glancing angle has a sheen of the
        // flat light, a little brighter than dry, never as white as snow.
        // The near half of the frame, seen from above.
        let near = (SIZE * SIZE * 2) as usize;
        let whiter = dry[near..]
            .chunks(4)
            .zip(wet[near..].chunks(4))
            .filter(|(d, w)| {
                brightness([w[0], w[1], w[2], 255]) > brightness([d[0], d[1], d[2], 255]) + 45
            })
            .count();
        assert_eq!(
            whiter, 0,
            "at wetness {wetness}, {whiter} pixels whiter than dry"
        );
    }
}
