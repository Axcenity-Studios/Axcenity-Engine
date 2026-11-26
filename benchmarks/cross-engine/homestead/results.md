# Desert Homestead: Axcenity against Unity HDRP, results

Measured 2026-11-25 on the owner's desktop: Ryzen 9 9950X3D, Radeon RX 9070 XT (16 GB), Fedora 45, kernel 7.3.4, Mesa 26.2.7 (RADV), GNOME on Wayland. What is compared and how: [README.md](README.md).

- **Axcenity**: `target/release/axcenity-player` built from main at a9370a3b with the working tree's changes, `benchmarks/features/owner-ultra.json` (Ultra, shadows Ultra to 300 studs, TAA, contact shadows, no ray tracing, no real-time GI).
- **Unity**: 6000.6.10f1 (Unity 6.6), HDRP 17.6.0, Vulkan, release Mono player, GPU Resident Drawer with GPU occlusion culling.
- Both pinned to CPUs 0-7 and 16-23, vsync off, a Wayland window, three runs of each (the median run is shown). Every run was limited by the GPU (99 to 100% load) unless a line says otherwise.

## In short

- **Unity HDRP draws this scene 1.2 to 1.7 times faster at 1920 × 1080 and 1.0 to 1.7 times faster at 2560 × 1405.** Five of the six views are between 1.3 and 1.7; the one with little sky in it (the porch) is 1.16 and 1.01.
- **Since then (2026-11-26) the clouds are done**: 0.04 to 0.30 ms with the same picture, and the gap is 1.1 to 1.4 times at 1920 × 1080 and 0.84 to 1.3 times at 2560 × 1405 ("After step 1" below). The rest of this section is as measured on 2026-11-25.
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
- **Foliage shadows.** The engine's rasterized shadows are each card's whole outline, so plants cast dark blocks. HDRP's follow the leaves and read as lighter and more believable. This is the clearest quality difference in Unity's favour, and the project's README already lists it as an engine limit.
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
| Foliage shadows' look | Unity | follows the cut-out |
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
2. **GPU timers by what is drawn: required before any work on the scene pass. The timers and their first table are done (2026-11-26): see "After step 2" below, which sets what follows.** At a quarter of the pixels the scene pass and the prepass still take 1.7 to 2.4 ms together; HDRP's whole fixed cost, shadows included, is 1.8 ms. Today only the whole pass is timed, and the experiments above give its parts only by taking things away.
   - Timers inside the prepass and the scene pass by what is drawn (terrain, painted foliage, models, sky and clouds), in `--bench`'s table, read at each view at a quarter of the pixels, at 1920 × 1080 and at 2560 × 1405, so each part has a fixed cost and a per-pixel cost.
   - Nothing below is started before that table exists, since without it the work could go to the wrong part. Then, by what it shows: the terrain's shader (seven layers with anti-tiling and height blend in every pixel) if the terrain is the large part; drawing solid geometry once where the prepass is not needed for it; culling items hidden behind others from the prepass's depth before the scene and shadow passes draw them (Unity culls on the GPU and has no full prepass: 7.5 M triangles a frame against the engine's 10.3 M for the same 3.6 M in view).
3. **The frame's thread: a prototype of plants kept on the GPU, before any rewrite.** 4.0 to 5.0 ms a frame is a ceiling of 200 to 250 frames a second. In the runs above the GPU was the limit at both sizes (its frame took longer than the thread's in every view but the porch at 1920 × 1080); with step 1 done the windmill view reaches the thread's ceiling, and steps 2 and 4 will bring the others to it. The plants' items are 1.9 ms of it: `Levels of detail`, `Materials`, `Uploads`, `Culling and batches` and `Motion history` are 2.2 ms together today, and the encoding of 3,500 draws another 0.9. Unity's main thread does the same scene in 1.5 ms, on 3.6 to 6.8 cores' worth of CPU time against the engine's 1.1 to 2.1.
   - Two candidates, measured against each other and against today's path at the same views. **Worker threads**: the plants' culling, level-of-detail choice and instance packing shared among the renderer's threads (some of the frame's planning is shared already, `renderer/parallel.rs`), which keeps every draw as it is. **On the GPU**: each foliage group's instances in a buffer that stays there, with what is in view, its level of detail and its cull distance chosen by a compute pass and drawn indirectly (`multi_draw_indexed_indirect_count`, which wgpu has on Vulkan and DirectX 12 as a native feature), so the thread's work is per group and not per plant.
   - Behind a switch, with today's path kept as the fallback (and for adapters without indirect counts).
   - Compared on: the thread's time, GPU time, the process's memory and the card's, and the picture (the same plants at the same levels of detail, the same cross-fades, sway, shadows and motion vectors).
   - The rewrite is decided from that comparison, not from Unity's number.
4. **Shadows, by measurement only: 1.05 to 1.38 ms whatever the window's size.** The same as HDRP's, so this is not where Unity is ahead, but it is a fifth of the frame. Measure the four cascades one by one first (as "Shadows on the patch, cascade by cascade" did). The far cascades change little from frame to frame and are the candidates for being kept; the engine uses over 1 GB less of the card's memory than HDRP here, so there is room to keep them. A kept cascade must still show the plants' sway where a texel of it can: that is part of what is measured. Nothing is changed without a cascade's measurement that says so.
5. **Cut-out shadows for foliage in the rasterized path: a matter of the picture, kept apart from the speed work.** The one place Unity's picture is plainly better. The shadow pass already has the pipeline (`fs_shadow_cut_out`, which hair cards cast through); leaf cards were left casting whole because through their holes the Shadows pass of 36,000 leaf cards went from 0.19 to 0.42 ms (`bench_cut_out_foliage_shadows`, `renderer/lighting.rs`). So this is a switch and a measurement in Homestead, not a new shader: solid casters' draws don't change. Its cost is measured on its own (shadow time with and without, view by view), not folded into step 4's numbers, and the shadow pass is not made dearer just to match the reference: if the cost is large it becomes a quality setting's choice, or cut-outs in the near cascades only, where they can be seen.
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

## What these numbers don't cover

- One scene, one machine, one driver. A scene with many lights, characters or indoor views would exercise other parts of both engines.
- Unity's plants don't sway, and its shadows follow cut-outs. The first saves it work and the second costs it. The no-wind run above bounds the first.
- HDRP is set to match the engine's settings feature by feature, not tuned by a Unity specialist, and the engine's levels of detail and cull rules are imposed on it. A project built for Unity would use its terrain's own trees and details, and would look and run differently.
- The ablations and experiments are one run each (the three-run tables' ranges are within 2%).
- The pictures are compared by eye and by mean brightness, not pixel by pixel.
