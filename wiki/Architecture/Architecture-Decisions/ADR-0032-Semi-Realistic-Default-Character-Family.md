---
title: "ADR-0032: Semi-realistic is the default character family, on Blender Studio's realistic CC0 bodies; stylized stays as a second family"
type: adr
adr: 32
adr_status: Accepted
date: 2026-11-14
supersedes: ADR-0030
superseded_by:
affects: [assets, project, anim, avatar, editor]
source: [crates/avatar/src/manifest.rs, assets/characters/humanoid/manifests.py, assets/characters/humanoid/Humanoid.base.json]
docs: [plans/axcenity-engine-character-studio.md, docs/development/character-studio/architecture.md, docs/development/character-studio/research.md]
tags: [adr, characters, assets]
---

# ADR-0032: Semi-realistic is the default character family, on Blender Studio's realistic CC0 bodies; stylized stays as a second family

- **Status:** Accepted (Paul, 2026-11-14: plan section 0, decision 1)
- **Date:** 2026-11-14
- **Affected systems:** the default humanoid asset, Character Data (`avatar`), Character Studio, `anim` (skeleton additions)
- **Supersedes:** [ADR-0030](ADR-0030-Smooth-Stylized-Humanoid-On-CC0-Base-Meshes-With-Mask-Shapes.md) (its art direction; its mask shapes are superseded by [ADR-0035](ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md))

## Context and problem

ADR-0030 made the default humanoid smooth stylized, on Blender Studio's CC0 *stylized* base bodies. Paul's overhaul brief of 2026-11-14 asks for semi-realistic characters (every reference pick is), with sound anatomy, a corrected face and both frames. The Stage 1 audit (`docs/development/character-studio/audit.md` §2) measured the stylized body's inherited proportions (about 6 heads tall, short upper arms, big masculine hands, narrow feminine shoulders) and found face skinning, eye and joint defects that the art direction can't fix. The research (§B.2) checked that the same CC0 bundle has **realistic** masculine and feminine bodies sharing one topology (10,582 vertices), a realistic head with separate iris and sclera meshes, and realistic hands, feet and teeth.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep stylized as the default | Built, tested | Contradicts every reference pick; the defects are in the base |
| Realistic CC0 bodies as the default, stylized as a second family (chosen) | Shippable (CC0), scan-based, clean animation topology, both frames on one topology; the stylized work stays usable | Art production is the hard part (plan risk 1) |
| MakeHuman's base mesh | Huge target library | Dated topology (its targets are used as data instead, D5) |
| MetaHuman, SMPL family, scan stores | Quality | Can't be bundled in an Apache-2.0 engine |

## Decision

Semi-realistic is the primary character family, built on the realistic CC0 bodies; the stylized bodies stay as an alternative family. The two don't share a visual standard but share every system: the skeleton contract (with additive twist bones), animation, clothing fitting, morphs and the Character Data formats. A family is a base model package with its own manifest; items name the bases they fit.

## Reasons

- The realistic pair is the only shippable scan-quality base found, and its shared topology gives masculine ↔ feminine as one blend.
- Keeping the skeleton contract keeps all 32 clips, retargeting, state machines and face profiles working.
- A family is data (`*.base.json`), so a second family costs content, not code.

## Consequences

- The stylized humanoid stays shipped and described by `assets/characters/humanoid/Humanoid.base.json` (`axcenity/stylized`) until the realistic one (CO4–CO6) is ready; format 1 character files migrate to it.
- Items are per base (`bases` in the item manifest); fitting by binding (ADR-0035) lets one garment target both families.
- Revisit if artist-made realistic bodies with a redistributable licence appear, or if the realistic topology can't take the required correctives.

## Implementation references

- `crates/avatar` (base model manifests, families), `assets/characters/humanoid/manifests.py`
- Plan: `plans/axcenity-engine-character-studio.md` D1, D5, CO4–CO6
- Tests: `crates/avatar/tests/shipped_library.rs`
