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
3. **Cut-out-aware shadow maps** (defect D11 of the runtime-render audit). Review fixes 1 limits them to hair cards: other cut-outs cast whole cards again, as on `main` (see below). A second shadow pipeline reads each cut-out caster's material (`vs_shadow_cut_out`, `fs_shadow_cut_out`, `Caster::cut_out`, `CasterGroup`). Hair hashes its fringe against a fixed per-texel pattern.
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
- `shadow-map-stripes.png`: the striped card's shadow in the shadow maps. Since review fixes 1: Standard casts a whole card (a slab, as on `main`); Hair and posed Hair cast stripes. The Standard card itself doesn't show against the wall because both are white and lit alike.

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

## Review fixes 1

The review found three blocking items. All three are fixed, each with a test run red first (the red runs are appended to `co7-hair-cloth-red-run.txt`).

### 1. Tiled cut-outs cast holes at the wrong scale

The reviewer was right. A cut-out whose texture is tiled by its size (`DrawItem::uv_tile` > 0, which is every textured Part) went through the cut-out shadow pipeline. That pipeline reads the mesh's own texture coordinates, so the shadow's holes didn't match what was drawn.

- **Test:** `a_tiled_cut_out_casts_its_whole_mesh_not_holes_at_the_wrong_scale` (`alpha_cards.rs`). A striped card with `uv_tile` 1 must cast a slab (at most 2 turns). Red: 7 turns.
- **Fix:** `plan_part` (`renderer/lighting.rs`) leaves tiled items out of the cut-out pipeline, as `ray_cut_outs.rs` does, so they cast their whole mesh as the docs said. Green: slab.
- After fix 3, the only cut-outs that use that pipeline are hair cards, so the test now uses a hair card. It passes.

### 2. The back-lit hair evidence had a torn shadow edge and a patch at the pole

The reviewer was right on both counts.

**How I found the cause.** I added three variants of the back-lit scene to `hair_and_cloth_evidence`: Off, TAA over 40 frames, and shadows off.
- With shadows off, the whole front of the ball lit up. So the hair model was lighting strands past the terminator, where no n·l term dims them, and the torn edge was the shadow map's self-shadow edge showing through.
- TAA averaged the noise but left blocky steps at the size of a shadow-map texel.
- The grey-blue patch stayed with shadows off, so it is not a shadow problem. It is the test ball's pole: the built-in sphere's meridians (the strand directions) meet there, and no tangent field over a sphere avoids that. Blender's render of the same ball shows its own small dark notch at the pole.

**Fix (`hair.wgsl`, `hair_shell`).** A single-sided hair material is now treated as a shell over something solid, such as a hair cap or this ball. Its sun and lamp light fade over its terminator, from whole at n·l 0.1 to none at n·l -0.05.
- Double-sided cards are unchanged: a card is a sheet of strands, lit from either side, and light from behind still comes through it (`light_from_behind_comes_through_hair` passes).
- The material says it is a shell with bit 4 of `MapsUniform::shading.y`, set from `!material.double_sided`. The uniform test pins this (`shading[1]` is 4 for a single-sided hair material without maps).
- I tried full strength at n·l 0.25 first. Then I narrowed it to 0.1, which matches Blender's rim better (highlight spread down: 5 rows against Blender's 6, compared with 4 at 0.25). The front-lit hair renders don't change: brightest rows 233 and 243, chroma and rim/middle are the same as before.

**Test:** `a_back_lit_hair_shells_lit_band_ends_smoothly_at_its_terminator` (`hair.rs`) draws the evidence scene at 512 px under Off and MSAA 4×. It projects the terminator column by column and leaves out the pole's 32 middle columns, as the test asset's own artefact. It checks two things:
- how far past the terminator the lit band ends: at most 6 rows. Red: 21.1. Green: 2.6.
- how much that end moves from one column to the next: under 0.4 rows. Red: 0.48. Green: 0.30.

