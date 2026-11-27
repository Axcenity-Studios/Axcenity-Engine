use super::*;

#[test]
fn the_target_follows_the_selection_and_falls_back_to_the_first_terrain() {
    let mut world = World::new();
    let make = |world: &mut World| {
        let id = world.create(ClassName::Terrain).unwrap();
        world.set_parent(id, Some(world.workspace())).unwrap();
        id
    };
    let a = make(&mut world);
    let b = make(&mut world);
    let layer = world.create(ClassName::TerrainLayer).unwrap();
    world.set_parent(layer, Some(b)).unwrap();
    let mut studio = TerrainStudio::default();
    assert_eq!(studio.resolve_target(&world, None), Some(a));
    // Selecting a layer edits its terrain, and it stays picked after.
    assert_eq!(studio.resolve_target(&world, Some(layer)), Some(b));
    assert_eq!(studio.resolve_target(&world, None), Some(b));
    world.destroy(b).unwrap();
    assert_eq!(studio.resolve_target(&world, None), Some(a));
}

#[test]
fn only_sculpting_and_painting_tools_brush_the_viewport() {
    let brushes: Vec<TerrainTool> = TerrainTool::all().filter(|t| t.is_brush()).collect();
    assert_eq!(brushes.len(), 9);
    assert!(!TerrainTool::Generate.is_brush());
    let keys: Vec<_> = TerrainTool::all().filter_map(TerrainTool::key).collect();
    let mut unique = keys.clone();
    unique.dedup();
    assert_eq!(keys, unique);
}

#[test]
fn the_panel_draws_with_and_without_a_terrain() {
    let ctx = crate::theme::test_context();
    let mut world = World::new();
    let mut studio = TerrainStudio::default();
    let draw = |studio: &mut TerrainStudio, world: &World| {
        let mut actions = Vec::new();
        let _ = crate::theme::run(&ctx, Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                actions = studio.show(
                    ui,
                    &StudioContext {
                        world,
                        selected: None,
                        editing: true,
                        time: 0.0,
                        textures: &[],
                        pictures: &HashMap::new(),
                        world_presets: &[],
                    },
                );
            });
        });
        actions
    };
    assert!(draw(&mut studio, &world).is_empty());
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(world.workspace())).unwrap();
    let layer = world.create(ClassName::TerrainLayer).unwrap();
    world.set_parent(layer, Some(terrain)).unwrap();
    studio.open_layer = Some(layer);
    for tool in TerrainTool::all() {
        studio.tool = Some(tool);
        draw(&mut studio, &world);
    }
}

/// With `AXCENITY_SNAPSHOTS` set, saves the panel with each of a few
/// tools picked, to look at.
#[test]
fn the_panel_snapshots_with_its_main_tools() {
    let Some(folder) = std::env::var_os("AXCENITY_SNAPSHOTS") else {
        return;
    };
    let mut world = World::new();
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(world.workspace())).unwrap();
    for preset in layers_for(Preset::MountainRange) {
        let layer = world.create(ClassName::TerrainLayer).unwrap();
        for (name, value) in preset.properties(128.0) {
            world.set_property(layer, name, value).unwrap();
        }
        world.set_name(layer, preset.name).unwrap();
        world.set_parent(layer, Some(terrain)).unwrap();
    }
    // Each tab wide (the tools' names shown) and in a 260-point column.
    let mut shots = vec![
        (None, "terrain-none.png".to_owned(), 340.0),
        (
            Some(TerrainTool::Erode),
            "terrain-erode.png".to_owned(),
            340.0,
        ),
        // Short settings, so the Terrain tab's Water and data show.
        (
            Some(TerrainTool::Masks),
            "terrain-tab-terrain-masks.png".to_owned(),
            340.0,
        ),
    ];
    for (tool, tab) in [
        (TerrainTool::Raise, "sculpt"),
        (TerrainTool::Paint, "materials"),
        (TerrainTool::Generate, "terrain"),
    ] {
        shots.push((Some(tool), format!("terrain-tab-{tab}-wide.png"), 520.0));
        shots.push((Some(tool), format!("terrain-tab-{tab}-narrow.png"), 280.0));
    }
    for (tool, file, width) in shots {
        let mut studio = TerrainStudio::default();
        studio.pick(tool);
        crate::snapshot::snapshot(
            &std::path::PathBuf::from(&folder).join(file),
            egui::vec2(width, 900.0),
            vec![Vec::new(); 3],
            |ui| {
                egui::Panel::right("terrain")
                    .default_size(width - 20.0)
                    .show(ui, |ui| {
                        studio.show(
                            ui,
                            &StudioContext {
                                world: &world,
                                selected: None,
                                editing: true,
                                time: 0.0,
                                textures: &[],
                                pictures: &HashMap::new(),
                                world_presets: &[],
                            },
                        );
                    });
            },
        );
    }
}

