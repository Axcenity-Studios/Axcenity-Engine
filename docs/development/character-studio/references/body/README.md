# Body references: anatomy and body parameter design

Stage 0 research for brief §4 (body customization) and §13.3 (anatomical reference collection). Collected 2026-11-16.

This folder holds links and numbers only. No image, scan or mesh from any source below is stored in the repository.

| File | What it holds |
|---|---|
| This file | What the body already has, the general index, and the four design tables (controls, regions, presets, implausible combinations) |
| [overall/README.md](overall/README.md) | Whole-body proportions: anthropometry tables, proportion canons, standing and walking references |
| [upper/README.md](upper/README.md) | Head and neck, shoulders, chest, arms, hands, abdomen, waist, back: references, data, MakeHuman targets, raised-arm and bent-elbow checks |
| [lower/README.md](lower/README.md) | Hips, thighs, calves, feet: references, data, MakeHuman targets, bent-knee and walking checks |

Licence classes are the brief's (§13.5): **1** CC0 or public domain, **2** open with a licence an Apache-2.0 engine can redistribute (named each time), **3** licensed commercial, **4** reference only, **5** original. "Unverified" means the licence was not read on the source's own page today, or that what was read there is not a copyright licence.

## 1. What the body has today

Read from source, not from the docs.

| Thing | State | Evidence |
|---|---|---|
| Frame | `frame`, 0 to 1, offered as two choices (Male 0, Female 1). Drives `Frame_Feminine` and bone offsets | `assets/characters/realistic/Realistic.base.json:158`, choices at `:742` |
| Whole-body build | `muscle` and `fat`, each -1 to 1. They walk MakeHuman's muscle by weight grid from where each frame's body sits on it | `assets/characters/realistic/shapes.py:62-65` (`PLACE`), `Realistic.base.json:940`, `:949` |
| Build by region | `muscle_<region>` and `fat_<region>` for chest, shoulders, arms, abdomen, back, hips, thighs, calves (16 parameters, advanced). Each adds up to 0.6 of the step to the heavy corner inside a smooth region | `shapes.py:66-67` (`REGIONS`, `LOCAL`), `shapes.py:196-220` (region weights from bone weights) |
| Proportions | Nine bone-scale parameters, 0.85 to 1.15; `hip_width` 1.0 to 1.15 only | `assets/characters/realistic/manifests.py:52`, `:55`; `Realistic.base.json:761-920` |
| What the proportions scale | `BodyHeight`: spine and legs. `LegLength`, `ArmLength`: both segments of the limb. `ShoulderWidth`: collarbones, and upper chest by the square root. `WaistWidth`, `HipWidth`: Spine and Hips bones side to side. `HeadSize`, `HandSize`, `FootSize`: uniform | `crates/anim/src/body_shape.rs:98-158` |
| Shapes in the model | `Body` carries 155 blend shapes today: 48 muscle and fat (2 frames x (8 whole-body + 16 region)), `Frame_Feminine`, 6 shoulder correctives, 3 underwear masks, and 97 for the face | Target names in `assets/characters/realistic/RealisticParts.glb`; `Realistic.build.json` (`body_shapes.shapes` = 48, `correctives` has 6 entries) |
| Source of the shapes | 16 MakeHuman corner bodies (young, 2 genders x 3 muscle x 3 weight, less the two averages), from 24 macro targets. 6 regional targets were read for the spike's sheets only | `assets/characters/transfer/shapes/LICENCE-RECORD.md`, `assets/characters/transfer/makehuman.sha256` |
| Held back | Fat shapes held to 15% over bone; nothing moves lids, sockets, lip edges; heaviest combinations kept apart in the rest pose | `shapes.py:68` (`HOLD`), `shapes.py:227-286`, `shapes.py:309-326` |
| Constraints | The manifest has no `constraints`. `max_sum` rejects any parameter whose range goes below 0. `follow` is defined but nothing applies it | `crates/avatar/src/manifest.rs:242-254`, `:532-533`; `crates/avatar/src/resolve.rs:45-62`; no use of `Constraint::Follow` outside `manifest.rs` |
| Presets | Six character presets: Male, Female, Athletic male and female, Heavyset male and female | `crates/editor/src/character_workspace.rs:145-190` |
| Randomizer | None in `crates/avatar` or the workspace | search of both for "random" returns nothing |

Joint heights of the masculine rest pose, read from `RealisticParts.glb` (metres): ankle 0.068, knee 0.500, hip 0.898, Spine 0.994, Neck 1.474, Head 1.586, shoulder joint 1.395 at x = ±0.148, top of head 1.750. These give the reach of each proportion slider in section 3.

## 2. General index

Region-specific entries are in the three sub-folders. IDs (B01 …) are used in the tables below.

