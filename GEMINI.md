# Axcenity Engine — Antigravity Guide

This file configures Antigravity for Axcenity Engine and points directly to the repository's existing authoritative documentation without duplicating it.

## Start here (read before non-trivial changes)

1. **[CLAUDE.md](CLAUDE.md)** — Core architecture invariants, build/test/lint commands, and topic-doc discipline.
2. **[AGENTS.md](AGENTS.md)** — Engineering wiki workflow, ADR rules, and when to update `wiki/` + run `python scripts/check-wiki.py`.
3. **[docs/README.md](docs/README.md)** — Map of per-change topic docs under `docs/`. Open only the single topic file that matches the task.
4. **[wiki/Architecture/System-Inventory.md](wiki/Architecture/System-Inventory.md)** & **[ADR Index](wiki/Architecture/Architecture-Decisions/ADR-Index.md)** — System directory, crate ownership, and accepted architectural decisions. Never silently override an accepted ADR.
5. **Crate guidance** — Read `crates/<name>/CLAUDE.md` for every crate you touch.

## Non-negotiable invariants

- `player` must not depend on `editor`, `egui`, or `blocks` (ADR-0001).
- `render` only consumes draw data; never depend on `Instance`s or scripts (ADR-0003).
- `World` is single-threaded with a generational-ID arena; workers never touch it (ADR-0002).
- `unsafe_code = "deny"` workspace-wide (only `crates/script/src/debugger.rs` is exempt).
- Keep desktop-first (Windows + Linux primary) cross-platform behavior; pin new dependencies in workspace `Cargo.toml` and run `cargo hakari generate` after dependency changes.

## Local verification & tooling notes

GitHub Actions is intentionally off except for release tags (ADR-0018); local checks are the only verification:

- Search: `git grep` works everywhere (`rg` may not be installed on all machines).
- Format: `cargo fmt --all`
- Targeted tests: `cargo nextest run -p <crate> [filter]` (fall back to `cargo test` if `cargo-nextest` is absent)
- Workspace lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Full tests: `cargo nextest run --workspace` and `cargo test --workspace --doc`
- Visual UI snapshots: set `AXCENITY_SNAPSHOTS=<dir>` when running editor UI tests and inspect the resulting PNGs.
- Wiki check (after wiki edits): `python scripts/check-wiki.py`

## Existing runbooks & skills

Detailed implementation, research, and verification runbooks already live in `.claude/skills/`:

- Implementation & phase workflow: `.claude/skills/implement/SKILL.md` (and `references/verification.md`, `references/repo-gotchas.md`, `references/phase-workflow.md`)
- Research & audits: `.claude/skills/research/SKILL.md`
- Repo & doc organization: `.claude/skills/organize/SKILL.md`
