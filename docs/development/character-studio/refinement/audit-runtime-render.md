# Stage 0 audit: Character Runtime and character rendering

Audit of the refinement [brief](brief.md) sections 6.1, 6.2, 6.3, 10, 16 and principle 8 ("Runtime consistency"), and of plan milestones CO7, CO8, CO13, CO17 and CO18, against the source on `main` at `99f5b181`. Read only: nothing was built or run. Source and tests are the evidence; where a document disagrees with them it is listed under "Defects found". A second reader then checked every row against the source; what changed is listed under "Verification" at the end.

## Summary

- **The Character Runtime is real and shared.** One applier (`gameplay::description::apply_description`) is used by games, by the Lua API, by the visual QA capture and by the Character Studio preview. Identity is baked into derived models named by a hash, and live shapes (44 expressions, 6 correctives) stay per frame.
- **Character rendering has not started.** The renderer has three shading models (Standard, Unlit, Toon). Skin, eye, cloth and hair draw as Standard. The contract has 27 material values across its five models (19 distinct names); three names are drawn: `tint`, `iris_tint`, `roughness_scale`. None of CO7 is built; two of its items are partly covered by older work (ray traced cut-out shadows, per-pass GPU timers).
- **Morphs and skinning still run in every pass** (scene, prepass, shadow views, motion vectors). A compute pass poses skins for ray tracing only, joints only. CO8 is not built.
- **The realistic body's brows, lashes, cornea, eye occlusion film and tear line are blended (see-through) materials, not cut-outs.** They cast no shadow, are left out of the prepass and are sorted by object origin (defect D12).
- **Skin, makeup and markings: nothing beyond stored numbers.** No layer, mask, palette, decal or makeup code exists. The description format keeps unknown fields, so layers can be added without a version change. The contract already has two hooks nothing uses: a `skin` category for manifest groups, and parameters that drive a material's number values (`material_values`). No shipped manifest has such a parameter and the workspace has no Skin page.
- **Expressions and clips exist in the engine but not in the studio.** 15 built-in expressions (all eight the brief lists), 48 face controls, 26 animation states. The studio preview only idles. A character made by `ApplyDescription` gets no `FaceControls`, so it cannot show an expression until one is added by hand.
- **Performance is all but unmeasured for described characters.** The one figure on record is informal: applying a description to the realistic preview, the body's bake included, takes about 6 ms in a debug build (`docs/development/character-studio/progress.md:37`). No hardware, method or bench goes with it, and the wiki still says the bake is unmeasured. Every step of a parameter that drives a shape or a bone re-bakes every part of the character (throttled to one per 120 ms while dragging). A colour change does not touch meshes.
- **Runtime consistency is held by one test that compares mesh IDs**, not material values or pixels. Three things can make a studio character differ from the game: library copies that never refresh, the Game Editor's edit mode not applying descriptions, and values the renderer ignores.

Sizes: S is up to a day, M a few days, L one to two weeks, XL more. Status follows the brief's rule: *implemented* means a user can do it today and a test or file proves it; *partial* means some of it exists; *missing* means nothing in the code (searched under at least three names, see "Search log").

## Requirements

### Plan milestones (true state)

| ID | Requirement | Plan | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-CO7-01 | Packed occlusion-roughness-metalness texture, freeing two material slots | CO7 | missing | `crates/render/src/shader.wgsl:51-53` (three bindings); `crates/render/src/renderer/lighting.rs:1122-1124` | Channel masks already let one image serve all three (`lighting.rs:1208-1212`), but the bindings are still separate | M |
| RR-CO7-02 | Skin shading model (pre-integrated diffusion, two specular lobes, detail normal) | CO7 | missing | `crates/render/src/renderer/types.rs:527-529`; `crates/scene/src/material.rs:151-152` | Values are in the contract (`crates/avatar/src/material.rs:137-178`); no shader path; `scene` cannot tell which material is skin | L |
| RR-CO7-03 | Eye shading model | CO7 | missing | `types.rs:527-529`; `crates/scene/src/material.rs:153` (`iris_tint` as a base colour multiply) | No cornea lobe, parallax, limbal ring or pupil; `sclera_tint`, `pupil_size`, `limbal_ring`, `iris_depth` stored only | L |
| RR-CO7-04 | Cloth shading model (sheen, anisotropy) | CO7 | missing | `types.rs:527-529`; no `sheen` or `anisotrop` in `crates/render/src` | Whole model | M |
| RR-CO7-05 | Hair shading model | CO7 | missing | `types.rs:527-529` | Whole model; brows, lashes and stylized hair draw as Standard | M |
| RR-CO7-06 | glTF material extensions read (sheen, specular, clear coat, anisotropy, variants) | CO7 | missing | `Cargo.toml:100` (the `gltf` crate is built with `KHR_texture_transform` and `extras` only); `crates/assets/src/model.rs:1781` | Every other extension. The shipped `RealisticParts.glb` already writes `KHR_materials_sheen` on its `Underwear` and `Waistband` materials (its `extensionsUsed`); the loader drops it | M |
| RR-CO7-07 | MikkTSpace tangents | CO7 | missing | `crates/assets/src/model.rs:2063-2067` (area-weighted averages) | Baked normal maps are read in a different tangent frame than the baker's; identity shapes do not move tangents (`crates/assets/src/model/identity.rs:394`) | M |
| RR-CO7-08 | Hashed alpha with per-frame noise under TAA | CO7 | missing | `crates/render/src/lighting.wgsl:236-238` (noise by pixel, no frame); `shader.wgsl:490` (hard cut); `shader.wgsl:786-791` (`dithered_out` is an item's LOD fade, not a texture's alpha); `crates/render/src/renderer/gpu.rs:361` (alpha to coverage only for the depth fill under MSAA) | Whole feature. Until it exists the realistic brows and lashes are blended instead (D12) | M |
| RR-CO7-09 | Cut-out-aware shadows | CO7 | partial | Ray traced: `crates/render/src/renderer/ray_cut_outs.rs:1-22`. Shadow maps: `crates/render/src/renderer/gpu.rs:625` (`fragment: None`) | Shadow-map shadows of cut-out cards are solid slabs. No shipped character part is a cut-out today: the realistic brows and lashes are blended and cast nothing (D12) | M |
| RR-CO7-10 | GPU timings for each model | CO7 | partial | `crates/render/src/renderer/gpu_timer.rs` (per-pass timers) | No character model exists to time; nothing recorded | S |
| RR-CO8-01 | Morph and skin once per frame into a posed buffer used by every pass | CO8 | missing | `shader.wgsl:273` (`vs_main`), `:358` (`vs_shadow`), `:903` (`vs_motion`) each call `pose_corner`; `shader.wgsl:121-144` walks every delta of a corner, zero weights too | Whole pass | L |
| RR-CO8-02 | Ray tracing sees morphs | CO8 | partial | `crates/render/src/ray_skin.wgsl:1-4` (compute posing for a per-item BLAS); `crates/render/src/renderer/ray_tracing.rs:456-457` (joints only) | Expressions and correctives are not in ray traced shadows, AO or GI | M |
| RR-CO8-03 | Done-when measurement: 20 characters with 150 shapes | CO8 | missing | `docs/rendering/performance.md:28-29` (existing benches have no morph load) | A bench scene with described realistic characters | S |
| RR-CO13-01 | Character presets as data | CO13 | partial | `crates/editor/src/character_workspace.rs:102-190` (six presets as Rust constants). Description files as library data do exist, but as test cases only: `assets/characters/benchmarks/*.character.json`, held sound by `crates/avatar/tests/shipped_library.rs:229` | Not files; a game or a pack cannot add one | S |
| RR-CO13-02 | Outfit presets as data | CO13 | missing | no `outfit` type in `crates/avatar/src` or `crates/editor/src/character_workspace*` | Whole feature | M |
| RR-CO13-03 | Save and load | CO13 | implemented | `crates/editor/src/character_workspace/tests.rs:274`; `crates/avatar/src/tests/formats.rs:48` | None | - |
| RR-CO13-04 | Placing a character in a project, copying its items | CO13 | partial | `crates/editor/src/character_workspace/packs.rs:289-316`; `crates/project/src/character_library.rs:98` | Copies every shipped package, not what the character wears; writes a description file, places no Instance; a game script cannot load a description by path, it must set `Data` to the file's text (`wiki/Roadmap/Known-Limitations.md:47`); edit mode does not apply it (RR-P8-04) | M |
| RR-CO13-05 | Several characters in one scene | CO13 | partial | `crates/gameplay/src/description.rs:561-610` (every kept description is applied); `crates/assets/src/derived.rs:140-159` (same identity shares one model) | No test with several described characters at once; no measurement | S |
| RR-CO13-06 | Description replicated over the network | CO13 | partial | `crates/core/src/class.rs:3013-3019` (`Data` is an ordinary property); `crates/gameplay/src/description_system.rs:56-71` (stepped on both sides) | No network test: no reference to descriptions in `crates/net` or `crates/server` | M |
| RR-CO13-07 | Export of a character as a package | CO13 | partial | `crates/editor/src/character_workspace/packs.rs:8-14` | The exported pack holds descriptions only, not the models and manifests of what they wear | M |
| RR-CO17-01 | Screen-space subsurface scattering | CO17 | missing | `crates/render/src/renderer/scene.rs:3185` (one colour target, no diffuse-only buffer) | Whole feature; needs a second target or a stencil-marked blur | L |
| RR-CO17-02 | Freckles, moles, scars, wrinkles | CO17 | missing | see RR-6.1-06 to 09 | Whole feature | L |
| RR-CO17-03 | Makeup layers | CO17 | missing | `crates/avatar/src/tests/formats.rs:59-69` (an unknown `makeup` field only survives a round trip) | Whole feature | L |
| RR-CO17-04 | Wrinkle maps by expression | CO17 | missing | no `wrinkle` map code; `crates/core/src/face.rs:219` is a control's description only | Whole feature | L |
| RR-CO18-01 | Per-character budgets measured | CO18 | missing | `wiki/Engine-Systems/Characters.md:92` ("Unverified"); plan section 7, `plans/axcenity-engine-character-studio.md:187` ("All Expected, not Measured") | No budget is measured. The only number is the informal 6 ms apply time (RR-16-11) | M |
| RR-CO18-02 | Merging a finished character's parts for games | CO18 | missing | no merge of character parts in `crates/avatar`, `crates/assets`, `crates/gameplay` | Whole feature | L |
| RR-CO18-03 | Character LOD sets switched together | CO18 | missing | `crates/render/src/renderer/lod.rs:6-8` (level kept per item) | Parts switch one by one; skin shows through simplified garments (`wiki/Roadmap/Known-Limitations.md:52`) | M |
| RR-CO18-04 | Crowd test | CO18 | partial | `docs/benchmarks.md:16`; `docs/rendering/performance.md:29` | Crowd scenes use other characters; none with described realistic characters, faces or hair | M |

