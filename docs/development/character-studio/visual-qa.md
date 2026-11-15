# Character Studio overhaul: visual QA and benchmarks

Addition A1 of [the plan](../../../plans/axcenity-engine-character-studio.md): a fixed set of benchmark characters and poses, measured and rendered at every stage, so each stage is judged by whether the characters **measurably look and behave better**, not by whether the code works. Tests and commands are in [testing.md](testing.md).

## The benchmark set

Every benchmark is a **description file** (architecture §5), so the same input feeds the in-engine capture, the Blender harness and later stages unchanged. They live in `assets/characters/benchmarks/`, one `*.character.json` each, named `<family>-<frame>-<case>` (`stylized-feminine-body-heavy`), written by `python3 -I assets/characters/benchmarks/make.py` from the library's manifests: run it again when a base model's parameters, slots or items change (it makes the set for every base model in the catalogue, so a new family gets its set by being listed). `avatar`'s `every_benchmark_character_is_sound` checks each resolves with nothing left out. Since CO4 the set is 136 files, 13 of them `realistic-*` (the masculine and feminine frames and their blend, each at the default, shortest and tallest proportions, and a neutral face and an underwear-only file for each frame; that base model has no builds, face parameters, items or hair yet). The Blender harness still sets its looks itself (`qa_scene.py`) and does not read these files, for either package.

| Group | Characters | Why |
|---|---|---|
| Bodies | Each family × masculine, feminine, the 50/50 blend; each × default, slim, heavy, muscular, heavy-and-muscular, the shortest and tallest proportions | Body proportions; composition layers; frame offsets |
| Faces | Each frame × neutral, and the identity extremes of each face region (eyes, brows, nose, cheeks, jaw, chin, lips, forehead, ears) at −1 and 1 | Facial identity stays human at the ends |
| Outfits | Underwear only; the default outfit; every wardrobe item alone; the heaviest layering (underwear, base, mid, outer) | Clothing fit and layer order |
| Hair | Every groom on each frame | Scalp contact, clipping, silhouette |

**Poses** (left side, `assets/characters/humanoid/qa/qa_scene.py` `POSES`): `rest` (A-pose), `elbow_135`, `knee_135`, `shoulder_abd_90`, `shoulder_abd_150`, `hip_flex_100`, `wrist_70`, `spine_twist_45`, `fist`. **Face poses:** neutral, full blink, each of the 15 built-in expressions, jaw fully open, each viseme. **Motion** *(CO11 on)*: the Idle and Walk clips at fixed times for hair and cloth.

## Measurements (`measure.py`)

| Area | Measures | Weighted as |
|---|---|---|
| Body proportions | Heads tall; shoulder (bideltoid) width, hand, upper arm and leg ÷ height, against Drillis and Contini (1966) | Body |
| Facial deformation | Eyeball front uncovered in a blink; eyeball vertices inside the lids at rest; teeth inside lip or chin flesh with the jaw open; lip self-intersection at rest; each identity shape's largest move; `Neck` weight above the head joint, `Jaw` weight in the upper lip | **As heavily as the body** (addition A4) |
| Joint deformation | `local_volume_ratio` and `ring_area_ratio` at each pose's joint; fist self-intersection | Body |
| Twist (CO4, `twist`) | A limb turned about its own length (`forearm_twist_90`, `upper_arm_twist_70`, `thigh_twist_45`): the smallest cross-section area along it, posed over rest (`ring_area_ratio_min`; 1 is no pinching) | Body |
| Clothing fit | Poke-through points and depth per garment, waist gap, layer order, for every frame × composition, at rest and in the poses | Clothing |

Each stage's numbers go into `docs/development/character-studio/<stage>/measurements.json` beside its sheets; the thresholds in [testing.md](testing.md) are checked with `measure.py --check`. A thresholds file holds `at_least`, `at_most` and `within` limits by a measure's path, and `targets` with the same keys for what a build is meant to reach but hasn't: `--check` prints unmet targets and still passes on the limits, `--strict` fails on them too.

**Packages.** `measure.py` and `render_baseline.py` take `--package` (`stylized`, the default, or `realistic`), which picks that package's parts file and build data; without it the stylized run is unchanged. The realistic body's limits are `assets/characters/realistic/qa/thresholds.json`. In every pose the scripts turn the twist and helper bones as `crates/anim/src/twist.rs` does (`Character.drive` in `qa_scene.py`); when that rule changes, change both.

