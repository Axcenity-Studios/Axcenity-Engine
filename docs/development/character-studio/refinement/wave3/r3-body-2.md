# Hand-back: RA3, R3's remaining body work (branch `r3-body-2`)

Plan 11.5, R3 row ("the stylized family brought to the same body checks"; "every proportion's range is recorded with the check or percentile that set each end"), wave 2's open items (the waist slider changes the waist's breadth by about a sixth of its value; heavy and muscular is a bodybuilder; the feminine heavy and muscular outer-thigh bulge). Branched from `refinement-wave2` (b39a9ed7). The rules and tables: [../r3/ranges.md](../r3/ranges.md); the pictures: [r3-body-2/](r3-body-2/).

## What it does

### 1. The waist (realistic)

`anim`'s `WaistWidth` scales only the skin weighted to the Spine bone. Measured at the natural waist (the trunk's narrowest level between the hip crest and the ribs, found on each frame's body), the bone alone moved the waist's breadth by **0.25** of the slider's change on the masculine frame and 0.22 to 0.24 on the feminine, its depth not at all, and its weight's edge left a step in the outline that set both ends of the old range (0.775 and 1.225, 6% narrower and wider).

Two shapes per frame, `<Frame>_Waist_Wide` (whole at 1.5) and `<Frame>_Waist_Narrow` (whole at 0.5), are driven by `waist_width` from 1 together with the bone (`shapes.waist`). They carry the difference between the bone's own move and the waist the slider means: the trunk scaled about its middle by 0.9 of the slider's change side to side and 0.5 front to back (the back a third of what the belly moves), whole at the natural waist and fading smoothly (smoothstep) to nothing 15% of the hip-to-chest height under the hip joints and 0.9 of it above the waist. Because each shape is that target less the bone's move (`T - B`, `B` the Spine weight times the vertex's distance from the middle), the bone and a shape always add up to the target: no kink at 1, no step at the bone's edge. The arms are left out by their bone weights (the A-pose's forearms hang beside the waist).

Scripted, not MakeHuman: `measure-waist-circ` isn't in the vendored set (it would need the bundle fetched and transferred), and it would also have needed the bone's move taken out. `LICENCE-RECORD.md` is unchanged. Reviewed in the engine under studio and hard light (below); the Blender clay close-ups of the waist were not made (the engine close-ups show the surface well at 1:1, and time went to the stylized checks).

Measured with the bone applied by `anim`'s own rule (`crates/gameplay/tests/realistic_waist.rs`):

| Frame | Value | Breadth (front view), share of the slider's change | Depth (side view) | Before (bone only) |
|---|---|---|---|---|
| masculine | 0.8 | **0.815** | 0.311 | 0.251 (at 0.775), depth 0 |
| masculine | 1.175 | **0.883** | 0.311 | 0.249, depth 0 |
| feminine | 0.8 | **0.828** | 0.286 | 0.222, depth 0 |
| feminine | 1.175 | **0.880** | 0.286 | 0.237, depth 0 |

At 0.8 the natural waist is 16% narrower than the default, at 1.175 15% wider (before: 6% at the ends). The sweep (`ranges.py`, rerun in full) stops the slider at **0.8** (the slight masculine body's outline steps by 0.052 at 0.775, where the change fades into the hips; the masculine population's 0.5th percentile of waist over hip breadth is also reached at 0.793) and **1.175** (the lean and muscular masculine body's outline steps by 0.052 at 1.2, under the chest). I widened both fades once (the first version, fading out at the hip joints and 0.75 above, stopped at 0.825 and 1.175). Every other proportion's ends came out unchanged. `PROPORTION_RANGES` and ranges.md's tables are updated; `realistic_ranges.rs` holds them together.

### 2. Heavy and muscular (realistic)

MakeHuman's most muscular and heaviest corner is a heavyweight bodybuilder. `<Frame>_Fat_Over_Muscle`, driven by `muscle` times `fat` (each 0 to 1), covers it (`shapes.fat_over_muscle`): what muscle adds to the heavy body (`E`, the heavy and muscular body less the heavy one, both from the held shapes) is split into its smooth part (40 rounds of halfway to the neighbours' mean, about 10 cm) and the rest; the shape takes off 0.8 of the rest (the definition: separate deltoids, biceps peaks, the trapezius' edge) and 0.45 of the smooth part (the mass). It is kept apart with the other shapes. The skin's normal layers already blend the defined map off at `fat` 1 (R5), so nothing changed there.

This one corrective also removes the feminine outer-thigh bulge: that corner's muscle took the fat off the hips (its hips 19 mm narrower at the hip joints than the heavy body's) while `Fat_Soft`'s saddlebags stayed, so the thigh stood out past the hips. Restoring part of the hip fat and smoothing the muscle's lumps gives a smooth profile.

