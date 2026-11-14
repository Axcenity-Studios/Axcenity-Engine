# Character Studio overhaul: progress

Where the overhaul stands, newest first. The plan and its stage IDs are in [plans/axcenity-engine-character-studio.md](../../../plans/axcenity-engine-character-studio.md).

| Stage | State | Notes |
|---|---|---|
| 1. Research and audit | **Done, approved** (2026-11-14) | [research.md](research.md), [audit.md](audit.md), [the plan](../../../plans/axcenity-engine-character-studio.md) (decisions in its section 0); baseline in [baseline/](baseline/) by `assets/characters/humanoid/qa/` |
| Before Stage 2 | **Done** (2026-11-14) | #401 fixed (waist hole, clothing intersections), tested and merged; #402 merged; #400 brought onto main and merged; the full suite run, its Windows-only editor failures fixed in #404 (see the log) |
| 2. Architecture and contracts (CO1) | **Done** (2026-11-14) | [architecture.md](architecture.md), [agent-assignments.md](agent-assignments.md), [testing.md](testing.md), [visual-qa.md](visual-qa.md), [dependencies.md](dependencies.md); ADR-0032 to ADR-0037; the `avatar` crate; the stylized humanoid's manifests (see the log) |
| 3. Foundation (CO2–CO6) | Next | Waits for Paul's go-ahead |
| 4. Parallel specialists (CO7–CO12) | Not started | |
| 5. Integration (CO13) | Not started | |
| 6. QA, optimization, second wave (CO14–CO18) | Not started | |
| 7. Final integration | Not started | |

## 2026-11-14: Stage 2 (CO1), architecture and contracts

