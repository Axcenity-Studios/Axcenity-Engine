# Facial anatomy and facial parameters: reference index

Stage 0 of the Character Studio refinement ([brief](../../refinement/brief.md) §5, §13.2). This folder holds:

- **README.md** (this file): the general index, landmark definitions, the control table for brief §5.2, the missing controls ranked, nine face presets, natural asymmetry, and the sections for brows, cheeks, chin and forehead.
- One README per feature: [eyes](eyes/README.md), [nose](nose/README.md), [jaw](jaw/README.md), [lips](lips/README.md), [ears](ears/README.md).

Nothing was downloaded into the repository. Every description is in our own words. Photographs and figures are links only.

## How this was collected

- Collected on 2026-11-16 by web search, by opening the pages, and by reading open-access full text through the Europe PMC service.
- **Opened** means the page or the paper's text was fetched and read. **Abstract only** means only the abstract was read. **Record only** means only the catalogue record (title, journal, licence flag) was read. **Not opened** means the page refused the fetch; the row is a lead, not evidence.
- The ANSUR II figures were computed from the two public data files (4,082 men, 1,986 women), read in a temporary folder outside the repository.
- The six files hold 63 index rows from 55 distinct sources. Eight rows in the feature files (N-02, N-03, J-02, J-03, L-01, L-02, L-03, EAR-01) repeat a general source to give that feature's own figures.
- "What ±1 moves today" is read from the constants in `assets/characters/realistic/face.py`. It is the largest offset of the shape, not a measured landmark distance. Nobody built or measured the head for this note.

## Licence classes (brief §13.5)

