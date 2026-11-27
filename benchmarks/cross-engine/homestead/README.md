# Desert Homestead: Axcenity against Unity HDRP

The owner's showcase scene, Desert Homestead, rebuilt in Unity's High Definition Render Pipeline from what Axcenity itself draws, and both timed at the same views on the same machine. [XB10k](../README.md) compares engines on parts, physics and scripts; this compares the renderers on a real outdoor scene: 65,490 placed things (116,048 drawn items, 65,000 of them painted foliage with three levels of detail), two terrains, a 210,000-triangle house, cascaded sun shadows, volumetric fog and clouds.

Results and what they say about the engine are in [results.md](results.md).

## How the port is made

Nothing is placed by hand, and no model is imported a second time through another importer.

1. `dump/dump.sh <project> <dump folder> assets/sequences/Tour.sequence.json` builds a small program against the engine (as the project's own `authoring/shot` does) that loads the project, builds the renderer's draw list and writes it out: every mesh with the simpler copies the engine draws at a distance (its own and the ones `meshopt` makes), every texture, every material, every item's transform, cull distance and whether it casts a shadow, the sun, and the tour's camera at each sixtieth of a second.
2. `export_terrain.py <project> <dump folder>` writes the two terrains' heights, paint and layer textures (each layer's mask map packed as Unity reads it).
3. `unity/setup.sh import` copies `unity/Assets/Homestead` into the Unity project and runs `HomesteadImport.Import` in the editor, headless: textures in the formats the engine gives the GPU (BC7, BC5 for normal maps), an HDRP Lit material per engine material, one mesh per model and level of detail (a submesh per primitive), two Unity Terrains, the sun, the look and the scene.
4. `unity/setup.sh build` builds the Linux player.

The Unity project is not in the repository (it is 6 GB with its import cache): `~/Projects/Unity Projects/Desert Homestead HDRP`, with the dump in its `Dump` folder. To make it again: create an HDRP project from Unity Hub's template, turn off **Auto Graphics API for Linux** in Player settings and leave Vulkan as the only one (see "Unity on Linux" below), then run the four steps.

The engine's space is right-handed and Unity's left-handed. Everything is turned over in X (meshes, places, the terrain, the sun, the camera), so Unity draws the engine's picture and not its mirror image.

## What is the same

| | Axcenity | Unity |
|---|---|---|
| Geometry | The engine's meshes | The same corners and triangles, from the dump |
| Levels of detail | A simpler copy below 30% and 12% of the screen's height (its bounding sphere's), cross-faded | A LODGroup per thing with the same sphere and the same two sizes, cross-faded |
| Foliage culling | Past a distance set by the plant's size, 60 to 1,500 studs | The same distance, as the LODGroup's last size for the field of view in use |
| Foliage shadows | Through the leaves' cut-outs (since 2026-11-26), whole to 35% of the cull distance (then thinning to none at 45%, see below) | Through the cut-outs, none past 35%, as a second renderer without shadows for the far part of each level |
| Textures | BC7, BC5 normal maps, full size | The same images in the same formats |
| Terrain | Its own, from `heights.png` and seven painted layers | Unity's Terrain from the same heights and weights, eight-layer TerrainLit, layers mixed by weight |
| Sun shadows | 4 cascades of 4096², to 300 studs, percentage-closer soft shadows, contact shadows | 4 cascades at HDRP's Ultra (4096²), to 300 units, High filtering (PCSS), contact shadows |
| Anti-aliasing | TAA | TAA |
| Ambient occlusion | Screen space, High | HDRP's screen-space ambient occlusion |
| Fog | Height fog lit by the sun through its shadows (a 160 × 90 × 64 grid) | HDRP fog with volumetric fog |
| Clouds | Volumetric, 64 steps and 6 toward the sun, casting shadows | HDRP Volumetric Clouds, 64 steps and 6 light steps (its defaults), casting shadows |
| Post-processing | Filmic tone mapping, bloom, vignette, color correction | Neutral tone mapping, bloom, vignette, color adjustments |
| Camera | Near 0.1, far 5,000, the tour's field of view | The same |
| Window | A Wayland window, vsync off | A Wayland window (`SDL_VIDEODRIVER=wayland`), vsync off |

Axcenity runs at the owner's Ultra settings, `benchmarks/features/owner-ultra.json`: no ray tracing and no real-time GI. Unity's player is the release (non-development) Mono build, with the GPU Resident Drawer and GPU occlusion culling on: they are Unity's own way to draw this many copies, and a Unity project of this scene would use them.

