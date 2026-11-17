# Skin, makeup and surface detail: references and the recommended model

Stage 0 research for brief §6 (skin, makeup, tattoos and markings) and §13.1 "Skin and materials". Collected 2026-11-16.

This file adds to what the repository already holds. It does not repeat [research.md](../../research.md) §B.5 or [notes/body-face-skin-cloth.md](../../notes/body-face-skin-cloth.md) §F (separable SSS, Burley's profile, Unreal's Subsurface Profile, Godot's forward SSS, pre-integrated skin, GPU Gems 3, Weyrich's face measurements, the eye model). Fabric, hair, eye and general PBR references are in [../materials/README.md](../materials/README.md).

Contents:

- [What the repository has today](#what-the-repository-has-today)
- [Index](#index)
- [1. Recommended skin model](#1-recommended-skin-model)
- [2. Brief §6.1 and §6.2, layer by layer](#2-brief-61-and-62-layer-by-layer)
- [3. Decal architecture for brief §6.3](#3-decal-architecture-for-brief-63)
- [Assets that could ship](#assets-that-could-ship)
- [Not verified](#not-verified)
- [Verification](#verification)

How to read the index:

- **[V]** the primary page, paper or licence text was opened and read this session. **[S]** seen only in a search summary or a secondary page. **[N]** not opened.
- Licence classes are the brief's (§13.5): 1 CC0 or public domain; 2 open with a licence compatible with an Apache-2.0 engine; 3 licensed commercial; 4 reference only; 5 original. A paper's equations and measured facts can be used; its text and figures cannot be copied.
- No image or asset from any source below is in the repository. Links only.

## What the repository has today

| Fact | Evidence |
|---|---|
| The skin maps are baked by the build: albedo, tangent normal, packed occlusion-roughness, 2K by default and 4K for the hero set | `assets/characters/realistic/skin.py:1-28`; `assets/characters/realistic/README.md:24` |
| The albedo is one tone, a constant, varied by region | `skin.py:45` (`TONE = (0.74, 0.53, 0.43)` sRGB), `skin.py:74-75` (`FLUSH`, `PALE` factors), `skin.py:664-671` |
| Regional fields already exist per vertex: flush (nose, chin, cheeks, ears, knuckles, elbows, knees), lips, pale (palms, soles), nail, oily (nose, chin, brow), dry (knuckles, elbows, knees, heels) | `skin.py:344-446` |
| Mottling is three noise scales in the body's own space, so it has no seams | `skin.py:666-669`; grain the same way, `skin.py:538-596` |
| Some shading is baked into the albedo on purpose: creases darker and redder, and the mouth lining's shade | `skin.py:682-684`, `skin.py:693-699` |
| Lips and nails are part of the `Skin` albedo; the mouth lining, gums, teeth, tongue, brows and lashes are separate materials | `skin.py:670-671`, `skin.py:787-801`; `assets/characters/realistic/Realistic.base.json:3957-4009` |
| The occlusion-roughness image's blue channel (metalness) is 0 on skin | `skin.py:707` |
| Texel density is about 10.9 texels per centimetre at 2048, with the face weighted 2.2 times | `assets/characters/realistic/Realistic.build.json:1392-1394`; `assets/characters/realistic/README.md:96` |
| One UV set, 26 islands; the head is one island cut up the back, arms and legs are cut along their inner sides | `assets/characters/realistic/README.md:96` |
| A body region map exists (1024 px, one index per region) | `assets/characters/realistic/regions.py:1-32`; `assets/characters/realistic/README.md:119` |
| The Skin shading model's values are `tint`, `melanin`, `redness`, `undertone`, `roughness_scale`, `detail_strength`, `scatter_scale`. Each has a neutral setting that changes nothing | `crates/avatar/src/material.rs:137-178`, test at `material.rs:361-373` |
| Only `tint`, `iris_tint` and `roughness_scale` are drawn today. The rest are stored and wait for the Skin shading model (CO7) | `crates/scene/src/material.rs:149-164`; `docs/development/character-studio/architecture.md` §10 |
| The realistic base manifest has no skin group and no parameter that drives a skin value. Only the test fixture shows the pattern | `Realistic.base.json:89-155` (every group's category is `body` or `face`; the file has no `material_values`); `crates/avatar/src/fixtures.rs:21`, `fixtures.rs:39-40` |
| A manifest parameter drives a material value through one linear span | `crates/avatar/src/manifest.rs:212-219`; `crates/avatar/src/resolve.rs:15-23`, `resolve.rs:352-360` |
| A description keeps unknown fields, reserved for skin and makeup layers | `crates/avatar/src/description.rs:36-39`; architecture.md §5 ("anything else") |
| The renderer has eight texture slots per material, with metalness, roughness and occlusion as three of them | `crates/render/src/renderer/types.rs:558-560` |
| The renderer already draws projected box decals from the depth and normal prepass, with a flag that leaves characters bare | `crates/render/src/renderer/decals.rs:1-11`, `decals.rs:34-36` |
| The plan already names the targets: D9 (Skin shading model), CO7 (character shading), CO17 (freckles, moles, scars, makeup layers) | `plans/axcenity-engine-character-studio.md:47`, `:133`, `:157` |

## Index

### Skin colour as pigment

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| A Spectral BSSRDF for Shading Human Skin | https://cseweb.ucsd.edu/~henrik/papers/skin_bssrdf/skin_bssrdf.pdf | Donner and Jensen, EGSR 2006 | 2026-11-16 | Skin tone, undertone | Four parameters: melanin fraction, melanin type blend (eumelanin to pheomelanin), haemoglobin fraction, oiliness. Gives the absorption power laws used in section 1.2. Shows light to deep skin by melanin fraction 0.1% to 50% | Class 4 (paper). Equations usable. [V] | data |
| A Layered, Heterogeneous Reflectance Model for Acquiring and Rendering Human Skin | https://gfx.cs.princeton.edu/pubs/Donner_2008_ALH/donner08hskin.pdf | Donner, Weyrich, d'Eon, Ramamoorthi, Rusinkiewicz, SIGGRAPH Asia 2008 | 2026-11-16 | Pigment maps | Six spatially varying parameters, with carotene (0 to 0.05) and epidermal haemoglobin added. Melanin and haemoglobin as painted maps | Class 4. [V] (Table 1 read) | data |
| A Practical Appearance Model for Dynamic Facial Color | https://www.iryoku.com/skincolor/ | Jimenez, Scully, Barbosa, Donner, Alvarez, Vieira, Matts, Orvalho, Gutierrez, Weyrich, SIGGRAPH Asia 2010 | 2026-11-16 | Tone lookup, blush, flush | A real-time lookup table: melanin on u, haemoglobin on v, each sampled by its cube root. Melanin is static; only haemoglobin changes with expression, as a scale and a bias map per pose at 256 x 256. Whole-face flush is a global scale (127% for anger, 81% for fear) | Class 4 (ACM copyright notice on the paper). The authors offered their maps and table through the ACM portal; terms not found. [V] | data |
| Skin Optics | https://omlc.org/news/jan98/skinoptics.html | Jacques, Oregon Medical Laser Center, 1998 | 2026-11-16 | Pigment ranges | Melanosome volume fraction: 1.3 to 6.3% light skin, 11 to 16% moderately pigmented, 18 to 43% darkly pigmented. Epidermis and dermis absorption as mixes of melanin, blood and a baseline | No licence on the page. Facts usable; cite. Class 4. [V] | data |
| Optical Absorption of Hemoglobin | https://omlc.org/spectra/hemoglobin/ | Prahl, Oregon Medical Laser Center | 2026-11-16 | Redness | Tabulated molar extinction of oxygenated and deoxygenated haemoglobin, and the conversion to an absorption coefficient (150 g/L, 64,500 g/mol) | "Copyright 1999 Scott Prahl", no reuse terms. Unverified for redistribution of the table, so treat as class 4; the build may read values and cite. [V] | data |
| Image-based skin color and texture analysis/synthesis by extracting hemoglobin and melanin information in the skin | https://www.mi.tj.chiba-u.jp/~tsumura/Tsumura/papers/siggraph2003.htm | Tsumura and others, SIGGRAPH 2003 | 2026-11-16 | Two-parameter model | Skin colour is close to linear in two pigments in optical density (log) space. This is the basis of a two-number tone model | Class 4. [V] (abstract and citation) | data |
| A Biophysical 3D Morphable Model of Face Appearance | https://openaccess.thecvf.com/content_ICCV_2017_workshops/w16/html/Alotaibi_A_Biophysical_3D_ICCV_2017_paper.html | Alotaibi and Smith, ICCV Workshops 2017 | 2026-11-16 | Pigment maps | Face colour moved into melanin and haemoglobin space, where editing and statistics behave | Class 4. [S] | visual guidance |
| A Hyperspectral Space of Skin Tones for Inverse Rendering of Biophysical Skin Properties, and its code | https://github.com/facebookresearch/BioSkin | Aliaga, Xia, Xie, Jarabo, Braun, Hery, EGSR 2023 | 2026-11-16 | Tone lookup | A learned mapping between skin properties and reflectance, in both directions. Could check the build's tone table offline | Class 2: MIT. Stated in the repository README ("BioSkin is MIT licensed") and confirmed in its `LICENSE` file (MIT License, copyright Meta Platforms). The README's wording is about source code; the pretrained models sit in the same repository with no separate statement. [V] README and licence file | data |
| Reference Data Set of Human Skin Reflectance | https://nvlpubs.nist.gov/nistpubs/jres/122/jres.122.026.pdf | Cooksey, Allen, Tsai, Journal of Research of NIST 122, 2017 | 2026-11-16 | Tone calibration | 100 subjects, inner forearm, 250 to 2500 nm at 3 nm. Data at https://doi.org/10.18434/M38597 | Class 1 in the United States: NIST works are not subject to copyright there; acknowledgement asked, changes must be stated. Where NIST holds rights abroad, the same page grants royalty-free use, changes and distribution worldwide. It is not a CC0 dedication, and the data file itself carries no rights text; the paper points to https://www.nist.gov/director/licensing. [V] | data |
| Characterising the variations in ethnic skin colours: a new calibrated data base for human skin | https://eprints.whiterose.ac.uk/id/eprint/100718/ | Xiao, Yates, Zardawi, Sueeprasan, Liao, Gill, Li, Wuerger, Skin Research and Technology 23, 2017 | 2026-11-16 | Regional variation, undertone | CIELAB means by body site for four groups. Redness is highest on the forehead and cheek (a* about 10.5 to 12), lower on the back of the hand (7.4 to 9.4) and lowest on the inner arm (5.6 to 7.1). The inner arm is the lightest site. Groups differ mostly in yellowness (b* 14.6 to 17.9), little in redness | Class 2: Creative Commons Attribution, stated in the paper and on the repository record (neither names a version). Attribution text needed if the numbers ship. No very deep tones in the sample. [V] | data |
| Physically Based (database of material values) | https://github.com/AntonPalmqvist/physically-based-api | Palmqvist | 2026-11-16 | Tone calibration, scatter radius | "Skin I" to "Skin VI": linear base colours from (0.847, 0.638, 0.552) to (0.09, 0.05, 0.02), index of refraction 1.4, roughness 0.5, subsurface radius (0.482, 0.169, 0.109) for I to III and (0.367, 0.137, 0.068) for IV to VI. Read from the API at https://api.physicallybased.info/v2/materials | Class 1: CC0 1.0, stated in the repository README. Its shader ball image is CC BY 4.0 and is not needed. [V] | data |

### Tone scales

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Monk Skin Tone scale | https://skintone.google/get-started | Monk; published by Google | 2026-11-16 | Palette | Ten tones: `#f6ede4`, `#f3e7db`, `#f7ead0`, `#eadaba`, `#d7bd96`, `#a07e56`, `#825c43`, `#604134`, `#3a312a`, `#292420`. The site's guidance: do not equate skin tone with race | Class 2: Creative Commons Attribution 4.0, stated on the page. Citation asked: Monk, Ellis. "Monk Skin Tone Scale," 2019. https://skintone.google. Read in the page's own source (the page is drawn by script). [V] | data |
| The validity and practicality of sun-reactive skin types I through VI | https://doi.org/10.1001/archderm.124.6.869 | Fitzpatrick, Archives of Dermatology 124, 1988 | 2026-11-16 | Palette | Six types by reaction to sun, not by colour. Widely used as a colour proxy, which later papers call inaccurate | Class 4. No colour values to take. [S] | visual guidance |
| Individual typology angle | https://doi.org/10.1111/bjd.12529 | Del Bino and Bernerd, British Journal of Dermatology 169, 2013 (after Chardon and others, 1991) | 2026-11-16 | Palette | ITA = arctan((L* - 50) / b*) in degrees: a measured lightness and yellowness class from very light to dark. Useful as a check on a palette's spread | Class 4. The class thresholds differ between sources and were not read in the paper. [S] | data |
| SkinTone Guide | https://www.pantone.com/pantone-skintone-guide | Pantone | 2026-11-16 | Palette | A commercial swatch system for skin | Class 4: reference only (a commercial product; nothing from it is used). The page returned 403 to the researcher and 404 to the verifier, so the URL itself is unconfirmed. [N] | visual guidance |

### How character creators expose skin, makeup and marks

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| MetaHuman skin material controls | https://dev.epicgames.com/documentation/metahuman/skin-material-controls | Epic Games | 2026-11-16 | Tone picker, freckles, accents | Tone is a 2D swatch with U and V sliders. Accents add redness by face region (redness, saturation, lightness), always symmetrical. Freckles are their own group | Class 4. The page is drawn by script and could not be read directly. [S] | visual guidance |
| MetaHuman Creator materials tools | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-materials-tools | Epic Games | 2026-11-16 | Makeup | Makeup groups: foundation, blush, lip, eye | Class 4. [S] | visual guidance |
| Skin and Makeup Layers Concepts (SkinGen) | https://manual.reallusion.com/character-creator-4/content/enu/4.0/15_skingen/Basic-Concepts/Skin-and-Makeup-Layers-Concepts.htm | Reallusion | 2026-11-16 | Layer order | Seven skin layers in order: skin base, normal effects, skin details, blemish (freckles, moles, acne, suntan), acquired (wounds, scars, tattoos, dirt), body hair, nails. Six makeup layers, face only: foundation, eye makeup, eyelash, lip, eyebrow, miscellaneous (face tattoos) | Class 4 (commercial product manual). [V] | visual guidance |
| The Sims 4, December 2020 update | https://simscommunity.info/2020/12/08/the-sims-4-december-2020-update-overview/ | Sims Community (secondary); the publisher's own notes could not be reached | 2026-11-16 | Tone, undertone, makeup opacity | Over 100 skin swatches in warm, neutral and cool, each with a light to dark value slider. Makeup gained an opacity slider | Class 4 (commercial game). [S] | visual guidance |
| The Sims 4, tattoo update | https://www.ea.com/games/the-sims/the-sims-4/news/businesses-and-hobbies-expansion-pack-doja-cat-tattoos | Electronic Arts, 2025-03-06 | 2026-11-16 | Tattoos | Tattoos can be layered. Stamps and stencils. Hue, saturation, opacity and brightness per tattoo. A paint mode | Class 4 (commercial game). [V] | visual guidance |
| A Deep Dive Into Dragon Age: The Veilguard's Expansive Character Creator | https://gameinformer.com/exclusive/2024/06/27/a-deep-dive-into-dragon-age-the-veilguards-expansive-character-creator | Game Informer | 2026-11-16 | Tone, undertone, lighting preview | 40 complexions; cool to warm hues; an undertone control and a melanin slider; a vitiligo slider (amount and intensity); more than 30 makeup options, among them eyeliner intensity, colour, glitter, eye shadow, lips and blush; tattoos with intensity; four lighting scenes to switch between while editing | Class 4 (commercial game, press article). [V] | visual guidance |
| Mutable: projector parameter node | https://github.com/anticto/Mutable-Documentation/wiki/Node-Projector-Parameter-(v5.5) | Anticto and Epic Games | 2026-11-16 | Decal placement | A projector the player can move, rotate and resize at runtime. Three kinds: planar, cylindrical (with a wrap angle), wrapping. Named use: "a tattoo on a character or a decal on a jacket". The result is baked into the character's textures | Class 4. Read on the publisher's GitHub wiki. [V] | visual guidance |

### Real-time skin shading not already in the notes

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Efficient Screen-Space Subsurface Scattering Using Burley's Normalized Diffusion in Real-Time | https://www.advances.realtimerendering.com/s2018/Efficient%20screen%20space%20subsurface%20scattering%20Siggraph%202018.pdf | Golubev, SIGGRAPH 2018 Advances course | 2026-11-16 | Subsurface scattering | The profile is normalized, so the blur conserves energy. Two albedo modes: post-scatter (albedo applied once, keeps detail) and pre-and-post (softer). Precomputed, importance-sampled disc samples | Class 4 (slides). [V] | data |
| Diffusion Profile reference | https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/diffusion-profile-reference.html | Unity Technologies | 2026-11-16 | Subsurface controls | The control set a profile needs: scattering colour, multiplier, maximum radius, index of refraction (skin about 1.4), texturing mode, dual lobe multipliers and lobe mix, diffuse shading power | Class 4 (documentation). [V] | visual guidance |
| Blending in Detail | https://blog.selfshadow.com/publications/blending-in-detail/ | Barré-Brisebois and Hill, 2012 | 2026-11-16 | Detail normal, scar normal | Compares linear, overlay, partial derivative, whiteout, UDN and reoriented normal mapping. Reoriented mapping rotates the detail normal onto the base normal | No licence on the code. Reimplement from the mathematics. Class 4. [V] | data |
| Stable Geometric Specular Antialiasing with Projected-Space NDF Filtering | https://www.jcgt.org/published/0010/02/02/ | Tokuyoshi and Kaplanyan, JCGT 2021 (follows their I3D 2019 paper) | 2026-11-16 | Micro roughness | Widen roughness from the normal's screen-space change so fine detail does not sparkle. The same idea turns pore detail into roughness at distance | Class 4. [S] | data |
| Practical Real-Time Strategies for Accurate Indirect Occlusion | https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf | Jimenez, Wu, Pesce, Jarabo, Activision memo | 2026-11-16 | Lighting consistency | Near-field bounce light as a fitted function of occlusion and albedo. Occluded skin picks up its own colour, so creases need no colour baked into the albedo | Class 4. [V] (partly read) | data |
| Measurement-Based Synthesis of Facial Microgeometry | https://vgl.ict.usc.edu/Research/Microgeometry/ | Graham, Tunwattanapong, Busch, Yu, Jones, Debevec, Ghosh, Eurographics 2013 | 2026-11-16 | Detail normal | Skin microstructure below what a face scan holds changes the specular lobe. Measured skin patches are offered for download | No licence or terms on the page for the data: reference only, class 4. [V] | visual guidance |
| Next Generation Character Rendering | https://www.iryoku.com/downloads/Next-Generation-Character-Rendering-v6.pptx | Jimenez and von der Pahlen, GDC 2013 | 2026-11-16 | Dual specular, cavity, eyes | The production summary of real-time skin and eyes. A 324 MB slide file | Class 4. [N] (not opened; the notes cite it at [S]) | visual guidance |
| Digital Human package (The Heretic) | https://github.com/Unity-Technologies/com.unity.demoteam.digital-human | Unity Technologies | 2026-11-16 | Wrinkles, tear line | Skin attachment for brows and lashes, a tear-line normal blur, wrinkle maps driven by skin tension, full skin, eye, teeth and hair shader graphs | Class 4: reference only. Licence file not opened, so unverified. [V] page | visual guidance |

### Regional colour, marks and decals

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Colour zones of the face | https://gurneyjourney.blogspot.com/2008/05/color-zones-of-face.html | Gurney, 2008 | 2026-11-16 | Regional variation | A painter's rule for light skin: forehead paler and more golden, the band from brow to nose base redder, the lower face cooler. Subtle, and stronger in men | Class 4 (blog). [V] | visual guidance |
| Brown spots and freckles | https://dermnetnz.org/topics/brown-spots-and-freckles | DermNet | 2026-11-16 | Freckles, sun spots | Freckles are usually under 3 mm, brown from melanin, on the mid face and sometimes wider, common in fair skin, stronger in summer. Sun spots run from a few millimetres to centimetres, even in colour, on the face and hands, and stay through winter | Class 4. Facts usable; images not. [V] | data |
| Physically based cosmetic rendering | https://people.cs.nycu.edu.tw/~wclin/cosmetics.html | Huang, Huang, Lin, Chuang, Computer Animation and Virtual Worlds 24, 2013 | 2026-11-16 | Makeup | Foundation, blush and lipstick as thin layers over skin (Kubelka-Munk), measured from real products, drawn in real time over screen-space skin. Pearl and glitter left out | Class 4. [S] | data |
| Rendering Wounds in Left 4 Dead 2 | https://www.slideshare.net/alexvlachos/gdc-2010-left4dead2-wounds | Vlachos, Valve, GDC 2010 | 2026-11-16 | Marks on skinned meshes | Wounds placed in the reference pose with ellipsoids and a projected texture, so they stay on the body as it animates | Class 4. [S] | visual guidance |
| Rendering Wounds on Characters | https://tomlooman.com/unreal-engine-render-character-wounds/ | Looman | 2026-11-16 | Marks on skinned meshes | Two ways: paint into a texture per character in UV space, or mask in the shader against the position before skinning. The second is limited to a few marks per character | Class 4 (blog). [V] | visual guidance |
| Using decals | https://docs.godotengine.org/en/stable/tutorials/3d/using_decals.html | Godot Engine | 2026-11-16 | Projected decals | A projected box decal can change albedo, normal, occlusion-roughness-metalness and emission, with the albedo's alpha masking the others. A cull mask keeps decals off chosen objects. Nothing is said about skinned meshes | Class 4 (documentation; the engine is MIT). [V] | visual guidance |
| The Devil is in the Details: idTech 666 | https://www.advances.realtimerendering.com/s2016/Siggraph2016_idTech6.pdf | Sousa and Geffroy, SIGGRAPH 2016 Advances course | 2026-11-16 | Projected decals | Decals in a clustered forward renderer, drawn in the main pass | Class 4. [S] | visual guidance |
| KHR_texture_transform | https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_texture_transform/README.md | Khronos Group | 2026-11-16 | Decal placement in UV space | Offset, rotation and scale of a texture in UV space, applied as translation times rotation times scale. Ratified. The workspace already turns the loader's feature on (`Cargo.toml:99-100`) | Class 4. A specification; the licence of its text was not read, and the idea needs none. [V] | data |

### Skin detail assets

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| ambientCG | https://docs.ambientcg.com/license | ambientCG | 2026-11-16 | Detail maps | Has no human skin or pore material. A search of its API for "skin" returned one leather. Useful for fabric and leather only | Class 1: "All ambientCG assets are provided under the Creative Commons CC0 1.0 Universal License". [V] | actual asset |
| Poly Haven | https://polyhaven.com/license | Poly Haven | 2026-11-16 | Detail maps | Has no human skin material. Textiles, leather and lighting environments only | Class 1: "Our assets are all licensed as CC0". Site text and renders are not CC0. [V] | actual asset |
| Human Face Skin Texture (Skin 0001) | https://www.texturecan.com/details/574/ | TextureCan | 2026-11-16 | Pore detail normal | A procedural, tileable face-skin material (base colour, normal, roughness, displacement, occlusion) at 1K to 4K | Class 1 by the site's terms page (https://www.texturecan.com/terms/), which covers all its textures: "Creative Commons CC0 1.0", redistribution with projects allowed. The asset's own page states no licence; it carries only the site's "CC0 Textures" banner and a link to those terms. Made in a commercial material tool; check the downloaded files' own notice before use. [V] terms and asset page | actual asset (candidate) |
| Human Materials 1 | https://sketchfab.com/3d-models/human-materials-1-cc0-be48a7526b304914b7c9f45c289b2ecf | A Sketchfab user | 2026-11-16 | Pore detail | Seamless 2K skin materials | Unverified, so class 4: the title and description say CC0, the licence field says CC Attribution, and how the maps were made is not stated. Do not use. [V] | visual guidance |
| TexturingXYZ, 3D Scan Store, Substance materials | https://texturing.xyz/pages/terms-of-service (the others: notes/body-face-skin-cloth.md §B) | Various | 2026-11-16 | Pore and scan detail | The quality bar for scanned pore detail | Class 4: reference only. Commercial libraries; the TexturingXYZ terms forbid redistributing raw materials and releasing them under an open source licence. Never files. [V] terms | visual guidance |
| Original, made by the build | `assets/characters/realistic/skin.py` | This project | 2026-11-16 | Detail normal, masks, spots | The build already makes grain, mottling and regional fields without any outside texture | Class 5 | actual asset |

**Finding:** no CC0 scan-quality pore or makeup texture was found. The skin detail normal, every makeup mask and every spot map should be original (class 5), made by the build as the grain is today.

## 1. Recommended skin model

The aim: the description holds **pigment amounts**, not a colour. A colour comes out of them the same way for every tone, so marks, makeup and lighting behave alike on all tones. This keeps the existing contract: a value modulates the authored maps and its neutral setting changes nothing (`crates/avatar/src/material.rs:1-7`).

### 1.1 What a description holds

All on the `Skin` material slot (`materials.Skin` in the description, architecture.md §5 and §10). No format change for the seven values that exist. The proposed `variation` is an added value name: a build without it reports an unknown name as a problem (`crates/avatar/src/material.rs:309-315`), so it needs the same care as any additive field.

| Value | Status | Range, neutral | Meaning in the model |
|---|---|---|---|
| `melanin` | exists (`material.rs:142-148`) | -1 to 1, 0 | Depth. An offset from the authored map in palette units: 1.0 is the full palette span, about 52 L* (section 1.3). Darker is positive |
| `undertone` | exists (`material.rs:156`) | -1 to 1, 0 | Cool (-1) to warm (1). The melanin type blend, plus a small yellow term, at constant lightness |
| `redness` | exists (`material.rs:149-155`) | -1 to 1, 0 | Haemoglobin: pallor to flush. Fades with depth, as real skin does |
| `tint` | exists (`material.rs:138-141`) | colour, white | A final multiply. For fantasy skin only; a realistic character leaves it white |
| `roughness_scale` | exists, drawn today (`scene/src/material.rs:159-163`) | 0.5 to 1.5, 1 | Oily to dry |
| `detail_strength` | exists, waits for CO7 | 0 to 2, 1 | Strength of the pore detail normal |
| `scatter_scale` | exists, waits for CO7 | 0 to 2, 1 | Subsurface radius |
| `variation` | **proposed** | 0 to 2, 1 | Strength of mottling and regional colour (brief §6.1 "skin variation"). 0 is an even, airbrushed tone |

Olive is not a fourth undertone value. It is warm with less red: `undertone` about +0.5 and `redness` about -0.6. That matches the measurements: groups differ in yellowness much more than in redness (Xiao 2017).

Each of these should also be a **manifest parameter** in a new `skin` group of the realistic base, driving the material value through `material_values` (`manifest.rs:212-219`), as the fixture does (`fixtures.rs:39-40`). Then the studio draws the Skin panel from the manifest like Body and Face, and nothing is hard-coded.

### 1.2 From values to albedo

**The physics, as published.** Skin colour is set by two absorbers in two layers. Wavelength `λ` in nanometres, coefficients per millimetre (Donner and Jensen 2006, equations 3 to 8, read in the paper):

```
eumelanin      a_eu(λ)   = 6.6e10 · λ^-3.33
pheomelanin    a_ph(λ)   = 2.9e14 · λ^-4.75
baseline       a_base(λ) = 0.0244 + 8.53 · exp(-(λ - 154) / 66.2)
epidermis      a_epi  = Cm · (βm · a_eu + (1 - βm) · a_ph) + (1 - Cm) · a_base
dermis         a_derm = Ch · (γ · a_oxy + (1 - γ) · a_deoxy) + (1 - Ch) · a_base,   γ = 0.75
scattering     s'(λ)  = 14.74 · λ^-0.22 + 2.2e11 · λ^-4     (halved in the dermis)
```

`Cm` is the melanin fraction, `βm` the eumelanin share, `Ch` the haemoglobin fraction. `a_oxy` and `a_deoxy` come from the haemoglobin tables: `a = 2.303 · e(λ) · 150 / 64500` per centimetre (Prahl). Published ranges: `Cm` 0 to 0.5, `Ch` 0 to 0.32 in the dermis with a quarter of that in the epidermis (Jimenez 2010, Table 1); melanosome fraction 1.3 to 6.3% for light skin and 18 to 43% for dark skin (Jacques). Carotene is a small third absorber (0 to 0.05, Donner 2008) and is the physical reason for a yellow term.

**The practical form.** Nobody evaluates that per pixel. Jimenez 2010 precomputes a table of RGB colours over melanin and haemoglobin, sampled by cube root (`u = Cm^(1/3)`, `v = Ch^(1/3)`), and multiplies a colourless skin shading by the table's colour. Tsumura 2003 shows why a table this small is enough: in log space, skin colour is nearly a straight-line mix of the two pigments.

**Recommended for this engine: a tone table made by the build.**

1. A build script (numpy only, no Blender) evaluates the equations above over a grid of depth, undertone and redness, turns each spectrum into linear sRGB under D65, and writes a small table of **ratios to the authored tone**. The table is data beside the base manifest. Suggested size: 17 depths by 5 undertones by 5 redness steps.
2. The table is re-spaced so that equal steps of depth are equal steps of lightness (L*), at every undertone. Then a palette spread evenly in values is spread evenly to the eye.
3. The entry at `melanin = 0`, `undertone = 0`, `redness = 0` is exactly (1, 1, 1). The authored map is untouched at neutral, as the contract requires.
4. `ratio = table(melanin, undertone, redness)`, by trilinear lookup. `albedo = authored albedo × ratio`, in linear space.

A single exponent per channel is **not** enough. In an indicative run of the equations (see the note under the palette), the density per unit of depth moved from about (1.3, 2.1, 3.2) for red, green and blue at the light end to about (3.0, 2.8, 2.7) at the deep end. A fixed vector would make deep tones too saturated. This is why `melanin` is "along a melanin absorption curve rather than a plain darkening" (`material.rs:147`) and why the curve should be a table.

**Two steps to build it.**

| Step | Where | What it gives | Limit |
|---|---|---|---|
| A. One ratio per character | `scene`'s `modulate`, which already multiplies the base colour by `tint` (`crates/scene/src/material.rs:149-158`). Add the table lookup beside it | Pigment-true skin tones on the Standard shading the engine draws today. No shader work, no new texture | Lips, nails, palms and soles tone with the rest, because they are in the same albedo (`skin.py:670-671`). Acceptable for a first cut within about ±0.25 of the authored depth; wrong for deep tones, where palms and lips must stay lighter |
| B. Per texel, with masks | The Skin shading model (CO7) | Palms, soles, lips and nails keep their own pigment. Flush follows `redness`. Marks become pigment (section 2) | Needs the shader and two mask channels |

**Masks for step B.** Two per-texel masks, both already computed by the build as vertex fields:

- a **melanin mask**: 1 on ordinary skin, low on palms and soles (the `pale` field, `skin.py:429-438`), 0 on lips and nails;
- a **haemoglobin mask**: the `flush` field (`skin.py:366-394`) plus the lips.

The occlusion-roughness image's blue channel is unused on skin (`skin.py:707`), so one mask fits there with no new texture. One condition: the loader reads that channel as metalness today (`crates/assets/src/model/maps.rs:759`, tested at `crates/assets/src/model/tests.rs:185`), so a mask there makes skin metallic wherever the material is still drawn as Standard, and in any other glTF tool. It only works once the Skin shading model ignores metalness, and it ends the "ordinary glTF maps" promise in `skin.py:25-27`. The second can take the normal map's blue channel if the shader rebuilds z, or a channel of the overlay described in section 2. With masks, the per-texel ratio is the table looked up at `melanin × melanin mask` and `redness` raised by the haemoglobin mask. The character's slice of the table can reach the shader as a tiny texture or as a few fitted coefficients; that choice belongs to CO7.

**Variation.** Because the authored tone is one known constant (`skin.py:45`), the strength of all baked colour variation can be changed per texel without a second map: `albedo' = TONE × (albedo / TONE)^variation`, masked off lips and nails. At 0 the skin is one even tone; at 1 it is as authored.

**What does not change with tone.** Specular reflectance stays at the skin value for every tone (F0 about 0.028, notes §F). Deep skin then looks glossier against its darker diffuse, which is correct and must not be "fixed" by scaling specular. The subsurface radius should shrink a little as depth rises: the CC0 database above lists (0.482, 0.169, 0.109) for its three lighter skins and (0.367, 0.137, 0.068) for its three deeper ones.

### 1.3 The palette: 36 tones by number

Nine depths by four undertones. Tones are named by depth number and undertone letter, never by a food, a place or a people. A tone is three values, so the palette is data in the base manifest and a game can extend it.

- **Depth 1 to 9**: `melanin = (depth - 3) × 0.125`, from -0.25 to +0.75. Depth 3 is the authored map (L* 61). Each step is 6.5 L*, from 74 down to 22.
- **Undertone**: C cool (`undertone` -0.7), N neutral (0), W warm (+0.7), O olive (`undertone` +0.5 and `redness` -0.6).
- **Number**: `(depth - 1) × 4 + column`, 01 to 36.

| Depth | `melanin` | Target L* | C (cool) | N (neutral) | W (warm) | O (olive) |
|---|---|---|---|---|---|---|
| 1 | -0.250 | 74.0 | 01 `#d9aca0` | 02 `#d9ac9e` | 03 `#daac9d` | 04 `#d9ad93` |
| 2 | -0.125 | 67.5 | 05 `#cb9986` | 06 `#cc9984` | 07 `#ce9980` | 08 `#cc9a7a` |
| 3 | 0.000 | 61.0 | 09 `#bb8871` | 10 `#bd876e` | 11 `#bf8669` | 12 `#bd8865` |
| 4 | +0.125 | 54.5 | 13 `#aa7760` | 14 `#ac765c` | 15 `#af7557` | 16 `#ad7655` |
| 5 | +0.250 | 48.0 | 17 `#976652` | 18 `#99664e` | 19 `#9c644a` | 20 `#9b6549` |
| 6 | +0.375 | 41.5 | 21 `#835847` | 22 `#855743` | 23 `#88553f` | 24 `#87563f` |
| 7 | +0.500 | 35.0 | 25 `#6d4a3d` | 26 `#6f493a` | 27 `#724836` | 28 `#714837` |
| 8 | +0.625 | 28.5 | 29 `#573d34` | 30 `#593d31` | 31 `#5b3c2e` | 32 `#5a3c2f` |
| 9 | +0.750 | 22.0 | 33 `#41312a` | 34 `#433128` | 35 `#4b3327` | 36 `#473227` |

Tone 10 (3N) is the authored `TONE`.

**The colours are indicative, not final.** They are the albedo's mean colour, computed this session from the equations in section 1.2 with a simplified two-layer Kubelka-Munk solve (0.06 mm epidermis) and haemoglobin values typed in by hand. They were not checked against a reference renderer. The build must recompute them from the tabulated sources. What the run does show:

- The published pigment ranges cover the palette. Depth 1 needs about 1.5 to 2% melanin and depth 8 about 50%.
- Depth 9 needed more melanin than the published 0 to 0.5 range in the simplified solve, and tones 35 and 36 stopped short of their target (L* 23.8 and 22.9). The deep end needs the full model or a direct calibration.
- With all melanin removed the authored map reaches about L* 78. That is the lightest tone the map can honestly give.
- The melanin type blend alone moves yellowness by only about 2 b* on light skin and 5 on mid tones. Undertone therefore needs the small yellow term, calibrated to about ±5 b* at constant lightness.
- Redness stops being visible at depth 9 (no change at all in the run) and is weak from depth 6. A blush on deep skin has to be a makeup layer, not haemoglobin.

**Checks against published scales.**

| Scale | What it says | Consequence |
|---|---|---|
| Monk scale (CC BY 4.0) | Its ten swatches run from L* 94 to 15 (computed from the hex values). Tones 1 to 4 are all above L* 87 | They are swatches for labelling pictures, not albedo. Do not use the hex values as skin colours. Use the scale to label coverage. By nearest lightness: depths 1 and 2 sit nearest Monk 5 (L* 78), depths 3 and 4 Monk 6 (L* 55), depths 5 and 6 Monk 7 (L* 42), depths 7 and 8 Monk 8 (L* 31), depth 9 Monk 9 (L* 21). Monk 1 to 4 are lighter than any albedo step and Monk 10 (L* 15) is deeper than any step. A swatch and an albedo are not the same quantity, so this is a rough label, not a match |
| Physically Based database (CC0) | Skin I to VI run from about L* 86 to 28.5 as linear base colours | Its lightest skin is lighter than depth 1. Consider one more light step once the build's table exists |
| Xiao 2017 (CC BY) | Mean measured skin: L* 58 to 61, a* 9 to 10, b* 15 to 18 | The authored tone (L* 61, a* 17, b* 22) has the measured lightness but is more saturated than measured skin. Albedo is not the same thing as measured surface colour, so this is a note, not a defect |

### 1.4 The advanced picker

Basic view: the 36 swatches, drawn from the tone table so they match the viewport.

Advanced view (brief §6.1 "a more detailed color picker for advanced users"):

| Control | Drives | Notes |
|---|---|---|
| A 2D field: depth across, undertone up | `melanin`, `undertone` | Painted from the tone table. The 36 swatches sit on it as dots. This is the 2D swatch with two sliders that MetaHuman uses |
| Redness | `redness` | Greyed towards the deep end, where it does nothing |
| Variation | `variation` | Even to blotchy |
| Roughness | `roughness_scale` | Oily to dry |
| Detail intensity | `detail_strength` | Pores |
| Subsurface | `scatter_scale` | |
| Numeric entry for each | the same values | Numeric entry is in CO12's scope (`plans/axcenity-engine-character-studio.md:138`) |
| Fantasy tint | `tint` | Collapsed by default, with a line of text that it leaves the realistic range |

No RGB or hue wheel in the realistic path. That is what brief §6.1 means by "avoid relying on a single RGB color parameter".

### 1.5 Staying consistent under different lights

- Albedo holds diffuse reflectance only: no highlights, no directional shadow. The two baked exceptions (`skin.py:682-684` crease tint, `skin.py:693-699` mouth shade) are deliberate and commented. Leave them until CO7; then replace the crease tint with bounce light computed from occlusion and albedo (the Activision memo above), so it follows the skin tone. A fixed reddish factor is tuned for light skin.
- Keep toned albedo inside a safe range: nothing above about 0.8 linear, nothing below about 0.02. A test over the 36 tones can assert it.
- Tone in linear space. Never in sRGB.
- Subsurface with post-scatter texturing (Golubev) so freckles and makeup edges stay sharp.
- Every palette tone must be judged under the four preview lights in [../materials/README.md](../materials/README.md), not under one.

## 2. Brief §6.1 and §6.2, layer by layer

"Made how" is one of: **build mask** (a procedural mask the build bakes from fields it already computes), **seed-placed** (spots placed by a seed over a density field), **value** (no texture, a material value), **authored** (an image file in the content library).

Blend order runs from 0 (bottom) to 10 (top). Layers 3 and 4 are pigment and take effect inside the tone step, so they look right on every tone and under every light.

| Brief item | Today | Made how | Inputs in the description | Changes | Order |
|---|---|---|---|---|---|
| §6.1 Skin tone | `melanin` stored, not drawn | value, through the tone table | `melanin` | albedo | 1 |
| §6.1 Undertone | `undertone` stored, not drawn | value | `undertone` | albedo | 1 |
| §6.1 Roughness | `roughness_scale` drawn | value | `roughness_scale` | roughness | 2 |
| §6.1 Variation | baked into albedo at one strength (`skin.py:664-669`) | value, by the power form in 1.2 | `variation` | albedo | 2 |
| §6.1 Subsurface controls | `scatter_scale` stored | value | `scatter_scale` | scatter radius | shading |
| §6.1 Detail intensity | grain baked into the normal map (`skin.py:538-596`) | build mask: a small tiling detail normal made by the build with the same cells-and-noise recipe at pore scale, laid over the base normal by reoriented blending, fading into roughness with distance | `detail_strength` | normal, roughness | 2 |
| §6.1 Freckles | none | seed-placed. Spots under 3 mm, densest on the nose, cheeks, shoulders and forearms (a new `sun` field beside `flush`). Each spot adds melanin | `amount`, `strength`, `seed`, optional `region` | melanin mask, so albedo | 3 |
| §6.1 Moles | none | seed-placed, few (tens on a body), 2 to 6 mm, with a slight bump | `amount`, `seed` | melanin mask, normal | 3 |
| §6.1 Blemishes | none | seed-placed red spots and patches. Adds haemoglobin | `amount`, `seed` | haemoglobin mask, slight roughness | 3 |
| §6.1 Scars | none | authored or build-made stamp at an anchor (section 3). Removes melanin, adds a little red when fresh, smoother, with a normal | `id`, `anchor`, `rotation`, `scale`, `opacity` | melanin and haemoglobin masks, normal, roughness | 4 |
| §6.2 Foundation | none | build mask: the head, less lips, eyes and brows (`HEAD_SETS` in `common.py`, used at `skin.py:249`) | `colour` (default: the character's own tone), `opacity`, `roughness`, `enabled` | albedo pulled towards one colour, which hides variation, freckles and redness; roughness pulled towards one value (matte about 0.6, dewy about 0.35) | 6 |
| §6.2 Blush | none; a fixed flush is baked (`skin.py:381-382`) | build mask: the cheek blobs the flush already uses, with two or three placement presets | `colour`, `opacity`, `roughness`, `preset`, `enabled` | albedo, roughness | 7 |
| §6.2 Eyeshadow | none | build mask from the lid's face sets and distance from the lash line; several shapes as presets | `colour`, `opacity`, `roughness`, `preset`, `enabled` | albedo, roughness (low for shimmer) | 7 |
| §6.2 Eyeliner | none | build mask from distance to the lash line: thin, winged, smudged | `colour`, `opacity`, `roughness`, `preset`, `enabled` | albedo, roughness | 8 |
| §6.2 Lip colour | lips are a fixed colour in the albedo (`skin.py:47`, `skin.py:670`) | build mask: the `lips` field (`skin.py:397-428`) | `colour`, `opacity`, `roughness` (matte about 0.6 to gloss about 0.15), `enabled` | albedo, roughness | 9 |
| §6.2 Eyebrow colour | `Brows` is its own material with the Hair value set | value: no layer needed | `materials.Brows`: `melanin`, `redness`, `dye`, `dye_amount` (`material.rs:226-271`) | brow colour | separate material |
| Mascara (not in the brief) | `Lashes` is its own material | value | `materials.Lashes` | lash colour | separate material |

Notes:

- **Presets** (brief §6.2) are named sets of these values in the content library, like outfit presets. A makeup preset is data.
- **Metalness** is not used for gloss. Skin and cosmetics are not metals; gloss is low roughness. Glitter is a later, separate effect.
- **Resolution.** The median is 10.9 texels per centimetre at 2048 and the face is weighted 2.2 times. If the weight is against the median, the face has about 24 texels per centimetre, and a 1 mm eyeliner is two or three texels wide. Eyeliner needs the 4096 set or a face-only overlay at higher density.
- **Tattoos** sit at order 5: ink is under the surface, so it multiplies the albedo and leaves roughness alone. Body paint and decorative marks sit at 10, on top, and replace the colour. See section 3.

### Where the layers are combined

| Option | How | For | Against |
|---|---|---|---|
| In the skin shader, one texture per layer | The shader samples a mask per layer | No per-character texture | The material has eight texture slots (`types.rs:558-560`). Sixteen layers do not fit |
| **Composite into two overlay images per character (recommended)** | A pure function takes the layer stack and writes: a pigment overlay (melanin and haemoglobin offsets, for orders 3 and 4) and a surface overlay (colour with coverage, roughness with coverage, for orders 5 to 10). The shader reads two extra textures however many layers there are | Fixed shader cost. Deterministic and testable without a GPU. Characters with the same layers share one overlay, as the identity bake shares a rest mesh (plan D4) | Slots: all eight are named today (`types.rs:558-560`). Packing occlusion, roughness and metalness (plan D9, CO7) frees two, which the two overlays use up. The tiling detail normal recommended above is then a ninth texture, and a tone table as a texture a tenth. CO7 must settle the budget: reuse the emissive, opacity or specular slots on skin, or fold the detail normal into an overlay. Memory per distinct character: a 2048 RGBA image is 16 MiB before compression. Use 1024 for the body and a face crop at higher density; bake for crowds |
| Bake everything into final albedo, normal and roughness | The same function, run to the end | Zero shader cost; works on Standard shading today | A full texture set per distinct character. This is the "baked for games" step the plan already defers (architecture.md §10, CO17 and CO18) |

The recommended option and the full bake are the same code at two stopping points. Build the compositing function first; it serves the studio preview, the runtime and the export.

In the description, layers go in an additive field (`Description::extra` keeps unknown fields today, `description.rs:36-39`), for example `"skin_layers": [{ "id": "axcenity/blush-soft", "colour": "#b5524a", "opacity": 0.4, "roughness": 0.5 }]`. Each `id` names a layer definition in the content library, which holds the mask or the recipe, the kind (which decides the order) and the ranges. Old builds keep the field untouched on a round trip.

## 3. Decal architecture for brief §6.3

Brief §6.3 asks for tattoos, birthmarks, scars, decorative markings and custom decals, with placement, rotation, scale, opacity and material blending, and says to settle the architecture before building any authoring tool.

| Option | How it works | Follows the skin | Seams | Cost | Fit |
|---|---|---|---|---|---|
| A. Stamp in UV space | The image is placed in the skin's UV space with an offset, rotation and scale (the glTF texture transform's three numbers) and composited into the overlay | Yes, exactly | Breaks at island edges: the arms and legs are cut along their inner sides, the torso down its sides (`realistic/README.md:96`). A band round an arm cannot cross the cut. Size varies with texel density (face 2.2 times) | None at draw time | Good for small marks inside one island |
| **B. Project in the rest pose, bake to UV (recommended)** | The decal is placed on the body's rest surface: an anchor, a rotation about the surface normal, a size in metres. The compositor finds each texel's rest position, projects it into the decal's frame and samples the image. The result goes into the same overlay as makeup | Yes: once baked it is texture | None: the projection is continuous in 3D, so a mark crosses a UV cut cleanly | None at draw time. Needs a rest position per texel, which the compositor can rasterize from the mesh once per base | The model Mutable uses for tattoos (planar and cylindrical projectors, baked to texture) |
| C. Runtime projected decal | The renderer's existing box decals, parented to a bone | No: a box projects through a deforming mesh, so the mark slides over the skin | None | Per frame; reads the prepass | Right for passing effects (mud, a splash). The renderer already has it and can leave characters bare (`decals.rs:34-36`). Wrong for tattoos |
| D. Shell mesh | A patch cut from the body surface with an alpha material, worn like an item | Yes | None | An extra draw; sorting and offset problems | Works with today's item system, as the underwear cut from the body does (architecture.md §9, the interim exception). A fallback only |

**Recommendation: B as the one contract, with A as its simplest case and no authoring tool.**

1. **Mark definitions in the content library.** A manifest per mark: an image with alpha (PNG), the kind (`tattoo`, `birthmark`, `scar`, `paint`), the blend (`multiply` for ink under the skin, `pigment` for birthmarks and scars so they tone with the skin, `over` for paint), optional roughness and normal, and a natural size in metres. Importing a mark is importing a picture. No painting tool is needed.
2. **Named anchors in the base manifest.** A short list such as upper arm (outer, left and right), forearm, shoulder blade, chest, calf, cheek: each a point on the rest surface with a frame and a largest size. The body region map (`regions.py`) says which regions an anchor may cover.
3. **Marks in the description**, in an additive field beside the layers: `"marks": [{ "id": "axcenity/mark-band", "anchor": "left_upper_arm_outer", "offset": [0.0, 0.01], "rotation": 15, "scale": 1.2, "opacity": 0.9, "mirror": false }]`. These are the five controls the brief lists. `offset` is in metres along the anchor's frame.
4. **One compositor.** Marks go through the same function as the makeup layers (section 2), at order 4 (scars), 5 (tattoos) or 10 (paint).
5. **Later, without changing the data:** free placement by clicking the body (the anchor becomes a triangle and barycentric coordinates, which the viewport picking planned for brief §3.4 would yield), cylindrical projection for bands, layered tattoos with hue and opacity each (as The Sims 4 does), a paint mode.

What Stage 5 should build of this: the description field and its validation, the mark manifest, three or four anchors, planar projection, and one test mark of each blend. That proves the architecture and leaves the tool for later, as the brief asks.

Birthmarks, vitiligo and scars are pigment. A birthmark adds melanin or haemoglobin; vitiligo removes melanin; an old scar removes melanin and smooths. Treating them as pigment, not as painted colour, is what makes them look right on all 36 tones.

## Assets that could ship

| Asset | Class | Where the licence was read | Use |
|---|---|---|---|
| Monk Skin Tone scale (ten values) | 2, CC BY 4.0 | https://skintone.google/get-started | Coverage labels and a check on the palette. Needs the citation in the credits |
| NIST skin reflectance data (100 spectra) | 1 in the United States | https://www.nist.gov/director/licensing | A check on the build's tone table. Acknowledge NIST |
| Xiao 2017 skin colour means by body site | 2, CC BY | The paper's first page | Targets for regional redness and lightness. Attribution |
| Physically Based database, skin entries | 1, CC0 1.0 | https://github.com/AntonPalmqvist/physically-based-api | Scatter radius and base colour cross-checks |
| BioSkin code and models | 2, MIT (README and `LICENSE` file) | https://github.com/facebookresearch/BioSkin | Offline check of the tone table. Not a runtime dependency |
| TextureCan Skin 0001 | 1 by the site's terms (the asset page states none of its own) | https://www.texturecan.com/terms/ | A candidate pore detail normal, if an original one falls short. Check the files' own notice first |
| Detail normal, makeup masks, spot placement, test marks | 5, original | made by the build | The recommended source for all of them |

## Not verified

- Epic's MetaHuman pages are drawn by script and were read only through search summaries. Control names are [S].
- The Sims 4 December 2020 notes on the publisher's site returned 404; the entry rests on a fan site.
- The Pantone page returned 403.
- The ITA class thresholds were not read in a primary paper.
- The albedo range (about 0.02 to 0.8 linear) is common practice; the vendor guide that states it could not be opened.
- Mole counts and sizes in section 2 are general knowledge, not from a source read this session.
- The palette colours and the density figures come from a simplified solve run this session. They show the shape of the model, not final numbers.
- The Jimenez 2010 maps and lookup table are said to be on the ACM portal; their terms were not found. Do not use them; compute the table.
- Whether the normal map's blue channel is free depends on how CO7 reads normals. Not checked in the renderer.
- The roughness figures for makeup in section 2 (matte about 0.6, dewy about 0.35, gloss about 0.15) are starting values with no source.
- Freckles on the shoulders and forearms in section 2 have no source read this session; the page read gives the mid face, "sometimes more widely".

## Verification

A second pass on 2026-11-16 and 2026-11-17 checked this file and [../materials/README.md](../materials/README.md) against their sources and against the repository. Papers and pages were read in a scratch folder outside the repository; nothing was added to it.

### Licences confirmed at the source

| Entry | Checked where | Result |
|---|---|---|
| Monk Skin Tone scale | The get-started page's own script | CC BY 4.0, the citation line and all ten hex values match. The site's own CIELAB values match the L* figures in section 1.3 |
| NIST skin reflectance data | The paper, the data file's header and the licensing page | 100 subjects, 250 nm upward in 3 nm steps. Not subject to copyright in the United States; acknowledgement and a notice of changes asked; a worldwide royalty-free grant covers rights held abroad |
| Xiao 2017 | The paper's first page and the repository record | Creative Commons Attribution, published version; no licence version named |
| Physically Based database | Repository README and the public API | CC0 1.0 for the data; every skin and eye value quoted here matches; 122 materials and 25 light sources |
| BioSkin | Repository README and `LICENSE` file | MIT |
| ambientCG | Licence page and the public API | CC0 1.0 for all assets; 106 fabric and 50 leather results; one leather for "skin" |
| Poly Haven | Licence page and the public API | CC0 for all assets; 44 textures in the textiles and leather section, none of skin; 96 studio environments |
| TextureCan Skin 0001 | Terms page and the asset page | The terms cover all textures as CC0 1.0. The asset page states no licence of its own |
| Khronos PBR Neutral | Repository `LICENSE.md` | Apache-2.0 |
| OpenPBR | Repository `LICENSE` | Apache-2.0 |

### Numbers confirmed at the source

| Numbers | Source read |
|---|---|
| The two melanin power laws, the baseline, the scattering law, γ = 0.75, melanin 0.1% to 50% | Donner and Jensen 2006, equations 3 to 8 and figure 3 |
| Six parameters; carotene and epidermal haemoglobin 0 to 0.05 | Donner 2008, Table 1 |
| Melanin 0 to 0.5, dermal haemoglobin 0 to 0.32, epidermal at a quarter of dermal, cube-root sampling, 256 x 256 maps, 127% and 81% | Jimenez 2010, Table 1, equation 7 and section 7 |
| Melanosome fraction 1.3 to 6.3%, 11 to 16%, 18 to 43% | Jacques 1998, section 1.3 |
| `a = 2.303 · e · 150 / 64500` | Prahl's page |
| Every CIELAB range quoted from Xiao 2017 | Its Table 2 |
| Hair pigment coefficients and the three eumelanin amounts | The book page |
| Freckles under 3 mm; sun spots a few millimetres to centimetres | DermNet |

### Corrections made

| Where | Was | Now | Why |
|---|---|---|---|
| Index, SkinTone Guide | Class 3 | Class 4, reference only; URL noted as unconfirmed | A commercial swatch system is never above reference only; the page did not load for either pass |
| Index, TexturingXYZ row | "Class 3 or 4"; no URL | Class 4; the terms page URL | Commercial scan libraries are reference only; the column needs a URL |
| Index, Digital Human, Prahl, KHR_texture_transform, Sketchfab pack | No class number | Class 4 stated | Every row needs a class |
| Index, BioSkin | Licence file not opened | MIT confirmed in the file | Opened |
| Index, NIST data | Class 1 with "foreign rights may apply" | The worldwide grant and "not CC0" added | Read on the licensing page |
| Index, TextureCan | Class 1 by the terms page | Same class; the asset page's lack of its own statement recorded | Per-asset check |
| Index, Veilguard article | "makeup with intensity, colour and glitter" | The article's own list | Read in the article |
| Index, Diffusion Profile | "maximum radius in millimetres" | "maximum radius" | The page read gives no unit |
| Index, Mutable | "Read through a wiki mirror" | Read on the publisher's GitHub wiki | The URL is the primary page |
| "What the repository has", manifest row | `Realistic.base.json:103-148` | `:89-155`, and the fixture's group line | The groups array spans those lines |
| Section 1.1 | "No format change" | The proposed `variation` is a new value name that an older build reports as a problem | `material.rs:309-315` |
| Section 1.2, masks | The blue channel "fits with no new texture" | The loader reads blue as metalness today | `maps.rs:759`, `tests.rs:185` |
| Section 1.3, Monk row | Depths 1 to 3 cover Monk 1 to 5, and so on | A nearest-lightness mapping, with its limits | The old mapping had no stated method and did not match the L* figures |
| Section 2, overlay option | "Fits the eight slots" | The slot budget spelled out | Two overlays use the two slots CO7 frees; the detail normal is a ninth texture |
| Materials file, OpenPBR | Licence unsure | Apache-2.0, class 2 | `LICENSE` file read |
| Materials file, sheen talk | Code licence not read | The code repository lists Apache-2.0 | Repository page read |
| Materials file, Khronos tone mapper | "repository licence" | `LICENSE.md` read | Opened |
| Materials file, fabric presets row | "Class 3 or 4"; no URL | Class 4; the product URL | As above |
| Materials file, Veilguard article | Scenes "added because skin tones had read badly under one fixed light" | The article ties them to the earlier game's green-lit creator; it does not name skin tones | Read in the article |
| Materials file, eyes note | Database and notes "agree (about 1.33 to 1.376)" | They differ: 1.376 against about 1.33 | `notes/body-face-skin-cloth.md` §F lines on the cornea |
| Materials file, Poly Haven textiles | "44 scanned textures" | 44 textures, two of them floor coverings | API listing |

### Checked and left as written

- Every `path:line` in "What the repository has today" and in sections 1 to 3 was opened and matches, apart from the rows above.
- The L* figures for the authored tone, the Monk swatches, the database's skins and all 36 palette colours were recomputed from the stated values and match. Tone 10 is exactly the authored `TONE`.
- All 66 index rows across the two files have the eight columns filled. No file from any source is in the two folders. No personal details were found.
- Fifteen further index URLs were opened and show what their rows say (the Veilguard article, SkinGen, the tattoo update, Mutable, the Diffusion Profile page, the Sketchfab pack, DermNet, the hair chapter, the Jimenez page, Jacques, Prahl, the decals page, the wounds article, the sheen talk, the colour zones post).

### Still open

- The indicative run behind the palette colours, the density figures and the "L* 78 with no melanin" limit was not reproduced. They remain the researcher's own figures.
- `melanin` runs from -1 to 1 in code. In palette units +1 would be about L* 9 and -1 lighter than the map can give, so the tone table must clamp or the range must be re-stated when CO7 defines the curve.
