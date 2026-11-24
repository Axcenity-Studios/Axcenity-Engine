//! `Lighting:TransitionToPreset` and a Sequence's Environment track, in a
//! game made here.

use std::path::PathBuf;

use axcenity_anim::environment::{
    EnvironmentKey, EnvironmentPreset, EnvironmentTarget, PresetValue, built_in, capture_like,
    preset_path,
};
use axcenity_anim::sequence::SequenceFile;
use axcenity_core::{ClassName, World};

use super::*;

const FRAME: f64 = 0.1;

struct Game {
    world: World,
    runtime: ScriptRuntime,
    folder: PathBuf,
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// A game with Clouds, a Weather and a ColorCorrectionEffect in Lighting
/// (and no Atmosphere), looking like the Clear preset.
fn game(test: &str) -> Game {
    let folder = std::env::temp_dir().join(format!(
        "axcenity-environment-{test}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join("assets/environments")).unwrap();
    std::fs::create_dir_all(folder.join("assets/sequences")).unwrap();
    let mut world = World::new();
    let lighting = world.lighting();
    for class in [
        ClassName::Clouds,
        ClassName::Weather,
        ClassName::ColorCorrectionEffect,
    ] {
        let id = world.create(class).unwrap();
        world.set_parent(id, Some(lighting)).unwrap();
    }
    axcenity_anim::environment::apply_preset(&mut world, &built_in("Clear").unwrap(), |_, _| false);
    let mut runtime = ScriptRuntime::new();
    runtime.set_assets(Assets::new(&folder));
    Game {
        world,
        runtime,
        folder,
    }
}

impl Game {
    fn command(&mut self, source: &str) {
        self.runtime.run_command(&mut self.world, source).unwrap();
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.runtime.input_events(&mut self.world, &[]);
            self.runtime.step(&mut self.world, FRAME);
        }
    }

    fn printed(&mut self) -> Vec<String> {
        self.runtime
            .take_output()
            .into_iter()
            .map(|line| match line.kind {
                OutputKind::Error => format!("error: {}", line.text),
                OutputKind::Warning => format!("warning: {}", line.text),
                OutputKind::Print => line.text,
            })
            .collect()
    }

    fn number(&self, target: EnvironmentTarget, name: &str) -> f64 {
        let id = target.find(&self.world).unwrap();
        let instance = self.world.get(id).unwrap();
        instance.property(name).unwrap().as_number().unwrap()
    }

    fn falling(&self) -> String {
        let id = EnvironmentTarget::Weather.find(&self.world).unwrap();
        match self.world.get(id).unwrap().property("Precipitation") {
            Some(Value::EnumItem(item)) => item.item.clone(),
            other => panic!("Precipitation is {other:?}"),
        }
    }

    /// Whether the world looks like `preset` in everything it has.
    fn looks_like(&self, preset: &str) -> bool {
        let mut preset = built_in(preset).unwrap();
        // This game has no Atmosphere.
        preset.atmosphere.clear();
        capture_like(&self.world, &preset) == preset
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-4
}

const CLOUDS: EnvironmentTarget = EnvironmentTarget::Clouds;
const WEATHER: EnvironmentTarget = EnvironmentTarget::Weather;
const LIGHTING: EnvironmentTarget = EnvironmentTarget::Lighting;

#[test]
fn a_transition_eases_to_the_preset_over_its_seconds() {
    let mut game = game("eases");
    game.command("game:GetService('Lighting'):TransitionToPreset('Storm', 2)");
    // The call itself changes nothing: it starts from how the world is.
    assert!(game.looks_like("Clear"));
    // This game has no Atmosphere, and Storm has haze: said once.
    let printed = game.printed();
    assert_eq!(printed.len(), 1, "{printed:?}");
    assert!(
        printed[0].starts_with("warning: ") && printed[0].contains("an Atmosphere"),
        "{printed:?}"
    );

    game.frames(2);
    let early = game.number(CLOUDS, "Cover");
    assert!(
        early > 0.3 && early < 0.3 + 0.55 * 0.1,
        "eased in, a tenth of the time is less than a tenth of the way: {early}"
    );
    game.frames(8);
    assert!(
        near(game.number(CLOUDS, "Cover"), 0.575),
        "half way at half time"
    );
    assert!(near(game.number(LIGHTING, "ClockTime"), 14.2));
    assert!(near(game.number(CLOUDS, "Density"), 0.625));
    assert_eq!(game.falling(), "Rain");
    assert!(near(game.number(WEATHER, "Intensity"), 0.45));
    game.frames(8);
    let late = game.number(CLOUDS, "Cover");
    assert!(late > 0.85 - 0.55 * 0.1 && late < 0.85, "eased out: {late}");
    game.frames(2);
    assert!(game.looks_like("Storm"));
    // Arrived, it lets go: a script's own change stays.
    game.command("game:GetService('Lighting').Clouds.Cover = 0.2");
    game.frames(3);
    assert_eq!(game.number(CLOUDS, "Cover"), 0.2);
    assert!(game.printed().is_empty());
}

#[test]
fn another_call_takes_over_mid_way_from_where_the_world_is() {
    let mut game = game("superseded");
    game.command(
        "local Lighting = game:GetService('Lighting')\n\
         Lighting:TransitionToPreset('Storm', 2)\n\
         task.wait(1)\n\
         Lighting:TransitionToPreset('SnowDay', 1)",
    );
    game.printed();
    game.frames(10);
    // A second in, nearly half way to Storm, the second call has just
    // taken over (`task.wait(1)` resumes on the tenth frame, before the
    // transition moves on).
    let cover = game.number(CLOUDS, "Cover");
    assert!(
        (cover - 0.575).abs() < 0.05,
        "no jump as it takes over: {cover}"
    );
    assert_eq!(game.falling(), "Rain");
    let rain = game.number(WEATHER, "Intensity");
    assert!(
        rain > 0.3 && rain < 0.45,
        "rain as hard as it had got: {rain}"
    );
    // It never gets to Storm: the cover stays under Storm's on its way to
    // SnowDay's 0.8, and rain stops before snow starts.
    let mut snowed = false;
    for _ in 0..12 {
        game.frames(1);
        assert!(game.number(CLOUDS, "Cover") <= 0.8 + 1e-9);
        match game.falling().as_str() {
            "Rain" => {
                assert!(!snowed, "rain doesn't come back");
                assert!(game.number(WEATHER, "Intensity") <= rain + 1e-9);
            }
            "Snow" => snowed = true,
            other => panic!("{other} falls"),
        }
    }
    assert!(snowed);
    assert!(game.looks_like("SnowDay"));
    assert!(game.printed().is_empty());
}

#[test]
fn no_seconds_is_at_once_and_stop_leaves_the_world_where_it_is() {
    let mut game = game("at-once");
    game.command(
        "local Lighting = game:GetService('Lighting')\n\
         Lighting:TransitionToPreset('Overcast')\n\
         print(Lighting.Clouds.Cover, Lighting.Clouds.Density)",
    );
    // There before the call returns (after the one warning that this game
    // has no Atmosphere for Overcast's haze).
    let printed = game.printed();
    assert_eq!(printed.len(), 2, "{printed:?}");
    assert_eq!(printed[1], "0.8\t0.6");
    assert!(game.looks_like("Overcast"));

    game.command("game:GetService('Lighting'):TransitionToPreset('Clear', 2)");
    game.frames(10);
    let half = game.number(CLOUDS, "Cover");
    assert!(near(half, 0.55), "{half}");
    game.command("game:GetService('Lighting'):StopPresetTransition()");
    game.frames(10);
    assert_eq!(game.number(CLOUDS, "Cover"), half, "stopped where it was");
    // Stopping with none under way is fine.
    game.command("game:GetService('Lighting'):StopPresetTransition()");
    assert!(game.printed().is_empty());
}

#[test]
fn the_clock_turns_the_way_asked_and_bad_arguments_say_why() {
    let mut game = game("clock");
    // Clear is at 13.4 and SnowNight at 19.3: backward is 18.1 hours.
    game.command("game:GetService('Lighting'):TransitionToPreset('SnowNight', 2, 'Backward')");
    game.frames(10);
    assert!(near(game.number(LIGHTING, "ClockTime"), 4.35));
    game.frames(10);
    assert!(near(game.number(LIGHTING, "ClockTime"), 19.3));

    game.printed();
    let mut fails = |source: &str| {
        game.command(source);
        let printed = game.printed();
        assert_eq!(printed.len(), 1, "{printed:?}");
        assert!(printed[0].starts_with("error: "), "{printed:?}");
        printed[0].clone()
    };
    let why = fails("game:GetService('Lighting'):TransitionToPreset('Monsoon', 1)");
    assert!(why.contains("Monsoon") && why.contains("Storm"), "{why}");
    let why = fails("game:GetService('Lighting'):TransitionToPreset(5)");
    assert!(why.contains("preset's name"), "{why}");
    let why = fails("game:GetService('Lighting'):TransitionToPreset('Storm', -1)");
    assert!(why.contains("seconds"), "{why}");
    let why = fails("game:GetService('Lighting'):TransitionToPreset('Storm', 1, 'Sideways')");
    assert!(why.contains("Forward"), "{why}");
    // None of them started anything.
    game.frames(5);
    assert!(near(game.number(LIGHTING, "ClockTime"), 19.3));
}

#[test]
fn a_projects_own_preset_is_found_by_name_and_touches_only_what_it_sets() {
    let mut game = game("project");
    let mut gale = EnvironmentPreset::new();
    gale.set(
        EnvironmentTarget::Workspace,
        "GlobalWind",
        PresetValue::Triple([40.0, 0.0, 0.0]),
    );
    std::fs::write(game.folder.join(preset_path("Gale")), gale.to_json()).unwrap();
    game.command("game:GetService('Lighting'):TransitionToPreset('Gale', 1)");
    game.frames(5);
    let wind = |game: &Game| {
        let workspace = game.world.get(game.world.workspace()).unwrap();
        workspace
            .property("GlobalWind")
            .unwrap()
            .as_vector3()
            .unwrap()
    };
    assert!((wind(&game).x - 22.0).abs() < 1e-3, "{:?}", wind(&game));
    game.frames(5);
    assert_eq!(wind(&game).x, 40.0);
    assert_eq!(wind(&game).z, 0.0);
    // The sky is as it was.
    let mut clear = built_in("Clear").unwrap();
    clear.workspace.clear();
    clear.atmosphere.clear();
    assert_eq!(capture_like(&game.world, &clear), clear);
    assert!(game.printed().is_empty());
}

#[test]
fn a_sequences_environment_track_plays_in_the_game_and_wins_over_a_transition() {
    let mut game = game("sequence");
    // Two seconds from Clear to SnowDay, straight.
    let mut file = SequenceFile::new(2.0);
    for (id, time, preset) in [("a", 0.0, "Clear"), ("b", 2.0, "SnowDay")] {
        file.environment.push(EnvironmentKey {
            interp: axcenity_anim::Interp::Linear,
            easing: None,
            ..EnvironmentKey::new(id, time, preset)
        });
    }
    std::fs::write(
        game.folder.join("assets/sequences/Weather.sequence.json"),
        file.to_json(),
    )
    .unwrap();
    let workspace = game.world.workspace();
    let sequence = game.world.create(ClassName::Sequence).unwrap();
    game.world.set_name(sequence, "Weather").unwrap();
    game.world.set_parent(sequence, Some(workspace)).unwrap();
    game.world
        .set_property(
            sequence,
            "SequenceId",
            Value::String("assets/sequences/Weather.sequence.json".into()),
        )
        .unwrap();
    // A transition to Storm is under way when the sequence starts: the
    // sequence's keys are applied after it each frame, so they show.
    game.command(
        "game:GetService('Lighting'):TransitionToPreset('Storm', 4)\n\
         workspace.Weather:Play()",
    );
    game.printed();
    game.frames(10);
    assert!(
        near(game.number(CLOUDS, "Cover"), 0.55),
        "half way from Clear to SnowDay"
    );
    assert_eq!(game.falling(), "Snow");
    assert!(near(game.number(WEATHER, "Intensity"), 0.3));
    assert!(near(game.number(WEATHER, "SnowCover"), 0.5));
    game.frames(10);
    assert!(game.looks_like("SnowDay"));
    // The sequence has ended and the transition still has two seconds to
    // run: it carries on toward Storm from there.
    game.frames(25);
    assert!(game.looks_like("Storm"));
    let printed = game.printed();
    assert!(
        printed.iter().all(|line| !line.starts_with("error")),
        "{printed:?}"
    );
}
