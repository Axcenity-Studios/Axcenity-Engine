---
title: "ADR-0048: A base model's material draws a library colour map chosen by the character's parameters"
type: adr
adr: 48
adr_status: Accepted
date: 2026-11-24
supersedes:
superseded_by:
affects: [avatar, gameplay, scene, render, core, project, assets]
source: [crates/render/src/draw_list.rs, crates/render/src/shader.wgsl, crates/avatar/src/manifest.rs, crates/avatar/src/resolve.rs, crates/avatar/src/randomize.rs, crates/core/src/material_values.rs, crates/gameplay/src/description.rs, crates/scene/src/material.rs, crates/scene/src/draw_list.rs, crates/project/src/character_library.rs, assets/characters/human/skins.py, assets/characters/human/manifests.py]
docs: [docs/development/character-studio/architecture.md, assets/characters/human/README.md]
tags: [adr, characters, rendering, assets]
---

# ADR-0048: A base model's material draws a library colour map chosen by the character's parameters

- **Status:** Accepted (Paul, 2026-11-24, after it was built as stage 3 of [ADR-0047](ADR-0047-The-Realistic-Family-Is-Rebuilt-On-MakeHumans-Body-And-Wardrobe.md): "manifest-driven base-color texture swapping in the scene layer is clean, lightweight, and keeps the 30 MB base mesh file untouched")
- **Date:** 2026-11-24
- **Affected systems:** Character Data (`avatar`: a manifest field, the resolver), the applier (`gameplay`), material translation (`scene`), the library's copies (`project`). Adds to [ADR-0036](ADR-0036-Character-Shading-Models-With-Modulating-Material-Values.md); doesn't change it.

## Context and problem

MakeHuman ships 22 photographed skins: one for each of three ages, three ethnicities and two sexes, a second young Caucasian pair, and a young pair with a bodysuit painted on. All are laid out on the base mesh's own texture coordinates. The human base ([ADR-0047](ADR-0047-The-Realistic-Family-Is-Rebuilt-On-MakeHumans-Body-And-Wardrobe.md)) had one of them inside its model file, so every character wore a young man's skin.

