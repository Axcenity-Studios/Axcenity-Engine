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

## The waist (wave 3)

`anim`'s `WaistWidth` widens only the skin weighted to the Spine bone, so on its own it moved the natural waist's breadth (the trunk's narrowest level between the hip crest and the ribs) by 0.25 of the slider on the masculine frame and 0.22 on the feminine, and its depth not at all; the bone's edge was also where the outline stepped. Since wave 3 two shapes per frame, `<Frame>_Waist_Wide` (whole at 1.5) and `<Frame>_Waist_Narrow` (whole at 0.5), driven by `waist_width` from 1 together with the bone, carry the difference between the bone's move and the waist the slider means (`assets/characters/realistic/shapes.py`, `waist`): the trunk scaled about its middle by 0.9 of the slider's change side to side and 0.5 front to back (the back a third of the belly's move), whole at the natural waist and fading smoothly to nothing 15% of the hip-to-chest height below the hip joints and 0.9 of it above the waist. Because the shapes are the target less the bone's own move, the two add up to the target at every value: no kink at 1 and no step where the bone's weight ends. Scripted, not MakeHuman's `measure-waist-circ`: that target would also have needed the bone's move taken out, and it is not in the vendored set, so the licence record is unchanged.

Measured on the built body (natural waist, rest pose, the bone applied as `anim` applies it; `crates/gameplay/tests/realistic_waist.rs`):

| Frame | Value | Breadth (front), share of the slider's change | Depth (side) | Before (bone only) |
|---|---|---|---|---|
| masculine | 0.8 | 0.815 | 0.311 | 0.25, depth 0 |
| masculine | 1.175 | 0.883 | 0.311 | 0.25, depth 0 |
| feminine | 0.8 | 0.828 | 0.286 | 0.22, depth 0 |
| feminine | 1.175 | 0.880 | 0.286 | 0.24, depth 0 |

The sweep then stops the waist at 0.8 (the slight masculine body's outline steps by 0.052 at 0.775, where the change fades into the hips; the masculine population's 0.5th percentile of waist over hip breadth is reached at 0.793 too) and at 1.175 (the lean and muscular masculine body's outline steps by 0.052 at 1.2, under the chest). Every other proportion's ends are unchanged by wave 3's shapes, `<Frame>_Fat_Over_Muscle` included (the sweep was rerun in full: [ranges/ranges.json](ranges/ranges.json)).

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
| `waist_width` | 0.8 | deformation, population | 1.175 | deformation |
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
| `waist_width` | 0.85 to 1.15 | 0.8 to 1.175 | Lower end wider, upper end a little narrower, and the slider now moves the waist by most of its value: the bone alone moved the natural waist's breadth by a quarter of the slider (1.225 was 6% wider), the waist shapes by 0.82 to 0.88 of it (1.175 is 15% wider, 0.8 is 16% narrower). A step in the outline sets both ends (below) |
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
| `waist_width` | low | masculine | 0.8512 | 0.7112 | 0.793 | 0.8 | 0.775: masculine slight: the trunk's outline steps by 0.052 at 0.577 of the hips to chest; masculine lean and muscular: the trunk's outline steps by 0.051 at 1.383 of the hips to chest |
| `waist_width` | low | feminine | 0.6984 | 0.5714 | not reached | 0.8 | 0.775: masculine slight: the trunk's outline steps by 0.052 at 0.577 of the hips to chest; masculine lean and muscular: the trunk's outline steps by 0.051 at 1.383 of the hips to chest |
| `waist_width` | high | masculine | 0.8512 | 0.9911 | 1.195 | 1.175 | 1.2: masculine lean and muscular: the trunk's outline steps by 0.052 at 1.383 of the hips to chest |
| `waist_width` | high | feminine | 0.6984 | 0.8254 | not reached | 1.175 | 1.2: masculine lean and muscular: the trunk's outline steps by 0.052 at 1.383 of the hips to chest |
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

## The stylized family (wave 3)

The stylized body (`assets/characters/humanoid/`) shipped every proportion from 0.5 to 1.5, `anim`'s clamp. Held to the same checks (`ranges.py --package stylized`, the same rules with its own build corners: its own body, slim, heavy, slim and muscular, heavy and muscular), it failed at once: `build` 1 alone crossed itself in 481 (masculine) and 620 (feminine) triangle pairs (the inner thighs, the double chin into the neck, the upper arms' folds), and `measure.py`'s sliders pass found 662 and 481 of 1,682 bodies with new crossings. Three fixes, none a redesign (the third added by the review, "Review fixes 1" in [../wave3/r3-body-2.md](../wave3/r3-body-2.md)):

- **Its build shapes are kept apart** as the realistic body's are (`humanoid/bodyshapes.py`, `keep_apart`; `transfer.apart`): `Build_Heavy` and `Build_Muscular` are scaled down, smoothly, only round the places where they made the body cross itself, on both frames and at the heavy, muscular and heavy and muscular corners, as if 1.25 times as strong (so the proportion sliders' small moves of the bones don't bring the crossings back). The belly, seat and flanks keep their whole move (the belly's largest forward move is 54.5 mm before and after); the inner thighs no longer meet and the chin's fat no longer folds into the neck. Since the review they are also kept apart with the hips narrowed (below) and with the head scaled to 0.88 and 1.12 (`APART_HEAD`: the skin `HeadSize` carries, Head and Jaw weights, scaled about the head's joint, and the shapes' offsets on it with it), and `Build_Slim` with them (the slim feminine jaw folded into itself as the head grew). That takes little more: on the jaw's skin `Build_Heavy`'s RMS move goes from 5.29 to 5.13 mm and its largest stays 9.5 mm (it changes by more than 0.1 mm at 93 jaw, 90 head and 42 hip and inner-thigh vertices, by the measure of the bone each vertex is most weighted to); `Build_Slim`'s jaw from 1.34 to 1.23 mm RMS; `Build_Muscular` at 13 vertices of the hips and thighs.
- **Its hips narrow by a shape** (review fixes 1; decision 8, "hips go both ways"), the realistic body's mechanism: `Masculine_Hips_Narrow` and `Feminine_Hips_Narrow` (`bodyshapes.hips_narrow`), corners with the frame, whole at `hip_width` 0.6; below 1 `HipWidth` stays at 1 (`ATTRIBUTE_FROM_ONE`). Each moves the pelvis and the thighs' tops toward the middle by up to 0.22 of their distance from it, side to side only, the inner thighs held, fading above the hip crest and down the thigh. The hips' breadth falls 0.51 (masculine) and 0.53 (feminine) of the slider's change below 1, against 0.57 and 0.52 above it, so the slider has no kink at 1. The garments follow both shapes.
- **Its ranges are the sweep's**, below. Differences from the realistic rules: the trunk's outline is measured up to 1.1 of the hips to chest, not 1.45 (the stylized chest joint sits higher, its armpits at about 1.18, and a level through them measures the arm's join); its eyes and teeth are not checked (its eye measure counts its own lids as flesh, 542 and 410 vertices at rest, and moves by a few as the head scales); and its hips narrow by their shape (by the bone, as before the review, the inner thighs crossed at 0.95, and with `build` 1 already at 0.975). Its joints are held at what they reach today (`humanoid/qa/body-thresholds.json`), not at the realistic body's limits: its elbow and knee keep 0.37 to 0.45 of their ring area at 135 degrees against the realistic body's 0.6, because it has no twist or helper bones, and adding them would redesign its rig.

The stylized body stands 1.75 m on both frames and is not proportioned like the survey's people, so its population ends are the same relative spreads as the realistic body's on its own dimensions.

## Stylized shipped ranges

| Parameter | Low end | Set by | High end | Set by |
|---|---|---|---|---|
| `height` | 0.85 | deformation, population | 1.15 | population |
| `leg_length` | 0.81 | population | 1.275 | deformation |
| `arm_length` | 0.92 | population | 1.08 | population |
| `shoulder_width` | 0.95 | deformation | 1.12 | population |
| `waist_width` | 0.775 | deformation | 1.225 | deformation |
| `hip_width` | 0.7 | deformation | 1.15 | deformation |
| `head_size` | 0.87 | population | 1.125 | deformation |
| `hand_size` | 0.81 | population | 1.11 | population |
| `foot_size` | 0.98 | population | 1.27 | population |

"The sliders pass" is `measure.py`'s check of every pair of the body's sliders at their ends (the sweep moves one proportion at a time over the build's corners, and counts a crossing only where the pair of triangles was more than 4 mm apart on the body at 1; the sliders pass counts against the neutral body). Before the review two pairs crossed at the sweep's ends and the range was cut to suit them: `build` 1 with `hip_width` 0.975 (the hips narrowed by the bone) and `build` 1 or -1 with `head_size` 1.075 or 1.05 (the chin's fat meets the neck), so the hips stopped at 1 and the head at 0.975 to 1.025. With the hips' narrowing shape and the build kept apart with the head scaled, no pair of the body's sliders crosses at the ends above, on either frame (`measure.py --package stylized` with `body-thresholds.json`: 0 bodies and 0 alone, 1,682 bodies a frame). The hips stop at 0.7, where the masculine outline steps by 0.050 below the hips (as the realistic body's do); the head at the population's 0.5th percentile (0.873 masculine, 0.863 feminine) and at 1.125 (at 1.15 the heavy masculine chin meets the neck again, 2 pairs).

### The stylized ends' numbers

| Parameter | Side | Frame | Dimension at 1 | Population bound | Its slider value | Last passing step | First failing step and why |
|---|---|---|---|---|---|---|---|
| `height` | low | masculine | 1.75 | 1.5783 | 0.849 | 0.85 | 0.825: masculine slim: section UpperChest@0.5 at 0.70 of its size at 1 |
| `height` | low | feminine | 1.75 | 1.4631 | not reached | 0.85 | 0.825: masculine slim: section UpperChest@0.5 at 0.70 of its size at 1 |
| `height` | high | masculine | 1.75 | 1.9337 | 1.159 | 1.175 | none in the sweep |
| `height` | high | feminine | 1.75 | 1.7929 | 1.038 | 1.175 | none in the sweep |
| `leg_length` | low | masculine | 0.4913 | 0.4744 | 0.927 | 0.8 | none in the sweep |
| `leg_length` | low | feminine | 0.5317 | 0.4829 | 0.805 | 0.8 | none in the sweep |
| `leg_length` | high | masculine | 0.4913 | 0.5516 | 1.295 | 1.275 | 1.3: feminine own body: the trunk's outline steps by 0.051 at 0.594 of the hips to chest |
| `leg_length` | high | feminine | 0.5317 | 0.5551 | 1.109 | 1.275 | 1.3: feminine own body: the trunk's outline steps by 0.051 at 0.594 of the hips to chest |
| `arm_length` | low | masculine | 0.2826 | 0.2614 | 0.925 | 0.9 | none in the sweep |
| `arm_length` | low | feminine | 0.259 | 0.2374 | 0.916 | 0.9 | none in the sweep |
| `arm_length` | high | masculine | 0.2826 | 0.3039 | 1.075 | 1.1 | none in the sweep |
| `arm_length` | high | feminine | 0.259 | 0.2807 | 1.084 | 1.1 | none in the sweep |
| `shoulder_width` | low | masculine | 0.1994 | 0.1778 | not reached | 0.95 | 0.925: masculine heavy and muscular: 12 new crossings |
| `shoulder_width` | low | feminine | 0.1401 | 0.124 | not reached | 0.95 | 0.925: masculine heavy and muscular: 12 new crossings |
| `shoulder_width` | high | masculine | 0.1994 | 0.2211 | 1.115 | 1.15 | none in the sweep |
| `shoulder_width` | high | feminine | 0.1401 | 0.1563 | 1.125 | 1.15 | none in the sweep |
| `waist_width` | low | masculine | 0.8405 | 0.7023 | not reached | 0.775 | 0.75: masculine heavy: the trunk's outline steps by 0.050 at 0.594 of the hips to chest; masculine heavy and muscular: the trunk's outline steps by 0.050 at 0.594 of the hips to chest |
| `waist_width` | low | feminine | 0.5585 | 0.4569 | not reached | 0.775 | 0.75: masculine heavy: the trunk's outline steps by 0.050 at 0.594 of the hips to chest; masculine heavy and muscular: the trunk's outline steps by 0.050 at 0.594 of the hips to chest |
| `waist_width` | high | masculine | 0.8405 | 0.9787 | not reached | 1.225 | 1.25: masculine heavy: the trunk's outline steps by 0.050 at 0.594 of the hips to chest; masculine heavy and muscular: the trunk's outline steps by 0.050 at 0.594 of the hips to chest |
| `waist_width` | high | feminine | 0.5585 | 0.66 | not reached | 1.225 | 1.25: masculine heavy: the trunk's outline steps by 0.050 at 0.594 of the hips to chest; masculine heavy and muscular: the trunk's outline steps by 0.050 at 0.594 of the hips to chest |
| `hip_width` | low | masculine | 0.1683 | 0.1397 | not reached | 0.7 | 0.675: masculine own body: the trunk's outline steps by 0.050 at -0.644 of the hips to chest; masculine slim: the trunk's outline steps by 0.054 at -0.644 of the hips to chest |
| `hip_width` | low | feminine | 0.2027 | 0.1666 | not reached | 0.7 | 0.675: masculine own body: the trunk's outline steps by 0.050 at -0.644 of the hips to chest; masculine slim: the trunk's outline steps by 0.054 at -0.644 of the hips to chest |
| `hip_width` | high | masculine | 0.1683 | 0.1969 | not reached | 1.15 | 1.175: feminine heavy and muscular: the trunk's outline steps by 0.051 at 0.706 of the hips to chest |
| `hip_width` | high | feminine | 0.2027 | 0.2387 | not reached | 1.15 | 1.175: feminine heavy and muscular: the trunk's outline steps by 0.051 at 0.706 of the hips to chest |
| `head_size` | low | masculine | 0.1557 | 0.1398 | 0.873 | 0.85 | none in the sweep |
| `head_size` | low | feminine | 0.1731 | 0.1537 | 0.863 | 0.85 | none in the sweep |
| `head_size` | high | masculine | 0.1557 | 0.1716 | 1.131 | 1.125 | 1.15: masculine heavy: 2 new crossings; masculine heavy and muscular: 2 new crossings |
| `head_size` | high | feminine | 0.1731 | 0.1924 | 1.143 | 1.125 | 1.15: masculine heavy: 2 new crossings; masculine heavy and muscular: 2 new crossings |
| `hand_size` | low | masculine | 0.1234 | 0.0993 | 0.805 | 0.8 | none in the sweep |
| `hand_size` | low | feminine | 0.1102 | 0.0991 | 0.899 | 0.8 | none in the sweep |
| `hand_size` | high | masculine | 0.1234 | 0.1209 | 1.000 | 1.125 | none in the sweep |
| `hand_size` | high | feminine | 0.1102 | 0.1233 | 1.119 | 1.125 | none in the sweep |
| `foot_size` | low | masculine | 0.1444 | 0.141 | 0.973 | 0.95 | none in the sweep |
| `foot_size` | low | feminine | 0.1337 | 0.1376 | 1.000 | 0.95 | none in the sweep |
| `foot_size` | high | masculine | 0.1444 | 0.1678 | 1.192 | 1.275 | none in the sweep |
| `foot_size` | high | feminine | 0.1337 | 0.165 | 1.274 | 1.275 | none in the sweep |

Regenerate: `python -I assets/characters/humanoid/qa/ranges.py --package stylized --out ranges-stylized.json` (about 5 minutes), then `--report ranges-stylized.json`. The whole output: [ranges/ranges-stylized.json](ranges/ranges-stylized.json).
