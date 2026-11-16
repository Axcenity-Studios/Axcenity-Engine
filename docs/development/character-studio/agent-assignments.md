# Character Studio overhaul: agent assignments

Who does what from Stage 3 on, how they work side by side and how work comes back. Section 5 of [the plan](../../../plans/axcenity-engine-character-studio.md) is the summary; this file is the working rule. The contracts every agent builds on are in [architecture.md](architecture.md).

## Agents and what each owns

"Owns" means only that agent changes those files. Anything outside a row's paths goes through the agent that owns it or through the lead.

| Agent | Owns | Consumes (never changes) |
|---|---|---|
| **Lead** (architect, integrator) | `crates/avatar` schemas (`description.rs`, `manifest.rs`, `item.rs`, `catalogue.rs`, `material.rs`, `migrate.rs`, `ids.rs`, `library.rs`, `resolve.rs`); `Cargo.toml` and `Cargo.lock`; `assets/characters/<family>/build.py`, `manifests.py`, `assets/characters/catalogue.py`; the format sections of `architecture.md`; ADRs; the plan; `docs/development/character-studio/`; the runtime applier (CO3: `script`'s `HumanoidDescription`, `anim`'s bone changes and shape followers with Agent 1); merging | Everything |
| **1 Modeling and anatomy** | `assets/characters/realistic/` body modules (`body.py`, `rig.py`, `bodyshapes.py`, skin bakes), `source/` of that package; the realistic base's parameters in its manifest generator; twist bones and correctives in `anim` (`body_shape.rs`, a new `twist.rs`, `correctives.rs`) with the lead | The skeleton (architecture §12) and manifest (§6–7) contracts |
| **2 Facial systems** | The realistic head's face, eye, brow and lash modules; `crates/anim/faces/` profiles and expressions; `anim/src/face*.rs` changes for new heads | The head topology from Agent 1; FaceControls (ADR-0028) |
| **3 Hair and simulation** | `assets/characters/hair/` (grooms, card generation); the groom converter in `crates/convert`; the groom runtime (`anim` spring bones, guide simulation; the strand `DrawList` field with Agent 6) | Shading from Agent 6; the head surface from Agents 1 and 2; the groom contract (§11) |
| **4 Clothing and wardrobe** | Garment build modules and `assets/characters/<family>/items/`; `crates/avatar/src/fit.rs` and `import.rs` (functions whose signatures the lead defines, §16); the import flow's editor dialog with Agent 5 | Item manifest (§8), regions and binding (§9); cloth shading from Agent 6 |
| **5 Studio UI and UX** | `crates/editor/src/character_workspace*`, `start/characters.rs`, `app/character_workspace.rs`, the face tab's placement and name | The manifest and catalogue (never their formats) |
| **6 Rendering and performance** | `crates/render`, `crates/render-types`, `crates/scene` material translation, `docs/benchmarks.md`, the wiki's performance pages | The material parameter contract (§10) |

Each crate's `CLAUDE.md` and the [development policy](../../development-policy.md) still apply: `player` never depends on `editor`, `egui` or `blocks`; `render` never on Instances.

## Assignments by stage

| Milestone | Owner | Helpers | Starts when | Branch |
|---|---|---|---|---|
| CO2 visual QA harness | Lead | Agent 6 (capture) | Now (Stage 3) | `co2-qa-harness` |
| CO3 runtime applier | Lead | Agent 1 | Now | `co3-applier` |
| CO4 realistic base body | Agent 1 | | Now | `co4-realistic-body` |
| CO5 body parameters | Agent 1 | | After the MakeHuman transfer spike | `co5-body-parameters` |
| CO6 head and face | Agent 2 | | When CO4's head topology is fixed | `co6-head-face` |
| **Checkpoint** | Lead | all | CO2–CO6 merged | A correctly modelled character in both frames, by the benchmarks ([visual-qa.md](visual-qa.md)) |
| CO7 character shading | Agent 6 | | Stage 4 | `co7-shading` |
| CO8 compute posing | Agent 6 | | Stage 4 | `co8-compute-posing` |
| CO9 clothing system | Agent 4 | Agent 5 (import dialog) | Stage 4 | `co9-clothing` |
| CO10 wardrobe wave 1 | Agent 4 | | After CO9's binding works | `co10-wardrobe-1` |
| CO11 hair cards | Agent 3 | Agent 6 (hair shading) | Stage 4 | `co11-hair-cards` |
| CO12 studio interface | Agent 5 | | Stage 4 | `co12-studio` |
| CO13 integration | Lead | Agent 5 | Stage 5 | `co13-integration` |
| CO14–CO18 | per the plan | | Stage 6 | `co14-…` |

## Working side by side

- **Worktrees.** Each agent works in its own git worktree on its own branch from the current `main`: `git worktree add ../axcenity-<branch> -b <branch> main`. Never two agents on one branch.
- **Rebase on contracts, not on each other.** When a contract change lands on `main`, every open branch merges `main` in (no force-push of shared branches).
- **Contract changes.** A specialist who needs a schema field, a new value in the material set, a new driver kind or a changed signature writes a short request (what, why, which milestone needs it) to the lead. The lead changes `avatar` and `architecture.md` in one PR with tests, bumping a version if the meaning changes (architecture §4), then specialists merge it. Nobody edits another agent's files to unblock themselves.
- **Shared files.** `Cargo.toml`, `Cargo.lock` and `crates/workspace-hack` change only through the lead (`cargo hakari generate`).
- **Large binaries.** Built `*.glb`, textures and grooms are committed by the agent that owns the package, with the script that made them; never hand-edited.

## Hand-back

Every PR description from a specialist has:

1. **Files changed**, grouped by owner (all must be the agent's own).
2. **Features**: what now works, in a sentence each.
3. **Interfaces** used or added (with links to `architecture.md` sections).
4. **Tests** written, and the commands run with their summary lines (`cargo nextest run …`: N passed, M failed; `measure.py --check`: pass or the regressions).
5. **Before and after** for every visual change: the benchmark sheets ([visual-qa.md](visual-qa.md)) from `main` and from the branch.
6. **Dependencies** added (none without the lead; [dependencies.md](dependencies.md)).
7. **Open issues** and **integration needs** (what another agent or the lead must do next).

## Review

The lead reviews **the code and the test output**, not the summary: runs the branch's tests and the benchmarks locally, opens the before and after sheets side by side, and checks the ownership rule. A milestone is accepted only when its "Done when" holds **and** the characters look and behave better in the benchmarks (section 0: a stage doesn't advance because the code works). Merges to `main` are Paul's.
