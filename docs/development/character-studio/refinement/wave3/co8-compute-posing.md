# Hand-back: RA7, CO8 compute posing (`co8-compute-posing`)

Task: morph and skin each character **once per frame** into a posed buffer every pass reads, and that ray tracing builds from (plan section 1, CO8; [ADR-0041](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0041-Shared-Base-Mesh-With-Live-Identity-Morphs-Baking-Is-Optional.md) decision 2; plan 11.5 R2a's open part; audit-runtime-render §3.10). Branched from `refinement-wave2` (`b39a9ed7`).

## What changed

- **A posing pass** (`crates/render/src/renderer/posing.rs`, `crates/render/src/posing.wgsl`), first in every frame (`FramePass::CharacterPosing`, "Character posing"):
  - **What is posed** is a (skin, mesh) pair. The plan lists each pair its camera instances and casters use (`lighting.rs`: `PoseUse`, `DrawPlan::posed`; an instance or caster waits as `skin = [pair, PENDING]`). `ScenePipeline::pose_frame` keeps the pairs some view draws (a camera instance, or a caster a shadow view kept), and, while something traces rays, every animated draw within the ray tracing distance (`ray_tracing::within_reach`, shared with the RT scene). Each pair gets a range of the frame's **posed buffer**, and its instances and casters get `skin = [first corner, 1]` (`[0, 0]`: drawn at rest).
  - **Active shapes only.** `active_shapes` lists, per pair, the shapes whose weight isn't 0 (and is a number), each with its moves. `cs_shapes` runs one thread per move of the active shapes and adds weight × move to its corner's sums, atomically in fixed point (2^20 steps a unit), so the total doesn't depend on the order and a frame poses the same every time. A shape at weight 0 is never read, and never uploaded.
  - `cs_pose` runs one thread per posed corner: the rest corner plus its sums (set back to 0 for the next frame), a turned normal renormalized, the joints blended as `Posed::skin` does. It writes five words: the position (three `f32`), and the normal and tangent (octahedral, two snorm16 each).
  - **The arena.** Every posed mesh's corners (copied from its vertex buffer), its joints and weights, and the moves of each of its shapes that has been active are taken up into three shared buffers, the first time each is needed. So the whole frame is posed in **two dispatches**, however many meshes it has. When something doesn't fit, the arena is filled afresh with the frame's own, half again as big, up to one storage binding's limit. Past that limit the items left over are drawn at rest, with a warning. I first wrote it one dispatch pair per mesh. That cost 2.7 ms of GPU and 6 ms of CPU submit for 50 baked characters, which are ~800 distinct meshes, and was a regression against the base branch, so I replaced it.
  - **Last frame.** While motion vectors are drawn, last frame's posed buffer is copied aside before this frame's posing (`Posing::previous`). `History` keeps each item's range by ID and mesh. `previous_corner` reads it: `skin.y` is 1 for an item new this frame, and otherwise 2 more than its old first corner.
- **Every raster pass reads the posed buffer** (`shader.wgsl`): `pose_corner` returns the posed corner (frame binding 9, which was the joints buffer) or the rest corner. `vs_main`, `vs_outline`, `vs_shadow` and `vs_motion` are otherwise unchanged. `morph_corner`, the per-pass skinning and the motion pass's per-pass previous pose are gone. Group 2 of the scene pipelines is now an empty group, kept so every pipeline's group numbers stay where they were. `skin.wgsl` is not touched.
- **Ray tracing builds each posed item's BLAS from the posed buffer** (`ray_sway.rs` `BentMeshes::pose`/`build_entry`: stride 20 bytes, first vertex at the item's range). So ray-traced shadows, AO, reflections and hardware GI see identity shapes and expressions. `ray_skin.wgsl` and its compute pass are deleted. Morph-only meshes are now traced posed too; before, they were traced at rest.
- **What `skinning.rs` keeps per mesh**: its joints and weights, and its shapes shape by shape (`ShapeRange`: each a range of moves naming their corners), on the CPU, for the arena. `JointRaw::of` writes joints only; weights no longer ride in the joints buffer. Animated meshes' vertex buffers gain `COPY_SRC`.
- **No fallback path.** Compute shaders, storage buffers in vertex shaders and storage atomics are core WebGPU on every backend the engine opens (DX12, Vulkan, Metal). Storage buffers in vertex shaders were already required for skinning. So no CPU or per-pass path is kept.
- **`RenderStats::posing`** (`PosingStats`: items, corners, active shapes, moves, bytes; the bytes are also in `buffer_bytes`). The bench prints it.
- **Capture tool:** `AXCENITY_CAPTURE_RT=shadows` (or `all`) turns hardware ray-traced shadows on, at High (`all` adds RT AO and reflections). VIEWS, LIGHTS, MORE_VIEWS and `Settings::default` are unchanged.
- **Bench:** `bench_distinct_characters` takes `AXCENITY_BENCH_COUNTS` (default `1,10,20,50`) and `AXCENITY_BENCH_MODES` (`live,baked,stylized`; `stylized` is N default stylized characters, the 41-shape body).

