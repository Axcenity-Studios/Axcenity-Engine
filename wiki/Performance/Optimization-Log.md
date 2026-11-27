---
title: Optimization log
type: performance
status: current
verified: 2026-11-11
verified_commit: 19c7179d
source: [docs/benchmarks.md, docs/rendering/performance.md, benchmarks/baselines.json]
docs: [docs/benchmarks.md, docs/rendering/performance.md, docs/research/engine-modernization-report.md]
tags: [perf, history]
---

# Optimization log

Every significant optimization: the problem, its root cause, the fix, and what it measured. Newest roadmap work first, then the graphics-plan work it built on. Raw tables live in `docs/benchmarks.md` and `docs/rendering/performance.md`; this log keeps the reasoning in one place. Add an entry from `Templates/Optimization-Entry.md`.

**Evidence**: **Measured** = timed, with the method given. **Expected** = reasoned from a measurement. **Theoretical** = from first principles. **Unverified** = a guess to check. Machines: **Desktop** = Ryzen 9 9950X3D, RX 9070 XT, Windows 11, pinned to the V-cache CCD; **Container** = Linux cloud container, 4 vCPU Xeon 2.1 GHz, lavapipe ([Hardware notes](Hardware-Notes.md)). Compare numbers only within one machine.

> [!summary] Net effect (Measured, Desktop, `300bbb5` → `5cb8302`, pinned, alternating)
> S10k frame: test runner 7.71 → 5.03 ms, server 5.13 → 3.82 ms, player 14.93 → 8.73 ms. Temperate Wet Patch test runner 28.97 → 0.048 ms; General Store Roadside player 6.99 → 4.40 ms. The one regression (Abandoned Farmhouse startup) was closed on 2026-11-11 by #65, #67, #69: first frame 4.35 s → 0.21 s. Then physics part 1 and the two thread pools (#84, #86, #88, #93) cut the XB10k cross-engine frame from 6.70 to 5.14 ms.

## Summary table

| Item | PR | Area | Headline (Measured) | Machine |
|---|---|---|---|---|
| The sky's clouds traced before the scene pass; the prepass and the scene pass timed by what is drawn | 2026-11-26 | Render | Desert Homestead, Ultra, 1920 × 1080: clouds 1.3–2.6 ms → 0.04–0.30 (the `Clouds` pass), the same picture; hero view 139.9 → 164.2 frames a second, windmill 149.8 → 230.4, the tour 151.9 → 181.4; at 2560 × 1405 the tour 112.4 → 147.2 (Unity HDRP: 150.9) | Desktop (Fedora) |
| A frame encoded on several threads; one query a ray for ray traced shadows and ambient occlusion; the ray traced scene bound only where rays are traced; the display's refresh asked for again | 2026-11-25 | Render, player | Desert Homestead with everything on at 1996 × 998: the still view's frame's thread 16.45 → 13.6 ms (encoding 4.00 → 1.57, submit 2.25 → 1.72), 57.0 → 68.3 frames a second; at 2560 × 1405 where the GPU is the limit, GPU frame 17.2 → 14.5 ms (ray traced shadows 6.57 → 4.66, ambient occlusion 3.39 → 2.54); the debug editor 44.4 → 52.0; a 1000 × 600 window as played 43.8 → 97.9 | Desktop under Fedora |
| The editor's frames held to 60 only when someone chooses a limit | 2026-11-27 | Editor | The focused editor no longer held at "60 FPS (held)" by a default nobody chose; the counter reads the real rate. Not timed in this change (a hand check on the Desktop) | Desktop under Fedora |
| Plants by the cluster, a terrain's key shared by its chunks, a lighter motion history; a hidden game's loop | 2026-11-25 | Render, player, editor | Desert Homestead's still view (17,000 plants): the frame's thread 18.5 → 16.5 ms, 51.0 → 57.0 frames a second; the tour as played 67.0 → 70.8; the editor 41.0 → 44.4 (debug). Input to finished picture measured: 38.7 → 27.6 ms with pacing. The frame's thread no longer spins while it waits (96% → 35% of a core) | Desktop under Fedora |
| Frames started while the GPU draws the last (frame pacing) | 2026-11-25 | Render, player, editor | Desert Homestead with the editor's ray traced settings at 1996 × 998: as shown 24.5 → 67.0 frames a second in the player (the card 59% → 89% busy, its GPU frame 24.2 → 13.4 ms), 26.7 → 41.0 in the debug editor and → 46.7 in a release one; 1% low 22 → 52 with the memory read moved off `smaps` | Desktop under Fedora |
| Swaying plants bent together for ray tracing | 2026-11-25 | Render (ray tracing) | Desert Homestead with the editor's ray traced settings at 1996 × 998: the frame's thread 33.8 → 14.3 ms, GPU frame 33.3 → 23.8 (acceleration structures 12.8 → 2.5); as shown 13 → 24 frames a second in the player | Desktop under Fedora |
| Undrawn meshes and textures kept within memory budgets | 2026-11-25 (ADR-0051) | Render, memory | Desert Homestead's tour: the frame on a cut back to a seen view 105 → 14 ms and 75 → 21 ms; first sights unchanged (30–62 ms) | Desktop under Fedora |
| Fog boxes read one at a time; the atmosphere's sky on the renderer's threads | 2026-11-25 | Render (shaders, sky) | Desert Homestead at 2560 × 1440: GPU frame 110 → 12.4 ms on High, 139 → 16.9 on Ultra (about 9 → 48 frames a second shown on High); Render step in its dusk time-lapse 29–37 → 10–13 ms | Desktop under Fedora |
| Character Studio's tiles previewed off the interface's thread | 2026-11-25 | Editor, Character Studio | A tile's first preview on the Clothing page 24–147 ms (median 44) → none for a tile crossed or scrolled past; a rested tile's first preview 79 → 47 ms with a renderer (hairstyle 133 → 22 ms), debug build | Desktop under Fedora |
| Editor open without a frozen first frame | #290 | Editor startup | Abandoned Farmhouse first open (debug, lavapipe): first editor frame after the load 8 s → 1.5 s; the rest goes to the GPU behind a counted loading screen | Container |
| Collaborate packing and the world load | C11 | Collaborate, project | S10k join message 953 → 82 KiB; `project::load` of S10k 5.66 → 0.19 s | Container |
| Streaming world: only what changed | Phase 80 (unified plan) | Network, ownership | Server with 1 player: Network send 16.7 → 2.0 ms, frame 21.9 → 4.2 ms | Container |
| DirectX 12 shader compiler and parallel pipelines | #172, #182 | Startup | Wet Patch assets loaded 11.4 → 2.25 s on DirectX 12 (Vulkan 0.98 s) | Desktop |
| Render planning pool | #88 | Render | Unpinned Render step 6–18% less CPU (S10k 2.60 → 2.14 ms) | Desktop |
| Physics own pool | #86, #93 | Physics | Unpinned S10k Physics 2.71 → 2.19 ms, frame 3.37 → 2.81 | Desktop |
| Physics step part 1 | #84 | Physics, events | S10k Physics 6.12 → 4.73 ms, Heartbeat 1.10 → 0.54 (−24% frame) | Container |
| Explorer and Console draw only visible rows | #73 | Editor | See `docs/research/usability/verification.md` | Container |
| Terrain layers on workers, built in parallel | #67, #69 | Startup | Farmhouse first frame 4,354 → 212 ms (with #65); General Store 1,230 → 446 ms | Desktop |
| L2 embedded cook | #65 | Assets | Farmhouse cooked first frame 2,926 → 1,869 ms; GPU textures 947 → 311 MB | Desktop |
| M9 | #61 | Packing | `world.json` ~50× smaller in `.axgame`; cooked games 10–21% smaller | Container |
| L4 query bounds | #59 | Physics queries | S10k Characters 2.05 → 0.31 ms | Container |
| L1 measured | #59 | Core | Name lookups ~9% of S10k instructions: **parked** | Container |
| L3 part 1 | #58 | Scripting | Numeric Luau 3.1× (interpreted), 6.2× (`--!native`) | Container |
| L2 (8 parts) | #45–#57 | Assets | 4096² upload 55–61 → 0.9–1.1 ms; 85 → 21 MiB; cooked model 143 → 1.8 ms | Container |
| M14 | #37 | UI | 500-element inventory layout 27.3 → 3.6 ms per frame | Container |
| M7 | #34 | Network | 8-player network send 45.2 → 5.06 ms; S10k snapshot 14.4 → 1.02 MB | Container |
| M3 | #25 | Physics | S10k Physics 11.32 → 8.87 ms | Container |
| M1 | #15, #16, #20, #24 | Frame | Editor Play S10k 5.99 → 5.09 ms | Desktop |
| M13 | #32 | Navigation | Agents worst 25–36 → 4.0–5.6 ms | Container |
| M6 (4 parts) | #22, #26, #27, #29 | Scripting | Empty Lua call 112 → 25 ns; 10k idle tasks 95,457 → 35 ns/step | Container |
| M4 | #23 | Scene | S10k Draw list 12.9 → 4.3 ms | Container |
| Mesh hulls | #21 | Physics queries | Biome patch Input 30.9 → 0.006 ms | Desktop |
| M5 | #18, #19 | Assets | Arid Dry Patch launch to assets 5,056 → 3,204 ms | Desktop |
| M2 | #17 | Core | S10k test runner frame 6.68 → 5.91 ms | Desktop |
| I11 | #8 | Player startup | the `roads` sample first frame 2,069–5,431 → 341–398 ms | Container |
| I10 | #9 | Watcher | Reload latency 463 → 103 ms | Container |
| I9 | #11 | Core | S10k Input −15%, Scripts −6 to −10% | Container |
| I8 | #10 | Scene | 2,000 MeshParts draw list 1.84–1.90 → 0.90–0.99 ms | Container |
| I7 | #12 | Navigation | Agents median 4.2–4.6 → 0.36–0.38 ms; worst 120–137 → 26–29 ms | Container |
| I4 | #7 | Scripting | `with_world` 3.3 → 0.15 µs | Container |
| Threaded planning | 3f2fc05 | Render | Forest 1440p Ultra CPU draw 39.6 → ~9.7 ms, GPU 31 → 13.7 ms | Desktop |
| GPU skinning | graphics Ph. 14 | Render | 20 walking characters CPU draw 4.85 → 0.38 ms | Desktop |
| Culling, LOD | graphics Ph. 12–13 | Render | Edge views draw only what's seen; foliage cull distances | Desktop |

