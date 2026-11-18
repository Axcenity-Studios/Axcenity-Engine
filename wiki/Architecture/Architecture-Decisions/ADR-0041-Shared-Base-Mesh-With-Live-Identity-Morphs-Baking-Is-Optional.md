---
title: "ADR-0041: A character is a shared base mesh with live identity morphs; baking is an optional optimization"
type: adr
adr: 41
adr_status: Accepted
date: 2026-11-17
supersedes: [ADR-0038]
superseded_by:
affects: [assets, anim, avatar, gameplay, scene, render, editor]
source: [crates/assets/src/model/identity.rs, crates/assets/src/derived.rs, crates/avatar/src/resolve.rs, crates/gameplay/src/description.rs, crates/anim/src/animators.rs, crates/render, crates/render/src/renderer/posing.rs]
docs: [docs/development/character-studio/architecture.md, plans/axcenity-engine-character-studio.md]
tags: [adr, characters, assets, animation, rendering]
---

# ADR-0041: A character is a shared base mesh with live identity morphs; baking is an optional optimization

- **Status:** Accepted. Paul decided the direction on 2026-11-17 (plan section 0, decision 9) when he declined to accept [ADR-0038](ADR-0038-Identity-Is-Baked-Into-Derived-Models-Named-By-MeshId.md) as the foundation. **Implemented (R2a, CO8 and CO9, 2026-11-17):** decisions 1 to 6 (3 on the runtime side) and the item parameters are built: the applier, followers and `DescriptionSync` wear identity live on the base meshes, and `gameplay::description::bake` is the explicit bake (R2a); **CO8 is done** (decision 2, refinement RA7): the renderer morphs (only shapes whose weight isn't 0) and skins each character once per frame into a posed buffer that the shadow, depth, motion and main passes read and ray tracing builds its BLASes from, so ray-traced shadows, AO, reflections and GI show identity and expression shapes (measured in `docs/development/character-studio/refinement/wave3/co8-compute-posing.md`). Decision 4 is built (CO9, refinement wave 3): `avatar::fit` transfers the body's identity and corrective shapes onto a garment at build or import, with push-out per shape and at the extremes; the realistic underwear keeps its own exact shapes (it is body topology). Not yet: a garment that passes the fit gate in every pose (the test garment and the boxer briefs still fail it, `docs/development/character-studio/refinement/wave3/co9-binding.md`).
- **Date:** 2026-11-17
- **Supersedes:** ADR-0038 as the source of truth for identity. ADR-0038's derived `MeshId` stays as the mechanism of the optional bake.
- **Affected systems:** Character Data (`avatar::resolve`), the Character Runtime (`gameplay`'s applier, `DescriptionSync`), Assets (derived models), Animation (posing, followers, `settle`), `scene` and `render` (morphing and skinning), Character Studio (live editing).

## Context and problem

ADR-0038 (CO3) bakes a character's identity, meaning its frame, build, face and proportion shapes, into a derived copy of every mesh it wears. That copy is named by an extended `MeshId` and rebuilt whenever the description changes. It made the runtime simple and kept identity shapes out of the per-frame morph cost, which the renderer pays again in every pass (audit §3.10).

Its costs show in the refinement:

- **Duplication and storage.** One copy per distinct description of every part it wears; a crowd of varied NPCs is a crowd of copies.
- **Editing is a rebuild.** A slider or a viewport handle (brief §3.4) makes a new bake each step, throttled to one every 120 ms (`DRAG_APPLY_EVERY`). The brief asks for immediate feedback and handles that follow the cursor.
- **Runtime customization is a rebuild.** Plan section 0, decision 6 lets games change appearance at runtime. Each change is a bake.
- **Clothing must follow the same body.** A garment fitted to a baked copy is fitted to a snapshot, not to the shape system the body is edited with.

Paul's decision: keep a shared base mesh with morph targets, or an equivalent deformation system, as the source of truth. Baking is an optional optimization for finalized characters, exports and LOD generation, not the foundation of customization. Clothing deformation and fitting stay compatible with the same body-shape system.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep ADR-0038: bake identity into derived models | Built; cheap per frame | The four costs above; declined by Paul |
| Live identity morphs on the CPU each frame | No renderer change | CPU skinning and morphing of 10 to 30k-vertex characters per frame doesn't scale to crowds |
| **Live identity morphs on the GPU: one shared base mesh per family, per-character weights, morphed and skinned once per frame into a posed buffer every pass reads (CO8)** (chosen) | One mesh in memory; editing and runtime changes are a weight upload; clothing reads the same weights | Needs CO8's compute posing so identity shapes don't cost in every pass; per-character weight storage (about 150 floats) |
| A sparse "identity delta" mesh per character | Smaller than a full copy | Still a rebuild per change; a second geometry state |

## Decision

1. **Source of truth.** A character on screen is the base model's shared mesh plus its description's resolved values: identity shape weights, bone changes after the pose (`settle`, kept from ADR-0038), material values and the parts list. `avatar::resolve` produces them as it does today. The applier sets them on the character's parts as **live** weights. It doesn't derive a mesh.
2. **Cost.** Identity weights are applied in the same morph pass as expressions and correctives. CO8 morphs and skins each character **once per frame** into a posed buffer that the depth, shadow, main and ray-tracing passes read, so the number of live shapes stops multiplying by the number of passes. Identity weights that are zero are skipped. A shape a description never moves costs nothing.
3. **Editing.** Character Studio and viewport handles change weights and bone changes directly. No bake happens while editing, and the 120 ms throttle goes.
4. **Clothing.** Garments follow the body through the same shape system: the body's identity and pose-corrective shapes are transferred onto each garment at build or import (by binding, ADR-0035). The garment then carries matching shapes, driven by the same weights. Layer push-out runs at transfer time per shape, and is checked at the parameter extremes by QA.
5. **Optional bake.** `Assets::derive` (ADR-0038's derived `MeshId`) stays, used only on request:
   - **finalizing** a character a game won't change (an NPC set, a crowd);
   - **exporting** a character package;
   - **generating LODs** of a specific character.

   A baked character drops its identity shapes. Changing it again takes it back to the live path. Baking is never needed to see a character correctly.
6. **Replication and saving.** Unchanged. The description is replicated and saved; each machine resolves the same weights. A saved `MeshId` names the base model unless the character was explicitly baked.

## Consequences

- CO8 (compute posing) moves forward. It is a prerequisite for the live path at crowd scale, and it gives ray-traced shadows the expressions as well (plan CO8).
- Until CO8 lands, a live character pays its identity shapes in every pass. That is acceptable in Character Studio and for a few characters; the R2a track measures it before switching the runtime over. CO8 landed on 2026-11-17: identity shapes now cost once a frame, and only those with a weight.
- `DescriptionSync`, followers and the applier change from "derive and swap `MeshId`" to "set weights and bone changes". Tests that compare a game with the studio preview (`a_character_file_applied_in_a_game_matches_the_workspaces_preview`) must hold to the same 0.2 mm.
- Derived-model registration and the 96-model cache serve the bake only.
- An item's own parameters (a hairstyle's length) are live weights on the item's part, not a bake.
- Revisit if a GPU class in plan section 0, decision 5 can't hold the posed buffers for the crowd budget of plan section 7. The fallback is the optional bake for static NPCs, which this decision keeps.

## Implementation references

- `crates/assets/src/model/identity.rs` (`Model::live`, `LiveIdentity`), `crates/assets/src/model/rig.rs` (`Posed::rebind`), `crates/assets/src/derived.rs` (`Assets::register_identity`, `identity`, `derived_count`)
- `crates/anim/src/live_identity.rs` (the `Identity` attribute, the per-model cache), `crates/anim/src/animators.rs` (settle after the clips, dress last, followers)
- `crates/gameplay/src/description.rs` (`apply_description`, `bake`, `is_baked`, `DescriptionSync`)
- `crates/render/src/renderer/posing.rs`, `crates/render/src/posing.wgsl` (CO8: the posing pass, its arena and active shapes), `crates/render/src/renderer/skinning.rs` (shapes kept shape by shape), `pose_corner` in `crates/render/src/shader.wgsl`, posed BLASes in `crates/render/src/renderer/ray_sway.rs`
- Tests: `assets` `an_identity_worn_live_poses_the_mesh_where_its_bake_does`; `player` `default_humanoid_tests::description` (weights only on a change, item parameters live, an explicit bake within 0.2 mm) and `character_capture::an_explicit_bake_draws_the_same_pixels_as_the_live_character`; `editor` `a_character_file_applied_in_a_game_matches_the_workspaces_preview`; `render` `tests/skinning.rs` (`the_posed_buffer_holds_each_corner_where_every_pass_used_to_pose_it`, `a_shape_at_weight_zero_is_not_posed_at_all`, `items_of_many_meshes_are_posed_together_as_the_arena_fills_afresh`) and `tests/ray_tracing.rs` `a_morphed_character_casts_the_ray_traced_shadow_of_its_shapes` (CO8)
