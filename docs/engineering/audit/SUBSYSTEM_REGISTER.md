# Subsystem Register (`8f4c2420`)

Every major subsystem audited directly against source code and tests. Maturity uses the 0–4 scale: **0** Missing, **1** Experimental, **2** Partial, **3** Implemented, **4** Production-ready.

| # | Subsystem | Crates | Maturity | Summary Verdict |
|---|---|---|---:|---|
| 1 | Core Runtime & Data Model | `core`, `sim` | **3** | Solid generational arena & 10-stage schedule; `Arena::insert_at` OOM risk on untrusted IDs |
| 2 | Scene & World Management | `scene`, `core::streaming`, `project` | **3** | Retained `DrawCache`, levels, data layers, streaming (off by default); full `DrawItem` clone/frame |
| 3 | Resource & Asset Management | `assets`, `cook`, `convert` | **3** | Async glTF/FBX/OBJ + BC7/BC5 + USD→GLB; unbounded identity/hidden/asset RAM caches |
| 4 | Memory & Concurrency | Workspace-wide | **2** | Clean single-threaded `World` + bounded sim pools; ad-hoc thread spawns & unbounded caches |
| 5 | Scripting & Visual Scripting | `script`, `blocks`, `blocks-runtime`, `gameplay` | **3** | Rich Luau sandbox + blocks/graphs + hot reload; `debugger.rs` unsafe stack gaps & 2 failing tests |
| 6 | Serialization & Saves | `project`, `save` | **2** | Versioned JSON + migrations + atomic writes; `NaN`/`Inf` corrupts saves, `project` tests broken |
| 7 | Rendering & Graphics | `render`, `render-types` | **3** | Broad tiled forward+ PBR, TAAU/EASU, compute posing; no render graph, no pipeline cache, 172B shadow instances |
| 8 | Materials & Shaders | `render`, `assets`, `editor` | **3** | Material graphs, `naga`-checked WGSL surface shaders, Skin/Hair/Cloth; no Eye shader |
| 9 | Lighting & GI | `render`, `scene` | **3** | CSM/spot/point shadows, probes, realtime GI, software GI, opt-in HW RT (disabled by default) |
| 10 | Physics | `physics` | **3** | Patched `rapier3d` 0.36, joints, vehicles, fracture, ragdolls; O(N) non-BVH `raycast`/queries |
| 11 | Audio | `audio` | **2** | Kira spatial audio, occlusion, buses, reverb zones, effects; no streaming audio & unbounded decode cache |
| 12 | Animation | `anim` | **3** | Clips, state machines, blend spaces, IK, retargeting, facial/viseme lip-sync, parallel eval |
| 13 | Character Creation & Deformation | `avatar`, `editor/character_workspace`, `anim` | **2** | Real v2 manifests, surface-binding fitter, compute posing; realistic wardrobe/hair are drafts with fit defects |
| 14 | Terrain & Procedural Generation | `terrain`, `procedural` | **3** | Heightfield sculpt/erosion/biomes/LOD, scatter, spline paths, buildings; voxel SDF is unintegrated spike |
| 15 | UI System | `ui`, `editor-ui-designer` | **2** | CSS + `taffy` flexbox, widgets, focus, accessibility; no grid/`flex-wrap`/`calc`, % padding zeroed in 6 widgets |
| 16 | VFX & Particles | `render/gpu_particles`, `scene/particles` | **3** | CPU + GPU compute particles (262k/emitter), bitonic sort, curves, screen-space collisions |
| 17 | AI & Navigation | `nav` | **2** | 2.5D column navmesh, A*, agents, perception, behavior trees; dynamic obstacles ignored, thread-per-rebuild |
| 18 | Multiplayer & Networking | `net`, `relay` | **2** | `renet` 2, TLS tokens, relay, prediction, lag comp, replays; critical auth/downgrade/relay security defects |
| 19 | Dedicated Servers | `server` | **2** | Headless 60 Hz host + `axcenity-bots`; no SIGINT/SIGTERM cleanup, no `/health`/structured logs |
| 20 | Editor Applications & Studios | `editor`, `editor-*` (7 crates) | **3** | Huge breadth (Model/UI/Character/Anim/Graph/Terrain/Effect studios); 257k-LOC monolith, 171-field `EditorApp`, 2 clippy errors |
| 21 | Plugin Architecture | `plugins`, `editor/app/plugins` | **2** | Luau Plugin API v3 with docks/handles/importers; **auto-executes untrusted project plugins with `files` access** |
| 22 | Asset Pipeline & Toolbox | `assets`, `cook`, `convert`, `toolbox` | **3** | Offline BC7/BC5, `.axmodel`, USD→GLB, multi-provider Toolbox; Toolbox zip extraction lacks size cap |
| 23 | Project Management & Collab | `project`, `collab`, `collab-server`, `repo`, `teamwork` | **2** | Real-time Yjs/WS collab + Git sync + Team Board; **critical `Mirror` path traversal & Git token/arg injection** |
| 24 | Build, Packaging & Deployment | `player`, `release`, `updater`, `packaging/`, `.github/` | **2** | Cross-OS installers, signed manifest updater, `pkexec` pkgs; `.axgame` unpacks to temp, broken `ci.yml`, unverified CI tools |
| 25 | Developer Tooling & AI/MCP | `mcp`, `testing`, `editor-benchmark`, `editor/profiler` | **2** | Full MCP server, Luau test runner, bench-check, F9 console; **live MCP privilege self-declaration**, no `cargo-fuzz` |
| 26 | Online Services | *(none — `docs/research/online-services.md`)* | **0** | Design-only (O1–O9); `crates/online` does not exist; only Ed25519 identity seam exists in `collab` |

