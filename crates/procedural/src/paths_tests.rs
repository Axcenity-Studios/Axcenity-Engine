//! Procedural worlds PW4: roads and paths on the terrain.

use std::sync::Arc;

use axcenity_core::spline::SplineCurve;
use axcenity_core::{ClassName, InstanceId, TerrainData, Value, Vector3, World};
use axcenity_terrain::Placement;
use glam::{Vec2, Vec3};

use crate::paths::{Applied, PathSettings, apply_path, keep_offs};
use crate::route::route;
use crate::terrain_scatter::TerrainScatter;

fn set(world: &mut World, id: InstanceId, name: &str, value: Value) {
    world.set_property(id, name, value).unwrap();
}

/// A 256-stud terrain round the origin (129 samples, 2 studs apart, a
/// 100-stud height range) with these heights in studs, by world X and Z.
fn terrain(world: &mut World, height: impl Fn(f32, f32) -> f32) -> InstanceId {
    let workspace = world.workspace();
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(workspace)).unwrap();
    set(
        world,
        terrain,
        "Position",
        Value::Vector3(Vector3::default()),
    );
    set(
        world,
        terrain,
        "Size",
        Value::Vector3(Vector3::new(256.0, 100.0, 256.0)),
    );
    let mut data = TerrainData::flat(129, 129, 0.0).unwrap();
    let all = data.bounds();
    let heights = data.heights_mut_in(all);
    for z in 0..129 {
        for x in 0..129 {
            let (wx, wz) = (x as f32 * 2.0 - 128.0, z as f32 * 2.0 - 128.0);
            heights[z * 129 + x] = height(wx, wz) / 100.0;
        }
    }
    world.set_terrain(terrain, Arc::new(data));
    terrain
}

fn placement(world: &World, terrain: InstanceId) -> Placement {
    Placement::of(world.get(terrain).unwrap()).unwrap()
}

/// A Spline through `points` with a TerrainPath in it.
fn path(world: &mut World, points: &[(f32, f32, f32)]) -> (InstanceId, InstanceId) {
    let workspace = world.workspace();
    let spline = world.create(ClassName::Spline).unwrap();
    world.set_parent(spline, Some(workspace)).unwrap();
    for &(x, y, z) in points {
        let point = world.create(ClassName::SplinePoint).unwrap();
        set(
            world,
            point,
            "Position",
            Value::Vector3(Vector3::new(x, y, z)),
        );
        world.set_parent(point, Some(spline)).unwrap();
    }
    let path = world.create(ClassName::TerrainPath).unwrap();
    world.set_parent(path, Some(spline)).unwrap();
    (spline, path)
}

/// World X and Z of every generated plant.
fn plants(world: &World, terrain: InstanceId) -> Vec<Vec2> {
    let data = world.terrain(terrain).unwrap();
    let placement = placement(world, terrain);
    data.generated_foliage()
        .map(|i| placement.to_world_xz(data, Vec2::new(i.x, i.z)))
        .collect()
}