| Class | Meaning | In this folder |
|---|---|---|
| 1 | CC0 or public domain | US-government survey data (F-01, F-04, F-05), the 1918 and 1909 anatomy plates (F-21, F-22, E-08, N-04, J-04, EAR-07), the base meshes already vendored (F-27, on the repository's own record; not confirmed here) |
| 2 | Open, compatible with an Apache-2.0 engine that redistributes it | CC BY papers (attribution required) and two CC BY figures (L-07, EAR-08) |
| 3 | Licensed commercial | None |
| 4 | Reference only: links, never files | CC BY-NC and CC BY-SA sources, paywalled papers, FaceBase, OpenStax, and every artists' reference |
| 5 | Original | None |
| unverified | Licence not confirmed on the source's own page | Marked per row |

A measured mean is a fact and can be quoted from any source with a citation. The class says what may be done with the source's own files, text and figures.

## What the repository already held

Not repeated here: MetaHuman's face tiers, the GNM Head and ICT FaceKit identity spaces, MakeHuman targets, FLAME's licence ([research.md](../../research.md) §B.2–B.3, [notes/body-face-skin-cloth.md](../../notes/body-face-skin-cloth.md)). Blender Studio's Human Base Meshes (CC0) are the head's source ([dependencies.md](../../dependencies.md)). None of those notes held anthropometric data, landmark definitions or asymmetry figures. This folder adds them.

## The parameters that exist

The realistic base has **47** face identity parameters, all from −1 to 1, in nine groups. They come from 16 paired features (a left and a right parameter each, linked by `mirror`) and 15 single ones: 31 controls, 53 shapes. Three paired features (brow height, arch and length) have a shape for each end. Four controls (six parameters) are marked advanced: eye depth and ear height (a pair each), nose to one side and jaw to one side.

Evidence: `assets/characters/realistic/faceparams.py:37-54` (paired), `:57-73` (single), `:80-81` (two-ended), `:106` (range); the shapes are made in `assets/characters/realistic/face.py:1372-1549`. The manifest `assets/characters/realistic/Realistic.base.json` lists 47 parameters in the nine `face` groups.

## General index

| Reference | Source URL | Author or publisher | Date collected (2026-11-16) | Feature | What it demonstrates | Licence or restrictions | Use (visual guidance / data / actual asset) |
|---|---|---|---|---|---|---|---|
| F-01 Head-and-Face Anthropometric Survey of U.S. Respirator Users (journal paper) | https://stacks.cdc.gov/view/cdc/190020 | Zhuang and Bradtmiller, NIOSH; J Occup Environ Hyg 2(11), 2005 | 2026-11-16 | Whole face | 3,997 US workers measured in 2003, weighted to the US population aged 18–66. Means, SDs and percentiles by sex for 18 head and face dimensions. The main source for breadths, nose and lip length. | Class 1 for the figures. The CDC Stacks record is labelled "Public Domain" (confirmed on the record page). The PDF it serves is the journal's typeset copy, which links to the publisher's terms, and one of the two authors worked for a contractor: quote the figures, do not pass on the PDF or its tables as images. Opened. | data |
| F-02 The survey's technical report | https://nap.nationalacademies.org/resource/11815/Anthrotech_report.pdf | Anthrotech for NIOSH, 2004 | 2026-11-16 | Whole face | The same tables, the measurement descriptions, and statistics for both sexes pooled. | unverified (no licence statement read; a contractor's report for a US agency). Opened. | data |
| F-03 NIOSH anthropometric data and digital headforms | https://www.cdc.gov/niosh/data/datasets/rd-10130-2020-0/default.html | NIOSH | 2026-11-16 | Whole face | The survey's raw data sheet and data dictionary, and the ISO headforms built from it. | unverified. Not opened (the page came back empty). | data |
| F-04 Facial anthropometric differences among gender, ethnicity and age groups | https://stacks.cdc.gov/view/cdc/187926 | Zhuang, Landsittel, Benson, Roberge, Shaffer; Ann Occup Hyg 54(4), 2010 | 2026-11-16 | Whole face | The same survey split by sex, four ethnic groups and three age bands. Sex changed 19 of 24 measures; age (45 and over against 18–29) changed 19. | Class 1 for the figures. Record labelled "Public Domain" (confirmed on the record page). The caution of F-01 applies to the journal's own copy. Record and abstract opened. | data |
| F-05 ANSUR II public data and summary report (TR-15/007) | https://www.openlab.psu.edu/ansur2/ | US Army Natick; hosted by a university lab | 2026-11-16 | Head, ears | 93 measures on 6,068 soldiers (2010–2012). Head breadth and length, bizygomatic breadth, interpupillary breadth, menton–sellion length, and the only large data set with ear length, breadth and protrusion. The report also defines gonion, menton, sellion, tragion and zygion. | Class 1 for the data. The report's cover and its form say "Approved for public release; distribution is unlimited"; the data memorandum says "cleared for UNLIMITED PUBLIC RELEASE" (both confirmed). These are release statements, not a named licence, and the hosting page states none. A US Army survey; five of the report's twelve authors worked for a contractor, so quote its figures and put its definitions in our own words, and do not copy its text or drawings. | data |
| F-06 3D Facial Norms database and its technical notes | https://www.facebase.org/resources/human/facial_norms/notes/ | FaceBase Consortium | 2026-11-16 | Whole face | 24 landmarks with definitions and 29 distances from 3D photographs of 2,454 people aged 3–40 of European ancestry. Age- and sex-specific means on the site. | Class 4. The terms of use (https://www.facebase.org/policies/tou/) say copyright stays with the contributing investigators, ask for citation, and say "Do not circulate the original datasets". No CC licence is named. Individual data need an approved request. Opened. | data (cite numbers; never redistribute files) |
| F-07 The 3D Facial Norms Database: Part 1 | https://pmc.ncbi.nlm.nih.gov/articles/PMC4841760 | Weinberg and others; Cleft Palate Craniofac J 53(6), 2016 | 2026-11-16 | Whole face | How F-06 was collected: four US sites, 952 males and 1,502 females. The numbers are in a supplement, not the text. | Class 4: an author manuscript with no open licence (the catalogue lists it as not open access). Opened. | data |
| F-08 The definitions of three-dimensional landmarks on the human face | https://pmc.ncbi.nlm.nih.gov/articles/PMC4832301 | Katina and others; J Anat 228(3), 2016 | 2026-11-16 | Landmarks | The traditional definition of each soft-tissue landmark next to a curvature-based one that does not depend on head pose. The source of most rows in the landmark table below. | Class 2: CC BY ("Creative Commons Attribution License", read on the article). Opened. | data |
| F-09 Inter-ethnic/racial facial variations: a systematic review and Bayesian meta-analysis | https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0134525 | Wen, Wong, Lin, Yin, McGrath; PLoS ONE 10(8), 2015 | 2026-11-16 | Whole face | 38 studies pooled: 11 angles and 18 distances. Group means differ by 8° to 17° in the nasofrontal and nasolabial angles. The reason the nose ranges below are wide. | Class 2: CC BY (read on the article). Opened; the per-group tables are images and were not read. | data |
| F-10 Anthropometric study of three-dimensional facial morphology in Malay adults | https://pmc.ncbi.nlm.nih.gov/articles/PMC5051712 | Othman and others; PLoS ONE 11(10), 2016 | 2026-11-16 | Eyes, nose, lips, profile | 109 adults aged 20–30. 17 distances, three angles, two ratios: lip heights, philtrum, nose tip protrusion, nasolabial angle, with SDs. Landmark definitions in a table. | Class 2: CC BY (read in the article). Opened. | data |
| F-11 Three-dimensional photographic analysis of the face in European adults from southern Spain | https://pmc.ncbi.nlm.nih.gov/articles/PMC6714104 | Menéndez López-Mateos and others; BMC Oral Health 19, 2019 | 2026-11-16 | Jaw, chin, lips | 100 adults. Mandible width, mandibular body length, chin height, lower face height. | Class 2: CC BY 4.0 (read in the article). Opened. Some labels in its table are wrong: see Cautions. | data |
| F-12 Changes in eyebrow position and movement with aging | https://pmc.ncbi.nlm.nih.gov/articles/PMC5300927 | Park, Yun, Son; Arch Plast Surg 44(1), 2017 | 2026-11-16 | Brows | Brow height above the eye at four points along the brow, at rest and in five actions, for ages 20–30 and 50–70. Women only, 20 in each age group (Korea): a small sample. | Class 4: CC BY-NC 4.0 (read in the article). Opened. | data |
| F-13 Facial asymmetry in healthy North American Caucasians | https://doi.org/10.1043/0003-3219(1981)051%3C0070:FAIHNA%3E2.0.CO;2 | Farkas and Cheung; Angle Orthod 51(1), 1981 | 2026-11-16 | Asymmetry | Asymmetry is very common and mild: about 3 mm or 3% between sides, the right side usually larger, most of it in the upper third. The abstract describes the sample as children. | Class 4. Abstract only (the journal page refused the fetch). | data |
| F-14 The effect of sex and age on facial asymmetry in healthy subjects | https://doi.org/10.1053/joms.2001.21872 | Ferrario, Sforza, Ciusa, Dellavia, Tartaglia; J Oral Maxillofac Surg 59, 2001 | 2026-11-16 | Asymmetry | 314 healthy people aged 12–56. No difference by sex or age. Tragion, gonion and zygion are the most asymmetric landmarks; endocanthion the least. | Class 4. Abstract only. | data |
| F-15 Quantitative facial asymmetry: using three-dimensional photogrammetry to measure baseline facial surface symmetry | https://doi.org/10.1097/SCS.0b013e3182a2e99d | Taylor and others; J Craniofac Surg 25(1), 2014 | 2026-11-16 | Asymmetry | 100 volunteers: the face against its mirror image differs by 0.80 ± 0.24 mm (root mean square). The largest difference sat in the middle third in 49%, the lower third in 41%, the upper third in 10%. | Class 4. Abstract only. | data |
| F-16 Three-dimensional assessment of facial asymmetry using dense correspondence | https://doi.org/10.1016/j.ajodo.2019.12.014 | Lum, Goonewardene, Mian, Eastwood; Am J Orthod Dentofacial Orthop 158(1), 2020 | 2026-11-16 | Asymmetry | 279 young adults: over half have an area 2–5 mm off its mirror image; about 7% have one over 5 mm. Most of it lies along one axis, read here as side to side (the abstract calls it the coronal plane, x-axis). | Class 4. Abstract only. | data |
| F-17 Facial asymmetry assessment in adults using three-dimensional surface imaging | https://pmc.ncbi.nlm.nih.gov/articles/PMC4614853 | Patel, Islam, Murray, Goonewardene; Prog Orthod 16, 2015 | 2026-11-16 | Asymmetry | Faces judged symmetrical still differ from their mirror image by 1.2 to 1.5 (root mean square); faces judged asymmetrical by 2.4 to 3.3, most in the lower face. | Class 2: CC BY 4.0 (read on the article). Opened. | data |
| F-18 The effect of sex and age on facial shape directional asymmetry in adults | https://pmc.ncbi.nlm.nih.gov/articles/PMC10399867 | Harnádková and others; PLoS ONE 18(8), 2023 | 2026-11-16 | Asymmetry | A consistent left–right difference is present in both sexes and every age group. Restates the perception thresholds of F-19. | Class 2: CC BY (read in the article). Opened in part. | data |
| F-19 Discriminative thresholds in facial asymmetry: a review of the literature | https://doi.org/10.1093/asj/sjw271 | Wang, Wessels, Hussain, Merten; Aesthet Surg J 37(4), 2017 | 2026-11-16 | Asymmetry | Each feature has its own threshold at which people start to see asymmetry. Eyelid position at rest is the most sensitive, at 2 mm. | Class 4. Abstract only. | data |
| F-20 Limits in the perception of facial symmetry: a prospective study | https://pmc.ncbi.nlm.nih.gov/articles/PMC11595402 | J Pers Med 14(11):1109, MDPI, 2024 | 2026-11-16 | Asymmetry | Collects earlier thresholds: eye closure 2 mm, mouth corner or brows 3 mm, nose 4 mm, a chin under 3 mm goes unnoticed, a drooping lid from about 1.5 mm. | Class 2: CC BY 4.0 (read in the article). Opened in part. | data |
| F-21 Gray's Anatomy (1918) plates: skull from the front, skull from the side, muscles of the head and face | https://commons.wikimedia.org/wiki/File:Gray190.png , https://commons.wikimedia.org/wiki/File:Gray188.png , https://commons.wikimedia.org/wiki/File:Gray378.png | Henry Gray, drawn by H. V. Carter; hosted on Wikimedia Commons | 2026-11-16 | Skull, muscles | The bone under each face region and the muscles that move it. Plate 190 is the front view, 188 the side view. | Class 1: public domain (read from each file's licence field on Commons). | visual guidance; could be shown in docs |
| F-22 Sobotta's atlas (1909), plate 260: superficial facial muscles | https://commons.wikimedia.org/wiki/File:Sobo_1909_260.png | J. Sobotta; hosted on Wikimedia Commons | 2026-11-16 | Muscles | The surface layer of the facial muscles from the side and slightly in front. | Class 1: public domain (licence field on Commons). | visual guidance |
| F-23 Anatomy and Physiology 2e, section 14.1 (eye and ear) and chapter 7 (skull) | https://openstax.org/books/anatomy-and-physiology-2e/pages/14-1-sensory-perception | OpenStax | 2026-11-16 | Eye, ear, skull | Textbook anatomy of the lids, tear apparatus, iris and auricle. | Class 4. The page read on this date says "Creative Commons Attribution-NonCommercial-ShareAlike License" and forbids use in training language models without permission. Opened. | visual guidance |
| F-24 Form of the Head and Neck; Anatomy of Facial Expression | https://anatomy4sculptors.com/ | Anatomy for Sculptors | 2026-11-16 | Whole face | Artists' breakdowns of the head into planes, fat pads and bone landmarks. Good for where a shape should fade out and what stays fixed on bone. | Class 4. The terms forbid reproducing or copying "any portion of the product" without permission (https://anatomy4sculptors.com/terms-and-conditions/, confirmed). | visual guidance |
| F-25 3D.sk photo references | https://www.3d.sk/ | 3D.sk | 2026-11-16 | Whole face | Consistent front, side and three-quarter photographs of many faces under even light. Good for checking a preset against real faces. | Class 4. The licence page refused the fetch; treat every photo as reference only. Not opened. | visual guidance |
| F-26 ArtStation | https://www.artstation.com/ | Individual artists | 2026-11-16 | Whole face | Finished realistic heads and sculpt breakdowns. Good for the quality bar and for how others present feature variation. | Class 4. Each work belongs to its artist. Not opened. | visual guidance |
| F-27 Blender Studio Human Base Meshes | https://developer.blender.org/docs/release_notes/4.0/asset_bundles/ | Blender Studio | 2026-11-16 | Whole head | The bundle the realistic head is already built from. It also has a planar head (the head as flat planes) and a generic head with shape keys for example head shapes: a ready check for the face-shape presets. | Class 1 on the repository's own record only: CC0 1.0 in [dependencies.md](../../dependencies.md). Not confirmed for this index: the download page (https://www.blender.org/download/demo-files/) refused the fetch in both passes, and the release notes name no licence for the bundle (their footer licence covers the notes' own text). A trade news article says CC0. Open the download page by hand before relying on this row. | actual asset (already vendored) |
| F-28 Sketchfab | https://sketchfab.com/ | Individual authors | 2026-11-16 | Whole face | Turnable head scans and anatomy models. Good for a feature seen from any angle. | Class 4 by default. Each model has its own licence (some CC0 or CC BY); read it on the model's page before any use. Licence page not opened. | visual guidance |
| F-29 Racial and sexual differences of eyebrow and eyelid morphology: three-dimensional analysis in young Caucasian and Chinese populations | https://qims.amegroups.org/article/view/132908/html | Gao and others; Quant Imaging Med Surg 15(1):882-897, 2025 | 2026-11-17 | Brows | 3D photographs of 147 adults aged 18 to 30. Caucasian men and women: the brow's lower edge over the inner eye corner 17.4 ± 2.0 and 17.5 ± 2.3 mm, over the outer corner 15.9 ± 2.9 and 18.5 ± 2.9, the lower brow length (a curved path from the first hair to the last) 69.5 ± 5.8 and 61.4 ± 6.1. No thickness figure. Used for the brows (wave 3). | Class 4: CC BY-NC-ND 4.0 (read in the article). Opened. | data |
| F-30 Unlocking nasal angle anthropometry and nasolabial angle preferences: a key to achieve the perfect rhinoplasty | https://dergipark.org.tr/tr/download/article-file/3745343 | Çavdar Yılmaz and others; Anatomy (Turkish Society of Anatomy and Clinical Anatomy), 2023 | 2026-11-17 | Profile | 142 adults aged 18 to 35 (71 women, 71 men), photogrammetry: the nasofrontal angle (glabella, nasion, the nose's dorsum) is 138.5 ± 9.3° in men and 141.6 ± 8.1° in women. Used for the masculine brow ridge (wave 3). | Class 4: no licence stated in the part read. Opened (PDF). | data |

