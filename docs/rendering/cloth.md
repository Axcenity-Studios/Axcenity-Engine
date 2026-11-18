# Cloth shading

`Shading::Cloth` (`render::ClothShading`, `crates/render/src/cloth.wgsl`): the shading model of fabric (ADR-0036, Character Studio CO7, R7). It draws the material's own light under a sheen layer, which brightens toward grazing angles, and can stretch the highlight along a direction. Every other material draws as it did. This page is the **material contract** a garment is built to.

## How a material becomes cloth

A model file says it: a glTF material whose `extras` hold `"axcenity_shading": "cloth"` is read by `assets` as `ModelShading::Cloth` and drawn by `scene` as `Shading::Cloth` (`draw_list/materials.rs`). Its sheen and anisotropy come from glTF's own extensions, which Blender's exporter writes from the Principled BSDF:

| Where | What | Notes |
|---|---|---|
| `extensions.KHR_materials_sheen` | `sheenColorFactor` (linear RGB) and `sheenRoughnessFactor` (0 to 1) | Blender: Sheen Weight × Sheen Tint, Sheen Roughness. Without it the cloth has no sheen. Its textures (`sheenColorTexture`, `sheenRoughnessTexture`) aren't read yet |
| `extensions.KHR_materials_anisotropy` | `anisotropyStrength` (0 to 1) and `anisotropyRotation` (radians, from the tangent) | Blender: Anisotropic and its rotation, but not the same stretch (see Anisotropy below). Its texture isn't read yet |
| mesh tangents | MikkTSpace | The direction the highlight stretches along is the texture's U, turned by the rotation |
| everything else | glTF's metallic-roughness material, as Standard reads it | Base colour, normal, roughness, occlusion maps |

`assets` reads both extensions' factors from the file's JSON for every material (`material_extensions`; the `gltf` crate drops extensions it doesn't know). A Standard material ignores them, so the realistic body's underwear, which already writes `KHR_materials_sheen`, draws as before until its file marks it cloth.

**Suggested looks** (starting points for the first garments; the content's own measurements win): cotton jersey sheen 0.15 to 0.25 grey, roughness 0.6; denim 0.1, 0.7; fleece and velvet 0.5 to 1.0 of the base colour's hue, 0.3 to 0.5; satin and silk sheen 0.1, with anisotropy 0.5 to 0.8 along the weave and a low base roughness (0.25 to 0.35).

## The values (ADR-0036's Cloth model)

`avatar::ShadingModel::Cloth`'s values, drawn live by `scene::material::modulate` (`set_cloth`) over what the file authored:

| Value | Range (neutral) | What it does |
|---|---|---|
| `tint` | colour (white) | Multiplies the base colour (as Standard) |
| `roughness_scale` | 0.5 to 1.5 (1) | Multiplies roughness |
| `sheen_tint` | colour (white) | Multiplies the authored sheen colour |
| `sheen_roughness_scale` | 0.5 to 1.5 (1) | Multiplies the sheen's roughness |
| `anisotropy_scale` | 0 to 2 (1) | Multiplies the authored anisotropy (held to -1..1) |
| `anisotropy_rotation` | -90 to 90 (0) | Turns the stretch direction, degrees from the authored (a stretched highlight repeats every half turn, so a wider range would only repeat itself: review fixes 2) |

The uniform (`MapsUniform`): `shading` = (`SHADING_CLOTH` 5, the sheen's roughness, the anisotropy, its turn in radians); `toon.xyz` the sheen's colour in linear light (cloth has no toon values). Cloth leaves `params` to a surface shader.

## What it draws

- **Sheen**: the Charlie distribution (Estevez and Kulla, "Production Friendly Microfacet Sheen BRDF", 2017; its alpha is the sheen roughness squared, as glTF has it) with Neubelt and Pettineo's visibility, as glTF's `KHR_materials_sheen` defines it. Fibres standing off the cloth catch light skimming it: next to nothing head-on, brightest toward the silhouette.
- **The layer under it**: the material's own diffuse and GGX specular light (Standard's `shade`), dimmed by what the sheen reflects, `1 - max(sheen) × E(n·v)`, where `E` is the sheen's directional albedo, a table integrated from the lobe over the hemisphere (`CLOTH_SHEEN_ALBEDO`, 8 roughnesses by 16 angles; the render test `the_sheen_albedo_table_is_charlies_integrated` recomputes it). So a sheen never adds light that wasn't there; head-on it takes 0.3% (roughness 0.5) to 6% (roughness 1). `KHR_materials_sheen` dims by the smaller of `1 - max(sheen) × E(n·v)` and `1 - max(sheen) × E(n·l)`; the engine uses the view's term only, so under a grazing light the layer under the sheen is up to a few per cent brighter than the spec's (where E(n·l) is large and E(n·v) small). Follow the spec if a garment's grazing-lit edge reads too bright against Blender.
- **Anisotropy**: with any, the specular lobe is anisotropic GGX (Burley 2012, with Heitz's height-correlated visibility, as Filament has it): alpha along the direction `α(1 + a)`, across it `α(1 - a)`; a positive value stretches the highlight along the direction, a negative one across it. The sky's reflection is bent toward the stretched direction. With none the lobe is Standard's own. This is Filament's mapping, not the glTF extension's: `KHR_materials_anisotropy` widens only along the direction, `α_t = mix(α, 1, strength²)` with `α_b = α`, and Blender's Principled BSDF has its own. The strength is passed straight through, so the same number stretches a highlight differently here than in the glTF spec or in Blender: judge a garment's anisotropy in the engine (a capture), not in Blender's viewport.
- **Sky and ambient light**: the layer under the sheen as Standard lights it, dimmed as above, plus the sheen's albedo times the sky round the normal.

With no sheen and no anisotropy, cloth draws exactly what Standard draws (`sheen_brightens_cloth_at_grazing_angles_and_not_head_on` checks it within 3 of 255).

A **fuzz or nap** term (Zeltner and others' LTC sheen with back-scattering, or OpenPBR's fuzz layer) isn't drawn: Charlie's lobe already gives the soft rim, and nothing on the cast yet needs more. Worth measuring again on the first fleece or velvet.

## Not yet

- The extensions' textures (sheen colour and roughness maps, the anisotropy direction map).
- No subsurface or wrapped diffuse for thin fabric lit from behind.
- Ray traced reflections and GI see cloth as Standard.

## Cost

`bench_hair_and_cloth_models` (render tests, ignored; RX 9070 XT, 1080p High with TAA, a ball filling the view, medians of 200 frames): the Scene pass costs 0.444 ms with sheen and 0.451 ms with sheen and anisotropy, against 0.432 ms drawn Standard (+0.01 and +0.02 ms).

## Tests

- `crates/render/src/renderer/tests/cloth.rs`: the uniform; sheen adding next to nothing head-on and over 25 of 255 at the rim; no sheen drawing Standard; anisotropy 0.8 stretching the highlight 1.6 times along the tangent, and along the turned direction at a quarter turn; the albedo table.
- `crates/render/src/renderer/tests/skin.rs`: `every_other_material_draws_as_it_did_before_the_skin_model` (Standard, normal-mapped, Toon, Unlit, a leaf card) still holds.
- `crates/assets/src/model/tests.rs`, `cooked_model.rs`: the extensions read (clamped, a broken one left out), cooked models keep them.
- `crates/scene/src/material_values_tests.rs`, `draw_list/tests.rs`: the values modulate the authored sheen and anisotropy; a model marked cloth draws its extensions; a Standard material ignores them.