/// With `AXCENITY_SNAPSHOTS` set, saves an open layer and the menu its
/// texture pickers show, to look at.
#[test]
fn an_open_layer_and_its_texture_menu_snapshot() {
    let Some(folder) = std::env::var_os("AXCENITY_SNAPSHOTS") else {
        return;
    };
    let folder = std::path::PathBuf::from(folder);
    let mut world = World::new();
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(world.workspace())).unwrap();
    let mut first = None;
    for preset in layers_for(Preset::MountainRange) {
        let layer = world.create(ClassName::TerrainLayer).unwrap();
        for (name, value) in preset.properties(128.0) {
            world.set_property(layer, name, value).unwrap();
        }
        world.set_name(layer, preset.name).unwrap();
        world.set_parent(layer, Some(terrain)).unwrap();
        first.get_or_insert(layer);
    }
    // A layer with a roughness map, whose slider then scales it.
    world
        .set_property(
            first.unwrap(),
            "RoughnessMap",
            Value::String("assets/textures/Ground108/Ground108_Roughness.jpg".into()),
        )
        .unwrap();
    let pick = |kind, path: &str| TexturePick {
        entry: AssetEntry {
            kind,
            path: path.to_owned(),
            name: crate::asset_browser::display_name(path),
        },
        thumbnail: None,
    };
    let textures = [
        pick(
            AssetKind::TextureSet,
            "assets/textures/Grass001/Grass001.texture.json",
        ),
        pick(
            AssetKind::TextureSet,
            "assets/textures/Ground108/Ground108.texture.json",
        ),
        pick(AssetKind::Texture, "assets/textures/checker.png"),
    ];
    let mut studio = TerrainStudio {
        open_layer: first,
        ..TerrainStudio::default()
    };
    crate::snapshot::snapshot(
        &folder.join("terrain-layer.png"),
        egui::vec2(300.0, 900.0),
        vec![Vec::new(); 3],
        |ui| {
            egui::Panel::right("terrain")
                .default_size(280.0)
                .show(ui, |ui| {
                    studio.show(
                        ui,
                        &StudioContext {
                            world: &world,
                            selected: None,
                            editing: true,
                            time: 0.0,
                            textures: &textures,
                            pictures: &HashMap::new(),
                            world_presets: &[],
                        },
                    );
                });
        },
    );
    let sets: Vec<&TexturePick> = textures.iter().take(2).collect();
    let mut search = String::new();
    crate::snapshot::snapshot(
        &folder.join("terrain-texture-menu.png"),
        egui::vec2(280.0, 160.0),
        vec![Vec::new(); 3],
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let _ = layers::texture_menu(ui, &sets, &mut search, "None");
            });
        },
    );
}

/// A wide panel names the tools beside their icons; a narrow one keeps
/// the icons (names on hover), so it still fits a 260-point column.
#[test]
fn a_wide_panel_names_its_tools() {
    let mut world = World::new();
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(world.workspace())).unwrap();
    let names_drawn = |width: f32| {
        let ctx = crate::theme::test_context();
        let mut studio = TerrainStudio::default();
        let mut output = None;
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                ..Default::default()
            };
            output = Some(crate::theme::run(&ctx, input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    studio.show(
                        ui,
                        &StudioContext {
                            world: &world,
                            selected: None,
                            editing: true,
                            time: 0.0,
                            textures: &[],
                            pictures: &HashMap::new(),
                            world_presets: &[],
                        },
                    );
                });
            }));
        }
        fn walk(shape: &egui::Shape, found: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => found.push(text.galley.text().to_owned()),
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        for clipped in &output.unwrap().shapes {
            walk(&clipped.shape, &mut found);
        }
        found.iter().any(|text| text.contains("Raise / Lower"))
    };
    assert!(names_drawn(600.0));
    assert!(!names_drawn(260.0));
}

