# Hair and grooming references

Stage 0 of the Character Studio refinement ([brief](../../refinement/brief.md) §7, §13.1 "Hair", §13.4, §13.5). Collected 2026-11-16. Research only: no source, manifest or plan was changed, and no image or asset was downloaded into the repository.

This file adds to [notes/hair.md](../../notes/hair.md) (2026-11-14: Unreal Groom, strand renderers, shading models, card alpha handling, simulation, interchange, curly and coily hair). It does not repeat it. Rows that only point back to those notes say so.

**Words used here.** "Opened" means the page text was read in this session. "Search summary only" means the page could not be opened and only a search result was seen. "Unverified" means the licence was not read on the source's own page; treat it as unusable until someone reads it.

**Licence classes** (brief §13.5): 1 CC0 or public domain; 2 open licence compatible with an Apache-2.0 engine that redistributes the asset (which licence is stated per row); 3 licensed commercial; 4 reference only (links, never files); 5 original, made in this repository.

## 1. What the repository has today

| Fact | Evidence |
|---|---|
| Six hair styles, all sculpted solid masses (a scalp cap plus tube clumps, voxel-remeshed), 2,500 to 10,000 triangles each | `assets/characters/humanoid/hair.py:158-159` (budgets), `:164-182` (`finish`), `:331-332` (`STYLES`) |
| All six fit the stylized base only. The realistic base has a `hair` slot and no groom that fits it, so the default family is bald | `assets/characters/humanoid/items/hair-*.item.json:9` (`bases`); `assets/characters/realistic/Realistic.base.json:3889`; `assets/characters/realistic/README.md:186` |
| Each is a groom item with a `mesh` tier only: no cards, no strands | `assets/characters/humanoid/items/hair-bob.item.json:29-34` |
| A groom item can name `cards`, `strands` and `mesh`; cards or a mesh are required | `crates/avatar/src/item.rs:114-125`, `:194-213` |
| An item has materials, variants, tags, a thumbnail and a source licence. It has **no field for per-style parameters** (length, volume, parting and so on) | `crates/avatar/src/item.rs:19-58` |
| The `hair` shading model has eight values: `tint`, `melanin`, `redness`, `dye`, `dye_amount`, `tip_lightening`, `highlight` (a strength), `roughness_scale` | `crates/avatar/src/material.rs:226-271` |
| The renderer has no hair shading yet. Hair draws as Standard with `tint` and `roughness_scale` only | `crates/scene/src/material.rs:151-164` |
| Cut-outs cast solid shadows in shadow maps: the raster shadow pipeline has no fragment stage. Ray-traced sun shadows (optional, on GPUs with ray queries) already follow a cut-out's holes, from a copy of the texture's alpha shrunk to 512 texels a side. So slab shadows remain on GPUs without ray queries, past the ray tracing distance, and for spot and point lights | `crates/render/src/renderer/gpu.rs:537-626` (`fragment: None` at `:625`); `crates/render/src/renderer/ray_cut_outs.rs:1-22`, `:32` (`ALPHA_SIZE`); `docs/rendering/ray-tracing.md:22`, `:61-63` |
| Dither noise is fixed per pixel (no frame index), so alpha cannot yet be resolved by TAA | `crates/render/src/lighting.wgsl:236-238`; `crates/render/src/shader.wgsl:786-791` |
| No spring bones exist in `anim` or `gameplay` | no match for "spring" under `crates/anim/src`, `crates/gameplay/src` |
| Brows and lashes on the realistic head are already cards with procedurally drawn alpha textures, bound to the skin, with the `hair` shading model | `assets/characters/realistic/lashes.py:54-73` (`_strands`), `:108` (`brow_texture`), `:340` (`_brow_card`); `assets/characters/realistic/manifests.py:78` |
| Neither base model has a facial hair slot. Slots can be marked `multiple` | `Realistic.base.json:3889` (seven slots: `hair`, two underwear, `top`, `outerwear`, `bottom`, `shoes`); `crates/avatar/src/manifest.rs:273-282` (`multiple` at `:278`) |
| A hairline table and a part position already exist in the generator | `assets/characters/humanoid/hair.py:35-37` (`Head.line`), `:194`, `:228` (`part_x`) |
| The contract: cards first, generated offline from the same strands; strands optional; mesh fallback; facial hair is a groom in a face slot | [architecture.md](../../architecture.md) §11; plan D10, CO11, CO14 |
| Budgets: hero cards up to 40k triangles, gameplay LOD0 up to 25k, crowd 3k (Expected, not Measured) | `plans/axcenity-engine-character-studio.md:193` |

## 2. Reference index

Every row was collected on the date shown. "Use" is one of: visual guidance, data (numbers, parameter names, method), actual asset.

