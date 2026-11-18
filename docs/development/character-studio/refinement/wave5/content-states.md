# Wave 5 hand-back: content states (Paul's decision 12)

Branch `content-states`, from `refinement-wave5`. Paul's decision 12 of 2026-11-18 ([decisions-2026-11-18.md](../decisions-2026-11-18.md)): three content states, Production, Legacy and Draft, everywhere items are listed, drawn or randomized. The decision record is [ADR-0043](../../../../../wiki/Architecture/Architecture-Decisions/ADR-0043-Three-Content-States-Production-Legacy-Draft-In-The-Item-Manifest.md) (Accepted: it records Paul's decision, which the lead applies).

## What was built

1. **The contract** (`avatar`): the item manifest's `quality` takes `production`, `legacy` or `draft` (`Quality::Legacy` added). `ItemManifest::state()` reads it, with `is_production`, `is_legacy`, `is_draft`:
   - **Unset is Legacy** from now on: a manifest with no state was written before the states (the stylized build's, an older project's copy). It still loads and draws, is hidden by default and never drawn at random.
   - **An unknown state reads as Draft** (`Quality::Other`, `#[serde(other)]`), so a later engine's manifest still loads instead of failing the whole file.
   - **A description never carries the state.** Format 2 descriptions and presets are untouched.
2. **The shipped-item check**, in its own file so the licensed-asset intake's provenance check merges beside it: `avatar::check_declared_state(&ItemManifest)` (`content_state.rs`). Every shipped item must say one of the three. It is not part of `validate`, because a project's older copies may leave the state out. Test: `tests/content_states.rs`, `every_shipped_item_declares_its_state`, over every `*.item.json` under `assets/characters/`, catalogued or not.
3. **Classification** (by the generators, not by hand):
   - the stylized family's 12 items: **Legacy** (Paul's words);
   - the realistic underwear (boxer briefs, briefs, sports bra): **Legacy**. It was made before the standard and has never been through the review, so it can't be shown to pass every check of asset-quality.md, and nothing becomes Production in this task anyway;
   - the four wave-4 items (short crop, crew tee, joggers, court sneakers): **Draft** (decision 13; their generators already wrote `draft`);
   - **Production: none.**
4. **Randomize** (`avatar::randomize`, `draw_items`) draws **Production only**:
   - `avatar::production_slots(library, base)` lists the slots with a Production item for the base. Only those slots are drawn again. Every other slot keeps what was worn, item, variant and values, so with no Production content (today) every item stays.
   - A slot that wore underwear (layer 0, the modesty layer) never draws "nothing", and gets its old underwear back if no Production underwear fits.
   - What a parameter's choice wears (the realistic body type's underwear, Legacy) is the choice's, so the realistic body is always dressed in its type's underwear.
5. **Character Studio** (`browser.rs`, `panel/browser.rs`, `panel/worn.rs`, `toolbar.rs`, `random.rs`):
   - **By default, the Hair and Clothing pages list Production items only.**
   - **"Legacy items"**: a chip at the end of the page's filter row, off by default. It is drawn only where the page has legacy content to show. With it on, legacy tiles appear marked **Legacy**. The tooltip gives the hidden count.
   - **Drafts are for content review only.** "Show drafts" left the filter bar. It is now the toolbar's new **View** menu, **"Show drafts for content review"**, off by default. With it on, drafts appear marked **Draft**, and the page says "Content review: drafts are shown, marked "Draft" (View menu)." in the warning colour.
   - Both switches are editor state (`Browser::show_legacy`, `show_drafts`, `Browser::shown()`). Neither is saved in a character.
   - Turning either off drops a hidden tile being tried on.
   - **A worn item is always listed, marked and worn, whatever its state, and its controls stay.** The controls' title says "(Legacy)" or "(Draft)".
   - **Empty pages say what is there** (`browser::empty_message`):
     - "No production hairstyles yet" / "No production clothes yet";
     - then "Legacy items: N (turn on Legacy items to show them)." and "Drafts: N (content review only)." as they apply;
     - or "Nothing matches. Clear the search or a filter." when the filters hide what is listed.
   - **Randomize** says when a scope has no Production item to draw:
     - Its menu entry adds "Worn items stay as they are: there are no production items to draw yet." (`random::no_items_to_draw`, `NO_ITEMS`).
     - A press that changes nothing makes no undo step. Its notice says "Nothing changed." with the same note.
6. **Old projects and the Game Editor:** loading, `resolve`, the applier and the player ignore the state. Default outfits and presets are unchanged:
   - The stylized default outfit and its presets stay Legacy. This is a compatibility case: the family has no Production item yet, and decision 7 keeps the family supported.
   - The realistic default character and presets keep the Legacy underwear as the modesty layer.
   - The docs say plainly that a default outfit of Legacy items is not Production content.
   - Insert > Character and new projects are untouched: they wear the defaults as before.

