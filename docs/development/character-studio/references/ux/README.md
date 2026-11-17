# Character creation UX: reference index

Stage 0 of the Character Studio refinement ([brief](../../refinement/brief.md) §3, §11, §12, §13.1 "Character creation UX"). This folder holds two files:

- **README.md** (this file): the index of sources, with what each one shows and how it may be used.
- **[patterns.md](patterns.md)**: the interaction patterns worth adopting, the recommended layout, the handle table, and what to avoid.

Everything here is **reference only**. Nothing was downloaded. No text or picture was copied; every description is in our own words. Links only.

## How this was collected

- Collected on 2026-11-16 by web search and by opening the pages.
- **Opened** means the page was fetched and read. **Search summary** means the page refused the fetch (HTTP 403, 402 or 429) or came back cut short, and the entry rests on the search engine's summary of it. Treat a search-summary entry as a lead, not as evidence.
- Primary sources (the publisher's own documentation, a patent, a conference listing) are preferred. Guides and reviews fill the gaps where a publisher has no public documentation; that is the case for The Sims 4, inZOI, Black Desert and Dragon's Dogma 2.

## Licence classes (brief §13.5)

| Class | Meaning | In this folder |
|---|---|---|
| 1 | CC0 or public domain | None |
| 2 | Open source, compatible with an Apache-2.0 engine that redistributes it | U-48 to U-51 (code and an icon font). U-49 and U-50 are workspace dependencies already and U-48 is the icon set inside U-49; U-51 is a comparison only and is not added |
| 3 | Licensed commercial | None |
| 4 | Reference only: links, never files | Everything else |
| 5 | Original | None |
| unverified | Licence not confirmed on the source's own page | Marked per row |

A product's documentation, a game's interface, a review and a patent are all class 4: we may read them and describe a behaviour in our own words. We may not copy their text, screenshots, icons or layout.

## What the repository already held

Read these first; this index adds to them and does not repeat them.

| Where | What it already covers |
|---|---|
| [research.md](../../research.md) §A.7 | The interface requirements drawn from the private reference library: a big live viewport, click-on-body navigation, filter chips, thumbnails, one-click camera framings, progressive disclosure, an idle instead of a T-pose |
| [notes/body-face-skin-cloth.md](../../notes/body-face-skin-cloth.md) §A1, §D | MetaHuman Creator's parameterization; The Sims 4's deformer maps and the 2015 GDC talk as a *technical* reference |
| [notes/hair.md](../../notes/hair.md), [notes/formats-and-libraries.md](../../notes/formats-and-libraries.md) | Hair and formats; nothing on interaction |

New here: how the interaction itself works (hover, select, handles, drag mapping, camera conflicts, randomize, compare), from the publishers' own pages where they exist.

## Index

Columns as brief §13.4 asks. "Date collected" is 2026-11-16 for every row. "Use" is visual guidance or actual asset, as brief §13.4 asks.

### The Sims 4 Create a Sim

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-01 GDC 2015 session listing, "Innovations in The Sims 4 Character Creator" | https://www.gdcvault.com/play/1022085/Innovations-in-The-Sims-4 | Sri Nair, Electronic Arts; GDC Vault | 2026-11-16 | Direct manipulation, randomize | Opened (the overview only; the recording is behind the Vault). Push and pull on the character runs on a data-driven set of selectors and modifiers. Smart randomization picks assets that share attributes so results look consistent | Class 4. Copyright the publisher | visual guidance |
| U-02 US patent 10,275,947 B2, "Modifying a simulated character by direct manipulation" | https://patents.google.com/patent/US10275947B2/en | Electronic Arts Inc. (assignee) | 2026-11-16 | Hover, select, drag mapping, zoom levels | Opened. The fullest primary description of the mechanism: a region map stores each hotspot as a colour; hover overlays a highlight and changes the cursor; three zoom modes (whole body, head, face detail) give the same area different hotspots; the same drag does different things by view angle (nose in front view: wider; in profile: longer); a key constrains a diagonal drag to one direction; a colour change warns near the limit. The claims were read in the verification pass through a fetch summary: 20 claims, three independent (1, 9, 15); the zoom level is an element of claim 1 | Class 4, and **an active patent** (Google Patents lists it as active, expiry 2035-03-31, as an assumption; its legal events show a maintenance fee paid in 2022). See patterns.md "Legal caution". Not legal advice | visual guidance |
| U-03 Developer blog post on the new Create a Sim (2014), as reposted | https://simscommunity.info/2014/05/21/up-close-and-personal-with-the-new-create-a-sim-in-the-sims-4/ | Written in the developer's voice; hosted by Sims Community | 2026-11-16 | Direct manipulation, detail mode, genetics | Opened. Start from ready-made face and body types, then drag to reshape. A detail mode zooms in on eyes, ears, nose, chin and mouth. Two characters can be mixed to preview a child | Class 4. A repost; the original page is gone | visual guidance |
| U-04 Game guide: appearance | https://www.gamepressure.com/thesims4/appearance/zb6858 | Gamepressure | 2026-11-16 | Detail edit entry point | Opened. Clicking the face opens face editing; an icon beside the head enters detail edit; body parts resize by dragging left or right on them | Class 4 | visual guidance |
| U-05 Game guide: Create a Sim tips | https://primagames.com/tips/sims-4-create-sim-tips-and-tricks | Prima Games | 2026-11-16 | Randomize scope, styled looks | Search summary (fetch refused). The randomizer can be limited to chosen aspects, up to seven. Styled looks give a whole outfit in one pick | Class 4 | visual guidance |
| U-06 Player forum thread on the randomize button | https://forums.ea.com/discussions/the-sims-4-general-discussion-en/why-do-all-my-sims-look-the-same/704013/replies/704019 | EA Forums (a player's post) | 2026-11-16 | Randomize scope | Search summary (fetch refused). An arrow beside the randomize button opens the choice of what is randomized | Class 4. A player's account, not the publisher's | visual guidance |
| U-07 Patch coverage, July 2019: a quiz-led way to start a character | https://simscommunity.info/2019/07/16/the-sims-4-new-update-patch-notes-july-16th-2019/ | Sims Community | 2026-11-16 | Guided start | Search summary. An optional short questionnaire sets a character's starting traits; the free-form creator stays available | Class 4 | visual guidance |
| U-08 Paper: "Entering the Design Space of Digital Portraiture: A Case Study in Avatar Creation Tools" | https://mkremins.github.io/casual-creators-workshop/papers/ICCC20_paper_197.pdf | J. T. Otto and A. G. Forbes, University of California, Santa Cruz; Casual Creators workshop, ICCC 2020 | 2026-11-16 | Sliders versus direct manipulation | Opened (PDF text read). Argues that slider arrays break down as features multiply, and that The Sims 4 replaces the slider panel with direct manipulation over curated controls, split into a coarse layer and detail layers. Creators ship "known good" starting faces. The paper's own case study is The Sims 3; The Sims 4 is a short comparison in it | Class 4. No licence stated in the PDF: unverified | visual guidance |

### MetaHuman Creator (the Unreal Engine version)

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-09 Navigating MetaHuman Creator | https://dev.epicgames.com/documentation/metahuman/navigating-metahuman-creator-in-unreal-engine | Epic Games | 2026-11-16 | Layout, camera, lighting, level of detail | Opened. A toolbox on the left (categories, tools, properties), the viewport, a hotkey list that changes with the tool. Five framing presets (face, body, far, hands, feet), three lens presets, four lighting environments, a light-rotation slider and a hold-key-and-drag to turn the light. Eight levels of detail to preview. Clay and topology preview materials. Toggles to hide hair and clothes while editing | Class 4. Copyright Epic Games | visual guidance |
| U-10 Head and Body tools overview | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-head-and-body-tools-in-unreal-engine | Epic Games | 2026-11-16 | Tool ladder | Opened. Four tools from broad to fine: Blend, Body Params, Head Transform, Head Sculpt. The suggested order is broad first, fine last | Class 4 | visual guidance |
| U-11 Blend tool | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-blend-tool-in-unreal-engine | Epic Games | 2026-11-16 | Preset blend circle | Opened. Up to three presets sit on a circle; control points on the character are dragged toward a preset; one point blends locally (the page does not say how large an area a point covers), Shift blends the whole head or body. Symmetry on by default for the head. Blending overwrites earlier edits and moving back does not restore them | Class 4 | visual guidance |
| U-12 Head Transform tool | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-head-transform-tool-in-unreal-engine | Epic Games | 2026-11-16 | Region markers | Opened. A few markers each move a whole facial region; move, rotate and uniform scale; the default drag is in the screen plane. Size and speed settings; markers hide while dragging; symmetry on by default | Class 4 | visual guidance |
| U-13 Head Sculpt tool | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-head-sculpt-tool-in-unreal-engine | Epic Games | 2026-11-16 | Fine markers | Opened. Markers appear on the face without being asked for; each has a small area; areas overlap. Ctrl and click adds or removes a marker. Drag speed defaults to 0.2. The result is approximated by the closest combination of scanned facial features in the database. That a drag therefore cannot make a broken face is our reading, not the page's words | Class 4 | visual guidance |
| U-14 Body Params tool | https://dev.epicgames.com/documentation/metahuman/metahuman-creator-body-params-tool-in-unreal-engine | Epic Games | 2026-11-16 | Correlated sliders, pinning | Opened. Four global sliders (masculine or feminine, muscularity, fat, height), then measurements by region. Sliders are statistically linked, so moving one moves others. Changing a value pins it; a pin icon locks or frees a value; a whole group can be pinned; a pinned measurement is drawn on the body | Class 4 | visual guidance |

### Character Creator 4 and 5

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-15 Morphs tab | https://manual.reallusion.com/Character-Creator-4/Content/ENU/4.0/04_Introducing_the_User_Interface/Modify_Morphs_Tab.htm | Reallusion | 2026-11-16 | Slider list with search | Opened. A tree of categories; a filter drop-down; type to search by name; a "currently used" view of every changed slider; a "favourite" view of sliders marked with a heart; reset and bake | Class 4. Copyright Reallusion | visual guidance |
| U-16 Control section (4.21) | https://manual.reallusion.com/Character-Creator-4/Content/ENU/4.21/04-Introducing-the-User-Interface/Control-Section.htm | Reallusion | 2026-11-16 | Dedicated edit mode | Opened. Direct morphing in the viewport is a button to switch on. The area under the cursor lights up yellow. A separate setting zooms toward the cursor instead of the centre | Class 4 | visual guidance |
| U-17 Setting values (5.0) | https://manual.reallusion.com/Character-Creator-5/Content/ENU/5.0/04-Introducing-the-User-Interface/Setting_Parameter_Value_in_Modify_Page.htm | Reallusion | 2026-11-16 | Drag mapping, changed-value marker | Opened. Three ways to set a value: slider, step arrows, typed number. A changed slider's knob is drawn solid, an untouched one hollow. On the face a vertical drag changes width and a horizontal drag changes height or length; on the body a drag along the bone changes length and a drag across it changes width and depth | Class 4 | visual guidance |
| U-18 Bug report: only one drag axis works | https://reallusion.com/FeedBackTracker/Issue/Only-vertical-axis-work-in-Morph-mode-while-editing-with-the-mouse | Reallusion feedback tracker (a user's report) | 2026-11-16 | Risk of unlabelled drag axes | Search summary. A user reported that one drag direction stopped doing anything while the other still worked; a second report says dragging did nothing at all. A dead direction reads as a fault | Class 4 | visual guidance |

### inZOI

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-19 Review of the character creator | https://simscommunity.info/2025/04/01/inzoi-create-a-zoi-review/ | Sims Community | 2026-11-16 | Basic and detailed editing, catalogue | Opened. Starts on a random character. Presets, or basic or detailed editing. A symmetry toggle. The body is edited by dragging on it plus a muscle and weight slider. A list of what is worn, with remove and hide. Hair filtered by length. Colour by wheel, hex code and saved swatches. The reviewer found detailed face editing overwhelming and missed presets for single features | Class 4 | visual guidance |
| U-20 Community wiki: face customization | https://thegameswiki.com/inzoi/wiki/face-customization | The Games Wiki (community) | 2026-11-16 | Drag nodes, symmetry | Search summary (fetch refused twice). A detailed mode shows drag nodes across the face, with separate nodes for nose bridge, tip and nostrils. A half-circle guide around each eye rotates it. Symmetry is on by default; turning it off edits each side alone | Class 4. Licence not read: unverified | visual guidance |
| U-21 Tips article | https://www.pcgamer.com/games/life-sim/inzoi-character-studio-tips | PC Gamer | 2026-11-16 | Preset first | Search summary (the fetch came back cut short). The base face preset decides more than later edits can undo; turning symmetry off adds lifelike irregularity | Class 4 | visual guidance |

### Black Desert

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-22 Guide: character creation and the shared gallery | https://www.blackdesertfoundry.com/?p=293 | Black Desert Foundry | 2026-11-16 | Temporary saves, edit history, stage | Opened. "Save temporarily" slots along the bottom of the screen hold alternatives to return to. An edit-history tab gives undo and redo. Files save to and load from disk. A shared gallery look is applied to the character from its thumbnail and can then be changed before saving. Weather, time of day, expressions and poses can be previewed. Hair is reshaped by dragging a strand | Class 4 | visual guidance |
| U-23 Guide: character creation basics | https://www.gamepressure.com/blackdesert/character-creation/z18711 | Gamepressure | 2026-11-16 | Three axes per region | Opened. A region is changed along height, width and depth, plus slant and spin. Depth needs the character turned 90 degrees first | Class 4 | visual guidance |
| U-24 Official guide, mobile edition | https://www.world.blackdesertm.com/Ocean/Wiki?wikiNo=34 | Pearl Abyss | 2026-11-16 | Basic and advanced modes | Opened. Ten presets, then a basic mode (icons on the left zoom to an area) and an advanced mode (press a shaded part of the face to change its length, width and depth) | Class 4. Copyright Pearl Abyss | visual guidance |

### Dragon's Dogma 2 and Baldur's Gate 3

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-25 Guide: character creator | https://www.gamepressure.com/dragons-dogma-2/character-creator/zc112ce | Gamepressure | 2026-11-16 | Staged flow, save slots | Opened. Body, then a base head and face, then details, then name and voice. Five save slots per character type. Height is shown in centimetres (160 to 215) | Class 4 | visual guidance |
| U-26 Article: the creator's base heads | https://www.gamesradar.com/why-is-dragons-dogma-2s-character-creator-so-good-its-director-says-the-rpgs-team-scanned-nearly-80-faces-for-base-head-variations-for-starters | GamesRadar+ | 2026-11-16 | Scanned starting faces | Opened (the start of the article; the rest was cut short). The director says the team scanned nearly 80 faces as base head variations | Class 4 | visual guidance |
| U-27 News: the stand-alone creator | https://www.gematsu.com/2024/03/dragons-dogma-ii-character-creator-storage-tool-now-available | Gematsu | 2026-11-16 | Creator as its own application | Search summary. The creator shipped ahead of the game as a free application; saved characters carry over | Class 4 | visual guidance |
| U-28 Article: how the creator was designed | https://www.gamedeveloper.com/art/building-an-inclusive-character-creator-for-the-fantasy-world-of-baldur-s-gate-3 | Game Developer | 2026-11-16 | Presets instead of sliders | Opened. The studio chose preset faces plus attachments (hair, scars, tattoos) over sliders: millimetre changes make or break a face, and slider-heavy creators risk bland faces. They name sliders as something to do better later. Advice: make it cheap for artists to add choices | Class 4 | visual guidance |
| U-29 Article: why there are no face sliders | https://80.lv/articles/larian-explained-why-there-are-no-face-sliders-in-baldur-s-gate-3 | 80 Level | 2026-11-16 | Presets instead of sliders | Opened. Players who tune sliders for hours tend to arrive at similar faces; a slider system must produce truly distinct results to be worth it | Class 4 | visual guidance |

### VRoid Studio and Ready Player Me

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-30 Store page | https://store.steampowered.com/app/1486350/VRoid_Studio/ | pixiv | 2026-11-16 | Slider groups, hair | Search summary. Parameters sit on the right, grouped by face part. Hair is built from preset groups or drawn freehand, with thickness, density and curl as parameters; bounce is set per hair group | Class 4. The official help centre (vroid.pixiv.help) refused the fetch: its articles are unverified | visual guidance |
| U-31 Release note: expressions in the face editor | https://vroid.com/en/news/202012171900 | pixiv | 2026-11-16 | Expression tab beside shape | Search summary. The face editor has a tab where each expression's face parts are adjusted, beside the tab for the neutral face | Class 4 | visual guidance |
| U-32 Press release for the stable version | https://www.animenewsnetwork.com/press-release/2021-11-02/the-3d-character-maker-vroid-studio-stable-ver-is-out-now-for-free/.179117 | Anime News Network (a press release) | 2026-11-16 | Scale of the slider set | Opened. More than 100 sliders in the face editor, including eye and nose shape and neck thickness | Class 4 | visual guidance |
| U-33 News: the acquisition | https://techcrunch.com/2025/12/19/netflix-acquires-gaming-avatar-maker-ready-player-me/ | TechCrunch | 2026-11-16 | Status of the product | Opened. Ready Player Me was acquired (article dated 2025-12-19). The same article says its services, the online avatar creator among them, wind down on 2026-01-31. Its documentation host no longer resolves (our fetch of docs.readyplayer.me failed with a name-lookup error) | Class 4. **The product can no longer be studied first-hand** | visual guidance |
| U-34 Blog post: body shape presets | https://readyplayer.me/blog/introducing-avatar-body-shape-customization | Ready Player Me | 2026-11-16 | Very small choice sets | Search summary; the page may be gone. Body shape was four presets to pick from, not a slider | Class 4. Unverified | visual guidance |

### Blender

The manual's licence was read at https://docs.blender.org/manual/en/latest/copyright.html : Creative Commons Attribution-ShareAlike 4.0, logos and icons excluded. ShareAlike does not suit copying into this repository's documentation, so these are class 4: read and describe, do not copy.

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-35 Shape Keys panel | https://docs.blender.org/manual/en/latest/animation/shape_keys/shape_keys_panel.html | Blender Documentation Team | 2026-11-16 | Per-shape lock and mute | Opened. Each shape has a value, a mute toggle and a lock that protects it from accidental edits; a new shape can be made from the current mix; shapes can be mirrored across the middle | Class 4 (CC BY-SA 4.0) | visual guidance |
| U-36 Number fields | https://docs.blender.org/manual/en/latest/interface/controls/buttons/fields.html | Blender Documentation Team | 2026-11-16 | Slider conventions | Opened. Drag to change; Ctrl snaps to steps; Shift is fine control; click to type; Esc or the right button cancels a drag; a soft limit bounds dragging while typing can reach a wider hard limit; several fields can be dragged at once | Class 4 (CC BY-SA 4.0) | visual guidance |
| U-37 Input preferences | https://docs.blender.org/manual/en/latest/editors/preferences/input.html | Blender Documentation Team | 2026-11-16 | Click versus drag | Opened. A drag threshold in pixels decides whether a press is a click or a drag, with a separate one for tablets. Release-confirms is an option. A three-button mouse can be emulated with Alt and the left button | Class 4 (CC BY-SA 4.0) | visual guidance |
| U-38 Sculpt symmetry | https://docs.blender.org/manual/en/latest/sculpt_paint/sculpting/tool_settings/symmetry.html | Blender Documentation Team | 2026-11-16 | Symmetry as a visible toggle | Opened. Mirroring is a per-axis toggle in the tool's own settings; a separate "symmetrize" action copies one side to the other in a chosen direction | Class 4 (CC BY-SA 4.0) | visual guidance |
| U-39 Viewport navigation | https://docs.blender.org/manual/en/latest/editors/3dview/navigate/navigation.html | Blender Documentation Team | 2026-11-16 | Camera on its own button | Opened. Orbit lives on the middle button, so the left button is free for tools. Alt while orbiting snaps to axis views | Class 4 (CC BY-SA 4.0) | visual guidance |
| U-40 Image editor: render slots | https://docs.blender.org/manual/en/latest/editors/image/introduction.html | Blender Documentation Team | 2026-11-16 | Compare slots | Opened. Results go into numbered slots; one key cycles forward and another back; number keys jump to a slot; slots can be renamed | Class 4 (CC BY-SA 4.0) | visual guidance |

### MakeHuman and Daz Studio

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-41 Configuration and settings | https://static.makehumancommunity.org/makehuman/docs/configuration_and_settings.html | MakeHuman Community | 2026-11-16 | Auto-focus, slider pictures | Opened. An option zooms the camera to the area being edited; it is off by default because it takes away the user's own view. Sliders can carry a picture of what they change; the pictures can be hidden for a compact list | Class 4. The documentation's licence was not shown on the page: unverified | visual guidance |
| U-42 Gender, Random, Measure and Custom | https://static.makehumancommunity.org/makehuman/docs/gender_random_measure_and_custom.html | MakeHuman Community | 2026-11-16 | Randomize with scope and amount | Opened. Checkboxes choose what is randomized (macro shape, height, face). One slider sets how much; one sets how asymmetric. Two buttons: replace the character, or nudge the current one | Class 4. Unverified, as above | visual guidance |
| U-43 Randomizing details (the Blender add-on) | https://static.makehumancommunity.org/mpfb/docs/randomization/details.html | MakeHuman Community | 2026-11-16 | Symmetric randomize | Search summary. Symmetry is on by default when randomizing, so a left and right pair get one value | Class 4. Unverified | visual guidance |
| U-44 Introduction to the posing tools | https://www.daz3d.com/blog/daz-bytes-intro-to-posing-tools-in-daz-studio | Daz 3D | 2026-11-16 | Control points for the whole body | Opened. A pane shows a control point per body part. Clicking one selects the part in the viewport; dragging it moves the part. The advice is to start from a preset and refine | Class 4 | visual guidance |

### Usability and comparison

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-45 Progressive Disclosure | https://www.nngroup.com/articles/progressive-disclosure/ | Jakob Nielsen, Nielsen Norman Group (2006) | 2026-11-16 | Detail levels | Opened. Show the few most-used options first. Two rules: split the features correctly, and make the way to the rest obvious. **More than two levels usually tests badly** because people lose their place. A step-by-step sequence (staged disclosure) is a different thing and suits independent steps | Class 4 | visual guidance |
| U-46 Direct Manipulation: Definition | https://www.nngroup.com/articles/direct-manipulation/ | S. Sherugar and R. Budiu, Nielsen Norman Group (2016, reviewed 2024) | 2026-11-16 | Direct manipulation | Opened. Three properties: the object stays visible, actions are physical, feedback is fast, small and reversible. Weak points: precision, repetition, things off screen, accidental activation, and people who cannot drag | Class 4 | visual guidance |
| U-47 The History panel and Before and After view | https://jkost.com/blog/2024/08/the-history-panel-and-before-after-view-in-lightroom-classic.html | Julieanne Kost (an Adobe evangelist's own site) | 2026-11-16 | Before and after, side by side | Opened. One key toggles before and after in place. Other keys give left and right, top and bottom, and a split of one picture. Any history step or saved snapshot can be made the "before". Three actions: copy before to after, after to before, swap | Class 4 | visual guidance |

### Usable in the engine (class 2)

These are code and an icon font, not character content. The engine already has its own gizmo (`crates/editor-tools/src/gizmo.rs`), so U-51 is a comparison, not a recommendation to add it.

| Reference | Source URL | Author or publisher | Date collected | Feature | What it demonstrates | Licence or restrictions | Use |
|---|---|---|---|---|---|---|---|
| U-48 Phosphor Icons | https://github.com/phosphor-icons/core | Phosphor Icons | 2026-11-16 | Icons for categories, lock, dice, compare, symmetry | The icon set the editor already draws with | Class 2: MIT. Read in the repository's `LICENSE` file ("MIT License, Copyright (c) 2023 Phosphor Icons") | actual asset |
| U-49 egui-phosphor | https://github.com/amPerl/egui-phosphor | amPerl | 2026-11-16 | The same icons as a font for egui | Already pinned in the workspace (`Cargo.toml:88`, version 0.14) | Class 2: MIT OR Apache-2.0 for the crate (read in the repository's `Cargo.toml` and in the 0.14.0 crate as downloaded by cargo, which carries both licence files). The icon fonts inside it are Phosphor's, MIT (U-48), as the crate's README says | actual asset |
| U-50 egui | https://github.com/emilk/egui | Emil Ernerfeldt and contributors | 2026-11-16 | Click versus drag, sliders, panels | Already the editor's UI library. Its input defaults separate a click from a drag at 6 points and 0.8 s, and a double click at 0.3 s (egui 0.36.2, `input_state/mod.rs:113-115`) | Class 2: MIT OR Apache-2.0 for the code (read in the repository's `Cargo.toml` and in the 0.36.2 crate's). The default fonts egui bundles are not under that licence alone: the `epaint_default_fonts` 0.36.2 crate declares `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`, and the workspace turns `default_fonts` on (`Cargo.toml:75`). Already a dependency; nothing new is added here | actual asset |
| U-51 transform-gizmo | https://github.com/urholaukkarinen/transform-gizmo | Urho Laukkarinen | 2026-11-16 | A ready-made move, rotate and scale gizmo for egui | A comparison point for handle picking and drawing. Its 0.11 targets egui 0.36 | Class 2: MIT OR Apache-2.0. Read in the repository's `Cargo.toml` (version 0.11.0) | visual guidance |

## Not verified

- **The Sims 4** has no public developer documentation of Create a Sim. The mechanism comes from the patent (U-02) and the GDC overview (U-01); what the randomize menu and styled looks do comes from guides and forum posts we could not open (U-05, U-06). How the camera is kept apart from editing (which button turns the character) was not found in any source we could open.
- **Black Desert**'s desktop creator: no source we could open describes its on-face gizmo. U-23 gives the axes; U-24 is the mobile edition.
- **Dragon's Dogma 2**: whether base faces are refined by picking from a grid of near matches was not confirmed. Do not design on it.
- **VRoid Studio**: the official help centre refused every fetch. Rows U-30 and U-31 are search summaries; U-32 (a press release) was opened in the verification pass.
- **Ready Player Me**: closed (U-33, opened in the verification pass). Nothing about its creator was confirmed first-hand; U-34 is still a search summary.
- **inZOI**: the drag nodes are from a community wiki we could not open (U-20).
- Adobe's own help page for the before and after view refused the fetch; U-47 stands in for it.
- The 2013 interview already cited in [notes/body-face-skin-cloth.md](../../notes/body-face-skin-cloth.md) §D was not opened again.

## Verification

A second pass on 2026-11-16 checked this folder against the sources and against the repository. Source and tests were taken as authoritative. It changed the two files in place; every change is listed here.

### What was checked

| Check | Result |
|---|---|
| Licences of the class 2 rows (U-48 to U-51) | All four confirmed on the source's own files. U-48: `LICENSE` in the repository, MIT, copyright 2023 Phosphor Icons. U-49: `Cargo.toml` in the repository and in the 0.14.0 crate, MIT OR Apache-2.0; its README says the icons are Phosphor's, MIT. U-50: `Cargo.toml` in the repository and in the 0.36.2 crate, MIT OR Apache-2.0. U-51: `Cargo.toml` in the repository, version 0.11.0, MIT OR Apache-2.0, egui 0.36 |
| A sample of other rows opened again | 26 rows: U-01, U-02, U-04, U-08, U-09, U-11, U-12, U-13, U-14, U-16, U-17, U-19, U-22, U-24, U-25, U-26, U-28, U-29, U-32, U-33, U-36, U-40, U-41, U-42, U-45, U-47. Every URL exists and shows what the row says, apart from the wording fixes below. U-05 refused the fetch again and stays a search summary |
| The Blender manual's licence | Confirmed on its licence page: CC BY-SA 4.0 or later; logos, trademarks, icons, source code and scripts excluded |
| Index hygiene | 51 rows, eight columns each, none empty. Nothing from a commercial game or tool is above class 4. The folder holds two Markdown files and nothing downloaded. No local paths and no private people; the names are authors and publishers of the sources. No copied passages; the only quotations are a licence line and a few short interface labels |
| Numbers | These files state no population mean, SD or range. The figures they do state were checked against their sources: see the next table |
| Claims about this repository | Every `path:line` in patterns.md section 0 was opened. Three were wrong or loose and six facts were missing: see the corrections |

### Numbers confirmed

| Number | Where it is used | Confirmed in |
|---|---|---|
| Height 160 to 215 cm; five save slots for each of two characters | U-25 | The guide's page |
| Eight levels of detail, five framings, three lenses, four lighting environments | U-09 | Epic's page |
| Drag speed 0.2 by default | U-13 (and the same default on U-12) | Epic's pages |
| Four global body sliders | U-14 | Epic's page |
| Up to three presets on the blend circle | U-11 | Epic's page |
| More than 100 sliders | U-32 | The press release |
| Nearly 80 scanned faces | U-26 | The article's opening |
| Ten presets | U-24 | The publisher's guide (the default look plus ten more) |
| Expiry 2035-03-31, marked as an assumption | U-02 | Google Patents |
| Published 2006 | U-45 | The article's date line |
| Click distance 6 points, click time 0.8 s, double click 0.3 s | U-50 | `egui` 0.36.2 `src/input_state/mod.rs:113-115` |
| Grab radius 8 points; fine turning at a quarter speed | patterns.md section 0 | `crates/editor-tools/src/gizmo.rs:50`, `:70` |
| 120 ms between applies during a slider drag | patterns.md section 0 | `crates/editor/src/character_workspace.rs:69` |
| 75 parameters, 13 groups, 14 map regions, `hip_width` 1.0 to 1.15 | patterns.md section 0 | `assets/characters/realistic/Realistic.base.json` |

Numbers with no source: "up to seven" randomize aspects (U-05, a search summary that could not be opened); about 6 ms per apply (the topic file's figure, not measured again); and the tuning values in patterns.md section 3 (240 points, 12 points, 0.4 s, a quarter of a second, the last tenth of a range, 45 degrees) and section 6 (six variants), which are proposals. Section 3 now labels them so.

### Corrections to this file

| Row | Change |
|---|---|
| U-02 | Added what the claims say (20 claims; 1, 9 and 15 independent; the zoom level is an element of claim 1) and that a maintenance fee was paid in 2022. The claims were read through a fetch summary, not by a qualified person |
| U-08 | Added that the paper's case study is The Sims 3 and that The Sims 4 is a short comparison in it. "Every creator" became "creators" |
| U-11 | "One point blends a region" became "one point blends locally": the page does not say how large an area a point covers. Symmetry on by default is stated for the head only |
| U-13 | "A drag cannot make a broken face" is now marked as our reading. The page says the result approximates the closest combination of scanned features |
| U-22 | The gallery look is applied to the character and can then be changed before saving. The row said it could be tried before it was applied |
| U-26, U-32, U-33 | Were search summaries; opened in this pass and confirmed. U-33's article itself gives the wind-down date, 2026-01-31 |
| U-49 | The licence cell now separates the crate (MIT OR Apache-2.0) from the icon fonts inside it (MIT) |
| U-50 | The licence cell now says the default fonts egui bundles carry OFL-1.1 and the Ubuntu font licence as well, and that the workspace turns them on. An existing dependency; nothing new |
| U-51 | Use changed from "data" to "visual guidance": it is a comparison and is not added. Brief §13.4 has two uses, so "data" was dropped from the column's legend |
| Licence classes table | The class 2 line now says which rows are dependencies already and that U-51 is not added |
| Not verified | The VRoid Studio and Ready Player Me lines updated for the rows opened in this pass |

### Corrections to patterns.md

| Where | Change |
|---|---|
| Section 0, Symmetry and Advanced | `ui.rs:444` became `:445` (the `advanced` filter) |
| Section 0, Alt | `viewport/input.rs:56` is the terrain brush and `:103` is plugin tools. The handle rule is at `:372-382` and the part rule at `:398`. Section 3 cites the same lines now |
| Section 0, P12, drag feel | "Shift for fine movement" was too wide. Shift is fine for turning only (`FINE_TURN` 0.25, `gizmo.rs:70`, `:1157`); move and scale have no fine mode. Shift on a handle drag extends the convention |
| Section 0, new rows | View buttons already sit over the top of the viewport (`ui.rs:297-311`). Picking the Face category already frames the face (`ui.rs:242-250`). The body's groups are not body parts. Which parameters are flagged advanced. `frame` moves bones. No shipped manifest declares `constraints` |
| P13, section 5 | The `follow` constraint and its pin are schema only (`manifest.rs:247-254`, checked at `:539`). No shipped manifest declares one and the workspace has no pin. The text said the manifest "already expects" it |
| P20 | Height in centimetres is not `skeleton.height` times the `height` parameter: `frame` moves the head bone. Read it from the resolved skeleton |
| Section 2 | "Features are the manifest's groups" holds for the face only |
| Section 3, drag feel | A line now says the tuning numbers are proposals with no source |
| Section 4, body table | `shoulder_width`, `waist_width`, `hip_width` and `head_size` are now marked as flagged advanced, as `leg_length` already was. Three of the four Quick body handles drive advanced parameters |
| Section 4, option C | "As in The Sims 4" removed: no row in the index supports it, and U-04 says The Sims 4 resizes body parts by dragging on them |
| Section 5, randomize | `max_sum` is enforced where declared; no shipped manifest declares one |
| Section 5, presets | Hover preview is our own design, not something U-22 shows |
| Section 6 | "Does not bake again" became "should find its model already made": expected from the design, not measured |
| Section 8 | The claims summary and the maintenance fee added. The advice is the same and is stronger for it: do not let the zoom level choose what is editable |
| Section 9 | Row 4 widened to the body flags. Rows 15 to 17 added: body features are not groups; `follow` and the pin are schema only; height in centimetres |

### Still not confirmed

- U-05, U-06, U-07, U-18, U-20, U-21, U-27, U-30, U-31, U-34 and U-43 are search summaries. They were not opened in either pass.
- Rows not in the sample (U-03, U-10, U-15, U-23, U-35, U-37 to U-39, U-44, U-46) were opened by the first pass only.
- The patent's claims need a qualified read. Nothing here is legal advice.
