# CO5 spike: MakeHuman's body targets on the realistic body

The spike before milestone CO5 (body parameters) of [the plan](../../../../plans/axcenity-engine-character-studio.md), decision D5: can MakeHuman's CC0 targets be carried onto the realistic base body's topology well enough to be its anatomy source for fat, muscle and frame shapes? Done 2026-11-14 on branch `co5-makehuman-spike`. The scripts are in [assets/characters/transfer/](../../../../assets/characters/transfer/); the numbers here are in [measurements.json](measurements.json).

## Answer: go, with conditions

**The transfer works.** All 16 corner targets (gender × muscle × weight at the young age) and six regional ones came over. On our body 14 of the 16 corners change the volume within 2.5 points of what they change on MakeHuman's own mesh (the two masculine corners that had to be cut back at the armpit are 3.1 and 4.5 points under: +84.9% against +88.0%, +40.3% against +44.8%), 14 put their largest move within 6 to 39 mm of MakeHuman's (the other two have two near-equal peaks and pick the other one), and none adds noise (the offsets are 0.59 to 0.86 as rough as the source at the median, so a little smoother, never rougher), and after two cheap fixes are symmetric and add no self-intersection. In the renders they read as fat and muscle, not as inflation: a belly that hangs over a crease, chest fat with a fold under it, flanks that fold at the back, a thin body that hollows at the cheeks and thighs. MakeHuman's corner mixing maps onto the manifest's `corners` exactly (16 shapes, difference 0).

**The conditions**, most important first:

