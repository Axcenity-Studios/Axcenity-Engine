# Hand-back: RA3, R3 wider ranges (branch `r3-ranges`)

Plan section 0, decision 8; plan 11.5, R3 ("every proportion's range is recorded with the check or percentile that set each end, and hips go below 1"). Branched from `refinement-r1-wave1`. The rules, the table and the numbers are in [../r3/ranges.md](../r3/ranges.md); the pictures in [../r3/ranges/](../r3/ranges/).

## What it does

1. **A sweep tool**, `assets/characters/humanoid/qa/ranges.py` (Blender). For each of the nine proportions, both frames, the frame's own body and the four corners of `muscle` and `fat`: steps of 0.025 outward from 1 both ways, each checked for new self-crossings at rest (the sliders pass's method), joint ring areas in the poses the slider bears on against `thresholds.json` (or 0.9 of the body's own where there is no limit), eyes and teeth in flesh, collapse (cross-section breadth and region skin area at least 70% of the body's at 1) and a step in the trunk's outline (at most 0.05 between neighbouring levels). It measures a dimension per slider against ANSUR II's 0.5th and 99.5th percentiles. Each end is the nearest to 1 of the last passing step, the farther frame's percentile (rounded toward 1), and `anim`'s clamp. Bones are placed exactly as `anim` places them on a pose (Blender `inherit_scale` NONE with the matrix computed per bone; self-check 0.027 mm on a posed body). `--report` prints the Markdown tables. `render_ranges.py` renders side-lit clay close-ups.
2. **Hips below 1.** `<Frame>_Hips_Narrow` (`shapes.hips_narrow`): a scripted shape built from the hip bone's own width change, applied to the skin (side to side only, toward the middle, inner thighs held, faded above the crest and down the thigh), a corner of the frame and `hip_width` from 1 to 0.6. Below 1 `hip_width` moves no bone: `manifests.ATTRIBUTE_FROM_ONE` makes its attribute driver run from 1 up (the bones scale less than the surface: not at all). Its strength matches the bone's widening slope, so the slider has no kink at 1 (hip breadth 0.864 at 0.7, 1.099 at 1.2). No MakeHuman target was used, so `LICENCE-RECORD.md` is unchanged. No corrective was needed: the hip flex passes at every step and looks the same at 0.7 as at 1.
3. **The found ranges shipped** in `manifests.PROPORTION_RANGES` (the attribute spans follow them; the hips' starts at 1). `anim` clamps at 0.5 to 1.5, which every range is inside, so `anim` is unchanged.

| | `height` | `leg_length` | `arm_length` | `shoulder_width` | `waist_width` | `hip_width` | `head_size` | `hand_size` | `foot_size` |
|---|---|---|---|---|---|---|---|---|---|
| CO5 | 0.85 to 1.15 | 0.85 to 1.15 | 0.85 to 1.15 | 0.85 to 1.15 | 0.85 to 1.15 | 1 to 1.15 | 0.85 to 1.15 | 0.85 to 1.15 | 0.85 to 1.15 |
| Now | 0.82 to 1.13 | 0.85 to 1.125 | 0.95 to 1.08 | 0.88 to 1.12 | 0.775 to 1.225 | **0.7** to 1.2 | 0.88 to 1.12 | 0.82 to 1.07 | 0.96 to 1.21 |
| Set by (low, high) | population, population | deformation and population, deformation | deformation, population | population, population | deformation, deformation | deformation, deformation | population, population | population, population | population, population |

**Five ranges are narrower than CO5's at one end or both.** Decision 8 says the ends come from deformation and population data, and some of CO5's ±15% was several SD: arms (±15% is ±5 SD), heads, shoulders. The base's feet are already short. Its masculine foot is under the 0.5th percentile, so a smaller foot leaves the population at once; the end is 0.96, from the feminine frame. If decision 8 means "wider" literally, the rule has to change, not the numbers. One range covers both frames, so each population end is the farther frame's. A feminine body at `height` 1.13 is 1.89 m, past the 99.5th percentile of women.

## Interfaces

- No schema or contract change. The manifest gains two corners (`Masculine_Hips_Narrow`, `Feminine_Hips_Narrow`) and new ranges, and `hip_width`'s attribute driver is `{"attribute": "HipWidth", "from": [1, 1.2]}`.
- `shapes.NARROW_AT` (0.6) and `NARROW_SCALE` (0.8) define the shape. `ranges.py` reads where the shape is whole from the manifest's corner.
- `benchmarks/make.py` writes `<family>-<frame>-body-narrowest-hips` and `-widest-hips` for every base with `hip_width`. That includes the stylized family, whose bone narrowing at 0.5 is known to cross.

## Tests (red first: [r3-ranges-red-run.txt](r3-ranges-red-run.txt))

- New: `crates/avatar/tests/realistic_ranges.rs`.
  - `the_shipped_proportion_ranges_are_the_sweeps` reads ranges.md's "Shipped ranges" table and the manifest, which must agree, with the attribute driver equal to the range except for the hips' start at 1.
  - `the_ranges_stay_inside_anims_clamp`.
  - `hips_narrow_below_one_by_a_shape_not_a_bone`, on both frames: `HipWidth` stays 1, the narrowing shape is driven, the hips are at least 5% narrower, and no vertex moves out or crosses the middle.
- Changed, `realistic_parameters.rs`: the hips' low end moves the baked body; the parameter count has both hip ends; the morph count is 160; the attribute check counts any change over 0.03, because the feet's end is 0.96.
- Changed, `shipped_library.rs`: a base with `hip_width` has hip benchmarks.
- Red: 4 failed, then 1 for the benchmarks (no ranges.md, hips `[1.0, 1.15]`, count 148 against 149, no hip benchmarks).
- Green:
  - `cargo nextest run -p axcenity-avatar -p axcenity-anim --no-fail-fast`: **268 passed**.
  - `-p axcenity-player -p axcenity-project -p axcenity-gameplay`: **465 passed** (20 skipped).
  - `-p axcenity-editor character`: **64 passed** (the face-view test sets `height` 0.85 and 1.15, which now clamp to 0.85 and 1.13, and still passes).
  - `cargo clippy -p axcenity-avatar -p axcenity-anim --all-targets -- -D warnings`: clean. `cargo fmt --all`: clean.
- **Gate:** `measure.py --package realistic --only files,weights,clipping,face,shapes,sliders,proportions,deformation,twist --check thresholds.json`: **`check: passed (2 targets not met)`**, 1,598 s. Sliders: 2,701 pairs and 10,804 bodies per frame, with 0 bodies with new crossings, 0 alone and 0 eye or teeth in flesh, on both frames. The two targets not met are the 90° shoulder's local volume (0.847, 0.88), as before. Nothing was loosened and `thresholds.json` is untouched. Output in [../r3/ranges/measure-check.txt](../r3/ranges/measure-check.txt) and `measurements.json`.
- The sweep rerun with the committed script gives the same table (`ranges/ranges.json`, 277 s). `python scripts/check-wiki.py`: 0 errors (58 warnings, all there before).

## Visual evidence and what I saw

All files are in [../r3/ranges/](../r3/ranges/), 7.7 MB in all.

- **`blender-hips.png`** (clay, one side lamp; front, three-quarter, back).
  - At 0.7 both frames' hips are visibly narrower: the feminine figure goes from an hourglass toward a straighter line, the masculine narrows a little at the hip and buttock. The surface is smooth, with no notch, shelf or flat patch where the shape fades above the crest or down the thigh. The back views show the buttocks narrower and still round.
  - The heavy and soft bodies at 0.7 keep their belly and flanks, and the narrowing reads as narrower hips under them.
  - The old bone narrowing at 0.95 looks almost the same in clay; its fault is crossing at the crotch, which the sweep counts (20 pairs), not a visible shape.
- **`blender-hip-flex.png`**: the hip flex at 100 degrees, 1 against 0.7. They fold alike. The groin crease and the faceted underside of the buttock are the same at 1: they belong to the hip's skinning, not to this change.
- **`blender-outline-check.png`** shows what the outline limit means: the slight masculine body at legs 1.15, waist 0.75 and hips 1.25, where the check fails at 0.051 to 0.055.
  - At legs 1.15 the waist pinches in slightly above the crest. The thigh's helper bones take the leg's length scale and stretch the flank, which is the engine's rule, not the tool's.
  - At waist 0.75 there is a slight corset line. At hips 1.25 a shelf shows at the crest.
  - All are visible but none is broken, so 0.05 is where a fault starts to show.
  - The waist at 1.15 in the spine twist, which first failed the ring check, looked like a crease across the back, but other bodies at the same step were unchanged. It was the ring ordered round a joint near the skin. The tool now orders it round the ring's middle and the waist reaches 1.225.
- **`engine-ends-<frame>-<light>.png`** (in the engine, studio and hard, front and side). Every end looks like a person, not a stretched body:
  - Height 0.82 is a short adult and 1.13 a tall one.
  - Leg length 1.125 gives long legs without a seam at the hips.
  - Arms 0.95 to 1.08 and head 0.88 to 1.12 are subtle, as the population says they should be.
  - Hands and feet change size at the extremities only.
  - The hard-light sheets are dark at sheet scale; the full-size captures show no faults.
- **`engine-widths-<frame>.png`** (close).
  - Shoulders 0.88 to 1.12 read as narrower and broader shoulders without a dent at the collarbone.
  - Waist 0.775 to 1.225 is a mild change: the bone moves the waist's breadth by about a sixth of the slider.
  - Hips 0.7 to 1.2: the narrow feminine hips are clear, and the briefs and boxers follow the narrowing with no gap or poke-through.
- **`engine-suite-<frame>-<light>.png`** (rest, T-pose, arms raised, elbows and knees bent, walk) at the hips' and shoulders' ends: nothing breaks in any pose.
  - In the walk the feminine briefs' side panel stands off the hip on every hip width, at 1 too. That is the baseline's H1, not this change.
- **`engine-benchmarks.png`**: the twelve stature and hip benchmarks.
  - `body-tallest` (height 1.13 with legs 1.125, about 2 m) has its head cut off by the capture's fixed framing, as the earlier tallest did.
  - `body-shortest` is a short, stocky adult.
  - `narrowest-hips` and `widest-hips` look natural on all three frames, the blend included.

## Dependencies

None added.

## Open issues

1. **The waist slider is weak.** `WaistWidth` scales only the skin weighted to the Spine bone: the waist's breadth changes by about a sixth of the slider, and the bone's edge leaves a step at about 0.75 and 1.25, which sets both ends. A waist shape, like the hips', would make it read.
2. **`arm_length`'s low end** is 0.95 because the heavy and soft body's lowered shoulder rises to 1.054 against a ceiling of 1.052 at 0.925. The ceiling is the body's own 1.042 plus `BULGE` 0.01, my rule, and the population would allow 0.92. Allowing 0.02 would give 0.92; it's the lead's call.
3. **`measure.py`'s `set_body_sliders`** sets bones with Blender's full scale inheritance, which drops a child's shear under a parent scaled on one axis. `ranges.py` uses `NONE` inheritance and matches `anim` exactly. RA6 could move `set_body_sliders` to the same rule; the sliders pass is unaffected at these ranges (0 crossings).
4. **The base's feet are short** (masculine under the 0.5th percentile). Fixing the base foot would let `foot_size` go below 0.96.
5. **One range for both frames.** Per-frame ranges would need a schema field. The randomizer draws from the middle half of each range, which suits both frames.
6. **The stylized hip benchmarks** are new files: the stylized body narrows by bone, to 0.5. R3 brings the stylized family to the same checks; that work is outside this task.
7. **The capture frames the tallest bodies with the head cut off.** The view is fixed, and changing `VIEWS` is not allowed.

## What the lead must do at merge

- Rebuild the generated files from the merged sources (`build.py` with the bpy Python, which runs `manifests.py`; then `presets.py`, `catalogue.py`, `benchmarks/make.py`). I regenerated and committed `RealisticParts.glb` (37.4 MB, +0.4 MB for the two shapes), `Realistic.build.json`, `Realistic.base.json`, the three underwear items, `catalogue.json` and the benchmarks (6 changed, 12 new). The presets were rewritten with the same content.
- If another branch changes `shapes.py` or the body, rerun `ranges.py` (about 5 minutes). If any end moves, update `PROPORTION_RANGES` and paste `--report` into ranges.md; `realistic_ranges.rs` holds the two together.
- Write the progress entry. I didn't add one; progress.md is the lead's.
