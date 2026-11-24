//! What's pressed into the snow: a thing prints its own shape where it
//! touches, as deep as it has sunk; a dragged thing leaves a trail; prints
//! stay behind the camera and fill in.

use axcenity_core::{ClassName, EnumItem, InstanceId, PartType, Value, Vector3, World};
use axcenity_render::Camera;
use axcenity_render::glam::Vec3;

use super::*;
use crate::particles::SurfaceHit;

/// Level ground at y = 0: a path down through it lands on it.
fn ground(_skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]) -> Vec<Option<SurfaceHit>> {
    paths
        .iter()
        .map(|(from, to)| {
            (from.y >= 0.0 && to.y <= 0.0).then(|| SurfaceHit {
                position: Vector3::new(from.x, 0.0, from.z),
                normal: Vector3::new(0.0, 1.0, 0.0),
            })
        })
        .collect()
}

fn set(world: &mut World, id: InstanceId, name: &str, value: Value) {
    world.set_property(id, name, value).unwrap();
}

fn vector(x: f32, y: f32, z: f32) -> Value {
    Value::Vector3(Vector3::new(x, y, z))
}

/// A world whose ground lies under `thickness` studs of old snow, with
/// nothing falling.
fn snowy_world(thickness: f64) -> (World, InstanceId) {
    let mut world = World::new();
    let weather = world.create(ClassName::Weather).unwrap();
    world.set_parent(weather, Some(world.lighting())).unwrap();
    let none = Value::EnumItem(EnumItem {
        enum_name: "Precipitation".to_owned(),
        item: "None".to_owned(),
    });
    set(&mut world, weather, "Precipitation", none);
    set(&mut world, weather, "SnowCover", Value::Number(1.0));
    set(
        &mut world,
        weather,
        "SnowThickness",
        Value::Number(thickness),
    );
    // Changes at once.
    set(&mut world, weather, "TransitionTime", Value::Number(0.0));
    (world, weather)
}

/// An unanchored part in the Workspace.
fn part(world: &mut World, shape: PartType, size: [f32; 3], at: [f32; 3]) -> InstanceId {
    let part = world.create(ClassName::Part).unwrap();
    set(world, part, "Shape", Value::PartType(shape));
    set(world, part, "Size", vector(size[0], size[1], size[2]));
    set(world, part, "Position", vector(at[0], at[1], at[2]));
    set(world, part, "Anchored", Value::Bool(false));
    world.set_parent(part, Some(world.workspace())).unwrap();
    part
}

/// A host's weather with its camera at `camera`, looking north.
fn state_at(camera: Vec3) -> WeatherState {
    let state = WeatherState::default();
    look_from(&state, camera);
    state
}

fn look_from(state: &WeatherState, camera: Vec3) {
    state.note_camera(Camera {
        position: camera,
        look_at: camera + Vec3::NEG_Z,
        fov_y_degrees: 50.0,
    });
}

fn update(state: &mut WeatherState, world: &World, seconds: f32) {
    state.update(world, seconds, &mut Random::default(), &mut ground);
}

/// The middles of the print map's cells within `reach` studs of (`x`, `z`).
fn cells_round(x: f32, z: f32, reach: f32) -> Vec<(f32, f32)> {
    let steps = (reach * 8.0).ceil() as i32;
    let middle = |at: f32, step: i32| ((at * 8.0).floor() + step as f32 + 0.5) / 8.0;
    let mut cells = Vec::new();
    for dz in -steps..=steps {
        for dx in -steps..=steps {
            let (cx, cz) = (middle(x, dx), middle(z, dz));
            if (cx - x).hypot(cz - z) <= reach {
                cells.push((cx, cz));
            }
        }
    }
    cells
}

