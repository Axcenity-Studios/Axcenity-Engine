//! Group as Mesh and Ungroup Mesh (Model ▸ Solid Modeling).

use axcenity_render::glam::{EulerRot, Quat, Vec3};

use super::solid::SolidAction;
use super::tests::copy_of_example;
use super::*;

/// A Part named `name` in the Workspace with this size and position.
fn part(app: &mut EditorApp, name: &str, size: [f32; 3], at: [f32; 3]) -> InstanceId {
    let id = app.world.create(ClassName::Part).unwrap();
    app.world.set_name(id, name).unwrap();
    app.world.set_property(id, "Size", v(size)).unwrap();
    app.world.set_property(id, "Position", v(at)).unwrap();
    app.world
        .set_property(id, "Anchored", Value::Bool(true))
        .unwrap();
    let workspace = app.world.workspace();
    app.world.set_parent(id, Some(workspace)).unwrap();
    id
}

fn v([x, y, z]: [f32; 3]) -> Value {
    Value::Vector3(axcenity_core::Vector3::new(x, y, z))
}

fn vector(app: &EditorApp, id: InstanceId, name: &str) -> Vec3 {
    let v = app
        .world
        .get(id)
        .unwrap()
        .property(name)
        .and_then(Value::as_vector3)
        .unwrap();
    Vec3::new(v.x, v.y, v.z)
}

fn workspace_names(app: &EditorApp) -> Vec<String> {
    let workspace = app.world.get(app.world.workspace()).unwrap();
    workspace
        .children()
        .iter()
        .map(|&id| app.world.get(id).unwrap().name().to_owned())
        .collect()
}

/// A plate, a brick on it and the hello-csg Doorway's model as a
/// MeshPart beside them.
fn build(app: &mut EditorApp) -> [InstanceId; 3] {
    let plate = part(app, "Plate", [8.0, 1.0, 8.0], [0.0, 0.5, 0.0]);
    let brick = part(app, "Brick", [2.0, 1.0, 2.0], [1.0, 1.5, 0.0]);
    let door = app.world.create(ClassName::MeshPart).unwrap();
    app.world.set_name(door, "Door").unwrap();
    for (name, value) in [
        (
            "MeshId",
            Value::String("assets/models/Doorway.union.glb".to_owned()),
        ),
        ("Size", v([6.0, 4.0, 1.0])),
        ("Position", v([0.0, 3.0, -3.5])),
        ("Anchored", Value::Bool(true)),
    ] {
        app.world.set_property(door, name, value).unwrap();
    }
    let workspace = app.world.workspace();
    app.world.set_parent(door, Some(workspace)).unwrap();
    [plate, brick, door]
}

