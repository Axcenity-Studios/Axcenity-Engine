# Wave 5 hand-back: the licensed-asset intake (decision 11)

Branch `licensed-intake` (from `refinement-wave5`). Paul's decision 11 of 2026-11-18: licensed CC0 and CC BY assets are the base, Blender scripts automate import, fitting, rigging, validation and export, every imported asset is licence-verified with its attribution tracked, and a game made with Axcenity carries the attributions it needs. This branch builds and proves that pipeline; it adds no item to the library.

## What was built

1. **Provenance** (`crates/avatar/src/provenance.rs`, `item.rs`). A sidecar record per imported asset, `<Object>.provenance.json` beside the item's manifest (schema in `architecture.md` section 8a): asset, source page, author, licence `{id, url}`, retrieval date, files `[{name, sha256, bytes}]` as downloaded, changes, attribution line, items fed, evidence quoted from the page. Sidecar, not a manifest block: one record can feed several items, the intake writes it before the item exists, and the manifest stays what the browsers read. The item's `source` gained two optional fields, `origin` (`original` | `imported`; unset reads as original) and `provenance` (the record's path).
2. **The licence rule and the check.** `licence_terms` accepts `CC0-1.0`, `CC-PDDC`, `LicenseRef-PublicDomain` (no credit) and `CC-BY-3.0`, `CC-BY-4.0`, `MIT`, `Apache-2.0` (attribution), and refuses with the reason share-alike, non-commercial, no-derivatives, custom and unknown IDs. `check_item_provenance` / `check_provenance` (a function of its own, for the content-state check to sit beside): every item has a source; non-imported work must be `CC0-1.0` (so a CC BY item that doesn't say it was imported fails); an imported item names a record that is there, reads, is sound, names the item and matches its licence; an attribution licence needs its line. `tests/shipped_provenance.rs` holds the repository's library to it.
3. **Attribution in games.** `character_credits` (avatar) and `project::character_credits`: `pack_game` writes `CREDITS-CHARACTERS.txt` at the top of the game file, Build Game writes it beside the game and notes it in the build report, and File > Build Game's Output says how many attributions it carries (a warning when an item isn't cleared). Every imported item of the project's library is listed (a game ships its whole library, so each can be worn), none of the engine's the project hasn't copied. CC0 items are listed as thanks, as a courtesy (cheap, and it says where an item came from); an item without a sound record is listed as not cleared. A game with no imported item gets no file. `add_character_packages` now copies an item's provenance record with it. `player` gained no dependency.
4. **The intake scripts** (`assets/characters/realistic/import/`, README first): `intake.py` runs `provenance.py` (licence gate, hashes), `load.py` (glTF, FBX, OBJ, `.blend` with scripts off; axes, units, maps), `topology.py` (report; fix; decimate or remesh help, measured), `conform.py` (place onto a bone's skin and settle; pose from an authored arm or leg angle to the rest pose with the body's skinning, inverse linear blend skinning), `skinning.py` (body weights, four at most, normalized, `split: arm-side` as a parameter), `levels.py` (LODs, measured), `checks.py` (asset-quality §2 per level), `fit.py` (the engine's own import per level, assembled with the source's maps, manifest `quality: "draft"`, record with the changes, read back; `rigid_bone` for rigid accessories). Each step runs alone too. `sheet.py` makes the review sheet from engine captures. It reuses `clothing/wardrobe.py` (GLB helpers, scratch library) and the engine's `import_garment`; it doesn't duplicate the fit.
5. **`ImportOptions::keep_placement`** (`avatar::fit::import_garment_with`, `--placed` on the `import_garment` example): the import's alignment (an even-ease search for garments made on a body) lifted the placed hat 6 cm and shrank it 8%; an item the intake placed now keeps its place. `import_garment` is unchanged; the fit algorithm is untouched.
6. **The proof**: Poly Haven's Fishermans Hat (PierreB3D, CC0, licence read on the asset's page and polyhaven.com/license) through every step: [licensed-intake/README.md](licensed-intake/README.md) and [the sheet](licensed-intake/fishermans-hat-sheet.jpg).

