# Technical Debt & Confirmed Defects Register (`8f4c2420`)

This register strictly separates **confirmed defects & security vulnerabilities** from **architectural debt, performance bottlenecks, and documentation drift**. All line numbers are verified at commit `8f4c2420`.

## A. Confirmed Build, Lint & Test Failures on `main` (`8f4c2420`)

| ID | Location | Evidence | Impact |
|---|---|---|---|
| **TD-001** | `crates/project/src/tests.rs:655-656` | `E0063` missing `pitch`, `wait_until_done`; `E0308` expected `ObjectRef`, found `String` in `BlockKind::PlaySound` | Workspace test suite (`cargo test --workspace`) does not compile |
| **TD-002** | `crates/script/src/definitions/tests.rs:85` | `every_property_has_a_description` fails: `PROPERTY_DOCS` drifted from `core/src/class.rs` | Luau API definitions & docs out of sync with `core` |
| **TD-003** | `crates/script/src/events_tests.rs:90` | `a_property_changed_signal_fires_only_for_its_property_with_no_arguments` fails (`Speed is not a property of Part`) | Broken unit test on `main` |
| **TD-004** | `crates/editor/src/animation/controller_editor/inspector.rs:634`, `crates/editor/src/profiler.rs:979` | `clippy::collapsible_if` and `clippy::type_complexity` fail under `-D warnings` | Workspace `clippy --all-targets -- -D warnings` fails |
| **TD-005** | `.github/workflows/ci.yml:69` | Passes removed `--features hot-patch` to `axcenity-editor` (removed in ADR-0023) | Manual CI workflow dispatch guaranteed to fail |
| **TD-006** | `crates/editor-toolbox/src/lib.rs` | `a_package_is_downloaded_once_and_installed_into_each_project` fails under parallel `cargo test`, passes in isolation | Flaky parallel-race test |

## B. Confirmed Security & Safety Vulnerabilities

