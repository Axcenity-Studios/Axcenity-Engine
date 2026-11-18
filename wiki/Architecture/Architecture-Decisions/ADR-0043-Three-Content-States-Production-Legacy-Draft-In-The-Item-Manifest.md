---
title: "ADR-0043: Character items have three content states, Production, Legacy and Draft, in the item manifest's `quality`; unset is Legacy, and only Production is offered by default or drawn at random"
type: adr
adr: 43
adr_status: Accepted
date: 2026-11-18
supersedes:
superseded_by:
affects: [avatar, assets, editor, project]
source: [crates/avatar/src/item.rs, crates/avatar/src/content_state.rs, crates/avatar/src/randomize.rs, crates/editor/src/character_workspace/browser.rs, crates/editor/src/character_workspace/panel/browser.rs, crates/editor/src/character_workspace/toolbar.rs, crates/editor/src/character_workspace/random.rs, assets/characters/humanoid/manifests.py, assets/characters/realistic/manifests.py]
docs: [docs/development/character-studio/architecture.md, docs/development/character-studio/asset-quality.md, docs/editor/character-studio.md, docs/development/character-studio/refinement/decisions-2026-11-18.md]
tags: [adr, characters, assets, editor]
---

# ADR-0043: Character items have three content states, Production, Legacy and Draft, in the item manifest's `quality`; unset is Legacy, and only Production is offered by default or drawn at random

