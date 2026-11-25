# The human base (`axcenity/human`)

The realistic family's new body: **MakeHuman's base mesh, targets and clothes**, brought onto the Axcenity Humanoid skeleton by driving the **MPFB** Blender add-on ([ADR-0047](../../../wiki/Architecture/Architecture-Decisions/ADR-0047-The-Realistic-Family-Is-Rebuilt-On-MakeHumans-Body-And-Wardrobe.md), Paul's decision of 2026-11-24). It ships beside `axcenity/realistic`, which it replaces once it has what that base has (below, "Not yet"). Everything here is generated: change the scripts, not the files.

| File | What |
|---|---|
| `HumanParts.glb` | The body and its parts (`Body`, `Eyes`, `Teeth`, `Tongue`, `Brows`, `Lashes`) under the armature `AxcenityHuman`, with the body's shapes as morph targets |
| `Human.base.json` | The base model manifest (`manifests.py`; schema in `docs/development/character-studio/architecture.md` section 6) |
| `HumanRegions.png` | The body's region map |
| `Human.build.json` | What the build measured (joints, shape reaches, versions) |
| `items/` | The underwear a new character wears, cut from the body (`underwear.py`): boxer briefs, and a bandeau top for the feminine frame |
| `wardrobe/` | Four MakeHuman garments (two casual outfits, two pairs of shoes), with `CREDITS.md` |

## How it is made

`build.py` (in Blender, with MPFB installed):

1. **The body**: MakeHuman's base mesh (`hm08`) at the masculine middle of its macro controls, made by MPFB.
2. **The shapes**: the body sampled at each corner of the controls the base offers, as differences (`build.SHAPES`): `Frame_Feminine`, and per frame muscle up and down, fat up and down, their four corners, and old age. MakeHuman has a target per gender, muscle and weight, so a frame's shapes are its own and the manifest drives them by the product of the sliders (`manifests.corners`).
3. **The pose**: MakeHuman rests with the arms about 48 degrees down and the elbows bent forward. `mh.Repose` turns the arms straight to the engine's 45 degree A-pose with MakeHuman's own weights, and every shape's moves turn with their corners.
4. **The skeleton**: the stylized package's bones and rolls (`../humanoid/rig.py`), so every clip plays. Joints are the middles of the **joint cubes** the base mesh carries (`mh.joint_groups`), which follow every target. MPFB 2.0.18's own rig fit isn't used for joints: under Blender 5.2 it leaves every other bone of a chain on the unshaped mesh. Its default rig's **weights** are used, gathered onto the Humanoid bones (`build.humanoid_bone`); the halves MakeHuman splits each limb into become the twist bones.
5. **The parts** (eyes, teeth, tongue, brows, lashes) are MakeHuman clothes-type assets: each corner hangs on the body by its `.mhclo` file, so a part's shapes are what that file makes of the body's.
6. **Standing**: 1.75 m tall on the floor, metres, +Z front after export.
7. **The face** (`face.py`), from MakeHuman's targets, read as data from the add-on's `data/targets`:
   - **What it does**: MakeHuman's 34 expression units become 30 blend shapes named after the engine's standard facial controls (`Blink_Left`, `Mouth_Smile_Right`, ...), which is what the built-in `axcenity` face profile looks for, so `FaceControls`, expressions, blinking and lip sync work with no profile file. A unit that moves both sides of the mouth is split into a left and a right shape. The jaw opens by its bone. Thirteen controls have no unit behind them (`face.UNMAPPED`: the brows drawn together, the mouth and jaw sideways, cheek raise and puff, the tongue).
   - **Who it is**: MakeHuman's own slider list (`target.json`) for the head, forehead, brows, eyes, nose, cheeks, mouth, chin, ears and neck becomes 149 identity parameters in ten groups, each driving its pair of opposite targets as shapes, mirrored where the feature is paired. A shape that moves an eye or the jaw moves that bone with it. Labels are made from the targets' names and read mechanically ("Nose width 2").
8. **The underwear** (`underwear.py`): boxer briefs and a bandeau, the body's own skin between smooth hem lines, cut from the skin's triangles so each cloth face lies over the skin triangle it came from, stood 3 mm off (5 mm at the hems) and relaxed outward, with a rim turned in at each hem. A corner is a fixed mix of two body corners, so the garment takes the body's weights and shapes. The bandeau is laid out on the feminine frame.

`wardrobe.py` does the same for garments and adds what an item needs: a licence check, a slot and layer from the garment's name and its pack's category, a cover map from the body corners its file hides (`delete_verts`), a thumbnail, a manifest with `fit: shapes`, and `CREDITS.md`. A garment follows every body slider exactly as MakeHuman fits it, with nothing solved at run time; the proxy fit of ADR-0046 isn't involved.

## Rebuilding

Blender 5.2 with the MPFB extension installed (Preferences, Get Extensions), run **without** `--factory-startup`. `<system>` is the unpacked "MakeHuman system assets" pack and `<pack>` any unpacked asset pack, both from <https://static.makehumancommunity.org/assets/assetpacks.html>; neither is stored here.

```bash
blender -b --python assets/characters/human/build.py -- --assets <system> --out assets/characters/human --check
blender -b --python assets/characters/human/wardrobe.py -- --assets <system> --system <system> \
    --out assets/characters/human/wardrobe \
    --only male_casualsuit01.mhclo --only shoes01.mhclo --only female_casualsuit01.mhclo --only shoes04.mhclo
python -I assets/characters/catalogue.py
python -I assets/characters/benchmarks/make.py
```

The build takes about 8 seconds and a garment about a third of a second. `--check` compares this package's reading of each clothes file with MPFB's own fit (they agree to 0.0003 mm). A whole wardrobe goes into a library outside the repository:

```bash
blender -b --python assets/characters/human/wardrobe.py -- --assets <system> <pack>... --system <system> \
    --out <library>/human/wardrobe
```

On 2026-11-24 seven packs (system assets, Underwear 01 to 04, Shirts 01, Pants 01) gave 86 garments in 30 seconds and left 10 out.

## The licence gate

A garment is taken only when **its own file's header and its pack's metadata both say CC0 or CC BY and agree** (`wardrobe.vet`). Left out, with the reason in `wardrobe.report.json`: files whose headers still say AGPL3 (the old MakeClothes default line, found in packs listed as CC0), files whose header and pack disagree, CC BY with no author, and names of characters or brands that aren't their authors' to release. A pack's file name isn't trusted (`underwear02_cc0.zip` holds CC BY assets). Per-asset licences are per the brief's rule in `docs/development/character-studio/references/README.md`.

## Sources and credit

- **MakeHuman** (<http://www.makehumancommunity.org>): the base mesh, the targets, and the system assets (skins, eyes, teeth, tongue, brows, lashes, the casual outfits and shoes). Released **CC0 1.0** by their copyright holders, Data Collection AB, Joel Palmius and Jonas Hauquier (each file's header: "This asset was explicitly released as CC0 in september 2020"). CC0 asks for no credit; it is given here, in `NOTICE` and in every item's `source`.
- **MakeHuman community asset packs**: each garment's author and licence (CC0 1.0 or CC BY 4.0) are in its item manifest and in the `CREDITS.md` written beside the items. CC BY requires that credit; keep `CREDITS.md` with the items wherever they go.
- **MPFB** (<https://static.makehumancommunity.org/mpfb.html>), by Joel Palmius and the MakeHuman community, GPL-3.0-or-later: the tool that makes the body, loads its rig weights and fits the clothes. **None of its code is in this repository** (the scripts call the installed add-on); what it outputs is the person's who made it (MPFB's `LICENSE.md`, section D, as MakeHuman's). The `.mhclo` and `.mhmat` readers in `mh.py` and `build.py` are written from the formats' published description.
- **Blender** (GPL) runs the scripts; nothing of it ships.

## Not yet

This is the first stage of ADR-0047. Against `axcenity/realistic` the base lacks:

- **Face**: the sliders' labels and descriptions are made from MakeHuman's target names and need writing by hand; none is marked `quick`; there are no features, handles or face presets, so Character Studio lists the face as plain sliders. Thirteen of the 48 standard controls play nothing. MakeHuman's face rig bones are gathered onto `Head` and `Jaw`. The face capture frames the feminine frame low (she is 13 cm shorter).
- **Skin**: one of MakeHuman's photographed skins on the `standard` model; no skin tone controls, makeup or the `skin` shading model. Brows and lashes are on `standard`, not `hair`.
- **Body**: frame, muscle, fat and age only (and the proportion sliders). No regional sliders, correctives or helper bones; no simpler copies (levels of detail).
- **Hair, presets, handles, features**: none.
- **Layering**: garments hide skin by their cover maps, but not each other's cloth.
- **Underwear**: the boxer briefs show a pale speck by each front leg hem; the bandeau's upper hem has small spikes and the cloth follows the chest's surface closely. One dark cloth, no fabric texture.
- **Quality**: every converted garment is a `draft` until reviewed (`docs/development/character-studio/asset-quality.md`).