/// The texts drawn by the panel at 260 points wide with `selected`.
fn texts_drawn(
    world: &World,
    selected: Option<InstanceId>,
    studio: &mut TerrainStudio,
) -> Vec<String> {
    let ctx = crate::theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        texts = frame(&ctx, world, selected, studio, 260.0, Vec::new());
    }
    texts.into_iter().map(|(text, _)| text).collect()
}

/// Draws the panel `width` points wide (and very tall, so nothing scrolls)
/// with `events`, and returns the texts drawn and where.
fn frame(
    ctx: &egui::Context,
    world: &World,
    selected: Option<InstanceId>,
    studio: &mut TerrainStudio,
    width: f32,
    events: Vec<egui::Event>,
) -> Vec<(String, egui::Rect)> {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width, 2400.0),
        )),
        events,
        ..Default::default()
    };
    let output = crate::theme::run(ctx, input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            studio.show(
                ui,
                &StudioContext {
                    world,
                    selected,
                    editing: true,
                    time: 0.0,
                    textures: &[],
                    pictures: &HashMap::new(),
                    world_presets: &[],
                },
            );
        });
    });
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => found.push((
                text.galley.text().to_owned(),
                text.galley.rect.translate(text.pos.to_vec2()),
            )),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut found);
    }
    found
}

#[test]
fn a_selected_biome_shows_its_settings_and_paint_and_its_rules_say_where_they_are() {
    let mut world = World::new();
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(world.workspace())).unwrap();
    for name in ["Grass", "Forest Floor"] {
        let layer = world.create(ClassName::TerrainLayer).unwrap();
        world.set_parent(layer, Some(terrain)).unwrap();
        world.set_name(layer, name).unwrap();
    }
    let biome = world.create(ClassName::Biome).unwrap();
    world.set_parent(biome, Some(terrain)).unwrap();
    world.set_name(biome, "Lowland Forest").unwrap();
    world
        .set_property(biome, "GroundLayer", Value::String("Forest Floor".into()))
        .unwrap();
    let rule = world.create(ClassName::ScatterRule).unwrap();
    world.set_parent(rule, Some(biome)).unwrap();
    world.set_name(rule, "Trees").unwrap();
    let mut studio = TerrainStudio::default();
    let found = texts_drawn(&world, Some(biome), &mut studio);
    for text in [
        "Biomes",
        "Lowland Forest",
        "Paint this biome",
        "1. Trees (in Lowland Forest)",
    ] {
        assert!(
            found.iter().any(|t| t.contains(text)),
            "{text} in {found:?}"
        );
    }
    assert!(
        found
            .iter()
            .any(|t| t.contains("Forest Floor · any height · 1 rule")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|t| t.contains("Add rule to Lowland Forest"))
    );
    assert_eq!(studio.target, Some(terrain), "a biome selects its terrain");
}

/// A terrain with the mountain range's layers, for the tab tests.
fn layered_terrain() -> (World, InstanceId) {
    let mut world = World::new();
    let terrain = world.create(ClassName::Terrain).unwrap();
    world.set_parent(terrain, Some(world.workspace())).unwrap();
    // Not "Terrain", which the target picker would show above the tabs.
    world.set_name(terrain, "Island").unwrap();
    for preset in layers_for(Preset::MountainRange) {
        let layer = world.create(ClassName::TerrainLayer).unwrap();
        world.set_name(layer, preset.name).unwrap();
        world.set_parent(layer, Some(terrain)).unwrap();
    }
    (world, terrain)
}

#[test]
fn each_tool_is_on_exactly_one_tab() {
    for tool in TerrainTool::all() {
        let tabs: Vec<StudioTab> = StudioTab::ALL
            .into_iter()
            .filter(|tab| tab.tools().contains(&tool))
            .collect();
        assert_eq!(tabs, vec![StudioTab::of(tool)], "{tool:?}");
    }
    let on_tabs: usize = StudioTab::ALL.iter().map(|tab| tab.tools().len()).sum();
    assert_eq!(on_tabs, TerrainTool::all().count());
    assert_eq!(StudioTab::of(TerrainTool::Holes), StudioTab::Sculpt);
    assert_eq!(StudioTab::of(TerrainTool::Paint), StudioTab::Materials);
    assert_eq!(StudioTab::of(TerrainTool::Masks), StudioTab::Terrain);
}

