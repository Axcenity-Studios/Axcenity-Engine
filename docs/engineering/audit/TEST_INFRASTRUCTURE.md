# Test & Benchmark Infrastructure Assessment (`8f4c2420`)

## 1. Execution Environment & Limitations

- **OS / Kernel:** Linux x86_64 (`7.3.4-300.fc45.x86_64`, Fedora 45 sandbox)
- **Toolchain:** `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo 1.97.0`
- **Unavailable Tools in Sandbox:** `cargo-nextest` and `cargo-hakari` are not installed in `PATH`; all test runs fell back to `cargo test --locked`. Network-dependent Toolbox tests (`#[ignore = "reaches ..."]`) and multi-machine LAN/Tailscale/GitHub tests cannot run in the offline sandbox.
- **Not Executed:** Full `axcenity-editor` (1,505 tests) and `axcenity-player` (263 tests) suites were not executed because `axcenity-editor` fails `-D warnings` clippy and `axcenity-project` fails to compile tests (blocking full `--workspace` execution). Live `axcenity-bench-check` was not executed against `desktop-9950x3d` baselines because baselines are strictly machine-specific.

## 2. Verified Command Execution Log (2026-11-22, Commit `8f4c2420`)

| Check / Command | Result | Exact Counts / Evidence |
|---|---|---|
| `cargo fmt --all --check` | **PASS** | Exit 0, zero formatting diffs |
| `cargo tree` boundary checks (`player`, `server`, `render`) | **PASS** | Zero forbidden edges (ADR-0001, ADR-0003) |
| `python3 scripts/check-wiki.py` | **WARN** | 115 pages, **0 errors**, **58 warnings** (2 invalid `verified_commit` hashes, 56 stale pages) |
| `cargo clippy --workspace --all-targets --locked --keep-going -- -D warnings` | **FAIL** | **2 compile errors** in `axcenity-project` (`tests.rs:655-656`); **2 clippy errors** in `axcenity-editor` (`inspector.rs:634` `collapsible_if`, `profiler.rs:979` `type_complexity`) |
| `cargo test --locked` (16 core/sim/net/script/collab/physics/blocks crates) | **FAIL** | **1,287 passed**, **2 FAILED** (`axcenity-script`: `every_property_has_a_description`, `a_property_changed_signal_fires_only_for_its_property_with_no_arguments`), 20 ignored |
| `cargo test --locked --no-fail-fast` (22 anim/assets/audio/avatar/mesh_edit/scene/server/terrain/testing/toolbox/ui/updater crates) | **PASS** | **All passed** (0 failed across all 22 crates; includes `avatar` fitting & realistic library tests, `audio` PCM DSP tests, `testing` S10k/template tests, `updater` real_binary/end_to_end tests) |
| `cargo test --locked --no-fail-fast` (7 extracted `editor-*` crates) | **FAIL (FLAKY)** | **300 passed**, **1 FAILED** under parallel run (`axcenity-editor-toolbox::tests::a_package_is_downloaded_once_and_installed_into_each_project`), 4 ignored. **Re-run in isolation: PASS (0.00s)** — confirmed parallel-race flakiness. |
| `cargo test --locked -p axcenity-render -- --test-threads=1` | **PASS** | **422 passed**, **0 failed**, 6 ignored (231.56s headless GPU suite) |
| `cargo test --workspace --doc --locked --exclude axcenity-project` | **PASS** | 0 failed (note: 0 doc-tests exist across workspace crates) |
| `cargo run --locked -p axcenity-testing --bin axcenity-test -- examples/hello-character` | **PASS** | **3 passed, 0 failed** (`tests/walk.spec.lua`) |

## 3. Evaluation by Test Category

| Category | Status | Assessment |
|---|---|---|
| **Unit Tests** | Strong (5,579 `#[test]`) | Broad coverage across all 47 crates, but **currently broken on `main`** (`project` compile error, 2 `script` failures, 1 flaky `editor-toolbox` test). |
| **Integration Tests (`tests/*.rs`)** | Uneven | Strong in `avatar` (11 files), `testing` (3), `updater` (2), `anim` (2), `mcp` (1). Absent in `editor-model-studio`, `editor-ui-designer`, `editor-node-graph`, `editor-tools`, `editor-toolbox`, `editor-benchmark`, `plugins`, `project`, `player`, `release`. |
| **Luau Game Tests (`axcenity-test`)** | Implemented | Clean isolated world runner with JUnit XML export (`crates/testing/src/junit.rs`). Verified passing on `examples/hello-character`. |
| **GPU & Visual Regression Tests** | Implemented | 422 headless GPU tests in `axcenity-render` pass sequentially (`--test-threads=1`). Known flakiness under parallel GPU contention (`every_example_renders` black frames, tracked in `wiki/Roadmap/Technical-Debt.md`). |
| **Fuzzing & Security Tests** | Weak | `collab/src/wire_fuzz_tests.rs` runs 20k seeded mutation rounds, and `axcenity-bots` has `--flood`/`--cheat`. **Zero `cargo-fuzz`/libFuzzer harnesses** for `net`, `relay`, `collab` Yjs, `project` unpackers, or `assets` parsers; no adversarial host→client tests in `collab`. |
| **Performance Benchmarks** | Implemented (Local-Only) | `benchmarks/baselines.json` + `axcenity-bench-check` + S10k + XB10k cross-engine suite (`benchmarks/cross-engine/`). Baselines last recorded `2026-11-11` at `15965da` on Ryzen 9 9950X3D / RX 9070 XT (S10k player 7.12 ms, test 3.75 ms, server 3.85 ms; XB10k Axcenity 5.14 ms vs Unity 3.11 ms vs Godot 10.27 ms). **No recent benchmark run recorded for `8f4c2420`**. |
| **CI & Cross-Platform Builds** | **Deficient** | GitHub Actions CI is manual-only (ADR-0018) and **currently broken** (`.github/workflows/ci.yml:69` references removed `--features hot-patch`). macOS builds are not verified per change; Linux/Windows verified locally only. |
