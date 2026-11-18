# CO6: the realistic head and face

Milestone CO6 of the [Character Studio overhaul](../../../../plans/axcenity-engine-character-studio.md): what the realistic base body's face is, how it is made, how to rebuild and check it, what the engine shows and what still falls short. The package is [assets/characters/realistic/](../../../../assets/characters/realistic/README.md). **The engine's pictures are the judge** ([engine/](engine/)); the Blender sheets beside this file are for forms and for views the capture has no camera for.

## What the face has

| | |
|---|---|
| Head | The body's own head (no graft), vertex order the bundle's, with 512 more vertices after them round the lids (`rims.py`: each loop round an eye has twice the bundle's, so the lids' edges are curves at 4K). Repairs at rest (`face.prepare`): the feminine frame's lids are the masculine frame's carried to the feminine eye (the feminine opening sat 1.5 mm higher on the eyeball and bared the whole iris: the stare); the lids' lining is lifted off the eyeball; the lips are closed (they rested 1.9 to 2.5 mm apart) without crossing each other. The upper lids' fine detail is taken out of the skin maps (`face.soften_lids`): a closing lid stretches that skin fourfold and pores stretched into streaks. |
| Expressions | 44 blend shapes on `Body`, named as the facial profile's controls (`face.EXPRESSIONS`); the four `Eye_Look_*` controls turn the eye bones. `Jaw_Open`, `Jaw_Left`, `Jaw_Right` are the jaw bone's turn plus a shape that puts right what the turn gets wrong. The three `Tongue_*` shapes are empty on the body and move the tongue (a part follows the body's shape of the same name). |
| Identity | 61 face parameters (`faceparams.py`) in nine groups (eyes, brows, nose, cheeks, jaw, chin, lips, forehead, ears), with 67 shapes (CO6's 45 parameters, the brows' length a side, and R4's 14: [the R4 section](#r4-the-high-value-controls-and-the-frames-faces)). Most parameters have one shape. Brow height and brow arch have one shape for each end (`faceparams.SPLIT`): a lowered brow's fold comes forward over the lid, while a raised one lies back on the brow bone. Paired features have a parameter a side that names the other as its `mirror`; `nose_tilt` and `jaw_shift` push the middle off centre. Eye parameters also move or scale the eye bones. Each end moves the face 4 mm or more (2.2 mm or more on the coarsest copy). |
| `Eyes` | Per eye: `Sclera` (the white, textured), `Iris` (a dish set back inside the limbus, with the pupil), `Cornea` (a clear cap 1.2 mm proud of the white, alpha-blended). 2,256 triangles. Its own shape `Pupil_Size` widens or narrows the pupil; nothing drives it yet (below). |
| `EyeShell` | `EyeOcclusion` (a film between the lids, dark at their edges) and `TearLine` (the wet line), on the head, following the lids by shapes and drawn back behind them as they close. 2,612 triangles. |
| `Lashes`, `Brows` | Cards, alpha-blended. Lashes are rooted on the lid's outer edge and turn with it about the eyeball (188 triangles); a brow card lies on the skin and rides it whole (352 triangles for the two). Since wave 3 the brow card tapers to its tail and carries hair drawn hair by hair (graded density, soft edges, a dark brown from melanin), and the feminine frame's card is thinner, shorter and more arched (its own part of `Frame_Feminine`): the section at the end. |
| `Teeth` | The teeth, at 0.85 of the bundle's size, meet 1.6 mm under the lips' line, 2.7 mm behind the upper lip on both frames (CO4 had them 13 mm lower and 12 mm further back). The bundle's lower shell and floor, which tore between head and jaw, are gone; a tongue (432 triangles, on the jaw) is made with `Tongue_Out`, `Tongue_Up`, `Tongue_Down`, and the lower teeth and the tongue carry their own `Jaw_Open` (review round 3, part 2). Shaded by palette images, with two simpler copies of its own (the last section). 9,022 triangles. |

Materials and shading in the manifest: `Skin` skin; `Sclera`, `Iris`, `Cornea` eye; `Lashes`, `Brows` hair; `Mouth`, `Teeth`, `Gums`, `Tongue`, `EyeOcclusion`, `TearLine` standard. `RealisticParts.glb` is 32.0 MB with CO5's body shapes; `Body` and each simpler copy carry 146 shapes (172 since R3 and R4; the file is 38.5 MB).

## How the shapes are made

Procedural, on the head's own loops, with the eyeball and the teeth as things the skin must stay off (decided after the spike, below).

- **Lids** (`lids.py`): every lid vertex turns about a line through the eyeball's centre, by its loop; both rims end on one closing line, so closing is exact; the skin beyond the rim takes the turn smoothly and stays above its own lining. A shape moves a vertex in a straight line and a lid's path is an arc, so the closed lid is raised where it travels furthest by enough that half way the straight path clears the white of the eye.
- **Mouth, brows, cheeks, nose** (`face.py`): offsets along the muscle's pull with a falloff, the skin then put back on the resting surface, the lips left free. A pair's two sides are shares of one shape made for both.
- **Off the teeth, as a surface**: lip and lining vertices are moved off the teeth in the jaw pose the control gives, on both frames; what single vertices needed is then spread (`Face.spread`: the largest push within three edges, then averaged), so a lip clears the highest tooth near it with a smooth edge instead of taking each tooth's print.
- **Following parts**: `EyeShell` is a mix of lid vertices whose height shrinks as the lids meet; lashes turn with their roots; a brow card's three vertices across take the skin's move under its middle one; the teeth take `Frame_Feminine`, `Jaw_Forward`, `Face_Mouth_Height` and (R4) `Face_Philtrum_Length`.

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
PY -I assets/characters/humanoid/qa/measure.py --package realistic --only sliders --out docs/development/character-studio/co6/sliders.json --check assets/characters/realistic/qa/thresholds.json
cargo nextest run -p axcenity-anim -p axcenity-avatar --no-fail-fast
```

**The build is not byte-for-byte reproducible**, so after a rebuild git shows `RealisticParts.glb` as changed even when nothing was. Two runs of the same sources differ in two ways:
- The teeth's vertex order changes. The triangles are the same, and every check gives the same numbers.
- 9 of the skin normal map's 4.2 million pixels differ by one level.

`teeth.py`'s `remove_doubles` over a set of BMVerts is one cause; ordering it alone did not make two builds agree.

A face pose in the engine (both frames; see [visual-qa.md](../visual-qa.md)):

```text
AXCENITY_CAPTURE_DESCRIPTIONS=assets/characters/benchmarks AXCENITY_CAPTURE_ONLY=realistic-masculine-face-neutral AXCENITY_CAPTURE_FACE="Blink_Left:1,Blink_Right:1" AXCENITY_CAPTURE_SIZE=1920x1080 AXCENITY_CAPTURE_DIR=<folder> cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture
```

Since the refinement's R1 the realistic `face-neutral` files are folded into the cast: use `AXCENITY_CAPTURE_ONLY=realistic-masculine-cast-average` (the same character).

Tests: `crates/anim/tests/realistic_face.rs` (four: `check_face` finds nothing; every standard control moves a vertex or a bone; the parts carry the shapes they follow; the ARKit profile's controls all play). Red runs before the work: [red-run-anim.txt](red-run-anim.txt), [red-run-face-checks.txt](red-run-face-checks.txt). Now: [face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json).

## Measured (`face_checks.py`, both frames)

The build this README describes; the round 1 numbers are in git history. *Held to 0* means the measures `--check` holds to 0:
- lid vertices in the eyeball;
- eyeball vertices in the lid;
- teeth or tongue in the flesh;
- lips in the teeth;
- the mouth's lining through the skin.

| Measure | Masculine | Feminine |
|---|---|---|
| Eye open at rest, left and right (mm) | 9.53, 9.42 | 9.4, 9.3 |
| Lips apart at rest (mm) | 0.9 | 0.4 |
| Head triangle pairs that cross at rest | 1 | 0 |
| Full `Blink` and `Eye_Closed`: bare rays | 0, 0 | 0, 0 |
| Blink at 0.25, 0.5, 0.75: lid vertices in the opaque eyeball | 0, 0, 0 | 0, 0, 0 |
| Blink at 0.25, 0.5, 0.75: eyeball vertices in the lid (reported) | 0, 0, 0 | 0, 0, 0 |
| Blink at 0.25, 0.5, 0.75: lid vertices in the clear cornea (reported) | 3 (to 0.33 mm), 5 (to 0.45 mm), 0 | 2 (to 0.32 mm), 5 (to 0.43 mm), 0 |
| 48 controls, 7 left-and-right pairs, 12 visemes, 15 expressions: held to 0 | 0 | 0 |
| The same: largest left-right difference of the mouth (mm, held to 0.1) | 0.07 | 0.07 |
| Identity ends (90 poses, reported): held-to-0 measures | 0 | `jaw_shift` +1 and -1: 1 lip vertex each in the teeth (to 0.27 mm) |
| Identity ends: head triangle pairs that newly cross (reported) | `eye_size` -1 36 and 28; `brow_ridge` +1 15; `brow_height_left` -1 10, `brow_arch_left` -1 7, right 3 and 1; `cheekbone` +1 3 and 5; `eye_tilt` +1 5 and 4 | `lower_lip_fullness` -1 98; `nose_length` -1 8; `brow_ridge` +1 6; `eye_tilt` +1 5 and 4 |
| Identity ends, then a full blink: bare rays | 0 | 0 |

`--check` exits 0 with **0 failures** ([face-checks-run.txt](face-checks-run.txt)). Nothing was loosened; round 1's run had 7 failures, and CO4's file had 42 crossing pairs at the feminine mouth at rest and 3 on the masculine head.

The sliders check (`measure.py --only sliders`) is in [Identity parameters at their ends](#identity-parameters-at-their-ends):
- no new crossings on either frame;
- red on one line, from three false counts.

## What the engine shows

**The face as it is now:** [engine/final-expressions-masculine.jpg](engine/final-expressions-masculine.jpg) and [engine/final-expressions-feminine.jpg](engine/final-expressions-feminine.jpg). These are the build this README describes, straight on (`face-front`, 3840 x 2160 captures cropped to the face). They show the neutral face, Happy at 0.5 and 1, the other fourteen recipes, `Jaw_Open` at 0.5 and 1, `Blink` and `Eye_Wide`.
- *Good:* the open mouths are ovals with a dark red inside, a tongue and the lower teeth; the teeth are separate adult teeth; the closed smile is one curve; Laughing closes the eyes to dark arcs over a toothy open mouth; Sad, Worried and Crying bend the brows up at their inner third; a blink and a closed eye are one clean line; Sleepy keeps its lips together.
- *Still wrong:* see [What still falls short](#what-still-falls-short).

The review rounds below have the close-ups, and the identity section has the parameters' ends.

The sheets that follow are review round 1's, kept as the record of what was seen then.

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

Judged against reference photographs in 4K engine captures, after the three review rounds and the identity work below. Every item is a known defect, not accepted behaviour.

1. **No creases anywhere.** A smile, a squint, a frown and raised brows move the skin but fold none of it: no lines at the eyes' corners, on the forehead or between the brows (there are no wrinkle maps).
2. **Eyes.**
   - The shade over the eye is a film between the lids (`EyeOcclusion`) that does not know where the light is, so the whites are lit alike in every light.
   - The iris's edge is 24 segments, which shows as corners close up at 4K.
   - There is no caruncle.
   - Eye shading is CO7's.
3. **Lids.**
   - Under side light, a closed lid shows the head's own shadow edge as a lit blotch.
   - A half-shut eye (Sleepy, the squints) shows pale wedges at its corners.
   - The feminine `Eye_Wide` has a faint lit band above the lid's outer corner.
   - A half blink dips the lid into the clear cornea by up to 0.45 mm (reported). The lid never enters the opaque eyeball.
4. **Mouth** (details: [round 3's list](#still-short-of-the-photographs-2)).
   - The closed smile is milder than a real one.
   - An open mouth's corners are points, and the lower back teeth can be counted at three times the size.
   - `Jaw_Open` 1 is a bell, not a broad U.
   - U and WQ show the front teeth, TH shows no tongue, and FV's lower lip does not go under the upper teeth.
   - A band of dark lining shows under the upper lip in every open mouth.
5. **Expressions.** Determined and Frustrated are close to a mild Angry.
6. **Lashes and brows** are blended cards: a fringe and a patch, not hairs. (Brows: since wave 3 drawn hair by hair on a tapering card; see the end.)
7. **The pupil has no slider.** `Pupil_Size` moves 1.3 mm of the eye, under what a parameter must move the body by.
8. **Identity ends** (details: [its list](#still-short)).
   - The sliders check is red on one line, from three false counts in `eye_metrics`. The measure is unchanged; changing it is Paul's call.
   - Several ends still have head triangles crossing; the masculine `eye_size` -1 shows as a lit streak on the upper lid.
   - Raised brows keep the faint marks they had before the identity work: a pale chevron on the fold with one brow parameter raised, and a grey smear with both.
   - `jaw_shift` leaves one feminine lip vertex 0.27 mm in the teeth.
   - Six parameters are hard to see from the front.
   - The right-side parameters were judged from the numbers and from the left side's pictures.
9. **Rendering.** In the engine, a cut-out (MASK) card on a skinned, morphed part shows the backdrop through the skin behind its clear pixels. The face's cards are blended to avoid it. This is worth a look in the renderer before hair (CO8).
10. **The simpler copies** were looked at in Blender only (`face_lods.png`).

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

## The mouth, round 3

Engine, 3840 x 2160, the straight-on `face-front` view, the mouth cut at full size unless a line says otherwise; a column per pose, the masculine frame above the feminine. Studio light unless a line says hard.

- **[engine/mouth-smile.jpg](engine/mouth-smile.jpg)**: the smile with the lips closed at 0.5 and 1, Happy at 0.5 and 1.
- **[engine/mouth-smile-close.jpg](engine/mouth-smile-close.jpg)**: the closed smile and Happy at twice the size, under the studio light and the hard light.
- **[engine/mouth-smile-before.jpg](engine/mouth-smile-before.jpg)**: the same two at twice the size on the commit this round started from.
- **[engine/mouth-open.jpg](engine/mouth-open.jpg)**: `Jaw_Open` 0.15, 0.5 and 1, Surprised.
- **[engine/mouth-laugh.jpg](engine/mouth-laugh.jpg)**: Laughing at 0.5 and 1, Excited at 0.5 and 1.
- **[engine/mouth-corner.jpg](engine/mouth-corner.jpg)**: one corner of `Jaw_Open` 0.5, Laughing and Surprised at three times the size, as this round found it (above) and now (below), masculine.
- **[engine/mouth-parted.jpg](engine/mouth-parted.jpg)**: lips a little apart at two and a half times the size: `Jaw_Open` 0.03, 0.05 and 0.08, `Mouth_Open` 0.15, 0.2 and 0.3.
- **[engine/mouth-parted-hard.jpg](engine/mouth-parted-hard.jpg)**: `Jaw_Open` 0.05, `Mouth_Open` 0.2 and 0.3 under the hard light, the same size.
- **[engine/mouth-closed.jpg](engine/mouth-closed.jpg)**, **[engine/mouth-closed-hard.jpg](engine/mouth-closed-hard.jpg)**: rest, the closed smile, Happy, Sad, Worried, Frustrated, Angry, Determined under each light (these two replace round 2's).
- **[engine/mouth-visemes.jpg](engine/mouth-visemes.jpg)**, **[engine/mouth-visemes-half.jpg](engine/mouth-visemes-half.jpg)**: the twelve visemes at full weight and at half, as the engine's lip sync poses them (0.4 of `Mouth_Open` given to the jaw).
- **[engine/mouth-fear-cry.jpg](engine/mouth-fear-cry.jpg)**: Fearful and Crying as their recipes have them and with the jaw and lower lip numbers of section 4 (set by hand), Disgusted.
- `teeth-open.jpg`, `teeth-smile.jpg`, `teeth-hard-light.jpg`, `teeth-close.jpg` and `teeth-other.jpg` (the lists above) are this build's again; `teeth-before.jpg` is as it was.

### 1. The closed smile's lip line

**Found.** As round 2 said: a lip vertex took the larger of its own share of the corner's pull and the skin's, and the skin's is the larger over most of the lip and rises evenly from the middle. On the masculine frame the lips' line stood 0.7, 1.1, 1.6, 2.0 and 2.7 mm over its middle at 5, 10, 14, 19 and 23 mm from it: two straight halves at 6 to 10 degrees. Two smaller things made a point of their own once that was gone. The little of the pull that passes across the middle rose faster on its own side than it fell on the other, which left the middle vertex 0.4 mm under its neighbours. And taking the lips off each other in steps of 0.12 mm a lip left the middle of the smile 0.3 mm further apart than the lips beside it.

**Changed** (`Face.smile`, `Face.lips_apart`). The two shares are blended along the lip: the lip's own from the middle out to 0.45 of the way to the corner, the skin's from 0.95, and off the lips the skin's as before, so the lip and the skin beside it move together at the corner (the lips' own share all the way was what put a tooth in the flesh in round 2). The lip's own share rises by a power of 2.6 (2), and the part that crosses the middle adds up to the same from side to side. The lips come off each other in steps of 0.04 mm with one step of room.

**Now.** The masculine line stands 0.05, 0.0, 0.2, 0.3, 0.8, 1.8, 2.6, 3.3, 4.0 and 4.4 mm over its middle at 5, 9, 13, 17, 21, 25, 29, 32, 33 and 34 mm from it: level to within 0.3 mm over the middle half and turning up into the corners. The feminine line stands 0.1, 0.1, 0.4, 0.6, 1.1, 2.2, 3.2, 3.6, 3.8 and 4.0 mm over its middle at 4, 7, 11, 14, 18, 22, 25, 27, 29 and 29 mm: level to within 0.4 mm over the middle third. The lips at the middle are 0.07 mm further apart than at rest. Seen ([engine/mouth-smile.jpg](engine/mouth-smile.jpg), [engine/mouth-smile-close.jpg](engine/mouth-smile-close.jpg)): the closed smile at 0.5 and 1 and Happy at 0.5 and 1 are one curve on both frames, with no point in the middle at twice the size under either light. Excited and Laughing read as they did ([engine/mouth-laugh.jpg](engine/mouth-laugh.jpg)); the smile crosses no faces on either frame.

### 2. The corners of an open mouth

**Found.** The lower back teeth turn about a hinge they are nearer to than the lip in front of them is, so with the jaw open they drop less than the lip: at `Jaw_Open` 1 the incisors' tips are 36 mm down, the first molars' 22 mm and the last molars' 15 mm, 2 mm under the corner of the mouth, while the lip's rim beside them was 23 to 27 mm down. Seen from the front that is a row of teeth climbing each side of the opening inside the lip. The mouth's bag is also 5 mm wide of the lower teeth behind the corner, so one looks down a gap at the molars. And they were black: a tooth's shade is its colour's and its occlusion's at once, and with the occlusion low enough to darken a white tooth the engine draws it black whatever its colour (a darker "no light" colour changed nothing), against lining that keeps most of its occlusion and is dark red.

**Changed.**
- `rig._jaw`: the lower lip's hold falls off as `1 - 0.70 * u ** 2.5` (3), so its rim stands higher toward the corners.
- `face.jaw_open`: the lower lip stands 2.5 mm higher again and 1 mm further in between its middle and each corner (`JAW_LIP_SIDE_UP`), and the cheek's lining behind each corner comes in up to 6 mm against the lower back teeth (`JAW_CHEEK_IN`; it is kept off them as the lips are).
- `teeth.py`: the teeth's palette begins with a tooth in the mouth's own shade (a dark red), and the lower second premolars and molars are that, lit 0.4, instead of white teeth at a shade of 0.03 to 0.1.

Tried and withdrawn: a power of 2 with 4 mm of rise (the opening became a funnel); the lining drawn in 11 mm (13 and 17 crossing pairs of faces at `Jaw_Open` 1); the corners taking 0.38 of the jaw (no fewer teeth in view).

**Now.** Points of the lower teeth and gums more than 12 mm from the middle that the body leaves in view of the `face-front` camera (of 1,050; the build's own posing, tongue left out):

| | Masculine before | now | Feminine before | now |
|---|---|---|---|---|
| `Jaw_Open` 1 | 270 | 28 | 50 | 0 |
| `Jaw_Open` 0.5 | 300 | 128 | 94 | 34 |
| Laughing | 488 | 296 | 310 | 116 |
| Surprised | 416 | 260 | 196 | 62 |
| Excited | 328 | 249 | 216 | 140 |

Seen ([engine/mouth-open.jpg](engine/mouth-open.jpg), [engine/mouth-laugh.jpg](engine/mouth-laugh.jpg), [engine/mouth-corner.jpg](engine/mouth-corner.jpg)): no black blocks in the corners of any of these on either frame under the studio light; what shows of the lower back teeth is dark red, the tone of the lip's inner side beside it, with a thin pale rim along each tooth. Under the hard light the corners are in the lip's shadow, as before. The three-quarter and side views show no lining through the cheek, and `face_checks.py` counts none. **Not solved:** the corner itself is still a point where the two lips meet, and at three times the size the lower back teeth are still shapes one can count.

### 3. The lips' seal, and the visemes

**Found.** A `LipSeal` flap was drawn back by its own height at a control's full weight, so at `Jaw_Open` 0.5, with the lips 18 mm apart, half of each still stood on its lip; its shade was 0.22 of the lining's red with the occlusion to match, which the engine draws black; and the upper flap faced into the mouth, so the front saw its unlit back.

**Changed** (`face.lip_seal`). Each flap stands out of its lip along the lip's own middle (`_flesh_axis`) and is drawn back along that line and on into the lip's flesh, several times its height at a control's full weight: a shape moves a vertex in a straight line, so a flap cannot stop when it is gone, and inside the lip nothing sees it. The upper flap hangs 2.2 mm (1.4) and the lower stands 1.2 mm (1.8); they end 0.82 of the way to the corners (0.95); both face the front; their shade is 0.55 of the lining's red. The rates: `Jaw_Open` 8, `Mouth_Open` 2.6, `Upper_Lip_Raise` 1.55, `Lower_Lip_Lower` 1.75, `Nose_Sneer` 0.85, `Mouth_Narrow` 1.15 (new: the WQ viseme left the flaps standing in a mouth 5 mm open).

**What a straight line allows.** A flap shrinks evenly from the first, so it cannot both hide the teeth up to 1.5 mm and be gone at 1.5 mm. As built, with the build's own posing:

| Lips apart at the middle | Front teeth points the flaps leave in view (masculine, feminine) | Flaps |
|---|---|---|
| 1.2 mm (`Jaw_Open` 0.03) | 0 of 124, 0 of 154 | 0.76 of their height |
| 1.2 mm (`Mouth_Open` 0.15) | 2 of 168, 12 of 236 | 0.6 |
| 1.6 mm (`Mouth_Open` 0.2) | 66 of 250, 114 of 304 | half |
| 2.0 mm (`Jaw_Open` 0.05) | 32 of 242, 20 of 233 | 0.6 |
| 2.3 mm (`Mouth_Open` 0.3) | 316 of 386, 380 of 457 | 0.2 |
| 3.1 mm (`Mouth_Open` 0.4, `Jaw_Open` 0.08) | all | gone for `Mouth_Open`; 0.36 for the jaw |
| 4.9 mm (`Jaw_Open` 0.125) and more | all | gone |

So the lip controls have the flaps gone by 3 mm of parting and the jaw by 5 mm, not by the 1.5 mm asked for: at 1.5 mm for the jaw (a rate of 26) a flap would be more than 50 mm inside the face at full opening. A laugh takes the upper flap 15 mm into the upper lip as it is. In the closed-mouth expressions (Angry, Worried, Determined, Frustrated, `Jaw_Forward`) the flaps still leave no tooth point in view.

**Held.** `face_checks.py` now measures `lips_open_mm` (how far the lips are apart at the middle, by rays from the front) and `seal_in_view` (vertices of `LipSeal` out of the body's flesh), and fails a pose with the lips more than 6 mm apart and any of it in view. It found two things the pictures had not: the WQ viseme (above), and, once the cheek's lining moved in section 2, the flaps' ends out through it beside the corners (their shorter reach and the lower flap's height are from that).

Seen ([engine/mouth-parted.jpg](engine/mouth-parted.jpg), [engine/mouth-parted-hard.jpg](engine/mouth-parted-hard.jpg), [engine/mouth-open.jpg](engine/mouth-open.jpg), [engine/mouth-closed.jpg](engine/mouth-closed.jpg), [engine/mouth-closed-hard.jpg](engine/mouth-closed-hard.jpg)): no black line along the lower lip at `Jaw_Open` 0.5 or 1, in Excited at 0.5 or round the teeth in Disgusted, on either frame; `Mouth_Open` 0.3 is lips a little apart with teeth between them and no stripes; `Jaw_Open` 0.03 and `Mouth_Open` 0.15 are a dark red slit with no enamel; at `Jaw_Open` 0.05 and `Mouth_Open` 0.2 a sliver of the teeth's edge shows under the red. Under the hard light the flaps are the same dark red. The eight closed-mouth poses are dark from corner to corner under both lights. A dark band stays under the upper lip's edge in an open mouth: that is the lip's own lining in its shadow, not a flap.

**The visemes** ([engine/mouth-visemes.jpg](engine/mouth-visemes.jpg), [engine/mouth-visemes-half.jpg](engine/mouth-visemes-half.jpg); all twelve looked at, at 1 and at 0.5, both frames):

| Viseme | At 1 | At 0.5 |
|---|---|---|
| Silence | Closed lips. | The same. |
| MBP | Closed lips, a little gathered. Reads. | Closed. |
| FV | The upper teeth bared over the lower lip. Reads, though the lip does not tuck under the teeth. | A slit with a sliver of teeth. |
| TH | A small opening with both rows of teeth. The tongue's tip does not show between them, so it reads as TDN. | A slit. |
| TDN | A small wide opening, the teeth nearly together. Reads. | A slit. |
| L | Open a little, the upper teeth and a dark space under them. The raised tongue is not seen from the front. | Small opening with teeth. |
| KG | Open a little, tongue and teeth. Reads. | Small opening. |
| E | Wide and shallow with both rows. Reads. | Wide slit with teeth. |
| AI | An open oval, both rows and the tongue. Reads. | A shallow oval. |
| O | A rounded opening with both rows of teeth in it. Rounder than it was; reads as O. | A small oval. |
| U | A small rounded opening, the upper teeth filling its top. Reads as a small O more than as U. | Lips a little gathered and apart. |
| WQ | As U, a little smaller. | As U. |

**Changed for them** (`face.py`, `Mouth_Narrow`): the control parts the lips in the middle only (all of its parting out to 0.3 of the way to a corner, none from 0.75), since a rounded mouth is closed toward its corners. Parted from corner to corner, O, U and WQ were a slot with both rows of teeth in it.

### 4. Fearful and Crying

Seen again after 2 and 3 ([engine/mouth-fear-cry.jpg](engine/mouth-fear-cry.jpg)). The black line round the opening and the black teeth at the sides are gone from both. What is left is the recipes, not the shapes:

- **Fearful** is a slit 6 mm high with the upper teeth's edge along its top and the lower teeth's tops along its bottom, from `Jaw_Open` 0.2 with `Lower_Lip_Lower` 0.3: the lower lip is drawn down off the teeth further than the jaw opens. With `Jaw_Open` 0.3 and `Lower_Lip_Lower` 0.1 (the rest as it is) it is an open, stretched mouth with the upper teeth and the tongue, and the lower teeth behind the lip. Tried in the engine with the controls set by hand; the recipe file is not changed here.
- **Crying** shows the lower row standing 4 mm over the lip from side to side, from `Lower_Lip_Lower` 0.4 with `Jaw_Open` 0.25. With `Jaw_Open` 0.3 and `Lower_Lip_Lower` 0.15 the lower teeth are a thin row and the mouth still turns down.

### 5. The upper incisors' height

Tried and withdrawn. With the jaw raised 1.5 mm (`parts.BITE_UNDER_LIPS` 0.0015 to 0) and the same taken off `face.JAW_UPPER_LIP_UP` (3.8 to 2.3 mm), as round 2 recommended:

- `face_checks.py --check` went to **7 failures**: the lips in the teeth in the feminine Angry, Frustrated and Sad (1 or 2 vertices, to 0.84 mm), 4 tooth points in the flesh in the feminine WQ viseme (to 0.72 mm), and `LipSeal` in view in `Jaw_Open` 1 (feminine) and Laughing (both frames).
- The pictures were not better. The jaw moves as one, so the lower teeth rose 1.5 mm over the lower lip as well: Excited, Laughing and Surprised showed a full lower row where they show little or none now, and with the upper lip rising less on an open jaw the upper crowns were cut to about half their height in Laughing and to a sliver at `Jaw_Open` 0.5. The references show the upper crowns whole and little of the lower row.

The incisors' edge stays 4.9 mm under the closed lips' line. Moving it wants the upper and lower rows placed separately (the upper row up, the lower left where it is) and the closed-mouth expressions kept off the teeth as wholes; neither was tried.

### Checks

`face_checks.py --check`: **exit 0, 0 failures** ([face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json)), with the new measure. No limit or measure was loosened; the one new measure only adds a way to fail. `anim` and `avatar`: 223 pass. The mouth's, the teeth's and the tongue's largest distance from their mirror images is unchanged (0.07, 0.00, 0.00 mm). Head triangle pairs that cross (reported, not held to 0) are as round 2 left them but for the feminine `Jaw_Left` and `Jaw_Right` (10 and 12, now 6 and 8) and the feminine WQ viseme (0, now 5). `LipSeal` is 48 triangles (68).

### Still short of the photographs

1. **The closed smile** is one curve now, but against the reference it is a mild one: the cheeks rise and the fold from the nose's wing deepens less than a smile's do, and there is no dimple beside the corner.
2. **The corners of an open mouth** are dark red and no longer black, but the corner is still a point, and the lower back teeth are still to be counted at three times the size, each with a pale rim. The reference's corner is a soft fold of lining with no teeth in it. Laughing and Excited leave the most in view, on the masculine frame, whose mouth is wide for its teeth.
3. **`Jaw_Open` 1 is narrower at the bottom than at the top** (a rounded bell), where the reference's wide-open mouth is a broad U.
4. **The flaps** are in view, as the lining's red, with the lips between 1.5 and 3 mm apart (5 mm for the jaw alone); under 1.5 mm they hide the teeth, and at 2 mm a sliver of enamel shows under them. Every control at full weight together takes a flap out of its lip (never in a recipe or a viseme).
5. **U and WQ** show the front teeth in an opening 20 mm wide, where a real one is a smaller, darker ring of lips well forward of the teeth: `Mouth_Pucker` gathers the lips to 0.7 of their width and 5.5 mm forward, and would need about half and more. **TH** shows no tongue, and **FV**'s lower lip does not go under the upper teeth.
6. **A band of dark lining** under the upper lip's edge in every open mouth; the speckle on teeth half in shadow, the missing gloss, the gum's edge and the tongue's plain surface are as round 2 left them.

## Identity parameters at their ends

`measure.py --package realistic --only sliders` sets every pair of the manifest's parameters to every combination of their range ends, on both frames (2,556 pairs, 10,082 bodies a frame), and counts pairs of the body's triangles that newly cross; for each parameter alone it also counts eye and teeth points in the flesh. The package's thresholds hold all of these at 0. It had not been run since the face's 45 parameters were added, and on the build before this section it was red: 14 masculine and 8 feminine bodies with new crossings (brow height with brow arch, the temples with the ears, eye size and eye height with a lowered brow), and 17 parameter ends a frame with eye or teeth points in the flesh.

### What was found

1. **The harness left the eyeballs where they rest.** An eye parameter is a shape that moves the lids and a `bones` driver that moves or scales the eye bone (eye height moves it 4.5 mm). `Character.set_sliders` gave `set_look` the manifest's shape weights as `build`, and `eye_transform` moved the eye bones for the frame and for `face` shapes only, so the lids moved off an eyeball left in place. The engine moves it: `avatar::resolve` sums each parameter's `bones` drivers (an offset times the mapped weight, a scale `1 + (scale - 1) * weight`), and `Model::bake` (`crates/assets/src/model/identity.rs`) carries a model that has none of a layer's shapes (the `Eyes`) by the bone and adds the change to every pose; `gameplay`'s applier bakes `resolved.identity`. Captured in the engine on that build, `eye_height_left` -1 has the iris where it is at rest in the opening, the whole eye about 4 mm lower, and `eye_size_left` -1 a smaller iris, with no eyeball through a lid (the eye shapes are not changed here, so those pictures are the same before and after) ([engine/identity-eyes.jpg](engine/identity-eyes.jpg)). `face_checks.py` already posed the bones and found these ends clean. 13 of the 17 entries on the masculine frame and 14 on the feminine were this; the rest were the tilt and the chin (below) and, on the masculine frame, `eye_size_left` -1, which is a false count (still short, below).
2. **Brow height and brow arch** at -1 together bring the brow down 11.6 mm, nearly all the height between the brow and the lash line. The identity brow shapes put every skin vertex back on the resting surface, the lids' and the fold's too, so the fold under a lowered brow was laid onto the nearest resting skin, the lid's own: the fold's loops 7 and 8 crossed the lid's loops 2 to 4 (20 to 43 pairs). The brow controls had already learned not to slide there (`brow_slide`). On the feminine frame the fold also stands no further forward than the lid below it, so even unslid it came straight down through the lid. Eye size -1 (the skin round the eye drawn in toward it) and eye height +1 (the lid's skin raised toward the brow) with a lowered brow crossed the same way on the masculine frame.
3. **Temple width** moved the skin round each ear's root out up to 3.7 mm and left the ear itself where it was (`~self.ear`), so the skin went out through the ear's root; with the ear laid back or raised that was 4 to 15 pairs.
4. **Ear protrusion** measured each ear point's reach from a plane 12 mm inside the ear's middle, so the ear's root moved 2.5 mm against the skin it grows from; laid back with the ear raised, the ear's root went into the skin above it (4 to 6 pairs).
5. **Chin forward** at -1 drew back the skin inside the lower lip that faces the teeth (face set 22, counted as skin) by 2.3 to 2.8 mm, and the front of the lower gum stood 0.84 mm in it (4 points); the shape was kept off the teeth at +1 only, where it moves away from them.
6. **Eye tilt** at +1 raised the rims straight up, so the lower lid's inner edge went 0.4 mm into the eyeball on the outer half, where the eyeball's surface faces down (3 skin points in the eyeball, 1 eyeball point in the lid, on both frames).

### What changed

The harness (`qa_scene.py`): `set_sliders` moves and scales the eye bones by the parameters' `bones` drivers as `avatar::resolve` sums them (`manifest_bones`; the frame's own drivers are still taken from the build data), and `slider_pairs`' notes say why. No limit, threshold or counted quantity changed.

The shapes (`face.py`):

- **Brows**: brow height and brow arch have a shape for each end (`faceparams.SPLIT`). A blend shape moves its vertices along one straight line both ways, and a lowered brow's fold has to come forward over the lid where a raised one lies back on the brow bone: one shape that did the first did the opposite of the second. In the manifest each of these parameters drives its lowering shape (`Face_Brow_Height_Left` and so on) from 0 to -1 (`"from": [-1, 0], "to": [-1, 0]`) and its raising shape (`Face_Brow_Height_Up_Left` and so on) from 0 to 1; `avatar::resolve` clamps each span, so only one of the two moves at a time. The body and its copies carry 146 shapes (142).
  - *Lowering*: the skin is put back on its resting surface as the brow controls put it (`brow_slide`: not on the lids and the fold), and the upper lid's loops 5 to 10 roll forward as they come down (`BROW_ROLLS`, up to 0.3 of the drop), so a much lowered brow's fold comes down over the lid, as a hooded lid's does. The roll is none at the inner corner and whole from a third of the way along the eye, and averaged over the lid (`BROW_ROLL_TAPER`, `BROW_ROLL_SMOOTH`). The drop is 5.33 and 6.48 mm (`BROW_RISES`, 5.2 and 6.4 before) so that the largest moves are no smaller than they were. The masculine frame needs no roll at all (its fold stands forward of the lid); the feminine frame's pair at -1 crossed in 28 places without it and still once (brow arch with eye size) with a roll of 0.2.
  - *Raising*: the shapes of the build before, unchanged (the raised skin slid back on its resting surface, the fold onto the brow bone; `BROW_RAISES`). The first version of this section had one shape for both ends, with the roll: it took a raised fold back behind the skin above it, which showed as a pale crescent over the lid with one brow parameter or both at +1.
- **Temple width** carries the ears out with the skin they grow from.
- **Ear protrusion** moves each ear point by its own distance from the head's skin (`EAR_OUT_REACH`, 12 mm for the whole 4.8 mm, at most one and a half times that): the root stays on the head and the rim moves most.
- **Chin forward**: the skin inside the lower lip that faces the teeth moves as little as the lining (0.15 of the chin's move). Making the shape at its -1 end off the teeth was tried first: it held that skin off the gum but left the lip's flesh 1 mm thick on the feminine frame, which crossed itself with the chin shortened (46 pairs) or the mouth lowered (24).
- **Eye tilt**: what of a rim vertex's move would take it nearer the eyeball than 0.9 of its resting height over it, at either end, is taken off along the line from the eyeball's centre and spread over the lid round it (`TILT_KEEPS`, `TILT_SPREADS`); the rims still rise and fall alike, and a full blink on a tilted eye leaves no ray bare. Turning the lower lid with the skin round it instead left a blink open (56 rays at -1), and the same change without the spreading moved single vertices of the lid's edge.

Every pair of the face's 45 parameters at its ends was free of new crossings on both frames in a fast copy of the measure first (head triangles only; it gives the same counts as the harness on the build before), then in the harness itself (below).

### Numbers

The sliders check, both frames (10,082 bodies a frame):

| | Masculine before, after | Feminine before, after |
|---|---|---|
| Bodies with new crossings | 14, 0 | 8, 0 |
| Parameter ends alone with new crossings | 0, 0 | 0, 0 |
| Parameter ends alone with eye or teeth points in the flesh | 17, 3 | 17, 0 |

Each pair that crossed, before and after (pairs of triangles newly crossing):

| Pair | Masculine before, after | Feminine before, after |
|---|---|---|
| `brow_height_left=-1, brow_arch_left=-1` | 20, 0 | 43, 0 |
| `brow_height_right=-1, brow_arch_right=-1` | 24, 0 | 32, 0 |
| `temple_width=1, ear_out_right=-1` | 15, 0 | 15, 0 |
| `temple_width=1, ear_out_left=-1` | 12, 0 | 14, 0 |
| `eye_size_left=-1, brow_arch_left=-1` | 11, 0 | 0, 0 |
| `eye_size_left=-1, brow_height_left=-1` | 10, 0 | 0, 0 |
| `eye_size_right=-1, brow_height_right=-1` | 8, 0 | 0, 0 |
| `eye_size_right=-1, brow_arch_right=-1` | 7, 0 | 0, 0 |
| `temple_width=1, ear_height_left=1` | 6, 0 | 7, 0 |
| `ear_height_left=1, ear_out_left=-1` | 6, 0 | 6, 0 |
| `temple_width=1, ear_height_right=1` | 4, 0 | 6, 0 |
| `ear_height_right=1, ear_out_right=-1` | 4, 0 | 6, 0 |
| `eye_height_right=1, brow_height_right=-1` | 3, 0 | 0, 0 |
| `eye_height_right=1, brow_arch_right=-1` | 3, 0 | 0, 0 |

Each parameter end with eye or teeth points in the flesh: before; with the eyes posed by the harness's correction, on the build before; after. Each cell is skin points in the eyeball / front eyeball points in the lid / teeth points in the lip or gums.

| End | Masculine: before, with the eyes posed, after | Feminine: the same |
|---|---|---|
| `eye_height_left=-1` | 53/185/0, 0/0/0, 0/0/0 | 49/186/0, 0/0/0, 0/0/0 |
| `eye_height_left=1` | 53/115/0, 0/0/0, 0/0/0 | 51/119/0, 0/0/0, 0/0/0 |
| `eye_height_right=-1` | 53/183/0, 0/0/0, 0/0/0 | 50/185/0, 0/0/0, 0/0/0 |
| `eye_height_right=1` | 52/116/0, 0/0/0, 0/0/0 | 48/119/0, 0/0/0, 0/0/0 |
| `eye_spacing_left=-1` | 104/50/0, 0/0/0, 0/0/0 | 96/54/0, 0/0/0, 0/0/0 |
| `eye_spacing_left=1` | 65/93/0, 0/0/0, 0/0/0 | 63/96/0, 0/0/0, 0/0/0 |
| `eye_spacing_right=-1` | 106/46/0, 0/0/0, 0/0/0 | 100/54/0, 0/0/0, 0/0/0 |
| `eye_spacing_right=1` | 64/94/0, 0/0/0, 0/0/0 | 64/96/0, 0/0/0, 0/0/0 |
| `eye_depth_left=-1` | 116/0/0, 0/0/0, 0/0/0 | 94/0/0, 0/0/0, 0/0/0 |
| `eye_depth_left=1` | 107/20/0, 0/0/0, 0/0/0 | 94/17/0, 0/0/0, 0/0/0 |
| `eye_depth_right=-1` | 115/0/0, 0/0/0, 0/0/0 | 95/0/0, 0/0/0, 0/0/0 |
| `eye_depth_right=1` | 104/22/0, 0/0/0, 0/0/0 | 95/17/0, 0/0/0, 0/0/0 |
| `eye_size_left=-1` | 116/91/0, 0/1/0, 0/1/0 | 103/110/0, 0/0/0, 0/0/0 |
| `eye_size_right=-1` | 116/89/0, 0/0/0, 0/0/0 | 101/102/0, 0/0/0, 0/0/0 |
| `eye_tilt_left=1` | 3/1/0, 3/1/0, 0/1/0 | 3/1/0, 3/1/0, 0/0/0 |
| `eye_tilt_right=1` | 3/1/0, 3/1/0, 0/1/0 | 3/1/0, 3/1/0, 0/0/0 |
| `chin_forward=-1` | 0/0/4, 0/0/4, 0/0/0 | 0/0/4, 0/0/4, 0/0/0 |

Each changed shape's largest move (millimetres, left and right; `Realistic.build.json`, `largest_move_mm`):

| Shape | Before | After |
|---|---|---|
| `Face_Brow_Height` (the lowering shape, at -1) | 5.47, 5.49 | 5.50, 5.50 |
| `Face_Brow_Height_Up` (the raising shape, at +1) | 5.47, 5.49 | 5.47, 5.49 (the shape before) |
| `Face_Brow_Arch` (lowering) | 6.34, 6.37 | 6.39, 6.38 |
| `Face_Brow_Arch_Up` (raising) | 6.34, 6.37 | 6.34, 6.37 (the shape before) |
| `Face_Ear_Out` | 6.75, 6.81 | 7.59, 7.59 |
| `Face_Head_Width` | 5.19 | 5.19 (and the ears now go with it) |
| `Face_Chin_Forward` | 5.2 | 5.2 |
| `Face_Eye_Tilt` | 5.44, 5.43 | 5.44, 5.43 |

The sliders check's output is [sliders.json](sliders.json) and its `--check` run [sliders-check.txt](sliders-check.txt).

### In the engine

3840 x 2160 captures of the description files for every offending pair and end (left side, which the face view is turned toward), before (the build at the start of this section) and after, on both frames, cropped at about full size; each tile is labelled `before` or `final` and with the description's name (`m` masculine, `f` feminine).

- **[engine/identity-eyes.jpg](engine/identity-eyes.jpg)**: rest, `eye_height_left` -1, `eye_size_left` -1, `eye_tilt_left` +1 before and after. The eyeball goes where the lids go; no white or iris shows through a lid. Before, the tilted eye's white ran over the lower lid's edge on the outer half (plainest under the hard light, [engine/identity-tilt-hard.jpg](engine/identity-tilt-hard.jpg), the outer corner close up); after, the lid's edge lies over it, and the corner shows only the thin light line it shows at rest.
- **[engine/identity-brows.jpg](engine/identity-brows.jpg)** (`before` and `after`): both brows at -1, eye size -1 with arch -1, brow height +1, brow arch +1, both brows at +1, and brow height +1 with arch -1. Before, a lowered brow left a pale strip under the brow's inner end (the fold's skin turned up to the light) and a hard edge where the fold lay on the lid, with the lashes through it. After, the fold comes down over the lid as a hooded eye's, with no pale strip; at -1 together it covers the lid down to the lashes. Raised, alone or together, the brows are as before (the raising shapes are the build before's).
- **[engine/identity-brows-split.jpg](engine/identity-brows-split.jpg)**: brow height +1, both brows at +1 and both at -1, in the first version of this section (`first`: one shape a parameter, rolled back as it rose) and now (`after`). The first version's pale crescent over a raised lid is gone; the lowered brows are the same.
- **[engine/identity-ears.jpg](engine/identity-ears.jpg)** (from the front, 9 degrees aside) and **[engine/identity-side.jpg](engine/identity-side.jpg)** (profile; the last two columns are the chin): before, a wider temple left a dark notch where the skin went through the top of the ear's root, and an ear raised and laid back stood a spike of skin at the top of its root; after, the ear's root stays on the head and goes out with it, and an ear laid back turns in at the rim. The chin at -1 recedes as much as before, with no gum through the lip.

### Checks

- `measure.py --only sliders --check`: no new crossings on either frame (0 of 10,082 bodies each) and no parameter end with new crossings; red on one line, `sliders/frames/masculine/alone_eye_or_teeth_in_flesh` 3 (at most 0; still short, 1). Its other lines in that mode say "not measured". About 14 minutes.
- `measure.py --check` (every other section): red on the shoulder's lines only (`shoulder_abd_150` at 0.463 and 0.513 under 0.54 and the 0.8 target, `shoulder_abd_90` at 0.847 and 0.88 under its targets), as before; its `sliders` lines say "not measured" in that mode.
- `face_checks.py --check`: **0 failures** ([face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json)); nothing outside the identity entries changed. Identity entries (reported, not held) that changed: `chin_forward` -1 has no teeth in the flesh (4 points, to 0.84 mm masculine, before) and no lip in the teeth (1); `eye_tilt` +1 has no lid in the eyeball and no eyeball in the lid (3 and 1 on each side and frame before); the masculine brow at -1 has fewer head triangles crossing (`brow_arch_left` 21 to 7, `brow_height_left` 21 to 10, `brow_arch_right` 16 to 1, `brow_height_right` 10 to 3); at +1 the brows are as before. A full blink on every identity end leaves no ray bare, as before. Still not clean (as before, but for the brows): head triangles crossing at `eye_size` -1 (36 and 28, masculine), `brow_ridge` +1 (15 masculine, 6 feminine), `eye_tilt` +1 (5 and 4), `cheekbone` +1 (3 and 5, masculine), the masculine brows (above), and on the feminine frame `lower_lip_fullness` -1 (98) and `nose_length` -1 (8); `jaw_shift` at either end leaves one feminine lip vertex 0.27 mm in the teeth. These count every pair of the head's triangles that cross, also those touching at rest, which the sliders check leaves out.
- `cargo nextest run -p axcenity-anim -p axcenity-avatar`: 223 pass (`avatar`'s `a_baked_body_keeps_only_its_live_shapes` counts 146 shapes on the body now, 142 before the brows' raising shapes).

### Still short

1. **The sliders check is red on one line**: `sliders/frames/masculine/alone_eye_or_teeth_in_flesh` is 3 (at most 0): `eye_size_left` -1 (which it counted before this section too, with the eyes posed), `eye_tilt_left` +1 and `eye_tilt_right` +1, one eyeball point each, "0.29 to 0.43 mm in the lid". None is in the lid: each point is in the open eye, 2.4 to 4.7 mm from the lid, and outside the body by three rays' parity; `face_checks.py`, which tests by parity, finds no eyeball in a lid at these ends. `eye_metrics` takes the side of the nearest face, and a point nearest the lid's edge is nearest a vertex that several faces share, at the same distance to a thousandth of a millimetre: which face Blender returns decides the sign. The tilt's real defect (the lower lid's edge 0.25 mm in the eyeball, inside by parity) is gone; moving the lid's edge by tenths of a millimetre to take it out of the eyeball tipped the tie at two such vertices. The measure was not changed (no measure may be retuned to pass): counting a point only where the nearest face's side is its own (its distance to the body no more than its depth) would keep the real case and drop these.

The lead checked the three points on the merged build:
- Blender's nearest point for each lies on an edge that two faces share, not at a vertex.
- By that edge's pseudo-normal (the sum of the two faces' normals, which is unambiguous there), each point is outside.
- Four rays out of each point agree.

Signing by the nearest feature's pseudo-normal would be the principled fix: unlike a depth filter, it cannot drop a point that is really inside. The measure is unchanged; whether to change it is Paul's decision.
2. **Both brows at -1** hood the upper lid down to the lash line, the fold over the lid as a hooded eye's; the lashes are under it. Under the hard light a stippled patch of shadow shows at the inner corner (the renderer's shadow pattern, which shows at the lower lid's inner corner at rest too).
3. **The brows at +1** are the build before's, with what that had: small white marks on the fold with one brow parameter raised and a grey smear with both (seen in the engine pictures before and after alike).
4. `face_checks.py`'s identity entries that are not clean (Checks, above) are as they were, but for the masculine brows, where fewer head triangles cross at -1.
5. Right-side parameters are on the far side of the face cameras; they were judged from the numbers and from the left side's pictures (each side's shape is made by the same rule on its own side, and differs from the other's mirror image by what the mesh does: at most 0.5 mm for the brows, 0.3 mm for the ear's protrusion).

## R4: the high-value controls and the frames' faces

The refinement's R4 (plan 11.5; the hand-back, with every number and picture, is [refinement/r2/r4-face-controls.md](../refinement/r2/r4-face-controls.md); the sheets are in [refinement/r4/](../refinement/r4/)).

- **Fourteen parameters** from the face references' high-value list (`references/face/README.md` §2), each with its `about`, feature and level (detailed): `nose_tip_projection`, `nose_tip_rotation` (`nose_tip` stays for saved characters), `nose_width` (the whole nose, root to wings; `nostril_width` is the wings), `philtrum_length` (the mouth, its bag and the teeth down from a nose that mostly holds; `mouth_height` moves the nose's base with the mouth), and a pair each of `jaw_angle`, `upper_lid_fold`, `lower_lid`, `eye_corner` (the outer corner with the inner one held; `eye_tilt` turns the whole eye about its middle) and `earlobe`. The lid controls never move the rims, so a blink still closes; the lower lid's margin (the eye's opening) is therefore not a control. Chin height is `chin_length` (li-me), now labelled so. Ends and their millimetres against the references' SDs: the hand-back's table.
- **The feminine frame's face** (`face.FEMININE`, applied in `face.prepare` before the teeth are placed): the bundle's two heads were the same size, so the feminine face is made 5% smaller about the eyes (the eyes hold, so they are larger against the face, as the references' interpupillary-to-face ratios say), the mouth moving as one; lips a little fuller, cheeks fuller, a rounder forehead, brows 2 mm higher, a narrower chin and a jaw corner a little more sloped. The masculine frame and everything below the jaw are unchanged.
- **Checks**: `face_checks.py` holds R4's parameters at both ends alone, in a full blink and under every expression and viseme (`HELD_IDENTITY`, the group `identity_faces`); the older parameters' ends are still reported only. Since the review of R4 it also holds those ends to no new crossing of the head's skin triangles, alone and on top of every other face parameter's end (`identity_pairs`), counting the pairs within 4 mm at rest that `measure.py`'s sliders check leaves out, but for the closed lips' own contact. On top of an older end that already crosses on its own, a new crossing is reported, not held. The sliders check's result for R4 is [r4-sliders.json](r4-sliders.json).

## R4, second pass (wave 3): brows, the frames' faces, the presets

The hand-back, with every number and picture: [refinement/wave3/r4-face-2.md](../refinement/wave3/r4-face-2.md); sheets in [refinement/wave3/r4-face-2/](../refinement/wave3/r4-face-2/).

- **Brows** (`lashes.py`): the texture is drawn hair by hair (`brow_texture`: 6,000 tapering, curving hairs on a 1024 x 256 image, up at the inner end and lying over toward the tail, those from the band's lower half up and its upper half down; density sparse at the head, densest over the first half, thinning to the tail; single hairs at the edges), coloured by melanin (`hair_colour`: eumelanin 0.8, pheomelanin 0.15, each hair's own eumelanin varying; about sRGB (61, 46, 36), a dark brown) instead of a near-black fill. The card tapers to its tail (`BROW_LOOKS`), runs from 0.20 before the brow line's inner end to 1.38 along it (the brow length parameter's ends 1.06 and 1.60), and the feminine frame's card is its own: thinner (7.2 mm half-height at the head against 8.0), shorter (to 1.26), more arched (1.6 mm at 62% along) with its tail 1.8 mm up, lowered 2.5 mm as a whole; the skin's `brows_up` of R4 is gone. The build measures where the hair lies against each frame's eyes (`face.brows` in `Realistic.build.json`, F-29's measures).
- **The feminine face** (`face.FEMININE`): R4's 5% smaller about the eyes, and 5% narrower below the brows (`narrower`: the skin of the cheekbones and the jaw, and the ears, easing up the cranium; not within 2 to 5 cm of the mouth), the chin's front up by 5% of its depth under the lips' line (`shorter`; the mouth holds), the brow ridge back 0.5 mm, the nose's tip in 1 mm, the cheeks 1.5 mm higher and fuller, fuller lips, a rounder forehead. Wider reaches of these (the mouth moved with the face, the head's inside narrowed, the face 6.5% smaller, the chin 12% narrower) crossed the mouth's bag or folded the chin in `measure.py`'s sliders check. `Face.measures` (the build's `face.rest.frames`) measures both faces on the skin: zy-zy 0.929, se-me 0.913, go-go 0.881 and al-al 0.895 of the masculine.
- **The masculine face** (`face.MASCULINE`, new): glabella and the brows' inner ends 1.5 mm forward (the brow ridge's field, within 30 mm of the middle and off the eyes' regions): glabella stands 5.1 mm forward of sellion (3.7 before); the nasofrontal angle is 127.8 degrees (F-30: 138.5 +- 9.3 in men).
- **Nose**: `nose_tip_projection` 6.75 mm (2.5 SD). `nose_tip_rotation`: see review fixes 1 below.
- **Review fixes 1** (the lead's review of this pass):
  - *Brows*: the first pass's brows read as a faint thin line at face distance and were 26 mm short of F-29's length. The texture is now denser and more opaque in its body (9,000 hairs, each near-opaque over its first half; roots closer to the band's middle in a wider band; density held up to 75% along), fades to nothing at the card's border (the first pass drew hair up to the card's ends), and the masculine card is fuller (9.2 mm half-height at the head). The card now runs to 2.30 along the brow line (feminine 2.12; the length parameter's ends 1.98 and 2.52), carried on past the line's tail along its tangent with less of its bend and fall (`lashes.brow_path`), so it no longer droops toward the eye's outer corner. Measured as F-29 does (first hair to last along the lower edge): 59.3 mm masculine, 51.2 feminine (F-29 69.5 +- 5.8 and 61.4 +- 6.1); the hair's height at the head 9.7 and 7.4 mm.
  - *`nose_tip_rotation`*: the turn's weight eases in with the distance from subnasale, is averaged over the nose above the lip, and the turn is averaged again with subnasale held; 34 degrees. The thin blue-grey line under the columella at -1 (a grazing reflection on the lobule's lower edge) is within a level of the neutral face's (4.0 against 3.5, blue-channel ridge), but the tip now turns about 9.5 degrees about subnasale either way (14 in the first pass): beyond that the line comes back.
- **Presets** (`presets.py`): strengthened toward their sentences until each changes at least 3% of the face's pixels against the default face in front and in profile (`player`'s `face_preset_tests`).
