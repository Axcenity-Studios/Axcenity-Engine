# Cutscene sequences

Phase 51. A sequence is a timeline of tracks in an `assets/sequences/<name>.sequence.json` file, made in the editor's **Sequencer** tab and played by a **Sequence** Instance. Like Unreal's Sequencer or Unity's Timeline.

## The file

`anim::sequence::SequenceFile` (`crates/anim/src/sequence.rs`), format 1, written with sorted keys and empty lists left out (`to_json`; `crates/samples/projects/cutscene` is canonical). `from_json` checks every section starts before it ends, inside `0..duration`. Fields added since (a cut's `transition`, the `environment` track) are optional and left out when empty or at their default, so the format is still 1 and older files read and write unchanged, byte for byte; an older engine refuses a file that has one.

| Field | What it is |
|---|---|
| `duration` | Seconds from start to end. |
| `camera_cuts` | `{ id, time, camera, transition }`: from `time` to the next cut, the view is the bound Camera's `Position`, `Orientation` and `FieldOfView`. Bound to a Spline (Phase 73), the camera rides it from its start at `time` to its end at the next cut (or the end), facing along it ([../architecture/splines.md](../architecture/splines.md)). An empty `camera` uses the sequence's own camera keys. Before the first cut the camera is the game's. `transition` is how the cut takes over (below); left out for a plain cut. |
| `tracks` | Property tracks, the same shape as a clip's (keys, easings, Bezier handles, `sample_track`). `target` is a binding; the target `@Camera` holds the sequence's own camera keys (`Position`, `Orientation`, `FieldOfView`). A Model's `Position` and `Orientation` are its pivot, as in clips. A sequence keys more properties than a clip (below). |
| `environment` | The Environment track's keys, `{ id, time, preset, interp, easing, bezier, clock }`, in time order: at `time` the sky and the weather look like the environment preset called `preset`, and between two keys they blend ([below](#environment-presets-and-the-environment-track)). Left out when there are none. |
| `animations` | `{ id, start, end, target, clip, blend_in, blend_out }`: the clip (`*.anim.json`) plays from its start on the bound Animator, or the Animator inside the bound Instance, its weight rising over `blend_in` and falling over `blend_out` seconds. |
| `audio` | `{ id, start, end, sound }`: the bound Sound plays from its start at `start` and stops at `end`. |
| `events` | `{ time, name }`: fired as `EventReached(name)` when passed. |
| `fades` | `{ id, start, end, from, to, color }`: the screen covered in `color` from `from` (0 clear, 1 covered) to `to`; after it, held at `to` until the next fade. |
| `letterbox` | `{ id, start, end, size, ramp }`: bars at the top and bottom, each `size` of the screen's height (0.12), sliding in and out over `ramp` seconds. |
| `subtitles` | `{ id, start, end, key, text }`: a line at the bottom showing the string table's `key` in the player's language (a TextLabel's `TextKey`), or `text` when the table hasn't got it. |
| `levels` | `{ id, start, end, level }`: the level is added beside the current one for the section (`LevelService:AddLevel`); one that reaches the end stays added. |

**Bindings** find what a track moves: a path of names from `game` (`Workspace/Door`, `Lighting`, `Lighting/Clouds`, `Workspace` itself), or `tag:` and a tag (`tag:Hero`), the first Instance in tree order with that tag, so placed prefab copies can be bound. A binding that finds nothing is warned about once.

### Transitions

A cut's `transition` (`sequence::Transition`), written as `{ "kind": …, "duration": … }`:

| Kind | What it does | Where |
|---|---|---|
| `Cut` | From one frame to the next. The default; not written. | Everywhere. |
| `Dissolve` | The shot before fades into this one over `duration` seconds while both carry on. | Rendered movies ([../editor/movie.md](../editor/movie.md)). The game and the Sequencer's preview show a cut at the cut's time: a dissolve draws the world twice each frame, which they don't. |
| `FadeThrough` | The screen fades to `color` (black if left out), the cut happens behind it, and it clears. What a fade out and a fade in around the cut do. | Everywhere: the overlay's `Fade` draws it. |

A transition is centred on the cut: half before, half after. It stops at the sequence's ends and half way to the cut before and the cut after, so two never overlap (`SequenceFile::transition_span`). `cover_at` is the fade sections or a fade through, whichever covers more. During a dissolve `Applied::dissolve` holds both shots and the mix (`from`, `to`, `weight`): the outgoing shot is its camera carried on past the cut (a Spline's camera stays at its end), the incoming one its camera before the cut (a Spline's waits at its start). `Applied::camera` still cuts at the cut's time, which is all a host with one view needs.

### What a track can key

Every property a clip keys (`PropertyName::keyable`: `Position`, `Orientation`, `Size`, `Color`, `Transparency`, the UI ones, `FieldOfView`, bones, face controls), and for sequences only the **scene's** (`PropertyName::Scene`, `files::scene_properties`, `PropertyName::sequence_keyable`): every number, Vector3, Color3 and switch of Lighting, Atmosphere, Clouds, Weather, the lights (PointLight, SpotLight, RectLight), the lens and screen effects (DepthOfFieldEffect, MotionBlurEffect, VignetteEffect, ColorCorrectionEffect, BloomEffect, ExposureEffect, FogEffect, FilmGrainEffect and the rest) and ParticleEmitters, and the Workspace's `GlobalWind`. The list is read from the classes' own properties, so a new one of theirs can be keyed with no change.

- Examples: Lighting's `ClockTime`, `Exposure`, `EnvironmentIntensity`, `SkyTopColor`; an Atmosphere's `Density` and `Haze`; Clouds' `Cover`; a Weather's `Intensity`, `Wetness`, `SnowCover`, `SnowThickness`; a light's `Brightness`, `Range` and `Color`; `FocusDistance`, `InFocusRadius`, `NearIntensity`, `FarIntensity`; a ColorCorrectionEffect's `Brightness`, `Contrast`, `Saturation`, `TintColor`.
- A switch (`Enabled`, `ShadowsEnabled`) is keyed as 0 or 1, and a Weather's `Precipitation` as its item's place (0 None, 1 Rain, 2 Snow; `files::KEYED_CHOICES`). Both are held from each key to the next whatever the key's interpolation says (`PropertyName::is_stepped`).
- A name shared by classes takes what each has: `Direction` is a number on Clouds and a Vector3 on a SpotLight, and `Size` is also a number on a VignetteEffect (`PropertyName::widths`).
- A Weather eases toward what its properties say over its `TransitionTime`, as in the game: set it to 0 for keys to be followed exactly.
- Clips are unchanged: the Animation tab offers what it did, though a clip file that names a scene property now reads.

## Environment presets and the Environment track

An **environment preset** (`anim::environment::EnvironmentPreset`, `crates/anim/src/environment.rs`) is the look of the sky and the weather under one name: values of Lighting, of the first Atmosphere, Clouds, Weather and ColorCorrectionEffect in it, and the Workspace's `GlobalWind`. A sequence's **Environment track** keys presets by name, `Lighting:TransitionToPreset` eases to one from a script, and the editor applies one or saves the world's look as one. All three use the same `blend` and `apply_preset`.

### Presets

**Built in** (`BUILT_IN`, `built_in`; a name in any case): each sets the same 15 values.

| Preset | ClockTime | Exposure | Atmosphere Density, Haze | Clouds Cover, Density, Color, Speed | Precipitation, Intensity | Wetness | SnowCover | GlobalWind | Saturation |
|---|---|---|---|---|---|---|---|---|---|
| Clear | 13.4 | 1 | 0.395, 0 | 0.3, 0.7, white, 20 | None, 0 | 0 | 0 | 4, 0, 1.5 | 0 |
| Evening | 19.2 | 1 | 0.395, 0.5 | 0.3, 0.7, white, 20 | None, 0 | 0 | 0 | 3, 0, 1 | 0 |
| Overcast | 13.4 | 1 | 0.5, 0.6 | 0.8, 0.6, 0.72 0.74 0.78, 25 | None, 0 | 0 | 0 | 8, 0, 3 | −0.15 |
| Storm | 15 | 1 | 0.6, 1 | 0.85, 0.55, 0.5 0.52 0.58, 45 | Rain, 0.9 | 0.9 | 0 | 16, 0, 6 | −0.25 |
| SnowDay | 13 | 1 | 0.45, 0.4 | 0.8, 0.5, 0.8 0.82 0.88, 15 | Snow, 0.6 | 0 | 1 | 4, 0, 1.5 | −0.15 |
| SnowNight | 19.3 | 1 | 0.45, 0.4 | 0.8, 0.5, 0.8 0.82 0.88, 15 | Snow, 0.6 | 0 | 1 | 4, 0, 1.5 | −0.15 |

Each also sets `SnowThickness` 0.35. Evening and SnowNight are after sunset (the sun sets at 18 on the engine's equinox day): moonlit.

**A project's own**: `assets/environments/<name>.environment.json` (`ENVIRONMENTS_FOLDER`, format 1, `to_json` and `from_json`), found by name before the built-ins (`find_preset`), so a file named `Storm` replaces the built-in Storm. A file that can't be read is an error, not the built-in. They're files, not Instances under Lighting, because a sequence names them and is a file in `assets/` too (both are packed by Build Game, shared in a Collaborate session and the same in every level), and because a file can leave a value out, which an Instance's properties can't. A new optional file doesn't change the project's format ([ADR-0016](../../wiki/Architecture/Architecture-Decisions/ADR-0016-Text-Project-Format-With-Migrations.md)).

```json
{
  "format_version": 1,
  "Lighting": { "ClockTime": 18.25, "Exposure": 0.9 },
  "Clouds": { "Cover": 0.6, "Color": [1.0, 0.8, 0.7] },
  "Weather": { "Precipitation": "None", "Intensity": 0.0 },
  "Workspace": { "GlobalWind": [6.0, 0.0, 2.0] }
}
```

- The sections are `Lighting`, `Atmosphere`, `Clouds`, `Weather`, `ColorCorrectionEffect` and `Workspace` (`EnvironmentTarget`). A value is a number, `true` or `false`, three numbers (a Color3 from 0 to 1, or a Vector3) or a choice's item (`"Rain"`): any property of that kind the class has, and of the Workspace only `GlobalWind` (`property_table`; `check` says which name or kind is wrong). Text (`SkyImage`) can't be blended, so it isn't held.
- **A value left out isn't touched.** A preset of only the wind changes only the wind.
- `capture(world)` reads a world's look (`CAPTURED`: Lighting's `ClockTime`, `Exposure`, `EnvironmentIntensity`; the Atmosphere's `Density`, `Haze`, `Glare`, `Color`, `Decay`; Clouds' `Cover`, `Density`, `Color`, `Speed`, `Direction`; the Weather's `Precipitation`, `Intensity`, `Wetness`, `SnowCover`, `SnowThickness`; the ColorCorrectionEffect's `Saturation`, `Brightness`, `Contrast`, `TintColor`; `GlobalWind`), leaving out what the world has no Instance for. The Weather's `TransitionTime`, masks and switches are settings, not a look, and stay the world's.
- `apply_preset(world, preset, skip)` sets each value that differs. Values for something Lighting hasn't got (no Clouds) go nowhere, and the targets missed are returned. It never makes Instances.

### Blending

`blend(a, b, t, turn)` is pure: `a` at 0, `b` at 1, exactly.

- Numbers, colors and vectors go straight from one to the other.
- A switch or a choice changes half way (`t` 0.5).
- **`ClockTime`** turns round the 24 hour clock (`blend_clock`, `ClockTurn`): `Shortest` (the default) the short way, so 22 to 2 goes through midnight, and so does 6 to 20 (10 hours back, not 14 on); `Forward` as time passes; `Backward` against it.
- **What falls** (`blend_falling`, the Weather's `Precipitation` and `Intensity` together): rain turning to snow, or back, eases to an intensity of 0 by half way, changes kind there, and eases in again, so the two never fall together. Nothing falling counts as 0 of whatever the other has: from Clear to Storm it rains from the first moment at a growing intensity, where changing at half way would switch rain on at half its strength.
- A value only one of the two sets is kept as that one has it. A value neither sets isn't in the result.

### The track

`EnvironmentTrack::new(keys, find)` (`SequenceRun` holds one) and `sample(time)`: before the first key its preset, after the last key its preset, between two keys their blend, as far along as the earlier key's `interp` says (`Linear`, `Ease` with an `easing`, `Bezier`, or `Step`, which holds until the next key; a property key's own), the clock turning the earlier key's `clock` way.

- A value is keyed only where a preset sets it. A key whose preset leaves a value out is given, when the track is made, what the keys around it that do set it blend to at its time: a preset of only the wind between Clear and Storm leaves the sky blending straight from Clear to Storm.
- A key that names a preset that isn't there is left out and reported once ("a sequence's track binds the environment preset Monsoon, which isn't in the game"), as are Clouds, a Weather or an Atmosphere the presets have values for and Lighting hasn't got. No ColorCorrectionEffect isn't reported.
- `SequenceRun::new` knows the built-ins; `with_presets(&assets)` finds the project's too. Every host calls it (`script/src/sequence.rs`, the editor's preview, `movie/job.rs`).
- **A property track wins.** A property a track keys on the same Instance (`Lighting/Clouds` `Cover`) is left to that track: the Environment track doesn't set it at all, at any time, so the two never set it in turn. A track with no keys overrides nothing.
- **The Weather's `TransitionTime`** still applies, as it does to property keys: the track sets the Weather's properties, and the weather drawn follows them by at most one whole change per `TransitionTime` ([../rendering/weather.md](../rendering/weather.md)). A blend slower than that is followed exactly; a quicker one, or a scrub, is held back to that rate, and then rain that the properties have already stopped can still be seen falling as the snow starts, as when `Precipitation` is changed by hand. The two never pull against each other: the weather drawn only ever moves toward the properties. Set `TransitionTime` to 0 for it to be followed exactly.

### From a script

`Lighting:TransitionToPreset(name, seconds, clock)` (`script/src/environment.rs`, `EnvironmentTransition`): from how the world is now to the preset over `seconds` (0 if left out: there before the call returns), slow at both ends (Sine InOut). `clock` is `"Shortest"`, `"Forward"` or `"Backward"`. `Lighting:StopPresetTransition()` leaves the world where it has got to.

- One runs at a time, shared by both sides. Another call takes over from where the world is at that moment, so nothing jumps; what the first was changing and the second's preset leaves out stays where it had got to.
- `ScriptRuntime::step` moves it on after both sides' scripts and before the Sequences, so a playing sequence's keys win over it for that frame. When the sequence ends the transition carries on from where it would have been.
- An unknown preset, seconds below 0 or another `clock` is an error. Values for Clouds, a Weather or an Atmosphere that Lighting hasn't got are skipped with one warning each a session.
- It sets properties on the device that called it: call it from a Script for every player to see it. The Weather's `TransitionTime` applies as above.

Tests: `anim/src/environment/tests.rs` (the built-ins, each kind of value, the clock, rain to snow, values left out, files, the track, a property track winning, old sequence files unchanged), `script/src/environment_tests.rs` (easing, a second call taking over, at once, stop, errors, a project's preset, a Sequence winning), `editor/src/app/environment_tests.rs` and `movie/tests.rs` (`a_movie_renders_a_sequences_environment_track`).

## Playing

`SequenceRun::apply(world, time)` puts a world where the sequence has it at `time`, whatever came before: keyed properties (through `apply_evaluated`), the Environment track's blend (but for what property tracks key), each animation section's clip loaded on its Animator through an Animation outside the tree and its track held at the section's time (speed 0, the section's weight), and the overlay, a ScreenGui named `Cutscene` in the Workspace (`DisplayOrder` 1000) with `Fade`, `TopBar`, `BottomBar` and `Subtitle`. It returns the camera shot, the Sounds to start and stop, and the levels that should be added; `finish` lets go of the clips, the overlay and the Sounds (keyed properties stay). The editor's preview and the game call the same code, so they show the same at the same time.

The **Sequence** Instance (`script/src/sequence.rs`): `SequenceId` (the file), `AutoPlay` (plays when the game starts), `Skippable` (on) and `LockPlayerInput` (on).

- `Play()` starts it from the start, or carries on after `Pause()`. `Stop()` lets go of it and goes back to the start: its clips, Sounds, overlay and levels go and the camera comes back; keyed properties stay and `Completed` doesn't fire. `TimePosition` reads where it is and jumps when set (applied at the next step, events between not fired); `IsPlaying` and `Length` read.
- `EventReached(name)` fires as it passes each event, and `Completed` once at its end; then the camera comes back.
- Both sides share the playbacks. `ScriptRuntime::step` moves them on after both sides' scripts (frame step 2), so the Animators sample this frame's time. While a cut is in charge the client's `workspace.CurrentCamera` is held Scriptable at the shot, and given back as it was after.
- **Skipping**: with `Skippable`, a press of the Menu action (Escape, ButtonStart) while it plays jumps it to its end: applied at `duration`, so everything it keys is at its last values, then let go of and `Completed` fired once. The events skipped over aren't fired. The press goes no further.
- **Locked input**: while a Sequence with `LockPlayerInput` plays or is paused, `ScriptRuntime::input_locked` is true. Hosts give the character controllers no movement (the player's `controller_input`, the editor's `Session::frame`), and `input_events` passes only releases to scripts and prompts, so nothing stays held. Control comes back when it ends, is skipped or stops.

## The Sequencer tab

`crates/editor/src/sequencer.rs`, its actions in `app/sequencer.rs`. Pick a sequence or type a name for a new one (one undo step, `History::record_asset`); the Assets panel's Sequence tiles open there too, and **Insert** puts a Sequence that plays it in the Workspace.

- The Animation tab's transport bar (`animation/transport.rs`) sits above the timeline: start, a frame back, play or pause, a frame on, end and stop, a seek bar, the time, the frame counter and the preview's rate (0.25× to 4×; it changes only how fast the preview plays, not the file). With the mouse over the tab and nothing being typed, **Space** plays or pauses, **Left** and **Right** step a frame (**Shift** for ten), and **Home** and **End** go to the ends (`app/sequencer_transport_tests.rs`). Preview Sounds play at real speed whatever the rate.
- Under it, the Sequencer's own row (AP5, `transport::sequence_bar`, `PlayMode`; kept while the editor runs, never saved): **Play backwards** (**Shift+Space**; again pauses, and Space then plays forwards), **Loop playback** (**L**), **Previous** and **Next edit point** (**Up** and **Down**: the start and end, every section's start and end, camera cuts and events, `SequenceDoc::edit_points`), and **In** and **Out** (**I** and **O**), which mark a play range at the playhead (Out before In starts it at 0, In after Out ends it at the end), shown as `0:01.20–0:03.40` with a clear button (**Alt+X**) and shaded on the seek bar and the section rows. Play from outside the range (or at its end) starts at In, play stops at Out (or loops back to In), Stop goes back to In, and played backwards the same holds from Out to In; seeked past the range, play runs on to the sequence's end. Preview Sounds play only forwards. Opening another sequence clears the range. These keys are only the Sequencer's: over the Animation tab, Shift+Space still plays or pauses and the rest are left alone. Tests: `transport::tests` (`playing_backwards_…`, `a_play_range_…`, `edit_points_…`, `the_sequencers_keys_are_its_own`) and `sequencer/tests.rs` (`edit_points_are_section_ends_cuts_and_events`, `the_sequencer_plays_its_range_round_and_round`).
- At the end of that row, **Render Movie** opens the Render Movie window ([../editor/movie.md](../editor/movie.md)).
- The Animation tab's pieces: its timeline (playhead, events, frame ticks) and the property editor's dope sheet and Curves view (`PropertyDoc`) over the sequence's keys. **Add track** and **Key selected** (K) key the Explorer's selection as the preview shows it; **Key camera** keys `@Camera` where the Scene view's camera is. Select Lighting, something inside it (Clouds, a Weather, a lens effect), a light or the Workspace in the Explorer, and **Add track** lists its numbers, colors and switches (`picked_focus`; test `the_sequencer_keys_the_sky_and_what_is_under_lighting`).
- Under the timeline, a row per track kind (`Row`); **Add section** adds one at the playhead, bound to the Explorer's selection (or by its first tag, with **Bind by tag**). Drag a section to move it or its ends to resize it, and set the picked one below. A picked camera cut has a **Transition** picker (Cut, Dissolve, Fade through color), how long it **Lasts** and a fade's **Color**; the Camera cuts row draws a line under the cut over the seconds the transition takes.
- **The Environment row**: **Add section ▸ Environment key** lists every preset (the built-ins, then the project's) and adds a key of it at the playhead (replacing a key at that very time), easing Sine InOut. A key is drawn from its time to the next key's, named after its preset, with a tick where the preset is reached; drag it to move it (keys stay in time order) and pick it to set its **Preset**, its **Blend** to the next key (Linear, Ease in nine shapes, or Hold, then switch) and which way the **Clock** turns (The short way, Forward, Backward). Key times are edit points. The preview shows the blend as the playhead moves.
- **Preview** (on when a sequence opens) shows the sequence at the playhead in the Scene view without Play: a copy of the world, rebuilt when the edit history or the sequence changes, applied, posed by its own Animators and seen from the camera cut, with no tools; while it's on, a line above the keys says to turn Preview off to move things and then key them (K), and an empty dope sheet says the same (`PropertyDoc::sequence`; test `the_sequencer_says_to_turn_the_preview_off_to_move_things`). Its Sounds play while the preview plays, not while scrubbing. The tab closing, or Play, hides it.
- Each change is written to the file once the mouse is let go, as one undo step ("Edit Intro"); undo and redo show the file as it is.

**Environment presets in Properties** (`crates/editor/src/environment_presets.rs`, its actions in `app/environment.rs`): with nothing or Lighting selected, outside Play, the Properties panel has an **Environment presets** section above the weather's. A button for each preset puts the edited world there as one undo step ("Apply Storm", `History::record_world`); Lighting gets Clouds and a Weather in the same step if the preset has values for them and it has none (an Atmosphere and a ColorCorrectionEffect change more than a preset says, so they aren't added: the Output panel says when a preset's haze needs an Atmosphere). **Save current as** writes `capture` of the edited world to `assets/environments/<name>.environment.json` as one undo step ("Save preset Dusk", `record_asset_files`), replacing a preset of that name; the name keeps letters, digits, spaces, `-` and `_`. The list is read from the folder again at most once a second, and at once after a save or an undo of one, when the Sequencer's preview is made again too. The section's weather-only presets below it (`weather_inspector.rs`, `.axcenity/weather_profiles.json`) are the editor's own and aren't environment presets: a sequence or a script can't name them.

## Blocks

"play cutscene *sequence*" is `sequence:Play()` and "when cutscene *sequence* finishes" is `sequence.Completed:Connect(function() … end)` ([../scripting/blocks.md](../scripting/blocks.md)). `:Play()` switched from Code back to Blocks reads as "play sound", which is the same Lua.

## Not yet

A built game reads sequences from `assets/`, so Build Game packs them; Asset Check doesn't check `SequenceId` or bindings yet. Sounds start from their beginning wherever the playhead is. One Instance per tag binding. A dissolve in the game and the preview (it plays as a cut there). Keys on text and on enums other than `Precipitation`. An environment preset holds no text (a `SkyImage`), and there are no events for a preset transition starting or ending. Environment preset files aren't tiles in the Assets panel, and nothing renames or deletes one from the editor (delete the file). The Environment track has no curve view: a key's blend is its easing.
