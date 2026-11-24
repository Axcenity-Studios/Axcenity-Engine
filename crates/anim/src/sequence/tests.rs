use super::*;
use crate::files::PropertyKeyframe;

fn key(id: &str, time: f64, value: &[f32]) -> PropertyKeyframe {
    PropertyKeyframe::new(id, time, value.to_vec())
}

fn track(
    id: &str,
    target: &str,
    property: PropertyName,
    keys: Vec<PropertyKeyframe>,
) -> PropertyTrack {
    PropertyTrack {
        id: id.to_owned(),
        target: target.to_owned(),
        property,
        keys,
    }
}

/// A world with a Door part, a Camera named Shot, a tagged Hero Model with
/// an Animator, and a Sound.
fn world() -> World {
    let mut world = World::new();
    let workspace = world.workspace();
    let make = |world: &mut World, class, name: &str, parent| {
        let id = world.create(class).unwrap();
        world.set_name(id, name).unwrap();
        world.set_parent(id, Some(parent)).unwrap();
        id
    };
    make(&mut world, ClassName::Part, "Door", workspace);
    let shot = make(&mut world, ClassName::Camera, "Shot", workspace);
    world
        .set_property(
            shot,
            "Position",
            Value::Vector3(Vector3::new(1.0, 2.0, 3.0)),
        )
        .unwrap();
    world
        .set_property(shot, "FieldOfView", Value::Number(30.0))
        .unwrap();
    let hero = make(&mut world, ClassName::Model, "Hero", workspace);
    world.add_tag(hero, "Hero").unwrap();
    make(&mut world, ClassName::Animator, "Animator", hero);
    make(&mut world, ClassName::Sound, "Music", workspace);
    world
}

fn sequence() -> SequenceFile {
    let mut file = SequenceFile::new(10.0);
    file.tracks.push(track(
        "door",
        "Workspace/Door",
        PropertyName::Orientation,
        vec![
            key("a", 2.0, &[0.0, 0.0, 0.0]),
            key("b", 4.0, &[0.0, 90.0, 0.0]),
        ],
    ));
    file.tracks.push(track(
        "camera",
        CAMERA_TARGET,
        PropertyName::Position,
        vec![
            key("a", 5.0, &[0.0, 10.0, 0.0]),
            key("b", 10.0, &[0.0, 20.0, 0.0]),
        ],
    ));
    file.camera_cuts = vec![
        CameraCut {
            id: "c1".into(),
            time: 0.0,
            camera: "Workspace/Shot".into(),
            transition: Default::default(),
        },
        CameraCut {
            id: "c2".into(),
            time: 5.0,
            camera: String::new(),
            transition: Default::default(),
        },
    ];
    file.animations.push(AnimationSection {
        id: "walk".into(),
        start: 1.0,
        end: 9.0,
        target: "tag:Hero".into(),
        clip: "assets/animations/Walk.anim.json".into(),
        blend_in: 1.0,
        blend_out: 0.0,
    });
    file.audio.push(AudioSection {
        id: "music".into(),
        start: 0.0,
        end: 8.0,
        sound: "Workspace/Music".into(),
    });
    file.events.push(ClipEvent {
        time: 3.0,
        name: "Knock".into(),
    });
    file.fades.push(FadeSection {
        id: "in".into(),
        start: 0.0,
        end: 1.0,
        from: 1.0,
        to: 0.0,
        color: black(),
    });
    file.letterbox.push(LetterboxSection {
        id: "bars".into(),
        start: 0.0,
        end: 10.0,
        size: 0.1,
        ramp: 1.0,
    });
    file.subtitles.push(SubtitleSection {
        id: "line".into(),
        start: 2.0,
        end: 4.0,
        key: "Intro.Line1".into(),
        text: "Hello".into(),
    });
    file.levels.push(LevelSection {
        id: "cellar".into(),
        start: 6.0,
        end: 10.0,
        level: "Cellar".into(),
    });
    file
}

