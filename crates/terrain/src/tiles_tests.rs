use super::*;
use axcenity_core::{Value, Vector3};

fn terrain(world: &mut World, position: Vec3, size: Vec3, samples: usize) -> InstanceId {
    let id = world.create(ClassName::Terrain).unwrap();
    let vector = |v: Vec3| Value::Vector3(Vector3::new(v.x, v.y, v.z));
    world
        .set_property(id, "Position", vector(position))
        .unwrap();
    world.set_property(id, "Size", vector(size)).unwrap();
    world.set_terrain(
        id,
        std::sync::Arc::new(TerrainData::flat(samples, samples, 0.25).unwrap()),
    );
    world.set_parent(id, Some(world.workspace())).unwrap();
    id
}

fn ramp(size: usize, along_x: bool) -> TerrainData {
    let mut data = TerrainData::flat(size, size, 0.0).unwrap();
    let all = data.bounds();
    let heights = data.heights_mut_in(all);
    for z in 0..size {
        for x in 0..size {
            let t = if along_x { x } else { z } as f32 / (size - 1) as f32;
            heights[z * size + x] = 0.2 + 0.5 * t;
        }
    }
    data
}

#[test]
fn a_neighbours_placement_shares_its_edge_exactly() {
    let placement = Placement {
        position: Vec3::new(10.0, -8.0, 20.0),
        size: Vec3::new(64.0, 32.0, 128.0),
    };
    let east = neighbour_placement(placement, Side::PosX);
    assert_eq!(east.position, Vec3::new(74.0, -8.0, 20.0));
    assert_eq!(east.size, placement.size);
    // Its -X edge is the first's +X edge.
    let edge = |p: Placement, dx: f32| p.position.x + dx * p.size.x / 2.0;
    assert_eq!(edge(placement, 1.0), edge(east, -1.0));
    let south = neighbour_placement(placement, Side::NegZ);
    assert_eq!(south.position, Vec3::new(10.0, -8.0, -108.0));
    for side in Side::ALL {
        let across = neighbour_placement(placement, side);
        assert_eq!(
            side_between(placement, (33, 65), across, (33, 65)),
            Some(side)
        );
        assert_eq!(neighbour_placement(across, side.opposite()), placement);
    }
}

#[test]
fn neighbours_need_the_same_size_resolution_and_height() {
    let mut world = World::new();
    let size = Vec3::new(64.0, 32.0, 64.0);
    let a = terrain(&mut world, Vec3::ZERO, size, 33);
    let east = terrain(&mut world, Vec3::new(64.0, 0.0, 0.0), size, 33);
    // Finer, taller, higher or a gap apart: not neighbours.
    let finer = terrain(&mut world, Vec3::new(-64.0, 0.0, 0.0), size, 65);
    let taller = terrain(
        &mut world,
        Vec3::new(0.0, 0.0, 64.0),
        Vec3::new(64.0, 40.0, 64.0),
        33,
    );
    let _higher = terrain(&mut world, Vec3::new(0.0, 5.0, -64.0), size, 33);
    let _apart = terrain(&mut world, Vec3::new(130.0, 0.0, 0.0), size, 33);
    assert_eq!(neighbours(&world, a), vec![(Side::PosX, east)]);
    assert_eq!(neighbour_on(&world, east, Side::NegX), Some(a));
    assert_eq!(neighbour_on(&world, a, Side::NegX), None);
    // Every side of `a` is covered by some terrain, neighbour or not.
    assert!(free_sides(&world, a).is_empty());
    assert_eq!(
        free_sides(&world, east),
        vec![Side::NegZ, Side::PosZ],
        "the one 2 studs off overlaps its +X"
    );
    let _ = (finer, taller);
    assert_eq!(connected(&world, a), vec![a, east]);
    let far_east = terrain(&mut world, Vec3::new(128.0, 0.0, 0.0), size, 33);
    assert_eq!(connected(&world, far_east), vec![far_east, east, a]);
}

#[test]
fn stitching_makes_a_shared_edge_the_same_on_both() {
    let mut a = ramp(17, true);
    let mut b = TerrainData::flat(17, 17, 0.1).unwrap();
    let all = b.bounds();
    b.paint_mut_in(all).fill([0, 200, 0, 0, 0, 0, 0, 0]);
    let (rect_a, rect_b) = stitch(&mut a, &mut b, Side::PosX).unwrap();
    assert_eq!(rect_a, SampleRect::new(16, 0, 17, 17));
    assert_eq!(rect_b, SampleRect::new(0, 0, 1, 17));
    let ours = edge_heights(&a, Side::PosX);
    assert_eq!(ours, edge_heights(&b, Side::NegX));
    assert!(ours.iter().all(|h| (h - 0.4).abs() < 1e-6), "{ours:?}");
    assert_eq!(a.paint()[a.index(16, 3)], b.paint()[b.index(0, 3)]);
    assert_eq!(a.paint()[a.index(16, 3)][1], 100);
    // Only the edge changes.
    assert_eq!(a.height(15, 3), ramp(17, true).height(15, 3));
    assert_eq!(b.height(1, 3), 0.1);
    assert!(
        stitch(
            &mut a,
            &mut TerrainData::flat(9, 9, 0.0).unwrap(),
            Side::PosX
        )
        .is_none()
    );
}

#[test]
fn blending_pins_the_new_edge_and_eases_back_to_its_own_ground() {
    let from = ramp(33, false);
    for side in Side::ALL {
        let mut new = TerrainData::flat(33, 33, 0.9).unwrap();
        let rect = blend_edge(&mut new, &from, side, 8).unwrap();
        assert_eq!(
            edge_heights(&new, side.opposite()),
            edge_heights(&from, side),
            "{side:?}"
        );
        assert_eq!(rect, side.opposite().band_rect((33, 33), 8));
        // Past the band it's its own ground; inside it's on the way there.
        let size = (33, 33);
        let (x, z) = side.opposite().sample(size, 10, 8);
        assert_eq!(new.height(x as isize, z as isize), 0.9, "{side:?}");
        let (x, z) = side.opposite().sample(size, 10, 4);
        let pinned = edge_heights(&from, side)[10];
        let halfway = new.height(x as isize, z as isize);
        assert!(
            (halfway - (pinned + 0.9) / 2.0).abs() < 1e-5,
            "{side:?} {halfway}"
        );
    }
    let mut odd = TerrainData::flat(17, 17, 0.0).unwrap();
    assert!(blend_edge(&mut odd, &from, Side::PosX, 8).is_none());
}

#[test]
fn edge_height_is_the_mean_of_the_edge() {
    let data = ramp(9, true);
    assert!((edge_height(&data, Side::NegX) - 0.2).abs() < 1e-6);
    assert!((edge_height(&data, Side::PosX) - 0.7).abs() < 1e-6);
    assert!((edge_height(&data, Side::PosZ) - 0.45).abs() < 1e-6);
}
