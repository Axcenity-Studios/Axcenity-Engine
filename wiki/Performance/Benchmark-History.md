---
title: Benchmark history
type: performance
status: current
verified: 2026-11-11
verified_commit: 19c7179d
source: [benchmarks/baselines.json, benchmarks/results/2026-11-10-desktop-9950x3d.jsonl]
docs: [docs/benchmarks.md, docs/rendering/performance.md]
tags: [perf, history]
---

# Benchmark history

Dated whole-frame numbers, **never overwritten**: add a row or a section when you measure. All times are milliseconds, medians unless marked. Numbers compare only within one machine ([Hardware notes](Hardware-Notes.md)). Methods: [Benchmarking](Benchmarking.md).

## S10k, frame median by host

| Date | Commit | Machine | Test runner | Server | Editor Play | Player | Notes |
|---|---|---|---|---|---|---|---|
| 2026-11-09 | `95000ea` (I1) | Container | 18.6 | 9.8 | 16.8 | 97.8 | Player Render is lavapipe (61 ms) |
| 2026-11-09 | `300bbb5` (I1–I11) | Desktop | 10.0 | 5.4 | 8.7 | 21.5 | Draw list 8.38 ms was the player's largest step |
| 2026-11-10 | `940fabc` → M2 | Desktop | 6.68 → 5.91 | 4.88 → 4.65 | | 15.48 → 15.26 | Pinned, 7 alternating runs |
| 2026-11-10 | `300bbb5` → `5cb8302` | Desktop | 7.71 → 5.03 | 5.13 → 3.82 | | 14.93 → 8.73 | Pinned, after M1–M7, M13, M14, L2 1–7 |
| 2026-11-10 | `4da1528` → `1832da8` (M1) | Desktop | | | 5.99 → 5.09 | | `bench_project_play`, 7 pinned rounds |
| 2026-11-10 | `39dd258` (baseline file) | Desktop | 3.718 | 3.742 | 3.246 | 7.048 | `benchmarks/baselines.json` for `axcenity-bench-check` |
| 2026-11-11 | `15965da` (baseline file) | Desktop | 3.746 | 3.848 | 3.44 | 7.117 | Re-recorded with #65/#67/#69 (startup); frames within tolerance |
| 2026-11-11 | `e52292c` → part 1 (#84) | Desktop pinned | 3.71 → 2.47 | 3.71 → 2.38 | | | XB10k 3.41 → 2.08 |
| 2026-11-11 | `e3257295` → own pool (#86) | Desktop unpinned | 3.37 → 2.81 | | | 8.79 → 8.36 | |

## S10k, step medians at the two reference points

| Step | Container `95000ea` (test runner) | Desktop `300bbb5` (test runner) | Desktop `300bbb5` (player) |
|---|---|---|---|
| Characters | 2.84 | 2.70 | 2.41 |
| Agents | 3.99 (worst 134) | 0.32 (worst 13.6) | 0.35 (worst 12.6) |
| Input | 1.82 | 0.86 | 0.88 |
| Scripts | 0.34 | 0.12 | 0.14 |
| Physics | 7.59 | 4.94 | 5.46 |
| Heartbeat | 1.52 | 0.86 | 0.87 |
| Animation | 0.28 | 0.12 | 0.32 |
| Draw list | | | 8.38 |
| Render | | | 2.69 |

## Owner's projects (Desktop, player)

| Date | Commit | Project | Frame | First frame | Launch to assets in |
|---|---|---|---|---|---|
| 2026-11-10 | `300bbb5` | Abandoned Farmhouse | 4.77 | 1,054 | 4,692 |
| 2026-11-10 | `5cb8302` | Abandoned Farmhouse | 5.50 | 4,517 | 6,207 |
| 2026-11-10 | `39dd258` | Abandoned Farmhouse | 5.473 | 4,354 | 6,043 |
| 2026-11-10 | main (pre-#65), uncooked | Abandoned Farmhouse | 5.449 | 2,926 | 7,224 |
| 2026-11-10 | `924720a` (#65), cooked | Abandoned Farmhouse | 5.413 | 1,869 | 3,330 |
| 2026-11-10 | `300bbb5` | General Store Roadside | 6.99 | 1,076 | 2,428 |
| 2026-11-10 | `5cb8302` | General Store Roadside | 4.40 | 1,251 | 2,476 |
| 2026-11-10 | `39dd258` | General Store Roadside | 4.19 | 1,230 | 2,472 |
| 2026-11-10 | `300bbb5` | Temperate Wet Patch | 47.80 | 1,796 | 3,099 |
| 2026-11-10 | `5cb8302` | Temperate Wet Patch | 5.52 | 1,003 | 2,265 |
| 2026-11-10 | `39dd258` | Temperate Wet Patch | 5.279 | 820 | 2,064 |

| 2026-11-11 | `15965da` | Abandoned Farmhouse | 5.399 | 212 | 1,579 |
| 2026-11-11 | `15965da` | General Store Roadside | 4.092 | 446 | 1,581 |
| 2026-11-11 | `15965da` | Temperate Wet Patch | 5.14 | 268 | 1,399 |
| 2026-11-11 | `15965da` | Temperate Dry Patch, Ultra 2560×1440 | 11.043 (GPU 10.9, GPU-bound) | 282 | 1,559 |

The pre-#65 Farmhouse row is from the PR's own run and isn't directly comparable to the `39dd258` baseline row (different session).

## Rendering benchmarks (RX 9070 XT, High, 1280×720)

| Date | Commit | Benchmark | CPU `build_draw_list` | CPU `ScenePipeline::draw` | GPU frame |
|---|---|---|---|---|---|
| 2026-10 (graphics Phase 11) | benches added | 10,000 parts, overview | 3.28 | 2.44 | 1.92 |
| 2026-10 (graphics Phase 11) | benches added | 2,000 MeshParts | 0.64 | 0.92 | 9.29 |
| 2026-10 (graphics Phase 11) | benches added | Terrain + foliage | 6.48 | 12.14 | 5.99 |
| 2026-11-09 | `300bbb5` | 10,000 parts | 8.5–8.9 | | |
| 2026-11-09 | `556b72b` | 10,000 parts | 8.0–8.8 | | |
| 2026-11-09 | `3f2fc05` | Forest 2560×1440 Ultra | 1.2 → 1.3 | 39.6 → 9.5–10.0 | 31.0 → 13.6–13.9 |

**Upscaling (G9, `bench_upscaling`, 2026-11-11, `c004365`)**: a light field of blocks, High, FXAA, median GPU frame in ms over two runs. EASU and RCAS cost the upscaled frame minus a native frame at the small size.

| Output | 1.00 | 0.67 | 0.50 | Upscale cost (0.67 / 0.5) |
|---|---|---|---|---|
| 1920×1080 | 1.01–1.19 | 0.85 | 0.63 | 0.15 / 0.13 |
| 2560×1440 | 1.03–1.07 | 0.99–1.12 | 0.93 | noise / 0.24 |
| 3840×2160 | 1.90–1.92 | 1.27 | 1.01 | 0.20–0.26 / below noise |

**Cut-out shadows on foliage (CO7 review fixes 1, 2026-11-18, `bench_cut_out_foliage_shadows`)**: 24 trees of 1,500 cut-out leaf cards (36,000) under a low sun with its cascades, 1080p High with TAA, medians of 200 frames; Desktop, pinned to CPUs 0 to 15, five rounds alternating with `main` (`e1658afb`). Casting every cut-out through its holes was dropped for this then, and only hair cards did. Since 2026-11-26 every cut-out of the standard material does, at the owner's asking: in Desert Homestead (Ultra, 1920 × 1080) the Shadows pass is 0.13 to 0.17 ms dearer for it in five views and 0.42 ms in one (`docs/rendering/shadows.md`).

| Build | Shadows | Prepass | Scene |
|---|---|---|---|
| `main` `e1658afb` | 0.190–0.194 | 0.206–0.208 | 1.121–1.132 |
| Every cut-out through its holes (`fef3f3d2`) | 0.412–0.419 | 0.208–0.209 | 1.119–1.127 |
| Only hair through its holes (review fixes 1) | 0.188–0.193 | 0.209–0.210 | 1.124–1.130 |

(One `main` round, 0.203 / 0.270 / 1.267, was disturbed and is left out of the ranges.)

**Unexplained**: `bench_10000_parts`'s `build_draw_list` rose from 3.3 to 8–9 ms between the commit that added the render benchmarks and `556b72b`; not bisected ([Known bottlenecks](Known-Bottlenecks.md)).

## Collaborate on S10k (C11)

Container, release, 2026-11-12 ([Optimization log](Optimization-Log.md#collaborate-on-s10k-and-the-world-load-2026-11-12)):

| Measure | Before | After |
|---|---|---|
| Join message (the session's World) | 953 KiB | 82 KiB (zstd level 1, 1 ms) |
| A member opens the session's files as its project | 5.9 s | 0.35 s |
| `project::load` of S10k | 5.66 s (debug ~30 s) | 0.19 s (debug 0.8 s) |
| Idle `team_frame`, mean (median) | | host 48–57 µs (12–19), member 34–36 µs (2–3) |
| Four busy members' presence through the host | | 19 KiB a second |

The join over Tailscale between two machines is still to be recorded (goal: under 10 s).

## Character Studio's browser pages (2026-11-25)

Desktop under Fedora, a **debug** build, one run, `timing_tests.rs` `browser_costs` ([Optimization log](Optimization-Log.md#character-studios-tile-previews-2026-11-25)):

| Measure (the interface's thread) | Before | After |
|---|---|---|
| A garment tile the pointer crosses or scrolls past | 24–147 ms, median 44 (79 with a renderer) | nothing |
| A garment tile's first preview, pointer rested | as above | 41–104 ms, median 47 (its model read on the loader's thread) |
| A hairstyle tile's first preview, pointer rested | 83–143 ms, median 105 (133 with a renderer) | 20–41 ms, median 22 |
| A second preview of a tile | 7–49 ms, median 12 (hair 4) | the same |
| A still frame of the Clothing page (Hair, Body) | 1.25 ms (0.63, 0.69) | the same |

## Desert Homestead (2026-11-25)

Desktop under Fedora (radv, 240 Hz), release player, `--bench` at 2560 × 1440, one run each, not pinned; `467da9f2` against the fog and sky fix ([Optimization log](Optimization-Log.md#fog-boxes-and-the-atmospheres-sky-2026-11-25)). GPU medians.

| Measure | Before | After |
|---|---|---|
| GPU frame, Low / Medium / High (200 frames) | 50.5 / 78.3 / 110.3 | not run / 7.3–8.9 / 12.4 |
| GPU frame, Ultra, the tour's first shot | 138.9 (p95 389) | 16.9 |
| GPU frame, Ultra, the whole tour (5,500 frames) | not run | 17.1 (p95 20.7, worst 21.2) |
| CPU frame, Ultra, the tour's first shot | 8.0 | 7.7 |
| Render step through the dusk time-lapse | 29–37 | 10–13 |
| A frame as shown (whole runs timed), High / Ultra | not timed (the GPU frame alone gives 9 and 7 a second) | 20.6 / 23.4 ms |
| The frame on a cut to a view not seen before | 31–103 | 30–62 (ADR-0051: what it shares with earlier shots is kept) |
| The frame on a cut back to a seen view | 105 and 75 | 12–14 and 21–23 (ADR-0051) |
| GPU memory held after the tour, Ultra | not measured | 1,579 MiB textures, 174 MiB meshes |
| Memory budgets at the default settings (High) on this 16 GB card | 1,024 MiB textures (the scene's were shrunk to fit) | 4,096 MiB textures, 2,048 MiB meshes, from the card's memory |

### With the editor's ray traced settings

The same day and machine, `--bench 300` at 1996 × 998 (the editor's viewport) with the editor's saved graphics settings (Ultra, real-time GI, ray traced shadows, ambient occlusion and reflections at High, TAA); `a9370a3b` against swaying plants bent together ([Optimization log](Optimization-Log.md#swaying-plants-bent-together-2026-11-25)). Medians, one run each.

| Measure | Before | After |
|---|---|---|
| Render step (the frame's thread) | 33.8 | 14.3 |
| GPU frame | 33.3 | 23.8 |
| Acceleration structures | 12.82 | 2.48 |
| Ray traced ambient occlusion / shadows / reflections | 3.79 / 5.45 / 0.37 | 4.15 / 5.87 / 0.42 |
| Scene pass | 7.35 | 7.37 |
| A frame as shown (whole runs timed) | 73.7, 75.9, 88.7 ms | 41.7 ms |
| Swaying plants bent a frame | 4,118 (the budget), a BLAS each | 3,030 (those that move a pixel), in 141 BLASes |

### With frames started early (ADR-0052)

The same day, machine, settings and size, after the row above: waiting for the display to show each frame against `FramePacer` ([Optimization log](Optimization-Log.md#the-cpu-and-the-gpu-took-turns-2026-11-25)). Medians, one run each; the frame rate is `--bench`'s `as shown` (start to start), which earlier rows of this page don't have.

| Measure | Waiting for the display | Started early |
|---|---|---|
| Player, as shown | 24.5 a second | 67.0 a second |
| Player, 1% low as shown | 16.0 | 43.8 to 54.1 |
| GPU frame | 24.24 | 13.40 |
| Frame's thread (steps less waits) | 15.3 | 14.7 |
| Card busy / core clock / power | 59% / 1,511 MHz / 70 W | 89% / 3,136 MHz / 280 W |
| Player in a 2560 × 1405 window (asked for 3840 × 2160, which GNOME doesn't give), as shown | 20.7 a second | 53.0 a second |
| Debug editor (its own camera, default window), as shown | 26.7 | 41.0 |
| Release editor, as shown | 26.7 | 46.7 |
| Without terrain foliage: frame's thread / GPU frame | — | 5.0 / 8.8 |
| Without any ray traced effect: frame's thread / GPU frame | — | 9.3 / 8.4 (97.6 a second) |

### After plants by the cluster, and against the targets

The same day and machine ([Optimization log](Optimization-Log.md#plants-by-the-cluster-and-the-audits-second-round-2026-11-25)). The **tour** is the project's camera sequence (what `--bench` flies); the **still view** is the editor's start camera with the tour stopped (15,009 items drawn at 1996 × 998, 25,193 traced), the heavier. Release player, 1996 × 998, medians of one run each, frames a second as shown.

| Configuration | Paul's target | The tour | The still view |
|---|---|---|---|
| Standard Ultra (no ray tracing) | 120+ | 119.2 | 102.0 |
| Ultra with real-time GI, no rays | 90+ | 118.3 | 104.4 |
| Ray traced shadows and ambient occlusion | 90+ | 74.0 | 59.3 |
| Everything on (the editor's settings) | 60+ (100 for this project) | 71.8 | 57.5 |

| Measure | Before | After |
|---|---|---|
| Still view, frame's thread (the fifteen sections) | 18.52 ms | 16.45 ms |
| Still view, as shown | 51.0 | 57.0 |
| Tour as played (vertical sync, paced), as shown | 67.0 | 70.8 |
| Editor at its start camera (998 × 405 viewport), debug / release | 41.0 / 46.7 | 44.4 / 50.2 |
| Input to finished picture, tour as played, waiting → paced | 38.7 ms | 27.6 ms |
| The same in a 2560 × 1405 window, waiting → paced (→ without one frame ahead) | 47.1 ms | 35.6 ms (42.1) |
| A light scene (`samples/projects/part`) as played, waiting / paced | 238.0 a second, 3.05 ms | 239.6 a second, 3.00 ms |

### After the third round: encoding in parts, one query a ray

The same day and machine ([Optimization log](Optimization-Log.md#a-frame-encoded-on-several-threads-and-one-query-a-ray-2026-11-25)). Frames a second as shown, 1996 × 998, release player. "As launched" is one run with no pinning, as the table above was measured; "pinned" is the regression check's median of three runs on the CPU's V-cache cores (`taskset -c 0-7,16-23`), recorded in `benchmarks/baselines.json` as `desktop-9950x3d-fedora`.

| Configuration | Paul's target | Tour, as launched | Still view, as launched | Tour, pinned | Still view, pinned |
|---|---|---|---|---|---|
| Standard Ultra (`owner-ultra.json`) | 120+ | 132.6 | 114.7 | 143.7 | 179.5 |
| Ultra with real-time GI (`owner-ultra-gi.json`) | 90+ | 131.7 | 116.5 | 143.0 | 178.6 |
| Ray traced shadows and ambient occlusion (`owner-rt-shadows-ao.json`) | 90+ | 76.4 | 71.2 | 79.8 | 96.3 |
| Everything on (`owner-rt-all.json`) | 60+ (100 for this project) | 77.2 | 68.3 | 79.3 | 91.1 |

| Measure | Before | After |
|---|---|---|
| Still view, encoding (`finish`) | 4.00 ms | 1.57 ms |
| Still view, submit | 2.25 ms | 1.72 ms |
| Still view, frame's thread | 16.45 ms | 13.6 ms |
| Still view at 2560 × 1405 (GPU-bound, pinned): ray traced shadows / ambient occlusion / GPU frame | 6.57 / 3.39 / 17.21 ms | 4.66 / 2.54 / 14.53 ms |
| Tour as played (vertical sync, paced), as shown | 70.8 | 77.0 |
| The tour as played in a 1000 × 600 window | 43.8 | 97.9 |
| Debug editor at its start camera (998 × 405 viewport) | 44.4 | 52.0 (65.8 pinned to the V-cache cores) |
| Still view as launched / on the V-cache cores / on the other eight | — | 66.3 / 90.9 / 67.4 |

## Against other engines (XB10k)

`benchmarks/cross-engine/` defines XB10k (S10k's parts, physics and scripts rebuilt in each engine with one timing rule; PR #80). On the Desktop, 2026-11-11, median frame: **s&box 2.05 ms** (real time, not ranked), **Unity 3.11**, **Axcenity 5.14** (6.70 before #84/#86/#88/#93), **Godot 10.27**; Unreal and Roblox not timed yet. Results: `benchmarks/cross-engine/results.md`; what the numbers mean against each engine's features: `docs/research/engine-comparison.md`.

## Against Unity HDRP (Desert Homestead)

`benchmarks/cross-engine/homestead/` rebuilds Desert Homestead in Unity 6.6 HDRP from the engine's own draw list (the same meshes, levels of detail, cull distances, textures and terrain paint) and times six still views and the tour in both, pinned to the V-cache cores. Desktop under Fedora, 2026-11-25, main at `a9370a3b` with the working tree's changes, the owner's Ultra settings (`owner-ultra.json`), frames a second, the median of three runs:

| View | Axcenity 1920 × 1080 | Unity 1920 × 1080 | Axcenity 2560 × 1405 | Unity 2560 × 1405 |
|---|---|---|---|---|
| hero | 139.9 | 234.1 | 101.8 | 163.4 |
| mailbox | 151.7 | 232.6 | 114.8 | 161.7 |
| porch | 203.7 | 236.5 | 165.2 | 166.1 |
| truck | 163.1 | 226.7 | 123.0 | 160.4 |
| windmill | 149.8 | 256.8 | 106.9 | 179.0 |
| crane | 150.5 | 248.8 | 114.2 | 172.7 |
| the tour's first 61 seconds | 151.9 | 208.7 | 112.4 | 150.9 |

Every run was limited by the GPU. Where the engine's time goes against HDRP's (hero view, 1080p): clouds 1.36 ms against 0.30 (1.6 to 2.9 ms with their shadows, by taking them out of the world), sun shadows 1.76 against 1.38, volumetric fog 0.38 against 0.88, and about 3.5 ms of GPU time that doesn't depend on the window's size against 1.8. The frame's thread takes 4.0 to 5.0 ms against Unity's main thread's 1.5 to 1.7. Axcenity uses 1.1 to 2.1 cores' worth of CPU time against 3.6 to 6.8 and 2.6 to 2.9 GB of video memory against 3.8 to 4.3. Unity's player reports no ray tracing support here, so the ray traced configurations (77 to 78 frames a second at these views) have nothing to be compared with. Everything else, and the plan: `benchmarks/cross-engine/homestead/results.md`.

### After the traced clouds (2026-11-26)

The first step of the plan those numbers gave: the sky's clouds traced before the scene pass, their shapes from a texture, half size where sky shows and pixel by pixel where the half-size texels disagree (`renderer/cloud_trace.rs`, [Rendering](../Engine-Systems/Rendering.md)). Same machine, settings and method, the working tree on main at `a9370a3b`; Unity's column is the day before's.

| View | Before, 1920 × 1080 | Now | Unity | Before, 2560 × 1405 | Now | Unity |
|---|---|---|---|---|---|---|
| hero | 139.9 | 164.2 | 234.1 | 101.8 | 128.3 | 163.4 |
| mailbox | 151.7 | 162.9 | 232.6 | 114.8 | 125.2 | 161.7 |
| porch | 203.7 | 202.1 | 236.5 | 165.2 | 164.0 | 166.1 |
| truck | 163.1 | 197.4 | 226.7 | 123.0 | 167.3 | 160.4 |
| windmill | 149.8 | 230.4 | 256.8 | 106.9 | 211.9 | 179.0 |
| crane | 150.5 | 172.6 | 248.8 | 114.2 | 137.1 | 172.7 |
| the tour's first 61 seconds | 151.9 | 181.4 | 208.7 | 112.4 | 147.2 | 150.9 |

The `Clouds` pass is 0.04 to 0.30 ms at 1920 × 1080 (HDRP's clouds: 0.30) and the scene pass lost 1.1 to 2.7 ms where sky shows. Against the sky pass's own clouds the stills differ by 0.01 to 0.14 of 255 levels on average. The windmill view at 1920 × 1080 is now limited by the frame's thread (4.15 ms). The lit surfaces' cloud shadows are untouched (about 0.25 ms of the scene pass in the hero view).

**By what is drawn** (the second step: `DrawnPart`, stamps between draws; hero view, a line through 960 × 540, 1920 × 1080 and 2560 × 1405): the GPU's frame is 3.75 ms whatever the window's size plus 1.09 ms for each million pixels. Of the fixed part: shadows 1.07, the prepass 0.51, the scattered items in the scene pass 0.82, other items 0.47, volumetric fog 0.38. By the pixel: the terrain 0.35 ms a million pixels, items 0.19, the dust's sprites 0.16 (0.34 ms at 1920 × 1080, which the Unity port doesn't draw). The cut-out surfaces' depth is drawn in the prepass and again in the scene pass (0.12 to 0.18 ms).

## Raw data

`benchmarks/results/2026-11-10-desktop-9950x3d.jsonl` holds one JSON line per run of the 2026-11-10 before/after comparison.