## Landmarks

Soft-tissue landmarks, written in our own words. "Source" says where the definition was checked.

| Landmark | Abbreviation | Where it is | Paired | Source |
|---|---|---|---|---|
| Nasion (soft tissue) | n | Midline point at the root of the nose, over the suture between the frontal and nasal bones. | no | F-06, F-08 |
| Sellion | se | The deepest point of the hollow between forehead and nose. Surveys use it in place of nasion. | no | F-05 |
| Glabella | g | The most forward midline point of the forehead between the brow ridges. | no | F-10 |
| Rhinion | rhi | Midline point on the bridge where the nasal bones end and cartilage begins. A dorsal hump sits here. | no | Standard usage; not confirmed against an opened source |
| Pronasale | prn | The most forward point of the nose tip. | no | F-06, F-08 |
| Subnasale | sn | Midline point where the base of the nose meets the upper lip. | no | F-06, F-08 |
| Alare | al | The most lateral point of each nostril wing. | yes | F-06, F-08 |
| Endocanthion | en | The inner corner of the eye, where the lids meet. | yes | F-06, F-08 |
| Exocanthion | ex | The outer corner of the eye, where the lids meet. | yes | F-06, F-08 |
| Cheilion | ch | The corner of the mouth. | yes | F-06, F-08 |
| Crista philtri | cph | Where each ridge of the philtrum meets the red of the upper lip: the peaks of the Cupid's bow. | yes | F-06, F-10 |
| Labiale superius | ls | Midline point on the upper edge of the red of the upper lip. | no | F-06, F-08 |
| Stomion | sto | Midline point where the closed lips meet. | no | F-06, F-08 |
| Labiale inferius | li | Midline point on the lower edge of the red of the lower lip. | no | F-06, F-08 |
| Sublabiale | sl | The deepest midline point of the groove between lower lip and chin. | no | F-06, F-08 |
| Pogonion (soft tissue) | pg | The most forward midline point of the chin. | no | F-08, F-10 |
| Gnathion | gn | The midline point between the front and the underside of the chin (F-08). F-06 and F-10 put it at the lowest point of the chin, where surveys put menton: say which meaning a measurement uses. | no | F-06, F-08, F-10 |
| Menton | me | The lowest midline point of the chin. | no | F-05 |
| Gonion | go | The most lateral point of the jaw's rear angle. | yes | F-05 |
| Tragion | t | The notch just above the tragus of the ear. | yes | F-05, F-06, F-08 |
| Zygion | zy | The most lateral point of the cheekbone arch. | yes | F-05 |
| Frontotemporale | ft | The most inward point of the temple line on each side of the forehead. | yes | F-05 |

## Population data used below

Means and SDs in millimetres (degrees where marked), adults, by sex. One population each; see Cautions.

| Dimension | Landmarks | Men | Women | Source |
|---|---|---|---|---|
| Bizygomatic breadth | zy–zy | 143.5 ± 6.9 | 135.1 ± 6.5 | F-01 |
| Bizygomatic breadth | zy–zy | 142.6 ± 6.2 | 133.8 ± 5.6 | F-05 |
| Bigonial breadth | go–go | 120.4 ± 10.4 | 110.1 ± 8.9 | F-01 |
| Minimum frontal breadth | ft–ft | 105.5 ± 5.7 | 102.9 ± 5.4 | F-01 |
| Maximum frontal breadth | | 112.3 ± 5.5 | 108.6 ± 5.3 | F-01 |
| Head breadth | | 153.0 ± 6.0 | 146.8 ± 5.6 | F-01 |
| Head length | | 197.3 ± 7.4 | 187.5 ± 7.2 | F-01 |
| Face length | me–se | 122.7 ± 7.0 | 113.4 ± 6.1 | F-01 |
| Interpupillary distance | | 64.5 ± 3.6 | 61.9 ± 3.5 | F-01 |
| Interpupillary breadth | | 64.0 ± 3.4 | 61.7 ± 3.6 | F-05 |
| Nose height | se–sn | 52.0 ± 4.1 | 48.2 ± 3.8 | F-01 |
| Nose breadth | al–al | 36.6 ± 4.1 | 33.2 ± 3.9 | F-01 |
| Nose protrusion | sn–prn | 21.1 ± 2.7 | 19.8 ± 2.7 | F-01 |
| Nasal root breadth | | 16.6 ± 2.3 | 16.3 ± 2.0 | F-01 |
| Lip length (mouth width) | ch–ch | 51.1 ± 4.2 | 48.0 ± 4.0 | F-01 |
| Upper lip length | sn–sto | 22.8 ± 2.1 | 21.3 ± 1.8 | F-10 |
| Philtrum length | sn–ls | 14.2 ± 2.3 | 12.7 ± 2.0 | F-10 |
| Philtrum width | cph–cph | 11.8 ± 1.9 | 10.4 ± 1.1 | F-10 |
| Upper vermilion height | ls–sto | 10.3 ± 1.9 | 10.3 ± 1.6 | F-10 |
| Lower vermilion height | sto–li | 10.2 ± 2.0 | 9.8 ± 1.7 | F-10 |
| Lower face height | sn–gn | 68.7 ± 4.6 | 64.7 ± 4.2 | F-10 |
| Chin height | li–me | 39.1 ± 7.0 | 38.1 ± 3.7 | F-11 |
| Mandibular body length | go–me | 92.6 ± 13.3 | 83.5 ± 7.5 | F-11 |
| Gonial angle (degrees) | | 121.6 ± 6.9 | 125.3 ± 6.6 | J-01 |
| Facial profile angle (degrees) | g–sn–pg | 166.9 ± 5.2 | 169.5 ± 4.6 | F-10 |
| Nasolabial angle (degrees) | | 105.1 ± 9.6 | 101.9 ± 9.3 | F-10 |
| Brow height over the inner eye corner | | — | 26.4 ± 3.1 (aged 20–30, 20 women) | F-12 |
| Brow height over the outer edge of the iris | | — | 31.0 ± 3.1 (aged 20–30, 20 women) | F-12 |
| Ear length | | 64.2 ± 4.5 | 59.4 ± 3.9 | F-05 |
| Ear breadth | | 36.1 ± 3.0 | 33.2 ± 2.8 | F-05 |
| Ear protrusion | | 23.2 ± 3.0 | 20.4 ± 2.7 | F-05 |

Eye figures (fissure width and height, lid margins, eyeball, cornea, eye protrusion, canthal tilt) are in [eyes](eyes/README.md).

