# CO5: body parameters on the realistic base

Milestone CO5 of [the plan](../../../../plans/axcenity-engine-character-studio.md): muscle and fat for the realistic body, whole and per region, on both frames; the proportion sliders checked and cut to what the body can take; corrective shapes for the raised shoulder with their driver in `anim`. It builds on [the spike](../co5-spike/README.md) and answers its conditions. How the package holds it is in [assets/characters/realistic/README.md](../../../../assets/characters/realistic/README.md); this page is the evidence.

## What the spike's conditions became

| Spike's condition | What was done |
|---|---|
| Features land 1 to 3 cm off; a hard line under the pectorals, a crease where the navel should be | Two causes, two fixes. **Sampling:** MakeHuman's trunk is finer than ours (17 mm edges, 5 mm at the navel, against 25 mm), and point-sampling its offsets folded detail our mesh can't hold into a crease and a line. `transfer.py`'s `filtered` method averages MakeHuman's offsets over each of our vertices' own patch of skin first. **Marks:** the navel and both nipples are pinned in the registration (`marks.py`: read from MakeHuman's topology, and from our sculpt, since our base mesh can't show them). Neither the line nor the crease is in [build_details_masculine.png](build_details_masculine.png). Depth-read features (chest crease, buttock, calf, …) were tried and **not** used: on MakeHuman's male and female neutrals, where the answer is known, they land 14 to 38 mm from the right vertex and leave the trunk's error where it was (13.3 mm before, 13.8 after); `features.py` records it. The slide along the limbs is therefore still there; smooth shapes don't show it. |
| The armpit: keep-apart cost arm girth and left a dark slot | Kept apart again, in the engine's rest pose (the A-pose), on the combinations the sliders can reach (the four corners of muscle and fat with every region's slider pushed the same way), not on MakeHuman's raw corners. The sliders stop short of MakeHuman's heaviest bodies, so less has to be cut. No slot shows in the sheets. |
| The masculine base is already athletic | The sliders walk MakeHuman's grid from where each frame's own body sits on it: masculine at muscle 0.45, weight 0.30; feminine at 0, 0. Slim and slight take off what the base starts with. 0 on both sliders is the bundle's body, untouched. Chosen from the renders; a girth fit says (0.15, 0.80) for the masculine body, which reads its broader frame as weight. |
| Extremes are caricatures | The sliders' ends are inside MakeHuman's square: masculine muscle -0.6 to 1.0, feminine -0.7 to 0.9, fat -0.75 to 0.85 on both. Its gaunt, obese and comic-book bodies are outside. The heavy and muscular corner is still a bodybuilder. |
| Fine definition is lost | Left lost, on purpose: it goes to a normal-map layer the muscle slider blends in (CO7). Nothing is faked in geometry. |
| Fat must respect bone | A mask built from the skeleton's joints and the face sets (not from MakeHuman's data, which was blotchy): the skull above the eyes, collarbones, elbows' points, wrists, kneecaps, the shins' front, ankles, hands and feet hold fat shapes to 15%. |
| Shape count and size | 53 shapes on `Body` and on both copies (1 frame, 16 whole-body, 32 regional, 4 corrective). `RealisticParts.glb` is 20.5 MB (10.2 MB before). The vendored source is 0.43 MB. |

## Numbers

