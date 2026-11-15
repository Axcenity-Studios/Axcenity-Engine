---
title: Known limitations
type: roadmap
status: current
verified: 2026-11-11
verified_commit: 19c7179d
docs: [docs/scripting/sandbox.md, docs/rendering/pipeline.md, docs/architecture/physics.md, docs/architecture/networking.md, docs/architecture/play-mode.md, docs/scripting/client-server.md, docs/rendering/lights.md]
tags: [roadmap, limitations]
---

# Known limitations

What the engine doesn't do, or does with a known flaw, today. Each entry names where it's documented. Performance issues are in [Known bottlenecks](../Performance/Known-Bottlenecks.md); structural problems in [Technical debt](Technical-Debt.md).

## Scripting

- No per-script **memory limit**; long C functions (`string.rep`, pathological patterns) aren't interrupted by the watchdog (`docs/scripting/sandbox.md`).
- `task.defer` runs at the next step (Roblox: later in the same frame); a RemoteEvent message with no handler is dropped (Roblox queues it); `ContextActionResult.Pass` and touch buttons not implemented (`docs/scripting/client-server.md`).
- Vector3 math allocates a userdata per result; `--!native` doesn't speed up engine calls.
- A breakpoint inside a C-called Lua function (a `table.sort` comparator), or in a script-made coroutine resumed from one, can't pause; a step goes on to the next line where it can. The same goes for a conditional breakpoint, a logpoint and Pause (Pause waits for the next script line that can pause, so a game whose scripts are all waiting pauses only when one wakes). Logpoints and conditions make their line yield on every pass, which slows a hot loop.

## Rendering

- Overlapping see-through items can blend in the wrong order (sorted by center).
- AMD/Vulkan/Windows: the sky clear isn't sRGB-encoded through an sRGB view (`docs/rendering/pipeline.md`).
- Lights are tiled in 2D, not clustered; rect lights use a representative point.
- No vertex colors or `KHR_materials_emissive_strength` from glTF; FBX uses the first UV set; glTF animations are read as bone takes, but their morph target weight channels aren't.
- Humanoid retargeting finds roles by bone names or by a rig definition of the project's (`assets/rigs/*.rig.json`); a retargeted take moves role bones only, so a bone without a role keeps its place (`docs/rendering/animation.md`).

## Physics

- MeshParts collide as convex hulls (hollows filled) unless already convex and small, their model has `_COL` collision pieces, or they're set to `PreciseConvexDecomposition` (R15.2). The Model Studio's Convex Pieces (R15.2b) runs on the UI thread and can take a few seconds on a big model.
- Setting `Position` stops a part (Roblox keeps its speed).
- Ragdolls can exceed joint limits briefly on hard landings.

## Networking

- **Game connections trust a typed address on first use** (MP4; a join link pins the fingerprint); a bare join code trusts the relay. Port mapping (MP5b) can't get past a provider's router (carrier-grade NAT; a relay can), and a killed host leaves its mapping until the 30-minute lease ends.
- No matchmaking or public relay (anyone can run `axcenity-relay`, MP5a), and prediction covers only characters (MP6a, under `Players.CharacterAuthority = "Server"`; physics bodies are MP6b; lag compensation and replays exist, Phase 81). Multiplayer has been measured with 16 bots across emulated home routers and 32 on one machine; a 16–32 client run on real machines (G13, MP7) is planned. All of it is planned in `docs/research/multiplayer-readiness.md` (MP1–MP7).
- **Server-moved characters** (MP6a): other players are drawn in the past on a device, so walking into a moving player costs a correction; moving platforms aren't rewound; seats, `MoveTo` and `MoveDirection` don't drive a remote player's character, and a device Script's `Jump` can be lost in a replay.
- Editor Play can't host or join a real network game.

## Characters

