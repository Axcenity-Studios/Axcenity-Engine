---
title: "ADR-0044: A garment that was not made on the body comes through an intake before the fit"
type: adr
adr: 44
adr_status: Superseded
date: 2026-11-24
supersedes:
superseded_by: ADR-0046
affects: [avatar, assets]
source: [crates/avatar/src/fit/import.rs, crates/avatar/src/fit/transfer.rs, crates/avatar/src/fit/glb.rs]
docs: [docs/development/character-studio/architecture.md, assets/characters/realistic/clothing/README.md, docs/development/character-studio/progress.md]
tags: [adr, characters, assets]
---

# ADR-0044: A garment that was not made on the body comes through an intake before the fit

- **Status:** Proposed (built on 2026-11-24, waiting for Paul's review of the result and of the questions under "Open").
- **Date:** 2026-11-24
- **Affected systems:** Character Data (`avatar::fit`: a new `intake` module, two new `FitOptions`, texture maps in the fitted model), Assets (`simplify_triangles`), the realistic wardrobe (`assets/characters/realistic/clothing/generated/`).

## Context and problem

The fit ([ADR-0035](ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md)) takes a garment **made on the body**: one layer of cloth, in the body's rest pose, at its size, at most a few centimetres off it. The first realistic wardrobe was made that way by scripts (patterns sewn and draped in Blender) and did not reach the standard in a wave (`asset-quality.md`); who makes the content was left to Paul.

Paul's answer (2026-11-24) is 33 garments from a text-to-3D tool. They look right and none of them is made on the body. Each is:

- in its own size and place (every file is about 1.9 units along its longest side, centred on the origin);
- in its own pose (arms 10 to 20 degrees from hanging, where the body's rest pose has them at 45);
- cut for a body that isn't ours (most for a feminine one);
- a closed surface, not a layer: a shell several millimetres thick with an inner wall, or a solid block with a shut pit at each opening;
- about 30,000 triangles, five times the budget, with three 4K maps.

Run through the import as it was, one stayed 1.9 m tall round the hips.

## Decision

1. **An intake stage, `avatar::fit::prepare`, runs before `fit_garment`** when an import asks for it (`ImportRequest::intake`). It makes the garment what the fit expects and changes nothing for a garment made on the body. The fit itself is the same code for both.
2. **The garment is read from level cross-sections** (240 planes): its trunk (the run of rings round its middle line with the most cloth), the sleeves beside it and legs under it, the underarm and crotch heights, and whether it is hollow. A ring is measured as a tape measure lies (its convex hull), so an open front, a zip slit and a curved hem read sensibly.
3. **One wall.** A loop with an odd count of loops round it is the inside of a tube: those points, and the floors of the pits, come off. A hollow's loop must be round and a real share of its ring, so the air under a pocket flap or between folds is left alone. Bands of cloth (straps, open fronts) are told apart by which way they face.
4. **Size and place follow how the garment is worn** (`GarmentKind`, guessed from the reading or given): a top hangs from the shoulders and is sized by its sleeves reaching the wrists, or by its chest with an ease; trousers sit at the crotch and are sized by their legs reaching the ankles, or by the seat; a skirt sits at the waist; a one-piece hangs from the shoulders with its hems at the ankles. Every guess can be overridden (`kind`, `ease`, `scale`, `nudge`), because a size read from a shape is a guess.
5. **The body goes to the garment's pose, then the garment comes back to the body's.** The body's arms and legs are aimed at the garment's cuffs and hems to place it; then each sleeve and leg is turned back about its joint, each corner by its share of the limb (1 on the limb, 0 on the trunk, settled between them over the shoulder). A garment cut for another body (`cut_for`, the feminine frame) is placed on that body and carried to the default one.
6. **Two fit options for garments that came this way**, both off by default so nothing made on the body changes: `smooth_shapes` (each carried shape's moves are smoothed over the cloth: an uneven binding otherwise pulls a loose garment into spikes) and `push_reach` (how deep inside the body a corner is still found: 12 cm in place of 3).
7. **The import carries the source's texture maps** (`texture_size`: base colour, normal, metal-roughness, resized and stored as JPEG) and **thins the mesh to a budget** (`triangles`, with `assets::simplify_triangles`).
8. **Source files stay outside the repository.** `clothing/generated/garments.json` lists them with their hints and `import_generated.py` imports them into a library given on the command line.

## Alternatives considered

- **Fix each file by hand in Blender** (scale, pose, delete the inner wall, decimate): right for one hero garment, not for a wardrobe, and it leaves "import a garment" out of the engine. Paul's own design note asks for an import that aligns and fits automatically with controls to adjust.
- **Non-rigid registration of the garment to the body.** Fits snug cloth well and shrinks loose cloth onto the body: a sweatshirt would lose its cut. The intake keeps the garment's own shape and only sizes, places and poses it.
- **Keep both walls.** Works for some, but the pits' floors then lie on the skin inside every neckline, and the inner wall is what the body pushes on while the outer wall is what is seen.
- **Bind to the posed body and skin back** (the first attempt). A hanging sleeve's lower half bound to the hip beside it and stayed there when the arm went back to rest. A limb's share read from the garment's own structure doesn't depend on what is nearby.

## Consequences

- 30 of the 32 garments come out in the engine, at rest, as the garment they are, in place and at a believable size (reviewed at contact-sheet size); two fail (a pleated maxi dress and a pantsuit, both made of loose sheets the reading can't follow). All are drafts, and none has been through the player's fit gate.
- A fitted item is 11 to 43 MB (the carried shapes are most of it), so a wardrobe of 32 is about 650 MB: too much for the repository as it stands.
- Layering between imported items is only what `--under` gives (the underwear): a top's hem and a pair of trousers imported apart can cross.
- The reading assumes a garment upright, facing +Z, in a relaxed pose. A T-pose sleeve reads as no sleeve (it is then placed but not turned).

## Amended the same evening (2026-11-24, after Paul's second review)

Four steps were added to the intake, each for a defect measured on the fitted garments (`docs/development/character-studio/architecture.md` §9 has the rules and numbers):

- **The cuts are closed** (`fit/mend.rs` `close_cuts`): a sleeve modelled hanging shares a wall with the garment's side, so cutting the joins left the sleeve open along its underside and the side open under the arm. Each is closed to itself.
- **Centre** (`intake/centre.rs`): each sleeve and trouser leg is brought round its limb ring by ring. Turned as one straight piece, a sleeve bent at the elbow lay beside the forearm, and the arm came out through it.
- **Mend** (`fit/mend.rs`): every open ring the body doesn't come through is a hole and is filled between its own corners. The source meshes come with 5 to 49 small holes, and taking off the inner wall, thinning and the cuts add more; 495 were filled over the 32 garments.
- **Weld** (`intake/weld.rs`): normals are made smooth and corners at one place with the same texture coordinates become one, 26% fewer corners over the 32 garments (12% to 42%).

Floating scraps (a piece with no corner near the largest) are taken out by the tidy step whatever their size.

## Open (for Paul)

- ~~The licence of the generated files.~~ Decided 2026-11-24: the models are his; the fitted items are published as CC BY 4.0, credited to Axcenity (`garments.json`).
- ~~Where the wardrobe ships from, given its size.~~ Decided 2026-11-24: a repository of its own ([Axcenity-Wardrobe](https://github.com/Axcenity-Studios/Axcenity-Wardrobe)) that garments are downloaded from into a project's library by its `install.py`; none is hosted in the engine's repository.
- Which parts of `asset-quality.md` §3 apply to generated garments (pattern construction, quads and edge loops can't be met by them).
