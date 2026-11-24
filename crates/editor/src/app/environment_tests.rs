//! Environment presets in the editor, on copies of the cutscene
//! example: the Sequencer's Environment track (added, keyed, scrubbed in
//! the preview, written as one undo step), and the Properties panel's
//! Environment presets section (apply and save, one undo step each).

use axcenity_anim::environment::{
    BUILT_IN, ClockTurn, EnvironmentPreset, EnvironmentTarget, PresetValue, blend, built_in,
    capture, capture_like, preset_path,
};
use axcenity_anim::sequence::SequenceFile;
use axcenity_anim::{Easing, EasingDirection, EasingStyle, Interp};

use super::tests::{copy_of_example, drawn_texts, last_line};
use super::*;
use crate::sequencer::{Row, Section};

const INTRO: &str = "assets/sequences/Intro.sequence.json";
const SIZE: egui::Vec2 = egui::vec2(1100.0, 900.0);

/// Draws three frames of `show` and returns the last one's texts.
fn texts(ctx: &egui::Context, show: impl FnMut(&mut egui::Ui)) -> Vec<String> {
    texts_at(ctx, SIZE, show)
}

/// [`texts`] on a screen of `size`.
fn texts_at(
    ctx: &egui::Context,
    size: egui::Vec2,
    mut show: impl FnMut(&mut egui::Ui),
) -> Vec<String> {
    let mut output = None;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            ..Default::default()
        };
        output = Some(crate::theme::run(ctx, input, &mut show));
    }
    drawn_texts(&output.unwrap())
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

fn has(texts: &[String], part: &str) -> bool {
    texts.iter().any(|text| text.contains(part))
}

fn number(world: &World, target: EnvironmentTarget, name: &str) -> f64 {
    let id = target
        .find(world)
        .unwrap_or_else(|| panic!("no {target:?}"));
    let instance = world.get(id).unwrap();
    instance.property(name).unwrap().as_number().unwrap()
}

