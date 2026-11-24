//! Draws the whole editor window (viewport included) in a few states a
//! beginner meets first and saves each as a PNG, for usability reviews
//! (`docs/research/usability/usability-audit.md`). Ignored, since it needs
//! a GPU and only saves pictures:
//!
//! `AXCENITY_SNAPSHOTS=<folder> cargo test -p axcenity-editor editor_tour -- --ignored`
//!
//! `TOUR_ONLY=<part of a name>` draws only the states whose names have it.
//! `TOUR_THEME=light` draws them in the light theme and `TOUR_DENSITY=compact`
//! at Compact density, for the design-system audit
//! (`docs/research/usability/design-system.md`); the files then end in
//! `-light` or `-compact`. `TOUR_CONTRAST=high` turns Increase contrast on
//! (`-contrast`).
//! The examples are copied to a temporary folder first, so nothing in
//! `crates/samples/projects/` changes.

use super::*;
use crate::layout::Pane;

fn copy_example(name: &str) -> PathBuf {
    let from = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../samples/projects")
        .join(name);
    let to = std::env::temp_dir().join(format!("axcenity-tour-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&to);
    copy_dir(&from, &to);
    to
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &to.join(entry.file_name()));
        } else {
            std::fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

fn shoot(app: &mut EditorApp, name: &str, setup: impl Fn(&mut EditorApp)) {
    let Ok(folder) = std::env::var("AXCENITY_SNAPSHOTS") else {
        return;
    };
    let folder = PathBuf::from(folder);
    std::fs::create_dir_all(&folder).unwrap();
    setup(app);
    let mut appearance = crate::theme::Appearance::default();
    let mut suffix = String::new();
    if std::env::var("TOUR_THEME").is_ok_and(|theme| theme == "light") {
        appearance.theme = crate::theme::ThemeChoice::Light;
        suffix.push_str("-light");
    }
    if std::env::var("TOUR_DENSITY").is_ok_and(|density| density == "compact") {
        appearance.density = crate::theme::Density::Compact;
        suffix.push_str("-compact");
    }
    if std::env::var("TOUR_CONTRAST").is_ok_and(|contrast| contrast == "high") {
        appearance.increase_contrast = true;
        suffix.push_str("-contrast");
    }
    crate::snapshot::snapshot_rendered(
        &folder.join(format!("tour-{name}{suffix}.png")),
        egui::vec2(1600.0, 900.0),
        vec![Vec::new(); 8],
        |ui, rs| {
            let ctx = &ui.ctx().clone();
            crate::theme::apply(ctx, &appearance);
            app.frame(ui, Some(rs));
        },
    );
}

fn find(app: &EditorApp, name: &str) -> InstanceId {
    let root = app.world.workspace();
    app.world.find_first_descendant(root, name).expect(name)
}

/// The service of `class` under the DataModel, such as Lighting.
fn service(app: &EditorApp, class: ClassName) -> InstanceId {
    let root = app.world.root();
    app.world
        .get(root)
        .unwrap()
        .children()
        .iter()
        .copied()
        .find(|&id| app.world.get(id).is_some_and(|i| i.class() == class))
        .expect("the service")
}

fn select(app: &mut EditorApp, id: InstanceId) {
    app.selection = vec![id];
    app.selected = Some(id);
}

fn fresh(example: Option<&str>) -> EditorApp {
    match example {
        None => EditorApp::new(None),
        Some(name) => EditorApp::new(Some(&copy_example(name))),
    }
}

fn first_child(app: &EditorApp) -> InstanceId {
    app.world.get(app.world.workspace()).unwrap().children()[0]
}

#[test]
#[ignore]
fn editor_tour_snapshots() {
    let only = std::env::var("TOUR_ONLY").unwrap_or_default();
    let run = |name: &str, example: Option<&str>, setup: &dyn Fn(&mut EditorApp)| {
        if !only.is_empty() && !name.contains(only.as_str()) {
            return;
        }
        let mut app = fresh(example);
        shoot(&mut app, name, setup);
    };
    run("00-start", None, &|_| {});
    run("01-open", Some("part"), &|_| {});
    run("02-part-selected", Some("part"), &|app| {
        let part = first_child(app);
        select(app, part)
    });
    for tab in [
        crate::ribbon_tabs::BuiltInTab::Model,
        crate::ribbon_tabs::BuiltInTab::World,
        crate::ribbon_tabs::BuiltInTab::Ui,
        crate::ribbon_tabs::BuiltInTab::Script,
        crate::ribbon_tabs::BuiltInTab::Test,
    ] {
        run(
            &format!("03-ribbon-{}", tab.label()),
            Some("part"),
            &|app| app.ribbon_tabs.set_active_for_tour(tab),
        );
    }
    run("04-assets", Some("part"), &|app| {
        app.layout.open(Pane::Assets)
    });
    // The systems the audit only reviewed (A8-A17), for their own pass.
    run("60-script-editor", Some("script"), &|app| {
        app.open_script("scripts/greet.server.lua", Some(1))
    });
    run("61-lighting", Some("lights"), &|app| {
        let lighting = service(app, ClassName::Lighting);
        select(app, lighting)
    });
    run("62-sound", Some("audio"), &|app| {
        let id = find(app, "Music");
        select(app, id)
    });
    run("63-mixer", Some("mixer"), &|app| {
        app.layout.open(Pane::Mixer)
    });
    run("64-collision-groups", Some("physics"), &|app| {
        app.collision_groups.open = true
    });
    run("65-physics-part", Some("physics"), &|app| {
        let part = first_child(app);
        select(app, part)
    });
    run("66-settings", Some("part"), &|app| app.settings.open = true);
    run("67-version-history", Some("part"), &|app| {
        app.version_history.open = true
    });
    run("68-input", Some("part"), &|app| {
        app.input_settings.open = true
    });
    run("69-tests", Some("part"), &|app| {
        app.layout.open(Pane::Tests)
    });
    run("70-profiler", Some("part"), &|app| {
        app.layout.open(Pane::Profiler)
    });
    // The design-system audit: the graph editors and other canvases that
    // draw with their own colours.
    run("80-material-graph", Some("material-graph"), &|app| {
        app.open_material("assets/materials/MossyStone.material.json")
    });
    run("81-behavior-tree", Some("behavior"), &|app| {
        app.open_behavior_tree("assets/ai/Guard.behaviortree.json")
    });
    run("82-dialogue", Some("dialogue"), &|app| {
        app.open_dialogue("assets/dialogue/Farmer.dialogue.json")
    });
    run("83-sequencer", Some("cutscene"), &|app| {
        app.open_sequence("assets/sequences/Intro.sequence.json")
    });
    run("84-terrain", Some("part"), &|app| {
        app.layout.open(Pane::Terrain)
    });
    run("85-blocks", Some("touched-blocks"), &|app| {
        app.open_script("scripts/pad.server.blocks.json", None)
    });
    // Design system Phase 5: the layout presets and the shortcut table.
    for preset in crate::layout::Preset::ALL {
        let name = format!(
            "90-layout-{}",
            preset.label().to_lowercase().replace([' ', '&'], "")
        );
        run(&name, Some("script"), &|app| {
            app.open_script("scripts/greet.server.lua", Some(1));
            // Opened before switching, not just now.
            let _ = app.tabs.take_focus();
            app.layout.use_preset(preset);
        });
    }
    run("91-shortcuts", Some("part"), &|app| {
        app.settings.open = true;
        app.settings.page = settings::SettingsPage::Shortcuts;
    });
    run("10-anim-empty", Some("animation"), &|app| {
        app.layout.open(Pane::Animation)
    });
    run("11-anim-noodle", Some("animation"), &|app| {
        app.layout.open(Pane::Animation);
        let id = find(app, "Noodle");
        select(app, id)
    });
    run("12-anim-door", Some("door"), &|app| {
        app.layout.open(Pane::Animation);
        let id = find(app, "Door");
        select(app, id)
    });
    run("13-anim-floor", Some("door"), &|app| {
        app.layout.open(Pane::Animation);
        let id = find(app, "Floor");
        select(app, id)
    });
    let designer = |app: &mut EditorApp, tab: crate::ui_designer::DesignerTab| {
        app.layout.open(Pane::UiDesigner);
        app.ui_designer.set_tab(tab);
    };
    run("40-ui-empty", Some("part"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::Design)
    });
    run("41-ui-menu", Some("menu"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::Design);
        let id = find(app, "Menu");
        select(app, id)
    });
    run("42-ui-button", Some("menu"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::Design);
        let id = find(app, "Play");
        select(app, id)
    });
    run("43-ui-label", Some("menu"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::Design);
        let id = find(app, "Title");
        select(app, id)
    });
    run("44-ui-graph", Some("ui-graph"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::EventGraph)
    });
    run("45-ui-strings", Some("menu"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::Strings)
    });
    run("46-ui-pages", Some("menu"), &|app| {
        designer(app, crate::ui_designer::DesignerTab::Pages)
    });
    run("14-animate-floor", Some("door"), &|app| {
        app.layout.open(Pane::Animation);
        let id = find(app, "Floor");
        select(app, id);
        app.animate(id);
    });
    // Model Studio (U20).
    let chair = |app: &mut EditorApp| {
        app.layout.open(Pane::ModelStudio);
        app.model_studio
            .open(crate::model_studio::tests::cube_doc());
        let doc = app.model_studio.doc.as_mut().unwrap();
        crate::model_studio::tools_tests::model_chair(doc);
        doc.select(|selection, _| selection.clear());
        doc.set_mode(crate::model_studio::Mode::Object);
    };
    run("20-studio-empty", Some("part"), &|app| {
        app.layout.open(Pane::ModelStudio)
    });
    run("21-studio-object", Some("part"), &|app| chair(app));
    run("22-studio-edit", Some("part"), &|app| {
        chair(app);
        let doc = app.model_studio.doc.as_mut().unwrap();
        doc.set_mode(crate::model_studio::Mode::Edit);
        doc.set_select_mode(axcenity_mesh_edit::SelectMode::Face);
    });
    for (name, mode) in [
        ("23-studio-paint", crate::model_studio::Mode::Paint),
        ("24-studio-sculpt", crate::model_studio::Mode::Sculpt),
        ("25-studio-rig", crate::model_studio::Mode::Armature),
    ] {
        run(name, Some("part"), &|app| {
            app.layout.open(Pane::ModelStudio);
            let model = crate::model_studio::rig_tests::humanoid_doc();
            app.model_studio.open(crate::model_studio::StudioDoc::new(
                String::new(),
                model,
                false,
            ));
            let doc = app.model_studio.doc.as_mut().unwrap();
            doc.set_mode(mode);
        });
    }
    run("26-studio-weights", Some("part"), &|app| {
        app.layout.open(Pane::ModelStudio);
        let model = crate::model_studio::rig_tests::humanoid_doc();
        app.model_studio.open(crate::model_studio::StudioDoc::new(
            String::new(),
            model,
            false,
        ));
        crate::model_studio::rig_tests::rigged(&mut app.model_studio);
        let doc = app.model_studio.doc.as_mut().unwrap();
        doc.set_mode(crate::model_studio::Mode::Weights);
    });
    run("27-studio-paint-new-cube", Some("part"), &|app| {
        app.layout.open(Pane::ModelStudio);
        app.model_studio
            .open(crate::model_studio::tests::cube_doc());
        let doc = app.model_studio.doc.as_mut().unwrap();
        doc.set_mode(crate::model_studio::Mode::Paint);
    });
    // The designer alone, tall, so its whole inspector shows.
    for (name, element, width) in [
        ("47-ui-inspector-button", "Play", 1100.0),
        ("48-ui-inspector-label", "Title", 1100.0),
        ("49-ui-inspector-slider", "Volume", 1100.0),
        ("50-ui-inspector-button-wide", "Play", 1600.0),
    ] {
        if !only.is_empty() && !name.contains(only.as_str()) {
            continue;
        }
        let Ok(folder) = std::env::var("AXCENITY_SNAPSHOTS") else {
            return;
        };
        let mut app = fresh(Some("menu"));
        let id = find(&app, element);
        select(&mut app, id);
        crate::snapshot::snapshot_rendered(
            &PathBuf::from(&folder).join(format!("tour-{name}.png")),
            egui::vec2(width, 2400.0),
            vec![Vec::new(); 4],
            |ui, rs| {
                egui::CentralPanel::default().show(ui, |ui| app.ui_designer_pane(ui, Some(rs)));
            },
        );
    }
}