### 2.1 Engine and tool documentation

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Hair Card Generator for Grooms | https://dev.epicgames.com/documentation/en-us/unreal-engine/hair-card-generator-for-grooms-in-unreal-engine | Epic Games | 2026-11-16 | Cards from strands | Opened. Experimental. Stages: strands to clumps, clumps to card geometry, cards to texture islands in one atlas. A "multi-card clump" is three cards for one volume of hair. Settings per filter: Num Clumps, Num Flyaways, Num Triangles, Num Textures; quality is the balance of clumps against triangles | Class 4. Documentation, proprietary engine | Data |
| Creating Hair Cards and LODs Using Hair Card Generator | https://dev.epicgames.com/documentation/unreal-engine/creating-hair-cards-and-lods-using-hair-card-generator | Epic Games | 2026-11-16 | Card layers, LODs | Opened. Five card groups with example LOD0 numbers (clumps / triangles / texture islands): Coverage 50 / 2,000 / 25; Mid 200 / 7,500 / 60; Top 500 / 25,000 / 60; Flyaways one card per strand, 371 strands / 10,000 / 25; Short hairs 500 / 10,000 / 75. Coverage is wide cards with repeated textures so visible layers get the budget. LOD1 either halves triangles and keeps textures, or builds fewer cards with new islands in the same atlas | Class 4 | Data |
| Hair Card Generator in Dataflow (MetaHuman) | https://dev.epicgames.com/documentation/metahuman/hair-card-generator-in-dataflow-in-unreal-engine | Epic Games | 2026-11-16 | Card layers, buns, beards | Opened. Card groups are authored in the DCC with the `groom_group_cards_id` attribute. Layers named core, mid, top, flyaways, short hairs. Example target "fewer than 50,000 triangles" for an updo with a messy bun. Tuning rules: jagged cards, fewer clumps; cards too thin, more clumps; core cutting through mid, more core triangles. A full beard example uses four LODs sharing one atlas | Class 4 | Data |
| Platform Support and LOD Specifications for MetaHumans | https://dev.epicgames.com/documentation/en-us/metahuman/platform-support-and-lod-specifications-for-metahumans | Epic Games | 2026-11-16 | Budgets | Opened. Guide values per LOD 0 to 7. Hair cards: 30,000 / 15,000 / 10,000 / 3,000 / 1,500 vertices at LOD 0 to 4. Hair mesh: 500 / 250 / 100 vertices at LOD 5 to 7. Facial hair cards: 15,000 / 7,000 / 3,000 / 1,000 / 500 vertices. Hair physics on LOD 0 to 2 only. Mobile: cards only, best LOD 3 | Class 4 | Data |
| Hair Overrides (MetaHuman) | https://dev.epicgames.com/documentation/metahuman/hair-overrides | Epic Games | 2026-11-16 | Colour controls per style | Opened. Colour is Melanin (0 white to 1 black), Redness, Dye Color, Roughness, plus Whiteness and Lightness for grey hair. Secondary colours each use the same melanin, redness and dye set: Ombre (Shift, Contrast, Intensity), Region (mohawk styles), Highlights (Balance, Intensity, Variation 0 to 3). "The attributes available to override may differ between hair items": only some styles have ombre or highlights | Class 4 | Data |
| Controlling MetaHuman LODs | https://dev.epicgames.com/documentation/metahuman/controlling-metahuman-levels-of-detail-lods-in-unreal-engine | Epic Games | 2026-11-16 | LOD chain | Opened in verification. The face and all hair grooms can use up to 8 LODs, the body parts 4. Strand grooms can be forced to cards. The page calls LOD 3 "the highest quality LOD for card-based hair geometry" when excluding strands; it does not give a table of strands against cards per LOD (the guide table two rows up lists card vertices from LOD 0) | Class 4 | Data |
| Setting Up Cards and Meshes for Grooms; Groom LOD; Alembic groom schema | https://dev.epicgames.com/documentation/unreal-engine/setting-up-cards-and-meshes-for-grooms-in-unreal-engine | Epic Games | 2026-11-14 | Card atlas content, LODs | Already summarised in notes/hair.md §1 and §6: card textures are Depth, Coverage, Tangent, Attributes (root UV, root-to-tip coordinate, seed), Auxiliary | Class 4 | Data |
| MH Groom Hairstyle Generator | https://dev.epicgames.com/documentation/metahuman/mh-groom-hairstyle-generator | Epic Games | 2026-11-14 | Curl and coil parameters | Already in notes/hair.md §10: coil amplitude, frequency, randomness; tuft and curl controls | Class 4 | Data |
| Hair Card Generate (SOP) | https://www.sidefx.com/docs/houdini/nodes/sop/haircardgen.html | SideFX | 2026-11-16 | Cards from strands | Opened. "Converts dense hair curves to a polygon card, keeping the style and shape of the groom": cluster nearby curves, one card per cluster, width from the cluster's spread, divisions across and along, UV layout with padding | Class 4. Documentation, commercial tool | Data |
| Work with XGen interactive grooming modifiers (Maya 2022 help) | https://help.autodesk.com/cloudhelp/2022/ENU/Maya-CharEffEnvBuild/files/GUID-FAD2F9C3-F6DA-4831-81C1-FC3010FF8F77.htm | Autodesk | 2026-11-16 | Groom as a modifier stack | Opened. Modifiers stack; each takes the output of the one below; reorder, disable or duplicate without losing settings. The modifier list (Clump, Cut, Noise, Coil, Guide, Linear Wire) was seen in search results and artist breakdowns only | Class 4. Documentation, commercial tool | Data |
| Content Dev: CC 3.4 Hair Creation Process | https://wiki.reallusion.com/Content_Dev:CC_3.4_Hair_Creation_Process | Reallusion | 2026-11-16 | Card texture set | Opened. Maps per card atlas: flow, ID, opacity, root, optional base colour; AO is named as a legacy channel and depth as a map the shader does not use. 2048 x 4096 for long hair. Strands at least 2 pixels wide. 32 pixels of padding on every map except opacity (padding opacity thickens strands). 10 to 20 pixels between UV islands. Growth must run top to bottom so the flow map works | Class 4. Commercial product documentation | Data |
| SkinGen: Body Hair layer settings | https://manual.reallusion.com/Character-Creator-4/Content/ENU/4.0/15_SkinGen/Skin/Body-Hair/Settings-in-Appearance-Editor-Hair.htm | Reallusion | 2026-11-16 | Short hair as a skin layer | Opened. Up to three stacked hair layers on the skin material. Each has primary and secondary colour, opacity, a growth range mask, a normal strength (with an invert switch the manual advises leaving off, because hair sits on the skin), roughness, and a scale for the whole pattern. One "Shaving Pattern" mask, shared by the three layers, removes hair by region | Class 4. Commercial product documentation | Data |

### 2.2 Blender

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Bundled hair node groups (Essentials library, `datafiles/assets/nodes/procedural_hair_node_assets.blend`) | https://projects.blender.org/blender/blender-assets | Blender Foundation | 2026-11-16 | Strand generation for every style | Read from an installed Blender 5.3.0 (build 2026-11-10), not from the web page (it returned HTTP 403, to the verifier too). 26 node group assets (re-counted in verification). Inputs that matter here: Generate Hair Curves (Density, Density Mask, Hair Length, Control Points); Interpolate Hair Curves (Part by Mesh Islands, Interpolation Guides, Density Mask); Trim Hair Curves (Length Factor, Replace Length, Length, Mask); Curl Hair Curves (Radius, Frequency, Curl Start, Random Offset); Braid Hair Curves (Radius, Frequency, Thickness, Flare Length, Hair Tie); Clump, Frizz, Roll, Hair Curves Noise, Smooth, Straighten, Shrinkwrap, Duplicate, Set Hair Curve Profile, Attach Hair Curves to Surface | **Class 1.** `datafiles/assets/LICENSE` in the install is the CC0 1.0 Universal legal code. Each node group's asset metadata reads licence "CC0 - Public Domain", copyright "Blender Foundation" | Actual asset (build input; may be vendored) |
| Hair Nodes (manual) | https://docs.blender.org/manual/en/latest/modeling/geometry_nodes/hair/index.html | Blender Foundation | 2026-11-16 | Node list | Opened (navigation only; node pages were too long for the fetcher). Categories: Deformation, Generation, Guides, Read, Utility, Write | Class 4 for the manual text (its own licence was not read) | Data |
| Procedural Hair Nodes (blog) | https://studio.blender.org/blog/procedural-hair-nodes | Blender Studio (S. Thommes), 22 Feb 2023 | 2026-11-16 | Why the nodes exist | Opened. "A set of node-group assets for procedural grooming with Geometry Nodes to be bundled with Blender"; first built-in essential assets | Class 2 for the text and images, by the site-wide Studio terms below (CC BY 4.0); the page itself states no licence. No asset on the page | Visual guidance |
| Introducing Hair Assets in Blender 3.5 (training) | https://studio.blender.org/training/geometry-nodes-from-scratch/introducing-hair-assets-in-blender-35 | Blender Studio (S. Thommes) | 2026-11-16 | Workflow | Opened. Page shows CC-BY for the tutorial, credit to Blender Studio (the version, 4.0, comes from the Studio terms). Links its demo files to blender.org | Class 2 for the tutorial: CC BY 4.0, credit Blender Studio. This does not cover the linked demo files | Visual guidance |
| Hair node demo files (hair style and fur example `.blend` files) | https://www.blender.org/download/demo-files/#geometry-nodes | Blender Foundation / Blender Studio | 2026-11-16 | Example grooms | Not opened: HTTP 403. Search results disagree (one says CC0, the tutorial page that links to it is CC-BY) | **Unverified.** Read the licence beside the download before use | Actual asset, if verified |
| Blender Studio terms and conditions | https://studio.blender.org/terms-and-conditions/ | Blender Studio | 2026-11-16 | Licence of Studio content | Opened. "Unless notified otherwise, all digital content (webpages, video, artwork, 3D data)" is CC BY 4.0. So a Studio character file with hair is class 2 with attribution unless its page says otherwise | Rule, not an asset | Data |
| Blender 4.2 release notes: USD and Alembic hair curves | https://developer.blender.org/docs/release_notes/4.2/pipeline_assets_io/ | Blender Foundation | 2026-11-14 | Strand export | Already in notes/hair.md §8 | Class 4 | Data |

