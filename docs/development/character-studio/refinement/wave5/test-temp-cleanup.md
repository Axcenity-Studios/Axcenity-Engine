# Wave 5 hand-back: test temp-folder cleanup

Branch `test-temp-cleanup`, from `refinement-wave5`.

**The problem.** The test suite left its temporary folders in the system temp folder. Over weeks this reached 60,106 folders and 1.4 TB, which filled Paul's disk. The task text gives one earlier full run as 1,087 folders and 33.5 GB.

**The fix.** One small helper, `axcenity_test_dirs::TestDir`, is now used across the workspace. It makes a folder that is removed when it drops, also when a test panics. Setting `AXCENITY_KEEP_TEST_DIRS=1` keeps the folders.

**The result.** After the fix, a full `cargo nextest run --workspace --no-fail-fast` leaves **no folder from a passing test**. What is left comes only from tests that nextest killed at its two-minute timeout, plus one empty folder made by production code. Details are below.

## Measured first: what one full run left (before)

How it was measured: `cargo nextest run --workspace --no-fail-fast` on `refinement-wave5`. `TMP` and `TEMP` were set to a private, empty folder, so the other specialists' parallel builds and tests could not mix into the count. Afterwards, everything in that folder was listed (scratch scripts `measure2.ps1`, `leftovers.py`, `families.py`, outside the repo).

**Before: 342 entries, 5.59 GB**, all named `axcenity-*`, from 5,496 tests. The number is lower than the 1,087 and 33.5 GB in the task text. The player had already gained a self-removing `TempCopy`, so its `axcenity-player-*-<process>-ThreadId(..)` copies no longer appear. The many `axcenity-editor-*` example copies are still there.

| Family | Folders | MB | Made by |
|---|---:|---:|---|
| Editor example copies (`axcenity-editor-<test>-test-<process>`) | 83 | 3,941 | editor `app/tests.rs` `copy_of_example`, `temp_project` (366 call sites; about 60 MB each, the character examples about 220 MB) |
| Collaborate bare remotes (`axcenity-editor-remote-*`) | 8 | 486 | editor `app/collab_repo_tests.rs` `bare_remote` |
| Collaborate second copies (`axcenity-editor-sam-*`, `scenario-d-sam`) | 3 | 365 | editor `app/collab_repo_tests.rs` tests |
| Collaborate remote checks (`axcenity-editor-check-*`) | 3 | 364 | editor `app/collab_repo_tests.rs` `remote_log` and tests |
| Team session data (`axcenity-team-<name>-host/joiner-<process>-<n>`) | 31 | 243 | editor `app/team_tests.rs`, `team_board_tests.rs` `team_folder` |
| Handoff data (`axcenity-collab-handoff-*`) | 2 | 182 | editor `app/collab_handoff_tests.rs` `data_folder` |
| Repo test repositories (`axcenity-repo-<process>`) | 30 | 2 | repo `testing.rs` `temp` (its sweep only ran on Linux, or after a day) |
| Small fixtures (about 60 name patterns) | 156 | 1 | assets (import, fbx, gltf, obj, loading, refs), anim, physics fracture, nav behavior, convert usd, save, script, project (levels, asset refs, collision groups, input), plugins bundle, teamwork, updater download/extract, editor update install, testing, release |
| Debug pictures (`axcenity-*.png`) | 11 | 0.4 | player `clouds_tests.rs`, `time_of_day_tests.rs`; editor `settings/tests.rs`, `graphics_benchmark_tests.rs` |
| Player | 8 | 0.1 | player `tests.rs` (saves, `stripes_project`), `level_tests.rs` |
| luau-lsp checker (`axcenity-luau-<process>-<n>`) | 4 | 0.4 | editor `luau_check.rs` `Checker::start`. Its thread removes the folder, but the test process exits first |
| Collaborate project records, editor saves, git credential store | 3 | 0 | collab `project.rs`; editor `session.rs` `play_save_folder`; repo `credentials.rs`. The credential store's path went through a shell that ate its backslashes, so git wrote a file named `Users...store` in the temp root |

What the families have in common:

- Almost every helper built `temp_dir().join(format!("axcenity-..-{pid}"))`, cleared it, and returned a `PathBuf`.
- Cleanup was a `let _ = remove_dir_all(..)` at the end of each test, which a panic skips. About 340 tests had no cleanup line at all.

## The approach

### `TestDir`

A new crate, `crates/test-dirs` (`axcenity-test-dirs`):

