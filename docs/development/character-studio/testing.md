# Character Studio overhaul: testing

How every milestone is tested, what runs where, and the tests that exist. Visual review and the benchmark set are in [visual-qa.md](visual-qa.md). GitHub Actions runs only for release tags ([ADR-0018](../../../wiki/Architecture/Architecture-Decisions/ADR-0018-Actions-For-Release-Tags-Only.md)), so **local runs are the only check**; every hand-back lists the commands run and their results.

## Rules

- **Red, then green.** A milestone starts with tests that fail for the right reason (a stub, a missing file, a defect the milestone fixes); record the failing run in the PR, then make them pass. A test that passes before the code exists doesn't count as evidence for that milestone.
- **Name tests after the behaviour** (`a_format_1_file_migrates_to_the_same_character`), so a criterion can cite one.
- **Never loosen a threshold to pass.** `assets/characters/humanoid/qa/thresholds.json` and the realistic package's thresholds only go down; a raise needs renders showing the cause isn't visible, and the lead's agreement.
- **Known visual defects never become baseline.** A measured defect stays in the audit's list until fixed; a test pins the fix.
- **Damaged input never panics.** Every reader has a test feeding it broken files.

## Layers

| Layer | What | Where | Runs |
|---|---|---|---|
| Unit | Schemas, checks, migration, resolution, fitting maths | `crates/avatar/src/tests/` and each crate's tests | `cargo nextest run -p axcenity-avatar` |
| Contract | The repository's library against its models: every shape, bone and material a manifest names exists; the catalogue is current; defaults and migrated files are sound; every shipped preset is sound, listed and resolves; the realistic parameters have their metadata; 1,000 seeded randomizations per base are sound (R1) | `crates/avatar/tests/shipped_library.rs` | Same; after any change to a manifest, a model or `catalogue.json` |
| Engine agreement | The manifests describe what the engine ships (sockets, body sliders, parts, default outfit); format 1 files resolve to what the workspace shows | `project` `default_humanoid::tests::the_character_data_manifests_describe_the_default_humanoid`; `editor` `character_workspace::tests::a_saved_look_migrates_to_the_same_parts_and_shapes` | `cargo nextest run -p axcenity-project -p axcenity-editor` |
| Runtime | A description applied in the player: parts, identity worn live on the base meshes (R2a; an explicit bake draws the same), followers taking live shapes, bone offsets, attributes, materials, kept descriptions applied again; game scripts calling the `HumanoidDescription` API | `player` `default_humanoid_tests/description.rs` and `description_lua.rs`; `gameplay` `description_tests.rs`; `assets` `model::identity`, `derived`; `avatar` `tests::resolving` | `cargo nextest run -p axcenity-player description`, `-p axcenity-gameplay`, `-p axcenity-assets identity derived` |
| Geometry QA | Poke-through, waist gaps, layer order, joint volume, eyelid closure, teeth and lips, self-intersection at slider extremes | `assets/characters/humanoid/qa/measure.py` (Blender, `bpy`; `--package realistic` for the realistic body, held to `assets/characters/realistic/qa/thresholds.json`) | Locally where `bpy` is installed (below) |
| Visual | Benchmark sheets, in Blender and in the engine | `render_baseline.py`; the in-engine capture (`player`'s `character_capture.rs`, the `capture_characters` tool) | [visual-qa.md](visual-qa.md) |
| UI | Walkthroughs: open, edit, undo, redo, save, reopen; every control changes the character; drawing snapshots | `editor` `character_workspace/tests.rs`, `app/character_workspace.rs` | `AXCENITY_SNAPSHOTS=<folder> cargo nextest run -p axcenity-editor character` |
| Performance | GPU timings per pass, the character budget, a crowd | `FramePass` timers; `docs/benchmarks.md` | On the reference PC, pinned to one CCD |

## Commands

```text
cargo fmt --all
cargo nextest run -p axcenity-avatar
cargo nextest run -p axcenity-project -p axcenity-editor -p axcenity-player
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace --no-fail-fast
cargo test --workspace --doc
python scripts/check-wiki.py
```

After a manifest or model change: `python -I assets/characters/humanoid/manifests.py` (or the family's generator), then `python -I assets/characters/catalogue.py`, then the contract tests.

**Geometry QA** needs Python 3.13 with `bpy` and Pillow (see `assets/characters/humanoid/qa/README.md`):

```text
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --only clipping --out <file> --check assets/characters/humanoid/qa/thresholds.json
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --out <file>      # everything, ~25 s
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --package realistic --out <file> --check assets/characters/realistic/qa/thresholds.json
```

## What each milestone must test

| Milestone | Tests written first |
|---|---|
| CO1 (done) | Schemas round-trip; newer files refused; damaged files are errors; format 1 migrates (values, defaults, ranges, unknown parts); every manifest entry names shapes, bones and materials its model has; checks find each kind of mistake; parameters resolve (spans, corners, constraints, regions) |
| CO2 | The same description, cameras and settings give identical images twice; the capture writes every view |
| CO3 | A character file applied in the player wears its parts, its parts take the body's shape weights, frame bone offsets move the eyes, attributes reach `BodyShape`, material values modulate (base colour map kept); the workspace and the player agree on the same file |
| CO4 | The realistic package's manifest against its model; geometry QA: no eye or teeth poke-through at rest; joint volume within the limits agreed below |
| CO5 | Every parameter moves geometry (a shape's largest move above 1 mm at its range's ends); extremes of every pair free of self-intersection |
| CO6 | `anim::check_face` clean for the new head and its profile; every expression and viseme without eyes through lids or teeth through lips; a full blink covers the eyeball |
| CO7 | Each shading model's output against a reference image at fixed lighting; neutral values match Standard; GPU timings recorded; `every_example_renders` unchanged |
| CO8 | 20 characters × 150 shapes at least as fast as today's 41; ray traced shadows show expressions |
| CO9 | Binding round-trips; every item fits both frames and composition extremes under the poke-through thresholds; layers stay ordered; an imported third-party garment fits after import |
| CO10, CO11 | Per item and style: the contract tests, geometry QA, no clipping into the head at rest |
| CO12 | Walkthroughs; every control in the manifest changes the character; the studio writes format 2 and reads format 1 |
| CO13 | Changing one subsystem breaks none of the others (the benchmark set before and after); a replicated description resolves the same on both ends |

## Thresholds agreed for Stage 3

Starting points from the Stage 1 baseline (`baseline/measurements.json`); a milestone may only tighten them.

| Measure | Stylized today | Realistic target (CO4–CO6) | Realistic at CO4 (masculine / feminine) |
|---|---|---|---|
| Waist gap, every frame × build | 0 mm | 0 mm | Not measured: no garments until CO9 |
| Garment poke-through (points, all combinations) | `thresholds.json` | 0 visible; counts recorded per item at CO9 | Not measured: no garments until CO9 |
| Elbow 135° ring area ratio | 0.37 | ≥ 0.6 with twist bones and correctives | 0.757 / 0.758, met |
| Shoulder abducted 150° | 0.56 (volume) | Ring area ≥ 0.92 / 0.81 (Paul's decision, 2026-11-16; was volume ≥ 0.8) | Ring area 0.929 / 0.82, met; volume 0.463 / 0.513, reported (below) |
| Knee 135° ring area ratio | 0.44 | ≥ 0.6 | 0.74 / 0.75, met |
| Eyeball front uncovered in a full blink | 20–23% | 0% | Not measured: no blink shape until CO6; at CO6 0 / 0 rays bare under `Blink` and `Eye_Closed`, met |
| Teeth vertices inside lip or chin flesh, jaw open | 62–192 | 0 | 6 / 14 (the lower lip's inner side), not met; at CO6 0 / 0 over every control, viseme and expression, met |
| Face vertices above the head joint carrying `Neck` weight | 911 | 0 | 0, met |

**The realistic body at CO4** (`co4/measurements.json`, checked by `measure.py --package realistic --check assets/characters/realistic/qa/thresholds.json`). Met: the elbow, the knee, the `Neck` weights, and CO4's own test of no eye or teeth interpenetration at rest (0 on both frames). **Not met: the shoulder raised 150°** keeps 0.562 / 0.561 of its local volume against 0.8. A crease between collar and arm and a flattened deltoid show in `co4/deformation_poses.png`; the corrective shapes are CO5's. The thresholds file holds it as a `target` (printed as not met; `--strict` fails on it) over a floor of 0.54, and it moves into `at_least` when it is met. Also still short: the fist self-intersects (247 / 199 triangle pairs), the teeth touch the lower lip's inner side with the jaw open, and the lips rest about 2 mm apart (CO6). No target above has changed.

**The realistic body at CO5** (`co5/measurements.json`). The shoulder has corrective shapes, a delta mush of the raised pose, chosen by the pictures (`co5/shoulders_*.png`, `co5/engine/cheer_shoulders_after.png`). On the measure it reads **0.463 / 0.513 at 150°** and 0.847 / 0.88 at 90° (the stylized body: 0.853 / 0.992): the target is not met, and the 150° figure is under the package's own limit of 0.54, so `--check` is red on those two lines. A first shape that moved skin outward until the measure read 0.86 was rejected: it passed the number and showed a step and a ball in the engine (`co5/engine/cheer_shoulders_before.png`). The same pose's ring area is 0.929 / 0.82. **No target or limit was changed; what the shoulder is held to is Paul's decision** (`progress.md`). New at CO5 and met: no pair of the manifest's parameters at the ends of their ranges makes the body newly cross itself at rest (0 of 351 pairs on each frame), and no shape slider puts skin in the eyes or teeth.

**Stage 3's decisions (2026-11-16).** Paul held the 150° shoulder to its ring area (0.92 / 0.81, what CO5 reached) instead of its local volume, and the eye check's distances are signed by pseudo-normals with exact closest-point regions (`qa/pseudo_normals.py`), which removed its three false counts. `measure.py --package realistic --check` and `--only sliders --check` pass every limit on both frames ([stage3/](stage3/README.md)).

**The realistic face at CO6** (`co6/sliders.json`, `co6/sliders-check.txt`).

The sliders check now covers the face's 45 parameters as well: 73 parameters, 2,556 pairs, 10,082 bodies a frame. No body newly crosses itself on either frame; before the identity work, 14 masculine and 8 feminine did. Since R4 the realistic manifest has 89 parameters (3,828 pairs, 15,138 bodies a frame), and `face_checks.py` also holds R4's 14 face parameters at their ends alone, in a blink and under every expression and viseme (`HELD_IDENTITY`), and to no new crossing of the head's skin triangles alone or on top of any other face parameter's end, including the pairs within 4 mm at rest that the sliders check leaves out (`identity_pairs`; the closed lips' contact aside). R4's second pass (wave 3) added `avatar` `tests/realistic_face_second_pass.rs` (the brow texture is hair: graded density, part-covered texels, soft edges, a body that reads, nothing on the card's border, a dark brown with hairs varying; the brow cards taper (their height across their own line, tail under 0.6 of the head) and the feminine one is smaller and more arched; the brows' lower edge against the eye corners and their length (first hair to last) within 2 SD of F-29, the feminine brow 0.80 to 0.96 of the masculine's length, and the hair's height at the head about F-12 less F-29 on the feminine frame and 1.15 times that or more on the masculine; the masculine glabella 5 mm or more forward of sellion and both nasofrontal angles within 1.5 SD of F-30; the feminine face's widths and height against the masculine at the references' ratios; review fixes 2: `nose_tip_rotation` moves the nasolabial angle (columella point, subnasale, labrale superius, on the midline's triangles) by 20 to 24 degrees at +1 and 15 to 24 at -1 on both frames, and no face preset hollows the forehead's profile more than 0.5 mm deeper than the default face) and `player` `face_preset_tests` (every face preset changes at least 3% of the face's pixels against the default face in the studio front and side pictures on both frames; GPU, skipped without one); `realistic_face_controls`' frames' difference is now more than 4.6 mm.

The harness now moves and scales the eye bones by the parameters' `bones` drivers, as `avatar::resolve` and the bake do (`qa_scene.manifest_bones`). Before, it moved the lids off eyeballs left at rest, which made 13 to 14 of each frame's 17 eye-in-flesh entries.

**Red on one line:** `sliders/frames/masculine/alone_eye_or_teeth_in_flesh` is 3 (at most 0). These are three false counts:
- `eye_metrics` signs an eyeball point by the face Blender returns as nearest.
- These three points are nearest an edge two faces share, and that face's side says "inside".
- By the edge's pseudo-normal and by four rays' parity, each point is 2.4 to 4.7 mm outside the lid.

**No threshold or measure was changed.** Whether to sign by the nearest feature's pseudo-normal is Paul's decision.

## Tests that exist (wave 4, CO9 part 2: the swing correctives, the motion and footwear checks)

- `anim` `correctives::tests`: `a_swung_arm_leans_on_its_swing_correctives` (the hanging arm swung back leans on `Corrective_Shoulder_Back_*` and its lowered shape, forward on `_Forward_*`, half way half), `the_ramp_eases_in_and_out` (the rule's ramp is smoothstep). `avatar` `tests/fitting.rs`: `the_swing_correctives_take_the_walks_fold_out_of_the_armpit` (the walk's widest fit poses: the fold's 95th percentile 10.6 to 4.9 and 5.9 mm on the masculine frame, no worse by more than 0.5 mm on the feminine), `the_sports_bras_straps_sit_on_the_shoulders_not_the_neck`, the walk swings in `the_fit_poses_lean_on_the_shoulder_correctives`.
- `player` `garment_checks` (helpers for content tracks, and their tests on the shipped items): `motion(assets, description, clip, frames)` and `footwear(assets, description, stance)` return reports whose `problems()` a test checks against `Record`s of open defects (a ratchet, as the fit gate's `Known`). Tests: the realistic cast in the test tee over the underwear in Idle, Walk and Run; the stylized cast in its outfit and a hairstyle each in Walk and Run; the stylized sneakers at rest, knees bent and in the walk; a unit test of the measures. `fit_gate_tests` checks layered pairs (`LAYERS`: the bra under the test tee, the stylized cargo pants and sports top under the tee) and leaves shoes to the footwear check. Run: `cargo nextest run -p axcenity-player -E 'test(/fit_gate_tests|garment_checks/)'` (about 2 minutes). Since CO9 part 2's review fixes: a record no longer earned fails too (its defect gone, or more than `garment_checks::SLACK`, 1 mm, under it: remove or lower it), so a check that stopped finding a recorded defect is caught; `motion`'s frame-by-frame core (`step`) and `footwear`'s (`foot_report`) have synthetic tests (a corner thrown 1 cm out of line is a pop, a weld split 1 mm a tear, a foot 5 mm through a box shoe's sole and a tread 5 mm off flat are found); seams are found at rest; the stylized gate runs the walk too; `avatar` `coverage_counts_a_crease_the_garment_closes_over`; `fitting.rs`'s fold test holds each frame's and side's 95th percentile and deepest corner at their records. Captures: `AXCENITY_CAPTURE_FULL_DETAIL=<part>` draws one part at full detail. Review fixes 2: **holes**: the gate (every pose) and `motion` (every frame) count the corners of body triangles the worn items hide (`hidden_triangles`: drawn bare, not dressed) that are more than 1 mm outside every garment (`holes`, by `avatar::fit::outside_cloth`), recorded as `hidden skin showing` / `holes`; synthetic tests `avatar` `hidden_skin_past_a_hem_or_through_the_cloth_is_a_hole` and `skin_shut_in_a_crease_is_no_hole`. `covered` leaves out skin nearer a hem (or the rim folded under it) than the cloth said to cover it by more than `BESIDE_HEM` (1 cm): skin beside the hem, not under the cloth (`skin_nearer_a_hem_than_the_cloth_over_it_is_not_covered`). The stylized gate measures the underwear alone as well as under the outfit (dressed, the cargo pants' mask shape sinks the skin under the briefs).

## Tests that exist (R3 wave 3, RA3: the waist, heavy and muscular, the stylized ranges)

`gameplay` `tests/realistic_waist.rs`: `the_waist_slider_changes_the_waists_breadth_by_most_of_its_value` (the description resolved, its identity baked, `WaistWidth` applied by `anim`'s `BodyShape`; at both ends of `waist_width` on both frames the natural waist's breadth changes by at least 0.6 of the slider's change and its depth by 0.15 to the breadth's share; built: 0.82 to 0.88 and 0.29 to 0.31) and `the_waist_shapes_are_driven_from_one_on_both_sides`. `avatar` `tests/realistic_heavy_muscular.rs`: `heavy_and_muscular_is_not_a_bodybuilder` (the upper body's definition, what is left of the move from the default once smoothed over about 10 cm, at most 1.5 times the heavy body's, and the muscle still adds mass) and `heavy_and_muscular_thighs_do_not_flare_below_the_hips` (the front silhouette's widest point 14 to 30 cm under the hip joints at most 3 mm past the hips', measured on level cuts of the skin without the underwear's mask). `avatar` `tests/realistic_ranges.rs`: `the_stylized_proportion_ranges_are_its_sweeps` (the stylized manifest ships ranges.md's "Stylized shipped ranges"; every slider goes both ways, the hips' driver from 1), `stylized_hips_narrow_below_one_by_a_shape_not_a_bone` (at the low end `HipWidth` stays 1, each frame's `<Frame>_Hips_Narrow` is driven and the hips' widest skin comes in by at least 0.35 of the slider's change, nothing moving out or across the middle) and `the_stylized_head_size_reaches_the_population` (at least 0.925 to 1.075; review fixes 1). Blender: `measure.py --package stylized --check assets/characters/humanoid/qa/thresholds.json,assets/characters/humanoid/qa/body-thresholds.json` with `sliders` in `--only` (the stylized body's own sliders make no crossings; its joints keep what they reach). The red run is [refinement/wave3/r3-body-2-red-run.txt](refinement/wave3/r3-body-2-red-run.txt).

## Tests that exist (R1 benchmarks, RA6)

`avatar` `every_benchmark_character_is_sound` now also holds that each file wears what its choices make it wear (so a file says what is drawn), that every base has the cast on both frames (`asymmetry` only where the base has sided parameters) and that a cast member other than `average` sets something; the realistic face reference is `cast-average`. `player` `character_capture`: `the_default_run_is_every_fixed_view_under_every_fixed_light` (no new light or view in a run that doesn't name it), `light_view_and_pose_names_are_read_and_unknown_ones_refused`, `the_suites_hold_the_brief_s_facial_and_body_tests`, `held_poses_turn_each_joint_to_its_angle_on_the_realistic_body` (both sides within 1 degree of 90, 150, 135 and 135, the forearms forward and the shins back), `added_lights_views_and_poses_repeat_and_differ` (GPU: the same pictures twice, each new light differs from `studio`, each held pose from rest) and `the_face_suite_draws_each_face_on_one_sheet` (GPU). (The game's Walk state keeps the Run take: the lead's decision at the R1 merge, recorded in progress.md. The capture's body suite poses the Walk clip at its own speed.) The red run is [refinement/r1/r1-benchmarks-red-run.txt](refinement/r1/r1-benchmarks-red-run.txt).

## Tests that exist (CO6)

`anim` `tests/realistic_face.rs` (4: `check_face` finds nothing wrong with the head and its profile; every standard control moves a vertex or a bone; the face's parts carry the shapes they follow; every control the ARKit profile uses plays); `anim` `face_tests.rs` `a_blink_closes_only_what_a_closed_eye_leaves_open` and `a_blink_closes_only_what_a_narrowed_eye_leaves_open` (the blink rule, on the shipped body); `avatar` `tests/realistic_parameters.rs` (every face parameter at each end moves the baked body and both copies, now 45 more); `player` `the_same_description_captures_identical_pictures_and_every_view_is_drawn` (six views). Blender: `assets/characters/humanoid/qa/face_checks.py --package realistic --check` poses every control, viseme, expression and identity end on both frames and holds, among others, lids in the eyeball, eyeballs in the lid, bare rays under a full blink, teeth and tongue in flesh, lips in teeth, lining through skin, new self-crossings, the mouth's, teeth's and tongue's mirror symmetry (0.1 mm) and the lips' seal out of view once the lips are 6 mm apart; identity ends are recorded, not held. Its record and output: `co6/face_checks.json`, `co6/face-checks-run.txt`. `measure.py --package realistic --only sliders` sets every pair of the 73 parameters at their ends on both frames (`co6/sliders.json`).

## Tests that exist (CO5)

`avatar` `tests/realistic_parameters.rs` (every parameter of the realistic manifest at each end of its range moves the baked body and both simpler copies by more than 2 mm, or changes one of `anim`'s body sliders; muscle and fat make four different corners; a baked body keeps only its corrective and expression shapes live); `avatar` `tests/realistic_fat.rs` (R3: `fat` 1 brings the belly at least 25 mm forward and widens the waist at least 40 mm on both frames; more belly on the masculine frame, more buttocks and thigh on the feminine; the slight masculine belly at least 15 mm behind the slim one); `anim` `tests/realistic_proportions.rs` (each of the nine body sliders moves the skin and both copies); `anim` `correctives::tests` (6: the rule, the constants against `Realistic.build.json`, the shipped body through the pose path on all three levels of detail, a model without such shapes left alone). Blender: the package thresholds' `sliders` entries (`measure.py --only` with `sliders` in the list, about four minutes) and `posed_crossings.py` (new crossings in a pose). The red runs are in `co5/` (`red-run-correctives.txt`, `red-run-parameters.txt`).

## Tests that exist (CO4)

`anim` `twist::tests` (12: the twist and helper rules, a skeleton without such bones left alone, slider scale, the one call site, the shipped realistic skeleton driven as it was built); `anim` `retarget::tests::a_take_named_like_a_body_of_other_proportions_keeps_that_bodys_bones` and `the_starter_animations_keep_the_realistic_bodys_proportions` (the clip rule of architecture section 12). Blender: `assets/characters/realistic/qa/thresholds.json` (`at_least`, `at_most` and `within` limits, and the unmet shoulder `targets`). The red runs are in `co4/` (`red-run-anim.txt`, `red-run-avatar.txt`, `red-run-thresholds.txt`).

**Left red on purpose:** `avatar` `every_manifest_entry_names_shapes_bones_and_materials_its_model_has` fails for `axcenity/realistic` on the test's own floor (a base body has more than 10 shapes): the body has one shape, `Frame_Feminine`, until CO5 adds the body shapes. The manifest's check against its model passes. It is not an accepted baseline. **Green since CO5:** the body has 53 shapes; the floor was not changed.

## Tests that exist (CO2)

`player` `character_capture::the_same_description_captures_identical_pictures_and_every_view_is_drawn`; `avatar` `every_benchmark_character_is_sound`.

## Tests that exist (CO3)

`assets` 13 (`model::identity::tests`, `derived::tests`); `avatar` 5 more in `tests::resolving` (`resolve`); `player` `default_humanoid_tests::description` (7: parts worn and swapped, identity baked and live shapes followed, the frame's bones re-seating the body and carrying the eyes, body sliders, material values modulating, a still character wearing its parts, kept descriptions applied again) and `description_lua` (3: the script API); `gameplay` `description_tests` (the API's pure functions); `project` `character_library::tests` (2); `editor` `a_character_file_applied_in_a_game_matches_the_workspaces_preview`. Three of the player's tests were checked against the behaviour switched off (no settle, no follower shapes, no modulation) and failed.

**R2a (live identity, ADR-0041):** `assets` `an_identity_worn_live_poses_the_mesh_where_its_bake_does` (four identities, at rest and turned, live against baked to 1e-5) and `a_registered_identity_is_found_by_its_key_and_bakes_nothing`; `player` `default_humanoid_tests::description` now 10 (the CO3 tests read the posed, live meshes; new: `a_parameter_change_on_an_applied_character_changes_weights_only` (no MeshId changes, `Assets::derived_count` stays 0), `an_items_own_parameters_are_live_weights_on_its_part`, `an_explicit_bake_puts_every_corner_where_the_live_character_has_it` (0.2 mm; a baked character stays baked through `DescriptionSync`)); `character_capture::an_explicit_bake_draws_the_same_pixels_as_the_live_character` (front, side and face under studio light); `editor` `a_character_file_applied_in_a_game_matches_the_workspaces_preview` back at 0.2 mm per corner with no derived `MeshId`. Measurements (ignored, release): `player` `bench_distinct_characters`, `editor` `drag_apply_time` (live and baked).

## Tests that exist (CO1)

`crates/avatar`: 35 tests (`src/tests/formats.rs`, `migration.rs`, `validation.rs`, `models.rs`, `resolving.rs`; `src/ids.rs`, `src/material.rs`; `tests/shipped_library.rs`). `project`: `the_character_data_manifests_describe_the_default_humanoid`. `editor`: `a_saved_look_migrates_to_the_same_parts_and_shapes`. The red run before the code is recorded in [progress.md](progress.md).