#[test]
fn scatter_rules_leave_a_paths_strip_empty_and_moving_it_remakes_only_chunks_near_it() {
    let mut world = World::new();
    let land = terrain(&mut world, |_, _| 10.0);
    let rule = world.create(ClassName::ScatterRule).unwrap();
    world.set_parent(rule, Some(land)).unwrap();
    set(
        &mut world,
        rule,
        "Models",
        Value::String("plant:Tree".into()),
    );
    set(&mut world, rule, "Spacing", Value::Number(3.0));
    set(&mut world, rule, "Density", Value::Number(100.0));
    let mut scatter = TerrainScatter::default();
    scatter.update(&mut world);
    let near_road = |plants: &[Vec2], reach: f32| {
        plants
            .iter()
            .filter(|p| p.y.abs() < reach && p.x.abs() < 100.0)
            .count()
    };
    let before = plants(&world, land);
    assert!(near_road(&before, 5.0) > 20, "{}", near_road(&before, 5.0));

    // A straight road along X at Z = 0: 8 wide, kept 1 stud past its edges.
    let (spline, road) = path(&mut world, &[(-100.0, 10.0, 0.0), (100.0, 10.0, 0.0)]);
    assert_eq!(keep_offs(&world).len(), 1);
    assert!(scatter.update(&mut world) > 0);
    let after = plants(&world, land);
    assert_eq!(near_road(&after, 5.0), 0, "plants on the road");
    // Away from it, the same plants as before.
    let far = |plants: &[Vec2]| {
        let mut far: Vec<_> = plants
            .iter()
            .filter(|p| p.y.abs() > 20.0)
            .map(|p| (p.x.to_bits(), p.y.to_bits()))
            .collect();
        far.sort();
        far
    };
    assert_eq!(far(&before), far(&after));

    // Nothing changed: nothing made. Moving one end remakes only chunks
    // near the road (of 16 on this terrain).
    assert_eq!(scatter.update(&mut world), 0);
    let end = world.get(spline).unwrap().children()[1];
    set(
        &mut world,
        end,
        "Position",
        Value::Vector3(Vector3::new(100.0, 10.0, 6.0)),
    );
    let remade = scatter.update(&mut world);
    assert!((1..16).contains(&remade), "{remade} chunks remade");

    // Off, or not clearing scatter, the plants come back.
    set(&mut world, road, "ClearScatter", Value::Bool(false));
    scatter.update(&mut world);
    assert!(near_road(&plants(&world, land), 5.0) > 20);
}

#[test]
fn apply_flattens_and_paints_within_width_and_blends_over_its_falloff() {
    let mut world = World::new();
    // Ground sloping up along Z, a stud every 4.
    let land = terrain(&mut world, |_, z| 30.0 + z / 4.0);
    let (spline, road) = path(&mut world, &[(-60.0, 30.0, 0.0), (60.0, 30.0, 0.0)]);
    set(&mut world, road, "Width", Value::Number(10.0));
    set(&mut world, road, "Falloff", Value::Number(6.0));
    // An even edge, to measure the blend (the ragged one is tested below).
    set(&mut world, road, "EdgeNoise", Value::Number(0.0));
    let settings = PathSettings::of(world.get(road).unwrap()).unwrap();
    let curve = SplineCurve::of(&world, spline).unwrap();
    let placement = placement(&world, land);
    let mut data = TerrainData::clone(world.terrain(land).unwrap());
    let changed = apply_path(&mut data, &placement, &curve, &settings, Some(2), None).samples;
    assert!(changed > 300, "{changed}");
    let ground = |z: f32| placement.ground_y(&data, 0.0, z).unwrap();
    let paint = |z: f32| {
        let grid = placement.to_grid(&data, 0.0, z);
        data.paint()[data.index(grid.x as usize, grid.y as usize)][2]
    };
    // Within the width: level with the road, fully painted.
    for z in [-4.0, 0.0, 4.0] {
        assert!((ground(z) - 30.0).abs() < 0.05, "{z}: {}", ground(z));
        assert_eq!(paint(z), 255, "{z}");
    }
    // In the falloff: between the road and the slope, partly painted.
    let blend = ground(8.0);
    assert!(blend > 30.05 && blend < 32.0, "{blend}");
    assert!((1..255).contains(&paint(8.0)), "{}", paint(8.0));
    // Past it: untouched.
    for z in [-12.0, 12.0] {
        assert!((ground(z) - (30.0 + z / 4.0)).abs() < 0.05, "{z}");
        assert_eq!(paint(z), 0);
    }
    // Flatten off and no layer: nothing to do.
    set(&mut world, road, "Flatten", Value::Bool(false));
    let settings = PathSettings::of(world.get(road).unwrap()).unwrap();
    assert_eq!(
        apply_path(&mut data, &placement, &curve, &settings, None, None),
        Applied::default()
    );
}

/// A straight road along X at Z = 0 on flat ground 30 studs up, with these
/// settings, and its terrain's data.
fn straight_road(
    properties: &[(&str, Value)],
) -> (PathSettings, SplineCurve, Placement, TerrainData) {
    let mut world = World::new();
    let land = terrain(&mut world, |_, _| 30.0);
    let (spline, road) = path(&mut world, &[(-100.0, 30.0, 0.0), (100.0, 30.0, 0.0)]);
    for (name, value) in properties {
        set(&mut world, road, name, value.clone());
    }
    let settings = PathSettings::of(world.get(road).unwrap()).unwrap();
    let curve = SplineCurve::of(&world, spline).unwrap();
    let placement = placement(&world, land);
    let data = TerrainData::clone(world.terrain(land).unwrap());
    (settings, curve, placement, data)
}

