# Clothing, wardrobe and accessories: reference index

Stage 0 of the Character Studio refinement ([brief](../../refinement/brief.md) §8, §9, §13.1 "Clothing", §14.1; plan D6, D8, CO9, CO10, CO16). One file:

- The index, as three tables: garments, fabrics, accessories.
- Section 5: the brief's categories against a production approach, and a first-wave pick list for the realistic family.
- Section 6: a capability manifest design note, for the lead to decide.
- Section 7: the fit review checklist.

Nothing was downloaded into the repository. No picture was copied. The only text taken from a source is short quotations in quotation marks, each beside its link. Section 9 lists what a second pass checked and corrected.

## 1. How this was collected

- Collected on 2026-11-16 by web search, by opening the pages, and by reading licence files and public catalogue APIs.
- **Opened** means the page or file was fetched and read. **Search summary** means the page refused the fetch (HTTP 403) or no longer resolves, and the entry rests on a search engine's summary. Treat a search-summary entry as a lead, not as evidence.
- A licence is claimed only where it was read on the source's own page or in its licence file. The row says where. Anything else is marked **unverified**.
- This is not legal advice.

## 2. Licence classes (brief §13.5)

| Class | Meaning | In this file |
|---|---|---|
| 1 | CC0 or public domain | Poly Haven textures and models, ambientCG materials, the MakeHuman packs marked CC0, Quaternius packs |
| 2 | Open source, compatible with an Apache-2.0 engine that redistributes it | FreeSewing (MIT), GarmentCode's pattern library (MIT), the MakeHuman packs marked CC-BY (attribution per asset) |
| 3 | Licensed commercial | None proposed |
| 4 | Reference only: links, never files | Product documentation, press breakdowns, encyclopedia text, every commercial game |
| 5 | Original | Every item the build scripts generate (`source.licence` `CC0-1.0`, `assets/characters/humanoid/manifests.py:23`) |
| unverified | Licence not confirmed on the source's own page | Marked per row |

**The Sims 4.** Its game files, and custom content built from them, cannot be shipped in an Apache-2.0 engine. They are class 4. The publisher's own pages say so:

- The EA User Agreement (G-36, opened) grants "a personal, limited, non-transferable ... revocable and non-exclusive license" for "non-commercial use", and says "You may not access, copy, modify or distribute any EA Service, Content or Entitlements" and may not "extract or otherwise use source code or other data from EA Services". Both sentences carry the usual exception for what the publisher expressly authorises or the law permits; nothing on the page authorises reuse in another product.
- The Sims 4 mods policy (G-35, opened) lets creators share mods, but only as mods for that game: "Mods must be non-commercial and distributed free of charge", they "can't be sold, licensed, or rented for a fee", and the game's logos and trademarks may not be used to promote them. It grants nothing for use in another product.
- Custom content is usually a recolour or a remesh of a game mesh, on the game's UV layout and rig. It carries the publisher's content inside it. A creator cannot license what they do not own.

So: read the public descriptions of how the system works, describe it in our own words, and take no file.

## 3. What the repository already held

Already written down, and not repeated here:

- How other products fit garments to changing bodies (per-garment shape copies, MetaHuman outfit sizes, The Sims 4 deformer maps, surface binding, cages, simulation): [research.md](../../research.md) B.4 and [notes/body-face-skin-cloth.md](../../notes/body-face-skin-cloth.md) D and E.
- Fabric shading (sheen, anisotropy, Filament's cloth recipes), seams and trim sheets: [research.md](../../research.md) B.6 and the notes' section G.
- Cloth simulation in games, and the Rust crates: [research.md](../../research.md) B.7 and the notes' section H.
- glTF extensions and VRM as formats: [notes/formats-and-libraries.md](../../notes/formats-and-libraries.md) sections 1.3 and 2.
- The decision itself: [ADR-0035](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md) (surface binding, layer push-out, region hiding).

What the code holds today, with evidence. This is the ground the references are measured against.

| Fact | Evidence |
|---|---|
| The item manifest has: slot, layer, tier, bases, model, hides, materials, variants, fit, groom, tags, thumbnail, source | `crates/avatar/src/item.rs:19-57` |
| Three fit methods are declared: `shapes`, `binding`, `socket` | `crates/avatar/src/item.rs:83-99` |
| `binding` is not built yet. A binding item resolves as an ordinary skinned part. No binding file, no runtime push-out | `crates/avatar/src/resolve.rs:299-300`; `docs/development/character-studio/progress.md:134` |
| Push-out between layers exists only at build time, for the stylized garments | `assets/characters/humanoid/fit.py:152` (`clear_body`), `:207` (`follow_layer`), `:296` (`clear_layer`) |
| The stylized garments are cut from the body's own surface and draped by smoothing. They are not built from patterns | `assets/characters/humanoid/clothes.py:1-10`, `:141` (`drape`) |
| The realistic family has three items, all underwear, fitted by `shapes` as an interim exception | `assets/characters/realistic/items/`; ADR-0035 "Interim exception" |
| No shipped item declares a variant. The field exists and is checked | `crates/avatar/src/item.rs:46`, `:107-112`, `:257-270`; no `variants` key in any `assets/characters/*/items/*.item.json` |
| A material slot is a name and a shading model. Nothing says which slot is the primary colour, or which values a user may change | `crates/avatar/src/material.rs:36-39` |
| The cloth model has `tint`, `sheen_tint`, `roughness_scale`, `sheen_roughness_scale`, `anisotropy_scale`. The standard model has `tint` and `roughness_scale`. Neither has a metallic value (brief §8.3 asks for one "where appropriate") | `crates/avatar/src/material.rs:132-135`, `:206-224` |
| An item sits in exactly one slot | `crates/avatar/src/item.rs:26` |
| Both bases have seven slots: hair, two underwear, top, outerwear, bottom, shoes. None is an accessory slot | `assets/characters/humanoid/manifests.py:87-95`; reused by `assets/characters/realistic/manifests.py:137` |
| Both bases have seven sockets: head, back, hips, two hands, two feet. None at the nose bridge, ears, neck or wrists | `assets/characters/humanoid/manifests.py:29-37`; `crates/project/src/default_humanoid.rs:55` |
| A socket fit is the socket's name only. No offset, no adjustment range | `crates/avatar/src/item.rs:98` |
| Only garments have a layer (0 to 3). An accessory with a layer is an error | `crates/avatar/src/item.rs:217-230` |
| A worn item in a description is `{id, variant?, materials?}`. No per-item fit value or offset | `crates/avatar/src/description.rs:45-51` |
| The description keeps unknown fields (`extra`). The item manifest does not, so writing one back drops fields a newer build added | `crates/avatar/src/description.rs:39`; `crates/avatar/src/item.rs:19-57` (no such field) |
| Clipping is measured per garment against thresholds, over every frame and build combination | `assets/characters/humanoid/qa/thresholds.json:2`; `docs/development/character-studio/testing.md:60`, `:72` |
| A garment and the body at different levels of detail show skin through the cloth. Open defect | `docs/development/character-studio/progress.md:118` |

## 4. The index

Every row was collected on 2026-11-16.

### 4.1 Garments

Construction, pattern sources, simulation, how other platforms declare and fit clothing, and meshes that could ship.

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| G-01 FreeSewing monorepo: parametric sewing patterns as code | https://codeberg.org/freesewing/freesewing | FreeSewing | 2026-11-16 | Pattern source for tops, shirts, hoodies, jackets, trousers, shorts, skirts, underwear | Opened (the `LICENSE` file, the README, and each design's `package.json`). About 70 designs, each a JavaScript module that drafts flat pattern pieces from body measurements in millimetres. Useful ones: `teagan` (T-shirt), `aaron` (tank top), `simon` and `simone` (button shirt), `hugo` (raglan hoodie), `hannah` (hoodie), `huey` (zip hoodie), `sven` (sweater), `jett` (bomber jacket), `devon` (denim jacket), `jaeger` (sport coat), `carlton` and `carlita` (long coat), `wahid` (waistcoat), `charlie` (chinos), `titan` (trouser block), `crux` (hiking trousers), `paco` (summer trousers), `shale` and `ashley` (shorts), `penelope` (pencil skirt), `sandy` (circle skirt), `bruce`, `uma`, `umbra` (underwear), `lily`, `lumira`, `lumina` (leggings), `sabrina` (sports top). There is no jeans design; the trouser block is the start for one. The tests run under Node, so no browser is needed | **Class 2, MIT.** Read in `LICENSE` at the repository root and as `"license": "MIT"` in every design's `package.json` opened (67 designs). Keep the copyright and permission notice with any copied or ported code. The old GitHub mirror stopped at 2025-04; Codeberg is the live repository | data |
| G-02 FreeSewing documentation: Teagan T-shirt | https://freesewing.eu/docs/designs/teagan/ | FreeSewing | 2026-11-16 | T-shirt construction | Opened. Pieces: front and back (each cut on the fold), two sleeves, a neck strip 6 cm wide. Thirteen required measurements (chest, waist, hips, neck, biceps, shoulder to shoulder, shoulder slope and lengths). Fabric: a stretch knit such as jersey, about 130 to 200 g/m² | The code is MIT (G-01). The documentation's own licence is not stated on the page: **unverified**, so read it, do not copy it | visual guidance |
| G-03 FreeSewing core API: `Pattern.getRenderProps()` | https://freesewing.dev/reference/api/pattern/getrenderprops | FreeSewing | 2026-11-16 | Scripted pattern-to-garment pipeline | Opened. After `draft()`, the pattern returns its width and height in millimetres, its settings, its layout and its drafted stacks, so that an external renderer can draw it. That is the hook for writing panels out as data for Blender | MIT (G-01). Documentation licence unverified | data |
| G-04 GarmentCode and its PyGarment library | https://github.com/maria-korosteleva/GarmentCode | The repository's author, who is the copyright holder named in `LICENSE`; the project's pages sit on ETH Zurich domains, but the README names no institution | 2026-11-16 | Pattern source in Python: panels, edges, stitches | Opened (the README, the `LICENSE` file, the folder listing). A Python framework for parametric sewing patterns: `Edge`, `Panel`, `Component`, `Interface`, with a JSON pattern form. Its garment programs cover a tee, a bodice, sleeves, collars, bands (cuffs, waistbands), trousers, a circle skirt, panelled and levelled skirts, godets. Body measurements come in as a file. It is the closest existing thing to "panels and seams as data", and it is Python, like the build | **Class 2, MIT** for this repository (read in `LICENSE`). **Its simulation step is not usable**: it depends on a fork of NVIDIA Warp under the "NVIDIA Source Code License for Warp" (read in that fork's `LICENSE.md`), which is not an open-source licence. Take the pattern library only, and sew and simulate in Blender | data |
| G-05 GarmentCodeData: 115,000 simulated garments with patterns | https://www.research-collection.ethz.ch/entities/researchdata/d9df8fe3-2add-49a2-bb58-21750ca7970c | ETH Zurich | 2026-11-16 | Shows the range one parametric pattern set reaches | Search summary. Tops, shirts, dresses, jumpsuits, skirts and trousers, each fitted to many bodies | **Unverified.** The record has a "Rights / License" field that the summary did not show. Do not use the data. Not needed: G-04's code is what feeds a pipeline | visual guidance |
| G-06 Seamly2D: open pattern-drafting program | https://github.com/FashionFreedom/Seamly2D | Seamly2D project | 2026-11-16 | Hand-drafted patterns, if a script is not enough | Catalogue record read (GitHub's licence field). A desktop program for drafting patterns from measurements | GPL-3.0. A tool an author may run; its code cannot be copied into the engine. Class 4 as a source of code | visual guidance |
| G-07 Blender manual: Cloth, Shape panel | https://docs.blender.org/manual/en/latest/physics/cloth/settings/shape.html | Blender Foundation | 2026-11-16 | Sewing flat panels with Blender's cloth | Opened. "Sewing springs are created by adding extra edges to a cloth mesh that are not included in any faces." They pull the joined vertices together. "Max Sewing Force": zero means unbounded and is "not recommended", because forces are extreme in the first frames while the panels are far apart. "Shrinking Factor" shrinks or grows the cloth: a way to add or remove ease. "Pin Group" holds a waistband or collar. "Dynamic Mesh" lets the rest shape follow shape keys | The manual is CC-BY-SA 4.0 (read in the page footer). Technique, not text: class 4 for the text | visual guidance |
| G-08 Press breakdown: animation-ready clothes in Marvelous Designer | https://80.lv/articles/creating-animation-ready-clothes-simulation-in-marvelous-designer | 80 Level | 2026-11-16 | The order of work for a simulated garment | Opened (dated 2023-01-10). Base patterns fitted to the avatar; block-out with fabric; buttons, internal seam lines and fold lines; simulate; set thickness to suit the cloth; lay the patterns out as UVs; texture colour, roughness and dirt | Class 4. Copyright the publisher and the artist | visual guidance |
| G-09 Press breakdown: digital outfits with Blender and CLO | https://80.lv/articles/setting-up-digital-outfits-with-blender-clo-3d/ | 80 Level | 2026-11-16 | Layered outfits; where simulated meshes need rework | Opened (dated 2023-06-19). In a multi-layered outfit each layer is separated before simulation; socks, tights and plain underwear are often better as textures on the avatar. One simulation generally takes at least a full working day, and tuning the settings takes more time on top. The author names retopology and file export as occasional difficulties | Class 4 | visual guidance |
| G-10 Blender Studio: remixing page | https://studio.blender.org/remixing/ | Blender Foundation | 2026-11-16 | Whether Blender Studio garment files may be reused | Opened. "Blender Studio content is generally released under Creative Commons Attribution License (CC-BY)", and each asset carries its own licence line. No garment construction breakdown was found on the site in this pass | Generally CC-BY, per asset. No garment asset was verified: treat as class 4 until one is | visual guidance |
| G-11 Encyclopedia: T-shirt | https://en.wikipedia.org/wiki/T-shirt | Wikipedia contributors | 2026-11-16 | T-shirt construction | Summary read. T-shaped body and sleeves, crew neck with no collar, light stretch knit | Text CC BY-SA 4.0 (read in the site footer). Share-alike: class 4, facts only | visual guidance |
| G-12 Encyclopedia: Dress shirt | https://en.wikipedia.org/wiki/Dress_shirt | Wikipedia contributors | 2026-11-16 | Button shirt construction | Summary read. A collar, a full-length front opening fastened with buttons; the article goes on to collars, plackets, yokes and cuffs | CC BY-SA 4.0. Class 4 | visual guidance |
| G-13 Encyclopedia: Hoodie | https://en.wikipedia.org/wiki/Hoodie | Wikipedia contributors | 2026-11-16 | Hoodie construction | Summary read. A sweatshirt or jacket with a hood | CC BY-SA 4.0. Class 4 | visual guidance |
| G-14 Encyclopedia: Jeans | https://en.wikipedia.org/wiki/Jeans | Wikipedia contributors | 2026-11-16 | Jeans construction | Summary read. Denim trousers with copper pocket rivets; the article covers the five-pocket layout, the back yoke and the fly | CC BY-SA 4.0. Class 4 | visual guidance |
| G-15 Encyclopedia: Seam | https://en.wikipedia.org/wiki/Seam_(sewing) | Wikipedia contributors | 2026-11-16 | Seam types | Summary read. A seam is the join where layers are held by stitches; the article lists plain, flat-felled, French and lapped seams | CC BY-SA 4.0. Class 4 | visual guidance |
| G-16 Encyclopedia: Hem | https://en.wikipedia.org/wiki/Hem | Wikipedia contributors | 2026-11-16 | Hems | Summary read. The edge folded and sewn to stop fraying and to set length, "at the end of the sleeve or the bottom of the garment" | CC BY-SA 4.0. Class 4 | visual guidance |
| G-17 Encyclopedia: Ease | https://en.wikipedia.org/wiki/Ease_(sewing) | Wikipedia contributors | 2026-11-16 | Ease | Summary read. "The amount of room a garment allows the wearer beyond the measurements of their body." Two kinds: wearing ease (room to move) and design ease (the style) | CC BY-SA 4.0. Class 4 | visual guidance |
| G-18 Encyclopedia: Placket | https://en.wikipedia.org/wiki/Placket | Wikipedia contributors | 2026-11-16 | Closures | Summary read. A finished opening at the neck, front, waist or sleeve, usually a doubled fold that the fasteners are sewn to | CC BY-SA 4.0. Class 4 | visual guidance |
| G-19 Encyclopedia: Yoke | https://en.wikipedia.org/wiki/Yoke_(clothing) | Wikipedia contributors | 2026-11-16 | Yokes on shirts, jeans, skirts | Summary read. A shaped piece round the shoulders or the hips that supports the looser part of the garment | CC BY-SA 4.0. Class 4 | visual guidance |
| G-20 Encyclopedia: Shoe | https://en.wikipedia.org/wiki/Shoe | Wikipedia contributors | 2026-11-16 | Sneaker and boot construction | Opened (the construction section). Sole in layers: insole, midsole ("typically there for shock absorption"), outsole. Heel. Upper, with a tongue under the laces. Vamp ("the front part of the shoe, starting behind the toe"). Toe box. Welt, a strip stitched between upper and sole. The upper is shaped on a last | CC BY-SA 4.0. Class 4 | visual guidance |
| G-21 Roblox: layered clothing overview | https://create.roblox.com/docs/art/accessories/layered-clothing | Roblox | 2026-11-16 | Fit by cages | Opened. A layered item is a mesh, skinning, and two cages. "The inner cage defines the inner surface of the clothing item where the clothes wrap over." "The outer cage defines the outer surface ... where any additional clothes can wrap and layer over" | Class 4. Product documentation | visual guidance |
| G-22 Roblox: layered clothing specifications | https://create.roblox.com/docs/art/accessories/clothing-specifications | Roblox | 2026-11-16 | What an item must satisfy | Opened. One mesh, at most 4,000 triangles, watertight. Cages named `<Name>_InnerCage` and `<Name>_OuterCage`; cage vertices and UVs must not be deleted, because they match the body's. Four bone influences a vertex. Textures up to 2048. An 8 x 8 x 8 stud bounding box. Tops attach at the body front, bottoms at the waist centre | Class 4 | visual guidance |
| G-23 Roblox: `WrapLayer` class | https://create.roblox.com/docs/reference/engine/classes/WrapLayer.md | Roblox | 2026-11-16 | What a layered item declares | Opened. `Order`: "Controls the composition order for layered clothing", higher on top, default 1. `Puffiness` (-1 to 1): how much the layers under an item inflate it. `ShrinkFactor`: shrinks or expands one item's mesh without touching other layers. `AutoSkin`: take skin weights from the body. `BindOffset`: a small position or rotation correction. `Enabled`. Puffiness and ShrinkFactor are marked deprecated | Class 4. We use Axcenity's own names, never Roblox branding | visual guidance |
| G-24 MakeHuman: introduction to MakeClothes | https://static.makehumancommunity.org/assets/creatingassets/makeclothes/introduction.html | MakeHuman community | 2026-11-16 | Surface binding, body hiding and layer order in an open system | Opened. "Each vertex of an item of clothing uses 3 vertices on base mesh or helper": the same idea as ADR-0035's binding. A per-garment delete group removes covered body vertices, to avoid "bleeding through". A z-depth number orders layers: "Lower values are used for clothes near the body" | Documentation: licence not stated on the page. Read it, do not copy it. The tool's code is AGPL or GPL (G-38): never copy code | visual guidance |
| G-25 MetaHuman: testing and configuring a parametric outfit | https://dev.epicgames.com/documentation/en-us/metahuman/testing-and-configuring-your-parametric-outfit-asset | Epic Games | 2026-11-16 | Outfit sizing; hidden faces | Opened. The outfit switches between its authored sizes to find the best fit; a body that fits badly gets a new size. Each size has a slot name, and a colour parameter that "must match in all materials across all sizes". The body hidden face map "tells MHC which faces of the body are always hidden by the clothing, and removes them". Head faces cannot be hidden | Class 4. MetaHuman content may not be used outside Unreal Engine | visual guidance |
| G-26 Unreal Mutable wiki: basic concepts | https://github.com/anticto/Mutable-Documentation/wiki/Basic-Concepts | Anticto | 2026-11-16 | A customizable object as typed parameters | Opened. "A Customizable Object is the blueprint of the customization." An instance is a set of parameter values. Parameter kinds: enumeration (from an object group), float, colour, projector | The wiki has no licence file: **unverified**, class 4 | visual guidance |
| G-27 Unreal Mutable wiki: UI metadata | https://github.com/anticto/Mutable-Documentation/wiki/UI-Metadata | Anticto | 2026-11-16 | Metadata that lets the interface build itself | Opened. Each parameter and each option can carry a section, a display order ("Lower values will show first"), a thumbnail, a float range, tags, and free "User defined key value pairs". "With this metadata, you can easily create data-driven UIs" | Unverified, class 4 | visual guidance |
| G-28 glTF `KHR_materials_variants` | https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_variants/README.md | Khronos Group | 2026-11-16 | Colour and material variants in an imported file | Opened. Ratified. A root list of named variants; each primitive maps a material to one or more variants. With no match, the viewer falls back to the primitive's own material. Variants are premade and whole-asset: a material swap, with no parameters and no geometry change | An open specification, already covered in the format notes. The document is copyright The Khronos Group; its copyright appendix grants a conditional licence to use and reproduce the unmodified specification without fee. A format to implement, not text or an asset to copy | data |
| G-29 Daz Studio: `DzTransferUtility` | https://docs.daz3d.com/public/software/dazstudio/4/referenceguide/scripting/api_reference/object_index/transferutility_dz | Daz 3D | 2026-11-16 | What auto-fit carries from a figure to a garment | Opened. "Implements the logic of transferring weights, morph, maps, etc using projection." Options: skin binding, morphs (all or a list), face groups, region groups, selection maps, UVs, fit to the figure, a projection template by garment shape, a smoothing modifier | Class 4 | visual guidance |
| G-30 Ready Player Me: asset creation guide | https://docs.readyplayer.me/asset-creation-guide | Ready Player Me | 2026-11-16 | Asset requirements for a web avatar platform | Search summary. **The documentation host no longer resolves.** Press reports say the service closed on 2026-01-31 after an acquisition. From the summary: GLB only, 10 MB a file, four skin weights a vertex, no custom glTF extensions, separate masculine and feminine skeletons, outfits exported with the body and the armature | Class 4, and gone. Do not design against it | visual guidance |
| G-31 "Create a Sim Technical Test Criteria (v2)" | https://media.contentapi.ea.com/content/dam/eacom/SIMS/common/create-a-sim-technical-test-criteria.pdf | Electronic Arts | 2026-11-16 | The publisher's own public account of what a clothing asset needs | Opened (a PDF). Names are built from life stage, body type and clothing type; types listed: Top, Bottom, FullBody, Shoes, Hat, Glasses, Necklace, Bracelet, Sock, Coat or Robe, Gloves. Two levels of detail are required. Triangle targets, level 0 then 1: tops 3,300 and 1,600; bottoms 1,200 and 770; full body 5,800 and 3,300; shoes 800 and 700; hats 1,200 and 1,000; earrings and necklaces 1,200 and 800. Diffuse and specular 1024 x 2048, normal 1024 x 1024, plus a shadow map. At least two colour variants. Vertex colour marks close-fitting against loose-fitting vertices, which changes how the garment follows the body. Edge loops should line up with the body's | Class 4. Copyright the publisher. Numbers quoted as facts for comparison | visual guidance |
| G-32 s4pi `CASPartResource.cs`: the fields of a clothing part, as the community reads them | https://github.com/s4ptacle/Sims4Tools/blob/develop/s4pi%20Wrappers/CASPartResource/CASPartResource.cs | Sims 4 Tools development team | 2026-11-16 | What one part declares | Opened (the source header and the property list). A part carries: body type and sub-type; age and gender flags; species; a list of tag flags; flags for the other parts it excludes; flags for the body-slider regions it switches off; a swatch colour list; hue, saturation, brightness and opacity sliders; a sort layer and a composition method; a region map; a fallback part and an opposite-gender part; a level-of-detail list; texture keys | GPL-3.0 (read in the file header). Field names only. Class 4: never copy the code, never read game files with it for assets | visual guidance |
| G-33 Sims 4 Studio forum: enabling and disabling accessories on outfits | https://sims4studio.com/thread/10162/enabling-disabling-accessories-outfits-solved | Sims 4 Studio community | 2026-11-16 | Exclusion flags | Opened. A part's exclude flags are a bit mask of the slots it switches off: hat, hair, shoes, earrings, glasses, necklace, wrists, full body, upper body, lower body, head. All zeros means anything can be worn with it | Class 4. A forum post | visual guidance |
| G-34 MakeHuman: asset packs | https://static.makehumancommunity.org/assets/assetpacks.html | MakeHuman community | 2026-11-16 | Garment meshes that could ship | Opened. Packs are listed under "Mesh assets shared under CC0" or "under CC-BY". CC0: Shirts 01, Pants 01, Skirts 01, Dress 01, Suits 01, Suits 02, Underwear 01, Underwear 04 (with socks), Shoes 01, Gloves 01. CC-BY: Shirts 02 and 03, Pants 02 and 03, Skirts 02, Dress 02 and 03, Suits 03, Underwear 02 and 03, Shoes 02 and 03 | **Class 1** for the CC0 packs, **class 2** for the CC-BY packs (credit per asset). Read on this page, which states the licence per pack, and on each pack's own page, whose table gives every asset's name, author, source and licence: every asset in every pack named here carries its pack's licence (section 9). The FAQ (G-38) links CC-BY to version 4.0. See section 5.3 for how to check one asset | actual asset |
| G-35 The Sims 4 policy on mods | https://help.ea.com/en/articles/the-sims/the-sims-4/mods-policy/ | Electronic Arts | 2026-11-16 | Licence evidence | Opened. Quoted in section 2 | Class 4 | visual guidance |
| G-36 EA User Agreement | https://www.ea.com/legal/user-agreement | Electronic Arts | 2026-11-16 | Licence evidence | Opened. Quoted in section 2 | Class 4 | visual guidance |
| G-37 MakeHuman: licence | https://static.makehumancommunity.org/about/license.html | MakeHuman community | 2026-11-16 | Licence evidence | Opened. "All core assets are shared under Creative Commons, CC0." "The source code of MPFB is shared under GPL." "The source code of MakeHuman is shared under AGPL" | As quoted | data |
| G-38 MakeHuman FAQ: using a CC-BY asset | https://static.makehumancommunity.org/oldsite/faq/what_do_i_need_to_do_when_i_use_a_ccby_asset.html | MakeHuman community | 2026-11-16 | Licence evidence | Opened. Contributed assets come "licensed in two forms: CC0 and CC-BY". With CC-BY you must "tell end users of your work that you used the model in question, and who you got the model from" | As quoted | data |
| G-39 Quaternius: Modular Character Outfits, Fantasy | https://quaternius.com/packs/modularcharacteroutfitsfantasy.html | Quaternius | 2026-11-16 | Garment meshes that could ship (stylized) | Opened. 12 outfits from 62 modular parts, three texture variations each, on a humanoid rig, in FBX, OBJ, Blend and glTF | **Class 1, CC0.** Read on the pack page ("Free to use in personal, educational and commercial projects. (CC0 License)") and in the site's FAQ ("All models are under the CC0 License"). Fantasy and low detail: for the stylized family or as an import test, not the realistic family | actual asset |
| G-40 Kenney: support page | https://kenney.nl/support | Kenney | 2026-11-16 | CC0 game assets | Opened. "All game assets on the asset pages are public domain licensed (CC0)". No realistic garment mesh was found there in this pass | Class 1, CC0, read on this page. Nothing to pick for the realistic family | actual asset |
| G-41 Sketchfab: downloadable models under CC0 | https://api.sketchfab.com/v3/search?type=models&license=cc0&downloadable=true&q=shoe | Sketchfab; each model has its own author | 2026-11-16 | CC0 garment and footwear meshes | Opened (the public search API, for shoe, boots, jacket, hoodie, glasses). The CC0 wearables are museum scans: historical shoes, uniform jackets, stage boots, at 100,000 to 1,100,000 faces each. "hoodie" and "glasses" returned nothing. None is a game garment | Each result is labelled "CC0 Public Domain" by the API. The licence must be read on each model's own page before use. **Class 4 until then.** A scan of a branded shoe can still carry trade mark or design rights | visual guidance |
| G-42 Smithsonian Open Access | https://www.si.edu/openaccess/faq | Smithsonian Institution | 2026-11-16 | CC0 museum scans (costume, footwear) | Not opened: the site refused the fetch (HTTP 403), and so did https://3d.si.edu/cc0. Smithsonian items appear in G-41's results with the CC0 label | **Unverified** from the source's own page. Class 4 until read | visual guidance |
| G-43 Poly Haven: Rubber Boots | https://polyhaven.com/a/rubber_boots | Poly Haven | 2026-11-16 | A boot mesh that could ship | Catalogue record read (the public API). A pair of rubber boots, 35,004 polygons, textures to 8K | **Class 1, CC0** (the licence page, F-01). A prop, not fitted to a foot: needs a level-of-detail chain, a split into left and right, and fitting | actual asset |

### 4.2 Fabrics

What makes each fabric read, how simulators describe a fabric, and texture sets that could ship.

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| F-01 Poly Haven: licence | https://polyhaven.com/license | Poly Haven | 2026-11-16 | Licence evidence | Opened. "Our assets are all licensed as CC0." The site's own renders, logos and text are excluded | **Class 1** for the assets | data |
| F-02 Poly Haven: Denim Fabric (and `denim_fabric_03` to `_06`, `denmin_fabric_02`) | https://polyhaven.com/a/denim_fabric | Poly Haven | 2026-11-16 | Denim | Catalogue record read. Denim with folds and stitching, 8K. The catalogue lists it as 100 mm wide (the asset page shows "0.1m"), and `denim_fabric_03` to `_06` as about 255 to 273 mm: set the tiling from the weave by eye, not from the listed size alone. Six denims in the fabric category | Class 1, CC0 (F-01; the asset page also says CC0) | actual asset |
| F-03 Poly Haven: Cotton Jersey, Jersey Melange | https://polyhaven.com/a/cotton_jersey | Poly Haven | 2026-11-16 | Cotton jersey (T-shirts) | Catalogue record read. Scanned knit, about 8K, about 26 cm across: the knit columns are resolved | Class 1, CC0 (F-01) | actual asset |
| F-04 Poly Haven: Polar Fleece, Knitted Fleece, Jogging Melange | https://polyhaven.com/a/polar_fleece | Poly Haven | 2026-11-16 | Fleece and sweatshirt fabric (hoodies, joggers) | Catalogue record read. Three scans at about 8K, about 27 cm across | Class 1, CC0 (F-01) | actual asset |
| F-05 Poly Haven: Brown Leather, `fabric_leather_01`, `fabric_leather_02`, `leather_white` | https://polyhaven.com/a/brown_leather | Poly Haven | 2026-11-16 | Leather (jackets, boots, belts) | Catalogue record read. 8K. Brown Leather is listed as 400 mm (the asset page shows "0.4m"). It and the two `fabric_leather` sets are tagged chair, seat, couch and upholstery: furniture leather, so check that the crease pattern reads as a garment and not as a sofa | Class 1, CC0 (F-01; the asset page also says CC0) | actual asset |
| F-06 Poly Haven: the rest of the fabric category | https://api.polyhaven.com/assets?t=textures&c=fabric | Poly Haven | 2026-11-16 | Other weaves and knits | Catalogue read: 43 fabric textures. Useful: `ribbed_corduroy`, `stretch_poplin` (shirting), `waffle_pique_cotton` (polo), `rough_linen`, `wool_boucle`, `poly_wool_herringbone` (suiting), `terry_cloth`, `velour_velvet`, `bi_stretch` | Class 1, CC0 (F-01) | actual asset |
| F-07 ambientCG: licence | https://docs.ambientcg.com/license/ | ambientCG | 2026-11-16 | Licence evidence | Opened. "All ambientCG assets are provided under the Creative Commons CC0 1.0 Universal License." | **Class 1** | data |
| F-08 ambientCG: Fabric003, Fabric069, Fabric077 | https://ambientcg.com/view?id=Fabric003 | ambientCG | 2026-11-16 | Denim | Catalogue record read (the public API). Three materials tagged "jeans" | Class 1, CC0 (F-07) | actual asset |
| F-09 ambientCG: the Leather materials (numbered up to Leather039) | https://ambientcg.com/view?id=Leather037 | ambientCG | 2026-11-16 | Leather | Catalogue record read. A search for "leather" returned 50 materials, some as lettered colour variants | Class 1, CC0 (F-07) | actual asset |
| F-10 ambientCG: Fabric070; Fabric081A to C and Fabric082A to C | https://ambientcg.com/view?id=Fabric070 | ambientCG | 2026-11-16 | Sportswear mesh; printed fabric | Catalogue record read. Fabric070 is tagged "sport, sportswear". The 081 and 082 sets are a fine weave with and without a plastisol print, some tagged "damaged": a reference for how a chest print sits on jersey | Class 1, CC0 (F-07) | actual asset |
| F-11 Encyclopedia: Denim | https://en.wikipedia.org/wiki/Denim | Wikipedia contributors | 2026-11-16 | What makes denim read | Opened. "A sturdy, cotton, warp-faced textile" in a twill that "produces a diagonal ribbing". "The warp threads are dyed while the weft threads are left white": blue outside, pale inside. Indigo sits on the yarn's surface, so wear shows the pale core, most at "the knees, seams, and pockets" | CC BY-SA 4.0. Class 4 | visual guidance |
| F-12 Encyclopedia: Jersey | https://en.wikipedia.org/wiki/Jersey_(fabric) | Wikipedia contributors | 2026-11-16 | What makes jersey read | Summary read. "A knit fabric used predominantly for clothing manufacture", of cotton, wool or synthetic fibre | CC BY-SA 4.0. Class 4 | visual guidance |
| F-13 Encyclopedia: Polar fleece | https://en.wikipedia.org/wiki/Polar_fleece | Wikipedia contributors | 2026-11-16 | What makes fleece read | Summary read. "A soft fabric made from polyester that is napped and insulating": a raised pile, no visible weave | CC BY-SA 4.0. Class 4 | visual guidance |
| F-14 Encyclopedia: Leather | https://en.wikipedia.org/wiki/Leather | Wikipedia contributors | 2026-11-16 | What makes leather read | Summary read. Tanned hide: "strong, flexible and durable" | CC BY-SA 4.0. Class 4 | visual guidance |
| F-15 Marvelous Designer help: fabric physical properties (stretch, shear, bending) | https://support.marvelousdesigner.com/hc/en-us/articles/47358420149017-FABRIC-PHYSICAL-PROPERTIES-Adjust-Stretch-Weft-Warp-Shear | CLO Virtual Fashion | 2026-11-16 | How a simulator describes a fabric | Search summary (the site refused the fetch). A fabric is stretch along the weft and the warp, shear, bending along the weft and the warp, a buckling ratio and stiffness, and density. Lower stretch stiffness gives "more elastic materials such as silk and jersey". Weft and warp are set apart, as in real cloth | Class 4. Commercial product documentation | visual guidance |
| F-16 CLO help: buckling stiffness | https://support.clo3d.com/hc/en-us/articles/115002685527-Adjust-Buckling-Stiffness | CLO Virtual Fashion | 2026-11-16 | Fold sharpness | Search summary (the site refused the fetch). Buckling stiffness is a share of the bending stiffness, applied where the fabric folds: it decides whether a fold is a crisp crease or a soft roll | Class 4 | visual guidance |

What makes each fabric read, in our own words (from F-02 to F-16, and the shading notes the repository already holds):

| Fabric | Structure to show in the detail normal | Colour | Light response | Folds and edges |
|---|---|---|---|---|
| Denim | Diagonal twill ribs. Count them on the scans (F-02): no source gives a figure, and the first draft's "3 to 4 a millimetre" had none (section 9) | Blue outside, pale inside; fades to the pale core at seams, knees, pocket edges; a contrasting topstitch | Rough, weak sheen | Thick. Wide folds with sharp ridges. Stacks at the ankle. Flat-felled seams read as a raised double line |
| Cotton jersey | Fine vertical knit columns | Even, matt | Rough, a soft sheen at grazing angles | Thin. Clings and drapes in many small folds. Hems are twin-needle lines. The neck is a rib band |
| Fleece and sweatshirt | No weave; a short raised pile | Slightly mottled (melange) | Very rough, strong fuzzy sheen at the silhouette | Thick and soft. Few, round folds. Rib cuffs and hem that gather the body |
| Leather | Grain and pores; creases where it bends | Darker in creases, paler on worn edges | Mid roughness, no sheen, a clear highlight that breaks up along creases | Stiff. Large, sharp, permanent creases at the elbow and across the vamp |
| Rib knit | Raised vertical ribs | As the garment, or a contrast | As jersey | Elastic. Pulls in and grips the wrist, waist or neck |

### 4.3 Accessories

Attachment conventions, sizing, and meshes that could ship.

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| A-01 Roblox: rigid accessory specifications | https://create.roblox.com/docs/art/accessories/specifications | Roblox | 2026-11-16 | Named attachment points and size boxes | Opened. Named attachment points for each accessory type: hat; hair; face front and face centre (glasses, masks); neck; left and right shoulder; left and right collar; body front; body back; waist front, centre and back. "Shoulder" points move with the arm; "Collar" points do not. Each type has a bounding box per body scale, centred on its attachment. One mesh, at most 4,000 triangles, textures to 2048 | Class 4. Our own socket names | visual guidance |
| A-02 Unreal Engine: skeletal mesh sockets | https://dev.epicgames.com/documentation/en-us/unreal-engine/skeletal-mesh-sockets-in-unreal-engine | Epic Games | 2026-11-16 | Socket as a bone plus an offset | Opened. "Sockets are dedicated attach points within the hierarchy of your Skeleton." Each is parented to a bone and stores a position, rotation and scale relative to it. Objects, effects and actors attach to a socket in the level, in a Blueprint or at run time | Class 4 | visual guidance |
| A-03 VRM `VRMC_node_constraint` | https://github.com/vrm-c/vrm-specification/blob/master/specification/VRMC_node_constraint-1.0/README.md | VRM Consortium | 2026-11-16 | Constraints for parts that hang off the rig | Opened. Three kinds, each with a source node and a weight from 0 to 1: roll (one axis of the source's turn: twist bones), aim (point an axis at the source: sleeves, straps), rotation (copy the source's turn). No cycles. VRM itself has no garment or layer declaration: a VRM avatar is one finished model | The specification repository has no licence file (the format notes say so): **unverified**. Class 4 as text | visual guidance |
| A-04 Glasses frame size guide | https://www.allaboutvision.com/eyewear/eyeglasses/styles/glasses-frame-size/ | All About Vision | 2026-11-16 | Glasses that fit a face width | Opened. A frame is three numbers in millimetres: lens width ("40 to 60 mm"), bridge width, "the distance between the lenses" ("14 to 24 mm"), and temple length ("120 to 150 mm"). Two frames with the same numbers can still fit differently | Class 4. Facts only | visual guidance |
| A-05 ANSUR II: body measurement survey | https://www.openlab.psu.edu/ansur2/ | Penn State Open Design Lab (host); US Army (source) | 2026-11-16 | Real ranges for head, wrist and foot sizes | Opened. 93 measures of 4,082 men and 1,986 women, in millimetres, as two CSV files. Published internally in 2012 and "made available publicly in 2017". Military personnel, so not a sample of the general population | The page states no licence: **unverified**. Use for reading ranges; do not commit the files | data |
| A-06 Poly Haven: Round Spectacles | https://polyhaven.com/a/round_spectacles | Poly Haven | 2026-11-16 | A glasses mesh that could ship | Catalogue record read. Round wire frames, 11,810 polygons, 158 mm wide, textures to 8K | **Class 1, CC0** (F-01). Vintage style. Needs a level-of-detail chain and a lens material slot | actual asset |
| A-07 Poly Haven: Digital Wrist Watch | https://polyhaven.com/a/digital_wrist_watch | Poly Haven | 2026-11-16 | A watch mesh that could ship | Catalogue record read. A retro digital watch, 10,390 polygons, laid flat (217 mm long): the strap must be bent round a wrist | Class 1, CC0 (F-01; the asset page also says CC0, and names no brand). The catalogue tags it "casio style": check that no brand mark is on the model or its textures before shipping | actual asset |
| A-08 Poly Haven: Fishermans Hat | https://polyhaven.com/a/fishermans_hat | Poly Haven | 2026-11-16 | A hat mesh that could ship | Catalogue record read. A wide-brim boonie hat, 10,304 polygons | Class 1, CC0 (F-01). Needs hat-safe hair under it | actual asset |
| A-09 MakeHuman asset packs: hats, glasses, masks, gloves, bags | https://static.makehumancommunity.org/assets/assetpacks.html | MakeHuman community | 2026-11-16 | Accessory meshes that could ship | Opened. CC0: Hats 01 (hats and caps), Hats 02 (helmets), Glasses 01, Masks 01, Gloves 01. CC-BY: Hats 03, Hats 04, Glasses 02, Masks 02, Equipment 03 (bags and tools) | Class 1 for the CC0 packs, class 2 for the CC-BY packs. Read on this page and in each pack page's per-asset table (section 9). Quality varies by contributor: review each. An asset named after a film, comic or series character is not cleared by its CC0 label: skip those | actual asset |
| A-10 FreeSewing: accessory designs | https://codeberg.org/freesewing/freesewing/src/branch/develop/designs | FreeSewing | 2026-11-16 | Pattern-built soft accessories | Opened (each `package.json`). `florent` (flat cap), `holmes` (deerstalker hat), `hortensia` (handbag), `magde` (messenger bag), `florence` (face mask) | Class 2, MIT (G-01) | data |
| A-11 Ready Player Me: glasses asset page | https://docs.readyplayer.me/ready-player-me/customizing-guides/create-custom-assets/glasses | Ready Player Me | 2026-11-16 | Glasses requirements on a web avatar platform | Not opened: the host no longer resolves (see G-30) | Class 4, and gone | visual guidance |

### Openverse references for pose correctives (wave 5, decision 14)

Looked at, never shipped (links only; `refinement/wave5/pose-correctives.md`): how a plain T-shirt's sleeve and underarm hang on a person with the arm down.

| What | Openverse | Source page | Author | Licence |
|---|---|---|---|---|
| A man in a plain grey crew-neck tee, arms hanging, three-quarter view: the sleeve hangs loose off the upper arm and the underarm makes one soft fold, with no tight bridge to the side | https://openverse.org/image/0828727f-4725-4c4f-a024-c1664c20e6bf | https://www.flickr.com/photos/45776673@N04/19837504423 | Steven Pisano | CC BY 2.0 |

## 5. Categories, production approach and the first wave

### 5.1 How each garment is built

Starting values for the build, from the sources above and from common pattern practice. They are to be tuned in renders, not measured facts. **Ease** here is the gap between body and cloth at the chest or hip, as a circumference.

| Garment | Panels | Seams | Hems and edges | Closure | Ease |
|---|---|---|---|---|---|
| T-shirt | Front, back, two sleeves, a neck band (G-02) | Shoulder, side, armhole, underarm | Twin-needle hems at sleeve and bottom; rib band at the neck | None | 5 to 15 cm; negative for a fitted one |
| Button shirt | Two fronts, back, yoke, two sleeves, collar and stand, two cuffs, a front placket, sleeve plackets | Yoke to back with a pleat; flat-felled side and sleeve seams | Narrow curved bottom hem; stiff collar and cuffs with an edge stitch | Buttons on the front placket and the cuffs | 10 to 20 cm |
| Hoodie | Front, back, two raglan or set-in sleeves, a hood in two or three pieces, a pouch pocket, rib cuffs and hem band | Overlocked; a cover stitch shows on top | Rib bands that gather the wrist and hip; a hood edge with a drawcord channel | None, or a front zip | 15 to 25 cm |
| Jacket (bomber) | Two fronts, back, two sleeves, rib collar, rib cuffs, rib hem, zip facing, pocket welts | Plain seams with topstitching | Rib bands at neck, wrist and hip | Front zip | 20 to 30 cm, to go over a top |
| Jacket (denim) | Fronts and back in panels, a yoke front and back, two-piece sleeves, a collar, cuffs, a waistband, chest pocket flaps | Flat-felled, twin topstitch in a contrast thread | Waistband with side tabs | Metal shank buttons | 15 to 25 cm |
| Jeans | Two fronts, two backs, a back yoke, a waistband, a fly, five pockets, belt loops | Flat-felled inseam and yoke; a pressed-open outseam; twin topstitch | Double-turned hem with a chain stitch; rivets at pocket corners | Zip or button fly, one waist button | 2 to 6 cm at the hip; the leg decides the style |
| Shorts | As trousers, cut above the knee; or two panels with an elastic waist | Side, inseam, crotch | Turned hem; elastic or drawcord waist | Fly, or none | 6 to 15 cm |
| Skirt (pencil) | Front, back in two, a waistband, darts | Side seams, centre back with a vent | Turned hem | Back zip | 2 to 5 cm |
| Skirt (circle) | One or two circle sectors, a waistband | Side | Narrow hem, a long edge | Zip or elastic | Fitted at the waist only; the rest hangs free |
| Sneaker | Upper: vamp, two quarters, a tongue, an eyestay, a toe cap, a heel counter, a collar lining. Then midsole and outsole (G-20) | Overlays stitched on the upper; upper cemented to the sole | Padded collar; a foxing or sidewall line round the sole | Laces through eyelets | A firm shell: it does not follow soft tissue |
| Boot | As a sneaker, with a shaft above the ankle, a heel block, a welt, a thicker lugged outsole | Stitched welt between upper and sole | A pull loop; a top edge binding | Laces with hooks, a zip, or elastic gussets | The shaft stands off the shin by a few millimetres |

### 5.2 Brief §8.1 and §9 against a production approach

Four ways to make an item, in order of cost:

- **Body-cut.** Copy the body's faces where the garment sits, cut the hems, hold it off the skin. This is what `clothes.py` and the realistic underwear do. Right for tight garments.
- **Pattern, sewn and simulated.** Draft panels from the body's measurements (G-01 or G-04), sew them in Blender with sewing springs (G-07), simulate on the rest pose, then clean, thicken and bake. This is plan D8. Right for anything that hangs.
- **Shell.** Offset the foot or head surface into a firm shell and add built parts (a sole, a brim). This is how `clothes.py` builds its sneakers (`assets/characters/humanoid/clothes.py:459`, `:547`). Right for footwear and hats.
- **Import.** A CC0 or CC-BY mesh, bound to the body on import (CO9).

| Category | Approach | Fit method | Layer | Notes |
|---|---|---|---|---|
| Underwear | Body-cut (exists) | `shapes` today; could stay | 0 | Done for the realistic family: boxer briefs, briefs, sports bra |
| T-shirts | Pattern (`teagan`, or GarmentCode's tee) | `binding` | 1 | The simplest pattern garment. The pilot for the whole pipeline |
| Tops (tank, fitted, sports) | Body-cut | `binding` or `shapes` | 1 | Cheap. A tank top and a fitted long-sleeve come almost free |
| Shirts | Pattern (`simon`, `simone`) | `binding` | 1 | Collar and stand are the hard part: build them as separate stiff pieces, pinned, not simulated |
| Sweaters | Pattern (`hugo`, `hannah`, `sven`) | `binding` | 2 | Rib bands hold the wrist and hip, which helps binding. The hood rests down; it moves only with CO15 |
| Jackets | Pattern (`jett` bomber first; `devon` denim later) | `binding` | 3 | The bomber's rib hem and cuffs stay close to the body. A denim jacket hangs open and is harder |
| Coats | Pattern (`carlton`, `carlita`) | `binding`, with cloth on the skirt of the coat | 3 | Wave 2. ADR-0035 already says long loose garments may need CO15 or the deformer-map fallback |
| Pants | Pattern (`titan` block for jeans; `charlie`; `crux` for cargo) | `binding` | 1 | No open jeans pattern was found: jeans are the trouser block plus a yoke, five pockets and rivets as built detail |
| Shorts | Pattern (`shale`) or the trousers cut short | `binding` | 1 | |
| Skirts "where supported" | Pencil: pattern (`penelope`). Circle: pattern (`sandy`) | Pencil: `binding`. Circle: needs cloth | 1 | A skirt vertex bound to one leg tears between the legs when they part. A skirt needs binding smoothed across both legs (the "robe" idea in the repository's notes) or cloth. Support the pencil skirt first; hold the circle skirt for CO15 |
| Footwear | Shell | Follows the foot bones and the foot-size slider, not the skin | 1 | A shoe must not swell with body fat. Hide the whole `feet` region. Sandals need a finer foot region than the one that exists |
| Full outfits | Not a mesh type | n/a | n/a | An outfit preset is a list of items and variants. A one-piece garment (dress, jumpsuit, overalls) needs an item to fill two slots, which the manifest cannot say today (`item.rs:26`) |
| Glasses, sunglasses | Import (A-06) or built | `socket` on a new nose-bridge socket | none | Scale the width to the face. Sunglasses are a lens material variant of the same mesh |
| Hats, headwear | Shell (cap) or import (A-08, A-09) | `socket` on `HeadSocket` | none | Needs hat-safe hair: either the hat names a hair variant to switch to, or it hides a scalp region of the groom |
| Watches | Import (A-07) or built | `socket` on a new wrist socket | none | Scale with the wrist; the strap is rigid |
| Earrings | Built | `socket` on new ear sockets | none | Wave 2. Tiny: they need their own level-of-detail cut-off |
| Necklaces | Built | `binding` round the neck base | needs an order | A bound accessory has no layer today, so it has no place in the push-out order (`item.rs:227-229`) |
| Belts | Built | `binding` at the waist, over the bottom | needs an order | Same gap. Simplest first version: a belt is part of the trousers, as a variant |
| Bags | Built or pattern (`magde`, `hortensia`) | `socket` on `BackSocket`; straps bound | needs an order for straps | Wave 2 |

### 5.3 First wave for the realistic family

The same 14 items as the plan's CO10, with a source and an order. Build them in this order: each one proves something the next needs.

| # | Item | Slot, layer | Approach and source | Fabric (texture source) | What it proves |
|---|---|---|---|---|---|
| 1 | Crew-neck T-shirt | top, 1 | Pattern: `teagan` (G-01) or GarmentCode's tee (G-04) | Cotton jersey (F-03) | The whole pattern pipeline, and `binding` |
| 2 | Joggers | bottom, 1 | Pattern: `paco` or the `titan` block, with rib cuffs | Sweatshirt fleece (F-04) | Trousers; the waist against the top |
| 3 | Shorts | bottom, 1 | Pattern: `shale` | Cotton twill (F-06) | A hem on the thigh; the `thighs_low` region |
| 4 | Sneakers | shoes, 1 | Shell, as `clothes.py` does | Leather and mesh (F-05, F-10), rubber sole | Footwear fit and coverage |
| 5 | Jeans | bottom, 1 | Pattern: `titan` block plus built detail | Denim (F-02, F-08) | Stiff cloth; stacking at the shoe; seams as baked detail |
| 6 | Long-sleeve shirt | top, 1 | The T-shirt's pattern with long sleeves and rib cuffs | Cotton jersey (F-03) | The sleeve to the wrist; the `forearms` region |
| 7 | Hoodie | outerwear, 2 | Pattern: `hugo` | Sweatshirt fleece (F-04), rib knit | Layer 2 over layer 1; a hood at rest |
| 8 | Bomber jacket | outerwear, 3 | Pattern: `jett` | Nylon shell or leather (F-05), rib knit | Layer 3; a zip; a material variant that changes the look |
| 9 | Cargo trousers | bottom, 1 | Pattern: `crux` or `charlie`, with built pockets | Cotton twill (F-06) | Pockets as geometry |
| 10 | Button shirt | top, 1 | Pattern: `simon` and `simone` | Poplin (F-06) | Collar, placket, buttons, cuffs |
| 11 | Boots | shoes, 1 | Shell; compare with G-43's CC0 mesh | Leather (F-05, F-09) | A shaft over the shin; trousers over or tucked |
| 12 | Cap | accessory | Shell | Cotton twill | `socket`, and hat-safe hair |
| 13 | Glasses | accessory | Built, or import A-06 | Metal or acetate, a lens | A face socket; width fitted to the face; sunglasses as a variant |
| 14 | Watch | accessory | Built, or import A-07 | Metal, glass, a strap | A wrist socket that scales |

Two notes on the list:

- **Both frames.** Every pattern is drafted twice, from the masculine and the feminine body's measurements, and simulated on each. FreeSewing has separate blocks where the shape differs (`simon` and `simone`). The build's `measure.py` already measures the body; it can supply the pattern's inputs.
- **Imported garments are a test, not the wave.** The MakeHuman CC0 packs (G-34) were made for MakeHuman's own base mesh, which is not the realistic body's topology or pose. The repository already maps one onto the other (`assets/characters/transfer/register.py:1-25`), so a MakeHuman garment can be carried across and re-bound. Pick one CC0 shirt and one CC0 pair of shoes as the import flow's test cases for CO9.

**How to check one MakeHuman asset's licence.** The pack page states the licence per pack (G-34). Each pack also has its own page (`/assets/assetpacks/<pack>.html`, such as `shirts01.html`) with a table of asset name, author, source and licence: find the asset's row there first. Then follow the row's source link to the asset's own page in the community asset repository and read its licence field; both must say CC0 or CC-BY (CC BY 4.0 is the version the FAQ links, G-38). Record the author, the asset's page and the licence in the item's `source` (`item.rs:130-135`), which imports must fill. For CC-BY, add the credit line to the engine's notices. If the page shows no licence, or anything else, do not use it.

## 6. Capability manifests: a design note

For the lead to decide. Nothing here is built.

### 6.1 What other systems declare per item

| System | Identity and place | Compatibility | Colour and material | Fit | Layer and exclusion | For the browser |
|---|---|---|---|---|---|---|
| Roblox layered clothing (G-21 to G-23, A-01) | An asset type, which fixes the attachment point | One of 15 standard reference bodies; a size box per body scale | A texture | Inner and outer cage; auto skin transfer; a small bind offset | `Order`; puffiness; shrink | The marketplace's own data |
| The Sims 4 (G-31 to G-33) | Body type and sub-type | Age, gender and species flags; an opposite-gender part; a fallback part | A swatch list; each swatch is its own part on the same mesh; optional hue, saturation, brightness and opacity sliders | Vertex colour for tight or loose; a second UV set into the body's deformer maps | A sort layer; flags for the slots it excludes; flags for the slider regions it switches off; a region map | Tag flags, sort priority, a thumbnail |
| MetaHuman outfit (G-25) | An outfit asset | Several authored sizes, each for a source body | A colour parameter that must match across sizes | Warp from the nearest size | A hidden face map over the body's UVs | n/a |
| Unreal Mutable (G-26, G-27) | An option of an object group | By the graph | Typed parameters: enumeration, float with a range, colour, projector, texture | Mesh reshape nodes | Remove-mesh and clip modifiers | Section, order, thumbnail, tags, free key-values, on every parameter and option |
| MakeHuman clothes (G-24) | A file per garment | The one base mesh | A material file | Each vertex bound to three body vertices | A delete group; a z-depth number | Tags |
| glTF variants (G-28) | n/a | n/a | Named variants that swap whole materials | n/a | n/a | The variant's name |
| Daz auto-fit (G-29) | A content type | A figure generation | n/a | Projection of weights, morphs and groups, by a template for the garment's shape; a smoothing and collision modifier | n/a | n/a |
| VRM (A-03) | None: one finished avatar | n/a | n/a | Spring bones and node constraints | n/a | Licence fields in the file |

Three things every working system has in common:

- The item says what it **covers and excludes**, not just where it sits.
- Colour choices are **premade and named** (swatches, variants), with free sliders as an extra that only some items allow.
- The interface is built **from the item's own data** (Mutable's UI metadata is the clearest case).

### 6.2 A recommended minimal set

The item manifest already carries most of brief §14.1: `slot`, `layer`, `bases`, `hides`, `materials`, `variants`, `fit`, `tags`, `thumbnail`, `source`. The advice is to **extend those fields, not add a parallel "capabilities" block**: one place per fact. Every addition is optional, so by architecture §4 none is a version change, and an item that declares nothing behaves as it does today.

| Need (brief §8.3, §8.4, §9, §14.1) | Today | Proposed addition | The interface shows |
|---|---|---|---|
| Supported colour slots: primary, secondary, accent | A material slot is `{name, shading}` | `materials[].role`: `primary`, `secondary`, `accent`, or absent. Absent means not user-editable. At most one slot per role. `materials[].label` for the control's name | One colour control per declared role, named by the label. No control for a slot with no role |
| Which values a user may change, and how far | Any value of the shading model, over its whole range | `materials[].editable`: a map of value name to `true` or a narrowed `[min, max]`, such as `{"tint": true, "roughness_scale": [0.8, 1.2]}` | A roughness slider only where it is listed, clamped to the item's range |
| Metallic "where appropriate" | No such value in any shading model | A `metallic_scale` value on the `standard` model (zips, buckles, frames). A shading-model change, owned by the materials work, not by the manifest | A slider only on items that list it |
| Material choices and texture variants | `variants[]` set values per slot | `variants[].group`: `colour`, `material` or `pattern`, so one item can offer a fabric choice and a colour choice side by side. `variants[].textures`: slot to a texture set. `variants[].swap`: slot to a material name in the model file, which is what a glTF variant imports as (G-28) | Swatches grouped by kind |
| Fit parameters | None | `fit.parameters[]`: `{id, label, range, default, shapes[]}`, the same shape as a base parameter's shape drivers (`manifest.rs:110`, `:173`). Examples: sleeves rolled, zip open, hem tucked. Stored on the worn item as `fit: {id: number}` | A slider per declared parameter; none otherwise |
| Compatible families | `bases[]`: exact base IDs | Keep `bases`. Add `fit.validated`: body parameter ID to the `[min, max]` the item was checked over, such as `{"fat": [-1, 0.6]}` | A "not checked for this body" note outside the range; the item still wears |
| Layer | `layer` 0 to 3, garments only | Allow `layer` on a **bound** accessory (belt, necklace, bag strap), so it has a place in the push-out order | n/a |
| Hidden regions | `hides[]` | Keep. Add `occupies[]`: other slots this item also fills (a dress fills `top` and `bottom`). Add `excludes[]`: slots that cannot be worn with it (a hood-up hoodie and a hat) | Wearing it takes off what it fills or excludes, and says so |
| Hair under a hat | Nothing | `requires.hair`: `hat_safe`. A groom declares a `hat_safe` variant or a scalp region it can drop | The hair switches to its hat-safe form; a warning when the groom has none |
| Attachment | `fit.socket` | `fit.offset`: `{position, rotation, scale}`, the item's rest placement on the socket | n/a |
| Adjustable offset ranges | Nothing | `fit.adjust`: `{position: [min, max] per axis, rotation: degrees, scale: [min, max]}`. Stored on the worn item as `offset` | Position, rotation and scale controls only for the axes with a range. Glasses: up and down the nose, and width. A watch: along the arm |
| Glasses that fit a face width | Nothing | `fit.scale_from`: a named measure of the base, with the item's reference value, such as `{"measure": "face_width", "reference": 0.138}`. The base manifest would list its measures as distances between two landmarks, so they follow the face sliders | Nothing: it fits by default; the width control is a correction on top |
| Limitations | Nothing | `limitations[]`: `{code, message}`. Codes are a short fixed list the browser can filter on (`body_range`, `needs_cloth_sim`, `single_frame`, `no_layer_above`); the message is for people | The compatibility line in the browser (brief §8.2) |
| Dependencies | Nothing | `requires.items[]` (a holster needs its belt) and `requires.features[]` (`cloth_sim`) | The item is greyed with the reason when a feature is missing |

A small example. A bomber jacket with three colour roles, two fabric choices, a zip, and a body range it was checked over:

```json
{
  "format_version": 1,
  "id": "axcenity/realistic-bomber",
  "label": "Bomber jacket",
  "kind": "garment",
  "slot": "outerwear",
  "layer": 3,
  "bases": ["axcenity/realistic"],
  "model": { "file": "../RealisticWardrobe.glb", "object": "AxcenityRealistic/Jacket_Bomber" },
  "hides": ["torso", "upper_arms_top", "upper_arms_low", "forearms"],
  "materials": [
    { "name": "Shell", "shading": "cloth", "role": "primary", "label": "Shell",
      "editable": { "tint": true, "roughness_scale": [0.8, 1.2] } },
    { "name": "Rib", "shading": "cloth", "role": "secondary", "label": "Rib knit",
      "editable": { "tint": true } },
    { "name": "Zip", "shading": "standard", "role": "accent", "label": "Zip",
      "editable": { "tint": true } },
    { "name": "Lining", "shading": "cloth" }
  ],
  "variants": [
    { "id": "nylon", "label": "Nylon", "group": "material", "textures": { "Shell": "textures/bomber_nylon" } },
    { "id": "leather", "label": "Leather", "group": "material", "swap": { "Shell": "Shell_Leather" } },
    { "id": "olive", "label": "Olive", "group": "colour",
      "materials": { "Shell": { "tint": "#5b6340" }, "Rib": { "tint": "#4a5136" } } }
  ],
  "fit": {
    "method": "binding",
    "data": "bomber.binding",
    "margin": 0.003,
    "parameters": [
      { "id": "zip", "label": "Zip", "range": [0, 1], "default": 1,
        "shapes": [{ "shape": "Fit_Zip_Open", "from": 1, "to": 0 }] }
    ],
    "validated": { "frame": [0, 1], "fat": [-1, 0.6] }
  },
  "limitations": [
    { "code": "body_range", "message": "Not checked above 0.6 body fat; may ride up over the belly." }
  ],
  "tags": ["casual", "zip", "ribbed"],
  "source": { "author": "Axcenity Engine contributors", "licence": "CC0-1.0" }
}
```

And glasses, as the worn item would sit in a description:

```json
{ "id": "axcenity/glasses-round", "variant": "tinted", "offset": { "position": [0, -0.002, 0], "scale": 1.04 } }
```

### 6.3 Points to decide

1. **In the fields, or in a block.** This note extends the existing fields. A separate `capabilities` object would be easier to version on its own but states some facts twice.
2. **Unknown fields are lost on write.** The description keeps fields it does not know (`description.rs:39`). The item manifest does not. Architecture §4 says manifests "ignore them", which is true on read; but an older editor that reads and rewrites an item drops every new field. Either give `ItemManifest` the same `extra` map, or state that only scripts and the importer write item manifests.
3. **The worn item needs two optional fields**, `fit` and `offset` (`description.rs:45-51`). That is additive to format 2. An older build reading the file would drop them, for the same reason as point 2.
4. **Variants today are one flat list** and a worn item names one (`description.rs:48`). Grouped variants mean a worn item may name one per group. The smallest change: `variant` stays, and a `variants` list is added beside it.
5. **Accessory slots and sockets do not exist yet.** Both bases need slots in the `accessories` category (the test fixture already has a `wrist` slot that holds several, `crates/avatar/src/fixtures.rs:56`) and sockets at the nose bridge, both ears, the neck base and both wrists. Slots and sockets are data in the base manifest, so this is a build-script change plus rig work, not a format change.
6. **Bound accessories need an order.** Either allow `layer` on accessories that use `binding`, or model belts and necklaces as garments in accessory slots.
7. **Role names.** `primary`, `secondary`, `accent` are the brief's words (§8.3). Three is enough for the first wave; a fourth should not be added until an item needs it.

## 7. Fit review checklist

For each item, on **both frames**, at the **composition extremes** (muscle and fat at each end, and each regional slider at its end), as CO9's acceptance asks (`testing.md:60`). Measure first, then look. The tools exist: `assets/characters/humanoid/qa/measure.py --only clipping` against `thresholds.json`, `posed_crossings.py` for poses, and the visual QA poses (`visual-qa.md:16`).

**Poses to check:** rest (A-pose), arms raised to 150 degrees, elbow bent to 135, a deep squat (hip to 100, knee to 135), a 45 degree spine twist, a seated pose, and the walk, run and idle clips.

| Check | Pass when |
|---|---|
| **Clipping** | |
| Skin through cloth | No body point outside the garment where the garment covers it, at rest and in every pose. Counts and depths per item stay under its threshold. A limit is lowered when it improves and never raised without a render |
| Hidden regions | Every region the item hides is fully covered by it at every extreme. No skin is hidden that the garment leaves bare (a cropped top must not hide the waist) |
| Region edges | No gap of missing skin at a hem: look up a sleeve, down a collar, up a trouser leg |
| Every level of detail | The check passes with the body and the garment at each pairing of levels the renderer can show. This is an open defect today (`progress.md:118`) |
| Self-intersection | No fold passes through another part of the same garment; collar, pocket flaps and hood do not sink into the body of the garment |
| **Layering** | |
| Order | A lower layer is never outside a higher one. Shirt under hoodie under jacket; trousers under a top's hem or tucked, as the item says |
| Bulk | An outer layer grows by about the thickness of what is under it, and no more. A jacket over a hoodie does not balloon |
| Every legal pair | Each top with each outerwear, each bottom with each top, each shoe with each bottom. A pair that cannot work is declared in `excludes`, not left to clip |
| Edges of covered layers | An inner sleeve does not poke out past an outer cuff unless it is longer by design |
| **Sleeves and waist** | |
| Armhole | With the arm raised, the side seam lifts and the armpit does not tear, spike or show skin |
| Elbow and cuff | The sleeve keeps its volume at 135 degrees. The cuff stays round and at the wrist, at every arm length |
| Shoulder | The shoulder seam sits on the shoulder point at every shoulder width, on both frames |
| Waist | Trousers hold at the waist at every waist and hip width, with no ring of skin and no waistband inside the belly |
| Top over bottom | The top's hem covers or meets the waistband in the squat and with arms raised, unless it is a cropped item |
| Chest | On the feminine frame the cloth bridges between the breasts and does not wrap each one |
| Crotch and seat | The cloth bridges the creases at rest and does not web or tear in the squat |
| **Footwear coverage** | |
| Foot | No toe, heel or ankle bone through the upper at every foot size |
| Sole | The sole is flat on the ground plane at rest. No foot inside the sole |
| Ankle | No gap of hidden skin between the shoe's collar and a bare ankle. With trousers: the hem rests on or over the shoe and neither passes through the other |
| Boot shaft | The shaft clears the shin and calf at the fat and muscle extremes. Trousers tuck in or sit over, as declared |
| Size | The shoe follows foot size and leg length, and does not swell with body fat |
| **Stability in animation** | |
| Loop | Through the walk, run and idle clips: no vertex jumps, no flicker of skin, no hem that swims against the limb |
| Joints | Volume is kept at the elbow and knee; no candy-wrapper twist at the forearm or the thigh |
| Rigid parts | Buttons, zips, pocket flaps, the collar and shoe soles keep their shape: they move, they do not stretch |
| Sockets | A hat, glasses and a watch stay put through head turns and arm swings, and through the face's expressions |
| Hat and hair | With a hat on, no hair passes through the crown or the brim |
| Rebuild | Changing a body slider and changing it back gives the same fitted mesh. A slider drag does not make the garment pop |
| **Declared against real** | |
| Manifest | The item's `hides`, `layer`, `fit.validated` and `limitations` match what the checks found. A failure outside the declared range is a limitation to write down; a failure inside it is a defect |
| Materials | Every colour role tints the part its label names. Each variant renders, on each frame |

## 8. Not verified

- The Smithsonian Open Access terms (G-42): the site refused every fetch.
- Marvelous Designer's and CLO's help pages (F-15, F-16): refused; search summaries only.
- Ready Player Me's documentation (G-30, A-11): the host no longer resolves.
- The licence of GarmentCodeData (G-05; its record page returned a server error in the second pass too), of the Mutable wiki (G-26, G-27), of FreeSewing's documentation text (G-02, G-03) and of the ANSUR II files (A-05). The MakeHuman pack pages say "CC-BY" with no version; the community FAQ (G-38) links to version 4.0, which is the best evidence there is until an asset's own page is read.
- Whether FreeSewing's `paco` has an elastic cuff option, and how much of GarmentCode's pattern library runs without its simulation dependencies. Both need a trial before the pipeline is planned around them.
- The international standard for spectacle frame measurement: its catalogue page refused the fetch, so it has no row. A-04 gives the same three measures.
- No Blender Studio garment breakdown and no ArtStation breakdown is indexed: none was opened in this pass. The reference library kept outside the repository holds the ArtStation picks that [research.md](../../research.md) A.4 describes.
- Individual Sketchfab and Quaternius assets: only the pack or catalogue level was read. MakeHuman assets were read one level deeper, in each pack page's per-asset table (section 9), but not on each asset's own repository page. Each asset is checked on its own page before it is used.

## 9. Verification

A second pass on 2026-11-16 and 17 re-opened the sources and the code. It changed the rows and sentences listed under "Corrections". Everything else in this section was confirmed as written.

### Licences confirmed on the source's own page

| Entry | Where it was read | Result |
|---|---|---|
| G-01, A-10 FreeSewing | `LICENSE` at the Codeberg repository root; the `license` field of 34 designs' `package.json` (every design this file names) | MIT. All 34 design folders exist. The GitHub copy is archived, last pushed 2025-04-02 |
| G-04 GarmentCode | `LICENSE` in the repository; `LICENSE.md` of the Warp fork | MIT for the repository. The fork's licence limits use to "non-commercially", meaning research or evaluation only (its clause 3.3): the simulation step stays out |
| F-01 to F-06, G-43, A-06 to A-08 Poly Haven | The licence page; the catalogue record of each of 24 named textures and the 4 models; the asset pages of `denim_fabric`, `brown_leather` and `digital_wrist_watch` | CC0. 43 textures in the fabric category, six of them denim. Polygon counts and sizes match the rows |
| F-07 to F-10 ambientCG | The licence page; the catalogue record of each named material | CC0 1.0. Three materials carry the "jeans" tag (a search for "denim" returns none); 50 leather materials, numbered to Leather039 |
| G-34, A-09 MakeHuman packs | The asset packs page; each of the 31 named packs' own page | Every asset row says its pack's licence: 148 assets across the 14 CC0 packs, 253 across the 17 CC-BY packs. No mixed pack |
| G-37, G-38 MakeHuman | The licence page and the FAQ | As quoted. The FAQ's links point to CC BY 4.0 |
| G-39 Quaternius | The pack page and the site FAQ | CC0 |
| G-40 Kenney | The support page | CC0 |

No usable entry had to be moved down a class.

### Corrections

| Where | Was | Now | Why |
|---|---|---|---|
| F-02 | Denim Fabric "1 m across" | 100 mm in the catalogue, "0.1m" on the asset page | The catalogue's sizes are in millimetres (its API schema says so) |
| F-05 | Brown Leather "4 m across" | 400 mm, "0.4m" on the asset page; tagged as upholstery | Same unit error |
| Fabric table, denim | "about 3 to 4 [twill ribs] a millimetre" | No figure; count on the scans | No source gave it. It is nearer a yarn count than a rib count |
| G-09 | "Fine-tuning one outfit can take a working day. Retopology ... the hard part" | One simulation takes at least a working day; tuning takes more; retopology and export are occasional difficulties | The article says the second, not the first |
| A-02 | "A preview mesh can be shown on a socket while placing it" | Removed | Not on the page |
| A-01 | "One attachment name per accessory type" | "Named attachment points for each accessory type" | Waist has three and shoulder five |
| G-03 | A phrase in quotation marks | A paraphrase | The page's wording differs slightly |
| G-04 | Publisher given as a university lab | The repository's author, as `LICENSE` names the holder | The README names no institution |
| G-28 | Licence "unverified here" | Khronos copyright, with a conditional licence for the unmodified specification | Read in the document |
| G-34, A-09, section 5.3 | Licence read at pack level only | Read per asset on each pack page; CC-BY version 4.0 per the FAQ; a faster way to check one asset | The pack pages carry a per-asset table |
| G-39 | Formats "FBX and glTF" | FBX, OBJ, Blend and glTF | The page lists four |
| Section 2 | The mods policy forbids using logos and trademarks | It forbids using them to promote a mod; the user agreement's exceptions are stated | Closer to the pages |
| Section 6.2 example | "above 60% fat" | "above 0.6 body fat" | The `fat` parameter runs from -1 to 1 (`assets/characters/realistic/shapes.py:172`), so 0.6 is not a percentage |
| Top of the file | "No text or picture was copied" | Short quotations are present and marked | Accuracy |

### Numbers

Confirmed in the cited source:

- A-04: lens width 40 to 60 mm, bridge 14 to 24 mm, temple 120 to 150 mm.
- A-05: 93 measures; 4,082 men and 1,986 women.
- G-02: 13 required measurements; a neck strip 6 cm wide; 130 to 200 g/m².
- G-31: every triangle target in the row (tops 3,300 and 1,600; bottoms 1,200 and 770; full body 5,800 and 3,300; shoes 800 and 700; hats 1,200 and 1,000; earrings and necklaces 1,200 and 800), the three texture sizes, and "at least two" colour variants. The document also allows up to 10 percent over target.
- G-22, A-01: 4,000 triangles, 2048 textures, an 8-stud box for clothing.
- G-41: the CC0 results run from about 119,000 to 1,127,000 faces; "hoodie" and "glasses" return nothing.
- Poly Haven: 35,004, 11,810, 10,390 and 10,304 polygons; 158 mm and 217 mm.

Without a source, and to be read as such:

- The ease values in section 5.1. The file already calls them starting values.
- The twill rib count for denim (removed above).
- `"reference": 0.138` for face width in section 6.2. It is an example value, not a measured mean. Take the real one from the base model, or from A-05 once its terms are known.

### Sample of other rows

Opened again and found as described: G-07 (its quotations, and the CC-BY-SA 4.0 footer), G-08, G-10, G-17, G-21, G-22, G-23, G-25, G-27, G-29, G-32 (GPL version 3 or later, in the file header), G-33, G-35, G-36, A-03 (the specification repository has no licence file), A-04, A-05, F-11. G-06 is GPL-3.0 by its catalogue record. G-30: several press reports give 2026-01-31 for the closure, after an acquisition announced in 2025-12.

Still refused or failing: G-42 (HTTP 403) and G-05 (HTTP 500). Both stay unverified.

### Index hygiene

- All 70 rows have eight filled columns, and the last is one of "data", "visual guidance" or "actual asset".
- No row from a commercial game or a commercial character product is above class 4. The file has no row from 3D.sk, Anatomy for Sculptors, ArtStation, TexturingXYZ, Character Creator or Pinterest.
- The folder holds this file only. No personal details, no local paths.

### Code claims

Every `path:line` in section 3 and section 6 was opened and matches, including: the manifest's fields (`crates/avatar/src/item.rs:19-57`), the fit methods (`:83-99`), the layer rule (`:217-230`), a binding item resolving as a skinned part (`crates/avatar/src/resolve.rs:298-301`), the worn item (`crates/avatar/src/description.rs:45-51`), the seven slots and seven sockets (`assets/characters/humanoid/manifests.py:29-37`, `:87-95`), the accessory slot in the test fixture (`crates/avatar/src/fixtures.rs:56`), and the three realistic items. No item manifest under `assets/characters/` has a `variants` key. The region IDs in the example (`torso`, `upper_arms_top`, `upper_arms_low`, `forearms`) exist (`assets/characters/realistic/regions.py:40-42`).

### Found and not added as rows

Poly Haven's model catalogue also holds Garden Gloves 01 (6,822 polygons), a Life Jacket (8,968) and two pocket watches, all CC0 by the same licence page. They are props, like the four indexed. Worth a look when CO16 picks accessories.
