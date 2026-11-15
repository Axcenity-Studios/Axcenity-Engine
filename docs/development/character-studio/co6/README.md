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
| `Teeth` | The teeth, at 0.85 of the bundle's size, meet 1.6 mm under the lips' line, 2.7 mm behind the upper lip on both frames (CO4 had them 13 mm lower and 12 mm further back). The bundle's lower shell and floor, which tore between head and jaw, are gone; a tongue (432 triangles, on the jaw) is made with `Tongue_Out`, `Tongue_Up`, `Tongue_Down`, and the lower teeth and the tongue carry their own `Jaw_Open` (review round 3, part 2). Shaded by palette images, with two simpler copies of its own (the last section). 9,022 triangles. |

Materials and shading in the manifest: `Skin` skin; `Sclera`, `Iris`, `Cornea` eye; `Lashes`, `Brows` hair; `Mouth`, `Teeth`, `Gums`, `Tongue`, `EyeOcclusion`, `TearLine` standard. `RealisticParts.glb` is 30.5 MB with CO5's body shapes; `Body` and each simpler copy carry 142 shapes.

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

## Review round 3, part 1 (open mouth, smile, laugh)

Engine, 3840 x 2160, studio light, both frames; a row per pose: masculine front, masculine face view, feminine front, feminine face view, feminine three-quarter.

- **[engine/close-open-mouth-and-smile.jpg](engine/close-open-mouth-and-smile.jpg)**: jaw open 0.5 and 1, the smile with the lips closed, Surprised.
- **[engine/close-happy-laughing-squint.jpg](engine/close-happy-laughing-squint.jpg)**: Happy, Laughing 0.5 and 1, squint with cheek raise at 0.5 and 1, blink 1.
- **[engine/close-jaw.jpg](engine/close-jaw.jpg)**, **[engine/close-blink.jpg](engine/close-blink.jpg)**: mouth and eye crops from the first build of this round (before the smile and squint changes).

What changed:

- **The spike in the mouth's floor** was `Jaw_Open`'s own shape: what the lips needed to stay off the teeth with the jaw a tenth open was divided by that tenth, so half a millimetre became a five millimetre spike at full. That step is gone; a laugh (jaw and both smiles together) is cleared instead, with what is added held to 2 mm (`Face.clear_with`).
- **Teeth**: 7,200 triangles instead of 3,600 (the part is 7,493 with the tongue; the file grew 0.13 MB). Each tooth now reads as a tooth.
- **The eyes smile.** `Eye_Squint` and `Cheek_Raise` raise the lower lid by a third and a quarter of the eye's opening (they raised it by a twentieth) and lift the cheek 7 mm, so Happy narrows the eyes and Laughing nearly shuts them from below.
- **The smile** pulls the corners back toward the ears as well as up, with a deeper fold and a fuller cheek, and no longer lifts the upper lip: `Mouth_Smile` alone keeps the lips together, `Mouth_Open` and `Upper_Lip_Raise` part them.
- **The tongue** lies 2.4 mm higher and is fuller.
- **The film and tear line** leave out the eye's corners; a lid's skin stands over its lining after the half-way raise; the lid texels are blurred less and flattened less.
- **Identity**: `brow_arch` is one broad rise (no hook); `jaw_shift` takes the lining and the lower teeth with it; `chin_length` moves the lining less.

**Not solved: the open mouth is still a box.** An attempt to make it an oval (the jaw's hold eased toward half along the lips near the corners, the corners drawn in 7 mm) did narrow and round the opening, but put the mouth's lining through the cheek beside the nose at full opening on both frames (50 and 53 crossing pairs by `face_checks.py`, red patches in the engine picture) and left a pinched fold beside the corner. It was withdrawn; `Jaw_Open` is the corner-in shape of the round before, less the spike. What it would take: the jaw's weights themselves falling off along the lips from chin to corner (`rig.py`), the cheek's lining weighted with the cheek's skin so the two cannot part, and then a corrective shape for the lips' curve.

Also still short after this part: at squint and cheek raise both at 1 the eye is all but shut and its lid line is ragged; at 4K the resting lips show a thin pale line between them (the upper teeth through the gap the seal leaves); the masculine smile's resting asymmetry (0.7 mm between the head's halves) was not looked at again; the reference photographs were not opened for lack of room in the session, so every reading here is against the lead's written description of them, not the photographs.

## Review round 3, part 2 (the open mouth)

This part's pictures were taken from the `face` view, which stands 9 degrees to one side; they are replaced by part 3's straight-on ones below, and the numbers under "What changed" here are part 2's (part 3 changed the corner's share, the lips' curve and the tongue).

What was found in the mesh (masculine frame, before):

