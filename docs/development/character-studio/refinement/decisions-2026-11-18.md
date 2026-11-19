# Paul's decisions of 2026-11-18: content pipeline and content states

Paul's answers to the questions the wave-4 merge raised (progress.md, 2026-11-18), kept as he gave them. The plan records them as section 0, decisions 11 to 13; [asset-quality.md](../asset-quality.md) applies them.

> For the character creator, I'd make the following decisions. I would also change the content pipeline rather than spend another wave trying to force the current procedural generation approach to produce production-quality clothing.
>
> **1. How content gets made**
>
> Recommendation: Hybrid approach. Use licensed assets as the foundation, hand-made assets for quality, and Blender scripts for automation.
>
> I'd divide responsibilities like this:
>
> | Method | Responsibility |
> |---|---|
> | CC0 / CC-BY assets | Base clothing, footwear, hairstyles and reference meshes |
> | Blender/manual modeling | Refinement, topology, fitting, styling and final polish |
> | Blender scripts | Importing, retopology assistance, fitting, rigging, validation and export |
> | Procedural generation | Supporting variations, adjustments and repetitive tasks |
>
> I would stop treating fully procedural Blender-generated clothing as the primary production method.
>
> The current results demonstrate that it isn't reliably achieving the quality standard. That doesn't mean abandoning the scripts; it means using them where they provide actual value.
>
> Important: Every imported asset needs license verification, attribution tracking where required, and compatibility with Axcenity's distribution model.
>
> **2. Older stylized items**
>
> Recommendation: Keep, but classify as legacy. I would not mark the older stylized assets as drafts if they were previously completed and usable.
>
> Instead, introduce three content states:
>
> - Production: Approved assets available by default.
> - Legacy: Older assets retained for compatibility and optional use.
> - Draft: Incomplete assets that cannot appear in normal character creation.
>
> For the character creator:
>
> - Randomize should only select Production assets.
> - Legacy assets should be hidden by default.
> - Users should be able to enable legacy content through an optional content filter.
> - Existing projects using legacy assets must continue loading correctly.
>
> This is particularly important given the decision to move Axcenity toward a semi-realistic character direction rather than the previous stylized direction.
>
> **Additional requirement before sign-off**
>
> I would establish a stricter acceptance standard for the next content wave.
>
> None of the four current items should be promoted to Production.
>
> | Asset | Required correction |
> |---|---|
> | Hairstyle | Actual hair geometry, natural volume and clean silhouette |
> | T-shirt | Proper garment construction, clean back and deformation-resistant fit |
> | Joggers | Clean cuffs, consistent geometry and proper leg deformation |
> | Sneakers | Correct footwear silhouette, clean toe construction and proper foot fit |
>
> Additionally, the T-shirt must pass the walking animation fit test, not just the static character fit test.
>
> I would require:
>
> - Front, side and rear inspection.
> - Neutral standing pose.
> - Walking animation.
> - Arm movement and shoulder deformation.
> - No visible clipping, tearing or mesh separation.
>
> My overall recommendation: Move the next wave toward a licensed-asset-first production pipeline, with manual quality control and automated Blender tooling. Don't keep generating batches of assets until the underlying quality problem is solved.
>
> The character creator is an important part of Axcenity's first impression. I would rather have 10 excellent clothing options than 100 mediocre ones.

## Later the same day: stage 3

After the wave-5 stage-2 report (the fit shrink-wraps loose garments; no usable licensed hair or sneaker without an account), Paul approved the stage-3 proposal (fit and deformation for loose clothing first) and added:

> Ok go ahead and move to the next phase. use references and make models from scratch if neccesary using image sources from openverse.

The plan records this as decision 14. The lead applies it so: where no licensed base is usable (today the hairstyle and the sneakers), the item is modelled from scratch in Blender against reference photographs found through Openverse (openverse.org, which indexes openly licensed images). References are looked at, never shipped: a reference image is not copied, traced into a texture or committed; its link, author and licence are recorded in the item's references list. CC0 and CC BY references are preferred; brand designs (logos, trademarked toe caps, stripes and swooshes) are never modelled.

## How the lead applies them

- **Licences:** "CC0 / CC-BY" is read with the existing classes ([references/README.md](../references/README.md)): class 1 (CC0, public domain) and class 2 (CC BY 3.0 or 4.0, MIT, Apache-2.0, with attribution recorded). Share-alike (CC BY-SA), non-commercial (NC) and no-derivatives (ND) licences are not accepted: a game made with Axcenity ships the assets under its own terms, and an asset is always modified (fitted, rigged, re-materialed). "Compatible with the distribution model" means a game built with Axcenity carries every CC BY attribution of the items it ships, without the game's author having to find them.
- **Provenance before anything else:** an imported asset has a provenance record (source page, author, licence and its version, the licence's own page, retrieval date, file hashes, what was changed, the attribution line) before any file of it lands in the repository; a check fails a shipped item without one.
- **"Hand-made"** in this repository means modelled in Blender by a person, or edited there by scripts under review, from the licensed base. Agents cannot sculpt by hand; where an item needs that, the lead says so instead of passing a scripted approximation.
- **Legacy** is a state, not a family: the stylized family stays selectable (decision 7), its older items become Legacy. A character that wears a Legacy or Draft item still loads and draws it.
- **The four wave-4 items stay Draft.** Their pipelines are kept as tools (the hair groom and card bake, the pattern ports, the drape, the fit) and are used where they help a licensed base; they are not the route to a Production item.
