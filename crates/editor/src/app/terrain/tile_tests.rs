use super::*;
use crate::terrain_studio::TileFill;
use axcenity_terrain::tiles::{self, Side};

fn app_with_project(name: &str) -> (EditorApp, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "axcenity-editor-tiles-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let folder = axcenity_project::create_project(&parent, "Tile Test").unwrap();
    let app = EditorApp::new(Some(&folder));
    assert!(app.project.is_some(), "the project opens");
    (app, parent)
}

fn create(app: &mut EditorApp) -> InstanceId {
    app.terrain_action(TerrainAction::Create {
        width: 128.0,
        length: 64.0,
        height: 32.0,
        samples: 65,
    });
    app.terrain.target.expect("the new terrain is the target")
}

/// Raises a hill on the terrain's +X edge, so its edges aren't all alike.
fn sculpt_an_edge(app: &mut EditorApp) {
    app.terrain.tool = Some(TerrainTool::Raise);
    app.terrain.brush.radius = 12.0;
    let pointer = |pressed, down, released| TerrainPointer {
        point: Some(Vec3::new(60.0, 0.0, 0.0)),
        pressed,
        down,
        released,
        seconds: 0.5,
        ..Default::default()
    };
    app.terrain_pointer(pointer(true, true, false));
    for _ in 0..6 {
        app.terrain_pointer(pointer(false, true, false));
    }
    app.terrain_pointer(pointer(false, false, true));
    app.terrain.tool = None;
}

fn add_tile(app: &mut EditorApp, from: InstanceId, side: Side, fill: TileFill) -> InstanceId {
    let before = crate::terrain_studio::terrains(&app.world);
    app.add_terrain_tile(from, side, fill);
    let ctx = egui::Context::default();
    while app.terrain.job.is_some() {
        std::thread::sleep(Duration::from_millis(5));
        app.poll_terrain_job(&ctx);
    }
    assert!(app.terrain.preview.is_none(), "a tile isn't a preview");
    let after = crate::terrain_studio::terrains(&app.world);
    assert_eq!(after.len(), before.len() + 1, "{side:?}");
    *after.iter().find(|id| !before.contains(id)).unwrap()
}

#[test]
fn a_tile_added_on_each_side_meets_its_edge_copies_its_setup_and_undoes_in_one_step() {
    let (mut app, folder) = app_with_project("sides");
    let terrain = create(&mut app);
    sculpt_an_edge(&mut app);
    let edge = tiles::edge_heights(app.world.terrain(terrain).unwrap(), Side::PosX);
    let rise = edge.iter().copied().fold(0.0, f32::max) - edge.iter().copied().fold(1.0, f32::min);
    assert!(rise > 0.01, "the +X edge was sculpted: {rise}");
    let source = Placement::of(app.world.get(terrain).unwrap()).unwrap();
    let source_folder = axcenity_project::data_folder(&app.world, terrain).unwrap();
    for side in Side::ALL {
        for fill in [TileFill::Continue, TileFill::Flat] {
            let tile = add_tile(&mut app, terrain, side, fill);
            let placement = Placement::of(app.world.get(tile).unwrap()).unwrap();
            assert_eq!(placement, tiles::neighbour_placement(source, side));
            assert_eq!(tiles::neighbour_on(&app.world, terrain, side), Some(tile));
            assert_eq!(
                materials::layers(&app.world, tile).len(),
                3,
                "layers copied"
            );
            let tile_folder = axcenity_project::data_folder(&app.world, tile).unwrap();
            assert_ne!(tile_folder, source_folder, "a data folder of its own");
            let (ours, theirs) = (
                app.world.terrain(terrain).unwrap(),
                app.world.terrain(tile).unwrap(),
            );
            assert_eq!(theirs.size(), ours.size());
            assert_eq!(
                tiles::edge_heights(theirs, side.opposite()),
                tiles::edge_heights(ours, side),
                "{side:?} {fill:?}"
            );
            assert_eq!(app.terrain.target, Some(tile));
            assert_eq!(app.selected, Some(tile));
            assert!(
                app.history
                    .undo_label()
                    .unwrap()
                    .starts_with("Add Terrain Tile")
            );
            assert!(!tiles::free_sides(&app.world, terrain).contains(&side));
            app.undo();
            assert!(
                !crate::terrain_studio::terrains(&app.world).contains(&tile),
                "one undo takes the tile away"
            );
            assert!(tiles::free_sides(&app.world, terrain).contains(&side));
        }
    }
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn tiles_continue_the_landscape_and_a_taken_side_is_refused() {
    let (mut app, folder) = app_with_project("continue");
    let terrain = create(&mut app);
    app.terrain.generator = Preset::MountainRange.settings();
    let generated = app
        .generate_now(
            Some(terrain),
            Preset::MountainRange.settings(),
            Vector3::new(128.0, 32.0, 64.0),
            "Generate".to_owned(),
        )
        .unwrap();
    assert_eq!(generated, terrain);
    let east = add_tile(&mut app, terrain, Side::PosX, TileFill::Continue);
    let settings = GeneratorSettings::from_json(
        app.world
            .terrain(east)
            .unwrap()
            .generation
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(settings.offset, [128.0, 0.0], "one tile along");
    // Away from the blended edge it's the landscape going on, unchanged.
    let continued = generate::generate(
        &settings,
        Target {
            size_x: 65,
            size_z: 33,
            world_size: Vec2::new(128.0, 64.0),
        },
        &axcenity_terrain::job::JobControl::new(),
    )
    .unwrap()
    .unwrap();
    let data = app.world.terrain(east).unwrap();
    assert_eq!(data.heights()[10 * 65 + 40], continued[10 * 65 + 40]);
    // A tile beside the new one, then the landscape counts three.
    let north = add_tile(&mut app, east, Side::PosZ, TileFill::Continue);
    assert_eq!(tiles::connected(&app.world, terrain).len(), 3);
    assert_eq!(
        tiles::neighbour_on(&app.world, east, Side::PosZ),
        Some(north)
    );
    // Taken: nothing is added.
    let count = crate::terrain_studio::terrains(&app.world).len();
    app.add_terrain_tile(terrain, Side::PosX, TileFill::Flat);
    assert!(app.terrain.job.is_none());
    assert_eq!(crate::terrain_studio::terrains(&app.world).len(), count);
    // Saved and opened again, the tiles are still tiles.
    assert!(app.save());
    let project = app.project.as_ref().unwrap().folder.clone();
    let reopened = EditorApp::new(Some(&project));
    let all = crate::terrain_studio::terrains(&reopened.world);
    assert_eq!(all.len(), 3);
    assert_eq!(tiles::connected(&reopened.world, all[0]).len(), 3);
    std::fs::remove_dir_all(folder).unwrap();
}
