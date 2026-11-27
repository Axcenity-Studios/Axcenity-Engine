# Handoff: shadow fixes committed; water diagnosed (nothing changed yet)

Written 2026-11-26, end of the seventh session of that day, on branch `homestead-shadows-and-thread`.

## State

- **Committed: `b7fe10cb`** (not pushed): shadow views draw each mesh once, lamp shadow faces kept while their still casters are (moving ones drawn over a kept copy), and the new light property `ShadowStart`. What, why and the numbers: the commit message; [benchmarks/cross-engine/homestead/results.md](benchmarks/cross-engine/homestead/results.md), section "Shadow views by mesh, lamp faces kept, and the porch lamp's lantern"; [docs/rendering/lights.md](docs/rendering/lights.md) ("Where a lamp's shadows start", "Kept lamp shadow views"); [docs/rendering/shadows.md](docs/rendering/shadows.md); wiki Optimization log entry of that name.
- **Still uncommitted: the fifth session's work** (TAA history across the camera's changes of pace, the editor's wake log): `git status` shows exactly its files plus `crates/editor/src/app/wake_log.rs`. The owner hasn't said whether he takes it. `HANDOFF.md` and `.claude/skills/handoff/` stay untracked (ask before adding them).
- **The water report** (below) exists only here and in the reply the owner read. Nothing about water was changed: the owner asked for a report **before changing defaults**.
- Measuring method and the still tool's new options: memory note `profiling-on-this-machine` (its last entry is this session's). Pictures and logs: `target/cross-engine/homestead/step7/` (shadows, `door-*-shadow-start.png` crops) and `step8/` (water; `aim/sheet.jpg` shows the four water views, `project/` and `project-dry/` are frozen copies).

## Waiting for the owner

1. **Water: which fixes he wants** (design discussion first; each needs its own failing measurement, and his rule stands: never trade picture quality for frame rate, memory note `renderer-performance-targets`).
2. **`ShadowStart` on his project**: set `Workspace/Lights/PorchLight` to 0.35 (not done; the project is outside the repo). `LampTable` and `LampBedroom` sit 0.05 studs from their oil lamps' brass and weren't looked at.
3. **The lamps' faces** save 0.8–1.1 ms, not the 1.5 ms he expected: swaying plants in every lamp's range are drawn each frame. The kept copy costs 100 MB at Ultra. He may want to know.
4. The fifth session's commit (above), and the earlier open items in its notes (results.md fifth-session section).

## Water findings (Desert Homestead's painted river at x ≈ −118, still tool, 1920 × 1080, owner's Ultra + TAA, frozen copy, pinned)

| | GPU water in view / away | Frame thread (CPU) in view |
|---|---|---|
| No ray-query device | 9.6 / 7.8 ms | 4.2 ms |
| Ray-query device, every RT effect off | 13.1 / 10.2 | 9.2 (RayScene 3.1, Submit 1.0) |
| Same, reflections Medium or Low | 9.4 / 7.8 | 4.2 |
| Every RT effect on | 28.2 / 24.3 | 10.1 |

1. **Main cause**: `WaterMode::traces_rays` (`crates/render/src/renderer/water.rs:75-77`) needs only a device with ray queries (`render-types/src/graphics.rs:1312`), not any RT effect setting, and the mode is decided by water anywhere in the draw list, on screen or not (`water.rs:48-50`). So the whole ray scene (23,500 instances, TLAS rebuilt each frame) is paid: +3.2 ms GPU acceleration structures, +3.1 ms CPU RayScene.
2. **4-second stall**: the first frame with water builds every BLAS: 4,026 ms (RayScene 3,939, 1,495 meshes). A frame that traces nothing drops the ray scene (`ray_tracing.rs:795-797`), so water leaving the list (streaming, delete, undo) and returning repeats it.
3. **Water's own cost is moderate**: Scene pass 2.44 → 3.69 ms; the screen-space march is ~0.3 ms (Medium vs High). `fs_water` runs 7 waves, a 24 + 5 step march (`water.wgsl:293-299`), sun visibility and surrounding light twice (`water.wgsl:349-371`). With every RT effect on, RT AO (8–13 ms) and RT shadows (4–9 ms) dominate, water or not.
4. **Editor drag** (optimized build; the debug editor's scene side is slower): `WaterLevel` with painted water and flooding on rebuilds every painted chunk under a new mesh key each frame (`scene/src/terrain_water.rs:118-139`): Uploads 0.24 → 15.6 ms, frame 15 → 32 ms (no RT device), 25 → 42 ms (RT device), ray scene +22 meshes a frame, the old ones kept. With flooding off (the project now) Level rebuilds nothing. A Material-tab hand-over (`assets/src/lib.rs:713` bumps the generation; `scene/src/retained.rs:205-218` rebuilds every retained item) measured only +0.15 ms with flooding alone. **Not measured, a subagent's code reading only**: the water brush stamping every 0.05 s and rebuilding the chunk and 4 neighbours (`editor .../strokes.rs:319-322`), Terrain Studio sliders recording an undo step per frame (`edits.rs:1202-1216`), the `TracedWater` pipeline dropped on frames without rays (`water.rs:252-259`).
5. **Repetition**: waves are shader-only, 7 Gerstner waves, the longest twice the next, all within ±0.6 rad of one direction except the small ones (`water.wgsl:106-108`); defaults length 12, height 0.25, variation 0.6 (`crates/render/src/water.rs:44-53`). On the shallow river the swell rises above the banks. No water object or gizmo: a Part's material file, or Terrain Studio's Water section and brush.
6. **Not reproduced**: the owner's exact 195 → 40 FPS (the editor's Play wasn't timed; the still tool runs CPU and GPU one after the other). Next measurement if needed: the editor with `AXCENITY_FRAME_LOG=1` on a project copy, water in and out of streaming range (memory note's editor method).

## Traps found this session

- **The owner saves Desert Homestead while you measure**: compare builds only on a frozen `cp -a --reflink=auto` copy.
- **Each still-tool shot reloads the world**: give every shot its own `"set"`.
- **The editor's unit-test binary overflows a 2 MiB test-thread stack** (`the_recover_prompt_draws_and_escape_keeps_it`, `a_drag_is_held_until_it_ends`; already so before `b7fe10cb`) and aborts the rest of that binary: `RUST_MIN_STACK=16777216` runs all 1,577. Not fixed (a `with_big_stack` wrap of one test only moved the failure to the next).
- Anything new the shadow shaders read must go into `lamp_shadows::fingerprint` (or `moves`), or kept lamp faces go stale (`crates/render/CLAUDE.md`).

## Suggested skills

- `implement` once the owner picks water fixes; read [AGENTS.md](AGENTS.md) and the Rendering wiki page and ADR-0025 (optional ray tracing) first.
- `code-review` on `b7fe10cb` before it's pushed, and on the fifth session's diff before it's committed.
- `run` to show the owner the editor with water in and out of view, or `PorchLight` with `ShadowStart` 0.35.
- `handoff` again at the end.
