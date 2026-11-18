# Realistic hair (Character Studio R6, CO11)

The realistic family's hairstyles: cards grown from a groom on the realistic scalp, with an atlas drawn from the same strands, shaded by the engine's Hair model ([docs/rendering/hair.md](../../../../docs/rendering/hair.md)). One style so far, a **short textured crop**, because Paul signs off the first style before any other is made ([asset-quality.md](../../../../docs/development/character-studio/asset-quality.md) §1). It is a **draft** (`quality: "draft"`): it doesn't yet pass every check of asset-quality §2 and §4 (see the R6 hand-back, `docs/development/character-studio/refinement/wave4/r6-hair-first.md`).

| File | What it is |
| --- | --- |
| `styles.py` | The styles as data (hairline, ramps, length over the head, comb, lift, thickness, wave, card layers, the head ranges a style holds) and the hair colour presets every style carries |
| `scalp.py` | The scalp read from the built body: the head's frame, the ears, the hairline as a signed distance on the skin and its density ramp, and the **binding** that carries the body's skinning and shapes onto hair |
| `groom.py` | The groom: roots by the density map (dart throwing, per layer), the comb field (away from the crown's whorl, down at the sides and back, swept at the front), length and thickness over the head, and each strand walked over the scalp |
| `atlas.py` | The atlas: strands drawn into islands and baked to the Hair model's layout (colour and coverage, the data map, the flow map, a normal map) |
| `cards.py` | Cards along the strands (per layer), the cap (the scalp lifted, mapped to the atlas's dense islands with the hairline's ramp), and the three levels |
| `glb.py` | Writes the model: the cards skinned to a copy of the body's skeleton, the shapes as sparse morph targets, the Hair material and its maps |
| `build_hair.py` | The build: style → groom → atlas → cards → binding → `ShortCrop.glb`, `ShortCrop.build.json` and `../items/realistic-hair-short-crop.item.json` |
| `qa_hair.py` | The measurements of asset-quality §4 that the model answers: counts, volume, coverage and the hairline's ramp, texel density, hair inside the body at rest, at the head's parameter ends and in Idle, Walk and Run |
| `ShortCrop.glb` | The first style (generated) |
| `ShortCrop.build.json` | What the build made: triangles per level and layer, the shapes carried, the atlas's islands |

## Building

Needs the `bpy` module (Python 3.13 for bpy 5.x), NumPy and Pillow, and the built body (`../RealisticParts.glb`): build the body first.

```bash
python -I assets/characters/realistic/hair/build_hair.py --style short-crop
python -I assets/characters/catalogue.py
python -I assets/characters/realistic/hair/qa_hair.py --style short-crop --out measurements.json
```

About 45 seconds (the atlas 15, the cards and the binding the rest); the measurements about a minute. Deterministic: every random draw is seeded from the style's seed and the layer's name.

## How a style is made

1. **The scalp and the hairline.** The head's centre is between the ears (the vertices `Face_Ear_Size_*` moves), 2 cm up. A style's hairline is an elevation against azimuth round that centre (the front about 6 cm above the brows, the temples a little recessed, a sideburn to the ear's middle, over the ear `ear_clear` off it, down to the nape), plus the ears' clearance. Its zero line is found on the body's triangles and the signed distance is the distance to that line on the skin, so it is right where the line runs up and down (temples, sideburns). The density is 0 at the line and 1 past the ramp (`ramp_front` 11 mm at the front and temples, `ramp_side` 8 mm elsewhere).
2. **Roots and strands.** Each card layer throws roots at its spacing and keeps them by the density (squared) and its region (`all`, `top`, `sides`, `hairline`). A strand walks over the scalp from its root along the comb for its length, standing off the scalp by the layer's share of the thickness (rising from the root, settling toward the tip, or flicking up), waving across, and stopping at an ear or at the hairline (the dense layers inside it, staggered; the fringe up to 4 mm past it).
3. **The atlas** (2048²) has two periodic islands for the cap (the dense interior, strands running away from the crown; the band, dense where it meets the cap and thinning to single hairs at the hairline) and tiles for each card kind (coverage, clumps, detail, short, hairline, sparse, flyaways). Strands are drawn at four times the size, each segment numbered, and every channel is looked up from the winning segment: colour by melanin per strand (a medium brown, eumelanin 0.55 and pheomelanin 0.12, about sRGB (72, 51, 33), spread ±30%), coverage (clear texels exactly 0, islands 8 px apart), root to tip, a value per strand, the streak mask (clusters, about 16%), occlusion (deeper strands and roots darker), the flow and a normal map of the strands' relief (the engine makes tangents only where a material has a normal map, and the Hair model lights along them).
4. **Cards and the cap.** A card is a strip along its strand, the strand's width wide, narrowing to the tip, turned a few degrees from lying flat, its normals the scalp's (so cards shade as one surface and highlights run over the head, not card by card), mapped root to tip down a tile; a top clump has a second card turned up across the first. Non-root vertices are kept 1.2 mm off the body; roots sit up to 0.7 mm in it. The cap is the scalp's triangles inside the hairline, lifted 0.8 mm.
5. **Levels.** Hero (the full object: every layer), gameplay (`_LOD1`: each layer thinned by `keep`, no flyaways) and crowd (`_LOD2`: the cap on the body's own `Body_LOD2`, projected onto the full surface and lifted 1.6 mm more, and the coverage cards, coarser). The engine picks a level by the hair part's own size on screen.
6. **Following the head.** Every vertex is bound to the nearest point of the body's surface, with its offset in that point's frame; it takes that point's bone weights (four at most) and every body shape that moves it by more than 0.2 mm, the offset turned with the surface's normal (fit by the head's own shapes, as the realistic underwear, `fit: shapes`; not `avatar::fit`'s binding, which is for garments made apart from the body). The engine drives same-named shapes on a worn part from the body's weights, so identity, the frame, fat and the face's expressions near the hairline all carry over; the head's size is a bone scale, which the skinning follows.

## The item

`items/realistic-hair-short-crop.item.json`, written by the build: a `groom` in the `hair` slot for `axcenity/realistic`, `cards` only (no strands, no mesh tier: the crowd level is the far copy), one material `Hair` offering every Hair value (its atlas has the root-to-tip and streak channels, so roots and highlights work; a test holds that a style offers only what its atlas carries), the colour presets as variants (`styles.COLOURS`: black, dark brown, brown, auburn, ginger, blonde, platinum, grey; and two colourings with dark roots and with highlights), the shapes it follows, the frames and head ranges it was measured on as `limitations`, and `quality: "draft"`.

## Licence

Everything here is made by these scripts from the body built in this repository (class 5, CC0-1.0); no third-party hair asset, texture or dataset is used.
