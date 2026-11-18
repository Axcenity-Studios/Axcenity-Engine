# R2 direct manipulation (`r2-handles`): hand-back

Stage R2 of the refinement (plan 11.5, brief §3.4, §4.3, §5.3; milestone CO21): picking on the posed, skinned, live-morphed character, hover highlight, click to select and frame a feature, handles that write the sliders' own intents, Symmetry, camera and editing kept apart, and a switch to turn it off. This branch does RA1's interface part and the lead's two parts delegated to it: the shared ray test in `scene` and the handle data written by `manifests.py`.

Red run: [r2-handles-red-run.txt](r2-handles-red-run.txt). Pictures: [r2-handles/](r2-handles/).

## What was built

- **The region map reaches the model** (`assets`). The build stores each body corner's region in its second texture coordinates (`TEXCOORD_1`, `u = (index + 0.5) / 16`, `v = 0.5`; the realistic README, "Body regions"). Nothing read it before. `assets` now reads it into a new `Primitive::regions: Vec<u8>`, one per corner. A second set that isn't such a map (a lightmap's coordinates) leaves the list empty. Levels of detail (picked and appended), the bake and the cooked model file all keep it.
- **A ray test against the skin as drawn** (`scene/src/surface_pick.rs`):
  - `ray_model` tests one model, posed by a frame's `Posed`, which carries the live identity. Without a pose it tests the rest mesh. The pose is morphed and skinned on the CPU the first time a ray asks for it, and `Posed::primitive` keeps that result with the pose, so one frame's rays share one skinning.
  - `ray_parts` tests a character's MeshParts.
  - A `MeshHit` gives the distance, point, normal, primitive, triangle, barycentric weights and region. `at_rest` maps the hit back to the rest mesh, so picking doesn't depend on the pose or the identity.
  - Helpers for handles: `rest_anchor` (bone and offset to fitted rest space), `nearest_corner` (prefers the skin-shaded primitives), `corner_drawn`, `corners_drawn`, `placement` and `fitted_per_metre`.
- **Handle data** (`assets/characters/realistic/handles.py`, called by `manifests.py` in a new `handles` step). There are 19 handles for the seven features, in the plan's order:

  | Feature | Handles |
  |---|---|
  | `nose` | tip, bridge, both nostrils |
  | `eyes` | each upper lid over the pupil; each outer corner |
  | `jaw` | both corners |
  | `lips` (the mouth) | both corners; upper and lower lip |
  | `shoulders` | each shoulder's top |
  | `waist` | each side, at the trunk's narrowest slice |
  | `hips` | each side, at the widest slice |

  - Every anchor is a corner of the skin, chosen by a rule on the built model. The corners are skin-shaded only: the inside of the mouth and the eye sockets share positions with the skin at their seams and face inward.
  - Every drive's `axis` and `travel` are how that corner moves from the parameter's lowest value to its highest, on the masculine frame. Shapes and corners are added at their weights. The proportion sliders' bone changes are worked out as `anim::BodyShape` makes them: lengths along a bone's Y, widths along its X, children moved out. The face's eye-bone drivers are included.
  - `screen` lists the framings where a drive works and its direction on the screen in each. A framing is listed only where at least 35% of the travel shows on the screen and no stronger drive of the same handle has the same line (within 25 degrees). `views` are the framings where any drive works.
  - Each left handle and its right twin name each other in `mirror`. The right twin is the mirrored skin corner.
- **Validation** (`avatar`): the existing checks already covered the handle's parameter, its feature and its mirror naming it back. A handle can no longer be its own mirror, and both sides of a pair must be on the same feature.
- **Studio** (`editor/src/character_workspace/direct.rs`, wired in `shell.rs`):
  - **Hover.** Moving over the character tints the feature under the pointer and outlines it. Which feature comes from a ray against the skin as drawn, then the manifest's features:
    - a feature that names the hit's body region is picked by that region;
    - a smaller feature whose anchors are within 1.2 times its radius wins over the region (the nose beats the head);
    - on the trunk, where no region feature exists, the nearest anchor wins, and a feature more than twice another's size gives way to it;
    - a hit on another part (an eyeball, the underwear) counts for the skin behind it within 3 cm.
  - **Tint and outline.** The tint is an egui mesh drawn over the preview, never a material change. It covers the regions a feature names, else the skin its parameters' shapes move, else a falloff round its anchors. It is drawn on skin-shaded primitives only, on triangles that face the camera. The outline keeps a feature's long boundary loops and drops holes such as the eye openings and the mouth.
  - **Cost.** The ray is cast again only when the pointer or the camera moves.
  - **Status line.** It names the hovered feature; on a hovered handle it says what each direction does; during a drag it shows the values and "Esc to cancel".
  - **Click.** A click sends `PickFeature`, and the shell now moves to the feature's category (clicking the nose from the Body page opens Face › Nose). Quick goes to Detailed, because features start there. It also sends `FocusFeature`, which frames the feature over 0.25 s when its extent is under 6% of the view's height or its centre is off the middle 80%. A click on the backdrop drops the selection.
  - **Handles.** The selected feature's handles are drawn on their skin corners as the frame's pose puts them. A handle on the far side is hidden (its normal points away by more than 0.5). Hovering one grows it and draws its live directions. While one is dragged, the others hide except its mirror when Symmetry moves it. A handle at its parameter's end turns amber.
  - **Drag.** A primary drag that starts within 11 points of a handle edits; any other drag turns the camera. Alt with a drag always turns it, and the right button pans.
    - The drag sends `SetParameter`, or with Symmetry (or at Quick) `SetMirrored`: `Drag` while it moves, then one `Commit` on release. So a drag is one undo step, and Esc (`CancelEdit`) takes it back.
    - Gain is the drive's projected travel, so the skin under the handle follows the pointer, or 240 points for the whole range when that's longer. Two drives are solved together. A second drive waits until its own part of the drag passes 8 points. Shift is a quarter of the gain.
  - **Switch.** `Intent::SetHandles` is a toolbar icon toggle beside Undo, "Edit on character". Off means no hover, no handles, and the viewport is the camera only.
  - **Panel title.** The panel's title now uses the feature's label ("Body › Waist").

## Interfaces (for the lead)

- `assets::Primitive` gains `pub regions: Vec<u8>`. Every struct literal of `Primitive` in `assets` needs it; there are no literals outside the crate. The cooked model format changed, and its build hash invalidates old caches by itself.
- `scene` exports `Corner`, `MeshHit` (`corners`, `at_rest`), `PartHit`, `ray_model`, `ray_parts`, `rest_anchor`, `nearest_corner`, `corner_drawn`, `corners_drawn`, `fitted_per_metre` and `placement`.
- `Intent` gains `SetHandles(bool)` and `FocusFeature(String)`, both editor state; neither edits the character. `Intent::PickFeature(Some(id))` now also switches to the feature's category when the base model's manifest has that feature in another one.
- `CharacterWorkspace` gains `pub direct: direct::Direct` (`on`, `hovered`, `view_rect`, `status`), `focus_feature` and `pick_feature`.
- `BaseManifest::validate` adds two handle problems: "its own mirror" and "on another feature".
- The handle contract's meaning (architecture §6, "Handle") is now: the anchor is a skin corner; `axis` and `travel` are that corner's move over the whole range; `screen` lists the live framings with their directions; `side` is the right profile; `sensitivity` divides the travel. The R1 text said `travel` came from `face.largest_move_mm`. That was never written, and the text is replaced.

## Tests (red first: [r2-handles-red-run.txt](r2-handles-red-run.txt))

Written first and seen failing (the old manifest without handles, and a stub `direct.rs`):

- `avatar`: `contracts::a_handle_s_mirror_is_another_handle_of_its_feature`, and `tests/realistic_handles.rs`, which has three tests:
  - the seven features have handles, in order;
  - each handle drives its own feature's parameters, and pairs name each other, mirrored;
  - drives have a unit axis, a travel of 2 to 80 mm and one line per view.
- `player`: `handle_tests.rs`:
  - `every_handle_stays_on_the_skin_at_its_parameters_ends` (the R2 "Done when"). As first written it measured a skin corner against its own skin, so it couldn't fail; Review fixes 1 below replaced it, and added `every_handle_s_anchor_is_on_the_skin_at_rest`;
  - `every_drive_moves_the_skin_by_its_measured_travel`: within 5% and cosine ≥ 0.99, masculine frame.
- `editor`: `character_workspace/handle_tests.rs`, driven by pointer events through `CharacterWorkspace::ui`:
  - one test per feature (`the_nose_handle_…`, `the_eye_…`, `the_jaw_…`, `the_mouth_…`, `the_shoulder_…`, `the_waist_…`, `the_hip_handle_gives_what_its_sliders_give`): a drag gives the same description as `SetMirrored` slider intents with the dragged values, it is one undo step, and one Undo restores the start;
  - `a_camera_drag_never_changes_the_description`: a drag on the cheek, on the backdrop, Alt with a drag on a handle, and a right-button pan;
  - `symmetry_moves_the_mirror`: on, the mirror's parameter is equal and its handle moved out by more than 2 points against the nose; off, it stays within 0.75 points across;
  - `hover_highlights_and_a_click_selects_and_frames_the_feature`;
  - `handles_off_leaves_the_camera_only`;
  - `handles_are_drawn_for_hover_selection_and_a_drag` (snapshots).

Written with the code (they couldn't compile before the field or module existed):

- `assets`: `the_realistic_bodys_region_map_reaches_every_corner_and_its_simpler_copies`, `only_a_region_map_in_the_second_coordinates_gives_regions`, `a_cooked_model_keeps_its_region_map`.
- `scene`: `surface_pick::tests`, three tests:
  - a ray at the posed nose tip hits within 1 mm, in the head's region, and a ray up at the sole is in the feet's;
  - the identity's weights (`Face_Nose_Length`, `Face_Nose_Tip_Projection` worn live) move the hit by more than 2 mm;
  - a ray pointing away, or beside the body, returns none.
- `editor`: `direct::tests` (framing names, screen and view point conversion, the outline dropping holes) and `hover_costs_per_frame` (ignored, timing).

Runs:

- The new tests together: `cargo nextest run -p axcenity-avatar -p axcenity-player -p axcenity-editor -p axcenity-scene -E 'test(/realistic_handles|a_handle_s_mirror|handle_tests|surface_pick|direct::/) | binary(realistic_handles)'` → all passed. The editor's 15 were run again after the last change: `15 tests run: 15 passed`.
- The full suites of `assets`, `scene`, `avatar`, `editor`, `player`, `anim` and `gameplay` (`cargo nextest run -p … --no-fail-fast`): `2299 tests run: 2297 passed (20 slow, 1 leaky), 2 failed, 32 skipped`. Both failures were rerun alone and passed:
  - `avatar` `shipped_library::the_realistic_parameters_say_what_they_are` still asserted that the realistic body had no handles ("handles are R2's"). It now asserts it has them.
  - `editor` `app::collab_repo_tests::closing_the_editor_uploads_what_was_saved` timed out at 92 s under the shared load (16 s alone); it isn't this branch's code.
- `cargo clippy -p axcenity-assets -p axcenity-scene -p axcenity-avatar -p axcenity-editor -p axcenity-player --all-targets -- -D warnings` is clean, and so is `cargo fmt --all`.
- `python3 -I scripts/check-wiki.py` gives 0 errors (58 warnings, none of them about pages this branch touched).

## Measurements

**Each drive's travel, measured on the skin as the player applies it, against the data** (`every_drive_moves_the_skin_by_its_measured_travel --nocapture`, masculine frame). Every one agrees to 0.1 mm, with cosine 1.00. The bone-driven ones are shoulders, waist and hips.

| Handle | Drive | Travel | | Handle | Drive | Travel |
|---|---|---|---|---|---|---|
| nose_tip | nose_length | 6.9 mm | | jaw_* | jaw_width | 12.2 mm |
| nose_tip | nose_tip_projection | 13.0 mm | | jaw_* | jaw_angle | 13.6 mm |
| nose_bridge | nose_bridge | 9.2 mm | | mouth_corner_* | mouth_width | 9.6 mm |
| nostril_* | nostril_width | 8.8 mm | | mouth_corner_* | mouth_height | 9.0 mm |
| eye_* | eye_spacing | 8.4 mm | | upper_lip | upper_lip_fullness | 9.7 mm |
| eye_* | eye_height | 9.0 mm | | lower_lip | lower_lip_fullness | 9.5 mm |
| eye_* | eye_depth | 8.4 mm | | shoulder_* | shoulder_width | 37.0 mm |
| eye_corner_* | eye_size | 3.6 mm (9.6 before Review fixes 1) | | waist_* | waist_width | 18.8 / 19.4 mm |
| eye_corner_* | eye_tilt | 6.8 mm (10.9 before) | | hip_* | hip_width | 44.7 mm |

- **Handles on the skin.** This first claimed every handle within 2 mm "to float precision": the test measured a skin corner against its own skin, which is 0 whatever the data says. See Review fixes 1 for what is measured now (the data's prediction, at most 1.67 mm off the applied skin).
- **On the screen.** In the Face view, a face drive's whole range covers about 8 to 14 points of the screen, so the 240-point fallback applies there. In the Front view, a hip's 44.7 mm covers about 14 points.
- **Hover cost.** Measured in a **debug** build under the seven builds' load, with `hover_costs_per_frame`: the studio's frame (no GPU) is 17.6 ms with the pointer off the viewport and 41.5 ms with it over the face, when every frame casts the ray. That is about 24 ms for the ray against the body's 25,000 triangles plus the hover tint. The ray is now cast again only when the pointer or the camera moves; with a still pointer, a frame over the face is 27.0 ms against 23.5 ms away (the tint alone). So a moving pointer still pays the ray, about 20 ms in debug. Release is not measured: run `cargo test -p axcenity-editor --release -- --ignored hover_costs --nocapture` alone. If it's too slow, a bounding-box test per triangle strip, or a coarse grid of the posed triangles, is the next step.

## Visual evidence and what I saw

Studio drawing snapshots (`AXCENITY_SNAPSHOTS=<folder> cargo nextest run -p axcenity-editor -E 'test(/handle_tests::handles_are_drawn/)'`), drawn for real at 1440 × 900 and looked at full size and cropped:

These describe the snapshots as first handed back. Review fixes 1 retook all four (the files in `r2-handles/` are the new ones); `handles-close-ups.jpg` now sets the old crops beside the new.

- `r2-handles/character-studio-handles-hover.png`, the nose hovered. Tint and outline cover the whole nose, bridge to columella, and the status line reads "Nose · click to select it". Faults:
  - The outline steps along the triangles; it isn't smooth.
  - The right nostril's opening is still outlined inside, because it touches the outer loop.
- `…-selected.png`, Face › Eyes selected. There are four handles: each upper lid over the pupil and each outer corner, all on the skin. The eyes' fainter tint covers the lids, the brows' lower edge and the upper cheek. That is the skin the eye shapes move, but it reads broader than "the eyes". The panel shows the eye rows.
  - In one of three runs the pointer, placed 80 points below a corner, was near the jaw's outline and the idle sway took it off the face, so nothing was hovered. In another run the head was hovered and its tint and outline covered the whole head.
- `…-dragging.png`, the left eye mid-drag. The corner handles hide and both lid handles stay (Symmetry), the dragged one larger. No tint shows during the drag. The status line read "Left eye: Left eye spacing 0.25 · Left eye height 0.12 · Esc to cancel", while the panel's sliders read 0.20 and 0.10: the status ran a frame ahead (this first said they matched; fixed in Review fixes 1). At this framing the eye's move is about one pixel, because the 240-point fallback covers the face's 8 mm range. The drag reads in the sliders, not yet in the picture.
- `…-body.png`, Body › Hips in the Front view. The handles sit at the widest point of the hips on the briefs' sides, and the hovered left one is larger with its left-right arrow. The hips' outline traces the region over the briefs. The status line reads "Left hip · drag left and right: Hip width".
- `r2-handles/handles-close-ups.jpg` held the four, cropped and doubled (now the before and after sheet).

Earlier snapshots exposed faults that are now fixed:

- An "Edit on character" chip in the bottom bar overlapped the lighting chips. It moved to the toolbar.
- The tint drew the inside of the mouth over the lips, because the overlay has no depth. Only the skin is tinted now.
- The eye handle sat at the inner corner, then on an inward-facing lid-margin corner. It now uses the "lid" rule: an outward-facing skin corner over the pupil.
- The waist handle sat on the back half of a sparse slice. It now uses the trunk's outline in 3 cm slices, in its own region.

## Dependencies

None added.

## Open issues

- **Gain on the face.** The face's handles use the 240-point fallback at the default Face framing; they follow the skin one to one only when zoomed in far enough that a range shows 240 points. Measured travels are 7 to 14 mm for the face, so the brief's "one cm of drag moves the surface about one cm" holds only close up. Whether 240 is the right feel needs the project owner's hand.
- **Overlay has no depth.** The tint and outline are drawn over the preview without its depth buffer. Skin facing away is left out, but a front-facing fold of the same feature behind another can show through. (The jagged outline is fixed in Review fixes 1.)
- **Straight-line drives.** The handles drive their parameters along straight lines. Nothing in the current set curves much (every travel's cosine is 1.00), but a future tilt-like drive would need `screen` overrides.
- **Weak waist.** The waist handle moves the skin 19 mm over the whole range, because the waist slider is weak (R3). A waist shape would make it worth dragging.
- **Picking near outlines.** Picking near a silhouette can flicker with the idle animation.
- **Hover cost.** About 20 ms in a debug build for each frame the pointer moves (above); release is unmeasured. Run `hover_costs_per_frame` in release and record the figure.
- **Not done (later R2 polish).**
  - The pattern's Ctrl axis lock.
  - Double-click on a handle to reset its parameters.
  - A hollow mirror handle while Symmetry is off.
  - Handles for the other features (brows, cheeks, chin, ears, forehead, the body's other regions).
  - The handles' level filter: all of a feature's handles show at every level, even where a drive's parameter is advanced.
- **Stylized family.** The stylized manifest has no features or handles, so the studio does nothing on hover for it.

## What the lead must do at merge

- **Regenerated:** `assets/characters/realistic/Realistic.base.json`, by `manifests.py` (bpy Python with numpy), which only adds the `handles` section. The `items/*.item.json` and `catalogue.json` came out with line-ending changes only and were not committed. The GLB, presets and benchmarks were not regenerated (they don't depend on this). If the face or body branches change the GLB, run `manifests.py` again after `build.py`. The handle measurements follow the model, and the player's travel test will catch a drift.
- **`Primitive` literal fields.** CO8 (compute posing) may also touch `Primitive` literals in `assets`. Merge by adding `regions` beside the other per-corner lists. `cooked_model.rs` encodes it after `tangents`.
- **`shell.rs`.** The viewport now returns intents, and the top-of-frame drag commit skips while a handle drag is on. The studio-polish branch (RA1) also edits `shell.rs`, `toolbar.rs` and `panel.rs` (the title), so expect small conflicts there.
- **`shipped_library.rs`.** `the_realistic_parameters_say_what_they_are` asserted "handles are R2's" (empty); it now asserts there are handles.
- **Plan and progress.** Tick R2's "Done when" in plan 11.5 and add a progress entry; this branch didn't edit `plans/` or `progress.md`.

## Review fixes 1

The review (fix-needed) found three blocking problems. All three are fixed, test first where the problem is behaviour, and five of its non-blocking notes are fixed too. Red runs are appended to [r2-handles-red-run.txt](r2-handles-red-run.txt). Pictures: [r2-handles/handles-close-ups.jpg](r2-handles/handles-close-ups.jpg) sets the old crops beside the new.

### 1. The "within 2 mm" test couldn't fail

The reviewer was right. `Character::handle` took the skin corner nearest the anchor and measured it against the triangles of the same posed skin, so the distance was always 0. It passed with the nose tip's anchor moved to `LeftFoot`.

**What the tests measure now** (`crates/player/src/handle_tests.rs`):

- `every_handle_s_anchor_is_on_the_skin_at_rest`: each anchor is within 2 mm of a skin corner at rest, measured in the file's metres.
- `every_handle_stays_on_the_skin_at_its_parameters_ends` predicts the handle from the data and does not use the snapped corner at the end:
  - It starts from the anchor where the starting description draws it: its skin corner as posed, plus the anchor's own offset from that corner, so an anchor off the skin stays off.
  - It adds `axis × travel × (value − default) / (range)`.
  - It measures that point against the triangles of the skin the player applied and posed for that end, at both ends of every drive's parameter, on both frames.
- Both mutations the reviewer asked for now fail:
  - **Anchor on `LeftFoot`:** all three handle tests fail, the anchor 24.0 mm off and the prediction 24.1 mm off.
  - **Offset 5 mm off the nose tip:** the rest test (4.99 mm) and the 2 mm test (4.99 mm) fail. The travel test still passes, because the snapped corner hides the offset.

**Measured:** 124 ends were checked. The worst on the masculine frame is 0.47 mm (a hip), and the face is at most 0.06 mm there. The worst on the feminine frame is 1.67 mm (the waist at its widest), because the travels were measured on the masculine frame.

**Where I departed from the reviewer's fix, and why.** The reviewer asked to carry the anchor by its bone's posed joint (the plan 11.3 wording). I measured that first. A bone-carried anchor is not on the skin:

| Case | Bone-carried anchor's distance from the skin at the start |
|---|---|
| Masculine frame, hips | 5.0 mm (the hip corner is shared with the thigh's twist bone) |
| Feminine frame, nose tip | 22.1 mm |
| Feminine frame, shoulders | 35.8 mm |
| Feminine frame, waist | 33.6 mm |
| Feminine frame, every other handle | 0.1 to 13 mm |

The frame's shapes move the skin relative to the bones, so a test built that way fails on data that is right. The studio draws a handle on its skin corner (see "Handle" in architecture §6), and the new test carries the anchor the same way. It still predicts the end from `axis` and `travel` alone. Architecture §6 and the wiki Characters page now say this.

**A real bug the first version of the rest check found.** That first version measured distances in the model's fitted box, whose axes are scaled separately (each axis is divided by the body's size along it). It reported the 5 mm mutation as 21.5 mm. The same distortion was inside the code:

- **`scene::nearest_corner`** chose corners by distance in the fitted box. In a new test, 20 of 55 points 3 mm off the skin found a farther corner than a search in metres finds. It now measures in metres through a new `scene::to_metres`. The travel test still agrees to 0.1 mm.
- **Picking by anchor in the editor** and **the tint's falloff** also measured distances in the fitted box against radii in metres. They now use metres too.

### 2. The eye-corner handles sat on the cheek

`handles.py` has a new `canthus` rule:

1. Take the skin corners within 2 cm of the eyeball's centre, within 4 mm of its height, that face into the eye. These are the lid margin turning into the opening.
2. The most lateral of them is where the lids meet.
3. The handle goes on the outward-facing skin corner nearest it, within 3 mm.

The left anchor moved from `Head + [0.0515, 0.0342, 0.0925]` to `[0.0487, 0.0473, 0.0930]`, from 10.1 mm below the eyes feature's anchor to 3.0 mm above it. That anchor is the eye region's middle, about 6 mm under the eyeball's centre, so the new corner is about 3 mm under the centre.

The drives changed with the place:

- `eye_size`: now 3.6 mm, almost straight out to the side (was 9.6 mm).
- `eye_tilt`: now 6.8 mm, almost straight up (was 10.9 mm).

In the Face view, a sideways drag now sizes the eye and an upward drag tilts it.

**Test:** `realistic_handles::the_eye_corner_handles_sit_at_the_eye_s_outer_corner` was written first and failed on the old data (−10.1 mm). It requires both corners within 4 mm of the eye anchor's height and up to 2 cm out to the side. The reviewer suggested about 3 mm. The new data is at 3.0 mm on the left, so 4 mm leaves room for a rebuilt GLB and still catches the cheek, which was 10 mm off.

**Regenerated:** `Realistic.base.json` (eye-corner handles only), by `manifests.py` with the bpy Python.

**Picture:** the selected snapshot shows each corner handle at the outer corner of the eye, on the skin. The player tests pass with the new data: travel within 0.1 mm, and the prediction 0.05 mm off the skin.

### 3. The overlay was jagged and too broad

**The outline** (`direct.rs`, `tint_mesh`):

- It is now drawn where the tint's weight crosses 0.5 (marching triangles). Each crossing is placed along its triangle's side by the corners' weights, so the line runs across the triangles instead of stepping along them. A crossing is computed from the edge's lower-numbered corner, so the two triangles that share it agree and the loop joins.
- The fill is cut at the same line.
- A body region's corners are split from the next region's along its border, so no triangle crosses there. The outline instead runs along such a seam edge when the corners on the other side are outside the feature, and along any edge where the skin ends (`open_edges`, kept in `Prepared`).
- Loops shorter than a third of the longest are still dropped.

**What counts as the feature's skin:**

- **Facing:** a corner counts only as far as its posed normal faces the camera, fading out below 0.3. Skin turning away drops out, and the outline follows the feature's silhouette.
- **Shape-based features (the face):**
  - Each shape is judged against its own largest move, not their sum: 0 at a sixth of it, 1 at a third, with no floor. This replaces "full at a sixth of the summed most, at least 0.35 above 3%".
  - The result is limited to within the feature's radius of its anchors and handles (full within 70% of it).
  - It is smoothed twice over the mesh, with a seam's two sides kept equal.
- **Hollows** (`hollows_out`): a corner whose normal meets the feature's own skin within 10 mm is inside a hollow and isn't tinted. This takes the inside of the nostrils out.
- **Face features share skin** (`compete`): where another face feature's weight is higher, a feature's weight falls, and the outline runs where the two are equal.
  - **Why:** the eyes' height, depth and spacing shapes move the whole socket as one (share 1.0 from 14 mm below the eye's centre to 8 mm above it, and still 0.33 at the brow 18 mm above). The shapes alone can't tell the eyes from the brows or the cheek.
  - **Effect:** the brows' and cheeks' own shapes move that skin more, so it goes to them.

**Picking:** the feature picked is the smallest one whose tint covers the hit, so what is picked matches what lights up. Where no tint reaches (behind an eyeball), the anchor rule still decides.

**Before and after** (the snapshots at full size, and the crops in `handles-close-ups.jpg`):

| View | Before | After |
|---|---|---|
| Nose hovered | Toothed outline reaching the upper lip and the cheeks beside the alae, a closed loop round a nostril, tint in the nostrils | One smooth outline from the bridge between the eyes to the alae and the base, no inner loop. A sliver of tint remains in each nostril's opening, and the outline has a small notch at the right nostril. |
| Eyes selected | A mask-shaped band from above the brows across the bridge to the upper cheeks, toothed below | Each eye's tint runs from under the brow to the top of the cheek, from the inner corner to just past the outer one. The brows and the bridge are clear. |

What is still left after the fix:

- **Eyes:** the outline bends inward at the inner corner, and its lower edge, where it meets the cheeks, wobbles by a point or two.
- **Hips:** unchanged; the outline follows the region's border along the mesh's edges.

### Non-blocking notes

- **Status line during a drag:** fixed. It now shows the description's values, as the sliders do; the drag's own values ran a frame ahead. In the new dragging snapshot both read 0.20 and 0.10.
- **`direct.rs` module doc:** fixed. It now says the switch is in the toolbar.
- **Esc:** a new test, `esc_cancels_a_handle_drag`. It drags the eye, then presses Esc mid-drag, moves on and releases. The description is as it was before the press and nothing is left to undo. It passed when first run, because it covers behaviour that was already there.
- **Turned head:** a new test, `a_ray_at_a_turned_head_meets_the_posed_skin`. With the head turned 30 degrees, a ray at the turned nose tip hits within 1 mm, and the same ray against the rest mesh does not come within 5 mm. The misleading comment in `the_identity_s_weights_change_where_the_ray_hits` now says the rest mesh is tested without a pose.
- **Fallback direction:** a drive's screen direction now comes from its projected move whenever that is at least 0.01 point (it was 3 points). Below that, the data's direction is used, with its x turned when the camera is on the character's left. The data's framings are from the right.
- **Not changed:**
  - The 240-point fallback, which is the project owner's call.
  - The release hover timing, still not measured.
  - The scope notes (intents, `Primitive::regions`), which are for the lead.
  - The plan tick and the progress entry, which are the lead's.

### Files changed in this round

- **Code:**
  - `crates/player/src/handle_tests.rs`
  - `crates/scene/src/surface_pick.rs` (`to_metres`, `normals_drawn`, a public `triangle_hit`, `nearest_corner` in metres)
  - `crates/scene/src/lib.rs`
  - `crates/scene/src/surface_pick_tests.rs`
  - `crates/editor/src/character_workspace/direct.rs`
  - `crates/editor/src/character_workspace/handle_tests.rs`
  - `crates/avatar/tests/realistic_handles.rs`
- **Data:**
  - `assets/characters/realistic/handles.py`
  - `assets/characters/realistic/Realistic.base.json` (regenerated)
- **Docs:**
  - `docs/development/character-studio/architecture.md` (§6 Handle)
  - `docs/editor/character-studio.md`
  - `wiki/Engine-Systems/Characters.md`
  - `wiki/Roadmap/Known-Limitations.md`
  - this hand-back
  - the red-run file
  - the four snapshots and the close-ups sheet

### Tests run in this round

- **Red runs:** in the red-run file.
- **Handle tests, all green:**
  - player `handle_tests` (`-E 'test(/handle_tests::/)'`): `3 tests run: 3 passed`.
  - scene `surface_pick`: `5 tests run: 5 passed`.
  - avatar `realistic_handles` and the contracts' handle tests: `6 tests run: 6 passed`.
  - editor `handle_tests|direct::`: `16 tests run: 16 passed`.
- **Full suites of `scene`, `avatar`, `editor`, `player` and `assets`** (`--no-fail-fast`): `2081 tests run: 2080 passed (22 slow), 1 failed, 33 skipped`.
  - The failure was this branch's `symmetry_moves_the_mirror` with Symmetry off. The unmoved right handle shifted 1.3 points against the nose (the tolerance is 0.75): under the shared load the idle sway turned the head between the two measurements. It passed three times run alone.
  - The test now measures the handle against the right eye's own anchor, which the head bone carries, so the sway hardly moves one against the other. It passed twice alone.
- **The same suites after that change, without `assets`, which was unchanged since the run above:** `1864 tests run: 1864 passed (27 slow), 33 skipped`.
- **Lint and format:** `cargo clippy -p axcenity-scene -p axcenity-avatar -p axcenity-editor -p axcenity-player -p axcenity-assets --all-targets -- -D warnings` is clean, and so is `cargo fmt --all`.
- **Wiki:** `python3 -I scripts/check-wiki.py` reports `113 pages, 0 errors, 58 warnings`; all are staleness warnings and none is about pages this branch touched.

### For the lead at merge

- **Regenerated:** `Realistic.base.json` only; its eye-corner handles changed. The items came out with line-ending changes only and were not committed. If another branch rebuilds `RealisticParts.glb`, run `manifests.py` after it. Three tests catch drift: the player travel test, the 2 mm test and `the_eye_corner_handles_sit_at_the_eye_s_outer_corner`.
- **New in `scene`:** `to_metres`, `normals_drawn` and `triangle_hit` (now public).
- **Changed in `scene`:** `nearest_corner` measures in metres.
- **Snapshots:** in the four snapshots, the right end of the status bar ("Saved in" and the test's temporary folder, which named the local user folder) is painted over with the bar's own colour. Nothing else in them is touched.