From [measurements.json](measurements.json) (`measure.py --package realistic --only files,weights,clipping,face,shapes,sliders,proportions,deformation,twist --check assets/characters/realistic/qa/thresholds.json --strict`: **failed**, on the shoulder only: two limits (150° under CO4's 0.54 on both frames) and four targets; everything else holds).

| Measure | Target | CO4 (masculine, feminine) | CO5 |
|---|---|---|---|
| Shoulder raised 150°: local volume kept | at least 0.8 | 0.562, 0.561 | **0.463, 0.513: not met, and under CO4's limit of 0.54** |
| Shoulder at 90°: local volume kept | the stylized body's 0.853, 0.992 | 0.813, 0.831 | 0.847, 0.88: not met |
| Elbow 135°, knee 135°: ring area kept | at least 0.6 | 0.757, 0.758 and 0.74, 0.75 | unchanged |
| Eyes and teeth in flesh at rest | 0 | 0 | 0 |
| Pairs of sliders at the ends of their ranges that make the body newly cross itself | 0 | not measured | **0 of 351 pairs (1,352 bodies) on each frame** |
| A slider alone that newly crosses | 0 | not measured | 0 of 53 ends |
| A shape slider alone that puts skin in an eye or the teeth | 0 | not measured | 0 of 36 ends |

**The shoulder, and why its number went down.** The first corrective shapes of this milestone moved the skin out from the joint until this measure read 0.86: fitted to the measure, and it looked it (a ball with a notch). They were replaced by a delta mush of the pose (the package README says how), which is shaped by nothing but the pose and the rest shape. The picture is right now ([shoulders_masculine.png](shoulders_masculine.png), [engine/cheer_shoulders_after.png](engine/cheer_shoulders_after.png)) and the measure is lower than with no corrective at all. One other number points the same way as the picture: the whole body's enclosed volume at 150° is +0.09 litre (masculine) and -0.05 litre (feminine) from rest with the shapes, against +0.52 and +0.28 litre for skinning alone. Why the local measure (the skin within 9 cm of the joint, as volume seen from the joint) falls while both improve hasn't been established: the smoothed skin may simply lie outside that 9 cm, or closer to the joint on top. The limit and the target are left as they are; the decision is the lead's and Paul's.

**Crossings before any shape.** The masculine neutral has 60 pairs of crossing triangles and the feminine 68 (between toes, and inside the head), which are the bundle's own. "Newly" leaves those out, and pairs that already touch within 4 mm in the neutral (closed lips, neighbouring toes).

**What the proportion sliders needed.** At `anim`'s full 0.5 to 1.5, 341 of 1,404 masculine bodies and 213 feminine ones crossed: shoulders at half width alone made 282 pairs, arms at half length 151, hips at half width 137. At 0.85 to 1.15 only narrower hips still did (26 pairs alone at 0.93, 12 at 0.98 on a heavy and muscular body, at the crotch), so the hips only widen. The harness applies the sliders as `anim::BodyShape::apply` does; its bone positions were compared with the engine's for six sliders and seven bones and agree within 0.1 mm.

**Kept apart** (`Realistic.build.json`, `body_shapes`): masculine heavy and soft 31 crossings (221 vertices cut, to 0.26 of the shape at the least), heavy and muscular 129 (431, to 0.41), slight 24 (206, to 0.64); feminine heavy and muscular 2 (94, to 0.80), slight 18 (137, to 0.64). All 0 after.

## The fold under the breast

Found under the side lamp, there at rest: the bundle's feminine body has a sharp fold under each breast (up to 49 degrees between neighbouring faces) running askew over quads 25 mm wide, so its edge was stair-stepped with a notch at its outer end. `body.soften_folds` relaxes it on the feminine frame only: the vertices of the edges under the chest that fold inward by more than 24 degrees, or bend outward by more than 26 on the ridge just above, and their neighbours, are moved toward the middle of their own neighbours a little at a time until no edge does (23 vertices were on such edges; the masculine frame is untouched, as are vertex order and count). The breast keeps its form and meets the ribs in a curve; what is lost is the fold's depth.

- **[chest_feminine_before.png](chest_feminine_before.png)** and **[chest_feminine.png](chest_feminine.png)**, **[chest_blend_before.png](chest_blend_before.png)** and **[chest_blend.png](chest_blend.png)** (clay under the side lamp, at rest; default, heavy and soft, slim; front, three-quarter, from below). *Before:* a stepped edge under each breast in the front view and a hooked crease from below, on every build. *After:* a smooth curve in all three views on the default and slim bodies, feminine and blend. *Wrong:* the breast is shallower underneath than the sculpt's and reads a little pointed in the three-quarter view; on the heavy and soft body a short crease is left on the belly from below (that body's own fold, MakeHuman's, not the breast's).
- **[engine/chest_after.png](engine/chest_after.png)** (the engine: feminine at rest and in Cheer at 0.8 s, then the blend body the same; studio front, three-quarter, hard light) beside **[engine/chest_before_cheer_feminine.png](engine/chest_before_cheer_feminine.png)** (the feminine body in Cheer before). *Before:* the breast's lower edge is faceted in the three-quarter view. *After:* a curve at rest and in Cheer on both. *Wrong:* under the hard light the breast's own shadow has a sharp edge across the ribs (a shadow, not the surface), and the normal map there was baked for the sharper fold, so the shading under the breast is a little flatter than the map expects.
- What rides on it: both simpler copies take `Frame_Feminine` from the body's surface, so they follow; the feminine build shapes are carried from the bundle's pose by each vertex's local frame and were kept apart again by the build; eyes and teeth are untouched; the slider pairs still make no new crossing on either frame.

## The armpit on heavy bodies

In the engine's Cheer the armpit of the heavy bodies is a near-black patch. It is shadow and occlusion, not crossing skin. `assets/characters/humanoid/qa/posed_crossings.py` ([posed_crossings.json](posed_crossings.json)) counts the pairs of triangles that cross in a pose and don't at rest (nor lie within 4 mm of touching there), with the twist bones and corrective shapes driven:

| Body | shoulder_abd_150: new crossings | Cheer at 0.8 s: new crossings | of them within 22 cm of a shoulder joint | of them in the hands |
|---|---|---|---|---|
| masculine default | 0 | 268 | 0 | 268 |
| masculine heavy and muscular | 0 | 268 | 0 | 268 |
| masculine heavy and soft | 0 | 306 | 0 | 306 |
| feminine default | 0 | 240 | 0 | 240 |
| feminine heavy and muscular | 0 | 240 | 0 | 240 |
| feminine heavy and soft | 0 | 241 | 0 | 241 |

Nothing crosses round the shoulders on any of them, so nothing was changed. Every new crossing in Cheer is in the fists (fingers passing through each other, the count `measure.py`'s `fist` pose already reports: 247 and 199 for the QA fist). That is CO4's hand weighting, not this milestone's, and is listed below.

## Tests

- `crates/avatar/tests/realistic_parameters.rs`: every parameter at each end of its range moves the baked body and both simpler copies by more than 2 mm (shape and bone sliders), or changes one of `anim`'s nine body sliders (proportions); muscle and fat make four different corners and the corner isn't the sum of the two; a baked body keeps only its four corrective shapes as live ones. Red first against CO4's files: [red-run-parameters.txt](red-run-parameters.txt).
- `crates/anim/tests/realistic_proportions.rs`: each of the nine body sliders moves the realistic body's skin and both copies at 0.5 and 1.5. (Green from the start: the sliders existed. It is a check, not a fix.)
- `crates/anim/src/correctives.rs`: the rule, the constants against the build data, the shipped body through the pose path on all three levels of detail. Red first against a stub: [red-run-correctives.txt](red-run-correctives.txt).

## What the sheets show

Looked at against anatomy: where fat sits (belly, flanks, chest, hips and thighs before the limbs' ends), what a lean body shows (ribs' edge, hip bones, a flat belly), what muscle does to a silhouette (shoulders and thighs before the waist).

- **[engine/corners_masculine.png](engine/corners_masculine.png), [engine/corners_feminine.png](engine/corners_feminine.png)** (in the engine: front, side, back, face; made from the capture's sheets by cropping). *Good:* every corner is a different body and reads as what its name says; slim is slim on the masculine frame (narrower chest and limbs than the default); heavy and soft carries its weight at the belly, flanks and chest and keeps thin wrists, shins and ankles; nothing crosses; the eyes sit right in every face. *Wrong:* the **normal map is the sculpt's**, so the soft bodies keep the lean body's abdominal lines drawn on a belly (clearest on masculine heavy-soft); heavy-muscular is still a **bodybuilder** on both frames, its trapezius as wide as the head; **the face barely changes** with fat (a thicker neck and little else: the shapes are held off the eyes and mouth, and fat is cut short of MakeHuman's heaviest); the masculine "slight" body has a small belly under a narrow chest, which is MakeHuman's low-muscle body, not a lean one.
- **[engine/regions_masculine.png](engine/regions_masculine.png), [engine/regions_feminine.png](engine/regions_feminine.png)**: belly, arms and shoulders, hips and thighs, chest and back, each at 1. *Good:* each changes its own place and blends out. *Wrong:* the masculine belly at 1 is slight from the front; `muscle_hips` does next to nothing (3 to 4 mm).
- **[build_details_masculine.png](build_details_masculine.png), [build_details_feminine.png](build_details_feminine.png)** (Blender, clay). *Good:* no hard line under the pectorals, no crease across the lower belly, no dark slot at the armpits; the feminine heavy and soft body has the fold under the breast and the hip shelf. *Wrong:* a faint horizontal fold remains across the masculine heavy and soft belly; the feminine heavy and muscular shoulder is lumpy where the trapezius meets it; clay at this light is flat, so fine faults could hide here that the engine sheets would show.
- **[build_grid_masculine.png](build_grid_masculine.png), [build_grid_feminine.png](build_grid_feminine.png)**: nine bodies each; no step between neighbours looks like a jump.
- **[build_regions_masculine.png](build_regions_masculine.png), [build_regions_feminine.png](build_regions_feminine.png)**: all sixteen regional sliders; same reading as the engine's.
- **[shoulders_masculine.png](shoulders_masculine.png), [shoulders_feminine.png](shoulders_feminine.png)** (clay under one lamp from the side, since flat clay hid the first shapes' faults): the arm at 150° from the front, back and above, at 90°, and the Cheer clip at 0.8 s. *Good:* one slope from the neck across the trapezius onto the arm in every view, no step, no ball; the armpit is a hollow with the pectoral's edge running into the arm. *Wrong:* at 150° a tight valley is left where the arm meets the neck (the collarbone doesn't lift in this pose); the deltoid has no form of its own (it is a smooth slope, not a muscle); (The jagged fold under each feminine breast that the first version of this sheet showed was at rest too; it is fixed, see the fold's section.)
- **[engine/cheer_shoulders_after.png](engine/cheer_shoulders_after.png)** beside [engine/cheer_shoulders_before.png](engine/cheer_shoulders_before.png) (the engine, Cheer at 0.8 s, same crop; masculine, feminine, the stylized body). *Good:* the step at the top of each shoulder and the ball behind it are gone on both frames. *Wrong:* the armpit's front edge is a hard shadow line, and the back of the masculine shoulder still shows a ridge where the arm's skin meets the back's.
- **[engine/cheer_shoulders_builds.png](engine/cheer_shoulders_builds.png)**: the same pose on the heavy and muscular and the heavy and soft bodies, both frames (front, back, three-quarter). The shapes ride on the baked builds without a step. *Wrong:* the armpit is a deep dark pit on the heavy bodies, and the heavy arms' own lumps show on the feminine back.
- **[deformation_poses.png](deformation_poses.png)**: the QA poses in the flat studio; the shoulder tiles are the same shapes as above.

## Still short

1. Muscle definition and the fat body's smoothness need the normal-map layers (CO7): today one map, baked from the athletic sculpt, is on every body.
2. The heavy and muscular corner; the face under fat; the masculine slight body (above). (R3, 2026-11-17, gave `fat` 1 a belly and flattened the slight masculine belly: `docs/development/character-studio/refinement/r3/README.md`.)
3. Correspondence along the limbs is as loose as in the spike (1 to 3 cm); only the navel and nipples are pinned.
4. Corrective shapes exist for one motion (the arm raised sideways, collarbone still). Elbow, knee, hip, the arm raised forward and a turned collarbone have none. The shoulder's local-volume number is under its target and its old limit (above).
8. The fold under the feminine breast is a soft curve now, shallower than the sculpt's; the normal map there was baked for the sharper fold.
9. Fists: 240 to 306 pairs of finger triangles cross in Cheer (and 247 and 199 in the QA fist pose). Not looked into here.
5. The hips can't narrow, and the proportion sliders stop at 15%: the body crosses itself beyond. (R3, 2026-11-17: each slider's ends are now found by a sweep, and the hips narrow by a shape: `docs/development/character-studio/refinement/r3/ranges.md`.)
6. The eyes and teeth don't follow body shapes because no shape moves the skin near them. If CO6 wants a fat face (jowls up to the lips, heavy lids), the held zone (`shapes.face_hold`) has to shrink and the eyes and teeth have to follow.
7. `assets/characters/benchmarks/make.py` names the realistic body's slim case `{"fat": 0, "muscle": 0, "weight": -1}`; with these parameters it should be `{"fat": -1}` (and heavy-and-soft `{"fat": 1, "muscle": -1}` is worth adding). Not changed here.

## Regenerating the vendored shapes

Needs the bundle, MakeHuman's CC0 files (fetched and verified by hash) and about three minutes. From `assets/characters/transfer/`, with `PY` a Python that has `bpy`, always with `-I`, and `WORK`, `MH` scratch folders:

```sh
python3 fetch_makehuman.py MH
PY -I extract.py --bundle path/to/human_base_meshes_bundle.blend --makehuman MH --work WORK
PY -I marks.py --work WORK                 # navel and nipples of both bodies
PY -I register.py --work WORK              # pins them
PY -I transfer.py --work WORK --method filtered --pack ../realistic/source/body_shapes.npz \
    --pack-extras masculine/m_thyroid,masculine/m_adrenal,masculine/neck_double,masculine/pregnant_decr,feminine/f_ovary,feminine/f_liver,feminine/upperarm_fat,feminine/neck_double
```

Then build the package (`assets/characters/realistic/README.md`). The `--pack-extras` list (R3, 2026-11-17) is the regional targets `realistic/shapes.py`'s `SOFT` and `FLAT` name; without it the build stops for want of them. The bundle path and `MH` must hold the files `makehuman.sha256` lists (41).
