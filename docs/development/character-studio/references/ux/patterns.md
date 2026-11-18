# Character creation UX: patterns to adopt

Stage 0 research for the Character Studio refinement ([brief](../../refinement/brief.md) §3, §4.3, §5.3, §5.4, §11, §12). Sources are in [README.md](README.md) and cited here as U-nn. Everything is described in our own words; nothing is copied.

This file recommends. It decides nothing: the plan and the lead do. It extends what exists (the `avatar` crate, format 2 descriptions, the base manifest, the workspace) and adds no second system.

Contents: [0 where the workspace stands](#0-where-the-workspace-stands-today) · [1 patterns](#1-patterns) · [2 layout](#2-the-three-panel-layout) · [3 hover, select, focus, handles](#3-hover-select-focus-and-handles-step-by-step) · [4 handle table](#4-the-handle-table) · [5 randomize, presets, favourites](#5-randomize-presets-and-favourites) · [6 comparison](#6-comparison) · [7 what to avoid](#7-what-to-avoid) · [8 legal caution](#8-legal-caution) · [9 gaps and open questions](#9-gaps-and-open-questions)

## 0. Where the workspace stands today

Source is authoritative. These are the facts the recommendations build on.

| Fact | Evidence |
|---|---|
| The layout is already toolbar, categories on the left, viewport, controls on the right, status line | `crates/editor/src/character_workspace/ui.rs:138` (toolbar), `:222` (categories), `:595` (status) |
| Six categories: Presets, Body, Face, Hair, Clothing, Stage | `crates/editor/src/character_workspace.rs:226-234` |
| The whole viewport is one click-and-drag area. Primary drag turns the camera, secondary or middle drag pans, scroll zooms, double-click frames the full body | `ui.rs:256-275` |
| Five views: Full body, Face, Front, Side, Back. Their buttons already sit in a bar over the top of the viewport, and again on the Stage page | `crates/editor/src/character_workspace/stage.rs:30-36`, `ui.rs:297-311` |
| Picking the Face category already moves the camera to the face view; going from Face to Body moves it back to the full body. No other selection moves the camera | `ui.rs:242-250` |
| Two checkboxes, Symmetry and Advanced. Advanced filters on the manifest's `advanced` flag | `ui.rs:429`, `:431`, `:445` |
| Symmetry writes the mirror parameter with the same value | `character_workspace.rs:477-484` |
| A slider drag is one undo step; double-click resets a slider | `character_workspace.rs:442-455`, `ui.rs:486` |
| Undo is a stack of whole descriptions | `character_workspace.rs:269` |
| No randomize, favourites, locks, pins or comparison exist | A search of `character_workspace.rs` and `character_workspace/ui.rs` for random, favourite, compare, snapshot, pin and lock finds only a mutex (`character_workspace.rs:307`) |
| The realistic manifest has 75 parameters in 13 groups (4 body, 9 face). Mirrored pairs name each other in `mirror`. `advanced` is a yes or no | `assets/characters/realistic/Realistic.base.json` (`groups`, `parameters`) |
| The four body groups are Frame, Build, Build by region and Proportions. No body group is a body part: shoulders, waist and hips are parameters inside Proportions | `Realistic.base.json` (`groups`) |
| Of the body parameters only `frame`, `height`, `muscle` and `fat` are not advanced. `shoulder_width`, `waist_width`, `hip_width`, `head_size`, `leg_length`, `arm_length`, `hand_size`, `foot_size` and all 16 `muscle_<region>` and `fat_<region>` parameters are flagged advanced. On the face the advanced ones are `eye_depth_*`, `ear_height_*`, `nose_tilt` and `jaw_shift` | `Realistic.base.json` (`parameters`) |
| `frame` (the body type) moves bones as well as shapes, the head bone among them, so a character's standing height is not the skeleton's `height` times the `height` parameter alone | `Realistic.base.json` (`parameters`, `frame`: `bones`) |
| No shipped base manifest declares `constraints`. `max_sum` is applied by resolve; `follow` is only in the schema and its check, and nothing in the workspace reads it or holds a pin | A search of `assets/characters/**/*.json` for `constraints` finds nothing; `crates/avatar/src/resolve.rs:46`; `crates/avatar/src/manifest.rs:247-254`, `:539` |
| The body has 14 map regions, but the whole head is one region (`head`, map index 1). No face feature has a pick region | `Realistic.base.json` (`regions`) |
| `nose_bridge` moves the bridge forward; `nose_tip` moves the tip up and forward together; `nose_length` moves the lower nose down | `assets/characters/realistic/face.py:1493-1499` |
| `hip_width` runs from 1.0 to 1.15: it only widens | `Realistic.base.json` (`parameters`) |
| Catalogue entries carry tags and a thumbnail | `crates/avatar/src/catalogue.rs:50-52` |
| The editor has its own handle code: hover test, an 8-point grab radius, and Shift for fine **turning** (a quarter speed on the rotate rings only; moving and scaling have no fine mode) | `crates/editor-tools/src/gizmo.rs:50`, `:598`, `:70`, `:713-718`, `:1157` |
| In the Game Editor's viewport, holding Alt stops a press from grabbing a handle or a part, so Alt and primary drag turns the camera | `crates/editor/src/viewport/input.rs:372-382` (handles), `:398` (a part); `:56` and `:103` are the same rule for the terrain brush and for plugin tools |
| egui already tells a click from a drag: 6 points or 0.8 s; a double click is 0.3 s | egui 0.36.2 `input_state/mod.rs:113-115` |
| A change is applied at once; during a slider drag at most every 120 ms. Applying takes about 6 ms on the realistic body in a debug build | `character_workspace.rs:69` (the 120 ms); [docs/editor/character-studio.md](../../../../editor/character-studio.md), "The character" (the 6 ms; the document's figure, not measured again here) |

Consequence: the primary button is already the camera. Direct manipulation needs a rule for when the primary button edits. Section 3 gives one.

## 1. Patterns

Each pattern: what it is, where it is seen, why it works, how it maps onto the manifest (parameters with ranges, mirrored pairs, groups).

### P1. Start from a good preset

- **What.** The first screen is a small set of finished characters or faces. Editing starts from one.
- **Seen in.** Baldur's Gate 3 goes furthest: preset heads only (U-28, U-29). Dragon's Dogma 2 picks a base head before details (U-25, U-26). inZOI opens on a random character and its guides say the base face decides most (U-19, U-21). MetaHuman's first tool blends presets (U-10, U-11). The Sims 4 starts from ready-made face and body types (U-03).
- **Why.** A person judges a face faster than they can build one. Small changes make or break a face (U-28). Slider-only tuning converges on similar faces (U-29).
- **Maps to.** A preset is plain data: parameter values and items, as `presets()` is today. Add **part presets**: the same data limited to one group or category (a nose, a face, a body). No new format; a part preset is a partial description.

### P2. Edit on the character, not beside it

- **What.** The person clicks a feature on the model and drags. The model is the control.
- **Seen in.** The Sims 4 (U-01, U-02, U-03, U-04). Character Creator's viewport morph mode (U-16, U-17). inZOI's body drag (U-19).
- **Why.** The object stays in view, the action is physical, and feedback is immediate and reversible (U-46). Nobody has to know a parameter's name.
- **Maps to.** A drag writes the same parameter a slider writes, through the same call (`set_parameter`). There is no separate geometry state (brief §3.4).

### P3. A few visible handles per feature

- **What.** After a feature is selected, two to four markers appear on it. Each marker moves one part of the feature.
- **Seen in.** MetaHuman's Transform markers (a few, broad) and Sculpt markers (more, fine) (U-12, U-13). inZOI's detailed mode nodes (U-20, unverified). Daz Studio's control point per body part (U-44).
- **Why.** A marker shows what can be changed before anything is dragged. It answers the weak point of P2: an invisible hotspot gives no hint of what a drag will do, and users report the mode as broken when a direction does nothing (U-18).
- **Maps to.** A handle is data in the manifest: a group, an anchor on the surface, an optional mirror, and up to two parameters per view. See section 4 and section 9.

### P4. Coarse first, fine on request

- **What.** The same area offers one broad control at first and several narrow ones when the person asks for detail.
- **Seen in.** The Sims 4: whole nose at head zoom; tip, nostrils and bridge at detail zoom (U-02), entered by an icon beside the head (U-04). MetaHuman: Transform then Sculpt (U-10). Black Desert mobile and inZOI: basic and detailed modes (U-24, U-19).
- **Why.** Progressive disclosure (U-45). The broad control gets most people most of the way.
- **Maps to.** Each parameter and each handle carries a level (1, 2 or 3). Today's `advanced` flag is the seed: see section 2.

### P5. The drag means what the view shows

- **What.** A drag moves the surface the way the cursor moves. In front view a sideways drag changes width; in side view it changes depth.
- **Seen in.** The Sims 4: the nose widens in front view and lengthens in profile for the same drag (U-02). Black Desert makes the person turn the character 90 degrees for depth (U-23).
- **Why.** No mapping has to be learned. The counter-example is Character Creator, where a vertical drag changes width (U-17): its users file bug reports (U-18).
- **Maps to.** Each handle lists parameters for the front view and the side view. A test can check the data: the anchor's movement on screen, for that parameter in that view, must run mostly along the stated direction.

### P6. Hover shows what is editable

- **What.** Moving over an editable area lights it up and changes the cursor before any click.
- **Seen in.** The Sims 4: a highlight overlay and a cursor that shows move, rotate or scale (U-02). Character Creator: yellow highlight (U-16).
- **Why.** Discoverability without reading (brief principle 6).
- **Maps to.** The highlight for a group can come from data already in the build: the vertices that the group's shapes move. No painted map is needed for the face. Body regions already have a map.

### P7. Feedback at the limit

- **What.** Near the end of a range the handle changes colour and stops following.
- **Seen in.** The Sims 4: a colour change approaching the limit and at it (U-02).
- **Why.** The person learns the range by feel. Without it a drag that does nothing looks like a fault.
- **Maps to.** Every parameter has a `range`; the description clamps to it. The handle tints in its last tenth and stops at the end.

### P8. Controls get out of the way while dragging

- **What.** Markers hide during a drag so the result is visible.
- **Seen in.** MetaHuman: on by default in both marker tools (U-12, U-13).
- **Why.** A marker covers exactly the millimetres being judged.
- **Maps to.** Workspace behaviour only.

### P9. Symmetry on, and visible

- **What.** Edits mirror by default. One visible toggle turns that off. Separate actions copy one side to the other.
- **Seen in.** MetaHuman (U-11 to U-13), inZOI (U-19, U-20), MakeHuman (U-43), Blender's sculpt settings with a separate symmetrize action (U-38).
- **Why.** Most faces are edited symmetrically; asymmetry is a late refinement (U-21). A separate copy action avoids surprises when the toggle comes back on: MetaHuman notes that turning symmetry on over an asymmetric face loses or keeps asymmetry depending on the tool.
- **Maps to.** `mirror` pairs exist. Add: "Left to right", "Right to left", "Reset side" as explicit actions (brief §5.4). Turning Symmetry on must not change values by itself.

### P10. The camera goes to the feature

- **What.** Choosing a feature frames it. Named framings are one click.
- **Seen in.** MetaHuman: face, body, far, hands, feet (U-09). Black Desert mobile: an icon zooms to an area (U-24). MakeHuman has it as an option that is **off by default** because it takes the person's view away (U-41).
- **Why.** Saves the zoom-and-pan before every edit. MakeHuman's default is the warning: an automatic move the person did not ask for is resented.
- **Maps to.** A framing per group can be computed from the same vertex set as the highlight (P6). Section 3 says when to move and when not to.

### P11. A slider list that can be searched and filtered

- **What.** Type to find a slider. Show only changed ones. Show only favourites. A changed slider looks different from an untouched one.
- **Seen in.** Character Creator (U-15, U-17).
- **Why.** At 75 parameters and growing, scanning fails. "Changed only" answers "what did I do to this face?".
- **Maps to.** Search over `label`. "Changed" is value not equal to `default`. Level 3 only; levels 1 and 2 never need search.

### P12. Number field manners

- **What.** Drag to change, Shift for fine steps, click to type, double-click for the default, Esc to cancel a drag.
- **Seen in.** Blender (U-36). Character Creator's typed values and step arrows (U-17).
- **Why.** They are the conventions people bring with them. Shift for fine is already the editor's own for turning a part (`gizmo.rs:70`, `:1157`); using it for a handle drag extends it.
- **Maps to.** `range` and `default`. Typed values at every level (RA1, "Drag feel" in section 3).

### P13. Pin what must not move

- **What.** A pin on a slider keeps it fixed while linked sliders move.
- **Seen in.** MetaHuman body: changing a value pins it; a group can be pinned; pinned measurements are drawn on the body (U-14). Blender locks a shape against accidental edits (U-35).
- **Why.** Linked controls feel out of control unless the person can hold a value.
- **Maps to.** The manifest schema's `follow` constraint says a target follows a source "unless the person pinned" it (`crates/avatar/src/manifest.rs:247-249`). That is schema only today: no shipped manifest declares a `follow`, and the workspace neither applies one nor has a pin. When it is built, the same pin can be the randomize lock of P15: one concept, one icon.

### P14. Blend toward a preset

- **What.** Mix the current character with a preset, for the whole face or one region.
- **Seen in.** MetaHuman's blend circle with up to three presets; one point is a region, Shift is everything (U-11).
- **Why.** "A bit more like that one" is how people describe faces.
- **Maps to.** Parameters are numbers in ranges, so a mix is a weighted average per parameter, limited to a group for a regional mix. MetaHuman's blend overwrites earlier work and cannot be walked back (U-11): ours must be one undo step.

### P15. Randomize with a scope and locks

- **What.** A dice button with a choice of what it touches, how far it goes, and what it must leave alone.
- **Seen in.** The Sims 4: a menu beside the button picks the aspects, and assets are matched by shared attributes (U-01, U-05, U-06). MakeHuman: checkboxes for scope, an amount slider, a symmetry slider, and "replace" versus "adjust current" (U-42, U-43).
- **Why.** Randomize is exploration. Without a scope it destroys work; without limits it makes monsters.
- **Maps to.** Section 5.

### P16. A catalogue with tags, filters and swatches

- **What.** Items are thumbnails in a grid. Filter chips narrow by tag. Colour variants sit on the tile.
- **Seen in.** The Sims 4 (already in [research.md](../../research.md) §A.7). inZOI: hair filtered by length; colour by wheel, hex and saved swatches; a list of what is worn with remove and hide (U-19).
- **Why.** Recognition, not recall.
- **Maps to.** Catalogue entries already carry `tags` and `thumbnail` (`catalogue.rs:50-52`). The "worn" list is the description's `items`.

### P17. Hold alternatives side by side

- **What.** Temporary slots keep a few versions to flip between.
- **Seen in.** Black Desert's "save temporarily" slots along the bottom (U-22). Dragon's Dogma 2's five slots (U-25). Blender's render slots with a key to cycle (U-40). Before and after views in photo editing (U-47).
- **Why.** Small changes are judged by flipping, not by memory.
- **Maps to.** Section 6.

### P18. A stage for judging

- **What.** Fixed lighting presets, a way to turn the light, plain-material and level-of-detail previews, poses, expressions, time of day.
- **Seen in.** MetaHuman: four environments, a hold-key drag that turns the light, clay and topology materials, eight levels of detail, toggles that hide hair and clothes (U-09). Black Desert: weather, time of day, poses, expressions (U-22).
- **Why.** A face tuned under one light fails under another. Hiding hair and clothes shows the shape being edited.
- **Maps to.** The Stage category has lighting and backdrop today. Add: turn the light, hide hair, hide clothes, a clay look. These are view state and never saved in the character.

### P19. Show what a control does

- **What.** A small picture or a live hover preview tells what a slider changes.
- **Seen in.** MakeHuman's slider pictures (U-41). The Sims 4 highlight (U-02).
- **Why.** "Bridge", "philtrum" and "gonial angle" are not words most people know.
- **Maps to.** Hovering a slider lights the same area on the character as hovering the area itself (P6). No pictures to author.

### P20. Measurements in real units

- **What.** Height in centimetres; measurements drawn on the body.
- **Seen in.** Dragon's Dogma 2 (U-25). MetaHuman's measurement lines (U-14).
- **Why.** "1.08" means nothing; "186 cm" does.
- **Maps to.** The manifest's skeleton has `height` in metres and the `height` parameter is a factor, but the product is not the answer: `frame` (the body type) also moves bones, the head among them. Read the height from the resolved skeleton. A display matter only.

## 2. The three-panel layout

Brief §3.1 gives the frame. This fills it in and says what each detail level hides.

### What goes where

| Area | Holds | Notes |
|---|---|---|
| **Top toolbar** | Home, the character's name, saved state, Undo, Redo, Reset, Randomize (a split button, section 5), Compare (section 6), the **detail level** (Quick, Detailed, Advanced), viewport settings | The detail level sits here because it changes all three panels. It is always visible |
| **Left: navigation** | The categories: Presets, Body, Face, Hair, Clothing, Skin and Makeup, Accessories, Expressions. Under the open category, its features: the manifest's groups of that category (Face: Eyes, Brows, Nose, Cheeks, Jaw, Chin, Lips, Forehead, Ears) | Narrow. A category with nothing in it for this body is not shown, as an empty slot is not shown today. Each row can show a lock (section 5) and a dot when something in it differs from the default |
| **Centre: viewport** | The character, the hover highlight, the handles. A breadcrumb at the top left ("Face › Nose") with a close mark to deselect. **Symmetry** and **Edit on character** toggles at the top right | Nothing else floats over the character |
| **Bottom of the viewport** | Views (Full body, Face, Front, Side, Back, Three-quarter), pose, lighting, turn the light, hide hair, hide clothes, turntable, screenshot. The variant strip when Compare is on | Today's Stage category moves here: it is about the view, not the character |
| **Right: context panel** | For the selected feature only: its part presets as tiles, then its sliders, then (Advanced) numbers and left and right. A header with the feature's name, Reset feature, the lock | Never a list of everything. With nothing selected it shows the category's presets and broad controls |
| **Status line** | The hint for what the pointer is over ("Drag: width and height · Shift: fine · Alt: turn the view") | The hint replaces a manual |

Side panels narrow before the viewport does, as they do today.

### What each level shows and hides

The three levels are **one flat filter**, not three nested screens. Each level shows everything the level below shows, plus more. This matters: designs with more than two nested disclosure levels test badly (U-45). A filter that is always visible in the toolbar is not nesting.

| | Quick (1) | Detailed (2) | Advanced (3) |
|---|---|---|---|
| **Left navigation** | Categories only | Categories and features | The same, plus a search field over all parameters |
| **Right panel** | Preset tiles, body type, skin tone, and the few broad sliders (height, muscle, body fat) | Part presets, then every level 2 slider of the feature | Adds level 3 sliders, left and right as two sliders, "changed only" and "favourites" filters |
| **Handles** | One per feature: the broad one | All of the feature's handles (two to four) | Adds the sideways-shift directions and per-side handles when Symmetry is off |
| **Symmetry** | Always on; no toggle shown | Toggle shown | Toggle, plus Left to right, Right to left, Reset side |
| **Randomize** | Whole character or category | Adds the selected feature | Adds amount and asymmetry |
| **Hidden** | Features list, per-parameter sliders, numbers, sides | Numbers, sides, level 3 parameters | Nothing |

Two rules keep the filter honest:

1. **Nothing changed is ever invisible.** If the current level hides a parameter that is not at its default, the panel shows a line: "2 more changes in Advanced", which switches level on click. Otherwise a lower level hides state (section 7).
2. **The level never changes the character.** Switching level changes what is shown and nothing else.

### How it maps to the manifest

- `advanced: true` becomes level 3; everything else is level 2. That needs no manifest change.
- Level 1 needs a marker for the handful of broad parameters (`height`, `muscle`, `fat`, `frame`). An optional `level` on a parameter, absent meaning "from `advanced`", would do it without a version bump, as `choices` was added.
- Features are the manifest's `groups`; categories are a group's `category`. Both exist. This holds for the face (Eyes, Brows, Nose and so on are groups). It does not hold for the body: its groups are Frame, Build, Build by region and Proportions, so the body features of section 4 (shoulders, chest, waist, hips, thighs, calves) need either new groups or a `feature` on the handle. See section 9, row 15.
- Capability-aware (brief principle 10): a body with no `ears` group shows no Ears row and no ear handles. Nothing in the workspace names a feature.

## 3. Hover, select, focus and handles, step by step

### The rule that keeps the camera and the edit apart

Creators separate the two in one of three ways:

| Way | Seen in | Cost |
|---|---|---|
| The camera has its own button | Blender: orbit on the middle button (U-39) | Needs three buttons; fails on a touchpad |
| Editing is a mode to switch on | Character Creator's viewport morph button (U-16); The Sims 4's detail edit (U-04); Black Desert mobile's advanced mode (U-24) | A mode can be forgotten; it must be visible |
| Only a visible handle edits; everything else is the camera | MetaHuman's markers (U-12, U-13) | Needs handles |

Recommended: the third, with the second as an off switch.

- **A primary drag that starts on a handle edits. A primary drag that starts anywhere else turns the camera**, as today.
- **The body itself is a click target, never a drag target.** A click on a feature selects it; a drag on it turns the camera. This is the one place we part from The Sims 4, where a drag on the body edits. It removes the conflict, and it stays clear of section 8.
- Secondary and middle drag pan, and scroll zooms, everywhere, handle or not. Unchanged.
- **Alt and primary drag always turns the camera**, even on a handle. It matches the Game Editor's viewport, where Alt stops a press from grabbing a handle (`viewport/input.rs:372-382`).
- **"Edit on character"** is a toggle on the viewport, on by default at Detailed and Advanced. Off means no hover highlight, no handles, camera only (brief §3.4: direct manipulation can be disabled). Its state shows on the toggle and in the status line.
- No gesture needs a middle button.

### The steps

| # | The person | The studio | Notes |
|---|---|---|---|
| 1 | Moves over the character, no button down | Tints the feature under the pointer and outlines it. The cursor becomes a pointing hand. The status line names it ("Nose") | Within a frame. No highlight while any button is down or the camera is moving. At Quick the areas are whole features; there is no finer hover |
| 2 | Rests there | A tooltip with the name after about 0.4 s | |
| 3 | Clicks (press and release within 6 points) | Selects the feature. The left navigation opens to it. The right panel shows its controls. The breadcrumb shows it | The same result as clicking the feature in the navigation. One selection, three places to make it |
| 4 | — | Frames the feature, over about a quarter of a second, **only if** it is small on screen or partly off it | Never moves the camera if the feature is already well in view. Never moves it during a drag. A person who has just set their own view keeps it. MakeHuman's lesson (U-41) |
| 5 | — | Shows the feature's handles (section 4). With Symmetry on, both sides' handles show and light together | Handles are a fixed size on screen. They sit on the surface and move with it |
| 6 | Moves over a handle | The handle grows. Small arrows show the directions that do something **in this view**. The status line and tooltip say what each does ("Out and in: width · Up and down: height") | A direction that does nothing in this view has no arrow. This is the cure for U-18 |
| 7 | Presses on the handle | The handle takes the pointer. Camera input from the primary button is off until release. The other handles hide (P8). The sliders this handle drives light up in the right panel | |
| 8 | Drags | Past the click distance, the parameters follow the pointer. The character updates every frame. The sliders move with it | See "Drag feel" below |
| 9 | Reaches a limit | The handle tints and stops; the pointer may go on, the value does not | P7 |
| 10 | Releases | One undo step for the whole drag, covering both parameters and both sides | The existing slider path: `set_parameter(.., done)` (`character_workspace.rs:477`) |
| 11 | Presses Esc during a drag | The drag is cancelled, values return, no undo step | Blender's convention (U-36) |
| 12 | Double-clicks a handle | Its parameters return to their defaults, one undo step | Matches the slider's double-click (`ui.rs:486`). Double-click on empty space still frames the body |
| 13 | Clicks empty space, or the breadcrumb's close mark | Deselects. Handles hide. The camera stays | Esc with no drag also deselects |
| 14 | Moves a slider instead | The same parameter changes; the handle moves on the character | Brief §3.4: the two can be mixed without loss |

### Drag feel

**Paul's drag rule of 2026-11-18** (RA1; built in `crates/editor/src/character_workspace/numeric.rs`, the studio's one numeric-drag helper) replaces the proposals that stood here: a handle that kept the skin under the pointer one to one, with a fallback of 240 points for a range that barely shows, and Ctrl to hold a drag to its first direction (never built). Paul's words: a full slider range takes about 100 to 120 points of horizontal drag; Shift about 25%, Ctrl about 2x; exact values can be typed where precision matters; the same rules for body, facial and every other numerical customization control. **The rule is consistency**: the same drag does the same to a slider row, a row's value and a handle. The other numbers in this section and in the steps above (12 points, 0.4 s, a quarter of a second, the last tenth of a range, 45 degrees) are proposals to tune by hand; none comes from a source. The 6-point click distance is egui's default.

- **A whole range takes 110 points of drag.** A slider row takes it horizontally, however wide the row (the panel narrows with the window). A handle takes it along the direction its skin moves on the screen in this view, however near the camera is: zoom changes what is seen, not the gain. So the skin under a handle no longer stays under the pointer; on the face's millimetre ranges it moves less than the pointer, on the body's broad ones it can move more. MetaHuman exposes one speed number for the same reason (U-13). A drive's `sensitivity` no longer changes the gain.
- **Shift is fine, a quarter; Ctrl is coarse, double.** Ctrl is egui's `command`: Ctrl on Windows and Linux, Cmd on macOS (where Ctrl with a click is the right button). Both held: half. A key pressed partway changes the speed from there on, so the value doesn't jump. The studio's Ctrl shortcuts (Z, Y, Shift+Z, S) are keys, not drags, so they don't clash; Shift for fine also matches the editor's turning of a part (`FINE_TURN`, `gizmo.rs:70`).
- **The drag is relative.** A press on a slider's track doesn't move the value to the pointer (egui's slider did): the value moves by how far the pointer goes, stops at the range's ends while the pointer goes on, and comes back once the pointer is back over the end. A click on the track changes nothing; a double click resets the row. With the track focused (Tab), the arrow keys move it 1/110 of the range a press, with Shift and Ctrl as a drag.
- **Exact values are typed, on every row at every level** (P12). A click or a double click on the row's value opens it for typing, its text selected; Enter, or clicking away, sets it as one undo step; Esc leaves the value as it was. A value outside the range is set to the nearest end and the row says so under it for 4 seconds ("1.5 is outside 0.800 to 1.175: set to 1.175"); text that isn't a number changes nothing and says so. A comma is read as a decimal point. The value shows two decimals, or three where the range's ends need them. The value itself also drags, by the same rule.
- **A drag is one undo step, and Esc cancels it** back to where it started, for a row as for a handle.
- **Two directions at once, with a dead zone.** A handle can drive one parameter sideways and one up and down. The first direction to pass 6 points starts; the second starts only after 12 points along its own axis, so a slightly crooked sideways drag does not also nudge the height.
- **Every frame, not every 120 ms.** The slider throttle of 120 ms would make a handle lag behind the pointer. Applying takes about 6 ms in a debug build, so a per-frame apply during a handle drag looks affordable; it has to be measured on the heavier characters before Stage 2 commits to it.
- **Out is out on both sides.** For a mirrored handle, "out" is away from the body's middle line, so dragging the left handle left and the right handle right both mean wider.

### Symmetry and sides

- Symmetry on: one drag writes both parameters of a `mirror` pair, as the slider does.
- Symmetry off: each side's handle writes its own parameter. The toggle shows its state at all times; when it is off, the unselected side's handle is drawn hollow.
- An unpaired parameter that affects both sides (chin width, shoulder width, waist width) has a handle on each side that drives the one parameter, whatever Symmetry says.

### Picking and highlighting without painting maps

- **Face features.** The head is one region today, so the region map cannot pick a nose. Two data-driven ways need no new authored map: (a) pick the feature whose handle anchors are nearest the pointer on screen; (b) at build time, write per group the set of vertices its shapes move, and hit-test and tint with that. (b) also gives P6's highlight, P10's framing and P19's slider hover from one piece of data.
- **Body regions.** The 14-region map exists (`RealisticRegions.png`, and `TEXCOORD_1`). Waist, hips, thighs, shins and upper arms are already separate regions. Shoulders and chest are both inside `torso` and need (b).
- `render` only consumes draw data (a project invariant). The tint has to arrive as draw data, like any other.

## 4. The handle table

How to read it:

- **Front view** means the camera is within 45 degrees of the character's front or back. **Side view** means within 45 degrees of a profile. Between them the nearer one wins. The arrows on the handle (step 6) always show which is live.
- **Horizontal** in front view is out from, or in toward, the middle line. In side view it is forward or back, the way the character faces.
- **Vertical** is up or down.
- Names in `code` are parameters in the realistic manifest today. A pair written `eye_height_*` is the left and right parameters. **(new)** means no parameter exists yet; the handle direction is dead until one does, and shows no arrow.
- "—" means the direction does nothing in that view.
- "Size" directions are radial: away from the feature's centre is bigger, in any view.
- "Volume" on a body region is its `muscle_<region>` or `fat_<region>` parameter: see the note under the body table.
- Level is the lowest detail level at which the handle shows (section 2).

### Face

| Feature | Handle | Level | Front: horizontal | Front: vertical | Side: horizontal | Side: vertical |
|---|---|---|---|---|---|---|
| **Nose** | Tip (centre) | 1 | Advanced only: `nose_tilt` (nose to one side) | `nose_length` (down is longer) | Tip projection (new; today `nose_tip` moves forward and up together) | `nose_tip` (up is upturned) |
| | Bridge (centre) | 2 | — | — | `nose_bridge` (forward is a higher bridge) | — |
| | Nostril (pair) | 2 | `nostril_width_*` (out is wider) | Nostril height (new) | — | Nostril height (new) |
| **Eyes** | Eye (pair, on the eye) | 1 | `eye_spacing_*` (out is wider apart) | `eye_height_*` | `eye_depth_*` (back is deeper set; flagged advanced today) | `eye_height_*` |
| | Outer corner (pair) | 2 | `eye_size_*` (out is bigger) | `eye_tilt_*` (up lifts the outer corner) | — | — |
| **Brows** | Inner end (pair) | 1 | — | `brow_height_*` | `brow_ridge` (forward is a heavier ridge; one parameter for both sides) | `brow_height_*` |
| | Peak (pair) | 2 | — | `brow_arch_*` | — | `brow_arch_*` |
| | Tail (pair) | 2 | `brow_length_*` (out is longer) | — | `brow_length_*` (back is longer) | — |
| **Cheeks** | Cheekbone (pair) | 1 | `cheekbone_*` (out is more prominent) | Cheekbone height (new) | `cheekbone_*` (forward is more prominent) | Cheekbone height (new) |
| | Cheek (pair, below the cheekbone) | 2 | `cheek_fullness_*` (out is fuller) | — | `cheek_fullness_*` (forward is fuller) | — |
| **Jaw** | Jaw corner (pair) | 1 | `jaw_width_*` (out is wider) | Jaw corner height (new) | Jaw corner forward or back (new) | Jaw corner height (new) |
| **Chin** | Chin (centre) | 1 | Advanced only: `jaw_shift` (jaw to one side) | `chin_length` (down is longer) | `chin_forward` | `chin_length` |
| | Chin side (a handle each side, one parameter) | 2 | `chin_width` (out is wider) | — | — | — |
| **Mouth** | Corner (pair) | 1 | `mouth_width_*` (out is wider) | `mouth_height` (the whole mouth up or down; one parameter) | — | `mouth_height` |
| | Upper lip (centre) | 2 | — | `upper_lip_fullness` (up is fuller) | `upper_lip_fullness` (forward is fuller) | `upper_lip_fullness` |
| | Lower lip (centre) | 2 | — | `lower_lip_fullness` (down is fuller) | `lower_lip_fullness` (forward is fuller) | `lower_lip_fullness` |
| **Ears** | Ear (pair, on the upper rim) | 1 | `ear_out_*` (out is more protruding) | `ear_height_*` (flagged advanced today) | `ear_size_*` (radial: away from the ear's centre is bigger) | `ear_height_*` |
| **Forehead** (not asked for; the parameters exist) | Forehead (centre) | 2 | — | `forehead_height` | `forehead_slope` (forward is fuller) | `forehead_height` |
| | Temple (a handle each side, one parameter) | 2 | `temple_width` (out is wider) | — | — | — |

That is 19 handles for the face, at most 3 on any one feature, 8 of them at Quick level.

### Body

| Feature | Handle | Level | Front: horizontal | Front: vertical | Side: horizontal | Side: vertical |
|---|---|---|---|---|---|---|
| **Height** | A bar above the head (centre), with the height in centimetres beside it | 1 | — | `height` (up is taller) | — | `height` |
| | Leg length: a small bar at the knee line (centre) | 3 | — | `leg_length` (up is longer legs; flagged advanced today) | — | `leg_length` |
| **Head size** | A corner handle at the top of the head, to one side | 2 | `head_size` (radial: away from the head's centre is bigger; flagged advanced today) | `head_size` | `head_size` | `head_size` |
| **Shoulders** | Shoulder (a handle each side, at the outer point) | 1 | `shoulder_width` (out is wider; one parameter; flagged advanced today) | Shoulder slope (new) | Shoulder volume (forward is more) | Shoulder slope (new) |
| **Chest** | Chest (a handle each side, at the outer chest) | 2 | Chest volume (out is more) | — | Chest volume (forward is more) | — |
| **Waist** | Waist (a handle each side, at the narrowest point) | 1 | `waist_width` (out is wider; one parameter; flagged advanced today) | — | Abdomen volume (forward is more) | — |
| **Hips** | Hip (a handle each side, at the widest point) | 1 | `hip_width` (out is wider; flagged advanced today; it cannot go narrower than the default) | — | Hip volume (back is more) | — |
| **Thighs** | Thigh (a handle each side, outer mid-thigh) | 2 | Thigh volume (out is more) | — | Thigh volume (forward is more) | — |
| **Calves** | Calf (a handle each side, outer calf) | 2 | Calf volume (out is more) | — | Calf volume (back is more) | — |

That is 9 handles for the body, 4 at Quick level. Three of those four (shoulder, waist, hip) drive parameters the manifest flags advanced today, and every volume direction is a `muscle_<region>` or `fat_<region>` parameter that is advanced too: see section 9, row 4. Arms (`arm_length`, arm volume), hands and feet follow the same shape and are not in the first set (brief §3.4 names eyes, nose, jaw, mouth, shoulders, waist and hips as the first priority).

**Volume: muscle or fat?** Each body region has two build parameters, `muscle_<region>` and `fat_<region>`. A drag has one direction for "more". Three ways to settle which it writes, for the lead to choose:

| Option | How | Trade-off |
|---|---|---|
| A. A visible two-way switch | The region's panel header has "Shape with: Muscle / Fat". The handle's tooltip says which ("Thigh size · muscle"). Default muscle for shoulders, chest, arms, thighs, calves; fat for abdomen, hips, back | One handle, no new data. It is state, so it must always be shown |
| B. By view | Front view writes muscle, side view writes fat | No switch, but arbitrary; hard to explain |
| C. Volume handles at Advanced only | At Quick and Detailed, body handles do widths and height only; muscle and fat stay on the global and regional sliders, as inZOI does with its one muscle and weight slider (U-19) | Simplest and closest to the references. Chest, thighs and calves then have no handle below Advanced |

C is the safest start; A is the better end state.

## 5. Randomize, presets and favourites

### Randomize

| Part | Recommendation | From |
|---|---|---|
| **The button** | A split button in the toolbar. The main part says what it will do in context ("Randomize face", "Randomize nose", "Randomize everything") and does it. The arrow opens a small popover, not a dialog | U-06 |
| **Scope** | Whole character, the open category, or the selected feature (brief §11). The scope follows the navigation, so there is no second place to choose it | U-05, U-42 |
| **Amount** | Subtle, Medium, Bold. Subtle moves from the **current** character; Bold starts from defaults. Level 3 shows it as a slider | U-42 ("adjust current" versus "replace") |
| **Locks** | A lock on a category row, a feature row, a slider or a worn item. Locked things are skipped. The button shows how many locks are in the way ("3 locked") | U-14, U-35 |
| **Valid results** | Draw each value around the middle of its range, not evenly across it: extremes are rare. Give a mirrored pair one value; add a small difference only when asked (level 3 "asymmetry"). Resolve enforces `max_sum` limits where a manifest declares them (no shipped manifest does yet). A parameter with `choices` takes one of its choices | U-01, U-43, architecture §7 |
| **Compatible items** | Choose only items the catalogue says fit this body and slot. Prefer items that share a tag with what is already worn, so an outfit hangs together | U-01 (matching attributes), `catalogue.rs:50` |
| **Undo** | One randomize is one undo step. Pressing it five times is five steps back | brief principle 5 |
| **Preview** | None. It applies at once; undo is the way back, and the variant strip (section 6) is the way to keep one | |

A lock is the same thing as the pin the schema's `follow` constraint expects (`manifest.rs:247-249`; not built yet, and no shipped manifest declares a `follow`). One icon, one stored set, two uses: skipped by randomize and by preset blending, and held still when a linked slider moves. A lock never blocks a direct edit of the locked thing itself: dragging a locked slider just works. Locks are workspace state, not part of the character file.

### Presets as starting points

| Part | Recommendation | From |
|---|---|---|
| **Kinds** | Whole character (exists). Body, face, one feature, hairstyle, outfit (brief §11). Each is a partial description: only the parameters of its groups, or only the items of its slots | P1 |
| **Where** | Whole-character presets on the Presets page and the start page. Part presets as the first row of the right panel for the selected category or feature | U-19 notes the lack of single-feature presets as a gap |
| **Try before applying** | Hovering a tile shows it on the character; moving away puts the character back. Clicking applies it as one undo step | Our own design. The nearest precedent is U-22, where a gallery look is put on the character and can be changed before saving |
| **After applying** | The tile is marked as the starting point. Once any of its parameters changes, the mark becomes "edited". A preset is where you started, not a mode you are in | U-21 |
| **Mix** | Under the applied tile, a "Mix" slider from the previous state to the preset. Level 2 and up | P14, U-11 |
| **Save your own** | "Save as preset" in the panel header stores the current scope. Thumbnails come from the workspace's existing offscreen picture renderer | brief §11 |
| **Locks** | Applying a part preset skips locked parameters and says so ("2 locked values kept") | P13 |

### Favourites

- A heart on preset tiles and catalogue tiles; at level 3 also on sliders (U-15).
- A "Favourites" chip in every grid's filter row, beside the tag chips.
- Stored per user beside the studio's other per-user lists (as `recent.json` is), not in the character and not in the library. A favourite that no longer exists is dropped when the list is read, as Recent drops a missing file.

## 6. Comparison

Brief §12 asks for three things. All three can be one mechanism: **held states**. A held state is a whole description kept in the workspace's memory, outside the undo stack and never written to the character file. Undo already stores whole descriptions (`character_workspace.rs:269`), so a held state is cheap, and identical descriptions share a derived model (architecture §7), so flipping back to a held state should find its model already made. That is expected from the design, not measured.

| Pattern | What the person does | What the studio does | From |
|---|---|---|---|
| **Before and after, in place** (§12.1) | Presses and holds the Compare button, or a key | Shows the "before" state while held; the camera, light, pose and animation time do not change. Release shows the current state again | U-47 (one key toggles before and after) |
| **What "before" is** | Nothing, by default | "Before" is the character as opened or last saved. "Set before to now" replaces it. Any held state can be made the "before" | U-47 (any history step or snapshot can become "before") |
| **Side by side** (§12.2) | Turns on Split | The viewport shows two characters: before on the left, current on the right, labelled. One camera drives both. Lighting, pose and animation time are shared. A link toggle frees the left camera when wanted | U-47 (left and right, top and bottom, split) |
| **Variants** (§12.3) | Presses "Hold this" | Adds the current state to a strip of up to six thumbnails under the viewport. Clicking one shows it. Number keys jump between them; one key cycles | U-22 (temporary slots along the bottom), U-40 (numbered slots, a key to cycle) |
| **Commit or discard** | "Use this" on a held state | It becomes the current character as **one undo step**. "Discard" removes it. Closing the workspace discards all of them | brief §12.3 |
| **Restore** | "Restore before" | The same as "Use this" on the before state | brief §12.1 |
| **Three actions between two states** | — | Use left, use right, swap | U-47 |

Rules:

- Looking is free. Showing a held state never adds an undo step and never marks the character unsaved. Only "Use this" changes the character.
- While a held state other than the current one is on screen, editing is off and the viewport says which state is showing. Otherwise a person edits the wrong one.
- The same strip serves preset, hairstyle and outfit comparison (brief §12.2): hold one, pick another, flip.
- A side-by-side view needs a second character in the preview. The start page's picture thread already builds a second workspace for offscreen pictures ([docs/editor/character-studio.md](../../../../editor/character-studio.md), "Pictures"), so a second preview is known ground; its cost on screen is not measured.

## 7. What to avoid

| Avoid | Why | Seen in | Instead |
|---|---|---|---|
| **A wall of sliders** | People cannot scan it, and tuning many sliders converges on similar faces | U-08, U-29; inZOI's detailed mode called overwhelming (U-19) | Show one feature's controls at a time; presets first; search only at level 3 |
| **A drag whose directions are not shown** | A direction that does nothing looks like a fault | U-17, U-18 | Arrows on the handle for the live directions; say them in the status line |
| **A mapping that fights the eye** | Vertical drag for width has to be learned | U-17 | The drag goes the way the surface goes (P5) |
| **Depth only after turning the character, unsaid** | People never find it | U-23 | The handle's arrows change with the view; the tooltip says "turn to the side for depth" when a feature has side-only directions |
| **The camera and the edit on one gesture with no rule** | Turning the view changes the face, or the reverse | brief §3.4 | Only a handle edits (section 3) |
| **An automatic camera move the person did not ask for** | It takes their view away | U-41 | Frame only when the feature is small or off screen; never during a drag |
| **An edit that silently undoes other edits** | Trust goes | U-11 (blend overwrites and cannot be walked back) | Every action is one undo step; say what was kept ("2 locked values kept") |
| **Linked sliders that move unseen** | The character changes somewhere the person is not looking | U-14 | Pins; and when a `follow` constraint moves another slider, flash that slider |
| **Modal dialogs** | They cover the character and stop the flow | brief §3.1 | Popovers anchored to their button; colour pickers inline; the only dialog is "Save changes?" on leaving |
| **Hidden state** | The same click does different things and nothing says why | U-45 | Symmetry, Edit on character, the detail level, locks, the muscle or fat switch, and which held state is showing are all visible all the time |
| **A level that hides changed values** | The character has edits the screen does not admit to | — | "2 more changes in Advanced" (section 2) |
| **More than two nested levels** | People lose their place | U-45 | The level is a flat filter in the toolbar |
| **Needing a middle button** | Laptops have none | U-37 (emulation exists because of this) | Primary drag off a handle is the camera; Alt forces it |
| **Gain that changes with zoom** | The same hand movement gives different results | brief §3.4 | Follow the pointer, or a fixed distance per range |
| **Markers that cover what they change** | The result cannot be judged | U-12, U-13 | Hide other handles during a drag |
| **Randomize that samples evenly** | Most results are extreme | U-01 | Draw around the middle; match items by tag; respect locks |
| **Randomize or presets that cannot be undone, or that wipe locked work** | Exploration becomes risk | U-11 | One undo step each; locks |
| **Drag-only editing** | Imprecise, and shuts out people who cannot drag | U-46 | Every handle has a slider; every slider takes a typed number and arrow keys at every level (RA1) |
| **Copying another product's look** | Brief §3.1; and their interfaces are their own | — | The editor's own theme and widgets |

## 8. Legal caution

This is a flag for the project's owner, not legal advice.

- Electronic Arts holds US patent 10,275,947 B2, "Modifying a simulated character by direct manipulation" (U-02). Google Patents lists it as active with an expected expiry of 2035-03-31, and marks both as its own assumption. It is a continuation of an application filed 2015-03-31.
- As its abstract reads, the method determines a zoom level, finds the cursor on the character image, identifies a modifiable portion from the cursor's location **and the zoom level**, and modifies that portion in at least two directions from the cursor's movement. The first pass read the abstract and the description. The verification pass read the claims through a fetch summary, which is not a substitute for a qualified read: 20 claims, three of them independent (1, a method; 9, a system; 15, a storage medium). As summarised, claim 1 needs all of these: the current zoom level is determined; the cursor's place on the character image is found; a modifiable portion is chosen from that place **and** the zoom level, out of at least two candidates that share the place and differ by zoom level; and the chosen portion changes in at least two directions with the cursor's movement. The highlight, the cursor shape and the colour at the limit are in dependent claims only. Someone qualified should still read the claims.
- Brief §3.4 asks for hover, click, handles and drag on the character. That is close in spirit.
- What this document recommends differs in ways that may matter, and should be kept if the owner wants distance: the body is never dragged, only clicked; edits go through **visible handles** that appear after a selection (the MetaHuman style, U-12, U-13); which handles appear depends on the **selected feature and the detail-level setting**, not on camera zoom; the detail level is a toolbar setting.
- Other shipping products drag on the character too (U-16, U-17, U-19). That says nothing about whether they hold a licence.
- Google Patents' legal events list a maintenance fee paid in 2022, so do not assume it has lapsed.
- Recommended: have this read before Stage 2 ("Interactive Viewport Customization") is built, and record the outcome as a decision.

## 9. Gaps and open questions

For the lead. None of this is decided here.

| # | Gap or question | Detail |
|---|---|---|
| 1 | **Handles are not in the manifest.** | A handle needs: an ID, a group, a label, an anchor on the surface, an optional mirror, a level, and for each of front and side view the parameter for horizontal and for vertical with a sign. As an optional manifest field it would need no version bump (the way `choices` and `correctives` were added). The build script could propose anchors: the vertex a shape moves most |
| 2 | **Three levels, one flag.** | `advanced` is yes or no. Level 1 needs a marker on a few parameters. See section 2 |
| 3 | **Handle directions with no parameter.** | Marked (new) in section 4: tip projection apart from upturn, nostril height, cheekbone height, jaw corner height and forward, shoulder slope. Brief §3.2 lists more (bridge width, nostril angle, eyelid shapes, eye opening, corner position). These are geometry work for Stage 4, to be made from references, not here |
| 4 | **Many existing flags fight the table.** | Face: `eye_depth_*` and `ear_height_*` are advanced today, yet they are the natural side-view and vertical directions of level 1 handles. Body: `shoulder_width`, `waist_width` and `hip_width` are advanced, yet they are the main direction of three of the four Quick body handles; `head_size` and every `muscle_<region>` and `fat_<region>` are advanced too. Either those handle directions are dead below Advanced, which leaves Quick with one working body handle (height), or the flags change (or the optional `level` of section 2 overrides them) |
| 5 | **`hip_width` only widens** (1.0 to 1.15). | A hip handle dragged inward does nothing. Either widen the range in the body work, or show the limit at once (P7) |
| 6 | **No face pick regions.** | The head is one region. Section 3 gives two data-driven ways; (b) needs the build to write a vertex set per group |
| 7 | **Highlight is a render feature.** | A tint over part of a mesh must reach `render` as draw data. Not designed here |
| 8 | **Per-frame apply during a handle drag.** | About 6 ms in a debug build on the realistic body, per the topic file. Not measured with clothes, hair and a second preview character |
| 9 | **Muscle or fat on a body drag.** | Three options in section 4 |
| 10 | **Stage moves out of the categories.** | Brief §3.1 puts camera, pose and lighting under the viewport. `Category::Stage` (`character_workspace.rs:233`) would go |
| 11 | **Where locks, favourites and held states live.** | Recommended: workspace state and per-user files, never the character file. Whether locks should survive closing the workspace is open |
| 12 | **The patent.** | Section 8 |
| 13 | **Sources we could not open.** | The Sims 4's randomize menu and how it keeps the camera apart from editing; Black Desert's desktop gizmo; Dragon's Dogma 2's base-face step; VRoid Studio's help centre. Listed in [README.md](README.md) "Not verified". A hands-on pass in those products by a person would settle them; nothing here depends on them |
| 14 | **Ready Player Me is closed** (U-33). | Brief §13.1 names it as a reference. It can no longer be studied first-hand |
| 15 | **Body features are not manifest groups.** | The face's groups are its features. The body's groups are Frame, Build, Build by region and Proportions. The left navigation's body features and the body handles need a home: new groups, or a `feature` on each handle, or the pick regions (`waist`, `hips`, `thighs_*`, `shins_*`) with shoulders and chest split out of `torso` |
| 16 | **`follow` and the pin are schema only.** | No shipped manifest declares a `follow` or a `max_sum`, and the workspace has no pin. The one-lock idea of section 5 is a design for when linked sliders exist, not a hook that is waiting |
| 17 | **Height in centimetres.** | `frame` moves the head bone, so the number shown must come from the resolved skeleton, not from `skeleton.height` times the `height` parameter (P20) |
