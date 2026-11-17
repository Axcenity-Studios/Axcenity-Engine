# R3 wider ranges (RA3): where each proportion slider ends

Plan section 0, decision 8 (2026-11-17): "The 0.85 to 1.15 cut is not a hard limit, and hips go both ways. Ranges are wider and anatomically sensible, each end set by deformation quality (no mesh collapse, no clothing-fit failure) and by population data, not by a fixed share." Plan 11.5, R3. The hand-back with the gates and the pictures is [../r2/r3-ranges.md](../r2/r3-ranges.md); the pictures are in [ranges/](ranges/).

The table under "Shipped ranges" is what `assets/characters/realistic/manifests.py` ships (`PROPORTION_RANGES`); `crates/avatar/tests/realistic_ranges.rs` reads both and fails when they differ.

## How the ends were found

`assets/characters/humanoid/qa/ranges.py` (Blender) steps each of the nine proportion sliders outward from 1 by 0.025, both ways, on both frames. At each step it checks five bodies per frame: the frame's own body and the four corners of `muscle` and `fat` at their ends (slight, lean and muscular, heavy and soft, heavy and muscular). A step fails when, on any of the ten:

| Check | Rule |
|---|---|
| Crossings | The skin newly crosses itself at rest: pairs of triangles that cross where neither the frame's neutral body nor the same body at 1 crosses, and that weren't within 4 mm of touching there (the sliders pass's method, `measure.py --only sliders`) |
| Joints | In the QA poses the slider bears on (height: knee, hip flex, spine twist, shoulder at 90 and 150 degrees; legs: knee, hip flex; arms: elbow, wrist, shoulder lowered, at 90 and at 150; shoulders: the three shoulder poses; waist: spine twist, hip flex; hips: hip flex, knee; hands: wrist), the joint's ring area ratio (posed over rest, as `measure.py`'s `deformation` measures it) stays at or above the package's limit in `qa/thresholds.json`, or, where there is none or the body at 1 is already under it, at 0.9 of the same body's at 1 (of its rest size, where the joint swells). The local volume ratio only where the package holds one: the shoulder at 90 degrees (at least the limit) and lowered (at most the limit, or the body's own at 1 plus 0.01 where that is already within 0.01 of the limit) |
| Eyes and teeth | No skin in an eyeball, no eyeball front in the lids' flesh, no tooth in the lips' or gums' flesh (`measure.eye_metrics`, on the frame's own body, with the eyes where the sliders put them) |
| Collapse | No cross-section of the trunk or a limb (12 of them: pelvis, waist twice, chest, upper chest, neck, thigh twice, shin, upper arm twice, forearm) loses more than 30% of its narrowest breadth, and no region (pelvis, waist, chest, thigh, shin, upper arm, forearm, neck) loses more than 30% of its skin area, against the same body at 1 |
| A step in the outline | The trunk's breadth, cut by level planes at 17 heights from about 15 cm below the hip joints to 10 cm above the chest joint (placed by those joints, so a longer body is cut at the same places), changes against the same body at 1 by no more than 0.05 more at one height than at the next (brief §4.4, "no abrupt deformation transitions") |

