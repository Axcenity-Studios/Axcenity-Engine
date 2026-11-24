//! The Scatter pane in the editor: palettes saved per project, items
//! picked from the Assets panel, and strokes as one undo step each.

use std::path::PathBuf;

use axcenity_core::{FoliageProp, FoliageSource};
use axcenity_terrain::brush::ScatterItem;

use super::*;
use crate::terrain_studio::{TerrainAction, TerrainTool};

fn app_with_project(name: &str) -> (EditorApp, PathBuf, PathBuf) {
    let parent = std::env::temp_dir().join(format!(
        "axcenity-editor-scatter-pane-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let folder = axcenity_project::create_project(&parent, "Scatter Pane Test").unwrap();
    let app = EditorApp::new(Some(&folder));
    assert!(app.project.is_some(), "the project opens");
    (app, parent, folder)
}

fn create_terrain(app: &mut EditorApp) -> InstanceId {
    app.terrain_action(TerrainAction::Create {
        width: 128.0,
        length: 128.0,
        height: 32.0,
        samples: 65,
    });
    app.terrain.target.expect("the new terrain is the target")
}

fn pointer(point: Vec3, pressed: bool, down: bool, released: bool, shift: bool) -> TerrainPointer {
    TerrainPointer {
        point: Some(point),
        pressed,
        down,
        released,
        modifiers: egui::Modifiers {
            shift,
            ..Default::default()
        },
        seconds: 1.0 / 60.0,
    }
}

/// A click-and-drag stroke from `from` to `to`.
fn stroke(app: &mut EditorApp, from: Vec3, to: Vec3, shift: bool) {
    app.scatter_pointer(pointer(from, true, true, false, shift));
    for step in 1..=8 {
        let at = from + (to - from) * (step as f32 / 8.0);
        app.scatter_pointer(pointer(at, false, true, false, shift));
    }
    app.scatter_pointer(pointer(to, false, false, true, shift));
}

/// The terrain's painted foliage, in a fixed order (undo puts chunks back
/// in their own order).
fn foliage(app: &EditorApp, terrain: InstanceId) -> Vec<axcenity_core::FoliageInstance> {
    let mut list = app.world.terrain(terrain).unwrap().foliage().to_vec();
    list.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.z.total_cmp(&b.z)));
    list
}

