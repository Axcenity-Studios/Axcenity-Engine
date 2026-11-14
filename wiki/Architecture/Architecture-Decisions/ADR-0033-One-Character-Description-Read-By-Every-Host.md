---
title: "ADR-0033: One versioned character description, driven by a parameter manifest, read by every host"
type: adr
adr: 33
adr_status: Accepted
date: 2026-11-14
supersedes:
superseded_by:
affects: [avatar, anim, assets, script, editor, project, net]
source: [crates/avatar/src/description.rs, crates/avatar/src/manifest.rs, crates/avatar/src/migrate.rs, crates/avatar/src/resolve.rs]
docs: [docs/development/character-studio/architecture.md, plans/axcenity-engine-character-studio.md]
tags: [adr, characters, formats]
---

# ADR-0033: One versioned character description, driven by a parameter manifest, read by every host

- **Status:** Accepted (Paul, 2026-11-14: plan D2, D3, section 0 decision 6 and addition A3)
- **Date:** 2026-11-14
- **Affected systems:** Character Data (`avatar`, new), `anim`, `assets`, `script` (`HumanoidDescription`, CO3), Character Studio, networking

## Context and problem

The audit (`docs/development/character-studio/audit.md` §3.1, §3.4) found that a customized character exists only in the editor: `Look` (format 1 `*.character.json`) lives in the `editor` crate, which the player must never depend on, so nothing made in Character Studio reaches a game. PR #400 added a second, unrelated representation (body attributes). Sliders, slots and categories are Rust enums and string matches, so every new slider or slot is a code change in two crates. Paul decided (section 0, decision 6) that games customize characters at runtime, with three layers: Character Studio (authoring), Character Runtime (applying) and Character Data (the shared format, no editor state).

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Attributes on the body only | Exists (#400) | No structure for a wardrobe, materials or versioning |
| Keep the description in the editor | Exists | Games can't use characters |
| One versioned description in a crate every host links, values only; a parameter manifest per base model (chosen) | Player, server and editor read one file; new sliders and slots are data; small to replicate | A new crate and formats to keep stable |

## Decision

A new crate, `avatar` (Character Data, depending on `assets` only), owns one versioned description (`*.character.json` format 2: base model ID, parameter values, worn items with variants and material values, base material values; unknown fields kept) and the **base model manifest** that says what each parameter drives (shapes, bones, attributes, material values; corner products; constraints), plus item manifests and the catalogue. Format 1 migrates on load. #400's body attributes become one output of resolving a description. The runtime (`HumanoidDescription`, `Humanoid:ApplyDescription`) resolves descriptions with the same code the studio previews with.

## Reasons

- One description makes the studio's character the game's character, and the server and network carry only values.
- A manifest per base model turns hundreds of sliders, slots and categories into data, so a new body family or slider needs no code.
- Values only, with no editor state, keeps the format stable and authoring decoupled from the renderer and the game runtime.

## Consequences

- Format rules: versions refuse newer files and migrate older ones; optional fields are added without a version bump; checks return every problem (`architecture.md` §4).
- The workspace keeps writing format 1 until CO12; a test proves a migrated format 1 file resolves to the shapes the workspace shows.
- `anim` and `assets` gain the runtime pieces in CO3 (bone changes after the pose, followers taking shape weights, derived models for the identity bake).
- Revisit if descriptions need per-instance overrides that don't fit values (for example sculpted deltas), which would need a referenced data file.

## Implementation references

- `crates/avatar/src/description.rs`, `migrate.rs`, `manifest.rs`, `resolve.rs`
- Tests: `crates/avatar/src/tests/`, `crates/avatar/tests/shipped_library.rs`, `project` `the_character_data_manifests_describe_the_default_humanoid`, `editor` `a_saved_look_migrates_to_the_same_parts_and_shapes`
- Plan: CO1 (contracts), CO3 (applier), CO12 (studio), CO13 (network)
