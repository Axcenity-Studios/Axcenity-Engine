# Desert Homestead: Axcenity against Unity HDRP, results

Measured 2026-11-25 on the owner's desktop: Ryzen 9 9950X3D, Radeon RX 9070 XT (16 GB), Fedora 45, kernel 7.3.4, Mesa 26.2.7 (RADV), GNOME on Wayland. What is compared and how: [README.md](README.md).

- **Axcenity**: `target/release/axcenity-player` built from main at a9370a3b with the working tree's changes, `benchmarks/features/owner-ultra.json` (Ultra, shadows Ultra to 300 studs, TAA, contact shadows, no ray tracing, no real-time GI).
- **Unity**: 6000.6.10f1 (Unity 6.6), HDRP 17.6.0, Vulkan, release Mono player, GPU Resident Drawer with GPU occlusion culling.
- Both pinned to CPUs 0-7 and 16-23, vsync off, a Wayland window, three runs of each (the median run is shown). Every run was limited by the GPU (99 to 100% load) unless a line says otherwise.

## In short

- **Unity HDRP draws this scene 1.2 to 1.7 times faster at 1920 × 1080 and 1.0 to 1.7 times faster at 2560 × 1405.** Five of the six views are between 1.3 and 1.7; the one with little sky in it (the porch) is 1.16 and 1.01.
- **Since then (2026-11-26) the clouds are done**: 0.04 to 0.30 ms with the same picture, and the gap is 1.1 to 1.4 times at 1920 × 1080 and 0.84 to 1.3 times at 2560 × 1405 ("After step 1" below). The rest of this section is as measured on 2026-11-25.
- **Then the scene pass (2026-11-26, later the same day)**: it lit most solid pixels before testing their depth. With the depth test first and the same picture, pixel for pixel, the six views are 186 to 246 frames a second at 1920 × 1080, 1.02 to 1.29 times behind Unity ("The plants' scene pass, profiled" below).
- **Then the draw order, foliage shadows through the leaves, and the thread (2026-11-26, third session)**: 200 to 254 frames a second at 1920 × 1080, 0.98 to 1.21 times behind Unity, with the one thing its picture had over the engine's (shadows that follow the leaves) now drawn ("Shadows through the holes, and the thread" below).
- **Then where a plant's shadow ends, and TAA in motion (2026-11-26, fourth session)**: shadows thin out over a band where they popped, and a moving picture stays a third sharper under TAA, for 3 to 6 frames a second (196 to 246 at 1920 × 1080, 1.01 to 1.23 times behind Unity). Cut-outs in the near cascades only was built and left off: most of their cost is in the near cascades ("Near cascades, where a plant's shadow ends, and TAA in motion" below).
- **Most of the gap is two things.** The engine's clouds cost 1.6 to 2.9 ms a frame at 1080p where the sky shows, against 0.3 ms for HDRP's. And the part of the engine's GPU frame that does not depend on the window's size is about 3.5 ms, against about 1.8 ms for HDRP's: at a quarter of the pixels the engine still needs 3.8 to 4.7 ms of GPU time a frame, which is what HDRP needs for the whole frame at full size (3.8 to 4.3 ms).
- **The frame's thread is the next limit.** It takes 4.0 to 5.0 ms (200 to 250 frames a second at most), 1.9 ms of it for the plants' 9,500 items. Unity's main thread takes 1.5 to 1.7 ms. With the clouds gone the windmill view is already limited by the thread, at 233 frames a second.
- **Axcenity is ahead on what it uses**: 1.1 to 2.1 cores' worth of CPU time against Unity's 3.6 to 6.8, 2.6 to 2.9 GB of video memory against 3.8 to 4.3 GB, volumetric fog at 0.38 ms against 0.88 ms, and it starts in 2.9 seconds against 4.2. It uses more memory (2.45 GB against 1.47 GB).
- **Unity cannot trace rays here.** Its player reports no ray tracing support with Vulkan under Linux, so only the rasterized configuration is compared. Axcenity's ray traced configurations run at 77 to 78 frames a second in the same views at 1080p.
- **The pictures are close** (`~/Projects/Unity Projects/Desert Homestead HDRP/Comparison/all-views.jpg`): the same framing, plants, terrain paint and lighting direction in all six views. What differs is listed under "Appearance".

## Frame rate

Frames a second (1% low in brackets). "Unity ÷ Axcenity" above 1 means Unity is faster.

1920 × 1080:

| View | Axcenity | Unity HDRP | Unity ÷ Axcenity | Axcenity's three runs | Unity's three runs |
|---|---|---|---|---|---|
| hero | 139.9 (133) | 234.1 (215) | 1.67× | 138–140 | 233–236 |
| mailbox | 151.7 (141) | 232.6 (225) | 1.53× | 152–153 | 231–234 |
| porch | 203.7 (158) | 236.5 (203) | 1.16× | 203–205 | 234–236 |
| truck | 163.1 (142) | 226.7 (205) | 1.39× | 162–163 | 224–227 |
| windmill | 149.8 (138) | 256.8 (241) | 1.71× | 149–150 | 256–257 |
| crane | 150.5 (135) | 248.8 (211) | 1.65× | 150–154 | 248–250 |
| the tour, 61 seconds | 151.9 | 208.7 | 1.37× | 151–152 | 206–210 |

2560 × 1405 (what GNOME gives a window asked to be 2560 × 1440; Unity was told to draw the same size):

| View | Axcenity | Unity HDRP | Unity ÷ Axcenity | Axcenity's three runs | Unity's three runs |
|---|---|---|---|---|---|
| hero | 101.8 (96) | 163.4 (159) | 1.61× | 102–102 | 163–165 |
| mailbox | 114.8 (108) | 161.7 (159) | 1.41× | 115–115 | 161–163 |
| porch | 165.2 (144) | 166.1 (162) | 1.01× | 164–166 | 166–166 |
| truck | 123.0 (118) | 160.4 (157) | 1.30× | 123–125 | 160–161 |
| windmill | 106.9 (104) | 179.0 (175) | 1.67× | 107–107 | 179–179 |
| crane | 114.2 (111) | 172.7 (168) | 1.51× | 114–115 | 172–173 |
| the tour, 61 seconds | 112.4 | 150.9 | 1.34× | 112–113 | 151–154 |

The tour shot by shot, 1920 × 1080, leaving out the half second after each cut:

| Shot | Axcenity | Unity HDRP | Unity ÷ Axcenity |
|---|---|---|---|
| hero | 141.8 (111) | 232.1 (187) | 1.64× |
| mailbox | 143.7 (136) | 210.0 (141) | 1.46× |
| porch | 201.6 (171) | 219.5 (137) | 1.09× |
| truck | 152.0 (141) | 210.9 (129) | 1.39× |
| windmill | 157.1 (144) | 262.2 (136) | 1.67× |
| crane | 140.5 (127) | 234.3 (129) | 1.67× |

Within a shot the engine's frames are the more even: its 1% low is 78 to 95% of its average, Unity's 52 to 81%. The frames at the tour's five cuts are not compared. The engine's take 28 to 60 ms. Unity's take about 170 ms, and most of that is the port's own doing: it sets all 65,490 LODGroups again when the field of view changes, because the engine culls plants by distance and Unity by size on screen.

## What the machine was doing

1920 × 1080, the median run. CPU cores used is the process's CPU time over the wall time, all its threads.

| View | Engine | Frame ms | GPU ms | CPU ms | GPU W | CPU cores used | Video memory MB | Memory MB | Triangles a frame |
|---|---|---|---|---|---|---|---|---|---|
| hero | Axcenity | 7.15 | 7.05 | 4.30 (frame thread) | 302 | 1.5 | 2,632 | 2,443 | 10.3 M |
| hero | Unity HDRP | 4.27 | 4.15 | 1.55 (main thread) | 258 | 5.8 | 3,846 | 1,472 | 7.5 M |
| porch | Axcenity | 4.91 | 4.80 | 4.11 | 266 | 2.1 | 2,599 | 2,487 | 9.7 M |
| porch | Unity HDRP | 4.23 | 4.09 | 1.56 | 245 | 5.8 | 3,841 | 1,469 | 6.9 M |
| windmill | Axcenity | 6.67 | 6.56 | 4.12 | 300 | 1.7 | 2,576 | 2,483 | 8.6 M |
| windmill | Unity HDRP | 3.89 | 3.78 | 1.60 | 259 | 6.8 | 3,848 | 1,460 | 5.4 M |
| crane | Axcenity | 6.64 | 6.53 | 5.05 | 301 | 2.0 | 2,626 | 2,468 | 10.7 M |
| crane | Unity HDRP | 4.02 | 3.90 | 1.58 | 260 | 6.5 | 3,846 | 1,480 | 6.7 M |

- **The same geometry.** From the dump, the engine's own rules select 9,788 items and 3.61 M triangles in the hero view. The engine reports 10,050 items drawn, and 10.3 M triangles over its depth prepass, its scene pass and its four shadow cascades (8.0 M with shadows off). Unity reports 7.5 M over its passes: it has no full depth prepass.
- **Energy.** At the hero view the card draws 302 W for 140 frames a second in Axcenity (2.2 joules a frame) and 258 W for 234 in Unity (1.1).
- **Unity's CPU time** is spread over its job threads, which is where its 5 to 7 cores' worth goes. Its render thread's time (2.4 to 2.9 ms at this size) includes waiting for the GPU and is not shown.
- **Start-up**, from launch to the first frame with everything loaded: Axcenity 2.9 seconds, Unity 4.2 (0.85 of it spawning the instances, which a scene saved with them in it would not need).

The engine's GPU time by pass, ms, 1920 × 1080 and 2560 × 1405:

| View | Shadows | Depth prepass | Volumetric fog | Scene | Everything else | Scene at 2560 × 1405 |
|---|---|---|---|---|---|---|
| hero | 1.07 | 0.69 | 0.38 | 4.66 | 0.22 | 6.99 |
| mailbox | 1.14 | 0.63 | 0.39 | 4.05 | 0.25 | 5.83 |
| porch | 1.36 | 0.64 | 0.40 | 2.09 | 0.28 | 2.94 |
| truck | 1.12 | 0.67 | 0.34 | 3.63 | 0.23 | 5.33 |
| windmill | 0.80 | 0.73 | 0.32 | 4.46 | 0.22 | 6.92 |
| crane | 0.95 | 0.77 | 0.39 | 4.17 | 0.23 | 5.94 |

The scene pass is two thirds of the frame wherever the sky shows, and half as long at the porch, where it mostly doesn't.

## What each feature costs

One feature off at a time (`ablate.sh`), hero view, 1920 × 1080, one run each. "Saved" is how much shorter the frame is without it.

| Off | Axcenity saved ms | Unity saved ms |
|---|---|---|
| Everything on | 7.14 ms a frame | 4.30 ms a frame |
| Clouds (Axcenity: a flat sheet in their place; Unity: none) | 1.36 | 0.30 |
| Sun shadows | 1.76 | 1.38 |
| Volumetric fog | 0.38 | 0.88 |
| Contact shadows | 0.37 | 0.05 |
| Anti-aliasing (TAA) | 0.28 | 0.12 |
| Ambient occlusion | 0.02 | 0.09 |
| Sun rays | 0.00 | not in the port |
| What is left with all six off | about 3.0 | about 1.5 |

The crane view gives the same order (clouds 0.99 against 0.26, shadows 1.64 against 1.14, fog 0.37 against 0.89).

## Experiments on the engine alone