| Reference | Source URL | Author or publisher | Date collected (2026-11-16) | Feature | What it demonstrates | Licence or restrictions | Use (visual guidance / data / actual asset) |
|---|---|---|---|---|---|---|---|
| B01 ANSUR II public data files (4,082 men, 1,986 women, 93 measures) | https://www.openlab.psu.edu/ansur2/ | U.S. Army Natick Soldier RDEC; hosted by a university design lab | 2026-11-16 | All body dimensions | Per-person measurements in millimetres. Every mean, SD, percentile, ratio and correlation in these files was computed from it today (the computed stature mean and SD match the report's table) | Unverified as a copyright licence; safe as numbers. The report's cover reads "Approved for public release; distribution is unlimited": a release marking, not CC0 or a public-domain dedication, and the report names contractor co-authors. The hosting page says the files "were made available publicly in 2017" and states no licence. Statistics computed from the data are facts and may be used; do not vendor the CSV files | Data |
| B02 ANSUR II report, NATICK/TR-15/007 (2014) | https://tools.openlab.psu.edu/publicData/ANSURII-TR15-007.pdf | U.S. Army Natick Soldier RDEC | 2026-11-16 | Measurement definitions | How each dimension is taken (landmark, posture, tape level). Needed before comparing a mesh measure to a population number | Unverified as a copyright licence. "Approved for public release; distribution is unlimited" (cover and form block 12) is a release marking; part of the authors are a contractor's staff. Use the definitions and numbers as facts; copy no text or figure. The defence archive's copy (accession ADA611869) refused a fetch (HTTP 403) | Data |
| B03 ANSUR 1988 report, NATICK/TR-89/044 | https://tools.openlab.psu.edu/publicData/ANSUR-TR89-044.pdf | U.S. Army Natick RD&E Center | 2026-11-16 | Head height | Menton to top of head (H30), the one head-height measure; ANSUR II does not have it. Read from a scanned table, so check the page (printed pages 526-527) before relying on a digit | Unverified as a copyright licence. "Approved for public release; distribution unlimited" (cover) is a release marking; the report was prepared by a contractor. Use the numbers as facts | Data |
| B04 NHANES anthropometric reference data 2015-2018 (Vital Health Stat 3(46), 2021) | https://www.cdc.gov/nchs/data/series/sr_03/sr03-046-508.pdf | U.S. National Center for Health Statistics | 2026-11-16 | Height, weight, BMI, waist, arm and upper-leg length | The civilian population, which is heavier than the Army sample: use it for the heavy end | Class 1. "All material appearing in this report is in the public domain and may be reproduced or copied without permission" (page ii) | Data |
| B05 CDC FastStats: body measurements | https://www.cdc.gov/nchs/fastats/body-measurements.htm | U.S. National Center for Health Statistics | 2026-11-16 | Height, weight, waist | Latest averages (2021-2023): men 68.9 in, 199.0 lb, waist 40.6 in; women 63.5 in, 171.8 lb, waist 38.5 in | Unverified: the page states no licence (the report series it cites is public domain, B04) | Data |
| B06 Drillis and Contini (1966), segment lengths as fractions of stature | Not online at source (NYU report 1166.03); the repository's constants cite Winter, *Biomechanics and Motor Control of Human Movement*, 4th ed., fig. 4.1 | New York University; Wiley for the textbook | 2026-11-16 | Segment ratios | Head 0.130 H, shoulder width 0.259 H, upper arm 0.186 H, forearm 0.146 H, hand 0.108 H, hip joint 0.530 H, knee 0.285 H, foot 0.152 H. Already the QA reference (`assets/characters/humanoid/qa/measure.py:37-52`) | Unverified at source. The ratios are facts; the figure is the publisher's | Data |
| B07 MakeHuman targets at the pinned commit (1,280 `.target` files) | https://github.com/makehumancommunity/makehuman/tree/a8bc2d54ff0ac92e78ff71431b1023eda42bf482/makehuman/data/targets | MakeHuman team | 2026-11-16 | Every region | Artist-made body shapes on one topology. Section 4 lists which folders are still unused | Class 1, CC0. Read today in `LICENSE.md` at that commit, section C: assets include "Targets and modifiers" and "have been released under CC0 1.0 Universal". 12 unused targets sampled today all carry the header "This asset was explicitly released as CC0"; the verifier read the header of all 627 files in `armslegs`, `torso`, `hip`, `stomach`, `buttocks`, `pelvis`, `neck`, `measure`, `bodyshapes`, `head`, `breast` and `macrodetails/proportions`: every one carries it | Actual asset (data only, never code) |
| B08 MakeHuman modifier lists | https://github.com/makehumancommunity/makehuman/tree/a8bc2d54ff0ac92e78ff71431b1023eda42bf482/makehuman/data/modifiers | MakeHuman team | 2026-11-16 | Control grouping | Which targets pair into one slider (decr/incr, in/out) and how they group (torso, hip, stomach, armslegs, measure). A naming aid | Class 1 by the same `LICENSE.md` ("Targets and modifiers"). They are JSON data; the Python that reads them is AGPL and is not to be read or copied | Data |
| B09 Blender Studio Human Base Meshes v1.4.1 | https://www.blender.org/download/demo-files/ | Blender Studio | 2026-11-16 | Base body | The realistic male and female bodies the build already uses; also a realistic skeleton for bony landmarks | Unverified today. CC0 as recorded in `assets/characters/transfer/shapes/LICENCE-RECORD.md`; the page refused a direct fetch on both days (HTTP 403), so the statement was not re-read at source. The v1.4.1 archive is present on the download server | Actual asset (already vendored) |
| B10 Anny parametric body | https://github.com/naver/anny | NAVER LABS Europe | 2026-11-16 | Parameterization | MakeHuman's shape space with phenotype parameters calibrated to population data | Class 2 for the code, Apache-2.0; its MakeHuman-derived data (`data/mpfb2`) is CC0 (repository README, read 2026-11-17). Its optional SMPL-X-compatible topology is "for non-commercial use only": never take that part | Data |
| B11 DXA body composition reference values from NHANES (PLoS One 2009, e7038) | https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0007038 | Kelly, Wilson, Heymsfield | 2026-11-16 | Body fat | At age 25 fat is about 24% of body weight in men and 38% in women. Fat mass index over 6 (men) or 9 (women) kg/m² is overweight; 9 or 13 is obese | Class 2, CC BY ("open-access article distributed under the terms of the Creative Commons Attribution License", read on the article page) | Data |
| B12 Sex differences in human adipose tissues: the biology of pear shape (Biol Sex Differ 3:13, 2012) | https://link.springer.com/article/10.1186/2042-6410-3-13 | Karastergiou, Smith, Greenberg, Fried | 2026-11-16 | Fat distribution | Subcutaneous fat is 80-90% of body fat, mainly abdominal, subscapular, gluteal and femoral. Visceral fat is 6-20%, higher in men. At the same BMI women carry about 10% more body fat, more of it on hips and thighs. Visceral fat rises in women at menopause and in men with age | Class 2, CC BY 2.0 (read under "Rights and permissions") | Data |
| B13 Visible Human Project | https://www.nlm.nih.gov/research/visible/visible_human.html | U.S. National Library of Medicine | 2026-11-16 | Layering | Cross-sections of a male and a female body: where bone, muscle and fat lie under the skin at each level | Class 1. The page calls it a "public-domain library" and says "As of 2019, a license is no longer required" | Visual guidance |
| B14 OpenStax Anatomy and Physiology 2e | https://openstax.org/books/anatomy-and-physiology-2e/pages/1-introduction | OpenStax | 2026-11-16 | Muscles, joints, pelvis | Plain descriptions of the muscles that shape the silhouette and of joint movement | Class 4. The book's page and catalogue entry read today say **CC BY-NC-SA 4.0**, not CC BY. NonCommercial and ShareAlike rule it out of anything shipped | Visual guidance |
| B15 "Anterior and Posterior Views of Muscles" (figure from the first OpenStax edition) | https://commons.wikimedia.org/wiki/File:1105_Anterior_and_Posterior_Views_of_Muscles.jpg | OpenStax, via Wikimedia Commons | 2026-11-16 | Muscle map | Front and back views of the surface muscles, labelled | Class 2, CC BY 4.0 (file page). Attribution: OpenStax, *Anatomy and Physiology*, with a link to the licence | Visual guidance |
| B16 Gray's Anatomy (1918) plates | https://commons.wikimedia.org/wiki/Category:Gray%27s_Anatomy_plates | Wikimedia Commons | 2026-11-16 | Muscles, bones | 528 plates of the 20th edition: back, deltoid, thigh adductors, lower leg | Class 1 per the category ("plates from the public domain (1918) textbook"); check each file page | Visual guidance |
| B17 Z-Anatomy | https://github.com/LluisV/Z-Anatomy | Z-Anatomy project | 2026-11-16 | 3D écorché | A free 3D atlas with every muscle and bone as a model: an open stand-in for a commercial anatomy viewer | Class 4 for us: CC BY-SA 4.0 (repository). ShareAlike makes it unfit to derive shipped shapes from | Visual guidance |
| B18 Anatomy for Sculptors | https://anatomy4sculptors.com/ | Anatomy For Sculptors | 2026-11-16 | Form | Books (*Understanding the Human Figure*, *Arm and Hand in Motion*) and a 3D reference library: simplified forms, fat pads, muscle in motion | Class 4. "All rights reserved" (footer) | Visual guidance |
| B19 Visible Body | https://www.visiblebody.com/ | Visible Body (Cengage Learning) | 2026-11-16 | Muscle and bone layers | Interactive 3D anatomy; a free "Learn" site at /learn/ | Class 4. Commercial, copyright in the footer | Visual guidance |
| B20 3D.sk | https://www.3d.sk/ | 3D.sk | 2026-11-16 | Photo sets | "Human photo references and textures for artists": turnarounds and pose sets of ordinary bodies of many builds | Class 4. Commercial; terms at /site/termsOfUse were not read in full. The site refused the verifier's fetch (HTTP 403), so the quoted tagline is unconfirmed | Visual guidance |
| B21 ArtStation | https://www.artstation.com/ | ArtStation (Epic Games) | 2026-11-16 | Character art | Body sculpts and anatomy studies by working artists | Class 4. Artists' copyright; the terms page refused a fetch | Visual guidance |
| B22 MetaHuman Creator body parameters | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-body-params-tool-in-unreal-engine | Epic Games | 2026-11-16 | Control design | Global sliders (masculine to feminine, muscularity, fat, height) plus region groups (upper torso, lower torso, neck and arms, legs, hands); linked sliders with pinning; two read-only measures (shoulder height, rise). The page names no unit. Read 2026-11-17 | Class 4. Product documentation | Visual guidance |

