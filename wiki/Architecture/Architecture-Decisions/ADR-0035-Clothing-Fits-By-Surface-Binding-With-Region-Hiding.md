---
title: "ADR-0035: Clothing fits any body by surface binding with layer push-out; covered skin is hidden by body regions"
type: adr
adr: 35
adr_status: Accepted
date: 2026-11-14
supersedes: ADR-0030
superseded_by:
affects: [avatar, assets, scene, render, editor]
source: [crates/avatar/src/item.rs, crates/avatar/src/manifest.rs]
docs: [docs/development/character-studio/architecture.md, docs/development/character-studio/research.md]
tags: [adr, characters, clothing]
---

# ADR-0035: Clothing fits any body by surface binding with layer push-out; covered skin is hidden by body regions

- **Status:** Accepted (Paul, 2026-11-14: plan D6)
- **Date:** 2026-11-14
- **Affected systems:** Character Data (`avatar`: item manifests, fitting), `assets` (fitted derived meshes), `scene` and `render` (region hiding), the import flow
- **Supersedes:** [ADR-0030](ADR-0030-Smooth-Stylized-Humanoid-On-CC0-Base-Meshes-With-Mask-Shapes.md)'s clothing masks as blend shapes (its art direction is superseded by [ADR-0032](ADR-0032-Semi-Realistic-Default-Character-Family.md))

## Context and problem

Each stylized garment fits because the build gave it its own copy of every body shape it follows (audit §3.5): that costs garments × shapes, works only for garments the script built, never for an imported one, and can't follow shapes added later. Covered skin is hidden by ten coarse mask shapes that pull skin in, all or nothing, which can't express a cropped top, a sleeve length or gloves, and can't stop two layers crossing. The brief needs any garment to fit any body, both frames, layered, imported or built.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Per-garment shape copies (today) | Exact for built garments | Doesn't scale; no imports; no new sliders |
| Several authored sizes per garment (MetaHuman) | Large shape changes look right | Too much authoring |
| Deformer maps in body UV space (The Sims 4) | Any number of sliders; imports | Resolution limits; loose garments approximate |
| Surface binding at build or import, rebuilt when the body changes, with push-out between layers (chosen) | Works from geometry alone; exact for tight garments; cheap to bind | Artefacts grow with distance from the body; needs a well-formed body |
| Cages per item (Roblox) | Works over layers | An authored cage per item |

## Decision

Garments fit by **surface binding**: each vertex bound to the body's surface (triangle, barycentric coordinates, offset along the normal), made at build or import; when a description's identity changes, fitted rest positions are rebuilt from the resolved body and pushed clear of the body and every lower layer (`layer` 0–3) by a margin. Covered skin is hidden by **regions**: a region index per body triangle from the body's second UV set or a region map, and each item lists the regions it `hides`. The stylized base keeps its mask shapes as region hiding (`regions[].mask_shape`) and its garments keep `fit: shapes` until it is retired. Deformer maps stay the fallback if binding fails on loose garments.

## Reasons

- Binding needs nothing but the garment's geometry, so imports fit like built items.
- Region hiding expresses any coverage and tells which lower layers a garment covers.
- The item manifest names the method (`shapes`, `binding`, `socket`), so the old and new ways coexist during the overhaul.

## Consequences

- Fitted meshes are derived per description (cached by identity, shared by characters with the same look, CO3/CO9).
- Body topology needs a second UV set or region map (CO4).
- Loose garments (skirts, coats) may need cloth on coarse proxies (CO15) or deformer maps.
- Revisit if binding artefacts on loose garments exceed the poke-through thresholds after push-out.
- **Interim exception (2026-11-16, before CO9):** the realistic body's underwear (`assets/characters/realistic/underwear.py`) is cut from the body's own surface, so it fits by `shapes` (each vertex a fixed mix of body vertices, exact under every shape, nothing to bind), and each item hides a region with a **mask shape** that sinks the skin under it, because region-map hiding isn't drawn yet. Paul asked for undergarments on every character before CO9. When CO9 lands they move to map regions and the mask shapes go; being body topology, they can keep `shapes`. Not a change of this decision.

## Implementation references

- `crates/avatar/src/item.rs` (`Fit`, `layer`, `hides`), `manifest.rs` (`Region`)
- Plan: D6, CO9 (binding, push-out, region hiding, the import flow)