Each is a copy of the standing-still project with one thing changed, or one graphics setting changed. 1920 × 1080, one run.

| Change | View | FPS | Frame ms | Scene ms | Depth prepass ms | Shadows ms | Frame thread ms |
|---|---|---|---|---|---|---|---|
| None | hero | 139.9 | 7.15 | 4.66 | 0.69 | 1.07 | 4.30 |
| None | windmill | 150.4 | 6.65 | 4.45 | 0.73 | 0.79 | 4.12 |
| None | porch | 204.6 | 4.89 | 2.10 | 0.63 | 1.35 | 4.13 |
| No clouds (`Cover` 0) | hero | 180.7 | 5.53 | 3.04 | 0.69 | 1.06 | 4.32 |
| No clouds | windmill | 232.8 (limited by the thread) | 4.30 | 1.57 | 0.75 | 0.80 | 4.16 |
| No clouds | porch | 209.2 | 4.78 | 1.96 | 0.64 | 1.36 | 4.13 |
| No wind | hero | 146.1 | 6.84 | 4.55 | 0.48 | 1.07 | 3.88 |
| No painted foliage | hero | 170.4 | 5.87 | 3.86 | 0.31 | 0.96 | 2.39 |
| No painted foliage | porch | 265.2 | 3.77 | 1.45 | 0.31 | 1.21 | 2.14 |
| A quarter of the pixels (render scale 50%) | hero | 209.8 | 4.77 | 2.41 | 0.55 | 1.05 | 4.33 |
| A quarter of the pixels | porch | 230.8 (limited by the thread) | 4.33 | 1.22 | 0.50 | 1.38 | 4.20 |
| A quarter of the pixels, flat clouds | hero | 222.0 (limited by the thread) | 4.50 | 1.88 | 0.55 | 1.06 | 4.37 |

What they say:

- **Clouds are 1.6 ms at the hero view and 2.9 ms at the windmill** (the sky march, and the cloud's shadow worked out from seven noise look-ups in every lit pixel and every fog cell). The windmill view goes from 150 to 233 frames a second without them and then waits on the frame's thread.
- **Wind is 0.3 ms of GPU time and 0.4 ms of the thread's** (the prepass draws the swaying plants' motion; `Motion history` goes from 0.45 to 0.07 ms). Unity's plants don't sway, so the fairest single number for the engine is the no-wind one: 146 against 234 at the hero view, still 1.6 times.
- **The plants are 1.3 ms of GPU time and 1.9 ms of the thread's**: 9,500 of the 10,050 items drawn.
- **About 3.5 ms of the engine's GPU frame doesn't depend on the window's size.** From the two sizes, the hero view costs 3.5 ms plus 1.75 ms a million pixels, and Unity's 1.8 ms plus 1.2. At the porch, with little sky, the engine's cost per pixel is the lower of the two (0.75 against 1.17 ms a million), and its fixed part is still 3.4 against 1.8. At a quarter of the pixels the scene pass still takes 1.2 to 1.9 ms and the prepass 0.5.
- **Without foliage and without clouds the hero view's scene pass would still be about 2.2 ms**: the terrain, the house, the truck, the mountains and the clear sky. What of that is the terrain's shader and what is geometry isn't known yet.

## Ray traced configurations (Axcenity only)

Unity's player reports `SystemInfo.supportsRayTracing` false on this machine, so HDRP's ray traced effects can't be run for comparison. Axcenity, 1920 × 1080, one run:

| Configuration | hero | crane | The owner's target |
|---|---|---|---|
| Ultra (`owner-ultra.json`) | 139.9 | 153.6 | 120 |
| Ultra with real-time GI (`owner-ultra-gi.json`) | 139.5 | 150.2 | 90 |
| Ray traced shadows and ambient occlusion (`owner-rt-shadows-ao.json`) | 78.3 | 78.4 | 90 |
| Ray traced reflections as well (`owner-rt-all.json`) | 76.8 | 77.8 | 60 (100 for this project) |

## Appearance

From the stills of the six views, side by side:

- **The same**: framing, every plant and object in its place, the terrain's paint (the track, the pale ground by the house, the soil under plants), the direction and length of shadows, the haze at the foot of the range, the overall brightness (mean within about 6% in the three views checked).
- **Foliage shadows.** As measured on 2026-11-25 the engine's rasterized shadows were each card's whole outline, so plants cast dark blocks, where HDRP's follow the leaves and read as lighter and more believable: the clearest quality difference in Unity's favour. Since 2026-11-26 the engine's follow the leaves too (below).
- **Clouds.** The engine's are wispy and torn, with more shape in them than HDRP's default cumulus. In the engine's favour, and what it pays 1.6 to 2.9 ms for.
- **Mountains.** Lit evenly in the engine. In Unity's pictures a cloud's shadow lies over the right half of the range. A difference in where the clouds are, not in what either engine can do.
- **Ground and plants.** Unity's are warmer and paler (its tone curve, and HDRP's leaf translucency). The engine's ground has more contrast.
- **Motion** can't be judged from stills: the engine's plants sway and Unity's don't.

## Where each is ahead

| | Ahead | By |
|---|---|---|
| Frame rate, rasterized Ultra | Unity | 1.2 to 1.7× at 1080p, 1.0 to 1.7× at 1405p |
| Clouds' cost | Unity | 0.3 against 1.4 to 2.9 ms |
| GPU time that doesn't depend on the size | Unity | about 1.8 against 3.5 ms |
| The frame's thread | Unity | 1.5 to 1.7 against 4.0 to 5.0 ms |
| Contact shadows, TAA | Unity | 0.05 and 0.12 against 0.37 and 0.28 ms |
| Memory | Unity | 1.47 against 2.45 GB |
| Foliage shadows' look | Unity on 2026-11-25; level since 2026-11-26 | both follow the cut-out |
| CPU time used | Axcenity | 1.1 to 2.1 against 3.6 to 6.8 cores |
| Video memory | Axcenity | 2.6 to 2.9 against 3.8 to 4.3 GB |
| Volumetric fog's cost | Axcenity | 0.38 against 0.88 ms |
| Evenness of frames in a shot | Axcenity | 1% low at 78 to 95% of the average against 52 to 81% |
| Start-up | Axcenity | 2.9 against 4.2 seconds |
| Hardware ray tracing under Linux | Axcenity | Unity has none here |
| Wind, cloud shapes | Axcenity | not in the port; the engine's own |

## Plan of action

In the order the measurements give, as revised after the owner's review (2026-11-25). Each step keeps the picture as it is (the owner's rule: no frame rate bought by drawing less), starts from a measurement that fails today, and is checked by running `run.py` again. The frame rates given as "expected" are estimates from taking things away. None of them is a target until the change itself is measured.

1. **Clouds, first: work them out once, not per pixel per frame. Done 2026-11-26, except their shadows: see "After step 1" below.** 1.6 to 2.9 ms of a 6.6 to 7.2 ms frame, and HDRP shows 0.3 ms is enough for the same job. The clearest measured gap. What was built differs from the first two points below where measuring said so: nothing is kept between frames (half size and the shapes' texture were enough, and it leaves nothing to trail), and where half size is not enough a pixel is worked out in full.
   - Trace the cloud layer into a target of its own at half size and keep it between frames, moved by the camera's turn and the wind (the way TAA keeps its history), then lay it into the sky. Ultra keeps its 64 and 6 steps; they are just not all paid for by every sky pixel every frame.
   - Read the cloud's shape from tiling noise textures baked from the same functions, in place of five to seven value-noise look-ups worked out by arithmetic at every step.
   - Draw the cloud's shadow into a small top-down texture once a frame and sample it in `light_surface` and in the fog, in place of `cloud_fill` in every lit pixel.
   - Accepted on pictures as well as time: before and after, side by side, of the clouds' edges (against the sky and against the horizon's silhouettes), their motion (a turning camera, a cut, the wind's drift: no trails, no swimming), their lighting (noon and a low sun), and their shadows on the ground held still and drifting (no shimmer, no crawl). The existing cloud tests' pictures stay within their tolerance.
   - Timed by a "Clouds" GPU timer of its own. From the no-clouds runs plus HDRP's 0.3 ms the windmill view would go from 150 to about 215 frames a second at 1080p and the hero view from 140 to about 170: an estimate to be replaced by the measurement.
2. **GPU timers by what is drawn: required before any work on the scene pass. The timers and their first table are done (2026-11-26): see "After step 2" below, which sets what follows. Its first two moves (the plants' per-triangle cost, the cut-outs' depth) are done too: "The plants' scene pass, profiled".** At a quarter of the pixels the scene pass and the prepass still take 1.7 to 2.4 ms together; HDRP's whole fixed cost, shadows included, is 1.8 ms. Today only the whole pass is timed, and the experiments above give its parts only by taking things away.
   - Timers inside the prepass and the scene pass by what is drawn (terrain, painted foliage, models, sky and clouds), in `--bench`'s table, read at each view at a quarter of the pixels, at 1920 × 1080 and at 2560 × 1405, so each part has a fixed cost and a per-pixel cost.
   - Nothing below is started before that table exists, since without it the work could go to the wrong part. Then, by what it shows: the terrain's shader (seven layers with anti-tiling and height blend in every pixel) if the terrain is the large part; drawing solid geometry once where the prepass is not needed for it; culling items hidden behind others from the prepass's depth before the scene and shadow passes draw them (Unity culls on the GPU and has no full prepass: 7.5 M triangles a frame against the engine's 10.3 M for the same 3.6 M in view).
3. **The frame's thread: a prototype of plants kept on the GPU, before any rewrite. Started 2026-11-26 (the motion history and the grouped draws' materials shared among threads, two per-frame costs that were not the plants' removed; the GPU candidate is not built): "Step 3, started" and "Shadows through the holes, and the thread".** 4.0 to 5.0 ms a frame is a ceiling of 200 to 250 frames a second. In the runs above the GPU was the limit at both sizes (its frame took longer than the thread's in every view but the porch at 1920 × 1080); with step 1 done the windmill view reaches the thread's ceiling, and steps 2 and 4 will bring the others to it. The plants' items are 1.9 ms of it: `Levels of detail`, `Materials`, `Uploads`, `Culling and batches` and `Motion history` are 2.2 ms together today, and the encoding of 3,500 draws another 0.9. Unity's main thread does the same scene in 1.5 ms, on 3.6 to 6.8 cores' worth of CPU time against the engine's 1.1 to 2.1.
   - Two candidates, measured against each other and against today's path at the same views. **Worker threads**: the plants' culling, level-of-detail choice and instance packing shared among the renderer's threads (some of the frame's planning is shared already, `renderer/parallel.rs`), which keeps every draw as it is. **On the GPU**: each foliage group's instances in a buffer that stays there, with what is in view, its level of detail and its cull distance chosen by a compute pass and drawn indirectly (`multi_draw_indexed_indirect_count`, which wgpu has on Vulkan and DirectX 12 as a native feature), so the thread's work is per group and not per plant.
   - Behind a switch, with today's path kept as the fallback (and for adapters without indirect counts).
   - Compared on: the thread's time, GPU time, the process's memory and the card's, and the picture (the same plants at the same levels of detail, the same cross-fades, sway, shadows and motion vectors).
   - The rewrite is decided from that comparison, not from Unity's number.