---

## Detailed Subsystem Notes

### 1. Core Runtime & Data Model (`core`, `sim`)
- **Source**: `crates/core/src/{arena.rs, world.rs, class.rs, journal.rs, streaming.rs}`, `crates/sim/src/{schedule.rs, steps.rs, occlusion.rs, golden.rs}`.
- **Tests**: 148 (`core`) + 15 (`sim`); all pass. `golden.rs` verifies frame order matches `docs/architecture/frame-loop.md`.
- **Defects & Risks**: `Arena::insert_at` (`arena.rs:187-197`) resizes `slots` to `id.index + 1` with no upper bound below `LOCAL_BASE` (`2^31`) — OOM/DoS vector from untrusted network IDs. Linear `free.retain` scan (`arena.rs:204`). `occlusion.rs:122` fires up to 48 O(N) raycasts/frame.
- **Recommendations**: Cap `insert_at` index growth; switch free-list tracking to O(1) slot tagging; route occlusion through Rapier BVH.

### 2. Scene & World Management (`scene`, `core::streaming`)
- **Source**: `crates/scene/src/{draw_list.rs, retained.rs, terrain.rs, particles.rs, foliage.rs, decals.rs, pick.rs}`.
- **Tests**: 218 (`scene`), all pass.
- **Defects & Performance**: `DrawCache::add_to` (`retained.rs:319`) clones `DrawItem` slices every frame even on cache hit; any single Workspace structural change forces full `go_over_tree` rebuild (`retained.rs:338-384`); `TerrainCache` (`terrain.rs:59-80`) couples draw-list retention and streaming to terrain.
- **Recommendations**: Move to spatial `ItemChunk`s for static geometry, decouple `DrawCache` from `TerrainCache`, and remove the legacy non-retained path.

### 3. Resource & Asset Management (`assets`, `cook`, `convert`)
- **Source**: `crates/assets/src/{lib.rs, loading.rs, derived.rs, hidden.rs, memory.rs, model.rs, model/fbx.rs}`, `crates/cook/src/lib.rs`, `crates/convert/src/usd.rs`.
- **Tests**: 225 (`assets`) + 12 (`cook`) + 10 (`convert`), all pass.
- **Defects & Gaps**: Unbounded caches in `Assets::register_identity` (`derived.rs:38-49`), `derived.hidden_models`/`hidings` (`hidden.rs:83-88,129-134`), and primary asset maps (`lib.rs:195-272`). `Assets::memory()` (`memory.rs:38-79`) omits derived/hidden models and identities. `cook` skips non-multiple-of-4 textures and hardcodes BC7 `very_fast`. `convert` (USD) is static-snapshot geometry-only (no skeletons, animations, blend shapes, or concave n-gons).
- **Recommendations**: Add LRU/refcount pruning for identities and hidden models, fix `Assets::memory()`, add release-quality BC7 tier.

### 4. Memory & Concurrency
- **Implementation**: Plain Rust ownership + `Arc` sharing; single-threaded `World`; dedicated Rayon pools in `physics` and `anim`; worker threads in `assets`.
- **Concerns**: Multiple ad-hoc thread spawns (`nav::SharedNavMesh`, `terrain::parallel_rows`, `procedural::make_chunks`, `mesh_edit::bake`, `net::secure`, `collab::socket`); unbounded caches across `assets`, `audio`, `mcp::Confirms`, `net::Issuer::log`.
- **Recommendations**: Consolidate background CPU jobs onto a shared bounded thread pool with cancellation tokens; enforce caps on all session caches.

