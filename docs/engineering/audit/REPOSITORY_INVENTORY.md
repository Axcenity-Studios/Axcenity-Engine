# Repository Inventory (`8f4c2420`)

## Workspace Summary

Measured directly from the repository tree at `8f4c2420`:

| Metric | Measured Value | Note vs `wiki/Architecture/System-Inventory.md` |
|---|---:|---|
| Workspace Crates | **47** | Wiki says 33 (stale since `19c7179d`) |
| Rust Source Files (`.rs`) | **1,626** | — |
| Rust Lines of Code (incl. tests) | **776,017** | Wiki says ~466k (stale) |
| WGSL Shader Files | **47** | Wiki says 24 (stale) |
| WGSL Lines of Code | **10,410** | Wiki says ~5.1k (stale) |
| `#[test]` / `#[tokio::test]` Functions | **5,579** | Wiki says ~3,150 (stale) |
| Lua / Luau Files (crates/samples/projects/templates/runner) | **120** | Wiki says ~108 |
| Example Projects (`crates/samples/projects/`) | **71** | Wiki says 67 |
| Genre Templates (`templates/`) | **6** | Matches |
| Compiled Binaries | **10** | Across 8 binary crates |
| Version (`[workspace.package]`) | `0.3.0-alpha.1` | Wiki says `0.2.0-alpha.1` (stale) |
| Toolchain (`rust-toolchain.toml`) | `1.97.0` | Edition 2024, Cargo resolver 3 |

## Delta Since Previously Audited Commit (`cc2ee95`)

Four commits landed between `cc2ee95` and `8f4c2420` (**263 files changed, +32,075 / −4,413 lines**):

| Commit | Summary | Key Areas Modified |
|---|---|---|
| `45042de0` | Updates: RPM, Debian, Arch and AppImage Installs Update from the Editor | `crates/updater/src/package.rs` (+291), `crates/editor/src/app/update_install.rs`, `crates/release/src/install.rs`, ADR-0043 |
| `788fc3f3` | Implemented Appearance Enhancements | Editor shell/chrome (`viewport_bar.rs`, `dock_chrome.rs`, `toolbar.rs`, `profiler.rs`, `output.rs`, `explorer.rs`, `start/*`), `editor-kit` widgets/theme |
| `dc487f0d` | Updated Appearance and Agent Usage | `GEMINI.md`, `.agents/skills/axcenity-workflow/SKILL.md`, `editor-kit/src/theme.rs` (+1,163), `editor-tools` gizmo/shortcuts, `settings/appearance.rs` & `graphics.rs`, `group_mesh.rs`, `mcp/check.rs` & `live.rs`, `anim/property_clip.rs`, `core/class.rs` |
| `8f4c2420` | Updated code and editor features | Major `blocks-runtime` (+6,081) & `blocks` (+1,598) expansion, `block_editor.rs` (+2,105) & `block_editor/graph.rs` (+823), animation controller/graph/retarget UI, `terrain/brush.rs`, `core/class.rs`, `definitions/axcenity.d.luau`, `documentation/api/`, packaging assets |

**Regressions introduced in this 4-commit delta (verified by live build/test runs):**
1. `crates/project/src/tests.rs:655-656` fails to compile (`BlockKind::PlaySound` gained `pitch`, `wait_until_done`, and `ObjectRef` in `8f4c2420`).
2. `crates/script` fails 2 unit tests (`every_property_has_a_description` due to new `core/class.rs` properties lacking `PROPERTY_DOCS`; `a_property_changed_signal_fires_only_for_its_property_with_no_arguments` due to `Speed is not a property of Part`).
3. `crates/editor` fails `cargo clippy -- -D warnings` on 2 lints (`animation/controller_editor/inspector.rs:634` `collapsible_if`, `profiler.rs:979` `type_complexity`).

## Complete 47-Crate Inventory