**29 further entries** are in the sub-folders (11 in overall, 11 in upper, 7 in lower, where two more rows repeat an upper-body source for its lower-body content): 51 in all.

## 3. Brief §4.1 controls against the existing parameters

Population numbers are ANSUR II (B01), men / women, mean ± SD with the 1st to 99th percentile in brackets, in millimetres unless said. NHANES (B04) is added where the civilian population differs. "Reach" is what the slider does to the built body, computed from the joint heights in section 1.

| Control (brief §4.1) | Existing parameter | Measurement it changes | Population | Reach today | Recommended range |
|---|---|---|---|---|---|
| Body frame | `frame`, two choices | Stature, shoulder to hip ratio, pelvis, bust | Women / men: stature 0.927, biacromial breadth 0.879, hip breadth 1.023. Bideltoid ÷ hip breadth 1.48 ± 0.07 men, 1.28 ± 0.07 women. Waist ÷ hip circumference 0.92 ± 0.06 men, 0.84 ± 0.06 women | Feminine ÷ masculine: stature 0.968 (1.694 ÷ 1.750), shoulder joint width 0.855 | Keep the two choices. The feminine frame is tall for a woman (85th percentile of B01), so presets and the randomizer should set `height` per frame |
| Height | `height` 0.85-1.15 (`BodyHeight`) | Stature | 1756 ± 69 (1602-1927) / 1628 ± 64 (1483-1778). NHANES 5th-95th: 1628-1874 / 1498-1725 | Scales 1.31 m of the 1.75 m (spine and legs; not head, neck, arms, hands or feet). Stature 1.55-1.95 m masculine, about 1.50-1.88 m feminine | Keep 0.85-1.15. Show centimetres. Basic view 0.88-1.12 (masculine 1.59-1.91 m). Average woman is `height` 0.95 on the feminine frame (1.63 m) |
| Overall build | None as one control: `muscle` × `fat` | Body mass, girths | BMI 27.7 ± 4.0 (19.5-38.2) / 25.5 ± 3.5 (18.8-34.8). NHANES mean 29.4 / 29.8 | See the estimate table below | **New in the interface only**: a build picker (the presets of section 6) or a two-axis pad over `muscle` and `fat`. No new manifest parameter |
| Muscle definition | `muscle` -1 to 1 changes mass and tone. Definition finer than the mesh's 25 mm quads is not in the geometry (`assets/characters/realistic/README.md:151`) | Flexed biceps 358 ± 35 / 306 ± 31; surface relief | Fat-free mass index of athletes who do not use steroids stops at about 25 (O09) | `muscle` +1 alone: upper arm +8%, waist unchanged (estimate) | Keep `muscle`. **New**: a definition layer (normal map) as a `material_values` driver, strongest at high muscle and low fat. Planned as CO7 |
| Body fat | `fat` -1 to 1 | Waist circumference at the navel | 941 ± 112 (725-1214) / 861 ± 100 (665-1115). NHANES (at the iliac crest) 1029 / 984, 95th 1326 / 1297. Body fat at 25: 24% / 38% (B11) | Masculine waist about 765-1145 mm, feminine 750-1090 mm (estimate), but only with `muscle` moved the other way: `fat` +1 alone gives 894 mm masculine | Keep -1 to 1. The interface must make "soft and heavy" reachable in one move (section 6, Heavyset) |
| Shoulder width | `shoulder_width` 0.85-1.15 (`ShoulderWidth`) | Biacromial breadth (bone); bideltoid breadth also grows with muscle and fat | Biacromial 416 ± 19 (371-463) / 365 ± 18 (322-406). Bideltoid 510 ± 33 / 450 ± 29. Biacromial ÷ stature 0.237 ± 0.010 / 0.224 ± 0.010 | Shoulder joint width 0.296 m ± 13.9% (0.255-0.338) | Basic 0.90-1.10 (about the 1st to 99th percentile). Keep 0.85-1.15 as the hard range |
| Waist width | `waist_width` 0.85-1.15 (`WaistWidth`) | Waist breadth | 326 ± 35 (257-413) / 300 ± 33 (232-385). Depth ÷ breadth 0.73 ± 0.05 / 0.71 ± 0.06 | Spine bone side to side ±15%; depth unchanged | Basic 0.92-1.08. Breadth and depth move together in people (r = 0.87 / 0.84), so larger changes belong to `fat_abdomen` and `fat`, not to this bone scale |
| Hip width | `hip_width` 1.0-1.15 (`HipWidth`) | Hip breadth; bicristal breadth (bone) | Hip breadth 346 ± 24 (295-408) / 354 ± 27 (293-423). Bicristal 275 ± 18 / 273 ± 22 | Widens only. Narrower crosses at the crotch (`README.md:177`) | Keep 1.0-1.15 on the bone. **New**: the narrow half as a shape (MakeHuman `hip-scale-horiz-decr`) with the keep-apart pass, to reach about -10% |
| Chest proportions | Partly: `muscle_chest`, `fat_chest`; `shoulder_width` widens the upper chest | Chest breadth, depth, circumference | Circumference 1059 ± 87 (873-1276) / 947 ± 83 (784-1160). Breadth 289 ± 18 / 269 ± 19. Depth 254 ± 26 / 247 ± 27. Depth ÷ breadth 0.88 / 0.92 | Built chest girth about 1043 / 859 mm (estimate, below) | **New**: `chest_width` (± 12%, two SD) and `chest_depth` (± 20%) from MakeHuman `torso-scale-horiz` and `torso-scale-depth`; bust on the feminine frame from the `breast/` cup targets |
| Leg proportions | `leg_length` 0.85-1.15 (`LegLength`), thigh and shin together | Crotch or hip height ÷ stature | Crotch height ÷ stature 0.482 ± 0.014 (0.450-0.519) / 0.480 ± 0.015. Trochanter ÷ stature 0.513 ± 0.015 / 0.519 ± 0.014. Shin share of the leg 0.520 ± 0.013 | Hip height ÷ stature 0.476-0.546 (base 0.513): -2.4 to +2.1 SD. Also changes stature by ± 0.12 m | Basic 0.90-1.10. Keep 0.85-1.15. A thigh to shin split is low priority (2.5% variation) |
| Arm proportions | `arm_length` 0.85-1.15 (`ArmLength`), upper arm and forearm together | Arm length ÷ stature; span | (Upper arm + forearm) ÷ stature 0.343 ± 0.010 (0.322-0.368) / 0.339 ± 0.011. Span ÷ stature 1.033 ± 0.027 / 1.020 ± 0.029. Forearm ÷ upper arm 0.80 ± 0.03 / 0.78 ± 0.04 | ± 15% of a 0.54 m arm: ± 5 SD | Basic 0.93-1.07. Keep 0.85-1.15. Make it follow `height` (section 7) |
| Head-to-body ratio | `head_size` 0.85-1.15 (`HeadSize`) | Heads tall = stature ÷ chin-to-crown height | Head height median 232 / 218 (5th-95th 218-247 / 204-232), B03. At median stature 7.6 / 7.5 heads. Artists: 7.5 average, 8 ideal, 8.5 heroic (O01). Head circumference varies only 2.8% / 3.5% | Built 7.70 / 7.84 heads. Slider gives 6.8 to 8.9 heads | Basic 0.93-1.07 (7.25 to 8.25 heads). Hard 0.88-1.12. Make it follow `height` weakly (section 7) |

