---
title: "ADR-0042: Makeup and markings are layers a description turns on over masks in the model file, drawn live by the Skin model; decals are placed in the rest pose later"
type: adr
adr: 42
adr_status: Proposed
date: 2026-11-17
supersedes:
superseded_by:
affects: [avatar, assets, scene, render, gameplay, editor]
source: [crates/avatar/src/skin.rs, crates/assets/src/model.rs, crates/scene/src/material.rs, crates/scene/src/draw_list/materials.rs, crates/render/src/skin.wgsl, crates/render/src/renderer/lighting.rs, assets/characters/realistic/makeup.py]
docs: [docs/development/character-studio/architecture.md, docs/rendering/skin.md, docs/development/character-studio/references/skin/README.md]
tags: [adr, characters, rendering, assets]
---

# ADR-0042: Makeup and markings are layers a description turns on over masks in the model file, drawn live by the Skin model; decals are placed in the rest pose later

- **Status:** Proposed (the lead delegated the decision to the R5 makeup track, Character Studio refinement wave 3, 2026-11-17; built in the same change).
- **Date:** 2026-11-17
- **Affected systems:** Character Data (`avatar`: the description's `skin` block, the base manifest's `skin_layers`, `resolve`), Assets (glTF `extras.axcenity_skin_masks`), `scene` (the material translation), `render` (the Skin model's layers), the realistic build (`makeup.py`), Character Studio (the Skin page's Makeup and markings).

## Context and problem

Brief §6.2 asks for makeup layers (foundation, blush, eyeshadow, eyeliner, lips, brows), each with colour, opacity and roughness, turned on and off on their own; §6.1 for freckles, moles, blemishes and scars; §6.3 for an architecture for tattoos, birthmarks, scars, marks and custom decals with placement, rotation, scale, opacity and blending, without an authoring tool yet. Plan 11.3 reserved a description field `skin` for R5's ADR and stored nothing in `extra` meanwhile.

The constraints:

- **Format 2 stays readable both ways** (plan 11.3): only optional fields, no new value in an existing enum (an older engine rejects a file with an unknown enum value).
- **Identity stays live** ([ADR-0041](ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md)): changing a colour or an opacity re-applies with no bake, and no texture is rebuilt on the CPU per frame.
- **Masks are content in the library** (UV-space masks per base, or seeded procedural ones), not per-character textures written at runtime unless that is measured cheap.
- **Drawn by the Skin model** ([ADR-0036](ADR-0036-Character-Shading-Models-With-Modulating-Material-Values.md)) in the skin references' layer order (`references/skin/README.md` §2): markings in the skin, then foundation (evens the tone), blush, eyeshadow, eyeliner, lips and brow fill over it, all under the specular light.
- **The renderer has no texture slots to spare.** The scene pass's fragment stage binds 16 sampled textures, the WebGPU default limit every device here is opened with (8 of the frame's, 8 of the material's). A new binding would need raised limits on every device (the editor, tests, benchmarks) and break the portable default.

## Considered alternatives

**Where the description keeps them:**

| Option | For | Against |
|---|---|---|
| Material values per layer on the Skin slot (`blush_colour`, `blush_opacity`, …; about 25 more Skin values) | Flows through the existing pipeline unchanged | An older engine's check reports every one as an unknown value; a flat list of 34 Skin values; no room for a list of decals with placement; turning one off loses its settings |
| **An optional `skin` block, `{makeup: [...], markings: [...]}`, each entry naming a layer the base offers (chosen)** | An older engine keeps the block in `extra` untouched; layers named by strings the base manifest offers, so no enum grows; on/off keeps settings; room for `decals` beside them; presets of kind `skin` carry it | `resolve` must turn it into draw values |

**How the masks reach the shader:**

