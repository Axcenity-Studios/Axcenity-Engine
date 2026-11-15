# Humanoid QA

Scripts that measure the default humanoid for defects and render a fixed set of comparison sheets, so a change to the character can be checked against a "before" set (the Character Studio overhaul keeps its baseline in `docs/development/character-studio/baseline/`). They read the glTF files only; they never change the asset.

| File | What it does |
|---|---|
| `qa_scene.py` | Shared helpers: import a file, set a look (frame, build, face, expression and mask shapes, the `Frame_Feminine` bone offsets and eye offsets and scales from `Humanoid.customization.json`), the fixed test poses, deformed geometry, the studio (Cycles CPU, AgX, OIDN, three area lights, grey cyclorama, seed 7) and contact sheets. |
| `measure.py` | Writes `measurements.json`: per-object stats and topology, skin weights, clothing and hair clipping for each frame and build, joint deformation in the test poses, face shapes (self-intersection, eyes and lids, teeth), shape ranges, proportions against Drillis and Contini (1966). |
| `render_baseline.py` | Renders the sheets: turnarounds, face close-ups and topology, base body, build range, hair styles, test poses, expressions, a details sheet, and `fit` (clothing fit at the extremes: the outfit, the tee alone and the underwear on both frames with the heavy build, the shoes on heavy and slim). |
| `thresholds.json` | The clipping limits `measure.py --check` holds the build to, with why each garment's limit isn't zero. |

## Running

Needs the `bpy` module (Python 3.13 for bpy 5.x) and Pillow:

```bash
python -m venv .bpy && .bpy/bin/pip install bpy pillow
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --source-ngons --out docs/development/character-studio/baseline/measurements.json
.bpy/bin/python -I assets/characters/humanoid/qa/render_baseline.py --out docs/development/character-studio/baseline
```

Measuring takes about 25 seconds (`--source-ngons` runs the build's `build()` into a temporary folder to count polygons before glTF triangulates them; leave it out to skip that). Rendering takes about a minute on a 16-core CPU. Both take `--parts` and `--default` to point at other files, `--only` to run some sections (`files,weights,clipping,face,shapes,proportions,deformation`) or sheets (`turnaround_masculine,turnaround_feminine,face_closeups,face_topology,body_clay,build_range,hair_styles,deformation_poses,expressions,details,fit`). **After changing the build**, check it: `measure.py --only clipping --out <file> --check assets/characters/humanoid/qa/thresholds.json` exits 1 and prints each limit that got worse (the waist gaps and the layer order must stay at zero). Look at `fit.png` too: a number isn't a look. For an "after" set, render into another folder with the same settings and compare sheet by sheet; don't change `STUDIO`, the cameras or the poses, or the sets stop being comparable.

## Reading the numbers

- Blender space: metres, Z up, facing -Y. Distances are millimetres unless a key ends in `_m`.
- Topology is on position-merged vertices (glTF splits vertices at UV seams). glTF stores only triangles, so n-gons are counted in the build (`source_ngons`); in the files they show up as sliver triangles.
- Clipping: a skin vertex is **covered** when a ray along its normal meets the layer within 5 cm (6 cm for hair), **poke** when instead the layer's outward face is within 15 mm under it (the skin shows through), **exposed** when that face is 15 to 30 mm under it (bare skin over an opening, such as a shoe's throat, or a layer passing through the head). `waist_gap_mm` is the height of bare skin seen between the outfit's garments from 16 directions (hits on an arm's skin don't count: a heavy build's arm reaches inside the rays' start); in `Humanoid.glb` that skin is removed, so it's a hole.
- Layering checks the build's layer order (the trousers' waistband under the untucked tee, the tee under the jacket) and counts only inner points facing away from the body: an opening's rolled-in band faces the skin and can't be seen.
- Known limits of the poke test, checked by eye in `fit.png`: in the body's midline creases (the crotch, the cleft between the buttocks) cloth bridges the fold and a ray along the skin's normal grazes it, so the briefs and trousers count points there that don't show; toe tips sit inside the sneakers' solid sole, whose open top rim lets their rays out; in `hip_flex_100` the raised thigh holds the waist rays' start points, so that pose's `waist_gap_mm` measures the thigh from inside (the renders show the waist covered).
- Poses (left side): flexion from straight, so `elbow_135` turns the forearm 135 degrees; the shoulder poses do the whole turn at the shoulder joint; `fist` curls each finger 85/100/70 degrees. `local_volume_ratio` is the signed volume of the skin within the listed radius of the joint, seen from the joint, posed over rest (rigid parts keep it at 1); `ring_area_ratio` is the area of the skin's cross-section at the joint, posed over rest.
- Feminine looks move the skeleton by the customization file's offsets and the eyes by their offset and `eye_scale`, which is what CS3 is meant to do; `face.feminine_eyes_not_moved` is the same look without that.

## Other packages

Both scripts take `--package` (`stylized`, the default, or `realistic`), which picks that package's parts file and its build data (`--custom` points at another); without it nothing about the stylized run changes. For the realistic body (`assets/characters/realistic/README.md`):

```bash
.bpy/bin/python -I assets/characters/humanoid/qa/measure.py --package realistic --out docs/development/character-studio/co4/measurements.json --check assets/characters/realistic/qa/thresholds.json
.bpy/bin/python -I assets/characters/humanoid/qa/render_baseline.py --package realistic --out docs/development/character-studio/co4
```

- A package without a wardrobe, hair or face shapes skips those sections and sheets, and gets `twist_poses`, `clips` (the starter animations retargeted onto it, as `anim`'s `retarget.rs` does; the last tile is the same take bound by bone name, which the engine no longer does for a body of other proportions), `lods` and `skin_maps` instead (`--only` takes these names too).
- Twist and helper bones (`Realistic.build.json` lists them and their rules) are turned in every pose as `crates/anim/src/twist.rs` turns them in the engine (`Character.drive` in `qa_scene.py`). When that rule changes, change both.
- `twist` in the measurements: a limb turned about its own length (`forearm_twist_90`, `upper_arm_twist_70`, `thigh_twist_45`), with the smallest cross-section area along it, posed over rest (`ring_area_ratio_min`; 1 is no pinching, a limb without twist bones collapses toward the joint).
- A thresholds file may hold `at_least`, `at_most` and `within` limits by a measure's path (`deformation/masculine/elbow_135/ring_area_ratio`), and `targets` with the same keys for what the build is meant to reach but hasn't: `--check` prints unmet targets and still passes on the limits; `--strict` fails on them too.
- A sheet over 600 KB is saved with a 256-colour palette, which bands soft shadows into hard-edged patches (under the jaw, between the fingers): that is the file format, not the mesh.
- A package whose manifest has sliders (the realistic body, CO5): `Character.set_sliders` sets shapes by the manifest's rule (parameters' drivers and corners, as `avatar::resolve`), `set_body_sliders` reshapes the rest pose as `anim`'s body sliders do, and `drive_correctives` leans on the pose-driven shapes the build data lists (`correctives`), as `crates/anim/src/correctives.rs` does. Sheets `build_grid_<frame>`, `build_details_<frame>` and `build_regions_<frame>` (frame `masculine` or `feminine`) show the sliders' corners and regions.
- `sliders` in the measurements (only when named in `--only`; about four minutes): every pair of the manifest's parameters at the ends of their ranges, both frames, in the rest pose, counted for triangles that newly cross (not crossing, and not within 4 mm of touching, in the frame's neutral body), and each shape slider alone for skin in the eyes or teeth.
