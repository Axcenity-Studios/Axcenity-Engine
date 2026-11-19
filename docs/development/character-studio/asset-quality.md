# Character asset quality standard

Paul's decision of 2026-11-17 (plan section 0, decision 10): hair and clothing must be **production-quality assets that visually belong alongside the character models**. "Clean and well made" is not the bar. A generated or imported asset is a **draft** until it passes every requirement below on the benchmark cast, in the engine, and in review.

Every item is in one of three **content states** (Paul's decision of 2026-11-18, plan section 0, decision 12; section 5 below): **Production** (passed this standard and the review, available by default), **Legacy** (older items kept for compatibility and optional use: the stylized family's items made before this standard), **Draft** (incomplete: never in normal character creation). Randomize draws Production only. Legacy is hidden by default and shown through an optional content filter. A character that wears a Legacy or Draft item still loads and draws it.

**How items are made** (decision 11): licensed assets (CC0, CC BY) are the base, Blender and manual modelling refine them, Blender scripts automate import, fitting, rigging, validation and export, and procedural generation only supports variations and repetitive work (section 6).

Numbers marked *(proposed)* are starting limits, set from the stylized items' measurements and the references. They are tightened as items improve and are **never loosened to pass an item** (plan section 6).

## 1. How an asset is reviewed

