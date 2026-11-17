# Hand-back: RA1, R1 step 2 part 1, the studio shell (`r1-studio-shell`)

Branched from `character-creator-stage0` (5b650d5d). Plan: [section 11](../../../../../plans/axcenity-engine-character-studio.md) (11.3 contracts, 11.4 DIS-06, DIS-17, DIS-18, 11.5 R1). Audit: [audit-studio-ui.md](../audit-studio-ui.md) §1 to §3 and D-01 to D-16. Brief §3.1 to §3.3 and §10.

## What changed

**Defects fixed, each with a test seen failing first** ([red run](r1-studio-shell-red-run.txt): 7 tests, 7 failed on the old code):

| Defect | Fix | Test |
|---|---|---|
| D-01 Save overwrote another character with the same name | Save never writes over a file that isn't this character's own: it saves under the first free "<name> 2" (the character takes that name) and the status line says so | `walkthrough_tests::save_never_overwrites_another_character_with_the_same_name` |
| D-02 closing the window with unsaved studio changes didn't ask | The workspace is in the editor's unsaved-documents list (`DocKind::Character`, `app/documents.rs`); "Save changes?" names the character, and its Save saves it (`save_character_workspace`, `app/session.rs` `answer`) | `app::character_workspace::tests::closing_the_window_with_unsaved_studio_changes_asks_and_saves_the_character` |
| D-03 a rename saved a second file | The workspace keeps the file it opened or last saved; a save after a rename moves it (Recent entry, pictures and pack places follow: `packs::rename_character`); a case-only rename keeps one file | `walkthrough_tests::a_rename_moves_the_saved_file_instead_of_copying_it` |
| D-04 the name field couldn't be emptied | The field keeps a draft while typed in and renames once, when left | `walkthrough_tests::a_change_of_nothing_keeps_redo` (the model side); the field itself is UI only |
| D-05 a click that changed nothing lost redo | `edit.rs` transactions record a step only when the character changed (`same_character`: a parameter at its default equals one left out) | `edit::tests::a_transaction_is_one_step_and_a_click_that_changes_nothing_keeps_redo`, `walkthrough_tests::a_change_of_nothing_keeps_redo` |
| D-06 the Face view aimed at a fixed height | It aims at the head bone as posed; on the default character that is where the fixed view was (4.52 of 5 studs) | `shell_tests::the_face_view_aims_at_the_head_of_a_tall_or_short_character` |
| D-07 the panel cloned the manifest per slider per frame | Panels get one `&Library` per frame and read values with `value_of` (no clone); presets compare against it | not measured (no test) |
| D-10 leaving Face kept the close-up | Face and Expressions frame the face; leaving them frames the whole character unless the camera was moved | `shell_tests::pages_frame_the_face_or_the_whole_character` |
| D-11 a mirrored pair whose sides differ was flattened | With Symmetry such a pair shows both sides | `panel::tests::the_level_filters_rows_and_symmetry_merges_only_equal_pairs` |
| D-12 view chips never showed the view; drawn twice | One set, in the bottom bar; the camera's view is lit until the camera moves | `shell_tests::pages_frame_the_face_or_the_whole_character` (state) |
| D-13 notices never cleared; one problem shown | A notice clears after 8 s; an apply's problems say "(and N more)" | `shell_tests::a_notice_clears_itself` |
| D-16 Ctrl+Shift+Z undid | Redo is matched before undo; shortcuts are left to a text field being typed in | `shell_tests::ctrl_shift_z_redoes_and_ctrl_z_undoes` |
| D-09 skin groups and accessory slots never drawn | Skin and Makeup and Accessories pages draw them (none shipped) | `shell_tests::every_page_is_drawn_with_its_controls` |

Not fixed: D-08 (`follow` constraints; no shipped manifest has one), D-14 (content), D-15 (a guard on the slider path: DIS-18 measured it, below, but no budget is asserted).

**Contract files written for the lead** (`crates/editor/src/character_workspace/`):

