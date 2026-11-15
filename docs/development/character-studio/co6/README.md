# CO6: the realistic head and face

Milestone CO6 of the [Character Studio overhaul](../../../../plans/axcenity-engine-character-studio.md): what the realistic base body's face is, how it is made, how to rebuild and check it, what the engine shows and what still falls short. The package is [assets/characters/realistic/](../../../../assets/characters/realistic/README.md). **The engine's pictures are the judge** ([engine/](engine/)); the Blender sheets beside this file are for forms and for views the capture has no camera for.

## What the face has

| | |
|---|---|
| Head | The body's own head (no graft), vertex order the bundle's. Repairs at rest (`face.prepare`): the feminine frame's lids are the masculine frame's carried to the feminine eye (the feminine opening sat 1.5 mm higher on the eyeball and bared the whole iris: the stare); the lids' lining is lifted off the eyeball; the lips are closed (they rested 1.9 to 2.5 mm apart) without crossing each other. The upper lids' fine detail is taken out of the skin maps (`face.soften_lids`): a closing lid stretches that skin fourfold and pores stretched into streaks. |
| Expressions | 44 blend shapes on `Body`, named as the facial profile's controls (`face.EXPRESSIONS`); the four `Eye_Look_*` controls turn the eye bones. `Jaw_Open`, `Jaw_Left`, `Jaw_Right` are the jaw bone's turn plus a shape that puts right what the turn gets wrong. The three `Tongue_*` shapes are empty on the body and move the tongue (a part follows the body's shape of the same name). |
| Identity | 45 shapes, one per face parameter (`faceparams.py`), in nine groups (eyes, brows, nose, cheeks, jaw, chin, lips, forehead, ears). Paired features have a parameter a side that names the other as its `mirror`; `nose_tilt` and `jaw_shift` push the middle off centre. Eye parameters also move or scale the eye bones. Each end moves the face 4 mm or more (2.2 mm or more on the coarsest copy). |
| `Eyes` | Per eye: `Sclera` (the white, textured), `Iris` (a dish set back inside the limbus, with the pupil), `Cornea` (a clear cap 1.2 mm proud of the white, alpha-blended). 2,256 triangles. Its own shape `Pupil_Size` widens or narrows the pupil; nothing drives it yet (below). |
| `EyeShell` | `EyeOcclusion` (a film between the lids, dark at their edges) and `TearLine` (the wet line), on the head, following the lids by shapes and drawn back behind them as they close. 320 triangles. |
| `Lashes`, `Brows` | Cards, alpha-blended. Lashes are rooted on the lid's outer edge and turn with it about the eyeball (160 triangles); a brow card lies on the skin and rides it whole (112 triangles). |
| `Teeth` | The teeth meet 1.6 mm under the lips' line just behind the upper lip (CO4 had them 13 mm lower and 12 mm further back). The bundle's lower shell and floor, which tore between head and jaw, are gone; a tongue (294 triangles) is made with `Tongue_Out`, `Tongue_Up`, `Tongue_Down`. 3,893 triangles. |

Materials and shading in the manifest: `Skin` skin; `Sclera`, `Iris`, `Cornea` eye; `Lashes`, `Brows` hair; `Mouth`, `Teeth`, `Gums`, `Tongue`, `EyeOcclusion`, `TearLine` standard. `RealisticParts.glb` is 29.2 MB with CO5's body shapes; `Body` and each simpler copy carry 142 shapes.

## How the shapes are made

Procedural, on the head's own loops, with the eyeball and the teeth as things the skin must stay off (decided after the spike, below).

- **Lids** (`lids.py`): every lid vertex turns about a line through the eyeball's centre, by its loop; both rims end on one closing line, so closing is exact; the skin beyond the rim takes the turn smoothly and stays above its own lining. A shape moves a vertex in a straight line and a lid's path is an arc, so the closed lid is raised where it travels furthest by enough that half way the straight path clears the white of the eye.
- **Mouth, brows, cheeks, nose** (`face.py`): offsets along the muscle's pull with a falloff, the skin then put back on the resting surface, the lips left free. A pair's two sides are shares of one shape made for both.
- **Off the teeth, as a surface**: lip and lining vertices are moved off the teeth in the jaw pose the control gives, on both frames; what single vertices needed is then spread (`Face.spread`: the largest push within three edges, then averaged), so a lip clears the highest tooth near it with a smooth edge instead of taking each tooth's print.
- **Following parts**: `EyeShell` is a mix of lid vertices whose height shrinks as the lids meet; lashes turn with their roots; a brow card's three vertices across take the skin's move under its middle one; the teeth take `Frame_Feminine`, `Jaw_Forward` and `Face_Mouth_Height`.

### The spike

Five hard shapes each way, masculine frame, clay. MakeHuman's CC0 expression units, carried by the CO5 spike's registration (nearest point, no face landmarks): [spike/s_mh_mouth.png](spike/s_mh_mouth.png), [spike/s_mh_eyes.png](spike/s_mh_eyes.png). The smile came over as a good broad form; everything with an edge failed (the eye closure tore the lids and left them open, 7.6 mm of the 11 needed; `mouth-open` tore the lips; the brow moved 5 mm on 67 vertices), and the set has no left and right mouth and no tongue. The procedural blink closed exactly on both frames at the first try. So: procedural.

Licence record for the spike: eight files read from `makehumancommunity/makehuman` at commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482`, `makehuman/data/targets/expression/units/caucasian/`, each stating on lines 3 and 4 "This asset was explicitly released as CC0 in september 2020". No code, rig or other file was read; nothing of MakeHuman's is in the package. SHA-256:

```text
cb7e7ea90f88c5f3b5509e91b4767bbee5828215590134cc66d79520bf28ea9c  mouth-corner-puller.target
6b539d8614cb4288175f69505a652d9cc3a23a4863c0d8a8f3f7c5225fab0c73  eye-left-closure.target
368dfd5e1bd0b283c3bd311cff06c6865a7289de91a30d92908396ba12150a84  eye-right-closure.target
8f00a3aa1975d9fd4819f42fb240fadf6393fdb197c22cd1a42c6b8d18e7ddf7  eyebrows-left-up.target
e3a4676637ce6f5445f550e2aac03127ade8b311cd42bd0d93126f4cd6bee3b9  eyebrows-right-up.target
8cb683e6cca007903dd3cc1d2916a3930460c8e06fc80e0ab02f73f8e97529b5  mouth-open.target
b5ac85674bd32211acde62f1fd1d494c6a08be5a17602d195f65cfb1ce3ded07  mouth-pursing.target
ee11909d563e4490c312991ddde1201874feed93506b3cc4de5d110ed914162c  mouth-elevation.target
```

## The ARKit mapping

ARKit's 52 face shapes reach this head through the standard controls: the engine's built-in `arkit` profile (`crates/anim/faces/arkit.face.json`) says which control each plays, and this head plays every one of those controls (test `every_control_the_arkit_profile_uses_plays_on_the_realistic_head`). Shapes are the body's unless said. No runtime ARKit input is part of this.

| ARKit shape | Control (share) | Played on this head by |
|---|---|---|
| `eyeBlinkLeft` | `Blink_Left`; `Eye_Closed_Left` | shape `Blink_Left`; shape `Eye_Closed_Left` |
| `eyeLookDownLeft` | `Eye_Look_Down` | the eye bones |
| `eyeLookInLeft` | `Eye_Look_Right` | the eye bones |
| `eyeLookOutLeft` | `Eye_Look_Left` | the eye bones |
| `eyeLookUpLeft` | `Eye_Look_Up` | the eye bones |
| `eyeSquintLeft` | `Eye_Squint_Left` | shape `Eye_Squint_Left` |
| `eyeWideLeft` | `Eye_Wide_Left` | shape `Eye_Wide_Left` |
| `eyeBlinkRight` | `Blink_Right`; `Eye_Closed_Right` | shape `Blink_Right`; shape `Eye_Closed_Right` |
| `eyeLookDownRight` | `Eye_Look_Down` | the eye bones |
| `eyeLookInRight` | `Eye_Look_Left` | the eye bones |
| `eyeLookOutRight` | `Eye_Look_Right` | the eye bones |
| `eyeLookUpRight` | `Eye_Look_Up` | the eye bones |
| `eyeSquintRight` | `Eye_Squint_Right` | shape `Eye_Squint_Right` |
| `eyeWideRight` | `Eye_Wide_Right` | shape `Eye_Wide_Right` |
| `jawForward` | `Jaw_Forward` | shape `Jaw_Forward` |
| `jawLeft` | `Jaw_Left` | the `Jaw` bone and the shape `Jaw_Left` |
| `jawRight` | `Jaw_Right` | the `Jaw` bone and the shape `Jaw_Right` |
| `jawOpen` | `Jaw_Open` | the `Jaw` bone and the shape `Jaw_Open` |
| `mouthFunnel` | `Mouth_Narrow` | shape `Mouth_Narrow` |
| `mouthPucker` | `Mouth_Pucker` | shape `Mouth_Pucker` |
| `mouthLeft` | `Mouth_Left` | shape `Mouth_Left` |
| `mouthRight` | `Mouth_Right` | shape `Mouth_Right` |
| `mouthSmileLeft` | `Mouth_Smile_Left` | shape `Mouth_Smile_Left` |
| `mouthSmileRight` | `Mouth_Smile_Right` | shape `Mouth_Smile_Right` |
| `mouthFrownLeft` | `Mouth_Frown_Left` | shape `Mouth_Frown_Left` |
| `mouthFrownRight` | `Mouth_Frown_Right` | shape `Mouth_Frown_Right` |
| `mouthDimpleLeft` | `Lip_Corner_Pull_Left` | shape `Lip_Corner_Pull_Left` |
| `mouthDimpleRight` | `Lip_Corner_Pull_Right` | shape `Lip_Corner_Pull_Right` |
| `mouthStretchLeft` | `Mouth_Wide` | shape `Mouth_Wide` |
| `mouthStretchRight` | `Mouth_Wide` | shape `Mouth_Wide` |
| `mouthLowerDownLeft` | `Mouth_Open` (0.5); `Lower_Lip_Lower`; `Lower_Lip_Lower_Left` | shape `Mouth_Open`; shape `Lower_Lip_Lower`; shape `Lower_Lip_Lower_Left` |
| `mouthLowerDownRight` | `Mouth_Open` (0.5); `Lower_Lip_Lower`; `Lower_Lip_Lower_Right` | shape `Mouth_Open`; shape `Lower_Lip_Lower`; shape `Lower_Lip_Lower_Right` |
| `mouthUpperUpLeft` | `Mouth_Open` (0.3); `Upper_Lip_Raise`; `Upper_Lip_Raise_Left` | shape `Mouth_Open`; shape `Upper_Lip_Raise`; shape `Upper_Lip_Raise_Left` |
| `mouthUpperUpRight` | `Mouth_Open` (0.3); `Upper_Lip_Raise`; `Upper_Lip_Raise_Right` | shape `Mouth_Open`; shape `Upper_Lip_Raise`; shape `Upper_Lip_Raise_Right` |
| `browDownLeft` | `Brow_Lower_Left`; `Brow_Inward_Left` (0.3) | shape `Brow_Lower_Left`; shape `Brow_Inward_Left` |
| `browDownRight` | `Brow_Lower_Right`; `Brow_Inward_Right` (0.3) | shape `Brow_Lower_Right`; shape `Brow_Inward_Right` |
| `browInnerUp` | `Brow_Raise_Left` (0.5); `Brow_Raise_Right` (0.5); `Brow_Inward_Left` (0.5); `Brow_Inward_Right` (0.5) | shape `Brow_Raise_Left`; shape `Brow_Raise_Right`; shape `Brow_Inward_Left`; shape `Brow_Inward_Right` |
| `browOuterUpLeft` | `Brow_Raise_Left` | shape `Brow_Raise_Left` |
| `browOuterUpRight` | `Brow_Raise_Right` | shape `Brow_Raise_Right` |
| `cheekPuff` | `Cheek_Puff` | shape `Cheek_Puff` |
| `cheekSquintLeft` | `Cheek_Raise_Left` | shape `Cheek_Raise_Left` |
| `cheekSquintRight` | `Cheek_Raise_Right` | shape `Cheek_Raise_Right` |
| `noseSneerLeft` | `Nose_Sneer_Left` | shape `Nose_Sneer_Left` |
| `noseSneerRight` | `Nose_Sneer_Right` | shape `Nose_Sneer_Right` |
| `tongueOut` | `Tongue_Out` | `Tongue_Out` on `Teeth` (the tongue) |

**Not played** (the profile gives them no control, so this head, like any head driven through the controls, has nothing for them): `mouthClose`, `mouthRollLower`, `mouthRollUpper`, `mouthShrugLower`, `mouthShrugUpper`, `mouthPressLeft`, `mouthPressRight`. `Tongue_Up` and `Tongue_Down` are controls ARKit has no shape for.

## Rebuilding and checking

`PY` is a Python with the `bpy` module, from the repository's root:

```text
PY -I assets/characters/realistic/build.py --out assets/characters/realistic
python3 -I assets/characters/catalogue.py
python3 assets/characters/benchmarks/make.py
PY -I assets/characters/humanoid/qa/face_checks.py --package realistic --out docs/development/character-studio/co6/face_checks.json --check
PY -I assets/characters/humanoid/qa/render_face.py --package realistic --out docs/development/character-studio/co6
cargo nextest run -p axcenity-anim -p axcenity-avatar --no-fail-fast
```

A face pose in the engine (both frames; see [visual-qa.md](../visual-qa.md)):

```text
AXCENITY_CAPTURE_DESCRIPTIONS=assets/characters/benchmarks AXCENITY_CAPTURE_ONLY=realistic-masculine-face-neutral AXCENITY_CAPTURE_FACE="Blink_Left:1,Blink_Right:1" AXCENITY_CAPTURE_SIZE=1920x1080 AXCENITY_CAPTURE_DIR=<folder> cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
```

Tests: `crates/anim/tests/realistic_face.rs` (four: `check_face` finds nothing; every standard control moves a vertex or a bone; the parts carry the shapes they follow; the ARKit profile's controls all play). Red runs before the work: [red-run-anim.txt](red-run-anim.txt), [red-run-face-checks.txt](red-run-face-checks.txt). Now: [face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json).

## Measured (`face_checks.py`, both frames)

| Measure | Masculine | Feminine |
|---|---|---|
| Eye open at rest, left and right (mm) | 9.53, 9.42 | 9.4, 9.3 |
| Full `Blink` and `Eye_Closed`: bare rays | 0, 0 | 0, 0 |
| Blink at 0.25, 0.5, 0.75: lid vertices in the opaque eyeball | 0, 0, 0 | 0, 0, 0 |
| Blink at 0.25, 0.5, 0.75: eyeball vertices in the lid | 2, 0, 0 | 1, 0, 0 |
| Blink at 0.25, 0.5, 0.75: lid vertices in the clear cornea (reported) | 6 (to 0.51 mm), 8 (to 0.54 mm), 0 (to 0.0 mm) | 6 (to 0.5 mm), 8 (to 0.51 mm), 0 (to 0.0 mm) |
| Head triangle pairs that cross at rest | 3 | 0 |
| 48 controls: `teeth_in_flesh` | Mouth_Left 1 (to 0.65 mm) | Jaw_Left 2 (to 0.9 mm); Jaw_Right 4 (to 1.84 mm) |
| 48 controls: `lips_in_teeth` | 0 | Jaw_Right 1 (to 0.24 mm) |
| 48 controls: every other measure held to 0 | 0 | 0 |
| 12 visemes: every other measure held to 0 | 0 | 0 |
| 15 expressions: `teeth_in_flesh` | Disgusted 8 (to 1.2 mm); Laughing 6 (to 1.29 mm) | 0 |
| 15 expressions: `lips_in_teeth` | Disgusted 1 (to 0.92 mm) | 0 |
| 15 expressions: every other measure held to 0 | 0 | 0 |
| Identity ends (90 poses, reported): `eyeball_in_lid` | eye_tilt_left+1 1 (to 0.38 mm); eye_tilt_right+1 1 (to 0.38 mm) | eye_tilt_left+1 1 (to 0.37 mm); eye_tilt_right+1 1 (to 0.37 mm) |
| Identity ends (90 poses, reported): `teeth_in_flesh` | chin_length-1 3 (to 0.93 mm) | chin_length-1 1 (to 0.53 mm) |
| Identity ends (90 poses, reported): `lips_in_teeth` | jaw_shift-1 1 (to 0.68 mm) | chin_length-1 1 (to 2.75 mm) |
| Identity ends (90 poses, reported): `lining_through_skin` | chin_forward-1 9; jaw_shift+1 12; jaw_shift-1 14 | chin_forward-1 8; jaw_shift-1 8 |
| Identity ends: a full blink left open (mm, left and right) | eye_depth_right-1 [0.0, 0.11] | none |

`--check` exits 1 with **7 failures** (the lines above that aren't 0 among controls, visemes and expressions). CO4's file had 42 crossing pairs at the feminine mouth at rest and 3 on the masculine head.

## What the engine shows

Sheets in [engine/](engine/): a row per pose; in a row the masculine frame's studio face view, hard-light face view and three-quarter view (the head cut from the full-body picture), then the feminine frame's. 1920 x 1080 captures, tiles at a third of their size.

- **[engine/face-blink.jpg](engine/face-blink.jpg)** (rest; blink 0.25, 0.5, 0.75, 1; eye closed; blink left). *Good:* at rest both frames have closed lips, brows with body and one dark lash line; the feminine lid covers the top of the iris. The lid comes down evenly and at 1 the eye is closed with one lash line, in studio and hard light, on both frames; no grey band, no strokes above the lid. *Found and fixed on the way:* cut-out (MASK) lash cards showed the backdrop through the skin behind their clear parts in the engine (the grey band under the eye, and the band along a closed lid); lashes are blended now. The streaks down a closed lid were its skin's pores stretched fourfold; that detail is out of the maps. *Still wrong:* in a 4x close-up of a closed eye a red speck of the lid's lining shows at the inner corner, and the closed lid's outer half catches the light as a paler patch; the lash line is a fringe, not hairs; no highlight on the open eye; the whites are flat blue-grey.
- **[engine/face-eyes.jpg](engine/face-eyes.jpg)** (squint, wide, the four looks, brow raise, brow raise left, brow lower, brow inward, cheek raise, nose sneer, nose sneer right). *Good:* the brow controls move skin and card together by a centimetre or so and the cards stay whole; lower reads as a scowl, inward as worry; the eyes turn (since the lead's engine fix) and the lids stay off them; sneer lifts the nose's wing and the lip. *Still wrong:* cheek raise is the weakest: a slightly fuller cheek and a higher lower lid, easy to miss; the top six rows (squint to look right) were read only at tile size.
- **[engine/face-mouth.jpg](engine/face-mouth.jpg)** (24 rows: smile to jaw open + smile). *Good:* smile lifts both corners and the cheeks and shows a line of teeth; left and right are mirror images (the built model's two halves agree to 0.7 mm, its resting asymmetry; from the engine's front view the smile is level); the lip edges are smooth curves in mouth open, upper lip raise, lower lip lower and narrow (close-ups); jaw open shows both rows of teeth and the tongue, with a clean neck under hard light. *Still wrong:* the teeth are a 3,600-triangle decimation and look faceted and grey in close-up, and jumbled where both rows show (Crying); cheek puff is slight; pucker and narrow differ little at this size; the face camera looks from the character's right, so a level smile still looks higher on the far side.
- **[engine/face-visemes.jpg](engine/face-visemes.jpg)**: twelve distinct mouths on both frames. *Still wrong:* FV has no lower lip under the upper teeth; TH shows little tongue; read at tile size only.
- **[engine/face-expressions.jpg](engine/face-expressions.jpg)**: all 15, both frames. Happy reads as a smile at a glance (corners up, cheeks, teeth, lower lids). Angry (brows down in a V, frown), Confused (one brow up, one down, mouth aside), Determined (brows level and low, mouth set) and Worried (inner brows up, eyes a little wide) are four different faces. *Still wrong:* Determined and Frustrated are close to a mild Angry; Crying's teeth are a jumble.
- **[engine/face-identity.jpg](engine/face-identity.jpg)**: every parameter at -1, 0, +1 (masculine, 1280 x 720 captures, from the benchmark characters `make.py` writes). *Seen at this size:* brow height, cheek fullness, chin length, ear size and protrusion (left ear), eye height and spacing, forehead height, jaw shift and width, mouth height and width, lip fullness, nose length and tip. *Hard to see:* cheekbone, brow ridge, eye depth, nose bridge, chin width, temple width; and every right-side parameter, which this camera has on the far side of the face. *Wrong:* `brow_arch` bends the brow card into a hook at -1 and a peak at +1. The feminine frame's ends were measured (`face_checks.json`), not captured.

## Blender sheets

`render_face.py`'s sheets (`face_*.png` here) are the same poses in the QA studio, whose lights wash the skin pale: use them for forms. Two views the engine capture has no camera for:

- **face_under_chin.png**: jaw closed, open, open with a smile, from below on both sides, both frames. Nothing comes through the skin under the chin. The "red patch on the neck" in the first hand-back's sheets was the jaw's shadow banded by a 256-colour PNG: the engine's hard-light jaw-open pictures ([engine/face-mouth.jpg](engine/face-mouth.jpg), rows `jaw open`) show a clean neck.
- **face_lods.png**: blink, jaw open and smile on `Body`, `Body_LOD1`, `Body_LOD2`. The lids close and the lips don't tear on either copy; on `Body_LOD2` the closed eye is a flat band and the open mouth a plain oval.

## What still falls short

1. `face_checks.py --check` is red: 7 failures, all teeth and the lips' inner side within 2 mm of each other (the table above). Single controls: the masculine `Mouth_Left` (1 point) and the feminine `Jaw_Left` and `Jaw_Right` (2 and 4 points), as before the review; the TH viseme and Angry are clear; Laughing is new (6 points to 1.3 mm) with the stronger smile. None shows in the engine sheets from outside; no picture from inside the mouth was made.
2. A half blink dips the lid into the clear cornea by up to 0.54 mm, and 1 to 2 eyeball vertices sit in the lid at a quarter blink (straight-line shapes).
3. Eyes: no highlight (the cornea's gloss is lost in alpha blending), flat whites, no caruncle, a red speck of lining at the closed eye's inner corner. Eye shading is CO7's.
4. Teeth: faceted and grey in close-up (`parts.TEETH_TRIANGLES` 3,600).
5. Lashes and brows are single blended cards: a fringe and a patch, not hairs; `brow_arch` bends its card.
6. Identity: six parameters are hard to see in a face-view picture though they pass the 2 mm test; `jaw_shift` at -1 puts 8 to 14 pairs of the mouth's lining through the skin and `chin_length` at -1 leaves 1 to 3 points of the teeth in the lining; `eye_depth_right` at -1 leaves a blink 0.11 mm open on the masculine frame.
7. The pupil has no slider: `Pupil_Size` moves 1.3 mm of the eye, under what a parameter must move the body by.
8. `Blink` and `Eye_Closed` are separate shapes that add (the lead is looking at it in `anim`).
9. In the engine a cut-out (MASK) card on a skinned, morphed part shows the backdrop through the skin behind its clear pixels; the face's cards are blended to avoid it. Worth a look in the renderer before hair (CO8).
10. The simpler copies were looked at in Blender only (`face_lods.png`); the alpha parts in a full-body engine view were seen only as the three-quarter column of the sheets, where brows and lash lines draw and nothing stands out.
11. `measure.py`'s `sliders` section (every pair of parameters) was not run with the face's 45 parameters added: it is some eight times the pairs it was.
