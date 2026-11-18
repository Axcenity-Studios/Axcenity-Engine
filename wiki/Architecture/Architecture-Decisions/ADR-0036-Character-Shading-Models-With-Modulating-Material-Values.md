---
title: "ADR-0036: Characters get Skin, Eye, Cloth and Hair shading models, and material values that modulate maps"
type: adr
adr: 36
adr_status: Accepted
date: 2026-11-14
supersedes:
superseded_by:
affects: [avatar, scene, render, assets]
source: [crates/avatar/src/material.rs]
docs: [docs/development/character-studio/architecture.md, docs/development/character-studio/audit.md]
tags: [adr, characters, rendering]
---

# ADR-0036: Characters get Skin, Eye, Cloth and Hair shading models, and material values that modulate maps

- **Status:** Accepted (Paul, 2026-11-14: plan D9)
- **Date:** 2026-11-14
- **Affected systems:** Character Data (`avatar`: the material parameter set), `scene` (material translation), `render` (shading models, CO7)

## Context and problem

The renderer has Standard, Unlit and Toon shading and a foliage path (audit §3.10): no subsurface scattering, sheen, anisotropy or hair lighting, and custom WGSL surfaces can't change lighting. All 16 material texture slots are used. Customizing a colour today goes through a SurfaceAppearance that **replaces** a model's maps, so tinting skin loses its texture (audit §3.8). The research (§B.5, §B.6) recommends pre-integrated skin with two specular lobes, Charlie sheen for cloth and Karis/Tafuri-style hair lobes.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Custom WGSL surfaces per material | No renderer change | Can't add lobes or change diffuse |
| Built-in Skin, Eye, Cloth and Hair shading models, with a fixed parameter set per model (chosen) | Correct lighting; one contract the studio and runtime share | Renderer work (CO7); slots must be freed |
| Raise device texture limits | No packing | Less portable; still no lobes |

## Decision

Every character material slot names a shading model (`standard`, `skin`, `eye`, `cloth`, `hair`), and each model has a fixed set of values (`avatar::ShadingModel::values`) that **modulate** the material's own maps (tints multiply, scales multiply, offsets shift); every value's neutral setting leaves the material as authored. The renderer gains the four models (CO7), freeing two texture slots by packing metalness, roughness and occlusion into one texture. Until then `scene` draws them as Standard with the same tint and roughness values.

## Reasons

- Modulating keeps baked detail (pores, fabric weave) while customizing colour and response.
- A fixed, checked parameter set per model is a contract the studio (pickers), descriptions (values) and renderer (uniforms) share without knowing each other.
- `render` stays free of Instances: `scene` translates plain values (ADR-0003).

## Consequences

- The parameter set is part of Character Data: adding a value is an optional addition; changing a meaning needs a description migration.
- CO7 measures each model's GPU cost on the reference PC; screen-space SSS is CO17.
- Revisit if glTF ratifies a subsurface extension the set should map onto.

## Notes

- **2026-11-17 (R5, `r5-skin-shading`): the Skin model is built; the decision stands, two details are settled.** (1) *How the renderer learns a slot's model*: the model file's glTF material `extras` (`axcenity_shading`, and for skin `axcenity_defined_normal`), read by `assets` and drawn by `scene`, rather than an attribute from the applier; the manifest's `shading` remains the value contract (architecture.md §10). (2) *Texture slots*: the slots were not freed by packing; a skin's second (defined) normal map takes the specular map's slot, which skin doesn't read, as a toon ramp does. Skin is pre-integrated diffusion on a blurred normal, a redder penumbra, and two GGX lobes with specular antialiasing (`docs/rendering/skin.md`); screen-space SSS stays CO17. Two Skin values were added (`definition`, `softness`: optional, neutral 0), an addition under this ADR's consequences.

- **2026-11-17 (R5 makeup branch):** makeup and markings ([ADR-0042](ADR-0042-Makeup-And-Markings-Are-Layers-Over-Library-Masks-Drawn-Live-By-The-Skin-Model.md)) are not Skin values: a description's `skin` block resolves to draw values `layer_<id>_<field>` on the slot, and the Skin model draws them from three masks bound in the metalness, emissive and opacity slots, which skin with layers doesn't read.

- **2026-11-18 (CO7 wave 4, `co7-hair-cloth`): the Hair and Cloth models are built; the decision stands.** They follow the Skin model's pattern: the model file marks the material (`"axcenity_shading": "hair"` or `"cloth"` in glTF `extras`), the values ride the material's uniform, and no texture binding is added (the item shader is at 16 sampled textures). Hair is a Marschner-style model in Karis's approximation (R white toward the root, TT through the fibre, TRT tinted toward the tip); its card atlas's data and flow maps take the metalness and specular maps' places (`docs/rendering/hair.md`), and its colour values move the colour along the eumelanin and pheomelanin absorption curve in the shader. Cloth is Charlie sheen with Neubelt's visibility over the material's own light, dimmed by the sheen's albedo, with optional anisotropic GGX; its sheen and anisotropy come from glTF's `KHR_materials_sheen` and `KHR_materials_anisotropy` (`docs/rendering/cloth.md`). Six optional values were added (Hair: `root_melanin`, `root_length`, `streaks`, `streak_melanin`, `shift_scale`; Cloth: `anisotropy_rotation`), additions under this ADR's consequences. Cut-out hair cards are hashed under TAA and graded under MSAA, and cast through their holes in the shadow maps (other cut-outs cast whole cards, as before: through their holes, foliage's Shadows pass more than doubled). Packed ORM is still not needed.

## Implementation references

- `crates/avatar/src/material.rs` (`ShadingModel`, `ValueSpec`, `check_values`)
- `crates/render/src/skin.wgsl`, `crates/render/src/renderer/tests/skin.rs` (the Skin model, R5)
- `crates/render/src/hair.wgsl`, `cloth.wgsl`, `crates/render/src/renderer/tests/hair.rs`, `cloth.rs`, `alpha_cards.rs` (the Hair and Cloth models, CO7 wave 4)
- Plan: D9, CO7, CO17
