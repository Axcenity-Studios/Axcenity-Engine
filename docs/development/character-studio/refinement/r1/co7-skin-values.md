# Hand-back: RA7, `co7-skin-values` (CO7's first slice)

Task: the renderer draws the character material values it stores (the Skin values first), and the worst skin shading artefacts of the baseline are found and fixed. Branched from `character-creator-stage0` (`5b650d5d`).

## What changed

**Skin and eye values drawn** (`crates/scene`, the material translation; no change to `avatar`, the applier or any asset):

- `melanin`, `undertone`, `redness` tone the base colour by one ratio in linear light (`scene/src/skin_tone.rs`). The curve is the tone table of `references/skin/README.md` §1.3 (nine depths by cool, neutral, warm and olive), interpolated in optical density (Beer and Lambert: pigments add there, so melanin darkens blue more than red at the light end and deep tones keep their colour), redness per unit taken from the olive column. Past the table's ends the curve slows down (one depth step past the light end at most, two past the deep end). All three at 0 give exactly (1, 1, 1) and the colour is left bit for bit. This is the references' "step A" (one ratio per material, on the CPU): free per item, no texture, binding or shader change, and it respects ADR-0003 (render only sees a `Material`).
- `sclera_tint` multiplies like `tint` and `iris_tint`; `detail_strength` multiplies the normal map's strength (held to 0 to 2; a distinct strength is a distinct material bind group, as any normal scale is).
- Not drawn, documented: `scatter_scale` (needs the Skin shading model); `pupil_size` (the Iris map is radial around (0.5, 0.5) with the pupil at 0.35 of the iris radius, so a shader path could rescale it, but it needs an Eye branch in the shader; the realistic eye already has the `Pupil_Size` shape, and a manifest driver on that shape is cheaper); `limbal_ring` and `iris_depth` (Eye model); cloth sheen and anisotropy, hair dye, tip lightening, highlight.

**Shadow and shading fixes** (`crates/render`):

- `shading_noise` (lighting.wgsl): the shadow filters' per-pixel Poisson turn, contact shadows' step jitter and the lamps' shadow turn move on each frame while TAA is on (`Frame::noise.x` = TAA frame index mod 64; `History::frame_index`). Cause found: interleaved gradient noise of the pixel alone stood still under TAA's jitter, so TAA kept it; a nearby arm's contact shadow showed as diagonal hatching (baseline H3). Without TAA nothing changes, so under the High and Ultra presets' default MSAA 4× the hatching still shows (see Review fixes).
- `bump_shadowing` (lighting.wgsl, in `shade_surface` for the sun and every lamp): the microfacet shadowing for bump maps of Conty Estevez, Lecocq and Stein (Ray Tracing Gems, 2019), plus no light where the surface's own normal faces away. Only for surfaces with a normal map (`bumped`, added in the review fixes below; the first version also darkened leaves, water and terrain). Cuts the noise across the terminator on the skin by 31% (baseline H2, in part).
- Tried and dropped: specular anti-aliasing (Kaplanyan 2016, as Filament does it) cut the lit skin's grain by 3% only; not worth a change to every glossy surface.

## Interfaces

- `render`: `FrameUniform::noise: [f32; 4]` appended (lighting.wgsl `Frame::noise`); `History::frame_index()`. No public API change.
- `scene`: `material::modulate` draws the values above; new private module `skin_tone` (`ratio(melanin, undertone, redness) -> [f32; 3]`, `srgb_to_linear`, `linear_to_srgb`). To use the build's own table later, only `TONES` changes.
- `player`: `character_capture::stage_world` is now `pub(crate)` (used by the new benchmark; nothing else changed in that file: VIEWS, LIGHTS and `Settings::default` untouched).

## Tests

Written first and seen failing: [co7-skin-values-red-run.txt](co7-skin-values-red-run.txt) (scene: 7 of 11 failed; player: 2 of 3 failed, "front: melanin 0.8 changed 0", "detail 0 changed 0").

| Test | What it proves |
|---|---|
| `scene` `skin_tone::tests` (5) | neutral is exactly 1; melanin darkens monotonically over -1 to 1, blue more than red at the light end; the table's 36 entries come back within 0.002; redness reddens, undertone warms; every combination stays within 0.005 to 0.82 linear on the authored tone |
| `scene` `material_values_tests` (6) | neutral values leave the material identical; melanin darker and lighter; redness redder, undertone warmer and cooler; a tint still multiplies after toning; `detail_strength` scales the normal map and keeps it; `sclera_tint` multiplies |
| `player` `skin_value_tests::neutral_skin_and_eye_values_draw_the_same_pixels_as_none` | every value of Skin, Sclera and Iris at neutral draws the identical 12 pictures as a description with none |
| `player` `skin_value_tests::skin_tone_values_change_the_picture_the_way_they_mean` | in the front and face-front captures: melanin 0.8 darker (luminance below 0.6×), -0.8 lighter (above 1.15×), redness 0.6 redder, undertone ±0.7 warmer and cooler |
| `player` `skin_value_tests::skin_surface_and_eye_values_change_the_picture_the_way_they_mean` | roughness 0.5 brightens the highlights; detail 0 smoother, detail 2 rougher; a pink sclera tint makes the whites pinker |
| `player` `skin_value_tests::bench_realistic_character` (ignored, a tool) | GPU time per `FramePass` for the realistic character, 1080p High with TAA, hard light |