Not in §4.1 but present: `hand_size` (hand length ÷ stature 0.110 ± 0.004, 1st-99th -8% to +10%: basic 0.90-1.10) and `foot_size` (foot length ÷ stature 0.154 ± 0.005 / 0.151 ± 0.005, ± 8%: basic 0.92-1.08).

### Estimated girths at the slider ends

Computed today from the girth changes of the 16 transferred corners (`docs/development/character-studio/co5-spike/measurements.json`) mixed by the manifest's rule (`shapes.py:99-115`, `:123-137`). **Estimates**: the built shapes are also held off bone and kept apart, and the spike measured the source bodies in their own pose and at their own size (1.690 m and 1.639 m tall, `assets/characters/realistic/source/body_masculine.obj` and `body_feminine.obj`), so girths are scaled here to the built heights (by 1.036 and 1.033). Body mass index assumes 1.03 kg per litre of the mesh's volume. Percentiles are read from the ANSUR II mean and SD for that sex on a normal curve; the rank counted in the sample is 1 to 3 points higher up to the median because waist girth is skewed (masculine 856 mm is the 25th, 894 the 36th; feminine 812 mm the 34th). Re-measure on the built body before fixing preset values.

| Sliders (`muscle`, `fat`) | Masculine chest / waist / hips, mm | Waist percentile | Body mass index | Feminine chest / waist / hips, mm | Waist percentile | Body mass index |
|---|---|---|---|---|---|---|
| 0, 0 (as built) | 1043 / 856 / 1019 | 23 | 25.4 | 859 / 812 / 1008 | 31 | 20.9 |
| 0, +1 | 1098 / 894 / 1057 | 34 | 29.0 | 917 / 882 / 1081 | 58 | 23.4 |
| 0, -1 | 991 / 792 / 961 | 9 | 21.8 | 839 / 775 / 954 | 19 | 18.5 |
| +1, 0 | 1047 / 863 / 1007 | 24 | 26.2 | 888 / 789 / 964 | 24 | 20.5 |
| -1, 0 | 1105 / 1003 / 1118 | 71 | 29.6 | 884 / 877 / 1065 | 56 | 22.3 |
| -1, +1 (heavy and soft) | 1195 / 1146 / 1208 | 97 | 36.2 | 1091 / 1089 / 1160 | 99 | 30.8 |
| +1, +1 (heavy and muscular) | 1135 / 886 / 1050 | 31 | 32.6 | 1040 / 792 / 985 | 24 | 25.3 |
| -1, -1 (slight) | 962 / 764 / 919 | 6 | 20.1 | 843 / 749 / 868 | 13 | 16.8 |
| +1, -1 (lean and muscular) | 947 / 820 / 954 | 14 | 21.0 | 858 / 762 / 918 | 16 | 18.6 |