1. **The look is MakeHuman's look, and its extremes are caricatures.** The two masculine extremes (max weight with min muscle, and with max muscle) are an obese body and a comic-book bodybuilder. Whether that meets a photoreal bar is the lead's call from [details.png](details.png); the middle of the range is the safer part. Nothing in the transfer makes the anatomy better than MakeHuman's.
2. **Features land 1 to 3 cm off along the trunk and limbs.** The correspondence is tight across the skin (1 mm) but slides along it: on a test with a known answer the middle error is 13 mm on the trunk and 20 mm on the legs (95%: 32 and 45 mm). Smooth fat doesn't show it. Edges do: MakeHuman's pectoral edge lands beside our body's own, which gives the hard line under the chest in the heavy-and-muscular and heavy-and-soft renders, and MakeHuman's navel lands on smooth skin and becomes a crease across the lower belly. CO5 needs a dozen hand-picked landmark pairs on the trunk (nipples, navel, pectoral edge, kneecaps) or has to accept these.
3. **Our masculine base is already athletic.** MakeHuman's shapes were sculpted from a slim neutral. Added to a body with its own pectorals and deltoids, "muscular" stacks on muscle that is already there, and "slim" stays broad-chested. Reads fine at moderate values; at the extremes it adds to condition 1.
4. **Definition finer than the base mesh is lost.** The sharpest tenth of each target is softened (roughness at the 95th percentile 0.53 to 0.83 of the source): the quadriceps' separation and the abdominal lines of MakeHuman's most muscular body come through faint. The base mesh has 10,582 vertices; this detail belongs in a normal-map layer that the muscle parameter blends in, which the renderer doesn't have yet.
5. **The heaviest masculine shapes need room at the armpit.** Our arm and chest sit closer than MakeHuman's. The keep-apart step removes the crossing (114 and 91 pairs to 0) but at a cost: the heavy-and-muscular upper arm grows 53% in girth instead of MakeHuman's 73%, and a dark slot shows at the armpit. Widening the shoulders with the parameter's `bones` would be the better fix. It must be redone in the engine's rest pose (CO4's A-pose), not the pose used here.
6. **The axes are muscle and weight, not fat and muscle.** MakeHuman's data is a 3 × 3 grid of sculpted bodies; "fat" is its low-muscle, high-weight corner. See [Mixing](#mixing-and-the-manifest) for the two ways to put that behind the plan's sliders. `max_sum` isn't needed: the corner where both are at their top is itself sculpted.

**If the lead says no on the look:** Anny would not help, since its shape space is these same targets. The fallback that changes the look is artist-made shapes on our topology, imported; everything here except `register.py` (the mixing, the manifest form, the keep-apart step, the regions, the mask, the packed format) carries over unchanged, and the import pipeline (D8) already plans for artist-made items.

## What was done

Six steps, one script each ([how to rerun](#how-to-rerun)):

1. **Data** (`fetch_makehuman.py`, `extract.py`). MakeHuman's base mesh and 30 targets from its repository at a pinned commit, read as data ([licence](#licence-record)). The realistic male and female bodies from the bundle: 10,582 vertices each, one topology (checked: the same faces over the same vertex indices), base vertices moved onto the sculpt as CO4 vendors them. MakeHuman's base mesh carries its helpers in the same file (clothes helpers, a skirt, hair, eyes, teeth, tongue, genitals, joint cubes); only the `body` group (the first 13,380 vertices, a closed mesh) is used, and the joint cubes' centres as joints.
2. **Pose** (`skeleton.py`). The two aren't posed alike: the bundle's arms hang about 70° below level, MakeHuman's 50° (forearm 41°), and the hands, legs and feet differ. The realistic body is turned to MakeHuman's pose (arms, forearms, hands, legs, feet; a Blender armature with bone-heat weights and a corrective smooth, joints from the face sets' rings), so everything is compared, measured and rendered in one pose that is close to the engine's A-pose. The shapes are turned back to the bundle's pose at the end by each vertex's local rotation between the two poses (up to 38°; turning there and back changes a shape by 0.000 mm).
3. **Correspondence** (`register.py`). MakeHuman's male neutral is wrapped onto the masculine body and its female neutral onto the feminine one: a smooth space warp carries 60 landmarks (joints, finger joints, eyes, crown, crotch, heels, toes) onto ours, then a non-rigid fit pulls the surface on (nearest point facing the same way, both directions, the smoothness relaxed in five stages). It ends 0.9 to 1.0 mm from our surface at the median (95%: 3.1 and 3.9 mm).
4. **Transfer** (`transfer.py`). Each realistic vertex takes the offset of the point of MakeHuman's wrapped surface nearest to it (9,799 skin vertices, all matched), carried by MakeHuman's **local frame** there (the rotation and scale between MakeHuman's own shape and the wrapped one). The 783 vertices inside the head (eye sockets, mouth) follow the nearest skin. Then two fixes: each shape is averaged with its mirror image, and **kept apart**: where a shape makes the body newly cross itself it is scaled down smoothly round the crossing until it doesn't.
5. **Mixing** (`mix.py`), **measuring** (`measure.py`, `validate.py`) and **rendering** (`sheets.py`).
6. **Bony landmarks and regions** (`landmarks.py`).

Simplest first, then only what the numbers asked for (the 16 corner shapes, both frames; errors are differences from MakeHuman's own percentage change, in points):

| Method | Gap to MakeHuman's surface, median mm | Roughness / source: median, 95% | Asymmetry share | Volume change error: mean, worst | Girth change error: mean, worst | New crossings, 16 shapes |
|---|---|---|---|---|---|---|
| `nearest`: landmark warp only, offsets copied | 4.35 | 0.77, 0.75 | 1.7% | 2.01, 5.98 | 1.95, 14.8 | 145 |
| `fit`: + the surface fit | 0.96 | 0.76, 0.67 | 1.0% | 1.60, 4.79 | 1.52, 14.0 | 147 |
| `frames`: + offsets carried by local frames | 0.96 | 0.80, 0.69 | 1.2% | 0.77, 2.44 | 0.97, 6.5 | 208 |
| `symmetric`: + mirrored | 0.96 | 0.80, 0.69 | 0.0% | 0.77, 2.45 | 0.97, 6.4 | 207 |
| `full`: + kept apart | 0.96 | 0.80, 0.69 | 0.0% | 1.09, 4.56 | 1.47, 20.0 | 0 |

The local frames are what make the bodies change as MakeHuman's do (they halve both errors). Keeping apart buys zero crossings with fidelity at the two heaviest masculine corners (the worst girth error, 20 points, is that upper arm). No smoothing was added: nothing called for it.

## Numbers

Per shape at full weight, in MakeHuman's pose. *Places apart*: the distance between our largest move and MakeHuman's, carried to our body. *New crossings*: pairs of triangles the shape makes cross, not counting pairs that already cross or touch within 4 mm in the neutral (closed lips, eyelids, neighbouring toes). *Roughness*: each vertex's offset against its neighbours' mean over the squared edge length, ours over the source's; under 1 is smoother. *Girth*: the tape length round the section, chest, waist, hips, thigh, upper arm; the trunk's levels are set on MakeHuman's mesh and carried over.

| Frame | Shape (muscle, weight) | Largest move mm (MakeHuman's) | Where | Places apart mm | New crossings: before, after keeping apart (MakeHuman's own) | Kept apart: vertices, largest cut mm | Asymmetry before the fix: largest mm, share | Roughness / source: median, 95% | Volume % (MakeHuman's) | Girth % (MakeHuman's) |
|---|---|---|---|---|---|---|---|---|---|---|
| masculine | min, min | 67.1 (66.3) | upper back | 393 | 0, 0 (0) | 0 | 4.96, 0.7% | 0.84, 0.80 | −33.5 (−34.7) | −15, −16, −19, −29, −31 (−13, −18, −18, −31, −34) |
| masculine | min, average | 47.6 (50.0) | waist, front | 10 | 0, 0 (4) | 0 | 4.63, 1.0% | 0.83, 0.78 | +13.0 (+12.2) | +5, +20, +10, +0, −0 (+5, +19, +10, −0, −0) |
| masculine | min, max | 147.1 (141.2) | waist, front | 10 | 114 (armpit 80, crotch and inner thigh 24, trunk 10), 0 (0) | 426, 43.9 | 6.57, 0.6% | 0.82, 0.53 | +84.9 (+88.0) | +28, +63, +32, +31, +26 (+35, +62, +33, +33, +41) |
| masculine | average, min | 22.8 (25.4) | thigh | 13 | 0, 0 (0) | 0 | 0.98, 0.8% | 0.74, 0.63 | −8.6 (−9.1) | +0, −9, −6, −10, −11 (+0, −9, −6, −10, −10) |
| masculine | average, max | 28.9 (28.7) | waist, back | 11 | 2 (armpit), 0 (0) | 50, 3.2 | 2.25, 1.1% | 0.79, 0.67 | +9.6 (+9.6) | +5, +10, +6, +2, +11 (+5, +10, +7, +2, +13) |
| masculine | max, min | 43.2 (41.2) | upper back | 11 | 0, 0 (0) | 0 | 1.35, 0.7% | 0.79, 0.65 | −14.2 (−14.3) | −9, −2, −7, −8, −14 (−10, −3, −7, −9, −16) |
| masculine | max, average | 41.9 (40.1) | upper back | 25 | 0, 0 (0) | 0 | 1.33, 0.9% | 0.78, 0.67 | −5.0 (−5.3) | −2, +3, −3, −1, +3 (−4, +3, −3, −1, +4) |
| masculine | max, max | 79.6 (75.5) | upper arm | 10 | 91 (armpit 54, crotch and inner thigh 37), 0 (11) | 326, 46.6 | 4.24, 0.8% | 0.80, 0.67 | +40.3 (+44.8) | +13, +8, +5, +25, +53 (+21, +8, +4, +32, +73) |
| feminine | min, min | 60.4 (68.6) | pelvis, back | 35 | 0, 0 (28) | 0 | 2.80, 1.1% | 0.59, 0.67 | −34.2 (−36.2) | −4, −16, −26, −42, −34 (−4, −14, −27, −43, −33) |
| feminine | min, average | 31.0 (34.1) | thigh | 19 | 0, 0 (0) | 0 | 1.06, 1.2% | 0.77, 0.74 | +9.3 (+10.0) | +4, +11, +8, +11, +8 (+4, +11, +8, +11, +7) |
| feminine | min, max | 100.5 (97.1) | upper back | 11 | 0, 0 (0) | 0 | 4.70, 1.2% | 0.86, 0.67 | +72.0 (+74.4) | +41, +51, +20, +33, +35 (+46, +52, +21, +33, +34) |
| feminine | average, min | 28.0 (30.1) | thigh | 14 | 0, 0 (0) | 0 | 2.93, 1.6% | 0.84, 0.67 | −15.2 (−15.8) | −3, −6, −7, −13, −11 (−3, −6, −8, −14, −10) |
| feminine | average, max | 21.3 (21.2) | thigh | 39 | 0, 0 (0) | 0 | 5.20, 2.1% | 0.82, 0.61 | +13.6 (+14.2) | +8, +10, +8, +14, +12 (+8, +10, +9, +14, +11) |
| feminine | max, min | 32.0 (33.0) | pelvis, back | 121 | 0, 0 (0) | 0 | 2.10, 1.4% | 0.85, 0.71 | −14.0 (−14.3) | −1, −7, −11, −8, −3 (−1, −8, −12, −8, −3) |
| feminine | max, average | 25.9 (27.2) | thigh | 8 | 0, 0 (0) | 0 | 2.75, 2.4% | 0.77, 0.72 | −2.5 (−2.4) | +4, −3, −5, +7, +3 (+4, −3, −5, +7, +2) |
| feminine | max, max | 64.4 (61.8) | upper arm | 6 | 0, 0 (0) | 0 | 1.72, 0.9% | 0.86, 0.83 | +26.3 (+26.3) | +26, −4, −3, +19, +70 (+28, −4, −4, +20, +70) |

The two large *places apart* (393 and 121 mm) are shapes with two near-equal peaks, where ours and MakeHuman's pick different ones (at MakeHuman's peak the masculine one moves 64.4 mm against its own largest 67.1); the rest agree within 6 to 39 mm, a vertex or three. The six regional targets (pregnant belly, toned stomach, buttocks, pectorals, largest and smallest breast) are in `measurements.json`: volume and girth changes within 2 points of MakeHuman's except the largest breast, which the keep-apart step cut back on both frames because it crossed the body (masculine: 67 mm against 92 mm; feminine: chest girth +21% against +27%).

**The neutrals.** The bundle's own bodies already cross themselves: 61 pairs on the masculine one (53 between toes, 8 in the head) and 133 on the feminine one (107 inside the head, 26 between toes), the same in both poses. That is CO4's to know. Our masculine neutral is 68.1 litres to MakeHuman's 63.3, with a chest girth of 1,007 mm to 905 and an upper arm of 346 mm to 286 (condition 3).

**The correspondence, where the answer is known** (`validate.py`): MakeHuman's male neutral wrapped onto its female neutral by the same code. The two share a topology, so wrapped vertex *i* should land on the female's vertex *i*.

| Part | Vertices | After the landmark warp: median, 95% mm | After the fit: median, 95%, largest mm |
|---|---|---|---|
| all | 13,380 | 6.5, 34.4 | 2.7, 26.6, 59.7 |
| trunk | 1,616 | 23.9, 42.3 | 13.3, 31.7, 38.8 |
| head and neck | 4,434 | 6.0, 11.5 | 2.4, 8.0, 15.8 |
| arms | 1,570 | 4.7, 30.1 | 3.0, 29.2, 41.1 |
| hands | 2,590 | 0.6, 1.6 | 1.2, 2.1, 3.1 |
| legs | 1,096 | 22.7, 49.8 | 20.5, 45.3, 59.7 |
| feet | 2,074 | 7.5, 14.2 | 4.2, 13.9, 22.5 |

After the fit the error across the skin is 1 to 2 mm everywhere; what is left runs along it (on the lower thigh the wrapped mesh sits 26 mm too high, on the lower shin 19 mm too low). A nearest-point fit can't see along a limb, and three joints per leg don't pin it. This test is kinder than the real case in its landmarks and pose and harsher in crossing genders (the two crotch landmarks are 30 mm apart), so take it as the size of the problem, not a bound. Where landmarks are dense (hands, head) the error is 1 to 3 mm.

## What the sheets show

All clay, Cycles, the QA studio's lights with a harder key, orthographic. MakeHuman's own mesh and ours at the same macro settings, both in MakeHuman's pose unless said.

- **[comparison_masculine.png](comparison_masculine.png), [comparison_feminine.png](comparison_feminine.png)**: neutral, heavy, heavy and soft, slim, muscular, heavy and muscular; MakeHuman above ours, front and three-quarter. *Looked at:* whether each column is the same kind of body in both rows. *Saw:* it is, in every column. "Heavy" (weight max at average muscle) is mild in both: MakeHuman's fat body is the min-muscle corner. Our masculine neutral is visibly more muscular than MakeHuman's, and stays so in "slim". Feminine columns match more closely than masculine ones. The masculine heavy-and-soft has a dark wedge at the armpit in the three-quarter view that MakeHuman's doesn't.
- **[details.png](details.png)**: the trunk close up, front and back. This is the sheet to judge the anatomy by. *Good:* heavy and soft: the belly hangs over a crease and the flanks fold at the back in the same place as MakeHuman's; the feminine one has the fold under the breast and the hip shelf. Heavy and muscular, back: the trapezius, the furrow of the spine and the latissimus read as on MakeHuman's. Slim and muscular are clean. *Wrong:* the masculine heavy-and-muscular chest has a hard, straight line under the pectorals that MakeHuman's doesn't (two pectoral edges that don't coincide); the masculine heavy-and-soft has the same line under the chest fat, a crease across the lower belly where MakeHuman has a navel, and no navel; both show the dark slot at the armpits; MakeHuman's abdominal lines and the cut of its thighs are fainter on ours. *Still a blob:* nothing is a featureless inflation. The masculine heavy-and-soft belly is the nearest: rounder and smoother than MakeHuman's, because the navel and the folds round it didn't survive.
- **[grid.png](grid.png)**: our body over five steps of muscle by five of weight, both frames; the tiles at half steps are mixtures. *Looked at:* whether mixtures stay bodies. *Saw:* every tile reads as one; no step jumps. Of 75 grid points (three frame values), one adds a single crossing pair.
- **[bundle_pose.png](bundle_pose.png)**: the same shapes turned back to the bundle's pose. *Saw:* the shapes survive the turn (no shearing at the shoulders). With the arms 20° lower the heavy shapes meet the chest again (dark patches under the arm): keeping apart is per pose.
- **[extras.png](extras.png)**: six regional targets. *Saw:* pregnant belly, buttocks, pectorals and the small breast match MakeHuman's; the largest breast is squarer underneath on ours.
- **[landmarks.png](landmarks.png)**: see [Bony landmarks](#bony-landmarks-and-regions).

Not in the sheets but looked at: faces and hands. The fat face gains jowls and a second chin and the thin one hollows at the cheek as MakeHuman's do (the feminine thin cheek has a slightly hard edge); fat fingers thicken without touching. Toes weren't looked at closely.

## Mixing and the manifest

MakeHuman's rule: a slider splits into min, average and max factors, and each corner target weighs the product of its factors. The manifest's rule (architecture §7): a corner is the product of its factors' spans, each monotone. "Average" (1 − min − max) isn't a monotone span, so the nine bodies per frame are re-expressed as eight shapes: four single-axis ones (`Muscle_Min`, `Muscle_Max`, `Weight_Min`, `Weight_Max`: the corners where the other axis is average) and four interactions (`Muscle_Max_Weight_Max` = that corner minus its two single-axis shapes, and so on). Each gets its frame's share as one more factor (`frame` from `[1, 0]` for masculine, `[0, 1]` for feminine). `mix.py` prints the entries and checks them: over 75 points of frame × muscle × weight the engine's rule and MakeHuman's give the same body to 0.000 µm.

**Cost: 8 shapes per frame, 16 in all**, for muscle × weight at one age. Baked, so no frame time: 16 × 10,582 offsets is 2.0 MB as 32-bit floats in memory per base model (shared by every character), and 0.43 MB packed in the repository. None of the eight can be dropped: the smallest interaction still moves 22.6 mm.

Two ways to put it behind sliders:

| | Parameters | Shapes | Keeps | Loses |
|---|---|---|---|---|
| **A. MakeHuman's grid** | `muscle` and `weight`, −1 to 1, 0 average | 16 | Every sculpted body, exactly | The plan's "fat" and "muscle" names: fat is muscle −1 with weight +1 |
| **B. Layers** | `fat` 0 to 1 (the heavy-and-soft corner), `muscle` 0 to 1 (the muscular one), their corner (the heavy-and-muscular body), `slim` 0 to 1 and its corner with muscle | 10 | The plan's sliders; the bodies people ask for | The two gaunt bodies (muscle min at weight min and average) and the mild "heavy"; fat rises straight to the extreme |

A is what was built and measured. B is the same data with different differences and is the closer fit to D5; it needs no new measurement, only the lead's choice. Either way the 3 × 3 × 4 ages × 2 genders of MakeHuman's full set (72 targets) isn't proposed: age isn't in the plan.

**Joints.** 11 of the 16 corner targets don't move MakeHuman's joints at all, including the most muscular male; the other five move a hip, the spine or a collarbone by 5 to 35 mm, which looks like leftovers. So these shapes need no bone offsets to stay on the skeleton, except by choice (condition 5).

## Bony landmarks and regions

**From the data.** Along the weight slider at average muscle, MakeHuman's artists kept fat off bone. `landmarks.py` measures how far the transferred skin moves along its normal from min to max weight (MakeHuman's joints don't move with these, so it is depth over bone):

| Place | Masculine: fat depth mm (share under 1.5 mm) | Feminine |
|---|---|---|
| Waist | 22.3 (0%) | 20.2 (14%) |
| Pelvis | 14.5 (7%) | 19.0 (0%) |
| Thigh | 7.9 (12%) | 15.0 (3%) |
| Upper arm | 9.5 (2%) | 8.6 (10%) |
| Chest and upper back | 6.1 (10%) | 13.4 (16%) |
| Knee ring | 5.5 (22%) | 6.0 (28%) |
| Elbow ring | 3.6 (15%) | −0.8 (65%) |
| Forearm | 2.1 (42%) | 1.4 (50%) |
| Shin | 0.3 (71%) | 1.2 (52%) |
| Ankle ring | 0.0 (100%) | −0.1 (92%) |
| Hand, foot, head | 0.5, 0.0, 0.0 | 0.0, 0.0, 0.0 |

The mask (`bony`: 1 under 1.5 mm, 0 over 6 mm) is the second pair of columns in [landmarks.png](landmarks.png). It is **a map of where fat goes more than a mask of bones**: shins, ankles, wrists, hands, feet, the skull, the spine's furrow and the sacrum are in it as they should be, and so are whole calves and forearms (MakeHuman puts no fat there at moderate weight); knees and elbows are only partly in it, and the collarbones hardly (4.4 mm masculine). It is blotchy on the masculine chest and behind the thighs and would need smoothing and a look by eye before use.

**MakeHuman's extreme corner ignores it.** From its thinnest to its fattest body at min muscle, the transferred skin over the shins, elbows and knees moves 21 to 22 mm (on MakeHuman's own mesh the front of the shin moves 31 mm). So the mask is for fat we make (a regional layer pushed past MakeHuman's range), not for MakeHuman's corners: the last tile of the sheet applies it to the heavy-and-soft corner and the result is worse (a dented chest, thin calves under heavy thighs).

**From the face sets.** They mark joints, not bone under skin: the rings between sets run round elbows, knees, wrists and ankles, and 72% (masculine) and 85% (feminine) of the ring vertices are bony by the data, but they cover only 19% and 29% of what the data marks on the limbs, and nothing on the trunk. Not enough alone; useful as the regions' borders.

**Regions.** CO5's regional layers come from the same shapes: smooth weights per vertex (chest, back, abdomen, hips, arms, thighs, calves, the rest) from the face sets, adding up to 1, so the regional shapes add up to the global one exactly. The sheet's bottom row shows the regions and "abdomen only" and "hips and thighs only" of the heavy-and-soft corner: both read as bodies. In the manifest that is corners with `muscle_<region>` and `weight_<region>` as factors and `follow` constraints from the global sliders; the regional shapes are sparse, so eight regions cost about one and a half times the data of the global set, not eight times.

## Licence record

Full record: [shapes/LICENCE-RECORD.md](../../../../assets/characters/transfer/shapes/LICENCE-RECORD.md). In short:

- **Source:** `https://github.com/makehumancommunity/makehuman`, `master` at commit `a8bc2d54ff0ac92e78ff71431b1023eda42bf482` (2024-06-26), a partial sparse clone holding only `LICENSE.md`, `LICENSE.ASSETS.md`, `makehuman/data/3dobjs/base.obj` and `makehuman/data/targets/` (136 MB on disk, in scratch, not committed).
- **Read:** 33 files: the two licence files, `base.obj`, 6 ethnic and 18 "universal" macro targets at the young age, 6 regional targets. Their SHA-256 are in [makehuman.sha256](../../../../assets/characters/transfer/makehuman.sha256); `fetch_makehuman.py DIR --check` verifies them.
- **CC0, verified in the download:** `LICENSE.md` lines 52 to 61: "The assets are defined as any data contributing to the graphical output of MakeHuman. This includes: * The base mesh and proxies * Targets and modifiers …" and "These assets have been released under CC0 1.0 Universal." Each of the 31 data files also says on its lines 3 and 4: "This asset was explicitly released as CC0 in september 2020." The research note's sentence ("All core assets are shared under … CC0") is from MakeHuman's website; the repository agrees with it.
- **No code:** the source is AGPL (`LICENSE.md` lines 23 to 29). None was fetched, read or adapted in this spike. The two formats were read from the files; the mixing rule was written from the research note's statement of it.
- **Not used:** MPFB2's pack and Anny's `data/mpfb2` (the same targets), MakeHuman's rig and weight files (not named in the licence's list of assets), third-party assets.
- **Residual risk:** CC0 is a waiver by MakeHuman's stated copyright holders; like every CC0 source it "disclaims responsibility for clearing rights of other persons" (`LICENSE.ASSETS.md`, 4c).

## Vendoring proposal

Commit **derived data only**, as CO4 does for the sculpt: one packed file of the shapes we use on our topology, with the scripts and the pinned source that make it.

- `assets/characters/realistic/source/body_shapes.npz` (in CO4's package once it exists): per shape, the indices of the vertices it moves and their offsets as 16-bit steps of 0.02 mm, deflated. **0.43 MB for the 16 corner shapes** (0.46 MB with the six regional ones); the largest difference from the unpacked shapes is 0.05 mm, the cut-off under which a vertex is left out. The spike's copy, on the bundle's pose, is [shapes/body_shapes.npz](../../../../assets/characters/transfer/shapes/body_shapes.npz).
- Beside it the licence record and `makehuman.sha256`; the transfer scripts stay in `assets/characters/transfer/`. MakeHuman's own files (7.3 MB for the base mesh and the 24 targets that feed the corners, 136 MB for the tree) are never committed: `fetch_makehuman.py` fetches and verifies them.
- The build reads the packed file, turns the shapes to the rest pose (`transfer.reframe`), re-expresses them (`mix.engine_shapes`) and writes them as blend shapes of `Body`. In the glTF file 16 dense position targets are 2.0 MB (4.0 MB with normals); sparse accessors would be about 40% of that. Regenerating the packed file needs Blender and the fetch; building from it needs neither MakeHuman's files nor the network.

## Open problems

1. Landmarks on the trunk and limbs (condition 2). Hand-picking a dozen pairs once is an hour's work and is what a wrapping tool's user does.
2. The armpit (condition 5): widen the shoulders with bone offsets, or keep apart in the rest pose and accept the thinner arm.
3. Muscle definition as a normal-map layer (condition 4): a renderer and CO4 question.
4. The masculine base is athletic (condition 3). An option worth a look: offer the base at muscle −0.3 or so as the default, so the sliders have room both ways.
5. The mask needs cleaning, and a decision on whether the extreme corner is cut back with it.
6. Carrying shapes to `Body_LOD1` and `Body_LOD2`: not tried.
7. Shapes against CO4's skinning: a fat body on weights painted for the neutral wasn't tested in any bent pose. The plan's corrective shapes and helper bones are where that lands.
8. The feet's toes and the mouth's inside were only checked for crossings.
9. The pose here is MakeHuman's (arms 50° down), close to the A-pose (45°) but not it; nothing was run against CO4's package, which didn't exist yet.

## What CO5 needs from CO4's package

- **The bundle's vertex order, unchanged**, for `Body` (10,582 vertices). If CO4 must change it (a welded head, added loops), a per-vertex map from the bundle's order.
- **Both frames' rest positions on that order, before any decimation**, and the masculine frame's faces. `reframe` turns the shapes to them from the bundle's pose; scale is handled (it carries the local scale too).
- **The face sets** as vendored (`fs_<id>` materials): landmarks, the inside of the head, the regions and the joint rings all come from them. The two bodies' sets differ on a few faces; the spike uses the masculine one's, as CO4 does.
- **The mirror pairing** CO4 symmetrizes with. Position-nearest pairing fails on two vertices of either body alone; pairing over both frames at once works (`geom.mirror_map`).
- **The skeleton's joints** in the rest pose, to keep apart there and to try shoulder offsets.
- **How LODs are made**: vertex subsets of `Body` would let shapes carry over by index.
- UVs aren't needed for the transfer. They are for a muscle-definition normal layer.

## How to rerun

`PY` is a Python with the `bpy` module (5.3) and Pillow, always with `-I`; `WORK` and `MH` are scratch folders outside the repository. From `assets/characters/transfer/`:

```sh
python3 fetch_makehuman.py MH                                   # 136 MB, verifies 33 SHA-256
PY -I extract.py --bundle path/to/human_base_meshes_bundle.blend --makehuman MH --work WORK
PY -I register.py --work WORK                                   # about 100 s
PY -I transfer.py --work WORK --pack shapes/body_shapes.npz     # 8 s
PY -I mix.py --work WORK                                        # the mixing check and the manifest entries
PY -I measure.py --work WORK --out ../../../docs/development/character-studio/co5-spike/measurements.json --methods
PY -I validate.py --work WORK                                   # the known-answer check, 50 s
PY -I sheets.py --work WORK --out ../../../docs/development/character-studio/co5-spike
PY -I landmarks.py --work WORK --out ../../../docs/development/character-studio/co5-spike
```

Everything is deterministic (fixed seeds, no randomness in the fit). The modules without a command of their own: `mh.py` (MakeHuman's two file formats and its factors), `geom.py` (mesh maths; a small conjugate-gradient solver, since the environment has no SciPy), `skeleton.py` (landmarks, the re-pose), `render.py` (the clay studio).