- **The box was the weights.** The upper lip's edge is 23 vertices from corner to corner, the lower lip's loops 15 to 21. Every vertex of the upper lip had 0 of the jaw; the lower lip's had 1 to within one vertex of the corner (0.97 at 23.5 mm from the middle, 0.75 at 25.3 mm) and the corner itself 0. So the jaw's whole drop (37 mm at the lips) was taken by the last 2 mm of lip: the box's upright sides. Over the cheeks the hold went from none to all across 0.08 radians about the hinge, 7 mm of skin beside the corner.
- **The lining tore along the cheek.** The mouth's upper lining was all the head's and its lower all the jaw's, with the seam between them (which runs back from the corner along the inside of the cheek) on the jaw: one row of faces from the corner back stretched the full drop.
- **The tongue was not on the jaw.** Its 155 vertices had no weight at all in the file (the vertex groups were written through a handle that no longer added anything), so it stayed up under the palate while the jaw dropped away: the empty floor. `face_checks.py` had been measuring the tongue there.
- **The hinge carries the jaw back.** It is 46 mm above the lips' line and 99 mm behind it, so a turn of 25 degrees takes the lower lip 37 mm down and 29 mm back, and raises the lower incisors' tips 2.7 mm over the lower lip's top.
- **The black line was baked.** The sculpts' lips are closed, and the occlusion bake shaded where they touch: 0.2 to 0.6 along the lower lip's top and 0.19 in the corners (1 is unshaded). The upper lip's inner side was the mouth's dark flesh colour from its very edge.

What changed:

- **Weights** (`rig._jaw`): one smooth field over skin and lining. On the lips' line the upper lip takes `0.42 * (1 - sqrt(1 - u * u))` of the jaw and the lower `1 - 0.58 * (1 - sqrt(1 - u * u))` (`u` from the middle to a corner), carried off the lips in elliptic coordinates about the two corners; over the cheeks the hold runs across 0.38 radians. The lips' inner sides take their outer sides' weights; from 6 to 16 mm behind the lips' line the lining goes over to the head (roof), the jaw (floor) and half (the seam).
- **`Jaw_Open`'s shape** (`face.jaw_open`): everything the jaw holds glides 19 mm forward (a jaw's joint slides forward as it opens; the lower teeth and tongue carry the same in a `Jaw_Open` of their own, which `Teeth` follows by name), the lower lip stands 5 mm higher and 2 mm back, the corners come in 4.5 mm and forward 2.5 mm.
- **The tongue** is on the jaw, round at the tip and as wide as the lower arch (432 triangles). Its `Jaw_Open` lays it in the floor: the tip onto the lower incisors, the upper side flat out to the teeth and 3 mm onto their inner cusps, the back sunk 18 mm along its middle. `Tongue_Out` rises 3 mm more to pass over the lower lip.
- **Skin maps** (`skin.bake`): the occlusion where the lips touch is lifted to 0.85 or more, and the lining within 13 mm of the lips' line shades from the lips' colour into flesh.

Checks: `face_checks.py --check` exits 1 with **1 failure** (masculine `Mouth_Left`, 1 tooth point to 0.65 mm, as before; the feminine `Jaw_Right` is clear), [face-checks-run.txt](face-checks-run.txt) and [face_checks.json](face_checks.json) are this build's, and the table under "Measured" above is still the first hand-back's. `Jaw_Open` puts no lining through the skin on either frame. The Blender sheets beside this file are re-rendered.

What was still short after this part is in part 3, which looked at it from the front.

## Review round 3, part 3 (the mouth from the front)

Engine, 3840 x 2160, studio light, the straight-on `face-front` view; a row per pose: the masculine mouth cut at full size and its three-quarter view (the head cut from the full-body picture, enlarged), then the feminine frame's.

- **[engine/mouth-front-open.jpg](engine/mouth-front-open.jpg)**: rest, `Jaw_Open` 0.5 and 1, Surprised.
- **[engine/mouth-front-smile.jpg](engine/mouth-front-smile.jpg)**: the smile with the lips closed, Happy, Laughing.

What was found:

- **The lips did not seal** (`mouth.seal`). After sealing, the lower lip's top still stood 0.64 to 1.31 mm under the upper lip over the middle 22 mm on the masculine frame (0.95 mm left of the middle against 0.64 mm right) and 1.35 to 1.46 mm from 5 mm right of the middle to 9 mm left on the feminine (0.3 mm at 9 mm right): the slanted line and the half smile. The sculpts are not the cause of the slant on the masculine frame (its lips differ left from right by 0.09 mm on average, 0.27 mm at most; the feminine by 0.43 and 1.27 mm). The upper lip's underside hangs 1.6 mm lower 4.5 mm behind its edge than at the edge, so the lower lip's loops behind its crest lie under it; the pass that keeps the lips from crossing pulled the crest down together with them. Seen from the front, the gap showed the upper lip's inner lining and a sliver of the upper teeth under it.
- **The "lip colour outside the lip"** was the lower lip's own red, not the lining's shading (that is masked to the lining's texels). With part 2's ellipse the lower lip left the corner upright: 2 mm from the corner it already had 0.65 of the jaw against the corner's 0.42, so its end hung 8.6 mm under the corner, and the corner was drawn 4.5 mm in past it.
- **The tongue was made to be seen** in part 2 (raised to the teeth's tops, its tip on the incisors, its sides up along the molars), which from the front is a tongue put out.
- **The upper lip hung over the teeth**: its edge is 5.3 mm above the upper incisors' edges at rest and nothing raised it as the jaw opened, so two to four teeth showed.
- **The teeth are large for the head.** At the body's scale the bundle's upper central incisors are 10.3 mm wide, the lower 6.2 mm, and the arch is 70 mm across the last molars and 55 mm deep; adult teeth are about 8.5 mm, 5 mm and 60 mm (the dental anatomy textbooks' crown widths and the usual arch width, recalled and not looked up for this work), so about 0.85 of this size would be believable. See "not done" below.
- **The feminine U and WQ crossings** are the lips crossing each other where they touch, 10 to 15 mm either side of the middle on the lips' line (52 of 53 and 63 of 63 pairs); `Mouth_Pucker` alone has 167 there with the jaw closed (163 now) and none on the masculine frame. Before part 2 the jaw's 8% opening parted the lips evenly along their length and hid it; now they part less toward the corners.

What changed:

- **The seal** leaves the crest where it has room of its own (over the middle 70% of the lips) and lowers only the loops behind it, then lowers any face that still cuts the upper lip. The lower lip's top is 0.3 mm (the touch room) under the upper lip from side to side on both frames; the feminine mouth has no crossing pair at rest (the source has 42), the masculine its source's 4 at one corner.
- **Weights**: the corners take 0.30 of the jaw (0.42), and along the lips the hold goes by a power of the way to the corner (`0.30 * u ** 2.5` above, `1 - 0.70 * u ** 3` below): a broad U that meets the corner at a slope, so the lip's end stays by the corner, and the opening is widest in its upper third.
- **`Jaw_Open`'s shape**: the upper lip's edge rises 3.8 mm off the upper teeth (less above it, so the lip thins) and the corners come in 2.5 mm (4.5).
- **The tongue** rests 6.5 mm under the plane the teeth meet on at its tip (2.4) and its `Jaw_Open` only lays it level and low: its root sinks 15 mm, it is hollowed 2.5 mm along its middle and thinned from below. `Tongue_Out` rises 4.5 mm more to come from there over the lower lip.
- **The lining's shade** at the lips' edges is a little darker than the lips (it is what shows as the line between closed lips).
- The teeth's fit counts the teeth and gums only, not the tongue made to fit them; the fit is as it was (masculine at the bundle's size 2.7 mm behind the lips, feminine at 0.97 of it 6.2 mm behind).

Not done: **the smaller teeth.** A fit at 0.85 of the bundle's size (with the flesh test made by ray parity, which the smaller jaw needs) put the feminine arch 4.1 mm behind the lips and looked right in an open mouth, but `face_checks.py --check` went from 1 failure to 10 (the lining through the skin in `Mouth_Right` and `Jaw_Forward` on both frames, 61 points of the teeth in the flesh in the masculine Disgusted, lips in the teeth in Happy, Determined and Frustrated). It was withdrawn. It needs the lips kept off the teeth in expressions as wholes, and the skin kept over a lining that the teeth push out.

Checks: `face_checks.py --check` exits 1 with **1 failure**, the masculine `Mouth_Left`'s 1 tooth point to 0.65 mm, as before ([face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json)); `anim` and `avatar`: 222 pass. Reported and not held to 0: on the feminine frame, with the lips now touching from side to side, more head triangles cross in `Cheek_Puff` (22, from 0), `Mouth_Right` (31, from 0), the U and WQ visemes (61 each, from 3 and 4 before part 2), Excited (21, from 4) and Laughing (72, from 52). The Blender sheets beside this file are re-rendered.

**Still short of the photographs:**

1. Blocks of lower teeth still show in both corners of `Jaw_Open` 1 and Laughing on the masculine frame, and Laughing's upper row ends in a longer tooth each side: the teeth are 15% too large (above), and that fit is not done.
2. The feminine teeth still stand 6.2 mm behind the lips.
3. The tongue is low and hollow now but still a smooth slab of one colour, lit like the lips: nothing in the engine shades the inside of a mouth, so at `Jaw_Open` 0.5 and in Surprised the dark above it is a narrow band.
4. The lower lip's red is as tall at the sides of an open mouth as at rest, where a mouth's thins toward the corners; lit from one side it reads as a pink strip beside the opening.
5. A thin pale line still marks the closed lips at 4K (the lining under the upper lip's edge catching the light), and Happy parts the lips by `Mouth_Open` 0.15, so a line of teeth shows in it; the closed-lip smile is `Mouth_Smile` alone.
6. The feminine opening at `Jaw_Open` 1 is taller than wide.
7. The crossings listed under "Checks".
8. The teeth are flat grey (CO7's), and the full-body views draw `Teeth` at a simpler level.

## Teeth and the inside of the mouth

Engine, 3840 x 2160, the straight-on `face-front` view, the mouth cut at full size; a row per pose, the masculine frame then the feminine. Studio light unless the sheet says otherwise.

- **[engine/teeth-open.jpg](engine/teeth-open.jpg)**: `Jaw_Open` 0.5 and 1, Surprised, the AI viseme.
- **[engine/teeth-smile.jpg](engine/teeth-smile.jpg)**: rest, the smile with the lips closed, Happy, Laughing.
- **[engine/teeth-other.jpg](engine/teeth-other.jpg)**: Crying, Disgusted, `Tongue_Out` with the jaw half open.
- **[engine/teeth-hard-light.jpg](engine/teeth-hard-light.jpg)**: `Jaw_Open` 1, Laughing and Happy under the hard light.
- **[engine/teeth-close.jpg](engine/teeth-close.jpg)**: Laughing's upper row and the resting lips at twice the size.
- **[engine/teeth-before.jpg](engine/teeth-before.jpg)**: `Jaw_Open` 1, Laughing and rest as this work found them.

### Adult teeth, measured

The bundle's jaw against measured adults. "Ours" is at the body's scale (the bundle's tooth cages measured along the arch, times 0.85); the studies are means of dental casts, male and female.

| | Bundle | Adults | Ours now | Source |
|---|---|---|---|---|
| Upper central incisor, width | 10.5 mm | 8.4 to 9.0 | 8.9 | [1] [2] [3] |
| Upper canine, width | 8.8 | 7.7 to 8.2 | 7.5 | [1] [2] [3] |
| Upper first molar, width | 12.7 | 10.0 to 10.6 | 10.8 | [1] [2] [3] |
| Lower central incisor, width | 6.3 | 5.4 to 5.7 | 5.4 | [1] [2] [3] |
| Lower first molar, width | 12.9 | 10.8 to 11.4 | 11.0 | [1] [2] [3] |
| Upper arch across the second molars | 71.3 (outer faces) | 56 to 58 (between their buccal cusps) | 60.6 (outer faces) | [4] [5] |
| Upper arch, incisors to the back of the second molars | 55.7 (their distal faces) | 43.5 to 44.4 (their distal cusps) | 47.3 (their distal faces) | [4] |
| Upper central incisor shown under the lip at rest | | 1.9 (men), 3.4 (women) | 4.9 under the closed lips' line | [6] |
| Last tooth seen in a posed smile | | second premolar in 42 to 45%, first molar in 31 to 35% | | [7] [8] |

1. Iranian orthodontic patients, 331, aged 12 to 35: <https://pmc.ncbi.nlm.nih.gov/articles/PMC8853791>
2. Turkish casts, 200, aged 10 to 26: <https://pmc.ncbi.nlm.nih.gov/articles/PMC12126891>
3. Korean adults, 50, aged 24 to 32: <https://pmc.ncbi.nlm.nih.gov/articles/PMC5582099>
4. Saudi adults with normal occlusion, 169, aged 18 to 33: <https://pmc.ncbi.nlm.nih.gov/articles/PMC5832096>
5. Jordanian normal occlusion, 520, aged 15 to 17: <https://pmc.ncbi.nlm.nih.gov/articles/PMC7941956>
6. Vig and Brundo 1978, as quoted by Zachrisson 1998: <https://www.jco-online.com/archive/1998/07/432-esthetic-factors-involved-in-anterior-tooth-display-and-the-smile-vertical-dimension/>
7. Nold and others 2014, 106 adults (a copy of the paper): <https://go.digitalsmiledesign.com/hubfs/DSD%20Articles/Articles%20by%20others/Analysis%20of%20Select%20Facial%20and%20Dental%20Esthetic%20Parameters.pdf>
8. Awad and others 2020, 157 Saudi adults: <https://pmc.ncbi.nlm.nih.gov/articles/PMC7718042>

The bundle's teeth are 15 to 25% over those means, tooth by tooth, and 0.85 of its size puts the incisors and first molars inside the studies' ranges and the canines just under. **Not sourced:** the textbook table (Wheeler's) could not be reached, only two of its figures quoted elsewhere (upper central 8.5 mm, canines 7.5 and 7.0 mm), which agree; no study was found that measures an arch across the teeth's outer faces, so the arch's width is compared across cusps, which lie some 2.5 mm inside each outer face (that allowance is an estimate, not a source). The teeth's height under the lips was left where it was (the upper incisors' edge 4.9 mm under the closed lips' line, 1.5 to 3 mm lower than [6]): raising it changes how much every open mouth shows, which the last round tuned.

### What was found

- **Why the feminine jaw stood 6.2 mm behind the lips.** The fit turned a place down when the mouth's bag, widened round the jaw, came nearer the skin than 2 mm. It measured that at every vertex the widening *wanted* to move, including the ones that could not move at all, because they already rest nearer the skin than that or in front of the lower lip's inner side: on the feminine frame one such vertex reads about -5 mm at rest, whatever the jaw's place. Counting only vertices that moved, the limit holds by construction.
- **The lower "gum" was a whole lower jaw.** The bundle's lower gum is a wall 19 mm tall down the front of the jaw, running on inside the teeth as the floor of the mouth, open underneath. Brought close behind the lips it came out through the skin under the lower lip (seen in the engine as a dark patch on the feminine chin with the mouth closed, and a band under the lip with it open), which no check saw: they count the teeth's *points* in the flesh, and a thinned sheet crosses the skin between its points. Its open underside is also why smaller teeth failed the last round's checks: a point of the lining under an open wall counts as behind it, and was pushed out of a solid that wasn't there (the lining through the skin, the lips "in" the teeth by 3 to 4 mm at one vertex).
- **The engine drew the teeth at half their triangles in every face picture.** `Teeth` is under 30% of the view's height whenever a whole face is in view, so the renderer draws its first simpler copy, which for a part without copies of its own is meshoptimizer's halving of its triangles wherever they fall: the facets. The build's own decimation of the whole jaw to 7,200 triangles, collapsing across teeth and gums alike, had done the same once already.
- **The "fangs"**: the bundle's upper canines hang 0.4 mm under the central incisors and 1.0 mm under the laterals.
- **The pale line between closed lips** was two things: the upper lip's wet underside mirroring the studio's floor through the 0.3 mm the seal left, and the teeth themselves behind that gap at the sides.
- **Light inside the mouth.** The studio's light is mostly sky light, which the engine takes an occlusion map off, with its reflection; a lamp's light it does not. So a dark tooth with a dark colour and no occlusion still mirrored the sky as a grey block.
- **The crossings** the last round reported were the lips cutting each other: a shape moves both lips by one rule made on the masculine frame, and nothing kept them off each other. Laughing's and Crying's pairs are nearly all at the eyes (the lids, 68 to 76 mm above the teeth's edge), not the mouth: the feminine Laughing's other 18 were at the mouth's corners. And the committed `face_checks.json` predated the last change to Laughing: on the commit this work started from, Laughing measured 69 pairs (masculine) and 90 (feminine), not 49 and 72.

### What changed

- **`teeth.py`** (new) makes the jaw: 0.85 of the bundle's size; the upper canines and premolars lifted 1.0, 0.7 and 0.4 mm (at the bundle's size) so the row's edges rise evenly from the middle; each tooth's root and the gum inside each tooth removed; the six front teeth of each jaw subdivided once (the bundle's teeth are cages for subdividing); the lower gum ended 9 mm under the teeth and closed along a keel, its floor inside the row cut away. Only the gums away from the teeth, the hidden shell over the palate and the lower front teeth are thinned.
- **Place.** Both frames: 2.7 mm behind the upper lip's inner side (feminine 6.2 before), no point in the flesh and no face of the jaw through a face of the body (a new test in the fit).
- **`Teeth_LOD1` and `Teeth_LOD2`** are made by the build. The first, which a face picture draws, has the full object's upper teeth and gum edges.
- **Shading, baked.** `Teeth`, `Gums` and `Tongue` each read a 64 px palette image by texture coordinates set per vertex: across it the colour (a tooth's neck, enamel, thin edge), up it the shade, divided between the colour and an occlusion image. A vertex is darker within 1.6 mm of the next tooth, within 2 mm of the gum, on sides that face into the mouth, and by how far back it stands (upper row: canines 0.84, first premolars 0.6, second 0.38, molars 0.2 and less; the lower row darker). The tongue is darker toward its root and underneath, with a darker groove. The lining (`Mouth`, on `Body`) is darkened in the skin's albedo and occlusion from 6 mm inside the lips to nearly black at 60 mm, and is matt for its first 5 mm.
- **Lips.** A shape now keeps the lips off each other on both frames (`Face.lips_apart`), and the shapes that mean the mouth to stay closed bring them back together where keeping them off the teeth parted them (`Face.lips_together`: the smile, frown, corner pull, wide, pucker, left and right, puff). The seal leaves 0.1 mm (0.3). The lower lip's red thins sooner toward the corners.

### Cost

| | Before | Now |
|---|---|---|
| `Teeth` | 7,632 triangles, 4,624 vertices, 652 kB | 9,022 triangles (teeth 7,094, gums 1,496, tongue 432), 5,022 vertices, 692 kB |
| `Teeth_LOD1` | (made by the engine at load: half) | 8,424 triangles, 658 kB |
| `Teeth_LOD2` | (made by the engine at load: a quarter) | 2,411 triangles, 219 kB |
| Images | none (three flat colours) | three palettes and an occlusion image, 64 px, 5.8 kB together |
| `RealisticParts.glb` | 29.40 MB | 30.45 MB |

Materials: `Teeth` roughness 0.16, `Gums` 0.34, `Tongue` 0.46, each with its palette as base colour and the shared occlusion image; `Mouth` now reads the skin's occlusion and roughness as well as its albedo. `TEXCOORD_0` on `Teeth` is a colour and a shade per vertex, not an unwrap.

### Checks

`face_checks.py --check` exits 0 with **0 failures** on both frames ([face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json); 1 before: the masculine `Mouth_Left`). No limit or measure was changed. `anim` and `avatar`: 222 pass. Head triangle pairs that cross (reported, not held to 0), on the commit this started from and now:

| | Masculine | Feminine |
|---|---|---|
| `Mouth_Smile` left, right | 30, 32 → 9, 8 | 32, 23 → 0, 0 |
| `Mouth_Pucker` | 0 → 0 | 163 → 1 |
| `Mouth_Right`, `Cheek_Puff` | 0, 0 → 0, 0 | 31, 22 → 0, 0 |
| `Jaw_Left`, `Jaw_Right` | 0, 0 → 0, 0 | 30, 28 → 10, 14 |
| U, WQ visemes | 0, 0 → 0, 0 | 61, 61 → 0, 2 |
| Happy, Excited | 36, 1 → 6, 1 | 43, 21 → 1, 1 |
| Laughing | 69 → 69 (the eyes) | 90 → 72 (the eyes) |
| Crying | 59 → 59 (the eyes) | 62 → 62 (the eyes) |

Identity ends (reported): `chin_forward` at -1 no longer puts the lining through the skin (31 and 22 pairs before) and leaves 2 points of the lower gum 0.6 to 0.8 mm in the flesh on each frame; `jaw_shift` at +1 leaves one feminine lining vertex 0.2 mm inside the gum. The Blender sheets beside this file are re-rendered. `measure.py --check` fails as before, on the shoulder's two lines and the sliders section it wasn't run with; its lines for the face and the teeth hold.

### Still short of the photographs

1. **Speckled teeth.** Where a tooth is half in the upper lip's shadow, the engine draws the shadow's edge as a pattern of dots (the laterals and canines in Laughing, [engine/teeth-close.jpg](engine/teeth-close.jpg)). It was there before this work and is the renderer's soft shadow, not the model: nothing in the package changes it.
2. **No gloss on the enamel.** The teeth are off-white with a neck and an edge, but no highlight shows in either light: with the mouth's occlusion on, the sky's reflection is gone too, and a lamp's highlight needs the lamp in the right place. Wet teeth and a wet tongue need the engine to shade a mouth's inside.
3. **The gum's edge is the bundle's cage**: a scallop over each tooth, of three or four straight pieces at twice the size.
4. **The tongue** is a smooth ramp with a darker middle. No surface, no wet highlight; `Tongue_Out` is still a small bump over the lower lip.
5. **Lower teeth at the corners** of an open mouth are still there as shapes, a little lighter than the black behind them (the AI viseme and `Jaw_Open` 0.5, masculine); the lower lip still stands as low at the sides as it did. On the masculine frame a speck of lit gum shows beside the upper lateral incisor on one side in those poses.
6. **The smile with the lips closed** shows a hair of pale line at one or two places on the feminine frame at twice the size; `Mouth_Pucker` on the masculine frame shows a short one at one corner.
7. **The upper teeth's height** is 1.5 to 3 mm lower under the lips than the measured display at rest (above).
8. **The mouth's back is black**, with no palate or throat to be seen; the photographs show dark red there under a camera's flash. The shade is one baked guess for every light.
9. The eyes' share of Laughing's and Crying's crossings is untouched (the lids are not this work's); the feminine `Jaw_Left` and `Jaw_Right` keep 10 and 14 pairs at one corner of the mouth, and the masculine smile 9 and 8.

## The mouth, round 2

Engine, 3840 x 2160, the straight-on `face-front` view, the mouth cut at full size; a row per pose, the masculine frame then the feminine.

- **[engine/mouth-closed.jpg](engine/mouth-closed.jpg)**: rest, the smile with the lips closed, Happy, Sad, Worried, Frustrated, Angry, Determined (studio light).
- **[engine/mouth-closed-hard.jpg](engine/mouth-closed-hard.jpg)**: rest, Happy, Worried, Angry under the hard light.
- **[engine/teeth-open.jpg](engine/teeth-open.jpg)**: `Jaw_Open` 0.5 and 1, Surprised, Fearful.
- **[engine/teeth-smile.jpg](engine/teeth-smile.jpg)**: Laughing, Excited, Crying, Disgusted.
- **[engine/teeth-hard-light.jpg](engine/teeth-hard-light.jpg)**: `Jaw_Open` 1, Laughing and Excited under the hard light.
- **[engine/teeth-close.jpg](engine/teeth-close.jpg)**: Laughing's upper row and the closed smile at twice the size.

- **[engine/teeth-other.jpg](engine/teeth-other.jpg)**: `Tongue_Out` with the jaw half open.

(`teeth-before.jpg` is the last section's: the commit that work started from.)

### 1. The mouth the same on both sides

How far a vertex of the mouth's skin lies from the mirror image of its opposite number (millimetres, the furthest):

| | Masculine before | now | Feminine before | now |
|---|---|---|---|---|
| Rest | 0.52 | 0.00 | 1.20 | 0.00 |
| `Mouth_Pucker` | 0.76 | 0.00 | 1.13 | 0.00 |
| `Mouth_Smile` left + right | 1.60 | 0.00 | 2.27 | 0.00 |
| Every other mouth shape and pair | 0.51 to 1.10 | 0.00 | 1.06 to 1.22 | 0.00 |
| `Cheek_Raise` left + right (an eye shape) | 0.52 | 0.05 | 1.20 | 0.05 |

What broke the mirror, in the order found: the bundle's own lips (0.3 mm masculine, 1.3 mm feminine); the seal (rays, and which way each quad is cut); every shape, which is made from nearest points and rays on a face that isn't even, so its two halves differ (the smile most: its cheek point is found from each eye); the jaw's hold on the lips; in the teeth, the lower gum's keel, gathered by nearness (2.45 mm at 24 hidden vertices) and one gum vertex left on one side by a tie in the inside-a-tooth test (0.99 mm).

What changed: `face.prepare` makes the mouth even at rest (everything within 40 mm of the lips' line, easing to nothing at 56 mm, and all of the lining), before the seal and again after it and after room is made for the jaw; the jaw's weights are made even over the same region; `Face.evened` makes every symmetric mouth shape its own mirror image and every left and right pair mirror images of each other, then takes the lips off each other again (the mean of two halves can cross where neither did) and makes them even once more; the keel is gathered onto stops chosen on one side and mirrored.

Held: `face_checks.py` now measures `mouth_asymmetry_mm`, `teeth_asymmetry_mm` and `tongue_asymmetry_mm` in every pose that is the same on both sides (rest, the controls with no left and right, seven left and right pairs together, the twelve visemes, the expressions whose recipes are even) and fails over **0.1 mm**. The largest now: mouth 0.07 (Laughing: its eye and cheek shapes reach into the mouth's region) mm, teeth 0.00 mm, tongue 0.00 mm.

### 2. Nothing lighter than the lips between closed lips

What parts the lips in the closed-mouth recipes is not a hair of error but controls that mean to: `Mouth_Narrow` 0.2 lifts the upper lip 0.24 mm and lowers the lower 0.4 mm (Worried, Determined); `Nose_Sneer` raises the upper lip 2.5 mm a side at 1, so 0.4 mm at 0.15 (Frustrated) and 0.8 mm at 0.3 (Angry); `Upper_Lip_Raise` 0.2 adds 0.9 mm (Angry: 1.7 mm in all); `Jaw_Forward` 0.2 carries the lower lip 1.1 mm forward from under the upper. The body's lips meet along a line, with the upper teeth 2.7 mm behind it.

`LipSeal` (new part, 68 triangles, `face.lip_seal`) is the depth real lips touch over: a flap hanging 1.4 mm from the upper lip's underside and one standing 1.8 mm up from the lower lip's top, each bound vertex by vertex to the loop behind its lip's edge, taking that vertex's move in every shape and its share of the jaw. Closed, each is inside the other lip. Parted by up to 3 mm, the mouth shows them, dark, in front of the teeth. The controls that bare the teeth draw them back (`Jaw_Open`, `Mouth_Open`, `Upper_Lip_Raise`, `Lower_Lip_Lower` all of it, `Nose_Sneer` half the upper one).

Seen ([engine/mouth-closed.jpg](engine/mouth-closed.jpg), [engine/mouth-closed-hard.jpg](engine/mouth-closed-hard.jpg)): in all eight closed-mouth poses on both frames under the studio light, and in rest, Happy, Worried and Angry under the hard light, the line between the lips is dark from corner to corner at full size, with no enamel in it; at twice the size the closed smile, Worried at 0.5 and Angry at 0.5 are the same ([engine/teeth-close.jpg](engine/teeth-close.jpg)). Angry at 1 parts the lips 1.7 mm and shows the flaps as a dark band, not teeth. The other four poses were not looked at under the hard light, and no pose but those three at twice the size.

### 3. The open mouth

- **Dark red, not black.** The engine's own shadows and screen-space occlusion already darken a mouth; the last section's bake on top of them made the black hole. The lining now keeps 0.45 of its light at the back (0.04 before) and only the throat's middle goes near black; the gums and palate fall to 0.3 at the back (0.1); the lining's occlusion takes less (the shade to the power 0.4). **What albedo and occlusion can and cannot do:** occlusion dims only the sky's light and its reflection, so it is right in the studio (mostly sky light) and does nothing under a lamp; albedo dims every light alike, so it is the only thing that works under the hard light, and too much of it is a black hole in the studio. The shade is divided between the two, so under the hard light the inside is lighter than under the studio's than a real mouth's would be. A proper answer is the engine shading a mouth's inside (or the lamp's shadow reaching into it).
- **The tongue fills the floor.** Its top at the tip is 3.8 mm under the plane the teeth meet on, a millimetre under the lower incisors' edges (6.5 mm before); with the jaw open its root sinks 6 mm (15), it is hollowed 0.8 mm (2.5) and thinned by 0.3 (0.65). `Tongue_Out` rises 7.8 mm (10.5) from the higher rest.
- **The lower front teeth show.** The last round stood the lower lip 5 mm higher on an open jaw to cover them; it is 1.2 mm now. Seen ([engine/teeth-open.jpg](engine/teeth-open.jpg)): a row of lower teeth over the lip at `Jaw_Open` 1 (thin) and in Surprised (clear), on both frames; at `Jaw_Open` 0.5 none show yet.

### 4. The corners of an open mouth

Not solved. The lower back teeth are lit to match the darker red round them (0.3 at the first premolar to 0.06 at the second molar) instead of near black, so they no longer stand as black saw teeth in the corners, but they are still there as shapes, and the notch at one corner of `Jaw_Open` 1 and Laughing was not looked into. What it needs: the lower lip and the cheek's lining carried round the corner in front of the lower back teeth as the jaw opens (the lining's faces there are a centimetre wide), which is the corner's weights and `Jaw_Open`'s shape again.

### 5. The closed smile

Not solved. The corner travels 7.0 mm out, 8.7 mm up and 6.8 mm back (13.1 mm in all) on both frames, which widens the mouth by a quarter: the travel was not what is short. The cheek's bulge is now 7.2 mm out and 6.2 mm up (6.0 and 5.2) and the fold from the nose's wing 4.2 mm deep (3.0). **The lips' line still meets in a point in the middle** ([engine/mouth-closed.jpg](engine/mouth-closed.jpg), [engine/teeth-close.jpg](engine/teeth-close.jpg)). Found why: a lip vertex takes the larger of two shares of the pull, its own along the lip and the skin's by distance from the corner (a bell 21 mm wide), and over most of the lip the skin's is the larger, which rises evenly from the middle to each corner: two straight halves. The lips' own share now goes by the square of the way to the corner, which changes little while the other governs. Giving the lips only their own share was tried and withdrawn: it left `face_checks.py` with one failure (Excited, masculine: one point of the teeth read as inside the flesh), which is the smile's corner under the open jaw again. What it needs: the two shares blended along the lip instead of the larger taken, with the open smile's corner checked as it is changed.

### 6. Small things

- The lit speck of gum beside an upper lateral (Fearful, masculine): not looked into this round.
- **The upper incisors' height.** Their edge is 4.9 mm under the closed lips' line; measured adults show 1.9 to 3.4 mm of them under the upper lip at rest. With Happy's lips closed nothing argues for the low place any more, and Laughing and Excited would show the whole crown with the gum's edge. Recommended: raise the jaw 1.5 mm (`parts.BITE_UNDER_LIPS` 0.0015 to 0) and take the same off `face.JAW_UPPER_LIP_UP` (3.8 to 2.3 mm) so an open jaw shows what it shows now. Not done here: it moves every picture of the teeth, and wants its own look.

### Checks

`face_checks.py --check`: **exit 0, 0 failures** ([face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json)), with the three new measures. `anim` and `avatar`: 223 pass. No limit or measure was loosened. `RealisticParts.glb` is 30.60 MB.

### Still short of the photographs

1. **The closed smile's lip line is a shallow V** on both frames (5, above), and the cheek and fold are only a little stronger: against the reference's one upward curve and full cheek this is the furthest of the closed mouths.
2. **The corners of an open mouth** (4): the lower back teeth still show as dim shapes in both corners at `Jaw_Open` 1, Laughing and Crying, and the notch at one corner was not looked into.
3. **The inside under the two lights.** Dark red with a black throat under the studio's light; under the hard light the same bake leaves it lighter than a real mouth's would be, since no lamp's shadow reaches in.
4. **Angry at 1** shows a dark band between the lips 1.7 mm apart, where the recipe's `Upper_Lip_Raise` 0.2 and `Nose_Sneer` 0.3 mean teeth or mean nothing. For lips together: both to 0, or `Nose_Sneer` to 0.15 at most. Worried and Determined part 0.6 mm by `Mouth_Narrow` 0.2; `Mouth_Pucker` 0.2 in its place keeps them together.
5. **The speckle on teeth** half in the upper lip's shadow, the missing gloss, the gum's edge in straight pieces, the tongue's plain surface: as the last section left them.
6. **No lower teeth at `Jaw_Open` 0.5**, and only a thin row at 1 (3). **The upper incisors' height** (6) and the gum's speck in Fearful: not done.
7. `LipSeal` was seen only from the front. From below or the side with the mouth a little open it is two thin dark walls behind the lips; no picture was taken of that.

