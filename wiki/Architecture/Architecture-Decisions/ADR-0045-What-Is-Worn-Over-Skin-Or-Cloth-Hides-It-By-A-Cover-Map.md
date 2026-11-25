---
title: "ADR-0045: What is worn over skin or cloth hides it by a cover map, and a garment follows the body by its parts"
type: adr
adr: 45
adr_status: Proposed
date: 2026-11-24
supersedes:
superseded_by:
affects: [avatar, assets, gameplay, editor]
source: [crates/avatar/src/fit/cover.rs, crates/avatar/src/fit/transfer.rs, crates/avatar/src/fit/import.rs, crates/avatar/src/fit/glb.rs, crates/avatar/src/item.rs, crates/avatar/src/resolve.rs, crates/assets/src/hidden.rs, crates/assets/src/model/hide.rs, crates/gameplay/src/description.rs, crates/editor/src/character_workspace/direct.rs, crates/editor/src/character_workspace/browser.rs]
docs: [docs/development/character-studio/architecture.md, docs/research/character-clothing-workflow.md, docs/editor/character-studio.md, docs/development/character-studio/progress.md]
tags: [adr, characters, assets]
---

# ADR-0045: What is worn over skin or cloth hides it by a cover map, and a garment follows the body by its parts

- **Status:** Proposed (built on 2026-11-24 after Paul's review of the generated wardrobe in Character Studio; waiting for his review of the result).
- **Date:** 2026-11-24
- **Affected systems:** Character Data (`avatar`: the item manifest's `cover_map`, `ItemManifest::wearing_order`, `Resolved::covers`, `ResolvedPart::under`; `avatar::fit`: `cover`, `parts`, the intake's `conform` and `tidy`, two fit options), Assets (`Primitive::skin_uvs`, `RegionHiding` with cover maps and garments, `Model::without_covered`), the Character Runtime (`gameplay`'s applier), Character Studio (the tint and handles over clothing, the browser).

## Context and problem

Paul tried the generated wardrobe ([ADR-0044](ADR-0044-Garments-Not-Made-On-The-Body-Come-Through-An-Intake-Before-The-Fit.md)) in Character Studio and found it broken: cloth through cloth and blotches on the clothes, garments that don't wrap the body, a character wearing two outfits at once, no separation of men's and women's clothes, no way to click a garment off, and body handles that are a struggle once the character is dressed. Reproduced in the capture harness, the causes were:

- **Garments imported apart cross each other.** A top's hem and a waistband pass through one another on every body, in every pose. The fit pushes a garment out of the body and of what the import names with `under`; it knows nothing of the garments it will be worn with.
- **A jumpsuit took only the `top` slot**, so trousers went on under it and came through.
- **No skin is hidden under the generated garments** (no map region passes the pose-safe test), so the fit's 2 mm margin, held per corner, is all that keeps skin from showing through a 6,000-triangle garment.
- **Character Studio's hover tint** is drawn on the skin and shown through anything within 1.5 cm of it, which on loose cloth is a blotch pattern; a handle under a garment is hidden.
- **The intake keeps a garment's own shape.** A sleeve modelled hanging and turned to the rest pose hangs under the arm like a wing; a garment cut wider than the body stands off it.
- **Sleeves are joined to the garment's side under the arm**, so the cloth there tears into strips when the arm lifts; cut edges are ragged.

The research note written for this (`docs/research/character-clothing-workflow.md`) found that shipped systems (MetaHuman, Mutable, Character Creator, MakeHuman, VRChat tooling) all **remove what is under the cloth**, by face or by a mask in the body's texture space, rather than push cloth off it, and bind loose cloth to a part-aware stand-in for the body, not to the nearest skin.

## Decision

1. **A fitted garment has a cover map** (`cover_map` in its item manifest, made by the fit: `avatar::fit::cover_map`): an image over the *body's* texture coordinates. Red is skin the body leaves out under the garment; green is skin under which a garment worn beneath leaves its own cloth out. Neither reaches the garment's openings: from a hem, cuff or neckline the cover is set back by how far the cloth stands off the skin there (2.5 times for the skin, between 2.5 and 6 cm; once for cloth, between 3 mm and 4 cm), so nothing is missing where one can see under the cloth. Skin is covered only by cloth that **moves with it** (the cloth's skin weights there share 0.8 of the skin's): the inner arm isn't hidden under the shirt's side it hangs beside, nor skin in a crease the garment only closes over. Being in the body's texture space it holds at every level of detail and for any garment bound to that body.
2. **A fitted garment's corners know their place on the skin** (`Primitive::skin_uvs`, the glTF attribute `_AXCENITY_SKIN_UV`, written by the fit from the binding): the body's texture coordinates at the point each corner is bound to. With them a garment's triangles are placed on any other garment's cover map.
3. **Garments have a wearing order** (`ItemManifest::wearing_order`): by `layer`, then by slot (underwear, shoes, bottoms, tops, outerwear). `resolve` gives the body every worn garment's cover map (`Resolved::covers`) and each garment the cover maps of those worn over it (`ResolvedPart::under`).
4. **The applier draws each from a model with what is covered left out** (`RegionHiding` with `covers`, the same mechanism and `…#h<key>` tag as hidden regions): the body without the triangles whose middle is covered; a garment without the triangles whose corners are all covered (`Model::without_covered`), the covered corners of the triangles left at the cover's edge sunk 15 mm toward the body so they dive under the hem over them. Nothing is pushed or solved at run time, and taking the outer garment off gives the plain model back.
5. **One outfit at a time is the slots' job.** A one-piece or a dress given the `top` slot also fills `bottom` (`also_fills`, which `resolve` and the browser already enforce); a suit is `outerwear` that fills `bottom`, so a shirt goes under it; overalls are `bottom` on the mid layer, over a shirt. `garments.json` says so for each generated garment.
6. **Who a garment is cut for is a tag and a limit**: `masculine`, `feminine` or `unisex`, and for the first two a `limitations` range on the body's `frame`. No new field. The Clothing page lists the garments for the character's frame, one frame's, or all (`browser::Cut`).
7. **The intake conforms a garment** (`intake::conform`, [`Conform`] per garment: `structured`, `partial`, `moderate`, `body`, `strong`): each patch of cloth is drawn in round **its own part of the body** (`fit::parts`: the trunk, each arm, each leg; cloth clearly nearest one part is that part's, the rest takes its shares from the cloth round it), by the patch's mean clearance, the moves smoothed over the cloth, so folds, collars and cuffs move with the cloth they stand on. A skirt's cloth under the crotch keeps its shape.
8. **A garment that came through the intake takes its skin weights by the same shares** (`FitOptions::weights_by_part`): a sleeve from the arm all the way round, a skirt from the hips and both legs (`FitOptions::skirt_from`). Triangles a fit pose still draws out five times (`FitOptions::cut_webs`, not judged with the arms overhead, never in a skirt) are taken out, and the intake's cut edges are tidied (scraps, teeth, the hems' zigzag).
9. **Character Studio shows a body feature on what covers it**: a worn garment's corners take the tint of the nearest skin (`direct::WornMap`), the garment hides the skin under it like any surface, and handles are hidden only by skin, so they show on the clothes and stay usable. A click on a worn tile takes the item off; a click on the tile being tried on lets it go.

## Alternatives considered

- **Resolve garments against each other when the outfit is put together** (Daz Studio's smoothing modifier with a collision item): right in principle, but a second fit at run time, per pair, per level of detail, with the body's shapes to carry through it. Hiding by a mask is a lookup.
- **Layer envelopes**: each garment fitted between surfaces that stand for the layers under and over it (the research note's D8). Needs every garment refitted when an envelope changes and still leaves a loose inner garment through a snug outer one. Kept as a possible addition for bulky inner items.
- **A shrink shape on the inner garment** (Cyberpunk 2077's GarmentSupport): one more carried shape per item and a score rule; the sink in decision 4 is the same idea done on the triangles left at the cover's edge.
- **A new vertex channel versus the binding file at run time.** The binding has each corner's place on the body, but it is a megabyte of JSON per item and knows only the full-detail corners. `skin_uvs` rides through the simpler copies, the cooked cache and the identity bake like any other corner data.
- **Non-rigid registration to the body** (rejected in ADR-0044 for flattening loose cloth): conforming by a patch's mean clearance is the part of it that is safe.
- **Weight inpainting** (Epic's 2023 method, the research note's D4): the better general answer for the underarm, and needs a sparse solve. Shares of the body's parts settled over the cloth reach most of it with what the intake already had; inpainting remains open.

## Consequences

- Separately imported tops, bottoms and outerwear no longer cross where the outer one covers the inner; skin can't show through cloth that covers it. Each distinct outfit is a hidden model per covered part (triangle lists only; corners are shared), made on first use.
- An item made before this has no cover map and no skin coordinates: it hides nothing and is never hidden, as before. The first realistic wardrobe (pattern-made) gets a cover map when it is next rebuilt through the import; its combined model does not carry `_AXCENITY_SKIN_UV` until `wardrobe.py` copies it.
- The cooked model cache gained a field (it is rebuilt by the engine's build stamp).
- Under an arm raised level or overhead, a garment whose source had the sleeve joined to its side still tears or shows a hole. The cure is in the source (decision below).
- The cover is made on the default body at rest. A garment that slides far over the skin in a pose (a loose sleeve's cuff) can uncover hidden skin; the setback and the moves-with rule are what guard against it.
- **The fit gate's hole rule changed with it** (`player`'s `fit_gate_tests::holes`, `avatar::fit::uncovered`): hidden skin is a hole where no cloth lies over it, not wherever it is outside the cloth. Cloth sunk a little under where hidden skin would be is what hiding is for: one sees cloth. On the test tee the gate finds no hole at rest or in the held poses, and a few corners (up to 13 mm) at the widest swing of the walk and the run, recorded as its other defects are.

## The authoring workflow this settles

1. **Source.** One garment per file, upright, facing +Z, **in an A-pose with the arms about 45 degrees from the body and the legs apart**, so nothing is joined under the arm or between the legs. Ask for one cut per frame where the design differs.
2. **List it** in `garments.json`: slot, layer, `also_fills`, `frame`, `conform`, and the hints the mesh can't give (`kind`, `ease`, `cut_for`, `triangles`).
3. **Import** (`import_generated.py`): intake (read, one wall, size and place, rest pose, conform, budget, tidy), fit (binding, weights by part, the body's shapes, push-out, webs), cover map, manifest.
4. **Look at it** in the capture harness's body suite on a slim, a default and a heavy body of its frame, alone and with a garment of each slot it overlaps. It stays a `draft` until it passes.

## Amended the same evening (2026-11-24, after Paul's second review)

Paul wore the remade garments in Character Studio and sent 40 pages of close-ups: cloth torn into slivers along sleeves and shoulders, holes with the skin or the black underwear behind them, arms outside their sleeves, faceted cloth, and a slow jacket. Each was measured on the fitted files before anything changed (`progress.md`, third entry of the day). What changed in this decision:

- **Skin is hidden where the cloth is proved to stay over it, not where its weights match.** The 0.8 weight-share rule left most skin under a loose garment drawn (the bomber jacket hid 566 of 3,349 covered triangles). `cover_map` now takes the fit's poses and pose states (`cover::Proof`) and hides a covered skin triangle when, in every one of them, each of its corners is under cloth by the gate's rule or meets cloth out along its normal within 25 cm. The setback from openings is as it was. Over the 32 garments a median of 36% of the skin under a garment is hidden (8% to 72%). The weight-share rule remains for a caller with no poses.
- **A garment with no skin coordinates borrows the body's** (`RegionHiding::skin`, `Model::with_skin_from`, `Model::without_well_covered`), so the underwear and the first realistic wardrobe are hidden under what is worn over them. This settles the second open question below: yes.
- **Seams never open under a pose.** The posed fit's following goes into a garment's correctives the same for corners at one place (`Welds`), and for a garment that came through the intake it is eased by how near the skin a corner lies (all within 1 cm, none past 4 cm), smoothed over the cloth and held to 2.5 cm. Before, corners at one place moved 4 cm to over a metre apart at a shoulder corrective's full weight (14 to 43 cm on most tops), and neighbouring corners of the bomber jacket up to 22 cm.
- **Cuffs and hems follow the forearm and the shin** (`parts::held_joints`): by-part weights on a hand, a finger, the forearm's twist bone, a foot or a toe go to the limb above.
- **The highlight on worn items is a soft mask, and cloth is picked by it** (decision 9, as built that evening; `crates/editor/src/character_workspace/direct/mask.rs`, `direct/worn.rs`, `direct/depth.rs`). Decision 9's tint was the garment's own triangles, each corner with the weight of the one skin corner nearest it, tested against a depth buffer to 3 mm: on the generated garments that was blotches with zig-zag outlines along their triangles, and under the pointer often nothing, because the skin a ray was to find behind the cloth is the skin this decision leaves out of the body. Now the character as posed is rasterized once into a buffer that keeps each texel's nearest triangle (the body's or a worn part's), a texel is a feature's where that surface's weight is at least a half, and the cover is blurred into a soft tint drawn as one textured quad: nothing is compared with a depth a second time. A worn item's weights come from `direct::worn::WornMap`: each of its points takes from up to 8 body corners near it at rest, and its points are its corners and, on triangles whose corners lie over different body regions (more coarsely on any over 4 cm), a lattice about 1.2 cm apart, so a feature's edge crosses a coarse garment where the skin's does. A ray that meets a worn item first picks the feature lit on the cloth there; the skin up to 25 cm behind is only the fallback where the cloth carries none. Handles are hidden only by skin, as decided. The rest is in [Characters](../../Engine-Systems/Characters.md) and `docs/editor/character-studio.md`, "Editing on the character".

## Amended again (2026-11-24, fourth entry: a top and a bottom worn together)

Paul asked why hems were bitten before any fill was written. They aren't: ten garments were imported with the mesh written out after every step, and every back hem is level to 3 to 25 mm from the single wall to the fitted garment. What shows is the bottom worn under the top. Each garment is fitted to the body and the underwear alone, so over the seat the bottom's cloth stood outside the top's over 29% to 90% of their overlap (3 to 13 mm in the middle, up to 3 cm), and in the setback of the top's hem, where this ADR leaves cloth worn beneath drawn, it was drawn in front of the top, cut along its own triangles.

Paul chose a fit step over a shape weighted when worn, with guards against tight tops ballooning. Decided:

- **A top or a jacket is fitted for the bottoms it will be worn with** (`fit/hang.rs`): its cloth round the hips (the skin from the waist to the top of the thighs, never the chest or the sleeves) is moved out along the skin's normal until it clears every bottom of the library, by 3 cm at the most, spread and smoothed over the cloth. A bottom a corner couldn't clear with twice that is left out for that corner.
- **A top that lies on the body is tucked in** (the tank top, the polo, the off-shoulder top): its cloth round the hips is drawn in instead, it is layer 0 (so the bottoms are worn over it; no new field, and an older engine reads it the same way), and it is hidden under a bottom's cover map **whole**: in blue, lit wherever the bottom's cloth is over the skin, with no setback. The setback is for a hem, under which one looks; nothing is seen under a waistband. Blue is lit that way only round the hips (`waist`, `hips`, `thighs_top`) and is as green anywhere else: a bottom is not all waistband, and the straps of overalls cut the polo's back away until this was added (2026-11-24, the sixth progress entry). This answers the question this ADR left open.
- **Bottoms are imported first**, and a record of which bottoms each top was fitted for makes a stale top an error (`import_generated.py --check`, run by the pack's build).

Rejected: fitting a top over the bottoms as lower layers (`--under`), which fixed the puffer and the V-neck over one pair of trousers and doubled the size of the tank top and the polo over all eleven (the push takes a layer for a closed body, isn't kept to the hips and has no cap); a low cap against slim bottoms only, which left the bites over cargo pants; hiding by a wider cover (a hole under a hem is worse than cloth through it).

Consequences: adding or changing a bottom means importing the tops again (three and a half minutes for the fourteen, seven at a time). The hem of a top opens by up to 3 cm at the seat to clear the bulkiest bottom, whichever is worn. The order is made at rest on the default body.

## Amended on 2026-11-25: the cut underwear is not drawn under what takes its place

Paul, looking at the wardrobe's contact sheets (the panties on the boxer briefs, whose legs showed whole round them): "If there is an overlapping piece of clothing then simply do not render the undergarments."

A cover map leaves an inner garment out only where the outer one's cloth is over it, so a smaller garment shows the larger one beneath it round its edges. That is right for a T-shirt under a jacket and wrong for underwear a body type always wears: nobody chose it, and it can't be taken off.

- A base model's slot can name the slots whose item takes its own off the character (`Slot::hidden_by`, an optional field: no format change). While one of them is filled, the item worn in the slot stays worn (no problem, still in the description) but `resolve` makes no part of it, and it hides no region and has no cover.
- The human base sets it on its two underwear slots: `underwear_bottom` under `bottom`, `underwear_top` under `top`. An outfit fills both. Shoes, socks, stockings, gloves and hats take nothing off.
- It goes by slot, not by what the cloth covers, so it is exact and costs nothing at runtime: a long top still leaves the briefs drawn below its hem, and an open back shows skin where the bandeau was.
- MakeHuman's own underwear and swimwear are filed as a top or a bottom of layer 0 (`wardrobe.py`), so they take the cut underwear's place instead of sitting on it.

Tests: `avatar` `an_item_in_a_slot_hidden_by_another_is_worn_but_not_drawn`, `the_human_bases_underwear_goes_under_its_clothes`; `player` `the_human_bases_underwear_is_not_drawn_under_an_outfit`.

## Open (for Paul)

- ~~Whether tucked-in tops are wanted.~~ Decided above: tops that lie on the body are.
- ~~Whether the underwear should be hidden under clothes too.~~ Built the same evening (above): it borrows the body's texture coordinates.
- Regenerating the garments whose sleeves are joined under the arm in an A-pose, which would remove the last tears.
