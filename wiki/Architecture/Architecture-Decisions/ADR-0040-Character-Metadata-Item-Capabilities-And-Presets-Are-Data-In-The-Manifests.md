---
title: "ADR-0040: Character metadata, item capabilities and presets are data in the manifests"
type: adr
adr: 40
adr_status: Proposed
date: 2026-11-17
supersedes:
superseded_by:
affects: [avatar, editor, gameplay, project]
source: [crates/avatar/src/manifest.rs, crates/avatar/src/item.rs, crates/avatar/src/description.rs, crates/avatar/src/preset.rs, crates/avatar/src/randomize.rs, crates/avatar/src/catalogue.rs]
docs: [docs/development/character-studio/architecture.md, docs/development/character-studio/refinement/audit-character-data.md]
tags: [adr, characters, formats]
---

# ADR-0040: Character metadata, item capabilities and presets are data in the manifests

- **Status:** Proposed (the lead's R1 contracts, plan section 11.3; the project owner may overturn it)
- **Date:** 2026-11-17
- **Affected systems:** Character Data (`avatar`: base and item manifests, descriptions, catalogue, presets, Randomize); Character Studio (`editor`, which reads them from R1 step 2); the applier (`gameplay`, item parameter shapes later); `project` (copying packages)

## Context and problem

The refinement brief (2026-11-16) asks for three levels of customization, tooltips, selectable features and handles on the character (§3.3, §3.4, §15), capability-aware items (§14.1), presets of every kind (§4.1, §5.1, §11) and Randomize with locks (§11). Before R1 the manifests had none of it: one `advanced` flag, no descriptions, no notion of a feature, items that declared only their materials and variants, and six presets as constants in the editor that replaced the whole character. The audits proposed up to three shapes for the same facts (`docs/development/character-studio/refinement/audit-character-data.md` §4 and §5; the studio audit §4). Several agents build on them in parallel, so one shape had to be settled first, and an older engine must still read every file (section 0, addition A3).

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Metadata and presets in the editor's code | No format change | Games, scripts and imported content can't carry them; every new body or item needs an editor change; presets stay whole-character |
| A new format version for manifests and descriptions | Room to restructure | Every old file migrates; an older engine refuses every new file |
| A separate metadata file beside each manifest | Manifests untouched | Two files that drift apart; the library reads twice as much |
| **Optional fields in the existing formats; presets as descriptions with a block (chosen)** | No version change; old files read to what they read before and an older engine reads new ones; one parser and one check for presets | Unknown fields of a manifest are dropped when one is written from its typed value (only the catalogue is, today); new enum values stay a breaking change |

## Decision

- **Every new field is optional with a default; no `format_version` changes and no new value in any existing enum.** The new enums (`level`, a material slot's `role`, a preset's `kind`) read a value they don't know as `other`, so they can grow later without breaking older engines.
- **Parameters** gain `about` (the tooltip), `feature`, `level` (`quick`, `detailed`, `advanced`; `advanced` kept and written in step, checked) and `random`. **Groups and slots** gain `about` and `icon`. **Material slots** gain `label`, `values` (what is offered, checked against the shading model) and `role`.
- **Features and handles** are top-level lists of the base manifest. Anchors are a **bone and an offset** (they follow bone changes), not body vertices. A handle only names parameters and how far a drag moves them (`axis`, `travel`): there is never a second geometry state. Anchors are measured on the built model by the package's scripts, not typed by hand.
- **Item capabilities** are declared, and operations derived from them: an item's own `parameters` (shaped like a base parameter, without group and mirror), `also_fills`, `requires`, `excludes`, `limitations`, `adjust` limits for socket items, variant pictures and parameter values. A worn item holds `parameters`, a `transform` inside `adjust`, and keeps unknown fields (`extra`).
- **Presets are format 2 descriptions with a `preset` block** (`*.preset.json`), kinds `body`, `face`, `hair`, `outfit`, `skin`, `character`, listed in the catalogue. **One function, `avatar::apply_preset`, copies only what the kind owns** and never the block, for the studio, scripts and Randomize alike. Favourites and a person's preset list are editor state.
- **Randomize is `avatar::randomize`** (not the editor), deterministic from a seed with an in-crate generator, inside each parameter's `random`, with scopes and locks, items only from what fits and isn't excluded.

## Reasons

- An older editor keeps loading every new file, and a new one reads every old file unchanged; nothing migrates.
- Presets need no parser and are checked by the same `Description::validate`, plus a kind check; the editor, scripts and NPC generation share one rule.
- Data in the manifests reaches every host (player, server, editor) and imported content; a new body or item needs no code change.
- Bone-and-offset anchors survive the frame and proportion bones that move the body; a handle that only names parameters keeps undo, saving and the sliders in one state (brief §3.4).

## Consequences

- An older engine that writes a project's catalogue again drops `presets` (the files stay); until `project::add_character_packages` refreshes a changed package (audit D2), an existing workspace folder keeps its old manifests and sees none of the metadata.
- Item parameter shapes resolve (`ResolvedPart::shapes`) but the applier doesn't bake them into the item's derived model yet; no shipped item has parameters. That lands with the first item that does (R6 hair).
- Anchors are measured on the masculine frame only; whether they sit well on the feminine frame is unchecked (audit risk 3). Handles are R2's content.
- Assigning `level` and `about` to every parameter is judgement, done by hand in the package scripts; the brief puts regional body sliders at the detailed level, which R3 decides (today they stay advanced).
- Revisit if a later need is a new enum value in an existing enum (a new category, fit method or shading model): that is a version bump with a migration.

## Implementation references

- `crates/avatar/src/manifest.rs` (`Parameter::level`, `Feature`, `Handle`, checks), `item.rs` (`ItemParameter`, `Limitation`, `Adjust`), `description.rs` (`WornItem`, `Transform`), `material.rs` (`MaterialRole`), `preset.rs`, `randomize.rs`, `catalogue.rs` (`CataloguePreset`), `library.rs`, `resolve.rs`
- `assets/characters/realistic/manifests.py`, `faceparams.py`, `anchors.py`, `presets.py`; `assets/characters/catalogue.py`
- Contract: `docs/development/character-studio/architecture.md` sections 4 to 8, 15, 15a, 15b; plan section 11.3
