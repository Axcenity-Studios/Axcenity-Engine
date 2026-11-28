//! Procedural worlds PW4 in the editor: Insert a Terrain Path, Apply it to
//! the terrain and Route it over a hill, each one undo step.

use std::sync::Arc;

use axcenity_core::{ClassName, TerrainData, Value, Vector3};
use axcenity_render::glam::{Vec2, Vec3};

use super::tests::inserted;
use super::*;
use crate::properties::Edit;

/// An editor with a 256-stud Terrain round the origin (2 studs a sample,
/// a 100-stud height range) shaped by `height` (studs, by world X and Z),
/// with a "Road" layer, and a Spline from (-110, 0) to `to` holding a
/// TerrainPath.
fn road(
    height: impl Fn(f32, f32) -> f32,
    to: Vec2,
) -> (EditorApp, InstanceId, InstanceId, InstanceId) {
    let mut app = EditorApp::new(None);
    let workspace = app.world.workspace();
    let terrain = app.world.create(ClassName::Terrain).unwrap();
    app.world.set_parent(terrain, Some(workspace)).unwrap();
    let _ = app
        .world
        .set_property(terrain, "Position", Value::Vector3(Vector3::default()));
    let _ = app.world.set_property(
        terrain,
        "Size",
        Value::Vector3(Vector3::new(256.0, 100.0, 256.0)),
    );
    let mut data = TerrainData::flat(129, 129, 0.0).unwrap();
    let all = data.bounds();
    let heights = data.heights_mut_in(all);
    for z in 0..129 {
        for x in 0..129 {
            heights[z * 129 + x] = height(x as f32 * 2.0 - 128.0, z as f32 * 2.0 - 128.0) / 100.0;
        }
    }
    app.world.set_terrain(terrain, Arc::new(data));
    for name in ["Grass", "Road"] {
        let layer = app.world.create(ClassName::TerrainLayer).unwrap();
        app.world.set_name(layer, name.to_owned()).unwrap();
        app.world.set_parent(layer, Some(terrain)).unwrap();
    }
    let spline = inserted(&mut app, Insertable::Spline, None);
    let points = crate::splines::points(&app.world, spline);
    for (point, at) in points.iter().zip([Vec2::new(-110.0, 0.0), to]) {
        let _ = app.world.set_property(
            *point,
            "Position",
            Value::Vector3(Vector3::new(at.x, height(at.x, at.y), at.y)),
        );
    }
    let path = inserted(&mut app, Insertable::TerrainPath, Some(spline));
    assert_eq!(app.world.get(path).unwrap().parent(), Some(spline));
    app.history.seal();
    (app, terrain, spline, path)
}

#[test]
fn apply_flattens_and_paints_under_the_path_as_one_step() {
    let (mut app, terrain, spline, path) = road(
        |x, z| 20.0 + (x * 0.2).sin() * 3.0 + z * 0.1,
        Vec2::new(100.0, 0.0),
    );
    app.apply(path, Edit::Property("Layer", Value::String("Road".into())));
    app.history.seal();
    let ground = |app: &EditorApp, x: f32, z: f32| {
        let data = app.world.terrain(terrain).unwrap();
        let placement = axcenity_terrain::Placement::of(app.world.get(terrain).unwrap()).unwrap();
        (
            placement.ground_y(data, x, z).unwrap(),
            data.paint()[data.index(
                placement.to_grid(data, x, z).x as usize,
                placement.to_grid(data, x, z).y as usize,
            )][1],
        )
    };
    let was = ground(&app, 10.0, 0.0);
    app.apply(path, Edit::ApplyTerrainPath);
    let name = app.world.get(path).unwrap().name().to_owned();
    assert_eq!(
        app.history.undo_label(),
        Some(format!("Apply {name}").as_str())
    );
    let curve = axcenity_core::spline::SplineCurve::of(&app.world, spline).unwrap();
    let on_curve = curve.position_at_distance(curve.closest_distance(Vector3::new(10.0, 0.0, 0.0)));
    let (height, paint) = ground(&app, 10.0, 0.0);
    assert!(
        (height - on_curve.y).abs() < 0.2,
        "{height} vs {}",
        on_curve.y
    );
    assert_eq!(paint, 255);
    // Past Width / 2 + Falloff (4 + 4), untouched.
    assert_eq!(ground(&app, 10.0, 20.0).1, 0);
    app.undo();
    assert_eq!(ground(&app, 10.0, 0.0), was);
}

#[test]
fn route_lays_points_up_a_hill_within_max_grade_as_one_step() {
    let cone = |x: f32, z: f32| (50.0 - Vec2::new(x, z).length() * 0.4).clamp(5.0, 45.0);
    let (mut app, _, spline, path) = road(cone, Vec2::ZERO);
    let ends = crate::splines::points(&app.world, spline);
    assert_eq!(ends.len(), 2);
    app.apply(path, Edit::RouteTerrainPath);
    let name = app.world.get(path).unwrap().name().to_owned();
    assert_eq!(
        app.history.undo_label(),
        Some(format!("Route {name}").as_str())
    );
    let points = crate::splines::points(&app.world, spline);
    assert!(points.len() > 4, "{} points", points.len());
    assert_eq!(points.first(), ends.first());
    assert_eq!(points.last(), ends.last());
    let at = |id: InstanceId| {
        let v = app
            .world
            .get(id)
            .unwrap()
            .property("Position")
            .unwrap()
            .as_vector3()
            .unwrap();
        Vec3::new(v.x, v.y, v.z)
    };
    let limit = 8.0_f32.to_radians().tan();
    for pair in points.windows(2) {
        let (a, b) = (at(pair[0]), at(pair[1]));
        let run = Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z));
        assert!((b.y - a.y).abs() <= limit * run + 0.01, "{a} to {b}");
    }
    // The TerrainPath is still in the Spline, after the points.
    assert_eq!(app.world.get(path).unwrap().parent(), Some(spline));
    app.undo();
    assert_eq!(crate::splines::points(&app.world, spline), ends);
}

