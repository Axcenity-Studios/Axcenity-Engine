---
title: "ADR-0038: A character's identity is baked into derived models named by their MeshId"
type: adr
adr: 38
adr_status: Proposed
date: 2026-11-14
supersedes:
superseded_by:
affects: [assets, anim, avatar, gameplay, scene]
source: [crates/assets/src/model/identity.rs, crates/assets/src/derived.rs, crates/avatar/src/resolve.rs, crates/gameplay/src/description.rs, crates/anim/src/animators.rs]
docs: [docs/development/character-studio/architecture.md, plans/axcenity-engine-character-studio.md]
tags: [adr, characters, assets, animation]
---

# ADR-0038: A character's identity is baked into derived models named by their MeshId

- **Status:** Proposed (the lead, Character Studio overhaul CO3; it carries out plan decision D4, which Paul approved on 2026-11-14, and fixes how; Paul accepts or changes it when he reviews CO3)
- **Date:** 2026-11-14
- **Affected systems:** Assets (models), Animation (posing, followers), Character Data (`avatar::resolve`), the Character Runtime (`gameplay`'s applier), `scene` (drawing by `MeshId`)

## Context and problem

A character's blend shapes are of two kinds. Identity shapes (the frame, the build, a nose's width, clothing masks) are set when the character is made; live shapes (expressions, correctives) move every frame. Before CO3 Character Studio's preview leaned every part toward its identity shapes each frame (`CharacterWorkspace::poses`), and nothing in the player did at all. The renderer morphs in every pass (audit §3.10), so 150 identity shapes cost every frame for something that never changes. Plan decision D4 says to bake identity into a per-character rest mesh shared by characters with the same description. That left three questions.

1. **Where the baked mesh lives and how a MeshPart names it.** Everything that draws, poses, picks or measures a MeshPart looks its model up by `MeshId` (`Assets::model_slot`, `model_ready` and `model_blocking` are called from about forty files).
2. **What a parameter's bone changes do.** The feminine frame moves the skeleton's joints as well as the skin. The body carries a `Frame_Feminine` shape, so its surface is already where it should be and only its joints must move. The eyeballs and teeth carry no such shape and must go with their bones. The Stage 1 QA harness did this by hand (move the rest skeleton, pose the eyes).
3. **How clips keep playing.** Clips key bones of the masculine skeleton, translations included.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep every shape live; set identity weights on each pose (the workspace's way) | No new machinery | The cost D4 exists to remove; followers and unanimated characters need the weights too |
| A side table of baked models by Instance (like `Poses`), handed to `scene` | No `MeshId` change | Every model lookup (drawing, posing, picking, physics, the editor's panels) must learn the table; a character with no Animator has no entry |
| An attribute on the MeshPart naming the identity, read by each lookup | `MeshId` stays plain | The same forty call sites change |
| **A derived `MeshId`: the base's plus the identity's key after a second `#`, resolved inside `Assets`** (chosen) | No caller changes; shared by key; saved and replicated as an ordinary string | The key is only meaningful to a run that has registered the identity |
| Encode the whole identity in the `MeshId` | Self-describing | Kilobytes of text per part in every saved world and snapshot |
| Change the derived rig's rest pose and let clips play on it | Simple | Clips' translation keys would put the joints back |

## Decision

`avatar::resolve` turns a description into an `Identity`: one layer per parameter, corner and region mask, each with its shape weights and its bone changes. `Assets::derive(mesh_id, identity)` registers it and returns the base `MeshId` with the identity's key (`…#Body#i<16 hex>`); `Assets` bakes the model the first time that ID is asked for and keeps it per base model version. The bake adds the identity shapes into the vertices and drops them from the model's blend shapes, moves the surfaces a layer **carries** (a model with none of the layer's shapes), and puts the bind on the changed skeleton, so a derived model's rest mesh is right as it is. Every pose adds the bone changes after the clips (`Model::settle`, called by `anim`). The description itself stays in the World as a `HumanoidDescription`, and `DescriptionSync` applies it again in any run that hasn't, so each device registers the same identity under the same key.

## Reasons

- One string names the mesh, so drawing, posing, followers, picking, preloading and the editor's panels work unchanged, and `Pick::of_mesh_id` gives asset tools the base file.
- The key is a hash of names and numbers (FNV-1a, `Identity::key`), the same everywhere, so a replicated or saved `MeshId` means the same model once the description is applied; until then it draws the base model instead of failing.
- "Has the layer's shape" decides re-seat or carry from data both sides already have, per model, with no per-item flags: garments made with `fit: shapes` carry the frame shape and are re-seated like the body; eyeballs, teeth and rigid parts are carried.
- Adding bone changes after the clips keeps one set of clips for every frame and proportion, as `BodyShape` already does for the body sliders.
- A derived model is ordinary `Model` data: LODs, cooked textures and GPU mesh sharing (`scene`'s `MeshCache`) apply to it as to any model, and characters with one description share it.

## Consequences

- A character's parts must all be derived with the body's identity (the applier does this), or a follower would take the body's settled pose onto an unsettled bind.
- A derived model is never cooked or kept on disk; it is baked again per run and when its base is reloaded (the bake's cost isn't measured yet; CO18 measures characters). `Assets` lets go of derived models nothing holds once it has 96; the registered identities themselves (a few kilobytes each) are kept for the run, since a `MeshId` in the World may still name one.
- Dragging a slider in Character Studio makes a new derived model per step (CO12 decides whether the preview keeps shapes live while dragging).
- Until a run applies a kept description, its characters show their base meshes: hosts must step `DescriptionSync` (the game systems do; the editor's edit mode is CO13's).
- A shape that should stay adjustable every frame must not be named by a parameter, corner or region.
- Revisit if identity must change every frame for many characters at once (a crowd morphing), or if CO8's compute posing makes live identity shapes free.

## Implementation references

- `crates/assets/src/model/identity.rs` (`Identity`, `Model::bake`, `Model::settle`, `derived_mesh_id`, `split_derived`), `crates/assets/src/derived.rs` (`Assets::derive`, the lookups), `crates/assets/src/model/objects.rs` (`Pick::of_mesh_id`)
- `crates/avatar/src/resolve.rs` (`resolve`, the layers), `crates/gameplay/src/description.rs` (`apply_description`, `DescriptionSync`), `crates/anim/src/animators.rs` (`settle` after `pose_of`; `follow`'s live shape weights)
- Tests: `assets` `model::identity::tests`, `derived::tests`; `avatar` `tests::resolving`; `player` `default_humanoid_tests::description` (parts baked, eyes carried, skeleton re-seated, kept descriptions applied again); `editor` `a_character_file_applied_in_a_game_matches_the_workspaces_preview`
- Plan: `plans/axcenity-engine-character-studio.md` D4, CO3