## Renders

**Blender harness** (`render_baseline.py`, Stage 1's): Cycles CPU, AgX, OIDN, three area lights (key, fill, rim) and a grey cyclorama, seed 7; sheets: turnarounds (front, three-quarter, side, back) per frame, face close-ups and topology, base body clay, build range, hair styles, the test poses, expressions, details, fit. A package without a wardrobe, hair or face shapes (`--package realistic` today) skips those sheets and gets four more: `twist_poses` (the limbs turned about their length), `clips` (the starter animations retargeted onto it; the last tile is the same take bound by bone name, which the engine no longer does), `lods` (the body and its two simpler copies) and `skin_maps`. CO4's sheets and `measurements.json` are in [co4/](co4/). Never change `STUDIO`, the cameras or the poses, or the sets stop being comparable.

**In-engine capture** (`crates/player/src/character_capture.rs`, CO2): the player's renderer draws a character from a description file, applied as a game applies it (`gameplay`'s applier), from five fixed cameras (`front`, `three-quarter`, `side`, `back`, `face`) under two lighting presets (`studio`: Character Studio's Studio stage on its grey backdrop; `hard`: a low key light with little fill on a dark backdrop, which shows shading, seams and clipping), at 1280 × 720, High quality, TAA, 48 frames drawn before each picture is read, each picture on a renderer of its own. The character stands in the rest pose (`AXCENITY_CAPTURE_POSE=idle` is one second into Idle). Same input gives identical pictures (`the_same_description_captures_identical_pictures_and_every_view_is_drawn`).

```text
AXCENITY_CAPTURE_DESCRIPTIONS=assets/characters/benchmarks AXCENITY_CAPTURE_DIR=<folder> \
  cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
```

It writes `<name>/<light>-<view>.png` per picture, a contact sheet `<name>.png` (a row per light, a column per view, at half size) and `captures.json` (the settings and what was drawn). `AXCENITY_CAPTURE_DESCRIPTIONS` takes files or folders separated by `;`; `AXCENITY_CAPTURE_ONLY=<text>` keeps the names containing it; `AXCENITY_CAPTURE_SHEETS=only` writes only sheets; `AXCENITY_CAPTURE_LIBRARY=<folder>` draws from another character library (a specialist's worktree). `captures.json` also records the level of detail each part was drawn at in each view: in the full-body views small parts are drawn from simpler copies, as in a game, and a simplified garment can show skin that the full mesh doesn't (a known defect, audit §3.10), so judge clothing fit in the `face` view (full detail) and in Blender until characters switch levels together (CO18). About 2 to 4 seconds a character on the reference PC. **Never change `VIEWS`, `LIGHTS`, the stage or `Settings::default`**: add a view or a preset instead, and re-capture the baseline in the same change if one must change.

## Review

1. Render, capture and measure `main` (the "before") and the branch (the "after") with the same scripts and settings, into `baseline/` (in-engine sheets in `baseline/engine/`) and `<stage>/` (`<stage>/engine/`).
2. Compare sheet by sheet at the same size; compare `measurements.json` key by key.
3. Look for: proportions against the anatomy references (research A.1); joints that pinch, collapse or candy-wrap; eyes through lids, teeth through lips; clothes through each other or the body; hair floating or sinking; shading seams; anything the numbers can't see.
4. Record what improved, what regressed and what is unchanged in [progress.md](progress.md), with the numbers.

**Accepted** means: no measure got worse beyond its threshold, the milestone's own measures improved, and the lead's review of the sheets found no new visible defect. The stage's reference picks (research A.0) are the bar; generated content is judged honestly against them (plan risk 1).

## Baseline

The current baseline is Stage 1's, re-rendered after #401's fixes: [baseline/](baseline/) (sheets and `measurements.json`), originals in [baseline/pre-fix/](baseline/pre-fix/). The in-engine baseline of the stylized character is [baseline/engine/](baseline/engine/): the contact sheets of a chosen set of benchmarks (both frames in the default outfit and in underwear at the default, slim, heavy and muscular builds, the blend, the shortest and tallest proportions, the heaviest layering) and its `captures.json`. The stylized set is 123 characters and is captured on demand, not committed. It was captured with CO3's applier, whose result a test holds to what Character Studio's preview showed before (`a_character_file_applied_in_a_game_matches_the_workspaces_preview`), so it is the pre-overhaul character.
