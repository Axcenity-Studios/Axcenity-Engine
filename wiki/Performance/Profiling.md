---
title: Profiling
type: performance
status: current
verified: 2026-11-25
verified_commit: a9370a3b
source: [crates/editor/src/app/viewport_bar.rs, crates/editor/src/profiler.rs, crates/render/src/renderer/gpu_timer.rs, crates/render/src/renderer.rs, crates/core/src/bench.rs, crates/player/src/dev_console.rs]
docs: [docs/benchmarks.md, docs/rendering/performance.md, docs/architecture/play-mode.md, docs/developer-console.md]
tags: [perf, tools]
---

# Profiling

Where to look when something is slow.

| Tool | Where | Shows | Cost when off |
|---|---|---|---|
| **Editor Profiler** (Game tab) | View > Panels > Profiler (`profiler.rs`) | Frame rate, CPU (game + drawing), viewport GPU time, process memory, both Lua heaps, physics (CPU, steps, bodies), audio, scripts, animation, network; 120-frame averages; a CPU-time bar graph against 60 fps | Readings are taken anyway; the panel only reads them |
| **Editor Profiler** (Scripts tab; scripting P2 PR 4) | same, then **Record** | Each event handler, script body and task by its own time (share, per call, longest run), and each script function by sampled own and with-calls time and its busiest line; click a row to open it | Off until Record: sampling costs a little script time |
| **Editor Profiler** (Graphics tab) | same | `RenderStats`: draw calls, batches, triangles, culled items and casters, LOD distribution, GPU memory by kind, per-pass GPU time, surface-shader compile errors | — |
| **`RenderStats` / per-pass GPU time** | `TextureRenderer::stats`; `gpu_timer.rs` | Timestamps around each `FramePass` (Shadows, Depth prepass, AO, Volumetric fog, GPU particles, Scene, Post, UI); read a frame or two later | Needs `TIMESTAMP_QUERY`; the player never asks for it |
| **Tracy** | Build a host with `--features profile-tracy` | `profiling` spans: every frame step, `build_draw_list`, `UiInteraction::layout`, `ScriptRuntime::step`/`stepped`/`heartbeat`, `Physics::step`/`sync`/`substep`/`write_back`, `Replicator::frame` | Zero (spans compile to nothing; hakari leaves `profiling` out so Tracy is never on by accident) |
| **Viewport bar's FPS** (2026-11-25) | Editor, top left of the Scene View (`app/viewport_bar.rs`) | Frames a second as the viewport shows them (the time from one frame to the next while they come one after another); the tooltip has the CPU and GPU work in one and the whole editor frame's CPU time. Until 2026-11-25 it was `1000 / max(CPU, GPU)`, an estimate that read 41 where 24 were shown | — |
| **Editor frame log** | `AXCENITY_FRAME_LOG=1` (`docs/benchmarks.md`) | A line every two seconds on standard error: frames a second as shown, viewport CPU, Play, the whole editor frame, GPU; frames keep coming whether or not anything changed | Off unless the variable is set |
| **The drawing's sections** (`CpuSection`, 2026-11-25) | `--bench` ("Drawing on the frame's thread, by section") and the editor's frame log (`drawing, ms:`), from `RenderStats::cpu_times` (`renderer/cpu_timer.rs`) | The frame's thread in fifteen sections of `ScenePipeline::draw`: levels of detail, materials, culling, motion history, the ray traced scene, wgpu's encoding, submit and the rest, with the items drawn and traced | A dozen clock reads a frame |
| **Input to finished picture** | `AXCENITY_BENCH_LATENCY=on` with `--bench` (`renderer/latency.rs`) | Each frame's time from its start to the GPU finishing it: the engine's part of input latency | A thread looking at the GPU; off unless asked |
| **`--bench`'s `as shown`** | Player (`core::bench::BenchRun::time_between_frames`) | The frame rate start to start, which counts the waits between frames; the bound is worked out against it | — |
| **The card's own sensors** (Linux, amdgpu) | `/sys/class/drm/card*/device/gpu_busy_percent`, `hwmon*/freq1_input` (core clock), `power1_average`, `mem_info_vram_used`; `/proc/<pid>/fdinfo/*` `drm-engine-gfx` for one process's GPU time | Whether the GPU is busy, and at what clock: a card idle half of each frame runs at about half its clock, so its pass times read nearly twice what they are under load ([Hardware notes](Hardware-Notes.md)) | Read by hand; no tool in the repo |
| **Player F9 console, Stats tab** | `dev_console.rs` | The frame's `Laps` (same step names as `--bench`) and the memory breakdown (G7) | Asked for only while the tab is open |
| **Profiler › Memory** (G7, PRs #125, #126; history, peaks and budgets #241) | Editor | Process, the counted total over the last minute as a chart, its peak, game memory by kind (Lua, Instances, Models, Textures, Sky and probes, Sounds, Physics) with each kind's change and peak since Play started, the project's memory budget (Settings > This project; one Output warning per Play per limit gone over) and the 12 heaviest Instance classes | — |
| **Debug views** | Editor Viewport > View Mode (`debug-views` feature of `render`, editor only) | Wireframe, normals, overdraw and others | Not built into the player |
| **Culling overlay** | Editor Viewport > Overlays > Culling | Freezes culling at a camera and shows drawn (green) vs culled (red) spheres | Editor only |
| **Frame-rate and bound tools** (rendering Phase 1, PR #83) | `profile_project`, the Profiler and F9 | Frame rate with 1% lows, whether a frame is CPU- or GPU-bound, GPU passes including per-cascade shadow timing | — |
| **Overdraw as numbers** (`OverdrawStats`) and the **LOD Inspector** tab | Editor | How many times pixels are shaded, whose overdraw it is; which LOD each item draws | Editor only |
| **Callgrind** | valgrind on the container | Instruction counts per function; stable across runs | — |

## Reading the numbers

- **Frames a second**: the Profiler's Frame rate (the time between game frames in Play) and the viewport bar's FPS are both measured from frame to frame and agree. Neither is `1000 / CPU` or `1000 / GPU`: a frame also waits (for the display, or for the GPU when it's the slower side).
- **`(held)` and `(idle)` on the viewport bar**: with no frame rate limit (the default since 2026-11-27) the bar reads the real rate, unmarked. `(held)` means a limit someone chose in Settings > Accessibility > Frame rate limit sets the rate while they work; `(idle)` means frames come on the 30 a second scenery timer while nobody does. The tooltip says what the frame's work would allow.
- **Editor against player**: only at the same camera and viewport size (`AXCENITY_START_CAMERA`; the frame log prints the size). The player's `--bench` flies the project's own camera.
- **CPU-bound vs GPU-bound**: compare the Profiler's CPU frame with the GPU frame. Owner's heavy scenes have been CPU-bound in `build_draw_list` + `ScenePipeline::draw` with the GPU under 3 ms (General Store Roadside, before threaded planning), and GPU-bound in fill and shadows (forest at Ultra) after it.
- **GPU pass times only compare when the GPU is the limit** (found 2026-11-25). The card's clock follows its load, so in a view the frame's thread limits, taking work off one pass slows the clock and the other passes read longer: Desert Homestead's still view at 1996 × 998 gave 12.8 ms of GPU frame with ray traced shadows and 12.0 with them skipped whole. Time a GPU change where `--bench` says `GPU-bound` (that view in a 2560 × 1405 window), and a CPU change where it says `CPU-bound`.
- **`drawn at`** (`--bench`, 2026-11-25): the size the window really is. GNOME gives a window that nearly fills the screen the screen's free area whatever was asked for; runs reported as 3840 × 2160 before that day were 2560 × 1405.
- **`each frame (median)`** (`--bench`): draw calls, items, batches, shadow casters, the ray traced scene's instances and the command buffers a frame was submitted as. Check them before believing a frame time moved.
- **Low overall CPU% is expected**: the frame runs on one thread ([Threading model](../Architecture/Threading-Model.md)).
- **First frame vs steady frame**: first-frame cost is loading (decode, mips, upload); see [Asset pipeline overview](../Asset-Pipeline/Pipeline-Overview.md).
- **Debug builds lie**: `scene` and most engine crates are unoptimized in dev.

## Related

[Benchmarking](Benchmarking.md) · [Debugging](../Development/Debugging.md)
