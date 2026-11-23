# Phased TDD Implementation Roadmap (RED / GREEN / REFACTOR)

This roadmap sequences the [Remediation Plan](REMEDIATION_PLAN.md) into **6 disciplined TDD phases**. **Do not implement any phase until the audit and plan are approved.**

---

## Phase 0 — Trunk Restoration, Flaky Test Isolation & CI Repair

### 1. Goals
Restore a 100% green local and manual-CI verification baseline on `main` (`REM-P0-01`, `REM-P0-02`, `REM-P0-03`, `REM-P0-14`).

### 2. Scope
- `crates/project/src/tests.rs` (`BlockKind::PlaySound`)
- `crates/script/src/definitions/` & `crates/script/src/events_tests.rs`
- `crates/editor/src/animation/controller_editor/inspector.rs` & `crates/editor/src/profiler.rs`
- `crates/editor-toolbox/src/lib.rs` (parallel test isolation)
- `.github/workflows/ci.yml` (remove `--features hot-patch` / `subsecond`)

### 3. Existing Architecture to Preserve
- ADR-0001 crate boundaries, ADR-0018 manual CI policy, ADR-0023 hot-reload without `subsecond`, `unsafe_code = "deny"`.

### 4. Tests to Write
- Add a repeat-run isolation test / unique temp-dir assertion for `axcenity-editor-toolbox::tests::a_package_is_downloaded_once_and_installed_into_each_project`.
- Keep and run existing `axcenity-project` (248) and `axcenity-script` (421) regression tests unmodified in intent.

### 5. Expected RED Behavior
- `cargo test --locked -p axcenity-project` fails to compile (`E0063`, `E0308`).
- `cargo test --locked -p axcenity-script` fails `every_property_has_a_description` and `a_property_changed_signal_fires_only_for_its_property_with_no_arguments`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` fails on `inspector.rs:634` and `profiler.rs:979`.
- `cargo clippy -p axcenity-editor --features hot-patch` fails (feature does not exist).

### 6. Minimum GREEN Implementation
- Update `BlockKind::PlaySound` initializer in `crates/project/src/tests.rs:655-656`.
- Add missing property doc strings to `PROPERTY_DOCS` and align `events_tests.rs` with `Part` property rules.
- Collapse `if` at `inspector.rs:634` and add `type` alias at `profiler.rs:979`.
- Use unique per-test temp directories in `editor-toolbox` test.
- Remove `--features hot-patch` line from `.github/workflows/ci.yml`.

### 7. Refactoring Opportunities
- Add a single `scripts/check-local.sh` (and `.ps1`) running `fmt --check`, `clippy -- -D warnings`, `cargo test --workspace`, and `check-wiki.py` so contributors have a one-command pre-push gate.

### 8. Integration Validation
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked` (or `cargo nextest run --workspace`)
- `python3 scripts/check-workflows.py`

### 9. Performance Requirements
- Zero runtime performance change; `editor-toolbox` test passes 50 consecutive parallel runs.

### 10. Documentation Updates
- Update `docs/engineering/audit/TEST_INFRASTRUCTURE.md` and `wiki/Changelog/Documentation-History.md`.

### 11. Exit Criteria
- Zero compile errors, zero clippy warnings under `-D warnings`, zero deterministic or flaky test failures across all 47 crates.

---

## Phase 1 — P0 Trust-Boundary & Security Hardening

### 1. Goals
Eliminate all P0 security and memory-safety vulnerabilities (`REM-P0-04` through `REM-P0-11`).

