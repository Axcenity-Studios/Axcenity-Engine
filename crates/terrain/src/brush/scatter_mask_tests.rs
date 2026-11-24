//! A scatter item's masks: the maths of each, and what a stroke places
//! with them.

use axcenity_core::{FoliagePlant, FoliageProp, FoliageSource, TerrainData};
use glam::{Vec2, Vec3};

use super::*;
use crate::brush::{Brush, Operation, Scatter, ScatterItem, apply};

/// A 256-stud terrain round the origin (129 samples, 2 studs apart, a
/// 100-stud height range) with these heights in studs, by world X and Z.
fn terrain(height: impl Fn(f32, f32) -> f32) -> (TerrainData, Placement) {
    let mut data = TerrainData::flat(129, 129, 0.0).unwrap();
    let all = data.bounds();
    let heights = data.heights_mut_in(all);
    for z in 0..129 {
        for x in 0..129 {
            heights[z * 129 + x] = height(x as f32 * 2.0 - 128.0, z as f32 * 2.0 - 128.0) / 100.0;
        }
    }
    let placement = Placement {
        position: Vec3::ZERO,
        size: Vec3::new(256.0, 100.0, 256.0),
    };
    (data, placement)
}

fn grass() -> FoliageSource {
    FoliageSource::Plant(FoliagePlant::Grass)
}

fn rock() -> FoliageSource {
    FoliageSource::Prop(FoliageProp::Rock)
}

/// Stamps `items` in nine circles 60 studs apart (none overlapping) round
/// the middle of the terrain, and returns where each `source` went (world
/// X and Z).
fn stroke(
    data: &mut TerrainData,
    placement: &Placement,
    items: Vec<ScatterItem>,
    spacing: f32,
) -> impl Fn(&TerrainData, &FoliageSource) -> Vec<Vec2> + use<> {
    let brush = Brush {
        radius: 30.0,
        strength: 1.0,
        falloff: 0.0,
        ..Brush::default()
    };
    let operation = Operation::Scatter(Scatter {
        items,
        // Under the 256 items a stamp may add, so no stamp is cut short.
        density: 0.06,
        spacing,
        erase: None,
        seed: 11,
    });
    for x in [-60.0, 0.0, 60.0] {
        for z in [-60.0, 0.0, 60.0] {
            apply(data, placement, &brush, &operation, Vec3::new(x, 0.0, z));
        }
    }
    let placement = *placement;
    move |data, source| {
        data.foliage()
            .iter()
            .filter(|instance| instance.source == *source)
            .map(|instance| placement.to_world_xz(data, Vec2::new(instance.x, instance.z)))
            .collect()
    }
}

#[test]
fn open_masks_are_the_default_and_leave_unmasked_items_exactly_where_they_were() {
    let open = ScatterMask::default();
    assert!(open.is_open());
    let (mut plain, placement) = terrain(|x, _| 20.0 + x * 0.1);
    assert_eq!(open.density(&plain, &placement, 40.3, 77.9), 1.0);
    // Numbers that aren't numbers, and ranges the wrong way round, are
    // still open or usable.
    let odd = ScatterMask {
        noise: f32::NAN,
        min_slope: 90.0,
        max_slope: 0.0,
        moisture: f32::NAN,
        ..ScatterMask::default()
    };
    assert!(odd.is_open());

    // Grass and rocks, both unmasked.
    let items = vec![ScatterItem::new(grass()), ScatterItem::new(rock())];
    let placed = stroke(&mut plain, &placement, items.clone(), 0.0);
    let (grass_before, rocks_before) = (placed(&plain, &grass()), placed(&plain, &rock()));
    assert!(grass_before.len() > 500 && rocks_before.len() > 500);

    // Mask the rocks only: every blade of grass is where it was (the
    // stroke's random numbers didn't move), and the rocks left are some of
    // the rocks there were.
    let (mut masked, _) = terrain(|x, _| 20.0 + x * 0.1);
    let mut items = items;
    items[1].mask = ScatterMask {
        noise: 1.0,
        noise_scale: 30.0,
        noise_contrast: 1.0,
        ..ScatterMask::default()
    };
    let placed = stroke(&mut masked, &placement, items, 0.0);
    assert_eq!(placed(&masked, &grass()), grass_before);
    let rocks = placed(&masked, &rock());
    assert!(
        rocks.len() > 50 && rocks.len() < rocks_before.len() * 3 / 4,
        "{} of {}",
        rocks.len(),
        rocks_before.len()
    );
    assert!(rocks.iter().all(|rock| rocks_before.contains(rock)));
}

