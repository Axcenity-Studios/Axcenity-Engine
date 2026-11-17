# Character Data audit: the avatar crate, the manifests and the catalogue

Stage 0 of the Character Studio refinement, 2026-11-16, at commit `99f5b181`. Audited against [brief.md](brief.md) §4.1, §4.2, §5.1, §5.2, §5.4, §14.1 and §15 (Parameter metadata, Asset metadata). Source and tests were read; nothing was built or run except plain Python that reads the manifests and the model's shape names. A second pass checked every row against the code and corrected the file: see [Verification](#verification).

## Summary

- **The foundation the brief asks to keep is sound.** One description format (format 2), one base manifest per body, one item manifest per asset, a catalogue, and checks with tests (`crates/avatar`, 4,535 lines with tests). The workspace already draws Body, Face, Hair and Clothing from the manifests.
- **Body (§4.1, §4.2).** The realistic body has 28 body parameters: frame, nine proportions, muscle, fat, and muscle and fat for eight regions. Nine of the twelve named controls exist and three are partly there: overall build has no control of its own on the realistic body (muscle and fat serve it), hip width only widens, and chest proportions are muscle and fat only. What is missing is not sliders but **structure**: nothing says which region a parameter belongs to, the 16 regional sliders sit in one advanced group, and there is no neck, shoulder slope, waist transition or torso length.
- **Face (§5.2).** The realistic face has 47 parameters (16 left and right pairs, 15 single). Of the brief's 63 named controls, 29 exist, 10 are partly there and 24 are missing. Every `Face_*` shape in the model is already driven by a parameter, so each missing control, and each partial one that should move on its own, needs a new shape from the Blender build, not just a manifest line.
- **Presets (§4.1, §5.1).** Six presets exist, as Rust constants in the editor, all whole-character and all body-only in content. There are no face presets and no preset files. Applying a preset replaces the whole character.
- **Symmetry (§5.4).** `mirror` pairs, Symmetry on by default and per-side editing exist and are tested. Mirror, copy left to right, copy right to left and a per-side reset do not exist. None of them needs new data.
- **Capabilities (§14.1).** An item declares its slot, bases, fit, hidden regions, materials and variants. Its only settings today are material values, fixed by each slot's shading model; hairs already differ in how many slots they have (two have one colour, four have a second for highlights). It cannot declare its own parameters (length, volume), cannot narrow which material values a slot offers, and has no dependencies or limitations. No control in the workspace sets any of it. No shipped item has a variant or a thumbnail.
- **Metadata (§15).** Parameters carry a label, a group, a range, a default, an advanced flag and a mirror. They carry no description, tooltip, icon, subgroup, detail level or viewport handle data.
- **Proposal.** Everything the brief asks for fits as optional fields on the existing files, with no version bump: section 4. Presets fit as description files with one extra block: section 5.
- **One contract correction.** `crates/avatar/CLAUDE.md` and the wiki say unknown fields are kept. Only the description's top level keeps them. Manifests, the catalogue and worn items ignore them on read and drop them on write; `architecture.md` section 4 says so for manifests but not for worn items (Defects, D1).
- **Two findings from verification.** The benchmark folder is stale against its generator, and `brow_length` has no benchmark (D8). The other audit, [audit-studio-ui.md](audit-studio-ui.md) section 4, proposes different names and shapes for the same metadata; the lead must pick one before any schema lands.

**Counts.** 173 rows: 68 implemented, 49 partial, 56 missing (the first pass had 171: 68, 39, 64).

**Short names used in evidence.** `R` is `assets/characters/realistic/Realistic.base.json`; `H` is `assets/characters/humanoid/Humanoid.base.json`; a number after the colon is the line of the parameter's `"id"`. Paths that start with `manifest.rs`, `item.rs`, `catalogue.rs`, `description.rs`, `library.rs`, `material.rs`, `resolve.rs` are in `crates/avatar/src/`. `character_workspace.rs` and `character_workspace/…` are in `crates/editor/src/`. `faceparams.py`, `shapes.py`, `face.py` are in `assets/characters/realistic/`.

**Sizes.** S: a manifest or small code change, under a day. M: a schema field with checks, tests and generators, or one new shape through the Blender build with its benchmarks. L: several shapes that interact with other systems (lids and blinks, grooms). XL: none here.

**Status.** "Implemented" means a user can do it today and a test or file proves it. For the region rows of §4.2 the status is whether parameters exist for that region; selecting the region is row CD-4.2-01. For the control rows of §5.2 the status is for the realistic family. "Partial" also covers data that ships with no UI, and a named property that an existing control changes but not on its own. "Missing" means nothing changes it.

