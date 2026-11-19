# Wave 5 stage 3 hand-back: a fit that keeps a loose garment's drape, and a check for cloth-only defects (CO9 part 4)

Branch `fit-drape`, from `refinement-wave5`. Task: the cloth-defect check first, then a drape-preserving fit (the intake's conform by cloth simulation, the engine fit keeping ease as the body changes), proven on the licensed tee's source and the test tee, with sheets.

**In one paragraph.** The new check (`player` `cloth_checks`) finds what the fit gate can't: triangles **stretched** past 1.5 times their rest length (the most power-stretch knit takes), triangles **folded** over against the way the bones carry them, and cloth **hanging** off the body (outside the hull of the nearest body part or joint by more than the item's declared ease, or a lowest hem pulled down into a point). It finds every cloth defect the `tee-licensed.md` hand-back listed on the stage-2 tee (the T-pose underarm strips, the arms-raised flaps, the arms-forward spike, the centre-back hem tab), and holds every garment the gate fits to exact records (172 of them). The intake now **grades** a garment to the body by measured girths and **drapes** it onto the body with Blender's cloth simulation; the engine fit has an opt-in **drape** mode that moves loose cloth by the body's displacement smoothed over the garment, so ease is kept. Re-run through the intake, the licensed tee keeps its hang: **the back hem hangs level across the seat** (its centre-back point went from 36 to 7 mm below the hem's line on the masculine average body, 44 to 11 mm on the feminine), the bodysuit look in the back view is gone, the underarm strips, spike and torn armpit flaps no longer show in the captures, and the stretch the check counts fell from 149 to 25 times (T-pose), 288 to 60 (arms raised), 233 to 59 (arms forward), about 200 to 30 (walk). **It is not ready for sign-off and stays Draft:** the walk still fails the gate on 47 to 54 bodies (up to 34 mm, mostly arm skin past the sleeves' turned hems and the armpit in the swing), several of the tee's gate worsts rose (listed below for the lead's ruling), the hem's side corners stick out in small points, the hem rides up over the stepping thigh in the walk, and on the feminine bodies the lower back flares. The engine drape mode made the **test tee** (not draped by simulation) worse in the walk, so it stays off by default; only draped intake items use it.

## Files changed

