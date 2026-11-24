//! Weather in a world: rain wets the ground over time in the frames a host
//! draws and it dries after, a script sets the weather, and it saves with
//! the world.

use axcenity_core::{ClassName, EnumItem, InstanceId, Value, Vector3};
use axcenity_render::TextureRenderer;
use axcenity_scene::Particles;

use super::tests::{run_until, start_example_in, test_gpu};
use super::*;

/// A wide grey floor under a Weather that doesn't rain yet, eases over
/// `transition` seconds and rains as hard as it can when it does.
fn floor_under_weather(transition: f64) -> (World, InstanceId) {
    let mut world = World::new();
    let floor = world.create(ClassName::Part).unwrap();
    world.set_parent(floor, Some(world.workspace())).unwrap();
    for (name, value) in [
        ("Size", Value::Vector3(Vector3::new(400.0, 1.0, 400.0))),
        ("Position", Value::Vector3(Vector3::new(0.0, -0.5, 0.0))),
        ("Anchored", Value::Bool(true)),
    ] {
        world.set_property(floor, name, value).unwrap();
    }
    let weather = world.create(ClassName::Weather).unwrap();
    world.set_parent(weather, Some(world.lighting())).unwrap();
    for (name, value) in [
        ("Precipitation", falling("None")),
        ("Intensity", Value::Number(1.0)),
        ("TransitionTime", Value::Number(transition)),
    ] {
        world.set_property(weather, name, value).unwrap();
    }
    (world, weather)
}

fn falling(item: &str) -> Value {
    Value::EnumItem(EnumItem {
        enum_name: "Precipitation".to_owned(),
        item: item.to_owned(),
    })
}