Measured on the built body:

| | Masculine before | after | heavy | Feminine before | after | heavy |
|---|---|---|---|---|---|---|
| Upper body's definition, RMS mm (`avatar` test: the move from the default less its 10 cm smoothing, height 0.55 to 0.85) | 9.3 | **6.5** | 5.7 | (8.8, Blender) | **5.2** | 4.6 |
| Upper arm girth, mm (Blender, convex hull) | 501 | 468 | 445 | 452 | 408 | 372 |
| Forearm girth, mm | 387 | 347 | 327 | 328 | 276 | 250 |
| Chest girth, mm | 905 | 844 | 790 | 736 | 691 | 649 |
| Shoulder over waist girth | 1.547 | 1.513 | 1.489 | 1.840 | 1.815 | 1.773 |
| Outer thigh past the hips, mm (widest 14 to 30 cm under the hip joints less widest 12 cm under to 4 over) | 4.1 | **0.1** | (-0.9) | 10.8 | **0.7** | (-1.2) |
| Kept apart: crossings before, least factor | 170, 0.33 | 139, 0.41 | | 81, 0.51 | 19, 0.80 | |

Judged against the reference data rather than photographs (references/body holds links and numbers only): the upper body's definition comes down to within 14% of the heavy body's on both frames (it was 63% over), the shoulder-to-waist ratio toward the heavy body's, and the muscle still adds mass over the heavy body (the test holds it). The arms stay large: the masculine upper arm's 468 mm (relaxed, by this convex-hull measure, which reads the default arm at 386) is still past ANSUR II's 99.5th percentile of the flexed biceps (448), as a strongman's would be; the heavy body's is 445.

### 3. The stylized family held to the same body checks

Before: every proportion 0.5 to 1.5; `measure.py --package stylized` with the sliders section found `build` 1 alone crossing itself in **481** (masculine) and **620** (feminine) pairs of triangles, and 662 and 481 of 1,682 bodies with new crossings. No body limit was held (its thresholds file holds the clothing only).

- **Its build shapes are kept apart** (`humanoid/bodyshapes.py`, `keep_apart`; the realistic body's `transfer.apart`): `Build_Heavy` and `Build_Muscular` are scaled down, smoothly, only where they made the body cross itself (the inner thighs, the double chin into the neck, the upper arms' folds), on both frames, at the heavy, muscular and heavy and muscular corners, as if 1.25 times as strong so the proportion sliders' small moves don't bring the crossings back. Cut: masculine heavy 1,332 vertices to 0.15 at the least (inner thighs and arms 0.4 to 0.46 kept on average, neck 0.38, jaw 0.63), feminine heavy 493 to 0.64. The belly's largest forward move is 54.5 mm before and after; the seat and flanks are untouched.
- **Its ranges are its own sweep's** (`ranges.py --package stylized`, new: its build corners, its outline under its armpits, no eye check, its hips by the bone), then two ends brought in by `measure.py`'s pair check: the hips don't narrow (with `build` 1 at 0.975 the heavy inner thighs cross, 11 pairs) and the head stops at 1.025 (with `build` ±1 at 1.05 the chin's fat meets the neck). Shipped in `humanoid/manifests.py` (`PROPORTION_RANGES`), recorded in ranges.md ("Stylized shipped ranges"), held by `the_stylized_proportion_ranges_are_its_sweeps`:

