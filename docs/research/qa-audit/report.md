# QA, usability and stability audit: final report (2026-11-27)

Paul's brief of 2026-11-27. Branch `qa-audit` from `main` at `cf132127` (version `0.3.0-alpha.1`). The record behind this report: [checklist.md](checklist.md) (every feature checked and how), [findings.md](findings.md) (QA-001 to QA-024), [progress.md](progress.md) (the session log), [README.md](README.md) (method and how to resume).

## 1. Executive summary

Axcenity's core workflow works from start to finish. In the real editor, driven by mouse and keyboard, a first-time user can make a project, build a scene, import a model, write a script, play, save, reopen and build a game that runs. None of the GUI runs crashed. Recovery is solid: a project killed with `kill -9` comes back through AutoRecover, and damaged project files open with a clear message instead of failing.

The audit found 29 issues: 2 High, 7 Medium and 20 Low. None are Critical (crash, data loss, security). Of the 29:

- 20 are fixed on the branch, each with a regression test where code changed.
- 1 is partly fixed (QA-013).
- 2 are not product bugs (QA-010, and QA-019, a test-environment limit).
- 3 need Paul's decision because they are deliberate designs (QA-005, QA-012, QA-024).
- 3 are open. They are tests that fail only under load, and they failed on `main` too (QA-025, QA-028, QA-029).

The full test suite (`cargo nextest run --workspace`, 5,680 tests) was then run on Paul's Linux computer (RX 9070 XT):

| Run | Failed | Timed out |
|---|---|---|
| `main` | 15 | 1 |
| The branch, after the fixes it prompted | 1 | 1 |

Both remaining failures are on `main` too, and both pass when run alone.

The two High issues were both on a beginner's first path:

- **Close Project could hang forever** while waiting for a thumbnail screenshot that never came (QA-021).
- **A brand-new project's first Build Game listed 84 false Asset Check problems** (QA-022). The default character's animation file was checked as if it were a broken mesh, so a profile set to "stop on problems" would have blocked every new project's first build.

The biggest limit of this audit: it ran in a cloud container with software Vulkan (lavapipe), no audio device and one computer. Real-GPU look and speed, sound, two-computer networking and native file pickers on Windows and macOS are **Unable to verify** until a run on Paul's machines. Section 10 lists them.

## 2. Inventory

The engine is 48 workspace crates. What was checked, grouped as in [checklist.md](checklist.md):

| Area | Rows | What it covers |
|---|---|---|
| 0 Baseline | 7 | Build, warnings, clippy, tests, doc tests, wiki check, format |
| 1 Launch and start screen | 7 | First launch, pages, New Project, Open Project, Recent, templates |
| 2 Editor shell | 9 | Menus, ribbon, shortcuts, Ctrl+K search, panels, window sizes, toasts, tooltips, quitting |
| 3 Scene and objects | 9 | Insert, select, transform tools, Explorer, Properties, undo, copy and paste, Levels, Prefabs |
| 4 Viewport | 4 | Camera, view cube and overlays, picking and gizmos, viewport bar |
| 5 Assets | 6 | Import, bad files, Assets panel, Asset Check, Toolbox, Materials |
| 6 Play mode | 5 | Play/Stop, script errors, repeated Play, multi-client Play, F9 console |
| 7 Scripting | 7 | Script tabs, runtime errors, loop watchdog, Blocks, debugger, hot reload, Find in Scripts |
| 8 Systems | 7 | Physics, audio, animation, UI Designer, Terrain, Character Studio, Model Studio |
| 9 Settings | 8 | Every page, persistence, bad values, reset, restart notes, project settings, Input, damaged file |
| 10 Save and recovery | 6 | Save and reopen, AutoRecover, Version History, Back Up, damaged files, read-only folder |
| 11 Build and export | 3 | Build Game, the built game, other OS targets |
| 12 Networking | 4 | Host and join, dedicated server, Collaborate, two computers |
| 13 Beginner workflow | 12 | The brief's twelve steps |

