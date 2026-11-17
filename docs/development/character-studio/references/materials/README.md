# Character materials: fabric, hair, eye and general PBR references

Stage 0 research for brief §13.1 "Skin and materials", §8.3 (clothing materials) and the "improve material previews" item of brief Stage 5. Collected 2026-11-16.

Skin itself, makeup and decals are in [../skin/README.md](../skin/README.md). This file adds to what the repository already holds and does not repeat it:

| Topic | Already covered in |
|---|---|
| Sheen (the glTF sheen extension, the "Charlie" distribution), Filament's cloth model, anisotropy, per-fabric look, seams and trim sheets | [notes/body-face-skin-cloth.md](../../notes/body-face-skin-cloth.md) §G; [research.md](../../research.md) §B.6 |
| Hair shading (Kajiya-Kay, Marschner, d'Eon, Chiang, Karis, Tafuri), Blender's Principled Hair, card and strand pipelines | [notes/hair.md](../../notes/hair.md); research.md §B.1 |
| Eyes (two-layer model, cornea refraction, limbal ring, occlusion shell, tear line) | notes/body-face-skin-cloth.md §F; research.md §B.3 |
| glTF material extensions and which ones the loader reads | [notes/formats-and-libraries.md](../../notes/formats-and-libraries.md) §1 |

How to read the index: **[V]** the primary page or licence text was opened this session; **[S]** seen only in a search summary; **[N]** not opened. Licence classes are the brief's (§13.5): 1 CC0 or public domain; 2 open and compatible with an Apache-2.0 engine; 3 licensed commercial; 4 reference only; 5 original. No file from any source is in the repository.

## What the repository has today

| Fact | Evidence |
|---|---|
| Five shading models with a value set each: Standard, Skin, Eye, Cloth, Hair | `crates/avatar/src/material.rs:17-31`, `material.rs:132-271` |
| Cloth takes `tint`, `sheen_tint`, `roughness_scale`, `sheen_roughness_scale`, `anisotropy_scale` | `material.rs:206-224` |
| Hair takes `melanin`, `redness`, `dye`, `dye_amount`, `tip_lightening`, `highlight`, `roughness_scale`, `tint` | `material.rs:226-271` |
| Eye takes `iris_tint`, `sclera_tint`, `pupil_size`, `limbal_ring`, `iris_depth` | `material.rs:180-204` |
| All of them draw as Standard today; only `tint`, `iris_tint` and `roughness_scale` have an effect | `crates/scene/src/material.rs:149-164` |
| The studio's stage has four lighting presets: Studio, Daylight, Sunset, Night. A preset sets only the time of day and a grey ambient that is 5% bluer than neutral | `crates/editor/src/character_workspace/stage.rs:124-161`, `stage.rs:228-240` |
| Four backdrops: Grey, Light, Dark, Sky | `stage.rs:162-175` |

## Index

### Fabric and leather

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| ambientCG, fabric and leather materials | https://ambientcg.com/api/v2/full_json?q=fabric&type=Material | ambientCG | 2026-11-16 | Cloth base maps, weave normals | Scanned and procedural PBR sets. Its API returned 106 results for "fabric" (Fabric001 to Fabric086, with carpets, wicker and chainmail) and 50 for "leather" (Leather001 to Leather039) | Class 1: "All ambientCG assets are provided under the Creative Commons CC0 1.0 Universal License", read at https://docs.ambientcg.com/license. Raw files may be included in a game. [V] | actual asset |
| Poly Haven, textiles and leather | https://api.polyhaven.com/assets?type=textures&category=textiles-leather | Poly Haven | 2026-11-16 | Cloth base maps, weave normals | 44 textures in the section (API listing; 43 tagged fabric, six of those leather, and two floor coverings among the 44): denim (six), cotton jersey, jersey melange, stretch poplin, rough linen, ribbed corduroy, wool bouclé, poly-wool herringbone, knitted and polar fleece, terry cloth, velour velvet, crepe satin, crepe georgette, jacquards, gingham, scuba suede, brown, red and white leather | Class 1: "Our assets are all licensed as CC0", read at https://polyhaven.com/license. The site's own text and renders are not CC0. [V] | actual asset |
| TextureCan | https://www.texturecan.com/terms/ | TextureCan | 2026-11-16 | Cloth and skin base maps | Procedural PBR sets with base colour, normal, roughness, displacement and occlusion | Class 1 by its terms page, which covers all its textures: "Creative Commons CC0 1.0", redistribution with projects allowed; the user must avoid any logos in a texture. Asset pages state no licence of their own. [V] | actual asset (candidate) |
| Practical Multiple-Scattering Sheen Using Linearly Transformed Cosines | https://tizianzeltner.com/projects/Zeltner2022Practical/ | Zeltner, Burley, Chiang, SIGGRAPH 2022 Talks | 2026-11-16 | Sheen | A sheen lobe from fibres standing on the surface, with the backward scattering the earlier sheen lacked. Gives cloth a soft edge glow and can read as dust | Class 4 (paper). Code is linked (github.com/tizian/ltc-sheen); its repository page lists Apache-2.0, and the licence file was not opened. [V] page | data |
| OpenPBR Surface | https://github.com/AcademySoftwareFoundation/OpenPBR | Academy Software Foundation | 2026-11-16 | Layering order | A standard surface model with a fuzz layer over a coat over the base. A reference for what order sheen, coat and base sit in | Class 2: Apache-2.0, read in the repository's `LICENSE` file. A badge in its README has stale alternative text naming another licence; the badge links to the same file. [V] page and licence file | data |
| PBR Guide | https://helpx.adobe.com/substance-3d-designer/using/pbr-guide-part-2.html | Adobe | 2026-11-16 | Base colour ranges | The usual source for "keep base colour free of lighting" and for the safe brightness range of non-metals | Class 4. The page returned 403 and was not read. [N] | visual guidance |
| Marvelous Designer and CLO fabric presets | https://www.marvelousdesigner.com/ (see also research.md §A.4) | CLO Virtual Fashion | 2026-11-16 | Fabric behaviour | Named fabric presets as a vocabulary for the wardrobe's material picker | Class 4: reference only (a commercial product). Never files. Only the product's home page was opened. [S] | visual guidance |

### Hair

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Scattering from Hair (Physically Based Rendering, 4th edition) | https://pbr-book.org/4ed/Reflection_Models/Scattering_from_Hair | Pharr, Jakob, Humphreys | 2026-11-16 | Hair colour from pigment | Absorption from two pigment amounts: `eumelanin × (0.419, 0.697, 1.37) + pheomelanin × (0.187, 0.4, 1.05)` in RGB. Eumelanin near 8, 1.3 and 0.3 gives black, brown and blonde. This is the curve the Hair model's `melanin` and `redness` values should follow | Book text CC BY-NC-ND 4.0: do not copy text. The coefficients are published values and may be used with a citation. Class 4. [V] | data |
| Principled Hair BSDF | https://docs.blender.org/manual/en/latest/render/shader_nodes/shader/hair_principled.html | Blender Foundation | 2026-11-16 | Hair controls | Melanin, melanin redness, tint, random colour and random roughness as the user-facing control set. The build's reference renders use it | Class 4 (manual). Already opened for notes/hair.md; not re-read. [N] | visual guidance |

### Eyes

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Physically Based database, eye entries | https://github.com/AntonPalmqvist/physically-based-api | Palmqvist | 2026-11-16 | Eye constants | Cornea index of refraction 1.376; lens 1.386; sclera 1.4 with linear base colour (0.652, 0.5, 0.394) and subsurface radius (0.482, 0.169, 0.109). Each entry cites its sources | Class 1: CC0 1.0, stated in the repository README. [V] | data |
| Next Generation Character Rendering | https://www.iryoku.com/downloads/Next-Generation-Character-Rendering-v6.pptx | Jimenez and von der Pahlen, GDC 2013 | 2026-11-16 | Eye shading | The production reference for eye refraction, the wet edge and eye occlusion | Class 4. A 324 MB slide file, not opened. [N] | visual guidance |

### General PBR and measured values

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Physically Based database | https://physicallybased.info/ | Palmqvist | 2026-11-16 | Material constants, light sources | 122 materials with base colour, roughness, index of refraction and, where it applies, subsurface radius: six skins, blood, bone, eye parts, plastics, metals. 25 light sources, among them a standard illuminant, a black body, sun, sky, a household bulb and film lights. No fabrics | Class 1: CC0 1.0 for the database, read in the repository README. Its shader ball image is CC BY 4.0 and is not needed. [V] | data |
| Khronos PBR Neutral tone mapper | https://github.com/KhronosGroup/ToneMapping | Khronos Group | 2026-11-16 | Material preview | A tone mapper made to show a material's base colour, hue and saturation faithfully under grey light, one to one up to a set brightness, with the rest kept for highlights. A filmic curve shifts skin and fabric colours; this one does not | Class 2: Apache-2.0, read in the repository's `LICENSE.md`. [V] | data |
| Moving Frostbite to Physically Based Rendering | https://seblagarde.wordpress.com/2015/07/14/siggraph-2014-moving-frostbite-to-physically-based-rendering/ | Lagarde and de Rousiers, SIGGRAPH 2014 course | 2026-11-16 | Lighting units, material model | A complete engine write-up: material model, light units, image-based lights. It lists energy conservation between diffuse and specular in layered materials as an open problem, which is the case for skin under makeup and cloth under sheen | Class 4 (course notes). [V] page | data |
| Practical Physically Based Shading in Film and Game Production | https://blog.selfshadow.com/publications/s2012-shading-course/ | SIGGRAPH 2012 course; Burley's "Physically Based Shading at Disney" among the talks | 2026-11-16 | Material model | The origin of the principled parameter set most tools share | Class 4. Course page opened; the talk itself was not. [V] page | visual guidance |
| Blending in Detail | https://blog.selfshadow.com/publications/blending-in-detail/ | Barré-Brisebois and Hill, 2012 | 2026-11-16 | Detail normals | How to lay a tiling weave or pore normal over a base normal without flattening either | No licence on the code; reimplement from the mathematics. Class 4. [V] | data |
| Stable Geometric Specular Antialiasing with Projected-Space NDF Filtering | https://www.jcgt.org/published/0010/02/02/ | Tokuyoshi and Kaplanyan, JCGT 2021 | 2026-11-16 | Shimmer on fine detail | Widen roughness where the normal changes quickly on screen, so weave and pore detail does not sparkle under a moving camera | Class 4. [S] | data |
| Practical Real-Time Strategies for Accurate Indirect Occlusion | https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf | Jimenez, Wu, Pesce, Jarabo, Activision | 2026-11-16 | Occlusion colour | Bounce light in creases as a function of occlusion and base colour, so folds in cloth and skin keep their colour without baking it in | Class 4. [V] (partly read) | data |

### Lighting environments for previews

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| Poly Haven HDRIs, studio | https://api.polyhaven.com/assets?type=hdris&categories=studio | Poly Haven | 2026-11-16 | Neutral preview light | 96 studio environments (API listing). `studio_small_08` is tagged low contrast; `studio_small_09` and `photo_studio_01` medium contrast | Class 1: CC0, read at https://polyhaven.com/license. [V] | actual asset |
| Poly Haven HDRIs, outdoor and overcast | https://api.polyhaven.com/assets?type=hdris&categories=outdoor,overcast | Poly Haven | 2026-11-16 | Cool and outdoor preview light | Overcast skies such as `overcast_soil_puresky` and `mud_road_puresky` give soft, cool, shadowless light | Class 1: CC0. [V] | actual asset |
| glTF Sample Environments | https://github.com/KhronosGroup/glTF-Sample-Environments | Khronos Group | 2026-11-16 | Preview light | The environments the glTF sample viewer uses, with a "neutral" one. Several come from third-party probe collections | Unverified: no licence is shown for the repository or per file. Do not ship. [V] | visual guidance |
| A Deep Dive Into Dragon Age: The Veilguard's Expansive Character Creator | https://gameinformer.com/exclusive/2024/06/27/a-deep-dive-into-dragon-age-the-veilguards-expansive-character-creator | Game Informer | 2026-11-16 | Lighting presets in a creator | Four lighting scenes the player can switch between at any time while editing. The article sets this against the earlier game's creator, whose green-tinted light made characters look different in play; it does not name skin tones | Class 4 (commercial game, press article). [V] | visual guidance |

## Notes for implementation

**Fabric.** The CC0 libraries cover the common fabrics well enough to ship: denim, jersey, poplin, linen, corduroy, wool, fleece, terry, velvet, satin and leather. What they give is base colour, normal, roughness and occlusion. They do not give sheen colour, sheen roughness or anisotropy; those are a few numbers per fabric and are original (class 5), set per the look table in notes/body-face-skin-cloth.md §G. A garment's `tint` multiplies the base colour (`material.rs:206-207`), so a scanned fabric used for tinting should be close to white or grey; most scans are not, and need their colour evened out by the build first.

**Hair.** The Hair model's `melanin` and `redness` (`material.rs:228-241`) should follow the two-pigment absorption above, not a darkening and a red tint. Then one pair of numbers runs from blonde through red and brown to black, and skin and hair share one idea of pigment. `Brows` and `Lashes` are Hair materials on the realistic base (`assets/characters/realistic/Realistic.base.json:3957-3985`), so eyebrow colour and mascara need no makeup layer.

**Eyes.** The CC0 database lists 1.376 for the cornea. The notes' recommendation uses about 1.33 (the default of the node they cite is 1.333), so the two differ; CO7 should pick one and record which. Sclera is not white: its listed base colour is a warm off-white, and it scatters like light skin.

## Previewing materials under four lights

Brief §6.1 asks for "consistent rendering under different lighting conditions" and Stage 5 for better material previews. A material judged under one light is tuned to that light. The stage needs four fixed, named conditions and a way to see a character under each quickly.

| Condition | What it is | What it catches | Nearest preset today |
|---|---|---|---|
| Neutral | White light near 6500 K, soft, from the front and above with a weak fill; mid-grey backdrop; fixed exposure; a neutral tone curve | The true base colour. Skin tone and fabric colour are read here and nowhere else | Studio, but Studio is the sun at 14:30 with a slightly blue ambient (`stage.rs:152-158`, `stage.rs:231-240`), so it is not neutral |
| Warm | A low, warm key near 3000 K, as from a lamp or a low sun | Skin going orange; red and pink fabrics merging with skin; makeup vanishing | Sunset |
| Cool | Overcast sky or shade, 7500 K or more, almost shadowless | Skin going grey or green; whether subsurface colour still shows without a strong key; cool fabrics merging | none (Night is dark, not cool daylight) |
| Outdoor | Direct sun with blue sky fill and hard shadows | Specular on skin and satin; pore and weave detail shimmer; shadow edges on skin; the terminator's red band | Daylight |

Recommendations:

1. **Add a true neutral condition.** A fixed white key and fill that do not depend on the time of day, a grey backdrop (the Grey backdrop exists, `stage.rs:162-175`), fixed exposure, and the neutral tone mapper if the renderer's own curve shifts hue. This is the reference for every palette swatch and every thumbnail.
2. **Add a cool overcast condition.** Either a time and weather setting of the existing sky or a CC0 overcast environment from Poly Haven.
3. **Keep the conditions as named data**, not as code, so tests and captures use the same four. The visual QA captures ([visual-qa.md](../../visual-qa.md)) should take each benchmark character under all four.
4. **A four-up view.** One button that shows the character under the four conditions side by side, or cycles them on a key. The Veilguard creator's four switchable scenes are the precedent.
5. **A reference object.** A mid-grey sphere and a row of the skin palette's nine neutral swatches as flat chips beside the character in the neutral condition. If the grey sphere is not grey, the preview is wrong before any material is.
6. **Tests that need no eye.** For each of the four conditions and each of the 36 palette tones, render the head and assert that mean lightness falls in order with depth and that no channel clips. A material that only works in one light fails here.

What "consistent" means in practice:

- Base colour holds no lighting. Occlusion is its own channel. See the skin file, section 1.5.
- Specular reflectance for non-metals stays near 0.04 (skin 0.028); never tint or scale it to fix a look in one light.
- Sheen adds energy at grazing angles; the base under it must be dimmed by the same amount (the albedo scaling in the notes, §G), or cloth glows under the outdoor rim light.
- Roughness detail must turn into wider roughness with distance, or satin and oily skin shimmer under the outdoor key.

## Assets that could ship

| Asset | Class | Where the licence was read | Use |
|---|---|---|---|
| ambientCG fabric and leather sets | 1, CC0 1.0 | https://docs.ambientcg.com/license | Base maps and weave normals for garments |
| Poly Haven textiles and leather (44) | 1, CC0 | https://polyhaven.com/license | The same |
| Poly Haven HDRIs (studio, overcast) | 1, CC0 | https://polyhaven.com/license | The neutral and cool preview conditions |
| Physically Based database | 1, CC0 1.0 | https://github.com/AntonPalmqvist/physically-based-api | Constants for eyes, skin scatter and light colours |
| Khronos PBR Neutral tone mapper | 2, Apache-2.0 | https://github.com/KhronosGroup/ToneMapping | The neutral preview's tone curve. Needs a NOTICE entry if code is taken |
| TextureCan sets | 1 by the site's terms | https://www.texturecan.com/terms/ | A further source; check each file's own notice |

## Not verified

- Epic's Unreal and MetaHuman material pages are drawn by script and could not be read; none is cited here.
- Adobe's PBR guide returned 403.
- Whether the engine's tone curve shifts skin hue was not measured. Recommendation 1 depends on it.
- The colour temperatures in the four conditions are conventional photographic values, not taken from a source read this session.
- The sheen code's licence file was not opened (its repository page lists Apache-2.0). OpenPBR's licence file has since been read: Apache-2.0.
- The specular figures (0.04 for non-metals, 0.028 for skin) are the usual ones; 0.028 is in the notes (§F), 0.04 was not taken from a source read this session.

A second pass checked this file. Its results and every correction made here are listed in the "Verification" section of [../skin/README.md](../skin/README.md#verification).
