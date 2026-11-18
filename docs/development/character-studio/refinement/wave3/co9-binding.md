# CO9 binding: hand-back (RA4, refinement wave 3)

Branch `co9-binding`, from `refinement-wave2`. Task: the clothing system's fitting core (plan section 1's CO9 row; 11.5 R7's first half; [ADR-0035](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0035-Clothing-Fits-By-Surface-Binding-With-Region-Hiding.md); [ADR-0041](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md) decision 4).

**In one paragraph.** The fitting core is built and tested: a garment made in the body's rest pose is bound to the body, takes its skin weights and all 127 of its identity and corrective shapes as same-named morph targets, and is pushed out of the body and the layers under it at rest, at each shape's full weight and at 108 extreme combinations. At run time it is an ordinary skinned part following the body's live weights, so nothing is fitted or rebuilt per description. Region hiding works live: a worn item's whole map regions are left out of the body drawn. The import flow turns a third-party OBJ or glTF into a draft library item with a model, binding, thumbnail and manifest, and it has a command line. **The test garment is a draft and it looks like one.** Its torso follows every frame and fat setting. The armpit of its voxel-made body fills the crease between the arm and the torso, and that fill shows as flaps beside the waist that stay down when the arms rise. Its edges are ragged. Skin comes through in the walk and with the arms raised (counts below). The underwear keeps its own shapes; the measurement says why. The stylized clothes have fit problems the gate measured; I didn't fix them.

## Files

| File | What |
|---|---|
| `crates/avatar/src/fit/` (new: `mod.rs`, `surface.rs`, `bind.rs`, `transfer.rs`, `coverage.rs`, `glb.rs`, `thumbnail.rs`, `import.rs`, `tests.rs`) | The fitting core and import flow |
| `crates/avatar/examples/import_garment.rs` (new) | The command line for the import (the realistic build or a person can call it) |
| `crates/avatar/src/item.rs`, `lib.rs` | `ItemManifest::quality` (`Quality::{Draft, Production}`, unset is a draft), `is_draft()`; `pub mod fit` |
| `crates/avatar/src/manifest.rs` | `BaseModel::regions_map` (optional), its path checked |
| `crates/avatar/src/resolve.rs` | `Resolved::hidden_regions` |
| `crates/avatar/Cargo.toml`, `Cargo.toml` | `glam`, `image` (both already workspace dependencies) for `avatar`; `avatar` at opt-level 3 in dev, like `mesh-edit` |
| `crates/assets/src/hidden.rs`, `model/hide.rs` (new); `derived.rs`, `model/identity.rs`, `model/objects.rs`, `model.rs`, `lib.rs`, `loading.rs` | Models with hidden regions (`…#h<16 hex>`): `RegionHiding`, `Assets::hide_regions`, `hiding_of`, `hidden_mesh_id`, `split_hidden`, `strip_tags`, `Model::without_triangles`; `split_derived` treats a hidden model as a base of its own (`…#h<key>#i<key>` bakes on top) |
| `crates/gameplay/src/description.rs` | The applier draws the body with its worn items' map regions hidden (`hidden_body`) |
| `crates/project/src/character_library.rs` | `add_character_packages` copies a base's region map |
| `assets/characters/realistic/manifests.py`, `Realistic.base.json` (regenerated) | `model.regions_map: "RealisticRegions.png"` |
| `crates/avatar/tests/fitting.rs` (new), `tests/fixtures/test-tee/` (new: `make_test_tee.py`, `TestTee.obj`, `TestTee.mtl`, `README.md`, CC0) | Integration tests; the test garment |
| `crates/player/src/fit_gate_tests.rs` (new), `main.rs` | The fit gate on applied, posed meshes; region hiding live |
| Docs | `architecture.md` (sections 6, 8, 9, 14, 16, the built list), `crates/avatar/CLAUDE.md`, `crates/assets/CLAUDE.md`, ADR-0035 (implementation notes), ADR-0041 (status), `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md` |

**Generated files regenerated:** only `Realistic.base.json` (by `manifests.py`, run with the bpy Python for numpy); the only change is the `regions_map` line. The item manifests came out with line-ending changes only; I didn't commit them. `RealisticParts.glb`, the catalogue, presets and benchmarks are unchanged; there is no need to rebuild them for this branch.

## What was built

