---
title: Configuration reference
type: reference
status: current
verified: 2026-11-11
verified_commit: 19c7179d
source: [crates/player/src/watch.rs, crates/server/src/main.rs, crates/editor/src/main.rs, crates/testing/src/main.rs, crates/testing/src/bench_check.rs, Cargo.toml]
docs: [docs/benchmarks.md, docs/development.md, docs/rendering/settings.md, docs/architecture/networking.md]
tags: [reference, configuration]
---

# Configuration reference

Command-line flags, environment variables, Cargo features and settings files. Gameplay configuration lives in the world (Instances) and project files ([File formats](File-Formats.md)).

## Command lines

| Binary | Flags |
|---|---|
| `axcenity-player [game]` | `--host [port]` (47800), `--join addr:port`, `--password <text>`, `--watch` (live reload from a project folder), `--dev` (F9 console in a built game), `--bench <frames>` |
| `axcenity-server <game>` | `--port <port>`, `--password <text>`, `--bench <frames>`, `--bench-clients <n>`, `--help` |
| `axcenity-editor [folder]` | `--cook <folder>` (cook and exit; 1 on failure), `--build-game …`, `--mcp <folder> [--profile lite\|standard\|full] [--mode read\|edit\|full]`, `--render-movie <folder> --sequence <path or name> --out <file or folder> [--size WxH] [--fps N] [--quality …] [--format h264\|av1\|ffv1\|png] [--range FROM:TO] [--warm-up N] [--motion-blur 0..1] [--no-ray-tracing] [--no-letterbox] [--no-ui]` (render a sequence as a movie and exit; `docs/editor/movie.md`), `--version`, `--resume <handoff>` (debug relaunch only) |
| `axcenity-release` | `check [--tag v…]`, `prepare <version>`, `query compare\|introduced\|search …` |
| `axcenity-toolbox` | The Toolbox CLI (`crates/toolbox/src/cli.rs`) |
| `axcenity-test <project>` | `--bench <frames>`, `--make-s10k <folder>`, `--junit <file>`, `--help` |
| `axcenity-bench-check` | `--machine <name>`, `--update`, `--only <text>`, `--rounds <n>`, `--fresh`, `--projects <folder>`, `--cook`, `--short`, `--fast`, `--exact` |

## Environment variables

| Variable | Effect |
|---|---|
| `RUST_LOG` | Log filter (default `warn,wgpu_hal=error`) |
| `AXCENITY_BENCH_PROJECT`, `AXCENITY_BENCH_FRAMES` | Project and frame count for `bench_project`, `bench_project_play` |
| `AXCENITY_BENCH_MACHINE`, `AXCENITY_PROJECTS` | Machine name and projects folder for the regression check |
| `AXCENITY_SHOT_DIR`, `_VIEWS`, `_SIZE`, `_QUALITY`, `_NOAA`, `_DEBUG` | `shoot_project` screenshots |
| `AXCENITY_SNAPSHOTS` | Drawing tests save PNGs there |
| `AXCENITY_FFMPEG` | The `ffmpeg` program Render Movie encodes videos with, in place of the one on `PATH` |
| `AXCENITY_WRITE_REFERENCES`, `AXCENITY_WRITE_EXAMPLES` | Rewrite reference images / generated example files |
| `AXCENITY_DUMP` | `dump_every_example` output file |
| `AXCENITY_RETAINED_DRAW_LIST=off` | Build the draw list from scratch each frame |
| `AXCENITY_PHYSICS_THREADS` | Physics' rayon pool size (default at most 12) |
| `AXCENITY_ANIM_THREADS` | The animation posing pool's size (default at most 8; 1 = the calling thread) |
| `AXCENITY_RENDER_THREADS` | The renderer's planning pool size (default at most 12; 0 = rayon's global pool) |
| `AXCENITY_TOOLBOX` | Path to the built `axcenity-toolbox` binary for the Python package's tests |
| `AXCENITY_PHYSICS_VERIFY` | Debug: cross-check incremental physics sync |
| `AXCENITY_NET_JSON=1` | Send the network wire as JSON |
| `AXCENITY_SAVE_DIR` | Override the saves folder (tests) |
| `AXCENITY_EXPORT_SMOKE` | Packaging smoke test destination |
| `AXCENITY_ASSETS_SOURCE_HASH` | Set by `assets/build.rs`; stamps cooked models |
| `AXCENITY_OCCLUSION_PROBE=<every>[/<texels>]` | Measuring only (2026-11-27): every `<every>` frames the renderer reads back the prepass's depth and, for the next 8 planned frames, leaves out items whose bounding box lies wholly behind it (`renderer/occlusion_probe.rs`); prints how many it hid. Not a feature: the read stalls the frame |
| `AXCENITY_BAKE_PROBES=1` | The editor opens the project, bakes every light and reflection probe once the scene has settled (120 frames), saves and closes (2026-11-27; Overgrown Street's `build_all.sh` uses it) |

## Cargo features

| Crate | Feature | Effect |
|---|---|---|
| hosts | `profile-tracy` | Send `profiling` spans to Tracy |
| `editor` | `engine-dev` | Rebuild and relaunch from the source tree (dev only, Phase 89) |
| `render` | `debug-views` | Editor debug views (only the editor turns it on) |
| `script` | `csg` (default) | `UnionAsync` and friends via `mesh_edit` |

## Settings files

| File | Scope | Holds |
|---|---|---|
| Editor settings (eframe persistence, OS app data) | Per user | Appearance, layout, recent projects, graphics settings, overlays, steady-frame pacing |
| Player `graphics.json` (game's data folder) | Per machine per game | `GraphicsSettings`; since 2026-11-27 an optional `ray_traced_shadow_resolution` (`"Full"` or `"Half"`; left out, Full at Ultra and Half below) |
| `.config/nextest.toml` | Repo | Timeouts, slow-first ordering, the `gpu` group (8) |
| `.config/hakari.toml` | Repo | Workspace-hack platforms and `final-excludes` (`http`, `httparse`, `winit`, `profiling`) |
| `rust-toolchain.toml` | Repo | Rust 1.97.0 |
| `benchmarks/baselines.json` | Repo | Per-machine benchmark baselines |