### 5. Scripting & Visual Scripting (`script`, `blocks`, `blocks-runtime`, `gameplay`)
- **Source**: `crates/script/src/{context.rs, sandbox.rs, watchdog.rs, debugger.rs, lent.rs, http.rs}`, `crates/blocks/src/{lua.rs, from_lua.rs, hot_reload.rs}`, `crates/blocks-runtime/src/{lib.rs, translate.rs}`, `crates/gameplay/src/`.
- **Tests**: 421 (`script`, **2 FAILING at HEAD**), 59 (`blocks`), 21 (`blocks-runtime`), 43 (`gameplay`).
- **Defects**: (1) `debugger.rs:1244-1289` unsafe `lua_getinfo`/`lua_getfenv`/`lua_xmove` called after `lua_checkstack` failure paths; re-entrant `tostring.call` inside `step_hook` before `lua_yield`; per-opcode hook overhead when debugger active. (2) No per-script memory limit; C functions (`string.rep`) bypass watchdog. (3) `HttpService` (`http.rs:1-55`) is JSON/GUID-only (no HTTP). (4) `Humanoid::State::Swimming` (`humanoid.rs:71-72`) is an unimplemented stub. (5) Two failing unit tests in `script` at `8f4c2420`.
- **Recommendations**: Fix the 2 failing tests immediately; harden `debugger.rs` stack checks and avoid per-opcode hooks when 0 breakpoints exist; add Luau memory allocator cap.

