# Findings

Every issue the audit found. IDs are permanent (QA-001…). **Severity**: Critical, High, Medium, Low ([README.md](README.md#method)). **State**: Open · Fixed (with the commit or PR) · Won't fix (with the reason) · Needs Paul.

| ID | Severity | Area | Finding | Found by | State |
|---|---|---|---|---|---|
| QA-001 | Medium | Start screen | On a Linux desktop without an XDG portal or zenity, Browse… (New Project) and Open Project… silently do nothing (rfd logs the failure only), and New Project's location couldn't be typed, so no project could be made or opened | Run (GUI), cloud | Fixed: Location is a text field beside Browse…, and a project folder dropped on the start screen opens |
| QA-002 | Medium | New Project | On a first run there is no location, so Create Project is greyed out with no reason given; a typed or remembered location that no longer exists failed with a raw `could not write …: No such file or directory (os error 2)` | Run (GUI), cloud | Fixed: defaults to the most recent project's folder, else `Documents/Axcenity Projects`; missing folders are made; a line says what's missing |
| QA-003 | Low | Command line | `axcenity-player --help` and `axcenity-server --help` print `Error:` and exit 1; `axcenity-editor --help` tries to open a window | Run (CLI), cloud | Fixed: all three print their usage (`axcenity_release::help_flag`) |
| QA-004 | Low | Docs | Getting started says samples are in the engine's `examples` folder; they are in `crates/samples/projects/` | Inspected | Open |
| QA-005 | Medium | Building | A part inserted from the ribbon has `BlueprintTexture` on, drawn as a dark grid whose lines take its Color: a new grey part looks almost black, and changing Color barely changes it, until Play shows the real colour. Confusing for a beginner (Properties and viewport disagree) | Run (GUI), cloud (lavapipe) | Needs Paul: a deliberate design (docs/editor/building.md) |
| QA-006 | Low | Templates | The Baseplate template has no SpawnLocation, so the first Play of every new project warns "Player has no enabled SpawnLocation…" | Run (GUI), cloud | Fixed: a 6 × 1 × 6 SpawnLocation at the centre (no ForceField) |
| QA-007 | Low | Layout | The default layout's Console is 15% of the height: one line of output shows at 1600 × 900 | Run (GUI), cloud | Fixed: 20% (three or four lines) |
| QA-008 | Low | Dialogs | The New Project dialog re-centres when an error line appears, so its buttons jump under the pointer | Run (GUI), cloud | Open |
| QA-009 | Medium | Accessibility | Unticked checkboxes are almost invisible in every theme (box fill 1.2:1 against the panel; WCAG 1.4.11 asks 3:1), so many settings read as plain text; slider rails too | Run (GUI), cloud | Fixed: `Palette::toggle_fill` (3:1 in every theme, tick still readable), test `checkboxes_and_slider_rails_reach_3_to_1_in_every_theme` |
| QA-010 | Low | Settings | Text & density's code-size slider and Viewport's Grid strength slider looked about 40 points long beside a wide gap | Run (GUI), cloud | Not a bug: the rest of the rail was invisible (QA-009); full length once fixed |
| QA-011 | Low | Open Project | Opening a folder that isn't a project said only "there's no axcenity.json", even for the usual mistake of picking the folder that holds the projects; a moved project said the same | Run (GUI), cloud | Fixed: says the folder isn't there, or isn't a project and names up to five projects inside it; test `opening_a_folder_of_projects_names_the_projects_inside` |
| QA-012 | Medium | New Project | A new Baseplate project copies 77 MB of assets (62 MB is the character library) and its first open cooks every one of the 22 character skins (163 MB cache, about 40 s on the cloud CPU) before the editor is usable | Run (GUI), cloud | Open |
| QA-013 | Low | Start screen | After quitting with the window's close button, the recent project's card shows the "MF" initials placeholder instead of a thumbnail | Run (GUI), cloud | Open |
| QA-014 | Low | Tests | Under plain `cargo test` (no nextest), `app::autorecover_tests::the_recover_prompt_draws_and_escape_keeps_it` overflows the 2 MB test-thread stack and aborts the whole editor test binary; nextest runs each test on a main thread (8 MB), so it passes there | Automated, cloud | Open |

## Details

