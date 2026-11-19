# The intake's proof: Poly Haven's Fishermans Hat (CC0)

One real CC0 asset through every step of the licensed-asset intake (`assets/characters/realistic/import/`), 2026-11-18. The proof is the pipeline, not the item: the hat is a **draft**, written to a scratch library and **not** put in the repository's library. The downloads stayed outside the repository.

- Spec: [fishermans-hat.intake.json](fishermans-hat.intake.json) (no local paths: the files are relative to `--downloads`).
- Sheet: [fishermans-hat-sheet.jpg](fishermans-hat-sheet.jpg): the masculine and feminine average cast bodies, Idle and the starter Walk at 0.3 s, front, side and rear, whole and close up (engine captures, studio light, 2560 x 1440).

## The licence, from the primary pages

- The asset's page, <https://polyhaven.com/a/fishermans_hat>: "License: CC0", "Author: PierreB3D".
- <https://polyhaven.com/license>: "Our assets are all licensed as CC0 Public Domain" and "You do not need to give credit or attribution when using them (although it is appreciated)."
- The API's info record (`api.polyhaven.com/info/fishermans_hat`): authors `{PierreB3D: All}`, `files_hash a1b99c79405e31ff8a3e74b7b0d51783007e4b45`.

Class 1 (CC0-1.0): accepted, no credit required; a game that ships the hat lists it under "With thanks".

## The command

```bash
python -I assets/characters/realistic/import/intake.py \
    --spec docs/development/character-studio/refinement/wave5/licensed-intake/fishermans-hat.intake.json \
    --downloads <folder with the 2K glTF download> --work <scratch> --out <scratch library>
```

