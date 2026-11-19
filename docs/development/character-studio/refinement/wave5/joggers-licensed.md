# Wave 5 hand-back: the first licensed joggers (decision 13)

Branch `joggers-licensed`, from `refinement-wave5`. Task: Paul's decision 13 for the bottom, "clean cuffs, consistent geometry and proper leg deformation", from a licensed base through the intake (decision 11).

**In one paragraph.** **Not ready for sign-off: the item fails the walk and the hip flex, shows a hole at the front crotch in the walk and a black notch on the seat with the hips flexed, fails asset-quality §3 on Folds and Construction, and its cuffs aren't clean up close.** The base is BlenderKit's *Grey sweat pants* (CC0). Its mesh is named `male_casualsuit06.001`, and it was made from MakeHuman's CC0 system asset of that name. Its provenance record, with that derivation and the page as retrieved, is beside the item. The uploader couldn't be confirmed as the maker; the record says so. The download went through the intake into `axcenity/realistic-fleece-joggers` (a Draft), with a new intake step, `refine` (`refine_joggers.py`). That step remakes the pants as quads in rings round the realistic body. **The item's mesh is made from the body's hull, not from the download's surface**: every ring is the convex hull of the realistic body at its height plus an ease, and the drawcord and pockets are generated; only the maps (baked onto the new surface) and the pockets' places come from the download (see Review fixes 1 for the decision-11 question this raises). It is shortened and tapered to an 8.5 cm hem with a gathered cuff, seven edge loops round the knee, and the waistband raised over the underwear's band. The waistband and cuffs are turned inside 3 mm, the back pockets rebuilt closed, and a drawcord added. It makes new texture coordinates and bakes the download's maps onto them. Against the wave-4 joggers:
- the ankle failure is gone (no skin out at the back of the ankle on any body);
- the cloth wraps the front of the bent knee (no surface point more than 1 mm inside the skin; the wave-4 joggers fail the same measure). Whether the back of the knee collapses is not proven: see Review fixes 1;
- the boxer briefs no longer come through at rest.

What fails, measured on the 54 gate bodies:
- the stride: up to 52 bodies, 41.7 mm, from the waistband's rounded top (a measure artifact, explained below), the knee's side and back, and the boxer briefs through the seat;
- the hip flex: 52 bodies;
- the waistband's top edge: up to 15.6 mm off the lumbar with the hips flexed;
- a 2.45 mm jolt in the Run;
- the simpler copies: 8 and 5 bodies at rest;
- the shipped texture coordinates: the engine fit lifts the rest after they are made;
- the cuffs: they look mottled close up;
- a hole at the front crotch in the walk, a black notch on the seat with the hips flexed, and asset-quality §3 Folds and Construction (Review fixes 1).

The tests stay strict (no records) and fail as measured. The state stays **Draft**.

## Files

| File | What |
|---|---|
| `assets/characters/realistic/import/refine.py` (new) | Intake step 3b: runs a refinement recipe of this folder named by the spec (`refine.recipe`), between `topology` and `conform` |
| `assets/characters/realistic/import/refine_joggers.py` (new) | The joggers' recipe (its module note says what it keeps and remakes, every number in the open): sections and hulls, the height maps to the download, the ring topology, the leg-aware pelvis rings, the drape, the crotch line, turned edges, pockets, drawcord, Blender finishing and minimum-stretch unwrap, and the bake (each texel followed round the body to the download; normals turned from the old texture frame to the new). It also cross-fades the two readings across the crotch's ring |
| `assets/characters/realistic/import/test_refine.py` (new) | 10 tests: hull and ray, a hull bridging a hollow, the height maps both ways, the knee loops (seven within 6 cm, at most 15 mm apart), cuff and waistband landmarks, the cuff gathered and the knee roomy, closed cord and pocket, the turned edge, the loader's UV layer and WebP |
| `assets/characters/realistic/import/specs/fleece-joggers.intake.json` (new) | The spec: item, source and evidence, load, topology, refine, levels, fit (`under`: boxer briefs and briefs), variants |
| `assets/characters/realistic/import/load.py` | `"uv": "textures"` reads the UV layer the base colour map uses (the download carries two; its maps use the second). Maps in a format the engine doesn't read (WebP) are written as PNG. A parameter no longer shadows the loop's `uv` |
| `assets/characters/realistic/import/intake.py`, `conform.py` | The `refine` step; `conform` reads `refined.npz` when a recipe ran (`source_of`) |
| `assets/characters/realistic/import/fit.py` | The refinement's changes go into the record. A map several materials share is stored once. `sheen` and `variants` pass from the spec to the model and manifest |
| `assets/characters/realistic/import/README.md`, `assets/characters/realistic/clothing/README.md` | The step, the spec keys, two known limits; the new item's row |
| `assets/characters/realistic/clothing/fleece-joggers/` (new, generated by the intake) | `Pants_FleeceJoggers.{glb, binding.json, png, item.json, provenance.json}` and `Pants_FleeceJoggers.source-page.txt` (the asset page and API record as retrieved, with hashes; the MakeHuman evidence) |
| `assets/characters/catalogue.json` | Regenerated by the export (`fit.write_catalogue`, `catalogue.py`): the new item listed |
| `crates/player/src/character_capture.rs` | `HeldPose::HipFlex` (thighs 90 degrees, knees 90, outside `ALL`); `hip-flex` for `AXCENITY_CAPTURE_POSE`; `the_hips_flexed_pose_raises_the_thighs_level_and_hangs_the_shins` |
| `crates/player/src/fit_gate_tests.rs` | Two layered pairs (boxer briefs and briefs under `Pants_FleeceJoggers`); the new test `garments_cover_the_cast_with_the_hips_flexed`. No existing test, rule or record changed |
| `crates/player/src/fleece_joggers_tests.rs` (new), `main.rs` | 17 tests (below; Review fixes 1 replaced the knee test with two) |
| `crates/avatar/tests/licensed_joggers.rs` (new) | 6 tests (below) |
| `crates/avatar/tests/content_states.rs` | `WAVE_5_DRAFTS`: the new item classified Draft (the test asks each shipped item to be classified) |
| `docs/development/character-studio/testing.md`, `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md` | The pose, the tests, the item's limits |
| `refinement/wave5/joggers-licensed-red-run.txt`, `joggers-licensed/` | The red run; sheets, close crops, before and after, and the refinement's, checks' and export's reports (2.9 MB) |

