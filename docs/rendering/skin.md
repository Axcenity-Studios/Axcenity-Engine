# Skin shading

`Shading::Skin` (`render::SkinShading`, `crates/render/src/skin.wgsl`): the shading model of characters' skin (ADR-0036, Character Studio R5). It replaces Standard shading's diffuse and specular light for a material marked skin, and leaves every other material as it was.

## How a material becomes skin

A model file says it: a glTF material whose `extras` hold `"axcenity_shading": "skin"` is read by `assets` as `ModelShading::Skin` (`model.rs`, `shading_of`), and `scene` draws it as `Shading::Skin` (`draw_list/materials.rs`, `model_material`). `"axcenity_defined_normal": {"index": <texture>}` names a second normal map, placed like the material's own (`ModelMaterial::defined_normal_texture`, `SkinShading::defined_normal`). Any other value of `axcenity_shading`, or none, is Standard, so older files and other viewers draw such a material as plain PBR with its own normal map. Cooked models keep both fields. The realistic body's build writes them (`assets/characters/realistic/skin.py`, `mark`), and a test holds that the realistic `Skin` material is marked and that every other material isn't (`assets` `model/tests.rs`).

Why the model file and not an attribute the applier writes: the second normal map has to come from the file anyway; the file then draws the same wherever it is drawn (a MeshPart, the Model Studio, an asset preview), with or without a character description; and it follows the precedent of `axcenity_sway` and `axcenity_translucency`. The manifest's `materials[].shading` stays the contract for which values a slot takes (`avatar::check_values`).

## What it draws

- **Diffuse light** is lit by the normal map read blurred: at least a 6-texel footprint (`SKIN_DIFFUSE_LEVEL`, 2.5 mip levels, more with `scatter`), and never sharper than one level past the pixel's own footprint, so a face far away isn't grained by pixel-sized pores either. Red reads it a level and a half blurrier (`SKIN_RED_LEVELS`). Pores are smaller than the distance light travels under skin, so they never reach the diffuse light: this is what removes the stippled terminator of the Standard path.
- **Spreading across the line where light ends**: a table pre-integrated from human skin's diffusion profile (d'Eon and Luebke's six Gaussians, GPU Gems 3 chapter 14) over a ring of the surface's radius (Penner and Borshukov, "Pre-Integrated Skin Shading", GPU Pro 2), 7 rows from 2 to 128 mm by 33 columns of the cosine to the light, red, green and blue (`SKIN_TABLE`). The radius is the surface's curvature measured from the screen-space change of its own (geometric) normal over its position (`skin_footprint`), divided by `scatter`; a stud is 0.35 m (`SKIN_MM_PER_UNIT`). On a nose or a lip red reaches past the terminator; on a forehead or a back it is nearly Lambert's. The render test `the_skin_table_is_skins_profile_pre_integrated` recomputes the table and prints it again if it is wrong.
- **Shadow edges** are redder: each channel of a light's visibility is raised to `SKIN_PENUMBRA` (0.55, 0.85, 0.95) to the power of `scatter`, so the penumbra is widened most in red.
- **Specular light**: two GGX lobes on the full normal map, 0.75 and 1.3 times the material's roughness, the sharp one 15% (Unreal's dual specular defaults), reflecting 2.8% head-on in white (index of refraction 1.4). Metalness and the specular map are left out. Their roughness is widened by how fast the normal turns within a pixel (`skin_widened`, Kaplanyan and others 2016, as Filament does it), so pores don't glitter with TAA or without (MSAA). Bump shadowing (`bump_shadowing`) still applies to the specular light.
- **Sky and ambient light**: the diffuse part with the blurred normal, the reflection with the full one at the lobes' mean roughness.
- **The defined normal map**: `normal_blend` (0 to 1) mixes it into the material's own in tangent space before anything is lit, in `surface_normal` and for the blurred diffuse normals alike. It takes the specular map's slot (as a toon ramp does). 0, or no defined map, draws the own map bit for bit; 1 draws the defined one bit for bit.

The uniform (`MapsUniform::shading`): x `SHADING_SKIN` (3), y the defined map's share (0 without one), z `scatter`, w 1 if the defined map is BC5. Water and terrain ignore skin.

## What sets it

`scene::material::modulate` draws a character's values on it (architecture.md §10): `scatter_scale` multiplies `scatter` (0 to 2), and `definition × (1 − softness)` (each 0 to 1) is `normal_blend`. The realistic base manifest drives `definition` from `muscle` and `softness` from `fat`, so only lean, muscular bodies show the defined map's abdominal lines (OPEN-07).

## Cost

Measured with `bench_realistic_character` (1080p High with TAA, hard light, RX 9070 XT, medians of four runs; details in `docs/development/character-studio/refinement/r2/r5-skin-shading.md`): the Scene pass 0.625 to 0.660 ms with the whole body in view and about 0.65 to 0.71 ms with the face filling the view; no other pass changed. Skin pixels read the normal map four to six times (the own and the defined map, sharp and at two blurred levels) and the table eight times per light; other materials only test the uniform's model.

## Not yet

- No screen-space subsurface blur (Godot-style, CO17's quality phase): light doesn't bleed across shadow-casting geometry, only along a pixel's own curvature and penumbra.
- No transmission (ears and nostrils lit from behind).
- The curvature comes from screen-space derivatives of the interpolated normal, so it is constant per triangle; on the realistic body's 5 to 25 mm faces the change between neighbouring triangles is small, but a baked curvature map would be steadier.
- Per-texel pigment (lips, nails, palms toned apart from the skin) and makeup layers: R5's later steps.

## Tests

- `crates/render/src/renderer/tests/skin.rs`: the binding and uniform; red spreading past the terminator on a 6 mm ball (standard stays grey; no scatter, no red); a bumpy normal map's pores left out of skin's diffuse light (lit and at the terminator, against a smooth ball's floor); the normal blend at 0 and 1 drawing each map bit for bit; every other material (Standard, normal-mapped, Toon, Unlit, a translucent leaf card) drawing what it drew before the Skin model; the table.
- `crates/scene/src/material_values_tests.rs`, `draw_list/tests.rs`: a model marked skin draws as skin with its defined map; neutral values leave it as made; `scatter_scale`, `definition` and `softness`.
- `crates/player/src/skin_value_tests.rs`: neutral values draw the same pixels as none; `scatter_scale` 0 is rougher and 2 redder; `definition` 1 on a heavyset body brings the relief back.
