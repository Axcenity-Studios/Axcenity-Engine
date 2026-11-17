# R1 step 1: the shared contracts (branch `r1-contracts`)

The contracts of plan section 11.3 in `crates/avatar`, their data for the realistic body, the shipped presets, and the docs. Every new field is optional with a default; no `format_version` changed; no value was added to any existing enum. Decision record: [ADR-0040](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0040-Character-Metadata-Item-Capabilities-And-Presets-Are-Data-In-The-Manifests.md) (Proposed). Contract text: `architecture.md` sections 4 to 8, 15, 15a (presets), 15b (Randomize), 16.

## What was built

| Task item | Done | Where |
|---|---|---|
| 1. Parameter `about`, `feature`, `level`, `random`; `Parameter::level()`; level and advanced agree; Group and Slot `about`, `icon` | Yes. New enum `Level {Quick, Detailed, Advanced, Other}` | `manifest.rs` |
| 2. `features` and `handles` with every check; anchor bones in `check_model` | Yes. `Feature`, `Anchor`, `Handle`, `Drive`; `BaseManifest::{feature, handle, group, category_of, feature_of}` | `manifest.rs`, `model_check.rs` |
| 3. Item `about`, `parameters`, `also_fills`, `requires`, `excludes`, `limitations`, `adjust`; MaterialSlot `label`, `values`, `role`; Variant `thumbnail`, `parameters` | Yes. `ItemParameter`, `Limitation`, `Adjust`, `MaterialRole {Primary, Secondary, Accent, Other}` | `item.rs`, `material.rs` |
| 4. WornItem `parameters`, `transform`, `extra`; validate learns item parameters, requires, excludes, transform (and `also_fills` slots); resolve: item parameter shapes | Yes, resolve side. `Transform`; `ResolvedPart::{shapes, transform}`; resolve also leaves out what `also_fills`, `excludes` and `requires` forbid, the later item winning as for slots. **The bake in `gameplay::description` is not done** (open issue 1) | `description.rs`, `resolve.rs` |
| 5. Presets as data; catalogue `presets`; `Library` loads them; `apply_preset`; `catalogue.py` and `Catalogue::of` together; project and `stage.py` copy them | Yes. Also `check_preset`, `matches_preset`, `PresetKind` (with `Other`), `PresetInfo`, `CataloguePreset`, `PRESET_SUFFIX`, `Library::{add_preset, preset, preset_path, presets}`. `add_character_packages` and `stage.py` also copy variant pictures and a groom's mesh now (audit D3) | `preset.rs`, `catalogue.rs`, `library.rs`, `crates/project/src/character_library.rs`, `packaging/stage.py`, `assets/characters/catalogue.py` |
| 6. Randomizer | Yes: `randomize(&Description, &Scope, &BTreeSet<String>, seed, &Library)`, `Scope {All, Category, Feature, Parameters}`, `random_span`, SplitMix64 in the crate | `randomize.rs` |
| 7. Data | Yes: all 75 realistic parameters have `about`, `feature`, `level`; 23 features (13 body regions asked for, plus `body` for the whole-body sliders, plus 9 face) with anchors measured on the model; handles empty; 29 presets | `assets/characters/realistic/{manifests.py, faceparams.py, anchors.py, presets.py, presets/}` |
| 8. Docs | Yes | `architecture.md`, `testing.md`, the package README, ADR-0040 and the ADR index, `wiki/Engine-Systems/Character-Data.md`, `crates/avatar/CLAUDE.md` |

### Interfaces other agents build on

