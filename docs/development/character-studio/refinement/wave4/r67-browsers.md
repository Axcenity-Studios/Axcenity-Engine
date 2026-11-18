# Hand-back: RA1, the Hair and Clothing browsers (`r67-browsers`)

Task: RA1's milestones **CO25** (the hair browser, stage R6) and **CO26** (the wardrobe browser, stage R7), built against today's items (the stylized hair and clothes, the realistic underwear, the CO9 test tee, all drafts) and the item contract (architecture §8). Branched from `main` at `e1658afb` (waves 1 to 3, PR #427).

**In one paragraph.** The Hair and Clothing pages are now browsers. Each page has a search, its slots as categories, and filters: kind (a tag), Recolourable, Fits this body and Favourites. "Show drafts" is off by default, and when on, a draft tile says "Draft". A "Recently used" row sits above the tiles, and each tile is a picture of the character being edited wearing that item. Hovering a tile, or clicking one, shows the character wearing it in the viewport, with no undo step and no change to the description. "Wear" (or a double click) is one undo step. Esc, ✕, leaving the page or any edit puts the character back. Under the filters, the worn item's own controls show only what the item declares: variants, a colour per colour role, and for a hairstyle natural colours for its colour, highlights and roots (the last two only where the style has slots with those roles), plus its own parameters. Favourites and recently used are editor state, kept in `favourites.json` beside the preset favourites. **What it doesn't do yet:** every shipped item is a draft and none declares colour roles. So with drafts hidden, the pages show only what is worn, and no shipped item shows colour controls. The behaviour is proven on test items. Real content is RA4's.

## Files

| File | What |
|---|---|
| `crates/editor/src/character_workspace/browser.rs` (new) | The logic and the editor state: `Shelf`, `Filters`, `Pick`, `Browse`, `Browser` (filters per page, Show drafts, hovered, picked, the preview), `list` → `Listing`/`Entry`, `offered`, `recolourable`, `misfit`, `matches`, `dressed` (what wearing takes off and puts on), `pictured` (a clothing tile's picture leaves off the layers above it), `remember` (recently used), `Declared` (what an item declares), `role_label`, `HAIR_COLOURS`; the workspace methods `browse`, `end_preview`, `toggle_item_favourite`, `set_item_variant`, `set_item_material`, `set_item_parameter`, `browser_tiles` |
| `crates/editor/src/character_workspace/panel/browser.rs` (new) | The page: search, categories, filters, Show drafts, the "Trying on … / Wear / ✕" bar, the worn items' controls, Recently used, tiles by slot (each slot's empty choice first), the tile (picture, label, star, Draft, Worn and "Trying on" badges, tooltip) |
| `crates/editor/src/character_workspace/panel/worn.rs` (new) | The worn item's controls: variant chips, natural hair colours per role, colour buttons per role, the item's parameters through the shared slider row, limitations as notes |
| `crates/editor/src/character_workspace/browser_tests.rs` (new) | 16 tests (below) |
| `crates/editor/src/character_workspace/shell_tests.rs` | The Clothing page's expected line follows the new wording |
| `crates/editor/src/character_workspace/intent.rs` | New intents (below) and their place in `edits()` |
| `crates/editor/src/character_workspace/shell.rs` | Applies them; an edit ends a preview first; Esc ends a preview (after a drag's Esc); leaving a page ends it; the page's tile pictures are asked for each frame |
| `crates/editor/src/character_workspace.rs` | Fields `browser`, `item_favourites`, `recent_items`, `item_pictures`; `wear` goes through `browser::dressed`, ends a preview and remembers the item; `start_over` ends a preview |
| `crates/editor/src/character_workspace/compare.rs` | `shown()` returns the tile tried on (a held version still wins) |
| `crates/editor/src/character_workspace/presets.rs` | `favourites.json` gains `items` and `recent_items` (both optional, so an older editor reads the file and keeps `presets`); `read_item_lists`, `write_item_lists`; `write_favourites` keeps the item lists |
| `crates/editor/src/character_workspace/thumbnails.rs` | `item_picture` (`thumbnails/items/<id>.<key>.png`), `Subject::Hair` and `hair_orbit` (head and shoulders, three-quarter), `load_item_pictures`, `item_texture` |
| `crates/editor/src/character_workspace/panel.rs` | Hair and Clothing go to the browser (Accessories keeps the chips, R8); both have the level switch; their one-line help says a tile can be pointed at |
| `crates/editor/src/character_workspace/stage.rs` | `HEIGHT` visible to the module (the hair framing uses it) |
| `crates/editor/CLAUDE.md` | `browser.rs` named (9 lines) |
| `docs/editor/character-studio.md` | The Hair and Clothing pages rewritten; files; `favourites.json`; tests |
| `docs/development/character-studio/architecture.md` | §8: how the browsers read an item's declarations (roles, highlights and roots), and that drafts are hidden in the browsers; §15c: the new intents and the editor state |
| `wiki/Editor/Specialized-Editors.md`, `wiki/Roadmap/Known-Limitations.md` | The browsers; their limits (the old "not yet: a wardrobe browser" removed) |
| `docs/development/character-studio/refinement/wave4/r67-browsers-red-run.txt`, `r67-browsers/*.jpg` | The red runs; ten sheets (1.05 MB) |

No dependency added. No generated file touched or regenerated (no manifest, catalogue, model or preset changed). Not touched: `avatar`, `render`, content, the face or body build, the Skin page.

## Interfaces

```rust
// intent.rs (the task allows browser intents; documented in architecture.md §15c)
Intent::ToggleItemFavourite(String)                 // editor state
Intent::Browse(browser::Browse)                     // editor state
Intent::EndPreview                                  // editor state
Intent::SetItemVariant { item: String, variant: Option<String> }                  // edit, one step
Intent::SetItemMaterial { item: String, slots: Vec<String>,
                          values: Vec<(String, Option<MaterialValue>)>, gesture } // edit
Intent::SetItemParameter { item: String, id: String, value: f32, gesture }        // edit
// Intent::Wear is unchanged; it now goes through browser::dressed.

// browser.rs
pub enum Shelf { Hair, Clothing }
pub struct Filters { search, slot: Option<String>, kind: Option<String>, recolourable, fits /* default true */, favourites }
pub struct Pick { slot: String, item: Option<String> }
pub enum Browse { Filters(Shelf, Filters), ShowDrafts(bool), Hover(Option<Pick>), Select(Option<Pick>) }
pub fn list<'a>(&'a Library, &Description, Shelf, &Filters, show_drafts: bool,
                favourites: &BTreeSet<String>, recent: &[String]) -> Listing<'a>
pub fn dressed(&Description, &Library, slot: &str, item: Option<&str>) -> Description
pub fn pictured(Description, &Library, slot: &str, item: Option<&str>) -> Description
pub struct Declared { roles, natural, variants, parameters }   // Declared::of(&ItemManifest)
```

**How an item declares highlights and roots** (no schema change; architecture §8): on a groom, the material slot with role `primary` is the hair's colour, `secondary` its highlights, `accent` its roots. Each such Hair slot that offers `melanin` gets the natural colour swatches, which set `melanin` and `redness` together. Each slot with a role that offers `tint` gets a colour button: for a groom only at Advanced, since the swatches come first (ux patterns P4); for a garment at every level. A slot with no role is no control. **RA4 should give each new style's material slots roles** (and `label`s). Today's stylized `HairSecondary` is the highlights slot by its making (audit-character-data CD-14.1-04), but it has no role, so it shows nothing.

**the project owner's drag rule (2026-11-18).** The only numeric control added is an item parameter's slider. It is the shared slider row (`panel/row.rs`, `row::slider`), the same one Body and Face use, so ux-drag's helper reaches it when it lands in that row. A `TODO(ux-drag)` in `panel/worn.rs` marks it. No ux-drag helper is on `main` yet. The row types values at Advanced, as today.

## Tests

`browser_tests.rs`, written first and seen failing (the red run: browser.rs's functions returning at once, 13 of 14 failing; the 14th, the snapshot test, returns at once without `AXCENITY_SNAPSHOTS`):

- `only_items_for_this_base_in_a_slot_the_page_takes_are_offered`: a stylized item isn't offered on the realistic body. Nor is one that also fills the underwear slot a body type's choice fills. The underwear is said, not offered. The slots are the categories, in order.
- `drafts_are_hidden_until_show_drafts_and_marked_when_shown`, plus a worn draft always listed.
- `search_matches_every_word_against_name_tags_and_about`.
- `filters_narrow_by_slot_kind_colour_fit_and_favourites`: fits uses a `limitations` range on `fat`, and gives the misfit note.
- `recently_used_lists_the_items_last_worn_newest_first`: once each, at most 12, no hidden drafts, only on the whole list.
- `wearing_takes_off_what_conflicts_and_what_its_slots_held`: `excludes`, a tile's "takes off" list, and the underwear kept.
- `every_style_declares_only_what_it_supports` (plan R6's "Done when"): a one-colour bob has its colour and no highlights, roots, variants or parameters; a streaked style has its highlights, roots and length; a garment's roles; a slot without a role is no control.
- `the_worn_style_shows_only_the_controls_it_declares`: drawn through the whole studio. The plain style shows "Hair colour" and no Highlights, Roots, Length or Variant. The streaked one shows them, and draws no widget twice.
- `a_tile_is_previewed_without_committing_until_wear` (plan R7's "preview without committing leaves undo untouched"): a pick and a hover show the item on the preview's parts. The description, undo and the saved flag stay as they were. Esc (a key event through the shell) restores the character, and so does leaving the page. Wear is exactly one undo step. Any edit ends a preview.
- `drafts_show_only_with_show_drafts_and_say_draft`: the shipped stylized hair. Turning drafts off drops a draft being tried on.
- `the_worn_items_controls_are_one_undo_step_each`: the variant, an undeclared variant refused, a natural colour on two slots as one step, a dragged length as one step held in range, a variant giving back what it sets, and the result validates.
- `favourites_and_recently_used_are_kept_beside_the_preset_favourites`: neither touches the description or undo, both survive reopening, and preset favourites are kept.
- `the_realistic_pages_say_what_they_have`.
- `a_clothing_tile_shows_its_item_over_what_is_under_it_only`: added during the visual review (below), and seen failing first.
- `the_browsers_are_drawn` (with `AXCENITY_SNAPSHOTS`) and `the_realistic_clothing_page_is_drawn_with_the_test_tee` (ignored: the CO9 import takes about 70 s in a debug build, past nextest's 2 minutes once the painter runs; run it with `cargo test -- --ignored`).

Runs (in this worktree):

- `cargo nextest run -p axcenity-editor --no-fail-fast -E 'test(/browser_tests::/)'`: **15 tests run: 15 passed** (the ignored one skipped).
- `AXCENITY_SNAPSHOTS=<dir> cargo test -p axcenity-editor --bin axcenity-editor -- --include-ignored browser_tests:: --test-threads 1`: **16 passed; 0 failed** (207.8 s).
- `cargo nextest run -p axcenity-editor --no-fail-fast` (the whole editor crate): **1414 tests run: 1412 passed, 2 failed**. `shell_tests::every_page_is_drawn_with_its_controls` expected the old Clothing line ("Underwear comes with the body type"), so its expected text is now "which comes with the body type". `app::collab_repo_tests::closing_the_editor_uploads_what_was_saved` timed out under the shared load (67.7 s) and isn't mine. Both rerun alone: **2 passed**. No ABORT lines.
- `cargo clippy -p axcenity-editor --all-targets -- -D warnings`: clean. `cargo fmt --all`: done. `python3 -I scripts/check-wiki.py`: 114 pages, **0 errors**, 58 warnings (all "may be stale", none on pages this changes).

## Measurements

- **Tile pictures** (thumbnails painter, debug build, its own GPU device, measured in the snapshot test): the painter's first picture took 1.0 to 3.0 s, since it opens a workspace and loads the models. After that a tile took 20 to 600 ms: about 20 to 155 ms for most, up to 600 ms for a style whose model loads for the first time, and 894 ms for the imported test tee's first use. A hair tile's character doesn't depend on which style is worn, so trying or wearing another style redraws no hair tile. A body or face change redraws the page's tiles, but not while a drag goes on.
- **Fits this body**: on by default. It hides only what an item's `limitations` say, so no shipped item is hidden by it today (none has limitations).
- **Drafts**: the stylized default character wears Swept, a draft. So with drafts hidden, its Hair page lists Swept and the 2 test items and says "5 hidden" (all 6 shipped styles are drafts). On the realistic body, Clothing says "1 hidden" (the imported test tee).

## Visual evidence and what I saw

Sheets in `r67-browsers/` (1440×900 drawn for real; the status line's local path painted over):

- `hair-drafts-off.jpg`: the drafts are hidden ("Show drafts (5 hidden)"). Bald, the two test styles and the worn Swept (a draft, so listed, with Draft and Worn badges) are left. No section for Swept: it declares nothing.
- `hair-drafts-on.jpg`, `hair-tiles-2x.jpg`: nine tiles, head and shoulders at three-quarter. **Bald, Bob, Buzz, Crop, Long, Ponytail and Swept read apart at tile size** (the ponytail's tail shows behind the neck). Plain crop and Streaked bob look like Crop and Bob because they are test items on the same meshes. Long hangs over the face, which is the stylized content, and the preview shows it the same way.
- `hair-preview.jpg`: Streaked bob is worn and Long is tried on. The viewport shows Long. The bar says "Trying on Long" with Wear and ✕. The worn style's controls (Variant: As made, Sun-lightened; Hair colour and Highlights swatches, Brown lit; Length 0.50) sit above the tiles. The first row of tiles is still in view at 900 points.
- `hair-search.jpg`: "short" lists Buzz ("very-short"), Crop, Plain crop and Swept. No Bald tile while searching.
- `clothing.jpg`, `underwear-tiles-2x.jpg`: the stylized Clothing page with every slot (the stylized body has no choices, so its underwear is offered). The underwear tiles show the underwear: None, Sports top and Briefs read apart.
- `clothing-filtered.jpg`: Bottoms only: None (bare legs, the tee kept, the jacket left off) and Cargo pants (worn, Draft).
- `realistic-clothing-drafts-off.jpg`: "Underwear: Boxer briefs, which comes with the body type.", "Show drafts (1 hidden)" and "Everything here is still a draft. Turn on Show drafts to see them."
- `realistic-clothing-preview.jpg`: the CO9 test tee (imported as a draft) tried on. The viewport shows the realistic man in the tee over the boxer briefs. The None tile is lit as worn, and the tee's tile has the Draft and "Trying on" badges. The tee is boxy with a mock neck, as wave 3 said. That is the garment, not the browser.

**Found in the review and fixed:**

1. **The underwear tiles all looked the same.** On a dressed character, None, Sports top and Briefs showed the same jacket and trousers. A clothing tile's picture now leaves off the garments on layers over the tile's (`pictured`; test added and seen failing first). The viewport's preview still shows the whole outfit.
2. **The worn item's controls fell below the fold** (Length out of reach at 900 points). They now sit above the tiles. A worn item that declares nothing has no section (it said "nothing to adjust" four times on the stylized clothing page). The hair tints moved to Advanced.
3. **Hair tiles framed too wide** (the head in the top third): `hair_orbit` moved from 4.6 to 3.6 units from the head.
4. **Mixed-up pictures in the snapshots.** This happened only in the test: each snapshot has its own egui context, and the workspace kept texture handles from the one before. The test now starts each snapshot with fresh caches. The editor has one context.
5. Two tiles of one item (Recently used and its slot) shared a widget ID. Each grid now has its own ID scope, and a test checks that no widget ID is used twice.

## Open issues

- **No shipped item can show the browsers at their best.** All shipped items are drafts (no `quality`), and none declares colour roles, variants or parameters. RA4's first hairstyle and garments should set `quality` only once they pass, give material slots `role` and `label`, and add `tags` (the Kind filter lists them) and `about`. The stylized items need roles from their owner, `humanoid/manifests.py`, if their colours should be pickable.
- **Natural hair colours are relative.** `melanin` and `redness` change the colour a style is made in. The swatches assume a medium brown, and a style made blonde would read lighter than its swatch. The renderer draws hair `melanin`/`redness` on the skin's curve for now. Dye, tip lightening and highlight strength aren't drawn until RA7's Hair model, which is why no "dye" or "lightened ends" controls were added.
- **Roots as a role.** Roots as a separate material slot means the cards need a root material. If RA4 or RA7 would rather have a root-to-tip value, the browser needs a different declaration (a contract change for the lead).
- **"Fits this body" knows only what `limitations` say.** Nothing is measured per body. The CO9 fit gate could write limitations at import.
- **Presets and Randomize still use drafts** (architecture §8 says so). That is `avatar`'s, not the studio's.
- Accessories is still chips (R8). No virtual list: fine for tens of items. No hover preview of a variant.
- The "Recently used" and favourites lists keep IDs of items that are no longer in the library. They're filtered out when listed, but not pruned from the file.

## For the lead at merge

- No generated files to rebuild for this branch.
- `intent.rs`, a lead file, gains six variants, as the task allowed. `edits()` lists the three item edits.
- Conflicts are likely with ux-drag in `panel/row.rs` (not touched here) and with anyone else's `shell.rs`/`intent.rs` additions. Mine are additive match arms and variants.
- `progress.md` and plan 11.5's status line are left for you.
