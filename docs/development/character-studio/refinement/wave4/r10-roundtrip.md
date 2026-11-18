# Hand-back: R10, the Game Editor round trip, OPEN-21, OPEN-22 and R10's tests (`r10-roundtrip`)

Branch `r10-roundtrip` from `main` (waves 1 to 3, PR #427). RA1 with the lead's part delegated: plan 11.5 R10's remaining work (brief §11, §12, §15, §18 Stage 10 "Complete Game Editor integration"), CO13, MAP-S10-08, OPEN-21 and OPEN-22. RED run: [r10-roundtrip-red-run.txt](r10-roundtrip-red-run.txt).

## What it does

1. **The round trip** (MAP-S10-08). The Explorer's right-click menu on a character, its body or one of its parts offers **Edit in Character Studio** (`character_workspace/game.rs`'s `editable_character`: a Model with a Humanoid and a skinned body that keeps a description, or that draws one of the engine's character models; an imported character isn't offered, since Done would put the engine's body in place of its own). The studio opens on the character's description (the kept one; for a character made before descriptions, the stylized default character named after it) in place of the Game Editor's panels. Its toolbar shows **Cancel** in Home's place and **Done** beside Save; Save still saves a copy into the studio's characters. Done gives the project's library what the character wears (from the studio's library: both families and a person's own items), applies the description with the Character Runtime's applier and records it as **one undo step**, "Edit <name> in Character Studio" (`History::record_world`). Play shows it because the description is kept in the character. If applying fails the studio stays open and says why. While a game's character is open, the studio's autosave is off and its recovery file is left alone. The title is "<character> - Character Studio - <project>".
2. **OPEN-21: edit mode shows described characters** (`app/edit_characters.rs`). Each edit frame (never during Play) the editor steps a `DescriptionSync` of its own with no game system: it applies the kept descriptions this run hasn't applied, which registers the identity in the editor's `Assets` and puts on the parts. It then poses the described characters **at rest** (the A-pose, the studio's Still) with the identity worn live (ADR-0041). Each character is copied into a World of its own, its Animators are emptied (no state machine; Animations, IKControls, Ragdolls and FaceControls removed), `anim`'s Animators pose it at `dt` 0, and the poses are mapped back to the editor's Instances for the viewport (`ViewportScene::poses` in edit mode). The parts that follow the body are placed on it as Play places them. Posing runs again only when a described character's Instances change (a hash of their revisions) or a model is still loading. What edit mode applies or places is done `World::unrecorded`, so Collaborate doesn't send it. For a world saved after its description was applied this changes nothing in the world, so nothing becomes unsaved (tested).
3. **OPEN-22: both families in every project, the realistic body the default game character** (decision 7; `crates/project/src/game_character.rs`):
   - `add_default_game_character(project, shipped)` writes the built-in stylized package, copies the engine's shipped packages (`add_both_character_libraries`; the folder is found by `shipped_character_library`: the working folder, then a folder above the program), then writes the `Character` prefab as the realistic character (`described_character`: the default humanoid's Model with a `HumanoidDescription` named `HumanoidDescription` in its Humanoid, `Base` `axcenity/realistic`, `Data` the family's default character from `game_character_description`).
   - New projects from the character templates (`create_project_from`) and the editor's Insert > Character use it. Insert > Character also applies the description at once, in the insert's undo step, named like the instance.
   - Without the shipped folder the prefab stays the stylized character. Files already there are kept, so an older project keeps its prefab and its characters and only gains the realistic family. The stylized family stays selectable in the studio (New, the Body page's Family chips).
   - The editor's `find_character_library` now asks `project::shipped_character_library` first, then a debug build's source tree.
4. **R10's Done-when tests.** Insert, edit, return, play (below). Randomize on both shipped families over every scope, with locks: see "Tests". What existed: `shipped_library.rs`'s `a_thousand_random_characters_on_each_shipped_base_are_sound` (1,000 seeds per base, `Scope::All` only, one parameter lock, realistic only) and `avatar`'s unit tests in `tests/randomizing.rs` (fixtures, every scope, locks). What was missing and is added: every scope on the shipped libraries, both families, presets as starting points, a different lock set each seed, and slot locks.
5. **Save and open polish found while doing this:**
   - **The studio's Ctrl+Z, Ctrl+Shift+Z and Ctrl+S were taken by the Game Editor first.** `EditorApp::frame` consumed Ctrl+S (`self.save()`) and ran the Game Editor's shortcuts (Ctrl+Z, Ctrl+Y, Delete, tools…) before the workspace drew. Over a project this would have undone or saved the game's world. While the workspace is open the Game Editor's save and shortcuts are now skipped. Tested with a project (`the_studios_keys_reach_it_inside_the_editor`). By reading the code, the same consumption happened with no project open, so the studio's Ctrl+Z and Ctrl+S were dead in the real app (the studio's own tests drive `Intent`s, so they never saw it). The gate covers that case too, but I didn't write a no-project test.
   - **Save could delete another session's recovery file.** `save_reporting` always removed `recovery/character.json`, even in a workspace that never autosaves (the start page's painter, and now a game's character). It now removes the file only when the workspace autosaves.
   - The round trip never reads, offers or writes the studio's recovery file (`cancel_leaves_the_game_character_as_it_was` checks the file never appears).

## Interfaces

- `axcenity_project`: `DEFAULT_GAME_BASE`, `KEPT_DESCRIPTION`, `shipped_character_library()`, `add_both_character_libraries(project, shipped) -> Library`, `game_character_description(&Library) -> Option<Description>`, `described_character(world, position, &Description) -> InstanceId`, `add_default_game_character(project, shipped) -> &'static str`. `add_default_humanoid` is unchanged (the studio folder and the capture stage still use it).
- Editor: `character_workspace::game::{GameLink, editable_character, game_description}`, `CharacterWorkspace::open_for_game`, `CharacterWorkspace::game()`; `Intent::Done`, `Intent::Cancel` (in `intent.rs`, a lead contract file: two variants added, nothing changed); `WorkspaceAction::Done`, `WorkspaceAction::Cancel`; `explorer::Action::EditInCharacterStudio`.
- No change to `avatar`'s schemas, the description format or any enum in a file. `architecture.md` §15 and §16 describe the default game character and edit mode.

## Files changed

- New: `crates/project/src/game_character.rs`, `crates/editor/src/app/character_round_trip.rs`, `crates/editor/src/app/edit_characters.rs`, `crates/editor/src/app/round_trip_tests.rs`, `crates/editor/src/character_workspace/game.rs`.
- Changed:
  - Project: `crates/project/src/{lib.rs, templates.rs}`.
  - Editor: `crates/editor/src/{app.rs, explorer.rs, character_workspace.rs}`, `crates/editor/src/app/{character_workspace.rs, session.rs, tree_ops.rs, chrome/panes.rs}`, `crates/editor/src/character_workspace/{intent.rs, shell.rs, toolbar.rs}`.
  - Tests: `crates/avatar/tests/shipped_library.rs` (a test added), `crates/player/src/default_humanoid_tests.rs` (see "Tests changed").
  - `Cargo.toml` (dev profile, see "Measurements").
  - Docs: `docs/editor/character-studio.md`, `docs/development/character-studio/architecture.md`, `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md`, `crates/editor/CLAUDE.md`, `crates/project/CLAUDE.md`.
- No dependency added. No generated file regenerated (none touched).

## Tests written and run

- `axcenity-project` `game_character::tests` (5):
  - the engine's library is found from a test;
  - a new project has both families and a realistic described `Character` prefab that validates;
  - without the shipped library the prefab stays stylized;
  - an older project keeps its prefab byte for byte and gains the realistic family;
  - a described character keeps its description in its Humanoid.
- `axcenity-editor` `app::round_trip_tests` (8):
  - Insert > Character is realistic with both families, applied, one undo step;
  - an older project (hello-character) keeps its character's whole tree and its prefab and gains the realistic family;
  - **insert, edit, return, play**: Edit in Character Studio from the Explorer's test; Done and Cancel drawn and no Game Editor menus; `fat` 0.8 in the studio; Done; one step labelled "Edit Character in Character Studio"; undo and redo; Play: the game's kept description has `fat` 0.8, the body's `Identity` key is the resolved identity's, and its pose is morphed;
  - Cancel changes nothing, adds no step and leaves the studio's recovery file alone;
  - a character without a description opens on the stylized family named after itself, and an imported one isn't offered;
  - OPEN-21: a saved described character reopened in a new run is unregistered before a frame; after one edit frame its identity is registered, the body is posed with its shapes, every part follows, the world isn't unsaved, and a second frame doesn't pose again;
  - OPEN-21 drawn on the GPU (below);
  - the studio's keys inside the editor.
- `axcenity-avatar` `shipped_library::every_scope_on_each_shipped_family_draws_sound_characters_and_keeps_its_locks`:
  - For each family, 1,000 seeds spread over everything, each of the six categories, every feature and a parameter list.
  - Starting points: the family's default character and each of its presets.
  - Locks: about one parameter in seven and one slot, different each seed.
  - Every character validates and resolves with no problem; locked parameters and the locked slot never change; All, Body and Face draw something new.
- Summary lines:
  - RED: `13 tests run: 2 passed, 11 failed` (the file explains each).
  - GREEN, after the changes: `cargo nextest run -p axcenity-editor -p axcenity-project -p axcenity-avatar -p axcenity-gameplay -p axcenity-player -p axcenity-testing --no-fail-fast`: `Summary [ 206.933s] 2081 tests run: 2081 passed (18 slow), 31 skipped`, `every_example_renders` among them (38 s). No ABORT lines.
  - After the last edit (edit mode's changes made unrecorded): `113 tests run: 113 passed` (round trip and the workspace's tests).
  - `cargo clippy -p axcenity-editor -p axcenity-project -p axcenity-avatar -p axcenity-player --all-targets -- -D warnings`: clean. `cargo fmt --all` run. `python3 -I scripts/check-wiki.py`: `114 pages, 0 errors, 58 warnings` (the warnings were there before).
- The earlier full run had two failures that weren't mine:
  - `axcenity-testing tests::tests_leave_the_project_files_alone_even_when_they_save_and_destroy` failed under load and passes alone (6.5 s).
  - `app::collab_repo_tests::closing_the_editor_uploads_what_was_saved` (hello-part, no character) failed twice: "Auto" checkpoint found where "Closing" was expected. It passed in the final run (79 s). It is a timing test on a project my change doesn't touch.

### Tests changed deliberately (they depended on the stylized default)

- `player`'s `default_humanoid_tests` (`Game::new`, 7 tests failed: parts following the body, sockets, body sliders, an item's own parameters as live weights, `Players:CreateHumanoidModelFromDescription`, a script-changed description). These tests are about the **stylized default humanoid** (its `HUMANOID_PARTS`, sockets and sliders) and built their game from the Baseplate template's starter character. That character now wears the realistic description. `Game::new` now takes the kept description out of the starter character, which leaves the stylized default character underneath, as these tests had before. They test what they always tested; the realistic default is tested by the new tests.
- `every_example_renders`: no change needed (examples carry their own files and don't use the templates).

## Measurements

- **Game tests got slower with the realistic default, so two crates are optimized in debug builds.** The editor's `every_genre_templates_tests_pass_in_the_tests_panel_as_on_the_command_line` timed out at nextest's 120 s. Run alone with no limit it took **269.7 s**: each game test spawns the realistic character, which reads the 40 MB `RealisticParts.glb` (181 shapes) and poses it in unoptimized `assets` and `anim`. With `[profile.dev.package.axcenity-assets]` and `axcenity-anim` at `opt-level = 3` (`Cargo.toml`, beside the existing per-package lines) it took **51.6 s** alone and 79.7 s in the full run. Every debug build and test reading or posing a model benefits. The cost is that those two crates compile optimized in dev builds. The lead may prefer another remedy, such as a model cache shared across a test run's games; see "At merge".
- Insert > Character in a debug build: about 2 to 3 s the first time in a run, before the profile change. It reads both bodies' models, because the applier refits from the stylized body's skeleton to the realistic one's. This is in the round-trip tests' times: 2.5 to 3.5 s each, project creation included.
- A new project now copies the realistic package, 40.4 MB, into `assets/characters/realistic/`.

## Visual evidence and what I saw

Sheet: [r10-roundtrip/edit-mode-before-now-play.jpg](r10-roundtrip/edit-mode-before-now-play.jpg) (each frame also on its own beside it). The test `edit_mode_draws_the_described_character` makes a project, inserts the character, sets `fat` 0.8 and `muscle` 0.3 in the studio, presses Done, saves, then reopens the project in a new run. It renders the character offscreen from the front three-quarter (480 × 640):

- **Before** (the editor as it drew before this change, no edit-mode step): the base realistic body, lean, at its bind pose. Its parts are heaped at its feet as a torn, inside-out tangle of black, white and red surfaces, because nothing placed them. The underwear isn't on the body. This is OPEN-21 as reported, and worse: the parts aren't only unshaped, they're misplaced.
- **Now** (edit mode): the heavy body with a belly, fuller face, arms and thighs, wearing the black boxer briefs at the waist. It stands in the A-pose and casts a shadow on the baseplate. 17.9% of the frame's pixels differ from Before (a channel moving by more than 15).
- **Play** (0.5 s in, idling): the same body, face and briefs in the Idle pose. It differs from edit mode by 11.9%, all of it pose (arms down, weight shift). Cropped faces side by side (edit, play) show the same face: eyes, brows and the fuller cheeks and jaw are identical. The skin and briefs shade the same.
- Honestly: the A-pose is the rest pose, not a natural standing pose. A scene full of edit-mode characters will look like a fitting room until Play. Posing at Idle's first frame would read more natural but would load clips in edit mode; I left that for the lead's call.

## Open issues

- **Randomize draws draft items.** Plan 11.3 says drafts never enter defaults, presets or randomization. `avatar::randomize` doesn't look at `quality`, and every stylized item has none, so all count as drafts by the contract. Filtering drafts would leave the stylized family with nothing to wear when randomized. Not changed here: the lead's contract call.
- **A slot a parameter's choices fill can't be locked** (the realistic underwear follows the frame): by contract. The studio offers no slot locks; the new test leaves those slots out.
- Edit mode poses only described characters, and only at rest: no Idle, no expression, and accessories on sockets aren't placed (only Play's Animators place sockets). A character without a description still draws its unposed model, as before.
- An older project's players stay stylized: its `Character` prefab is never rewritten (decision: "existing projects keep what they use"). A person who wants the realistic players must replace the prefab, for example by inserting a character, editing it and saving it as the prefab. No one-click way yet.
- **Cancel discards the studio's changes without asking.**
- Collaborate: edit mode's parts are made unrecorded (local IDs). A later recorded Done on the same character edits those parts, and I haven't tested how a session sends edits to Instances with local IDs.
- The studio's Ctrl+Z and Ctrl+S with no project: fixed by the same gate, untested (see "What it does", item 5).
- Not in this task and still open in R10: "comparison complete" and user presets and favourites beyond R1's.
- `audit-plan-status.md`'s rows OPEN-21, OPEN-22 and MAP-S10-08 still say missing or partial; I didn't edit the audit, which is the lead's record.

## For the lead at merge

- Generated files: none touched or regenerated.
- `Cargo.toml`'s two new `[profile.dev.package.*]` entries: keep them, or replace them with your remedy for the game tests' time. Without them, `every_genre_templates_tests_pass_in_the_tests_panel_as_on_the_command_line` times out under nextest.
- `intent.rs` gained `Intent::Done` and `Intent::Cancel`; `shell.rs` maps them to new `WorkspaceAction`s. Other branches that add `Intent` variants will conflict there, in one line each.
- `CharacterWorkspace` gained one field (`game`) and its line in `open`'s struct literal: a one-line conflict with any branch that adds a field.
- Tracks that create projects from templates in their tests now get the realistic starter character. A test about the stylized default humanoid should take the kept description out, as `player`'s `default_humanoid_tests` now does.
- `progress.md` and the audit rows (OPEN-21, OPEN-22, MAP-S10-08) are yours to update.
