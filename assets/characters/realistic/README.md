# Realistic base body

The `axcenity/realistic` base model (Character Studio overhaul, CO4): a body with human proportions, a masculine and a feminine frame, eyes, teeth, a baked skin texture set, twist and helper bones, body regions and two simpler copies. Since CO6 it has a face: expression shapes for the standard facial controls, face identity parameters, layered eyes, a tongue, lashes and brows (**[the face](../../../docs/development/character-studio/co6/README.md)**). Its first items are underwear cut from the body itself (below); no hair or other clothing yet.

| File | What it is |
| --- | --- |
| `RealisticParts.glb` | Skeleton `AxcenityRealistic` (72 bones) with `Body`, `Body_LOD1`, `Body_LOD2`, `Eyes`, `Teeth`, `Teeth_LOD1`, `Teeth_LOD2`, `EyeShell`, `Lashes`, `Brows`, `LipSeal`, and the underwear `Underwear_BoxerBriefs`, `Underwear_Briefs`, `Underwear_Bra`. The skin maps are inside it. |
| `items/*.item.json` | The underwear's item manifests (written by `manifests.py`). |
| `Realistic.base.json` | The base model manifest (written by `manifests.py`), with every parameter's `about`, `feature` and `level`, the 23 `features`, whose anchors `anchors.py` measures on `RealisticParts.glb` (R1), and the 19 `handles` Character Studio's viewport draws for the nose, eyes, jaw, mouth, shoulders, waist and hips, whose skin corners, axes and travels `handles.py` measures (R2). `manifests.py` is plain Python with numpy and runs without Blender. |
| `presets/*.preset.json` | The shipped presets: 14 body (seven per body type), 9 face, 6 character (written by `presets.py`; R1). |
| `Realistic.build.json` | What the build measured and decided: frame bone offsets, twist and helper rules, regions, texel density, how the eyes and teeth were fitted. The QA scripts read it. |
| `RealisticRegions.png` | The body regions as an index map (below). |
| `source/` | The vendored source meshes and `extract.py`, which made them; `body_shapes.npz`, MakeHuman's bodies on this topology (CO5, below). |
| `qa/thresholds.json` | The limits `measure.py --package realistic --check` holds the build to. |

## Building

Needs the `bpy` module (Python 3.13 for bpy 5.x), NumPy and Pillow. About 40 seconds on a 16-core CPU, three and a half minutes on a 6-core laptop. Blender itself works too (`blender -b --factory-startup --python-exit-code 1 --python build.py -- --out DIR`); its own Python has NumPy but not Pillow, so install Pillow for Python 3.13 into a folder and start Blender with `--python-use-system-env` and `PYTHONPATH` naming it.

```bash
python -I assets/characters/realistic/build.py --out assets/characters/realistic
python -I assets/characters/catalogue.py
```

