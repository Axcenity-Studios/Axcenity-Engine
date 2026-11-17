//! The custom ribbon tab editor drawn in the whole editor: its preview is
//! the ribbon's own code, under a cover that keeps its buttons from doing
//! anything.

use std::path::PathBuf;

use crate::ribbon_tabs::RibbonGroup;

use super::tests::example;
use super::ui_designer_tests::press;
use super::*;

const SIZE: egui::Vec2 = egui::vec2(1280.0, 860.0);

/// One whole-editor frame, `SIZE` big.
fn frame(app: &mut EditorApp, ctx: &egui::Context, clock: &mut f64, events: Vec<egui::Event>) {
    *clock += 0.05;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SIZE)),
        events,
        time: Some(*clock),
        ..Default::default()
    };
    let _ = crate::theme::run(ctx, input, |ui| app.frame(ui, None));
}

#[test]
fn the_tab_editor_previews_with_the_ribbons_code_and_its_buttons_do_nothing() {
    let mut app = EditorApp::new(Some(&example("hello-touched")));
    let ctx = theme::test_context();
    let mut clock = 0.0;
    let groups = [RibbonGroup::Game, RibbonGroup::Tools, RibbonGroup::Parts];
    app.ribbon_tabs
        .edit_for_test("Building", "HAMMER", &groups, None);
    for _ in 0..4 {
        frame(&mut app, &ctx, &mut clock, Vec::new());
    }
    // The ribbon's own row drew the groups (it remembers their width).
    let scene = app.scene_shown(&ctx);
    let width_id = egui::Id::new(("ribbon width", "tab preview", format!("{groups:?}"), scene));
    assert!(
        ctx.data(|data| data.get_temp::<f32>(width_id)).is_some(),
        "the preview is drawn by `ribbon_row`"
    );
    let preview = ctx
        .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("tab preview rect")))
        .expect("the preview is drawn");
    // Click all along the ribbon's buttons: Play, the tools and the parts.
    let tool = app.tool;
    let y = preview.top() + 7.0 + 26.0 + 6.0 + 20.0;
    let mut x = preview.left() + 8.0;
    while x < preview.right() - 8.0 {
        let at = egui::pos2(x, y);
        for events in [
            vec![egui::Event::PointerMoved(at)],
            vec![press(at, true)],
            vec![press(at, false)],
        ] {
            frame(&mut app, &ctx, &mut clock, events);
        }
        x += 14.0;
    }
    assert_eq!(
        app.tool, tool,
        "the preview's tool buttons don't pick tools"
    );
    assert!(!app.playing(), "its Play doesn't play");
    let roots = app
        .world
        .get(app.world.workspace())
        .map(|workspace| workspace.children().len());
    let before = EditorApp::new(Some(&example("hello-touched")));
    assert_eq!(
        roots,
        before
            .world
            .get(before.world.workspace())
            .map(|workspace| workspace.children().len()),
        "its part buttons insert nothing"
    );
    assert!(app.ribbon_tabs.is_editing(), "the editor stays open");
}

/// With `AXCENITY_SNAPSHOTS` set: the tab editor with its preview and
/// icon picker, in both themes.
#[test]
fn the_tab_editor_draws_in_both_themes() {
    let snapshots = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from);
    for (name, appearance, picking) in [
        ("ribbon-tab-editor-dark", Appearance::default(), Some("")),
        (
            "ribbon-tab-editor-light",
            Appearance {
                theme: theme::ThemeChoice::Light,
                ..Appearance::default()
            },
            Some(""),
        ),
        (
            "ribbon-tab-editor-search-dark",
            Appearance::default(),
            Some("car"),
        ),
        (
            "ribbon-tab-editor-closed-picker-light",
            Appearance {
                theme: theme::ThemeChoice::Light,
                ..Appearance::default()
            },
            None,
        ),
    ] {
        let mut app = EditorApp::new(Some(&example("hello-touched")));
        app.ribbon_tabs
            .add_custom("Lights", "LIGHTBULB", &[RibbonGroup::Lights]);
        app.ribbon_tabs.edit_for_test(
            "Level Design",
            "MOUNTAINS",
            &[
                RibbonGroup::Game,
                RibbonGroup::Tools,
                RibbonGroup::Parts,
                RibbonGroup::Terrain,
            ],
            picking,
        );
        let mut draw = |ui: &mut egui::Ui| {
            let ctx = &ui.ctx().clone();
            theme::apply(ctx, &appearance);
            app.frame(ui, None);
        };
        match &snapshots {
            Some(folder) => crate::snapshot::snapshot(
                &folder.join(format!("{name}.png")),
                SIZE,
                vec![Vec::new(); 4],
                &mut draw,
            ),
            None => {
                let ctx = theme::test_context();
                for _ in 0..3 {
                    let _ = crate::theme::run(&ctx, Default::default(), |ui| draw(ui));
                }
            }
        }
        assert!(app.ribbon_tabs.is_editing());
    }
}