/// The middle brightness of the frame (the floor's, whatever raindrops
/// cross it), as a host draws it after moving its particles on by
/// `seconds`.
fn floor_brightness(
    renderer: &mut TextureRenderer,
    particles: &mut Particles,
    world: &World,
    seconds: f32,
) -> u32 {
    let folder = std::env::temp_dir().join(format!("axcenity-weather-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let assets = Assets::new(&folder);
    // Particles move at most a quarter of a second a frame.
    let mut left = seconds;
    while left > 0.0 {
        particles.update(world, left.min(0.25));
        left -= 0.25;
    }
    let camera = FlyCamera::looking_at(Vec3::new(0.0, 20.0, 12.0), Vec3::ZERO);
    let mut list = axcenity_scene::build_draw_list(world, &assets, camera.to_render_camera());
    list.sprites = particles.sprites(world, &assets);
    particles.add_weather(&mut list);
    renderer.render(&list).unwrap();
    let mut brightness: Vec<u32> = renderer
        .read_pixels()
        .chunks(4)
        .map(|pixel| pixel[..3].iter().map(|&c| u32::from(c)).sum())
        .collect();
    brightness.sort_unstable();
    brightness[brightness.len() / 2]
}

#[test]
fn rain_wets_the_ground_over_time_and_it_dries_after() {
    let Some((device, queue)) = test_gpu() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 160, 90);
    let (mut world, weather) = floor_under_weather(2.0);
    let mut particles = Particles::new();
    let dry = floor_brightness(&mut renderer, &mut particles, &world, 0.25);
    assert!(
        !particles.is_active(&world),
        "dry and settled: no frames needed"
    );

    world
        .set_property(weather, "Precipitation", falling("Rain"))
        .unwrap();
    let mut wetting = Vec::new();
    for _ in 0..3 {
        wetting.push(floor_brightness(
            &mut renderer,
            &mut particles,
            &world,
            0.75,
        ));
    }
    assert!(particles.is_active(&world), "it's raining");
    assert!(
        dry > wetting[0] && wetting[0] > wetting[1] && wetting[1] >= wetting[2],
        "darker as it wets: {dry} dry, then {wetting:?}"
    );

    world
        .set_property(weather, "Precipitation", falling("None"))
        .unwrap();
    let drying = floor_brightness(&mut renderer, &mut particles, &world, 1.0);
    let dried = floor_brightness(&mut renderer, &mut particles, &world, 1.5);
    assert!(
        wetting[2] < drying && drying < dried,
        "lighter as it dries: {} soaked, {drying}, {dried}",
        wetting[2]
    );
    assert_eq!(dried, dry, "dry again");
    assert!(!particles.is_active(&world));
}

#[test]
fn a_script_sets_the_weather_and_it_saves_with_the_world() {
    let (mut world, mut scripts, folder) = start_example_in("part");
    let (mut physics, mut clock) = (Physics::new(), 0);
    assert!(
        axcenity_scene::build_draw_list(
            &world,
            &Assets::new(&folder),
            FlyCamera::looking_at(Vec3::new(0.0, 10.0, 10.0), Vec3::ZERO).to_render_camera()
        )
        .weather
        .is_clear(),
        "no Weather, no weather"
    );

    let script = world.create(ClassName::Script).unwrap();
    world
        .set_parent(script, Some(world.server_script_service()))
        .unwrap();
    scripts.start_script(
        &mut world,
        script,
        "scripts/snow.server.lua",
        "local weather = Instance.new(\"Weather\")\n\
         weather.Precipitation = Enum.Precipitation.Snow\n\
         weather.Intensity = 0.25\n\
         weather.SnowCover = 0.75\n\
         weather.Parent = game.Lighting\n\
         print(weather.Precipitation == Enum.Precipitation.Snow)",
    );
    run_until(&mut world, &mut scripts, &mut physics, &mut clock, 0.1);
    let printed: Vec<String> = scripts
        .take_output()
        .into_iter()
        .map(|line| line.text)
        .collect();
    assert!(printed.contains(&"true".to_owned()), "{printed:?}");

    let drawn = |world: &World| {
        axcenity_scene::build_draw_list(
            world,
            &Assets::new(&folder),
            FlyCamera::looking_at(Vec3::new(0.0, 10.0, 10.0), Vec3::ZERO).to_render_camera(),
        )
        .weather
    };
    let snowy = axcenity_render::Weather {
        wetness: 0.0,
        snow: 0.75,
        snow_thickness: axcenity_render::DEFAULT_SNOW_THICKNESS,
        rain: 0.0,
        snowfall: 0.25,
        lightning: 0.0,
    };
    assert_eq!(drawn(&world), snowy);

    axcenity_project::save_world(&folder, &world).unwrap();
    let reloaded = axcenity_project::load(&folder).unwrap().world;
    let weather = reloaded
        .find_first_child(reloaded.lighting(), "Weather")
        .and_then(|id| reloaded.get(id))
        .expect("the Weather saved");
    assert_eq!(
        weather.property("Precipitation"),
        Some(&falling("Snow")),
        "its Precipitation saved"
    );
    assert_eq!(drawn(&reloaded), snowy, "and draws the same");
}

/// Moves `particles` on by a sixtieth of a second against the world's real
/// surfaces (physics' `surface_hits`, as a host passes them).
fn step_against_the_world(particles: &mut Particles, world: &World, assets: &Assets) {
    particles.update_against(world, 1.0 / 60.0, &mut |skip, paths| {
        axcenity_physics::surface_hits(world, assets, skip, paths)
            .into_iter()
            .map(|hit| {
                hit.map(|hit| axcenity_scene::SurfaceHit {
                    position: hit.position,
                    normal: hit.normal,
                })
            })
            .collect()
    });
}

/// [`floor_under_weather`] under half a stud of old snow, and an unanchored
/// crate (2 studs a side) standing on the floor at the origin.
fn crate_in_the_snow() -> (World, InstanceId, InstanceId) {
    let (mut world, weather) = floor_under_weather(0.0);
    for (name, value) in [("SnowCover", 1.0), ("SnowThickness", 0.5)] {
        world
            .set_property(weather, name, Value::Number(value))
            .unwrap();
    }
    let crate_ = world.create(ClassName::Part).unwrap();
    world.set_parent(crate_, Some(world.workspace())).unwrap();
    for (name, value) in [
        ("Size", Value::Vector3(Vector3::new(2.0, 2.0, 2.0))),
        ("Position", Value::Vector3(Vector3::new(0.0, 1.0, 0.0))),
        ("Anchored", Value::Bool(false)),
    ] {
        world.set_property(crate_, name, value).unwrap();
    }
    (world, weather, crate_)
}

#[test]
fn a_crate_leaves_its_own_print_in_the_snow_on_a_floor() {
    let Some((device, queue)) = test_gpu() else {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    };
    let mut renderer = TextureRenderer::new(&device, &queue, 160, 90);
    let (mut world, _, crate_) = crate_in_the_snow();
    let folder = std::env::temp_dir().join(format!("axcenity-prints-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let assets = Assets::new(&folder);
    let mut particles = Particles::new();
    // Looking straight down on where the crate stands, from 9 studs up.
    let camera = FlyCamera::looking_at(Vec3::new(0.0, 9.0, 0.3), Vec3::ZERO);
    let mut draw = |world: &World, particles: &mut Particles| {
        let mut list = axcenity_scene::build_draw_list(world, &assets, camera.to_render_camera());
        particles.add_weather(&mut list);
        renderer.render(&list).unwrap();
        (list, renderer.read_pixels())
    };
    // The host draws (which tells the weather where the camera is), then
    // moves its particles on.
    draw(&world, &mut particles);
    step_against_the_world(&mut particles, &world, &assets);
    let (list, _) = draw(&world, &mut particles);
    let prints = list.weather_field.as_ref().and_then(|f| f.prints.as_ref());
    assert!(prints.is_some(), "the crate presses the snow it stands in");

    // The crate is put far away, and its print stays. (A few more updates,
    // as a host runs.)
    world
        .set_property(
            crate_,
            "Position",
            Value::Vector3(Vector3::new(60.0, 1.0, 0.0)),
        )
        .unwrap();
    for _ in 0..40 {
        step_against_the_world(&mut particles, &world, &assets);
    }
    let (_, pixels) = draw(&world, &mut particles);
    let brightness = |x: i32, y: i32| -> u32 {
        pixels[(y as usize * 160 + x as usize) * 4..][..3]
            .iter()
            .map(|&c| u32::from(c))
            .sum()
    };
    // A stud is about 11.5 pixels. The print's floor is the middle of the
    // frame, and the snow 3 studs east and west of it is untrodden.
    let stud = 11.5;
    let untrodden = brightness(80 - 34, 45).min(brightness(80 + 34, 45));
    let floor = brightness(80, 45);
    assert!(
        floor + 12 < untrodden,
        "the print's packed floor is darker: {floor} against {untrodden}"
    );
    // It's the crate's square, a stud each way from its middle: packed 0.6
    // of a stud out and untrodden 1.5 out, along x and along z.
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let at = |studs: f32| {
            let pixels = (studs * stud) as i32;
            brightness(80 + dx * pixels, 45 + dy * pixels)
        };
        assert!(at(0.6) + 8 < untrodden, "inside, {dx} {dy}: {}", at(0.6));
        assert!(at(1.5) + 25 > untrodden, "outside, {dx} {dy}: {}", at(1.5));
    }
    // And a hollow: the wall away from the sun, a stud east of the middle,
    // is in its own shade.
    let wall = (88..=94).map(|x| brightness(x, 45)).min().unwrap();
    assert!(
        wall + 60 < untrodden,
        "its east wall: {wall} against {untrodden}"
    );
}

/// Run with `cargo test -p axcenity-player prints_cost_against_the_world --
/// --ignored --nocapture`: a host's particle update in a world of 2,000
/// anchored parts with 20 crates dragged through the snow, the ground under
/// them found by physics, against the same with footprints off.
#[test]
#[ignore = "a measurement, not a check"]
fn prints_cost_against_the_world() {
    let run = |footprints: bool| {
        let (mut world, weather, first) = crate_in_the_snow();
        world
            .set_property(weather, "FootprintsEnabled", Value::Bool(footprints))
            .unwrap();
        let folder = std::env::temp_dir().join(format!("axcenity-prints-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let assets = Assets::new(&folder);
        let mut crates = vec![first];
        for index in 0..2019 {
            let part = world.create(ClassName::Part).unwrap();
            world.set_parent(part, Some(world.workspace())).unwrap();
            // 19 more crates to drag, and a town of anchored blocks.
            let anchored = index >= 19;
            let at = if anchored {
                Vector3::new(
                    (index % 50) as f32 * 6.0 - 150.0,
                    2.0,
                    (index / 50) as f32 * 6.0 + 40.0,
                )
            } else {
                Vector3::new(index as f32 * 2.0 - 20.0, 0.5, 0.0)
            };
            for (name, value) in [
                ("Size", Value::Vector3(Vector3::new(1.0, 1.0, 1.0))),
                ("Position", Value::Vector3(at)),
                ("Anchored", Value::Bool(anchored)),
            ] {
                world.set_property(part, name, value).unwrap();
            }
            if !anchored {
                crates.push(part);
            }
        }
        let mut particles = Particles::new();
        let camera = FlyCamera::looking_at(Vec3::new(0.0, 9.0, 0.3), Vec3::ZERO);
        let mut list = axcenity_scene::build_draw_list(&world, &assets, camera.to_render_camera());
        particles.add_weather(&mut list);
        step_against_the_world(&mut particles, &world, &assets);
        let frames = 300;
        let mut spent = std::time::Duration::ZERO;
        for frame in 0..frames {
            for (index, &part) in crates.iter().enumerate() {
                let at = Vector3::new(
                    index as f32 * 2.0 - 20.0,
                    if index == 0 { 1.0 } else { 0.5 },
                    -10.0 + frame as f32 / 15.0,
                );
                world
                    .set_property(part, "Position", Value::Vector3(at))
                    .unwrap();
            }
            let started = std::time::Instant::now();
            step_against_the_world(&mut particles, &world, &assets);
            spent += started.elapsed();
        }
        spent.as_secs_f64() * 1e6 / frames as f64
    };
    let (off, on) = (run(false), run(true));
    println!(
        "an update against 2,020 parts: {on:.0} us with prints, {off:.0} us with footprints off ({:.0} us for the prints)",
        on - off
    );
}