Nothing in Character Data could name another texture. ADR-0036 made material values **modulate** a material's own maps and never replace them, because the only replacement the engine had, a SurfaceAppearance, replaces every map at once: on a skin it drops the normal maps and the makeup masks with the colour. Putting 22 skins in the model file would add about 11 MB to a 30 MB file that every character of the base loads whole.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Every skin inside the model file, one body mesh or material per skin | No engine change | The file grows by every skin; every character loads them all; a body per skin repeats 339 shapes |
| A SurfaceAppearance put inside the body by the applier | No `scene` change | Replaces every map: the skin loses its normal maps and makeup masks as soon as it has them (ADR-0036's reason) |
| A derived model per skin (a third `MeshId` tag beside hidden regions and baked identity) | No attribute | A copy of the body's geometry on the GPU per skin in use; three tags to compose |
| A `skin` slider per axis (age, ethnicity, sex) | Plain parameters | Three controls a person must keep in step with the body by hand; an old body with a young skin by default |
| **Library images listed in the manifest, each at the parameter values it was made for; the nearest is drawn, named on the MeshPart by an attribute** | The model file doesn't grow; only the skin in use is read; the skin follows the body type and the age by itself; the material keeps everything else | One new manifest field, one attribute, one place in `scene` that replaces a map |

## Decision

1. **A base manifest may list colour maps** (`material_maps`: `{material, colour_map, at}`): an image beside the manifest for one of the base's material slots, and the parameter values it was made for. They are files of the library, never part of the model file.
2. **The nearest is drawn.** For each material that has maps, `resolve` takes the one whose `at` is nearest the character's parameter values, each parameter's distance counted as a share of its range, the first of equals (`BaseManifest::material_maps_for`, `Resolved::colour_maps`). A parameter a map is "at" can be a slider or a choice: the human base's maps sit at the `skin` choice, the body type and the age. A combination nobody made (an old skin for the second Caucasian pair) falls to the nearest one that exists.
3. **The applier names it on the part, the scene draws it.** The body and the base's own parts get the attribute `Mat_<material>_colour_map` (a project path), beside the material values and handled with them (saved, copied, replicated, removed when stale). `scene` sets that image as the material's base colour map and changes nothing else, before the values modulate it. Until the image is read, and if it can't be, the model file's own colour map is drawn. A SurfaceAppearance on the same material still wins.
4. **Values still only modulate** (ADR-0036 stands). This is the one map a character replaces, and only with a map its base model offers: a description can't name a file.
5. **Randomize keeps to a choice's `random` span.** A parameter with choices and a `random` draws only the choices inside it, so a base can offer a skin (the painted bodysuit) that Randomize never draws. Without `random`, every choice is drawn as before.

## Reasons

- Measured on 2026-11-24: the 22 skins are 11 MB as 2048 by 2048 JPEGs beside the manifest; `HumanParts.glb` is byte for byte what it was (30 MB); a character reads one of them.
- The skin can't disagree with the body: the sex of the skin is the body type's, its age is the Age slider's (young up to a quarter of the slider, old from three quarters), with no control for either.
- Only the colour map is replaced, so the Skin shading model's normal maps and makeup masks (ADR-0042) will stay when the human base gets them.

## Consequences

- Every map of a material must be laid out as the model file's own colour map is. Nothing checks that; the library test checks only that each is an image the engine reads and that a material's maps are one size.
- Every map of a material should name the same parameters: one that names fewer is nearer than it should be.
- A base's maps are part of its package: `project::add_character_packages` and the capture tool's library copy take them. A character pack's file list (`toolbox::character_pack`) is its own and would have to list them.
- As first built, the skin changed in steps as the Age slider crossed a quarter and three quarters. Since the note below it fades.
- Ray traced reflections and GI see the chosen skin, as they see any material's colour map.
- Revisit if a second kind of map needs choosing (a normal map per age), or if blending two maps becomes worth a shader change.

## Notes

- **2026-11-24 (the polish pass Paul asked for the same day): a map can fade into its neighbour.** The step as the Age slider crossed a quarter and three quarters was the first fault logged against this decision. A map may now name one of its `at` parameters as `blend`: between its own value and its neighbour's (the map at the same values of every other parameter, on the side the character's value lies) the character draws a mix of the two, none of the other at a map's own value and half of each where they are as near (`BaseManifest::material_blend_for`, `Resolved::colour_blends`). The applier names the second map and its share beside the first (`Mat_<material>_colour_map_toward`, `_amount`), and the **Skin** shading model draws both (`render::SkinShading::second_colour`): the second map rides in the occlusion map's place, read as colour, its share in the material's uniform, mixed in linear light before the tone and the light (`shader.wgsl`). Other shading models draw the first map alone, as before. Decision 4 stands: a character still replaces no map but the colour map, with maps its base offers. Costs: a skin with an occlusion map of its own can't fade (none has one); each distinct share is its own material on the GPU while a slider is dragged; ray traced reflections see the nearer map only. The human base's skins all blend along `age`, and its second Caucasian pair is listed at the older ages with the first pair's skins so it ages into them.

## Implementation references

- `crates/avatar/src/manifest.rs` (`MaterialMap`, `material_maps_for`, `check_material_map`), `resolve.rs` (`Resolved::colour_maps`), `randomize.rs` (`draw_choice`)
- `crates/core/src/material_values.rs` (`MATERIAL_COLOUR_MAP`), `crates/gameplay/src/description.rs` (`set_material_values`), `crates/scene/src/material.rs` (`colour_map`)
- `assets/characters/human/skins.py` (the 22 skins and where each sits), `manifests.py` (the `skin` choice)
- Tests: `avatar` `a_material_draws_the_colour_map_nearest_the_characters_values`, `a_colour_map_with_mistakes_reports_each`, `shipped_library.rs` `the_human_bases_skins_follow_the_body_type_and_the_age`; `scene` `a_colour_map_takes_the_place_of_the_materials_own_and_keeps_the_rest`; `player` `the_human_bases_skin_is_a_colour_map_on_the_body_that_follows_its_sliders`; `editor` `the_human_bases_skin_is_a_choice_on_the_skin_page`