Summary lines (debug, this worktree, RX 9070 XT):

- `cargo nextest run -p axcenity-render -p axcenity-scene --no-fail-fast`: `Summary [ 200.140s] 562 tests run: 562 passed (2 slow), 6 skipped`
- `cargo nextest run -p axcenity-player --no-fail-fast`: `Summary [  64.318s] 177 tests run: 177 passed (1 slow), 19 skipped` (includes `every_example_renders` and `material_values_modulate_the_material_and_keep_its_maps`)
- `cargo nextest run -p axcenity-scene --no-fail-fast material_values_tests skin_tone`: `Summary [   0.204s] 11 tests run: 11 passed, 186 skipped`
- `cargo nextest run -p axcenity-player --no-fail-fast skin_value_tests`: `Summary [  59.475s] 3 tests run: 3 passed (2 slow), 193 skipped`
- `cargo clippy -p axcenity-render -p axcenity-scene -p axcenity-player --all-targets -- -D warnings`: clean. `cargo fmt --all`: applied.
- `python scripts/check-wiki.py`: 0 errors (58 warnings, all "may be stale" on pages this change doesn't touch).

## GPU time (FramePass timers, median of 240 frames, release)

`cargo test -p axcenity-player --release -- --ignored bench_realistic_character --nocapture`

| View | Pass | Before | After (two runs) |
|---|---|---|---|
| front | Scene | 0.617 ms | 0.620, 0.618 ms |
| front | Shadows | 0.078 ms | 0.079, 0.074 ms |
| front | frame | 1.233 ms | 1.231, 1.214 ms |
| face | Scene | 0.637 ms | 0.636, 0.637 ms |
| face | frame | 1.272 ms | 1.259, 1.257 ms |

No measurable change. The skin tone is CPU work per item (one table lookup when a part has pigment values).

## Visual evidence

Committed under [../co7/](../co7/README.md) (6.8 MB): the five skin values on both frames (studio face-front, hard face, feminine full body), before and after crops of the hatched limb shadow and the face terminator, the default bodies' contact sheets before and after, and the experiments that found each cause. What I saw, honestly:

- The skin values do what they say. `melanin` 0.8 is a convincing deep brown but darkens the lips with the skin (one ratio per material). `melanin` -0.8 is very fair and **lavender-pink**, cooler than natural fair skin: the table's light end needs the build's recomputed table. Redness and undertone are visible but modest.
- The hatched limb shadow is gone in both frames' stride under the hard light, under TAA (the captures use TAA). Under MSAA it still shows.
- The skin is **still grainy** and the terminator still speckled, less than before. The experiments show why: the speckle stays with every shadow off and goes with a flat normal map. The skin normal map's grain (cells about 2.6 texels, near a pixel in the face view) lit by Standard shading is the cause; the content (`skin.py`'s grain strength, RA3/RA5) and the Skin shading model (pre-integrated diffusion) are the fixes, not a renderer bug.
- OPEN-09, the chest band: diagnosed, not fixed. It stays with the body left out of the shadow map (contact shadows draw it as well) and goes with every shadow off; its edge follows the facets of the chest. It is light grazing the upper chest: the shadow terminator of a coarse mesh with smooth vertex normals (both shadow techniques see the triangles, the shading sees the smooth normal). A fix is a receiver offset toward the smooth surface (Chiang, Li and Burley's "Taming the Shadow Terminator", 2019) or a larger grazing normal offset, both global shadow changes I did not make without the lead.

## Dependencies added

None.

## Open issues

1. Per-texel toning (lips, nails, palms and soles keeping their pigment) needs the Skin shading model and masks (references/skin §1.2 step B).
2. The tone table is indicative; RA5's build should recompute it from the pigment spectra (only `TONES` in `skin_tone.rs` changes). The light end is lavender-pink.
3. `scene` can't tell a slot's shading model (audit-runtime-render "Shared contracts"); Hair's `melanin` and `redness` take the skin curve meanwhile. The lead's contract decision (an `extras` field in the glTF material or an attribute from the applier) settles it.
4. `pupil_size`, `limbal_ring`, `iris_depth`, `scatter_scale` not drawn (above).
5. Skin grain is content strength plus no subsurface softening (above). Suggest RA3/RA5 halve the grain bump or move its cells to 4 or more texels; `detail_strength` 0.5 is a per-character stopgap.
6. Without TAA (the presets' MSAA 4×) contact shadows keep their fixed per-pixel pattern; a limb a few centimetres in front of the body can still cast a false screen-space contact shadow (softened, not hatched, under TAA).
7. OPEN-09 (above).

## For the lead at merge

- `crates/render/src/lighting.wgsl` and `renderer/types.rs` gain `Frame::noise` at the end of the uniform: merge in sequence with any other change to the `Frame` struct (CO8 rewrites `shader.wgsl`, not this struct).
- No generated files touched; nothing to rebuild.
- `docs/development/character-studio/refinement/baseline/README.md` H2, H3, H4 and `audit-plan-status.md` OPEN-09 can point at `co7/README.md`; I left both files to their owners.

## Review fixes

The review found two blocking problems. Both are fixed, with tests written first and seen failing (appended to [co7-skin-values-red-run.txt](co7-skin-values-red-run.txt)).

**1. `bump_shadowing` darkened every leaned shading normal, not just normal maps.** The review was right. The term multiplied every light in `shade_surface`, so leaves' crown-bent normal (`lit_leaf`, `CANOPY_BEND`), water's waves, decals, terrain and surface shaders lost light wherever their shading normal leaned from the geometric one. Fix: `light_surface` and `shade_surface` take `bumped: bool`, and `bumps()` applies `bump_shadowing` only when it is set.
- The standard item shader passes `maps.settings.x != 0.0`, which is the normal map's scale and 0 without a normal map, so surface shaders follow their material's map too.
- `lit_leaf`, `water.wgsl`, `decal.wgsl` and `terrain.wgsl` pass `false`. Terrain's choice is deliberate. Its layers do have normal maps, but nothing showed a terrain terminator problem, and the term would darken grazing-lit slopes in every outdoor scene without evidence, so terrain looks as it did before the change.
- Characters are unaffected: skin has a normal map, and only plants' glTF extras give a material translucency.

Tests in `crates/render/src/renderer/tests/shading.rs`:
- `a_leafs_sunlit_crown_edge_is_lit_whichever_side_of_its_card_the_sun_is` is the review's probe: a 4.4-stud translucent leaf card, shadows off, sun 2, green at the crown edge, x=25 of 32. It reads 45 (sun side-behind) against 84 (side-front) on the branch as handed in. After the fix it reads 104 against 103, the same as before the change.
- `a_surface_without_a_normal_map_is_lit_by_its_own_shading_normal`: a surface shader leans the normal 70° toward a sun slightly behind the card. The card was black on the branch and is lit now.
- `a_normal_mapped_surface_keeps_a_clean_terminator_under_a_grazing_sun`: a sphere with a random-bump normal map, lit from the side. No pixels are lit past its own terminator: 0 with the term, 21 of 558 without. Pixel-to-pixel noise across the terminator is 25 with the term and 38 without. This test fails against the base commit's shader.
- The review asked for "a Standard surface without a normal map draws identical pixels". With n equal to the geometric normal the term was already 1, so such a test could not fail on either shader. The surface-shader test above covers the same claim (no normal map, so no change) and does fail on the buggy shader.

**2. The render-side changes had no tests.** `ScenePipeline::shading_noise_step` now holds the TAA-only rule that `scene.rs` had inline, and the frame uniform reads from it, so the behaviour is unchanged. Tests in `point_and_contact_shadows.rs`:
- `with_taa_the_shadow_noise_moves_every_frame`: the step differs between consecutive TAA frames. It fails with the step pinned to 0, as at the base commit.
- `without_taa_soft_and_contact_shadow_edges_draw_the_same_pixels_every_frame`: Off, FXAA and MSAA 4×, with motion blur on so the history counts frames, and a block with soft sun shadows and contact shadows. Two still frames are bit-identical and the step is 0. With the noise moving regardless of TAA (a mutation) it fails, and the pixel comparison alone catches it: 99 pixels changed.

**Docs corrected:** `docs/rendering/shadows.md` (bump shadowing applies to normal maps only; the noise fix applies only under TAA), `wiki/Engine-Systems/Rendering.md`, `wiki/Roadmap/Known-Limitations.md`, and this file's own "unchanged without a normal map" and "hatching is gone" lines.

**Non-blocking review points:**
- The hatching fix works only under TAA. This is now said in every doc. The High and Ultra presets default to MSAA 4×, and Character Studio's viewport uses the editor's settings, so the baseline's H3 hatching still shows there. The root cause is a false screen-space contact shadow from a limb that is in front of the body from the camera but not toward the sun, and it passes the 0.5-stud thickness test. It is listed in Known limitations with the two possible fixes; I did not change contact shadows here.
- Not changed: the lavender light end of melanin (RA5's TONES), the 0.005 albedo floor test, Hair taking the skin curve, the still-ragged face terminator and OPEN-09. These remain as listed under Open issues.
- The `wind::a_swaying_plants_shadow_moves_as_far_as_the_plant` abort on exit (0xc0000409) did not recur in my full render run.

**Runs after the fixes:**
- `cargo nextest run -p axcenity-render --no-fail-fast`: Summary [142.376s] 373 tests run: 373 passed, 3 skipped
- `cargo nextest run -p axcenity-player -p axcenity-scene --no-fail-fast`: Summary [50.620s] 371 tests run: 371 passed (3 slow), 22 skipped (every_example_renders and the three skin_value_tests pass)
- `cargo clippy -p axcenity-render -p axcenity-scene -p axcenity-player --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean
- `python scripts/check-wiki.py`: 111 pages, 0 errors, 58 warnings (all "may be stale" on pages this change does not touch)