/// How many steps there are to undo.
fn steps_of(app: &EditorApp) -> usize {
    app.history.undo_labels().len()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

/// What the world has of `preset`, to compare with it (the example has no
/// Atmosphere or ColorCorrectionEffect, so those values go nowhere).
fn in_world(preset: &EnvironmentPreset) -> EnvironmentPreset {
    let mut preset = preset.clone();
    preset.atmosphere.clear();
    preset.color_correction.clear();
    preset
}

#[test]
fn applying_a_preset_is_one_undo_step_that_adds_the_clouds_and_weather_it_needs() {
    let folder = copy_of_example("cutscene", "environment-apply");
    let mut app = EditorApp::new(Some(&folder));
    assert!(EnvironmentTarget::Clouds.find(&app.world).is_none());
    assert!(EnvironmentTarget::Weather.find(&app.world).is_none());
    let before = capture(&app.world);
    let count = app.world.descendants(app.world.root()).len();
    let steps = steps_of(&app);

    app.apply_environment_preset("Storm");
    let storm = in_world(&built_in("Storm").unwrap());
    assert_eq!(capture_like(&app.world, &storm), storm);
    assert_eq!(number(&app.world, EnvironmentTarget::Clouds, "Cover"), 0.85);
    assert_eq!(
        number(&app.world, EnvironmentTarget::Lighting, "ClockTime"),
        15.0
    );
    assert_eq!(app.history.undo_label(), Some("Apply Storm"));
    assert_eq!(steps_of(&app), steps + 1, "one undo step");
    assert!(app.unsaved());
    // No Atmosphere in this world: said, not added.
    assert!(EnvironmentTarget::Atmosphere.find(&app.world).is_none());
    assert!(last_line(&app).text.contains("needs an Atmosphere"));

    app.undo();
    assert_eq!(capture(&app.world), before, "the look is as it was");
    assert_eq!(app.world.descendants(app.world.root()).len(), count);
    assert!(EnvironmentTarget::Clouds.find(&app.world).is_none());
    app.redo();
    assert_eq!(capture_like(&app.world, &storm), storm);

    // Another preset over it is another step, and reuses what's there.
    app.apply_environment_preset("SnowDay");
    assert_eq!(steps_of(&app), steps + 2);
    assert_eq!(app.world.descendants(app.world.root()).len(), count + 2);
    let snow = in_world(&built_in("SnowDay").unwrap());
    assert_eq!(capture_like(&app.world, &snow), snow);

    // A preset that isn't there changes nothing and says so.
    app.apply_environment_preset("Monsoon");
    assert_eq!(steps_of(&app), steps + 2);
    assert!(last_line(&app).text.contains("Monsoon"));
    // Not during Play.
    app.play();
    app.apply_environment_preset("Clear");
    app.stop();
    assert_eq!(capture_like(&app.world, &snow), snow);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn saving_the_worlds_look_as_a_preset_is_one_undo_step_and_it_applies_back() {
    let folder = copy_of_example("cutscene", "environment-save");
    let mut app = EditorApp::new(Some(&folder));
    app.apply_environment_preset("Storm");
    // A look of the creator's own: later, and windier.
    let lighting = app.world.lighting();
    app.world
        .set_property(lighting, "ClockTime", Value::Number(17.5))
        .unwrap();
    let workspace = app.world.workspace();
    app.world
        .set_property(
            workspace,
            "GlobalWind",
            Value::Vector3(Vector3::new(30.0, 0.0, 0.0)),
        )
        .unwrap();
    let look = capture(&app.world);
    let steps = steps_of(&app);
    let world_before = app.world.clone();

    app.save_environment_preset("Dusk Gale");
    let path = preset_path("Dusk Gale");
    assert_eq!(path, "assets/environments/Dusk Gale.environment.json");
    let text = std::fs::read_to_string(folder.join(&path)).unwrap();
    let saved = EnvironmentPreset::from_json(&text).unwrap();
    assert_eq!(saved, look);
    assert_eq!(
        saved.number(EnvironmentTarget::Lighting, "ClockTime"),
        Some(17.5)
    );
    assert_eq!(
        saved.get(EnvironmentTarget::Workspace, "GlobalWind"),
        Some(&PresetValue::Triple([30.0, 0.0, 0.0]))
    );
    // The Weather's own settings aren't part of a look.
    assert!(
        saved
            .get(EnvironmentTarget::Weather, "TransitionTime")
            .is_none()
    );
    assert_eq!(app.history.undo_label(), Some("Save preset Dusk Gale"));
    assert_eq!(steps_of(&app), steps + 1, "one undo step");
    assert_eq!(
        capture(&app.world),
        capture(&world_before),
        "saving changes no look"
    );
    assert!(last_line(&app).text.contains("Dusk Gale"));
    // It's listed after the built-ins, for the Sequencer and the section.
    let names = app.environment_presets.names(&app.assets).to_vec();
    assert_eq!(&names[..BUILT_IN.len()], &BUILT_IN[..]);
    assert_eq!(&names[BUILT_IN.len()..], ["Dusk Gale"]);

    // Applied from another look, the world is back where it was saved.
    app.apply_environment_preset("Clear");
    app.apply_environment_preset("Dusk Gale");
    assert_eq!(capture(&app.world), look);
    app.undo();
    app.undo();

    // Undo takes the file away again, and redo puts it back.
    app.undo();
    assert!(
        !folder.join(&path).exists(),
        "undoing the save removes the file"
    );
    assert_eq!(
        app.environment_presets.names(&app.assets).len(),
        BUILT_IN.len()
    );
    app.redo();
    assert_eq!(std::fs::read_to_string(folder.join(&path)).unwrap(), text);

    // A name with nothing to name a file by is refused.
    app.save_environment_preset("../..");
    assert!(last_line(&app).text.contains("needs a name"));
    assert_eq!(steps_of(&app), steps + 1);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_properties_panel_offers_the_presets_with_nothing_or_lighting_selected() {
    let folder = copy_of_example("cutscene", "environment-section");
    let mut app = EditorApp::new(Some(&folder));
    let ctx = crate::theme::test_context();
    // A screen tall enough for all of Lighting's rows and the sections
    // under them: what's scrolled out of sight isn't drawn.
    let panel = |app: &mut EditorApp| {
        texts_at(&ctx, egui::vec2(420.0, 6000.0), |ui| {
            egui::CentralPanel::default().show(ui, |ui| app.properties_pane(ui));
        })
    };
    app.selected = None;
    app.selection.clear();
    let shown = panel(&mut app);
    assert!(has(&shown, "Environment presets"), "{shown:?}");
    for name in BUILT_IN {
        assert!(shown.iter().any(|text| text == name), "{name}: {shown:?}");
    }
    assert!(has(&shown, "Save current as"), "{shown:?}");
    assert_eq!(steps_of(&app), 0, "drawing it changes nothing");

    // With Lighting selected too; with a part, not.
    let lighting = app.world.lighting();
    app.select_from_viewport(Some(lighting));
    assert!(has(&panel(&mut app), "Environment presets"));
    let door = app
        .world
        .find_first_child(app.world.workspace(), "Door")
        .unwrap();
    app.select_from_viewport(Some(door));
    assert!(!has(&panel(&mut app), "Environment presets"));

    // The section's actions are the app's: a button's click applies, and
    // Save writes the preset, which is then a button of its own.
    app.environment_action(crate::environment_presets::EnvironmentAction::Apply(
        "Overcast".into(),
    ));
    assert_eq!(app.history.undo_label(), Some("Apply Overcast"));
    app.environment_action(crate::environment_presets::EnvironmentAction::Save(
        "Grey".into(),
    ));
    app.select_from_viewport(None);
    let shown = panel(&mut app);
    assert!(shown.iter().any(|text| text == "Grey"), "{shown:?}");
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        crate::snapshot::snapshot(
            &snapshots.join("environment-presets.png"),
            egui::vec2(320.0, 700.0),
            vec![Vec::new(); 3],
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| app.properties_pane(ui));
                });
            },
        );
    }
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_sequencer_keys_the_environment_track_and_the_preview_scrubs_the_blend() {
    let folder = copy_of_example("cutscene", "environment-track");
    let mut app = EditorApp::new(Some(&folder));
    // The world gets its clouds and weather (and looks Clear).
    app.apply_environment_preset("Clear");
    let weather = EnvironmentTarget::Weather.find(&app.world).unwrap();
    app.world
        .set_property(weather, "TransitionTime", Value::Number(0.0))
        .unwrap();
    app.open_sequence(INTRO);
    let read = |app: &EditorApp| {
        let folder = &app.project.as_ref().unwrap().folder;
        let text = std::fs::read_to_string(folder.join(INTRO)).unwrap();
        (SequenceFile::from_json(&text).unwrap(), text)
    };
    let before = read(&app).1;
    assert!(!before.contains("environment"));
    let ctx = crate::theme::test_context();
    let tab = |app: &mut EditorApp| {
        texts(&ctx, |ui| {
            egui::CentralPanel::default().show(ui, |ui| app.sequencer_pane(ui));
        })
    };
    // The row is there with the others, empty.
    let shown = tab(&mut app);
    assert!(has(&shown, "Environment"), "{shown:?}");
    assert_eq!(read(&app).1, before, "drawing it changes nothing");

    // Add section > Environment key > a preset, at the playhead: Clear at
    // 2 s, SnowDay at 18 s, then Storm between them at 10 s.
    let steps = steps_of(&app);
    let doc = app.sequencer.doc.as_mut().unwrap();
    assert_eq!(doc.sections(Row::Environment), []);
    for (time, preset) in [(2.0, "Clear"), (18.0, "SnowDay"), (10.0, "Storm")] {
        doc.keys.playhead = time;
        doc.add(Row::Environment, String::new(), preset);
    }
    // In time order, each lasting to the next; the new one is picked.
    assert_eq!(
        doc.sections(Row::Environment),
        [
            (2.0, 10.0, "Clear".to_owned()),
            (10.0, 18.0, "Storm".to_owned()),
            (18.0, 20.0, "SnowDay".to_owned()),
        ]
    );
    let storm = Section {
        row: Row::Environment,
        index: 1,
    };
    assert_eq!(doc.selected, Some(storm));
    // A new key eases gently, the short way round the clock.
    let key = &doc.file.environment[1];
    assert_eq!(key.interp, Interp::Ease);
    assert_eq!(
        key.easing,
        Some(Easing::new(EasingStyle::Sine, EasingDirection::InOut))
    );
    assert_eq!(key.clock, ClockTurn::Shortest);
    // Their times are edit points (Up and Down go to them).
    for time in [2.0, 10.0, 18.0] {
        assert!(doc.edit_points().contains(&time));
    }

    // The picked key's inspector: its preset, its blend and the clock.
    let shown = tab(&mut app);
    for label in [
        "Preset",
        "Blend",
        "Clock",
        "Storm",
        "Ease Sine InOut",
        "The short way",
    ] {
        assert!(has(&shown, label), "{label}: {shown:?}");
    }
    // Drawing wrote the edit, as one undo step.
    let (file, text) = read(&app);
    assert_eq!(file.environment.len(), 3);
    assert_eq!(text.matches("\"environment\"").count(), 1);
    assert_eq!(app.history.undo_label(), Some("Edit Intro"));
    assert_eq!(steps_of(&app), steps + 1);

    // Scrubbing shows the blend live in the preview's world, by the code
    // the game plays with; the world being edited is never touched.
    let (clear, storm_look, snow) = (
        in_world(&built_in("Clear").unwrap()),
        in_world(&built_in("Storm").unwrap()),
        in_world(&built_in("SnowDay").unwrap()),
    );
    let sine = |t: f32| f64::from(Easing::new(EasingStyle::Sine, EasingDirection::InOut).apply(t));
    let edited = capture(&app.world);
    for (time, from, to, t) in [
        (0.0, &clear, &storm_look, 0.0),
        (4.0, &clear, &storm_look, sine(0.25)),
        (6.0, &clear, &storm_look, 0.5),
        (10.0, &clear, &storm_look, 1.0),
        (14.0, &storm_look, &snow, 0.5),
        (16.0, &storm_look, &snow, sine(0.75)),
        (20.0, &storm_look, &snow, 1.0),
        (3.0, &clear, &storm_look, sine(0.125)),
    ] {
        app.sequencer.doc.as_mut().unwrap().keys.playhead = time;
        assert!(app.update_sequence_preview());
        let preview = app.sequencer.shown().unwrap();
        let want = blend(from, to, t, ClockTurn::Shortest);
        let got = capture_like(&preview.world, &want);
        for (target, name, value) in want.fields() {
            match (got.get(target, name), value) {
                (Some(PresetValue::Number(got)), PresetValue::Number(want)) => {
                    assert!(
                        near(*got, *want),
                        "{target:?}.{name} at {time}s: {got} not {want}"
                    )
                }
                (Some(PresetValue::Triple(got)), PresetValue::Triple(want)) => {
                    for (got, want) in got.iter().zip(want) {
                        assert!((got - want).abs() < 1e-4, "{target:?}.{name} at {time}s");
                    }
                }
                (got, _) => assert_eq!(got, Some(value), "{target:?}.{name} at {time}s"),
            }
        }
    }
    assert_eq!(capture(&app.world), edited, "the edited world is as it was");
    // Half way from Storm to SnowDay rain has stopped and snow not begun.
    app.sequencer.doc.as_mut().unwrap().keys.playhead = 14.0;
    app.update_sequence_preview();
    let preview = app.sequencer.shown().unwrap();
    assert!(near(
        number(&preview.world, EnvironmentTarget::Weather, "Intensity"),
        0.0
    ));
    assert!(near(
        number(&preview.world, EnvironmentTarget::Clouds, "Cover"),
        0.825
    ));

    // The inspector's choices are the key's: another preset, a straight
    // blend, the clock forward. The preview follows at once.
    let doc = app.sequencer.doc.as_mut().unwrap();
    doc.file.environment[1].preset = "Overcast".into();
    doc.file.environment[1].interp = Interp::Linear;
    doc.file.environment[1].easing = None;
    doc.file.environment[1].clock = ClockTurn::Forward;
    doc.keys.playhead = 10.0;
    assert!(app.update_sequence_preview());
    let preview = app.sequencer.shown().unwrap();
    assert!(near(
        number(&preview.world, EnvironmentTarget::Clouds, "Cover"),
        0.8
    ));
    assert!(near(
        number(&preview.world, EnvironmentTarget::Lighting, "ClockTime"),
        13.4
    ));
    let shown = tab(&mut app);
    assert!(
        has(&shown, "Overcast") && has(&shown, "Linear"),
        "{shown:?}"
    );
    assert!(has(&shown, "Forward (time passes)"), "{shown:?}");
    let (file, _) = read(&app);
    assert_eq!(file.environment[1].preset, "Overcast");
    assert_eq!(file.environment[1].clock, ClockTurn::Forward);

    // Dragging a key past another keeps them in time order and the key
    // picked; deleting it leaves the others.
    let doc = app.sequencer.doc.as_mut().unwrap();
    doc.set_span(storm, 19.0, 19.0);
    assert_eq!(doc.file.environment[2].preset, "Overcast");
    assert_eq!(
        doc.selected,
        Some(Section {
            row: Row::Environment,
            index: 2
        })
    );
    doc.delete(doc.selected.unwrap());
    assert_eq!(doc.file.environment.len(), 2);
    app.flush_sequence(&ctx);

    // A project's own preset is offered and previews too.
    let lighting = app.world.lighting();
    app.world
        .set_property(lighting, "ClockTime", Value::Number(5.0))
        .unwrap();
    app.save_environment_preset("Dawn");
    let doc = app.sequencer.doc.as_mut().unwrap();
    doc.keys.playhead = 20.0;
    doc.add(Row::Environment, String::new(), "Dawn");
    assert!(app.update_sequence_preview());
    let preview = app.sequencer.shown().unwrap();
    assert!(near(
        number(&preview.world, EnvironmentTarget::Lighting, "ClockTime"),
        5.0
    ));
    assert!(
        app.sequencer.errors.is_empty(),
        "{:?}",
        app.sequencer.errors
    );
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        crate::snapshot::snapshot(
            &snapshots.join("sequencer-environment.png"),
            egui::vec2(900.0, 760.0),
            vec![Vec::new(); 3],
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| app.sequencer_pane(ui));
            },
        );
    }
    app.flush_sequence(&ctx);

    // Undone all the way, the file is as it was and the tab shows it.
    while steps_of(&app) > steps {
        app.undo();
    }
    assert_eq!(read(&app).1, before, "undone, the file is as it was");
    assert!(
        app.sequencer
            .doc
            .as_ref()
            .unwrap()
            .file
            .environment
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(&folder);
}