#[test]
fn group_as_mesh_makes_one_meshpart_keeping_the_parts_as_one_undo_step() {
    let folder = copy_of_example("hello-csg", "group-mesh");
    let mut app = EditorApp::new(Some(&folder));
    let parts = build(&mut app);
    let names = workspace_names(&app);
    let before: Vec<_> = parts
        .iter()
        .map(|&id| crate::history::snapshot(&app.world, id))
        .collect();

    app.select_many(parts.to_vec());
    assert!(SolidAction::GroupAsMesh.usable(&app.world, &app.solid_parts()));
    app.solid_action(SolidAction::GroupAsMesh);
    assert_eq!(app.history.undo_label(), Some("Group 3 parts as a mesh"));
    let mesh = app.selected.unwrap();
    let instance = app.world.get(mesh).unwrap();
    assert_eq!(instance.class(), ClassName::MeshPart);
    assert_eq!(instance.name(), "Group");
    // It takes the plate's place, with the plate's physics.
    assert_eq!(instance.parent(), Some(app.world.workspace()));
    assert_eq!(instance.property("Anchored"), Some(&Value::Bool(true)));
    let mesh_id = instance.property("MeshId").and_then(Value::as_str).unwrap();
    assert_eq!(mesh_id, "assets/models/Group.group.glb");
    assert!(folder.join(mesh_id).is_file());
    let model = app.assets.model_blocking(mesh_id).expect("it loads");
    assert_eq!(model.collision.len(), 2 + 1, "each part collides as before");
    // The parts are kept inside it, out of the world.
    let kept = axcenity_core::csg::grouped_parts(&app.world, mesh).unwrap();
    assert_eq!(app.world.get(kept).unwrap().children(), parts);
    for id in parts {
        assert!(axcenity_core::csg::out_of_world(&app.world, id));
    }
    let last = super::tests::last_line(&app).text.clone();
    let summary = app
        .output
        .lines()
        .iter()
        .find(|line| line.text.starts_with("Grouped 3 parts: "))
        .map(|line| line.text.clone());
    assert!(summary.is_some(), "{last}");
    // The move tools move the MeshPart, never the parts kept inside it.
    let moved = crate::viewport::movable_parts(&app.world, &[mesh], Some(mesh));
    assert_eq!(moved.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [mesh]);

    // Undo puts the parts back; redo groups them again.
    app.undo();
    assert_eq!(workspace_names(&app), names);
    let after: Vec<_> = parts
        .iter()
        .map(|&id| crate::history::snapshot(&app.world, id))
        .collect();
    assert_eq!(after, before);
    app.redo();
    assert_eq!(app.world.get(kept).unwrap().children(), parts);

    // Ungrouping where it was gives back the very same parts.
    app.select_many(vec![mesh]);
    assert!(SolidAction::UngroupMesh.usable(&app.world, &app.solid_parts()));
    app.solid_action(SolidAction::UngroupMesh);
    assert_eq!(app.history.undo_label(), Some("Ungroup mesh into 3 parts"));
    assert!(app.world.get(mesh).unwrap().parent().is_none());
    assert_eq!(app.selection, parts);
    let after: Vec<_> = parts
        .iter()
        .map(|&id| crate::history::snapshot(&app.world, id))
        .collect();
    assert_eq!(after, before);
    assert_eq!(workspace_names(&app), names);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn ungroup_mesh_puts_the_parts_where_the_moved_turned_and_stretched_group_is() {
    let folder = copy_of_example("hello-csg", "ungroup-mesh");
    let mut app = EditorApp::new(Some(&folder));
    let plate = part(&mut app, "Plate", [8.0, 1.0, 8.0], [0.0, 0.5, 0.0]);
    let brick = part(&mut app, "Brick", [2.0, 1.0, 4.0], [3.0, 1.5, 0.0]);
    app.select_many(vec![plate, brick]);
    app.solid_action(SolidAction::GroupAsMesh);
    let mesh = app.selected.unwrap();
    let origin = vector(&app, mesh, "Position");
    assert_eq!(origin, Vec3::new(0.0, 1.0, 0.0));

    // Moved 10 along X, turned 90° about Y, and twice as tall.
    let size = vector(&app, mesh, "Size");
    for (name, value) in [
        ("Position", v([10.0, 1.0, 0.0])),
        ("Orientation", v([0.0, 90.0, 0.0])),
        ("Size", v([size.x, size.y * 2.0, size.z])),
    ] {
        app.world.set_property(mesh, name, value).unwrap();
    }
    app.select_many(vec![mesh]);
    app.solid_action(SolidAction::UngroupMesh);
    let turn = Quat::from_euler(EulerRot::YXZ, 90f32.to_radians(), 0.0, 0.0);
    let expect = |was: Vec3| {
        let local = was - origin;
        Vec3::new(10.0, 1.0, 0.0) + turn * Vec3::new(local.x, local.y * 2.0, local.z)
    };
    for (id, was, size) in [
        (plate, Vec3::new(0.0, 0.5, 0.0), Vec3::new(8.0, 2.0, 8.0)),
        (brick, Vec3::new(3.0, 1.5, 0.0), Vec3::new(2.0, 2.0, 4.0)),
    ] {
        let at = vector(&app, id, "Position");
        assert!((at - expect(was)).length() < 1e-4, "{at} {}", expect(was));
        let orientation = vector(&app, id, "Orientation");
        assert!((orientation - Vec3::new(0.0, 90.0, 0.0)).length() < 1e-3);
        assert!((vector(&app, id, "Size") - size).length() < 1e-4);
        assert!(!axcenity_core::csg::out_of_world(&app.world, id));
    }
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_selected_model_is_kept_whole_and_names_the_group() {
    let folder = copy_of_example("hello-csg", "group-mesh-model");
    let mut app = EditorApp::new(Some(&folder));
    let house = app.world.create(ClassName::Model).unwrap();
    app.world.set_name(house, "House").unwrap();
    let workspace = app.world.workspace();
    app.world.set_parent(house, Some(workspace)).unwrap();
    let floor = part(&mut app, "Floor", [8.0, 1.0, 8.0], [0.0, 0.5, 0.0]);
    let wall = part(&mut app, "Wall", [8.0, 4.0, 1.0], [0.0, 3.0, 3.5]);
    for id in [floor, wall] {
        app.world.set_parent(id, Some(house)).unwrap();
    }
    app.select_many(vec![house]);
    app.solid_action(SolidAction::GroupAsMesh);
    let mesh = app.selected.unwrap();
    assert_eq!(app.world.get(mesh).unwrap().name(), "House");
    let kept = axcenity_core::csg::grouped_parts(&app.world, mesh).unwrap();
    assert_eq!(app.world.get(kept).unwrap().children(), [house]);
    assert_eq!(app.world.get(house).unwrap().children(), [floor, wall]);
    // The Explorer lists what's kept, under the MeshPart.
    app.explorer.reveal(&app.world, wall);
    let rows = app.explorer.rows(&app.world);
    assert!(rows.iter().any(|row| row.id == kept));
    assert!(rows.iter().any(|row| row.id == wall));
    // With `AXCENITY_SNAPSHOTS`, how the Explorer shows them (dimmed).
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS") {
        let world = app.world.clone();
        let explorer = &mut app.explorer;
        crate::snapshot::snapshot(
            &std::path::PathBuf::from(snapshots).join("group-mesh-explorer.png"),
            egui::vec2(260.0, 260.0),
            vec![Vec::new(); 3],
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let _ = explorer.show(ui, &world, &[mesh], true);
                });
            },
        );
    }

    // Saved and opened again, it's all still there.
    assert!(app.save());
    let reopened = EditorApp::new(Some(&folder));
    let mesh_again = reopened
        .world
        .find_first_child(reopened.world.workspace(), "House")
        .unwrap();
    assert_eq!(
        reopened.world.get(mesh_again).unwrap().class(),
        ClassName::MeshPart
    );
    let kept_again = axcenity_core::csg::grouped_parts(&reopened.world, mesh_again).unwrap();
    assert_eq!(
        reopened.world.get(kept_again).unwrap().property("Origin"),
        app.world.get(kept).unwrap().property("Origin")
    );
    let house_again = reopened
        .world
        .find_first_child(kept_again, "House")
        .unwrap();
    assert_eq!(reopened.world.get(house_again).unwrap().children().len(), 2);
    assert!(axcenity_core::csg::out_of_world(
        &reopened.world,
        reopened.world.get(house_again).unwrap().children()[0]
    ));

    app.select_many(vec![mesh]);
    app.solid_action(SolidAction::UngroupMesh);
    assert_eq!(app.world.get(house).unwrap().parent(), Some(workspace));
    assert_eq!(app.world.get(house).unwrap().children(), [floor, wall]);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn group_as_mesh_needs_two_parts_and_does_nothing_during_play() {
    let folder = copy_of_example("hello-csg", "group-mesh-play");
    let mut app = EditorApp::new(Some(&folder));
    let [plate, brick, _] = build(&mut app);
    app.select_many(vec![plate]);
    assert!(!SolidAction::GroupAsMesh.usable(&app.world, &app.solid_parts()));
    app.solid_action(SolidAction::GroupAsMesh);
    assert!(app.world.get(plate).unwrap().parent() == Some(app.world.workspace()));

    app.select_many(vec![plate, brick]);
    let label = app.history.undo_label().map(str::to_owned);
    app.play();
    app.solid_action(SolidAction::GroupAsMesh);
    app.stop();
    assert_eq!(app.history.undo_label().map(str::to_owned), label);
    assert!(!folder.join("assets/models/Group.group.glb").exists());
    let _ = std::fs::remove_dir_all(&folder);
}

