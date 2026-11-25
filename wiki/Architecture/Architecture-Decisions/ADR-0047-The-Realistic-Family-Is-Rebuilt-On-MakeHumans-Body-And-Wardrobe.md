---
title: "ADR-0047: The realistic family is rebuilt on MakeHuman's body, and its wardrobe is MakeHuman's clothes, by driving MPFB as a tool"
type: adr
adr: 47
adr_status: Accepted
date: 2026-11-24
supersedes:
superseded_by:
affects: [avatar, assets]
source: [assets/characters/human/build.py, assets/characters/human/face.py, assets/characters/human/underwear.py, assets/characters/human/skins.py, assets/characters/human/hair.py, assets/characters/human/mh.py, assets/characters/human/wardrobe.py, assets/characters/human/manifests.py, assets/characters/human/Human.base.json, assets/characters/benchmarks/make.py]
docs: [assets/characters/human/README.md, docs/development/character-studio/architecture.md, docs/development/character-studio/references/README.md]
tags: [adr, characters, clothing, assets, licensing]
---

# ADR-0047: The realistic family is rebuilt on MakeHuman's body, and its wardrobe is MakeHuman's clothes, by driving MPFB as a tool

- **Status:** Accepted (Paul, 2026-11-24). Stages 1 and 2, the skins of stage 3 and the hair of stage 5 built the same day; the rest is listed under "Stages".
- **Date:** 2026-11-24
- **Affected systems:** Character assets (`assets/characters/human/`), Character Data (`crates/avatar`: no code change, the base is data). Amends [ADR-0032](ADR-0032-Semi-Realistic-Default-Character-Family.md) (the realistic family's source body) and narrows [ADR-0046](ADR-0046-Non-Destructive-Proxy-Fitting-And-Biharmonic-Inpainting.md) (its upstream preparation of generated garments is no longer the wardrobe's source).

## Context and problem

The realistic wardrobe had two sources and neither held up. The 33 text-to-3D garments needed a preparation script before the fit and still came out lumpy; the pattern-and-drape pipeline made clean garments but one design at a time. An audit on 2026-11-24 found the fitting core sound on clean input (two native tees imported in a second each, rest drift 0.0001 mm) and no library of realistic garments that is both openly redistributable and commercially usable, except one: the MakeHuman community's clothes, CC0 or CC BY per asset, several hundred of them, all modelled on MakeHuman's own base mesh (`hm08`).

MakeHuman clothes hang on that mesh by a per-corner rule (`.mhclo`: three body corners, three weights, an offset), so on that body they follow every body shape exactly, with nothing to solve. On any other body they must be warped across. Paul chose the body the clothes were made for.

MPFB, the Blender add-on that builds MakeHuman bodies, is GPL-3.0-or-later. The engine is Apache-2.0.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep `axcenity/realistic` and warp MakeHuman clothes onto it | Keeps the face system, skin model and 218 shapes | Every garment is refitted by the proxy fit; MakeHuman's helper geometry (skirts, tights) has no counterpart on that body |
| Add MakeHuman's body as a third family | Nothing is retired | Two realistic bodies to maintain and two wardrobes |
| **Rebuild the realistic family on MakeHuman's body** | Clothes fit as authored and follow every slider; MakeHuman's targets, skins, hair and face assets come with it | The realistic base's face system, skin model and body shapes are redone |
| Copy or port MPFB's code into the build | Less to write | GPL code in an Apache-2.0 repository |
| A separate GPL repository for the converter | MPFB code could be adapted freely | A second repository; not needed, since calling the add-on is enough |

## Decision

