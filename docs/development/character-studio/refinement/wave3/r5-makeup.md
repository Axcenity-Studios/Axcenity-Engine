# Hand-back: RA5 with RA7, `r5-makeup` (R5's makeup, markings and the grain)

Task: R5's remaining parts. Makeup layers (foundation, blush, eyeshadow, eyeliner, lip colour, brow fill) with colour, opacity and roughness; markings (freckles, moles, a scar) as seeded masks; the markings and decal architecture as an ADR; a minimal Makeup and markings section on the Skin page; and the skin grain's Voronoi crackle (wave 2's high-priority open issue) replaced by pores. Branched from `refinement-wave2` (`b39a9ed7`).

## The decision: [ADR-0042](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0042-Makeup-And-Markings-Are-Layers-Over-Library-Masks-Drawn-Live-By-The-Skin-Model.md) (Proposed)

- **Stored** as an optional description block `skin: {makeup: [SkinLayer], markings: [SkinLayer]}`, each layer naming one of the base manifest's new optional `skin_layers` by ID (a string, so no enum grows; an older engine keeps the block in `extra`). Chosen over ~25 new Skin material values, which an older engine would report as unknown and which leave no room for decals.
- **Resolved** to draw values `layer_<id>_<field>` on the layer's material, written by the applier as material value attributes: a change is a few attributes, live, no bake, no per-character texture.
- **Masks are content**: three RGBA images in the model file (`extras.axcenity_skin_masks`), one layer to a channel, made by the build. Freckles and moles are seeded spots: each candidate spot has a rank, and the shader shows those whose rank offset by `seed × golden ratio` is below `amount`.
- **Drawn** by the Skin model into albedo and roughness before light, in the references' order (markings as pigment, then makeup over it, foundation first), keeping part of the skin's fine variation under each layer. The masks take the metalness, emissive and opacity slots (the item shader is already at wgpu's default 16 sampled textures, so no binding is added); the values ride where skin reads nothing else in the uniform (the surface parameters, toon, the specular row's last two).
- **Decals** (tattoos, birthmarks, placed marks): a later `skin.decals` list placed by projection in the rest pose; not built.

## What changed

**Character Data** (`crates/avatar`; the contract change this task names):

- `skin.rs` (new): `Skin`, `SkinLayer`, `SkinLayerKind` (`makeup`, `marking`, `#[serde(other)] Other`), `SkinLayerSpec`, `skin_layer_value`; the checks (`check_specs`, `check_skin`) and `draw_values`.
- `Description::skin` (optional; not written when empty); `BaseManifest::skin_layers` and `BaseManifest::skin_layer`; `validate` on both checks them; `resolve` adds the draw values; a `skin` (and `character`) preset carries the block, and `check_preset` reports one on another kind.

**Assets** (`crates/assets`): `skin_masks_of` reads `axcenity_skin_masks` (exactly three, on a material marked skin, with a base colour map); `ModelMaterial::skin_masks`; `SKIN_MASKS`; cooked models keep them; `images()`, `has_textures`, `remove_textures`.

**Scene** (`crates/scene`): `model_material` gives a skin with masks `SkinShading::layers` with every layer off; `modulate` sets them from the attributes (`set_skin_layers`) and drops them when none is on (no masks bound, no cost); a `SurfaceAppearance` leaves them out with the file's maps.

**Render** (`crates/render`), within the fragment shading, the material bindings and their slots (nothing on the vertex side):

- `draw_list.rs`: `SkinShading::layers: Option<SkinLayers>`; `SkinLayers`, `Makeup`, `Spots`, `SKIN_MAKEUP_LAYERS` (exported).
- `renderer/lighting.rs`: `material_textures` binds the masks in slots 2, 5 and 6 (metalness, emissive, opacity) for a skin with layers; `skin_layers`, `skin_layer_uniform`; `material_key` puts the values in `params`, `toon` and `specular.zw`, and no opacity map is read.
- `renderer/gpu.rs`: `create_material` reads the emissive slot through its linear view when it holds masks.
- `skin.wgsl`: `skin_layered` (markings, then makeup), `skin_spot_shown`, `skin_scar_relief` (Mikkelsen's surface-gradient bump from the scar mask, only while a scar is on), constants `SKIN_TONE_LEVEL`, `SKIN_DETAIL_LEVEL`, `SKIN_FOUNDATION_DETAIL`, `SKIN_MAKEUP_DETAIL`, `SKIN_FRECKLE`, `SKIN_MOLE`, `SKIN_SCAR`, `SKIN_SCAR_ROUGHNESS`, `SKIN_SCAR_HEIGHT`.
- `shader.wgsl`: **one block at the end of `standard_surface`** calls them (guarded by `maps.shading.x == SHADING_SKIN && maps.specular.z != 0.0`). Nothing else in `shader.wgsl` changed.

**Content** (`assets/characters/realistic`):

- `makeup.py` (new): a position map rasterised from the body's texture coordinates on the face at rest; makeup masks from the face's landmarks (`face.Face`: the lids' loops, the brows' line, the cheeks, the jaw, the ears) and the albedo's own lip field (`skin.fields`); seeded freckles (13,002 candidates over a sun field: nose, cheeks, forehead, shoulders, upper back and chest, arms) and moles (70, 9 on the face), each a soft, slightly irregular blob measured in millimetres on the rest surface, with its rank spread a few texels past it; one scar (36.7 mm, 3 mm wide, down the left cheek); `mark` adds the three PNGs to the GLB.
- `skin.py`: the grain's cells are pits (Voronoi F1, a pit of 0.24 of a cell at each point) over the gentle wrinkling and new fine lines (noise stretched round the body), bump height 0.09 to 0.12 texels; `grain_shape` measures the result on the belly (`textures.grain.height_skew`, `pores_per_cm2` in `Realistic.build.json`).
- `manifests.py`: `SKIN_LAYERS` (the nine layers, labels, tooltips, starting colours, roughness, opacity and amounts) written as the base's `skin_layers`. Nothing else in it changed.
- `build.py`: three lines (call `makeup.masks` after `face.prepare`, `makeup.mark` after `skin.mark`, `makeup.clean_up`).

**Editor** (`crates/editor/src/character_workspace`, kept small): `Intent::SetSkinLayer {id, marking, to, gesture}`; `CharacterWorkspace::set_skin_layer`, `skin_layer`; `shell.rs` routes it; `panel/materials.rs`'s new `makeup` section (a check box per layer; colour, opacity, roughness for makeup; opacity, amount and **New pattern** for markings; one undo step per change or drag); `edit.rs`: Reset of the Skin category also clears the block.

**Docs and wiki**: ADR-0042 (new, index row, System-Inventory), a dated note on ADR-0036, `docs/rendering/skin.md` (Makeup and markings, the channel table, tests), `architecture.md` §5, §6 and §10, `docs/editor/character-studio.md`, `crates/avatar/CLAUDE.md`, `crates/render/CLAUDE.md`, the realistic README, wiki Character-Data, Rendering and Known-Limitations (the grain's entry rewritten; a new makeup entry). `python3 -I scripts/check-wiki.py`: 114 pages, 0 errors, 58 warnings (all "may be stale" on pages this change doesn't touch).

## Interfaces

- `avatar`: `Skin { makeup, markings, extra }`, `SkinLayer { layer, enabled, colour, opacity, roughness, amount, seed, extra }` (`new`, `from_spec`), `SkinLayerSpec { id, label, kind, material, about, colour, roughness, opacity, amount }`, `SkinLayerKind`, `skin_layer_value(layer, field)`; `Description::skin`; `BaseManifest::skin_layers`, `skin_layer(id)`.
- Draw values on a material (attributes `Mat_<slot>_layer_<id>_<field>`): `opacity`, `colour`, `roughness`, `amount`, `seed`. IDs the renderer draws: `foundation`, `blush`, `eyeshadow`, `eyeliner`, `lips`, `brow_fill`, `freckles`, `moles`, `scar`.
- glTF: material `extras.axcenity_skin_masks: [{index}, {index}, {index}]`.
- `assets`: `ModelMaterial::skin_masks`, `SKIN_MASKS`.
- `render`: `SkinShading::layers`, `SkinLayers { masks, makeup, freckles, moles, scar }`, `Makeup { colour, opacity, roughness }`, `Spots { amount, opacity, seed }`, `SKIN_MAKEUP_LAYERS`.
- editor: `Intent::SetSkinLayer`.

## Tests

Written first and seen failing: [r5-makeup-red-run.txt](r5-makeup-red-run.txt) (avatar 4 of 101, assets 2 of 217, render 4 of 21, scene 2 of 15, player 3 of 5, editor by compilation).

| Test | What it proves |
|---|---|
| `avatar` `skin_layers::a_layered_description_round_trips` | the brief's "a layered description round-trips", with unknown fields in a layer and in the block |
| `avatar` `a_description_without_makeup_writes_no_skin_block_and_an_older_file_reads`, `the_skin_block_is_an_additive_field_older_versions_keep` | format 2 compatibility both ways; no new enum value |
| `avatar` `a_base_offers_its_skin_layers_and_checks_them`, `skin_layers_are_checked_against_what_the_base_offers` | the manifest's and the description's checks (unknown layer, wrong list, twice, out of range, NaN, a non-skin slot) |
| `avatar` `layers_resolve_to_draw_values_on_their_material`, `a_skin_layers_default_opacity_is_where_a_new_layer_starts` | resolution, the base's defaults, off and opacity 0 draw nothing |
| `avatar` `skin_presets_carry_makeup_and_markings_and_other_kinds_leave_them` | presets |
| `assets` `a_skins_extras_name_its_three_layer_masks`, `the_realistic_skin_has_its_makeup_and_marking_masks` | the parser; the built file's masks: the size of the maps, every channel covering some skin and not most, the spare channel empty, at least 2,000 freckles and 40 moles as separate small patches |
| `assets` `the_realistic_skins_grain_is_pores_not_a_crackle_network` | the grain's height skews below -1.5 (pits in a level surface; wave 2's edge network measured +0.05) with 0.5 to 20 pores per cm² |
| `scene` `a_skin_models_makeup_and_marking_masks_reach_the_renderer_all_off`, `skin_layers_draw_from_their_values_and_none_on_draws_no_layers`, `skin_without_masks_ignores_layer_values` | the translation; no masks bound with nothing on |
| `render` `skin_layers::skin_layers_bind_their_masks_in_place_of_metalness_emissive_and_opacity` | the slots and the uniform |
| `render` `each_makeup_layer_and_marking_changes_the_skin_where_its_mask_is` | each of the nine layers reads its own channel (masks with only that channel set) and changes the skin there only (GPU) |
| `render` `layers_turned_off_draw_the_skin_bit_for_bit`, `a_markings_amount_shows_a_share_of_its_spots_and_its_seed_picks_which`, `a_layers_roughness_changes_the_skins_highlight` | off is exact; amount 0.5 shows 8 of 16 rank rows and seed 0.5 the other 8, amount 0 none; gloss 0.15 brighter and 0.95 duller than the skin's own (GPU) |
| `player` `makeup_tests::foundation_changes_the_face_against_bare_skin` and one test per layer (`blush_`, `eyeshadow_`, `eyeliner_`, `lip_colour_`, `brow_fill_`, `freckles_change_`, `moles_change_`, `the_scar_changes_...`) | **R5's "every layer changes the pixels of a capture against neutral"**, a test per layer on the realistic feminine average (face-front, studio, 480 × 270): makeup makes what it covers bluer (a blue test colour at full opacity), spots darker, the scar paler, within a share of the picture that keeps each where it belongs; off and opacity 0 draw the bare picture exactly |
| `player` `a_makeup_layers_roughness_changes_the_lips_highlight` | glossy against matte lips in a capture |
| `player` `softness_alone_hides_the_definition_the_body_drives` | wave 2's open item: `softness` 1 on its own on the athletic body changes 0.29% of the front picture and lowers the relief there (11.25 to 10.81) |
| `player` `a_makeup_colour_change_re_applies_live_without_a_bake` | **R5's "a colour change re-applies without a re-bake (measured)"**: 20 blush colour changes applied; the body keeps its base `MeshId` (no derived model, `is_baked` false), the last colour is the attribute; median time printed and held under 250 ms |
| `editor` `walkthrough_tests::a_makeup_layer_is_one_undo_step_per_change_and_reaches_the_preview_live` | turned on, recoloured, a drag of opacity, turned off: four undo steps; attributes on the preview's body, no rebake; Skin reset clears it |

Every Skin **value** now has a capture test: waves 1 and 2 covered melanin, redness, undertone, roughness scale, detail strength, scatter scale and definition with softness (`skin_value_tests.rs`); this branch adds `softness` alone and `tint` (`a_skin_tint_changes_the_face_the_way_it_means`: a green tint changes 15% of the face-front picture and raises green over red by 46%; written after the code it tests, which already worked, so not seen failing).

Summary lines (debug, this worktree, RX 9070 XT; the machine shared with six other builds):

- `cargo nextest run -p axcenity-avatar -p axcenity-assets -p axcenity-render -p axcenity-scene -p axcenity-player -p axcenity-gameplay -p axcenity-editor --no-fail-fast`: `Summary [ 604.698s] 2520 tests run: 2520 passed (37 slow, 1 leaky), 35 skipped` (includes `every_example_renders` PASS in 47.8 s, the probe guard `every_other_material_draws_as_it_did_before_the_skin_model`, all 12 wave 2 `skin::` tests, and the studio's `every_page_is_drawn_with_its_controls`). The leaky one is `luau_checks::tests::requires_are_followed_into_modules_and_a_module_edit_checks_again` (editor, not mine: leaked a handle, passed).
- An earlier full run (before the per-layer split): `2512 tests run: 2511 passed (21 slow), 1 timed out`: the single nine-layer capture test ran past the 2-minute limit under the shared load (then split, above). In that run `every_page_is_drawn_with_its_controls` passed; in a run on its own minutes earlier it timed out at 120 s (load).
- `cargo clippy -p axcenity-avatar -p axcenity-assets -p axcenity-render -p axcenity-scene -p axcenity-player -p axcenity-gameplay -p axcenity-editor --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean. `python3 -I scripts/check-wiki.py`: 114 pages, 0 errors, 58 warnings (all "may be stale" on pages this change doesn't touch).
- `cargo test -p axcenity-player --release makeup_tests::a_makeup_colour_change -- --nocapture`: `release build: a blush colour change re-applied in median 0.144 ms (min 0.130, max 0.464)`.

## Measurements

- **A colour change re-applies live**: `a_makeup_colour_change_re_applies_live_without_a_bake`, `apply_description` for a blush colour change: debug median 0.57 to 0.63 ms over two runs; **release median 0.144 ms** (min 0.130, max 0.464), 20 changes. No bake, no texture made: the body keeps its base `MeshId`.
- **The grain** (`Realistic.build.json`, `textures.grain`, measured on the bump before it is mixed with the sculpt): height skewness **-2.82** (pores) against **+0.05** for wave 2's recipe built the same way; 1.26 pores per cm² deeper than 1.5 deviations on the belly (1.11 for the edge network's lowest junctions). Measures of how connected the lowest texels are did not separate the two recipes (the edges are under a texel wide and break up when thresholded), so the skewness is the measure, and the pictures carry the judgement (below).
- **The masks** (`textures.makeup`): texels over half covered, of the 2K map: foundation 4.3%, lips 0.10%, blush 0.093%, eyeshadow 0.071%, eyeliner 0.025%, brow fill 0.024%; the scar 36.7 mm long; 13,002 freckle and 70 mole candidates (9 on the face). The GLB is 38.0 MB (was 42.4): the three masks add about 0.6 MB, and the normal maps compress smaller without the edge network.
- **Captures** (the per-layer `player` tests, 480 × 270, face-front, studio): share of the picture changed by more than 4 of 255 in a channel, makeup in a blue test colour at full opacity, markings at full amount: foundation 8.94%, blush 1.40%, lips 0.46%, eyeshadow 0.43%, brow fill 0.17%, eyeliner 0.11%, the scar 0.05%, freckles 0.04%, moles 0.03% (freckles are small and soft at this size: most change a pixel by less than 4).
- **GPU cost**: not measured. A skin with a layer on reads three more textures and the albedo twice more at fixed levels, once per pixel (not per light); with no layer on nothing is bound or read.

## Visual evidence and what I saw

Sheets in [r5-makeup/](r5-makeup/) (JPG, 2.2 MB): every layer alone at its starting settings, face-front on both frames under studio, warm, cool and outdoor light (`layers-*-face-front.jpg`) and skin-close under studio (`layers-*-studio-skin-close.jpg`); all makeup and all markings under the five lights, three-quarter (`all-*-lights-face-three-quarter.jpg`); 2× crops of the eyes and cheeks under studio and hard light (`detail-*-eyes-cheek.jpg`); skin-close bare, makeup, markings (`close-*-skin-close.jpg`); MSAA 4× (`msaa-makeup-markings.jpg`); placement in vivid colours (`placement-vivid.jpg`); the masks over the albedo (`masks-over-albedo.jpg`); the grain before and after, skin-close under hard and studio light with TAA and MSAA (`grain-skin-close-before-after.jpg`) and its bump height integrated from the normal maps (`grain-height-before-after.png`). The captures behind them are outside the repository; the descriptions are the cast's average files with one `skin` block each.

Looked at at full size and in 2× to 5× crops:

- **The crackle is gone.** Before, close views under hard light showed a network of polygonal cells over the whole face (cracked skin), under TAA and MSAA alike. After, there is no network: smooth skin with faint, small pits in the highlights. The cost is that a close view under the soft studio light now reads **smooth, nearer airbrushed than scanned**. A first try at a higher bump (0.16 texels) made the pits read as scattered small bumps under the hard light, so I lowered it to 0.12 with smaller pits; denser real pores need a tiling pore detail normal or a 4K face set (open issue 1).
- **Makeup reads as makeup on skin** at its starting settings: a soft rose blush on the cheek apples that fades into the skin (no edge), a taupe-plum shadow on the upper lids, a dark liner along the upper lid ending in a short wing, a lip colour close to the lips' own (at its starting colour it hardly changes them; a stronger colour shows clearly in the vivid sheet), foundation that evens the cheeks' flush. Under warm, cool and outdoor light they keep their hue relative to the skin and nothing glows or turns flat: they are lit as skin. In the vivid colours the placement is right: shadow from the lash line to under the brow, blush on the cheekbones swept toward the temple, foundation over the face from the hairline to under the jaw, stopping in front of the ears.
- **The eyeliner's first version was a defect**, fixed: drawn on the lashes' roots, the lid's rim and lashes hid the line and its wing stood alone above the outer corner as a stray stroke. It is now drawn on the lid's skin just outside the roots, so line and wing join.
- **Freckles read as freckles**: small, slightly irregular brown spots, densest on the nose and cheeks, sparser on the forehead, varying a little in darkness. Close up they are a little too even in size and too dark at the starting opacity on this skin tone; the opacity slider takes them down. **Moles** read as moles (a few darker, rounder spots, one at the temple, one beside the lips).
- **The scar** reads as a scar under the hard and outdoor light (a pale, slightly raised line down the left cheek), and only faintly under the soft studio light at face-front distance. It took two passes: at 1.5 mm wide and 10% paler it was invisible.
- **Brow fill barely shows**: the realistic brows' card is dense, so the fill shows only as a slight darkening at the card's edges. Honest result: it does little on this brow.
- The masculine face shows the same; the foundation's edge along the jaw is not visible as an edge.

## Dependencies added

None.

## Open issues

1. **Pores are faint and sparse** (see above): a close view under soft light reads smooth. Next: a tiling pore detail normal made by the build (references/skin §2) or a 4K face set.
2. **Decals** (tattoos, birthmarks, placed marks) are ADR-0042's later part: the data shape and the projection are described, nothing is built. The ADR is Proposed: the lead accepts it or changes it.
3. The masks' **channel layout is fixed** (nine layers, one spare channel); a tenth kind needs a fourth mask (no slot left) or the decal path.
4. **Ray-traced reflections and GI** don't see makeup (they read the material colour, not the masks). Known-Limitations says so.
5. **Eyeliner at 2K** is two to three texels on the face; its wing is short (9 mm). A face-only mask at higher density would sharpen it.
6. The **GPU cost** of layers on is not measured (`bench_realistic_character` draws no makeup).
7. **Brow fill** barely shows on the realistic brows (above).
8. Wave 2's other open items are untouched: the shadowed lips' extra red, the probe guard's hard-coded numbers, `DRAWN_TODAY` and the Skin page's outdated footer note ("Until skin shading lands...": RA1's file; I added the new section above it and left the note).
9. The page snapshot test `shell_tests::every_page_is_drawn_with_its_controls` timed out once at 120 s when run alone under the shared machine's load (its skin page drew, with the new section) and passed in 15.7 s in the full run. No snapshot of the section with layers turned on was added (`shell_tests.rs` is RA1's); the walkthrough test drives it through `Intent`s.
10. Freckles and moles are a fixed population: a seed picks others, it can't move one.

## For the lead at merge

- **Regenerated by me** (committed): `RealisticParts.glb` (38.0 MB: the masks, the new grain), `Realistic.base.json` (only `skin_layers` added), `Realistic.build.json` (`textures.grain` and `textures.makeup`). Built with the bpy 5.3 python, `build.py --out <scratch>`, then copied in; `catalogue.py` gave no change; presets and benchmarks unchanged (not run: nothing they read changed); the item manifests differ only in line endings, not committed. If RA2 or RA3 changed the same files, rebuild with `build.py` and `catalogue.py` after merging: my content changes are `makeup.py`, the grain in `skin.py` (`bake`'s bump function, the constants, `grain_shape`), three lines of `build.py` and `SKIN_LAYERS` plus one line in `manifests.py`'s base dict.
- **Render, for CO8's merge**: I touched `skin.wgsl` (new functions), the block at the end of `standard_surface` in `shader.wgsl`, `material_textures` and `material_key` in `renderer/lighting.rs` (plus `skin_layers`, `skin_layer_uniform`), `create_material` in `renderer/gpu.rs`, and `draw_list.rs`'s `SkinShading`. No vertex code, bind group layout, `MapsUniform` field or binding count changed.
- **Contract**: `avatar` gains `skin` (description) and `skin_layers` (base manifest), both optional (ADR-0042); `architecture.md` §5, §6 and §10 say so. The lead's `intent.rs` and `edit.rs` gained one variant and one line.
- The capture tool and `Settings::default`, VIEWS, LIGHTS and MORE_VIEWS are untouched.
