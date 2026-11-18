# Hair shading

`Shading::Hair` (`render::HairShading`, `crates/render/src/hair.wgsl`): the shading model of hair cards (ADR-0036, Character Studio CO7, R6). It lights a card along its strands, not along its normal, and draws a cut-out hair card with hashed alpha under TAA and graded coverage under MSAA. Every other material draws as it did. This page is the **material contract** a hairstyle is built to: what marks a material hair, the atlas's channels, and the values a description sets.

## How a material becomes hair

A model file says it. A glTF material whose `extras` hold `"axcenity_shading": "hair"` is read by `assets` as `ModelShading::Hair` (`model.rs`, `shading_of`), and `scene` draws it as `Shading::Hair` (`draw_list/materials.rs`, `model_material`). Any other value, or none, is Standard, so another viewer draws the card as a plain PBR cut-out. Cooked models keep everything below.

| Where | What | Notes |
|---|---|---|
| `alphaMode` | `MASK`, with `alphaCutoff` (glTF's default 0.5) | A hair card is a cut-out. `BLEND` draws see-through (sorted, no shadow, no depth) as before |
| `doubleSided` | `true` for cards | Cards are seen from both sides, and lit from either (light from behind comes through). `false` makes a **shell**: a single-sided hair surface over something solid (a hair cap over the skull), whose light fades out over its terminator (`hair_shell`, below) |
| `pbrMetallicRoughness.baseColorTexture` | the atlas's colour (RGB, sRGB) and coverage (A) | The colour the values move. Paint it with the strands' own variation (lighter and darker strands) and a believable natural colour; a description moves it from there |
| `baseColorFactor` | multiplies it | |
| `roughnessFactor` (and a roughness map, if any) | the strands' longitudinal roughness | 0.3 to 0.45 reads as human hair. The lobes' widths are `roughness²` (R), half of it (TT) and twice it (TRT) |
| `extras.axcenity_hair_data` | `{"index": <texture>}`: the atlas's data map | Optional. Placed like the base colour map; read only where there is one |
| `extras.axcenity_hair_flow` | `{"index": <texture>}`: the flow map | Optional; without it the strands run along the texture's +V |
| mesh tangents | MikkTSpace (glTF `TANGENT`, or the engine's own) | The strand's direction is built from them |
| `metallicFactor` | ignored | Hair isn't metal; the metalness map's place holds the data map |

### The atlas's data map

One RGBA texture the size of the atlas (or smaller), read as plain values (linear, not sRGB), placed exactly like the colour map:

| Channel | Holds | Used for |
|---|---|---|
| R | **Root to tip**: 0 at the root, 1 at the tip, along each strand | The root colour (`root_melanin` over `root_length`) and lighter tips (`tip_lightening`) |
| G | **A value per strand** (or per clump, or card): any value 0 to 1, constant along the strand, different from the next | Each strand's highlight shift varies by ±30% (`HAIR_SHIFT_SPREAD`), so highlights break up; under TAA, crossing cards don't cut the same pixels (`hair_cut_threshold`) |
| B | **Streak mask**: 1 where a streak (highlight or lowlight) is, 0 elsewhere | `streaks` × `streak_melanin` |
| A | **Ambient occlusion of the strands**: 1 open, 0 deep in a clump | Multiplies sky and ambient light |

Without a data map every pixel is halfway along its strand (0.5), has the middle per-strand value, no streak and no occlusion of its own: the root, tip and streak values then change nothing.

### The flow map

RG: the strand's direction from root to tip in the card's tangent space, each `0.5 + 0.5 × d` (red along the texture's U, green along its V); B unused. `(0.5, 1.0)` is +V, the default. Read linear, placed like the colour map, bound in the specular map's place (hair reads no specular map). Use it when the cards' UVs don't run root to tip along V (curly strands drawn on a straight card, swirls at the crown).

### What a card should look like

What the shading assumes, and what the reviews look for (asset-quality.md §4):

- Strands run root to tip along V (or as the flow map says), tapering to the tip in the colour map's alpha.
- Alpha is whole (1) on the strands and clear (0) between them, with a soft fringe one or two texels wide. Under TAA only opacities between 0.2 and 0.8 are hashed (`HAIR_HASH_BAND`, round the cutoff of 0.5); fainter is cut, denser whole. A wide soft gradient flickers more than a narrow one (see Alpha cards).
- No lighting baked into the colour: the model adds both highlights.

## The values (ADR-0036's Hair model)

`avatar::ShadingModel::Hair`'s values, set by a description's `materials`, drawn live by `scene::material::modulate` (`set_hair`) into `HairShading`. A value's change re-binds the material (its uniform is part of the material key); no mesh is rebuilt (the render test checks `mesh_bytes`). Every neutral setting draws the material as made.

| Value | Range (neutral) | What it does |
|---|---|---|
| `melanin` | -1 to 1 (0) | Darker or lighter along the pigment curve: the colour's absorption times 2^(3 × melanin) (×8 at 1, ÷8 at -1) |
| `redness` | -1 to 1 (0) | Pheomelanin's share of the same melanin toward all of it (1) or none (-1), as Blender's "melanin redness": redder and lighter, or ashier |
| `dye`, `dye_amount` | colour (white), 0 to 1 (0) | How much of the natural colour a dye replaces |
| `tint` | colour (white) | Multiplies the colour last |
| `tip_lightening` | 0 to 1 (0) | Lighter toward the tips: up to -1.5 melanin at the tip, from 40% of the length |
| `root_melanin`, `root_length` | -1 to 1 (0); 0 to 1 (0.15) | A melanin offset at the root, fading out over that share of the strand (needs the data map) |
| `streaks`, `streak_melanin` | 0 to 1 (0); -1 to 1 (0) | The streak mask's melanin offset, by how much of it shows (needs the data map) |
| `highlight` | 0 to 2 (1) | The secondary (coloured, TRT) highlight's strength |
| `shift_scale` | 0 to 2 (1) | Multiplies the cuticle's tilt, which shifts both highlights along the strand |
| `roughness_scale` | 0.5 to 1.5 (1) | Multiplies roughness |

**The colour.** `hair_colour` turns the material's colour into absorption with Chiang, Bitterli, Tappan and Burley's mapping (2016, azimuthal roughness 0.3: the same one Blender's Principled Hair BSDF uses for "direct colouring"), splits it into eumelanin and pheomelanin by least squares on their absorption spectra (Donner and Jensen's, as pbrt v4 gives them: `(0.419, 0.697, 1.37)` and `(0.187, 0.4, 1.05)`; what neither explains is kept), moves them by the values and turns it back into a colour. With every value neutral the colour isn't touched. So one pair of numbers runs from blonde through red and brown to black, and skin and hair share one idea of pigment (references/materials).

The realistic body's `Brows` and `Lashes` are Hair slots in its manifest but their file doesn't mark them hair (they are blended cards, CO6): on them, `melanin` and `redness` still take the skin's curve on the base colour, as before.

## What it draws

A Marschner-style model in Karis's approximation ("Physically Based Hair Shading in Unreal", SIGGRAPH 2016), chosen over Kajiya-Kay with two shifted lobes because its lobes come from the fibre: the secondary's tint and the light through hair from behind follow from the same absorption, and it is the model the Blender references draw (Chiang's), so the two can be compared.

- **R** (primary): reflection off the cuticle, white, shifted toward the root by twice the cuticle's tilt (2°, `HAIR_TILT`, Blender's default), longitudinal width `roughness² × √2 × cos(φ/2)`.
- **TT**: through the fibre, tinted by its absorption, shifted toward the tip by the tilt, narrow; strong only when the light is behind the hair (the glow round a back-lit head).
- **TRT** (secondary): once round inside, shifted toward the tip by four times the tilt, twice as wide, tinted by the absorption raised to `0.8 / cos θd`: on brown hair a warm, dimmer second band below the primary.
- **Many strands**: a soft Kajiya-Kay term for light scattered among strands: `(1 - |sin θl|)` along the strand times a half-wrapped cosine round it (a normal facing the camera across the strand), a quarter as strong as Karis's (`HAIR_SCATTER` 0.25), more saturated in shadow. At Karis's full strength a ball of strands stayed 35 to 50% as bright 30° off its highlight where Blender's Chiang model falls to 3 to 10%: the hair read flat.
- **Sky and ambient light**: the colour lit by the sky round that normal, and a little rough reflection, times the data map's occlusion.
- **Shells** (single-sided hair, `hair_shell`, review fixes 1): no term above dims with the surface's own normal, so on a closed surface the strands past its terminator stayed lit and showed the shadow map's self-shadow edge there, torn by the filter's per-pixel noise (the back-lit ball of the evidence: a blotchy band 21 rows past its terminator, at 512 px). A single-sided hair material's sun and lamps fade over its terminator, from n·l 0.1 (whole) to -0.05 (none): the lit band now ends within 2.6 rows of the terminator with a smooth edge (`a_back_lit_hair_shells_lit_band_ends_smoothly_at_its_terminator`). Double-sided cards are untouched: a card is a sheet of strands, lit from either side.

The strand's direction (toward the root) is the flow map's direction, or +V, in the tangent frame of the item's own normal and tangent (not turned to face the camera, so both sides of a card agree). On a ball of strands the primary's peak lies 0.02 to 0.13 px from where the model puts it (tilt × 0, 1, 2: rows 121.67, 115.50, 109.67 against 121.54, 115.52, 109.58), and the secondary's 0.24 px (render tests).

The uniform (`MapsUniform`): `shading` = (`SHADING_HAIR` 4, maps bound: 1 data + 2 flow, + 4 for a single-sided shell, the tilt's scale, the secondary's strength); `params[0..4]` the colour values (`renderer/lighting.rs`, `hair_uniform`): melanin, redness, tip lightening, dye amount; the dye (linear), root melanin; root length, streaks, streak melanin; the tint (linear). A surface shader on a hair material would read these as its parameters: hair takes none.

## Alpha cards

A hair card is a cut-out (`alpha_cutoff` > 0). Its depth is drawn before the scene is lit (`fs_depth_only`, `fs_depth_coverage`), so only the nearest strand at a pixel is lit and nothing is sorted; the skin behind shows through its clear pixels.

- **Under TAA** (`Frame::noise.y` 1): hashed alpha. Each pixel's cut threshold is interleaved gradient noise stepped by the golden ratio each frame (`hair_cut_threshold`), over the band 0.2 to 0.8 round the cutoff, offset by the strand's data value so crossing cards cut different pixels; TAA averages the frames into the strands' coverage: soft edges, no stair-steps. The depth fill, the prepass (AO, contact shadows, ray traced shadows' pixels) and a surface shader's own pass all cut with the same threshold.
  - Measured (`a_hair_card_under_taa_holds_still_with_soft_edges`, a light brown card 3 studs square, 3 studs away, whose opacity ramps clear to whole, lit from the front, 16 frames after 32): the root mean square of each pixel's frame-to-frame deviation is 3.38 of 255 (a Standard cut-out, hard-edged, under the same TAA: 0.6). The limit is 4.5 (`TAA_VARIANCE_LIMIT`), provisional: asset-quality.md §4 has Paul set it at the first hairstyle's sign-off. With the earlier, brighter diffuse term the same card measured 4.98 hashed over the band, 5.73 hashed over every opacity, and 5.96 with Jimenez's frame step instead of the golden ratio's. What is left is the floor of a binary sample blended in a tenth at a time (`TAA_SHARE`).
  - Where it shows: at the band's bottom (opacity 0.2 to 0.3) single pixels come and go and TAA's neighbourhood clamp loses them: the largest deviations are there (11 to 15 of 255 per pixel along the test card's middle row, 3 to 5 times the average). Atlases with a narrow fringe show little of it.
- **Under MSAA**: alpha to coverage with the opacity itself as the coverage (`fs_depth_coverage`), so a soft fringe covers a graded share of each pixel's samples (5 levels at 4×). A Standard cut-out ramps its coverage over one pixel at its cutoff, as before. Because any opacity above 0.002 covers samples, an atlas must keep its clear texels exactly 0 and pad its islands (or leave a clear border of a few texels round each), so the texture filter and mip levels don't bleed a neighbouring island's alpha into a card's edge: the test ramp card's repeat-wrapped left edge shows such a bleed as a solid line under MSAA.
- **Off and FXAA**: cut at the cutoff, hard-edged. The card's shadow is still hashed over 0.2 to 0.8 (below), so a fringe between 0.2 and 0.5 that isn't drawn still casts a faint stippled shadow (visible left of the card in the evidence's Off panel).

### Shadows

- **Shadow maps** cast through a hair card's holes (`fs_shadow_cut_out`, the shadow pipeline for cut-out casters, which reads each caster's material; `Caster::cut_out`, `CasterGroup`). Only hair: a Standard cut-out (leaf cards, fences) casts its whole card as before, because casting foliage through its holes more than doubled the Shadows pass ([shadows.md](shadows.md): 0.19 to 0.42 ms for 36,000 leaf cards at 1080p High). A hair card's opacity is hashed against a fixed per-texel pattern over the same band round its cutoff (0.2 to 0.8), so the shadow filter averages a strand's soft fringe into a partial shadow; whole strands cast whole, clear gaps nothing: many strands, not a slab. On a card whose opacity ramps over its whole width (the evidence's test card) the hashed part shows as a faint stipple in the shadow's lit side; real atlases' fringes are a texel or two. The hashed pattern is fixed in shadow-map texel space, so on moving hair it may crawl: to check on the first hairstyle in Walk.
- **What casts its whole mesh**: every cut-out that isn't hair, and a hair card with a surface shader or a texture tiled by its size (`DrawItem::uv_tile` > 0: every textured Part, a fence or grate on a Part). The shadow pass reads the mesh's own texture coordinates, so a tiled cut-out's holes would fall at the wrong scale (review fixes 1 found 7 shadow stripes under 30 drawn); `plan_part` leaves tiled items out as `ray_cut_outs` does, and `a_tiled_cut_out_casts_its_whole_mesh_not_holes_at_the_wrong_scale` pins it. A hairstyle's cards are imported models: they keep their own coordinates.
- **Ray traced shadows and AO** go through cut-out cards on posed (skinned, morphed) items too: their per-frame BLAS is built without the opaque flag and reads the same texture coordinates (`ray_tracing.rs`, CO8). Tested on a skinned, morphed card: `a_striped_cards_ray_traced_shadow_is_striped_on_a_posed_part_too`, `a_clear_card_on_a_posed_part_casts_no_ray_traced_shadow_or_occlusion`.

### OPEN-19

"A cut-out card on a skinned, morphed part shows the backdrop through the skin behind its clear pixels" (CO6) does not happen in the renderer since CO8 poses every corner once a frame for every pass (`posing.rs`): before it, the depth fill and the lit pass could pose a morphed card differently. `a_cut_out_card_on_a_skinned_morphed_part_shows_the_skin_behind_it` draws a striped cut-out card (Standard and Hair) on a skinned, morphed part over skinned, morphed skin, under Off, MSAA 4×, TAA and TAA with AO: the strands are drawn and the skin shows between them in every mode. The face's brows and lashes can move to cut-outs (their build is CO6's owner's).

## Cost

`bench_hair_and_cloth_models` (render tests, ignored; RX 9070 XT, 1080p High with TAA, medians of 200 frames, the sun with shadows): a ball filling the view costs 0.432 ms in the Scene pass drawn Standard, 0.464 ms drawn Hair (+0.03 ms), 0.479 ms Skin. 24 layered cut-out cards over the view: 0.681 ms as Hair under TAA against 0.818 ms as Standard cut-outs (the hashed band leaves out fainter pixels), 0.928 ms against 0.927 ms under MSAA 4×; their shadow maps 0.029 to 0.037 ms (the cut-out shadow pipeline reads each caster's texture). One run, on a machine shared with other builds. For scale: the same pipeline on 36,000 leaf cards cost 0.22 ms more than casting them whole ([shadows.md](shadows.md)), so a hairstyle of a few hundred cards adds a few hundredths of a millisecond.

## Not yet

- Strands (decision 3's level 3) and deep opacity maps: self-shadowing between cards is the hashed shadow map's.
- No azimuthal roughness value or eccentricity (elliptical fibres).
- The TAA flicker at the hashed band's bottom (above): a reactive mask (keep history un-clamped on hashed hair pixels, from the prepass's flags) would take most of it.
- Wet hair: the weather doesn't darken or gloss hair.
- Ray traced reflections and GI see the hair's colour as made (they don't read the values).

## Tests

- `crates/render/src/renderer/tests/hair.rs`: the data and flow maps in the metalness and specular places and the values in the uniform; the primary highlight on a ball of strands within 1.5 px of where the model puts it for tilts of 0, 1 and 2 (toward the root as the tilt grows), white; the secondary below it, where the model puts it, tinted by the hair (red over blue) and darker with more melanin; the highlight's band turning with the flow map; light from behind coming through hair (TT) and coloured by it; melanin, redness, the roots, the tips, streaks and a dye changing the pixels where the data map says, with nothing rebuilt, and neutral drawing what neutral drew.
- `crates/render/src/renderer/tests/hair.rs` also: a back-lit single-sided hair ball's lit band ending within 6 rows of its terminator with an edge that moves under 0.4 rows from column to column (Off and MSAA 4×).
- `crates/render/src/renderer/tests/alpha_cards.rs`: TAA variance and soft edges; MSAA graded coverage; OPEN-19; a striped hair card's shadow striped in the shadow maps (posed too), a Standard one's a slab, and striped in ray traced shadows (Standard and Hair, posed too); a tiled striped hair card casting a slab, not holes at the untiled scale; a clear posed card casting no ray traced shadow or occlusion.
- `crates/render/src/renderer/tests/card_set.rs`: the test card set (striped, ramped and flat cards, a data map, flow maps) and the cloth swatch.
- `crates/assets/src/model/tests.rs`, `cooked_model.rs`: the extras read, the maps read where the colour map is, cooked models keep them.
- `crates/scene/src/material_values_tests.rs`, `draw_list/tests.rs`: the values reach `HairShading`, not the skin curve; a model marked hair draws as hair; an appearance leaves the atlas's maps out.