## Files changed

- New: `crates/avatar/src/provenance.rs`, `crates/avatar/src/tests/provenance.rs`, `crates/avatar/tests/shipped_provenance.rs`, `crates/avatar/tests/intake_placement.rs`, `crates/project/src/character_credits.rs`, `assets/characters/realistic/import/` (`README.md`, `licences.json`, `intake.py`, `provenance.py`, `load.py`, `blendmesh.py`, `topology.py`, `conform.py`, `skinning.py`, `bodyglb.py`, `levels.py`, `checks.py`, `fit.py`, `sheet.py`, `workmesh.py`, `test_intake.py`), `docs/development/character-studio/refinement/wave5/licensed-intake.md`, `licensed-intake-red-run.txt`, `licensed-intake/` (`README.md`, `fishermans-hat.intake.json`, `fishermans-hat-sheet.jpg`, 0.37 MB).
- Changed: `crates/avatar/src/{item.rs, lib.rs, tests.rs, fit/import.rs, fit/mod.rs}`, `crates/avatar/examples/import_garment.rs`, `crates/avatar/tests/fitting.rs` (a `Source` literal), `crates/avatar/CLAUDE.md`, `crates/project/src/{lib.rs, packed.rs, character_library.rs}`, `crates/editor/src/build_game.rs`, `crates/editor/src/app/files/export.rs`, `crates/editor/src/character_workspace/browser_tests.rs` and `crates/player/src/fit_gate_tests.rs` (`Source` literals), `docs/development/character-studio/{architecture.md, asset-quality.md (§6), references/README.md}`, `docs/project/export.md`, `assets/characters/realistic/README.md`, `wiki/Engine-Systems/Character-Data.md`, `wiki/Roadmap/Known-Limitations.md`.

## Interfaces

- `axcenity_avatar`: `Source { .., origin: Option<Origin>, provenance: Option<String> }` (now `Default`), `Origin`, `Provenance`, `LicenceRecord`, `SourceFile`, `LicenceTerms`, `ACCEPTED_LICENCES`, `licence_terms`, `ORIGINAL_LICENCE`, `PROVENANCE_VERSION`, `PROVENANCE_SUFFIX`, `is_imported`, `check_item_provenance`, `read_provenance`, `check_provenance`, `character_credits`, `CharacterCredits { required, thanks, uncleared }` with `text(game)`, `Credit`; `fit::ImportOptions { keep_placement }`, `fit::import_garment_with`.
- `axcenity_project`: `CHARACTER_CREDITS_FILE`, `character_credits(folder) -> Option<(CharacterCredits, String)>`; `pack_game*` add the file.
- Editor: `BuiltGame::character_credits`, `build_game::credits_note`.
- **Compatibility decision** (plan 11.3): the two `source` fields are optional; item manifests ignore unknown fields in older engines, so the new enum value set of `Origin` can't break an older engine reading a manifest; no description or project-file field changed. The provenance record is a new file with its own `format_version` (1).

## Tests (red first: `licensed-intake-red-run.txt`)