### 2. Scope
- `crates/collab/src/{client.rs, mirror.rs}` (client-side `check_path` enforcement)
- `crates/editor/src/app/plugins.rs` (opt-in plugin trust model)
- `crates/net/src/{session.rs, game.rs, transport.rs, handshake.rs}` (admin auth, strict TLS, constant-time password compare)
- `crates/repo/src/{git.rs, credentials.rs}` (host-scoped `TOKEN_HELPER`, `"--"` guards, binary `Git::show`, newline checks)
- `crates/editor/src/app/mcp/live.rs` & `crates/mcp/src/safety.rs` (CSPRNG token, `0600` file mode, server-side policy cap, bounded `Confirms`)
- `crates/core/src/arena.rs` (`insert_at` slot ceiling + `Result`)
- `crates/script/src/debugger.rs` (`lua_checkstack` guards, no re-entrant `tostring`, capped table scan)

### 3. Existing Architecture to Preserve
- `collab` Ed25519 + TLS 1.3 cert pinning (ADR-0020/0021/0022), `net` server-authoritative replicas (ADR-0010/0026), single-threaded `World` (ADR-0002), `unsafe_code = "deny"`.

### 4. Tests to Write (RED first)
- `collab`: Adversarial host sending `Welcome` with `../../escape`, `/tmp/pwn`, `a\..\b`, and `plugins/evil.lua` -> assert rejected and zero files created/deleted outside root.
- `editor/plugins`: Open project containing `plugins/untrusted/init.lua` with `files` permission -> assert script does not run until explicitly approved for its content hash.
- `net`: (a) Modified client spoofing `Hello.user_id` of an admin -> assert `SessionEvent::Console` denied. (b) Client connecting with password when TCP token port is closed -> assert connection fails without sending cleartext `Hello`.
- `repo`: (a) `TOKEN_HELPER` invoked with `host=evil.example` -> emits empty stdout. (b) `remote_default_branch("--upload-pack=id")` -> returns error. (c) `Git::show` on non-UTF-8 binary blob -> byte-identical output. (d) `credentials` with `\n` -> returns error.
- `mcp`: Live TCP `Hello` requesting `Mode::Full` + `allow_import: ["/"]` against `Standard` / `[]` server -> assert clamped/rejected; assert endpoint file mode `0600` on Unix.
- `core`: `Arena::insert_at(InstanceId { index: (1 << 31) - 1, .. })` -> returns `Err` without large allocation.
- `script`: Debugger stress test with 200-frame call stack, 100k-key table, and throwing `__tostring` userdata.

### 5. Expected RED Behavior
- Every adversarial test above currently fails: `Mirror` writes outside root, untrusted plugin executes, spoofed `UserId` runs console commands, cleartext fallback succeeds, `TOKEN_HELPER` leaks token, `Git::show` corrupts bytes with `U+FFFD`, MCP grants `Mode::Full`, `Arena::insert_at` panics/OOMs.

### 6. Minimum GREEN Implementation
- Call `files::check_path` on all client ingress and inside `Mirror::take_theirs`.
- Store approved `(project, plugin_id, sha256)` in user-local config; gate execution on approval.
- Authenticate `ConsoleService` admin access via Ed25519 key / TLS admin token; disable silent plaintext fallback when password is set or remote host is targeted.
- Parse `host=github.com` in `TOKEN_HELPER`; add `"--"` and leading-`-` checks; add `SystemRunner::run_bytes`; reject `\r`/`\n` in `credentials.rs`.
- Use `ring::rand::SystemRandom` + `0600` permissions + server-enforced max `Mode`/`allow_import` in `mcp/live.rs`; cap `Confirms::issued`.
- Return `Result` from `Arena::insert_at` with `MAX_REPLICA_SLOTS` cap.
- Guard every `lua_getinfo`/`lua_getfenv`/`lua_xmove` with `lua_checkstack`; remove re-entrant `tostring.call` from `step_hook`; disable `lua_singlestep` when 0 breakpoints & not stepping.

### 7. Refactoring Opportunities
- Share Ed25519 challenge-response helpers cleanly between `collab` and `net`; replace O(N) `Arena::free.retain` with O(1) free-list tracking.

