# Hand-back: RA2 with RA1's UI, R9's remaining work (`r9-expressions`)

Branched from `main` after wave 3 (e1658afb, PR #427). Plan: section 11.5 row R9 ("Expressions never change the saved description; each expression captured on the face benchmarks and reviewed against reference photographs"), 11.7 (RA2's face modules and `anim/faces`, RA1's `character_workspace/`). Brief §10. OPEN-14 from [audit-plan-status.md](../audit-plan-status.md); the mouth's history in [co6/README.md](../../co6/README.md) ("Review round 3", "The mouth, round 3"). Paul's drag rule of 2026-11-18 for numeric controls.

## What changed

### 1. The Expressions page (brief §10), `character_workspace/expressions.rs`, `panel/expressions.rs`

- **Layout, top to bottom:** Pose and motion chips (wave 3's); the **Face preview** switch; the 15 built-in expressions as **tiles**, three a row, each with an icon (Neutral first, lit when nothing is shown); the **face controls** in six folding sections:
  - Brows: raise, lower, inner ends up.
  - Eyes: blink, squint, wide, look down or up, look sideways.
  - Cheeks: raise, puff. Nose: sneer.
  - Mouth: smile, frown, open lips, wide, narrow, pucker, upper lip up, lower lip down, corners back, sideways.
  - Jaw: open, forward, sideways, tongue out.
  - Brief §10's six controls (smile intensity, brow movement, eye squint, mouth opening, jaw movement, blink intensity) are all there. Left and right move together; a sideways or up-and-down row is one slider from -1 to 1 over two opposite controls (`RowControls::Signed`).
- **How it shows:** the preview's own FaceControls ("Studio Expression Preview", as before). The expression plays on its Expression layer; a moved control is held as a gameplay override (`FaceState::overrides`, the Override layer, which goes over everything under it), so a slider shows exactly the value it says. A slider not moved shows what the expression gives its control (`face_value`, `row_value`); its reset and a double click go back to that value and let the override go. Picking an expression starts the controls again from its values; **Neutral** takes the expression and every held control off.
- **Preview only:** all of it is editor state. `Intent::PreviewFaceControl`, `FacePreview`, `FacePreviewMotion` are new; none is in `Intent::edits()`. Leaving the page, or a new or opened character, takes the expression, controls and face preview off (New and Open since Review fixes 1: before, they left the face preview on).
- **Drag rule hook:** the sliders are the panel's shared slider rows (`panel/row.rs`'s `slider`), with the value typed in. They set no drag speed of their own, so when ux-drag's helper lands in that row they follow Paul's rule (full range in about 100 to 120 points, Shift about a quarter, Ctrl about double, typed values) with the body and face sliders. `TODO(ux-drag)` in `panel/expressions.rs`'s `controls` says so. ux-drag's helper was not on `main` when this branched.

### 2. The face preview mode

- On: the camera straight on and close (2 studs, `FACE_DISTANCE`; aimed 0.06 studs over the head bone as posed, the capture tool's `face-front` stands at 2.3); the body **Still**; the face's **blinking and idle motion off** (`BlinkEnabled` false, `IdleMotion` 0); the **Neutral** light; the expression held.
- **Blink** and **Idle motion** check boxes turn each back on (idle motion is the face's and the body's Idle). The camera frames the face again once the body has its new pose.
- **Neutral light** (`stage::Lighting::Neutral`, new): the Studio sun, more and greyer fill (ambient 0.62, no blue), softer shadow edges (`ShadowSoftness` 0.6; every other preset now sets the class default 0.2 explicitly). It is offered on the page, not in the bottom bar: the bar is full at 1440 points (a chip there overlapped the motion menu in the first snapshot), so while it is on no lighting chip is lit in the bar.
- Off (or leaving the page) puts back the camera, the view chip, the light and the motion it found.

### 3. OPEN-14: the open mouth, the smile, the visemes (face scripts, `anim/faces`)

Measured by rays from the front on a 0.25 mm grid over the mouth, each frame (masculine, feminine): the opening is what meets no skin within 30 mm of the lips' front; in it, what is seen first. Script: see "Measurements".

- **`Jaw_Open` 1 no longer a bell with pointed corners** (`face.jaw_u`, `JAW_U_LOWER`, `JAW_U_UPPER`, `JAW_U_REACH`, `JAW_U_UPPER_FROM`). `rig._jaw` (RA3's) is not changed. `Jaw_Open`'s shape takes the lips toward a share of the jaw's turn that falls off by the sixth power of the way to a corner instead of the weights' 2.5th: the lower lip keeps its drop out toward the corners and turns up steeply into them, and the upper lip stays up near its corners (from 0.7 of the way to them).
  - Width at 75% of the opening's height over width at 25%: 0.69 and 0.70 before, **0.86 and 0.86** after (1 is a straight-sided U).
  - The opening's height 2 mm in from each end over its largest (0 is a point): 0.10 and 0.15 before, **0.25 and 0.31** after.
  - `Jaw_Open` 0.5: 0.81 and 0.77 before, 1.04 and 1.01 after; corners 0.10 and 0.18, now 0.24 and 0.33.
- **The lower first premolars were put in the mouth's shade** (`teeth.LOWER_IN_SHADE`, a dark red by whole tooth): with the lower lip lower toward the corners they stood in the U as pale-rimmed blocks. Replaced in Review fixes 1 by a shade that eases per vertex from the canine back, toward dimmed enamel, not red (below).
- **The smile** (`SMILE_PULL` 14.8 mm, was 12.8; `SMILE_CHEEK` 8.6 and 7.8 mm, was 7.2 and 6.2; `SMILE_FOLD` 5.4 mm, was 4.2) and **Happy's recipe** (smile 1, was 0.8; cheek raise 0.75, was 0.6; squint 0.35, was 0.3; corners back 0.5, was 0.4; the 0.15 brow raise taken out: a smile's brows don't rise). `Mouth_Smile`'s largest move is 15.2 mm (13.1).
- **Visemes**
  - `Mouth_Pucker` gathers the lips by 0.40 of their width (0.31) and 7.2 mm forward (5.4): `PUCKER_GATHERS`, `PUCKER_FORWARD`.
  - `LipSeal`: `Mouth_Narrow` draws its flaps back at 0.4 (1.15), so a rounded mouth's small opening shows the lips' inner red where the teeth showed. `Jaw_Open` at 7.2 (8): with the broad U a laugh drew the upper flap out through the top of a thinner upper lip (with `nose_width` -1 and two other identity ends, found by `face_checks.py`); at 7.2 it is still gone by 5.5 mm of parting, under the check's 6 mm.
  - The viseme map (`default.visemes.json`): U narrow 0.9, pucker 0.9, open 0.08 (was 0.6, 0.8, 0.2); WQ narrow 0.95, pucker 0.95, open 0.05 (0.7, 0.9, 0.15; at narrow and pucker 1 two points of the feminine lower incisors were 0.55 mm in the lip, with or without a little opening); TH `Jaw_Open` 0.08, open 0.3, tongue out 0.6 (open 0.25, tongue 0.5: the jaw 0.2 in all); FV the upper lip up 0.45 and no opening (open 0.1, lip 0.35).
  - Teeth seen in U: 108 and 100 mm² before, **69 and 47** after; WQ 104 and 84, **52 and 34**; tongue seen in TH 33 and 12 mm², **135 and 117**.
- **Withdrawn:** bringing the upper lip's inner side forward with the jaw (to light the dark band) changed nothing in the pictures and, in a laugh, drew `LipSeal`'s upper flap out through the lip on three identity ends; taken out.

## Files

- New: `crates/editor/src/character_workspace/expressions.rs`, `expressions_tests.rs`, `panel/expressions.rs`; this hand-back, `wave4/r9-expressions-red-run.txt`, `wave4/r9-expressions/` (sheets).
- Changed, editor: `character_workspace.rs` (fields `face_controls`, `face_view`; `preview_expression` moved to `expressions.rs`; `start_over` clears the page), `intent.rs` (three intents, at the end of the preview group), `shell.rs` (dispatch; leaving the page leaves the face preview first), `panel.rs` (the page moved out), `stage.rs` (`Lighting::Neutral`, `ShadowSoftness`), `crates/editor/CLAUDE.md`.
- Changed, anim: `faces/expressions/Happy.expression.json`, `faces/visemes/default.visemes.json`; `face_play.rs` and `face_tests.rs` (their Happy numbers: smile 1, squint 0.35).
- Changed, the realistic package: `face.py`, `teeth.py`; regenerated `RealisticParts.glb` (40.38 MB; again in Review fixes 1) and `Realistic.build.json` (only `Mouth_Pucker` and `Mouth_Smile_*` largest moves change).
- Docs: `docs/editor/character-studio.md` (the Expressions page), `co6/README.md` (an R9 section; its "What still falls short" mouth items), `assets/characters/realistic/README.md` (the jaw), `wiki/Roadmap/Known-Limitations.md`.

## Interfaces

- `Intent::PreviewFaceControl { name, value: Option<f32> }`, `Intent::FacePreview(bool)`, `Intent::FacePreviewMotion { blink, idle }`. `PreviewExpression` now also clears the held controls. Additions to the lead's `intent.rs`; nothing removed or renamed.
- `CharacterWorkspace::face_controls: BTreeMap<String, f32>`, `face_view: Option<expressions::FaceView>`, `face_value`, `expression_value`, `row_value`. `expressions::{ROWS, Row, RowControls, FaceGroup}`.
- `stage::Lighting::Neutral` (not in `Lighting::ALL`).
- No avatar schema, ADR, architecture or format change. No dependency added.

## Tests (written first; the red run is `wave4/r9-expressions-red-run.txt`)

`character_workspace::expressions_tests`:
- `expression_edits_never_change_the_saved_file`: saves, then picks Happy, moves five controls, turns the face preview on with blink and idle, sets the neutral light, runs, picks Laughing, turns the preview off, Neutral; the description is equal, nothing is dirty or undoable, and the saved file is **byte for byte** the first one. (It passed against the stubs by design: it is the guard.)
- `face_controls_move_the_face_over_the_expression_and_neutral_resets_them`: a control alone moves the morph weights (over 0.3); Happy's sliders start at its values; a move holds over it and letting go shows the expression's value again; Neutral clears both and the face is back at rest within 0.02 of every weight; leaving the page removes the preview's FaceControls.
- `the_face_preview_frames_the_face_holds_it_still_and_leaving_restores_the_view`: closer than the Face view, aimed within 0.2 studs of the head bone, straight on; Still, Neutral, `BlinkEnabled` false, `IdleMotion` 0; over 7 s (a blink comes every 2 to 6 s) no morph weight moves by 1e-4; with Blink on one moves by over 0.3; Idle motion plays Idle and sets 0.5; off restores orbit, view, light (Sunset) and motion exactly; leaving the page does too and frames the whole body.
- `the_face_controls_are_grouped_by_feature`: six groups have rows, every row names real controls, brief §10's controls are on rows.
- `the_expressions_page_draws_tiles_groups_and_the_face_preview`: the 15 names, the six group labels, Face preview, Blink, Idle motion, Neutral are drawn.
- `expressions_pages_are_drawn`: snapshots with `AXCENITY_SNAPSHOTS`.

Runs:
- `cargo nextest run -p axcenity-editor --no-fail-fast`: 1405 run, 1404 passed, 1 failed (`app::collab_repo_tests::closing_the_editor_uploads_what_was_saved`, a timing test under the shared load; passes alone, 32.7 s). No ABORT.
- `cargo nextest run -p axcenity-anim --no-fail-fast`: 182 passed.
- The player's release test binary filtered on `face` (14 passed, the face presets' pixel tests among them) and `lip` (3 passed).
- `cargo clippy -p axcenity-editor -p axcenity-anim --all-targets -- -D warnings`: clean. `cargo fmt --all`. `python3 -I scripts/check-wiki.py`: 0 errors.
- Blender gates on the committed build: see "Gates".

## Gates

On the committed build (`RealisticParts.glb` as committed; the viseme map as committed):

- `python -I assets/characters/humanoid/qa/face_checks.py --package realistic --out ... --check` (bpy 5.3): **exit 0, 0 failures**, identity_pairs included (masculine: 3,388 poses, 6 with new crossings, all on older parameters' ends and reported, as on `main`; feminine: 0). `co6/face_checks.json` and `co6/face-checks-run.txt` are this run's. Two earlier runs on this branch failed and were fixed, not loosened: 33 failures (a laugh drew `LipSeal`'s upper flap out through the lip with three identity ends; the feminine WQ put two lower-incisor points 0.54 mm in the lip), then 29 (WQ alone).
- `measure.py --package realistic --check assets/characters/realistic/qa/thresholds.json --only face,sliders`: **sliders 3,828 pairs on 15,312 bodies a frame, 0 new crossings on both frames**; the face section passes. The command exits 1 only on the two shoulder targets, which `--only` leaves unmeasured ("not measured (run every section)"; they are the known open shoulder targets, untouched here). Run on the build before the WQ recipe's last change, which `measure.py` doesn't read.

## Measurements

The mouth from the front (script in the session's scratch folder, not committed: rays along the face's forward axis on a 0.25 mm grid from 12 mm above the lips' line to 50 mm under it, 72 mm across; the masks are in `r9-expressions/masks-front.png`, colours: skin, teeth white, gums, tongue pink, lining dark red, `LipSeal` purple). Masculine, feminine; before > after:

| Pose | Width 75%/25% | Ends' height | Teeth seen mm² | Tongue mm² |
|---|---|---|---|---|
| `Jaw_Open` 1 | 0.69 > 0.86, 0.70 > 0.86 | 0.10 > 0.25, 0.15 > 0.31 | 179 > 240, 165 > 166 | 477 > 534, 380 > 480 |
| `Jaw_Open` 0.5 | 0.81 > 1.04, 0.77 > 1.01 | 0.10 > 0.24, 0.18 > 0.33 | 157 > 208, 139 > 148 | |
| Surprised | 0.82 > 0.98, 0.80 > 0.96 | 0.11 > 0.22, 0.20 > 0.33 | 229 > 306, 169 > 194 | |
| Laughing | 0.57 > 0.68, 0.56 > 0.67 | 0.10 > 0.15, 0.18 > 0.28 | 446 > 571, 359 > 424 | |
| U | | | 108 > 69, 100 > 47 | 35 > 2, 11 > 0 |
| WQ | | | 104 > 52, 84 > 34 | |
| TH | | | 135 > 183, 134 > 152 | 33 > 135, 12 > 117 |
| FV | | | 123 > 44, 114 > 47 | |

"Teeth seen" counts every tooth, the shaded lower back teeth included: in the open poses it grows with the opening (the U is wider at the bottom, so more of the lower row is in it); the pictures showed the lower premolars dark red there (until Review fixes 1; the shade changes no geometry, so these areas stand). The dark band (lining seen between the upper lip and the first tooth below it, the middle 10 mm): 1.50 mm masculine, 0.82 feminine at `Jaw_Open` 1, before and after.

## Visual evidence and what I saw

Captures: the benchmark cast's average characters of both frames (`realistic-masculine-cast-average`, `realistic-feminine-cast-average`), every built-in expression, `Jaw_Open` 0.5 and 1, the closed smile and six visemes (U, WQ, TH, FV, O, AI, as the lip sync poses them), views `face-front`, `face-three-quarter`, `face-side`, lights `studio` and `hard`, 2560 x 1440 (`AXCENITY_CAPTURE_FACE`). Before is `main` (e1658afb), after this branch. Sheets in [r9-expressions/](r9-expressions/): `expressions-1..4.jpg` (each expression, each frame, before above after: front, three-quarter and side under studio light, front under hard light), `mouth-open.jpg`, `mouth-smile.jpg`, `mouth-visemes.jpg` (the mouth from the front at full size, studio then hard, before then after), `masks-front.png`, `page-*.jpg` (the page: at rest, Happy with the jaw at 0.35, the face preview on Grey and Light; the status line cut off).

The references: [references/face](../../references/face/README.md) has no index of expression photographs (its rows are anthropometry and anatomy plates); L-07 (a drawn open mouth, CC BY 2.0) is the only open-mouth reference in it, and co6's reviews judged against written descriptions. So what follows is judged against L-07's description and the anatomy of the expressions (a smile: the corners drawn up and out toward the cheekbone, the cheeks lifted, the lower lids pushed up, the fold from the nose's wing deepened; a wide-open mouth: a rounded opening whose corners are soft folds of lip, the upper teeth's crowns in view, the lower lip covering most of the lower row; U and WQ: a small ring of lips well forward of the teeth; TH: the tongue's tip between the teeth; FV: the upper incisors on the lower lip). An index of licensed expression photographs is an open item (below).

- **`Jaw_Open` 1** (`mouth-open.jpg`): the out-turned points at both corners are gone on both frames under both lights; the opening's sides fall nearly straight from the corners and the bottom is broad. It reads as an open mouth rather than a bell. The lower sides are straight lines rather than a curve (a broad U drawn with a ruler, not an oval), and a row of dark-red lower premolars with pale rims shows along each lower side, more than before on the masculine frame. The dark band under the upper lip's edge is as before.
- **Surprised, Laughing, Excited**: broader at the bottom; Laughing's corners still show the upper back teeth's edges, as before; Surprised shows more of the lower incisors' tops than before.
- **Happy** (`mouth-smile.jpg`, `expressions-1.jpg`): the corners rise further and the mouth is wider; the cheeks are fuller at the corners and the lower lids a little higher. It reads more as a smile than before, most on the feminine frame, but it is still a closed-lip smile with no dimple or crease. At full size the lips' line shows a slight point at its middle in the after pictures (both frames, clearer under the hard light) that the before pictures don't have. Trying Happy with the upper lip raised (0.25, to bare the upper teeth) showed a dark slit, not teeth (`LipSeal` covers them at that parting), so it was not kept.
- **U, WQ** (`mouth-visemes.jpg`): smaller and rounder openings with the lips further forward; on the feminine frame both are a small ring (13 to 14 mm across). (Corrected in Review fixes 1: at three times the size, the feminine U's and WQ's openings are still mostly pale enamel, with a dark cross at the incisors' gap; teeth still show, as the 47 and 34 mm² say.) The masculine U and WQ still show the upper teeth's edges across a 23 to 25 mm opening.
- **TH**: the tongue now shows between the rows in the middle (pink under the upper incisors, both lights); the opening is taller than before.
- **FV**: the upper teeth show over the lower lip, which sits right under their edges, but the lower lip does not tuck under them.
- **The other expressions** (Neutral, Sad, Angry, Confused, Worried, Frustrated, Disgusted, Fearful, Crying, Determined, Sleepy): no shape they use changed but the jaw's (Crying, Fearful at 0.3); the pictures match before at sheet size (Sad checked at full size: the same). Crying and Fearful (the jaw at 0.3) are a little broader at the bottom; in Crying under the hard light a lower premolar now shows dark red beside a white canine on the feminine frame (the shade rule is whole teeth by rank), which reads as a gap or a dark tooth close up.
- **The page** (`page-*.jpg`): tiles in three columns fit the 330-point panel; the face controls start under the tiles (only Brows is in view at 900 points high; the rest scroll). The face preview frames the face from crown to collarbones, straight on. With the neutral light on, no light chip is lit in the bottom bar (documented).

## Open issues

1. **FV's lower lip doesn't tuck under the upper teeth.** No standard control moves the lower lip up and back (ARKit's `mouthRollLower`); it needs a lip-roll control, which is a change to the standard face controls (`core::face::FACE_CONTROLS`, the profiles, the build's `EXPRESSIONS`): the lead's call.
2. **The dark band under the upper lip** in every open mouth is unchanged (1.5 mm high at the middle on the masculine frame). The one attempt (the lip's inner side forward) was withdrawn. It probably needs the lining's shading (RA5/RA7: specular occlusion and a lighter inner lip) rather than a shape.
3. **The broad U's lower sides are straight**, and the lower premolars show along them. A rounder bottom needs the lower lip's own curve (a corrective along the lip), not more of the jaw's turn. (Their dark red, pale-rimmed shade and the dark red premolar beside a white canine in Crying are fixed in Review fixes 1; the teeth themselves still show as a row of shaded teeth.)
4. **Happy is a closed-lip smile** with a slight point at the middle of the lips' line at full size; no dimple or nasolabial crease (no wrinkle maps). A teeth-showing smile needs `LipSeal`'s flaps to give way to `Upper_Lip_Raise` sooner, which the seal's other cases constrain.
5. **The masculine U and WQ** still show the upper teeth's edges (23 to 25 mm openings; a real one is about half that).
6. **No reference photographs of expressions are indexed** (`references/face` has anthropometry and anatomy plates only). The judgements above are against written anatomy and one drawing (L-07), so R9's "Done when" ("reviewed against reference photographs") is **not met** and stays open until licence-compatible expression photographs are indexed. Asked of the lead or RA6: links only, Class 1, 2 or 5, for example Wikimedia Commons CC0 or CC BY photographs of a Duchenne smile, a wide-open mouth and the U, TH and FV mouth shapes; then the captures here are reviewed against them.
7. The face controls move left and right together (no one-sided preview); the expression tiles have no pictures.
8. The drag rule: the face sliders take ux-drag's helper only once it is in `panel/row.rs`'s `slider` (the hook above).

## What the lead must do at merge

- Rebuild the generated files on the merged sources (`build.py` with the bpy python, then `presets.py`, `catalogue.py`, `make.py`): this branch regenerated and commits `RealisticParts.glb` and `Realistic.build.json` only; `Realistic.base.json`, the item manifests, presets, catalogue and benchmarks came out with line-ending differences only and are not committed. `co6/face_checks.json` and `co6/face-checks-run.txt` are this build's (see Gates).
- `intent.rs` gains three variants (the lead's file); other wave 4 tracks may add intents beside them.
- `panel/expressions.rs`'s sliders go through `panel/row.rs`: if ux-drag replaces `row::slider`, check the face rows still call the shared one.
- `anim`'s two tests now hold Happy's new numbers (smile 1, squint 0.35); a track that changes Happy again must change them too.
- If another track changes `rig._jaw`, `face.jaw_u`'s formula assumes the weights along the lips are `0.30 u^2.5` and `1 - 0.70 u^2.5`.

## Review fixes 1

The review found three blocking items. All three are fixed below; no item was disputed.

### 1. New or Open left the face preview on

`start_over` cleared the expression and the controls but not `face_view`, so with the face preview on, a new or opened character kept the Neutral light and the old character's head framing (wrong for a taller or shorter character), and the stored restore held the old orbit.

- Test first: `expressions_tests::a_new_or_opened_character_leaves_the_face_preview` (Open of the saved file, then New, each with Happy and the face preview on). Red: `the face preview is off` failed (`wave4/r9-expressions-red-run.txt`, "Review fixes 1").
- Fix: `start_over` now calls `leave_expressions()` first (the face preview off, which puts back the light, camera, view and motion it found; then no expression and no held control), before the description changes. The test asserts `face_view` is None, the light is the Sunset it found, the orbit, view and motion are those from before the preview, no expression, and the preview's FaceControls is gone.

### 2. The lower teeth's shade: dark red, pale-rimmed, by whole tooth

The broad U shows more of the lower row, and the lower first premolars had been put in a dark red shade by whole tooth: at full size the masculine `Jaw_Open` 1 had a sawtooth of dark red, pale-rimmed teeth down each lower side; Surprised and Crying under the hard light had a white canine beside a dark red premolar (a fang beside a decayed tooth); the page's face preview showed dark red rounded lower teeth.

Fix (`teeth.py` only; no geometry changes):
- The shade's colour is now **dimmed, then darkened enamel** (`SHADED_ENAMEL`, two stops), not a dark red, at the **far end** of the teeth's palette after the thin edge's pale grey (`OWN` = 0.75: neck, enamel, edge, then the shade). Part of the way over a tooth is a paler or darker grey-ivory and never mixes with the yellow neck. A first try with the shade colour at the start of the ramp, as before, came through olive.
- The lower teeth go over to it **per vertex, eased from the canine back** (`LOWER_SHADE_FROM` 6 to 34 mm behind the incisors' front, smoothstep), in place of `LOWER_IN_SHADE`'s whole teeth by rank. No tooth stands out from its neighbour.
- Lit 0.6 (`SHADE_LIT`, was 0.4). At 0.4, or with a far stop of 0.30, the back teeth read as charcoal, which is a decayed look too.

Tried and not kept (all captured): shade colours 0.40, 0.52 and 0.56 grey at the ramp's start (olive or slate); the far end with one stop (slate grey); a darker far stop (charcoal molars).

Measured on the pixels that changed between the branch before the review and after it (front view, 2560 x 1440; mean sRGB of 255; "red" is R less the mean of G and B):

| Pose | Frame | Light | Pixels | Before: mean, red | After: mean, red |
|---|---|---|---|---|---|
| `Jaw_Open` 1 | masculine | studio | 1,728 | (35, 21, 24), 13.0 | (47, 43, 43), 4.1 |
| `Jaw_Open` 1 | masculine | hard | 1,480 | (13, 6, 9), 5.5 | (20, 19, 21), -0.4 |
| `Jaw_Open` 0.5 | masculine | studio | 2,107 | (36, 20, 23), 13.9 | (48, 44, 44), 4.5 |
| Surprised | masculine | hard | 1,972 | (14, 7, 9), 6.7 | (22, 22, 24), -1.0 |
| Laughing | masculine | studio | 3,914 | (36, 20, 23), 14.2 | (49, 44, 44), 5.1 |
| Crying | feminine | hard | 2,044 | (22, 10, 11), 11.8 | (32, 33, 36), -2.8 |
| Crying | feminine | studio | 2,127 | (37, 20, 22), 16.3 | (50, 50, 53), -1.6 |
| Happy + `Jaw_Open` 0.35 | feminine | studio | 1,119 | (36, 22, 25), 12.8 | (49, 43, 42), 6.9 |

Across all 24 cases (6 poses, 2 frames, 2 lights), "red" fell from 4.8 to 16.3 to between -2.8 and 6.9.

What I saw:
- [r9-expressions/review-teeth.jpg](r9-expressions/review-teeth.jpg): six poses, both frames, studio and hard light, before and after each.
- [review-teeth-close.jpg](r9-expressions/review-teeth-close.jpg): the same at 3x.
- The refreshed `page-r9-*.jpg`.

At 3x, by pose:
- **`Jaw_Open` 1, masculine:** the lower sides are a row of grey-ivory teeth that darkens toward the back, with no red and no pale-rimmed dark blocks. The teeth are still there to be seen: the lower lip doesn't cover them (open issue 3).
- **Crying, feminine, hard light:** the white canine is followed by premolars that dim smoothly to grey. Before, there was a glossy dark red tooth.
- **Surprised, masculine, hard light:** the canine still carries a bright specular spot from the key light. Its neighbours now continue the row, so it no longer reads as a fang beside a dark gap.
- **Laughing:** the lower molars are now a lighter grey-ivory than before and more visible as teeth. I judge that more natural than dark red, but they can still be counted at 3x (as before).
- **The page's face preview (Happy with the jaw at 0.35):** grey, dim lower teeth beside the tongue instead of dark red rounded ones.
- **Three-quarter view (Laughing):** no change at sheet size beyond the colour.

Gates on this build:
- `face_checks.py --package realistic --check`: exit 0, 0 failures. The identity_pairs result is unchanged: masculine 6 poses with new crossings, the same optional pairs as `main`; feminine 0. The output is byte for byte the committed `co6/face_checks.json` (the shade changes no geometry).
- `measure.py --only face,sliders`: sliders 3,828 pairs on 15,312 bodies a frame, **0 new crossings on both frames**; no measured line is worse; the command exits 1 only on the two shoulder targets `--only` leaves unmeasured, as before.

### 3. The hand-back overstated the photograph review and misdescribed the feminine U

- **R9's "Done when", "reviewed against reference photographs": not met.** `references/face` indexes no expression photographs. The review was against written anatomy and one drawing (L-07). The stage stays open until licence-compatible expression photographs are indexed. Asked of the lead or RA6: links only, Class 1, 2 or 5. For example, Wikimedia Commons CC0 or CC BY photographs of a Duchenne smile, a wide-open mouth, and the U, TH and FV mouth shapes. Open issue 6 says so.
- **The feminine U and WQ:** at 3x their openings are still mostly pale enamel, with a dark cross at the incisors' gap. The "What I saw" bullet is corrected. Teeth seen stay at 47 and 34 mm².

### Non-blocking items taken up

- The front-view mouth measurement is now committed as [r9-expressions/mouth_measure.py](r9-expressions/mouth_measure.py) (bpy: `--repo REPO --out FILE.json [--png DIR]`), so the bell and corner numbers can be reproduced.
- Not changed:
  - The drag rule waits on ux-drag. Until then the face rows drag at about 300 points for the full range, with no Shift or Ctrl.
  - Restoring the light the face preview found discards a bottom-bar pick made during it.
  - Surprised's corner wedges.
  - The tongue band in Happy with the jaw at 0.35.

### Tests and checks (Review fixes 1)

- Red, then green: `a_new_or_opened_character_leaves_the_face_preview`.
- `cargo nextest run -p axcenity-editor -E 'test(/character_workspace::/)'`: 112 run, 112 passed.
- `cargo nextest run -p axcenity-editor --no-fail-fast`: 1,406 run, 1,405 passed, 1 failed, 10 skipped, no ABORT. The failure is `app::collab_repo_tests::closing_the_editor_uploads_what_was_saved`, timing under the shared load. Run alone, it passes in 12.3 s.
- `cargo clippy -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- In-engine captures: before and after, 6 poses, both cast-average frames, `face-front` and `face-three-quarter`, studio and hard, 2560 x 1440. Page snapshots: `AXCENITY_SNAPSHOTS ... expressions_pages_are_drawn`.

### Files (Review fixes 1)

- `crates/editor/src/character_workspace.rs` (`start_over`).
- `crates/editor/src/character_workspace/expressions_tests.rs` (the new test).
- `assets/characters/realistic/teeth.py` (shade).
- Regenerated `RealisticParts.glb`. `Realistic.build.json` is unchanged. The base, items, presets, catalogue and benchmarks had line-ending-only differences and are not committed.
- Docs:
  - `assets/characters/realistic/README.md` (the palette)
  - `docs/development/character-studio/co6/README.md` (the R9 Teeth bullet)
  - `docs/editor/character-studio.md` (New and Open leave the page)
  - this hand-back
  - the red run
- `r9-expressions/review-teeth.jpg`, `review-teeth-close.jpg`, `mouth_measure.py` and the four refreshed `page-r9-*.jpg`.

At merge: rebuild as before. Nothing else depends on `LOWER_IN_SHADE`, which is gone.