## The rule for ranges

- **±2 SD** covers about 95% of people. Use it for presets and the randomizer.
- **±2.5 SD** covers about 99%. A slider's end should land near it.
- Beyond **±3 SD** reads as stylised or as a medical condition. Allow it only where the slider is meant to stylise.
- A single shape can only reach part of a measured distance cleanly. A shape that stretches one region further than about 6 mm starts to distort its neighbours, so several of the ranges below stop short of 2.5 SD on purpose.
- The SD is the within-sex figure. The masculine and feminine frame is a separate body parameter, so a face slider only has to span variation within one frame.

## 1. Brief §5.2 controls against the existing parameters

"a side" means each of the left and right parameters. "new" means no parameter provides it. Ranges are recommendations for Stage 4, not changes made here.

### Eyes

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Eye size | `eye_size_left/right` | Fissure width (en–ex) and the eyeball | Fissure width 25.6 ± 2.0; eyeball 24.2 ± 1.9 across; cornea 11.8, SD about 0.5 (E-01, E-04, E-05) | Eyeball ×1.12, skin round it up to ×1.20; the opening's height is held (`face.py:337-338`, `:1398-1409`) | Keep for the lids: about ±1.5 to 2.5 SD of fissure width. The eyeball scale is about ±1.5 SD of eyeball size, but it scales the iris 12% while real corneas vary about ±4% (1 SD). Consider letting the eyeball follow at half strength. |
| Eye spacing | `eye_spacing_left/right` | en–en, pupil to pupil | Interpupillary 64.5 ± 3.6 M, 61.9 ± 3.5 F; en–en 33.2 ± 3.5 (F-01, E-01) | ±4.2 mm a side, ±8.4 mm between the pupils (`face.py:1396`) | Keep. About ±2.4 SD. Randomizer within ±0.85 (2 SD). |
| Eye depth | `eye_depth_left/right` (advanced) | Eye protrusion from the outer orbital rim | 16.5 M, 16.2 F; SD about 1.9, derived (E-06) | ±4.2 mm (`face.py:1397`) | Keep. About ±2.2 SD. |
| Eye height | `eye_height_left/right` | Eye level on the face; no standard distance | No data found | ±4.5 mm (`face.py:1395`) | Keep. Hold the left–right difference under 2 mm in presets: lid-level asymmetry is seen at 2 mm (F-19). |
| Eye tilt | `eye_tilt_left/right` | Canthal tilt: the angle of the en–ex line | 5.1 ± 2.5° M, 6.1 ± 2.9° F, outer corner higher (E-03, one population) | ±13° (`face.py:1413`) | The full travel is about ±4.5 to 5 SD. Keep the slider for stylised faces; hold presets and the randomizer within ±0.5 (±6.5°, about 2.5 SD). |
| Eye shape | new | Fissure height against width (round against almond) | Fissure height 8.7 ± 1.3 (E-01) | — | New `eye_openness`: ±2.6 mm of fissure height (2 SD). High value. Both rims must still meet in a blink. |
| Upper eyelid | new | Lid fold over the lid (hooded against exposed); upper lid margin to pupil centre | Margin to pupil 3.5 ± 0.8 (E-01); crease height not collected | Lowered brows roll the fold forward (`face.py:300-322`), which hoods the eye as a side effect | New `upper_lid_fold`. High value: it is the largest difference between real eyes that no slider reaches. Margin height within ±1.6 mm (2 SD). |
| Lower eyelid | new | Lower lid margin to pupil centre (straight against rounded) | 5.2 ± 1.0 (E-01) | — | New `lower_lid`: ±2 mm (2 SD). Medium value. |
| Inner and outer eye corners | partly `eye_tilt` (outer) | Endocanthion cover (the fold over the inner corner); exocanthion height | en–en 33.2 ± 3.5 (E-01) | Tilt raises the outer corner | New `inner_corner` (inner fold). Medium to high. The outer corner needs nothing beyond tilt. |
| Eyeball proportions | new | Eyeball and cornea diameter | 24.2 ± 1.9; cornea 10.8 to 12.8 covers 2 SD (E-04, E-05) | — | Not worth a slider. People differ in lid opening, not visibly in eyeball. |
| Iris and pupil appearance | none, by design | Not a shape | Cornea as above | The pupil shape moves 1.3 mm and has no parameter (`faceparams.py:17-22`) | Belongs with the eye material (brief §6), not with face shape. |

### Eyebrows

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Brow height | `brow_height_left/right` | Brow's upper edge above the eye | 26.4 ± 3.1 over the inner corner; 31.0 ± 3.1 over the iris's outer edge (F-12) | Down 5.3 mm with a forward roll; up 5.2 mm (`face.py:323`, `:331`) | Keep. About ±1.7 SD. |
| Brow width | new | Gap between the brows' inner ends | No data found | — | Low value. If wanted, make it a card parameter like brow length. |
| Brow arch | `brow_arch_left/right` | Height of the arch over the brow's ends | The outer measure is 4.7 mm higher than the inner on average (F-12) | ±6.4 mm at 60% along the brow (`face.py:323`, `:331`, `:1459`) | Keep. Height and arch both at −1 bring the brow down 11 mm (`face.py:307-309`), about 3.5 SD: hold their sum to about 8 mm in presets and the randomizer. |
| Brow length | `brow_length_left/right` | Where the brow's tail ends (the card only) | No data found | The tail ends at 0.80 or 1.34 along the brow's line, against 1.08 at rest (`assets/characters/realistic/lashes.py:322-324`) | Keep. |
| Brow thickness | new | Height of the band of hair | No data found (F-29 has none either) | — | High value, but as a card scale or texture choice, not a skin shape. Belongs with grooming (brief §7.3). |
| Brow angle | new | Slope from the brow's inner end to its tail | No data found | — | New `brow_tilt`, about ±5° by eye. Medium to high. Make it a turn about the brow's middle: pushing the inner end against the arch bent the brow into a hook (`face.py:1457-1458`). |
| Brow ridge | `brow_ridge` | Projection at glabella and over the orbits | No data found | ±4.6 mm along the normal (`face.py:1504`) | Keep. |

### Nose

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Nose length | `nose_length` | se–sn | 52.0 ± 4.1 M, 48.2 ± 3.8 F (F-01) | 5.0 mm down and 1.4 forward, the lower nose only (`face.py:1493`) | Keep. About ±1.3 SD. Going further needs a shape that stretches the whole bridge. |
| Bridge width | new | Nasal root breadth | 16.6 ± 2.3 M, 16.3 ± 2.0 F (F-01) | — | New `nose_bridge_width`: ±2.8 mm a side (2.5 SD). High value. |
| Bridge height | `nose_bridge` | Projection of the bridge between the eyes | Nasofrontal angle 137 ± 8° (N-01) | ±4.6 mm forward (`face.py:1499`) | Keep. |
| Bridge profile | new | The bridge line at rhinion: humped against scooped | No data found | — | New `nose_profile`: about ±2 mm at rhinion, by eye. High value in side view. |
| Tip size | new | Width of the tip | No data found | — | New `nose_tip_width`: about ±2.5 mm, by eye. Medium to high. |
| Tip projection | `nose_tip` (coupled with rotation) | sn–prn | 21.1 ± 2.7 M, 19.8 ± 2.7 F (F-01) | 3.0 mm forward together with 3.8 mm up (`face.py:1495`) | Give projection its own control: ±5.4 mm (2 SD). |
| Tip rotation | `nose_tip` (coupled with projection) | Nasolabial angle | 105.4 ± 10.7° (N-01); 105.1 ± 9.6° M (F-10) | The same shape: about 11° on a 20 mm tip, about 1 SD | Give rotation its own control: ±20° (2 SD). High value. Keep `nose_tip` working for saved characters. |
| Nostril width | `nostril_width_left/right` | al–al | 36.6 ± 4.1 M, 33.2 ± 3.9 F (F-01) | ±4.6 mm a side (`face.py:1466`) | Keep. About ±2.3 SD. |
| Nostril height | new | Height of the nostril wing | No data found | — | Not worth a slider. |
| Nostril angle | new | Flare of the wings; the nostril's long axis | No data found | — | Low. Only if direct manipulation needs a handle for it. |
| Overall nose width | `nostril_width_left/right` | al–al | As above | As above | Not a new shape: the existing pair already is the nose's width. Offer one handle that drives bridge width and nostril width together. |

