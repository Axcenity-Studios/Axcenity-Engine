# Stage 0 audit: gaps, contradictions and what nothing feeds yet

The completeness check of Stage 0. The [brief](brief.md) was walked bullet by bullet, sections 1 to 21, against the five audits ([studio UI](audit-studio-ui.md), [character data](audit-character-data.md), [asset pipeline](audit-asset-pipeline.md), [runtime and rendering](audit-runtime-render.md), [plan status](audit-plan-status.md)), the [QA baseline](baseline/README.md) and the reference indexes under [references/](../references/). Source was read at `99f5b181`. Nothing was built or run. Two small Python scripts read the two base manifests, the QA thresholds and the index tables.

## Summary

- **The audits cover the brief's feature sections well.** Sections 3, 4.1, 4.2, 5.1, 5.2, 5.4, 6, 7, 8.1, 8.3, 8.4, 9 to 12, 14 to 17 and the stage bullets of 18 each have a row per bullet.
- **No audit row covers these:** principles 1 to 7 and 9 to 11, section 2, 4.3, 4.4, 5.3, 8.2, 13.5, the opening rule of 17, the eleven stage acceptance lines of 18, and sections 19, 20 and 21. Section 2 below gives each one a status from the code: 73 rows.
- **Section 8.2 (the wardrobe browser) is the largest hole.** Nine bullets, no row citing it. Only "thumbnails, tags, search, filters" appears, as one CO12 row.
- **Sixteen status contradictions** between files. Most come from two status rules: the plan-status audit marks "the engine has it, the workspace does not" as missing, the other audits mark it partial. The real disagreements are about Save, the face controls, the Heavyset body and the Walk clip.
- **Five proposal contradictions the lead must settle before any schema lands:** the parameter's text field (two names), features and handles (three shapes), item capabilities (three shapes), decal placement (two), and where the randomizer lives (two).
- **The reference tree is complete as folders and as index fields, and thin as pictures.** No facial feature has front, side, three-quarter and close-up photographs. No body index row shows a three-quarter view, raised arms, bent elbows or bent knees in a photograph. The indexes say so themselves.
- **Stage 0 deliverables.** The audit and the reference index exist. The benchmark plan has a full proposal. The implementation breakdown and the updated visual QA requirements have inputs but three things nothing feeds: per-stage tests to write first, a measured cost of one edit, and a baseline of the stylized family on `main`.
- **One risk sits in a reference file only:** an active patent on editing a character by direct manipulation ([ux/patterns.md](../references/ux/patterns.md) section 8). No audit and no stage plan carries it.

## 1. Coverage by section

| Brief § | Bullets | Rows that cover it | Not covered |
|---|---|---|---|
| 1 | 11 principles | RR-P8-01 to -06 (principle 8) | 10 principles (section 2) |
| 2 | 8 systems, 5 rules | OPEN-22, OPEN-31, DIS-03 (families, default) | The eight systems, "format 2 stays", "no second system", "don't replace working systems" |
| 3.1 to 3.4 | 108 | UI-3.1-01 to UI-3.4-27 | None |
| 4.1, 4.2 | 46 | CD-4.1-01 to CD-4.2-24 | None |
| 4.3 | 5 | UI-3.4-06 to -08 (shoulder, waist, hip handles) | The height handle |
| 4.4 | 10 | OPEN-01, OPEN-03 touch it | All ten as bullets |
| 5.1, 5.2, 5.4 | 85 | CD-5.1-01 to CD-5.4-08 | None |
| 5.3 | 8 | UI-3.4-01 to -05 by content only | All as bullets |
| 6 | 31 | RR-6-00 to RR-6.3-07 | None |
| 7 | 31 | AP-7.1-01 to AP-7.3-08 | None |
| 8.1, 8.3, 8.4 | 28 | AP-8.1-01 to AP-8.4-08 | None |
| 8.2 | 9 | CO12-05, PLAN-CO12-03, UI-16-02 (one line of it) | All nine as bullets |
| 9 | 12 | AP-9-01 to AP-9-12 | None |
| 10 | 22 | UI-10-01 to -22, RR-10-01 to -22 | None |
| 11, 12 | 28 | UI-11-01 to UI-12-16 | None |
| 13.1 to 13.4 | sources, views, folders, fields | The indexes themselves; MAP-S0-06 to -08 | Section 5 below |
| 13.5 | 5 classes, 2 rules | MAP-S0-09, DIS-08 | The classes per shipped asset |
| 14, 14.1, 14.2 | 45 | AP-14-*, CD-14.1-*, AP-14.2-* | None |
| 15 | 31 | CD-15-*, UI-15-* | None |
| 16 | 12 | RR-16-01 to -12, UI-16-01 to -03 | None |
| 17 | 42 | AP-17.1-01 to AP-17.3-04 | The opening rule |
| 18 | 99 stage bullets, 11 acceptance lines, 5 deliverables | MAP-S0-01 to MAP-S11-11 | Acceptance lines, deliverables (section 6) |
| 19 | 6 agents, 5 steps | DIS-05 | The five steps |
| 20 | 8 steps, 12 things to test | OPEN-37 (one command) | All |
| 21 | 21 | DIS-03 (families) | 20, by pointer only |