## Interfaces

- `avatar::Quality { Draft, Legacy, Production, Other }` (the `Default` derive is gone: unset no longer means draft). `ItemManifest::{state, is_production, is_legacy, is_draft}`.
- `avatar::check_declared_state(&ItemManifest) -> Vec<Problem>` and `avatar::production_slots(&Library, base) -> BTreeSet<String>`.
- Editor (crate-private):
  - `browser::Shown { legacy, drafts }` (the default lists Production only) with `shows(state)`;
  - `browser::list(.., content: Shown, ..)`, which replaces the `show_drafts: bool` parameter;
  - `Entry::state`, which replaces `draft`, and `Listing::hidden_legacy`;
  - `browser::empty_message`, `Browse::ShowLegacy`;
  - `toolbar::view_options` / `ViewOption`;
  - `random::no_items_to_draw`, `random::NO_ITEMS`.

**Compatibility decision** (plan 11.3: an older engine rejects an unknown enum value). I put Legacy in `quality` as a third value rather than in a second field.

- **No released engine reads item manifests**: characters arrived after v0.2.0-alpha.1 (that tag has no `avatar` crate; corrected in Review fixes 1).
- **Development builds from CO9 (2026-11-17) to this change** know only `draft` and `production`. They reject a manifest that says `legacy`. In a project a newer engine wrote, those items drop out with a reported problem, and characters there draw without them.
  - No release carries such a build. Two fields for one fact would have to be reconciled forever.
  - From this change on, an unknown state reads as Draft instead of failing, so a fourth state won't break this engine.
- **Descriptions and project files other than the library's manifests are unchanged.**
- This is recorded in ADR-0043, architecture.md's `quality` row and Known Limitations.

## Files changed

- `crates/avatar/src/item.rs`, `content_state.rs` (new), `randomize.rs`, `lib.rs`, `fixtures.rs` (the fixture libraries are Production), `tests.rs`, `tests/content_states.rs` (new), `tests/randomizing.rs`; `crates/avatar/tests/content_states.rs` (new); `crates/avatar/CLAUDE.md`
- `crates/editor/src/character_workspace/browser.rs`, `panel/browser.rs`, `panel/worn.rs`, `toolbar.rs`, `random.rs`, `browser_tests.rs`, `shell_tests.rs`
- `crates/player/src/content_state_tests.rs` (new), `main.rs` (the module)
- Generators: `assets/characters/humanoid/manifests.py` (`QUALITY = "legacy"`), `assets/characters/realistic/manifests.py` (`UNDERWEAR_QUALITY = "legacy"`). Regenerated with them: `assets/characters/humanoid/items/*.item.json` (12), `assets/characters/realistic/items/realistic-{boxer-briefs,briefs,sports-bra}.item.json`. Each gains one line, `"quality": "legacy"`. `wardrobe.py` and `build_hair.py` already write `draft` and were not changed.
- Docs: `wiki/Architecture/Architecture-Decisions/ADR-0043-…md` (new), `ADR-Index.md`, `wiki/Architecture/System-Inventory.md`, `wiki/Engine-Systems/Character-Data.md`, `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md`, `docs/development/character-studio/architecture.md` (the `quality` row, Randomize's items, §15c's `Browse`), `asset-quality.md` §5, `docs/editor/character-studio.md`
- This hand-back and `content-states-red-run.txt`; pictures in `content-states/`.

## Tests

RED first (`content-states-red-run.txt`), in three parts:

1. **avatar:** 7 of 22 state tests failed against stubs: Randomize drew `axcenity/sports-top` (Legacy) at seed 0, and the stubbed `production_slots` listed every slot.
2. **editor:** 9 of 29 browser and shell tests failed: legacy items listed by default, no empty message, no View option, no Randomize note.
3. **the real check on the manifests as they were:** 15 items "doesn't say its content state".

GREEN:

- **`cargo nextest run -p axcenity-avatar -p axcenity-project -p axcenity-gameplay --no-fail-fast`:** 467 tests run: 467 passed, 1 skipped. Avatar alone: 177 passed, including:
  - the new `tests::content_states::*` (4);
  - `tests::randomizing::{randomize_draws_production_items_only, a_slot_without_production_items_keeps_what_it_wore, randomize_never_strips_the_modesty_layer}`;
  - `tests/content_states.rs` (7).