## Interfaces (new or changed)

```rust
// render (public)
pub struct PosingStats { pub items: u32, pub corners: u64, pub shapes: u32, pub moves: u64, pub bytes: u64 }
RenderStats::posing: PosingStats                    // new field (Default)
FramePass::CharacterPosing                          // new, first in FramePass::ALL; COUNT 12 → 13
// render (crate-private)
InstanceRaw::skin = [first posed corner, 1] | [0, 0]   // was [first joint, joints | weights << 16]
MotionInstance::skin = [now, 0 | 1 | previous + 2]
frame binding 9 = posed corners (array<u32>, 5 a corner)  // was the joints
motion group 3 binding 1 = last frame's posed corners     // was last frame's joints
Skinning::upload(&self, &dyn MeshSource) -> Option<GpuSkin>   // keeps data on the CPU now
```

No change to `avatar`, the description format, the Lua API, `scene`'s draw list or `Skin`. `render` still knows nothing of Instances.

## Tests

Written first and seen failing: [co8-compute-posing-red-run.txt](co8-compute-posing-red-run.txt) (5 of 11 failed against stubs; the ray-traced shadow test failed as specified: the wall stayed lit, `[82, 82, 82]` both ways).

| Test | What it proves |
|---|---|
| `render` `ray_tracing::sun_shadows::a_morphed_character_casts_the_ray_traced_shadow_of_its_shapes` (1) | a jointed mesh whose blend shape alone moves it between the wall and the sun, ray-traced shadows on and no shadow maps: weight 1 shades the wall (> 40 darker than weight 0), and the item is a posed BLAS. Failed on the base branch. |
| `render` `skinning::the_posed_buffer_holds_each_corner_where_every_pass_used_to_pose_it` (2) | a 6-corner strip on two blended joints (turned, scaled), three shapes, one with normal deltas and one without, weights 0.5 / 0 / −0.75: every posed corner equals the old per-pass math (`morph_corner` then `pose_corner`, rebuilt in the test): positions within 1e-4, normals and tangents within 2e-3 |
| `render` `skinning::a_shape_at_weight_zero_is_not_posed_at_all` (3) | `active_shapes` lists only non-zero, finite weights with running starts; on the GPU: 1 item, 6 corners, **2 active shapes, 9 moves** (the weight-0 shape left out), and the arena holds only those 9 moves; at all-zero weights the joints still pose it with 0 shapes and 0 moves |
| `render` `skinning::items_of_many_meshes_are_posed_together_as_the_arena_fills_afresh` | 3 meshes, then 42 (the arena fills afresh), then the 3 again: every item of every frame matches the per-pass math |
| `render` `skinning::a_shape_moves_a_jointed_mesh_before_its_joints`, `temporal::motion_vectors_hold_…` (now also a shape-only move) | drawing, and motion vectors from last frame's kept corners |
| `render` `skinning::items_posed_differently_share_one_mesh_and_batch`, `a_skins_joints_go_to_the_gpu_without_its_shape_weights` | changed for the new contract (two pairs, one batch; no weights in the joints) |
| (4) `player` `every_example_renders`, `character_capture::the_same_description_captures_identical_pictures_and_every_view_is_drawn`, `character_capture::an_explicit_bake_draws_the_same_pixels_as_the_live_character`, `skinning_tests::a_posed_character_looks_the_same_skinned_on_the_gpu_as_on_the_cpu`; `render` `skin::every_other_material_draws_as_it_did_before_the_skin_model` | all pass unchanged |