#[test]
fn a_shoulder_layer_is_feathered_from_the_roads_edge_over_the_falloff() {
    // 8 wide (4 each side), a 12-stud shoulder, an even edge.
    let (settings, curve, placement, mut data) = straight_road(&[
        ("Falloff", Value::Number(12.0)),
        ("EdgeNoise", Value::Number(0.0)),
        ("Flatten", Value::Bool(false)),
    ]);
    // The weights across the shoulder, as numbers.
    let at = Vec2::new(0.0, 0.0);
    assert_eq!(settings.paint_amounts(0.0, at, true), (1.0, 1.0));
    assert_eq!(settings.paint_amounts(4.0, at, true), (1.0, 1.0));
    // The road's layer is gone a quarter of the way out (3 studs); the
    // shoulder is half way down at its middle (6 studs) and gone at 12.
    let (road, verge) = settings.paint_amounts(5.5, at, true);
    assert!((road - 0.5).abs() < 1e-4 && verge > 0.9, "{road} {verge}");
    assert_eq!(settings.paint_amounts(7.0, at, true).0, 0.0);
    assert!((settings.paint_amounts(10.0, at, true).1 - 0.5).abs() < 1e-4);
    assert_eq!(settings.paint_amounts(16.0, at, true), (0.0, 0.0));
    // Each falls steadily from the edge out.
    let amounts: Vec<(f32, f32)> = (0..=24)
        .map(|step| settings.paint_amounts(4.0 + step as f32 * 0.5, at, true))
        .collect();
    assert!(
        amounts
            .windows(2)
            .all(|pair| pair[1].0 <= pair[0].0 && pair[1].1 <= pair[0].1)
    );
    // Without a shoulder, the road's layer takes the whole falloff, as before.
    assert!((settings.paint_amounts(10.0, at, false).0 - 0.5).abs() < 1e-4);
    assert_eq!(settings.paint_amounts(10.0, at, false).1, 0.0);

    // On the terrain: layer 2 is the road, layer 3 its shoulder.
    let applied = apply_path(&mut data, &placement, &curve, &settings, Some(2), Some(3));
    assert!(applied.samples > 1000, "{applied:?}");
    let paint = |z: f32| {
        let grid = placement.to_grid(&data, 0.0, z);
        let weights = data.paint()[data.index(grid.x as usize, grid.y as usize)];
        (weights[2], weights[3])
    };
    // On the road: all road. Beside it: all shoulder. Further: shoulder
    // fading into the ground (what's left is the terrain's own rules).
    assert_eq!(paint(0.0), (255, 0));
    assert_eq!(paint(4.0), (255, 0));
    assert_eq!(paint(8.0).0, 0);
    assert!(paint(8.0).1 > 180, "{:?}", paint(8.0));
    let (near, far) = (paint(10.0).1, paint(14.0).1);
    assert!(near > far && far > 0 && near < 255, "{near} {far}");
    assert_eq!(paint(16.0), (0, 0));
    assert_eq!(paint(-10.0).1, near, "the same on both sides");
    // Only a shoulder: it covers the road too.
    let (_, _, _, mut data) = straight_road(&[]);
    apply_path(&mut data, &placement, &curve, &settings, None, Some(3));
    let grid = placement.to_grid(&data, 0.0, 0.0);
    assert_eq!(
        data.paint()[data.index(grid.x as usize, grid.y as usize)][3],
        255
    );
}