### 2.3 Artist breakdowns and production talks

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Techniques for Game Character Hair Creation | https://80.lv/articles/techniques-for-game-character-hair-creation | C. Verhoolen, interview on 80 Level, 11 Jan 2018 (also on the CGMA blog) | 2026-11-16 | Card workflow, long hair | Opened. Plan the atlas first: 2:1 for long hair, density falling across it. Groom strands over a ground plane, bake height, alpha, gradients and an ID map, cut the plane into cards. Layers: densest base first so no skull shows, second and third for depth with gaps, then flyaways for the silhouette. About 40k polygons, "quite heavy for hair". Flyaway layer is the first thing dropped at LOD1. AO baked into vertices | Class 4. Article and images, all rights reserved | Visual guidance, data |
| Designing a Real-Time Traditional Chinese Hairstyle in Blender | https://80.lv/articles/designing-a-real-time-traditional-chinese-hairstyle-in-blender | N. Jerabek, interview on 80 Level, 19 Sep 2025 | 2026-11-16 | Layers, hairline, bun | Opened. Six layers in order: hairline, base shape, volume and breakup, sparse strands, flyaways, baby hair. Four texture densities: opaque dense, semi-sparse, sparse, near-transparent single strand. The bun and tied sections use dense opaque cards side by side with shorter textures. 46,221 triangles. Dense cards at the hairline and the part hide the hard scalp edge | Class 4. Uses commercial tools (a card add-on, a texture tool) | Visual guidance, data |
| Curly Hair character breakdown | https://gamesartist.co.uk/curly-hair-character-breakdown-thomas-sykutera/ | T. Sykutera, Games Artist, 17 Jun 2021 | 2026-11-16 | Curly hair with cards | Opened. Curls live in the texture and the clump shape, not in twisted cards: around 6 thick and thin clump versions and 5 flyaway versions, a thick clump is 11 cards, cards pinched at the base and bent along hand-shaped curves. Base layer leaves gaps between curls; a breakup layer and hairline hairs follow. One strand texture is short curls for the nape. Over 30 texture iterations | Class 4 | Visual guidance, data |
| Realistic Hairstyle Tutorial | https://gamesartist.co.uk/realistic-hairstyle/ | M. Cecot, Games Artist, 7 Feb 2021 | 2026-11-16 | Card sets, maps | Opened. "In most cases, I used 3 cards to get 1 full hair card set"; flyaways use 2. Maps: 2:1 strand atlas, normal, AO, alpha, greyscale diffuse, a specular noise map blurred along the strand; AO in vertex colours, hand-adjusted | Class 4 | Visual guidance, data |
| Tips on Normal Map Baking for Hair Cards | https://80.lv/articles/tips-on-normal-map-baking-for-hair-cards | 80 Level (work by V. Sorici), 11 Sep 2018 | 2026-11-16 | Triangle count | Opened. A real-time style at about 19k triangles. The baking tips themselves are on the artist's portfolio page and were not opened | Class 4 | Data |
| Strand-based hair in a 2024 action-adventure game: the card workflow slide | https://www.advances.realtimerendering.com/s2025/content/Strand%20Hair%20in%20IJGC%20-%20Final%20Slides%20(Post-Conference).pdf | S. Kulikov, SIGGRAPH 2025 Advances | 2026-11-14 | Why cards cost artist time | Already in notes/hair.md §4 and §6: atlas first from strands, then place and sculpt cards, then fix normals | Class 4 | Data |
| Strands2Cards; automatic card extraction | https://github.com/kenji-tojo/strns2cards | Tojo et al., SIGGRAPH Asia 2025 | 2026-11-14 | Automatic cards | Already in notes/hair.md §6. No licence file, needs CUDA: the method can be read, the code cannot be reused | Class 4 | Data |
| Fast Real-Time Shading for Polygonal Hair | https://arxiv.org/abs/2302.06468 | Gerard, 2023 | 2026-11-14 | Curly card budgets, card shading | Already in notes/hair.md §6. The paper's timing table has two curly test models: "Curly hair (9K tri)" and "Long curly hair (34K tri)" (read in the PDF in verification) | Class 4 (paper) | Data |

### 2.4 Curly, coily and protective styles

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Open Source Afro Hair Library: licence | https://afrohairlibrary.org/license/ | Open Source Afro Hair Library | 2026-11-16 | Coily styles, locs, braids | Opened. Free models of Black hair styles. Licence is the project's own "Black Open Source Software (BOSS) License v1.0", not Creative Commons. Use in personal and professional projects is allowed. "You cannot directly distribute OSAHL 3D Models to the public"; modified models cannot be distributed either; no AI training use; some fields of use are barred | **Class 4 for this engine.** An engine library hands the model files to every user, which the licence forbids. A game made with the engine could use them directly from the source | Visual guidance |
| Code My Crown guide | https://www.unilever.com/files/code-my-crown-dove-unilever-guide.pdf | Dove with the Open Source Afro Hair Library, Nov 2023 | 2026-11-16 | Textured and protective styles, step by step | Not opened: HTTP 403. Search summaries: a free guide of about 230 pages with 15 hair sculpts and step-by-step instructions. notes/hair.md §10 covers the same material through the SIGGRAPH 2024 course | Unverified. Treat as class 4 | Visual guidance |
| More Than Killmonger Locs: A Style Guide for Black Hair | https://par.nsf.gov/servlets/purl/10637184 | Darke, Olander, Kim, SIGGRAPH 2024 Courses | 2026-11-14 | Hair type system, locs, cornrows, twists | Already in notes/hair.md §10. Cornrow partings belong in the scalp normal map; braids and locs are sculpted geometry with fibre detail in maps | Class 4 (course notes) | Visual guidance, data |