/// The hello-mesh example's 31 props (one Model of textured MeshParts),
/// with a plate and a wall of flush bricks, drawn before and after Group
/// as Mesh from the same camera, lit and as a wireframe: the lit pictures
/// must match. With `AXCENITY_SNAPSHOTS` naming a folder, the four are
/// saved there (`group-mesh-*.png`). Skipped without a GPU.
#[test]
fn a_grouped_build_looks_the_same() {
    use axcenity_render::{DebugView, TextureRenderer};
    let instance = axcenity_render::adapters::instance();
    let Ok(adapter) = pollster::block_on(axcenity_render::adapters::adapter(&instance, None))
    else {
        return;
    };
    let Ok((device, queue)) = pollster::block_on(adapter.request_device(&Default::default()))
    else {
        return;
    };
    let folder = copy_of_example("hello-mesh", "group-mesh-look");
    let mut app = EditorApp::new(Some(&folder));
    let props = app
        .world
        .find_first_child(app.world.workspace(), "CityPropsCollection")
        .unwrap();
    let mut parts = vec![props];
    parts.push(part(&mut app, "Plate", [6.0, 0.4, 4.0], [6.0, -7.3, -12.0]));
    for row in 0..4 {
        for x in 0..3 {
            let shift = if row % 2 == 0 { 0.0 } else { 0.5 };
            let at = [4.5 + x as f32 + shift, -6.85 + row as f32 * 0.5, -13.5];
            let brick = part(&mut app, "Brick", [1.0, 0.5, 0.5], at);
            let color = if (x + row) % 3 == 0 {
                axcenity_core::Color3::new(0.7, 0.3, 0.2)
            } else {
                axcenity_core::Color3::new(0.8, 0.8, 0.75)
            };
            app.world
                .set_property(brick, "Color", Value::Color3(color))
                .unwrap();
            parts.push(brick);
        }
    }
    let camera = axcenity_render::Camera {
        position: Vec3::new(8.0, -1.0, 2.0),
        look_at: Vec3::new(-3.0, -6.5, -11.0),
        fov_y_degrees: 60.0,
    };
    // Close by the bricks and the dumpster, to see the triangles.
    let close = axcenity_render::Camera {
        position: Vec3::new(5.5, -4.0, -5.0),
        look_at: Vec3::new(4.0, -6.6, -12.0),
        fov_y_degrees: 60.0,
    };
    // Close by the ATM, the one prop with textures (a SurfaceAppearance's
    // color, normal and glow maps).
    let atm = axcenity_render::Camera {
        position: Vec3::new(-8.2, -5.1, -16.6),
        look_at: Vec3::new(-8.75, -5.41, -17.8),
        fov_y_degrees: 60.0,
    };
    let (width, height) = (960, 540);
    let mut renderer = TextureRenderer::new(&device, &queue, width, height);
    let mut draw = |app: &EditorApp, view: Option<DebugView>, camera| {
        renderer.set_debug_view(view);
        // Models and textures load in the background once asked for, so
        // draw until the picture stops changing.
        let mut last: Vec<u8> = Vec::new();
        for frame in 0..200 {
            let list = axcenity_scene::build_draw_list(&app.world, &app.assets, camera);
            renderer.render(&list).unwrap();
            let pixels = renderer.read_pixels();
            if frame >= 4 && pixels == last {
                break;
            }
            last = pixels;
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        last
    };
    let before = draw(&app, None, camera);
    let before_wire = draw(&app, Some(DebugView::Wireframe), camera);
    let before_close = draw(&app, Some(DebugView::Wireframe), close);
    let before_atm = draw(&app, None, atm);
    let before_atm_color = draw(&app, Some(DebugView::BaseColor), atm);
    let before_atm_unlit = draw(&app, Some(DebugView::Unlit), atm);
    app.select_many(parts);
    app.solid_action(SolidAction::GroupAsMesh);
    let mesh = app.selected.unwrap();
    assert_eq!(app.world.get(mesh).unwrap().class(), ClassName::MeshPart);
    let summary = app
        .output
        .lines()
        .iter()
        .find(|line| line.text.starts_with("Grouped "))
        .map(|line| line.text.clone())
        .unwrap();
    println!("{summary}");
    let after = draw(&app, None, camera);
    let after_wire = draw(&app, Some(DebugView::Wireframe), camera);
    let after_close = draw(&app, Some(DebugView::Wireframe), close);
    let after_atm = draw(&app, None, atm);
    let after_atm_color = draw(&app, Some(DebugView::BaseColor), atm);
    let after_atm_unlit = draw(&app, Some(DebugView::Unlit), atm);

    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(std::path::PathBuf::from) {
        for (name, pixels) in [
            ("before", &before),
            ("after", &after),
            ("before-wireframe", &before_wire),
            ("after-wireframe", &after_wire),
            ("before-wireframe-close", &before_close),
            ("after-wireframe-close", &after_close),
            ("before-atm", &before_atm),
            ("after-atm", &after_atm),
        ] {
            let file =
                std::fs::File::create(snapshots.join(format!("group-mesh-{name}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(pixels)
                .unwrap();
        }
    }
    // The same pictures: hardly any pixel differs, and none by much on
    // average. The ATM's normal map is left out: its model has no normal
    // map of its own, so the scene draws the SurfaceAppearance's with no
    // tangents (all along +X), while the grouped model's own normal map
    // gets real ones; its color and glow are compared.
    for (what, before, after) in [
        ("wide", &before, &after),
        ("ATM color", &before_atm_color, &after_atm_color),
        ("ATM unlit", &before_atm_unlit, &after_atm_unlit),
    ] {
        let differing = before
            .chunks(4)
            .zip(after.chunks(4))
            .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 24))
            .count();
        let total: u64 = before
            .iter()
            .zip(after)
            .map(|(x, y)| u64::from(x.abs_diff(*y)))
            .sum();
        let mean = total as f64 / before.len() as f64;
        println!("{what}: {differing} pixels differ by more than 24; mean difference {mean:.3}");
        assert!(
            differing * 1000 < before.len() / 4,
            "{what}: {differing} pixels differ"
        );
        assert!(mean < 1.0, "{what}: {mean}");
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// Groups everything in a few examples' Workspaces and prints what Group
/// as Mesh did. Run with `--run-ignored only --no-capture` to see.
#[test]
#[ignore = "a measurement"]
fn grouping_whole_examples() {
    for name in [
        "hello-part",
        "hello-mesh",
        "hello-buildings",
        "hello-physics",
        "hello-welds",
        "hello-vehicle",
        "hello-csg",
        "hello-touched",
    ] {
        let folder = copy_of_example(name, &format!("group-whole-{name}"));
        let mut app = EditorApp::new(Some(&folder));
        let workspace = app.world.workspace();
        let roots: Vec<InstanceId> = app
            .world
            .get(workspace)
            .unwrap()
            .children()
            .iter()
            .copied()
            .filter(|&id| {
                let class = app.world.get(id).unwrap().class();
                class.is_base_part() || matches!(class, ClassName::Model | ClassName::Folder)
            })
            .collect();
        app.select_many(roots);
        let started = std::time::Instant::now();
        app.solid_action(SolidAction::GroupAsMesh);
        let lines: Vec<String> = app
            .output
            .lines()
            .iter()
            .map(|line| line.text.clone())
            .filter(|text| text.starts_with("Grouped") || text.contains("group"))
            .collect();
        println!("{name} ({:?}): {lines:?}", started.elapsed());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