25 s in all on the reference PC (record 0.0 s, load 0.8, topology 0.5, conform 2.3, weights 0.1, levels 0.3, checks 1.3, export 10.1 of which the engine's import of three levels about 9).

## What each step did, measured

| Step | Result |
|---|---|
| record | 5 files hashed as downloaded (8,761,778 bytes): `fishermans_hat_2k.gltf` `5087e037...f917`, `fishermans_hat.bin` `793a2701...43e1`, `textures/fishermans_hat_diff_2k.jpg` `7ea63069...1b69`, `_nor_gl_2k.jpg` `57abd519...2904`, `_arm_2k.jpg` `64912d3c...71bc` (the full hashes are in the record below) |
| load | glTF, one object, units 1 (0.324 x 0.124 x 0.350 m), 6,696 vertices as split at the seams, 10,304 faces, one material with base colour, normal and ORM (Poly Haven's `arm`: occlusion, roughness, metal, glTF's order) |
| topology | Before: 0 degenerate, 0 open edges, 19 non-manifold edges, 0 flipped, all triangles (0% quads). Fixed: 1,560 split corners welded, 2 disagreeing edges found and 12 faces turned, 1 face dissolved by the weld. After: 10,303 triangles, 17 non-manifold edges (left: where the brim's shell meets the band; reported, not hidden) |
| conform (place) | Centred over the Head bone's upper skin, scaled 1.1, tilted 8 degrees (the front down), lowered 0.167 m until 4.0 mm off the skin; median clearance 31.5 mm, 0 vertices inside the body. At scale 1.0 the crown stopped 3 to 5 cm high on the back of the head: its opening is about 10% smaller than the realistic head at the brow, so the hat was enlarged (about 25 settings of scale, tilt and offset tried; the one that seats it lowest without touching the forehead first) |
| weights | 5,136 vertices, one influence each (the head's skin is all on Head), normalized; split off (a hat has no arm) |
| levels | LOD1 5,151 and LOD2 2,575 triangles; their surfaces stray from LOD0's by at most 0.27 and 0.63 mm (p95 0.15 and 0.37 mm) |
| checks | See below |
| export | Three levels through the engine's own import with `--placed` (alignment scale 1, offset 0); the hat covers 26% of the head region and hides no region; rigid on Head (`fit.rigid_bone`): the engine fit had pushed the brim's side tips out by up to 32.8 mm (they were bound to the shoulders' skin along their normals), which the rigid step undoes; the body's 138 shapes dropped. Read back: 10,303 / 5,151 / 2,575 triangles, texture coordinates on every level, 1 influence, 0 unnormalized, 0 unweighted. The library check (`AXCENITY_PROVENANCE_LIBRARY=... cargo test -p axcenity-avatar --test shipped_provenance -- --ignored`): the scratch library's 20 items validate and pass the provenance check |

### Section 2 checks per level (`checks.json`)

| | LOD0 | LOD1 | LOD2 |
|---|---|---|---|
| Triangles | 10,303 | 5,151 | 2,575 |
| Degenerate faces | 0 | 0 | 0 |
| Non-manifold edges | **17 (fails)** | **17 (fails)** | **17 (fails)** |
| Flipped edges | 0 | 0 | 0 |
| Quad share | 0% | 0% | 0% |
| UV overlap | 0 | 0 | 0 |
| Area over 10% stretch (limit 5%) | 0.3% | **23.8% (fails)** | **50.3% (fails)** |
| Stretch, p95 | 3.4% | 28.1% | 48.6% |
| Texel density px/cm at 2048, median / p5 (limit 8) | 30.2 / 29.6 | 30.2 / 28.0 | 30.0 / 26.3 |
| Influences, most | 1 | 1 | 1 |
| Clearance from the body: least / median, inside | 4.0 / 31.5 mm, 0 | 4.0 / 33.7 mm, 0 | 4.5 / 32.8 mm, 0 |

## What the sheet shows

Looked at full size and cropped (the close-ups are 3.4x the whole views):

- **Neutral (Idle), both bodies:** the hat sits on the head with the brim's front edge at about the brows (read off the close-ups, not measured); the canvas, band, stitching and the brass eyelets read from the source's maps. From the front the brim's sides dip and wave as the source models them (a worn hat). On the feminine body (a smaller head, no shapes on a rigid item) the crown sits about as high and the brim stands a little further from the temples; no gap shows in the views.
- **Walk, both bodies:** the hat turns with the head and keeps its seat; nothing of it bends, stretches or separates (it is rigid on Head). In the side close-ups the head leans out of the crop's box: the crop is fixed, the head moves.
- **Rear:** the brim's back, tilted up 8 degrees and seen from the camera at hip height, is almost edge-on: the back of the head shows through the brim's outline from the brim's line up to the band, which reads at first like a missing piece of brim. It is the geometry (a Blender render from head height shows the band of brim under the crown), but a reviewer may judge the tilt too high from behind.
- **Before the rigid step** (not on the sheet): the brim's side tips stood out as spikes of up to 33 mm, the engine fit's push-out at the shoulders; the rigid step removed them, and they don't show on the sheet.
- **Not checked here:** arms raised and shoulder deformation (a hat on the head is untouched by them), the other cast bodies and the body parameters' ends.

## The provenance record as exported

```json
{
  "format_version": 1,
  "asset": "Fishermans Hat",
  "source_page": "https://polyhaven.com/a/fishermans_hat",
  "author": "PierreB3D",
  "licence": { "id": "CC0-1.0", "url": "https://creativecommons.org/publicdomain/zero/1.0/" },
  "retrieved": "2026-11-18",
  "files": [
    { "name": "fishermans_hat_2k.gltf", "sha256": "5087e03713c70f5d25e9c3129c059f36f727606b2aeb19bc85983e405ed5f917", "bytes": 2788 },
    { "name": "fishermans_hat.bin", "sha256": "793a27019bd1d99302452729a3feb0cd271b3f268f059a8fec60ec0c23a443e1", "bytes": 276096 },
    { "name": "textures/fishermans_hat_diff_2k.jpg", "sha256": "7ea63069ceb09a0ace0b901f4c42c876232ae18d418b97f40a4565936c831b69", "bytes": 1829371 },
    { "name": "textures/fishermans_hat_nor_gl_2k.jpg", "sha256": "57abd519f399b0bd1b39cf1d278733df18407e27833a6108bd1a5c87575e2904", "bytes": 3661910 },
    { "name": "textures/fishermans_hat_arm_2k.jpg", "sha256": "64912d3c67ccda31e8bd48ac6ae55414bbb06dad0e25714895c222bd792971bc", "bytes": 2991613 }
  ],
  "changes": [
    "topology repaired: 1560 welded, 12 faces turned",
    "placed on the Axcenity realistic body's head (scaled x1.1, turned 8 degrees about X)",
    "simpler copies made by decimation (levels of detail)",
    "fitted by the Axcenity engine's fit, then made rigid on the Head bone",
    "materials rebuilt for the Axcenity engine"
  ],
  "attribution": "\"Fishermans Hat\" by PierreB3D (https://polyhaven.com/a/fishermans_hat), licensed under CC0 1.0 (https://creativecommons.org/publicdomain/zero/1.0/); changed: ... , by Axcenity Engine contributors",
  "items": ["intake/fishermans-hat"],
  "evidence": "The asset's page (polyhaven.com/a/fishermans_hat) gives License: CC0 and Author: PierreB3D; ..."
}
```

The credits a game made from the scratch library carries (`character_credits`), under "With thanks (public domain, no credit required)": the attribution line above, "Used for: Fisherman's hat (intake proof)".

## Rerun after review fixes 1 (2026-11-18)

The chain ran again on the same downloads with the fixed scripts (29 s; the spec no longer gives `licence_url`). What changed in the item:

- **The record's `licence.url`** is now CC0's own text, `https://creativecommons.org/publicdomain/zero/1.0/` (above); Poly Haven's licence page stays quoted in `evidence`. The first record, with `https://polyhaven.com/license`, now fails the check: `licence.url: ... isn't the text of CC0-1.0`.
- **The hat's material is single-sided** (`doubleSided: false`, the new default). At 2560x1440, 0 pixels differ by more than 24/255 from the first captures in all 12 views (both bodies, idle and Walk@0.3, front, side, rear): the shell is closed and its normals agree, so nothing was hiding behind double-sided rendering. The committed sheet is unchanged.
- **The shipped weights are judged** (`export.json`): every level has at most 1 influence, normalized, and 0 vertices past the next joint. The far rule isn't judged on a `rigid_bone` item, but it is measured, and measured on the engine fit's weights before the rigid step (LOD0) it **fails: 344 of 6,437 vertices** (the brim tips bound to the shoulders). That is the problem the rigid step works round, now caught by a rule.
- **`catalogue.json`** of the output library is written by the export (1 item); the provenance check on the item in a library with the base passes (20 items), and the credits print it under thanks with `Licence: CC0-1.0 (https://creativecommons.org/publicdomain/zero/1.0/)`, `Source:` and `Changed:` lines.

## What it proves and what it doesn't

Proved on a real asset: the licence gate and record, glTF import with its maps, the topology report and repair, placement onto the body, weights, levels, the section 2 checks, the engine's fit and import, the rigid option, the export with its manifest and record, the library and provenance checks, the game credits and the engine capture. Not proved on a real asset: the pose conform from a T-pose or another A-pose and the arm-and-side weight split (tested on small made meshes in `test_intake.py`), FBX, OBJ and `.blend` input, `decimate` and `remesh` (the hat needed neither), and a garment under the engine's fit as it ships (the hat is rigid).
