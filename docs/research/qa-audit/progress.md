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