### 2.5 Assets and datasets checked for shipping

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| MakeHuman asset packs (index) | https://static.makehumancommunity.org/assets/assetpacks.html | MakeHuman Community | 2026-11-16 | Licence per pack | Opened. Packs are listed under headings by licence: "Mesh assets shared under CC0", "Mesh assets shared under CC-BY", and for functional packs "The assets are shared under CC0" | Rule, not an asset | Data |
| Hair 01 | https://static.makehumancommunity.org/assets/assetpacks/hair01.html | Ten community authors | 2026-11-16 | Scalp hair meshes, braids, bobs, buns | Opened. 25 styles, each row of the table marked CC0. "Mostly low-poly and stylized". 217 MB. Includes three braid styles, a braided bun and a plain bun | **Class 1.** CC0, stated per asset on the pack page | Actual asset (fitted to another body; see section 3) |
| Hair 02 | https://static.makehumancommunity.org/assets/assetpacks/hair02.html | One community author (pack page names them) | 2026-11-16 | Scalp hair meshes, an afro, braided rows, updos | Opened. 21 styles, each marked CC-BY. "Mostly high-poly". 68 MB | **Class 2**, CC-BY with attribution to the author named on the pack page. The page does not give the CC-BY version, and the packs FAQ links "CC-BY" to a CC BY 2.5 deed, so the version is open until it is read in the files | Actual asset, with attribution, once the version is read |
| Hair 03 | https://static.makehumancommunity.org/assets/assetpacks/hair03.html | Three community authors | 2026-11-16 | Scalp hair meshes | Opened in verification. 14 styles, each row marked CC-BY. "A set of alpha 7 hair adaptations and misc hair": four are named as adaptations of older MakeHuman hair, so their origin is a second thing to check. 38 MB | **Class 2**, CC-BY per asset row; version not stated (see Hair 02) | Actual asset, with attribution, once the version is read |
| Bodyparts 05: beards and moustaches | https://static.makehumancommunity.org/assets/assetpacks/bodyparts05.html | Four community authors | 2026-11-16 | Facial hair meshes | Opened. "A set of beards and moustaches shared under CC0." Five assets, each marked CC0 | **Class 1** | Actual asset |
| Bodyparts 06: beards and moustaches | https://static.makehumancommunity.org/assets/assetpacks/bodyparts06.html | Three community authors | 2026-11-16 | Facial hair meshes | Opened in verification. "A set of beards and moustaches shared under CC-BY." Four assets (two moustaches, two beards), each row marked CC-BY | **Class 2**, CC-BY per asset row; version not stated (see Hair 02) | Actual asset, with attribution, once the version is read |
| Eyebrows 01, Eyelashes 01 | https://static.makehumancommunity.org/assets/assetpacks/eyebrows01.html and https://static.makehumancommunity.org/assets/assetpacks/eyelashes01.html | One community author | 2026-11-16 | Brow and lash cards | Both pack pages opened in verification. 14 eyebrow sets and 5 eyelash sets, every row marked CC0. Described as "high-res" (the eyelash page repeats the eyebrow description) | **Class 1.** CC0, stated per asset on both pack pages | Actual asset |
| Hair editor pack (`hair.blend`, `fur.blend`) | https://static.makehumancommunity.org/assets/assetpacks/haireditor.html | One community author (pack page names them) | 2026-11-16 | Geometry-nodes hair templates | Opened. "Geometry nodes hair and fur, necessary for the hair editor." 18 MB. The pack page has no licence line; the index lists it under functional packs "shared under CC0". The site's licence page speaks only of "core assets" (CC0) and of the add-on's code (GPL) | Class 1 by the index page's section statement only. Confirm inside the archive before use | Actual asset (build input), after confirming |
| MPFB 2.0.11 release notes | https://static.makehumancommunity.org/mpfb/releases/release_2011.html | MakeHuman Community, 22 Sep 2025 | 2026-11-16 | Geometry-nodes hair in a character tool | Opened. "Experimental support for adding and working with geometry hair"; starter systems with noise, curl and colour, then Blender's hair sculpt brushes; "still in an early stage" | Class 4 (release notes; the add-on's licence was not read here) | Data |
| Asset packs FAQ | https://static.makehumancommunity.org/assets/assetpacks/faq.html | MakeHuman Community | 2026-11-16 | What CC0 and CC-BY mean for the packs | Opened. CC0: donated to the public domain. CC-BY: free use with credit to the author. Assets are mostly by third-party authors. Its "CC-BY" link goes to a CC BY 2.5 (Sweden) deed, and no page gives a version per asset | Rule, not an asset | Data |
| Hair Alphas For Days | https://opengameart.org/content/hair-alphas-for-days | A community author on OpenGameArt | 2026-11-16 | Strand clump alphas | Not opened: HTTP 403, to the verifier too. Search summary: 85 strand clump images, 2048 x 2048, straight, wavy and curly, listed CC0. Each image also has its own page, so the licence has to be read per image | **Unverified.** Class 1 only if the pages confirm CC0. Treat as class 4 until then | Actual asset, if verified |
| Poly Haven | https://polyhaven.com/faq | Poly Haven | 2026-11-16 | Hair textures | Search found no hair, fur-card or strand-alpha asset. Everything on the site is CC0, but there is nothing to use for hair. Verification read the site's public asset list (2,389 assets): the only matches are three fur fabric materials, which are cloth textures, not strand alphas | Nothing found | None |
| ambientCG search for "hair" | https://ambientcg.com/list?q=hair | ambientCG | 2026-11-16 | Hair textures | Opened. The search returns no assets (the site's search API also reports 0 results for "hair") | Nothing found | None |
| HAIR model files | https://cemyuksel.com/research/hairmodels/ | C. Yuksel | 2026-11-16 | Strand models (eight, 10k to 50k strands, straight, wavy, curly) | Opened. "You may freely download and use the hair model files provided above for your personal or research projects", with a link back when results are published | **Class 4.** Personal or research use; no grant to redistribute | Data (testing a converter locally only) |
| Hair20K | https://zhouyisjtu.github.io/project_hair/hair20k.html | Zhou et al. | 2026-11-16 | Strand dataset, 21,054 styles built from the USC-HairSalon models | Opened. "The license granted is for internal, non-commercial research, evaluation or testing purposes only." No redistribution | **Class 4** | None |
| CT2Hair | https://github.com/facebookresearch/CT2Hair | Meta research | 2026-11-16 | Strand models from CT scans of wigs, 10 styles | Opened. README: "Creative Commons Attribution-NonCommercial (CC BY-NC) 4.0" | **Class 4.** Non-commercial | None |
| USC-HairSalon | http://www-scf.usc.edu/~liwenhu/SHM/database.html (the address papers cite; the host no longer resolves) | Hu et al., 2015 | 2026-11-16 | 343 strand models | Licence not found. The dataset derived from it is non-commercial | Unverified. Treat as class 4 | None |
| DiffLocks dataset | https://github.com/Meshcapade/difflocks | Rosu et al., 2025 | 2026-11-16 | 40k synthetic strand styles made in Blender | Opened. The LICENSE file and the project's licence page were read in verification: "DiffLocks License (Non-Commercial Scientific Research Use Only)", covering model, data and software; no commercial use, no sharing or redistribution | **Class 4.** Non-commercial research only | None |