1. **Binding and transfer** (`fit_garment`). The steps, in order:
   - **Rest push-out.** The garment is pushed out of the body and every lower layer to `margin` (default 2 mm). Each corner moves along the obstacle's normal at its closest point. The move is spread over 6 rings at 0.7 a ring, and corners at the same place are welded so that seams stay closed.
   - **Binding.** Each corner is bound where a ray inward along its normal meets the body. When that point is more than three times as far as the closest point (and more than 3 cm farther), the closest point is used instead. A binding stores the triangle, the barycentric weights and the offset in the frame of the smoothed normal and first edge, so the rest pose reconstructs exactly.
   - **Weights.** The body's weights at the bound point are mixed, cut to the four largest, normalized, and smoothed over the garment for 2 rounds.
   - **Shapes.** Every identity shape (driven by a parameter or a corner) and every `Corrective_*` shape is carried; `Mask_*` shapes are never carried. Each is rebuilt through the binding on the shaped body, the garment's normals turn with the frame, and the shape is pushed out at its full weight. A shape that moves nothing is still written, as an empty sparse target, so the count always matches.
   - **Extremes.** Where the garment is still inside at an extreme combination, its rest pose is lifted, rechecked up to 4 times.
   - **Report.** `FitReport` records what each layer pushed (`LayerOffset`), the extremes checked and lifted, and `inside_most`.
   - **Output.** `write_fitted_glb` writes a skinned glTF with a copy of the body's skeleton and sparse morph targets.
2. **Region hiding** (live). The base manifest's `regions_map` gives each body triangle a region (sampled at the middle of its UVs). `resolve` lists the hidden regions, and the applier swaps the body's `MeshId` to the hidden variant (`…#AxcenityRealistic/Body#h<key>`). The variant is made once, shared, and changes only the index lists, at every level of detail. Taking the item off restores the plain `MeshId`. Wearing the tee hides `waist`: 256 of the body's 22,184 triangles. `torso` is not hidden because the tee covers 82% of it: the realistic `torso` region runs up the neck, past a crew neck.
3. **Import** (`import_garment`, and `cargo run -p axcenity-avatar --release --example import_garment -- <file> <library> <base> <id> <label> <folder> [--under <id>]...`):
   - **Alignment.** Units are detected (cm, mm), and the garment is centred across and front to back on the body's band at its heights.
   - **Coverage, regions and slot.** For each region, the share of its triangles under the garment is measured. Regions covered at 99.5% or more are hidden. The slot is `top`, `bottom`, `shoes` or a dress (`top` plus `bottom`).
   - **Fitting.** The import runs the fit above, against the `--under` layers.
   - **Output.** A 256 px software-rendered thumbnail; `<Object>.glb`, `<Object>.binding.json`, `<Object>.png` and `<Object>.item.json` (`fit: binding`, `tier: premium`, `quality: draft`, `source` required); and the item listed in `catalogue.json`.
   - **Test garment.** `TestTee.obj` (3,152 vertices, 6,048 triangles) imports in 34 to 58 s in release on the shared machine. It lands as slot `top`, layer 1, hiding `waist`, with alignment offset 18 mm in z.
4. **Underwear.** Measured, not changed. Binding the underwear to the body and carrying its shapes agrees with its own shapes where the cloth lies on the skin:
   - boxer briefs: 90.7% of offsets within 1 mm, worst 27 mm;
   - briefs: 93.9%, worst 18 mm;
   - bra: 95.7%, worst 8 mm;
   - `Frame_Feminine`: 87 to 93% within 2 mm.

   The worst offsets are where it bridges a hollow: the gusset (up to 64 mm) and the bra between the breasts on the feminine frame (22 mm). There its own per-frame shapes are right and the transfer is not. So it keeps `fit: shapes` and its mask shapes. It covers no map region whole, so map hiding can't replace the masks.
5. **Stylized clothes.** Measured by the gate (below), not changed: see Open issues.

## Interfaces

All new fields are optional and no format version changed:

- `ItemManifest.quality`;
- `BaseModel.regions_map`;
- `Resolved.hidden_regions`.

The `MeshId` tag `#h<16 hex>` is new and sits beside `#i`. The full Rust API is in `architecture.md` section 16 under "Fitting". It replaces the signatures the plan pencilled in, `bind(item, body)` and `fit(binding, body_rest, lower_layers, margin)`: ADR-0041 decision 4 moved fitting from run time to build and import time.

## Tests (red first: `co9-binding-red-run.txt`)

