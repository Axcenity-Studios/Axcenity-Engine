use axcenity_core::{ClassName, Color3, EnumItem, Value, Vector3, World};

use super::EnvironmentTarget::{Atmosphere, Clouds, ColorCorrection, Lighting, Weather, Workspace};
use super::*;
use crate::files::{PropertyKeyframe, PropertyName, PropertyTrack};
use crate::sequence::{SEQUENCE_FORMAT, SequenceFile, SequenceRun};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

/// A world with an Atmosphere, Clouds, a Weather that changes at once and a
/// ColorCorrectionEffect in Lighting.
fn world() -> World {
    let mut world = World::new();
    let lighting = world.lighting();
    for (class, name) in [
        (ClassName::Atmosphere, "Atmosphere"),
        (ClassName::Clouds, "Clouds"),
        (ClassName::Weather, "Weather"),
        (ClassName::ColorCorrectionEffect, "Grade"),
    ] {
        let id = world.create(class).unwrap();
        world.set_name(id, name).unwrap();
        world.set_parent(id, Some(lighting)).unwrap();
    }
    world
}

fn value(world: &World, target: EnvironmentTarget, name: &str) -> Value {
    let id = target.find(world).unwrap();
    world.get(id).unwrap().property(name).unwrap().clone()
}

fn number(world: &World, target: EnvironmentTarget, name: &str) -> f64 {
    value(world, target, name).as_number().unwrap()
}

fn falling(world: &World) -> String {
    match value(world, Weather, "Precipitation") {
        Value::EnumItem(item) => item.item,
        other => panic!("Precipitation is {other:?}"),
    }
}

fn preset(name: &str) -> EnvironmentPreset {
    built_in(name).unwrap()
}

#[test]
fn the_built_in_presets_are_whole_valid_and_as_documented() {
    for name in BUILT_IN {
        let preset = preset(name);
        preset.check().unwrap();
        // Each sets the same properties, so any two blend value for value.
        let fields: Vec<_> = preset.fields().map(|(t, n, _)| (t, n.to_owned())).collect();
        let clear: Vec<_> = self::preset("Clear")
            .fields()
            .map(|(t, n, _)| (t, n.to_owned()))
            .collect();
        assert_eq!(fields, clear, "{name}");
        // And the file a project would write of it reads back the same.
        assert_eq!(
            EnvironmentPreset::from_json(&preset.to_json()).unwrap(),
            preset
        );
    }
    assert!(built_in("storm").is_some(), "any case finds a built-in");
    assert!(built_in("Blizzard").is_none());

    let clear = preset("Clear");
    assert_eq!(clear.number(Lighting, "ClockTime"), Some(13.4));
    assert_eq!(clear.number(Clouds, "Cover"), Some(0.3));
    assert_eq!(
        clear.get(Weather, "Precipitation"),
        Some(&PresetValue::Choice("None".into()))
    );
    let storm = preset("Storm");
    assert_eq!(storm.number(Clouds, "Cover"), Some(0.85));
    assert_eq!(storm.number(Weather, "Intensity"), Some(0.9));
    assert_eq!(storm.number(Weather, "Wetness"), Some(0.9));
    assert_eq!(
        storm.get(Workspace, "GlobalWind"),
        Some(&PresetValue::Triple([16.0, 0.0, 6.0]))
    );
    assert_eq!(
        storm.get(Weather, "Precipitation").unwrap().as_choice(),
        Some("Rain")
    );
    for snowy in ["SnowDay", "SnowNight"] {
        let snow = preset(snowy);
        assert_eq!(snow.number(Weather, "SnowCover"), Some(1.0));
        assert_eq!(snow.number(Weather, "Intensity"), Some(0.6));
        assert_eq!(
            snow.get(Weather, "Precipitation").unwrap().as_choice(),
            Some("Snow")
        );
    }
    for night in ["Evening", "SnowNight"] {
        let hour = preset(night).number(Lighting, "ClockTime").unwrap();
        assert!((19.0..=20.0).contains(&hour), "{night} is at {hour}");
    }
}

#[test]
fn numbers_colors_and_vectors_blend_straight_and_the_ends_are_exact() {
    let (clear, storm) = (preset("Clear"), preset("Storm"));
    let half = blend(&clear, &storm, 0.5, ClockTurn::Shortest);
    // A number.
    assert!(near(half.number(Clouds, "Cover").unwrap(), 0.575));
    assert!(near(half.number(Clouds, "Density").unwrap(), 0.625));
    assert!(near(
        half.number(ColorCorrection, "Saturation").unwrap(),
        -0.125
    ));
    assert!(near(half.number(Atmosphere, "Haze").unwrap(), 0.5));
    // A color.
    let Some(PresetValue::Triple(color)) = half.get(Clouds, "Color") else {
        panic!("the clouds' color is three numbers");
    };
    for (got, want) in color.iter().zip([0.75, 0.76, 0.79]) {
        assert!((got - want).abs() < 1e-5);
    }
    // A vector.
    assert_eq!(
        half.get(Workspace, "GlobalWind"),
        Some(&PresetValue::Triple([10.0, 0.0, 3.75]))
    );
    let quarter = blend(&clear, &storm, 0.25, ClockTurn::Shortest);
    assert!(near(quarter.number(Clouds, "Cover").unwrap(), 0.4375));

    assert_eq!(blend(&clear, &storm, 0.0, ClockTurn::Shortest), clear);
    assert_eq!(blend(&clear, &storm, 1.0, ClockTurn::Shortest), storm);
    // Out of range and not a number are held to the ends.
    assert_eq!(blend(&clear, &storm, -3.0, ClockTurn::Shortest), clear);
    assert_eq!(blend(&clear, &storm, 7.0, ClockTurn::Shortest), storm);
    assert_eq!(blend(&clear, &storm, f64::NAN, ClockTurn::Shortest), clear);
}