fn property(world: &World, path: &str, name: &str) -> Value {
    let id = resolve_binding(world, path).unwrap();
    world.get(id).unwrap().property(name).unwrap().clone()
}

#[test]
fn a_sequence_file_round_trips_and_bad_sections_say_why() {
    let file = sequence();
    let text = file.to_json();
    assert_eq!(SequenceFile::from_json(&text).unwrap(), file);
    assert_eq!(SequenceFile::from_json(&text).unwrap().to_json(), text);

    let mut late = sequence();
    late.subtitles[0].end = 12.0;
    let err = SequenceFile::from_json(&late.to_json()).unwrap_err();
    assert!(err.0.contains("subtitle `line`"), "{}", err.0);
    let err =
        SequenceFile::from_json(&text.replace("\"format_version\": 1", "\"format_version\": 9"))
            .unwrap_err();
    assert!(err.0.contains("format 9"), "{}", err.0);
}

#[test]
fn bindings_find_instances_by_path_or_tag() {
    let world = world();
    let hero = resolve_binding(&world, "tag:Hero").unwrap();
    assert_eq!(world.get(hero).unwrap().name(), "Hero");
    assert_eq!(resolve_binding(&world, "Workspace/Hero"), Some(hero));
    assert_eq!(
        binding_path(&world, hero).as_deref(),
        Some("Workspace/Hero")
    );
    assert_eq!(resolve_binding(&world, "tag:Nobody"), None);
    assert_eq!(resolve_binding(&world, CAMERA_TARGET), None);
}

