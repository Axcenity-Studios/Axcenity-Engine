# Hand-back: RA7 with RA5, `r5-skin-shading` (CO7's Skin model)

Task: the core of the Skin shading model, so skin stops looking like glittering grain (diffusion, two specular lobes, micro-roughness that doesn't glitter under TAA or MSAA); the grain halved; a smooth and a defined normal map blended by a material value the manifest drives from muscle and fat (OPEN-07). Branched from `refinement-r1-wave1` (`6267cb98`).

## What changed

**The Skin shading model** (`crates/render`; `docs/rendering/skin.md`):

- `Shading::Skin(SkinShading { scatter, defined_normal, normal_blend })`, drawn by `skin_surface` in the new `skin.wgsl` (included after `shader.wgsl` in every shader that includes it). Chosen as the cheapest that removes the grain and gives a soft, red-tinted terminator, with no new targets or passes:
  - **Diffuse** is lit by the normal map read blurred (`textureSampleLevel` at at least 2.5 mip levels, a 6-texel footprint, and never sharper than one level past the pixel's own footprint), red a level and a half blurrier; green and blue keep 10% and 20% of the full map's detail; red never falls below green (no cyan fringe in nostrils and ear folds).
  - **Pre-integrated scattering**: a 7 × 33 table (radius 2 to 128 mm by the cosine to the light) pre-integrated from d'Eon and Luebke's six-Gaussian skin profile over a ring (Penner), indexed by the surface's curvature from screen-space derivatives of its own normal divided by `scatter`. A render test recomputes the table from the profile and prints it if it drifts.
  - **Redder shadow edges**: each channel's visibility raised to (0.55, 0.85, 0.95)^scatter.
  - **Two GGX lobes** (0.75× and 1.3× the roughness, 15% sharp; Unreal's defaults), F0 0.028 in white, roughness widened by the normal's screen-space variance (Kaplanyan 2016, as Filament), bump shadowing kept on the specular light.
  - **The defined normal map** takes the specular map's slot (as a toon ramp does), and is mixed into the own map in tangent space before lighting (`mapped_normal`, shared by `surface_normal` and the blurred reads), written so blend 0 and 1 give each map bit for bit.
  - `surface_normal` was split into `unpacked_normal` and `mapped_normal` without changing any other material's arithmetic (the probe test holds every other material to its old pixels).
- **The contract** (wave 1 open issue 3): **glTF material `extras`**, `"axcenity_shading": "skin"` and `"axcenity_defined_normal": {"index": <texture>}`, read by `assets` (`shading_of`, `ModelMaterial::shading`, `ModelMaterial::defined_normal_texture`, kept by cooked models) and drawn by `scene` (`model_material`). Chosen over the applier's attribute because the defined map has to come from the file anyway, the model then draws alike wherever it is drawn (MeshPart, Model Studio, previews, with or without a description), and `axcenity_sway` is the precedent. The manifest's `materials[].shading` stays the value contract. `render` still sees only a `Material`. Recorded in architecture.md §10 and as a dated note on ADR-0036.
- **Values drawn** (`scene::material::modulate`): `scatter_scale` (× scatter, 0 to 2) and `definition × (1 − softness)` → `normal_blend`, on a Skin-shaded material only.

**Content** (`assets/characters/realistic`):

- `skin.py`: the grain's cells 2.6 → 5 texels, the fine wrinkling 9 → 12, the bump height kept at 0.09 texels, so its slope is about half (`GRAIN_*`). A **smooth** normal map (the material's own) flattens the sculpt's forms by 90% over soft tissue (`soft_tissue`: trunk, shoulders, arms, legs; not the head down to mid-neck, hands, feet, knees, elbows, nipples, nor creases deeper than an occlusion of 0.8 to 0.92, which keeps the navel); the **defined** map is the old one. `mark` adds the defined map to the exported GLB and writes the `extras` on `Skin`. `Realistic.build.json` records `grain` and `smoothed_share` (0.475 of the texels).
- `build.py`: one line, calls `skin.mark` after the export.
- `manifests.py`: exactly the driver asked for, `SKIN_DRIVERS`: `muscle` drives Skin `definition` (from −0.5..0.8 to 0..1), `fat` drives `softness` (from −0.5..0.5 to 0..1). The cast: athletic 0.83 (masculine) and 0.76 (feminine), average 0.19, slim 0.23, heavyset 0, heavy-muscular 0.

**Shared contract change**: `avatar`'s Skin value set gains `definition` and `softness` (0 to 1, neutral 0: optional additions, ADR-0036's consequences), needed for the driver to validate. No format change.

**Capture tool** (addition only): `AXCENITY_CAPTURE_AA=msaa|fxaa|off` draws with that anti-aliasing instead of TAA; unset, nothing changes (VIEWS, LIGHTS, MORE_VIEWS, `Settings::default` untouched; `captures.json` says which).

## Interfaces

- `render`: `Shading::Skin(SkinShading)`, `SkinShading { scatter: f32, defined_normal: Option<Arc<dyn TextureSource>>, normal_blend: f32 }` (exported). `MapsUniform::shading` for skin: `[3, blend, scatter, defined_is_bc5]`; `SHADING_SKIN`. `lit_pixel` takes a `SkinFootprint` (computed before `discard` in `fs_main` and in `fs_prefilled`).
- `assets`: `ModelShading { Standard, Skin }` (exported), `ModelMaterial::shading`, `ModelMaterial::defined_normal_texture`.
- `avatar`: Skin values `definition`, `softness`.
- glTF: material `extras.axcenity_shading` (`"skin"`; anything else is Standard) and `extras.axcenity_defined_normal` (`{"index": n}`).

## Tests

Written first and seen failing: [r5-skin-shading-red-run.txt](r5-skin-shading-red-run.txt) (render 4 of 5 failed, the fifth is the guard; assets, scene and avatar 6 of 23 failed; player 2 of 3).

| Test | What it proves |
|---|---|
| `render` `skin::a_skin_materials_shading_picks_its_binding_and_its_defined_map_takes_the_specular_maps_place` | the uniform and slot; no blend without a defined map; clamping |
| `render` `skin::light_spreads_past_where_it_ends_on_skin_further_in_red` | on a 6 mm ball red/green is 1.20 where the light ends (standard 1.00; scatter 0, 1.00), and more of the row is lit (GPU) |
| `render` `skin::a_bumpy_normal_maps_pores_dont_grain_skins_diffuse_light` | of the grain a bumpy normal map adds to a smooth ball, skin keeps 18% lit and none where the light ends (GPU) |
| `render` `skin::the_normal_blend_draws_the_own_map_at_0_and_the_defined_one_at_1` | identical pixels at 0 and 1, between at 0.5 (GPU) |
| `render` `skin::every_other_material_draws_as_it_did_before_the_skin_model` | Standard, normal-mapped, Toon, Unlit and a translucent leaf card read what they read on the base (the wave 1 leaf-probe style; numbers measured on the base) |
| `render` `skin::the_skin_table_is_skins_profile_pre_integrated` | the WGSL table is the profile pre-integrated |
| `assets` `a_gltf_materials_extras_say_it_is_skin_and_name_its_defined_normal_map`, `the_realistic_skin_is_marked_skin_with_a_smooth_and_a_defined_normal_map` | the parser; the built file: only `Skin` is skin, the two maps differ |
| `scene` `a_model_material_marked_skin_draws_with_skin_shading_and_its_defined_normal_map`, `neutral_values_leave_skin_shading_as_the_model_made_it`, `scatter_scale_multiplies_how_far_light_spreads_under_the_skin`, `definition_shows_the_defined_normal_map_unless_softness_hides_it` | the translation and the values |
| `avatar` `muscle_and_fat_drive_how_much_of_the_skins_definition_shows` | the manifest's drivers on the cast's bodies |
| `player` `scatter_scale_softens_and_reddens_the_skin`, `definition_and_softness_show_and_hide_the_defined_normal_map` | in captures: scatter 0 rougher (0.0148 against 0.0095), 2 redder; definition 1 on both heavyset bodies brings relief back |
| `player` `neutral_skin_and_eye_values_draw_the_same_pixels_as_none` (wave 1) | still passes with the Skin model |

Changed in wave 1's `skin_surface_and_eye_values_change_the_picture_the_way_they_mean`: "detail 0 changed" more than 2% → 0.5% of the picture (it now moves 1.3%: diffuse light no longer sees pores and the grain is half as steep); its "smoother" assertion is unchanged and passes.

Summary lines (debug, this worktree, RX 9070 XT):

- `cargo nextest run -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-player --no-fail-fast`: `Summary [ 379.533s] 1073 tests run: 1073 passed (8 slow), 25 skipped` (includes `every_example_renders` and every test above)
- `cargo clippy -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings`: clean
- `cargo check -p axcenity-editor -p axcenity-mesh-edit --tests`: builds (no other crate needed a change for the new `ModelMaterial` fields)
- `cargo fmt --all -- --check`: clean. `python scripts/check-wiki.py`: 113 pages, 0 errors, 58 warnings (all "may be stale" on pages this change doesn't touch).

## GPU time (FramePass timers, release, RX 9070 XT)

`cargo test -p axcenity-player --release -- --ignored bench_realistic_character --nocapture` (1080p High, TAA, hard light; four runs each, other agents' builds running, so the face view's before is noisy):

| View | Pass | Before (base) | After |
|---|---|---|---|
| front | Scene | 0.621, 0.633, 0.621, 0.629 ms | 0.660, 0.662, 0.659, 0.659 ms |
| front | frame | 1.237, 1.262, 1.239, 1.252 ms | 1.276, 1.281, 1.275, 1.275 ms |
| face | Scene | 0.743, 0.654, 0.642, 0.794 ms | 0.711, 0.713, 0.711, 0.712 ms |
| face | frame | 1.481, 1.299, 1.279, 1.601 ms | 1.343, 1.351, 1.346, 1.347 ms |

Shadows, prepass, ambient occlusion and post unchanged (0.076 to 0.094, 0.18 to 0.23, 0.09 to 0.10, 0.25 to 0.30 ms). The Skin model costs about **+0.035 ms** in the Scene pass with the whole body in view and **+0.06 ms** with the face filling it (medians 0.625 → 0.660 and about 0.65 → 0.712).

## Visual evidence and what I saw

Captures of the cast's average, heavyset and athletic on both frames, front, face-front and skin-close, under studio, hard, warm, cool and outdoor, with TAA and with MSAA 4×, before (the base) and after; chosen sheets in [../r5/](../r5/README.md) (5.4 MB). The full captures are outside the repository; to make them again:

```text
AXCENITY_CAPTURE_DESCRIPTIONS="<the six cast files, ;-separated>" AXCENITY_CAPTURE_LIGHTS=studio,hard,warm,cool,outdoor \
AXCENITY_CAPTURE_VIEWS=front,face-front,skin-close [AXCENITY_CAPTURE_AA=msaa] AXCENITY_CAPTURE_DIR=<folder> \
  cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
```

Looked at at full size and in 1.5× and 2× crops:

- **The stippled terminator on forehead, nose and chin is gone**, under TAA and under MSAA alike: the shadow line is a soft gradient with a slight warm (red) band, where before it was a ragged pixel stipple. Lit skin no longer glitters.
- **Not plastic, not wax, mostly**: the orange-peel grain is gone from diffuse light; close up the pores remain as a faint polygonal network in the highlights (forehead, cheek), which reads as skin texture at a metre. Under the soft studio light the cheeks look a little smooth and matte; with the hard and outdoor lights the face reads well. It is still a generated texture: no real pore pattern, so it doesn't reach a scanned look.
- **Heavy bellies show no abdominal lines** on either frame (bellies sheet). The average bodies show almost none; the athletic ones keep theirs, more softly than before, because diffuse light now sees the defined map blurred.
- Every light: warm and outdoor look natural; cool is cool and soft; nothing turned orange.
- Residual artefacts: a small, saturated red spot where the nose's shadow meets the upper lip (MSAA and TAA) and at the nostril's corner in skin-close, from high curvature plus the redder penumbra; the heavyset navel is faint (the occlusion test keeps it, the flattening around it softens it).

## Dependencies added

None.

## Open issues

1. The red spot at the nose's shadow on the upper lip and in the nostril corner (above). A cap on the curvature at the smallest radius or a gentler red penumbra would tame it; I left the physical values.
2. Curvature comes from screen-space derivatives, constant per triangle; a baked curvature map (a texture slot) would be steadier.
3. No screen-space SSS (light across shadow-casting geometry) and no transmission (ears and nostrils from behind): CO17's quality phase.
4. The grain's Voronoi cells still show as a faint polygonal network in close highlights; a real pore pattern (or a tiling detail normal, references/skin §2) is RA5's next content step.
5. OPEN-42 is not addressed: one detail map still serves both frames.
6. The smooth map flattens soft tissue uniformly by 90%; fat isn't regional in the normal maps (the regional fat sliders don't move `softness`).
7. The studio's Skin page (RA1's `panel/materials.rs`) still says "not drawn yet" for every value but tint and roughness (`DRAWN_TODAY`), and it now also lists `definition` and `softness` as sliders, which override the body's drivers when set. RA1 should mark all Skin values drawn and hide or explain the two driven ones.
8. Wave 1's MSAA contact-shadow hatching (a limb's false contact shadow) is unchanged: this branch fixes the grain, not that.
9. `audit-plan-status.md` OPEN-07 can be marked done by its owner (the lead).

## For the lead at merge

- **Regenerated by me** (commit includes them): `RealisticParts.glb` (40.7 MB, was 37.0: the second normal map), `Realistic.base.json` (the two drivers only), `Realistic.build.json` (`textures.grain`, `smoothed_share`). Built with `python -I assets/characters/realistic/build.py --out <dir>` (bpy 5.3) and copied in; `catalogue.py` gave no change; items, presets and benchmarks unchanged. If RA2 or RA3 changed the same files, rebuild with `build.py` and `catalogue.py` after merging: my changes are all in `skin.py`, one line of `build.py` and `SKIN_DRIVERS` in `manifests.py`.
- `shader.wgsl`'s `surface_normal` and `lit_pixel` changed signatures (CO8 rewrites `shader.wgsl`: keep `mapped_normal`, `skin_footprint` before `discard`, and the `SHADING_SKIN` branch). `skin.wgsl` must be included wherever `shader.wgsl` is.
- `avatar`'s Skin value set grew by two values: the editor's Skin page shows them (open issue 7).
- `character_capture.rs` gained `AXCENITY_CAPTURE_AA` (RA6's file, addition only).
