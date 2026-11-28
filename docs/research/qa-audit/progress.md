# Progress log

Newest entry last. Each entry: what ran, the result, what is next.

## 2026-11-27 (session 1, cloud)

- Started from `main` at `cf132127`. Branch `qa-audit`.
- Wrote the plan ([README.md](README.md)), the checklist ([checklist.md](checklist.md)) and the GUI harness ([gui-harness.sh](gui-harness.sh)).
- Baseline build running.
- Baseline: build clean with 0 warnings, `cargo fmt --check` clean, wiki check 0 errors. Every crate's tests run one crate at a time (`cargo test -p <crate>`, no nextest in the cloud). The editor, render, player and editor-benchmark binaries were killed for memory when run beside a GUI editor; they are re-run alone with `--test-threads=4`.
- GUI runs (Xvfb + lavapipe, about 16–24 FPS on a small project): start screen, New Project, Settings pages, Play/Stop, Explorer rename/duplicate/delete/undo, Properties filter and invalid input, save and reopen, AutoRecover after `kill -9`, Version History checkpoint and restore.
- CLI runs: `axcenity-server` on script fixtures (errors with file and line, the 3 s loop watchdog), `axcenity-test` on `character` (3 passed), `--build-game` on `part` and the built game.
- Fixed so far: QA-001, QA-002, QA-003, QA-006, QA-007, QA-009, QA-011, QA-014, QA-015, QA-016, QA-017. Commits on `qa-audit`; nothing pushed.
- Next: finish the re-run of the four big crates, clippy, then Import (glTF/OBJ/images), scripting tabs, Collaborate and host/join on one computer, the beginner end-to-end run, and the report.

## 2026-11-27 (session 1, continued)

- The editor's tests, one per process ([run-each.sh](run-each.sh)): 1,597 pass except three that fail only in the cloud (no audio device, lavapipe, the container's ffmpeg). Render: 119 passed before memory ran out. Player: not run in the cloud.
- `cargo clippy --workspace --all-targets -- -D warnings` clean; doc tests pass (there are none); format and wiki check clean.
- Found and fixed QA-018 to QA-023 (import content check, damaged settings file, Close Project hang, Asset Check on a new project, Build Game's missing picker). QA-005, QA-012 and QA-024 wait for Paul.
- Checklist: 74 of 94 rows done; 15 Not started, left for a run on a real machine.
- Wrote [report.md](report.md).
- Paul connected his Linux computer (Fedora, Ryzen 9 9950X3D, RX 9070 XT) and approved pushing `qa-audit` and opening a PR. A separate worktree there (`~/Documents/VSCode Projects/Axcenity-Engine-qa`; his checkout is left alone) runs `cargo nextest run --workspace`.
- Linux computer, `cargo nextest run --workspace`, about 160 s per run. `main`: 5,667 run, 15 failed, 1 timed out. Branch at `3b02a628`: 14 failed, 1 timed out, one of them a regression from the Baseplate pad. Fixed: the pad test, the 11 team tests' stacks (QA-014) and the port-0 race (QA-026), and the garment checks got 4 minutes (QA-027). Branch at `ab3c6e91`: 5,678 of 5,680 passed; the overlay test (QA-028) and the garment timeout fail on `main` too and pass alone.
- Next: the 15 Not started rows there; real-GPU checks of the beginner project and the built game; host and join on one computer.
