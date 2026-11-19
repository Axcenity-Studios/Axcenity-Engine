# Wave 5 hand-back: the first sneakers from scratch (decision 14)

Branch `sneakers-scratch`, from `refinement-wave5`. The task was the first sneakers, modelled from scratch in Blender against Openverse reference photographs (Paul's decision 14). They are measured against Paul's decision 13 row ("correct footwear silhouette, clean toe construction and proper foot fit") and against asset-quality.md §2 and §3 (Footwear: no foot through the sole or upper in any pose, the sole flat on the floor at rest within 2 mm, the toe box deforming only at the toe joint).

**Summary.** `axcenity/realistic-sneakers-low` is a plain white low-top court sneaker built as a shoemaker builds one:

- **Last and sole:** a last made from the realistic foot, and a stitched cupsole with a toe spring and a tread of our own.
- **Upper:** leather, with a toe cap, eyestay facings and a heel counter drawn as stitched overlays.
- **Collar, tongue and lacing:** a padded collar and lining, a padded tongue, ten eyelets, and criss-cross laces tied in a bow.
- **Fit:** it is fitted to the realistic skeleton, follows the foot's bones and follows the body's shapes as smooth maps.

**What passes:** on the 18 cast bodies it holds the feet with no defect at rest, in Idle, in all five held poses and at each simpler level. The tread is flat on the floor at rest, from 1.28 to 1.37 mm under it on every body. The toe box doesn't deform in any pose. It moves without popping or tearing in Idle, Walk and Run. It keeps to the budget (3,940 / 2,264 / 1,316 triangles), its meshes are sound, and no edge is a single layer on any level.

**What still fails:**

- **The stride:** in the walk's and the run's stride, foot corners leave the shoe on all 18 cast bodies, up to 9.1 mm. These are recorded.
- **Texture coordinates:** they fall short of §2 in places, mostly overlap on the simpler levels. These are recorded.
- **What the pictures show:** a faceted sole outline at the toe and the heel, a tongue that stands up as a flat plate, a stiff bow, and a zig-zag in the eyestay facing's edge at the throat.

**It is not ready for sign-off and stays Draft.** It is the first shoe in this repository that keeps the feet in at rest and in the held poses, and the first with a flat tread.

I also added two measures to the footwear check, `exposed` and `toe box`, each with a synthetic test. The new measures found one more defect on the wave-4 sneakers, which is now recorded. They found nothing on the stylized sneakers.

## 1. References (Openverse, looked at, never shipped)

I searched Openverse's API without an account (`api.openverse.org/v1/images/?q=<words>&license=cc0,by,pdm`) for 27 queries and looked at about 300 results as thumbnails. I took 15 at full size and used 12. They are saved outside the repository in `axcenity-w5-sources/references/sneakers-scratch/used/` (the search results and metadata are beside them). The table below is also in `references/clothing/README.md` §10, as links only.

| Ref | Used for | Openverse | Source page | Author | Licence |
|---|---|---|---|---|---|
| 9ee9a035 | Side: plain white leather court sneaker; silhouette, sole height, toe spring, collar line, sole-wall stitch | [Openverse](https://openverse.org/image/9ee9a035-332d-4831-a822-0c0d16318a87) | [flickr](https://www.flickr.com/photos/83136374@N05/49688016318) | ajay_suresh | CC BY 2.0 |
| 15a6f675 | Three-quarter: toe cap and eyestay overlays with double stitching, padded collar | [Openverse](https://openverse.org/image/15a6f675-1667-49fb-b978-4a41a5ef618d) | [rawpixel](https://www.rawpixel.com/image/11524416/white-sneaker-with-pink-led-lights) | not given | CC0 1.0 |
| ecbda6ab | The sole from below: outline, plain rim, tread field, heel area | [Openverse](https://openverse.org/image/ecbda6ab-45c8-43da-a1fc-50972e2fa837) | [flickr](https://www.flickr.com/photos/130010866@N02/16766908790) | spera.de | CC BY 2.0 |
| d7818b24 | A canvas shoe and its sole from below: outline and rim | [Openverse](https://openverse.org/image/d7818b24-10bf-40bf-8025-0e7b8d422ab0) | [flickr](https://www.flickr.com/photos/183535671@N03/48578718317) | Fun with My Shoes | Public Domain Mark 1.0 |
| 092344c7 | From above: toe cap stitching, vamp, eyestays | [Openverse](https://openverse.org/image/092344c7-bb57-4938-a786-4d0a46b5a9e4) | [flickr](https://www.flickr.com/photos/80502454@N00/1698273982) | the bridge | CC BY 2.0 |
| 2a505527 | From above, on feet: toe box and lacing | [Openverse](https://openverse.org/image/2a505527-280b-4eac-8da9-4711dcc90ad9) | [flickr](https://www.flickr.com/photos/62961837@N00/8647879736) | c.j.b | CC BY 2.0 |
| 5529acbd | Close: the toe box creased across the ball (the flex line), stitching, eyelets | [Openverse](https://openverse.org/image/5529acbd-ce72-49a1-a768-74697775d569) | [flickr](https://www.flickr.com/photos/92132559@N00/165050255) | ansik | CC BY 2.0 |
| 4e388e90 | Close: eyelet rings, flat laces, double stitch rows (scale only) | [Openverse](https://openverse.org/image/4e388e90-aaa4-4636-a8e6-d03bea267f8a) | [rawpixel](https://www.rawpixel.com/image/5970294/shoelaces) | not given | CC0 1.0 |
| 0dfa9f5f | Close: flat woven laces criss-crossed through eyelets | [Openverse](https://openverse.org/image/0dfa9f5f-30f9-48b0-9208-8e320a237ac1) | [flickr](https://www.flickr.com/photos/14226960@N00/9315466117) | lamdogjunkie | CC BY 2.0 |
| 66838b71 | Close (back): a padded collar at the heel and its lining | [Openverse](https://openverse.org/image/66838b71-f831-4f46-b02f-cdb64f67747c) | [flickr](https://www.flickr.com/photos/26782864@N00/12156073744) | wwarby | CC BY 2.0 |
| a24cf57c | Construction: trainers cut lengthwise; cupsole wall, footbed, the upper lasted into the sole | [Openverse](https://openverse.org/image/a24cf57c-c6b9-4a0b-8624-81ae7c416523) | [flickr](https://www.flickr.com/photos/41639353@N08/4370537302) | CORE-Materials | CC BY 2.0 |
| 50cfe28c | A wooden shoemaker's last: toe-box room, a last's smooth form | [Openverse](https://openverse.org/image/50cfe28c-87ed-4583-8b92-31b5e18b7721) | [Museums Victoria](https://collections.museumsvictoria.com.au/items/1333096) | not given | CC BY 4.0 |

**Brand designs avoided.** None is modelled:

- no logo, mark or embossing;
- no Converse-style rubber toe cap or foxing stripe;
- no swoosh and no three stripes;
- no contrasting heel tab;
- no known model's perforation pattern.

The tread is our own: a 5 mm plain rim, diamond lugs at a 7 mm pitch, two flex grooves across the ball, and fine bars under the arch. Results that showed a brand clearly (Nike, adidas, New Balance, Converse, Vans, Skechers and others) were not used. Where a used photograph carried a brand's detail, that detail was not taken:

- the side reference's black heel tab and embossed mark;
- the three-quarter reference's light-up sole;
- the canvas shoe's design in the eyelet close-up.

The maps are baked from the shoe's own geometry and from CC0 material photographs (Poly Haven's Leather White and Cotton Jersey, and ambientCG's Rubber004), never from a reference.

## 2. How it is built

The approach is a hand-made method carried out by script: the shoe is modelled by script in Blender's Python, under review. No person sculpted or refined it. Decision 11's "manual modelling" row is still open (see section 8).

1. **The last** (`sneakers_last.py`):
   - Measures the foot as built: masculine 251.1 mm long and 97.8 mm wide across the ball; feminine 242.2 by 88.6 mm. The toe joint is at 0.676 of the foot's length.
   - Builds the foot's signed distance on a 2 mm grid, using Blender's BVH of the skin.
   - Runs the toe box on 11 mm past the toes: the distance is moved forward along the foot ahead of the ball, and the union of the two forms is kept.
   - Blurs the result (4 mm) and joins it to the raw distance by a smooth minimum, so the form is never nearer the foot than the foot itself.
   - Puts surfaces at a given distance off the foot by Newton steps along the field's gradient.
2. **The shoe** (`sneakers_low.py`), at three levels:
   - **Cupsole:** one closed solid.
     - The tread is flat at −1.3 mm, with bevels steeper than 25 degrees at the toe (a toe spring) and at the heel.
     - The wall stands 3.2 mm outside the upper at every height it covers (the arch is filled) and rises to 30 mm at the heel and 21.5 mm at the toe.
     - A lip meets the upper, and an inner wall comes down to the footbed under the foot.
   - **Upper:** one shell, lasted 4 mm into the sole between the sole's walls.
     - It has 32 columns (10 a quarter, 12 for the vamp) and 6 rows.
     - It stands 5.0 mm off the foot, 8.2 mm over the toe box, and 7.4 to 7.8 mm at the opening (room for the collar's padding and for the tongue under the eyestays).
     - The vamp's columns fan from the throat. Its outer columns, from the ball to the throat's corners, make a loop across the toe joint.
   - **Opening:** the collar runs 71 mm high at the heel, dips to 52 mm under the outer ankle bone (the inner side is 4 mm higher) and rises at the front of the ankle to the eyestays.
     - The eyestays run 12.5 mm either side of the foot's middle line, from 40% to 58.5% of its length.
     - The edge rolls over a 5.6 mm padded collar at the heel (5.0 mm at the sides, 2.6 mm at the eyestays) and comes back down inside as the lining.
   - **Tongue:** a padded slab 3.6 mm thick (1.6 mm at its edges), tucked under the vamp and the eyestays.
   - **Eyelets:** five a side, each a closed ring of 5 segments, at full detail only. The simpler levels have a small closed stud each, so the renderer doesn't simplify the full rings.
   - **Laces:** flat, of 3 sides, criss-crossed over the tongue and the eyestays, tied in a bow with two loops and two ends.
   - **Clearance and symmetry:** the right shoe is the left mirrored. Each shoe's upper is then moved out to at least 4 mm off its own foot, because the body's feet are not exact mirrors.
   - **Simpler levels:** their flat faces are moved out where their middles came within 1.8 mm of the foot.
3. **Texture coordinates** (`build_sneakers_low.py`):
   - The full level gets Blender's minimum-stretch unwrap (400 iterations) along seams placed so each piece lies nearly flat: the upper in four pieces plus the collar band and the lining, the sole's wall, lip, inner wall, bevel and both caps apart, each lace side and each eyelet face apart.
   - Each island is scaled to its true size and packed at one density: 2.51 texels per mm, or 25 px/cm against the 8 asked.
   - The simpler levels are unwrapped the same way and each island is laid onto the full level's island by a rigid fit, so they read the same texels.
4. **The maps** (`sneakers_maps.py`, 2048 px):
   - **Leather:** Poly Haven's Leather White grain.
   - **Overlays:** the toe cap, the eyestay facings and the heel counters stand 0.9 mm proud at their edges, each with two stitch rows (1.6 and 3.8 mm in, 3.2 mm stitches). A topline stitch runs 4 mm under the collar. The eyelet holes are dark under the rings.
   - **Textiles:** Cotton Jersey for the textiles, with a woven rib for the laces and a bound-edge stitch on the tongue.
   - **Sole:** Rubber004 for the sole, with its stitch row 5 mm under the lip and the tread pattern above.
   - **Eyelets:** painted metal (metalness 1, roughness 0.35).
   - **Padding:** every texel outside the islands is filled, so mipmaps don't draw the background along the seams.
5. **Weights** (each corner, by bone name):
   - The toes and the foot are split across the ball as the skin under the corner is split (a Gaussian mean, 8 mm).
   - The toe box (from 74% of the foot's length) is on the toes alone.
   - The collar, the tongue's top and the laces near the ankle take the skin's shin share, faded in from 35 to 62 mm up.
   - Round the heel (14% to 32% of the length) the whole shoe, sole included, takes the skin's shin share. The body's own skinning carries the heel's skin partly with the shin when the ankle turns, and a rigid heel cup let that skin out by 5 to 10 mm.
6. **Shapes.** For each of the 21 body shapes that move the foot, the shoe takes one affine map per foot. The map is a least-squares fit of the foot's skin under the shoe. Its height part depends on height alone (no tilt), and its offset is the move of the foot's lowest point. The shoe follows the frame and builds as the foot does, without the skin's bumps, and its tread stays flat. Before the offset rule, the feminine tread was 3.84 mm under the floor.
7. **Fit** (`fit_sneakers_low.py`):
   - Each level goes through the engine's own import (`import_garment --placed`) for the skeleton, the binding kept for refitting, the regions (none hidden: the `feet` region reaches above the collar) and the thumbnail.
   - The build's positions, normals, weights and shapes are then put back, matched by texture coordinate and nearest place. The import's push-out had moved the sole's inner wall and footbed up to 70 mm.
   - The model is written afresh with only what it uses: 4.4 MB.
   - The fit algorithm, the hiding rule, render and the Studio UI are untouched.

## 3. The footwear check: two new measures

`player` `garment_checks::footwear` (asset-quality §3 Footwear) said nothing at rest. Its cover rule follows only corners the shoe covers at rest, so a toe already out of the toe box at rest was never seen, which was the wave-4 reviewers' complaint. It also had no measure of the toe box. I added two measures:

- **`exposed`** (`FootReport::exposed`, `exposed_most`):
  - **What it counts:** a foot corner that is neither covered at rest nor covered now, with the shoe's outside within 2 cm under it along the way into the foot. That face must face the way the skin does, be away from the shoe's hems, and be the shoe's nearest point (within 2 mm). No shoe may lie over the corner within 10 cm. The corner is exposed if it is more than 1 mm out.
  - **When:** counted in every pose, rest included. Corners the cover rule already follows are left to `through`, so nothing is counted twice.
- **`toe box`** (`toe_box_bend`): the most any two corners of each shoe's front quarter change their distance apart from rest. More than 1 mm is a problem.
- **Synthetic tests:**
  - `the_footwear_check_finds_a_toe_out_of_the_toe_box_at_rest`: a toe patch 5 mm in front of a box shoe at rest is exposed by 5 mm, and one 5 mm inside, or a foot inside, is not.
  - `the_footwear_check_finds_a_toe_box_that_deforms`: a rigid 20-degree heel raise bends nothing, and the toe box's sole corners pushed up 5 mm bend it by 5 mm.
  - The existing synthetic test still passes unchanged.
- **How the `exposed` rule was reached.** The first version, with no nearest-face or nothing-over-the-corner conditions, flagged corners inside other shoes' cavities. Those were 36 to 97 corners up to 20 mm, a false reading: the inner floor of the sole was under them. I tightened it and checked against captures. The wave-4 sneakers' toes visibly out at the front still aren't found, because their way out meets the crumpled sole in front and they count as covered.
- **The wave-4 sneakers:** the new measures find one corner exposed on three feminine bodies, 4.35 mm, at rest, with the knees bent and in the walk frame. It is recorded as found in `wardrobe_tests::FOOTWEAR`, as a newly measured defect, not a raised limit.
- **The stylized sneakers:** nothing new found.
- **For the lead's ruling:** these are additions to a shared measure.

`footwear_through(assets, description, clip, frames)` plays a clip once and measures each frame against the rest pose. `foot_report` is now `pub(crate)`.

## 4. Tests

Red first: `sneakers-scratch-red-run.txt`.

- **Avatar crate:** 8 tests ran, 0 passed, 8 failed; the item didn't exist, and `content_states` didn't list it.
- **Player crate:** 13 ran, 1 passed, 12 failed. The two new synthetic tests failed on their assertions (the measures read zero), and the sneaker tests failed because the item didn't exist.

| Command | Result |
|---|---|
| `cargo nextest run -p axcenity-avatar --no-fail-fast` | 214 run, 214 passed, 1 skipped |
| `cargo nextest run -p axcenity-player --no-fail-fast -E 'test(/sneakers_low\|footwear\|hold_the_feet\|garment_checks\|wardrobe_tests\|tee_crew\|fit_gate/)'` | (with `content_state\|character_capture` added) 77 run, 77 passed, no ABORT |
| `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | clean |
| `python3 -I scripts/check-wiki.py` | 115 pages, 0 errors, 57 warnings (staleness notes on pages this branch didn't touch) |

The new tests:

- **`avatar` `tests/realistic_sneakers_low.rs`** (8 tests):
  - the manifest: a fitted built-in draft of original CC0 work with no provenance record;
  - the shading models: leather and rubber Standard, the textiles Cloth with sheen, 2K maps;
  - the budget and the file's own levels;
  - texture coordinates on every level, with `UV_RECORDS`;
  - sound meshes on every level;
  - no single-layer edge on any level;
  - skinning: only the foot chain on each side, the sole's front half on the foot and toes, the toe box on the toes alone;
  - `Frame_Feminine` carried.
- **`avatar` `content_states.rs`:** the item is classified as a wave-5 draft.
- **`player` `sneakers_low_tests.rs`** (10 tests on the 18 cast bodies):
  - footwear at rest and in Idle, in the five held poses, and at each simpler level at rest, with no records;
  - the walk's three thirds and the run, against records;
  - motion in Idle, Walk and Run, with no records.
- **`player` `garment_checks`:** 2 new synthetic tests.

## 5. Measurements

The footwear check on the cast (18 bodies):

| Case | Result |
|---|---|
| Rest, Idle | 0 corners through, 0 exposed, toe box 0.00 mm. Tread 1.28 to 1.37 mm under the floor on every body (feminine average −1.37 to −1.30 mm, masculine average −1.30 mm flat); limit 2 mm |
| Held poses (T-pose, arms raised, arms forward, elbows bent, knees bent) | No problem |
| LOD1, LOD2 at rest | No corner through or exposed; tread flat |
| Walk, every third frame, frames 2 to 44 | Foot corners out on all 18 bodies, up to 9.0 mm (record 9.1 mm in each third); the worst is the heavyset feminine body |
| Run, frames 2 to 35 | The same, up to 9.0 mm (record 9.1 mm) |
| Toe box in every pose | 0.00 mm (the toe box is on the toes alone) |
| Motion, Idle, Walk, Run | No pop, no tear (no records) |

**Where the stride fails.** These places were located in the debug run before the final toe-room change, when the worst was 8 mm.

- **Feminine bodies:** the inner heel at 6.2 cm (about 2 mm); the inner side at 3.5 cm behind the ankle (about 6.7 mm); the outer forefoot at 4 cm during push-off (about 7.4 mm); the little toe at 1 cm (about 2 mm).
- **Masculine bodies:** the inner side at 3.3 cm (about 5 mm).
- **Both frames:** skin 7.5 to 9.5 cm up at the side of the ankle (2.6 to 6.7 mm).
- **Why:** the feminine foot is narrower and shorter than the last, and one affine map a shape doesn't give it its own shape. The ankle skin is a cover-rule edge case: skin just above the rolled collar counts as covered at rest because its way out meets the collar's roll, and it moves away from the collar as the ankle turns.

**Geometry, from asset-quality §2:**

- **Triangles:** 3,940 (pair, LOD0; budget 4,000), 2,264 (57%) and 1,316 (58%).
- **Topology:** all quads except the caps' end triangles, the lace caps and the studs.
- **Faces:** no degenerate face, no edge in more than two triangles, consistent normals, and every raw edge 2 to 25 mm inside a shell on every level.
- **Clearance off the foot (masculine, raw field):** the upper 4.0 mm, the lining 2.15 mm, the tongue 4.0 mm, the sole 3.4 mm.

**Texture coordinates** (share of each material's area within 10% stretch / at 8 px/cm or more):

| Level | Upper | Lining | Tongue | Laces | Eyelets | Midsole | Outsole |
|---|---|---|---|---|---|---|---|
| LOD0 | 99.3 / 99.94 | 97.5 / 99.98 | 100 / 100 | 99.4 / 99.9 | 100 / 100 | 98.2 / 100 | 99.9 / 100 |
| LOD1 | 98.6 / 99.86 | 95.4 / 99.53 | 100 / 100 | 99.7 / 100 | 100 / 100 | 97.8 / 100 | 99.8 / 100 |
| LOD2 | **90.6** / 99.76 | 96.1 / 100 | 100 / 100 | 100 / 100 | 100 / 100 | 97.0 / 100 | 98.3 / 100 |

§2 asks for 95% within 10% stretch and 100% at 8 px/cm.

- **Density:** the shortfalls are a few sliver triangles: at the vamp's convergence on the throat, at the eyestay corner and at the bow's knot.
- **Overlap at full detail:** at most 18 texels in about 27,000 drawn twice.
- **Overlap on the simpler levels:** their islands, laid onto the full level's, overlap their neighbours. The worst shares of texels drawn twice:

  | Material | LOD1 | LOD2 |
  |---|---|---|
  | Laces | 26% | 30% |
  | Midsole | 1% | 19.5% |
  | Upper | 2.5% | 11% |
  | Tongue | 0% | 9.7% |
  | Eyelets | 3.2% | 6.7% |
  | Lining | 1.4% | 1.3% |

All of these are `UV_RECORDS`.

**Other measures:**

- **Shapes carried:** 21.
- **Files:** `Shoes_LowSneakers.glb` 4.4 MB, binding 0.41 MB.
- **Times:** the build about 45 s, the fit about 15 s.

## 6. Visual evidence and what I saw

The sheets are in `refinement/wave5/sneakers-scratch/` (1.6 MB, JPG). Each picture was looked at whole, and cropped where noted.

| Sheet | What |
|---|---|
| `rest-studio.jpg`, `rest-hard.jpg` | 8 cast bodies (both frames: average, slim, heavyset, athletic) standing: side, three-quarter, front, back, top and at the floor |
| `walk-studio.jpg` | The same bodies at `Walk@0.3` |
| `close-studio-hard.jpg` | 3840 × 2160 crops on both average bodies: toe, collar, laces, heel under studio light, and toe and heel under hard light |
| `blender-sole-and-views.jpg` | Blender workbench renders of the pair with its colour map: the soles from below (the tread), the back, the side and three-quarter |
| `body-studio.jpg` | Both average bodies, full body: front, side, back |

**What looks right.** It reads as a plain white leather court sneaker on every body of both frames:

- a cupsole with a visible stitch row and the outsole's edge groove;
- a stitched toe cap, eyestay facings with double rows and a heel counter seam;
- metal eyelet rings, criss-cross flat laces and a bow;
- a padded collar with the lining showing inside, and a slight toe spring.

**Toes and fit.** No skin shows through the shoe at rest in any picture.

- An earlier build showed a skin-coloured nub at the toe of the right shoe on every masculine body. The skin was 0.68 mm out, under the check's 1 mm, so the check passed while the picture failed. I gave the toe box more room (8.2 mm) and kept each shoe at least 4 mm off its own foot, and the nub is gone. I confirmed this both in a 4K capture and by measuring: no foot corner is outside the shoe at rest on any body.
- On the feminine bodies the shoe fits the smaller foot with no visible gap.

**Defects I see** (each one a reason this is not ready for sign-off):

1. **The sole's outline is faceted.** At the toe and the heel the 30 corners round the outline show as straight runs, about 10 mm long at the toe (close-up, hard light). The heel's back reads boxy and vertical.
2. **The tongue's top stands up as a flat plate** above the opening, with a straight, sharp top edge, about 20 mm above the collar's front corner (collar and laces close-ups).
3. **The bow's loops are thick, stiff semicircles.** The laces bunch at the top eyelets (laces close-up).
4. **The eyestay facing's edge zig-zags** near the throat, in the stitched outline (toe close-up, top of picture).
5. **The collar shows no padding bulge from outside.** It reads as a thick rolled edge, and the heel counter is a little tall and upright compared with the side reference.
6. **The toe box is flat across its top** and slightly pointed in plan view. The references' toe boxes are rounder.
7. **The leather is near-white and uniform.** The grain reads only under hard light, and the overlays' 0.9 mm steps read weakly at arm's length.
8. **The walk captures are of limited use.** The fixed foot cameras don't follow the walking character, so one foot leaves the frame. The defects the check finds in the stride (section 5) are not visible in these frames: the stance foot shows no skin through. I didn't capture the push-off frames where the check finds the forefoot's side out.
9. **The soles weren't captured in the engine.** Captured `knees-bent` lifts the feet out of the foot cameras' frame, so the soles are shown by the Blender render only.

## 7. Files

New:

- `assets/characters/realistic/clothing/sneakers_last.py`, `sneakers_low.py`, `sneakers_maps.py`, `build_sneakers_low.py`, `fit_sneakers_low.py`: the scripts.
- `assets/characters/realistic/clothing/sneakers-low/`: `Shoes_LowSneakers.glb`, `.binding.json`, `.png`, `.item.json` (`quality: "draft"`, `origin: "original"`, CC0-1.0). All are generated by the commands in `clothing/README.md`.
- `crates/avatar/tests/realistic_sneakers_low.rs` and `crates/player/src/sneakers_low_tests.rs`: the tests.
- `docs/development/character-studio/refinement/wave5/sneakers-scratch.md`, `sneakers-scratch-red-run.txt` and `sneakers-scratch/` (6 sheets): this hand-back, the red run and the sheets.

Changed:

- `crates/player/src/garment_checks.rs`: the `exposed` and `toe box` measures, `footwear_through`, the 2 tests.
- `crates/player/src/wardrobe_tests.rs`: the wave-4 sneakers' `exposed` records.
- `crates/player/src/character_capture.rs`: `FOOT_VIEWS`, added to `views_named`. No existing view, light or setting was changed.
- `crates/player/src/main.rs`: the module.
- `crates/avatar/tests/content_states.rs`: `WAVE_5_DRAFTS`.
- `assets/characters/catalogue.json`: regenerated by `catalogue.py`; one item added.
- `assets/characters/realistic/clothing/README.md`.
- `docs/development/character-studio/asset-quality.md`: the Footwear row's "measured by" column; no limit changed.
- `docs/development/character-studio/testing.md` and `visual-qa.md`.
- `docs/development/character-studio/references/clothing/README.md` §10.
- `wiki/Roadmap/Known-Limitations.md` and `wiki/Engine-Systems/Characters.md`.

**Interfaces** (all `pub(crate)` in `player`, test-only):

- `garment_checks::{EXPOSED_REACH, TOE_BOX_FROM, TOE_BOX_RIGID, footwear_through, foot_report}`;
- `FootReport::{exposed, exposed_most, toe_box_bend}`;
- `character_capture::FOOT_VIEWS`.

No shared contract changed. Avatar schemas, `architecture.md` and the ADRs are untouched; the manifest uses existing fields only.

**Dependencies:** none. The scripts use the Blender Python environment the wardrobe already uses: bpy 5.3, NumPy, Pillow. The CC0 tiles come from `clothing/fabrics.py` and are not stored here.

## 8. Open issues

1. **The stride.** Foot corners leave the shoe in the walk and the run on all 18 cast bodies, up to 9.1 mm (recorded). The likely next steps:
   - a feminine last of its own (a second build over the feminine foot, carried as the `Frame_Feminine` shape), not one affine map;
   - more room at the inner heel and the side of the ball;
   - for the collar-edge cases, a rule for the lead: a corner whose way out enters the shoe's collar from outside is not covered.
2. **The visible defects in section 6:** the faceted sole outline, the plate-like tongue, the stiff bow, the eyestay zig-zag, the flat toe-box top and the weak leather read. Most need a person in Blender (decision 11's manual row), or more geometry where the budget allows. The sole outline alone would take about 300 more triangles.
3. **Texture coordinates on the simpler levels** overlap (records). A level-specific atlas, or a fit that keeps islands apart, would fix it.
4. **Captures.** The foot cameras are fixed, so walk frames lose a foot, and `knees-bent` lifts the feet out of frame. A camera that follows a bone would capture the stride and the sole.
5. **The 1 mm threshold.** The footwear check's `FOOT_OUT` is the gate's poke-through threshold. A nub 0.68 mm out showed in a 4K capture while every check passed (now fixed in the item, not in the check). Whether shoes should use a smaller threshold is the lead's call.
6. **Paul's sign-off of the first shoe** waits for an item that passes everything.

## 9. For the lead at merge

- **Generated files:** `assets/characters/catalogue.json` was regenerated with `python -I assets/characters/catalogue.py` and adds one item. The item's files were generated by `build_sneakers_low.py` and `fit_sneakers_low.py` (the commands are in `clothing/README.md`; the release `import_garment` example must be built). A body change means rebuilding them: the item carries the body's skeleton and its shapes' maps.
- **Likely conflicts:**
  - `content_states.rs` `WAVE_5_DRAFTS`: the hair-from-scratch branch will add its ID too, so keep both.
  - `character_capture.rs` `views_named`: one `.chain(&FOOT_VIEWS)` line in each of two places, beside any new view set from another branch.
  - `main.rs`: one `mod` line.
- **Shared measure:** `garment_checks::FootReport` gained three fields. Another branch that builds a `FootReport` literal needs `..FootReport::default()`.
- **Records for the lead's review:** the wave-4 sneakers' records in `wardrobe_tests::FOOTWEAR` grew by three `exposed` entries (a new measure's finding, 4.35 mm on three feminine bodies). The walk and run records in `sneakers_low_tests.rs` and the `UV_RECORDS` were first set on this branch, for an item new on it.
- `progress.md` and the plan were not edited.