- **Agents:** the lead alone, as the plan assigns CO1; no specialist started.
- **Contracts:** [architecture.md](architecture.md) fixes the three layers (Character Studio, Character Runtime, Character Data), the seven pipeline systems and their owners (addition A2), the description (format 2, with A3 field by field), the base model and item manifests, what parameters mean (spans, corners, constraints, regions; identity against live shapes), fitting methods, the material parameter set per shading model, the groom file's binary layout, the skeleton's additive twist bones, the face's layers (addition A4), how parts take the body's shapes, the content library and catalogue, and the Rust and Lua signatures for CO3 and CO9. [agent-assignments.md](agent-assignments.md), [testing.md](testing.md) (with thresholds agreed for Stage 3), [visual-qa.md](visual-qa.md) (the A1 benchmark set) and [dependencies.md](dependencies.md) beside it.
- **Decisions:** ADR-0032 (semi-realistic default family; supersedes ADR-0030's art direction), ADR-0033 (one description, a parameter manifest), ADR-0034 (content library and catalogue), ADR-0035 (binding and region hiding; supersedes ADR-0030's masks), ADR-0036 (shading models, modulating values), ADR-0037 (one groom, three tiers). ADR-0030 is marked superseded; its stylized asset is still shipped.
- **Code:** `crates/avatar` (new, `axcenity-avatar`, depends on `assets` only): `Description` (format 2, unknown fields kept, format 1 migrated), `BaseManifest`, `ItemManifest`, `Catalogue`, `Library`, `ShadingModel` and material values, checks returning every `Problem`, `parameter_values`, `body_shape_weights`, `span`. `assets/characters/humanoid/manifests.py` (plain Python, run by `build.py` after its JSON) writes `Humanoid.base.json` (`axcenity/stylized`: 31 parameters, 10 regions, 7 slots, 7 sockets, 8 materials) and twelve `items/*.item.json`; `assets/characters/catalogue.py` writes `catalogue.json`. **No behaviour change:** nothing in the engine reads them yet; the workspace still writes format 1.
- **Red, then green:** the tests were written against stubs (`todo!()`) before the logic and before the manifests existed. First run: `35 tests run: 11 passed, 24 failed`. The 11 that passed only exercised serde-derived round trips and the ID helpers, which the derives make true from the start; every migration, check, resolution and shipped-library test failed. After the code and the manifests: `35 tests run: 35 passed`. A deliberate wrong shape name in `Humanoid.base.json` (`Build_Muscly`) made `every_manifest_entry_names_shapes_bones_and_materials_its_model_has` fail with `parameters[2].shapes[0]: the body has no shape`; the file was regenerated.
- **Agreement tests** (written after the manifests, so not seen red): `project` `the_character_data_manifests_describe_the_default_humanoid` (the manifest's parts file is the embedded one; sockets are `HUMANOID_SOCKETS`; attribute drivers are `anim::BODY_SLIDERS`; every object of the parts file is the body, a face part or an item; the default outfit matches); `editor` `a_saved_look_migrates_to_the_same_parts_and_shapes` (every preset and a mixed look: same parts, same body shape weights as `Look::shape_weights`).
- **Dependencies added:** none (`serde`, `serde_json`, `thiserror` already pinned). `cargo hakari generate` also removed a stale `zeroize` line from `crates/workspace-hack`.
- **Visual QA:** no asset or renderer change, so no new renders; the baseline stands.

## 2026-11-14: the plan moves to `plans/`

- The plan is now [plans/axcenity-engine-character-studio.md](../../../plans/axcenity-engine-character-studio.md), with the engine's other plans; Stage 1's research, audit, this log and the baseline stay here. Its status lines now say what's done: #400, #403 (Stage 1) and #404 (the editor test fixes) are merged.

## 2026-11-14: before Stage 2

- **#401 (CS1) fixed before merging**, as decision 4 asked. The waist hole came from the tee's sleeve cut: an endless plane that took the side of the torso with it. It now cuts only near the arm. The clothes are kept clear of the body and of each other on both frames and every build: new `assets/characters/humanoid/fit.py`, run by `build.py`. Each garment is pushed out per shape key and per frame-and-build mix. An inner garment follows the outer one's shape where it's covered, then is pushed clear of it. The sneaker sole covers the foot's footprint, and the hip weights are smoothed so the trousers don't tear in a hip flex. Before and after: [audit.md](audit.md) §2.3.
- **Regression tests:** two in `crates/project/src/default_humanoid.rs` (no hole round the waist of the shipped character; the modular outfit covers the waist with and without the jacket), red on the old asset and green on the new one. `measure.py --check` compares the measurements with `qa/thresholds.json` and fails on any regression. The residual counts there are what's left to fix, mostly at the shoes and the briefs' rim.
- **Baseline re-rendered** from the fixed asset; the sheets of the original are in [baseline/pre-fix/](baseline/pre-fix/), and a new fit sheet shows each garment, the layers and the poses.
- **Merged:** #401, then #402 (main merged into it first). #400 has main merged into it rather than rebased, so its branch needed no force-push. The conflicts were in the humanoid README and the sockets list, and one test followed CS1's renamed hair part. #400 is pushed and mergeable; merging it is left to Paul.
- **Full suite on #400 merged with main:** fmt, clippy (`-D warnings`) and doc tests clean; nextest 4789 passed, 9 failed, none from the character work. Three were timing tests that passed when run again. Six editor tests (MCP, Collaborate, AutoRecover, team history) fail on main on Windows. Their fixes are on the separate `fix-windows-editor-tests` branch: `\\?\` paths, Git's CRLF checkouts, two tests that needed the big-stack thread their neighbours use, and example copies that took a local `.axcenity` history.

## 2026-11-14: Stage 1

- **Agents:** the lead read the code, the reference library and the existing plans itself; five specialists ran in parallel: hair research, body/face/skin/cloth research, formats and libraries, a render-pipeline audit (read-only) and a geometry and render baseline (Blender). The lead checked every claim a decision rests on against the source, the vendored code, the cargo registry or the repositories' declared licences (marked ✔ in the research).
- **Files:** this folder (research, audit, plan, progress, the researchers' notes in `notes/`, the baseline); `assets/characters/humanoid/qa/` (measurement and render scripts); a row in `docs/README.md`; a pointer in `docs/research/character-studio.md`; a stale line fixed in `docs/research/platform-expansion.md` (rapier has soft bodies since 0.36.0); the wiki's roadmap, documentation history and home page.
- **Tests:** none run beyond the scripts' measurements (no engine code changed). `python scripts/check-wiki.py`: 0 errors. Relative links in the new docs checked.
- **Dependencies added:** none.
- **Findings that need action before the overhaul:** a hole at the waist of CS1's shipped default (unmerged #401); three open character PRs to merge in order.
