//! Render Movie in the app, on copies of the cutscene example: the
//! Sequencer's button and the window, a movie rendered through the app as
//! a creator would, and a cut's transition and the sky keyed in the tab.

use axcenity_anim::PropertyName;
use axcenity_anim::sequence::{SequenceFile, Transition};

use super::tests::{copy_of_example, drawn_texts, last_line};
use super::*;
use crate::movie::encode::{Encoders, Ffmpeg};
use crate::movie::{Format, MovieSettings, Resolution};
use crate::sequencer::{Row, Section};

const INTRO: &str = "assets/sequences/Intro.sequence.json";
const SIZE: egui::Vec2 = egui::vec2(1440.0, 900.0);

/// Draws three frames of `show` and returns the last one's texts.
fn texts(ctx: &egui::Context, mut show: impl FnMut(&mut egui::Ui)) -> Vec<String> {
    let mut output = None;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SIZE)),
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

/// An ffmpeg with the software encoders, as the probe would find one.
fn an_ffmpeg() -> Ffmpeg {
    Ffmpeg {
        program: PathBuf::from("/usr/bin/ffmpeg"),
        encoders: Encoders::parse(
            " ------\n V....D libx264 x\n V..... libsvtav1 x\n V.S..D ffv1 x\n",
        ),
    }
}

