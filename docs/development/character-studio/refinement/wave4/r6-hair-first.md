# Wave 4 hand-back: R6 hair, the pipeline and the first realistic hairstyle (RA4, CO11)

Branch `r6-hair-first`, from `co7-hair-cloth` (main plus the Hair and Cloth models). Task: the hair card pipeline, **one** first realistic hairstyle to Paul's standard (asset-quality.md §1: Paul signs off the first style before any other is made), hair colour as data, and stubble as a skin layer.

**Outcome in one line:** the pipeline and the style are built, tested and measured; the style is honestly a **draft** (`quality: "draft"`): it fails four measured checks of asset-quality §4 and the review finds visible defects. Stubble's density map is built into ADR-0042's spare channel, but the Skin model can't draw it without a renderer change this task excluded, so the base doesn't offer the layer yet.

## What was built

1. **The pipeline** (`assets/characters/realistic/hair/`, bpy 5.3 + NumPy + Pillow; deterministic; [its README](../../../../../assets/characters/realistic/hair/README.md)):
   - `scalp.py`: the scalp read from the built `RealisticParts.glb`. The head's frame is centred between the ears, which are found as the vertices `Face_Ear_Size_*` moves.
     - The hairline is an angular table (elevation against azimuth) plus a clearance off each ear.
     - Its zero line is found on the body's triangles, and the signed distance is the distance to that line on the skin. The first version used the angle itself as the distance, which was wrong where the line runs up and down: at the temples and sideburns it put the cap's opaque interior right at the line, a hard edge.
     - The density ramps from 0 at the line to 1 over 11 mm at the front and temples and 8 mm elsewhere.
   - `scalp.py` also has the **binding**: each vertex is bound to the nearest surface point with its offset in that point's frame. It takes that point's weights (four at most) and every body shape that moves it by more than 0.2 mm, with the offset turned by the normal's change.
   - `groom.py`, the groom:
     - roots by density per layer (dart throwing);
     - a comb field: away from the crown's whorl, down at the sides and back (a little back over the ears), and swept 12° at the front;
     - length and thickness over the head;
     - each strand walked over the scalp. It stands off by its share of the thickness and waves across. It stops at an ear, or at the hairline: the dense layers stop inside the line (staggered, so tips don't line up), and the fringe may reach up to 4 mm past it.
   - `atlas.py`: the 2048² atlas, baked to the Hair model's layout.
     - Islands: two periodic islands for the cap (its dense interior, and the hairline band that thins to single hairs), plus tiles for coverage, clumps, detail, short, hairline, sparse and flyaway cards.
     - Strands are drawn at 4× with numbered segments, and every channel is read from the winning segment.
     - The maps: colour by per-strand melanin; coverage (clear texels exactly 0, islands 8 px apart); data (R root→tip, G per strand, B streak clusters, A occlusion); flow; and a normal map of the strand relief.
   - `cards.py`: the cards (a strip along each strand, lying in the layer, root to tip down a tile; a top clump gets a second, turned card), the cap (the scalp lifted 0.8 mm), and three levels: hero, gameplay `_LOD1` and crowd `_LOD2`. The crowd cap is built on `Body_LOD2`, projected onto the full surface.
   - `glb.py`: writes the file directly. It holds a copy of the body's skeleton (the same joints, rest and inverse bind matrices), the shapes as sparse targets, and the material's `extras` naming the data and flow maps (Blender's exporter would drop them).
   - `build_hair.py` writes `hair/ShortCrop.glb`, `hair/ShortCrop.build.json` and `items/realistic-hair-short-crop.item.json`.
   - `qa_hair.py`: the measurements below.
2. **The style:** a short textured crop.
   - Length: top 55 to 65 mm, pushed forward with a piecey fringe; sides about 20 to 30 mm; nape about 14 to 18 mm.
   - Triangles: **27,618 hero, 15,980 gameplay, 466 crowd** (budgets 40k, 25k and 3k).
   - Following the head: it carries 43 body shapes. 35 are identity, frame, fat and muscle shapes (`fit: shapes`, `follows`). The other 8 are expression and corrective shapes near the hairline, driven by name like every worn part. Head size is a bone scale, which the skinning follows.
   - I chose the head's own shapes over `avatar::fit`'s binding transfer. The cards are made on the body itself, so a per-vertex surface binding at build time gives the same result without the runtime fit.
3. **Hair colour as data:** eight presets by melanin and redness (`styles.COLOURS`: black, dark brown, brown, auburn, ginger, blonde, platinum, grey), plus two colourings showing roots and highlights (`blonde-dark-roots`, `brown-highlights`). They are item variants, so a description sets them live through the Hair model's values.
   - The atlas is painted a medium brown (eumelanin 0.55, pheomelanin 0.12, about sRGB (72, 51, 33)). Black is about ×8 of that absorption and platinum about ÷8.
   - Grey and platinum add a little dye: grey hair is white strands among pigmented ones, which no single melanin gives.
4. **Capabilities:** the one material `Hair` lists the values it offers. That is every Hair value, because the atlas carries root-to-tip and the streak mask. A test holds that a realistic style offers only what its atlas carries.
   - `limitations` record what it was measured on: both frames, plus `head_size`, `temple_width`, `forehead_height`, `forehead_slope` and `brow_ridge` at their ends.
   - No item parameters (no length or volume shapes yet).
5. **Stubble:** `makeup.py`'s `stubble_mask` fills mask 2's alpha (ADR-0042's spare channel).
   - The region: the beard (the cheeks below a line from the sideburn to above the mouth's corner, the upper lip, the chin, under the jaw onto the neck), soft over about 6 mm, off the lips.
   - It is multiplied by a follicle stipple about every 1.2 mm (splitmix-hashed; a first XOR hash drew stripes).
   - `docs/rendering/skin.md` and ADR-0042 have a line each for the channel's new use.
   - **Not drawn:** the Skin model's layer uniform is full (`skin_layer_uniform`: `params[0..7]`, `toon` and `specular.w` all taken), and this task excluded render changes. So the base manifest does **not** offer a `stubble` layer yet: an offered layer would be a control that does nothing (brief §14.1). The request for RA7 is below.
6. **Capture tool** (additions only): `HAIR_VIEWS` adds `hair-back`, `hair-top`, `hair-hairline`, `hair-temple`, `hair-crown` and `hair-nape`, named through `AXCENITY_CAPTURE_VIEWS`. `VIEWS`, `MORE_VIEWS`, `LIGHTS` and `Settings::default` are unchanged; the existing test now also checks the hair views' names don't clash.

## Files changed

- New: `assets/characters/realistic/hair/` (`README.md`, `styles.py`, `scalp.py`, `groom.py`, `atlas.py`, `cards.py`, `glb.py`, `build_hair.py`, `qa_hair.py`, generated `ShortCrop.glb` 17.5 MB and `ShortCrop.build.json`).
- New: `assets/characters/realistic/items/realistic-hair-short-crop.item.json` (generated).
- Changed: `assets/characters/realistic/makeup.py` (`stubble_mask`).
- Regenerated: `RealisticParts.glb` (the new mask B), `Realistic.build.json` (the mask's digest, `stubble` coverage share).
- Changed: `assets/characters/realistic/README.md`, `assets/characters/catalogue.json` (regenerated).
- Tests: `crates/avatar/tests/realistic_hair.rs` (new); `crates/player/src/hair_tests.rs` (new); `crates/assets/src/model/tests.rs` (mask 2's alpha is stubble's, no longer held empty); `crates/player/src/main.rs` (module line); `crates/player/src/character_capture.rs` (`HAIR_VIEWS`).
- Docs:
  - `docs/rendering/hair.md` (the first style; the tangent gap);
  - `docs/rendering/skin.md` (stubble's channel);
  - `docs/editor/character-studio.md`;
  - `docs/development/character-studio/visual-qa.md` (the hair views);
  - wiki `Roadmap/Known-Limitations.md`, and ADR-0042 (a consequence line).
- Evidence: `docs/development/character-studio/refinement/wave4/r6-hair-first/` (25 files, 4.6 MB), `r6-hair-first-red-run.txt`, and this hand-back.

No dependency added. No third-party source was used (everything is class 5, made by these scripts), so `references/README.md` has nothing new. No shared contract changed: no avatar schema, no `architecture.md` contract, no `format_version`. The item uses fields that already exist: `materials[].values`, `variants`, `limitations` and `quality`.

## Interfaces

- `axcenity/realistic-hair-short-crop`: a `groom` in slot `hair` for `axcenity/realistic`.
  - `groom.cards` = `../hair/ShortCrop.glb#AxcenityRealistic/Hair_ShortCrop` (with `_LOD1` and `_LOD2`).
  - Material `Hair` (hair shading, role primary, 13 offered values); 10 variants.
- The content contract it follows is `docs/rendering/hair.md`: MASK at 0.5, two-sided, roughness 0.3, `axcenity_hair_data` and `axcenity_hair_flow` in `extras`, a normal map at scale 0.5.
- Mask 2's alpha now holds stubble's density (content contract, `docs/rendering/skin.md`).

## Tests

**Red run first** ([r6-hair-first-red-run.txt](r6-hair-first-red-run.txt)):
- The 5 avatar tests failed with "the library has no `axcenity/realistic-hair-short-crop`".
- The 2 player tests failed with "the library has no item …", and widths of 0 px.
- The stubble test failed with chin 0.000.

**Written** (`crates/avatar/tests/realistic_hair.rs`):
- `the_first_style_is_a_card_groom_for_the_realistic_base_that_follows_the_head`: the slot, the base, cards only, fitted by the head's shapes, quality stated, CC0.
- `its_model_is_hair_cut_out_two_sided_and_within_the_budgets`: Hair, MASK, two-sided, colour and data maps, skinned, two simpler levels, triangles within 40k, 25k and 3k, and every followed shape in the file.
- `a_realistic_style_offers_only_the_colour_values_its_atlas_carries`: **the browser-facing capability list**. Every realistic groom lists its values. It offers the root values only if the data map's R runs root to tip, and the streak values only if B marks some strands and not all. Variants set only offered values, and parameters drive only shapes the cards have.
- `its_colour_presets_run_from_black_to_platinum_and_grey`.
- `it_says_which_frames_and_head_ranges_it_holds_on`.
- `the_spare_mask_channel_holds_stubble_over_the_beard_only`: mean alpha on the chin 0.44; forehead and crown 0.

**Written** (`crates/player/src/hair_tests.rs`):
- `the_first_hairstyle_draws_on_both_frames`: from the back and the front it changes 8.2 to 8.8% and 4.1 to 4.2% of the picture. It is drawn (hero level), and changes under 1% of the bottom fifth (the hair's shadow on the neck).
- `the_first_hairstyle_follows_the_heads_width`: from the back the hair is 232 px wide at `temple_width` -1 and 249 px at 1 (960 × 540).
- `measure_hair_stability_and_show_through` and `bench_hair`: ignored, the measurements below.

**Run:**
- `cargo nextest run -p axcenity-avatar -p axcenity-assets --no-fail-fast` and `cargo nextest run -p axcenity-player --no-fail-fast`: the summary lines are under "Final runs" at the end.
- `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings`: clean.
- `cargo fmt --all`.
- `python3 -I scripts/check-wiki.py`: 0 errors.

## Measurements (asset-quality §4; all in [r6-hair-first/measurements.json](r6-hair-first/measurements.json))

| Requirement | Limit | Measured | Pass |
|---|---|---|---|
| Budgets | 40k / 25k / 3k | 27,618 / 15,980 / 466 | yes |
| Volume (outermost hair along the scalp normal, inside the ramp; 27,627 points) | ≥ 8 mm (declared) | median 16.7 mm, 5th percentile 7.3 mm; **7.8% under 8 mm**; min 0.79 (cap only) | **no** |
| Scalp coverage, geometric (rays out through the cards by alpha, inside the ramp) | < 2% | 0.29% | yes |
| Scalp show-through in captures (green vs red skin, 1920 × 1080; all hair pixels / away from the edge) | < 2% | masculine: front 10.7% / **5.7%**, side 2.4% / 0.2%, back 2.6% / 0.0%, top 7.1% / 0.8%. Feminine: front 9.2% / 1.7%, side 2.0% / 0.1%, back 2.0% / 0.0%, top 6.5% / 0.8%. Counts the hair's shadow on the forehead too | **no** (masculine front) |
| Hairline ramp (cover from 10% to 90%) | ≥ 8 mm | front 7 mm, temples **2 mm**, sides and back 7 mm | **no** |
| Card edges at 1080p | none visible | review: visible (below) | **no** |
| TAA stability (consecutive frames 40 to 48, hair pixels) | set at sign-off (CO7 provisional 4.5) | rms 1.93 to 2.55 of 255; 99th percentile 6.0 to 10.0 | provisionally yes |
| Hair inside the body (>1 mm) | 0 | 0 at rest on both frames, at forehead/slope/brow-ridge/head-size ends and every muscle and fat corner. **1 to 2 vertices of ~27k, 5 to 7 mm deep:** at `temple_width` -1 (hero, masculine; gameplay, feminine), by the ear; in Walk and Run (one vertex, gameplay masculine, hero Run); and Idle, Walk, Run feminine (gameplay) | **no** |
| Texel density | scalp ≥ 10 px/cm | cap median 25.4 (5th percentile 20.1); cards 99 along, 138 across | yes |
| GPU (RX 9070 XT, 1280 × 720 High, one run, shared machine) | gameplay < 1 ms per character | one head filling the view: frame 0.571 → 0.715 ms (Scene 0.346 → 0.460, Shadows 0.042 → 0.057, Prepass 0.028 → 0.042). Twenty at gameplay distance: 1.159 → 1.197 ms (about 2 µs each) | yes |
| Motion check helper | | the fit-2 track's player helper isn't merged here; `qa_hair.py` poses the body in Blender (the starter Idle, Walk and Run retargeted as `qa_scene.pose_take` does, 8 frames each) and skins the hair with numpy by the same bones | |

On the "inside" vertices: the depth is measured to the nearest face of the posed body along that face's normal. Near the ear, that face can be the ear's own back, so some of these may be the measure's sign at a thin ear, not hair inside the skull. I didn't establish which; the positions are in the JSON (`deepest_at`). Until shown otherwise, they count against the style.

GPU memory: the hair's four 2048² maps add 85 MB uncooked, as RGBA8 with mips (89.8 → 175.1 MB in the bench). Cooked builds compress them.

## Visual evidence and what I saw

All in [r6-hair-first/](r6-hair-first/), with a sign-off line in its [README](r6-hair-first/README.md):
- the capture matrix: both frames, five bodies, six views, studio, hard, warm and outdoor light, rest, Idle and a walk frame;
- 4K crops of the hairline, temple, crown and nape under studio and hard light, plus the six hair views reduced;
- the colour presets.

Judged against the references' checklist (references/hair §7):
- **What works:**
  - It reads as short hair on both frames and on every body.
  - It follows the head in Idle and Walk; at sheet size I saw no clipping.
  - The nape is a tapered short cut, after the first build's nape tail down the neck was fixed (the back hairline raised from -62° to -37°).
  - The silhouette is a rounded short crop with some piecey texture.
  - No slab shadows: hair cards cast through their holes.
  - Under warm and hard light it reads as brown hair.
- **What fails (visible defects; any one keeps it a draft):**
  - **A broad pale grey sheen** over the top under the studio and outdoor light, so brown hair looks grey from the front (`views-masculine-studio.jpg`, the matrix's face columns). The cards' normals are the scalp's, so this is the Hair model's R lobe on the head's curve. CO7 measured that lobe as wider (26 against 16 rows) and its off-highlight light as higher than Blender's, and said to tune it on the first real hairstyle: this is that evidence.
  - **Brush strokes, not strands, close up** (`close-*-hair-hairline.jpg`, `-temple.jpg`). Clumps read as painted strokes with pale flat patches. The cap's band shows between cards as flat translucent grey areas at the front hairline. Fine dark single hairs at the hairline read as scribbles at 4K.
  - **The crown's whorl is a pinwheel** with a bright fan under hard light (`close-masculine-hard-hair-crown.jpg`): the cap's polar mapping meets at a point, and the cards radiate from it.
  - **The temple's edge is a dark band**, not a soft 8 mm ramp (measured 2 mm). The nape ends in a jagged, blunt line.
  - **Colours:** black, dark brown and brown read right from the back. Auburn and ginger read as drab browns, and blonde, platinum and grey as greys. Redness alone only lightens the colour, as CO7 noted, and the grey sheen dulls every light colour. The presets are calibrated by eye, once, on these captures.
  - It reads curlier than a "straight textured crop": the waves and the short clumps give a tousled look, closer to a short curly crop on the feminine face.

## Open issues

- **For RA7 (render; I changed no shader):**
  1. **Draw stubble.** The proposed packing: mask 2's alpha as a marking-kind layer `stubble` with opacity and a colour (the hair's melanin through the Hair model's `hair_colour` mapping, or a colour value). It darkens as pigment where the stipple is, with roughness about 0.5. The uniform has no free slot, so one of three is needed:
     - pack the six makeup roughnesses into fewer floats, or move them;
     - a second uniform row;
     - drop the moles' seed from `specular.w` by deriving it from the freckles' seed.

     Then the base offers `{id: "stubble", kind: "marking", material: "Skin"}` (`manifests.py`'s `SKIN_LAYERS`) and a capture test per layer, like the makeup tests.
  2. **Tangents for hair without a normal map.** `assets` makes tangents only for a material with a normal map and doesn't read glTF `TANGENT`, so such hair is lit along model-space +X (`NO_TANGENT`). This style works round it with a normal map; hair.md now says so.
  3. **The R lobe's width and strength.** Above: on this style the sheen greys brown hair. Tune it against Blender's Principled Hair on these captures.
- **For the lead (`avatar`; fixed in Review fixes 1 below):** `avatar::randomize` didn't leave drafts out (asset-quality §5 says it must, and that a test holds it; no such test exists). With this branch, Randomize on the realistic base can wear this draft. The stylized items have no `quality` at all, so they are drafts too; filtering drafts would drop them from stylized randomization. That is the lead's call. The browsers branch (`r67-browsers`) hides drafts behind "Show drafts".
- **The style itself, to reach production:**
  - a hairline built of fine baby-hair cards over a softer band instead of the band carrying it;
  - a whorl made from cards, not a polar cap;
  - fewer and wider clumps with more strand contrast inside each tile, so it reads as strands;
  - longer and lower side cards for the 8 mm volume;
  - the near-ear vertices kept off the ear;
  - colour presets re-judged once RA7 tunes the sheen.

  None of this needs a new contract.
- Volume, the hairline ramp and the near-ear check are the numbers to watch when the style is reworked; the TAA limit is Paul's to set at sign-off.

## What the lead must do at merge

- **Generated files I regenerated:** `RealisticParts.glb` and `Realistic.build.json` (the new stubble mask; geometry unchanged, digests updated); `hair/ShortCrop.glb`, `hair/ShortCrop.build.json`; `items/realistic-hair-short-crop.item.json`; `catalogue.json`.
  - `Realistic.base.json`, the presets and the benchmarks are unchanged; their line-ending-only differences were not committed.
- **Rebuild order after merging other tracks:** `build.py` (the body), then `python -I assets/characters/realistic/hair/build_hair.py --style short-crop` (the hair reads the built body and copies its skeleton and shapes: rebuild it whenever the body's skeleton, scalp or shapes change), then `catalogue.py`.
- **Likely conflicts:**
  - `makeup.py` (RA5's file: one added function and the B stack);
  - `crates/player/src/main.rs` (module list);
  - `character_capture.rs` (the `views_named` chain and the doc comment);
  - `docs/rendering/hair.md` and `skin.md`;
  - Known-Limitations;
  - `docs/editor/character-studio.md`, which the browsers track edits too.
- After merge: the full workspace suite, and `qa_hair.py` on the merged body.

## Final runs

- `cargo nextest run -p axcenity-avatar -p axcenity-assets --no-fail-fast`: `Summary [ 18.338s] 372 tests run: 371 passed, 1 failed, 0 skipped`. The failure was `the_realistic_skin_has_its_makeup_and_marking_masks`, which held mask 2's alpha empty. That channel is stubble's now, so the test checks it as a layer like the others (`crates/assets/src/model/tests.rs`). Rerun: `1 test run: 1 passed`. No ABORT lines.
- `cargo nextest run -p axcenity-player --no-fail-fast`: `Summary [ 197.185s] 217 tests run: 216 passed (14 slow), 1 failed, 22 skipped`.
  - The failure was `makeup_tests::blush_changes_the_face_against_bare_skin` ("blush turned off changed the picture", under the parallel load). Alone it passes: `1 passed`. Its means differ in the fourth figure between runs, so it is a determinism flake under load, not this branch.
  - The run includes both `hair_tests`, `every_example_renders` and the capture determinism test. No ABORT lines.

## Review fixes 1

The review found two blocking items. Both are fixed test first; the red run is appended to `r6-hair-first-red-run.txt`.

### 1. Randomize wore the draft (59 of 100 seeds)

- **Fix:** `crates/avatar/src/randomize.rs` `draw_items` leaves out every item marked `quality: "draft"` (asset-quality §5). An item with no `quality` (every item from before the standard, the stylized family's included) is still drawn, so stylized randomization is unchanged. Whether unmarked items count as drafts stays the lead's call; the filter is one line to tighten.
- **Tests:**
  - `crates/avatar/src/tests/randomizing.rs` `randomize_never_wears_an_item_marked_draft`: a fixture library with a draft copy of its hair; 1,000 seeds never wear it, and the unmarked hair is still drawn. Red: `seed 1: ["test/draft-hair", "test/dress", "test/hood"]`.
  - `crates/avatar/tests/realistic_hair.rs` `randomize_never_wears_a_draft_on_either_realistic_frame`: 1,000 seeds from each realistic frame's average cast member, against the shipped library, never wear an item whose quality is `draft`. Red: `masculine seed 0 wears the draft axcenity/realistic-hair-short-crop`.
- **Studio:** the Hair page still lists the style until `r67-browsers`' "Show drafts" filter is merged; the editor was outside this task. Merge that branch with or before this one.

### 2. The limitations claimed checks the measurements contradict

- **Fix:** `build_hair.py` `manifest()` no longer writes a fixed note. Each range's note is the measurement over it, kept as data in `styles.py` (`measured`, next to `holds`, with a comment to rewrite it whenever the style is re-measured). The regenerated item now says:
  - frame: "Built for both frames; a draft on both. Front scalp show-through 5.7% (masculine) and 1.7% (feminine) against 2% allowed; in Idle, Walk and Run one card vertex goes up to 6.4 mm inside by the ear."
  - temple_width: "Measured over this range (draft): at -1, 1 or 2 vertices 6 to 7 mm inside the head by the ear (masculine hero 6.1 mm, feminine gameplay 7.2 mm); none over 1 mm at +1."
  - head_size, forehead_height, forehead_slope, brow_ridge: "Measured over this range (draft): no hair vertex more than 1 mm inside the head at either end, on either frame." (`measurements.json`: every end of these four has `over_1mm` 0 on both frames and all three levels.)
  - No note claims "no scalp through it" any more.
- **Tests** (`crates/avatar/tests/realistic_hair.rs`, reading the committed `measurements.json`):
  - `its_limitations_claim_only_what_its_measurements_show`: no note says "no hair inside" over a parameter with any end that has a vertex over 1 mm inside; no note says "no scalp" while the captured show-through fails; a draft's frame note doesn't start "Fits". Red: `a draft doesn't claim it fits: "fits both frames."` (the temple_width note would have failed next).
  - `it_stays_a_draft_while_a_measured_check_fails`: the item can't be marked production while any `asset_quality_2` or `asset_quality_4` check in `measurements.json` has `pass: false` (the reviewer's non-blocking item; it passes now, as it should, and guards the flip).
- **Regenerated:** `python -I assets/characters/realistic/hair/build_hair.py --style short-crop` (bpy python), then `python3 -I assets/characters/catalogue.py`. Only `items/realistic-hair-short-crop.item.json` and `catalogue.json` changed; `ShortCrop.glb` and `ShortCrop.build.json` came out byte-identical (the build is deterministic).

### Non-blocking items taken

- The tag `straight` is now `wavy` (`styles.py`; the captures read as a short wavy crop).
- `references/hair/README.md` §2.6 has a row for `assets/characters/realistic/hair/` (class 5, CC0-1.0).
- `makeup.py` `stubble_mask`: the hash expression joined onto one line is split inside parentheses. The expression is the same, so the mask is unchanged and nothing was regenerated.
- Docs: Known-Limitations and `docs/editor/character-studio.md` say Randomize never draws a draft.

### Non-blocking items left (recorded, not changed)

- The pipeline criterion is **partial**, not met: the groom has a sinusoidal `wave` only, no curl or coil parameter (one style was made).
- Asymmetric shapes on the hair (`Blink_Left`, `Nose_Sneer_Left`, `Face_Eye_Size_Left`), skin slivers at the whorl's centre, black and dark brown reading grey-brown from the front, and roots and highlights barely reading: for the style's rework, after RA7's sheen tuning.
- The §1 body-suite poses and the slider extremes in captures, and a named reference side by side: before any production claim.
- GPU memory (+85 MB uncooked): for RA7 or the lead before more styles exist.

### Found while re-testing: an editor test this branch breaks

- `axcenity-editor` `character_workspace::shell_tests::every_page_is_drawn_with_its_controls` fails, also alone: it expects the realistic Hair page to say "No hairstyles yet", and the page now lists "Bald" and "Short textured crop". This comes from the item added in the first commit, not from these fixes; the earlier runs didn't include the editor. The editor is outside this task, so I didn't change it. **For the lead:** if `r67-browsers`' "Show drafts" filter keeps "No hairstyles yet" with drafts hidden, merging it fixes this; otherwise change the expectation to "Short textured crop" (or to the hidden-drafts text).

### Runs

- Red: `4 tests run: 1 passed, 3 failed, 147 skipped` (above).
- `cargo nextest run -p axcenity-avatar -p axcenity-assets --no-fail-fast`: `Summary [ 23.258s] 376 tests run: 376 passed, 0 skipped`. No ABORT lines.
- `cargo nextest run -p axcenity-player --no-fail-fast`: `Summary [ 203.254s] 217 tests run: 217 passed (14 slow), 22 skipped`. No ABORT lines.
- `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(/character_workspace::/)'`: `105 tests run: 104 passed (1 slow), 1 failed`; the failure is the one above.
- `cargo clippy -p axcenity-avatar -p axcenity-assets -p axcenity-player --all-targets -- -D warnings`: clean. `cargo fmt --all`: clean.