- `avatar` unit tests (`fit::tests`, 11), on a made-up tube body with two bones, a fat shape, a corrective and a mask:
  - the closest point agrees with brute force;
  - a binding puts every vertex back (1e-5 m);
  - weights add up to 1 and match the body's;
  - a garment sunk 2 mm is pushed to its margin;
  - carried shapes follow the body (fat moves the sleeve 2 cm) and clear it at full weight and at the extremes;
  - masks are never carried;
  - an outer layer stays over a lower one at every state;
  - coverage finds the wholly covered band;
  - the written glTF reads back with the same skeleton, weights and shapes (1e-4 m);
  - the thumbnail draws;
  - alignment from centimetres and off centre.
- `avatar` `tests/fitting.rs` (3):
  - **The underwear agreement above.**
  - **The import.** Slot `top`, `quality: draft`, `fit: binding`. It hides `waist`, and only regions covered at 99.5% or more. The library reads back and validates, and `check_model` passes. Its shapes equal `carried_shapes` (127, `Frame_Feminine` and the correctives among them). Weights add up to 1. `inside_most` is under 1.2 mm; measured 0.9 to 1.1 mm, see Open issues.
  - **The layered pair.** The tee imported over the sports bra stays at least 1.5 mm outside it at rest and at all 108 extremes plus rest.
- `player` `fit_gate_tests` (5). The garments are on the applied, posed meshes of the stage the capture tool builds. The bodies: 54 realistic cast and body benchmarks with the tee over the underwear, in the rest pose, a walk frame and the four held poses. A body corner is "covered" when, at rest, its closest point on the garment is away from the hems (two rings) and lies along the garment's normal facing the skin's way. It counts as through when it is more than 1 mm outside the garment's outer surface. Every garment measured is a draft. Each test fails when there are more failing cases or worse ones than recorded (a ratchet, to be lowered as items improve; a `production` item must have none):
  - rest and walk: 67 of 300 checks failing, worst 28.3 mm;
  - T-pose and arms raised: 63 of 300, worst 18.2 mm;
  - elbows and knees bent: 10 of 300, worst 19.5 mm;
  - the stylized outfit (tee, cargo pants, sneakers) on the stylized cast at rest and in the T-pose: 67 of 160, worst 33.9 mm;
  - region hiding live: 256 triangles hidden while the tee is worn, all back when it is taken off.
- An `assets` split test: hidden and baked tags split, `strip_tags`.

**Runs:**

- `cargo nextest run -p axcenity-assets -p axcenity-avatar -p axcenity-gameplay -p axcenity-project --no-fail-fast`: **604 tests run: 604 passed, 1 skipped**.
- `cargo nextest run -p axcenity-player -E 'test(/fit_gate_tests|default_humanoid_tests|character_capture|skin_value/)'`: **38 tests run: 38 passed**.
- `cargo clippy -p axcenity-avatar -p axcenity-assets -p axcenity-gameplay -p axcenity-project -p axcenity-player --all-targets -- -D warnings`: clean.
- `cargo fmt --all`: done.
- `cargo hakari generate`: no changes.
- `python3 -I scripts/check-wiki.py`: 0 errors.
- Each gate test takes 60 to 90 s under the shared load, inside nextest's 120 s.

**The gate's failures by garment and pose** (cases, worst):

| Garment | Rest | Walk | T-pose | Arms raised | Elbows bent | Knees bent |
|---|---|---|---|---|---|---|
| Test tee | 3, 19.5 mm | 54, 28.3 mm | 5, 13.2 mm | 54, 14.9 mm | 3, 19.5 mm | 3, 19.5 mm |
| The bra through the tee | 0 | 7, 12.3 mm | 0 | 0 | 0 | 0 |
| Boxer briefs (heavy-soft body, crotch) | 1, 14.4 mm | 1, 17.5 mm | 1 | 1 | 1 | 1 |
| Briefs (heavy-soft bodies, gusset) | 4, 18.2 mm | 0 | 4 | 1 | 1 | 1 |
| Stylized sneakers (sole of the foot) | 16, 33.8 mm | | 16, 33.8 mm | | | |
| Stylized tee | 7, 24.7 mm | | 8, 24.8 mm | | | |
| Stylized cargo pants | 7, 33.8 mm | | 7, 33.8 mm | | | |

## Visual evidence (`co9-binding/`)

The sheets are the capture tool's body suite under studio and hard light, in the front, three-quarter and side views. Rows run rest, T-pose, arms raised, elbows bent, knees bent and walk, each under both lights. The sheets are:

- `tee-*-body.jpg`: the test tee on the masculine and feminine average, heavyset, muscular and slim cast;
- `bare-*-body.jpg`: the underwear;
- `crops-rest.jpg`: torso crops at full size;
- `test-tee-thumbnail.png`: the import's thumbnail.

What I saw:

- **The tee follows the body's shape system.** On the feminine frame its chest takes the bust and its waist narrows. On the heavyset bodies it carries the belly. It never shows skin through its front or back at rest.
- **The armpit fill fails.** The voxel union filled the crease between the arm and the torso. At rest the fill is pushed out by up to 34 mm and stands off the waist as flaps, visible from the front on both frames. In the T-pose and with the arms raised the flaps stay with the torso while the arm leaves. Inward rays put the sleeve on the arm, but this fill sits between the two limbs. The thumbnail shows the same flaps.
- **The edges are ragged.** The hem, neck and sleeve ends are voxel stair-steps. There is no hem thickness, construction or folds. These are asset-quality §3 failures, as a draft is expected to have.
- **Dark gaps.** The side view shows dark gaps at the armpit, where the shell folds.
- **The underwear holds.** On the cast, in every pose, I saw no skin through it at this size. The gate's underwear cases are inside the crotch, not visible in these views.
- **The knees-bent row floats.** Both legs bend and nothing plants the feet. This comes from the capture tool's held pose, not from this work.

## Open issues

1. **Binding across a crease.** The armpit, and the crotch for loose garments. Inward rays fix sleeves over a limb but not fill between two limbs. What would fix it:
   - weights blended from both sides by distance;
   - a better test garment with a real armhole (pattern-made: FreeSewing);
   - cloth proxies or deformer maps (ADR-0035's fallback).
2. **Deep push-out folds.** Pushes larger than about 1 cm, spread over 6 rings, fold the cloth into flaps. Push-out should stay for small corrections; a garment intersecting the body by centimetres should be refused or reported at import rather than pushed.
3. **Two-sided clearance.** Where a crease is narrower than twice the margin, the garment can't clear both sides: `inside_most` is 0.9 to 1.1 mm for the tee. The import test holds it under 1.2 mm.
4. **Pose-driven poke-through.** The walk and arms raised (counts above) are not checked at fit time, only by the gate. Pose correctives exist only for the shoulder.
5. **Shipped clothes the gate found failing.** All are drafts; none is fixed here:
   - realistic boxer briefs and briefs: the inner thigh through the gusset on the heavy-soft bodies, up to 18 mm;
   - stylized sneakers: 400 to 500 foot corners out by up to 34 mm on every stylized body, at the sole;
   - stylized tee and cargo pants: up to 25 and 34 mm on some bodies.

   The stylized numbers want a look in close captures before anyone fixes them (the sneakers could be a sole drawn above the foot's bottom). Re-fitting them with the transfer would mean writing them as separate binding-fitted models and changing the humanoid's `manifests.py`; I left that for its owner.
6. **Region hiding.**
   - The realistic regions are coarse (`torso` reaches up the neck), so a T-shirt hides little. A finer map (upper chest and back apart from the trapezius) belongs to `regions.py` (RA3).
   - Each hidden set is its own model, so the body's mesh and shapes are uploaded once per such outfit. An index-range draw in `render` would avoid that (RA7, with CO8).
7. **`quality` exists but isn't enforced.** Drafts aren't yet hidden in the studio, or kept out of defaults, presets and Randomize (RA1 and `randomize`).
8. **Import gaps.**
   - No glTF material variants (`KHR_materials_variants`) are read.
   - Textures aren't carried: the import writes colour and roughness only.
   - The thumbnail is flat-shaded, not a render.
9. **Captures of extremes.** The body extremes are covered by the gate, not captured.

## At merge

- `Cargo.toml` adds a `[profile.dev.package.axcenity-avatar]` block.
- `Cargo.lock` gains `glam` and `image` for `avatar`.
- The lead rebuilds nothing for this branch. If another branch regenerates `Realistic.base.json`, keep `model.regions_map`; it comes from `manifests.py`.
- `ItemManifest.quality`, `BaseModel.regions_map` and `Resolved.hidden_regions` are contract additions in `avatar`. A parallel branch may add `quality` too; keep one.
- `crates/assets/src/derived.rs` and `model/identity.rs` (`split_derived`) changed. Check them against CO8's branch if it touches derived models.
- The gate's ratchet numbers (`Known` in `fit_gate_tests.rs`) are this branch's. Lower them when an item improves; never raise them to pass one.
