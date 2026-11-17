# R1 hand-back: RA6, the benchmark plan (`r1-benchmarks`)

Plan 11.6 and 11.7 (RA6), brief §17. Branched from `character-creator-stage0` (5b650d5d). Red run: [r1-benchmarks-red-run.txt](r1-benchmarks-red-run.txt).

## What changed

| File | Change |
|---|---|
| `assets/characters/benchmarks/make.py` | `CAST` per base model, written as `<family>-<frame>-cast-<name>` on the masculine and feminine frames, bare; the realistic `face-neutral` files no longer written (`NEUTRAL_FACE_IN_CAST`); what a parameter's choices fill (the realistic underwear) follows the choice nearest the file's value, ties to the first as the resolver does, so every file lists what is drawn; outfits and single items that would only repeat the chosen underwear are left out |
| `assets/characters/benchmarks/*.character.json` | Regenerated: 377 files (was 337). New: 34 cast files (18 realistic, 16 stylized) and the 8 realistic `brow-length` ends the committed set lacked. Removed: `realistic-masculine-face-neutral`, `realistic-feminine-face-neutral`. Changed: the 213 other realistic files now list their underwear. Stylized files unchanged |
| `crates/avatar/tests/shipped_library.rs` | `every_benchmark_character_is_sound` extended (below) |
| `crates/player/src/character_capture.rs` | Additions only: `MORE_VIEWS` (`face-three-quarter`, `face-side`, `skin-close`), `MORE_LIGHTS` (`warm`, `cool`, `outdoor`, each a `Light` and a `Tint`), `HeldPose` and `held_pose` (`t-pose`, `arms-raised`, `elbows-bent`, `knees-bent`), `Stance::Held`, `Suite` (`face`, `body`), `capture_in`, `capture_suite`, `contact_grid`, `stance_named`, `lights_named`, `views_named`; the tool reads `AXCENITY_CAPTURE_LIGHTS`, `_VIEWS`, `_SUITE` and the new pose names. `VIEWS`, `LIGHTS`, `Settings::default`, the stage, `capture`'s signature and the default run's outputs are unchanged (`capture` now calls `capture_in` with every fixed light and view; `contact_sheet` shares its drawing code) |
| `crates/project/src/default_humanoid.rs` | The Walk state plays the `Walk` take (it played `Run`), at 3.86× so its feet keep up with `WalkSpeed` 16; a test |
| `docs/development/character-studio/visual-qa.md`, `testing.md` | The set's counts, the cast and its sources, the new lights, views, poses and suites with commands, the new tests |
| `docs/development/character-studio/refinement/baseline/README.md` | Section 9: the cast, the commands, the committed sheets and what they show |
| `docs/development/character-studio/refinement/baseline/engine/cast/` | 9 contact sheets and 3 `captures.json`, 13.2 MB |
| `docs/research/default-humanoid.md`, `docs/scripting/api-character.md` | The Walk state's take and speed |
| `wiki/Engine-Systems/Characters.md`, `crates/player/CLAUDE.md` | The capture's additions and the cast (the wiki also said "five fixed cameras"; there are six) |

## Interfaces

- **Environment variables** of `capture_characters`: `AXCENITY_CAPTURE_LIGHTS=studio,hard,warm,cool,outdoor` (any, in order; default `studio,hard`, or `studio` with a suite), `AXCENITY_CAPTURE_VIEWS=front,...,face-three-quarter,face-side,skin-close` (default the six `VIEWS`, or the suite's), `AXCENITY_CAPTURE_POSE=` also `t-pose`, `arms-raised`, `elbows-bent`, `knees-bent`, `AXCENITY_CAPTURE_SUITE=face|body`. An unknown name panics with the list of known ones. A suite writes `<name>-<suite>.png` (a row per state and light, a column per view) and `<name>/<suite>/<state>/<light>-<view>.png`; its `captures.json` adds `suite` and `states`. A run without these writes exactly what it wrote before.
- **Held poses** are played as a clip: `held_pose` turns one bone per side about its head in the model file's space (Y up, facing +Z) at the Blender harness's angles (`qa_scene.POSES`: abduction 90 and 150, `_elbow` 135 toward the front, `_knee` 135 toward the back), and the result is written into the stage as `assets/animations/capture/<pose>-<hash>.anim.json` (one constant key per turned bone) and played by an `Animation` with `AutoPlay`, the path `Clip@sec` uses. The skeleton is the body's own (the derived model's rig), so the pose fits each character.
- **Suites**: `face` = neutral, `Happy:1` (smile), `Sad:1` (frown), `Blink_Left:1,Blink_Right:1` (eyes closed), `Jaw_Open:1` (mouth open), from `face-front`, `face`, `face-three-quarter`, `face-side`. `body` = rest, the four held poses, `Walk@0.3`, from `front`, `three-quarter`, `side`, `back`.
- **make.py**: `CAST` (person → values, or `{"masculine": …, "feminine": …}`), `REALISTIC_ASYMMETRY`, `NEUTRAL_FACE_IN_CAST`.

