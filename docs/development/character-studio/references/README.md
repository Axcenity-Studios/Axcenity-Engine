# Character Studio references

The reference library asked for by the refinement brief, §13 ([brief](../refinement/brief.md)). Stage 0 built it on 2026-11-16. Each index was written by a researcher, then checked by a second agent that re-opened every licence page of a usable asset and a sample of the other rows (each index ends with its "Verification" section).

**What this folder holds: links, numbers and descriptions in our own words. Nothing else.** No photograph, scan, mesh or texture from any source is stored in the repository, because it is public and a public picture is not a licence to redistribute it. Photographs Paul supplies for review are looked at and never committed.

## The indexes

| Area | Index | Also holds |
|---|---|---|
| Character creation UX | [ux/README.md](ux/README.md) | [ux/patterns.md](ux/patterns.md): the layout, hover, select and handle behaviour, a handle table per feature, randomize with locks, comparison, what to avoid |
| Face | [face/README.md](face/README.md) | The brief's §5.2 controls against the existing parameters, with population means and SDs and recommended ranges; nine face presets; natural asymmetry. Per feature: [eyes](face/eyes/README.md), [nose](face/nose/README.md), [jaw](face/jaw/README.md), [lips](face/lips/README.md), [ears](face/ears/README.md) (brows, cheeks, chin and forehead are sections of the face index) |
| Body | [body/README.md](body/README.md) | The §4.1 controls with anthropometry, the meaningful controls per region, eight body presets, implausible combinations. Per area: [overall](body/overall/README.md), [upper](body/upper/README.md), [lower](body/lower/README.md) |
| Hair | [hair/README.md](hair/README.md) | A production approach per style group, what a card style can honestly support, facial hair, review checks |
| Clothing and accessories | [clothing/README.md](clothing/README.md) | A production approach per category, a first-wave pick list, a capability-manifest design note, a fit review checklist |
| Skin, makeup, markings | [skin/README.md](skin/README.md) | A recommended skin model (melanin, haemoglobin, undertone) with a tone palette, the layer order for details and makeup, decal options |
| Materials | [materials/README.md](materials/README.md) | Fabric, hair, eye and general PBR references; previewing under neutral, warm, cool and outdoor light |

Every index uses the same eight columns: Reference, Source URL, Author or publisher, Date collected, Feature, What it demonstrates, Licence or restrictions, Use (visual guidance, data or actual asset).

**Known gaps** (from [audit-gaps.md](../refinement/audit-gaps.md)): the face and body indexes are mostly data papers and drawn plates, not consistent photographs from the front, side and three-quarter; there are no pictures of arms raised or joints bent; nose tips and nostril shapes lack data. These are to be filled before R3 and R4 author new shapes (plan section 11.5).

## Sources of the Hair and Cloth shading models (CO7, wave 4)

Code written in this repository (class 5) from published formulas, each cited where it is used (`crates/render/src/hair.wgsl`, `cloth.wgsl`); no code or text was copied. Class 4 (papers and course notes): Karis, "Physically Based Hair Shading in Unreal" (SIGGRAPH 2016; see [hair/README.md](hair/README.md) and notes/hair.md); d'Eon, Francois, Hill, Letteri and Aubry, "An Energy-Conserving Hair Reflectance Model" (2011), for the lobes' shifts; Chiang, Bitterli, Tappan and Burley, "A Practical and Controllable Hair and Fur Model for Production Path Tracing" (2016), for the colour-to-absorption mapping; the eumelanin and pheomelanin absorption coefficients as pbrt v4 gives them (see [materials/README.md](materials/README.md), "Hair"); Estevez and Kulla, "Production Friendly Microfacet Sheen BRDF" (2017), and Neubelt and Pettineo (SIGGRAPH 2013), as glTF's `KHR_materials_sheen` specifies them; Burley (2012) and Heitz (2014) for anisotropic GGX, as Filament documents it. The reference renders are made with Blender (GPL; renders are ours, nothing of it ships). The test card set and cloth swatch are generated in code (`crates/render/src/renderer/tests/card_set.rs`).

## Sources of the first realistic wardrobe (R7, wave 4)

The crew-neck tee, the joggers and the sneakers (`assets/characters/realistic/clothing/`), checked on each source's own page on 2026-11-18:

| Source | What it gave | Licence (class) | Where it is recorded |
|---|---|---|---|
| FreeSewing 4.0.0 (https://codeberg.org/freesewing/freesewing; read from its GitHub mirror, `develop`), designs `teagan`, `brian` (its base and sleevecap), `paco`, `titan`, and `@freesewing/core`'s points, paths and utilities | The tee's and the joggers' drafts, ported to Python (`teagan.py`, `paco.py`, `freesewing.py`); no pattern file is vendored | MIT (2), the root `LICENSE` and each design's `package.json` (`"license": "MIT"`) | The MIT notice in `clothing/freesewing.py`, which the two drafts point to; each item's `source.author` |
| Poly Haven, [Cotton Jersey](https://polyhaven.com/a/cotton_jersey) (photography colormass, processing Rico Cilliers) | The tee's jersey and rib, the shoes' lining, tongue and laces: height, light and roughness, tiled at 263.6 mm and baked into each item's maps (colour taken out) | CC0 1.0 (1), [polyhaven.com/license](https://polyhaven.com/license) | Item `source`; `clothing/README.md` |
| Poly Haven, [Jogging Melange](https://polyhaven.com/a/jogging_melange) (colormass, Rico Cilliers) | The joggers' fleece, rib and cord, tiled at 273.5 mm | CC0 1.0 (1) | As above |
| Poly Haven, [Leather White](https://polyhaven.com/a/leather_white) (Rob Tuytel) | The sneakers' upper and midsole grain, tiled at 300 mm | CC0 1.0 (1) | As above |
| ambientCG, [Rubber004](https://ambientcg.com/view?id=Rubber004) | The outsole's rubber, tiled at 200 mm | CC0 1.0 (1), [docs.ambientcg.com/license](https://docs.ambientcg.com/license/) | As above |

No downloaded file is stored in the repository: the build reads the tiles from a folder given on its command line (`clothing/README.md` says where to get them) and the shipped maps are our bakes (class 5 over class 1).

## Imported assets: the provenance rule (decision 11, wave 5)

Since Paul's decision of 2026-11-18 (plan section 0, decision 11) character content starts from licensed assets. Every asset imported into an item has a **provenance record** before any file of it is committed: `<Object>.provenance.json` beside the item's manifest, which names it (`source.origin: "imported"`, `source.provenance`). The record holds the source page, the author, the licence's SPDX ID and version and its page, the retrieval date, the SHA-256 and original name of every file as downloaded, what was changed, the attribution line exactly as the licence asks for it, the item IDs it feeds and the licence evidence quoted from the page (schema: [architecture.md](../architecture.md) section 8a). The intake writes it (`assets/characters/realistic/import/provenance.py`); `crates/avatar/tests/shipped_provenance.rs` fails a shipped item that is imported without one, under a refused licence, or under CC BY, MIT or Apache-2.0 without its line. Classes 1 and 2 below are accepted; share-alike, non-commercial, no-derivatives, custom and unknown licences are refused (a game made with Axcenity ships its assets under its own terms, may be sold, and an imported asset is always changed). A game carries the attributions of the items its library holds (`CREDITS-CHARACTERS.txt`). Original work (class 5) says `"origin": "original"` or leaves it unset, and is released CC0-1.0. The intake's first proof, Poly Haven's [Fishermans Hat](https://polyhaven.com/a/fishermans_hat) (PierreB3D, CC0, checked on its page 2026-11-18), is recorded in [refinement/wave5/licensed-intake.md](../refinement/wave5/licensed-intake.md); it is not in the library.

## Licence classes (brief §13.5)

| Class | What | May ship in the engine? |
|---|---|---|
| 1 | CC0 and public domain | Yes |
| 2 | Open licences compatible with an Apache-2.0 engine that redistributes the asset (MIT, Apache-2.0, CC BY with its attribution kept) | Yes, with the attribution recorded beside the asset |
| 3 | Licensed commercial assets | **Never in the repository.** A person may import their own into their project (the premium tier, plan section 0, decision 2) |
| 4 | Reference only | No: link and describe. This includes the game files and custom content of The Sims 4 and other games, MetaHuman, Character Creator, 3D.sk, Anatomy for Sculptors, ArtStation, TexturingXYZ and Pinterest |
| 5 | Original Axcenity work | Yes, released CC0 like the rest of the character library |

A licence is recorded only after reading it on the source's own page; a per-asset licence on a community site (MakeHuman's asset packs, Sketchfab) is checked per asset, not by the site's default.

## What ships today, by class

Every character asset in `assets/characters/` on 2026-11-16.

| Asset | Made from | Class | Where the licence is recorded |
|---|---|---|---|
| The realistic body family (`assets/characters/realistic/`) | Blender Studio's Human Base Meshes v1.4.1 (CC0 1.0), rebuilt by our scripts | 1 and 5 | [realistic/README.md](../../../../assets/characters/realistic/README.md), "Source and licence", with the bundle's SHA-256 |
| The realistic body's composition shapes (`source/body_shapes.npz`) | MakeHuman's targets and base mesh (CC0 1.0) at a pinned commit, transferred by `assets/characters/transfer/` | 1 | [transfer/shapes/LICENCE-RECORD.md](../../../../assets/characters/transfer/shapes/LICENCE-RECORD.md), `transfer/makehuman.sha256` |
| The stylized body family (`assets/characters/humanoid/`) | Human Base Meshes, stylized pair (CC0 1.0) | 1 and 5 | [humanoid/README.md](../../../../assets/characters/humanoid/README.md), "Licence" |
| 15 items: the realistic underwear (3), the stylized clothes and hair (12) | Generated by the build scripts | 5 (released CC0-1.0) | Each item manifest's `source` |
| Benchmark descriptions | Generated by `benchmarks/make.py` | 5 | — |
| The first realistic wardrobe (3 items, drafts; R7, 2026-11-18) | Our pipeline (`realistic/clothing/`) from FreeSewing patterns (MIT) and CC0 fabric photographs (Poly Haven, ambientCG) | 2, 1 and 5 (released CC0-1.0) | "Sources of the first realistic wardrobe" above; each item manifest's `source` |

The base manifests carry no `source` field of their own; the package READMEs hold it. A new asset gets a row here and a `source` in its manifest in the same change.