#[test]
fn noise_gathers_an_item_into_patches_by_scale_threshold_contrast_and_seed() {
    let mask = ScatterMask {
        noise: 1.0,
        noise_scale: 24.0,
        noise_threshold: 0.5,
        noise_contrast: 1.0,
        noise_seed: 3,
        ..ScatterMask::default()
    };
    // Over a grid of spots: the share inside a patch.
    let spots: Vec<Vec2> = (0..60)
        .flat_map(|i| {
            (0..60).map(move |j| Vec2::new(i as f32 * 3.7 - 110.0, j as f32 * 3.7 - 110.0))
        })
        .collect();
    let share = |mask: &ScatterMask| {
        spots.iter().map(|&at| mask.noise_at(at)).sum::<f32>() / spots.len() as f32
    };
    // A hard edge: nearly every spot is in (1) or out (0), about half each.
    let hard = spots
        .iter()
        .filter(|&&at| {
            let v = mask.noise_at(at);
            v <= 0.01 || v >= 0.99
        })
        .count();
    assert!(hard > spots.len() * 9 / 10, "{hard} of {}", spots.len());
    let half = share(&mask);
    assert!((0.3..0.7).contains(&half), "{half}");
    // A higher threshold leaves more bare; a lower one less.
    let sparse = share(&ScatterMask {
        noise_threshold: 0.8,
        ..mask.clone()
    });
    let full = share(&ScatterMask {
        noise_threshold: 0.2,
        ..mask.clone()
    });
    assert!(
        sparse < half - 0.15 && full > half + 0.15,
        "{sparse} {half} {full}"
    );
    assert!(sparse < 0.2 && full > 0.8, "{sparse} {full}");
    // No contrast: soft edges, so many spots are in between.
    let soft = ScatterMask {
        noise_contrast: 0.0,
        ..mask.clone()
    };
    let between = spots
        .iter()
        .filter(|&&at| (0.1..0.9).contains(&soft.noise_at(at)))
        .count();
    assert!(between > spots.len() / 2, "{between}");
    // Half the amount: thinned to half between the patches, never to none.
    let light = ScatterMask {
        noise: 0.5,
        ..mask.clone()
    };
    assert!(spots.iter().all(|&at| light.noise_at(at) >= 0.5));
    // The same seed gives the same patches; another moves them; a larger
    // scale makes them larger (neighbouring spots agree more often).
    assert!(
        spots
            .iter()
            .all(|&at| mask.noise_at(at) == mask.clone().noise_at(at))
    );
    let other = ScatterMask {
        noise_seed: 4,
        ..mask.clone()
    };
    let differ = spots
        .iter()
        .filter(|&&at| (mask.noise_at(at) - other.noise_at(at)).abs() > 0.5)
        .count();
    assert!(differ > spots.len() / 5, "{differ}");
    let flips = |mask: &ScatterMask| {
        spots
            .windows(2)
            .filter(|pair| (mask.noise_at(pair[0]) - mask.noise_at(pair[1])).abs() > 0.5)
            .count()
    };
    let large = ScatterMask {
        noise_scale: 96.0,
        ..mask.clone()
    };
    assert!(
        flips(&large) * 2 < flips(&mask),
        "{} vs {}",
        flips(&large),
        flips(&mask)
    );

    // A stroke places the item only inside the patches.
    let (mut data, placement) = terrain(|_, _| 20.0);
    let item = ScatterItem {
        mask: mask.clone(),
        ..ScatterItem::new(grass())
    };
    let placed = stroke(&mut data, &placement, vec![item], 0.0);
    let placed = placed(&data, &grass());
    assert!(placed.len() > 200, "{}", placed.len());
    assert!(placed.iter().all(|&at| mask.noise_at(at) > 0.0));
    let inside = placed
        .iter()
        .filter(|&&at| mask.noise_at(at) > 0.99)
        .count();
    assert!(
        inside * 10 > placed.len() * 9,
        "{inside} of {}",
        placed.len()
    );
}

#[test]
fn slope_and_height_masks_keep_an_item_between_their_ends_with_soft_edges() {
    let mask = ScatterMask {
        min_slope: 10.0,
        max_slope: 30.0,
        ..ScatterMask::default()
    };
    assert_eq!(mask.slope_range(), Some((10.0, 30.0)));
    assert_eq!(mask.slope_at(20.0), 1.0);
    assert_eq!(mask.slope_at(5.0), 0.0);
    assert_eq!(mask.slope_at(35.0), 0.0);
    // Half way through each fade (SLOPE_FADE degrees, centered on the end).
    assert!((mask.slope_at(10.0) - 0.5).abs() < 1e-4);
    assert!((mask.slope_at(30.0) - 0.5).abs() < 1e-4);
    // An end at 0 or 90 is open: flat ground is fully in.
    let gentle = ScatterMask {
        max_slope: 12.0,
        ..ScatterMask::default()
    };
    assert_eq!(gentle.slope_at(0.0), 1.0);
    assert_eq!(gentle.slope_at(20.0), 0.0);
    let heights = ScatterMask {
        min_height: 10.0,
        max_height: 20.0,
        ..ScatterMask::default()
    };
    assert_eq!(heights.height_range(), Some((10.0, 20.0)));
    assert_eq!(heights.height_at(15.0), 1.0);
    assert_eq!(heights.height_at(7.0), 0.0);
    assert_eq!(heights.height_at(23.0), 0.0);
    assert!((heights.height_at(20.0) - 0.5).abs() < 1e-4);
    let low = ScatterMask {
        max_height: 12.0,
        ..ScatterMask::default()
    };
    assert_eq!(low.height_at(-500.0), 1.0);

    // Ground that's flat (at 10 studs) west of X = 0 and climbs 20 degrees
    // east of it: rocks on slopes from 10 degrees, grass below 14 studs.
    let rise = 20.0_f32.to_radians().tan();
    let (mut data, placement) = terrain(|x, _| 10.0 + x.max(0.0) * rise);
    let rocks = ScatterItem {
        mask: ScatterMask {
            min_slope: 10.0,
            ..ScatterMask::default()
        },
        ..ScatterItem::new(rock())
    };
    let grasses = ScatterItem {
        mask: low,
        ..ScatterItem::new(grass())
    };
    let placed = stroke(&mut data, &placement, vec![rocks, grasses], 0.0);
    let (rocks, grasses) = (placed(&data, &rock()), placed(&data, &grass()));
    assert!(rocks.len() > 200 && grasses.len() > 200);
    // A sample's slope is from its neighbours 2 studs either side, so the
    // edge is a sample wide.
    assert!(rocks.iter().all(|at| at.x > -2.1), "rocks on the flat");
    // 14 studs is 11 studs east; its fade ends 2 studs higher, 5.5 further.
    assert!(grasses.iter().all(|at| at.x < 17.0), "grass up the slope");
    assert!(grasses.iter().any(|at| at.x < -50.0));
    assert!(rocks.iter().any(|at| at.x > 50.0));
}

