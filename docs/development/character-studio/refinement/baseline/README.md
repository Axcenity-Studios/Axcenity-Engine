# Refinement baseline: `main` before the work starts

The state of the realistic character family on `main` (commit `99f5b181`, 2026-11-16), measured and captured before the Character Studio refinement ([brief](../brief.md) Stage 0, "Establish visual QA baselines"; brief §17). Every later stage compares against this folder. The harness is described in [visual-qa.md](../../visual-qa.md) and [testing.md](../../testing.md); nothing in it was changed.

Run on the reference PC: Windows 11, AMD Radeon RX 9070 XT (Dx12), rustc 1.97.0, cargo-nextest 0.9.148, Python 3.13.14 with bpy 5.3.0.

## 1. Tests

| # | Command | Result |
|---|---|---|
| 1 | `cargo nextest run -p axcenity-avatar -p axcenity-anim --no-fail-fast` | `Starting 226 tests across 6 binaries` / `Summary [   4.438s] 226 tests run: 226 passed, 0 skipped` |
| 2 | `cargo nextest run -p axcenity-editor --no-fail-fast character` | **Did not build.** `error: failed to remove file target\debug\axcenity-editor.exe` / `Access is denied. (os error 5)`. An editor was running from `target/debug`. Nothing was stopped. |
| 2b | `cargo nextest run -p axcenity-editor --bins --no-fail-fast character` (the same filter on the unit tests only, so the open program is not relinked) | `Starting 37 tests across 1 binary (1281 tests skipped)` / `Summary [  10.909s] 37 tests run: 37 passed, 1281 skipped` |
| 3 | `cargo nextest run -p axcenity-gameplay -p axcenity-project --no-fail-fast description` | `Starting 8 tests across 2 binaries (275 tests skipped)` / `Summary [   0.032s] 8 tests run: 8 passed, 275 skipped` |
| 4 | `cargo nextest run -p axcenity-player --no-fail-fast default_humanoid` | `Starting 17 tests across 1 binary (175 tests skipped)` / `Summary [   4.194s] 17 tests run: 17 passed, 175 skipped` |

Notes:

- Row 2b is a substitute, not the command asked for. The editor crate's only integration test file (`crates/editor/tests/mcp_stdio.rs`) has no test with "character" in its name, so the filter selects the same 37 tests either way. Run row 2 again with the editor closed to confirm.
- The 37 are: `character_workspace::tests` (12), `character_workspace::shelf_tests` (8), `start::characters::tests` (6), `character_studio::tests` (7), `app::character_studio::tests` (2), `app::character_workspace::tests` (1), and one unrelated test the filter also matches (`scripts::tests::line_starts_count_characters_not_bytes`).
- No test failed in any run. The full workspace suite was not run.

## 2. In-engine capture

Tool: `capture_characters` (`crates/player/src/character_capture.rs`). Settings as recorded in each `captures.json`: 1280 × 720, High quality, TAA, 48 frames, lights `studio` and `hard`, views `front`, `three-quarter`, `side`, `back`, `face`, `face-front`. A contact sheet is 3840 × 720: one row per light, one column per view, at half size.

The library holds 214 `realistic-*` benchmark files (180 of them face identity ends), so groups were captured, not the whole set. `<scratch>` is a folder under the system temp directory; `<B>` is `assets/characters/benchmarks`.

