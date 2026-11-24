use super::*;

/// Draws the pane 320 points wide (and tall, so nothing scrolls) with
/// `events`, and returns the texts drawn and where, and its actions.
fn frame(
    ctx: &egui::Context,
    pane: &mut ScatterPane,
    terrain: Option<&str>,
    events: Vec<egui::Event>,
) -> (Vec<(String, egui::Rect)>, Vec<ScatterAction>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(320.0, 2400.0),
        )),
        events,
        ..Default::default()
    };
    let mut actions = Vec::new();
    let output = crate::theme::run(ctx, input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            actions = pane.show(
                ui,
                &ScatterContext {
                    editing: true,
                    terrain: terrain.map(str::to_owned),
                    heights: None,
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
    let mut texts = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut texts);
    }
    (texts, actions)
}

fn with_palette() -> ScatterPane {
    let mut pane = ScatterPane::default();
    pane.palettes.push(PaletteEntry {
        file: None,
        palette: ScatterPalette::new(FIRST_PALETTE),
    });
    pane
}

fn rock() -> FoliageSource {
    FoliageSource::Prop(FoliageProp::Rock)
}

#[test]
fn an_item_is_added_once_and_shows_its_name_and_share() {
    let mut pane = with_palette();
    assert!(pane.add(rock()));
    assert!(!pane.add(rock()), "already there");
    assert!(pane.add(FoliageSource::Plant(FoliagePlant::Grass)));
    pane.palette_mut().unwrap().items[0].weight = 3.0;
    assert_eq!(pane.sources().len(), 2);
    assert!(pane.dirty);

    let ctx = crate::theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        texts = frame(&ctx, &mut pane, Some("Terrain"), Vec::new()).0;
    }
    let drawn = |name: &str| texts.iter().any(|(text, _)| text.ends_with(name));
    for name in [
        "Rock",
        "75%",
        "Grass",
        "25%",
        "Weight",
        "Size",
        "Collide",
        "Rotation",
        "Align to slope",
        "Sink",
        "Density",
        "Spacing",
        "Erase everything",
        "Paint",
    ] {
        assert!(drawn(name), "{name} in {texts:?}");
    }
    assert!(!drawn("Scatter paints onto Terrain"));
    // An item that's off has no share.
    pane.palette_mut().unwrap().items[1].enabled = false;
    let texts = frame(&ctx, &mut pane, Some("Terrain"), Vec::new()).0;
    assert!(texts.iter().any(|(text, _)| text == "100%"));
    assert!(texts.iter().any(|(text, _)| text == "0%"));
}

#[test]
fn without_a_terrain_the_pane_says_so_and_paint_is_off() {
    let mut pane = with_palette();
    let ctx = crate::theme::test_context();
    let mut texts = Vec::new();
    for _ in 0..2 {
        texts = frame(&ctx, &mut pane, None, Vec::new()).0;
    }
    assert!(
        texts
            .iter()
            .any(|(text, _)| text == "Scatter paints onto Terrain"),
        "{texts:?}"
    );
    // Paint can't be clicked.
    let (_, rect) = texts
        .iter()
        .find(|(text, _)| text.ends_with("Paint"))
        .unwrap()
        .clone();
    let click = |pressed| egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(
        &ctx,
        &mut pane,
        None,
        vec![egui::Event::PointerMoved(rect.center())],
    );
    frame(&ctx, &mut pane, None, vec![click(true)]);
    let (_, actions) = frame(&ctx, &mut pane, None, vec![click(false)]);
    assert!(
        !actions.contains(&ScatterAction::Paint(true)),
        "{actions:?}"
    );
}

#[test]
fn editing_an_item_marks_the_palette_to_save_and_keeps_size_in_order() {
    let mut pane = with_palette();
    pane.add(rock());
    pane.dirty = false;
    let ctx = crate::theme::test_context();
    frame(&ctx, &mut pane, Some("Terrain"), Vec::new());
    assert!(!pane.dirty, "drawing alone changes nothing");
    let item = &mut pane.palette_mut().unwrap().items[0];
    item.min_scale = 3.0;
    item.max_scale = 1.0;
    frame(&ctx, &mut pane, Some("Terrain"), Vec::new());
    let item = &pane.palette().unwrap().items[0];
    assert_eq!((item.min_scale, item.max_scale), (1.0, 3.0));
    assert!(pane.dirty);
}

#[test]
fn new_names_are_unique_ignoring_case() {
    let entry = |name: &str| PaletteEntry {
        file: None,
        palette: ScatterPalette::new(name),
    };
    let palettes = vec![entry("Palette"), entry("palette 2"), entry("Rocks")];
    assert_eq!(unique_name(&palettes, "Palette", None), "Palette 3");
    assert_eq!(unique_name(&palettes, "ROCKS", None), "ROCKS 2");
    // Renaming one to its own name keeps it.
    assert_eq!(unique_name(&palettes, "rocks", Some(2)), "rocks");
    assert_eq!(unique_name(&palettes, "  ", None), "Palette 3");
    assert_eq!(unique_name(&palettes, "Beach", None), "Beach");
}

