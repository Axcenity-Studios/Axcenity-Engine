# Wave 5, stage 3: hiding the body under clothing (hand-back)

Branch `body-hiding`, from `refinement-wave5`. Task: hide the body (and the underwear) under clothing wherever no hole can show, the usual game approach, so skin and underwear can't come through a garment by construction; prove no hole shows; count poke-through only on skin that is drawn; prove it on the licensed tee, the wave-4 crew tee, the joggers and the test tee.

**In one paragraph.** The rule, the masks, the drawing and the proof are built and hold: no hidden place shows in any gate pose, walk frame or motion-check frame on any gate body, and the before and after captures are identical, pixel for pixel within 24 of 255, in all 640 pictures compared after review fixes 1 (the licensed tee and the wardrobe, eight cast bodies, five poses, the standard views and six new views into the openings, two of them at the joggers' cuff). **But it doesn't reach the defects.** Skin and underwear come through the tees where the cloth flips at the armpit and at the turned hems, and that skin is seen: past the flipped cloth (which would show the garment's inside, a hole, whether its back faces are drawn or not) or up a hem or a sleeve. Hiding it would show a hole there, so the rule keeps it drawn. What the masks hide is skin nobody sees and nothing came through: the licensed tee hides 25 of the body's 22,184 triangles (a patch at the sternum and one between the shoulder blades), 72 of the sports bra's, 14 of the briefs' and 11 of the boxer briefs'; the joggers hide 666 (the thighs and shins inside them) and the crew tee 43 (all after review fixes 1's trim with the fine views). **No poke-through record changed** on any of the four garments; the stylized wardrobe's records are unchanged. Two findings for the lead: the view-based hole check sees the test tee's hidden waist region from below (51 bodies at rest), so imports no longer hide whole regions; and a mask made in the fit's poses is not enough: the runtime's walk shows a few hidden places the fit's walk didn't (the belly seen up the front hem), so a shipped item's masks are trimmed to the runtime's poses by an offline step.

## Files

| File | What |
|---|---|
| `crates/avatar/src/fit/enclosure.rs` (new), `enclosure_tests.rs` (new) | The rule (`hidden_under`, `hide_under`, `HideRule`, `HIDE_RULE`), the hole check (`shows`, `shows_at`, `holes_seen`, `Views`, `Openings`, `view_directions`), the masks (`mask_texels`, `numbered_texels`, `write_mask`, `write_hide_maps`), `layer_masks`. Seven synthetic tests |
| `crates/avatar/src/fit/surface.rs` | `Surface::cast_where` (a cast meeting only the triangles a filter takes, an edge widened by a millionth so a ray can't slip between two triangles, the walk stopped where the ray leaves the grid); the set of triangles tried kept per thread with a cheap hasher. `cast` is unchanged in what it returns |
| `crates/avatar/src/fit/pose.rs` | `hide_poses`: the fit's poses, the arms raised forward, a long stride either way round |
| `crates/avatar/src/fit/import.rs`, `mod.rs`, `examples/import_garment.rs` | The import writes the masks (`Imported::hiding`), and **hides no whole region** (`hides` empty; `unhidden` lists the regions covered whole) |
| `crates/avatar/examples/hide_under_clothing.rs` (new) | The masks of an item already made: `--under`, `--outfit`, `--bodies`, `AXCENITY_HIDE_RULE` (measuring only) |
| `crates/avatar/src/item.rs`, `lib.rs`, `resolve.rs`, `library.rs` | `ItemManifest::hide_maps` (`HideMap { under, map }`, optional), its checks; `Resolved::hide_maps` (`ResolvedHideMap`); the library checks the paths |
| `crates/avatar/src/tests/hiding_under_clothing.rs` (new), `tests.rs`, `tests/shipped_library.rs`, `tests/fitting.rs` | Format, checks and resolving; every shipped mask fits its model; the import's new behaviour |
| `crates/assets/src/hidden.rs`, `model/hide.rs`, `model.rs`, `lib.rs` | `RegionHiding::masks`, `MASK_HIDES`, masking by triangle number for a model without texture coordinates (`Model::without_triangles_where`, `TriangleAt`); the key is unchanged without masks. A test |
| `crates/gameplay/src/description.rs` | The applier hides the body by the worn items' masks, and a worn lower layer by the masks of the items over it |
| `crates/project/src/character_library.rs` | A project's copy of the library takes the masks; its test checks them |
| `crates/player/src/fit_gate_tests.rs` | `Posed::hidden`; `holes` by views (`holes_found`, `FoundHole`); poke-through counts drawn skin only (`AXCENITY_FIT_GATE_COUNT_HIDDEN=1` the old count); `hidden_at_rest`; the test tee's masks from its folder (`TEE_MASKS`); the hiding test rewritten; a synthetic test |
| `crates/player/src/hide_trim_tests.rs` (new), `main.rs` | The trim to the runtime (an ignored test run by hand) |
| `crates/player/src/garment_checks.rs` | The motion check's holes every third frame (below), a lower layer's hidden triangles too |
| `crates/player/src/tee_crew_tests.rs`, `wardrobe_tests.rs` | A test each that the items hide skin and underwear; the wardrobe gate counts holes (it didn't keep them before), in its static poses, and in 30 tests of their own for the walk, idle and run (`wardrobe_holes`) |
| `crates/avatar/tests/intake_placement.rs`, `crates/editor/src/character_workspace/browser_tests.rs` | Their imports skip the masks (`ImportOptions::skip_hiding`) |
| `crates/player/src/character_capture.rs` | `OPENING_VIEWS`: `hem-up-front`, `hem-up-back`, `neck-down`, `sleeve-up` (new, only when named) |
| `assets/characters/realistic/clothing/{tee-crew,crew-tee,joggers}/*.hide*.png`, `*.item.json` | The masks and the manifests' `hide_maps` |
| `crates/avatar/tests/fixtures/test-tee/TestTee.hide*.png` | The test tee's trimmed masks |
| `assets/characters/realistic/clothing/wardrobe.py`, `import/fit.py` | The builds carry the import's masks into the item (`hide_maps`) |
| Docs | `architecture.md` (§8 `hide_maps`, §9 hiding under clothing and the import's change, §16 API), `asset-quality.md` §3 (the rule), `testing.md`, `crates/{avatar,assets}/CLAUDE.md`, `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md`; this file, `body-hiding-red-run.txt`, `body-hiding/*.jpg` |

**Generated files.** The masks and the three manifests' `hide_maps` are mine (commands below). Nothing else was regenerated: no GLB, catalogue, preset or base manifest changed (`catalogue.json` holds no `hide_maps`; the shipped-library test says it is current).

## The rule (`avatar::fit::hidden_under`, `HIDE_RULE`)

A triangle of the body, or of a lower layer, is hidden when every one of its corners:

1. is **enclosed** at rest on every body state: its way out along its normal meets, within 8 cm, cloth whose triangle faces out (cloth facing back towards the skin on the way, a lining, the inner wall of cloth made with thickness, a flipped fold, is passed through);
2. keeps the cloth within **6 cm** in every case;
3. keeps **4 cm** from every point of the garment's open edges in every case;
4. **can't be seen** in any case, and neither can the triangle's middle (`shows`, below), with what rules 1 to 3 hide left undrawn. Drawing a triangle only covers more, so one pass is enough.

The cases: rest and `hide_poses` (the fit's ten poses, the arms raised forward, a long stride either way round) on the fit's body states (`pose_states_of`, 38) and, for a shipped item, the gate's 54 bodies' shapes (not their bone proportions): 1,846 cases a outfit for a tee. The body wears the mask shapes of the layers under the garment (they sink the skin 5 mm and open a gap a viewer can look along). Each **outfit** of layers worn together is checked on its own (the feminine frame's bra and briefs; the masculine frame's boxer briefs; for the crew tee also each with the joggers), and a mesh is hidden where every outfit that wears it hides it: worn all at once, the boxer briefs covered the feminine bodies' seat in the rule and the gate saw it (217 holes at rest).

**Seen** (`shows`): from a direction towards the viewer, a place is covered when the way to the viewer meets a drawn triangle facing the viewer (garments first, then the skin and layers left drawn; back faces aren't drawn, so a flipped fold is seen through). Otherwise what is seen is what lies behind it: cloth facing the viewer within 3 cm (the skin had come out through it; the cloth is seen) covers it; cloth facing away (a garment's inside) is a hole; nothing within 3 cm is a hole unless the place is out through the cloth by at most that with the cloth behind it as the viewer sees it (the cloth's outline). A place seen more than 78 degrees off its normal (`GRAZE`, a cosine of 0.2) isn't counted: drawn, it would be a sliver.

**Views** (`Views::new`, the gate's): 64 directions over the sphere, the capture matrix's front, three-quarter, side and back either way round, along each opening's axis either way and six ways round it at 25 degrees, and through each opening at the place itself (towards points 4 cm apart round each rim a quarter of the way in, and its middle, within 60 cm). Openings are the open-edge loops at least 10 cm long, of the garment and of the layers worn with it; loops looking the same way within 10 degrees share an axis. `Views::fine`, the trim's: those, 256 more directions, rim points 2 cm apart at 2 to 60 hundredths of the way in, counting places to 87 degrees off their normal (a cosine of 0.05). (The licensed tee's fine trims ran with denser gate views, 2 cm at a tenth and two fifths, and fine rim points 1 cm apart; the gate's were thinned afterwards for the wardrobe's time.)

**Masks** (`write_mask`): over the body's texture coordinates (2048 square; the body's islands don't overlap), or by triangle number for the underwear, which has no texture coordinates (256 a row, a texel per triangle, `NUMBERED_WIDTH`). Each triangle's middle reads its own value as the engine reads it; a drawn triangle's texels, one texel wider, are painted over hidden ones, so a simpler copy's triangle (the body's levels of detail) is hidden only well inside hidden skin; a model masked by number keeps every triangle of its simpler copies. A texel two triangles' middles share draws.

**Trim to the runtime** (`player` `hide_trim_tests`). The gate found hidden places the fit's poses didn't show: the belly 12 to 16 mm under the cloth seen from below up the front hem in the runtime walk (9 to 12 bodies a third of the stride), the boxer briefs' waistband under the hem, the bra's lower band. The trim plays what the gate and the motion check play (rest, Idle, the five held poses, every frame of the walk on every body, the run and the idle on the cast), finds every hidden place a view shows, and draws its triangles again. It is an offline step like the captures.

**What I tried and dropped.** (a) A margin from the openings growing with the gap (5 times it, the steepest graze counted): it left 85 of the licensed tee's 289 hidden triangles and the gate still saw the belly. (b) Denser fixed directions instead of the views through the openings (256 over the sphere, rings at 10 to 45 degrees round each axis, 12 a ring): 10 times slower, and a hole the gate found between them was still missed. (c) The whole waist region the old rule hid: seen from below on 51 bodies at rest.

## Measurements

**What the masks hide** (triangles; the body has 22,184, the sports bra 2,407, the briefs 1,214, the boxer briefs 1,568):

| Item | Rule (fit's poses) | After the trim | Rule time |
|---|---|---|---|
| Licensed tee, body | 191 | 25 (163 after a trim by the gate's views, 117 by the fine views at 78 degrees, 25 by the fine views at 87 degrees; unchanged by review fixes 1) | 972 s, 1,846 cases an outfit, 2 outfits |
| its sports bra / briefs / boxer briefs | 164 / 21 / 171 | 72 / 14 / 11 (74 / 14 / 19 before review fixes 1) | |
| Wave-4 crew tee, body | 151 | 43 (128 with the gate's views; 68 after the fine sampled check, 43 after the fine full trim) | 1,668 s, 4 outfits |
| its sports bra / boxer briefs | 118 / 7 | 80 / 7 (108 / 7 with the gate's views) | |
| Joggers, body | 1,046 | 666 (1,014 with the gate's views; 803, then 666) | 592 s, 2 outfits |
| their briefs / boxer briefs | 24 / 144 | 24 / 142 | |
| Test tee (the import), body / sports bra | 83 / 211 | 11 / 48 (11 / 60 before review fixes 1) | in the gate's library build (the gate now takes the kept masks and skips this) |

The crew tee and joggers worn together hide 709 body triangles on the masculine and feminine average bodies (1,142 before review fixes 1), 149 of the boxer briefs' and 104 of the bra's and briefs' (132 before).

Where (`body-hiding/tee-what-is-hidden.jpg`, the tee not drawn and the masks applied): the licensed tee hides a patch over the sternum and a strip down to the navel, a patch between the shoulder blades, small patches on the shoulder tops and the upper arms' backs, the bra's centre and side panels and the briefs' and boxer briefs' waistband sides. Nothing at the armpit, the neck, the hems or the sleeves.

**The gate, licensed tee** (records as committed before this branch, the corrected measure): every pose and every third of the stride **unchanged** (rest 7 bodies 28.2 mm; Idle 53, 37.9; the walk 54 every third, up to 32.5 mm; arms forward 54, 30.2; T-pose 7; arms raised 14; elbows 8; knees 7; the underwear through it as recorded). Holes: **0** in every pose and walk frame after the trim (before the trim: 1 body at rest and in four held poses, 0.4 mm from the cloth; 4 to 12 bodies a third of the walk). With `AXCENITY_FIT_GATE_COUNT_HIDDEN=1` (hidden skin counted, as before) the bodies and the worst are the same at rest and with the arms forward, and for the wardrobe at rest: the skin that comes through isn't where anything is hidden.

**Crew tee and joggers** (`wardrobe_tests`): every record unchanged (the joggers 53 bodies, 31.0 mm at rest, from the waistband; the crew tee 51 to 54 bodies; the layered pairs as recorded). Holes 0 at rest, in Idle and the held poses, and in the new hole tests: the whole stride every third frame on all 54 bodies (18 tests), and every third frame of the idle and run on the cast (6 each). One body (feminine heavy) showed a hidden place in every static pose (6.8 mm from the cloth) when the gate's aperture views were thinned for speed; a trim of that body drew 6 triangles again.

**Test tee** (`fit_gate_tests`): every record unchanged (the walk thirds 54 bodies, 13.5 to 16.5 mm; Walk@18 38 bodies, 10.2 mm; arms forward 44 bodies, 9.4 mm; arms raised 1 body, 2.8 mm; the bra through it as recorded). Holes 0 in every pose and third. Its import no longer hides the waist (below).

**The stylized gate** (`stylized_clothes_cover_the_stylized_cast`): unchanged and passing (it hides by mask shapes, no masks of mine).

**Time.** The gate's hole check looks at about 150 directions a place, and through the openings at points 4 cm apart round each rim (the trim's fine views: 2 cm, six depths, 256 more directions): the licensed tee's rest test takes 15 to 19 s on 54 bodies, a third of the walk 70 to 81 s (the runner stops a test at 2 minutes). The wardrobe's walk thirds took over 2 minutes with holes, so its holes are tests of their own (above). The motion check now looks for holes every third frame (2, 5, 8, ... as the gate's walk), not every frame: every frame took over 2 minutes a clip on the cast. **That is less often than before** (the old check was every frame, by the 1 mm geometric rule), said here for the lead's ruling; the trim plays every frame.

## Visual evidence (`body-hiding/`)

Engine captures at 1920 × 1080, studio light, the eight cast bodies (masculine and feminine slim, average, heavyset, muscular), poses rest, `Walk@0.183`, `Walk@0.367` (right arm back), `Walk@0.7` (right arm forward) and arms raised; views front, side and back, and the new opening views up under the hem front and back, down the neck and up the right sleeve.

- `tee-<pose>.jpg`: before (no masks: a copy of the library without `hide_maps`, `AXCENITY_CAPTURE_LIBRARY`) and after, front, side, back, and the front's difference amplified 8 times.
- `tee-open-<pose>.jpg`: the same from the opening views.
- `tee-what-is-hidden.jpg`: the tee drawn see-through (`AXCENITY_CAPTURE_UNDRAWN`, review fixes 1; the first try renamed its model's object, which drew a placeholder box over the legs, and the sheet was never committed) with the masks applied: what is hidden, seen through.
- `wardrobe-<pose>.jpg`, `wardrobe-open-<pose>.jpg`: the same for the crew tee and the joggers, over the underwear, barefoot (the capture descriptions wear no sneakers: a stricter view of the cuffs than the gate's outfit, which wears them). Since review fixes 1 the wardrobe sheets show the whole body.
- `wardrobe-cuffs.jpg` (review fixes 1): the two new cuff views, whole frames, before, after and the difference x8, eight bodies, five poses.
- `wardrobe-what-is-hidden.jpg` (review fixes 1): the crew tee and joggers drawn see-through (`AXCENITY_CAPTURE_UNDRAWN`) with their masks applied.
- `crops-*.jpg`: full-size crops at the neck, armpit, waist hem and sleeve end, four bodies (masculine average and heavyset, feminine average and slim), rest, `Walk@0.367` and arms raised, front, side and back; the wardrobe's also at the cuffs.

**What I saw.**

- **Before and after are the same picture** in the standard views: 120 of 120 pictures of the licensed tee have no pixel differing by more than 24 of 255 (the difference column is black); the opening views 126 of 128. That is what hiding promises (nothing seen changes), and it is also what it delivers: none of the defects below changed.
- **A sliver of hidden boxer briefs showed** in the first trim's masks: `Walk@0.367`, masculine heavyset, `hem-up-back`, an area 8 × 19 pixels (about 4 × 9 mm) inside the back hem where the boxers' waistband rim was hidden and the arm and a black line showed through instead (22 pixels differ; 1 on masculine average). Most likely a place seen at a graze past the 78 degrees the gate's views count (`GRAZE`): the trim with places counted to 87 degrees (`Views::fine`) drew far more again (117 to 25 body triangles), and the recaptured pictures match the unmasked ones in every pixel. The wardrobe was first trimmed with the gate's views only; review fixes 1 found that the fine views saw hidden places on every cast body and trimmed it again with them in every pose (below).
- **The wardrobe's own defects, the same before and after** (`wardrobe-Walk-at-0_367.jpg` and the others): black tears at the joggers' front by the crotch and at the back of the waist on the masculine bodies, skin specks at the tee's sides by the waist, black specks on the feminine bodies' chests, and the tee's armpit folds.
- **The defects the gate and the earlier hand-backs hold, all still there and unchanged** (`tee-licensed.md` section 5): the torn, flipped flaps at the front of the armpit in the walk with skin between them; skin specks of about 6 mm through the belly above the navel on the masculine bodies (front view, every walk frame; from below, two dark slivers under it); the hem riding into the crotch and wrapping the seat (a bodysuit look); the long flaps under the sleeves with the arms raised; the strips under the arms in the T-pose; the waist hem's centre back hanging in a point; the dark lip at the back of the neck. Hiding can't take any of these out: the skin at the armpit and the belly specks is seen through flipped cloth or up the hem, and the others are cloth defects.
- The reveal sheet (`tee-what-is-hidden.jpg`) shows the hidden patches as holes with the tee not drawn: they are where the tee lies close and far from every opening. The feminine bodies' briefs lose a small patch at each hip under the hem.

## Tests

**Red** (`body-hiding-red-run.txt`): the new tests with the implementation stubbed (nothing hidden, no hole found, masks not read, not resolved, not checked): 14 run, 3 passed (format and geometry helpers the stubs leave alone), 11 failed, each for the reason expected.

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-avatar -p axcenity-assets -p axcenity-gameplay -p axcenity-project --no-fail-fast` | **739 tests run: 739 passed** (2 slow), 2 skipped; no ABORT |
| `cargo nextest run -p axcenity-player --no-fail-fast` | **299 tests run: 299 passed** (59 slow), 23 skipped; no ABORT |
| `cargo nextest run -p axcenity-player -E 'test(/wardrobe_tests::\|tee_crew_tests::\|garment_checks::\|fit_gate_tests::/)'` | 88 tests run: 88 passed (after the floors were set) |
| `cargo clippy -p axcenity-avatar -p axcenity-assets -p axcenity-gameplay -p axcenity-project -p axcenity-player --all-targets -- -D warnings`; `-p axcenity-editor --tests` | clean |
| `cargo fmt --all` | done |
| `python3 -I scripts/check-wiki.py` | 115 pages, 0 errors (57 staleness warnings on other pages) |
| The trims (above) | `test result: ok` each |

New tests: `avatar` `fit::enclosure::tests` (7), `tests::hiding_under_clothing` (3), `shipped_library::every_hide_map_is_there_and_fits_its_model`; `assets` `hidden::tests::a_mask_leaves_out_the_triangles_over_its_red_texels`; `player` `fit_gate_tests::hidden_skin_isnt_counted_through_the_cloth_and_shows_only_through_a_hole`, `tee_crew_tests::the_licensed_tee_hides_the_skin_and_underwear_it_encloses`, `wardrobe_tests::the_wardrobe_hides_the_skin_and_underwear_it_encloses` and the 30 `the_wardrobe_shows_no_hidden_place_*`; `project`'s package test checks the masks are copied. Changed: `fit_gate_tests::wearing_the_tee_hides_the_skin_it_encloses_and_taking_it_off_shows_it`, `fitting.rs`'s import test (no whole region; a mask), the import tests that don't need masks skip them (`ImportOptions::skip_hiding`).

## Interfaces and contracts

- **A contract addition for the lead's ruling:** the item manifest's optional `hide_maps` (`[{under?, map}]`, architecture §8). Older engines ignore it and draw the skin; an engine with it reads a manifest without it as before. No version bump; no ADR written (the lead decides whether one is wanted: it extends ADR-0035's region hiding with masks).
- **A rule change for the lead's ruling** (asset-quality §3): poke-through counts only drawn skin; the hole check is by views and must hold with no record. Nothing loosened: no limit changed, no record raised.
- **A behaviour change:** `import_garment` hides no whole region (`hides` empty). The pose-safe rule's waist was seen from below by the view-based check.
- Rust: `avatar::fit::{hidden_under, hide_under, HideInput, HideCase, Placed, HideRule, HIDE_RULE, HideReport, encloses, shows, shows_at, holes_seen, SeenHole, Views, Openings, view_directions, write_hide_maps, write_mask, mask_texels, numbered_texels, layer_masks, hide_poses, GRAZE, FINE_GRAZE, BEHIND, BODY_MASK, LAYER_MASK, NUMBERED_WIDTH}`, `Imported::hiding`, `Surface::cast_where`; `avatar::{HideMap, ResolvedHideMap}`, `ItemManifest::hide_maps`, `Resolved::hide_maps`; `assets::{RegionHiding::masks, MASK_HIDES, TriangleAt, Model::without_triangles_where}`. `RegionHiding` gained a field: a struct literal elsewhere needs `masks`.
- No new dependency.

## Commands

- An item's masks: `AXCENITY_HIDE_RULE` unset, `cargo run -p axcenity-avatar --release --example hide_under_clothing -- assets/characters <item> --outfit <layers,...>... --bodies assets/characters/benchmarks` (the licensed tee: `--outfit axcenity/realistic-sports-bra,axcenity/realistic-briefs --outfit axcenity/realistic-boxer-briefs`; the joggers: `--outfit axcenity/realistic-briefs --outfit axcenity/realistic-boxer-briefs`; the crew tee: those two and each with `axcenity/realistic-joggers`). Then the manifest's `hide_maps` from its output (I set them with a small script keeping the build's layout; `wardrobe.py` and `import/fit.py` now carry an import's masks themselves).
- The trim: `AXCENITY_TRIM=tee-crew|wardrobe|test-tee cargo test -p axcenity-player -- --ignored trim_hide_maps --nocapture` (`AXCENITY_TRIM_VIEWS=gate` is faster but leaves the gate's 78-degree blind spot, so every shipped item is trimmed with the default fine views: the wardrobe's full fine trim took 1 h 55 min on a release build; `AXCENITY_TRIM_ONLY=<body>` for some bodies). About 80 minutes for the licensed tee, 2 hours for the wardrobe with the gate's views, 30 minutes for the test tee. I ran: the licensed tee three times (the gate's views, the fine views, the fine views at 87 degrees), the test tee twice, the wardrobe once with the gate's views and once for the feminine heavy body.

## Open issues

1. **Hiding doesn't fix the tees.** The skin and underwear that come through are seen, through flipped folds or up a hem. The fit (the drape-preserving specialist's work) has to stop the cloth flipping; then this rule hides what the cloth keeps covered.
2. **The masks depend on the fit.** A garment fitted again needs its masks made and trimmed again (an hour or more each); the gate's hole check catches stale masks. The test tee's trimmed masks are kept beside its source and replace its import's.
3. **Other animations aren't proven.** The masks are trimmed to the gate's poses and the motion check's three clips. A jump, crouch or sit may look up a hem at a hidden place. Bone proportions (the gate's tallest, shortest, widest-hips bodies) are in the trim but not in the rule.
4. **Masks are per item.** Two items worn together hide the union of what each hides alone; the wardrobe outfit is checked together by the gate, other combinations aren't.
5. **The motion check's holes run every third frame** (see Time).
6. **The masks are made in hours, not seconds,** and the gate's hole tests for an outfit that hides much (the joggers) are split thirty ways to stay inside the runner's two minutes.
7. **The test tee's masks are kept in the repository** (`crates/avatar/tests/fixtures/test-tee/`), not made by the gate: if its fit changes, delete them, let the gate's import make them (minutes) and run `AXCENITY_TRIM=test-tee`.

## Merge notes

- The masks are made against today's fit. If the fit-drape branch refits the licensed tee, the crew tee, the joggers or the test tee, their masks must be made and trimmed again (commands above), or the gate's holes fail.
- `ItemManifest`, `RegionHiding` and `Posed` gained fields: other branches' struct literals need them (`hide_maps: Vec::new()`, `masks: Vec::new()`, `hidden: Vec::new()`).
- `Surface::cast` now delegates to `cast_with` (same results); a branch editing `cast`'s walk merges there.
- `fit::pose.rs` gained `hide_poses` at its end; `character_capture.rs` gained `OPENING_VIEWS`.
- Line endings: the manifests and PNGs are written with LF; no line-ending-only change is committed.

## Review fixes 1

The reviewer found three blocking problems. All three are fixed. The third showed that the wardrobe's masks hid far more than the gate could prove, which is a real finding.

### 1. The reveal sheet was not committed

`body-hiding/tee-what-is-hidden.jpg` is now committed, and `body-hiding/wardrobe-what-is-hidden.jpg` is added. The first attempt renamed the tee's model object in a copy of the library. A worn part whose model can't be found draws a placeholder box, and that box covered the legs, so the sheet was never made. The capture now has `AXCENITY_CAPTURE_UNDRAWN=<item IDs>`. It sets those worn items' parts to `Transparency` 1 after the description is applied (`character_capture::undraw`), so their masks still apply. A test was written first and seen failing (`body-hiding-red-run.txt`): `character_capture::undrawn_items_are_see_through_and_keep_hiding`. Both sheets show the eight cast bodies at rest and in `Walk@0.367`, front, side and back.

What they show (hidden places read as see-through holes, dark where the inside of the body is seen; sizes measured at about 1.9 mm a pixel in the 1920 × 1080 front view):

- **Licensed tee:** the hidden areas are three body patches. A sternum patch, roughly 6 × 8 cm. A thin strip about 9 × 1.5 cm under the left pectoral, on the masculine bodies. A patch between the shoulder blades, right of the spine, about 6 × 6 cm. The sports bra's centre back is also hidden (about 5 × 5 cm), and in the walk so are part of the briefs' seat at the hip (about 10 × 6 cm, seen from the side) and a little of the boxer briefs. Nothing is hidden at the armpit, neck, hems or sleeves.
- **Wardrobe (crew tee and joggers):** most of what is hidden is the legs. Irregular patches cover the thighs and shins front and back, starting about 5 cm below the underwear's leg openings and stopping at least about 15 cm above the cuffs at rest. The crew tee hides a few small torso patches (sternum, one on the left upper chest, between the shoulder blades) and the bra's centre back. Part of the briefs' seat is hidden on the feminine bodies. The patch edges are jagged because masks follow triangles; nobody sees them when the clothes are drawn.

### 2. No view up the joggers' cuffs

`OPENING_VIEWS` gains two views: `cuff-up` (from 1.1 studs in front of the left leg, 0.06 above the floor, looking up at the cuff) and `cuff-close` (0.34 in front, looking steeply up into the gap between the rib and the ankle). The existing `VIEWS`, `LIGHTS`, `MORE_VIEWS` and `Settings::default` are unchanged. The crops now include the cuffs (`crops-wardrobe.jpg`). `wardrobe-cuffs.jpg` shows both cuff views as whole frames, before and after, with an 8x difference column, for the eight cast bodies at rest, `Walk@0.183`, `Walk@0.367`, `Walk@0.7` and arms raised.

**Before and after, measured:** the wardrobe was captured with masks off (a copy of the library without `hide_maps`) and with masks on, after the trims below. All **360 of 360 pictures** match: 8 bodies, 5 poses, and front, side, back, hem-up-front, hem-up-back, neck-down, sleeve-up, cuff-up and cuff-close. No pixel differs by more than 24 of 255; the largest channel difference anywhere is 1 of 255, in 125 pictures (temporal anti-aliasing noise). The licensed tee was captured again after its masks changed: **280 of 280** match, with a largest difference of 1 of 255. The captures wear no sneakers, so the cuffs are seen open, a stricter view than the gate's outfit.

### 3. The gate does not count places seen more than 78 degrees off their normal

- **The limit is now pinned by a test.** `fit::enclosure::tests::a_hidden_rim_seen_at_a_graze_is_left_to_the_fine_views` hides a ring of skin 10 cm up a sleeve that sits 1.5 cm off the skin. That ring can be seen only along the sleeve, at most about 8.5 degrees off its axis, which is 81.5 degrees or more off the skin's normal. `Views::new` (the gate's views) finds nothing. `Views::fine` finds the ring, and every place it finds is seen between 78 and 87 degrees. The test checks existing behaviour, so it passed as written. It was seen failing against a mutation: with `GRAZE` set to `FINE_GRAZE`, the gate's views found the ring (`body-hiding-red-run.txt`).
- **asset-quality §3** now calls this a limit of the gate in its "Hidden skin and layers" row: a shipped item's masks must also be trimmed or checked with the fine views, with 0 places left. The trim's notes say the same.
- **The wardrobe was checked, and the blind spot was real.** I ran the trim's fine views on the cast at rest, with the arms raised and every third walk frame. This is the new `AXCENITY_TRIM_POSES=sample`, run with `AXCENITY_TRIM_ONLY=-cast-`. The masks made with the gate's views showed hidden places on **every one of the 18 cast bodies**: 139 to 717 places a body (a place is a corner or middle found in a frame). The trim drew back 271 body triangles and 5 of the bra's.
- **Full re-trim with the fine views:** `AXCENITY_TRIM=wardrobe`, every pose the trim plays, on all 54 bodies, 6,899 s. It drew back 162 more body triangles and 23 of the bra's, and found places on 52 of the 54 bodies (up to 806 places a body).
- **Wardrobe masks now:** the crew tee hides 43 body triangles (from 128) and 80 of the bra's (from 108). The joggers hide 666 (from 1,014). The boxer briefs (7 and 142) and briefs (24) are unchanged. A final fine sampled check on the cast found **0 places on all 18 bodies**.
- **The tees, checked the same way:** the licensed tee's and the test tee's earlier fine trims used fine views from before they were thinned. The current fine views, in a sampled check, still found places: the licensed tee's boxer briefs (5 triangles) and bra (2), and the test tee's bra (11). A full fine trim of both then drew back 3 more of the licensed tee's boxer-brief triangles and 1 more of the test tee's bra.
- **The tees' masks now:** the licensed tee hides 25 body, 72 bra, 14 briefs and 11 boxer-brief triangles. The test tee hides 11 body and 48 bra triangles (the fixture is updated). Every shipped item and the test tee are now trimmed with the fine views in every pose the trim plays.
- **Test floors lowered to the new masks:** the counts in `wardrobe_tests::HIDES_SKIN` / `HIDES_UNDER` (709 and 709; 149 and 104) and `tee_crew_tests::HIDES_BRA` (72). These floors record how much the masks hide. They are not quality limits, and lower means less is hidden, which is the stricter side. No limit or poke-through record was changed.

### Non-blocking items addressed

- **The double-sided wording.** `asset-quality.md` §3, `wiki/Roadmap/Known-Limitations.md`, this file and the comments in `enclosure.rs` said flipped cloth is seen through because "back faces aren't drawn". They now say it would show the garment's inside, which is a hole by the brief's definition whether its back faces are drawn (the crew tee and joggers, `doubleSided`) or culled. The check itself always treated a triangle facing away as not covering, so its results don't change.
- **`Surface::cast_with`'s per-thread set** now has a comment: `keep` must not cast on the same thread.
- **Not done, still open:** a triangle count or hash beside each numbered mask, the hole check at the body's levels of detail, measured distributions for `HIDE_RULE`, and an ADR for `hide_maps` (the lead's call). The motion check still looks every third frame.

### Visible defects in the new sheets (all present before this branch; before and after match)

- **Crew tee, waist front:** a horizontal tear across the lower front with skin and the boxer briefs' white waistband showing through, on the masculine bodies at rest (`wardrobe-rest.jpg`, masculine average: a skin gap about 4 × 5 cm and a band about 7 × 3 cm). Below it, the hem hangs in loose vertical strips about 10 cm long.
- **Joggers:** black tears at the crotch front in the walk (`crops-wardrobe.jpg`, `Walk@0.367` front, about 3 × 4 cm), and black slits with specks of skin at the centre back of the waist at rest. A white fragment, most likely the boxer briefs, shows at the crotch between the legs in `cuff-close`.
- **The joggers' rib cuffs:** the bottom edge is a sawtooth, with teeth about 1 cm deep, and the cuff's outline is faceted (about eight flat sides) in `cuff-close`. A pointed notch at the front of the cuff shows the ankle above the rim.
- **The licensed tee:** unchanged from section "What I saw".

### Tests (review fixes 1)

| Run | Result |
|---|---|
| Red: `cargo nextest run -p axcenity-player -E 'test(undrawn_items)'` with `undraw` stubbed | 1 run, 0 passed, 1 failed |
| Mutation: the new enclosure test with `GRAZE` = 0.05 | 1 run, 0 passed, 1 failed |
| `cargo nextest run -p axcenity-avatar -E 'test(enclosure::)'` | 8 tests run: 8 passed |
| `cargo nextest run -p axcenity-avatar -p axcenity-assets -p axcenity-gameplay -p axcenity-project -p axcenity-player --no-fail-fast` | **1040 tests run: 1040 passed** (46 slow), 25 skipped; no FAIL, ABORT or TIMEOUT |
| `cargo clippy -p axcenity-avatar -p axcenity-assets -p axcenity-gameplay -p axcenity-project -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all` | done; `cargo clippy -p axcenity-editor --tests -- -D warnings` clean |
| `python3 -I scripts/check-wiki.py` | 115 pages, 0 errors, 57 warnings (staleness on other pages) |
| Fine trims (`trim_hide_maps_to_the_runtime`, release build) | wardrobe sampled on the cast 662 s; wardrobe full 6,899 s; verification on the cast 0 places on 18 bodies (433 s); tees sampled 63 s; tees full 612 s; each `test result: ok` |

Commands: `AXCENITY_TRIM=wardrobe AXCENITY_TRIM_POSES=sample AXCENITY_TRIM_ONLY=-cast- cargo test -p axcenity-player --release -- --ignored trim_hide_maps --nocapture`, then the same without `AXCENITY_TRIM_POSES` and `AXCENITY_TRIM_ONLY` (and `AXCENITY_TRIM=tee-crew,test-tee`). Reveals: `AXCENITY_CAPTURE_UNDRAWN=axcenity/realistic-crew-tee,axcenity/realistic-joggers` with the usual capture command.

### Merge notes (review fixes 1)

- Masks changed: `Shirt_CrewTee.hide.png`, `Shirt_CrewTee.hide.realistic-sports-bra.png`, `Pants_Joggers.hide.png`, `Shirt_TeeCrew.hide.realistic-sports-bra.png`, `Shirt_TeeCrew.hide.realistic-boxer-briefs.png`, and `crates/avatar/tests/fixtures/test-tee/TestTee.hide.realistic-sports-bra.png`. No manifest, GLB, catalogue or preset changed.
- `OPENING_VIEWS` has six views (adds `cuff-up` and `cuff-close`). There is a new capture variable, `AXCENITY_CAPTURE_UNDRAWN`, and a new trim variable, `AXCENITY_TRIM_POSES=sample`.