## 2. Brief bullets with no audit row

Status rule as in the audits: **implemented** means a user can do it today and a test or a file proves it; **partial** means some of it; **missing** means nothing in the code; **unknown** means it is a rule for later work and nothing today can prove or break it.

Short names: WS `crates/editor/src/character_workspace.rs`; UI `crates/editor/src/character_workspace/ui.rs`; STG `crates/editor/src/character_workspace/stage.rs`; TST `crates/editor/src/character_workspace/tests.rs`; MAN `crates/avatar/src/manifest.rs`; ITEM `crates/avatar/src/item.rs`; DH `crates/project/src/default_humanoid.rs`; MEAS `assets/characters/humanoid/qa/measure.py`; THR `assets/characters/realistic/qa/thresholds.json`.

### Section 1, principles

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-1-01 | Character-first interaction | partial | The viewport takes camera input only (UI:255-275); every edit is a slider or chip in the right panel (UI:422-470, 511-564) | Selection and handles: UI-3.4 rows |
| GAP-1-02 | Progressive complexity | partial | One flag per parameter (MAN:118-120) and one switch (UI:431-432); 30 of the realistic body's 75 parameters are advanced | Three levels: UI-3.3-21 |
| GAP-1-03 | Immediate feedback, no manual apply | partial | No apply button; a drag re-applies, at most every 120 ms (WS:69, 671-697) | About eight updates a second while dragging (DIS-18) |
| GAP-1-04 | Anatomical consistency | partial | Values are clamped to their ranges (WS:758-780); geometry limits are checked offline (THR:4-37) | `follow` constraints are declared and never applied (MAN:249-254); nothing in the editor stops a pair of extremes that crosses |
| GAP-1-05 | Non-destructive: undo, redo, reset individual changes | partial | Undo and redo (WS:577-592); reset of the whole character (WS:596-604); a double click resets one slider (UI:486-488) | No reset by category, feature or side |
| GAP-1-06 | Visual discoverability | missing | A parameter has no description (MAN:110-139); hover text exists on toolbar buttons and the two switches only (UI:179-212, 430-432); hovering the character does nothing but zoom (UI:267-272) | Descriptions, tooltips, hover marks |
| GAP-1-07 | Scalable content | partial | Manifests, a catalogue and a library on disk (`crates/avatar/src/library.rs`); the panel is drawn from the manifest (UI:422-470) | No import flow (AP-CO9-06); items cannot declare their own controls (CD-14.1-10) |
| GAP-1-09 | Reference-driven development | partial | The reference tree exists as indexes with links (13 folders, 15 files) | No per-feature photographs (section 5); no shape has been made from the indexes yet |
| GAP-1-10 | Capability-aware customization | partial | Items are filtered by base (WS:492-500); an item lists its material slots (ITEM:19-57) | No control reads a capability (CD-14.1-09) |
| GAP-1-11 | Professional presentation | partial | A workspace of its own (`crates/editor/src/app/character_workspace.rs:25`) | Stock sliders, checkboxes and headers (UI:429-432, 482) |

### Section 2, constraints

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-2-01 | The eight named systems exist and are kept | implemented | `crates/avatar/src/`; WS and UI; `assets/characters/realistic/Realistic.base.json` (75 parameters); `assets/characters/humanoid/Humanoid.base.json` (31); `crates/anim/src/correctives.rs`, `twist.rs`, `face_play.rs`; `assets/characters/*/build.py` and `assets/characters/humanoid/qa/`; the applier (`crates/gameplay/src/description.rs:223`); the Lua methods (`crates/gameplay/src/description_system.rs:610-625`) | None |
| GAP-2-02 | Format 2 and manifest-driven parameters stay the foundation | implemented | `DESCRIPTION_VERSION` is 2 (`crates/avatar/src/description.rs:16`); a format 1 file opens and saves as format 2 (TST:240) | None |
| GAP-2-03 | No second competing description system | implemented | One description type. The older `Humanoid.customization.json` is still compiled into `project` (DH:46-47) but only a test reads it there (DH:741); otherwise it is an input of the stylized build (`assets/characters/humanoid/manifests.py:291`) | The comment at DH:45 says "Character Studio reads it". It does not. Remove the public constant or fix the comment |
| GAP-2-04 | Don't replace working geometry, fitting or runtime for a UI redesign | unknown | A rule for later stages. The written proposals add optional fields and new editor files only (audit-character-data section 4; audit-studio-ui section 3) | One proposal touches the runtime: a faster or partial bake (audit-studio-ui contract 11, DIS-18). It needs a measurement first |

