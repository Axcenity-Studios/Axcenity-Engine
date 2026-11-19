---
title: "ADR-0044: A garment carries its own pose correctives, baked from cloth simulation and driven by its joints by the body correctives' rule"
type: adr
adr: 44
adr_status: Proposed
date: 2026-11-19
supersedes:
superseded_by:
affects: [avatar, assets, anim]
source: [crates/avatar/src/fit/cloth.rs, crates/avatar/src/item.rs, crates/assets/src/model/morph.rs, crates/assets/src/model.rs, crates/assets/src/model/cooked_model.rs, crates/anim/src/correctives.rs, crates/anim/src/animators.rs, assets/characters/realistic/import/correctives.py]
docs: [docs/development/character-studio/architecture.md, docs/development/character-studio/refinement/wave5/pose-correctives.md, assets/characters/realistic/import/README.md]
tags: [adr, characters, animation, assets]
---

# ADR-0044: A garment carries its own pose correctives, baked from cloth simulation and driven by its joints by the body correctives' rule

- **Status:** Proposed. Built by wave 5's pose-correctives track (stage 3, Paul's approval of 2026-11-18 to fix the fit and deformation first); the lead decides whether the gate's test tee and the licensed tee take their correctives (`refinement/wave5/pose-correctives.md`: they help the stride and hurt some held cases).
- **Date:** 2026-11-19
- **Affected systems:** Character Data (`avatar`: the item manifest's `correctives`, `fit::cloth`), Assets (`MorphTarget::driver`, read from glTF, cooked), Animation (`anim::correctives`: `detect`, `driven`, `drive_own`; followers).

## Context and problem

A fitted garment takes the body's skin weights and copies of its shapes, so in a pose it moves as the skin under it does. Loose cloth doesn't: the bridge between a hanging arm and the side is squeezed and folds over itself, and the skin comes through (`refinement/wave5/tee-walk.md`). The body's own swing correctives (CO5, CO9 part 2) are constants in `anim`, found by name, the same for every body. A garment's corrections are its own: each garment needs its own shapes and rules, and the runtime must drive them on every frame without the applier, in every posing path (players, Character Studio, captures, the fit gate).

## Considered alternatives

- **Add the cloth corrections into the garment's copies of the body's correctives** (no new data): only the shoulders have body correctives, and the walk also needs the elbow, the hip and the knee; and the copies are already the posed fit's (`fit::posed`).
- **Rules in the item manifest only, passed by the applier as an attribute** (as the live identity's key is): the model alone wouldn't pose right (a MeshPart made by hand, a cooked model, a capture without the applier), and the attribute would be parsed per part.
- **Rules in the shape names** (`Corrective_Cloth|LeftUpperArm|0.33,…`): no schema, but unreadable and fragile.
- **Rules in the model's glTF mesh `extras`, read into the shape's `driver`, with the item manifest stating the same** (chosen).

## Decision

- **An item may carry pose correctives:** the item manifest's optional `correctives`, `[{shape, bone, axis, angles: [start, full]}]`, the form architecture.md section 7 gave the body correctives' rule. No version bump; an older engine ignores the field.
- **Its model carries the same rules** in each garment mesh's glTF `extras.axcenity_correctives`. `assets` reads them into `MorphTarget::driver` (`PoseDriver`; a rule naming no shape of the mesh, or whose angles don't rise, is skipped) and keeps them in cooked models. The shipped-library check holds model and manifest equal (`fit::rules_match`, `tests/shipped_library.rs`), and `ItemManifest::check_model` that each rule names a shape of the item and a bone of the body.
- **`anim` drives them by the body correctives' rule** (the bone's twist about the axis, eased): `Correctives::detect` takes a model's own driven shapes before the constant rules; a follower drives only its own (`drive_own`), after taking the leader's weights by name and before its identity. Their names start `Corrective_Cloth_`: live shapes, never identity.
- **They are baked from cloth simulation** by the intake's `correctives.py` around `avatar::fit::cloth` (architecture.md section 9), as a separable step after the fit. A garment without them poses exactly as before.

## Consequences

- A garment's deformation can be corrected per joint and per garment with no runtime cost beyond a weight per rule a frame and the shapes' moves in the posing pass (50 characters: +400 active shapes, +2% moves, +0.02 ms GPU posing; `pose-correctives.md`).
- The shapes are baked for one fit: a refit (the drape-preserving fit, a changed body) makes them stale. `add_pose_shapes` refuses a model whose fingerprint differs; the bake runs again after every refit.
- They are baked on the base body and added to every body's identity shapes: a body far from the base gets the base's correction.
- The first bake (wave 5) does not pass the fit gate's records everywhere, so neither shipped garment carries its correctives yet; the gate measures the test tee with them beside it without them (`player` `pose_corrective_tests`).

## Implementation references

- `crates/avatar/src/item.rs` (`PoseCorrective`, its check), `crates/avatar/src/fit/cloth.rs`, `crates/avatar/examples/pose_correctives.rs`, `assets/characters/realistic/import/correctives.py`.
- `crates/assets/src/model/morph.rs` (`PoseDriver`), `crates/assets/src/model.rs` (`morphs`), `crates/assets/src/model/cooked_model.rs`.
- `crates/anim/src/correctives.rs` (`driven`, `drive_own`), `crates/anim/src/animators.rs` (`follow`).
- Tests: `assets` `a_gltf_meshs_extras_say_which_joint_drives_a_shape`, `a_cooked_model_keeps_the_joints_that_drive_its_shapes`; `anim` `correctives::tests`; `avatar` `tests/cloth_correctives.rs`, `fit::tests::pose_correctives_written_into_a_garments_file_load_as_driven_shapes`, `tests::validation::an_items_pose_correctives_read_back_and_bad_ones_are_reported`; `player` `pose_corrective_tests`.
