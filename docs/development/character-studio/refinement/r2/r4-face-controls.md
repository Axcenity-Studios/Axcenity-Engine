# R4 face controls (RA2): hand-back

Branch `r4-face-controls`, from `refinement-r1-wave1`. Plan 11.5 R4 (the face shape work; the UI is RA1's later) and brief §5. Red run: [r4-face-controls-red-run.txt](r4-face-controls-red-run.txt). Sheets: [refinement/r4/](../r4/).

## What changed

### 1. Fourteen new face parameters (nine controls)

From the face references' high-value list (`references/face/README.md` §2 and the per-feature files). Each has an `about` sentence, its group as `feature`, level `detailed`, range -1 to 1, and a `mirror` where the feature is paired. Shapes are made on the head's own topology with CO6's methods (`face.py`, `Face.identity`), kept off the eyeballs and the teeth where they come near them.

| Parameter (shape) | What it does | Each end, measured on the head | Reference (SD) | Ends in SD |
|---|---|---|---|---|
| `nose_tip_projection` (`Face_Nose_Tip_Projection`) | The tip out along the line from subnasale to the tip | sn-prn +-6.45 mm (both frames) | sn-prn 21.1 +- 2.7 men, 19.8 +- 2.7 women (F-01) | +-2.4 |
| `nose_tip_rotation` (`Face_Nose_Tip_Rotation`) | The column, the tip's underside and the nostrils' rims turned about subnasale (up at +1); the dorsum holds | nasolabial angle +19.1 / -20.0 degrees (masculine), +20.4 / -20.3 (feminine) | 105.1 +- 9.6 men, 101.9 +- 9.3 women (F-10) | +-2.0 to 2.2 |
| `nose_width` (`Face_Nose_Width`) | The whole nose spread from the middle, root to wings (`nostril_width` is the wings alone) | al-al +-9.8 mm; root about +-2 mm a side | al-al 36.6 +- 4.1 men, 33.2 +- 3.9 women; root 16.6 +- 2.3 (F-01) | +-2.4 / 2.5 (al-al); about +-1.8 (root) |
| `philtrum_length` (`Face_Philtrum_Length`) | The mouth, its bag and the teeth down from a nose that goes a fifth of the way (`mouth_height` moves the nose's base most of the way with the mouth) | sn-ls +3.85 / -3.73 mm (masculine), +3.9 / -3.8 (feminine) | 14.2 +- 2.3 men, 12.7 +- 2.0 women (F-10) | +-1.65 men, +-1.9 women: held short (below) |
| `jaw_angle_left`, `_right` (`Face_Jaw_Angle_*`) | The jaw's corner along the bisector of its angle in the side's plane, not wider: +1 square, -1 sloped | corner 6.8 mm; the soft tissue's angle (corner to a point up the ramus and one along the jaw's lower edge) -10.6 / +12.6 degrees (masculine), -12.1 / +14.7 (feminine) | gonial angle 121.6 +- 6.9 men, 125.3 +- 6.6 women (J-01) | about +-1.7 men, +-2.0 women |
| `upper_lid_fold_left`, `_right` (`Face_Lid_Fold_*`) | The fold over the upper lid down and forward over the lid at +1 (hooded), up and back at -1 (a high, deep crease); the rims and the crease's two loops don't move, so a blink closes, and the fold's underside slides along itself (review fixes) | fold 2.54 mm; lid show is 3.3 mm at rest | not collected (eyes README); margin-to-pupil 3.5 +- 0.8 (E-01) is the rim, which can't move | by eye |
| `lower_lid_left`, `_right` (`Face_Lower_Lid_*`) | The lower lid's skin up and forward at +1 (full, short, its junction with the cheek high and filled), down and back at -1 (flat, long, a hollow under it) | 3.47 mm (2.32 before the review) | the margin (5.2 +- 1.0 under the pupil, E-01) is the eye's opening; no identity shape can move it and keep a blink closed | by eye |
| `eye_corner_left`, `_right` (`Face_Eye_Corner_*`) | The outer part of the eye raised (or lowered) with the inner corner held; `eye_tilt` turns the whole eye about its middle (its sentence now says so) | outer corner +-3.0 mm: canthal tilt +-6.9 degrees | 5.1 +- 2.5 men, 6.1 +- 2.9 women (E-03) | +-2.75 men, +-2.4 women |
| `earlobe_left`, `_right` (`Face_Earlobe_*`) | The lobe stretched longer, or drawn up into the ear, from a point inside the ear 24 mm above its lowest point | lowest point about +-8.5 mm (largest move 8.54; 5.57 before the review) | lobe 25.6 +- 4.5 men, 23.7 +- 4.5 women (EAR-02) | about +-1.9 |

**Chin height** is `chin_length` (li-me: the chin's height under the lower lip, 6.2 mm a side, unchanged); its label is now "Chin height", and no second shape was made. The references' table lists it as the existing control; li-me is 39.1 +- 7.0 men and 38.1 +- 3.7 women (F-11), so its ends are about 0.9 and 1.7 SD. **The pupil** (also in plan 11.5's list) has no parameter, as before (`faceparams.py`'s note; it's the eye shading's `pupil_size`); it wasn't in this task.

**Held short of 2.5 SD, and why:**
- *Philtrum*: 6 mm (sn-ls about +-4.6) pushed four vertices of the lips' seal (`LipSeal`) out under the nose in a laugh at -1 (the mouth's bag goes up with the teeth while the nose's base holds); 5 mm with the nose's base going a fifth of the way is clean.
- *Earlobe*: 1.9 SD, not 2.5: at 10.5 mm the short lobe of a smaller ear (`ear_size` -1) crossed itself; at 8.5 mm nothing crosses (review fixes).
- *Jaw angle*: a skin shape can only move the corner's soft tissue; 7.5 mm along the bisector is the most that stays a smooth corner.
- *Lid fold and lower lid*: no population figure exists for either; the ends are by eye, limited by the crease's two loops, which lie on each other, and (the lower lid) by its margin, which a blink needs where it is.

### 2. The two frames' faces more different (`face.FEMININE`, `feminine_face`, `Face.feminine`)

Through the `Frame_Feminine` shape's face part: `face.prepare` moves the feminine face before the teeth are placed and the rig fitted, so the teeth fit the new mouth. The masculine frame and everything below the jaw are unchanged (the masculine pictures before and after are the same).

What the references say against what the bundle had (`references/face/README.md`, "Population data"):
- The bundle's two heads are the **same size** (zy-zy 149.3 and 149.1 mm on the skin), where women's faces are about 6% narrower (135.1 against 143.5) and 7.5% shorter (se-me 113.4 against 122.7) with eyes barely smaller (interpupillary 61.9 against 64.5): the main difference, and the one missing. So the feminine face is **5% smaller about the point between the eyes** (the eyes and lids hold, so they are larger against the face; the cranium, the ears and the neck don't move; the mouth, its bag and the lips move as one, by their middle's move, since a mouth made smaller round teeth of the same size showed the seal in a laugh).
- Beyond size: **lips** a little fuller (the same red heights, 10.3 mm, on a smaller face: 0.9 mm out, borders 0.7 mm apart), **cheeks** fuller (1.0 mm the soft cheek, 1.8 mm over the cheekbone), a rounder, more upright **forehead** (2 mm), **brows** 2 mm higher, a **chin** 8% narrower and a **jaw corner** 3 mm more sloped (gonial angle 125.3 against 121.6 degrees).
- Already feminine in the bundle, left alone: the **brow ridge** (the feminine glabella stands 0.4 mm behind sellion, the masculine 3.7 mm in front), the **jaw's width** against the cheekbones (go-go/zy-zy 0.84 against 0.89) and the **nose's width** (al-al/zy-zy 0.233 against 0.253).

Measured on the head (masculine; feminine before; feminine after): zy-zy 149.3; 149.1; 146.3 mm. se-me 119.4; 117.4; 113.0. Upper lip red (ls-sto) 5.2; 4.9; 5.5. li-me 40.9; 42.6; 40.6. Outer eye corners against zy-zy 0.623; 0.627; 0.638 (references: 0.449 and 0.458 for the pupils, a ratio of 1.02). The test's measure (below): the frames' faces differ by 4.50 mm root mean square over the front of the face, against 3.12 mm before.

### 3. Checks

- `face_checks.py` holds R4's parameters (`HELD_IDENTITY`) at both ends alone, in a full blink (no ray bare) and under each of the 15 expressions and 12 visemes (group `identity_faces`: 27 poses for each of the 28 parameter ends, 756 a frame). The older parameters' ends are still reported only (some aren't clean; `co6/README.md`). `--only TEXT` runs the poses whose names contain it.
- Since the review, `face_checks.py` also holds those ends to no new crossing of the head's skin triangles, alone and on top of each end of every other face parameter (`identity_pairs`, 28 x 121 poses a frame): the pairs that cross with both and neither at rest nor with the other end alone, counting the pairs within 4 mm at rest that `measure.py`'s sliders check leaves out (the lids' crease lies on itself there). Left out: the closed lips' own contact (pairs within 4 mm at rest and within `LIPS_EDGE` of `LipSeal`). Held on top of an end that crosses nothing of its own; on top of an older end that already crosses, reported (`required: false`).
- `render_face.py` has a sheet `face_identity_sidelit_<id>` (a parameter's -1, 0, +1 under one low side light, close up, two views chosen by its group; `--params`).

### 4. Presets (`presets.py`)

The R4 controls where they serve a preset (face README §3: jaw angle + for square and angular, - for round, heart and soft; the rest by the presets' sentences); every value at most 0.7, both sides of a pair alike:

| Parameter | Round | Oval | Square | Heart | Long | Broad | Narrow | Soft | Angular |
|---|---|---|---|---|---|---|---|---|---|
| `jaw_angle` (both) | -0.4 | | 0.6 | -0.3 | | 0.2 | | -0.5 | 0.6 |
| `nose_width` | 0.2 | | | | -0.1 | 0.4 | -0.4 | | |
| `nose_tip_projection` | | | | | 0.3 | | | -0.2 | 0.3 |
| `nose_tip_rotation` | | | | | | | | 0.2 | -0.1 |
| `philtrum_length` | -0.3 | | | | 0.4 | -0.2 | | | |
| `lower_lid` (both) | 0.3 | | | | | | | 0.4 | -0.2 |
| `upper_lid_fold` (both) | | | | | | | | 0.2 | -0.3 |
| `eye_corner` (both) | | | | 0.2 | | | | | 0.3 |

Oval takes none (it is close to the base head). Earlobe serves none.

## Files changed

- `assets/characters/realistic/face.py`: the new shapes (`Face.identity`; `jaw_landmarks`, `jaw_corner`, `subnasale`, `nose_width`, `_rims_off_ball` with a `keeps` share, factored out of the tilt), their constants (`NOSE_*`, `PHILTRUM_*`, `JAW_CORNER_OUT`, `LID_FOLD_*`, `LOWER_LID_*`, `EYE_CORNER_*`, `EARLOBE_*`), the teeth following `Face_Philtrum_Length` (`_teeth_follow` takes the lifts by name), the new mouth shapes in `EVEN_SHAPES` / `EVEN_PAIRS`, and the feminine face (`FEMININE`, `feminine_face`, `Face.feminine`, called by `prepare`). Also: six lines in `Face` (CO6's chin, and mine) had lost their `\` line continuations (one long line each); they have them again, with the same meaning.
- `assets/characters/realistic/faceparams.py`: the nine controls (`PAIRED`, `SINGLE`, `ABOUT`), `eye_tilt`'s sentence, `chin_length` labelled "Chin height", the features' sentences.
- `assets/characters/realistic/presets.py`: the table above.
- `assets/characters/humanoid/qa/face_checks.py`: `HELD_IDENTITY`, `identity_faces`, bare rays held for those parameters' blinks, `--only`; since the review `head_crossings` and `identity_pairs`.
- `assets/characters/humanoid/qa/render_face.py`: `face_identity_sidelit`, `--params`.
- Tests: `crates/avatar/tests/realistic_face_controls.rs` (new), `crates/avatar/tests/realistic_parameters.rs` (counts; positions now in metres, below), `crates/avatar/tests/shipped_library.rs` (89 parameters), `crates/anim/tests/realistic_face.rs` (the teeth follow the mouth's identity shapes).
- Generated, regenerated by me with the README's commands (`build.py` with the bpy Python, `presets.py`, `catalogue.py`, `benchmarks/make.py`, then `face_checks.py --out docs/development/character-studio/co6/face_checks.json --check`): `RealisticParts.glb` (38.5 MB, 37.0 before), `Realistic.base.json`, `Realistic.build.json`, `presets/*.preset.json` (the nine face presets change; the rest are line endings only), `catalogue.json`, the benchmarks (56 new `face-*` files: the new parameters' ends on both frames), `co6/face_checks.json`. `manifests.py` is unchanged.
- Docs: `co6/README.md` (counts, the teeth's shapes, an R4 section), `testing.md`, the wiki's `Characters.md` and `Character-Data.md`, this file, the red run, `refinement/r4/` (23 sheets, 4.5 MB).

## Interfaces

No contract changed: 14 new parameters and shapes in the manifest, all with the R1 fields; `Teeth` and `LipSeal` follow `Face_Philtrum_Length` by name as they follow `Face_Mouth_Height`. Python: `face.feminine_face(topo, F, eyes_f, radius_f)`, `Face.jaw_landmarks(s)`, `Face.subnasale()`, `face_checks.held_identity(pid)`.

## Tests

Red first ([r4-face-controls-red-run.txt](r4-face-controls-red-run.txt)): `cargo nextest run -p axcenity-avatar`: 88 run, 83 passed, 5 failed (no parameter `nose_tip_projection`; no largest move for the new shapes; the frames' faces differ by 3.12 mm; 158 shapes where 172 are wanted; 148 parameter ends where 176 are). `-p axcenity-anim -E 'binary(realistic_face)'`: 4 run, 3 passed, 1 failed (`Teeth doesn't move with Face_Philtrum_Length`).

Green:
- `cargo nextest run -p axcenity-avatar -p axcenity-anim --no-fail-fast`: 268 tests run: 268 passed.
- `cargo clippy -p axcenity-avatar -p axcenity-anim --all-targets -- -D warnings`: clean. `cargo fmt --all`: done.
- `python -I scripts/check-wiki.py`: 113 pages, 0 errors (58 warnings, none new).
- `face_checks.py --package realistic --out docs/development/character-studio/co6/face_checks.json --check`: **0 failures** on both frames (19 minutes). Each frame: 48 controls, 15 expressions, 12 visemes, 122 identity ends with their blinks, and R4's 756 `identity_faces` poses; at rest the eyes are open 9.53 / 9.42 mm (masculine) and 9.4 / 9.3 mm (feminine), and a full blink leaves 0.0 mm and no bare ray.
- `measure.py --package realistic --only files,weights,clipping,face,shapes,sliders,proportions,deformation,twist --check assets/characters/realistic/qa/thresholds.json`: **check passed** (30 minutes). Sliders: 89 parameters, 3,828 pairs, 15,138 bodies a frame; 0 bodies with new crossings on either frame, 0 parameters alone with new crossings, 0 parameters alone putting the eyes or the teeth in flesh. The two targets not met are the shoulder at 90 degrees (local volume 0.847 / 0.88 against 0.853 / 0.992), as before R4: a target, not a limit (`thresholds.json`), and a body measure this branch doesn't touch. The measurements were written outside the repository (the `--only` run is partial, so `co4/measurements.json` is left as it was).

**Second session (the gates).** The first session ended with the work uncommitted and these two gates not yet run. The package was rebuilt from the sources as they stand (`build.py` with the bpy Python, 2 minutes 18 seconds): `Realistic.base.json`, `Realistic.build.json`, `RealisticRegions.png` and the item manifests came out byte for byte the same as the committed ones; `RealisticParts.glb` differs only in the order of the teeth's vertices (the same positions, sorted) and one byte of the skin's normal map's PNG, which the build doesn't hold stable, so the committed file was kept. `presets.py`, `catalogue.py` and `benchmarks/make.py` re-run left every preset, the catalogue and all 433 benchmark files unchanged. Then `face_checks.py` rewrote `co6/face_checks.json` on that package, `measure.py` ran with the sliders section, and the Rust tests, clippy and fmt ran again (268 passed; clean). The sheets were looked at again at full size (below): nothing needed changing.

`realistic_parameters.rs`'s "every parameter moves the body" measured in the model's own units (its box scaled to one on each axis), so a move up counted 1.75 times less than it is and a move forward three times more; it now measures in metres (the model's `size`), as `realistic_fat.rs` does. Every parameter still moves the body and both copies by more than 2 mm. `shipped_library.rs`'s parameter count (75 to 89) is a count, not a loosened check.

New tests (`realistic_face_controls.rs`): every new parameter exists with its group, feature, level, range, a sentence naming its side and its mirror; each new shape's largest move is between 2 and 12 mm; the feminine face differs from the masculine by more than 4 mm root mean square over the front of the face, the head's own move taken out (3.12 before, 4.50 now); since the review, the lower lid moves 3.4 mm or more and the earlobe 7 mm or more (`the_lower_lid_and_the_earlobe_move_enough_to_be_seen`).

## Review fixes

The review found three blocking problems. Red first: [r4-face-controls-red-run.txt](r4-face-controls-red-run.txt) (the new test against the reviewed package: `Face_Lower_Lid_Left` 2.32 mm, `Face_Earlobe_Left` 5.57; and the new crossing count on the reviewed package).

1. **`upper_lid_fold` +1 and the crease.** Counted with every pair, those within 4 mm at rest too: alone, neither end of the reviewed fold made a single new crossing on either frame (so "+1 no more than rest" already held alone). What the hand-back meant was the fold on top of older ends that already cross at the crease: masculine `eye_size_left` -1 (36 pairs alone) and `cheekbone_left` +1 (3) gained 11 and 10 with the fold at +1, feminine `brow_ridge` +1 (3) gained 14. The cause is the fold's underside (loop 6 to loop 7) turning down onto the lid below the crease. Moving the crease's loops with the fold by one shared amount (the review's first idea) made -1 far worse (69 pairs on four older ends); moving loop 7 forward only left 11. The fix: loop 7 slides along the underside itself (`LID_FOLD_SLIDES`, 1.3 mm), so the underside keeps its angle, and loops 8 to 11 carry the hood (a little more than before: 2.54 mm, so the coarsest copy still moves over 2 mm). Now the fold makes no new crossing alone or on top of any of the 121 other face parameter ends, on either frame, at either end, the older broken ones included. `face_checks.py` holds this from now on (`identity_pairs`, above): any new pair fails `--check` unless the other end already crosses on its own.
2. **`lower_lid` visible.** The shape now moves the lid's lower loops about 1.5 times as far (3.6 mm at loop 7) and brings the lid's junction with the cheek forward at +1 and back at -1 (1.3 to 1.5 mm at loops 7 to 9): a tear-trough hollow at -1, filled at +1; the rims still don't move, and a full blink leaves no ray bare. Begun nearer the inner corner it crossed the skin there with a smaller eye (feminine `eye_size_left` -1, up to 36 pairs), so it starts a sixth of the way along the eye (`LOWER_LID_FROM`), averaged three times. In-engine front close-up under the studio light (a 180 x 170 px crop of the left eye, brow to cheek, from 1920 x 1080; pixels changed by more than 15 against the neutral): masculine 3.6 / 3.7% (2.0 / 1.7% before), feminine 8.0 / 8.8% (3.7 / 3.9%); the upper lid fold on the same crop is 3.2 / 4.8% and 2.5 / 2.7%, the eye corner 11.9 / 11.8% and 13.2 / 13.2%. Largest move 3.47 mm (2.32 before).
3. **`earlobe`.** The lobe is now stretched from a point inside the ear 24 mm above its lowest point (front to back by a third of that), not moved as one; where it joins the head it moves by 0.55 of that, easing to its whole move five rings in, and the cheek's skin under it eases with it. Held fully at the join, the -1 lobe kept its point; moved fully, +1 had the wedge. Its lowest point now moves about 8.5 mm each way (5.57 before), about 1.9 SD of lobe length: at 10.5 mm, the short lobe of a smaller ear (`ear_size_left` -1, -1) crossed itself (2 and 7 pairs). Sheets: `blender-earlobe-left.jpg`, `engine-earlobe.jpg`.

Non-blocking items:
- **Counts**: 61 face parameters and 67 shapes (45 + the brows' length a side + 14), in `co6/README.md` and the wiki's `Characters.md`.
- **Sliders evidence**: the sliders section and the check's lines are committed as `co6/r4-sliders.json` (89 parameters, 3,828 pairs, 15,138 bodies a frame; 0 bodies with new crossings; 0 parameters alone with new crossings or putting the eyes or the teeth in flesh).
- **Presets against the default**: the preset sheets now have the default as their first column and each preset's changed share against it (above).
- **Three-quarter row**: the engine eye sheets' second row now shows the near eye (the right one; both sides are set), not the far one behind the nose.
- **Preset sheet title**: whole.
- **Naming**: `Face_Nose_Width` is kept: the stylized body's whole-nose shape has the same name (`humanoid/face.py`, `humanoid/manifests.py`), and the realistic wings, a shape a side, are `Face_Nose_Width_Left` / `_Right`.
- **`nose_tip_rotation` -1's pale line**, **ends short of 2.5 SD** (philtrum, jaw angle, nose tip rotation; their reasons above), **RA6's sign-off on the qa scripts** (the changes only add checks), **the metres in `realistic_parameters.rs`**, **the feminine three-quarter view**: unchanged; for the lead.
- The pairs check found one more case outside the lids, now left out on purpose: `philtrum_length` -1 with `mouth_height` +1 moves the closed lips' corners 9.5 mm up together, and a few hundredths of a millimetre between the two lips' triangles there make 4 pairs cross that touch at rest. That is the closed lips' contact `measure.py` leaves out too; `identity_pairs` leaves out exactly those pairs (within 4 mm at rest and within `LIPS_EDGE` of `LipSeal`) and counts the lids'.

Gates after the fixes (the package rebuilt with bakes; `presets.py`, `catalogue.py` and `benchmarks/make.py` left every file unchanged):
- `face_checks.py --package realistic --out docs/development/character-studio/co6/face_checks.json --check`: **0 failures** (22 minutes). `identity_pairs`: 3,388 poses a frame; feminine none with a new crossing; masculine 6, all `eye_corner` on top of older ends that already cross (open issues), reported. A full blink leaves 0.0 mm and no bare ray on both frames.
- `measure.py --package realistic --only files,weights,clipping,face,shapes,sliders,proportions,deformation,twist --check assets/characters/realistic/qa/thresholds.json`: **check passed (2 targets not met)**, the shoulder at 90 degrees as before (22 minutes). Sliders: 0 bodies with new crossings on either frame.
- `cargo nextest run -p axcenity-avatar -p axcenity-anim --no-fail-fast`: 269 passed. Clippy clean, fmt clean, `check-wiki.py` 0 errors.

## Visual evidence and what I saw

In-engine (studio light, 1920 x 1080 captures, cropped): `engine-<control>.jpg`, -1, neutral and +1 on both frames, two views (paired controls with both sides set, so the front view shows them). Blender, one low side light, close up: `blender-<parameter>.jpg`. Frames: `frames-before-after.jpg`. Presets: `presets-masculine-before-after.jpg`, `presets-feminine-before-after.jpg` (the cast's average body with the preset, front and side).

- **Nose tip projection, rotation, width**: read as different noses in the front and the profile, and as natural ones. The first rotation (the whole tip turned about subnasale with the dorsum easing off above it) bent the nose into a hook at +1 with a crease across it, and the drooped tip was a pale lump; the shipped one turns the column and the tip's underside and lets the tip and dorsum follow, which reads as an upturned or drooping tip. At -1 a thin pale line shows under the column in the engine; the neutral face has the same rim, a little weaker.
- **Philtrum length**: the upper lip longer or shorter with the nose where it was; natural at both ends in front and profile.
- **Jaw angle**: plain in profile and at three-quarter (a defined corner at +1, a sloped one at -1); small from the front, where the corner is behind the cheek's outline.
- **Upper lid fold**: in the side-lit close-ups the fold comes down over the outer lid at +1 and a higher, deeper crease shows at -1; under the flat studio light the change is small (3.2 / 4.8% of the eye's crop masculine, 2.5 / 2.7% feminine). The first version carried the fold on past the outer corner as a sharp line; it now eases off there. The review's change (the fold's underside slides along itself) looks the same as before in both sheets.
- **Lower lid** (after the review): under side light a tear-trough hollow under the lid at -1 and a fuller, filled lid at +1, on both frames, natural; under the studio light the shadow under the eye grows at -1 and shrinks at +1, plainly on the feminine frame and modestly on the masculine (3.6 / 3.7% of the eye's crop changed, from 2.0 / 1.7% before; feminine 8.0 / 8.8%, from 3.7 / 3.9%). Before the review it was nearly a no-op in the engine.
- **Eye corner**: clear at both ends (an upswept outer corner, a dropping one) and distinct from the tilt.
- **Earlobe** (after the review): +1 is a longer, rounded lobe on both frames, with the notch where its front joins the cheek; -1 is a short lobe that joins the cheek in a smooth curve, an attached lobe. The point at -1 and the straight cut edge at +1 are gone. The small dark tag at the lobe's front join in the engine is the bundle's own (it shows at 0 as well).
- **The frames**: the feminine face is now a smaller face with the same eyes, a smaller nose, a shorter chin, fuller lips and cheeks and higher brows; side by side with the masculine one it reads as a different, more feminine person, and natural, not caricatured. It doesn't read as a woman on its own yet: the brow cards are heavy and straight (grooming, R6), and the head is bald.
- **Presets**: each changes visibly in the places its sentence names; on the feminine frame the change from before is mostly the frame's own new face. The presets are still mild (C1's finding): their strengths are the references' first guesses, and the new controls add to them only at 0.2 to 0.6. Since the review the preset sheets have the default as their first column and print each preset's share of changed pixels against it: after R4, masculine 1.7% (oval) to 6.5% (broad) in front and 0.8% to 2.7% in profile; feminine about 1.3% to 6.0% and 0.7% to 2.4%. Oval takes no R4 control and is close to the base head by design; the rest each differ from the default in both views.

A second look at every sheet, at full size, in the second session agreed with the above, with three notes. `nose_tip_rotation` +1 shows the nostrils plainly from the front; an upturned tip at about 2 SD does, so it reads as a real nose, but it is the most striking end of the nine. On the masculine frame the presets' change from before is small in both views (the new controls at 0.2 to 0.6 on a face that didn't otherwise change). And the masculine profile's brow ridge stands only a little forward of the eyes; the frames differ there as the references say (above), but a heavier masculine ridge would part them more, and is a choice for the lead, not a defect.

## Open issues

- `eye_corner` +1 on top of four older ends that already cross at the upper lid's crease on their own (masculine `brow_height_left` -1, 10 pairs alone; `brow_arch_left` -1, 7; `brow_ridge` +1, 16; `cheekbone_left` +1, 3) makes 4 more pairs there, and 1 to 2 in two other such cases (`face_checks.json`, `identity_pairs`, `required: false`). Moving the crease's loops as one, the fold's loop with them, a gentler ramp along the eye and keeping the eyeball's clamp off loop 7 all left it the same. The older ends' own crossings are CO6's (reported, `co6/README.md`); fixing them is that work. On the feminine frame and on every clean end, `eye_corner` adds nothing.
- The masculine lower lid is still the quieter of the two frames under the studio light (above): its baked normal map carries most of the shading there.
- At `earlobe` -1 the bundle's small tag at the lobe's front join stays (it shows at 0 too).
- The feminine frame's eyeballs are as the bundle fitted them; no change to the frame's eye scale was made (a bone driver, and the lids are fitted to them).
- R4's "nine presets as data" are R1's files, now using the new controls; "asymmetry tools" are RA1's.
- The anchors (`anchors.py`) are measured on the masculine frame, as before; the feminine face's 5% move is not in them.

## What the lead must do at merge

- Rebuild the generated files on the merged sources (`build.py`, `presets.py`, `catalogue.py`, `benchmarks/make.py`, `face_checks.py --out docs/development/character-studio/co6/face_checks.json --check`): mine are committed but conflict with RA3's.
- `face_checks.py` now takes about 22 minutes (R4's 756 extra poses a frame, and since the review 3,388 `identity_pairs` poses a frame, which measure crossings only).
- A body change from RA3 that moves the feminine head's skin would move the input of `feminine_face`; it is head-only (`topo.is_head`), so body shapes don't change it.
- No new dependency.