### Section 4.3, direct body manipulation

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-4.3-01 | Dragging height handles adjusts overall height | missing | No handle of any kind (UI:255-275). The target exists: `height`, 0.85 to 1.15 (`Realistic.base.json:761`) | A handle; the Face and Full body views aim at constant heights (STG:66-75), so a height drag also needs the camera to follow |

The other three examples are UI-3.4-06 to -08. The closing rule is UI-3.4-19.

### Section 4.4, deformation quality

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-4.4-01 | No visible holes or intersections | partial | Boundary edges and non-manifold vertices are counted (MEAS:85-93); new crossings at slider ends are held at 0 (THR:20-25) | The fist crosses itself (OPEN-06); the keep-apart pass works in the rest pose only (`assets/characters/realistic/shapes.py:39-41`) |
| GAP-4.4-02 | No abrupt deformation transitions | partial | Judged by eye on the sheets only | No measure exists (searched smooth, transition, gradient in MEAS). The baseline lists hard transitions it saw (baseline H1, M9) |
| GAP-4.4-03 | No broken anatomy | partial | Proportion ratios are held in ranges (THR `within`, 21 keys) | No girths on the built body (body index, finding 7); the built foot and hand sit outside the population (body index, finding 6) |
| GAP-4.4-04 | No unexpected changes to unrelated regions | missing | A test proves every parameter moves the body (`crates/avatar/tests/realistic_parameters.rs:115`). None proves a parameter moves only its own region (searched locality, leak, unrelated in MEAS and the tests) | A locality check per parameter |
| GAP-4.4-05 | No excessive volume loss | partial | Ring area and local volume limits at elbow, knee and shoulder (THR:30-37; MEAS:702, 712) | The 90 degree shoulder misses its targets (THR:127-128; OPEN-01) |
| GAP-4.4-06 | No clothing penetration | partial | Stylized garments have poke-through limits (`assets/characters/humanoid/qa/thresholds.json`; MEAS:410) | The realistic underwear is not measured (asset pipeline defect 3); rest pose only |
| GAP-4.4-07 | No broken joint deformation | partial | Limits for elbow, knee, shoulder and three twists (THR:30-45) | Correctives for the shoulder only (OPEN-03); hip and wrist poses have no limit |
| GAP-4.4-08 | No eye, teeth or attached-part intersections | partial | Held at 0 at rest and at each slider end alone (THR:4-9, 22, 25) | `jaw_shift` leaves a lip vertex in the teeth; `eye_size` at -1 crosses (OPEN-18) |
| GAP-4.4-09 | Test extreme combinations, not single sliders | partial | Every pair of parameters at their ends (MEAS:1021) | Pairs only; not run since two shapes were added to the body and two to the brows (OPEN-36); 15 to 40 minutes a run |
| GAP-4.4-10 | Keep using the Blender and in-engine QA | implemented | Both ran for this stage's baseline (baseline sections 2 and 6) | No single command says pass (OPEN-37) |

### Section 5.3, direct facial manipulation

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-5.3-01 | An editing mode: select a region, then focus, highlight, handles, contextual parameters, immediate update | missing | Picking Face in the navigation frames the face (UI:241-253); nothing can be selected in the viewport (UI:255-275) | As UI-3.4-11 to -16 |
| GAP-5.3-02 | Nose example: handles for bridge width, tip position, overall proportions | missing | No handles. Bridge width has no parameter (CD-5.2-20); tip projection and rotation are one axis (`Realistic.base.json:1686`) | Two of the three handles need new shapes first |
| GAP-5.3-03 | Appropriate camera framing and sensitivity | partial | One Face view at a constant height and distance (STG:70-75) | No framing per feature; sensitivity has no data (UI-3.4-22) |
| GAP-5.3-04 | Sliders stay alongside direct manipulation | partial | The sliders exist and write the one description (WS:477-489) | Holds once handles call the same function |

### Section 8.2, wardrobe browser

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-8.2-01 | Visual thumbnails | missing | Items are text chips (UI:540-552). The field exists (ITEM:54); 0 of 15 items set it | Pictures, drawn by the existing painter or shipped |
| GAP-8.2-02 | Category filtering | partial | One section per slot of the category (UI:530-541) | No filter control; underwear slots are hidden (UI:516-528) |
| GAP-8.2-03 | Search | missing | Searched search, filter, query in UI and WS | A search field (the kit has one) |
| GAP-8.2-04 | Favorites | missing | Searched favo in UI and WS | A per-user store |
| GAP-8.2-05 | Recently used items | missing | `recent.rs` lists characters, not items (`crates/editor/src/character_workspace/recent.rs:1-4`) | A per-user list |
| GAP-8.2-06 | Outfit presets | missing | `Preset.items` exists and every preset leaves it empty (WS:145-190) | A preset scope (CD section 5) |
| GAP-8.2-07 | Color variations | missing | `Variant` exists in the schema (ITEM:107); 0 items have one; `wear` never sets one (WS:513-526) | Variants as data and a swatch row |
| GAP-8.2-08 | Compatibility information | partial | Items that do not fit the base are left out (WS:492-500); an empty page says "Nothing in the library fits" (UI:553-562) | Nothing says why an item is absent or what it was checked on |
| GAP-8.2-09 | Preview an item without committing | missing | A click wears the item as an undo step (UI:545-549; WS:513-526); hover does nothing | A hover or held preview outside the history (as UI-12-13) |