Summary lines (debug, this worktree, RX 9070 XT):

- After the last code change (the arena's shape index and the dedupe cleanup): `cargo nextest run -p axcenity-render --no-fail-fast`: `Summary [ 258.580s] 389 tests run: 389 passed (3 slow), 3 skipped`
- Final run, on the same code: `cargo nextest run -p axcenity-scene -p axcenity-player --no-fail-fast`: `Summary [ 108.950s] 388 tests run: 388 passed (5 slow), 23 skipped` (the gates in (4) among them)
- An earlier full run, on the per-shape arena: `cargo nextest run -p axcenity-scene -p axcenity-player -p axcenity-render --no-fail-fast`: `Summary [ 362.203s] 791 tests run: 791 passed (7 slow), 26 skipped`
- `cargo nextest run -p axcenity-render --features debug-views -E 'test(/debug_views|path_trac|ray_hits|skinning/)'`: `Summary [ 31.262s] 22 tests run: 22 passed, 383 skipped` (run on the first version; debug views share the scene shader)
- `cargo nextest run -p axcenity-editor -E 'test(/profiler|legend|a_character_file_applied|a_slider_changes_the_realistic|character_workspace/)'`: `Summary [ 39.194s] 83 tests run: 83 passed (2 slow), 1280 skipped` (final code)
- `cargo clippy -p axcenity-render -p axcenity-player --all-targets -- -D warnings` and `-p axcenity-render --features debug-views`: clean. `cargo check -p axcenity-editor -p axcenity-editor-benchmark --tests`: clean. `cargo fmt --all`: applied.
- `python3 -I scripts/check-wiki.py`: 0 errors.

## Measurements (release, RX 9070 XT, Ryzen 9 9950X3D, each run pinned to CPUs 0–15; other agents were building)

`cargo test -p axcenity-player --release -- --ignored bench_distinct_characters --nocapture --test-threads 1`. N realistic characters, each with a distinct randomized description worn live (or baked, or N stylized defaults), idling, 1280 × 720 at High (MSAA, so no last-frame copy).

- **Before** is the base branch's test binary, built before any render change. **After** is this branch.
- Five rounds, alternating which binary ran first.
- Each cell is the **lowest** of the five runs' fastest-batch medians, with the median of the five in parentheses. GPU times are in ms.
- The GPU clock state varied between runs (the bimodal before-columns), so compare the lowest values, or the medians, not one against the other.
- These are the per-shape arena build. It is GPU-identical to the final code; the final code only changed the CPU bookkeeping, checked below.

| N | mode | GPU frame before → after | posing (after) | shadows before → after | depth prepass before → after | scene before → after | CPU draw (submit) before → after | mesh MB before → after | posing: items, corners, active shapes, moves, MB |
|---|---|---|---|---|---|---|---|---|---|
| 1 | live | 0.56 (0.89) → **0.23 (0.33)** | 0.02 (0.02) | 0.24 (0.39) → 0.04 (0.04) | 0.09 (0.13) → 0.01 (0.02) | 0.15 (0.24) → 0.08 (0.12) | 0.56 (0.58) → 0.57 (0.63) | 8.2 → 0.6 | 17, 9298, 351, 102721, 7.51 |
| 1 | baked | 0.28 (0.46) → **0.22 (0.32)** | 0.01 (0.01) | 0.08 (0.13) → 0.04 (0.04) | 0.03 (0.05) → 0.01 (0.02) | 0.10 (0.15) → 0.08 (0.12) | 0.51 (0.58) → 0.56 (0.62) | 2.7 → 0.6 | 17, 9298, 8, 1807, 3.50 |
| 1 | stylized | 0.30 (0.48) → **0.41 (0.41)** | 0.01 (0.01) | 0.06 (0.10) → 0.04 (0.04) | 0.03 (0.04) → 0.02 (0.02) | 0.14 (0.21) → 0.20 (0.20) | 0.54 (0.56) → 0.55 (0.59) | 1.9 → 1.0 | 19, 14694, 4, 1061, 4.63 |
| 10 | live | 2.21 (2.57) → **0.76 (0.77)** | 0.09 (0.09) | 1.10 (1.28) → 0.09 (0.09) | 0.37 (0.43) → 0.04 (0.04) | 0.67 (0.77) → 0.41 (0.42) | 0.97 (1.00) → 0.95 (1.05) | 8.5 → 0.7 | 156, 90572, 3114, 930672, 14.07 |
| 10 | baked | 1.03 (1.66) → **0.47 (0.74)** | 0.02 (0.03) | 0.38 (0.63) → 0.07 (0.10) | 0.14 (0.23) → 0.03 (0.05) | 0.42 (0.67) → 0.28 (0.44) | 1.26 (1.34) → 1.08 (1.22) | 25.7 → 5.9 | 156, 90572, 52, 15662, 17.01 |
| 10 | stylized | 0.65 (0.73) → **0.56 (0.97)** | 0.02 (0.03) | 0.13 (0.15) → 0.08 (0.12) | 0.06 (0.07) → 0.03 (0.05) | 0.38 (0.43) → 0.36 (0.63) | 0.74 (0.78) → 0.79 (0.86) | 1.9 → 1.0 | 190, 146940, 40, 10610, 9.02 |
| 20 | live | 4.72 (5.11) → **1.22 (1.22)** | 0.16 (0.17) | 2.12 (2.37) → 0.15 (0.15) | 0.67 (0.81) → 0.07 (0.07) | 1.42 (2.11) → 0.70 (0.70) | 1.51 (1.59) → 1.54 (1.73) | 8.5 → 0.7 | 318, 182176, 6404, 1911307, 22.14 |
| 20 | baked | 3.03 (3.06) → **1.25 (1.25)** | 0.04 (0.05) | 1.19 (1.19) → 0.17 (0.18) | 0.42 (0.43) → 0.07 (0.07) | 1.28 (1.30) → 0.82 (0.82) | 2.33 (2.40) → 2.05 (2.17) | 51.5 → 11.8 | 318, 182176, 116, 32356, 32.02 |
| 20 | stylized | 0.97 (1.79) → **0.83 (1.48)** | 0.03 (0.04) | 0.21 (0.39) → 0.12 (0.21) | 0.09 (0.17) → 0.04 (0.08) | 0.59 (1.10) → 0.55 (1.01) | 0.89 (0.90) → 1.00 (1.01) | 1.9 → 1.0 | 380, 293880, 80, 21220, 14.03 |
| 50 | live | 9.57 (14.56) → **1.48 (2.39)** | 0.24 (0.39) | 4.89 (7.77) → 0.21 (0.33) | 1.67 (2.69) → 0.09 (0.14) | 2.95 (4.11) → 0.86 (1.40) | 2.96 (3.04) → 3.56 (3.76) | 11.9 → 0.9 | 796, 444252, 16058, 4625399, 45.31 |
| 50 | baked | 6.75 (6.82) → **1.56 (2.43)** | 0.14 (0.14) | 2.78 (2.78) → 0.24 (0.37) | 0.98 (1.00) → 0.09 (0.15) | 2.82 (2.87) → 1.00 (1.63) | 5.13 (5.27) → 4.66 (5.01) | 123.8 → 28.8 | 796, 444252, 292, 79278, 80.07 |
| 50 | stylized | 3.34 (3.37) → **2.69 (2.72)** | 0.08 (0.08) | 0.80 (0.81) → 0.45 (0.46) | 0.33 (0.34) → 0.15 (0.16) | 2.07 (2.09) → 1.86 (1.88) | 1.29 (1.39) → 1.57 (1.59) | 2.9 → 1.5 | 950, 695496, 200, 50266, 29.07 |

**The final code** (two more rounds at 20 and 50, interleaved; GPU clocks were lower in this window throughout):

| N | mode | GPU frame before → after | CPU draw (submit) before → after |
|---|---|---|---|
| 20 | live | 4.60 → 1.30 | 1.53 → 1.47 |
| 20 | stylized | 1.79 → 1.48 | 0.92 → 0.94 |
| 50 | live | 14.02 → 2.40 | 2.91 → 3.23 |
| 50 | baked | 6.71 → 2.43 | 5.10 → 5.25 |
| 50 | stylized | 3.38 → 2.73 | 1.33 → 1.52 |

**Reading.**

- **Live identity no longer costs in every pass.**
  - 50 live characters: 9.6 → 1.5 ms of GPU (lowest), 14.6 → 2.4 (median).
  - 20 live characters: 4.7 → 1.2 ms.
  - Shadows at 50 go from 4.9 to 0.21 ms and the depth prepass from 1.7 to 0.09 ms. The scene pass is faster too (2.95 → 0.86 ms), because it also morphed per vertex.
  - The posing pass itself takes 0.24 ms for 50 characters (16,058 active shapes, 4.6 M moves, 444 k corners).
- **Baked and stylized characters are faster too**, because skinning is no longer repeated per pass: 50 baked 6.75 → 1.56 ms, 50 stylized 3.34 → 2.69 ms.
- **One stylized character** reads 0.30 (0.48) → 0.41 (0.41). The before runs are bimodal and the after median is lower, so I don't read a regression into it, but I can't rule out a small one: the after scene pass was 0.20 in every run.
- **CPU submit** (`ScenePipeline::draw`) is within noise at 1–20 characters. At 50 it is +0.3 ms (live) and +0.2 ms (stylized) in the final interleaved check. That is the per-item planning (`pose_frame`, the active-shape lists, the job list), about 0.3 µs a posed item. For baked crowds it is within noise.
- **Mesh memory** falls (8.2 → 0.6 MB for one live character) because the joints, weights and shapes now live on the CPU and in the arena. Those GPU buffers show in the posing column instead.

**CO8's "Done when": "frame time with 20 characters with 150 shapes at least as fast as today with 41".**

- **Met** against the realistic body on the base branch: 20 live characters went 4.72 → 1.22 ms.
  - The base branch's per-pass shader reads every move of every shape whatever its weight (`morph_corner` walks the corner's whole delta list). So its cost for the realistic body with its identity at zero weights is the cost measured here for live weights.
  - It is also met against the realistic body baked on the base branch, which is what characters were before R2a: 3.03 → 1.25 ms.
