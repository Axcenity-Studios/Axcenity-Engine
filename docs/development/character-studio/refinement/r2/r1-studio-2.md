# Hand-back: RA1, R1 step 2 part 2 (`r1-studio-2`)

Branched from `refinement-r1-wave1` (6267cb98). Plan: section 0 decision 7, [section 11](../../../../../plans/axcenity-engine-character-studio.md) (11.3 presets, randomizer and editor-only state; 11.5 R1's "Done when"). Brief §11 and §12. Audit: [audit-studio-ui.md](../audit-studio-ui.md); patterns: [references/ux/patterns.md](../../references/ux/patterns.md) P13, P15, P17, sections 5 and 6. Contract: architecture.md §15a to §15c.

## What changed

1. **Both families selectable** (decision 7). `offers_base` now offers every base the library has. `presets::families` lists them with realistic first. New lists each family, then its whole-character presets, or its default character when it has none (the stylized family has no presets yet). The Body page starts with **Family** chips: the current family is lit, and picking another starts again on that body as one undo step. Stylized controls come from its own manifest. Its groups stand in for features in the navigation, in Randomize's feature scope and in the levels, through the fallbacks already in `selection.rs`.
2. **Presets from data.** The six code presets (`Preset`, `presets()`) are gone.
   - A new character starts from `Start::Family(base)` or `Start::Preset(id)` (`CharacterWorkspace::open_new`, `new_character`).
   - `Intent::ApplyPreset(String)` applies a preset by ID with `avatar::apply_preset`, as one undo step through `edit`.
   - The Body and Face pages start with their kind's presets as **Start from** chips. A body preset tagged `male` or `female` shows only on that body type. The matching chip is lit, and **Custom** is lit when none matches (`matches_preset`).
   - The **Presets** page groups the base's presets by kind (Characters, Bodies, Faces, then Hair, Outfits and Skin once they exist) as **picture tiles**. The tiles come from the existing thumbnails painter, which draws any description. A part preset is drawn on its base's default character, and a face preset is framed on the face (`Subject::Face`). Favourites come first and have a star.
   - **Save as preset** takes a kind and a name and writes `presets/<slug>.preset.json` in the workspace: a format 2 description with a `preset` block, ID `my/<slug>`, holding only what the kind owns. A body preset leaves out the body type and is tagged with it. An existing file is never overwritten (`-2`, `-3`…). The preset is listed alongside the others and checked with `check_preset`.
   - **Favourites** are kept in `favourites.json` beside `recent.json` (`Intent::ToggleFavourite`).
   - The start page and the thumbnails cache now address presets by ID: `thumbnails/presets/<id with - for />.<key>.png`, where the key hashes the drawn description and its framing. `StartAction::NewCharacter` carries a `Start`.
3. **Randomize** (`random.rs`). The toolbar has a Randomize menu with Everything, the open category and the picked feature. Each choice says how many locked values it keeps. It calls `avatar::randomize`; the feature scope is `Scope::Parameters` of the parameters whose studio feature it is, so a stylized group works too.
   - Every slider row and the body type have a lock toggle. A mirrored row locks both sides. `frame` is locked by default.
   - Locks are editor state (`Intent::SetLock`): no undo step and nothing in the description.
   - The seed is a press counter mixed with an FNV hash of the character, never the clock.
   - One press is one undo step.
4. **Comparison** (`compare.rs`, CO19). A strip over the viewport has **Hold as A/B/C** (up to three), chips to show A, B or C or the Current character, **Side by side**, **Keep X** and Discard.
   - A held version shown in place of the character uses the same viewport, camera and light. The panel is disabled and the viewport says "Showing A (held): editing is off…". An edit intent sent anyway first goes back to Current.
   - Side by side puts a second character in the preview World, 30 studs to the side, with its own floor. It is applied by the same applier and drawn in the left half with the same orbit camera translated to it, so the camera stays in sync.
   - Keep makes the version the character as one undo step and keeps the current name.
   - Held versions never enter undo or the file, and starting another character drops them.
5. **Autosave** (`autosave.rs`, CO20).
   - While there is unsaved work (a change since the save, or since a new character started), `recovery/character.json` is written, outside `characters/`.
   - It is written at most every 3 s, never during a drag, and never while a recovery is being offered.
   - Save removes it, and so do undoing back to the saved state and a clean close (Drop, which skips panics and only removes what that workspace wrote).
   - The next open asks **Recover the unsaved character?** Recover brings the character back unsaved, to its own file if it had one. Discard removes the file.
   - `autosave_step(now)` takes the clock, so tests drive it directly.
   - The start page's picture painter turns autosave off.
6. **Audit D2** (`project::add_character_packages`): a shipped package's files (manifest, model, groom files, thumbnails, presets) are copied again when they differ from the shipped ones, by size or by bytes. The catalogue entry is refreshed. A project's own files, and an ID the project lists at a different path, are left alone. The function returns the IDs added or updated.

Small related changes: `stage::add_floor_at`, and `stage::apply` now colours every floor. `Intent::edits()` was added. The status line and labels say what is shown.

## Interfaces

- **`character_workspace`:**
  - `Start { Family(String), Preset(String) }` (Default is the realistic family; `Start::shipped(name)` exists in tests only)
  - `CharacterWorkspace::open_new(&Path, &Start)`, `new_character(&Start)`, `use_preset(&str)`, `presets_of(&Library, PresetKind)`, `own_presets()`, `save_as_preset(PresetKind, &str)`, `toggle_favourite(&str)`
  - `randomize(RandomScope)`, `set_lock(&[String], bool)`
  - `capture()`, `show_held(Option<usize>)`, `set_side_by_side(bool)`, `keep_held(usize)`, `discard_held(usize)`, `shown()`
  - `autosave_step(Instant)`, `unsaved_work()`, `recover(bool)`
  - Public fields: `favourites`, `locks`, `comparison`, `recovery`
  - Removed: `Preset`, `presets()`, `open_preset`
- **`Intent`:** `ApplyPreset(String)`, `New(Start)` (both were `usize`), plus `SaveAsPreset {kind, name}`, `ToggleFavourite`, `Randomize(RandomScope)`, `SetLock {ids, locked}`, `Capture`, `ShowHeld`, `SideBySide`, `KeepHeld`, `DiscardHeld`, `Recover(bool)`.
  - `intent.rs` is a lead-owned contract file (11.7). This task required these variants ("through an Intent"), so they were added. Please review them as the contract.
- **Modules:** `presets` (`Listed`, `all`, `of_kind`, `find`, `suits`, `families`, `read_own`, `make`, `save_own`, `read_favourites`, `write_favourites`, `KINDS`), `random` (`RandomScope`, `scope_for`, `default_locks`), `compare` (`Comparison`, `MAX_HELD`, `OTHER_AT`), `autosave` (`RECOVERY_FILE`, `AUTOSAVE_EVERY`, `Recovery`, `read_recovery`, `remove_recovery`).
- **`thumbnails`:** `Subject::{Character, Face}` (`Preset(usize)` was removed), `preset_picture(folder, id, &Subject) -> PathBuf`, `preset_subject`, `face_orbit`.
- **Editor glue outside my files**, the minimum to move from indexes to IDs: `start.rs` (`StartAction::NewCharacter(Start)`), `app/session.rs` (one match arm), `app/character_workspace.rs` (`open_character_workspace(Option<Start>, …)`).
- **`project`:** `add_character_packages` keeps its signature; its behaviour changed as described above.

## Tests

The red run is in [r1-studio-2-red-run.txt](r1-studio-2-red-run.txt).

- D2 failed against the old copier. Its summary line: `2 tests run: 1 passed, 1 failed`.
- The editor tests use the new API, so against the base branch they don't compile.
  - For a red run that fails at runtime, each feature's entry point was switched off with an early return and `offers_base` was put back to the old rule. Then everything was switched back on.
  - Result: `12 tests run: 1 passed, 11 failed`. The one that passed only checks that the lock icons are drawn.
  - Two tests first failed because New on an unsaved new character asks "Save changes?". The tests now answer Don't Save, and the red run was repeated with each failure at its feature.

New tests:

- **`feature_tests.rs`** (9 tests):
  - a stylized character opened, edited, randomized by group, saved and reopened identical, and moved across families as one step
  - body and face presets, one step each, with Custom and the page chips
  - Save as preset (kind filtering, tags, no overwrite, another workspace sees it)
  - favourites
  - Randomize: scopes, default and slider locks, one step each, repeatable across workspaces
  - lock icons drawn
  - held versions never in undo or the file, side by side drawing a second character, Keep as one step, max three
  - autosave timing, drag, Save, undo and clean close
  - crash recovery offered (drawn), not written over while offered, Recover to its file, Discard
- **`walkthrough_tests.rs`**: `the_whole_r1_walkthrough_with_presets_randomize_comparison_and_recovery`, which runs through the task's item 7 list.
- **`tests.rs`**: `every_new_character_choice_is_sound_on_both_families` replaces the code-preset soundness test.
- **`project`**: `an_older_copy_of_a_shipped_package_is_brought_up_to_date`.

Updated tests (same assertions, new API):

- **`tests.rs`**: the stylized family is now *offered*, which reverses an assertion for decision 7.
- **`shelf_tests.rs`**: picture keys by ID and framing.
- **`start/characters/tests.rs`**: seven start cards (six realistic presets and the stylized default); Pick by `Start`.
- **`shell_tests.rs`**: the Body Quick page expects "Athletic", "Custom" and "Family", where it used to expect the old "Athletic female" chip. With `AXCENITY_SNAPSHOTS` it also draws the Presets page with real pictures, side by side, a held version shown, the stylized Body and Face pages, and the recovery question.

Runs:

- `cargo nextest run -p axcenity-editor -p axcenity-project --no-fail-fast` (full): `1594 tests run: 1593 passed, 1 failed, 10 skipped`.
  - The failure is `app::mcp::tests::the_open_editor_keeps_drawing_while_a_toolbox_download_runs`, which this branch doesn't touch. It passed when run again on its own (below), so it looks like a timing flake under load.
- After a last change to Drop, the studio, start page, D2 and that MCP test: `85 tests run: 85 passed`.
- `cargo clippy -p axcenity-editor -p axcenity-project --all-targets -- -D warnings`: clean. `cargo fmt --all`: clean.
- `python scripts/check-wiki.py`: `113 pages, 0 errors, 58 warnings` (the warnings were there before).

## Visual evidence

The snapshots come from `AXCENITY_SNAPSHOTS=<folder outside the repo> cargo nextest run -p axcenity-editor every_page_is_drawn`. They were not committed because the status line shows a local path. What I saw:

- **Body (Detailed):**
  - Family chips (Realistic lit, Stylized).
  - Start from chips (Slim, Average, Athletic, Muscular, Heavyset, Lean, Broad), with Custom lit on the default character, which matches none of them.
  - Body type with a lit lock, and an open lock on every slider row, beside the value.
- **Presets:**
  - The Characters tiles show real pictures: Male, Female, Athletic and Heavyset, each in two variants, with Male outlined as applied. Bodies show only the male builds.
  - Honest note: at about 100 px, the full-body framing makes the body builds hard to tell apart (Slim, Average and Athletic look nearly alike). A tighter crop, or a torso framing for body presets, would make them readable; I didn't change the picture framing.
  - The stars appear only on hover or for a favourite, so none show in the snapshot.
- **Side by side:**
  - Two halves labelled "A (held)" and "Current". The current one has body fat 0.9, a visibly heavier belly and waist.
  - Same framing and light in both halves. The strip shows Hold as B, A lit, Side by side, and Keep A.
- **Showing A:** the slim A is in place of the heavy current one, the panel is greyed out and the label says editing is off.
- **Stylized Body:** Family with Stylized lit; Frame (a masculine-to-feminine slider, locked by default), Build and Proportions (only Height at Detailed); the stylized figure is dressed.
- **Recover:** the modal "Recover the unsaved character?" over a dimmed studio, with Discard and Recover.

I did not run the editor interactively, so the menus (New, Randomize), hovering and the colour of the star were checked only by the drawing tests and by reading the code.

## Generated files

None regenerated: no asset, manifest, preset, catalogue or benchmark changed.

## Dependencies

None added.

## Open issues

- **The stylized family has no presets.** New and the start page offer its default character, and its Body and Face pages show no Start from chips. Making some would mean touching `assets/characters/humanoid/` and the catalogue scripts, which this task didn't name.
- **No hair, outfit or skin presets are shipped.** The Presets page lists those kinds once they exist, and Save as preset already offers them where the body has those slots or materials.
- **Quitting with "Don't Save"** relies on eframe dropping the app to remove the recovery file. If the app isn't dropped at exit, the next open would offer work the person chose to throw away. The workspace and Home paths drop it explicitly. A line in `app/documents.rs`'s `discard_unsaved` (not my file) would make this certain.
- **New on an untouched new character still asks "Save changes?"** (existing behaviour, which `new_and_open_ask_to_save_unsaved_changes_first` relies on). Autosave uses the narrower `unsaved_work()`, so an untouched character is never written to the recovery file.
- **Side by side** draws two canvases, so two renders per frame; this isn't measured. Side by side is also off when nothing is held.
- **Comparison scope:** Hold has no keyboard keys, and the strip has no amount (Subtle, Medium, Bold) or mix slider (patterns P14 and P15's amount).
- **Body preset tiles are small** (above).
- **`architecture.md` §15c** should list the new intents and editor state (comparison, locks, favourites, recovery). It is the lead's file and I left it alone.

## For the lead at merge

1. Review the new `Intent` variants as the contract (`intent.rs`), and add them, `Start` and the editor-only state to `architecture.md` §15c.
2. Conflicts are likely in `character_workspace.rs`, `panel.rs`, `toolbar.rs`, `shell.rs` and `start/characters.rs` if another RA1 branch touched them, and in `docs/editor/character-studio.md`.
3. `add_character_packages` now compares bytes: on each studio open, it reads both copies of the 32 MB realistic model once when their sizes are equal. That cost isn't measured; if it shows, compare a hash stamp instead.
4. Progress entry: R1's comparison, autosave, presets by ID, Randomize UI and both families are done; D2 is fixed.
