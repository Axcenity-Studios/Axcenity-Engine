# Content sourcing: licensed bases for the first four items (wave 5, decision 11)

Branch `content-sourcing`, from `refinement-wave5`. Task: decision 11's first step. Find licensed base assets (CC0, CC BY, MIT or Apache-2.0 only) for the four items decision 13 names: a hairstyle, a crew-neck T-shirt, joggers and sneakers. Download the most promising ones where no account is needed, evaluate them in Blender beside the realistic body, score them against decision 13 and [asset-quality.md](../../asset-quality.md) sections 3 and 4, rank them, and say what needs Paul. This is research and evaluation. Nothing in the engine changed.

**In one paragraph.** No base reachable without an account comes close to Production, and none is close for hair or sneakers. Two CC0 garments from BlenderKit are worth starting from: the **Plain T-shirt** (Introspective Mnisi, 2,800 triangles, 4K maps) and the **Grey sweat pants** (the same author, 6,143 triangles, elastic cuffs, a drawcord waistband, back pockets, 2K maps). Both have their own fit, rig and level-of-detail work ahead of them. A third CC0 tee, **Men t-shirt Turbulent sun** (Joachim Bornemann), has the better drape but 21,689 triangles and a printed motif to strip. Every MakeHuman asset I tried (16 of them, CC0 and CC BY) is from an older generation: 400 to 4,000 triangle garments, hair made of shells or flat slabs, sneakers either low-poly or copies of famous designs. The best semi-realistic hair cards and the cleanest game-ready garments are on **Sketchfab under CC BY 4.0**, which needs an account to download. Pocolov Studio's series of about 12 card hairstyles (28k to 34k triangles each) is the strongest hair lead, and MadeByYeshe's Basic T-Shirt (7,202 triangles, Marvelous Designer drape) is the strongest tee lead. **Sneakers are the hardest kind.** Most free sneakers, scanned or modelled, are copies of a branded design, such as the Converse Chuck Taylor, Vans Old Skool, Nike or New Balance. A licence on the mesh does not clear the trade mark or the design. **Recommendation:** start the tee and the joggers from the two BlenderKit CC0 bases now. Ask Paul for a Sketchfab account so the hair can come from Pocolov's set. Treat the sneaker as a model built by hand from a CC0 reference, unless an unbranded CC BY sneaker passes review on its own page.

## Files

| File | What |
|---|---|
| `docs/development/character-studio/refinement/wave5/content-sourcing.md` (new) | This report: survey, candidates with licence evidence, scores, ranking, recommendation, what needs Paul |
| `.../wave5/content-sourcing/tops.jpg`, `bottoms.jpg`, `shoes.jpg`, `hair.jpg` (new) | One row per evaluated candidate: as placed (no fit), then masculine front, side, back and three-quarter, then feminine front, side and back (both after a rough push-out, see below), then the item's own topology, front and side |
| `.../wave5/content-sourcing/closeups.jpg` (new) | The two tees and the two bottoms at full render size |

No code, schema, manifest, test or generated file changed, so there is no red run. No dependency was added. **No downloaded file is in the repository.** Downloads are under `Desktop/axcenity-w5-sources/<kind>/<name>/`, outside the repository, with their SHA-256 hashes (below). My scripts are in my scratch folder, not the repository: the evaluator (`evaluate.py`), the sheet builders, the BlenderKit search and download helpers, and the MakeHuman pack extractor.

## How candidates were judged