### 2.6 Made in this repository

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Brow and lash cards with drawn alpha textures | `assets/characters/realistic/lashes.py` | This project | 2026-11-16 | Facial hair as cards | A working, scripted route: strand images drawn by code (`_strands`, `:54`), a card bound to the skin so identity shapes move it (`_brow_card`, `:340`), the `hair` shading model | Class 5, CC0-1.0 as the project's built-in items declare | Actual asset; the pattern to extend to moustaches and beards |
| Sculpted hair masses | `assets/characters/humanoid/hair.py` | This project | 2026-11-16 | Mesh tier | Scalp cap plus clumps, remeshed. The audit calls the result lumpy and helmet-like (audit.md defect 4). Its `Head` class (hairline table, scalp vertex set, comb functions) is reusable as the guide generator | Class 5 | Actual asset (mesh tier and crowd LOD only) |

## 3. What can ship, strictly

Only these could ship in the engine or feed its build today.

| Asset | Class | What it is good for | Limits |
|---|---|---|---|
| Blender's bundled hair node groups | 1 (CC0, read in the shipped LICENSE file and in each asset's metadata) | The strand source for every built-in style: generate, interpolate with a part, trim, curl, braid, clump, frizz. Runs headless in the existing Blender build scripts | A build input. The strands, cards and atlases the build makes from them are class 5 |
| MakeHuman Hair 01, Bodyparts 05 | 1 (CC0, per asset on the pack pages) | Test content for the import path; CC0 starting topology for braids and a bun; a few beards | Fitted to the MakeHuman body, not this one. "Mostly low-poly and stylized": below the semi-realistic target. Record author and pack in each item's `source` |
| MakeHuman Hair 02, Hair 03, Bodyparts 06 | 2 (CC-BY, per asset on the pack pages) | More styles, including an afro, braided rows and updos; two more beards and two moustaches | Attribution text must ship with the engine and travel with any project that copies the item. The CC-BY version is not stated on any pack page and the FAQ links a 2.5 deed: do not ship one until the version is read in its files. Four Hair 03 styles are adaptations of older MakeHuman hair |
| MakeHuman Eyebrows 01, Eyelashes 01 | 1 (CC0, per asset on the pack pages) | Brow and lash cards to compare layouts with, or as fixtures | Fitted to the MakeHuman head |
| MakeHuman Hair editor pack | 1 by the index page's section statement only | Geometry-nodes hair templates | The pack page is silent on licence. Confirm inside the archive before use |
| Everything this project generates | 5 | The built-in library | None |

**Not usable as files:** every strand dataset found (personal, research or non-commercial terms); the Open Source Afro Hair Library (its licence forbids redistribution); anything from a commercial game, a commercial character tool, a marketplace or a portfolio site. No CC0 or CC-BY strand dataset was found. No CC0 hair-card atlas was found on Poly Haven or ambientCG.

**How to check a MakeHuman community asset.** Licences vary per asset, not per site. (1) Find the asset's pack on the asset packs index and read which licence heading it sits under. (2) Open the pack page and read the licence column of the asset's own row. (3) After download, read the licence line in the asset's own files and confirm it matches. (4) For CC-BY, copy the author's name exactly as the pack page gives it into the item manifest's `source.author` and add the credit to the engine's notices. (5) If any step disagrees or is missing, the asset is unverified and does not ship. Assets outside the packs, on the community's user-contribution pages, were not checked here; the same per-asset rule applies.

## 4. The style groups of brief §7.1 against a production approach

**The common recipe** (architecture §11, CO11, not a new design): Blender hair curves on the realistic scalp, built with the bundled CC0 nodes and driven by the existing comb and hairline code; clusters of strands become cards; strands are baked into one atlas per style family; cards are layered; LODs drop layers; a solid mesh is the far fallback. One style is one groom item with `cards` and `mesh`, and later `strands` from the same curves.

**Layers** (from the Unreal tutorial and the three artist breakdowns above):

| Layer | Job | Share of triangles |
|---|---|---|
| Scalp underlay | A painted or baked hair-coloured layer on the scalp skin so gaps never show skin. Zero triangles when it is a skin layer; today's `cap` (`hair.py:128`) is the mesh form | 0 to 2k |
| Coverage | Wide, opaque cards with repeated textures. Stops the skull and the far side showing | about 5 to 10% |
| Mid | Sets the overall shape | about 15 to 25% |
| Top | The visible strand detail | about 40 to 50% |
| Hairline and part | Short, dense, soft cards along the hairline and the part; baby hairs | about 5 to 10% |
| Flyaways | Single-strand cards for the silhouette. First layer dropped at LOD1 | about 10 to 15% |

The shares are this file's proposal, not a published rule. The one published split is the Unreal tutorial's example: coverage 2,000, mid 7,500, top 25,000, flyaways 10,000 and short hairs 10,000 triangles, which is about 4%, 14%, 46%, 18% and 18% of 54,500.

**Atlas content** to bake per card, from the references: coverage (alpha), depth within the clump, root-to-tip coordinate, a per-strand seed or ID, strand direction (flow), and AO. Which of these the Hair shading model reads, and how they are packed, is CO7's decision with Agent 6.

Triangle counts below are proposals inside the plan's budgets (hero up to 40k, gameplay up to 25k, crowd 3k; `plans/axcenity-engine-character-studio.md:193`). They are Expected, not Measured. Published data points: 19k, about 40k ("quite heavy") and 46k for single artist-made styles; under 50k for an updo with a bun; 30,000 card vertices at LOD0 in the MetaHuman guide table.