| ID | Severity | Location | Finding |
|---|---|---|---|
| **SEC-001** | **P0** | `crates/collab/src/client.rs:1119-1131`, `mirror.rs:282-319` | **Arbitrary file write/delete via path traversal from malicious Collaborate host**: `files::check_path` never called on `ToClient::Welcome { files }` or `Mirror::take_theirs` (`self.root.join(path)`). Can write `../../.bashrc` or `plugins/evil.lua`. |
| **SEC-002** | **P0** | `crates/editor/src/app/plugins.rs:66-121` | **Auto-execution of untrusted project plugins with `files` permission**: Opening any project runs `<project>/plugins/*/init.lua` without trust prompt, granting full project read/write. |
| **SEC-003** | **P0** | `crates/net/src/session.rs:352-377`, `game.rs:1667-1684` | **Unauthenticated `UserId` spoofing -> Remote Console Execution**: `Host::greet` accepts client-asserted `user_id` verbatim; `ConsoleService.AllowedUserIds` authorizes against it. |
| **SEC-004** | **P0** | `crates/net/src/transport.rs:546-550, 642-645` | **SSL-stripping / plaintext password downgrade**: `Trust::Remember` silently falls back to cleartext UDP on TCP token failure, transmitting server password in plaintext JSON. |
| **SEC-005** | **P0** | `crates/repo/src/git.rs:29, 278-284, 422-654` | **GitHub OAuth token leak & Git option injection**: `TOKEN_HELPER` ignores `host=` and sends token to any HTTPS remote/redirect; `ls-remote`/`fetch`/`push`/`merge` omit `"--"` allowing `--upload-pack=` RCE. |
| **SEC-006** | **P0** | `crates/editor/src/app/mcp/live.rs:84-103, 230-231, 472-498` | **Live MCP privilege self-declaration**: `.axcenity/mcp-endpoint.json` written with default umask (`0644`) and non-CSPRNG SipHash token; server blindly accepts client-supplied `Mode::Full` and `allow_import: ["/"]`. |
| **SEC-007** | **P0** | `crates/core/src/arena.rs:187-197` | **Unbounded allocation OOM/DoS**: `Arena::insert_at` resizes `slots` to `id.index + 1` with no cap below `LOCAL_BASE` (`2^31`). |
| **SEC-008** | **P0** | `crates/script/src/debugger.rs:1244-1289` | **`unsafe` Luau C-stack overflow / UB risk**: `lua_getinfo`, `lua_getfenv`, `lua_xmove` called after `lua_checkstack` failure paths; re-entrant `tostring.call` inside `step_hook` before `lua_yield`. |
| **SEC-009** | **P1** | `crates/relay/src/lib.rs:339-387, 495-539, 648-668` | **Open relay abuse & reflection**: Unauthenticated game-slot exhaustion (`max_games=100`), unrate-limited UDP `Bind` reflection/brute-force, unmetered TCP `splice()` tunnel, plaintext host `secret`. |
| **SEC-010** | **P1** | `plugins/src/bundle.rs:148-166`, `project/src/package.rs:342-367`, `user_templates.rs:315-350`, `toolbox/src/guard.rs:43-85` | **Four archive unpackers lack zip-bomb bounds & symlink checks** (`.axtemplate` also lacks `\` check on Linux; `.axgame` lacks total archive byte cap). |
| **SEC-011** | **P1** | `crates/net/src/secure.rs:413-422`, `relayed.rs:186-245`, `repo/src/credentials.rs:58-65` | Password brute-force bypasses token rate limit; uncapped host thread spawning & unbounded `read_line` from relay; newline injection in `git credential` stdin. |
| **SEC-012** | **P1** | `.github/workflows/release.yml:391-395`, `crates/updater/src/package.rs:39, 69-90` | Unverified `nfpm`/`appimagetool` (`continuous`) CI downloads; unvalidated `sha256` string passed to `pkexec /bin/sh` in `install_system_package`. |

## C. Confirmed Correctness & Data-Loss Bugs

| ID | Severity | Location | Finding |
|---|---|---|---|
| **BUG-001** | **P0** | `crates/save/src/state.rs:116-180` | `NaN`/`Inf` floats serialize as JSON `null`, permanently corrupting save files on reload (`SaveError::Damaged`). |
| **BUG-002** | **P0** | `crates/relay/src/lib.rs:245-250, 369, 615` | Concurrent unsynchronized `write_all` on cloned `game.control` `TcpStream` from UDP and TCP threads corrupts framed JSON control stream. |
| **BUG-003** | **P0** | `crates/repo/src/git.rs:140, 722-726` | `Git::show` decodes stdout via `String::from_utf8_lossy`, silently corrupting binary files with `U+FFFD`. |
| **BUG-004** | **P1** | `crates/server/src/main.rs:377-415` | `axcenity-server` has no SIGINT/SIGTERM handler; leaks temp dirs, UPnP port maps, client disconnects, and stats file. |
| **BUG-005** | **P1** | `crates/net/src/predict.rs:100-121, 421, 458`, `game.rs:1706-1715` | `Driven::step` breaks `u32` sequence wraparound; `yaw`/`stick` and `StreamAround.position` lack `is_finite()` validation. |
| **BUG-006** | **P1** | `crates/mesh_edit/src/csg.rs:441-535` | Recursive BSP splits on `polygons.first()` -> O(N) recursion depth and stack-overflow risk on smooth meshes via `UnionAsync`/`SubtractAsync`. |
| **BUG-007** | **P2** | `crates/ui/src/{chat,dialogue_box,hotbar,interact,leaderboard,prompt}.rs` | Six UI widgets silently treat `%` margins/padding as `0.0`. |
| **BUG-008** | **P2** | `crates/terrain/src/job.rs:78-84` | `Job::try_take` silently swallows worker thread panics as `Some(None)`. |

## D. Structural, Memory & Performance Debt

| ID | Severity | Area | Evidence & Impact |
|---|---|---|---|
| **PERF-001** | **P1** | `crates/physics/src/query.rs:243-287` | `raycast`/`shapecast`/`parts_in_*` linearly scan Workspace bounds & build fresh colliders instead of using Rapier's BVH (amplified by 64-slice character moves & 48-ray audio occlusion). |
| **PERF-002** | **P1** | `crates/assets/src/derived.rs`, `hidden.rs`, `audio/src/lib.rs:467` | Unbounded RAM caches: `Assets::register_identity` (0.6–1.4 MB/s during Character Studio drags), `derived.hidden_models`, `AudioEngine::decoded` (no streaming audio). |
| **PERF-003** | **P1** | `crates/render/src/renderer/` | Zero `wgpu::PipelineCache` (`cache: None` at 30+ sites); 172-byte `InstanceRaw` used in shadow passes; 3,703-line hand-ordered `scene.rs`. |
| **PERF-004** | **P1** | `crates/net/src/game.rs:1288-1293`, `session.rs:426`, `streaming.rs:247` | Re-serializes `Message::Update` N× per frame (once per peer); O(N) `connection_of` scan; `rebuild_parts` every 200ms per peer even without changes. |
| **PERF-005** | **P2** | `crates/player/src/main.rs:155-189`, `project/src/packed.rs:149-214` | `.axgame` read fully into RAM and unpacked to temp dir on every launch; `pack_game_reporting` holds ~2× game size in RAM. |
| **PERF-006** | **P2** | `crates/scene/src/retained.rs:319, 338-384` | Retained draw cache still clones all `DrawItem` slices per frame and walks entire Workspace on any structural change. |
| **PERF-007** | **P2** | `crates/physics/src/fracture.rs:144-145` | Synchronous `assets.model_blocking` on simulation thread during `physics.shatters`. |
| **ARCH-001** | **P1** | `crates/editor/src/app.rs:208-604` | 257k-LOC monolithic `editor` crate & 171-field `EditorApp` God struct slow incremental builds and invite merge conflicts. |
| **ARCH-002** | **P2** | `nav/shared.rs`, `terrain/job.rs`, `procedural`, `mesh_edit/bake.rs` | Ad-hoc OS thread spawning & detached threads on drop instead of shared bounded job pool. |
| **DOC-001** | **P2** | `wiki/` (58 warnings), `GEMINI.md`, `Cargo.toml` | `check-wiki.py` reports 58 warnings (2 invalid `verified_commit`s, 56 stale pages); `System-Inventory.md` still cites 33 crates / wgpu 25 / 0.2.0; `release/src/console.rs` `unsafe` exemption undocumented. |

## E. Code Hygiene Baseline (Strengths)

Across 776,017 lines of Rust: **1** `TODO` (`editor/src/character_workspace/panel/worn.rs:271`), **0** `FIXME`/`HACK`/`XXX`, **0** `todo!()`, **0** `unimplemented!()`, and only **14** `allow(dead_code)` attributes. `cargo fmt --all --check` passes cleanly.