#[test]
fn items_fit_a_narrow_column() {
    let mut pane = with_palette();
    pane.add(rock());
    pane.add(FoliageSource::Model(
        "assets/models/Tall Fern Clump.glb".into(),
    ));
    pane.painting = true;
    pane.picking = true;
    let ctx = crate::theme::test_context();
    let mut fit = None;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(260.0, 2400.0),
            )),
            ..Default::default()
        };
        let _ = crate::theme::run(&ctx, input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let room = ui.available_rect_before_wrap();
                pane.show(
                    ui,
                    &ScatterContext {
                        editing: true,
                        terrain: Some("Terrain".into()),
                        heights: None,
                    },
                );
                fit = Some((ui.min_rect(), room));
            });
        });
    }
    let (used, room) = fit.unwrap();
    assert!(
        used.width() <= room.width() + 0.5,
        "drew {} wide in {}",
        used.width(),
        room.width()
    );
}

#[test]
fn masks_and_decals_fit_a_narrow_column() {
    let mut pane = with_palette();
    pane.add(rock());
    pane.palette_mut().unwrap().items[0].mask = ScatterMask {
        noise: 0.8,
        min_slope: 2.0,
        max_slope: 25.0,
        min_height: 0.0,
        max_height: 40.0,
        moisture: 0.6,
        ..ScatterMask::default()
    };
    assert!(pane.add_decal("assets/textures/oil stain.png", 1.0));
    assert!(
        !pane.add_decal("assets/textures/oil stain.png", 1.0),
        "already there"
    );
    assert!(pane.add_decal("assets/textures/long tyre tracks.png", 0.25));
    assert_eq!(pane.decal_textures().len(), 2);
    assert_eq!(decal_name("assets/textures/oil stain.png"), "oil stain");
    pane.painting = true;
    pane.decals = true;
    pane.picking_decals = true;
    let ctx = crate::theme::test_context();
    // Open the item's Masks (folded until clicked).
    let mut texts = Vec::new();
    for _ in 0..2 {
        texts = frame(&ctx, &mut pane, Some("Terrain"), Vec::new()).0;
    }
    let masks = texts
        .iter()
        .find(|(text, _)| text.starts_with("Masks (4 on)"))
        .unwrap_or_else(|| panic!("the masks that are on are named: {texts:?}"))
        .1
        .center();
    let press = |pressed| egui::Event::PointerButton {
        pos: masks,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let open = vec![
        vec![egui::Event::PointerMoved(masks)],
        vec![press(true)],
        vec![press(false)],
        Vec::new(),
        Vec::new(),
    ];
    for events in open.clone() {
        texts = frame(&ctx, &mut pane, Some("Terrain"), events).0;
    }
    // It unfolds over a few frames.
    for _ in 0..30 {
        texts = frame(&ctx, &mut pane, Some("Terrain"), Vec::new()).0;
    }
    let drawn = |label: &str| texts.iter().any(|(text, _)| text == label);
    for label in [
        "Noise",
        "Scale",
        "Threshold",
        "Contrast",
        "Seed",
        "Slope",
        "Height",
        "Moisture",
        "Reach",
        "Decals",
        "Opacity",
        "Land on parts",
        "For Medium quality",
    ] {
        assert!(drawn(label), "`{label}` isn't drawn: {texts:?}");
    }
    // Paint decals is armed, and says how to use it.
    assert!(
        texts
            .iter()
            .any(|(text, _)| text.ends_with("Stop painting decals"))
    );
    assert!(
        texts
            .iter()
            .any(|(text, _)| text.starts_with("Drag to lay decals"))
    );

    // At 260 points wide nothing runs past the column.
    let mut fit = None;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(260.0, 3200.0),
            )),
            ..Default::default()
        };
        let _ = crate::theme::run(&ctx, input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let room = ui.available_rect_before_wrap();
                pane.show(
                    ui,
                    &ScatterContext {
                        editing: true,
                        terrain: Some("Terrain".into()),
                        heights: Some((0.0, 64.0)),
                    },
                );
                fit = Some((ui.min_rect(), room));
            });
        });
    }
    let (used, room) = fit.unwrap();
    assert!(
        used.width() <= room.width() + 0.5,
        "drew {} wide in {}",
        used.width(),
        room.width()
    );

    // A picture to look at, when asked for.
    if let Some(folder) = std::env::var_os("AXCENITY_SNAPSHOTS") {
        let mut pane = with_palette();
        pane.add(rock());
        pane.palettes[0].palette = self::ScatterPalette {
            decals: pane_decals(),
            ..pane.palettes[0].palette.clone()
        };
        pane.palette_mut().unwrap().items[0].mask = ScatterMask {
            noise: 0.8,
            min_slope: 2.0,
            max_slope: 25.0,
            min_height: 0.0,
            max_height: 40.0,
            moisture: 0.6,
            ..ScatterMask::default()
        };
        pane.painting = true;
        pane.decals = true;
        crate::snapshot::snapshot(
            &std::path::PathBuf::from(folder).join("scatter-pane-masks-decals.png"),
            egui::vec2(320.0, 1500.0),
            open,
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    pane.show(
                        ui,
                        &ScatterContext {
                            editing: true,
                            terrain: Some("Terrain".into()),
                            heights: Some((0.0, 64.0)),
                        },
                    );
                });
            },
        );
    }
}

/// Two decals, for the snapshot.
fn pane_decals() -> Vec<ScatterDecal> {
    vec![
        ScatterDecal::new("assets/textures/oil stain.png"),
        ScatterDecal {
            aspect: 0.25,
            rotation: 0.0,
            ..ScatterDecal::new("assets/textures/long tyre tracks.png")
        },
    ]
}