### Section 13.5, licensing

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-13.5-01 | Assets separated into the five classes | partial | Every reference row carries a class (all 15 index files). Every shipped item carries an SPDX licence and an author (ITEM:128-132; 15 of 15 say `CC0-1.0`) | A base manifest has no source field; no file lists shipped assets by class; class 3 is never to be in the repository (DIS-08) and nothing says so |
| GAP-13.5-02 | Record attribution and licence requirements | partial | `docs/development/character-studio/dependencies.md` (data and content table); `assets/characters/transfer/shapes/LICENCE-RECORD.md` | The CC BY figures and assets in the indexes (hair, clothing) have no attribution file to land in |
| GAP-13.5-03 | No reference mesh or texture ships without a verified permission | partial | Sources are pinned by URL and hash in `source/extract.py` | The CC0 statement of the vendored base meshes was not confirmed at its source in this stage (face index F-27; body index B09) |

### Sections 17 and 18

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-17-01 | A permanent benchmark collection, part of acceptance | partial | 337 description files (`assets/characters/benchmarks/`); "Accepted means" in `docs/development/character-studio/visual-qa.md:52` | The set is stale against its generator; no test compares them (asset pipeline defect 1) |
| GAP-18-S1 | Stage 1 acceptance: a new user creates and changes a character without knowing the parameters | partial | TST:148, 274 drive the model, not the pointer | No way to measure it is defined (DIS-17) |
| GAP-18-S2 | Stage 2: direct edits give the same result as the controls | missing | No handles | A test that a handle drag and a slider reach the same description |
| GAP-18-S3 | Stage 3: body adjusted without geometry defects | partial | 28 body parameters; pairs check (MEAS:1021) | GAP-4.4 rows |
| GAP-18-S4 | Stage 4: substantially different, believable faces by presets and controls | partial | 47 face parameters; no face preset (CD-5.1-01) | The baseline finds the two frames have nearly the same face (baseline M6) |
| GAP-18-S5 | Stage 5: distinct skin appearances | partial | `tint` and `roughness_scale` are drawn (`crates/scene/src/material.rs:153-162`); settable from a file or a script only | No control; five of seven skin values are not drawn (OPEN-24) |
| GAP-18-S6 | Stage 6: different hairstyles in one browser, no unsupported controls | partial | Six stylized styles as chips | None for the realistic body (OPEN-30) |
| GAP-18-S7 | Stage 7: complete outfits with reliable fit | partial | Stylized: three garments and shoes with limits | None for the realistic body |
| GAP-18-S8 | Stage 8: accessories equipped, adjusted, saved, reproduced | missing | No accessory item; a worn item holds no transform (`crates/avatar/src/description.rs:44-51`) | AP-9 rows |
| GAP-18-S9 | Stage 9: inspect under several expressions and animations | partial | The engine and the Game Editor's face tab; the workspace idles only (WS:732) | UI-10 rows |
| GAP-18-S10 | Stage 10: the workflow is polished and reliable | partial | Save, open, Recent, packs | Two data-loss defects (audit-studio-ui D-01, D-02) |
| GAP-18-S11 | Stage 11: ready for regular use | partial | Sum of the above | |

### Section 19, agents

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-19-01 | Six agents by area | partial | Six roles exist, with other borders (`docs/development/character-studio/agent-assignments.md:9-17`) | DIS-05: no rendering owner in the brief's list; the numbers clash |
| GAP-19-02 | Assign ownership of files and subsystems | partial | The same table; each audit ends with ownership notes | The new modules (audit-studio-ui section 3) and the shared QA folder have no owner yet |
| GAP-19-03 | Define interfaces and expected deliverables | partial | Hand-back list (`agent-assignments.md`, "Hand-back"); `architecture.md` section 16 | The workspace's internal contract (`Intent`, selection, edit) is a proposal in one audit |
| GAP-19-04 | Agree shared parameter and asset contracts | missing | Three competing shapes each for parameter metadata and item capabilities (section 3 below) | The lead picks one before parallel work |
| GAP-19-05 | Identify dependencies | partial | "Plan milestones the brief does not mention but depends on" (audit-plan-status section 4) | Not yet a stage order |
| GAP-19-06 | Separate branches | partial | Worktree rule (`agent-assignments.md`, "Working side by side") | Builds share one target directory (audit-studio-ui ownership notes); the realistic parts file is one 36 MB binary that cannot be merged |
| GAP-19-07 | The lead reviews and integrates | implemented | `agent-assignments.md`, "Review" | None |

