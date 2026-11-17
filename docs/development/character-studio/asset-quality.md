# Character asset quality standard

Paul's decision of 2026-11-17 (plan section 0, decision 10): hair and clothing must be **production-quality assets that visually belong alongside the character models**. "Clean and well made" is not the bar. A generated or imported asset is a **draft** until it passes every requirement below on the benchmark cast, in the engine, and in review.

An item manifest records it as `quality: "draft"` or `"production"` (plan 11.3). Character Studio hides drafts unless "Show drafts" is on. Drafts never appear in default outfits, presets or randomization, and a test holds that.

Numbers marked *(proposed)* are starting limits, set from the stylized items' measurements and the references. They are tightened as items improve and are **never loosened to pass an item** (plan section 6).

## 1. How an asset is reviewed

| Step | What | Evidence kept |
|---|---|---|
| Automated geometry checks | The Blender harness (`measure.py`, the fit checks) and `avatar`'s manifest checks, per section 3 | `measurements.json`, the check output |
| In-engine capture matrix | `capture_characters` with the benchmark **cast** (plan 11.6) of **both frames**, at least **five body types each** (average, slim, athletic, heavyset, muscular, plus the slider extremes the item supports), in **six views** (front, three-quarter, side, back, face, face-front), under **studio, hard, warm and outdoor** light, in the **rest pose, Idle and a walk frame**, and in the body suite's poses (T-pose, arms raised, elbows and knees bent) | Contact sheets in the milestone's folder |
| Close review | Full-size single pictures cropped to the item's edges (hairline, hems, collars, cuffs, soles), at 3840 × 2160 where detail matters | Crops beside the sheets |
| Sign-off | The lead reviews every item. Paul signs off the first item of each kind (the first hairstyle, the first top, bottom, shoe and accessory) before more of that kind are made | A line per item in the milestone's README |

An item passes only when **every** check of its kind passes **and** the review finds no visible defect. A defect seen in a picture fails the item even if every number passes.

## 2. Requirements for every asset

| Area | Requirement | Measured by |
|---|---|---|
| Geometry | No degenerate or zero-area triangles; no non-manifold edges in closed parts; normals consistent (no flipped faces); no surface seen from both sides unless it is a card with a two-sided material | Blender harness |
| Topology | Mostly quads before triangulation; edge loops round every joint the item covers; triangle budget per tier (plan section 7, and section 4 below) | Blender harness: counts per LOD |
| UVs | No overlaps except deliberate mirroring; stretch under 10% on 95% of the area *(proposed)*; texel density at least 8 px/cm at 2K for clothing and 10 px/cm for the scalp area of hair *(proposed)* | Blender harness |
| Skinning | At most four influences per vertex, normalized; no vertex weighted to a bone more than one joint away from where it sits; joint area ratios in the deformation poses at least the body's own limits (`thresholds.json`) | `measure.py` deformation poses |
| Shading | MikkTSpace tangents; no visible normal-map seams in any view; material on the right shading model (Cloth, Hair or Standard, ADR-0036); colours and roughness inside the material's declared ranges; reads the same under the four lights (no baked lighting in albedo) | Capture matrix; review |
| Clipping | No body or under-layer through the item in any capture of the matrix | Section 3 numbers and the review |
| Fit across bodies | Holds on every cast body and at every body parameter's ends the item declares it supports | Section 3 numbers |
| LODs | Each LOD keeps the silhouette and stays clear of the body (no skin through a simpler garment: audit §3.10) | LOD sheet; fit check per LOD |
| Licence | Class 1, 2 or 5 only, recorded in `source` and in [references/README.md](references/README.md) | Manifest check |
| Performance | GPU time per character within the tier budget (plan section 7) on the reference PC, checked on the minimum class | `FramePass` timers, crowd scene |

## 3. Clothing

