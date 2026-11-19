# The realistic wardrobe

The garment pipeline of the realistic family (Character Studio R7, milestone CO10) and the items it makes. The first item of each kind is waiting for Paul's sign-off (`docs/development/character-studio/asset-quality.md` §1), so there is exactly one of each and no other is made until then:

| Item | Folder | ID | Approach | Quality |
|---|---|---|---|---|
| Crew-neck T-shirt | `crew-tee/` | `axcenity/realistic-crew-tee` | Pattern (FreeSewing `teagan`), sewn and draped | draft |
| Joggers | `joggers/` | `axcenity/realistic-joggers` | Pattern (FreeSewing `paco` on `titan`), sewn and draped, gathered on elastic | draft |
| Court sneakers | `sneakers/` | `axcenity/realistic-sneakers` | Shell over the foot, a cemented sole | draft |
| Crew-neck T-shirt (licensed) | `tee-crew/` | `axcenity/realistic-tee-crew` | Licensed base (BlenderKit's Plain T-shirt, CC0) through the intake (`../import/`, `items/tee-crew.intake.json`): turned hems, a rib band, maps from Poly Haven's Cotton Jersey | draft (wave 5) |

All of them are **drafts**. The licensed tee (wave 5, Paul's decision 11) is made by the licensed-asset intake, not by the pipeline below: rebuild it with `python -I assets/characters/realistic/import/intake.py --spec assets/characters/realistic/import/items/tee-crew.intake.json --downloads <its download folder> --work <scratch> --out assets/characters` and `AXCENITY_FABRIC_TILES=<tiles>`; its provenance record is `tee-crew/Shirt_TeeCrew.provenance.json`, and what it fails is in `docs/development/character-studio/refinement/wave5/tee-licensed.md` and `crates/player/src/tee_crew_tests.rs`. The first three: what each still fails, with numbers, is in `docs/development/character-studio/refinement/wave4/r7-wardrobe-first.md` and in the tests' records (`crates/avatar/tests/realistic_wardrobe.rs`, `crates/player/src/wardrobe_tests.rs`). Drafts are never in a preset or drawn by Randomize (`avatar::randomize` skips an item that says `quality: "draft"`).

Each folder holds the item's model (`<Object>.glb`: the garment and its two simpler copies `<Object>_LOD1`, `_LOD2`, skinned to the realistic skeleton, with the body's identity and corrective shapes as morph targets, and the 2048 px maps on Cloth or Standard materials), its binding (`<Object>.binding.json`), a thumbnail and its manifest. They are generated: rebuild them with the commands below, never edit them.

## How an item is made

1. **Measure** (`body.py`, Blender's Python): the realistic masculine body as a tailor measures it, in FreeSewing's measurements (chest, waist, seat, neck, biceps, the arcs, the cross seam, the heights...), and the landmarks the drape places pieces by.
2. **Draft** (`teagan.py`, `paco.py`, on `freesewing.py`): FreeSewing's designs ported to Python, line for line comparable with the JavaScript (MIT; the notice is in `freesewing.py`). The options each item changes are inside the design's own ranges and named in its build script (the tee: length +40%, neckline depth 20% and width 10%; the joggers: length -11%, heel ease 35%, waist height 50%, waistband 3.5%).
3. **Grid** (`panels.py`): each piece is a grid of quads spanned by its four sides (a Coons patch), in the pattern's millimetres, so the texture coordinates are the pattern itself (the straight grain up each piece). A piece is gridded finely (10 or 11 mm) for the simulation and again at each level's spacing.
4. **Sew and drape** (`tee.py`, `joggers.py`, `drape.py`): the pieces start round the body as long as they were cut (rolled round an ellipse or a tube along their rows, never stretched: Blender's cloth takes its starting lengths as its rest), corners closer than 1 cm to the skin pushed out, each seam a row of sewing springs; Blender's cloth simulation (quality 10, 200 frames, gravity eased in over the first 60 to 70) sews and drapes them on the body in its rest pose. The joggers' waist and hems start on elastic rings round the waist and ankles and are held there (pinned): the gather. Sewn corners are joined at their middle.
5. **Finish** (`finish.py`): each level is carried onto the fine drape by its place in the pattern; seams are welded exactly; edges are turned hems (a rounded fold `THICK` (3 mm) thick, the turn-up lying inside for 20 to 22 mm) or bands (the tee's rib neck band, the joggers' waistband casing and cuffs: standing past the edge, folded, carried back down inside past the seam), so no edge is one layer; bands are moved out of the body column by column. The joggers get a drawcord with aglets out of two grommets. The atlas packs every piece (and each hem's turn-up, below its piece) at one texel density.
6. **Bake** (`textures.py`): base colour (the fabric's light and shade, its colour taken out: the item's tint gives the colour), a tangent-space normal map (the fine drape's folds over the level's surface, kept within 25 to 50 degrees of it; seam valleys; stitch rows; the fabric's own height, tiled at its photographed size along the grain; a rib's wales), occlusion, roughness and metal.
7. **Fit** (`wardrobe.py`): each level is imported by the engine's own import (`cargo run -p axcenity-avatar --release --example import_garment`: binding, the body's shapes, push-out from the body and the items it is worn over, region hiding), in wearing order (the joggers over the underwear, the tee over the sports bra and the joggers' waistband, then the shoes), in a scratch copy of the library. The three fitted levels and the maps are put together into one model with cloth materials (`KHR_materials_sheen`, `docs/rendering/cloth.md`), and the manifest is written (colour roles, variants, `quality`, `source`).

The sneakers are a shell, not a drape (`build_sneakers.py`): the upper is one ring of quads from the sole line (tucked into the midsole) to the collar opening, each column laid over the foot with room at the toes and kept clear of the skin, smoothed; the collar is rolled over a pad and lined; a padded tongue sits under five lace rungs; the midsole and outsole are a closed solid with a flat tread. The toe cap and heel counter overlays, their stitching and the punched eyelets are drawn by the maps.

## Rebuilding

Blender's Python (bpy 5.x with NumPy and Pillow) for the builds; plain Python for the rest. `<tiles>` is a folder of the CC0 fabric tiles, `<built>` a scratch folder outside the repository:

```bash
python -I assets/characters/realistic/clothing/fabrics.py --out <tiles>
python -I assets/characters/realistic/clothing/build_tee.py --body assets/characters/realistic/RealisticParts.glb --tiles <tiles> --out <built>/crew-tee
python -I assets/characters/realistic/clothing/build_joggers.py --body assets/characters/realistic/RealisticParts.glb --tiles <tiles> --out <built>/joggers
python -I assets/characters/realistic/clothing/build_sneakers.py --body assets/characters/realistic/RealisticParts.glb --tiles <tiles> --out <built>/sneakers
python -I assets/characters/realistic/clothing/wardrobe.py --built <built>
python -I assets/characters/catalogue.py
```

Each build takes one to three minutes (the tee's drape about 45 s on 16 cores); the imports about a minute in all. The cloth simulation is deterministic for one Blender version and machine; another may drape a few millimetres differently. A body change (`RealisticParts.glb`) means a rebuild: the items carry the body's shapes.

## Sources and licence

The garments are ours, released CC0 1.0 (class 5), drafted after FreeSewing's patterns (MIT, class 2: the notice is kept in `freesewing.py`) and textured from CC0 photographs (class 1): Poly Haven's Cotton Jersey, Jogging Melange and Leather White, ambientCG's Rubber004. The tiles themselves are not stored here. The full record is in `docs/development/character-studio/references/README.md`, "Sources of the first realistic wardrobe".
