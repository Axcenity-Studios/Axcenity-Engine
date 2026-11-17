# Jaw: reference index

Part of the [face reference index](../README.md) (brief §5.2 "Jaw", §13.2 "Jawlines"). Row ids starting F- are in that file. The chin has its section in that file. Collected 2026-11-16. Links only; nothing downloaded.

## Index

| Reference | Source URL | Author or publisher | Date collected (2026-11-16) | Feature | What it demonstrates | Licence or restrictions | Use (visual guidance / data / actual asset) |
|---|---|---|---|---|---|---|---|
| J-01 Comparison of the gonial angle with age and gender using cone-beam computed tomography images | https://pmc.ncbi.nlm.nih.gov/articles/PMC9191628 | Bakan and others; Cureus 14(5), 2022 | 2026-11-16 | Jaw angle | 235 adults: the jaw's angle is 121.6 ± 6.9° in men and 125.3 ± 6.6° in women (right side); left and right differ by under 1° on average; it changes little with age in adults. | Class 2: CC BY (read in the article). Opened. | data |
| J-02 Jaw rows of the US respirator survey | https://stacks.cdc.gov/view/cdc/190020 | Zhuang and Bradtmiller, NIOSH, 2005 (F-01) | 2026-11-16 | Jaw width, face length | Jaw breadth at the corners 120.4 ± 10.4 mm (men), 110.1 ± 8.9 (women), from 88 to 160 mm across the sample: the most variable breadth of the face. Face length 122.7 ± 7.0 and 113.4 ± 6.1. | Class 1: record labelled "Public Domain". Opened. | data |
| J-03 Jaw rows of the southern Spain 3D study | https://pmc.ncbi.nlm.nih.gov/articles/PMC6714104 | Menéndez López-Mateos and others, 2019 (F-11) | 2026-11-16 | Jaw length | Jaw corner to chin 92.6 ± 13.3 mm (men), 83.5 ± 7.5 (women), each side given; jaw width by 3D photograph 113.5 ± 6.2 and 107.6 ± 7.5. | Class 2: CC BY 4.0. Opened. | data |
| J-04 Gray's Anatomy (1918) plate 176: the mandible from the side | https://commons.wikimedia.org/wiki/File:Gray176.png | Henry Gray, drawn by H. V. Carter; Wikimedia Commons | 2026-11-16 | Bone | The jaw's parts: the body, the upright branch, the angle between them, the chin's front. | Class 1: public domain (the file's licence field on Commons). | visual guidance; could be shown in docs |
| J-05 A morphometric evaluation of the mandibular condyle, coronoid process and gonial angle: age and gender differences | https://pmc.ncbi.nlm.nih.gov/articles/PMC12191486 | Dogan and others; Diagnostics 15(12), 2025 | 2026-11-16 | Jaw angle | A second sample for the jaw's angle by age and sex. A lead for Stage 4. | Class 2: CC BY 4.0 (licence statement confirmed in the article). Text not read. | data |

Also used: F-05 (definitions of gonion and menton), F-14 (gonion is among the most asymmetric landmarks), F-17 (asymmetry concentrates in the lower face), F-19 and F-20 (a chin under 3 mm off centre goes unnoticed; about 6 mm is seen), F-21 plate 378 (the chewing muscle over the jaw's corner), F-24 and F-25 (artists' breakdowns and photographs).

## Anatomy

The lower jaw (mandible) is one bone: a horizontal body carrying the teeth and the chin, and an upright branch on each side that hinges under the cheekbone arch in front of the ear. The corner where body and branch meet is the angle of the jaw; gonion is its most lateral point. The large chewing muscle lies over the outside of the corner and adds bulk there, more so in people who clench. The jawline seen in a face is the lower border of the bone from corner to chin, softened by the fat under it and the skin of the neck. With weight or age, soft tissue hangs below the border in front of the corner and under the chin, and the line blurs.

## Shapes that vary between people

| Shape | What changes | Measured range |
|---|---|---|
| Width at the corners | Narrow against wide jaw | 110 to 120 mm, SD 9 to 10 (J-02) |
| Angle | Square (near 110°) against sloped (near 135°) | 122 to 125°, SD 7 (J-01) |
| Corner position | Corner low and back against high and forward | Corner to chin 84 to 93 mm, SD 8 to 13 (J-03) |
| Muscle bulk | Flat against rounded corner | Not collected |
| Definition | Sharp bone edge against soft, full line | Not collected; follows body fat and age |
| To one side | The chin and lower jaw off the midline | Under 3 mm is normal and unnoticed (F-20) |
| Width against cheekbones | Tapering face against straight-sided face | Jaw 120 mm against cheekbones 144 mm in men (F-01): a ratio near 0.84 |

## Sensible limits

| Control | Limit | Reason |
|---|---|---|
| Jaw width | ±6.2 mm a side (as built); up to about ±8 mm a side | 1.3 SD now, 1.5 to 1.8 SD at 8 mm. Keep the jaw narrower than the cheekbones for a realistic face: the ratio should stay under about 0.95. At ±9 mm a side the average man's jaw would reach 0.96 of the average cheekbone breadth (F-01). |
| Jaw angle (new) | ±13° | 2 SD. Move the corner along the bone's border; do not widen with it. |
| Jaw depth (new) | ±5 mm | By eye, inside 1 SD of corner-to-chin length. |
| Jaw definition (new) | From sharp to full | Tie the full end to body weight so a lean body cannot have a heavy jawline by accident. |
| Jaw to one side | ±5 mm (as built); within 1.5 mm for a natural face; 3.5 mm for the asymmetry benchmark | People notice between 3 and 6 mm. The lower teeth and the mouth's lining move with it (`assets/characters/realistic/face.py:1529-1532`). |
| Left against right width | Within 1.9 mm for a natural face | The jaw's corner is among the most asymmetric points of a healthy face (F-14). |