### Cheeks

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Cheekbone prominence | `cheekbone_left/right` | zy–zy | 143.5 ± 6.9 M, 135.1 ± 6.5 F (F-01) | ±4.8 mm along the normal a side (`face.py:1468`) | Keep. About ±1.3 SD of the breadth. |
| Cheek fullness | `cheek_fullness_left/right` | Soft tissue under the cheekbone; no bony landmark | Follows body fat; no data | ±6.0 mm along the normal (`face.py:1470`) | Keep. Tie it to body weight in presets. |
| Midface width | partly `cheekbone` | zy–zy | As cheekbone | — | Medium. Offer one "face width" handle that drives cheekbone, temple width and jaw width, not a new shape. |
| Under-eye fullness | new | The groove between lower lid and cheek | No data found | — | Low to medium as geometry. Mostly an age detail for the skin (brief §6). Defer. |
| Cheek projection | partly `cheekbone` | Forward projection of the cheekbone | No data found | The cheekbone shape moves along the normal, partly forward | Low to medium. Not in the first set. |

### Jaw

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Jaw width | `jaw_width_left/right` | go–go | 120.4 ± 10.4 M, 110.1 ± 8.9 F (F-01) | ±6.2 mm a side, 1.4 mm down (`face.py:1473`) | About ±1.3 SD. The most variable breadth of the face: could reach about ±8 mm a side (1.5 to 1.8 SD). At ±9 mm the average man's jaw would be 0.96 of the average cheekbone breadth, over the 0.95 limit in [jaw](jaw/README.md). |
| Jaw angle | new | Gonial angle | 121.6 ± 6.9° M, 125.3 ± 6.6° F (J-01) | — | New `jaw_angle`: ±13° (2 SD). High value: square against sloped. |
| Jaw depth | new | go–me; the jaw's corner forward or back | 92.6 ± 13.3 M, 83.5 ± 7.5 F (F-11) | — | New `jaw_depth`: ±5 mm. Medium. |
| Jaw definition | new | Soft tissue under the jawline | No data found | — | New `jaw_definition`. High value. Tie it to body fat. |
| Jaw asymmetry | `jaw_shift` (advanced), and each side's `jaw_width` | Menton off the midline | A chin under 3 mm off goes unnoticed; 6 mm is seen (F-19, F-20) | ±5.0 mm (`face.py:353`, `:1532`) | Keep. Randomizer within ±0.3. |

### Chin

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Chin width | `chin_width` | Breadth of the chin; no standard distance | No data found | The chin scaled ±36% across (`face.py:1528`) | Keep. |
| Chin height | `chin_length` | li–me, sn–gn | 39.1 ± 7.0 M, 38.1 ± 3.7 F; lower face 68.7 ± 4.6 M (F-11, F-10) | ±6.2 mm (`face.py:1517`) | Keep. About ±1.4 SD of lower face height. |
| Chin projection | `chin_forward` | Pogonion forward or back; profile angle g–sn–pg | 166.9 ± 5.2° M, 169.5 ± 4.6° F (F-10) | ±5.6 mm, about 5° (`face.py:1527`) | About ±1 SD. The forward end could reach 8 mm. The back end is limited by the lower teeth (`face.py:1518-1525`). |
| Chin shape | new | Pointed against square | No data found | — | New `chin_shape`. Medium. |
| Chin cleft | new | Midline groove | No data found | — | Low. A normal-map detail with a shape of 1 to 2 mm, where supported. |

### Mouth and lips

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Mouth width | `mouth_width_left/right` | ch–ch | 51.1 ± 4.2 M, 48.0 ± 4.0 F (F-01) | ±4.8 mm a corner (`face.py:1474-1475`) | Keep. About ±2.3 SD. |
| Mouth height | `mouth_height` | sn–sto | 22.8 ± 2.1 M, 21.3 ± 1.8 F (F-10) | ±4.5 mm (`face.py:352`, `:1548`) | Keep. About ±2.1 to 2.5 SD. |
| Upper lip fullness | `upper_lip_fullness` | ls–sto | 10.3 ± 1.9 M, 10.3 ± 1.6 F (F-10) | 4.6 mm out and the border 4.0 mm up (`face.py:1540`) | Keep. About ±2 SD. |
| Lower lip fullness | `lower_lip_fullness` | sto–li | 10.2 ± 2.0 M, 9.8 ± 1.7 F (F-10) | 4.6 mm out and the border 4.0 mm down (`face.py:1542`) | Keep. About ±2 SD. |
| Cupid's bow | new | Height of the philtrum peaks over ls; cph–cph | 11.8 ± 1.9 M, 10.4 ± 1.1 F (F-10) | — | New `cupids_bow`: about ±1.5 mm, flat against peaked. Medium to high. |
| Lip corners | new | Cheilion above or below stomion | No data found | — | New `lip_corners`: about ±2 mm. High value. Keep it apart from the smile and frown expression shapes and clamp the sum. |
| Mouth projection | new | Lips forward of the sn–pg line | No data collected | — | New `mouth_projection`: about ±3 mm. High value in side view. The teeth must move with it. |
| Philtrum length | `mouth_height` | sn–ls | 14.2 ± 2.3 M, 12.7 ± 2.0 F (F-10) | Mouth height moves it ±4.5 mm | Not a new slider: mouth height already is philtrum length. Say so in the label's help. |

### Forehead

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Forehead height | `forehead_height` | Hairline to nasion; depends on the hairline | No verified data | 8.0 mm up, crown included (`face.py:1506`) | Keep. |
| Forehead slope | `forehead_slope` (labelled "Forehead fullness") | The forehead's lean | No data found | ±5.5 mm forward at mid-forehead (`face.py:1508`) | Keep. |
| Forehead width | `temple_width` | ft–ft | 105.5 ± 5.7 M, 102.9 ± 5.4 F (F-01) | ±5.2 mm a side (`face.py:1512`) | Not a separate slider. About ±1.9 SD. |
| Temple width | `temple_width` (shape `Face_Head_Width`) | ft–ft; head breadth 153.0 ± 6.0 M | As above | As above | Keep. |
| Brow-to-forehead transition | `brow_ridge` with `forehead_slope` | The step from ridge to forehead | No data found | — | Not worth a slider: the two existing ones make it. |

### Ears

| Brief control | Existing parameter | Landmark distance it changes | Population mean ± SD | What ±1 moves today | Recommended range and reason |
|---|---|---|---|---|---|
| Ear size | `ear_size_left/right` | Ear length and breadth | Length 64.2 ± 4.5 M, 59.4 ± 3.9 F; breadth 36.1 ± 3.0 M, 33.2 ± 2.8 F (F-05) | ±16% about the ear's root, about ±10 mm of length (`face.py:1480`) | Keep. About ±2.3 to 2.6 SD. |
| Ear height | `ear_height_left/right` (advanced) | The ear's level on the head | No data found | ±4.8 mm (`face.py:1481`) | Keep. |
| Ear protrusion | `ear_out_left/right` | Mastoid to the ear's outer edge | 23.2 ± 3.0 M, 20.4 ± 2.7 F (F-05) | 4.8 mm out, up to 7.2 mm at the rim (`face.py:351`, `:1489`) | Keep. About ±1.6 to 2.4 SD. |
| Ear rotation | new | Lean of the ear's long axis | No data collected | — | Low to medium. `ear_tilt` about ±8° by eye, if added. |
| Earlobe size | new | Lobe length; attached against free | 25.6 ± 4.5 M, 23.7 ± 4.5 F (EAR-02) | — | New `earlobe`: about ±5 mm. Medium. |
| Ear shape | new | Outline of the rim | No data found | — | Low for a realistic face. |

### Summary of the table

- 63 controls in brief §5.2.
- 35 are provided by 30 of the 31 existing controls (`nose_tilt` serves asymmetry, brief §5.4, and matches no §5.2 control; some are provided by the same parameter: nostril width is also overall nose width; temple width is also forehead width; mouth height is also philtrum length).
- 3 are partly provided (outer eye corner by tilt; midface width and cheek projection by cheekbone).
- 25 are new.
- The existing ends sit between ±1.2 and ±2.5 SD of the measured distance. Two go further: eye tilt (±13°, about ±4.5 to 5 SD) and brow height with brow arch both lowered (11 mm, about 3.5 SD).

## 2. Missing controls: which are worth a slider

### High value (visibly meaningful; first set for Stage 4)

