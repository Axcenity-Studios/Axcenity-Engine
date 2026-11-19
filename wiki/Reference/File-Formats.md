---
title: File formats
type: reference
status: current
verified: 2026-11-11
verified_commit: 19c7179d
source: [crates/project/src/lib.rs, crates/project/src/world_file.rs, crates/project/src/packed.rs, crates/project/src/package.rs, crates/anim/src/files.rs, crates/save/src/lib.rs, crates/assets/src/cooked.rs, crates/assets/src/asset_id.rs]
docs: [docs/project/layout.md, docs/project/world.md, docs/project/animation-files.md, docs/project/export.md, docs/project/packages.md, docs/project/data-tables.md, docs/project/sequences.md, docs/architecture/saves.md, docs/rendering/models.md, docs/rendering/textures.md]
tags: [reference, file-formats]
---

# File formats

Every file the engine reads or writes, with its owner and where the full format is documented. Versioning rules: [Change a file format](../Guides/Recipes/Change-a-File-Format.md).

## Project files (committed)

| File | Holds | Owner | Version | Format doc |
|---|---|---|---|---|
| `axcenity.json` | Manifest: name, `format_version`, `engine` (`created_with`, `saved_with`), `version`, start level, loading screen, splash, developer console | `project` | `FORMAT_VERSION` 6 | `docs/project/world.md` |
| `world.json`, `levels/<name>.world.json` | Instances (services, Workspace) | `project::world_file` | project's | `docs/project/world.md`, `levels.md` |
| `scripts/**/*.server.lua`, `*.client.lua`, `*.module.lua` | Luau | `project`, `script` | — | `docs/scripting/client-server.md`, `modules.md` |
| `*.server.blocks.json`, `*.client.blocks.json` | Block scripts (+ Graph view layout) | `blocks-runtime` | — | `docs/project/world.md` |
| `ui/*.css`, `ui/*.uigraph.json` | Stylesheets; UI Event Graphs (format_version 1) | `ui`, `project::ui_graphs` | — | `docs/ui.md`, `docs/scripting/ui-graphs.md` |
| `input.json`, `strings.json`, `collision-groups.json` | Input map, string table, collision groups | `core`, `project` | optional files | `docs/project/layout.md` |
| `assets/animations/**/*.anim.json`, `*.statemachine.json` | Clips, state machines | `anim` | `anim::files::FORMAT_VERSION` 2 | `docs/project/animation-files.md` |
| `*.sequence.json` | Cutscene sequences | `anim::sequence` | — | `docs/project/sequences.md` |
| `*.material.json` | Material files (values, maps, shader, graph) | `assets::material` | optional | `docs/rendering/models.md` |
| `*.texture.json` | Texture sets | `assets::texture_set` | — | `docs/editor/assets.md` |
| `*.prefab.json` | Prefabs, variants | `project::prefabs` | — | `docs/project/world.md` |
| `*.data.json` | Data tables | `assets::data_table` | — | `docs/project/data-tables.md` |
| `*.behaviortree.json`, `*.dialogue.json` | Behavior trees, dialogues | `nav`, `script` | — | `docs/architecture/navigation.md`, `docs/scripting/api-dialogue.md` |
| `*.meshedit.json` | Model Studio sidecar (modifiers, paint layers, sculpt masks) | `mesh_edit::sidecar` | — | `docs/editor/model-studio.md` |
| `<asset>.import.json` | Stable asset ID `ax://…`, import preset, the file it was converted from (`converted_from`, P7.2) | `assets::asset_id` | format_version 1 | `docs/editor/assets.md` |
| `import-presets.json` | Texture import presets and folders' presets (R12), model import presets (`model_presets`, P7.3) | `assets::import_presets`, `assets::model_presets` | format_version 1 | `docs/editor/assets.md`, `docs/editor/import.md` |
| `tests/**/*.spec.lua` | Game tests | `testing` | — | `docs/project/game-tests.md` |
| `objects/**/*.lua` | Scripts inside objects, named by a script node's `SourceFile` (G2); world, level and prefab files holding such nodes are written as format `2` | `project` | — | `docs/project/world.md` |
| `build-profiles.json`; beside a built game `build-report.json`, `build-report.txt` | Build profiles (R13; not packed); what a build put in the game and with what settings | `project::build_profiles` | format_version 1 | `docs/project/export.md` |
| `CREDITS.txt` | Credits for assets fetched through the Toolbox | `toolbox::provenance` | — | `docs/research/asset-sources/` |
| `CREDITS-CHARACTERS.txt` (inside `game.axgame` and beside a built game; not committed) | The credits of the character items made from imported assets: per credit the attribution line, the licence and the address of its text, the source, what was changed, the items and the MIT or Apache notice; thanks for public-domain assets; items not cleared for shipping; the Apache License's terms once when an item is under it (Paul's decision 11) | `avatar::character_credits`, `project::character_credits` | — (plain text, written fresh each build) | `docs/development/character-studio/architecture.md` §8a, `docs/project/export.md` |
| `assets/characters/**/<Object>.provenance.json` | An imported item's provenance record, beside its manifest (named by `source.provenance`): source page, author, licence and the address of its text, retrieval date, each downloaded file's name and SHA-256, the changes, the attribution line, the MIT or Apache notice, the items it feeds, the licence evidence | `avatar::Provenance` | format_version 1 (a newer one is refused) | `docs/development/character-studio/architecture.md` §8a |
| `assets/probes/*` | Reflection-probe and light-probe bakes | `assets::probe`, `light_probes` | — | `docs/rendering/sky.md` |
| Terrain folders (`heights.png`, paint maps, optional `foliage.json`, `holes.png` and `water.png`) | Terrain data; each painted instance's `lean` and `sink` are optional (left out at 0, so older files read the same) | `terrain::files`, `project::terrain_files` | `GENERATOR_VERSION` for generators | `docs/terrain.md` |
| `presets/*.axscatter` | Scatter palettes: the Scatter pane's items (`prop:Crate`, `plant:Grass` or a model path) with weight, size range, collide, rotation, align to slope and sink, and the palette's density and spacing | `project::scatter_palettes` | `format` 1 | `docs/editor/scatter.md` |
| `assets/characters/` (`catalogue.json`, `<package>/*.base.json`, `<package>/items/*.item.json`, the packages' glTF files) | The project's character library, which applying a description resolves against (Character Studio overhaul CO3). `project::add_character_library` writes the engine's built-in stylized package there, keeping files already present | `avatar` (`LIBRARY_FOLDER`), `project::character_library` | format_version 1 each | `docs/development/character-studio/architecture.md` §15 |

## Per-machine files (git-ignored `.axcenity/`)

| Path | Holds |
|---|---|
| `.axcenity/imported/textures/*.axtex` (incl. `embedded-<hash>.axtex`) | Cooked BC7/BC5 textures with mips |
| `.axcenity/imported/meshes/*.axlod` | Simplified LOD caches |
| `.axcenity/imported/models/*.axmodel`, `*-embedded.done` | Cooked whole models; embedded-copy notes |
| `.axcenity/asset-ids.json` | Remembered path → asset ID |
| `.axcenity/previous/<version>/` | Files kept by a format migration |
| `.axcenity/asset-check.json` | Ignored Asset Check findings |
| `.axcenity/asset-sources.json` | Where each Toolbox asset came from (provenance) |
| `.axcenity/history`, version history, undo | Editor state |
| `.axcenity/axcenity.d.luau` | Luau definitions for VS Code |

## Shipped and exchanged files

| File | Holds | Owner |
|---|---|---|
| `game.axgame` | Packed game: `AXCGAME\0`, version 2, entries with flags, mtime, lengths; zstd per entry when it saves ≥ 5% | `project::packed` |
| `*.axpkg` | Asset package (zip + manifest) | `project::package` |
| `*.character.json` | A character: base model, parameter values, worn items, material values (format 2; format 1, Character Studio CS2's `Look`, migrates on load). The workspace still writes format 1 until CO12. Since CO3 a description is applied at runtime and kept in the World as a `HumanoidDescription` whose `Data` is this text, so a saved `world.json` holds it, with the character's derived `MeshId`s (`file.glb#Objects#i<key>`: the base model's ID plus an identity key; the derived model itself is never on disk, `docs/project/assets.md`) and its `Mat_<material>_<value>` attributes | `avatar::Description`; `docs/development/character-studio/architecture.md` §5 |
| `*.base.json`, `*.item.json`, `catalogue.json` | A character library: base model manifests (parameters, regions, slots, sockets, materials, skeleton, face), item manifests (garments, accessories, grooms) and the generated index; the repository's library is `assets/characters/`, and a project's is its own `assets/characters/` (format_version 1 each) | `avatar`; `architecture.md` §6–8, §15 |

## Engine repository files

| File | Holds | Owner |
|---|---|---|
| `releases/unreleased/*`, `releases/history/<version>.toml` | Change entries and release records | `release` |
| `axcenity-release.json` (+ `.minisig`), `axcenity-history.json` | Published release manifest (signed) and public history | `release`, release workflow |
| Toolbox `metadata.db` (in the central asset directory) | SQLite record of downloaded assets, versioned schema | `toolbox::db` |

## Player data (never in the project)

| File | Holds | Owner |
|---|---|---|
| `slot-<n>.json`, `players/<UserId>.json`, `players/<UserId>/` | Save slots and per-player data | `save` (`SAVE_VERSION` 2) |
| `user-id.txt` | The install's random `UserId` | `save` |
| `graphics.json` | The player's graphics settings | `player` |
| `logs/latest.log` | Player log and crash message | `player::logs` |

## Wire

The network protocol (`PROTOCOL_VERSION` 10): `Hello`/`Refused` as JSON, everything else postcard after a format byte. See [Networking](../Engine-Systems/Networking.md).