## 1. Requirements

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| CD-4.1-01 | Control: body frame | 4.1 | implemented | R:158 `frame` (a choice: Male 0, Female 1, each wears its underwear); H:70 `frame` (a slider); `character_workspace/tests.rs:55` | It means masculine to feminine. If the brief means frame size (slight to broad), no single control does it | S |
| CD-4.1-02 | Control: height | 4.1 | implemented | R:761 `height` 0.85 to 1.15; H:567 0.5 to 1.5; `crates/avatar/tests/realistic_parameters.rs:84` | None | S |
| CD-4.1-03 | Control: overall build | 4.1 | partial | R:940 `muscle`, R:949 `fat` (both −1 to 1, corner shapes with `frame`); H:528 `build`; `realistic_parameters.rs:173` | The realistic body has no build control of its own: the same two sliders are counted again for rows 04 and 05, so three named controls map to two. A macro or the body presets must cover it | S |
| CD-4.1-04 | Control: muscle definition | 4.1 | implemented | R:940 `muscle`; H:553 `muscle` (0 to 1) | It is muscle mass (a shape). No separate surface definition control | S |
| CD-4.1-05 | Control: body fat | 4.1 | implemented | R:949 `fat`; `realistic_parameters.rs:173` | Stylized has only `build` (H:528) | S |
| CD-4.1-06 | Control: shoulder width | 4.1 | implemented | R:820, H:626 `shoulder_width` (attribute `ShoulderWidth`) | Marked `advanced`, so hidden until Advanced is ticked | S |
| CD-4.1-07 | Control: waist width | 4.1 | implemented | R:840, H:646 `waist_width` | Marked `advanced` | S |
| CD-4.1-08 | Control: hip width | 4.1 | partial | R:860 `hip_width` range 1.0 to 1.15; H:666 0.5 to 1.5; `assets/characters/realistic/manifests.py:53` | Realistic hips only widen (narrower makes the skin cross). Marked `advanced` | M |
| CD-4.1-09 | Control: chest proportions | 4.1 | partial | R:958 `muscle_chest`, R:968 `fat_chest` (advanced) | No chest width, depth or bust size (searched bust, breast, chest_width, chest_depth, ribcage). Stylized: none | M |
| CD-4.1-10 | Control: leg proportions | 4.1 | implemented | R:780, H:586 `leg_length`; R:1078 to R:1108 thighs and calves, muscle and fat | No thigh and shin length split | S |
| CD-4.1-11 | Control: arm proportions | 4.1 | implemented | R:800, H:606 `arm_length`; R:998 `muscle_arms`, R:1008 `fat_arms` | No upper arm and forearm split | S |
| CD-4.1-12 | Control: head-to-body ratio | 4.1 | implemented | R:880, H:686 `head_size` (attribute `HeadSize`) | Marked `advanced` | S |
| CD-4.1-13 | Create visual presets (the preset system) | 4.1 | partial | `character_workspace.rs:102` `Preset`, `:145` six presets as Rust constants; pictures `character_workspace/thumbnails.rs:77`; `tests.rs:369` | Presets are code, whole-character only, addressed by index. No body-only preset, no data file | M |
| CD-4.1-14 | Preset: Slim | 4.1 | missing | Not in `character_workspace.rs:145-190`. Only benchmark data: `assets/characters/benchmarks/make.py:49` | Expressible today with `fat`, `muscle` | S |
| CD-4.1-15 | Preset: Average | 4.1 | partial | `character_workspace.rs:147-160`: "Male" and "Female" are the default build | Not named Average; one per frame | S |
| CD-4.1-16 | Preset: Athletic | 4.1 | implemented | `character_workspace.rs:161-174` "Athletic male", "Athletic female"; `tests.rs:369` | One per frame; applying it resets the rest of the character (CD-4.1-22) | S |
| CD-4.1-17 | Preset: Muscular | 4.1 | missing | Not in `character_workspace.rs:145-190`. Benchmark data: `make.py:51` | Expressible with `muscle` | S |
| CD-4.1-18 | Preset: Heavyset | 4.1 | implemented | `character_workspace.rs:175-188`; `tests.rs:369` | As Athletic | S |
| CD-4.1-19 | Preset: Lean | 4.1 | missing | Not in `character_workspace.rs:145-190`. Benchmark data: `make.py:54` | Expressible with `fat`, `muscle` | S |
| CD-4.1-20 | Preset: Broad | 4.1 | missing | None (searched broad, wide, frame_size) | Expressible with `shoulder_width` (15% at most), `muscle_shoulders`, `muscle_back`, `muscle_chest` | S |
| CD-4.1-21 | Preset: Custom | 4.1 | partial | `character_workspace.rs:565` `is_preset`; `character_workspace/ui.rs:357` (no preset is marked when none matches) | No "Custom" entry | S |
| CD-4.1-22 | Presets set starting values, they don't lock | 4.1 | partial | `character_workspace.rs:551` `use_preset` is one undo step; every slider stays free | It replaces the whole description (`character_workspace.rs:113-115` starts from the default character), so a body preset also resets face, hair and clothes | M |
| CD-4.2-01 | Select a region in the viewport or a simplified selector (the data it needs) | 4.2 | partial | Half the data ships, with no UI: the realistic body's region map says which region each triangle is (14 indexes on `TEXCOORD_1`; `Realistic.build.json` `regions_map`; R:3802 `regions`; `manifest.rs:257-269`). A parameter carries `group` only (`manifest.rs:110-114`) | No parameter-to-region link, no pick or focus data, and nothing selects a region today (the UI audit's row). Three region lists disagree: the 14 hide regions (R:3804 to R:3869), the 8 build regions (`shapes.py:203-212`: chest, shoulders, arms, abdomen, back, hips, thighs, calves; their weights are not exported) and the brief's 13 | M |
| CD-4.2-02 | Region: head and neck | 4.2 | partial | R:880 `head_size` | No neck length or thickness (searched neck_length, neck_thick, neck_width) | M |
| CD-4.2-03 | Region: shoulders | 4.2 | implemented | R:820 `shoulder_width`, R:978 `muscle_shoulders`, R:988 `fat_shoulders` | Split over groups `proportions` and `build_regions` | S |
| CD-4.2-04 | Region: chest | 4.2 | implemented | R:958 `muscle_chest`, R:968 `fat_chest` | See CD-4.1-09 | S |
| CD-4.2-05 | Region: upper arms | 4.2 | partial | R:998 `muscle_arms`, R:1008 `fat_arms` cover the whole arm (`shapes.py:208`) | No upper-arm-only control | M |
| CD-4.2-06 | Region: forearms | 4.2 | partial | The same two parameters (`shapes.py:208`) | No forearm-only control | M |
| CD-4.2-07 | Region: hands | 4.2 | implemented | R:900, H:706 `hand_size` | Size only | S |
| CD-4.2-08 | Region: abdomen | 4.2 | implemented | R:1018 `muscle_abdomen`, R:1028 `fat_abdomen` | None | S |
| CD-4.2-09 | Region: waist | 4.2 | implemented | R:840 `waist_width` | Width only | S |
| CD-4.2-10 | Region: back | 4.2 | implemented | R:1038 `muscle_back`, R:1048 `fat_back` | None | S |
| CD-4.2-11 | Region: hips | 4.2 | implemented | R:860 `hip_width`, R:1058 `muscle_hips`, R:1068 `fat_hips` | `hip_width` only widens | S |
| CD-4.2-12 | Region: thighs | 4.2 | implemented | R:1078 `muscle_thighs`, R:1088 `fat_thighs` | None | S |
| CD-4.2-13 | Region: calves | 4.2 | implemented | R:1098 `muscle_calves`, R:1108 `fat_calves` | None | S |
| CD-4.2-14 | Region: feet | 4.2 | implemented | R:920, H:726 `foot_size` | Size only | S |
| CD-4.2-15 | Each region exposes its own controls | 4.2 | partial | All 16 regional sliders sit in one group `build_regions`, all `advanced` (`shapes.py:174-177`; `realistic_parameters.rs:95-104`) | No field says which region a parameter belongs to | S |
| CD-4.2-16 | Abdomen example: abdominal fullness | 4.2 | implemented | R:1028 `fat_abdomen` (label "Abdomen fat") | Label | S |
| CD-4.2-17 | Abdomen example: abdominal muscle definition | 4.2 | implemented | R:1018 `muscle_abdomen` | Label | S |
| CD-4.2-18 | Abdomen example: waist transition | 4.2 | missing | None (searched waist_trans, taper, transition) | New shape | M |
| CD-4.2-19 | Abdomen example: side profile depth | 4.2 | missing | None (searched belly, side_profile, profile_depth, abdomen_depth) | `fat_abdomen` is nearest; new shape | M |
| CD-4.2-20 | Shoulders example: shoulder width | 4.2 | implemented | R:820 `shoulder_width` | None | S |
| CD-4.2-21 | Shoulders example: deltoid volume | 4.2 | implemented | R:978 `muscle_shoulders` (label "Shoulders muscle") | Label | S |
| CD-4.2-22 | Shoulders example: shoulder slope | 4.2 | missing | None (searched clavicle, shoulder_slope, trapez, shoulder_height) | New shape or bone driver | M |
| CD-4.2-23 | Shoulders example: upper-body proportions | 4.2 | partial | R:820 `shoulder_width`, R:840 `waist_width` | No torso length or chest size (searched torso_length, upper_body, spine_length) | M |
| CD-4.2-24 | No meaningless combinations of controls | 4.2 | partial | Sliders come only from the base's manifest (`character_workspace/ui.rs:422-470`); each must move the body: `realistic_parameters.rs:115` | Every build region gets the same pair, muscle and fat (`shapes.py:174-177`): a uniform grid, not controls chosen for each region's anatomy, and there is no region panel yet to judge. Keep the test when parameters are added | S |
| CD-5.1-01 | Introduce a facial preset library | 5.1 | missing | `presets()` sets only `frame`, `muscle`, `fat` (`character_workspace.rs:145-190`) (searched face_preset, face shape, head_shape, oval, heart) | No face preset exists, in code or data | M |
| CD-5.1-02 | Presets are complete starting configurations the user then changes | 5.1 | partial | Whole-character presets do this (`character_workspace.rs:551`) | No face-only application | M |
| CD-5.1-03 | Face preset: round | 5.1 | missing | None | Expressible: `cheek_fullness`, `jaw_width`, `chin_length`, `chin_width` | S |
| CD-5.1-04 | Face preset: oval | 5.1 | missing | None | Expressible: `jaw_width`, `chin_width`, `forehead_height` | S |
| CD-5.1-05 | Face preset: square | 5.1 | missing | None | Expressible: `jaw_width`, `chin_width`; jaw angle is missing (CD-5.2-36) | S |
| CD-5.1-06 | Face preset: heart-shaped | 5.1 | missing | None | Expressible: `temple_width`, `cheekbone`, `jaw_width`, `chin_width` | S |
| CD-5.1-07 | Face preset: long | 5.1 | missing | None | Partly: `chin_length`, `forehead_height`, `nose_length`. No face length control | S |
| CD-5.1-08 | Face preset: broad | 5.1 | missing | None | Partly: `temple_width`, `jaw_width`, `cheekbone`. No midface width (CD-5.2-32) | S |
| CD-5.1-09 | Face preset: narrow | 5.1 | missing | None | As broad, the other way | S |
| CD-5.1-10 | Face preset: soft features | 5.1 | missing | None | Partly: `cheek_fullness`, `brow_ridge`, lips. No jaw definition (CD-5.2-38) | S |
| CD-5.1-11 | Face preset: angular features | 5.1 | missing | None | Partly: `cheekbone`, `brow_ridge`, `jaw_width`. No jaw angle or definition | S |
| CD-5.1-12 | Presets are visual starting points, not biological categories | 5.1 | missing | No face presets exist to judge | A naming rule for the new data: name by shape (Round, Angular), never by ancestry or sex | S |
| CD-5.1-13 | Presets are data-driven and compatible with the description | 5.1 | partial | A preset produces a format 2 `Description` (`character_workspace.rs:113`) but is a Rust constant (`character_workspace.rs:102-109`) | No preset file, nothing in the catalogue | M |
| CD-5.2-01 | Eyes: eye size | 5.2 | implemented | R:1318, R:1347 `eye_size_left/right` (shape and eye bone scale); H:834 `eyes_size` | None | S |
| CD-5.2-02 | Eyes: eye spacing | 5.2 | implemented | R:1184, R:1217 `eye_spacing_left/right`; H:746 `eyes_spacing` | None | S |
| CD-5.2-03 | Eyes: eye depth | 5.2 | implemented | R:1250, R:1284 `eye_depth_left/right` (advanced) | Stylized: none | S |
| CD-5.2-04 | Eyes: eye height | 5.2 | implemented | R:1118, R:1151 `eye_height_left/right`; H:790 `eyes_height` | None | S |
| CD-5.2-05 | Eyes: eye tilt | 5.2 | implemented | R:1376, R:1395 `eye_tilt_left/right`; H:870 `eyes_tilt` | None | S |
| CD-5.2-06 | Eyes: eye shape | 5.2 | missing | None (searched eye_shape, eye_round, almond) | New identity shapes (round to almond) | M |
| CD-5.2-07 | Eyes: upper eyelid | 5.2 | missing | None as identity (searched eyelid, upper_lid, hood). Lid shapes are expressions only (`Blink_*`, `Eye_Closed_*`, `Eye_Squint_*` in `RealisticParts.glb`, no parameter) | A new identity shape that must work with the blink | L |
| CD-5.2-08 | Eyes: lower eyelid | 5.2 | missing | None as identity (same search) | As the upper eyelid | L |
| CD-5.2-09 | Eyes: inner and outer corners | 5.2 | partial | R:1376, R:1395 `eye_tilt_left/right` raise the outer corner by turning the eye's opening (`face.py:1410-1415`) | The corners move only together, as a tilt. No inner-corner control, no corner moved alone (searched canthus, eye_corner, inner_corner, epicanth) | M |
| CD-5.2-10 | Eyes: eyeball proportions | 5.2 | partial | `eye_size_*` scale the eye bones (`face.py:1409`) | No control of the eyeball alone | M |
| CD-5.2-11 | Eyes: iris and pupil appearance | 5.2 | partial | Eye shading values `iris_tint`, `pupil_size`, `limbal_ring`, `iris_depth` (`material.rs:180-204`); stored in `materials` (`description.rs:32-35`); slots Iris, Cornea, Sclera (R:3957) | The workspace has no material controls. `scene` reads only `iris_tint` until CO7 (`crates/scene/src/material.rs:153`). The `Pupil_Size` shape has no parameter (`faceparams.py:17-20`) | M |
| CD-5.2-12 | Eyebrows: brow height | 5.2 | implemented | R:1414, R:1444 `brow_height_left/right`; H:888 `brows_height` | None | S |
| CD-5.2-13 | Eyebrows: brow width | 5.2 | missing | None (searched brow_width, brow_spacing, brow_inner) | If it means the gap between the brows: new shape. If it means extent: see brow length | M |
| CD-5.2-14 | Eyebrows: brow arch | 5.2 | implemented | R:1474, R:1504 `brow_arch_left/right` | Stylized: none | S |
| CD-5.2-15 | Eyebrows: brow length | 5.2 | implemented | R:1534, R:1564 `brow_length_left/right` (moves the Brows card only) | Stylized: none | S |
| CD-5.2-16 | Eyebrows: brow thickness | 5.2 | missing | None (searched brow_thick, brow_density, brow_weight) | New card shape or a hair-material value | M |
| CD-5.2-17 | Eyebrows: brow angle | 5.2 | missing | None (searched brow_angle, brow_tilt, brow_slant) | New shape | M |
| CD-5.2-18 | Eyebrows: brow ridge | 5.2 | implemented | R:1594 `brow_ridge` | None | S |
| CD-5.2-19 | Nose: nose length | 5.2 | implemented | R:1650, H:924 `nose_length` | None | S |
| CD-5.2-20 | Nose: bridge width | 5.2 | missing | None (searched bridge_width, dorsum) | New shape | M |
| CD-5.2-21 | Nose: bridge height | 5.2 | implemented | R:1668, H:942 `nose_bridge` (moves the bridge 4.6 mm forward, `face.py:1499`) | Label is "Nose bridge" | S |
| CD-5.2-22 | Nose: bridge profile | 5.2 | partial | `nose_bridge` is one axis (R:1668) | No hump or scoop control (searched bridge_profile, nose_hump) | M |
| CD-5.2-23 | Nose: tip size | 5.2 | missing | None (searched tip_size, tip_width) | New shape | M |
| CD-5.2-24 | Nose: tip projection | 5.2 | partial | R:1686 `nose_tip` moves the tip up 3.8 mm and forward 3.0 mm together (`face.py:1495`) | Projection and rotation are one axis | M |
| CD-5.2-25 | Nose: tip rotation | 5.2 | partial | The same `nose_tip` (R:1686) | As above | M |
| CD-5.2-26 | Nose: nostril width | 5.2 | implemented | R:1612, R:1631 `nostril_width_left/right`; H:906 `nose_width` | None | S |
| CD-5.2-27 | Nose: nostril height | 5.2 | missing | None (searched nostril_height, alar) | New shape | M |
| CD-5.2-28 | Nose: nostril angle | 5.2 | missing | None (searched nostril_angle, nostril_flare) | New shape | M |
| CD-5.2-29 | Nose: overall nose width | 5.2 | partial | H:906 `nose_width`. Realistic: `nostril_width_*` drive `Face_Nose_Width_*`, the wings only (`face.py:1466`) | Realistic has no whole-nose width | M |
| CD-5.2-30 | Cheeks: cheekbone prominence | 5.2 | implemented | R:1723, R:1742 `cheekbone_left/right`; H:1068 `cheekbones` | None | S |
| CD-5.2-31 | Cheeks: cheek fullness | 5.2 | implemented | R:1761, R:1780 `cheek_fullness_left/right` | Stylized: none | S |
| CD-5.2-32 | Cheeks: midface width | 5.2 | missing | None (searched midface, mid_face, face_width) | New shape. Nearest: `cheekbone_*` (a 24 mm patch at the cheek point) and `temple_width` (a band above the eye); neither widens the midface as a whole | M |
| CD-5.2-33 | Cheeks: under-eye fullness | 5.2 | missing | None (searched under_eye, tear_trough, eye_bag) | New shape near the lids | M |
| CD-5.2-34 | Cheeks: cheek projection | 5.2 | partial | `cheekbone_*` and `cheek_fullness_*` both move the cheek out along its normal (`face.py:1468`, `:1470`), which is projection | No control of its own (searched cheek_proj, cheek_forward, cheek_depth); a forward-only move would be a new shape | M |
| CD-5.2-35 | Jaw: jaw width | 5.2 | implemented | R:1799, R:1818 `jaw_width_left/right`; H:1050 `jaw_width` | None | S |
| CD-5.2-36 | Jaw: jaw angle | 5.2 | missing | None (searched jaw_angle, gonial, jaw_corner) | New shape | M |
| CD-5.2-37 | Jaw: jaw depth | 5.2 | missing | None (searched jaw_depth, jaw_length, jaw_height) | New shape | M |
| CD-5.2-38 | Jaw: jaw definition | 5.2 | missing | None (searched jaw_def, jawline) | New shape (soft to sharp jawline); it interacts with `fat` | M |
| CD-5.2-39 | Jaw: jaw asymmetry | 5.2 | implemented | R:1837 `jaw_shift` (advanced); `jaw_width_left` and `_right` set apart | None | S |
| CD-5.2-40 | Chin: chin width | 5.2 | implemented | R:1892 `chin_width` | Stylized: none | S |
| CD-5.2-41 | Chin: chin height | 5.2 | implemented | R:1856, H:1014 `chin_length` | Named length | S |
| CD-5.2-42 | Chin: chin projection | 5.2 | implemented | R:1874, H:1032 `chin_forward` | None | S |
| CD-5.2-43 | Chin: chin shape | 5.2 | missing | None (searched chin_shape, chin_round, chin_square, chin_point) | New shape | M |
| CD-5.2-44 | Chin: cleft where supported | 5.2 | missing | None (searched cleft, dimple) | New shape or a normal-map detail | M |
| CD-5.2-45 | Mouth: mouth width | 5.2 | implemented | R:1910, R:1929 `mouth_width_left/right`; H:978 `mouth_width` | None | S |
| CD-5.2-46 | Mouth: mouth height | 5.2 | implemented | R:1984 `mouth_height` (the teeth follow) | Stylized: none | S |
| CD-5.2-47 | Mouth: upper lip fullness | 5.2 | implemented | R:1948 `upper_lip_fullness`; H:996 `lips_full` (both lips) | None | S |
| CD-5.2-48 | Mouth: lower lip fullness | 5.2 | implemented | R:1966 `lower_lip_fullness` | None | S |
| CD-5.2-49 | Mouth: cupid's bow | 5.2 | missing | None (searched cupid, lip_bow, vermilion) | New shape | M |
| CD-5.2-50 | Mouth: lip corners | 5.2 | missing | None as identity (searched lip_corner, mouth_corner). `Lip_Corner_Pull_*` are expression shapes | A new identity shape that must add to the expressions | M |
| CD-5.2-51 | Mouth: mouth projection | 5.2 | missing | None (searched mouth_proj, mouth_forward, lip_proj) | New shape; the teeth must follow | M |
| CD-5.2-52 | Mouth: philtrum length | 5.2 | missing | None (searched philtrum) | `mouth_height` and `nose_length` change it as a side effect (`face.py:1548`, `:1493`); no control of its own | M |
| CD-5.2-53 | Forehead: forehead height | 5.2 | implemented | R:2002 `forehead_height` | Stylized: none | S |
| CD-5.2-54 | Forehead: forehead slope | 5.2 | implemented | R:2020 `forehead_slope`, labelled "Forehead fullness" (`faceparams.py:71`) | Label and ID disagree. The shape pushes a band of the mid forehead forward or back (`face.py:1507-1508`): a fullness that changes the profile, not a turn about the brow | S |
| CD-5.2-55 | Forehead: forehead width | 5.2 | partial | `temple_width` (R:2038) is nearest | No own control (searched forehead_width) | M |
| CD-5.2-56 | Forehead: temple width | 5.2 | implemented | R:2038 `temple_width` (shape `Face_Head_Width`); H:1086 `head_width` | None | S |
| CD-5.2-57 | Forehead: brow-to-forehead transition | 5.2 | partial | R:1594 `brow_ridge` moves the band at the glabella along its normal (`face.py:1501-1504`) and R:2020 `forehead_slope` the band above it (`face.py:1507-1508`): together they set the step | No single control (searched glabella, forehead_transition) | M |
| CD-5.2-58 | Ears: ear size | 5.2 | implemented | R:2056, R:2075 `ear_size_left/right`; H:1122 `ears_size` | None | S |
| CD-5.2-59 | Ears: ear height | 5.2 | implemented | R:2094, R:2114 `ear_height_left/right` (advanced) | Stylized: none | S |
| CD-5.2-60 | Ears: ear protrusion | 5.2 | implemented | R:2134, R:2153 `ear_out_left/right`; H:1140 `ears_out` | None | S |
| CD-5.2-61 | Ears: ear rotation | 5.2 | missing | None (searched ear_rot, ear_angle, ear_tilt) | New shape | M |
| CD-5.2-62 | Ears: earlobe size | 5.2 | missing | None (searched earlobe, ear_lobe, lobe) | New shape | M |
| CD-5.2-63 | Ears: ear shape | 5.2 | missing | None (searched ear_shape, ear_point, helix) | New shape | M |
| CD-5.2-64 | Not every parameter is exposed at first; the visible ones come first | 5.2 | implemented | `advanced` hides six face parameters (`manifest.rs:118-120`; `character_workspace/ui.rs:444`); the pupil was left out on purpose (`faceparams.py:17-20`) | Two levels today; the brief wants three (CD-15-02) | S |
| CD-5.4-01 | Keep the existing symmetry system | 5.4 | implemented | `mirror` (`manifest.rs:121-124`), checked both ways (`manifest.rs:446-459`); 16 pairs (`faceparams.py:37-54`); `character_workspace.rs:477-490`; `tests.rs:183` | None in data. A pair's shared label is made in editor code, by cutting "Left" or "Right" out of the label (`character_workspace.rs:207-222`) | S |
| CD-5.4-02 | Symmetry on by default | 5.4 | implemented | `character_workspace.rs:359`; `tests.rs:183` | It is workspace state, not saved, which is right | S |
| CD-5.4-03 | Independent left and right editing | 5.4 | implemented | With Symmetry off both sides show (`character_workspace/ui.rs:444-452`); `tests.rs:201-204` | Stylized has no pairs (no `mirror` in H) | S |
| CD-5.4-04 | Mirror current values | 5.4 | missing | None (searched mirror_values, flip, swap_sides) | Needs no new data: `mirror` is enough. A pure function in `avatar` plus a button | S |
| CD-5.4-05 | Copy left to right | 5.4 | missing | None (searched copy_left, left_to_right) | As above | S |
| CD-5.4-06 | Copy right to left | 5.4 | missing | None (searched copy_right, right_to_left) | As above | S |
| CD-5.4-07 | Reset individual sides | 5.4 | partial | Double-click resets one slider (`character_workspace/ui.rs:486-488`); with Symmetry off that is one side | No named action, no test | S |
| CD-5.4-08 | Natural asymmetry allowed, never forced | 5.4 | partial | Per-side parameters default to 0; `nose_tilt` (R:1704) and `jaw_shift` (R:1837) | With Symmetry on a pair shows the first side's value and a drag writes it to both (`ui.rs:445-452`; `character_workspace.rs:483-485`): a saved asymmetry is lost without warning | S |
| CD-14-01 | Each asset has a manifest describing what it supports | 14 | implemented | 15 item manifests and 2 base manifests; every one is in the catalogue: `crates/avatar/tests/shipped_library.rs:132-155`; checked against its model: `shipped_library.rs:158` | The realistic body has only three items, all underwear; no hair | S |
| CD-14.1-01 | Capability-aware customization is a core architectural requirement | 14.1 | partial | Pages are drawn from the manifests (`character_workspace/ui.rs:422`, `:511`); items declare slot, bases, materials and shading model (`item.rs:19-57`) | No per-item parameters; a material's values come from its shading model, the same for every item (`material.rs:283`) | L |
| CD-14.1-02 | Every hairstyle, garment and accessory declares the properties it supports | 14.1 | partial | `item.rs:19-57`: `materials`, `variants`, `hides`, `fit`, `bases`, `slot` | No list of supported parameters or of supported material values | M |
| CD-14.1-03 | Hair example: primary colour | 14.1 | partial | Hair shading `tint`, `melanin`, `redness`, `dye` (`material.rs:226-249`); stored per worn item (`description.rs:49-50`); checked (`description.rs:185-193`) | No control in the workspace; `scene` reads only `tint` and `roughness_scale` until CO7 (`crates/scene/src/material.rs:153-159`) | M |
| CD-14.1-04 | Hair example: highlight colour | 14.1 | partial | Four stylized hairs have a second material, `HairSecondary`, made for highlights (`assets/characters/humanoid/hair.py:4`, `:169`); its `tint` and `dye` are the highlight colour, set per worn item (`description.rs:49-50`). `highlight` itself is a strength (`material.rs:258`) | No control in the workspace, and nothing tells a UI that this slot means highlights (no slot label). The other two hairs have one slot | M |
| CD-14.1-05 | Hair example: length | 14.1 | missing | `ItemManifest` has no parameters (`item.rs:19-57`) (searched hair_length, "length") | Needs item parameters and a groom that has the shape | L |
| CD-14.1-06 | Hair example: volume | 14.1 | missing | The same (searched volume) | The same | L |
| CD-14.1-07 | Hair example: parting | 14.1 | missing | Only a tag, `side-part` (`assets/characters/humanoid/items/hair-swept.item.json`) (searched parting) | The same, or variants | L |
| CD-14.1-08 | Another hairstyle may support fewer properties | 14.1 | partial | Items already differ by material slot: two stylized hairs declare `Hair` only, four declare `Hair` and `HairSecondary` (`assets/characters/humanoid/items/hair-*.item.json`), checked against the model (`shipped_library.rs:158-190`) | Inside a slot every `hair` material gets the same eight values (`material.rs:226-271`): an item can't narrow or extend them. No UI shows any of it | M |
| CD-14.1-09 | The interface shows only supported controls | 14.1 | partial | Slots list only items whose `bases` include the body, and empty slots are hidden (`character_workspace.rs:492-500`; `ui.rs:535-539`) | There are no item property controls at all yet | M |
| CD-14.1-10 | Capability metadata: supported parameters | 14.1 | partial | The only item-level settings are material values: fixed by each slot's shading model (`material.rs:283-296`), set per worn item (`description.rs:49-50`), checked (`description.rs:185-193`), resolved (`resolve.rs:157-158`) | No parameters of the item's own (length, volume): `item.rs:19-57` has none. Add `parameters` to the item manifest | M |
| CD-14.1-11 | Capability metadata: supported parameter ranges | 14.1 | partial | Material values have ranges, constants per shading model (`material.rs:132-271`), enforced on worn items (`description.rs:185-193`) | Not per item, and none for item parameters, which don't exist. Comes with item `parameters` | M |
| CD-14.1-12 | Capability metadata: default values | 14.1 | partial | Each material value has a neutral setting that leaves the material as made (`material.rs:93-98`); a variant is a named set of values (`item.rs:107-112`) | Per shading model, not per item; no item parameters to have defaults. Comes with item `parameters`; variants can carry defaults | S |
| CD-14.1-13 | Capability metadata: material slots | 14.1 | implemented | `materials: [{name, shading}]` (`item.rs:42-44`; `material.rs:36-39`); checked against the model (`shipped_library.rs:158-190`) | No friendly label per slot | S |
| CD-14.1-14 | Capability metadata: compatible body families | 14.1 | implemented | `bases` (`item.rs:34-35`); checked (`library.rs:118-129`; `description.rs:152-158`); in the catalogue (`catalogue.rs:48`) | By exact base model, not by family (`manifest.rs:22-24`) | S |
| CD-14.1-15 | Capability metadata: attachment requirements | 14.1 | partial | `slot` and `fit` (shapes, binding, socket) (`item.rs:25-26`, `:83-99`); `check_against` (`item.rs:285-310`) | Every shipped item uses `shapes`. `binding` is not fitted yet (`resolve.rs:164-166`). No item can fill two slots | M |
| CD-14.1-16 | Capability metadata: supported variants | 14.1 | partial | `variants` (`item.rs:45-46`, `:107-112`), checked (`item.rs:257-271`); a description picks one (`description.rs:47-48`) | No shipped item has a variant (0 of 15); no control in the workspace | S |
| CD-14.1-17 | Capability metadata: supported customization operations | 14.1 | missing | None (searched operations, capabilit, supports) | Mostly derivable from the other fields; see section 4.3 | S |
| CD-14.1-18 | Capability metadata: required dependencies | 14.1 | missing | None (searched requires, depends, dependenc) | Add `requires` | S |
| CD-14.1-19 | Capability metadata: known compatibility limitations | 14.1 | missing | None (searched conflict, incompatib, exclusive, limitation). `layer` only orders garments (`item.rs:27-31`) | Add `excludes` and `limitations` | S |
| CD-14.1-20 | No universal controls that do nothing for an asset | 14.1 | partial | No item controls exist yet | Drawing every value of a shading model for every item would make them (a one-material hair would get `highlight`) | S |
| CD-14.1-21 | The manifest is extensible and versioned | 14.1 | partial | `format_version`, newer refused (`crates/avatar/src/lib.rs:117-136`; `src/tests/formats.rs:135-151`); unknown fields are ignored on read (no `deny_unknown_fields` anywhere) | Unknown fields are dropped when a manifest or catalogue is written again; a new enum value makes an older reader reject the file (section 4.6) | S |
| CD-15-01 | Parameter metadata: friendly descriptions | 15 | missing | `Parameter` has `label` only (`manifest.rs:110-138`) (searched about, description, hint) | Add `about` | S |
| CD-15-02 | Parameter metadata: categories and subcategories | 15 | partial | `Group {id, label, category}` (`manifest.rs:87-104`); `Parameter.group` (`manifest.rs:113-114`) | Two levels. No feature under a group, no detail level beyond `advanced` | S |
| CD-15-03 | Parameter metadata: tooltips | 15 | missing | None in data (searched tooltip, hint, help_text); the only hover texts are code strings on the two toggles (`character_workspace/ui.rs:430-432`) | `about` serves as the tooltip | S |
| CD-15-04 | Parameter metadata: visual icons | 15 | missing | None in data (searched icon); category icons are code (`character_workspace/ui.rs:37-46`) | Add `icon` (a name) to groups and features | S |
| CD-15-05 | Parameter metadata: supported ranges | 15 | implemented | `range` (`manifest.rs:115`), checked (`manifest.rs:415-420`), enforced (`resolve.rs:27-44`; `description.rs:132-140`) | None | S |
| CD-15-06 | Parameter metadata: default values | 15 | implemented | `default` (`manifest.rs:116-117`), checked inside the range (`manifest.rs:421-429`) | None | S |
| CD-15-07 | Parameter metadata: preset groupings | 15 | missing | Nothing links parameters to preset kinds (searched preset in `crates/avatar`) | Derivable from group and category; the preset file names its kind | S |
| CD-15-08 | Parameter metadata: symmetry relationships | 15 | implemented | `mirror` (`manifest.rs:121-124`), checked (`manifest.rs:446-459`) | None | S |
| CD-15-09 | Parameter metadata: viewport manipulation metadata | 15 | missing | None (searched handle, gizmo, anchor, landmark in `crates/avatar` and both manifests) | Add `handles`. The build already measures each shape's largest move (`Realistic.build.json`, `face.largest_move_mm`); anchors are not exported | M |
| CD-15-10 | Asset metadata: preview thumbnails | 15 | partial | `thumbnail` on items (`item.rs:52-54`) and in the catalogue (`catalogue.rs:51-52`), checked and copied (`library.rs:136-138`; `crates/project/src/character_library.rs:146`) | No item has one (0 of 15); nothing draws them; bases and variants have none | M |
| CD-15-11 | Asset metadata: supported body families | 15 | implemented | `bases` on items; `family` on bases (`manifest.rs:22-24`; `catalogue.rs:30-35`) | None | S |
| CD-15-12 | Asset metadata: supported customization options | 15 | partial | Materials and variants only (`item.rs:42-46`) | See CD-14.1-10 | M |
| CD-15-13 | Asset metadata: material variants | 15 | partial | `variants` (`item.rs:107-112`) | No content, no control (CD-14.1-16) | S |
| CD-15-14 | Asset metadata: compatibility rules | 15 | partial | `bases`, `slot` (one item per slot unless `multiple`: `description.rs:159-175`), `layer`, `hides` | No requires or excludes | S |
| CD-15-15 | Asset metadata: asset tags | 15 | partial | `tags` (`item.rs:50-51`), in the catalogue (`catalogue.rs:49-50`); all 15 items have them | Nothing reads them: no search or filter in the workspace | S |
| CD-15-16 | Asset metadata: capability declarations | 15 | partial | See CD-14.1-01 and CD-14.1-02 | See section 4.3 | M |
| CD-15-17 | The `avatar` crate stays the source of truth for character data | 15 | partial | `editor`, `gameplay`, `player` and `project` all read it (their `Cargo.toml`); `crates/avatar/CLAUDE.md` | Presets live in the editor (`character_workspace.rs:102-190`), not in `avatar` | S |
| CD-15-18 | No editor-only selection is stored as character identity | 15 | implemented | `Description` holds values only (`description.rs:1-4`, `:20-40`); symmetry, advanced, category and camera are workspace fields (`character_workspace.rs:276-282`) | Keep it so: favourites, locks and selections go in workspace files | S |
| CD-15-19 | The studio stays a consumer of the shared description and asset systems (outside the two lists; added in verification) | 15 | implemented | The preview is the game's applier (`character_workspace.rs:54`, `:683`); a saved character applied in a game uses the same models: `character_workspace/tests.rs:473` | Keep it so when handles and presets arrive | S |
| CD-15-20 | Every editor interaction updates the character description (outside the two lists; added in verification) | 15 | implemented | Every change goes through `edit` or `drag` on the `Description` (`character_workspace.rs:429-459`); sliders, choices, items, presets and reset all call them (`character_workspace.rs:477-604`) | Handles and Randomize must use the same two functions | S |