## Entries

### The sky's clouds traced before the scene pass, and GPU time by what is drawn (2026-11-26)

| | |
|---|---|
| **Roadmap item / PR** | Steps 1 and 2 of the plan from the comparison with Unity HDRP (`benchmarks/cross-engine/homestead/results.md`), as Paul revised it on 2026-11-25: clouds first, with pictures of their edges, motion, lighting and shadows before and after; GPU timers by what is drawn before any work on the scene pass |
| **Evidence** | Measured |
| **Problem** | The clouds cost 1.6 to 2.9 ms of a 6.6 to 7.2 ms GPU frame where sky shows, against 0.3 ms for HDRP's volumetric clouds. The scene pass was timed whole, so what its fixed 2 ms was could only be guessed by taking things away |
| **Root cause** | `clouded_sky` ran in every pixel of sky: 64 steps and 6 toward the sun from each, and at every one of them `cloud_fill`'s seven noise look-ups (28 hashes), though the shapes depend only on the place |
| **Solution** | `renderer/cloud_trace.rs`, `cloud_trace.wgsl`: `cloud_sum` kept in a 4096² texture round the camera (drawn again when the camera is 1,200 studs from its middle); the clouds worked out at half size only where the prepass's depth shows sky; then each pixel read between the half-size texels where they agree and worked out as before where they don't (a cloud's edge, the horizon); the sky pass reads its pixel's clouds and lays its own sky behind. Nothing is kept between frames. `gpu_timer.rs`: `DrawnPart`, a stamp between draws where the part changes (`TIMESTAMP_QUERY_INSIDE_PASSES`), solid items sorted by distance band and then by part |
| **Affected components** | `crates/render/src/renderer/cloud_trace.rs`, `clouds.wgsl`, `cloud_fill.wgsl`, `cloud_trace.wgsl`, `shader.wgsl` (`fs_sky`), `renderer/gpu_timer.rs`, `scene.rs`, `lighting.rs` (`SortKey::part`), `renderer.rs`; `crates/core/src/bench.rs`, `crates/ui/src/console.rs`, `crates/player/src/main.rs` |
| **Before → after** | 1920 × 1080, three runs, pinned: hero 139.9 → 164.2 frames a second, mailbox 151.7 → 162.9, porch 203.7 → 202.1, truck 163.1 → 197.4, windmill 149.8 → 230.4, crane 150.5 → 172.6, the tour 151.9 → 181.4. 2560 × 1405: 101.8 → 128.3, 114.8 → 125.2, 165.2 → 164.0, 123.0 → 167.3, 106.9 → 211.9, 114.2 → 137.1, 112.4 → 147.2. Scene pass in the hero view 4.66 → 3.32 ms with a `Clouds` pass of 0.30 |
| **Method** | `benchmarks/cross-engine/homestead/run.py` (release player `--bench`, 1,800 frames, pinned to the V-cache cores), the old path on the same build with `AXCENITY_NO_CLOUD_TRACE=1`. The picture: stills of five views with both paths (mean difference 0.01 to 0.14 of 255 levels), and in the renderer's tests both against a picture drawn at four times the size |
| **Hardware** | Desktop under Fedora: see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | 52 MB of textures at 1920 × 1080 (the shapes' 32 MB and two targets); the process's video memory read 221 to 262 MB higher, not explained further. Drawing the shapes' texture again costs 0.47 ms on the frame that does. The new draw order and the stamps left the frame rate where it was (within 1%); the frame's thread read 0.07 to 0.15 ms higher |
| **Found, not changed** | Reading between half-size texels everywhere made a busy horizon up to 10 levels too bright, and a 2048² shapes' texture changed thin veils: hence the per-pixel step and 4096². The lit surfaces' cloud shadows still work the noise out in every pixel (about 0.25 ms in the hero view; the item shader has no texture binding free). By what is drawn, hero view: the GPU's frame is 3.75 ms whatever the window's size (shadows 1.07, prepass 0.51, scattered items in the scene pass 0.82, other items 0.47, fog 0.38) plus 1.09 ms a million pixels (terrain 0.35, items 0.19, the dust's sprites 0.16). The cut-out surfaces' depth is drawn in the prepass and again in the scene pass |
| **Corrected** | The comparison's note that the dust emitters are a small cost: 0.34 to 0.42 ms in the two views that show them, which the Unity port doesn't draw |
| **Detail** | `docs/rendering/clouds.md`, "The traced clouds"; `docs/rendering/performance.md` (per-pass GPU time); `benchmarks/cross-engine/homestead/results.md`, "After step 1" and "After step 2" |

### A frame encoded on several threads, and one query a ray (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | Paul's second review of ADR-0052, 2026-11-25: keep the unfocused window's rule, ray tracing's cost on the GPU (sampling, BVH traversal, denoising), encoding on several threads and fewer draw calls, a background rendering policy to consider, the measurements as regression benchmarks |
| **Evidence** | Measured |
| **Problem** | `finish` took 4.0 ms of the frame's thread; ray traced shadows and ambient occlusion were 10 of the GPU's 17 ms at 2560 × 1405; submit was 2.4 ms with ray tracing on; a heavy game in a small window was shown at 44 frames a second where it draws 98 |
| **Root cause** | wgpu 30 encodes a pass's commands when its encoder is finished, on the calling thread, and the frame was one encoder. Every shadow and ambient occlusion ray was two queries, solid items and then cut-outs, since each had its own nearest distance. At submit wgpu walks every instance of a TLAS for each pass that bound it, and six passes bound it where three trace. The player asked the display's refresh once, when its window wasn't on a display yet under Wayland, and took 60 Hz: frames 17 to 25 ms long never counted as missing a 240 Hz display's refresh |
| **Solution** | `renderer/encoders.rs` (`FrameEncoders`): the frame in parts, finished on the renderer's threads together and submitted in order. One query a ray, a cut-out's candidates confirmed in the shader past its nearest distance; a soft shadow's ray that a cut-out stopped asks once more for a solid item, so its edge is as wide as before. The sun's rays and the fog's air in one pass with one binding of the scene; the smoothing passes with a layout without it. The refresh asked for again every 30 shown frames |
| **Affected components** | `crates/render/src/renderer/encoders.rs`, `scene.rs`, `ray_shadows.rs`, `ray_tracing.rs`, `parallel.rs`, `ray_shadows.wgsl`, `ray_ao.wgsl`; `crates/player/src/window.rs`, `main.rs`; `crates/core/src/bench.rs`; `crates/testing/src/bench_check.rs`; `benchmarks/baselines.json`, `benchmarks/features/owner-*.json` |
| **Before → after** | Still view at 1996 × 998, everything on: encoding 4.00 → 1.57 ms, submit 2.25 → 1.72, the frame's thread 16.45 → 13.6, 57.0 → 68.3 frames a second. At 2560 × 1405 (GPU-bound, pinned): ray traced shadows 6.57 → 4.66 ms, ambient occlusion 3.39 → 2.54, GPU frame 17.21 → 14.53, 57.5 → 68.0. Tour: Standard Ultra 119.2 → 132.6, everything on 71.8 → 77.2; as played 70.8 → 77.0. Debug editor 44.4 → 52.0. As played at 1000 × 600: 43.8 → 97.9 |
| **Method** | Release player `--bench` on the tour and the still view, one change at a time with a way back (`AXCENITY_FRAME_ENCODERS=one`; the old shaders built again for the A/B); each encoder's `finish` timed; `eu-stack` samples inside `finish` and `RayScene::prepare`; steps of the ray traced passes left out one at a time at a size where the GPU is the limit; `shoot_project` pictures from three cameras against the second round's |
| **Hardware** | Desktop under Fedora: see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | The parts cost 5.5 ms of thread time between them where one encoder cost 4.0 (1.5 ms of the frame's thread). A soft shadow's ray that a cut-out blocks is still two queries |
| **Found, not changed** | Pinned to the CPU's V-cache cores the still view is 90.9 frames a second against 66.3 as launched (the system prefers the other eight cores). 350 of 923 scene batches are terrain chunks (3% of the triangles): merging them would save 0.2 to 0.3 ms now. Bundling every plant took submit from 2.19 to 0.90 ms and left tracing within 3%, but costs 7.2 ms of GPU if built each frame: still plants need bundles built once. The denoising passes are 0.5 ms; the rays are 5.4 |
| **Corrected** | Runs reported as 3840 × 2160 were 2560 × 1405 (GNOME); GPU pass times compared in a view the frame's thread limits |
| **Detail** | `docs/rendering/performance.md`, "Encoding on several threads, and one query a ray (the audit's third round, Desert Homestead)" |

### The editor's frame rate held only by a limit someone chose (2026-11-27)

| | |
|---|---|
| **Roadmap item / PR** | Paul, 2026-11-27, asked several times before: "this FPS counter is not my actual FPS, it is a held value again; I want a real FPS value unless I specify it to be held", and it changed when he clicked away |
| **Evidence** | Unverified on hardware here (the cloud has no display); the cause is read from the code and his settings' history, the fix is covered by tests |
| **Problem** | Focused: `60 FPS (held)`. In the background: 120 |
| **Root cause** | Steady frames (a second after any input, or always with Hold the refresh rate steady) were held to `Appearance::steady_hz`, 60 by default since 2026-11-08, and every saved settings file carried the 60. In the background there is no input, so no steady frames and no hold. The two earlier fixes changed the counter, not the hold: 2026-11-22 worked it out from `max(cpu, gpu)` (read 41 where 24 were shown), 2026-11-26 measured again and added `(held)` |
| **Solution** | Settings > Accessibility > **Frame rate limit**, No limit by default, saved as `frame_limit_hz` so the old saved 60 is dropped (`held_apart`); a frame the last asked for at once is never `(idle)`, focused or not |
| **Affected components** | `crates/editor-kit/src/theme.rs` (`Appearance`), `crates/editor/src/app.rs` (`pace_frame`, `held_apart`), `app/viewport_bar.rs`, `app/settings/appearance.rs`, `app/settings/search.rs` |
| **Before → after** | Focused editor: held to 60 → paced like the player (by the display while frames fit a refresh, at once when they don't, ADR-0052) |
| **Method** | Tests `frames_have_no_limit_unless_one_is_chosen`, `frames_are_held_only_to_a_frame_rate_limit_someone_chose`, `with_no_limit_the_bar_reads_the_real_rate_unmarked` |
| **Hardware** | — |
| **Trade-offs** | A FreeSync or G-Sync OLED or VA panel that flickered without the grid needs the limit chosen again. A limit chosen before this change is dropped once too (it can't be told from the old default) |

### Plants by the cluster, and the audit's second round (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | Paul's review of ADR-0052, 2026-11-25: the editor's extra 6 ms, foliage clustering, a persistent ray scene, the shadow maps, Windows and macOS with hidden windows, input latency; and "I still do not fully trust the frame counters" |
| **Evidence** | Measured |
| **Problem** | The frame's thread was 18.5 ms in a view of 17,000 plants; nothing said where in the frame it went; the editor seemed 6 ms slower than the player; a hidden window's game stopped or drew for nobody; the viewport read 60 FPS with ray tracing off without saying it was held |
| **Root cause** | Every pass worked out again each frame, plant by plant, what never changes about a plant (its material binding, mesh, place, what rays find on it); a terrain's 1,400 chunks each hashed all its layers twice a frame; the motion history built a whole scene instance and hashed an ID for every swaying plant. The editor's 6 ms was its view (17,190 items against the tour's 11,000). The player's event loop turned over (`Poll`) while it waited; under Wayland nothing asks a hidden window for frames. The editor holds frames to its steady rate (60 Hz) and the counter didn't say |
| **Solution** | `renderer/clusters.rs`: what a group of items is, kept by the address of the group's shared items, read by the material pass and the ray scene; `terrain::LastArrays`; a motion history that reads only what the pass uses. `renderer/cpu_timer.rs` (`CpuSection`) in `--bench` and the editor's frame log; `renderer/latency.rs`. The player's loop sleeps, steps a hidden game undrawn 30 times a second (`window::next_frame`), and an unfocused window waits for the display. The viewport bar's `(held)` and `(idle)` |
| **Affected components** | `crates/render/src/renderer/clusters.rs`, `lod.rs`, `lighting.rs`, `ray_tracing.rs`, `temporal.rs`, `terrain.rs`, `cpu_timer.rs`, `latency.rs`, `scene.rs`, `pacing.rs`; `crates/player/src/window.rs`, `main.rs`; `crates/editor/src/app.rs`, `app/viewport_bar.rs`, `viewport.rs`; `crates/core/src/bench.rs`; `crates/ui/src/console.rs` |
| **Before → after** | The still view at 1996 × 998: frame's thread 18.52 → 16.45 ms (Materials 1.37 → 1.05, Uploads 1.00 → 0.67, Motion history 2.12 → 1.11, Ray traced scene 4.58 → 4.04), 51.0 → 57.0 frames a second. The tour as played 67.0 → 70.8. Editor at its start camera 41.0 → 44.4 (debug), 46.7 → 50.2 (release). Latency, input to finished picture: 38.7 → 27.6 ms (a 2560 × 1405 window 47.1 → 35.6; 42.1 without the limit of one frame ahead). The frame's thread while waiting for the display: 96% → 35% of a core. A minimized light game: never finished → steps on at 0.3% of a core |
| **Method** | Release player `--bench` on the tour and on a copy of the project with the tour stopped (the still view), with the new section timings and temporary timers inside `RayScene::prepare`; the editor's frame log at the same camera (`AXCENITY_START_CAMERA`); `shoot_project` pictures from three cameras before and after (identical, or within what two runs of one build differ by); `AXCENITY_BENCH_LATENCY`, `AXCENITY_FRAME_CALLBACKS`, `AXCENITY_FRAMES_AHEAD`, `AXCENITY_BENCH_MINIMIZE_AFTER`. One run each, not pinned |
| **Hardware** | Desktop under Fedora: see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | A cluster holds its group's items until it's unused for 600 frames. A draw is 8 bytes bigger. A window seen without focus draws a heavy scene at the waiting pace. Windows and macOS not run (a checklist in `docs/benchmarks.md`) |
| **Found, not changed** | The shadow maps are read with ray traced shadows on (leaves lit from behind, fog, see-through surfaces, the far band): not redundant. A wgpu TLAS build costs for every instance each frame while anything sways: fewer instances would pay, cached tables wouldn't |
| **Detail** | `docs/rendering/performance.md`, "Plants by the cluster, and the audit's second round (Desert Homestead)", with the standing against Paul's frame rate targets |

### The CPU and the GPU took turns (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | Paul's report of 2026-11-25: Desert Homestead "still significantly underutilizing my hardware", 41 FPS in the viewport against 13 in the Profiler; [ADR-0052](../Architecture/Architecture-Decisions/ADR-0052-A-Frame-Starts-While-The-GPU-Draws-The-Last-When-Frames-Miss-The-Display.md) (Proposed) |
| **Evidence** | Measured |
| **Problem** | A frame was shown every 40.8 ms where the CPU's work was 15 and the GPU's 24. The card was 59% busy at 1,511 MHz and 70 W of 304. The viewport bar said 41 FPS, `--bench` said 68 |
| **Root cause** | Under Wayland, winit's `pre_present_notify` holds the next redraw until the compositor says the last frame was shown, which it can't before the GPU has finished it: each frame's CPU work waited for the last frame's GPU work, and the idle card clocked down, stretching its own time. Both frame rate readings were worked out from the work (`1000 / max(CPU, GPU)`, and the steps' sum), not measured. Separately, the player read `/proc/self/smaps` (24 ms) on the frame's thread every 30 frames |
| **Solution** | `FramePacer` (`render/src/pacing.rs`): wait for the display while frames fit in a refresh, start early when they keep missing it; shown renderers one frame ahead (`set_one_frame_ahead`); the editor through a switch in a vendored egui-wgpu. The viewport bar and `--bench` report the rate as shown (`BenchRun::time_between_frames`); the editor's frame log (`AXCENITY_FRAME_LOG`). `memory-stats` with `always_use_statm` |
| **Affected components** | `crates/render/src/pacing.rs`, `renderer/scene.rs`, `renderer.rs`; `crates/player/src/window.rs`, `main.rs`; `crates/editor/src/app.rs`, `app/viewport_bar.rs`, `viewport.rs`; `crates/core/src/bench.rs`; `vendor/egui-wgpu`; `Cargo.toml` |
| **Before → after** | Player as shown 24.5 → 67.0 frames a second (1% low 16 → 44 to 54); GPU frame 24.2 → 13.4 ms; card 59% → 89% busy, 1,511 → 3,136 MHz, 70 → 280 W. A 2560 × 1405 window (asked for 3840 × 2160) 20.7 → 53.0. Debug editor 26.7 → 41.0, release editor 26.7 → 46.7 |
| **Method** | Release player `--bench` (600 to 1,500 frames) at `--size 1996x998` with the editor's saved graphics settings, with and without `AXCENITY_VSYNC=on`; the editor with `AXCENITY_FRAME_LOG=1` on a copy of the project, with and without `AXCENITY_FRAME_CALLBACKS=on`; the card's sysfs sensors and the process's `drm-engine-gfx` time sampled twice a second; `eu-stack` samples of the frame's thread; the scene with one class of content or one setting removed at a time. One run each, not pinned |
| **Hardware** | Desktop under Fedora (Ryzen 9 9950X3D, RX 9070 XT, Mesa 26.2 radv, GNOME 51 on Wayland, a 240 Hz display): see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | Started early with the GPU the slower side, input is two GPU frames from its picture (worked out, not timed). The card draws near its limit in a heavy scene. `vendor/egui-wgpu` is copied again with each egui upgrade. Not run on Windows or macOS; a hidden window under an older compositor not tried |
| **Detail** | `docs/rendering/performance.md`, "The CPU and the GPU took turns (frame pacing, Desert Homestead)", which also has where the frame goes now and the targets left |

### Swaying plants bent together (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | Paul's report of 2026-11-25: "in the editor I am still running at 24 FPS" on Desert Homestead, every ray traced effect on |
| **Evidence** | Measured |
| **Problem** | With hardware ray tracing on and the wind blowing, the frame's thread spent 33.8 ms in the Render step and the GPU 12.8 ms of its 33.3 building acceleration structures, every frame |
| **Root cause** | Each swaying plant within the 400,000-triangle budget had a BLAS, a positions buffer, a uniform and a compute dispatch of its own, all redone each frame (wgpu has no refit). The budget counted triangles; the cost is by the BLAS, the dispatch and the buffer written: about 3 µs of GPU time and 5 µs of the thread a plant, and this view bent 4,118 clumps of about 100 triangles |
| **Solution** | Every copy is bent into one positions buffer where it stands in the world, a mesh's copies by one dispatch, and built as the geometries of BLASes shared by kind and by square of the ground (`BundleKey`, `bundle_cell`); a hit's item is its instance's first slot plus the geometry hit, in every ray shader. A bundle's BLAS is made with room to spare and built with fewer or smaller geometries as the items change. Plants the wind moves less than a pixel on screen are traced at rest |
| **Affected components** | `crates/render/src/renderer/ray_sway.rs`, `ray_tracing.rs`, `path_tracer.rs`, `ray_sway.wgsl` and the ray shaders (`ray_shadows.wgsl`, `ray_cut_outs.wgsl`, `ray_gi_hardware.wgsl`, `ray_reflections.wgsl`, `water_rays.wgsl`, `ray_hits.wgsl`, `path_tracer.wgsl`), `draw_list.rs` (`Sway::reach_in`) |
| **Before → after** | Medians: Render step 33.8 → 14.3 ms; GPU frame 33.3 → 23.8 (acceleration structures 12.82 → 2.48, ray traced ambient occlusion 3.79 → 4.15, ray traced shadows 5.45 → 5.87). A frame as shown, whole runs timed: 73.7, 75.9 and 88.7 ms → 41.7 (13 → 24 frames a second) |
| **Method** | Release player `--bench 300` at `--size 1996x998` with the editor's saved graphics settings (`--graphics`), `a9370a3b` against the change, one run each; the cause from `eu-stack` samples of the running editor, a cap on the bent count, and a stand-alone wgpu probe of BLAS layouts. The picture compared with `shoot_project` from three cameras (mean difference 0.005 of 255). Not pinned; the editor itself not timed after |
| **Hardware** | Desktop under Fedora (Ryzen 9 9950X3D, RX 9070 XT, radv, a 240 Hz display): see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | Rays trace a little slower against bundles than against a BLAS a plant (0.8 ms here). A bundle's BLAS holds room for more than a frame's copies (up to twice the budget's triangles). Rays find an item by instance and geometry, so every shader that reads a slot adds the two. The Path Traced view turns normals by a matrix in its slot table (48 bytes a slot, written each frame the view is shown) |
| **Detail** | `docs/rendering/performance.md`, "Swaying plants bent together (ray tracing, Desert Homestead)" |

### Kept within a budget (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | [ADR-0051](../Architecture/Architecture-Decisions/ADR-0051-A-Renderer-That-Shows-A-World-Keeps-Undrawn-Meshes-And-Textures-Within-Memory-Budgets.md); Paul's choice among the options for the stall on a camera cut |
| **Evidence** | Measured |
| **Problem** | Every cut of Desert Homestead's tour stalled one frame, 30–105 ms |
| **Root cause** | What wasn't drawn for 600 frames and 10 seconds was freed and uploaded again in the frame that next drew it, however much memory was free; the tour's shots are 10 seconds long. First sights upload in their frame too |
| **Solution** | The player's window and the editor's viewport keep meshes and color textures while they fit the mesh and texture memory budgets; past one, the least recently drawn undrawn ones are freed first, before any drawn texture loses a level (`renderer/undrawn.rs`). Meshes get a budget of half the textures' |
| **Affected components** | `crates/render/src/renderer/undrawn.rs`, `scene.rs`, `crates/render-types/src/graphics.rs`, `crates/player/src/window.rs`, `crates/editor/src/viewport.rs` |
| **Before → after** | The cut frame, Ultra: back to the first view 105.3 → 14.3 ms, the tour's end 75.1 → 21.3; first sights 31.3 → 30.8, 61.9 → 61.8, 46.3 → 38.6, 47.3 → 30.0, 103.0 → 57.7. High: the same within 2 ms |
| **Method** | Player `--bench 5500` (the whole tour) at `--size 2560x1440`, `AXCENITY_BENCH_TIMELINE`, the worst frame around each cut; one run each |
| **Hardware** | Desktop under Fedora: see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | GPU memory in use grows toward the budgets and stays (Desert Homestead: 1,579 MiB of textures and 174 MiB of meshes after the tour on Ultra). Dead keys (a chunk's mesh from before an edit) stay until a budget is full. The budgets follow the card's memory (4,096 and 2,048 MiB on this card at every preset; the measurements here were taken with the presets' fixed numbers, and the tour at the default settings afterwards gave the same cuts: 12.7 and 22.7 ms on the two returns) |
| **Detail** | `docs/rendering/performance.md`, "Kept within a budget (ADR-0051)" |

### Fog boxes and the atmosphere's sky (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | Paul's report of 2026-11-25: Desert Homestead "loads at about 11 FPS" on an RX 9070 XT |
| **Evidence** | Measured |
| **Problem** | The scene drew at 9 to 12 frames a second at every quality preset. Its last tour shot, a dusk time-lapse, also held the CPU at 29–37 ms a frame |
| **Root cause** | (1) fog.wgsl's functions take the `Fog` as a value and indexed its array of eight dust boxes with the loop's counter (`fog.dust[i]`), which copies the array for each look: about 50 ms of GPU time a box, every pixel, and the scene has two (a valley's haze, the house's air). Reading the cause from the cost: the compiled shader wasn't looked at. (2) An Atmosphere's 256 × 128 sky image is worked out again whenever the sun moves a tenth of a degree, on the frame's thread: 16,000 looks through the air |
| **Solution** | (1) A box is read through `fog_dust(i)`, defined by lighting.wgsl and sprite.wgsl over their own uniforms. (2) `sky_pixels` shares the image's columns among the renderer's threads (`parallel::map_each`), in order, so the image is the same |
| **Affected components** | `crates/render/src/fog.wgsl`, `lighting.wgsl`, `sprite.wgsl`, `renderer/atmosphere_sky.rs` |
| **Before → after** | GPU frame, median: High 110.3 → 12.4 ms, Ultra 138.9 → 16.9 (scene pass 109.2 → 10.4 and 134.5 → 13.4; volumetric fog's pass 2.53 → 0.45). As shown on the screen, timed over whole runs: 20.6 ms a frame on High and 23.4 on Ultra after. Render step through the time-lapse, median of each 3 s: 29–37 → 10–13 ms |
| **Method** | Player `--bench` (200 to 5,500 frames) at `--size 2560x1440`, `467da9f2` against the fix, one run each; the cause found by taking one kind of Instance out of a copy's `world.json` at a time; the time-lapse by `AXCENITY_BENCH_TIMELINE` and `eu-stack` samples of the frame's thread. Not pinned |
| **Hardware** | Desktop under Fedora (Ryzen 9 9950X3D, RX 9070 XT, radv, a 240 Hz display): see [Hardware notes](Hardware-Notes.md) |
| **Trade-offs** | Each shader that includes fog.wgsl must define `fog_dust`. The sky image wakes the renderer's threads on a frame that makes one |
| **Detail** | `docs/rendering/performance.md`, "Fog boxes read one at a time (Desert Homestead)" |

### Character Studio's tile previews (2026-11-25)

| | |
|---|---|
| **Roadmap item / PR** | Paul's Character Studio review of 2026-11-25 ("noticeable lag when scrolling through or viewing clothing models") |
| **Evidence** | Measured (the interface's thread, in a test); not measured in the running editor |
| **Problem** | Scrolling the Clothing page, or moving the pointer across it, stalled the frame once per tile |
| **Root cause** | A tile under the pointer was previewed at once: the character wearing it was applied to the preview, which read the garment's model (1 to 15 MB of glTF with 15 to 81 blend shapes) with `Assets::model_blocking` on the interface's thread, then made the body's copy with its hidden skin and uploaded both. With the wheel turning, each tile that came under the pointer did so. Besides: each change to the character queued a picture for every tile of the page behind the ones already queued (nothing took a stale request back), and every finished picture was read from its PNG and decoded on the interface's thread, four a frame |
| **Solution** | The panel says which tile the pointer is over; the workspace asks the loader's thread for what that tile would show (`Assets::request_model`) and the preview starts only once the pointer has rested there 0.15 s, with the wheel still, and the models are in. A picture no longer wanted is dropped from the picture thread's queue, and a finished one comes back with its pixels |
| **Affected components** | `crates/editor/src/character_workspace/browser.rs`, `panel/browser.rs`, `shell.rs`, `thumbnails.rs` |
| **Before → after** | A tile's first preview, 49 garments: 24–147 ms each, median 44, 2.9 s in all (CPU only), or 48–212 ms, median 79, 4.8 s with a renderer drawing 830 by 780 → nothing for a tile not rested on; a rested tile 41–104 ms, median 47 (the read, 2.35 s for the 49, on the loader's thread). Ten hairstyles: median 133 → 22 ms. A second preview of the same tile: 12 ms and 4 ms, unchanged. A still frame of the page: 1.25 ms, unchanged. A tile's picture: 14 ms once its models are loaded (55 tiles: 0.8 s on the picture thread) |
| **Method** | `cargo test -p axcenity-editor -- --ignored browser_costs --nocapture` (`character_workspace/timing_tests.rs`): the studio driven through its intents on the shipped library, each tile hovered in turn, the frame's work timed on the interface's thread with a `TextureRenderer` drawing and reading back each frame. A **debug** build, as `cargo run` makes the editor (the `avatar`, `assets`, `anim`, `scene` and `render` crates are optimized in it); one run each |
| **Hardware** | Desktop under Fedora (Ryzen 9 9950X3D, radv): see [Hardware notes](Hardware-Notes.md); the Desktop's Windows numbers are not comparable |
| **Trade-offs** | A preview starts 0.15 s after the pointer stops, not at once. What is left at a first preview is on the interface's thread still: the body's copy with its hidden skin (one per outfit that hides something) and the uploads |
| **Detail** | `docs/editor/character-studio.md`, "A click wears, a hover previews" |

### Collaborate on S10k and the world load (2026-11-12)

| | |
|---|---|
| **Roadmap item / PR** | Collaborate C11 |
| **Evidence** | Measured |
| **Problem** | The brief asked for an idle session under 0.1 ms a frame and a join of S10k under 10 s. Measuring showed the member's join spent 5.9 s opening the session's files as a project, on the UI thread, and the join message was 953 KiB |
| **Root cause** | `parse_world` looked up each Instance's prefab segment (`prefab::segment_of`), which lists all its siblings: 10,000² comparisons for S10k's Workspace. Every open of a big project paid it, not only Collaborate |
| **Solution** | Find the segment only inside a prefab copy, where it's used. Pack wire messages of 16 KiB or more with zstd level 1 when that saves an eighth, bounded by each side's message limit |
| **Affected components** | `crates/project/src/world_file.rs`, `crates/collab/src/wire.rs` |
| **Before → after** | `project::load` of S10k 5.66 → 0.19 s release, ~30 → 0.8 s debug; a member's open of S10k 5.9 → 0.35 s; join message 953 → 82 KiB (1 ms to pack). Idle `team_frame` mean 48–57 µs host, 34–36 µs member |
| **Method** | Ignored tests `collab::perf_tests` and `editor::team_tests::idle_session_cost_on_s10k`, release, 300 frames 16 ms apart |
| **Hardware** | Container |
| **Trade-offs** | A new dependency (zstd, BSD-3); the wire format number went up. Snapshot streaming and more presence coalescing were measured as not needed at S10k's size |
| **Detail** | `docs/benchmarks.md`, "Collaborate on S10k (C11)"; `docs/research/collaborate.md` §9.6 |

### Streaming world: read only what changed (2026-11-12)

| | |
|---|---|
| **Roadmap item / PR** | Unified plan Phase 80 (the 4 × 4 km criterion), branch `phase80-streaming-bench` |
| **Evidence** | Measured |
| **Problem** | The streaming world (50,000 anchored parts, 1,000 crates, streaming on) cost the server 16.7 ms of Network send a frame with one joined player |
| **Root cause** | Callgrind: `owned_parts` read every part's `Anchored` and regrouped assemblies every frame; SipHash on Instance IDs in the replicator, partition and player sets; unit boxes measured whether or not they moved; the always-there parts list walked the Workspace each tick; `Abilities::step` walked the Workspace; any tree change anywhere split the partition and rebuilt replication with B-trees of 51,000 IDs; a leaving unit was found by linear search |
| **Solution** | Revision-keyed caches (ownership's parts, units' boxes), the Workspace moves log for ownership, Fx hash maps, the class index for abilities, partition keyed on the Workspace and the characters, units indexed by root |
| **Affected components** | `crates/script/src/ownership.rs`, `crates/core/src/streaming.rs`, `crates/net/src/{replicate,streaming,motion}.rs`, `crates/gameplay/src/ability.rs` |
| **Before → after** | Server medians: Network send 16.71 → 2.00–2.14 ms, Heartbeat 1.20 → 0.02, frame 21.86 → 4.20–4.47, worst 86 → 45 ms. Test runner frame 4.83 → 3.29 ms |
| **Method** | `axcenity-server <streaming> --bench 600 --bench-clients 1` and `axcenity-test <streaming> --bench 600`, release, two runs after |
| **Hardware** | Container |
| **Trade-offs** | Ownership keeps a map of every part's revision (about 1 MB at 50,000 parts). A change to the Workspace's tree still splits the whole partition |
| **Detail** | `docs/benchmarks.md`, "The streaming world (Phase 80)" |

### DirectX 12 startup (2026-11-11)

| | |
|---|---|
| **Roadmap item / PR** | M12 follow-ups, PR #172 (static DXC) and #182 (parallel scene pipelines) |
| **Evidence** | Measured |
| **Problem** | After #104 made DirectX 12 the default on Windows, projects took about 8.5 s longer to load than on Vulkan |
| **Root cause** | Without DXC, wgpu compiled every shader with the old FXC. With DXC, building pipelines still costs about 0.9 s at every launch, because DirectX 12 has no cache between launches (Vulkan's driver keeps one: 76 ms) |
| **Solution** | Link DXC in (wgpu `static-dxc`, needs Visual Studio's ATL); make the scene's ten pipelines on threads of their own (`gpu::in_parallel`) |
| **Affected components** | `Cargo.toml`, `crates/render/src/renderer/scene.rs`, `gpu.rs` |
| **Before → after** | Temperate Wet Patch launch to assets loaded 11.4 s (FXC) → 2.41 s (DXC) → 2.25 s (parallel); pipelines ready 1,123 → 886 ms. Farmhouse 10.1 → 2.60 → 2.36 s. Vulkan 0.98 s |
| **Method** | Release player, `--bench 300`, one warm-up then medians of 3 |
| **Hardware** | Desktop |
| **Trade-offs** | About 21.5 MB more per Windows executable; Windows builds need the ATL component. DirectX 12 also takes about 0.75 s longer than Vulkan to reach `using graphics adapter` (instance, surface and adapter setup; listing Vulkan and GL adapters too costs at most 50 ms). The `debug` lines in `Renderer::new` split it up |
| **Detail** | `docs/rendering/pipeline.md`, "Graphics API" |

### Physics step and thread pools (2026-11-11)

| | |
|---|---|
| **PRs** | #84 (part 1), #86 and #93 (own pool), #88 (render pool), #99 (contact recycle, behavior) |
| **Problem** | The engine comparison (XB10k, #80) found physics the largest cost of a frame; about 40% of `Physics::step` was Axcenity's own work around rapier |
| **Root cause** | Resetting forces on every body; touches in `BTreeSet`/`BTreeMap` with a part lookup per contact; property writes looked up by name and allocating; `Changed` lookups for ~4,400 writes per frame on both sides; rayon's global pool of 32 threads oversubscribing rapier and draw planning |
| **Solution** | Reset only pushed bodies; sorted merge with a per-collider owner table; `set_known_property`; motion-only re-reads; skip unwatched Instances in `changes::fire`; own pools of at most 12 threads for physics and rendering |
| **Before → after** | Container S10k test runner: Physics 6.12 → 4.73 ms, Heartbeat 1.10 → 0.54, frame 8.07 → 6.09. Desktop unpinned: Physics 2.71 → 2.19, frame 3.37 → 2.81; S10k Render step 2.60 → 2.14 |
| **Rejected** | Fewer solver iterations or no stabilization: up to −20% but stacks fell and chains stretched |
| **Side fix** | Torque movers' torques used to pile up every step |

### Terrain layers off the main thread (2026-11-11)

| | |
|---|---|
| **PRs** | #67, #69 (with #65), recorded in #70 |
| **Problem** | After #65, ~1.2 s of Farmhouse's first frame was the terrain decoding 30 cooked 2K layer maps on the main thread (`Image::rgba()`) |
| **Solution** | Ask for layer maps with their pixels on the loading workers; build layer arrays in parallel, resizing each texture once |
| **Before → after** | Desktop baselines `39dd258` → `15965da`: Farmhouse first frame 4,354 → 212 ms, assets in 6,043 → 1,579 ms; General Store first frame 1,230 → 446; Wet Patch 820 → 268 |

### L2 embedded cook

| | |
|---|---|
| **PR** | #65 (`924720a`, merged in `21aadbf`) |
| **Problem** | Abandoned Farmhouse's 29 `.glb` files embed all their 4K maps; the cook skipped images inside models, so they decoded and mipmapped on the first frame |
| **Root cause** | Cooking only covered texture *files*; embedded images needed an import to copy them out |
| **Solution** | Cook embedded images in place, one shared copy per distinct image per project (`embedded-<hash>.axtex`), shared `Image` in memory |
| **Before → after** | main uncooked → branch cooked: first frame 2,926 → 1,869 ms; launch to assets 7,224 → 3,330 ms; steady frame 5.449 → 5.413 ms; GPU textures 947 → 311 MB |
| **Method** | Player `--bench`, release, pinned, medians of 3; parity shots max diff 9/255 |
| **Trade-offs** | Per-model copies (first version) used 1,764 MB on disk and more GPU memory; fixed by sharing |
| **Left** | ~1.2 s of first-frame Render is the terrain decoding 30 cooked 2K layer maps on the main thread (`Image::rgba()`) |

### M9: compressed game files

| | |
|---|---|
| **PR** | #61 |
| **Problem** | `.axgame` v1 stored files raw; unpacking reset modified times so cooked copies were re-hashed |
| **Solution** | v2: zstd level 3 per entry when it saves ≥ 5%, restored mtimes, unpacking on up to 8 threads |
| **Before → after** | the `roads` sample 830 → 745 MB, unpack 210 → 166 ms; S10k 10 → 0.2 MB |
| **Trade-offs** | Packing slower (the `roads` sample 1.0 → 3.3 s). Read-in-place not done: unpacking is only 30–170 ms |

### L4 query bounds

| | |
|---|---|
| **PR** | #59 |
| **Problem** | L4 (parallel stages) was to be guided by data; S10k's Characters step was 2.05 ms with one character |
| **Root cause** | Callgrind: 93% in `Reach::may_touch`: every ray walked all 10,000 parts reading `Position` and `Size` by name |
| **Solution** | Keep each part's bounding ball between queries (`physics::query::WorkspaceBounds`), checked by revision. Fixing the scan beat threading it |
| **Before → after** | Test runner Characters 2.05 → 0.31 ms; frame 10.03 → 8.56 ms |
| **Method** | Container, alternating, three 600-frame runs |

### L1 measured, parked

| | |
|---|---|
| **PR** | #59 (second commit; recorded only in the PR body, see [Documentation history](../Changelog/Documentation-History.md#open-documentation-gaps)) |
| **Question** | Do string-keyed properties still dominate after M2 and M4? |
| **Result** | Callgrind over whole S10k frames: physics 58%, navmesh refresh 19%, `fire_changes` 11%, name lookups ~9% of instructions |
| **Decision** | L1 (typed per-class slots, ~500 call sites) not worth it now |

### L3 part 1: fast builtins and opt-in native code

| | |
|---|---|
| **PR** | #58 |
| **Root cause** | Without `safeenv`, Luau looks up every `math.sin` and calls through the C API; native code bails out at each such call (`--!native` alone measured 0.99×) |
| **Solution** | Read-only per-script libraries + `safeenv`; `--!native` header compiles with `luau-jit` |
| **Before → after** | 3M `math.sin(i) * math.sqrt(i)`: 384–387 → 124–127 ms (interpreted) → 62–63 ms (native) |
| **Not changed** | Engine calls: Vector3 math and Instance reads stay mlua userdata calls; S10k frame unchanged |

### L2: cooked textures

| | |
|---|---|
| **PRs** | #45 (BC7), #47 (ship in builds), #49 (LOD cache), #50 (asset IDs), #51 (cooked models), #53 (orphans), #55 (cook on change), #57 (BC5 normals) |
| **Problem** | First frames spent seconds decoding JPEGs, making CPU mips and uploading RGBA8; models parsed FBX/glTF and simplified LODs on every load |
| **Solution** | Editor cooks offline: BC7 mips (`intel_tex_2`), BC5 normal maps, `.axlod` LOD cache, whole `.axmodel` models |
| **Before → after** | 4096² map: upload 55–61 → 0.9–1.1 ms, worker read ~650 → ~30 ms, GPU 85 → 21 MiB. Simplifying CityProps 68 → 6 ms. Cooked model CityProps 143 → 1.7–1.9 ms, Walking.fbx 178 → 19–24 ms |
| **Trade-offs** | Built games carry sources and cooked copies (the `roads` sample 393 → 830 MB). BC7 "very fast" PSNR 35.8 dB color; normal maps needed BC5. lavapipe shows no first-frame gain (software BC7 decode) |

### M14: UI layout cache

| | |
|---|---|
| **PR** | #37 |
| **Root cause** | Callgrind: the CSS cascade was 88% of a frame; each element matched every rule against itself and its ancestors twice a pass; editor Play laid UI out four times |
| **Solution** | `StyleCache` per frame, inherited custom properties shared, selectors matched once per rule, a reused `TaffyTree` per thread, `hit_layout` reused as the first pass |
| **Before → after** | One pass 12.8–13.5 → 1.88–1.90 ms; player frame 27.3–28.2 → 3.58–3.66 ms; editor Play 53.4–55.8 → 3.60–3.69 ms |

### M7: replication wire

| | |
|---|---|
| **PR** | #34 (protocol 10) |
| **Problem** | Nobody could join S10k: its 14.4 MB JSON snapshot overflowed the 5 MiB reliable channel; network send with 8 players was 45 ms |
| **Root cause** | 92% of send in `MotionSender::frame`, re-reading 10,000 parts by name per player per frame; the diff cloned every moving part's properties |
| **Solution** | postcard wire, creates carry only non-default properties, motion re-read only when `motion_revision` moved, diff skips motion-only changes |
| **Before → after** | Snapshot 14.4 → 1.02 MB, join 242 → 19 frames, send 45.2 → 5.06 ms, frame 57.4 → 14.9 ms, bytes/frame 10,324 → 5,464 |

### M3: physics sync

| | |
|---|---|
| **PR** | #25 |
| **Root cause** | Physics re-read what it wrote itself; any reparent forced a full resync; any script change woke every body; write-back each substep converted quaternions to Euler |
| **Solution** | Physics-owned revisions, per-service structure versions, wake neighbours, write back once per frame (awake bodies), rapier `parallel` |
| **Before → after** | Physics 11.32 → 8.87 ms (test runner), 9.91 → 8.15 ms (server); sync 1.6–1.7 → 0.7 ms; rapier's step −17% |
| **Trade-offs** | `parallel` costs tiny scenes (`vehicle` 0.05–0.07 → 0.19 ms) |

### M1: one frame schedule

| | |
|---|---|
| **PRs** | #15, #16, #20, #24 |
| **Problem** | Four hand-ordered loops had drifted (correctness, not speed) |
| **Measured side effect** | Editor Play S10k frame 5.99 → 5.09 ms, Physics 3.34 → 2.29 ms, worst 13.8–17.2 → 5.9–6.9 ms (seven pinned alternating rounds); an earlier container hint of a 5.9% slowdown was noise |

### M13: navmesh on a worker

| | |
|---|---|
| **PR** | #32 |
| **Solution** | `MeshBuilder::look` reads changes on the game thread; `navmesh build` thread assembles; one `SharedNavMesh` for agents and Humanoids |
| **Before → after** | Agents worst 25–36 → 4.0–5.6 ms, mean 0.89–1.07 → 0.33–0.38 ms; median unchanged |

### M6: script hot paths

| | |
|---|---|
| **PRs** | #22 (heap scheduler, prune on destroy), #26 (watchdog `Cell`, class method tables), #27 (chunk cache), #29 (handler coroutine reuse) |
| **Before → after** | 10k idle tasks 95,457 → 35 ns/step; step with 10k handlers 59,363 → 2,949 ns; empty Lua call 112 → 25 ns; `part.Position` read 512 → 397 ns; 200 script copies 2.4 → 0.2 ms each; handler call −18% instructions |
| **Rejected** | `mlua::LuaString` keys: slower with mlua 0.12 (5,869 vs 5,417 instructions) |

### M4: retained draw items

| | |
|---|---|
| **PR** | #23 |
| **Root cause** | Every frame re-derived every part's `DrawItem` from properties |
| **Solution** | `DrawCache` keyed by revision plus recorded dependencies; randomized equality tests against from-scratch |
| **Before → after** | `bench_build_10000_parts` 6.4–8.0 → 1.2–1.5 ms; S10k Draw list 12.9 → 4.3–4.4 ms |
| **Left** | Copying kept items (~1.2 ms per 10k); next: spatial `ItemChunk`s |

### Mesh hulls and the mouse ray

| | |
|---|---|
| **PR** | #21 |
| **Root cause** | `Clicks::step` cast the mouse ray every frame; each query built a new convex hull of every corner of nearby MeshParts (2–3 M corners in the biome patches' scanned trees) |
| **Solution** | Hulls kept by model and size (`shapes::hull`); no ray without a ClickDetector |
| **Before → after** | Arid Dry Patch Input 30.9 → 0.006 ms, frame 31.4 → 0.21 ms (test runner); player frame 44.9 → 6.7 ms |

### M5: loading off the lock

| | |
|---|---|
| **PRs** | #18, #19 |
| **Root cause** | Parsing, decoding and simplifying held `Assets`' global mutex; models loaded one at a time |
| **Solution** | Lock only to look up and store; worker threads; `LoadTicket`; draw nothing until a model is in |
| **Before → after** | Arid Dry Patch launch to assets 5,056 → 3,204 ms, worst frame 168 → 59 ms |
| **Trade-off** | Texture work landed on the first frame instead (Farmhouse regression, later addressed by L2 and #65) |

### M2: class and tag index

| | |
|---|---|
| **PR** | #17 |
| **Root cause** | ~15–20 full Workspace walks per frame across systems, each allocating |
| **Before → after** | Desktop S10k frame: test runner 6.68 → 5.91 ms, server 4.88 → 4.65, player 15.48 → 15.26 |
| **Note** | Small worlds gain nothing measurable (six real projects within noise) |

### I11, I10, I9, I8, I7, I4

| Item | Root cause → fix | Measured |
|---|---|---|
| I11 (#8) | First frame decoded every texture on the main thread → preload thread from project load | the `roads` sample first frame 2,069–5,431 → 341–398 ms |
| I10 (#9) | 0.5 s polling walk → `notify` events, polling fallback | Reload latency 463 → 103 ms (100 ms is the settle wait) |
| I9 (#11) | Allocating `descendants()`, SipHash on ID keys → `descendants_iter`, `FxHashMap` (needed `#[inline]`, else Scripts got 40% slower) | Input −15%, Scripts −6 to −10.5%, Characters −6.5% |
| I8 (#10) | A new mesh and LOD copies per MeshPart per frame → shared per model (`mesh_cache.rs`) | 2,000 helmets draw list 1.84–1.90 → 0.90–0.99 ms |
| I7 (#12) | Navmesh fingerprint hashed moving drops via `format!` → anchored ground only, incremental builder | Agents median 4.2–4.6 → 0.36–0.38 ms, worst 120–137 → 26–29 ms |
| I4 (#7) | `with_world` built a 14-Instance default World per call → swap into a `LentWorld` slot | 3.3 → 0.15 µs per call; S10k Scripts 0.391 → 0.361 ms |

### Threaded draw planning (3f2fc05)

| | |
|---|---|
| **Problem** | Painted forest at 1440p Ultra: 26 fps, 40.8 ms CPU on 1 of 32 threads |
| **Root cause** | `LodStates::resolve` 5.2, material keys 3.4, `upload_new` 8.4, `plan_draws` 12.9 ms; hashing a 330-byte `MaterialKey` twice per draw; every leaf lit then covered |
| **Solution** | rayon above 4,096 items in fixed-order shares; per-thing not per-draw work; cut-out depth fill; far cascades alternate past 12,000 casters; cheaper leaf sun visibility |
| **Before → after** | CPU `ScenePipeline::draw` 39.6 → 9.5–10.0 ms; GPU 31.0 → 13.6–13.9 ms; picture identical within noise |

### Graphics plan (Phases 12–15)

- **Culling (12)**: before it, edge views drew everything and shadows were 43–52% of the GPU frame. **LOD (13)**: 154.5 M triangles a frame for 2,000 helmets called for it; foliage cull distances by kind. **GPU skinning (14)**: 20 walking characters 4.85 → 0.38 ms CPU, mesh memory 13 → 2.3 MB. **Texture budget (15)**: caps texture memory by preset. Tables: `docs/rendering/performance.md`.