- **Not met** against the stylized body's 41 shapes on the base branch: 20 stylized took 0.97 ms (lowest) against 1.22 ms for 20 live realistic after.
  - The gap is not the shape count. Shadows plus prepass are 0.22 ms after against 0.30 before.
  - It is the realistic content's scene pass (0.70 against 0.59 ms: the Skin shading model and its maps, R5's) plus the posing pass (0.16 ms).
  - The 6,404 active shapes cost 0.16 ms in total, once.
  - I used the realistic body as "today" for the headline because it is the same mesh and material, so only the shape cost differs. The lead decides whether the stylized reading is what the plan meant.

**Memory for the posed buffers:**

- **Per character, in the posed buffer:** 20 bytes a posed corner. The bench's characters average 8,885 posed corners (their LOD mix at this camera), so that is **174 KB each**.
- **Last frame's copy, while TAA or motion blur is on:** a further 20 bytes a corner (**174 KB**).
- **Shape sums:** 24 bytes a corner of a part with an active shape (up to **213 KB**).
- **Rounding:** the buffers round up to powers of two, so the measured totals are larger: 22 MB of posing buffers for 20 live, 45 MB for 50.
- **The arena** is shared by every character that wears the same meshes. For one realistic character it is 7.5 MB, with 1.5× room: 102,721 moves × 32 B, plus 48 B a corner and 32 B a corner's joints and weights. It grows only when a new mesh, or a shape no one used before, appears.
- **Baked characters** each bring their own meshes: 80 MB for 50, against 124 MB of derived meshes they already take.
- **On the CPU**, each animated mesh keeps its joints, weights and moves: about 6.4 MB for the realistic body's full-detail mesh.