#[test]
fn switches_and_choices_change_half_way() {
    let mut off = EnvironmentPreset::new();
    off.set(Lighting, "ShadowsEnabled", PresetValue::Switch(false));
    off.set(Lighting, "BloomEnabled", PresetValue::Switch(true));
    let mut on = EnvironmentPreset::new();
    on.set(Lighting, "ShadowsEnabled", PresetValue::Switch(true));
    on.set(Lighting, "BloomEnabled", PresetValue::Switch(true));
    off.check().unwrap();
    let at = |t| {
        blend(&off, &on, t, ClockTurn::Shortest)
            .get(Lighting, "ShadowsEnabled")
            .cloned()
    };
    assert_eq!(at(0.49), Some(PresetValue::Switch(false)));
    assert_eq!(at(0.5), Some(PresetValue::Switch(true)));
    assert_eq!(at(0.9), Some(PresetValue::Switch(true)));

    // What falls is a choice: rain to snow changes half way.
    let (storm, snow) = (preset("Storm"), preset("SnowDay"));
    let kind = |t| {
        blend(&storm, &snow, t, ClockTurn::Shortest)
            .get(Weather, "Precipitation")
            .and_then(PresetValue::as_choice)
            .map(str::to_owned)
    };
    assert_eq!(kind(0.49).as_deref(), Some("Rain"));
    assert_eq!(kind(0.5).as_deref(), Some("Snow"));
}

#[test]
fn the_clock_turns_the_short_way_unless_told_otherwise() {
    // Through midnight.
    assert!(near(blend_clock(22.0, 2.0, 0.5, ClockTurn::Shortest), 0.0));
    assert!(near(
        blend_clock(22.0, 2.0, 0.25, ClockTurn::Shortest),
        23.0
    ));
    assert!(near(blend_clock(2.0, 22.0, 0.25, ClockTurn::Shortest), 1.0));
    assert!(near(
        blend_clock(2.0, 22.0, 0.75, ClockTurn::Shortest),
        23.0
    ));
    // No further than it has to.
    assert!(near(
        blend_clock(13.0, 15.0, 0.5, ClockTurn::Shortest),
        14.0
    ));
    assert!(near(
        blend_clock(15.0, 13.0, 0.5, ClockTurn::Shortest),
        14.0
    ));
    // Morning to evening is 14 hours on and 10 back: the short way is back
    // through midnight, which is why a key can say Forward.
    assert!(near(blend_clock(6.0, 20.0, 0.5, ClockTurn::Shortest), 1.0));
    assert!(near(blend_clock(6.0, 20.0, 0.5, ClockTurn::Forward), 13.0));
    assert!(near(blend_clock(20.0, 6.0, 0.5, ClockTurn::Forward), 1.0));
    assert!(near(blend_clock(20.0, 6.0, 0.5, ClockTurn::Backward), 13.0));
    assert!(near(blend_clock(13.0, 15.0, 0.5, ClockTurn::Backward), 2.0));
    // The same hour doesn't go round.
    for turn in ClockTurn::ALL {
        assert!(near(blend_clock(9.0, 9.0, 0.5, turn), 9.0));
    }

    // In a blend: SnowNight (19.3) to Clear (13.4) goes back 5.9 hours the
    // short way and on 18.1 hours forward.
    let (night, clear) = (preset("SnowNight"), preset("Clear"));
    let hour = |turn| {
        blend(&night, &clear, 0.5, turn)
            .number(Lighting, "ClockTime")
            .unwrap()
    };
    assert!(near(hour(ClockTurn::Shortest), 16.35));
    assert!(near(hour(ClockTurn::Forward), 4.35));
    // The ends are the presets' own hours whichever way it turns.
    for turn in ClockTurn::ALL {
        assert_eq!(blend(&night, &clear, 1.0, turn), clear);
        assert_eq!(blend(&night, &clear, 0.0, turn), night);
    }
}