| Step | What | Evidence kept |
|---|---|---|
| Automated geometry checks | The Blender harness (`measure.py`, the fit checks) and `avatar`'s manifest checks, per section 3 | `measurements.json`, the check output |
| In-engine capture matrix | `capture_characters` with the benchmark **cast** (plan 11.6) of **both frames**, at least **five body types each** (average, slim, athletic, heavyset, muscular, plus the slider extremes the item supports), in **six views** (front, three-quarter, side, back, face, face-front), under **studio, hard, warm and outdoor** light, in the **rest pose, Idle and a walk frame**, and in the body suite's poses (T-pose, arms raised, elbows and knees bent) | Contact sheets in the milestone's folder |
| Close review | Full-size single pictures cropped to the item's edges (hairline, hems, collars, cuffs, soles), at 3840 × 2160 where detail matters | Crops beside the sheets |
| Required views and motions (decision 13) | Front, side and rear; the neutral standing pose; the walk (a top passes the **walking** fit test, not only the static one); arm movement with shoulder deformation (arms raised, arms forward, the walk's swing). No visible clipping, tearing or mesh separation in any of them | Sheets per view and motion; the fit gate's walk run (since wave 5: the whole stride, every third frame of frames 2 to 44, on every gate body, and the arms raised forward to level, `HeldPose::ArmsForward`) |
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
| Licence | Class 1, 2 or 5 only ([references/README.md](references/README.md)): CC0 or public domain; CC BY 3.0 or 4.0, MIT or Apache-2.0 with the attribution recorded; original work. **No** share-alike, non-commercial or no-derivatives licence. An imported asset has a **provenance record** (source page, author, licence and version, the licence's page, retrieval date, hashes of the files as downloaded, what was changed, the attribution line) before any file of it is committed; a game built with Axcenity carries the attributions of the items it ships | Manifest and provenance check |
| Performance | GPU time per character within the tier budget (plan section 7) on the reference PC, checked on the minimum class | `FramePass` timers, crowd scene |

## 3. Clothing

| Requirement | Limit | Measured by |
|---|---|---|
| Construction | Panels follow a real pattern (FreeSewing or equivalent); seams at pattern seams as geometry or a baked normal with visible stitching; hems turned with visible thickness (at least 2 mm); collars, cuffs, waistbands and closures (buttons, zips, drawstrings) modelled; no single-layer edge visible as a paper-thin line | Review against [references/clothing](references/clothing/README.md) |
| Folds | Folds from a cloth simulation of the sewn panels, baked to the normal map at least; compression folds at elbows, knees, waist and the back of the knee in the bent poses; no folds painted that contradict the drape | Review in the bent poses |
| Materials | Fabric reads as its fabric at arm's length: denim twill, jersey knit, fleece nap, leather grain (Cloth shading: sheen, anisotropy where the fabric has it); colour roles primary, secondary and accent declared where the item has them | Review under the four lights |
| Poke-through | 0 body vertices outside the garment by more than 1 mm in covered regions, at rest and in every pose of the body suite, on every cast body and at the supported parameter extremes *(proposed)* | Fit check (`player` `fit_gate_tests::poke_through`: a body corner the garment covers at rest is out by how far it is past the nearest cloth along that cloth's normal; where that says out, but the way out along the corner's own normal meets the garment from inside within 5 cm, the garment encloses it and it is inside: another part of a loose garment, a sleeve beside the chest, came nearer than the cloth over it. From inside means both the cloth's smooth normal there and its posed triangle's face normal point along the way out (`from_inside`): a fold flipped in a pose, its back face towards the skin, is the cloth's outside, and the skin under it counts out. Wave 5, the face condition since its review fixes 2; the closest point alone, `AXCENITY_FIT_GATE_CLOSEST_ONLY=1`, is for comparison only) |
| Layers | Clearance between layers at least the item's `fit.margin`; no inner layer through an outer one | Fit check |
| Sleeves and waist | A sleeve hem stays on the arm through arms raised; a waistband stays closed round the waist in the walk and the hip flex; no gap to skin at the waist | Fit check and review |
| Footwear | No foot through the sole or upper in any pose; sole flat on the floor at rest (within 2 mm); toe box deforms only at the toe joint | Footwear check (`player` `garment_checks::footwear`: a foot corner the shoe covers at rest, a ray out along its normal meeting the shoe, is out when that ray meets no shoe in the pose; the tread is the shoe's downward faces within 1 cm of its lowest point. Since wave 5, in every pose, rest included: a corner the shoe doesn't cover with the shoe's outside right under it, nothing of the shoe over it, is **exposed** (a toe out of the toe box at rest, which the cover rule can't follow); and the **toe box** (each shoe's front quarter) may change no two of its corners' distance apart by more than 1 mm, so it bends only at the toe joint behind it) |
| Stability in motion | No popping or tearing between consecutive frames of Idle and Walk (vertex displacement continuous) | Motion check (`player` `garment_checks::motion`, Idle, Walk and Run frame by frame: no garment corner jolts, its second difference, more than 1.25 times the most the skin within 10 cm of it does plus 2 mm; corners made at one place stay within 0.2 mm) |
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
| Motion | 0 hair vertices more than 1 mm inside the head in sampled frames of Idle, Walk and Run; spring-boned parts settle without jitter | Motion check (`player` `garment_checks::motion`) |
| Budgets (plan section 7) | Cards up to 40k triangles at hero LOD, 25k at gameplay LOD0, 3k at crowd LOD; strands optional (section 0, decision 3) | Counts; GPU timers |

## 5. Content states: what each may and may not do

| | Draft | Legacy | Production |
|---|---|---|---|
| Listed in Character Studio | Not in normal character creation; only for content review, marked "Draft" | Only with the Legacy content filter on, marked "Legacy" | Yes |
| In randomization | No (a test holds this) | No | Yes |
| In default outfits and presets | No | Only kept for compatibility (the stylized family's defaults and presets; the realistic body type's underwear) | Yes |
| Loads and draws when a saved character wears it | Yes | Yes | Yes |
| Shipped in the library | Yes, so it can be reviewed and improved | Yes | Yes |
| Counted toward a milestone's "Done when" | No | No | Yes |

An item moves to Production only by passing every check of its kind and the review (section 1); the first of each kind also needs Paul's sign-off.

How it is built ([ADR-0043](../../../wiki/Architecture/Architecture-Decisions/ADR-0043-Three-Content-States-Production-Legacy-Draft-In-The-Item-Manifest.md)): the item manifest's `quality` says `production`, `legacy` or `draft`, and every shipped item says it (a check holds this); a manifest with none (an older project's copy) reads as Legacy. Character Studio's "Legacy items" filter and its View menu's "Show drafts for content review" are off by default. Randomize leaves a slot with no Production item as it was, so it never strips a character. A default outfit or preset made of Legacy items is a compatibility case, not Production content. Today: the stylized family's items and the realistic underwear are Legacy, the four wave-4 items Draft (decision 13), nothing Production.

## 6. How production items are made (decision 11)

| Method | Responsibility |
|---|---|
| CC0 / CC BY assets | The base: clothing, footwear, hairstyles, reference meshes |
| Blender and manual modelling | Refinement, topology, fitting, styling, final polish |
| Blender scripts | Import, retopology help, fitting, rigging, validation, export |
| Procedural generation | Variations, adjustments, repetitive tasks |

The wave-4 pipelines (the hair groom and card bake, the FreeSewing ports and drape, the fit) stay as tools for the last two rows; they are no longer the route to a Production item. No batch of items is made until the first of its kind passes. Where no licensed base is usable, the item is modelled from scratch in Blender against reference photographs found through Openverse (decision 14); references are recorded by link, author and licence and never shipped, and brand designs are never modelled.

The Blender-scripts row is the intake (`assets/characters/realistic/import/`, wave 5): it refuses a licence not accepted, records the provenance, imports, reports and repairs topology, places or poses the asset onto the rest pose, weights it, makes the levels, runs section 2's checks per level, fits it with the engine's own import and exports a **draft** with its provenance record. It does not retopologize or rework a garment's construction: that is the second row, by a person. Its optional `refine` step (wave 5) adds construction details to the source's own cut by script (texture coordinates laid out again, a band split off along an edge, turned hems with thickness, maps baked with seams and stitching) and lists each in the provenance record; it doesn't change the cut, and an item it refines is held to sections 2 and 3 like any other.
