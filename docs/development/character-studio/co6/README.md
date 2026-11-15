# CO6: the realistic head and face

Milestone CO6 of the [Character Studio overhaul](../../../../plans/axcenity-engine-character-studio.md): what the realistic base body's face now is, how it is made, how to rebuild and check it, what was measured and what still falls short. The package is [assets/characters/realistic/](../../../../assets/characters/realistic/README.md).

## What the face has

| | |
|---|---|
| Head | The body's own head (no graft), vertex order the bundle's. Three repairs at rest (`face.prepare`): the feminine frame's lids are the masculine frame's carried to the feminine eye (the feminine opening sat 1.5 mm higher on the eyeball and bared the whole iris: the stare); the lids' lining is lifted off the eyeball; the lips are closed (they rested 1.9 to 2.5 mm apart). |
| Expressions | 44 blend shapes on `Body`, named as the facial profile's controls (`face.EXPRESSIONS`); the four `Eye_Look_*` controls turn the eye bones. `Jaw_Open`, `Jaw_Left`, `Jaw_Right` are the jaw bone's turn plus a shape that puts right what the turn gets wrong. The three `Tongue_*` shapes are empty on the body and move the tongue (a part follows the body's shape of the same name). |
| Identity | 45 shapes, one per face parameter end (`faceparams.py`): 46 parameters in nine groups (eyes, brows, nose, cheeks, jaw, chin, lips, forehead, ears). Paired features have a parameter a side that names the other as its `mirror`; `nose_tilt` and `jaw_shift` push the middle off centre. Eye parameters also move or scale the eye bones. `pupil_size` drives `Pupil_Size` on `Eyes`. |
| `Eyes` | Per eye: `Sclera` (the white, textured), `Iris` (a dish set back inside the limbus, with the pupil), `Cornea` (a clear cap 1.2 mm proud of the white, alpha-blended). 2,256 triangles. |
| `EyeShell` | `EyeOcclusion` (a film between the lids, dark at their edges) and `TearLine` (the wet line), on the head, following the lids by shapes. 320 triangles. |
| `Lashes`, `Brows` | Cards. Lashes are rooted on the lid's outer edge and turn with it about the eyeball (alpha cut-out, 160 triangles); brows lie on the skin and follow it (alpha-blended, 112 triangles). |
| `Teeth` | Re-seated: the teeth meet 1.6 mm under the lips' line just behind the upper lip (CO4 had them 13 mm lower and 12 mm further back, behind the lower lip, so an open mouth showed gum and no upper teeth). The bundle's lower shell and floor, which tore between head and jaw, are gone; a tongue (294 triangles) is made with `Tongue_Out`, `Tongue_Up`, `Tongue_Down`. 3,893 triangles. |

Materials and shading in the manifest: `Skin` skin; `Sclera`, `Iris`, `Cornea` eye; `Lashes`, `Brows` hair; `Mouth`, `Teeth`, `Gums`, `Tongue`, `EyeOcclusion`, `TearLine` standard.

## How the shapes are made

**Procedural, on the head's own loops, with the eyeball and the teeth as things the skin must stay off.** Decided after a spike of both kinds (below).

- **Lids** (`lids.py`): every lid vertex turns about a line through the eyeball's centre, by its loop; both rims end on one closing line, so closing is exact, and every vertex keeps its height over the eyeball and the cornea. A shape moves a vertex in a straight line and a lid's path is an arc, so the closed lid is raised where it travels furthest by enough that half way the straight path clears the white of the eye.
- **Mouth, brows, cheeks, nose** (`face.py`): offsets along the muscle's pull with a falloff, the skin then put back on the resting surface (it slides over what is under it), the lips left free. Each side of a pair is a share of one shape made for both sides.
- **Off the teeth**: every mouth shape is finished by moving lip and lining vertices off the teeth, and faces off the teeth's points, in the jaw pose the control gives, on both frames.
- **Following parts**: `EyeShell` is a mix of lid vertices, its height shrinking as the lids meet; lashes turn with their roots; brows are bound to skin triangles; the teeth take `Frame_Feminine`, `Jaw_Forward` and `Face_Mouth_Height`.

### The spike

Five hard shapes each way, masculine frame, clay.

- **MakeHuman's CC0 expression units**, carried by the CO5 spike's registration (nearest point, no face landmarks): [spike/s_mh_mouth.png](spike/s_mh_mouth.png), [spike/s_mh_eyes.png](spike/s_mh_eyes.png). The smile (`mouth-corner-puller`) came over as a good broad form. Everything with an edge failed: the eye closure tore the lids and left them open (36 vertices moved, 7.6 mm against the 11 mm needed), `mouth-open` tore the lips (the two lips' vertices were matched to each other's), the brow moved 5 mm on 67 vertices. The set has 34 units with no left and right for the mouth and no tongue.
- **Procedural**: blink closed exactly on both frames at the first try; smile, jaw open with lips, brow raise and the narrow and pucker visemes read as what they are.

So: procedural for all expression shapes, because exact lid and lip edges are the criterion and the transfer can't give them without ring landmarks that MakeHuman's mesh would need found by hand. Identity shapes are procedural too; MakeHuman's face detail targets (nose, chin, cheeks) remain a possible source of broader forms later.

Licence record for the spike: eight files read from `makehumancommunity/makehuman` at commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482` (the CO5 spike's clone), `makehuman/data/targets/expression/units/caucasian/`, each stating on lines 3 and 4 "This asset was explicitly released as CC0 in september 2020". No code, rig or other file was read. Nothing of MakeHuman's is in the package: the two sheets above are the only output. SHA-256:

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

## Rebuilding and checking

`PY` is a Python with the `bpy` module, from the repository's root:

```text
PY -I assets/characters/realistic/build.py --out assets/characters/realistic
python3 -I assets/characters/catalogue.py
python3 -I assets/characters/benchmarks/make.py
PY -I assets/characters/humanoid/qa/face_checks.py --package realistic --out docs/development/character-studio/co6/face_checks.json --check
PY -I assets/characters/humanoid/qa/render_face.py --package realistic --out docs/development/character-studio/co6
cargo nextest run -p axcenity-anim -p axcenity-avatar --no-fail-fast
```

Tests: `crates/anim/tests/realistic_face.rs` (three: `check_face` finds nothing; every standard control moves a vertex or a bone; the parts carry the shapes they follow). Red runs before the work: [red-run-anim.txt](red-run-anim.txt) (3 failed), [red-run-face-checks.txt](red-run-face-checks.txt) (56 failures, a blink that left the eye 9.4 to 10 mm open). Now: [face-checks-run.txt](face-checks-run.txt), [face_checks.json](face_checks.json).

## Measured (both frames, `face_checks.py`)

| Measure | Masculine | Feminine |
|---|---|---|
| Eye open at rest (bare eyeball height, left and right) | 9.53, 9.42 mm | 9.40, 9.30 mm |
| Full `Blink` and `Eye_Closed`: bare rays, open height | 0, 0 mm | 0, 0 mm |
| Lid in the opaque eyeball, eyeball in a lid: 48 controls, 12 visemes, 15 expressions, blink at 0.25, 0.5, 0.75 | 0 in all but one: 2 eyeball vertices in the lid at blink 0.25 | 0 in all but one: 1 at blink 0.25 |
| Lid in the clear cornea (reported, not held to 0) | 6 and 9 vertices to 0.54 mm at blink 0.25 and 0.5; 3 at `Eye_Look_Down`; 5 in Sleepy | 6 and 8 to 0.51 mm; 3; 2 |
| Teeth in the flesh, lips in the teeth, at rest | 0, 0 | 0, 0 |
| The same, 48 controls | 1 pose: `Mouth_Left`, 1 tooth point 0.65 mm | 2 poses: `Jaw_Left` 2 points to 0.9 mm, `Jaw_Right` 4 points to 1.84 mm and 1 lip vertex 0.24 mm |
| The same, 12 visemes | 1 pose: TH, 1 lip vertex 0.8 mm | 0 |
| The same, 15 expressions | 1 pose: Disgusted, 1 tooth point 1.11 mm and 1 lip vertex 0.94 mm | 1 pose: Angry, 1 lip vertex 0.66 mm |
| Tongue in the flesh; lining through the skin | 0; 0 | 0; 0 |
| Head triangle pairs that cross at rest | 3 (3 in CO4's file) | 104, all at the mouth (42 in CO4's file by this measure) |
| Most new crossing pairs in a control | 162, `Eye_Closed` (the closed rims lie in each other) | 160 |
| Identity ends (92 poses): blink left open | `eye_size` +1: 0.11 and 0.23 mm | none |
| Identity ends: teeth in the flesh | `mouth_height` -1: 13 points to 1.4 mm; 2 more poses | `mouth_height` -1: 13 to 1.54 mm, 3 lip vertices to 2.85 mm; 3 more poses |

`--check` therefore still exits 1 with **8 failures**, all one to four points under 2 mm between the teeth and the lips' inner side, none through a lip to the outside. They are not accepted: see below. An earlier attempt to remove them by sharing corrections among the shapes of each built-in mix made single controls worse and was taken out.

Size: `RealisticParts.glb` 10.2 MB before, 18.6 MB now. `Body` and each of its two simpler copies carry 91 shapes (1 frame, 44 expressions, 45 identity, `Pupil_Size`); `EyeShell` 56, `Brows` 38, `Lashes` 26, `Teeth` 6, `Eyes` 1.

## The sheets, and what is wrong in them

In-engine (the player's renderer; colour is judged here): [engine/](engine/). Blender (`render_face.py`; its lights wash the skin pale): the `face_*.png` here.

- **engine/faces-close.png, engine/before-after-feminine.png**: both frames have brows, a lash line and closed lips; the feminine upper lid now covers the top of the iris. Wrong: no highlight on the eye (the cornea's gloss is lost in alpha blending); the lash line with the occlusion film reads as a dark outline, heaviest under the eye; the whites are flat blue-grey; brows are thin strokes on a card and sit a little high; no caruncle; still bald.
- **face_rest.png**: eyes and mouth close up. The feminine inner corner keeps a small dark pinch above it (the bundle's cage). Lashes are sparse cards, readable as lashes from the front, as flat fans from the side.
- **face_blink.png**: closed on both frames. Wrong: the upper lid's skin stretches four times its length and its texture streaks; a dark pit at the inner corner and a short crease at the outer one.
- **face_controls_eyes.png**: every control reads. Brow shapes are small; `Brow_Inward` and `Nose_Sneer` are hard to tell from rest at this size.
- **face_controls_mouth.png**: smile, frown, wide, narrow, pucker, open, jaw and tongue read. Wrong: where the lips are held off the teeth the lip's edge is uneven (`Mouth_Open`, `Upper_Lip_Raise`, `Lower_Lip_Lower`); the smile is a corner pull with a mild cheek, no crease deep enough to see at the nose's wing; `Cheek_Puff` is slight.
- **face_visemes.png, face_expressions.png**: all 12 and all 15 on both frames; the visemes are distinct mouths. Wrong: with the teeth showing (AI, E, KG, O) the lips' edge is serrated tooth by tooth, with dark gaps between lip and teeth; FV raises the upper lip but the lower lip doesn't tuck under the upper teeth; TH shows hardly any tongue. The quiet expressions (Confused, Determined, Worried) differ little.
- **face_identity.png**: every parameter's two ends, 184 small tiles. At that size most ends can't be told from rest by eye: that each parameter moves geometry rests on the build's measure (`face.largest_move_mm` in `Realistic.build.json`: 1.25 to 8 mm at an end), not on this sheet. Whether the ends stay human needs larger pictures of the benchmark characters `make.py` now writes for each.
- **face_clay.png**: rendered, not reviewed tile by tile.
- **face_sidelit.png**: meant as the hard-light check, and not good enough for one: the studio's own light still fills it and the cornea mirrors the key. The engine's `hard` preset at rest is the only true side light here.
- A dark patch under the jaw in jaw-open tiles is the shadow banded by the 256-colour PNG, not geometry.

## What still falls short

1. The 8 check failures above (teeth and lip lining within 2 mm in `Mouth_Left`, the feminine `Jaw_Left` and `Jaw_Right`, the TH viseme, Disgusted, Angry).
2. A half blink dips the lid into the clear cornea by up to 0.54 mm (straight-line shapes; an in-between shape or a corrective would end it), and 1 to 2 eyeball vertices sit in the lid at a quarter blink.
3. Eyes: no highlight, no caruncle, flat whites, a heavy lash line. Eye shading is CO7's.
4. The upper lid's texture stretches when closed; no wrinkles anywhere (CO17).
5. Lips: uneven edges in parted-lip shapes; the feminine lips cross each other slightly where they meet (104 pairs).
6. Lashes and brows are single cards: thin from the side, no volume.
7. `mouth_height` at -1 puts 13 points of the teeth in the lining; `eye_size` at +1 leaves a blink 0.1 to 0.2 mm open.
8. `Blink` and `Eye_Closed` are separate shapes that add: both at 1 over-close the lid. The runtime should cap their sum.
9. The face was seen in the engine only at rest: the capture can't pose a face control.