## Tests

Written first and seen failing (the red run file), then green:

| Test | Holds |
|---|---|
| `avatar` `every_benchmark_character_is_sound` (extended) | Each file wears what its choices make it wear (failed red on `realistic-blend-body-default`: the files listed no underwear); every base has the cast on both frames, `asymmetry` exactly where it has sided parameters; a cast member other than `average` sets something; the face reference is `face-neutral` or `cast-average` |
| `player` `the_default_run_is_every_fixed_view_under_every_fixed_light` | The default lights and views are `LIGHTS` and `VIEWS` in order, untinted; no added name collides with a fixed one |
| `player` `light_view_and_pose_names_are_read_and_unknown_ones_refused` | Parsing, order, errors; `Walk@0.25` and `Cheer` as before |
| `player` `the_suites_hold_the_brief_s_facial_and_body_tests` | The two suites' rows and columns; every face control named exists |
| `player` `held_poses_turn_each_joint_to_its_angle_on_the_realistic_body` | On the shipped realistic skeleton, both sides within 1 degree of 90, 150, 135, 135; forearms forward, shins back; one joint a side |
| `player` `added_lights_views_and_poses_repeat_and_differ` (GPU) | Same input, same pictures, for the new lights, `skin-close` and every held pose; each new light differs from `studio`; each held pose from rest |
| `player` `the_face_suite_draws_each_face_on_one_sheet` (GPU) | Five rows of four views; each face differs from neutral; the sheet's size |
| `project` `the_walk_state_plays_the_walk_take_at_the_walk_speed` | The Walk clip links `Walk`; its speed times the take's travel (read from `Walk_RootMotion`) is 16 studs a second within 0.3 |

Runs (all in this worktree):

```text
cargo nextest run -p axcenity-avatar -p axcenity-project -p axcenity-player --no-fail-fast
  Summary [  48.437s] 468 tests run: 468 passed, 19 skipped
cargo nextest run -p axcenity-player --no-fail-fast -E 'test(/character_capture/)'
  Summary [  12.511s] 7 tests run: 7 passed, 191 skipped
cargo clippy -p axcenity-avatar -p axcenity-project -p axcenity-player --all-targets -- -D warnings   (clean)
cargo fmt --all   (clean)
python scripts/check-wiki.py
  111 pages, 0 errors, 58 warnings   (the warnings are "may be stale" notes on pages this change doesn't touch)
```

The determinism test `the_same_description_captures_identical_pictures_and_every_view_is_drawn` stays green. Beyond it, the default run's sheets of `realistic-masculine-body-default` and `realistic-feminine-body-heavy` were captured again and are **byte-identical** to `refinement/baseline/engine/rest/`.

Not run: the workspace-wide suite, the editor crate, Blender QA (nothing it measures changed).

## Dependencies added

None.

## Visual evidence and what I saw

Captured the 18 realistic cast members with the face suite, the body suite and the lights `studio,warm,cool,outdoor` (views `front,three-quarter,face-front,skin-close`); every one of the 54 sheets was opened. Nine are committed in `refinement/baseline/engine/cast/`; the full list of findings is section 9 of `refinement/baseline/README.md`. In short:

- **The face cast is too alike.** The face members differ from `average` by only 0.3 to 0.7 % of the `face-front` pixels (the body members by up to 1.8 %). Broad and narrow change the jaw and little else; the skull and eyes stay. Asymmetry shows (one brow and eye higher, the chin off to one side) but only beside the average; the distinctive face's bridge and nose barely show from the front. The face parameters at the references' preset strengths don't make different people. That is R4's problem, not something to hide by raising the cast's values.
- **Heavyset isn't heavy** (H6 still), and the abdominal lines are still on every body (H5).
- **The held poses work.** T-pose and arms raised turn cleanly at the shoulder at this size. Bent elbows put the fingers against the chest (I couldn't judge from these views whether they pass in). Bent knees leave the body hanging above the floor, as the Blender `knee_135` pose does. With the arms raised the hands leave the top of the fixed full-body frames.
- **Walk is a walk.** It is upright, the arms low and the trailing foot lifting, where the baseline's `Walk@0.4` was a running stride.
- **The new lights read as named:** warm (a low orange sun), cool (soft blue-grey overcast, which leaves the face dark and muddy), outdoor (blue sky, a midday sun, the face lit). The skin grain (H4) and the hard forehead shadow line (H2) are worst under `outdoor` and in `skin-close`.
- The mouth open (H8), the flat smile (M12), the briefs (H1), the straps (M9) and the jaw line (M4) are unchanged. Eyes closed is clean in all four views.

## Open issues

1. **Walk at `WalkSpeed` 16 plays the Walk take 3.86× as fast.** The take covers 1.45 m/s; 16 studs a second is 5.6 m/s, a jog. To keep the feet from sliding (the rule the other states follow), the step rate is about 7 a second, which will look hurried in a game. That's why HD3 had chosen the Run take (`docs/research/default-humanoid.md`). This change does what the plan asks ("Walk plays a walk"). The alternatives are a lower default `WalkSpeed` for the default character (about 4), or keeping Run for the state machine while the capture's `Walk@` clip plays the walk. **Decision for the lead or Paul.** Reverting is one line in `STATES` plus the test.
2. **The asymmetry values.** The task said "1 to 3 percent of each mirrored face pair's range". Taken literally, that is 0.02 to 0.06 on the −1 to 1 scale, 0.1 to 0.3 mm, below every perception threshold in the face references, so the character would look symmetric. I used the references' own "benchmark asymmetry face" column instead (§4: 0.25 to 0.70, 1.6 to 3.7 mm a feature). That is the 1 to 3 per cent of a paired *measurement* the literature reports (F-13), at the size people start to notice. If the literal reading was meant, change `REALISTIC_ASYMMETRY` in `make.py` and recapture.
3. **The blend wears the Male underwear** (M11). `make.py` doesn't choose it: the resolver's `nearest_choice` gives the 0.5 tie to the first choice, and the resolver overrides whatever a file says. The files now list the boxer briefs, so they say what is drawn. Fixing it needs either a third choice or a tie rule in `avatar` (the lead's contract).
4. **The face parameters are weak** (C1 in the baseline README): for R4 (RA2).
5. The fixed cameras miss the raised hands and the tallest bodies' heads (baseline section 5). A wider body view would be a further addition, not a change.
6. **Stylized `face-neutral` kept.** The stylized family keeps its `face-neutral` files, which are the same characters as its `body-default` and `cast-average`. Only the realistic ones were folded, as the task said. Folding them too is one entry in `NEUTRAL_FACE_IN_CAST`.
7. The stylized cast was generated and checked but not captured.

## What the lead must do at merge

- Run `python3 -I assets/characters/benchmarks/make.py` again after merging RA2/RA3/RA4 (manifests change), then `cargo nextest run -p axcenity-avatar every_benchmark_character_is_sound`. `make.py` now follows a base's `choices`, so new underwear choices need no change here.
- `every_benchmark_character_is_sound` now requires the cast for every base: a new base in the catalogue needs a `CAST` entry in `make.py` (an empty table fails the test, on purpose).
- Decide open issue 1 (the Walk speed). If Walk goes back to Run, `the_walk_state_plays_the_walk_take_at_the_walk_speed` goes with it, and the body suite's walk row becomes a run again.
- `character_capture.rs` is RA6's (additions only). If RA7 changes the renderer, recapture `engine/cast/` with the commands in the baseline README section 9 before judging a regression.