#[test]
fn applying_a_time_sets_properties_clips_overlay_camera_and_levels() {
    let mut world = world();
    let mut run = SequenceRun::new(sequence()).unwrap();

    let at_three = run.apply(&mut world, 3.0);
    assert_eq!(
        property(&world, "Workspace/Door", "Orientation")
            .as_vector3()
            .unwrap()
            .y
            .round(),
        45.0
    );
    // The first cut is the Shot camera's.
    let shot = at_three.camera.unwrap();
    assert_eq!(shot.position, Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(shot.field_of_view, 30.0);
    assert_eq!(
        at_three.sounds,
        vec![SoundCue::Start(
            resolve_binding(&world, "Workspace/Music").unwrap()
        )]
    );
    assert!(at_three.levels.is_empty());
    // The walk is loaded on the Hero's Animator, held at the sequence's time.
    let animator = world
        .find_first_child(resolve_binding(&world, "tag:Hero").unwrap(), "Animator")
        .unwrap();
    let track = *world.animator(animator).unwrap().tracks.last().unwrap();
    let state = world.animation_track(track).unwrap();
    assert!(state.playing);
    assert_eq!((state.time, state.speed, state.weight), (2.0, 0.0, 1.0));
    // The overlay shows the subtitle and the bars, and the fade is over.
    let overlay = world
        .find_first_child(world.workspace(), OVERLAY_NAME)
        .unwrap();
    let subtitle = world.find_first_child(overlay, "Subtitle").unwrap();
    assert_eq!(
        world.get(subtitle).unwrap().property("TextKey"),
        Some(&Value::String("Intro.Line1".into()))
    );
    assert_eq!(
        world.get(subtitle).unwrap().property("Visible"),
        Some(&Value::Bool(true))
    );
    let fade = world.find_first_child(overlay, "Fade").unwrap();
    assert_eq!(
        world.get(fade).unwrap().property("Visible"),
        Some(&Value::Bool(false))
    );

    // At 7.5 the sequence's own camera keys are in charge, the Cellar is
    // added, and nothing new starts.
    let later = run.apply(&mut world, 7.5);
    let own = later.camera.unwrap();
    assert_eq!(own.position, Vector3::new(0.0, 15.0, 0.0));
    assert_eq!(own.field_of_view, 50.0);
    assert_eq!(later.levels, vec!["Cellar".to_owned()]);
    assert!(later.sounds.is_empty());
    // Past the music's end it stops; the subtitle is gone.
    let end = run.apply(&mut world, 10.0);
    assert_eq!(
        end.sounds,
        vec![SoundCue::Stop(
            resolve_binding(&world, "Workspace/Music").unwrap()
        )]
    );
    assert_eq!(
        end.levels,
        vec!["Cellar".to_owned()],
        "a level section to the end stays"
    );
    assert_eq!(
        world.get(subtitle).unwrap().property("Visible"),
        Some(&Value::Bool(false))
    );
    assert!(run.missing.is_empty(), "{:?}", run.missing);

    // Going back is the same as getting there first.
    run.apply(&mut world, 3.0);
    assert_eq!(
        property(&world, "Workspace/Door", "Orientation")
            .as_vector3()
            .unwrap()
            .y
            .round(),
        45.0
    );

    run.finish(&mut world);
    assert!(
        world
            .find_first_child(world.workspace(), OVERLAY_NAME)
            .is_none()
    );
    assert!(!world.animation_track(track).unwrap().playing);
}

#[test]
fn a_clip_blends_in_over_its_blend_time() {
    let mut world = world();
    let mut run = SequenceRun::new(sequence()).unwrap();
    run.apply(&mut world, 1.5);
    let animator = world
        .find_first_child(resolve_binding(&world, "tag:Hero").unwrap(), "Animator")
        .unwrap();
    let track = *world.animator(animator).unwrap().tracks.last().unwrap();
    assert!((world.animation_track(track).unwrap().weight - 0.5).abs() < 1e-9);
    // Before the section nothing was loaded; after it the track lets go.
    run.apply(&mut world, 9.5);
    assert!(!world.animation_track(track).unwrap().playing);
}

#[test]
fn fades_hold_their_end_and_events_pass_in_order() {
    let mut file = sequence();
    file.fades.push(FadeSection {
        id: "out".into(),
        start: 8.0,
        end: 9.0,
        from: 0.0,
        to: 1.0,
        color: [1.0, 1.0, 1.0],
    });
    assert_eq!(file.fade_at(0.5).0, 0.5);
    assert_eq!(file.fade_at(5.0).0, 0.0);
    assert_eq!(file.fade_at(9.5), (1.0, [1.0, 1.0, 1.0]));
    assert!((file.letterbox_at(0.5) - 0.05).abs() < 1e-6);
    assert_eq!(file.letterbox_at(5.0), 0.1);

    file.events.push(ClipEvent {
        time: 10.0,
        name: "End".into(),
    });
    assert_eq!(file.events_between(0.0, 3.0), Vec::<String>::new());
    assert_eq!(file.events_between(3.0, 10.0), vec!["Knock", "End"]);
}

#[test]
fn missing_bindings_are_reported() {
    let mut world = World::new();
    let mut run = SequenceRun::new(sequence()).unwrap();
    let applied = run.apply(&mut world, 3.0);
    assert!(applied.camera.is_none());
    for binding in [
        "Workspace/Door",
        "tag:Hero",
        "Workspace/Music",
        "Workspace/Shot",
    ] {
        assert!(
            run.missing.contains(binding),
            "{binding}: {:?}",
            run.missing
        );
    }
}

#[test]
fn the_example_sequence_and_walk_clip_are_canonical() {
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../samples/projects/cutscene/assets");
    let text = std::fs::read_to_string(example.join("sequences/Intro.sequence.json"))
        .unwrap()
        .replace("\r\n", "\n");
    let file = SequenceFile::from_json(&text).unwrap();
    assert_eq!(file.to_json(), text);
    assert_eq!(file.duration, 20.0);
    assert_eq!(file.camera_cuts.len(), 3);
    assert_eq!(file.subtitles.len(), 2);
    let clip = std::fs::read_to_string(example.join("animations/Hero/Walk.anim.json"))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(ClipFile::from_json(&clip).unwrap().to_json(), clip);
}

#[test]
fn a_camera_cut_bound_to_a_spline_rides_it_from_the_cut_to_the_next() {
    let mut world = World::new();
    let rail = world.create(ClassName::Spline).unwrap();
    world.set_name(rail, "Rail").unwrap();
    world.set_parent(rail, Some(world.workspace())).unwrap();
    for x in [0.0, 20.0] {
        let point = world.create(ClassName::SplinePoint).unwrap();
        world.set_parent(point, Some(rail)).unwrap();
        world
            .set_property(point, "Position", Value::Vector3(Vector3::new(x, 5.0, 0.0)))
            .unwrap();
    }
    let mut file = SequenceFile::new(10.0);
    file.camera_cuts = vec![
        CameraCut {
            id: "ride".into(),
            time: 2.0,
            camera: "Workspace/Rail".into(),
            transition: Default::default(),
        },
        CameraCut {
            id: "own".into(),
            time: 6.0,
            camera: String::new(),
            transition: Default::default(),
        },
    ];
    let mut run = SequenceRun::new(file).unwrap();
    // From 2 to 6 it rides the 20 studs: halfway along at 4.
    let shot = |run: &mut SequenceRun, world: &mut World, time| {
        run.apply(world, time).camera.expect("a shot")
    };
    let start = shot(&mut run, &mut world, 2.0);
    assert!((start.position.x - 0.0).abs() < 1e-3);
    let middle = shot(&mut run, &mut world, 4.0);
    assert!((middle.position.x - 10.0).abs() < 1e-2, "{middle:?}");
    assert_eq!(middle.position.y, 5.0);
    // Facing +X, along the rail.
    assert!((middle.orientation.y + 90.0).abs() < 1e-2, "{middle:?}");
    let end = shot(&mut run, &mut world, 5.999);
    assert!((end.position.x - 20.0).abs() < 0.05, "{end:?}");
}

/// Two cuts at 0 and 4 of a 10 second sequence: the Camera named Shot, then
/// a second Camera named Wide, taking over with `transition`.
fn two_shots(world: &mut World, transition: Transition) -> SequenceFile {
    let wide = world.create(ClassName::Camera).unwrap();
    world.set_name(wide, "Wide").unwrap();
    world.set_parent(wide, Some(world.workspace())).unwrap();
    world
        .set_property(
            wide,
            "Position",
            Value::Vector3(Vector3::new(50.0, 5.0, 0.0)),
        )
        .unwrap();
    let mut file = SequenceFile::new(10.0);
    file.camera_cuts = vec![
        CameraCut {
            id: "close".into(),
            time: 0.0,
            camera: "Workspace/Shot".into(),
            transition: Transition::Cut,
        },
        CameraCut {
            id: "wide".into(),
            time: 4.0,
            camera: "Workspace/Wide".into(),
            transition,
        },
    ];
    file
}

#[test]
fn a_cut_without_a_transition_is_written_as_before_and_old_files_read() {
    // A file from before transitions: no `transition` anywhere.
    let old = r#"{
  "format_version": 1,
  "duration": 10.0,
  "camera_cuts": [
    {
      "id": "close",
      "time": 0.0,
      "camera": "Workspace/Shot"
    }
  ]
}
"#;
    let file = SequenceFile::from_json(old).unwrap();
    assert_eq!(file.camera_cuts[0].transition, Transition::Cut);
    assert_eq!(file.to_json(), old, "a plain cut writes no transition");
    assert_eq!(SEQUENCE_FORMAT, 1, "the field is optional: no new format");
}

