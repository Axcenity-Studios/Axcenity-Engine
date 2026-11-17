# Hand-back: the lead's track R2a, `r2a-live-identity` (live identity, ADR-0041)

Task: a character on screen is its base model's shared mesh plus live identity weights and bone changes (plan section 0, decision 9; [ADR-0041](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md), which supersedes ADR-0038). The bake stays as an explicit API. Branched from `refinement-r1-wave1` (`6267cb98`).

## What changed

- **Live identity in `assets`.** `Model::live(&Identity) -> LiveIdentity` works out what the bake does to one model and gives it as data applied to each pose, with no vertex copied. It holds three things:
  - each identity shape's weight, by morph index;
  - the bone changes added after the clips (`LiveIdentity::settle`, as `Model::settle` did for derived models);
  - one matrix per joint. That matrix is `fit · inverse_bind⁻¹ · inverse_bind' · fit⁻¹ · carry`: the bake's re-seated bind and its carried surface (eyes and teeth moved and scaled by their bones), folded into the joint because neither depends on the pose.

  `LiveIdentity::dress(&mut Posed)` sets the weights and multiplies the joints (`Posed::rebind`, new). `Model::bake` and `Model::live` share one `prepare`, so they can't drift apart.
- **`Assets`.** `register_identity` registers an identity by key without making a model. `identity(key)` looks it up. `derived_count()` counts bakes. `derive` registers the identity the same way.
- **`anim`.**
  - A MeshPart names the identity it wears in the attribute `Identity` (16 hex digits, `anim::IDENTITY`; `identity_key`, `set_identity_key`).
  - `Animators` keeps a `LiveIdentity` per model and identity (`live_identity.rs`). Past 1024 kept, the ones not asked for this frame or the last are let go; the limit grows to twice what's in use, so a crowd's are never remade (see Review fixes).
  - In both posing paths the identity is settled right after the clips, where `Model::settle` was, and dressed last in `skinned`, after the face and the correctives.
  - Followers take the leader's weights by name as before, then put on their own identity: the body's, plus the item's own shapes.