| Proposed id | Group | Why it matters |
|---|---|---|
| `eye_openness` | Eyes | Round against narrow eyes. No existing slider changes the opening's height. |
| `upper_lid_fold` | Eyes | Hooded, creased or single lid. The largest difference between real eyes that the face cannot make now. |
| `inner_corner` | Eyes | The fold over the inner corner. With tilt and lid fold it gives the range of real eye shapes. |
| `brow_tilt` | Brows | Straight, rising or falling brows change the resting expression more than height does. |
| Brow thickness (card or texture) | Brows | Read at any distance. Not a skin shape. |
| `nose_bridge_width` | Nose | Measured variation of 2 mm SD on a 16 mm width: plain in front view. |
| `nose_profile` | Nose | Humped, straight or scooped. The main difference between noses in side view. |
| `nose_tip_rotation` and tip projection, apart | Nose | One shape now moves both. Upturned and long-projected are different noses. |
| `nose_tip_width` | Nose | Fine against broad tip, apart from nostril width. |
| `jaw_angle` | Jaw | Square against sloped jaw. Needed by the square and angular presets. |
| `jaw_definition` | Jaw | Sharp against soft jawline. Needed by the soft and round presets. |
| `lip_corners` | Lips | Corners up or down change the resting mood. |
| `mouth_projection` | Lips | Lip protrusion in side view. |
| `cupids_bow` | Lips | Flat against peaked upper lip. |

### Medium value (second set)

`lower_lid`, `chin_shape`, `jaw_depth`, `earlobe`, and two compound handles that drive existing parameters together: face width (cheekbone, temple width, jaw width) and face length (forehead height, nose length, chin length).

### Not worth a slider

| Brief control | Reason |
|---|---|
| Eyeball proportions | Real eyeballs and corneas vary by a few percent. The lids make an eye look large or small. |
| Iris and pupil appearance | A material matter, not a shape (brief §6). |
| Brow width | Small effect; the card's length and the grooming choice cover it. |
| Nostril height, nostril angle | Small in every view at game distance. |
| Overall nose width | Already the nostril width pair. |
| Under-eye fullness | An age detail for skin maps. |
| Cheek projection | Mostly covered by cheekbone. |
| Chin cleft | A skin detail; a small shape only where supported. |
| Philtrum length | Already mouth height. |
| Forehead width | Already temple width. |
| Brow-to-forehead transition | Already brow ridge with forehead slope. |
| Ear rotation, ear shape | Ears are seen least; size, height and protrusion carry them. |

## 3. Nine face presets

The shipped presets (`assets/characters/realistic/presets.py`) started from this table and were strengthened in wave 3 (every value still at most 0.7) until each changes at least 3% of the face's pixels against the default face in front and in profile on both frames (`player`'s `face_preset_tests`); the table below is the first guess.

These are **visual starting points**. They are not types of people, and no real face belongs to one. Each is a direction on the existing parameters: a sign and a rough strength on the −1 to 1 scale. A paired name means both sides. Strengths are a first guess to be tuned by eye in the studio against photo references. Parameters not listed stay at 0.

| Parameter | Round | Oval | Square | Heart-shaped | Long | Broad | Narrow | Soft features | Angular features |
|---|---|---|---|---|---|---|---|---|---|
| `forehead_height` | −0.2 | +0.2 | 0 | +0.2 | +0.5 | −0.2 | +0.1 | 0 | 0 |
| `forehead_slope` | +0.2 | 0 | 0 | +0.1 | 0 | 0 | 0 | +0.3 | −0.2 |
| `temple_width` | +0.3 | −0.1 | +0.3 | +0.5 | −0.3 | +0.6 | −0.6 | 0 | 0 |
| `brow_ridge` | −0.2 | 0 | +0.2 | 0 | 0 | 0 | 0 | −0.5 | +0.4 |
| `cheekbone` | −0.2 | +0.2 | 0 | +0.5 | −0.2 | +0.6 | −0.4 | −0.2 | +0.6 |
| `cheek_fullness` | +0.6 | −0.1 | −0.2 | −0.2 | −0.3 | +0.2 | −0.3 | +0.4 | −0.6 |
| `jaw_width` | +0.3 | −0.3 | +0.7 | −0.6 | −0.3 | +0.5 | −0.5 | −0.2 | +0.3 |
| `chin_width` | +0.4 | −0.2 | +0.5 | −0.6 | 0 | +0.3 | −0.3 | +0.1 | −0.2 |
| `chin_length` | −0.4 | +0.1 | −0.1 | +0.2 | +0.6 | −0.2 | +0.1 | −0.1 | +0.1 |
| `chin_forward` | −0.1 | 0 | +0.1 | 0 | 0 | 0 | 0 | −0.2 | +0.3 |
| `nose_length` | −0.1 | 0 | 0 | 0 | +0.4 | −0.1 | +0.1 | −0.1 | +0.1 |
| `nose_bridge` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | −0.3 | +0.4 |
| `nose_tip` | 0 | 0 | 0 | 0 | 0 | 0 | 0 | +0.2 | 0 |
| `nostril_width` | 0 | 0 | 0 | 0 | −0.1 | +0.2 | −0.2 | 0 | 0 |
| `eye_spacing` | 0 | 0 | 0 | 0 | 0 | +0.3 | −0.3 | 0 | 0 |
| `mouth_width` | 0 | 0 | +0.1 | −0.1 | 0 | +0.3 | −0.2 | 0 | 0 |
| `upper_lip_fullness`, `lower_lip_fullness` | +0.1 | 0 | 0 | 0 | 0 | 0 | 0 | +0.3 | −0.2 |

What each one is, in a sentence:

| Preset | The look |
|---|---|
| Round | Width and length close to equal, full cheeks, a short wide chin, no corners. |
| Oval | Close to the base head: slightly longer than wide, widest at the cheekbones, tapering gently to the chin. |
| Square | Jaw as wide as the forehead, a wide flat chin, straight sides. |
| Heart-shaped | Wide forehead and cheekbones, a narrow jaw and a small chin. |
| Long | Tall forehead, long nose and chin, narrow across. |
| Broad | Wide across the temples, cheekbones and jaw, features set wider, a little shorter. |
| Narrow | The reverse of broad. |
| Soft features | Smooth transitions: low brow ridge, full cheeks and lips, a low bridge, a rounded chin. |
| Angular features | Strong planes: high cheekbones over hollow cheeks, a defined ridge, bridge and chin. |

With the proposed parameters, add: `jaw_angle` (+ for square and angular, − for round, heart and soft), `jaw_definition` (+ for angular and long, − for round and soft), `nose_profile` (a slight hump for angular, slight scoop for soft). The presets work without them.

Rules for preset data:

- A preset sets face parameters only. It does not touch the frame, the body, the skin or the hair.
- A preset sets both sides of a pair to the same value. Asymmetry is a separate, later step.
- No preset value above 0.7: a preset is a start, and the user needs room to go further.
- Presets are lists of parameter id and value, as the body presets are. Those are Rust constants today (`crates/editor/src/character_workspace.rs:144-190`), not data files, and using one replaces the whole character (`crates/editor/src/character_workspace.rs:551-562`). Brief §5.1 asks for data-driven presets, so the face presets need a data file and a face-only way to apply them first. A test checks every id against the shipped manifest.

## 4. Natural asymmetry

### What the literature says

| Finding | Figure | Source |
|---|---|---|
| Asymmetry is the rule in healthy faces, and it is mild | About 3 mm, or 3%, between paired measurements; the right side usually larger; most in the upper third | F-13 (abstract; the sample was children) |
| Whole face against its mirror image | 0.80 ± 0.24 mm root mean square | F-15 (abstract) |
| Faces judged symmetrical | 1.2 to 1.5 root mean square; 2.6 is at the 99.8th percentile of that group | F-17 |
| Faces judged asymmetrical | 2.4 to 3.3 root mean square, most in the lower face | F-17 |
| Share of young adults with some area 2 to 5 mm off | 52% of women, 58% of men | F-16 (abstract) |
| Share with some area over 5 mm off | About 7% | F-16 (abstract) |
| Direction | Most along one axis, read here as side to side. The abstract does not say plainly which of the other two axes is up and down | F-16 (abstract) |
| Most and least asymmetric landmarks | Tragion, gonion and zygion most; endocanthion least (less than half as much) | F-14 (abstract) |
| Sex and age | No difference in amount by sex or age in adults | F-14 (abstract) |
| Left and right eye openings | Mean difference 0.0 to 0.1 mm | E-01 |
| Left and right gonial angle | Mean difference under 1° | J-01 |

### What people notice