What this says:

- Both base bodies are on the lean side. The masculine waist is near the 23rd percentile of soldiers, with a body mass index about 25 (soldiers' mean 27.7). The feminine body is about 21, about the 8th percentile of women soldiers (mean 25.5).
- `fat` alone is mild: +1 takes the masculine waist to about 894 mm, still under the soldiers' mean of 941. The soft, heavy body needs `muscle` down as well, which the package README says (`README.md:141`) but the interface does not.
- `muscle` +1 with `fat` -1 makes the masculine body smaller than it is built (chest 947 against 1043 mm, upper arm 288 against 358). It reads lean, not athletic.
- The heaviest whole-body setting (waist about 1146 / 1089 mm) is the 97th to 99th percentile of soldiers but only about the 75th of the public (NHANES 75th percentile: 1121 / 1092). Heavier needs `fat_abdomen`, `fat_hips` and `fat_chest`.
- The slight corner on the feminine frame is a body mass index under 17: underweight.

## 4. MakeHuman targets not yet used

Counts are `.target` files at the pinned commit (B07). The transfer in `assets/characters/transfer/` carries any of them; six regional ones were already carried for the spike's sheets.

| Folder | Files | Read so far | What it offers |
|---|---|---|---|
| `macrodetails/` | 348 | 24 (young: 6 ethnic, 18 universal) | 144 height and 108 proportion corners (not needed: bones do this), baby, child and old ages |
| `armslegs/` | 140 | 0 | Per side: upper arm and forearm muscle, fat and three scales; shoulder muscle; thigh and lower leg muscle, fat and scales; `leg-valgus`; hand scale, finger length, diameter and spread; foot scales; `upperlegs-height`, `lowerlegs-height` |
| `torso/` | 18 | 1 (pectoral) | `torso-vshape`, `torso-muscle-dorsi`, `torso-muscle-pectoral`, `torso-scale-depth`, `-horiz`, `-vert` |
| `hip/` | 14 | 0 | `hip-scale-horiz`, `-depth`, `-vert`, `hip-waist-up` and `-down` (where the waist sits) |
| `stomach/` | 8 | 2 | `stomach-tone`, `stomach-pregnant` (belly forward), navel in, out, up, down |
| `buttocks/`, `pelvis/` | 2, 4 | 1 | `buttocks-volume`, `pelvis-tone` |
| `neck/` | 16 | 0 | `neck-scale-horiz`, `-depth`, `-vert`, `neck-double`, `neck-back-scale-depth` |
| `measure/` | 40 | 0 | One pair per tape measure: neck, upper arm, wrist, bust, underbust, waist, hips, thigh, knee, calf, ankle circumference; shoulder distance; upper arm, lower arm, upper leg, lower leg length; nape to waist; waist to hip |
| `bodyshapes/` | 22 | 0 | Whole-figure types (apple, triangle, inverted triangle, rectangle, hourglass, lean column …), CC0 since October 2021 by their headers |
| `breast/` | 228 | 2 | Cup size by firmness by muscle by weight corners; position and spacing |
| `head/` | 27 | 0 | Head scale, shape and fat (shared with the face work) |

No target is named for shoulder slope. The nearest are `measure-shoulder-dist`, `torso-vshape` and the neck scales; a slope control is a bone offset on the collarbone's outer end plus trapezius volume.

Cost to remember: `RealisticParts.glb` went from 10.2 to 20.5 MB when the first 53 shapes were added and is about 36 MB with today's 155, and each garment copies the body's build shapes (`assets/characters/realistic/README.md:128`, `:185`). Add shapes sparingly, and sum left and right targets into one symmetric shape.

## 5. Brief §4.2 regions and their controls

Two to four controls per region, each tied to something that exists or to a target that would drive it. "Existing" parameters are in `Realistic.base.json`; targets are B07.

| Region (brief §4.2) | Control | Driven by | Notes |
|---|---|---|---|
| Head and neck | Head size | `head_size` (existing) | Face shape belongs to the face categories |
| | Neck thickness | New: `neck-scale-horiz` + `neck-scale-depth`, or `measure-neck-circ` | Neck circumference 398 ± 26 / 330 ± 19 mm |
| | Neck length | New: `neck-scale-vert` or `measure-neck-height` | |
| Shoulders | Shoulder width | `shoulder_width` (existing) | Bone |
| | Deltoid volume | `muscle_shoulders` (existing); `upperarm-shoulder-muscle` for a deltoid-only shape | |
| | Shoulder slope | New: bone offset on the shoulder joints (the manifest's `bones` driver) with trapezius volume | No MakeHuman target |
| Chest | Pectoral volume | `muscle_chest` (existing); `torso-muscle-pectoral` | |
| | Chest fat or bust | `fat_chest` (existing); feminine cup size from `breast/` | |
| | Rib cage width | New: `torso-scale-horiz` | Chest breadth SD 6% |
| | Rib cage depth | New: `torso-scale-depth` | Chest depth SD 10% |
| Upper arms | Muscle | `muscle_arms` (existing, whole arm); `upperarm-muscle` to split | Biceps and forearm girth move together (r = 0.88 / 0.85): one control is honest |
| | Fat | `fat_arms` (existing, whole arm); `upperarm-fat` | Fat sits on the back of the upper arm |
| | Arm length | `arm_length` (existing) | Whole arm |
| Forearms | Forearm muscle | New, optional: `lowerarm-muscle` | Offer only as an offset from the upper arm |
| | Wrist thickness | New, optional: `measure-wrist-circ` | Frame size; wrist 176 ± 9 / 155 ± 8 mm |
| Hands | Hand size | `hand_size` (existing) | |
| | Finger length | New: `hand-fingers-length` | |
| | Finger thickness | New: `hand-fingers-diameter` | Finger spread is a pose, not a shape |
| Abdomen | Abdominal fullness | `fat_abdomen` (existing) | |
| | Abdominal muscle | `muscle_abdomen` (existing); `stomach-tone`; fine lines in the definition layer | |
| | Side profile depth | New: `stomach-pregnant` at small weights, named for what it does (belly depth) | Waist depth SD 15% |
| Waist | Waist width | `waist_width` (existing) | Bone |
| | Waist height | New: `hip-waist-up`, `-down` | The "waist transition" of the brief's example |
| | Flank fullness | New: `measure-waist-circ`, or the side share of `fat_hips` and `fat_back` | |
| Back | Lat width | `muscle_back` (existing); `torso-muscle-dorsi` | |
| | V taper | New: `torso-vshape` | Chest ÷ waist girth 1.13 ± 0.07 / 1.11 ± 0.07 |
| | Back fat | `fat_back` (existing) | Posture is a pose, not a shape |
| Hips | Hip width | `hip_width` (existing, widen); `hip-scale-horiz-decr` for narrow | |
| | Glute volume | `muscle_hips` (existing); `buttocks-volume` | |
| | Hip fat | `fat_hips` (existing) | |
| Thighs | Thigh muscle | `muscle_thighs` (existing) | |
| | Thigh fat | `fat_thighs` (existing) | Thigh and buttock girth are one thing in people (r = 0.94) |
| | Leg length | `leg_length` (existing) | A thigh gap is a result, not a control |
| Calves | Calf muscle | `muscle_calves` (existing) | |
| | Calf and ankle fullness | `fat_calves` (existing); `measure-ankle-circ` | |
| | Leg alignment | New, optional: `leg-valgus` | Knock knee to bow leg; advanced only |
| Feet | Foot size | `foot_size` (existing) | |
| | Foot width | New: `foot-scale-horiz` | |

Kept out on purpose: a fat control on shoulders as a main control (the acromion and collarbone are bone; `fat_shoulders` stays an advanced offset), a muscle control on hands, feet, head or waist, and every pose-like control (posture, finger spread, stance).

**Picking a region in the viewport.** The body's region map (`TEXCOORD_1`, `RealisticRegions.png`) has 14 cells of 16 (`assets/characters/realistic/regions.py:30-49`), cut for hiding skin under clothes. It separates head, neck, hips, upper arms, forearms, hands, thighs, shins and feet, but chest, abdomen and back are all `torso` and `waist`, and the shoulders have no cell. The shape regions (`shapes.region_weights`) do split chest, abdomen, back and shoulders. A pick can use the map for limbs and the shape regions (or the hit normal: front or back) on the trunk, without a new map.

## 6. Body presets

Values on existing parameters only. Girths are the estimates of section 3, so treat the numbers as starting points to check against renders and a real measure. "Custom" is a state, not a set of values.

| Preset | Masculine frame | Feminine frame | Reads as | Estimated waist (percentile) |
|---|---|---|---|---|
| Slim | `muscle` -0.3, `fat` -0.6, `shoulder_width` 0.97 | `muscle` -0.2, `fat` -0.5, `height` 0.95 | Small frame, little muscle, little fat | 805 (11) / 797 (26) |
| Average | `muscle` -0.5, `fat` +0.5 | `muscle` -0.4, `fat` +0.4, `height` 0.95 | An ordinary adult who does not train | 901 (36) / 889 (61) |
| Athletic | `muscle` +0.4, `fat` -0.1 | `muscle` +0.6, `fat` -0.1, `height` 0.97 | Trained, not bulky. The masculine base already is this | 853 (22) / 795 (25) |
| Muscular | `muscle` +0.9, `fat` +0.5, `muscle_shoulders` +0.3 | `muscle` +0.9, `fat` +0.4 | Visible mass in arms, chest and thighs | 875 (28) / 795 (25) |
| Heavyset | `muscle` -0.7, `fat` +0.9, `fat_abdomen` +0.4, `waist_width` 1.05 | `muscle` -0.6, `fat` +1.0, `fat_hips` +0.3, `hip_width` 1.05, `height` 0.95 | Soft and heavy | 1008+ (73+) / 1006+ (93+) |
| Lean | `muscle` +0.3, `fat` -0.7 | `muscle` +0.3, `fat` -0.6, `height` 0.97 | Low fat with tone showing: a runner | 817 (13) / 785 (22) |
| Broad | `muscle` +0.3, `fat` +0.4, `shoulder_width` 1.10, `waist_width` 1.05, `hip_width` 1.05 | `muscle` +0.2, `fat` +0.3, `shoulder_width` 1.08, `hip_width` 1.08, `waist_width` 1.04 | Wide frame, solid | 872 (27) / 824 (36) before the width sliders |
| Custom | Shown when the values match no preset | Same | Picking it changes nothing | |

Rules for the presets:

- A body preset writes only the body's build and proportion parameters it lists and resets the other build parameters to their defaults. It never touches `frame`, the face, skin, hair or items.
- Presets are starting values (brief §4.1): moving any slider afterwards switches the picker to Custom and keeps everything.
- Slim differs from Lean by muscle: slim is slight, lean is toned. Broad differs from Muscular by the skeleton: width sliders, not mass.
- The six existing character presets stay. Their athletic values (`muscle` 0.7, `fat` -0.4) are estimated to shrink the masculine chest by about 47 mm and the upper arm by 9%; their heavyset values (`fat` 0.8, `muscle` 0.2) give a masculine waist near 885 mm (31st percentile, under the mean) and a feminine body mass index near 23. Both are worth re-checking in the renders.

## 7. Implausible combinations

The sliders test proves no pair of parameter ends makes the body cross itself (`README.md:177`). That is not the same as plausible. Correlations are ANSUR II, men / women.

| Combination | Why it is implausible | Damp it how |
|---|---|---|
| `muscle` +1 and `fat` +1 with the region sliders up | The heavy and muscular corner is a bodybuilder (`README.md:182`); estimated upper arm 484 mm, over 3 SD above the flexed mean | Randomizer: keep `muscle` + `fat` under about 1.2 when both are positive. Interface: allow, it is a valid extreme |
| `muscle` -1 and `fat` -1 | Very thin: estimated hips at the 1st to 10th percentile; the feminine body near a body mass index of 17 | Randomizer: keep `muscle` + `fat` above about -1.2 |
| A region slider far from its neighbour: `fat_thighs` against `fat_hips`; `fat_abdomen` against `fat_hips`; `muscle_thighs` against `muscle_calves` | Thigh and buttock girth r = 0.94; waist and buttock 0.88 / 0.80; thigh and calf 0.80 / 0.77; biceps and forearm 0.88 / 0.85 | Randomizer: draw one offset per body and small per-region noise (SD 0.2), neighbours within 0.5 of each other |
| A region slider against the whole body: `fat` -1 with `fat_abdomen` +1 | A pot belly on a gaunt body; waist and weight r = 0.89 / 0.84 | Randomizer: region values within ± 0.5. Interface: allow |
| `fat` +1 with `fat_<region>` +1 everywhere | Locally 1.6 times the heavy corner; the keep-apart pass already cuts the masculine shapes to 0.26 at the armpits and thighs (`Realistic.build.json`, `kept_apart`) | A manifest rule would need a new constraint kind (below). Until then the randomizer keeps whole + 0.6 × region under about 1.3 |
| `shoulder_width` 0.85 with `hip_width` 1.15 | Biacromial ÷ hip breadth falls to 0.90 masculine (population 1.21 ± 0.08, 1st percentile 1.03) and 0.78 feminine (1.04 ± 0.08, 1st percentile 0.85) | `follow` or randomizer: `hip_width` - `shoulder_width` at most 0.12 |
| `waist_width` under 0.92 with `fat` above 0 | Waist breadth follows weight (r = 0.87 / 0.81) and depth; a narrow flat waist under a heavy belly | `follow`: `waist_width` rises 0.05 per unit of `fat` unless pinned |
| `head_size` 0.85 with `height` 1.15, or the reverse | 10 heads tall, or 6: outside every canon and the population (estimated 6.8 to 8.4, see overall) | `follow`: `head_size` follows `height` at 0.2. Randomizer: heads tall within 7.0-8.3 |
| `height` 1.15 alone | Arms, hands, feet, shoulders and head stay as built: a tall body with arms 10% short and a small head | `follow` from `height`: `arm_length` 0.8, `hand_size` and `foot_size` 0.65, `shoulder_width` 0.55, `head_size` 0.2 (from the growth exponents in [overall/README.md](overall/README.md)) |
| `height` × `leg_length` both at an end | They multiply on the legs (`crates/anim/src/body_shape.rs:99`): legs 1.32 times, stature 2.09 m, above the tallest soldier measured | Randomizer: keep the product within 0.82-1.18 |
| `arm_length` and `leg_length` at opposite ends | Span ÷ stature leaves its range (0.97 to 1.10): about 1.21 | Randomizer: within 0.10 of each other |
| `hand_size` and `foot_size` at opposite ends | Hand and foot length r = 0.79 / 0.80 | Randomizer: within 0.10 of each other |
| `frame` between 0 and 1 | The manifest allows it and benchmark files use it, but the studio offers two choices | Randomizer: 0 or 1 only |

**What the manifest can and cannot do today.**

- `follow` is the right tool for the proportion couplings, and architecture.md §7 already defines it as an editing rule with pinning. It is not implemented.
- `max_sum` cannot name `muscle`, `fat` or any region slider: validation rejects ranges below 0 (`crates/avatar/src/manifest.rs:532-533`). Resolution itself only sums the positive parts but scales every named value (`crates/avatar/src/resolve.rs:49-62`). Using it here would be a contract change, so it needs a decision, not a quiet edit.
- The statistical rules (neighbours move together, extremes are rare) fit the randomizer better than hard limits: a person may want an extreme; a random character should not land on one.

## 8. Findings for the lead

1. **The feminine frame is tall and slim.** 1.694 m is the 85th percentile of women (B01); its estimated body mass index is about 21, about the 8th percentile. Presets and the randomizer need per-frame values.
2. **`fat` alone does not make a heavy body.** Estimated masculine waist at `fat` +1 is 894 mm, under the soldiers' mean of 941 mm. The heavy, soft body is `fat` +1 with `muscle` -1.
3. **The existing Athletic and Heavyset presets may not read as named** (section 6).
4. **`height` scales three quarters of the body** and leaves arms, hands, feet, shoulders and head alone. Coupling them is a `follow` job.
5. **Proportion ranges of ± 15% are several SD wide** for bone dimensions (arm length ± 5 SD, head ± 4 to 5 SD). A narrower basic range with the full range under Advanced matches brief §3.3.
6. **Measured oddities in the built body** (`docs/development/character-studio/co5/measurements.json`): masculine foot 0.246 m is the 2nd percentile of all men (mean 0.271) and under the 1st for men of 1.74 to 1.76 m (their mean is also 0.271); feminine hand 0.206 m is longer than the masculine 0.196 m and at the 99th percentile for women (mean 0.181). Bideltoid 0.449 / 0.403 m reads narrow against 0.510 / 0.450, but the build measures in the A-pose and the survey with arms hanging, so that one is not comparable.
7. **The QA measure has no girths on the built body** (it has stature, head height, bideltoid and shoulder-joint width, hand and foot length). `assets/characters/transfer/measure.py` has chest, waist, hip, thigh and upper-arm girths for the spike only. Adding waist at the navel, buttock girth, hip breadth and biacromial breadth to the package measure would let presets be set in centimetres and held by a threshold.
8. **OpenStax Anatomy and Physiology 2e is CC BY-NC-SA 4.0**, so it is reference only. Its first-edition figures on Wikimedia Commons are CC BY 4.0 (B15).
9. **The package README's shape counts are stale**: "53 blend shapes" with "4 pose-driven correctives" (`assets/characters/realistic/README.md:128`) against "Six live shapes" (`:155`). The model has 155 shapes on `Body`, six of them correctives.

## 9. Verification

An independent pass on 2026-11-17 re-opened the sources and the repository files behind this folder. Nothing was downloaded into the repository; survey files and reports were read from a scratch directory outside it.

### What was checked

| Check | Result |
|---|---|
| Licence of every entry classed 1 or 2 | Read at source for B01 to B04, B07, B08, B10 to B13, B15, B16, O03, O04, O10, O11, U06 to U11, L06 to L09. Three changed class (below). B09 could not be re-read (HTTP 403 twice). B12's publisher page redirects to a cookie check, so its licence (CC BY 2.0) and figures were read in the article's open-access full text at Europe PMC; O09's abstract was read through the same service |
| MakeHuman targets | `LICENSE.md` section C read at the pinned commit. The CC0 header was read in all 627 files of the twelve body folders, not a sample: 605 say "released as CC0 in september 2020", the 22 in `bodyshapes/` say October 2021. File counts per folder (1,280 in all) match a listing of the commit. No target has "slope" in its name |
| ANSUR II numbers | Every mean, SD, percentile, ratio, correlation and growth exponent in the four files was recomputed from the two public files (4,082 and 1,986 rows). All agree except the three items below. The report prints stature 175.62 ± 6.86 cm and 162.85 ± 6.42 cm, which the files reproduce |
| NHANES numbers | All 56 values of the table in [overall/README.md](overall/README.md), the 75th and 95th percentile waists and the public-domain statement were read in the report |
| Other numbers | Head height (B03: six percentiles, both means), joint ranges (U04, L04: all eleven pairs), body fat (B11, B12), the muscle ceiling (O09), the canons and waist to height (O01) and the latest averages (B05) match their sources |
| Other entries | 20 further rows opened; all exist and show what the row says, except U01 and B22 (below). 3D.sk refused a fetch |
| Repository claims | Every `path:line` in sections 1, 3, 4, 6 and 7 was opened and holds: 28 body parameters, the ranges, 155 shapes on `Body` (48 + 1 + 6 + 3 + 97), no `constraints`, the `max_sum` check, `follow` unused, no randomizer, the six presets, the 36.4 MB model, the joint heights (read from the model's bind matrices), the reach of each proportion slider computed from them (`height` over 1.31 m, `shoulder_width` ± 13.9% at the joints, a 0.54 m arm), the 0.262 least keep-apart factor |
| Estimated girths | The table in section 3 and the waists in section 6 were rebuilt independently from `co5-spike/measurements.json` with the mixing rule: every cell agrees to the millimetre. They are still estimates |
| Index hygiene | All 53 rows have eight filled columns. No commercial source is above class 4. The folder holds four text files and nothing else. No personal details, no local paths, no long quotations |

### Corrections made

| Where | Was | Now | Why |
|---|---|---|---|
| B01, B02, B03 licence | "Class 1" | Unverified as a copyright licence; usable as numbers | "Approved for public release; distribution is unlimited" is a release marking, not CC0 or a public-domain dedication, and both reports name a contractor's staff among the authors. Statistics are facts, so the data use stands. Text, figures and the files themselves are not cleared |
| B01 quotation | "publicly available" | "were made available publicly in 2017" | The page's words |
| B02 address | The defence archive's copy | The copy on the hosting site | The archive refused every fetch (HTTP 403); the cover was read in the hosted copy |
| B03 page | 525-526 | Printed pages 526-527 | Read in the report |
| B09 licence | "Class 1, CC0" | Unverified today | The statement could not be read at source on either day. The repository's licence record still stands |
| B10 | "Not re-read" | Apache-2.0 code, CC0 MakeHuman data, a non-commercial optional part | Read in the repository's README |
| B20 | | Tagline unconfirmed | HTTP 403 |
| B22 | "measurements in centimetres" | Two read-only measures; no unit named | Read on the page |
| Women's stature mean (section 3; overall) | 1629 | 1628 | 1628.47 in the files |
| Percentiles of the estimated girths (section 3, section 6; overall) | "of ANSUR II" | Labelled as normal-curve values, with the counted ranks given | The sample is skewed: counted ranks are 1 to 3 points higher below the median |
| Feminine body mass index rank (section 3, section 8) | "near the 10th percentile" | About the 8th | Counted: 7.9% of women are below 20.9 |
| Masculine foot (section 8; overall) | "3rd percentile for a 1.75 m man" | 2nd percentile of all men; under the 1st for men of 1.74 to 1.76 m | Counted in the files |
| Slight body's hips (section 7) | "2nd to 10th percentile" | 1st to 10th | Counted |
| Span, spread after stature, women (overall) | 2.9% | 2.8% | Recomputed |
| Head height (overall) | Percentiles only | Means and SDs added | Read in the report |
| "Lines (sampled)" (upper, lower) | 6,360, 886, 255, 720, 455, 1,396, 5,586, 2,158, 858 | 6,342, 868, 237, 702, 437, 1,378, 5,568, 2,140, 840, as "Vertices moved" | The counts included each file's 18-line header |
| U01 quotation | "forms the rounded contour of the shoulder" | "the thick muscle that creates the rounded lines of the shoulder" | The page's words; "rounded contour" is said there of the thumb's base |
| U02, L03 | "chapter text was not read" | Read, with one line of content each | Read on 2026-11-17 |
| U10, L07, L09 licence notes | By `LICENSE.md` only | Header read in every file | Above |
| `stomach-pregnant` (upper) | "waist +17 to 18%" | +16.5% feminine, +18.0% masculine | `co5-spike/measurements.json` |

### Still not confirmed at source

- B06 (segment ratios): only the repository's constants and their textbook citation.
- B09 (base meshes): the CC0 statement, as above.
- U05 (shoulder blade rhythm): the page refused a fetch; its figures come from search summaries.
- The swing-phase knee bend of about 60 degrees (lower): O06 gives no figure for it.
- The population range of heads tall (6.8 to 8.4) is an estimate, not a measured distribution.
- B18 to B21 were opened only far enough to confirm they are commercial; none is used above class 4.