## What differs, and which way it leans

- **Wind.** The engine sways every plant in the vertex shader (and its shadow, and its motion vectors). HDRP's Lit shader has no vertex animation, and the port doesn't add a Shader Graph for it, so Unity's plants stand still. This saves Unity work: the vertex shader's sway and an object motion vector pass over the foliage.
- **Cut-out shadows.** The same since 2026-11-26: both shadow passes follow the leaves' cut-outs. Until then the engine's rasterized shadows drew each card whole, which was cheaper and looked heavier; the tables of 2026-11-25 and the first two sessions of 2026-11-26 were measured that way.
- **Where a plant's shadow ends.** Both cast it whole to 35% of the plant's cull distance. Since 2026-11-26 (the fourth session) the engine's then thins out to nothing at 45%, so shadows don't pop as the camera moves; the port's still ends at 35% (`ShadowReach` in `HomesteadWorld.cs`). The engine draws 1,700 to 2,400 more shadow caster draws a view for it, 0.07 to 0.11 ms: its cost, and Unity's tables of 2026-11-25 don't have it.
- **Leaves.** The engine lets light through leaves with its own term; Unity's leaves use HDRP's translucent material with its foliage diffusion profile.
- **Sun rays, dust.** The engine's screen-space sun rays and the two dust emitters have no counterpart in the port. Axcenity's cost, and not small where the dust is in view: 0.34 to 0.42 ms of GPU time in the hero and mailbox views at 1920 × 1080 (results.md, "After step 2").
- **Scripts.** The still views run no scripts in either engine. In the tour the engine's script turns the windmill, rocks the chair and rolls a tumbleweed; Unity's tour moves only the camera, the lens and the fades.
- **Cloud shape and place.** Each engine's own clouds, so a cloud's shadow may lie on the mountains in one picture and not the other.
- **Terrain detail.** Each engine's own terrain mesh and level of detail. The engine's anti-tiling and height blend are left out of Unity (HDRP's height blend picks one layer instead of mixing).
- **Units.** One Unity unit is one stud (0.35 m). HDRP's sky and fog read it as a metre; their numbers are set by eye.
- **Ray tracing.** Unity traces no rays on this machine, so only the rasterized configuration is compared and Axcenity's ray traced configurations are timed alone. See "Ray tracing" below.
- **Screen-space reflections** (since 2026-11-26) and the engine's later shading (micro-shadowing, glossy leaves) have no counterpart in the port: Axcenity's cost.

## The look

HDRP's lighting is in physical units and the engine's is not, so the sun (100,000 lux), the fixed exposure (EV 14.55), the sky's own exposure (+2.15 stops) and the fill light (0.3 of the sky's) were set by eye and by the pictures' mean brightness, against the engine's own stills of the same six views (`authoring/shot`). They change what the picture looks like and not what is drawn. `HomesteadBench`'s `-hs-set` changes them for one run without a rebuild.

## How it is timed

`run.py` runs one benchmark at a time, the engines taking turns, each pinned to the same cores (`taskset -c 0-7,16-23`, the 9950X3D's V-cache cores).

- **Six still views**: the tour's camera at one moment of each shot (5.5, 15.5, 25, 35, 45 and 58 seconds). In Axcenity each is a copy of the project standing still (no scripts, the tour not playing) with a three-line LocalScript that places the camera; in Unity the same numbers on the command line. 1,800 frames after the warm-up (60 frames in Axcenity, 240 in Unity).
- **The tour**: its first 61 seconds, the six daytime shots, with its cuts, lenses and fades, at a sixtieth of a second of game time a frame in both (`--bench`, `Time.captureFramerate`). The dusk time-lapse is left out.
- **Frame time** is the time between the starts of consecutive frames as each engine's own clock has it (Axcenity's timeline file, a `Stopwatch` in Unity), vsync off. FPS is frames over their time; the 1% low is the rate of the slowest hundredth.
- **The machine**, read five times a second while frames are measured: the card's load, power, clock and memory (`/sys/class/drm/card1`), the process's memory on the card (`fdinfo`), its CPU time over all threads and its memory.
- Three runs of each; the tables show the run with the median frame rate and the range.

