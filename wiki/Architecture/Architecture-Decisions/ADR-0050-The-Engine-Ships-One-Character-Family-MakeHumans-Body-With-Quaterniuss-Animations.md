---
title: "ADR-0050: The engine ships one character family: MakeHuman's body, its CC0 wardrobe and Quaternius's animations"
type: adr
adr: 50
adr_status: Accepted
date: 2026-11-25
supersedes: ADR-0032
superseded_by:
affects: [avatar, project, gameplay, anim, assets, editor, player]
source: [crates/project/src/default_humanoid.rs, crates/project/src/game_character.rs, crates/project/src/character_library.rs, crates/avatar/src/migrate.rs, crates/avatar/src/library.rs, crates/editor/src/character_workspace.rs, crates/editor/src/character_workspace/motion.rs, assets/characters/human/default_model.py, assets/characters/human/animations.py, assets/characters/human/mocap.py, assets/characters/human/take_library.py, assets/characters/human/glb.py]
docs: [assets/characters/human/README.md, docs/editor/character-studio.md, docs/research/default-humanoid.md]
tags: [adr, characters, assets, animation]
---

# ADR-0050: The engine ships one character family: MakeHuman's body, its CC0 wardrobe and Quaternius's animations

- **Status:** Accepted (Paul, 2026-11-25: "Instead of keeping the old bases and legacy assets alongside the new pipeline, I want to make a clean break and consolidate Character Studio onto the MakeHuman Humanoid base, the CC0 MakeHuman library, and the Quaternius animation set.")
- **Date:** 2026-11-25
- **Affected systems:** Character Data (`avatar`), the default humanoid and a project's character library (`project`), the Character Runtime's defaults (`gameplay`), Character Studio (`editor`), the starter animations.
- **Supersedes:** [ADR-0032](ADR-0032-Semi-Realistic-Default-Character-Family.md) (the realistic default family with the stylized one beside it). Carries out stage 7 of [ADR-0047](ADR-0047-The-Realistic-Family-Is-Rebuilt-On-MakeHumans-Body-And-Wardrobe.md) (retiring the realistic base) and changes what [ADR-0049](ADR-0049-The-Starter-Animations-Are-Quaterniuss-Library-Baked-Onto-The-Humanoid-Skeleton.md)'s bake targets and keeps.

## Context and problem

The engine shipped three character families: the stylized humanoid (ADR-0030, built into the program, the default humanoid's own body), the realistic one (ADR-0032, the default in Character Studio and for a game's players) and, since ADR-0047, the human base on MakeHuman's body. Each had its own build scripts, wardrobe, presets, benchmarks and tests. The realistic one's wardrobe was the underwear cut from its body, three garments built from patterns and garments generated with Meshy and fitted by `avatar::fit`, six of whose record tests had failed since ADR-0046. The starter animations were keyed on the stylized skeleton and reached the other two through humanoid roles.

Paul asked for one family and no legacy beside it: the human base, every CC0 item of the MakeHuman library, and Quaternius's animations alone.

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Keep all three families | Nothing removed | Three sets of assets, tests and failures to keep; the default humanoid's body isn't what Character Studio edits |
| Keep the stylized humanoid as the light default beside the human base | A 3 MB default character built into the program | Still two families and two skeletons; the animations stay baked for a body nobody edits; not what was asked |
| **One family: the human base, with a light model of its default body built into the program** | One body, one skeleton the takes are baked on, one wardrobe; the default humanoid is the body Character Studio edits | The base's model (30 MB) and wardrobe (270 MB) can't be built into the program; engine features only the realistic base had lose their shipped test asset |

## Decision

