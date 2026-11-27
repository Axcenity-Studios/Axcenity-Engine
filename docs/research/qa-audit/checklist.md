# Testing checklist

One row per feature. **Status**: Passed · Passed after fix · Partially functional · Failed · Not implemented · Unable to verify · Not started. **How**: Run (GUI) · Run (CLI) · Automated · Inspected (see [README.md](README.md#method)). **Where**: Cloud (Xvfb + lavapipe), PC, Linux box, Mac. Findings are in [findings.md](findings.md).

## 0. Baseline

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 0.1 | `cargo build --workspace --all-targets` | Not started | | |
| 0.2 | Compiler warnings | Not started | | |
| 0.3 | `cargo clippy --workspace --all-targets -- -D warnings` | Not started | | |
| 0.4 | Every crate's tests | Not started | | |
| 0.5 | Doc tests | Not started | | |
| 0.6 | `python3 scripts/check-wiki.py` | Not started | | |
| 0.7 | `cargo fmt --all --check` | Not started | | |

## 1. Launch and start screen

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 1.1 | First launch with no saved settings | Not started | | |
| 1.2 | Start screen pages (Home, Recent, All projects, Learn, Documentation, Characters, Team Projects) | Not started | | |
| 1.3 | New Project dialog: templates, name, location, validation, Enter/Esc | Not started | | |
| 1.4 | Open Project: valid folder, empty folder, not-a-project, damaged `axcenity.json`, newer format | Not started | | |
| 1.5 | Recent projects: open, remove, missing folder | Not started | | |
| 1.6 | Project settings dialog from the start screen | Not started | | |
| 1.7 | User templates: Save as Template, import, delete | Not started | | |

## 2. Editor shell

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 2.1 | Header menus: File, Edit, View, Debug, Help, Build Game | Not started | | |
| 2.2 | Ribbon tabs and their buttons | Not started | | |
| 2.3 | Keyboard shortcuts (Ctrl+S, Ctrl+Z/Y, Ctrl+D, Del, F2, F5, Ctrl+K, tool keys) | Not started | | |
| 2.4 | Universal search / command palette (Ctrl+K), Quick Open | Not started | | |
| 2.5 | Panels: show/hide, drag, dock, float, Reset Layout, saved layouts | Not started | | |
| 2.6 | Window resize, small window, large window | Not started | | |
| 2.7 | Toasts, Console/Output messages, error wording | Not started | | |
| 2.8 | Tooltips on ribbon, toolbar and Properties | Not started | | |
| 2.9 | Close project / Quit with unsaved changes | Not started | | |

## 3. Scene and objects

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 3.1 | Insert parts and objects (ribbon, Explorer context menu) | Not started | | |
| 3.2 | Select, multi-select, box select | Not started | | |
| 3.3 | Move / Scale / Rotate tools, snapping, typed values | Not started | | |
| 3.4 | Explorer: rename, reparent by drag, duplicate, delete, group, search | Not started | | |
| 3.5 | Properties: edit every field type, invalid input, reset, multi-edit | Not started | | |
| 3.6 | Undo / redo across tools, Undo History | Not started | | |
| 3.7 | Copy / paste between projects | Not started | | |
| 3.8 | Levels: new, switch, rename, delete | Not started | | |
| 3.9 | Prefabs: make, place, apply, revert, unpack | Not started | | |

## 4. Viewport

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 4.1 | Camera: orbit, pan, fly, zoom, focus (F) | Not started | | |
| 4.2 | View cube, grid, guides, overlays, View Mode | Not started | | |
| 4.3 | Picking and gizmos | Not started | | |
| 4.4 | Viewport bar (FPS, quality) | Not started | | |

## 5. Assets

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 5.1 | Import: glTF/GLB, FBX, OBJ, images, audio | Not started | | |
| 5.2 | Import of a missing, empty or corrupt file | Not started | | |
| 5.3 | Assets panel: browse, search, rename, delete, drag into the scene | Not started | | |
| 5.4 | Check Assets: missing references, fixes | Not started | | |
| 5.5 | Toolbox (online sources) | Not started | | |
| 5.6 | Materials: Material tab, custom materials | Not started | | |

## 6. Play mode

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 6.1 | Play, Pause, Stop; edits discarded on Stop | Not started | | |
| 6.2 | Play with a script error | Not started | | |
| 6.3 | Play repeatedly (start/stop many times) | Not started | | |
| 6.4 | Multi-client Play | Not started | | |
| 6.5 | Developer console (F9) | Not started | | |

## 7. Scripting

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 7.1 | New Script, script tab, save, completion, diagnostics | Not started | | |
| 7.2 | Runtime errors reported with file and line | Not started | | |
| 7.3 | Infinite loop watchdog | Not started | | |
| 7.4 | Blocks editor and Graph view | Not started | | |
| 7.5 | Debugger (breakpoints, step) | Not started | | |
| 7.6 | Hot reload in Play | Not started | | |
| 7.7 | Find in Scripts | Not started | | |

## 8. Physics, audio, animation, UI, terrain

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 8.1 | Physics: falling parts, anchored, joints, collision groups | Not started | | |
| 8.2 | Audio: Sound objects, Mixer | Not started | | |
| 8.3 | Animation tab: clips, record, play | Not started | | |
| 8.4 | UI Designer: ScreenGui, widgets, live preview | Not started | | |
| 8.5 | Terrain Studio: sculpt, paint, generate | Not started | | |
| 8.6 | Character Studio workspace | Not started | | |
| 8.7 | Model Studio | Not started | | |

## 9. Settings

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 9.1 | Every Settings page opens and its controls respond | Not started | | |
| 9.2 | Settings persist across a restart | Not started | | |
| 9.3 | Invalid values (empty paths, extreme numbers) | Not started | | |
| 9.4 | Reset to default | Not started | | |
| 9.5 | Restart-required settings say so | Not started | | |
| 9.6 | Settings > This project (project settings) | Not started | | |
| 9.7 | Input window (File > Input…) | Not started | | |
| 9.8 | Damaged editor settings file | Not started | | |

## 10. Save, load and recovery

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 10.1 | Save, reopen, everything kept | Not started | | |
| 10.2 | AutoRecover after a kill | Not started | | |
| 10.3 | Version History, checkpoints, restore | Not started | | |
| 10.4 | Back Up Project | Not started | | |
| 10.5 | Damaged `world.json` / script files on open | Not started | | |
| 10.6 | Read-only project folder | Not started | | |

## 11. Build and export

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 11.1 | Build Game for this OS (`--build-game` and the menu) | Not started | | |
| 11.2 | The built game runs | Not started | | |
| 11.3 | Build Game for another OS | Not started | | |

## 12. Networking

| # | Check | Status | How / where | Notes |
|---|---|---|---|---|
| 12.1 | Host and join on one computer (player) | Not started | | |
| 12.2 | Dedicated server, headless | Not started | | |
| 12.3 | Collaborate session on one computer | Not started | | |
| 12.4 | Two-computer LAN / Tailscale | Not started | | |

## 13. Beginner end-to-end workflow

| # | Step | Status | How / where | Notes |
|---|---|---|---|---|
| 13.1 | Launch for the first time | Not started | | |
| 13.2 | Create a new project | Not started | | |
| 13.3 | Create and configure a scene | Not started | | |
| 13.4 | Import assets | Not started | | |
| 13.5 | Create and manipulate objects | Not started | | |
| 13.6 | Configure properties | Not started | | |
| 13.7 | Add game behaviour (script) | Not started | | |
| 13.8 | Configure project settings | Not started | | |
| 13.9 | Run and test | Not started | | |
| 13.10 | Save and close | Not started | | |
| 13.11 | Reopen, check persistence | Not started | | |
| 13.12 | Build / export | Not started | | |
