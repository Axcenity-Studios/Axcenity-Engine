# Refinement Stage 0: plan status and open items

Audit of [the plan](../../../../plans/axcenity-engine-character-studio.md) (milestones CO1 to CO18, acceptance section 9) against the code on `main` at `99f5b181` (2026-11-16), every open item carried forward, and [the brief](brief.md)'s Stages 0 to 11 mapped onto the plan. Source and tests were read; nothing was built or run.

## Summary

- **Done and on main:** CO1 to CO6 (Stages 2 and 3, PR #412), CO12's first part (#413), the lowered-shoulder correctives (#414), Male or Female with underwear and brow length (#415, #416), the start page with Recent, pictures and packs (#418). #417 is terrain work, not character work. #419 to #423, also on main at this commit, hold no character work.
- **Not started:** CO7 to CO11 and CO14 to CO18. Only schemas exist for binding, grooms and the shading models' values; the runtime resolves variants but no item has one. The realistic body has three items, all underwear.
- **CO12:** three of its thirteen listed sub-items are done, four are partial and six are missing. CO13 is partly delivered by packs.
- **The brief** names only CO12. Of its 99 stage bullets and the Stage 2 priority list, 1 implemented, 64 partial, 35 missing. Of the 88 rows in Stages 1 to 11, 28 are new work with no milestone (viewport interaction, comparison, randomization, locking, favorites, per-item parameters, decals, accessory positioning, tooltips, three customization levels).
- **The brief depends on milestones it never mentions:** CO7 (Stages 5 and 6), CO9 and CO10 (Stages 7 and 8), CO11 (Stage 6), CO13 (Stages 8 and 10), CO17 (Stage 5), CO8 and CO18 (Stage 11 and §16).
- **44 open items** are carried forward (42 with nothing fixed, 2 part fixed); twelve older ones are fixed and listed so nobody carries them again.
- **The brief says the plan's section 11 holds this mapping. The plan ends at section 10.**

Status words: **implemented** means a user can do it today and a test or a file proves it; **partial** means some of it; **missing** means nothing in the code (searched under at least three names). For an open item, missing means nothing of it is fixed. Sizes: S under a day, M days, L a week or more, XL a milestone.

## 1. Plan milestones CO1 to CO18

11 implemented, 10 partial, 17 missing. CO12 and CO13 are split into their sub-items.

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| PLAN-CO1-01 | CO1: contract documents, ADR-0032 to 0037, the `avatar` crate (description, manifests, validation, format 1 migration) | plan §4 Stage 2 | implemented | crates/avatar/tests/shipped_library.rs:158; crates/avatar/src/tests/migration.rs:17; PR #406 | The plan's evidence names `a_saved_look_migrates_to_the_same_parts_and_shapes`, which no longer exists (CS2's `Look` went with CO12). Its successors are migration.rs:17 and crates/editor/src/character_workspace/tests.rs:240. Fix the plan's text. | S |
| PLAN-CO2-01 | CO2: in-engine capture at fixed cameras and lights from a description file; Blender harness; geometry checks; same input gives identical images; baseline on merged main | plan §4 Stage 3 | partial | crates/player/src/character_capture.rs:707; assets/characters/benchmarks/ (214 realistic, 123 stylized files); docs/development/character-studio/baseline/engine/ | The only in-engine baseline is the stylized one, captured on CO2's branch, not on main. No baseline set of the realistic body as it is today. The Blender harness still doesn't read description files (no reader in assets/characters/humanoid/qa/*.py). `--check` exits 1 in every recorded run (OPEN-37). | M |
| PLAN-CO3-01 | CO3: `HumanoidDescription`, `ApplyDescription`, Lua; resolve; derived models; material values that modulate | plan §4 Stage 3 | implemented | crates/player/src/default_humanoid_tests/description.rs:182, :287, :402, :575; crates/editor/src/character_workspace/tests.rs:473; crates/gameplay/src/lib.rs:68; PR #412 | The agreement test was renamed and now compares MeshIds only (crates/editor/src/character_workspace/tests.rs:499), not corners to 0.2 mm as the plan says. ADR-0038 is still Proposed. `modulate` draws three values only (crates/scene/src/material.rs:153-162). | S |
| PLAN-CO4-01 | CO4: realistic base body, skeleton with twist and helper bones, UVs, skin maps, LODs; geometry QA passes | plan §4 Stage 3 | implemented | assets/characters/realistic/Realistic.base.json; crates/anim/src/twist.rs:197; assets/characters/realistic/qa/thresholds.json:125; docs/development/character-studio/stage3/measure-check.txt:55 | The 90 degree shoulder targets print as not met (0.847 / 0.88 against 0.853 / 0.992). The fist crosses itself. co4/ has sheets and numbers but no README. | M |
| PLAN-CO5-01 | CO5: frame, muscle and fat (whole and eight regions), proportions, corrective shapes, manifest entries | plan §4 Stage 3 | implemented | crates/avatar/tests/realistic_parameters.rs:115, :173; crates/anim/src/correctives.rs:40, :227, :272; docs/development/character-studio/stage3/sliders-check.txt | Proportions run 0.85 to 1.15 and hips only widen (assets/characters/realistic/Realistic.base.json:863). Correctives cover the shoulder only (six rules). The rules are constants in `anim`, not manifest data. | L |
| PLAN-CO6-01 | CO6: realistic head, eyes, lash and brow cards, teeth and mouth, 44 expression shapes, ARKit mapping, identity regions, the face profile | plan §4 Stage 3 | implemented | crates/anim/tests/realistic_face.rs:63, :77, :170; crates/anim/src/face_tests.rs:409; assets/characters/realistic/Realistic.base.json (47 face parameters in 9 groups) | Done-when is met. The still-short list stands (docs/development/character-studio/co6/README.md:183-216): OPEN-11 to OPEN-20. Pupil size, listed in CO6's content, is a `Pupil_Size` shape on `Eyes` that nothing drives (assets/characters/realistic/eyes.py:7): no parameter and no face control. | L |
| PLAN-CO7-01 | CO7: packed ORM; Skin, Eye, Cloth, Hair shading models; glTF material extensions; MikkTSpace tangents; hashed alpha under TAA; cut-out-aware shadows | plan §4 Stage 4 | missing | crates/render/src/draw_list.rs:2109-2117 (Standard, Unlit, Toon only); crates/avatar/src/material.rs:19 (contract only) | Not started. Searched render, scene and assets for sheen, subsurface, anisotrop, charlie, mikk, KHR_materials: no shading code. The brief never names it; brief Stages 5 and 6 need it. Older work covers part of two sub-items: ray traced shadows respect cut-outs (crates/render/src/ray_shadows.wgsl:49-64) while shadow maps don't, and per-pass GPU timers exist (crates/render/src/renderer/gpu_timer.rs:19). See audit-runtime-render.md, rows RR-CO7. | XL |
| PLAN-CO8-01 | CO8: morph and skin once per frame in compute, used by every pass and by ray tracing | plan §4 Stage 4 | missing | crates/render/src/ray_skin.wgsl (ray tracing path only) | Not started. Searched for posed buffer, compute morph, pose_once: nothing. Brief §16 needs its result once many shapes are live. | L |
| PLAN-CO9-01 | CO9: item manifest and catalogue; binding and fitting; layer push-out; region hiding; material variants; the import flow | plan §4 Stage 4 | partial | crates/avatar/src/item.rs:19-57, :91 (`Fit::Binding` schema), :107 (`Variant`); crates/avatar/src/catalogue.rs:17; crates/avatar/src/resolve.rs:279-284, :304; crates/gameplay/src/description.rs:368 | More than schemas: a worn item's variant and its own material values are resolved (test crates/avatar/src/tests/resolving.rs:252) and `SetItem` takes a variant; regions hide skin through mask shapes (the stylized garments, the realistic underwear); a socket item gets its Attachment. Not built: binding, fitting, push-out, hiding by the region map (`map_index` is read by nothing), the import flow (searched import_item, transfer_weights, classify_slot, surface_bind, push_out). No shipped item has a variant, a `binding` fit or a `socket` fit. | XL |
| PLAN-CO10-01 | CO10: the high-to-low garment pipeline and about 14 items | plan §4 Stage 4 | missing | assets/characters/realistic/items/ (three underwear items only) | No garment pipeline and no clothes for the realistic body. The stylized package keeps its 12 old items. | XL |
| PLAN-CO11-01 | CO11: groom file and converter; card generation; spring bones; facial hair; about 8 styles; hair colour | plan §4 Stage 4 | missing | crates/avatar/src/item.rs:118 (`Groom` schema only) | No groom reader, converter or runtime; no spring bones (searched spring_bone, SpringBone, groom in anim and render). The realistic body has no hair item. | XL |
| PLAN-CO12-01 | CO12: panels drawn from the manifest, basic and advanced | plan §4 Stage 4 | implemented | crates/editor/src/character_workspace/ui.rs:422-470; crates/editor/src/character_workspace/tests.rs:402; PR #413 | None for what is built. | S |
| PLAN-CO12-02 | CO12: nine categories (Presets, Body, Face, Skin and Makeup, Hair, Clothing, Accessories, Expressions and Animation, Stage) | plan §4 Stage 4 | partial | crates/editor/src/character_workspace/ui.rs:28-33; crates/editor/src/character_workspace.rs:226-233 | Six of nine. Skin and Makeup, Accessories, and Expressions and Animation don't exist. The manifest contract already has `skin` and `accessories` categories (crates/avatar/src/manifest.rs:97-104); the workspace shows four of its six (crates/editor/src/character_workspace.rs:247-256). | L |
| PLAN-CO12-03 | CO12: wardrobe browser with thumbnails, tags, search and filters | plan §4 Stage 4 | partial | crates/editor/src/character_workspace/ui.rs:511-563 (a text chip per item); crates/editor/src/character_workspace/tests.rs:274 | Hair and Clothing list each slot's items as chips by label, and wearing one is tested. No thumbnails, tags, search or filters. The item schema already has `tags` and `thumbnail` (crates/avatar/src/item.rs:51, :54). On the realistic body the pages have nothing to list. | L |
| PLAN-CO12-04 | CO12: colour and material pickers | plan §4 Stage 4 | missing | crates/editor/src/character_workspace/ui.rs (no picker; searched color_palette, edit_srgb, picker) | The description holds `materials` values (crates/avatar/src/description.rs:35) but no page edits them. | M |
| PLAN-CO12-05 | CO12: presets and outfit presets | plan §4 Stage 4 | partial | crates/editor/src/character_workspace.rs:145-190; crates/editor/src/character_workspace/ui.rs:350-386 | Six body presets as Rust constants. No outfit presets (searched outfit: 0 hits). | M |
| PLAN-CO12-06 | CO12: numeric entry | plan §4 Stage 4 | partial | crates/editor/src/character_workspace/ui.rs:481-485 | The slider is egui's, whose value box takes typed numbers by default. Nothing tests it and there is no dedicated field. | S |
| PLAN-CO12-07 | CO12: symmetry | plan §4 Stage 4 | implemented | crates/editor/src/character_workspace/ui.rs:429, :446-452; crates/editor/src/character_workspace/tests.rs:183 | A toggle only: no mirror, copy side or reset side (brief §5.4). | S |
| PLAN-CO12-08 | CO12: camera framings | plan §4 Stage 4 | implemented | crates/editor/src/character_workspace/stage.rs:39-53; crates/editor/src/character_workspace/tests.rs:448 | Full body, Face, Front, Side, Back. No three-quarter preset and no per-feature focus. | S |
| PLAN-CO12-09 | CO12: turntable | plan §4 Stage 4 | missing | crates/editor/src/character_workspace/ui.rs:255-275 | Searched turntable, spin, auto-rotate: nothing. | S |
| PLAN-CO12-10 | CO12: preview clips and expressions | plan §4 Stage 4 | missing | crates/editor/src/character_workspace.rs:732 (the preview only idles) | No clip or expression choice in the workspace (searched expression: 0 hits there). | M |
| PLAN-CO12-11 | CO12: click on the body to open its category | plan §4 Stage 4 | missing | crates/editor/src/character_workspace/ui.rs:256-275 | The viewport takes clicks and drags for the camera only. No picking code in the workspace. | M |
| PLAN-CO12-12 | CO12 and D12: the Game Editor entry and round trip | plan §4 Stage 4 | missing | crates/editor/src/app/chrome/menus.rs:135 | No menu item and no "Edit in Character Studio" on a selected character (searched). The workspace opens from the start screen or the flag only. | M |
| PLAN-CO12-13 | CO12 and D12: the face tab renamed so only the workspace is called Character Studio | plan §4 Stage 4 | missing | crates/editor/src/layout.rs:205 | The Game Editor's face tab is still titled "Character Studio". | S |
| PLAN-CO12-14 | CO12 done-when: every control changes the character; walkthrough tests; drawing snapshots | plan §4 Stage 4 | implemented | crates/editor/src/character_workspace/tests.rs:148, :274, :402; crates/editor/src/app/character_workspace.rs:143 | Holds for what is built only. | S |
| PLAN-CO12-15 | Delivered outside the plan's CO12 list: the start page with Recent, pictures and packs (#418); Male or Female as a choice, underwear always on (#415, #416) | plan §4 Stage 4 | implemented | crates/editor/src/character_workspace/shelf_tests.rs; crates/editor/src/start/characters/tests.rs; crates/editor/src/character_workspace/tests.rs:55 | The plan and progress.md have no entry for #416 or #418. | S |
| PLAN-CO13-01 | CO13: character presets and outfit presets as data | plan §4 Stage 5 | missing | crates/editor/src/character_workspace.rs:145 (Rust constants) | architecture.md §18 plans preset files (descriptions without `name`); none exist. | M |
| PLAN-CO13-02 | CO13: save and load | plan §4 Stage 5 | implemented | crates/editor/src/character_workspace.rs:376, :618; crates/editor/src/character_workspace/tests.rs:274 | None. | S |
| PLAN-CO13-03 | CO13: placing a character in a project, copying its items | plan §4 Stage 5 | partial | crates/editor/src/character_workspace/packs.rs:289-300; crates/gameplay/src/description_system.rs:570; crates/player/src/default_humanoid_tests/description_lua.rs:217 | A pack's Add to Project writes descriptions and whole packages (not only the items used). A script can make and place a described character (`Players:CreateHumanoidModelFromDescription`, tested). Nothing in the editor places a character file in a scene, and a script can't load a description by path. | M |
| PLAN-CO13-04 | CO13: several characters in one scene | plan §4 Stage 5 | partial | wiki/Architecture/Architecture-Decisions/ADR-0038-Identity-Is-Baked-Into-Derived-Models-Named-By-MeshId.md:47; crates/player/src/default_humanoid_tests/description_lua.rs:217 | Derived models are shared by key, by design. The nearest test has two characters in one scene, one of them described. None has several described characters. | M |
| PLAN-CO13-05 | CO13: the description replicated over the network | plan §4 Stage 5 | partial | crates/player/src/default_humanoid_tests/description.rs:575 | The kept `HumanoidDescription` is an Instance and is applied again after a load. No network test (searched crates/net and crates/server: no mention). | M |
| PLAN-CO13-06 | CO13: export of a character as a package | plan §4 Stage 5 | partial | crates/editor/src/character_workspace/packs.rs:179, :245 | Pack export is descriptions only, with no assets. Enough while every item is in the shipped library. | M |
| PLAN-CO13-07 | CO13 done-when: changing one subsystem breaks none of the others, checked by the QA harness | plan §4 Stage 5 | missing | docs/development/character-studio/visual-qa.md | No cross-subsystem run exists; there is nothing beyond body and face to cross yet. | M |
| PLAN-CO14-01 | CO14: strand hair tier | plan §4 Stage 6 | missing | crates/render/src (searched strand: 0 hits) | Not started. The brief has no stage for it; only "strand density" in §7.2 hints at it. | XL |
| PLAN-CO15-01 | CO15: cloth simulation on coarse proxies; wind | plan §4 Stage 6 | missing | crates/physics/src/volume_tests.rs (soft bodies exist in the vendored physics only) | Not started. Not in the brief. | XL |
| PLAN-CO16-01 | CO16: wardrobe wave 2 | plan §4 Stage 6 | missing | assets/characters/catalogue.json | Not started. Brief §8.1 and §9 name its categories (sweaters, coats, hats, belts, bags, jewellery). | XL |
| PLAN-CO17-01 | CO17: screen-space SSS; freckles, moles, scars, wrinkles; makeup layers; wrinkle maps | plan §4 Stage 6 | missing | crates/avatar/src/material.rs:137-178 (skin values exist as contract; no makeup values) | Not started. Brief Stage 5 is this milestone plus tone, undertone and a decal architecture. | XL |
| PLAN-CO18-01 | CO18: budgets measured; merging a finished character's parts; LOD sets switched together; crowd test | plan §4 Stage 6 | missing | crates/assets/src/model/lod.rs:45-54 (authored `_LOD1` and `_LOD2` copies are read) | Nothing measured. Parts still pick their levels one by one (OPEN-27). #421's Group as Mesh merges static parts in the editor and groups a rigged model as it stands at rest (crates/mesh_edit/src/group.rs:264); it is not character merging. | L |

## 2. Plan section 9, acceptance criteria

3 implemented, 8 partial, 5 missing.

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| ACC-01 | Sound body geometry | plan §9 #1 | partial | assets/characters/realistic/qa/thresholds.json; docs/development/character-studio/stage3/measure-check.txt:55-57 | Every limit holds; two shoulder targets not met; fist crossings. True of the realistic body only. | M |
| ACC-02 | Corrected face | plan §9 #2 | partial | crates/anim/tests/realistic_face.rs:63; docs/development/character-studio/co6/README.md:183 | The checks are clean. The still-short list is long and each item is recorded as a defect, not accepted. | L |
| ACC-03 | Masculine and feminine | plan §9 #3 | implemented | crates/editor/src/character_workspace/tests.rs:55; crates/player/src/default_humanoid_tests/description.rs:287 | None. It is a choice of two, not a blend, since #415. | S |
| ACC-04 | Customization moves geometry | plan §9 #4 | implemented | crates/avatar/tests/realistic_parameters.rs:115 | 75 parameters, each moving the baked body. | S |
| ACC-05 | Convincing hair and movement | plan §9 #5 | missing | assets/characters/catalogue.json (no realistic hair item) | Nothing for the realistic body. The stylized hair is audit defect 4. | XL |
| ACC-06 | A varied wardrobe | plan §9 #6 | missing | assets/characters/realistic/items/ | Underwear only. | XL |
| ACC-07 | Fabric materials | plan §9 #7 | missing | crates/render/src/draw_list.rs:2109 | No Cloth shading model. | XL |
| ACC-08 | Clothing fits every body | plan §9 #8 | partial | crates/avatar/tests/realistic_parameters.rs:289 | True of the underwear, by copied shapes (an interim exception to ADR-0035). No binding. | XL |
| ACC-09 | Better footwear and accessories | plan §9 #9 | missing | assets/characters/realistic/Realistic.base.json (seven sockets, no socket item) | Nothing shipped. | XL |
| ACC-10 | Facial expressions | plan §9 #10 | implemented | crates/anim/tests/realistic_face.rs:77; crates/anim/src/face_play.rs:42 | In the runtime and the Game Editor's face tab. Not shown in the workspace. | M |
| ACC-11 | Presets saved and loaded | plan §9 #11 | partial | crates/editor/src/character_workspace.rs:145, :618 | Characters save and load; presets are built in and can't be saved by a user. | M |
| ACC-12 | Works with the animation system | plan §9 #12 | partial | crates/anim/src/retarget_tests.rs:1064; crates/editor/src/character_workspace.rs:732 | The starter clips play on the realistic body. The workspace shows the idle only. | M |
| ACC-13 | A dedicated, functional Character Studio | plan §9 #13 | partial | crates/editor/src/character_workspace/ui.rs:64-105; crates/editor/src/start/characters.rs | CO12 part 1 and the start page. See PLAN-CO12-02 to -13. | XL |
| ACC-14 | Pipeline open to expansion | plan §9 #14 | partial | crates/avatar/src/library.rs; crates/project/src/character_library.rs:98 | Manifests, a catalogue and packages on disk. No import flow for artist-made items. | L |
| ACC-15 | Acceptable, scalable performance | plan §9 #15 | missing | docs/development/character-studio/progress.md:37 | One figure exists (about 6 ms to apply, debug build, before the underwear). No budget measured. | L |
| ACC-16 | Automated and visual tests | plan §9 #16 | partial | crates/player/src/character_capture.rs:707; assets/characters/humanoid/qa/measure.py:1204 | The harness exists and is used. No picture is compared automatically, and the Blender check never exits 0 in one run. | M |

## 3. Open items carried forward

Every "still short", "open", "left", "not met", "defect" and "limit" in `progress.md`, `audit.md`, the stage folders and the wiki that is not fixed on main. The Brief § column says where it belongs in the brief's stages.

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| OPEN-01 | The shoulder at 90 degrees misses its targets (0.847 / 0.88 against 0.853 / 0.992) | §18 Stage 3 (§4.4) | missing | docs/development/character-studio/stage3/measure-check.txt:55-56; assets/characters/realistic/qa/thresholds.json:125 | A collarbone that lifts with the arm was left for the second wave (wiki Known-Limitations.md:60). | M |
| OPEN-02 | Proportion sliders stop at 0.85 to 1.15 and the hips only widen; the trade was left for the project owner to confirm | §18 Stage 2 and 3 (§4.1, §4.3) | missing | assets/characters/realistic/Realistic.base.json:863; docs/development/character-studio/progress.md:82 | No confirmation is recorded. Waist and hip handles (brief Stage 2 priorities 6 and 7) inherit the short range. | L |
| OPEN-03 | Corrective shapes exist for the shoulder only (raised and lowered); none for elbow, knee, hip, the arm raised forward or a turned collarbone | §18 Stage 3 (§4.4) | missing | crates/anim/src/correctives.rs:40; docs/development/character-studio/co5/README.md:87 | Brief §4.4 asks for no broken joint deformation. | L |
| OPEN-04 | Corrective rules are constants in `anim`, with no manifest field | §18 Stage 3 (§14.1, §15) | missing | crates/anim/src/correctives.rs:40; crates/avatar/src/manifest.rs:74-77 | A second body family can't declare its own correctives. A contract change. | M |
| OPEN-05 | Twist and helper bones are driven by name; no manifest field for the driver rule | §18 Stage 3 (§15) | missing | crates/anim/src/twist.rs:197; wiki/Roadmap/Known-Limitations.md:63 | The lookup by name each pose was to be measured in "CO19", a milestone the plan doesn't have (docs/development/character-studio/progress.md:101). | M |
| OPEN-06 | The fist crosses itself (247 / 199 triangle pairs; 240 to 306 in Cheer) | §18 Stage 3 and 9 | missing | wiki/Roadmap/Known-Limitations.md:61; docs/development/character-studio/co5/README.md:89 | Hand weights from CO4. Shows as soon as gestures are previewed (brief §10). | M |
| OPEN-07 | One normal map, baked from the athletic sculpt, is on every build, so soft bodies show abdominal lines | §18 Stage 5 (§6.1) | missing | docs/development/character-studio/co5/README.md:84 | Needs normal-map layers the muscle and fat sliders blend. Waits for the renderer. | L |
| OPEN-08 | Body shapes still short: heavy and muscular reads as a bodybuilder; the face barely changes with fat; the slight masculine body has a small belly; limb correspondence is 1 to 3 cm loose | §18 Stage 3 (§4.1, §13.3) | missing | docs/development/character-studio/co5/README.md:85-86 | Brief §13.3 warns against bodybuilding references as the only source. | L |
| OPEN-09 | A hard dark band across the upper chest under studio light, and a dark spot or rim on one shoulder: unexplained | §18 Stage 5 | missing | docs/development/character-studio/progress.md:32, :87 | Assigned to CO7. Not investigated since. | M |
| OPEN-10 | The fold under the feminine breast was softened, but the normal map there was baked for the sharper fold | §18 Stage 5 | missing | docs/development/character-studio/co5/README.md:88 | Rebake with the skin maps. | S |
| OPEN-11 | No creases anywhere on the face (no wrinkle maps) | §18 Stage 5 and 9 | missing | docs/development/character-studio/co6/README.md:187 | Plan CO17. | L |
| OPEN-12 | Eyes: the shade is a film that doesn't know where the light is; the iris edge is 24 segments; no caruncle | §18 Stage 5 (§5.2 iris and pupil appearance) | missing | docs/development/character-studio/co6/README.md:188-192 | Eye shading is CO7's. | L |
| OPEN-13 | Lids: a lit blotch on a closed lid under side light; pale wedges on a half-shut eye; a faint band on the feminine `Eye_Wide`; a half blink dips 0.45 mm into the cornea | §18 Stage 9 | missing | docs/development/character-studio/co6/README.md:193-197 | Open as written; no work since. | M |
| OPEN-14 | Mouth: a mild closed smile; pointed corners when open; `Jaw_Open` 1 is a bell; U and WQ show teeth, TH shows no tongue, FV's lip; a dark lining band under the upper lip | §18 Stage 9 | missing | docs/development/character-studio/co6/README.md:198-203 | Open as written; no work since. | L |
| OPEN-15 | Determined and Frustrated are close to a mild Angry | §18 Stage 9 (§10) | missing | docs/development/character-studio/co6/README.md:204 | Open as written; no work since. | S |
| OPEN-16 | Lashes and brows are blended cards: a fringe and a patch, not hairs | §18 Stage 6 (§5.2 brow thickness, §7.3) | missing | docs/development/character-studio/co6/README.md:205 | Open as written; no work since. | L |
| OPEN-17 | The pupil has no slider | §18 Stage 4 (§5.2) | missing | assets/characters/realistic/Realistic.base.json (no `pupil` parameter); crates/avatar/src/material.rs:184 (`pupil_size`, a value nothing draws); assets/characters/realistic/eyes.py:7 (the `Pupil_Size` shape, driven by nothing) | The shape moves 1.3 mm, under the 2 mm a parameter must move. Needs the Eye shading model. | M |
| OPEN-18 | Face identity ends: some still cross the head's own triangles (masculine `eye_size` -1 shows a lit streak); marks on raised brows; `jaw_shift` leaves a lip vertex 0.27 mm in the teeth; six parameters hard to see from the front; right-side parameters judged from numbers | §18 Stage 4 (validate combinations) | missing | docs/development/character-studio/co6/README.md:207-213, :712-716 | Open as written; no work since. | M |
| OPEN-19 | A cut-out (MASK) card on a skinned, morphed part shows the backdrop through the skin behind it | §18 Stage 6 | missing | docs/development/character-studio/co6/README.md:214 | Not re-checked here (needs a run). Blocks hair cards and lashes as cut-outs. | M |
| OPEN-20 | The face's simpler copies (LODs) were looked at in Blender only | §18 Stage 11 | missing | docs/development/character-studio/co6/README.md:215 | Open as written; no work since. | S |
| OPEN-21 | The editor's edit mode doesn't apply kept descriptions: a saved described character shows its base meshes until Play | §18 Stage 10 (Game Editor integration) | missing | crates/gameplay/src/lib.rs:68 (the only host that adds `DescriptionSystem`) | Plan CO13. | M |
| OPEN-22 | A new project, Insert > Character and an older project get only the stylized built-in library; the default game character is still stylized | §18 Stage 10 and 11 (§2: realistic is the default direction) | missing | crates/project/src/default_humanoid.rs:183; crates/editor/src/character_workspace/packs.rs:297-300 | The realistic package reaches a project only through a pack's Add to Project. | L |
| OPEN-23 | A script can't load a description by path; it sets `Data` to the file's text | §18 Stage 10 | missing | crates/gameplay/src/description_system.rs (no path API); wiki/Roadmap/Known-Limitations.md:47 | Open as written; no work since. | S |
| OPEN-24 | Most material values aren't drawn: only `tint`, `iris_tint` and `roughness_scale` | §18 Stage 5 (blocks it) | missing | crates/scene/src/material.rs:153-162 | `melanin`, `undertone`, `redness`, `detail_strength`, `scatter_scale` are stored and ignored. | XL |
| OPEN-25 | Binding-fitted and socket items have no content; a `binding` item would be drawn unfitted | §18 Stage 7 and 8 | missing | crates/avatar/src/item.rs:91; crates/avatar/src/resolve.rs:299-300 | Open as written; no work since. | XL |
| OPEN-26 | The realistic underwear is an interim fit (copied shapes and mask shapes), an exception to ADR-0035; a crumple at the briefs' back hem | §18 Stage 7 | missing | wiki/Architecture/Architecture-Decisions/ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md:52 | Moves to map regions when CO9 lands. | M |
| OPEN-27 | A character's parts pick their levels of detail one by one; skin shows through simplified garments from about 9 studs | §18 Stage 7 and 11 (§16) | missing | wiki/Roadmap/Known-Limitations.md:52; docs/development/character-studio/audit.md:152 | Seen on the stylized body. This stage's baseline reports it on the realistic underwear too: at level 1 the briefs lose their sides (refinement/baseline/README.md, defect H1; not re-run for this audit). | M |
| OPEN-28 | The identity bake's cost isn't measured; derived models are baked again every run and registered identities are kept for the run | §18 Stage 11 (§16) | missing | wiki/Roadmap/Known-Limitations.md:53 | The 6 ms apply figure is from a debug build before #415 and #416. | M |
| OPEN-29 | ADR-0038 is still Proposed, though Stage 3 passed its checkpoint | §18 Stage 0 | missing | wiki/Architecture/Architecture-Decisions/ADR-0038-Identity-Is-Baked-Into-Derived-Models-Named-By-MeshId.md:5 | Needs the project owner's accept or change. | S |
| OPEN-30 | The realistic family has no hair and no clothes beyond underwear | §18 Stage 6 and 7 | missing | assets/characters/realistic/items/; assets/characters/catalogue.json | Open as written; no work since. | XL |
| OPEN-31 | The stylized body isn't offered for new characters, while the brief says both families stay supported | §2, §17.1, §21 | partial | crates/editor/src/character_workspace.rs:194 | The family is kept for games and old files. A decision: does the brief reverse the 2026-11-16 request to remove it from the studio? | S |
| OPEN-32 | The stylized body's audit defects 2 to 11 stand (face skinning, eyes, hair, joints, shoes, garments, proportions) | §17.1 (benchmarks include stylized) | missing | docs/development/character-studio/audit.md:64-73 | It is still every game's default character (OPEN-22). | XL |
| OPEN-33 | Two things are called Character Studio; no way from the Game Editor to the workspace and back | §18 Stage 10 | missing | crates/editor/src/layout.rs:205; crates/editor/src/app/chrome/menus.rs:135 | Plan D12, CO12. | M |
| OPEN-34 | The Blender harness doesn't read the benchmark description files; it sets its looks itself | §18 Stage 0 and 11 (§17) | missing | assets/characters/humanoid/qa/ (no reader of `*.character.json`); docs/development/character-studio/progress.md:120 | The engine capture and the Blender check don't measure the same characters. | M |
| OPEN-35 | No in-engine baseline of the realistic character as it is on main | §18 Stage 0 (visual QA baselines) | partial | docs/development/character-studio/baseline/engine/captures.json (stylized only); docs/development/character-studio/refinement/baseline/README.md (this stage, in the working tree) | Being closed in this stage: 34 realistic bodies captured at rest, with a smile, a blink, an open jaw and a stride; 25 sheets kept. Not captured: the 180 face identity ends, hair and clothes (the family has none). | M |
| OPEN-36 | The sliders pass (every pair of parameters at their ends) hasn't run since Stage 3 | §18 Stage 0 and 3 | missing | docs/development/character-studio/progress.md:25 | #414 added two shapes to the body and #415 two to the brows. A 15 to 40 minute Blender run. | S |
| OPEN-37 | `measure.py --check` exits 1 in every recorded run: a full run counts the unmeasured sliders lines as worse, and a sliders run counts the rest | §18 Stage 0 and 11 (§20) | missing | assets/characters/humanoid/qa/measure.py:1204, :1235; docs/development/character-studio/stage3/measure-check.txt:49-58 | No one command says pass. A stage gate can't be read from an exit status. | S |
| OPEN-38 | Suite noise: the path-traced view mode test and the 500k sculpt timing fail on main on the reference machine; four others pass alone | §18 Stage 11 | missing | docs/development/character-studio/progress.md:25 | Not character code. Record it as known so a red suite isn't read as a character failure. | S |
| OPEN-39 | The plan's status text is stale: "waiting for review" on CO2 to CO6, two test names that no longer exist, and no section 11 | §18 Stage 0 | missing | plans/axcenity-engine-character-studio.md:99, :113-125 | The lead owns the plan. | S |
| OPEN-40 | The wiki's character pages are stale | §18 Stage 0 and 11 | missing | wiki/Engine-Systems/Characters.md:17, :68, :88; wiki/Engine-Systems/Character-Data.md:17; wiki/Architecture/System-Inventory.md:73; wiki/Reference/File-Formats.md:65; wiki/Roadmap/Known-Limitations.md:55, :59 | They say CO3 and CO4 wait for review, the workspace writes format 1, and the sliders check is red; Characters.md:88 names the agreement test by its old name with its 0.2 mm claim. None of this is true now. | S |
| OPEN-41 | progress.md has no entry for #416 (underwear always on) or #418 (start page: Recent, pictures, packs) | §18 Stage 0 | missing | docs/development/character-studio/progress.md:16-25 | Its newest entry is #415's. Open as written; no work since. | S |
| OPEN-42 | One normal and occlusion map serves both frames: detail that sits differently on the two is misplaced on the feminine frame (two short dark marks above each feminine eye) | §18 Stage 5 (§6.1) | missing | assets/characters/realistic/README.md:184 | Added at verification. Goes with OPEN-07: a map per frame, or layers the frame blends. | M |
| OPEN-43 | Seven of ARKit's 52 face shapes are not played: the standard controls have no place for them | §18 Stage 9 | missing | docs/development/character-studio/co6/README.md:97; crates/anim/tests/realistic_face.rs:170 | Added at verification. A limit of the 48 controls, not of this head. It matters only if face capture comes into scope; the brief doesn't ask for it. | S |
| OPEN-44 | Seen at CO4 and not closed since: the skin is evenly glossy, and the feminine face stares (the eyes read too open) | §18 Stage 5 and 4 | missing | docs/development/character-studio/progress.md:99; docs/development/character-studio/refinement/baseline/README.md (defects H4, M2) | Added at verification. The gloss waits for CO7. The stare isn't in CO6's still-short list; this stage's baseline sees it on both frames. | M |

### Closed since they were written (do not carry)

| Item | Where it was open | Closed by | Evidence |
|---|---|---|---|
| The sliders check red on three false eye counts | progress.md:69-72; co6/README.md:208 | Pseudo-normal signing, Stage 3 | docs/development/character-studio/stage3/sliders-check.txt; assets/characters/humanoid/qa/pseudo_normals.py |
| The 150 degree shoulder under its 0.8 local-volume target | progress.md:84 | Decided: held by ring area | assets/characters/realistic/qa/thresholds.json:2 |
| The `avatar` floor test left red at CO4 | progress.md:100 | CO5's shapes | crates/avatar/tests/shipped_library.rs:158 |
| The realistic slim benchmark named with a `weight` parameter | co5/README.md:92 | Fixed | assets/characters/benchmarks/make.py:49 |
| The workspace writes format 1 and poses its own preview | progress.md:134 | CO12 part 1 | crates/editor/src/character_workspace/tests.rs:240, :473 |
| Squared, raised shoulders in the idle | progress.md:38 | #414 | crates/anim/src/correctives.rs:272 |
| Feet through the floor, shoulders rolled forward | progress.md:19 | #415 | crates/anim/src/retarget_tests.rs:1064 |
| Teeth touching the lower lip with the jaw open; lips resting 2 mm apart | progress.md:101 | CO6 | wiki/Roadmap/Known-Limitations.md:61 |
| No brows, lashes or mouth interior on the realistic head | progress.md:99 | CO6 | crates/anim/tests/realistic_face.rs:113 |
| Nothing wrote the character library into a project | progress.md:133 | CO3 | crates/project/src/default_humanoid.rs:183 |
| The harness raised the arm when asked to lower it | progress.md:30 | #414 | docs/development/character-studio/lowered-shoulder/README.md |
| Stage 3 never reached main | progress.md:43 | #412 | git log: `6d9d1ae3` |

## 4. The brief's Stages 0 to 11 on the plan's milestones

One row per bullet of brief §18. The Gap column starts with the plan milestone the bullet falls under, or **New** when no milestone covers it. Status is what the code does today. 1 implemented, 64 partial, 35 missing.

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| MAP-S0-01 | Review current Character Studio implementation | §18 Stage 0 | partial | docs/development/character-studio/audit.md:1-5 (2026-11-14, before the overhaul) | New. The refinement audit files of this stage replace the old audit. | M |
| MAP-S0-02 | Identify completed and incomplete CO12 requirements | §18 Stage 0 | implemented | PLAN-CO12-01 to -15 in this file | CO12. Three of thirteen sub-items done, four partial, six missing. | S |
| MAP-S0-03 | Review the existing realistic and stylized families | §18 Stage 0 | partial | assets/characters/realistic/README.md; docs/development/character-studio/audit.md:36-97 | CO4 to CO6 for realistic. The stylized review is the 2026-11-14 audit. | S |
| MAP-S0-04 | Audit the existing UI and viewport interactions | §18 Stage 0 | partial | crates/editor/src/character_workspace/ui.rs:255-323 | New (another specialist's file). | S |
| MAP-S0-05 | Review parameter manifests | §18 Stage 0 | partial | assets/characters/realistic/Realistic.base.json (75 parameters, 13 groups) | New (another specialist's file). | S |
| MAP-S0-06 | Collect UX references | §18 Stage 0 | partial | docs/development/character-studio/research.md:88; docs/development/character-studio/references/ux/README.md | New. research.md A.7 has interface notes. This stage adds `references/ux/` (an index and a patterns file) and `references/hair/`, in the working tree. | M |
| MAP-S0-07 | Collect facial anatomy references | §18 Stage 0 | partial | docs/development/character-studio/research.md:35 | New. CO6 was judged against photographs kept out of the repository. `references/face/` has its five folders, all empty at verification. | M |
| MAP-S0-08 | Collect body anatomy references | §18 Stage 0 | partial | docs/development/character-studio/research.md:23 | New. No index exists, and `references/body/` doesn't exist yet. | M |
| MAP-S0-09 | Review asset licensing | §18 Stage 0 | partial | docs/development/character-studio/dependencies.md; assets/characters/transfer/shapes/LICENCE-RECORD.md | CO1. The brief's five classes (§13.5) aren't recorded per asset. | S |
| MAP-S0-10 | Establish visual QA baselines | §18 Stage 0 | partial | docs/development/character-studio/baseline/engine/; docs/development/character-studio/refinement/baseline/README.md | CO2. The committed baseline is stylized only. This stage's realistic baseline is in the working tree (OPEN-35). | M |
| MAP-S0-11 | Audit existing benchmark characters | §18 Stage 0 | partial | assets/characters/benchmarks/make.py:47-55 | CO2 (addition A1). 337 files exist; none is audited against brief §17.1's list. | S |
| MAP-S0-12 | Identify missing asset capability metadata | §18 Stage 0 | partial | crates/avatar/src/item.rs:19-57; docs/development/character-studio/refinement/audit-character-data.md (section 3) | New. Item manifests have no parameters of their own. This stage's character-data audit sets today's fields against the brief's metadata lists. | S |
| MAP-S1-01 | Redesign the three-panel workspace | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:74-104 | CO12. Toolbar, left categories, viewport, right panel and status line exist. No bottom viewport bar. | L |
| MAP-S1-02 | Improve category navigation | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:222-237 | CO12. Six categories, no features under them (Nose, Eyes). | M |
| MAP-S1-03 | Implement contextual properties | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:338-347, :457 | CO12 in part. The page follows the category; nothing follows a selected feature. | M |
| MAP-S1-04 | Improve viewport controls | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:257-275 | CO12 (turntable). Orbit, pan, zoom and reset exist. | M |
| MAP-S1-05 | Improve camera presets | §18 Stage 1 | partial | crates/editor/src/character_workspace/stage.rs:39-53 | CO12. Five views; no three-quarter and no per-feature framing. | S |
| MAP-S1-06 | Introduce cleaner control grouping | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:435-469 | CO12. Groups come from the manifest; each is one flat list of sliders. | M |
| MAP-S1-07 | Improve preset presentation | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:350-386; crates/editor/src/character_workspace/thumbnails.rs:77 | CO12, CO13. Text cards in the workspace; pictures only on the start page. | M |
| MAP-S1-08 | Implement undo/redo and category reset UX | §18 Stage 1 | partial | crates/editor/src/character_workspace.rs:577-604; crates/editor/src/character_workspace/ui.rs:486 | New for category reset. Undo, redo, whole reset and per-slider reset exist. | M |
| MAP-S1-09 | Introduce progressive customization | §18 Stage 1 | partial | crates/editor/src/character_workspace/ui.rs:431, :444; crates/avatar/src/manifest.rs:120 | New. Two levels today (`advanced`); three need a manifest field. | M |
| MAP-S1-10 | Introduce the character comparison workflow | §18 Stage 1 | missing | crates/editor/src/character_workspace.rs (searched compare, comparison, variant, snapshot: no state for it) | New. No milestone. | L |
| MAP-S1-11 | Establish viewport interaction architecture | §18 Stage 1 | missing | crates/editor/src/character_workspace/ui.rs:256 | New. CO12's "click on the body" is its seed. No picking in the workspace. | L |
| MAP-S2-01 | Implement feature selection | §18 Stage 2 | missing | crates/editor/src/character_workspace/ui.rs:256-275 | New (CO12 "click on the body to open its category"). | L |
| MAP-S2-02 | Implement body region selection | §18 Stage 2 | missing | assets/characters/realistic/Realistic.base.json (`regions`, 17 entries) | New. The region map and the body's second UV set can serve picking. | M |
| MAP-S2-03 | Implement facial feature selection | §18 Stage 2 | missing | assets/characters/realistic/Realistic.base.json (`regions`) | New. The regions are body regions for hiding skin; face features have none. | L |
| MAP-S2-04 | Add visual highlighting | §18 Stage 2 | missing | crates/render/src/draw_list.rs | New. Must reach `render` as plain draw data (CLAUDE.md invariant). | M |
| MAP-S2-05 | Implement contextual manipulation handles | §18 Stage 2 | missing | crates/editor/src/character_workspace/ui.rs:255 | New. | L |
| MAP-S2-06 | Connect manipulation handles to existing parameters | §18 Stage 2 | missing | crates/avatar/src/manifest.rs:110-137 | New. `Parameter` has no handle or viewport metadata. | M |
| MAP-S2-07 | Implement symmetry-aware interaction | §18 Stage 2 | missing | crates/editor/src/character_workspace.rs:477-478 | New. Sliders already set the mirror; handles can call the same function. | S |
| MAP-S2-08 | Add camera focus behavior | §18 Stage 2 | partial | crates/editor/src/character_workspace/ui.rs:241-253 | CO12. Picking Face frames the face; nothing else focuses. | M |
| MAP-S2-09 | Add direct manipulation undo/redo | §18 Stage 2 | missing | crates/editor/src/character_workspace.rs:443 | New. `drag` already makes one undo step per drag. | S |
| MAP-S2-10 | Prevent conflicts between camera navigation and feature editing | §18 Stage 2 | missing | crates/editor/src/character_workspace/ui.rs:257 | New. The left drag is the orbit today. | M |
| MAP-S2-11 | Priority order: nose, eyes, jaw, mouth, shoulders, waist, hips | §18 Stage 2 | missing | assets/characters/realistic/Realistic.base.json (nose 6, eyes 10, jaw 3, lips 5 parameters; `shoulder_width`, `waist_width`, `hip_width`) | New. Every priority has parameters to drive. Hips only widen (OPEN-02). | L |
| MAP-S3-01 | Improve overall body controls | §18 Stage 3 | partial | assets/characters/realistic/Realistic.base.json (frame, muscle, fat, height basic; eight proportions advanced) | CO5. `head_size` (advanced) is the head-to-body ratio. No chest proportion, and no muscle definition apart from muscle mass (one normal map, OPEN-07). | M |
| MAP-S3-02 | Introduce visual body presets | §18 Stage 3 | partial | crates/editor/src/character_workspace.rs:145-190 | CO13. Six presets as text; the brief lists Slim, Average, Athletic, Muscular, Heavyset, Lean, Broad. | M |
| MAP-S3-03 | Improve regional customization | §18 Stage 3 | partial | assets/characters/realistic/Realistic.base.json (group `build_regions`, 16 advanced sliders) | CO5. Eight regions, muscle and fat each; the brief lists thirteen with their own controls. | L |
| MAP-S3-04 | Add viewport region selection | §18 Stage 3 | missing | crates/editor/src/character_workspace/ui.rs:256 | New. Same work as MAP-S2-02. | M |
| MAP-S3-05 | Implement direct body manipulation | §18 Stage 3 | missing | crates/editor/src/character_workspace/ui.rs:256 | New. Same work as MAP-S2-05 for body handles. | L |
| MAP-S3-06 | Improve parameter descriptions and tooltips | §18 Stage 3 | missing | crates/avatar/src/manifest.rs:110-137 (no description field) | New. A manifest field and text for 75 parameters. | M |
| MAP-S3-07 | Validate deformation quality | §18 Stage 3 | partial | docs/development/character-studio/stage3/sliders-check.txt; assets/characters/realistic/qa/thresholds.json | CO4, CO5. Pairs of sliders at rest pass; OPEN-01, -03, -06 stand. | M |
| MAP-S4-01 | Reorganize existing facial parameters | §18 Stage 4 | partial | assets/characters/realistic/Realistic.base.json (47 parameters, 9 groups) | CO6, CO12. Grouped already; regrouping is manifest data. | M |
| MAP-S4-02 | Introduce facial presets | §18 Stage 4 | missing | crates/editor/src/character_workspace.rs:145-190 (body presets only) | CO13 (preset files). No face preset exists. | L |
| MAP-S4-03 | Improve feature-specific customization | §18 Stage 4 | partial | crates/editor/src/character_workspace/ui.rs:457-468 | CO12. One collapsible group per feature. | M |
| MAP-S4-04 | Implement facial camera focusing | §18 Stage 4 | partial | crates/editor/src/character_workspace/stage.rs:70 | CO12. One face framing. | M |
| MAP-S4-05 | Introduce direct facial manipulation | §18 Stage 4 | missing | crates/editor/src/character_workspace/ui.rs:256 | New. Same work as MAP-S2-05 for face handles. | L |
| MAP-S4-06 | Improve asymmetry controls | §18 Stage 4 | partial | crates/editor/src/character_workspace/ui.rs:429 | New. A Symmetry toggle exists; mirror, copy side and reset side don't. | M |
| MAP-S4-07 | Add missing high-value facial parameters | §18 Stage 4 | partial | assets/characters/realistic/Realistic.base.json (47 face parameters) | CO6 extended. By parameter ID, about 31 of the 63 features brief §5.2 lists have a parameter: eyes 5 of 11, brows 4 of 7, nose 5 of 11, cheeks 2 of 5, jaw 2 of 5, chin 3 of 5, lips 4 of 8, forehead 3 of 5, ears 3 of 6. Absent, among others: lid shapes, eye corners, brow thickness and angle, bridge width, nostril height and angle, jaw angle and definition, chin shape, cupid's bow, lip corners, philtrum, earlobe, pupil. | XL |
| MAP-S4-08 | Validate facial deformation combinations | §18 Stage 4 | partial | docs/development/character-studio/stage3/sliders-check.txt | CO6. Pairs at their ends pass; OPEN-18 stands. | M |
| MAP-S5-01 | Expand skin customization | §18 Stage 5 | partial | crates/scene/src/material.rs:153-162; crates/gameplay/src/description_system.rs:289; crates/editor/src/character_workspace/ui.rs:28-33 (no Skin page) | CO12 (Skin and Makeup page), CO7, CO17. From a script only: a Skin `tint` and `roughness_scale` are set by `SetMaterialValue` and drawn (test crates/player/src/default_humanoid_tests/description.rs:480). Nothing in the studio sets a material value. | L |
| MAP-S5-02 | Implement skin tone and undertone selection | §18 Stage 5 | partial | crates/scene/src/material.rs:153-162; crates/gameplay/src/description_system.rs:289 | CO7. A `tint` multiplies the skin's colour, from a script only. `melanin` and `undertone` are in the contract and not drawn. No studio control. | L |
| MAP-S5-03 | Add surface detail options | §18 Stage 5 | missing | crates/avatar/src/material.rs:106 (`detail_strength` only) | CO17 (freckles, moles, scars). | L |
| MAP-S5-04 | Implement makeup layers | §18 Stage 5 | missing | docs/development/character-studio/architecture.md:94 (room in `extra`) | CO17. No schema yet. | L |
| MAP-S5-05 | Establish the decal architecture | §18 Stage 5 | missing | crates/render/src/renderer/decals.rs (world decals, not characters) | New. No milestone (tattoos and markings, brief §6.3). | L |
| MAP-S5-06 | Improve material previews | §18 Stage 5 | missing | crates/editor/src/character_workspace/ui.rs (no material control) | CO12 (pickers). | M |
| MAP-S6-01 | Improve hairstyle browser | §18 Stage 6 | partial | crates/editor/src/character_workspace/ui.rs:511-563 | CO12. A chip per item; no pictures, search or favorites. | M |
| MAP-S6-02 | Expand hairstyle library | §18 Stage 6 | missing | assets/characters/catalogue.json (no realistic hair) | CO11. | XL |
| MAP-S6-03 | Add hair color customization | §18 Stage 6 | partial | crates/avatar/src/material.rs:226-271 (Hair values, contract only); crates/scene/src/material.rs:153-158 | CO11, CO7, CO12 pickers. A `tint` on a hair material is drawn, from a script only; `melanin`, `redness` and the rest are not. The realistic body has no hair to colour. | L |
| MAP-S6-04 | Add supported style-specific parameters | §18 Stage 6 | missing | crates/avatar/src/item.rs:20-56; crates/avatar/src/description.rs:46-50 | New. Items declare no parameters and a worn item holds no values. | L |
| MAP-S6-05 | Improve facial hair options | §18 Stage 6 | missing | assets/characters/catalogue.json | CO11. None exist to improve. | L |
| MAP-S6-06 | Validate grooming deformation and rendering | §18 Stage 6 | missing | crates/avatar/src/item.rs:118 | CO11, CO7. | M |
| MAP-S6-07 | Implement asset capability-aware controls | §18 Stage 6 | partial | crates/editor/src/character_workspace/ui.rs:537; crates/editor/src/character_workspace.rs:497 | New. Slots and items already hide when nothing fits the body. | M |
| MAP-S7-01 | Build visual clothing browser | §18 Stage 7 | partial | crates/editor/src/character_workspace/ui.rs:511-563 | CO12. The same page as hair (MAP-S6-01): a chip per item by label. No pictures, tags, search or filters, and nothing to list on the realistic body. | L |
| MAP-S7-02 | Expand clothing categories | §18 Stage 7 | missing | assets/characters/realistic/Realistic.base.json (slots top, outerwear, bottom, shoes; no item fits them) | CO10, CO16. | XL |
| MAP-S7-03 | Add outfit presets | §18 Stage 7 | missing | crates/editor/src/character_workspace.rs:145 | CO12, CO13. | M |
| MAP-S7-04 | Improve color and material variants | §18 Stage 7 | partial | crates/avatar/src/item.rs:107; crates/avatar/src/resolve.rs:279-284; crates/gameplay/src/description_system.rs:203 | CO9, CO12. The runtime resolves a worn item's variant and its own material values, and `SetItem` takes a variant (test crates/avatar/src/tests/resolving.rs:252, on a fixture). No shipped item has a variant and no studio control picks one. | M |
| MAP-S7-05 | Improve compatibility filtering | §18 Stage 7 | partial | crates/editor/src/character_workspace.rs:497 | CO9. Filtered by `bases` only. | S |
| MAP-S7-06 | Expand fitting QA | §18 Stage 7 | partial | assets/characters/humanoid/qa/thresholds.json | CO9. Fit checks exist for the stylized outfit only. | L |
| MAP-S7-07 | Validate clothing under different body shapes and animations | §18 Stage 7 | partial | crates/avatar/tests/realistic_parameters.rs:289 | CO9. The underwear follows the frame; nothing else to validate. | L |
| MAP-S7-08 | Implement clothing capability manifests | §18 Stage 7 | partial | crates/avatar/src/item.rs:20-56 | CO9 extended. Slot, layer, bases, hides, materials, variants, fit, tags and thumbnail exist; limits and operations don't. | M |
| MAP-S8-01 | Implement accessory browser | §18 Stage 8 | missing | crates/editor/src/character_workspace/ui.rs:28-33 (no Accessories category) | CO12. | M |
| MAP-S8-02 | Expand attachment support | §18 Stage 8 | partial | crates/anim/src/sockets.rs:34; crates/avatar/src/resolve.rs:299 | CO9, CO10. Seven sockets and a socket fit exist; no socket item ships. | L |
| MAP-S8-03 | Add material variants | §18 Stage 8 | partial | crates/avatar/src/item.rs:107; crates/avatar/src/resolve.rs:279-284 | CO9, CO10. The variant code serves every item kind (MAP-S7-04). No accessory ships. | M |
| MAP-S8-04 | Add accessory positioning | §18 Stage 8 | missing | crates/avatar/src/description.rs:46-50 | New. A worn item has no offset, turn or scale. | M |
| MAP-S8-05 | Validate attachment behavior under animation | §18 Stage 8 | partial | crates/anim/src/sockets.rs:34 | CO10. Sockets are tested in `anim`; no character accessory to test. | M |
| MAP-S8-06 | Implement accessory capability manifests | §18 Stage 8 | partial | crates/avatar/src/item.rs:61 | CO9 extended. The accessory kind and its socket exist. | M |
| MAP-S9-01 | Expand expression browser | §18 Stage 9 | partial | crates/editor/src/character_studio/expressions.rs; crates/anim/src/face_play.rs:42 | CO12, D12. It lives in the Game Editor's face tab, not in the workspace. | M |
| MAP-S9-02 | Improve expression controls | §18 Stage 9 | partial | crates/editor/src/character_studio/rig.rs | CO12. The 48 controls are sliders in the face tab only. | M |
| MAP-S9-03 | Add facial preview modes | §18 Stage 9 | partial | crates/editor/src/character_studio/expressions.rs:149-152; crates/editor/src/character_workspace.rs:732 | CO12. The Game Editor's face tab previews control values, an expression (scaled, or blended with another) and visemes on a selected character. The workspace has none. | M |
| MAP-S9-04 | Add animation preview | §18 Stage 9 | partial | crates/editor/src/character_workspace.rs:732 | CO12. The idle plays; no clip can be chosen. | M |
| MAP-S9-05 | Validate expressions against different facial identities | §18 Stage 9 | partial | assets/characters/humanoid/qa/face_checks.py | CO6. Expressions and identity ends are checked apart, not combined. | M |
| MAP-S10-01 | Expand preset management | §18 Stage 10 | partial | crates/editor/src/character_workspace.rs:145 | CO13. Built-in presets only. | M |
| MAP-S10-02 | Implement selective randomization | §18 Stage 10 | missing | crates/editor/src/character_workspace.rs (searched random: 0 hits) | New. Plan section 0 decision 6 wants NPC randomizing in games, so it belongs in `avatar`, not the editor. | L |
| MAP-S10-03 | Implement feature locking | §18 Stage 10 | missing | crates/editor/src/character_workspace.rs | New. Editor state, never saved with the character. | M |
| MAP-S10-04 | Improve favorites | §18 Stage 10 | missing | crates/editor/src/character_workspace.rs (searched favorite, favourite: 0 hits) | New. None exist to improve. | M |
| MAP-S10-05 | Improve character save/open workflows | §18 Stage 10 | partial | crates/editor/src/start/characters.rs; crates/editor/src/character_workspace/recent.rs:64 | CO13. Recent, pictures, packs and Delete exist (#418). No New or Open in the workspace toolbar and no autosave. | M |
| MAP-S10-06 | Complete comparison tools | §18 Stage 10 | missing | crates/editor/src/character_workspace.rs | New. | L |
| MAP-S10-07 | Complete the Character Studio walkthrough | §18 Stage 10 | partial | crates/editor/src/character_workspace/tests.rs:274; crates/editor/src/app/character_workspace.rs:143 | CO12 if it means the walkthrough tests; new if it means a first-run guide. Ask. | M |
| MAP-S10-08 | Complete Game Editor integration | §18 Stage 10 | partial | crates/editor/src/character_workspace/packs.rs:289; crates/editor/src/app/chrome/menus.rs:135; crates/editor/src/layout.rs:205 | CO12 (D12), CO13. One way only: a pack's Add to Project writes characters and the library into a game project. No entry from the Game Editor, no round trip, no placing a character file, and edit mode doesn't apply a description (OPEN-21, OPEN-33). | L |
| MAP-S11-01 | Run automated tests | §18 Stage 11 | partial | docs/development/character-studio/progress.md:25 | Plan Stage 7. The suite runs with known noise (OPEN-38). | S |
| MAP-S11-02 | Run Blender asset QA | §18 Stage 11 | partial | assets/characters/humanoid/qa/measure.py:1114 | Plan Stage 7. OPEN-37. | M |
| MAP-S11-03 | Run in-engine visual QA | §18 Stage 11 | partial | crates/player/src/character_capture.rs:707 | CO2. | M |
| MAP-S11-04 | Validate runtime consistency | §18 Stage 11 | partial | crates/editor/src/character_workspace/tests.rs:473 | CO3, CO13. Holds for body and face; nothing else exists yet. | M |
| MAP-S11-05 | Test extreme parameter combinations | §18 Stage 11 | partial | docs/development/character-studio/stage3/sliders-check.txt | CO5, CO6. Pairs, at rest. Not in poses and not with clothes. | L |
| MAP-S11-06 | Test all supported body families | §18 Stage 11 | partial | crates/editor/src/character_workspace/tests.rs:402 | Every page draws on both bodies. OPEN-31 decides what "supported" means. | M |
| MAP-S11-07 | Validate clothing and accessories | §18 Stage 11 | missing | assets/characters/realistic/items/ | CO9, CO10. | L |
| MAP-S11-08 | Review performance | §18 Stage 11 | missing | docs/development/character-studio/progress.md:37 | CO8, CO18. Nothing measured. | L |
| MAP-S11-09 | Run benchmark character comparisons | §18 Stage 11 | partial | assets/characters/benchmarks/ | CO2 (A1). 337 characters; compared by eye. | M |
| MAP-S11-10 | Review visual regression evidence | §18 Stage 11 | partial | crates/player/src/character_capture.rs:707 | New in part. The capture is deterministic; no picture is compared with a stored one. | M |
| MAP-S11-11 | Document the completed system | §18 Stage 11 | partial | docs/editor/character-studio.md | Plan Stage 7. OPEN-39, OPEN-40. | M |

### Plan milestones the brief does not mention but depends on

| Milestone | Needed by | Why |
|---|---|---|
| CO7 character shading | Stage 5 (all of it), Stage 6 (hair colour, cut-out cards), Stage 4 (iris and pupil) | Skin tone, undertone, scattering and eye values are stored and not drawn (crates/scene/src/material.rs:153-162) |
| CO8 compute posing | §16, Stage 11 | Every live shape is morphed in every pass today |
| CO9 clothing system | Stages 7 and 8 | No binding, no region hiding, no import |
| CO10 wardrobe wave 1 | Stages 7 and 8 | Nothing to browse; cap, glasses and watch are its socket items |
| CO11 hair, card tier | Stage 6 | The brief calls the groom architecture "existing" (§7.3); only its schema exists |
| CO13 integration | Stages 8, 10, 11 | Preset files, runtime reproduction, the Game Editor round trip, edit mode |
| CO16 wardrobe wave 2 | Stage 7 (§8.1 sweaters, coats), Stage 8 (§9 hats, belts, bags, jewellery) | The brief's category lists are wave 2's |
| CO17 skin, phase 2 | Stage 5 | Freckles, moles, scars, makeup |
| CO18 performance | Stage 11, §16 | Budgets, LOD sets, the bake's cost |
| CO14 strands, CO15 cloth simulation | Not needed | The brief asks for neither. "Strand density" (§7.2) and "skirts where supported" (§8.1) are the only hints |

### Where the brief and the plan disagree

| ID | Brief | Plan or later decision | What to settle |
|---|---|---|---|
| DIS-01 | The header says the plan's section 11 maps the brief's stages | The plan has sections 0 to 10 | The lead adds section 11 |
| DIS-02 | §22: interaction first, the catalogue later. Stage order: UX, viewport, body, face, skin, hair, clothing | progress.md:11: after CO12 comes CO7, CO8, then CO9 to CO11 | One order. Stages 5 to 8 can't finish before CO7 and CO9 |
| DIS-03 | §2, §17.1, §21: realistic and stylized families both stay supported and both are benchmarked | Section 0 decision 1 keeps stylized as an alternative family; on 2026-11-16 the project owner asked to remove it, and it is no longer offered for new characters (crates/editor/src/character_workspace.rs:194) | Is stylized offered again, or supported for old files and games only? |
| DIS-04 | §8.1 lists Underwear as a wardrobe category | #416: underwear always on, its slots hidden from the Clothing page (crates/editor/src/character_workspace/ui.rs:516-528); scripts can't remove it | Stays always on, or becomes a choice |
| DIS-05 | §19: six agents: UX, face, body, hair with clothing and accessories, skin and materials, QA | D13 and plan section 5: modelling, face, hair, clothing, studio UI, rendering and performance | New numbering clashes with progress.md ("Agent 1" is modelling there). No rendering owner in the brief, yet Stage 5 needs CO7 |
| DIS-06 | §3.1: left navigation is Body, Face, Hair, Clothing, Skin and Makeup, Accessories, Expressions, Presets; lighting moves to a bottom bar | CO12: Presets first, and a Stage category | Order, and whether Stage stays a category |
| DIS-07 | §10: expressions and animations are preview-only in the studio | D12: the face tab's editors (rig mapping, expression and lip sync authoring) also appear in the workspace's Face category | Whether the F6 editors move in at all |
| DIS-08 | §13.5 class 3: licensed commercial assets; §13.1 names commercial sources | D8, section 8 and dependencies.md: only CC0, MIT and Apache-2.0 content ships; section 0 decision 2 has a Premium/imported tier brought by users | State that class 3 is never in the repository |
| DIS-09 | §13.4: a references directory tree | CO6's reference photographs stayed out of the repository | The tree holds index files with links only |
| DIS-10 | §17.2: studio, warm, cool and outdoor lighting, close-up skin | visual-qa.md: two lighting presets, and the settings never change | Add presets beside the two; never change the two |
| DIS-11 | §17.3: don't loosen thresholds because a result looks acceptable | The 150 degree shoulder moved from local volume to ring area by a recorded decision; the proportion ranges were cut to pass the pairs check and the confirmation isn't recorded (OPEN-02) | Record the range decision |
| DIS-12 | §14.1: every asset declares supported parameters, ranges and operations; §9: accessories have position, rotation and scale | D6, D10 and the item schema: variants and material slots only; a worn item is an ID, a variant and material values | A schema change in `avatar` (below) |
| DIS-13 | §11: randomization in the studio | Section 0 decision 6: games randomize NPCs at runtime | One randomizer in `avatar`, used by both |
| DIS-14 | §16: budgets from measured results | Plan section 7: budgets written down as Expected before measuring | Keep section 7 as targets until CO18 measures |
| DIS-15 | §7.3, §9: "existing groom architecture", "existing character binding architecture" | Both are schemas with no code behind them (crates/avatar/src/item.rs:91, :118) | Plan them as new work |
| DIS-16 | §7: one hairstyle library, no quality tiers named ("strand density" is a parameter in §7.2) | Section 0 decision 3: three hair levels (cards, better cards, strands); strands optional, never required | Say that the brief's hair work is the card tier, and that strand controls show only where a groom has strands |
| DIS-17 | Stages 1, 2 and 10: acceptance is about use ("a new user can create and modify a character") | Section 0's quality gate: a stage advances only when the characters measurably look and behave better, on the benchmark set | What the gate measures for a stage that changes no character (a walkthrough test, a fixed task list) |
| DIS-18 | Principle 3, §3.4, §16: handles and controls update the character in real time; incremental geometry updates | D4 and ADR-0038: every change bakes new derived models; while a slider is dragged the preview applies at most every 120 ms (crates/editor/src/character_workspace.rs:69) | Whether a drag previews with live shapes and bakes on release, or the bake is made fast enough. Measure first (OPEN-28) |

## 5. docs/editor/character-studio.md against the code

The file is accurate on the workspace and the start page: categories, views, presets, undo, files, Recent's limit of 24, the 3 second listing, 360 by 450 pictures and pack formats all match the source. What is no longer true or can't be shown:

| ID | Line | Statement | Code |
|---|---|---|---|
| DOC-01 | 32 | "The default-humanoid work adds its body and customization workspaces to the same tab." | Not so. Body and customization are the separate workspace (crates/editor/src/character_workspace.rs:1-4); the tab has Rig, Expressions and Lip sync only (crates/editor/src/character_studio/) |
| DOC-02 | 3 | Cites the old CS2 research document and its decision D9 for the split into workspace and tab | The governing decisions are ADR-0031 and the plan's D12, which says the tab is to be renamed; the file doesn't say so |
| DOC-03 | 12 | Applying with the bake takes about 6 ms in a debug build | Measured at CO12 part 1, before the underwear and its 37 to 53 copied shapes a garment. Not measured since |
| DOC-04 | 13 | "its hair and clothes are CO10 and CO11" | True, but it reads as soon; neither has started and CO9 comes first |
| DOC-05 | whole file | Says nothing of what CO12 still lacks | The list is only in wiki/Roadmap/Known-Limitations.md:47 |

## Defects found

| Severity | Defect | Evidence |
|---|---|---|
| Medium | Three committed check outputs hold a local home-directory path with a user name. The repository is public | docs/development/character-studio/stage3/measure-check.txt:48; stage3/sliders-check.txt:18; lowered-shoulder/measure-check.txt:48 |
| Medium | `measure.py --check` never exits 0 in a recorded run: sections not measured count as worse, so the full run fails on the sliders lines and the sliders run on the rest | assets/characters/humanoid/qa/measure.py:1204, :1235; docs/development/character-studio/stage3/measure-check.txt:49-58 |
| Medium | A game's default character and a new project's library are still the stylized package; the realistic one reaches a project only through a pack | crates/project/src/default_humanoid.rs:183; crates/editor/src/character_workspace/packs.rs:297-300 |
| Medium | The Game Editor's face tab and the workspace are both titled Character Studio | crates/editor/src/layout.rs:205; crates/editor/src/character_workspace/ui.rs:147 |
| Low | The plan cites two tests that no longer exist under those names, and CO3's agreement test now compares MeshIds, not corner positions to 0.2 mm | plans/axcenity-engine-character-studio.md:99, :125; crates/editor/src/character_workspace/tests.rs:473-509 |
| Low | The plan still says CO2 to CO6 wait for review, and that the sliders check is red | plans/axcenity-engine-character-studio.md:113-125 |
| Low | Wiki pages say CO3 and CO4 wait for review, the workspace writes format 1, and the sliders check is red; one names the agreement test by its old name | wiki/Engine-Systems/Characters.md:17, :68, :88; wiki/Engine-Systems/Character-Data.md:17; wiki/Architecture/System-Inventory.md:73; wiki/Reference/File-Formats.md:65; wiki/Roadmap/Known-Limitations.md:55, :59 |
| Low | ADR-0038 is still Proposed | wiki/Architecture/Architecture-Decisions/ADR-0038-Identity-Is-Baked-Into-Derived-Models-Named-By-MeshId.md:5 |
| Low | progress.md points a measurement at "CO19", a milestone that doesn't exist | docs/development/character-studio/progress.md:101 |
| Low | docs/editor/character-studio.md line 32 describes a plan that didn't happen (DOC-01) | docs/editor/character-studio.md:32 |
| Low | The realistic manifest has no pupil parameter, though the contract has `pupil_size` and the eyes carry a `Pupil_Size` shape | assets/characters/realistic/Realistic.base.json; crates/avatar/src/material.rs:184; assets/characters/realistic/eyes.py:7 |

## Shared contracts that would have to change (the lead owns them)

| Contract | Change | Needed by |
|---|---|---|
| Base manifest, `Parameter` (crates/avatar/src/manifest.rs:110-137) | A description or tooltip, an icon, a customization level of three in place of `advanced`, a feature or subcategory, and viewport handle data (which region, which axis, which parameters) | Stages 1, 2, 3; brief §3.3, §15 |
| Base manifest, regions | Pickable feature regions on the face and body, apart from the regions that hide skin | Stage 2 |
| Base manifest, skeleton | Corrective rules and twist driver rules as data (OPEN-04, OPEN-05) | Stage 3 |
| Item manifest (crates/avatar/src/item.rs:20-56) | Per-item parameters with ranges and defaults, supported operations, known limits, attachment adjustment limits | Stages 6, 7, 8; brief §14.1 |
| Description, `WornItem` (crates/avatar/src/description.rs:46-50) | Per-item parameter values; an accessory's offset, turn and scale | Stages 6, 8 |
| Description, skin and makeup | Makeup layers and markings, today only room in `extra` (architecture.md:94) | Stage 5 |
| Preset files | Whole and partial descriptions (face, body, hair, outfit) as files (architecture.md §18 names them for CO13) | Stages 3, 4, 7, 10 |
| A randomizer in `avatar` | Valid ranges and compatible items, with locks passed in | Stage 10; plan section 0 decision 6 |
| Material values (crates/avatar/src/material.rs) | Makeup, freckles, decals; nothing changes until CO7 draws the existing ones | Stage 5 |
| `visual-qa.md` and the capture's presets | More lighting presets and face expression views, added beside the fixed ones | Stage 0, brief §17.2 |
| `render` draw data | A highlight for a picked region, as plain data | Stage 2 |
| The plan | A section 11 with this mapping; a milestone or stage for each New row | Stage 0 |

## File ownership notes for parallel work

- `crates/editor/src/character_workspace/ui.rs` (607 lines) holds the toolbar, categories, viewport, every page and the status line. Brief Stages 1, 2, 3, 4, 9 and 10 all change it. Split it by page before parallel work starts, or one agent owns it.
- `crates/editor/src/character_workspace.rs` (780 lines) holds the state, undo, presets and the preview. Comparison states, locks and randomization all touch its undo. One owner.
- `crates/avatar/src/manifest.rs`, `item.rs`, `description.rs` and `docs/development/character-studio/architecture.md` are the lead's. Every specialist needs a field there; collect the fields first and land them in one change.
- `assets/characters/realistic/Realistic.base.json`, the item manifests and `assets/characters/catalogue.json` are generated (`manifests.py`, `catalogue.py`). Face and body specialists both regenerate them, and `RealisticParts.glb` (32 MB, binary) can't be merged. Rebuild in sequence, never in parallel branches.
- `assets/characters/humanoid/qa/` is shared by both families. One owner (the brief's QA agent).
- The brief's agent numbers (§19) differ from the plan's (section 5). Name agents by role in every hand-back.
- The start page (`crates/editor/src/start/characters.rs` and `characters/`, `shelf.rs`, `packs.rs`, `recent.rs`, `thumbnails.rs`) is separate from the workspace files and can have its own owner.
- `crates/render` and `crates/scene/src/material.rs` have no owner in the brief's agent list. CO7 needs one.

## Risks

- The brief's Stages 5 to 8 rest on CO7, CO9, CO10 and CO11, none of which has started and each of which is XL. If the brief's order is followed literally, four stages stall on missing foundations.
- The brief presumes more exists than does: the groom and binding "architectures" are schemas only.
- Every geometry change rebuilds one 32 MB binary. Two specialists changing shapes at once will conflict.
- The proportion ranges are short (0.85 to 1.15, hips only widen). Direct manipulation of the waist and hips will feel weak until the body takes wider ranges, which needs new shapes or correctives.
- No single command says the geometry check passes, and the sliders pass takes 15 to 40 minutes, so it gets skipped (it was after #415).
- No picture is compared automatically. Visual regression rests on someone looking.
- Every change re-bakes the character's meshes, and a drag shows at most about eight updates a second (DIS-18). Direct manipulation will feel slow unless this changes; the bake's cost is unmeasured in a release build.
- Stylized is still the character every game starts with, while the studio no longer offers it. What a creator makes and what a new game shows are different families.
- Scope: about a third of the brief's stage bullets are new work, on top of eleven plan milestones that have not started (CO7 to CO11, CO14 to CO18) and two that are part done (CO12, CO13).
- The status text of the plan and the wiki is stale in places. An agent reading them alone will redo finished work or trust a red check that is green.

## Verification

A second pass on 2026-11-16, made to refute this audit, not to agree with it. Every "missing" and "implemented" row was checked against the source under other names and in other crates. Over 30 of the 55 "partial" rows of section 4 were opened, and every partial row of sections 1 to 3. Every cited path and line was opened. The brief's §18 was counted again: 99 bullets and the Stage 2 priority list, 100 rows, none absent. Plan section 9 has 16 criteria and 16 rows. Nothing was built or run.

The counts in the Summary and under each heading are the corrected ones.

### Statuses changed

| Row | Was | Now | Why |
|---|---|---|---|
| PLAN-CO12-03 | missing | partial | The Hair and Clothing pages are a browser without pictures: a chip per item, and wearing one is tested (crates/editor/src/character_workspace/ui.rs:511-563; tests.rs:274). The same reasoning made numeric entry partial |
| OPEN-35 | missing | partial | Another specialist of this stage captured a realistic baseline (refinement/baseline/, in the working tree) |
| MAP-S0-12 | missing | partial | This stage's character-data audit does the task (audit-character-data.md, section 3) |
| MAP-S5-01, MAP-S5-02 | missing | partial | A script can set a Skin `tint` and `roughness_scale` and they are drawn (`SetMaterialValue`, crates/gameplay/src/description_system.rs:289; crates/scene/src/material.rs:153-162; test crates/player/src/default_humanoid_tests/description.rs:480). Nothing in the studio |
| MAP-S6-03 | missing | partial | The same `tint` path colours a hair material |
| MAP-S7-01 | missing | partial | It is the page MAP-S6-01 was rated partial for |
| MAP-S7-04, MAP-S8-03 | missing | partial | The runtime resolves variants and `SetItem` takes one (crates/avatar/src/resolve.rs:279-284; test crates/avatar/src/tests/resolving.rs:252). No shipped item has one |
| MAP-S9-03 | missing | partial | The Game Editor's face tab previews controls, expressions and visemes (crates/editor/src/character_studio/expressions.rs:149-152), as MAP-S9-01 and -02 already said |
| MAP-S10-08 | missing | partial | A pack's Add to Project is one-way integration that exists and is tested (crates/editor/src/character_workspace/packs.rs:289) |

### Rows added

| Row | Why |
|---|---|
| OPEN-42 | The realistic package's own "Known limits" (assets/characters/realistic/README.md:184) has a limit no row carried |
| OPEN-43 | co6/README.md:97 records seven ARKit shapes that are not played |
| OPEN-44 | progress.md:99 lists two things seen at CO4 that no later entry closes |
| DIS-16 | Section 0 decision 3 (three hair levels) has no counterpart in the brief |
| DIS-17 | Section 0's quality gate measures characters; three brief stages change none |
| DIS-18 | D4's bake on every change against the brief's real-time handles (the 120 ms limit is at crates/editor/src/character_workspace.rs:69) |

### Text and citations corrected

| Where | Correction |
|---|---|
| PLAN-CO9-01 | "Only the schemas and the catalogue exist" was wrong. Variants are resolved, mask-shape regions hide skin, and a socket item gets its Attachment (crates/avatar/src/resolve.rs:279-284, :304; crates/gameplay/src/description.rs:368). Binding, push-out, map-region hiding and import are the parts not built |
| PLAN-CO13-03 | "Nothing places a character in a scene" was wrong for scripts: `Players:CreateHumanoidModelFromDescription` does, with a test (crates/player/src/default_humanoid_tests/description_lua.rs:217). True for the editor |
| PLAN-CO13-04 | That test has two characters in one scene, one described |
| PLAN-CO6-01, OPEN-17, the pupil defect | A `Pupil_Size` shape exists on `Eyes` and nothing drives it (assets/characters/realistic/eyes.py:7). `pupil_size` is at crates/avatar/src/material.rs:184, not :125 |
| PLAN-CO7-01 | Two sub-items are partly covered by older work: cut-out-aware ray traced shadows and GPU timers. Status kept: no character shading exists |
| PLAN-CO12-02 | The data contract already has `skin` and `accessories` categories (crates/avatar/src/manifest.rs:97-104) |
| PLAN-CO17-01 | The skin values are at crates/avatar/src/material.rs:137-178, not :80-113 |
| PLAN-CO18-01 | #421 (Group as Mesh) merged after the range first audited. It doesn't merge animated characters |
| PLAN-CO9-01, OPEN-25, DIS-15 | `Fit::Binding` is at crates/avatar/src/item.rs:91; line 83 is the `Fit` enum |
| OPEN-27 | The audit said the realistic underwear was not re-checked. This stage's baseline reports the fault there |
| OPEN-40 and the wiki defect | wiki/Engine-Systems/Characters.md:76 is still true and was removed. Added: Characters.md:68 and :88, wiki/Architecture/System-Inventory.md:73, wiki/Reference/File-Formats.md:65 |
| MAP-S0-06, -07, -08, -10 | "The references folder is empty" and "stylized only" were true when written. `references/ux/`, `references/hair/` and `refinement/baseline/` now exist in the working tree |
| MAP-S0-02 | CO12's count follows PLAN-CO12-03: three done, four partial, six missing |
| MAP-S3-01 | `head_size` is the head-to-body ratio the brief lists; it was called absent |
| MAP-S4-04 | The Face framing is at crates/editor/src/character_workspace/stage.rs:70; line 50 is its label |
| MAP-S4-07 | "6 of the brief's 11" counted parameters, not features. Counted by feature: 31 of 63 over the nine groups, nose 5 of 11 |
| Summary | #419 to #423 are on main at the audited commit and hold no character work. "Schemas only" no longer names variants |
| Risks | The bake on every change is added as a risk to direct manipulation |

### Checked and upheld

- **Missing, confirmed.** CO7's shading models, CO8, CO10, CO11, CO14, CO15, CO16, CO17 and CO18 have no code under any name searched (sheen, subsurface, anisotropy, mikk, KHR_materials, hashed alpha; spring bone, groom, strand; jiggle, secondary motion, cloth). In the workspace and its start page: no turntable, comparison, snapshot, lock, favorite, randomize, search, picking, highlight, handle, clip or expression choice, autosave.
- **The Game Editor entry.** The workspace opens from the start screen and the `--character-studio` flag only (crates/editor/src/app/session.rs:1533-1536; crates/editor/src/main.rs:304).
- **Implemented, confirmed.** Each test named for CO1, CO3 to CO6, CO12 and CO13's save and load exists at the cited line and tests what the row says. CO4 to CO6 stay implemented: their done-when lines are met and the project owner passed the checkpoint; what is short is carried as open items.
- **Counts.** 75 parameters in 13 groups, 47 of them face parameters in 9 groups; 17 regions, 7 sockets, 7 slots; 15 catalogue items, 3 for the realistic body; 214 realistic and 123 stylized benchmark files; six corrective rules.
- **Defects.** Each is real today. The three check outputs do hold a local home path (found in no other file under `docs/development/character-studio/`, `assets/characters/` or `wiki/`). `measure.py --check` exits 1 in every recorded run, and again in this stage's baseline run.
- **The topic file.** DOC-01 to DOC-05 hold. Its other statements that were sampled match the code: the lighting and backdrop names, Recent's 24, the 3 s listing, 360 by 450 pictures, the 20 s thread, four loads a frame, and the window title.

### Not checked

- Nothing was run, so OPEN-19 (the cut-out card) and OPEN-36 (the sliders pass) stand as written.
- The other specialists' files were read only where a row here rests on them.