**What I saw** (`backlit-hair-shell-terminator-x3.png`, the ball's top at 3×; and the new `engine-vs-blender.jpg`):
- After the fix the rim is a thin, smooth crescent under Off, MSAA and TAA, the same shape as Blender's. The torn band is gone.
- Remaining differences, unchanged and still open:
  - The engine's rim is warm gold at the sides, where Blender's is white.
  - It fades out a little sooner down the sides.
  - The pole patch is still grey, as the test asset's artefact.
- Front-lit scenes look the same as before.

### 3. The cut-out shadow cost on foliage wasn't measured

The reviewer was right, and the measurement changed the design.

**The bench.** `bench_cut_out_foliage_shadows` (new file `renderer/tests/foliage_shadow_bench.rs`, ignored, run by hand) is self-contained, so the same file built on `main`.
- The scene: 24 trees of 1,500 cut-out leaf cards each (36,000 cards), under a low sun with its cascades, at 1080p High with TAA.
- Method: medians of 200 frames; RX 9070 XT; pinned to CPUs 0 to 15; five rounds alternating with `main` at `e1658afb`.

| Build | Shadows (ms) | Prepass (ms) | Scene (ms) |
|---|---|---|---|
| `main` `e1658afb` | 0.190–0.194 | 0.206–0.208 | 1.121–1.132 |
| Every cut-out through its holes (`fef3f3d2`) | 0.412–0.419 | 0.208–0.209 | 1.119–1.127 |
| Only hair through its holes (this fix) | 0.188–0.193 | 0.209–0.210 | 1.124–1.130 |

One `main` round (0.203 / 0.270 / 1.267 ms) was disturbed by other work on the machine and is left out of the ranges.

**Why it cost so much, and the change.** Casting every cut-out through its holes multiplied the Shadows pass by 2.2: the cut-out pipeline samples the texture and discards, so it loses early depth rejection in every cascade. That is well over the reviewer's 10–15% bar, and Paul's foliage projects are already GPU-bound. So **only hair cards cast through their holes now** (`plan_part`: `matches!(item.material.shading, Shading::Hair(_))`). Leaf cards and fences cast their whole cards again, exactly as on `main`, which the table confirms.
- **Test:** `a_standard_cut_out_casts_its_whole_card_in_the_shadow_maps` (red: 7 turns; green: a slab). It replaces the Standard case of the old striped test, which is now `a_striped_hair_cards_shadow_is_striped_in_the_shadow_maps` (Hair, and Hair posed).
- **Hair's cost** stays small. The same pipeline cost 0.22 ms more on 36,000 cards, so a hairstyle of a few hundred cards adds a few hundredths of a millisecond.
- **Where it is recorded:** `docs/rendering/shadows.md`, `docs/rendering/hair.md` (Cost), and `wiki/Performance/Benchmark-History.md`. The statements that every cut-out casts through its holes are corrected in the wiki Rendering page, the ADR-0036 note, Known-Limitations and the shader's comments.
- **For Paul:** whether foliage should be able to opt in to leaf-shaped shadows (a graphics setting, or Ultra only) at that cost. It is not built.

### Non-blocking items

Taken up in the docs:
- `hair.md`: under MSAA, any opacity above 0.002 covers samples. So an atlas must keep its clear texels exactly 0 and pad its islands. The ramp card's solid line at its left edge comes from the texture wrapping round (repeat sampler).
- `hair.md` and Known-Limitations: under Off and FXAA, a fringe between 0.2 and 0.5 that isn't drawn still casts a faint stipple.
- `hair.md`: the hashed shadow pattern is fixed in shadow-map texel space, so it may crawl on moving hair. To check on the first hairstyle in Walk.
- `cloth.md`: cloth's anisotropy follows Filament's mapping, α(1±a), not `KHR_materials_anisotropy`'s (α_t = mix(α, 1, strength²)), and not Blender's. Garment authors should judge anisotropy in an engine capture.

Left as reported:
- the OPEN-19 check on real face content, for the face owner before lashes or brows switch to MASK;
- the extra metalness-slot sample in every material's fragment (the Prepass column above moves by 0.002 ms);
- the KHR extensions' textures;
- `DRAWN_TODAY` and the drag rule, which are RA1's when the values are exposed.

The empty stray folder `C:\co7hc_ev` from the first round is still there for Paul to delete.

### Files changed in this round

- `crates/render/src/hair.wgsl`: `hair_shell` and its constants; the shell bit in the header.
- `crates/render/src/renderer/lighting.rs`: `plan_part` sends only untiled hair cards to the cut-out shadow pipeline; `shading_uniform` sets the shell bit.
- `crates/render/src/shader.wgsl`: comments only.
- `crates/render/src/renderer/tests.rs`: registers `foliage_shadow_bench`.
- `crates/render/src/renderer/tests/foliage_shadow_bench.rs`: new.
- `crates/render/src/renderer/tests/alpha_cards.rs`: the tiled and Standard-slab tests; the striped test split.
- `crates/render/src/renderer/tests/hair.rs`: the back-lit shell test; the shell bit in the uniform test.
- `crates/render/src/renderer/tests/character_models_evidence.rs`: the back-lit variants.
- Docs: `docs/rendering/hair.md`, `cloth.md`, `shadows.md`. Wiki: `Engine-Systems/Rendering.md`, `Roadmap/Known-Limitations.md`, `Performance/Benchmark-History.md`, the ADR-0036 note.
- This hand-back, the red-run file, and the evidence: `engine-vs-blender.jpg` and `shadow-map-stripes.png` remade, `backlit-hair-shell-terminator-x3.png` new.

No dependency was added, no shared contract changed, and no generated file was regenerated.

### Tests run

- `cargo nextest run -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-player --no-fail-fast`: `Summary [ 539.252s] 1223 tests run: 1223 passed (29 slow), 29 skipped`. There are no FAIL or ABORT lines. The run includes `every_example_renders`, `the_same_description_captures_identical_pictures_and_every_view_is_drawn`, the probe guard `every_other_material_draws_as_it_did_before_the_skin_model`, `the_skin_table_is_skins_profile_pre_integrated`, and the four new tests.
- The red runs: the tiled test (7 turns), the back-lit shell test (21.1 rows, 0.48), and the Standard-slab test (7 turns). All are in `co7-hair-cloth-red-run.txt`.
- Run by hand: `hair_and_cloth_evidence` (debug), and `bench_cut_out_foliage_shadows` (release, pinned, alternating with `main`, before and after the gate).
- `cargo clippy -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `python3 -I scripts/check-wiki.py`: 114 pages, 0 errors, 58 warnings (staleness on pages this round didn't touch).

## Review fixes 2

The second review found one blocking item. The round-1 terminator fade (`hair_shell`) applied only to single-sided hair, but `hair.md` tells content to make cards double-sided. So no real hairstyle got the fix: a back-lit head of double-sided cards still showed the torn self-shadow band, and nothing reported it. The reviewer was right; I reproduced it.

### The fix: a shell mark for double-sided hair (review option B)

- **The model file marks it.** A new glTF `extras` flag, `axcenity_hair_shell: true`, on a hair material makes it a shell even when it is double-sided. The contract in `hair.md` tells content to set it on the cap and on every card layer lying over the scalp. It stays off only for loose cards that stand clear of the head (a ponytail's length, flyaways), which light from behind should still come through. Single-sided hair is a shell as before.
- **The fade uses the mesh's own normal.** `hair_shell` now measures n·l with the normal as made, not the one turned toward the camera. On a single-sided shell these are the same. On a double-sided shell, the normal as made says which side the head is on, so a card seen from underneath (a fringe from below) still takes light from outside the head.
- **Why not the other options.** A pure-shader fade on every double-sided card (option A) can't tell light through a loose card from light behind a solid head: both arrive from the card's far side. That fade would remove the transmitted glow that `light_from_behind_comes_through_hair` checks, which is real for loose cards. A head-relative normal (option C) needs data the item doesn't carry. The flag is one line in the model file, and the content specialists build on `hair.md`.
- **Plumbing.**
  - `render::HairShading::shell`: bit 4 of `MapsUniform::shading.y` is now `!double_sided || shell`.
  - `assets::ModelMaterial::hair_shell`: read by `hair_shell_of` (hair materials only) and kept in cooked models.
  - `scene`: `model_material` passes it through.

**Test, red first:** `back_lit_double_sided_hair_over_a_head_ends_smoothly_at_its_terminator`.
- It draws the evidence's back-lit ball, double-sided and marked, over a solid Standard ball at radius 0.97, under Off and MSAA 4× at 512 px.
- The limits are the shell test's: the lit band ends within 6 rows of the terminator, and its edge moves under 0.4 rows a column.
- Red, with `shell` a stub: 21.1 rows and 0.48 a column under both modes. Green: 2.6 rows and 0.30, the same as the single-sided shell.
- The uniform test also checks the shell bit for a marked double-sided material.
- The assets and scene tests for the flag went red first too (4 failures, in the red-run file).

**What I saw** (`backlit-double-sided-hair-over-head-x3.png`, the ball's top at 3×):
- **Top row: double-sided and loose, over the head**, as the reviewer's probe drew it. Under MSAA 4× the lit band runs well past the terminator and its inner edge is torn into blotches. Under TAA (40 frames) it averages into texel-sized steps. Loose cards still do this where they close into a surface; it is reported in `hair.md` (Shells, Not yet) and Known-Limitations.
- **Middle row: double-sided and marked a shell.** A thin, smooth crescent under MSAA and TAA alike, matching the single-sided shell (bottom right).
- **Bottom: Blender.** Its crescent is white and a little thicker at the top; the engine's is warm gold at the sides. This is the colour difference from round 1, still to tune on the first hairstyle. The grey patch at the pole is the test ball's artefact (its strand directions meet there), as before.

### Non-blocking items

Done in this round:
- **`anisotropy_rotation` is now -90 to 90 degrees** (it was -180 to 180).
  - A stretched highlight repeats every half turn, so half the old range repeated itself. Under Paul's drag rule the full range now moves about 1.6 degrees a point instead of 3.3.
  - Tests went red first: the avatar check now refuses 120, and `scene` holds 135 at 90.
  - Updated in `architecture.md` §10, `cloth.md` and the value's help text.
- **Redness.** I computed the shader's colour curve. At redness 1, brown (115, 76, 46), hue 27°, becomes (151, 96, 55), hue 26°: lighter, about the same hue, as the reviewer measured. With melanin 0.3 as well it is (124, 65, 28), hue 23°, auburn; with melanin 0.6, (94, 37, 8), a deep copper. `hair.md` now says that a red head is redness 1 with melanin raised, and gives these numbers for content and RA1's presets.
- **Per-lobe roughness or shift.** `hair.md` now says what the model has: one roughness for all lobes (their widths are fixed multiples, as in Karis's and Blender's models), one tilt scale for both shifts, and `highlight` for the secondary's strength.
- **Ray-traced shadows onto the scalp.** I read `cut_out_near` in `ray_shadows.wgsl` and `ray_ao.wgsl`; the reviewer's arithmetic holds (about 16 mm at 3 studs from a solid pixel). I didn't change it. It is noted in `hair.md` "Not yet" and Known-Limitations, to check on the first hairstyle.
- **The Blender scripts are committed** under `co7-hair-cloth/tools/` with a README: `blender_references.py`, `engine_vs_blender_sheet.py` and this round's `backlit_double_sided_sheet.py`. They contain no local paths.
- **The KHR sheen albedo.** `cloth.md` now says the engine dims by E(n·v) only, where the spec takes the smaller of the n·v and n·l terms, and what that changes under a grazing light.

Not done:
- The dark-orange speckles outside the back-lit silhouette in `engine-vs-blender.jpg` are left to check on the first hairstyle.
- The items still open from round 1 are unchanged: TAA flicker at the bottom of the hashed band, the MSAA screen-door, the shadow stipple, the ramp card's wrap line, the pole patch, and OPEN-19 on real face content.
- The stray folder `C:\co7hc_ev` is still for Paul to delete.

### Files changed in this round

- `crates/render/src/draw_list.rs`: `HairShading::shell`.
- `crates/render/src/hair.wgsl`: `hair_shell` uses the mesh's own normal; its comments.
- `crates/render/src/renderer/lighting.rs`: the shell bit is `!double_sided || shell`.
- `crates/render/src/renderer/tests/hair.rs`: the double-sided test; the shell test's measuring moved into `terminator_failures`; the shell bit in the uniform test.
- `crates/render/src/renderer/tests/character_models_evidence.rs`: the double-sided loose and shell variants.
- `crates/assets/src/model.rs`, `model/cooked_model.rs`, `model/tests.rs`: `ModelMaterial::hair_shell`, `hair_shell_of`, its cooked encoding, and tests.
- `crates/scene/src/draw_list/materials.rs`, `draw_list/tests.rs`: the flag passed through, and its test.
- `crates/avatar/src/material.rs`, `crates/scene/src/material.rs`, `material_values_tests.rs`: `anisotropy_rotation` held to -90 to 90.
- Docs: `docs/rendering/hair.md`, `cloth.md`, `models.md`, `docs/development/character-studio/architecture.md`. Wiki: `Engine-Systems/Rendering.md`, `Roadmap/Known-Limitations.md`, the ADR-0036 note.
- This hand-back, the red-run file, the new sheet `backlit-double-sided-hair-over-head-x3.png`, and `co7-hair-cloth/tools/`.

Shared contracts:
- A new optional model-file flag, `axcenity_hair_shell` in glTF `extras`, documented in `hair.md`, `models.md` and `architecture.md` §10.
- A narrower range for `anisotropy_rotation`, a value this branch added.
- The cooked model encoding gains one byte per material. Cooked caches are keyed by engine build, so old ones are just remade.

No dependency was added, no description or manifest enum grows, and no generated file was regenerated.

### Tests run

- `cargo nextest run -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-player --no-fail-fast`: `Summary [ 802.246s] 1224 tests run: 1223 passed (34 slow), 1 timed out, 29 skipped`. No FAIL or ABORT lines.
  - The timeout was `skin_value_tests::skin_tone_values_change_the_picture_the_way_they_mean` (122.8 s under the shared load). It is not this change's: rerun alone it passes in 61.5 s.
  - The run includes `every_example_renders`, the capture determinism test, the probe guard `every_other_material_draws_as_it_did_before_the_skin_model`, the skin table test and the new double-sided test.
- Red runs, in `co7-hair-cloth-red-run.txt`:
  - the double-sided test and the uniform bit (21.1 rows, 0.48 a column; shell bit 0 against 4);
  - the flag's assets and scene tests (4 failed);
  - the rotation range (2 failed).
- `cargo nextest run -p axcenity-editor -E 'test(/character_workspace::panel/)'`: 5 passed.
- Run by hand: `hair_and_cloth_evidence`, for the new double-sided variants.
- `cargo clippy -p axcenity-render -p axcenity-scene -p axcenity-assets -p axcenity-avatar -p axcenity-player -p axcenity-editor --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `python3 -I scripts/check-wiki.py`: 114 pages, 0 errors, 58 warnings (staleness on pages this round didn't touch).