| Group (brief §7.1) | Production approach | Cards LOD0 (gameplay) | Hero | Crowd | Notes |
|---|---|---|---|---|---|
| Bald and shaved | **Texture layer** on the scalp skin: a stubble shadow with the hairline's shape, plus a micro normal. No mesh. A buzz cut adds a thin shell or very short cards | 0 (bald, shaved); 2k to 5k (buzz) | same | 0 | Belongs with the skin layers of brief §6. The hairline shapes of section 5 are shared with it |
| Short | **Cards from generated strands**, with the scalp underlay. Mostly coverage and top layers; few flyaways | 8k to 15k | up to 20k | 2k mesh | Short hairs are many small cards: shape comes from the texture more than from geometry |
| Medium | **Cards from generated strands**, all layers | 15k to 22k | up to 30k | 3k mesh | The bob-length styles. Ends need their own tip textures |
| Long | **Cards from generated strands**, all layers, on spring bones | 20k to 25k | up to 40k | 3k mesh | Must be pushed clear of the shoulders and back at rest. Declares fewer shape parameters than short styles (section 5) |
| Straight | Not a separate build: the straight strand texture family, fewest segments along each card | lower end of its length's range | | | Highlights show every card edge on straight hair: needs smooth shared normals |
| Wavy | Waves in the card shape (more segments along) and in the strand texture | length's range plus about 20% | | | Wave in geometry only reads as ribbon; it must be in both |
| Curly | **A separate style family, never a slider.** Curl in the strand texture and in clump shapes; base layer leaves gaps between curls; a breakup layer on top | 18k to 25k | up to 40k | 3k mesh | Coily and afro-textured hair: a volume mesh core with a noisy alpha shell of short curl cards. Two published curly card test models are 9k and 34k triangles. Review these on dark and on light hair: card overlap shows more on light hair |
| Braided | **Mesh plus cards.** Each braid, loc or twist is a solid tube mesh with a braid normal and height map; cornrow rows are low mesh ridges and the partings are in the scalp normal map; cards only for the hairline, loose ends and flyaways | braids 6k to 14k as mesh, plus 4k to 8k cards | up to 35k | 3k mesh | Braids are generated from the Braid Hair Curves node and converted to tubes, or modelled. Free-hanging braids ride a spring bone chain each |
| Ponytails | **Cards plus a mesh core.** Scalp section pulled tight to the tie: coverage and top cards following the head. Tail: a low mesh core for volume under cards, on one spring bone chain | 12k to 20k | up to 30k | 2.5k mesh | The tie is a small separate mesh and a material slot. The tail's length and the tie's height are honest parameters here |
| Buns | **Mesh plus cards.** The bun is a mesh core with dense opaque cards wrapped side by side, short textures; scalp section as for ponytails | 12k to 20k | up to 30k | 2.5k mesh | No simulation needed. Loose strands and baby hairs make it read as hair instead of a ball |

## 5. The capabilities of brief §7.2 against what a card style can honestly support

Each style declares only the rows it supports (brief §7.2, §14.1). The item manifest has no field for this today (`crates/avatar/src/item.rs:19-58`); the table is what such a declaration has to be able to say. The shape of the field is the lead's decision. The ranges are this file's estimates, to be proven per style in CO11, not measurements. No reference gives the ±20% length or ±15 mm volume figures. The roughness range is the contract's own (`crates/avatar/src/material.rs:264-270`).

