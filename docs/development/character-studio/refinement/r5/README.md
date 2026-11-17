# R5: skin shading (`r5-skin-shading`)

In-engine captures of the benchmark cast (average, heavyset, athletic; both frames), before (the `refinement-r1-wave1` renderer and content) and after this branch (the Skin shading model, the halved grain, the smooth and defined normal maps), at 1280 × 720, under the capture tool's five lights, with TAA (the capture default) and with MSAA 4× (`AXCENITY_CAPTURE_AA=msaa`, added for this). What changed: [../r2/r5-skin-shading.md](../r2/r5-skin-shading.md); the model: [docs/rendering/skin.md](../../../../rendering/skin.md).

| Sheet | What it shows |
|---|---|
| `terminator-masculine-hard-taa.png`, `-msaa.png`, `terminator-feminine-hard-taa.png`, `-msaa.png` | The face under the hard light: the stippled line where light ends on forehead, nose and chin, before and after |
| `skin-close-masculine.png`, `skin-close-feminine.png` | The close skin view under studio and hard light, TAA and MSAA, before then after |
| `bellies-hard-front.png` | Heavyset before and after, then the average and athletic bodies after: abdominal lines only on the athletic |
| `lights-face-masculine.png`, `lights-face-feminine.png` | Face-front under studio, hard, warm, cool and outdoor, before (top) and after |
| `lights-front-cast-after.png` | Heavyset and athletic, both frames, front, every light, after |

Made by `pair.py`-style crops of the captures (the full captures stay outside the repository; the commands are in the hand-back).

## What the lead should see

- The stipple across the terminator is gone under TAA and MSAA: the shadow edge on the forehead, nose and chin is a soft, slightly warm gradient.
- The skin reads as skin, not orange peel: the pores no longer reach diffuse light; close up, the grain's cells still show faintly in the highlights (a fine polygonal network on the forehead and cheek).
- Heavyset bellies have no abdominal lines; the athletic bodies keep theirs, more softly than before (diffuse light now sees the defined map blurred).
- Residual: a small saturated red spot where the nose's shadow meets the upper lip and in the nostril's corner (high curvature plus the redder penumbra), and the heavyset navel is faint.