### Section 20, tests

| ID | Bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| GAP-20-01 | The eight-step red and green workflow | partial | Rules in `docs/development/character-studio/testing.md:5-11`; review steps in `visual-qa.md:45-52` | "What each milestone must test" stops at CO13 (`testing.md:48-62`): no row for a brief stage |
| GAP-20-02 | Test default characters | implemented | `crates/avatar/tests/shipped_library.rs:193`; TST:22 | None |
| GAP-20-03 | Test extreme body shapes | partial | Benchmark files at the slider ends are held sound (`shipped_library.rs:229`); `realistic_parameters.rs:173` | Soundness of data, not of geometry; the geometry check is the Blender run |
| GAP-20-04 | Test extreme facial parameters | partial | 180 face identity end files; the pairs check | `brow_length` has no file (CD defect D8); none captured in the baseline |
| GAP-20-05 | Test symmetry and asymmetry | partial | TST:183 | No asymmetric character anywhere (AP-17.1-10) |
| GAP-20-06 | Test clothing combinations | partial | Stylized layering limits (`humanoid/qa/thresholds.json`, `layering`) | Nothing for the realistic body |
| GAP-20-07 | Test material variations | partial | `crates/player/src/default_humanoid_tests/description.rs:479` | No benchmark sets a material value (RR-6.1-13) |
| GAP-20-08 | Test expression combinations | partial | `crates/anim/tests/realistic_face.rs:77`; `face_checks.py` | Single controls and built-in expressions, not combinations with identity ends |
| GAP-20-09 | Test runtime reproduction | partial | TST:473 compares mesh IDs | Not material values or pictures (RR-P8-06) |
| GAP-20-10 | Test save and reopen | implemented | TST:274 | None |
| GAP-20-11 | Test comparison state isolation | missing | No comparison | With the feature |
| GAP-20-12 | Test direct manipulation consistency | missing | No handles | With the feature |
| GAP-20-13 | Test asset capability restrictions | partial | `crates/avatar/src/tests/resolving.rs:128`; `validation.rs:138` | Fit to base only; no capability to restrict yet |

### Section 21, definition of done

No row cites section 21. Each bullet is covered by rows of another section:

| Bullet | Status | Rows |
|---|---|---|
| Dedicated workspace | partial | UI-3.1, ACC-13 |
| Realistic and stylized families | partial | OPEN-31, DIS-03 |
| Body customization | partial | CD-4.1, CD-4.2 |
| Facial customization | partial | CD-5.2 |
| Direct viewport manipulation | missing | UI-3.4 |
| Skin and makeup | missing | RR-6 |
| A substantial hairstyle library | missing | AP-7.1 |
| A functional wardrobe | missing | AP-8, GAP-8.2 |
| Accessories | missing | AP-9 |
| Facial expressions; animation previews | partial | UI-10, RR-10 |
| Presets; selective randomization | partial; missing | UI-11 |
| Reliable undo and redo | partial | UI-15-01 |
| Comparison | missing | UI-12 |
| Capability-aware customization | partial | CD-14.1 |
| Consistent runtime appearance | partial | RR-P8 |
| Scalable asset import and authoring | partial | AP-CO9-06, AP-14 |
| Automated geometry and visual QA | partial | AP-17.3, ACC-16 |
| Standardized benchmark characters | partial | AP-17.1 |
| Clear user documentation | partial | DOC-01 to DOC-05 |

## 3. Contradictions

### Status