### 6. Serialization & Saves (`project`, `save`)
- **Source**: `crates/project/src/{packed.rs, package.rs, user_templates.rs, tests.rs}`, `crates/save/src/{lib.rs, state.rs, migrate.rs}`.
- **Tests**: 248 (`project`, **FAILS TO COMPILE at HEAD**), 26 (`save`, all pass).
- **Defects**: (1) `crates/project/src/tests.rs:655-656` fails to compile. (2) `save/src/state.rs:116-180` serializes `NaN`/`Inf` floats as JSON `null`, permanently corrupting save files on reload (`SaveError::Damaged`). (3) Runtime terrain edits, scripts, players, and UI are excluded from save state. (4) `.axpkg` and `.axtemplate` unpackers lack zip-bomb bounds; `.axtemplate` lacks `\` check on Linux.
- **Recommendations**: Fix `project/src/tests.rs`; sanitize/reject non-finite floats in `save::state`; unify archive extraction behind a bounded safe-zip helper.

### 7–9. Rendering, Materials/Shaders, Lighting & GI (`render`, `render-types`)
- **Source**: `crates/render/src/renderer/{scene.rs, gpu.rs, lighting.rs, posing.rs, ray_tracing.rs, ray_soft.rs, realtime_gi.rs, water.rs, temporal.rs, texture_budget.rs}`, `crates/render/src/adapters.rs`, 47 `.wgsl` files.
- **Tests**: 442 (`render`, headless GPU suite; 6 ignored benchmarks/evidence).
- **Defects & Concerns**: 3,703-line hand-ordered `scene.rs`; zero `wgpu::PipelineCache` (`cache: None` at 30+ sites); 172-byte `InstanceRaw` used in shadow passes; `EXPERIMENTAL_RAY_QUERY` disabled by default due to RX 9070 XT DX12 flashing (`adapters.rs:111-113`); software GI omits skinned characters; no Eye shading model; leftover empty bind group 2 (`skinning.rs:128-135`).
- **Recommendations**: Add disk `wgpu::PipelineCache`, slim 88-byte shadow instance stream, implement Eye shading model, and plan render graph migration (L8).

### 10. Physics (`physics`, `vendor/rapier3d`)
- **Source**: `crates/physics/src/{lib.rs, sync.rs, query.rs, fracture.rs, joints.rs, vehicles.rs, threads.rs}`.
- **Tests**: 161 (`physics`), all pass.
- **Defects & Performance**: `query.rs:243-287` (`raycast`, `shapecast`, `parts_in_*`) linearly scans Workspace bounding balls and allocates fresh Rapier colliders per hit instead of using Rapier's BVH query pipeline; `fracture.rs:144-145` calls `assets.model_blocking` synchronously on the sim thread.
- **Recommendations**: Route `query.rs` through Rapier's `QueryPipeline`; preload fracture piece meshes asynchronously.

### 11. Audio (`audio`)
- **Source**: `crates/audio/src/{lib.rs, effects.rs, render_tests.rs}`.
- **Tests**: 21 (`audio`, including PCM DSP verification), all pass.
- **Defects**: `AudioEngine::decoded` (`lib.rs:467`) caches all decoded `StaticSoundData` forever with no byte cap; no streaming playback for long music/ambience tracks.
- **Recommendations**: Add `StreamingSoundData` path for long audio assets and LRU byte budget on `decoded`.

### 12–13. Animation & Character Creation (`anim`, `avatar`, `character`, `editor/character_workspace`)
- **Source**: `crates/anim/src/{controller.rs, ik.rs, retarget.rs, face.rs, lip_sync.rs, threads.rs}`, `crates/avatar/src/{description.rs, fit/*.rs}`, `crates/character/src/{lib.rs, predict.rs}`.
- **Tests**: 188 (`anim`), 165 (`avatar`), 45 (`character`), all pass.
- **Defects & Gaps**: Realistic family ships only underwear + 1 draft hairstyle + 3 draft garments that fail `asset-quality.md` fit/poke-through gates; Character Studio doesn't filter drafts yet; per-part LOD switching causes skin poke-through at distance; `character` controller casts up to 64 O(N) raycast slices per step.
- **Recommendations**: Ship draft filtering ("Show drafts"), synchronized character LOD sets (CO18), and complete realistic wardrobe/hair QA.

### 14. Terrain & Procedural Generation (`terrain`, `procedural`)
- **Source**: `crates/terrain/src/{brush.rs, erosion.rs, mesh.rs, job.rs, volume.rs}`, `crates/procedural/src/{terrain_scatter.rs, paths.rs, building.rs}`.
- **Tests**: 68 (`terrain`), 29 (`procedural`), all pass.
- **Defects & Gaps**: `terrain/src/volume.rs` (439 LOC voxel SDF) is an unintegrated benchmark spike; `Job::try_take` (`job.rs:78-84`) swallows worker panics; ad-hoc thread spawns.
- **Recommendations**: Log/propagate `Job` panics; decide whether to integrate or archive `volume.rs`.

### 15. UI System (`ui`, `editor-ui-designer`)
- **Source**: `crates/ui/src/{lib.rs, style.rs, css.rs, paint.rs, chat.rs, hotbar.rs, interact.rs}`.
- **Tests**: 109 (`ui`), 47 (`editor-ui-designer`), all pass.
- **Defects & Gaps**: Missing CSS `display: grid`, `flex-wrap`, `min/max-width/height`, `em/rem/vh/vw`, `calc()`. Six widgets (`chat.rs:77`, `dialogue_box.rs:36`, `hotbar.rs:85`, `interact.rs:1001`, `leaderboard.rs:167`, `prompt.rs:162`) silently treat `%` margins/padding as `0.0`.
- **Recommendations**: Fix `%` margin/padding resolution and add `flex-wrap` + `min/max` sizing.

### 16. VFX & Particles (`render/gpu_particles`, `scene/particles`)
- **Source**: `crates/render/src/renderer/gpu_particles.rs`, `particles.wgsl`, `crates/scene/src/particles.rs`.
- **Status**: Real CPU→GPU compute particle pipeline (threshold 3,000, max 262,144/emitter), curves, flipbooks, screen-space depth/normal collisions. Missing node-based programmable VFX graph (deferred to R17.5).

### 17. AI & Navigation (`nav`)
- **Source**: `crates/nav/src/{lib.rs, builder.rs, shared.rs, path.rs, agents.rs, perception.rs, behavior.rs}`.
- **Tests**: 48 (`nav`), all pass.
- **Defects**: Ignores unanchored dynamic obstacles (`builder.rs:12-15`); spawns a new OS thread per rebuild and detaches on drop (`shared.rs:49-53`); column-grid navmesh lacks Recast polygon/off-mesh link richness.
- **Recommendations**: Reuse a cancellable background worker; add dynamic obstacle avoidance/carving.

### 18–19. Multiplayer, Relay & Dedicated Servers (`net`, `relay`, `server`)
- **Source**: `crates/net/src/{transport.rs, secure.rs, session.rs, game.rs, predict.rs, relayed.rs, streaming.rs}`, `crates/relay/src/lib.rs`, `crates/server/src/main.rs`.
- **Tests**: 114 (`net`), 6 (`relay`), 20 (`server`), all pass.
- **Critical Defects**:
  1. Unauthenticated `UserId` spoofing grants `ConsoleService.AllowedUserIds` remote command execution (`session.rs:352-377`, `game.rs:1675`).
  2. `Trust::Remember` silently downgrades to plaintext UDP and sends server password in cleartext on TCP token failure (`transport.rs:546-550, 642-645`).
  3. Password brute-force bypasses token rate limit (`secure.rs:413-422`); slow-loris TCP DoS (`MOST_AT_ONCE = 64`).
  4. `relay`: open unauthenticated game-slot exhaustion, unrate-limited UDP `Bind` reflection, unmetered TCP `splice()` tunnel, **concrete unsynchronized `TcpStream` concurrent write data race on `game.control`** (`relay/src/lib.rs:245-250, 369, 615`).
  5. `server`: no SIGINT/SIGTERM handler (`server/src/main.rs:377-415`) — leaks temp dirs and UPnP leases.
  6. Scalability/bugs: `Driven::step` breaks `u32` sequence wraparound (`predict.rs:421, 458`); `NaN`/`Inf` unchecked in `Move::drive` & `StreamAround`; `Message::Update` re-serialized N× per frame (`game.rs:1288-1293`).

### 20–21. Editor, Studios & Plugins (`editor`, `editor-*`, `plugins`)
- **Source**: `crates/editor/src/app.rs` (171 fields), `crates/editor/src/app/plugins.rs`, `crates/plugins/src/{lib.rs, bundle.rs, files.rs}`.
- **Tests**: 1,505 (`editor`, blocked by 2 clippy errors under `-D warnings`), 305 across 7 `editor-*` crates (1 flaky test in `editor-toolbox`), 19 (`plugins`).
- **Critical Defects**:
  1. **Auto-execution of untrusted project plugins with `files` permission** (`editor/src/app/plugins.rs:66-121`) without user consent.
  2. `.axcenity-plugin` unpacker lacks zip-bomb and symlink checks (`plugins/src/bundle.rs:148-166`).
  3. 257k-LOC monolithic `editor` crate & 171-field `EditorApp` hurting compile times and maintainability.

### 22–23. Project Collaboration, Repo & Teamwork (`collab`, `collab-server`, `repo`, `teamwork`)
- **Source**: `crates/collab/src/{client.rs, host.rs, mirror.rs, files.rs, live.rs, socket.rs}`, `crates/repo/src/{git.rs, credentials.rs}`, `crates/teamwork/src/{board.rs, history.rs}`.
- **Tests**: 182 (`collab`), 6 (`collab-server`), 54 (`repo`), 12 (`teamwork`), all pass.
- **Critical Defects**:
  1. **Arbitrary file write/delete path traversal from malicious Collaborate host** (`collab/src/client.rs:1119-1131`, `mirror.rs:282-319` — `files::check_path` never called client-side!).
  2. **GitHub OAuth token exfiltration to arbitrary HTTPS remotes** (`repo/src/git.rs:29, 278-284` — `TOKEN_HELPER` ignores `host=`).
  3. **Git option/argument injection** (`repo/src/git.rs:422-654` — missing `"--"` in `ls-remote`/`fetch`/`push`/`merge`).
  4. **Binary file corruption in `Git::show`** (`repo/src/git.rs:140, 722-726` — `String::from_utf8_lossy`).
  5. Newline injection in `git credential` stdin (`repo/src/credentials.rs:58-65`); uncollected upload blobs in `FolderStore`.

### 24–26. Build/Updater, Developer Tooling/MCP & Online Services (`player`, `release`, `updater`, `mcp`, `testing`, `editor-benchmark`)
- **Tests**: 263 (`player`), 52 (`release`), 26 (`updater`), 23 (`mcp`), 28 (`testing`), 23 (`editor-benchmark`).
- **Critical Defects**:
  1. **Live MCP TCP client self-declares `Mode::Full` & `allow_import: ["/"]`** over a `0644` non-CSPRNG `.axcenity/mcp-endpoint.json` (`editor/src/app/mcp/live.rs:84-103, 230-231, 472-498`).
  2. **Broken manual CI workflow**: `.github/workflows/ci.yml:69` passes removed `--features hot-patch`.
  3. **Unverified supply-chain downloads in `release.yml:391-395`**: `nfpm` and `appimagetool` (`continuous` tag) fetched without SHA-256 checks.
  4. **`.axgame` unpacked to temp dir on every player launch** (`player/src/main.rs:155-189`); `pack_game_reporting` uses ~2× game size in RAM.
  5. `updater/src/package.rs:39,69-90` lacks defense-in-depth hex validation on `sha256` passed to `pkexec /bin/sh`.
  6. `editor-benchmark/src/video_memory.rs:87-106` only detects Linux VRAM on `amdgpu`.
  7. **Online Services**: 0% implemented (`docs/research/online-services.md` O1–O9 is design-only).