- `intent.rs`: `Intent` (SetParameter, SetMirrored, Choose, Wear, SetMaterial, SetBase, Rename, ApplyPreset, New, Open, Save, Undo, Redo, CancelEdit, Reset(Scope), PickCategory, PickFeature, PickLevel, SetSymmetry, FocusView, SetLighting, SetBackdrop, PreviewExpression, Home) and `Gesture` (Drag, Commit). Capture and comparison are left for part 2 (a comment says so).
- `selection.rs`: `Selection { category, feature: Option<String>, level }`, `Level` (Quick, Detailed, Advanced; one flat filter; default Detailed, today's view). Editor-only, never saved.
- `edit.rs`: `History` (record, begin, commit, cancel, undo, redo; an undo during a drag closes it first) moved out of `character_workspace.rs`; `Scope` (All, Category, Group) and `reset`; `same_character`.

**Module split (the shell only)**: `shell.rs` (layout, shortcuts, notices, `apply(Intent)`, the New/Open "Save changes?"), `nav.rs` (the brief's order: Body, Face, Hair, Clothing, Skin and Makeup, Accessories, Expressions, Presets; features from Detailed; changed counts), `panel.rs` with `panel/parameters.rs`, `panel/row.rs`, `panel/materials.rs`, `panel/tests.rs`, `toolbar.rs` (name, New, Open, Save, undo, redo, Reset with Everything, the category, the picked feature), `bottom_bar.rs` (views with a named Three-quarter, lighting, backdrop: the Stage page is gone). `ui.rs` is deleted. Panels return `Vec<Intent>`; only `shell.rs` changes the workspace. Camera input stays in `shell.rs` (`viewport`) until R2's `viewport.rs`.

Pages: Body (preset chips, body family when there's a choice, groups), Face (nine groups folded until opened or picked), Hair, Clothing, Accessories (slots, or a short empty state), Skin and Makeup (every value of the skin-shaded material slot `Skin`: tint as a colour button, six sliders; "not drawn yet" on the values the renderer ignores until CO7: only `tint` and `roughness_scale` are drawn today, `panel/materials.rs` `DRAWN_TODAY` mirrors `scene::material::modulate`), Expressions (the 15 built-ins as chips, previewed on the face through a FaceControls of the preview's own on the body, as the capture tool does; never in the description, never an undo step; leaving the page takes it off), Presets (cards).

## Interfaces

- `CharacterWorkspace::apply(Intent) -> Option<WorkspaceAction>`, `answer(Option<bool>)` (the New/Open question), `asking: Option<Intent>`, `selection: Selection`, `view: Option<ViewPreset>`, `expression: Option<String>`, `file()`, `save_reporting() -> Result<Saved, String>`, `transact(change, Gesture)`, `set_value`, `set_material`, `material_value`, `rename`, `reset_scope(&Scope)`, `new_character(index)`, `open_character(path)`, `focus(ViewPreset)`, `preview_expression`, `cancel_edit`, `dragging()`.
- Removed: `advanced: bool`, `category` (now `selection`), `drag`, `is_preset`, `ui::WorkspaceAction` (now `shell::WorkspaceAction`, re-exported as before). `Category` lost `Stage`, gained `SkinAndMakeup`, `Accessories`, `Expressions`; `ViewPreset` gained `ThreeQuarter`. `base()`, `parameter()`, `set_parameter()`, `items_for()`, `worn_in()`, `reset()` are test-only now (the panels use free functions that take a borrowed library).
- `packs::free_name` is `pub(crate)`; new `packs::rename_character`.
- Editor app: `DocKind::Character`; `EditorApp::save_character_workspace`.

**Shims the lead swaps at merge** (`selection.rs`, one function each): `parameter_level` (advanced → Advanced, choices → Quick, else Detailed) for `Parameter.level`; `parameter_feature` (the group) for `Parameter.feature`; `parameter_about` (the label) for `Parameter.about`. Until then **Quick shows only presets and the body type** on Body, and nothing on Face but a line pointing to Detailed: the manifest marks no parameter `quick` yet (height, muscle and fat are the ux patterns' choice).

## Tests

Written and run (`cargo nextest run -p axcenity-editor --no-fail-fast character`): **62 tests run: 62 passed, 1287 skipped**. New: `walkthrough_tests.rs` (7: the walkthrough through Intents, New/Open asking, D-01, D-03, D-05, reset by scope, a skin value without a rebake), `shell_tests.rs` (6), `panel/tests.rs` (5), `edit.rs` (3), `selection.rs` (3), `timing_tests.rs` (1, ignored), and the D-02 test in `app/character_workspace.rs`. Updated for the moved code, no assertion weakened: `tests.rs` (levels for the Advanced bool, `selection.category`, a three-quarter check added), `shelf_tests.rs` (`opening_and_saving_put_a_character_first_in_recent` saved two characters by renaming one, which is D-03; it now makes a second with `new_character`, same assertions). Also run: `-E 'test(every_page_is_drawn) | test(/documents|unsaved|save_changes|close/)'`: 42 passed. `cargo clippy -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all`: clean. `python scripts/check-wiki.py`: 0 errors (58 warnings, all pre-existing staleness).

The whole editor suite and the workspace were not run (only this crate's character filter and the related app tests).

## DIS-18

[drag-timing.md](drag-timing.md): release build, 9950X3D and RX 9070 XT, 20 steps per parameter, three runs (two pinned to one CCD). A shape parameter applies in 2 to 3.6 ms median, 10 to 18 ms worst step; with the next frame's step and draw list about 6 to 8 ms median. Under the 33 ms rule, so the drag stays as it is; the 120 ms throttle is now the larger delay. Not measured: the upload, a dressed character with hair, the minimum GPU class.

## Visual evidence

`AXCENITY_SNAPSHOTS=<folder> cargo nextest run -p axcenity-editor every_page_is_drawn` saves every page at 1440 by 900 with the preview rendered (`character-studio-page-{body-quick,body,body-advanced,face,hair,clothing,skin,accessories,expressions,presets}.png`); they were written outside the repository (they show a local path in the status line). What I saw:

- Body (Detailed): three columns, no nested frames; the nav shows Frame, Build, Proportions under Body; the panel has the level switch and Symmetry, preset chips ("Male" lit), Body type chips, Muscle, Body fat, Height as compact sections. The bottom bar fits at 1440 with Full body lit, Studio and Grey lit.
- Body (Quick): thin, only presets and Body type (the shim, above). Body (Advanced): typed number boxes on the right of each row; Build by region open with 16 rows, a long list.
- Face › Nose: the face framed at the head bone as before; four nose sliders; the nav lists the nine features with Nose highlighted.
- Skin and Makeup: Tint colour button, six sliders, "not drawn yet" in muted text after five of them, and the note at the bottom; no Symmetry on that page.
- Expressions: Happy lit; the preview's face smiles, but weakly (the content's expression, OPEN-14, not this change).
- Clothing: "Underwear comes with the body type" and the "No clothes yet" empty state.
- The editor was not run interactively with `--character-studio`: no screenshot of the live window, so drags, menus, the colour picker and the name field were only exercised by tests and code reading.

## Dependencies

None added.

## Open issues

- Quick level is thin until the manifest has `level` (above).
- `character_workspace.rs` is about 1,150 lines (the model plus the preview: refresh, head, expression preview). A `preview.rs` split would be the next tidy step.
- The colour button's drag closes its undo step when the pointer is let go (a frame-start safety net in `shell.rs`); a colour typed in the picker's text field becomes a step on the next frame.
- D-01 renames instead of asking. A question ("Replace / Save as 'X 2' / Cancel") would need a dialog; the notice says what happened.
- Category reset keeps the body type (a choice): documented in `edit.rs` and the topic file; Everything resets it.
- `architecture.md` (the lead's) doesn't yet name `intent.rs`, `selection.rs`, `edit.rs`; no ADR was written for them (11.3 says the lead's contract change brings one).

## For the lead at merge

1. Swap the three shims in `selection.rs` for `Parameter.level`, `feature`, `about`; mark height, muscle and fat `quick` so the Quick level has its broad controls.
2. `intent.rs`, `selection.rs` and `edit.rs` are the plan's lead-owned files: review them as the contract (Locks and `features.rs` are not here).
3. Conflicts are likely in `crates/editor/src/app/session.rs` (`answer`, the prompt's name) and `app/documents.rs` if another branch touched them.
4. `docs/editor/character-studio.md`, `crates/editor/CLAUDE.md`, the wiki's ADR-0031 file list and `Editor/Specialized-Editors.md` were updated; add the contracts to `architecture.md`.