| # | One file says | Another says | Which is right |
|---|---|---|---|
| 1 | PLAN-CO12-04 colour and material pickers: missing | CO12-04 and UI-3.1-19: partial | A rule difference. No picker exists; the values and the kit's colour button do |
| 2 | PLAN-CO12-08 camera framings: implemented | CO12-09 and MAP-S1-05 (same file as the first): partial | Partial. Five constants, none per feature, no three-quarter by name (STG:57-80) |
| 3 | PLAN-CO12-10 preview clips and expressions: missing | CO12-11, RR-10-01 to -21, MAP-S9-04 (same file): partial | A rule difference. The workspace has no control; the engine has the clips and expressions |
| 4 | PLAN-CO12-12 Game Editor entry and round trip: missing | CO12-13 and MAP-S10-08 (same file): partial | Partial. A pack's Add to Project is half the trip (`crates/editor/src/character_workspace/packs.rs:289`) |
| 5 | PLAN-CO12-14 done-when (controls, walkthroughs, snapshots): implemented | CO12-17, -18, -20: partial | Partial. The tests call the model, not the pointer; snapshots are compared with nothing |
| 6 | PLAN-CO13-01 presets as data: missing | RR-CO13-01: partial | Missing for presets. The partial counts benchmark descriptions, which are not presets |
| 7 | PLAN-CO10-01, -CO11-01, -CO15-01, -CO18-01, -CO7-01: missing | AP-CO10-02, -06, -08, -11; AP-CO11-02, -05 to -09; AP-CO15-02; RR-CO18-04; RR-CO7-09, -10: partial | Both. The milestone has not started; older or stylized work covers pieces. The plan needs one wording |
| 8 | PLAN-CO13-02, RR-CO13-03, RR-P8-01 save and load: implemented, gap "None" | audit-studio-ui D-01, D-02, D-03: Save overwrites a same-named character, closing the window does not ask, a rename leaves a second file | The defects stand (WS:607-628). "None" is wrong |
| 9 | UI-3.2-04 whole-nose width: missing | CD-5.2-29: partial; face index: provided by the nostril pair | Partial. `nostril_width_*` move the wings only |
| 10 | UI-3.2-06 bridge height: partial | CD-5.2-21: implemented | Implemented. `nose_bridge` moves the bridge forward 4.6 mm; width is the separate, missing control |
| 11 | UI-3.2-09 tip rotation: missing; UI-3.2-21 eye corner position: missing | CD-5.2-25: partial; CD-5.2-09: partial | Partial, by the data audit's rule (an existing control moves it, not alone) |
| 12 | Face index: of 63 controls, 35 provided, 3 partly, 25 new | Data audit: 29, 10, 24 | Differences: philtrum length (provided by mouth height, or CD-5.2-52 missing), forehead width (temple width, or CD-5.2-55 partial), tip projection and rotation, whole-nose width. One tally is needed before Stage 4 is sized |
| 13 | UI-3.3-09 regional body adjustments: implemented | CD-4.2-15: partial; CD-4.2-05, -06: partial | Partial. Sixteen sliders in one advanced group; upper arm and forearm share one pair |
| 14 | CD-4.1-18 Heavyset preset and AP-17.1-06 heavyset benchmark: implemented | Baseline H6: "Heavy is not heavy"; body index findings 2 and 3 | The preset and the file exist; neither looks heavy. `fat` 1 alone gives a waist under the population mean |
| 15 | UI-10-17, RR-10-17, AP-17.2-17: a Walk clip exists (partial for lack of a control) | Baseline section 2: the Walk state plays the Run take | The baseline. `s("Walk", "Run", ...)` (DH:143), written as `Walk.anim.json` (DH:349-356). A Walk take is authored (`assets/characters/humanoid/animations.py:548`) and no state plays it |
| 16 | Runtime audit, Verification: principle 8 has 3 bullets | Its own table has six rows (RR-P8-01 to -06) | Six rows; the count is the brief's three clauses |

### Proposals

| # | Subject | Shapes on offer | To settle |
|---|---|---|---|
| P1 | Parameter text | `Parameter.description` (audit-studio-ui section 4) or `about` (audit-character-data 4.1) | One name |
| P2 | Features and handles | A `features` table with `pick`, `anchor` as a body-mesh vertex, `focus` and `handles` inside it (studio UI). Or features with `anchors` as bone and offset plus `radius`, and a separate `handles` table with `drives`, `axis`, `travel` (character data). Or a handle with a parameter per view for horizontal and vertical (ux/patterns.md gap 1) | The studio UI audit says bones cannot tell face features apart, and a bone offset does not follow an identity shape. A vertex anchor needs indices from the build |
| P3 | Item capabilities | `parameters` at the item's top, `materials[].values` as a list, `also_fills`, `requires` and `excludes` as item IDs, `limitations {note, parameter, range}` (character data 4.3). Or `fit.parameters`, `materials[].role` and `.editable` as a map with ranges, `occupies`, `excludes` as slots, `requires {items, features, hair}`, `limitations {code, message}`, `fit.validated`, `fit.adjust` (clothing index 6.2). Or `MaterialSlot.label`, `.category`, `.editable`, `.swatches` on the base (studio UI fact 5) | One schema. The worn item differs too: `parameters` against `fit` and `offset` |
| P4 | Decal placement | "A placement in UV space" (runtime audit, recommendation B). Or an anchor on the rest surface with an offset in metres, projected and baked (skin index section 3, which rejects plain UV stamps because they break at island edges) | One. The description field depends on it |
| P5 | The randomizer's home | `avatar`, shared with games (DIS-13) or `character_workspace/randomize.rs` (studio UI section 3) | One |

### Stale lines (written before another file existed)

