# The realistic wardrobe

The garment pipeline of the realistic family (Character Studio R7, milestone CO10) and the items it makes. The first item of each kind is waiting for Paul's sign-off (`docs/development/character-studio/asset-quality.md` §1), so there is exactly one of each and no other is made until then:

| Item | Folder | ID | Approach | Quality |
|---|---|---|---|---|
| Crew-neck T-shirt | `crew-tee/` | `axcenity/realistic-crew-tee` | Pattern (FreeSewing `teagan`), sewn and draped | draft |
| Joggers | `joggers/` | `axcenity/realistic-joggers` | Pattern (FreeSewing `paco` on `titan`), sewn and draped, gathered on elastic | draft |
| Court sneakers | `sneakers/` | `axcenity/realistic-sneakers` | Shell over the foot, a cemented sole | draft |

All three are **drafts**: what each still fails, with numbers, is in `docs/development/character-studio/refinement/wave4/r7-wardrobe-first.md` and in the tests' records (`crates/avatar/tests/realistic_wardrobe.rs`, `crates/player/src/wardrobe_tests.rs`). Drafts are never in a preset or drawn by Randomize (`avatar::randomize` skips an item that says `quality: "draft"`).

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

## Generated garments (`generated/`)

Since 2026-11-24 the wardrobe has a second source: garments from a text-to-3D tool, which Paul supplies (33 files; one is a heavier copy of another and is left out). They are not made on the body, so they come in through the engine's **intake** (`axcenity_avatar::fit::prepare`, ADR-0044 and ADR-0045, `docs/development/character-studio/architecture.md` §9): read from cross-sections, the inner wall taken off, sized and placed by how the garment is worn, turned to the body's rest pose, **drawn in round the body** by how closely that kind of garment follows it (each sleeve and leg round its own limb), thinned to the budget, its cut edges tidied, then fitted like any other import, with its skin weights taken by body part and its **cover map** made (what it hides when worn: the skin under it, and the cloth of a garment worn beneath it).

- `generated/garments.json` lists them: the start of each source file's name, the item's ID, label, slot and layer, the hints the mesh can't give (`kind` when the guess is wrong, `ease`, `cut_for` for garments cut for the feminine frame, `triangles`), and since ADR-0045:
  - `also_fills`: a jumpsuit or a dress in the `top` slot takes `bottom` too, so nothing is worn there under it. A suit (jacket and trousers in one) is `outerwear` that fills `bottom`, so a shirt goes under it; overalls are `bottom` on layer 2, so a base-layer top goes under the bib.
  - `frame`: `masculine`, `feminine` or `unisex`. It becomes a tag and, for the first two, a limit on the body's `frame` (the list's `frames`), which is what Character Studio's "Cut for" and "Fits this body" read.
  - `worn` (2026-11-24, fourth entry): how a top or a jacket is worn with the bottoms. Unset, it hangs **over** them: its cloth round the hips is moved outside every bottom of the library, by `worn_most` (3 cm) at the most. `"tucked"`, with `layer` 0, is for a top that lies on the body (the tank top, the polo, the off-shoulder top): drawn in under the bottoms and hidden under them from the waistband down. A dress or a one-piece (`also_fills` `bottom`) is worn with no bottom and has neither.
  - `conform`: how closely it follows the body, after Paul's fitting profiles: `strong` (leggings), `body` (T-shirts, tank tops, polos, snug dresses), `moderate` (shirts, jeans, trousers, shorts), `partial` (sweaters, sweatpants, wide legs), `structured` (jackets, coats, suits), `none`.
- `generated/import_generated.py` imports them into a library:

  ```bash
  cargo build -p axcenity-avatar --release --example import_garment
  cp -r assets/characters <library>
  python -I assets/characters/realistic/clothing/generated/import_generated.py \
      --source <folder of the .glb files> --library <library>
  ```

  About a quarter of an hour for all 32 on 32 cores, four at a time (most of it the fit against the underwear at every body extreme). `--only <text>` imports the ones whose ID has the text.

  **Bottoms first.** The import runs in two rounds: what is fitted to the body alone (the bottoms among them), then the tops and jackets, each fitted for the bottoms then in the library (`--worn-with`, `axcenity_avatar::fit::Hang`). A top whose bottom isn't in the library has it imported first; a bottom imported again has every top fitted for it imported again after it (`--no-dependents` leaves them, out of date). `<library>/generated.worn-with.json` records which bottoms (by their models' SHA-256) each top was fitted for, and `import_generated.py --library <library> --check` fails when a top is out of date: a bottom added, changed or taken out. The wardrobe pack's `build_pack.py` runs that check before it packs.
- `cargo run -p axcenity-avatar --release --example cover_map -- <library> [<part of an ID>]...` makes items' cover maps again from their fitted models, without fitting them again (seconds for all of them): after the rule for cover maps changes, or for an item imported before there were any.
- `cargo run -p axcenity-avatar --release --example read_garment -- <file.glb>` prints how a file reads (its loops by height, trunk, sleeves, legs and the kind guessed): the first thing to look at when one comes out wrong.

**Neither the source files nor the fitted items are in the repository** (1.2 GB and about 650 MB). The fitted items are downloaded from a repository of their own, [Axcenity-Wardrobe](https://github.com/Axcenity-Studios/Axcenity-Wardrobe), never hosted here: its `tools/build_pack.py` takes them from the library this import filled, and its `install.py` downloads the ones asked for into a project's `assets/characters/`; Character Studio's Clothing page lists and downloads them too (**Online wardrobe**, `docs/editor/character-studio.md`). Every item made is a `draft`, recorded with the `author` and `licence` at the top of `garments.json` (Axcenity, CC BY 4.0: Paul's decision, 2026-11-24; the generated models are his). Captures and what each garment still fails are in `docs/development/character-studio/progress.md` (2026-11-24).

## Making a new garment for this wardrobe

The workflow ADR-0045 settles (research: `docs/research/character-clothing-workflow.md`):

1. **Generate or model it in an A-pose**: upright, facing +Z, arms about 45 degrees from the body, legs apart, one garment a file. A garment modelled with the arms hanging has its sleeves joined to its sides, which is where the tears under a raised arm come from. Where a design differs by frame, make one per frame.
2. **List it** in `generated/garments.json` (slot, layer, `also_fills`, `frame`, `conform`, and `kind` or `ease` only when the reading guesses wrong: `read_garment` shows the reading).
3. **Import it** (`import_generated.py --only <name>`), then **look at it** with the capture tool's body suite (`AXCENITY_CAPTURE_SUITE=body`, `crates/player/src/character_capture.rs`) on a slim, a default and a heavy body of its frame, alone and with one garment of each slot it overlaps.
4. It is a `draft` until it passes; the faults it has are written beside it.

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
