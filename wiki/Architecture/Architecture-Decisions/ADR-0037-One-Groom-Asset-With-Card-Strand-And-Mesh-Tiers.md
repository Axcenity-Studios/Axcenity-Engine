---
title: "ADR-0037: Hair is one groom asset with card, strand and mesh tiers; cards first, strands optional"
type: adr
adr: 37
adr_status: Accepted
date: 2026-11-14
supersedes:
superseded_by:
affects: [avatar, convert, anim, scene, render]
source: [crates/avatar/src/item.rs]
docs: [docs/development/character-studio/architecture.md, docs/development/character-studio/research.md]
tags: [adr, characters, hair, rendering]
---

# ADR-0037: Hair is one groom asset with card, strand and mesh tiers; cards first, strands optional

- **Status:** Accepted (Paul, 2026-11-14: plan D10 and section 0 decision 3)
- **Date:** 2026-11-14
- **Affected systems:** Character Data (`avatar`: groom items), `convert` (the groom converter, editor only), `anim` (spring bones, guide simulation), `scene` and `render` (card shading, the strand pipeline)

## Context and problem

Today's hair styles are voxel-remeshed solid masses (audit §2.2 defect 4): lumpy, floating off the scalp or sinking into the cheeks, with no strand detail. Production characters use cards for most hardware and strands where affordable (research §B.1). Paul decided: cards first, strands second, in three quality levels; strands are optional and scale independently, and a character always loads and draws without them.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Strands first | Most visible win | Most risk; no help for crowds or low-end GPUs |
| Separate assets per tier | Simple | Tiers drift apart; three things to author per style |
| One groom with card, strand and mesh tiers made from the same strands (chosen) | One authoring source; the runtime picks a tier per machine and distance | A converter and a binary groom file |
| TressFX, HairWorks | Existing | Frozen or closed |

## Decision

A hairstyle is one **groom item** (`kind: groom`) with up to three tiers: **cards** (alpha-clipped cards with hair shading and spring bones), **strands** (an optional `*.groom` file of strands, guides and scalp roots, drawn by a compute pipeline, CO14), and **mesh** (a solid fallback). A groom must have cards or a mesh. Cards are generated offline from the same strands. Grooms come from Blender hair curves or USD `BasisCurves`, converted on import by the editor-only `convert` crate (ADR-0027). A legal review of hair-rasterizer patents comes before strands ship.

## Reasons

- Every GPU and every NPC needs cards; strands then improve hero characters without becoming a dependency.
- One strand source keeps the tiers consistent.
- The groom file format is fixed in `architecture.md` §11 before Agent 3 starts, so the converter and runtime agree.

## Consequences

- The stylized hair styles are grooms with only a `mesh` tier today.
- Hair cards need cut-out-aware shadows and TAA-friendly alpha (CO7) to avoid slab shadows.
- Revisit tiers if mesh shaders become broadly available in wgpu, or if the patent review blocks the strand rasterizer.

## Implementation references

- `crates/avatar/src/item.rs` (`Groom`, `ItemKind::Groom`; the cards-or-mesh rule in `ItemManifest::validate`)
- Plan: D10, CO11 (cards), CO14 (strands)