| Option | For | Against |
|---|---|---|
| A texture array of single-channel masks | One binding, any number of layers | A new binding type in the material group (all 16 slots are taken), a second texture path through `assets`, `scene` and the budget |
| One texture per layer | Simple | Nine textures; no slots |
| **Three packed RGBA masks in the model file, one layer to a channel, bound in the metalness, emissive and opacity maps' places, which skin with layers doesn't read (chosen)** | No new binding or limit; ordinary glTF images the cooker and texture budget already handle; three texture reads | A fixed channel layout (nine layers, one spare channel); a skin with layers can't also have an emissive or opacity map |
| Composite every layer into a per-character overlay on the CPU (`references/skin` §2's recommendation) | Fixed shader cost; marks crossing UV seams by projection | A texture per distinct character (16 MiB at 2K before compression), rebuilt on every change: against "live" and "no per-character textures" unless measured cheap; it is the optional bake's job later |

**Freckles and moles:** a fixed mask can't be re-seeded; a fully procedural shader spot needs the surface's 3D position at rest per pixel (the skinned position moves). Chosen: **seeded spots in the mask**. The build places candidate spots once over a density field (where the sun falls for freckles), measured on the rest surface, and stores each spot's coverage and a **rank** (0 to 1, constant over the spot). The shader shows the spots whose rank, offset by the description's `seed` times the golden ratio, is below its `amount`. Another seed shows other spots, a larger amount more of them, all live.

## Decision

1. **The description** (format 2, optional): `skin: {makeup: [SkinLayer], markings: [SkinLayer]}`, unknown fields kept. A `SkinLayer` is `{layer, enabled?, colour?, opacity?, roughness?, amount?, seed?}` (`avatar::SkinLayer`). `layer` names one of the base's `skin_layers`; `enabled` false keeps its settings and draws nothing.
2. **The base manifest** (optional): `skin_layers: [{id, label, kind: makeup | marking, material, about?, colour?, roughness?, opacity?, amount?}]` (`avatar::SkinLayerSpec`): what a person can turn on, on which skin-shaded material, with the settings a new layer starts at. `kind` reads an unknown value as `Other` (listed, never drawn), so the field can grow. Checks: IDs once, the material exists and is skin-shaded, numbers 0 to 1; a description's layers exist, sit in the list of their kind, once each, numbers 0 to 1.
3. **Resolution** (`avatar::resolve`): every layer turned on with some opacity becomes **draw values** on its material, `layer_<id>_<field>` (opacity, colour, roughness; amount and seed for markings), the base's settings filling what the description leaves out. The applier writes them as the body's material value attributes like every other value, so a change is a few attributes: live, replicated, no bake. A preset of kind `skin` (and `character`) carries the block; others leave it alone.
4. **The model file** names its masks: glTF material `extras.axcenity_skin_masks: [{index}, {index}, {index}]`, three textures placed like the base colour map, on a material marked skin. The channel layout is the contract (`docs/rendering/skin.md`): foundation, blush, eyeshadow, eyeliner; lips, brow fill, scar, (spare); freckles, their ranks, moles, their ranks. The layer IDs the renderer draws are that fixed set; a base offers any of them.
5. **Drawing** (`render::SkinLayers`, `skin.wgsl`'s `skin_layered`): into the albedo and roughness before light, markings as pigment (multiplied: freckles and moles darker and browner, the scar paler and smoother), then makeup mixed over the skin by its coverage, foundation first, each keeping part of the skin's own fine variation (the albedo over its 8-texel neighbourhood) so it reads as makeup on skin, not paint. Foundation without a colour takes the skin's own tone (the albedo over about 32 texels), which evens it. The values ride in the uniform where skin reads nothing else (the surface shader's values, the toon values, the specular row's last two). With every layer off, `scene` binds no masks at all: skin without makeup costs what it did.
6. **Decals (tattoos, birthmarks, paint, custom marks; not built):** a later `skin.decals: [{mark, anchor, offset, rotation, scale, opacity, mirror?}]` beside the layers, kept today by `Skin::extra`. A mark is a library image with a kind (`tattoo`, `birthmark`, `scar`, `paint`) and a blend (`multiply` for ink under the skin, `pigment` for birthmarks and scars, `over` for paint); an anchor is a named point on the base's rest surface with a frame. Marks are placed by **projection in the rest pose** (`references/skin` §3, option B), so they follow the skin and cross UV seams, and are drawn either by a per-character overlay composited when the description changes (measured first) or, for a few marks, by projection in the shader from a rest-position attribute. Runtime box decals stay for passing effects only: a box projects through a deforming mesh.

## Consequences

- Format 2 files with makeup open in older engines unchanged (the block rides in `extra`) and draw bare skin there.
- The fixed channel layout is part of the content contract; a tenth layer kind needs a fourth mask (and a slot: the specular map's is taken by the defined normal map) or the decal path.
- A skin material with layers loses its own emissive and opacity maps while any layer is on. No character skin has them; a fantasy skin that glows would need a different route.
- Ray-traced reflections and GI see the skin's albedo without makeup (they read the material's colour, not the masks). Recorded in Known Limitations.
- The masks are 2K like the skin maps: a 1 mm eyeliner is about two texels on the face. A face-only mask at higher density is the upgrade if close-ups need it.
- Revisit when decals are built (point 6), or if CO8's posed buffer gives the shader a rest position (then freckles could be procedural at any density).

## Implementation references

- `crates/avatar/src/skin.rs` (`Skin`, `SkinLayer`, `SkinLayerSpec`, `skin_layer_value`, the checks and `draw_values`), `description.rs`, `manifest.rs`, `resolve.rs`, `preset.rs`; tests `crates/avatar/src/tests/skin_layers.rs`
- `crates/assets/src/model.rs` (`skin_masks_of`, `ModelMaterial::skin_masks`), `model/cooked_model.rs`
- `crates/scene/src/material.rs` (`set_skin_layers`), `draw_list/materials.rs` (`skin_masks`)
- `crates/render/src/draw_list.rs` (`SkinLayers`, `Makeup`, `Spots`), `renderer/lighting.rs` (`skin_layer_uniform`, the slots), `renderer/gpu.rs` (the masks read as values), `skin.wgsl` (`skin_layered`), `shader.wgsl` (the call in `standard_surface`); tests `renderer/tests/skin_layers.rs`
- `assets/characters/realistic/makeup.py` (the masks), `manifests.py` (`SKIN_LAYERS`), `build.py`
- `crates/editor/src/character_workspace/panel/materials.rs` (Makeup and markings), `intent.rs` (`SetSkinLayer`)
- `crates/player/src/makeup_tests.rs` (every layer in a capture, off draws bare skin, a colour change live, softness alone)
