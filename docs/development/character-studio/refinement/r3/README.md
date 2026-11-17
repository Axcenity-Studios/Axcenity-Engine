# R3 body (RA3): fat that looks like fat

The pictures the R3 body work was judged by, before and after. Plan section 11.5, R3; the baseline's defect H6 ([../baseline/README.md](../baseline/README.md)); OPEN-08. How the shapes are made is in [assets/characters/realistic/README.md](../../../../../assets/characters/realistic/README.md) ("Fat that is fat (R3)"); the hand-back with the numbers and the gates is [../r1/r3-body-fat.md](../r1/r3-body-fat.md).

## What changed

`fat` from 0 to 1 now also adds `<Frame>_Fat_Soft`: MakeHuman's own CC0 body-shape and regional targets, carried by the existing transfer (`assets/characters/transfer/`) and mixed by fixed weights, placed by frame as the references describe (B12: more central and abdominal on men; more hips, thighs and buttocks on women). The masculine slight body (`muscle` and `fat` both -1) gets `Masculine_Belly_Flat`, MakeHuman's flatter belly. Nothing is sculpted by script; no parameter, range or manifest field changed (three new shapes and their corners, written by `shapes.corners()`).

## The sheets

"Before" is `main` at 5b650d5d (the same assets as the baseline), captured with the same tool and settings; "after" is this branch. Crops of the 1280 x 720 captures, scaled.

| File | What it shows | What I saw |
|---|---|---|
| [engine-heavy-masculine.png](engine-heavy-masculine.png) | Default, then heavy, heavy-soft and heavy-muscular before and after; studio front and side, hard three-quarter | **Heavy after** reads as a heavier man: a round belly in profile (62 mm further forward than the default's), a softer, wider waist, love handles over the waistband, fuller chest and thighs. Before, heavy was the athletic body a little thicker. **Heavy-soft after** is heavier still, the belly over the waistband. **Heavy-muscular after** has a belly and a thicker waist, but its arms, shoulders and trapezius are still a bodybuilder's. The athletic normal map still draws abdominal lines on every belly (CO7, H5). |
| [engine-heavy-feminine.png](engine-heavy-feminine.png) | The same for the feminine frame | **Heavy after** is fuller at the hips, thighs and buttocks, with a lower-belly curve in profile and fuller upper arms; the waist widens with it. The outer upper thigh has a distinct bulge (MakeHuman's `fem-ovary` shape) that throws a shadow line under the hard light: it reads as fat, not as a fault, but it is the sharpest form of the change. **Heavy-soft** carries a belly apron and a roll over the briefs at the back. **Heavy-muscular** keeps very muscular thighs. |
| [engine-lean-end.png](engine-lean-end.png) | Slim and slight, before and after, side and three-quarter, studio and hard light | Masculine slim is unchanged and reads lean (narrower chest and limbs, flat belly). **Masculine slight after**: the soft bulge above the waistband is gone; the abdomen runs flat from the ribs to the pubis. The feminine slight body is unchanged (very thin, the ribs' edge shows under the hard light; it was before). |
| [engine-walk.png](engine-walk.png) | `Walk@0.4` (the run take), heavy before and after, heavy-soft after, both frames | The belly and flanks ride the stride without a step or a tear. The hatched shadow of the arm (H3) and the briefs' loose side panel (H1) are as in the baseline. |
| [engine-face.png](engine-face.png) | Face views, default and heavy, both lights | A fuller jaw line and fat under the chin on the masculine heavy face; on the feminine, slightly. The cheeks don't fill (the face is held round the eyes and mouth). |
| [blender-belly-masculine.png](blender-belly-masculine.png), [blender-belly-feminine.png](blender-belly-feminine.png) | Clay from the built `RealisticParts.glb` (shapes set by the manifest's rule), one hard lamp from the side, no denoising; default, `fat` 1, `fat` 1 with `muscle` -1, and slight; side, three-quarter, front and back three-quarter | **Masculine `fat` 1**: a round belly from under the chest to the pubis, flanks that bulge over the hip crest at the back, no step, crease or facet. **Heavy and soft**: a bigger belly that hangs a little over the pubis. **Feminine `fat` 1**: a lower belly, wide hips and full buttocks, no step. On the feminine heavy and soft body the belly's outline in profile has a slight corner a third of the way down (the 25 mm quads at a large move). The dark jagged patch on the chest in the three-quarter views is the arm's shadow, there on the default body too. |

## Pixels

Heavy against the default, `studio-front`, 921,600 pixels:

| Rule | Masculine before | after | Feminine before | after |
|---|---|---|---|---|
| any channel differs at all | 62,572 (6.79%) | 85,660 (9.29%) | 48,023 (5.21%) | 79,957 (8.68%) |
| any channel differs by more than 8 | 12,057 (1.31%) | 25,802 (2.80%) | 9,789 (1.06%) | 23,670 (2.57%) |
| any channel differs by more than 16 | 7,373 (0.80%) | 18,944 (2.06%) | 5,726 (0.62%) | 18,664 (2.03%) |

The baseline's 7,564 and 6,052 are close to the third rule (more than 15 gives 7,693 and 5,975). Under that rule the 5% gate cannot be met by any body: the whole figure in `studio-front` covers about 28,000 to 33,000 pixels (3 to 3.6% of the frame, counted by skin and cloth colour), and about 60% of it changed. See the hand-back.