#[test]
fn a_transition_round_trips_and_a_bad_one_says_why() {
    let mut world = world();
    for transition in [
        Transition::Dissolve { duration: 1.5 },
        Transition::FadeThrough {
            duration: 2.0,
            color: [1.0, 1.0, 1.0],
        },
    ] {
        let file = two_shots(&mut world, transition);
        let text = file.to_json();
        assert!(text.contains("\"transition\""), "{text}");
        let read = SequenceFile::from_json(&text).unwrap();
        assert_eq!(read, file);
        assert_eq!(read.to_json(), text);
        // Only the cut with a transition names one.
        assert_eq!(text.matches("\"transition\"").count(), 1, "{text}");
    }
    // A fade through black needs no color in the file.
    let text = r#"{ "format_version": 1, "duration": 4,
        "camera_cuts": [ { "id": "a", "time": 2, "transition":
            { "kind": "FadeThrough", "duration": 1 } } ] }"#;
    let read = SequenceFile::from_json(text).unwrap();
    assert_eq!(
        read.camera_cuts[0].transition,
        Transition::FadeThrough {
            duration: 1.0,
            color: [0.0, 0.0, 0.0]
        }
    );
    let bad = text.replace("\"duration\": 1 }", "\"duration\": -1 }");
    let error = SequenceFile::from_json(&bad).unwrap_err().0;
    assert!(error.contains("transition lasts -1s"), "{error}");
    let unknown = text.replace("FadeThrough", "Wipe");
    assert!(SequenceFile::from_json(&unknown).is_err());
}