The bones are placed as `anim::BodyShape::apply` places them on a pose (each bone's scale its own, its children moved; the twist and helper bones take their limb's and joint's scale); the script checks itself against that rule on a posed body (0.027 mm). `Character.set_body_sliders`, which `measure.py`'s sliders pass uses, sets each bone with Blender's full scale inheritance and drops a child's shear; the sweep uses Blender's `NONE` inheritance and so matches `anim` exactly.

**The population.** Each slider has a dimension measured on the body at every step, compared with ANSUR II (4,082 men, 1,986 women; [references/body/overall/README.md](../../references/body/overall/README.md)): the 0.5th and 99.5th percentiles are the mean less or plus 2.576 SD on a normal curve. Where the body's own dimension is comparable with the survey's, the bound is where the body's own value reaches it ("own"); where the landmarks differ (joint centres against bony landmarks), the body is taken to sit at the mean and only the relative spread is used ("mean").

| Slider | Dimension on the body | Survey's (men; women) | Anchor |
|---|---|---|---|
| `height` | stature, m | stature 1.756 ± 0.069; 1.628 ± 0.064 | own |
| `leg_length` | hip joint height over stature | trochanter height over stature 0.513 ± 0.015; 0.519 ± 0.014 | own |
| `arm_length` | shoulder to elbow to wrist joint over stature | (upper arm + forearm) over stature 0.343 ± 0.010; 0.339 ± 0.011 | mean |
| `shoulder_width` | shoulder joint width over stature | biacromial breadth over stature 0.237 ± 0.010; 0.224 ± 0.010 | mean |
| `waist_width` | waist breadth (at the spine's middle) over hip breadth | waist over hip breadth 0.94 ± 0.06; 0.85 ± 0.06 | mean |
| `hip_width` | hip breadth (the trunk's widest, hip joints less 12 cm to the spine) over stature | hip breadth over stature 0.197 ± 0.013; 0.217 ± 0.015 | mean |
| `head_size` | head breadth over stature (scales as the circumference) | head circumference over stature 0.327 ± 0.013; 0.345 ± 0.015 | mean |
| `hand_size` | wrist joint to fingertip over stature | hand length over stature 0.1101 ± 0.0042; 0.1112 ± 0.0047 | own |
| `foot_size` | foot length over stature | foot length over stature 0.1544 ± 0.0052; 0.1513 ± 0.0053 | own |

The waist is held against the hips, not against stature: the slider changes the waist and nothing else, and waist over stature varies three times as much as the frame (a wasp waist would be "in the population").

**One range for both frames.** A manifest parameter has one range. Each end is the nearest to 1 of: the last passing step, if any step failed (on either frame, any body); the population's percentile, **the farther of the two frames'**, rounded to 0.01 toward 1 (each frame's own people reach their own end: a range stopped at the nearer would hold tall men to the 99.5th percentile of women, 1.79 m); and `anim`'s clamp (0.5 to 1.5), or for the hips below 1 where the narrowing shape is whole (0.6). So a feminine body at `height` 1.13 is 1.89 m, past the 99.5th percentile of women, and a masculine foot can't go below 0.96, which is already shorter than the 0.5th percentile of men (the base's foot is at the 2nd: [references/body](../../references/body/README.md), section 8). The randomizer still draws from the middle half of each range.

## The hips below 1

`anim`'s `HipWidth` widens the hip bone's skin and moves the legs' roots out with it; below 1 it pulls the legs in and the inner thighs cross at the crotch. The sweep, the old way (`--hips-by-bone`), on the build before this change: `HipWidth` 0.975 passes, **0.95 fails** (masculine own body 20 new crossings, slight 12, lean and muscular 12, heavy and muscular 16), as CO5 found.

So below 1 no bone moves (the manifest's attribute driver for `hip_width` runs from 1 up: `"from": [1, 1.2]`) and a shape narrows the surface: `<Frame>_Hips_Narrow` (`assets/characters/realistic/shapes.py`, `hips_narrow`), a corner of the frame and `hip_width` from 1 down to 0.6. It is the hip bone's width change made on the skin: every vertex of the pelvis and the thighs' tops brought toward the middle by 20% of its distance from it (side to side only, so the two sides can't cross, and smooth enough that the skin's order across never folds), the inner thighs held (they are let go from 12 to 70 mm from the middle), fading out above the hip crest (8 to 24 cm above the hip joints) and down the thigh (10 to 25 cm below), on the trunk's and legs' skin only (a hand hanging at the hip doesn't narrow). Its strength is chosen so that the hips' breadth falls as fast below 1 as `HipWidth` widens it above 1, about half a per cent of breadth per per cent of slider, so the slider has no kink at 1: at 0.7 the hip breadth is 0.864 of the default (masculine) and 0.865 (feminine); at 1.2 it is 1.099 and 1.095. No MakeHuman target was used (its `hip-scale-horiz-decr` is the same kind of scale, and it would have needed the bundle and a new transfer), so the licence record is unchanged. The hip flex pose passed at every step (its ring area ratio stays above 0.9 of the body's own), so no corrective shape was added. In the side-lit clay at 100 degrees ([ranges/blender-hip-flex.png](ranges/blender-hip-flex.png)) the narrow hips fold exactly as the default ones do: the groin crease and the faceted underside of the buttock are there at 1 too. The sweep stops the narrowing at 0.7: at 0.675 the slight masculine body's outline steps by 0.051 above the hip crest. The population's 0.5th percentile of hip breadth (0.83 of the mean) would be reached near 0.66.

## The pictures

In [ranges/](ranges/). What I saw is in the hand-back, [../r2/r3-ranges.md](../r2/r3-ranges.md).

| File | What it shows |
|---|---|
| `engine-ends-<frame>-<light>.png` | In the engine, every proportion at its two new ends beside the default, front and side, studio and hard light |
| `engine-widths-<frame>.png` | The shoulders, waist and hips at their ends, close, front and side, both lights |
| `engine-suite-<frame>-<light>.png` | The body suite (rest, T-pose, arms raised, elbows and knees bent, a walk frame) at the hips' and shoulders' ends |
| `engine-benchmarks.png` | The benchmarks `body-shortest`, `body-tallest`, `body-narrowest-hips` and `body-widest-hips` on the three frames |
| `blender-hips.png` | Clay under one side lamp: the hips at 1 and 0.7 on both frames and on the heavy and soft bodies, front, three-quarter and back; the old bone narrowing at 0.95; the hip flex at 0.7 |
| `blender-hip-flex.png` | Clay, the hip flex at 100 degrees at 1 and 0.7, both frames |
| `blender-outline-check.png` | Clay, the slight masculine body at the steps where the outline check fails (legs 1.15, waist 0.75, hips 1.25) beside legs 1 and 1.1, and the waist at 1.15 in the spine twist: what the 0.05 limit looks like |


## Shipped ranges

| Parameter | Low end | Set by | High end | Set by |
|---|---|---|---|---|
| `height` | 0.82 | population | 1.13 | population |
| `leg_length` | 0.85 | deformation, population | 1.125 | deformation |
| `arm_length` | 0.95 | deformation | 1.08 | population |
| `shoulder_width` | 0.88 | population | 1.12 | population |
| `waist_width` | 0.775 | deformation | 1.225 | deformation |
| `hip_width` | 0.7 | deformation | 1.2 | deformation |
| `head_size` | 0.88 | population | 1.12 | population |
| `hand_size` | 0.82 | population | 1.07 | population |
| `foot_size` | 0.96 | population | 1.21 | population |

Set by: "deformation" is the last passing step before a check failed, "population" the 0.5th or 99.5th percentile (the farther frame's), "the narrowing shape's end" where `<Frame>_Hips_Narrow` is whole. Where two candidates round to the same value both are named.

## Against CO5's cut

| Parameter | CO5 | Now | Wider or narrower, and why |
|---|---|---|---|
| `height` | 0.85 to 1.15 | 0.82 to 1.13 | Lower end wider (the 0.5th percentile of women, 1.46 m); upper end a little narrower (the 99.5th percentile of men, 1.93 m) |
| `leg_length` | 0.85 to 1.15 | 0.85 to 1.125 | Upper end narrower: at 1.15 the slight masculine body's waist pinches in (the hip helper bones take the thighs' length scale, which stretches the flank's skin upward) |
| `arm_length` | 0.85 to 1.15 | 0.95 to 1.08 | Both narrower. 1.15 is 5 SD; the 99.5th percentile is 1.08. Below 0.95 the heavy and soft body's lowered shoulder swells past its limit (1.054 against 1.052 at 0.925); the population would allow 0.92 |
| `shoulder_width` | 0.85 to 1.15 | 0.88 to 1.12 | Both a little narrower: the percentiles of biacromial breadth |
| `waist_width` | 0.85 to 1.15 | 0.775 to 1.225 | Both wider. The bone changes the waist's breadth by only about a sixth of the slider (1.225 is 4% wider), so the population is never reached; a step in the outline sets both ends |
| `hip_width` | 1 to 1.15 | 0.7 to 1.2 | Both wider; **the hips narrow** (above) |
| `head_size` | 0.85 to 1.15 | 0.88 to 1.12 | Both narrower: the percentiles of head circumference over stature |
| `hand_size` | 0.85 to 1.15 | 0.82 to 1.07 | Lower end wider, upper narrower: the feminine hand is already at the 99th percentile of women and the masculine at the 70th, so the population allows less growth than shrinking |
| `foot_size` | 0.85 to 1.15 | 0.96 to 1.21 | Lower end much narrower, upper wider: the base's feet are short (masculine under the 0.5th percentile, feminine at the 5th) |

Five ranges are narrower at one or both ends than CO5's. That is the rule decision 8 sets (population data as well as deformation), not a fixed share; where it bites hardest (arms, feet) it is because the base body itself sits off the middle of the population or because ±15% was several SD.

## Each end's numbers

| Parameter | Side | Frame | Dimension at 1 | Population bound | Its slider value | Last passing step | First failing step and why |
|---|---|---|---|---|---|---|---|
| `height` | low | masculine | 1.75 | 1.5783 | 0.869 | 0.8 | none in the sweep |
| `height` | low | feminine | 1.694 | 1.4631 | 0.817 | 0.8 | none in the sweep |
| `height` | high | masculine | 1.75 | 1.9337 | 1.139 | 1.15 | none in the sweep |
| `height` | high | feminine | 1.694 | 1.7929 | 1.078 | 1.15 | none in the sweep |
| `leg_length` | low | masculine | 0.5133 | 0.4744 | 0.844 | 0.85 | 0.825: masculine slight: the trunk's outline steps by 0.054 at 0.577 of the hips to chest |
| `leg_length` | low | feminine | 0.5226 | 0.4829 | 0.840 | 0.85 | 0.825: masculine slight: the trunk's outline steps by 0.054 at 0.577 of the hips to chest |
| `leg_length` | high | masculine | 0.5133 | 0.5516 | not reached | 1.125 | 1.15: masculine slight: the trunk's outline steps by 0.055 at 0.711 of the hips to chest |
| `leg_length` | high | feminine | 0.5226 | 0.5551 | not reached | 1.125 | 1.15: masculine slight: the trunk's outline steps by 0.055 at 0.711 of the hips to chest |
| `arm_length` | low | masculine | 0.3101 | 0.2868 | not reached | 0.95 | 0.925: masculine heavy and soft: shoulder_abd_10 local_volume_ratio 1.054 over 1.052 |
| `arm_length` | low | feminine | 0.3129 | 0.2868 | not reached | 0.95 | 0.925: masculine heavy and soft: shoulder_abd_10 local_volume_ratio 1.054 over 1.052 |
| `arm_length` | high | masculine | 0.3101 | 0.3334 | 1.075 | 1.1 | none in the sweep |
| `arm_length` | high | feminine | 0.3129 | 0.3391 | 1.084 | 1.1 | none in the sweep |
| `shoulder_width` | low | masculine | 0.1694 | 0.151 | 0.883 | 0.85 | none in the sweep |
| `shoulder_width` | low | feminine | 0.1492 | 0.1321 | 0.875 | 0.85 | none in the sweep |
| `shoulder_width` | high | masculine | 0.1694 | 0.1878 | 1.118 | 1.15 | none in the sweep |
| `shoulder_width` | high | feminine | 0.1492 | 0.1664 | 1.126 | 1.15 | none in the sweep |
| `waist_width` | low | masculine | 0.8512 | 0.7112 | not reached | 0.775 | 0.75: masculine slight: the trunk's outline steps by 0.053 at 0.711 of the hips to chest |
| `waist_width` | low | feminine | 0.6984 | 0.5714 | not reached | 0.775 | 0.75: masculine slight: the trunk's outline steps by 0.053 at 0.711 of the hips to chest |
| `waist_width` | high | masculine | 0.8512 | 0.9911 | not reached | 1.225 | 1.25: masculine slight: the trunk's outline steps by 0.053 at 0.711 of the hips to chest |
| `waist_width` | high | feminine | 0.6984 | 0.8254 | not reached | 1.225 | 1.25: masculine slight: the trunk's outline steps by 0.053 at 0.711 of the hips to chest |
| `hip_width` | low | masculine | 0.2126 | 0.1765 | not reached | 0.7 | 0.675: masculine slight: the trunk's outline steps by 0.051 at 0.711 of the hips to chest |
| `hip_width` | low | feminine | 0.22 | 0.1808 | not reached | 0.7 | 0.675: masculine slight: the trunk's outline steps by 0.051 at 0.711 of the hips to chest |
| `hip_width` | high | masculine | 0.2126 | 0.2488 | not reached | 1.2 | 1.225: masculine slight: the trunk's outline steps by 0.052 at 0.711 of the hips to chest |
| `hip_width` | high | feminine | 0.22 | 0.2592 | not reached | 1.2 | 1.225: masculine slight: the trunk's outline steps by 0.052 at 0.711 of the hips to chest |
| `head_size` | low | masculine | 0.1077 | 0.0966 | 0.888 | 0.875 | none in the sweep |
| `head_size` | low | feminine | 0.107 | 0.095 | 0.877 | 0.875 | none in the sweep |
| `head_size` | high | masculine | 0.1077 | 0.1187 | 1.114 | 1.15 | none in the sweep |
| `head_size` | high | feminine | 0.107 | 0.119 | 1.126 | 1.15 | none in the sweep |
| `hand_size` | low | masculine | 0.1122 | 0.0993 | 0.885 | 0.8 | none in the sweep |
| `hand_size` | low | feminine | 0.1214 | 0.0991 | 0.817 | 0.8 | none in the sweep |
| `hand_size` | high | masculine | 0.1122 | 0.1209 | 1.078 | 1.1 | none in the sweep |
| `hand_size` | high | feminine | 0.1214 | 0.1233 | 1.016 | 1.1 | none in the sweep |
| `foot_size` | low | masculine | 0.1406 | 0.141 | 1.000 | 0.95 | none in the sweep |
| `foot_size` | low | feminine | 0.1428 | 0.1376 | 0.959 | 0.95 | none in the sweep |
| `foot_size` | high | masculine | 0.1406 | 0.1678 | 1.219 | 1.225 | none in the sweep |
| `foot_size` | high | feminine | 0.1428 | 0.165 | 1.177 | 1.225 | none in the sweep |

"Its slider value" is where the dimension reaches the bound, between the steps that straddle it; 1 where the body at 1 is already past it; "not reached" where the sweep stopped first (a failure, or the clamp). The sweep stops a side at its first failing step, or once both frames are past their bound. The sweep's whole output (every step, every body, every check) is [ranges/ranges.json](ranges/ranges.json). Regenerate: `python -I assets/characters/humanoid/qa/ranges.py --out ranges.json` with a Python that has `bpy` (about 5 minutes), then `--report ranges.json` prints these two tables. `measure.py --check` on this build, sliders included: [ranges/measure-check.txt](ranges/measure-check.txt).
