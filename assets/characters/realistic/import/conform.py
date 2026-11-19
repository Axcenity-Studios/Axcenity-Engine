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
3. **Grade** (optional, wave 5 stage 3): the item's girth measured round
   the body at the heights the spec gives (`grade.at`, metres: the chest,
   waist and hips), against the body's there; where the item has less than
   `grade.least_ease` (metres of girth) at any of them (the body's girth
   taken across the item's width there and 3 cm past it, not round the
   arms), it is scaled across there (X and Z, about the body's middle)
   until it has, the scale going linearly between the heights and back to
   1 at the item's top (its neck), before anything presses it onto the
   skin.
4. **Drape** (optional, wave 5 stage 3): the item hung on the body by
   Blender's cloth simulation (`../clothing/drape.py`): every corner still
   inside the body (or nearer than `drape.start_clearance`) is first moved
   out along the body's normal, so the cloth starts outside it; the body
   is a collision object at rest, the item's own edges are its springs' rest
   lengths, its corners within `drape.pin_top` of its top are pinned
   where they are (a tee's neck and shoulders; a pant's waistband), and
   gravity does the rest. It keeps its ease and hangs (a tee's hem falls
   from the chest and the shoulder blades, free below the seat) instead of
   being pushed onto the skin. `drape.settings` are the cloth settings
   (`drape.Settings`). The export then fits it with `--drape`
   (`FitOptions::drape`), which keeps that ease as the body changes.

    python -I conform.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import json
import math
import os
import sys

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


def _hull_perimeter(xz):
    """The perimeter of the convex hull of 2D points (monotone chain)."""
    pts = sorted(set(map(tuple, np.round(np.asarray(xz, float), 6))))
    if len(pts) < 3:
        return 0.0

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    hull = np.array(lower[:-1] + upper[:-1])
    return float(np.linalg.norm(hull - np.roll(hull, -1, axis=0), axis=1).sum())


def _triangles(body):
    """A body's triangles (a `bodyglb` body's, or a work mesh's)."""
    tris = getattr(body, "tris", None)
    return np.asarray(body.triangles() if tris is None else tris, np.int64)


def girth(points, y, band=0.01, x_range=None):
    """The girth round `points` at height `y`: the perimeter of the convex
    hull of those within `band` of it, seen from above (a tape measure
    round the outside). With `x_range` (lo, hi), only points across that
    (the torso between the arms)."""
    P = np.asarray(points, float)
    sel = np.abs(P[:, 1] - y) <= band
    if x_range is not None:
        sel &= (P[:, 0] >= x_range[0]) & (P[:, 0] <= x_range[1])
    return _hull_perimeter(P[sel][:, [0, 2]])


def grade(mesh, body, spec):
    """Grades the item across (X and Z, about the body's middle) so it has
    at least `least_ease` of girth over the body's at every height in `at`
    (see the module's note): each height's scale is what it needs (1 where
    it has the ease), and between them, and up to the item's top (its
    neck, where it is 1 again), the scale goes linearly; below the lowest
    height it stays. Returns the girths and the scales."""
    least = float(spec.get("least_ease", 0.0))
    rows = []
    knots = []
    for y in spec.get("at", []):
        sel = np.abs(mesh.P[:, 1] - y) <= 0.01
        if not sel.any():
            continue
        # The body across the item's width there, and a little past it (an
        # item too small for the body is narrower than it).
        lo, hi = mesh.P[sel, 0].min() - 0.03, mesh.P[sel, 0].max() + 0.03
        item = girth(mesh.P, y)
        skin = girth(body.P, y, x_range=(lo, hi))
        scale = max(1.0, (skin + least) / item) if item > 0 else 1.0
        body_sel = (np.abs(body.P[:, 1] - y) <= 0.01) & (body.P[:, 0] >= lo) & (body.P[:, 0] <= hi)
        around = body.P[body_sel] if body_sel.any() else mesh.P[sel]
        centre = (around.min(axis=0) + around.max(axis=0)) / 2
        knots.append((y, scale, centre))
        rows.append({"at_m": y, "item_m": round(item, 4), "body_m": round(skin, 4),
                     "ease_m": round(item - skin, 4), "scale": round(scale, 4)})
    most = max([k[1] for k in knots], default=1.0)
    if most > 1.0:
        knots.sort(key=lambda k: k[0])
        top = mesh.P[:, 1].max()
        ys = [k[0] for k in knots] + [top]
        ss = [k[1] for k in knots] + [1.0]
        xs = [k[2][0] for k in knots] + [knots[-1][2][0]]
        zs = [k[2][2] for k in knots] + [knots[-1][2][2]]
        y = mesh.P[:, 1]
        s = np.interp(y, ys, ss)
        cx, cz = np.interp(y, ys, xs), np.interp(y, ys, zs)
        mesh.P = mesh.P.copy()
        mesh.P[:, 0] = cx + (mesh.P[:, 0] - cx) * s
        mesh.P[:, 2] = cz + (mesh.P[:, 2] - cz) * s
        for row in rows:
            row["item_after_m"] = round(girth(mesh.P, row["at_m"]), 4)
    return {"least_ease_m": least, "girths": rows, "scale": round(most, 4)}


def push_out(mesh, body, clearance):
    """Moves every corner of the item inside the body, or nearer it than
    `clearance`, out along the body's normal at its nearest point, so the
    cloth simulation starts outside the body; returns how many and how
    far."""
    import checks
    tree = checks._tree(body.P, _triangles(body))
    moved = []
    P = mesh.P.copy()
    for k, p in enumerate(P):
        loc, normal, _, _ = tree.find_nearest(tuple(p))
        if loc is None:
            continue
        n = np.array(normal)
        off = float(np.dot(p - np.array(loc), n))
        if off < clearance:
            P[k] = p + n * (clearance - off)
            moved.append(clearance - off)
    mesh.P = P
    return {"pushed": len(moved), "most_m": round(max(moved, default=0.0), 4)}


def drape(mesh, body, spec):
    """Hangs the item on the body by Blender's cloth simulation (see the
    module's note); returns what it did."""
    import bpy
    sys.path.append(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "clothing"))
    import drape as cloth  # the realistic wardrobe's cloth simulation
    import checks

    def to_blender(P):
        return np.stack([P[:, 0], -P[:, 2], P[:, 1]], axis=1)

    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    me = bpy.data.meshes.new("Body")
    tris = _triangles(body)
    me.from_pydata(to_blender(body.P).tolist(), [], tris.tolist())
    me.update()
    collider = bpy.data.objects.new("Body", me)
    scene.collection.objects.link(collider)
    collider.modifiers.new("Collision", "COLLISION")
    collider.collision.thickness_outer = 0.002
    collider.collision.thickness_inner = 0.02
    collider.collision.damping = 0.5
    collider.collision.cloth_friction = 5.0
    pushed = push_out(mesh, body, float(spec.get("start_clearance", 0.004)))
    top = mesh.P[:, 1].max()
    pins = np.where(mesh.P[:, 1] >= top - float(spec.get("pin_top", 0.05)))[0]
    settings = cloth.Settings(**spec.get("settings", {}))
    before = mesh.P.copy()
    out = cloth.simulate(to_blender(mesh.P), [list(map(int, f)) for f in mesh.faces()], [],
                         settings, pins=pins, name="Item")
    mesh.P = np.stack([out[:, 0], out[:, 2], -out[:, 1]], axis=1)
    moved = np.linalg.norm(mesh.P - before, axis=1)
    d = checks.distances(mesh.P, body.P, tris, signed=True)
    # The rest lengths kept: each edge now against before.
    edges = set()
    for f in mesh.faces():
        for a, b in zip(f, np.roll(f, -1)):
            edges.add((min(a, b), max(a, b)))
    edges = np.array(sorted(edges))
    was = np.linalg.norm(before[edges[:, 0]] - before[edges[:, 1]], axis=1)
    now = np.linalg.norm(mesh.P[edges[:, 0]] - mesh.P[edges[:, 1]], axis=1)
    ratio = now / np.maximum(was, 1e-6)
    return {
        "pushed_out_first": pushed,
        "pinned": int(len(pins)),
        "frames": settings.frames,
        "moved_m": {"most": round(float(moved.max()), 4), "median": round(float(np.median(moved)), 4)},
        "clearance_m": {"least": round(float(d.min()), 4), "median": round(float(np.median(d)), 4),
                        "inside_body": int((d < 0).sum())},
        "edges_stretched": {"p95": round(float(np.percentile(ratio, 95)), 4),
                            "most": round(float(ratio.max()), 4)},
    }


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
    if options.get("grade"):
        report["grade"] = grade(mesh, body, options["grade"])
    if options.get("drape"):
        report["drape"] = drape(mesh, body, options["drape"])
    mesh.save(os.path.join(work, target), report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work), indent=1))


if __name__ == "__main__":
    main()