#[test]
fn rain_turns_to_snow_through_nothing_falling() {
    let (storm, snow) = (preset("Storm"), preset("SnowDay"));
    let at = |t: f64| {
        let look = blend(&storm, &snow, t, ClockTurn::Shortest);
        (
            look.get(Weather, "Precipitation")
                .and_then(PresetValue::as_choice)
                .unwrap()
                .to_owned(),
            look.number(Weather, "Intensity").unwrap(),
        )
    };
    assert_eq!(at(0.0), ("Rain".to_owned(), 0.9));
    let (kind, hard) = at(0.25);
    assert_eq!(kind, "Rain");
    assert!(near(hard, 0.45), "rain eases off: {hard}");
    let (kind, hard) = at(0.499);
    assert_eq!(kind, "Rain");
    assert!(hard < 0.01, "rain has all but stopped: {hard}");
    let (kind, hard) = at(0.5);
    assert_eq!(kind, "Snow");
    assert!(near(hard, 0.0), "nothing falls as it changes: {hard}");
    let (kind, hard) = at(0.75);
    assert_eq!(kind, "Snow");
    assert!(near(hard, 0.3), "snow eases in: {hard}");
    assert_eq!(at(1.0), ("Snow".to_owned(), 0.6));
    // Never rain as hard as the straight blend would have it once snow
    // falls, and never snow before the rain has stopped.
    for step in 0..=100 {
        let t = f64::from(step) / 100.0;
        let (kind, hard) = at(t);
        if t < 0.5 {
            assert_eq!(kind, "Rain");
            assert!(hard <= 0.9 * (1.0 - 2.0 * t) + 1e-9);
        } else {
            assert_eq!(kind, "Snow");
            assert!(hard <= 0.6 * (2.0 * t - 1.0) + 1e-9);
        }
    }
    // Wetness and snow cover have something between, so they go straight.
    let half = blend(&storm, &snow, 0.5, ClockTurn::Shortest);
    assert!(near(half.number(Weather, "Wetness").unwrap(), 0.45));
    assert!(near(half.number(Weather, "SnowCover").unwrap(), 0.5));

    // From clear, rain starts gently from the first moment instead of
    // switching on half way at half its strength, and stops the same way.
    let clear = preset("Clear");
    let begins = blend(&clear, &storm, 0.1, ClockTurn::Shortest);
    assert_eq!(
        begins.get(Weather, "Precipitation").unwrap().as_choice(),
        Some("Rain")
    );
    assert!(near(begins.number(Weather, "Intensity").unwrap(), 0.09));
    let ends = blend(&storm, &clear, 0.9, ClockTurn::Shortest);
    assert_eq!(
        ends.get(Weather, "Precipitation").unwrap().as_choice(),
        Some("Rain")
    );
    assert!(near(ends.number(Weather, "Intensity").unwrap(), 0.09));
    let dry = blend(&storm, &clear, 1.0, ClockTurn::Shortest);
    assert_eq!(
        dry.get(Weather, "Precipitation").unwrap().as_choice(),
        Some("None")
    );
}

#[test]
fn a_value_a_preset_leaves_out_is_left_alone() {
    // A preset of only the wind.
    let mut windy = EnvironmentPreset::new();
    windy.set(
        Workspace,
        "GlobalWind",
        PresetValue::Triple([30.0, 0.0, 0.0]),
    );
    let mut world = world();
    let lighting = world.lighting();
    world
        .set_property(lighting, "ClockTime", Value::Number(7.0))
        .unwrap();
    let clouds = Clouds.find(&world).unwrap();
    world
        .set_property(clouds, "Cover", Value::Number(0.11))
        .unwrap();
    let before = world.clone();
    assert!(apply_preset(&mut world, &windy, |_, _| false).is_empty());
    assert_eq!(
        value(&world, Workspace, "GlobalWind"),
        Value::Vector3(Vector3::new(30.0, 0.0, 0.0))
    );
    assert_eq!(number(&world, Lighting, "ClockTime"), 7.0);
    assert_eq!(number(&world, Clouds, "Cover"), 0.11);
    for target in [Lighting, Atmosphere, Clouds, Weather, ColorCorrection] {
        let id = target.find(&world).unwrap();
        assert_eq!(
            world.get(id).unwrap().properties().collect::<Vec<_>>(),
            before.get(id).unwrap().properties().collect::<Vec<_>>(),
            "{target:?} wasn't touched"
        );
    }

    // Blended with a whole preset, the wind blends and the rest is the
    // whole one's: nothing is made up for what the other leaves out.
    let storm = preset("Storm");
    let half = blend(&storm, &windy, 0.5, ClockTurn::Shortest);
    assert_eq!(
        half.get(Workspace, "GlobalWind"),
        Some(&PresetValue::Triple([23.0, 0.0, 3.0]))
    );
    assert_eq!(half.number(Clouds, "Cover"), Some(0.85));
    // And two presets that both leave a value out blend to one that does.
    let mut calm = EnvironmentPreset::new();
    calm.set(
        Workspace,
        "GlobalWind",
        PresetValue::Triple([0.0, 0.0, 0.0]),
    );
    let half = blend(&calm, &windy, 0.5, ClockTurn::Shortest);
    assert_eq!(half.fields().count(), 1);
    assert!(half.get(Lighting, "ClockTime").is_none());
}