| Requirement | Limit | Measured by |
|---|---|---|
| Construction | Panels follow a real pattern (FreeSewing or equivalent); seams at pattern seams as geometry or a baked normal with visible stitching; hems turned with visible thickness (at least 2 mm); collars, cuffs, waistbands and closures (buttons, zips, drawstrings) modelled; no single-layer edge visible as a paper-thin line | Review against [references/clothing](references/clothing/README.md) |
| Folds | Folds from a cloth simulation of the sewn panels, baked to the normal map at least; compression folds at elbows, knees, waist and the back of the knee in the bent poses; no folds painted that contradict the drape | Review in the bent poses |
| Materials | Fabric reads as its fabric at arm's length: denim twill, jersey knit, fleece nap, leather grain (Cloth shading: sheen, anisotropy where the fabric has it); colour roles primary, secondary and accent declared where the item has them | Review under the four lights |
| Poke-through | 0 body vertices outside the garment by more than 1 mm in covered regions, at rest and in every pose of the body suite, on every cast body and at the supported parameter extremes *(proposed)* | Fit check |
| Layers | Clearance between layers at least the item's `fit.margin`; no inner layer through an outer one | Fit check |
| Sleeves and waist | A sleeve hem stays on the arm through arms raised; a waistband stays closed round the waist in the walk and the hip flex; no gap to skin at the waist | Fit check and review |
| Footwear | No foot through the sole or upper in any pose; sole flat on the floor at rest (within 2 mm); toe box deforms only at the toe joint | Fit check |
| Stability in motion | No popping or tearing between consecutive frames of Idle and Walk (vertex displacement continuous) | Motion capture check |
| Budgets *(proposed)* | LOD0: top 6k, jacket or coat 10k, bottom 6k, shoes 4k a pair, accessory 2k triangles; textures 2K (hero 4K) | Counts |

## 4. Hair

| Requirement | Limit | Measured by |
|---|---|---|
| Silhouette | The style's outline matches its reference silhouette in front, side and back (the reference picked in [references/hair](references/hair/README.md)); no hollow or "helmet" outline | Review side by side with the reference |
| Volume | Hair stands off the scalp by at least the style's declared minimum thickness everywhere it covers (for example 8 mm for a crop, 20 mm for medium styles) *(proposed)* | Rays from the scalp's normals |
| Scalp coverage | Scalp show-through under 2% of the covered region's pixels in front, side, back and top views, except where the style declares a parting, a shaved area or thinning *(proposed)* | Capture: masked pixel count |
| Hairline | A density ramp over at least 8 mm at the front hairline and temples, with fine flyaway cards; no hard edge or straight line where hair meets skin | Review at face-front, close |
| Card or strand quality | No visible card edge or straight alpha cut at 1080p in any view; strand-like detail in the atlas (clumps, tapered ends, flyaways); no stair-stepping or shimmer under TAA (frame-to-frame variance of hair pixels under a limit set at the first sign-off) | Review; a TAA stability capture |
| Material response | Dedicated hair shading (CO7 Hair model): a primary and a shifted secondary highlight along the strands, no plastic sheen; colour by melanin and redness, with roots and highlights where declared | Review under the four lights |
| No slab shadows | Self-shadowing reads as many strands, not as solid sheets | Review under hard and outdoor light |
| Motion | 0 hair vertices more than 1 mm inside the head in sampled frames of Idle, Walk and Run; spring-boned parts settle without jitter | Motion capture check |
| Budgets (plan section 7) | Cards up to 40k triangles at hero LOD, 25k at gameplay LOD0, 3k at crowd LOD; strands optional (section 0, decision 3) | Counts; GPU timers |

## 5. What a draft may and may not do

| | Draft | Production |
|---|---|---|
| Listed in Character Studio | Only with "Show drafts" on, marked "Draft" | Yes |
| In default outfits, presets, randomization | No (a test holds this) | Yes |
| Shipped in the library | Yes, so it can be reviewed and improved | Yes |
| Counted toward a milestone's "Done when" | No | Yes |