- **Licence**, read on the source's own page: CC0 1.0, public domain, CC BY 3.0 or 4.0, MIT, Apache-2.0 only. **Refused:** share-alike, non-commercial, no-derivatives, custom "royalty free" licences (BlenderKit's RF), personal or research-only terms, and anything whose licence wasn't readable. Also refused: re-uploads ("ALL CREDITS TO ZEPETO", "Made by CAPCOM"), assets built for a commercial base ("designed for Character Creator 4"), scans of identifiable people, and copies of a branded product. The asset licence may be real while the trade mark or design right is not cleared.
- **Geometry**, measured in Blender (bpy 5.3) on the file as downloaded:
  - triangles;
  - quad share (a glTF is always triangulated, so I also report the share of quads after Blender's Tris to Quads at 40 degrees, on a copy welded at 0.1 mm: "quads after join");
  - edges shared by more than two faces ("non-manifold"), and open edges;
  - UV layers, texture maps and their sizes;
  - rig, and size.
- **Pictures**, Eevee, 32 samples, beside `RealisticParts.glb`, masculine frame and feminine frame (`Frame_Feminine` = 1, the item scaled by 1.694 / 1.75), rest pose, orthographic views:
  - **As placed:** the item moved by its bounding box onto the body (hair to the crown, tops to the collar, bottoms to the waist, shoes to the floor). MakeHuman items are scaled from decimetres. A pair of shoes is split per foot and turned 22 degrees toes-out to match our feet.
  - **Fit (rough push-out):** for display only, each vertex inside the body or closer than a margin (7 mm for clothing, 2 mm for hair) is moved out along the nearest skin normal. Clothing gets one linear subdivision first. This is **not** the asset's own fit and not the engine's fitter. The small tears it leaves at armpits and inner legs are this script's artefacts. The topology views show the item's own surface closed. Shoes are not pushed: the foot showing through a shoe shows the size and shape mismatch.
  - **Topology:** the item alone, flat grey with its wireframe.
- **Scores** against decision 13's row for the kind and asset-quality.md sections 3 (clothing) and 4 (hair), on a five-step scale: **Production-close** (fit, rig and LODs only), **Good base** (needs our construction details too), **Weak base** (keep the silhouette or UVs, rebuild much), **Reference only**, **Refused**. None scored Production-close.

## 1. Sources surveyed

| Source | Licences offered | Account to download? | Usable here? | What I saw |
|---|---|---|---|---|
| [MakeHuman community asset packs](https://static.makehumancommunity.org/assets/assetpacks.html) (MakeHuman and MPFB system and community assets) | CC0 or CC BY, per pack and per asset (the pack page's table). CC BY is version 4.0 per the community FAQ | **No** (zip packs from `files2.makehumancommunity.org`; the old per-asset repository pages return 404) | Licence-clean, quality **below the bar** | 16 assets evaluated. Clothes 400 to 4,000 triangles, all quads, made for MakeHuman's base mesh; hair is shells or flat slabs with alpha, one alpha-card curly style. Old-generation look |
| [BlenderKit](https://www.blendkit.com/docs/licenses/) (now Blendkit) | CC0 and "Royalty Free". RF is refused: no resale "even if modified", and it must not be "directly extractable" from a game | **No for "Free" CC0 assets**: the public search API and the download endpoint work anonymously and give a glTF. **Yes (paid plan)** for CC0 assets not marked Free | **Yes**: the best no-account garments | The two best no-account garment bases (tee, sweat pants). Scanned shoes from one uploader ("Free poly") are all branded designs |
| [Sketchfab](https://sketchfab.com), CC0 and CC BY downloads | Per model: CC0, CC BY 4.0, also SA, NC, ND (refused). The API states "Author must be credited. Commercial use is allowed." for CC BY | **Yes**: "Downloading a model requires the user to be authenticated with a Sketchfab account" (download API docs); search and metadata are public | **Yes, behind an account** | The best semi-realistic hair cards (Pocolov series) and game-ready garments (Marvelous Designer tees, low-poly joggers). Many re-uploads of game rips and branded sneakers to filter out |
| [Poly Haven](https://polyhaven.com/license) | CC0 | No | Textures only (already used); no garment, hair or sneaker | 522 models: clothing is only Rubber Boots, Fishermans Hat, Round Spectacles. Fabric scans already feed R7 |
| [ambientCG](https://docs.ambientcg.com/license/) | CC0 | No | Materials only | Fabric, leather, rubber materials (R7 uses Rubber004) |
| [Blender Studio](https://studio.blender.org/characters/) | CC BY, per asset | Some free; some need the subscription | Not for this | Characters are stylized (Rain, Snow, Sprite Fright); no semi-realistic garment or hair found |
| [Quaternius](https://quaternius.com/faq.html) | "All models are under the CC0 License." | No | Stylized family only | Low-poly stylized outfits (stage 0's G-39) |
| [Kenney](https://kenney.nl/support) | CC0 | No | No | No realistic garment |
| [OpenGameArt](https://opengameart.org) | Mixed per item: CC0, CC BY, CC BY-SA, GPL | No | No | 3D results for "hair", "t-shirt", "sneakers" are low-poly stylized ("Upcomb hair style for male model", "T-Shirt and Hanger") |
| Smithsonian Open Access (3d.si.edu) | CC0 | API needs an api.data.gov key; the site refused the fetch in stage 0 (403) | Reference only | Historic footwear scans appear on Sketchfab under CC0 (museum accounts) |
| Museum CC0 scans on Sketchfab (e.g. [Małopolska virtual museums](https://sketchfab.com/3d-models/priest-karol-wojtyas-sports-shoes-678f96cfadc741a3b260548eeb75954a)) | CC0 ("Credit is not mandatory. Commercial use is allowed.") | Yes (Sketchfab) | Reference only | 118,620-face scan of worn leather sports boots: a shape and wear reference, not a sneaker |
| GitHub: [TressFX](https://github.com/GPUOpen-Effects/TressFX) | MIT (repository licence) | No | Not evaluated | `Ratboy_short.tfx`, strands for a stylized character's head. No separate art licence is stated, so the MIT file is the only evidence; TFX format; stylized. Not pursued |
| GitHub: [open-source-avatars](https://github.com/ToxSam/open-source-avatars) | Registry of CC0 VRM avatars (no SPDX on the repo) | No | No | Anime-stylized VRM avatars |
| Research data: [DiffLocks](https://difflocks.is.tue.mpg.de/license.html) (40k hairstyles) | Non-commercial research; commercial via Meshcapade | Registration | **Refused** (NC) | |
| Research data: [CT2Hair](https://github.com/facebookresearch/CT2Hair) | "Creative Commons Attribution-NonCommercial (CC BY-NC) 4.0" | No | **Refused** (NC) | |
| Research data: [Cem Yuksel's hair models](http://www.cemyuksel.com/research/hairmodels/) | "for your personal or research projects" | No | **Refused** (custom, not commercial) | 8 strand models, 10k to 50k strands |
| Research data: [GarmentCodeData](https://igl.ethz.ch/projects/GarmentCodeData/) | **Not stated** on the project page; its ETH record returned HTTP 500 (as in stage 0) | Unknown | Unverified: not used | 115k simulated made-to-measure garments with patterns |
| Other scanned-garment and avatar sets (4D-DRESS, CAPE, THuman, DeepFashion3D, BEDLAM) | Not re-read in this pass. Earlier reading: research-only licences | Registration | Not used | Re-read before any use; expect NC |

## 2. Candidates

Measurements are on the file as downloaded. "Pushed" is how many vertices the display push-out moved (after one subdivision for clothing) and the farthest move: that is how far the item's own fit is from our body. It measures the mismatch, not the item's quality.

### Hairstyle (decision 13: actual hair geometry, natural volume, clean silhouette)

**No acceptable candidate is reachable without an account.** All five MakeHuman hairs evaluated are below §4 (`hair.jpg`):

| Rank | Candidate | Licence | Triangles | What I saw | Verdict |
|---|---|---|---|---|---|
| 1 | MakeHuman `punkduck_alpha7_curly` (hair03 pack), punkduck | CC BY 4.0 | 11,304, 1K atlas | Real alpha cards and many strands, but long and curly (not the short or medium cut asked for), ribbon cards visible from the back, 1,024 px atlas, the scalp shows at the parting | Weak base |
| 2 | `o4saken_chinesebob01` (hair03), punkduck | CC BY 4.0 | 10,108, 2K | Cards, but blocky and patchy; the crown and back read as stacked rectangles | Weak base |
| 3 | `elvs_short_side_do` (hair02), Elvaerwyn | CC BY 4.0 | 98,660, 1K | A smooth shell ("helmet") with a painted texture. The material file is named `elvs_thejournalist1` and the texture `darkmaggiehair.png`, which may come from other work. Check its provenance before any use | Reference only |
| 4 | `elvs_grump_hair` (hair02), Elvaerwyn | CC BY 4.0 | 12,332, 2K | Flat slabs fanned round the head; reads as plates in side view | Refused on quality |
| 5 | `culturalibre_hair_14` (hair03), culturalibre | CC BY 4.0 | 23,489, 1K | Solid shell, black flat texture | Refused on quality |

**Behind a Sketchfab account (CC BY 4.0), the leads.** Ranked from each model's page record and pictures; not downloaded:

| Rank | Model | Faces | Why |
|---|---|---|---|
| 1 | [Pocolov Hair 09](https://sketchfab.com/3d-models/pocolov-hair-09-51535548ce284623b6fb60b1200ce74f), Pocolov Studio ("Published by Pocolov Studio 2019 Artist Giang") | 33,574 | A short layered cut in semi-realistic cards: the shortest style of the series, judged from the pictures. One of about 12 Pocolov styles (Hair 03, 05, 06, 07, 10, 11, 13, 14, 15, 16, 17; 27,874 to 33,286 faces), mostly medium and long, all CC BY |
| 2 | [Female Short Hair Cards](https://sketchfab.com/3d-models/female-short-hairt-cards-2daf1e54b13c4086847be90ded2dfeeb), ebadraza225 | 32,892 ("around 3.5K quads" of hair) | A chin-length bob in real cards, game-ready by its description |
| 3 | [Curly Hair 1 (Beatrice)](https://sketchfab.com/3d-models/curly-hair-1-beatrice-character-hair-free-89ed2c07a0d9418d9d6b231bedf2772a), continuumreed | 39,250 | "Completely handmade, offered free CC-BY… No remixed or recycled models in this one." Long curly, not rigged |
| 4 | [FREE Male Fashion Hair collection 01](https://sketchfab.com/3d-models/free-male-fashion-hair-collection-01-lowpoly-2735370f6f764078a18f5d80651050a2) and 02, VincentFreelance | 694,304 for about 21 styles | Good men's short cuts, but "Textures not included": geometry only, so a new atlas is needed. A promotion for a paid library: check that the page licence covers the geometry |

Refused on Sketchfab: zHairezt's short styles ("ALL CREDITS TO ZEPETO": game rips), rendysix's Hair Packs ("Made by CAPCOM"), MotionStudioArts ("exclusively designed for Character Creator 4"), and Warpnik's "Haircut" (a face scan of a person).

### T-shirt (decision 13: proper garment construction, clean back, deformation-resistant fit; must pass the walk)

| Rank | Candidate | Licence | Triangles | Quads after join | Maps | What I saw (`tops.jpg`, `closeups.jpg`) | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | **BlenderKit [Plain T-shirt](https://www.blendkit.com/asset-gallery-detail/cd6d0347-484a-426c-9c72-9b8c117ef500/)**, Introspective Mnisi | CC0 | 2,800 (within §3's 6k) | 0.896 | 4K colour, normal, roughness | A real crew neck with a band, clean back with no seam or tear, set-in short sleeves, straight hem. 0 non-manifold edges, 128 open edges (neck, sleeves, hem: no turned thickness). 0.61 m tall. Fits a smaller body: the push-out moved it up to 71 mm | **Good base** |
| 2 | BlenderKit [Men t-shirt Turbulent sun](https://www.blendkit.com/asset-gallery-detail/263a0760-fcb4-4413-b461-676d8bef9e9b/), Joachim Bornemann | CC0 | 21,689 (3.6x the budget) | 0.936 | 1K colour, normal, ORM | The best drape of the no-account tees, with a rib collar and sleeve hems, but a large printed sun motif front and back, and only 1K maps. 0.70 m long, a man's regular fit | Good base for shape (retopology and a new texture) |
| 3 | MakeHuman `toigo_basic_tucked_t-shirt` (shirts01), MargaretToigo | CC0 | 2,700 | 1.0 (quads) | 2K colour | Clean quads but cut short to tuck in; flat faceted shading | Weak base |
| 4 | MakeHuman `elvs_crude_t-shirt_male` (shirts01), Elvaerwyn | CC0 | 2,068 | 1.0 | 1K | As named: crude, short, wide neck, no hems | Refused on quality |
| 5 | MakeHuman `drednicolson_short-tail_camo_tee` (shirts03), DredNicolson | CC BY 4.0 | 3,746 | 1.0 | 2K | Cropped, camouflage, faceted | Refused on quality |

Behind a Sketchfab account: [Basic T-Shirt](https://sketchfab.com/3d-models/basic-t-shirt-71bdf5940b5d41b8b46628a615e9b0ed) by MadeByYeshe. It has 7,202 faces, was "made in Marvelous Designer, textured in Substance Painter and remeshed in Blender", is game-ready and CC BY, and is the strongest tee lead. [Standard T-shirt](https://sketchfab.com/3d-models/standard-t-shirt-1815c01895bd4538bc0bd05c3394698d) by Badboy17Aiden has 362,067 faces and is a Marvelous drape: a fold reference only. Refused: u-76's "T-shirt LP" is a henley with raglan sleeves, not a crew neck. Hyperrealitystudio's "T-shirt" is a stylized polo. "Casual White Tee And Beige Joggers" is a scan of a person.

### Joggers (decision 13: clean cuffs, consistent geometry, proper leg deformation)

| Rank | Candidate | Licence | Triangles | Quads after join | Maps | What I saw (`bottoms.jpg`, `closeups.jpg`) | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | **BlenderKit [Grey sweat pants](https://www.blendkit.com/asset-gallery-detail/e46f029b-7877-41fd-a870-60b98fc2a259/)**, Introspective Mnisi | CC0 | 6,143 (at the 6k budget) | 0.804 | 2K colour, normal, roughness; 2 UV layers | Gathered elastic cuffs, an elastic waistband with a drawcord, two back patch pockets, fold detail in the normal map. The UVs are laid out as pattern pieces (front and back legs, pockets, band). 0 non-manifold edges, 183 open edges. Long and baggy: the cuffs pool over the instep. The second UV set made the waistband read as a printed band in my render; check it | **Good base** |
| 2 | BlenderKit [Cargo pants](https://www.blendkit.com/asset-gallery-detail/1aaf05aa-c788-46f4-9143-4c2b735f95a2/), Gabriel Mensah | CC0 | 32,844 (5.5x) | 0.949 | 4K colour, normal, roughness | Cuffed cargo joggers with large side pockets and a camouflage texture. **180 non-manifold edges.** Short for our body: the cuffs sit at mid-calf | Weak base |
| 3 | MakeHuman `cortu_cargo_pants` (pants01), Cortu | CC0 | 392 | 1.0 | 2K | Far too coarse for a realistic leg | Refused on quality |

Behind a Sketchfab account (CC BY), all low-poly game garments:

- [Grey trousers (joggers)](https://sketchfab.com/3d-models/grey-trousers-joggers-4740f4cd5be14976bcc5e7bdcf6eae77), 1799danly: 3,598 faces, technical joggers with cargo pockets.
- [Joggers](https://sketchfab.com/3d-models/joggers-a81888a64bf84a6ebb46cd770050b132), MSinkula: 3,506 faces.
- [Simple Sweatpants Model](https://sketchfab.com/3d-models/simple-sweatpants-model-9c6f80d0d57c439084fb0dc13f268442), earlymonke, 2025: 10,922 faces, "decent topology".
- [Male sweatpants and sweatshirt](https://sketchfab.com/3d-models/male-sweatpants-and-sweatshirt-anorak-f72f208b242a415c84d0087a790faa5f), kurmanin: the sweatpants are 3,852 triangles, rigged to the MetaHuman skeleton. We would re-rig, and nothing of MetaHuman may ship.

Refused: "Dillard, Darren Joggers", because its texture copies the Champion logo.

### Sneakers (decision 13: correct footwear silhouette, clean toe construction, proper foot fit)

**No acceptable candidate is reachable without an account**, and most free sneakers anywhere copy a branded design (`shoes.jpg`; the rows show the foot through the shoe because shoes aren't pushed):

| Rank | Candidate | Licence | Triangles | What I saw | Verdict |
|---|---|---|---|---|---|
| 1 | MakeHuman `punkduck_running_shoes_01` (shoes02), punkduck | CC BY 4.0 | 2,792 a pair, 2K | A plain low runner; low-poly faceted toe; texture with a repeating made-up wordmark; open at the collar (412 open edges). 0.23 m long against our 0.248 m masculine foot | Weak base (silhouette only) |
| 2 | MakeHuman `punkduck_comfortable_sneakers` (shoes02) | CC BY 4.0 | 1,556 | A slip-on, not a lace-up sneaker; very coarse | Refused on quality |
| 3 | MakeHuman `punkduck_tennis_shoes` (shoes02) | CC BY 4.0 | 3,340 | Shoe and sock fused into one mesh; boxy | Refused on quality |
| 4 | MakeHuman `culturalibre_sneakers` (shoes02) | CC BY 4.0 | 9,042, 1K | High-top basketball shoe whose panels follow a famous retro basketball design | Refused: design copy |
| 5 | MakeHuman `elvs_zombie_killer_sneakers` (shoes02) | CC BY 4.0 | 153,938, 42 non-manifold edges | A Chuck Taylor-style high top (ankle patch, toe cap, stripe) | Refused: design copy |
| – | BlenderKit [Scan shoe11](https://www.blendkit.com/asset-gallery-detail/56a82c29-d8fe-454d-b4d6-8eb44be3e38c/), "Free poly" | CC0 label | 100,000, 8K | The most convincing material of any sneaker here (canvas, rubber toe cap, laces), but a closed solid with no opening (0 open edges), 0.459 quads after join. A low canvas sneaker with the side eyelet vents and toe cap of a Chuck Taylor. The uploader's other scans are Converse, New Balance, Adidas and Nike shoes. BlenderKit's own article says it rejects CC0 re-uploads made for profit, so provenance is doubtful | Reference only (shape and wear) |
| – | BlenderKit Scan Shoe 7, "Free poly" | CC0 label | 10,000 | A Converse high top with its star patch | Refused: brand. Not in the sheets |

Behind a Sketchfab account (CC BY), none verified unbranded yet:

- [Black sneakers - scan](https://sketchfab.com/3d-models/black-sneakers-scan-c99b76c916904cd5818b68acb3665bf9), tijerin_art: 13,018 faces, a game-ready low poly from a scan of a worn skate shoe. Check for a logo.
- [Sneakers Low Poly](https://sketchfab.com/3d-models/sneakers-low-poly-b8ade0cd40c449ff8870958c7460cebd), meshatrech: 33,172 faces, a high top with a sock.
- [Clunky White Dad Sneaker Shoe Scan](https://sketchfab.com/3d-models/clunky-white-dad-sneaker-shoe-scan-5630902088d14774aabb7d0bc9a2d672), lzou: 47,281 faces; looks like a designer chunky sneaker.

Refused on brand: abazibiz's "sneakers" (Converse star), meerschaumdigital's "Blue Sneaker" (Vans side stripe), "Nike Air Jordan", "Adidas…", "Salomon XT6", "FILA", ItsKrish7's "Sports Sneaker" (Nike swoosh), yanix's "Brown Sneakers" (a Reebok-style logo).

**My reading:** a clean, unbranded, game-ready sneaker under CC0 or CC BY is rare. Making ours means building a lace-up low top by hand in Blender (decision 11's "manual modelling" row). Generic CC0 references for proportion and wear: Scan shoe11, the museum boots, Poly Haven's Rubber Boots. Materials come from Poly Haven and ambientCG, as R7's did.

### Reference meshes met (CC0 or CC BY)

- Museum footwear scans under CC0 on Sketchfab, such as the worn leather sports shoes in the Małopolska virtual museums' collection (118,620 faces): a wear and stitching reference.
- BlenderKit "Free poly" scans: shape references only, given their provenance.
- No CC0 or CC BY scanned **garment** worn on a body, and no scanned **head** without an identifiable person, was found: the scans of people are of real, identifiable people and are refused.

## 3. Recommendation

1. **Tee:** start from BlenderKit's **Plain T-shirt** (CC0). It is the only no-account tee with a clean crew neck and back inside the triangle budget. Work it needs:
   - refit it to the realistic body on both frames (it was made for a smaller one);
   - turned hems with thickness at the neck, sleeves and hem, which §3 requires;
   - a rig through the body's skinning, the walk fit test, and LODs;
   - the jersey texture from Poly Haven's Cotton Jersey, and colour taken out of the albedo.

   If Paul provides a Sketchfab account, compare it first with MadeByYeshe's Basic T-Shirt (CC BY), which has a Marvelous drape at 7,202 faces.
2. **Joggers:** start from BlenderKit's **Grey sweat pants** (CC0). It has the construction decision 13 asks for: gathered cuffs, a drawcord waistband and pocket panels. Work it needs:
   - shorten and taper the legs so the cuffs sit at the ankle;
   - refit to both frames;
   - edge loops at the knee for deformation;
   - LODs, a rig and the walk test.
3. **Hair:** needs Paul. The no-account options are shells, slabs or long curls. Ask for a Sketchfab account and download Pocolov Hair 09 (short) and two medium Pocolov styles. Check each model's page licence and that the textures are included. Then re-run this evaluation on them. Our groom and card bake (wave 4) would only re-pack the atlas and LODs.
4. **Sneakers:** build by hand from a CC0 reference rather than import. If Paul wants an import, he or the lead should open tijerin_art's "Black sneakers - scan" page (CC BY) and confirm there is no logo, then download it.
5. **Provenance before any file lands:** fill the provenance record of asset-quality.md §2 for each chosen base: source page, author, licence and version, retrieval date (2026-11-18), the hashes below, and the attribution line.
   - For BlenderKit CC0 items, record the asset page's "Creative commons zero" and the [licensing FAQ](https://www.blendkit.com/docs/licenses/licensing-faq/) ("the CC0 license technically allows you to do anything with the assets").
   - BlenderKit's [position on CC0 models](https://www.blendkit.com/articles/our-position-on-cc0-models/) says it rejects re-uploads of others' CC0 work made for profit but has no option yet to exclude an item from redistribution. Check on each author's page that the author made the item; I did not read the author pages.

## 4. What needs Paul

- **A Sketchfab account** (or Paul downloading the listed models himself into a new folder outside the repository) for: Pocolov Hair 09, 03, 05, 16; ebadraza225's Female Short Hair Cards; MadeByYeshe's Basic T-Shirt; earlymonke's and 1799danly's joggers; tijerin_art's Black sneakers - scan. Downloads go through the account's OAuth token, which I don't have.
- **A decision on sneakers:** hand-modelled from references (recommended), or an imported CC BY base after a brand check on its page.
- **Optional:** a BlenderKit account is not needed for the two recommended bases; it would only add CC0 assets marked for the paid plan, such as "Pants and Tshirt Model Rigged" (16,744 faces).

## 5. Downloads (outside the repository, `Desktop/axcenity-w5-sources/`), SHA-256 as downloaded

| Folder | File | SHA-256 |
|---|---|---|
| `top/bk-plain-t-shirt` | glb (BlenderKit `gltf` file of asset `cd6d0347-…`) | `a0bde896788e40219791bc79b16f67a3619d487c7d2ae8ba3fd20df48eaecc5f` |
| `top/bk-men-tshirt-turbulent-sun` | glb (asset `263a0760-…`) | `1d0d79ce41d72942de1c1ccc5f5bdba54bbe30b65bff216210bcc4f1817c3900` |
| `bottom/bk-grey-sweat-pants` | glb (asset `e46f029b-…`) | `04abe82b560270bc7612f0bbc8ae05209917bbf3476949c9dd3588804f1fa235` |
| `bottom/bk-cargo-pants` | glb (asset `1aaf05aa-…`) | `6341fe63a09b1b9af53f5c99b51427f794a847b358147b6a71b74602cf64adbb` |
| `shoes/bk-scan-shoe11` | glb (asset `56a82c29-…`) | `55f8df5c03e7a2cda5b0478eeac1ff5ae3456ec5ca7feb741c9e2ff1ce354c72` |
| `shoes/bk-scan-shoe7` | glb | `abfc724b6d147a028260e796ff1d8c54fad39441d53d84b82793c1dec65cdfb7` |
| `_zips/makehuman` | `shirts01_cc0.zip` | `a5a723b0e84a109bb190fcfeac7f1de4138d875da3e30fe5b3340eac9f38bcd3` |
| | `pants01_cc0.zip` | `e4e0ec60db34f279be291a83cfd7b342a7c5cf09bb7676682a5f39f4f6ac4ad9` |
| | `shirts03_ccby.zip` | `b3ff3596723dbc87cb07389895da760b667cf413f2c67869ccb0cdfb82a79a8a` |
| | `hair02_ccby.zip` | `c681e5efd37df4007a52253a8d071aedbfe3b614f199d8dae4ae76d5bd7d95c9` |
| | `hair03_ccby.zip` | `776c277cb6bcebc32907e05fa94d119e48742cdc1c34183996fc3e65e17a2740` |
| | `shoes02_ccby.zip` | `1b544d87dd8b3d3a9c8317e4f059be456491be979cbc9984fc53f403046f5061` |

Each MakeHuman item extracted into its own folder (`<kind>/mh-<name>/`) has a `<folder>.sha256` beside it, listing every file's hash. No script that came with a download was run. No `.blend` was opened: BlenderKit's glTF files and MakeHuman's OBJ and material text were read as data.

## 6. Attribution of the pictures in `content-sourcing/`

The sheets show renders of these assets. They are ours, but they depict CC BY works, so the credits are:

- "punkduck_alpha7_curly", "o4saken_chinesebob01", "punkduck_running_shoes_01", "punkduck_tennis_shoes" and "punkduck_comfortable_sneakers" by punkduck;
- "elvs_short_side_do", "elvs_grump_hair" and "elvs_zombie_killer_sneakers" by Elvaerwyn;
- "culturalibre_hair_14" and "culturalibre_sneakers" by culturalibre;
- "drednicolson_short-tail_camo_tee" by DredNicolson.

All are CC BY 4.0, from the MakeHuman community asset packs. The CC0 items need no credit: they are by Introspective Mnisi, Joachim Bornemann, Gabriel Mensah and "Free poly" (BlenderKit), and by MargaretToigo, Elvaerwyn and Cortu (MakeHuman). The realistic body is ours, built from Blender Studio's CC0 Human Base Meshes.

## Open issues

- **Not seen in the engine.** The pictures are Blender renders with a rough display fit. None of the bases has been through the engine's import, fitter, `garment_checks` or the walk. Those are the intake track's job once a base is chosen.
- **Sketchfab leads are unverified beyond their API records and pictures.** Each model's own page must be read before download (licence, "textures included", no logo, author is the maker).
- **BlenderKit's CC0 assets were downloaded without the add-on**, through the same public API it calls. The licensing FAQ puts no condition on CC0. If the lead prefers, re-download through the add-on with an account and compare hashes.
- **MakeHuman `elvs_*` files** carry names of other works (`thejournalist`, `darkmaggiehair`). The pack table lists Elvaerwyn as author under CC BY. I did not trace further: none of these items is recommended.
- The MakeHuman system-assets pack (CC0: `short01` to `short04`, `shoes01` to `shoes06`, the casual suits) and `hair01` (CC0) were still downloading (about 160 KB/s) when this report was written. From the pack thumbnails they are the same generation as the items above.

## At merge

Nothing to rebuild. The report and the five JPGs (about 1.6 MB) are the only files. If wave 5 adopts a base, the intake track records its provenance (section 5's hashes) before any file of it is committed.