#[test]
fn a_transition_is_centred_on_its_cut_and_stops_half_way_to_the_next_cut() {
    let mut world = world();
    let mut file = two_shots(&mut world, Transition::Dissolve { duration: 2.0 });
    assert_eq!(file.transition_span(0), None, "a plain cut has none");
    assert_eq!(file.transition_span(1), Some((3.0, 5.0)));
    // Longer than the shots around it: it stops half way back to the cut
    // before (the shot before runs 0 to 4), and at the sequence's end.
    file.camera_cuts[1].transition = Transition::Dissolve { duration: 30.0 };
    assert_eq!(file.transition_span(1), Some((2.0, 10.0)));
    // With a cut after it at 8, half way on to that one.
    let mut three = file.clone();
    three.camera_cuts.push(CameraCut {
        id: "last".into(),
        time: 8.0,
        camera: String::new(),
        transition: Transition::Dissolve { duration: 30.0 },
    });
    assert_eq!(three.transition_span(1), Some((2.0, 6.0)));
    assert_eq!(three.transition_span(2), Some((6.0, 10.0)));
    // A transition on the first cut starts at the sequence's start.
    file.camera_cuts[0].transition = Transition::FadeThrough {
        duration: 1.0,
        color: black(),
    };
    assert_eq!(file.transition_span(0), Some((0.0, 0.5)));
    // No length: a plain cut.
    file.camera_cuts[1].transition = Transition::Dissolve { duration: 0.0 };
    assert_eq!(file.transition_span(1), None);
}