| `height` | `leg_length` | `arm_length` | `shoulder_width` | `waist_width` | `hip_width` | `head_size` | `hand_size` | `foot_size` |
|---|---|---|---|---|---|---|---|---|
| 0.85 to 1.15 | 0.81 to 1.275 | 0.92 to 1.08 | 0.95 to 1.12 | 0.775 to 1.225 | 1 to 1.15 | 0.975 to 1.025 | 0.81 to 1.11 | 0.98 to 1.27 |

- **Held:** `humanoid/qa/body-thresholds.json` (new, beside the clothing's `thresholds.json`): no pair of the body's own sliders and none alone may cross (`body_bodies_with_new_crossings`, `body_alone_with_new_crossings`, new counts in `measure.py`'s sliders pass); each joint keeps the ring area it reaches today. Its elbow (0.37, 0.38) and knee (0.44, 0.45) are far under the realistic body's 0.6: it has no twist or helper bones, and adding them is a redesign, so they are held, not raised.

## QA tool changes (the qa scripts are RA6's; each change is what this task needed)

- `ranges.py`: `--package stylized` and `--parts` (a scratch build); per-package build corners (`COMPOSITIONS`), outline span (`OUTLINE_SPANS`: the stylized armpits are at 1.18 of the hips-to-chest height, so a level through them measured the arm's join and failed every step), the eye check held to the body at 1 and skipped on the stylized body (its eye measure counts its own lids as flesh: 542 and 410 vertices at rest); the hips by the bone where the manifest has no narrowing shape. The realistic sweep is unchanged (rerun: the same ends but the waist's).
- `measure.py`: the sliders pass places bones exactly as `anim` does (`ranges.shaped`, self-checked to 0.027 mm) instead of `set_body_sliders`' full scale inheritance, which drops a child's shear (wave 2's open item 3; on the stylized body it counted 73 masculine body pairs that the engine doesn't make); body-only counts; `--check` takes several files.
- `render_ranges.py`: `--package`, `--parts`, `--aim body`.

## Files

- `assets/characters/realistic/shapes.py` (`waist`, `trunk_levels`, `fat_over_muscle`, the corners, `weights_of`'s default for `waist_width`), `manifests.py` (the waist's range), `README.md`.
- `assets/characters/humanoid/bodyshapes.py` (`keep_apart`), `build.py` (calls it), `manifests.py` (`PROPORTION_RANGES`), `README.md`; `qa/ranges.py`, `qa/measure.py`, `qa/render_ranges.py`, `qa/README.md`, `qa/body-thresholds.json` (new).
- Generated: `realistic/RealisticParts.glb` (44.2 MB, was 42.4; 180 shapes), `Realistic.base.json`, `Realistic.build.json`, the three realistic underwear items; `humanoid/HumanoidParts.glb`, `Humanoid.base.json`; 12 stylized stature and hip benchmarks.
- Tests: `crates/gameplay/tests/realistic_waist.rs` (new), `crates/avatar/tests/realistic_heavy_muscular.rs` (new), `realistic_ranges.rs` (the stylized table; a table ends at any heading), `realistic_parameters.rs` (180 shapes), `crates/gameplay/src/description_tests.rs` (two tests set `height` 1.2 and expected 0.5 as the clamp on the stylized body; now 1.1 and 0.85).
- Docs: `refinement/r3/ranges.md` (the waist, the stylized family, both tables, `ranges/ranges.json` and `ranges/ranges-stylized.json`), `testing.md`, `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md`, this file, the red run, the sheets.

## Interfaces

No schema or contract change. The realistic manifest gains six corners (`<Frame>_Waist_Narrow`, `_Waist_Wide`, `_Fat_Over_Muscle`) and `waist_width` 0.8 to 1.175; the underwear items follow the new shapes. The stylized manifest's proportion ranges and attribute drivers change (no new field). `measure.py --check a.json,b.json`.

## Tests

Red first, against the assets as they were, with the tests in their final form: [r3-body-2-red-run.txt](r3-body-2-red-run.txt) (`14 tests run: 7 passed, 7 failed`: no stylized table, no waist shapes, the waist at 0.251 of the slider, the thigh 4.1 mm past the hips, the definition 9.3 against 5.7 mm, 174 shapes).

Green:

| Command | Result |
|---|---|
| the same 4 binaries | `14 tests run: 14 passed` |
| `cargo nextest run -p axcenity-avatar -p axcenity-anim -p axcenity-project -p axcenity-gameplay --no-fail-fast` | `563 tests run: 563 passed, 1 skipped` (after updating `gameplay`'s two description tests: before that, 561 passed and those 2 failed on the stylized `height` range) |
| `cargo nextest run -p axcenity-player --no-fail-fast -E 'test(description) \| test(humanoid)'` | `27 tests run: 27 passed` (after updating three description tests that set the stylized `height` to 1.2 and 0.8 and `head_size` to 0.9; before, 24 passed, 3 failed) |
| `cargo nextest run -p axcenity-player --no-fail-fast` (all) | `189 tests run: 188 passed, 1 timed out` (`skin_value_tests::skin_tone_values_change_the_picture_the_way_they_mean`, R5's GPU test, at 120 s under seven builds' load; alone it passes in 90 s) |
| `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(character)'` | `76 tests run: 76 passed` |
| `cargo clippy -p axcenity-avatar -p axcenity-gameplay -p axcenity-anim --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | applied |
| `measure.py --package realistic --only files,weights,clipping,face,shapes,sliders,proportions,deformation,twist --check assets/characters/realistic/qa/thresholds.json` | **`check: passed (2 targets not met)`**, 2,968 s (run with the sliders pass's bones placed as before, `set_body_sliders`): 3,828 pairs and 15,312 bodies a frame, 0 bodies with new crossings, 0 alone, 0 eye or teeth in flesh, on both frames; the 2 targets not met are the 90 degree shoulder's local volume (0.847, 0.88), as before. Nothing loosened; the realistic `thresholds.json` is untouched. Output: [r3-body-2/measure-realistic-check.txt](r3-body-2/measure-realistic-check.txt) |
| `measure.py --package realistic --only sliders` with the bones placed exactly as `anim` does (the final `measure.py`) | 0 bodies with new crossings, 0 alone, 0 eye or teeth in flesh on both frames (3,828 pairs, 15,312 bodies a frame), the same as with the old placement; the body-only counts are 0 too. (`--check` with `--only sliders` reports the other sections as not measured, so only the sliders lines are read.) |
| `measure.py --package stylized --only (the same) --check assets/characters/humanoid/qa/thresholds.json,assets/characters/humanoid/qa/body-thresholds.json` | **`check: passed`**, 404 s: the body's own sliders 0 bodies and 0 alone with new crossings on both frames (1,625 bodies a frame in all; the 110 masculine and 5 feminine that cross are face parameters, below); the clothing limits as before; every joint at its floor. Output: [r3-body-2/measure-stylized-check.txt](r3-body-2/measure-stylized-check.txt), [r3-body-2/measure-stylized.json](r3-body-2/measure-stylized.json). Before: 662 and 481 bodies, `build` 1 alone 481 and 620 pairs |
| `ranges.py` (realistic, all nine) and `ranges.py --package stylized` | the tables in ranges.md |
| `python3 -I scripts/check-wiki.py` | `113 pages, 0 errors, 58 warnings` (the existing staleness notes) |

## Visual evidence and what I saw

All in [r3-body-2/](r3-body-2/) (about 1.9 MB). "Before" is this branch's start (b39a9ed7's assets), captured with the same tool and settings.

- **`engine-waist-close-<frame>.jpg`** (1:1 crops; before at 0.775, 1, 1.225, after at 0.8, 1, 1.175; front then side; studio then hard) and `engine-waist-<frame>.jpg` (whole figures).
  - Before, the three waists are hard to tell apart at 1:1 in front and identical from the side.
  - After, feminine: 0.8 is a clear hourglass, the narrowing centred above the navel and running out smoothly into the ribs and the hip crest; 1.175 is a straight, fuller waist. No step, crease or shelf where the change fades, under either light; the briefs' and the bra's bands follow.
  - After, masculine: 0.8 tapers the flanks toward the waist, 1.175 straightens them and the belly comes a little forward in the side view; the boxers' waistband follows. The change is quieter in the side view (depth 0.3 of the slider), as intended.
- **`engine-heavy-muscular-<frame>.jpg`** (heavy, then heavy and muscular before and after; studio front, side, three-quarter, hard three-quarter).
  - Masculine before: a bodybuilder: separate deltoid heads, peaked biceps, a ridged trapezius, quads with a sweep. After: a big, strong, heavy man, thick through the shoulders, arms and neck with the muscle rounded under fat, a belly like the heavy body's; the deltoid cap still shows as a shape under the hard light, without the separations.
  - Feminine before: muscular arms with visible heads, a narrow pelvis and thighs flaring out below the briefs (the jodhpur outline). After: full hips and thighs tapering smoothly to the knee from the front and the side, thick arms; the forearm still shows some muscle under the hard light.
- **`blender-heavy-muscular-<frame>.jpg`** (clay under one lamp from the side; front, three-quarter, side, back; heavy, heavy and muscular before and after): the same, without the skin's maps. The back views lose the bodybuilder's trapezius and lats ridges; no dent or flat patch where the muscle was smoothed.
- **`blender-stylized-kept-apart.jpg`** (clay; stylized masculine heavy and feminine heavy and muscular, before and after): the thighs no longer meet at the crotch, the neck is a little slimmer under the chin, the belly and seat are the same.
- **`engine-stylized-heavy-after.jpg`**: the stylized heavy and heavy and muscular bodies in the engine after the change; nothing broken, the outfit-less body reads as before but with a gap between the thighs.

## Dependencies

None added.

## Open issues

1. **The stylized hips can't narrow and its head barely scales** (1 to 1.15; 0.975 to 1.025). The bone crosses the heavy inner thighs, and the heavy build's chin meets the neck as the head grows. A narrowing shape like the realistic body's, and keeping the build apart across the head's scale, would widen both; that is shape work on the stylized body, past "don't redesign".
2. **The stylized elbow and knee** keep 0.37 to 0.45 of their ring area at 135 degrees (the realistic body 0.6 or more): no twist or helper bones. Held where they are.
3. **The stylized face** isn't checked by these body checks: its eye measure counts its own lids (541 and 410 vertices at rest), and `eyes_size` 1 and `ears_size` 1 cross the head's own triangles (16 and 10 pairs). `face_checks.py` covers the realistic face only.
4. **One range for both frames** still (wave 2's item 5).
5. **The waist's ends** are set by the outline check's 0.05 at 3 cm; a wider fade would reach the population's 0.79 to 1.19 but would reach into the hips and chest more.
6. `benchmarks/make.py` writes `stylized-*-body-narrowest-hips` at `hip_width` 1 now (the range's low end), the same as the default body.
7. The Blender clay close-ups of the waist under a side lamp were not made; the engine close-ups were judged instead.
8. **The stylized underwear shows skin through it in blotches** on the heavy body in the engine (the sports top and briefs, front and side). It is there before this change too (captured from `main`'s parts file): the garments' clearance, not the body; RA4's or the stylized clothing's.

## What the lead must do at merge

- Rebuild the generated files on the merged sources: `build.py` (realistic, with the bpy Python), `presets.py`, `catalogue.py`, `benchmarks/make.py`. If another branch changes `shapes.py` or the body, rerun `ranges.py` (about 7 minutes) and check the waist's ends. I regenerated and committed `RealisticParts.glb`, `Realistic.base.json`, `Realistic.build.json`, the three realistic underwear items, `HumanoidParts.glb`, `Humanoid.base.json` and 12 stylized benchmarks; presets and the catalogue came out with line-ending changes only (not committed).
- **The stylized package was rebuilt** (`humanoid/build.py`) for the kept-apart build shapes. I committed only `HumanoidParts.glb` and `Humanoid.base.json`: the rebuild also rewrites `Humanoid.glb` (the default character, which carries no build shape) with float noise in its JSON (about 27 kB of digits; the same happens with no change to the scripts), so I kept `main`'s. The examples' copy needs nothing.
- The morph count in `realistic_parameters.rs` (180 here) adds to whatever another branch adds.
- Write the progress entry (progress.md is the lead's).
