# Wave 4 hand-back: CO7 Hair and Cloth shading, alpha cards, cut-out-aware shadows, OPEN-19

Branch `co7-hair-cloth` (RA7, milestone CO7's remaining models). The material contract the hairstyle and garment specialists build to is [docs/rendering/hair.md](../../../../rendering/hair.md) and [docs/rendering/cloth.md](../../../../rendering/cloth.md).

## What was built

1. **Hair model** (`crates/render/src/hair.wgsl`, `Shading::Hair`, `render::HairShading`). A model file marks a material `"axcenity_shading": "hair"` in glTF `extras`, as Skin. Marschner-style, in Karis's 2016 approximation, chosen over Kajiya-Kay with two shifted lobes because its lobes come from the fibre (the secondary's tint and the light through back-lit hair follow from one absorption), and it is the model Blender's Principled Hair BSDF draws, so the two can be compared:
   - R (white, toward the root by twice the cuticle's 2° tilt), TT (through the fibre, for back light), TRT (toward the tip by four tilts, twice as wide, tinted by the absorption);
   - a soft Kajiya-Kay term for light among strands, at a quarter of Karis's strength, set against the Blender references;
   - lit along the strand: the card's tangent frame with +V or a flow map.
   - Colour by pigment: the material's colour turned into absorption (Chiang 2016's mapping, as Blender's "direct colouring"), split into eumelanin and pheomelanin by least squares, moved by melanin (×8 to ÷8), redness (pheomelanin's share of the same melanin, as Blender's "melanin redness"), the roots, the tips, streaks over the atlas's data map, then dyed and tinted. Neutral values leave the colour untouched.
2. **Alpha cards.**
   - Under TAA, a hair card's alpha is hashed: interleaved gradient noise stepped by the golden ratio each frame, over the band 0.2 to 0.8 round the cutoff, offset per strand. The depth fill, the prepass and `fs_main` all cut alike.
   - Under MSAA, its coverage is its opacity (alpha to coverage, graded).
   - Off and FXAA cut it at the cutoff.
   - The depth prepass cuts with the same threshold, so there is no halo and no sorting.
3. **Cut-out-aware shadow maps for every cut-out** (defect D11 of the runtime-render audit). A second shadow pipeline reads each cut-out caster's material (`vs_shadow_cut_out`, `fs_shadow_cut_out`, `Caster::cut_out`, `CasterGroup`). Hair hashes its fringe against a fixed per-texel pattern.
   - Ray traced shadows and AO already went through cards on posed parts: CO8's posed BLASes are built non-opaque and read the same texture coordinates. Tests now prove it.
4. **OPEN-19.** It doesn't reproduce in the renderer since CO8. A test now pins it, with two kinds of card (Standard, Hair) and four anti-aliasing settings (Off, MSAA 4×, TAA, TAA with AO), on a skinned, morphed card over skinned, morphed skin.
5. **Cloth model** (`cloth.wgsl`, `Shading::Cloth`, `render::ClothShading`, `"axcenity_shading": "cloth"`).
   - Charlie sheen with Neubelt's visibility over Standard's light, dimmed by the sheen's directional albedo (a pre-integrated table, recomputed by a test).
   - Optional anisotropic GGX along the tangent, turned.
   - The sheen and anisotropy come from glTF `KHR_materials_sheen` and `KHR_materials_anisotropy`, read from the file's JSON (factors only).
   - With neither, cloth draws exactly as Standard.
   - No fuzz term: Charlie gives the soft rim, and nothing yet needs more.
6. **Values (ADR-0036 additions; optional, no enum grows):**
   - Hair: `root_melanin`, `root_length`, `streaks`, `streak_melanin`, `shift_scale`.
   - Cloth: `anisotropy_rotation`.
   - Every Hair and Cloth value is drawn live. A changed value re-binds the material; no mesh is rebuilt.
   - The new names are added to `DRAWN_TODAY`.
7. **Test card set and cloth swatch** (`crates/render/src/renderer/tests/card_set.rs`): striped, ramped and flat cards, a data map, flow maps, a card mesh with tangents, and the swatch material.

## Files changed

- render: `hair.wgsl`, `cloth.wgsl` (new); `shader.wgsl`, `lighting.wgsl` (the noise comment), `draw_list.rs`, `lib.rs`, `renderer/gpu.rs`, `renderer/lighting.rs`, `renderer/scene.rs`, `renderer/types.rs`, `renderer/debug_views.rs`; tests: `tests.rs`, `tests/hair.rs`, `tests/cloth.rs`, `tests/alpha_cards.rs`, `tests/card_set.rs`, `tests/character_models_evidence.rs` (new), `tests/shadows.rs` (group type); `CLAUDE.md`.
- assets: `model.rs` (`ModelShading::Hair`, `Cloth`; `Sheen`, `Anisotropy`; hair maps; `material_extensions`), `model/cooked_model.rs`, `model/tests.rs`, `lib.rs`.
- scene: `draw_list/materials.rs`, `material.rs` (`set_hair`, `set_cloth`), `material_values_tests.rs`, `draw_list/tests.rs`.
- avatar: `material.rs` (six values, a test).
- editor: `character_workspace/panel/materials.rs` (`DRAWN_TODAY` only).
- docs: `docs/rendering/hair.md`, `cloth.md` (new), `models.md`, `shadows.md`, `docs/rendering.md`; `docs/development/character-studio/architecture.md` §10 (the value table, the contract); `references/README.md` (sources); ADR-0036 (a note); wiki `Engine-Systems/Rendering.md`, `Roadmap/Known-Limitations.md`.
- This hand-back, the red run, and `co7-hair-cloth/` (evidence, 288 KB).

## Interfaces

- `render::Shading::{Hair(HairShading), Cloth(ClothShading)}`: their fields are documented on the types.
- `SHADING_HAIR` = 4, `SHADING_CLOTH` = 5.
- `Frame::noise.y` is 1 while TAA is on.
- `assets::ModelShading::{Hair, Cloth}`, `ModelMaterial::{hair_data_texture, hair_flow_texture, sheen, anisotropy}`, `assets::{Sheen, Anisotropy}`.
- The cooked model format gains these fields. Cooked caches are keyed by the engine build, so old caches are simply remade.
- Content contract (see hair.md / cloth.md):
  - Hair: `alphaMode: MASK`, `doubleSided`, MikkTSpace tangents, strands along +V.
  - Hair's `extras.axcenity_hair_data` channels: R root→tip, G per strand, B streak mask, A strand AO.
  - Hair's `extras.axcenity_hair_flow`: RG direction.
  - Cloth: `KHR_materials_sheen`, `KHR_materials_anisotropy`.

## Tests

Red run: [co7-hair-cloth-red-run.txt](co7-hair-cloth-red-run.txt). 15 new render tests ran against stubs: 12 failed and 3 passed. The 3 that passed already were OPEN-19 and the two ray-traced posed-card tests; CO8 had fixed those. That OPEN-19 run is not evidence: its card turned out to be drawn as bright as the wall behind it, so it couldn't fail. The test now also checks that the card's strands are drawn.

The assets, scene and avatar tests were written alongside the implementation, not before it.

Final runs (RX 9070 XT; a machine shared with other builds):

- `cargo nextest run -p axcenity-render -p axcenity-avatar -p axcenity-assets -p axcenity-scene --no-fail-fast`: **991 tests run, 990 passed, 1 aborted.**
  - The aborted test was `every_other_material_draws_as_it_did_before_the_skin_model`. It printed "ok" with the probe exactly equal to `PROBE_BEFORE`, then the process ended with 0xc0000409 under load.
  - Rerun alone, it passes. This is not a test failure; see open issues.
- After the last shader changes (the hair diffuse and the shadow band): `cargo nextest run -p axcenity-render -E 'test(/tests::(hair|cloth|alpha_cards|skin|shadows)::/)'`: **37 passed**.
- `cargo nextest run -p axcenity-player -E 'test(/every_example_renders|the_same_description_captures_identical|makeup|skin_value/)'`: **21 passed**. This covers `every_example_renders`, the capture determinism test, and the skin and makeup value captures.
- `cargo nextest run -p axcenity-editor -E 'test(/character_workspace::panel/)'`: **5 passed**.
- `cargo clippy -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all`. `python3 -I scripts/check-wiki.py`: 0 errors.

New tests (render unless marked):

- **Hair (`hair.rs`)**
  - Binding and uniform.
  - The primary's peak against the model, at tilt × 0, 1 and 2:

    | Tilt | Drawn (row) | Model (row) |
    |---|---|---|
    | × 0 | 121.67 | 121.54 |
    | × 1 | 115.50 | 115.52 |
    | × 2 | 109.67 | 109.58 |

    It is white, and moves toward the root as the tilt grows.
  - The secondary is at row 127.86 against the model's 127.62, below the primary. It is tinted (red over blue by 1.4) and darker at melanin 0.8.
  - The highlight band turns with the flow map: spread 48.8 across by 7.2 down, against 2.9 by 42.0.
  - Light through a card from behind: 143, 129, 107, against Standard's 0, 0, 0.
  - Each value changes the pixels where it should, with `mesh_bytes` unchanged: melanin, redness ±, roots, tips, streaks and dye.
- **Cloth (`cloth.rs`)**
  - Uniform.
  - Sheen along the middle row, from the centre to 95% of the radius: −3 head-on rising to +312 at the rim. No sheen draws as Standard, within 3 of 255.
  - Anisotropy 0.8 spreads the highlight 7.5 across by 1.0 down, and 1.0 by 7.5 when turned a quarter. Isotropic is 3.1 by 3.1.
  - The sheen albedo table.
- **Alpha cards (`alpha_cards.rs`)**
  - TAA: rms frame-to-frame deviation 3.38 of 255, with 20 or more soft-edge pixels. A Standard cut-out stays hard (4 or fewer).
  - MSAA: graded, with 3 or more coverage levels.
  - OPEN-19.
  - Striped shadows in the shadow maps, 7 turns each: Standard, Hair, and posed Hair.
  - Striped ray traced shadows on a posed card.
  - A clear posed card casts no ray traced shadow or AO.
- **assets**: hair and cloth extras; KHR sheen and anisotropy (clamped, a broken one left out); hair maps read only where the colour map is; a cooked model keeps hair and cloth materials.
- **scene**: Hair values reach `HairShading`, not the skin curve. Cloth values modulate the authored sheen and anisotropy. Models marked hair or cloth draw as such. An appearance leaves out the atlas's maps.
- **avatar**: Hair and Cloth have the values their models draw.

**ABORT lines:** one in the four-crate run, described above. None in the other runs.

## Measurements

**GPU** (`bench_hair_and_cloth_models`, ignored render test; RX 9070 XT, 1080p High with TAA, medians of 200 frames, one run):

| What | Scene | Shadows | Prepass |
|---|---|---|---|
| Ball filling the view, Standard | 0.432 ms | 0.031 | 0.071 |
| Skin | 0.479 | 0.031 | 0.069 |
| **Hair** | **0.464** | 0.031 | 0.069 |
| **Cloth, sheen** | **0.444** | 0.031 | 0.070 |
| **Cloth, sheen + anisotropy** | **0.451** | 0.031 | 0.069 |
| 24 layered cut-out cards, Hair, TAA | 0.681 | 0.029 | 0.107 |
| the same as Standard cut-outs, TAA | 0.818 | 0.037 | 0.144 |
| Hair cards, MSAA 4× | 0.928 | 0.032 | 0.060 |
| Standard cut-out cards, MSAA 4× | 0.927 | 0.037 | 0.111 |

**Engine against Blender** (Cycles, Principled Hair BSDF Chiang with direct colouring, roughness 0.2, radial 0.3, offset 2°; Principled BSDF sheen 1, roughness 0.5; anisotropic 0.8 on a radial tangent).

The scenes match: the same 512² camera (6 studs from a ball of radius 1, 30° lens) and the same suns.

Blender's exposure was matched to the ball's mean luminance, which is printed on the sheet. The script is `co7hc_blender.py`; the measures come from `co7hc_sheet.py`. Both scripts were kept out of the repository.

- **Hair, brown**
  - The highlight's brightest row is 233 in the engine and 240 in Blender (512 px).
  - Chroma, linear and normalised, is close: engine (0.47, 0.32, 0.21), Blender (0.50, 0.32, 0.18).
  - The bright band is wider in the engine: 26 rows against 16.
  - Away from the band the engine keeps more light than Blender: 0.14 to 0.27 of the peak against 0.04 to 0.27, measured 20° to 45° off the band. It is closer since the diffuse change; it was 0.35 to 0.5.
- **Hair, blonde**: rows 243 and 236. Chroma: engine (0.44, 0.34, 0.22), Blender (0.42, 0.35, 0.23).
- **Back-lit blonde**
  - Both show the light only along the upper rim; the ball shadows the rest (shadow maps in the engine).
  - The engine's rim is warm, its TT tinted. Blender's rim is white, an R lobe at grazing.
  - Not yet matched.
- **Cloth with no sheen**: the same, with rim/middle 0.46 against 0.47.
- **Cloth sheen**
  - The rim brightens in both.
  - The engine's brightening is twice as strong and wider: rim/middle 2.35 against 1.29.
  - Blender 5's sheen is Zeltner's microfibre lobe, not Charlie (glTF's), so this is a model difference, not a bug.
- **Satin**: both stretch the highlight along the tangent. The engine's is longer and thinner: spread 15/2 against 11/3. Blender maps "anisotropic" to the lobe's widths differently.

## Visual evidence and what I saw

All in `co7-hair-cloth/`:

- `engine-vs-blender.jpg`: as above.
  - The engine's hair reads as a white primary band above a warmer, wider secondary.
  - In Blender the two merge into one warm band, and strand-level detail (individual strands) shows, which a smooth ball can't.
  - The engine ball is the built-in sphere, so its silhouette is faceted.
- `hair-values.jpg`: brown hair, the same at melanin 0.6, and at redness 1.
  - Melanin 0.6 darkens it, and its secondary goes deep brown.
  - Redness 1 is lighter and warmer (auburn), as Blender's redness behaves. Pheomelanin absorbs less, so redness alone never reads as vivid red.
- `card-over-skin-by-anti-aliasing.jpg`: a card whose opacity ramps across its whole width (a worst case) over Skin, under Off, MSAA 4× and TAA.
  - Off is a hard cut.
  - MSAA shows alpha to coverage's regular dither pattern across the ramp: a visible screen-door on a wide gradient.
  - TAA is smooth, with a slightly ragged edge at the hashed band's bottom.
  - The hashed shadow shows a faint stipple on the skin where the card's opacity is mid-range.
  - Real atlases' fringes are a texel or two wide; the first hairstyle will tell how much of this shows.
- `taa-ramp-card-four-frames-x4.png`: four consecutive TAA frames of the ramp card's edge, ×4. The gradient is steady; the outermost edge's lone pixels come and go.
- `shadow-map-stripes.png`: the striped card's shadow in the shadow maps (Standard, Hair, and posed Hair): stripes, not slabs.

## Open issues

- **The TAA flicker at the hashed band's bottom.**
  - The worst pixels deviate 11 to 15 of 255 per frame, against a 3.38 average.
  - Fix proposal: a reactive mask, so TAA doesn't clamp the history of hashed hair pixels. The prepass's normal.w flags could carry it.
  - The 4.5 limit is provisional until Paul's first-hairstyle sign-off (asset-quality.md §4).
- **MSAA's alpha-to-coverage dither** shows on wide gradients. It is hardware's pattern.
- **Hair calibration against Blender.**
  - The engine's primary is a distinct greyish band.
  - Its back-lit rim is warm, where Blender's is white.
  - Off-highlight light is a little high.
  - Worth tuning on the first real hairstyle (R lobe width, the TT/R balance at grazing), not on a ball.
- **Cloth sheen is stronger and wider than Blender 5's.** That is Charlie against Zeltner's lobe; a fuzz (LTC) lobe would match Blender's better if fabrics read wrong.
- **What the extensions and other paths don't cover yet:**
  - `KHR_materials_sheen` and `KHR_materials_anisotropy` textures aren't read.
  - Ray traced reflections and GI see hair and cloth as made.
  - The weather doesn't wet hair.
- **The probe guard's process abort** (0xc0000409 after "ok", under the full parallel run). It was not seen alone; it looks like a teardown crash under load. Worth watching in the lead's merged run.
- **The realistic brows and lashes** are still blended cards marked Standard. Moving them to Hair cut-outs is the face build owner's job (CO6). OPEN-19 no longer blocks it.

## What the lead must do at merge

- **Shared contract:** the six optional values and the hair/cloth contract in `architecture.md` §10. No `format_version` change; no enum grows in descriptions or manifests. `ModelShading` grows inside `assets` only, and cooked caches are build-keyed.
- **Conflicts likely with the parallel tracks:**
  - `DRAWN_TODAY`: ux-drag and the browsers may touch the panel.
  - `crates/render/src/renderer/tests.rs` module list.
  - `docs/rendering.md`.
  - The wiki Rendering page and Known-Limitations.
  - The content tracks (first hairstyle, first garments) will add files marking materials hair or cloth; nothing of theirs is in this branch.
- **Generated files:** none regenerated. Nothing here changes a build script or generated asset.
- **After merge**, rerun the full workspace suite, and the probe guard alone if it aborts again.