#[test]
fn a_fade_through_covers_the_screen_at_the_cut_like_two_fades() {
    let mut world = world();
    let white = [1.0, 1.0, 1.0];
    let file = two_shots(
        &mut world,
        Transition::FadeThrough {
            duration: 2.0,
            color: white,
        },
    );
    // The same as a fade out to the cut and a fade in after it.
    let mut fades = two_shots(&mut world, Transition::Cut);
    for (id, start, end, from, to) in [("out", 3.0, 4.0, 0.0, 1.0), ("in", 4.0, 5.0, 1.0, 0.0)] {
        fades.fades.push(FadeSection {
            id: id.into(),
            start,
            end,
            from,
            to,
            color: white,
        });
    }
    for step in 0..=100 {
        let time = f64::from(step) * 0.1;
        let (cover, color) = file.cover_at(time);
        let (wanted, _) = fades.fade_at(time);
        assert!((cover - wanted).abs() < 1e-5, "{time}: {cover} vs {wanted}");
        if cover > 0.0 {
            assert_eq!(color, white);
        }
    }
    assert_eq!(file.cover_at(4.0).0, 1.0);
    assert_eq!(file.cover_at(2.9).0, 0.0);

    // The overlay draws it, with no fade section in the file: the game,
    // the preview and a movie all show it.
    let mut run = SequenceRun::new(file).unwrap();
    let applied = run.apply(&mut world, 3.5);
    assert_eq!(applied.dissolve, None);
    assert_eq!(
        property(&world, "Workspace/Cutscene/Fade", "BackgroundTransparency"),
        Value::Number(0.5)
    );
    assert_eq!(
        property(&world, "Workspace/Cutscene/Fade", "BackgroundColor3"),
        Value::Color3(Color3::new(1.0, 1.0, 1.0))
    );
    // The cut itself is still at its time, behind the cover.
    assert_eq!(
        applied.camera.unwrap().position,
        Vector3::new(1.0, 2.0, 3.0)
    );
    let after = run.apply(&mut world, 4.5).camera.unwrap();
    assert_eq!(after.position, Vector3::new(50.0, 5.0, 0.0));
    // A section's fade still wins where it covers more.
    let mut both = run.file.clone();
    both.fades.push(FadeSection {
        id: "hold".into(),
        start: 2.0,
        end: 3.2,
        from: 1.0,
        to: 1.0,
        color: black(),
    });
    assert_eq!(both.cover_at(3.1), (1.0, black()));
}

#[test]
fn a_dissolve_gives_both_shots_and_their_mix_while_the_view_cuts_at_the_cut() {
    let mut world = world();
    let file = two_shots(&mut world, Transition::Dissolve { duration: 2.0 });
    assert_eq!(file.dissolve_at(3.0), None, "not yet at its very start");
    assert_eq!(file.dissolve_at(5.0), None, "over at its end");
    let (index, weight) = file.dissolve_at(4.0).unwrap();
    assert_eq!(index, 1);
    assert!((weight - 0.5).abs() < 1e-6);
    let mut run = SequenceRun::new(file).unwrap();
    let close = Vector3::new(1.0, 2.0, 3.0);
    let wide = Vector3::new(50.0, 5.0, 0.0);
    // Before the cut: the view is still the first shot's, and the
    // dissolve names both.
    let applied = run.apply(&mut world, 3.5);
    assert_eq!(applied.camera.unwrap().position, close);
    let dissolve = applied.dissolve.unwrap();
    assert_eq!(dissolve.from.unwrap().position, close);
    assert_eq!(dissolve.to.unwrap().position, wide);
    assert!((dissolve.weight - 0.25).abs() < 1e-6);
    // After it: a host with one view has cut; the mix carries on.
    let applied = run.apply(&mut world, 4.5);
    assert_eq!(applied.camera.unwrap().position, wide);
    let dissolve = applied.dissolve.unwrap();
    assert_eq!(dissolve.from.unwrap().position, close);
    assert!((dissolve.weight - 0.75).abs() < 1e-6);
    assert_eq!(run.apply(&mut world, 6.0).dissolve, None);
    // No overlay is made for a dissolve.
    assert!(resolve_binding(&world, "Workspace/Cutscene").is_none());
    // A dissolve on the first cut comes from the host's own camera.
    let mut first = run.file.clone();
    first.camera_cuts.remove(0);
    let mut run = SequenceRun::new(first).unwrap();
    let dissolve = run.apply(&mut world, 3.5).dissolve.unwrap();
    assert_eq!(dissolve.from, None);
    assert_eq!(dissolve.to.unwrap().position, wide);
}