#[test]
fn a_picked_tool_shows_its_tab_and_a_tab_picks_its_last_tool_again() {
    let mut studio = TerrainStudio::default();
    assert_eq!(studio.tab(), StudioTab::Sculpt);
    // No tool held: switching tabs picks nothing.
    studio.switch_tab(StudioTab::Terrain);
    assert_eq!((studio.tab(), studio.tool), (StudioTab::Terrain, None));
    studio.pick(Some(TerrainTool::Auto));
    studio.pick(Some(TerrainTool::Paint));
    assert_eq!(studio.tab(), StudioTab::Materials);
    studio.pick(Some(TerrainTool::Flatten));
    assert_eq!(studio.tab(), StudioTab::Sculpt);
    // With a tool held, a tab picks its last tool, or its first.
    studio.switch_tab(StudioTab::Materials);
    assert_eq!(studio.tool, Some(TerrainTool::Paint));
    studio.switch_tab(StudioTab::Terrain);
    assert_eq!(studio.tool, Some(TerrainTool::Generate));
    studio.switch_tab(StudioTab::Sculpt);
    assert_eq!(studio.tool, Some(TerrainTool::Flatten));
    // Putting the tool down keeps its tab.
    studio.pick(None);
    assert_eq!((studio.tab(), studio.tool), (StudioTab::Sculpt, None));
    // A tool set directly shows its tab at once, and is remembered for it
    // once the panel is drawn.
    let (world, _) = layered_terrain();
    studio.tool = Some(TerrainTool::Erode);
    assert_eq!(studio.tab(), StudioTab::Terrain);
    texts_drawn(&world, None, &mut studio);
    studio.pick(Some(TerrainTool::Raise));
    studio.switch_tab(StudioTab::Terrain);
    assert_eq!(studio.tool, Some(TerrainTool::Erode));
}

#[test]
fn clicking_a_tab_shows_it_and_picks_its_last_tool() {
    let (world, _) = layered_terrain();
    let ctx = crate::theme::test_context();
    let mut studio = TerrainStudio::default();
    studio.pick(Some(TerrainTool::Masks));
    studio.pick(Some(TerrainTool::Smooth));
    let mut texts = Vec::new();
    for _ in 0..2 {
        texts = frame(&ctx, &world, None, &mut studio, 260.0, Vec::new());
    }
    let (_, rect) = texts
        .iter()
        .find(|(text, _)| text == "Terrain")
        .expect("the Terrain tab is drawn")
        .clone();
    let at = rect.center();
    let click = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let moved = vec![egui::Event::PointerMoved(at)];
    frame(&ctx, &world, None, &mut studio, 260.0, moved);
    frame(&ctx, &world, None, &mut studio, 260.0, vec![click(true)]);
    frame(&ctx, &world, None, &mut studio, 260.0, vec![click(false)]);
    assert_eq!(studio.tab(), StudioTab::Terrain);
    assert_eq!(studio.tool, Some(TerrainTool::Masks));
}

#[test]
fn each_section_is_drawn_under_its_tab_only() {
    let (world, _) = layered_terrain();
    // The sections' titles (and Layers' Add layer), and each tab's tools'
    // names, which a wide panel shows.
    let sections: [(StudioTab, &[&str]); 3] = [
        (StudioTab::Sculpt, &["Raise / Lower", "Holes"]),
        (
            StudioTab::Materials,
            &[
                "Paint",
                "Auto Materials",
                "Add layer",
                "Biomes",
                "Scatter Rules",
            ],
        ),
        (
            StudioTab::Terrain,
            // "Water" alone is the Materials tab's Water brush too.
            &[
                "Generate",
                "Masks",
                "Fill the hollows",
                "Terrain data",
                "New ground",
            ],
        ),
    ];
    for (tab, own) in sections {
        let mut studio = TerrainStudio::default();
        studio.switch_tab(tab);
        let ctx = crate::theme::test_context();
        let mut texts = Vec::new();
        for _ in 0..2 {
            texts = frame(&ctx, &world, None, &mut studio, 600.0, Vec::new());
        }
        let drawn = |name: &str| texts.iter().any(|(text, _)| text.contains(name));
        for (other, names) in sections {
            for name in names {
                assert_eq!(drawn(name), other == tab, "{name} on {tab:?}");
            }
        }
        assert!(own.iter().all(|name| drawn(name)));
        // The view is on every tab.
        assert!(drawn("Slope"), "View on {tab:?}");
    }
}