/// Play shows what the preview showed: the Intro plays by itself, and its
/// Environment track moves the game's sky and weather.
#[test]
fn play_shows_the_environment_track_as_the_preview_did() {
    let folder = copy_of_example("cutscene", "environment-play");
    let mut app = EditorApp::new(Some(&folder));
    app.apply_environment_preset("Clear");
    app.open_sequence(INTRO);
    let doc = app.sequencer.doc.as_mut().unwrap();
    for (time, preset) in [(0.0, "Clear"), (4.0, "Storm")] {
        doc.keys.playhead = time;
        doc.add(Row::Environment, String::new(), preset);
    }
    app.flush_sequence(&crate::theme::test_context());
    let mut previewed = Vec::new();
    for time in [1.0, 2.0, 3.0] {
        app.sequencer.doc.as_mut().unwrap().keys.playhead = time;
        assert!(app.update_sequence_preview());
        let world = &app.sequencer.shown().unwrap().world;
        previewed.push((
            number(world, EnvironmentTarget::Clouds, "Cover"),
            number(world, EnvironmentTarget::Lighting, "ClockTime"),
            number(world, EnvironmentTarget::Weather, "Wetness"),
        ));
    }
    assert!(previewed[0].0 < previewed[1].0 && previewed[1].0 < previewed[2].0);
    app.play();
    for (index, shown) in previewed.iter().enumerate() {
        super::tests::play_for(&mut app, 1.0);
        let world = app.play.as_ref().unwrap().world();
        let played = (
            number(world, EnvironmentTarget::Clouds, "Cover"),
            number(world, EnvironmentTarget::Lighting, "ClockTime"),
            number(world, EnvironmentTarget::Weather, "Wetness"),
        );
        let close = |a: f64, b: f64| (a - b).abs() < 0.02;
        assert!(
            close(played.0, shown.0) && close(played.1, shown.1) && close(played.2, shown.2),
            "at {}s: played {played:?}, previewed {shown:?}",
            index + 1
        );
    }
    app.stop();
    let _ = std::fs::remove_dir_all(&folder);
}