| Feature | Asymmetry at which most people see it | Source |
|---|---|---|
| A drooping upper lid | About 1.5 mm | F-20 |
| Eyelid position at rest | 2 mm | F-19, F-20 |
| Mouth corner | 3 mm | F-20 |
| Brow position | 3 mm | F-20 |
| Nose | 4 mm | F-20 |
| Chin | Under 3 mm unnoticed; clearly seen from 4 mm in one study, by most people at 6 mm in another | F-19, F-20 |

The 2 mm (eye closure) and 3 mm (smile) figures come from a study that altered a face making those expressions, as F-20 retells it; they are not all measured on a face at rest.

So the eyes tolerate the least and the outline of the face the most. This matches where real faces differ most (ears, jaw corners, cheekbones).

### Magnitudes for the randomizer and the benchmark face

Values are the difference between the left and right parameter (or the value of a single off-centre parameter), on the −1 to 1 scale, with the millimetres that follow from the shape sizes in `face.py`.

| Parameter | mm per unit | Natural asymmetry: stay within | Benchmark "asymmetry" face | Reason |
|---|---|---|---|---|
| `eye_height` left against right | 4.5 | 0.15 (0.7 mm) | 0.35 (1.6 mm) | Lid-level difference is seen at 2 mm. Stay under it. |
| `eye_size` left against right | about 12% | 0.10 | 0.25 | Real eye openings match within 0.1 mm on average. |
| `eye_tilt` left against right | 13° | 0.10 (1.3°) | 0.25 (3°) | The inner corner is the least asymmetric landmark. |
| `brow_height`, `brow_arch` left against right | 5.3, 6.4 | 0.25 (1.3 mm) | 0.55 (2.9 mm) | Brows are seen at 3 mm. One higher brow is the commonest visible asymmetry. |
| `nostril_width` left against right | 4.6 | 0.20 (0.9 mm) | 0.45 (2.1 mm) | The nose is seen at 4 mm. |
| `nose_tilt` | 5.2 | 0.25 (1.3 mm) | 0.60 (3.1 mm) | As above. |
| `cheekbone` left against right | 4.8 | 0.30 (1.4 mm) | 0.60 (2.9 mm) | Zygion is among the most asymmetric landmarks. |
| `jaw_width` left against right | 6.2 | 0.30 (1.9 mm) | 0.60 (3.7 mm) | Gonion is among the most asymmetric landmarks. |
| `jaw_shift` | 5.0 | 0.30 (1.5 mm) | 0.70 (3.5 mm) | A chin under 3 mm off goes unnoticed; the benchmark should just be seen. |
| `mouth_width` left against right | 4.8 | 0.20 (1.0 mm) | 0.45 (2.2 mm) | The mouth corner is seen at 3 mm. |
| `ear_height`, `ear_out`, `ear_size` left against right | 4.8 | 0.35 (1.7 mm) | 0.70 (3.4 mm) | The ear region is the most asymmetric and the two ears are rarely seen together. |

Rules that keep a random face believable:

- Symmetry stays the default (brief §5.4). The randomizer adds asymmetry only when asked, or as a small "natural" amount that stays inside the third column.
- Draw each difference from a bell curve with the column's value at 2 SD, so most faces get about half of it.
- Move the lower face as one: give `jaw_shift`, `nose_tilt` and the mouth corners the same side more often than not. A chin to one side with a nose to the other reads as broken.
- Keep the eyes the most symmetric and the ears and jaw the least.
- Check the whole face: the surface against its mirror image should differ by about 0.6 to 1.0 mm (root mean square) for a natural face and no more than about 1.5 for the benchmark. Above 2.5 no longer reads as a healthy face. These limits join two studies whose methods differ (healthy volunteers average 0.80 mm in F-15; faces judged symmetrical score 1.2 to 1.5 in F-17), so they are not on one scale. Treat them as a first guess and set them again once the check runs on our own head.
- The benchmark "asymmetry" face uses the fourth column on every row at once, all to the same side for the lower face. It should look like one particular person, not like a fault.

## Brows

**Anatomy.** The brow is hair-bearing skin over the upper rim of the orbit, on a pad of fat that slides over the bone. Its inner end sits about over the inner eye corner, its peak over the outer edge of the iris or just beyond, and its tail ends past the outer eye corner. The frontalis muscle raises it; the muscles round the eye and between the brows lower it. Under it the bone forms the brow ridge, strongest at glabella and over the inner orbit.

