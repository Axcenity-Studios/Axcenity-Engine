# R7 wardrobe, first items: hand-back (RA4 content, wave 4)

Branch `r7-wardrobe-first`, from `co7-hair-cloth` with `co9-fit-2` merged (clean merge, no conflicts). Task: the garment pipeline and the first realistic top, bottom and pair of shoes (plan section 1's CO10 row, 11.5 R7; asset-quality.md §1 to §3, §5), one of each, to Paul's standard, then stop for his sign-off.

**In one paragraph.** The pipeline is built and makes all three items end to end: FreeSewing's Teagan and Paco (on Titan) ported to Python and drafted at the realistic body's measurements, the pieces gridded as quads whose texture coordinates are the pattern, sewn and draped by Blender's cloth simulation, finished with turned hems and rib bands (no single-layer edge), baked (the drape's folds, seam valleys, stitch rows, CC0 fabric detail) and fitted to the realistic base by the engine's own import, at three levels of detail each. The shoes are a shell over the foot with a cemented sole. **All three are drafts and none is close to production.** They read as a white tee, light grey joggers and white sneakers in the engine, and the tee's drape is the best thing here, but the fit gate finds skin out of the joggers under the cuffs and out of the tee round the waist at rest on almost every body, the joggers' drawcord through the tee's hem, the underwear through both, the simpler copies cutting into the skin, the tee's armpit jolting in motion, the shoes letting the heel and toes out in the walk with a tread that tilts, and texture coordinates far from §2's limits on most materials. Every number is a record in the tests, which can only go down. There is no outfit preset (drafts never appear in one), and Randomize now skips declared drafts (a fix, with a test). Which bottom: **joggers**, not jeans (below).

## Files

