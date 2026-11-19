# Wave 5, stage 3: hiding the body under clothing (hand-back)

Branch `body-hiding`, from `refinement-wave5`. Task: hide the body (and the underwear) under clothing wherever no hole can show, the usual game approach, so skin and underwear can't come through a garment by construction; prove no hole shows; count poke-through only on skin that is drawn; prove it on the licensed tee, the wave-4 crew tee, the joggers and the test tee.

**In one paragraph.** The rule, the masks, the drawing and the proof are built and hold: no hidden place shows in any gate pose, walk frame or motion-check frame on any gate body, and the before and after captures are identical, pixel for pixel within 24 of 255, in all 528 pictures compared (the licensed tee and the wardrobe, eight cast bodies, five poses, the standard views and four new views into the openings). **But it doesn't reach the defects.** Skin and underwear come through the tees where the cloth flips at the armpit and at the turned hems, and that skin is seen: through the flipped cloth's back faces (not drawn) or up a hem or a sleeve. Hiding it would show the background there, so the rule keeps it drawn. What the masks hide is skin nobody sees and nothing came through: the licensed tee hides 25 of the body's 22,184 triangles (a patch at the sternum and one between the shoulder blades), 74 of the sports bra's, 14 of the briefs' and 19 of the boxer briefs'; the joggers hide 1,014 (the legs inside them). **No poke-through record changed** on any of the four garments; the stylized wardrobe's records are unchanged. Two findings for the lead: the view-based hole check sees the test tee's hidden waist region from below (51 bodies at rest), so imports no longer hide whole regions; and a mask made in the fit's poses is not enough: the runtime's walk shows a few hidden places the fit's walk didn't (the belly seen up the front hem), so a shipped item's masks are trimmed to the runtime's poses by an offline step.

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
| Licensed tee, body | 191 | 25 (163 after a trim by the gate's views, 117 by the fine views at 78 degrees, 25 by the fine views at 87 degrees) | 972 s, 1,846 cases an outfit, 2 outfits |
| its sports bra / briefs / boxer briefs | 164 / 21 / 171 | 74 / 14 / 19 | |
| Wave-4 crew tee, body | 151 | 128 (the gate's views) | 1,668 s, 4 outfits |
| its sports bra / boxer briefs | 118 / 7 | 108 / 7 | |
| Joggers, body | 1,046 | 1,014 (the gate's views) | 592 s, 2 outfits |
| their briefs / boxer briefs | 24 / 144 | 24 / 142 | |
| Test tee (the import), body / sports bra | 83 / 211 | 11 / 60 (fine views at 87 degrees) | in the gate's library build (the gate now takes the kept masks and skips this) |

The crew tee and joggers worn together hide 1,142 body triangles on the masculine and feminine average bodies, 149 of the boxer briefs' and 132 of the bra's and briefs'.

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
- `tee-what-is-hidden.jpg`: the tee not drawn (its model's object renamed in a copy of the library) and the masks applied: what is hidden, seen through.
- `wardrobe-<pose>.jpg`, `wardrobe-open-<pose>.jpg`: the same for the crew tee and the joggers (with the sneakers).
- `crops-*.jpg`: full-size crops at the armpit, neck, waist hem, sleeve end and the seat.

**What I saw.**

- **Before and after are the same picture** in the standard views: 120 of 120 pictures of the licensed tee have no pixel differing by more than 24 of 255 (the difference column is black); the opening views 126 of 128. That is what hiding promises (nothing seen changes), and it is also what it delivers: none of the defects below changed.
- **A sliver of hidden boxer briefs showed** in the first trim's masks: `Walk@0.367`, masculine heavyset, `hem-up-back`, an area 8 × 19 pixels (about 4 × 9 mm) inside the back hem where the boxers' waistband rim was hidden and the arm and a black line showed through instead (22 pixels differ; 1 on masculine average). Most likely a place seen at a graze past the 78 degrees the gate's views count (`GRAZE`): the trim with places counted to 87 degrees (`Views::fine`) drew far more again (117 to 25 body triangles), and the recaptured pictures match the unmasked ones in every pixel. The wardrobe was trimmed with the gate's views only (the fine views take hours a body with four garments' openings); its 280 pictures match too, but a graze in another view is possible there.
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
- The trim: `AXCENITY_TRIM=tee-crew|wardrobe|test-tee cargo test -p axcenity-player -- --ignored trim_hide_maps --nocapture` (`AXCENITY_TRIM_VIEWS=gate` for the wardrobe: with four garments' openings the fine views take hours a body; `AXCENITY_TRIM_ONLY=<body>` for some bodies). About 80 minutes for the licensed tee, 2 hours for the wardrobe with the gate's views, 30 minutes for the test tee. I ran: the licensed tee three times (the gate's views, the fine views, the fine views at 87 degrees), the test tee twice, the wardrobe once with the gate's views and once for the feminine heavy body.

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