- It uses the standard library only and depends only on `axcenity-workspace-hack`, as every engine crate does.
- It is a dev-dependency of 28 crates. Nothing engine-internal sits below it, so there is no cycle.
- `tempfile` is not a workspace dependency (it is only transitive), so no new third-party dependency was added.

The API:

- `TestDir::new(name)` makes an empty `axcenity-<name>-<process>-<n>` under the temp folder.
- `TestDir::unmade(name)` names one without making it.
- `TestDir::adopt(path)` takes charge of a path the test chose: a folder or a file, made or not.
- `dir.at(path)` stands for a path inside the folder, such as a project made in it, and still removes the whole folder.

How it behaves:

- **It derefs to `PathBuf`.** That is the one non-obvious choice. A helper that returned a `PathBuf` can return a `TestDir`, and its callers' `&folder`, `folder.join(..)`, `Some(&folder)` and `folder.clone()` keep working. `clone()` gives a plain `PathBuf` copy.
- **Drop removes the folder.** It clears read-only flags (Git's object files) and retries for about 3 s while another process still holds a file. That happens on Windows: luau-lsp and the updater were still exiting when their tests ended.
- **It refuses** to remove the temp folder itself, a folder above it, or a root (`is_protected`).
- **It never panics.** A panic in drop during unwinding would abort the process and hide the real failure, so what it can't remove it names on standard error.
- **`AXCENITY_KEEP_TEST_DIRS=1`** (anything but empty, `0` or `false`) keeps everything and prints each kept path.

### The conversion

Behaviour is kept in every case. A test that checks something survives a restart holds one `TestDir` for its whole body.

**Mechanical edits**, made by scripts, then fixed against compiler errors:

- **About 140 `let x = temp_dir()...;` lines** in `#[test]` functions became `let x = TestDir::adopt(temp_dir()...);`. The folder names are unchanged.
- **About 130 helpers returning `PathBuf`** now return `TestDir`. Where the compiler asked, their wrappers, tuple returns and struct fields changed too.

**Places where a `TestDir` would have dropped too early.** Three checks were run over the whole workspace:

1. Unbound temporaries.
2. `helper(..).join(..)` chains.
3. A `TestDir` made inside a helper that returns something else.

They found:

- `Some(&team_folder("host"))` and `EditorApp::new(Some(&copy_of_example(..)))` were bound to a `let` first: 30 sites in `team_tests.rs`, `team_board_tests.rs`, `collab_*_tests.rs`, `relaunch_tests.rs`, `level_tests.rs` and release.
- Helpers that open an app on a copied example now return the `TestDir` **first** in the tuple, so it drops after the app: agent, block undo, camera, plugin and prefab tests.
- The same applies to the physics fracture `slab`, nav behavior `project`, Collaborate `linked_project` (with its bare remote), `remote_log` (its clone is read afterwards), the editor `session_with_a_crate` (both data folders), and repo `conflict` (its remote).
- `scratch("x").join("update.zip")` in updater `download.rs` and `temp_file("cert").with_file_name(..)` in net `secure_tests.rs` were bound first.

**Debug pictures:**

- The PNGs a test writes for a person to look at are adopted at the call.
- They are removed after the statement that writes them, and kept with `AXCENITY_KEEP_TEST_DIRS=1`.
- Files: player `clouds_tests.rs`, `time_of_day_tests.rs`; editor `settings/tests.rs`, `graphics_benchmark_tests.rs`.

**Repo crate:**

- `testing::temp` returns `TestDir::new("repo-<name>")`.
- `ConfigRunner` keeps its git config folder alive with the runner.
- The old sweep of `axcenity-repo-<process>` folders stays, to clear what earlier runs left.

**Shared parents flattened.** `axcenity-release-tests/<name>-<pid>` became `axcenity-release-tests-<name>-<pid>`, and the same was done for `axcenity-testing-tests`. That leaves no empty parent behind.

**Player `TempCopy`.** It now holds a `TestDir`, which gains the retry and the keep switch. Its own `Drop` is gone.

**Tests that only take a folder (allowed by the task):**

- `luau_check::Checker::start_in(program, folder)` is new. `start` names its folder as before and calls it.
- The test-only `LuauChecks::start_with` takes a folder.
- The two direct checker tests and the two app tests pass one. It is bound before the app, so it is removed after the app's checker stops.

**Other test-side cleanups:**

- `play_tests.rs` adopts Play's per-process save folder (`axcenity-editor-saves-<process>`).
- `convert_tests.rs` adopts the per-process USD conversion folder, and its `.usda` files written beside the project.
- The git credential store test now passes the store path with forward slashes, so Git's shell no longer eats it. The store lands in the test's own folder, and the assertions are unchanged.

**Two bugs this work introduced, caught and fixed before the final run:**

1. The first conversion script also wrapped five bare `let folder = std::env::temp_dir();` lines. On drop, those would have emptied the whole temp folder: `light_probes.rs`, `probe.rs`, `modifiers_tests.rs`, `uv_tests.rs`, `time_of_day_tests.rs`. A review found them before any test ran, and that run was stopped while it was still building. The lines were put back, and `is_protected` now refuses such a removal (red run 3).
2. The first full run after the conversion failed six editor tests (agent, plugin, prefab). Their helpers returned an app whose project had already been removed. `remote_log` also lost the clone a test reads afterwards. All of these were fixed as described above.

## Tests

### New: `axcenity-test-dirs`, 9 unit tests and 1 doc test

- `new_makes_an_empty_uniquely_named_folder_under_the_temp_folder`
- `dropping_removes_the_folder_and_everything_in_it` (with a read-only file)
- `a_panicking_test_still_removes_its_folder`
- `adopt_removes_a_file_or_folder_the_test_named_itself`
- `unmade_leaves_making_the_folder_to_the_code_under_test`
- `at_stands_for_a_path_inside_and_still_removes_the_whole_folder`
- `the_temp_folder_and_the_folders_above_it_are_never_removed`
- `the_keep_variable_keeps_folders_when_set_to_a_true_value`
- `a_file_still_open_for_a_moment_is_removed_once_it_is_let_go` (Windows)

Red runs are in [test-temp-cleanup-red-run.txt](test-temp-cleanup-red-run.txt):

1. With `Drop` and `keep_requested` stubbed: 5 of 6 failed, each leaving its folder.
2. The guard: `is_protected(temp, temp)` failed.
3. The retry: `couldn't remove ... being used by another process`.

Green: `Summary [0.315s] 9 tests run: 9 passed, 0 skipped`. Doc test: `1 passed`.

`at` was written together with its test during the conversion. It had no separate red run.

### The workspace, before and after

Each run used a private temp folder.

| Run | Tests | Result | Left behind |
|---|---|---|---|
| Before (`refinement-wave5`, test-dirs excluded) | 5,496 | 5,486 passed, 4 failed, 6 timed out | **342 entries, 5.59 GB** |
| First run after converting | 5,504 | 5,492 passed, 8 failed, 4 timed out | 29 entries, 1.34 GB (led to the fixes above) |
| **Final** | **5,505** | **5,499 passed, 2 failed, 4 timed out** | **12 entries, 1.15 GB: none from a passing test** |

**What the final run left:**

- **11 entries** are the folders of three tests nextest killed at 2 minutes:
  - `collab_repo_tests::a_file_changed_on_both_sides_keeps_both_until_someone_chooses`
  - `collab_repo_tests::the_same_scene_changed_on_two_computers_keeps_both`
  - `collab_scenario_tests::when_everyone_leaves_the_project_waits_in_its_repository`

  A killed process never runs its drops. These three and `the_resolve_window...` also timed out in the before run. Under the full suite's load their `git` work is slow.
- **`axcenity-test-saves`** is one empty folder made by the game-test runner's production `SaveFolder` (in the `testing` crate, not test code). It is the shared parent of per-test slot folders that are removed. It never grows.

**Rerun alone (private temp)**, every test that failed or timed out in the final run: `13 tests run: 13 passed (9 slow)`, with **nothing left** in the temp folder. That covers:

- the 11 Collaborate repository and scenario tests, including the three that timed out;
- `net tests::a_search_from_the_network_lists_the_game_at_its_network_address_and_joins_it`;
- `render renderer::tests::skin_layers::each_makeup_layer_and_marking_changes_the_skin_where_its_mask_is`.

**Every test that passed before passes**, with two notes:

- The only test that passed in the before run and not in the final run is the GPU timing test `skin_layers::each_makeup_layer_and_marking_changes_the_skin_where_its_mask_is`. It timed out at 120 s under the full load and passes alone in 64 s.
- `collab_repo_tests::closing_the_editor_uploads_what_was_saved` and the network LAN search failed in the before run too. Both pass alone.
- The first run after converting had one ABORT (`0xc0000409`, `makeup_tests::the_cheek_close_up_shows_pores`). It is a GPU test with no temp folder, and it passed in the final run.

### Other checks

- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all`: done.
- `python3 -I scripts/check-wiki.py`: 115 pages, 0 errors, 57 warnings (existing staleness warnings).
- `cargo hakari generate`: no changes. `cargo hakari generate --diff`: clean. `manage-deps --dry-run` lists every crate in the workspace, including ones this branch doesn't touch, so it isn't caused by this change.

## Dependencies added

None from outside the workspace. The new workspace crate `axcenity-test-dirs` (Apache-2.0) uses the standard library only. It is pinned as `axcenity-test-dirs = { path = "crates/test-dirs" }` in `[workspace.dependencies]` and added as a dev-dependency to 28 crates: anim, assets, audio, avatar, character, collab, convert, cook, editor, editor-toolbox, gameplay, mesh_edit, nav, net, physics, player, plugins, project, release, repo, save, scene, script, server, teamwork, testing, toolbox and updater. `Cargo.lock` gained the crate.

## Production code

The only production change is `Checker::start_in` (editor, `luau_check.rs`). `Checker::start` makes the same folder as before and hands it to `start_in`. Otherwise production code is unchanged.

Two production temp leftovers are recorded in `wiki/Roadmap/Known-Limitations.md`, not fixed:

- the USD import's `axcenity-converted-<process>/<n>`;
- a luau-lsp checker folder when the editor exits mid-check.

## Files changed

**New:**

- `crates/test-dirs/` (`Cargo.toml`, `CLAUDE.md`, `src/lib.rs`, `src/tests.rs`)
- this hand-back and its red run

**Workspace:** `Cargo.toml`, `Cargo.lock`, and 28 crate `Cargo.toml` files (dev-dependency).

**Test code** in 218 `.rs` files under `crates/*`. Each change converts a temp folder to `TestDir`, binds it, or returns it. Assertions are unchanged.

**Docs:**

- `docs/development.md`: a new "Temporary folders in tests" section (the API, keeping a value alive, removal and the guard, the opt-out, killed tests, how to measure a run);
- `docs/README.md` (map row);
- `docs/architecture/crates.md` (a `test-dirs` row);
- wiki `Development/Testing-Strategy.md`, `Architecture/Repository-Structure.md`, `Roadmap/Known-Limitations.md`.

## Open issues

- **Killed tests.** Three Collaborate git tests time out under the full suite's load, before and after this change, and leave about 1.1 GB each run. Two ways to fix it:
  - give them a longer slow-timeout or a test group in `.config/nextest.toml`;
  - make them faster.

  That is outside this task (it changes nextest settings), so it is named here for the lead.
- **The avoided files.** `crates/avatar/tests/fitting.rs:63` (`axcenity-fit-<name>-<pid>`) still uses a bare `temp_dir()`. It left nothing in either run, so it cleans up on success, but it leaks if it panics. The other avoided files (`avatar/src/fit*.rs`, `player/src/fit_gate_tests.rs`, `wardrobe_tests.rs`, `garment_checks.rs`) left nothing in either run. Convert `fitting.rs` once the fit specialist's branch is in.
- **`axcenity-test-saves`.** Flattening the game-test runner's `SaveFolder` to `axcenity-test-saves-<pid>-<n>` would leave nothing, but it changes production code.
- **Remaining bare `temp_dir()` uses.** About 30 tests still use one directly. They create nothing, read nothing, or clean up on success: `Assets::new(temp_dir())` reads, `ModelDoc::open(.., &temp_dir(), ..)` and others. `TestDir` is there for the next one who touches them.
- **Line endings.** Some edited files are LF in the working tree. Git stores them as LF either way, so the diff shows no line-ending changes.

## What the lead must do at merge

- Expect conflicts only with branches that touch the same test helpers. Most likely is the editor's `app/tests.rs` (`copy_of_example`) or `collab_repo_tests.rs`, in any branch adding tests there.
- A new test that calls `copy_of_example`, `team_folder` or another converted helper gets a `TestDir`. Bind it with `let`, and if a helper returns an app, return the `TestDir` too (see `docs/development.md`).
- Run `cargo hakari generate` after merging dependency changes (no change from this branch).
- Optional, for the open issue above: raise the slow-timeout for `collab_repo_tests` and `collab_scenario_tests`, then rerun the suite. The temp folder should then stay empty except for `axcenity-test-saves`.
- To check on the shared machine: set `TMP` and `TEMP` to an empty folder, run `cargo nextest run --workspace --no-fail-fast`, and list that folder.
