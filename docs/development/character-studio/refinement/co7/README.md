# CO7, first slice: skin values drawn, character shadow artefacts

Captures for RA7's first slice of CO7 (branch `co7-skin-values`, hand-back in [../r1/co7-skin-values.md](../r1/co7-skin-values.md)). Every picture is from the in-engine capture (`capture_characters`; 1280 × 720, High, TAA, 48 frames, the unchanged `studio` and `hard` lights) on an RX 9070 XT. "Before" is `5b650d5d` (the refinement baseline's renderer); "after" is this branch.

## Skin values

The description files set one value on the realistic `Skin` material and nothing else (`"materials": {"Skin": {"melanin": 0.8}}` and so on); they were kept in scratch, not committed.

| File | What it shows |
|---|---|
| [tones-masculine-studio-face-front.png](tones-masculine-studio-face-front.png) | Masculine default; `melanin` -0.8 and 0.8, `redness` 0.6, `undertone` -0.7 and 0.7; studio light, face-front view |
| [tones-masculine-hard-face.png](tones-masculine-hard-face.png) | The same under the hard light, face view |
| [tones-feminine-studio-face-front.png](tones-feminine-studio-face-front.png) | Feminine default, the same five values, studio, face-front |
| [tones-feminine-studio-front.png](tones-feminine-studio-front.png) | Feminine default, the same, whole body (front view) |

What the lead should see, and what I saw:

- `melanin` 0.8 is a deep brown near the bottom of the palette (about depth 9, L* near 20). The lips darken with the skin and nearly vanish: they are in the same albedo and the ratio is one per material (expected for this step; per-texel masks wait for the Skin model). The specular stays the same, so the deep skin reads glossier, which the skin references say is right.
- `melanin` -0.8 is very fair and leans **lavender-pink**, cooler than a natural fair skin. Past the table's light end the curve keeps the end's slope, which lightens blue most. The table is indicative (references/skin §1.3); recomputing it from the pigment spectra should give a peachier light end.
- `redness` 0.6 is a visible, even flush. `undertone` -0.7 is pinker and greyer, +0.7 more golden. Both are modest, as the palette's columns are.
- Before this change all five files drew exactly the default (the values were stored and not drawn).

## Shadow and shading artefacts

| File | What it shows |
|---|---|
| [before-after-feminine-walk-hard-front.png](before-after-feminine-walk-hard-front.png) | `Walk@0.4`, hard light, front: before, diagonal hatching on the left flank behind the arm; after, gone |
| [before-after-masculine-walk-flank.png](before-after-masculine-walk-flank.png) | The masculine stride's flank and upper arm, 3× (before, after) |
| [before-after-masculine-hard-face.png](before-after-masculine-hard-face.png) | Masculine face, hard light, 2×: the terminator on the forehead, nose and cheek before and after |
| [sheets/](sheets/) | Contact sheets of both default bodies before and after (all six views, both lights) |

## Evidence (experiments, not shipped)

Made with temporary switches in the shader (removed before the commit), on the masculine default:

| File | Finding |
|---|---|
| [evidence/hatching-is-contact-shadow-noise.png](evidence/hatching-is-contact-shadow-noise.png) | The hatched limb shadow: present with the old fixed noise ("49 original", "1 static noise"); gone with contact shadows off ("3", "2"); gone with the noise moved per frame under TAA ("0 all new", "16"). It is contact shadows' step jitter, a fixed pattern that TAA kept |
| [evidence/speckle-is-the-normal-map.png](evidence/speckle-is-the-normal-map.png) | The speckled terminator and grain: still there with every shadow off ("6 no shadows"); gone with the normal map ignored ("8 flat normal"). It is the skin normal map's grain lit by plain n·l |
| [evidence/terminator-fixes-forehead.png](evidence/terminator-fixes-forehead.png), [evidence/terminator-fixes-chest.png](evidence/terminator-fixes-chest.png) | Original; all fixes; without the bump shadowing; without specular anti-aliasing (tried, measured 3% less grain, not kept) |
| [evidence/chest-band.png](evidence/chest-band.png), [evidence/chest-band-casters.png](evidence/chest-band-casters.png) | OPEN-09, the chest band under the studio light: it stays with the body left out of the shadow map ("64 body casts none"; contact shadows draw it too) and goes with every shadow off ("6"). Its edge follows the mesh's facets: light grazing the upper chest, the shadow terminator of a coarse mesh with smooth normals. Not fixed |

Numbers (`grain`: mean absolute luminance difference between neighbouring pixels, linear light), masculine `hard-face`:

| Region | Before | After | Flat normal map |
|---|---|---|---|
| Terminator band on the forehead (560, 170, 70 × 120) | 0.00736 | 0.00505 (-31%) | 0.00087 |
| Lit forehead (640, 170, 100 × 80) | 0.02339 | 0.02336 | 0.00534 |

The lit grain is the content's: the skin normal map's cells are about 2.6 texels across (`assets/characters/realistic/skin.py`, the grain bump), near one pixel in the face view, and lit with no subsurface softening.