| Crate | `.rs` Files | Rust LOC | `.wgsl` Files | WGSL LOC | `#[test]` | Role |
|---|---:|---:|---:|---:|---:|---|
| `editor` | 527 | 257,297 | 0 | 0 | 1,505 | Main eframe/egui editor binary (`axcenity-editor`) |
| `script` | 123 | 54,953 | 0 | 0 | 421 | Luau runtime (`mlua`), sandbox, debugger, engine API |
| `render` | 103 | 53,671 | 47 | 10,410 | 442 | `wgpu` 30 tiled forward+ PBR, GI, RT, post, compute posing |
| `mesh_edit` | 59 | 26,954 | 0 | 0 | 134 | Half-edge modeling, CSG, fracture, UV, paint, sculpt, bake, rig |
| `player` | 58 | 26,868 | 0 | 0 | 263 | Standalone game runtime (`axcenity-player`) |
| `assets` | 47 | 25,501 | 0 | 0 | 225 | Async glTF/FBX/OBJ loader, LODs, cooked models, derived identities |
| `project` | 52 | 23,952 | 0 | 0 | 248 | Project JSON I/O, migrations, prefabs, `.axgame`/`.axpkg`/`.axtemplate` |
| `anim` | 42 | 22,581 | 0 | 0 | 188 | Clips, state machines, blend spaces, IK, retargeting, facial/lip-sync |
| `scene` | 41 | 20,259 | 0 | 0 | 218 | World-to-`DrawList` bridge, retained cache, terrain/particles/picking |
| `collab` | 47 | 20,192 | 0 | 0 | 182 | Real-time Collaborate: TLS+WS, Ed25519, Yjs live scripts, handoff |
| `avatar` | 50 | 18,361 | 0 | 0 | 165 | Character v2 schemas, manifests, clothing surface-binding fitter |
| `net` | 41 | 18,308 | 0 | 0 | 114 | `renet` 2 + `postcard`, TLS tokens, relay client, prediction, streaming |
| `core` | 32 | 17,512 | 0 | 0 | 148 | Generational `Arena`, `World`, `Instance`, `ClassName`, `EditJournal` |
| `ui` | 31 | 16,426 | 0 | 0 | 109 | CSS cascade, `taffy` flexbox layout, widgets, focus, accessibility |
| `editor-model-studio` | 23 | 13,728 | 0 | 0 | 62 | Extracted Model Studio editor crate |
| `editor-ui-designer` | 29 | 13,504 | 0 | 0 | 47 | Extracted UI Designer artboard crate |
| `blocks-runtime` | 9 | 13,419 | 0 | 0 | 21 | Runtime-safe block/graph AST & Luau codegen (used by `player`) |
| `physics` | 31 | 12,187 | 0 | 0 | 161 | `rapier3d` 0.36 (patched), joints, vehicles, fracture, queries |
| `editor-tools` | 19 | 10,622 | 0 | 0 | 80 | Gizmos, shortcuts, posing, terrain tools |
| `gameplay` | 23 | 10,109 | 0 | 0 | 43 | `GameSystem`s: Debris, Abilities, Humanoid/Health, Tools, Story, Avatar |
| `blocks` | 8 | 10,051 | 0 | 0 | 59 | Editor-only Luau parser, Lua→Blocks decompiler, hot-reload rewriter |
| `toolbox` | 23 | 7,096 | 0 | 0 | 85 | Online asset providers (ambientCG, Poly Haven, Openverse), SQLite DB |
| `terrain` | 18 | 6,606 | 0 | 0 | 68 | Heightfield brushes, erosion, biomes, LOD meshing, voxel SDF spike |
| `editor-kit` | 10 | 6,228 | 0 | 0 | 41 | Shared egui design-system theme & widgets |
| `release` | 13 | 5,619 | 0 | 0 | 52 | SemVer release CLI (`axcenity-release`), minisign verification |
| `procedural` | 17 | 5,367 | 0 | 0 | 29 | TerrainScatter, TerrainPath splines, procedural buildings |
| `nav` | 12 | 5,257 | 0 | 0 | 48 | 2.5D column navmesh, A*, agents, perception, behavior trees |
| `repo` | 12 | 5,136 | 0 | 0 | 54 | Git CLI wrapper, GitHub device OAuth, auto-checkpoint & conflict sync |
| `editor-toolbox` | 8 | 4,967 | 0 | 0 | 41 | Extracted Toolbox panel & audio preview |
| `character` | 7 | 4,512 | 0 | 0 | 45 | Kinematic 1st/3rd-person controllers, seats, prediction replay |
| `testing` | 13 | 3,861 | 0 | 0 | 28 | `axcenity-test` Luau spec runner, S10k/crowd scenes, `axcenity-bench-check` |
| `plugins` | 10 | 3,728 | 0 | 0 | 19 | Editor Luau Plugin API v3 runtime & `.axcenity-plugin` bundler |
| `server` | 12 | 3,357 | 0 | 0 | 20 | Headless `axcenity-server` & `axcenity-bots` load/soak harness |
| `editor-benchmark` | 8 | 3,086 | 0 | 0 | 23 | In-editor GPU benchmark & preset recommender |
| `editor-node-graph` | 6 | 3,022 | 0 | 0 | 12 | Shared node-graph canvas (UI graphs, behavior trees, anim/blocks) |
| `updater` | 10 | 2,859 | 0 | 0 | 26 | Resumable HTTPS updater (`axcenity-updater`), atomic swap, `pkexec` pkgs |
| `audio` | 3 | 2,524 | 0 | 0 | 21 | `kira` 0.12 spatial audio, buses, reverb zones, effect chains |
| `render-types` | 3 | 2,511 | 0 | 0 | 22 | Draw-list & graphics config types shared between `scene` and `render` |
| `save` | 5 | 2,442 | 0 | 0 | 26 | Versioned JSON save slots, checkpoints, world-state diff/restore |
| `teamwork` | 6 | 2,173 | 0 | 0 | 12 | File-backed Team Board (`team/board/`) & person history JSONL |
| `mcp` | 9 | 2,120 | 0 | 0 | 23 | `rmcp` MCP server core & confirmation safety |
| `convert` | 6 | 2,104 | 0 | 0 | 10 | Editor-only USD (`openusd`) → `.glb` converter |
| `sim` | 8 | 1,563 | 0 | 0 | 15 | Unified 10-stage frame schedule across 4 hosts + sound occlusion |
| `relay` | 4 | 1,262 | 0 | 0 | 6 | Self-hostable `axcenity-relay` TCP/UDP rendezvous & hole-punch server |
| `cook` | 3 | 1,117 | 0 | 0 | 12 | Editor-only BC7/BC5 texture cooker (`intel_tex_2`) |
| `collab-server` | 3 | 1,042 | 0 | 0 | 6 | Self-hostable headless `axcenity-collab` server |
| `workspace-hack` | 2 | 3 | 0 | 0 | 0 | `cargo-hakari` unified feature hack |

## Workspace Binaries (10)

| Binary | Crate | Purpose |
|---|---|---|
| `axcenity-editor` | `axcenity-editor` | Desktop editor + Character Studio + `--mcp` AI server |
| `axcenity-player` | `axcenity-player` | Standalone game runtime (host/join, `--dev`, `.axgame`) |
| `axcenity-server` | `axcenity-server` | Headless dedicated game server |
| `axcenity-bots` | `axcenity-server` | Multi-client scale, churn, flood, and cheat bot harness |
| `axcenity-collab` | `axcenity-collab-server` | Headless self-hosted Collaborate server |
| `axcenity-relay` | `axcenity-relay` | Self-hosted NAT traversal / join-code / UDP+TCP relay |
| `axcenity-release` | `axcenity-release` | Release preparation, manifest generation, verification CLI |
| `axcenity-test` | `axcenity-testing` | Headless Luau game spec runner (`tests/*.spec.lua`) & `--bench` |
| `axcenity-bench-check` | `axcenity-testing` | Performance regression checker against `benchmarks/baselines.json` |
| `axcenity-updater` | `axcenity-updater` | Out-of-process self-updater (archive swap, AppImage, `pkexec` pkgs) |