#[test]
fn edge_noise_makes_the_paints_outer_edge_ragged_and_leaves_the_road_and_heights_even() {
    let ragged = [
        ("Falloff", Value::Number(12.0)),
        ("EdgeNoise", Value::Number(1.0)),
        ("NoiseScale", Value::Number(12.0)),
    ];
    let (settings, curve, placement, mut data) = straight_road(&ragged);
    assert_eq!(settings.paint_reach(), 4.0 + 24.0);
    // The reach past the edge changes from place to place, between 0 and
    // twice the falloff, smoothly.
    let reaches: Vec<f32> = (0..200)
        .map(|step| settings.edge_reach(Vec2::new(step as f32 - 100.0, 10.0)))
        .collect();
    let (least, most) = reaches
        .iter()
        .fold((f32::MAX, f32::MIN), |(low, high), &r| {
            (low.min(r), high.max(r))
        });
    assert!(least >= 0.0 && most <= 2.0, "{least} {most}");
    assert!(
        most - least > 0.8,
        "the edge barely moves: {least} to {most}"
    );
    assert!(
        reaches
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.35)
    );

    // A hill to flatten, so the heights can be compared.
    let all = data.bounds();
    for height in data.heights_mut_in(all) {
        *height = 0.34;
    }
    let mut even_data = data.clone();
    apply_path(&mut data, &placement, &curve, &settings, Some(2), Some(3));
    let even = PathSettings {
        edge_noise: 0.0,
        ..settings.clone()
    };
    apply_path(&mut even_data, &placement, &curve, &even, Some(2), Some(3));
    // The same heights with or without it, and the road itself all road.
    assert_eq!(data.heights(), even_data.heights());
    // How far the shoulder reaches (in samples, 2 studs each) at each X.
    let reach_at = |data: &TerrainData, x: usize| {
        (64..129)
            .take_while(|&z| data.paint()[data.index(x, z)][3] > 0 || z < 67)
            .count()
    };
    let columns = 20..110;
    for x in columns.clone() {
        for z in 62..=66 {
            assert_eq!(
                data.paint()[data.index(x, z)][2],
                255,
                "the road at {x}, {z}"
            );
        }
    }
    let even_reaches: Vec<usize> = columns.clone().map(|x| reach_at(&even_data, x)).collect();
    assert!(
        even_reaches.iter().all(|&r| r == even_reaches[0]),
        "{even_reaches:?}"
    );
    let ragged_reaches: Vec<usize> = columns.map(|x| reach_at(&data, x)).collect();
    let (least, most) = (
        *ragged_reaches.iter().min().unwrap(),
        *ragged_reaches.iter().max().unwrap(),
    );
    assert!(most >= least + 3, "the edge is even: {ragged_reaches:?}");
    assert!(
        least < even_reaches[0] && most > even_reaches[0],
        "{ragged_reaches:?}"
    );
    // Never past its reach: 28 studs is 14 samples.
    assert!(most <= 15, "{most}");
    // The same again: the edge is by place, not by chance.
    let (_, _, _, mut again) = straight_road(&ragged);
    let all = again.bounds();
    for height in again.heights_mut_in(all) {
        *height = 0.34;
    }
    apply_path(&mut again, &placement, &curve, &settings, Some(2), Some(3));
    assert_eq!(again.paint(), data.paint());
}

/// Painted grass on every other sample of `data` (4 studs apart).
fn sow(data: &mut TerrainData) {
    let foliage = data.foliage_mut();
    for z in (2..127).step_by(2) {
        for x in (2..127).step_by(2) {
            foliage.push(axcenity_core::FoliageInstance {
                source: axcenity_core::FoliageSource::Plant(axcenity_core::FoliagePlant::Grass),
                // Off the samples, so each has a place of its own.
                x: x as f32 + 0.25,
                z: z as f32 + (x % 7) as f32 * 0.25,
                yaw: 0.0,
                scale: 1.0,
                collide: false,
                lean: 0.0,
                sink: 0.0,
            });
        }
    }
}