```bash
benchmarks/cross-engine/homestead/dump/dump.sh "$HOME/Projects/Axcenity Projects/Desert Homestead" "$HOME/Projects/Unity Projects/Desert Homestead HDRP/Dump" assets/sequences/Tour.sequence.json
python3 benchmarks/cross-engine/homestead/export_terrain.py "$HOME/Projects/Axcenity Projects/Desert Homestead" "$HOME/Projects/Unity Projects/Desert Homestead HDRP/Dump"
benchmarks/cross-engine/homestead/unity/setup.sh both
cargo build --release -p axcenity-player
python3 benchmarks/cross-engine/homestead/run.py --size 1920x1080            # both engines, stills and the tour
python3 benchmarks/cross-engine/homestead/run.py --size 2560x1440 --unity-size 2560x1405   # the larger window (GNOME makes it 2560 x 1405)
python3 benchmarks/cross-engine/homestead/collect.py ultra-1920x1080          # the tables
benchmarks/cross-engine/homestead/ablate.sh                                   # one feature off at a time, both engines
benchmarks/cross-engine/homestead/ablate.sh table                             # those runs as a table
benchmarks/cross-engine/homestead/ablate.sh world                             # the engine alone: no clouds, no wind, no foliage
```

The Axcenity runs read the project copies the regression check makes (`target/bench-projects`; `axcenity-bench-check --only homestead` makes them). Keep the benchmark's window visible and in front: a covered Wayland window is not given frames.

## Unity on Linux

Unity's editor opened the HDRP template with OpenGL on this machine (Fedora, RX 9070 XT, Mesa 26.2), and HDRP does not draw with OpenGL: every HDRP shader is "unsupported", and the editor's log says `Platform StandaloneLinux64 with graphics API OpenGLCore is not supported with HDRP`. The project names Vulkan, but with **Auto Graphics API for Linux** on the editor takes OpenGL all the same. Vulkan itself works (RADV). The fix is per project: Player settings > Other Settings, **Auto Graphics API for Linux** off, Vulkan the only entry (in `ProjectSettings.asset`, `m_Automatic: 0` under `LinuxStandaloneSupport`). `-force-vulkan` on the editor's command line does the same for one start.

## Ray tracing

HDRP traces rays only with DirectX 12, so its ray traced effects need the Windows player, and that player must be built by the **Windows** editor. Unity's Linux editor builds a DirectX 12 player (`unity/setup.sh windows`, `HomesteadImport.BuildWindows`, into `Build-Windows/`), but leaves out every ray tracing shader: the `.raytrace` shaders come out empty and Lit's ray tracing passes aren't compiled (a known Unity issue). The findings are in results.md, "Ray tracing in both engines".

What's ready for a run from the Windows editor:

- `HomesteadImport.BuildWindows` allows ray tracing, screen-space shadows and screen-space reflections in the pipeline asset for that build only, sets DirectX 12 as the only API, and puts the asset back afterwards.
- `-hs-set rtshadows=4,rtao=2,rtreflections=1` turns on HDRP's traced sun shadows (rays a pixel), traced ambient occlusion (rays a pixel, to 1.5 units) and traced reflections (one ray, Performance mode, surfaces smoother than 0.5, to 800 units). They are set to the work the engine's High does, not to HDRP's High presets, which trace far more. The result's JSON gives `hdrp_ray_tracing` (whether HDRP's ray tracing was live) and `ray_traced`.
- `run.py --unity-player windows` runs that player through GE-Proton (`~/.local/share/proton-ge/GE-Proton11-14-x86_64`, `HOMESTEAD_PROTON` for another), whose vkd3d-proton turns DXR into Vulkan's ray tracing. It hasn't been run yet. Natively on Windows, run the player with the same arguments `unity_run` gives it.

The engine's side: `--graphics benchmarks/features/owner-rt-shadows-ao.json` (traced shadows and AO at High) or `owner-rt-all.json` (reflections too).

## Layout

- `dump/`: the program that writes what the engine draws.
- `export_terrain.py`: the terrains.
- `unity/Assets/Homestead/Editor/HomesteadImport.cs`: the importer and the player build. `Runtime/HomesteadWorld.cs` spawns the instances with the engine's level-of-detail rules, `HomesteadBench.cs` is the benchmark, `HomesteadCatalog.cs` what the importer hands them.
- `unity/setup.sh`: copies the scripts in and runs the editor headless (`windows`: the DirectX 12 player with ray tracing allowed).
- `run.py`, `collect.py`, `ablate.sh`: the runs, the tables, and one feature off at a time.
- `results.md`: the numbers and what follows from them.
