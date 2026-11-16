# Stage 3 close: measures after Paul's decisions (2026-11-16)

The realistic package measured on the Stage 3 branch merged with `main`, after the two decisions in [progress.md](../progress.md) (the raised shoulder held by its ring area; distances signed by pseudo-normals).

| File | What |
|---|---|
| `measurements.json`, `measure-check.txt` | `measure.py --package realistic --check assets/characters/realistic/qa/thresholds.json`: every limit holds; the 90° shoulder targets print as not met; the sliders lines say "not measured" in this mode |
| `sliders.json`, `sliders-check.txt` | `measure.py --package realistic --only sliders --check …`: 0 new crossings in 10,082 bodies on each frame; 0 parameter ends with eye or teeth points in the flesh |
| `eye_points.py`, `eye_points.json` | The diagnostic for the eye check's counts: for each eyeball point the old or new sign puts in the lid, how deep by each, the nearest feature of the body (face, edge, corner; open or shared) and four rays' parity. With exact regions only CO6's three points remain, all outside the lid (4.7, 2.5, 2.4 mm) by both the pseudo-normal and the rays |

Run with Blender 5.2 (the Flatpak), from the repository's root:

```text
flatpak run --filesystem=$PWD --cwd=$PWD org.blender.Blender -b --factory-startup --python-exit-code 1 \
  --python $PWD/assets/characters/humanoid/qa/measure.py -- --package realistic \
  --out $PWD/docs/development/character-studio/stage3/measurements.json \
  --check $PWD/assets/characters/realistic/qa/thresholds.json
```

The Flatpak starts in its own folder, so every path is absolute. The sliders run takes 15 to 40 minutes.