1. **The realistic family's body becomes MakeHuman's.** A new base, `axcenity/human` (`assets/characters/human/`), is built from MakeHuman's base mesh and targets. It ships beside `axcenity/realistic` under its own family (`human`) until it has what that base has; then `axcenity/realistic`, its build scripts, its pattern wardrobe and the generated-garment preparation are retired and the family name passes to it.
2. **MPFB is a tool, never a source.** The build scripts run in Blender and call the installed add-on's services. No MPFB or MakeHuman code is copied, ported or vendored. File formats are read from their published descriptions and checked against MPFB's output.
3. **The engine's skeleton and contracts stay.** The body is re-posed to the 45 degree A-pose and skinned to the Axcenity Humanoid skeleton with its twist bones; the base is described by the existing manifest. No crate changes for a base to be added.
4. **Garments are `fit: shapes` items with cover maps.** A garment's shapes are what its clothes file makes of the body's; the body corners the file hides become its cover map ([ADR-0045](ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md)). The proxy fit of ADR-0046 stays for third-party imports, and isn't used for this wardrobe.
5. **A licence gate per asset, and credit always.** A garment is taken only when its file header and its pack's metadata both say CC0 or CC BY and agree. Each item records its author, licence and source; `CREDITS.md` travels with the items; `NOTICE` credits MakeHuman and MPFB although CC0 asks for nothing.

## Reasons

- Measured on 2026-11-24: the body builds in 3 seconds and a garment converts in a third of a second; this package's reading of the clothes files agrees with MPFB's own fit to 0.0003 mm; 86 garments from seven packs converted in 30 seconds with 10 refused by the gate (eight AGPL3 headers, two header-and-pack disagreements).
- In engine captures (body suite: rest, T-pose, arms raised, elbows and knees bent, a run frame) five dressed characters across both frames, heavy, muscular and old-slim bodies showed no skin through cloth at sheet scale.
- The engine's base model contract already expresses MakeHuman's macro controls: a target per gender, muscle and weight is a corner shape driven by the product of sliders (architecture.md section 7, written for MakeHuman's targets in CO5).

## Consequences

- The realistic base's face identity parameters, expression shapes, skin model, makeup layers, hair, presets, handles and features have no counterpart on the new base yet. Until they do, both bases ship.
- Men's underwear doesn't exist in the CC0 or CC BY packs, so the underwear a new character wears is cut from the body's own surface (`underwear.py`: boxer briefs, and a bandeau top on the feminine frame), as the realistic base's was.
- Garment quality is the community's: clean single-shell quad meshes with photographed or painted textures, mostly without roughness maps. Each is a `draft` until reviewed.
- Garments don't hide each other's cloth (no skin-space coordinates on them yet), so two layered garments can show through one another.
- MPFB 2.0.18's rig fit is wrong under Blender 5.2 (every other bone of a chain stays on the unshaped mesh), so joints come from the base mesh's joint cubes. A later MPFB may fix it; the cubes stay the source either way.
- The wardrobe is large (about 3 MB a garment): only the four garments a new character wears are in the repository, the rest are published from the wardrobe repository as before.
- Revisit if MakeHuman's licence terms for its assets change, or if the face rebuilt on its targets can't reach the realistic base's quality.

## Stages

| Stage | What | State |
|---|---|---|
| 1 | The body (frame, muscle, fat, age), parts, skeleton, manifest, region map, the garment converter with its licence gate and credits, underwear cut from the body, four MakeHuman garments, benchmarks | Built 2026-11-24 |
| 2 | Face: identity parameters from MakeHuman's face targets, expression shapes for the standard controls, eyelids | Built 2026-11-24 (`face.py`): 149 identity parameters in ten groups, 30 control shapes on the built-in `axcenity` profile. Left: hand-written labels, quick-level picks, features, handles and presets; 13 controls with no MakeHuman unit |
| 3 | Skin: MakeHuman's skins as choices or the `skin` model over them, brows and lashes on `hair`, makeup | Built 2026-11-24 but for makeup and normal maps: the skins, then in the polish pass the Skin shading model with tone controls, a fade between ages (ADR-0048's note), fuller brows and the lashes on the Hair model. First: skins built 2026-11-24 ([ADR-0048](ADR-0048-A-Base-Models-Material-Draws-A-Library-Colour-Map-Chosen-By-The-Characters-Parameters.md), `skins.py`): the 22 skins as library images, one `skin` choice of five, the sex and the age of the skin following the body type and the Age slider. Left, and held until hair works so skin and hair are tuned under the same lighting (Paul, 2026-11-24): the `skin` shading model with tone controls and blending between ages, brows and lashes on `hair`, makeup |
| 4 | Body detail: regional targets, height and proportions by targets, correctives, levels of detail | Not started |
| 5 | Hair, presets, features and handles; Character Studio's pages on the new base | Hair built 2026-11-24 (`hair.py`): MakeHuman's ten CC0 hairstyles as `groom` items on the Hair shading model, following the head's shapes and not its expressions, with colour variants; five in the repository. In the polish pass: colour and flow maps made from the painted textures (highlights along the strands, the same eight colours on every hairstyle), Randomize drawing them, hand-written names for the 149 face sliders, ten face features with 24 handles, 30 presets. Left: a person's review of the names, handles and presets in Character Studio; body features and handles |
| 6 | The whole wardrobe converted, reviewed and published; cloth-under-cloth hiding | Partly (2026-11-25): the 51 CC0 garments are in the repository and each is measured in the fit gate; cloth is hidden under cloth by cover maps and the cut underwear is not drawn under a top or a bottom; nothing is reviewed or published |
| 7 | Retire `axcenity/realistic`, its wardrobe scripts and `prep_meshy_garments.py` | Done 2026-11-25 ([ADR-0050](ADR-0050-The-Engine-Ships-One-Character-Family-MakeHumans-Body-With-Quaterniuss-Animations.md): the stylized base went with it) |