#[test]
fn apply_paints_a_ragged_shoulder_and_clears_painted_foliage_as_one_step() {
    let (mut app, terrain, _, path) = road(|_, _| 20.0, Vec2::new(100.0, 0.0));
    let gravel = app.world.create(ClassName::TerrainLayer).unwrap();
    app.world.set_name(gravel, "Gravel".to_owned()).unwrap();
    app.world.set_parent(gravel, Some(terrain)).unwrap();
    // Painted scrub every 4 studs over the middle of the terrain.
    let mut data = TerrainData::clone(app.world.terrain(terrain).unwrap());
    for z in (40..90).step_by(2) {
        for x in (20..110).step_by(2) {
            data.foliage_mut().push(axcenity_core::FoliageInstance {
                source: axcenity_core::FoliageSource::Plant(axcenity_core::FoliagePlant::Bush),
                x: x as f32 + 0.3,
                z: z as f32 + (x % 5) as f32 * 0.2,
                yaw: 0.0,
                scale: 1.0,
                collide: false,
                shadow: true,
                lean: 0.0,
                sink: 0.0,
            });
        }
    }
    app.world.set_terrain(terrain, Arc::new(data));
    for (name, value) in [
        ("Layer", Value::String("Road".into())),
        ("ShoulderLayer", Value::String("Gravel".into())),
        ("Falloff", Value::Number(10.0)),
        ("ClearFalloff", Value::Number(10.0)),
        ("Flatten", Value::Bool(false)),
    ] {
        app.apply(path, Edit::Property(name, value));
    }
    app.history.seal();
    let was = app.world.terrain(terrain).unwrap().clone();
    let placement = axcenity_terrain::Placement::of(app.world.get(terrain).unwrap()).unwrap();
    // The road's and the shoulder's paint at world (x, z), and how many
    // painted bushes stand within `from` to `to` studs of the road.
    let paint = |app: &EditorApp, x: f32, z: f32| {
        let data = app.world.terrain(terrain).unwrap();
        let grid = placement.to_grid(data, x, z);
        let weights = data.paint()[data.index(grid.x as usize, grid.y as usize)];
        (weights[1], weights[2])
    };
    let bushes = |app: &EditorApp, from: f32, to: f32| {
        let data = app.world.terrain(terrain).unwrap();
        data.foliage()
            .iter()
            .filter(|i| {
                let at = placement.to_world_xz(data, Vec2::new(i.x, i.z));
                at.x.abs() < 60.0 && (from..to).contains(&at.y.abs())
            })
            .count()
    };
    let before = (
        bushes(&app, 0.0, 5.0),
        bushes(&app, 5.0, 15.0),
        bushes(&app, 15.0, 40.0),
    );
    assert!(before.0 > 30, "{before:?}");

    app.apply(path, Edit::ApplyTerrainPath);
    let name = app.world.get(path).unwrap().name().to_owned();
    assert_eq!(
        app.history.undo_label(),
        Some(format!("Apply {name}").as_str())
    );
    // The road is all Road; just past its edge (4 studs) is all Gravel.
    assert_eq!(paint(&app, 10.0, 0.0), (255, 0));
    let beside = paint(&app, 10.0, 6.0);
    assert!(beside.0 < 128 && beside.1 > 100, "{beside:?}");
    // The shoulder's outer edge isn't a straight line: how many samples
    // out (2 studs each) Gravel reaches differs along the road.
    let reach = |app: &EditorApp, x: f32| {
        (0..20)
            .take_while(|&step| step < 3 || paint(app, x, step as f32 * 2.0).1 > 0)
            .count()
    };
    let reaches: Vec<usize> = (-30..30)
        .map(|step| reach(&app, step as f32 * 2.0))
        .collect();
    let (least, most) = (
        *reaches.iter().min().unwrap(),
        *reaches.iter().max().unwrap(),
    );
    assert!(most >= least + 2, "an even edge: {reaches:?}");
    // No bushes on the road, fewer beside it, all of them further off.
    let after = (
        bushes(&app, 0.0, 5.0),
        bushes(&app, 5.0, 15.0),
        bushes(&app, 15.0, 40.0),
    );
    assert_eq!(after.0, 0);
    assert!(after.1 > 0 && after.1 < before.1, "{after:?} of {before:?}");
    assert_eq!(after.2, before.2);
    assert!(
        app.output
            .lines()
            .iter()
            .any(|entry| entry.text.contains("painted items off it")),
    );

    // One undo puts the paint and every bush back.
    app.undo();
    let data = app.world.terrain(terrain).unwrap();
    assert_eq!(data.paint(), was.paint());
    assert_eq!(data.foliage(), was.foliage());
    app.redo();
    assert_eq!(paint(&app, 10.0, 0.0), (255, 0));
    assert_eq!(bushes(&app, 0.0, 5.0), 0);

    // A Shoulder Layer the terrain doesn't have is a warning, not an error.
    app.apply(
        path,
        Edit::Property("ShoulderLayer", Value::String("Dust".into())),
    );
    app.apply(path, Edit::ApplyTerrainPath);
    assert!(
        app.output
            .lines()
            .iter()
            .any(|entry| entry.text.contains("Shoulder Layer Dust isn't one of")),
    );
}
