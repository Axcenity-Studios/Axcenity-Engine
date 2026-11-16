---
title: "ADR-0034: Character content lives in a library on disk with manifests and a catalogue, not in the binaries"
type: adr
adr: 34
adr_status: Accepted
date: 2026-11-14
supersedes:
superseded_by:
affects: [avatar, project, editor, assets]
source: [crates/avatar/src/catalogue.rs, crates/avatar/src/library.rs, crates/avatar/src/item.rs, assets/characters/catalogue.py]
docs: [docs/development/character-studio/architecture.md]
tags: [adr, characters, assets]
---

# ADR-0034: Character content lives in a library on disk with manifests and a catalogue, not in the binaries

- **Status:** Accepted (Paul, 2026-11-14: plan D7 and section 0 decision 2)
- **Date:** 2026-11-14
- **Affected systems:** Character Data (`avatar`), `project` (embedding), the editor's global assets, Character Studio's wardrobe

## Context and problem

`project` embeds the humanoid with `include_bytes!`: every binary carries `HumanoidParts.glb` (every garment and hair style, 6.8 MB) and the rest, about 11 MB (audit §3.6). A wardrobe of 40+ items with 2K textures would add hundreds of MB to the editor, player and server, and a changed garment means rebuilding the engine. Items carry none of the metadata a wardrobe needs (tags, thumbnails, variants, layer, regions hidden, compatible bodies). Paul asked for two content tiers, built-in (generated) and premium or imported (artist-made), with the import pipeline designed from the start.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep embedding | Nothing to install | Every item grows every binary; no metadata |
| One big parts file with a JSON sidecar | Few files | Can't add, replace or import one item |
| A library folder of packages (glTF + small manifest per item and base model) with a catalogue index (chosen) | Items independent; imports are just more packages; the browser reads one index | Install and copy steps; a catalogue to keep current |

## Decision

Character content is a library on disk: base model packages and items, each a glTF plus a manifest (`*.base.json`, `*.item.json`), listed by a `catalogue.json` index at the library's root. The repository's library is `assets/characters/`. Items carry a `tier` (`built_in` or `premium`) and a `source` (author, SPDX licence). The editor installs the library as global assets and a project copies what it uses (CO9); one default character stays embedded for new projects.

## Reasons

- Binaries stop growing with the wardrobe; an item changes without rebuilding the engine.
- Imported and artist-made items are the same kind of package as built-in ones, so better art can replace any generated item.
- The catalogue lets the browser list and filter a large wardrobe without opening every manifest; it is generated, and a test fails when it is stale.

## Consequences

- `assets/characters/catalogue.py` writes the catalogue; `crates/avatar/tests/shipped_library.rs` checks it equals `Catalogue::of` and lists every manifest.
- Until CO9 the stylized items share `HumanoidParts.glb` and `project` still embeds it; the manifests already describe it.
- Paths in manifests are relative and must stay inside the library.
- Revisit if projects need content streamed from a server rather than copied.

## Implementation references

- `crates/avatar/src/catalogue.rs`, `library.rs`, `item.rs`; `assets/characters/catalogue.py`
- Plan: D7, CO9 (moving the wardrobe out, the import flow)