#[test]
fn apply_takes_painted_foliage_off_the_strip_and_thins_it_over_the_clear_falloff() {
    // 8 wide and a stud of margin: clear to 5 studs, thinning to 25.
    let (settings, curve, placement, mut data) = straight_road(&[
        ("ClearFalloff", Value::Number(20.0)),
        ("Flatten", Value::Bool(false)),
    ]);
    assert_eq!(settings.scatter_kept(0.0), 0.0);
    assert_eq!(settings.scatter_kept(5.0), 0.0);
    assert!((settings.scatter_kept(15.0) - 0.5).abs() < 1e-4);
    assert_eq!(settings.scatter_kept(25.0), 1.0);
    assert_eq!(settings.scatter_kept(60.0), 1.0);
    sow(&mut data);
    let sown = data.foliage().len();
    let band = |data: &TerrainData, from: f32, to: f32| {
        data.foliage()
            .iter()
            .filter(|i| {
                let at = placement.to_world_xz(data, Vec2::new(i.x, i.z));
                at.x.abs() < 90.0 && (from..to).contains(&at.y.abs())
            })
            .count()
    };
    let before = [
        band(&data, 0.0, 5.0),
        band(&data, 5.0, 15.0),
        band(&data, 15.0, 25.0),
        band(&data, 25.0, 60.0),
    ];
    // No layers and no flattening: the foliage alone is something to do.
    let applied = apply_path(&mut data, &placement, &curve, &settings, None, None);
    assert_eq!(applied.samples, 0);
    assert_eq!(applied.foliage, sown - data.foliage().len());
    let after = [
        band(&data, 0.0, 5.0),
        band(&data, 5.0, 15.0),
        band(&data, 15.0, 25.0),
        band(&data, 25.0, 60.0),
    ];
    assert!(before[0] > 80, "{before:?}");
    // None on the strip; few beside it; most further out; all past it.
    assert_eq!(after[0], 0, "{after:?}");
    assert!(after[1] * 10 < before[1] * 4, "{after:?} of {before:?}");
    assert!(after[1] > 0 && after[2] > after[1], "{after:?}");
    assert!(
        after[2] * 10 > before[2] * 6 && after[2] < before[2],
        "{after:?} of {before:?}"
    );
    assert_eq!(after[3], before[3]);
    // Applying again takes no more: which go is by each one's place.
    let again = apply_path(&mut data, &placement, &curve, &settings, None, None);
    assert_eq!(again, Applied::default());

    // ClearScatter off leaves the foliage alone; no falloff is a clean edge.
    let (off, ..) = straight_road(&[
        ("ClearScatter", Value::Bool(false)),
        ("Flatten", Value::Bool(false)),
    ]);
    let (_, _, _, mut data) = straight_road(&[]);
    sow(&mut data);
    assert_eq!(
        apply_path(&mut data, &placement, &curve, &off, None, None),
        Applied::default()
    );
    let (clean, ..) = straight_road(&[("ClearFalloff", Value::Number(0.0))]);
    apply_path(&mut data, &placement, &curve, &clean, None, None);
    assert_eq!(band(&data, 0.0, 5.0), 0);
    assert_eq!(band(&data, 5.0, 15.0), before[1]);
}

#[test]
fn scatter_rules_thin_out_over_a_paths_clear_falloff() {
    let mut world = World::new();
    let land = terrain(&mut world, |_, _| 10.0);
    let rule = world.create(ClassName::ScatterRule).unwrap();
    world.set_parent(rule, Some(land)).unwrap();
    set(
        &mut world,
        rule,
        "Models",
        Value::String("plant:Grass".into()),
    );
    set(&mut world, rule, "Spacing", Value::Number(2.0));
    set(&mut world, rule, "Density", Value::Number(100.0));
    let mut scatter = TerrainScatter::default();
    scatter.update(&mut world);
    let band = |plants: &[Vec2], from: f32, to: f32| {
        plants
            .iter()
            .filter(|p| p.x.abs() < 90.0 && (from..to).contains(&p.y.abs()))
            .count()
    };
    let before = plants(&world, land);
    let (_, road) = path(&mut world, &[(-100.0, 10.0, 0.0), (100.0, 10.0, 0.0)]);
    set(&mut world, road, "ClearFalloff", Value::Number(20.0));
    let strip = &keep_offs(&world)[0];
    assert_eq!((strip.reach, strip.fade), (5.0, 20.0));
    scatter.update(&mut world);
    let after = plants(&world, land);
    assert_eq!(band(&after, 0.0, 5.0), 0);
    let (near, far) = (band(&after, 5.0, 15.0), band(&after, 15.0, 25.0));
    assert!(near * 10 < band(&before, 5.0, 15.0) * 4, "{near}");
    assert!(near > 0 && far > near * 2, "{near} then {far}");
    assert!(far < band(&before, 15.0, 25.0));
    assert_eq!(band(&after, 25.0, 60.0), band(&before, 25.0, 60.0));
    // Every plant left is one that was there: thinning only takes away.
    let was: std::collections::HashSet<(u32, u32)> = before
        .iter()
        .map(|p| (p.x.to_bits(), p.y.to_bits()))
        .collect();
    assert!(
        after
            .iter()
            .all(|p| was.contains(&(p.x.to_bits(), p.y.to_bits())))
    );
    // No falloff: the clean edge it always was, and the strip's hash is
    // the one it had before strips could fade.
    set(&mut world, road, "ClearFalloff", Value::Number(0.0));
    let clean = keep_offs(&world)[0].clone();
    assert_eq!(clean.fade, 0.0);
    assert_eq!(
        clean.hash,
        crate::paths::KeepOff::new(clean.line.clone(), clean.reach)
            .unwrap()
            .hash
    );
    scatter.update(&mut world);
    let after = plants(&world, land);
    assert_eq!(band(&after, 5.0, 15.0), band(&before, 5.0, 15.0));
}