The Character Runtime (Character Studio overhaul CO3; [Characters](../Engine-Systems/Characters.md#applying-a-description-character-runtime), `docs/development/character-studio/architecture.md`):

- **Character Studio doesn't use the applier yet.** Its workspace still writes format 1 character files and poses its own preview by leaning every part toward its shapes each frame (CO12). A test holds the preview and the applier to the same result.
- **The editor's edit mode doesn't apply kept descriptions.** `DescriptionSync` is stepped by the game systems, which run in Play, the player and the server. A saved character that had a description applied therefore shows its base meshes in the editor (its derived `MeshId`s fall back to the base model) until Play (CO13).
- **A project made before CO3 has no character library until Insert > Character is used.** The library is written with the default character (`project::add_default_humanoid`: a new project with a character, Insert > Character, Character Studio's workspace folder). An older project gets it the next time Insert > Character is used; until then `Humanoid:ApplyDescription` fails with a message that says the project has no character library, and a packed game built from such a project has none.
- **Most material values aren't drawn yet.** Skin, eye, cloth and hair materials draw as Standard, and of a character's material values only `tint`, `iris_tint` and `roughness_scale` change the picture; the others (`melanin`, `redness`, `sheen_tint` and the rest) are stored on the parts and wait for the renderer's shading models (CO7).
- **Binding-fitted and socket items have no shipped content** (CO9, CO10): every item in the built-in library fits by its own copies of the body's shapes. A `binding` item would be drawn as a plain skinned part, unfitted.
- **A character's parts pick their levels of detail one by one.** A small part (underwear, eyes, brows) switches to a simpler copy nearer the camera than the body does, and a simplified garment doesn't keep its clearance from the skin: from about 9 studs the stylized character's skin shows through its underwear in blotches (seen in CO2's in-engine captures, which record each part's level in `captures.json`). Fixing it is CO18's "character LOD sets switched together", with CO9's region hiding (the body under a garment isn't drawn) and CO4's constrained LODs; until then judge clothing fit in the captures' full-detail view and in the Blender harness.
- **The identity bake's cost isn't measured** (CO18). A derived model is baked again in every run and whenever its base model is reloaded, never cooked, and each distinct description makes its own.

The realistic base body (Character Studio overhaul CO4, built and waiting for review; [Characters](../Engine-Systems/Characters.md#the-realistic-base-body), `docs/development/character-studio/testing.md`). Numbers are **Measured** with the Blender QA harness (`measure.py --package realistic`; masculine / feminine frame) and kept in `docs/development/character-studio/co4/measurements.json`:

- **`axcenity/realistic` is not yet a usable character family.** It has its body shapes (CO5); the face arrives in CO6. It has no items, face shapes or hair, and the library written into a project is still the stylized package alone.
- **The raised shoulder misses its target, and the measure and the picture disagree (open decision).** With the arm raised 150 degrees and CO5's corrective shapes the shoulder keeps 0.463 / 0.513 of its local volume (`docs/development/character-studio/co5/measurements.json`); the target is 0.8 or more, plain skinning gave 0.562 / 0.561 at CO4, and the package's own limit is 0.54, so its check is red on those two lines. The shapes are a delta mush of the pose and were chosen by the pictures: a first shape that pushed skin outward until the measure passed showed a step and a ball in the engine and was rejected. Nothing was loosened; what the shoulder is held to is the project owner's decision (`docs/development/character-studio/progress.md`). The other CO4 targets are met: the elbow at 135 degrees keeps 0.757 / 0.758 and the knee 0.74 / 0.75 of the ring area (0.6 or more), no face vertex above the head joint carries `Neck` weight (the stylized body has 911), and eyes and teeth don't interpenetrate at rest.
- **Still short elsewhere on that body:** the fist self-intersects (247 / 199 triangle pairs), the teeth touch the lower lip's inner side with the jaw open, and the lips rest about 2 mm apart (CO6).
- **The realistic body's sliders stop short.** Its proportion sliders run 0.85 to 1.15 and its hips only widen, because the body crosses itself beyond (**Measured**: 341 of 1,404 masculine bodies at the full 0.5 to 1.5). One normal map, baked from the athletic sculpt, is on every build, so soft bodies show abdominal lines; corrective shapes exist for the arm raised sideways only; the corrective rules are constants in `crates/anim/src/correctives.rs`, with no manifest field yet.
- **Twist and helper bones are found by name.** No manifest field describes the driver rule yet, so only a skeleton that uses the names in `crates/anim/src/twist.rs` is driven ([Animation](../Engine-Systems/Animation.md)).

## Editor

- Script completion and Go to Definition (F12) follow chains and declarations but doesn't type-check: a value it can't follow (a function's result, a table field, a module's return) offers nothing, luau-lsp ships in every package (a source build downloads it on request) and follows `require(game.ReplicatedStorage.X)` and `GetService` paths into modules (not `require(script.Parent.X)`, nor children's types such as `workspace.Door`); only open tabs are checked, and VS Code's set-up still checks each file alone.
- Panics while drawing the Play world aren't caught.
- Model files aren't watched for changes while the editor runs (read again on open).
- Bakes (reflection probes, light probe volumes) go stale and must be rebaked by hand (flagged). A bake runs a little each frame with progress and Cancel, but each probe still reads its six faces back one at a time, so thousands of probes take minutes.

## Content and platforms

- References may name an asset by path or by ID (PR #78); older projects still use paths.
- Built games ship cooked textures without their sources (G12); what the CPU reads of such a texture (terrain layers, color grading lookups, UI images) is decoded from BC7 or BC5, so it's close to the source, not equal. Model files (FBX, glTF) still ship beside their cooked copies.
- macOS is not verified on changes; mobile and web are not supported.
- No binary world format (S10k's 10 MB `world.json` loads in 0.19 s since C11).