No dependency was added (the bpy environment already has NumPy and Pillow). No shared contract changed (the fit, the gate's rule, the manifest schema, the content-state UI, render and the Studio UI are untouched).

## Provenance

- **The record** is `Pants_FleeceJoggers.provenance.json` (format 1). It holds:
  - the asset, the page (`blendkit.com/asset-gallery-detail/e46f029b-…`), the author as the page names them (Introspective Mnisi), and CC0-1.0 with its legal code;
  - the retrieval date, 2026-11-18;
  - the download, `bk-grey-sweat-pants.glb`, SHA-256 `04abe82b…1fa235` (451,156 bytes);
  - the changes the steps made;
  - the evidence, quoted.
- **Beside it, `Pants_FleeceJoggers.source-page.txt`:**
  - the page as retrieved: its visible text in full, plus the HTML's SHA-256 `be9c0222…` (857,425 bytes);
  - the public API record (`"license": "cc_zero"`, SHA-256 `309b20cb…`). The uploader's profile fields are left out; the hash is of the whole record;
  - BlendKit's licensing FAQ quote (SHA-256 `c7db5b38…`).
- **The derivation.** The glTF's mesh is `male_casualsuit06.001`. MakeHuman's `male_casualsuit06.mhclo` (SHA-256 `9cebda22…`) says "This asset was explicitly released as CC0 in september 2020". MakeHuman's LICENSE.md lists "Clothes (any MHCLO-based asset)" under CC0 1.0 (LICENSE.ASSETS.md is the CC0 legal code).
- **The shape match.** At MakeHuman's scale the pants lie a median 17 to 20 mm from the suit's surface: the same garment, retopologized, lengthened and loosened.
- **The uploader.** Authorship of the upload couldn't be confirmed: the uploader is new and the page doesn't mention MakeHuman. Both the upload and its origin are CC0, so the item is cleared either way.
- **The page is saved** because BlenderKit says authors may later exclude assets from redistribution. Individual MakeHuman developers' names are left out of the repository (the files name them).
- `shipped_provenance` passes on the library; the record is sound.

## How it was made (the refinement, measured)

- **The download**, as analysed by the refinement:
  - 6,143 triangles, all of them triangles (0% quads);
  - the crotch at 0.662 m, 13 cm under the body's;
  - the cuffs pooled on the floor round a 52 cm opening;
  - its maps are WebP in the second UV set, and its "roughness" map has metal = 1 in B;
  - no drawcord, though the sourcing report listed one (the waistband is elastic only).
- **Made:**
  - 5,280 triangles, 98.2% of the faces quads;
  - the pelvis in 15 rings of 32 columns, each leg in 31 rings of 24 columns;
  - seven loops within 6 cm of the knee, 15 mm apart;
  - waistband top 1.035 m with a 4 cm band, the hem 0.085 m with a 5 cm cuff.
- **Girths (cloth against the body's hull, masculine left leg):**

  | Height | Cloth | Body's hull |
  |---|---|---|
  | Waist top | 892 mm | 848 mm |
  | Thigh at 0.62 m | 581 mm | 477 mm |
  | Knee | 464 mm | 365 mm |
  | Above the cuff at 0.155 m | 299 mm | 219 mm |
  | Cuff seam | 247 mm | 216 mm |
  | Hem | 296 mm | 265 mm (the ankle bone) |

- **Ease** over the hull of the body with its boxer briefs: 8 mm at the band, 13 to 17 mm over the seat, 16 to 18 mm at the thigh and knee, 5.5 mm at the cuff. The legs hang from the seat: a column comes in at most 0.35 of its drop.
- **Maps:**
  - one 2048 px set (base colour, normal, occlusion-roughness-metal) shared by Fleece, Rib and Cord, with 71% of the atlas used;
  - 2,733 of 2,992,016 texels fell back to the nearest point;
  - the albedo's mean is brought to 0.8, and the heather-grey tint #c7c7c7 gives the download's grey back;
  - sheen: Fleece 0.35, Rib 0.30, Cord 0.25. Aglets and eyelets are Standard, without a map.
- **Variants:** heather grey, charcoal, black, navy (white cord) and olive.
- **The intake's §2 checks per level** (`joggers-licensed/intake-checks.json`):

  | Check | LOD0 | LOD1 | LOD2 |
  |---|---|---|---|
  | Triangles | 5,280 | 2,640 | 1,319 |
  | Degenerate, non-manifold or flipped | 0 | 0 | 0 |
  | Inside the body | none, least 0.7 mm | none | none |
  | Area over 10% stretch (before the fit) | 4.0% | 31% (collapse decimation) | 52% |
  | Texel density | 15 px/cm and more | | |
  | `no_weight_past_the_next_joint` | fails | fails | fails |

  The far-weight rule fails on every level. The same rule fails the realistic body's own skin on 4,101 of its 12,692 corners. The joggers carry the body's weights (the engine fit's), so the rule's proxy (the nearest bone's head) is what's wrong. I measured a segment version too: the body still fails it on 2,381 corners. Open for the lead.
- **The engine fit** (`intake-export.json`):
  - pushed 48 corners out of the body at rest, at most 1.3 mm;
  - lifted the rest on 855 corners, up to 52.5 mm, for the body extremes (`extremes_lifting`): the crotch seam rose 2.3 cm and the drawcord came forward 16 mm;
  - `inside_most` 8.6 mm.

## Tests (red first: `joggers-licensed-red-run.txt`)

**Red.** Every new test failed for the reason expected: the item wasn't in the library, `HeldPose::HipFlex` turned no bones, and `refine_joggers` didn't exist. The walk thirds passed with no joggers worn, so they now assert the joggers are worn and fail.

**Final runs** (the item as committed):

| Run | Result |
|---|---|
| `python -I test_refine.py` (Blender's Python) | 10 tests OK |
| `python -I test_intake.py` | 26 tests OK |
| `cargo nextest run -p axcenity-avatar --test licensed_joggers` | 5 passed, 1 failed: texture coordinates even on 90.4% (Fleece) and 88.7% (Rib) of their area, §2 asks 95% (dense 100%, no overlap; Cord 97.1%) |
| `cargo nextest run -p axcenity-player -E 'test(/hips_flexed_pose\|held_poses/)'` | 2 passed |
| `cargo nextest run -p axcenity-player -E 'test(/fleece_joggers/)'` | 2 passed (since Review fixes 1: the knee's front and the measure's own check), 15 failed (below) |
| `fit_gate_tests::garments_cover_the_cast_with_the_hips_flexed` | failed: test tee 8 bodies 5.77 mm, briefs 1 body 3.27 mm, hidden skin showing 9 bodies 3.77 mm |
| Whole suites, clippy, fmt, wiki | below, "Suites" |

**What the joggers' gate measures.** Bodies failing out of 54 and the worst; Pants means skin through the joggers, Boxers and Briefs mean that underwear through them:

| Pose | Pants | Boxers | Briefs |
|---|---|---|---|
| Rest, T-pose, arms raised, elbows bent | 5, 33.0 mm | none | none |
| Idle | 5, 33.5 mm | none | none |
| Knees bent | 6, 33.0 mm | none | none |
| Arms forward | 6, 32.3 mm | 1, 14.2 mm | none |
| Walk@18 | 5, 33.6 mm | 17, 10.5 mm | none |
| The stride, first third | 51, 40.6 mm | 33, 10.4 mm | 1, 3.6 mm |
| The stride, second third | 22, 38.0 mm | 32, 13.2 mm | 2, 3.1 mm |
| The stride, last third | 50, 40.5 mm | 33, 10.2 mm | 1, 3.7 mm |
| Hips flexed | 52, 32.1 mm | 33, 13.4 mm | 20, 5.9 mm |
| Levels at rest | LOD1 8, 30.2 mm; LOD2 5, 31.1 mm | | |

The motion check: Idle and Walk pass (pops under 0.42 mm, no tear). The Run fails on all 18 cast bodies, a corner jolting 2.31 to 2.45 mm at the seat's back (0.80 m) and the crotch's back (0.74 m).

The waistband check: the top edge is up to 15.6 mm off the skin on the masculine bodies (the lumbar, with the hips flexed) and 11.2 to 13.6 mm on the feminine ones. The limit, set before measuring, is 12 mm. The knee check (since Review fixes 1): with the knees bent, no sample of the cloth's surface over the front of either knee lies more than 1 mm inside the skin on any cast body (passes; the wave-4 joggers fail it on the right knee of every cast body, 2 to 6 samples, up to 3.5 mm). The convex-hull section test it replaced could not fail.

**Where the failures are**, from `AXCENITY_FIT_GATE_DEBUG=1`, by region:
- **Rest** (the run before the final ease, the same classes):
  - 18 corners on 4 bodies at the waistband's top at the back, up to 32 mm;
  - 18 corners on 12 heavy bodies at the inner thigh's top, up to 6.7 mm;
  - the boxer briefs at the seat: 29 corners on 11 bodies, 5.5 mm.
- **The stride:**
  - the boxer briefs through the seat and the crotch's back (up to 278 corners on 28 bodies, 12.5 mm);
  - the waistband's top at the back (71 corners on 8 bodies, 41.7 mm);
  - the knee's side and back (100 corners on 31 to 39 bodies, 7.4 mm);
  - the inner thigh's top (36 corners on 15 bodies, 7.4 mm).
- **The hips flexed:**
  - the groin's front and the waistband's front (245 corners on 39 bodies, 6.5 mm);
  - the underwear at the front of the waistband and groin (up to 13 mm).

**The waistband-top cases are the gate's measure, not skin through cloth.** The gate leaves out skin near a garment's hem only near an open edge (`hem_corners`). A turned waistband has none: its raw edge is the facing's, 2 cm inside. So the lumbar skin above the band, its normal leaning up like the fold's, counts as covered by the fold's upward-facing top and "out" by its height above it ("way out meets nothing"). I didn't reshape the fold to dodge the measure. A gate change would treat a fold ring (two faces turning more than 120 degrees) as a hem; that is the lead's.

## Visual evidence (`joggers-licensed/`)

In-engine captures, release, studio light, of the masculine and feminine average, slim, heavyset and muscular cast bodies wearing the joggers over their underwear:
- 2560 × 1440: rest, Walk@0.133 and Walk@0.383, knees bent and hips flexed;
- 3840 × 2160 for the average bodies: rest, Walk@0.133 and hips flexed;
- the wave-4 joggers, the same way, for before and after.

Sheets: `joggers-{rest, walk-0.133, walk-0.383, knees-bent, hip-flex}.jpg` show one body a row (in the order above) and front, side and back in the columns. Also `before-wave4-joggers-{rest, walk-0.133}.jpg`, `before-after-{front, side, back}.jpg`, and `close-01` to `close-10` (cuffs, knees, waistband and drawcord, seat and pockets, the walk's knee and crotch, the hip flex).

**What I saw:**
- **Overall.** They read as grey fleece joggers on every body and both frames.
  - The waistband has its elastic channels and pleated gathers, and the front its pleats.
  - Two cords with aglets come out of metal eyelets.
  - The back has its patch pockets: one clearly outlined at the right buttock on the masculine average (`close-05`); the other reads only by its stitching.
  - The legs taper to the ankle. The hem sits over the ankle bone, not pooling (`before-after-side`: the wave-4 cuff stood wide off the ankle, its band's dark inside showing).
  - The fit is slim rather than relaxed: about 10 cm of ease round the thigh, 10 cm round the knee. On the heavyset bodies the seat and thighs fill the cloth.
- **Wave 4 against these** (`before-after-*.jpg`). The wave-4 joggers show black gaps at the waistband's top and the boxer band's dark rim above it, a pocket hole at the hip, and the cuff's dark inside. These show none of those. The waistband sits over the boxer band, which shows only as a thin dark line above it at the back.
- **The cuffs fail "clean cuffs"** (`close-01`, `close-02`, every side cell of every sheet). The 5 cm cuff band is mottled, with white and dark smears and no readable gathers. It is the download's gathered cuff (pooled, 52 cm round) baked onto a 25 cm band: its normal-map ruffles compressed about 2:1 and broken up. The hem's edge is round and whole (no tear), but the band doesn't read as a rib cuff. This is the item's most visible defect. A rib texture of its own for the cuff (the Rib material) is the fix.
- **A speck of skin, about 5 mm, at the hip in the walk** (`close-07`, at the top of the crop). It shows on all four masculine bodies at Walk@0.133 (`joggers-walk-0.133.jpg`, side column), between the seat and the side pocket's line. It is the gate's "crotch/thigh top" class. A dark speck also shows at the fly in the front column of the masculine average, slim and heavyset at Walk@0.133.
- **A hole at the front crotch in the walk** (`close-08`; `close-11`, masculine muscular, Walk@0.8, front). Skin and a black wedge show through an opening about 6 cm wide and 2.5 cm high below the drawcord's end (measured on the 2560 x 1440 capture, about 650 px a metre). Added in Review fixes 1; the first hand-back missed it.
- **A black notch on the seat with the hips flexed** (`close-10`, `joggers-hip-flex.jpg` back column; `close-12` feminine heavyset and `close-13` masculine average, after Review fixes 1). On the masculine average, heavyset and muscular and the feminine heavyset, lower right of the seat seen from behind. It is a mesh fault, not the texture: Review fixes 1 gives the evidence.
- **Painted folds** (§3 Folds fails). The bake carries the download's baggy-drape folds onto the slim shape: its drop-crotch folds show as dark vertical streaks down the middle of the seat at rest (`close-05`, `before-after-back.jpg`), and with the hips flexed the waistband's gather stripes run about 14 cm down onto the seat (`close-10`, `close-12`, `close-13`). They contradict the drape. There are no compression folds at the knee or the back of the knee with the knees bent.
- **The drawcord's aglets** render as flat bright-white wedges (`close-08`, `close-11`).
- **The knee in the walk** (`close-07`) bends smoothly: no collapse, no folded-over triangle visible. The gate's knee cases (up to 7.4 mm, side and back) don't show at this size.
- **The hips flexed** (`close-09`, `close-10`, `joggers-hip-flex.jpg`).
  - The seat and thighs stretch smoothly round the bend.
  - The waistband at the back stands a little off the lumbar: a dark gap between band and skin, about 1 cm. That is the waist check's 15.6 mm.
  - On the feminine bodies the boxer or briefs line shows at the front fold.
- **Texture.** The fleece melange reads at arm's length under the studio light. The stitching lines of the download's seams follow the new panels: a side seam, and the inseam continuing to a short vertical stitch line at the cuff. The waistband gathers carry over. Not captured: the hard, warm and outdoor lights, or a turntable.

## Open issues

1. **The stride and the hip flex** (the main failure).
   - The boxer briefs come through the seat and crotch in the walk: their pouch and leg openings stand up to 29 mm off the skin, and the fit's lift moved the crotch seam up 2.3 cm into them.
   - The knee's side and back come out up to 7.4 mm.
   - The groin's front fails with the hips flexed.

   Larger ease didn't help: legs at 24 mm, against the final 16 to 18 mm, measured 53 bodies and 34.5 mm in the stride's first third (the final ease: 51 bodies, 40.6 mm). Within binding and linear skinning I found no setting that passes. As the tee-walk hand-back found for the tee, this is the fit's: a deformer, or a fit that doesn't lift the default rest for the extremes.
2. **The waistband-top count** is the gate's hem rule (above). A gate ruling for the lead.
3. **The cuffs aren't clean up close** (visual; `close-01` now shows them, `close-02`). They need their own rib map instead of the download's compressed gather.
4. **Texture coordinates after the fit.** Even on 90.4% (Fleece) and 88.7% (Rib) of the area, against 95%. Before the fit it is 98.7% and 89.8%: the lift moves 230 fleece corners more than 2 mm, up to 24 mm, after the unwrap. The Rib's own unwrap of the band and facings is under 95% too.
   - Fix A: a two-pass export (unwrap on the lifted rest).
   - Fix B: the joggers made to the extremes' envelope.
5. **The simpler copies** come from collapse decimation: stretch fails, and they cut the skin on 8 and 5 bodies at rest (the same waist-top class and the inner thigh). Simpler copies made by the recipe itself (every other ring and column) are the fix; not done.
6. **The far-weight rule** fails the body's own skin (above).
7. **The Run jolt**, 2.45 mm at the seat's back, over the motion check's limit.
8. **The gate's new hip-flex test** fails for the existing garments too (test tee 8 bodies, 5.8 mm; hidden skin 9 bodies). It has no record, because records are the lead's to set on a new pose.
9. **Not measured:** GPU time; the four lights; the body-parameter extremes beyond the gate's 54 bodies; a shoe worn with the joggers. The cuff's hem sits above a sneaker's collar by construction, but nothing checks it.
10. **The front crotch hole in the walk** (`close-08`, `close-11`): about 6 x 2.5 cm on the masculine muscular at Walk@0.8, skin and black showing.
11. **The black notch on the seat with the hips flexed** (`close-10`, `close-12`, `close-13`): a fold-over in the strip joining the pelvis tube to the legs (Review fixes 1).
12. **§3 Folds**: the download's drop-crotch folds baked as dark streaks down the seat at rest, the waistband's gather stripes 14 cm down the seat with the hips flexed, no compression folds at the bent knee. Fix routes: mask the seat and the area under the band out of the bake's normal and occlusion reading, and give the knee folds of its own (a person's work, or a cloth simulation).
13. **§3 Construction**: the panels are rings round the body's hull, not a sewing pattern.
14. **The speck at the hip** shows on all four masculine bodies at Walk@0.133, not only the average.

## For the lead at merge

- **Nothing is recorded.** The 15 joggers tests, one avatar test and the gate's hip-flex test fail as measured above. Choose one:
  - set records for this Draft (exact counts as above);
  - hold the item back from the merge;
  - fix the fit first.

  I didn't write records: making the suite pass over these failures is your decision.
- **Generated files.** I regenerated the item folder and `catalogue.json` with the intake:

  ```
  python -I assets/characters/realistic/import/intake.py --spec assets/characters/realistic/import/specs/fleece-joggers.intake.json --downloads <folder with bk-grey-sweat-pants.glb> --work <scratch> --out assets/characters
  ```

  It takes about 60 s; the export runs `import_garment --release`. If the body is rebuilt at merge, run it again. `catalogue.json` gained one item and nothing else.
- **Content state.** The item is `quality: "draft"`, hidden from normal character creation and never drawn at random.
- **`LAYERS`** in `fit_gate_tests.rs` grew to 9 pairs. Another branch adding pairs merges by hand.
- **Rulings for you:**
  - the gate's hem rule for turned edges (open issue 2);
  - the far-weight rule (open issue 6);
  - records or not for the hip flex;
  - whether a recipe that remakes the garment from the body's hull counts as "retopology help" or needs a person's modelling (decision 11; Review fixes 1), and with it asset-quality.md §6's sentence that the intake doesn't retopologize.

## Suites

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-player --no-fail-fast` | 270 tests run: 254 passed, 16 failed (exactly the 15 failing `fleece_joggers_tests` and `fit_gate_tests::garments_cover_the_cast_with_the_hips_flexed`); every test that existed before passes, 22 skipped, no ABORT lines |
| `cargo nextest run -p axcenity-avatar --no-fail-fast` | 211 tests run: 210 passed, 1 failed (`licensed_joggers` texture coordinates); `content_states` passes with the item classified, `shipped_provenance` passes |
| `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | applied, clean |
| `python3 -I scripts/check-wiki.py` | 115 pages, 0 errors, 57 warnings (staleness notes on pages this branch didn't touch) |
| `test_refine.py`, `test_intake.py` (Blender's Python) | 10 and 26 tests OK |

## Review fixes 1

The review found three blocking problems. Each is fixed or answered below. Everything was measured on the worktree after the fixes. The item stays **Draft** and is not ready for sign-off.

### 1. Visible defects the first hand-back left out

All four are now in "What I saw", "Open issues" (10 to 14), Known-Limitations' joggers line and the returned openIssues. Two crops were added and two replaced:
- `close-01` (replaced). The old crop cut the cuff off at the top. The new one shows the cuffs from the front, in Idle, on the masculine average. The 5 cm band is plain and mottled, with light smears and the inseam's stitch line. No gathers can be read.
- `close-11` (new): the front crotch in the walk, masculine muscular, Walk@0.8.
- `close-12` and `close-13` (new): the seat with the hips flexed, on the feminine heavyset and the masculine average, after the normal-map fix below.

**(a) The front crotch hole in the walk.** It is in `close-08` and `close-11`. Skin and a black wedge show through an opening below the drawcord's end. The opening is about 6 cm wide and 2.5 cm high: 40 by 16 px on a 2560 x 1440 capture, at about 650 px a metre. At Walk@0.133, a dark speck shows at the same place on the masculine average, slim and heavyset bodies (`joggers-walk-0.133.jpg`, front column). The gate's stride counts it among the crotch failures.

**(b) The black notch on the seat with the hips flexed.** It is a mesh fault (a fold-over). It is not a texture fault, and it is not the underwear. The evidence:
- **Not the underwear.** I captured the masculine frame in the joggers twice: once with the boxer briefs and once with no underwear. The notch is the same in both: 20 px in the same box, (1324, 777) to (1330, 783). The gate's debug run of the hip flex finds no skin or underwear through the seat. Every skin or boxer corner it reports is at the waistband (y 0.99 to 1.01) or at the front of the groin (z 0.37).
- **Not the texture.** The baked normal map did carry a texture fault:
  - 176 texels faced into the surface (tangent z at or below 0), and 172 of them were on two seat triangles at x -0.03, y 0.80, z -0.10.
  - They came from the download's own normal map, which has 71,519 such texels inside its islands (in a band of rows 448 to 512).
  - I fixed this, test first: `refine_joggers.outward` reads any texel with z under 0.2 as flat (`test_refine.py`, `BakeTest`).
  - After rerunning the intake, 1,866 texels changed and 2 texels with z at or below 0 are left. The geometry and the other maps are byte for byte the same.

  **The notch did not change.** It is the same 20 pixels, in a flat, unlit colour (12, 14, 17).
- **What it is.** The cause is the strip that joins the pelvis tube to the legs along the back of the crotch line. Its winding is consistent (the intake's flipped-edge check is 0), but its triangles fold over:
  - At rest on the masculine average, at least 8 of its triangles already face against their corners' normals. They are thin and nearly edge-on (face against normal: -0.0007 to -0.21), at x -0.075 (y 0.84 to 0.93) and at x +0.05 to +0.15 (y 0.83).
  - With the hips flexed, at least 10 face against their normals on the masculine average and 19 on the feminine heavyset. The strip's triangles stretch up to 3.6 times their rest area (35 to 128 mm²). These are counts from a printout cut at 40 lines, so the masculine counts are lower bounds.
- **Why it shows black.** Garments are drawn double-sided (`avatar::fit::glb`), so a folded-over triangle shows its back face. That face's normals point away from the light and the camera, so it draws flat and unlit.
- **The fix** belongs in the recipe's crotch transition, where the pelvis tube's last ring meets the legs' first (`build`, the crotch line and the relaxed transition). I didn't find which corners cross over; that isn't a narrow change, so I didn't make it in this pass.

**(c) Painted folds: a §3 Folds failure.** The bake carries the download's baggy-drape folds onto the slim shape:
- The download's drop-crotch folds show as dark vertical streaks down the middle of the seat at rest (`close-05`, `before-after-back.jpg`).
- With the hips flexed, the gathers baked under the waistband run about 14 cm down onto the seat (`close-10`, `close-12`, `close-13`).
- With the knees bent, there are no compression folds at the front or the back of the knee.

I listed this as a failure rather than masking the seat out of the bake. A mask would leave a flat seat, which doesn't pass §3 either. The real fix is folds made for this shape: a person's work, or a cloth simulation.

§3 Construction fails too: the panels are rings round the body's hull, not a sewing pattern.

**(d) The hip speck** shows on all four masculine bodies at Walk@0.133 (`joggers-walk-0.133.jpg`, side column), not only on the average body.

### 2. The knee test could not fail

The reviewer showed that the old convex-hull section test passes the wave-4 joggers too (ratios 1.27 to 1.61). I tried four measures of collapse. On every cast body with the knees bent 135 degrees, none of them could tell the fleece joggers (seven edge loops round the knee) from the wave-4 joggers:
- **An ordered, non-convex section loop, in the skin ring's plane.** The skin's own ratio is 2.8 to 4.8.
- **The same loop, in the knee's mitre plane.** The ratios run from 1.8 to 14 because the thigh and the calf merge. On the heavy bodies, no loop encloses the centre.
- **The share of the knee's area whose triangles fold over:**
  - wave 4: 5.0 to 10.7%;
  - fleece: 5.9 to 9.7%;
  - the body's own skin: 3.5 to 7.6%.
- **How far the cloth lies inside the skin at the back of the knee:**
  - wave 4: up to 11.5 mm;
  - fleece: up to 14.6 mm.

  This is the crease that the calf and the thigh close.

**What I kept: the front of the bent knee**, the side that the bend stretches. The test checks whether any point of the cloth's surface lies more than 1 mm inside the skin. It samples seven points per triangle, not only the corners as the gate does.
- `the_fleece_joggers_wrap_the_front_of_the_bent_knee`: 0 samples inside on every cast body and both knees, out of about 1,500 to 1,750 samples per knee. It passes.
- `the_knee_measure_catches_the_wave_4_joggers`: the wave-4 joggers fail on the right knee of every cast body, with 2 to 6 samples inside, up to 3.5 mm. The test asserts that failure, which shows the measure can fail. The red run is in `joggers-licensed-red-run.txt`.

**What this proves.** The cloth wraps the front of the bent knee without cutting into it. It does not prove "without collapse":
- the back of the knee isn't judged;
- the fleece joggers fold as much there as the wave-4 ones;
- there are no compression folds.

So the claim "the knee bends without collapse (passes its test)" is withdrawn from this hand-back, from testing.md and from Known-Limitations.

**Not tested.** I didn't rebuild the joggers with `LEG_RINGS` thinned through the knee. So I can't say how much of the front's pass comes from the edge loops and how much from the ease (16 to 18 mm at the knee).

### 3. The mesh comes from the body's hull: a decision-11 question

**The item's mesh is not the download's surface.** Every ring is the convex hull of the realistic body, wearing its boxer briefs, at that ring's height, plus an ease. The drawcord and the pockets are generated.

What the download contributes:
- its maps, baked onto the new surface;
- the pockets' places;
- the heights of the cuff and the waistband.

Its proportions were measured and then replaced.

Decision 11 moves away from fully procedural, Blender-generated clothing as the main way to make clothing. The lead's rule is that where an item needs hand modelling, "the lead says so instead of passing a scripted approximation".

**A ruling for the lead:** does a recipe like `refine_joggers.py` count as asset-quality.md §6's "Blender scripts: retopology help"? Or is it procedural modelling, so the licensed base needs a person to model it? I read it as closer to the second: what this base gave was its maps and its licence, not its shape.

Docs:
- The import README's known limit now says that the `refine` step does retopologize and remake the garment when a spec names a recipe.
- asset-quality.md §6 still says the intake "does not retopologize or rework a garment's construction". That needs an update or a ruling from the lead; I didn't change it.

### Non-blocking items

- **Domain.** The record's `source_page` is the blendkit.com address the brief gave. The API record that corroborates the licence is on blenderkit.com, under the same asset id. I left the record as retrieved: it is evidence, and changing the URL after retrieval would misstate what was saved. The canonical domain is blenderkit.com.
- **Aglets.** They render as flat bright-white wedges (`close-08`, `close-11`). Now listed.
- **Pockets.** They are asymmetric: one is outlined and the other only stitched. Not changed.

### Files changed in Review fixes 1

- `crates/player/src/fleece_joggers_tests.rs`: `knee_front_cut`, `knee_front_failures` and two tests replace the convex-hull knee test. The module note is updated.
- `assets/characters/realistic/import/refine_joggers.py`: `MIN_NORMAL_Z` and `outward`, which the bake uses. `test_refine.py`: `BakeTest`.
- `assets/characters/realistic/clothing/fleece-joggers/Pants_FleeceJoggers.glb`: regenerated by the intake with the command under "For the lead at merge". Only the normal map changed, by 1,866 texels. The geometry, weights, LODs and other maps are byte-identical. `catalogue.json`, the manifest, the binding and the provenance came out unchanged.
- `assets/characters/realistic/import/README.md`, `docs/development/character-studio/testing.md`, `wiki/Roadmap/Known-Limitations.md`, this file and `joggers-licensed-red-run.txt`.
- `joggers-licensed/close-01.jpg` (replaced); `close-11.jpg`, `close-12.jpg` and `close-13.jpg` (new).

### Runs

| Run | Result |
|---|---|
| `python -I test_refine.py` (Blender's Python) | red first: `BakeTest` errored (`no attribute 'outward'`); then 11 tests OK |
| `python -I test_intake.py` | 26 tests OK |
| `intake.py --spec specs/fleece-joggers.intake.json ...` (rerun) | load 5.2 s, refine 46.4 s, export 20.7 s; only the GLB's normal map changed |
| `cargo nextest run -p axcenity-player -E 'test(/front_of_the_bent_knee\|knee_measure_catches/)'` | 2 passed (the wave-4 joggers fail the measure on every cast body's right knee; the fleece joggers pass it everywhere) |
| `cargo nextest run -p axcenity-player --no-fail-fast` | 271 tests run: 249 passed, 21 failed, 1 timed out, 22 skipped, no ABORT lines. The failures: the same 15 `fleece_joggers_tests` and `fit_gate_tests::garments_cover_the_cast_with_the_hips_flexed` as before, plus 5 `makeup_tests` and the timed-out `skin_value_tests::skin_tone_values_change_the_picture_the_way_they_mean`, which are load (other builds on the machine). Rerun alone with `-j 1`, the 7 makeup tests that failed in either run and the skin-tone test all pass |
| `cargo nextest run -p axcenity-avatar --no-fail-fast` | 211 tests run: 210 passed, 1 failed (`licensed_joggers` texture coordinates, unchanged: the fix touched only the normal map) |
| `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | applied |
| `python3 -I scripts/check-wiki.py` | 115 pages, 0 errors, 57 warnings (staleness notes on pages this branch didn't touch) |

The gate's numbers are unchanged: the regenerated GLB has the same geometry and weights byte for byte.