#[test]
fn applying_sets_every_kind_of_value_and_capture_reads_them_back() {
    let mut world = world();
    let storm = preset("Storm");
    assert!(apply_preset(&mut world, &storm, |_, _| false).is_empty());
    assert_eq!(number(&world, Lighting, "ClockTime"), 15.0);
    assert_eq!(number(&world, Lighting, "Exposure"), 1.0);
    assert_eq!(number(&world, Clouds, "Density"), 0.55);
    assert_eq!(number(&world, Atmosphere, "Haze"), 1.0);
    assert_eq!(number(&world, Clouds, "Cover"), 0.85);
    assert_eq!(
        value(&world, Clouds, "Color"),
        Value::Color3(Color3::new(0.5, 0.52, 0.58))
    );
    assert_eq!(
        value(&world, Weather, "Precipitation"),
        Value::EnumItem(EnumItem {
            enum_name: "Precipitation".into(),
            item: "Rain".into()
        })
    );
    assert_eq!(number(&world, Weather, "Wetness"), 0.9);
    assert_eq!(number(&world, ColorCorrection, "Saturation"), -0.25);
    assert_eq!(
        value(&world, Workspace, "GlobalWind"),
        Value::Vector3(Vector3::new(16.0, 0.0, 6.0))
    );

    // The world's look read back sets what Storm set to the same values.
    let captured = capture(&world);
    captured.check().unwrap();
    for (target, name, value) in storm.fields() {
        assert_eq!(captured.get(target, name), Some(value), "{target:?}.{name}");
    }
    // What a preset doesn't hold of the Weather stays the world's.
    assert!(captured.get(Weather, "TransitionTime").is_none());
    assert_eq!(capture_like(&world, &storm), storm);

    // Applying what the world already has changes nothing.
    let version = world.clone();
    apply_preset(&mut world, &storm, |_, _| false);
    assert_eq!(
        capture(&world),
        capture(&version),
        "applying it again leaves the look as it was"
    );

    // A world without Clouds or a Weather: their values go nowhere, and
    // the caller is told.
    let mut bare = World::new();
    let missing = apply_preset(&mut bare, &storm, |_, _| false);
    assert_eq!(missing, vec![Atmosphere, Clouds, Weather, ColorCorrection]);
    assert_eq!(number(&bare, Lighting, "ClockTime"), 15.0);
    assert!(capture(&bare).get(Clouds, "Cover").is_none());
}

