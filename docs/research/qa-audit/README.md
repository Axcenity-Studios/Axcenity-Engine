# Whole-engine QA and usability audit (2026-11-27)

Paul's brief of 2026-11-27: an end-to-end assessment of Axcenity from a technical and a first-time-user point of view, with fixes, regression tests and a final report. This folder is the persistent record, so the work can resume in another session.

| File | What it holds |
|---|---|
| [README.md](README.md) | Method, environment, status legend, how to resume |
| [checklist.md](checklist.md) | The feature inventory and testing checklist, one row per feature, with its status and how it was checked |
| [findings.md](findings.md) | Every issue found (QA-nnn), its severity, and what was done |
| [progress.md](progress.md) | Session log: what ran, results, what is next |
| [report.md](report.md) | The final assessment (written last) |

## Baseline

- `main` at `cf132127` (2026-11-26, "Continue Render Pipeline Fixes"), version `0.3.0-alpha.1`.
- Work branch: `qa-audit`. Nothing is pushed without Paul's go-ahead.

## Method

Each feature in [checklist.md](checklist.md) is checked in the strongest way available, and the row says which:

| Method | Meaning |
|---|---|
| **Run (GUI)** | The real editor or player was launched and driven by mouse and keyboard (in the cloud: Xvfb with Mesa's lavapipe software Vulkan, driven with `xdotool`, checked by screenshots) |
| **Run (CLI)** | A real binary was run from the command line (`axcenity-test`, `axcenity-server`, `--build-game`, `--bench`) |
| **Automated** | Covered by the engine's own tests, which were run and passed |
| **Inspected** | Only the source was read. Never counted as passed at runtime |

Severity follows the brief: **Critical** (crash, data loss, security, unusable core), **High** (broken feature, major workflow interruption), **Medium** (usability, inconsistency, noticeable performance), **Low** (polish).

## Status legend

Passed · Passed after fix · Partially functional · Failed · Not implemented · Unable to verify · Not started

## Environment limits

- **Cloud container** (4 vCPU, 15 GB RAM, Linux, no GPU, no audio device, no network to most asset sites). GPU work runs on lavapipe, which is correct but slow, so frame times there say nothing about real hardware. No sound plays, so audio is checked through its tests and logs only.
- **Paul's computers** (Windows PC with an RX 9070 XT; a Linux machine with the checkout at `~/Documents/VSCode Projects/Axcenity-Engine`; a MacBook for CPU work) can be used only when Paul writes in the thread to connect them. Real-GPU, real-audio, multi-computer and native file-dialog checks wait for that, and the checklist marks them **Unable to verify** until then.
- Paul's own projects (`Documents\Axcenity Projects`) are read-only. Tests use temporary copies of the samples under `crates/samples/projects/` and new projects in a scratch folder.

## How to resume

1. Read [progress.md](progress.md) from the bottom: the last entry says what ran and what is next.
2. Pick the first row in [checklist.md](checklist.md) whose status is **Not started**, or the open findings in [findings.md](findings.md) by severity.
3. Cloud setup: `apt-get update && apt-get install -y libdbus-1-dev libasound2-dev libudev-dev pkg-config libxkbcommon-x11-0 mesa-vulkan-drivers xvfb xdotool imagemagick`, then `CARGO_INCREMENTAL=0 cargo build --workspace --all-targets`. No `cargo-nextest` there: run tests per crate with `cargo test -p <crate>`. The GUI harness is [`gui-harness.sh`](gui-harness.sh).