#[test]
fn a_sequence_keys_the_sky_the_weather_lights_and_lens_effects() {
    let mut world = world();
    let lighting = world.lighting();
    let workspace = world.workspace();
    let mut make = |class, name: &str, parent| {
        let id = world.create(class).unwrap();
        world.set_name(id, name).unwrap();
        world.set_parent(id, Some(parent)).unwrap();
        id
    };
    make(ClassName::Atmosphere, "Atmosphere", lighting);
    make(ClassName::Clouds, "Clouds", lighting);
    make(ClassName::Weather, "Weather", lighting);
    make(ClassName::DepthOfFieldEffect, "Focus", lighting);
    make(ClassName::MotionBlurEffect, "Blur", lighting);
    make(ClassName::VignetteEffect, "Vignette", lighting);
    make(ClassName::ColorCorrectionEffect, "Grade", lighting);
    make(ClassName::PointLight, "Lamp", workspace);
    make(ClassName::SpotLight, "Spot", workspace);

    // Every one is a property a track can name, by its own name.
    let numbers = [
        ("Lighting", "ClockTime", 6.0, 18.0),
        ("Lighting", "Exposure", 1.0, 2.0),
        ("Lighting", "EnvironmentIntensity", 0.2, 0.8),
        ("Lighting", "BloomIntensity", 0.0, 1.0),
        ("Lighting/Atmosphere", "Density", 0.1, 0.5),
        ("Lighting/Atmosphere", "Haze", 0.0, 4.0),
        ("Lighting/Clouds", "Cover", 0.0, 1.0),
        ("Lighting/Clouds", "Direction", 0.0, 90.0),
        ("Lighting/Weather", "Intensity", 0.0, 1.0),
        ("Lighting/Weather", "Wetness", 0.0, 1.0),
        ("Lighting/Weather", "SnowCover", 0.0, 1.0),
        ("Lighting/Weather", "SnowThickness", 0.0, 2.0),
        ("Lighting/Focus", "FocusDistance", 5.0, 45.0),
        ("Lighting/Focus", "InFocusRadius", 2.0, 6.0),
        ("Lighting/Focus", "NearIntensity", 0.0, 1.0),
        ("Lighting/Focus", "FarIntensity", 0.0, 1.0),
        ("Lighting/Blur", "Intensity", 0.0, 1.0),
        ("Lighting/Vignette", "Intensity", 0.0, 1.0),
        ("Lighting/Vignette", "Size", 0.2, 0.6),
        ("Lighting/Grade", "Brightness", -0.5, 0.5),
        ("Lighting/Grade", "Contrast", 0.0, 1.0),
        ("Lighting/Grade", "Saturation", -1.0, 1.0),
        ("Workspace/Lamp", "Brightness", 0.0, 4.0),
        ("Workspace/Lamp", "Range", 8.0, 24.0),
        ("Workspace/Spot", "Angle", 20.0, 60.0),
    ];
    let colors = [
        ("Lighting", "SkyTopColor"),
        ("Lighting", "FogColor"),
        ("Lighting/Atmosphere", "Color"),
        ("Lighting/Atmosphere", "Decay"),
        ("Lighting/Clouds", "Color"),
        ("Lighting/Grade", "TintColor"),
        ("Workspace/Lamp", "Color"),
        ("Workspace", "GlobalWind"),
    ];
    let mut file = SequenceFile::new(10.0);
    for (i, (target, name, from, to)) in numbers.iter().enumerate() {
        let property = PropertyName::from_name(name)
            .unwrap_or_else(|| panic!("`{name}` isn't a property a track can key"));
        assert_eq!(property.as_str(), *name);
        file.tracks.push(track(
            &format!("n{i}"),
            target,
            property,
            vec![key("a", 0.0, &[*from]), key("b", 10.0, &[*to])],
        ));
    }
    for (i, (target, name)) in colors.iter().enumerate() {
        let property = PropertyName::from_name(name)
            .unwrap_or_else(|| panic!("`{name}` isn't a property a track can key"));
        file.tracks.push(track(
            &format!("c{i}"),
            target,
            property,
            vec![
                key("a", 0.0, &[0.0, 0.25, 1.0]),
                key("b", 10.0, &[1.0, 0.75, 0.0]),
            ],
        ));
    }
    // What falls is a choice and a light's switch is on or off: both are
    // held from each key to the next, whatever the key says.
    file.tracks.push(track(
        "falls",
        "Lighting/Weather",
        PropertyName::from_name("Precipitation").unwrap(),
        vec![
            key("a", 0.0, &[0.0]),
            key("b", 4.0, &[2.0]),
            key("c", 8.0, &[1.0]),
        ],
    ));
    file.tracks.push(track(
        "lit",
        "Workspace/Lamp",
        PropertyName::from_name("Enabled").unwrap(),
        vec![key("a", 0.0, &[0.0]), key("b", 6.0, &[1.0])],
    ));

    // The file round-trips with the properties' own names.
    let text = file.to_json();
    assert!(text.contains("\"property\": \"ClockTime\""), "{text}");
    let read = SequenceFile::from_json(&text).unwrap();
    assert_eq!(read.to_json(), text);

    let mut run = SequenceRun::new(read).unwrap();
    run.apply(&mut world, 5.0);
    assert_eq!(run.missing, BTreeSet::new(), "every binding and property");
    let number = |world: &World, path: &str, name: &str| match property(world, path, name) {
        Value::Number(n) => n,
        other => panic!("{path}.{name} is {other:?}"),
    };
    for (target, name, from, to) in numbers {
        let now = number(&world, target, name);
        let wanted = f64::from(from + (to - from) * 0.5);
        assert!((now - wanted).abs() < 1e-4, "{target}.{name}: {now}");
    }
    assert_eq!(
        property(&world, "Lighting/Clouds", "Color"),
        Value::Color3(Color3::new(0.5, 0.5, 0.5))
    );
    assert_eq!(
        property(&world, "Workspace", "GlobalWind"),
        Value::Vector3(Vector3::new(0.5, 0.5, 0.5))
    );
    let falls = |world: &World| match property(world, "Lighting/Weather", "Precipitation") {
        Value::EnumItem(item) => item.item,
        other => panic!("{other:?}"),
    };
    assert_eq!(falls(&world), "Snow", "held from the key at 4s");
    assert_eq!(
        property(&world, "Workspace/Lamp", "Enabled"),
        Value::Bool(false),
        "off until its key at 6s"
    );
    run.apply(&mut world, 2.0);
    assert_eq!(falls(&world), "None", "nothing between None and Snow");
    run.apply(&mut world, 9.0);
    assert_eq!(falls(&world), "Rain");
    assert_eq!(
        property(&world, "Workspace/Lamp", "Enabled"),
        Value::Bool(true)
    );
}