#[test]
fn a_preset_file_round_trips_and_a_bad_one_says_why() {
    let mut dusk = EnvironmentPreset::new();
    dusk.set(Lighting, "ClockTime", PresetValue::Number(18.25));
    dusk.set(Clouds, "Color", PresetValue::Triple([1.0, 0.5, 0.25]));
    dusk.set(Weather, "Precipitation", PresetValue::Choice("Snow".into()));
    dusk.set(Lighting, "ShadowsEnabled", PresetValue::Switch(false));
    let text = dusk.to_json();
    assert_eq!(
        text,
        r#"{
  "format_version": 1,
  "Lighting": {
    "ClockTime": 18.25,
    "ShadowsEnabled": false
  },
  "Clouds": {
    "Color": [
      1.0,
      0.5,
      0.25
    ]
  },
  "Weather": {
    "Precipitation": "Snow"
  }
}
"#
    );
    assert_eq!(EnvironmentPreset::from_json(&text).unwrap(), dusk);
    assert_eq!(EnvironmentPreset::from_json(&text).unwrap().to_json(), text);

    let bad = |text: &str| EnvironmentPreset::from_json(text).unwrap_err().0;
    assert!(bad(r#"{"format_version": 2}"#).contains("format 2"));
    assert!(
        bad(r#"{"format_version": 1, "Lighting": {"Sunshine": 1}}"#).contains("Lighting.Sunshine")
    );
    assert!(
        bad(r#"{"format_version": 1, "Clouds": {"Cover": "lots"}}"#).contains("takes a number")
    );
    assert!(
        bad(r#"{"format_version": 1, "Weather": {"Precipitation": "Frogs"}}"#)
            .contains("None, Rain, Snow")
    );
    // Text can't be blended, so a preset doesn't hold it.
    assert!(
        bad(r#"{"format_version": 1, "Lighting": {"SkyImage": "a.hdr"}}"#)
            .contains("Lighting.SkyImage")
    );
    // Of the Workspace, only the wind.
    assert!(bad(r#"{"format_version": 1, "Workspace": {"Gravity": 10}}"#).contains("Gravity"));
    assert!(bad(r#"{"format_version": 1, "Sky": {}}"#).contains("Sky"));
}

fn project(test: &str) -> (std::path::PathBuf, Assets) {
    let folder = std::env::temp_dir().join(format!(
        "axcenity-environment-{test}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join(ENVIRONMENTS_FOLDER)).unwrap();
    let assets = Assets::new(&folder);
    (folder, assets)
}

#[test]
fn a_projects_presets_are_found_before_the_built_ins() {
    let (folder, assets) = project("find");
    let mut dusk = EnvironmentPreset::new();
    dusk.set(Lighting, "ClockTime", PresetValue::Number(18.25));
    std::fs::write(folder.join(preset_path("Dusk")), dusk.to_json()).unwrap();
    // The project's own Storm: only a harder wind.
    let mut storm = EnvironmentPreset::new();
    storm.set(
        Workspace,
        "GlobalWind",
        PresetValue::Triple([40.0, 0.0, 0.0]),
    );
    std::fs::write(folder.join(preset_path("Storm")), storm.to_json()).unwrap();
    std::fs::write(folder.join(ENVIRONMENTS_FOLDER).join("notes.txt"), "x").unwrap();

    assert_eq!(find_preset(&assets, "Dusk").unwrap(), dusk);
    assert_eq!(find_preset(&assets, "Storm").unwrap(), storm);
    assert_eq!(find_preset(&assets, "Clear").unwrap(), preset("Clear"));
    assert_eq!(project_presets(&assets), ["Dusk", "Storm"]);
    assert_eq!(
        preset_names(&assets),
        [
            "Clear",
            "Evening",
            "Overcast",
            "Storm",
            "SnowDay",
            "SnowNight",
            "Dusk"
        ]
    );
    let why = find_preset(&assets, "Monsoon").unwrap_err();
    assert!(why.contains("Monsoon") && why.contains("Clear"), "{why}");
    assert!(find_preset(&assets, "../world").is_err());
    assert!(find_preset(&assets, "").is_err());
    // A broken file is an error, not quietly the built-in.
    std::fs::write(folder.join(preset_path("Clear")), "{").unwrap();
    assert!(find_preset(&assets, "Clear").is_err());
    // With no project, the built-ins.
    assert_eq!(
        find_preset(&Assets::default(), "Storm").unwrap(),
        preset("Storm")
    );
    assert_eq!(
        preset_name("assets/environments/Dusk.environment.json"),
        Some("Dusk")
    );
    assert_eq!(preset_name("assets/sequences/Intro.sequence.json"), None);
    let _ = std::fs::remove_dir_all(&folder);
}

fn linear(id: &str, time: f64, preset: &str) -> EnvironmentKey {
    EnvironmentKey {
        interp: Interp::Linear,
        easing: None,
        ..EnvironmentKey::new(id, time, preset)
    }
}

#[test]
fn a_track_of_three_keys_is_the_blend_of_the_two_around_each_time() {
    let keys = [
        linear("a", 2.0, "Clear"),
        linear("b", 6.0, "Storm"),
        linear("c", 10.0, "SnowDay"),
    ];
    let track = EnvironmentTrack::new(&keys, built_in);
    assert!(track.missing.is_empty());
    let (clear, storm, snow) = (preset("Clear"), preset("Storm"), preset("SnowDay"));
    let short = ClockTurn::Shortest;
    // Held before the first key and after the last.
    assert_eq!(track.sample(0.0).unwrap(), clear);
    assert_eq!(track.sample(2.0).unwrap(), clear);
    assert_eq!(track.sample(10.0).unwrap(), snow);
    assert_eq!(track.sample(99.0).unwrap(), snow);
    // On a key, that key's preset exactly.
    assert_eq!(track.sample(6.0).unwrap(), storm);
    // Between keys, their blend.
    for (time, from, to, t) in [
        (3.0, &clear, &storm, 0.25),
        (4.0, &clear, &storm, 0.5),
        (5.0, &clear, &storm, 0.75),
        (7.0, &storm, &snow, 0.25),
        (8.0, &storm, &snow, 0.5),
        (9.5, &storm, &snow, 0.875),
    ] {
        assert_eq!(
            track.sample(time).unwrap(),
            blend(from, to, t, short),
            "at {time}s"
        );
    }
    // Keys given out of order are the same track.
    let shuffled = [keys[2].clone(), keys[0].clone(), keys[1].clone()];
    assert_eq!(EnvironmentTrack::new(&shuffled, built_in), track);

    // A key's easing bends how far along the blend is, as a property key's.
    let mut eased = keys.clone();
    eased[0] = EnvironmentKey::new("a", 2.0, "Clear");
    let track = EnvironmentTrack::new(&eased, built_in);
    let sine = Easing::new(EasingStyle::Sine, EasingDirection::InOut);
    assert_eq!(
        track.sample(3.0).unwrap(),
        blend(&clear, &storm, f64::from(sine.apply(0.25)), short)
    );
    assert!(
        track.sample(3.0).unwrap().number(Clouds, "Cover").unwrap()
            < blend(&clear, &storm, 0.25, short)
                .number(Clouds, "Cover")
                .unwrap(),
        "eased in, it has moved less by a quarter of the time"
    );
    // A Step key holds its preset until the next.
    eased[0].interp = Interp::Step;
    let track = EnvironmentTrack::new(&eased, built_in);
    assert_eq!(track.sample(5.9).unwrap(), clear);
    assert_eq!(track.sample(6.0).unwrap(), storm);
    // A key can say which way the clock turns to the next.
    let mut forward = [linear("a", 0.0, "SnowNight"), linear("b", 10.0, "Clear")];
    forward[0].clock = ClockTurn::Forward;
    let track = EnvironmentTrack::new(&forward, built_in);
    assert!(near(
        track
            .sample(5.0)
            .unwrap()
            .number(Lighting, "ClockTime")
            .unwrap(),
        4.35
    ));

    // No keys: nothing to apply. A preset that isn't there: its key is
    // left out and named.
    assert!(EnvironmentTrack::new(&[], built_in).sample(1.0).is_none());
    let track = EnvironmentTrack::new(
        &[linear("a", 0.0, "Clear"), linear("b", 5.0, "Monsoon")],
        built_in,
    );
    assert_eq!(track.missing.iter().collect::<Vec<_>>(), ["Monsoon"]);
    assert_eq!(track.sample(5.0).unwrap(), clear);
}

#[test]
fn a_key_that_leaves_a_value_out_lets_it_blend_across() {
    // Clear, then a preset of only the wind, then Storm: the sky goes from
    // Clear to Storm over the whole stretch, and only the wind stops by.
    let mut windy = EnvironmentPreset::new();
    windy.set(
        Workspace,
        "GlobalWind",
        PresetValue::Triple([30.0, 0.0, 0.0]),
    );
    let find = |name: &str| match name {
        "Windy" => Some(windy.clone()),
        other => built_in(other),
    };
    let keys = [
        linear("a", 0.0, "Clear"),
        linear("b", 5.0, "Windy"),
        linear("c", 10.0, "Storm"),
    ];
    let track = EnvironmentTrack::new(&keys, find);
    let cover = |time| track.sample(time).unwrap().number(Clouds, "Cover").unwrap();
    assert!(near(cover(0.0), 0.3));
    assert!(near(cover(2.5), 0.4375));
    assert!(
        near(cover(5.0), 0.575),
        "no jump at the key that leaves it out"
    );
    assert!(near(cover(7.5), 0.7125));
    assert!(near(cover(10.0), 0.85));
    let wind = |time| match track.sample(time).unwrap().get(Workspace, "GlobalWind") {
        Some(PresetValue::Triple(wind)) => wind[0],
        _ => panic!("the wind is three numbers"),
    };
    assert_eq!(wind(0.0), 4.0);
    assert_eq!(wind(5.0), 30.0);
    assert_eq!(wind(7.5), 23.0);
    assert_eq!(wind(10.0), 16.0);
    // Rain starts at the first key and builds to the last.
    let look = track.sample(5.0).unwrap();
    assert_eq!(
        look.get(Weather, "Precipitation").unwrap().as_choice(),
        Some("Rain")
    );
    assert!(near(look.number(Weather, "Intensity").unwrap(), 0.45));
}

/// A sequence 10 seconds long with Clear at 0, Storm at 4 and SnowDay at 10.
fn weather_sequence() -> SequenceFile {
    let mut file = SequenceFile::new(10.0);
    file.environment = vec![
        linear("a", 0.0, "Clear"),
        linear("b", 4.0, "Storm"),
        linear("c", 10.0, "SnowDay"),
    ];
    file
}

#[test]
fn a_sequence_applies_the_blend_of_its_environment_keys_at_any_time() {
    let mut world = world();
    let mut run = SequenceRun::new(weather_sequence()).unwrap();
    let (clear, storm, snow) = (preset("Clear"), preset("Storm"), preset("SnowDay"));
    // In any order of times: where it is depends only on the time.
    for (time, from, to, t) in [
        (7.0, &storm, &snow, 0.5),
        (1.0, &clear, &storm, 0.25),
        (10.0, &storm, &snow, 1.0),
        (2.0, &clear, &storm, 0.5),
        (0.0, &clear, &storm, 0.0),
        (8.5, &storm, &snow, 0.75),
        (4.0, &clear, &storm, 1.0),
    ] {
        run.apply(&mut world, time);
        let want = blend(from, to, t, ClockTurn::Shortest);
        let got = capture_like(&world, &want);
        for (target, name, value) in want.fields() {
            let got = got.get(target, name).unwrap();
            match (got, value) {
                (PresetValue::Number(got), PresetValue::Number(want)) => {
                    assert!(
                        near(*got, *want),
                        "{target:?}.{name} at {time}s: {got} not {want}"
                    )
                }
                _ => assert_eq!(got, value, "{target:?}.{name} at {time}s"),
            }
        }
    }
    assert!(run.missing.is_empty(), "{:?}", run.missing);
    // At 7 s rain has just stopped and snow not begun.
    run.apply(&mut world, 7.0);
    assert_eq!(falling(&world), "Snow");
    assert!(near(number(&world, Weather, "Intensity"), 0.0));
    run.apply(&mut world, 5.5);
    assert_eq!(falling(&world), "Rain");
    assert!(near(number(&world, Weather, "Intensity"), 0.45));
}

fn track(target: &str, name: &str, keys: &[(f64, &[f32])]) -> PropertyTrack {
    PropertyTrack {
        id: format!("{target}.{name}"),
        target: target.to_owned(),
        property: PropertyName::from_name(name).unwrap(),
        keys: keys
            .iter()
            .enumerate()
            .map(|(i, (time, value))| PropertyKeyframe::new(format!("k{i}"), *time, value.to_vec()))
            .collect(),
    }
}

#[test]
fn a_property_track_wins_over_the_environment_track_for_its_property() {
    let mut file = weather_sequence();
    // The clouds' cover and the wind are keyed by hand.
    file.tracks.push(track(
        "Lighting/Clouds",
        "Cover",
        &[(0.0, &[0.05]), (10.0, &[0.15])],
    ));
    file.tracks.push(track(
        "Workspace",
        "GlobalWind",
        &[(0.0, &[0.0, 0.0, 0.0]), (10.0, &[0.0, 0.0, 50.0])],
    ));
    // A track with no keys says nothing, so it overrides nothing.
    file.tracks.push(track("Lighting", "Exposure", &[]));
    let mut world = world();
    let clouds = Clouds.find(&world).unwrap();
    let mut run = SequenceRun::new(file).unwrap();
    world.record_changes();
    for (time, cover, wind, density, exposure) in [
        (0.0, 0.05, 0.0, 0.7, 1.0),
        (2.0, 0.07, 10.0, 0.625, 1.0),
        (4.0, 0.09, 20.0, 0.55, 1.0),
        (10.0, 0.15, 50.0, 0.5, 1.0),
    ] {
        // Count what's written: the Environment track must not set the
        // cover first for the property track to set it again.
        run.apply(&mut world, time);
        assert!(
            near(number(&world, Clouds, "Cover"), cover),
            "the property track's cover at {time}s"
        );
        let writes = world
            .take_changes()
            .into_iter()
            .filter(|recorded| {
                recorded.change
                    == axcenity_core::Change::Property {
                        instance: clouds,
                        name: "Cover",
                    }
            })
            .count();
        assert_eq!(writes, 1, "the cover is set once a frame, by its own track");
        let Value::Vector3(blows) = value(&world, Workspace, "GlobalWind") else {
            panic!("the wind is a Vector3");
        };
        assert!(
            near(f64::from(blows.z), wind),
            "the property track's wind at {time}s"
        );
        assert_eq!(blows.x, 0.0);
        // The rest of the clouds and the sky are still the presets'.
        assert!(
            near(number(&world, Clouds, "Density"), density),
            "at {time}s"
        );
        assert!(
            near(number(&world, Lighting, "Exposure"), exposure),
            "at {time}s"
        );
    }
    assert!(run.missing.is_empty(), "{:?}", run.missing);
}

#[test]
fn the_environment_track_round_trips_and_files_without_one_are_unchanged() {
    let mut file = weather_sequence();
    file.environment[0] = EnvironmentKey::new("a", 0.0, "Clear");
    file.environment[1].clock = ClockTurn::Forward;
    let text = file.to_json();
    assert!(
        text.contains(
            r#"  "environment": [
    {
      "id": "a",
      "time": 0.0,
      "preset": "Clear",
      "interp": "Ease",
      "easing": {
        "style": "Sine",
        "direction": "InOut"
      }
    },
    {
      "id": "b",
      "time": 4.0,
      "preset": "Storm",
      "interp": "Linear",
      "clock": "Forward"
    },"#
        ),
        "{text}"
    );
    let read = SequenceFile::from_json(&text).unwrap();
    assert_eq!(read, file);
    assert_eq!(read.to_json(), text);
    assert_eq!(SEQUENCE_FORMAT, 1, "the track is optional: no new format");

    // Files from before the track: byte for byte what they were.
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples/projects");
    for path in [
        "cutscene/assets/sequences/Intro.sequence.json",
        "splines/assets/sequences/Flyby.sequence.json",
    ] {
        let old = std::fs::read_to_string(examples.join(path))
            .unwrap()
            .replace("\r\n", "\n");
        assert!(!old.contains("environment"));
        let file = SequenceFile::from_json(&old).unwrap();
        assert!(file.environment.is_empty());
        assert_eq!(file.to_json(), old, "{path}");
    }
    let old = "{\n  \"format_version\": 1,\n  \"duration\": 10.0\n}\n";
    assert_eq!(SequenceFile::from_json(old).unwrap().to_json(), old);
    assert_eq!(SequenceFile::new(10.0).to_json(), old);

    // Keys are read in time order, and a bad one says why.
    let mut shuffled = weather_sequence();
    shuffled.environment.reverse();
    let read = SequenceFile::from_json(&shuffled.to_json()).unwrap();
    assert_eq!(read.environment, weather_sequence().environment);
    let mut late = weather_sequence();
    late.environment[2].time = 11.0;
    let why = SequenceFile::from_json(&late.to_json()).unwrap_err().0;
    assert!(why.contains("environment key `c`"), "{why}");
    let mut unnamed = weather_sequence();
    unnamed.environment[0].preset = "../world".into();
    let why = SequenceFile::from_json(&unnamed.to_json()).unwrap_err().0;
    assert!(why.contains("preset"), "{why}");
}

#[test]
fn a_sequence_finds_the_projects_presets_and_names_what_it_cant_find() {
    let (folder, assets) = project("sequence");
    let mut dusk = EnvironmentPreset::new();
    dusk.set(Lighting, "ClockTime", PresetValue::Number(18.0));
    std::fs::write(folder.join(preset_path("Dusk")), dusk.to_json()).unwrap();
    let mut file = SequenceFile::new(10.0);
    file.environment = vec![
        linear("a", 0.0, "Clear"),
        linear("b", 10.0, "Dusk"),
        linear("c", 5.0, "Monsoon"),
    ];
    let mut world = World::new();
    // Without the project, only the built-ins are known.
    let mut run = SequenceRun::new(file.clone()).unwrap();
    run.apply(&mut world, 10.0);
    assert!(run.missing.contains("the environment preset Dusk"));
    assert_eq!(number(&world, Lighting, "ClockTime"), 13.4);

    let mut run = SequenceRun::new(file).unwrap().with_presets(&assets);
    run.apply(&mut world, 5.0);
    assert!(near(number(&world, Lighting, "ClockTime"), 15.7));
    run.apply(&mut world, 10.0);
    assert_eq!(number(&world, Lighting, "ClockTime"), 18.0);
    assert!(!run.missing.contains("the environment preset Dusk"));
    assert!(run.missing.contains("the environment preset Monsoon"));
    // Clear has clouds and weather and this world has neither: said once,
    // by name. No ColorCorrectionEffect is nothing to report.
    assert!(
        run.missing
            .contains("Lighting/Clouds (for the Environment track)")
    );
    assert!(
        run.missing
            .contains("Lighting/Weather (for the Environment track)")
    );
    assert!(
        !run.missing
            .iter()
            .any(|missing| missing.contains("ColorCorrection"))
    );
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_transition_eases_from_how_the_world_is_and_another_takes_over_from_there() {
    let mut world = world();
    apply_preset(&mut world, &preset("Clear"), |_, _| false);
    let mut to_storm =
        EnvironmentTransition::start(&world, preset("Storm"), 10.0, ClockTurn::Shortest);
    assert!(to_storm.missing.is_empty());
    assert!(!to_storm.step(&mut world, 0.0));
    assert!(
        near(number(&world, Clouds, "Cover"), 0.3),
        "it starts where the world is"
    );
    // Slow at first: a tenth of the time is under a tenth of the way.
    assert!(!to_storm.step(&mut world, 1.0));
    let early = number(&world, Clouds, "Cover");
    assert!(early > 0.3 && early < 0.3 + 0.55 * 0.1, "eased in: {early}");
    assert!(!to_storm.step(&mut world, 4.0));
    assert!(
        near(number(&world, Clouds, "Cover"), 0.575),
        "half way at half time"
    );
    assert!(near(number(&world, Lighting, "ClockTime"), 14.2));
    assert_eq!(falling(&world), "Rain");
    assert!(near(number(&world, Weather, "Intensity"), 0.45));

    // Another takes over mid-way from where the world is now: no jump.
    let mut to_snow =
        EnvironmentTransition::start(&world, preset("SnowDay"), 4.0, ClockTurn::Shortest);
    to_snow.step(&mut world, 0.0);
    assert!(near(number(&world, Clouds, "Cover"), 0.575));
    assert!(near(number(&world, Weather, "Intensity"), 0.45));
    to_snow.step(&mut world, 2.0);
    assert!(near(number(&world, Clouds, "Cover"), 0.6875));
    assert_eq!(falling(&world), "Snow");
    assert!(near(number(&world, Weather, "Intensity"), 0.0));
    assert!(to_snow.step(&mut world, 2.0), "it has arrived");
    assert_eq!(capture_like(&world, &preset("SnowDay")), preset("SnowDay"));
    assert!(to_snow.step(&mut world, 5.0), "and stays arrived");

    // No seconds: there at the first step.
    let mut at_once =
        EnvironmentTransition::start(&world, preset("Clear"), 0.0, ClockTurn::Shortest);
    assert!(at_once.step(&mut world, 0.0));
    assert_eq!(capture_like(&world, &preset("Clear")), preset("Clear"));
    // In a world without what the preset sets, the caller is told.
    let bare = World::new();
    let started = EnvironmentTransition::start(&bare, preset("Storm"), 1.0, ClockTurn::Shortest);
    assert_eq!(
        started.missing,
        vec![Atmosphere, Clouds, Weather, ColorCorrection]
    );
}
