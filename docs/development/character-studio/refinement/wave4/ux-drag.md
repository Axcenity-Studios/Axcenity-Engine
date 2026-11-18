# Wave 4 hand-back: ux-drag (RA1, the drag rule)

Branch `ux-drag`, from `main` at the wave 3 merge (PR #427). Task: Paul's drag rule of 2026-11-18 for every numeric customization control, in his words: "a full slider range should take approximately 100 to 120 points of horizontal drag. Hold Shift to reduce sensitivity to approximately 25% of normal. Hold Ctrl to increase sensitivity to approximately 2x normal. Allow users to enter an exact value where precision matters. Apply the same interaction rules to body, facial, and other numerical customization controls." He agreed to drop the handles' 240-point fallback.

## What changed

One helper, `crates/editor/src/character_workspace/numeric.rs`, is used by every numeric control in the studio today. The rule is **consistency**: the same drag does the same thing everywhere.

| | Before | Now |
|---|---|---|
| Slider row, whole range | egui's absolute slider: the row's width, about 300 points at a 1440-point window and about 235 at 1100 (the track is 316 and 251 points wide, measured) | **110 points** of horizontal drag at any width. The drag is relative: a press doesn't jump the value to the pointer |
| Handle, whole range | The skin under the handle followed the pointer, at least 240 points for a range (`FALLBACK`). Every handle in the seven features' views was at the 240 floor (red run) | **110 points** along the handle's screen direction at any zoom. The skin no longer follows the pointer one to one |
| Shift | Handles: a quarter. Rows: nothing (egui's slider has no fine mode) | **A quarter** on rows, values and handles |
| Ctrl | Nothing | **Double** (egui's `command`: Ctrl on Windows and Linux, Cmd on macOS). Shift and Ctrl together: half |
| A modifier pressed partway | Handles: the value jumped (the gain was applied to the whole drag) | The speed changes from that point on, with no jump (`Travel`) |
| Typed value | Advanced only (egui's `DragValue`, clamped silently) | **Every row at every level**: click or double-click the value; Enter or clicking away sets it as one undo step; Esc keeps the old value. A value outside the range is set to the range's end and the row says so for 4 s. Text that isn't a number changes nothing and says so. Opening the field and leaving it unchanged sets nothing, so the value isn't rounded to what's shown |
| Esc during a row drag | The shell cancelled the transaction, but egui's slider kept sending values, so the drag reopened a step | The row's drag stops until the button is let go (`cancelled_by_shell`), as a handle's does |
| Value shown | Two decimals | Two decimals, or three where the range's ends need them (the waist's 0.800 to 1.175) |

- **Ctrl clash check.** The studio's Ctrl shortcuts are key presses: Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z and Ctrl+S (`shell.rs`). No drag or click in the studio used Ctrl. The old "Ctrl holds the drag to its first direction" in patterns.md was never built, so nothing clashes. On macOS the helper reads egui's `command` (Cmd) because Ctrl with a click is the right button there.
- **Arrow keys.** With a track focused (Tab), each arrow press moves the value 1/110 of the range. Shift and Ctrl work as they do for a drag, and each press is one undo step. egui's slider moved one point per press, which depended on the row's width.
- **The handles.** A drive's screen vector is now its direction times `numeric::points_per_unit(range)`. The two-drive solve and the 8-point dead zone are unchanged. The dead zone is measured on the raw pointer travel, and the value follows the scaled travel. A drive's manifest `sensitivity` no longer changes the gain (architecture.md says so).
- **Controls added later.** docs/editor/character-studio.md has a "Numeric controls" bullet with a note for the specialists building the Expressions page and the browsers. A new control uses the panel's slider row, or `numeric::track`, `numeric::field` and `numeric::note`. A drag of its own uses `numeric::NumberDrag`, or `numeric::Travel` with `numeric::points_per_unit`. It never uses a bare `egui::Slider` or `egui::DragValue`. The studio has no item-parameter controls yet (the Clothing page lists items), so none were converted.

## Files changed

- `crates/editor/src/character_workspace/numeric.rs` (new): constants, `speed`, `points_per_unit`, `Travel`, `NumberDrag`, `parse_typed`, `shown`, the `track` and `field` widgets, `note`, `begin_typing`, `cancelled_by_shell`, and its unit tests.
- `crates/editor/src/character_workspace/numeric_tests.rs` (new): integration tests driven by pointer and key events, and the snapshots.
- `crates/editor/src/character_workspace/panel/row.rs`: the row draws `numeric::field`, `numeric::track` and `numeric::note`. `Row::typed` is removed.
- `panel/parameters.rs`, `panel/materials.rs`, `panel.rs`: no more `typed:`. The unused `level` parameters are dropped.
- `direct.rs`: `FALLBACK` and `FINE` are removed. The gain comes from `numeric::points_per_unit`, and the drag accumulates through `numeric::Travel` with the held modifiers. New test accessor: `handle_drives_on_screen`.
- `shell.rs`: `gesture()` is removed (it was egui slider glue). Esc taken during a drag calls `numeric::cancelled_by_shell`.
- `selection.rs`: Advanced's description no longer says "typed numbers".
- `character_workspace.rs`: the two new modules.
- Docs: `docs/editor/character-studio.md` (handles, the slider row, the new "Numeric controls" bullet, the test list), `references/ux/patterns.md` ("Drag feel" rewritten; P12, the level table and the anti-pattern row), `references/ux/README.md` (240 points out of the unsourced list), `architecture.md` (the Handle paragraph's gain), `wiki/Roadmap/Known-Limitations.md` (the 240-point note replaced), `crates/editor/CLAUDE.md` (one clause).

## Interfaces

These are inside the `character_workspace` module. No shared contract changed: no avatar schema, no `Intent` and no ADR.

```rust
pub const FULL_RANGE: f32 = 110.0; pub const FINE: f32 = 0.25; pub const COARSE: f32 = 2.0;
pub fn speed(modifiers: egui::Modifiers) -> f32;
pub fn points_per_unit(range: [f32; 2]) -> f32;
pub struct Travel<V> { pub raw: V, pub scaled: V }  // add(step, modifiers)
pub struct NumberDrag;  // new(start, range), step(points, modifiers) -> Option<f32>, cancel(), finish() -> Option<f32>
pub fn parse_typed(text: &str, range: [f32; 2]) -> Typed;  // Value { value, clamped } | NotANumber
pub fn shown(value: f32, range: [f32; 2]) -> String;
pub fn track(ui, row: &str, value, range, about) -> (egui::Response, Option<(f32, Gesture)>);
pub fn field(ui, row: &str, value, range) -> Option<(f32, Gesture)>;
pub fn note(ui, row: &str);
pub fn begin_typing(ctx, row: &str, value, range);
pub fn cancelled_by_shell(ctx);
```

## Tests

Red first: `docs/development/character-studio/refinement/wave4/ux-drag-red-run.txt`. That run had 14 tests: 13 failed and the snapshot test was skipped without `AXCENITY_SNAPSHOTS`. The helper was a stub with `todo!()` bodies, the rows still used egui's slider, and every live handle drive measured 240 points.

Green:

- `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(/numeric/)'`: **15 tests run: 15 passed**.
  - Unit tests (6): Paul's numbers (110 inside 100 to 120, Shift 0.25, Ctrl or Cmd 2, both 0.5); 110 points is the whole range and 100 falls short; a modifier pressed partway; cancel and back-to-start commit nothing; typed values clamp, and a comma reads as a decimal point; the decimals a value shows.
  - Through the studio (9): a row's whole range in 110 points at panel widths of 316 and 251 points; Shift and Ctrl on a row, one undo step each; Esc cancelling a row drag; typed values (one step, clamped with the note, Esc, not a number, unchanged text sets nothing, the value field dragging); a face row at Quick; the Skin page's melanin and blush opacity; every live drive of the seven features' handles at 110 points at two zooms; a handle drag the same at two zooms, with Shift and Ctrl; the snapshots.
- `cargo nextest run -p axcenity-editor --no-fail-fast` (whole crate): **1414 tests run: 1414 passed (13 slow), 10 skipped**, with no ABORT. An earlier full run had 1 failure, `app::mcp::tests::the_open_editor_keeps_drawing_while_a_toolbox_download_runs`, a timing test under the shared load. It passed on the rerun and isn't related.
- `cargo nextest run -p axcenity-editor -E 'test(/character_workspace/)'`: **120 passed**. That includes the R2 handle tests (`handle_tests.rs`) at the new gain.
- `cargo clippy -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all`: applied. `python3 -I scripts/check-wiki.py`: 114 pages, 0 errors (58 old staleness warnings).

## Measurements

- A whole range: 110 points (rows and handles). The tests hold it to within 2e-4 of the range on a row, and the handle screen vectors to within 0.01 points.
- Old row ranges, from the measured track widths minus the knob: about 302 points at a 1440-point window and about 237 at 1100. Old handles: 240 points for all seven features' live drives in their views, at either zoom. That is the red run, and it means the "one to one" branch never applied in those views.
- A 22-point drag on the nose tip moves `nose_length` 0.4 (a fifth of -1 to 1) at either zoom, 0.1 with Shift and 0.8 with Ctrl. Before, it moved 0.183.

## Visual evidence (`wave4/ux-drag/`)

- `numeric-typing.jpg` and `numeric-typing-row-x3.png`: the waist row being typed into ("1.12", the field framed in the accent colour, the caret at the right). The track stays at 1.000 until Enter. The text sits close to the field's right edge, and the caret touches the frame: legible, but tight at 52 points wide.
- `numeric-clamped.jpg` and `numeric-clamped-row-x3.png`: "1.5" entered. The row reads 1.175, the knob is at the right end, and an amber note under it says "1.5 is outside 0.800 to 1.175: set to 1.175". The changed bar shows.
- `skin-page-rows.png`: the Skin page. Every value row has the same framed value field and track. The tracks look as egui's did (the same rail, fill and knob), and the value boxes line up on the right.
- The full frames are cropped above the status line, which shows a temporary folder.
- **Seen, not caused by this change:** in `numeric-clamped.jpg`, the waist is at 1.175 in the Full body view and **neither waist handle is drawn**. A probe showed both handles at 1.0, only the right one at 1.1, and none at 1.175. The probe was a temporary test, removed. Handle visibility is R2's depth test (`direct/depth.rs`), which this change doesn't touch. It's probably the hands beside a widened waist hiding the handle's corner. This is for R2's owner. A person who widens the waist all the way can't then grab its handle in that view.

## Open issues

- **The waist handles hide at a wide waist** in the Full body view (above).
- **The colour buttons' picker** (`color_palette::edit_srgb`, egui's HSV picker) isn't covered by the rule. It's the kit's shared control, not a customization slider. If Paul wants it covered, it needs its own change in `editor-kit`.
- **The handles no longer keep the skin under the pointer.** That follows from Paul's rule (consistency over following the skin). On the face, the skin moves less than the pointer; on the body, where a range used to take more than 110 points of skin travel, it can move more. patterns.md and Known-Limitations say so.
- **Feel at 110 points** is pinned by tests, but no person has tried it yet. A body row of 0.8 to 1.175 now moves 0.0034 a point, about 2.7 times as fast as egui's slider at a 1440-point window (0.00124 a point). Paul's call is the number; the constant is one line (`FULL_RANGE`).
- **No item-parameter controls exist yet** to convert. The note in docs/editor/character-studio.md covers them.

## For the lead at merge

- **No generated files** were regenerated or changed. **No dependency** was added.
- **Possible conflicts:**
  - `panel/row.rs`: `Row` lost its `typed` field. Any `Row { .. typed: .. }` another specialist adds (the Expressions page, the browsers) has to drop that line.
  - `shell.rs`: `gesture()` is gone. Code that used it for an `egui::Slider` should use `numeric::track` or the row instead.
  - `materials::skin` and `materials::makeup` lost their `level` parameter.
  - `docs/editor/character-studio.md`: lines 12, 13 and 78 (the test list) are long single-line bullets that other tracks also edit.
- `progress.md`'s wave 3 entry still lists "the drag gain's 240-point fallback ... the owner's call" as open. Paul decided it on 2026-11-18 and this branch builds it, so the wave 4 progress entry can close it.
- The plan's numeric-entry item (CO12, audit PLAN-CO12-06 "partial") is met by this change: a typed value on every row at every level, with tests.