| Where | Says | Now |
|---|---|---|
| MAP-S0-07 | `references/face/` has five folders, all empty | Six files, 63 index rows |
| MAP-S0-08 | No body index; `references/body/` does not exist | Four files, 52 rows |
| MAP-S0-06 | This stage adds `references/ux/` and `references/hair/` | Also face, body, clothing, skin, materials |
| MAP-S0-11 | No benchmark is audited against 17.1 | AP-17.1-01 to -11 do it |
| MAP-S0-09 | The five classes are not recorded per asset | Each item has an SPDX licence; each reference has a class. Missing: a class per shipped asset and a source on the base manifests |
| AP-17.3-02, OPEN-35 | The realistic baseline "is being captured" | Captured: 25 sheets in `baseline/engine/` |

## 4. Reference kinds the indexes do not hold

Folders: all 13 of brief 13.4 exist. Fields: every index table has the eight of brief 13.4. Every row is dated 2026-11-16. No row comes from Pinterest.

### Brief 13.2, per facial feature

| Feature | Front | Side | Three-quarter | Close-up | Different shapes | Levels of detail |
|---|---|---|---|---|---|---|
| Eyes, lids | E-09 (one photograph) | none | none | E-09 | Data only (E-01 to E-07) | none |
| Brows | none | none | none | none | Data from 20 women (F-12) | none |
| Nose profile, tip, nostrils | none | A 1918 plate (N-04) | none | none | Angles and breadths (N-01 to N-03); "no data found" for tip size, nostril height and nostril angle | none |
| Cheekbones | A skull plate (F-21) | A skull plate | none | none | Breadth data only | none |
| Jawline | none | A plate (J-04) | none | none | Angle and breadth data | none |
| Chin | none | none | none | none | Heights and a profile angle | none |
| Lips | A drawing (L-07) | none | none | none | Distances (L-01 to L-06) | none |
| Ears | none | A plate (EAR-07) | none | none | Sizes and lobe types | none |

- The only photograph sources are general sites (F-25, F-26, F-28), none opened, all class 4.
- The face index says this itself under "Cautions". It is the main thing still to collect for Stage 4.
- Brows, cheeks, chin and forehead have a section in the face index and no index rows of their own.

### Brief 13.3, body

| Kind | Held | Missing |
|---|---|---|
| Front, back | O04 (two ordinary adults, photograph), B15 (muscle chart) | More than one build |
| Side | O03 (a 1921 drawing), O05 (1887 sequence plates, licence unverified per file) | A modern photograph |
| Three-quarter | none | All |
| Arms raised | U05 (text, from search summaries) | Any picture |
| Arms lowered | none as a row | Any picture |
| Bent elbows, bent knees | U03, U04, L04, L05 (text and joint ranges) | Any picture |
| Walking, standing | O05, O06; O03, O04 | A modern stride from the front |
| Different proportions | O01, O02, O07, O10, O11 | Photographs |
| Muscle development | O09 (a ceiling as data) | Photographs of ordinary and athletic builds |
| Body fat distribution | B11, B12 (data) | Photographs |
| Shoulders, waist, hips, legs | Data and MakeHuman target lists | Photographs |

The upper and lower files each end with "Photo sets to look for", which is a list of what was not collected. The rule against bodybuilders as the only source is stated and kept.

### Brief 13.1, named sources with no usable row

| Area | Source | State |
|---|---|---|
| Hair | ArtStation | No row |
| Clothing | Fashion and clothing reference photography | No row |
| Clothing | Blender Studio, ArtStation | No breakdown indexed (the index says so) |
| Clothing | Marvelous Designer, CLO3D | Help pages refused; search summaries only (F-15, F-16) |
| Materials | Unreal Engine material documentation | Could not be read; none cited |
| Materials | Adobe Substance 3D documentation | One row; the page returned 403 |
| UX | Ready Player Me | Closed; cannot be studied first-hand |
| UX | The Sims 4 Create a Sim | No first-hand source for how the camera is kept apart from editing |

Not asked by brief 13 and not held: references for expressions and for accessories on their own (accessories are a part of the clothing index).

## 5. Claims without evidence

