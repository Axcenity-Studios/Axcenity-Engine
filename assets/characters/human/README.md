# The human base (`axcenity/human`)

The engine's one character family ([ADR-0050](../../../wiki/Architecture/Architecture-Decisions/ADR-0050-The-Engine-Ships-One-Character-Family-MakeHumans-Body-With-Quaterniuss-Animations.md)): **MakeHuman's base mesh, targets and clothes**, brought onto the Axcenity Humanoid skeleton by driving the **MPFB** Blender add-on ([ADR-0047](../../../wiki/Architecture/Architecture-Decisions/ADR-0047-The-Realistic-Family-Is-Rebuilt-On-MakeHumans-Body-And-Wardrobe.md), Paul's decision of 2026-11-24), with **Quaternius's animations** baked on that skeleton ([ADR-0049](../../../wiki/Architecture/Architecture-Decisions/ADR-0049-The-Starter-Animations-Are-Quaterniuss-Library-Baked-Onto-The-Humanoid-Skeleton.md)). The default humanoid's model and animation file are here too. Everything here is generated: change the scripts, not the files.

| File | What |
|---|---|
| `HumanParts.glb` | The body and its parts (`Body`, `Eyes`, `Teeth`, `Tongue`, `Brows`, `Lashes`) under the armature `AxcenityHuman`, with the body's shapes as morph targets |
| `Human.base.json` | The base model manifest (`manifests.py`; schema in `docs/development/character-studio/architecture.md` section 6) |
| `HumanRegions.png` | The body's region map |
| `skins/` | MakeHuman's 22 photographed skins, one image each (`skins.py`): what the body's `Skin` material draws, chosen by the manifest's `material_maps` |
| `Human.build.json` | What the build measured (joints, shape reaches, versions) |
| `items/` | The underwear a new character wears, cut from the body (`underwear.py`): boxer briefs, and a bandeau top for the feminine frame |
| `wardrobe/` | 51 MakeHuman garments, every CC0 one of the converted library (`take_library.py`), with `CREDITS.md` |
| `hair/` | MakeHuman's ten hairstyles, with `CREDITS.md` (`hair.py`) |
| `Humanoid.glb` | The default humanoid's model, built into the engine: the default body out of `HumanParts.glb` on the same skeleton bone for bone, with its face parts and the boxer briefs, keeping only the face's 30 control shapes (`default_model.py`, plain Python: `glb.py` rewrites the files without Blender) |
| `HumanoidAnimations.glb` | The starter animations, built into the engine: 84 takes of Quaternius's Universal Animation Library keyed on this skeleton, and its walk and jog again with root motion (`animations.py`, `mocap.py`) |
| `presets/` | 30 presets: two characters, six builds, eight faces, seven face shapes, seven noses (`presets.py`) |

## How it is made

`build.py` (in Blender, with MPFB installed):

1. **The body**: MakeHuman's base mesh (`hm08`) at the masculine middle of its macro controls, made by MPFB.
2. **The shapes**: the body sampled at each corner of the controls the base offers, as differences (`build.SHAPES`): `Frame_Feminine`, and per frame muscle up and down, fat up and down, their four corners, and old age. MakeHuman has a target per gender, muscle and weight, so a frame's shapes are its own and the manifest drives them by the product of the sliders (`manifests.corners`).
3. **The pose**: MakeHuman rests with the arms about 48 degrees down and the elbows bent forward. `mh.Repose` turns the arms straight to the engine's 45 degree A-pose with MakeHuman's own weights. A **body shape** is the difference of whole bodies each brought into that pose about **its own joints** (`Build.shape_moves`); a face shape, which moves no joint to speak of, is turned with its corners. Turning a body shape's moves with the default body's bones is not the same thing where two bones turn differently and the joint between them moves: the feminine frame's elbows are 12 cm from the masculine ones, and until 2026-11-25 her forearms came out up to 13 mm off her upper arms, a notch at each elbow.
4. **The skeleton**: the Humanoid bones and rolls (`rig.py`), which the starter animations are baked on. Joints are the middles of the **joint cubes** the base mesh carries (`mh.joint_groups`), which follow every target. MPFB 2.0.18's own rig fit isn't used for joints: under Blender 5.2 it leaves every other bone of a chain on the unshaped mesh. Its default rig's **weights** are used, gathered onto the Humanoid bones (`build.humanoid_bone`); the halves MakeHuman splits each limb into become the twist bones. A parameter's `bones` list every bone a shape moves, the twist bones among them (`build.head_moves`): the engine leaves a bone no change names where it is, whatever its parent does.
5. **The parts** (eyes, teeth, tongue, brows, lashes) are MakeHuman clothes-type assets: each corner hangs on the body by its `.mhclo` file, so a part's shapes are what that file makes of the body's.
6. **Standing**: 1.75 m tall on the floor, metres, +Z front after export.
7. **The face** (`face.py`), from MakeHuman's targets, read as data from the add-on's `data/targets`:
   - **What it does**: MakeHuman's 34 expression units become 30 blend shapes named after the engine's standard facial controls (`Blink_Left`, `Mouth_Smile_Right`, ...), which is what the built-in `axcenity` face profile looks for, so `FaceControls`, expressions, blinking and lip sync work with no profile file. A unit that moves both sides of the mouth is split into a left and a right shape. The jaw opens by its bone. Thirteen controls have no unit behind them (`face.UNMAPPED`: the brows drawn together, the mouth and jaw sideways, cheek raise and puff, the tongue).
   - **Who it is**: MakeHuman's own slider list (`target.json`) for the head, forehead, brows, eyes, nose, cheeks, mouth, chin, ears and neck becomes 149 identity parameters in ten groups, each driving its pair of opposite targets as shapes, mirrored where the feature is paired. A shape that moves an eye or the jaw moves that bone with it. Each slider has a name, a tooltip and a level written by hand (`face.NAMES`: 35 quick, 82 detailed, 44 advanced), from what its target was measured to move, because MakeHuman's names mislead (`eye-push1` is the outer corner, `chin-bones` the jaw's corners).
   - **Pointing at it**: each of the ten pages is a feature a person can click on the face, and 24 handles sit on the skin (`face.FEATURES`, `face.HANDLES`): a handle is put on the corner its shape moves most (an ear's and the neck's on the outline of what the shape moves, so they show from the front), and each slider it drives moves that corner along a direction read from the shape. Each drive lists the framings it can be dragged in and its direction on the screen there (`face._framings`, the realistic base's rules: at least 35% of its travel across the screen, 25 degrees from the handle's other drives); Character Studio draws no handle without them. A slider's name with its side is at most 22 characters, which is what the panel shows uncut. `presets.py` writes the presets.
8. **The underwear** (`underwear.py`): boxer briefs and a bandeau, the body's own skin between smooth hem lines, cut from the skin's triangles so each cloth face lies over the skin triangle it came from, stood 3 mm off (5 mm at the hems), with a rim turned in at each hem. Where the cloth would follow a hollow it is relaxed outward to span it, each corner moving only along the line out of its own skin (a corner let move sideways walked up to 2 cm, which was the notch at the leg hems and the spikes). A garment's edge by its bones (under the arm) is cut through the faces like a hem. A corner is a fixed mix of two body corners, so the garment takes the body's weights and shapes. The bandeau is laid out on the feminine frame.

9. **The skins** (`skins.py`, [ADR-0048](../../../wiki/Architecture/Architecture-Decisions/ADR-0048-A-Base-Models-Material-Draws-A-Library-Colour-Map-Chosen-By-The-Characters-Parameters.md)): every textured skin of MakeHuman's system assets (three ages by three ethnicities by two sexes, a second young Caucasian pair, and a young pair with a bodysuit painted on: 22) is written as a 2048 by 2048 JPEG in `skins/`, 11 MB in all, and **not** into `HumanParts.glb`, which keeps only the default character's as the map drawn until a skin is read. The manifest lists each skin with the parameter values it was made for (`material_maps`), and a character draws the one nearest its own. So a person picks one thing, the `skin` choice (African, Asian, Caucasian, Caucasian 2, Bodysuit: MakeHuman's own names), and the rest follows the body:
   - the **sex** of the skin is the body type's (`frame`);
   - its **age** is the Age slider's: the young skin up to a quarter of the slider, the old one from three quarters, the middle-aged one between;
   - the second Caucasian pair exists young only, so an older character with it draws the first pair's skin of its age;
   - the bodysuit is listed at every age, so it stays on as the Age slider moves, and Randomize never draws it (the choice's `random` span stops before it).

   Between two ages the skin is a mix of the two photographs (each map's `blend`, drawn by the Skin shading model), so nothing snaps as the Age slider moves. The `Skin` material is on the engine's **Skin shading model** (`extras.axcenity_shading`), which gives it light under the surface and the tone controls on the Skin and Makeup page (melanin, undertone, redness, tint, roughness, scatter) over whichever skin is drawn. Brows (MakeHuman's `eyebrow008`, the fullest natural pair) and lashes are on the **Hair model**, cut out by their textures.

   Every MakeHuman skin is laid out on the base mesh's own texture coordinates, so the face and body sliders move the same skin under every choice: lips, lids, nipples and navel sit where the mesh has them on all 22 (looked at in engine captures of each).

`wardrobe.py` does the same for garments and adds what an item needs: a licence check, a slot and layer from the garment's name and its pack's category, a cover map, each corner's place on the skin, a thumbnail, a manifest with `fit: shapes`, and `CREDITS.md`. A garment follows every body slider exactly as MakeHuman fits it, with nothing solved at run time; the proxy fit of ADR-0046 isn't involved.

**Layering** (ADR-0045, on these clothes since 2026-11-25). A cover map is an image over the body's texture coordinates with three channels (`build.write_cover`):

- **red**, skin the body leaves out under the garment: the body corners its file hides (`delete_verts`);
- **green**, skin over which anything worn **beneath** the garment leaves its own cloth out: where a line out of the skin meets the garment's cloth within 10 cm (`build.cloth_over`), set back from the garment's openings by the cloth's distance from the skin there (3 mm to 4 cm, the engine's own rule), and wherever red is lit;
- **blue**, the same for a top tucked into a bottom: for a `bottom`, round the hips, with no setback.

Green and blue are grown 12 texels past the edges of the body's texture islands (never over skin), because the engine counts a corner covered when most of the map round it is lit, and a corner on a seam (the middle of the crotch) has map on one side only. What is under what is the engine's `wearing_order`: the layer, then the slot. The converter gives underwear and legwear layer 0, everything else 1 or 2, and a top whose name says it is tucked layer 0 so that bottoms go over it.

A converted garment's corners carry their place on the skin (`_AXCENITY_SKIN_UV`, the body's texture coordinates at the nearest skin point: `build.skin_uvs_of`), so under another garment the engine cuts it as it cuts one it fitted itself: triangles wholly under the cover go, and those that reach out from under it dip their covered corners into the body. That is what takes the shoes' socks under a pair of jeans. Blender's exporter writes the attribute (`export_attributes`); it must be made after the texture coordinates, or the exporter writes materials that name texture set -1.

The **underwear cut from the body** (`underwear.py`) has no such attribute: it lies on the skin, the engine places it by the skin corner nearest each of its own, and takes out what is well under a cover. It also has a shape, `Shrink_Under`, that presses it to 1 mm off the skin; the engine turns that shape on while anything is worn over it (ADR-0046), so the little of it left at a cover's edge is never what stands through a tight garment. Since 2026-11-25 it is not drawn at all while a garment takes its place: the base's `underwear_bottom` slot is `hidden_by` `bottom` and `underwear_top` by `top` (`manifests.SLOTS`), so what is left for the cover and the shape is what shoes, socks and gloves are worn over. For the same reason the converter files MakeHuman's own underwear and swimwear as a top or a bottom of layer 0 (`wardrobe.KINDS`): the underwear slots are the body type's, and nothing else can be worn in them.

`hair.py` makes hairstyles the same way. A MakeHuman hairstyle is a clothes-type asset too: a mesh of 500 to 4,200 faces with a painted texture, hung on the head and shoulders by its `.mhclo` file. It becomes a `groom` item in the `hair` slot with its mesh as the groom's `mesh` tier:

- It carries the body's and the face's shapes, so it keeps to the head through every slider, but **not** the face's expression shapes (a fringe bound to the brow would rise with it), and no normals with its shapes (they were half of each file).
- Its material is marked for the engine's **Hair shading model** (`extras.axcenity_shading`, `axcenity_hair_shell`) and cut out by the texture's alpha, so its edge is soft under TAA and it casts its shadow through its holes (`docs/rendering/hair.md`).
- Its maps are made from MakeHuman's painted texture (`hair.maps`): the **colour map** is the painted strands brought to one authored brown with the painting's own shading taken out, and the **flow map** is the direction the painted strands run (read from the painting), which is where the Hair model puts its highlights.
- **Colour** is a variant, the same eight on every hairstyle (black, dark brown, brown, auburn, ginger, blonde, platinum, grey): the realistic hairstyle's values, with a dye for the three that need a hue.
- Randomize draws them though they are drafts (the base's hair slot says `random_drafts`), so a random character isn't always bald.
- A new character is bald, as on the realistic base: a hairstyle is picked on the Hair page.

## Rebuilding

Blender 5.2 with the MPFB extension installed (Preferences, Get Extensions), run **without** `--factory-startup`. `<system>` is the unpacked "MakeHuman system assets" pack and `<pack>` any unpacked asset pack, both from <https://static.makehumancommunity.org/assets/assetpacks.html>; neither is stored here.

```bash
blender -b --python assets/characters/human/build.py -- --assets <system> --out assets/characters/human --check
blender -b --python assets/characters/human/wardrobe.py -- --assets <system> <pack>... --system <system> \
    --out <library>/human/wardrobe
blender -b --python assets/characters/human/hair.py -- --assets <system> --system <system> \
    --out <library>/human/hair
python -I assets/characters/human/take_library.py <library>/human
python -I assets/characters/human/default_model.py
blender -b --python assets/characters/human/animations.py -- --out <folder> \
    --mocap <UAL1_Standard.glb> <UAL2_Standard.glb>
python -I assets/characters/catalogue.py
python -I assets/characters/benchmarks/make.py
```

The build takes about 8 seconds and a garment about a third of a second. `--check` compares this package's reading of each clothes file with MPFB's own fit (they agree to 0.0003 mm).

The wardrobe and the hair are converted into a **library outside the repository** (`<library>`, a copy of this package's base files with every garment the converters accept), and `take_library.py` takes its CC0 items into this package. On 2026-11-24 seven packs (system assets, Underwear 01 to 04, Shirts 01, Pants 01) gave 86 garments in 30 seconds and left 10 out; 51 of the 86 are CC0 and are here (211 MB), 35 are CC BY and stay outside. The system assets' ten hairstyles convert in 7 seconds (50 MB).

`default_model.py` makes `Humanoid.glb` from the files here (no Blender). `animations.py` bakes the starter animations in about 20 seconds from the two packs of Quaternius's Universal Animation Library (<https://quaternius.com>, the packs' `Unreal-Godot/UAL*_Standard.glb`, not stored here) and prints, for each take, its frames, how far the feet were from the library's before the hips were corrected, how high the hips ride and how far the take was lifted to keep the joints that lie on the ground out of it; copy the file it writes here and into `crates/samples/projects/humanoid/assets/models/Humanoid/` (with `Humanoid.glb`). Both files are built into the engine (`crates/project/src/default_humanoid.rs`, whose tests hold the model's box and the take count).

## The licence gate

**Only CC0 items are in this repository** (`take_library.py`: the converter's report and the item's own manifest must both say CC0 1.0). The converter itself takes a garment into the outside library when **its own file's header and its pack's metadata both say CC0 or CC BY and agree** (`wardrobe.vet`). Left out, with the reason in `wardrobe.report.json`: files whose headers still say AGPL3 (the old MakeClothes default line, found in packs listed as CC0), files whose header and pack disagree, CC BY with no author, and names of characters or brands that aren't their authors' to release. A pack's file name isn't trusted (`underwear02_cc0.zip` holds CC BY assets). Per-asset licences are per the brief's rule in `docs/development/character-studio/references/README.md`.

## Sources and credit

- **MakeHuman** (<http://www.makehumancommunity.org>): the base mesh, the targets, and the system assets (the 22 skins in `skins/`, the hairstyles in `hair/`, eyes, teeth, tongue, brows, lashes, the casual outfits and shoes). Released **CC0 1.0** by their copyright holders, Data Collection AB, Joel Palmius and Jonas Hauquier (each file's header: "This asset was explicitly released as CC0 in september 2020"). CC0 asks for no credit; it is given here, in `NOTICE` and in every item's `source`.
- **MakeHuman community asset packs**: each garment's author and licence are in its item manifest and in the `CREDITS.md` written beside the items (CC0 1.0 for every one here; CC0 1.0 or CC BY 4.0 in the outside library). CC BY requires that credit; keep `CREDITS.md` with the items wherever they go.
- **Quaternius** (<https://quaternius.com>): the Universal Animation Library and the Universal Animation Library 2, "Standard", released **CC0 1.0**. The 84 takes in `HumanoidAnimations.glb` are theirs, brought onto this skeleton and renamed (`mocap.TAKES` has each one's own name). Credited here and in `NOTICE`.
- **MPFB** (<https://static.makehumancommunity.org/mpfb.html>), by Joel Palmius and the MakeHuman community, GPL-3.0-or-later: the tool that makes the body, loads its rig weights and fits the clothes. **None of its code is in this repository** (the scripts call the installed add-on); what it outputs is the person's who made it (MPFB's `LICENSE.md`, section D, as MakeHuman's). The `.mhclo` and `.mhmat` readers in `mh.py` and `build.py` are written from the formats' published description.
- **Blender** (GPL) runs the scripts; nothing of it ships.

## Not yet

ADR-0047's stages 1 to 3 and 5 are built, with a polish pass on 2026-11-24, and since 2026-11-25 this is the engine's only base (ADR-0050). Against the realistic base it replaced it lacks:

- **Face**: 13 of the 48 standard controls play nothing. The names, handles and presets haven't been reviewed in Character Studio by a person. MakeHuman's face rig bones are gathered onto `Head` and `Jaw`.
- **Skin**: no normal maps and no makeup (so the Skin model's `definition`, `softness` and `detail_strength` aren't offered). Lightening a dark skin with `melanin` goes grey: pick a lighter skin. The scalp of every skin has hair painted on it.
- **Body**: frame, muscle, fat and age only (and the proportion sliders). No regional sliders, correctives or helper bones, no body features or handles, no simpler copies (levels of detail).
- **Hair**: MakeHuman's ten hairstyles, all drafts, low meshes with painted textures: see `hair.py` for what its maps can and can't recover. No sway. The community's hair packs aren't converted.
- **Motion**: the starter animations are the library's 84 takes only; it has no walking or running backward or sideways, no wave, point, cheer, clap or shrug, and no ladder climb.
- **Layering**: what is worn under a garment is left out under its cloth, but nothing is pushed clear of anything: within a garment's setback from its openings (3 mm to 4 cm) an inner garment that stands farther off the skin can still show at the hem. The cut underwear is pressed thin whenever anything of a higher order is worn, shoes included, not only where it is covered. An open back or a keyhole shows the bandeau under it, as it would. The covers are made on the default body at rest.
- **Garments on extreme bodies**: a community garment follows the body as its file binds it; the polo shirt (`polo-t-shirt`, pack `shirts01`) is ragged at the hem and across the back on the lean muscular body.
- **Underwear**: the boxer briefs' legs are longer than short shorts and show below their hems. A small flap at each back corner of the bandeau's upper hem. One dark cloth, no fabric texture.
- **Quality**: every converted garment and hairstyle is a `draft` until reviewed (`docs/development/character-studio/asset-quality.md`). What the fit gate measures on the two outfits is in the wiki's Known limitations.