- `cargo nextest run -p axcenity-avatar -p axcenity-project --no-fail-fast`: 437 tests run: 437 passed, 2 skipped (the ignored capture-style checks).
- `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(/build_game|browser|round_trip/)'`: 67 tests run: 67 passed.
- `cargo nextest run -p axcenity-player --no-fail-fast -E 'test(/fit_gate|wardrobe/)'`: 22 tests run: 22 passed (the player builds with the `Source` change).
- `python -I assets/characters/realistic/import/test_intake.py` (Blender's Python): 20 tests OK (provenance: hashes, refusals, attribution line, record holes; geometry: cube, flipped face, degenerate and n-gon, non-manifold, split seams; UVs: density 20.48 px/cm on a square metre at 2048, overlap, stretch; skinning: four influences and normalized, arm and side kept apart, the check's counts; conform: a T-pose sleeve to the 45-degree rest pose exactly; settle: a plate stops 4 mm over a head; distances).
- `AXCENITY_PROVENANCE_LIBRARY=<proof library> cargo test -p axcenity-avatar --test shipped_provenance -- --ignored --nocapture`: 1 passed (20 items validate and pass the provenance check; prints the credits).
- `cargo clippy -p axcenity-avatar -p axcenity-project -p axcenity-editor -p axcenity-player --all-targets -- -D warnings`: clean (0 warnings). `cargo fmt --all`: clean. `python -I scripts/check-wiki.py`: 114 pages, 0 errors (57 warnings, all staleness notes on pages this branch didn't touch).
- New Rust tests: 17 in `tests::provenance`, 3 in `shipped_provenance` (one ignored, env-driven), 1 in `intake_placement`, 4 in `project::character_credits`, 1 in `character_library`, 2 in `build_game`.

## Dependencies

None added (Rust or Python). The scripts use the bpy environment the wardrobe already uses (bpy 5.x, NumPy, Pillow, `mathutils` from bpy).

## Measurements and visual evidence

All in [licensed-intake/README.md](licensed-intake/README.md). In short: 25 s for the whole chain; the hat's LOD0 10,303 triangles (0% quads, 17 non-manifold edges left), 0 UV overlap, 0.3% of its area over 10% stretch, 30.2 px/cm; LOD1 and LOD2 fail stretch (23.8%, 50.3%) from the collapse decimation; 4.0 mm least clearance, none inside; the engine fit's push-out at the brim's tips (up to 32.8 mm) undone by the rigid step. On the sheet (looked at whole and cropped): the hat seats on both average bodies, keeps its seat and shape in the walk, shows its own canvas, band and eyelets; the rear view's tilted-up back brim, nearly edge-on from hip height, lets the back of the head show up to the band, which a reviewer may read as a missing piece of brim (it is the geometry). Arms raised and shoulder deformation were not captured (a hat is untouched by them).

## Open issues

- The pose conform and the arm-and-side split are proven on made meshes only, not on a real garment (the hat needed neither); FBX, OBJ and `.blend` input, `decimate` and `remesh` are not run on a real asset.
- No retopology: decimate and remesh are help; a scan stays as it came. Collapse decimation fails §2's stretch on the simpler levels.
- Placement is numbers in the spec found by trying (about 25 settings for the hat); no automatic opening-to-head or shoe-to-foot fit.
- The engine fit binds far vertices to skin along their normals (a brim's tip to a shoulder). `rigid_bone` works round it for rigid accessories, at the cost of the body's shapes (the hat doesn't follow the head's size; on the feminine head it stands a little further from the temples). For garments this is the tee-walk specialist's area.
- No headwear slot on the realistic base: the proof used `hair` (accessories, R8, should add one).
- Only base colour, normal and ORM maps are carried.

## For the lead at merge

- **`Source` literals**: `Source` gained two fields. Any `Source { author, licence, url }` literal another branch adds must end with `..Source::default()` (this branch fixed the five in the tree: the example, `fitting.rs`, `intake_placement.rs`, `browser_tests.rs`, `fit_gate_tests.rs`).
- The content-state specialist's check goes beside `check_provenance` (both are separate functions; `tests.rs` gains one `mod provenance;` line, `lib.rs` one `mod` and one `pub use` block).
- Nothing generated was rebuilt (no catalogue, manifest, preset or model changed). The shipped items are all original work and pass the check as they are; when the generators next run, writing `"origin": "original"` explicitly is optional (unset reads as original).
- `fit.py` calls the `import_garment` example with `--placed`: the release example must be rebuilt after the merge (`cargo build -p axcenity-avatar --release --example import_garment`).
- `progress.md` and the plan's status were not edited (yours).