| File | What |
|---|---|
| `assets/characters/realistic/clothing/freesewing.py` (new) | FreeSewing's core points, paths and utilities in Python (MIT notice kept) |
| `.../clothing/teagan.py`, `paco.py` (new) | The drafts: Teagan on Brian's base and sleevecap; Paco on Titan's back and front (cross-seam fitting, inseam and outseam matching), its waistband and cuff |
| `.../clothing/body.py` (new) | The realistic body measured as a tailor would (FreeSewing's measurements per frame), and the landmarks |
| `.../clothing/panels.py` (new) | Pattern pieces as Coons-patch quad grids in pattern millimetres; finding a coarse corner on the fine grid |
| `.../clothing/drape.py` (new) | Sewing springs and Blender's cloth simulation on the body; seam joins |
| `.../clothing/tee.py`, `joggers.py` (new) | Each garment's pieces, seams and starting placement (isometric along the rows) |
| `.../clothing/finish.py` (new) | Levels of detail carried from the drape; turned hems, bands, band clearance; atlas packing; the OBJ for the import |
| `.../clothing/textures.py` (new) | The bake (colour, normal, occlusion-roughness-metal) |
| `.../clothing/build_tee.py`, `build_joggers.py`, `build_sneakers.py` (new) | The three builds (Blender's Python) |
| `.../clothing/wardrobe.py` (new) | The imports (`import_garment`) in wearing order, assembly of the model (three levels, maps, cloth materials), the manifests |
| `.../clothing/fabrics.py`, `README.md` (new) | Fetching the CC0 tiles; the pipeline's documentation |
| `.../clothing/crew-tee/`, `joggers/`, `sneakers/` (new, generated) | Each item's `.glb`, binding, thumbnail and manifest |
| `assets/characters/catalogue.json` | Regenerated (`catalogue.py`): the three items listed |
| `crates/avatar/src/randomize.rs` | **Fix**: Randomize never draws an item that says `quality: "draft"` (asset-quality §5 wasn't enforced at all) |
| `crates/avatar/src/tests/randomizing.rs` | `a_declared_draft_is_never_drawn` |
| `crates/avatar/tests/realistic_wardrobe.rs` (new) | Seven tests on the shipped items (below) |
| `crates/player/src/wardrobe_tests.rs` (new), `main.rs` | Twelve tests: the fit gate in seven poses, the levels' clearance, motion, footwear |
| `crates/player/src/fit_gate_tests.rs` | `posed_at` (a level of detail), the wardrobe's layered pairs in `LAYERS`, the gate's helpers and record types visible to the wardrobe tests (no rule changed) |
| `crates/player/src/garment_checks.rs` | `check_motion` visible to the wardrobe tests |
| Docs | `docs/development/character-studio/architecture.md` (the `quality` row, Randomize's items), `testing.md`, `references/README.md` (the sources and licences, the shipped-assets table), `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md`; this hand-back, `r7-wardrobe-first-red-run.txt`, `r7-wardrobe-first/` (sheets and the sign-off lines) |

**Regenerated:** `catalogue.json` (by `catalogue.py`) and the three item folders (by the builds and `wardrobe.py`). Not touched: `RealisticParts.glb`, `Realistic.base.json`, `Realistic.build.json`, presets, benchmarks, `face_checks.json`.

## What was built

1. **Measurements** (`body.py`): neck 400, chest 1016, waist 841, hips 880, seat 983, biceps 348, knee 365, ankle 215, heel 320 (1.3 times the foot's 246 mm length: a girth round the heel and instep isn't a section of a mesh), shoulder to shoulder 358 (across the back, the acromion taken 3 cm past where the A-posed deltoid falls away), shoulder slope 15.5 degrees, HPS to waist (back) 410, waist to armpit 173, to hips 92, to seat 247, to upper leg 277, to knee 574, to floor 1073, the cross seam 317 front and 348 back (mm, masculine frame; the feminine frame's are measured too, `body.py --body`).
2. **Drafts.** Teagan and Paco with their defaults, and these options inside each design's range: the tee 40% longer (Teagan's 15% ends at this body's short hip line), its shallowest and narrowest neckline (a true crew neck, so the body's torso region can be hidden under it); the joggers at a 50% waist height, a 3.5% waistband, 11% shorter (the cuff over the ankle bone, above the shoe), and 35% heel ease (Paco's leg is the heel girth plus ease; at 5% it was narrower than this body's knee, a legging).
3. **Drape.** The lesson of the session: Blender's cloth takes its springs' rest lengths from the starting shape, so the pieces must start as long as they were cut. Squeezing the shoulders in at the start baked 20% stretch into 16% of the edges; a rest shape key (`rest_shape_key`) changed nothing; a base mesh unbending to the flat pieces (`use_dynamic_mesh`) dragged the sleeves into the armholes. Starting the pieces rolled round an ellipse and tubes at their own widths, straight up over the shoulders, the seams pull them together: 4.3% of the fine edges stretched over 10% (95th percentile 1.08). Seam gaps left by the sim: the tee up to 6.8 mm (mean 0.9), the joggers up to 22.2 mm (mean 0.8; joined at their middle after).
4. **Finishing.** Levels of detail: the tee 5,512 / 2,120 / 840 triangles, the joggers 4,140 / 1,672 / 848, the sneakers 3,376 / 1,936 / 1,160 (a pair), each level within budget (6k, 6k, 4k) and at most 60% of the one before. Turned hems 20 to 22 mm deep with a 3 mm rounded fold; the tee's rib neck band 17 mm, the joggers' waistband casing 38 mm and cuffs 56 mm, each folded and carried 8 to 12 mm inside past the seam; bands moved clear of the body column by column (up to 13 mm on the tee, 21 mm on the joggers). The drawcord: two 2.5 mm cords out of grommets at the front, 24 cm long, with aglets. The tee hides `torso` and `upper_arms_top`; the joggers hide `hips`, `thighs_top`, `thighs_low` and `shins_top`; the shoes hide nothing (the `feet` region reaches above the collar).
5. **Maps** (2048 px): 17.3 texels per mm on the tee, 14.1 on the joggers, 25.0 on the shoes. Fabrics: Poly Haven's Cotton Jersey (tee; shoes' lining, tongue, laces), Jogging Melange (joggers), Leather White (upper, midsole grain), ambientCG's Rubber004 (outsole), each tiled at its photographed size along the grain, colour taken out so a tint gives the colour. Cloth materials carry `KHR_materials_sheen` (jersey 0.22, fleece 0.35, rib 0.3); leather, foam, rubber and the aglets are Standard.
6. **Manifests.** Built-in tier, layer 1, fit by binding (margin 2 mm), colour roles (tee: shirt primary, neck band secondary; joggers: fleece primary, bands secondary, cord accent; shoes: leather primary, lining and tongue secondary, laces and outsole accent), variants (tee 6, joggers 5, shoes 4), `quality: "draft"`, `source` naming FreeSewing (MIT) and the CC0 fabrics with a link.
7. **Randomize** (`avatar::randomize`, the bug fix): an item that says `quality: "draft"` is never drawn. Items made before the standard, with `quality` unset, are still drawn: the standard calls unset a draft, but leaving them out would empty every stylized random character's outfit, so that is the lead's call.

### Why joggers, not jeans

The references' first-wave order (references/clothing §5.3) puts joggers before jeans, and binding suits them: soft fleece gathered at the waist and ankles stays near the body and moves with it, and its cuff sits clear of a shoe. Jeans are stiff denim that stacks at the shoe and keeps its own shape through a bend, which is what ADR-0035's cloth-proxy fallback is for. Joggers also prove the waist against the tee's hem, the layering this wave had to show.

## Interfaces

- `avatar::randomize` skips declared drafts (behaviour; no signature changed).
- `player` (tests only): `fit_gate_tests::{posed_at, measure, check, pose_name, Known, Failure}` and `Cover.body` are `pub(super)`; `LAYERS` has 7 pairs; `garment_checks::check_motion` is `pub(super)`.
- No manifest schema, no shared contract and no dependency changed. The pipeline's scripts need Blender's Python (bpy 5.x, NumPy, Pillow), as the realistic build does.

## Tests (red first: `r7-wardrobe-first-red-run.txt`)

**Red:** every new test failed for the reason expected (the library had none of the items). Three motion tests passed at first, because nothing required the cast to wear the items; they now assert it and failed on the rerun. `a_declared_draft_is_never_drawn` was written with the fix; the red evidence for the behaviour is `drafts_are_in_no_preset_and_never_drawn_at_random` failing on the first build of the items ("Randomize drew the draft").

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-avatar --test realistic_wardrobe` | 7 passed (records hold the drafts' UV and fold defects) |
| `cargo nextest run -p axcenity-player -E 'test(/wardrobe_tests::/)'` | 12 passed (records hold the gate, level, motion and footwear defects) |
| `cargo nextest run -p axcenity-avatar -E 'test(/randomiz|preset/)'` | 22 passed |
| `cargo nextest run -p axcenity-anim -p axcenity-avatar -p axcenity-player -p axcenity-assets -p axcenity-gameplay -p axcenity-project --no-fail-fast` | **1084 tests run: 1084 passed** (22 slow), 21 skipped; no ABORT lines |
| `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `python3 -I scripts/check-wiki.py` | 114 pages, 0 errors |

`realistic_wardrobe.rs`: shipped as fitted built-in items with a source; cloth with sheen and 2K maps, colour roles, three or more variants; budgets and three levels of the file's own; a sound mesh (no degenerate or flipped face, normalized weights, no edge in three triangles); texture coordinates even, dense and not overlapping; no single-layer edge (every raw edge 1.8 to 25 mm inside a shell); drafts in no preset and never drawn at random. `wardrobe_tests.rs`: the fit gate on 54 bodies at rest, in Idle, the walk frame and the four held poses, with the layered pairs; the simpler copies' clearance; motion in Idle, Walk and Run on the 18-body cast; footwear on the cast (the 54 bodies would pass the test runner's two minutes: each pose is applied twice).

## Measurements: what the drafts fail

**Fit gate** (bodies failing of 54, worst; the same in every held pose unless noted):

| Case | Rest | Idle | Walk@18 | Arms raised | T-pose |
|---|---|---|---|---|---|
| Joggers (skin out under the cuff, back of the ankle) | 53, 31.0 mm | 53, 31.0 | 51, 22.1 | 53, 31.0 | 53, 31.0 |
| Tee (round the waist over the waistband; armpit in poses) | 38, 28.1 mm | 54, 42.1 | 54, 34.4 | 54, 37.1 | 54, 24.6 |
| Joggers through the tee (drawcord, waistband) | 54, 18.4 mm | 54, 22.5 | 54, 17.1 | 54, 18.4 | 54, 18.4 |
| Boxer briefs through the joggers | 33, 8.9 mm | 32, 9.3 | 33, 8.8 | 33, 8.9 | 33, 8.9 |
| Briefs through the joggers | 1, 1.1 mm | 1, 1.0 | 6, 1.6 | 1, 1.1 | 1, 1.1 |
| Bra through the tee | 21, 5.4 mm | 21, 15.8 | 21, 5.4 | 21, 3.7 | 21, 4.3 |

With the elbows or knees bent the numbers are the rest pose's (the worst cases are the same static ones). On the default masculine body at rest the joggers have 6 skin corners out at the back of both ankles (1.7 to 12.5 mm) and the tee 34, all between 0.89 and 0.99 m, round the waist (1.1 to 8.5 mm).

**The import's own report** (`inside_most` after its push-out): the tee 30 mm, the joggers 19 mm (23 at LOD1), the shoes 1 mm. The fit core couldn't clear the tee and joggers in some of their states: the cases above.

**Levels of detail** (at rest, simpler copy against the full body): joggers LOD1 54 bodies 30.8 mm, LOD2 54, 18.7; tee LOD1 54, 31.4, LOD2 54, 29.8. Shoes: none.

**Motion** (the 18-body cast): the tee's armpit pops 2.4 mm a frame past the skin near it in the walk, 5.1 in the run (as the test tee did, CO9 part 2); no tears; the joggers and shoes are steady in all three clips.

**Footwear** (the cast): no foot out at rest or knees bent; in the walk's stride 3 to 12 foot corners out, up to 11.3 mm (masculine), 6.0 (feminine); the tread is not flat at rest: masculine from 3.9 mm under the floor to 5.0 over it (up to 9.8 mm off flat on the heavyset body), feminine 4.0 to 5.1 mm under the floor (the fit carries the body's shapes onto the sole, so it moves with the foot).

**Texture coordinates** (share of each material's area within 10% of its median texel scale, and at 8 px/cm or more): tee jersey 51.7% / 96.5%, rib 4.9% / 100%; joggers fleece 39.8% / 96.6%, rib 36.8% / 92.8%, cord 96.0% / 99.1%, aglets 65.9% / 92.1%; shoes upper 3.4% / 79.7%, lining 4.9% / 100%, midsole 13.4% / 71.8%, outsole 69.7% / 97.8%, tongue 19.0% / 96.0%, laces 75.6% / 97.3%. No material overlaps (at most 64 of 155,161 texels drawn twice). The tee's jersey keeps 95% of its *drape* edges within 10% of the pattern; the fit's push-out over the bra and the waistband stretches it after.

**Folds:** one raw edge on the tee (at the side hem, over the waistband) is 1.4 mm inside its shell, under the 1.8 mm floor; every other open edge of the three items is 1.8 to 25 mm inside one.

**Not measured:** GPU time per character (§2 "Performance": no capture of the frame passes with the outfit was run), the feminine frame's own drape against the bound one (the feminine draft is measured, not draped), skinning against `thresholds.json`'s joint ratios (Blender's `measure.py` doesn't load an item), the asset-quality §3 compression folds (binding has none: a bent elbow or knee bends the bound cloth with the skin).

## Visual evidence (`r7-wardrobe-first/`, 5.1 MB)

In-engine captures, release build, TAA: the dressed cast (`wardrobe-<frame>-<body>`: the five bodies of each frame wearing their underwear and the three items, written to a scratch folder, not shipped).

| Sheet | What |
|---|---|
| `cast-{rest,idle,walk}-{masculine,feminine}-studio.jpg` | Each frame's five bodies (average, slim, athletic, heavyset, muscular), six views (front, three-quarter, side, back, face, face-front), studio light; at rest, one second into Idle, and the walk at 0.3 s |
| `lights-{rest,idle,walk}-{masculine,feminine}-average.jpg` | The average bodies under studio, hard, warm and outdoor light, six views |
| `suite-{masculine,feminine}-{average,heavyset}.jpg` | The body suite: rest, T-pose, arms raised, elbows bent, knees bent, the walk frame |
| `close-tee-collar-hem-sleeve-joggers-cuffs.jpg` | 3840 × 2160 crops, masculine average, front: the crew neck, the hem over the waistband, the sleeve hem, the joggers' cuffs and the shoes (drawn at LOD1, as a game draws them at that distance) |
| `close-tee-back-bust-and-bent-poses.jpg` | 3840 × 2160: the tee's back, the feminine front, knees bent (side), elbows bent (side) |
| `close-sneakers-full-detail.jpg` | 3840 × 2160, the shoes forced to full detail (`AXCENITY_CAPTURE_FULL_DETAIL`): front, three-quarter, side |
| `blender-{tee,joggers}-drape.jpg`, `blender-sneakers.jpg` | Workbench renders of the drapes and the shell before the fit (the tee is the trial drape the build's settings came from) |

**What I saw**, against references/clothing (§5.1's construction table) and §7's fit checklist:

- **The tee's drape is right in kind.** Over the back and the shoulder blades it falls in soft diagonal folds as a jersey tee does; over the feminine bust it drapes and gathers to the side seam; the sleeves sit off the arm with a few folds. The crew neck's rib band, with its wales, sits at the neck's base on both frames, but the neckline itself is wrong (next point). The sleeve hems are clean turned hems with a visible edge thickness. At arm's length it reads as a white cotton tee, consistently under the four lights; the hard light brings out the folds strongly, and in outdoor light the sides show many thin vertical folds (the fold map reads a little strong there).
- **The crew neck is faceted, with tabs at the shoulder seams** (`close-tee-collar-hem-sleeve-joggers-cuffs.jpg`, first panel; the face and face-front cells, columns 5 and 6, of every `cast-*` sheet). The neckline is a polygon of straight runs, not a curve. Measured on the shipped `.glb` (open loops, corners welded at 0.1 mm), the LOD0 neck edge has 22 corners round 537 mm: runs of 24.4 mm on average and up to 39.1 mm, where the hem has 86 corners at 12.7 mm (LOD1's neck: 12 corners, runs up to 52 mm). At both shoulder seams the body shell's edge stands up above the band as a pointed tab, and on the right one the shell's dark inside shows. Likely causes: the band and its raw edge are kept at LOD0's coarse spacing, and the shell isn't turned under the band at the shoulder seams.
- **The tee bulges out at the lower back and flares at the hips** (the side and back cells, columns 3 and 4, of `cast-rest-masculine-studio.jpg` and `cast-rest-feminine-studio.jpg` on every row, worst on the heavyset row; the back cell of every row of `suite-feminine-heavyset.jpg`). The tee's lower back stands out over the seat as a rounded bulge, like a bustle, and on the heavyset bodies the hem flares out at the hips like a peplum. It shows at full-body distance: a silhouette defect, apart from the torn hem below. Read off the side cells (the rightmost cloth pixel per row, figures 300 px tall), the back stands out past the joggers' seat line below it by 17 px on the average feminine body, 18 on the heavyset feminine, 15 on the average masculine and 14 on the heavyset masculine: roughly 80 to 100 mm. That is a reading from downscaled pictures through a perspective camera, not a mesh measurement. A tee hangs straight down from the shoulder blades and at most just clears the seat. Likely cause: the import's push-out over the joggers' seat, waistband and drawcord, stacked on the bands already moved clear of the body (up to 13 mm on the tee, 21 mm on the joggers), with the tee draped on the bare body rather than over the joggers.
- **The tee's hem over the joggers' waistband is broken** (front, side and back, every body): torn, shredded fragments of the hem and its turn-up, gaps where the waistband's rib and the skin above it show, black holes at the lower back. This is the gate's "tee round the waist" and "joggers through the tee" records made visible: the import pushed the tee out over the waistband and the drawcord, and its folded hem didn't survive it. It is the tee's worst defect and the first to fix.
- **The bra comes through the tee** on the feminine bodies: dark patches on the chest under every light and a spot at the armhole (21 bodies in the gate).
- **A hole at the side by the waist** shows skin with the elbows bent and in Idle.
- **The black patches at the joggers' crotch** (the back at rest; the front in the walk, `cast-walk-masculine-studio.jpg`) are not holes: LOD0 has only its three open loops (the waistband and two cuffs). They are the recorded boxer briefs through the joggers (33 bodies, 9 mm), as that record looks on screen.
- **The joggers read as fleece joggers**: a melange that looks right, a straight relaxed leg, rib cuffs and a waistband, folds at the knee. **The cuffs end in a sawtooth** (the bottom edge of the cuff band, moved clear of the heel column by column) and don't wrap the ankle closely; with the knees bent there are no compression folds behind the knee (binding bends the cloth with the skin).
- **The sneakers are not acceptable.** Drawn at full detail the sole's edge is crumpled, the toes show between the upper and the sole, and skin shows in gaps over the sole; at LOD1 (what a game draws at this distance) the toes come through the toe box. In Blender, before the fit, the same shoes are whole (`blender-sneakers.jpg`): the foot stands inside the sole (the body's underside is on the floor), so the import pushes the sole's top out of the foot and drags the sole's wall with it. The shape (a low-top with a padded collar, tongue and laces) reads right from a distance; up close it is broken.
- **In motion** (Idle, the walk): the clothes stay on and follow the body; the hem fault and the side hole show; the stride's back foot shows the outsole.

None of the three meets Paul's bar ("natural, judged against references"): the tee comes closest in its drape, and its hem over the waistband, its faceted neckline and its bulge at the lower back rule it out.

## Open issues

1. **Skin out of the joggers under the cuffs** (53 bodies, 31 mm): the cuff's inner layer is moved clear of the heel column by column (`finish.clear_bands`), which leaves skin between columns outside it, and the gate counts the skin under the cuff as covered. A cuff gathered round the ankle (rows pulled in, not pushed out) is the fix.
2. **The tee round the waist over the joggers' waistband** (38 bodies at rest): the hem's turn-up lies over the waistband, and the import's push-out from the waistband leaves the inner layer inside the skin in places. With the arms raised and in the walk, the armpit and sleeve end, as the test tee.
3. **The drawcord through the tee** (every body): the tee was pushed clear of the waistband but its hem still covers the cords' tops, which stand 4.5 mm off the cloth. A cord tucked under the waistband's hem, or the tee pushed clear of the cords, is next.
4. **Underwear through the joggers and the tee** (33 and 21 bodies): the boxers at the gusset and hip under the joggers' close rise; the bra's side band under the tee's armpit (as with the test tee).
5. **Levels of detail cut into the skin** (up to 31 mm): the simpler copies are carried from the same drape but fitted on their own; they need the full copy's push-out (or the per-part LOD switching CO18 plans).
6. **Texture coordinates** short of §2 on most materials: the joggers' gathers and the bands' push stretch the fleece; the shoes' upper is one ring laid out by rows, which isn't conformal; the sole's caps are projected from above. A conformal unwrap (Blender's minimum-stretch or a LSCM pass per island) is the next step.
7. **The shoes are broken after the fit** (the captures): the foot stands inside the sole (its underside is on the floor, so there is no room for a footbed under it), and the import's push-out drags the sole's top and wall out of the foot, crumpling the edge and opening gaps the toes show through. A sole with no surface inside the foot (walls round it and a tread under it, fitted with a margin near zero) is the redesign; also a sole that follows only the foot's bones and the foot-size slider (references §5.2: "a shoe must not swell with body fat"), since the tread now tilts with the foot (up to 9.8 mm off flat; the feminine sole 4 to 5 mm under the floor). The heel and toes come out in the walk's stride; at LOD1 the toes come through the toe box.
8. **The tee's armpit pops in motion** (2.4 mm walk, 5.1 run), as the test tee's did.
9. **No compression folds in bent poses**: binding bends cloth with the skin; asset-quality §3 asks for folds at the elbows, knees and the back of the knee, which needs per-pose corrective shapes for the garment or cloth at run time (ADR-0035).
10. **Item file size**: 12.2, 9.4 and 6.8 MB (the maps are 1.2 to 1.5 MB of each; the rest is 138 sparse morph targets on each of three levels).
11. **Feminine frame**: one masculine drape, carried by the binding. The feminine draft is measured (`body.py`), not draped.
12. **The tee's crew neck is faceted, with tabs above the band at the shoulder seams** (Visual evidence): 22 corners round 537 mm at LOD0, runs up to 39.1 mm. The neck band and its raw edge need finer spacing than the body (the hem has 12.7 mm runs), and the shell must turn under the band at the shoulder seams. No test measures either yet. A neckline check (the longest run, the angle between runs) and a check that no shell corner stands above the band belong in `realistic_wardrobe.rs`, with the fix.
13. **The tee bulges out at the lower back and flares at the hips** (Visual evidence): roughly 80 to 100 mm past the seat line, read off the side cells, on every cast body and worst on the heavyset ones. The likely cause is the push-out over the joggers' seat, waistband and drawcord. The fix is to drape the tee over the joggers rather than the bare body, or a push-out that keeps the hem's line. No test measures the silhouette yet. A check of how far the hem stands off the body at the back, compared with the drape, belongs with the fix.
14. **The three drafts appear unmarked in Character Studio.** asset-quality §5 lists a draft in the studio only with "Show drafts" on, marked "Draft". Nothing outside `randomize.rs` and the tests reads `quality` or `ItemManifest::is_draft` (a grep over `crates/`), and `catalogue.json` lists the three items. So once this merges, the studio's Clothing page offers the tee, the joggers and the visibly broken sneakers on the realistic base as ordinary choices. The filter (a "Show drafts" toggle and a "Draft" mark, from `ItemManifest::is_draft`) is the editor owner's work; this branch may not change the editor. See "At merge".

## At merge

- **Generated files:** the item folders under `assets/characters/realistic/clothing/` are made by the builds and `wardrobe.py` against this branch's `RealisticParts.glb` (co7-hair-cloth plus co9-fit-2). If the body is rebuilt at merge (other branches' shapes or regions), rebuild the items too (the README's commands; about 10 minutes with the tiles fetched), then `catalogue.py`. The tests' records were measured on this build.
- **Randomize** now skips declared drafts: decide whether items with `quality` unset (every stylized item, the realistic underwear) count as drafts too (asset-quality §5 says so); the test `a_declared_draft_is_never_drawn` holds both sides of today's rule.
- **Drafts in the studio (decide before merging to main):** until Character Studio hides drafts (the editor owner's "Show drafts" toggle and "Draft" mark, from `ItemManifest::is_draft`), merging this branch puts the three drafts in the studio's wardrobe for the realistic base, unmarked, including the sneakers with their crumpled soles and toes showing. Either merge after that filter is in, or accept this explicitly and say so in the progress entry. A third option is to keep the items out of `catalogue.json` until then, which would also take them out of the tests that load them through the library.
- **Paul's sign-off** of the first top, bottom and shoe is prepared in `r7-wardrobe-first/README.md`; all three are drafts, so they are offered for review of the pipeline and the direction, not for production.

## Review fixes 1

The review found two blocking items. Both were reporting gaps, not code faults, and both were right: I checked each against the pictures and the shipped files before writing. No item was rebuilt and no code changed, so there is no red run for this round; the quality of all three stays `draft`.

1. **Two visible tee defects were not reported.** I had called the crew neck right. Enlarged, the collar crop and the face cells show a polygonal neckline with a pointed tab of the shell at each shoulder seam (the right one showing the shell's dark inside). The side and back cells show the bulge at the lower back and the flare at the hips. Both are now in "What I saw", open issues 12 and 13, the tee's row of `r7-wardrobe-first/README.md` and the first-wardrobe bullet of `wiki/Roadmap/Known-Limitations.md`, with measurements: the neck edge's corners and runs, measured on the shipped `.glb` in Blender (22 corners round 537 mm, runs up to 39.1 mm at LOD0, against the hem's 12.7 mm), and the bulge read off the side cells (14 to 18 px past the seat line in figures 300 px tall, roughly 80 to 100 mm; said to be a picture reading).
2. **The drafts appear unmarked in Character Studio.** Confirmed: no code under `crates/` reads `quality` or `is_draft` outside `randomize.rs` and the tests, and `catalogue.json` lists the three items. Added open issue 14 and an "At merge" decision for the lead (wait for the editor's draft filter, accept it explicitly, or hold the items out of the catalogue), mirrored in Known Limitations.

From the non-blocking list I also said what the joggers' black crotch patches are: the boxer-briefs record as it looks on screen, not holes (LOD0 has three open loops). The rest stand as the review lists them, for the next round: the 1.8 mm hem proxy and the 0.99 density share in `realistic_wardrobe.rs`; the footwear check passing at rest while the full-detail crop shows toes (the fitting core's owner); footwear and motion on the 18 cast bodies only, and no sheet at the parameter extremes; the close crops kept only as downscaled strips, with no crop of the soles from below or of the waistband alone; the trial drape in `blender-tee-drape.jpg`; the tread figure (5.0 mm over the floor in "Measurements", 5.7 mm in the `FOOTWEAR` comment; the 9.8 mm record holds either way); the shoes hiding no region; the item files' size in git without LFS; and the records following the worst depth but not the body count.

Runs after the fixes:

- `cargo nextest run -p axcenity-avatar -p axcenity-assets -p axcenity-player --no-fail-fast`: Summary 615 tests run: 615 passed (19 slow), 20 skipped; no FAIL, TIMEOUT or ABORT lines.
- `cargo clippy -p axcenity-avatar -p axcenity-assets -p axcenity-player --all-targets -- -D warnings`: clean.
- `python3 -I scripts/check-wiki.py`: 114 pages, 0 errors, 58 warnings (staleness on other pages).