#[test]
fn a_plank_in_the_snow_prints_its_own_turned_rectangle() {
    let (mut world, _) = snowy_world(0.5);
    // A long plank lying on the ground, turned 30 degrees.
    let (x, z) = (0.3, 0.2);
    let plank = part(&mut world, PartType::Block, [6.0, 0.5, 1.0], [x, 0.25, z]);
    set(&mut world, plank, "Orientation", vector(0.0, 30.0, 0.0));
    let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);

    let turn = rotation_from_orientation(Vector3::new(0.0, 30.0, 0.0));
    let (mut inside, mut beside) = (0, 0);
    for (cx, cz) in cells_round(x, z, 3.4) {
        // The cell in the plank's own space: 3 studs along it, half across.
        let own = turn.inverse() * Vec3::new(cx - x, 0.0, cz - z);
        let pressed = state.prints.pressed_at(cx, cz);
        if own.x.abs() < 2.8 && own.z.abs() < 0.3 {
            inside += 1;
            assert!(pressed > 0.99, "{pressed} inside it, at {own}");
        } else if own.x.abs() > 3.01 || own.z.abs() > 0.51 {
            assert_eq!(pressed, 0.0, "outside it, at {own}");
            // The old print was a disc round the part, 2.5 studs across.
            if (cx - x).hypot(cz - z) < 2.5 {
                beside += 1;
            }
        }
    }
    assert!(inside > 150, "{inside} cells in the plank's print");
    assert!(
        beside > 600,
        "{beside} cells the old disc would have pressed"
    );
}

