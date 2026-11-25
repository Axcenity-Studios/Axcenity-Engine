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

## Implementation notes (CO9, 2026-11-17)

- Fitting happens at build or import time, not when a description changes ([ADR-0041](ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md) decision 4): the binding carries the body's skin weights and every identity and corrective shape onto the garment as same-named morph targets, pushed out of the body and lower layers per shape and at the parameter extremes. At run time a binding item is a skinned part that takes the body's weights by name.
- Region hiding uses the base manifest's `model.regions_map` (the region map image); the body is drawn from a model with the hidden map regions' triangles left out (`…#h<key>`, `crates/assets/src/hidden.rs`).
- The realistic underwear keeps `shapes` and its mask shapes: it covers no map region whole, and where it bridges hollows its own per-frame shapes are better than a transfer (measured in `avatar`'s `tests/fitting.rs`, removed with the realistic package on 2026-11-25). Its build pushes those shapes out of the skin the same way (`underwear.py` `push_out`: each shape at full weight and the build's muscle and fat corners, on both frames).
- Skin weights come from the skin under each garment corner (rays out along the skin's normals that meet it), so cloth bridging a crease takes both sides; a corner no skin reaches keeps its bound point's. Binding to the closest point is still ambiguous in concave places, and linear skinning of cloth that stands off the skin leaves the underarm open with the arms raised; loose garments still need proxies, deformer maps or solved pose correctives (review fixes 1, 2026-11-17).

## Notes

- **2026-11-25 ([ADR-0050](ADR-0050-The-Engine-Ships-One-Character-Family-MakeHumans-Body-With-Quaterniuss-Animations.md)).** The realistic and stylized packages were removed, and with them the assets this ADR's notes measure on: the realistic underwear, the test tee (`avatar`'s `tests/fixtures/test-tee`) and `tests/fitting.rs`. The mechanism stays: `avatar::fit` with its unit tests, region hiding and the import flow. No shipped item is fitted by binding now: every item of the human base is `fit: shapes`, converted from MakeHuman's clothes ([ADR-0047](ADR-0047-The-Realistic-Family-Is-Rebuilt-On-MakeHumans-Body-And-Wardrobe.md)), and none names a region to hide (they hide skin by cover maps, [ADR-0045](ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md)).

## Implementation references

- `crates/avatar/src/fit/` (binding, transfer, push-out, coverage, the import flow), `crates/assets/src/hidden.rs`, `crates/gameplay/src/description.rs` (`hidden_body`); tests `avatar` `fit::tests` and `tests/fitting.rs`, `player` `fit_gate_tests`

- `crates/avatar/src/item.rs` (`Fit`, `layer`, `hides`), `manifest.rs` (`Region`)
- Plan: D6, CO9 (binding, push-out, region hiding, the import flow)
