# Wave 5 hand-back: the first hairstyle modelled from scratch (decision 14)

Branch `hair-scratch`, from `refinement-wave5`. Task: the first hairstyle made from scratch against Openverse reference photographs, to decision 13's row ("actual hair geometry, natural volume and clean silhouette") and asset-quality.md §4.

**In one paragraph.** **The style does not pass. It is not ready for sign-off and stays Draft.** `axcenity/realistic-hair-crop-taper`, a short textured crop with a tapered nape, was groomed from scratch in Blender (by script: guide curves by zone, clumps, cards from the clumps, tiles drawn from the clumps' strands, a cap mapped triangle by triangle) against twelve CC BY 2.0 photographs found through Openverse. It fixes, measurably, four of the wave-4 crop's failures: the hairline's ramp is 9 mm at the front and 10 at the temples (wave 4: 7 and 2), no hair vertex goes more than 1 mm inside the head at any parameter end, body corner or frame of Idle, Walk and Run (wave 4: 5 to 7 mm by the ear), no skin shows through the top in the geometric measure (0.3%), and TAA is steadier (1.1 to 1.9 of 255 against 1.9 to 2.6). It still **fails** two measured checks (1.0% of the scalp under the declared 8 mm of volume; the masculine front's captured show-through 6.3% against 2%) and the review: up close the clumps read as painted strokes, the hairline and nape end in a jagged dark line of clump tips, the crown's whorl is a radiating burst, the back and sides read as a bowl rather than the references' taper, and brown hair reads grey-brown under three of the four lights (the Hair model's sheen, not changed here). A person refining it in Blender is what it needs next (below).

## What was built

1. **References first** (decision 14). Openverse's API (`license=cc0,by,pdm`, no account), 25 searches, about 450 thumbnails looked at; 22 candidates downloaded and looked at full size; **12 kept** for one style: three front, two side, two hairline close-ups, two back, a nape close-up, two from above (the crown's whorl). All CC BY 2.0. Recorded by link, author and licence in [references/hair/README.md §10](../../references/hair/README.md) with the style's measured description (lengths by zone, the whorl, no parting, the taper). The pictures are kept outside the repository (`axcenity-w5-sources/references/hair-scratch/used/`, SHA-256 per file in its `meta.json`) and nothing of them is in any file here.
2. **The style's data** (`assets/characters/realistic/hair/styles.py`, `CROP_TAPER`): the hairline (front 6 cm above the brows, temples a little recessed, a rounded nape line lower than wave 4's), ramps (11 mm front, 9 mm sides, a declared **16 mm nape taper**), the whorl (azimuth 172°, elevation 58°, a 40° clockwise turn fading over 35 mm), and the **zones**: length and height off the scalp by elevation at the front, side and back (front top 46 mm long and 17 mm high, over the ears about 17 mm long and 12.5 mm high, the nape tapering to 10 mm), the comb's sweep and the sides' and nape's lie, the tips' lift on top and settle at the sides, where each zone stops at the hairline, the scissor taper and how the hair lowers toward the line.
3. **The guide groom** (`guides.py`, new): guide curves grown from the zones about every 12 mm (`build_hair.py --guides-blend <file>` writes them as a Blender curve object for a person to look at and edit); clumps per layer taking the zones' values, varied; staggered stops inside the ramp, longer hairline strands allowed past the line only behind the temples; a clump that would cross the line is regrown shorter (it keeps at least two steps otherwise); a clump's strands in its own frame (spread at the root, converging to pointed tips that end at staggered lengths, frizz, strays).
4. **Cards and the atlas from the clumps** (`clump_cards.py`, new): a card per clump along its centre strand (three vertices across, bowed, narrowing 20% to the tip), a second turned card on a fifth of the outer clumps; tiles drawn from clump strands (the 2048² atlas: 6 `under`, 10 `outer`, 10 `hairline`, 12 `fly` tiles and the cap's patch), their normal map carrying each strand's own height so a clump's highlight breaks into strands; the **cap**: scalp triangles past 85% of the ramp, lifted 1.2 mm, each mapped flat into a whole, darker patch with +V along the comb (no polar point, no translucent band). Layers: `under` (low, wide, dense), `whorl` (a fill round the crown), `outer` (the textured layer), `hairline` (fine short clumps), `fly` (strays). Levels: hero 39,370 triangles, gameplay 19,626, crowd 2,944 (the cap on `Body_LOD2` and coarser `under` and `outer` clumps).
5. **The material**: Hair, MASK 0.5, two-sided, roughness 0.45 (the top of the Hair model's 0.3 to 0.45 human range; at 0.35 the grey sheen was broader), the four maps, and `axcenity_hair_shell: true` (every layer lies on the head, as hair.md asks). Binding and skeleton exactly as the first style (`scalp.Binding`, `glb.py`): it carries 37 body shapes: 33 identity, frame, fat and muscle shapes it follows, and 4 brow expression shapes near the hairline.
6. **Pipeline changes kept small**: `scalp.Hairline.ramp` takes an optional `ramp_back` (only this style has one); `qa_hair.py` finds a guide-groomed style's cap by its build report's `atlas.cap` for the texel measure; `build_hair.py` dispatches on `method: "guides"` and writes the tile places into `CropTaper.build.json`. **The wave-4 crop rebuilds byte-identical** (`ShortCrop.glb` SHA-256 `c1ed1c1d…` before and after) and stays the draft it is.

## Files changed

- New: `assets/characters/realistic/hair/guides.py`, `clump_cards.py`; generated `CropTaper.glb` (17.7 MB), `CropTaper.build.json`; `assets/characters/realistic/items/realistic-hair-crop-taper.item.json` (generated).
- Changed: `assets/characters/realistic/hair/styles.py` (`CROP_TAPER`, `ENDS_HELD`), `build_hair.py` (dispatch, `--guides-blend`, tiles and cap in the report, the shell extra), `scalp.py` (`ramp_back`), `qa_hair.py` (the cap's patch), `README.md`; `assets/characters/catalogue.json` (regenerated: the new item).
- Tests: `crates/avatar/tests/realistic_hair_crop_taper.rs` (new, 6 tests); `crates/avatar/tests/content_states.rs` (the crop in `WAVE_5_DRAFTS`); `crates/editor/src/character_workspace/browser_tests.rs` (two draft counts, 1 to 2); `crates/player/src/hair_tests.rs` (the draw and width tests parameterised by style, two new tests for the crop; the ignored measurement and benchmark take `AXCENITY_HAIR_STYLE`).
- Docs: `docs/development/character-studio/references/hair/README.md` §10; `docs/rendering/hair.md` (the second style and what it shows about the model); `wiki/Roadmap/Known-Limitations.md`; this hand-back, `hair-scratch-red-run.txt`, and `hair-scratch/` (23 JPG sheets, 4.0 MB, and `measurements.json`).

No dependency added. No shared contract changed (no schema, no `architecture.md` contract, no ADR): the item uses fields that exist, and `axcenity_hair_shell` is hair.md's existing material extra. No render, fit, clothing or Studio UI change.

## Interfaces

- `axcenity/realistic-hair-crop-taper`: a `groom` in slot `hair` for `axcenity/realistic`; `groom.cards` = `../hair/CropTaper.glb#AxcenityRealistic/Hair_CropTaper` (with `_LOD1`, `_LOD2`); material `Hair` offering the 13 Hair values; the ten colour variants of the first style; `fit: shapes`; `limitations` for both frames and the head ranges, each saying what was measured; `quality: "draft"`; source CC0-1.0 (made here).
- `build_hair.py --style crop-taper [--guides-blend <file>]`; `CropTaper.build.json` gains `atlas.tiles` (each card tile's kind and pixel rectangle) and `atlas.cap`.
- `hair_tests.rs`: `AXCENITY_HAIR_STYLE=<item id>` selects the style `measure_hair_stability_and_show_through` and `bench_hair` wear (default the first style, so existing runs are unchanged).

## Tests

**Red first** ([hair-scratch-red-run.txt](hair-scratch-red-run.txt)): the six avatar tests failed with "the library has no `axcenity/realistic-hair-crop-taper`" (and "a section for the crop's references"); the two player tests with "the library has no item" and 0 px wide hair.

**Written** (`crates/avatar/tests/realistic_hair_crop_taper.rs`):
- `the_crop_is_a_new_card_groom_for_the_realistic_base_that_follows_the_head`: kind, slot, base, validate, cards only, fit by shapes following the frame and the head's identity shapes, CC0, Draft; the wave-4 crop still ships as a draft.
- `its_model_is_hair_cut_out_two_sided_a_shell_and_within_the_budgets`: Hair, MASK, two-sided, marked a shell, colour, data, flow and normal maps, skinned, two simpler levels, 40k / 25k / 3k, every followed shape in the file.
- `its_atlas_tiles_end_in_tapered_tips_not_straight_cuts`: for every card tile (from the build report): no covered texel in its two side columns, its tip tenth covers under half of its middle fifth, and where the strands end varies by more than 4% of the tile's height (a straight cut would put them on one row).
- `its_references_are_recorded_with_accepted_licences`: at least eight Openverse rows in the crop's section, each CC0, CC BY or public domain, none SA, NC or ND.
- `it_stays_a_draft_while_a_measured_check_fails`: every §4 check has a verdict in `measurements.json`; with any failing, the item must be a draft.
- `its_limitations_claim_only_what_its_measurements_show`: as the first style's guard, against this style's measurements.

**Written** (`crates/player/src/hair_tests.rs`): `the_crop_taper_draws_on_both_frames`, `the_crop_taper_follows_the_heads_width` (the first style's tests, parameterised).

**Runs** (summary lines under "Final runs" below).

## Measurements (asset-quality §4; all in [hair-scratch/measurements.json](hair-scratch/measurements.json))

| Requirement | Limit | Measured | Pass |
|---|---|---|---|
| Budgets | 40k / 25k / 3k | 39,370 / 19,626 / 2,944 | yes |
| Volume (outermost hair along the scalp normal, past the ramps; 27,256 points) | ≥ 8 mm everywhere | median 15.2 mm, 5th percentile 10.9 mm; **0.96% under 8 mm** (most 7 to 7.8 mm, over the ears and on the nape just past the taper); min 1.2 mm (the cap where no card lies above) | **no** |
| Scalp coverage, geometric (rays through the cards by alpha, past the ramp) | < 2% | 0.30% | yes |
| Scalp show-through in captures (green vs red skin, 1920 × 1080, studio; all hair pixels / away from the edge) | < 2% | masculine: front 12.4% / **6.3%**, side 4.8% / 0.35%, back 5.4% / 0.12%, top 7.1% / 0.47%. Feminine: front 11.9% / 1.9%, side 4.1% / 0.12%, back 6.2% / 0.34%, top 6.3% / 0.82% | **no** (masculine front) |
| Hairline ramp (cover from 10% to 90%) | ≥ 8 mm front and temples | front 9 mm, temples 10 mm (sides and back 12) | yes (by number; the review fails the edge, below) |
| TAA stability (frames 40 to 48, hair pixels) | set at sign-off (CO7 provisional 4.5) | rms 1.13 to 1.87 of 255, 99th percentile 3.4 to 6.4 | provisionally yes |
| Hair inside the head (> 1 mm) | 0 | **0** at rest on both frames, at both ends of `head_size`, `temple_width`, `forehead_height`, `forehead_slope`, `brow_ridge`, at the fat and muscle corners, and in 8 frames each of Idle, Walk and Run, every level; deepest 0.78 mm (the roots, which sit up to 0.7 mm in the skin by design) | yes |
| Texel density | scalp ≥ 10 px/cm | cap 40 px/cm (5th percentile 39.9); cards 176 along, 167 across | yes |

On the show-through measure (not changed; for the lead's ruling): it leaves out pixels within 12 px of the hair's edge as the declared ramp, but in the face-front view at 1080p the 9 to 11 mm ramp is about 25 to 30 px wide, and the measure counts the hair's shadow on the forehead as show-through too. So part of the 6.3% is the ramp and the shadow. I didn't widen the band: that needs a synthetic test showing real show-through is still caught.

On volume: "everywhere it covers" is read strictly, as no point under 8 mm. The nape's declared 16 mm taper is part of the ramp the measure leaves out, like every ramp; I chose it because the references' napes fade over about 1.5 to 2 cm.

## Visual evidence and what I saw

In [hair-scratch/](hair-scratch/):
- `matrix-{rest,idle,walk}-{masculine,feminine}-average.jpg`: 9 views (front, three-quarter, side, back, face-front, face-three-quarter, face-side, hair-back, hair-top) × studio, hard, warm, outdoor, in the rest pose, Idle and Walk at 0.5 s.
- `cast-rest-studio.jpg`: all ten cast bodies (both frames × average, slim, athletic, heavyset, muscular), face three-quarter and back.
- `close-{masculine,feminine}-{studio,hard,warm,outdoor}.jpg`: the hairline, temple, crown, nape, top and back at 3840 × 2160, reduced; `crop4k-masculine-{studio,hard}-hair-{hairline,temple,crown,nape}.jpg`: 1920 × 1080 crops of the 4K pictures at full size.
- `atlas.jpg`: the colour atlas over grey.

The reference comparison (each reference beside the capture from the same side: front, side, back, top) is kept with the references, outside the repository (`axcenity-w5-sources/references/hair-scratch/compare-crop-taper.jpg`), because a reference picture is never committed.

What I saw:
- **Works:** at sheet size it reads as a short dark crop on both frames and every cast body; no skin shows through the top, the crown or the back (the cap); the hair follows the head in Idle and the walk with no clipping; the front's height and the top's volume are close to the references'; the textured top's ends lift a little; no card's side shows as a straight alpha edge at 1080p sheet size.
- **Fails (each keeps it a draft):**
  - **Silhouette:** the back and sides read as a bowl, not a taper. The hair ends about level with the ears' tops and its lower edge flares slightly outward, where c16d1119 and a34e7d9b narrow into the nape. From the front the fringe line is a rounded band, not e293009a's lifted, uneven front.
  - **Strokes, not strands, close up:** every 4K crop and the 1080p temple view show the clumps as flat painted strokes with straight-sided facets. The per-strand relief in the normal map helped only a little.
  - **Hairline and nape:** the ramp measures 9 to 10 mm, but in the pictures it shows as scattered dark specks on skin under a dense line of clump tips. Over the ears and down the nape the edge is a jagged dark line; the fine nape hair of the taper hardly shows.
  - **Crown:** no skin and no pinwheel of the cap's map, but the whorl reads as a radiating burst of flat card facets with a busy centre, and a pale highlight spot at its middle under the studio and outdoor light.
  - **Colour:** `brown` reads grey-brown under the studio and outdoor light, straw-grey on the lit side under the hard light, and brown only under the warm light.
  - **Hard light:** the hair splits into a lit half and a flat black half along a sharp line over the crown and the back (the shell's terminator, with little ambient light there). It reads as two solid masses, not many strands.

## Open issues

- **For the lead (render, not changed here; the same evidence as R6):**
  1. The Hair model's primary lobe and sky reflection grey brown hair over the head's smooth curve under the studio, hard and outdoor light (`close-*-studio.jpg`, `close-*-hard.jpg`). Roughness 0.45 narrowed it a little against 0.35. Tuning against Blender's Principled Hair is RA7's.
  2. Under a hard side light, a shell's terminator splits the hair into a lit half and a flat black half (`close-masculine-hard.jpg`). Real hair passes some light round the terminator. A softer fade, or some multiple scattering past it, would help. Leaving the shell mark off makes back-lit hair glow instead (hair.md).
  3. The show-through measure's 12 px edge band is narrower than a declared 8 to 11 mm ramp at 1080p, and it counts the hair's shadow. That needs a ruling and a synthetic test before any change.
- **The style itself, to reach production.** Decision 11's second row: a person in Blender, starting from `--guides-blend`'s curves and this file's cards.
  - Shape the back and sides into the references' taper, not a bowl edge.
  - Thin and lengthen the hairline's and nape's fine hair into a soft band.
  - Reshape the whorl.
  - Break the clump strokes, either with more, narrower outer cards (the hero budget has 600 triangles left) or with an atlas of rendered strands rather than drawn lines.

  Scripted tuning took the measurable checks a long way, but the remaining defects are the kind decision 11 gives to hand work. I didn't pass a scripted approximation as finished.
- Volume (0.96% under 8 mm) and the masculine front's show-through are the numbers to watch when it is reworked.
- The hairline and the nape are judged by the review, not the ramp number: the number passes while the picture doesn't.
- Not measured: GPU time (`bench_hair` with `AXCENITY_HAIR_STYLE`), the §1 body-suite poses, and the LOD sheet (the gameplay and crowd copies are within budget and measured inside-head clean, but I didn't capture them side by side).

## What the lead must do at merge

- **Generated files I regenerated:** `hair/CropTaper.glb`, `hair/CropTaper.build.json`, `items/realistic-hair-crop-taper.item.json` (`python -I assets/characters/realistic/hair/build_hair.py --style crop-taper`, bpy python), and `catalogue.json` (`python3 -I assets/characters/catalogue.py`). The build is deterministic: the same body gives the same bytes. `ShortCrop.*` rebuilt byte-identical; its line-ending-only differences weren't committed.
- **Rebuild order after other tracks:** the body first (`build.py`), then both hair styles (they read the built body and copy its skeleton and shapes), then `catalogue.py`. If the body-hiding track hides scalp skin under hair, re-run `qa_hair.py --style crop-taper` and `measure_hair`: the cap stands 1.2 mm off the skin.
- **Likely conflicts:**
  - `crates/avatar/tests/content_states.rs` (`WAVE_5_DRAFTS`: the sneakers track adds its item too; take both);
  - `catalogue.json` (regenerate);
  - `wiki/Roadmap/Known-Limitations.md`;
  - `crates/player/src/hair_tests.rs`, only if another track touches it.
- **The editor:** a second draft hairstyle on the realistic Hair page changes two expectations in `crates/editor/src/character_workspace/browser_tests.rs` ("Drafts: 1" to "Drafts: 2" in `an_empty_page_says_what_is_hidden` and `the_realistic_pages_say_what_they_have`). Only the counts and comments changed, no editor code. The sneakers track adds a draft too: after both merge the count is 3.

## Final runs

- `cargo nextest run -p axcenity-avatar -p axcenity-assets --no-fail-fast`: `Summary [  25.703s] 437 tests run: 437 passed, 1 skipped`. No ABORT lines. (The run before had `content_states::the_shipped_items_are_classified` fail, "`axcenity/realistic-hair-crop-taper` isn't classified here": fixed by listing it in `WAVE_5_DRAFTS`.)
- `cargo nextest run -p axcenity-player --no-fail-fast`: `Summary [ 263.877s] 268 tests run: 268 passed (23 slow), 22 skipped`. No ABORT or TIMEOUT lines. The crop's two tests, alone with output: from the back it changes 8.95% (masculine) and 8.39% (feminine) of the picture and from the front 3.66% and 3.54%, drawn at the hero level; from the back it is 227 px wide at `temple_width` -1 and 243 px at 1.
- `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(/character_workspace::/)'`: first `151 tests run: 149 passed, 2 failed` (the two draft counts above); after the change `Summary [  31.047s] 151 tests run: 151 passed, 1322 skipped`.
- `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` and `cargo clippy -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all`: applied.
- `python3 -I scripts/check-wiki.py`: `115 pages, 0 errors, 57 warnings` (the warnings are older pages' staleness).
- Ignored measurements (release): `AXCENITY_HAIR_STYLE=axcenity/realistic-hair-crop-taper cargo test -p axcenity-player --release -- --ignored measure_hair --nocapture --test-threads 1` (the table above), and `python -I assets/characters/realistic/hair/qa_hair.py --style crop-taper --out <file>`.
