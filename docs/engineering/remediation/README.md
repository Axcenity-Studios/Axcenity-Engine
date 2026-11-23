# Axcenity Engine — Remediation Plan Overview (`8f4c2420`)

This directory contains the prioritized, evidence-based remediation plan and phased TDD implementation roadmap derived from the [`docs/engineering/audit/`](../audit/README.md) findings at commit `8f4c2420`.

**No engine source code has been modified.** Implementation awaits owner review and approval.

## Documents

| Document | Purpose |
|---|---|
| [REMEDIATION_PLAN.md](REMEDIATION_PLAN.md) | Complete prioritized register (`P0`–`P3`) with 12 mandatory fields per item, separating **Confirmed Defects** from **Recommendations & Strategic Opportunities** |
| [TDD_IMPLEMENTATION_ROADMAP.md](TDD_IMPLEMENTATION_ROADMAP.md) | 6-phase RED/GREEN/REFACTOR execution plan with all 11 required engineering sections per phase |

## Severity Distribution

| Priority | Meaning | Confirmed Defects | Recommendations / Strategic | Total |
|---|---|---:|---:|---:|
| **P0** | Critical correctness, security, stability, build-breakage, or data-loss | 14 | 0 | **14** |
| **P1** | Foundational architecture, operational reliability, and production-readiness | 8 | 4 | **12** |
| **P2** | Significant missing functionality, workflow gaps, and memory/perf debt | 3 | 6 | **9** |
| **P3** | Optimization, polish, and longer-term platform capabilities | 0 | 4 | **4** |
| **Total** | | **25** | **14** | **39** |

## Recommended Execution Order

1. **Phase 0 (Trunk Restoration & CI Repair)** — Restore green `cargo test --workspace`, `clippy -- -D warnings`, flaky `editor-toolbox` test, and broken `.github/workflows/ci.yml`.
2. **Phase 1 (P0 Trust-Boundary & Security Hardening)** — Close Collaborate `Mirror` path traversal, plugin auto-execution, `UserId` console spoofing, SSL-strip downgrade, `repo` Git token/arg injection, live MCP privilege self-declaration, `Arena::insert_at` OOM, and `debugger.rs` `unsafe` stack checks.
3. **Phase 2 (P0/P1 Data Integrity, Relay & Server Operations)** — Fix `save` `NaN`/`Inf` corruption, `relay` `TcpStream` write race + abuse caps, `Git::show` binary corruption, `axcenity-server` SIGINT/SIGTERM cleanup, `net` `Driven::step` wraparound & float validation, zip-bomb unpacker bounds, and `csg.rs` BSP stack safety.
4. **Phase 3 (Runtime Performance & Memory Bounding)** — Route `physics::query` through Rapier BVH, bound `assets`/`audio` caches + add streaming audio, add `wgpu::PipelineCache` + slim shadow instances, single-serialize `net` broadcasts, and mount `.axgame` in place.
5. **Phase 4 (Editor Decomposition, Character & UI Polish)** — Extract Character Studio out of 257k-LOC `editor`, ship draft filter + synchronized character LODs + Eye shader, fix UI `%` padding & `flex-wrap`, pool ad-hoc worker threads.
6. **Phase 5 (Verification Maturity, Docs & Multi-Machine Scale)** — Refresh stale `wiki/` & `GEMINI.md`, pin/verify CI packaging tools (`nfpm`/`appimagetool`), add `cargo-fuzz` harnesses, and execute real 16–32 player multi-machine soak tests.