## 2. Every parameter the two families have

So nothing is lost when controls are renamed or regrouped. "Brief" is the control of §4.1, §4.2 or §5.2 the parameter serves; "none" means the brief does not name it.

### Realistic (`axcenity/realistic`): 75 parameters

Body, 28. Proportions are attributes (`anim`'s body sliders); muscle and fat drive shapes only through corners with `frame` (55 corners, R:2172).

| ID | Label | Group | Range | Default | Advanced | Brief |
|---|---|---|---|---|---|---|
| `frame` | Body type | frame | 0 to 1 (choices Male, Female) | 0 | no | Body frame |
| `height` | Height | proportions | 0.85 to 1.15 | 1 | no | Height |
| `leg_length` | Leg length | proportions | 0.85 to 1.15 | 1 | yes | Leg proportions |
| `arm_length` | Arm length | proportions | 0.85 to 1.15 | 1 | yes | Arm proportions |
| `shoulder_width` | Shoulder width | proportions | 0.85 to 1.15 | 1 | yes | Shoulder width |
| `waist_width` | Waist width | proportions | 0.85 to 1.15 | 1 | yes | Waist width |
| `hip_width` | Hip width | proportions | 1.0 to 1.15 | 1 | yes | Hip width |
| `head_size` | Head size | proportions | 0.85 to 1.15 | 1 | yes | Head-to-body ratio |
| `hand_size` | Hand size | proportions | 0.85 to 1.15 | 1 | yes | Region: hands |
| `foot_size` | Foot size | proportions | 0.85 to 1.15 | 1 | yes | Region: feet |
| `muscle` | Muscle | build | −1 to 1 | 0 | no | Muscle definition, overall build |
| `fat` | Body fat | build | −1 to 1 | 0 | no | Body fat, overall build |
| `muscle_chest`, `fat_chest` | Chest muscle, Chest fat | build_regions | −1 to 1 | 0 | yes | Chest proportions; region chest |
| `muscle_shoulders`, `fat_shoulders` | Shoulders muscle, fat | build_regions | −1 to 1 | 0 | yes | Region shoulders; deltoid volume |
| `muscle_arms`, `fat_arms` | Arms muscle, fat | build_regions | −1 to 1 | 0 | yes | Arm proportions; regions upper arms, forearms |
| `muscle_abdomen`, `fat_abdomen` | Abdomen muscle, fat | build_regions | −1 to 1 | 0 | yes | Region abdomen; both abdomen examples |
| `muscle_back`, `fat_back` | Back muscle, fat | build_regions | −1 to 1 | 0 | yes | Region back |
| `muscle_hips`, `fat_hips` | Hips muscle, fat | build_regions | −1 to 1 | 0 | yes | Region hips |
| `muscle_thighs`, `fat_thighs` | Thighs muscle, fat | build_regions | −1 to 1 | 0 | yes | Region thighs; leg proportions |
| `muscle_calves`, `fat_calves` | Calves muscle, fat | build_regions | −1 to 1 | 0 | yes | Region calves; leg proportions |

Face, 47. Every one is −1 to 1 with default 0 and drives one identity shape (two for the brow pairs). A pair is `<id>_left` and `<id>_right`, each naming the other as `mirror` (`faceparams.py:37-54`); singles are `faceparams.py:57-73`.

| ID | Pair | Group | Advanced | Brief |
|---|---|---|---|---|
| `eye_height` | yes | eyes | no | Eye height |
| `eye_spacing` | yes | eyes | no | Eye spacing |
| `eye_depth` | yes | eyes | yes | Eye depth |
| `eye_size` | yes | eyes | no | Eye size (and eyeball proportions, partly) |
| `eye_tilt` | yes | eyes | no | Eye tilt |
| `brow_height` | yes | brows | no | Brow height |
| `brow_arch` | yes | brows | no | Brow arch |
| `brow_length` | yes | brows | no | Brow length |
| `brow_ridge` | no | brows | no | Brow ridge |
| `nostril_width` | yes | nose | no | Nostril width (and overall nose width, partly) |
| `nose_length` | no | nose | no | Nose length |
| `nose_bridge` | no | nose | no | Bridge height (and bridge profile, partly) |
| `nose_tip` | no | nose | no | Tip projection and tip rotation, as one axis |
| `nose_tilt` | no | nose | yes | **none** (the nose to one side; it belongs with §5.4's asymmetry) |
| `cheekbone` | yes | cheeks | no | Cheekbone prominence |
| `cheek_fullness` | yes | cheeks | no | Cheek fullness |
| `jaw_width` | yes | jaw | no | Jaw width |
| `jaw_shift` | no | jaw | yes | Jaw asymmetry |
| `chin_length` | no | chin | no | Chin height |
| `chin_forward` | no | chin | no | Chin projection |
| `chin_width` | no | chin | no | Chin width |
| `mouth_width` | yes | lips | no | Mouth width |
| `upper_lip_fullness` | no | lips | no | Upper lip fullness |
| `lower_lip_fullness` | no | lips | no | Lower lip fullness |
| `mouth_height` | no | lips | no | Mouth height |
| `forehead_height` | no | forehead | no | Forehead height |
| `forehead_slope` | no | forehead | no | Forehead slope (labelled "Forehead fullness") |
| `temple_width` | no | forehead | no | Temple width |
| `ear_size` | yes | ears | no | Ear size |
| `ear_height` | yes | ears | yes | Ear height |
| `ear_out` | yes | ears | no | Ear protrusion |

The only realistic parameter the brief does not name is `nose_tilt`. Every `Face_*` and body shape of `RealisticParts.glb`'s `Body` is driven by a parameter or a corner; the shapes no parameter drives are the six shoulder correctives and the expression shapes. The `Eyes` part has one more shape, `Pupil_Size`, with no parameter (`faceparams.py:17-20`).

### Stylized (`axcenity/stylized`): 31 parameters

No pairs, no `mirror`, no corners. Body: `frame`, `build`, `muscle`, and the same nine proportions at 0.5 to 1.5 (H:70 to H:726). Face, all −1 to 1: `eyes_spacing`, `eyes_height`, `eyes_size`, `eyes_tilt`, `brows_height`, `nose_width`, `nose_length`, `nose_bridge`, `nose_tip`, `mouth_width`, `lips_full`, `chin_length`, `chin_forward`, `jaw_width`, `cheekbones`, `head_width`, `head_length`, `ears_size`, `ears_out` (H:746 to H:1140). `build` is the brief's overall build and `head_width` its temple width; the brief does not name `head_length`.

IDs the two families share: `frame`, `muscle`, the nine proportions, `nose_length`, `nose_bridge`, `nose_tip`, `chin_length`, `chin_forward`. Every other face ID differs (`eyes_size` against `eye_size_left`), so a preset or a handle is per base model.

## 3. Fields today against the brief's metadata lists

### 3.1 A parameter (`manifest.rs:110-138`)

Today: `id`, `label`, `group`, `range`, `default`, `advanced`, `mirror`, `shapes`, `bones`, `attributes`, `material_values`, `choices` (`{value, label, items}`). A group is `{id, label, category}`; categories are a closed list: `body`, `face`, `skin`, `hair`, `clothing`, `accessories` (`manifest.rs:95-104`).

| Brief §15, Parameter metadata | Field today | Missing |
|---|---|---|
| Friendly descriptions | none | yes |
| Categories and subcategories | `group`, the group's `category` | a third level |
| Tooltips | none | yes |
| Visual icons | none | yes |
| Supported ranges | `range` | no |
| Default values | `default` | no |
| Preset groupings | none (derivable from `group`) | a rule, not a field |
| Symmetry relationships | `mirror` | no |
| Viewport manipulation metadata | none | yes |

### 3.2 A slot (`manifest.rs:273-282`)

Today: `id`, `label`, `category`, `multiple`, `empty_label`. Both families have the same seven: `hair`, `underwear_top`, `underwear_bottom`, `top`, `outerwear`, `bottom`, `shoes` (`assets/characters/humanoid/manifests.py:87-95`). There is no accessory slot yet, though the `accessories` category exists; adding slots is a data change. Sockets are `{name, bone}` (`manifest.rs:287-290`), seven on each body.

### 3.3 An item (`item.rs:19-57`)

Today: `format_version`, `id`, `label`, `kind` (garment, accessory, groom), `slot`, `layer`, `tier`, `bases`, `model {file, object}`, `hides`, `materials [{name, shading}]`, `variants [{id, label, materials}]`, `fit` (`shapes {follows}`, `binding {data, margin}`, `socket {socket}`), `groom {cards, strands, mesh}`, `tags`, `thumbnail`, `source {author, licence, url}`.

Used by the 15 shipped items: all but `variants` and `thumbnail` (none has either); every `fit` is `shapes`; no accessory exists.

| Brief | Field today | Missing |
|---|---|---|
| §14.1 Supported parameters | material values only, by each slot's shading model | the item's own parameters |
| §14.1 Supported parameter ranges | none (material value ranges are per shading model, in code) | yes |
| §14.1 Default values | none (neutral values per shading model, in code) | yes |
| §14.1 Material slots | `materials` (a second slot is how four hairs offer highlights) | a label, and which values the item offers |
| §14.1 Compatible body families | `bases` | no |
| §14.1 Attachment requirements | `slot`, `fit`, `layer`, `hides` | an item that fills two slots |
| §14.1 Supported variants | `variants` | a picture per variant |
| §14.1 Supported customization operations | none | derivable |
| §14.1 Required dependencies | none | yes |
| §14.1 Known compatibility limitations | none | yes |
| §15 Preview thumbnails | `thumbnail` | content |
| §15 Supported body families | `bases` | no |
| §15 Supported customization options | `materials`, `variants` | item parameters |
| §15 Material variants | `variants` | content |
| §15 Compatibility rules | `bases`, `slot`, `layer` | requires, excludes |
| §15 Asset tags | `tags` | no |
| §15 Capability declarations | the manifest as a whole | the rows above |

### 3.4 The catalogue (`catalogue.rs:17-53`)

Today: `format_version`; `bases [{id, manifest, label, family}]`; `items [{id, manifest, label, kind, slot, tier, bases, tags?, thumbnail?}]`. `Library::read` opens every manifest the catalogue lists (`library.rs:76-103`), so the workspace always has the full manifests in memory; the catalogue is an index, not a cache anyone depends on. It lists no presets.

### 3.5 A description (`description.rs:20-51`)

Today: `format_version`, `name`, `base`, `parameters {id: number}`, `items [{id, variant?, materials?}]`, `materials {slot: values}`, and `extra` (any other top-level field, kept).

## 4. Proposal: the fewest new optional fields

Names and shapes only. Every field is optional with a default, so no `format_version` changes: base 1, item 1, catalogue 1, description 2. Section 4.6 says how old files keep loading, with what was checked.

### 4.1 Base manifest

On a **parameter**:

```json
{
  "id": "nostril_width_left", "label": "Left nostril width", "group": "nose",
  "range": [-1, 1],
  "about": "How wide the nostril flares.",
  "feature": "nose",
  "level": "detailed"
}
```

| Field | Type | Meaning | Absent means |
|---|---|---|---|
| `about` | string | One sentence. The friendly description and the tooltip | No tooltip |
| `feature` | feature ID | The selectable thing on the character it belongs to | Its group's ID, when a feature has that ID; else none |
| `level` | `"quick"`, `"detailed"`, `"advanced"` | Brief §3.3's three levels | `"advanced"` when `advanced` is true, else `"detailed"` |
| `random` | `[min, max]` | The part of the range Randomize draws from (can wait for Stage 10) | A UI default (the middle of the range) |

`advanced` stays. Generators write both, with `advanced` true exactly when `level` is `"advanced"`, so an older editor still hides the same sliders. A check reports a manifest where the two disagree.

On a **group**: `about` (string) and `icon` (string: a name from the editor's icon set; an unknown name draws no icon).

New top-level list **`features`**: what a person can point at. It is the "subgroup" and it carries the selection and camera data.

```json
"features": [
  { "id": "nose", "label": "Nose", "category": "face", "icon": "nose",
    "about": "Length, bridge, tip and nostrils.",
    "anchors": [ { "bone": "Head", "offset": [0.0, 0.035, 0.118] } ],
    "radius": 0.035 },
  { "id": "shoulders", "label": "Shoulders", "category": "body", "icon": "shoulders",
    "anchors": [ { "bone": "LeftShoulder", "offset": [0.06, 0.0, 0.0] },
                 { "bone": "RightShoulder", "offset": [-0.06, 0.0, 0.0] } ],
    "radius": 0.12, "regions": ["upper_arms_top"] }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id`, `label`, `category` | string, string, category | As a group's |
| `about`, `icon` | string | As above |
| `anchors` | `[{bone, offset}]` | Points on the character, each a bone and an offset in metres in model space at rest (glTF axes). One for a middle feature, two for a pair (left first). The camera focuses on them; a click picks the feature whose anchor is nearest within `radius` |
| `radius` | metres | How far around an anchor counts, and how much the camera frames |
| `regions` | `[region ID]` | Optional: body regions (the region map's) that also pick this feature |

A parameter with `feature: "shoulders"` can stay in group `proportions`; the group orders the slider page, the feature drives selection. That puts `shoulder_width`, `muscle_shoulders` and `fat_shoulders` under one region without moving them. For the face the features are the nine groups that exist.

New top-level list **`handles`**: viewport manipulation.

```json
"handles": [
  { "id": "nostril_left", "feature": "nose", "mirror": "nostril_right",
    "anchor": { "bone": "Head", "offset": [0.017, 0.028, 0.105] },
    "drives": [ { "parameter": "nostril_width_left", "axis": [1, 0, 0], "travel": 0.0046 } ] },
  { "id": "nose_tip", "feature": "nose",
    "anchor": { "bone": "Head", "offset": [0.0, 0.03, 0.125] },
    "drives": [ { "parameter": "nose_length", "axis": [0, -1, 0], "travel": 0.005 },
                { "parameter": "nose_tip",    "axis": [0, 0.78, 0.62], "travel": 0.0048 } ] },
  { "id": "eye_tilt_left", "feature": "eyes", "mirror": "eye_tilt_right",
    "anchor": { "bone": "LeftEye", "offset": [0.016, 0.0, 0.01] },
    "drives": [ { "parameter": "eye_tilt_left", "axis": [0, 1, 0], "travel": 0.0054,
                  "screen": { "front": [0, 1] } } ],
    "views": ["front", "three_quarter"] }
]
```

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Unique in the manifest |
| `feature` | feature ID | Shown when that feature is selected |
| `label` | string | Optional; the driven parameter's label when absent |
| `anchor` | `{bone, offset}` | Where the handle sits at the parameter's default |
| `drives` | one or two `{parameter, axis, travel, screen?, sensitivity?}` | What a drag changes |
| `drives[].axis` | `[x, y, z]` | Model-space direction the surface moves for a rising value. The editor projects it into the view, so one vector serves every camera; a view where it points at the camera gets no handle |
| `drives[].travel` | metres | How far the surface at the anchor moves over the whole range. With it the handle follows the cursor one to one: this is the sensitivity |
| `drives[].sensitivity` | number | Optional multiplier on that (default 1) |
| `drives[].screen` | `{view: [dx, dy]}` | Optional per-view override, as a screen direction (x right, y up), for parameters that are not a straight move (tilt, size) |
| `mirror` | handle ID | The partner on the other side; each names the other. With Symmetry on, dragging one moves both |
| `views` | `["front", "side", "three_quarter", "back"]` | Optional: only show the handle in these framings |

`travel` can be generated: the build already writes each shape's largest move (`Realistic.build.json`, `face.largest_move_mm`, for example 4.41 mm for `Face_Nose_Width_Left`). Anchors are known to the build (`face.py`'s landmarks) but are not exported yet. Nothing here is a second editing state: a handle only names parameters.

Checks to add with the fields (`BaseManifest::validate`, `check_model`): feature and handle IDs unique; a parameter's `feature`, a handle's `feature` and `drives[].parameter` exist; `mirror` names back; anchor bones exist in the model; `axis` is finite and not zero; `travel` and `radius` are above 0; `level` and `advanced` agree.

### 4.2 Slot

`about` and `icon`, as on a group. Nothing else: accessory slots (`glasses`, `hat`, `earrings` with `multiple`) are new entries, not new fields.

### 4.3 Item manifest: capabilities

The manifest stays the capability declaration; these fields complete it.

```json
{
  "format_version": 1, "id": "axcenity/hair-bob", "label": "Bob", "kind": "groom", "slot": "hair",
  "bases": ["axcenity/realistic"],
  "about": "A chin-length bob.",
  "materials": [
    { "name": "Hair", "shading": "hair", "label": "Colour",
      "values": ["tint", "melanin", "redness", "dye", "dye_amount"] },
    { "name": "HairSecondary", "shading": "hair", "label": "Highlights", "values": ["tint"] }
  ],
  "parameters": [
    { "id": "length", "label": "Length", "range": [0, 1], "default": 0.5, "level": "detailed",
      "about": "From jaw to shoulder.", "shapes": [ { "shape": "Hair_Length" } ] },
    { "id": "parting", "label": "Parting", "range": [0, 2], "default": 0,
      "choices": [ { "value": 0, "label": "Middle" }, { "value": 1, "label": "Left" }, { "value": 2, "label": "Right" } ],
      "shapes": [ { "shape": "Hair_Part_Left", "from": [0, 1] }, { "shape": "Hair_Part_Right", "from": [1, 2] } ] }
  ],
  "variants": [ { "id": "blonde", "label": "Blonde", "thumbnail": "thumbs/bob-blonde.png",
                  "materials": { "Hair": { "melanin": -0.6 } }, "parameters": { "length": 0.3 } } ],
  "also_fills": [],
  "requires": [],
  "excludes": ["axcenity/realistic-hood"],
  "limitations": [ { "note": "Clips through high collars.", "parameter": "length", "range": [0, 0.7] } ]
}
```

| Brief §14.1 | Field | Type | Meaning | Absent means |
|---|---|---|---|---|
| Supported parameters, ranges, defaults | `parameters` | list shaped like a base parameter: `id`, `label`, `range`, `default`, `about`, `level`, `choices`, `shapes`, `material_values` | The item's own sliders. `shapes` name shapes of the item's model; `material_values` name its own material slots. No `group`, no `mirror` | No sliders: the UI shows none |
| Material slots | `materials[].label` | string | The slot's name for people ("Colour", "Highlights") | The material's name |
| Material slots | `materials[].values` | list of value names of the slot's shading model | The values this item offers | Every value of the shading model, as today |
| Supported variants | `variants[].thumbnail`, `variants[].parameters` | path; map ID to number | A picture; values for the item's parameters | No picture; no values |
| Compatible body families | `bases` (exists) | | The family is the base's `family` | |
| Attachment requirements | `slot`, `fit`, `layer`, `hides` (exist); `also_fills` | list of slot IDs | Other slots the item occupies (a dress: `bottom`) | Only its slot |
| Required dependencies | `requires` | list of item IDs | Items that must be worn with it | None |
| Known compatibility limitations | `excludes` | list of item IDs | Items it can't be worn with | None |
| Known compatibility limitations | `limitations` | `[{note, parameter?, range?}]` | A note for the browser; with `parameter` and `range` (a base or item parameter) the editor can warn outside the range | None |
| Supported customization operations | none | | Derived: has variants; has material slots with values (recolour); has parameters. An `operations: [string]` list can be added later if something can't be derived | |
| Friendly description | `about` | string | Shown in the browser | None |

The value of an item parameter goes in the description, on the worn item, beside `variant` and `materials`:

```json
"items": [ { "id": "axcenity/hair-bob", "variant": "blonde", "parameters": { "length": 0.4 } } ]
```

Resolution reuses `span` and the driver types; the applier bakes an item's parameter shapes into the item's derived model as it bakes the body's. That part is real work in `resolve.rs` and `gameplay`, not only schema.

Checks to add: parameter IDs unique in the item; ranges and defaults as for a base; `materials[].values` are values of the slot's shading model; `also_fills` slots exist on every base; `requires` and `excludes` are IDs (the library check reports ones it doesn't have); a description's `items[].parameters` name parameters the item has, in range; an item whose `requires` is not worn, or that is worn with one it excludes, is a `Problem`, and `resolve` leaves it out and says why, as it does for a second item in a slot today.

### 4.4 Catalogue

One new list, so the library lists presets without a folder scan:

```json
"presets": [
  { "id": "axcenity/body-athletic", "file": "realistic/presets/body-athletic.preset.json",
    "label": "Athletic", "kind": "body", "base": "axcenity/realistic",
    "tags": ["build"], "thumbnail": "realistic/presets/body-athletic.png" }
]
```

Nothing else. The catalogue needs no capability summary because `Library::read` opens every manifest. `catalogue.py` and `Catalogue::of` must gain the list together: a test holds them equal (`crates/avatar/tests/shipped_library.rs:139-143`).

### 4.5 Description

`items[].parameters` (4.3). And `WornItem` should keep fields it doesn't know, as the top level does (an `extra` map), so the next addition to a worn item survives an older editor.

### 4.6 How old files keep loading (checked)

| Case | What happens | Evidence |
|---|---|---|
| A new engine reads an old manifest, catalogue or character | Every proposed field has a default, so it reads to the same value as before. (The tests compare values, not bytes: the Python generators write `1` where serde writes `1.0`. Nothing in the engine writes a manifest back; it copies the file) | Existing optional fields work this way (`manifest.rs:27-48`, `item.rs:30-56`); round-trip tests `crates/avatar/src/tests/formats.rs:113-132` |
| An old engine reads a new manifest or catalogue | It loads. Unknown fields are ignored: no struct uses `deny_unknown_fields` | Searched `crates/avatar/src`: none |
| An old engine reads a character with unknown top-level fields | It loads and keeps them on save | `description.rs:36-39`; test `formats.rs:60-69` |
| An old engine saves a character with `items[].parameters` | **It drops them.** `WornItem` has no `extra` | `description.rs:44-51` |
| Any engine writes a manifest or catalogue again from its typed value | **Unknown fields are dropped.** The engine does this only to a project's catalogue today; manifests are copied as files (`character_library.rs:112`) | `manifest.rs:16-49`, `item.rs:18-57`, `catalogue.rs:16-53` have no `extra`; `crates/project/src/character_library.rs:158` |
| An old engine copies a package into a folder whose catalogue lists `presets` (4.4) | **It writes the catalogue again without the list** | `crates/project/src/character_library.rs:155-159` |
| An installer is built from a library with preset files, variant pictures or a groom mesh of its own | **They are not staged.** The payload takes only what the catalogue's bases and items name: manifest, model, groom cards, strands, thumbnail | `packaging/stage.py:60-78` |
| A new manifest uses a new enum value (a new `category`, `kind`, `fit.method`, `constraints[].kind`, `shading`, `tier`) | **An old engine rejects the whole file** as damaged, and the library drops that base or item | Closed serde enums: `manifest.rs:95-104`, `:240-255`; `item.rs:59-68`, `:81-99`; `material.rs:17-33`; `library.rs:77-103` |
| A file has a higher `format_version` | Refused with a message | `crates/avatar/src/lib.rs:117-136`; `formats.rs:135-151` |

So the proposal adds fields only, and no enum value. If a later stage needs a new category (makeup) or fit method, that is a version bump with a migration, or the enum gets an "unknown" catch-all first.

## 5. Presets as data files

**Today.** Six presets are constants in the editor (`character_workspace.rs:145-190`): a name, a sentence, a base, parameter values and optional items. Each makes a whole `Description` from the base's default character (`character_workspace.rs:113-131`). The start page and the picture cache address them by index (`character_workspace.rs:387`; `character_workspace/thumbnails.rs:77`). The benchmark characters already are description files written by a script (`assets/characters/benchmarks/make.py`), checked by a test (`shipped_library.rs:229`).

**Already written down.** The plan's CO13 is "character presets and outfit presets as data" (`plans/axcenity-engine-character-studio.md:148`), and `architecture.md:368` sketches them as "descriptions without `name`". That sketch does not read today: `name` is required (`description.rs:22`). The proposal below keeps `name`, so no parser changes. The nearest thing in data today is a character pack (`crates/editor/src/character_workspace/packs.rs:1-19`): a named list of saved characters, exported with each description in full. A whole-character preset set could ship as one.

**Proposal: a preset is a format 2 description with one extra block.** No new parser.

```json
{
  "format_version": 2,
  "name": "Athletic",
  "base": "axcenity/realistic",
  "parameters": { "muscle": 0.7, "fat": -0.4 },
  "preset": {
    "id": "axcenity/body-athletic",
    "kind": "body",
    "about": "Lean and muscular.",
    "tags": ["build"],
    "thumbnail": "body-athletic.png",
    "order": 30
  }
}
```

- `Description::from_json` reads it today: `preset` lands in `extra` and survives a save (`description.rs:36-39`). `Description::validate` checks it against the library today. The picture painter already draws a description.
- The file ends `.preset.json`, so the saved-characters shelf and the manifest scans don't pick it up. Shipped ones live in the package: `assets/characters/realistic/presets/`. The catalogue lists them (4.4).
- A person's own presets are the same files in the workspace's folder (`presets/`, beside `characters/`), found by a folder scan as saved characters are. "Favourite" is editor state: a list in a workspace file, like the Recent list, never in the preset.
- `kind` says what applying it changes. Everything else in the character is left alone:

| `kind` | Applying it sets | Left out of the file means |
|---|---|---|
| `body` | Every parameter whose group's category is `body` | The parameter's default. A parameter with `choices` (Male or Female) changes only when the file names it |
| `face` | Every parameter whose group's category is `face` | The default: a face preset is a complete face (brief §5.1) |
| `hair` | The items in `hair` slots, with their variant, materials and parameters | Bald |
| `outfit` | The items in `clothing` and `accessories` slots, except slots a choice fills (`chosen_slots`, `crates/avatar/src/choices.rs:10`) | Nothing worn there |
| `skin` | The base's `materials` values | Neutral |
| `character` | Everything but the name | As a description |

- That is the "preset grouping" of §15: a parameter belongs to a preset kind through its group's category. No field on the parameter.
- One function in `avatar`, `apply_preset(&mut Description, &Description, &Library)`, does it, so the editor, a script and Randomize share the rule. `Custom` is simply "no preset of this kind matches", which `is_preset` already computes.
- A preset fits one base model, like any description, because parameter IDs differ between the families (section 2).
- The eight body presets of §4.1 can be written today from `muscle`, `fat` and the proportions, one file each instead of one per frame (a body preset leaves `frame` alone). The nine face presets of §5.1 can be drafted from the 47 face parameters, but Long, Broad, Narrow, Soft and Angular will look better once the missing jaw and midface controls exist; an artist should judge them against the references.
- A test like `every_benchmark_character_is_sound` can hold every shipped preset sound, named after its file, and in the catalogue.
- **Three things the proposal must not forget** (found in verification). The `preset` block rides in `extra`, so opening a preset as a character and saving it would carry the block into the saved character: `apply_preset` copies the named parts only, never `extra`. Preset files reach an installed editor and a project only if `packaging/stage.py:60-78` and `project::add_character_packages` (`character_library.rs:117-154`) learn to copy them; both copy only what a base or item manifest names. And the stylized package is built into the program file by file (`character_library.rs:23-47`).

## Defects found

| # | Severity | Defect | Evidence |
|---|---|---|---|
| D1 | low | **"Unknown fields are kept" is true only for a description's top level.** `crates/avatar/CLAUDE.md` and the wiki say it without that limit. `architecture.md` section 4 is right for manifests ("manifests ignore them") but says nothing of worn items or the catalogue. Nothing is lost today: no file has such a field, manifests are copied as files, and only the catalogue is written from typed data. It becomes data loss the day a worn item gains a field | Only `description.rs:36-39` has `extra`; `manifest.rs:16-49`, `item.rs:18-57`, `catalogue.rs:16-53`, `description.rs:44-51` don't. `crates/avatar/CLAUDE.md` line 6; `wiki/Engine-Systems/Character-Data.md:60`; `docs/development/character-studio/architecture.md:52`. The only test is for the description (`formats.rs:60-69`). Copying: `crates/project/src/character_library.rs:112`, `:158` |
| D2 | medium | **A changed shipped manifest never reaches a folder that already has that ID.** `add_character_packages` skips every base and item the target library already lists, so new parameters or metadata in `Realistic.base.json` would not arrive in an existing workspace folder or project. Its own comment says a newer package replaces an older copy. Read from the code; not run | `crates/project/src/character_library.rs:118-120`, `:132-134`; the comment at `:88-90`; the test covers only the unchanged case (`:257-266`). Where it does copy, it compares sizes only (`:106`). The built-in stylized package has the same hole by another road: `add_character_library` writes each file only where none is there (`:73-83`; `crates/project/src/default_humanoid.rs:203-207`). The workspace calls both on every open (`character_workspace.rs:311-316`) |
| D3 | low | The same function does not copy a groom's `mesh` file, only `cards` and `strands`. Harmless today (the stylized hair's mesh is in the built-in model) | `character_library.rs:140-146`; `packaging/stage.py:73-77` leaves the mesh out the same way |
| D4 | low | **`follow` constraints are dead.** Defined, checked and documented as an editing rule for CO12, but nothing applies them, and neither manifest has any `constraints` (so `max_sum` is unused by content too) | `manifest.rs:247-254`, `:539-548`; no use of `Constraint` in `crates/editor` or `crates/gameplay`; neither manifest has the key |
| D5 | low | **Applying a preset resets the whole character.** Fine with six body presets; wrong once face and body presets exist | `character_workspace.rs:113-115`, `:561` |
| D6 | low | **Symmetry on hides and then overwrites an asymmetric pair.** The pair shows the first side's value only, and the first drag writes it to both sides | `character_workspace/ui.rs:445-452`, `:479`; `character_workspace.rs:477-490` |
| D8 | low | **The benchmark folder is stale against its generator.** Run today, `make.py` would add 15 files and rewrite all 214 realistic ones to list their underwear. Eight of the 15 are the `brow_length` cases: that pair has no benchmark at all, so 45 of the 47 face parameters are covered. No test compares the folder with the script; `every_benchmark_character_is_sound` passes because `resolve` wears the underwear anyway. Before running it, look at `make.py:97`: it takes the first item of each underwear slot, so the masculine files would list a sports bra. Checked by importing `make.py` and comparing in memory; nothing was written | `assets/characters/benchmarks/make.py:91-160`; `crates/avatar/tests/shipped_library.rs:229-268`; 337 files, none named `*brow-length*` |
| D7 | low | Labels and IDs that disagree with what they drive: `forehead_slope` is labelled "Forehead fullness"; `nostril_width_*` drive `Face_Nose_Width_*`; `temple_width` drives `Face_Head_Width`; `chin_length` is the brief's chin height | `faceparams.py:46`, `:71`, `:72`, `:64` |

## Shared contracts that would have to change (the lead owns them)

0. **One proposal, not two.** [audit-studio-ui.md](audit-studio-ui.md) section 4 names the same facts differently: `Parameter.description` (here `about`); handles inside each feature with `per_metre` (here a `handles` list with `travel`); an anchor that is a mesh vertex (here a bone and an offset); `MaterialSlot.editable`, `.category`, `.swatches` (here `values`, `label`); no icons in data (here `icon`). The lead picks one before any schema lands.
1. **`architecture.md` §4, `crates/avatar/CLAUDE.md` and the wiki page:** say that only a description's top level keeps unknown fields, worn items included (D1), and add "a new enum value is a breaking change" (section 4.6).
2. **`Parameter`** (`manifest.rs`): `about`, `feature`, `level`, `random`. **`Group`** and **`Slot`**: `about`, `icon`.
3. **`BaseManifest`**: new lists `features` and `handles`, with their checks and the model check for anchor bones.
4. **`ItemManifest`**: `about`, `parameters`, `also_fills`, `requires`, `excludes`, `limitations`; **`MaterialSlot`**: `label`, `values` (it is shared with the base manifest, so the base's skin and eye slots get them too); **`Variant`**: `thumbnail`, `parameters`.
5. **`WornItem`**: `parameters`, and `extra`. **`Description::validate`** and **`resolve`**: item parameters, `requires`, `excludes`, `also_fills`.
6. **`Catalogue`**: `presets`; **`Library`**: holds presets; `Catalogue::of` and `catalogue.py` change together.
7. **A preset contract** in `architecture.md`: the `preset` block, the kinds and `apply_preset`, in `avatar`.
8. **`Resolved` and the applier** (`gameplay::description`): item parameter shapes in an item's identity.
9. **Lua API** (architecture §17), if scripts should read item parameters or apply presets: a decision for the lead, not required by the brief.
10. **`project::add_character_packages`** and **`add_character_library`**: refresh a changed package (D2), or every field above is invisible to existing folders. Both must also copy preset files, variant pictures and a groom's mesh (D3).
11. **`packaging/stage.py`** (`:60-78`) and **`scripts/check-packages.py`**: stage and check the same new files, or an installed editor has none of them.
12. **`architecture.md:368`** (CO13: preset files are "descriptions without `name`") against `Description.name` being required (`description.rs:22`): settle with the preset contract (item 7).

## File ownership notes for parallel work

| Files | Owner today (plan §5) | Note |
|---|---|---|
| `crates/avatar/src/*.rs`, `architecture.md` | Lead | Every schema change above. One agent at a time: `manifest.rs`, `item.rs` and `description.rs` are each touched by several proposals. Land schema first, content after |
| `assets/characters/realistic/faceparams.py`, `face.py` | Face agent | New face shapes and their parameters, `about`, `level`, the face features and handles (anchors come from `face.py`'s landmarks) |
| `assets/characters/realistic/shapes.py`, `body.py`, `rig.py` | Body agent | New body shapes, body features and handles |
| `assets/characters/realistic/manifests.py` | Lead (the manifest's format) | It assembles groups, proportions, body and face. Features and handles from two agents meet here; give each module a `features()` and `handles()` function so the two never edit the same list |
| `assets/characters/humanoid/manifests.py` | Lead | The realistic generator imports `SLOTS`, `SOCKETS`, `PROPORTIONS`, `bone_drivers`, `objects`, `write_json` from it (`realistic/manifests.py:82-172`). A change there changes both families |
| `Realistic.base.json`, `Realistic.build.json`, `RealisticParts.glb`, `items/*.item.json`, `catalogue.json` | Generated | Never hand-edited and never merged by hand: regenerate after merging the scripts. Two branches that both rebuild the 36 MB model will conflict; only one branch at a time should run `build.py` |
| `assets/characters/benchmarks/*.character.json` | Generated by `make.py` | Regenerated whenever parameters change; expect large diffs (two files per face parameter per frame). Stale today (D8): the first run rewrites 214 files and adds 15 whatever else changed |
| `assets/characters/catalogue.py` | Lead | Gains the preset list |
| New `assets/characters/realistic/presets/*.preset.json` | New: assign one owner | Hand-written or script-written data; safe to work on in parallel with code once the contract exists |
| `crates/editor/src/character_workspace.rs` (`Preset`, `presets()`) | UI agent | The six constants become files; `open_preset(index)` and `Subject::Preset(index)` become IDs. Touches the start page and `thumbnails.rs` |
| `crates/project/src/character_library.rs` | Lead | D2, D3. Its `MANIFESTS` list is the stylized package file by file (`:23-48`); a preset file added to that package must be added there too |
| `crates/gameplay/src/description*.rs` | Lead with the runtime owner | Item parameters in the applier |
| `packaging/stage.py`, `scripts/check-packages.py` | Lead | Stage preset files and any new file a manifest names; today only manifest, model, groom cards, strands and thumbnail are staged (`stage.py:60-78`) |

## Risks

1. **Most missing face controls are geometry, not data.** 24 named controls need new identity shapes in the Blender build, and most of the 10 partial ones need one to move on their own, each checked against expressions, the lids, the lashes and brow cards, the teeth and the LODs. The data side of each is one line. Plan the face work by shape, not by slider.
2. **Handle anchors drift.** An anchor is a bone and an offset at the default face. After `nose_length` at +1 the real tip is 5 mm away from the anchor. Either the editor adds the handle's own drive to the anchor (it knows axis and travel), or anchors need a surface point. Decide before Stage 2 builds on it.
3. **Bone-space anchors assume the frame.** The feminine frame moves 71 bones (`frame`'s drivers, R:158). Offsets measured on the masculine frame may sit off the surface on the feminine one. The build should measure both and the check should fail when an anchor is more than a few millimetres off either.
4. **Item parameters cross four layers** (item manifest, description, resolve, applier and its derived models) and the shipped library has no item that would use them: the realistic body has no hair at all. Without content the capability system can't be tested end to end. Sequence it with the hair stage.
5. **Old editors and `items[].parameters`** (section 4.6). Check whether a released editor already saves format 2 before relying on worn-item fields; add `extra` to `WornItem` in the first schema change either way.
6. **Three detail levels need judgement, not a rule.** Today's `advanced` flags don't map onto the brief's levels: the regional body sliders are advanced but the brief calls them detailed; shoulder, waist and hip width are advanced but the brief wants them up front. Someone must assign `level` to all 75 parameters by hand.
7. **Stale copies of the library** (D2). Until it is fixed, a tester with an existing workspace folder may see none of the new metadata and report that nothing changed.
8. **Regrouping is safe, renaming is not.** `group`, `feature`, `level`, `label` and `about` can change freely: a saved character stores only parameter IDs and values. Renaming an ID (`chin_length` to `chin_height`) breaks saved characters, benchmarks, presets and scripts unless a migration maps it. Prefer fixing labels.
9. **Presets by index today.** Start-page cards, cached pictures and tests address presets by position (`shelf_tests.rs:254-262`, `tests.rs:532`). Moving to files changes their order and identity; the picture cache and those tests move with it.
10. **Generated-file churn.** Any parameter change rewrites `Realistic.base.json` (73 KB), many of the 337 benchmark files and the catalogue. Reviews should read the script diffs and let the tests vouch for the generated files.
11. **Handle data is in model space; the workspace is not.** Anchors and axes are metres with the front at +Z (`manifest.rs:70`). The workspace's character is 5 studs tall and faces −Z (`crates/editor/src/character_workspace/stage.rs:8-9`; `character_workspace/tests.rs:451-453`). The editor must take every anchor and axis through the character's transform; a test should pin one handle's screen direction.
12. **Two audits, two shapes for the same metadata** (contracts, item 0). If each stage builds on its own audit, the manifest gets both.
13. **A catalogue `presets` list is fragile in old hands.** An older editor that copies packages writes the catalogue again from its typed value and drops the list (section 4.6). A folder scan for `*.preset.json` beside the catalogue entry would survive that.
14. **The `preset` block can leak into saved characters** through `extra` (section 5). `apply_preset` and "open a preset" must build the character from named parts, and a test should say so.
15. **Benchmarks are not a safety net for brow length** until the folder is regenerated (D8).

## Verification

A second pass, 2026-11-16, at the same commit. Its job was to refute the audit. Every row marked missing was searched for under other names and in other crates. Every row marked implemented had its evidence opened. Every partial row had its evidence opened too. Every `path:line` in the table and the defects was opened. The brief's bullets were counted against the rows: §4.1 has 12 controls and 8 presets, §4.2 has 13 regions and 8 example controls, §5.1 has 9 presets, §5.2 has 63 controls, §5.4 has 6 actions, §14.1 has 5 example properties and 10 metadata bullets, §15 has 9 and 7. None was absent. Nothing was built or run but plain Python that reads files.

**What held.** All 75 and 31 parameters, their line numbers, ranges, groups and flags. The claim that every `Face_*` and body shape of the model is driven: the model's only undriven shapes are the six shoulder correctives, the expression shapes and `Pupil_Size`. The counts of items (15), variants (0), thumbnails (0), corners (55) and frame bones (71). Defects D2 to D7, read from the code as the first pass read them.

### Status changes

| Row | Was | Now | Why |
|---|---|---|---|
| CD-4.1-03 overall build | implemented | partial | The realistic body has no build control. `muscle` and `fat` were counted three times, for rows 03, 04 and 05 |
| CD-4.2-01 region selection data | missing | partial | The region map ships and says which region each triangle is; only the link from parameters to regions is absent. Data with no UI is partial, not missing |
| CD-4.2-24 no meaningless combinations | implemented | partial | The test proves each slider moves the body. It does not prove the controls suit each region: every build region gets the same muscle and fat pair |
| CD-5.2-09 eye corners | missing | partial | `eye_tilt_*` is written as "the outer corner up" (`face.py:1410`) |
| CD-5.2-34 cheek projection | missing | partial | Both cheek shapes move the cheek along its normal (`face.py:1468`, `:1470`) |
| CD-5.2-57 brow-to-forehead transition | missing | partial | `brow_ridge` is a band at the glabella (`face.py:1501-1504`); with `forehead_slope` it sets the step. The same reasoning made forehead width partial in the first pass |
| CD-14.1-08 fewer properties on another hairstyle | missing | partial | Two hairs declare one material slot and four declare two. That is the brief's own example, in data, with no UI |
| CD-14.1-10, -11, -12 parameters, ranges, defaults | missing | partial | Material values are the item-level settings that exist: named, ranged, with a neutral default, stored per worn item, checked and resolved. The first pass called hair's primary colour partial on this evidence and these three rows missing on the same evidence |

### Rows corrected without a status change

- **CD-14.1-04 highlight colour.** The gap said there is no highlight colour value. There is: `HairSecondary` is the highlights material (`assets/characters/humanoid/hair.py:4`), and its `tint` is the colour. What is absent is the control and a label that says so.
- **CD-5.2-54 forehead slope.** Kept as implemented. The gap now says what the shape is: a band of the mid forehead moved forward or back, not a turn about the brow.
- **CD-5.4-01.** A pair's shared label is made by string editing in the editor (`character_workspace.rs:207-222`), not by data.
- **CD-5.2-32, CD-5.2-52.** Kept as missing; the nearest controls and the side effects are named.

### Rows added

- **CD-15-19, CD-15-20.** Two general sentences of §15 that sit outside the two metadata lists but concern data: the studio stays a consumer, and every interaction updates the description. Both implemented, with a test or the code path cited.

### Defects

- **D1, lowered from medium to low, evidence corrected.** The first pass said `architecture.md:52` claims unknown fields are kept everywhere. It does not: it says "the description's `extra`; manifests ignore them". The loose wording is in `crates/avatar/CLAUDE.md` and the wiki. Nothing loses data today, because manifests are copied as files and no file has an unknown field. The worn-item gap is real and must close with the first worn-item field.
- **D2, widened.** The stylized package has the same hole: `add_character_library` writes a file only where none is there.
- **D3, widened.** `packaging/stage.py` leaves a groom's mesh out too.
- **D8, new.** The benchmark folder is stale against `make.py`; `brow_length` has no benchmark.

### Proposal and text

- **Section 4.6.** "Writes back the same bytes" was wrong: the tests compare values, and the Python generators and serde print numbers differently. Two cases added: an old engine rewriting a catalogue that lists presets, and installers not staging new files.
- **Section 5.** The plan and `architecture.md:368` already name preset files, as descriptions without `name`; `name` is required in code. Character packs are the nearest existing data. Three cautions added: `extra` leaking, packaging, the built-in file list.
- **Contracts.** Item 0 added: the UI audit proposes other names and shapes for the same metadata. Items 11 and 12 added: packaging, and the CO13 preset sketch.
- **Counts.** `crates/avatar` is 4,535 lines with tests, not 3,633 (the unit tests in `src/tests/` were left out). `realistic/manifests.py` uses the stylized generator from line 82 to line 172, not to 103.
- **Summary, risks and section 3.3** updated to match: 9 of 12 body controls exist; the face is 29, 10, 24; risks 11 to 15 are new.

### Notes from the first pass that were not in the file

- The brief (§2) says the facial identity controls are largely established. That is true for 29 of its 63 named face controls.
- The brief's "Advanced" toggle is `Parameter.advanced`, a flag in the manifest (`manifest.rs:118-120`), not only a UI switch. Three levels need a new optional field.
- `architecture.md` section 7 names `follow` constraints as a CO12 editing rule. Nothing implements them (D4).
- The brief's header points at a section 11 of the plan. At this commit the plan ends at section 10.

### Not checked

- Nothing was compiled or run, so D2 and D6 are read from the code, as the first pass said.
- Whether a released editor already saves format 2 (risk 5).
- The example numbers in the proposal's JSON (anchors, travel) are illustrations, not measurements.
