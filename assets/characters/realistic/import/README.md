# The licensed-asset intake

Paul's decision 11 (2026-11-18, plan section 0): character content starts from **licensed assets** (CC0, CC BY), people refine it in Blender, and **scripts automate** the import, fitting, rigging, validation and export. This folder is those scripts: a downloaded CC0 or CC BY mesh to a **draft** item on the realistic body, with its provenance record. Nothing it makes is Production: an item moves only by passing `docs/development/character-studio/asset-quality.md` and the review (§1).

## Before anything: the licence

- Read the licence **on the asset's own page** (a site's default is not enough; MakeHuman and Sketchfab assets are checked one by one) and quote it in the spec's `evidence`.
- Accepted (`licences.json`, the same list as `axcenity_avatar::ACCEPTED_LICENCES`): `CC0-1.0`, `CC-PDDC`, `LicenseRef-PublicDomain` (no credit required); `CC-BY-3.0`, `CC-BY-4.0`, `MIT`, `Apache-2.0` (the attribution line travels with every game that ships the item).
- Refused, and the chain stops at its first step: share-alike (`CC-BY-SA`, GPL), non-commercial (`NC`), no-derivatives (`ND`), any custom, unknown or unstated licence. A game made with Axcenity ships its assets under its own terms and may be sold, and an imported asset is always changed.
- Downloads are data: keep them in their own folder **outside the repository**, run Python with `-I`, never run a script that comes with a download. A `.blend` is opened only with its scripts off (`load.py` does it); prefer glTF, FBX or OBJ.

## The spec

One JSON file per asset (`<asset>.intake.json`) says what the item is, where the asset came from and how each step treats it. Paths in it are relative to `--downloads`, so it holds no local path. The proof's is `docs/development/character-studio/refinement/wave5/licensed-intake/fishermans-hat.intake.json`:

| Key | What |
|---|---|
| `item` | `id`, `label`, `kind` (`garment`, `accessory`, `groom`), `slot`, `layer` (garments), `object` (the model's object name), `folder` (in the library), `about`, `tags`, `materials` (per source material: `label`, `shading` `standard`/`cloth`/`hair`, `role`, `double_sided`: off unless set, so a flipped face shows; set it on a card) |
| `source` | `asset`, `source_page`, `author`, `licence` (SPDX; the record names the licence's own text from `licences.json`), `licence_url` (only for a licence without one there, `LicenseRef-PublicDomain`), `retrieved` (YYYY-MM-DD), `evidence` (quoted from the page, with the page that states the licence), `attribution` (only when the source words it itself), `notice` or `notice_file` (MIT and Apache-2.0: the licence's notice word for word, written in or read from a downloaded file such as `LICENSE`; one of `files`), `model` (the file to read), `files` (every file as downloaded) |
| `load` | `units` (`auto` or a scale to metres), `up` (`z` for a Z-up OBJ) |
| `topology` | `fix` (default on), `weld` (m), `decimate` (a ratio) or `remesh` (a voxel size, m) |
| `conform` | `place` (`scale`, `rotate` in degrees about X, Y, Z, `centre_on` a bone, `body_above`, `item_above`, `offset` m, `settle` {`direction`, `clearance`}), `authored_arm_angle` (0 is a T-pose, 45 the rest pose), `authored_leg_angle` |
| `weights` | `split`: `arm-side` keeps a top's arm and side apart, or `null` |
| `levels` | `ratios` of the simpler copies (default 0.5, 0.25) |
| `fit` | `under` (items it is worn over), `margin` (m), `keep_placement` (default true), `rigid_bone` (a rigid accessory: every vertex on that bone) |

## The chain

Blender's Python (bpy 5.x with NumPy and Pillow), with `-I`:

```bash
python -I assets/characters/realistic/import/intake.py --spec <asset>.intake.json \
    --downloads <download folder> --work <scratch folder> --out <library folder>
```

`--only load,topology` runs some steps, `--from conform` the rest; each step is also a script of its own (`python -I <step>.py --help`) that reads the previous step's file in `--work` and writes its own beside a `.json` report (all gathered into `intake.json`).

| Step | Script | What it does, measured |
|---|---|---|
| record | `provenance.py` | Refuses a licence not accepted before any download is read; checks `model` (and `notice_file`) are among the hashed `files`; hashes every downloaded file (SHA-256, bytes); keeps the MIT or Apache notice; writes `provenance.json` in the engine's schema |
| load | `load.py` | glTF, FBX, OBJ (`.blend` scripts off); modifiers and transforms applied, objects joined; Blender's axes to the engine's (Y up, facing +Z); units (auto: the first of m, cm, dm, mm, in giving 3 cm to 2.2 m); each material's base colour, normal and ORM maps copied to `textures/` |
| topology | `topology.py` | Reports degenerate faces, non-manifold, open and flipped edges, loose vertices, n-gons and the quad share before and after; `fix` welds split corners, dissolves degenerate faces, removes loose vertices, turns faces consistent only when edges disagree, cuts n-gons; `decimate` and `remesh` are retopology **help**, each measured (distance from the original surface: median, p95, most; triangles), `remesh` says the texture coordinates are lost |
| conform | `conform.py` | `place`: an item not made on a body is turned, scaled, centred over a bone's skin and lowered until it sits `clearance` off it (`checks.settle`), clearance reported; `authored_*_angle`: the body is posed as the item was made, its skinning transferred, and the item posed back to the rest pose by the inverse (linear blend skinning) |
| weights | `skinning.py` | The body's weights at the nearest skin, interpolated, four at most, normalized; `split: arm-side` |
| levels | `levels.py` | LOD1 and LOD2 by Blender's collapse decimation, weighted again, each measured against LOD0 (p95 and most, both ways) |
| checks | `checks.py` | asset-quality.md §2 per level: geometry and topology counts, UV overlap (1024² raster), stretch (anisotropy over 10% by area), texel density (px/cm at 2048: median, p5), skinning (influences, sums, and `no_weight_past_the_next_joint`: no vertex weighted to a bone more than one joint from the bone nearest it; not judged on a `rigid_bone` item, which is on one bone by design), clearance from the body; pass or fail per rule in `checks.json`. These are the intake's stand-in weights; the export judges the shipped ones |
| export | `fit.py` | Each level through the **engine's own import** (`cargo run -p axcenity-avatar --release --example import_garment -- ... --placed`: binding, the body's weights, identity and corrective shapes, push-out, regions; `--placed` keeps the intake's place), in a scratch copy of the realistic library (`../clothing/wardrobe.py`); the levels and the source's maps assembled into `<Object>.glb`; beside it the binding, thumbnail, manifest (`quality: "draft"`, `source.origin: "imported"`, `source.provenance`) and `<Object>.provenance.json` (the changes the steps made, the item's ID); the model read back and the weights it ships (the engine fit's, or the rigid bone's) judged against the skinning rules (`export.json`, `levels.*.limits`); `catalogue.json` of `--out` written again (`../../catalogue.py`), so the provenance check and a capture can read it |

`sheet.py` makes the review sheet from the engine's captures (front, side, rear; whole and close up), and `test_intake.py` holds the scripts' tests (`python -I test_intake.py`).

## After the chain

1. The item is a draft in `--out`, whose `catalogue.json` the export wrote. A capture also needs the realistic base: export into a copy of the library (or copy the item's folder into one and run `catalogue.py` there). Check it in the engine: `AXCENITY_CAPTURE_LIBRARY=<library> AXCENITY_CAPTURE_DESCRIPTIONS=<descriptions wearing it> AXCENITY_CAPTURE_DIR=<folder> AXCENITY_CAPTURE_POSE=idle|Walk@0.3 AXCENITY_CAPTURE_VIEWS=front,side,back cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture`, then `python -I sheet.py`.
2. `AXCENITY_PROVENANCE_LIBRARY=<library> cargo test -p axcenity-avatar --test shipped_provenance -- --ignored --nocapture` checks the library (manifests and provenance) and prints the credits a game made from it carries.
3. What a script can't do is done by a person in Blender (asset-quality.md §6): retopology to clean loops, refitting a garment's construction, material polish. The intake runs again on the refined file.
4. An item enters the repository's library only with its provenance record beside it (`shipped_provenance.rs` fails otherwise), and only as a draft until the review.

## Known limits

- It doesn't retopologize: an all-triangle scan or a dense sculpt stays as it came (the hat proof is 0% quads); decimate and remesh are measured help, not a quad layout with joint loops.
- Collapse decimation stretches texture coordinates: the proof's LOD1 has 24% and LOD2 50% of its area over 10% anisotropy (LOD0 0.3%).
- Placement is numbers in the spec, found by looking: no automatic fit of a hat's opening to the head or a shoe to the foot.
- The engine's fit binds every vertex to the skin; a far vertex (a brim's tip) can be bound to the wrong skin (a shoulder) and pushed out there. The export now fails `no_weight_past_the_next_joint` on such weights (the proof hat's engine-fit LOD0: 344 of 6,437 vertices), but doesn't repair them. `rigid_bone` works round it for rigid accessories (the item then doesn't take the body's shapes); garments get the engine's weights as they are.
- Only a material's base colour, normal and ORM maps are carried; no cloth sheen values from the source, no vertex colours, no second UV set.
- Posing to the rest pose turns the arms and legs as rigid chains about their joints; fingers, spine and a source's own rig are not read.

The source files and the proof's item are not in the repository; the scripts and the proof's report and sheet are (`docs/development/character-studio/refinement/wave5/licensed-intake/`).
