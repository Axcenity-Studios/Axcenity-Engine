# Pose correctives from cloth simulation: hand-back (refinement wave 5, stage 3)

Branch `pose-correctives`, from `refinement-wave5` (main, waves 1 to 4, PR #428, Paul's decisions 11 to 14, wave 5 stages 1 and 2). Task: pose correctives for garments, baked from cloth simulation and driven by the joints like the body's own swing correctives, so a garment's folds and the cloth between the arm and the side behave in the shoulder, arm-swing, elbow, hip and knee poses.

**In one paragraph.** The pipeline is built end to end and the engine drives it: an item can carry its own joint-driven shapes (an optional `correctives` in the item manifest, the same rules in its model's glTF extras, read into `MorphTarget::driver`, kept in cooked models, driven per frame on every body by `anim`), and a new intake step (`correctives.py` around `avatar::fit::cloth`) bakes them with Blender's cloth simulation. **The main goal is not met.** On the test tee the correctives lower the stride's poke-through in its second third (16.6 to 14.1 mm) and its corners under a flipped fold in every third (147, 199, 161 to 122, 136, 139), but the folds still flip, and the slit, the bra triangle and the arms-raised flaps look the same in 4K captures. They also make some cases worse: the bra through the tee in the first third (20 bodies, 10.8 mm to 21, 14.9 mm), one body more at Walk@18 for the tee and for the bra, and a bra case with the arms raised. Taking them into the gate would raise records, so the gate keeps its test tee without them and a new test module measures it with them, with its own exact records. On the licensed tee the correctives made the held poses much worse (the T-pose from 7 to 53 bodies, the arms raised from 14 to 54), so I didn't commit them. The cause I can see: the cloth simulation can't put cloth between a hanging arm and the side where the body's own skin overlaps (the arm sinks into the side), and it moves the bridge of cloth 5 to 10 cm in ways a blend between key poses can't follow. The frame cost is small (50 characters: +2% shape moves, about +0.02 ms of GPU posing).

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

   A shape that moves nothing over 0.5 mm is dropped. The half-way checks compare the blend with the simulation (`solve.json`).
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
- **The half-way checks** (the blend against the simulation): median 0.0 to 1.5 mm. The 95th percentile is 4.0 mm for hanging, 15 to 16 mm for the swings, 9.6 mm for level and 26.7 mm for raised. The worst corners are off by 25 to 103 mm.
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
- `bake-hang-armpit-skinned-vs-simulated.jpg`: Blender renders from the bake itself, not the engine. The left armpit with the arms hanging, front then side, the skinned goal (blue) beside the simulated cloth (orange).

Sizes are measured in sheet pixels at about 25 capture px a centimetre (±30%).

**What I saw:**
- **The correctives change little you can see.** At the sheet's scale every before and after pair is nearly the same picture. That is expected with moves of at most 6 mm.
- **Walk, the near arm back** (`armpit-walk-0-633-3x.jpg`):
  - the slit of skin at the front of the armpit on masculine average is there before (about 6 × 5 mm, at 165 to 210, 135 to 175) and after (the same size, at 595 to 640, 135 to 175, its edges shifted);
  - the black bra triangle on feminine heavyset is about 5 × 4 mm before (1915 to 1955, 170 to 200) and a little lower and flatter after (2350 to 2395, 170 to 195);
  - feminine average shows no hole in either.
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
   - The cause: a cloth simulation can't put cloth between a hanging arm and the side where the body's own skin overlaps. The bridge moves 5 to 10 cm and folds differently in each key pose, which no blend of shapes follows between key poses (half-way 95th percentile 15 to 27 mm), so the shapes are held to 6 mm.
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
7. **The engine doesn't check the manifest against the model.** `rules_match` exists and is tested, but no shipped item has correctives, so the shipped-library check doesn't call it yet.

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