| # | Claim | Where | What is missing |
|---|---|---|---|
| 1 | An apply with the bake takes about 6 ms in a debug build | `progress.md:37`, the topic file; quoted by three audits and ux/patterns.md | Hardware, method, a repeatable bench. It predates the underwear and covers the body's bake only |
| 2 | The Face view cuts off short and tall characters (D-06); the panel clones the manifest per slider per frame (D-07) | audit-studio-ui | Read from the code. Not run, not measured |
| 3 | Baseline defects M5 (the Excited smile), L3 and H3 (the idle) and the pixel counts of H6 | baseline/README.md | The pictures were left in scratch, not committed. 17 of the 34 rest sheets were "captured and not reviewed" |
| 4 | The editor's character tests pass | baseline/README.md, row 2 | The command as asked did not build; a narrower one ran |
| 5 | Hair length holds about ±20%, volume about ±15 mm; the triangle budgets | hair index sections 4 to 6 | Estimates; the index says no reference gives them |
| 6 | "Sensible limits" per facial feature; "what ±1 moves today" | face index and its feature files | Read from constants in `face.py`; nothing was measured on the built head. Ranges for bridge profile and tip size are "by eye" |
| 7 | Girths of the built body at the slider ends; the heads-tall population range | body index sections 3 and 8 | Estimates |
| 8 | The vendored base meshes are CC0 | face F-27, body B09 | The repository's own record only; the source page refused the fetch |
| 9 | Palette colours, density figures, the "L* 78" limit; makeup roughness values; mole counts | skin index | "A simplified solve run this session", not reproduced; no source for the rest |
| 10 | Colour temperatures of the four preview lights; the 0.04 specular figure | materials index | Not from a source read |
| 11 | Eleven UX rows are search summaries (U-05, -06, -07, -18, -20, -21, -27, -30, -31, -34, -43) | ux index | Never opened |
| 12 | The direct-manipulation patent is active and close to brief 3.4 | ux/patterns.md section 8 | The claims were read through a summary; it needs a qualified read |
| 13 | The cut-out card shows the backdrop through skin (OPEN-19); garments lose their fit at a simpler level on the realistic body (OPEN-27) | audit-plan-status | Not re-run; carried from older notes and the baseline |
| 14 | The new capture views' camera positions; "about 2,600 pictures, 12 to 28 minutes"; whether one clip file can hold a pose for both skeletons | audit-asset-pipeline, benchmark plan | Proposals and an extrapolation; nothing tried |
| 15 | The example numbers in the proposed JSON (anchors, travel) | audit-character-data section 4 | Illustrations |
| 16 | A changed shipped manifest never reaches a folder that already has it (D2) | audit-character-data | Read from the code, not run |

## 6. Stage 0 deliverables: what feeds each

| Deliverable (brief 18) | Fed by | Nothing feeds |
|---|---|---|
| Audit | The five audits, this file | A current review of the stylized family: the newest is `audit.md` of 2026-11-14, before the overhaul (MAP-S0-03) |
| Reference index | 15 index files, 8 fields each | Section 4 above: photographs per feature and per pose |
| Implementation breakdown | Sizes on every row; the module split (audit-studio-ui section 3); contracts to change (every audit); the dependency table and MAP rows (audit-plan-status section 4); "the shortest honest path" and its order (audit-asset-pipeline) | (a) Tests to write first for each brief stage: `testing.md` stops at CO13. (b) How Stages 1, 2 and 10 are accepted when no character changes (DIS-17). (c) A measured cost of one edit in a release build, dressed and bare: the choice in DIS-18 and every handle design rest on it. (d) An outcome for the patent caution. (e) Decisions P1 to P5 |
| Updated visual QA requirements | Baseline sections 5 and 8; the capture additions (audit-asset-pipeline); DIS-10; asset pipeline defect 7 (`visual-qa.md` is out of date) | (a) What a reviewer checks in the workspace itself: its snapshots are compared with nothing, none is committed, and two print a local folder path. (b) A check of the studio's picture against the game's (RR-P8-06) has no proposal. (c) No measure for abrupt transitions or for a parameter leaking into other regions (GAP-4.4-02, -04) |
| Benchmark plan | The proposal in audit-asset-pipeline (cast, suites, capture and Blender additions) | (a) The plan's walk phases use `Walk@0.25` to `@0.75`: the number is seconds (`crates/player/src/character_capture.rs:588`) and the clip is the Run take (contradiction 15). (b) The stylized family has no baseline on `main`: its sheets were captured on a branch (PLAN-CO2-01), and the asymmetric cast member cannot be expressed on it. (c) Performance is not in it: no representative characters or budgets for brief 16 (RR-16-11, -12). (d) The Blender side of the baseline is numbers only: the sheets, `face_checks.py`, `posed_crossings.py` and the pairs pass were not run |

### Found in one file and carried by no audit

- **The patent** (ux/patterns.md section 8). `dependencies.md` lists three patent areas to check; this is not one of them.
- **Measured oddities of the built body** (body index, finding 6): the masculine foot is at the 2nd percentile for length; the feminine hand is longer than the masculine one.
- **No girths in the package measure** (body index, finding 7), so presets cannot be held in centimetres.
- **The Walk state plays the Run take** (baseline section 2).
- **The capture's cameras miss the head of the tallest body and the face of the shortest** (baseline section 5). The workspace's Face view has the same constant (audit-studio-ui D-06).

## 7. Method and limits

- Each audit's requirement table was read as a list of ID, requirement, section and status; the brief was then walked bullet by bullet against it. Rows were opened in full where a status was compared.
- Every status in section 2 rests on a line opened for this file, except where it names another audit's row.
- The reference indexes were read for their tables, their "not verified" sections and their pose and view sections. Their links (about 360 rows) were not opened.
- Not checked: anything that needs a build or a run; the wiki pages; the plan beyond its headings and what the plan-status audit quotes.
