# Testing checklist

One row per feature. **Status**: Passed · Passed after fix · Partially functional · Failed · Not implemented · Unable to verify · Not started. **How**: Run (GUI) · Run (CLI) · Automated · Inspected (see [README.md](README.md#method)). **Where**: Cloud (Xvfb + lavapipe), PC, Linux box, Mac. Findings are in [findings.md](findings.md).

## 0. Baseline

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 0.1 | `cargo build --workspace --all-targets` | Passed | Run (CLI), cloud | Clean build at `cf132127` |
| 0.2 | Compiler warnings | Passed | Run (CLI), cloud | 0 warnings |
| 0.3 | `cargo clippy --workspace --all-targets -- -D warnings` | Passed | Run (CLI), cloud | 0 warnings with `-D warnings` |
| 0.4 | Every crate's tests | Passed after fix | Automated, cloud and Linux computer | Cloud: every crate one at a time (editor one test per process). Linux computer, `cargo nextest run --workspace` on the RX 9070 XT: `main` 15 failed and 1 timed out; the branch after fixes 5,678 of 5,680 passed. The two left (QA-027, QA-028) fail on `main` too and pass alone. Fixes: QA-006's player test, QA-014, QA-026 |
| 0.5 | Doc tests | Passed | Run (CLI), cloud | `cargo test --workspace --doc`: 45 crates, no doc tests in any of them |
| 0.6 | `python3 scripts/check-wiki.py` | Passed | Run (CLI), cloud | 0 errors, 59 warnings (old `verified_commit`s) |
| 0.7 | `cargo fmt --all --check` | Passed | Run (CLI), cloud | |

## 1. Launch and start screen

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 1.1 | First launch with no saved settings | Passed | Run (GUI), cloud | Start screen, no saved settings |
| 1.2 | Start screen pages (Home, Recent, All projects, Learn, Documentation, Characters, Team Projects) | Passed | Automated, cloud | 60 `start::` tests pass; Home, Recent and Learn also seen in GUI runs |
| 1.3 | New Project dialog: templates, name, location, validation, Enter/Esc | Passed after fix | Run (GUI), cloud | QA-001, QA-002; QA-008 open |
| 1.4 | Open Project: valid folder, empty folder, not-a-project, damaged `axcenity.json`, newer format | Passed after fix | Run (GUI), cloud | Empty folder, folder of projects (QA-011), damaged `axcenity.json` (opens empty with a clear message), format 999 (refused clearly) |
| 1.5 | Recent projects: open, remove, missing folder | Partially functional | Run (GUI), cloud | Open and persistence after restart pass; thumbnail placeholder QA-013 |
| 1.6 | Project settings dialog from the start screen | Not started | | |
| 1.7 | User templates: Save as Template, import, delete | Passed | Automated, cloud | `save_as_template_makes_a_template_new_project_lists` and 4 more |

## 2. Editor shell

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 2.1 | Header menus: File, Edit, View, Debug, Help, Build Game | Partially functional | Run (GUI), cloud | File menu items checked (New, Open, Save, Version History, Checkpoint, Import, Build Game, Close Project); Close Project hung (QA-021, fixed). Edit, View, Debug, Help menus opened but not every item run |
| 2.2 | Ribbon tabs and their buttons | Passed | Automated, cloud | 24 ribbon tests, e.g. `every_insertable_on_the_ribbon_says_what_it_does`; Insert and Script tabs used in GUI runs |
| 2.3 | Keyboard shortcuts (Ctrl+S, Ctrl+Z/Y, Ctrl+D, Del, F2, F5, Ctrl+K, tool keys) | Passed | Run (GUI), cloud | Ctrl+S, Ctrl+Z, Ctrl+D, Delete, F2 checked |
| 2.4 | Universal search / command palette (Ctrl+K), Quick Open | Passed | Automated, cloud | command palette tests (fuzzy match, chips, recents, synonyms); `every_ribbon_and_menu_command_can_be_run_from_the_palette` |
| 2.5 | Panels: show/hide, drag, dock, float, Reset Layout, saved layouts | Passed | Automated, cloud | floating-panel tests (tear off, dock back, move) |
| 2.6 | Window resize, small window, large window | Passed | Automated, cloud | 21 narrow-column and responsive tests; the GUI ran at 1600 × 900 only |
| 2.7 | Toasts, Console/Output messages, error wording | Passed | Automated, cloud | `an_error_stays_until_closed_and_a_click_shows_the_console`; toasts seen in GUI runs |
| 2.8 | Tooltips on ribbon, toolbar and Properties | Not started | | |
| 2.9 | Close project / Quit with unsaved changes | Not started |  | |

## 3. Scene and objects

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 3.1 | Insert parts and objects (ribbon, Explorer context menu) | Passed | Run (GUI), cloud | Ribbon Block; new parts land at the end of Workspace. Blueprint look QA-005 |
| 3.2 | Select, multi-select, box select | Not started | | |
| 3.3 | Move / Scale / Rotate tools, snapping, typed values | Not started | | |
| 3.4 | Explorer: rename, reparent by drag, duplicate, delete, group, search | Passed | Run (GUI), cloud | F2 rename, Ctrl+D (copy in place, named Crate2…), Delete, undo |
| 3.5 | Properties: edit every field type, invalid input, reset, multi-edit | Passed after fix | Run (GUI), cloud | Filter properties; text in a number field is refused; QA-015 |
| 3.6 | Undo / redo across tools, Undo History | Passed | Run (GUI), cloud | Undo of delete and of a property |
| 3.7 | Copy / paste between projects | Not started | | |
| 3.8 | Levels: new, switch, rename, delete | Passed | Automated, cloud | 23 level tests, e.g. `a_level_opens_with_the_projects_services_and_saves_to_its_own_file` |
| 3.9 | Prefabs: make, place, apply, revert, unpack | Passed | Automated, cloud | prefab tests: variants, Apply, linked copies |

## 4. Viewport

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 4.1 | Camera: orbit, pan, fly, zoom, focus (F) | Passed | Automated, cloud | camera tests; orbit and zoom used in GUI runs but not checked one by one |
| 4.2 | View cube, grid, guides, overlays, View Mode | Passed | Automated, cloud | overlay tests (constraints, paths, navmesh) |
| 4.3 | Picking and gizmos | Not started | | |
| 4.4 | Viewport bar (FPS, quality) | Not started | | |

## 5. Assets

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 5.1 | Import: glTF/GLB, FBX, OBJ, images, audio | Passed | Run (GUI) and Run (CLI, MCP `asset_import`), cloud | OBJ through File > Import (GTK picker); glTF/GLB, PNG, WAV through the same import code. FBX through the corrupt-asset fixture only |
| 5.2 | Import of a missing, empty or corrupt file | Passed after fix | Run (GUI) and Run (CLI), cloud | Corrupt FBX reported; empty/random `.glb` refused; damaged PNG/WAV were copied in silently (QA-018, fixed) |
| 5.3 | Assets panel: browse, search, rename, delete, drag into the scene | Passed | Automated, cloud | asset_refs tests: rename keeps uses, a taken name refused, Unused filter |
| 5.4 | Check Assets: missing references, fixes | Passed | Automated, cloud | Asset Check tests incl. levels not open and data tables; QA-022 found by Build Game |
| 5.5 | Toolbox (online sources) | Unable to verify | Automated, cloud | toolbox tests pass offline; the real sites are blocked from the cloud |
| 5.6 | Materials: Material tab, custom materials | Passed | Automated, cloud | custom material and Material tab tests |

## 6. Play mode

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 6.1 | Play, Pause, Stop; edits discarded on Stop | Passed | Run (GUI), cloud | Play autosaves first; Stop restores the world |
| 6.2 | Play with a script error | Passed | Run (CLI), cloud | `axcenity-server` on fixtures: syntax error and nil index reported with `scripts/Script.server.lua:2` |
| 6.3 | Play repeatedly (start/stop many times) | Passed | Run (GUI), cloud | Three Play/Stop runs: editor memory 2.31 GB after each |
| 6.4 | Multi-client Play | Not started | | |
| 6.5 | Developer console (F9) | Passed | Automated, cloud | `f9_in_play_opens_the_console_which_runs_timescale_and_the_games_commands` |

## 7. Scripting

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 7.1 | New Script, script tab, save, completion, diagnostics | Partially functional | Run (GUI), cloud | New Script from the Script tab, typed, saved with Ctrl+S, ran in Play. Completion and diagnostics need luau-lsp (the editor offers a 5 MB download; not taken offline) |
| 7.2 | Runtime errors reported with file and line | Passed | Run (CLI), cloud | File and line, with a stack line |
| 7.3 | Infinite loop watchdog | Passed | Run (CLI), cloud | `while true do end` stopped after 3 s with advice to add `task.wait()` |
| 7.4 | Blocks editor and Graph view | Passed | Automated, cloud | 50 block editor tests |
| 7.5 | Debugger (breakpoints, step) | Passed | Automated, cloud | `a_breakpoint_holds_the_whole_frame_until_continue` and 3 more |
| 7.6 | Hot reload in Play | Not started | | |
| 7.7 | Find in Scripts | Passed | Automated, cloud | `find_in_scripts_finds_a_name_in_lua_and_blocks_and_replace_all_undoes_in_one_step` |

## 8. Physics, audio, animation, UI, terrain

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 8.1 | Physics: falling parts, anchored, joints, collision groups | Passed | Automated, cloud | physics crate 158 tests; falling parts seen in Play |
| 8.2 | Audio: Sound objects, Mixer | Unable to verify | Automated, cloud | audio crate 21 tests pass; no sound device, nothing heard |
| 8.3 | Animation tab: clips, record, play | Passed | Automated, cloud | anim crate 198 tests, 64 editor animation tests |
| 8.4 | UI Designer: ScreenGui, widgets, live preview | Passed | Automated, cloud | editor-ui-designer 47 tests, 24 editor designer tests |
| 8.5 | Terrain Studio: sculpt, paint, generate | Passed | Automated, cloud | terrain crate 71 tests, 48 editor terrain tests |
| 8.6 | Character Studio workspace | Passed | Automated, cloud | 197 Character Studio tests (one fails on lavapipe only) |
| 8.7 | Model Studio | Passed | Automated, cloud | editor-model-studio 69 tests, mesh-edit 131 |

## 9. Settings

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 9.1 | Every Settings page opens and its controls respond | Passed after fix | Run (GUI), cloud | Every page opened; checkboxes were nearly invisible (QA-009, QA-010) |
| 9.2 | Settings persist across a restart | Passed | Run (GUI), cloud | Recent projects and the window kept after quitting and starting again with the same settings folder; 21 settings tests. Not every setting checked one by one |
| 9.3 | Invalid values (empty paths, extreme numbers) | Not started | | |
| 9.4 | Reset to default | Not started | | |
| 9.5 | Restart-required settings say so | Not started | | |
| 9.6 | Settings > This project (project settings) | Passed | Run (GUI), cloud | The page opens and its fields show; nothing changed |
| 9.7 | Input window (File > Input…) | Passed | Automated, cloud | `edits_add_and_remove_inputs_and_actions_and_reset` |
| 9.8 | Damaged editor settings file | Passed after fix | Run (GUI), cloud | A truncated `app.ron` reset every setting silently (QA-020, fixed) |

## 10. Save, load and recovery

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 10.1 | Save, reopen, everything kept | Passed | Run (GUI), cloud | Renames and copies kept after quit and reopen; the beginner project reopened from Recent with its part, model and script |
| 10.2 | AutoRecover after a kill | Passed after fix | Run (GUI), cloud | Snapshot after 2 min, `kill -9`, reopen offers Recover; recovered parts back. Wording QA-016 |
| 10.3 | Version History, checkpoints, restore | Passed after fix | Run (GUI), cloud | Checkpoint with a message, restore to an older save (Before restore kept). Times QA-017 |
| 10.4 | Back Up Project | Not started | | |
| 10.5 | Damaged `world.json` / script files on open | Passed | Run (GUI), cloud | Damaged manifest; damaged world opens empty with a message |
| 10.6 | Read-only project folder | Not started | | |

## 11. Build and export

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 11.1 | Build Game for this OS (`--build-game` and the menu) | Passed after fix | Run (CLI) and Run (GUI), cloud | `--build-game` and File > Build Game > For this OS (9.2 s, 136.5 MB). A new project's first build listed 84 false Asset Check problems (QA-022, fixed). No picker, no build from the menu (QA-023) |
| 11.2 | The built game runs | Partially functional | Run (CLI), cloud | The built game starts and runs its script ("Hello world!"); its window stayed black for the minute watched on lavapipe: check on real GPU |
| 11.3 | Build Game for another OS | Unable to verify |  | needs the other OS's player beside the editor |

## 12. Networking

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 12.1 | Host and join on one computer (player) | Unable to verify | Run (CLI), cloud | Host started and the joiner connected far enough to say "Joining", then the host was killed for memory (two software-GPU players plus tests). Needs the Linux box or PC |
| 12.2 | Dedicated server, headless | Passed | Run (CLI), cloud | `axcenity-server` hosts with encryption and reports the UPnP failure in plain words |
| 12.3 | Collaborate session on one computer | Passed | Automated, cloud | collab 175 tests, 59 editor Collaborate tests incl. host hand-off |
| 12.4 | Two-computer LAN / Tailscale | Unable to verify |  | needs two computers |

## 13. Beginner end-to-end workflow

| # | Step | Status | How / where | Notes |
|---|---|---|---|---|
| 13.1 | Launch for the first time | Passed | Run (GUI), cloud | Fresh settings folder: start screen |
| 13.2 | Create a new project | Passed | Run (GUI), cloud | Baseplate, typed name, default location `~/Axcenity Projects`, Enter |
| 13.3 | Create and configure a scene | Passed | Run (GUI), cloud | Lighting > ClockTime slider to night and back (undo) |
| 13.4 | Import assets | Passed | Run (GUI), cloud | File > Import… cube.obj through the GTK picker |
| 13.5 | Create and manipulate objects | Passed | Run (GUI), cloud | Ribbon Block lands on the spawn pad. It looks black until Play (QA-005) |
| 13.6 | Configure properties | Passed | Run (GUI), cloud | Lighting and Part properties |
| 13.7 | Add game behaviour (script) | Passed | Run (GUI), cloud | Script ribbon > Script inside the part; four typed lines; ran in Play ("Spinning Part", colour changes) |
| 13.8 | Configure project settings | Partially functional | Run (GUI), cloud | Settings > This project opened earlier (9.6); not changed in this run |
| 13.9 | Run and test | Passed | Run (GUI), cloud | F5 Play, character spawned on the pad, Stop restored the world |
| 13.10 | Save and close | Passed after fix | Run (GUI), cloud | Saved; Close Project hung (QA-021) until fixed |
| 13.11 | Reopen, check persistence | Passed | Run (GUI), cloud | Reopened from Recent: 6 objects, 1 script |
| 13.12 | Build / export | Passed after fix | Run (GUI), cloud | Build Game > For this OS: built, but with 84 false problems (QA-022, fixed) |
