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
| `Teeth` | The teeth meet 1.6 mm under the lips' line just behind the upper lip (CO4 had them 13 mm lower and 12 mm further back). The bundle's lower shell and floor, which tore between head and jaw, are gone; a tongue (432 triangles, on the jaw) is made with `Tongue_Out`, `Tongue_Up`, `Tongue_Down`, and the lower teeth and the tongue carry their own `Jaw_Open` (review round 3, part 2). 7,632 triangles. |

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

Engine, 3840 x 2160, studio light; a row per pose: masculine face view and three-quarter view (the head cut from the full-body picture), then the feminine frame's. The mouths are cut from the pictures at full size.

- **[engine/mouth-open.jpg](engine/mouth-open.jpg)**: the masculine `Jaw_Open` 1 as it was before this part, then rest, `Jaw_Open` 0.5 and 1.
- **[engine/mouth-smile-laugh.jpg](engine/mouth-smile-laugh.jpg)**: the smile with the lips closed, Happy, Laughing, Surprised.
- **[engine/mouth-speech.jpg](engine/mouth-speech.jpg)**: `Mouth_Open` 1 and the AI viseme.

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

**Still short of the photographs:**

1. From the face camera (beside and above the mouth) the far row of lower teeth still shows beside the tongue as a column of three or four pale blocks, in `Jaw_Open` and most in Laughing. The lower arch is tilted with the jaw, so their tops face the camera; the tongue now covers their inner sides only. The arch is also large for the head (71 mm across the molars).
2. The tongue reads as a slab with a ridge across it, not as flesh: one flat colour, 16 points round, lit like the lips (nothing in the engine shades the inside of a mouth). At `Jaw_Open` 0.5 and in Surprised it fills the opening and no dark shows behind it.
3. On the feminine frame the teeth stand 6.2 mm behind the lips (2.7 mm on the masculine; the teeth's fit, unchanged), so a dark gap shows between the lower lip and the tongue from above.
4. The opening is wider than the photographs' in proportion at 0.5 (a long almond) and its corners are rounder than a mouth's at 1; the feminine opening at 1 is taller than wide.
5. Laughing: the upper teeth's last tooth on the near side hangs below the row like a fang, and the far cheek shows both rows of molars.
6. The teeth are flat grey (their material is CO7's), and in the full-body views the engine draws `Teeth` at a simpler level, where the tongue is a few blocks.
7. The feminine U and WQ visemes count more head triangles crossing than before (53 and 63 pairs, from 3 and 4; reported, not held to 0): with a jaw barely open the lips' corners now part less, and the pucker's crossing lips (167 pairs on its own, as before) show there.
8. At rest a thin pale line still shows between the lips at 4K (as before).