`--texture-size 4096` bakes the hero texture set (2048 is committed); `--skip-bakes` leaves the maps out for quick checks of the mesh and rig. The build is one pass through `body.py` (load, re-pose, stand), `uvs.py`, `rims.py` (twice the vertices round the lids), `skin.py` (bakes), `face.py` (the face put right at rest), `makeup.py` (the makeup and marking masks, measured on that face: R5, ADR-0042), `eyes.py`, `parts.py` and `teeth.py` (eyes, teeth, gums, tongue), `rig.py` (skeleton, weights), `shapes.py` and `correctives.py` (the body's shapes), `face.py` again (expression and identity shapes; `EyeShell`, `Lashes`, `Brows` from `eyes.py` and `lashes.py`), `underwear.py`, `regions.py`, `lods.py` and `manifests.py` (with `faceparams.py`). It reuses the stylized package's skeleton builder and manifest helpers (`common.stylized`), so bone names, slots and sockets stay the same.

After a build, check it and render its sheets (see `../humanoid/qa/README.md`):

```bash
python -I assets/characters/humanoid/qa/measure.py --package realistic --out docs/development/character-studio/co4/measurements.json --check assets/characters/realistic/qa/thresholds.json
python -I assets/characters/humanoid/qa/render_baseline.py --package realistic --out docs/development/character-studio/co4
```

## Source and licence

Everything is built from Blender Studio's **Human Base Meshes** bundle, v1.4.1, released under **CC0 1.0** (public domain dedication; no attribution required, given here anyway):

- <https://download.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip>
- SHA-256 of the zip: `811f43accbb31a88266d932f8f5563b2d13586fca0ba2693aad1f5fe582b3515`

`source/extract.py` (run once, against the bundle's `.blend`) wrote the files in `source/`; the build never needs the bundle. What was taken:

| Vendored file | From the bundle | Used for |
| --- | --- | --- |
| `body_masculine.obj`, `body_feminine.obj` | `GEO-body_male_realistic`, `GEO-body_female_realistic`, base level (10,582 vertices each, one topology by vertex index) | The body and its `Frame_Feminine` shape. Sculpt face sets are kept as materials `fs_<id>`. |
| `sculpt_masculine.npz`, `sculpt_feminine.npz` | The same objects' multiresolution level 3 (677,442 vertices), as a float16 displacement from the base subdivided three times | Baked into the normal and occlusion maps. |
| `eyes.obj` | `GEO-head_animation_realistic` sclera and iris, left | The eyeballs. |
| `teeth.obj` | `Jaw - Realistic` | Teeth and gums (its tongue and the floor of its mouth aren't used: the build makes a tongue). |
| `head_animation.obj` | `GEO-head_animation_realistic` | Reference only (see the head, below). |

The skin's colour, roughness and fine grain are generated by `skin.py` (the grain is pores, a pit at each of a cell pattern's points, over gentle wrinkling and fine lines; until the R5 makeup branch it was grooves along the cells' edges, which read as cracked skin), and its makeup and marking masks by `makeup.py` (foundation, blush, eyeshadow, eyeliner, lip colour, brow fill, a scar, and seeded freckles and moles: `docs/rendering/skin.md`); no photographs or scans are used.

## The body

- **Frames.** `Body` is the masculine frame; `Frame_Feminine` moves it to the feminine one (exported with its normals). Both frames are scaled by one factor, so the masculine stands 1.75 m and the feminine 1.694 m, 7.7 and 7.8 heads tall. The manifest's `frame` parameter also moves the bones (`bones` offsets from the two frames' fitted joints) and scales the eyes; `Teeth` carries its own `Frame_Feminine`.
- **Pose.** The bundle's arms hang about 70 degrees below level; `body.repose` turns them to the contract's 45 degree A-pose and the legs to 2 degrees out, then takes out the skinning's damage with a corrective smooth.
- **Repairs to the source.** The feminine cage folds through itself inside both eye sockets; `body.unfold` relaxes those vertices (65 crossing face pairs to 0). The fold under each feminine breast is sharper than 25 mm quads can carry (it stair-stepped); `body.soften_folds` relaxes it to 24 degrees between faces (CO5; the masculine frame is untouched). The mouth's bag is widened round the teeth (`parts.make_room`).
- **More vertices round the lids.** The bundle's lid edge is a ring of 20 vertices, 8 of them on the upper lid, and at 4K the eye's opening read as a polygon. `rims.split` cuts the quads of the strips that run out from that ring in two, along the strip: back over five loops of the socket's lining and out over seven loops of the lid's skin, past the fold (two strips under the inner corner stop at the fifth loop, where the loops run into the face's grid). Each of those loops has twice the vertices: 512 new vertices and 1,024 more triangles for both eyes. Where a strip stops, the quad beyond it is a fan of three triangles from the one new vertex on its edge (on the lining behind the eyeball, under the brow and on the cheek). A new vertex lies on the cubic through the four vertices of its loop round it, not at the middle of its edge, as a fixed mix of those four: the same mix gives its place on both frames, in the bundle's pose and in the vendored body shapes, so the bundle's vertices keep their indices and everything stored by them still fits (`Rims.grow`; the mirror pairing is `Rims.mirror`). The eyes are fitted, the texture coordinates laid out and the sculpts subdivided on the bundle's own topology; everything after that is on the finer one.
- **Counts.** `Body` 22,184 triangles and 11,094 vertices (10,964 quads and 256 triangles before export; the bundle's body is 21,160 triangles and 10,582 vertices), `Eyes` 2,256, `Teeth` 9,022 (teeth 7,094, gums 1,496, tongue 432), `Teeth_LOD1` 8,424, `Teeth_LOD2` 2,411.

## The head (for CO6)

The body's own head is used as it is; the bundle's separate animation head was **not** grafted on. The bundle's body has 3,710 faces (3,732 vertices) in its head region against the animation head's 3,234 faces (3,242 vertices, which include its neck and shoulders), and already has the loops an animated face needs, so a graft would have cost a seam at the neck and a second set of texture coordinates for no gain. `source/head_animation.obj` is kept for comparison. See `docs/development/character-studio/co4/face_topology.png`.

Face sets in the head (`common.py` names them), as built, with the lids' strips cut (`rims.py`; in the bundle set 17 has 2,026 faces and 2,131 vertices, a socket 225 and 236). All quads but the fans at the strips' ends (116 triangles in set 17, 60 in each socket) and the bundle's own 20 triangles (in the mouth and the ears):

| Set | Faces | Vertices | What |
| --- | --- | --- | --- |
| 17 | 2,376 | 2,443 | Skull, face with the nose **and the upper lip**, neck |
| 22 | 494 | 539 | Lower lip, chin, the skin round the mouth's corners |
| 7 | 190 | 213 | Inside the mouth, upper: roof and the upper lip's inner side |
| 8 | 100 | 119 | Inside the mouth, lower: floor and the lower lip's inner side |
| 2, 3 | 365 each | 356 each | Left and right eye socket (the lining behind each eyeball) |
| 4, 5 | 225 each | 236 each | Left and right ear |

The head's sets together: 4,340 faces, 4,244 vertices. Above the head joint: 2,938 vertices, 5,746 triangles.

- **Eyelids.** Each socket set meets the face along a closed ring of 40 vertices (the lid edge; 20 in the bundle); concentric loops run round it out to the brow and cheek before they merge into the face's grid. Lid shapes should move the ring and the first two loops; the socket lining behind follows.
- **Lips.** The upper lip's edge (sets 17 and 7) is a line of 23 vertices from corner to corner; the lower lip's inner edge (sets 22 and 8) has 21. Loops run round the mouth concentrically (the lip, the lip border, then the muzzle border of 61 vertices between sets 17 and 22). At rest the lips are about 2 mm apart.
- **Nasolabial.** The muzzle border (sets 17 and 22) and the loop outside it run from the nose's wing round the mouth's corner to the chin, which is the fold's line: a smile shape can crease along it without cutting across quads.
- **Jaw.** The `Jaw` bone carries the chin, the lower teeth and the tongue, and its hold on the skin is one smooth field (`rig._jaw`): all of the lower lip's middle and none of the upper lip's, 0.30 at the corners, and between them `0.30 * u ** 2.5` along the upper lip and `1 - 0.70 * u ** 2.5` along the lower (`u` from the middle 0 to a corner 1), so the open mouth is a shallow arch over a U that narrows downward; over the cheeks it runs from none to all across 0.38 radians about the hinge. The lips' inner sides take the same weights as their outer sides; deeper, the roof of the mouth is the head's and the floor the jaw's. The bone only turns: `Jaw_Open`'s shape adds the jaw's glide forward (19 mm, on `Body` and on `Teeth`), stands the lower lip over the lower teeth (2.5 mm higher again between its middle and each corner, where the lower back teeth would otherwise show over its rim), draws the upper lip up off the upper ones and the corners in, brings the cheek's lining in against the lower back teeth behind each corner, and the tongue's share of it lays the tongue low and level in the floor of the mouth. At rest the lips are sealed (`mouth.seal`: 0.3 mm apart from side to side). No face vertex above the head joint has any `Neck` weight.

## Skeleton, twist and helper bones

The 56 bones of the humanoid convention with the same names, plus 16 the build adds (`skeleton.added_bones` in the manifest). None of them is animated; the engine turns them from the bones they read (`crates/anim/src/twist.rs`, which the QA scripts' `Character.drive` copies):

| Bone (Left and Right) | Parent | Rule |
| --- | --- | --- |
| `UpperArmTwist`, `UpperLegTwist` | the limb's upper bone | Turns back by half of its parent's own twist about its length, so the shoulder and hip keep their shape while the lower half follows. |
| `LowerArmTwist`, `LowerLegTwist` | the limb's lower bone | Turns by half of the hand's or foot's twist about the limb's length. |
| `ShoulderHelper`, `ElbowHelper`, `HipHelper`, `KneeHelper` | the joint's parent | Sits on the joint and turns half as far as the bone it reads (`UpperArm`, `LowerArm`, `UpperLeg`, `LowerLeg`), which holds the joint's volume when it bends. |

At most four weights per vertex on every mesh. A clip made for the stylized humanoid is **retargeted** onto this skeleton (its bones are other lengths): bound by bone name it would bring the stylized bone offsets with it (last tile of `clips.png`), so `anim` plays such a take through humanoid roles (`clip.rs`, `reshapes`).

## Texture coordinates and skin maps

One set of texture coordinates for the skin (`TEXCOORD_0`), 26 islands: the head is one island cut up the back, the torso is cut down the sides, arms and legs along their inner sides; hands, feet, ears, eye sockets and the mouth's interior keep the bundle's islands. Texel density is weighted (face 2.2, hands 1.15, hidden linings 0.4) and is about 10.9 texels per centimetre at 2048.

Materials: `Skin` (the manifest marks it `skin`, and so does the file: its glTF `extras` say `"axcenity_shading": "skin"`, which the engine draws with its Skin shading model, `docs/rendering/skin.md`), `Mouth` (the mouth's interior, on the skin's maps), `Sclera` and `Iris` (`eye`), `Teeth`, `Gums`, `Tongue` (below). The skin's maps, embedded in the file:

- **Albedo** (JPEG): one tone with flush on the cheeks, nose, ears, chin, knuckles, elbows and knees, both lips, paler palms and soles, nails.
- **Normal**, two of them (PNG): the sculpt's forms above the base mesh plus grain. One map serves both frames: where the two frames' bakes agree it is their mean, where they differ it leans to the flatter. `RealisticSkin_NormalDefined` is that map (lean and muscular: abdominal lines, muscle separation); `RealisticSkin_Normal`, the material's own, is a smooth copy with the sculpt's forms flattened (90%) over soft tissue (`skin.soft_tissue`: the trunk, shoulders, arms and legs, but not the head, hands, feet, knees, elbows or nipples, nor deep creases such as under a breast; and not within 2 cm of the navel, fading to 3.5 cm, which `skin.navel` finds as the belly's midline vertex lying furthest in, because the occlusion keeps only its deep middle and the smooth map once flattened it away). `Realistic.build.json` records where it is (`textures.navel_uv`), and an `assets` test checks the two maps agree there. `skin.mark` adds the defined map to the file after export (`extras.axcenity_defined_normal`); the engine blends the two by `definition × (1 − softness)`, which the manifest drives from `muscle` and `fat` (`manifests.SKIN_DRIVERS`), so soft and heavy bodies show no abdominal lines (R5, OPEN-07). The grain's cells are 5 texels across and its bump 0.09 texels high (`skin.GRAIN_*`; 2.6 texels before R5, half as steep now), so it reads as skin at a metre and as smooth at three.
- **Occlusion, roughness, metalness** (JPEG, red, green, blue): cavity occlusion baked on the sculpt; roughness lower on the forehead, nose and lips, higher on elbows, knees and heels.

## The inside of the mouth

The engine lights a mouth's inside like the face, so its shade is baked (`teeth.py`, `skin._mouth_shade`; the reasons and the pictures are in [the face's notes](../../../docs/development/character-studio/co6/README.md)):

- **`Teeth`, `Gums`, `Tongue`** each have a 64 px palette image (`RealisticTeeth`, `RealisticGums`, `RealisticTongue`: about 2 kB each): across it the material's colours (a tooth in the mouth's shade, which the lower second premolars and molars are so that they are dark red in the corners of an open mouth and not black, then a tooth's neck, enamel and thin edge; the gum's edge, the firm gum and the redder lining; the tongue's groove and back), up it from no light to full light. A vertex's texture coordinates are its place in it, so `TEXCOORD_0` on `Teeth` is a colour and a shade per vertex, not an unwrap. One occlusion image (`RealisticMouthOcclusion`) is laid out alike. Roughness is a number per material (0.16, 0.34, 0.46). A real texture for the teeth would need the teeth unwrapped first.
- **`Mouth`** (the lining, on `Body`) is darkened in the skin's albedo and occlusion by how far it is from the lips' line, and is matt for the first 5 mm inside the lips (roughness in the skin's map), so it doesn't mirror the sky between closed lips.
- **`LipSeal`** (48 triangles, material `LipSeal`: the gums' palette at the shade of the lining just inside the lips) is the depth the lips touch over: a flap hanging 2.2 mm from the upper lip's underside and one standing 1.2 mm up from the lower lip's top, each riding its own lip a few millimetres behind its edge, behind the other lip while the mouth is closed. Lips parted by up to about 1.5 mm show the lining's dark red between them, not the teeth behind. `Jaw_Open`, `Mouth_Open`, `Upper_Lip_Raise`, `Lower_Lip_Lower`, `Nose_Sneer` and `Mouth_Narrow` draw each flap back along its lip's middle and on into the lip's flesh (a shape moves a vertex in a straight line, so a flap cannot stop when it is gone): the lip controls have a flap out of sight by 3 mm of parting, the jaw by 5 mm. `face_checks.py` holds that no vertex of it is in view with the lips more than 6 mm apart.
- **Even.** The mouth's resting form, the jaw's hold on it and every mouth shape are mirror images across the middle (`face.prepare`, `Face.evened`; the bundle's lips are not), and `face_checks.py` holds that to 0.1 mm.
- **Size and place.** The jaw is 0.85 of the bundle's (adult teeth: the sources are in the face's notes) and stands 2.7 mm behind the upper lip's inner side on both frames.
- **`Teeth_LOD1`, `Teeth_LOD2`** are made by the build (`teeth.LEVELS`). The engine draws the first whenever the part is under 30% of the view's height, which is every picture of a whole face, so its upper front teeth and its gums' edges are the full object's.

## Body regions

Fifteen regions, so clothing can hide the skin it covers (ADR-0035): `head`, `neck`, `torso`, `waist`, `hips`, `upper_arms_top`, `upper_arms_low`, `forearms`, `hands`, `thighs_top`, `thighs_low`, `shins_top`, `shins_low`, `feet` and `upper_arms_mid`, with `map_index` 1 to 15 in the manifest (15 is the last the second texture coordinates hold). A triangle belongs to one region. Two cuts follow where garments end (CO9 part 2, review fixes 1): the `neck` takes the trunk's skin round the neck above its joint (within `NECK_ROUND`, 8.8 cm, of its line: the skin a crew neck leaves bare), so a top can cover the `torso` whole; and the upper arm has three bands, `upper_arms_top` to halfway along the bone, `upper_arms_mid` the faces a short sleeve keeps covered (every corner within `SLEEVE_END + SLEEVE_TILT × facing in`, 0.8 of the bone on the sides, 0.9 underneath, 0.7 on top), and `upper_arms_low` to the elbow. The imported test tee hides `torso`, `waist`, `upper_arms_top` and `upper_arms_mid`. The index is stored twice:

- `RealisticRegions.png`: 1024 px, 8-bit grey, laid out by `TEXCOORD_0`; a texel's value is its region's index (0 outside the islands).
- `TEXCOORD_1` on `Body` and its copies: every corner of a triangle has `u = (index + 0.5) / 16`, `v = 0.5`.

## Simpler copies

The engine reads `_LOD1` and `_LOD2` objects in a file as a model's levels of detail and pairs them with the full primitives by material name (`docs/rendering/models.md`). `Body_LOD1` (10,500 triangles) holds back the face, hands and joint rings; `Body_LOD2` (5,000) is a plain error-driven collapse. Both keep the skeleton, four weights per vertex, texture coordinates, materials, regions and `Frame_Feminine`. `Teeth` has its own two (above); `Eyes` has none.

## Body shapes (CO5)

`Body` and both of its copies carry, besides the face's shapes, `Frame_Feminine`, 51 for muscle and fat (`shapes.py`: 48 from CO5 and 3 from R3), the hips' narrowing per frame (`<Frame>_Hips_Narrow`, R3's wider ranges; "Proportions" below) and 6 pose-driven correctives (`correctives.py`). All but the correctives are **identity** shapes: a character's description sets them as live weights on the shared mesh, and a bake is optional (ADR-0041). `RealisticParts.glb` is 42.4 MB (37.4 MB before R4's face shapes and R5's defined normal map; 36.4 MB before R3's three fat shapes and two narrowing shapes).

**Source.** MakeHuman's CC0 targets (base mesh, 24 macro targets and, since R3, 8 regional and body-shape targets at the pinned commit in `../transfer/makehuman.sha256`; licence record in `../transfer/shapes/LICENCE-RECORD.md`), carried onto this topology by `../transfer/` and vendored as `source/body_shapes.npz` (0.55 MB: 16 sculpted bodies as offsets, 8 per frame, and 8 regional targets, in the bundle's pose). The build needs neither MakeHuman's files nor the network. To regenerate the file, see `docs/development/character-studio/co5/README.md` (its "Regenerating the vendored shapes" block, which carries R3's `--pack-extras` list).

**The sliders.** MakeHuman sculpted nine bodies per gender, muscle (min, average, max) by weight (min, average, max), and mixes them bilinearly. `muscle` and `fat` walk a part of that grid from where each frame's own body sits on it (`shapes.PLACE`), so the body is always `base + F(m, w) - F(m0, w0)` and 0 on both sliders is the bundle's body unchanged:

| Frame | The base sits at (muscle, weight) | `muscle` -1 to 1 reaches | `fat` -1 to 1 reaches |
| --- | --- | --- | --- |
| Masculine | 0.45, 0.30 | -0.6 to 1.0 | -0.75 to 0.85 |
| Feminine | 0, 0 | -0.7 to 0.9 | -0.75 to 0.85 |

- The masculine body is placed above MakeHuman's average because the bundle's is athletic: its slim and slight ends take off muscle and mass the base starts with, so slim reads slim. The placement was chosen from the renders; a fit of five girths and the volume to MakeHuman's own grid says (0.15, 0.80), which reads the base's broader frame as weight and makes the slim end gaunt.
- The ends are inside MakeHuman's square (it runs -1 to 1) where its bodies are caricatures: its gaunt bodies below -0.75 weight, its obese and comic-book bodies above 0.85.
- `fat` is MakeHuman's weight axis: with little muscle it is fat, with much it is bulk. At this body's muscle MakeHuman's heaviest body is bulk (7 mm deeper at the belly at average muscle, 108 mm at its least), so `fat` 1 read as athletic (the refinement baseline's H6).
- In the manifest every shape is a corner with the frame: per frame four shapes driven by one slider (`<Frame>_Muscle_Up`, `_Muscle_Down`, `_Fat_Up`, `_Fat_Down`) and four by the product of two (`<Frame>_Muscle_Up_Fat_Up`, …). The spans are in `shapes.corners()`.

**Fat that is fat (R3).** Two more shapes per body, both MakeHuman's own CC0 targets carried by the same transfer, mixed by fixed weights (`shapes.SOFT`, `shapes.FLAT`), nothing sculpted by script:

- `<Frame>_Fat_Soft`, driven by `fat` from 0 to 1 (all of it from `muscle` 0 up, half at `muscle` -1, whose own heavy corner already has a belly). Placed by frame as people carry fat: masculine `bodyshapes-elvs-man-thyroid` 0.4 (the whole body), `-man-adrenal` 0.3 (belly and flanks) and `neck-double-incr` 0.6 (under the chin); feminine `bodyshapes-elvs-fem-ovary` 0.5 (hips, thighs, buttocks), `-fem-liver` 0.35 (lower belly), both `upperarm-fat-incr` 0.7 and `neck-double-incr` 0.5. At `fat` 1, against the default (measured on the built model; before R3 in brackets): the masculine belly comes 62 mm forward (8), the waist widens 72 mm (21), the hips 59 (13), a thigh 24 (8), the buttocks 12 mm back (5); the feminine belly 37 mm (12), waist 75 (27), hips 65 (21), a thigh 45 (14), buttocks 29 (5). Fat under the chin and along the jaw moves up to 20 mm (masculine) and 9 mm (feminine). It is a fat shape, so the bony places hold it back like the others.
- `Masculine_Belly_Flat`, the product of `muscle` and `fat` both going to -1: `stomach-pregnant-decr` at 1, which takes in the slack lower belly of MakeHuman's least-muscled body, so the slight body is lean, not small-bellied (its belly sits 25 mm behind the slim body's). The feminine frame has none (`FLAT` is empty for it).
- The region sliders' fat step (`fat_<region>`) is to the heavy and soft corner with its `Fat_Soft` share, so a region's fat is the same fat.

**Fat over muscle (R3, wave 3).** MakeHuman's most muscular and heaviest corner is a heavyweight bodybuilder: separate deltoids, biceps and trapezius, a V-taper, and on the feminine frame narrow hips over thighs that flare out below them (its muscle takes the fat off the hips while `Fat_Soft`'s saddlebags stay). `<Frame>_Fat_Over_Muscle`, driven by `muscle` times `fat` (each from 0 to 1, so whole at both 1), makes that corner a strong, heavy person (`shapes.fat_over_muscle`): what muscle adds to the heavy body (the heavy and muscular body less the heavy one, both from the held shapes) is split into its smooth part (40 rounds of halfway to the neighbours' mean, about 10 cm) and the rest; the shape takes off 0.8 of the rest (the definition) and 0.45 of the smooth part (the mass), as fat covers muscle. It is kept apart with the other shapes. On the built body (Blender, rest pose, before in brackets): the masculine upper arm's girth 468 mm (501; heavy 445), the forearm 347 (387), the chest 844 (905); the feminine upper arm 408 (452; heavy 373), the hips 1,148 (1,131); the upper body's definition (what is left of the move once smoothed over 10 cm, RMS) 5.4 mm masculine (9.1; heavy 4.1) and 4.9 feminine (8.8; heavy 3.6). The feminine thigh no longer flares past the hips: its widest point side to side between 14 and 30 cm under the hip joints is 0.9 mm past the hips' (10.8 before; heavy -1.2). The kept-apart cut is smaller with it: the feminine heavy and muscular body crosses itself in 19 pairs before keeping apart (81) and is cut to 0.80 (0.51).

**Per region.** `fat_<region>` and `muscle_<region>` for chest, shoulders, arms, abdomen, back, hips, thighs and calves (advanced, -1 to 1): up to 0.6 of the step to the heavy and soft body (fat) or the heavy and muscular one (muscle) inside the region, added or taken off. Regions are smooth and come from the skin's bone weights, so they add up to one over the trunk and limbs.

**What holds the shapes back.**
- *Bone.* Fat shapes are held to 15% over the skull above the eyes, the collarbones, the elbows' points, wrists, kneecaps, the shins' front, ankles, hands and feet (`shapes.bony`).
- *Eyes and mouth.* No shape moves the lids, the sockets, the lips' edges or the mouth's inside (`shapes.face_hold`), so the eyes and teeth stay fitted at every setting.
- *Crossing.* The heaviest combinations are checked in the rest pose and every shape of the frame is scaled down smoothly round the places where they made the body cross itself (`Realistic.build.json`, `body_shapes.frames.*.kept_apart`): on the masculine frame at the armpits and between the thighs (the heavy and soft body 331 vertices, the heavy and muscular one 423, the slight one 134, to 0.21 of the shape at the least), on the feminine frame 123, 94 and 178 vertices (to 0.80). (Before R3's wave 3, without `Fat_Over_Muscle`: 331, 493 and 76 masculine to 0.21; 123, 316 and 124 feminine to 0.51. Before R3: 221, 431 and 100 masculine to 0.26; 94 feminine to 0.80.)

**Not in the geometry.** The base mesh's trunk quads are 25 mm wide. MakeHuman's finer detail (abdominal lines, the cut between thigh muscles, the navel's folds) is averaged out when its offsets are sampled (`transfer.py`, `filtered`), on purpose: point-sampled, it folded into a crease across the belly and a hard line under the chest. That definition belongs in a normal-map layer the muscle slider blends in (CO7).

## Corrective shapes (CO5)

Six **live** shapes, set on every pose by `crates/anim/src/correctives.rs` (and by the QA scripts' `Character.drive_correctives`): `Corrective_Shoulder_Level_Left/Right`, `Corrective_Shoulder_Raised_Left/Right` and `Corrective_Shoulder_Lowered_Left/Right`. The rule is in `Realistic.build.json` (`correctives`): a shape's weight is how far its bone (the upper arm) has turned from rest about an axis in its parent's rest space, 0 at `angles[0]` degrees and 1 at `angles[1]`: `Level` from 0 to 45 (the arm level), `Raised` from 45 to 105 (150 degrees from hanging), and `Lowered` from 0 to 35 about the raise axis reversed (the arm hanging 10 degrees out, as the starter Idle holds it; Walk swings it 6 to 22). Without `Lowered` the cap squares off and a ridge rises beside the neck whenever the arms hang, which is most of the time: the shoulder gained 12.5 / 9.3% of its local volume (masculine / feminine); with it, 2.4 / -1.4% (`docs/development/character-studio/lowered-shoulder/`). `Lowered` is made on its own, not on top of the raising shapes.

They are a **delta mush** of their poses (`correctives.Mush`): the skinned pose smoothed (24 times, each halfway to the neighbours' mean), with each vertex's rest-pose offset from its own smoothed place put back in the smoothed surface's frame. A shape is that target less the skinned pose inside a soft region round the shoulder girdle (full within 11 cm of the line neck's root, shoulder joint, middle of the upper arm; none past 21 cm), carried to rest by each vertex's skinning matrix. Nothing in them is solved against a measure. What they do: the skin runs in one slope from the neck across the trapezius onto the arm instead of caving at the joint, and the armpit is a hollow instead of a fold. With them the whole body's enclosed volume at 150 degrees is within 0.1 litre of its rest volume (skinning alone adds 0.5 litre of overlap). The QA scripts' local-volume measure at the joint reads **lower** with them (0.463 masculine, 0.513 feminine at 150 degrees, against 0.56 for skinning alone); why that measure falls while the picture and the whole volume improve hasn't been established. See `docs/development/character-studio/co5/README.md`.

Made for raising the arm sideways with the collarbone still; an arm swung forward leans on them only as far as it is also raised. The starter animations don't turn the collarbones at all (their `LeftShoulder` and `RightShoulder` keys are the rest pose), so that is the case clips play today.

## Brow length

`brow_length_left` and `_right` (2026-11-16, Paul: "I can not adjust the brow length") move the `Brows` card alone: `Face_Brow_Length_<side>` draws the tail in to 0.80 of the way along the brow's line (the card runs to 1.08 as built), driven from 0 to -1, and `Face_Brow_Length_Long_<side>` carries it on down the temple to 1.34, driven from 0 to 1 (`lashes.brow_lengths`, the same card laid out again). The hair's texture runs the card's whole length, so a shorter brow is a whole brow, shorter. The body carries the four shapes empty, so the manifest can name them (architecture.md section 14).

## Underwear

`underwear.py` (2026-11-16) cuts three garments from the body's own surface: **boxer briefs** (from the hip bones' crest to a third of the way down the thigh, with a waistband), **briefs** (high cut: the leg opening runs round the thigh from the crotch up above the hip joint at the side, and across the buttocks behind) and a **sports bra** (laid out on the feminine frame: under the bust to a scooped neckline, across the shoulder blades behind, cut away under the arms, with a strap over each shoulder). Male (`frame` 0) wears the boxer briefs; Female (1) the briefs and the bra (the parameter's `choices`, written by `manifests.py`); every one fits both frames. Each garment's shapes are then pushed out of the skin (`push_out`, CO9: 2 mm, at every shape's full weight and at the build's four muscle and fat corners, on both frames, checked from the cloth's vertices and from the skin's under it; `Realistic.build.json` reports `shapes_pushed` and `push_most`).

- **Cutting.** Each garment is a few simple, nearly linear fields on the body (height, distance down the thigh and which way round it, distance from the middle, the arms' and thighs' weights) and a rule over them. The surface is split along each field's zero line in turn, then the pieces the rule keeps are the garment, so hems are curves and a strap narrower than the body's triangles is still cut exactly. A gusset strip joins the leg openings under the crotch. The waistband and the bra's band are a second material.
- **Fitting.** A garment vertex is a fixed mix of body vertices, so it takes their weights and every shape the body has (the frame, muscle and fat with their corners, the correctives): `fit: shapes`. A point on a hem can mix two vertices' eight influences; past four, each goes to its nearest kept ancestor bone (the build data's `underwear.*.weight_moved_most` is the most any vertex moved: 0.07 to 0.12).
- **Lying like cloth.** It stands 2.4 to 3 mm off the skin; it is drawn straight across the cleavage and the buttocks' cleft, then smoothed outward only (`taut`), so it spans hollows and keeps to convex skin; at every hem a rim folds 4 mm down into the skin.
- **The skin under it.** Each item hides its own region, whose mask shape (`Mask_<object>`) sinks the skin it covers 5 mm and eases back over 15 mm past the hems. Without it the body's simpler copies came through the cloth at a distance, and at a strap's edge the skin beside it did. This is an interim exception to ADR-0035 (fitting by binding, hiding by the region map): CO9 builds those.
- **Look.** Charcoal cloth with a darker, glossier elastic band (`cloth` shading).

## Proportions (CO5, R3)

The nine proportion sliders set `anim`'s body sliders, which scale bones. `anim` takes 0.5 to 1.5. CO5 cut this body's to 0.85 to 1.15 and the hips to 1 to 1.15; since R3 (plan section 0, decision 8) each slider has its own range (`manifests.PROPORTION_RANGES`), each end the last step that still deforms cleanly on both frames and at the corners of muscle and fat, or the population's 0.5th or 99.5th percentile, whichever is nearer to 1, found by `../humanoid/qa/ranges.py`:

| `height` | `leg_length` | `arm_length` | `shoulder_width` | `waist_width` | `hip_width` | `head_size` | `hand_size` | `foot_size` |
|---|---|---|---|---|---|---|---|---|
| 0.82 to 1.13 | 0.85 to 1.125 | 0.95 to 1.08 | 0.88 to 1.12 | 0.8 to 1.175 | 0.7 to 1.2 | 0.88 to 1.12 | 0.82 to 1.07 | 0.96 to 1.21 |

What set each end, with the numbers, is in `docs/development/character-studio/refinement/r3/ranges.md`; `avatar`'s `tests/realistic_ranges.rs` holds the manifest to that table. Some ends are nearer 1 than CO5's: 15% of an arm or a head is several SD, and the base's feet are already short.

**The hips narrow.** `HipWidth` below 1 pulls the legs' roots in and the inner thighs cross at the crotch (at 0.95 already), so `hip_width` drives the attribute only from 1 up (`manifests.ATTRIBUTE_FROM_ONE`) and below 1 a shape narrows the surface: `<Frame>_Hips_Narrow` (`shapes.hips_narrow`), a corner of the frame and `hip_width` from 1 to 0.6 (`shapes.NARROW_AT`). It moves the skin of the pelvis and the thighs' tops toward the middle, side to side only, by up to 20% of each vertex's distance from it, holding the inner thighs and fading out above the hip crest and down the thigh, so the hips' breadth falls about as fast below 1 as the bone widens it above (0.864 of the default at 0.7, 1.099 at 1.2). The underwear and both simpler copies carry it like the other body shapes.

**The waist (wave 3).** `WaistWidth` scales only the skin weighted to the Spine bone, which moved the natural waist's breadth by a quarter of the slider and stepped where the bone's weight ends. `<Frame>_Waist_Wide` (whole at 1.5) and `<Frame>_Waist_Narrow` (whole at 0.5), driven by `waist_width` from 1, carry the difference between the bone's move and the waist the slider means (`shapes.waist`): the trunk scaled about its middle by 0.9 of the slider's change side to side and 0.5 front to back (the back a third of the belly's move), whole at the natural waist (the trunk's narrowest level, found on each frame) and fading smoothly to nothing just under the hip joints and 0.9 of the hip-to-chest height above the waist. The shapes are that target less the bone's own move, so the two add up to it at every value. Measured with the bone applied as `anim` applies it, the natural waist's breadth changes by 0.82 to 0.88 of the slider's change and its depth by 0.29 to 0.31, at both ends on both frames (`crates/gameplay/tests/realistic_waist.rs`). Scripted, not a MakeHuman target (its `measure-waist-circ` isn't vendored and would also have needed the bone's move taken out).

## Known limits

- The raised shoulder's corrective shapes are made for raising the arm sideways with the collarbone still; raised forward, or on a rig that turns the collarbone, the shoulder has only its skinning. At 150 degrees a tight valley remains where the arm meets the neck.
- The body's shapes are MakeHuman's anatomy at this mesh's resolution: the heavy and muscular corner is a strong, heavy build since R3's wave 3 (`Fat_Over_Muscle`; a bodybuilder's before), though the feminine forearm still shows some muscle under the hard light, and muscle definition finer than 25 mm isn't in the geometry (the defined normal-map layer, R5).
- Fat reaches the face only under the chin and along the jaw (`neck-double-incr`); the cheeks don't fill (the face is held round the eyes and mouth, `shapes.face_hold`).
- One normal and occlusion map serves both frames, so detail that sits differently on the two (the upper eyelid's fold) is softened or slightly misplaced on the feminine frame: two short dark marks above each feminine eye.
- The underwear's shapes are copied, so a garment is as heavy as its share of the body's shapes (37 to 53 shapes each); the briefs' back hem meets the gusset in a small crumple seen from below.
- No subsurface map and no hair. What the face still falls short of is listed in [the face's notes](../../../docs/development/character-studio/co6/README.md).