| File | Change |
|---|---|
| `crates/player/src/cloth_checks.rs` | New. The cloth check: `stretched`, `folded`, `hanging` (with `Hulls` and `hem_points`), the records (`ClothRecord`, `check_cloth`), four garment sets, 33 record tests and 6 synthetic tests. |
| `crates/player/src/main.rs` | `mod cloth_checks` (tests only). |
| `crates/player/src/tee_crew_tests.rs` | The licensed tee's gate records set from the re-intaken item (below); the run's pop record removed (gone); the comments say which records are the stage-2 item's history. |
| `crates/avatar/src/fit/transfer.rs` | `FitOptions::drape`: loose cloth (more than `ON_SKIN` 1 cm off the body or the layer under it, fully at `LOOSE` 3 cm) takes each carried shape's displacement smoothed over the garment (`DRAPE_ROUNDS` 48, `Welds::smooth_field`), taking in a bulge of the skin only up to its ease; loose cloth's weights smoothed `LOOSE_WEIGHT_ROUNDS` (6) rounds more (`smooth_weights` takes a per-corner share). Off by default: unchanged. |
| `crates/avatar/src/fit/import.rs`, `crates/avatar/examples/import_garment.rs`, `crates/avatar/tests/intake_placement.rs` | `ImportOptions::drape`; the example's `--drape`. |
| `crates/avatar/src/fit/tests.rs` | `a_loose_garment_keeps_its_ease_when_the_body_changes`. |
| `assets/characters/realistic/import/conform.py` | `girth`, `grade` (by height, back to 1 at the neck), `push_out` (before the simulation), `drape` (Blender cloth on the body as a collision object, pinned at the top). |
| `assets/characters/realistic/import/fit.py` | `--drape` to the engine import for a draped item; the provenance's changes list the grade and the drape. |
| `assets/characters/realistic/import/items/tee-crew.intake.json` | `conform.grade` (`at` 0.92, 1.00, 1.08, 1.16 m; `least_ease` 0.08 m) and `conform.drape` (`pin_top` 0.04 m, 60 frames, quality 8, self-collision). |
| `assets/characters/realistic/import/test_intake.py` | `DrapeTest` (3 tests). |
| `assets/characters/realistic/clothing/tee-crew/Shirt_TeeCrew.*` | The re-intaken tee (model, binding, thumbnail, provenance; the manifest only differs in line endings and isn't committed). Draft. |
| Docs | `asset-quality.md` §3 (the new "Cloth in motion" row), `architecture.md` §9 and §16 (`drape`), `testing.md`, `import/README.md`, `intake.py`'s table, `references/clothing/README.md` §10, `crates/avatar/CLAUDE.md`, `wiki/Roadmap/Known-Limitations.md` (the licensed tee rewritten; the cloth check's and the drape fit's limits). |
| `refinement/wave5/fit-drape-red-run.txt`, `fit-drape/*.jpg` | The red runs; the sheets. |

## 1. The cloth-defect check

Per garment, body and pose, each triangle against the same garment at rest on the same body (same indices, so identity shapes cancel):

- **Stretched**: an edge longer than `STRETCH` = 1.5 times its rest length (rest edges under 0.1 mm count as 0.1 mm, so a zero-length fold pulled open is found). The limit, written into asset-quality.md §3 as a new row: power-stretch fabric takes 30 to 50% and comfort stretch 20 to 30% (Guru, Varshney and Kumar, "Compression and Recovery Functional Application for the Sportswear Fabric", IntechOpen 2022, https://intechopen.com/chapters/79743, opened); single jersey is a moderate-stretch knit and woven cloth takes less, so past 50% no garment fabric we make holds: it is a tear.
- **Folded**: at rest the triangle's face agreed with its corners' normals (cosine at least 0.5); in the pose it has turned past perpendicular (`FOLDED` = 0) from its corners' posed normals, which the skinning turned with the bones. Measured as the angle.
- **Hanging**: farther outside the convex hull of the nearest body part (torso, head, each upper arm, forearm, hand, thigh, shin, foot, by each body corner's most-weighted bone) or joint (two parts that meet) than the item's declared ease (`EASE`: a regular tee 30 mm from FreeSewing Teagan's 18% hip ease; joggers 8 mm from Paco's 5% seat ease; cargo pants 25 mm; underwear 5 mm), at a triangle's middle or a corner, measured from the bare body posed alike. Cloth across a hollow (under the bust, the cleft) or a joint's crease (a gusset, the armpit) is inside the hull and rests on the body; a flap or a strip is not. Hull distance is a lower bound (supports along 128 spread directions and up to 384 of the body's own normals); the nearest body corner, an upper bound, skips the hulls for cloth within its ease. **And** a lowest level hem pulled down into a point: on the garment's lowest open-edge loops that lie within 25 degrees of level, each corner's lowest cloth straight below it (a turned hem's fold) more than the ease below the mean of the hem 10 cm along either side. The stage-2 tee's centre-back tab is 40 to 50 mm from the skin, within its ease, and inside the hull (drawn into the cleft), so only the hem measure finds it.

Records: per garment, pose and kind, the bodies, the triangles on all of them and the worst; exact (`check_cloth` fails more bodies, more triangles or a worse worst, and asks to lower a record with fewer bodies, fewer triangles, a worst more than the kind's slack under it, or none). Sets: the gate's test tee and underwear (54 bodies), the licensed tee (54), the realistic wardrobe's crew tee and joggers (54; the sneakers are the footwear check's), the stylized wardrobe and underwear (its 16 cast). Poses: rest, Idle, the five held poses and the walk's whole stride in thirds (on the 18 cast bodies only: two posings a frame, dressed and bare, take the whole benchmark set past the test time limit), the stylized set at rest, T-pose and Walk@18.

**It finds what the `tee-licensed.md` hand-back listed** (the stage-2 tee, run before the refit, `AXCENITY_CLOTH_DEBUG=masculine-cast-average`, grouped by place):

| Listed defect | Found as |
|---|---|
| T-pose underarm strips | Stretched: 123 triangles on the sleeves' undersides (62 right, 61 left), up to 149 times rest; 16,112 on the 54 bodies |
| Arms-raised sleeve flaps | Stretched: 181 triangles on the sleeves' undersides, up to 288 times; 32,093 on the 54 bodies |
| Arms-forward spike at the side of the armpit | Stretched: 364 triangles at the front of both armpits, up to 233 times; 35,454 on the 54 bodies |
| Torn, flipped armpit flaps (walk, arms forward) | Folded: 92 triangles at the front of the armpits with the arms forward, up to 178 degrees; walk 5,375 to 5,952 a third on the 54 bodies |
| Centre-back hem tab (rest and every pose) | Hanging (hem point): the waist hem's centre back, 39 to 55 mm below its line; 71.4 mm worst on the 54 bodies |
| Dark lip at the back of the neck | Not found: a 2 to 3 mm lip is under every limit (an early version of the hem measure flagged the neck's centre back, but only by taking the neckline for a hem; the final one looks only at a garment's lowest hems) |

Synthetic tests (red first, `fit-drape-red-run.txt`): a sheet stretched evenly 1.4 times (none) and a band pulled to twice (exactly its triangles, worst 2.0); a sheet turned over whole with its normals (none) and a row of corners pushed past the next (that row, 180 degrees); cloth resting across a 3 cm hollow of a limb (none), a corner pulled 6 cm out (only its triangles), two parts, cloth across a joint's crease (resting) and between two parts that don't meet (hanging); a level hem, the same with a corner raised, or turned upright (none), a corner pulled 5 cm down (that corner, 41 to 55 mm), a turned hem whose fold is pulled down (found at the open edge above it); the bone names to parts; the ratchet.

## 2. The drape-preserving fit

**(a) The intake.** `conform.py` now grades and drapes (see `import/README.md`):

- **Grade**: the girth round the item at each height in `grade.at` (a tape round the outside: the perimeter of the slice's convex hull from above) against the body's across the item's width there and 3 cm past it; where the item has less than `least_ease`, it is scaled across (X and Z about the body's middle there), the scale linear between the heights and back to 1 at its neck. The tee's source is much smaller than the body: placed, 1,055 of its 1,462 corners were inside the default body (median 11 mm inside). Graded with 8 cm of girth ease, the scales at 0.92, 1.00, 1.08 and 1.16 m are 1.205, 1.093, 1.041 and 1.192, the girths after 0.965, 0.929, 0.876 and 1.011 m (the hips before: 0.804 against the body's 0.889). With 12 cm the feminine bodies' lower back flared like a skirt in the captures, so 8 cm was kept (below Teagan's 18% hip ease; chosen by looking).
- **Drape**: corners still inside the body (or nearer than 4 mm) are moved out along the body's normal (869 corners, up to 40 mm, mostly the shoulders and sleeves, whose scale goes back to 1), then Blender's cloth (`../clothing/drape.py`'s settings: mass 0.15, tension and compression 25, bending 0.08, self-collision) hangs it for 60 frames on the default body as a collision object, its 65 corners within 4 cm of its top (the neck and the shoulders' tops) pinned. After: 0 corners inside, clearance least 2.3 mm, median 8.1 mm; the edges p95 1.05 times their length before the simulation, the most 4.0 (a few short edges at the pushed-out shoulders). The export fits it with `--drape`.

**(b) The engine fit.** `FitOptions::drape` (`transfer.rs`): each carried shape's offsets (the bound point's frame, as before) are, for loose cloth, mixed towards the same field smoothed over the garment (48 rounds, each half and half with the neighbours' mean), and the cloth then takes in a bulge of the skin under it only up to its ease there (its distance from the body or the layer under it, less 1 cm); past that it moves out with the skin; the push-out runs after, as before (only where skin would come through). Loose cloth's weights are smoothed 6 rounds more (the armpit band, the hem between the legs). Corners on the skin are unchanged. Synthetic test (red first): a sleeve with 4 cm of ease over a 3 cm bulge followed it all the way before (30.0 mm), 8 mm or less now, while it still moves 2 cm out with Fat and stays clear, and a sleeve on the skin still follows the bulge.

**Why opt-in.** On by default, the layered fitting test (`avatar` `fitting.rs`, a shirt over the sports bra, posed on every pose state) rose from its record 18.7 mm to 19.0 to 19.3 mm (with every ease threshold and smoothing length I tried: 16, 32, 48 rounds; ease from the body or the nearest layer; the bulge capped by the ease), and the gate's test tee fitted with `drape` was worse in the walk (every body in each third at 15.1 to 20.1 mm against records of 13.6 to 16.6; Walk@18 51 bodies at 20.1 mm against 38 at 10.3) though better with the arms forward in its worst (53 bodies at 8.8 mm against 44 at 9.4). The test tee isn't draped by simulation (its source hugs the body), so the gate's import stays as it was; only items the intake drapes use it. **For the lead:** the shape smoothing helps a garment that already hangs with ease, and costs a cloth made on the skin a little in the armpit.

**What I did not change.** The between-corners pass (`needs`) that made the stage-2 tab is unchanged: with the hem hanging free it has nothing to push there (the tab is gone without touching it). The turned-hem gate rule stays the lead's decision.

## 3. The proof

### The licensed tee (`axcenity/realistic-tee-crew`, re-run through the intake, Draft)

Command (Blender's Python; the downloads a copy of the source folder; the export into a copy of the library, then the item's files copied into the repository's):

```bash
AXCENITY_FABRIC_TILES=<tiles> python -I assets/characters/realistic/import/intake.py \
    --spec assets/characters/realistic/import/items/tee-crew.intake.json \
    --downloads <folder with bk-plain-t-shirt.glb> --work <scratch> --out <library copy>
```

Conform 4.3 s, refine 87 s, export 25.5 s. 4,080 / 2,039 / 1,020 triangles. Weights past the next joint (asset-quality §2): 1,740 corners at LOD0 (stage 2: 1,919), 1,073 and 654.

**The hem** (the gate's dumps, `AXCENITY_FIT_GATE_DUMP`, the lowest tee corner in each 2 cm strip across the back, |x| < 12 cm):

| | Back hem height | Lowest corner below the hem's line | Lowest corner from the nearest skin | Below the seat's back-most point |
|---|---|---|---|---|
| Masculine average, stage 2 | 0.791 to 0.904 m | 36 mm (the centre-back point) | 46 mm (drawn into the cleft) | 102 mm |
| Masculine average, now | 0.856 to 0.865 m | 7 mm | 29 mm | 37 mm, standing 21 to 46 mm behind the body |
| Feminine average, stage 2 | 0.783 to 0.901 m | 44 mm | 37 mm | 63 mm |
| Feminine average, now | 0.847 to 0.926 m (the sides rise with the hips) | 11 mm | 30 mm | 1 mm, standing 17 mm or more behind the body |

No corner is drawn under the buttocks: the back hem hangs straight across the seat. The front hem is level within 9 mm (0.859 to 0.868 m on the masculine average).

**The gate** (`tee_crew_tests`; bodies failing, worst; stage 2 → now). Fewer bodies almost everywhere; **worsts that rose are marked ↑** and the records were set from the new item's first run (it is a new item, not a pass of the old one): the lead rules on them.

| Pose | The tee | Bra through it | Briefs through it | Boxers through it |
|---|---|---|---|---|
| Rest | 7, 28.2 → 3, 27.7 | none → 1, 6.9 ↑ | 11, 17.9 → 1, 18.3 ↑ (0.4 mm) | 1, 15.1 → none |
| Idle | 53, 38.0 → 40, 27.4 | 17, 4.1 → 2, 7.6 ↑ | 11, 18.0 → 1, 18.3 | 1, 14.5 → none |
| T-pose | 7, 28.2 → 3, 33.0 ↑ | none | 11, 17.9 → 1, 18.3 | 1, 15.1 → none |
| Elbows bent | 8, 28.2 → 2, 36.5 ↑ | none → 1, 6.9 ↑ | 11 → 1 | 1 → none |
| Knees bent | 7, 28.2 → 3, 27.7 | none → 1, 6.9 ↑ | 11 → 1 | 1 → none |
| Arms raised | 14, 28.2 → 19, 34.7 ↑ | 1, 3.1 → 18, 6.9 ↑ | 11 → 1 | 1 → none |
| Arms forward | 54, 30.2 → 54, 34.2 ↑ | 19, 3.2 → 21, 10.7 ↑ | 3, 17.3 → 4, 18.3 ↑ | 1 → none |
| Walk, first third | 54, 32.5 → 54, 34.2 ↑ | 17, 5.7 → 5, 8.1 ↑ | 12, 18.5 → 4, 24.0 ↑ | 2 → none |
| Walk, second third | 54, 31.3 → 47, 33.4 ↑ | 20, 9.2 → 5, 6.8 | 13, 22.7 → 4, 13.8 | 26, 2.4 → 18, 1.7 |
| Walk, last third | 54, 32.0 → 53, 34.2 ↑ | 10, 7.4 → 5, 8.1 ↑ | 12, 18.3 → 4, 24.1 ↑ | 2, 14.2 → 1, 2.1 |
| LOD1 / LOD2 at rest | 10, 12.0 / 54, 30.7 → none / 8, 6.7 | | | |
| Run (motion) | pop 6.1 mm → none | | | |

The tee's worst cases are the arm's skin just past a sleeve's turned hem (the masculine athletic body: 33 mm in the T-pose under the left sleeve's end, 34 mm in the walk with the arm swung forward; the masculine slight and slim bodies at rest 20 to 32 mm under the sleeve's end): the gate counts skin just past a turned hem's fold as covered and out (`tee-licensed.md` open issue 3). The bra comes through at the neck's side (heavy-soft feminine, 7 mm) and the front of the armpit with the arms forward (11 mm).

**The cloth check** (the licensed tee; bodies, triangles, worst; stage 2 on all 54 bodies → now; the walk now on the 18 cast bodies):

| Pose | Stretched | Folded | Hanging |
|---|---|---|---|
| Rest | | | 54, 2,094, 71.4 mm → 54, 3,603, 48.5 mm |
| Idle | 54, 9,942, 136 × → 54, 4,966, 19.7 × | 54, 5,166 → 54, 3,154 | 53, 2,068, 72 mm → 54, 3,745, 51 mm |
| T-pose | 54, 16,112, 149 × → 54, 6,685, 25.2 × | 54, 2,697 → 54, 2,479 | 54, 1,762, 71 mm → 54, 3,399, 48 mm |
| Arms raised | 54, 32,093, 288 × → 54, 25,387, 59.9 × | 54, 5,028 → 54, 4,723 | 52, 1,659 → 54, 3,823, 48 mm |
| Arms forward | 54, 35,454, 233 × → 54, 32,446, 59.2 × | 54, 7,635 → 54, 5,588 | 48, 771, 66 mm → 54, 4,950, 50 mm |
| Elbows bent | none → 22, 50, 1.76 × | none → 1, 1 | 54, 1,964 → 54, 3,571, 48 mm |
| Knees bent | | | 54, 2,052, 71 mm → 16, 222, 36 mm |
| Walk (a third) | 54, ~20,000, 165 to 202 × → 18, ~5,200, 27 to 30 × | 54, ~5,500 → 18, ~1,550 | 54, ~2,300, 70 to 73 mm → 18, 463 to 1,149, 64 to 65 mm |

More triangles hang at rest and in the held poses: the back hem now stands 30 to 49 mm out from the body's hull behind the seat, past the tee's declared 30 mm. Whether that ease belongs in the declaration (a tee hung from the shoulder blades stands off a flat seat) is for the lead; I didn't change the declaration.

### The test tee (the gate's, `avatar`'s fixture)

Fitted as before (no drape: see "Why opt-in"). Its records in `fit_gate_tests.rs` are unchanged; its cloth records are new (`records::GATE_*`): e.g. the T-pose 54 bodies, 16,859 triangles stretched up to 43.9 times; the walk folded about 5,500 a third; hanging at rest 14 bodies, 39.6 mm (its back hem points and the shoulder blades on the heavy bodies).

### Every other garment

The fit gate's, the wardrobe's and the stylized records are unchanged (`fit_gate_tests`, `wardrobe_tests`, `garment_checks` pass as they were); nothing else was refitted. Their cloth records are new: the wave-4 crew tee (stretched up to 96 times in the walk on the cast, 178 times with the arms raised; hanging up to 86 mm at rest), the joggers (hanging on every body in every pose, up to 77 mm: much of the leg stands 25 to 37 mm off it, against Paco's 8 mm seat ease, so they are baggier than their own pattern), the realistic underwear (stretched up to 4 times (briefs) and 10.6 times (boxer briefs) in the walk at the crotch and hips, where the skin itself stretches; the feminine briefs pulled into a point at the waistband on the heavy-soft body, 69 mm), the stylized tee, cargo pants, briefs and sports top.

### The walk

Better than stage 2 in the captures (no torn armpit flaps at Walk@0.3, the back clean, no bodysuit look), but the gate still fails it on 47 to 54 bodies a third, up to 34 mm, and the check counts thousands of stretched and folded triangles at the armpit in the swing. I didn't chase it: the body-hiding and pose-correctives tracks are on it.

## 4. Visual evidence and what I saw

Sheets in `fit-drape/` (studio light, 1920 × 1080 captures cropped; the masculine average and heavyset, the feminine average and slim):

- `tee-rest.jpg`: before and after at rest, front, side, back. **Before**: the back hem drawn into the cleft in a point on all four (a bodysuit from behind); the side view shows the hem's back tucked under the seat. **After**: the back hem straight across the seat on all four; the sides hang straight; the front reads as a T-shirt with ease. **Defects after**: the feminine bodies' lower back flares into an A-line (the hem about 3 cm wider than the hips each side); the masculine heavyset's back panel has soft lumps (from the drape starting with cloth pushed out of the body); small points at the hem's side corners on all four.
- `tee-walk.jpg`: Walk@0.3. **Before**: torn flaps at the front of the armpit on all four (the side view), the hem tucked between the legs at the back. **After**: the armpit is clean in this frame; the back hangs over the seat. **Defects after**: the hem rides up diagonally over the stepping (left) thigh at the front on all four (the hem takes the thigh's weights); pointed corners at the hem's sides, standing out about 2 to 3 cm in the back view on the masculine average and both feminine bodies.
- `crops-rest.jpg` and `crops-walk.jpg`: the hem from behind, the back, the armpit from the side and the hem from the front, before and after, on both average bodies. At rest after: the hem level at the back with small pointed side corners (masculine) and the A-line flare with pointed corners (feminine); the back clean (no neck lip seen at this size); the armpit clean, the sleeve's end a thin pale edge. In the walk after: the hem's left side corner sticks out in a point 2 to 3 cm (both bodies); the front hem curves up over the stepping thigh; the armpit and back clean.

The tee reads as a T-shirt with ease at rest from the front and the side; from behind on the feminine bodies the flare and the side points keep it from natural. Not ready for sign-off.

## 5. References (decision 14)

Looked at while judging the hang (Openverse, 2026-11-18; links in `references/clothing/README.md` §10; the pictures are in my working folder outside the repository, never in a texture; the shirts' prints are not modelled):

| Openverse page | Source page | Author | Licence |
|---|---|---|---|
| https://openverse.org/image/91dc9a0f-7fb3-44f7-ad7a-5ef33e53fdb2 | https://www.flickr.com/photos/71401718@N00/6450856615 | Wonderlane | CC0 1.0 |
| https://openverse.org/image/35bbe976-93f9-42af-a756-b5b7d9f2ff82 | https://www.flickr.com/photos/71401718@N00/6450855363 | Wonderlane | CC0 1.0 |
| https://openverse.org/image/7d2df657-4a29-4164-bd44-5469bcd60d75 | https://www.flickr.com/photos/128305985@N02/20876909820 | Optimaz | CC BY 2.0 |
| https://openverse.org/image/a6f72d52-3d83-4151-b3e8-ca74798546ac | https://www.flickr.com/photos/10361931@N06/4513522387 | Horia Varlan | CC BY 2.0 |

Against them: a loose tee's hem hangs level and free at the front and sides (ours now does), the sleeves stand off the upper arm (ours do at rest), and the cloth under a raised arm bunches into soft folds rather than strips (ours still folds over in the check).

## Interfaces

- `axcenity_avatar::fit::FitOptions { drape: bool, .. }` (new field, default false), `ImportOptions { keep_placement, drape }` (new field; `ImportOptions` derives `Default`), `import_garment --drape`. No schema, manifest or file format changed; the binding and the model are as before.
- The intake spec's `conform.grade` `{at, least_ease}` and `conform.drape` `{pin_top, start_clearance, settings}` (optional; specs without them run as before).
- `player` test helpers: `cloth_checks::{stretched, folded, hanging, hem_points, Hulls, check_cloth, ClothRecord, rec, Kind, EASE, STRETCH, FOLDED}`.

## Tests run

| Command | Result |
|---|---|
| `cargo nextest run -p axcenity-player -E 'test(/cloth_checks::synthetic/)'` (red, stubs) | 5 run, 2 passed, 3 failed (the red); then the hem point's red: 6 run, 5 passed, 1 failed |
| the same, after | 6 passed |
| `cargo nextest run -p axcenity-avatar -E 'test(/fit::tests::(a_loose|carried_shapes)/)'` (red) | 2 run, 1 passed, 1 failed ("followed the bump 0.030000001") |
| `python -I assets/characters/realistic/import/test_intake.py DrapeTest` (red) | 3 errors (no `girth`, `grade`, `drape`) |
| `python -I assets/characters/realistic/import/test_intake.py` | Ran 33 tests, OK |
| `cargo nextest run -p axcenity-player -p axcenity-avatar --no-fail-fast --test-threads 8` | 512 run: 506 passed, 5 failed, 1 timed out (all `makeup_tests` and `skin_value_tests`, GPU pictures under the load of five worktrees) |
| `cargo nextest run -p axcenity-player -E 'test(/makeup_tests::/) | test(/skin_value_tests::/)' --test-threads 2` | 19 run: 19 passed (the failures above were load) |
| `cargo nextest run -p axcenity-player -E 'test(/cloth_checks::/) | test(/tee_crew_tests::/) | test(/fit_gate_tests::/) | test(/wardrobe_tests::/)' --test-threads 6` (final) | 84 run: 84 passed |
| `cargo clippy -p axcenity-player -p axcenity-avatar --all-targets -- -D warnings` | clean |
| `cargo fmt --all`; `python -I scripts/check-wiki.py` | done; 115 pages, 0 errors, 57 warnings (as before) |

## Dependencies

None added.

## Open issues

1. **The tee's gate worsts that rose** (table above, ↑): the arm's skin past the sleeves' turned hems (up to 36.5 mm, elbows bent; 33 mm T-pose; 34 mm walk and arms forward) and the bra at the neck's side and the armpit. Set from the new item's first run; the lead rules.
2. **The hem's side corners** stick out in small points at rest and in the walk (all four cast bodies in the sheets), and **the hem rides up over the stepping thigh** in the walk: the hem's corners take the thigh's weights. Not counted as a hem point (they rise, not drop) nor reliably as hanging.
3. **The feminine lower back flares** (the shape displacements smoothed over the loose hem move it out with the hips); **soft lumps on the back panel** of the heavier bodies (the drape starts from cloth pushed out of the body at the shoulders and sleeves, whose grade goes back to 1 at the neck).
4. **`drape` is opt-in**: on by default it worsens the layered fitting test and the test tee's walk (section 2). A drape run per frame (the feminine frame, the heavy bodies) instead of one on the default body would be the next step.
5. **The hanging measure's declared ease** lives in the check (`EASE`), not in the item's manifest (that would be a contract change: an optional `fit.ease`); the tee's 30 mm is less than the draped back hem's stand-off behind the seat.
6. **The cloth check's walk** runs on the cast only (18 bodies), for time.
7. The dark lip at the back of the neck (stage 2) didn't show at the sheets' size; I didn't crop the neck at 4K.

## Merge notes

- New test module `crates/player/src/cloth_checks.rs` and `mod cloth_checks` in `main.rs`. `fit_gate_tests.rs` is unchanged (its helpers were already `pub(super)`).
- The licensed tee's files are regenerated by the intake (command above); `Shirt_TeeCrew.item.json` differs only in line endings and isn't committed. `catalogue.json` is unchanged.
- If the body-hiding or pose-correctives branches change the realistic body (mask shapes, correctives) or the tee, the cloth records move: each run prints `measured: rec(...)` lines ready to paste; records only go down.
- The intake's `conform.drape` imports `../clothing/drape.py` (the wave-4 cloth settings).
