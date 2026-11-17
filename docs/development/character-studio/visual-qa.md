# Character Studio overhaul: visual QA and benchmarks

Addition A1 of [the plan](../../../plans/axcenity-engine-character-studio.md): a fixed set of benchmark characters and poses, measured and rendered at every stage, so each stage is judged by whether the characters **measurably look and behave better**, not by whether the code works. Tests and commands are in [testing.md](testing.md).

## The benchmark set

Every benchmark is a **description file** (architecture §5), so the same input feeds the in-engine capture, the Blender harness and later stages unchanged. They live in `assets/characters/benchmarks/`, one `*.character.json` each, named `<family>-<frame>-<case>` (`stylized-feminine-body-heavy`), written by `python3 -I assets/characters/benchmarks/make.py` from the library's manifests: run it again when a base model's parameters, slots or items change (it makes the set for every base model in the catalogue, so a new family gets its set by being listed). `avatar`'s `every_benchmark_character_is_sound` checks each resolves with nothing left out, wears what its choices make it wear (the realistic underwear follows Male or Female, so a file lists the underwear that is drawn; the 50/50 blend gets the Male choice, as the resolver gives a tie to the first), and that every base has the cast. Since the refinement's R1 the set is 377 files: 238 `realistic-*` (30 bodies, 188 face identity ends, 2 underwear outfits, 18 cast) and 139 `stylized-*` (123 and 16 cast). The realistic `face-neutral` files are folded into the cast: they were the same characters as `body-default` (0 pixels differ), and `cast-average` is now the realistic face reference. The Blender harness still sets its looks itself (`qa_scene.py`) and does not read these files, for either package.

| Group | Characters | Why |
|---|---|---|
| Bodies | Each family × masculine, feminine, the 50/50 blend; each × default, slim, heavy, muscular, heavy-and-muscular, the shortest and tallest proportions | Body proportions; composition layers; frame offsets |
| Faces | Each frame × neutral, and the identity extremes of each face region (eyes, brows, nose, cheeks, jaw, chin, lips, forehead, ears) at −1 and 1 | Facial identity stays human at the ends |
| Outfits | Underwear only; the default outfit; every wardrobe item alone; the heaviest layering (underwear, base, mid, outer) | Clothing fit and layer order |
| Hair | Every groom on each frame | Scalp contact, clipping, silhouette |
| **Cast** (R1, brief §17.1) | Each family × masculine and feminine × `average`, `slim`, `athletic`, `muscular`, `heavyset`, `broad-face`, `narrow-face`, `distinctive`, and `asymmetry` where the base has sided face parameters (the realistic one; the stylized manifest has none). Bare (underwear, bald) until the family has clothes; then each gets a dressed twin | Representative people rather than slider ends: the set every stage is judged on before and after |

The cast's values are in `make.py`'s `CAST`, with their sources: `athletic` and `heavyset` are the workspace's Athletic and Heavyset presets (so the Presets page and the benchmark show the same people); `broad-face` and `narrow-face` the Broad and Narrow presets of [references/face/README.md](references/face/README.md) §3; `distinctive` the asset audit's proposal (a high bridge, long nose, strong brow ridge, forward chin, ears out, deep-set eyes, a thin upper lip); `asymmetry` that file's §4 "benchmark asymmetry face" column, every row at once (1.6 to 3.7 mm a feature, the 1 to 3 per cent between sides that healthy faces show, at the size people start to notice): the left brow and eye higher, the right cheekbone, jaw, ear and eye larger, the jaw, nose and mouth all a little to the left. Change a value only with a re-capture of the cast in the same change.

**Poses** (left side, `assets/characters/humanoid/qa/qa_scene.py` `POSES`): `rest` (A-pose), `elbow_135`, `knee_135`, `shoulder_abd_10`, `shoulder_abd_90`, `shoulder_abd_150`, `hip_flex_100`, `wrist_70`, `spine_twist_45`, `fist`. In the engine (R1), both sides at once: `t-pose` (`shoulder_abd_90`), `arms-raised` (`shoulder_abd_150`), `elbows-bent` (`elbow_135`), `knees-bent` (`knee_135`). **Face poses:** neutral, full blink, each of the 15 built-in expressions, jaw fully open, each viseme. **Motion** *(CO11 on)*: the Idle and Walk clips at fixed times for hair and cloth.

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