#[test]
fn scene_properties_are_only_a_sequences_and_what_clips_key_is_unchanged() {
    let scene = crate::files::scene_properties();
    assert!(scene.windows(2).all(|pair| pair[0].name < pair[1].name));
    for property in scene {
        assert!(
            !PropertyName::ALL
                .iter()
                .any(|p| p.as_str() == property.name),
            "{} is already a clip's",
            property.name
        );
        assert_eq!(
            PropertyName::from_name(property.name).unwrap().as_str(),
            property.name
        );
    }
    // What a clip offers to key hasn't grown.
    assert!(PropertyName::keyable().all(|p| !matches!(p, PropertyName::Scene(_))));
    assert_eq!(
        PropertyName::sequence_keyable().count(),
        PropertyName::keyable().count() + scene.len()
    );
    // A Clouds' Direction is degrees and a SpotLight's is a Vector3.
    assert_eq!(
        PropertyName::from_name("Direction").unwrap().widths(),
        &[1, 3]
    );
    assert!(
        PropertyName::from_name("Precipitation")
            .unwrap()
            .is_stepped()
    );
    assert!(!PropertyName::from_name("ClockTime").unwrap().is_stepped());
    // Text and other enums still can't be keyed.
    assert_eq!(PropertyName::from_name("SkyImage"), None);
    assert_eq!(PropertyName::from_name("ToneMapping"), None);
}