```rust
Parameter::level() -> Level                       // absent: Advanced if `advanced`, else Detailed; unknown reads as absent
BaseManifest::feature_of(&Parameter) -> Option<&Feature>   // its `feature`, or the feature with its group's ID
BaseManifest::category_of(&Parameter) -> Option<Category>
ItemManifest::{parameter(id), slots(), conflicts_with(&ItemManifest)}
MaterialSlot::offered() -> Vec<&ValueSpec>        // `values`, or the whole shading model
Description::preset() -> Option<PresetInfo>
apply_preset(&mut Description, &Description, &Library) -> Result<(), Problem>   // changes nothing on Err
matches_preset(&Description, &Description, &Library) -> bool                    // "Custom" = none matches
check_preset(&Description, &Library) -> Vec<Problem>
randomize(&Description, &Scope, locks: &BTreeSet<String> /* parameter and slot IDs */, seed: u64, &Library) -> Description
ResolvedPart { .., shapes: BTreeMap<String, f32>, transform: Transform }
```

### Choices made where the task left room

- **Level data**: quick = `frame`, `height`, `muscle`, `fat` and the face's main sizes (`eye_size_*`, `nose_length`, `mouth_width_*`, `jaw_width_*`); advanced = exactly what was advanced before (the eight proportions other than height, the 16 regional build sliders, `eye_depth_*`, `ear_height_*`, `nose_tilt`, `jaw_shift`); detailed = the rest. Totals: 11 quick, 34 detailed, 30 advanced. The brief puts regional body sliders at detailed (audit risk 6): that is R3's call, and needs `advanced` changed with it.
- **Features**: a `body` feature (whole body) holds `frame`, `height`, `muscle`, `fat`, so every parameter has a feature. `leg_length` belongs to `thighs`, `arm_length` to `arms`. The face's features are its nine groups, so `feature` equals `group` there.
- **Anchors** (`anchors.py`, numpy, no Blender): each feature's vertices are those its shapes move, weighted by how far (`Face_Nose_*`; `Masculine_Fat_Chest` + `Masculine_Muscle_Chest`), or its region of the region map (`TEXCOORD_1`) for head, neck, hands, waist and feet. The anchor is their weighted centre, snapped to the nearest of them for the face (on the skin) and left at the centre for the body; its bone is the one carrying most of those vertices (skin weights); the offset is from that bone's head at bind, from the inverse bind matrices; `radius` is the weighted RMS distance. Masculine frame only. No rebuild: `manifests.py` reads `Realistic.build.json` and the GLB, so `RealisticParts.glb` is unchanged. Regenerating gave `Realistic.base.json` equal to the old one apart from the new keys (checked by comparing the parsed JSON) and the three item manifests byte-identical.
- **Body presets are per frame**: the reference values differ for the two frames, and a body preset must not change `frame`, so each of the seven is two files tagged `male` or `female` (the frame choices' labels). That makes 14 files, not 8: brief §4.1's eighth, Custom, is the state "no preset matches" (`matches_preset`), as plan section 11.5 R3 says. **RA1 must filter body presets by the frame tag.**
- **Kinds own categories**: `body`, `face`, `hair`, `skin` own the parameters of groups in that category (and hair the items of hair slots, skin the base's material values); `outfit` owns items in clothing and accessory slots except the slots a choice fills; `character` is everything but the name. A worn item is copied whole, its own `extra` included; the description's `extra` (the `preset` block) never is.
- **Randomize**: no `random` means the middle half of the range; values rounded to 0.001; a parameter with choices is drawn when the scope names it (so `All` and `Category(Body)` draw the frame unless it is locked); each slot draws one of the fitting items or nothing, a `multiple` slot up to two (so an item that `requires` another there can follow it); material values are left alone (skin is R5's).
- **New enums read unknown values as `Other`** (`#[serde(other)]`), so they can grow without breaking older engines.

## Tests

Written first. The red run is `r1-contracts-red-run.txt` beside this file: the avatar tests didn't compile (142 errors in the unit tests, 16 in `shipped_library`, every one a missing contract item), and the project test failed with the copying loop taken out.

New tests (`crates/avatar`):

- `src/tests/contracts.rs` (16): round trips with every new field; old manifests read unchanged and write none of the new keys; an old description reads unchanged; a worn item keeps parameters, transform and unknown fields; `level()` rules and an unknown level; `feature_of`; the rich fixtures are sound; every new base check fires (20 problems in one manifest, plus mirror-back and duplicate handles); anchor bones in `check_model`; every new item check fires (16); item checks against the base and the library (requires, excludes, variant pictures); item parameter shapes in `check_model`; a sound description with capabilities; a description breaking every item rule (10 problems); resolve gives item shapes (defaults, variant, worn, clamped, choices), material values in order and the transform; resolve leaves out excluded, doubly-filled and unmet-requirement items with reasons.
- `src/tests/presets.rs` (9): the block reads and survives a round trip; body (frame left alone unless named, nothing else touched, block never copied, `matches_preset`); face; hair and outfit (bald when empty; an outfit leaves the hair); skin and character; refusals change nothing; `check_preset` fires on wrong-kind parameters, items and materials and a bad block; library and catalogue hold presets (sorted, thumbnail resolved, no `presets` key when empty); `Library::read` from a folder, and a mismatched ID is a problem.
- `src/tests/randomizing.rs` (6): deterministic per seed; 1,000 seeds on two test libraries validate and resolve and every fitting item is drawn by some seed; values inside `random` / the middle half, choices, `max_sum`, item parameters; locks on parameters and a slot over 1,000 seeds; scopes (category, feature, list, clothing only) change only what they name; an unknown base comes back unchanged.
- `tests/shipped_library.rs` (4 new): `an_outfit_preset_leaves_the_chosen_underwear`; `every_shipped_preset_is_sound_listed_and_resolves` (files equal the catalogue, `check_preset` clean, resolves with nothing left out, applied to the default character it validates and matches, 14 body, 9 face, 6 character); `the_realistic_parameters_say_what_they_are` (75 parameters with a sentence, a level in step with `advanced` and an existing feature; the 22 named features exist; handles empty); `a_thousand_random_characters_on_each_shipped_base_are_sound` (both bases, a locked `nose_length` holds).
- `crates/project` `the_shipped_packages_are_copied_beside_the_built_in_one` now also checks the presets arrive file for file and the copied library validates.

Runs (green):

- `cargo nextest run -p axcenity-avatar --no-fail-fast`: **81 tests run: 81 passed, 0 skipped** (46 before).
- `cargo nextest run -p axcenity-project --no-fail-fast`: **241 tests run: 241 passed, 1 skipped**.
- `cargo nextest run -p axcenity-gameplay --no-fail-fast description`: **8 tests run: 8 passed**.
- `cargo nextest run -p axcenity-editor --no-fail-fast character`: **37 tests run: 37 passed**. `cargo nextest run -p axcenity-player --no-fail-fast description`: **11 tests run: 11 passed**. (Filtered runs of the crates that read Character Data, not their whole suites.)
- `cargo clippy -p axcenity-avatar --all-targets -- -D warnings` and `-p axcenity-project`: clean. `cargo check -p axcenity-gameplay -p axcenity-editor -p axcenity-player -p axcenity-server --all-targets`: clean (after adding the two new `Variant` fields to one struct literal in `gameplay/src/description_tests.rs`). `cargo fmt --all`: applied. `python scripts/check-wiki.py`: 0 errors (57 warnings, all staleness notes on other pages).

## Dependencies added

None. Randomize uses an in-crate SplitMix64; `anchors.py` uses numpy, which `manifests.py` already needed through `shapes.py` (run it with the Blender Python environment, or any Python with numpy).

## Visual evidence

- `r1-contracts-anchors-body.png` and `r1-contracts-anchors-face.png` (beside this file): every anchor drawn on the body's rest vertices, front and side, with its radius (blue: body, red: face). What I saw: the face's features sit where they should and in order (brows over eyes, ears at the sides at eye level, cheeks below and outside the eyes, the nose on the middle line, lips over chin, the jaw's two anchors at the angles of the jaw, behind and below the ears in the side view). The body's are at the centres of their parts: shoulders, chest above abdomen, back behind chest, hips and waist low on the torso, thighs, calves and feet down the legs, hands at the hands. Weak points: the forehead anchor is high, at the hairline (its shapes move the upper forehead), with a 7 cm radius that overlaps the brows; the nose anchor is 4 mm off the middle line (the nearest vertex to the centre); the arms' anchors are at the upper arm, and `body`'s 43 cm radius covers the torso. Those are for R2 to tune when it picks against them; nothing here is wrong enough to block.
- **In-engine captures of every shipped preset** (release `capture_characters`, sheets only, default VIEWS, LIGHTS and settings; outside the repository). Each preset was turned into a character file for its frame (a female body preset with `frame` 1 and the female underwear), plus the two default characters to compare. I looked at the sheets and measured the share of pixels that differ from the default of the same frame (top row, threshold 8 grey levels):
  - The six `character` presets reproduce the editor's: `male` and `female` are pixel-identical to the defaults (0.00%), as they should be.
  - **Face presets barely show**: 0.3% to 1.3% of the face close-up changes (Oval 0.27%, Square 0.52%, Broad 1.31%); at the sheet's size Round and Long look like the default. The values are the references' first guesses on today's 47 face parameters, which move a few millimetres each; the audit and the face references already say the shape presets need the missing jaw and midface controls (R4) and tuning by eye.
  - **Body presets on the male frame are subtle**: Heavyset 1.7% of the top row, with a softer chest and no belly in the side view, the baseline's known defect ("fat that looks like fat" is R3's); Muscular 3.1% and Broad 2.4% read best. The female body presets differ by 6% to 8%, but most of that is the height change (0.95 or 0.97) moving the framing, not shape.
  - Nothing broke: every preset resolved and drew, the underwear followed the frame, and no capture showed a new defect beside the baseline's.

## Open issues

1. **The bake of item parameter shapes** (`gameplay::description::apply_description`) is not done. The change itself is small (for a part with `shapes`, derive its model with the body's identity plus one layer of those shapes), but no item has parameters, so it can't be tested end to end; it belongs with the first item that has them (R6 hair). `ResolvedPart::transform` is likewise not applied yet (R8).
2. **Audit D2 is unchanged**: `add_character_packages` skips a base, item or preset the target already has, so an existing workspace folder or project keeps its old `Realistic.base.json` and sees none of the new metadata until the folder is cleared. RA1 will meet this at once. Fixing it (replace a package's files when the shipped ones differ, keep a project's own edits) is the lead's.
3. An older engine that writes a project's catalogue again drops `presets` (the files stay); the old D1 note is now true only of manifests and the catalogue.
4. Anchors are measured on the masculine frame; how far they sit off the feminine surface is unchecked (audit risk 3). A check could measure both frames.
5. Presets are starting values from the references, not tuned by eye. The editor still uses its six constants (RA1 switches to the data; `presets()` order equals the `character` presets' `order`).
6. The stylized base's parameters have no metadata (out of scope); its features list is empty.
7. ADR-0040 is a new number: if another branch (for example `co7-skin-values`) also writes ADR-0040, renumber at merge.

## What the lead must do at merge

- Generated files: `Realistic.base.json` and `catalogue.json` are generated. If RA2 or RA3 change `faceparams.py`, `shapes.py` or the model, rerun `manifests.py` (numpy), then `presets.py` and `catalogue.py`; resolve conflicts in the scripts, never in the JSON. New face parameters need an `ABOUT` sentence in `faceparams.py` (the manifest step fails with a `KeyError` without one), and new body parameters a `BODY_ABOUT` / `REGION_WORDS` entry in `manifests.py`.
- Check the ADR number and the ADR index row.
- `crates/gameplay/src/description_tests.rs` gained two fields in one `Variant` literal; any other branch that builds a `Variant`, `MaterialSlot` or `WornItem` by literal needs the same.
