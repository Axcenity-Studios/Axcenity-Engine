"""Does the skin cross itself round the shoulders when the arms are raised?

    python -I posed_crossings.py [--package realistic] [--out FILE.json]

For each frame and a few builds (the default body and the heavy ones),
counts the pairs of the body's triangles that cross in a pose and don't
at rest: in the QA pose `shoulder_abd_150` and in the starter animations'
Cheer at 0.8 s, with the twist and helper bones and the corrective shapes
driven as the engine drives them. A pair counts as new when it doesn't
cross in the same body at rest and isn't already within 4 mm of touching
there. "Round the shoulders" is within 22 cm of either upper arm's joint
in the pose; "in the hands" within 16 cm of either wrist (Cheer makes
fists, and the fingers of a fist pass through each other: the hands'
own count in measure.py's `fist` pose). A dark armpit with no crossings
is shadow and occlusion, not broken skin.
"""

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402
import numpy as np  # noqa: E402

import measure as M  # noqa: E402
import qa_scene as Q  # noqa: E402

BUILDS = [("default", {}), ("heavy and muscular", {"muscle": 1.0, "fat": 1.0}),
          ("heavy and soft", {"muscle": -1.0, "fat": 1.0})]
NEAR = 0.22


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="realistic", choices=sorted(Q.PACKAGES))
    ap.add_argument("--out", default="")
    a = ap.parse_args()
    parts, _, custom = Q.package_paths(a.package)
    Q.reset()
    ch = Q.Character(parts, merge_vertices=True, custom=Q.load_custom(custom))
    ch.show_only(["Body"])
    take_arm, actions = Q.load_takes(os.path.join(Q.ASSET_DIR, "HumanoidAnimations.glb"))
    start, end = actions["Cheer"].frame_range
    share = min(1.0, 0.8 * bpy.context.scene.render.fps / max(end - start, 1e-6))
    poses = [("shoulder_abd_150", lambda: Q.apply_pose(ch, "shoulder_abd_150")),
             ("Cheer at 0.8 s", lambda: Q.pose_take(ch, take_arm, actions["Cheer"], share))]
    out = {}
    for label, frame in (("masculine", 0.0), ("feminine", 1.0)):
        for build, values in BUILDS:
            ch.pose_reset()
            ch.set_sliders(dict(values, frame=frame))
            Q.update()
            rest = Q.Geo(ch.parts["Body"])
            known = M.crossing_pairs(rest)
            tris, P = rest.tris, rest.co
            for pose, apply in poses:
                ch.set_sliders(dict(values, frame=frame))
                apply()
                Q.update()
                geo = Q.Geo(ch.parts["Body"])
                joints = [np.array(ch.arm.pose.bones[s + "UpperArm"].head) for s in ("Left", "Right")]
                hands = [np.array(ch.arm.pose.bones[s + "Hand"].head) for s in ("Left", "Right")]
                new = near = fists = 0
                for i, j in M.crossing_pairs(geo) - known:
                    if np.linalg.norm(P[tris[i]][:, None] - P[tris[j]][None], axis=2).min() <= 0.004:
                        continue
                    new += 1
                    at = geo.co[tris[i]].mean(axis=0)
                    near += min(np.linalg.norm(at - c) for c in joints) < NEAR
                    fists += min(np.linalg.norm(at - c) for c in hands) < 0.16
                out[f"{label}/{build}/{pose}"] = {"crossing_at_rest": len(known), "new_crossings": new,
                                                  "new_crossings_round_the_shoulders": int(near),
                                                  "new_crossings_in_the_hands": int(fists)}
                print(f"{label:9s} {build:18s} {pose:16s} rest {len(known):3d}  new {new:3d}  round the shoulders {near:3d}  "
                      f"in the hands {fists:3d}", flush=True)
    if a.out:
        with open(a.out, "w", newline="\n") as f:
            json.dump(out, f, indent=1)
            f.write("\n")


if __name__ == "__main__":
    main()
