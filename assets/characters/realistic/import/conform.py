"""Step 4 of the intake: put the item on the realistic body at rest
(NumPy, and `mathutils`'s BVH: Blender's Python).

1. **Place** (optional, for an item not made on a body: a hat, a pair of
   shoes): turn and scale it about its middle, centre it over the skin of
   a bone (`centre_on`; only the skin above `body_above` of its height,
   and the item above `item_above` of its own, are centred), move it
   across and front to back by `offset`, start it clear of that skin and
   lower it along `settle.direction` until it sits `settle.clearance` off
   the skin (`checks.settle`), then move it up or down by `offset`. Every number is in the
   spec; what it did is in the report.
2. **Pose** (optional, for an item made on a body in another pose): the
   arms (and legs) of the body are posed as the item was made
   (`authored_arm_angle`: 0 is a T-pose, 45 the rest pose's A-pose; and
   `authored_leg_angle`, degrees out from straight down), the body's
   skinning is transferred to the item from that posed body
   (`skinning.transfer`, with the spec's `split`), and the item is posed
   back to the body's rest pose by the inverse of the same pose (linear
   blend skinning). Then the engine's fit takes over (`fit.py`).

    python -I conform.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import json
import math
import os

import numpy as np

FORWARD = np.array([0.0, 0.0, 1.0])  # glTF: the body faces +Z


def rotation(axis, degrees, about):
    """A 4x4 rotation by `degrees` about `axis` through the point `about`."""
    axis = np.asarray(axis, float) / np.linalg.norm(axis)
    a = math.radians(degrees)
    x, y, z = axis
    c, s, t = math.cos(a), math.sin(a), 1 - math.cos(a)
    R = np.array([[t * x * x + c, t * x * y - s * z, t * x * z + s * y],
                  [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
                  [t * x * z - s * y, t * y * z + s * x, t * z * z + c]])
    M = np.eye(4)
    M[:3, :3] = R
    M[:3, 3] = np.asarray(about) - R @ np.asarray(about)
    return M


def arm_angle(heads, side):
    """Degrees the upper arm hangs below level, from its joints."""
    d = heads[f"{side}LowerArm"] - heads[f"{side}UpperArm"]
    return math.degrees(math.atan2(-d[1], abs(d[0])))


def leg_angle(heads, side):
    """Degrees the upper leg stands out from straight down."""
    d = heads[f"{side}LowerLeg"] - heads[f"{side}UpperLeg"]
    return math.degrees(math.atan2(abs(d[0]), -d[1]))


def descendants(parents, bone):
    out = {bone}
    changed = True
    while changed:
        changed = False
        for child, parent in parents.items():
            if parent in out and child not in out:
                out.add(child)
                changed = True
    return out


def arm_pose(heads, parents, authored=None, legs_authored=None):
    """Each bone's move from the rest pose to the pose the item was made
    in (4x4, by bone name; a bone left out stays): each upper arm turned
    about its shoulder joint, round the body's forward axis, from the rest
    pose's angle to `authored`; each upper leg likewise from its rest
    spread to `legs_authored`. Every bone below turns with it."""
    pose = {}
    for side, sign in (("Left", 1.0), ("Right", -1.0)):
        if authored is not None and f"{side}UpperArm" in heads and f"{side}LowerArm" in heads:
            # Raising the left arm (out along +X) turns it towards +Y, a
            # positive turn about +Z; the right arm turns the other way.
            delta = (arm_angle(heads, side) - authored) * sign
            M = rotation(FORWARD, delta, heads[f"{side}UpperArm"])
            for bone in descendants(parents, f"{side}UpperArm"):
                pose[bone] = M
        if legs_authored is not None and f"{side}UpperLeg" in heads and f"{side}LowerLeg" in heads:
            delta = (legs_authored - leg_angle(heads, side)) * sign
            M = rotation(FORWARD, delta, heads[f"{side}UpperLeg"])
            for bone in descendants(parents, f"{side}UpperLeg"):
                pose[bone] = M
    return pose


def _blend(points, joints, weights, bones, pose, inverse):
    points = np.asarray(points, float)
    homo = np.hstack([points, np.ones((len(points), 1))])
    out = np.zeros_like(points)
    mats = [pose.get(b) for b in bones]
    for k in range(joints.shape[1]):
        for j in np.unique(joints[:, k]):
            sel = joints[:, k] == j
            w = weights[sel, k]
            if not np.any(w):
                continue
            M = mats[j] if j < len(mats) else None
            if M is None:
                moved = points[sel]
            else:
                M = np.linalg.inv(M) if inverse else M
                moved = (homo[sel] @ M.T)[:, :3]
            out[sel] += w[:, None] * moved
    total = weights.sum(axis=1, keepdims=True)
    return np.where(total > 1e-9, out / np.maximum(total, 1e-9), points)


def to_pose(points, joints, weights, bones, pose):
    """Linear blend skinning from the rest pose into `pose`."""
    return _blend(points, joints, weights, bones, pose, inverse=False)


def to_rest(points, joints, weights, bones, pose):
    """Linear blend skinning back from `pose` to the rest pose (each bone's
    move undone, blended by the weights)."""
    return _blend(points, joints, weights, bones, pose, inverse=True)


def place(mesh, body, spec):
    """Turns, scales, centres and settles an item not made on a body (see
    the module's note); returns what it did."""
    import bodyglb
    import checks
    P = mesh.P
    centre = (P.min(axis=0) + P.max(axis=0)) / 2
    report = {}
    scale = float(spec.get("scale", 1.0))
    rotate = spec.get("rotate", [0, 0, 0])
    for axis, degrees in zip(np.eye(3), rotate):
        if degrees:
            M = rotation(axis, degrees, centre)
            P = (np.hstack([P, np.ones((len(P), 1))]) @ M.T)[:, :3]
    P = centre + (P - centre) * scale
    report["scale"], report["rotate_degrees"] = scale, list(rotate)
    if spec.get("centre_on"):
        skin = bodyglb.bone_region(body, spec["centre_on"])
        # Only the skin (and the item) above this share of its height is
        # centred: a hat's crown over the skull, not its brim over the
        # face with the nose.
        lo_s, hi_s = skin[:, 1].min(), skin[:, 1].max()
        part = skin[skin[:, 1] >= lo_s + (hi_s - lo_s) * spec.get("body_above", 0.0)]
        lo, hi = P.min(axis=0), P.max(axis=0)
        top = P[P[:, 1] >= lo[1] + (hi[1] - lo[1]) * spec.get("item_above", 0.0)]
        mid = (part.min(axis=0) + part.max(axis=0)) / 2
        own = (top.min(axis=0) + top.max(axis=0)) / 2
        shift = np.array([mid[0] - own[0], 0.0, mid[2] - own[2]])
        # Start clear above the skin, then settle down onto it.
        shift[1] = hi_s + 0.05 - lo[1]
        P = P + shift
        report["centred_on"] = spec["centre_on"]
    # The offset across and front to back comes before settling (the item
    # then comes down where it is put); up and down after it.
    offset = np.asarray(spec.get("offset", [0, 0, 0]), float)
    P = P + offset * [1, 0, 1]
    settle = spec.get("settle")
    if settle:
        direction = np.asarray(settle.get("direction", [0, -1, 0]), float)
        drop = checks.settle(P, body.P, body.tris, direction, settle.get("clearance", 0.004))
        P = P + drop * direction / np.linalg.norm(direction)
        report["settled_m"] = round(drop, 4)
    P = P + offset * [0, 1, 0]
    report["offset_m"] = offset.tolist()
    d = checks.distances(P, body.P, body.tris, signed=True)
    report["clearance_m"] = {"least": round(float(d.min()), 4), "median": round(float(np.median(d)), 4),
                             "inside_body": int((d < 0).sum())}
    report["bounds_m"] = [P.min(axis=0).round(4).tolist(), P.max(axis=0).round(4).tolist()]
    mesh.P = P
    return report


def source_of(spec):
    """The step's input: the refinement's mesh when the spec names a
    recipe (`refine.py`), else the topology step's."""
    return "refined.npz" if spec.get("refine") else "topology.npz"


def step(spec, work, source="topology.npz", target="conformed.npz"):
    import bodyglb
    import skinning
    from workmesh import WorkMesh
    body = bodyglb.load(spec.get("body"))
    mesh = WorkMesh.load(os.path.join(work, source))
    options = spec.get("conform", {})
    report = {}
    if options.get("place"):
        report["place"] = place(mesh, body, options["place"])
    arms = options.get("authored_arm_angle")
    legs = options.get("authored_leg_angle")
    if arms is not None or legs is not None:
        pose = arm_pose(body.heads, body.parents, arms, legs)
        posed = skinning.Body(to_pose(body.P, body.joints, body.weights, body.bones, pose), body.tris,
                              body.joints, body.weights, body.bones,
                              {b: (pose[b] @ np.append(h, 1))[:3] if b in pose else h
                               for b, h in body.heads.items()}, body.parents)
        joints, weights = skinning.transfer(posed, mesh.P, split=spec.get("weights", {}).get("split"))
        before = mesh.P.copy()
        mesh.P = to_rest(mesh.P, joints, weights, body.bones, pose)
        moved = np.linalg.norm(mesh.P - before, axis=1)
        report["pose"] = {
            "rest_arm_angle": round(arm_angle(body.heads, "Left"), 2),
            "authored_arm_angle": arms,
            "rest_leg_angle": round(leg_angle(body.heads, "Left"), 2),
            "authored_leg_angle": legs,
            "moved_m": {"most": round(float(moved.max()), 4), "median": round(float(np.median(moved)), 4)},
        }
    mesh.save(os.path.join(work, target), report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work, source=source_of(spec)), indent=1))


if __name__ == "__main__":
    main()