## Notes

- **2026-11-25 ([ADR-0050](ADR-0050-The-Engine-Ships-One-Character-Family-MakeHumans-Body-With-Quaterniuss-Animations.md)): stage 7 is done.** `axcenity/realistic`, its wardrobe scripts and `prep_meshy_garments.py` were removed from the repository, with the stylized package, so the human base no longer ships beside another base: it is the engine's one family and the default (`avatar::DEFAULT_BASE`), and the default humanoid's model is its default body. The repository's wardrobe is no longer the four garments and five hairstyles of the text above but every CC0 item of the converted library, 51 garments and 10 hairstyles (`take_library.py`). Stages 4 and 6 are as the table has them.
- **2026-11-25 (faults Paul found testing Character Studio before the merge).** Three, all in how the package brings MakeHuman's data into the engine, none in the decision. (1) *Underwear came through the outfits*: the converter wrote only the red of a cover map (ADR-0045), so nothing worn beneath a garment was ever left out. It now writes green and blue from where the garment's cloth lies over the skin, gives a converted garment its corners' places on the skin (so the engine cuts and sinks it under another as it does its own fitted garments), and the cut underwear has a `Shrink_Under` shape. (2) *The feminine frame's arms were notched at the elbow*: a body shape's moves were turned into the engine's pose with the default body's bones; they are now the difference of whole bodies each posed about its own joints, and a parameter's `bones` name the twist bones. (3) *White shapes at the ankles*: the socks that come with MakeHuman's shoes, standing through the jeans; gone with (1). Measured: 108 gate checks of the underwear under an outfit, none failing; the shape change moves the feminine frame's skin by up to 13 mm and lowers two walk records.

## Implementation references

- `assets/characters/human/mh.py` (driving MPFB, the clothes reader, the re-pose), `build.py` (the body, the skeleton, the export), `manifests.py`, `wardrobe.py` (the converter, `vet`, `CREDITS.md`)
- `assets/characters/human/skins.py` (the skins, ADR-0048), `hair.py` (the hairstyles and their maps), `face.py` (`NAMES`, `FEATURES`, `HANDLES`), `presets.py`
- `crates/player/src/fit_gate_tests.rs` (`human_clothes_cover_the_human_cast`: the base in the fit gate)
- `assets/characters/human/README.md` (how it is made, rebuilding, sources and credit, what is missing)
- `crates/avatar/tests/shipped_library.rs` holds the base to the library's checks (manifest against model, default character, 1,000 random characters, benchmarks)
- `NOTICE`, "Public-domain character base data"