## 3. Coverage and method

Each row says how it was checked, from strongest to weakest:

- **Run (GUI):** the real editor was launched and driven in Xvfb with xdotool, and screenshots were checked ([gui-harness.sh](gui-harness.sh)).
- **Run (CLI):** a real binary was run: `axcenity-server`, `axcenity-test`, `--build-game`, the built game, and the MCP server's `asset_import`.
- **Automated:** the engine's own tests were run and passed.
- **Inspected:** source only. **Inspected was never counted as passed.**

Test data was always temporary: copies of `crates/samples/projects/`, new projects in scratch folders, and a fresh settings folder per run. Paul's projects and computers weren't touched.

Results over the checklist's 94 rows:

| Status | Rows |
|---|---|
| Passed | 57 |
| Passed after fix | 12 |
| Partially functional | 5 |
| Failed | 0 |
| Not implemented | 0 |
| Unable to verify | 5 |
| Not started | 15 |

How the 74 finished rows were checked: 37 by a GUI run, 29 by passing tests only (marked **Automated**), and the rest by the command line. The 15 **Not started** rows are left for the run on Paul's Linux computer: selection and transform tools by hand, gizmos, the viewport bar, tooltips, quitting with unsaved changes, copy between projects, multi-client Play, hot reload, settings limits, reset and restart notes, Back Up, and a read-only folder. Their code has passing tests, but they weren't driven by hand.

## 4. Passed features

At runtime in the GUI, these passed:

- First launch.
- New Project, after its fix.
- Open Project with an empty folder, a folder of projects, a damaged `axcenity.json` and a newer format.
- Recent projects across a restart.
- The File menu.
- Ctrl+S, Ctrl+Z, Ctrl+D, Delete and F2.
- Inserting parts.
- Explorer rename, duplicate, delete and undo.
- Properties: filter, refused text in number fields, limits.
- Every Settings page.
- Play and Stop. Stop restores the world, and three runs kept memory flat at 2.31 GB.
- A script made, typed, saved and run in Play.
- Import of OBJ through the GTK picker.
- Save and reopen.
- AutoRecover after `kill -9`.
- Version History checkpoint and restore.
- A damaged world file.
- Build Game for this OS.
- All twelve beginner steps (two after fixes).

From the command line, these passed:

- Script errors with file and line.
- The 3-second loop watchdog.
- `axcenity-test` on the character sample.
- The headless dedicated server with encryption.
- `--build-game`.
- Import of glTF, GLB, PNG and WAV.
- Refusal of damaged files.

Systems without a GUI run are covered by their own tests, which passed. The coverage table above has the counts.

## 5. Failed and partly working features

Nothing is left **Failed**. **Partially functional**:

| Row | What works | What doesn't, or wasn't seen |
|---|---|---|
| 1.5 Recent projects | Open, persistence | Quitting by the window's close button doesn't wait for a thumbnail, so the card shows initials (QA-013) |
| 2.1 Header menus | File menu items | Edit, View, Debug and Help were opened but not every item run |
| 7.1 Script tabs | New, type, save, run | Completion and diagnostics need luau-lsp, a 5 MB download not taken offline |
| 11.2 The built game | It starts and runs its script | Its window stayed black on lavapipe for the minute watched; check on a real GPU |
| 13.8 Project settings | The page opens | Not changed in the end-to-end run |

**Unable to verify** in the cloud: host and join (12.1). The joiner reached "Joining", then the container ran out of memory with two software-GPU players.

## 6. Bugs found, by severity

Full wording in [findings.md](findings.md).

