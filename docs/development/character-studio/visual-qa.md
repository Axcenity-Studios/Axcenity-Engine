# Character Studio overhaul: visual QA and benchmarks

Addition A1 of [the plan](../../../plans/axcenity-engine-character-studio.md): a fixed set of benchmark characters and poses, measured and rendered at every stage, so each stage is judged by whether the characters **measurably look and behave better**, not by whether the code works. Tests and commands are in [testing.md](testing.md).

## The benchmark set

Every benchmark is a **description file** (architecture §5), so the same input feeds the Blender harness, the in-engine capture *(CO2)* and later stages unchanged. They live in `assets/characters/benchmarks/` *(CO2 writes them)*, one `*.character.json` each, named `<family>-<frame>-<case>`.

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
| Clothing fit | Poke-through points and depth per garment, waist gap, layer order, for every frame × composition, at rest and in the poses | Clothing |

Each stage's numbers go into `docs/development/character-studio/<stage>/measurements.json` beside its sheets; the thresholds in [testing.md](testing.md) are checked with `measure.py --check`.

## Renders

**Blender harness** (`render_baseline.py`, Stage 1's): Cycles CPU, AgX, OIDN, three area lights (key, fill, rim) and a grey cyclorama, seed 7; sheets: turnarounds (front, three-quarter, side, back) per frame, face close-ups and topology, base body clay, build range, hair styles, the test poses, expressions, details, fit. Never change `STUDIO`, the cameras or the poses, or the sets stop being comparable.

**In-engine capture** *(CO2)*: the player's renderer from a description file, at 1280×720 or larger, fixed cameras (full body, three-quarter, side, back, face) and two lighting presets (the workspace's Studio, and a hard key light that shows shading and clipping), TAA settled (the same number of frames each time), the same graphics preset each time, written as PNG sheets. Same input must give identical images.

## Review

1. Render and measure `main` (the "before") and the branch (the "after") with the same scripts and settings, into `baseline/` and `<stage>/`.
2. Compare sheet by sheet at the same size; compare `measurements.json` key by key.
3. Look for: proportions against the anatomy references (research A.1); joints that pinch, collapse or candy-wrap; eyes through lids, teeth through lips; clothes through each other or the body; hair floating or sinking; shading seams; anything the numbers can't see.
4. Record what improved, what regressed and what is unchanged in [progress.md](progress.md), with the numbers.

**Accepted** means: no measure got worse beyond its threshold, the milestone's own measures improved, and the lead's review of the sheets found no new visible defect. The stage's reference picks (research A.0) are the bar; generated content is judged honestly against them (plan risk 1).

## Baseline

The current baseline is Stage 1's, re-rendered after #401's fixes: [baseline/](baseline/) (sheets and `measurements.json`), originals in [baseline/pre-fix/](baseline/pre-fix/). CO2 adds the in-engine baseline captured on merged `main`.