### Brief 6.1 Skin appearance

Two hooks in the contract matter for every row below, and neither is used yet. A manifest group can be in the `skin` category (`crates/avatar/src/manifest.rs:96-103`), and a parameter can drive a number value of a material (`material_values`, `manifest.rs:132`, `:212`; resolved in `crates/avatar/src/resolve.rs:352-360`; test `crates/avatar/src/tests/resolving.rs:167`; `architecture.md` section 6). A value set in `description.materials` wins over a driven one (`resolve.rs:386-387`). So a skin slider can be manifest data, with no new description field. What is missing is a parameter in a shipped manifest (neither base manifest has `material_values`), a Skin page in the workspace (`crates/editor/src/character_workspace.rs:247-255` maps Body, Face, Hair and Clothing only) and a renderer that draws the driven values. Drivers give numbers, not colours, so `tint` cannot be driven.

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-6-00 | A dedicated appearance category built on the existing material system (section 6's opening line) | 6 | partial | The contract's `skin` category and material drivers (above); material values reach the renderer through one function (`crates/scene/src/material.rs:149`) | No Skin page in the workspace; no skin parameter in a shipped manifest; no colour control of any kind in the workspace | M |
| RR-6.1-01 | Skin tone | 6.1 | partial | `crates/scene/src/material.rs:153-158` (`Skin.tint` multiplies the albedo); test `crates/player/src/default_humanoid_tests/description.rs:480`; `crates/avatar/src/material.rs:142-148` (`melanin`, stored only) | Settable only from Lua or the file; no studio control (`crates/editor/src/character_workspace.rs:226-234` has no skin category); `melanin` not drawn, so a `material_values` parameter on it would show nothing yet | M |
| RR-6.1-02 | Skin undertone | 6.1 | partial | `crates/avatar/src/material.rs:156`; checked by `crates/gameplay/src/description_tests.rs:305` | Stored and replicated, never drawn (`crates/scene/src/material.rs:151-152`) | M |
| RR-6.1-03 | Skin roughness | 6.1 | partial | `crates/scene/src/material.rs:159-163` (`roughness_scale` drawn) | No studio control. The nearest to done: a manifest parameter driving `Skin.roughness_scale` plus the Skin page would work with today's renderer | S |
| RR-6.1-04 | Skin variation | 6.1 | missing | `assets/characters/realistic/skin.py:15-18` (mottling is baked into the albedo, fixed) | No value, no mask, no control | M |
| RR-6.1-05 | Subsurface scattering controls | 6.1 | partial | `crates/avatar/src/material.rs:171-177` (`scatter_scale`, stored only) | No scattering in the renderer (RR-CO7-02) | L |
| RR-6.1-06 | Freckles | 6.1 | missing | only a test fixture's shape name (`crates/anim/src/face_tests.rs:45`) and a round-trip fixture (`crates/avatar/src/tests/formats.rs:41-44`) | Whole feature | M |
| RR-6.1-07 | Moles | 6.1 | missing | no hit for `mole` | Whole feature | M |
| RR-6.1-08 | Blemishes | 6.1 | missing | no hit for `blemish`, `acne`, `spot` | Whole feature | M |
| RR-6.1-09 | Scars | 6.1 | missing | no hit for `scar` | Whole feature | M |
| RR-6.1-10 | Skin detail intensity | 6.1 | partial | `crates/avatar/src/material.rs:164-170` (`detail_strength`, stored only); the normal map already has a strength in the material (`crates/render/src/renderer/lighting.rs:1227`) | Not drawn. Driving the normal map's strength is a small change but scales sculpt detail too; pores alone need a detail normal | S to M |
| RR-6.1-11 | Designed palette plus a detailed picker | 6.1 | missing | no palette or swatch data in `crates/avatar`, the manifests or the workspace. A parameter's `choices` (`crates/avatar/src/manifest.rs:137-151`) are the nearest thing, but a choice holds one number and items, not a set of tone values | Palette data (in the base manifest) and both controls; the workspace has no colour picker at all | M |
| RR-6.1-12 | Not a single RGB colour | 6.1 | partial | `crates/avatar/src/material.rs:137-178` (melanin, redness, undertone, tint) | In effect it is a single RGB multiply today: only `tint` is drawn | M |
| RR-6.1-13 | Material layering; consistent under different lighting | 6.1 | partial | Lighting: skin takes the scene's lights as Standard; the workspace's Stage page switches four lightings (`crates/editor/src/character_workspace/stage.rs:126-141`; `ui.rs:566-574`); the capture draws every character under fixed light presets (`crates/player/src/character_capture.rs:9-15`) | No layering of any kind; no benchmark character sets a material value, so no capture shows a skin tone under each light | L |

### Brief 6.2 Makeup

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-6.2-01 | Foundation | 6.2 | missing | no makeup code; `crates/avatar/src/tests/formats.rs:59-69` only proves unknown fields survive | Layer, mask, values, drawing | M |
| RR-6.2-02 | Blush | 6.2 | missing | as RR-6.2-01; a cheek flush is baked into the albedo (`assets/characters/realistic/skin.py:15-16`) | As above | M |
| RR-6.2-03 | Eyeliner | 6.2 | missing | as RR-6.2-01 | As above; needs texel density at the lash line (about 11 texels per cm at 2K, `assets/characters/realistic/README.md:96`) | M |
| RR-6.2-04 | Eyeshadow | 6.2 | missing | as RR-6.2-01 | As above | M |
| RR-6.2-05 | Lip colour | 6.2 | missing | lips are painted into the albedo (`assets/characters/realistic/skin.py:15-17`) | A lip mask and a layer | M |
| RR-6.2-06 | Eyebrow colour | 6.2 | partial | `Brows` is a `hair` material (`assets/characters/realistic/Realistic.base.json`, `materials`); its `tint` is drawn (`crates/scene/src/material.rs:153-158`) | Lua or file only; `melanin` and `redness` not drawn; no test on `Brows` | S |
| RR-6.2-07 | Each layer: colour | 6.2 | missing | no layer type | Schema and drawing | M |
| RR-6.2-08 | Each layer: opacity | 6.2 | missing | no layer type | As above | S |
| RR-6.2-09 | Each layer: roughness where appropriate | 6.2 | missing | no layer type | Needs a roughness change carried per texel | M |
| RR-6.2-10 | Each layer: presets | 6.2 | missing | no layer type | Preset data in the library | S |
| RR-6.2-11 | Each layer: independent enable and disable | 6.2 | missing | no layer type | A flag per layer | S |

### Brief 6.3 Tattoos and markings

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-6.3-01 | Tattoos | 6.3 | missing | no hit for `tattoo` | Whole feature | L |
| RR-6.3-02 | Birthmarks | 6.3 | missing | no hit for `birthmark` | Whole feature | M |
| RR-6.3-03 | Scars | 6.3 | missing | no hit for `scar` | Whole feature; a raised scar also needs a normal change | M |
| RR-6.3-04 | Decorative markings | 6.3 | missing | no character marking code | Whole feature | M |
| RR-6.3-05 | Custom decals | 6.3 | missing | The engine's decals are a square on a part's box face or a world-space projection through the depth buffer that skips characters by default (`crates/scene/src/decals.rs:1-4`; `docs/rendering/decals.md`) | Neither follows a deforming body; character decals must be in UV space | L |
| RR-6.3-06 | Placement, rotation, scale, opacity, material blending | 6.3 | missing | no code | A placement in the body's UV space per decal, and blend modes | M |
| RR-6.3-07 | Architecture first, no decal authoring tool | 6.3 | missing | The only hook: unknown description fields are kept (`crates/avatar/src/description.rs:36-39`; `architecture.md` section 5, last table row) | The schema, where masks and decal images live, how they are composed and cached (see "Recommendation") | M |

### Brief 10 Expressions and animation preview

"Runtime" below means the engine can do it and tests prove it. "Studio" means the Character Studio workspace. The workspace has no expression or animation control (`crates/editor/src/character_workspace.rs:226-234`), and the preview character has no `FaceControls` (defect D3). The QA capture already shows that a described character can be given a pose, a clip or an expression from outside: `AXCENITY_CAPTURE_POSE` (`rest`, `idle`, or a clip at a time such as `Cheer@0.6`) and `AXCENITY_CAPTURE_FACE` (`crates/player/src/character_capture.rs:33-38`, `:307-325`). The workspace can do the same in its own preview World.

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-10-01 | Expression: Neutral | 10 | partial | `crates/anim/src/face_play.rs:24-40`; `crates/anim/faces/expressions/Neutral.expression.json`; test `crates/anim/src/face_tests.rs:661`; the Game Editor's face tab previews it (`crates/editor/src/character_studio/expressions.rs:1-3`) | Not in the studio | S |
| RR-10-02 | Expression: Happy | 10 | partial | as RR-10-01 (`Happy.expression.json`); the capture can draw it (`crates/player/src/character_capture.rs:35-38`) | Not in the studio | S |
| RR-10-03 | Expression: Sad | 10 | partial | as RR-10-01 (`Sad.expression.json`) | Not in the studio | S |
| RR-10-04 | Expression: Angry | 10 | partial | as RR-10-01 (`Angry.expression.json`) | Not in the studio | S |
| RR-10-05 | Expression: Surprised | 10 | partial | as RR-10-01 (`Surprised.expression.json`) | Not in the studio | S |
| RR-10-06 | Expression: Disgusted | 10 | partial | as RR-10-01 (`Disgusted.expression.json`) | Not in the studio | S |
| RR-10-07 | Expression: Fearful | 10 | partial | as RR-10-01 (`Fearful.expression.json`) | Not in the studio | S |
| RR-10-08 | Expression: Confused | 10 | partial | as RR-10-01 (`Confused.expression.json`) | Not in the studio | S |
| RR-10-09 | Expression: Custom | 10 | partial | Project expressions are `*.expression.json` files, edited and saved in the face tab (`crates/editor/src/character_studio.rs:10-12`) | Not in the studio; the workspace has no project folder for them | M |
| RR-10-10 | Control: smile intensity | 10 | partial | `crates/core/src/face.rs:124` (`Mouth_Smile_Left`, `_Right`); sliders in the face tab (`crates/editor/src/character_studio/rig.rs:225`) | Not in the studio | S |
| RR-10-11 | Control: brow movement | 10 | partial | `crates/core/src/face.rs:94`, `:104`, `:114` (raise, lower, inward, each side) | Not in the studio | S |
| RR-10-12 | Control: eye squint | 10 | partial | `crates/core/src/face.rs:69` | Not in the studio | S |
| RR-10-13 | Control: mouth opening | 10 | partial | `crates/core/src/face.rs:144` (`Mouth_Open`) | Not in the studio | S |
| RR-10-14 | Control: jaw movement | 10 | partial | `crates/core/src/face.rs:227` (`Jaw_Open`, forward, left, right); jaw bone turn tested on the realistic body (`crates/anim/src/face_tests.rs:362`) | Not in the studio | S |
| RR-10-15 | Control: blink intensity | 10 | partial | `crates/core/src/face.rs:50`; full closure tested on the realistic body (`crates/anim/src/face_tests.rs:409`) | Not in the studio | S |
| RR-10-16 | Animation: idle | 10 | partial | The preview idles (`crates/editor/src/character_workspace.rs:732-747`); the clip keeps the realistic body's proportions (`crates/anim/src/retarget_tests.rs:1064`) | Idle is the only thing shown and it is not a choice: it cannot be paused, left or returned to. No workspace test asserts that the preview moves or which state plays | S |
| RR-10-17 | Animation: walking | 10 | partial | State `Walk` (`crates/project/src/default_humanoid.rs:143`); plays on the realistic body (`crates/anim/src/retarget_tests.rs:1003`) | No clip picker in the studio | S |
| RR-10-18 | Animation: running | 10 | partial | State `Run` (`crates/project/src/default_humanoid.rs:144`) | No clip picker in the studio | S |
| RR-10-19 | Animation: standing | 10 | partial | The capture's rest stance clears the state machine (`crates/player/src/character_capture.rs:307-313`) | That is the 45° A-pose; there is no still standing pose and no studio control | S |
| RR-10-20 | Animation: basic gestures | 10 | partial | States `Wave`, `Point`, `Cheer`, `Clap`, `Shrug` (`crates/project/src/default_humanoid.rs:160-167`) | No clip picker in the studio | S |
| RR-10-21 | Preview only unless saved on purpose | 10 | partial | A description has no expression or clip field (`crates/avatar/src/description.rs:20-40`); the preview's animators live in the workspace, not the file | Nothing to preview yet in the studio. In the face tab the 48 control sliders edit the Instance as undo steps (`crates/editor/src/character_studio.rs:5-8`), so they are not preview only | S |
| RR-10-22 | Identity independent of the current expression | 10 | implemented | Identity shapes are baked and dropped, expression shapes stay live (`crates/avatar/tests/realistic_parameters.rs:228`; `crates/assets/src/model/identity.rs:281-302`) | None | - |

### Brief 16 Performance

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-16-01 | Incremental geometry updates | 16 | partial | An item change keeps the other parts (`crates/gameplay/src/description.rs:324-379`). A parameter change gives every part a new derived model (`description.rs:250-251`), each a full copy of positions, normals, tangents and live shapes (`crates/assets/src/model/identity.rs:373-394`) | One identity key for the whole character: moving the nose re-bakes the underwear and the teeth. A parameter that drives only attributes or material values adds no identity layer (`crates/avatar/src/resolve.rs:346-348`), so it re-bakes nothing | M |
| RR-16-02 | Efficient shape blending | 16 | partial | Identity is baked once (`identity.rs:274`); live shapes are sparse deltas on the GPU (`crates/render/src/renderer/skinning.rs:12-18`) | Evaluated again in every pass (RR-CO8-01) | L |
| RR-16-03 | Cached deformation data | 16 | partial | Derived models cached by ID, 96 kept, shared by equal identities (`crates/assets/src/derived.rs:16`, `:54-81`); follower shape maps cached (`crates/anim/src/animators.rs:661-665`) | Bake cost measured once, informally (RR-16-11); no on-disk cache (`wiki/Roadmap/Known-Limitations.md:53`); identities kept for ever (defect D4) | M |
| RR-16-04 | Avoid unnecessary mesh rebuilds | 16 | partial | Applying the same description changes nothing (`description.rs:255-256`, `:361`); while dragging the preview applies at most every 120 ms (`crates/editor/src/character_workspace.rs:68`, `:671-697`) | See RR-16-01; the body's bake blocks the caller (`description.rs:257-264`) | M |
| RR-16-05 | Efficient material updates | 16 | partial | A value is an attribute; the tint and roughness reach the renderer as per-item values, keeping maps and mesh (`crates/scene/src/draw_list.rs:685-712`; test `crates/player/src/default_humanoid_tests/description.rs:480`) | The change still runs the whole applier; no test pins "a colour change makes no new model"; no studio control | S |
| RR-16-06 | Asynchronous thumbnail loading | 16 | partial | Character and preset pictures are drawn on a thread and read at most four a frame (`crates/editor/src/character_workspace/thumbnails.rs:14-17`, `:39-40`; test `shelf_tests.rs:299`) | No item thumbnails exist: none of the 15 shipped items names one | M |
| RR-16-07 | Efficient wardrobe browsing | 16 | partial | The catalogue indexes items so a browser need not open manifests (`architecture.md` section 15); the library is cached by the catalogue's time stamp (`crates/gameplay/src/description.rs:70-93`) | `Library::read` still parses every manifest (`crates/avatar/src/library.rs:64-104`); fine at 15 items, unmeasured at hundreds | S |
| RR-16-08 | Appropriate LOD behaviour | 16 | partial | The body and teeth have two authored simpler copies that follow identity (`crates/avatar/tests/realistic_parameters.rs:115`); faces have their own LOD (`crates/anim/src/face_tests.rs:865`) | Parts switch one by one (RR-CO18-03) | M |
| RR-16-09 | Efficient viewport selection and picking | 16 | missing | A MeshPart is picked by its box (`crates/scene/src/pick.rs:187-188`); no posed-triangle pick; the region map is data only and the loader reads one UV set (`crates/assets/src/model.rs:1617`); the workspace viewport only orbits. Parts to build from: CPU posing of a primitive (`crates/assets/src/model/rig.rs:417`), a ray and triangle test in Model Studio (`crates/editor-model-studio/src/view.rs:789`), the manifest's `regions` (`crates/avatar/src/manifest.rs:257-262`) | A pick against the posed, morphed character that returns a triangle and its region | L |
| RR-16-10 | A colour change must not rebuild the whole character | 16 | partial | By reading: material values are not part of the identity key (`crates/assets/src/model/identity.rs:99-118`), so mesh IDs stay | True today; not pinned by a test (RR-16-05) | S |
| RR-16-11 | Measure with representative characters and complex combinations | 16 | partial | One informal figure: applying a description to the realistic preview, the body's bake included, about 6 ms in a debug build (`docs/development/character-studio/progress.md:37`; `docs/editor/character-studio.md:12`). Representative characters exist as data: the benchmark descriptions in `assets/characters/benchmarks/` (`crates/avatar/tests/shipped_library.rs:229`) | No hardware, method or repeatable bench behind the figure; it covers the body's bake only (the other parts bake when first drawn); nothing for upload, frame cost, a release build or combinations; the wiki still says unmeasured (`wiki/Engine-Systems/Characters.md:92`, `Character-Data.md:51`) | M |
| RR-16-12 | Budgets from measured results | 16 | missing | Plan section 7: every budget is "Expected, not Measured" | Follows RR-16-11 | S |

### Principle 8 Runtime consistency

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| RR-P8-01 | The same when saved | 1 (principle 8) | implemented | `crates/editor/src/character_workspace/tests.rs:274`; `crates/avatar/src/tests/formats.rs:48` | None | - |
| RR-P8-02 | The same when reopened | 1 (principle 8) | implemented | `crates/editor/src/character_workspace/tests.rs:341-351` | Equality of the description, not of the picture | - |
| RR-P8-03 | The same when used in a game | 1 (principle 8) | partial | The preview calls the game's applier (`crates/editor/src/character_workspace.rs:683`); `tests.rs:473-505` holds preview and game to the same mesh IDs | The test does not compare material value attributes or pixels | S |
| RR-P8-04 | The same in the Game Editor before Play | 1 (principle 8) | partial | What an earlier apply left in the saved World is still drawn in edit mode: the worn parts are Instances, and material values are attributes read by `scene` in every host (`crates/scene/src/draw_list.rs:685-712`). The shape is not: descriptions are applied only by the game systems (`crates/gameplay/src/lib.rs:68`; the editor's only caller is the workspace), and a derived `MeshId` whose identity this run has not registered falls back to the base model (`crates/assets/src/derived.rs:62-64`) | A placed character shows its base, unshaped meshes in edit mode (`wiki/Roadmap/Known-Limitations.md:48`); a description file added to a project has never been applied, so it shows nothing at all | M |
| RR-P8-05 | The same on every machine | 1 (principle 8) | partial | The identity key is a fixed hash of names and numbers (`crates/assets/src/model/identity.rs:99-118`); a loaded world applies kept descriptions (`crates/player/src/default_humanoid_tests/description.rs:575`) | No client and server test; library copies can differ (defect D1) | M |
| RR-P8-06 | A picture-level check of studio against game | 1 (principle 8) | missing | The capture (`crates/player/src/character_capture.rs`) and the studio's preview test (`crates/editor/src/character_workspace/tests.rs:513`) each draw; nothing compares them | One description drawn by both under the same light and camera, compared | M |

## What the material system can carry today

| Limit | Evidence | Consequence |
|---|---|---|
| The item shader samples 16 textures, the default device limit: 8 per frame, 8 per material | `crates/render/src/shader.wgsl:47-56`; `crates/render/src/lighting.wgsl:164-193`; `docs/rendering/stylized.md:32` | No new texture binding without freeing or reusing one |
| The realistic `Skin` material uses 3 images on 5 of the 8 material bindings (albedo, normal, one packed occlusion-roughness image bound up to three times) | `assets/characters/realistic/skin.py:1-27`; `crates/assets/src/model.rs:1766-1769` | The emissive, opacity and specular bindings are a white pixel on skin: **three free slots today**, five after ORM packing |
| A binding can be reused for another kind of image | The toon ramp takes the specular binding (`crates/render/src/renderer/lighting.rs:1127-1131`) | Precedent for binding skin layers in unused slots |
| All 16 vertex attributes are used; the loader reads one UV set | `wiki/Engine-Systems/Rendering.md:109`; `crates/assets/src/model.rs:1617` | Masks and decals must use the body's first UV set |
| The material uniform has 8 free `vec4` (used only by custom surface shaders and water) and spare components | `crates/render/src/renderer/types.rs:494-524`; `lighting.rs:1137-1168` | About 32 numbers for a skin model without growing the uniform |
| Base colour and roughness are per item; everything else is per material bind group | `crates/render/src/renderer/lighting.rs:1184-1270` | A tint is free; tone uniforms cost one bind group per distinct skin |
| A custom `surface` function cannot change lighting | `crates/render/src/surface.wgsl`; `audit.md` section 3.10 | Layers that change colour, roughness or normal could live there; scattering cannot |
| Mips are made on the CPU at upload; a composed image is plain RGBA8 | `crates/render/src/renderer/gpu.rs:1057`, `:1216` | A composed 2K image is about 21 MB on the GPU against about 5 MB for a BC7 map (only a cooked texture goes up as BC7, `gpu.rs:861-885`); time to compose and upload is unmeasured |
| One colour target, no diffuse-only buffer | `crates/render/src/renderer/scene.rs:3185` | Screen-space scattering needs renderer work; pre-integrated skin does not |
| `scene` does not know a material's shading model | `scene` does not depend on `avatar`; the applier writes values, not the model (`crates/gameplay/src/description.rs:497-534`) | A contract decision is needed before CO7 (see "Shared contracts") |
| A manifest parameter can drive a material's number values | `crates/avatar/src/manifest.rs:132`, `:212`; `crates/avatar/src/resolve.rs:352-360` | Sliders for melanin, redness, undertone, roughness, detail and scatter need no new description field. Colours cannot be driven |
| A blended (glTF `BLEND`) material is drawn see-through | `crates/scene/src/draw_list/materials.rs:240-245`; `crates/render/src/renderer/lighting.rs:840`; `docs/rendering/shadows.md:15` | It casts no shadow, is not in the prepass and is sorted by object origin. The realistic brows, lashes, cornea, eye occlusion film and tear line are such materials |
| Engine decals do not follow a deforming mesh | `docs/rendering/decals.md`; `crates/scene/src/decals.rs:1-4` | Not usable for tattoos |

**Can a layered skin be composed today?** No: there is no composition code on the CPU or in the shader. The renderer has room for it (three free slots on skin, 32 uniform numbers), and `assets` already has the pattern it needs (a derived asset named by a hash, built on first use, shared).

## Recommendation: how skin tone, details and makeup should be carried

Three carriers, chosen by how often a value changes and how many characters share it.

| Carrier | What goes in it | Where it lives | Cost of a change |
|---|---|---|---|
| **A. Material values of the Skin model** (exists) | Tone: `melanin`, `redness`, `undertone`, `tint`. Surface: `roughness_scale`, `detail_strength`, `scatter_scale`. Eyebrow colour on `Brows` | `description.materials`, then `Mat_…` attributes, then uniforms in the shader | None: a uniform |
| **B. A layer stack composed into overlay images** (new) | Makeup layers, freckles, moles, blemishes, scars, birthmarks, tattoos, markings, custom decals | A typed optional field of the description (layer ID from the library, colour, opacity, roughness, enabled, and for decals a placement in UV space). `avatar` resolves it to a plain list; `assets` composes it on the CPU into one overlay set **cached by a hash of the stack**, as derived models are; the applier writes one attribute naming it; `scene` binds it to the skin material's free slots | One recomposition of the overlay (not the albedo), at preview size while dragging |
| **C. A flattened bake for games** (later, CO18) | Tone and overlays baked into one compressed albedo and roughness map | A build step | Offline |

Details of B:

- Two images cover every layer kind: an **over** image (premultiplied colour and coverage: foundation, blush, eyeliner, eyeshadow, lip colour, decorative markings) and an **under** image (a multiply colour, with a roughness change in alpha: freckles, moles, birthmarks, tattoos, which should take the skin tone above them). The shader adds two samples and a few operations after toning.
- Masks are authored per base model in its first UV set and shipped in the library. `skin.py` already computes the regions it paints (cheeks, lips, nose, knuckles), so it can write them as masks.
- A character with no layers binds nothing and costs nothing. Fifty characters that differ only in tone share every texture.
- The palette is data in the base manifest: named tones, each a set of the three tone numbers. The advanced picker is the three sliders plus `tint`.

First step that needs no shader work: `scene::material::modulate` can turn `melanin`, `redness` and `undertone` into a multiply colour today (S). It tones lips and palms with the rest, so it is a stopgap until the Skin model has a real curve and a tone mask.

Notes added in verification:

- **Use the hooks the contract has before adding fields.** Carrier A's numbers can be exposed as manifest parameters with `material_values` in a `skin` group (see the note above the 6.1 table). The work is then manifest data from `manifests.py`, a Skin page in the workspace and the drawing. Two contract gaps remain: a driver cannot give a colour, and a palette entry (several numbers at once) has no form.
- **`modulate` matches by value name, not by shading model.** `melanin` and `redness` are values of both Skin and Hair (`crates/avatar/src/material.rs:142-155`, `:228-241`), so the stopgap would tone brows and lashes with the same curve unless it is given the material's model. That is the same contract question as "how `scene` learns a shading model".
- **B needs a skin branch in the shader.** Standard does not sample the reused slots as overlays, so layers cannot be drawn before RR-CO7-02 has at least a skin path. Until then the only way to show a layer is to compose the albedo on the CPU (the first alternative below), which could serve as a preview stopgap.
- **Eight channels, one roughness.** The two images hold an over colour with coverage (4) and an under multiply colour (3), leaving one channel. It must be the roughness change of the whole stack, makeup included (lip gloss, matte foundation: RR-6.2-09), worked out on the CPU, not a channel of the under layers only.
- **A raised scar needs a third image** (a normal or height change, RR-6.3-03). Skin has three reusable slots today, so two overlays plus one normal overlay use all of them and leave none for a detail normal until ORM packing (RR-CO7-01) lands.

Alternatives considered:

| Alternative | For | Against |
|---|---|---|
| Compose the whole albedo on the CPU, cached by description hash | No shader change, no slot, works with Standard today | Every tone change recomposes a 2K or 4K image and its mips; every distinct tone is its own uncompressed albedo (about 21 MB at 2K), so crowds stop sharing; slow feedback on the most used control |
| All layers in the shader, each with its own mask | Fully live, no composition, masks shared by every character | A mask per layer does not fit: 3 free slots give 12 packed masks and leave nothing for decals; 6 makeup layers already use about 30 of the 32 free uniform numbers; tattoos with free placement cannot be masks |
| A custom WGSL `surface` function for skin | Exists today, can read 8 parameters | Parameters come from material files, not from a description; cannot add scattering; a second mechanism beside the shading models |
| Engine `ProjectedDecal` for tattoos | Exists | World space, skips characters by default, slides on a deforming body |
| One RGB tint (today) | Free | The brief rules it out |

## Search log

Each "missing" was searched case-insensitively across `crates/` and `assets/characters/` (`.rs`, `.wgsl`, `.json`, `.py`):

| Feature | Names searched | Result |
|---|---|---|
| Skin details | `freckle`, `mole`, `blemish`, `acne`, `scar`, `pore`, `wrinkle`, `variation`, `mottle` | Only test fixtures and baked albedo code in `skin.py` |
| Makeup | `makeup`, `make_up`, `blush`, `eyeliner`, `eyeshadow`, `lipstick`, `lip_colo`, `foundation`, `brow_colo` | Only audio "makeup gain" and a round-trip fixture |
| Markings | `tattoo`, `birthmark`, `marking`, `decal`, `overlay`, `skin_layer` | Only the engine's world decals and a round-trip fixture |
| Palette | `palette`, `swatch`, `skin_tone` | Only the editor theme's palette |
| Shading models | `sheen`, `subsurface`, `sss`, `anisotrop`, `scatter`, `pre-integrat`, `kajiya`, `marschner`, `cornea`, `SHADING_` | Three models; `scatter` hits are fog and atmosphere |
| Alpha and shadows | `alpha_to_coverage`, `dither`, `hashed`, `cut_out`, `alpha_cutoff` | As in RR-CO7-08 and 09 |
| Compute posing | `ray_skin`, `posed buffer`, `morph`, `pose_corner` | Ray tracing only |
| Presets and outfits | `preset`, `outfit`, `pack` | Presets in code; packs of descriptions |
| Merging and LOD sets | `merge`, `lod_group`, `lod set`, `switched together` | Nothing for characters |
| Picking | `pick`, `ray_triangle`, `hit_triangle`, `region` | Box picking; a triangle pick exists only for Model Studio's edit mesh (`crates/editor-model-studio/src/view.rs:789-806`) |
| Added in verification: skin controls as data | `material_values`, `MaterialDriver`, `Category::Skin`, `tone` | The contract has them (`crates/avatar/src/manifest.rs`), a test fixture uses them (`crates/avatar/src/fixtures.rs:39-40`), no shipped manifest or workspace page does |
| Added in verification: alpha modes of shipped parts | `alphaMode`, `AlphaMode::Blend`, `extensionsUsed` (in the model files' JSON) | Realistic brows, lashes, cornea, eye occlusion and tear line are `BLEND`; no `MASK` material in either family; `KHR_materials_sheen` is written and not read |
| Added in verification: measurements | `ms`, `Measured`, `bench`, `benchmarks/` | One informal 6 ms figure; benchmark descriptions exist as data, none sets a material value |

## Defects found

| # | Severity | Defect | Evidence |
|---|---|---|---|
| D1 | medium | **A library copy is never refreshed.** `add_character_packages` skips every base model and item the target library already lists, so a rebuilt package never replaces the copy in a project or in the studio's own folder. The function's comment and the user document say files are replaced when their size changed. The studio and a game can then draw different meshes from one description | `crates/project/src/character_library.rs:119-121`, `:132-134` against `:88-90` and `docs/editor/character-studio.md:10` |
| D2 | medium | **The Game Editor's edit mode does not apply descriptions**, so a placed character shows base meshes until Play (known limitation; it breaks principle 8) | `crates/gameplay/src/lib.rs:68`; `wiki/Roadmap/Known-Limitations.md:48` |
| D3 | medium | **The manifest's `face` field has no consumer.** It is validated but the applier never reads it and never adds a `FaceControls`, so a character made by `ApplyDescription` has no expressions, blinks or lip sync until a creator adds the Instance by hand. The studio preview is such a character | `crates/avatar/src/manifest.rs:292`, `:399`; no reference in `crates/gameplay/src/description.rs`; `crates/anim/src/animators.rs:566` |
| D4 | low | **Registered identities are kept for ever.** Each slider step registers one; nothing removes them (derived models are trimmed, identities are not) | `crates/assets/src/derived.rs:37-41`, `:71-75` |
| D5 | low | **Every step of a parameter that drives a shape or a bone re-bakes every part**, including parts the parameter cannot change, because one key covers the whole identity | `crates/gameplay/src/description.rs:250-251`; `crates/assets/src/derived.rs:31-43` |
| D6 | low | **Most material values are accepted and silently ignored.** `SetMaterialValue("Skin", "melanin", …)` succeeds and changes nothing on screen; the scripting document does not say so (only the known limitations do) | `crates/scene/src/material.rs:151-163`; `docs/scripting/api-character.md:63`; `wiki/Roadmap/Known-Limitations.md:50` |
| D7 | low | **Material slot names differ between the two families** (`Brow` and `Lash` on stylized; `Brows` and `Lashes` on realistic, which also has `Cornea`, `EyeOcclusion`, `LipSeal`, `TearLine`, `Tongue`). The scripting document lists only the stylized names, and a script written for one body errors on the other | `assets/characters/humanoid/Humanoid.base.json` and `assets/characters/realistic/Realistic.base.json` (`materials`); `docs/scripting/api-character.md:63` |
| D8 | low | **Scripts cannot set a worn item's material values** and have no getter for any material value: `SetMaterialValue` writes the base model's materials only; to read one a script must parse the description's `Data` text itself | `crates/gameplay/src/description_system.rs:289-338`, `:602-628` |
| D9 | low | **The consistency test compares mesh IDs only**, so a regression in material values between preview and game would pass | `crates/editor/src/character_workspace/tests.rs:501-504` |
| D10 | low | **The brief's header points at a plan section that does not exist yet**: "its section 11 maps this brief's Stages 0 to 11 onto the plan's milestones"; the plan ends at section 10 (`:227`). Stage 0 is expected to add it | `docs/development/character-studio/refinement/brief.md:3`; `plans/axcenity-engine-character-studio.md` headings |
| D11 | low | **Shadow-map shadows of cut-out items are solid.** The shadow pipeline has no fragment stage, so a cut-out (glTF `MASK`) card casts a slab unless ray traced shadows are on. No shipped character part is a cut-out today (see D12); it will matter for card hair | `crates/render/src/renderer/gpu.rs:625`; `crates/render/src/renderer/ray_cut_outs.rs:1-22`; `docs/rendering/shadows.md:15` |
| D12 | low | **The realistic body's brows, lashes, cornea, eye occlusion film and tear line are blended materials.** `RealisticParts.glb` gives them `alphaMode: BLEND`; the loader keeps their opacity under 1 so they are drawn see-through. They cast no shadow, are left out of the prepass (so no ambient occlusion or motion vectors from them) and are sorted by object origin, which is one point for several nested parts of one head | the model's materials (read from the file's JSON); `crates/assets/src/model.rs:1814-1819`; `crates/scene/src/draw_list/materials.rs:240-245`; `crates/render/src/renderer/lighting.rs:840`; `crates/render/src/renderer/temporal.rs:846-849`; `docs/rendering/shadows.md:15` |
| D13 | low | **Documents disagree on whether the bake is measured.** The progress log and the user document give about 6 ms in a debug build; the wiki and the known limitations say it is not measured | `docs/development/character-studio/progress.md:37`; `docs/editor/character-studio.md:12`; `wiki/Engine-Systems/Characters.md:92`; `wiki/Engine-Systems/Character-Data.md:51`; `wiki/Roadmap/Known-Limitations.md:53` |
| D14 | low | **The contract's `skin` and `accessories` categories and its material drivers have no consumer.** No shipped manifest uses them and the workspace maps only Body, Face, Hair and Clothing, so a group in either category would never be shown | `crates/avatar/src/manifest.rs:96-103`, `:132`; `crates/editor/src/character_workspace.rs:247-255` |

## Shared contracts that would have to change (the lead owns them)

| Contract | Change | Why |
|---|---|---|
| Description, `architecture.md` section 5 (`crates/avatar/src/description.rs`) | A typed optional field for the skin layer stack (makeup, details, markings, decals) with validation. Optional, so no version bump (section 4) | Today such data only survives in `extra` with no meaning |
| Material values, section 10 (`crates/avatar/src/material.rs`) | Decide whether `variation` joins the Skin values; say which values each renderer stage draws | 18 of the 27 values are stored and ignored |
| How `scene` learns a material's shading model, section 10 | Either the applier writes it as an attribute, or the model file carries it in the glTF material's `extras` (as `axcenity_sway` does, `crates/assets/src/model.rs:353-365`) and the manifest check holds the two equal. The second works for a character nothing has applied yet | Nothing in the World says a material is skin; CO7 cannot start without it |
| Base manifest, section 6 | Optional `skin` data: the tone palette, the layer list (ID, label, mask file, blend, default colour, region), decal anchors. Also a colour kind of material driver, or a palette entry that sets several values at once (today's `material_values` drive numbers only, and a `choice` holds one number) | Palette and layers must be data, per base model. The `skin` category and number drivers already exist and need no change |
| Catalogue and item manifest, sections 8 and 15 | A way to ship layer content (a new item `kind`, or a `layers` list in the catalogue) | Tattoos and makeup presets are content, not code |
| `Mat_…` attributes (`crates/core/src/material_values.rs`) | One attribute that names the composed overlay set by its hash; a value kind beyond number and colour | The overlay must be saved, copied and replicated with the part like other values |
| Lua API, section 17 (`docs/scripting/api-character.md`) | `GetMaterialValue`; item material values; layer methods | D8; games need the same reach as the studio |
| Base manifest `face`, section 13 | State who creates the `FaceControls` (the applier, from `face.profile`) | D3 |
| Presets, section 18 | Preset files (a description without a name) and outfit presets as library data | RR-CO13-01, RR-CO13-02 |
| Material slot names across families | One naming rule (`Brows` or `Brow`) | D7 |

## File ownership notes for parallel work

| Files | Who should own them | Conflict to avoid |
|---|---|---|
| `crates/render/src/shader.wgsl`, `lighting.wgsl`, `renderer/lighting.rs`, `renderer/types.rs`, `renderer/gpu.rs` | One rendering owner | CO7 (shading models, bindings, the material uniform) and CO8 (`pose_corner` and every vertex entry point) both rewrite `shader.wgsl`: do them in sequence, not in parallel |
| `crates/scene/src/material.rs` (`modulate`), `crates/scene/src/draw_list.rs:685-712` | The rendering owner | Every skin, eye, cloth and hair value passes through these few lines |
| `crates/avatar/src/material.rs`, `description.rs`, `manifest.rs`, `item.rs` | The lead (contracts) | Specialists ask for fields; they do not add them |
| `crates/gameplay/src/description.rs`, `description_system.rs`, `description_tests.rs` | One runtime owner | The studio agent must not edit the applier to get a preview feature |
| `crates/assets/src/derived.rs`, `model/identity.rs` | The runtime owner | Put overlay composition in a new file beside them, not inside them |
| `crates/anim/src/animators.rs` | Shared and hot: the face, twist and corrective call sites are all in `skinned` (`:258-275`) | One owner at a time; `face*.rs`, `correctives.rs` and `twist.rs` can be split between agents |
| `crates/editor/src/character_workspace*` | The studio agent | It owns its preview World and animators, so it can add a `FaceControls` and pick a state there without touching runtime files |
| `crates/editor/src/character_studio*` (the face tab) | The face agent | Do not merge it with the workspace in passing |
| `assets/characters/realistic/skin.py`, `manifests.py` | The asset agent | Mask export and palette data come from here; generated files are never edited by hand |
| `crates/player/src/character_capture.rs` | The QA owner | Its cameras, lights and settings must not change; add views instead |
| `crates/project/src/character_library.rs` | The runtime owner | D1's fix changes what every project and the studio folder receive |

## Risks

| Risk | Why it matters | Mitigation |
|---|---|---|
| The albedo has one baked tone with flush, lips and mottling painted in | A curve over it turns dark tones muddy and tints lips with the skin | Re-bake the albedo as tone-neutral detail plus masks before judging any tone model |
| Composition and upload time is unknown | Overlay updates may stutter at 2K | Measure first; compose at 1K while dragging; reuse the 120 ms throttle |
| The texture limit is exhausted | Reusing the emissive, opacity and specular bindings on skin rules those maps out for imported skins | Land ORM packing (RR-CO7-01) before layers, or accept the rule for the `skin` model |
| Per-pass morph cost grows with expression preview | The studio will show expressions on a 30k-triangle body in every pass | CO8 before or with Stage 9; measure with RR-CO8-03 |
| Slider drags churn derived models | Each step copies and uploads every part; with ray tracing on, each is a new BLAS | Measure (RR-16-11); key the bake per model by the shapes and bones it has (D5) |
| No `FaceControls` on applied characters | The studio preview cannot show an expression or blink, and a game character has no face until a creator adds the Instance | The workspace can add one to its own preview World (as the capture does, `crates/player/src/character_capture.rs:314-325`) without a contract change; decide D3 in the contract for games |
| Edit mode and stale libraries | A user sees a different character in the Game Editor or after an engine update | Fix D1 and D2 before claiming principle 8 |
| Overlay keys and masks must match on every machine | A client without the mask file draws no makeup | Deterministic key like the identity's; masks ship in the project's library; test with a client and a server |
| Raster shadows of cut-out cards are solid | Card hair authored as cut-outs would cast slabs without ray tracing. Today's realistic lashes and brows are blended instead and cast nothing (D12) | RR-CO7-08 and RR-CO7-09 before card hair ships; then move brows and lashes to the same path |
| Values that do nothing | Users and scripts set `melanin` and see no change | Draw them (stopgap in `modulate`) or warn until CO7 |
| Tangents | Identity shapes and the loader's tangent frame both differ from the baker's | RR-CO7-07; check the face under raking light |
| Blended brows and lashes (D12) | They cast no shadow, write no depth and sort by one origin per part, so they can draw in the wrong order against the cornea and the eye occlusion film, and ghost under TAA | Check the eye region in the captures; move them to cut-outs once RR-CO7-08 and RR-CO7-09 exist |

## Verification

A second reader checked this audit against the source on the same commit, read only (nothing built or run). Checked: every "missing" row (searched again under other names and in other crates), every "implemented" row (the evidence line opened), at least one evidence line of each of the 48 "partial" rows, every defect, and the bullet count of brief sections 6.1 (13), 6.2 (11), 6.3 (7), 10 (22), 16 (12), principle 8 (3) and plan milestones CO7 (10), CO8 (3), CO13 (7), CO17 (4), CO18 (4). No bullet was absent. Every cited path and line that was opened says what the row claims, except as listed here.

| # | Correction | Why |
|---|---|---|
| V1 | Added the note above the 6.1 table, row RR-6-00, a row in "What the material system can carry today", defect D14, a recommendation note and a change to the base manifest contract row | The audit missed that the contract already has a `skin` category for groups and parameters that drive material numbers (`crates/avatar/src/manifest.rs:96-103`, `:132`, `:212`; `crates/avatar/src/resolve.rs:352-360`; test `crates/avatar/src/tests/resolving.rs:167`). Skin sliders are manifest data plus a workspace page, not a new description field |
| V2 | RR-16-11 from missing to partial; RR-16-03, RR-CO18-01 and the summary reworded; defect D13 added | "Never measured" was wrong: `docs/development/character-studio/progress.md:37` and `docs/editor/character-studio.md:12` record about 6 ms for an apply with the body's bake in a debug build. It is one informal figure, and the wiki contradicts it |
| V3 | RR-10-16 from implemented to partial | The row's own gap said it cannot be paused or changed; idle is not a choice in a preview system, and no workspace test asserts the preview moves or which state plays |
| V4 | RR-P8-04 from missing to partial | Parts and material value attributes left by an earlier apply are drawn in edit mode; only the baked shape is lost (`crates/assets/src/derived.rs:62-64`) |
| V5 | Defect D12 added; D11, RR-CO7-08, RR-CO7-09, a risk row and a summary bullet corrected | The audit said lashes and brows cast slab shadows. They do not: `RealisticParts.glb` marks them `BLEND`, so they are see-through items that cast nothing and are left out of the prepass (`crates/scene/src/draw_list/materials.rs:240-245`; `docs/rendering/shadows.md:15`). Slab shadows apply to cut-out (`MASK`) cards, of which no shipped character part is one |
| V6 | D11 added to the defects table | It was in the audit's structured result but not in this file |
| V7 | RR-CO7-06 evidence strengthened | `Cargo.toml:100` is the direct evidence (the `gltf` crate's features); the shipped realistic model already writes `KHR_materials_sheen`, which the loader drops |
| V8 | Recommendation: four more notes (the `modulate` stopgap matches by value name and would tone hair too; layers need a skin shader path first; one roughness channel for the whole stack; a raised scar needs a third image) | Limits the recommendation did not state |
| V9 | RR-16-01 and D5 narrowed to parameters that drive a shape or a bone | A parameter that drives only attributes or material values adds no identity layer (`crates/avatar/src/resolve.rs:346-348`) |
| V10 | RR-16-09 evidence extended with the parts a pick can be built from; status unchanged | CPU posing, a ray and triangle test and the manifest's regions exist; the pick itself does not |
| V11 | RR-CO13-01, RR-CO13-04, RR-6.1-13 and the section 10 preamble extended; statuses unchanged | Benchmark descriptions exist as data files; a script cannot load a description by path; the Stage page already switches four lightings; the capture already poses and animates a described character from outside |
| V12 | D8 reworded | A script can read material values by parsing the description's `Data` text; what is missing is a getter |
| V13 | D10 reworded | The plan's section 11 is expected from Stage 0; it does not exist yet |
| V14 | The "No `FaceControls`" risk reworded | "Stage 9 cannot start" contradicted the ownership note: the workspace can add a `FaceControls` to its own preview World |
| V15 | BC7 caveat in the texture memory row | Only cooked textures go up as BC7 (`crates/render/src/renderer/gpu.rs:861-885`) |

Confirmed without change: defects D1 to D4, D6, D7 and D9; the counts (27 material values, 19 names, 3 drawn; 15 built-in expressions; 48 face controls; 26 animation states; 15 shipped items, none with a thumbnail; 16 of 16 sampled textures; three images on the realistic `Skin` material); all CO7, CO8, CO17 and CO18 "missing" rows; all 6.1, 6.2 and 6.3 "missing" rows (no code under any other name).

Not checked: anything that needs a build or a run (the 6 ms figure, the 21 MB estimate, whether blended brows visibly mis-sort).