**In-engine capture** (`crates/player/src/character_capture.rs`, CO2): the player's renderer draws a character from a description file, applied as a game applies it (`gameplay`'s applier), from six fixed cameras (`front`, `three-quarter`, `side`, `back`, `face`, and `face-front`, added in CO6: `face` stands a little to one side, so a mouth or a brow that is higher on one side is judged straight on) under two lighting presets (`studio`: Character Studio's Studio stage on its grey backdrop; `hard`: a low key light with little fill on a dark backdrop, which shows shading, seams and clipping), at 1280 × 720, High quality, TAA, 48 frames drawn before each picture is read, each picture on a renderer of its own. The character stands in the rest pose (`AXCENITY_CAPTURE_POSE=idle` is one second into Idle; `AXCENITY_CAPTURE_POSE=Cheer@0.8` is that many seconds into one of the default character's clips played alone, which shows a joint as a game's animation leaves it: CO5's raised shoulders were judged this way). `AXCENITY_CAPTURE_FACE=Blink_Left:1,Jaw_Open:0.5` or `Happy:1` shows face controls and expressions at those strengths, with blinking, gaze and idle motion off, so a face is judged in the engine and not only in Blender: CO6's first engine pictures showed the lower teeth shut inside an open mouth, which no Blender sheet could. Same input gives identical pictures (`the_same_description_captures_identical_pictures_and_every_view_is_drawn`).

```text
AXCENITY_CAPTURE_DESCRIPTIONS=assets/characters/benchmarks AXCENITY_CAPTURE_DIR=<folder> \
  cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
```

