---
title: "ADR-0039: Group as Mesh makes a MeshPart and keeps its parts in a GroupedParts"
type: adr
adr: 39
adr_status: Proposed
date: 2026-11-16
supersedes:
superseded_by:
affects: [core, mesh_edit, editor, scene, physics]
source: [crates/core/src/csg.rs, crates/mesh_edit/src/group.rs, crates/mesh_edit/src/optimize.rs, crates/editor/src/app/group_mesh.rs, crates/editor/src/viewport.rs]
docs: [docs/architecture/solid-modeling.md]
tags: [adr, csg, modeling, data-model]
---

# ADR-0039: Group as Mesh makes a MeshPart and keeps its parts in a GroupedParts

- **Status:** Proposed (Paul asked for the tool on 2026-11-16; he accepts or changes this when he reviews the pull request)
- **Date:** 2026-11-16
- **Affected systems:** Instances (`core::csg`), Mesh editing and CSG (`mesh_edit`), the editor's Solid Modeling group and Explorer, the move tools, the scene and physics (what's out of the world)

## Context and problem

Paul wants to group MeshParts (and parts) into one new MeshPart whose model is optimized, unlike a Roblox Studio union, and to ungroup it and see the parts it was made of. Union already makes a `UnionOperation` from parts and keeps them inside it, out of the world (`core::csg::in_union`), for Separate, but it needs closed meshes (`solid_of`), cuts with BSP booleans, and flattens looks to a `GlbMaterial` (base color texture only). Grouping must accept open meshes, keep every look (textures, uvs, SurfaceAppearances) and never change what's seen. Three things had to be decided:

1. **What the result is.** A new class, a `UnionOperation`, or a plain `MeshPart`.
2. **Where the parts are kept** so they're out of the world, can be looked at, and come back where the result has moved to.
3. **What it collides as.** MeshParts collide as their hull by default, which fills a grouped room in.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| A new part class (say `MeshGroup`) holding its parts as children | One class says it all | A new BasePart class touches every system that draws, collides, serializes or scripts parts; scripts would see something that isn't a MeshPart |
| Reuse `UnionOperation` | Separate, `in_union` and the Explorer hiding all work already | It's a different thing (no booleans, open meshes allowed); its `UsePartColor` and `Hull` default don't fit; Separate would treat it as a union |
| A plain `MeshPart` with the parts as its direct children, marked by an attribute | No new class | Children of a MeshPart are in the world; every system would need to read an attribute to leave them out; scripts and lights the user puts inside would be confused with kept parts |
| **A plain `MeshPart` with a `GroupedParts` child holding the parts** | The result is an ordinary MeshPart everywhere (drawing, physics, scripts, Properties); one small container class marks what's kept; `in_union` covers it with one more class; the user's own children of the MeshPart are untouched | One more class (no properties of a part, two of its own) |

## Decision

Group as Mesh makes an ordinary **MeshPart** whose model is one optimized `.glb` (one object `Group`, a primitive per look, each source part's collision pieces as `Group_COLn` objects). The selection's roots go into a **GroupedParts** inside it, out of the world like a union's parts (`core::csg::in_union` covers both; `in_group` is it alone), listed dimmed in the Explorer, never moved by the move tools. GroupedParts records the MeshPart's place and size when grouped (`Origin`, `OriginSize`), and Ungroup Mesh moves, turns and stretches the parts by how the MeshPart has changed since.

## Reasons

- The result needs no new behavior anywhere: a MeshPart is drawn, collided, saved, replicated and scripted already. Only "what's inside a GroupedParts is out of the world" is new, and it rides on the check every system already makes for unions.
- Recording the origin, rather than moving the kept parts with the MeshPart as the union's do, makes Ungroup right however the MeshPart was moved (the move tools, Properties, a script with kept Play changes, a duplicate) and keeps the parts exactly as they were when nothing moved.
- Collision pieces per source part make the group collide as its parts did, concave or not, with no `PreciseConvexDecomposition` (slow, approximate) needed: MeshParts' Default fidelity already uses a model's `_COL` pieces (R15.2).

## Consequences

- One more class in `core`, the API docs and Luau definitions. Scripts can make a GroupedParts, but grouping itself is editor-only; there's no script call, since nothing Roblox-familiar does this.
- The `.glb` is a new kind of derived asset, `<name>.group.glb`, saved in `assets/models`, or higher (`assets`) when a look's textures are elsewhere, since a glTF's textures must be inside its folder. Undo keeps the file, as with unions and fractures.
- Looks a glTF can't hold are reported, not carried: built-in Material textures (Wood, Brick…), MaterialVariants, a Part's SurfaceAppearance, opacity and specular maps, DirectX normal maps, Decals. Baking those into textures is the obvious next step if Paul wants them.
- Scripts inside kept parts still run, as inside a union's parts or a stored Tool; nothing changed there.
- Revisit if grouping should happen at runtime (a script call) or if grouped models should stay linked to their parts (re-group on edit).

## Implementation references

- `crates/core/src/csg.rs` (`in_union`, `in_group`, `keeps_parts`, `grouped_parts`), `crates/core/src/class.rs` (`GroupedParts`)
- `crates/mesh_edit/src/group.rs` (`group_source`, `group`, `Grouped::glb`, `save_group`), `crates/mesh_edit/src/optimize.rs`
- `crates/editor/src/app/group_mesh.rs`, `crates/editor/src/viewport.rs` (`gather` leaves kept parts out), `crates/editor/src/explorer.rs` (dimmed rows, right-click items)
- Tests: `crates/mesh_edit/src/optimize_tests.rs`, `crates/mesh_edit/src/group_tests.rs`, `crates/editor/src/app/group_mesh_tests.rs`, `crates/core/src/csg.rs`
