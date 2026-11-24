# Character Studio refinement, Stage 0: asset pipeline, content and QA harness

An audit of the character asset pipeline, the shipped content and the QA harness against [the brief](brief.md) §7.1 to §7.3, §8.1, §8.3, §8.4, §9, §14 (the pipeline list), §14.2 and §17.1 to §17.3, and of plan milestones CO9, CO10, CO11, CO14, CO15 and CO16. Source and tests were read; no build, test or capture was run. Nothing but this file was written.

## Summary

- **136 requirement rows (every bullet, plus each section's closing requirement): 25 implemented, 66 partial, 45 missing.** Verification changed two rows from missing to partial; see [Verification](#verification). What is implemented is the base (bodies, shapes, face, skeleton, skin, manifests, the two QA harnesses). What is missing is content and the systems that carry content.
- **The realistic family, the default for new characters, has three items: underwear.** No hair, no garment, no footwear, no accessory, no facial hair. The stylized family has 6 hair styles (solid meshes), 3 garments, 1 pair of shoes and 2 underwear pieces. Character Studio does not offer the stylized body for new characters (`editor/src/character_workspace.rs:194-196`), so a character made there today can wear no hair, garment or shoe at all. The stylized content reaches a user only through a saved stylized character, a format 1 file, or a game's description or script.
- **CO9, CO10 and CO11 have not started in the engine.** The item manifest, catalogue, layers, variants, groom tiers and sockets exist as *formats*. Fitting by binding in the engine, runtime push-out, region-map hiding, the import flow, item thumbnails, the groom file, scalp card generation and spring bones do not exist. Surface binding and card generation exist only inside the build scripts, for the brows, the lashes and the simpler copies. CO14, CO15 and CO16 have not started.
- **No item in the library has a variant, a thumbnail, a `binding` fit or a `socket` fit.** Colour and roughness work through material values in a description, with no studio control.
- **Hair parameters and accessory placement have no place in the data.** An item manifest can't declare parameters and a worn item can't carry them or a transform. Both are contract changes for the lead.
- **The benchmark set covers slider extremes, not people.** Five of the brief's ten characters map onto existing files and a sixth (athletic) only roughly; broad, narrow, distinctive and asymmetric faces don't exist. The committed set is stale against its own generator (defect 1), and the generator would dress realistic characters wrongly if run today (defect 2).
- **The capture tool has 6 views and 2 lights.** Poses and faces are one per run, set by hand. Warm, cool and outdoor light, a T-pose, face side and three-quarter views, fixed walk phases and a skin macro view are missing from the capture tool (the workspace's Stage page has Daylight, Sunset and a Sky backdrop, for looking by eye only). The Blender harness has the joint poses and expression sheets, with garment and hair names hard-coded to the stylized package, so the realistic underwear is not measured (defect 3).

## How to read this

- **Paths.** `humanoid/…`, `realistic/…`, `benchmarks/…`, `transfer/…` and `catalogue.*` are under `assets/characters/`. `avatar/…`, `player/…`, `editor/…`, `scene/…`, `gameplay/…`, `project/…`, `anim/…`, `physics/…` and `core/…` are under `crates/`. Other paths are from the repository's top folder.
- **Status.** *implemented*: a user can do it today and a test or a file proves it. *partial*: some of it. *missing*: nothing in the code, after searching under at least three names (the names are in the evidence).
- **Size** is the work that remains: S under a day, M a few days, L a week or more, XL a milestone of its own. On an implemented row S means little or nothing.
- **Both families count.** A row is partial when only the stylized family has it, because the brief makes the realistic family the default direction (§2). "Stylized only" also means out of reach for a new character in Character Studio: the stylized body is not offered there (`editor/src/character_workspace.rs:194-196`, held by `editor/src/character_workspace/tests.rs:45-46`). It stays in the library for games, scripts and saved files.
- Counts of triangles, shapes, materials and texture coordinates were read from the JSON chunk of `HumanoidParts.glb` and `RealisticParts.glb`.

## Requirements

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size | Depends on |
|---|---|---|---|---|---|---|---|
| AP-7.1-01 | Short hairstyles | §7.1 | partial | `humanoid/hair.py:187` (Swept), `:277` (Crop), `:295` (Buzz); `humanoid/manifests.py:120-124` | Stylized only, solid mesh tier. The realistic family has no hair item (`realistic/README.md:186`). | L | AP-7.3-08; a scalp and hairline on the realistic head |
| AP-7.1-02 | Medium hairstyles | §7.1 | partial | `humanoid/hair.py:222` (Bob); `humanoid/manifests.py:121` | Stylized only; one style. | M | AP-7.1-01 |
| AP-7.1-03 | Long hairstyles | §7.1 | partial | `humanoid/hair.py:254` (Long), `:302` (Ponytail) | Stylized only; rigid (no spring bones). The earlier audit measured long hair inside the cheeks (`docs/development/character-studio/audit.md:66`). | L | AP-7.1-01; spring bones (CO11) |
| AP-7.1-04 | Curly hairstyles | §7.1 | missing | `humanoid/README.md:67` ("no curly style yet"). Searched curly, coily, curl, afro: only the lashes' curl (`realistic/lashes.py:6`) | The voxel remesh smooths curls away; needs cards or strands. | L | AP-7.3-08 |
| AP-7.1-05 | Wavy hairstyles | §7.1 | missing | Searched wavy, wave, waves: the bob is written as "a wavy bob" (`humanoid/hair.py:223`), but its wave is 3.5 mm (`:239-240`) under a 4.5 mm voxel remesh (`:160`), it is tagged `straight` (`humanoid/manifests.py:121`) and its sheet shows no wave (`docs/development/character-studio/baseline/hair_styles.png`); no style is tagged wavy (`humanoid/manifests.py:119-126`) | No style that reads as wavy in either family. | M | AP-7.3-08 |
| AP-7.1-06 | Straight hairstyles | §7.1 | partial | Tag `straight` on Swept, Bob and Long (`humanoid/manifests.py:120-122`) | Stylized only. | M | AP-7.1-01 |
| AP-7.1-07 | Braided hairstyles | §7.1 | missing | Searched braid, plait, cornrow in `assets/characters` and `crates`: nothing | No braided style; braids need authored cards or a mesh with a normal map. | L | AP-7.3-08 |
| AP-7.1-08 | Ponytails | §7.1 | partial | `humanoid/hair.py:302`; `humanoid/items/hair-ponytail.item.json` | Stylized only; the tail doesn't swing. | M | AP-7.1-01; spring bones |
| AP-7.1-09 | Buns | §7.1 | missing | Searched bun, updo, topknot: nothing | No bun in either family. | M | AP-7.1-01 |
| AP-7.1-10 | Bald and shaved options | §7.1 | partial | Bald is the empty `hair` slot, labelled "Bald", on both bases (`humanoid/manifests.py:88`; `realistic/manifests.py:137`; chip at `editor/src/character_workspace/ui.rs:543`). Shaved is Buzz, stylized only (`humanoid/hair.py:295`) | No shaved or stubble scalp on the realistic head. | S | Skin layers (CO17) or a short card shell |
| AP-7.1-11 | Visual thumbnail browser with search, filtering and favorites | §7.1 | partial | Text chips per slot (`editor/src/character_workspace/ui.rs:511-552`). The catalogue carries `tags` and `thumbnail` (`avatar/src/catalogue.rs:52`) | No item has a thumbnail and nothing makes an item's (0 `"thumbnail"` in either `items/`). Pictures of whole characters and presets are drawn offscreen and cached (`editor/src/character_workspace/thumbnails.rs:1-17`): the renderer an item thumbnail can reuse. No search, filter or favourites. | M | UI area for the browser; this area for thumbnail generation |
| AP-7.2-01 | Hair length | §7.2 | missing | `ItemManifest` has no per-item parameters (`avatar/src/item.rs:19-57`); `WornItem` holds id, variant and materials only (`avatar/src/description.rs:45-51`); a hair item follows three head shapes and nothing else (`humanoid/items/hair-swept.item.json:23`). Searched length, hair_length: build constants only (`humanoid/hair.py:76`) | No contract for item parameters and no length shapes on any style. | L | AP-7.2-12 |
| AP-7.2-02 | Hair volume | §7.2 | missing | `ItemManifest` has no per-item parameters (`avatar/src/item.rs:19-57`); `WornItem` holds id, variant and materials only (`avatar/src/description.rs:45-51`). Searched volume: nothing on hair | As AP-7.2-01. | M | AP-7.2-12 |
| AP-7.2-03 | Parting | §7.2 | missing | `ItemManifest` has no per-item parameters (`avatar/src/item.rs:19-57`); `WornItem` holds id, variant and materials only (`avatar/src/description.rs:45-51`). Swept has one fixed side part (tag `side-part`, `humanoid/manifests.py:120`) | Likely a set of variants of one style, not a slider. | M | AP-7.2-12 |
| AP-7.2-04 | Hairline | §7.2 | missing | The hairline is a constant table in the build (`humanoid/hair.py:35`); neither base manifest has a scalp or hairline region or parameter | Needs a scalp region on the head and hairline shapes or a mask. | M | AP-7.2-12 |
| AP-7.2-05 | Fringe/bangs | §7.2 | missing | `ItemManifest` has no per-item parameters (`avatar/src/item.rs:19-57`); `WornItem` holds id, variant and materials only (`avatar/src/description.rs:45-51`). Searched fringe, bangs: nothing | As AP-7.2-01. | M | AP-7.2-12 |
| AP-7.2-06 | Curl intensity | §7.2 | missing | `ItemManifest` has no per-item parameters (`avatar/src/item.rs:19-57`); `WornItem` holds id, variant and materials only (`avatar/src/description.rs:45-51`). No curly style exists (AP-7.1-04) | Needs the card or strand tier. | L | AP-7.3-08 |
| AP-7.2-07 | Strand density | §7.2 | missing | No card or strand tier is shipped (`humanoid/manifests.py:278` writes `mesh` only). Searched density, strand in `crates/render`, `crates/scene`: nothing | A strand-tier or card-atlas property. | L | CO11, CO14 |
| AP-7.2-08 | Hair color | §7.2 | partial | Hair values `tint`, `melanin`, `redness`, `dye` (`avatar/src/material.rs:226-249`); `tint` is drawn (`scene/src/material.rs:153`); set per worn item (`avatar/src/description.rs:49`) | Only `tint` reaches the picture; no control in the workspace (searched tint, melanin in `character_workspace`: 0); no realistic hair to colour. | S | CO7 (hair shading); UI area |
| AP-7.2-09 | Highlight color | §7.2 | partial | `highlight` is a strength, not a colour (`avatar/src/material.rs:257`). Four stylized styles have a second material, `HairSecondary`, for the clumps' ends, with its own `tint` (`humanoid/manifests.py:103`) | No highlight colour value; `highlight` is stored and not drawn. | M | Lead (material value set) |
| AP-7.2-10 | Root color | §7.2 | partial | Only `tip_lightening`, a root-to-tip number (`avatar/src/material.rs:250`) | Stored, not drawn; no root colour value. | M | Lead (material value set); CO7 |
| AP-7.2-11 | Hair roughness | §7.2 | partial | `roughness_scale` 0.5 to 1.5 (`avatar/src/material.rs:264`), drawn (`scene/src/material.rs:159`) | No control in the workspace. | S | UI area |
| AP-7.2-12 | Each style declares which parameters it supports | §7.2 | missing | `avatar/src/item.rs:19-57` has no capability or parameter field, and unknown fields are dropped on read (no `extra` map, unlike `avatar/src/description.rs:38`) | A contract change the lead owns (brief §14.1). | M | Lead |
| AP-7.3-01 | Mustaches | §7.3 | missing | Searched beard, stubble, mustache, moustache, sideburn, facial hair: one comment (`realistic/lashes.py:3`). No slot for facial hair in either base (`humanoid/manifests.py:87-95`) | No asset, no slot. | M | AP-7.3-08 |
| AP-7.3-02 | Beards | §7.3 | missing | Searched beard, stubble, mustache, moustache, sideburn, facial hair: one comment (`realistic/lashes.py:3`). No slot for facial hair in either base (`humanoid/manifests.py:87-95`) | No asset, no slot. Must follow the jaw and every mouth shape. | L | AP-7.3-08 |
| AP-7.3-03 | Stubble | §7.3 | missing | Searched beard, stubble, mustache, moustache, sideburn, facial hair: one comment (`realistic/lashes.py:3`). No slot for facial hair in either base (`humanoid/manifests.py:87-95`) | Best as a skin layer or a short card shell; neither exists. | M | Skin layers (CO17) |
| AP-7.3-04 | Sideburns | §7.3 | missing | Searched beard, stubble, mustache, moustache, sideburn, facial hair: one comment (`realistic/lashes.py:3`). No slot for facial hair in either base (`humanoid/manifests.py:87-95`) | Must meet the hairstyle at the temple. | S | AP-7.3-02; AP-7.1-01 |
| AP-7.3-05 | Different beard lengths | §7.3 | missing | Searched beard, stubble, mustache, moustache, sideburn, facial hair: one comment (`realistic/lashes.py:3`). No slot for facial hair in either base (`humanoid/manifests.py:87-95`) | Needs item parameters or one item per length. | M | AP-7.2-12 |
| AP-7.3-06 | Beard shapes | §7.3 | missing | Searched beard, stubble, mustache, moustache, sideburn, facial hair: one comment (`realistic/lashes.py:3`). No slot for facial hair in either base (`humanoid/manifests.py:87-95`) | As AP-7.3-05. | M | AP-7.3-02 |
| AP-7.3-07 | Color matching | §7.3 | missing | `Brows` and `Lashes` are separate hair materials of the base (`realistic/manifests.py:74-79`); values are applied slot by slot with no link (`avatar/src/resolve.rs:278-292`). Searched match, link, inherit in `avatar/src/material.rs`: nothing | No rule ties brow or beard colour to the hair's. | S | Lead (a link in the manifest or the studio) |
| AP-7.3-08 | Hair assets use the existing groom architecture and tiered quality | §7.3 | partial | The groom kind with cards, strands and mesh validates (`avatar/src/item.rs:118-126`, `:194-215`); the resolver draws cards, else the mesh (`avatar/src/resolve.rs:262-265`); brows and lashes are cards bound to the skin (`realistic/lashes.py:182`, `:384`, `:444`) | Every shipped groom is mesh only (`humanoid/manifests.py:278`). No `*.groom` reader or converter, no scalp card generator, no spring bones (searched AXGROOM, BasisCurves, spring_bone, jiggle, verlet in `crates`: 0). | XL | CO11; CO7 for alpha and shadows |
| AP-8.1-01 | Tops | §8.1 | partial | `axcenity/tee` in slot `top`, `axcenity/sports-top` (`humanoid/manifests.py:109`, `:116`) | Stylized only. Realistic has a sports bra and nothing else (`realistic/manifests.py:62`). | M | AP-8.4-01 |
| AP-8.1-02 | Shirts | §8.1 | missing | Searched shirt, button, long-sleeve, polo: only `Shirt_Tee`, the T-shirt | No shirt with a collar, placket or long sleeves. | M | AP-8.4-01 |
| AP-8.1-03 | T-shirts | §8.1 | partial | `humanoid/clothes.py:247`; `humanoid/items/tee.item.json` | Stylized only; no variants. | M | AP-8.4-01 |
| AP-8.1-04 | Sweaters | §8.1 | missing | Searched sweater, hoodie, jumper, knit: nothing | None in either family. | L | AP-8.4-01 |
| AP-8.1-05 | Jackets | §8.1 | partial | `humanoid/clothes.py:296` (cropped jacket, layer 3) | Stylized only. | L | AP-8.4-01; AP-8.4-04 |
| AP-8.1-06 | Coats | §8.1 | missing | Searched coat, parka, trench: nothing | Long tails need spring bones or cloth (CO15). | L | AP-8.4-01; CO15 |
| AP-8.1-07 | Pants | §8.1 | partial | `humanoid/clothes.py:383` (cargo pants) | Stylized only; one style. | M | AP-8.4-01 |
| AP-8.1-08 | Shorts | §8.1 | missing | Searched shorts: nothing (the boxer briefs are underwear) | On the realistic body a fitted pair can be cut like the boxer briefs. | S |  |
| AP-8.1-09 | Skirts where supported | §8.1 | missing | Searched skirt, dress: no garment | Needs binding plus spring bones or cloth. | L | AP-8.4-01; CO15 |
| AP-8.1-10 | Footwear | §8.1 | partial | `humanoid/clothes.py:459` (sneakers), `:547` (sole) | Stylized only, with 42 toe points allowed inside the sole (`humanoid/qa/thresholds.json:9`). The realistic body is barefoot. | L | AP-8.4-01 |
| AP-8.1-11 | Underwear | §8.1 | implemented | Stylized briefs and sports top (`humanoid/clothes.py:672`, `:687`); realistic boxer briefs, briefs and sports bra (`realistic/underwear.py:572`; `realistic/items/`), worn by body type (`avatar/src/choices.rs:75`); `avatar/tests/shipped_library.rs:132` | The realistic pieces have no texture coordinates and no clipping measure (defects 3 and 5). | S |  |
| AP-8.1-12 | Full outfits | §8.1 | partial | The stylized default outfit (`humanoid/build.py:46`); benchmarks `outfit-default` and `outfit-layered` (`benchmarks/make.py:142-150`) | No one-piece item, no outfit preset file, nothing for realistic. | M | CO13 (outfit presets) |
| AP-8.3-01 | Primary color | §8.3 | partial | Cloth `tint` per material (`avatar/src/material.rs:207`), drawn (`scene/src/material.rs:153`), set per worn item (`avatar/src/description.rs:49`) | Nothing marks which material is primary; no shipped variant; no control. | S | Lead (material roles) |
| AP-8.3-02 | Secondary color | §8.3 | partial | Items with several materials: jacket 3, sneakers 4 (`humanoid/manifests.py:98-104`), realistic underwear 2 (`realistic/manifests.py:67`) | `MaterialSlot` is a name and a shading model (`avatar/src/material.rs:36-39`): no role. | S | Lead (material roles) |
| AP-8.3-03 | Accent color | §8.3 | partial | `JacketTrim`, `ShoeAccent` are their own materials (`humanoid/manifests.py:101-102`) | As AP-8.3-02. | S | Lead (material roles) |
| AP-8.3-04 | Material selection | §8.3 | missing | A value is a number or a colour (`avatar/src/material.rs:83-86`); a variant maps material slots to values (`avatar/src/item.rs:107-112`). Searched fabric, swap, material_id in `crates/avatar`: nothing | A fabric can't be swapped (its maps or shading model). | M | Lead |
| AP-8.3-05 | Roughness | §8.3 | partial | `roughness_scale` on cloth and standard (`avatar/src/material.rs:134`, `:209`), drawn (`scene/src/material.rs:159`) | No control; no per-item range. | S | UI area |
| AP-8.3-06 | Metallic properties where appropriate | §8.3 | missing | No metalness value in any shading model's set (`avatar/src/material.rs:131-268`). Searched metal, metallic, metalness in `crates/avatar`: 0 | Wanted for buckles, zips and jewellery. | S | Lead |
| AP-8.3-07 | Texture variants | §8.3 | missing | `KHR_materials_variants` is not read (searched materials_variants, texture variant, variant map in `crates`: 0); garments carry no textures (the stylized parts file holds two images, skin and iris); realistic underwear has no `TEXCOORD_0` (read from `RealisticParts.glb`) | Needs garment UVs, texture sets and a variant that can name maps. | L | Lead; AP-14-05 |
| AP-8.3-08 | Customization is data-driven and constrained by each item's material manifest | §8.3 | partial | `materials[]` with a shading model per item; variant values are checked against it (`avatar/src/item.rs:258-270`) | 0 variants shipped; no allowed palette, range or role per slot. | M | Lead |
| AP-8.4-01 | Compatibility with supported body shapes | §8.4 | partial | Stylized garments copy the frame and three builds (`humanoid/build.py:297-299`; `humanoid/fit.py:152`), measured on 8 combinations (`humanoid/qa/measure.py:35`). Realistic underwear follows every body shape exactly (`realistic/underwear.py:294`) | Fitting by binding is not built: a `binding` item is drawn as a plain skinned part (`avatar/src/resolve.rs:300`). No item fits both families. | XL | CO9 |
| AP-8.4-02 | Reliable deformation | §8.4 | partial | Garments take the body's weights, four per vertex (`humanoid/build.py:122`, `:315`; `realistic/underwear.py:383`) | An offset garment has no copy of the shoulder corrective shapes; no joint measure covers garments. | M | AP-8.4-01 |
| AP-8.4-03 | No visible clipping | §8.4 | partial | Limits: tee and jacket 0 points; trousers 12, sneakers 42, briefs 28, sports top 8 (`humanoid/qa/thresholds.json:6-11`) | Rest pose only (`humanoid/qa/measure.py:410`); realistic underwear unmeasured (`:33`, `:1158`); a garment's simpler copy shows skin (`docs/development/character-studio/visual-qa.md:43`). | M | Defect 3; CO18 |
| AP-8.4-04 | Appropriate garment layering | §8.4 | partial | `layer` 0 to 3 (`avatar/src/item.rs:27-31`); three pairs cleared at build time (`humanoid/build.py:56-57`; `humanoid/fit.py:207`, `:296`) | No push-out at runtime; a new pair needs build code. | L | CO9 |
| AP-8.4-05 | Proper sleeve and waist behavior | §8.4 | partial | Waist gap held at 0 mm (`humanoid/qa/thresholds.json:3-4`); sleeves cut round the arm only (`humanoid/clothes.py:91`) | Short sleeves only; no sleeve or hem measure with the arms raised. | M | AP-17.2-21 |
| AP-8.4-06 | Correct footwear coverage | §8.4 | partial | The sneakers hide `feet`; the sole encloses the footprint (`humanoid/clothes.py:547`) | Stylized only; toe tips inside the sole are allowed; no heel height or toe pose. | M | AP-8.1-10 |
| AP-8.4-07 | Stable fitting under animation | §8.4 | partial | Garments are skinned with the body; the capture can pose any description (`player/src/character_capture.rs:585`) | No clipping measure in clips; the `fit` sheet is the rest pose (`humanoid/qa/render_baseline.py:283`); no spring bones or cloth. | L | AP-17.2-21; CO15 |
| AP-8.4-08 | An item declares its supported body families and fitting limitations | §8.4 | partial | `bases`, checked against each base's slot, regions and socket (`avatar/src/item.rs:34`, `:285-310`) | No field for limitations (ranges it fits, layers it can't be worn with). | S | Lead |
| AP-9-01 | Glasses | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched glasses, spectacles, eyewear: nothing | `HeadSocket` exists, but a rigid frame won't follow the face-width and ear sliders. | M | AP-9-11; AP-9-12 |
| AP-9-02 | Sunglasses | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched sunglasses, shades, lens: nothing | A variant of glasses with a lens material. | S | AP-9-01 |
| AP-9-03 | Earrings | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched earring, stud, piercing: nothing | No ear socket; the ear sliders move the lobe, so it needs a surface anchor. | M | AP-9-11 |
| AP-9-04 | Necklaces | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched necklace, chain, pendant: nothing | No neck socket; must follow the chest's shapes, so it is a bound item. | M | AP-8.4-01 |
| AP-9-05 | Watches | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched watch, wrist, bracelet: nothing | No wrist socket: `LeftHandSocket` rides the hand bone (`humanoid/manifests.py:33`). | M | AP-9-11 |
| AP-9-06 | Hats | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched hat, cap, beanie: nothing | `HeadSocket` exists; a hat must hide or flatten the hair under it. | M | AP-9-11; hair |
| AP-9-07 | Headwear | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched headwear, headband, hood, helmet: nothing | As AP-9-06. | M | AP-9-06 |
| AP-9-08 | Belts | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched belt, buckle, strap: nothing | Deforms with the waist: a fitted item, not a socket item. | M | AP-8.4-01 |
| AP-9-09 | Bags | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched bag, backpack, satchel: nothing | `BackSocket` on `UpperChest` exists; straps need fitting. | M | AP-9-11 |
| AP-9-10 | Other wearable accessories | §9 | missing | `catalogue.json` lists 9 garments and 6 grooms, no accessory; no slot has category `accessories` (`humanoid/manifests.py:87-95`, reused by `realistic/manifests.py:137`). Searched glove, scarf, tie, mask: nothing | None. | S | AP-9-11 |
| AP-9-11 | Use attachment sockets and the existing character binding architecture | §9 | partial | Seven sockets on both bases (`humanoid/manifests.py:29-37`; `project/src/default_humanoid.rs:55`); `fit: socket` validates and resolves (`avatar/src/item.rs:245-253`; `avatar/src/resolve.rs:299`); the applier adds the attachments (`gameplay/src/description.rs:310-322`, `:368-374`); a hat on `HeadSocket` is tested (`player/src/default_humanoid_tests.rs:403`). The schema already has an `accessories` category and slots that hold several items (`avatar/src/manifest.rs:103`, `:278`), and the resolver is tested with a watch on a wrist socket in a `multiple` slot (`avatar/src/tests/resolving.rs:88-125`, `:128-154`; fixtures `avatar/src/fixtures.rs:56`, `:95`) | No shipped accessory and no accessory slot in either base, so the path is proven on test fixtures only, and the applier's socket branch (`gameplay/src/description.rs:368-374`) has no test. The workspace has no Accessories page (`editor/src/character_workspace/ui.rs:28-33`) and wears one item per slot (`editor/src/character_workspace.rs:513`). Sockets follow bones, not skin; none at the ears, neck, wrists or nose. | M | Lead (slots, sockets) |
| AP-9-12 | Accessories support position, rotation, scale and material variants | §9 | partial | Variants are in the schema (`avatar/src/item.rs:107-112`) | `WornItem` has no transform (`avatar/src/description.rs:45-51`); the item's attachment is made at its origin (`gameplay/src/description.rs:368-374`); `Fit::Socket` holds only a name (`avatar/src/item.rs:98`). | M | Lead |
| AP-14-01 | Base body meshes | §14 | implemented | `humanoid/body.py`, `realistic/body.py`; `Body` is 25,000 and 22,184 triangles in the two parts files; `avatar/tests/shipped_library.rs:158` | None. | S |  |
| AP-14-02 | Shape keys | §14 | implemented | `realistic/shapes.py`; the realistic `Body` has 155 targets and the stylized 74 (read from the parts files); `avatar/tests/realistic_parameters.rs` | The realistic parts file is 36.4 MB with them. | S |  |
| AP-14-03 | Facial deformation shapes | §14 | implemented | `realistic/face.py`, `realistic/faceparams.py`; `humanoid/qa/face_checks.py` with `--check` | None. | S |  |
| AP-14-04 | Corrective shapes | §14 | partial | Six shoulder shapes (`realistic/correctives.py`; `realistic/README.md:155`) | Shoulder only (plan D5 says 10 to 20); none on the stylized body; an offset garment doesn't get them. | L | Body area |
| AP-14-05 | UV maps | §14 | partial | `realistic/uvs.py`: 26 islands, regions in `TEXCOORD_1` (`realistic/README.md:96`, `:120`) | Realistic underwear has no `TEXCOORD_0`; stylized brows and lashes have none; no garment is unwrapped for a texture. | M |  |
| AP-14-06 | Skin materials | §14 | implemented | `realistic/skin.py` (albedo, normal, occlusion-roughness-metalness bakes); `humanoid/materials.py:143` | No subsurface map (`realistic/README.md:186`). | S | CO7, CO17 |
| AP-14-07 | Hair | §14 | partial | `humanoid/hair.py` (six mesh styles) | No groom, card or strand pipeline; nothing for realistic. | XL | CO11 |
| AP-14-08 | Clothing | §14 | partial | `humanoid/clothes.py`, `humanoid/fit.py`; `realistic/underwear.py` | No high-to-low pipeline (plan D8), no fabric maps (`humanoid/README.md:68`), no binding. | XL | CO9, CO10 |
| AP-14-09 | Accessories | §14 | missing | Searched accessory, socket, rigid in `assets/characters/*/*.py`: no generator; 0 accessory items in `catalogue.json` | No pipeline and no item. | L | AP-9-11 |
| AP-14-10 | Skeletons | §14 | implemented | `humanoid/rig.py`; `realistic/rig.py` (the 56 shared names plus 16 added: `realistic/README.md:84`); `anim`'s twist and retarget tests (`docs/development/character-studio/testing.md`, CO4) | None. | S |  |
| AP-14-11 | Attachment sockets | §14 | partial | Declared in the manifests (`humanoid/manifests.py:29-37`) and made by the applier (`gameplay/src/description.rs:310-322`) | Bone origins only: no authored offset per family, none at the ears, neck, wrists or face. | M | Lead |
| AP-14-12 | Each asset has a manifest describing its supported functionality | §14 | partial | The builds write every base and item manifest (`humanoid/manifests.py:290`; `realistic/manifests.py:162`); a test fails on a stale catalogue (`avatar/tests/shipped_library.rs:136-142`) | An item says its slot, layer, materials and fit, not its capabilities (brief §14.1). | M | Lead |
| AP-14.2-01 | Clean topology | §14.2 | partial | Measured per object (`humanoid/qa/measure.py:85-138`) | No threshold holds it; the stylized shoes had zero-area and sliver triangles (`docs/development/character-studio/audit.md:57`). | M |  |
| AP-14.2-02 | Appropriate edge flow | §14.2 | partial | The bodies keep the source meshes' loops (`realistic/README.md:62`); garments are cut from the body | Hair is a voxel remesh, then decimated (`humanoid/hair.py:164`); nothing measures flow. | M |  |
| AP-14.2-03 | Consistent UVs | §14.2 | partial | The body's layout and texel density are recorded (`realistic/README.md:96`) | As AP-14-05. | M | AP-14-05 |
| AP-14.2-04 | Correct normals | §14.2 | partial | `inconsistent_normal_edges` is measured (`humanoid/qa/measure.py:138`); the realistic build exports shape normals (`realistic/build.py:221`) | The stylized build exports none for shapes (`humanoid/build.py:394`); neither exports tangents (`realistic/build.py:224`); not gated. | S |  |
| AP-14.2-05 | Appropriate deformation weights | §14.2 | partial | The body is gated: none over four influences, none unnormalized, joint limits (`realistic/qa/thresholds.json:12-16`, `:30-45`) | Garments and hair take copied weights and have no limit of their own. | M |  |
| AP-14.2-06 | Stable skeleton compatibility | §14.2 | implemented | Both families use the same bone names (`realistic/README.md:84`); `project` test `the_character_data_manifests_describe_the_default_humanoid` (`docs/development/character-studio/testing.md:19`) | None. | S |  |
| AP-14.2-07 | Proper material separation | §14.2 | implemented | One named glTF material per surface, each listed with its shading model; `avatar/tests/shipped_library.rs:158` | None. | S |  |
| AP-14.2-08 | No unintended mesh intersections | §14.2 | partial | Slider pairs: no new crossings (`humanoid/qa/measure.py:1021`; `realistic/qa/thresholds.json:20-25`); `humanoid/qa/posed_crossings.py` | Four stylized garments have limits above zero; hair clipping is measured (`humanoid/qa/measure.py:448`) and held by no limit (`:1247`); realistic underwear unmeasured. | M | Defect 3 |
| AP-14.2-09 | Correct LOD generation | §14.2 | partial | `Body_LOD1`, `Body_LOD2` (`realistic/lods.py:33`); `Teeth_LOD1`, `Teeth_LOD2` (`realistic/teeth.py:80`) | No authored copies for realistic underwear, eyes, lashes or brows, and none in the stylized file (`humanoid/README.md:10`): the engine simplifies those itself at load (`crates/assets/src/model/lod.rs:5-9`), which keeps no clearance from the skin; parts change level one by one. | M | CO18 |
| AP-14.2-10 | Documented dependencies | §14.2 | implemented | `docs/development/character-studio/dependencies.md`; sources and hashes (`realistic/README.md:35-38`; `transfer/makehuman.sha256`) | No pinned versions of `bpy`, NumPy or Pillow. | S |  |
| AP-14.2-11 | Automated Blender scripts for repeatable generation and QA | §14.2 | implemented | `build.py` in both packages; `measure.py --check` (`humanoid/qa/measure.py:1221`); `humanoid/qa/face_checks.py` | Garment and hair names are hard-coded (`humanoid/qa/measure.py:33-34`); the harness doesn't read description files (`docs/development/character-studio/visual-qa.md:7`). | M |  |
| AP-17.1-01 | Average male character | §17.1 | implemented | `benchmarks/realistic-masculine-body-default.character.json`, `benchmarks/stylized-masculine-body-default.character.json` (`benchmarks/make.py:121`); `avatar/tests/shipped_library.rs:229` | Bald, in underwear. The committed set is stale (defect 1). | S |  |
| AP-17.1-02 | Average female character | §17.1 | implemented | `benchmarks/realistic-feminine-body-default.character.json`, `benchmarks/stylized-feminine-body-default.character.json` (`benchmarks/make.py:121`) | As AP-17.1-01. | S |  |
| AP-17.1-03 | Slim character | §17.1 | implemented | `*-body-slim` in both families (`benchmarks/make.py:40`, `:49`) | A slider's end, not a representative person. | S |  |
| AP-17.1-04 | Athletic character | §17.1 | partial | Nearest: realistic `body-lean-muscular` and `body-muscular` (`benchmarks/make.py:51`, `:54`). The workspace has Athletic presets (`muscle` 0.7 or 0.6, `fat` −0.4: `editor/src/character_workspace.rs:161-174`) that no benchmark file uses | No moderate athletic benchmark file in either family. | S |  |
| AP-17.1-05 | Muscular character | §17.1 | implemented | `*-body-muscular`, `*-body-heavy-muscular` (`benchmarks/make.py:42-43`, `:51`, `:55`) | None. | S |  |
| AP-17.1-06 | Heavyset character | §17.1 | implemented | `*-body-heavy`, realistic `body-heavy-soft` (`benchmarks/make.py:41`, `:50`, `:53`) | None. | S |  |
| AP-17.1-07 | Character with broad facial proportions | §17.1 | missing | Face cases move one parameter at a time (`benchmarks/make.py:133-137`). Searched broad, wide, narrow in `benchmarks/`: nothing | No combined face. | S |  |
| AP-17.1-08 | Character with narrow facial proportions | §17.1 | missing | As AP-17.1-07 (`benchmarks/make.py:133-137`; searched narrow, thin, slender: nothing) | No combined face. | S |  |
| AP-17.1-09 | Character with distinctive facial features | §17.1 | missing | As AP-17.1-07 (`benchmarks/make.py:133-137`). Searched distinctive, cast, portrait: nothing | No such character. | S |  |
| AP-17.1-10 | Character with natural facial asymmetry | §17.1 | missing | The realistic manifest has left and right parameters and `jaw_shift`, but only one-sided extremes exist (`benchmarks/realistic-masculine-face-brow-height-left-max.character.json`). Searched asymmetry, asymmetric, uneven in `benchmarks/`: nothing | No such character. The stylized manifest has no sided parameter, so it can't be expressed there. | S |  |
| AP-17.1-11 | Both realistic and stylized families | §17.1 | partial | The set is made for every base in the catalogue (`benchmarks/make.py:166`): 214 realistic and 123 stylized files | No realistic hair or outfit cases (no content); 15 generated files are missing and 214 differ (defect 1). | S | Defects 1 and 2 |
| AP-17.2-01 | Front | §17.2 facial | implemented | Engine view `face-front` (`player/src/character_capture.rs:96-101`); Blender `CAM_FACE` (`humanoid/qa/render_baseline.py:43`) | None. | S |  |
| AP-17.2-02 | Side | §17.2 facial | partial | Full-body `side` only (`player/src/character_capture.rs:76-81`); Blender face close-ups are front and 35 degrees (`humanoid/qa/render_baseline.py:132`) | No face profile in either harness. | S |  |
| AP-17.2-03 | Three-quarter | §17.2 facial | partial | Blender face close-up at 35 degrees (`humanoid/qa/render_baseline.py:39`, `:132`); the engine's `face` stands about 9 degrees off axis (`player/src/character_capture.rs:88-93`) | No three-quarter face view in the engine. | S |  |
| AP-17.2-04 | Neutral expression | §17.2 facial | implemented | The default capture sets no face (`player/src/character_capture.rs:172-180`); `*-face-neutral` files (`benchmarks/make.py:132`) | None. | S |  |
| AP-17.2-05 | Smile | §17.2 facial | partial | `AXCENITY_CAPTURE_FACE=Happy:1` or the `Mouth_Smile` controls (`player/src/character_capture.rs:600`); Blender `smile` (`humanoid/qa/render_baseline.py:58`) | In the engine, one face per run, chosen by hand. Only the Blender sheets are a fixed set. | S |  |
| AP-17.2-06 | Frown | §17.2 facial | partial | The same option with `Mouth_Frown` or `Sad` (`player/src/character_capture.rs:600`); Blender `frown` (`humanoid/qa/render_baseline.py:59`) | In the engine, one face per run, chosen by hand. Only the Blender sheets are a fixed set. | S |  |
| AP-17.2-07 | Eyes closed | §17.2 facial | partial | The same option with `Eye_Closed` or `Blink` (`player/src/character_capture.rs:600`); Blender `eye closed`, `blink` (`humanoid/qa/render_face.py:37`) | In the engine, one face per run, chosen by hand. Only the Blender sheets are a fixed set. | S |  |
| AP-17.2-08 | Mouth open | §17.2 facial | partial | The same option with `Jaw_Open` (`player/src/character_capture.rs:600`); Blender `jaw open` (`humanoid/qa/render_baseline.py:60`) | In the engine, one face per run, chosen by hand. Only the Blender sheets are a fixed set. | S |  |
| AP-17.2-09 | Front | §17.2 body | implemented | `player/src/character_capture.rs:64-69`; `humanoid/qa/render_baseline.py:39` | None. | S |  |
| AP-17.2-10 | Side | §17.2 body | implemented | `player/src/character_capture.rs:76-81`; `humanoid/qa/render_baseline.py:39` | None. | S |  |
| AP-17.2-11 | Back | §17.2 body | implemented | `player/src/character_capture.rs:82-87`; `humanoid/qa/render_baseline.py:39` | None. | S |  |
| AP-17.2-12 | A-pose | §17.2 body | implemented | `Stance::Rest`, the default (`player/src/character_capture.rs:139`, `:175`); Blender `rest` (`humanoid/qa/qa_scene.py:578`) | None. | S |  |
| AP-17.2-13 | T-pose where applicable | §17.2 body | missing | Searched t-pose, tpose, t_pose in `crates` and `assets`: nothing. Nearest: `shoulder_abd_90`, left arm only (`humanoid/qa/qa_scene.py:584`) | No T-pose in either harness. | S |  |
| AP-17.2-14 | Arms raised | §17.2 body | partial | Blender `shoulder_abd_150`, left arm (`humanoid/qa/qa_scene.py:585`); engine `Cheer@0.8` (`docs/development/character-studio/visual-qa.md:36`) | Not in a default engine run; the engine has clips, no QA poses. | S |  |
| AP-17.2-15 | Bent elbows | §17.2 body | partial | Blender `elbow_135` (`humanoid/qa/qa_scene.py:579`) | In the engine only where a clip bends them. | S |  |
| AP-17.2-16 | Bent knees | §17.2 body | partial | Blender `knee_135`, `hip_flex_100` (`humanoid/qa/qa_scene.py:580`, `:586`) | In the engine only where a clip bends them. | S |  |
| AP-17.2-17 | Walking animation | §17.2 body | partial | Engine `AXCENITY_CAPTURE_POSE=Walk@0.5` (`player/src/character_capture.rs:585-598`); Blender `clips` sheet (`humanoid/qa/render_baseline.py:526`) | One frame per run; no fixed walk phases. | S |  |
| AP-17.2-18 | Multiple body shapes | §17.2 wardrobe | partial | Clipping on two frames by four builds (`humanoid/qa/measure.py:35`); the `fit` sheet (`humanoid/qa/render_baseline.py:283`) | Stylized only; outfit benchmarks exist at the default build only (`benchmarks/make.py:140-150`). | M |  |
| AP-17.2-19 | Multiple clothing layers | §17.2 wardrobe | partial | `stylized-*-outfit-layered` (`benchmarks/make.py:145-150`); layering measures (`humanoid/qa/measure.py:433-438`) | The realistic family has one layer. | S | A realistic wardrobe |
| AP-17.2-20 | Different materials | §17.2 wardrobe | missing | A benchmark item is written as its id alone (`benchmarks/make.py:87`); 0 variants in either `items/`. Searched variant, materials, tint in `benchmarks/`: nothing | No material case. | S | AP-8.3-08 |
| AP-17.2-21 | Different poses | §17.2 wardrobe | partial | The pose option works on any description (`player/src/character_capture.rs:585`) | Clipping is measured at rest only (`humanoid/qa/measure.py:410`); the `fit` sheet is at rest. | M |  |
| AP-17.2-22 | Extreme supported body proportions | §17.2 wardrobe | partial | `body-shortest`, `body-tallest`, in underwear (`benchmarks/make.py:125-129`) | No outfit at the proportion extremes. | S |  |
| AP-17.2-23 | Neutral studio lighting | §17.2 rendering | implemented | `studio` (`player/src/character_capture.rs:119-125`); Blender `STUDIO` (`humanoid/qa/qa_scene.py:859`) | None. | S |  |
| AP-17.2-24 | Warm lighting | §17.2 rendering | partial | The workspace's Stage page has Sunset, 18.1 h (`editor/src/character_workspace/stage.rs:130`, `:156`): a user can look under it. The capture's presets are `studio` and `hard` (`player/src/character_capture.rs:118-132`); a `Light` has no colour (`:106-113`). Searched warm, cool, sunset, kelvin, outdoor in the capture and `humanoid/qa`: nothing | By eye in the workspace only. No warm preset in either harness, so no fixed picture. | S |  |
| AP-17.2-25 | Cool lighting | §17.2 rendering | missing | The presets are `studio` and `hard` (`player/src/character_capture.rs:118-132`); a `Light` has no colour (`:106-113`). Searched warm, cool, sunset, kelvin, outdoor in the capture and `humanoid/qa`: nothing. The stage has no preset named or tuned as cool; its fourth is Night (`editor/src/character_workspace/stage.rs:131`) | No cool preset in either harness or in the workspace. | S |  |
| AP-17.2-26 | Outdoor lighting | §17.2 rendering | partial | The workspace's Stage page has Daylight, 12.5 h, and a Sky backdrop that keeps the procedural sky (`editor/src/character_workspace/stage.rs:129`, `:155`, `:169`, `:250`): a user can look under them. The capture's presets are `studio` and `hard` (`player/src/character_capture.rs:118-132`) and it removes the sky image and atmosphere (`:437-438`). Searched warm, cool, sunset, kelvin, outdoor in the capture and `humanoid/qa`: nothing | By eye in the workspace only. No outdoor preset in either harness, so no fixed picture. | S |  |
| AP-17.2-27 | Close-up skin inspection | §17.2 rendering | partial | `face` and `face-front`, a head portrait from 0.8 m (`player/src/character_capture.rs:88-101`); Blender `details` and `skin_maps` (`humanoid/qa/render_baseline.py:638-642`) | No macro view of cheek, hand or shoulder skin. | S |  |
| AP-17.3-01 | Consistent camera positions, lighting, resolution and character states for automated captures | §17.3 | implemented | Fixed `VIEWS`, `LIGHTS` and `Settings::default` (`player/src/character_capture.rs:63`, `:118`, `:172`); the same input gives the same pictures (`:707`); Blender seed 7 (`humanoid/qa/qa_scene.py:885`) | No picture is compared with a stored one: review is by eye (searched ssim, image diff, golden: 0). | M |  |
| AP-17.3-02 | Benchmark evidence stored alongside the implementation work | §17.3 | partial | Sheets and `measurements.json` per stage in `docs/development/character-studio/` (`baseline/`, `co4/`, `co5/`, `co6/`, `stage3/`) | Engine pictures are hand-picked cases per stage, not the set: 12 stylized sheets (`baseline/engine/`), 5 realistic sheets (`co4/engine/`), 9 pictures (`co5/engine/`) and 57 (`co6/engine/`). No stage holds the whole realistic set. A Stage 0 baseline of the realistic bodies is being captured beside this report (`refinement/baseline/engine/`). | S |  |
| AP-17.3-03 | Automated measurements supplemented by human visual review | §17.3 | implemented | `measure.py --check` (`humanoid/qa/measure.py:1221`); the review steps (`docs/development/character-studio/visual-qa.md`, Review); results in `progress.md` | None. | S |  |
| AP-17.3-04 | Geometric thresholds are not loosened because a result looks acceptable | §17.3 | implemented | The rule (`docs/development/character-studio/testing.md:9`) and each thresholds file's `about` text (`realistic/qa/thresholds.json:2`); the one change is a recorded decision (`testing.md:74`) | No test enforces it; hair and realistic garments have no limit to hold. | S |  |

## Content today

| Family | Kind | Count | Objects in the parts file (triangles) | Item files |
|---|---|---|---|---|
| Stylized | Hair | 6 | `Hair_Swept` (6,998), `Hair_Bob` (8,999), `Hair_Long` (9,990), `Hair_Crop` (7,000), `Hair_Buzz` (688), `Hair_Ponytail` (7,998) | `humanoid/items/hair-swept`, `hair-bob`, `hair-long`, `hair-crop`, `hair-buzz`, `hair-ponytail` `.item.json` |
| Stylized | Garments | 3 | `Shirt_Tee` (3,350; top, layer 1), `Jacket_Cropped` (6,398; outerwear, layer 3), `Pants_Cargo` (6,740; bottom, layer 1) | `tee`, `cropped-jacket`, `cargo-pants` |
| Stylized | Footwear | 1 | `Shoes_Sneakers` (5,806; layer 1) | `sneakers` |
| Stylized | Underwear | 2 | `Underwear_Briefs` (1,320), `Underwear_Top` (1,468) | `briefs`, `sports-top` |
| Stylized | Accessories, facial hair | 0 | | |
| Realistic | Hair | 0 | Bald only | |
| Realistic | Garments, footwear | 0 | | |
| Realistic | Underwear | 3 | `Underwear_BoxerBriefs` (1,568), `Underwear_Briefs` (1,214), `Underwear_Bra` (2,476) | `realistic/items/realistic-boxer-briefs`, `realistic-briefs`, `realistic-sports-bra` `.item.json` |
| Realistic | Accessories, facial hair | 0 | `Brows` and `Lashes` are cards, but parts of the base, not items | |

Across all 15 items: every fit is `shapes`; every groom has the `mesh` tier only; 0 variants; 0 thumbnails; 0 items of tier `premium`; 0 items that fit both families. Hair styles carry 3 shapes each, stylized garments 3 or 4, realistic underwear 37 to 53.

Who can reach it: Character Studio's workspace offers only the realistic body for new characters (`editor/src/character_workspace.rs:194-196`), so a new character there has its body type's underwear and nothing to choose. The stylized items show only for a saved stylized character or a format 1 file opened in the workspace, and in games through a description or a script.

## Plan milestones

Status of each bullet of the plan's milestone text (`plans/axcenity-engine-character-studio.md`, Stage 4 and Stage 6).

| ID | Milestone bullet | Status | Evidence | Gap |
|---|---|---|---|---|
| AP-CO9-01 | Item manifest and catalogue | implemented | `avatar/src/item.rs:19-57`; `avatar/src/catalogue.rs`; `catalogue.py`; `avatar/tests/shipped_library.rs:132` | None. |
| AP-CO9-02 | Binding and fitting | partial | Build scripts only: binding a point to the nearest body triangle with barycentric coordinates exists three times (`realistic/lods.py:66`; `humanoid/face.py:402-422`; `realistic/lashes.py:444`) and is baked into copied shapes. In the engine, `crates/avatar/src` has no `fit.rs` (modules: `avatar/src/lib.rs:22-32`); `Fit::Binding` parses and validates (`avatar/src/item.rs:91-96`, `:234-244`) and is then drawn as a skinned part (`avatar/src/resolve.rs:300`). Searched bind, binding, barycentric, shrinkwrap, surface deform, nearest triangle in `crates/avatar`, `crates/assets`, `crates/gameplay`, `crates/anim`: no fitting code | Nothing in the engine: no binding data format, no binder, no fitted rest mesh. |
| AP-CO9-03 | Layer push-out | partial | Build time only, three stylized pairs (`humanoid/build.py:56-57`; `humanoid/fit.py:296`) | Nothing at runtime. |
| AP-CO9-04 | Region hiding | partial | A region index per triangle in `TEXCOORD_1` and `RealisticRegions.png`, `map_index` 1 to 14 (`realistic/regions.py:30-50`; `realistic/manifests.py:133`). The runtime hides only by `mask_shape` (`avatar/src/resolve.rs:373-385`) | Nothing reads `map_index` or `TEXCOORD_1` in `scene`, `render` or `gameplay`. |
| AP-CO9-05 | Material variants | partial | Schema and resolution (`avatar/src/item.rs:107-112`; `avatar/src/resolve.rs:278-292`) | 0 variants shipped; glTF variants not read. |
| AP-CO9-06 | Import flow: align to the base, transfer weights, classify the slot, compute regions and binding, make a thumbnail | missing | No `import.rs`. Searched import item, transfer_weights, weight transfer, classify slot in `crates`: nothing. `project::add_character_packages` copies shipped packages only (`project/src/character_library.rs:98`) | All five steps. |
| AP-CO9-07 | Done when: every item fits both frames and the composition extremes under the threshold | partial | Stylized: six garments on 8 combinations (`humanoid/qa/measure.py:35`), four with limits above zero (`humanoid/qa/thresholds.json:8-11`) | Realistic underwear is not measured. |
| AP-CO9-08 | Done when: an imported third-party garment fits after import | missing | As AP-CO9-06 | |
| AP-CO10-01 | The high-to-low garment pipeline | missing | Garments are offsets of the body's surface (`humanoid/clothes.py:1-9`); no pattern, simulation or bake step; no fabric maps (`humanoid/README.md:68`) | |
| AP-CO10-02 | T-shirt | partial | Stylized, the old generator (`humanoid/clothes.py:247`) | None for realistic. |
| AP-CO10-03 | Long-sleeve shirt | missing | Searched long-sleeve, longsleeve, sleeve: short sleeves only | |
| AP-CO10-04 | Hoodie | missing | Searched hoodie, hood, sweatshirt: nothing | |
| AP-CO10-05 | Button-up shirt | missing | Searched button, placket, collar: nothing | |
| AP-CO10-06 | Jacket | partial | Stylized cropped jacket (`humanoid/clothes.py:296`) | None for realistic. |
| AP-CO10-07 | Jeans | missing | Searched jeans, denim: nothing | |
| AP-CO10-08 | Cargo pants | partial | Stylized (`humanoid/clothes.py:383`) | None for realistic. |
| AP-CO10-09 | Joggers | missing | The cargo pants are tagged `joggers` (`humanoid/manifests.py:112-113`); no separate item | |
| AP-CO10-10 | Shorts | missing | Searched shorts: nothing | |
| AP-CO10-11 | Sneakers | partial | Stylized (`humanoid/clothes.py:459`) | None for realistic. |
| AP-CO10-12 | Boots | missing | Searched boot: nothing | |
| AP-CO10-13 | Cap, on a socket | missing | No accessory item (AP-9-06) | |
| AP-CO10-14 | Glasses, on a socket | missing | AP-9-01 | |
| AP-CO10-15 | Watch, on a socket | missing | AP-9-05 | No wrist socket. |
| AP-CO10-16 | Colour and material variants | missing | 0 `"variants"` in either `items/` | |
| AP-CO10-17 | Done when: each item reviewed in renders for construction (seams, hems, closures), folds and fabric response | missing | No CO10 item exists to review. The four stylized garments predate the pipeline and were judged puffy, with sine-wave folds and no seams (`docs/development/character-studio/audit.md:69`). The Blender `fit` sheet is the only garment sheet (`humanoid/qa/render_baseline.py:283`) | No review, and no close-up sheet of seams, hems or closures. |
| AP-CO11-01 | The groom file and converter | missing | Searched AXGROOM, groom, BasisCurves in `crates/convert`, `crates/anim`, `crates/scene`, `crates/render`: nothing. The format is text only (`docs/development/character-studio/architecture.md`, section 11) | |
| AP-CO11-02 | Card generation and atlas from strands | partial | Cards and a painted strand texture are generated for the brows and lashes only (`realistic/lashes.py:54`, `:182`, `:340`, `:384`): laid out and drawn by script, not made from a groom's strands | No scalp card generator, no cards from strand curves, no hair atlas. |
| AP-CO11-03 | Spring bones | missing | Searched spring_bone, springbone, jiggle, verlet, secondary motion in `crates`: nothing | |
| AP-CO11-04 | Facial hair bound to the face | missing | AP-7.3-01 to AP-7.3-06 | |
| AP-CO11-05 | About 8 styles across straight, wavy, curly and coily, short to long | partial | Six stylized mesh styles, short to long; three tagged `straight`, none wavy, curly or coily (`humanoid/hair.py:331`; `humanoid/manifests.py:119-126`) | 0 for realistic; no wavy, curly or coily. |
| AP-CO11-06 | Colour by melanin and redness with highlights and root-to-tip gradients | partial | The values exist (`avatar/src/material.rs:226-268`); `tint` and `roughness_scale` are drawn (`scene/src/material.rs:153-159`) | The rest waits for hair shading (CO7). |
| AP-CO11-07 | Done when: no clipping into the head at rest | partial | Measured for the stylized styles (`humanoid/qa/measure.py:448`) | Held by no limit (defect 6). |
| AP-CO11-08 | Done when: styles reviewed in renders and in motion (idle, walk) | partial | A still sheet of the six stylized styles (`humanoid/qa/render_baseline.py:186`; `docs/development/character-studio/baseline/hair_styles.png`); the earlier audit's review found them lumpy (`docs/development/character-studio/audit.md:66`) | No review in motion: hair is rigid, and the Blender `clips` sheet is in the realistic set only, which has no hair (`humanoid/qa/render_baseline.py:637-642`). Nothing for realistic. |
| AP-CO11-09 | Done when: no slab shadows | partial | Ray traced shadows read cut-outs (`crates/render/src/ray_cut_outs.wgsl`). The shadow-map pipeline has no fragment stage (`crates/render/src/renderer/gpu.rs:625`), so a cut-out card casts a solid slab there. No shipped hair is a card: the six styles are solid meshes, and the realistic brows and lashes are blended (`alphaMode: BLEND` in `RealisticParts.glb`) and cast no shadow | Unmet for shadow maps; it only does not show today because no groom is a card. Needs cut-out-aware shadow maps (CO7) before card hair ships. |
| AP-CO14-01 | The legal review of hair-rasterizer patents | missing | Still listed as to do (`docs/development/character-studio/dependencies.md:42`) | Blocks strands. |
| AP-CO14-02 | Strand compute pipeline (skin, simulate guides, interpolate, rasterize, composite) | missing | Searched strand, groom, guide in `crates/render`, `crates/scene`: nothing | |
| AP-CO14-03 | Hair self-shadowing | missing | Searched deep opacity, self-shadow, hair shadow in `crates/render`: nothing | |
| AP-CO14-04 | LOD from strands to cards | missing | No strand tier | |
| AP-CO15-01 | Cloth simulation on coarse proxies | missing | The physics crate steps the vendored library's soft bodies (`physics/src/sync.rs:579`); nothing connects them to a garment. Searched cloth, cloth sim, xpbd in `crates/anim`, `crates/gameplay`, `crates/physics`: no cloth code | |
| AP-CO15-02 | Wind | partial | The Workspace has `GlobalWind` (`core/src/class.rs:1206`) | Nothing on a character reads it. |
| AP-CO16-01 | Tops: polo, sweater, coat, vest, formalwear | missing | AP-8.1-02, AP-8.1-04, AP-8.1-06 | |
| AP-CO16-02 | Bottoms: chinos, dress pants, sweatpants, athletic shorts, workwear | missing | Searched chino, sweatpants, workwear: nothing | |
| AP-CO16-03 | Footwear: running shoes, work and hiking boots, dress shoes, sandals | missing | Searched boot, sandal, dress shoe: nothing | |
| AP-CO16-04 | Accessories: hats, belts, jewellery, backpacks, gloves | missing | AP-9-01 to AP-9-10 | |

## Benchmark coverage (brief §17)

| | In-engine capture (`player/src/character_capture.rs`) | Blender harness (`humanoid/qa/`) | Missing |
|---|---|---|---|
| Characters | 337 description files in `benchmarks/`: 214 realistic, 123 stylized. Bodies at the slider ends, one face parameter at a time, stylized outfits and hair | Sets its own looks: two frames by four builds, face shapes, expressions. Does not read description files | Athletic (moderate), broad face, narrow face, distinctive face, natural asymmetry. Outfits on other builds. Material cases |
| Views | `front`, `three-quarter`, `side`, `back`, `face`, `face-front` (`:63-101`) | Turnaround at 0, 35, 90 and 180 degrees; face at 0 and 35; a camera per joint pose; details | Face side; face three-quarter in the engine; a skin macro view; an upper-body view |
| Poses | `rest` (A-pose), `idle`, any starter clip at a time (`Walk@0.5`, `Cheer@0.8`), one per run (`:137-146`, `:585-598`) | `rest`, `elbow_135`, `knee_135`, `shoulder_abd_10`, `_90`, `_150`, `hip_flex_100`, `wrist_70`, `spine_twist_45`, `fist`, left side (`qa_scene.py:577-590`); twist poses; a clips sheet | T-pose; both-sided arms raised, elbows and knees bent in the engine; fixed walk phases |
| Expressions | Any face control or built-in expression, one set per run (`:600`); 15 built-in expressions (`anim/src/face_play.rs:24-40`) | Neutral, smile, frown, jaw open, blink, brow raise, sneer (`render_baseline.py:56-65`); every control, viseme and expression (`render_face.py:36-52`, `:237`) | A fixed list run every time: neutral, smile, frown, eyes closed, mouth open |
| Lighting | `studio`, `hard` (`:118-132`) | One studio: key, fill, rim (`qa_scene.py:859-869`); a raking lamp for shoulder and chest sheets (`qa_scene.py:677`) | Warm, cool, outdoor. The workspace's Stage page has Daylight, Sunset, Night and a Sky backdrop, for looking by eye, not for captures (`editor/src/character_workspace/stage.rs:126-170`) |
| Comparison | The same input gives the same bytes (`:707`); sheets compared by eye. Stored: 12 stylized sheets (`baseline/engine/`) and hand-picked realistic cases (`co4/engine/`, `co5/engine/`, `co6/engine/`) | `measurements.json` against a thresholds file; sheets by eye | A stored baseline of the whole realistic set (Stage 0 is capturing the bodies in `refinement/baseline/engine/`); any picture-to-picture check |

## What the Blender environment needs

From `humanoid/README.md:33-47`, `realistic/README.md:15-31`, `humanoid/qa/README.md:15-24` and `docs/development/character-studio/co5/README.md:94-104`.

- **Python 3.13 with the `bpy` module (5.x), NumPy and Pillow**, or Blender 4.2 or later (the scripts are written against 5.3). Blender's own Python has NumPy but not Pillow: install Pillow into a folder and start Blender with `--python-use-system-env` and `PYTHONPATH` naming it.
- Always run with `-I`.

```sh
python -m venv .bpy && .bpy/bin/pip install bpy pillow numpy     # Windows: .bpy\Scripts\pip

# Stylized package (about 15 seconds). Then copy Humanoid.glb and HumanoidAnimations.glb
# into crates/samples/projects/humanoid/assets/models/Humanoid/.
python -I assets/characters/humanoid/build.py --out assets/characters/humanoid
# or: blender -b --factory-startup --python assets/characters/humanoid/build.py -- --out assets/characters/humanoid

# Realistic package (about 40 seconds on 16 cores; --texture-size 4096, --skip-bakes)
python -I assets/characters/realistic/build.py --out assets/characters/realistic

# Plain Python, no Blender: manifests alone, the catalogue, the benchmark set
python -I assets/characters/humanoid/manifests.py
python -I assets/characters/catalogue.py
python3 -I assets/characters/benchmarks/make.py        # not before defect 2 is fixed

# Checks and sheets
python -I assets/characters/humanoid/qa/measure.py --only clipping --out <file> --check assets/characters/humanoid/qa/thresholds.json
python -I assets/characters/humanoid/qa/measure.py --package realistic --out <file> --check assets/characters/realistic/qa/thresholds.json
python -I assets/characters/humanoid/qa/render_baseline.py --package realistic --out <folder>
python -I assets/characters/humanoid/qa/face_checks.py --check
python -I assets/characters/humanoid/qa/render_face.py --out <folder>
```

- **Only to regenerate `realistic/source/body_shapes.npz`:** the source bundle's `.blend`, MakeHuman's CC0 files and about three minutes, from `assets/characters/transfer/`: `python3 fetch_makehuman.py MH`, then `extract.py --bundle … --makehuman MH --work WORK`, `marks.py --work WORK`, `register.py --work WORK`, `transfer.py --work WORK --method filtered --pack ../realistic/source/body_shapes.npz`. The package build needs none of this.
- **Only to refresh `source/`:** `python source/extract.py path/to/human_base_meshes_bundle.blend` (URL and checksum in `extract.py`).
- **Engine pictures** need a GPU and a release build: `AXCENITY_CAPTURE_DESCRIPTIONS=assets/characters/benchmarks AXCENITY_CAPTURE_DIR=<folder> cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture`.
- Not pinned anywhere: the versions of `bpy`, NumPy and Pillow (defect 8).

## The shortest honest path to a first realistic wardrobe and hair

### What can be reused

| Piece | Where | How far it carries |
|---|---|---|
| Garments cut from the body, exact under every shape | `realistic/underwear.py` (`cut` `:198`, `taut` `:269`, `garment` `:294`, `recess` `:350`) | Any close-fitting garment: T-shirt, tank top, shorts, leggings, socks, swimwear, gloves. A new set of fields and a rule per garment. No binding needed: each vertex is a mix of body vertices, so it has the body's weights, all 49 identity shapes and the shoulder correctives |
| Offset-and-drape garments | `humanoid/clothes.py` (`region` `:54`, `cut_limb` `:91`, `drape` `:141`, `rim` `:179`) and `humanoid/fit.py` (`clear_body` `:152`, `follow_layer` `:207`, `clear_layer` `:296`) | They work from joints, weights and face-set landmarks, and the realistic build already loads stylized modules (`realistic/common.py:65`). To run on the realistic body: the bone lists must take in the 16 twist and helper bones (`humanoid/clothes.py:23-26` name the shared bones only), hems and gaps retuned, and about 55 shapes copied per garment instead of 4. Fit carries over. The look does not: the earlier audit called these garments puffy with sine-wave folds (`docs/development/character-studio/audit.md:69`) |
| Sneakers | `humanoid/clothes.py:459-670` | The construction carries over. The result was judged "a lumpy white sock on a saucer" (`audit.md:68`): not a realistic shoe |
| Mesh hair | `humanoid/hair.py` | `Head` reads the stylized face sets and face module (`humanoid/hair.py:21-45`), so it needs an adapter for the realistic head. It gives the `mesh` tier, a solid mass: right for the far level of detail, not for the first realistic hair |
| Cards with alpha, bound to the skin | `realistic/lashes.py` (`_strands` `:54`, `_brow_card` `:340`, `over_skin` `:415`, `follow_bound` `:444`) | The only card code in the repository. The base for scalp cards, beards and stubble: strips laid on the skin, each vertex bound to the triangle under it, following every shape |
| Binding a point to the body's surface | `realistic/lods.py:66` (`_bind`); the stylized `SkinFollow` used at `humanoid/build.py:109-119` | The build-side half of ADR-0035 exists twice. One becomes the binding writer |

### What must be made new

- A scalp region and a hairline on the realistic head: the root surface for cards, and what a hat hides.
- A scalp card generator (layered shells along a comb field), its strand atlas, and the mesh tier from the same guides.
- Texture coordinates and at least one fabric texture set for garments. Without them every garment is a flat colour.
- Footwear built on a last for the realistic foot. A shoe is not an offset of a foot.
- Rigid accessories, their slots and more sockets.
- Thumbnails for every item.
- Loose garments (hoodie, jacket, coat, skirt). No generator makes these to the realistic bar. They need binding plus a sewn, simulated or imported source (plan D8).

### Proposed order

| Step | What | Size | Needs |
|---|---|---|---|
| 1 | Close-fitting set cut from the realistic body: T-shirt, tank top, shorts, leggings, socks. Keep the interim mask-shape hiding. Carry `TEXCOORD_0` through the cut | S each | Nothing new |
| 2 | One short hairstyle as cards on the scalp, with its mesh tier; stubble and a short beard from the same generator | M | A scalp region; a `facial_hair` slot (lead); alpha and shadow quality depends on CO7 |
| 3 | Drape garments ported with copied shapes: sweatshirt, trousers, a first shoe | M each | The twist and helper bones in the garment code; accept the file size |
| 4 | Fitting by binding in `avatar`, then move step 3's garments to `binding` and open the import path | XL | The list below |

Steps 1 and 2 give a dressed, haired realistic character without waiting for CO9. Step 3 is honest only if the lead accepts stylized-generator quality as an interim. Step 4 is the plan's CO9.

### What fitting by binding (ADR-0035) still lacks in code

1. **The binding data.** A type and a file format for "triangle, barycentric coordinates, offset along the normal" per item vertex. `fit.data` is validated as a path and nothing reads or writes it (`avatar/src/item.rs:234-237`). Architecture section 9 names "a binding file" without a layout.
2. **The binder.** Nearest body triangle per garment vertex, in Rust for imports (or in the build, then stored). `crates/avatar/src/fit.rs` doesn't exist.
3. **The fit.** Rebuilding an item's rest positions from the resolved body's baked rest mesh when identity changes. Today a `binding` item is drawn unfitted with no warning (`avatar/src/resolve.rs:300`, defect 4).
4. **Push-out.** Clearing each layer from the body and from lower layers by `margin`, in layer order, at runtime.
5. **A derived model for bound items.** `assets` bakes an item's own identity shapes; a bound item needs the body's baked positions as input, cached by the same identity key.
6. **Live shapes.** A bound garment has no copy of the shoulder correctives, and a bound beard none of the mouth shapes. The binding must carry those shapes' deltas onto the item at bake time, or sleeves poke through raised shoulders and beards through open jaws.
7. **Weights.** Skin weights transferred through the binding (the import flow's "transfer weights"), limited to four.
8. **Region hiding by the map.** Something must read `map_index` and `TEXCOORD_1`: the renderer, or the bake (dropping hidden triangles from the derived body mesh, which needs no shader change).
9. **Levels of detail.** Bound items need simpler copies that stay clear of the body's simpler copies, or characters must switch level together (CO18).
10. **Tests.** The ones `docs/development/character-studio/testing.md:60` lists for CO9: binding round trip, fit under the thresholds on both frames and the composition extremes, layer order, an imported garment.

## Benchmark plan proposal

### The named characters, as description files

Add a `CAST` table to `benchmarks/make.py`, so the files are generated like the rest (the script deletes any description it didn't write: `benchmarks/make.py:174-176`). Name them `<family>-<frame>-cast-<name>-bare` and `-dressed`, which keeps the rule `every_benchmark_character_is_sound` checks. `bare` is underwear and bald and never changes. `dressed` wears the family's default outfit and hair, and is captured again in the same change whenever that content changes. Values are starting points: tune them once by eye, then freeze them.

| Character | Frame | Realistic parameters | Stylized parameters |
|---|---|---|---|
| `average-male` | 0 | Defaults | Defaults |
| `average-female` | 1 | Defaults | Defaults |
| `slim` | 1 | `fat` −0.6, `muscle` −0.2 | `build` −0.6 |
| `athletic` | 1 | `muscle` 0.5, `fat` −0.3 | `muscle` 0.5, `build` −0.2 |
| `muscular` | 0 | `muscle` 0.9, `fat` 0.1, `shoulder_width` 1.05 | `muscle` 1.0, `build` 0.2 |
| `heavyset` | 0 | `fat` 0.8, `muscle` −0.2, `waist_width` 1.08 | `build` 0.9 |
| `broad-face` | 0 | `jaw_width_left`, `_right` 0.7; `cheekbone_left`, `_right` 0.5; `temple_width` 0.5; `chin_width` 0.6; `nostril_width_left`, `_right` 0.4; `mouth_width_left`, `_right` 0.3 | `head_width` 0.7, `jaw_width` 0.7, `cheekbones` 0.5, `nose_width` 0.4, `mouth_width` 0.3 |
| `narrow-face` | 1 | The same parameters at −0.6 and −0.5; `chin_length` 0.3 | `head_width` −0.7, `jaw_width` −0.6, `head_length` 0.3, `nose_width` −0.4 |
| `distinctive` | 0 | `nose_bridge` 0.8, `nose_length` 0.6, `brow_ridge` 0.7, `chin_forward` 0.5, `ear_out_left`, `_right` 0.6, `eye_depth_left`, `_right` 0.5, `upper_lip_fullness` −0.4 | `nose_bridge` 0.8, `nose_length` 0.6, `chin_forward` 0.5, `ears_out` 0.7, `eyes_size` −0.4, `lips_full` −0.4 |
| `asymmetric` | 1 | `brow_height_left` 0.25, `brow_height_right` −0.10, `eye_height_left` 0.15, `eye_size_right` −0.20, `mouth_width_left` 0.20, `nostril_width_right` 0.25, `cheek_fullness_right` 0.20, `ear_out_left` 0.30, `jaw_shift` 0.30, `nose_tilt` 0.30 | Not expressible: the stylized manifest has no sided parameter. Leave it out and say so, or add sided parameters first |

Every parameter ID in the table exists in its base manifest and every value is inside its range (checked in verification).

Also in the generator:

- Start `athletic` and `heavyset` from the workspace's presets (Athletic: `muscle` 0.7 male or 0.6 female, `fat` −0.4; Heavyset: `fat` 0.8, `muscle` 0.2: `editor/src/character_workspace.rs:161-188`), or record why the benchmark differs, so the Presets page and the benchmark show the same people.
- Fix the underwear rule to follow the base's `choices` (defect 2) before running it again.
- Outfits on other bodies: `outfit-default` and `outfit-layered` at slim, heavy, muscular, shortest and tallest (brief §17.2, wardrobe).
- A material case per item once variants exist.
- A staleness check, as the catalogue has: at least "every face parameter has its two files, every item its `item-` file".

### Capture additions (add, never change)

`VIEWS`, `LIGHTS` and `Settings::default` stay as they are, so today's 2 by 6 run, its file names and its contact sheet stay byte-comparable.

| Addition | What | Note |
|---|---|---|
| Extra views, a new constant beside `VIEWS` | `face-side` (eye −2.3, 4.6, 0), `face-three-quarter` (eye −1.63, 4.62, −1.63), both looking at 0, 4.58, 0 at 30 degrees; `skin-close` (cheek, about 12 degrees); `upper-body` (shoulders to hips, for sleeves and waist) | Not appended to `VIEWS`: the sheet has a column per view (`player/src/character_capture.rs:475`) |
| Extra lights, a new constant beside `LIGHTS` | `warm` (the stage's Sunset time, a warm tint), `cool` (overcast, a cool ambient), `outdoor` (midday, sky and atmosphere kept) | `Light` needs two optional fields (a tint, "keep the sky"). The two existing presets keep their values; prove it by capturing one description before and after and comparing bytes |
| QA poses | `t-pose`, `arms-raised`, `elbows-bent`, `knees-bent`, both sides, at the angles `qa_scene.POSES` uses | As pose files in `benchmarks/poses/` copied into the capture's scratch project and played like a clip (`:343-355`), or a new `Stance` variant. Whether a clip file can hold a pose for both skeletons needs checking first |
| Pose and face lists | `AXCENITY_CAPTURE_POSES=rest;Walk@0.25;Walk@0.5;Walk@0.75` and `AXCENITY_CAPTURE_FACES=neutral;smile;frown;eyes-closed;mouth-open` (named sets of controls) | Today one pose and one face per process (`:585`, `:600`) |
| Suites | `AXCENITY_CAPTURE_SUITE=face`, `body`, `wardrobe`, `rendering`: each a fixed list of characters, views, poses, faces and lights | Without it, today's run. Suite pictures go to `<name>/<pose>/<face>/<light>-<view>.png` |
| A hash list per baseline | `hashes.json` beside `captures.json` | Says which pictures changed on the same machine. Not a perceptual check |

| Suite | Characters | Views | Poses | Faces | Lights |
|---|---|---|---|---|---|
| `face` | Cast, bare | `face-front`, `face`, `face-three-quarter`, `face-side` | `rest` | Neutral, smile, frown, eyes closed, mouth open | `studio`, `hard` |
| `body` | Cast, bare | `front`, `three-quarter`, `side`, `back` | `rest`, `t-pose`, `arms-raised`, `elbows-bent`, `knees-bent`, four walk phases | Neutral | `studio` |
| `wardrobe` | Cast, dressed; the outfit files on other bodies | The four body views, `upper-body` | `rest`, `arms-raised`, `knees-bent`, `Walk@0.5` | Neutral | `studio`, `hard` |
| `rendering` | The two averages (later: at several skin tones) | `front`, `face-front`, `skin-close` | `rest` | Neutral | `studio`, `hard`, `warm`, `cool`, `outdoor` |

Together about 2,600 pictures. The Stage 0 baseline measured 3.2 to 7.8 seconds for a character's 12 pictures (`refinement/baseline/README.md`), so that is roughly 12 to 28 minutes on the reference PC.

### Blender harness additions

- `--description <file>` for `render_baseline.py` and `measure.py`, so both harnesses draw the same characters (`Character.set_sliders` already resolves a manifest's parameters: `humanoid/qa/qa_scene.py:234`).
- Garment and hair lists read from the item manifests, not the constants at `humanoid/qa/measure.py:33-34` and `humanoid/qa/render_baseline.py:66`. This is what lets the realistic underwear, and every later item, be measured.
- New entries in `POSES` for the T-pose and the both-sided poses; new lighting dictionaries beside `STUDIO` for warm, cool and outdoor (a sun and Blender's built-in sky: no file to vendor). `STUDIO`, the cameras and the existing poses are never edited.
- Clipping measured in the QA poses as well as at rest, and a limit for hair.

## Defects found

| # | Severity | Defect | Evidence |
|---|---|---|---|
| 1 | Low | **The committed benchmark set is stale.** Importing `benchmarks/make.py` and comparing its output with the folder, without writing: 352 files generated, 337 committed, 15 missing (the eight `brow-length` face cases, `outfit-default` and `outfit-layered` on both frames, and three `item-` files, all realistic), and all 214 realistic files differ. They differ only in the `items` list the generator would add: the committed files have none and still draw the right underwear, because the resolver puts it on (`avatar/src/resolve.rs:185-186`). So what the pictures lack is the eight brow-length cases. The generator was not run after the underwear and the brow length landed. No test notices: the soundness test reads the files as they are | `benchmarks/make.py:166-177`; `avatar/tests/shipped_library.rs:229` |
| 2 | Medium | **The generator ignores body-type choices.** It takes the first item of each underwear slot, so run today it writes boxer briefs and a sports bra onto every realistic character, masculine ones included, and `item-realistic-briefs` onto the masculine frame. The resolver then replaces what the file says with the body type's own underwear, so the file no longer describes what is drawn | `benchmarks/make.py:97`, `:155-159`; `avatar/src/choices.rs:59-69`; `avatar/src/resolve.rs:185-186` |
| 3 | Medium | **The realistic underwear's clipping is not measured.** The harness names the six stylized garments, and gives the realistic package an empty outfit, which skips the clipping section. The realistic thresholds hold no garment. `testing.md` still says "no garments until CO9" | `humanoid/qa/measure.py:33`, `:1158`; `humanoid/qa/qa_scene.py:137-139`; `docs/development/character-studio/testing.md:71-72` |
| 4 | Low | **A `binding` item is drawn unfitted, silently.** The resolver maps it to a plain skinned part and reports no problem | `avatar/src/resolve.rs:298-301` |
| 5 | Medium | **The realistic underwear has no texture coordinates.** `Underwear_BoxerBriefs`, `Underwear_Briefs` and `Underwear_Bra` carry no `TEXCOORD_0` in `RealisticParts.glb`, so they can take no fabric texture, normal map or texture variant. The stylized garments have one | Read from the parts file. Searched uv, texcoord, unwrap in `realistic/underwear.py`: nothing; the cut builds positions, weights and shapes (`realistic/underwear.py:294`) |
| 6 | Low | **Hair clipping is measured and held by nothing.** `clipping.hair` is written, and `check` reads only the waist, `garments` and `layering` keys. The earlier audit's hair-inside-the-cheeks finding has no limit | `humanoid/qa/measure.py:448`, `:1247`; `humanoid/qa/thresholds.json` |
| 7 | Low | **Documents out of date.** `visual-qa.md` says the set is 136 files with 13 realistic and that the realistic base has no builds, face parameters or items; it has 337 and 214. Its pose list omits `shoulder_abd_10`. The realistic README gives the parts file as 20.5 MB; it is 36.4 MB. The plan and the agent assignments name `assets/characters/hair/`, which doesn't exist | `docs/development/character-studio/visual-qa.md:7`, `:16`; `humanoid/qa/qa_scene.py:583`; `realistic/README.md:128`; `docs/development/character-studio/agent-assignments.md:14` |
| 8 | Low | **The Python environment is not pinned and its commands are POSIX-only.** The READMEs use `.bpy/bin/python`; on Windows it is `.bpy\Scripts\python`. `.bpy` is not ignored by git. No file pins `bpy`, NumPy or Pillow, and the hair build depends on Blender's voxel remesh and decimation, which can change between versions | `humanoid/README.md:44-45`; `humanoid/qa/README.md:19-21`; `.gitignore` |

## Shared contracts that would have to change (the lead owns them)

| # | Contract | Change | For |
|---|---|---|---|
| 1 | Item manifest (`avatar/src/item.rs`) | Declared parameters per item (id, range, default, what each drives: item shapes, material values, variants), and the item's capabilities | §7.2, §7.3, §14.1 |
| 2 | Description (`WornItem`, `avatar/src/description.rs:45`) | Per-item parameter values; a per-item transform (position, rotation, scale) within limits the item declares | §7.2, §9 |
| 3 | Material values (`avatar/src/material.rs`) | A role per material slot (primary, secondary, accent); a metalness value; a highlight colour and a root colour for hair; variants that can name maps or a fabric, not only numbers and colours; allowed palettes or ranges per slot; a link so brows and facial hair can follow the hair's colour | §7.2, §7.3, §8.3 |
| 4 | Base manifest (`avatar/src/manifest.rs`) | Slots for facial hair and accessories (several with `multiple`): the schema has the `accessories` category and `multiple` already (`:103`, `:278`), so this part is data in `manifests.py` and a workspace page, not a format change; sockets at the ears, neck, wrists and nose bridge, with an offset; a scalp region and hairline; a rule for what a hat does to hair | §7.3, §9 |
| 5 | Fitting (architecture sections 9 and 16) | The binding file's layout; the signatures of `avatar::fit`; how bound items get the body's live shapes | §8.4, CO9 |
| 6 | Grooms (architecture section 11) | Whether built-in cards may be authored directly, without a strand source; the scalp binding for cards | §7.1, CO11 |
| 7 | Region hiding | Who reads `map_index`: the renderer or the bake | §8.4 |
| 8 | Item limits | A field for fitting limitations; items that list both families in `bases` once binding exists, and the `realistic-` prefix in item IDs | §8.4 |
| 9 | Benchmarks | The `cast` group's names; the rule that only `make.py` writes the folder; a staleness test | §17.1 |
| 10 | Capture tool | The extra views, lights, pose and face lists and suites; the `Light` fields | §17.2 |
| 11 | `gltf` crate features | Turning on the extensions feature to read `KHR_materials_variants` (`docs/development/character-studio/dependencies.md`) | §8.3 |
| 12 | Where items live | Items in their own model files, not in the family's parts file (see ownership) | §14 |

## File ownership notes for parallel work

- **One parts file per family is the main collision.** `RealisticParts.glb` holds the body, the face parts and every item (`realistic/build.py:201-204`). Any two agents who rebuild it conflict on a 36 MB binary. Item manifests can already name any model file (`avatar/src/item.rs:38`): put each new item, or each agent's set, in its own file (`realistic/items/<name>.glb`), built by its own script that *reads* the body from the parts file.
- **`build.py`, `manifests.py` and `catalogue.py` are the lead's** (`docs/development/character-studio/agent-assignments.md:11`), yet every new garment or hair module must be called from `build.py` and listed in `manifests.py`. Separate item builds, as above, remove that funnel.
- **`humanoid/qa/` is one harness for both packages.** Give it one owner. Content agents add data it reads (item manifests, thresholds), not code.
- **`benchmarks/*.character.json` and `catalogue.json` are generated.** Never hand-edit or hand-merge them: regenerate after a merge. Only `make.py` and `catalogue.py` are edited.
- **`player/src/character_capture.rs` needs one owner.** Additions only.
- **New Rust files** `avatar/src/fit.rs` and `import.rs` belong to the clothing agent; the schema files stay with the lead.
- **Hair has no home yet.** The plan gives the hair agent `assets/characters/hair/`; hair today is inside each family's build. Decide before two agents start.
- **Builds write into the asset folders in place.** Two agents must not build in one checkout: build to a scratch `--out` first.
- **Evidence** goes in a folder per stage under `docs/development/character-studio/`, one owner per folder.

## Risks

1. **Quality against the brief's bar.** The generators that exist make offset-from-body garments and voxel hair. Porting them fits the body and does not reach the look the brief asks for (plan risk 1). Steps 1 and 2 of the proposed path are the parts that can.
2. **File size.** Fitting by copied shapes costs a garment its share of about 55 shapes. The realistic parts file grew from 10 MB before the body shapes to 36 MB with the body shapes, the face and three underwear pieces, and every rebuild adds a full copy to git history.
3. **Binding and live shapes.** Without the shoulder correctives on bound sleeves and mouth shapes on bound beards, binding looks worse than copied shapes in motion.
4. **Hair cards depend on rendering.** Cards without TAA-stable alpha and cut-out shadows (CO7) look worse than the solid meshes. Hair content and hair rendering have to land in step.
5. **Legal reviews are outstanding** for the strand rasterizer (CO14) and cloth tethers (CO15). `dependencies.md` also lists Delta Mush as unchecked, while `realistic/correctives.py` uses a delta mush offline to author the corrective shapes; the lead should confirm that offline use is outside that note.
6. **Benchmark growth.** A full cross of characters, views, poses, faces and lights is tens of thousands of pictures. Suites keep it near 2,600; they must be fixed early, because adding to a suite later re-baselines it.
7. **Changing the capture tool by accident.** Nothing compares today's pictures with stored ones, so a change to the stage, the renderer's defaults or the `Light` struct can shift every baseline unnoticed. Capture a byte baseline before touching it.
8. **Reproducible builds.** Unpinned `bpy` and Blender version drift can change remeshed and decimated meshes (the stylized hair, the simpler copies) without a source change.
9. **Rigid accessories against sliders.** A socket follows a bone. Face width, ear and fat sliders move skin, not bones, so glasses and earrings drift unless they are anchored to the surface or the item declares the range it fits.
10. **Underwear that can't come off.** The body type's underwear is always worn (`avatar/src/choices.rs:1-4`). Under a real wardrobe it is a layer below every garment, in every character: it needs region hiding or it will show through thin or simplified garments.
11. **The stylized family falls behind.** It has no sided face parameters, a narrower build model and the older content. Every "both families" row costs twice unless the brief's "remain supported" is read as "keeps working".
12. **Acceptance by eye.** Until the Blender harness reads item lists from manifests, realistic garments have no numbers at all, and the rule against loosening thresholds has nothing to hold.

## Verification

A second pass by another agent, whose job was to refute this audit. Source and tests were read again. Nothing was built or run except three read-only Python scripts: the parts files' JSON chunks, every item manifest, and `benchmarks/make.py` imported and compared with the folder without writing.

### What was checked

- **Coverage.** The owned brief sections were counted bullet by bullet: §7.1 11, §7.2 12, §7.3 8, §8.1 12, §8.3 8, §8.4 8, §9 12, §14 12, §14.2 11, §17.1 11, §17.2 27, §17.3 4. That is 136, and every one has a row. The plan's "done when" cells for CO10 and CO11 were not all rows: three were added (V8).
- **Every "missing" row** was searched again across `crates/` and `assets/characters/` under more names (facial hair, braids, buns, eyewear, jewellery, belts, bags, gloves, spring bones, groom files, hair cards, glTF variants, favourites, weight transfer, push-out, T-pose, picture comparison, shrinkwrap, surface deform, cloth). Two rows changed to partial (V2). Two plan rows changed to partial (V3, V4). The rest stand. The only T-pose pattern that matched was `rest_pose`.
- **Every "implemented" row** was opened at its evidence. All 25 stand.
- **Partial rows.** 40 of the 64 were opened at their evidence. Their statuses stand. Six had their evidence or gap corrected (V5 to V7, V9 to V11).
- **Citations.** About 150 `path:line` references were opened. Three were off (V12). The numbers read from the parts files (triangles, shape counts, materials, texture coordinates, 36.4 MB) and from the item manifests (15 items, all `shapes`, no variant, no thumbnail) are right.
- **Defects.** All eight are real today. Defect 1 was reproduced exactly (352 generated, 337 committed, 15 missing, 214 differ) and its severity lowered (V13).
- **The proposal.** Every parameter ID in the cast table exists in its base manifest and every value is in range. The capture additions do not touch `VIEWS`, `LIGHTS` or `Settings::default`.

### Corrections

| # | What changed | Why |
|---|---|---|
| V1 | Summary, "How to read this" and "Content today" now say the stylized body is not offered for new characters | `offers_base` leaves the stylized base out whenever another base is in the library (`editor/src/character_workspace.rs:194-196`; test `editor/src/character_workspace/tests.rs:45-46`), and every preset is on the realistic base (`:145-190`). The audit's "stylized only" rows read as if a user could pick that content. For a new character in Character Studio the hair and clothing pages are empty |
| V2 | AP-17.2-24 (warm) and AP-17.2-26 (outdoor): missing to partial. The counts are now 25, 66 and 45 | The audit's own rule is that missing means nothing in the code. The workspace's Stage page has Sunset (18.1 h), Daylight (12.5 h) and a Sky backdrop that keeps the procedural sky (`editor/src/character_workspace/stage.rs:126-170`, `:250`). A user can review under them by eye; neither capture harness has them. Cool stays missing: no preset is cool |
| V3 | AP-CO9-02 (binding and fitting): missing to partial | The report's own reuse table lists surface binding in the build scripts (`realistic/lods.py:66`, `humanoid/face.py:402-422`, `realistic/lashes.py:444`). AP-CO9-03 was rated partial for the same reason (build time only). Nothing exists in the engine, which the gap now says |
| V4 | AP-CO11-02 (card generation and atlas): missing to partial | Cards and a painted strand texture are generated for the brows and lashes (`realistic/lashes.py:54`, `:182`, `:340`, `:384`), which AP-7.3-08 already counts as partial. There are no scalp cards and nothing is made from strand curves |
| V5 | AP-17.3-02 gap and the "Comparison" cell of the coverage table | The audit said the realistic set has no committed baseline. Realistic engine pictures are committed per stage: `co4/engine/` (5 sheets and `captures.json`), `co5/engine/` (9), `co6/engine/` (57). They are hand-picked cases, not the set. A Stage 0 baseline is being captured in `refinement/baseline/engine/` |
| V6 | AP-9-11 evidence and gap; contract 4 | The audit said nothing proves the path from a description, and asked for accessory slots as a contract change. The schema already has the `accessories` category and `multiple` slots (`avatar/src/manifest.rs:103`, `:278`), and the resolver is tested with a watch on a wrist socket (`avatar/src/tests/resolving.rs:88-154`). What is missing is manifest data, a workspace page (`editor/src/character_workspace/ui.rs:28-33`), several items per slot in the workspace (`editor/src/character_workspace.rs:513`) and a test of the applier's socket branch |
| V7 | AP-14.2-09 gap | "No copies" was wrong about what is drawn. Where a file has no `_LOD` objects the engine simplifies each part itself at load (`crates/assets/src/model/lod.rs:5-9`). The gap is that those copies keep no clearance from the skin |
| V8 | Rows added: AP-CO10-17, AP-CO11-08, AP-CO11-09 | The plan's "done when" for CO10 (items reviewed in renders) and two of CO11's three (reviewed in renders and in motion; no slab shadows) had no row, while CO9's and CO11's third did. 41 plan rows became 44 |
| V9 | AP-7.1-05 evidence (status stays missing); AP-CO11-05 evidence | The bob is written as "a wavy bob" (`humanoid/hair.py:223`), so "only a wave hook" understated it. Its 3.5 mm wave is below its 4.5 mm voxel, it is tagged `straight`, and its sheet shows no wave, so no style reads as wavy. "All straight" was also loose: three styles carry the tag |
| V10 | AP-17.2-05 to AP-17.2-08 gap | "Not a fixed set" is true of the engine only. The Blender sheets are a fixed set for both packages (`humanoid/qa/render_baseline.py:56-65`; `humanoid/qa/render_face.py:36-52`) |
| V11 | AP-7.1-11 gap; AP-17.1-04 evidence; one bullet in the benchmark plan | The audit missed two things that exist: an offscreen picture renderer with a cache for characters and presets (`editor/src/character_workspace/thumbnails.rs`), which item thumbnails can reuse, and the workspace's Athletic and Heavyset presets (`editor/src/character_workspace.rs:161-188`), which the benchmark cast should start from or knowingly differ from |
| V12 | Three citations | `realistic/README.md:3` does not say there is no hair; line 186 does. Hair `roughness_scale` is at `avatar/src/material.rs:264`, not 263. The capture timing "2 to 4 seconds for 12 pictures" had no source: the Stage 0 baseline measured 3.2 to 7.8 seconds (`refinement/baseline/README.md`), which makes the suites 12 to 28 minutes, not a quarter of an hour |
| V13 | Defect 1: medium to low, with two sentences added | All 214 differing files differ only in the `items` list the generator would add. The committed files have no items and still draw the right underwear, because the resolver puts it on (`avatar/src/resolve.rs:185-186`). The real loss is eight brow-length cases. Defect 2 (the generator's wrong underwear rule) stays medium: it must be fixed before anyone regenerates |
| V14 | Summary wording: "have not started in code" to "in the engine"; "thumbnails" to "item thumbnails"; "card generation" to "scalp card generation" | To agree with V3, V4 and V11 |

### Checked and left as it was

- AP-8.1-11 (underwear, implemented): both families ship it and tests hold it. On the realistic body it is fixed by body type and cannot be chosen, which risk 10 and defects 3 and 5 already carry.
- AP-17.1-01 to -03, -05 and -06 (implemented): the files exist and a test requires `-body-default`. They are slider ends, as the rows say.
- AP-14.2-10 (documented dependencies, implemented): sources, licences and hashes are written down. Unpinned tool versions are defect 8.
- AP-CO10-01, AP-CO9-06 and the CO14 and CO15 rows: nothing found under any other name.