#[test]
fn route_winds_up_a_steep_hill_keeping_every_stretch_under_its_grade() {
    let mut world = World::new();
    // A cone about 22 degrees steep, 45 studs high, with a flat top.
    let land = terrain(&mut world, |x, z| {
        (50.0 - Vec2::new(x, z).length() * 0.4).clamp(5.0, 45.0)
    });
    let placement = placement(&world, land);
    let data = world.terrain(land).unwrap();
    let (from, to) = (Vec3::new(-110.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0));
    let grade = 10.0_f32;
    let points = route(data, &placement, (from, to), grade, 8.0).expect("a way up");
    assert!(points.len() > 4, "{points:?}");
    let flat = |p: Vec3| Vec2::new(p.x, p.z);
    assert!(flat(points[0]).distance(flat(from)) < 1e-3);
    assert!(flat(*points.last().unwrap()).distance(flat(to)) < 1e-3);
    let limit = grade.to_radians().tan();
    let mut length = 0.0;
    for pair in points.windows(2) {
        let run = flat(pair[0]).distance(flat(pair[1]));
        length += run;
        let climb = (pair[1].y - pair[0].y).abs();
        assert!(
            climb <= limit * run + 0.6,
            "{:?} to {:?}: {} degrees",
            pair[0],
            pair[1],
            (climb / run).atan().to_degrees()
        );
    }
    // 43 studs up at 10 degrees takes at least 244 studs: much further
    // than straight up the slope.
    assert!(length > 240.0, "{length}");
    // Each point is on the ground, or cut into it or built up over it by
    // at most half the road's width.
    for p in &points {
        let ground = placement.ground_y(data, p.x, p.z).unwrap();
        assert!((ground - p.y).abs() <= 4.01, "{p:?} vs {ground}");
    }
    // The curve through them climbs within the grade (a little over where
    // it might round a turn), measured over 4-stud stretches.
    let mut world = World::new();
    let (spline, _) = path(
        &mut world,
        &points.iter().map(|p| (p.x, p.y, p.z)).collect::<Vec<_>>(),
    );
    let curve = SplineCurve::of(&world, spline).unwrap();
    let line = curve.polyline(4.0);
    let steepest = line
        .windows(2)
        .map(|pair| {
            let (a, b) = (crate::v3(pair[0]), crate::v3(pair[1]));
            ((b.y - a.y).abs() / flat(a).distance(flat(b)).max(1e-3))
                .atan()
                .to_degrees()
        })
        .fold(0.0_f32, f32::max);
    assert!(steepest < grade + 1.0, "{steepest} degrees");
}

#[test]
fn route_gives_up_where_no_way_is_within_the_grade() {
    let mut world = World::new();
    // A wall: 60 studs up in 2.
    let land = terrain(&mut world, |x, _| if x < 0.0 { 0.0 } else { 60.0 });
    let placement = placement(&world, land);
    let data = world.terrain(land).unwrap();
    let points = route(
        data,
        &placement,
        (Vec3::new(-50.0, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0)),
        10.0,
        8.0,
    );
    assert_eq!(points, None);
    // Off the terrain: none either.
    assert_eq!(
        route(
            data,
            &placement,
            (Vec3::new(-500.0, 0.0, 0.0), Vec3::ZERO),
            10.0,
            8.0
        ),
        None
    );
}