| Run | Command (all with `AXCENITY_CAPTURE_SHEETS=only cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture`) | Characters |
|---|---|---|
| `rest` | `AXCENITY_CAPTURE_DESCRIPTIONS=<the 32 realistic body and outfit files and the 2 neutral face files, separated by ;> AXCENITY_CAPTURE_DIR=<scratch>/capture/rest` | 34, 12 pictures each, 3.2 to 7.8 s each, 139 s in all |
| `smile` | `AXCENITY_CAPTURE_DESCRIPTIONS="<B>/realistic-masculine-face-neutral.character.json;<B>/realistic-feminine-face-neutral.character.json" AXCENITY_CAPTURE_FACE=Happy:1 AXCENITY_CAPTURE_DIR=<scratch>/capture/smile` | 2 |
| `blink` | the same two files, `AXCENITY_CAPTURE_FACE=Blink_Left:1,Blink_Right:1` | 2 |
| `jaw-open` | the same two files, `AXCENITY_CAPTURE_FACE=Jaw_Open:1` | 2 |
| `walk` | `AXCENITY_CAPTURE_DESCRIPTIONS="<B>/realistic-masculine-body-default.character.json;<B>/realistic-feminine-body-default.character.json" AXCENITY_CAPTURE_POSE=Walk@0.4` | 2 |

Every run ended `test result: ok. 1 passed; 0 failed`. The first release build took 1 min 42 s.

Captured as well and left in scratch (not committed): `Excited:1` (the smile that shows teeth), `Sad:1` (a frown), `Eye_Closed_Left:1,Eye_Closed_Right:1`, and `AXCENITY_CAPTURE_POSE=idle` for the two default bodies. Run the commands above with those values to get them again.

Things to know about the runs:

- **The `Walk` clip is a run.** The default character's `Walk` state plays the `Run` take (`crates/project/src/default_humanoid.rs:143`), so `Walk@0.4` is a running stride.
- **The neutral face files are the default bodies.** `realistic-<frame>-face-neutral` and `realistic-<frame>-body-default` give identical pictures (0 pixels differ in `studio-front`).
- **The blend files wear the Male underwear.** `captures.json` lists `Underwear_BoxerBriefs` on 22 of the 34 characters (the masculine and blend files) and the bra and briefs on 12.
- **Not captured:** the 180 face identity ends, hair and clothing (the family has none), the stylized family (its baseline is [../../baseline/engine/](../../baseline/engine/)).

## 3. The committed set

[engine/](engine/): 25 contact sheets and 5 `captures.json` files, copied as they were written. **16.9 MB** (16,931,030 bytes), about 0.65 MB a sheet.

| Folder | Sheets |
|---|---|
| [engine/rest/](engine/rest/) | `realistic-masculine-body-` and `realistic-feminine-body-` × `default`, `slim`, `heavy`, `muscular`, `heavy-muscular`, `shortest`, `tallest` (14); `realistic-blend-body-default`; `realistic-masculine-face-neutral`, `realistic-feminine-face-neutral` |
| [engine/smile/](engine/smile/) | the two neutral faces with `Happy` at 1 |
| [engine/blink/](engine/blink/) | the two neutral faces with both `Blink` controls at 1 |
| [engine/jaw-open/](engine/jaw-open/) | the two neutral faces with `Jaw_Open` at 1 |
| [engine/walk/](engine/walk/) | the two default bodies 0.4 s into `Walk` |