#[test]
fn a_tools_settings_are_under_its_tab() {
    let (world, _) = layered_terrain();
    for (tool, setting) in [
        (TerrainTool::Raise, "Strength"),
        (TerrainTool::Paint, "Strength"),
        (TerrainTool::Water, "Above where the stroke starts"),
        (TerrainTool::Auto, "Rules"),
        (TerrainTool::Generate, "World Presets"),
        (TerrainTool::Erode, "Erosion"),
        (TerrainTool::Masks, "Only between heights"),
    ] {
        let mut studio = TerrainStudio::default();
        studio.pick(Some(tool));
        let texts = texts_drawn(&world, None, &mut studio);
        assert!(
            texts.iter().any(|t| t.ends_with(setting)),
            "{tool:?}: {texts:?}"
        );
    }
}

/// Draws the panel 260 points wide with `events` and returns its actions
/// and the texts drawn, with where.
fn frame_actions(
    ctx: &egui::Context,
    world: &World,
    studio: &mut TerrainStudio,
    events: Vec<egui::Event>,
) -> (Vec<TerrainAction>, Vec<(String, egui::Rect)>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(260.0, 2400.0),
        )),
        events,
        ..Default::default()
    };
    let mut actions = Vec::new();
    let output = crate::theme::run(ctx, input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            actions = studio.show(
                ui,
                &StudioContext {
                    world,
                    selected: None,
                    editing: true,
                    time: 0.0,
                    textures: &[],
                    pictures: &HashMap::new(),
                    world_presets: &[],
                },
            );
        });
    });
    fn walk(shape: &egui::Shape, found: &mut Vec<(String, egui::Rect)>) {
        match shape {
            egui::Shape::Text(text) => found.push((
                text.galley.text().to_owned(),
                text.galley.rect.translate(text.pos.to_vec2()),
            )),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut found);
    }
    (actions, found)
}

#[test]
fn the_tiles_section_adds_a_tile_on_a_free_side_only() {
    use axcenity_terrain::tiles::Side;
    let (mut world, terrain) = layered_terrain();
    let mut studio = TerrainStudio::default();
    studio.switch_tab(StudioTab::Terrain);
    let ctx = crate::theme::test_context();
    let click = |ctx: &egui::Context, world: &World, studio: &mut TerrainStudio, label: &str| {
        let (_, texts) = frame_actions(ctx, world, studio, Vec::new());
        let (_, texts) = if texts.is_empty() {
            frame_actions(ctx, world, studio, Vec::new())
        } else {
            (Vec::new(), texts)
        };
        let at = texts
            .iter()
            .find(|(text, _)| text.ends_with(label))
            .unwrap_or_else(|| panic!("{label} in {texts:?}"))
            .1
            .center();
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut actions = Vec::new();
        for events in [
            vec![egui::Event::PointerMoved(at)],
            vec![button(true)],
            vec![button(false)],
        ] {
            actions.extend(frame_actions(ctx, world, studio, events).0);
        }
        (actions, texts)
    };
    let (actions, texts) = click(&ctx, &world, &mut studio, "+X");
    assert!(
        texts.iter().any(|(text, _)| text.contains("One tile")),
        "{texts:?}"
    );
    assert!(
        actions.contains(&TerrainAction::AddTile(Side::PosX)),
        "{actions:?}"
    );
    // With a tile on +X, its button does nothing.
    let east = world.duplicate(terrain).unwrap();
    world.set_parent(east, Some(world.workspace())).unwrap();
    let size = world
        .get(terrain)
        .unwrap()
        .property("Size")
        .unwrap()
        .as_vector3()
        .unwrap();
    let mut position = world
        .get(terrain)
        .unwrap()
        .property("Position")
        .unwrap()
        .as_vector3()
        .unwrap();
    position.x += size.x;
    world
        .set_property(east, "Position", Value::Vector3(position))
        .unwrap();
    studio.target = Some(terrain);
    let (actions, texts) = click(&ctx, &world, &mut studio, "+X");
    assert!(
        texts.iter().any(|(text, _)| text.contains("2 tiles")),
        "{texts:?}"
    );
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, TerrainAction::AddTile(_))),
        "{actions:?}"
    );
    let (actions, _) = click(&ctx, &world, &mut studio, "−Z");
    assert!(
        actions.contains(&TerrainAction::AddTile(Side::NegZ)),
        "{actions:?}"
    );
}