| Severity | IDs |
|---|---|
| Critical | none |
| High | QA-021 Close Project hang, QA-022 Asset Check false problems on a new project |
| Medium | QA-001 no way to pick a folder without a system picker, QA-002 New Project's unexplained grey button and raw error, QA-005 new parts look black (blueprint grid), QA-009 invisible checkboxes in every theme, QA-014 12 editor tests overflow the test stack on `main`, QA-018 damaged images and sounds imported silently, QA-020 a damaged settings file reset every setting silently |
| Low | QA-003, QA-004, QA-006, QA-007, QA-008, QA-010, QA-011, QA-012, QA-013, QA-015, QA-016, QA-017, QA-019, QA-023, QA-024, QA-025, QA-026, QA-027, QA-028, QA-029 |

## 7. Fixes and quality-of-life changes

| ID | Change | Regression test |
|---|---|---|
| QA-021 | Close Project asks for a repaint and gives up on the thumbnail after 3 s | `close_project_goes_ahead_when_the_thumbnail_screenshot_never_comes` |
| QA-022 | Asset Check checks a clip's source for its take only; an animations-only model isn't "unreferenced" | `every_template_passes_asset_check` (every template) |
| QA-018 | Images and sounds are checked before they're copied in | `a_damaged_image_or_sound_is_refused_and_not_copied` |
| QA-020 | A damaged `app.ron` is kept as `app.ron.damaged`, with a Console message and a toast | `a_damaged_settings_file_is_kept_and_reported` |
| QA-009 | Checkboxes and slider rails reach 3:1 contrast in every theme (`Palette::toggle_fill`) | `checkboxes_and_slider_rails_reach_3_to_1_in_every_theme` |
| QA-001, QA-002 | New Project's location can be typed, has a sensible default, missing folders are made, and a line says what's missing; a project folder dropped on the start screen opens | start screen tests |
| QA-008 | New Project's buttons stay put when a message line appears | `new_projects_buttons_stay_put_when_a_line_appears` |
| QA-011 | Open Project names the projects inside a folder of projects, or says the folder is gone | `opening_a_folder_of_projects_names_the_projects_inside` |
| QA-023 | Build Game says so when no folder picker can open, and gives the `--build-game` command | `a_picker_that_stayed_open_a_while_was_cancelled_not_missing` |
| QA-003 | `--help` prints usage for the editor, player and server | release crate tests |
| QA-006 | The Baseplate template has a SpawnLocation | `every_template_makes_a_project_that_loads_cleanly` |
| QA-007 | The default Console is 20% of the height | layout tests |
| QA-015 | Transparency, PlaybackSpeed, Volume, RollOff and others are limited to useful ranges | `transparency_is_limited_to_0_to_1` |
| QA-016 | "about 4 minutes ago", "1 document" | `the_age_of_a_copy_reads_as_words` |
| QA-017 | Version History shows local time | used by its tests |
| QA-014 | 12 editor tests that overflowed the 2 MB test stack (11 Collaborate tests and AutoRecover's prompt; they failed on `main` under nextest on Paul's Linux computer) run on a big stack | themselves |
| QA-004 | Getting started names the real samples folder; GitHub links follow the renamed account | docs |

The audit also added a per-test runner ([run-each.sh](run-each.sh)) for machines without nextest.

## 8. Files changed

59 files, about 1,300 lines added on `main`'s `cf132127`, in 8 commits on `qa-audit`:

- **Engine code** (22 files): `assets/src/import.rs` and `texture_pack.rs`; `editor-kit/src/theme.rs`; in `editor/src/`: `app.rs`, `main.rs`, `layout.rs`, `start.rs`, `start/dialogs.rs`, `properties/values.rs`, and in `app/`: `autorecover.rs`, `session.rs`, `version_history.rs`, `files/export.rs`, `files/import.rs`, `files/pickers.rs`; `player/src/main.rs`, `server/src/main.rs`; `project/src/templates.rs` and `validate.rs`; `release/src/lib.rs` and `build_info.rs`.
- **Tests** (10 files): `project/src/tests.rs`; in `editor/src/`: `start/tests.rs`, `properties/tests.rs`, and in `app/`: `tests.rs`, `autorecover_tests.rs`, `settings/tests.rs`, `files/tests.rs`, `plugin_tests.rs` (fake images replaced by real ones after QA-018).
- **Docs**: `docs/editor/` (`assets.md`, `import.md`, `selection.md`, `shell.md`), `docs/project/` (`export.md`, `releases.md`, `templates.md`), the user docs in `documentation/` (Getting started, the editor interface, GitHub links), and the wiki (Testing strategy, Engineering changelog).
- **The audit's record**: `docs/research/qa-audit/` (this report, the checklist, findings, the progress log, the GUI harness and `run-each.sh`).