1. **`axcenity/human` is the one family** (`avatar::DEFAULT_BASE`). The realistic and stylized packages, their build scripts, benchmarks, the shape-transfer tools and the Meshy import tooling are removed from the repository (its history keeps them). A character that names no base is on the library's default (`Library::default_base`: the human base, or the stylized one in an older project whose own library still has it). Format 1 still migrates to `axcenity/stylized` (`avatar::V1_BASE`); the engine no longer ships that package, so such a file opens on the default character with a notice unless the project has its own copy.
2. **The default humanoid's model is the human base's default body.** `Humanoid.glb` is made from `HumanParts.glb` without Blender (`default_model.py`, `glb.py`): the same skeleton bone for bone, the body, its face parts and the boxer briefs, with the 30 face control shapes kept and the other 309 shapes dropped (4.7 MB). It is built into the program, as the stylized model was: what a character is drawn as before its description is applied, and where a project has no character library. The stylized character's parts file and its API (`modular_character`, `humanoid_part`, `HUMANOID_PARTS`, `Humanoid.customization.json`) are gone, and so is the library built into the program (`built_in_character_library`).
3. **A project's library holds what its characters name, not the wardrobe.** The whole shipped library is 300 MB. `add_character_packages_for` copies a description's base (model, region map, skins), the items it wears and what the base's body types put on; New Project, Insert > Character, Character Studio's Done and Add to Project use it. Character Studio's own folder still takes everything (`add_character_packages`). The default game character is the described default body in MakeHuman's shirt and jeans, brown shoes and short hair (`DEFAULT_GAME_OUTFIT`).
4. **The starter animations are Quaternius's alone, baked on the human base's skeleton.** `animations.py` rebuilds the skeleton out of `HumanParts.glb` (each bone's own axes as the file has them) and `mocap.py` bakes the library's 84 takes onto it, so the engine plays them on the body with nothing retargeted. `Idle` is the library's idle. The fifteen keyed takes are retired with the stylized rig they were keyed on: the default state machine has a state for each of the 84 takes and no other (86 takes with the walk's and the jog's root-motion versions).
5. **The repository's wardrobe is every CC0 item of the converted MakeHuman library**: 51 garments and all 10 hairstyles (`take_library.py`, which takes an item only when the converter's report and the item's own manifest both say CC0 1.0). The 35 CC BY garments stay in the library outside the repository. Every one of the 51 is measured in the fit gate on its own (`every_garment_of_the_human_wardrobe_is_in_the_gate`), and what the gate finds in them is recorded, not fixed: they are drafts as their authors fitted them. The underwear cut from the body is not drawn while a top or a bottom takes its place ([ADR-0045](ADR-0045-What-Is-Worn-Over-Skin-Or-Cloth-Hides-It-By-A-Cover-Map.md)'s amendment of the same day).
6. **Tests follow the assets.** Tests of the retired assets are removed, with the six realistic test-tee record tests that had failed since ADR-0046; tests of the runtime, the applier, the scripts' API, picking, handles, the capture tool and Character Studio are ported to the human base.

## Reasons

- Measured on 2026-11-25: `assets/characters` went from 213 MB (three families) to 309 MB (one, with 51 garments and 10 hairstyles); the realistic package alone was 102 MB.
- The takes are baked for the body people edit: the baked file's skeleton is `HumanParts.glb`'s to 0.6 micrometres and 0.05 degrees a bone, and a clip binds to it by name (`anim` `the_starter_animations_play_on_the_human_bases_own_skeleton`).
- A new project's library is the base and five items (what its default character wears and the underwear of both body types), not 63.
- Porting the realistic base's own checks to the human base found faults its own tests had passed: every face handle's `travel` was half the manifest's contract (`face.py`); the face cameras of the capture tool cut the shorter feminine frame's face off; in Character Studio the nose, the eyes and the cheeks were hardly lit or picked, because the whole-face feature took their skin (`direct.rs` `compete`), and a click on an eyeball picked nothing; MakeHuman's own underwear was filed in slots only the body type can fill, so it could never be worn; and MakeHuman's shoes were not what the footwear check measures.

## Consequences

- **Engine features only the realistic base had are no longer exercised by a shipped asset**: corrective shapes and helper bones (`anim` `correctives.rs`, `twist.rs`'s helpers), makeup and marking layers (ADR-0042), the Skin model's defined normal map, eye and iris values, per-corner body regions, a character's levels of detail, body handles, and `avatar::fit` end to end on a shipped body (its unit tests stay). Their synthetic tests remain; their tests against the realistic body are removed.
- **Motions the library lacks are gone**: walking and running backward and sideways, waving, pointing, using, pulling, climbing a ladder, cheering, clapping and shrugging. Character Studio's preview offers five of the library's gestures instead.
- **A program run away from its files has no family**: the default humanoid is drawn as it is (the default body in its underwear) and can't be given a description. Before, the stylized package was always there.
- **An older project keeps what it has.** Its own copies of the stylized or realistic packages still resolve its characters; nothing brings them up to date any more, and Character Studio offers such a body only if its own library has it.
- The repository's working tree is 96 MB larger, and the installed program's library with it.
- Garments made with Meshy and the wardrobe pack published from them were for the realistic base: nothing the engine ships wears them now. Copies outside the repository (the wardrobe repository, Character Studio's own folder on a person's machine) are untouched.
- Revisit when a second family is wanted again: the code still takes any number of bases (the family row in Character Studio is drawn when a library has more than one).

## Implementation references

- `crates/avatar/src/migrate.rs` (`DEFAULT_BASE`, `V1_BASE`), `library.rs` (`default_base`)
- `crates/project/src/default_humanoid.rs` (the model, the 84 states), `game_character.rs` (`DEFAULT_GAME_BASE`, `DEFAULT_GAME_OUTFIT`, `add_game_character_library`), `character_library.rs` (`add_character_packages_for`)
- `crates/editor/src/character_workspace.rs` (`DEFAULT_BASE`), `motion.rs` (`GESTURES`), `panel.rs` (`base_picker`)
- `assets/characters/human/default_model.py`, `glb.py`, `animations.py`, `mocap.py`, `rig.py`, `take_library.py`
- Tests: `player` `fit_gate_tests` `every_garment_of_the_human_wardrobe_is_in_the_gate`; `project` `a_new_project_gets_the_human_base_and_the_default_characters_clothes`, `a_project_gets_what_its_characters_name_and_no_more`; `anim` `the_starter_animations_play_on_the_human_bases_own_skeleton`; `player` `garment_checks` `human_garments_and_hair_stay_steady_in_the_walk`
