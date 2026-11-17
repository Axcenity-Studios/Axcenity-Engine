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