- **The applier (`gameplay::description`).**
  - `apply_description` gives the body and every part its **base** `MeshId`, registers the identity and writes its key. A part with item parameters gets the body's identity plus a layer of its item's shapes (`ResolvedPart::shapes`), so item parameters are live weights on the item's part.
  - New `bake(world, assets, character)` and `is_baked`. `bake` swaps each owned MeshPart to the derived `MeshId` and removes `Identity`. Nothing calls it by default.
  - `DescriptionSync` applies kept descriptions as before, and bakes a character again if it was baked (a finished NPC stays finished where it's loaded).
  - The refit on a model change asks the base model, so it never bakes.
- **The editor:**
  - **The 120 ms drag throttle is gone** (`DRAG_APPLY_EVERY` and `refresh`'s parameter; one call in `shell.rs`). The measurement is below.
  - Tests changed: the agreement test (now 0.2 mm per corner), the slider test (a new identity key, nothing baked) and the timing test (a live pass and a baked pass).
- **The capture tool:** `capture_in` is split into `stage_world` and `draw_staged` (new, `pub(crate)`) so a staged world can be baked between draws. VIEWS, LIGHTS, MORE_VIEWS and `Settings::default` are untouched.

## Interfaces (new or changed)

```rust
// assets
Model::live(&Identity) -> LiveIdentity
LiveIdentity::{is_empty, settle(&mut [BoneTransform]), settles, weights() -> &[(usize, f32)], dress(&mut Posed)}
Posed::rebind(&mut self, binds: &[Mat4])
Assets::{register_identity(&Arc<Identity>) -> Option<u64>, identity(u64) -> Option<Arc<Identity>>, derived_count() -> usize}
// anim
IDENTITY ("Identity");  identity_key(&World, part) -> Option<u64>;  set_identity_key(&mut World, part, Option<u64>)
// gameplay::description
bake(&mut World, &Assets, character) -> Result<usize, String>;  is_baked(&World, character) -> bool
// apply_description: signature unchanged; MeshIds are now the base ones
```

A saved or replicated world carries the key, not a derived `MeshId`. A run that hasn't applied the description draws the base meshes until `DescriptionSync` registers the key, as before with derived IDs. **No change to the `avatar` schemas, the description format or the Lua API.**

## Tests

Written first and seen failing: [r2a-live-identity-red-run.txt](r2a-live-identity-red-run.txt) (12 of 34 failed, against stubs of the new API: `Model::live` returning nothing, the applier still deriving).

| Test | What it proves |
|---|---|
| `assets` `an_identity_worn_live_poses_the_mesh_where_its_bake_does` | four identities (shapes only, a re-seated joint, a carried and scaled surface, both layers), at rest and turned: every corner where the bake puts it (1e-5), on the model's own mesh |
| `assets` `a_registered_identity_is_found_by_its_key_and_bakes_nothing` | registering makes no model; one `derive` and lookup is one bake, counted |
| `player` `a_parameter_change_on_an_applied_character_changes_weights_only` | build 1 → 0.4 changes the body's and the jacket's `Build_Heavy` weight; no MeshId changes; `derived_count` stays 0 |
| `player` `an_items_own_parameters_are_live_weights_on_its_part` | an item parameter (added to the tee in the test's library) is a weight on the tee alone; changing it bakes nothing |
| `player` `an_explicit_bake_puts_every_corner_where_the_live_character_has_it` | `bake` derives every owned part and removes `Identity`; every corner within 0.2 mm of live; a fresh run's `DescriptionSync` keeps it baked with the same key; applying again is live |
| `player` `applied_parts_wear_the_bodys_identity_live_and_follow_its_live_shapes` (was `…_baked_in_…`) | the jacket's posed rest is the base's `Build_Heavy` to 0.1 mm; masks at 1; lashes blink with the lid; the identity holds under the expression |
| `player` `the_frames_bone_offsets_move_the_skeleton_and_carry_the_eyes` (rewritten on posed meshes) | the joints at the manifest's offsets (0.1 mm); the skin moved by the shape alone; the eyeballs moved by their bone (2 mm) and grown by its scale (1%); under the idle the eye bone stays where the settled skeleton has it |
| `player` `a_kept_description_is_applied_again_where_the_world_is_loaded`, `a_description_applied_in_the_player_wears_its_parts`, `description_lua::a_script_builds_…` | the same checks, on keys and base MeshIds instead of derived ones |
| `player` `character_capture::an_explicit_bake_draws_the_same_pixels_as_the_live_character` | the realistic feminine frame with fat 0.7, a longer nose and a larger left eye, front, side and face under studio light, drawn live then baked: **0 pixels differ by more than 8; mean difference 0.0002, 0.0000, 0.0002** |
| `editor` `a_character_file_applied_in_a_game_matches_the_workspaces_preview` (restored from `a_saved_…`, which only compared MeshIds) | the same models, and every corner of every part within **0.2 mm**, preview against game, after 20 frames, fat 0.6 and nose 0.5; asserts no derived `MeshId` in either World |
| `editor` `a_slider_changes_the_realistic_preview_while_it_is_dragged` | a slider gives a new identity key on the same base model, every drag step shows it, the drag is one undo step, nothing is baked |
| `player` `character_capture::the_same_description_captures_identical_pictures_and_every_view_is_drawn` (CO2) | unchanged, passes |

Summary lines (debug, this worktree):

- `cargo nextest run -p axcenity-assets -p axcenity-anim -p axcenity-gameplay -p axcenity-avatar -p axcenity-scene -p axcenity-project -p axcenity-player -p axcenity-editor --no-fail-fast`: `Summary [ 177.071s] 2481 tests run: 2481 passed (21 slow, 2 leaky), 33 skipped` (before the throttle was removed)
- after removing the throttle, `cargo nextest run -p axcenity-editor --no-fail-fast`: `Summary [  83.402s] 1342 tests run: 1342 passed (9 slow), 9 skipped`
- the gates, after implementing: `Summary [  16.584s] 34 tests run: 34 passed, 1735 skipped`
- `cargo clippy -p axcenity-assets -p axcenity-anim -p axcenity-gameplay -p axcenity-player -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all`: applied.
- `python3 -I scripts/check-wiki.py`: 0 errors (58 warnings, all "may be stale" on pages this change doesn't touch).

## Measurements (release, RX 9070 XT, Ryzen 9 9950X3D, pinned to the V-cache CCD with `start /affinity FFFF`; other agents were building)

**The before figures are the bake path in the same binary.** `bake` after each apply is exactly what every apply did before R2a. R1's own figures are in [../r1/drag-timing.md](../r1/drag-timing.md).

### Drag apply time

`cargo test -p axcenity-editor --release -- --ignored drag_apply_time --nocapture`. 20 steps per parameter, milliseconds (min / median / max).

| Parameter | live: apply | live: apply + step + draw list | baked: apply + step + draw list | R1 (before): apply / apply + step + draw list, median |
|---|---|---|---|---|
| `fat` | 0.07 / 0.11 / 0.15 | 0.14 / 0.24 / 4.76 | 6.64 / 8.05 / 20.23 | 2.70 / 6.61 |
| `muscle` | 0.06 / 0.08 / 0.20 | 0.13 / 0.15 / 2.85 | 5.87 / 6.78 / 18.66 | 2.81 / 6.09 |
| `height` (attribute only) | 0.06 / 0.06 / 0.14 | 0.14 / 0.14 / 3.25 | 0.27 / 0.31 / 9.27 | 0.11 / 0.22 |
| `fat_abdomen` | 0.08 / 0.12 / 0.17 | 0.16 / 0.24 / 4.00 | 5.65 / 7.34 / 18.51 | 2.56 / 6.47 |
| `nose_length` | 0.06 / 0.07 / 0.15 | 0.13 / 0.14 / 3.38 | 5.11 / 6.96 / 21.31 | 2.33 / 6.44 |
| `eye_size_left` | 0.06 / 0.07 / 0.11 | 0.23 / 0.25 / 2.93 | 5.01 / 6.98 / 21.04 | 2.17 / 6.21 |
| `jaw_width_left` | 0.07 / 0.07 / 0.11 | 0.14 / 0.15 / 3.32 | 5.03 / 6.20 / 17.97 | 2.18 / 5.36 |

A live parameter change costs about 0.1 ms to apply and 0.15 to 0.25 ms with the next frame's posing and draw list, 30 to 50 times less than the bake. The worst step (3 to 5 ms) is the first: new `LiveIdentity`s and the GPU's first sight of the shared mesh. A frame of the one-character preview costs about 0.85 ms of GPU time (below). **So the throttle was removed: every drag step is applied.** Not covered: the minimum GPU class (RX 6600 / RTX 3060), and a dressed character with hair (none exists yet).

### Frame time, N realistic characters of distinct descriptions

`cargo test -p axcenity-player --release -- --ignored bench_distinct_characters --nocapture --test-threads 1`. N default characters in a grid, each with `randomize(…, Scope::All, seed i + 1)` (every identity distinct, checked), idling, at 1280 × 720 on High. Each number is the fastest batch's median of 5 × 60 frames.

| N | mode | GPU frame | Shadows | Depth prepass | Scene | CPU posing | CPU draw list | CPU draw (submit) | batches / draw calls | mesh memory |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | live | 0.851 ms | 0.362 | 0.126 | 0.235 | 0.151 ms | 0.020 | 0.635 | 17 / 65 | 7.8 MB |
| 1 | baked (before) | 0.480 ms | 0.137 | 0.055 | 0.159 | 0.144 | 0.020 | 0.616 | 17 / 65 | 2.6 MB |
| 10 | live | 3.958 ms | 1.995 | 0.666 | 1.164 | 0.831 | 0.091 | 0.966 | 63 / 175 | 8.1 MB |
| 10 | baked (before) | 1.667 ms | 0.641 | 0.236 | 0.660 | 0.787 | 0.110 | 1.258 | 156 / 580 | 25.5 MB |
| 50 | live | **15.120 ms** | 7.519 | 2.588 | 4.687 | 3.333 | 0.423 | 2.682 | 205 / 325 | 11.3 MB |
| 50 | baked (before) | **6.700 ms** | 2.817 | 0.994 | 2.724 | 3.325 | 0.684 | 4.857 | 796 / 2978 | 123.2 MB |

**Reading.**

- **The CPU and memory win.** Live characters share one mesh per part: mesh memory is 11 MB against 123 MB for 50, draws batch (205 batches and 325 draw calls against 796 and 2978), the CPU's submit is 45% cheaper, and nothing is baked (859 derived models for 50 baked characters).
- **The GPU loss, as ADR-0041 expected.** The renderer morphs in every pass, and its shader walks every delta of every corner whatever its weight (`morph_corner` in `shader.wgsl`). The realistic body's identity shapes (about 150), which the bake dropped, are now in that loop in every pass: the shadow cascades, the depth prepass and the main pass. Per character that is about 0.30 ms of GPU against 0.13 ms baked (2.3 times), shadows the most (0.15 against 0.056 ms).
- **The budget.** 50 live characters take 15 ms of GPU on the reference card: too slow for crowds, fine for the studio (one character, 0.85 ms) and for a few characters in a game. **Live weights for the realistic body's ~150 identity shapes are too slow in every pass at crowd scale until CO8.**

**Proposed CO8 order (RA7):**

1. A compute pass that morphs and skins each skinned item **once per frame** into a posed vertex buffer that the shadow, depth and main passes read as a plain mesh. This removes the factor of the pass count; shadows alone are half the live cost.
2. In that pass, walk only the shapes with a non-zero weight. Today every delta of a corner is visited. Splitting a mesh's deltas by shape (per-shape corner ranges) and dispatching over the item's active shapes makes a shape the description never moves cost nothing (ADR-0041 decision 2).
3. Build the ray tracing BLAS from the same posed buffer (`ray_skin.wgsl` skins today but doesn't morph). This also gives ray-traced shadows the identity and expression shapes.
4. Until then, a game with a crowd can call `gameplay::description::bake` on NPCs it won't change (ADR-0041's fallback; tested to draw the same pixels).

## Visual evidence

- [r2a-live-identity-feminine-heavyset.png](r2a-live-identity-feminine-heavyset.png): `realistic-feminine-cast-heavyset` live (no derived model anywhere), six views under studio and hard light.
  - The feminine frame, the belly and fuller hips are there.
  - The sports bra and briefs follow the body's shapes by name.
  - The eyes sit in their sockets and the brows on the brow ridge.
  - Nothing I could see differs from a baked character. The capture test above measures the two identical to within rounding.
- [r2a-live-identity-face-suite.png](r2a-live-identity-face-suite.png): `realistic-masculine-cast-distinctive`'s face suite, live. It checks that expressions go on over a live identity:
  - The smile, the frown and closed eyes read on the distinctive face. The lashes close with the lids, because a follower takes the leader's weights and then its own identity.
  - The open mouth shows the teeth carried with the jaw.
  - The open mouth is still the bell shape with pointed corners. That is the baseline's OPEN-14 (R9), not this change.
  - The skin's stippled shadow edge is the baseline's too.

  I judged the pictures for breakage only (live against baked). This track changes no look, so a review against references doesn't apply.

## Dependencies added

None.

## Generated files

None regenerated: no manifest, model, catalogue, preset or benchmark changed.

## Open issues

- **Hardware ray-traced effects of live characters** lack the identity and expression shapes (the bone changes and the joint corrections are in them). `ray_skin.wgsl` skins the ray tracing copy without morphing, and that copy is in the one TLAS that ray-traced shadows, ambient occlusion, reflections and hardware GI all trace (`ray_shadows.rs`, `ray_ao.rs`, `ray_reflections.rs`, `ray_gi.rs`). Before R2a the derived rest mesh carried the identity, so with any of these on this is a regression for live characters; all four are off at every preset (`render-types` `graphics.rs` tests). A baked character's identity is right. It is RA7's, with CO8 step 3.
- **GPU cost** with many live characters (above): CO8.
- **The Game Editor's Face Rig tab** (`editor/src/character_studio.rs`, `anim::pose_face`) poses a copy of a MeshPart at rest and doesn't put on the part's live identity, so an applied character's face shows the base face there. `CharacterWorkspace::head()`'s fallback before the first frame uses `Model::settle` and misses a live identity's bone changes. Both are editor code, not touched here (the task allows only the throttle). Each needs `LiveIdentity` from `assets.identity(identity_key(...))` and `Model::live`.
- **Edit mode** (CO13, unchanged): a placed character's kept description isn't applied in the Game Editor's edit mode, so it shows base meshes, as before.
- **Identities are kept for the run** (audit D4, unchanged). A realistic identity names every identity shape, even at weight 0 (about 150 `(String, f32)` entries with the corners and masks) plus its bone changes: about 8 to 10 KB each (estimated from its entries, not measured), and one more per part with item parameters. A drag now registers one per frame instead of one per 120 ms: about 0.6 to 1.4 MB a second of continuous dragging at 60 to 144 Hz, against about 80 KB a second with the throttle, so a long editing session can reach hundreds of MB. The fix is a reference-aware prune: drop keys that no MeshPart's `Identity` attribute names, run from the workspace when a drag ends (or the applier unregisters a character's previous key). In Known limitations.
- **Clothing shapes by binding transfer** (ADR-0041 decision 4) is CO9's. `fit: shapes` garments already follow live weights by name.

## For the lead at merge

- `crates/editor/src/character_workspace/shell.rs` changed by one line (`refresh()`), with `character_workspace.rs`'s throttle. RA1's branches that call `refresh(bool)` need the argument dropped.
- `crates/player/src/character_capture.rs` gained `draw_staged` (split from `capture_in`, same behaviour) and one test. `bench_tests.rs`'s `run`, `camera` and `Steps` are now `pub(crate)` for the new bench.
- Anything that asserted a derived body `MeshId` after `apply_description` must look at `anim::identity_key` instead. All such tests in this repository were updated.
- Update the plan's 11.5 R2a row (implemented in part: the applier, followers, `DescriptionSync`, item parameters and the bake are done; CO8 is not) and the progress entry. ADR-0041's status line says "implemented in part".

## Review fixes

The review asked for two blocking fixes. Both are done.

### 1. The LiveIdentity cache no longer drops everything at 1024

`LiveIdentities::of` used to clear the whole map when it reached 1024 entries. A crowd wearing more than 1024 (model, identity) pairs (about 100 live characters at 8 to 12 parts each) therefore emptied and refilled the map every frame, and nearly every part ran a full `Model::live` every frame.

Now (`crates/anim/src/live_identity.rs`):

- Each entry keeps the frame it was last asked for. `Animators::update` calls `LiveIdentities::next_frame` once per update, and a hit refreshes the stamp.
- When the map reaches its limit, only entries not asked for this frame or the last are let go (`retain`). An entry the current frame is using is never dropped.
- After letting go, the limit becomes twice what's left (at least 1024). A crowd that needs more than 1024 keeps them all, and letting go stays rare, so the `retain` is amortised and doesn't run on every miss.
- A drag still stays bounded: one new identity per frame, the old ones let go once 1024 are kept.

Tests, written first (red run appended to `r2a-live-identity-red-run.txt`; the old cache failed with "frame 1 remade 2000 live identities"):

- `live_identity::tests::a_crowd_larger_than_the_cache_limit_is_not_remade_every_frame`: 2,000 distinct (model, identity) pairs asked for on three updates in a row. The first makes 2,000 and the next two make none (counted by a miss counter, `made`).
- `live_identity::tests::identities_no_part_wears_any_more_are_let_go`: 3,072 identities, a new one each frame, as a drag makes them. The map never holds more than 1024, and the one worn this frame is still there.

Runs after the fix:

- `cargo nextest run -p axcenity-anim --no-fail-fast`: Summary [3.604s] 182 tests run: 182 passed, 0 skipped
- `cargo nextest run -p axcenity-anim -p axcenity-assets -p axcenity-gameplay -p axcenity-scene -p axcenity-avatar -p axcenity-player --no-fail-fast`: Summary [158.239s] 900 tests run: 900 passed (7 slow), 23 skipped (the gate tests among them: the explicit bake's pixels, weights only, item parameters, the frame's bone offsets, CO2 determinism)
- `cargo nextest run -p axcenity-editor -E 'test(a_character_file_applied) | test(a_slider_changes_the_realistic) | test(character_workspace)'`: Summary [27.673s] 48 tests run: 48 passed, 1303 skipped
- `cargo clippy -p axcenity-anim -p axcenity-assets -p axcenity-gameplay -p axcenity-player --all-targets -- -D warnings`: clean; `cargo fmt --all --check`: clean

No frame-time numbers changed: the 50-character bench (about 500 pairs) never reached the old limit, so the fix changes nothing it measured.

### 2. Wiki and architecture pages

- `wiki/Engine-Systems/Animation.md`: the settle-step bullet and its diagram node now describe live identity (`LiveIdentity::settle` after the clips in both posing paths, `LiveIdentity::dress` last in `skinned` and on followers after the leader's shapes, `Model::settle` only for a baked model), link ADR-0041, and add `live_identity.rs` to the page's sources. The correctives bullet no longer says "after the identity bake".
- `wiki/Roadmap/Known-Limitations.md`, Characters: the old "identity bake's cost isn't measured" entry is replaced by three: live identity's GPU cost in every pass until CO8 (the measured 1, 10 and 50 numbers, with the advice to bake crowds that don't change); hardware ray-traced shadows, AO, reflections and GI tracing an unmorphed copy (all four trace the one TLAS that `ray_skin.wgsl` poses; all are off at every preset); and registered identities kept for the run, with the corrected memory figure. The edit-mode entry no longer speaks of derived `MeshId`s alone.
- `wiki/Engine-Systems/Characters.md`: the body parameters line says identity shapes are worn live or baked on request, and the limits paragraph names all four ray-traced effects.
- `docs/development/character-studio/architecture.md`: lines 5, 12, 24, 39 (the data flow), 166, 288 and 449 say identity is worn live (ADR-0041) and the bake is optional (`gameplay::description::bake`).
- This hand-back: the ray-tracing open issue names all four effects. The memory wording is corrected: about 8 to 10 KB per identity (estimated from its ~150 entries, not measured), about 0.6 to 1.4 MB a second of continuous dragging at 60 to 144 Hz against about 80 KB a second with the throttle. That can reach hundreds of MB over a long session, not "megabytes". The planned fix is a reference-aware prune when a drag ends.

`python3 -I scripts/check-wiki.py`: 113 pages, 0 errors, 58 warnings (the same staleness warnings as before).

### Non-blocking items not changed here

- The editor agreement test would also pass if neither World wore its identity. The player tests check the weights; a check that the identity shows (a non-zero `Fat`-driven weight, or corners more than 1 mm from the base's) is left for the lead or RA1, since this delegation may not change editor code beyond the throttle.
- The Face Rig tab and `CharacterWorkspace::head()`'s fallback (editor code) still don't wear a live identity. Assign them to RA1 or the lead before R4.
- `Humanoid:ApplyDescription` on a baked character makes it live again. This is documented in architecture.md §16; a note in `docs/scripting/api-character.md` is left for the scripting docs' owner.
- A "before" capture at `refinement-r1-wave1` for the two cast members wasn't made. The live-against-bake pixel test is the before-and-after evidence for R2a.
- Plan 11.5's R2a "Done when" items that belong to CO8 (20 characters with 150 shapes as fast as today with 41; ray-traced shadows showing expressions) remain unmet. The row stays "implemented in part".
