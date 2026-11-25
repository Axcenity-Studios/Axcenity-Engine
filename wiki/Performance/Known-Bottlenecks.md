---
title: Known bottlenecks
type: performance
status: current
verified: 2026-11-11
verified_commit: 19c7179d
source: [benchmarks/baselines.json]
docs: [docs/benchmarks.md, docs/rendering/performance.md, docs/research/engine-modernization-report.md]
tags: [perf, open-issues]
---

# Known bottlenecks

Open performance issues, each with its evidence class. When one is fixed, move it to the [Optimization log](Optimization-Log.md) and leave a line here saying where it went.

| Issue | Evidence | Where | Likely lever | Roadmap |
|---|---|---|---|---|
| Farmhouse steady frame ~0.6 ms above `300bbb5`'s 4.77 ms (now 5.40) | Measured, unexplained | Player | Profile with `profile_project` | — |
| Physics is still the largest simulation cost (58% of S10k instructions before #84; ~2.2 ms of a 2.8 ms test-runner frame after) | Measured | rapier's own step | Solver settings tried and rejected (#99 notes); `simd8` untested | — |
| Navmesh look re-reads turning anchored parts on the game thread (2.9–3.9 ms worst) | Measured | `MeshBuilder::look` | Move the read off the frame or narrow it | after M13 |
| `fire_changes` was 11% of S10k | Measured (Callgrind) | `script/src/changes.rs` | Partly fixed by #84 (skip Instances with no handler: Heartbeat −52%) | — |
| Copying retained draw items (~1.2 ms per 10,000 parts) | Measured | `scene/src/retained.rs` | Spatial `ItemChunk`s for still parts | M4 next step |
| `bench_10000_parts` `build_draw_list` rose 3.3 → 8–9 ms before `556b72b` | Measured, unexplained | `scene` | Bisect between the render benchmarks commit and `556b72b` | — |
| Shadow views copy 176-byte instances, read 88 | Measured (byte counts) | `render` | Slim shadow instance stream | L5 |
| GPU fill at 1440p Ultra in dense alpha foliage (forest ~14 ms GPU; Farmhouse ~40 ms frame) | Measured | Shadows, lighting, overdraw | LOD on heavy project models, shadow LOD (rejected once: changed the trees' look) | L5 |
| ~~A cooked game ships each texture twice (the `roads` sample 393 → 830 MB)~~ Fixed by G12: the `roads` sample packs to 337 MB instead of 711 MB | Measured | `project/src/packed.rs` | Ship cooked only, read the pack in place | G12 |
| Collaborate's folder check (`Mirror::sync`) costs ~1 ms every 0.5 s on S10k, the bulk of an idle session's ~50 µs mean | Measured | `collab/src/mirror.rs` | Move the look to a worker or follow the editor's file watcher, if bigger projects need it | — |
| A Workspace tree change in a streaming world splits the whole partition again (~12 ms at 51,000 parts); a teleport's tick sends a second's Instances at once (~14 ms of `replicate::create`) | Measured (Container, streaming world) | `core/src/streaming.rs`, `net/src/replicate.rs` | Split only the moved branches from the moves log; cheaper `create` (borrowed property names) | Phase 80 follow-up |
| Instance property reads cost ~4,000 instructions, mostly mlua | Measured (Callgrind) | `script/src/instance.rs` + mlua | mlua `__index` specialization upstream, or native `vector` | L3 |
| Vertex-heavy scenes pay ~5% for the skin field on every instance | Measured | `render` | Separate pipelines for skinned items | — |
| Editor repaints everything each Play frame | Measured in the design-system audit (A9) | `editor` | — (V7 and V8, visible-rows-only Explorer and Console, landed in #73) | Design system |
| A first sight stalls one frame 30–62 ms in the player (five of Desert Homestead's tour cuts). ~~Coming back after 10 s stalled too (75–105 ms)~~: fixed by [ADR-0051](../Architecture/Architecture-Decisions/ADR-0051-A-Renderer-That-Shows-A-World-Keeps-Undrawn-Meshes-And-Textures-Within-Memory-Budgets.md), 12–23 ms now | Measured | `render`: what is seen for the first time is uploaded on the frame's thread outside a streaming world | Preload behind the loading screen as the editor does; or background uploads in the player (rejected for now: white textures for a few frames) | — |
| ~~The memory budgets were fixed per preset whatever the card has (Desert Homestead's 1,579 MiB of textures didn't fit High's 1,024 MiB on a 16 GB card)~~ Fixed 2026-11-25 ([ADR-0051](../Architecture/Architecture-Decisions/ADR-0051-A-Renderer-That-Shows-A-World-Keeps-Undrawn-Meshes-And-Textures-Within-Memory-Budgets.md)): a quarter of the card's memory where that is more. Left: NVIDIA and Intel cards on Linux don't say their memory and keep the presets' numbers; the Windows read wasn't run | Measured (Linux, amdgpu) | `render/src/video_memory.rs` | A reader for those cards (NVML, or the Vulkan memory budget extension through wgpu-hal) | — |
| The clouds' march is 2.6 ms (High) to 5.1 ms (Ultra) of the scene pass at 1440p with half the screen sky | Measured | `clouds.wgsl` `clouded_sky`: full size, noise worked out per step | March at half size and upscale, or noise from a texture | — |
| The player's shown frame under Wayland is the CPU's frame plus the GPU's (20.6 ms where they are 6.7 and 13.3): the next frame starts when the compositor says the last was shown, and `--bench`'s FPS line doesn't count that wait | Measured (whole runs timed) | `player/src/window.rs` (redraw asked in `about_to_wait`), winit's frame callbacks | Start the next frame's CPU work while the GPU draws the last | — |
| A struct with an array passed by value in WGSL and indexed by a loop counter costs a copy of the array per look (fixed for `Fog::dust`, 2026-11-25) | Measured | any shader | Read the uniform itself; none other found in a scan of the shaders that day | — |
| Per-Part heap ~2 KB | **Unverified** estimate | `core` | L1 | parked |

## Abandoned Farmhouse startup regression

> [!success] Closed 2026-11-11
> PRs #65, #67 and #69 brought the first frame from 4.35 s to **0.21 s** and assets-in from 6.0 s to 1.6 s (baselines at `15965da`, PR #70). Kept here for history; see the [Optimization log](Optimization-Log.md#terrain-layers-off-the-main-thread-2026-11-11).

- **Symptom** (Measured, Desktop, player): first frame ~1.05 s at `300bbb5` → ~4.4 s at `39dd258`; launch to assets in 4.7 → 6.0 s. Carried as a `known` note in `benchmarks/baselines.json`.
- **Cause**: M5 moved texture work from startup onto the first frame; the project's 29 `.glb` files embed every 4K map, which L2 couldn't cook until PR #65.
- **Status after PR #65** (Measured, pinned, medians of 3): cooked first frame **1,869 ms**, launch to assets in **3,330 ms**, GPU textures 947 → 311 MB.
- **Last step (#67, #69)**: ~1.2 s of the first frame had been the terrain decoding its 30 cooked 2K layer maps on the main thread; they're now decoded on the loading workers and the layer arrays built in parallel. Baselines re-recorded at `15965da` (#70).

## Related

[Performance overview](Performance-Overview.md) · [Technical debt](../Roadmap/Technical-Debt.md) · [Current roadmap](../Roadmap/Current-Roadmap.md)