4. **Shadows, by measurement only: 1.05 to 1.38 ms whatever the window's size.** The same as HDRP's, so this is not where Unity is ahead, but it is a fifth of the frame. Measure the four cascades one by one first (as "Shadows on the patch, cascade by cascade" did). The far cascades change little from frame to frame and are the candidates for being kept; the engine uses over 1 GB less of the card's memory than HDRP here, so there is room to keep them. A kept cascade must still show the plants' sway where a texel of it can: that is part of what is measured. Nothing is changed without a cascade's measurement that says so.
5. **Cut-out shadows for foliage in the rasterized path: a matter of the picture, kept apart from the speed work. Done 2026-11-26 at the owner's asking, on in every cascade, its cost measured on its own: "Shadows through the holes, and the thread". The near cascades only was then built and measured, and is off: "Near cascades, where a plant's shadow ends, and TAA in motion".** The one place Unity's picture is plainly better. The shadow pass already has the pipeline (`fs_shadow_cut_out`, which hair cards cast through); leaf cards were left casting whole because through their holes the Shadows pass of 36,000 leaf cards went from 0.19 to 0.42 ms (`bench_cut_out_foliage_shadows`, `renderer/lighting.rs`). So this is a switch and a measurement in Homestead, not a new shader: solid casters' draws don't change. Its cost is measured on its own (shadow time with and without, view by view), not folded into step 4's numbers, and the shadow pass is not made dearer just to match the reference: if the cost is large it becomes a quality setting's choice, or cut-outs in the near cascades only, where they can be seen.
6. **Put off until the large items are done**: contact shadows (0.37 against 0.05 ms), TAA (0.28 against 0.12), and memory (2.45 against 1.47 GB: look for meshes' and textures' CPU copies kept after upload).
7. **Ray tracing is its own track**: 77 to 78 frames a second against a target of 90 for shadows and ambient occlusion, with nothing in Unity to compare against on this machine. Steps 1 to 3 help it too: its frames contain the same clouds, scene pass and thread work.

After steps 1 to 3, by these numbers, the hero view at 1080p would be a little over 4 ms of GPU time and about 2.5 ms on the thread: about where HDRP is. That is an estimate from taking things away, not a measurement of the changes.

## After step 1: the traced clouds (2026-11-26)

The sky's clouds are now worked out before the scene pass: their shapes read from a texture, at half size where sky shows, then pixel by pixel, afresh wherever the half-size texels disagree (`crates/render/src/renderer/cloud_trace.rs`, [docs/rendering/clouds.md](../../../docs/rendering/clouds.md#the-traced-clouds)). Nothing is kept from one frame to the next. Same machine, settings and method; Axcenity run again (three runs, the median shown), Unity's numbers are those of 2026-11-25 above.

| View | Axcenity before | Axcenity now | Unity | Unity ÷ Axcenity now | Axcenity before | Axcenity now | Unity | Unity ÷ Axcenity now |
|---|---|---|---|---|---|---|---|---|
| | 1920 × 1080 | | | | 2560 × 1405 | | | |
| hero | 139.9 | 164.2 | 234.1 | 1.43× | 101.8 | 128.3 | 163.4 | 1.27× |
| mailbox | 151.7 | 162.9 | 232.6 | 1.43× | 114.8 | 125.2 | 161.7 | 1.29× |
| porch | 203.7 | 202.1 | 236.5 | 1.17× | 165.2 | 164.0 | 166.1 | 1.01× |
| truck | 163.1 | 197.4 | 226.7 | 1.15× | 123.0 | 167.3 | 160.4 | 0.96× |
| windmill | 149.8 | 230.4 | 256.8 | 1.11× | 106.9 | 211.9 | 179.0 | 0.84× |
| crane | 150.5 | 172.6 | 248.8 | 1.44× | 114.2 | 137.1 | 172.7 | 1.26× |
| tour (61 s) | 151.9 | 181.4 | 208.7 | 1.15× | 112.4 | 147.2 | 150.9 | 1.03× |

The runs' ranges are within 1.5% of their medians. In the larger window the engine is now level with HDRP over the tour and ahead in two views. The windmill view at 1920 × 1080 is limited by the frame's thread (4.15 ms; the GPU's frame is 4.16 ms at 98% load), as step 3 expects.

GPU time, ms, 1920 × 1080:

| View | Scene pass before | Scene pass now | Clouds pass | Saved | GPU frame before | GPU frame now |
|---|---|---|---|---|---|---|
| hero | 4.66 | 3.32 | 0.30 | 1.04 | 7.05 | 6.01 |
| mailbox | 4.05 | 3.50 | 0.14 | 0.41 | 6.49 | 6.06 |
| porch | 2.09 | 2.10 | 0.04 | −0.05 | 4.80 | 4.86 |
| truck | 3.63 | 2.35 | 0.23 | 1.05 | 6.03 | 4.98 |
| windmill | 4.46 | 1.74 | 0.30 | 2.42 | 6.56 | 4.16 |
| crane | 4.17 | 3.09 | 0.26 | 0.82 | 6.53 | 5.71 |

The clouds pass is 0.04 to 0.30 ms, against HDRP's 0.30 (0.06 to 0.44 ms in the larger window). Drawing the shapes' texture again, when the camera has moved 1,200 studs through the clouds, costs 0.47 ms once (measured by drawing it every frame).

**The picture**, traced against the sky pass's own clouds (which `AXCENITY_NO_CLOUD_TRACE=1` still draws), stills at 1920 × 1080 with TAA settled over 24 frames, in `target/cross-engine/homestead/clouds/` (each `*-compare.jpg`: the sky pass's, the traced, their difference eight times over):

| View | Mean difference of a channel (of 255) | Pixels more than 4 apart | More than 8 apart | Mean brightness, before and now |
|---|---|---|---|---|
| hero | 0.040 | 0.10% | 0.003% | 120.42, 120.39 |
| mailbox | 0.006 | 0.007% | 0.001% | 111.95, 111.95 |
| windmill | 0.143 | 0.76% | 0.04% | 95.44, 95.37 |
| crane | 0.025 | 0.06% | 0.003% | 115.56, 115.54 |
| wide (the tour at 78 s) | 0.041 | 0.10% | 0.003% | 116.35, 116.32 |

- **Edges.** A silhouette in front of the sky is the sky pass's own, at full size: only the clouds behind it come from the trace. A cloud's own edge is where the half-size texels disagree, so it is worked out per pixel as before, from the same per-pixel offset.
- **Motion.** There is no history, so a turning camera, a cut or the wind's drift cannot leave a trail: every frame is the picture a still of that moment would be. Under TAA a still view changes from one settled frame to the next by 0.391 of a level with traced clouds and 0.389 with the sky pass's (`traced_clouds_flicker_no_more_than_the_sky_passs`). The shapes' texture moves in whole texels, so drawing it again changes nothing (`traced_clouds_dont_change_when_their_shapes_are_drawn_again`).
- **Lighting.** Noon in the stills above. A low sun (24° up, the view toward the horizon behind posts) is in the renderer's tests at 512 × 512: against a picture drawn at four times the size and brought down, the traced clouds' error is 0.73 of a level on Ultra and the sky pass's 0.70, and the worst 32-pixel square is 1.42 against 1.40 levels. The tour's dusk could not be shot still (the still tool does not run the time of day), so a low sun in Homestead itself is not shown.
- **Shadows.** The clouds' shadows on the ground are untouched: every lit pixel still works them out from the noise, as before, so they are the same and as steady as they were. That is also what is left of the clouds' cost in the scene pass (about 0.25 ms in the hero view, by the no-clouds run): the item shader has no texture binding free for the shapes' texture, so it needs another way in.
- **Two findings on the way.** At 2048² the shapes' texture made thin veils visibly different, so it is 4096² (32 MB). And reading between half-size texels everywhere, without the per-pixel step, made a busy horizon up to 10 levels too bright in the test view; with it the cloud pass costs 0.19 ms more in the hero view and matches.