### 8. Integration Validation
- Run `axcenity-collab`, `axcenity-net`, `axcenity-repo`, `axcenity-mcp`, `axcenity-core`, `axcenity-script`, and `axcenity-editor` test suites; run `axcenity-bots --cheat`.

### 9. Performance Requirements
- Removing per-opcode `lua_singlestep` when 0 breakpoints exist must eliminate debugger idle overhead; `Arena::insert_at` becomes O(1).

### 10. Documentation Updates
- Update `docs/developer-console.md`, `docs/architecture/networking.md`, `docs/editor/plugins.md`, `docs/editor/mcp.md`, and `wiki/Roadmap/Known-Limitations.md`.

### 11. Exit Criteria
- All 8 P0 security/safety tests pass; zero regressions in existing 5,579 tests.

---

## Phase 2 — Data Integrity, Relay Concurrency, Archive Bounds & Server Operations

### 1. Goals
Close remaining P0 data-loss/race bugs and P1 operational/unpacker hardening (`REM-P0-12`, `REM-P0-13`, `REM-P1-01` through `REM-P1-04`, `REM-P1-07`, `REM-P1-08`).

### 2. Scope
- `crates/save/src/state.rs` (`NaN`/`Inf` handling)
- `crates/relay/src/lib.rs` & `crates/net/src/relayed.rs` (`Mutex<TcpStream>`, rate limits, 16 KiB splice cap, bounded `read_line`/threads)
- `crates/server/src/main.rs` (SIGINT/SIGTERM handler, env-var password, randomized temp dir)
- `crates/net/src/{secure.rs, predict.rs, game.rs}` (rate-limit order, `u32` wraparound, `is_finite()` checks)
- `crates/{plugins,project,toolbox}` (unified bounded safe-zip extraction + symlink/`\` checks)
- `crates/mesh_edit/src/csg.rs` (balanced split heuristic + iterative BSP/Drop)
- `crates/updater/src/package.rs` & `.github/workflows/release.yml` (hex `sha256` validation, pinned tool SHA-256)

### 3. Existing Architecture to Preserve
- Versioned JSON save format v2 + atomic writes (ADR-0016), self-hostable `axcenity-relay` (ADR-0026), server-moved character prediction (ADR-0029), `pkexec` Linux updater (ADR-0043).

### 4. Tests to Write (RED first)
- `save`: Save + reload `World` with `NaN`/`+Inf`/`-Inf` in `Number`, `Vector3`, `CFrame`, `Color3`.
- `relay`: (a) 1,000 concurrent `Punch` + `Open` writes -> assert 0 corrupted JSON lines. (b) >16 KiB TCP splice -> assert terminated. (c) UDP `Bind` flood -> assert rate-limited.
- `server`: Spawn `axcenity-server`, send SIGINT -> assert exit 0 and temp dir deleted.
- `net`: (a) Wrong-password flood -> assert throttled at `TOKENS_PER_MINUTE`. (b) `Driven::step` across `u32::MAX -> 0` wraparound. (c) `NaN` `yaw`/`stick`/`StreamAround` packets -> rejected/clamped.
- `project`/`plugins`/`toolbox`: 100:1 zip-bomb, symlink entry, and `a\..\b` entry tests across all 5 archive readers.
- `mesh_edit`: CSG union/subtract on two 2,000-triangle spheres without stack overflow.
- `updater`: `install_system_package` with shell metacharacters in `sha256` -> returns `Err`.

### 5. Expected RED Behavior
- `save` reload fails with `SaveError::Damaged`; `relay` interleaves JSON or relays >16 KiB; `server` leaks temp dir on SIGINT; `net` allows unlimited wrong passwords and breaks at `u32::MAX`; zip-bombs exhaust RAM/disk; dense CSG overflows stack; `updater` passes unvalidated `sha256`.

### 6. Minimum GREEN Implementation
- Validate/reject non-finite floats before atomic save write.
- Serialize `game.control` writes behind `Arc<Mutex<TcpStream>>`; add relay per-IP rate limits and 16 KiB splice cap; cap `relayed.rs` line length and thread count.
- Wire SIGINT/SIGTERM atomic flag in `axcenity-server`; read `AXCENITY_SERVER_PASSWORD`.
- Move `recent.allow` before `admit()` in `secure.rs`; fix `Driven::step` ordering with `seq_before`; validate floats.
- Extract a bounded safe-zip reader contract across `plugins`, `project`, `toolbox`.
- Make `csg.rs` BSP split-sampled and iterative.
- Validate 64-hex `sha256` in `package.rs` and pin `nfpm`/`appimagetool` SHA-256 in `release.yml`.

### 7. Refactoring Opportunities
- Consolidate duplicated archive path/size/symlink checks into a single audited utility module.

### 8. Integration Validation
- Run full `save`, `relay`, `server`, `net`, `project`, `plugins`, `toolbox`, `mesh_edit`, and `updater` test suites; run `scripts/check-packages.py` and `scripts/check-workflows.py`.

### 9. Performance Requirements
- Balanced BSP should reduce high-segment sphere CSG time by >=5×; relay lock contention <50 µs/op.

### 10. Documentation Updates
- Update `docs/architecture/networking.md`, `docs/architecture/saves.md`, `docs/architecture/solid-modeling.md`, and `docs/project/releases.md`.

### 11. Exit Criteria
- All Phase 2 RED tests pass GREEN; zero temp-dir leaks on server stop; all archive unpackers bounded.

---

## Phase 3 — Runtime Performance, Memory Bounding & In-Place `.axgame`

### 1. Goals
Close key CPU/GPU/RAM runtime gaps (`REM-P1-05`, `REM-P1-06`, `REC-P1-01`, `REC-P1-02`, `REC-P1-03`).

### 2. Scope
- `crates/physics/src/query.rs` (Rapier `QueryPipeline` BVH)
- `crates/assets/src/{derived.rs, hidden.rs, memory.rs}` & `crates/audio/src/lib.rs` (bounded caches + streaming audio)
- `crates/render/src/renderer/` (disk `wgpu::PipelineCache` + 88-byte `ShadowInstanceRaw`)
- `crates/net/src/{game.rs, session.rs, streaming.rs}` (single-serialize `Update` + O(1) peer lookup + dirty streaming)
- `crates/player/src/main.rs` & `crates/project/src/packed.rs` (in-place `.axgame` VFS mount + streaming pack writer)

### 3. Existing Architecture to Preserve
- ADR-0002 (single-threaded `World`), ADR-0003 (`render` consumes draw data only), ADR-0007 (60 Hz physics + kinematic characters), ADR-0008 (offline cooking), ADR-0041 (live identity morphs).

### 4. Tests to Write (RED first)
- `physics`: Benchmark/parity test comparing BVH raycast/shapecast on 10,000 parts (<1 ms for 100 rays).
- `assets`: 1,000 sequential `register_identity` + `hide_regions` calls -> assert RSS/entry count stays below cap and `Assets::memory()` reflects derived entries.
- `audio`: Stream a long audio fixture without decoding full PCM into `decoded` map; assert `decoded` respects byte budget.
- `render`: Cold vs warm `wgpu::PipelineCache` load test; visual parity test for 88-byte `ShadowInstanceRaw`.
- `net`: 32-peer broadcast test asserting single serialization per frame.
- `player`/`project`: Launch packed `.axgame` and assert zero temp-dir files created.

### 5. Expected RED Behavior
- 10k-part raycast scales linearly; `register_identity` grows to 1,000 entries unbounded; `AudioEngine::decoded` holds full track; `player` creates temp dir on `.axgame` launch.

### 6. Minimum GREEN Implementation
- Sync and query Rapier `QueryPipeline` in `physics::query`.
- Add refcount/LRU pruning to `derived.identities` and `hidden_models`; fix `Assets::memory()`; add Kira `StreamingSoundData` + LRU cap on `decoded`.
- Persist `wgpu::PipelineCache` to `.axcenity/shader-cache/`; split `ShadowInstanceRaw` (88B).
- Serialize shared `Message::Update` once per frame in `net::game`; index peers in `HashMap`.
- Implement random-access `.axgame` reader in `assets`/`player` and streaming writer in `project::packed`.

### 7. Refactoring Opportunities
- Remove redundant bounding-ball scan code in `physics/src/query.rs` once edit-mode parity is verified; remove leftover empty bind group 2 in `render/src/renderer/skinning.rs`.

### 8. Integration Validation
- Run full `axcenity-render` 422-test GPU suite (`--test-threads=1`), `axcenity-physics`, `axcenity-assets`, `axcenity-audio`, `axcenity-net`, `axcenity-player`, and `axcenity-bench-check` on desktop hardware.

### 9. Performance Requirements
- S10k / XB10k frame time improves measurably (target: move from 5.14 ms toward <=3.8 ms on `desktop-9950x3d`); shadow pass GPU time drops; warm player startup improves; zero temp-dir disk writes on `.axgame` launch.

### 10. Documentation Updates
- Record measured before/after rows in `wiki/Performance/Optimization-Log.md` and `wiki/Performance/Benchmark-History.md`; close resolved entries in `wiki/Performance/Known-Bottlenecks.md`.

### 11. Exit Criteria
- All benchmarks verified without visual regressions; all memory-bound tests pass; `.axgame` mounts in place.

---

## Phase 4 — Editor Decomposition, Character Completeness, UI & Concurrency Polish

### 1. Goals
Reduce `editor` monolith coupling, finish Character Studio production gaps, fix UI `%` layout bugs, and eliminate ad-hoc thread churn (`REM-P2-01`, `REM-P2-02`, `REC-P1-04`, `REC-P2-01` through `REC-P2-04`).

### 2. Scope
- Extract `crates/editor-character-studio` (and `crates/editor-animation`) from `crates/editor`.
- Wire `ItemManifest::is_draft` filter, synchronized character LODs (CO18), and Eye shading model (`crates/render`, `crates/avatar`).
- Fix `%` margins/padding in 6 `crates/ui` widgets + expose `flex-wrap` and `min/max` sizing.
- Spatial `ItemChunk`s in `crates/scene/src/retained.rs` + decouple `TerrainCache`.
- Pooled cancellable background jobs in `nav`/`terrain`/`procedural`/`mesh_edit` + dynamic navmesh obstacles + async fracture preload.

### 3. Existing Architecture to Preserve
- Action-returning sub-crate pattern (`editor-ui-designer`), `editor-kit` design system, ADR-0031/0032/0036/0041 character contracts, single-threaded `World`.

### 4. Tests to Write (RED first)
- `ui`: `%` margin/padding tests on all 6 widgets + `flex-wrap` & `min/max` layout tests.
- `terrain`: Panicking job test asserting `JobPoll::Failed` (not `None`).
- `nav`: Dynamic unanchored obstacle routing test + `SharedNavMesh::drop` clean cancellation test.
- `render`/`avatar`: Eye shading snapshot test; multi-LOD character capture asserting 0 skin poke-through.
- `physics`: Async fracture preload test asserting zero blocking disk I/O during `shatter`.

### 5. Expected RED Behavior
- `%` margins resolve to `0.0`; `terrain` panics return `None`; agents walk through unanchored obstacles; per-part LOD shows skin poke-through.

### 6. Minimum GREEN Implementation
- Implement shared `%` resolver + `flex-wrap`/`min/max` in `ui`.
- Propagate `JobPoll::Failed` in `terrain`.
- Extract `editor-character-studio` behind clean `Action` enum; wire `is_draft` filter; implement Eye shader and grouped character LOD switching.
- Implement `ItemChunk` static batching in `scene`; route background jobs through a shared bounded pool.

### 7. Refactoring Opportunities
- Remove legacy non-retained `DrawList` path in `crates/scene`; remove duplicated thread-spawning helpers across 4 crates.

### 8. Integration Validation
- Full workspace build, clippy, unit tests, `axcenity-render` GPU suite, `avatar` fit-gate tests, and editor visual snapshots (`AXCENITY_SNAPSHOTS`).

### 9. Performance Requirements
- `crates/editor` incremental rebuild time reduced; `bench_10000_parts` `build_draw_list` <= 2 ms; 20-character crowd GPU time stays <= 1.5 ms on RX 9070 XT.

### 10. Documentation Updates
- Update `docs/ui.md`, `docs/editor/character-studio.md`, `docs/architecture/navigation.md`, `wiki/Editor/Editor-Architecture.md`, and `wiki/Roadmap/Known-Limitations.md`.

### 11. Exit Criteria
- `crates/editor` < 200k LOC, `EditorApp` < 140 fields, all 6 UI widgets pass `%` tests, starter character passes LOD0–LOD2 fit gates with Eye shader.

---

## Phase 5 — Verification Maturity, Fuzzing, Docs Sync & Multi-Machine Scale (MP7)

### 1. Goals
Eliminate documentation drift, add continuous fuzzing harnesses, and prove dedicated-server multiplayer at 16–32 real clients (`REM-P2-03`, `REC-P2-05`, `REC-P2-06`).

### 2. Scope
- Refresh `wiki/` (fix 2 broken `verified_commit`s, update `System-Inventory.md` to 47 crates / wgpu 30 / `0.3.0-alpha.1`), `GEMINI.md`, and `Cargo.toml` (`release/src/console.rs` `unsafe` exemption).
- Add `fuzz/` (`cargo-fuzz`) targets for `net::wire`, `relay::protocol`, `collab::wire`, `project::packed`.
- Add `--health-port` + structured JSON logs to `axcenity-server`; run 16–32 client two-machine soak test (MP7).

### 3. Existing Architecture to Preserve
- ADR-0012 (wiki complements topic docs), ADR-0011/0018 (desktop-first local verification), `axcenity-server` headless boundary (ADR-0001).

### 4. Tests to Write (RED first)
- `scripts/check-wiki.py` strict check (0 errors, 0 invalid-commit warnings).
- `fuzz/` harnesses for wire & archive parsers.
- `server` `/health` HTTP endpoint integration test.

### 5. Expected RED Behavior
- `check-wiki.py` currently emits 2 invalid-commit warnings and 56 stale warnings; `/health` endpoint does not exist; no `fuzz/` targets exist.

### 6. Minimum GREEN Implementation
- Update `wiki/` frontmatter and `System-Inventory.md`, `GEMINI.md`, `Cargo.toml`.
- Add 4 `cargo-fuzz` targets and seed corpora.
- Implement `/health` + JSON log flag on `axcenity-server`; run and document MP7 soak test.

### 7. Refactoring Opportunities
- Archive or integrate the unintegrated `crates/terrain/src/volume.rs` voxel SDF spike so dead prototypes don't accumulate.

### 8. Integration Validation
- `python3 scripts/check-wiki.py` (0 errors, 0 invalid commits), 10M+ fuzz iterations/target with 0 crashes, 1-hour 32-client soak test passing.

### 9. Performance Requirements
- Dedicated server 32-client p99 tick < 16.6 ms with flat RSS over 1 hour.

### 10. Documentation Updates
- Update `wiki/Performance/Benchmark-History.md`, `wiki/Engine-Systems/Networking.md`, `wiki/Roadmap/Current-Roadmap.md`, and `wiki/Changelog/Documentation-History.md`.

### 11. Exit Criteria
- Wiki clean, fuzz targets passing, MP7 multi-machine soak results recorded, engine ready for public beta hardening.
