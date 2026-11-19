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
- **The silhouette** (found by the review, measured in section 6 item 10): the lacing is short and set back towards the ankle and the vamp is long, so from the front and the three-quarter view the shoe reads as a slip-on or plimsoll, not a court sneaker.
- **The body extremes** (measured in review fixes 1): on the shortest and tallest bodies the tread tilts and sinks up to 9.1 mm, and on the slim feminine body a foot corner leaves the shoe in Idle.
- **Decision 13** (section 6a): none of its three criteria (silhouette, toe construction, foot fit) is met.
- **References:** seven of the 12 references used are branded products (section 1).

**It is not ready for sign-off and stays Draft.** It is the first shoe in this repository that keeps the feet in at rest and in the held poses, and the first with a flat tread.

I also added two measures to the footwear check, `exposed` and `toe box`, each with a synthetic test. The new measures found one more defect on the wave-4 sneakers, which is now recorded. They found nothing on the stylized sneakers.

## 1. References (Openverse, looked at, never shipped)

I searched Openverse's API without an account (`api.openverse.org/v1/images/?q=<words>&license=cc0,by,pdm`) for 27 queries and looked at about 300 results as thumbnails. I took 15 at full size and used 12. Review fixes 1 added about 60 more queries (rawpixel, StockSnap, Wikimedia and museum collections; the anonymous API throttled most of the way) and three more photographs, so 15 are recorded. They are saved outside the repository in `axcenity-w5-sources/references/sneakers-scratch/used/` (the search results and metadata are beside them, review fixes 1's under `review1/`). The table below is also in `references/clothing/README.md` §10, as links only.

**Not an unbranded set.** The brief asked for photographs of plain, unbranded low-tops. Six of the 12 photographs used to model the shoe are branded products, by their Openverse title or tags or by a mark in the picture: Coach, Dunlop Volley, adidas, Nike (twice) and Marina Militare. A seventh shows a Mossimo insole print. The first hand-back described them only as plain shoes, which was wrong. The brand column below now says so. From each branded one only proportions or construction were taken (where its sole, toe box, collar or lacing sits, how a cupsole is built), never a design element: no logo, panel layout, tab, stripe, perforation or tread pattern. That was true when modelling, too, and the model shows no brand design (the review found none either). Review fixes 1 added an unbranded true-profile side view (1cafb6c7) and two unbranded front views (98a63874, 06135a5f). I found no unbranded rear view of a low-top within the API's anonymous limits. The only back reference is still a Nike heel-collar close-up (66838b71). So the set does not yet cover side, top, front, back and sole with unbranded photographs, and acceptance point 1 is **not met**: for the lead's ruling, keep the branded references as disclosed proportion-only sources, or have the unbranded rear view found (an Openverse API key or a photograph Paul supplies).