`git diff --stat cf132127..qa-audit` has the full list.

## 9. Test results

All runs in the cloud container:

| Check | Result |
|---|---|
| `cargo build --workspace --all-targets` | Clean, 0 warnings |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| `cargo fmt --all --check` | Clean |
| `python3 scripts/check-wiki.py` | 124 pages, 0 errors, 59 warnings (old `verified_commit`s, as before) |
| `cargo test --workspace --doc` | Passed (the workspace has no doc tests) |
| The other 45 crates, `cargo test -p <crate>` | All pass: about 3,370 tests, e.g. script 419, project 246, assets 227, scene 234, anim 198, collab 175, physics 158 |
| Editor, one test per process | 1,597 tests: all pass except the 3 cloud-only failures in section 10 |
| Render, one test per process | 119 passed before the run was stopped; the rest would not fit in memory on lavapipe |
| Player | Not run in the cloud: too large on lavapipe |

**On Paul's Linux computer** (Fedora 45, Ryzen 9 9950X3D, RX 9070 XT, 60 GB). The suite ran in a separate worktree, so Paul's checkout wasn't touched. Command: `cargo nextest run --workspace --no-fail-fast`, about 160 s per run. Build: 0 warnings.

| Commit | Run | Passed | Failed | Timed out |
|---|---|---|---|---|
| `main` `cf132127` | 5,667 | 5,651 | 15 | 1 |
| `qa-audit` `3b02a628` | 5,679 | 5,664 | 14 | 1 |
| `qa-audit` `ab3c6e91` | 5,680 | 5,678 | **1** | **1** |

What the first two runs found:

- **The branch caused one regression.** A player test still expected the character on the bare baseplate (QA-006's spawn pad). It's fixed.
- **11 Collaborate tests overflowed the stack on `main` too.** They're fixed (QA-014).
- **A secured host on port 0 could fail with "Address already in use".** It's fixed (QA-026).

What's left in the last run:

- **`the_overlay_is_drawn_only_where_the_feature_shows` failed.** It fails under load on `main` too, and passes alone (QA-028).
- **`human_garments_and_hair_stay_steady_in_the_idle` timed out.** It takes 112 s alone against the 120 s limit (QA-027). `.config/nextest.toml` now gives it 4 minutes.

## 10. Remaining issues and what couldn't be verified

**Needs Paul's decision:**

- **QA-005:** new parts look almost black in edit mode because of the blueprint grid. Possible changes: a lighter grid, or blueprint off for parts from the ribbon.
- **QA-012 and QA-024:** a new project copies 77 MB, its first open cooks 22 skin textures (about 40 s on the cloud CPU), and a one-part game builds to 136.5 MB. Both come from shipping the whole default character library (ADR-0050). Copying or cooking it on first use would shrink both.

**Open, low:**

- **QA-013:** quitting by the window's close button doesn't save a fresh thumbnail.
- **QA-023's wider case:** the editor's other pickers (Import, textures, sky, Back Up) still do nothing silently on a Linux desktop with no portal and no zenity.
- **QA-016's rest:** Collaborate messages still say "change(s)".

**Unable to verify in the cloud:**

- **Real GPU:** the look, speed, ray tracing and the built game's window on the RX 9070 XT. Lavapipe ran 4 to 24 FPS, which says nothing about real hardware.
- **Audio playback:** there is no sound device in the container.
- **Networking:** host and join, Collaborate, two computers, LAN and Tailscale.
- **Native pickers:** the Windows and macOS file dialogs.
- **Build Game for another OS** (11.3).
- **Editor tests that failed only in the cloud:** `every_tutorial_sample_opens_and_plays_in_the_editor` (no audio device) and `ffmpeg_encodes_every_format_it_offers` (the container's ffmpeg rejects an AV1 parameter). Both pass on the Linux computer.

**Recommended next run** (on Paul's Linux machine at `~/Documents/VSCode Projects/Axcenity-Engine`, or the PC):

1. Open the beginner project and Build Game on a real GPU, and run the built game.
2. Host and join on one computer.
3. Play a Sound.
4. The rows still **Not started** in [checklist.md](checklist.md).

## 11. Recommendations

1. **Decide QA-005 and QA-012/QA-024.** They're what a beginner meets first: a black part, and a large, slow first project and build.
2. **Run the full suite on a real machine before merging** (`cargo nextest run --workspace`), since the cloud couldn't run render and player in full.
3. **Give every system picker one fallback.** Route them through one helper that says when no picker opened (the QA-023 check), and offer a typed path where it matters (Import, Back Up).
4. **Keep the test suite runnable without nextest.** Twelve tests overflowed the 2 MB test stack (under nextest too) and one depended on AccessKit being on. The wiki's Testing strategy page now says what plain `cargo test` needs.
5. **Add a "first five minutes" test.** Make a Baseplate project, insert a part, Play, Stop, Build Game, and assert no warnings. Two High findings would have been caught by one test like that; `every_template_passes_asset_check` is the start of it.

## 12. Ratings

Out of 10, from what this audit ran. Anything left unverified pulls a rating toward caution.

| Area | Rating | Why |
|---|---|---|
| Functionality | **7.5** | Every step of the beginner workflow works in the real editor, and the feature set is very broad: scripting, blocks, terrain, animation, UI, Collaborate, building games. Two High bugs sat right on the first-project path. Some areas were checked only by their tests. |
| Stability | **7** | No crash in any GUI or CLI run. AutoRecover after `kill -9` works, damaged project files open with a message, and the script watchdog works. The full suite passes on a real GPU, apart from two load-sensitive tests. Against that: one hang (QA-021), a damaged settings file wiped silently (QA-020), and 11 tests aborting on `main`. Networking wasn't checked by hand. |
| Beginner friendliness | **6** | The Learn page, templates, plain-language errors and a working first Play are strong. A new part looks black until Play (QA-005). A new project is 77 MB with a 40 s background cook, and a one-part game is 136 MB. Pickers can fail silently on some Linux desktops. |
| UI/UX | **6.5** | The design system is consistent and panels fit narrow columns. But unticked checkboxes were nearly invisible in every theme (QA-009), a dialog's buttons jumped (QA-008), and the ribbon and settings are dense for a newcomer. |
| Performance | **6** (low confidence) | Not measurable on lavapipe. Earlier PC benchmarks put the XB10k frame at 5.14 ms, slower than s&box (2.05) and Unity (3.11) but faster than Godot (10.27). Memory stayed flat over repeated Play at 2.31 GB. The first open's texture cook and build sizes are heavy. |
| Settings | **7** | Every page opens and responds, the input settings reset, and a damaged file is now kept and reported. Limits, reset to default and restart notes weren't checked by hand. |
| Code quality | **8** | Clean build, clippy and format. About 5,000 tests, behaviour-named and driving real paths. Thorough docs, a wiki and ADRs. The suite runs in under 3 minutes on a 32-thread machine. Minus points: very large modules, 15 tests failing on `main` with nobody noticing (no CI), and a suite that needs plenty of memory. |
| Production readiness | **5** | It's an alpha (`0.3.0-alpha.1`), and the label is fair. The core loop is dependable, but audio, multiplayer, other-OS builds and the built game on a real GPU haven't been checked by hand, builds are large, and there's no CI beyond release tags, so every check depends on local runs. |