## Visual evidence

Captures with **ray-traced shadows on** (`AXCENITY_CAPTURE_RT=shadows`: the High preset plus `ray_traced_shadows: High`, on this PC's RX 9070 XT, which has ray queries; every preset has them off) under the `outdoor` and `hard` lights. Sheets and numbers: [co8-compute-posing/README.md](co8-compute-posing/README.md). Each crop shows before | after | shadow maps.

- **Slim** (`realistic-feminine-cast-slim`): the clearest case.
  - Before, the ray-traced copy was the unmorphed base body. It is larger than the slim body, so the slim body sat inside it and was shadowed by its own outline. In the hard side view the back, the arm, the backs of the legs and a foot were black, and outdoors the whole right side and the scalp were dark.
  - After, the self-shadowing matches the shadow maps. Over the changed pixels, the mean distance to the shadow-map picture fell from 37–70 to 0.7–4.8 (of 255).
  - 1.8–2.6% of each picture changed.
- **Heavyset** (`realistic-masculine-cast-heavyset`): little changed (0.03–0.25% of pixels). The heavier body is mostly outside the base copy.
  - In the hard three-quarter view, dark specks on the briefs (the base copy's surface poking through) are gone.
  - From the front I see no difference on the body, only in the floor shadow's outline.
- **Happy face** (`realistic-masculine-cast-distinctive`, `AXCENITY_CAPTURE_FACE=Happy:1`): small changes (0.2% of pixels outdoors).
  - The jaw's shadow on the neck now follows this character's jaw and smile rather than the base face's.
  - Under the hard light's face views nothing changed, because the light falls on the face side the views see.
  - So "ray-traced shadows show expressions" is proven by the GPU test (1) and shows in these pictures as the jaw-line shadow only. The face suite has no light that casts an expression's shadow across the face.
- **Determinism:** the after pictures are identical between two separate builds (the per-mesh and the arena version), as the fixed-point sums intend.

## Dependencies added

None.

## Generated files

None regenerated: no manifest, model, catalogue, preset or benchmark changed.

## Open issues

- **The stylized reading of the Done-when** (above): 20 live realistic characters take 1.22 ms against 0.97 ms for 20 stylized on the base branch. The gap is the realistic material and the posing pass, not the shape count.
- **The arena's ceiling.** All posed meshes' data must fit one storage binding: 128 MB by default, and every device the engine opens has at least that. 50 distinct baked characters take 80 MB with the 1.5× room, so about 80 baked would pass the limit, and items past it are drawn at rest with a warning. Live characters share meshes and don't come near it. Splitting the arena into several bindings, or posing in several batches, would lift the limit.
- **CPU submit** grows by about 0.3 µs a posed item, which is 0.2–0.3 ms at 50 characters.
- **GI traced in software** (`ray_soft.rs`, GPUs without ray queries) still leaves skinned items out. That is unchanged.
- **A wiki staleness note:** `check-wiki.py` reports the usual 58 "may be stale" warnings on pages this change doesn't touch.

## For the lead at merge

- **Other tracks may conflict here:**
  - `crates/render/src/shader.wgsl`: the vertex inputs (lines 60–130) and the motion section.
  - `renderer/scene.rs` (`draw`).
  - `renderer/temporal.rs`, `renderer/lighting.rs` (`join_plan`) and `renderer/skinning.rs`.
  - `skin.wgsl` is untouched; the makeup track works there.
- **A branch that adds a `FramePass` or reads `InstanceRaw::skin`**: `CharacterPosing` is now first. `stats_time_each_pass_the_frame_draws` lists it among the passes a plain frame skips.
- **`ray_skin.wgsl` is deleted.**
- **Update** the plan's 11.5 R2a row (CO8 done) and the progress entry. ADR-0041's status line now says CO8 is done.
- **Not done here (ownership):** `docs/development/character-studio/progress.md`, the plan.

## Commands

- Captures (the full set is outside the repository):
  `AXCENITY_CAPTURE_DESCRIPTIONS=<cast-heavyset;cast-slim> AXCENITY_CAPTURE_DIR=<dir> AXCENITY_CAPTURE_LIGHTS=outdoor,hard AXCENITY_CAPTURE_VIEWS=front,three-quarter,side AXCENITY_CAPTURE_POSE=idle AXCENITY_CAPTURE_RT=shadows cargo test -p axcenity-player --release -- --ignored capture_characters --nocapture`
- The face, the same with:
  `AXCENITY_CAPTURE_VIEWS=face-front,face-three-quarter,face-side AXCENITY_CAPTURE_FACE=Happy:1`
- The shadow-map reference: the same without `AXCENITY_CAPTURE_RT`.
- Bench:
  `AXCENITY_BENCH_COUNTS=1,10,20,50 AXCENITY_BENCH_MODES=live,baked,stylized cargo test -p axcenity-player --release -- --ignored bench_distinct_characters --nocapture --test-threads 1`
