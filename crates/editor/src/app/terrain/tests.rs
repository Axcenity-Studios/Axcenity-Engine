use super::*;

fn app_with_project(name: &str) -> (EditorApp, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "axcenity-editor-terrain-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let folder = axcenity_project::create_project(&parent, "Terrain Test").unwrap();
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

fn pointer(point: Vec3, pressed: bool, down: bool, released: bool) -> TerrainPointer {
    TerrainPointer {
        point: Some(point),
        pressed,
        down,
        released,
        seconds: 1.0 / 60.0,
        ..Default::default()
    }
}

#[test]
fn a_new_terrain_is_one_undo_step_with_square_cells_and_starter_layers() {
    let (mut app, folder) = app_with_project("create");
    let terrain = create(&mut app);
    assert_eq!(app.world.terrain(terrain).unwrap().size(), (65, 33));
    assert_eq!(materials::layers(&app.world, terrain).len(), 3);
    assert_eq!(app.selected, Some(terrain));
    assert_eq!(
        axcenity_project::data_folder(&app.world, terrain).as_deref(),
        Some("assets/terrain/Terrain")
    );
    // The flat ground sits at Y = 0.
    let placement = Placement::of(app.world.get(terrain).unwrap()).unwrap();
    let data = app.world.terrain(terrain).unwrap();
    assert!(placement.ground_y(data, 0.0, 0.0).unwrap().abs() < 1e-4);
    app.undo();
    assert!(crate::terrain_studio::terrains(&app.world).is_empty());
    app.redo();
    assert_eq!(crate::terrain_studio::terrains(&app.world), vec![terrain]);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_stroke_raises_the_ground_and_undoes_as_one_step() {
    let (mut app, folder) = app_with_project("stroke");
    let terrain = create(&mut app);
    app.terrain.tool = Some(TerrainTool::Raise);
    app.terrain.brush.radius = 8.0;
    let ground = |app: &EditorApp, x: f32| {
        let placement = Placement::of(app.world.get(terrain).unwrap()).unwrap();
        placement
            .ground_y(app.world.terrain(terrain).unwrap(), x, 0.0)
            .unwrap()
    };
    app.terrain_pointer(pointer(Vec3::new(-20.0, 0.0, 0.0), true, true, false));
    for step in 1..=20 {
        let x = -20.0 + step as f32 * 2.0;
        app.terrain_pointer(pointer(Vec3::new(x, 0.0, 0.0), false, true, false));
    }
    app.terrain_pointer(pointer(Vec3::new(20.0, 0.0, 0.0), false, false, true));
    assert!(app.terrain.stroke.is_none());
    // Raised all along the path, with no gaps.
    for x in [-20.0, -10.0, 0.0, 10.0, 20.0] {
        assert!(ground(&app, x) > 0.2, "raised at {x}: {}", ground(&app, x));
    }
    assert!(ground(&app, 50.0).abs() < 1e-4, "far away stays");
    assert!(!app.history.is_saved());
    app.undo();
    assert!(
        ground(&app, 0.0).abs() < 1e-4,
        "one undo takes the stroke away"
    );
    app.redo();
    assert!(ground(&app, 0.0) > 0.2);
    // Shift lowers.
    app.terrain_pointer(TerrainPointer {
        modifiers: egui::Modifiers::SHIFT,
        ..pointer(Vec3::new(40.0, 0.0, 0.0), true, true, false)
    });
    app.terrain_pointer(pointer(Vec3::new(40.0, 0.0, 0.0), false, false, true));
    assert!(ground(&app, 40.0) < -0.01);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn painting_needs_a_layer_and_ctrl_click_picks_one() {
    let (mut app, folder) = app_with_project("paint");
    let terrain = create(&mut app);
    app.terrain.tool = Some(TerrainTool::Paint);
    app.terrain.paint_layer = 2;
    app.terrain.paint_mode = axcenity_terrain::brush::PaintMode::Replace;
    app.terrain.brush.radius = 6.0;
    app.terrain_pointer(pointer(Vec3::ZERO, true, true, false));
    app.terrain_pointer(pointer(Vec3::ZERO, false, false, true));
    let data = app.world.terrain(terrain).unwrap();
    let (x, z) = (data.size().0 / 2, data.size().1 / 2);
    assert!(data.paint()[data.index(x, z)][2] > 200);
    assert_eq!(app.history.undo_label(), Some("Paint Rock"));
    // Ctrl+click on the painted spot picks Rock again.
    app.terrain.paint_layer = 0;
    app.terrain_pointer(TerrainPointer {
        modifiers: egui::Modifiers::COMMAND,
        ..pointer(Vec3::ZERO, true, true, false)
    });
    assert_eq!(app.terrain.paint_layer, 2);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn painting_foliage_scatters_plants_and_props_and_undo_removes_the_stroke() {
    let (mut app, folder) = app_with_project("foliage");
    let terrain = create(&mut app);
    app.terrain.tool = Some(TerrainTool::Foliage);
    app.terrain.foliage_source =
        axcenity_core::FoliageSource::Plant(axcenity_core::FoliagePlant::Grass);
    app.terrain.foliage_density = 0.4;
    app.terrain.brush.radius = 10.0;
    app.terrain.brush.strength = 1.0;
    app.terrain.brush.falloff = 0.0;
    app.terrain_pointer(pointer(Vec3::ZERO, true, true, false));
    app.terrain_pointer(pointer(Vec3::ZERO, false, false, true));
    let plants = app.world.terrain(terrain).unwrap().foliage().len();
    assert!(plants > 3, "scattered {plants} plants");
    assert_eq!(app.history.undo_label(), Some("Paint Grass Foliage"));
    app.undo();
    assert!(app.world.terrain(terrain).unwrap().foliage().is_empty());
    app.redo();
    assert_eq!(app.world.terrain(terrain).unwrap().foliage().len(), plants);
    assert!(
        app.world
            .terrain(terrain)
            .unwrap()
            .foliage()
            .iter()
            .all(|i| !i.collide)
    );

    app.terrain.foliage_source =
        axcenity_core::FoliageSource::Prop(axcenity_core::FoliageProp::Crate);
    app.terrain.foliage_collide = true;
    app.terrain.foliage_density = 0.15;
    app.terrain_pointer(pointer(Vec3::new(20.0, 0.0, 0.0), true, true, false));
    app.terrain_pointer(pointer(Vec3::new(20.0, 0.0, 0.0), false, false, true));
    let mixed = app.world.terrain(terrain).unwrap().foliage().len();
    assert!(
        mixed > plants,
        "a prop stroke adds more: {mixed} vs {plants}"
    );
    assert!(
        app.world
            .terrain(terrain)
            .unwrap()
            .foliage()
            .iter()
            .any(|i| i.source
                == axcenity_core::FoliageSource::Prop(axcenity_core::FoliageProp::Crate)
                && i.collide)
    );
    app.undo();
    assert_eq!(app.world.terrain(terrain).unwrap().foliage().len(), plants);
    assert!(
        app.world
            .terrain(terrain)
            .unwrap()
            .foliage()
            .iter()
            .all(|i| !i.collide)
    );
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_generated_preview_changes_nothing_until_applied_then_saves_and_reloads() {
    let (mut app, folder) = app_with_project("generate");
    let terrain = create(&mut app);
    app.terrain.new_terrain.samples = 65;
    app.terrain.new_terrain.width = 128.0;
    app.terrain.new_terrain.length = 128.0;
    app.terrain.generator = Preset::MountainRange.settings();
    app.terrain.generator.seed = 7;
    app.start_generate();
    let ctx = egui::Context::default();
    while app.terrain.job.is_some() {
        std::thread::sleep(Duration::from_millis(5));
        app.poll_terrain_job(&ctx);
    }
    let preview = app.terrain.preview.as_ref().expect("a preview");
    assert_eq!(preview.data.size(), (65, 65));
    assert_eq!(
        app.world.terrain(terrain).unwrap().size(),
        (65, 33),
        "not applied yet"
    );
    let made = preview.data.heights().to_vec();

    app.terrain_action(TerrainAction::ApplyPreview);
    let data = app.world.terrain(terrain).unwrap().clone();
    assert_eq!(data.heights(), made.as_slice());
    let settings = GeneratorSettings::from_json(data.generation.as_deref().unwrap()).unwrap();
    assert_eq!(
        (settings.seed, settings.version),
        (7, generate::GENERATOR_VERSION)
    );

    assert!(app.save());
    let project = app.project.as_ref().unwrap().folder.clone();
    let reopened = EditorApp::new(Some(&project));
    let id = crate::terrain_studio::terrains(&reopened.world)[0];
    let loaded = reopened.world.terrain(id).unwrap();
    assert_eq!(loaded.size(), (65, 65));
    let worst = loaded
        .heights()
        .iter()
        .zip(&made)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    assert!(worst < 1e-4, "saved to 16 bits: off by {worst}");

    // Undo puts the old landscape and size back.
    app.undo();
    assert_eq!(app.world.terrain(terrain).unwrap().size(), (65, 33));
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn texture_names_fill_the_right_maps() {
    let roles = axcenity_assets::texture_roles("Rock_ORM.png");
    let edits = layer_map_edits(&roles, "assets/textures/Rock_ORM.png");
    assert!(edits.contains(&Edit::Property(
        "RoughnessChannel",
        Value::TextureChannel(TextureChannel::Green)
    )));
    assert!(edits.contains(&Edit::Property(
        "AmbientOcclusionMap",
        Value::String("assets/textures/Rock_ORM.png".to_owned())
    )));
}

#[test]
fn layers_move_past_each_other() {
    let (mut app, folder) = app_with_project("layers");
    let terrain = create(&mut app);
    let names = |app: &EditorApp| -> Vec<String> {
        materials::layers(&app.world, terrain)
            .into_iter()
            .map(|id| app.world.get(id).unwrap().name().to_string())
            .collect()
    };
    assert_eq!(names(&app), ["Grass", "Dirt", "Rock"]);
    let rock = materials::layers(&app.world, terrain)[2];
    app.terrain_action(TerrainAction::MoveLayer(rock, true));
    assert_eq!(names(&app), ["Grass", "Rock", "Dirt"]);
    app.terrain_action(TerrainAction::MoveLayer(rock, false));
    assert_eq!(names(&app), ["Grass", "Dirt", "Rock"]);
    app.terrain_action(TerrainAction::AddLayer(
        crate::terrain_studio::LayerPreset {
            ..layers_for(Preset::Island)[0]
        },
    ));
    assert_eq!(names(&app).len(), 4);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_project_texture_set_fills_the_chosen_layer_as_one_undo_step() {
    let (mut app, folder) = app_with_project("texture-set");
    let terrain = create(&mut app);
    let dirt = materials::layers(&app.world, terrain)[1];
    let project = app.project.as_ref().unwrap().folder.clone();
    let set = axcenity_assets::texture_set_from_files(
        "test",
        "Mud",
        "",
        &[
            ("Mud_Color.png", "assets/textures/Mud/Mud_Color.png"),
            ("Mud_NormalGL.png", "assets/textures/Mud/Mud_NormalGL.png"),
        ],
    );
    std::fs::create_dir_all(project.join("assets/textures/Mud")).unwrap();
    let path = "assets/textures/Mud/Mud.texture.json";
    axcenity_assets::write_texture_set(&project.join(path), &set).unwrap();
    let map = |app: &EditorApp, name| {
        app.world
            .get(dirt)
            .unwrap()
            .property(name)
            .and_then(Value::as_str)
            .unwrap()
            .to_owned()
    };
    let before = map(&app, "ColorMap");

    let entry = crate::asset_browser::AssetEntry {
        kind: crate::asset_browser::AssetKind::TextureSet,
        path: path.to_owned(),
        name: "Mud".to_owned(),
    };
    app.terrain_action(TerrainAction::UseTextureSet(dirt, entry));
    assert_eq!(map(&app, "ColorMap"), "assets/textures/Mud/Mud_Color.png");
    assert_eq!(
        map(&app, "NormalMap"),
        "assets/textures/Mud/Mud_NormalGL.png"
    );
    // The other layers are left alone.
    let grass = materials::layers(&app.world, terrain)[0];
    assert_ne!(
        app.world
            .get(grass)
            .unwrap()
            .property("ColorMap")
            .and_then(Value::as_str),
        Some("assets/textures/Mud/Mud_Color.png")
    );
    // Set up to suit its maps: no roughness or metalness map, so ground
    // values; a normal map at full strength.
    let number = |app: &EditorApp, name| {
        app.world
            .get(dirt)
            .unwrap()
            .property(name)
            .and_then(Value::as_number)
            .unwrap()
    };
    assert_eq!(number(&app, "Roughness"), 0.9);
    assert_eq!(number(&app, "Metalness"), 0.0);
    assert_eq!(number(&app, "NormalStrength"), 1.0);

    // A tweak stays until Auto settings puts it back, as its own undo step.
    app.terrain_action(TerrainAction::SetLayer(
        dirt,
        "Roughness",
        Value::Number(0.4),
    ));
    app.history.seal();
    app.terrain_action(TerrainAction::AutoLayerSettings(dirt));
    assert_eq!(number(&app, "Roughness"), 0.9);
    app.undo();
    assert_eq!(number(&app, "Roughness"), 0.4);
    app.undo();
    app.undo();
    assert_eq!(map(&app, "ColorMap"), before);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn auto_settings_let_a_roughness_or_metalness_map_show_as_it_is() {
    let mut world = World::new();
    let layer = world.create(ClassName::TerrainLayer).unwrap();
    for map in ["RoughnessMap", "MetalnessMap"] {
        world
            .set_property(layer, map, Value::String("assets/textures/rock.png".into()))
            .unwrap();
    }
    let settings = crate::app::texture_set::auto_layer_settings(world.get(layer).unwrap());
    assert!(settings.contains(&("Roughness", Value::Number(1.0))));
    assert!(settings.contains(&("Metalness", Value::Number(1.0))));
}

#[test]
fn the_holes_brush_cuts_a_cave_mouth_that_saves_and_shift_fills_it() {
    let (mut app, folder) = app_with_project("holes");
    let terrain = create(&mut app);
    app.terrain.tool = Some(TerrainTool::Holes);
    app.terrain.brush.radius = 4.0;
    let ground = |app: &EditorApp, x: f32| {
        let placement = Placement::of(app.world.get(terrain).unwrap()).unwrap();
        placement.ground_y(app.world.terrain(terrain).unwrap(), x, 0.0)
    };
    app.terrain_pointer(pointer(Vec3::ZERO, true, true, false));
    app.terrain_pointer(pointer(Vec3::ZERO, false, false, true));
    assert_eq!(app.history.undo_label(), Some("Cut Terrain Holes"));
    assert!(ground(&app, 0.0).is_none(), "no ground over the hole");
    assert!(ground(&app, 20.0).is_some(), "the rest stays");
    // It saves with the terrain.
    assert!(app.save());
    let project = app.project.as_ref().unwrap().folder.clone();
    let data_folder = project.join("assets/terrain/Terrain");
    assert!(data_folder.join("holes.png").exists());
    app.undo();
    assert!(ground(&app, 0.0).is_some(), "one undo fills it");
    app.redo();
    assert!(ground(&app, 0.0).is_none());
    // Shift fills it back in.
    app.terrain_pointer(TerrainPointer {
        modifiers: egui::Modifiers::SHIFT,
        ..pointer(Vec3::ZERO, true, true, false)
    });
    app.terrain_pointer(pointer(Vec3::ZERO, false, false, true));
    assert_eq!(app.history.undo_label(), Some("Fill Terrain Holes"));
    assert!(!app.world.terrain(terrain).unwrap().has_holes());
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn the_water_brush_paints_a_pond_that_saves_undoes_and_shift_removes() {
    let (mut app, folder) = app_with_project("water");
    let terrain = create(&mut app);
    app.terrain.tool = Some(TerrainTool::Water);
    app.terrain.brush.radius = 4.0;
    let placement = Placement::of(app.world.get(terrain).unwrap()).unwrap();
    let ground = placement
        .ground_y(app.world.terrain(terrain).unwrap(), 0.0, 0.0)
        .unwrap();
    let middle = |app: &EditorApp| {
        let data = app.world.terrain(terrain).unwrap();
        let (size_x, size_z) = data.size();
        data.water_at(size_x / 2, size_z / 2)
            .map(|level| placement.world_y(level))
    };
    // Above where the stroke starts, by the depth.
    app.terrain.water_depth = 2.0;
    let start = Vec3::new(0.0, ground, 0.0);
    app.terrain_pointer(pointer(start, true, true, false));
    app.terrain_pointer(pointer(start, false, false, true));
    assert_eq!(app.history.undo_label(), Some("Paint Terrain Water"));
    let level = middle(&app).expect("the brush painted water");
    assert!((level - (ground + 2.0)).abs() < 0.01, "{level} vs {ground}");
    // It saves with the terrain.
    assert!(app.save());
    let project = app.project.as_ref().unwrap().folder.clone();
    assert!(project.join("assets/terrain/Terrain/water.png").exists());
    app.undo();
    assert!(
        !app.world.terrain(terrain).unwrap().has_water(),
        "one undo takes it away"
    );
    app.redo();
    assert!(middle(&app).is_some());
    // Ctrl+click picks a level of its own.
    app.terrain_pointer(TerrainPointer {
        modifiers: egui::Modifiers::COMMAND,
        ..pointer(Vec3::new(0.0, ground + 5.0, 0.0), true, true, false)
    });
    app.terrain_pointer(pointer(start, false, false, true));
    assert!(!app.terrain.water_from_stroke);
    assert_eq!(app.terrain.water_level, ground + 5.0);
    // Shift removes it.
    app.terrain_pointer(TerrainPointer {
        modifiers: egui::Modifiers::SHIFT,
        ..pointer(start, true, true, false)
    });
    app.terrain_pointer(pointer(start, false, false, true));
    assert_eq!(app.history.undo_label(), Some("Remove Terrain Water"));
    assert!(!app.world.terrain(terrain).unwrap().has_water());
    std::fs::remove_dir_all(folder).unwrap();
}

/// Runs one frame of the studio's keys with `key` pressed.
fn press_studio_key(app: &mut EditorApp, key: egui::Key) {
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = crate::theme::run(&egui::Context::default(), input, |ui| {
        let ctx = ui.ctx().clone();
        app.terrain_frame(&ctx);
    });
}

#[test]
fn a_tool_picked_by_key_ribbon_or_search_shows_its_studio_tab() {
    use crate::terrain_studio::StudioTab;
    let (mut app, folder) = app_with_project("tabs");
    create(&mut app);
    // A new terrain starts with Raise, on Sculpt.
    assert_eq!(app.terrain.tool, Some(TerrainTool::Raise));
    assert_eq!(app.terrain.tab(), StudioTab::Sculpt);
    // Keys: 7 is Paint, 9 Generate; Escape keeps the tab.
    press_studio_key(&mut app, egui::Key::Num7);
    assert_eq!(app.terrain.tool, Some(TerrainTool::Paint));
    assert_eq!(app.terrain.tab(), StudioTab::Materials);
    press_studio_key(&mut app, egui::Key::Num9);
    assert_eq!(app.terrain.tab(), StudioTab::Terrain);
    press_studio_key(&mut app, egui::Key::Escape);
    assert_eq!(app.terrain.tool, None);
    assert_eq!(app.terrain.tab(), StudioTab::Terrain);
    // The ribbon's Brush menu.
    app.layout.toggle(Pane::Terrain);
    assert!(!app.layout.is_open(Pane::Terrain));
    app.pick_terrain_tool(TerrainTool::Foliage);
    assert!(app.layout.is_open(Pane::Terrain));
    assert_eq!(app.terrain.tab(), StudioTab::Materials);
    // Universal search's Terrain Brush commands.
    let ctx = crate::theme::test_context();
    app.run_command(
        &ctx,
        super::super::command_palette::Command::TerrainTool(TerrainTool::Raise),
    );
    assert_eq!(app.terrain.tab(), StudioTab::Sculpt);
    // Back on Materials, the last tool there is picked again.
    app.terrain.switch_tab(StudioTab::Materials);
    assert_eq!(app.terrain.tool, Some(TerrainTool::Foliage));
    app.terrain.switch_tab(StudioTab::Terrain);
    assert_eq!(app.terrain.tool, Some(TerrainTool::Generate));
    std::fs::remove_dir_all(folder).unwrap();
}