- **Status:** Accepted. It records Paul's decision 12 of 2026-11-18 (`docs/development/character-studio/refinement/decisions-2026-11-18.md`; plan section 0, decision 12), which the lead applies; the wave-5 content-states track built it in the same change.
- **Date:** 2026-11-18
- **Affected systems:** Character Data (`avatar`: the item manifest's `quality`, `ItemManifest::state`, `check_declared_state`, `production_slots`, `randomize`), the shipped library's item manifests and their generators, Character Studio (the Hair and Clothing browsers, the View menu, Randomize's menu and notice), projects (their copy of the library).

## Context and problem

Wave 4 gave the item manifest an optional `quality`, `draft` or `production`, and the lead read unset as "listed and drawn as before" so the stylized family's items, made before the asset quality standard, stayed in Character Studio and in Randomize. Paul's decision 12 replaces that two-way split with three states:

- **Production:** approved assets available by default.
- **Legacy:** older assets retained for compatibility and optional use.
- **Draft:** incomplete assets that cannot appear in normal character creation.

And for the character creator: Randomize selects Production only; Legacy is hidden by default and shown through an optional content filter; existing projects that use Legacy items keep loading. Decision 13 keeps the four wave-4 items (short crop, crew tee, joggers, court sneakers) Draft.

The constraints:

- **Descriptions and project files stay readable both ways** (plan 11.3): only optional fields; an older engine rejects a file with an unknown enum value. A description never carries the state: the state is the item's, not the character's.
- **Item manifests are copied into projects** (`project::add_character_library`, `add_character_packages`), so a project carries the manifests of the engine that wrote it, and an older project's copies say no state at all.
- **A character must never be undressed by Randomize**, and the realistic body type's underwear (the modesty layer) is worn by the `frame` parameter's choices.

## Considered alternatives

**How a manifest says Legacy:**

| Option | For | Against |
|---|---|---|
| **A third value in `quality`: `production`, `legacy`, `draft` (chosen)** | One field, one meaning; the generators and the check read one value | An engine that knows only `draft` and `production` rejects a manifest that says `legacy` (see below) |
| A separate optional field (`legacy: true`, or `state`) beside `quality` | No engine rejects it | Two fields for one fact, which can disagree (`quality: production` and `legacy: true`); every reader must combine them, forever |
| Keep two values and read unset as Legacy, writing nothing for Legacy | No new value | "Every shipped item declares its state" can't be checked: unset would mean both "an older file" and "legacy on purpose" |

**What unset means from now on:**

| Option | For | Against |
|---|---|---|
| Draft (wave 4's first rule, `Default`) | Strict | An older project's stylized items, which say nothing, would vanish from normal character creation and never be shown with the Legacy filter; Paul said they are not drafts |
| Production (wave 4's reading) | Nothing changes for older files | Randomize would draw content that never passed the review: against decision 12 |
| **Legacy (chosen)** | Exactly what an unset manifest is: written before the states, by the stylized build or an older engine; still loads, draws and can be picked with the filter | A new item written by hand without a state is Legacy until it says otherwise; the shipped library's check catches that |

## Decision

1. **The item manifest's `quality`** (optional, `avatar::Quality`) takes `production`, `legacy` or `draft`. `ItemManifest::state()` reads it: **unset is Legacy**, and a value this engine doesn't know (a later engine's) is read as **Draft** through `Quality::Other` (`#[serde(other)]`), so the manifest still loads and the item still draws but is never offered in normal creation or drawn at random. An import (`fit::import_garment`) is always `draft`.
2. **Every item the engine ships says its state** (`avatar::check_declared_state`, kept apart from `validate` because a project's older copies may leave it out; test `tests/content_states.rs`, `every_shipped_item_declares_its_state`). The generators write it: `humanoid/manifests.py` (`legacy`), `realistic/manifests.py` (the underwear, `legacy`), `clothing/wardrobe.py` and `hair/build_hair.py` (`draft` until an item passes).
3. **Classification at this decision:** the stylized family's twelve items are Legacy (Paul's words); the realistic underwear (briefs, boxer briefs, sports bra) is Legacy: made before the standard, it never passed it, and nothing becomes Production without the review and, for the first of a kind, Paul's sign-off; the four wave-4 items stay Draft (decision 13). No item is Production yet.
4. **Randomize** (`avatar::randomize`) draws **Production items only**. A slot with no Production item for the base (`avatar::production_slots`) keeps what the character wore there, item, variant and values; a slot that wore underwear (layer 0) never draws "nothing" and gets its underwear back if nothing fits; what a parameter's choice wears (the realistic body type's underwear) is the choice's, whatever its state. So a randomized character is never stripped.
5. **Character Studio** lists Production by default on the Hair and Clothing pages. **Legacy** shows through the page's "Legacy items" filter (off by default, shown only when there is legacy content), each tile marked "Legacy". **Drafts** are never in normal character creation: they show only for **content review**, the toolbar's View menu "Show drafts for content review" (off by default), marked "Draft", and the page then says it is reviewing. Both switches are editor state, never in a character. An item the character wears is always shown as worn and its controls stay, whatever its state. An empty page says what is there ("No production hairstyles yet", the count of hidden legacy items and drafts). Randomize's menu and notice say when a scope has no Production item to draw, and such a press makes no undo step.
6. **Loading and drawing never look at the state.** `resolve`, the applier and the player draw a Legacy or Draft item a description wears exactly as before; default outfits and presets are unchanged. A default outfit or preset made of Legacy items (the stylized family's, the realistic body type's underwear) is a **compatibility case**, not Production content: no shipped default or preset wears a Draft (test `default_outfits_keep_their_legacy_items_and_wear_no_draft`).

## Consequences

- **Older engines.** Released engines (up to v0.2.0-alpha.1) predate `quality` and ignore it: they read every new manifest and list and draw every item, as before. Development builds from CO9 (2026-11-17) to this change know `draft` and `production` only, so they reject a manifest that says `legacy`: a project whose library a newer engine wrote loses those items there (`Library::read` reports them). No release carries such a build, so this was accepted rather than adding a second field. From this change on, an unknown state reads as a draft instead of failing (decision 1), so a fourth state would not break this engine.
- **Older projects** keep loading: their copies of the stylized items say nothing and read as Legacy; a character wearing them resolves and draws (test `an_older_projects_library_without_states_still_loads`; the player test `content_state_tests`). Character Studio's workspace copies the newer manifests in (`add_character_packages` replaces changed files), so its library says the states.
- **The stylized family has no Production item yet**, so its Hair and Clothing pages show only what is worn until "Legacy items" is turned on, and Randomize on it changes only parameters. Decision 7 keeps the family fully supported; Legacy is a state, not a family.
- **The realistic body's Hair and Clothing pages are empty in normal creation** until a first item passes the review (they say so). Its default character and presets keep the Legacy underwear as the modesty layer.
- A preset or a default outfit that wears a Draft is a mistake the shipped-library tests catch; nothing at runtime refuses it.
- Revisit when a fourth state is needed, or when a Production underwear set replaces the Legacy one (then the realistic defaults and presets should wear it).

## Implementation references

- `crates/avatar/src/item.rs` (`Quality`, `ItemManifest::state`, `is_production`, `is_legacy`, `is_draft`), `content_state.rs` (`check_declared_state`, `production_slots`), `randomize.rs` (`draw_items`); tests `src/tests/content_states.rs`, `src/tests/randomizing.rs`, `tests/content_states.rs`
- `assets/characters/humanoid/manifests.py` (`QUALITY`), `assets/characters/realistic/manifests.py` (`UNDERWEAR_QUALITY`), `clothing/wardrobe.py`, `hair/build_hair.py`
- `crates/editor/src/character_workspace/browser.rs` (`Shown`, `list`, `empty_message`, `Browse::ShowLegacy`, `Browse::ShowDrafts`), `panel/browser.rs` (the filter, badges, the review line), `panel/worn.rs`, `toolbar.rs` (`view_options`, the View menu), `random.rs` (`no_items_to_draw`, `NO_ITEMS`); tests `browser_tests.rs`, `shell_tests.rs`
- `crates/player/src/content_state_tests.rs` (a saved character wearing Legacy and Draft items draws on both families; an older project's library)
