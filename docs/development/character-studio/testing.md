# Character Studio overhaul: testing

How every milestone is tested, what runs where, and the tests that exist. Visual review and the benchmark set are in [visual-qa.md](visual-qa.md). GitHub Actions runs only for release tags ([ADR-0018](../../../wiki/Architecture/Architecture-Decisions/ADR-0018-Actions-For-Release-Tags-Only.md)), so **local runs are the only check**; every hand-back lists the commands run and their results.

## Rules

- **Red, then green.** A milestone starts with tests that fail for the right reason (a stub, a missing file, a defect the milestone fixes); record the failing run in the PR, then make them pass. A test that passes before the code exists doesn't count as evidence for that milestone.
- **Name tests after the behaviour** (`a_format_1_file_migrates_to_the_same_character`), so a criterion can cite one.
- **Never loosen a threshold to pass.** `assets/characters/humanoid/qa/thresholds.json` and the realistic package's thresholds only go down; a raise needs renders showing the cause isn't visible, and the lead's agreement.
- **Known visual defects never become baseline.** A measured defect stays in the audit's list until fixed; a test pins the fix.
- **Damaged input never panics.** Every reader has a test feeding it broken files.

## Layers

| Layer | What | Where | Runs |
|---|---|---|---|
| Unit | Schemas, checks, migration, resolution, fitting maths | `crates/avatar/src/tests/` and each crate's tests | `cargo nextest run -p axcenity-avatar` |
| Contract | The repository's library against its models: every shape, bone and material a manifest names exists; the catalogue is current; defaults and migrated files are sound | `crates/avatar/tests/shipped_library.rs` | Same; after any change to a manifest, a model or `catalogue.json` |
| Engine agreement | The manifests describe what the engine ships (sockets, body sliders, parts, default outfit); format 1 files resolve to what the workspace shows | `project` `default_humanoid::tests::the_character_data_manifests_describe_the_default_humanoid`; `editor` `character_workspace::tests::a_saved_look_migrates_to_the_same_parts_and_shapes` | `cargo nextest run -p axcenity-project -p axcenity-editor` |
| Runtime | A description applied in the player: parts, identity baked into derived models, followers taking live shapes, bone offsets, attributes, materials, kept descriptions applied again; game scripts calling the `HumanoidDescription` API | `player` `default_humanoid_tests/description.rs` and `description_lua.rs`; `gameplay` `description_tests.rs`; `assets` `model::identity`, `derived`; `avatar` `tests::resolving` | `cargo nextest run -p axcenity-player description`, `-p axcenity-gameplay`, `-p axcenity-assets identity derived` |
| Geometry QA | Poke-through, waist gaps, layer order, joint volume, eyelid closure, teeth and lips, self-intersection at slider extremes | `assets/characters/<family>/qa/measure.py` (Blender, `bpy`) | Locally where `bpy` is installed (below) |
| Visual | Benchmark sheets, in Blender and in the engine | `render_baseline.py`; the in-engine capture (`player`'s `character_capture.rs`, the `capture_characters` tool) | [visual-qa.md](visual-qa.md) |
| UI | Walkthroughs: open, edit, undo, redo, save, reopen; every control changes the character; drawing snapshots | `editor` `character_workspace/tests.rs`, `app/character_workspace.rs` | `AXCENITY_SNAPSHOTS=<folder> cargo nextest run -p axcenity-editor character` |
| Performance | GPU timings per pass, the character budget, a crowd | `FramePass` timers; `docs/benchmarks.md` | On the reference PC, pinned to one CCD |

## Commands

```text
cargo fmt --all
cargo nextest run -p axcenity-avatar
cargo nextest run -p axcenity-project -p axcenity-editor -p axcenity-player
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace --no-fail-fast
cargo test --workspace --doc
python scripts/check-wiki.py
```

After a manifest or model change: `python -I assets/characters/humanoid/manifests.py` (or the family's generator), then `python -I assets/characters/catalogue.py`, then the contract tests.

**Geometry QA** needs Python 3.13 with `bpy` and Pillow (see `assets/characters/humanoid/qa/README.md`):

```text
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --only clipping --out <file> --check assets/characters/humanoid/qa/thresholds.json
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --out <file>      # everything, ~25 s
```

## What each milestone must test

| Milestone | Tests written first |
|---|---|
| CO1 (done) | Schemas round-trip; newer files refused; damaged files are errors; format 1 migrates (values, defaults, ranges, unknown parts); every manifest entry names shapes, bones and materials its model has; checks find each kind of mistake; parameters resolve (spans, corners, constraints, regions) |
| CO2 | The same description, cameras and settings give identical images twice; the capture writes every view |
| CO3 | A character file applied in the player wears its parts, its parts take the body's shape weights, frame bone offsets move the eyes, attributes reach `BodyShape`, material values modulate (base colour map kept); the workspace and the player agree on the same file |
| CO4 | The realistic package's manifest against its model; geometry QA: no eye or teeth poke-through at rest; joint volume within the limits agreed below |
| CO5 | Every parameter moves geometry (a shape's largest move above 1 mm at its range's ends); extremes of every pair free of self-intersection |
| CO6 | `anim::check_face` clean for the new head and its profile; every expression and viseme without eyes through lids or teeth through lips; a full blink covers the eyeball |
| CO7 | Each shading model's output against a reference image at fixed lighting; neutral values match Standard; GPU timings recorded; `every_example_renders` unchanged |
| CO8 | 20 characters × 150 shapes at least as fast as today's 41; ray traced shadows show expressions |
| CO9 | Binding round-trips; every item fits both frames and composition extremes under the poke-through thresholds; layers stay ordered; an imported third-party garment fits after import |
| CO10, CO11 | Per item and style: the contract tests, geometry QA, no clipping into the head at rest |
| CO12 | Walkthroughs; every control in the manifest changes the character; the studio writes format 2 and reads format 1 |
| CO13 | Changing one subsystem breaks none of the others (the benchmark set before and after); a replicated description resolves the same on both ends |

## Thresholds agreed for Stage 3

Starting points from the Stage 1 baseline (`baseline/measurements.json`); a milestone may only tighten them.

| Measure | Stylized today | Realistic target (CO4–CO6) |
|---|---|---|
| Waist gap, every frame × build | 0 mm | 0 mm |
| Garment poke-through (points, all combinations) | `thresholds.json` | 0 visible; counts recorded per item at CO9 |
| Elbow 135° ring area ratio | 0.37 | ≥ 0.6 with twist bones and correctives |
| Shoulder abducted 150° volume ratio | 0.56 | ≥ 0.8 |
| Knee 135° ring area ratio | 0.44 | ≥ 0.6 |
| Eyeball front uncovered in a full blink | 20–23% | 0% |
| Teeth vertices inside lip or chin flesh, jaw open | 62–192 | 0 |
| Face vertices above the head joint carrying `Neck` weight | 911 | 0 |

## Tests that exist (CO2)

`player` `character_capture::the_same_description_captures_identical_pictures_and_every_view_is_drawn`; `avatar` `every_benchmark_character_is_sound`.

## Tests that exist (CO3)

`assets` 13 (`model::identity::tests`, `derived::tests`); `avatar` 5 more in `tests::resolving` (`resolve`); `player` `default_humanoid_tests::description` (7: parts worn and swapped, identity baked and live shapes followed, the frame's bones re-seating the body and carrying the eyes, body sliders, material values modulating, a still character wearing its parts, kept descriptions applied again) and `description_lua` (3: the script API); `gameplay` `description_tests` (the API's pure functions); `project` `character_library::tests` (2); `editor` `a_character_file_applied_in_a_game_matches_the_workspaces_preview`. Three of the player's tests were checked against the behaviour switched off (no settle, no follower shapes, no modulation) and failed.

## Tests that exist (CO1)

`crates/avatar`: 35 tests (`src/tests/formats.rs`, `migration.rs`, `validation.rs`, `models.rs`, `resolving.rs`; `src/ids.rs`, `src/material.rs`; `tests/shipped_library.rs`). `project`: `the_character_data_manifests_describe_the_default_humanoid`. `editor`: `a_saved_look_migrates_to_the_same_parts_and_shapes`. The red run before the code is recorded in [progress.md](progress.md).