What it costs elsewhere: three textures, 52 MB at 1920 × 1080 (the shapes' 32 MB, a half-size and a full-size target). The process's video memory as the system reports it rose by 221 to 262 MB in every view (2.80 to 2.89 GB at 1920 × 1080). The rise is nearly the same in every view and five times the textures' size, which is what one more block from the memory allocator would look like, but that was not checked. The frame's thread is unchanged (4.1 to 4.9 ms).

One caution on the 1% lows: the three-run tables were measured while other work ran on the machine, and their 1% lows at 1920 × 1080 are lower than a quiet rerun's (hero 147 to 156, porch 157 to 160 frames a second, against the porch's 156 to 158 with the sky pass's clouds). The averages agree within 1%.

## After step 2: GPU time by what is drawn (2026-11-26)

The depth prepass and the scene pass are now timed by what is drawn, with a stamp between draws wherever that changes (`DrawnPart` in `crates/render/src/renderer/gpu_timer.rs`; `--bench` prints the table, `"gpu_parts"` in its JSON). For that, solid items are sorted by distance band and then by part: items, items scattered over the terrain (its painted plants, stones and litter), then the terrain. The new order and the stamps change nothing that can be measured: over three runs at 1920 × 1080 the hero view is 164.2 frames a second before, 165.7 with the new order and no stamps, 164.0 with them; the porch 202.1, 204.1, 203.3; the windmill 230.4, 228.6, 229.6; and the scene pass is within 0.05 ms in each. The frame's thread reads 0.07 to 0.15 ms higher in all three views (4.30 to 4.45 ms in the hero view, with 39 more draw calls of 3,500: a mesh that is both scattered and placed no longer shares a batch), which may be a real cost of about 2%.

GPU time, ms, at 1920 × 1080 (one run, with sprites timed apart; the three-run table, in which sprites were still counted with see-through items, agrees within 0.03 ms. The parts of the scene pass add up to it within 0.05 ms; the prepass's remainder is the motion vectors' pass, 0.24 ms in the hero view):

| View | Prepass: items | Prepass: scattered | Prepass: terrain | Scene: items | Scene: scattered | Scene: terrain | Scene: cut-outs' depth | Scene: sky | Scene: sprites |
|---|---|---|---|---|---|---|---|---|---|
| hero | 0.22 | 0.20 | 0.03 | 0.87 | 0.93 | 0.90 | 0.14 | 0.02 | 0.34 |
| mailbox | 0.23 | 0.18 | 0.03 | 1.02 | 0.73 | 1.15 | 0.14 | 0.01 | 0.42 |
| porch | 0.23 | 0.17 | 0.03 | 1.29 | 0.54 | 0.06 | 0.14 | 0.00 | 0.00 |
| truck | 0.22 | 0.18 | 0.03 | 0.94 | 0.70 | 0.45 | 0.12 | 0.02 | 0.00 |
| windmill | 0.14 | 0.27 | 0.01 | 0.43 | 0.99 | 0.02 | 0.18 | 0.04 | 0.00 |
| crane | 0.19 | 0.25 | 0.04 | 0.69 | 1.19 | 0.93 | 0.14 | 0.01 | 0.06 |

Decals and see-through items are 0.00 to 0.02 ms everywhere.

**By resolution.** Each view once more at 960 × 540 and 2560 × 1405 (one run each), and a line fitted through the three sizes: what a part costs whatever the window's size (fixed, ms) and what it costs for each million pixels of window (ms). Hero view, with the range over the six views:

| | Fixed, hero | Per million pixels, hero | Fixed, all views | Per million pixels, all views |
|---|---|---|---|---|
| Shadows | 1.07 | 0.00 | 0.83 to 1.37 | 0.00 |
| Depth prepass (whole) | 0.51 | 0.09 | 0.44 to 0.64 | 0.04 to 0.10 |
| Volumetric fog | 0.38 | 0.00 | 0.33 to 0.40 | 0.00 |
| Clouds | 0.16 | 0.07 | 0.01 to 0.16 | 0.01 to 0.08 |
| Scene: scattered items | 0.82 | 0.07 | 0.51 to 0.93 | 0.02 to 0.14 |
| Scene: items | 0.47 | 0.19 | 0.20 to 0.47 | 0.11 to 0.46 |
| Scene: terrain | 0.14 | 0.35 | 0.02 to 0.24 | 0.00 to 0.39 |
| Scene: cut-outs' depth | 0.07 | 0.04 | 0.06 to 0.15 | 0.01 to 0.04 |
| Scene: sprites | 0.00 | 0.16 | 0.00 | 0.00 to 0.21 |
| Post-processing and ambient occlusion | 0.07 | 0.08 | 0.06 to 0.07 | 0.06 to 0.10 |
| **GPU frame** | **3.75** | **1.09** | **3.30 to 3.77** | **0.38 to 1.22** |

What the table says, largest first:

1. **The fixed 3.3 to 3.8 ms is geometry, and its largest single part is shadows.** Shadows are 0.8 to 1.4 ms and do not move with the window (step 4). The prepass is 0.44 to 0.64 ms. The scene pass's fixed part is 0.9 to 1.6 ms, and about half of that or more is the scattered items.
2. **The scattered items cost the scene pass 0.5 to 0.9 ms however few pixels they cover**, and the prepass 0.11 to 0.23 ms for the same triangles. A part that is four to six times dearer in the scene pass than in the prepass, and does not get cheaper with fewer pixels, is paying per triangle: either the scene's vertex work (sway, and what lighting needs carried to each corner) or the shading of triangles smaller than a pixel's neighbourhood. Which of the two is not yet known, and it decides what to do: the next measurement is the scene pass with the plants' pixels shaded flat.
3. **Their cut-outs' depth is drawn twice.** The scene pass draws the cut-out surfaces' depth before lighting them (0.12 to 0.18 ms), though the prepass has just drawn the same depth. The scene pass could test against the prepass's depth where it has one.
4. **The terrain is the dearest pixel**: 0.35 to 0.39 ms for each million pixels of window where the ground fills the lower half of the view (hero, mailbox, crane), 0.9 to 1.15 ms at 1920 × 1080. Its seven layers with anti-tiling and height blend are the candidate, as the plan guessed, but for three of six views, not as the frame's fixed cost.
5. **Items** (the house, the truck, the windmill) are 0.4 to 1.3 ms at 1920 × 1080 and largest on the porch, where the house fills the window (0.46 ms for each million pixels).
6. **The dust is 0.34 to 0.42 ms in the hero and mailbox views** (0.60 and 0.76 ms at 2560 × 1405), all of it by the pixel: two emitters' sprites drawn over each other. The Unity port has no dust, so in those two views, and in the tour's first two shots, the engine draws something Unity does not. "What differs" called this small; it is a tenth of the scene pass there. Taking its time off the engine's frame, those two views would be about 1.33 to 1.35 times apart at 1920 × 1080, not 1.43 (worked out, not run).
7. **The sky is 0.00 to 0.06 ms** now that its clouds are traced before it.

So step 2's next moves, in this order and each from its own failing measurement: find which per-triangle cost the scattered items pay in the scene pass; let the scene pass use the prepass's depth for cut-outs; then the terrain's shader. Occlusion culling is not what the table points to first: nothing here says hidden items are a large share, and that needs its own count (items drawn whose pixels all fail the depth test) before it is built.

## The plants' scene pass, profiled (2026-11-26)

"After step 2" left one question: why the same plants cost the scene pass four to six times what they cost the prepass, whatever the window's size. The owner's brief asked for three comparisons before anything was changed. All numbers are at 1920 × 1080 on the owner's Ultra, the engine pinned as before; the tables are three runs each, the median shown.

### What the three comparisons found

**1. What the vertex shader carries to each corner.** The scene pass's fragment shader reads all 13 values `vs_main` hands on (32 numbers a corner). The prepass's has use for 9 of them (22 numbers): not for the light an item gives off, the item's middle and half size (the leaves' canopy shading) or whether it hangs. That count is from reading the shaders; what the driver then drops was not dumped. To price that, a measuring switch draws the scattered items' pixels with part of the work left out (`AXCENITY_PLANT_PROBE`, `crates/render/src/renderer/probe.rs`; the picture is wrong while it is on):

| Scattered items in the scene pass, ms | Hero | Windmill |
|---|---|---|
| As drawn | 0.94 | 1.01 |
| `flat`: only the position carried, a pixel costs nothing | 0.18 | 0.27 |
| `carried`: all 13 values carried and read, nothing done with them | 0.24 | 0.33 |
| `surface`: the material's textures read, nothing lit | 0.22 | 0.31 |
| The prepass, the same triangles | 0.20 | 0.27 |

So the corners' own work and the rasterizer's are 0.18 to 0.27 ms, the prepass's cost exactly. Carrying everything is 0.06 ms. The textures are nothing that can be measured. **Lighting the pixels is 0.7 ms of the 0.94 to 1.01.** A leaner plant vertex would save at most 0.06 ms here: that is not where the time is, and it sets what the step 3 prototype should expect from a lean payload.

**2. Draw calls.** In the hero view the frame's 3,532 draw calls are 879 in the scene pass, 869 in the prepass, 169 for the cut-outs' depth, 70 for the motion vectors and 1,545 in the shadow views. The plants are 339 of the scene pass's batches (9,473 items: 28 to a draw) and the same 339 in the prepass, so the count is not what differs between the two passes. What does matter is what changes between one draw and the next: see "The order of the draws" below.

**3. The depth test against `discard`.** This is where the time was. The standard material's fragment shader, `fs_main`, can `discard` (a cut-out's holes, a fading item's dither), and it writes depth. A GPU cannot test depth before running such a shader, because it does not know yet whether the pixel will be there to write its depth: it runs the whole shader first and tests after. Cut-outs already had their own way round this (their depth drawn first, then lit where depth is equal). Everything else solid did not: the house, the truck, the mountains, the stones, every plant part without holes. Each of their pixels was lit, shadows and clouds and all, whether or not it ended up hidden, and it stayed 0.7 ms however small the window because the hidden ones were lit too.

### What was changed, with the picture the same

Three things are in, on by default, each with a switch that puts the old path back. With all three the six views' stills (1920 × 1080, TAA settled over 24 frames) are **the original's pixel for pixel: 0 of 2,073,600 pixels differ in each view** (`target/cross-engine/homestead/scene-depth/`: `original` and `default`).

