# Stage 0 audit: the Character Studio workspace and its interface

Audited against [brief.md](brief.md) sections 3.1 to 3.4, 10, 11, 12, 15 (character editing, viewport interaction) and 16 (thumbnails, browsing, picking), and against the open part of CO12 in [the plan](../../../../plans/axcenity-engine-character-studio.md). Source and tests were read; nothing was built or run. Where a defect says "by reading", it was found in the code and not reproduced. A second agent then checked every row and every cited line against the source; what it changed is listed under [Verification](#verification) at the end.

## Summary

- The workspace is a working three-column editor: a toolbar, six categories on the left, a rendered preview, a panel drawn from the manifest. It edits a format 2 description, has its own undo, saves, and the start page lists characters with pictures drawn on a thread.
- It is still a properties panel beside a viewer. The viewport takes camera input only. There is no selection, no picking, no handles, no highlight.
- The panel is one flat list per category. There is no feature level (Face, then Nose), one Advanced switch instead of three levels, no colour or material pickers, no thumbnails inside the workspace.
- Nothing exists for randomize, comparison, variants, user presets, favourites, locks, autosave or a turntable. Pose choice, clip choice, screenshots and expressions have pieces outside the workspace (the capture tool in `player`, the thumbnail painter, the face tab) and no control in it.
- The engine already has most of what the missing preview features need: 15 built-in expressions, 48 face controls, the starter clips, material values per shading model, sockets, item variants and tags. None of it is reachable from the workspace.
- Every identity change re-bakes the body and every worn part and uploads them again. It is throttled to one apply per 120 ms while a slider is dragged. Material values and the proportion attributes do not re-bake.
- Two data-loss defects were found by reading: Save overwrites a character with the same name, and closing the editor window with unsaved changes in the workspace does not ask.
- The realistic body has no hair and no clothes in the library (three items, all underwear), so two of the six pages are empty on the default character.

Counts of the table below: 196 rows; 33 implemented, 79 partial, 84 missing.

### Status rule used

- **implemented**: a user can do it in the workspace today, and a test or a shipped file proves it.
- **partial**: some of it. This includes "engine only" rows, where the data model, an engine feature or the Game Editor's tab already carries the capability and the workspace has no control for it. The gap column says so.
- **missing**: no control and no supporting piece. Each was searched with `rg -i` under at least three names in `crates/editor/src/character_workspace*`, `crates/editor/src/start/characters*`, `crates/editor/src/app/character_workspace.rs` and `crates/avatar/src`; the names are in the evidence column.

Sizes: **S** one file, under a day. **M** one module, a few days. **L** several modules or a shared contract. **XL** needs engine or asset work in other crates.

### Short names for paths

| Short | Path |
|---|---|
| WS | `crates/editor/src/character_workspace.rs` |
| UI | `crates/editor/src/character_workspace/ui.rs` |
| STG | `crates/editor/src/character_workspace/stage.rs` |
| THB | `crates/editor/src/character_workspace/thumbnails.rs` |
| TST | `crates/editor/src/character_workspace/tests.rs` |
| SHT | `crates/editor/src/character_workspace/shelf_tests.rs` |
| START | `crates/editor/src/start/characters.rs` |
| STT | `crates/editor/src/start/characters/tests.rs` |
| APPW | `crates/editor/src/app/character_workspace.rs` |
| FTAB | `crates/editor/src/character_studio.rs` and `character_studio/` (the Game Editor's face tab) |
| MAN | `crates/avatar/src/manifest.rs` |
| DESC | `crates/avatar/src/description.rs` |
| ITEM | `crates/avatar/src/item.rs` |
| MAT | `crates/avatar/src/material.rs` |
| RES | `crates/avatar/src/resolve.rs` |
| APPLY | `crates/gameplay/src/description.rs` |
| DER | `crates/assets/src/derived.rs` |
| IDN | `crates/assets/src/model/identity.rs` |
| RB | `assets/characters/realistic/Realistic.base.json` |
| DH | `crates/project/src/default_humanoid.rs` |
| FACE | `crates/core/src/face.rs` |
| FPLAY | `crates/anim/src/face_play.rs` |
| PICK | `crates/scene/src/pick.rs` |

## 1. Map of the UI source

### The workspace

| File | Lines | Purpose | Main types and functions |
|---|---|---|---|
| WS | 780 | State and editing model: the description, undo, the preview World, the library, presets, saving | `CharacterWorkspace` (fields :260-292), `Category` :226, `Preset` :102, `presets()` :145, `open` :303, `open_file` :376, `open_preset` :387, `edit` :430, `drag` :443, `set_parameter` :477, `wear` :513, `choose` :529, `set_base` :537, `use_preset` :551, `undo` :577, `redo` :586, `reset` :596, `save` :618, `show_only` :633, `refresh` :671, `step` :732, `tidy` :758, `find_character_library` :84, `offers_base` :194, `symmetric_label` :207 |
| UI | 607 | All drawing, as one `impl CharacterWorkspace` | `WorkspaceAction` :17, `ui` :64 (layout rectangles :74-88), `shortcuts` :107, `toolbar` :138, `categories` :222, `pick_category` :241, `viewport` :255, `properties` :325, `presets_page` :350, `base_picker` :392, `parameters_page` :422, `slider` :472, `choices` :495, `slots_page` :511, `stage_page` :566, `status` :595 |
| STG | 259 | Camera, lighting and backdrop presets, the floor | `Orbit` :15 (`camera` :84, `turn` :100, `zoom` :106, `pan` :112), `ViewPreset` :30 (`orbit` :57), `Lighting` :126, `Backdrop` :164, `add_floor` :209, `apply` :228, `backdrop` :250 |
| THB | 446 | Pictures of characters and presets for the start page, drawn offscreen on a thread, cached as PNGs | `Painter` :162 (`paint` :183), `Thumbnails` :253 (`request` :287, `poll` :335, `texture` :374), `Subject`, `Job`, `picture_orbit`, `character_picture`, `preset_picture`, `draw_pictures` |
| `character_workspace/shelf.rs` | 197 | Saved characters as the start page lists them; overview text; delete | `SavedCharacter`, `list`, `library_for`, `Overview`, `overview`, `delete_character` |
| `character_workspace/packs.rs` | 317 | Character packs: make, export, import, add to a project | `Pack`, `packs`, `save_pack`, `export_pack`, `import_pack`, `add_to_project` |
| `character_workspace/recent.rs` | 96 | The Recent list (`recent.json`) | `remember`, `forget`, `recent` |
| TST | 592 | 12 tests: ten of the model, one that draws every page, one that renders the preview | see section "Tests that exist" |
| SHT | 399 | 8 tests of Recent, overview, delete, packs, pictures | |
| APPW | 195 | The editor's side: open, draw in place of the start screen, leave with a Save question | `open_character_workspace` :25, `character_workspace_ui` :48 |

### The start page

| File | Lines | Purpose | Main types and functions |
|---|---|---|---|
| START | 624 | The Character Studio page: New Character, Recent, your characters, packs, presets | `Shelf` :57, `Pick` :74, `characters_page` :197, `character_grid` :359, `preset_grid` :407, `packs_section` :448, `follow_pick` :519 |
| `start/characters/cards.rs` | 318 | Character, preset and pack cards with pictures | `character_card` :45, `pack_card` :172, `small_picture` :291 |
| `start/characters/dialogs.rs` | 263 | Pack editor and delete questions | `ShelfDialog`, `shelf_dialog` :41, `pack_editor` :137 |
| STT | 341 | 6 tests of the page | |

### The Game Editor's face tab (still named "Character Studio")

| File | Lines | Purpose |
|---|---|---|
| `character_studio.rs` | 628 | The tab: the selected character's face in a preview of its own; the pane's title is "Character Studio" (`crates/editor/src/layout.rs:205`) |
| `character_studio/rig.rs` | 499 | Face profile, the 48 FaceControls as sliders (:225-252), profile mapping |
| `character_studio/expressions.rs` | 253 | Built-in and project expressions, previewed and blended (:87-115) |
| `character_studio/lip_sync.rs` | 559 | Lip sync timeline |
| `app/character_studio.rs` | 326 | Applies what the tab asks for |

### The visual language (what a redesign must reuse)

| File | Lines | What it gives |
|---|---|---|
| `crates/editor-kit/src/theme.rs` | 1955 | Tokens: `type_scale` :30, `radius` :45, `space` :59, `size` :75, `Palette` :469 (`app_bg`, `panel`, `surface`, `inset`, `control*`, `text*`, `accent*`), `palette(ctx)` :872, `ease` :1204, fonts (`medium_family`, `display_family`), `test_context` :1576, `run` :1590 |
| `crates/editor-kit/src/widgets.rs` | 1516 | `button` with `Role` :12/:25, `icon_button` :124, `segmented` :187, `section_label` :295, `nav_item` :339, `nav_rail` :444, `nav_tab` :568, `search_field` :679, `divider` :773, `tool_window` :1000, `section` :1309, `empty_state` :1405, `chip` :1436, `info_card` :1487 |
| `crates/editor-kit/src/responsive.rs` | 162 | `toolbar`, `form`, `split`: rows that fit a 260 point column |
| `crates/editor-kit/src/color_palette.rs` | 504 | `edit_srgb` :356, `swatches` :214: the editor's colour button and saved palettes |
| `crates/editor-kit/src/screen.rs` | 188 | `Screen`: `ray` :26, `point` :31, `line` :37, `studs_per_point` :43 |
| `crates/editor-kit/src/snapshot.rs` | 184 | `snapshot` :10, `snapshot_rendered` :21, `snapshot_timed` :39, `save_png` :174 |
| `crates/editor/src/theme.rs` | 216 | The editor's own icons |

The workspace uses the palette for every colour (UI:72, 139, 279) and the kit's `nav_item`, `chip`, `section_label`, `button` and `icon_button`. It uses stock egui for the sliders, the two checkboxes and the group headers (UI:429-432, 457, 482).

### How the preview is drawn and its camera driven

- The preview is a World of its own holding the default character and a floor (WS:655-666). The description is applied to it with the runtime's `apply_description` (WS:683).
- Each frame `viewport` (UI:255) reads the pointer, steps the animators (WS:732), sets lighting and floor colour (STG:228), builds a draw list with `axcenity_scene::build_draw_list_posed` (UI:282), overrides the sky with the backdrop (STG:250) and paints it through `axcenity_editor_tools::preview::Canvas` (`crates/editor-tools/src/preview.rs:30-87`), which owns a `TextureRenderer` the size of the rectangle and shows its texture in egui.
- The camera is `Orbit` (yaw, pitch, distance, target; STG:15). Left drag turns, right or middle drag pans, the wheel zooms, a double click returns to Full body (UI:257-275). The field of view is fixed at 30 degrees (STG:95). The view presets are constants for a character 5 studs tall (STG:8, 57-80).
- The workspace asks for a repaint every frame (UI:291).

### How undo is kept

- Two stacks of whole `Description` clones, `history` and `future` (WS:269-270). `edit` pushes one step for a change (WS:430). `drag` opens a step on its first call and closes it when `done` is true (WS:443). `undo` and `redo` swap the description and re-apply (WS:577-592). `dirty` compares with the description last saved (WS:425).
- It is separate from the Game Editor's `History` (`crates/editor/src/app/edits.rs`), which the workspace does not use.
- Editor-only state (category, symmetry, advanced, orbit, lighting, backdrop) is not in the stacks and not in the file (WS:275-282).

### How drawing snapshot tests work

- `theme::test_context()` and `theme::run` draw UI without a GPU. `the_workspace_draws_every_category_on_both_bodies_without_a_gpu` draws all six pages on both bodies, Advanced on and off (TST:402-418).
- With `AXCENITY_SNAPSHOTS=<folder>` the same test saves the whole workspace at 1440 by 900 on the Body and Face pages, preview rendered, through `snapshot_rendered` (TST:420-438). `the_preview_draws_the_realistic_character` renders the preview offscreen and checks the character covers the middle of the frame (TST:513-592). The start page saves its own pictures (STT:263-341).
- Snapshots are for looking at. Nothing compares them with a stored picture. Without a graphics adapter they skip (`crates/editor-kit/src/snapshot.rs:46-50`).

### Tests that exist

| Test | What it proves |
|---|---|
| TST:22 `the_workspace_offers_the_realistic_body_and_shows_it` | The default body and its parts |
| TST:55 `male_or_female_is_a_choice_that_wears_its_own_underwear` | Choices |
| TST:148 `a_slider_changes_the_realistic_preview_while_it_is_dragged` | A drag re-applies, one undo step; prints a timing, asserts none |
| TST:183 `symmetry_moves_a_parameters_mirror_too` | Symmetry |
| TST:212 `switching_the_body_starts_its_default_character_and_undo_returns` | Body switch and undo |
| TST:240 `a_format_1_character_file_opens_on_the_stylized_body_and_saves_as_format_2` | Migration |
| TST:274 `the_workspace_dresses_the_preview_undoes_and_saves` | Wear, undo, save |
| TST:369 `every_preset_is_sound_for_the_shipped_library` | Presets |
| TST:402 `the_workspace_draws_every_category_on_both_bodies_without_a_gpu` | Every page draws without a panic and asks for nothing. It asserts no text, so it does not prove that a named slider is there |
| TST:448 `the_views_look_at_the_character_from_where_they_say` | View presets and camera limits |
| TST:473 `a_saved_character_applied_in_a_game_matches_the_workspaces_preview` | Runtime consistency |
| TST:513 `the_preview_draws_the_realistic_character` | The preview renders |
| APPW:143 `character_studio_opens_from_the_start_screen_as_a_workspace_of_its_own` | Open and leave; the texts Customize, Presets, Face, Clothing and a preset's name are drawn; the Save question is asked |
| SHT:299 `pictures_are_drawn_cached_and_drawn_again_when_the_file_changes` | Thumbnails |
| SHT:134 `a_pack_is_made_edited_renamed_exported_imported_and_added_to_a_project` | Packs, and a pack's characters written into a game project |

No test drives the workspace through pointer or key events. The tests call the model's methods.

### Picking that exists in the editor

| Piece | Where | Reusable for a click on the character? |
|---|---|---|
| Ray from the camera through a screen point | PICK:50-73 `CameraView::new`, `ray`; `Screen::ray` in `crates/editor-kit/src/screen.rs:26` | Yes, as is |
| World point to screen point | PICK:96-99 `project_point`; `Screen::point` | Yes: handles and labels |
| Part picking | PICK:121-180 `pick`, `pick_hit_where` | No: a MeshPart is hit by its box (PICK:186-187), and the character's body has `CanQuery` false (DH:452) |
| Ray against a triangle | `crates/editor-model-studio/src/view.rs:789-817` `hit_triangle`, `pick_face` | The method, yes. Both are private to Model Studio and work on an `EditMesh` |
| CPU-posed vertices of a skinned model | `crates/assets/src/model/rig.rs:390` `Posed::primitive(model, index, level)`, with levels of detail | Yes: the triangles to test, at a coarse level |
| Posed bone positions | `crates/assets/src/model/rig.rs:349` `Posed::bones`; `Skeleton::joints` in `crates/editor-tools/src/posing.rs:314` | Yes: anchors for body handles and camera focus |
| Bone picking on screen | `crates/editor-model-studio/src/rig.rs:758` `pick_bone` | The method |
| Handle hover and painting | `crates/editor-tools/src/gizmo.rs:598` `hovered`, :635 `paint` | The pattern: handles are egui shapes over the viewport |
| A ray against a mesh through a bounding volume hierarchy | `crates/mesh_edit/src/reference.rs:104` `Reference::ray` (public; built from an `EditMesh`, :41); `crates/mesh_edit/src/bvh.rs:40, 163` `Bvh::build`, `Bvh::ray` (private to the crate; they take positions and triangles) | The structure, yes: it is what makes hover picking cheap. It needs a public way in from plain positions and triangles. The editor already depends on `mesh_edit` (`crates/editor/Cargo.toml:30`) |
| A line round a drawn item | `crates/render/src/draw_list.rs:2080-2094` `Material::outline`, `Outline` (an inverted hull; "smooth meshes (spheres, characters) get an unbroken line") | Yes, for marking a whole part (a garment, the hair, the body). Not a region of the body |
| The engine's raycast | `crates/physics/src/query.rs:445` | No: colliders, not the skinned surface |
| The body's region map | `RealisticRegions.png` and the body's second UV set (architecture.md section 6) | Not at runtime: the loader keeps one UV set (`crates/assets/src/model.rs:231-262`) |

There is no ray test against a posed skinned mesh anywhere. It has to be written; the pieces above are enough to write it. Two ray-against-triangle tests and one hierarchy already exist (Model Studio's, `mesh_edit`'s), all private or tied to an `EditMesh`, so the smallest change is to make one of them reachable, not to write a third.

## 2. From a slider drag to the preview changing

Call by call:

1. `egui::Slider` reports a change. `slider` calls `set_parameter(parameter, value, done)` with `done = !response.dragged()` (UI:481-491).
2. `set_parameter` writes the value, and its mirror when Symmetry is on, inside `drag` (WS:477-489).
3. `drag` (WS:443-459): on the first call it pushes a history step and clears redo. It clones the description, applies the change, runs `tidy` (WS:758: reads the library, drops unknown parameters, clamps, snaps choices, wears what choices wear) and calls `refresh(done)`.
4. `refresh` (WS:671-697) returns if the description equals what is applied. While dragging it also returns if the last apply was under 120 ms ago (`DRAG_APPLY_EVERY`, WS:69). Otherwise it calls `apply_description`.
5. `apply_description` (APPLY:223):
   - reads the library (a file stat of `catalogue.json` and a global lock, APPLY:70-93);
   - clones the description and resolves all of it: every parameter, corner, region and item (RES:308-405);
   - makes one `Identity` for the body and every part (RES:129-132) and derives new `MeshId`s from its hash (DER:31-44);
   - asks for the body's new model at once to compare skeleton fits (APPLY:257-264). That bakes the body now: `Model::bake` clones every primitive's positions, normals and tangents, adds the identity shapes, keeps the live shapes and rebuilds the bind (IDN:274-420);
   - sets `MeshId` on the body and on every part, since they share the identity (APPLY:360-363);
   - writes body slider attributes that changed, sockets, material value attributes, and the kept description as JSON (APPLY:202-216, 272-294, 382-414).
6. Next frame, `step` (WS:732) runs the animators. An owner whose model changed drops its bound clips, its legs and feet state, and remakes its state machine where it was (`crates/anim/src/animators.rs:783-789`).
7. `build_draw_list_posed` (UI:282) asks the mesh cache for each part's model (`crates/scene/src/mesh_cache.rs:120-137`). The parts' derived models are baked here, on the UI thread, the first time they are asked for (DER:85-92). The renderer then uploads new buffers for the body and every part (UI:290).
8. When the drag ends, `refresh(true)` applies once more.

**Is the whole character rebuilt?** Yes, for any parameter that drives a shape or a bone: the body and every worn part get a new derived model and a new upload. Nothing is incremental. The Instances are kept (APPLY:326-380), so the pose carries on.

**What does not rebuild.** Parameters that only write attributes (height and the eight proportions, RB:761-939) and material values are outside the identity (RES:349-360, 386-391). They cost a resolve and a few attribute writes, no bake. So "do not rebuild the character for a colour change" (brief 16) already holds below the UI.

**Timings stated anywhere.**

| Source | Figure | What it covers |
|---|---|---|
| `docs/development/character-studio/progress.md:37` | "about 6 ms in a debug build" | `apply_description` on the realistic preview. This includes the body's bake (step 5). It does not include the parts' bakes or the upload (step 7), or the clip rebinding (step 6) |
| TST:154-160 | printed, not asserted | The same call |
| WS:69 | 120 ms | The throttle while dragging |
| DER:14-16 | 96 | Derived models kept before unheld ones are dropped; "a slider dragged in Character Studio makes one per step" |

No test holds a budget for this path. The realistic preview wears only underwear today; a dressed character with hair has more parts to bake per step.

**Panel cost per frame** (not measured): `base()` clones the whole manifest (WS:419-421) and `parameter()` calls it once per slider (WS:463-473, UI:479). Each `library()` call stats a file (APPLY:76-78). See defect D-07.

## 3. Proposed module split for the redesign

New files only, all under `crates/editor/src/character_workspace/`. One owner per row. The first two rows land before anyone else starts.

| Area | New files | Holds | Depends on |
|---|---|---|---|
| Contract (the lead) | `intent.rs`, `selection.rs`, `edit.rs`, `features.rs` | `Intent`: everything a panel can ask for (set a parameter, wear, pick a category or feature, focus the camera, undo, save, reset a scope, randomize a scope, capture, compare). `Selection`: category, feature, level; editor-only. `edit.rs`: history, transactions with cancel, reset by scope, locks. `features.rs`: the feature table read from the manifest (groups, regions, anchors, handles) | `avatar` |
| Shell and layout | `shell.rs`, `shell_tests.rs` | The frame: rectangles, responsive rules, shortcuts, notices; calls each area's draw function and applies the `Intent`s they return. Takes over `UI:64-135, 595-607`, after which `ui.rs` is deleted | Contract |
| Navigation | `nav.rs`, `nav_tests.rs` | Categories, the features under a category (Face: Eyes, Nose, ...), the level switch | Contract |
| Contextual panel | `panel.rs`, `panel/parameters.rs`, `panel/controls.rs`, `panel/materials.rs`, `panel/tests.rs` | Dispatch by selection; parameter groups; the slider row (label, value entry, reset, lock, tooltip); colour and material pickers | Contract; kit additions |
| Toolbar | `toolbar.rs`, `toolbar_tests.rs` | Name, New, Open, Save, undo, redo, reset, randomize, compare, viewport settings | Contract; new `WorkspaceAction` variants |
| Bottom bar | `bottom_bar.rs`, `playback.rs`, `capture.rs`, `bottom_bar_tests.rs` | Camera, pose, lighting and clip buttons; preview-only expression and clip state; screenshot and turntable | Contract; `stage.rs` |
| Viewport interaction and handles | `viewport.rs`, `viewport/camera.rs`, `viewport/picking.rs`, `viewport/handles.rs`, `viewport/highlight.rs`, `viewport/tests.rs` | Drawing and input routing between camera and handles; focus and framing from bones; ray against the posed mesh; handles that write through `Intent`; hover and selection marks | Contract; `features.rs` |
| Comparison | `compare.rs`, `compare/view.rs`, `compare/tests.rs` | Captured state and variants held outside the history; before and after toggle; split view with one camera | Contract; viewport |
| Presets and randomize | `presets.rs`, `randomize.rs`, `presets_tests.rs`, `randomize_tests.rs` | Built-in and user presets by scope, favourites; random values by scope with locks | Contract; the preset file format |
| Browsers | `browsers.rs`, `browsers/wardrobe.rs`, `browsers/hair.rs`, `browsers/presets.rs`, `browsers/item_pictures.rs`, `browsers/tests.rs` | Grids with thumbnails, search, tags, filters; item pictures drawn by the existing `Painter` | Contract; THB |
| Recovery | `autosave.rs`, `autosave_tests.rs` | A recovery file outside `characters/`, offered on the next open | Contract |

Rules that keep the files disjoint:

- A draw function takes `&CharacterWorkspace` (or the narrow view it needs) and returns `Vec<Intent>`. Only the shell changes the workspace. No area calls another area.
- Each area keeps its own state in its own struct in its own file (`nav::Nav`, `viewport::View`, `compare::Compare`, `randomize::Locks`). The lead adds one field per struct to `CharacterWorkspace` in the contract change.
- Each area adds its own test file and its own snapshot. Nobody edits TST.

## 4. Facts the UI needs that the manifest does not carry

The smallest set, each optional so existing files stay valid (the precedent is `choices`, added without a version change; architecture.md section 6).

| # | Fact | Where | Why the UI cannot do without it | Today |
|---|---|---|---|---|
| 1 | A sentence saying what a parameter changes | `Parameter.description` | Tooltips; "understand a control before using it" (brief principle 6) | `Parameter` has `id`, `label`, `group`, `range`, `default`, `advanced`, `mirror` and drivers only (MAN:110-139) |
| 2 | Three levels instead of two | `Parameter.level`: quick, detailed or advanced; absent means detailed, or advanced when `advanced` is true | Brief 3.3 | One bool (MAN:118-120) |
| 3 | Which body region or face feature a parameter belongs to | `Parameter.feature` (a feature ID) | Click on the waist, show the waist's controls; reset or randomize one feature. Face groups already are features (eyes, nose, jaw, ...). Body groups are by kind (build, build by region, proportions; RB:89-155), so "waist" is spread over three groups | Nothing links a parameter to a region |
| 4 | Features: what can be selected in the viewport | `features: [{id, label, category, pick, anchor, focus, handles}]` on the base manifest. `pick`: bones (body) or an anchor with a radius (face). `anchor`: a vertex of the body mesh, so it follows the identity. `handles`: `{parameter, axis, per_metre, side}` | Selection, highlight, camera focus and handles (brief 3.4, 15). All face features sit on the Head bone, so bones alone cannot tell the nose from the jaw | None. Regions carry `id`, `label`, `mask_shape`, `map_index` (MAN:262-269); no bones, no anchors |
| 5 | Which material slots a person edits, and how they are shown | `MaterialSlot.label`, `.category`, `.editable` (value names), `.swatches` (named value sets such as skin tones) | Colour and material pickers. The realistic base lists 13 slots; `EyeOcclusion`, `TearLine` and `LipSeal` are not for people (RB:3957) | `{name, shading}` only (MAT:36). The value kinds and ranges per shading model exist (MAT:102, 283) |
| 6 | A range for random values | `Parameter.random: [low, high]`, inside `range` | "Controlled parameter ranges" (brief 11). Without it the code would hard-code a share of the range | Nothing |
| 7 | Item thumbnails that exist | `ItemManifest.thumbnail`, `CatalogueItem.thumbnail` | The wardrobe browser | The field exists (ITEM:54, `crates/avatar/src/catalogue.rs:52`). 0 of 15 shipped items set it and nothing reads it. A fallback is to draw them with THB's `Painter` |

Not needed in the manifest: icons (the UI can map group IDs, with a default), the expression list (`anim::BUILT_IN_EXPRESSIONS`, FPLAY:24), the clip list (the character's state machine, DH:142-165), face framing (the head bone, as the face tab does at `crates/editor/src/character_studio.rs:446`).

Item-side capabilities (brief 14.1: a hairstyle's own length or volume) are a separate contract: `ItemManifest` has no parameters of its own (ITEM:19-57).

## 5. Requirements

| ID | Requirement | Brief § | Status | Evidence (path:line) | Gap | Size |
|---|---|---|---|---|---|---|
| UI-3.1-01 | Left navigation: Body | 3.1 | implemented | WS:229; UI:222-237, 340-343; TST:402 | None for the entry. No features under it | S |
| UI-3.1-02 | Left navigation: Face | 3.1 | implemented | WS:230; UI:246, 344; TST:402; APPW:143 | None for the entry. No features under it | S |
| UI-3.1-03 | Left navigation: Hair | 3.1 | partial | WS:231; UI:345, 511-564 | The page is empty on the realistic body: no hair item fits it (catalogue: 3 realistic items, all underwear) | S |
| UI-3.1-04 | Left navigation: Clothing | 3.1 | partial | WS:232; UI:345, 518-528 | On the realistic body it shows only the underwear note | S |
| UI-3.1-05 | Left navigation: Skin and Makeup | 3.1 | partial | Engine only: MAN:100 `Category::Skin`; MAT:283 skin values. No entry: WS:226-234, 247-256. Searched skin, makeup, tint, melanin | No category, no page; no group of category skin in RB | M |
| UI-3.1-06 | Left navigation: Accessories | 3.1 | partial | Engine only: MAN:103, 278 (`multiple`), 287 sockets; ITEM:98 `Fit::Socket`. No entry: WS:226-234. Searched accessor, socket, glasses | No category, no page, no accessory items | M |
| UI-3.1-07 | Left navigation: Expressions | 3.1 | partial | Engine and the face tab only: FPLAY:24-40; FTAB `expressions.rs:87-115`. Searched expression, FaceControls, emotion in WS, UI, STG: none | No entry in the workspace | M |
| UI-3.1-08 | Left navigation: Presets | 3.1 | implemented | WS:228; UI:339, 350-387; TST:369; APPW:143 | Text cards, no pictures | S |
| UI-3.1-09 | Viewport: large interactive character preview | 3.1 | implemented | UI:80-88, 255-296; TST:513 | Interactive for the camera only | S |
| UI-3.1-10 | Viewport: orbit, pan and zoom | 3.1 | implemented | UI:257-272; STG:100-121; TST:461-469 | Pan is clamped to a fixed box (STG:117-120) | S |
| UI-3.1-11 | Viewport: front, side, rear and three-quarter views | 3.1 | partial | STG:76-78 Front, Side, Back; TST:448-456 | No three-quarter view by name (Full body is turned 25 degrees, STG:66-69); one side only. The capture tool has a three-quarter camera (`crates/player/src/character_capture.rs:70-75`) | S |
| UI-3.1-12 | Viewport: full-body and face camera presets | 3.1 | implemented | STG:66-75; UI:246-249; TST:457-460 | The face target is a constant height (defect D-06). No upper body, hands or feet | S |
| UI-3.1-13 | Viewport: neutral lighting and configurable studio lighting | 3.1 | partial | STG:126-160, 228-240; UI:567-574 | Four times of day. "Studio" is the sun at 14:30. Nothing is configurable; no light rig | M |
| UI-3.1-14 | Viewport: character selection and direct manipulation | 3.1 | missing | UI:256-275 handles camera input only. Searched pick, ray, hit, select, handle, gizmo in WS, UI, STG | Everything (see 3.4) | XL |
| UI-3.1-15 | Viewport: optional anatomical region highlighting | 3.1 | missing | Searched highlight, outline, region, hover in WS, UI, STG. The loader keeps one UV set: `crates/assets/src/model.rs:231-262` | Everything for a region. Pieces: a line round a whole part (`crates/render/src/draw_list.rs:2080-2094`); the region table (RB:3802) names 14 body regions, but its map is not loaded at run time | L |
| UI-3.1-16 | Panel: context-sensitive customization tools | 3.1 | partial | UI:338-347 switches by category | Not by selected feature; sliders and chips only | M |
| UI-3.1-17 | Panel: feature-specific controls | 3.1 | partial | UI:435-469: one collapsible header per manifest group | All groups of the category at once | M |
| UI-3.1-18 | Panel: preset selection | 3.1 | partial | UI:350-387 whole-character presets | No presets per feature or category | M |
| UI-3.1-19 | Panel: colour and material pickers | 3.1 | partial | Engine only: DESC:33-36, 49; MAT:283; `crates/editor-kit/src/color_palette.rs:356`. Searched colour, color, tint, picker, edit_srgb in WS, UI: none | No picker in the workspace; `wear` never sets a variant or values (WS:522) | M |
| UI-3.1-20 | Panel: advanced controls | 3.1 | implemented | UI:431-432, 444; MAN:118-120; TST:408-409 | One switch (see UI-3.3-21) | S |
| UI-3.1-21 | Panel: numeric parameter entry | 3.1 | partial | UI:481-485: egui's slider value box, two decimals | No field of its own, no units, no test | S |
| UI-3.1-22 | Toolbar: character name | 3.1 | implemented | UI:152-163; WS:776-778 | Defect D-04: the field cannot be emptied | S |
| UI-3.1-23 | Toolbar: new character | 3.1 | partial | START:232-260 on the start page; STT:177. Toolbar: Home only (UI:142) | Not in the workspace | S |
| UI-3.1-24 | Toolbar: open character | 3.1 | partial | START:359; APPW:32-33; STT:177 | Not in the workspace | S |
| UI-3.1-25 | Toolbar: save | 3.1 | implemented | UI:116, 172-183; WS:618-628; TST:274 | Defects D-01, D-02, D-03 | S |
| UI-3.1-26 | Toolbar: undo and redo | 3.1 | implemented | UI:108-124, 197-216; WS:577-592; TST:148, 212, 274 | Defects D-05, D-16 | S |
| UI-3.1-27 | Toolbar: reset | 3.1 | implemented | UI:184-195; WS:596-604 | Whole character only | S |
| UI-3.1-28 | Toolbar: randomize | 3.1 | missing | Searched randomi, shuffle, dice, fn random in the workspace, start page, avatar, APPLY: none | Everything | L |
| UI-3.1-29 | Toolbar: character comparison | 3.1 | missing | Searched compar, before_after, side.by.side, variant, a/b in WS, UI, STG: none | Everything | L |
| UI-3.1-30 | Toolbar: viewport settings | 3.1 | partial | UI:566-593: a Stage category in the navigation | Not in the toolbar | S |
| UI-3.1-31 | Bottom bar: camera presets | 3.1 | partial | UI:297-315: chips at the top of the viewport | No bottom bar | S |
| UI-3.1-32 | Bottom bar: pose selection | 3.1 | partial | Engine only: the capture tool stages rest (the A-pose), idle or one clip at a time (`crates/player/src/character_capture.rs:137-147, 286-312`). Searched pose, a_pose, rest, clip, StateMachine in WS, UI, STG: none. The preview always idles (DH:381) | No control in the workspace. Same evidence as UI-10-19 | M |
| UI-3.1-33 | Bottom bar: lighting presets | 3.1 | partial | UI:567-574 on the Stage page | No bottom bar | S |
| UI-3.1-34 | Bottom bar: animation preview | 3.1 | partial | WS:732-747: the idle always plays | No choice of clip, no play or pause | M |
| UI-3.1-35 | Bottom bar: screenshot and turntable capture | 3.1 | partial | Engine only: the workspace already draws a character offscreen and writes a PNG, for the start page (THB:183-215, 226); the capture tool writes six fixed views and a contact sheet (`crates/player/src/character_capture.rs:63, 389, 475`). Searched screenshot, turntable, capture, save_png, export in WS, UI, STG, APPW: none | No control; no turntable anywhere (CO12-10). Reuse THB:183 and `crates/editor-kit/src/snapshot.rs:174` | M |
| UI-3.1-36 | Avoid excessive borders, nested panels and oversized controls | 3.1 | partial | UI:95-99 flat panels without borders | Stock egui headers, sliders and checkboxes (UI:429-432, 457, 482); a file path in the status line (UI:598-603) | M |
| UI-3.1-37 | Modern, minimal and polished | 3.1 | partial | Snapshots of Body and Face only (TST:420-438) | The redesign itself | L |
| UI-3.1-38 | Use the editor's visual language | 3.1 | partial | UI:72, 139, 142, 173, 225, 307: palette tokens and kit widgets | Sliders, checkboxes and headers are stock egui | S |
| UI-3.2-01 | Show only the controls of the selected category or feature | 3.2 | partial | UI:338-347 | By category, not by feature | M |
| UI-3.2-02 | Selecting Face opens facial customization | 3.2 | implemented | UI:241-253, 344; TST:402 | None | S |
| UI-3.2-03 | Selecting a feature (Nose, Eyes) shows that feature's controls | 3.2 | partial | The face's groups are the features (Eyes, Brows, Nose, Cheeks, Jaw, Chin, Lips, Forehead, Ears; RB:89-155) and each is a header that opens and closes (UI:457-460). No selection below the category: WS:279. Searched feature, selected_group, focus, sub in WS, UI | No selected feature: every group is open at once, nothing in the navigation. The body's groups are kinds (build, proportions), not features | M |
| UI-3.2-04 | Nose: width | 3.2 | missing | RB:1612-1720: the nose group has nostril width, length, bridge, tip, tilt; no whole-nose width | A parameter and its shape (the face area); the panel draws it with no code | S |
| UI-3.2-05 | Nose: length | 3.2 | implemented | RB:1650; UI:461-463; TST:402 | None | S |
| UI-3.2-06 | Nose: bridge height | 3.2 | partial | RB:1668: one "Nose bridge" | Height and width are one slider | S |
| UI-3.2-07 | Nose: bridge width | 3.2 | missing | RB:1668 is the only bridge parameter | A parameter (the face area) | S |
| UI-3.2-08 | Nose: tip projection | 3.2 | partial | RB:1686: one "Nose tip" | Projection and rotation are one slider | S |
| UI-3.2-09 | Nose: tip rotation | 3.2 | missing | RB:1686 is the only tip parameter | A parameter (the face area) | S |
| UI-3.2-10 | Nose: nostril width | 3.2 | implemented | RB:1612, 1631; UI:445-452; TST:183 | None | S |
| UI-3.2-11 | Nose: nostril height | 3.2 | missing | No such parameter in RB:1612-1720 | A parameter (the face area) | S |
| UI-3.2-12 | Nose: nostril angle | 3.2 | missing | No such parameter in RB:1612-1720 | A parameter (the face area) | S |
| UI-3.2-13 | Eyes: size | 3.2 | implemented | RB:1318, 1347; UI:461-463 | None | S |
| UI-3.2-14 | Eyes: spacing | 3.2 | implemented | RB:1184, 1217 | None | S |
| UI-3.2-15 | Eyes: height | 3.2 | implemented | RB:1118, 1151 | None | S |
| UI-3.2-16 | Eyes: depth | 3.2 | implemented | RB:1250, 1284 (advanced) | Shown only with Advanced | S |
| UI-3.2-17 | Eyes: tilt | 3.2 | implemented | RB:1376, 1395 | None | S |
| UI-3.2-18 | Eyes: upper eyelid shape | 3.2 | missing | The eyes group has ten parameters (RB:1118-1410), none for a lid. Searched lid, eyelid in RB | A parameter (the face area) | S |
| UI-3.2-19 | Eyes: lower eyelid shape | 3.2 | missing | As above | A parameter (the face area) | S |
| UI-3.2-20 | Eyes: eye opening | 3.2 | missing | As above | A parameter (the face area) | S |
| UI-3.2-21 | Eyes: eye corner position | 3.2 | missing | As above | A parameter (the face area) | S |
| UI-3.2-22 | Controls in logical groups, not one long list | 3.2 | implemented | UI:435-469; RB:89-155 (13 groups: 4 body, 9 face); TST:402 | "Build by region" is 16 sliders in one group; no sub-groups | S |
| UI-3.3-01 | Level 1: body presets | 3.3 | partial | WS:145-190: six presets that set frame, muscle and fat | They replace the whole character (WS:551-562); no body-only preset | M |
| UI-3.3-02 | Level 1: face presets | 3.3 | missing | No preset sets a face parameter (WS:145-190). Searched face preset, head preset | Everything; needs a preset scope | M |
| UI-3.3-03 | Level 1: hair presets | 3.3 | partial | UI:540-552: chips per hair slot | No hair on the realistic body; no pictures | M |
| UI-3.3-04 | Level 1: clothing presets | 3.3 | partial | WS:106-108 `Preset.items` exists; every preset leaves it empty (WS:145-190) | No outfit presets, no list | M |
| UI-3.3-05 | Level 1: skin tones | 3.3 | partial | Engine only: MAT:283 (`melanin`, `undertone`, `redness`); DESC:33-36. Searched skin, tone, melanin in WS, UI: none | No control | M |
| UI-3.3-06 | Level 1: overall proportions | 3.3 | implemented | RB:91 (body type), 761 (height), 940, 949 (muscle, fat); UI:435-469; TST:55, 148 | None | S |
| UI-3.3-07 | Level 1: broad facial adjustments | 3.3 | partial | Basic face sliders are single features (RB:1118-2170) | No broad ones (face width, roundness, age) | M |
| UI-3.3-08 | Level 2: individual facial parameters | 3.3 | implemented | RB:1118-2170 (47 parameters, 9 groups); UI:422-470; TST:402 | None | S |
| UI-3.3-09 | Level 2: regional body adjustments | 3.3 | implemented | RB: `build_regions` (16 parameters) and proportions, advanced; UI:444 | Behind Advanced | S |
| UI-3.3-10 | Level 2: hairline and hairstyle adjustments | 3.3 | missing | Items have no parameters (ITEM:19-57). Searched hairline, parting, volume, length in RB and items | Everything; item capabilities | L |
| UI-3.3-11 | Level 2: clothing variations | 3.3 | partial | Engine only: ITEM:107-112 `Variant`; DESC:46-48. 0 of 15 items have variants; WS:522 never sets one | No control, no data | M |
| UI-3.3-12 | Level 2: skin details | 3.3 | partial | Engine only: MAT:283 (`detail_strength`, `roughness_scale`) | No control; freckles and moles are CO17 | M |
| UI-3.3-13 | Level 2: material adjustments | 3.3 | partial | Engine only: DESC:33-36; MAT:102, 283 | No control | M |
| UI-3.3-14 | Level 3: independent left and right facial adjustments | 3.3 | implemented | UI:429-430, 445-452; WS:477-489; TST:183 | Defect D-11 | S |
| UI-3.3-15 | Level 3: precise numeric values | 3.3 | partial | UI:481-485 | Two decimals in the slider's box only | S |
| UI-3.3-16 | Level 3: anatomical proportions | 3.3 | implemented | RB:780-939 (leg, arm, shoulder, waist, hip, head, hand, foot), advanced; UI:444 | None | S |
| UI-3.3-17 | Level 3: fine geometry adjustments | 3.3 | missing | Only manifest parameters exist. Searched sculpt, vertex, fine in WS, UI | Must stay parameters (brief 3.4); needs more of them | XL |
| UI-3.3-18 | Level 3: advanced material parameters | 3.3 | partial | Engine only: MAT:283 (`scatter_scale`, `iris_depth`, `limbal_ring`) | No control | M |
| UI-3.3-19 | Level 3: detailed feature positioning | 3.3 | partial | RB:1118, 1184, 1250 (eye height, spacing, depth), 1414 (brow height), 1984 (mouth height), 2094 (ear height) | Nothing moves the nose up or down, the brows together or apart, or the eye corners | M |
| UI-3.3-20 | Level 3: advanced asymmetry controls | 3.3 | partial | RB:1704 `nose_tilt`, 1837 `jaw_shift`; per-side sliders with Symmetry off | No copy side, flip or asymmetry amount | M |
| UI-3.3-21 | The Advanced switch becomes three levels | 3.3 | missing | One bool: MAN:118-120; WS:277-278; UI:431, 444. Searched level, tier, quick, detail in WS, UI, MAN | A level per parameter (section 4, fact 2) and a switch | M |
| UI-3.4-01 | Select the nose; adjust width, bridge or tip | 3.4 | missing | UI:256-275. Targets exist: RB:1612, 1668, 1686 | Selection and handles | XL |
| UI-3.4-02 | Select the eyes; adjust spacing, height or tilt | 3.4 | missing | UI:256-275. Targets exist: RB:1184, 1118, 1376 | Selection and handles | XL |
| UI-3.4-03 | Select the jaw; adjust width or definition | 3.4 | missing | UI:256-275. Target: RB:1799 (width); none for definition | Selection and handles | XL |
| UI-3.4-04 | Select the mouth; adjust width or lip proportions | 3.4 | missing | UI:256-275. Targets: RB:1910, 1948, 1966 | Selection and handles | XL |
| UI-3.4-05 | Select the ears; adjust size or protrusion | 3.4 | missing | UI:256-275. Targets: RB:2056, 2134 | Selection and handles | L |
| UI-3.4-06 | Select the shoulders; adjust shoulder width | 3.4 | missing | UI:256-275. Target: RB:820 | Selection and handles | L |
| UI-3.4-07 | Select the waist; adjust waist proportions | 3.4 | missing | UI:256-275. Target: RB:840 | Selection and handles | L |
| UI-3.4-08 | Select the hips; adjust hip width | 3.4 | missing | UI:256-275. Target: RB:860, range 1.0 to 1.15 (it only widens) | Selection and handles | L |
| UI-3.4-09 | Select the legs; adjust supported proportions | 3.4 | missing | UI:256-275. Target: RB:780 | Selection and handles | L |
| UI-3.4-10 | Select the head; adjust its size | 3.4 | missing | UI:256-275. Target: RB:880 | Selection and handles | L |
| UI-3.4-11 | Hovering a supported feature gives subtle feedback | 3.4 | missing | UI:267-272 uses hover for zoom only. Searched hover, highlight, outline | Picking each frame and a mark | L |
| UI-3.4-12 | Clicking selects the feature | 3.4 | missing | UI:256: clicks only reset the view (UI:273). Searched select, pick, click | Picking and a selection | L |
| UI-3.4-13 | The viewport focuses the selected region | 3.4 | partial | Picking Face in the navigation frames the face (UI:245-249), to a constant (STG:70-75). No test asserts it. Nothing is selected in the viewport | Focus from bones or anchors, per feature. Same evidence as UI-15-11 | M |
| UI-3.4-14 | Manipulation handles appear | 3.4 | missing | Searched handle, gizmo, grip in WS, UI, STG. Pattern: `crates/editor-tools/src/gizmo.rs:598, 635` | Everything | L |
| UI-3.4-15 | Dragging a handle changes the parameter | 3.4 | missing | As above. Write path exists: WS:477-489 | Handle to parameter mapping (section 4, fact 4) | L |
| UI-3.4-16 | The panel reflects the new value | 3.4 | missing | No handles. The panel reads the description every frame (UI:479), so it will follow | None once handles write the description | S |
| UI-3.4-17 | Switch between handles and sliders without losing changes | 3.4 | missing | No handles. One description is the only edited state (WS:266) | None once handles write the description | S |
| UI-3.4-18 | Use the existing parameter and deformation systems | 3.4 | missing | No handles. Sliders use them: WS:477, APPLY:223 | Handles must call `set_parameter` | S |
| UI-3.4-19 | Handles update the same values as the controls | 3.4 | missing | No handles | As above | S |
| UI-3.4-20 | No separate geometry editing state | 3.4 | missing | No handles. Holds for sliders: WS:266 | Keep it so | S |
| UI-3.4-21 | Undo and redo for direct manipulation | 3.4 | missing | No handles. `drag` already makes one step per drag (WS:443-459; TST:148) | Use `drag`; add cancel | S |
| UI-3.4-22 | Predictable interaction sensitivity | 3.4 | missing | No handles. `Screen::studs_per_point` exists (`crates/editor-kit/src/screen.rs:43`) | Sensitivity per handle (section 4, fact 4) | M |
| UI-3.4-23 | Camera movement never modifies the character | 3.4 | missing | No handles. Today a left drag anywhere turns the camera (UI:257-260) | Input routing: a hovered handle takes the drag | M |
| UI-3.4-24 | Symmetry-aware editing | 3.4 | missing | No handles. Sliders mirror: WS:478-486; TST:183 | Handles on both sides; decide set-equal or same change (defect D-11) | M |
| UI-3.4-25 | Direct manipulation can be turned off | 3.4 | missing | No handles, no setting. Searched disable, direct, manipul | A setting, kept by the editor | S |
| UI-3.4-26 | A few meaningful handles, not sculpting | 3.4 | missing | No handles | A design rule for the feature table | S |
| UI-3.4-27 | First: eyes, nose, jaw, mouth, shoulders, waist, hips | 3.4 | missing | No handles. Parameters exist for all seven (RB:1118-1410, 1612-1720, 1799, 1910-1990, 820, 840, 860); no whole-nose width; hips only widen | Features and handles for the seven | XL |
| UI-10-01 | Expression: Neutral | 10 | partial | Engine and the face tab only: FPLAY:24; FTAB `expressions.rs:104-115` | Not in the workspace; its preview character has no FaceControls (APPLY:223-421, DH:407-470 make none) | M |
| UI-10-02 | Expression: Happy | 10 | partial | FPLAY:25; as above | As above | S |
| UI-10-03 | Expression: Sad | 10 | partial | FPLAY:26; as above | As above | S |
| UI-10-04 | Expression: Angry | 10 | partial | FPLAY:27; as above | As above | S |
| UI-10-05 | Expression: Surprised | 10 | partial | FPLAY:28; as above | As above | S |
| UI-10-06 | Expression: Disgusted | 10 | partial | FPLAY:33; as above | As above | S |
| UI-10-07 | Expression: Fearful | 10 | partial | FPLAY:34; as above | As above | S |
| UI-10-08 | Expression: Confused | 10 | partial | FPLAY:29; as above | As above | S |
| UI-10-09 | Expression: Custom | 10 | partial | The face tab edits and saves expression files (FTAB `expressions.rs:1-12, 87`) | Not in the workspace; project files only | M |
| UI-10-10 | Facial control: smile intensity | 10 | partial | FACE:124, 129 `Mouth_Smile_*`; face tab sliders FTAB `rig.rs:225-252` | Not in the workspace; one control per side | S |
| UI-10-11 | Facial control: brow movement | 10 | partial | FACE:94-119 `Brow_Raise_*`, `Brow_Lower_*`, `Brow_Inward_*`; as above | As above | S |
| UI-10-12 | Facial control: eye squint | 10 | partial | FACE:69-70; as above | As above | S |
| UI-10-13 | Facial control: mouth opening | 10 | partial | FACE:144 `Mouth_Open`; as above | As above | S |
| UI-10-14 | Facial control: jaw movement | 10 | partial | FACE:227-242 `Jaw_Open`, `Jaw_Forward`, `Jaw_Left`, `Jaw_Right`; as above | As above | S |
| UI-10-15 | Facial control: blink intensity | 10 | partial | FACE:50, 55 `Blink_*`; as above | As above | S |
| UI-10-16 | Animation preview: idle | 10 | partial | WS:732-747; DH:142, 381 | Always on; cannot be chosen or stopped | S |
| UI-10-17 | Animation preview: walking | 10 | partial | Engine only: DH:143 | No control | S |
| UI-10-18 | Animation preview: running | 10 | partial | Engine only: DH:144 | No control | S |
| UI-10-19 | Animation preview: standing | 10 | partial | Engine only: an Animator with nothing to play poses at rest (APPLY:296-307); the capture tool does it (`crates/player/src/character_capture.rs:312`). No clip named Stand (searched stand in DH) | No control; rest is the A-pose, not a standing pose | M |
| UI-10-20 | Animation preview: basic gestures | 10 | partial | Engine only: DH:160-165 (Wave, Point, Cheer) | No control | S |
| UI-10-21 | Expressions and clips are preview-only unless saved on purpose | 10 | partial | The description has no expression or clip field (DESC:20-41); save writes only it (WS:618-623) | Holds for the idle; nothing else to preview yet | S |
| UI-10-22 | Identity is independent of the current expression | 10 | implemented | IDN:5-13: identity shapes are baked, live shapes stay; test IDN:601 `identity_shapes_are_baked_in_and_live_shapes_stay`; DESC:20-41 has no expression field | No workspace test with an expression showing, since none can be shown | S |
| UI-11-01 | Create characters from a starting preset | 11 | implemented | START:407-442; WS:387-403; STT:177; TST:369 | None | S |
| UI-11-02 | Save custom presets | 11 | missing | Presets are compiled in: WS:102-109, 145. Searched save_preset, custom preset, user preset, template | A preset file format and store | L |
| UI-11-03 | Save facial presets | 11 | missing | As above; searched face preset | A preset scope | M |
| UI-11-04 | Save body presets | 11 | missing | As above; searched body preset | A preset scope | M |
| UI-11-05 | Save hairstyles and outfits | 11 | missing | Packs group whole characters (`crates/editor/src/character_workspace/packs.rs:1-19`). Searched outfit, hairstyle preset | A preset scope | M |
| UI-11-06 | Favourite presets | 11 | missing | Searched favorit, favourite, starred, pinned in the workspace, start page, avatar | A favourites store in editor data. Pattern: the Toolbox's favourites (`crates/editor-toolbox/src/drawer.rs:16, 344`) | S |
| UI-11-07 | Randomize the entire character | 11 | missing | Searched randomi, shuffle, dice, fn random | Everything | M |
| UI-11-08 | Randomize only the selected category | 11 | missing | As above | A scope | S |
| UI-11-09 | Randomize selected facial features | 11 | missing | As above | A scope; needs features (section 4, fact 3) | S |
| UI-11-10 | Lock features while randomizing others | 11 | missing | Searched locked, lock_, frozen, freeze, pin in WS, UI, avatar ("pinned" only in a comment, MAN:248) | Locks, editor-only | M |
| UI-11-11 | Randomization produces valid characters | 11 | missing | No randomizer. Reusable: `tidy` clamps (WS:758-780); `Description::validate` (DESC:111) | Tests over many seeds | M |
| UI-11-12 | Controlled ranges and compatible assets | 11 | missing | No randomizer. Compatibility exists: `items_for` filters by base (WS:492-500). No random range in MAN:110-139 | A range per parameter (section 4, fact 6) | M |
| UI-12-01 | Capture a state before making changes | 12.1 | missing | Searched compar, capture, snapshot, baseline in WS, UI. History holds earlier states (WS:269) but none is kept by choice | Everything | M |
| UI-12-02 | Compare the current state with the captured one | 12.1 | missing | As above | Everything | M |
| UI-12-03 | Toggle between before and after | 12.1 | missing | Only undo and redo (WS:577-592), which the brief rules out | A toggle that leaves history alone | S |
| UI-12-04 | Compare with the same camera and lighting | 12.1 | missing | No comparison. One camera and stage per workspace (WS:280-282) | Comes free in one viewport | S |
| UI-12-05 | Restore the original state | 12.1 | missing | Only undo, or Reset to the body's default (WS:596-604) | One undoable step back to the capture | S |
| UI-12-06 | Two-character comparison | 12.2 | missing | One character, one canvas (WS:264, 283). Searched side.by.side, second, split | A second character or World and a second canvas | L |
| UI-12-07 | Side-by-side facial comparison | 12.2 | missing | As above | As above, framed on the face | S |
| UI-12-08 | Body proportion comparison | 12.2 | missing | As above | As above | S |
| UI-12-09 | Preset comparison | 12.2 | missing | As above; `use_preset` replaces the character (WS:551-562) | A preset shown as a variant | S |
| UI-12-10 | Hairstyle comparison | 12.2 | missing | As above | As above | S |
| UI-12-11 | Clothing comparison | 12.2 | missing | As above | As above | S |
| UI-12-12 | Synchronized camera in comparisons | 12.2 | missing | No comparison. `Orbit` is plain data (STG:15) | One orbit for both views | S |
| UI-12-13 | Temporary variants without duplicate files | 12.3 | missing | Searched variant, alternative, draft in WS, UI (only item variants, ITEM:107) | Variants held in memory | M |
| UI-12-14 | Compare several variations before picking one | 12.3 | missing | As above | A strip of variants | M |
| UI-12-15 | Variants are isolated; discard or commit | 12.3 | missing | As above | Commit as one `edit` | S |
| UI-12-16 | Comparison never disturbs undo history or saved data | 12.3 | missing | No comparison. Today every preset click is a history step (WS:561) | Keep variants outside `history` and outside the file | S |
| UI-15-01 | Reliable undo and redo | 15 | partial | WS:269-270, 430-459, 577-592; TST:148, 212, 274 | Defect D-05: redo is lost by a click that changes nothing. Defect D-16: Ctrl+Shift+Z undoes. Unbounded history | S |
| UI-15-02 | Transactional edits | 15 | partial | `edit` (WS:430) and `drag` with `done` (WS:443-459) | No cancel; no step that spans several controls | M |
| UI-15-03 | Category-level reset | 15 | missing | Whole reset (WS:596) and one slider by double click (UI:486-488) only. Searched reset_category, reset_group, reset group | Reset by scope | S |
| UI-15-04 | Preset application | 15 | implemented | WS:551-567; UI:382-384; TST:369 | Whole character only | S |
| UI-15-05 | Randomization | 15 | missing | See UI-11-07 | Everything | M |
| UI-15-06 | Autosave and recovery | 15 | missing | Searched autosave, auto_save, recover, backup, crash in the workspace and APPW: none. The editor's AutoRecover is for projects (`crates/editor/src/app/autorecover.rs:1-2`) | Everything; see defect D-02 | M |
| UI-15-07 | Temporary comparison states | 15 | missing | See UI-12-13 | Everything | M |
| UI-15-08 | Viewport: feature selection | 15 | missing | See UI-3.4-12 | Everything | L |
| UI-15-09 | Viewport: region selection | 15 | missing | See UI-3.4-12. Regions in the manifest have no bones or anchors (MAN:262-269) | Everything | L |
| UI-15-10 | Viewport: contextual manipulation | 15 | missing | See UI-3.4-14 | Everything | L |
| UI-15-11 | Viewport: camera focus | 15 | partial | UI:241-253; STG:70-75 | By category, to a constant; not by feature | M |
| UI-15-12 | Viewport: selection highlighting | 15 | missing | See UI-3.1-15 | Everything | L |
| UI-15-13 | Viewport: direct manipulation handles | 15 | missing | See UI-3.4-14 | Everything | L |
| UI-15-14 | Every interaction updates the description consistently | 15 | implemented | All controls go through `edit` or `drag` and `tidy` (WS:430-459, 758-780); TST:473 | None for what exists | S |
| UI-15-15 | No editor-only selection stored as identity data | 15 | implemented | Category, symmetry, advanced, orbit, lighting, backdrop are workspace fields (WS:275-282), not description fields (DESC:20-41); UI:55-57 | `Description.extra` keeps unknown fields (DESC:37-40): do not use it for editor state | S |
| UI-16-01 | Asynchronous thumbnail loading | 16 | partial | Start page: a worker thread, a disk cache, four loads a frame (THB:14-17, 40, 253-410; SHT:299) | Nothing inside the workspace; no item pictures (ITEM:54 unused) | M |
| UI-16-02 | Efficient wardrobe browsing | 16 | partial | UI:540-552 chips per slot; `items_for` scans every item per slot per frame (WS:492-500) | No grid, search, tags, filters or virtual list. Fine for 15 items only | L |
| UI-16-03 | Efficient viewport selection and picking | 16 | missing | No picking in the workspace. Pieces: PICK:50-73; `crates/assets/src/model/rig.rs:390` | A ray test against the posed mesh at a coarse level, on click and throttled on hover | L |
| CO12-01 | Skin and makeup category | plan CO12 | partial | See UI-3.1-05 | A page and pickers | M |
| CO12-02 | Accessories category | plan CO12 | partial | See UI-3.1-06 | A page; items | M |
| CO12-03 | Expressions and animation category | plan CO12 | partial | See UI-3.1-07, UI-10 | A page; FaceControls on the preview | M |
| CO12-04 | Colour and material pickers | plan CO12 | partial | See UI-3.1-19 | Pickers | M |
| CO12-05 | Wardrobe browser: thumbnails, tags, search, filters | plan CO12 | partial | UI:540-552; tags exist and are never read by the UI (ITEM:51, `crates/avatar/src/catalogue.rs:50`) | The browser | L |
| CO12-06 | Presets and outfit presets | plan CO12 | partial | WS:145-190 | Outfit presets; presets as data (CO13) | M |
| CO12-07 | Numeric entry | plan CO12 | partial | See UI-3.1-21 | A proper field | S |
| CO12-08 | Symmetry | plan CO12 | implemented | WS:275-276, 477-489; UI:429-430; TST:183 | Defect D-11 | S |
| CO12-09 | Camera framings | plan CO12 | partial | STG:30-80; UI:297-315 | Five constants; none per feature | S |
| CO12-10 | Turntable | plan CO12 | missing | Searched turntable, spin, rotate in the workspace, the start page and the capture tool: none. Fixed cameras only (`crates/player/src/character_capture.rs:63`) | Everything | S |
| CO12-11 | Preview clips | plan CO12 | partial | See UI-3.1-34 | A clip choice | M |
| CO12-12 | Click on the body to open its category | plan CO12 | missing | See UI-3.4-12 | Picking | L |
| CO12-13 | The Game Editor entry and round trip | plan CO12, D12 | partial | Studio to game exists: a pack's Add to Project writes its characters into a project (`crates/editor/src/character_workspace/packs.rs:289`; START:597; `crates/editor/src/start/characters/cards.rs:278`; SHT:134). Game Editor to Studio does not: the workspace opens from the start screen and a flag only (`crates/editor/src/app/session.rs:1532-1537`, `crates/editor/src/main.rs:223-225`). Searched "Edit in Character Studio", open_character_studio, open_character_workspace call sites | A menu item, an action on a selected character, and the way back. One character cannot be added without a pack | L |
| CO12-14 | The face tab renamed | plan CO12, D12 | missing | Still "Character Studio": `crates/editor/src/layout.rs:205`. Searched "Face Rig" in `crates/editor/src`: none | Rename; show its editors in the workspace's Face category | M |
| CO12-15 | Panels from the manifest, basic and advanced | plan CO12 | implemented | UI:422-470; TST:402 | None | S |
| CO12-16 | `follow` constraints while editing | architecture.md section 7 item 6, section 18 | missing | MAN:246-254 declares them; no use outside `crates/avatar/src/manifest.rs:250, 539` | Apply them in `drag`; pins. No shipped manifest has one | S |
| CO12-17 | Walkthrough tests: open, edit, undo, redo, save, reopen | plan CO12 | partial | TST:148, 212, 274; APPW:143 | Through the model's methods, not through pointer and key events | M |
| CO12-18 | Drawing snapshots reviewed | plan CO12 | partial | TST:420-438: Body and Face pages | Other pages; no stored pictures to compare with | S |
| CO12-19 | Stage category | plan CO12 | implemented | WS:233; UI:566-593; STG:126-206; TST:402 | Not saved with the character, as the plan wants. See UI-3.1-13 for what it lacks | S |
| CO12-20 | Every control changes the character | plan CO12 (acceptance) | partial | Holds for what is built: TST:55 (choice), 148 (slider), 183 (symmetry), 212 (body), 274 (wear), 369 (presets) | Nothing to prove for the controls that are not built. No test moves a control through pointer events | S |

## Defects found

All by reading unless a test is named.

| ID | Severity | Defect | Evidence |
|---|---|---|---|
| D-01 | high | **Save overwrites another character with the same name, without asking.** The file is named after the character (WS:607-614) and written over whatever is there (WS:618-623). Every new character starts as "New Character" (WS:393, 398, 776-778), so saving two new characters without renaming them loses the first. Names that differ only in punctuation share a file (`crates/editor/src/character_studio.rs:570-577`). Importing a pack already guards against this by numbering the name (`crates/editor/src/character_workspace/packs.rs:197-243`); Save does not | WS:607-628 |
| D-02 | high | **Closing the editor window with unsaved changes in the workspace does not ask.** The close button asks only when `unsaved()` is true (`crates/editor/src/app.rs:1556-1559`), and that list has no entry for the workspace (`crates/editor/src/app/edits.rs:53-55`, `crates/editor/src/app/documents.rs:68-121`). The "Save changes?" question exists only on Home (APPW:73-116) | as cited |
| D-03 | medium | **Renaming a saved character saves a second file.** Save derives the path from the name and keeps no record of the file it opened (WS:268, 607-628). The old file stays and the start page shows both | WS:607-628 |
| D-04 | medium | **The name field cannot be emptied.** Each keystroke goes through `tidy`, which turns an empty name into "New Character" (UI:152-160; WS:776-778); the field reads the description again each frame | as cited |
| D-05 | medium | **Redo is lost by a click that changes nothing.** `drag` clears `future` when it starts (WS:444-448) and only then finds nothing changed (WS:454-456). A click on a slider (UI:489-491) or a visit to the name field (UI:161-163) does it | as cited |
| D-06 | medium | **The Face view aims at a constant height.** Its target is 0.905 of a 5 stud character (STG:8, 70-75). Height runs from 0.85 to 1.15 (RB:761), which moves the head by up to about 0.68 studs; at the view's distance of 3.2 and 30 degree field of view about 0.86 studs show above and below the centre (STG:73, 95). Not measured. The face tab aims at the head bone instead (`crates/editor/src/character_studio.rs:446`) | as cited |
| D-07 | medium | **The panel clones the manifest once per slider per frame.** `base()` clones the whole `BaseManifest` (WS:419-421); `parameter()` calls it (WS:463-473) for every slider and choice (UI:479, 496); each `library()` call stats a file and takes a global lock (APPLY:70-93). The Face page with Advanced on and Symmetry off draws about 50 sliders, and the workspace repaints every frame (UI:291). Not measured | as cited |
| D-08 | low | **`follow` constraints are never applied.** The contract gives them to the studio (MAN:246-254; architecture.md section 7 item 6). No shipped manifest has one | `crates/avatar/src/manifest.rs:250, 539` are the only uses |
| D-09 | low | **Groups of category `skin` and slots of category `accessories` are never drawn.** `Category::data` maps four of the six data categories (WS:247-256; MAN:97-104). No shipped data uses the other two | as cited |
| D-10 | low | **Leaving Face for Clothing keeps the face close-up**, so the clothes are out of frame (UI:245-251). A double click returns to Full body on every page, the Face page too (UI:273-275) | as cited |
| D-11 | low | **A mirrored pair whose sides differ shows one value and is flattened on the next move.** With Symmetry on, the slider shows the first side (UI:445-452, 479) and a move sets both sides to it (WS:478-486) | as cited |
| D-12 | low | **View chips never show the active view**, and the same five chips are drawn twice (UI:307, 588) | as cited |
| D-13 | low | **The status notice is never cleared** ("Saved ..." stays; WS:286, UI:598-603), and only the first problem of an apply is shown (WS:690-692) | as cited |
| D-14 | low | **Two pages are empty on the default character.** The library has three items for the realistic body, all underwear worn by the body type, so Hair shows "Nothing in the library fits" and Clothing shows a note (UI:518-563). Content, not code | `assets/characters/catalogue.json` |
| D-15 | low | **No performance guard on the slider path.** The one timing is printed (TST:154-160) | as cited |
| D-16 | low | **Ctrl+Shift+Z undoes instead of redoing.** `shortcuts` asks for Ctrl+Z first (UI:110) and egui's `consume_key` ignores an extra Shift (egui 0.36.2, `input_state/mod.rs:716-721`: "match most specific shortcuts first"), so the key is taken as undo before the redo test at UI:112-115 sees it. Ctrl+Y works. Found by the verifier, by reading | UI:108-124 |

Documents that disagree with the source:

- The plan lists numeric entry as open. The slider's value box already takes typed numbers (UI:481-485). It is partial, not absent.
- The plan's open list for CO12 leaves out four things that are also open: the face tab's rename (D12), preview clips, tags, search and filters in the wardrobe browser, and `follow` constraints (architecture.md section 18 gives them to CO12).
- `progress.md:37` says an apply takes about 6 ms "the bake included". That is the body's bake. The parts bake on the next draw and the upload follows (section 2, steps 6 and 7).
- The note at the top of brief.md points to "section 11" of the plan. The plan ends at section 10.

## Shared contracts that would have to change (the lead owns them)

| # | Contract | Change | Why |
|---|---|---|---|
| 1 | Base manifest (`crates/avatar/src/manifest.rs`, architecture.md section 6) | Optional `Parameter.description`, `.level`, `.feature`, `.random`; a `features` table with pick, anchor, focus and handles; `MaterialSlot` label, category, editable values, swatches | Section 4, facts 1 to 6 |
| 2 | `Realistic.base.json` and its `manifests.py` | Emit the new fields; anchors need vertex indices from the build | Fact 4 |
| 3 | Item manifest and catalogue | Thumbnails filled in; item-side parameters if items get their own controls (brief 14.1) | Fact 7; UI-3.3-10 |
| 4 | A preset file format (new; plan CO13) | Versioned presets with a scope (whole, body, face, hair, outfit); built-in presets become data. Favourites and locks stay editor data | UI-11-02 to UI-11-06. Presets are addressed by index today: `StartAction::NewCharacter(usize)` (`crates/editor/src/app/session.rs:1532`), THB:77-86 |
| 5 | `CharacterWorkspace`'s surface (WS:260-292) and `WorkspaceAction` (UI:17-20) | An `Intent` type, scoped reset and randomize, locks, a transaction with cancel, New and Open actions | Every UI module needs them before it can be written apart |
| 6 | The editor's unsaved-documents list (`crates/editor/src/app/documents.rs:68`, `app.rs:1556`) | Count a dirty workspace | Defect D-02 |
| 7 | A shared ray test against a posed skinned mesh | A new public function (in `scene` or `editor-tools`) from `Posed::primitive`; Model Studio's `hit_triangle` is private. `mesh_edit` has the hierarchy (`bvh.rs:40, 163`) but only inside the crate: a public constructor from positions and triangles would serve | Picking |
| 8 | `assets`: region data per vertex | Keep the second UV set or region indices, if highlight or picking uses the region map | UI-3.1-15; today one UV set (`crates/assets/src/model.rs:231-262`) |
| 9 | Staging a pose and a face on a character | The method lives in `player`'s capture tool (`crates/player/src/character_capture.rs:286-355`); the editor cannot use `player`. Move it to a shared crate | Pose, clip and expression preview |
| 10 | `editor-kit` widgets | Add the slider row with a value field, a swatch grid, a thumbnail tile | Shared by the whole editor: additions only |
| 11 | Derived models (`crates/assets/src/derived.rs`) | A budget, and an incremental or partial bake if the measurements ask for it | Section 2; brief 16 |
| 12 | The face tab's name (`crates/editor/src/layout.rs:205`, the command palette, the macOS menu twin, docs) | Rename per D12 | CO12-14 |

## File ownership notes for parallel work

- **`ui.rs` is one `impl` block of 607 lines** on `CharacterWorkspace`. Every area of the redesign touches it. One owner must split it first (section 3); nobody else starts before that lands.
- **WS holds all state as fields** and TST reaches private ones (`workspace.world`, `.assets`, `.canvas`; TST:428, 540-543). Moving a field breaks TST. Keep WS and TST with the lead.
- **THB's `Painter` owns a whole workspace** and reads its private fields (THB:196-199). A change to how the viewport draws changes the pictures. Give `Painter` one narrow method, or give THB to the viewport's owner.
- **The start page depends on presets by index** (START:47-52, 407-442; THB:77-86). Whoever changes the preset model owns those lines too, or the lead changes index to ID first.
- **APPW handles what the workspace asks for** (APPW:73-116). New toolbar actions need new `WorkspaceAction` variants handled there: the lead adds them in the contract change.
- **STG is shared** by UI, THB (THB:28, 151-158) and TST (TST:3, 448). One owner (viewport); the bottom bar reads it.
- **`editor-kit` is shared with the whole editor.** Add widgets; do not change the look of existing ones (other panels' snapshots depend on them).
- **The face tab** (`character_studio*`, `app/character_studio.rs`) is a separate tree. The rename touches `layout.rs`, the command palette and the macOS menu: one small change by one agent.
- **`app/documents.rs` and `app.rs`** are outside the studio's files (agent-assignments.md gives Agent 5 `character_workspace*`, `start/characters.rs`, `app/character_workspace.rs`). The fix for D-02 is the lead's.
- **Tests:** each new module brings its own test file and its own snapshot test. TST stays as it is.
- **`shell.rs` would sit beside `shelf.rs` and `shelf_tests.rs`**, one letter apart. Consider `frame.rs` for the shell so the wrong file is not opened or edited by mistake.
- **Builds share one target directory**, so agents cannot build at the same time in one checkout. Separate worktrees with separate target directories are needed for real parallel work.

## Risks

1. **Face features cannot be told apart by bones.** Nose, eyes, jaw and mouth are all on the Head bone. Selection needs anchors or vertex sets from the build scripts, so Stage 2 of the brief depends on a manifest change and a rebuild of the realistic package.
2. **Picking needs CPU-posed triangles.** `Posed::primitive` skins on the CPU. Doing it every frame at full detail on hover will cost. Test at a coarse level of detail, on click and throttled on hover, and measure.
3. **Every identity step re-bakes every part and rebinds the clips.** The 120 ms throttle hides it for sliders. Handles want tighter feedback, and a dressed character has more parts. Measure a dressed character in a release build before fixing a budget. The derived cache keeps up to 96 models.
4. **Left drag is the camera today.** Handles must take the drag only when hovered, or every user's muscle memory breaks and the brief's "no accidental edits" fails.
5. **Side by side doubles the work.** Two characters bake twice, and a second canvas is a second renderer with its own GPU memory.
6. **Three levels need data.** Without a level per parameter the UI would hard-code parameter IDs, against "panels from the manifest".
7. **Presets are compiled in and addressed by index.** Making them data changes the start page, its picture cache keys and the start action.
8. **Expressions in the workspace need a FaceControls on the preview character.** It must not leak into the saved description or the start page's pictures (they use the same workspace type), and automatic blinking makes snapshots differ between runs.
9. **The panel's per-frame cost grows with the manifest** (defect D-07). The brief adds many parameters.
10. **Snapshot tests skip without a GPU and compare with nothing.** UI regressions pass unseen. CI runs on release tags only, so local runs are the only check.
11. **The workspace repaints every frame** (UI:291). With a comparison view at a high resolution this is a steady GPU load.
12. **Autosave must not write into `characters/`.** The start page lists that folder every 3 seconds (START:37) and would show recovery files as characters.
13. **Symmetry means "set both sides equal"** (WS:478-486). Asymmetry features and two-sided handles need a decision: equal values or equal change.
14. **Random characters can be valid by range and still intersect.** Runtime checks only clamp (`tidy`). Crossings at pairs of extremes are checked offline by the QA scripts, not in the editor.
15. **Content gaps will read as UI gaps.** With no realistic hair or clothes, the browsers, hair presets, outfit presets and clothing comparison cannot be judged until those assets exist.

## Verification

A second agent checked this report against the source. Nothing was built or run. What was checked:

- Every bullet of brief 3.1 (38), 3.2 (22), 3.3 (21), 3.4 (27), 10 (22), 11 (12), 12 (16), 15 "Character editing" and "Viewport interaction" with the two closing rules (15), and the three bullets of 16 was counted against the table. None was absent.
- The plan's CO12 row (plan line 138) was counted against the CO12 rows. Two things had no row and were added.
- Every row marked missing was searched again under other names, in the workspace, the start page, the face tab, `avatar`, `gameplay`, `player`, `mesh_edit`, `render` and `editor-toolbox`.
- Every row marked implemented was opened at its evidence lines.
- All partial rows of 3.1, 3.3, 10, 15, 16 and CO12 were opened (more than a third of the partial rows).
- Every defect was read again in the code.
- The cited lines were opened. Apart from the ones listed below they say what is claimed.

### Corrections

| # | What changed | Why |
|---|---|---|
| 1 | UI-3.1-32 (pose selection): missing to partial | The report's own rule makes an engine-only capability partial, and UI-10-19 was partial on the same evidence. The capture tool stages rest, idle or one clip (`crates/player/src/character_capture.rs:137-147, 286-312`) |
| 2 | UI-3.1-35 (screenshot and turntable): missing to partial | The workspace's own thumbnail painter already draws a character offscreen and writes a PNG (THB:183-215, 226), and the capture tool writes fixed views and a contact sheet. No control uses either. The turntable stays missing (CO12-10) |
| 3 | UI-3.2-03 (selecting a feature shows its controls): missing to partial | The face's groups are the features and each is drawn under its own header (UI:457-460). UI-3.1-17 and UI-3.2-01 were partial on the same evidence. What is missing is the selection, not the grouping |
| 4 | UI-3.4-13 (the viewport focuses the selected region): missing to partial | UI-15-11 (camera focus) was partial on the same evidence: picking Face frames the face (UI:245-249). No test asserts it |
| 5 | CO12-13 (the Game Editor entry and round trip): missing to partial | Half of the trip exists and is tested: a pack's Add to Project writes its characters into a game project (`crates/editor/src/character_workspace/packs.rs:289`, START:597, SHT:134). The entry from the Game Editor and the way back are missing |
| 6 | UI-3.2-22: "15 groups" to 13 | The realistic manifest has 13 groups: 4 body, 9 face (RB:89-155) |
| 7 | UI-3.3-19: the gap said nothing positions the brows | Brow height exists (RB:1414, 1444). Evidence and gap rewritten |
| 8 | UI-10-01 to UI-10-08: `FPLAY` line numbers | Each was one line late. Neutral is :24, Happy :25, Sad :26, Angry :27, Surprised :28, Confused :29, Disgusted :33, Fearful :34 |
| 9 | UI-10-22: evidence | TST:473 does not test expressions. The test that proves it is IDN:601 `identity_shapes_are_baked_in_and_live_shapes_stay` |
| 10 | New defect D-16: Ctrl+Shift+Z undoes | Found while checking UI-3.1-26. `shortcuts` tests Ctrl+Z before Ctrl+Shift+Z and egui ignores the extra Shift. UI-3.1-26 and UI-15-01 now name it |
| 11 | New rows CO12-19 (Stage category) and CO12-20 (every control changes the character) | Both are in the plan's CO12 row and had no row here |
| 12 | Picking table: two rows added, the closing sentence changed, contract 7 extended | `mesh_edit` already has a ray test through a bounding volume hierarchy (`reference.rs:104`, `bvh.rs:163`), and the renderer has a line round a drawn item (`crates/render/src/draw_list.rs:2080-2094`). Neither reaches a posed skinned mesh today, so the statuses stay; the work is smaller than "write it" |
| 13 | Section 1: TST has 12 tests, not 13 | Counted: ten of the model, one that draws, one that renders |
| 14 | "Tests that exist": TST:402 asserts no text; SHT:134 added | Rows that cite TST:402 are proven to draw, not to show a named control. The shipped manifest (RB) and the panel code are the proof that a named slider is there |
| 15 | D-01: one line added | Pack import already numbers a clashing name (`packs.rs:197-243`), which shows the guard Save lacks |
| 16 | UI-3.1-15, UI-3.1-11, UI-11-06, CO12-10: evidence or gap extended, status unchanged | Pieces found under other names: the outline, the capture tool's three-quarter camera, the Toolbox's favourites. CO12-10 no longer points at a row that is now partial |
| 17 | Summary and counts | 196 rows: 33 implemented, 79 partial, 84 missing |
| 18 | Ownership notes: `shell.rs` beside `shelf.rs` | One letter apart; a likely slip for agents working by file name |

### Checked and left as they were

- **Defects D-01 to D-15** are all real by reading. D-02 was traced through `modified_documents` (`crates/editor/src/app/documents.rs:68-121`): the workspace is not in it, and no other use of `character_workspace` in `crates/editor/src/app*` guards the close. D-04 and D-05 follow from `drag` and `tidy` as described. D-06's arithmetic holds (0.68 studs of travel against 0.86 studs of half view).
- **UI-3.3-21** (three levels) stays missing. Two levels exist as one bool; the row is about the third level and a level per parameter, and nothing of that exists.
- **UI-3.4-16 to UI-3.4-20** stay missing. They describe handles; the report's gap column already says each comes free once handles write the description.
- **UI-3.1-15, UI-15-12** (highlighting) stay missing. The manifest's region table and the region map exist, but no crate reads `map_index` or the map at run time (searched `map_index`, `region_map`, `RealisticRegions` in `crates/`).
- **UI-11-02 to UI-11-05** (saving presets) stay missing. A saved character is not a preset: the start page has no "duplicate" or "start from this character" (searched duplicate, copy in `start/characters*`). Renaming and saving makes a copy only as a side effect (defect D-03).
- **Randomize, comparison, variants, locks, autosave**: searched again across `crates/` under randomi, shuffle, compar, side by side, before_after, variant, lock, autosave, recover. Nothing for characters.
- **The slider trace** (section 2) was followed through `drag`, `refresh`, `apply_description` and `resolve`. It is right: attributes and material values are outside the identity (RES:349-360, 386-391), so they do not re-bake.
- **The correction about `progress.md:37`** holds: the timed call bakes the body only (APPLY:257-264).