/// Draws the Scatter pane beside the Assets panel with `events`, then runs
/// the pane's per-frame work, and returns the texts drawn and where.
fn panes_frame(
    app: &mut EditorApp,
    ctx: &egui::Context,
    clock: &mut f64,
    events: Vec<egui::Event>,
) -> Vec<(String, egui::Rect)> {
    *clock += 0.05;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1400.0, 2000.0),
        )),
        events,
        time: Some(*clock),
        ..Default::default()
    };
    let output = crate::theme::run(ctx, input, |ui| {
        egui::Panel::left("scatter")
            .exact_size(340.0)
            .show(ui, |ui| app.scatter_pane(ui));
        egui::CentralPanel::default().show(ui, |ui| app.assets_pane(ui));
        let ctx = ui.ctx().clone();
        app.scatter_frame(&ctx);
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
    texts
}

/// Clicks the text drawn last frame that ends with `label` (an icon may
/// come first), in the `x` range given.
fn click_text(
    app: &mut EditorApp,
    ctx: &egui::Context,
    clock: &mut f64,
    texts: &[(String, egui::Rect)],
    label: &str,
    x: std::ops::Range<f32>,
) -> Vec<(String, egui::Rect)> {
    let (_, rect) = texts
        .iter()
        .find(|(text, rect)| text.ends_with(label) && x.contains(&rect.center().x))
        .unwrap_or_else(|| panic!("`{label}` is drawn: {texts:?}"))
        .clone();
    let at = rect.center();
    let press = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    panes_frame(app, ctx, clock, vec![egui::Event::PointerMoved(at)]);
    panes_frame(app, ctx, clock, vec![press(true)]);
    panes_frame(app, ctx, clock, vec![press(false)])
}

#[test]
fn the_ribbon_command_opens_the_pane_and_a_new_project_starts_with_an_empty_palette() {
    let (mut app, parent, _) = app_with_project("open");
    assert!(!app.layout.is_open(Pane::Scatter));
    let ctx = crate::theme::test_context();
    app.run_command(&ctx, super::super::command_palette::Command::Scatter);
    assert!(app.layout.is_open(Pane::Scatter));
    let commands = app.palette_commands(&ctx);
    let scatter = commands
        .iter()
        .find(|c| c.command == super::super::command_palette::Command::Scatter)
        .expect("universal search lists Scatter");
    assert_eq!(scatter.entry.title, "Scatter");
    app.refresh_scatter_palettes();
    let names: Vec<&str> = app
        .scatter
        .palettes
        .iter()
        .map(|e| e.palette.name.as_str())
        .collect();
    assert_eq!(names, ["Palette"]);
    assert!(app.scatter.palette().unwrap().items.is_empty());
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn clicking_assets_while_adding_fills_the_palette_and_it_saves_with_the_project() {
    let (mut app, parent, folder) = app_with_project("pick");
    create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    let ctx = crate::theme::test_context();
    let mut clock = 0.0;
    let mut texts = Vec::new();
    for _ in 0..3 {
        texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    // Add from Assets, in the pane on the left.
    texts = click_text(
        &mut app,
        &ctx,
        &mut clock,
        &texts,
        "Add from Assets",
        0.0..340.0,
    );
    assert!(app.scatter.picking);
    assert!(app.scatter_picking());
    // A prop tile in the Assets panel, on the right.
    texts = click_text(&mut app, &ctx, &mut clock, &texts, "Tree", 340.0..1400.0);
    let sources = app.scatter.sources();
    assert_eq!(sources, [FoliageSource::Prop(FoliageProp::Tree)]);
    click_text(&mut app, &ctx, &mut clock, &texts, "Crate", 340.0..1400.0);
    assert_eq!(app.scatter.sources().len(), 2);
    // Saved once the mouse is let go.
    panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    assert!(!app.scatter.dirty);
    let file = folder.join("presets").join("Palette.axscatter");
    let saved = axcenity_project::ScatterPalette::from_json(
        &std::fs::read_to_string(&file).expect("the palette is saved"),
    )
    .unwrap();
    assert_eq!(saved.items.len(), 2);

    // Opened again, the project has it.
    let mut again = EditorApp::new(Some(&folder));
    again.refresh_scatter_palettes();
    assert_eq!(again.scatter.palette().unwrap(), &saved);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn a_stroke_scatters_the_mix_as_one_undo_step_and_shift_erases() {
    let (mut app, parent, _) = app_with_project("stroke");
    let terrain = create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    let rock = FoliageSource::Prop(FoliageProp::Rock);
    let crate_prop = FoliageSource::Prop(FoliageProp::Crate);
    {
        let palette = app.scatter.palette_mut().unwrap();
        palette.density = 40.0;
        palette.spacing = 0.5;
        palette.items = vec![
            ScatterItem {
                collide: true,
                min_scale: 2.0,
                max_scale: 2.5,
                ..ScatterItem::new(rock.clone())
            },
            ScatterItem::new(crate_prop.clone()),
        ];
    }
    app.scatter.radius = 10.0;
    app.terrain.pick(Some(TerrainTool::Raise));
    app.scatter_action(ScatterAction::Paint(true));
    assert!(app.scatter.painting);
    assert_eq!(app.terrain.tool, None, "one brush at a time");
    let ctx = crate::theme::test_context();
    assert!(app.scatter_brush_view(&ctx).is_some());

    stroke(
        &mut app,
        Vec3::new(-20.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        false,
    );
    let placed = foliage(&app, terrain);
    assert!(placed.len() > 20, "placed {}", placed.len());
    assert!(placed.iter().any(|i| i.source == rock));
    assert!(placed.iter().any(|i| i.source == crate_prop));
    for instance in placed.iter().filter(|i| i.source == rock) {
        assert!(instance.collide && (2.0..=2.5).contains(&instance.scale));
    }
    assert_eq!(
        app.history.undo_label().map(str::to_owned),
        Some(format!("Scatter {} Items", placed.len()))
    );
    app.undo();
    assert!(
        foliage(&app, terrain).is_empty(),
        "the whole stroke undoes at once"
    );
    app.redo();
    assert_eq!(foliage(&app, terrain), placed);

    // Shift erases only the palette's items...
    app.world
        .terrain_mut(terrain)
        .unwrap()
        .foliage_mut()
        .push(axcenity_core::FoliageInstance {
            source: FoliageSource::Model("assets/models/Fern.glb".into()),
            ..placed[0].clone()
        });
    stroke(
        &mut app,
        Vec3::new(-30.0, 0.0, 0.0),
        Vec3::new(30.0, 0.0, 0.0),
        true,
    );
    assert_eq!(app.history.undo_label(), Some("Erase Scatter"));
    assert_eq!(foliage(&app, terrain).len(), 1, "the fern stays");
    // ...or, with Erase everything, any painted foliage.
    app.scatter.erase_everything = true;
    stroke(
        &mut app,
        Vec3::new(-30.0, 0.0, 0.0),
        Vec3::new(30.0, 0.0, 0.0),
        true,
    );
    assert!(foliage(&app, terrain).is_empty());
    app.undo();
    assert_eq!(foliage(&app, terrain).len(), 1);

    // Picking a terrain tool puts the brush down.
    app.terrain.pick(Some(TerrainTool::Paint));
    app.scatter_frame(&ctx);
    assert!(!app.scatter.painting);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn escape_puts_the_brush_down_and_closing_the_pane_stops_it() {
    let (mut app, parent, _) = app_with_project("escape");
    create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    let ctx = crate::theme::test_context();
    app.scatter_action(ScatterAction::Paint(true));
    let input = egui::RawInput {
        events: vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        ..Default::default()
    };
    let _ = crate::theme::run(&ctx, input, |ui| {
        let ctx = ui.ctx().clone();
        app.scatter_frame(&ctx);
    });
    assert!(!app.scatter.painting);
    app.scatter_action(ScatterAction::Paint(true));
    app.scatter_action(ScatterAction::Pick(true));
    app.layout.toggle(Pane::Scatter);
    app.scatter_frame(&ctx);
    assert!(!app.scatter.painting && !app.scatter.picking);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn without_terrain_the_brush_stays_down() {
    let (mut app, parent, _) = app_with_project("no-terrain");
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    app.scatter_action(ScatterAction::Paint(true));
    assert!(!app.scatter.painting);
    let ctx = crate::theme::test_context();
    assert!(app.scatter_brush_view(&ctx).is_none());
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn palettes_are_made_renamed_and_deleted_as_files() {
    let (mut app, parent, folder) = app_with_project("files");
    app.refresh_scatter_palettes();
    let presets = folder.join("presets");
    app.add_scatter_item(FoliageSource::Prop(FoliageProp::Rock));
    app.save_scatter_palette();
    assert!(presets.join("Palette.axscatter").is_file());

    app.scatter_action(ScatterAction::New);
    assert_eq!(app.scatter.current, 1);
    assert!(presets.join("Palette 2.axscatter").is_file());
    assert!(app.scatter.palette().unwrap().items.is_empty());

    app.scatter_action(ScatterAction::Rename("Beach Rocks".into()));
    assert!(presets.join("Beach Rocks.axscatter").is_file());
    assert!(!presets.join("Palette 2.axscatter").exists());
    // A taken name gets a number.
    app.scatter_action(ScatterAction::Rename("palette".into()));
    assert_eq!(app.scatter.palette().unwrap().name, "palette 2");

    app.scatter_action(ScatterAction::Select(0));
    assert_eq!(app.scatter.palette().unwrap().items.len(), 1);
    app.scatter_action(ScatterAction::Delete);
    assert!(!presets.join("Palette.axscatter").exists());
    assert_eq!(app.scatter.palettes.len(), 1);
    app.scatter_action(ScatterAction::Delete);
    // The last one gone, an empty one takes its place, unsaved until edited.
    assert_eq!(app.scatter.palettes.len(), 1);
    assert_eq!(app.scatter.palette().unwrap().name, "Palette");
    let left: Vec<_> = std::fs::read_dir(&presets)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "axscatter"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    std::fs::remove_dir_all(parent).unwrap();
}

/// The ProjectedDecals in the Workspace's "Scattered Decals" Folder.
fn scattered_decals(app: &EditorApp) -> Vec<InstanceId> {
    let workspace = app.world.get(app.world.workspace()).unwrap();
    workspace
        .children()
        .iter()
        .copied()
        .filter(|&child| {
            let folder = app.world.get(child).unwrap();
            folder.class() == ClassName::Folder && folder.name() == "Scattered Decals"
        })
        .flat_map(|folder| app.world.get(folder).unwrap().children().to_vec())
        .filter(|&id| app.world.get(id).unwrap().class() == ClassName::ProjectedDecal)
        .collect()
}

fn decal_position(app: &EditorApp, id: InstanceId) -> Vec3 {
    let at = app
        .world
        .get(id)
        .unwrap()
        .property("Position")
        .and_then(Value::as_vector3)
        .unwrap();
    Vec3::new(at.x, at.y, at.z)
}

#[test]
fn masks_are_shown_folded_per_item_and_a_masked_stroke_places_only_where_they_allow() {
    let (mut app, parent, folder) = app_with_project("masks");
    let terrain = create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    app.add_scatter_item(FoliageSource::Prop(FoliageProp::Rock));
    let ctx = crate::theme::test_context();
    let mut clock = 0.0;
    let mut texts = Vec::new();
    for _ in 0..3 {
        texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    let drawn = |texts: &[(String, egui::Rect)], label: &str| {
        texts
            .iter()
            .any(|(text, rect)| text.ends_with(label) && rect.center().x < 340.0)
    };
    // Folded away until opened: the item's card isn't any longer for them.
    assert!(drawn(&texts, "Masks"), "{texts:?}");
    assert!(!drawn(&texts, "Moisture"));
    texts = click_text(&mut app, &ctx, &mut clock, &texts, "Masks", 0.0..340.0);
    for _ in 0..12 {
        texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    for label in ["Noise", "Slope", "Height", "Moisture"] {
        assert!(drawn(&texts, label), "{label} isn't drawn: {texts:?}");
    }
    // Noise's and Moisture's own settings show only once they're on.
    assert!(!drawn(&texts, "Threshold") && !drawn(&texts, "Reach"));
    {
        let mask = &mut app.scatter.palette_mut().unwrap().items[0].mask;
        mask.noise = 0.7;
        mask.moisture = 0.5;
    }
    for _ in 0..3 {
        texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    for label in ["Scale", "Threshold", "Contrast", "Seed", "Reach"] {
        assert!(drawn(&texts, label), "{label} isn't drawn: {texts:?}");
    }
    assert!(drawn(&texts, "Masks (2 on)"), "{texts:?}");
    // No text runs out of the pane.
    for (text, rect) in texts.iter().filter(|(_, rect)| rect.min.x < 340.0) {
        assert!(rect.max.x <= 340.0, "`{text}` is cut off at {rect:?}");
    }

    // The masks save with the palette, and read back the same.
    app.scatter.dirty = true;
    app.save_scatter_palette();
    let file = folder.join("presets").join("Palette.axscatter");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("\"mask\""), "{text}");
    let saved = axcenity_project::ScatterPalette::from_json(&text).unwrap();
    assert_eq!(saved.items[0].mask.noise, 0.7);
    assert_eq!(saved.items[0].mask.moisture, 0.5);

    // A height mask above the whole terrain: a stroke places nothing, and
    // makes no undo step.
    {
        let palette = app.scatter.palette_mut().unwrap();
        palette.density = 40.0;
        palette.items[0].mask = axcenity_terrain::brush::ScatterMask {
            min_height: 5000.0,
            ..Default::default()
        };
    }
    app.scatter_action(ScatterAction::Paint(true));
    app.history.seal();
    let steps = app.history.undo_label().map(str::to_owned);
    stroke(
        &mut app,
        Vec3::new(-20.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        false,
    );
    assert!(foliage(&app, terrain).is_empty());
    assert_eq!(app.history.undo_label().map(str::to_owned), steps);
    // Opened up again, the same stroke places rocks.
    app.scatter.palette_mut().unwrap().items[0].mask = Default::default();
    stroke(
        &mut app,
        Vec3::new(-20.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        false,
    );
    assert!(foliage(&app, terrain).len() > 20);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn the_ground_clutter_preset_makes_a_palette_of_small_rocks_and_saves_it() {
    let (mut app, parent, folder) = app_with_project("clutter");
    let terrain = create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    app.scatter_action(ScatterAction::NewGroundClutter);
    let palette = app.scatter.palette().unwrap().clone();
    assert_eq!(palette.name, "Ground Clutter");
    assert_eq!(palette.items.len(), 3);
    assert!(
        folder
            .join("presets")
            .join("Ground Clutter.axscatter")
            .is_file()
    );
    // A second one gets a number.
    app.scatter_action(ScatterAction::NewGroundClutter);
    assert_eq!(app.scatter.palette().unwrap().name, "Ground Clutter 2");
    // A stroke with it: small rocks, lying to the slope and sunk.
    app.scatter.radius = 6.0;
    app.scatter_action(ScatterAction::Paint(true));
    stroke(
        &mut app,
        Vec3::new(-10.0, 0.0, 0.0),
        Vec3::new(10.0, 0.0, 0.0),
        false,
    );
    let placed = foliage(&app, terrain);
    assert!(placed.len() > 50, "placed {}", placed.len());
    for instance in &placed {
        assert_eq!(instance.source, FoliageSource::Prop(FoliageProp::Rock));
        assert!(instance.scale <= 0.25 && instance.lean == 1.0 && instance.sink > 0.0);
        assert!(!instance.collide);
    }
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn pictures_clicked_in_assets_become_the_palettes_decals_and_save() {
    let parent = std::env::temp_dir().join(format!(
        "axcenity-editor-scatter-pane-decal-pick-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).unwrap();
    let folder = axcenity_project::create_project(&parent, "Scatter Pane Test").unwrap();
    // A track four times as long as it's wide, and a square stain.
    std::fs::create_dir_all(folder.join("assets/textures")).unwrap();
    let picture = |width, height| image::RgbaImage::from_pixel(width, height, image::Rgba([40; 4]));
    picture(16, 64)
        .save(folder.join("assets/textures/tyretrack.png"))
        .unwrap();
    picture(32, 32)
        .save(folder.join("assets/textures/oilstain.png"))
        .unwrap();
    let mut app = EditorApp::new(Some(&folder));
    create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    let ctx = crate::theme::test_context();
    let mut clock = 0.0;
    let mut texts = Vec::new();
    for _ in 0..3 {
        texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    let in_pane = |texts: &[(String, egui::Rect)], label: &str| {
        texts
            .iter()
            .any(|(text, rect)| text.ends_with(label) && rect.center().x < 340.0)
    };
    for label in ["Decals", "Paint decals", "Land on parts"] {
        assert!(in_pane(&texts, label), "{label} isn't drawn: {texts:?}");
    }
    assert!(in_pane(&texts, "For Medium quality"));
    click_text(
        &mut app,
        &ctx,
        &mut clock,
        &texts,
        "Add pictures from Assets",
        0.0..340.0,
    );
    assert!(app.scatter.picking_decals && app.scatter_decal_picking());
    // One kind of picking at a time.
    app.scatter_action(ScatterAction::Pick(true));
    assert!(!app.scatter.picking_decals);
    app.scatter_action(ScatterAction::PickDecals(true));
    assert!(!app.scatter.picking && app.scatter.picking_decals);
    texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    // A prop isn't a picture: clicking one adds nothing.
    texts = click_text(&mut app, &ctx, &mut clock, &texts, "Crate", 340.0..1400.0);
    assert!(app.scatter.palette().unwrap().items.is_empty());
    assert!(app.scatter.palette().unwrap().decals.is_empty());
    texts = click_text(
        &mut app,
        &ctx,
        &mut clock,
        &texts,
        "tyretrack",
        340.0..1400.0,
    );
    texts = click_text(
        &mut app,
        &ctx,
        &mut clock,
        &texts,
        "oilstain",
        340.0..1400.0,
    );
    // Clicking one again doesn't add it twice.
    texts = click_text(
        &mut app,
        &ctx,
        &mut clock,
        &texts,
        "oilstain",
        340.0..1400.0,
    );
    let decals = app.scatter.palette().unwrap().decals.clone();
    assert_eq!(
        decals
            .iter()
            .map(|d| d.texture.as_str())
            .collect::<Vec<_>>(),
        [
            "assets/textures/tyretrack.png",
            "assets/textures/oilstain.png"
        ]
    );
    assert_eq!((decals[0].aspect, decals[1].aspect), (0.25, 1.0));
    // Each has its card in the pane, by its file's name.
    for _ in 0..2 {
        texts = panes_frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    assert!(in_pane(&texts, "tyretrack") && in_pane(&texts, "Opacity"));
    for (text, rect) in texts.iter().filter(|(_, rect)| rect.min.x < 340.0) {
        assert!(rect.max.x <= 340.0, "`{text}` is cut off at {rect:?}");
    }
    assert!(!app.scatter.dirty, "saved once the mouse is let go");
    let saved = axcenity_project::ScatterPalette::from_json(
        &std::fs::read_to_string(folder.join("presets").join("Palette.axscatter")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved.decals, decals);
    let mut again = EditorApp::new(Some(&folder));
    again.refresh_scatter_palettes();
    assert_eq!(again.scatter.palette().unwrap().decals, decals);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn a_decal_stroke_lays_projected_decals_as_one_undo_step_and_shift_takes_them_away() {
    let (mut app, parent, _) = app_with_project("decal-stroke");
    let terrain = create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    assert!(app.scatter.add_decal("assets/textures/stain.png", 1.0));
    assert!(app.scatter.add_decal("assets/textures/track.png", 0.25));
    {
        let palette = app.scatter.palette_mut().unwrap();
        palette.decal_density = 6.0;
        palette.decal_spacing = 1.5;
        palette.decals[1].rotation = 0.0;
    }
    app.scatter.radius = 8.0;
    app.terrain.pick(Some(TerrainTool::Raise));
    app.scatter_action(ScatterAction::PaintDecals(true));
    assert!(app.scatter.painting && app.scatter.decals);
    assert_eq!(app.terrain.tool, None, "one brush at a time");
    let ctx = crate::theme::test_context();
    assert!(app.scatter_brush_view(&ctx).is_some());
    let placement = axcenity_terrain::Placement::of(app.world.get(terrain).unwrap()).unwrap();
    let ground = placement
        .ground_y(app.world.terrain(terrain).unwrap(), 0.0, 0.0)
        .unwrap();
    app.history.seal();
    let before_instances = app.world.descendants(app.world.root()).len();

    let (from, to) = (Vec3::new(-20.0, ground, 0.0), Vec3::new(20.0, ground, 0.0));
    stroke(&mut app, from, to, false);
    let placed = scattered_decals(&app);
    assert!(placed.len() > 10, "placed {}", placed.len());
    assert!(foliage(&app, terrain).is_empty(), "decals, not items");
    assert_eq!(
        app.history.undo_label().map(str::to_owned),
        Some(format!("Scatter {} Decals", placed.len()))
    );
    let mut textures = std::collections::BTreeSet::new();
    for &id in &placed {
        let decal = app.world.get(id).unwrap();
        let texture = decal.property("Texture").and_then(Value::as_str).unwrap();
        textures.insert(texture.to_owned());
        // On the ground, under the brush's path.
        let at = decal_position(&app, id);
        assert!((at.y - ground).abs() < 1e-3, "{at}");
        assert!(at.x.abs() <= 28.0 && at.z.abs() <= 8.0, "{at}");
        // Lying flat (the ground is), faint or strong within its range.
        let facing = decal
            .property("Orientation")
            .and_then(Value::as_vector3)
            .unwrap();
        assert!((facing.x - 90.0).abs() < 0.01, "{facing:?}");
        let size = decal.property("Size").and_then(Value::as_vector3).unwrap();
        assert!((3.0..=6.0 + 1e-4).contains(&size.x.max(size.y)), "{size:?}");
        if texture.ends_with("track.png") {
            assert!((size.x * 4.0 - size.y).abs() < 1e-3, "{size:?}");
        }
        let transparency = decal
            .property("Transparency")
            .and_then(Value::as_number)
            .unwrap();
        assert!(
            (0.1 - 1e-4..=0.4 + 1e-4).contains(&transparency),
            "{transparency}"
        );
        assert_eq!(
            decal.name(),
            if texture.ends_with("track.png") {
                "track"
            } else {
                "stain"
            }
        );
    }
    assert_eq!(textures.len(), 2, "both pictures were laid: {textures:?}");
    // No two closer than the spacing.
    for (index, &a) in placed.iter().enumerate() {
        for &b in &placed[index + 1..] {
            let apart = decal_position(&app, a).distance(decal_position(&app, b));
            assert!(apart >= 1.5, "{apart}");
        }
    }
    // One undo takes the Folder and every decal; redo puts them back.
    app.undo();
    assert!(scattered_decals(&app).is_empty());
    assert_eq!(
        app.world.descendants(app.world.root()).len(),
        before_instances
    );
    app.redo();
    assert_eq!(scattered_decals(&app).len(), placed.len());

    // Shift+drag takes away the scattered decals under the brush, and
    // leaves a decal placed by hand alone.
    let own = app.world.create(ClassName::ProjectedDecal).unwrap();
    let workspace = app.world.workspace();
    app.world.set_parent(own, Some(workspace)).unwrap();
    let _ = app.world.set_property(
        own,
        "Position",
        Value::Vector3(axcenity_core::Vector3::new(0.0, ground, 0.0)),
    );
    app.history.seal();
    let all = scattered_decals(&app).len();
    stroke(
        &mut app,
        Vec3::new(-30.0, ground, 0.0),
        Vec3::new(0.0, ground, 0.0),
        true,
    );
    let left = scattered_decals(&app);
    assert!(
        left.len() < all && !left.is_empty(),
        "{} of {all}",
        left.len()
    );
    assert!(left.iter().all(|&id| decal_position(&app, id).x > 0.0));
    assert!(app.world.get(own).is_some(), "a decal placed by hand stays");
    assert_eq!(
        app.history.undo_label().map(str::to_owned),
        Some(format!("Erase {} Decals", all - left.len()))
    );
    app.undo();
    assert_eq!(scattered_decals(&app).len(), all);

    // A stroke with no pictures switched on changes nothing and makes no step.
    for decal in &mut app.scatter.palette_mut().unwrap().decals {
        decal.enabled = false;
    }
    let label = app.history.undo_label().map(str::to_owned);
    stroke(
        &mut app,
        Vec3::new(-20.0, ground, 30.0),
        Vec3::new(20.0, ground, 30.0),
        false,
    );
    assert_eq!(scattered_decals(&app).len(), all);
    assert_eq!(app.history.undo_label().map(str::to_owned), label);

    // Paint goes back to items; Escape or closing the pane puts it down.
    app.scatter_action(ScatterAction::Paint(true));
    assert!(app.scatter.painting && !app.scatter.decals);
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn the_decal_brush_stops_at_the_renderers_limit_and_lands_on_parts_when_asked() {
    let (mut app, parent, _) = app_with_project("decal-budget");
    let terrain = create_terrain(&mut app);
    app.layout.open(Pane::Scatter);
    app.refresh_scatter_palettes();
    app.scatter.add_decal("assets/textures/stain.png", 1.0);
    {
        let palette = app.scatter.palette_mut().unwrap();
        palette.decal_density = 50.0;
        palette.decal_spacing = 0.0;
    }
    app.scatter.radius = 12.0;
    app.scatter_action(ScatterAction::PaintDecals(true));
    let placement = axcenity_terrain::Placement::of(app.world.get(terrain).unwrap()).unwrap();
    let ground = placement
        .ground_y(app.world.terrain(terrain).unwrap(), 0.0, 0.0)
        .unwrap();
    // Medium quality draws the nearest 128 within 150 studs: on this
    // 128-stud terrain every decal is within that of every other, so the
    // brush lays 128 and no more, and says so.
    let budget = app.scatter.decal_budget();
    assert_eq!((budget.count, budget.distance), (128, 150.0));
    for z in [-30.0, 0.0, 30.0] {
        stroke(
            &mut app,
            Vec3::new(-40.0, ground, z),
            Vec3::new(40.0, ground, z),
            false,
        );
    }
    assert_eq!(scattered_decals(&app).len(), 128);
    let said = app
        .output
        .lines()
        .iter()
        .rev()
        .find(|line| line.text.starts_with("Left out"))
        .expect("the brush says what it left out")
        .text
        .clone();
    assert!(
        said.contains("128 projected decals already lie within 150 studs"),
        "{said}"
    );
    // Full: another stroke adds none and makes no undo step.
    let label = app.history.undo_label().map(str::to_owned);
    stroke(
        &mut app,
        Vec3::new(-40.0, ground, 45.0),
        Vec3::new(40.0, ground, 45.0),
        false,
    );
    assert_eq!(scattered_decals(&app).len(), 128);
    assert_eq!(app.history.undo_label().map(str::to_owned), label);
    // Off, the limit is Ultra's, the most any quality draws.
    app.scatter.decals_for_medium = false;
    let budget = app.scatter.decal_budget();
    assert_eq!((budget.count, budget.distance), (1024, 600.0));
    stroke(
        &mut app,
        Vec3::new(-40.0, ground, 45.0),
        Vec3::new(40.0, ground, 45.0),
        false,
    );
    assert!(scattered_decals(&app).len() > 128);

    // A porch: a slab 4 studs over the ground. Decals lie on the ground
    // under it, or with Land on parts on its top.
    while !scattered_decals(&app).is_empty() {
        app.undo();
    }
    let porch = app.world.create(ClassName::Part).unwrap();
    let workspace = app.world.workspace();
    app.world.set_parent(porch, Some(workspace)).unwrap();
    for (name, value) in [
        (
            "Position",
            Value::Vector3(axcenity_core::Vector3::new(0.0, ground + 4.0, 0.0)),
        ),
        (
            "Size",
            Value::Vector3(axcenity_core::Vector3::new(40.0, 1.0, 40.0)),
        ),
        ("Anchored", Value::Bool(true)),
    ] {
        let _ = app.world.set_property(porch, name, value);
    }
    app.history.seal();
    app.scatter.palette_mut().unwrap().decal_density = 4.0;
    app.scatter.radius = 6.0;
    let across = (Vec3::new(-8.0, ground, 0.0), Vec3::new(8.0, ground, 0.0));
    stroke(&mut app, across.0, across.1, false);
    let under = scattered_decals(&app);
    assert!(!under.is_empty());
    assert!(
        under
            .iter()
            .all(|&id| (decal_position(&app, id).y - ground).abs() < 1e-3)
    );
    app.undo();
    app.scatter.decals_on_parts = true;
    stroke(&mut app, across.0, across.1, false);
    let on_top = scattered_decals(&app);
    assert!(!on_top.is_empty());
    for &id in &on_top {
        let at = decal_position(&app, id);
        assert!((at.y - (ground + 4.5)).abs() < 1e-3, "{at}");
    }
    std::fs::remove_dir_all(parent).unwrap();
}