- **Solid items that leave no pixel out are lit by a shader with no `discard`** (`fs_prefilled`, with the plain depth test and depth written as before; `PrefillPipelines::whole`). A solid item of the standard material with no cutoff, in a batch where nothing is fading, has nothing `fs_main` would discard, so the two shaders give the same pixels, and the GPU now tests depth before lighting. A batch with a fading item in it keeps `fs_main`. Off with `AXCENITY_NO_WHOLE_SOLIDS=1`.
- **The scene pass's depth starts from the prepass's**, one representable step farther, in place of the far plane (`crates/render/src/renderer/depth_start.rs`). The prepass has drawn the same solid items from the same corners, so what will show is still nearer than what is there and is drawn as before, and whatever is behind it fails the test before it is lit, in whatever order it comes. The step back is what keeps the picture exact: a surface's own pixel still passes the same "nearer than" test and writes its own depth, so a second surface at exactly that depth fails as it always did. Only where the two passes have one sample a pixel each and the prepass is drawn: not under MSAA, which draws as before. The terrain's position is now `@invariant` like the items', which this needs. Off with `AXCENITY_NO_DEPTH_START=1`.
- **The motion vectors' instances** are worked out on the renderer's threads and only for the plants the camera sees (step 3's first piece, below). Off with `AXCENITY_NO_MOTION_SHARING=1`.

| View | As it was | Now | Unity | Unity ÷ now | 1% low, was and now |
|---|---|---|---|---|---|
| hero | 164.8 | 186.5 | 234.1 | 1.26× | 148, 159 |
| mailbox | 161.4 | 191.3 | 232.6 | 1.22× | 146, 163 |
| porch | 202.0 | 232.0 | 236.5 | 1.02× | 158, 165 |
| truck | 198.3 | 219.5 | 226.7 | 1.03× | 155, 166 |
| windmill | 225.4 | 246.1 | 256.8 | 1.04× | 173, 182 |
| crane | 170.3 | 192.9 | 248.8 | 1.29× | 146, 157 |

"As it was" is this build with the three switches set, which draws the original's pictures exactly; the runs' ranges are within 1.1% of their medians (the windmill's "now" 242.5 to 247.2). Unity's column is 2026-11-25's.

GPU time, ms (the scene pass, then its parts):

| View | Scene pass, was | Now | Items | Scattered items | Terrain | Cut-outs' depth | GPU frame, was and now | Frame's thread, was and now |
|---|---|---|---|---|---|---|---|---|
| hero | 3.27 | 2.57 | 0.86 → 0.55 | 0.93 → 0.80 | 0.90 → 0.66 | 0.14 → 0.11 | 5.96, 5.25 | 4.41, 4.16 |
| mailbox | 3.52 | 2.58 | 1.01 → 0.59 | 0.70 → 0.59 | 1.15 → 0.77 | 0.14 → 0.10 | 6.10, 5.14 | 4.17, 3.85 |
| porch | 2.08 | 1.45 | 1.27 → 0.69 | 0.52 → 0.50 | 0.06 → 0.06 | 0.14 → 0.09 | 4.85, 4.20 | 4.23, 4.00 |
| truck | 2.30 | 1.83 | 0.95 → 0.55 | 0.66 → 0.62 | 0.46 → 0.46 | 0.12 → 0.10 | 4.93, 4.45 | 4.57, 4.25 |
| windmill | 1.72 | 1.38 | 0.43 → 0.35 | 1.01 → 0.74 | 0.02 → 0.02 | 0.18 → 0.16 | 4.13, 3.72 | 4.30, 3.92 |
| crane | 3.14 | 2.51 | 0.70 → 0.51 | 1.24 → 0.97 | 0.92 → 0.73 | 0.14 → 0.14 | 5.76, 5.09 | 5.11, 4.71 |

The process's video memory and memory are unchanged (2.79 to 2.92 GB and 2.49 GB in both). The depth start is one full-screen pass writing depth; its cost is inside the scene pass's numbers above.

**The duplicate cut-out depth is not gone.** The scene pass still draws the cut-outs' depth (0.09 to 0.16 ms now, since most of what it draws fails the test early). Dropping it means lighting cut-outs wherever their depth *equals* the prepass's, which was built and measured first: every solid item lit where depth is equal, no fill, no `discard` anywhere. It was the fastest (hero 207.8 frames a second, scene pass 1.98 ms) and it is not in, because it does not draw the same picture: where two surfaces land on exactly the same depth value, "equal" lets both through and the last drawn shows, where "nearer than" shows the first. That was 0.14 to 1.2% of the pixels of a still (far mountains and foliage, where one step of the depth buffer is a stud or more, and a few props with doubled faces). It cannot be mended by drawing in the opposite order, since the ties are also between triangles of one mesh.

### The order of the draws: built, measured, off until the owner has looked

The scattered items were still 0.60 ms where the depth-equal experiment had them at 0.35, with the same pixels lit. The difference turned out to be **changes of pipeline between small draws**. Solid items are sorted by distance band, part, then mesh, so a band's plants alternate between the pipelines of cut-outs, solid parts and fading ones mesh by mesh, several hundred times a frame. Sorted by pipeline before mesh within each band and part (`pipeline_rank` in `renderer/lighting.rs`), the same draws cost the GPU 0.25 to 0.5 ms less. The frame's thread gains little: its encoding is 0.04 to 0.08 ms shorter over three runs (a first single run had suggested 0.15), and the 110 to 140 more draw calls the order makes take most of that back:

| View | Now | With the pipeline order | Scattered items, ms | Scene pass, ms |
|---|---|---|---|---|
| hero | 186.5 | 205.4 | 0.80 → 0.38 | 2.57 → 2.11 |
| mailbox | 191.3 | 205.2 | 0.59 → 0.29 | 2.58 → 2.23 |
| porch | 232.0 | 240.5 | 0.50 → 0.26 | 1.45 → 1.19 |
| truck | 219.5 | 224.9 | 0.62 → 0.32 | 1.83 → 1.47 |
| windmill | 246.1 | 245.0 (the thread's limit) | 0.74 → 0.43 | 1.38 → 1.06 |
| crane | 192.9 | 206.1 | 0.97 → 0.51 | 2.51 → 2.00 |

It is **off unless `AXCENITY_PIPELINE_ORDER=1` is set**, because it does not pass the owner's rule as it stands: any change to the order solid items are drawn in changes which of two surfaces shows where both land on exactly the same depth value. Against the original's stills:

| View | Pixels that differ | More than 4 of 255 apart | More than 8 apart | Largest difference | Mean difference of a channel |
|---|---|---|---|---|---|
| hero | 744 (0.036%) | 0.012% | 0.006% | 51 | 0.0013 |
| mailbox | 369 (0.018%) | 0.006% | 0.003% | 43 | 0.0007 |
| porch | 64 (0.003%) | 0.001% | 0.001% | 39 | 0.0002 |
| truck | 158 (0.008%) | 0.002% | 0.001% | 38 | 0.0003 |
| windmill | 355 (0.017%) | 0.003% | 0.001% | 30 | 0.0003 |
| crane | 4,623 (0.22%) | 0.073% | 0.031% | 46 | 0.0074 |

They are single pixels scattered through distant foliage (`target/cross-engine/homestead/scene-depth/crane-pipeline-order-compare.png` and `hero-…`: the original, the new order, their difference eight times over; the stills are in `pipeline-order`). Neither picture is the more correct one there: the depth buffer cannot tell the two surfaces apart, and which shows was always decided by the order they happened to be drawn in. The sort by part that step 2 added moved the same kind of pixel. For scale: in each view compared both times the mean difference is smaller than the traced clouds' was (hero 0.0013 against 0.040, mailbox 0.0007 against 0.006, windmill 0.0003 against 0.143, crane 0.0074 against 0.025), but its single largest differences are larger and, in the hero view, more of its pixels are over 8 apart (0.006% against 0.003%). **The decision is the owner's**: with it on, Unity's frame rate is 0.98 to 1.21 times the engine's across the six views (1.02 to 1.29 without).

What would remove the question is a depth buffer that can tell far surfaces apart (reversed depth: the same 32 bits, spread evenly over distance). Then the two paths would agree, and so would "equal", which would let the cut-outs' second depth go as well. It touches every pass that reads depth, so it is its own piece of work, not part of this.

### Two things learned about measuring

- A split that looked free was not. Giving fading items batches of their own *in the draws' order* (to keep the picture exact) took the hero view from 3,532 to 5,534 draw calls, the plants' scene time from 1.04 to 1.63 ms and the thread from 4.37 to 5.20 ms: slower than doing nothing. A batch with a fading item now simply keeps the old shader.
- Per-draw counts are now in `--bench`'s counts (`prepass draws`, `motion draws`, `depth fill draws`, `scattered batches`, `scattered items`, `depth from prepass`).

## Step 3, started: the frame's thread (2026-11-26)

With the scene pass shorter, the thread is the limit in the windmill view (3.92 ms against the GPU's 3.72) and within 10% of the GPU's frame in the porch, truck and crane views; in the hero and mailbox views the GPU's frame is still a millisecond longer (5.25 against 4.16).

**Where the plants' thread time is** (hero view, the thread's sections with and without the painted foliage, from the earlier take-away run): levels of detail 0.25 against 0.02 ms, materials 0.55 against 0.32, uploads 0.42 against 0.39, culling and batches 0.50 against 0.20, motion history 0.46 against 0.02, instances written 0.10 against 0.01, recording 0.32 against 0.20, encoding 0.85 against 0.58. Two things in that were not expected: **uploads cost 0.4 ms a frame with nothing to upload and no plants**, and materials 0.32 ms for 577 items. Neither is the plants', and both are on the thread every frame.

**The worker-thread candidate, first piece: the motion history.** It was the largest section only the plants pay: one thread built a 150-byte motion instance for every swaying plant the frame knew of, including those only a shadow view reaches, which the motion pass can never draw (it draws where the prepass did, and the prepass draws what the camera sees). Now the renderer's threads share the work and only the camera's draws are kept (`History::moved`, `DrawPlan::in_view`). Same build, both paths, 1920 × 1080:

| View | Motion history, ms | Frame's thread, ms | Motion draws | Frames a second |
|---|---|---|---|---|
| hero | 0.49 → 0.21 | 4.40 → 4.12 | 70 → 65 | 186.1 → 188.5 (the GPU's limit) |
| truck | 0.56 → 0.24 | 4.61 → 4.25 | 75 → 57 | 210.9 → 219.2 |
| windmill | 0.58 → 0.24 | 4.29 → 3.95 | 92 → 69 | 225.9 → 245.6 |

The picture is unchanged (0 pixels differ in the six stills, with the two changes above). One trap on the way: putting the instances in mesh order by sorting them moved 150 bytes at every swap and cost more than the sharing saved (0.79 ms on one thread); their places are sorted now, and each instance is copied once.

**What the candidate has left**, by the sections above: the materials' loop over grouped draws (one thread, a hash look-up a draw), culling and batches (shared already; 0.3 ms of it is the plants'), levels of detail (shared; a lock an item), and the 0.4 ms of uploads that is not the plants' at all. At the motion history's rate these are worth perhaps 0.5 to 0.8 ms together: an estimate, not a measurement.

**The GPU candidate is not built.** What was checked for it in wgpu 30's source, so the next step starts from facts:

- `RenderPass::multi_draw_indexed_indirect_count` needs `Features::MULTI_DRAW_INDIRECT_COUNT` (DirectX 12, and Vulkan 1.2 or `VK_KHR_draw_indirect_count`); no device in the engine asks for it yet.
- A draw's first instance in an indirect buffer needs `Features::INDIRECT_FIRST_INSTANCE`. On DirectX 12, `@builtin(instance_index)` leaves the indirect buffer's first instance out unless `InstanceFlags::VALIDATION_INDIRECT_CALL` is on, which a release build turns off by default and which adds a validating pass over every indirect buffer. The engine draws with DirectX 12 on Windows, so a path that finds its instances from `instance_index` must either keep that validation on or hand each draw its offset another way.
- What it can save on the GPU is bounded by the probes above: the plants' corners and rasterizing are 0.18 to 0.27 ms a pass and carrying every value 0.06 ms. Its case is the thread's 1.5 ms, not the GPU's.
- What it must reproduce to draw the same picture: each plant's level of detail with its hysteresis and 30-frame cross-fade (state per plant, kept between frames), the fade near the cull distance, shadows to 35% of it, the sway in all four passes, and the batches' order, which decides the pixels at depth ties as the section above shows.

## Shadows through the holes, and the thread (2026-11-26, third session)

The owner's brief: the rasterized foliage shadows drawn as each card's whole outline, a shimmer he saw with ray traced shadows on, the two costs on the thread that are not the plants', the rest of the worker-thread candidate, and the draw order's decision. Same machine, settings and method; three runs a view, the median shown, the engine alone (Unity's column is 2026-11-25's).

### Foliage shadows follow the leaves

Not a regression from the scene pass's new shader: `plan_part` sent only hair cards through the shadow pass's cut-out pipeline, a cost decision of 2026-11-18 (step 5 of the plan). Every cut-out of the standard material goes through it now, and each shadow view draws its whole casters first and its cut-outs after, so the pipeline changes once a view and not mesh by mesh (worth 0.11 ms of the Shadows pass in the hero view, 0.12 at the windmill). `AXCENITY_NO_CUT_OUT_SHADOWS=1` casts whole cards as before.

The Shadows pass, ms, with the draw order below on in both columns:

| View | Whole cards | Through the holes | More | Frames a second, whole and through |
|---|---|---|---|---|
| hero | 1.06 | 1.19 | 0.13 | 206.0, 200.9 |
| mailbox | 1.14 | 1.30 | 0.16 | 206.7, 200.4 |
| porch | 1.37 | 1.51 | 0.14 | 249.4, 241.1 |
| truck | 1.12 | 1.29 | 0.17 | 241.7, 232.0 |
| windmill | 0.79 | 1.21 | 0.42 | 272.6, 254.2 |
| crane | 0.96 | 1.13 | 0.17 | 213.8, 206.4 |

It is the feature's price, paid in every cascade (the cut-out pipeline reads the texture and can `discard`, so depth is not tested first), and HDRP pays the same for the same thing. Reading the opacity map only where a material has one was tried and was worth 0.00 to 0.02 ms, so it is not in. If the windmill view's 0.42 ms is too much, the plan's other choices are still open: a quality setting, or the near cascades only.

**The picture** (`target/cross-engine/homestead/final/`: `as-before`, `shadows-only`, `default`, 1920 × 1080, TAA settled over 24 frames). The blocks under plants are gone and the plants themselves are lighter, since their own cards no longer shade them whole: 7 to 9% of the pixels change in the four views with open ground (hero, mailbox, windmill, crane), 1.2% at the truck, 0.1% at the porch; mean brightness rises by 0.8 to 2.7 levels in those four. Against a ray traced still of the hero view the rasterized shadows now have the same shape and weight (`target/cross-engine/homestead/shadows/`: `whole`, `cut-out`, `rt`). With the switch set the six stills are the earlier build's pixel for pixel.

### The shimmer with ray traced shadows: what was checked and what was found

The three causes the brief named:

1. **The ray traced scene only holds what the frame's views reach: true, and mended where it could show.** Items in groups (the painted plants) are passed over when no view reaches them, and the views are the camera's, the shadow maps' cascades and the lamps'. In the hero view 14,869 of the 42,357 items within the ray tracing distance are traced, and the count changes every frame of a turn. At the owner's settings the cascades reach as far as the rays, so nothing that shades the picture is missing: the six ray traced stills with the whole world traced differ from the usual ones only on plants a side effect touched (with 28,000 more candidates the bending budget ran out, and in five of the views 1,800 to 4,500 swaying plants were traced at rest). Where the maps are off or reach less far than the rays, a plant beside the screen cast no shadow into it until it was turned into view (`a_grouped_item_out_of_view_still_casts_its_ray_traced_shadow_into_it` failed). Such a frame now also keeps what the sun's view of the camera's view holds (`sun_reach`). The owner's ray traced stills are unchanged by it (0 pixels in five views, 8 pixels one level apart in the sixth).
2. **The cut-outs' alpha is read at level 0 already** (`textureSampleLevel` in `cut_out_covers`, ray_shadows.wgsl and ray_cut_outs.wgsl). Nothing to change.
3. **There is no history in the ray traced shadows.** They are traced afresh each frame, smoothed by two passes across and down, and TAA does the rest; nothing of theirs can trail or be rejected. Frame to frame the picture changes no more with them than with the shadow maps: 0.56 against 0.58 levels over the ground of the hero view standing still in the wind, 5.25 against 5.48 turning at 3 degrees a second.

What did turn out to depend on the shadow maps with ray tracing on: **the light that comes through a leaf from behind** (`through_thin`, `soft_sun_visibility`) is always read from the cascades. With whole-card maps that put card-shaped patches on the plants themselves, 2.0 to 2.5% of the pixels of the hero, mailbox and crane views (0.5 to 1.0% more than 8 levels apart, up to 97), tied to the cascades' texels, which move with the camera. With the maps following the leaves those pixels are lit through the holes. Their frame-to-frame change standing still fell a little (0.89 to 0.64% of them more than 8 levels apart from one frame to the next).

**Not reproduced**: a shimmer that only ray traced shadows have. What the stills and sequences did show, in both modes, and what may be what was seen: (a) painted plants cast no shadow past 35% of their cull distance (`SHADOW_REACH`), so in the crane view most mid-distance grass and bushes have none, and as the camera moves each plant's shadow appears or goes at once; ray traced shadows are sharp enough to show it. It is a rule both engines were given (README, "What is the same"), so changing it is the owner's choice. (b) Under TAA a moving frame is 4 to 5 levels from the settled one over the ground in both modes: the picture softens in motion. Neither was changed.

### The draw order is the default

The owner's brief: if `hero-pipeline-order-compare.png` and `crane-pipeline-order-compare.png` show no structural z-fighting, make the order the default. They show single pixels scattered through far foliage and no bands or patches on any surface, so solid draws are now sorted by pipeline before mesh, and `AXCENITY_PIPELINE_ORDER=0` puts the old order back. With whole-card shadows this build's stills in the new order are last session's `pipeline-order` stills pixel for pixel; with the shadows through the holes the two orders differ by 63 to 4,626 pixels a view, as before.

### The thread: two costs that were not the plants', and one more shared loop

Each was found by timing inside the section, then mended with the picture unchanged (this build with `AXCENITY_PIPELINE_ORDER=0` and `AXCENITY_NO_CUT_OUT_SHADOWS=1` draws last session's stills: 0 of 2,073,600 pixels differ in each of the six views).

- **Uploads, 0.42 ms with nothing to upload.** 0.33 ms of it was one loop: 1,280 of the list's 1,659 own items are pieces of the two terrains, and each piece made its terrain's layers ready again every frame (the texture arrays looked up, the uniform of eight layers worked out and copied to the heap, 255 ns a piece). A terrain's layers are now made ready once a frame, by its first piece (`TerrainPass::prepare`).
- **Materials, 0.32 ms for 577 items.** The frame has 1,479 distinct materials, since each terrain piece is one, and a material's key is 480 bytes. Joining the threads' shares went through a map of the keys themselves, which hashed and moved each key and did it all again each time the map doubled: 0.23 ms. Sized up front it was 0.14 ms; found by a digest that the threads work out with the keys (`MaterialPlaces`), 0.06 ms.
- **The grouped draws' materials** (the plants' 14,800 draws, 0.25 ms on one thread) are worked out in parts on the renderer's threads and joined in order, so a material's place is still where the draws first come to it (the draw order follows it; `thousands_of_grouped_items_get_the_bindings_they_would_listed_one_by_one`). Worth about 0.05 ms: less than hoped, the joining being most of what is left.

The thread's sections, hero view, ms (last session's table, then this build):

| | Before | Now |
|---|---|---|
| Levels of detail | 0.24 | 0.24 |
| Materials | 0.54 | 0.33 |
| Uploads | 0.41 | 0.20 |
| Culling and batches | 0.58 | 0.62 |
| Motion history | 0.23 | 0.21 |
| Encoding | 0.88 | 0.83 |
| **The frame's thread** | **4.16** | **3.74** |

**Culling and batches is not done.** Timed inside, its 0.6 ms is the draws' keys 0.06 (shared), splitting solid from see-through 0.09 (one thread), the sort of 16,500 keys 0.27 (shared already), the culling and batching itself 0.13 (shared) and the joining 0.04. Sorting the draws' indexes by their keys, in place of the keys, was tried for the sort and the split and was worth 0.015 ms, inside the noise, so it is not in. The new draw order makes 100 to 140 more batches a view, which is the 0.02 to 0.05 ms this section rose by. Levels of detail were not touched (shared already; the time is a lock an item). What is left of the worker-thread candidate is small, by these numbers about 0.2 ms; the larger sums on the thread are now the recording and encoding of 3,650 draws (1.15 ms), which only fewer draws shortens.

### Where that leaves the six views

Frames a second (1% low), 1920 × 1080, Ultra:

| View | Last session | This build, old order and whole-card shadows | New order, whole-card shadows | Now (new order, shadows through the holes) | Unity | Unity ÷ now | GPU frame, was and now | Frame's thread, was and now |
|---|---|---|---|---|---|---|---|---|
| hero | 186.5 (159) | 189.6 (175) | 206.0 (183) | 200.9 (189) | 234.1 | 1.17× | 5.25, 4.89 | 4.16, 3.74 |
| mailbox | 191.3 (166) | 192.8 (178) | 206.7 (191) | 200.4 (184) | 232.6 | 1.16× | 5.14, 4.91 | 3.83, 3.56 |
| porch | 232.0 (142) | 235.5 (211) | 249.4 (223) | 241.1 (207) | 236.5 | 0.98× | 4.22, 4.06 | 3.96, 3.57 |
| truck | 219.5 (166) | 222.6 (195) | 241.7 (214) | 232.0 (209) | 226.7 | 0.98× | 4.46, 4.23 | 4.22, 3.79 |
| windmill | 246.1 (182) | 264.4 (232) | 272.6 (244) | 254.2 (222) | 256.8 | 1.01× | 3.72, 3.85 | 3.92, 3.53 |
| crane | 192.9 (156) | 195.2 (179) | 213.8 (195) | 206.4 (188) | 248.8 | 1.21× | 5.08, 4.75 | 4.71, 4.23 |

The runs' ranges are within 0.5% of their medians (last session's 1% lows were measured with other work on the machine and are low for that). Every run was limited by the GPU. Memory and video memory are unchanged (2.48 GB; 2.85 to 2.93 GB).

Ray traced shadows and ambient occlusion (`owner-rt-shadows-ao.json`, one run each, this build with the two switches set and without): hero 90.9 → 93.2 frames a second, crane 89.6 → 91.7 (the scene pass 2.32 → 1.90 and 2.29 → 1.82 ms by the draw order, the Shadows pass 0.13 and 0.18 ms dearer).

**Not run**: the larger window, the tour, Unity again, DirectX 12 or another card, the other crates' tests and the regression check (`axcenity-bench-check`).

## Near cascades, where a plant's shadow ends, and TAA in motion (2026-11-26, fourth session)

The owner's brief: keep the cut-out shadows to the near cascades to win back the windmill view's 0.42 ms, stop the plants' shadows popping at 35% of their cull distance, and find whether TAA or the shadows themselves are behind the distortion he sees round plant shadows while the camera moves. Same machine, settings and method; three runs a view, the median shown. `--bench` now times each of the sun's cascades (`Shadows: cascade 1` to `4` in `"gpu_parts"`).

### Cut-outs in the near cascades only: built, measured, left at every cascade

The windmill's 0.42 ms is not where the brief took it to be. By cascade (the windmill view, whole cards against through the holes, one run): the first cascade 0.14 → 0.29 ms, the second 0.18 → 0.31, the third 0.24 → 0.34, the fourth 0.22 → 0.25. So 0.28 of the 0.41 ms is in the two nearest cascades, where a plant covers the most texels and its holes are plainest, and those are the ones the brief keeps.

`ScenePipeline::cut_out_cascades` (`AXCENITY_CUT_OUT_CASCADES=0` to `4`) is how many of the nearest cascades cast through the holes; past them a cut-out casts whole, a hair card excepted. The Shadows pass, ms, and what the stills show against every cascade (1920 × 1080, TAA settled over 24 frames):

| View | Every cascade | Nearest three | Nearest two | Pixels that change with two | More than 16 of 255 apart |
|---|---|---|---|---|---|
| hero | 1.20 | 1.17 | 1.10 | 2.1% | 0.58% |
| mailbox | 1.30 | 1.27 | 1.21 | 1.1% | 0.27% |
| porch | 1.51 | 1.49 | 1.42 | 0.1% | 0.04% |
| truck | 1.30 | 1.26 | 1.18 | 0.5% | 0.19% |
| windmill | 1.22 | 1.19 | 1.08 | 1.6% | 0.33% |
| crane | 1.13 | 1.11 | 1.04 | 4.4% | 1.5% |

With two, the frame rate is 4 to 10 frames a second higher (windmill 252.9 → 263.2) and the picture loses what the third session added from 51 studs out: blocks under the mid-distance plants of the crane view, and the hero view's junipers shading themselves whole (`target/cross-engine/homestead/step2/crane-cascades-compare.png`, `hero-cascades-compare.png`). With three, 0.02 to 0.04 ms and 0.02 to 0.75% of pixels. The premise that the far cascades' texels are too coarse for a leaf's holes does not hold here: the tour's lenses are long (30° to 46°), so in the hero view the third cascade's texel is about 0.04 studs and the fourth's 0.10 (worked out from the cascades' sizes, not read back). **It is left at every cascade**, which draws the third session's stills pixel for pixel; two is one constant away if the owner wants the 0.1 ms more than the mid-distance shadows.

### A plant's shadow thins out before it ends

Painted plants cast no shadow past 35% of their cull distance, all at once. Now the shadow is whole to 35% and thins to nothing at 45% (`ShadowReach` in `renderer/lod.rs`; [docs/rendering/shadows.md](../../../docs/rendering/shadows.md)): dithered in the shadow maps, ray by ray in ray traced shadows (an item without holes, whose hits a ray cannot pass over, still goes at once there, half way through the band).

**The pop, measured.** Two settled stills of the crane view with the line moved by 1% of the cull distance, which is what the camera moving that much nearer does to the shadows and to nothing else (`target/cross-engine/homestead/step3/crane-shadow-pop-compare.png`):

| | Pixels more than 8 levels apart | More than 16 | Largest change |
|---|---|---|---|
| Shadow maps, sudden end | 0.027% | 0.020% | 99 |
| Shadow maps, thinning | 0.003% | 0.000% | 25 |
| Ray traced, sudden end | 0.027% | 0.016% | 95 |
| Ray traced, thinning | 0.001% | 0.000% | 29 |

**What reach costs** (the Shadows pass against 35% with a sudden end, one run a view): to 50% with no fade 0.10 to 0.13 ms; to 70% 0.22 to 0.27 ms in five views, and in the crane view the frame plans over 12,000 casters there, so its far cascades start taking turns; whole to 25% thinning to 35%, 0.00 to 0.04 ms (and less shadow than before); whole to 30% thinning to 40%, 0.03 to 0.06 ms; whole to 35% thinning to 45%, 0.08 to 0.12 ms. The last is in, since it takes no shadow away that was there. The Unity port still ends its shadows at 35% (README, "What differs").

### TAA in motion: what was measured

Sequences of 90 moving frames and 50 held, with the wind stopped so only the camera moves: a sideways walk of 0.2 studs a frame and a turn of 0.4° a frame, from the hero and the crane views (`target/cross-engine/homestead/step3/motion/`). "Apart" is the last moving frame against the settled frame of the same view, in levels of 255 (mean; the share of pixels more than 8 apart).

| Drawn with | hero, walk | hero, turn | crane, walk | crane, turn | Still, frame to frame |
|---|---|---|---|---|---|
| Shadow maps, no anti-aliasing | 0.01 | 0.00 | 0.00 | 0.00 | 0.001 |
| Ray traced shadows, no anti-aliasing | 0.01 | 0.00 | 0.00 | 0.00 | 0.001 |
| Shadow maps, TAA as it was | 2.01 (5.6%) | 3.57 (12.9%) | 1.94 (5.0%) | 3.62 (12.7%) | 0.30 to 0.57 |
| Ray traced shadows, TAA as it was | 1.90 (5.1%) | 3.34 (11.8%) | 1.87 (4.6%) | 3.32 (11.0%) | 0.29 to 0.54 |
| Ray traced, history read through the spline | 1.56 (3.8%) | 2.26 (6.0%) | 1.44 (3.2%) | 2.27 (5.7%) | 0.29 to 0.54 |
| Ray traced, history held to one spread of the mean | 1.68 (4.2%) | 2.95 (9.5%) | 1.64 (3.6%) | 2.92 (8.7%) | 0.33 to 0.60 |
| Ray traced, each pixel's own motion | 1.95 (5.3%) | 3.34 (11.8%) | 1.90 (4.8%) | 3.31 (10.9%) | 0.29 to 0.54 |
| Ray traced, the spline and the spread together | 1.38 (3.2%) | 1.98 (4.6%) | 1.27 (2.6%) | 1.98 (4.4%) | 0.33 to 0.60 |
| Ray traced, this frame's share 0.2 | 1.56 (3.7%) | 2.56 (7.5%) | 1.57 (3.4%) | 2.58 (7.1%) | 0.58 to 1.11 |
| The editor's settings at 998 × 405, as it was | 1.99 (5.9%) | 3.58 (12.3%) | 1.97 (5.5%) | 3.64 (12.8%) | 0.35 to 0.61 |
| The editor's settings at 998 × 405, the spline | 1.70 (4.8%) | 2.44 (6.5%) | 1.57 (4.0%) | 2.51 (7.0%) | 0.35 to 0.61 |

What that says:

1. **The shadows are steady under a moving camera, both kinds.** Without anti-aliasing a moving frame is the settled frame, to a hundredth of a level. Nothing in the shadow maps or in the ray traced shadows trails, warps or lags when only the camera moves.
2. **All of what changes in motion is TAA, and it is the same with either kind of shadow.** It is a softening of the whole picture, the ground's texture and the plants' edges alike, not something round shadows (`hero-turn-taa-compare.png`, `crane-turn-taa-compare.png` in the motion folder).
3. **Its largest part was how last frame's picture was read.** Between pixels it was a plain blend of four, which blurs a little more each frame. Read through the spline that upscaling TAA already used, a turning frame is a third nearer the settled one, for 0.01 ms. This is in (`fs_taa`, [docs/rendering/temporal.md](../../../docs/rendering/temporal.md)); a still picture changes by one level in 1.3 to 1.5% of its pixels.
4. **Variance clipping helps a little and costs steadiness**: a tenth less softening, a tenth more flicker standing still. Not in. A larger share for this frame doubles the flicker. Taking a pixel's own motion changes nothing: the nearest surface's motion is not dragging the ground along.
5. **The editor's viewport and the player differ in size and pace, not in TAA.** One pipeline, the same eight jitter places, the same motion vectors. The editor's saved layout draws 998 × 405 pixels, so a blade of grass and its shadow are under a pixel wide sooner (a tenth more flicker standing still above); an editor nobody touches draws swaying scenery at 30 frames a second, where TAA's blend takes three times as long to follow a swaying shadow as at 90; and the owner's editor settings add real-time GI and ray traced reflections.
6. **Ray traced shadows of swaying plants are not left behind here.** In every sequence with the wind on, at both sizes, the ray traced scene bent every plant whose sway shows (470 to 3,500 a frame, none left out for the budget, none waiting for a build).

**Still not reproduced: a shimmer only ray traced shadows have.** What was found and mended is in both modes: the pop at the end of a plant's shadow, and the softening in motion. If something is left after these, the next things to look at are a swaying plant's shadow on still ground (the ground's motion is the camera's alone, so TAA holds the old shadow to the colors round the pixel and follows it over about ten frames) and the editor's 30 frames a second while idle.

### Where that leaves the six views

Frames a second (1% low), 1920 × 1080, Ultra. "As it was" is this build with `AXCENITY_SHADOW_REACH=0.35,0` and `AXCENITY_TAA_PROBE=soft`, which draws the third session's stills pixel for pixel (0 of 2,073,600 in each view).

| View | As it was | Now | Unity | Unity ÷ now | Shadows pass, was and now | Frame's thread, was and now |
|---|---|---|---|---|---|---|
| hero | 199.4 (188) | 196.1 (185) | 234.1 | 1.19× | 1.19, 1.27 | 3.71, 3.71 |
| mailbox | 198.6 (182) | 195.6 (183) | 232.6 | 1.19× | 1.30, 1.37 | 3.46, 3.51 |
| porch | 238.4 (207) | 233.3 (212) | 236.5 | 1.01× | 1.51, 1.62 | 3.56, 3.56 |
| truck | 229.3 (199) | 223.9 (201) | 226.7 | 1.01× | 1.30, 1.41 | 3.79, 3.82 |
| windmill | 251.6 (228) | 245.5 (221) | 256.8 | 1.05× | 1.22, 1.32 | 3.45, 3.56 |
| crane | 204.9 (186) | 201.6 (187) | 248.8 | 1.23× | 1.13, 1.22 | 4.17, 4.31 |

Ray traced shadows and ambient occlusion (`owner-rt-shadows-ao.json`), the same two builds:

| View | As it was | Now | Ray traced shadows pass, was and now | Frame's thread, was and now |
|---|---|---|---|---|
| hero | 92.7 (89) | 91.9 (88) | 2.18, 2.25 | 6.45, 6.42 |
| mailbox | 84.1 (80) | 84.5 (81) | 2.62, 2.54 | 6.22, 6.16 |
| porch | 109.3 (104) | 107.8 (103) | 2.15, 2.16 | 6.57, 6.61 |
| truck | 116.1 (109) | 116.8 (110) | 1.54, 1.48 | 6.71, 6.76 |
| windmill | 92.2 (88) | 90.8 (86) | 3.01, 3.03 | 6.71, 6.79 |
| crane | 90.9 (87) | 87.5 (84) | 2.15, 2.35 | 8.32, 8.60 |

The runs' ranges are within 1% of their medians, one run excepted (the hero's third "as it was", 194.8). Every run was limited by the GPU. All of the cost is the longer reach: 1,700 to 2,400 more caster draws a view. **The crane view with ray traced shadows goes under the owner's 90**, from 90.9 to 87.5: the price of the fade as built, and whole to 30% thinning to 40% would be about half of it.

**Not run**: the larger window, the tour, Unity again (its port still ends shadows at 35%), DirectX 12 or another card, the regression check (`axcenity-bench-check`). Tests run: `cargo test -p axcenity-render` (507 passed), the `ui` and `core` crates' library tests, `cargo clippy --workspace --all-targets -- -D warnings`.

## The idle editor, a swaying shadow on still ground, and the camera changing pace (2026-11-26, fifth session)

The owner's brief: check how the editor's 30 frames a second while idle meets camera movement, wind and every ray traced effect at once (ray traced shadows, ambient occlusion, reflections, real-time GI) at the editor's 998 × 405; make sure input wakes it without a late frame, which would show as a jump in what TAA has gathered; and find whether swaying plants' shadows on still ground trail or warp while standing in the wind, before changing any filter. Same machine. Pictures are from the still tool at the editor's saved graphics settings (`target/cross-engine/homestead/step5/graphics/editor.json`), 998 × 405; the editor itself was timed in its debug build (the one the owner runs) on a copy of the project with scratch settings.

### The editor has no 30 frames a second to wake from

`AXCENITY_WAKE_LOG=3` (new; [docs/benchmarks.md](../../../docs/benchmarks.md)) prints every frame of an untouched editor, how long after the last it came and which lines of code asked for it, and every three seconds wakes the editor as an input event would.

| The project copy | Frames while nobody touches it | Asked for by | A woken frame, after it was asked for |
|---|---|---|---|
| As it is | 18.4 ms apart (54 a second; 1 in 100 over 20.4, the longest 24.5, of 2,056) | the particles, every frame (`viewport.rs`, "Particles keep moving"), and the scenery's timer | median 6.8 ms, at most 18.0 |
| Its two dust emitters removed | 18.3 ms apart (the longest 22.5 of 2,605) | the scenery's timer alone | median 4.6 ms, at most 14.8 |
| And the wind stopped | 15.9 ms apart | the scenery's timer (the clouds drift) | 0.0 to 13.8 ms |

1. **The pace meant to be 30 a second is 55 to 60.** `viewport::shader_time` asks for the next frame a thirtieth of a second on; egui takes its guess at a frame's length, a sixtieth, off every such wait (`Context::request_repaint_after`), so the frame comes 16.7 ms after it was asked for. In the debug editor Desert Homestead with every ray traced effect draws in 18.3 ms, so the editor simply draws as fast as it can, touched or not.
2. **Particles ask for every frame**, so a project with an emitter in view never had the slower pace even in intent.
3. **Input is taken by the next frame.** A woken frame came a median 5 to 7 ms after it was asked for and never more than one frame; the frame it woke was as long as any other (18.0 to 20.8 ms). eframe replaces a pending timed repaint with one now when a window event arrives (`handle_event_result`), and `keep_frames_steady` counts any event, a held key or a held button as input. A view cube swing and a held fly key ask for the next frame themselves. Nothing here arrives a frame or two late.
4. **Were the pace really 30, the picture would not jump on waking.** Two sequences, standing still in the wind at a thirtieth and at a ninetieth of a second a frame and then turning at a ninetieth, with the same clock, camera and jitter from the first moving frame on, are 0.80 levels apart before the turn and 0.84 at its first frame (the clouds, which gather over frames, are all of it).

Not changed: the pace itself. Making it a real 30 would spare the GPU as the code means to, and coarsen the sway of an untouched editor; it is in [the wiki's limitations](../../../wiki/Roadmap/Known-Limitations.md) for the owner to decide.

### A swaying shadow on still ground does not trail

Standing still for 150 frames with the wind on, then the clock stopped for 50: the last windy frame against the settled picture of the same moment, in levels of 255 (mean; the share of pixels more than 8 apart). "Floor" is two settled frames against each other, which TAA's jitter alone makes differ. "Ground" is the pixels no plant ever covers (from a sequence drawn without shadows or anti-aliasing), so only light and shadow can change there.

| Drawn with | View | Whole picture, 30 a second | 90 a second | Floor | Ground, 30 a second | 90 a second | Floor |
|---|---|---|---|---|---|---|---|
| Every ray traced effect, no anti-aliasing | all three | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 |
| Every ray traced effect, TAA | hero | 1.14 (1.2%) | 0.99 (0.9%) | 0.81 (0.4%) | 1.02 (0.50%) | 1.00 (0.49%) | 0.91 (0.28%) |
| | crane | 1.16 (1.0%) | 1.08 (0.9%) | 0.87 (0.3%) | 0.86 (0.27%) | 0.86 (0.26%) | 0.81 (0.14%) |
| | a bush from 10 studs | 1.15 (0.9%) | 1.09 (0.8%) | 0.95 (0.3%) | 0.97 (0.04%) | 0.96 (0.04%) | 0.91 (0.03%) |
| Shadow maps, TAA | hero | 1.19 (1.5%) | 1.03 (1.2%) | 0.84 (0.4%) | 1.05 (0.73%) | 1.03 (0.71%) | 0.94 (0.34%) |
| | crane | 1.22 (1.3%) | 1.13 (1.2%) | 0.91 (0.3%) | 0.92 (0.55%) | 0.91 (0.53%) | 0.84 (0.18%) |
| | a bush from 10 studs | 1.20 (1.3%) | 1.13 (1.0%) | 0.98 (0.4%) | 0.99 (0.17%) | 0.98 (0.13%) | 0.93 (0.05%) |
| Every ray traced effect, TAA, four times the wind | a bush from 10 studs | | | | 1.05 (0.22%) | 1.00 (0.09%) | 0.91 (0.03%) |
| Shadow maps, TAA, four times the wind | a bush from 10 studs | | | | 1.07 (0.38%) | 1.01 (0.23%) | 0.92 (0.04%) |

1. **Nothing in the ray traced effects keeps anything from one frame to the next.** Without anti-aliasing a windy frame is the settled picture of its moment to one level, with ray traced shadows, ambient occlusion, reflections and GI all on. There is no filter over time in them to trail (the fog's and the clouds' gathering apart); what follows the wind late can only be TAA.
2. **On the ground TAA is within a tenth of a level of its own floor**, at either pace, and the ray traced picture is nearer its floor than the shadow maps' is. Two to three in a thousand ground pixels are more than 8 levels off for the wind's sake, and they sit round the plants' feet and thin stems, not in the shadows (`step5/editor-hero-30-where.png`, `bush-w4-worst-ground.png`).
3. **Only at four times the wind does the pace show**: 0.22% of ground pixels at 30 a second against 0.09% at 90, none over 41 levels. That is TAA following a shadow edge over its ten frames, and too little to see.
4. The settings' real-time GI changed no pixel of these views, and ray traced reflections 0.2% of them by at most 6 levels: the picture is its shadows and ambient occlusion.

So no change to any filter for this. The warping the owner sees round plant shadows is not the shadows trailing while he stands still.

### The camera changing pace threw TAA's history away: mended

Found while measuring the wake. TAA drops a pixel's history where its motion differs from last frame's by more than two pixels, meant for what a moving part uncovers. A camera that starts to turn, stops, or turns by a different amount each frame changes every pixel's motion at once, so the whole picture started again from one frame as drawn: no anti-aliasing, and every per-pixel noise bare (ray traced shadows' and ambient occlusion's lean on TAA most). The sequences of the fourth session turned by exactly the same amount every frame, which is the one motion that never showed it.

Sharpness of each frame (the mean size of the Laplacian below the sky; the settled picture reads 20, the picture without anti-aliasing 26.5), the bush view turning 0.4° a frame, every ray traced effect on:

| Sequence | As it was | Now |
|---|---|---|
| A steady turn | 11.5 to 11.9 every frame | the same (within 0.02 levels a pixel) |
| The first frames of a turn, from standing still | 26.8, 20.5, 17.8, 16.3, 15.0 … 11.7 after twenty | 16.8, 15.6, 14.4, 13.9, 13.4 … 11.7 |
| One frame three times as long, in a steady turn | 27.4, 27.4, 20.6, 18.0, 16.1 … fourteen frames to settle | 11.7, 11.8, 11.6: no mark |
| Each step 40% uneven (a hand on a mouse) | 20 to 25 every frame, changing frame to frame | 11.3 to 11.8 every frame |

Shadow maps read within 1 of these in every row: this was in both kinds of shadow. The owner's editor draws Desert Homestead at 54 frames a second with 1 frame in 100 over a tenth longer than the rest, under a mouse; it was in the uneven row whenever he turned. **This is the likeliest source of the shimmer he sees in motion**, and of its being worse with ray traced shadows, whose one ray a pixel is noise until TAA has gathered it. It is for him to say whether it is gone.

The mend (`camera_change` in temporal.wgsl; [docs/rendering/temporal.md](../../../docs/rendering/temporal.md)): the camera's own change of a still surface's motion, from this frame's view and the two before it, is taken out of the difference. With the camera still or keeping its pace it is nothing, so a still view is the same pixel for pixel. `AXCENITY_TAA_PROBE=jumpy` is the rule as it was (the same pictures exactly, 0 of 404,190 pixels in nine frames compared). The plain-wall ghost tests turned out to pass with the rule removed altogether, so `taa_drops_the_history_of_what_a_moving_part_uncovers` was added first: a grey part over stripes, 12 levels off the stripes alone with the camera still and 15 with it turning unevenly, 37 with the history never dropped.

**What it costs**: 0.004 ms. Post-processing, which TAA is timed in, is 0.179 ms in the hero view and 0.184 in the crane view at 1920 × 1080 with it and 0.175 and 0.180 with `jumpy`, two runs each on the one build; the frame rates are 194.9 and 200.2 against 194.6 and 199.7, inside what two runs differ by.

**Not run**: the other four views, the tour, upscaling TAA in Desert Homestead (its tests pass), a release editor, another machine. Tests run: `cargo test -p axcenity-render` (510 passed), `cargo clippy --workspace --all-targets -- -D warnings`. The editor crate's tests were not run.

## Shadow views by mesh, lamp faces kept, and the porch lamp's lantern (2026-11-26, seventh session)

The sixth session's diagnosis of the owner's Profiler reading at dusk (10,607 draw calls, Shadows 4.10 ms, "z-fighting" on the front door) found three engine causes; the owner asked for all three to be fixed, with the picture kept where it should be. Everything here is from the still tool (`crate-s4`, `SHOT_COUNTS=1`) at 1920 × 1080, the owner's Ultra with TAA (`owner-ultra.json`; ray traced: `owner-rt-all.json`), the eight views of `step6/door-maps` (front, oblique, grazing and yard, by day and at dusk with the six lamps lit as `homestead.server.lua` lights them). Both builds drew a frozen copy of the project (the owner was saving it meanwhile, which made the first comparison wrong); "before" is the same engine built from a worktree at `7956a6b7` with the fifth session's changes. GPU and CPU times are one run each, the CPU's the mean of frames 8 to 31.

### What was changed

- **Each shadow view draws a mesh once** (`lighting::casters_in_views`): it sorts its casters by the material it draws them through and their mesh (one packed key, `caster_order`), not by the scene's order, which starts with the camera's distance band.
- **A lamp's face is drawn again only where it changed** (`renderer/lamp_shadows.rs`): its still casters' fingerprint is kept with its layer, and the same fingerprint keeps the layer. Swaying plants (the world's wind is 4.5, 0, −2.5) and posed casters move every frame: they're drawn every frame over a copy of the still casters, kept in a second depth array (100 MB at Ultra). Without that split only 5 of the 24 faces were kept, since every lamp reaches plants.
- **`ShadowStart`** on PointLight, SpotLight and RectLight: studs from the light before anything casts its shadows, for a light inside a fixture that's part of a larger mesh.

### Draws, triangles and time

The same pixels before and after in all sixteen stills (maps and ray traced), and with `AXCENITY_NO_LAMP_SHADOW_KEEP=1`.

| View | Draw calls | Shadow views' draws | Of those, lamp faces' after (faces kept of 24; drawn every frame, without keeping) | Triangles | Shadows (GPU) | Frame's thread (CPU) |
|---|---|---|---|---|---|---|
| Front, day | 7,826 → 3,745 | 5,776 → 1,695 | none | 13.0 M | 3.32 → 3.27 ms | 7.01 → 6.46 ms |
| Oblique, day | 6,957 → 3,701 | 4,751 → 1,495 | none | 10.5 M | 3.21 → 3.18 | 6.33 → 6.18 |
| Grazing, day | 7,027 → 3,725 | 4,830 → 1,528 | none | 10.9 M | 3.25 → 3.21 | 6.30 → 6.24 |
| Yard, day | 7,581 → 3,885 | 5,382 → 1,686 | none | 12.5 M | 3.05 → 3.03 | 6.92 → 6.73 |
| Front, dusk | 10,019 → 4,101 | 7,980 → 2,062 | 246 (24; 2,138) | 18.5 → 12.7 M | 4.80 → 4.02 | 8.28 → 6.71 |
| Oblique, dusk | 11,957 → 4,613 | 9,754 → 2,410 | 251 (24; 2,143) | 18.6 → 12.7 M | 5.31 → 4.18 | 9.36 → 7.84 |
| Grazing, dusk | 11,940 → 4,611 | 9,746 → 2,417 | 257 (24; 2,154) | 18.8 → 13.0 M | 5.34 → 4.28 | 9.21 → 7.92 |
| Yard, dusk | 9,692 → 4,266 | 7,496 → 2,070 | 251 (24; 2,046) | 17.9 → 12.4 M | 4.26 → 3.31 | 7.69 → 7.44 |

- The sorting itself costs the frame's thread 0.1 to 0.2 ms more in the `Casters` section (0.11–0.27 → 0.23–0.46 ms; a tuple key was 0.34–0.70 ms), and the passes' recording and encoding save more than that.
- Fewer draws save little GPU time by day (0.02 to 0.05 ms): the cascades' time is their triangles, not their draws. At dusk the lamps' faces save 0.78 to 1.12 ms of the Shadows pass, not the 1.4 to 1.5 ms all of them took: the swaying plants are still drawn into every face each frame, after a copy of 24 layers. With every ray traced effect on, the frame's GPU time at dusk is 25.9 → 25.2 ms (front) and 34.3 → 32.6 ms (oblique).

### The porch lamp's lantern

`PorchLight` sits 0.022 studs below the top of its lantern's brass, which is in `House.glb`'s one mesh with the door, so no part's reach can keep it out of the light's shadows: the brass reaches 0.29 studs above the light, 0.76 below and 0.2 to the sides. With `ShadowStart` set on the light in the shots (the project itself isn't changed), a lit plank of the door in the front and oblique views at dusk:

| `ShadowStart` | Shadow maps: brightness, pixel-to-pixel variation | Ray traced: brightness, variation |
|---|---|---|
| 0 (as now) | 15.1, 13.1% (the lantern's shadow across the door) | 32.6, 10.9% |
| 0.35 | 48.7, 8.8% | 48.7, 8.7% |
| 0.8 | 49.0, 8.7% | 48.9, 8.7% |

0.35 takes the lantern's shadow off the door and makes the ray traced door match the maps; 0.8 changes at most 0.03% of a view's pixels more. What's nearer the light than that no longer shades anything from it, so the lantern's cap no longer shades the porch ceiling above it. Crops: `target/cross-engine/homestead/step7/door-*-shadow-start.png`. The plank gaps are the asset's (left to the owner).

## What these numbers don't cover

- One scene, one machine, one driver. A scene with many lights, characters or indoor views would exercise other parts of both engines.
- Unity's plants don't sway, and its shadows follow cut-outs. The first saves it work and the second costs it. The no-wind run above bounds the first.
- HDRP is set to match the engine's settings feature by feature, not tuned by a Unity specialist, and the engine's levels of detail and cull rules are imposed on it. A project built for Unity would use its terrain's own trees and details, and would look and run differently.
- The ablations and experiments are one run each (the three-run tables' ranges are within 2%).
- The pictures are compared by eye and by mean brightness, not pixel by pixel.