**Shapes that vary.** Height above the eye; how arched or straight; the slope from inner end to tail; thickness and density; length of the tail; the gap between the brows; how strongly the bone ridge under them stands out. Height falls with age at the outer end first (F-12: 31.0 mm at ages 20–30 against 27.0 mm at 50–70 over the iris's outer edge).

**By sex** (F-29, young Caucasian adults): the lower edge stands about as high over the inner eye corner in men and women (17.4 and 17.5 mm) and 2.6 mm higher over the outer corner in women (18.5 against 15.9), so women's brows rise more toward the tail; women's are shorter (61.4 against 69.5 mm along the lower edge). The realistic brows follow this since wave 3 (`lashes.BROW_LOOKS`; `avatar`'s `realistic_face_second_pass`).

**Sensible limits.** Height ±5 mm (about 1.7 SD). Height and arch together no more than about 8 mm down, or the brow reaches the lashes. Left against right within 1.3 mm for a natural face. Brow ridge ±4 to 5 mm; more reads as a different skull.

References: F-12 (data), F-21 plate 378 and F-22 (muscles), F-24 (planes and fat pads), F-25 (photographs).

## Cheeks

**Anatomy.** The cheekbone (zygomatic bone and arch) is the widest bony point of the face; zygion is its most lateral point. Below and in front of it lie fat compartments over the chewing muscle and the hollow of the cheek. The bone sets the width and the highlight; the fat sets how full or hollow the cheek is, and that follows body fat and age.

**Shapes that vary.** Width at the cheekbones (SD 6 to 7 mm); how far forward the cheekbone stands; how high it sits; fullness or hollowness under it; the fold from nose to mouth corner; fullness under the eye.

**Sensible limits.** Cheekbone ±5 mm a side keeps the breadth within about 1.5 SD. Fullness ±6 mm, more only with a heavier body. Never move the bone and the fat by the same shape: a hollow cheek keeps its cheekbone.

References: F-01 and F-05 (bizygomatic breadth), F-21 plates 188 and 190 (bone), F-24, F-25.

## Chin

**Anatomy.** The chin is the front of the mandible below the lower lip, capped by a pad of soft tissue. Pogonion is its most forward point, menton its lowest, gnathion between them. The groove above it (sublabiale) separates it from the lower lip. The bone's lower border may be one rounded point or two bumps, which makes a square chin or a cleft.

**Shapes that vary.** Height from lip to menton (38 to 39 mm, SD 4 to 7); forward projection (the profile angle varies with an SD near 5°); width; pointed, round or square outline; the depth of the groove above; a cleft or dimple.

**Sensible limits.** Height ±6 mm. Projection ±6 mm, to 8 mm forward at most; back is limited by the lower teeth. Width about ±35% of the chin's own breadth. A chin off centre by under 3 mm goes unnoticed.

References: F-10, F-11 (heights and profile angle), F-05 (menton), J-04 (mandible plate), F-24.

## Forehead

**Anatomy.** The forehead is the frontal bone from the brow ridges to the hairline, under thin skin and the flat frontalis muscle. It narrows at the temples where the temporal lines turn back (frontotemporale). Its lean and its fullness come from the bone; the hairline sets its apparent height.

**Shapes that vary.** Height to the hairline; width across the temples (minimum frontal breadth, SD 5 to 6 mm); how upright or sloped it is; rounded (bossed) or flat; the step from brow ridge to forehead, stronger on the masculine frame.

**Sensible limits.** Width ±5 mm a side (about 1.9 SD). Slope ±5 to 6 mm at mid-forehead. Height ±8 mm; the hairstyle moves the apparent height far more, so check with a hairline, not on a bald head.

References: F-01 (frontal breadths), F-21 plates 188 and 190, F-24.

## Cautions

- **The parameter count is 47, not 45.** The manifest and `faceparams.py` agree: 16 pairs and 15 singles.
- **Each figure is one population.** F-01 is US workers of every ethnic group, weighted; F-05 is soldiers; F-10 is Malay adults; F-11 is southern Spanish adults; F-12 (women only, 20 in each age group) and EAR-02 are Korean; E-03 is South Indian. Use the SD to size a range, not the mean to judge the base head. Group means differ by more than 1 SD on some measures (F-04, F-09).
- **Methods differ.** Calipers on the skin (F-01, F-05), 3D photographs (F-06, F-10, F-11) and CT (EAR-02, J-01, E-04) give different numbers for the same name. Fissure height is 8.7 mm in E-01 and 11.3 mm in E-03.
- **F-11 has mislabelled rows.** Its "nasolabial angle" is the angle at nasion (28°), and its "facial width" of 114 mm is far below the 135 to 144 mm of the caliper surveys. Only its mandible and chin rows are used here.
- **Abstract-only sources** (F-13 to F-16, F-19, EAR-03, EAR-04) give the headline, not the tables.
- **The eye-protrusion SD is derived** from "mean plus two SD is 20.1 mm" in E-06.
- **Rhinion** was not confirmed against an opened source.
- **OpenStax is no longer a free-to-reuse source.** The page read on 2026-11-16 states a non-commercial share-alike licence and a restriction on use with language models.
- **FaceBase data may not be passed on.** Quote a number with a citation; never store its files in the repository.
- **Nothing here was measured on the built head.** Before any range changes, measure al–al, ch–ch, en–en, zy–zy, go–go, se–sn and sn–prn on the realistic head at 0 and at each end, and compare with the table.
- **No photographs for each feature yet.** Brief §13.2 asks for front, side, three-quarter and close-up photographs of every feature. This folder holds data papers and drawn plates. The only photograph sources are the general ones (F-25, F-26, F-28) and one eye close-up (E-09). The photograph sets are still to collect.
- **The brow figures rest on 20 women.** F-12 is the only source for brow height and its SD.
- **Pages that refused the fetch** on this date: the Blender download page, 3D.sk, the Sketchfab help pages, PubMed, the NCBI Bookshelf, and one journal site. Their rows say "not opened".

## Verification

A second pass on 2026-11-16 and 2026-11-17 checked these six files against the sources and the code. It changed the files in place. Files fetched for reading (two survey data files, two reports, the open text of 23 papers) went to a temporary folder outside the repository.

### How it was checked

| Check | Method | Result |
|---|---|---|
| Licence of every class 1 and class 2 row | The licence statement inside each paper's own open text (23 papers); each Commons file's licence field (13 files); the two CDC Stacks record pages; the ANSUR II report's cover and form and the data memorandum; the FaceBase terms; the OpenStax page footer | All confirmed as written, except the rows under "Corrections made" |
| Class 4 rows | Same sources | E-05, L-06 (CC BY-NC-ND), F-12, EAR-02 (CC BY-NC), EAR-05 (CC BY-NC-SA), E-09 (CC BY-SA 4.0), F-23 (CC BY-NC-SA 4.0 and a bar on use with language models) all confirmed. No commercial or artists' source is above class 4 |
| Numbers | Read from the source's own table | Confirmed: every F-01 row of the population table (men from Table VI, women from Table VII); every F-10 row; F-11 chin, mandible, lip and width rows; F-12 brow heights; J-01 gonial angle; N-01 angles and ratios; E-01 lid and fissure figures; E-02 age change; E-03 tilt and fissure height with SDs; E-04 eyeball; EAR-02 lobe and protrusion; F-17 mirror-difference values |
| ANSUR II figures | Computed again from the two public data files (4,082 men, 1,986 women) | Ear length, breadth and protrusion, bizygomatic breadth and interpupillary breadth all match |
| Abstract-only rows | The abstract in the catalogue | F-13 to F-16, F-19, E-05, E-06, EAR-03 and EAR-04 match their abstracts, with the corrections below |
| Code claims | `assets/characters/realistic/faceparams.py`, `face.py`, `lashes.py`, `Realistic.base.json`, `crates/editor/src/character_workspace.rs` | Every `face.py` constant and line cited is right. 47 parameters, 16 pairs, 15 singles, 53 shapes, nine groups, all −1 to 1: confirmed. The 63 controls of brief §5.2 and the 35, 3 and 25 split: confirmed |
| Index hygiene | A script over the six tables | 63 rows, eight columns each, none empty; no file was downloaded into the folder; no personal details or local paths |

### Corrections made

| Where | Was | Now | Evidence |
|---|---|---|---|
| "The parameters that exist" | Four parameters marked advanced | Four controls, six parameters | The manifest marks `eye_depth_left`, `eye_depth_right`, `ear_height_left`, `ear_height_right`, `nose_tilt`, `jaw_shift` |
| F-01, F-04 | Class 1 | Class 1 for the figures only | The record's badge reads "Public Domain". The PDF served for F-01 is the journal's typeset copy and links to the publisher's terms; one of its two authors worked for a contractor |
| F-05 | "A US Army work" | Class 1 for the data; a release statement, not a named licence | Five of the report's twelve authors are marked as contractor staff on its cover; the hosting page states no licence |
| F-07 | unverified | Class 4 | An author manuscript; the catalogue lists it as not open access |
| F-10 | Two angles | Three angles and two ratios | The paper's measurement table |
| F-12 and the two brow rows of the population table | "Korean adults", "sexes pooled" | Women only, 20 in each age group | The paper's methods and figure captions |
| F-13 | Healthy faces | The abstract describes the sample as children | The abstract |
| F-16 and the asymmetry table | "Side to side, then up and down, least front to back" | Only the first axis is stated; the order of the other two is not plain in the abstract | The abstract names planes and axes without saying which is vertical |
| "What people notice" | Chin seen at about 6 mm | 4 mm in one study, 6 mm in another; the 2 mm and 3 mm figures come from altered expressions | F-20, introduction |
| Whole-face check | 0.6 to 1.0, 1.5 and 2.5 given as one scale | Marked as a first guess: F-15 and F-17 use different methods | F-15 abstract, F-17 Table 1 |
| F-24 | "terms page opened" | The terms page's address added | https://anatomy4sculptors.com/terms-and-conditions/ |
| F-27 | Class 1: CC0 | Class 1 on the repository's own record only; not confirmed here | The download page refused the fetch in both passes; neither release-notes page names a licence for the bundle |
| Landmarks: gnathion | One meaning | Two meanings, by source | F-08 (front and underside), F-06 and F-10 (lowest point) |
| Landmarks: frontotemporale | "Standard usage" | F-05 | The ANSUR II report defines it |
| Brow length row | "0.80 or 1.34 of its length", `lashes.py:323-324` | 0.80 or 1.34 along the brow's line against 1.08 at rest, `lashes.py:322-324` | `assets/characters/realistic/lashes.py:322-324` |
| Jaw width (this file and jaw/README.md) | Could reach ±9 mm a side | About ±8 mm a side | (120.4 + 18) / 143.5 = 0.96, over the folder's own 0.95 limit; with 16 it is 0.95 |
| Summary of the control table | 35 provided by the 31 existing controls | By 30 of the 31 | `nose_tilt` matches no control of brief §5.2 |
| Preset rules | "Plain data … the same form as the body presets" | Body presets are Rust constants, and using one replaces the whole character | `crates/editor/src/character_workspace.rs:144-190`, `:551-562` |
| N-01 (nose/README.md) | Differs from the "ideal" on three of five | Four of five | The paper's first results table: four rows under 0.001, one at 0.70 |
| E-05 (eyes/README.md) | "Adults" | Adults aged 40 to 64 | The abstract |
| E-03 (eyes/README.md) | unverified | unverified, treat as class 4 | No licence on the page; not listed as open access |
| Eyes, anatomy paragraph | "About a third of its front showing" | The fraction removed | No source gave it |
| EAR-02 (ears/README.md) | Figures without a side | Marked as right-ear figures | The paper's table gives left and right |
| E-05, E-06, E-07, N-05, J-05, L-04, L-05, L-06, EAR-06 | Licence "from the catalogue record" | Licence confirmed in the article's own text (EAR-06 is CC BY 2.0) | Each paper's licence statement |
| Cautions | — | Two added: no per-feature photographs yet (brief §13.2); the brow figures rest on 20 women | — |

### Not confirmed in either pass

- F-27: the CC0 status of the Human Base Meshes on the publisher's own page.
- F-03, F-25, F-26, F-28, E-10: the pages refused the fetch or came back empty. They stay as leads.
- F-02: the cover and contents pages carry no licence or distribution statement. It stays unverified.
- F-07: the count by sex (952 and 1,502) is not in the abstract and was not checked.
- The rhinion definition.
- The tables behind the abstract-only rows.