#[test]
fn moisture_is_wet_in_hollows_and_dry_on_rises_and_a_bias_prefers_either() {
    // A wash along X at Z = -60 (3 studs deep, 20 wide each side) and a
    // ridge along X at Z = 60 (3 studs high), on ground climbing gently
    // with X.
    let shape = |x: f32, z: f32| {
        let bump = |middle: f32| (1.0 - ((z - middle) / 20.0).powi(2)).max(0.0) * 3.0;
        30.0 + x * 0.05 - bump(-60.0) + bump(60.0)
    };
    let (mut data, placement) = terrain(shape);
    let wet = |data: &TerrainData, x: f32, z: f32, reach: f32| {
        let grid = placement.to_grid(data, x, z);
        wetness(data, &placement, grid.x, grid.y, reach)
    };
    let (wash, ridge, plain) = (
        wet(&data, 0.0, -60.0, 16.0),
        wet(&data, 0.0, 60.0, 16.0),
        wet(&data, 0.0, 0.0, 16.0),
    );
    assert!(wash > 0.9, "the wash is {wash}");
    assert!(ridge < 0.1, "the ridge is {ridge}");
    // Evenly sloping ground is neither.
    assert!((plain - 0.5).abs() < 0.02, "the plain is {plain}");
    // A reach far wider than the wash still finds it, more faintly.
    let wide = wet(&data, 0.0, -60.0, 80.0);
    assert!(wide > 0.5 && wide < wash, "{wide}");

    let mask = |bias: f32| ScatterMask {
        moisture: bias,
        ..ScatterMask::default()
    };
    assert_eq!(mask(1.0).moisture_at(1.0), 1.0);
    assert_eq!(mask(1.0).moisture_at(0.5), 0.5);
    assert_eq!(mask(1.0).moisture_at(0.0), 0.0);
    assert_eq!(mask(-1.0).moisture_at(0.0), 1.0);
    assert_eq!(mask(-1.0).moisture_at(1.0), 0.0);
    assert_eq!(mask(0.5).moisture_at(0.0), 0.5);
    assert_eq!(mask(0.0).moisture_at(0.0), 1.0);

    // Grass that likes it wet, rocks that like it dry.
    let grasses = ScatterItem {
        mask: mask(1.0),
        ..ScatterItem::new(grass())
    };
    let rocks = ScatterItem {
        mask: mask(-1.0),
        ..ScatterItem::new(rock())
    };
    let placed = stroke(&mut data, &placement, vec![grasses, rocks], 0.0);
    let near = |spots: &[Vec2], middle: f32| {
        spots
            .iter()
            .filter(|at| (at.y - middle).abs() < 6.0)
            .count()
    };
    let (grasses, rocks) = (placed(&data, &grass()), placed(&data, &rock()));
    let (grass_wash, grass_plain, grass_ridge) = (
        near(&grasses, -60.0),
        near(&grasses, 0.0),
        near(&grasses, 60.0),
    );
    let (rock_wash, rock_plain, rock_ridge) =
        (near(&rocks, -60.0), near(&rocks, 0.0), near(&rocks, 60.0));
    assert!(
        grass_wash > grass_plain * 5 / 4 && grass_plain > grass_ridge * 2,
        "grass: wash {grass_wash}, plain {grass_plain}, ridge {grass_ridge}"
    );
    assert!(
        rock_ridge > rock_plain * 5 / 4 && rock_plain > rock_wash * 2,
        "rocks: wash {rock_wash}, plain {rock_plain}, ridge {rock_ridge}"
    );
}