- **`cargo nextest run -p axcenity-editor --no-fail-fast`:** 1457 tests run: 1457 passed (11 slow), 11 skipped, including `character_workspace::browser_tests::{legacy_and_drafts_are_hidden_by_default_and_marked_when_shown, an_empty_page_says_what_is_hidden, production_is_listed_legacy_on_request_and_drafts_only_for_review, the_view_menu_shows_drafts_for_content_review, randomize_says_when_there_are_no_production_items_to_draw}`.
- **`cargo nextest run -p axcenity-player -E 'test(/content_state_tests|default_humanoid_tests|description/)'`:** 23 passed, the two new `content_state_tests` among them. No ABORT lines.
- **Checks:**
  - `cargo clippy -p axcenity-avatar -p axcenity-editor -p axcenity-player --all-targets -- -D warnings`: clean.
  - `cargo fmt --all`: clean.
  - `python3 -I scripts/check-wiki.py`: 115 pages, 0 errors, 57 warnings (staleness warnings on pages this change doesn't touch).
- **Drawing tests with `AXCENITY_SNAPSHOTS` set:** four of them went past nextest's 120 s limit under the shared machine's load: `the_browsers_are_drawn`, `every_page_is_drawn_with_its_controls`, `handles_are_drawn_for_hover_selection_and_a_drag` and `wave3_pages_are_drawn`. I needed the first two for pictures, and run alone with `cargo test` they passed in 88 s and 90 s. Without snapshots all four pass in the full run above.

What the 1,000-seed tests measure:

- **The shipped library:** 1,000 randomizations per family, from the default and every preset, with the scopes Everything, Hair and Clothing.
  - 0 Legacy or Draft items drawn.
  - 0 items taken off, other than the underwear a body-type change swaps.
  - Every realistic result wore its type's underwear.
  - Every result resolved with 0 problems.
- **With one Production copy added per family:** it was drawn (the test fails otherwise), and still 0 Legacy or Draft items were drawn.
- **The modesty test, on the fixtures:** the underwear slot was filled in 2,000 of 2,000 draws (1,000 seeds, two scopes), and both Production underwear items were drawn.

## Visual evidence

Studio drawing snapshots (`AXCENITY_SNAPSHOTS`, 1440 by 900; the status line names a local folder, so it is cropped off), in `content-states/`:

- **`stylized-hair-normal.jpg`** (the stylized Hair page in normal creation):
  - Four tiles: Bald, the test's two Production styles (Plain crop, Streaked bob) and the worn Swept. Swept carries a grey "Legacy" badge beside the blue "Worn" one.
  - None of the five other legacy styles shows, and no draft.
  - "Legacy items" sits after Favourites, unlit.
  - The toolbar has the new "View" menu between the handles switch and Randomize. Nothing overlaps at 1440 points.
- **`stylized-hair-legacy-and-review.jpg`** (the same page, "Legacy items" lit and content review on):
  - Nine tiles: Bob, Buzz, Crop, Long and Ponytail marked "Legacy", Draft crop marked "Draft" (amber text), and the Production ones unmarked.
  - The amber line "Content review: drafts are shown, marked "Draft" (View menu)." over the "Trying on" bar wraps to two lines in the 330-point panel.
  - The badges sit at the picture's lower left and don't cover the face.
- **`realistic-hair-empty.jpg`** (the realistic default, in boxer briefs, on Hair):
  - "No production hairstyles yet" and "Drafts: 1 (content review only)."
  - No Legacy chip: there is no legacy hair for this body.
  - The "Point at a tile to try it on" bar with a Wear button still shows over the empty page. That is earlier behaviour (the bar keeps its place so tiles don't move), and it reads oddly with nothing to point at. Noted under open issues.
- **`realistic-clothing-empty.jpg`** (Clothing): "Underwear: Boxer briefs, which comes with the body type.", then "No production clothes yet" and "Drafts: 3 (content review only)."

## Open issues

- **No Production item exists**, so in normal character creation:
  - both families' Hair and Clothing pages show only what is worn, or say "No production … yet";
  - Randomize changes only parameters.

  That is the decision's intended effect until the first item passes the review and Paul signs it off.
- **The realistic underwear is Legacy but is still each body type's default and modesty layer** (worn by the `frame` choice, in every realistic preset). A Production underwear set should replace it; then `presets.py` and the base's `defaults` should name it.
- **The "Legacy items" filter is shared by the Hair, Clothing and Accessories pages** (one switch, like content review). It is not per page and not remembered between sessions. Per-viewer memory could be added if Paul wants it.
- **The development-build compatibility gap** described under Interfaces.
- ~~The "Trying on" bar on an empty page.~~ Fixed in Review fixes 1: a page with no tile draws no bar.

## What the lead must do at merge

- Regenerate nothing else: `catalogue.json`, the base manifests, the presets and the benchmarks don't carry the state, and are unchanged (both base manifests came out identical but for line endings, which I did not commit).
- The licensed-asset intake adds provenance checks in avatar. Keep `check_declared_state` beside them: both are separate functions, and `tests/content_states.rs` is its own file.
- Any item the content waves add must say its state: the check fails otherwise. An import is `draft`.
- Promote an item to Production only after the review: change its generator's state, not the manifest by hand.

## Review fixes 1

**Blocking 1: the Accessories page ignored the states.** It now follows the same rule as the browsers.
- `browser::slot_choices(library, description, slot, Shown)` gives a slot page's chips: the items `Shown` lists, plus any item worn there whatever its state, labelled through `browser::marked_label` ("Watch (Legacy)", "Watch (Draft)"). It also counts the hidden legacy items and drafts. `panel/worn.rs` uses the same `marked_label`.
- `panel.rs` `slots_page` lists through it. It draws a "Legacy items" chip (the shared switch) where there is legacy content, and the content-review line when drafts are shown.
- An empty page says what it hides, through `browser::hidden_sentence`, which `empty_message` now shares: for example "No production accessories yet" and "Legacy items: 1 (turn on Legacy items to show them)."
- The free `items_for`, which ignores the state, is now `#[cfg(test)]` only.
- Test `browser_tests::the_accessories_page_follows_the_content_states` uses a realistic workspace whose base gets a "Wrist" accessories slot, because no shipped base has one. It runs with the watch Legacy and then Draft, and checks four things:
  - hidden by default, with an honest empty page;
  - the Legacy filter shows only the legacy watch;
  - content review shows only the draft watch;
  - the watch is always listed and marked while worn.
- Red run: the legacy watch was listed unmarked as "Watch" (red-run part 4).

**Blocking 2: stale docs.** The reviewer is right. `git ls-tree v0.2.0-alpha.1 crates/` lists no `avatar`, and `git grep ItemManifest v0.2.0-alpha.1` finds nothing. Fixed:
- ADR-0043, architecture.md's `quality` row, Known-Limitations and this hand-back now say: "No released engine reads character item manifests (characters arrived after v0.2.0-alpha.1)". The development-build gap stays as stated.
- The old "Show drafts" wording now names the "Legacy items" filter and the View menu's "Show drafts for content review", in:
  - `wiki/Editor/Specialized-Editors.md`;
  - `docs/editor/character-studio.md` (the intents sentence, the Accessories sentence and the test list);
  - `intent.rs`'s module comment.
- `testing.md` names `randomize_draws_production_items_only`.
- Two more stale Known-Limitations sentences from wave 4 were corrected: "the Hair page lists it until Show drafts is merged" and "Character Studio doesn't hide drafts yet".

**Non-blocking, done:**
- New test `tests::randomizing::underwear_that_nothing_can_replace_goes_back_on`. Production boxers exclude the hair, which a Clothing randomize keeps. Over 500 seeds the Legacy briefs go back on and the boxers are never worn. The test passes on the current code. With the put-back branch disabled (`if bare && false`, restored with `git checkout`), it fails at randomizing.rs:480.
- ADR-0043 now says a later engine's unknown value is not kept if this engine rewrites the manifest. Today nothing does: only `fit::import_garment` writes manifests, always new and `draft`.
- docs/editor/character-studio.md tells project authors to set `"quality": "production"` on their own items, because an unset state reads as Legacy.
- The "Trying on" bar is no longer drawn on a page with no tile: no entries, no recently used, no slot filter and nothing picked. `the_realistic_pages_say_what_they_have` now asserts this. It failed first (red-run part 5).

**Non-blocking, left:**
- The stylized snapshot shows test-fixture Production tiles. Its caption above already says so.
- The Legacy badge has lower contrast than the Draft badge. The reviewer judged it acceptable.

**Tests run:**
- `cargo nextest run -p axcenity-avatar -p axcenity-editor --no-fail-fast`: 1636 tests run, 1636 passed (12 slow), 11 skipped, no ABORT.
- After a last text-only fix to two strings in `slots_page`: `cargo nextest run -p axcenity-editor -p axcenity-player -E 'test(/character_workspace::/) | test(/content_state_tests/)'` ran 153 tests, all passed.
- `cargo clippy -p axcenity-avatar -p axcenity-editor -p axcenity-player --all-targets -- -D warnings`: clean.
- `cargo fmt --all`: clean.
- `python -I scripts/check-wiki.py`: 115 pages, 0 errors, 57 warnings (staleness on untouched pages).

**For the lead:** new crate-private editor helpers: `browser::slot_choices`, `SlotChoices`, `marked_label` and `hidden_sentence`. No new snapshot: the Accessories page has no shipped content to draw.
