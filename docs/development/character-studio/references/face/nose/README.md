# Nose: reference index

Part of the [face reference index](../README.md) (brief §5.2 "Nose", §13.2 "Nose profiles", "Nose tips", "Nostril shapes"). Row ids starting F- are in that file. Collected 2026-11-16. Links only; nothing downloaded.

## Index

| Reference | Source URL | Author or publisher | Date collected (2026-11-16) | Feature | What it demonstrates | Licence or restrictions | Use (visual guidance / data / actual asset) |
|---|---|---|---|---|---|---|---|
| N-01 Anthropometric study of the Caucasian nose in the city of Curitiba | https://pmc.ncbi.nlm.nih.gov/articles/PMC9449246 | Ballin and others; Braz J Otorhinolaryngol 84(4), 2018 | 2026-11-16 | Profile angles, tip | 100 adults: nasolabial angle 105.4 ± 10.7°, nasofrontal angle 137.1 ± 8.0°, tip projection as a share of nose length 0.63 ± 0.05, nostril width against inner eye-corner width 1.15 ± 0.10. Measured people differ from the textbook "ideal" on four of five. | Class 2: CC BY 4.0 (read in the article). Opened. | data |
| N-02 Nose rows of the US respirator survey | https://stacks.cdc.gov/view/cdc/190020 | Zhuang and Bradtmiller, NIOSH, 2005 (F-01) | 2026-11-16 | Length, width, projection, bridge width | Nose height 52.0 ± 4.1 mm (men), 48.2 ± 3.8 (women); breadth 36.6 ± 4.1, 33.2 ± 3.9; protrusion 21.1 ± 2.7, 19.8 ± 2.7; root breadth 16.6 ± 2.3, 16.3 ± 2.0. Breadth ran from 22 to 58 mm across the sample. | Class 1: record labelled "Public Domain". Opened. | data |
| N-03 Nose rows of the Malay 3D study | https://pmc.ncbi.nlm.nih.gov/articles/PMC5051712 | Othman and others, 2016 (F-10) | 2026-11-16 | Tip, width | A second population: tip protrusion 17.3 ± 1.8 mm (men), width 39.6 ± 2.3, height 54.1 ± 3.6, bridge length 46.9 ± 3.3, nasolabial angle 105.1 ± 9.6°. | Class 2: CC BY. Opened. | data |
| N-04 Gray's Anatomy (1918) plates 852 and 853: cartilages of the nose, side and from below | https://commons.wikimedia.org/wiki/File:Gray852.png , https://commons.wikimedia.org/wiki/File:Gray853.png | Henry Gray, drawn by H. V. Carter; Wikimedia Commons | 2026-11-16 | Structure | Where bone ends and cartilage begins on the bridge; the paired tip cartilages that set the tip's width and the nostril's outline. | Class 1: public domain (each file's licence field on Commons). | visual guidance; could be shown in docs |
| N-05 Nasal anthropometry: an assessment among the Akan and Ewe ethnic groups in Ghana | https://pmc.ncbi.nlm.nih.gov/articles/PMC11315967 | Robertson and others; Biomed Res Int, 2024 | 2026-11-16 | Width, height | A third population for nose width and height. A lead for Stage 4. | Class 2: CC BY (licence statement confirmed in the article). Text not read. | data |

Also used: F-04 and F-09 (how nose shape differs between groups: nasofrontal and nasolabial angles differ by 8° to 17° between group means), F-06 (the al, prn and sn landmarks; nasal width, height, protrusion, bridge length), F-08 (landmark definitions), F-20 (a nose 4 mm off centre is noticed), F-24 and F-25 (artists' breakdowns and profile photographs).

## Anatomy

The upper third of the nose is bone: the two nasal bones, joined to the forehead at nasion. Below them the bridge is cartilage, and the join (rhinion) is where a hump shows. The tip is made by two curved cartilages, one a side, whose domes set the tip's width and whose side arms shape the nostril rims. The nostril wings (alae) are skin and fibrous fat with no cartilage at their base; alare is their widest point. The column between the nostrils (columella) meets the lip at subnasale. The angle there, between column and lip, is the nasolabial angle: larger means a more upturned tip. Skin is thin over the bridge and thick over the tip, so bone shapes show above and soft shapes below.

## Shapes that vary between people

| Shape | What changes | Measured range |
|---|---|---|
| Length | Root to base | 48 to 52 mm, SD 4 (N-02) |
| Width at the nostrils | Wing to wing | 33 to 37 mm, SD 4 (N-02); group means differ by several mm (F-04) |
| Bridge width | Narrow against broad root | 16.5 mm, SD 2.0 to 2.3 (N-02) |
| Bridge height | High, sharp bridge against low, flat one | Nasofrontal angle 137 ± 8° (N-01) |
| Bridge profile | Humped, straight or scooped | Not collected |
| Tip projection | How far the tip stands off the face | 20 to 21 mm, SD 2.7 (N-02) |
| Tip rotation | Upturned against drooping | Nasolabial angle 102 to 105°, SD 9 to 11 (N-01, N-03) |
| Tip width | Fine against broad | Not collected |
| Nostril shape | Long and narrow against round and flared | Not collected |
| To one side | The lower nose off the midline | Common in small amounts; noticed at 4 mm (F-20) |

## Sensible limits

| Control | Limit | Reason |
|---|---|---|
| Length | ±5 mm (as built), to ±8 mm only with a shape that stretches the whole bridge | 1.3 SD now; 2 SD at 8 mm. The existing shape moves the lower nose only (`assets/characters/realistic/face.py:1491-1493`). |
| Nostril width | ±4.6 mm a side (as built) | ±2.3 SD of nose breadth. |
| Bridge width (new) | ±2.8 mm a side | 2.5 SD of root breadth. |
| Bridge height | ±4.6 mm (as built) | Keeps the bridge in front of the inner eye corners at the low end. |
| Bridge profile (new) | ±2 mm at rhinion | By eye; a hump larger than that reads as an injury. |
| Tip projection | ±5.4 mm | 2 SD. |
| Tip rotation | ±20° of nasolabial angle | 2 SD. Never rotate up so far that the nostrils face forward. |
| Tip width (new) | ±2.5 mm | By eye. |
| To one side | Within 1.3 mm for a natural face; 3 mm for the asymmetry benchmark | People notice 4 mm. |
| Ratio check | Nostril width between 0.95 and 1.35 of the distance between the inner eye corners | Mean 1.15, SD 0.10 (N-01). A cheap test for a random face. |