`engine/rest/captures.json` lists all 34 characters of that run; 17 of their sheets are here. The other 17 (`heavy-soft`, `lean-muscular`, `slight`, the blend's builds, the two `outfit-underwear` files) were captured and not reviewed.

What the builds set (from `<B>/make.py`): slim `fat -1`; heavy `fat 1`; muscular `muscle 1`; heavy-muscular both at 1; shortest `height 0.85, leg_length 0.85`; tallest both at 1.15.

## 4. Defects seen

Seen by opening the sheets, and the same pictures at full size (1280 × 720, captured into scratch with the same settings and without `AXCENITY_CAPTURE_SHEETS=only`). "Known" means [progress.md](../../progress.md) or the CO6 notes record it. Sheet paths are under `engine/`. This is a list of what is visible, not of causes: where a cause is given it is read from `captures.json` or the source and marked.

### High

| # | Defect | Where | Known? |
|---|---|---|---|
| H1 | **The briefs have no sides.** From the front they are a triangle that ends in points at the hips; the back panel is a separate piece. In the stride the back panel stands off the hip as a grey wedge with a free corner, and the hip is bare. | `rest/realistic-feminine-body-*.png` (front, three-quarter), `walk/realistic-feminine-body-default.png` (three-quarter, side) | Not this. The full-body views draw the underwear at level 1 (`captures.json`), and the package has no simpler copies of the underwear, so the engine makes them. Progress records the general risk (a simplified garment loses its fit, CO18) and says lost strap pieces were fixed in the build. |
| H2 | **Shadow edges on skin are ragged and speckled.** Blotches of lit and unlit skin with torn edges on the chest, the flank and along the spine; a stippled line where light ends across the forehead, the nose, the chin; dots in the nostrils. | Every sheet, both lights. Clearest: `rest/realistic-masculine-body-default.png` (front, back), every `hard` face view | In part. Progress records "a hard-edged dark band across the upper chest" (cause not confirmed) and "a small dark spot on one shoulder under the hard light" (also visible here), both left for CO7. The speckled edge on faces is not recorded. |
| H3 | **A nearby limb casts a hatched shadow.** Where a hand or arm is close in front of the body its shadow is drawn as diagonal lines. | `walk/realistic-masculine-body-default.png` and `walk/realistic-feminine-body-default.png` (front, both lights: the arm behind the torso). Also the hand over the thigh in the idle (scratch) | No |
| H4 | **Skin is coarse, glittering grain with an even gloss.** Close up it reads as wet sand. The dark side falls to flat brown; nothing softens the edge of a shadow. | Every face view; the chest in the face views of `rest/realistic-masculine-body-tallest.png` | Even gloss: known (CO7). The grain: no. |
| H5 | **Every body has the athletic body's surface detail.** Abdominal lines on the feminine default, on the heavy bodies and on the blend. | `rest/realistic-feminine-body-default.png`, `-heavy.png`, hard three-quarter | Known (one normal map for all, CO7) |
| H6 | **"Heavy" is not heavy.** Body fat at 1 thickens the waist and limbs a little. There is no belly in the side view. Against the default, 7,564 of 921,600 pixels differ in `studio-front` (masculine) and 6,052 (feminine); muscular differs by 5,015. | `rest/realistic-*-body-heavy.png` and `-muscular.png` beside `-default.png` | Not as a range. Progress records that the face barely changes with fat. |
| H7 | **No hair and no clothes.** Every character is bald in underwear. | Every sheet | Known (CO9 to CO11) |
| H8 | **The mouth open is not a mouth.** The corners are pinched into jagged points, a pale line runs round the inside of the lips, the opening is a bell, and a hard dark patch sits on the cheek beside it under the hard light. | `jaw-open/*.png` (face, face-front) | In part: "corners are points", "bell-shaped" (CO6 notes). The pale rim and the cheek patch: no. |

### Medium

| # | Defect | Where | Known? |
|---|---|---|---|
| M1 | Brows are dark opaque blocks, cut square at the inner end, the same on both frames. Lashes are a faint fuzz. | Every face view | In part ("lashes and brows are cards") |
| M2 | Eyes are flat: an even blue-grey white, no highlight, no pink inner corner. Both faces stare. | Every face view | In part (eye shading is CO7's; "the feminine face stares") |
| M3 | Lips are an even glossy mauve with a hard border, on the masculine face too. No lip texture. | Every face view | No |
| M4 | A thin pale line follows the jaw where it meets the neck, on both frames in both lights. | Every face view | No |
| M5 | Teeth are even white blocks with dark gaps at the sides and speckled shading; the lower teeth are a thin line; the tongue is a flat slab. Worse in the `Excited` smile (scratch), where the corners of the mouth are dark triangles. | `jaw-open/*.png` | In part (CO6 notes) |
| M6 | The two frames have nearly the same face: same brows, eyes, lips, skin. Only the jaw is narrower. | `rest/realistic-masculine-face-neutral.png` beside `rest/realistic-feminine-face-neutral.png` | No |
| M7 | The shortest body has short thick legs under a full torso and head; the tallest stands on stilts. Height and leg length both scale bones, and the benchmark sets both. | `rest/realistic-*-body-shortest.png`, `-tallest.png` | No |
| M8 | Heavy and muscular is a bodybuilder on both frames. | `rest/realistic-*-body-heavy-muscular.png` | Known |
| M9 | The bra's straps are wide flat bands with square, faceted tops. They run in a straight line from the bust to the back of the neck and stand off the hollow above the collarbone. The cups are faceted. | `rest/realistic-feminine-body-*.png` (side, face views) | The garment is recorded as interim until CO9, drawn across hollows on purpose. The look is not recorded. |
| M10 | A dark lump hangs below the briefs' back panel between the legs. | `rest/realistic-feminine-body-*.png` (back) | No |
| M11 | The blend body has a partly feminine chest and wears only the boxer briefs. | `rest/realistic-blend-body-default.png` (side) | No. Character Studio now offers Male or Female only, so this body is reached from a file or a script. |
| M12 | No creases in any expression: the smile, the blink and the open jaw move smooth skin. The smile is a wide thin line. | `smile/*.png`, `blink/*.png`, `jaw-open/*.png` | Known (CO6 "still short") |

### Low

| # | Defect | Where | Known? |
|---|---|---|---|
| L1 | Without hair the head reads tall and egg-shaped. The masculine neck is nearly as wide as the head. | Face views | No |
| L2 | The shoulder's outline is straight segments in close views. | Face views, lower corners | No |
| L3 | Fingers are long smooth tubes without knuckle creases. In the idle the palms turn forward with the fingers spread (scratch). | Side and three-quarter views | No |
| L4 | Knees are smooth, with no kneecap. Side views show the head carried forward. | Side views | No |
| L5 | The boxer briefs are flat black. The waistband is glossy and kinks at the hip in the stride. | `walk/realistic-masculine-body-default.png` | No |
| L6 | The floor shadow has a saw-tooth edge beside the feet, and the backdrop shows bands. These belong to the stage and renderer, not the character. | Every full-body view | No |

Nothing in the blink looked wrong at this size: the lids meet and no eyeball shows (`blink/*.png`).

## 5. What the harness cannot show yet

| Limit | Evidence |
|---|---|
| The cameras are fixed for a default-height body. The tallest body's head leaves the frame in the full-body views and its face views show the chest. The shortest body's face views show an empty backdrop. | `rest/realistic-*-body-tallest.png`, `-shortest.png` |
| The face views are not full detail. `Brows`, `EyeShell`, `Eyes` and `Teeth` are drawn at level 1 there and at level 2 in the full-body views. Only `Body` is at level 0 everywhere. [visual-qa.md](../../visual-qa.md) says the face view is full detail. | `engine/rest/captures.json`, `levels_of_detail` |
| `captures.json` lists no `Lashes` and no `LipSeal` part. | The same file |
| There is no pose for arms raised, bent elbows or bent knees in the engine capture (brief §17.2); only `rest`, `idle` and a clip at a time. The Blender harness has those poses. | `character_capture.rs:585` |
| One light pair. Brief §17.2 asks for warm, cool and outdoor light as well. | `character_capture.rs` `LIGHTS` |
| Side, three-quarter and back views of the face do not exist; `face` and `face-front` are both near the front. | `character_capture.rs` `VIEWS` |

## 6. Blender check

The `bpy` environment from Stage 3 still exists (Python 3.13.14, bpy 5.3.0; Stage 3 used Blender 5.2). Run from the repository's root, without the `sliders` section (it only runs when named in `--only`):

```text
<bpy python> -I assets/characters/humanoid/qa/measure.py --package realistic --out <scratch>/blender-measurements.json --check assets/characters/realistic/qa/thresholds.json
```

12.2 s. Output:

```text
WORSE: sliders/frames/masculine/bodies_with_new_crossings: not measured (run every section)
WORSE: sliders/frames/masculine/alone_with_new_crossings: not measured (run every section)
WORSE: sliders/frames/masculine/alone_eye_or_teeth_in_flesh: not measured (run every section)
WORSE: sliders/frames/feminine/bodies_with_new_crossings: not measured (run every section)
WORSE: sliders/frames/feminine/alone_with_new_crossings: not measured (run every section)
WORSE: sliders/frames/feminine/alone_eye_or_teeth_in_flesh: not measured (run every section)
TARGET NOT MET: deformation/masculine/shoulder_abd_90/local_volume_ratio: 0.847, at least 0.853
TARGET NOT MET: deformation/feminine/shoulder_abd_90/local_volume_ratio: 0.88, at least 0.992
check: failed (2 targets not met)
```

Exit code 1. This is the same output as Stage 3's record ([../../stage3/measure-check.txt](../../stage3/measure-check.txt)): the six `WORSE` lines are the sliders limits, which this mode does not measure, and they are what makes the exit code 1. **Every limit that was measured holds** (checked key by key against `thresholds.json`: 16 `at_least`, 18 `at_most`, 21 `within`). The two 90° shoulder targets are not met, as before.

| Measure (masculine / feminine) | Now | Limit |
|---|---|---|
| Elbow 135°, ring area | 0.757 / 0.758 | ≥ 0.6 |
| Knee 135°, ring area | 0.74 / 0.75 | ≥ 0.6 |
| Shoulder 150°, ring area | 0.929 / 0.82 | ≥ 0.92 / 0.81 |
| Shoulder 90°, local volume | 0.847 / 0.88 | ≥ 0.78; target 0.853 / 0.992, not met |
| Shoulder 10° (arm hanging), ring area | 0.907 / 0.962 | ≥ 0.9 / 0.96 |
| Shoulder 10°, local volume | 1.024 / 0.986 | ≤ 1.03 / 0.99 |
| Forearm twist 90°, smallest ring | 0.858 / 0.854 | ≥ 0.84 |
| Upper arm twist 70°, smallest ring | 0.934 / 0.931 | ≥ 0.9 |
| Thigh twist 45°, smallest ring | 1.001 / 0.978 | ≥ 0.95 |
| Skin in the eyeball, eye in the lid, teeth in flesh, at rest | 0 / 0 | 0 |
| Heads tall | 7.7 / 7.84 | 7.3 to 8.1 |
| Shoulder width ÷ height | 0.256 / 0.238 | 0.227 to 0.291 |

Against the last committed set ([../../lowered-shoulder/measurements.json](../../lowered-shoulder/measurements.json)) only 8 of 1,124 shared numbers differ, all from the underwear and brow-length build: `Body` and its two copies have 155 shapes (were 148), `Brows` 47 (was 43), the file has 68,262 triangles (was 63,004), and `Jaw_Left` and `Jaw_Right` move 74 vertices (were 75). No deformation, twist, face or proportion number moved.

Package contents by the same run: `Body` 22,184 triangles, `Body_LOD1` 10,500, `Body_LOD2` 5,000; `Teeth` 9,024 with two copies; `Eyes` 2,256; `EyeShell` 2,612; `Brows` 352; `Lashes` 188; `LipSeal` 48; `Underwear_BoxerBriefs` 1,568, `Underwear_Bra` 2,476, `Underwear_Briefs` 1,214, none with simpler copies.

Not run: `--only sliders` (15 to 40 minutes), `face_checks.py`, `render_baseline.py`, `posed_crossings.py`.

## 7. The workspace's drawing snapshots

No snapshot image is committed anywhere in the repository. The tests write them only when `AXCENITY_SNAPSHOTS=<folder>` is set:

```text
AXCENITY_SNAPSHOTS=<scratch>/snapshots cargo nextest run -p axcenity-editor --bins --no-fail-fast character
```

`Summary [  16.100s] 37 tests run: 37 passed, 1281 skipped`. Twelve pictures were written to scratch:

| File | Size | Written by | Shows |
|---|---|---|---|
| `character-studio-body.png` | 1440 × 900 | `crates/editor/src/character_workspace/tests.rs:402` `the_workspace_draws_every_category_on_both_bodies_without_a_gpu` (the snapshot part at lines 420 to 439) | The whole workspace on the Body page, realistic body, Advanced off. Left: Presets, Body, Face, Hair, Clothing, Stage. Middle: the Male default idling at three-quarter, with view chips Full body, Face, Front, Side, Back. Right: Symmetry and Advanced, Body type (Male, Female chips), Muscle, Body fat, Height. |
| `character-studio-face.png` | 1440 × 900 | The same test | The Face page: the head close up; sliders in groups Eyes (height, spacing, size, tilt), Brows (height, arch, length, ridge), Nose (nostril width, length, bridge, tip…), scrolling on. |
| `character-studio-realistic.png`, `-realistic-feminine.png`, `-stylized.png` | 640 × 720 | `tests.rs:513` `the_preview_draws_the_realistic_character` | The preview alone at the Full body view: the Male and Female presets, and a stylized character in its default outfit. |
| `character-picture-ada.png` | 360 × 450 | `crates/editor/src/character_workspace/shelf_tests.rs:299` `pictures_are_drawn_cached_and_drawn_again_when_the_file_changes` | A saved character's shelf picture. |
| `start-characters.png`, `-narrow.png`, `-pack-editor.png` | 1400 × 2200, 640 × 3400, 1400 × 900 | `crates/editor/src/start/characters/tests.rs:263` and the pack editor test | The start screen's Characters page, wide and narrow, and the pack editor. |
| `character-studio-rig.png`, `-expressions.png`, `-lip-sync.png` | 460 × 900 | `crates/editor/src/app/character_studio.rs:289` | The older Character Studio tab inside the editor (Rig, Expressions, Lip sync), not the workspace. |

Two things to know before any of these are kept:

- **The Body and Face snapshots print a local folder path** in the workspace's status line ("Characters are saved in …"). They must not be committed as they are.
- The snapshots are drawn, not compared: no test holds them to a reference picture. They show the same skin grain, pale jaw line and saw-tooth floor shadow as the captures.

## 8. Open points for the lead

1. Row 2 of the tests needs a run with the editor closed.
2. H1 to H3 are visible in the default view of the default characters and are not in the plan's defect lists. H1 (the briefs) shows in every full-body view of the Female body.
3. The benchmark set has no arms-raised, bent-elbow or bent-knee capture in the engine, no warm, cool or outdoor light, and its cameras miss the head of the shortest and tallest bodies (section 5). The benchmark plan should decide whether to add views and presets (the rule in visual-qa.md is to add, never change).
4. The `Walk` capture is a run stride: the default state machine plays the `Run` take for Walk and the `Sprint` take for Run (`crates/project/src/default_humanoid.rs:127` to `:144`). Brief §17.2 asks for a walking animation; the benchmark plan should say which take that is.

## 9. The R1 cast (RA6, 2026-11-17)

Added in the refinement's R1 (plan 11.6, brief §17.1 and §17.2): the benchmark **cast**, the capture's new lights, views, held poses and suites, and the Walk state playing the Walk take. Captured on the same reference PC from the `r1-benchmarks` branch (this folder's `main` plus RA6's change); how to run everything is in [visual-qa.md](../../visual-qa.md) ("Added beside them").

### The cast

`assets/characters/benchmarks/realistic-<frame>-cast-<name>.character.json`, for `masculine` and `feminine`, written by `make.py` (`CAST`). All bare: underwear and bald, as the family has no clothes or hair.

| Name | Realistic parameters (both frames unless shown) | Source |
|---|---|---|
| `average` | None: the default body. It replaces the `face-neutral` files, which were the same characters as `body-default` (section 2) | Brief §17.1 |
| `slim` | `fat` −0.6, `muscle` −0.2 | Asset audit's proposal |
| `athletic` | `muscle` 0.7 (masculine) or 0.6 (feminine), `fat` −0.4 | The workspace's Athletic presets |
| `muscular` | `muscle` 0.9, `fat` 0.1, `shoulder_width` 1.05 | Asset audit |
| `heavyset` | `fat` 0.8, `muscle` 0.2 | The workspace's Heavyset presets |
| `broad-face` | The Broad face preset: temples 0.6, cheekbones 0.6, jaw 0.5, eye spacing 0.3, mouth 0.3, chin width 0.3 and the rest of its column | [references/face/README.md](../../references/face/README.md) §3 |
| `narrow-face` | The Narrow face preset (temples −0.6, jaw −0.5, cheekbones −0.4, …) | The same |
| `distinctive` | `nose_bridge` 0.8, `nose_length` 0.6, `brow_ridge` 0.7, `chin_forward` 0.5, ears out 0.6, eyes deep 0.5, `upper_lip_fullness` −0.4 | Asset audit |
| `asymmetry` | Every row of the face references' §4 "benchmark asymmetry face" column at once: the left brow and eye higher, the right cheekbone, jaw, ear and eye larger, `jaw_shift` 0.7 and `nose_tilt` 0.6, the mouth wider on the same side | The same, §4 |

The stylized family has the same cast less `asymmetry` (its manifest has no sided parameter), on its own parameters (`build`, `muscle`, `head_width`, …).

### Commands

`<cast>` is the 18 files `assets/characters/benchmarks/realistic-*-cast-*.character.json`, separated by `;`; `<scratch>` a folder under the system temp directory. Every run ended `test result: ok. 1 passed; 0 failed`.

| Run | Command (all with `AXCENITY_CAPTURE_DESCRIPTIONS=<cast> AXCENITY_CAPTURE_SHEETS=only cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture`) | Time |
|---|---|---|
| `face` | `AXCENITY_CAPTURE_SUITE=face AXCENITY_CAPTURE_DIR=<scratch>/cast/face` | 168 s (6.7 to 13 s a character, 20 pictures) |
| `body` | `AXCENITY_CAPTURE_SUITE=body AXCENITY_CAPTURE_DIR=<scratch>/cast/body` | 224 s (24 pictures each) |
| `lights` | `AXCENITY_CAPTURE_LIGHTS=studio,warm,cool,outdoor AXCENITY_CAPTURE_VIEWS=front,three-quarter,face-front,skin-close AXCENITY_CAPTURE_DIR=<scratch>/cast/lights` | 152 s (16 pictures each) |

A face sheet is 2560 × 1800 (rows: neutral, smile, frown, eyes closed, mouth open; columns: `face-front`, `face`, `face-three-quarter`, `face-side`), a body sheet 2560 × 2160 (rows: rest, T-pose, arms raised, elbows bent, knees bent, `Walk@0.3`; columns: front, three-quarter, side, back), a lights sheet 2560 × 1440 (rows: studio, warm, cool, outdoor).

**The default run is unchanged.** Captured without the new options, `realistic-masculine-body-default` and `realistic-feminine-body-heavy` give sheets byte-identical to `engine/rest/` here, and the same `captures.json` settings.

### The committed set

[engine/cast/](engine/cast/): 9 sheets and 3 `captures.json` files (each lists all 18 characters of its run), **13.2 MB** (13,165,700 bytes). Every one of the 54 sheets was looked at; these were chosen to show the findings below.

| Folder | Sheets |
|---|---|
| [engine/cast/face/](engine/cast/face/) | `realistic-masculine-cast-average-face`, `realistic-feminine-cast-average-face`, `realistic-feminine-cast-asymmetry-face`, `realistic-masculine-cast-distinctive-face` |
| [engine/cast/body/](engine/cast/body/) | `realistic-masculine-cast-average-body`, `realistic-feminine-cast-heavyset-body`, `realistic-masculine-cast-muscular-body` |
| [engine/cast/lights/](engine/cast/lights/) | `realistic-masculine-cast-average`, `realistic-feminine-cast-heavyset` |

### What the sheets show

Against the defects of section 4. "Still" means a defect listed there is visible again; "new" means it isn't in that list.

| # | What | Where | Against section 4 |
|---|---|---|---|
| C1 | **The face cast barely differs.** In the neutral `face-front` picture the face members differ from `average` by 0.3 to 0.7 % of the pixels (masculine: asymmetry 0.66, broad 0.45, distinctive 0.42, narrow 0.37; feminine 0.61, 0.39, 0.46, 0.32), less than the body members (heavyset 1.8, muscular 1.5). Side by side the broad and narrow faces differ in the jaw and a little at the cheekbones; the skull, brow and eyes stay the same, so they read as the same person. The asymmetry shows as one brow and eye a little higher and the chin off to one side; it reads as a person, not a fault, but only beside the average. The distinctive face shows its ears and a slightly heavier brow; the high bridge and long nose hardly show from the front. At the references' preset strengths the face parameters are too weak to make different people: R4's work (and M6, the two frames' nearly identical faces) | `face/*-face.png`, row 1 | **New** (M6 is its frame-to-frame form) |
| C2 | **Heavyset is not heavy** on either frame: a thicker waist and arms, abdominal lines still drawn, no belly in the side view | `body/realistic-feminine-cast-heavyset-body.png`, `lights/realistic-feminine-cast-heavyset.png` | Still H6, H5 |
| C3 | The open mouth is a dark bell with pointed corners; from the side the jaw drops as a block. The smile is a thin line with no cheek movement; the frown moves the brows and the mouth corners only | `face/*`, rows 2, 3, 5 | Still H8, M12 |
| C4 | Eyes closed: both lids meet and no eyeball shows, in all four views, on every cast member | `face/*`, row 4 | As before: no defect |
| C5 | **The held poses work and show what the clips did not.** The T-pose and arms raised turn at the shoulder with no visible collapse at this size; the shoulder top flattens a little at 150 degrees. Bent elbows bring the hands to the front of the chest, the fingers at the skin, so whether they pass into it can't be judged from these views. Bent knees fold the shins back, and the body hangs above the floor (only the knees turn, as in the Blender harness's `knee_135`). With the arms raised the hands leave the top of the full-body frames (fixed cameras) | `body/*`, rows 2 to 5 | New poses; no new defect seen at this size. The 90-degree shoulder targets the Blender check misses (section 6) are not visible here |
| C6 | **Walk is a walk.** `Walk@0.3` is now an upright stride with the arms low, against the running stride of `engine/walk/` | `body/*`, row 6 | Section 2's "the Walk clip is a run": fixed |
| C7 | Skin under the close view: a coarse, even pebble grain over the nose, cheek and forehead in every light, and an even wet gloss on the nose and lips. The new view shows it at its worst | `lights/*`, the last column | Still H4 |
| C8 | The new lights read as meant: warm is a low orange sun from the front left with a long floor shadow (the skin turns saturated orange, the darkest shadow brown); cool is a soft blue-grey overcast, under which the face goes dark and muddy; outdoor is a blue-sky midday with a short shadow and a lit face. None shows a defect the studio light hides, except that the grain (H4) and the hard shadow-line on the forehead (H2) show most under `outdoor` | `lights/*` | H2 and H4 still |
| C9 | The briefs' pointed sides, the bra's flat straps, the ragged shadow edges and the pale jaw line are unchanged on every cast member | Every sheet | Still H1, H2, M4, M9 |
| C10 | The 50/50 blend still wears the Male boxer briefs: a tie between the Male and Female choices goes to the first (`avatar/src/choices.rs`, `manifest.rs` `nearest_choice`). The benchmark files now list it, so they say what is drawn | `realistic-blend-*.character.json` | Still M11; its cause is the resolver, not `make.py` |

Not captured: the stylized cast (its sheets are in [../../baseline/engine/](../../baseline/engine/) for the older cases), the face suite under `hard`, the `face-three-quarter`, `face-side` and `skin-close` views of the body members.
