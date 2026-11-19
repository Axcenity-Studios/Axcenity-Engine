# Pose correctives from cloth simulation: hand-back (refinement wave 5, stage 3)

Branch `pose-correctives`, from `refinement-wave5` (main, waves 1 to 4, PR #428, Paul's decisions 11 to 14, wave 5 stages 1 and 2). Task: pose correctives for garments, baked from cloth simulation and driven by the joints like the body's own swing correctives, so a garment's folds and the cloth between the arm and the side behave in the shoulder, arm-swing, elbow, hip and knee poses.

**In one paragraph.** The pipeline is built end to end and the engine drives it: an item can carry its own joint-driven shapes (an optional `correctives` in the item manifest, the same rules in its model's glTF extras, read into `MorphTarget::driver`, kept in cooked models, driven per frame on every body by `anim`), and a new intake step (`correctives.py` around `avatar::fit::cloth`) bakes them with Blender's cloth simulation. **The main goal is not met.** On the test tee the correctives lower the stride's poke-through in its second third (16.6 to 14.1 mm) and its corners under a flipped fold in every third (147, 199, 161 to 122, 136, 139), but the folds still flip, and the slit, the bra triangle and the arms-raised flaps look the same in 4K captures. They also make some cases worse: the bra through the tee in the first third (20 bodies, 10.8 mm to 21, 14.9 mm), one body more at Walk@18 for the tee and for the bra, and a bra case with the arms raised. Taking them into the gate would raise records, so the gate keeps its test tee without them and a new test module measures it with them, with its own exact records. On the licensed tee the correctives made the held poses much worse (the T-pose from 7 to 53 bodies, the arms raised from 14 to 54), so I didn't commit them. The cause I can see: the cloth simulation can't put cloth between a hanging arm and the side where the body's own skin overlaps (the arm sinks into the side), and it moves the bridge of cloth up to 12 cm, crumpled at a triangle's scale. Measured on the kept bake (review fixes 2), three things lose that move: blending between key poses (even shapes that meet each key pose exactly are off half way through the swings by a 95th percentile of 9 to 16 mm, where the cloth moves 16 to 19 mm), the smoothing, and the 6 mm hold, which the gate sweep chose. The frame cost is small (50 characters: +2% shape moves, about +0.02 ms of GPU posing).

## Files

| File | What |
|---|---|
| `crates/assets/src/model/morph.rs` | `MorphTarget::driver: Option<PoseDriver>`; `PoseDriver {bone, axis, start, full}` and `PoseDriver::from_json` (a rule naming no bone, a zero axis or angles that don't rise is skipped); `MorphNames::drive`. |
| `crates/assets/src/model.rs`, `lib.rs` | glTF loading reads a mesh's `extras.axcenity_correctives` into the drivers of the shapes it names. `PoseDriver` exported. |
| `crates/assets/src/model/cooked_model.rs` | The driver is cooked (a flag, the bone, five floats). |
| `crates/assets/src/model/identity.rs`, `crates/anim/src/face_tests.rs`, `live_identity.rs` | `driver: None` in test literals. |
| `crates/assets/src/model/tests.rs` | `blinking_face_with_a_driver`, `a_gltf_meshs_extras_say_which_joint_drives_a_shape`. |
| `crates/anim/src/correctives.rs` | `Correctives::driven` (a model's own driven shapes), `detect` takes them before the constants, `drive_own` (a part drives only its own). Three tests. |
| `crates/anim/src/animators.rs` | `follow` calls `drive_own` after the leader's weights by name and before the part's identity. |
| `crates/avatar/src/item.rs`, `lib.rs` | `ItemManifest::correctives: Vec<PoseCorrective>` (optional), `PoseCorrective {shape, bone, axis, angles}` and its check (a bone, a unit axis, `start < full`, unique shapes). |
| `crates/avatar/src/fit/cloth.rs` (new), `fit/mod.rs` | `cloth_rules`, `rule_weight`, `cloth_runs`, `posed_body`, `write_bake_input`, `fingerprint`, `same_garment`, `solve`, `PoseShapes`/`PoseShape`, `add_pose_shapes`, `add_to_item`, `rules_match`, `item_meshes`. |
| `crates/avatar/src/fit/pose.rs`, `glb.rs`, `import.rs` | `twist_angle`, `glb` and `base_of` made `pub(super)`; the import's manifest literal has `correctives: Vec::new()`. |
| `crates/avatar/examples/pose_correctives.rs` (new) | `dump`, `solve`, `add` from the command line. |
| `crates/avatar/tests/cloth_correctives.rs` (new), `src/fit/tests.rs`, `src/tests/validation.rs` | Tests (below). |
| `crates/avatar/tests/fixtures/test-tee/TestTee.correctives.json` (new, 600 KB) | The test tee's baked shapes (20 kept), for the gate's library with correctives. |
| `assets/characters/realistic/import/correctives.py` (new), `test_intake.py` | The intake step (dump, simulate, solve, add); two tests of its pin goal. |
| `crates/player/src/fit_gate_tests.rs` | `gate_library` split into `make_gate_library(correctives)`; `gate_library_with_correctives`; `gate_in` and `gate_walk_in` (the gate on a given library); `under_flipped_folds` (the tee-walk measure's flipped-fold count); `BRA_THROUGH_TEE` and `walk_records` `pub(super)`. The gate's own records are unchanged. |
| `crates/player/src/pose_corrective_tests.rs` (new), `main.rs` | 17 tests (below). |
| `crates/player/src/garment_checks.rs` | `realistic_cast` `pub(super)`. |
| `crates/player/src/live_identity_bench_tests.rs` | Bench modes `tee` and `tee-correctives`. |
| Docs | `architecture.md` (sections 7, 8, 9, 14, 16), ADR-0044 (new, Proposed) and the ADR index, `assets/characters/realistic/import/README.md`, `docs/rendering/animation.md`, `testing.md`, `references/clothing/README.md`, `wiki/Engine-Systems/Characters.md`, `wiki/Engine-Systems/Animation.md`, `wiki/Roadmap/Known-Limitations.md`, the `avatar`, `anim` and `assets` CLAUDE.md files. |
| `refinement/wave5/pose-correctives-red-run.txt`, `pose-correctives/*.jpg` | The red runs and the sheets. |

**Not changed:** the shipped licensed tee (`Shirt_TeeCrew.glb` and its manifest: measured with correctives and restored, below), every other generated file, every limit in `asset-quality.md`, every record of `fit_gate_tests`, `tee_crew_tests`, `wardrobe_tests` and `garment_checks`. No new dependency.

## Interfaces

- **Item manifest** (optional, no version bump): `correctives: [{shape, bone, axis, angles: [start, full]}]`.
- **glTF**: a garment mesh's `extras.axcenity_correctives`, the same list. `assets::MorphTarget::driver`, `assets::PoseDriver`.
- **`anim::correctives`**: `Correctives::driven`, `drive_own`. `detect` now includes a model's own driven shapes.
- **`avatar::fit`**: as listed under Files; sidecar format `PoseShapes` (format 1: `{format_version, fingerprint, rules, shapes: [{name, corners, offsets}], report}`, offsets in file space at rest, metres).
- **Shape names**: `Corrective_Cloth_<Joint>_<Kind>_<Side>`; the swings are in two halves, `Shoulder_ForwardHalf` (0 to 20 degrees) and `Shoulder_Forward` (20 to 40), `Shoulder_BackHalf` (0 to 25) and `Shoulder_Back` (25 to 50). The other shoulder rules are the body's (`Level` 0 to 45, `Raised` 45 to 105, `Lowered` 0 to 35). `Elbow_Bent` and `Knee_Bent` 0 to 90, `Hip_Forward` 0 to 30, `Hip_Back` 0 to 20, each about the axis across the limb.
- **`player` (tests)**: `fit_gate_tests::{gate_library_with_correctives, gate_in, gate_walk_in, under_flipped_folds}`.

## How the bake works

The bake runs on the base body (no identity).
1. **Dump** (Rust: the engine's own posing, the twist bones included). There are nine runs, each from rest:
   - the arms let down to hang;
   - the walk's swings on the hanging arms, either way round, each to half and then to full;
   - raised to level, and on to 105 degrees;
   - the elbows bent;
   - the thighs, one forward and one back, either way round;
   - the knees bent.

   Each move takes 16 frames and each pose is held 8 to 14 frames. Each run is read at rest, half way to each key pose (a check), and at each key pose (where the shapes are solved). Every frame's skin is posed with the body's own correctives, and the garment as skinning poses it today.
2. **Simulate** (bpy 5.3). The cloth is cotton:
   - no gravity: the bake corrects poses, not how the garment hangs;
   - a 2 mm skin distance, the fit's margin, and self-collision;
   - the body is a collider played through the frames by absolute shape keys;
   - the garment starts at rest with its seams welded, and its skinned place in each frame is the pin goal. The goal holds at 0.95 (Blender raises the weight to the fourth power) wherever skinning keeps the cloth's edges true, and lets go to 0.7 where it stretches or squeezes them by 12% or more, spread over four rings.

   The test tee takes 70 s for all runs and the licensed tee 3 min (`simulate.json`).
3. **Solve** (Rust). At each key pose the shape is the simulated cloth less the skinned one, less what the simulation moved at rest:
   - at most 6 mm a corner;
   - taken back to rest through the corner's skinning;
   - smoothed 12 rounds over the surface, with corners at one place treated as one;
   - split between the two sides' shapes across ±4 cm of the middle;
   - solved on top of the shapes before it.

   A shape that moves nothing over 0.5 mm is dropped. Then, with every shape solved, each half-way check poses the garment with all the shapes at their weights there and compares it with the simulation (`solve.json`). (Before review fixes 1 a check ran before the shape it is half way to was solved; see below.)
4. **Add** (Rust). The shapes go into every level of the model (a simpler copy's corner takes the full detail's offsets at its closest point) and into the manifest. A model whose rest corners have another fingerprint is refused ("bake again"), and so is one that already has the shapes.

## Tests (red first: `pose-correctives-red-run.txt`)

**Red.**
- **The contract and the runtime**, on stubs (the glTF rules not read, `driven` empty, the manifest check empty): 15 run, 8 passed, 7 failed.
  - The glTF driver, the cooked driver, the manifest check and the GLB write test all failed on the missing driver or check.
  - Both of the anim tests that drive a shape failed.
  - The runs test failed on its own float comparison (identical quaternions), which I fixed in the test.
  - `fit::cloth` itself was drafted before these tests ran, so its own tests were not seen red, except the GLB one, which needed the loader.
- **The player tests**, with every record at 0: 14 run, 5 passed, 9 failed. The failures gave the measured values the records now hold.

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-player --no-fail-fast -E 'test(/pose_corrective_tests::/)'` | **17 tests run: 17 passed** (4 slow) |
| `cargo nextest run -p axcenity-assets -p axcenity-anim -p axcenity-avatar -p axcenity-player --no-fail-fast` | **909 tests run: 908 passed** (28 slow), 1 failed, 23 skipped; no ABORT lines. The failure, `makeup_tests::lip_colour_changes_the_face_against_bare_skin` ("lips turned off changed the picture", a GPU picture comparison), passed alone (26 s): load from the parallel run |
| `cargo nextest run -p axcenity-avatar --no-fail-fast` (after the last clippy fix) | **211 tests run: 211 passed** (1 slow), 1 skipped |
| `python -I assets/characters/realistic/import/test_intake.py` (bpy) | **Ran 32 tests: OK** |
| `cargo clippy -p axcenity-assets -p axcenity-anim -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 116 pages, 0 errors (57 staleness warnings on other pages) |

## Measurements

### The test tee, the fit gate (54 bodies), without and with its correctives

| Case | Without: the gate's records | With: `pose_corrective_tests`' records | |
|---|---|---|---|
| Walk, first third: tee | 54, 13.6 mm | 54, 13.4 mm | lower |
| Walk, first third: bra through tee | 20, 10.8 mm | **21, 14.9 mm** | **higher** |
| Walk, second third: tee | 54, 16.6 mm | 54, 14.1 mm | lower |
| Walk, second third: bra | 20, 17.1 mm | 20, 10.4 mm | lower |
| Walk, last third: tee | 54, 14.7 mm | 54, 14.5 mm | lower |
| Walk, last third: bra | 20, 13.1 mm | 20, 11.5 mm | lower |
| Walk@18: tee | 38, 10.3 mm | **39**, 9.0 mm | **one body more** |
| Walk@18: bra | 6, 15.2 mm | **7**, 10.2 mm | **one body more** |
| Arms forward: tee | 44, 9.4 mm | 43, 7.2 mm | lower |
| Arms forward: bra | 15, 3.6 mm | 6, 2.9 mm | lower |
| Arms raised: tee | 1, 2.9 mm | 1, 2.5 mm | lower |
| Arms raised: bra | none | **1, 2.3 mm** (feminine heavy-soft) | **new** |
| Rest, T-pose, elbows and knees bent | none | none | |

Two more measures:
- **Corners out in the stride** (each body's worst frame, added up, per third): 259, 237 and 249 without; 207, 239 and 169 with.
- **The stride's armpit folds** (`under_flipped_folds`): body corners the gate counts out whose way out meets a fold flipped towards the skin, each body's most over a third's frames, added over the 54 bodies. Without: 147, 199, 161. With: 122, 136, 139. Both are exact records in `pose_corrective_tests`, and the test holds "with" below "without".

**The motion check** (the cast, the walk and the run):
- With the correctives the tee jolts at most 4.06 mm a frame in the walk (frame 34) and 11.65 mm in the run (frame 28).
- The records without them are 4.1 and 11.7 mm. The correctives add no popping; Idle has none.

**What the bake measured** (`solve.json`):
- **Shapes:** 20 kept, each moving at most 2.6 to 6.3 mm (the 6 mm limit). The knees moved nothing over 0.5 mm and were dropped.
- **The half-way checks** (every solved shape blended at its weight there, against the simulation; re-solved in review fixes 1): median 0.0 to 1.0 mm. The 95th percentile is 3.8 mm for hanging, 14.2 to 15.9 mm for the swings, 7.7 mm for level and 24.0 mm for raised. The worst corners are off by 24 to 103 mm. Table under review fixes 1. These mix three losses (the hold, the smoothing and blending); review fixes 2 separates them, with the key poses' own figures beside them.
- **Shapes over 6 mm:** the 6 mm limit holds a corner's move in the key pose; the stored shape is that divided by the rule's weight there (at least 0.5) and shared between the sides, so a stored offset can be a little over it (the most is 6.3 mm).
- **Inside the skin:** the simulation's cloth ends up to 15 mm inside the skin at some swing and hang reads (`simulate.json`, up to 24 corners). That is where the arm overlaps the side.

### What I tried, on the test tee (each a full gate run with the same measure)

Columns: the walk thirds' worst; Walk@18 (bodies, worst); arms forward; arms raised; T-pose; flipped-fold counts.

| Bake | Walk thirds | Walk@18 | Arms fwd | Arms raised | T-pose | Flipped |
|---|---|---|---|---|---|---|
| None (the gate's records) | 13.6 / 16.6 / 14.7 | 38, 10.3 | 44, 9.4 | 1, 2.9 | none | 147 / 199 / 161 |
| Gravity, free pins at 0.3, whole moves | 23.5 / 28.4 / 24.8 | 54, 20.5 | 54, 26.7 | 54, 46.8 | 52, 21.5 | 144 / 142 / 142 |
| No gravity, free 0.5 | 15.8 / 24.7 / 15.9 | 54, 11.5 | 54, 8.0 | 41, 10.2 | none | 136 / 127 / 131 |
| No gravity, free 0.7 | 21.6 / 21.0 / 19.6 | 38, 11.5 | 49, 8.9 | 1, 2.2 | 1, 2.1 | 150 / 131 / 170 |
| Free 0.5, 4 mm skin distance | 19.9 / 13.8 / 17.7 | 50, 13.0 | 54, 15.1 | 42, 10.3 | none | 126 / 112 / 154 |
| Swings in two halves, free 0.7 (the sims kept) | 21.5 / 12.6 / 15.1 | 43, 12.6 | 49, 5.1 | 1, 2.1 | 1, 2.1 | 122 / 116 / 162 |
| ... only the move out along the skin's normal | 11.6 / 20.0 / 12.3 | 49, 13.0 | 49, 9.1 | 1, 2.7 | none | 128 / 166 / 123 |
| ... whole moves held to 10 mm | 14.6 / 13.8 / 15.5 | 36, 8.4 | 42, 11.8 | 1, 2.1 | none | 129 / 147 / 117 |
| ... moves out held to 10 mm | 12.0 / 14.7 / 13.4 | 49, 7.6 | 51, 7.5 | 1, 2.7 | none | 135 / 149 / 131 |
| **... whole moves held to 6 mm (committed)** | 13.4 / 14.1 / 14.5 | 39, 9.0 | 43, 7.2 | 1, 2.5 | none | 122 / 136 / 139 |
| ... moves held to 6 mm, no move inwards | 17.5 / 15.2 / 25.2 | 40, 10.0 | 50, 6.7 | 1, 2.3 | none | 151 / 142 / 125 |
| ... whole moves held to 4 mm | 12.6 / 16.6 / 13.5 | 49, 9.3 | 49, 7.8 | 1, 2.6 | none | 122 / 164 / 140 |

**What this means.** The results move a few millimetres and bodies either way from one setting to the next. No setting improves every record at once: each one trades the bra's cases or the body counts against the worsts. With the 6 mm limit the bake helps broadly and does no harm in the held poses, but it does not dominate. The experiments' code beyond the committed settings is not kept (they were environment switches in the same files).

### The licensed tee (`axcenity/realistic-tee-crew`), with its correctives, against its records

I baked it (`correctives.py` on the shipped library: 22 shapes, 2,407 corners, three levels, seams welded; the simulation moved it 15 to 17 mm at rest, its hems' turn-ups settling), added the shapes to the shipped model in the worktree, ran `tee_crew_tests` and then restored the files (`git checkout`). The result was 14 tests run: 4 passed, 10 failed.

| Case | Record (without) | With |
|---|---|---|
| T-pose: tee | 7, 28.2 mm | **53, 28.2 mm** |
| Arms raised: tee | 14, 28.2 mm | **54, 28.2 mm** |
| Arms raised: bra | 1, 3.1 mm | **4, 5.3 mm** |
| Idle: tee | 53, 38.0 mm | **54, 40.4 mm** |
| Idle: bra | 17, 4.1 mm | 5, 3.2 mm |
| Walk thirds: tee | 32.5 / 31.3 / 32.0 mm | **36.2 / 38.6 / 36.1 mm** |
| Walk, second third: boxer briefs | 26, 2.4 mm | 16, **7.9 mm** |
| Walk, second third: briefs | 13, 22.7 mm | 12, **27.9 mm** |
| Arms forward: tee | 54, 30.2 mm | 54, 27.7 mm |
| Arms forward: bra | 19, 3.2 mm | **21, 5.8 mm** |
| Rest, elbows and knees bent | as recorded | as recorded |

The licensed tee doesn't carry its correctives: they make it worse.

### The frame budget (`bench_distinct_characters`, 50 characters, release, 1280 × 720 High; two runs each, while other agents were building)

Each of the 50 characters has a random description and wears the test tee, without (`tee`) and with (`tee-correctives`) its correctives.

| | Without | With |
|---|---|---|
| Active shapes | 17,929 | 18,329 (+400: about 8 per character idling) |
| Shape moves | 5.648 M | 5.767 M (+2.1%) |
| GPU character posing | 0.496, 0.481 ms | 0.512, 0.498 ms |
| GPU frame | 2.82, 2.78 ms | 3.15, 2.77 ms |
| CPU `Animators::update` | 9.3, 6.7 ms | 5.7, 24.4 ms |

- The CPU times swing with the machine's load. The drive itself is about 20 weight computations per garment a frame.
- The posing buffers are the same (48.56 MB).
- Command: `AXCENITY_BENCH_COUNTS=50 AXCENITY_BENCH_MODES=tee,tee-correctives,tee,tee-correctives cargo test -p axcenity-player --release -- --ignored bench_distinct_characters --nocapture --test-threads 1`.

## Visual evidence (`pose-correctives/`)

These are in-engine captures (release, studio light, 2560 × 1440) of the gate's test tee over the underwear, on four cast bodies: masculine average and heavyset, feminine average and heavyset. Each is shown before (the gate's library) and after (the library with correctives).

**The poses:**
- `Walk@0.183` (first third: the near arm forward);
- `Walk@0.367` (second third);
- `Walk@0.633` (last third: the near arm back);
- `arms-raised`;
- `arms-forward`.

**The sheets:**
- `test-tee-<pose>.jpg` (2700 × 1360): one body a row. Columns: front before, front after, side before, side after, back before, back after. Each tile is 900 × 680 px of the capture drawn at 450 × 340.
- `armpit-<pose>.jpg` (1800 × 1800): side and back, before and after. Each tile is 300 px of the capture drawn at 450 (400 or 560 px for the arms raised).
- `armpit-walk-0-633-3x.jpg`: 150 px at 3×, one tile each for masculine average, feminine average and feminine heavyset, before then after.
- `armpit-walk-0-367-3x.jpg` (review fixes 1): the side view's armpit at Walk@0.367, 150 px of the capture (x 1150 to 1300, y 455 to 605) at 3× (nearest), one row a body, before then after.
- `armpit-front-<pose>.jpg` (review fixes 1): the front view's chest and both armpits, before and after, 520 × 320 px of the capture (560 × 345 for the arms raised) drawn at 1.5×, one row a body.
- `bake-hang-armpit-skinned-vs-simulated.jpg`: Blender renders from the bake itself, not the engine. The left armpit with the arms hanging, front then side, the skinned goal (blue) beside the simulated cloth (orange).

Sizes are measured in sheet pixels at about 25 capture px a centimetre (±30%).

**What I saw:**
- **The correctives change little you can see.** At the sheet's scale every before and after pair is nearly the same picture. That is expected with moves of at most 6 mm.
- **Walk, the near arm back** (`armpit-walk-0-633-3x.jpg`):
  - the slit of skin at the front of the armpit on masculine average is there before (about 6 × 5 mm, at 165 to 210, 135 to 175) and after (the same size, at 595 to 640, 135 to 175, its edges shifted);
  - the black bra triangle on feminine heavyset is about 5 × 4 mm before (1915 to 1955, 170 to 200) and a little lower and flatter after (2350 to 2395, 170 to 195);
  - feminine average shows no hole in either.
- **Walk, the second third** (`armpit-walk-0-367.jpg`, `armpit-walk-0-367-3x.jpg`; added in review fixes 1, measured in the captures at 2.5 px a millimetre, ±30%). Each defect is in the side view, at the front of the armpit, where the arm swung back meets the side:
  - **feminine average, a black bra triangle** about 1.6 × 1.8 mm before (capture x 1201 to 1204, y 526 to 530; 10 pixels under a luminance of 40). **After, it is gone** (no pixel under 40 in the armpit window). It is an improvement, but a small one: it is one triangle of the bra.
  - **feminine average, a light speck** about 2.2 × 0.8 mm (capture x 1224 to 1229, y 540 to 542) before, and about 2.0 × 0.6 mm and fainter after (x 1220 to 1225). **It is not a see-through gap to the background.** Its brightest pixel is (69, 78, 92) before and (66, 75, 89) after. That is blue over red by 23, the tee's own lit colour (119, 130, 146 on the open cloth). The studio background behind the body is neutral and brighter (131, 134, 140). So the speck is tee cloth, a fold's far side lit through a crease in the near fold. That is the flipped fold the gate counts, not a hole through the garment.
  - **feminine heavyset, a black bra triangle** about 7 × 3.5 mm before (x 1208 to 1225, y 529 to 537; 78 pixels under a luminance of 40) and 6.4 × 2.8 mm after (x 1208 to 1223, y 530 to 536; 47 pixels). It is still there, a little smaller.
  - **feminine heavyset, a light sliver** below that triangle, about 2.4 × 1.4 mm before (x 1207 to 1213, y 539 to 543; brightest (114, 125, 139)) and 2.8 × 0.8 mm and darker after (brightest (81, 90, 103)). By colour it is tee cloth in a crease, not background, as on feminine average.
  - **both masculine bodies, a pinched cone** at the top of the armpit's fold, before and after:
    - masculine average: a dark tip about 2 × 2 mm (x 1221 to 1226, y 475 to 481), unchanged after;
    - masculine heavyset: a cone about 15 × 13 mm whose facet edges show as light ridge lines. The lines are a little softer after. These are the flipped folds `under_flipped_folds` counts.
  - **back view, all four bodies:** the small cloth spikes at the back of the near shoulder (`tee-walk.md`), before and after the same. On the masculine bodies the boxer briefs show under a hem drawn up into the seat (the shrink-wrapped fit's bodysuit look, `tee-walk.md`), unchanged.
- **Front views** (`armpit-front-<pose>.jpg`, review fixes 1):
  - **every pose, before and after alike:** the collar's edge is wavy by a few millimetres (`TestTee` README), and a sharp lit ridge crosses the chest above the bust (about 6 cm), the cloth bridging between the chest's two sides.
  - **arms forward:** a dark flat flap stands out beside the torso under each sleeve on every body (about 12 × 16 mm, masculine average at capture x 1123 to 1153, y 427 to 467), the same before and after.
  - **arms forward, feminine average: a stippled strip that the correctives make worse.** A checkered dotting lies on the lit slope beside the near sleeve. Before it is faint, about 4 × 6 mm (capture x 1322 to 1331, y 407 to 422). After it is plain, about 7 × 25 mm (x 1292 to 1310, y 395 to 457).
    - Counted as pixels whose luminance differs from their 3 × 3 mean by more than 4 (capture x 1285 to 1340, y 380 to 470): 123 before, 352 after. The other bodies are 111 to 119 (feminine heavyset, both) and 19 to 35 (masculine).
    - The dots are tee-coloured (mean 89, 97, 111), not the near-black bra. So this is the tee shading itself, not the bra through it. My reading: either shadow acne where the corrective lays the sleeve's underside at a grazing angle, or two layers of the tee nearly touching.
    - It is a visible defect the correctives add; arms forward isn't in the stride.
  - **arms raised:** the crumpled folds and side flaps are the same shapes, their edges shifted by 2 to 4 mm. On both feminine bodies, the back view's flap has a row of small dark dots along its inner edge (about 12 mm), before and after.
  - **the walk's three thirds:** the armpits are behind the arms; nothing new.
- **Walk, the near arm forward** (`armpit-walk-0-183.jpg`): the arm covers the armpit in the side view. The back view shows nothing new: no spike, flap or hole added.
- **Arms raised** (`armpit-arms-raised.jpg`):
  - the large dark triangular flaps under the arm on both feminine bodies (about 2.5 × 5 cm: feminine average at 255 to 330, 990 to 1130 before and 705 to 780 after) are the same;
  - the crumpled vertical folds under the arm on the masculine bodies are the same.
- **Arms forward** (`armpit-arms-forward.jpg`, `test-tee-arms-forward.jpg`): no visible difference at this scale. The gate's 43 bodies (from 44) and 7.2 mm (from 9.4) are below what the sheet resolves.
- **The bake's own simulation** (`bake-hang-armpit-skinned-vs-simulated.jpg`):
  - the skinned goal is smooth from outside;
  - the simulated cloth is ragged at the armpit: torn-looking triangles where the cloth is caught between the arm and the side, whose skins overlap, and fine crinkles over the shoulder and the back.

  The 6 mm limit and the smoothing keep most of that out of the shapes. Most of it is the bridge the cloth can't go through.
- **What the gate doesn't count, unchanged:** the shoulder-seam tab and the cloth spikes at the back of the armpit in the walk (`tee-walk.md`).

## References (Openverse, decision 14)

| What | Openverse | Source | Author | Licence |
|---|---|---|---|---|
| A plain grey crew-neck tee on a man with the arms hanging (three-quarter view): the sleeve hangs loose off the upper arm and the underarm makes one soft fold, with no tight bridge to the side. Used to judge the armpit. | https://openverse.org/image/0828727f-4725-4c4f-a024-c1664c20e6bf | https://www.flickr.com/photos/45776673@N04/19837504423 | Steven Pisano | CC BY 2.0 |

- The picture is saved outside the repository (`axcenity-w5-sources/references/pose-correctives/`) and is also listed in `references/clothing/README.md`.
- Openverse's results for walking, raised arms and stretching were mostly unrelated: flags, events, a product shot with a brand logo. I looked at four more pictures, used none and deleted them.
- The reference shows what the test tee lacks: room in the sleeve and an underarm that hangs. That is the fit and the pattern's business (the drape-preserving fit track), not a pose corrective's.

## Open issues

1. **The main goal is not met.**
   - The stride's armpit folds still flip (397 corners under a flipped fold over the three thirds, from 507), and the slit, the bra triangle and the arms-raised flaps look the same.
   - The cause: a cloth simulation can't put cloth between a hanging arm and the side where the body's own skin overlaps. The bridge moves up to 12 cm (the worst corner at each shoulder key pose: 49 to 116 mm) and is crumpled at a triangle's scale. Three things lose that move, measured on the kept bake (review fixes 2):
     - **blending between key poses:** shapes that meet every key pose exactly (whole, unsmoothed) are off half way through the swings by a 95th percentile of 9.0 to 16.1 mm, where the cloth itself moves 16.4 to 18.5 mm, and by 35 to 51 mm at the worst corners. Half way to the hanging arms the blend (95th percentile 9.6 mm) is further from the cloth than no shape at all (4.0 mm). Raised and level do blend (3.8 and 2.1 mm, where the cloth moves 30.9 and 9.6 mm);
     - **the smoothing** (12 rounds) irons out the crumpling: whole smoothed moves miss the shoulder key poses by a 95th percentile of 6.5 to 12.9 mm;
     - **the 6 mm hold**, which the gate sweep chose ("What I tried"), not blending, adds 1.3 to 4.3 mm to the 95th percentile at the hang's and the swings' key poses, 8.1 mm at level and 21.7 mm at raised.
     - So on the stride's poses (the hang and the swings) blending alone leaves errors half way (9 to 16 mm) of the same size as the hold and the smoothing leave at the key poses (13.5 to 16.7 mm). No hold or smoothing setting would remove it: it needs more key poses or a blend that isn't linear (below).
   - What would help, each for the lead:
     - the drape-preserving fit and a garment with room under the arm (the parallel fit track);
     - the body's own armpit overlap fixed, with a corrective for the arm against the side;
     - more key poses (quarters of the swing) and a runtime blend that isn't linear (radial basis pose-space deformation);
     - or simulating cloth at run time near the armpit only.
2. **The correctives aren't in the gate.** They improve 10 of the gate's cases for the test tee and worsen 4 (the bra in the first third; one body more at Walk@18 for the tee and the bra; a new bra case with the arms raised). Taking them in means raising those four records, which is the lead's call. Until then `pose_corrective_tests` holds the "with" numbers exactly.
3. **The licensed tee is worse with them** (the T-pose 7 to 53 bodies, the arms raised 14 to 54, the walk 32 to 39 mm), so it doesn't carry them.
4. **Baked for one fit.** Any refit, including the drape-preserving fit track's, makes `TestTee.correctives.json` stale. `add_pose_shapes` then refuses it, and every `pose_corrective_tests` case fails with "bake again". The fingerprint catches a refit that moves the garment as a whole (a mean or spread change of 5 µm), not a single corner moved.
5. **Baked on the base body only** and added to every body's identity shapes. A feminine or heavy body gets the masculine average's correction.
6. **The bake's repeatability is not checked.** I didn't run one setting twice, and Blender's cloth is sensitive: settings a step apart moved the swings' worsts by several millimetres (the table above). The committed sidecar is the one measured; a re-bake on another machine or Blender version must be measured again.
7. ~~The engine doesn't check the manifest against the model.~~ Fixed in review fixes 1: the shipped-library check calls `rules_match` for every item, and `check_model` checks each rule's shape and bone.

## At merge

- **Nothing to regenerate.** No shipped item changed.
- **ADR-0044 is new (Proposed).** If another branch also took 0044, renumber one.
- **`fit_gate_tests.rs`:**
  - `gate_library` now calls `make_gate_library(None)`; a branch editing the library builder must follow the split;
  - `gate`, `gate_walk` and their records are untouched;
  - `gate_in`, `gate_walk_in`, `walk_records`, `BRA_THROUGH_TEE` and `under_flipped_folds` are `pub(super)`.
- **After the drape-preserving fit merges:**
  1. rebuild a gate library (run the gate once);
  2. copy `target/<profile>/fit-gate/<newest>` to a scratch folder;
  3. `python -I assets/characters/realistic/import/correctives.py --library <that folder> --item imported/test-tee --work <scratch> --out crates/avatar/tests/fixtures/test-tee/TestTee.correctives.json --only dump,simulate,solve` (about 70 s);
  4. re-measure `pose_corrective_tests` and lower or re-set its records. They are records of this bake, not of the gate.
- **`MorphTarget` has a new field (`driver`).** A branch constructing `MorphTarget` literals adds `driver: None`.
- **Your decisions:**
  - whether the gate's test tee takes its correctives (open issue 2);
  - the route for the armpit (open issue 1);
  - ADR-0044.

## Review fixes 1

The review found three blocking problems. All three are fixed. No record changed, and the shapes are byte-for-byte the same.

### 1. The half-way check ran before the shape it checks

**The bug.** `solve` took a run's reads in order. Each run's check (half way) comes before its key pose (where the shape is solved), so a check posed the garment only with the shapes solved earlier. Most checks therefore measured the simulation against skinning plus earlier shapes, not the blend. The reviewer measured 0.0001 mm where the blend is 2.24 mm off.

**Red first.** Two tests in `tests/cloth_correctives.rs` (`pose-correctives-red-run.txt`):
- `a_simulated_move_comes_back_as_its_poses_shapes` now also asserts that a simulation which doesn't move half way gives a check of the rule's weight times the move: 0.5 × 4.47 = 2.236 mm. **It failed:** median 0.0000, most 0.0001.
- `the_half_way_check_measures_the_blend_of_the_solved_shapes` (new): a simulation that moves the cloth by the weight times the move half way must give about 0 mm. **It failed:** median 2.2359, most 2.2361.

**The fix** (`fit::cloth::solve`): two passes.
- The first pass solves every shape in run order, each on top of those before, and keeps each run's move at rest.
- The second pass takes each check with the complete set of shapes. It poses the garment with every shape at its rule's weight there and compares that with the simulated cloth, less the move at rest.

Both tests pass, and so do the other two in the file.

**Re-solved** on the kept work folder (`--only solve`: `cargo run -p axcenity-avatar --release --example pose_correctives -- solve <the gate library the bake used> imported/test-tee <work> <out>`).
- The shapes, rules and fingerprint are identical to the committed sidecar's, and so are the per-shape figures. Only the report's `checks` changed.
- The new report is in `TestTee.correctives.json`. Each cell gives median / 95th percentile / most, in mm:

| Check | Before the fix (skinning plus earlier shapes) | After (the blend of every shape) |
|---|---|---|
| hang, frame 33 | 0.25 / 3.98 / 25.4 | 0.27 / 3.83 / 24.7 |
| swing-left, 55 (half of the first half) | 0.65 / 15.98 / 83.4 | 0.52 / 15.89 / 82.9 |
| swing-left, 109 | 0.89 / 16.37 / 103.2 | 0.74 / 15.69 / 103.2 |
| swing-right, 55 | 0.65 / 15.70 / 82.5 | 0.52 / 15.64 / 81.8 |
| swing-right, 109 | 0.87 / 15.05 / 85.7 | 0.73 / 14.18 / 85.3 |
| level, 33 | 0.67 / 9.61 / 25.1 | 0.39 / 7.72 / 24.2 |
| raised, 53 | 1.49 / 26.69 / 74.5 | 0.98 / 23.96 / 73.1 |
| elbows, 33 | 0.00 / 0.31 / 3.1 | 0.00 / 0.25 / 2.6 |
| hips-left, 33 | 0.04 / 2.55 / 8.7 | 0.04 / 2.18 / 7.2 |
| hips-right, 33 | 0.04 / 2.62 / 8.6 | 0.04 / 2.21 / 7.3 |
| knees, 33 | 0.00 / 0.02 / 0.6 | 0.00 / 0.02 / 0.6 |

**What this means.** With the blend included, every check is a little closer to the simulation, but only a little. The 95th percentiles drop by 0.1 to 2.7 mm, and the worst corners are still 24 to 103 mm off. Away from the armpit (the elbows, hips and knees) the blend is within 2.2 mm at the 95th percentile. *(Corrected in review fixes 2: these figures are the committed bake's, whose shapes are held to 6 mm and smoothed, so they can't say how much of the gap comes from blending. The conclusion this paragraph drew, that no blend follows the cloth at the armpit, was not measured here; review fixes 2 measures it.)*

### 2. Visible defects at Walk@0.367 not reported

They are now in **What I saw**, under "Walk, the second third", with places, sizes and before against after. There is a new 3× crop, `armpit-walk-0-367-3x.jpg`.
- **Feminine average:** the bra triangle (about 1.6 × 1.8 mm) is gone after the correctives.
- **The light specks are not gaps to the background.** Their colour is the tee's (blue over red by about 23), not the neutral background, so they are tee cloth seen in a crease.
- **Feminine heavyset:** the bra triangle shrinks from about 7 × 3.5 mm to 6.4 × 2.8 mm.

While checking every capture pair for differences (the largest 32-pixel cell differences, per pose, body and view), I also found **a defect the correctives add**. At arms forward on feminine average, a stippled strip beside the near sleeve grows from 123 to 352 pixels (about 7 × 25 mm). It is reported under "Front views".

The reviewer's non-blocking point about close crops: the front view's armpits are now in `armpit-front-<pose>.jpg`, for all five poses.

### 3. The docs claimed that `rules_match` enforces the match, but nothing called it

Now something does.
- **`fit::rules_match`** takes all of an item's models (`impl IntoIterator<Item = &Model>`). Every rule must have a driven shape there, every driven shape a rule, and a shape driven in several meshes must be driven the same in each.
- **`tests/shipped_library.rs` `every_items_pose_correctives_are_its_models`** calls it for every shipped item. Today that is a no-op, because no item has correctives.
  - To check it bites, I temporarily gave `Shirt_TeeCrew.item.json` a `correctives` entry. The test then failed with "axcenity/realistic-tee-crew: the model has no driven shape `Corrective_Cloth_Shoulder_Lowered_Left`".
  - `every_manifest_entry_names_shapes_bones_and_materials_its_model_has` failed too. I restored the file with `git checkout`.
- **`ItemManifest::check_model`** now reports a corrective whose shape isn't the item's, or whose bone isn't the body's (`tests::models::an_items_pose_correctives_name_its_shapes_and_the_bodys_bones`, seen red first). That also covers the reviewer's non-blocking point that a misspelt bone was skipped silently at run time.
- **architecture.md** section 8 and the `fit` listing, **ADR-0044** and `testing.md` now say this. Open issue 7 is closed.

### Non-blocking points also taken

- **`add_pose_shapes`:** a simpler copy's corner with no full-detail surface within 5 cm now takes no offset. Before, it took corner 0's offsets. The logic moved into `lower_detail_offsets`, tested by `fit::tests::a_simpler_copys_corner_far_from_the_full_detail_takes_no_corrective`, which was seen red against the old fallback. The test tee's levels have no such corner, so its records are unchanged (the player run below).
- **"At most 6 mm" is now worded exactly:** the limit is on the move in the key pose, and a stored offset can be up to 6.3 mm (under "What the bake measured").

**Not taken (left for the lead):**
- caching `Correctives::driven` per model key;
- a CPU bench of the walk;
- a synthetic test of `under_flipped_folds` alone.

### Tests

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-avatar --no-fail-fast -E 'binary(cloth_correctives)'` (red, before the fix) | 4 tests run: 2 passed, 2 failed |
| `cargo nextest run -p axcenity-avatar --no-fail-fast -E 'test(an_items_pose_correctives_name_its_shapes_and_the_bodys_bones) or ...'` (red) | 5 tests run: 4 passed, 1 failed |
| `... -E 'test(a_simpler_copys_corner_far_from_the_full_detail_takes_no_corrective)'` (red, old fallback put back) | 1 test run: 0 passed, 1 failed |
| `cargo nextest run -p axcenity-avatar --no-fail-fast` | **215 tests run: 215 passed** (2 slow), 1 skipped |
| `cargo nextest run -p axcenity-avatar -p axcenity-gameplay -p axcenity-player --no-fail-fast` | **541 tests run: 541 passed** (25 slow), 23 skipped; no ABORT lines (`pose_corrective_tests`, the gate, `tee_crew_tests`, `wardrobe_tests` and `garment_checks` all pass on their records) |
| `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 0 errors |

**Files changed in this fix:**
- `crates/avatar/src/fit/cloth.rs`: `solve` in two passes, `lower_detail_offsets`, `rules_match` over all of an item's models.
- `crates/avatar/src/model_check.rs`.
- Tests: `crates/avatar/src/fit/tests.rs`, `src/tests/models.rs`, `tests/cloth_correctives.rs`, `tests/shipped_library.rs`.
- `crates/avatar/tests/fixtures/test-tee/TestTee.correctives.json`: its report only.
- Docs: `assets/characters/realistic/import/correctives.py` (its docstring) and `README.md`, `architecture.md`, `testing.md`, ADR-0044, this hand-back and its red-run file.
- Sheets: `pose-correctives/armpit-walk-0-367-3x.jpg` and `armpit-front-*.jpg` (5).

## Review fixes 2

The second review found one blocking problem: the stated cause, "a blend of shapes can't follow the bridge between key poses, so the shapes are held to 6 mm", rested on a figure from the buggy half-way check ("up to 10 cm" in `MOST_MOVE`'s comment). Review fixes 1 re-solved only the committed bake, whose shapes are held to 6 mm and smoothed, so its half-way figures mix three losses and can't show which one is blending's. The reviewer was right. This fix measures each loss on its own and corrects every sentence that stated the cause. No shape, no sidecar shape data and no record changed.

### What changed in the solve

- **The report now has the key poses' own figures** (`keys`) beside the half-way `checks`: the garment posed with every solved shape at its weight there, against the simulated cloth, computed the same way. Each entry also gives how far the simulation moved the cloth from skinning with no shape (`moved_p95_mm`, `moved_most_mm`), and the report states the hold (`hold_mm`).
- **`solve_with(.., Solving)`** (`fit::cloth`): `Solving { hold, smooth_rounds }`. `solve` is `solve_with(.., Solving::SIDECAR)` (held to `MOST_MOVE`, smoothed `SMOOTH_ROUNDS` times), so the sidecar is solved exactly as before. `Solving::whole(rounds)` takes each key pose's move whole, for measuring only.
- **`pose_correctives -- solve-whole <library> <ID> <work> <report.json> [rounds]`** writes only the report, never a sidecar. With 0 rounds every key pose is met exactly (0.0 mm, below), so whatever is left half way is what blending between key poses loses, and nothing else.

**Red first** (`pose-correctives-red-run.txt`, `tests/cloth_correctives.rs`):
- `the_report_measures_what_the_shapes_leave_at_each_key_pose` (a 4.47 mm move, under the hold, is met at the key pose) and `a_whole_move_solve_separates_the_blend_from_the_hold` (a 22.4 mm move: held, the key pose keeps 16.4 mm and half way about half of it, 8.18 mm; solved whole, both are met; solved whole against a cloth that doesn't move half way, off by the weight's share of 22.4 mm). **Both failed** on a stub without `keys`: 6 tests run, 4 passed, 2 failed.
- `an_unsmoothed_whole_solve_meets_each_key_pose` (a 4 mm checkerboard crumple in 5 mm cells: smoothed, the key pose keeps over 2 mm; whole and unsmoothed, the key pose and half way are met). **It failed** on a stub that ignored the rounds ("the key pose keeps 3.7879 / 6.2642 mm"): 7 tests run, 6 passed, 1 failed.
- All 7 pass after the change.

**Re-solved on the kept bake** (the same work folder and gate library as review fixes 1):
- `pose_correctives -- solve`: shapes, rules and fingerprint identical to the committed sidecar's, and the half-way checks identical to review fixes 1's table. `TestTee.correctives.json` changed only its report (it gained `keys`, `moved_*` and `hold_mm`).
- `pose_correctives -- solve-whole ... 12` (whole, smoothed as the sidecar is) and `... 0` (whole, unsmoothed). I first ran the unsmoothed case with a temporary edit of `SMOOTH_ROUNDS` (restored at once); the command reproduces it exactly.

### The three losses, separated (the test tee's kept bake)

Each cell is the 95th percentile / the worst corner, in mm, of the garment as the runtime would pose it against the simulated cloth (less the move at rest). "The cloth's own move" is the same with no shape, so a figure above it means the shapes make that place worse.

**Half way** (95th percentile / most, mm):

| Run, frame | The cloth's own move (no shape) | Committed (held to 6 mm, smoothed) | Whole, smoothed | Whole, unsmoothed |
|---|---|---|---|---|
| hang, 33 | 4.0 / 25.4 | 3.8 / 24.6 | 5.7 / 22.8 | 9.6 / 39.5 |
| swing-left, 55 | 17.0 / 85.7 | 15.9 / 82.9 | 14.4 / 71.5 | 15.9 / 50.8 |
| swing-left, 109 | 18.5 / 103.2 | 15.7 / 103.1 | 13.3 / 99.4 | 10.7 / 45.1 |
| swing-right, 55 | 16.4 / 84.3 | 15.6 / 81.8 | 14.3 / 69.1 | 16.1 / 47.0 |
| swing-right, 109 | 17.3 / 88.4 | 14.2 / 85.3 | 11.9 / 72.1 | 9.0 / 35.0 |
| level, 33 | 9.6 / 25.1 | 7.7 / 24.2 | 3.9 / 22.3 | 2.1 / 6.6 |
| raised, 53 | 30.9 / 76.6 | 24.0 / 73.1 | 8.6 / 63.2 | 3.8 / 8.6 |
| elbows, 33 | 0.3 / 3.1 | 0.2 / 2.6 | 0.3 / 3.2 | 0.2 / 8.6 |
| hips-left, 33 | 2.6 / 8.6 | 2.2 / 7.2 | 2.1 / 6.4 | 0.9 / 5.0 |
| hips-right, 33 | 2.6 / 8.6 | 2.2 / 7.3 | 2.1 / 6.5 | 0.8 / 4.8 |
| knees, 33 | 0.0 / 0.6 | 0.0 / 0.6 | 0.0 / 0.6 | 0.0 / 0.3 |

**At the key pose** (95th percentile / most, mm):

| Run, frame | The cloth's own move (no shape) | Committed (held to 6 mm, smoothed) | Whole, smoothed | Whole, unsmoothed |
|---|---|---|---|---|
| hang, 63 | 19.1 / 97.7 | 16.7 / 95.2 | 12.9 / 77.3 | 0.0 / 0.0 |
| swing-left, 85 | 16.2 / 80.2 | 13.8 / 76.8 | 11.7 / 62.0 | 0.0 / 0.0 |
| swing-left, 139 | 20.6 / 98.0 | 15.8 / 96.4 | 11.5 / 85.5 | 0.0 / 0.0 |
| swing-right, 85 | 16.2 / 72.4 | 13.5 / 69.3 | 12.2 / 60.7 | 0.0 / 0.0 |
| swing-right, 139 | 20.6 / 115.5 | 15.5 / 115.8 | 11.3 / 107.5 | 0.0 / 0.0 |
| level, 63 | 19.0 / 48.7 | 14.6 / 46.6 | 6.5 / 42.2 | 0.0 / 0.0 |
| raised, 83 | 39.4 / 98.5 | 30.6 / 93.2 | 8.9 / 73.9 | 0.0 / 0.0 |
| elbows, 63 | 0.7 / 14.1 | 0.5 / 13.1 | 0.5 / 11.7 | 0.0 / 0.0 |
| hips-left, 63 | 5.0 / 18.6 | 3.8 / 14.7 | 3.6 / 12.1 | 0.0 / 0.0 |
| hips-right, 63 | 5.1 / 16.7 | 3.9 / 13.2 | 3.8 / 11.7 | 0.0 / 0.0 |
| knees, 63 | 0.0 / 0.6 | 0.0 / 0.6 | 0.0 / 0.6 | 0.0 / 0.0 |

**What the numbers show.**
- **Blending between key poses does lose the stride's armpit, measured without the hold or the smoothing.** Whole and unsmoothed, every key pose is met exactly, yet half way through the swings the blend is off by a 95th percentile of 9.0 to 16.1 mm, where the cloth itself moves 16.4 to 18.5 mm, and by 35 to 51 mm at the worst corners. At a quarter of the swing (frame 55, half way to the swing's first half) the linear blend barely helps (15.9 and 16.1 mm against 17.0 and 16.4 mm): the cloth there doesn't move like half of the half-swing's move. Half way to the hanging arms the blend is worse than no shape (9.6 mm against 4.0 mm; worst 39.5 against 25.4 mm), and so is the elbows' worst corner (8.6 against 3.1 mm).
- **Elsewhere blending follows well:** raising the arms (3.8 mm against a move of 30.9 mm), level (2.1 against 9.6 mm) and the hips (0.8 to 0.9 against 2.6 mm).
- **The smoothing loses a lot at the key poses themselves.** Whole and smoothed, the shoulder key poses are still off by a 95th percentile of 6.5 to 12.9 mm, and by 42 to 107.5 mm at the worst corners. The simulated move at the armpit is crumpled at a triangle's scale, which the smoothing is there to leave out (`SMOOTH_ROUNDS`).
- **The 6 mm hold is the gate's choice, not blending's.** It adds 1.3 to 4.3 mm to the 95th percentile at the hang's and the swings' key poses, 8.1 mm at level and 21.7 mm at raised. The gate sweep chose it ("What I tried": whole moves gave the stride's first third 21.5 mm and 43 bodies at Walk@18; held to 6 mm, 13.4 mm and 39), not a measurement of blends of whole moves.
- **So on the stride's poses** (the hang and the swings) blending alone leaves errors half way (9 to 16 mm) of the same size as the hold and the smoothing together leave at the key poses (13.5 to 16.7 mm). No hold or smoothing setting would remove that. More key poses (the swing in quarters) or a blend that isn't linear would be needed, as open issue 1 already lists.

### Wording corrected

- `MOST_MOVE`'s doc comment (`fit/cloth.rs`) now says the gate sweep chose 6 mm, gives the sweep's figures, and gives the measured blending loss with `Solving::whole(0)`. The "up to 10 cm" figure is gone. `SMOOTH_ROUNDS`'s comment gives the smoothing's loss.
- This hand-back: the opening paragraph, "What the bake measured", open issue 1 (now the three losses with their figures) and review fixes 1's "What this means". That paragraph's conclusion is marked as not measured there. Its "within 0.3 mm at the 95th percentile" away from the armpit is corrected to 2.2 mm (the hips; its own table says 2.18 and 2.21).
- `wiki/Roadmap/Known-Limitations.md`: the cause sentence now gives the measured blending loss and says the gate chose the hold.
- `architecture.md` section 9, `import/README.md`, `correctives.py`'s docstring and `testing.md`: the report's `keys`, `solve_with` and `Solving`, and `solve-whole`.
- The first fixes' acceptance row ("the corrected half-way figures confirm that the blend can't follow the underarm bridge") was in the structured hand-back, not in a file. For the stride's poses the claim holds, now on this measurement; for raised and level it doesn't.

### Non-blocking points also taken

- `wiki/Engine-Systems/Characters.md` no longer says "at most 6 mm a corner": the hold is 6 mm in the key pose, and a stored offset reaches 6.3 mm.

**Not taken (left for the lead):**
- caching `Correctives::driven` and skipping unchanged weights in `drive_own`;
- a CPU bench of the walk;
- a synthetic test of `under_flipped_folds` alone;
- requiring the `Corrective_` prefix in `PoseCorrective::check`. The bake writes `Corrective_Cloth_*` names, and a check would be cheap, but it changes a manifest rule, which is the lead's call.

### Tests

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-avatar --no-fail-fast -E 'binary(cloth_correctives)'` (red: no `keys`) | 6 tests run: 4 passed, 2 failed |
| the same (red: the rounds ignored) | 7 tests run: 6 passed, 1 failed |
| the same, after | **7 tests run: 7 passed** |
| `cargo nextest run -p axcenity-avatar --no-fail-fast` | **218 tests run: 218 passed**, 1 skipped |
| `cargo nextest run -p axcenity-avatar -p axcenity-gameplay -p axcenity-player --no-fail-fast` | **544 tests run: 544 passed** (22 slow), 23 skipped; no FAIL, ABORT or TIMEOUT lines (`pose_corrective_tests`, the gate, `tee_crew_tests`, `wardrobe_tests` and `garment_checks` all pass on their unchanged records) |
| `cargo clippy -p axcenity-avatar -p axcenity-gameplay -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 116 pages, 0 errors, 57 warnings |
| `pose_correctives -- solve` on the kept bake | shapes, rules and fingerprint identical to the committed sidecar; half-way checks identical to review fixes 1's |
| `pose_correctives -- solve-whole ... 12` and `... 0` | the tables above; with 0 rounds every key pose 0.0 mm |

**Files changed in this fix:**
- `crates/avatar/src/fit/cloth.rs`: `Solving`, `solve_with`, the report's `keys`, `moved_*` and `hold_mm`; `MOST_MOVE` and `SMOOTH_ROUNDS` public, their comments corrected. `crates/avatar/src/fit/mod.rs`: the exports.
- `crates/avatar/examples/pose_correctives.rs`: `solve-whole`.
- `crates/avatar/tests/cloth_correctives.rs`: three new tests; `hang_bake` takes a move per corner and a `Solving`.
- `crates/avatar/tests/fixtures/test-tee/TestTee.correctives.json`: its report only.
- Docs: this hand-back and its red-run file, `architecture.md`, `testing.md`, `import/README.md`, `correctives.py`'s docstring, `wiki/Roadmap/Known-Limitations.md`, `wiki/Engine-Systems/Characters.md`.

**Merge notes:** `fit::solve` keeps its signature; `solve_with`, `Solving`, `MOST_MOVE` and `SMOOTH_ROUNDS` are new exports. The sidecar's report has new fields (nothing reads the report). After the drape-preserving fit merges and the tee is baked again, run `solve-whole ... 0` on the new work folder to see whether blending still loses the stride.
