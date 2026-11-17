# Hand-back: RA1, the studio's next pieces (`r1-studio-3`)

Branched from `refinement-wave2` (b39a9ed7). Plan: section 11.5 rows R4 (asymmetry tools), R9 (Expressions page, clip preview) and R10 (polish); section 11.7 (RA1's files). Brief §5.4 (facial asymmetry), §10 (expressions and animation preview), §12. Wave 2's open list: [r2/r1-studio-2.md](../r2/r1-studio-2.md) and the progress log's wave 2 entry.

## What changed

1. **Asymmetry tools (R4, brief §5.4)**, `character_workspace/sides.rs`.
   - Four tools for left and right pairs: **Mirror left to right**, **Mirror right to left**, **Reset left side** and **Reset right side**. Left and right are the character's own.
   - A pair is two parameters that name each other as `mirror`. The side comes from the ID (`_left`, `_right`, `left_`…), then from the label. A pair whose sides can't be told apart takes the parameter the manifest lists first as the left.
   - Each parameter is already written for its own side, so mirroring a side copies its value across. Reset puts a side back to the body's default character's value, as the toolbar's Reset does.
   - `Intent::Sides { tool, of }` makes one undo step. `of` is either:
     - `SidesOf::Feature { feature, level }`: every pair of a feature that the level shows. A pair the level hides is left alone, and the panel's "N more changes in Advanced" still counts it.
     - `SidesOf::Pair(id)`: one pair, named by either side.
   - Where the controls are:
     - A group with pairs starts with a **Left and right ⌄** menu for its feature.
     - Each paired row (a mirrored row, or each side's row when the sides differ) has a ⇄ menu for its own pair, beside the lock.
     - A tool that would change nothing is greyed out. Asked anyway, it makes no step and the status line says "nothing to change".
   - Symmetry's behaviour is unchanged. A slider still moves both sides while they agree, and a pair whose sides differ still shows both rows. Once a tool makes the sides agree, the pair is one row again.
2. **Pose and motion (R9, brief §10)**, `character_workspace/motion.rs`.
   - The choices: **Still** (the rest pose, the A-pose), **Idle** (the default: the state machine the preview always played), **Walk**, **Run**, and the default character's gestures **Wave**, **Point**, **Cheer**, **Clap** and **Shrug** (found among the shipped takes).
   - Where they are:
     - Chips under **Pose and motion** at the top of the Expressions page.
     - A compact menu in the bottom bar, after the views: the walking figure and the motion's name, in the accent colour while the body moves.
     - To make room for the menu, the bar's leading camera icon is gone.
   - `Intent::PreviewMotion(Motion)` is editor state, like an expression. It never enters the description, undo or a saved file (tested). It is kept across pages and new characters, and an expression shows on top of it.
   - How a take plays: as the capture tool plays a clip.
     - The Animator's state machine is cleared.
     - An Animation of the preview's own ("Studio Motion Preview"), looped at speed 1, plays a clip written once to `assets/animations/Humanoid/Preview/<take>.anim.json` in the workspace folder. The clip is linked to the take over its whole length.
   - **Walk is the Walk take at its own speed**: 1.067 s a cycle, measured 0.4 s of clip time per 0.4 s of preview time.
     - It plays in place. Over three cycles the head's mean position moves less than 0.1 studs.
     - The game's Walk state still plays the Run take (tested). The controller's default `WalkSpeed` is a jog, so the true walk is the studio's preview.
   - Run is the Run take at its own speed.
   - The held version side by side moves too.
   - **The camera follows the head while a take plays** (`follow_motion`).
     - It is smoothed at a rate of 6 per second and starts from where the head was when the motion began. Starting from there matters: a run leans the head forward at once.
     - A close view follows the head up and down as well; a far view only follows it across the floor.
     - Stopping a motion frames the view again after the next pose.
3. **Wave 2's open list.**
   - **Status line.** While a held version is shown it says "Showing A (held). Editing is off until you go back to Current.". Side by side it says "Side by side: A beside the current character…", and going back says "Showing the current character.". Discarding the shown version, or starting another character, removes the line.
   - **The Presets page's body tiles** are framed on the body: neck to knees, from the front and a little turned (`thumbnails::Subject::Body`, `body_orbit`). The crop is below the chin because a body preset doesn't change the face, and a first try that cut through the mouth read badly.
   - **Stylized presets as data**, in `assets/characters/humanoid/presets/*.preset.json`, written by a new `presets.py` beside the manifest using the stylized parameters:
     - Bodies: Slim, Athletic, Heavy and Broad, for each end of the stylized frame (8 files). Each names `frame` 0 or 1.
     - Faces: Round, Long, Angular and Soft.
     - Characters: Masculine (the default character as built), Feminine and Athletic.
     - New and the start page offer the three characters. The Body and Face pages show the body and face presets as Start from chips.
     - The stylized frame is a slider, not a choice. A body preset owns every body parameter, frame included, so `presets::suits` offers a body preset only to a character whose frame is within half the slider's span of the preset's frame. On a feminine character only the feminine builds are offered, and applying one keeps the frame (tested).
   - **The recovery file goes on Don't Save.** `EditorApp::discard_unsaved` (`app/documents.rs`) calls the new `CharacterWorkspace::discard_recovery`. That removes the file this workspace wrote and stops the same work being written again in the frames before the window closes. It runs before `discard_unsaved`'s early returns, because Character Studio has no project. A crash's file that is still being offered is left alone.
   - **architecture.md §15c** now lists the new intents and the preview's motion as editor state.

## Files

- New:
  - `crates/editor/src/character_workspace/sides.rs`, `motion.rs`
  - Tests: `sides_tests.rs`, `motion_tests.rs`, `polish_tests.rs`
  - `assets/characters/humanoid/presets.py`, `assets/characters/humanoid/presets/` (15 files)
  - this hand-back and its red run
- Changed (studio):
  - `character_workspace.rs`: modules, the `motion`, `preview_clip`, `follow` and `reframe` fields, and `advance`
  - `intent.rs`: `Sides`, `PreviewMotion`; `Sides` counts as an edit
  - `shell.rs`: applies the two intents
  - `compare.rs`: the status lines, and the side-by-side character posed for the motion
  - `autosave.rs`: `discard_recovery`
  - `bottom_bar.rs`: the motion menu
  - `panel.rs`: Pose and motion chips; the default character passed to the parameter pages
  - `panel/parameters.rs`: the Left and right menu, the pair menus, `side_choices`
  - `panel/row.rs`: `sides_menu`, `slider_with_sides`, `Asked::Sides`. `slider` is unchanged, so `panel/materials.rs` (the Skin page's materials section) isn't touched.
  - `presets.rs`: `suits` for a slider frame
  - `thumbnails.rs`: `Subject::Body`, `body_orbit`
- Changed (editor glue): `app/documents.rs` (three lines) and `app/character_workspace.rs` (a test).
- Changed (tests): `tests.rs` and `start/characters/tests.rs` now expect the stylized presets where they expected its default character. `shell_tests.rs`: `snap` and `paint_preset_pictures` are `pub(super)`. `walkthrough_tests.rs` is extended.
- Not touched: `stage.rs` (no change), the viewport's picking and handles, `render`, the `avatar` crate's contracts, `panel/materials.rs`, and every asset except the stylized presets and the catalogue.
- Docs: `docs/editor/character-studio.md`, `docs/development/character-studio/architecture.md` §15c, `crates/editor/CLAUDE.md` (one line), `wiki/Editor/Specialized-Editors.md`, `wiki/Roadmap/Known-Limitations.md`.

## Interfaces

- `sides`:
  - `SideTool::{MirrorLeftToRight, MirrorRightToLeft, ResetLeft, ResetRight}` (`ALL`, `label`, `about`)
  - `SidesOf::{Feature { feature, level }, Pair(id)}`, `Pair { left, right }`
  - `pair_of`, `pairs`, `apply`, `would_change`, `CharacterWorkspace::use_sides`
- `motion`:
  - `Motion::{Still, Idle, Walk, Run, Gesture(String)}` (`all`, `label`, `about`, `take`, `moves`)
  - `GESTURES`, `PREVIEW_CLIPS_FOLDER`, `PREVIEW_MOTION`, `preview_clip(take)`
  - `CharacterWorkspace::preview_motion`, the `motion` field, `advance(seconds)` (`step_by` plus the camera follow)
- `Intent::Sides { tool, of }` and `Intent::PreviewMotion(Motion)`. `intent.rs` is a lead-owned contract file; the task asked for these as Intents. Please review them as the contract.
- `CharacterWorkspace::discard_recovery()`; `thumbnails::Subject::Body`, `body_orbit()`.

## Tests

The red run is in [r1-studio-3-red-run.txt](r1-studio-3-red-run.txt).

- For a red run that fails at runtime rather than at compile time:
  - each entry point was switched off with an early return;
  - body tiles were drawn with the old full-body framing;
  - the status line, the stylized presets and the recovery removal were not written yet.
- Result: `21 tests run: 2 passed, 19 failed`.
  - The two that passed were `pairs_are_found_by_side_and_by_feature` (pure lookup, written with the code) and the old `every_new_character_choice_is_sound_on_both_families`. The latter was then updated for the stylized presets.
  - Old framing, neighbouring body tiles: Slim and Average 2.5%, Average and Athletic 2.1%, and 3.2 to 4.6% for the rest.

New tests:

- `sides_tests.rs` (9 tests):
  - pairs by side and feature, each level
  - each tool on a feature whose sides differ, one step each, undo
  - a pair's tools change only that pair
  - nothing to change makes no step and greys the menu
  - a hidden pair stays put, and the hidden-changes notice counts it (1, then 2, then none)
  - Symmetry unchanged: one row again after mirroring
  - the menus drawn on Eyes, and not on Forehead
- `motion_tests.rs` (6 tests):
  - Walk is the Walk take, at its own speed, looped, in place, and the game's Walk still plays Run
  - Still holds the rest pose; Run and every gesture move; one Animation only; Idle restores the state machine
  - never saved or undone; an edit while walking keeps it walking; side by side both walk
  - an expression on top of a walk
  - the camera keeps the head framed
  - the controls drawn on the Expressions page and in the bottom bar
- `polish_tests.rs` (4 tests):
  - the status line while a held version is shown, side by side, back to Current, after a discard and after New
  - stylized presets: sound (`check_preset`), in range, they apply and resolve, New offers them, the first is the default character, feminine builds only on a feminine frame, the frame kept
  - body tiles framed so neighbouring tiles differ by at least 3% of their pixels (GPU)
  - `wave3_pages_are_drawn` (snapshots, only with `AXCENITY_SNAPSHOTS`)
- `walkthrough_tests::the_whole_r1_walkthrough…`: one eye made bigger, the eyes mirrored left to right (one step, undo and redo), Walk and a smile previewed, and nothing of either in the saved file (motion Idle and no expression on reopening).
- `app::character_workspace::tests::dont_save_on_quit_removes_the_studio_recovery_file`.

Runs: see "Test runs" at the end.

## Measurements

- **Camera follow**, Face view, over 2 s once the motion has settled. Head position in the picture, as a share of half its height:

  | | Mean off the middle | Worst |
  |---|---|---|
  | At rest | 0.014 | – |
  | Run | 0.016 | 0.103 |
  | Run, no following | 0.154 | 0.279 |
  | Cheer | 0.082 | 0.253 |
  | Cheer, no following | – | 0.225 |

  The test asks for a mean under 0.1 and a worst under 0.35.
- **Body tiles**, realistic male builds in the library's order, share of pixels with a channel moved by more than 15:

  | Neighbours | Old full-body framing | Neck to knees |
  |---|---|---|
  | Slim / Average | 2.5% | 7.4% |
  | Average / Athletic | 2.1% | 6.1% |
  | Athletic / Muscular | 3.2% | 8.3% |
  | Muscular / Heavyset | 3.2% | 9.3% |
  | Heavyset / Lean | 4.4% | 12.9% |
  | Lean / Broad | 4.6% | 13.2% |

- **Walk**: a cycle of 1.067 s (the take's length) at speed 1. Over 0.4 s of preview the clip advanced 0.4 s (within 0.05). The head's mean moved under 0.1 studs over three cycles.

## Visual evidence

Snapshots: `AXCENITY_SNAPSHOTS=<folder outside the repo> cargo nextest run -p axcenity-editor -E 'test(wave3_pages_are_drawn)'` (1440 × 900, under Grey and Light). The committed JPGs in [r1-studio-3/](r1-studio-3/) have the status line's local path blanked. What I saw at full size and cropped at 2 to 2.5×:

- **Face › Eyes, sides uneven** (Grey and Light):
  - "Left and right ⌄" reads as a framed small button under the level row.
  - Every paired row has a quiet ⇄ beside its lock: Eye height and Eye spacing (one row each, Symmetry on), then Left and Right eye size and Left and Right eye tilt (both rows shown, since they differ), and so on. The changed rows have the accent bar.
  - The ⇄ is the same grey as the lock and easy to miss. I kept it quiet so the rows don't get busier.
- **Expressions, mid-walk with a smile**:
  - "Pose and motion" chips (Still, Idle, Walk lit, Run, Wave, Point, Cheer, Clap, Shrug) sit above the 15 expression chips, with a line saying Walk and Run play at their own pace, in place.
  - The face smiles while the shoulders are mid-swing, and the face stays centred in the Face view.
- **Body, mid-walk, Side view**:
  - A natural mid-stride: the front foot planted heel to toe, the rear foot on its toes, the arms swinging opposite the legs, the trunk upright.
  - The fingers curl a little (the take's hand pose).
  - The bottom bar reads "Back · 🚶 Walk · ☼ Studio…" with nothing overlapping. The first draft's full-size menu overlapped the lighting chips at this width; it is now the compact menu.
  - The stippled shadow edge at the feet is the renderer's, as before.
- **Body, Wave** (Light): the right arm raised, the hand open, the head and the other arm natural. "Wave" is in the accent colour in the bar, close to the sun icon but not touching it.
- **Presets page, body tiles**:
  - Neck to knees. Average and Heavyset show a belly; Heavyset is round; Muscular is broad in the chest and shoulders; Lean has a narrow waist with visible abdominal tone; Broad is wide.
  - Slim and Athletic are the closest pair: Athletic has a fuller chest and arms. They still read apart, where in wave 2 Slim, Average and Athletic looked the same.
- **Showing A (held)** (Light): the status line reads "Showing A (held). Editing is off until you go back to Current.", matching the viewport's card; "Held as A…" is gone.
- **Stylized Body page** (a feminine character):
  - Family: Stylized lit; Start from: Slim, Athletic, Heavy, Broad, Custom (only the feminine builds).
  - **Stylized Presets page**: Masculine, Feminine (lit) and Athletic as character tiles. The body tiles (feminine builds) show the stylized figure in its default outfit, so the builds differ less than bare realistic bodies do (the jacket hides the waist). The face tiles show Round, Long and Angular heads.

Honest notes:

- The stylized presets are starting values, checked for soundness and range only. I did not judge them against references or capture them.
- The stylized body tiles differ less than the realistic ones because they are clothed; their difference is not measured.
- Walk and Run play in place: the character doesn't travel, so a planted foot slides back along the floor as on a treadmill. This is recorded in Known-Limitations.

## Dependencies

None added.

## Generated files

- Regenerated: `assets/characters/catalogue.json` (`python3 -I assets/characters/catalogue.py`). It adds the 15 stylized presets; nothing else in it changed.
- The stylized presets come from `python3 -I assets/characters/humanoid/presets.py`.
- `assets/characters/benchmarks/make.py` was run and changed nothing.
- No model, manifest, realistic preset or `face_checks.json` changed.

## Open issues

- The bottom bar is full at 1440 wide. On a narrower window the motion menu and the lighting chips will crowd, as the view and lighting chips already do. A two-row bar, or folding lighting and backdrop into menus, is the fix.
- The Presets page's pictures are drawn once per GPU context. The snapshot harness makes one device per snapshot, so a second Presets snapshot in the same run shows blank tiles; only the Grey one is taken. The app has one device, so this doesn't affect it.
- The asymmetry tools work on a feature's pairs at the shown level. There is no "every pair of the face" scope; the Face category's Reset is the closest.
- Pose and motion has no scrubbing, speed control or pause, and no turntable. Gestures loop; they don't play once and return to Idle as they do in a game.
- The stylized body presets need a capture review against the stylized benchmarks.
- `intent.rs` and `architecture.md` are the lead's files; I changed them because the task required it.

## For the lead at merge

1. Review `Intent::Sides` and `Intent::PreviewMotion` as the contract (`intent.rs`), and architecture §15c's new sentence.
2. Regenerate the catalogue after merging any other preset or item change (`catalogue.py`). This branch adds the stylized presets to it.
3. Conflicts are likely in:
   - `panel.rs`, `panel/row.rs`, `panel/parameters.rs`, `bottom_bar.rs` and `character_workspace.rs`, if another RA1 branch (direct manipulation, makeup on the Skin page) touched them;
   - `docs/editor/character-studio.md`.
   `stage.rs` is untouched here.
4. Progress entry: R4's asymmetry tools with tests; R9's pose and motion preview (Walk at its own speed); wave 2's status line, body tiles, stylized presets and Don't Save recovery are done.

## Test runs

- `cargo nextest run -p axcenity-editor -p axcenity-avatar -p axcenity-project --no-fail-fast`: `1709 tests run: 1708 passed (10 slow), 1 failed, 10 skipped`.
  - The failure is `app::collab_repo_tests::closing_the_editor_uploads_what_was_saved`, the timing test that also failed under load in wave 2. This branch doesn't touch it. Run again on its own it passed: `1 test run: 1 passed`.
  - This run started before the last layout fixes (the bottom bar's spacing, a stale comparison notice). The studio set below was run after them.
- After the last changes, `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(/character_workspace|start::characters/)'`: `86 tests run: 86 passed`.
- With `AXCENITY_SNAPSHOTS` set, the two snapshot tests draw for real and take long under the machine's shared load.
  - Run together they hit nextest's 120 s timeout.
  - Alone: `wave3_pages_are_drawn` passed in 91.6 s. `every_page_is_drawn_with_its_controls`, which is unchanged except for the new controls it draws, took 124.6 s, just over nextest's limit. With `cargo test` it passed (`1 passed`).
  - Without snapshots, both take seconds.
- `cargo clippy -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all --check`: clean.
- `python3 -I scripts/check-wiki.py`: `113 pages, 0 errors, 58 warnings` (the same warnings as before).
