---
title: "ADR-0030: The default humanoid: smooth stylized, built on CC0 base meshes, clothing masks as blend shapes"
type: adr
adr: 30
adr_status: Superseded
date: 2026-11-14
supersedes:
superseded_by: ADR-0032 (art direction), ADR-0035 (clothing masks)
affects: [assets, project, anim, editor]
source: [crates/project/src/default_humanoid.rs]
docs: [docs/research/character-studio.md, docs/research/default-humanoid.md]
tags: [adr, characters, assets]
---

# ADR-0030: The default humanoid: smooth stylized, built on CC0 base meshes, clothing masks as blend shapes

- **Status:** Superseded (2026-11-14, Paul's overhaul decisions) by [ADR-0032](ADR-0032-Semi-Realistic-Default-Character-Family.md) (semi-realistic is the default family; this stylized humanoid stays as a second family) and [ADR-0035](ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md) (fitting by binding and hiding by regions replace mask shapes for new bodies). The stylized asset described here is still shipped and still hides skin with its masks until it is retired
- **Date:** 2026-11-14
- **Affected systems:** the default humanoid asset (`assets/characters/humanoid`), `project`'s default character, Character Studio (CS2–CS5)

## Context and problem

The default humanoid of `docs/research/default-humanoid.md` (decision D1) was faceted low-poly, generated from nothing by `build.py`: about 5,400 triangles of lofted rings, a mask-like head, tube fingers, helmet hair. Paul's brief of 2026-11-14 asks for a polished, smooth stylized character with appealing anatomy, real hair volumes, believable clothes, a facial rig and live body and face customization (`docs/research/character-studio.md`). Hand modelling isn't available in this workflow, and the result must stay rebuildable and reviewable. Customizable clothes need the body hidden under them without seams or cracks.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep generating anatomy from nothing | Fully original, small script | Can't reach smooth, appealing anatomy or faces |
| Blender Studio's Human Base Meshes (CC0 1.0) as the base, everything else by script | Clean all-quad animation topology, stylized male and female sharing one topology, face sets for every region, CC0 | A 3.2 MB vendored source; the look starts from someone else's sculpt |
| MakeHuman base mesh and targets (CC0 since 2020) | Huge library of artist-made shape targets | Realistic and dated; restyling it is more work than the gain |
| Blender Studio's film characters (CC-BY) | Production quality | Named characters, production rigs, attribution |
| Body split into region objects that are hidden under clothes | No overdraw under clothes | The engine simplifies each primitive's LODs alone, so region borders crack at a distance; long MeshPart pick lists |
| Clothing masks as blend shapes that pull covered skin in | One seamless mesh at every LOD; per-character weights the engine already carries (`Posed::morph_weights`, file default weights) | Covered skin still drawn (the fixed default file deletes what its outfit fully hides) |

## Decision

The default humanoid is built by script on Blender Studio's CC0 stylized base meshes, vendored as OBJ with provenance; the masculine body is the mesh and the feminine body its `Frame_Feminine` shape; the skeleton contract (names, rolls, 45° A-pose, 1.75 m, +Z front) is unchanged. Clothes hide skin through mask blend shapes on the one body mesh, named by each garment in `Humanoid.customization.json`.

## Reasons

- The base meshes give what script geometry couldn't: anatomy, hands, feet and a face with animation loops, under a licence that lets the engine ship and relicense the result as CC0.
- Keeping the skeleton contract keeps every clip, retarget profile, state machine, prefab and face profile working (the existing tests pass on the new files).
- Masks keep the body one primitive, so the engine's own LODs (which keep blend shapes) can't open seams, and a character's look stays a set of weights.

## Consequences

- The asset files grew (2.9 + 6.8 + 1.3 MB, from 1.7 + 2.7 + 1.3), mostly shape keys; `project` embeds them.
- Frame and eye identity shapes also move bones; the engine must apply the offsets in `Humanoid.customization.json` (Character Studio CS3) before those shapes are usable on their own.
- A part's shapes (clothes following the frame and build) need the character's weights on every part, not only the body (CS3).
- Revisit if an artist-made character replaces the generated one (the file names and shape names are the contract), or if overdraw under clothes shows in profiles.

## Notes

- **2026-11-25 ([ADR-0050](ADR-0050-The-Engine-Ships-One-Character-Family-MakeHumans-Body-With-Quaterniuss-Animations.md)).** The stylized package this ADR describes (`assets/characters/humanoid/`: its build scripts, `Humanoid.customization.json` and `HumanoidParts.glb`) was removed from the repository, with the realistic package; the repository's history keeps both. The default humanoid's model is now the human base's default body (`assets/characters/human/Humanoid.glb`), and `HUMANOID_PARTS` and `modular_character` named below no longer exist. What stays is the mechanism: one skinned body on the humanoid bone names (the human base's manifest keeps `skeleton.convention: humanoid`) with shapes driven by parameters.

## Implementation references

- `assets/characters/humanoid/build.py` and its modules; `source/extract.py` (bundle URL and SHA-256)
- `crates/project/src/default_humanoid.rs` (`HUMANOID_PARTS`, the model box, `modular_character`)
- Tests: `project` `default_humanoid::tests`, `player` `default_humanoid_tests`
- Plan: `docs/research/character-studio.md` D1–D5, stage CS1
