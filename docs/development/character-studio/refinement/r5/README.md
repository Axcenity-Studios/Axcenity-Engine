# R5: skin shading (`r5-skin-shading`)

In-engine captures of the benchmark cast (average, heavyset, athletic; both frames), before (the `refinement-r1-wave1` renderer and content) and after this branch with its review fixes (the Skin shading model, the halved grain, the smooth and defined normal maps; re-captured 2026-11-17 after the second review's fix), at 1280 × 720, under the capture tool's five lights, with TAA (the capture default) and with MSAA 4× (`AXCENITY_CAPTURE_AA=msaa`, added for this). What changed: [../r2/r5-skin-shading.md](../r2/r5-skin-shading.md); the model: [docs/rendering/skin.md](../../../../rendering/skin.md).

| Sheet | What it shows |
|---|---|
| `terminator-masculine-hard-taa.png`, `-msaa.png`, `terminator-feminine-hard-taa.png`, `-msaa.png` | The face under the hard light: the stippled line where light ends on forehead, nose and chin, before and after |
| `skin-close-masculine.png`, `skin-close-feminine.png` | The close skin view under studio and hard light, TAA and MSAA, before then after |
| `bellies-hard-front.png` | Heavyset and average before and after, then the athletic body after: abdominal lines only on the athletic, the navel on every body |
| `navel-hard-front.png` | The same, the belly at 2× |
| `lights-face-masculine.png`, `lights-face-feminine.png` | Face-front under studio, hard, warm, cool and outdoor, before (top) and after |
| `lights-front-cast-after.png` | Average, heavyset and athletic, both frames, front, every light, after |

Made by `pair.py`-style crops of the captures (the full captures stay outside the repository; the commands are in the hand-back).

## What the lead should see

- The stipple across the terminator is gone under TAA and MSAA: the shadow edge on the forehead, nose and chin is a soft, slightly warm gradient.
- No saturated red: the ear's helix fold, the nostril's corner, the crease under the lit eye and the nose's shadow on the upper lip are skin-coloured again (the first review found them blood-red). Counted on the face-front crop under the hard light, outside the lips: pixels with red over twice green were 0 before R5, 532 to 594 at the review, 22 to 54 after the fixes (red over green at most 2.2 to 2.4); under studio, cool and outdoor light 0 (the feminine faces 12 to 18 under cool light, at 2.0).
- The skin reads as skin, not orange peel: the pores no longer reach diffuse light; close up, the grain's cells still show faintly in the highlights (a fine polygonal network on the forehead and cheek).
- Heavyset bellies have no abdominal lines; the athletic bodies keep theirs, more softly than before. Every body has its navel (heavyset and average lost it at the first review): a dimple under the studio, warm, cool and outdoor lights; in the shadowed half under the hard light it is dark, as before R5 (the second review found it glowing orange there).
- Residual: the lips in shadow are deep red (their albedo, as before R5).