#[test]
fn a_ball_prints_a_circle_as_wide_as_it_has_sunk() {
    // A stud of snow, and balls a stud in radius.
    let (mut world, _) = snowy_world(1.0);
    let ball = |world: &mut World, x: f32, y: f32| {
        part(world, PartType::Ball, [2.0; 3], [x, y, 0.0]);
    };
    // On the ground, 0.6 over the snow's surface with its middle, and clear of it.
    ball(&mut world, 0.0, 1.0);
    ball(&mut world, 10.0, 1.6);
    ball(&mut world, 20.0, 2.2);
    let mut state = state_at(Vec3::new(10.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    let at = |x: f32, away: f32| state.prints.pressed_at(x + away + 0.01, 0.01);

    // On the ground: pressed right through, as wide as the ball.
    assert!(at(0.0, 0.0) > 0.98, "{}", at(0.0, 0.0));
    assert!(at(0.0, 0.85) > 0.0);
    assert_eq!(at(0.0, 1.1), 0.0);
    // 0.4 into the snow: 0.4 deep, and a circle of radius 0.8.
    assert!((at(10.0, 0.0) - 0.4).abs() < 0.03, "{}", at(10.0, 0.0));
    assert!(at(10.0, 0.65) > 0.0);
    assert_eq!(at(10.0, 0.9), 0.0, "narrower than the ball");
    // A bowl: deepest in the middle.
    assert!(at(10.0, 0.0) > at(10.0, 0.3) && at(10.0, 0.3) > at(10.0, 0.6));
    // Clear of the snow: nothing.
    assert_eq!(at(20.0, 0.0), 0.0);
}

#[test]
fn a_thing_presses_as_deep_as_it_has_sunk() {
    let (mut world, _) = snowy_world(1.0);
    // One on the ground, one held half-way into the snow, one resting on it.
    part(&mut world, PartType::Block, [2.0; 3], [0.0, 1.0, 0.0]);
    part(&mut world, PartType::Block, [2.0; 3], [10.0, 1.5, 0.0]);
    part(&mut world, PartType::Block, [2.0; 3], [20.0, 2.0, 0.0]);
    let mut state = state_at(Vec3::new(10.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    let [through, half, on_top] = [0.0, 10.0, 20.0].map(|x| state.prints.pressed_at(x, 0.0));
    assert!(through > 0.99, "{through}");
    assert!((half - 0.5 * through).abs() < 0.02, "{half}");
    assert_eq!(on_top, 0.0, "a thing lying on the snow barely presses it");
}

#[test]
fn a_cylinder_prints_a_strip_lying_and_a_disc_standing() {
    let (mut world, _) = snowy_world(0.5);
    // Lying along x, 4 long and 1 across, on the ground: half sunk.
    part(
        &mut world,
        PartType::Cylinder,
        [4.0, 1.0, 1.0],
        [0.0, 0.5, 0.0],
    );
    // Standing on its end, 2 across.
    let standing = part(
        &mut world,
        PartType::Cylinder,
        [1.0, 2.0, 2.0],
        [10.0, 0.5, 0.0],
    );
    set(&mut world, standing, "Orientation", vector(0.0, 0.0, 90.0));
    let mut state = state_at(Vec3::new(5.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    let at = |x: f32, z: f32| state.prints.pressed_at(x + 0.01, z + 0.01);

    assert!(at(0.0, 0.0) > 0.98);
    assert!(at(1.8, 0.0) > 0.0, "all along it");
    assert_eq!(at(2.1, 0.0), 0.0);
    assert!(at(0.0, 0.4) > 0.0 && at(0.0, 0.4) < at(0.0, 0.0), "round");
    assert_eq!(at(0.0, 0.6), 0.0, "as wide as it");

    assert!(at(10.0, 0.0) > 0.98 && at(10.8, 0.0) > 0.98, "a flat floor");
    assert_eq!(at(10.8, 0.8), 0.0, "round: its box's corner isn't pressed");
}

#[test]
fn a_dragged_part_leaves_an_unbroken_trail_of_its_width() {
    let (mut world, _) = snowy_world(0.5);
    let crate_ = part(&mut world, PartType::Block, [1.0; 3], [-5.0, 0.5, 0.3]);
    let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    // Dragged east a stud and a half an update: further than it is wide.
    let mut x = -5.0;
    while x < 5.5 {
        x += 1.5;
        set(&mut world, crate_, "Position", vector(x, 0.5, 0.3));
        update(&mut state, &world, 0.016);
    }
    let mut along = -5.0;
    while along <= x {
        let pressed = state.prints.pressed_at(along, 0.3);
        assert!(pressed > 0.98, "{pressed} in the trail at x = {along}");
        // As wide as the crate, and no wider.
        assert!(state.prints.pressed_at(along, 0.0) > 0.9);
        assert!(state.prints.pressed_at(along, 0.6) > 0.9);
        assert_eq!(state.prints.pressed_at(along, 0.3 + 0.57), 0.0);
        assert_eq!(state.prints.pressed_at(along, 0.3 - 0.57), 0.0);
        along += 0.125;
    }
    // Kept as a few long outlines, not one an update.
    assert!(
        (2..=6).contains(&state.prints.kept()),
        "{}",
        state.prints.kept()
    );

    // A part that's put somewhere else leaves no trail between.
    set(&mut world, crate_, "Position", vector(x, 0.5, 20.3));
    update(&mut state, &world, 0.016);
    assert!(state.prints.pressed_at(x, 20.3) > 0.98);
    assert_eq!(state.prints.pressed_at(x, 10.0), 0.0);
}

#[test]
fn prints_stay_when_the_camera_leaves_and_fill_in_over_the_fade_time() {
    let (mut world, weather) = snowy_world(0.5);
    set(
        &mut world,
        weather,
        "FootprintFadeTime",
        Value::Number(10.0),
    );
    let crate_ = part(&mut world, PartType::Block, [2.0; 3], [0.0, 1.0, 0.0]);
    let home = Vec3::new(0.0, 5.0, 0.0);
    let mut state = state_at(home);
    update(&mut state, &world, 0.016);
    let pressed = |state: &WeatherState| state.prints.pressed_at(0.0, 0.0);
    assert!(pressed(&state) > 0.99);

    // While the crate lies there its print stays whole, and nothing is
    // filling in, so a host has nothing to draw for it.
    for _ in 0..40 {
        update(&mut state, &world, 0.5);
    }
    assert!(pressed(&state) > 0.9, "{}", pressed(&state));
    assert!(!state.is_active(&world));

    // Lifted out, its print is left, and fills in at a tenth a second.
    set(&mut world, crate_, "Position", vector(0.0, 50.0, 0.0));
    update(&mut state, &world, 0.0);
    let left = pressed(&state);
    assert!(left > 0.9);
    update(&mut state, &world, 2.5);
    assert!(
        (pressed(&state) - (left - 0.25)).abs() < 0.01,
        "{}",
        pressed(&state)
    );
    assert!(
        state.is_active(&world),
        "a host keeps drawing while prints fill in"
    );

    // The camera goes far away and comes back: the print is as it was.
    look_from(&state, Vec3::new(400.0, 5.0, -300.0));
    update(&mut state, &world, 0.0);
    assert_eq!(pressed(&state), 0.0, "off the map");
    look_from(&state, home);
    update(&mut state, &world, 0.0);
    assert!(
        (pressed(&state) - (left - 0.25)).abs() < 0.01,
        "{}",
        pressed(&state)
    );
    // And a little way, so the map only moves.
    look_from(&state, home + Vec3::new(20.0, 0.0, 9.0));
    update(&mut state, &world, 0.0);
    assert!(
        (pressed(&state) - (left - 0.25)).abs() < 0.01,
        "{}",
        pressed(&state)
    );

    // Falling snow fills it two and a half times as fast.
    set(&mut world, weather, "Precipitation", {
        Value::EnumItem(EnumItem {
            enum_name: "Precipitation".to_owned(),
            item: "Snow".to_owned(),
        })
    });
    set(&mut world, weather, "Intensity", Value::Number(1.0));
    update(&mut state, &world, 0.0);
    update(&mut state, &world, 1.0);
    assert!(
        (pressed(&state) - (left - 0.5)).abs() < 0.01,
        "{}",
        pressed(&state)
    );
    update(&mut state, &world, 2.1);
    assert_eq!(pressed(&state), 0.0, "filled in");
    assert_eq!(state.prints.kept(), 0);
    assert!(state.field().unwrap().prints.is_none(), "no map to draw");
}

/// A character whose skinned body has foot sockets, 5 studs tall, standing
/// at the origin facing -Z. Returns its model and its left and right
/// sockets.
fn posed_character(world: &mut World) -> (InstanceId, [InstanceId; 2]) {
    let model = world.create(ClassName::Model).unwrap();
    let humanoid = world.create(ClassName::Humanoid).unwrap();
    world.set_parent(humanoid, Some(model)).unwrap();
    let root = world.create(ClassName::Part).unwrap();
    world.set_name(root, "HumanoidRootPart").unwrap();
    set(world, root, "Size", vector(1.7, 5.0, 1.7));
    set(world, root, "Position", vector(0.0, 2.5, 0.0));
    world.set_parent(root, Some(model)).unwrap();
    let body = world.create(ClassName::MeshPart).unwrap();
    set(world, body, "Size", vector(2.0, 5.0, 1.0));
    set(world, body, "Position", vector(0.0, 2.5, 0.0));
    // As the default humanoid's is: a glTF character faces +Z.
    set(world, body, "Orientation", vector(0.0, 180.0, 0.0));
    world.set_parent(body, Some(model)).unwrap();
    let sockets = ["LeftFoot", "RightFoot"].map(|bone| {
        let socket = world.create(ClassName::Attachment).unwrap();
        world
            .set_attribute(socket, "Bone", Some(Value::String(bone.to_owned())))
            .unwrap();
        world.set_parent(socket, Some(body)).unwrap();
        socket
    });
    world.set_parent(model, Some(world.workspace())).unwrap();
    (model, sockets)
}

/// Puts a foot socket of [`posed_character`] at a world place, turned
/// `yaw` degrees from facing -Z.
fn place_foot(world: &mut World, socket: InstanceId, at: [f32; 3], yaw: f32) {
    // In the body's space: the body is at (0, 2.5, 0), turned half round.
    set(
        world,
        socket,
        "Position",
        vector(-at[0], at[1] - 2.5, -at[2]),
    );
    set(world, socket, "Orientation", vector(0.0, 180.0 + yaw, 0.0));
}

#[test]
fn a_posed_characters_feet_print_where_they_are_down() {
    let (mut world, _) = snowy_world(0.5);
    let (_, [left, right]) = posed_character(&mut world);
    // The left foot on the ground (its ankle a quarter stud up), the right
    // one lifted a stud.
    place_foot(&mut world, left, [-0.4, 0.25, 0.0], 0.0);
    place_foot(&mut world, right, [0.4, 1.25, 0.3], 0.0);
    let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    let at = |x: f32, z: f32| state.prints.pressed_at(x, z);

    // A foot three quarters of a stud long, most of it ahead of the ankle
    // (-Z), and a third wide.
    assert!(at(-0.4, -0.17) > 0.8, "{}", at(-0.4, -0.17));
    assert!(at(-0.4, -0.45) > 0.0, "its toes");
    assert!(at(-0.4, 0.12) > 0.0, "its heel");
    assert_eq!(at(-0.4, -0.65), 0.0, "no longer than a foot");
    assert_eq!(at(-0.4, 0.3), 0.0);
    assert_eq!(at(-0.4 + 0.3, -0.17), 0.0, "no wider than a foot");
    assert_eq!(at(-0.4 - 0.3, -0.17), 0.0);
    // The lifted foot presses nothing.
    for z in [-0.4, -0.2, 0.0, 0.2, 0.4] {
        assert_eq!(at(0.4, z), 0.0);
    }

    // It comes down a stride ahead, turned a quarter round to the left
    // (toward -X): its print lies that way.
    place_foot(&mut world, right, [3.0, 0.25, -3.0], 90.0);
    update(&mut state, &world, 0.016);
    let at = |x: f32, z: f32| state.prints.pressed_at(x, z);
    assert!(at(3.0 - 0.17, -3.0) > 0.8);
    assert!(at(3.0 - 0.45, -3.0) > 0.0, "its toes, toward -X");
    assert_eq!(at(3.0 - 0.17, -3.0 + 0.3), 0.0, "narrow across");
    assert_eq!(at(3.0 - 0.17, -3.0 - 0.3), 0.0);
    // The left foot's print is still there, and nothing lies between them.
    assert!(at(-0.4, -0.17) > 0.8);
    assert_eq!(at(1.3, -1.5), 0.0);

    // Lifted again, the print stays behind.
    place_foot(&mut world, right, [3.0, 1.5, -3.0], 90.0);
    update(&mut state, &world, 0.016);
    assert!(state.prints.pressed_at(3.0 - 0.17, -3.0) > 0.8);
}

#[test]
fn a_block_characters_steps_print_at_its_legs_in_turn() {
    let (mut world, _) = snowy_world(0.5);
    // Insert > Character's block rig: its legs are parts that follow it.
    let model = world.create(ClassName::Model).unwrap();
    let humanoid = world.create(ClassName::Humanoid).unwrap();
    world.set_parent(humanoid, Some(model)).unwrap();
    let limb = |world: &mut World, name: &str, size: [f32; 3], at: [f32; 3]| {
        let limb = world.create(ClassName::Part).unwrap();
        world.set_name(limb, name).unwrap();
        set(world, limb, "Size", vector(size[0], size[1], size[2]));
        set(world, limb, "Position", vector(at[0], at[1], at[2]));
        world.set_parent(limb, Some(model)).unwrap();
        limb
    };
    let torso = limb(&mut world, "Torso", [2.0, 2.0, 1.0], [0.0, 3.0, 0.0]);
    let left = limb(&mut world, "Left Leg", [1.0, 2.0, 1.0], [-0.5, 1.0, 0.0]);
    let right = limb(&mut world, "Right Leg", [1.0, 2.0, 1.0], [0.5, 1.0, 0.0]);
    world.set_parent(model, Some(world.workspace())).unwrap();
    let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);

    // Standing: a print under each leg, and none between them.
    assert!(state.prints.pressed_at(-0.5, 0.0) > 0.99);
    assert!(state.prints.pressed_at(0.5, 0.0) > 0.99);
    assert_eq!(state.prints.pressed_at(0.0, 0.0), 0.0);
    assert_eq!(state.prints.kept(), 2);

    // Walking north, it steps left and right in turn.
    let mut z = 0.0;
    for _ in 0..40 {
        z -= 0.25;
        for (part, x, y) in [(torso, 0.0, 3.0), (left, -0.5, 1.0), (right, 0.5, 1.0)] {
            set(&mut world, part, "Position", vector(x, y, z));
        }
        update(&mut state, &world, 0.016);
    }
    let steps = state.prints.kept() - 2;
    assert!((7..=9).contains(&steps), "{steps} steps in 10 studs");
    let (mut lefts, mut rights) = (0, 0);
    let mut at = -2.0;
    while at > -10.0 {
        let left = state.prints.pressed_at(-0.5, at) > 0.5;
        let right = state.prints.pressed_at(0.5, at) > 0.5;
        lefts += u32::from(left && !right);
        rights += u32::from(right && !left);
        at -= 0.125;
    }
    assert!(
        lefts > 10 && rights > 10,
        "{lefts} left only, {rights} right only"
    );
}

#[test]
fn the_field_holds_the_average_and_the_map_changes_only_when_something_presses() {
    let (mut world, _) = snowy_world(0.5);
    let slab = part(
        &mut world,
        PartType::Block,
        [4.0, 1.0, 4.0],
        [0.5, 0.5, 0.0],
    );
    let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    let field = state.field().unwrap();
    let cell = |field: &WeatherField, x: f32, z: f32| {
        let (gx, gz) = (
            (x - field.origin[0]) as usize,
            (z - field.origin[1]) as usize,
        );
        field.cells[gz * field.size as usize + gx][2]
    };
    // A stud wholly under the slab is pressed through; one its edge
    // crosses, partly; open snow, not at all.
    assert!(cell(&field, 0.5, 0.5) > 0.99);
    assert!(
        (0.2..0.8).contains(&cell(&field, 2.5, 0.5)),
        "{}",
        cell(&field, 2.5, 0.5)
    );
    assert_eq!(cell(&field, 6.5, 0.5), 0.0);

    let prints = field.prints.as_ref().expect("the fine map");
    assert!(prints.cells.is_valid());
    let (key, version) = (prints.cells.key, prints.cells.version);
    // Time passes: the prints fill in, and the cells stay as they are.
    update(&mut state, &world, 0.5);
    let later = state.field().unwrap();
    let later = later.prints.as_ref().unwrap();
    assert!(later.fill > prints.fill);
    assert_eq!((later.cells.key, later.cells.version), (key, version));
    assert!(std::sync::Arc::ptr_eq(&later.cells, &prints.cells));

    // The slab is pushed a little: only the rows it covers change.
    set(&mut world, slab, "Position", vector(1.0, 0.5, 0.0));
    update(&mut state, &world, 0.016);
    let moved = state.field().unwrap();
    let moved = moved.prints.as_ref().unwrap();
    assert_eq!(moved.cells.key, key);
    assert!(moved.cells.version > version);
    let rows = moved
        .cells
        .row_versions
        .iter()
        .filter(|&&row| row > version)
        .count();
    assert!((1..=48).contains(&rows), "{rows} rows changed");
}

#[test]
fn nothing_presses_with_footprints_off_under_shelter_or_standing_anchored() {
    let (mut world, weather) = snowy_world(0.5);
    let crate_ = part(&mut world, PartType::Block, [2.0; 3], [0.0, 1.0, 0.0]);
    // A wall that stands in the snow, and a gate a script slides.
    let wall = part(&mut world, PartType::Block, [2.0; 3], [10.0, 1.0, 0.0]);
    let gate = part(&mut world, PartType::Block, [2.0; 3], [0.0, 1.0, 10.0]);
    for anchored in [wall, gate] {
        set(&mut world, anchored, "Anchored", Value::Bool(true));
    }
    // And a crate nothing collides with.
    let ghost = part(&mut world, PartType::Block, [2.0; 3], [-10.0, 1.0, 0.0]);
    set(&mut world, ghost, "CanCollide", Value::Bool(false));

    set(&mut world, weather, "FootprintsEnabled", Value::Bool(false));
    let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
    update(&mut state, &world, 0.016);
    assert_eq!(state.prints.pressed_at(0.0, 0.0), 0.0);
    assert!(state.field().unwrap().prints.is_none());

    set(&mut world, weather, "FootprintsEnabled", Value::Bool(true));
    update(&mut state, &world, 0.016);
    assert!(state.prints.pressed_at(0.0, 0.0) > 0.99, "the crate");
    assert_eq!(state.prints.pressed_at(10.0, 0.0), 0.0, "the wall");
    assert_eq!(state.prints.pressed_at(0.0, 10.0), 0.0, "the gate, shut");
    assert_eq!(state.prints.pressed_at(-10.0, 0.0), 0.0, "the ghost");

    // The gate slides: it presses where it goes.
    set(&mut world, gate, "Position", vector(3.0, 1.0, 10.0));
    update(&mut state, &world, 0.016);
    assert!(state.prints.pressed_at(3.0, 10.0) > 0.99);

    // A WeatherVolume that keeps the snow out: nothing to press there.
    let volume = world.create(ClassName::WeatherVolume).unwrap();
    world.set_parent(volume, Some(world.workspace())).unwrap();
    set(&mut world, volume, "Position", vector(20.0, 5.0, 0.0));
    set(&mut world, volume, "Size", vector(10.0, 10.0, 10.0));
    set(&mut world, crate_, "Position", vector(20.0, 1.0, 0.0));
    update(&mut state, &world, 0.016);
    assert_eq!(state.prints.pressed_at(20.0, 0.0), 0.0);

    // No snow, no prints, and nothing kept.
    set(&mut world, weather, "SnowCover", Value::Number(0.0));
    update(&mut state, &world, 0.016);
    assert_eq!(state.prints.pressed_at(0.0, 0.0), 0.0);
    assert_eq!(state.prints.kept(), 0);
}

#[test]
fn what_stands_in_the_snow_is_no_roof_to_the_ground_under_it() {
    // A tall crate on level ground, as physics would answer: a ray down
    // lands on its top unless it's the thing passed by.
    let crate_field = |footprints: bool| {
        let (mut world, weather) = snowy_world(0.5);
        set(
            &mut world,
            weather,
            "FootprintsEnabled",
            Value::Bool(footprints),
        );
        let tall = part(
            &mut world,
            PartType::Block,
            [2.0, 5.0, 2.0],
            [1.0, 2.5, 1.0],
        );
        let mut surfaces = |skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            paths
                .iter()
                .map(|(from, to)| {
                    let over = (0.0..2.0).contains(&from.x) && (0.0..2.0).contains(&from.z);
                    let top = if over && skip != Some(tall) { 5.0 } else { 0.0 };
                    (from.y >= top && to.y <= top).then(|| SurfaceHit {
                        position: Vector3::new(from.x, top, from.z),
                        normal: Vector3::new(0.0, 1.0, 0.0),
                    })
                })
                .collect()
        };
        let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
        // Long enough for the field to have looked at every cell twice.
        for _ in 0..80 {
            state.update(&world, 0.016, &mut Random::default(), &mut surfaces);
        }
        let field = state.field().unwrap();
        let cell = |x: f32, z: f32| {
            let (gx, gz) = (
                (x - field.origin[0]) as usize,
                (z - field.origin[1]) as usize,
            );
            field.cells[gz * field.size as usize + gx]
        };
        (cell(0.5, 0.5), cell(1.5, 1.5), cell(5.5, 0.5))
    };
    // The ground under it is open to the sky, with no snow banked against it.
    let (under, also_under, beside) = crate_field(true);
    assert_eq!((under[0], also_under[0], beside[0]), (0.0, 0.0, 0.0));
    assert_eq!((under[3], beside[3]), (0.0, 0.0));
    assert!(under[2] > 0.6, "and its print is in the snow there");
    // (With footprints off nothing is tracked, and it's a roof as before.)
    let (under, _, beside) = crate_field(false);
    assert_eq!((under[0], beside[0]), (5.0, 0.0));
}

/// Run with `cargo test -p axcenity-scene prints_cost -- --ignored --nocapture`:
/// the time an update takes with 20 parts dragged through the snow and 4
/// characters walking, against the same world with footprints off, and how
/// much of the print map an update changes.
#[test]
#[ignore = "a measurement, not a check"]
fn prints_cost() {
    let run = |footprints: bool| {
        let (mut world, weather) = snowy_world(0.5);
        set(
            &mut world,
            weather,
            "FootprintsEnabled",
            Value::Bool(footprints),
        );
        let parts: Vec<InstanceId> = (0..20)
            .map(|i| {
                let shape = [PartType::Block, PartType::Ball, PartType::Cylinder][i % 3];
                let at = [i as f32 * 2.0 - 20.0, 0.5, -10.0];
                part(&mut world, shape, [1.0; 3], at)
            })
            .collect();
        let walkers: Vec<[InstanceId; 2]> = (0..4).map(|_| posed_character(&mut world).1).collect();
        let mut state = state_at(Vec3::new(0.0, 5.0, 0.0));
        update(&mut state, &world, 0.016);
        let (frames, mut rows, mut version) = (600, 0usize, 0);
        let mut spent = std::time::Duration::ZERO;
        // The queries for the ground under things (the ones that pass a
        // thing by: the sky's rays pass nothing by).
        let mut queries = 0;
        let mut counted = |skip: Option<InstanceId>, paths: &[(Vector3, Vector3)]| {
            queries += usize::from(skip.is_some());
            ground(skip, paths)
        };
        for frame in 0..frames {
            let t = frame as f32 / 60.0;
            for (i, &part) in parts.iter().enumerate() {
                // 4 studs a second, each on its own line, turning as it goes.
                let x = i as f32 * 2.0 - 20.0;
                set(
                    &mut world,
                    part,
                    "Position",
                    vector(x, 0.5, -10.0 + 4.0 * t),
                );
                set(&mut world, part, "Orientation", vector(0.0, 20.0 * t, 0.0));
            }
            for (i, feet) in walkers.iter().enumerate() {
                // Each foot down for half of a one-second stride.
                for (side, &socket) in feet.iter().enumerate() {
                    let phase = (t + side as f32 * 0.5).fract();
                    let lift = if phase < 0.5 { 0.0 } else { 0.8 };
                    let x = i as f32 * 3.0 - 4.5 + side as f32 * 0.8;
                    let z = 5.0 - 3.0 * (t + side as f32 * 0.5).floor() - 3.0 * phase.max(0.5);
                    place_foot(&mut world, socket, [x, 0.25 + lift, z], 0.0);
                }
            }
            let started = std::time::Instant::now();
            state.update(&world, 1.0 / 60.0, &mut Random::default(), &mut counted);
            spent += started.elapsed();
            if let Some(prints) = state.field().and_then(|field| field.prints.clone()) {
                rows += prints
                    .cells
                    .row_versions
                    .iter()
                    .filter(|&&row| row > version)
                    .count();
                version = prints.cells.version;
            }
        }
        (
            spent.as_secs_f64() * 1e6 / frames as f64,
            rows as f64 / frames as f64,
            state.prints.kept(),
            queries as f64 / frames as f64,
        )
    };
    let (off, ..) = run(false);
    let (on, rows, kept, queries) = run(true);
    println!("ground queries an update: {queries:.1}");
    let row_bytes = axcenity_render::WEATHER_PRINT_SIZE as f64 * 4.0;
    println!(
        "an update: {on:.0} us with prints, {off:.0} us with footprints off ({:.0} us for the prints)",
        on - off
    );
    println!(
        "uploaded an update: {rows:.0} of {} rows, {:.0} KiB (the whole map is {:.0} KiB); {kept} outlines kept",
        axcenity_render::WEATHER_PRINT_SIZE,
        rows * row_bytes / 1024.0,
        row_bytes * axcenity_render::WEATHER_PRINT_SIZE as f64 / 1024.0
    );
}
