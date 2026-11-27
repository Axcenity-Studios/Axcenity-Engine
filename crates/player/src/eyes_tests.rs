//! A third-person camera zoomed all the way in sees from the character's
//! eyes (`Controllers::place_eyes`): the posed eye bones of the default
//! character, which the head carries as it animates, and from there the
//! head itself is not in the way of the view.

use axcenity_anim::Animators;
use axcenity_character::{Controllers, MoveInput};
use axcenity_core::{ClassName, Value, World};
use axcenity_render::glam::Vec3;
use axcenity_render::{Camera, GraphicsSettings, Quality, TextureRenderer};

use super::character_capture::{Stage, Stance, repository_library, stage_world};

const FRAME: f32 = 1.0 / 60.0;

/// The default character staged (`stage_world`, playing Idle), its
/// camera zoomed all the way in, after `frames` frames of the game's
/// order: the controllers, the Animators, then the eyes.
fn zoomed_in(stage: &Stage, frames: usize) -> (World, Animators, Controllers) {
    looking(stage, frames, None)
}

/// [`zoomed_in`], the mouse turned in the first frame to look `pitch`
/// degrees up (down negative), when given.
fn looking(stage: &Stage, frames: usize, pitch: Option<f32>) -> (World, Animators, Controllers) {
    let default = axcenity_gameplay::description::default_description(
        &axcenity_gameplay::description::library(&stage.assets),
        axcenity_avatar::DEFAULT_BASE,
    );
    let (mut world, mut animators) =
        stage_world(&stage.assets, &default, Stance::Idle, &[]).expect("the stage");
    let controller = world.all_below(world.workspace(), |class| {
        class == ClassName::ThirdPersonController
    })[0];
    world
        .set_property(controller, "CameraDistance", Value::Number(0.5))
        .unwrap();
    let mut controllers = Controllers::default();
    for frame in 0..frames {
        let mut input = MoveInput::default();
        if frame == 0
            && let Some(pitch) = pitch
        {
            // Play starts 18° down; the mouse turns 0.003 radians a pixel.
            input.look_dy = (-18.0 - pitch).to_radians() / 0.003;
        }
        controllers.step(&mut world, &stage.assets, input, FRAME);
        controllers.keep_kinematic(&mut world);
        animators.update(&mut world, &stage.assets, f64::from(FRAME));
        controllers.place_eyes(&world, &stage.assets, animators.poses());
    }
    (world, animators, controllers)
}

#[test]
fn zoomed_all_the_way_in_the_camera_is_at_the_default_characters_eyes() {
    let stage = Stage::new("eyes", &repository_library()).expect("the stage");
    let (world, _, controllers) = looking(&stage, 30, Some(0.0));
    assert!(controllers.zoomed_to_eyes(&world));
    let view = controllers.view(&world).unwrap();
    // A 1.75 m body (5 studs) has its eyes about 1.63 m up (4.66 studs),
    // and they're in front of the middle of its head, which faces -Z.
    assert!(
        view.position.y > 4.4 && view.position.y < 4.9,
        "eyes {} studs up",
        view.position.y
    );
    assert!(
        view.position.z < -0.1 && view.position.z > -0.7,
        "eyes {} studs ahead",
        -view.position.z
    );
    assert!(view.position.x.abs() < 0.1, "{:?}", view.position);
    // Idle moves the head: the eyes move with it.
    let (world_later, _, later) = zoomed_in(&stage, 75);
    let moved = later.view(&world_later).unwrap().position;
    assert_ne!(moved, view.position, "the eyes ride on the posed head");
}

/// What the eyes see at `pitch` degrees (up positive), as RGBA, with the
/// character drawn or left out.
fn seen(
    device: &axcenity_render::wgpu::Device,
    queue: &axcenity_render::wgpu::Queue,
    stage: &Stage,
    pitch: f32,
    with_character: bool,
) -> Vec<u8> {
    let (mut world, animators, controllers) = looking(stage, 30, Some(pitch));
    let view = controllers.view(&world).unwrap();
    assert!(
        (view.orientation.x - pitch).abs() < 0.01,
        "{:?}",
        view.orientation
    );
    let at = Vec3::new(view.position.x, view.position.y, view.position.z);
    let (sin, cos) = pitch.to_radians().sin_cos();
    let camera = Camera {
        position: at,
        look_at: at + Vec3::new(0.0, sin, -cos),
        fov_y_degrees: 70.0,
    };
    if !with_character {
        for part in world.all_below(world.workspace(), |class| class.is_base_part()) {
            if world.get(part).is_some_and(|p| p.name() != "Floor") {
                let _ = world.set_property(part, "Transparency", Value::Number(1.0));
            }
        }
    }
    let mut renderer = TextureRenderer::new(device, queue, SIZE.0, SIZE.1);
    renderer.set_graphics(&GraphicsSettings::preset(Quality::High));
    let mut cache = axcenity_scene::TerrainCache::new();
    for _ in 0..12 {
        let list = axcenity_scene::build_draw_list_posed(
            &world,
            &stage.assets,
            camera,
            animators.poses(),
            &mut cache,
        );
        renderer.render(&list).unwrap();
    }
    renderer.read_pixels()
}

const SIZE: (u32, u32) = (320, 180);

#[test]
fn from_the_eyes_looking_ahead_or_up_nothing_of_the_head_is_in_the_way() {
    let Some((device, queue)) = super::bench_tests::bench_gpu() else {
        return;
    };
    let stage = Stage::new("eyes-view", &repository_library()).expect("the stage");
    let evidence = std::env::var_os("AXCENITY_EYES_DIR").map(std::path::PathBuf::from);
    for pitch in [0.0, 30.0, -60.0] {
        let with = seen(&device, &queue, &stage, pitch, true);
        if let Some(folder) = &evidence {
            let _ = std::fs::create_dir_all(folder);
            let path = folder.join(format!("eyes-{pitch}.png"));
            super::character_capture::save_png(&path, SIZE.0, SIZE.1, &with).unwrap();
        }
        if pitch < 0.0 {
            // Looking down, the body is seen, as it should be.
            continue;
        }
        let without = seen(&device, &queue, &stage, pitch, false);
        let differ = with
            .chunks(4)
            .zip(without.chunks(4))
            .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(x, y)| x.abs_diff(*y) > 24))
            .count();
        let share = differ as f64 / f64::from(SIZE.0 * SIZE.1);
        assert!(
            share < 0.02,
            "looking {pitch}°: {:.1}% of the view is the head",
            share * 100.0
        );
    }
}