| Parameter | Honest mechanism on cards | Range that holds up | What it costs | Which styles should declare it |
|---|---|---|---|---|
| Length | (a) **Discrete lengths as variants** built from the same strands with Trim Hair Curves: the safe default. (b) A **shape key** between two built lengths, for free-hanging hair: cards shorten along their flow, textures compress. (c) **Clipping along the card's V** against a root-to-tip coordinate in the shader, with the tip fade moved to the cut | (a) any; (b) about ±20%; (c) only shortening, and only where every card runs root to tip | (b) one shape on every card vertex, and the collision at rest must hold at both ends; (c) a new material value and shader support that do not exist yet | Long and medium free-hanging styles; ponytail tails. Not buns, braids, short crops |
| Volume | A **shape key** that moves cards outward from the head along the scalp normal, weighted by layer (top most, coverage least), with the scalp underlay unchanged | About ±15 mm at the crown before gaps open or the hairline lifts | One shape. Must be checked against hats | Short, medium, curly. Not sleek tied styles |
| Parting | **Variants**: left, centre, right, none. Built from the same guide set with a different part line (the Interpolate Hair Curves "Part by Mesh Islands" input, or the comb function's `part_x`). A symmetric style gets left and right by mirroring | Discrete | One extra card set per part; the atlas is shared | Styles whose top is combed from a part. Not buzz, afro, slicked-back |
| Hairline | Two pieces. The **scalp underlay mask** on the skin (straight, rounded, widow's peak, high or receding), a property of the head shared by all styles. And the style's **hairline card set**, which follows the mask by a shape key or by per-hairline variants | Three or four named hairlines | The first is cheap and belongs to the base model. The second multiplies card sets, so most styles should support one or two | Styles that expose the forehead. Fringe styles hide it and should not declare it |
| Fringe / bangs | A **separate sub-item**: its own object in the cards model, its own layer set, switched on or off or swapped (none, side-swept, full, curtain). Not a deformation of the main style | Discrete | Needs sub-items or a second hair slot; a slot can already be `multiple` (`crates/avatar/src/manifest.rs:278`) | Medium and long styles built to take one |
| Curl intensity | **Separate styles.** Straight, wavy, curly and coily are different strand textures and different clump shapes. At most a small "relaxed to defined" blend between two near neighbours by a shape key. A true curl slider is a strand-tier feature (architecture §11: curl at interpolation time, CO14) | None as a slider | n/a | No card style. Shown in the browser as a filter and as "related styles" |
| Strand density | (a) **Layer switches**: flyaways and breakup layers on or off (two or three steps). (b) An **alpha bias** on coverage that thins or thickens every card | (a) discrete; (b) a narrow band before cards turn to ribbons or dissolve | (b) one new material value | Most card styles can declare (a). (b) only where the scalp underlay is good |
| Hair colour | **Material values**, already in the contract: `tint`, `melanin`, `redness`, `dye`, `dye_amount` | Full | None | Every style |
| Highlight colour | A second colour applied through a **mask**: per-strand seed or ID from the atlas, or a second material slot (today's `HairSecondary`). Today the contract has `highlight` as a strength only | Full, where the atlas carries the seed | A highlight colour value (or reuse of melanin, redness and dye for it, as MetaHuman does); several mask patterns if "variation" is wanted | Styles whose atlas has the seed channel. Not the mesh tier |
| Root colour | A gradient over the **root-to-tip coordinate**. Today's `tip_lightening` covers lighter tips only; darker or grown-out roots and ombre need a root colour and a shift | Full, where cards run root to tip | Root colour, shift and contrast values | Styles whose cards all carry the coordinate. Braids and buns need it baked along the braid, not along each card |
| Roughness | **Material value** `roughness_scale`, already in the contract | 0.5 to 1.5 | None | Every style |

Three rules follow. A style built as mesh plus cards declares less than one built from cards alone. A parameter that needs a channel (root-to-tip, seed) is only declared when the style's atlas has it. A control that would do nothing is not shown (brief §14.1).

## 6. Facial hair of brief §7.3

Facial hair is a groom in a face slot, bound to the face (architecture §11). Neither base has that slot yet. Brows and lashes already show the route (`assets/characters/realistic/lashes.py`).

| Type | Approach | Budget (LOD0) | Variation | Notes |
|---|---|---|---|---|
| Stubble | **Skin texture layer**: darkening, a micro normal that is raised, not indented, a roughness change, and a region mask. A heavy stubble may add very short cards at hero distance only | 0 triangles (layer); up to 2k (hero cards) | Density, length (as layer strength and scale), region masks: full, goatee area, moustache area, neck line | Part of the skin layer stack (brief §6). The reference product keeps up to three such layers and a separate shaving mask |
| Moustaches | **Cards** from generated strands, on the upper lip, bound to the skin so the mouth's expressions move them | 1k to 3k | Shapes as separate items (pencil, natural, handlebar, walrus); length as two or three variants | Must not cross the lip line in visemes: check on every mouth pose |
| Beards | **Cards in layers** as for scalp hair: coverage against the skin, top detail, a soft edge layer at the cheek line and the neck line. Long beards add a mesh core and one spring bone chain | Short boxed 3k to 6k; full 8k to 15k (the MetaHuman guide gives 15,000 card vertices for all facial hair at LOD0) | **Lengths** are separate builds from the same strands (trim): short, medium, long. **Shapes** come from region sub-items (cheeks, chin, moustache, neck, sideburn) switched on and off, so goatee, chin strap, boxed and full share cards and one atlas | The stubble layer sits under every beard so the edge fades into skin instead of ending at a card |
| Sideburns | Short: the **skin layer** with a sideburn mask. Long: a small **card set**. Either way it must meet the scalp style at the temple | 0 to 1.5k | Length: none, short, medium, long | Each scalp style declares whether it covers the sideburn region, so the two never double up or leave a gap |
| Beard lengths | Discrete variants; a shape key between two neighbours is possible for a fine slider | n/a | Three or four steps | Same rule as scalp length: no clipping in the shader until the renderer supports it |
| Beard shapes | Region sub-items plus the cheek-line and neck-line masks | n/a | Combinations of regions | Needs the same sub-item support as the fringe |
| Colour matching | Facial hair uses the same `hair` material values. By default the studio copies the scalp hair's values to it ("match hair"), and the user may unlink it to set a redder or greyer beard. Brows follow the same rule | n/a | Linked or free | Stored as ordinary material values in the description; the link is studio behaviour, not a runtime feature |

**Binding.** Facial hair must follow identity shapes (jaw width, chin length) and expressions (jaw open, smile). Use the item manifest's `binding` fit when CO9 lands, or carried shapes as the brows do today. A beard that is rigid on the head bone fails on the first jaw-open pose.

## 7. What to look for in review

The plan's acceptance for CO11: "styles reviewed in renders and in motion (idle, walk); no slab shadows; no clipping into the head at rest". [visual-qa.md](../../visual-qa.md) adds scalp contact, clipping and silhouette for every groom on each frame.

| Check | How to look | Fails when |
|---|---|---|
| No slab shadows | Key light from the side and from above; look at the shadow the hair casts on the forehead, neck and shoulders | The shadow has straight card edges or is a solid block. Today it would in shadow maps: the raster shadow pass ignores alpha (`crates/render/src/renderer/gpu.rs:625`). Ray-traced sun shadows already follow cut-out holes (`crates/render/src/renderer/ray_cut_outs.rs:1-22`), so check with ray tracing off and on, and under a spot light |
| No scalp show-through | Top-down and three-quarter-back views; the part line; behind the ears; the nape. Repeat on the lightest and darkest skin with the lightest and darkest hair | Skin is visible between cards anywhere but the part, or the part is a bare stripe |
| Hairline | Face close-up, front and side | A hard edge where cards start, or cards floating off the forehead. Baby hairs and the scalp underlay should blend it |
| Silhouette | Each style against a plain bright and a plain dark background, eight turntable angles | The outline is a smooth blob (no flyaways) or shows card ends as rectangles |
| No flat cards | Rotate under a single hard light | A card flashes as one plane; highlights break at card edges instead of running across the head |
| Highlight | Move the light around the head | No band, or a band that follows card orientation instead of hair flow |
| Alpha stability | Still camera, then a slow orbit, at 1080p and 1440p | Shimmer, crawling or ghost trails on thin strands; sorting pops between layers |
| Depth and layering | Side views of layered styles | Cards cut through each other in visible lines; the far side of the head shows through the near side |
| Contact at rest | Measure, as the existing QA does for the mesh styles | Hair floating off the scalp (the audit found 18 mm) or inside the head, cheeks, ears or shoulders |
| Follows the head | Every style on both frames, at the extremes of head width and length, ear sliders and body builds | Scalp or ears poke through (the audit found both on the mesh styles) |
| Hats and collars | Each style under a cap and over a high collar | Hair through the hat; long hair through the collar or jacket |
| Motion | Idle, walk, a fast head turn, a nod | Long hair, tails and braids are rigid; or they pass through the neck and shoulders; or they stretch or jitter. Short styles should not move at all |
| LOD changes | Dolly out through every LOD | A visible jump in volume, colour or silhouette between cards LODs or to the mesh |
| Colour range | Black, dark brown, red, blonde, grey, white, one dyed colour; roots and highlights where declared | Blonde and white look grey or flat (multiple scattering missing); dark hair loses all strand detail |
| Curly and coily | The same checks, on dark and on light hair | Curls read as ribbons or as a helmet; no gaps of light inside the volume |
| Facial hair | Every expression and viseme, jaw open | Cards cross the lips, lift off the skin, or the beard's edge is a hard line. The beard does not match the hair colour when linked |
| Only supported controls | Open each style in the studio | A control is shown that does nothing for that style (brief §14.1) |
| Cost | Triangle count and GPU time per style against the plan's budget | Over budget, or overdraw makes close-ups slow |

## 8. Gaps and things not verified

- blender.org and projects.blender.org returned HTTP 403 to this session. The bundled hair nodes' licence was read from an installed Blender instead. The demo files' licence is unverified.
- OpenGameArt returned HTTP 403. The CC0 label of the strand alpha pack is a search summary only.
- The Code My Crown PDF returned HTTP 403.
- MakeHuman Hair 03, Bodyparts 06, Eyebrows 01 and Eyelashes 01 pack pages were opened in verification (section 9). The hair editor pack page has no licence line.
- No MakeHuman page gives the CC-BY version of a CC-BY asset. The FAQ's link goes to a CC BY 2.5 deed.
- The original USC-HairSalon page no longer resolves, so its terms cannot be read.
- The quality of the MakeHuman hair assets was not inspected (nothing was downloaded). Their own description is "mostly low-poly and stylized".
- No published production talk dedicated to card hair (as opposed to strands) was found with slides. The card evidence here is engine documentation and artist breakdowns.
- No primary source was found for beard card layouts from a shipped game.
- Sketchfab was not searched asset by asset. Its licence filter is per model; a model there is usable only when its own page shows CC0 or CC-BY and the uploader is the author.
- Triangle counts in sections 4 and 6 are proposals, not measurements.

## 9. Verification

A second pass on 2026-11-16 checked this file against its sources and against the repository. "Confirmed" means the source's own page, file or line was read again.

### 9.1 Licences of everything classed 1 or 2

| Entry | Result | How |
|---|---|---|
| Blender bundled hair node groups | Confirmed class 1 | `datafiles/assets/LICENSE` in a Blender 5.3.0 install is the CC0 1.0 Universal legal code. Blender run in background mode listed 26 node group assets in `procedural_hair_node_assets.blend`; every one has licence "CC0 - Public Domain" and copyright "Blender Foundation". The input names in the row match. The repository web page still returns HTTP 403 |
| MakeHuman Hair 01 | Confirmed class 1 | Pack page: 25 rows, all CC0. Ten authors, not nine |
| MakeHuman Bodyparts 05 | Confirmed class 1 | Pack page: 5 rows, all CC0, four authors |
| MakeHuman Eyebrows 01, Eyelashes 01 | Raised from "by the index" to confirmed class 1 | Both pack pages opened: 14 and 5 rows, all CC0, one author |
| MakeHuman Hair 02 | Class 2 kept, with a condition | Pack page: 21 rows, all CC-BY, one author. No version anywhere; the FAQ links a CC BY 2.5 deed. Every CC BY version allows redistribution with credit, so the class holds, but the exact version must be read in the files before one ships |
| MakeHuman Hair 03, Bodyparts 06 | Raised from "index only" to class 2, same condition | Pack pages opened: 14 rows and 4 rows, all CC-BY, three authors each |
| MakeHuman Hair editor pack | Class 1 kept as "by the index only" | The index page's "Functional asset packs" section says "The assets are shared under CC0." The pack page and the site licence page say nothing about it |
| Blender Studio blog and training pages | Class 2 kept for text and images | Terms page: all digital content is Creative Commons Attribution 4.0 unless notified otherwise. The training page shows CC-BY; the blog page shows no licence of its own |
| Hair node demo files | Still unverified | blender.org returned HTTP 403 again |
| Hair Alphas For Days | Still unverified | The site returned HTTP 403 again; only search summaries were seen |
| Everything made in this repository | Confirmed class 5 | The built-in items declare `"licence": "CC0-1.0"` (`assets/characters/humanoid/items/hair-bob.item.json:39-42`; `assets/characters/realistic/manifests.py:71`) |

### 9.2 Corrections made

| Where | Was | Now |
|---|---|---|
| Section 1, shadows | Cut-outs cast solid shadows | True for shadow maps only. Ray-traced sun shadows already follow cut-out holes (`crates/render/src/renderer/ray_cut_outs.rs:1-22`; `docs/rendering/ray-tracing.md:22`, `:61-63`). Section 7's slab-shadow check says the same |
| 2.1, Controlling MetaHuman LODs | Search summary: strands at LOD 0, cards at all other LODs | Opened. The page does not say that. It gives 8 LODs above the neck, 4 below, and a way to force cards |
| 2.1, Character Creator hair wiki | AO and depth listed as baked maps of the set | AO is a legacy channel and depth is not used by the shader |
| 2.1, SkinGen body hair | Normal "never inverted"; a shaving mask per layer | An invert switch exists and the manual advises against it; one shaving mask is shared by the three layers |
| 2.2, Blender Studio rows | "Page shows CC-BY 4.0" | The page shows CC-BY; the version comes from the terms page. The licence does not extend to the linked demo files |
| 2.3, polygonal hair paper | Curly styles of 9k to 34k triangles | Two test models, 9K and 34K triangles, read in the paper's table |
| 2.5, Hair 01 | Nine authors; four braid styles and a bun | Ten authors; three braid styles, a braided bun and a plain bun |
| 2.5, Hair 03, Bodyparts 06, Eyebrows 01, Eyelashes 01 | Index only | Pack pages opened; counts, authors and per-asset licences filled in |
| 2.5, Hair 02 and FAQ | Version "not stated" | Also: the FAQ's CC-BY link is a 2.5 deed |
| 2.5, DiffLocks | Unverified | Licence read: non-commercial scientific research only, no redistribution. Class 4 |
| 2.5, USC-HairSalon | No URL | The cited address added; its host no longer resolves |
| 2.5, Poly Haven, ambientCG | Search only | Checked against both sites' public catalogue data: no hair assets |
| Section 3 | Eyebrows, Eyelashes and the Hair editor in one row "by the index" | Split: the first two are confirmed, the editor pack is not |
| Section 4, layer shares | Given without a source | Marked as a proposal; the one published split added beside it |
| Section 5, ranges | "Estimates from the references" | No reference gives the ±20% and ±15 mm figures; said so |

### 9.3 Numbers confirmed at the source

| Number | Source | Result |
|---|---|---|
| 30,000 / 15,000 / 10,000 / 3,000 / 1,500 card vertices, LOD 0 to 4; 500 / 250 / 100 mesh vertices, LOD 5 to 7 | MetaHuman LOD specifications page | Confirmed. The unit is vertices, not triangles |
| 15,000 / 7,000 / 3,000 / 1,000 / 500 facial hair card vertices | Same page | Confirmed |
| Coverage 50 / 2,000 / 25; Mid 200 / 7,500 / 60; Top 500 / 25,000 / 60; Flyaways 371 strands / 10,000 / 25; Short hairs 500 / 10,000 / 75 | Unreal hair card tutorial | Confirmed |
| "fewer than 50,000 triangles" for an updo with a bun | MetaHuman Dataflow page | Confirmed |
| About 40k polygons, "quite heavy for hair" | 80 Level, 11 Jan 2018 | Confirmed |
| 46,221 triangles | 80 Level, 19 Sep 2025 | Confirmed |
| About 19k triangles | 80 Level, 11 Sep 2018 | Confirmed |
| About 6 clump and 5 flyaway versions, 11 cards a thick clump, over 30 iterations | Games Artist, 17 Jun 2021 | Confirmed |
| 2048 x 4096, 2 pixel strands, 32 pixel padding, 10 to 20 pixels between islands | Character Creator hair wiki | Confirmed |
| Eight strand models of 10,000 to 50,000 strands | HAIR model files page | Confirmed |
| 21,054 styles from 343 models | Hair20K page | Confirmed |
| Hero 40k, gameplay 25k, crowd 3k triangles | `plans/axcenity-engine-character-studio.md:193` | Confirmed |
| 2,500 to 10,000 triangles for today's styles | `assets/characters/humanoid/hair.py:158-159` | Confirmed |
| Median 18 mm off the scalp | `docs/development/character-studio/audit.md:66` | Confirmed |

The file states no population mean, standard deviation or anthropometric range. Numbers with no source are the proposals: the triangle ranges per style group and per facial hair type, the layer shares, and the length and volume ranges. Each is labelled as a proposal where it appears.

### 9.4 Claims about this repository

Every `path:line` in section 1 was opened and holds. One limit was added (ray-traced shadows, above). Both base models have the same seven slots and none is marked `multiple`.

### 9.5 Hygiene

- All 50 index rows have the eight columns filled.
- Nothing from a commercial game, a commercial character tool, a marketplace or a portfolio site is above class 4. Epic, Autodesk, SideFX and Reallusion documentation rows are class 4 and used for numbers and method only.
- The folder holds this file only. No image or asset was downloaded into the repository.
- No personal details or local paths. Authors named are those of public articles and papers. Quotes are a sentence or less, each beside its source.