| Ref | View: what was taken (and what was not) | Brand (from the source's title or tags, or seen in the picture) | Openverse | Source page | Author | Licence |
|---|---|---|---|---|---|---|
| 9ee9a035 | Side (three-quarter from the front): proportions only, the sole's height, the toe spring and the collar line; its black heel tab, embossed mark and panel layout not taken | **Coach** (title "White Sneakers - Coach"; tags coach, coachshoes, tapestryinc) | [Openverse](https://openverse.org/image/9ee9a035-332d-4831-a822-0c0d16318a87) | [flickr](https://www.flickr.com/photos/83136374@N05/49688016318) | ajay_suresh | CC BY 2.0 |
| 1cafb6c7 | Side, true profile (added in review fixes 1): a plain grey canvas low-top; the eyelet run and the throat's place measured from it (section 10); nothing of its design taken | none (no mark visible) | [Openverse](https://openverse.org/image/1cafb6c7-6fca-4aca-82bc-9e2a59e9b41f) | [rawpixel](https://www.rawpixel.com/image/5926912/photo-image-public-domain-fashion-free) | not given | CC0 1.0 |
| 15a6f675 | Three-quarter: the toe cap and eyestay overlays with double stitching, the padded collar; its light-up sole not taken | none named (an unbranded product photograph; its panel layout is close to a well-known basketball court shoe's, and that layout was not copied) | [Openverse](https://openverse.org/image/15a6f675-1667-49fb-b978-4a41a5ef618d) | [rawpixel](https://www.rawpixel.com/image/11524416/white-sneaker-with-pink-led-lights) | not given | CC0 1.0 |
| 98a63874 | Front three-quarter, the pair (added in review fixes 1): the toe box's rounded front and the lacing run seen from the front; the same unbranded product as 15a6f675 | none named (as 15a6f675) | [Openverse](https://openverse.org/image/98a63874-3b99-405d-9f0e-05287a2ad983) | [rawpixel](https://www.rawpixel.com/image/11524393/white-low-top-sneaker-with-purple-led-lights) | not given | CC0 1.0 |
| 06135a5f | Front, on feet (added in review fixes 1): the toe box's height and roundness from the front, the throat's width; its canvas design, foxing stripe, side badge and tongue label not taken | none named; a small badge on the outer side and a tongue label show, unreadable | [Openverse](https://openverse.org/image/06135a5f-7fac-4a09-b5af-781187d35a02) | [stocksnap](https://stocksnap.io/photo/white-sneakers-SLQUL1BR5B) | Dhyamis Kleber | CC0 1.0 |
| ecbda6ab | The sole from below: a cupsole outline with a plain rim, a tread field and a heel area; proportions only, its tread pattern not copied | **Marina Militare** (title "Marina Militare Sneaker 580 ..."; tags marina, marinamilitare) | [Openverse](https://openverse.org/image/ecbda6ab-45c8-43da-a1fc-50972e2fa837) | [flickr](https://www.flickr.com/photos/130010866@N02/16766908790) | spera.de | CC BY 2.0 |
| d7818b24 | A canvas shoe and its sole from below: the outline and the rim; its tread not copied | **Mossimo** (printed on the insole, seen in the picture; not in the title or tags) | [Openverse](https://openverse.org/image/d7818b24-10bf-40bf-8025-0e7b8d422ab0) | [flickr](https://www.flickr.com/photos/183535671@N03/48578718317) | Fun with My Shoes | Public Domain Mark 1.0 |
| 092344c7 | From above: toe cap stitching, the vamp, the eyestays; its curved side overlay not taken | none (no mark visible; the side overlay is the maker's own design and was not copied) | [Openverse](https://openverse.org/image/092344c7-bb57-4938-a786-4d0a46b5a9e4) | [flickr](https://www.flickr.com/photos/80502454@N00/1698273982) | the bridge | CC BY 2.0 |
| 2a505527 | From above, on feet: proportions only, the toe box and the lacing seen top down; nothing of its design taken | **Dunlop Volley** (title "Leather Dunlop Volleys"; tags dunlop, volleys) | [Openverse](https://openverse.org/image/2a505527-280b-4eac-8da9-4711dcc90ad9) | [flickr](https://www.flickr.com/photos/62961837@N00/8647879736) | c.j.b | CC BY 2.0 |
| 5529acbd | Close: the toe box creased across the ball (the flex line), stitching, eyelets; construction only | **adidas** (tag adidas) | [Openverse](https://openverse.org/image/5529acbd-ce72-49a1-a768-74697775d569) | [flickr](https://www.flickr.com/photos/92132559@N00/165050255) | ansik | CC BY 2.0 |
| 4e388e90 | Close: eyelet rings, flat laces, double stitch rows; their scale only | none named (no mark visible; red canvas with white stitching, much like a well-known canvas sneaker, whose design was not taken) | [Openverse](https://openverse.org/image/4e388e90-aaa4-4636-a8e6-d03bea267f8a) | [rawpixel](https://www.rawpixel.com/image/5970294/shoelaces) | not given | CC0 1.0 |
| 0dfa9f5f | Close: flat woven laces criss-crossed through eyelets | none (laces only) | [Openverse](https://openverse.org/image/0dfa9f5f-30f9-48b0-9208-8e320a237ac1) | [flickr](https://www.flickr.com/photos/14226960@N00/9315466117) | lamdogjunkie | CC BY 2.0 |
| 66838b71 | Close (back): a padded collar at the heel and its lining; construction only, its pull loop not taken | **Nike** (tag nike) | [Openverse](https://openverse.org/image/66838b71-f831-4f46-b02f-cdb64f67747c) | [flickr](https://www.flickr.com/photos/26782864@N00/12156073744) | wwarby | CC BY 2.0 |
| a24cf57c | Construction: trainers cut lengthwise; the cupsole wall, the footbed, the upper lasted into the sole; construction only | **Nike** on the upper shoe (a swoosh shows on its upper, seen in the picture; not in the title or tags) | [Openverse](https://openverse.org/image/a24cf57c-c6b9-4a0b-8624-81ae7c416523) | [flickr](https://www.flickr.com/photos/41639353@N08/4370537302) | CORE-Materials | CC BY 2.0 |
| 50cfe28c | A wooden shoemaker's last: toe-box room, a last's smooth form | none (a last) | [Openverse](https://openverse.org/image/50cfe28c-87ed-4583-8b92-31b5e18b7721) | [Museums Victoria](https://collections.museumsvictoria.com.au/items/1333096) | not given | CC BY 4.0 |

**Brand designs avoided.** None is modelled:

- no logo, mark or embossing;
- no Converse-style rubber toe cap or foxing stripe;
- no swoosh and no three stripes;
- no contrasting heel tab;
- no known model's perforation pattern or panel layout.

The tread is our own: a 5 mm plain rim, diamond lugs at a 7 mm pitch, two flex grooves across the ball, and fine bars under the arch. The first hand-back said "results that showed a brand clearly were not used". That was wrong: results showing a logo in the picture (Nike, adidas, New Balance, Converse, Vans, Skechers and others) were set aside, but six used photographs are branded by their records or a mark, and a seventh shows a brand's insole print, as the table says. Where a used photograph carried a brand's detail, that detail was not taken:

- the Coach side reference's black heel tab, embossed mark and panel layout;
- the three-quarter reference's light-up sole and panel layout;
- the canvas shoe's design in the eyelet close-up;
- every tread pattern photographed from below.

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
  - **What it counts:** a foot corner that is neither covered at rest nor covered now, with the shoe's outside within 2 cm under it along the way into the foot. That face must face the way the skin does, be away from the shoe's hems, and be the shoe's nearest point (within 2 mm). Nothing of the same shoe may lie over the corner within 10 cm (since review fixes 1, the pair's other shoe doesn't count: it is not one the corner is inside). The corner is exposed if it is more than 1 mm out.
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
10. **The silhouette: a short lacing run set back and a long, low vamp** (found by the review; not named in the first hand-back). It shows in the front and three-quarter views of `rest-studio.jpg` and `rest-hard.jpg` and in the side view of `blender-sole-and-views.jpg`: the shoe reads as a low slip-on or plimsoll with the laces bunched at the ankle, not a court sneaker. Measured from `Shoes_LowSneakers.glb` (left shoe, LOD0, 266.7 mm long), and from the side references on a grid (fractions of the shoe's length from the heel):

    | | Ours | 1cafb6c7 (unbranded canvas, true profile) | a24cf57c (Nike trainer, sectioned, true profile) | 9ee9a035 (Coach, three-quarter; perspective lengthens the front) |
    |---|---|---|---|---|
    | Eyelets (first to last) | 34.4% to 54.2% (52.9 mm, five a side) | 41% to 65% (five a side) | about 34% to 76% | about 30% to 80% |
    | Throat (where the lacing ends at the vamp) | 54% | 71% | 76% | about 80% |
    | Vamp, throat to toe | 46% of the length | 29% | 24% | about 20% |
    | Top of the shoe over the floor at 80% / 90% | 43.8 / 36.4 mm (sole top 22.9 mm at 90%) | about 35 / 29 mm (sole about 20 mm) | about 47 / 41 mm (a running sole) | not measurable (perspective) |

    The cause is in `sneakers_low.py`: `EYESTAY_FROM = 0.40` and `THROAT = 0.585` of the foot's length put the eyestays 15 to 20% of the length (40 to 55 mm) behind the references', so the toe box's flat top runs on for 46% of the length. The toe itself is not lower than the unbranded reference's: at 80% and 90% it is 6 to 9 mm higher. That part of the review's reading ("a low toe box") isn't borne out by the measurement. What reads low is the long, flat vamp with no lacing over it. The fix is to move the throat to about 70% of the length, lengthen the eyestays to six or seven eyelets a side, and round the toe box's top (item 6). That means rebuilding the shoe and re-fitting it, and the review asked for no model change in this hand-back, so none was made.

## 6a. Decision 13: "correct footwear silhouette, clean toe construction and proper foot fit"

The first hand-back never judged this row. Each criterion, against the evidence above:

| Criterion | Met? | Why |
|---|---|---|
| Correct footwear silhouette | **No** | The lacing is short and set back (eyelets at 34% to 54% of the length against 41% to 65% on the unbranded side reference), and the vamp is long (46% of the length against 24% to 29%), so it reads as a slip-on or plimsoll (section 6 item 10). The sole's outline is faceted at the toe and the heel (item 1), the tongue's top stands as a flat plate (item 2), the collar's padding doesn't show (item 5), and the heel counter is tall and upright. |
| Clean toe construction | **No** | The toe box is flat across its top and slightly pointed in plan (item 6), its sole outline faceted (item 1). The eyestay facing's edge zig-zags at the throat (item 4), and the toe cap is a baked overlay, not a panel. What does hold: the toe box is rigid, bending 0.00 mm in every pose, and no toe shows through at rest. |
| Proper foot fit | **No** | The cast holds the feet at rest, in Idle and in the held poses. In the walk and the run, foot corners leave the shoe on all 18 cast bodies, up to 9.1 mm. On the body extremes the tread tilts and sinks up to 9.1 mm on the shortest and tallest bodies, and a corner leaves the shoe on the slim feminine body in Idle and with the knees bent, 5.69 mm (review fixes 1). |

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
7. **The silhouette** (section 6 item 10): the throat at 54% of the length (the references 71% to 80%) and the vamp at 46% (24% to 29%). It needs the shoe rebuilt with its eyestays moved forward and its toe box rounded; none of decision 13's three criteria is met (section 6a).
8. **The references** (section 1): seven of the 12 used to model the shoe are branded products (proportions or construction only were taken), and no unbranded rear view was found. The lead's ruling is needed: keep them as disclosed, or have the set completed.
9. **The body extremes** (review fixes 1): the tread tilts and sinks up to 9.1 mm on the shortest and tallest bodies, and a corner leaves the shoe on the slim feminine body (5.69 mm), recorded in `EXTREME_RECORDS`. The shoe follows the foot's size by one affine map per shape, which doesn't keep the tread flat when the foot's length changes.
10. **The cover rule's own reach** (found in review fixes 1, not changed): the shared cover rule (`foot_cover`) counts a corner as covered when its way out meets *any* shoe face within 4 cm, the pair's other shoe included. A corner that left its own shoe facing the other foot, nearer than 4 cm in the stride, would still count as covered. `exposed` now ignores the other shoe (review fixes 1); applying the same side rule to the cover rule is the lead's call, since it changes `through` for every shoe.

## 9. For the lead at merge

- **Generated files:** `assets/characters/catalogue.json` was regenerated with `python -I assets/characters/catalogue.py` and adds one item. The item's files were generated by `build_sneakers_low.py` and `fit_sneakers_low.py` (the commands are in `clothing/README.md`; the release `import_garment` example must be built). A body change means rebuilding them: the item carries the body's skeleton and its shapes' maps.
- **Likely conflicts:**
  - `content_states.rs` `WAVE_5_DRAFTS`: the hair-from-scratch branch will add its ID too, so keep both.
  - `character_capture.rs` `views_named`: one `.chain(&FOOT_VIEWS)` line in each of two places, beside any new view set from another branch.
  - `main.rs`: one `mod` line.
- **Shared measure:** `garment_checks::FootReport` gained three fields. Another branch that builds a `FootReport` literal needs `..FootReport::default()`.
- **Records for the lead's review:** the wave-4 sneakers' records in `wardrobe_tests::FOOTWEAR` grew by three `exposed` entries (a new measure's finding, 4.35 mm on three feminine bodies). The walk and run records in `sneakers_low_tests.rs` and the `UV_RECORDS` were first set on this branch, for an item new on it.
- `progress.md` and the plan were not edited.

## Review fixes 1

The review found two blocking problems, both in the record rather than the model, and seven non-blocking ones. The item stays Draft and is not ready for sign-off. Nothing in the model changed: `Shoes_LowSneakers.glb` and its files are as they were.

### Blocking 1: five branded references, undisclosed (fixed in the record; the set is still not unbranded)

The review is right. I checked each record's title and tags and looked at every used picture again:

- **Branded by the record:** 9ee9a035 is Coach, 2a505527 Dunlop Volley, 5529acbd adidas, 66838b71 Nike and ecbda6ab Marina Militare.
- **Branded in the picture:** two more the review didn't name. a24cf57c's upper shoe has a swoosh on its upper. d7818b24's insole is printed "Mossimo".
- **Look-alikes:** two others resemble well-known designs, with no mark visible. 4e388e90 is red canvas with white stitching, like a well-known canvas sneaker. 15a6f675's panel layout is close to a well-known court shoe's.

So seven of the 12 used are branded.

**What I changed:**

- Section 1's table and `references/clothing/README.md` §10 now have a brand column, giving the title, tags or the mark seen, and say what was taken from each (proportions or construction only) and what was not.
- The wrong sentence ("results that showed a brand clearly ... were not used") is corrected in both.
- Three unbranded photographs are added:
  - **1cafb6c7:** a plain grey canvas low-top in true profile (rawpixel, CC0). It is the side reference for the silhouette measurement.
  - **98a63874:** the unbranded white product pair from the front three-quarter (rawpixel, CC0).
  - **06135a5f:** white canvas low-tops from the front, on feet (StockSnap, CC0). They have a small unreadable side badge and a tongue label, which were not taken.

**What I could not fix:**

- **No unbranded rear view.** About 60 more queries found none: rawpixel, StockSnap, Wikimedia, Museums Victoria, and phrases like "back of sneakers", "shoes from behind" and "sneaker heel". The Smithsonian source names were rejected, and the anonymous API returned 429 for long stretches. Every rear view I found was branded (an "All Star" heel, a Stan Smith tab) or not a low-top.
- **The model was built against the branded photographs**, so they stay in the record as what was used.
- **Acceptance point 1 is not met.** The lead's ruling: keep the branded photographs as disclosed proportion-only sources, or have the set completed (an unbranded rear view from an Openverse API key or from Paul) and the remaining branded ones replaced.

### Blocking 2: the silhouette, unreported (now reported and measured)

Added to section 6 as item 10, and judged against decision 13 in section 6a. The review's numbers are confirmed by my own measurement of the GLB:

- the eyelets run from 34.4% to 54.2% of the length (52.9 mm);
- the top of the shoe stands 43.8 mm over the floor at 80% of the length and 36.4 mm at 90% (the review's 42 and 35 mm, within the 1% band I took).

Against the references, the lacing ends at 54% where they end at 71% to 80%, and the vamp is 46% of the length where theirs is 24% to 29%. That is the defect. One part of the review's reading does not hold up: the toe is not low. It stands 6 to 9 mm higher than the unbranded side reference's at 80% and 90%. What reads low is the long, flat vamp.

- **Cause:** `EYESTAY_FROM` and `THROAT` in `sneakers_low.py`.
- **Fix:** move the throat to about 70% and add eyelets. As the review asked, no model change was made.
- **Decision 13:** all three criteria are stated as not met, each with its cause (section 6a). The silhouette is also in section 8 and Known-Limitations.

### Non-blocking items

1. **The FOOT_VIEWS row contradicted the hand-back.** `visual-qa.md` said that `knees-bent` with `feet-back` shows the soles. I looked at the capture (`captures/sole/.../studio-feet-back.png`): it shows only the floor and the hand's shadow. The row now says the cameras are fixed, so `knees-bent` lifts the feet out of every foot view and the soles have not been captured in the engine.
2. **The body extremes were unmeasured. They are measured now, and they fail.**
   - **The run:** the rest, held-pose, walk and run tests over the 36 `-body-` descriptions (12 for each of the masculine, feminine and blend frames).
   - **Walk and run:** within the cast's records, the worst 8.88 mm on the heavy feminine body.
   - **Tread (§3 asks for flat within 2 mm):** at rest the tread is off flat on the shortest and tallest body of every frame. The worst is the shortest feminine body, from −9.10 to −0.38 mm; the masculine shortest runs from −2.65 to +5.51 mm, tilted 8.2 mm front to back. One affine map per body shape doesn't keep the tread flat when the foot's length changes.
   - **Foot through:** on the slim feminine body one corner leaves the shoe in Idle and with the knees bent, 5.69 mm.
   - **The new test:** `sneakers_low_tests::the_sneakers_hold_the_feet_of_the_body_extremes` holds them at rest, in Idle and in the five held poses against `EXTREME_RECORDS` (tread at rest 9.2 mm; through in Idle and with the knees bent, 5.8 mm each). These are first-set records for a test new on this branch, and the stale and SLACK checks hold them down.
   - **What isn't tested:** the stride over the extremes is not a committed test (it would add about 3 minutes); the one-off run's numbers are above.
3. **The `exposed` measure had no negative control, and the control found a real gap (fixed).**
   - **The test:** `garment_checks::the_footwear_check_finds_an_inner_side_out_beside_the_other_shoe` builds the pair as one mesh, as an item ships it: two box shoes 6 cm apart. A patch 5 mm out of the left shoe's inner side, facing the right shoe, must be exposed.
   - **Red:** it failed with 0 exposed against 20 (red run appended to `sneakers-scratch-red-run.txt`). The "nothing of the shoe over the corner" filter (`EXPOSED_CLEAR`, 10 cm) took the other shoe for a shoe over the corner, so a true exposure on the inner side, facing the other foot in the stride, was hidden.
   - **The fix:** only a face of the same shoe, on the same side of the body (the left foot at +X, as the toe-box measure already tells the shoes apart), counts as over the corner. The test passes, and so do the other three synthetic tests.
   - **Effect on the items:** none. Every sneaker, wave-4 and stylized footwear test passes with its records unchanged, and none went stale. This is a change to a shared measure, for the lead's ruling.
   - **Not changed:** the shared cover rule (`foot_cover`) has the same reach across to the other shoe (4 cm). Section 8 item 10 raises that for the lead.
4. **The toe-box measure.** It reads exactly 0 because the shoe's front quarter is weighted to the toes alone, so on this item it proves the weights. The band from the toe joint (about 62% to 68% of the length) to `TOE_BOX_FROM` (75%) is not measured by it. That meets "deforms only at the toe joint", but it doesn't show the joint's own crease. Stated here, not changed.
5. **The tread is built 1.3 mm under the floor** (−1.28 to −1.37 mm on the cast), a deliberate sink within the 2 mm limit, not a tread exactly on the floor. Noted for the lead.
6. **Evidence gaps** are still as the first hand-back said: one walk frame, no push-off frames, no sole captured in the engine, a broken walk top view, and no references side by side in the sheets. No new captures were made in this round.
7. **Records and scope.** The 1 mm `FOOT_OUT` question and the collar-edge cover-rule case are still for the lead's ruling. Scope is unchanged: no fit algorithm, hiding rule, render or Studio UI change.

### Files changed in review fixes 1

- `crates/player/src/garment_checks.rs`: the same-shoe rule in `exposed`, `box_pair` and the negative-control test.
- `crates/player/src/sneakers_low_tests.rs`: `extremes()`, `wearing()`, `hold_the_feet_of`, the body-extremes test, `EXTREME_RECORDS`.
- `docs/development/character-studio/refinement/wave5/sneakers-scratch.md` (sections 1, 6, 6a and 8, this section) and `sneakers-scratch-red-run.txt` (appended).
- `docs/development/character-studio/references/clothing/README.md` §10, `visual-qa.md`, `testing.md`, `assets/characters/realistic/clothing/README.md`, `wiki/Roadmap/Known-Limitations.md`.

### Tests run in review fixes 1

| Command | Result |
|---|---|
| `cargo nextest run -p axcenity-player -E 'test(/garment_checks::the_footwear_check/)'` before the side rule | 4 run: 3 passed, 1 failed (the negative control: 0 exposed against 20) |
| The rest and held-pose tests over the 36 body extremes, one-off, before `EXTREME_RECORDS` | 6 run: 4 passed, 2 failed (the tread on six bodies, a corner out on one) |
| `cargo nextest run -p axcenity-player --no-fail-fast -E 'test(/sneakers_low\|footwear\|hold_the_feet\|garment_checks\|wardrobe_tests\|tee_crew\|fit_gate\|content_state\|character_capture/)'` | 79 run, 79 passed (8 slow), no ABORT |
| `cargo nextest run -p axcenity-avatar -p axcenity-player --no-fail-fast` | 494 run, 494 passed (26 slow), 23 skipped; no FAIL, ABORT or TIMEOUT |
| `cargo clippy -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 115 pages, 0 errors, 57 warnings (staleness notes on pages this branch didn't touch) |
