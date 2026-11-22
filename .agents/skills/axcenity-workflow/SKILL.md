---
name: axcenity-workflow
description: >-
  Project workflow for implementing, verifying, and documenting changes in
  Axcenity Engine. Use when building features, fixing bugs, running checks,
  or updating docs/wiki in this repository.
---

# Axcenity Engine Workflow

Use the repository's existing authoritative docs and runbooks rather than guessing conventions.

## Before coding

1. Read [CLAUDE.md](../../../CLAUDE.md) and [AGENTS.md](../../../AGENTS.md).
2. Map the affected systems in [wiki/Architecture/System-Inventory.md](../../../wiki/Architecture/System-Inventory.md) and check linked ADRs in [wiki/Architecture/Architecture-Decisions/ADR-Index.md](../../../wiki/Architecture/Architecture-Decisions/ADR-Index.md).
3. Open only the matching topic doc listed in [docs/README.md](../../../docs/README.md) and the affected crate's `crates/<name>/CLAUDE.md`.
4. For phased roadmap work or deeper checklists, read the existing runbooks under `.claude/skills/`:
   - `.claude/skills/implement/SKILL.md`
   - `.claude/skills/implement/references/verification.md`
   - `.claude/skills/implement/references/repo-gotchas.md`
   - `.claude/skills/implement/references/phase-workflow.md`

## After coding

1. Update the topic doc in `docs/` when behavior changes.
2. Update `wiki/` (and `wiki/Changelog/Documentation-History.md`) when the change materially affects documented architecture, performance, or limitations, then run `python scripts/check-wiki.py`.
3. Run narrow tests first, then widen:
   - `cargo fmt --all`
   - `cargo nextest run -p <changed-crate>`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo nextest run --workspace` (when practical for scope)
4. If `Cargo.toml` dependencies changed, run `cargo hakari generate`.