It writes `<name>/<light>-<view>.png` per picture, a contact sheet `<name>.png` (a row per light, a column per view, at half size) and `captures.json` (the settings and what was drawn). `AXCENITY_CAPTURE_DESCRIPTIONS` takes files or folders separated by `;`; `AXCENITY_CAPTURE_ONLY=<text>` keeps the names containing it; `AXCENITY_CAPTURE_SHEETS=only` writes only sheets; `AXCENITY_CAPTURE_LIBRARY=<folder>` draws from another character library (a specialist's worktree). `captures.json` also records the level of detail each part was drawn at in each view: in the full-body views small parts are drawn from simpler copies, as in a game, and a simplified garment can show skin that the full mesh doesn't (a known defect, audit §3.10), so judge clothing fit in the `face` view (full detail) and in Blender until characters switch levels together (CO18). About 2 to 4 seconds a character on the reference PC. **Never change `VIEWS`, `LIGHTS`, the stage or `Settings::default`**: add a view or a preset instead, and re-capture the baseline in the same change if one must change.

**Added beside them (refinement R1, brief §17.2).** A run without the new options draws exactly what it drew before (checked: the default sheets of `realistic-masculine-body-default` and `realistic-feminine-body-heavy` are byte-identical to the refinement baseline's).

| Addition | Names | What |
|---|---|---|
| Lights (`MORE_LIGHTS`), chosen with `AXCENITY_CAPTURE_LIGHTS=studio,warm,...` | `warm`, `cool`, `outdoor` | `warm`: a low orange sun from the front left at the end of the day, a warm ambient, a dusky backdrop. `cool`: an overcast light, soft and bluish, on a blue-grey backdrop. `outdoor`: a high white midday sun under a blue sky gradient that lights from above. Each sets its sun's direction, colour and strength and the ambient's colour (`Tint`); `studio` and `hard` keep theirs |
| Views (`MORE_VIEWS`), chosen with `AXCENITY_CAPTURE_VIEWS=front,skin-close,...` | `face-three-quarter`, `face-side`, `skin-close` | The face at 45 degrees and in profile, at `face`'s distance; a close crop of the left cheek and the nose at about 3/4 (field of view 24 degrees) for the skin's grain, gloss and shadow edges. The cameras are fixed for a default-height head: the feminine frame's crop shows the eye and the nose, the masculine one's the nose and the lips |
| Held poses, `AXCENITY_CAPTURE_POSE=` | `t-pose`, `arms-raised`, `elbows-bent`, `knees-bent` | Both sides, at the Blender harness's angles (abduction 90 and 150 degrees, elbows 135 forward, knees 135 back), turned on the body's own skeleton and played as a clip of one key a bone, the way `Walk@0.5` plays a starter clip (`HeldPose`, `held_pose`) |
| Suites, `AXCENITY_CAPTURE_SUITE=` | `face`, `body` | One sheet per character, `<name>-<suite>.png`: a row per state (and light), a column per view. `face`: neutral, smile (`Happy`), frown (`Sad`), eyes closed (`Blink_Left`, `Blink_Right`), mouth open (`Jaw_Open`), from `face-front`, `face`, `face-three-quarter`, `face-side`. `body`: rest, `t-pose`, `arms-raised`, `elbows-bent`, `knees-bent`, a walk frame (`Walk@0.3`, legs apart), from `front`, `three-quarter`, `side`, `back`. Lights default to `studio`; `AXCENITY_CAPTURE_LIGHTS` and `_VIEWS` override. Pictures go to `<name>/<suite>/<state>/<light>-<view>.png`; `captures.json` adds `suite` and `states` (the rows in order) |

Since R1 the default character's `Walk` state and clip play the Walk take (it was the Run take), so `Walk@<seconds>` is a walking stride.

```text
# The cast, both frames, with each suite (a sheet per character and suite)
AXCENITY_CAPTURE_DESCRIPTIONS=<the realistic-*-cast-* files, separated by ;> AXCENITY_CAPTURE_SUITE=face \
  AXCENITY_CAPTURE_SHEETS=only AXCENITY_CAPTURE_DIR=<folder> \
  cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
# ... AXCENITY_CAPTURE_SUITE=body ...
# Under the rendering lights: a row per light, a column per view
AXCENITY_CAPTURE_LIGHTS=studio,warm,cool,outdoor AXCENITY_CAPTURE_VIEWS=front,three-quarter,face-front,skin-close ...
```

On the reference PC a character's face suite (20 pictures) takes about 9 s and its body suite (24) about 12 s, after a release build. The R1 captures of the realistic cast and what they show are in [refinement/baseline/README.md](refinement/baseline/README.md) section 9.

## Review

1. Render, capture and measure `main` (the "before") and the branch (the "after") with the same scripts and settings, into `baseline/` (in-engine sheets in `baseline/engine/`) and `<stage>/` (`<stage>/engine/`).
2. Compare sheet by sheet at the same size; compare `measurements.json` key by key.
3. Look for: proportions against the anatomy references (research A.1); joints that pinch, collapse or candy-wrap; eyes through lids, teeth through lips; clothes through each other or the body; hair floating or sinking; shading seams; anything the numbers can't see.
4. Record what improved, what regressed and what is unchanged in [progress.md](progress.md), with the numbers.

**Accepted** means: no measure got worse beyond its threshold, the milestone's own measures improved, and the lead's review of the sheets found no new visible defect. The stage's reference picks (research A.0) are the bar; generated content is judged honestly against them (plan risk 1).

## Baseline

The refinement's baseline of the realistic family is [refinement/baseline/](refinement/baseline/README.md) (`main` at 99f5b181, and the R1 cast in `engine/cast/`). The current baseline of the stylized family is Stage 1's, re-rendered after #401's fixes: [baseline/](baseline/) (sheets and `measurements.json`), originals in [baseline/pre-fix/](baseline/pre-fix/). The in-engine baseline of the stylized character is [baseline/engine/](baseline/engine/): the contact sheets of a chosen set of benchmarks (both frames in the default outfit and in underwear at the default, slim, heavy and muscular builds, the blend, the shortest and tallest proportions, the heaviest layering) and its `captures.json`. The stylized set is 123 characters and is captured on demand, not committed. It was captured with CO3's applier, whose result a test holds to what Character Studio's preview showed before (`a_character_file_applied_in_a_game_matches_the_workspaces_preview`), so it is the pre-overhaul character.