#[test]
fn the_sequencer_opens_the_render_movie_window_with_its_sections() {
    let folder = copy_of_example("cutscene", "movie-window");
    let mut app = EditorApp::new(Some(&folder));
    app.open_sequence(INTRO);
    let ctx = crate::theme::test_context();
    // The button is on the Sequencer's own row.
    let tab = texts(&ctx, |ui| {
        egui::CentralPanel::default().show(ui, |ui| app.sequencer_pane(ui));
    });
    assert!(has(&tab, "Render Movie"), "{tab:?}");
    assert!(!app.movie.open);

    // No ffmpeg on the machine: a PNG sequence, and the window says why.
    app.open_render_movie();
    app.movie.set_ffmpeg(None);
    let shown = texts(&ctx, |ui| app.frame(ui, None));
    assert!(app.movie.open);
    // With `AXCENITY_SNAPSHOTS`, the window over the editor, to look at.
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        crate::snapshot::snapshot(
            &snapshots.join("render-movie-window.png"),
            SIZE,
            vec![Vec::new(); 3],
            |ui| app.frame(ui, None),
        );
    }
    for part in [
        "WHAT TO RENDER",
        "All of Intro (0:20, 600 frames)",
        "The play range",
        "PICTURE",
        "The movie is 1920 × 1080 pixels.",
        "QUALITY",
        "Warm-up",
        "OUTPUT",
        "ffmpeg wasn't found on this machine",
        "Check again",
        "Save in the folder",
        "Render",
    ] {
        assert!(has(&shown, part), "`{part}` in {shown:?}");
    }
    assert_eq!(app.movie.formats(), [Format::Png]);
    assert_eq!(app.movie.settings.format, Format::Png);
    assert_eq!(app.movie.settings.output, folder.join("movies/Intro"));

    // With one, every format, and the default becomes a video named after
    // the sequence in the project's movies folder.
    app.movie = Default::default();
    app.open_render_movie();
    app.movie.set_ffmpeg(Some(an_ffmpeg()));
    let shown = texts(&ctx, |ui| app.frame(ui, None));
    assert!(has(&shown, "Videos are encoded by"), "{shown:?}");
    assert_eq!(app.movie.formats(), Format::ALL);
    app.movie.settings.format = Format::H264;
    let shown = texts(&ctx, |ui| app.frame(ui, None));
    assert!(has(&shown, "H.264 video (.mp4)"), "{shown:?}");
    assert!(has(&shown, "Save as"), "{shown:?}");
    assert_eq!(app.movie.settings.output, folder.join("movies/Intro.mp4"));

    // A play range marked in the Sequencer can be rendered alone.
    assert_eq!(app.movie.job_settings(None).range, None);
    app.sequencer.mode.range = Some((2.0, 5.0));
    app.movie.use_range = true;
    let shown = texts(&ctx, |ui| app.frame(ui, None));
    assert!(
        has(&shown, "The play range, 2.00 s to 5.00 s (90 frames)"),
        "{shown:?}"
    );
    assert_eq!(
        app.movie.job_settings(Some((2.0, 5.0))).range,
        Some((2.0, 5.0))
    );
    // Cleared again: back to the whole sequence.
    app.sequencer.mode.range = None;
    let _ = texts(&ctx, |ui| app.frame(ui, None));
    assert!(!app.movie.use_range);

    // A path typed in stays when the format changes; only a default
    // follows the sequence.
    app.movie.set_output(folder.join("out/Trailer.mkv"));
    assert_eq!(app.movie.settings.format, Format::Ffv1);
    let _ = texts(&ctx, |ui| app.frame(ui, None));
    assert_eq!(app.movie.settings.output, folder.join("out/Trailer.mkv"));

    // Without a graphics device there's nothing to render with, said in
    // the window, not a crash.
    app.start_movie(None, None);
    assert!(app.movie.error.as_deref().unwrap().contains("graphics"));
    assert!(!app.movie.rendering());
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_movie_renders_through_the_app_and_the_output_panel_says_where() {
    let folder = copy_of_example("cutscene", "movie-render");
    let mut app = EditorApp::new(Some(&folder));
    app.open_sequence(INTRO);
    let out = folder.join("movies/Intro");
    // A second of it (through the cut at 7 s), small, as PNGs.
    app.sequencer.mode.range = Some((6.5, 7.5));
    app.movie.use_range = true;
    app.open_render_movie();
    app.movie.set_ffmpeg(None);
    app.movie.settings = MovieSettings {
        resolution: Resolution::Custom,
        custom: (192, 108),
        fps: 24,
        warm_up: 4,
        format: Format::Png,
        output: out.clone(),
        ..MovieSettings::default()
    };
    let world_before = app.world.descendants(app.world.root()).len();
    let (mut started, mut frames_drawn, mut widest) = (false, 0, 0.0_f32);
    let snapshot = std::env::var_os("AXCENITY_SNAPSHOTS")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("render-movie.png");
    crate::snapshot::snapshot_rendered(
        &snapshot,
        SIZE,
        vec![Vec::new(); 80],
        |ui, render_state| {
            if !started {
                started = true;
                app.start_movie(Some(render_state), Some((6.5, 7.5)));
                assert_eq!(app.movie.error, None);
                assert!(app.movie.rendering());
            }
            if app.movie.rendering() {
                frames_drawn += 1;
            }
            // The editor's whole frame, which steps the movie.
            app.frame(ui, Some(render_state));
            let window = ui
                .ctx()
                .memory(|memory| memory.area_rect(egui::Id::new("Render Movie")));
            widest = widest.max(window.map_or(0.0, |rect| rect.width()));
        },
    );
    if !started {
        eprintln!("skipped: no graphics adapter on this machine");
        return;
    }
    assert!(!app.movie.rendering(), "the movie ended");
    assert!(frames_drawn > 1, "it was spread over editor frames");
    // The window stays the size it opened at, rendering and done (a row a
    // little too wide once made it grow every frame).
    assert!(widest > 300.0 && widest < 520.0, "{widest}");
    let done = app.movie.done.as_ref().expect("the movie finished");
    assert_eq!((done.frames, done.size, done.fps), (24, (192, 108), 24));
    assert_eq!(done.output, out);
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 24);
    assert!(out.join("Intro_0001.png").is_file() && out.join("Intro_0024.png").is_file());
    let line = &last_line(&app).text;
    assert!(line.starts_with("Rendered the movie "), "{line}");
    assert!(line.contains("24 frames at 24 fps, 192 × 108"), "{line}");
    assert!(line.contains(&*out.to_string_lossy()), "{line}");
    // The cut at 7 s is in it: the frames before and after differ.
    let frame = |index: u32| {
        let path = crate::movie::encode::png_frame_path(&out, "Intro", index);
        image::open(path).unwrap().to_rgb8().into_raw()
    };
    let differing = |a: &[u8], b: &[u8]| {
        a.iter()
            .zip(b)
            .filter(|(a, b)| a.abs_diff(**b) > 40)
            .count()
    };
    assert!(differing(&frame(11), &frame(12)) > 2000, "the cut");
    assert!(differing(&frame(10), &frame(11)) < 600, "within a shot");
    // The edited world is left as it was: the job has its own copy (no
    // overlay ScreenGui, no changed properties, nothing to save).
    assert_eq!(app.world.descendants(app.world.root()).len(), world_before);
    assert!(!app.unsaved());

    // Cancel stops a movie and says so.
    let mut cancelled = false;
    crate::snapshot::snapshot_rendered(
        &std::env::temp_dir().join(format!("axcenity-movie-cancel-{}.png", std::process::id())),
        SIZE,
        vec![Vec::new(); 3],
        |ui, render_state| {
            if !cancelled && !app.movie.rendering() {
                app.movie.settings.warm_up = 500;
                app.start_movie(Some(render_state), Some((6.5, 7.5)));
                assert!(app.movie.rendering());
                assert_eq!(app.movie.done, None, "a new movie forgets the last one");
            }
            app.movie_frame(ui.ctx(), Some(render_state));
            if !cancelled {
                app.cancel_movie();
                cancelled = true;
            }
        },
    );
    assert!(!app.movie.rendering());
    assert!(last_line(&app).text.contains("cancelled at frame 0 of 24"));
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_picked_cut_has_a_transition_that_is_written_and_undone() {
    let folder = copy_of_example("cutscene", "movie-transition");
    let mut app = EditorApp::new(Some(&folder));
    app.open_sequence(INTRO);
    let read = |app: &EditorApp| {
        let folder = &app.project.as_ref().unwrap().folder;
        let text = std::fs::read_to_string(folder.join(INTRO)).unwrap();
        (SequenceFile::from_json(&text).unwrap(), text)
    };
    let before = read(&app).1;
    assert!(!before.contains("transition"));
    let ctx = crate::theme::test_context();
    let tab = |app: &mut EditorApp| {
        texts(&ctx, |ui| {
            egui::CentralPanel::default().show(ui, |ui| app.sequencer_pane(ui));
        })
    };

    // The second cut (at 7 s), picked: a Transition picker, a plain cut.
    let cut = Section {
        row: Row::Camera,
        index: 1,
    };
    app.sequencer.doc.as_mut().unwrap().selected = Some(cut);
    let shown = tab(&mut app);
    assert!(has(&shown, "Transition"), "{shown:?}");
    assert!(shown.iter().any(|text| text == "Cut"), "{shown:?}");
    assert!(!has(&shown, "Lasts"), "{shown:?}");
    assert_eq!(read(&app).1, before, "drawing it changes nothing");

    // A dissolve: how long it lasts, and that the game shows a cut.
    let doc = app.sequencer.doc.as_mut().unwrap();
    doc.file.camera_cuts[1].transition = Transition::Dissolve { duration: 1.5 };
    let shown = tab(&mut app);
    assert!(has(&shown, "Dissolve") && has(&shown, "Lasts"), "{shown:?}");
    assert!(has(&shown, "plays as a cut"), "{shown:?}");
    if let Some(snapshots) = std::env::var_os("AXCENITY_SNAPSHOTS").map(PathBuf::from) {
        crate::snapshot::snapshot(
            &snapshots.join("sequencer-transition.png"),
            egui::vec2(900.0, 700.0),
            vec![Vec::new(); 3],
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| app.sequencer_pane(ui));
            },
        );
    }
    let (file, text) = read(&app);
    assert_eq!(
        file.camera_cuts[1].transition,
        Transition::Dissolve { duration: 1.5 }
    );
    assert_eq!(text.matches("\"transition\"").count(), 1);
    assert_eq!(app.history.undo_label(), Some("Edit Intro"));
    // A fade through a color has the color.
    let doc = app.sequencer.doc.as_mut().unwrap();
    doc.file.camera_cuts[1].transition = Transition::FadeThrough {
        duration: 1.0,
        color: [1.0, 1.0, 1.0],
    };
    let shown = tab(&mut app);
    assert!(has(&shown, "Fade through color") && has(&shown, "Color"));
    assert!(!has(&shown, "plays as a cut"), "{shown:?}");

    app.undo();
    app.undo();
    assert_eq!(read(&app).1, before, "undone, the file is as it was");
    let doc = app.sequencer.doc.as_ref().unwrap();
    assert_eq!(doc.file.camera_cuts[1].transition, Transition::Cut);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_sequencer_keys_the_sky_and_what_is_under_lighting() {
    let folder = copy_of_example("cutscene", "movie-sky-keys");
    let mut app = EditorApp::new(Some(&folder));
    app.open_sequence(INTRO);
    // A Weather and a lens effect in Lighting, as a creator would insert.
    let lighting = app.world.lighting();
    for (class, name) in [
        (ClassName::Weather, "Weather"),
        (ClassName::DepthOfFieldEffect, "Focus"),
    ] {
        let id = app.world.create(class).unwrap();
        app.world.set_name(id, name).unwrap();
        app.world.set_parent(id, Some(lighting)).unwrap();
    }
    let ctx = crate::theme::test_context();
    // What the dope sheet's buttons do with the Explorer's selection: Add
    // track keys one property at the playhead, and Key selected (K) keys
    // again every property of it that has a track.
    let key = |app: &mut EditorApp, id: InstanceId, at: f64, set: &[(&str, Value)]| {
        app.selected = Some(id);
        for (name, value) in set {
            app.world.set_property(id, name, value.clone()).unwrap();
        }
        let picked = crate::sequencer::Picked {
            id,
            path: axcenity_anim::sequence::binding_path(&app.world, id).unwrap(),
            tags: Vec::new(),
            name: app.world.get(id).unwrap().name().to_owned(),
        };
        let focus =
            crate::sequencer::picked_focus(&app.world, &picked, false).expect("something to key");
        let doc = app.sequencer.doc.as_mut().unwrap();
        doc.keys.playhead = at;
        let tracked = doc
            .keys
            .file
            .tracks
            .iter()
            .any(|t| t.target == focus.target);
        if tracked {
            doc.keys.key_focus(&focus);
        } else {
            // Add track lists every property that can be keyed.
            let wanted: Vec<_> = focus
                .values
                .iter()
                .filter(|value| set.iter().any(|(name, _)| value.property.as_str() == *name))
                .cloned()
                .collect();
            assert_eq!(wanted.len(), set.len(), "each is offered by Add track");
            doc.keys.key_values(&wanted);
        }
        focus.target
    };
    let weather = app.world.find_first_child(lighting, "Weather").unwrap();
    let focus = app.world.find_first_child(lighting, "Focus").unwrap();
    let workspace = app.world.workspace();
    let number = Value::Number;
    // Lighting itself binds as `Lighting`, what's in it by its path.
    assert_eq!(
        key(
            &mut app,
            lighting,
            0.0,
            &[("ClockTime", number(6.0)), ("Exposure", number(1.0))]
        ),
        "Lighting"
    );
    key(
        &mut app,
        lighting,
        10.0,
        &[("ClockTime", number(18.0)), ("Exposure", number(2.0))],
    );
    assert_eq!(
        key(
            &mut app,
            weather,
            0.0,
            &[("Wetness", number(0.0)), ("Intensity", number(0.0))]
        ),
        "Lighting/Weather"
    );
    key(
        &mut app,
        weather,
        10.0,
        &[("Wetness", number(1.0)), ("Intensity", number(0.8))],
    );
    assert_eq!(
        key(&mut app, focus, 0.0, &[("FocusDistance", number(5.0))]),
        "Lighting/Focus"
    );
    key(&mut app, focus, 10.0, &[("FocusDistance", number(45.0))]);
    let wind = |x: f32| Value::Vector3(Vector3::new(x, 0.0, 0.0));
    assert_eq!(
        key(&mut app, workspace, 0.0, &[("GlobalWind", wind(0.0))]),
        "Workspace"
    );
    key(&mut app, workspace, 10.0, &[("GlobalWind", wind(20.0))]);

    // Written to the file by their own names, and read back.
    app.flush_sequence(&ctx);
    let text = std::fs::read_to_string(folder.join(INTRO)).unwrap();
    for name in [
        "ClockTime",
        "Exposure",
        "Wetness",
        "Intensity",
        "FocusDistance",
        "GlobalWind",
    ] {
        assert!(
            text.contains(&format!("\"property\": \"{name}\"")),
            "{name}"
        );
    }
    let file = SequenceFile::from_json(&text).unwrap();
    let clock = file
        .tracks
        .iter()
        .find(|track| track.property == PropertyName::from_name("ClockTime").unwrap())
        .unwrap();
    assert_eq!(clock.target, "Lighting");
    assert_eq!(clock.keys.len(), 2);

    // The preview plays them: half way, half way between.
    app.layout.open(Pane::Sequencer);
    app.sequencer.preview_on = true;
    app.sequencer.doc.as_mut().unwrap().keys.playhead = 5.0;
    assert!(app.update_sequence_preview());
    let world = &app.sequencer.shown().unwrap().world;
    let read = |id: InstanceId, name: &str| world.get(id).unwrap().property(name).cloned();
    assert_eq!(read(world.lighting(), "ClockTime"), Some(number(12.0)));
    assert_eq!(read(world.lighting(), "Exposure"), Some(number(1.5)));
    let weather = world.find_first_child(world.lighting(), "Weather").unwrap();
    assert_eq!(read(weather, "Wetness"), Some(number(0.5)));
    let focus = world.find_first_child(world.lighting(), "Focus").unwrap();
    assert_eq!(read(focus, "FocusDistance"), Some(number(25.0)));
    assert_eq!(read(world.workspace(), "GlobalWind"), Some(wind(10.0)));
    let _ = std::fs::remove_dir_all(&folder);
}
