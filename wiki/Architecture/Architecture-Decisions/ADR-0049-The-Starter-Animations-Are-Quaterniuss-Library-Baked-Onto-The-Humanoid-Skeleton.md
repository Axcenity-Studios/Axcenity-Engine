---
title: "ADR-0049: The starter animations are Quaternius's library, baked onto the Humanoid skeleton"
type: adr
adr: 49
adr_status: Proposed
date: 2026-11-25
supersedes:
superseded_by:
affects: [assets, project, anim, player, editor]
source: [assets/characters/human/mocap.py, assets/characters/human/animations.py, crates/project/src/default_humanoid.rs, crates/anim/src/retarget.rs]
docs: [assets/characters/human/README.md, docs/research/default-humanoid.md]
tags: [adr, animation, characters, assets]
---

# ADR-0049: The starter animations are Quaternius's library, baked onto the Humanoid skeleton

- **Status:** Proposed (built 2026-11-25 on Paul's request to rebuild the animations from the two packs he had downloaded; for him to accept after looking at it in the editor)
- **Date:** 2026-11-25
- **Affected systems:** the default humanoid's animation file and state machine (`assets`, `project`), every test and fit record that poses a character with a starter clip (`player`, `editor`). Adds to default-humanoid decision D8 (animations in one glTF file, in place); doesn't change the skeleton contract.

## Context and problem

The default humanoid's 32 starter animations were keyed by a script (`animations.py`: poses written as angles, no motion capture). They read as placeholders: a walk with no weight, an idle that is a pose with a breath. Every character of every base (stylized, realistic, human) plays them.

Paul downloaded Quaternius's **Universal Animation Library** 1 and 2 ("Standard"): 84 takes of human motion, released under CC0 1.0, on Unreal's mannequin skeleton (65 bones named `pelvis`, `upperarm_l`, ...), 30 frames a second, each pack with and without root motion.

The engine can already play such a file as it is: a take for another humanoid skeleton is retargeted through roles at run time (`anim` `retarget.rs`).

## Considered alternatives

| Option | For | Against |
|---|---|---|
| Ship the library's files and let the engine retarget them at run time | Nothing to build; the source as it is | Every character retargets every frame; two skeletons in the default files; the files carry the library's mannequin mesh (15 MB); the rest-pose differences play as part of each take unless a profile is tuned |
| Retarget in the editor's own retarget panel and save clips | The engine's own maths | Clips with keys in project files, made by hand, not rebuildable by a script |
| **Bake the takes onto the Humanoid skeleton offline, into `HumanoidAnimations.glb`** | Nothing changes at run time or in any consumer: the same file, the same take names, keyed the same way; rebuildable from the two source files by one script | A retargeter of our own to keep (250 lines of Python in Blender); the source files stay outside the repository |
| Keep keying by hand | No dependency | It is what is being replaced |

## Decision

1. **`mocap.py` bakes the library onto the Axcenity Humanoid skeleton** in Blender and writes its takes into `HumanoidAnimations.glb` beside the keyed ones, 30 frames a second, in place, Euler XYZ in each bone's own axes, as `animations.py` keys. A take turns each of our bones as far as it turns the library's counterpart from its rest, measured in the world. The limbs are lined up first (our bone's direction at rest turned onto the library's), the hands by their whole frame, and the collarbones take only where the library's point, measured from its standing pose and not its rest (which holds the shoulders 19 degrees back). The hips travel as the library's pelvis does, scaled by hip height, and ride higher or lower so the lower foot is at the height the library's is; then higher again wherever a joint that lies on the ground in the take (the hips, the spine, the head, a shoulder, elbow, hand or knee) would be lower than the library's is, counted over the last 12 cm above the floor. Twist bones, the jaw and the eyes are not keyed.
2. **A library take with a keyed take's name takes its place** (15: Walk, Run, Sprint, Jump, Fall, Land, Crouch, CrouchWalk, PickUp, Push, Interact, Swim, Sit, Dance, HitReact). **The 14 keyed takes the library has no motion for stay** (walking and running backward and sideways, Wave, Point, Use, Pull, Climb, Cheer, Clap, Shrug), **and so does the keyed `Idle`**: the library's idle is a fighter's ready stance, feet wide and one back, and `Idle` is what every character stands in, in a game and in Character Studio (where it is what clothes and bodies are judged in, at ease since 2026-11-24). The library's is `IdleReady`. The two takes with root motion are made from the library's walk and jog. **The library's other 69 takes are added** under engine-style names (`Roll`, `SwordAttack`, `IdleFoldArms`: `mocap.TAKES` is the table). That is 101 takes.
3. **The default state machine has a state for every take** (96 states), so each plays by name. `Walk` and `Run` still play the faster takes sped to the controller's speeds: the library's jog covers 5.36 m a second against `WalkSpeed`'s 5.6, its sprint 8.25 against `RunSpeed`'s 11.2 (speeds read from the library's own root motion). The library's walk, 0.97 m a second, is the state `Stroll`.
4. **The source files stay outside the repository**, as MakeHuman's do (ADR-0047): `build.py --mocap <UAL1_Standard.glb> <UAL2_Standard.glb>` makes the animation file again, and without it the build leaves the file alone. Quaternius is credited in `NOTICE` and the package's README though CC0 asks for nothing.
5. **Fit records follow the motion.** A garment's record in the walk, the run or the idle (`player` `fit_gate_tests`, `garment_checks`, `wardrobe_tests`) is a measurement in a pose a player sees; with other clips it is measured again and recorded with the old number beside it. The realistic test tee's failing records are left failing (Paul, 2026-11-24).

## Reasons

- Measured on 2026-11-25: 84 takes bake in 20 seconds; the file is 5.4 MB (1.35 MB before); our feet end within 9 mm of the library's standing and are corrected by the hips elsewhere (up to 109 mm in the roll before the correction). Lining up the feet alone put the body under the floor in the takes that lie on it (seen in stills of every state the same day: the slide's hips and hand, the death's back); eleven takes are lifted for it, by 21 to 93 mm, and the other 73 are the same to the byte.
- Baking keeps the one property everything downstream relies on: a starter take is keys on the Humanoid skeleton's own bones, read by name, with no retarget at run time on the stylized base.
- The library is CC0: redistributable with the engine and with games made in it.

## Consequences

- The animation file is embedded in the editor and copied into each project that adds the default character: 4 MB more of each.
- A rebuild needs the two packs from quaternius.com; nothing in the repository can remake the 84 takes without them.
- A character stands still in a keyed pose and moves in captured motion. The library's sit is on a seat about 54 cm high (the keyed one was 47 cm).
- The keyed takes that stay (strafes, backward, waving, climbing) now sit beside captured motion and look it.
- The mannequin's proportions are near ours (hips at 0.917 m against 0.910 m), which is why turns alone carry over; a skeleton far from it would want the feet placed, not the hips corrected.
- Revisit if the retarget in `anim` gains what this bake does (limb alignment from rest, a neutral pose for the collarbones): the bake could then be the engine's own, and a creator's library files could get the same treatment in the editor.

## Notes

- **2026-11-25, the same day ([ADR-0050](ADR-0050-The-Engine-Ships-One-Character-Family-MakeHumans-Body-With-Quaterniuss-Animations.md), Paul's consolidation onto one family): the bake's target is the human base's skeleton, and the keyed takes are retired.** What changed in the decisions above:
  - *Decision 1.* The stylized humanoid and its rig are gone. `assets/characters/human/animations.py` rebuilds the skeleton out of `HumanParts.glb` (every joint a bone with the axes the file gives it) and `mocap.py` bakes onto that, so the takes are keys on the bones of the body people edit: the baked file's skeleton is the model's to 0.6 micrometres and 0.05 degrees a bone, and a clip binds by name with nothing retargeted (`anim` `the_starter_animations_play_on_the_human_bases_own_skeleton`).
  - *Decision 2.* `Idle` is the library's idle (Paul: "swap the default Idle state to the Quaternius library's idle stance"); there is no `IdleReady`. All fifteen keyed takes are retired with the rig they were keyed on, so the motions the library lacks are gone: walking and running backward and sideways, Wave, Point, Use, Pull, Climb, Cheer, Clap, Shrug.
  - *Decision 3.* 84 states, one for each library take; the file holds 86 takes (the walk and the jog again with root motion) and is 5.3 MB.
  - *Decision 4.* The command is `animations.py --out <folder> --mocap <UAL1_Standard.glb> <UAL2_Standard.glb>`.
  - *Measured on the human skeleton:* before the hips' correction the lower foot is 18 to 42 mm from the library's in the standing and running takes (the human base's legs are proportioned less like the mannequin's than the stylized rig's were) and up to 91 mm in the swim; eleven takes are lifted to keep the joints that lie on the ground out of it, by 11 to 96 mm. The human base's fit records in the walk were measured again (the men's shirt and jeans 1.77 to 4.25 mm).
  - The status stays Proposed: Paul has not watched the takes in motion.

## Implementation references

- `assets/characters/human/mocap.py` (`TAKES`, `BONES`, `ALIGNED`, `SWUNG`, `Target.pose`, `GROUND`, `bake`), `animations.py` (`skeleton`, `export`, `--mocap`)
- `crates/project/src/default_humanoid.rs` (`STATES`, the speeds)
- Tests: `project` `every_state_plays_a_take_the_animation_file_has_and_the_files_read`
