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

From [measurements.json](measurements.json) (`measure.py --package realistic --only files,weights,clipping,face,shapes,sliders,proportions,deformation,twist --check assets/characters/realistic/qa/thresholds.json --strict`: passed).

| Measure | Target | CO4 (masculine, feminine) | CO5 |
|---|---|---|---|
| Shoulder raised 150°: local volume kept | at least 0.8 | 0.562, 0.561 | **0.86, 0.925** |
| Shoulder at 90°: local volume kept | the stylized body's 0.853, 0.992 | 0.813, 0.831 | **0.955, 1.001** |
| Elbow 135°, knee 135°: ring area kept | at least 0.6 | 0.757, 0.758 and 0.74, 0.75 | unchanged |
| Eyes and teeth in flesh at rest | 0 | 0 | 0 |
| Pairs of sliders at the ends of their ranges that make the body newly cross itself | 0 | not measured | **0 of 351 pairs (1,352 bodies) on each frame** |
| A slider alone that newly crosses | 0 | not measured | 0 of 53 ends |
| A shape slider alone that puts skin in an eye or the teeth | 0 | not measured | 0 of 36 ends |

**Crossings before any shape.** The masculine neutral has 60 pairs of crossing triangles and the feminine 68 (between toes, and inside the head), which are the bundle's own. "Newly" leaves those out, and pairs that already touch within 4 mm in the neutral (closed lips, neighbouring toes).

**What the proportion sliders needed.** At `anim`'s full 0.5 to 1.5, 341 of 1,404 masculine bodies and 213 feminine ones crossed: shoulders at half width alone made 282 pairs, arms at half length 151, hips at half width 137. At 0.85 to 1.15 only narrower hips still did (26 pairs alone at 0.93, 12 at 0.98 on a heavy and muscular body, at the crotch), so the hips only widen. The harness applies the sliders as `anim::BodyShape::apply` does; its bone positions were compared with the engine's for six sliders and seven bones and agree within 0.1 mm.

**Kept apart** (`Realistic.build.json`, `body_shapes`): masculine heavy and soft 31 crossings (221 vertices cut, to 0.26 of the shape at the least), heavy and muscular 129 (431, to 0.41), slight 24 (206, to 0.64); feminine heavy and muscular 2 (94, to 0.80), slight 18 (137, to 0.64). All 0 after.

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
- **[deformation_poses.png](deformation_poses.png)**: the shoulder at 90° is clean. At 150° the cap is back (it was caved in CO4's sheet) but it is a smooth mass with a notch where it meets the neck, not a deltoid; the armpit is still a plain stretch.

## Still short

1. Muscle definition and the fat body's smoothness need the normal-map layers (CO7): today one map, baked from the athletic sculpt, is on every body.
2. The heavy and muscular corner; the face under fat; the masculine slight body (above).
3. Correspondence along the limbs is as loose as in the spike (1 to 3 cm); only the navel and nipples are pinned.
4. Corrective shapes exist for one motion (the arm raised sideways). Elbow, knee, hip and the arm raised forward have none; the shoulder's is built to the QA measure and by eye, not from a scan or a simulation.
5. The hips can't narrow, and the proportion sliders stop at 15%: the body crosses itself beyond.
6. The eyes and teeth don't follow body shapes because no shape moves the skin near them. If CO6 wants a fat face (jowls up to the lips, heavy lids), the held zone (`shapes.face_hold`) has to shrink and the eyes and teeth have to follow.
7. `assets/characters/benchmarks/make.py` names the realistic body's slim case `{"fat": 0, "muscle": 0, "weight": -1}`; with these parameters it should be `{"fat": -1}` (and heavy-and-soft `{"fat": 1, "muscle": -1}` is worth adding). Not changed here.

## Regenerating the vendored shapes

Needs the bundle, MakeHuman's CC0 files (fetched and verified by hash) and about three minutes. From `assets/characters/transfer/`, with `PY` a Python that has `bpy`, always with `-I`, and `WORK`, `MH` scratch folders:

```sh
python3 fetch_makehuman.py MH
PY -I extract.py --bundle path/to/human_base_meshes_bundle.blend --makehuman MH --work WORK
PY -I marks.py --work WORK                 # navel and nipples of both bodies
PY -I register.py --work WORK              # pins them
PY -I transfer.py --work WORK --method filtered --pack ../realistic/source/body_shapes.npz
```

Then build the package (`assets/characters/realistic/README.md`).
