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
   waist and hips; `[height, least ease]` for a height's own ease),
   against the body's there; where the item has less than
   `grade.least_ease` (metres of girth) at any of them (the body's girth
   taken across the item's width there and 3 cm past it, without the arms
   and the sleeves), it is scaled across there (X and Z, about the body's
   middle) until it has, the scale going linearly between the heights and
   held above the highest (to the neck: review fixes 1; it used to go back
   to 1 there, leaving the neck and shoulders inside the body) and below
   the lowest. With `grade.sleeves`, each sleeve is graded round its arm
   too (`grade_sleeve`), before anything presses it onto the skin.
4. **Drape** (optional, wave 5 stage 3): the item hung on the body by
   Blender's cloth simulation (`../clothing/drape.py`): every corner still
   inside the body (or nearer than `drape.start_clearance`) is first moved
   out along the body's normal (with `drape.smooth_push`, as one smooth
   field over the cloth: `push_out`), so the cloth starts outside it; the
   body is a collision object at rest, the item's own edges are its
   springs' rest lengths, its corners within `drape.pin_top` of its top
   and within `drape.pin_band` of its neck's open edge are pinned where
   they are (a tee's neck band and shoulders; a pant's waistband), and
   gravity does the rest. It keeps its ease and hangs (a tee's hem falls
   from the chest and the shoulder blades, free below the seat) instead of
   being pushed onto the skin. With `drape.over_layers` (`true`, or a
   list of `fit.under` IDs), the underwear it is worn over collides too
   and is pushed out of first (`under_layers`): a top's hem then hangs over
   the briefs, not into the cleft they bridge (review fixes 2). With
   `drape.relax_edges`, its open edges
   are then relaxed along themselves (`relax_edges`). `drape.settings` are the cloth settings
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


# The bones whose skin a sleeve goes round (by part of their names): the
# arm's from the upper arm down, not the shoulder helper, whose weights
# reach over the shoulder's top almost to the neck (that is the torso's
# grade, by height).
GRADE_ARM_PARTS = ("UpperArm", "LowerArm", "ElbowHelper", "Hand")


def _armness(body, side):
    """How much each body corner is that side's arm, 0 to 1: its skin
    weights on the arm's bones (`GRADE_ARM_PARTS`), or `body.armness` (by
    side) where a test gives it."""
    given = getattr(body, "armness", None)
    if given is not None:
        return np.asarray(given.get(side, np.zeros(len(body.P))), float)
    arm_bones = np.array([b.startswith(side) and any(part in b for part in GRADE_ARM_PARTS) for b in body.bones])
    return (np.asarray(body.weights, float) * arm_bones[np.asarray(body.joints)]).sum(axis=1)


def _smoothstep(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3 - 2 * x)


def _sleeve_share(points, body, side, armness=None):
    """How much each point belongs to that side's sleeve, 0 to 1: the
    arm-ness of the body's skin nearest it (its 4 nearest corners, by
    inverse distance), eased from a quarter to three quarters. A sleeve
    is the arm's; the side below the armpit is the trunk's, as the body's
    own weights split them."""
    import bpy  # noqa: F401  (mathutils comes with the bpy module)
    from mathutils.kdtree import KDTree
    arm = _armness(body, side) if armness is None else armness
    tree = KDTree(len(body.P))
    for k, p in enumerate(body.P):
        tree.insert(tuple(p), k)
    tree.balance()
    out = np.zeros(len(points))
    for k, p in enumerate(np.asarray(points, float)):
        near = tree.find_n(tuple(p), 4)
        w = np.array([1.0 / max(d, 1e-6) for _, _, d in near])
        out[k] = float(np.dot(w, [arm[i] for _, i, _ in near]) / w.sum())
    return _smoothstep((out - 0.25) / 0.5)


def _neighbours(mesh):
    """Each corner's neighbours along the mesh's edges."""
    nb = [set() for _ in range(len(mesh.P))]
    for f in mesh.faces():
        for a, b in zip(f, np.roll(f, -1)):
            nb[int(a)].add(int(b))
            nb[int(b)].add(int(a))
    return [np.array(sorted(s), np.int64) for s in nb]


def _smooth(values, nb, rounds):
    """A field over the mesh (one value, or one vector, per corner) mixed
    `rounds` times half and half with its neighbours' mean."""
    out = np.asarray(values, float).copy()
    for _ in range(rounds):
        mean = np.array([out[n].mean(axis=0) if len(n) else out[k] for k, n in enumerate(nb)])
        out = 0.5 * out + 0.5 * mean
    return out


def _across(axis):
    """Two unit vectors across `axis`."""
    u = np.cross(axis, [0.0, 0.0, 1.0])
    if np.linalg.norm(u) < 1e-6:
        u = np.cross(axis, [1.0, 0.0, 0.0])
    u = u / np.linalg.norm(u)
    return np.stack([u, np.cross(axis, u)], axis=1)


def grade_sleeve(P, body, side, w, w_body, ease):
    """The sleeve on `side` graded round its arm: at stations every 2 cm
    along the upper arm where the sleeve closes round the arm (no gap over
    60 degrees about the arm's middle), the sleeve's girth across the arm
    against the arm's; where it has less than `ease`, its middle is moved
    onto the arm's (so it goes round the arm, not through it) and it is
    scaled round it until it has. Between stations the grade goes
    linearly; towards the shoulder, where the cap is the body's pieces
    wrapping the arm (an open ring), it fades back to none over 4 cm.
    Returns the graded points (all of them; only the sleeve's are used)
    and the report, or None without a closed sleeve."""
    heads = body.heads
    S, E = heads[f"{side}UpperArm"], heads[f"{side}LowerArm"]
    arm = (E - S) / np.linalg.norm(E - S)
    basis = _across(arm)
    full = w >= 0.99
    if full.sum() < 6:
        return None, None
    r = P - S
    t = r @ arm
    q = (r - np.outer(t, arm)) @ basis
    rb = body.P - S
    tb = rb @ arm
    qb = (rb - np.outer(tb, arm)) @ basis
    on_arm = (w_body >= 0.99) & (np.linalg.norm(qb, axis=1) < 0.15)
    rows = []
    lo, hi = t[full].min(), t[full].max()
    for t0 in np.linspace(lo, hi, int(np.ceil((hi - lo) / 0.02)) + 1):
        si = full & (np.abs(t - t0) <= 0.015)
        sb = on_arm & (np.abs(tb - t0) <= 0.015)
        if si.sum() < 4 or sb.sum() < 6:
            continue
        ca = (qb[sb].min(axis=0) + qb[sb].max(axis=0)) / 2
        angles = np.sort(np.arctan2(q[si, 1] - ca[1], q[si, 0] - ca[0]))
        gap = np.max(np.diff(np.r_[angles, angles[0] + 2 * np.pi]))
        if gap > np.radians(60):
            continue
        item, skin = _hull_perimeter(q[si]), _hull_perimeter(qb[sb])
        if item <= 0:
            continue
        scale = max(1.0, (skin + ease) / item)
        ci = (q[si].min(axis=0) + q[si].max(axis=0)) / 2
        rows.append((float(t0), scale, ci, ca, item, skin))
    if not rows:
        return None, None
    ts = [row[0] for row in rows]
    s = np.interp(t, ts, [row[1] for row in rows])
    # Moved onto the arm's middle only where it is graded (fully past 2%),
    # and fading out towards the shoulder before the first closed ring.
    k = (np.clip((s - 1.0) / 0.02, 0.0, 1.0) * _smoothstep((t - (ts[0] - 0.04)) / 0.04))[:, None]
    s = 1.0 + (s - 1.0) * k[:, 0]
    ci = np.stack([np.interp(t, ts, [row[2][j] for row in rows]) for j in (0, 1)], axis=1)
    ca = np.stack([np.interp(t, ts, [row[3][j] for row in rows]) for j in (0, 1)], axis=1)
    centre = ci + k * (ca - ci)
    graded = S + np.outer(t, arm) + (centre + (q - ci) * s[:, None]) @ basis.T
    report = {"least_ease_m": ease, "scale": round(max(row[1] for row in rows), 4),
              "stations": [{"along_m": round(row[0], 3), "item_m": round(row[4], 4), "arm_m": round(row[5], 4),
                            "scale": round(row[1], 4),
                            "moved_onto_arm_m": round(float(np.linalg.norm(row[3] - row[2])), 4)}
                           for row in rows]}
    return graded, report


def grade(mesh, body, spec):
    """Grades the item to the body by measured girths (see the module's
    note). The torso: across (X and Z, about the body's middle there) so it
    has at least its least ease of girth over the body's at every height in
    `at` (a height, or `[height, least ease]`; `least_ease` otherwise):
    each height's scale is what it needs (1 where it has the ease), linear
    between them, held below the lowest and above the highest (give a
    height at the neck to grade the neck and the shoulders' tops). With
    `sleeves` (`least_ease`) and the body's arm joints, each sleeve round
    its arm (`grade_sleeve`), blended into the torso's grade by its share
    (`_sleeve_share`, the body's own arm weights, smoothed over the item so
    the armhole doesn't tear). The girths are measured without the sleeves
    and the arms. Returns the girths and the scales."""
    least = float(spec.get("least_ease", 0.0))
    heads = getattr(body, "heads", None) or {}
    sleeves = spec.get("sleeves")
    sides = [side for side in ("Left", "Right")
             if sleeves is not None and f"{side}UpperArm" in heads and f"{side}LowerArm" in heads]
    share = np.zeros(len(mesh.P))
    share_body = np.zeros(len(body.P))
    shares = {}
    nb = _neighbours(mesh) if sides else None
    for side in sides:
        armness = _armness(body, side)
        w = _sleeve_share(mesh.P, body, side, armness)
        shares[side] = (w, _smoothstep((armness - 0.25) / 0.5))
        share = np.maximum(share, w)
        share_body = np.maximum(share_body, shares[side][1])
    torso, trunk = mesh.P[share < 0.5], body.P[share_body < 0.5]
    rows = []
    knots = []
    for entry in spec.get("at", []):
        y, ease = (entry[0], float(entry[1])) if isinstance(entry, (list, tuple)) else (entry, least)
        sel = np.abs(torso[:, 1] - y) <= 0.01
        if not sel.any():
            continue
        # The body across the item's width there, and a little past it (an
        # item too small for the body is narrower than it).
        lo, hi = torso[sel, 0].min() - 0.03, torso[sel, 0].max() + 0.03
        item = girth(torso, y)
        skin = girth(trunk, y, x_range=(lo, hi))
        scale = max(1.0, (skin + ease) / item) if item > 0 else 1.0
        body_sel = (np.abs(trunk[:, 1] - y) <= 0.01) & (trunk[:, 0] >= lo) & (trunk[:, 0] <= hi)
        around = trunk[body_sel] if body_sel.any() else torso[sel]
        centre = (around.min(axis=0) + around.max(axis=0)) / 2
        knots.append((y, scale, centre))
        rows.append({"at_m": y, "least_ease_m": ease, "item_m": round(item, 4), "body_m": round(skin, 4),
                     "ease_m": round(item - skin, 4), "scale": round(scale, 4)})
    most = max([k[1] for k in knots], default=1.0)
    graded = mesh.P.copy()
    if most > 1.0:
        knots.sort(key=lambda k: k[0])
        ys = [k[0] for k in knots]
        y = mesh.P[:, 1]
        # np.interp holds the end values past the first and last heights.
        s = np.interp(y, ys, [k[1] for k in knots])
        cx = np.interp(y, ys, [k[2][0] for k in knots])
        cz = np.interp(y, ys, [k[2][2] for k in knots])
        graded[:, 0] = cx + (mesh.P[:, 0] - cx) * s
        graded[:, 2] = cz + (mesh.P[:, 2] - cz) * s
    report = {"least_ease_m": least, "girths": rows, "scale": round(most, 4)}
    # The sleeves are graded from where the torso's grade put them, so a
    # corner the sleeve's grade leaves alone (past its fade) keeps the
    # torso's.
    torso_graded = graded.copy()
    for side in sides:
        w, w_body = shares[side]
        sleeve, sleeve_report = grade_sleeve(torso_graded, body, side, w, w_body,
                                             float(sleeves.get("least_ease", 0.04)))
        if sleeve is None:
            continue
        # The share smoothed over the item (8 rounds), so the grade moves
        # from the torso's to the sleeve's over a few rows, not one edge.
        blend = _smooth(w, nb, 8)[:, None]
        graded = graded + blend * (sleeve - graded)
        report.setdefault("sleeves", {})[side] = sleeve_report
    mesh.P = graded
    torso_after = mesh.P[share < 0.5]
    for row in rows:
        row["item_after_m"] = round(girth(torso_after, row["at_m"]), 4)
    return report


def push_out(mesh, body, clearance, rounds=0):
    """Moves every corner of the item inside the body, or nearer it than
    `clearance`, out along the body's normal at its nearest point, so the
    cloth simulation starts outside the body; returns how many and how
    far. With `rounds`, the moves are a smooth field over the cloth
    instead: smoothed `rounds` times with the neighbours' (`_smooth`),
    each pushed corner kept at least as far out as it needs after each
    round, so a patch moves out together and carries its neighbours,
    rather than one row of corners stepping out from the next (the notched
    neckline of review fixes 1)."""
    import checks
    tree = checks._tree(body.P, _triangles(body))
    need = np.zeros(len(mesh.P))
    normals = np.zeros((len(mesh.P), 3))
    for k, p in enumerate(mesh.P):
        loc, normal, _, _ = tree.find_nearest(tuple(p))
        if loc is None:
            continue
        n = np.array(normal)
        off = float(np.dot(p - np.array(loc), n))
        if off < clearance:
            need[k] = clearance - off
            normals[k] = n
    moves = normals * need[:, None]
    if rounds:
        nb = _neighbours(mesh)
        for _ in range(rounds):
            moves = _smooth(moves, nb, 1)
            short = need - np.einsum("ij,ij->i", moves, normals)
            moves += normals * np.maximum(short, 0.0)[:, None]
    mesh.P = mesh.P + moves
    moved = np.linalg.norm(moves, axis=1)
    report = {"pushed": int((need > 0).sum()), "most_m": round(float(need.max(initial=0.0)), 4)}
    if rounds:
        report["rounds"] = rounds
        report["carried"] = int(((need <= 0) & (moved > 0.001)).sum())
    return report


def relax_edges(mesh, rounds):
    """Relaxes the item's open edges along themselves: each corner of an
    open loop moved `rounds` times towards the middle of its two neighbours
    on the loop (Taubin's two steps); nothing else moves. The cloth simulation
    leaves a free hem (a sleeve's end) in a zig-zag where its corners
    caught on the body and on each other (review fixes 1: 10 to 15 mm off
    their neighbours' line); a hem lies in a smooth line. Returns how many
    corners, and how far the worst was off its neighbours' middle before
    and after (mm)."""
    from refine import boundary_loops
    loops = [np.array(lp) for lp in boundary_loops(mesh) if len(lp) >= 4]

    def worst(P):
        return max((float(np.linalg.norm(P[lp] - (P[np.roll(lp, 1)] + P[np.roll(lp, -1)]) / 2, axis=1).max())
                    for lp in loops), default=0.0) * 1000

    P = mesh.P.copy()
    before = worst(P)
    # Taubin's smoothing: a step towards the neighbours' middle, then a
    # slightly larger one away, so a zig-zag goes but a round loop (a
    # sleeve's end) isn't drawn in as plain averaging would draw it.
    for _ in range(rounds):
        for step in (0.5, -0.53):
            for lp in loops:
                mid = (P[np.roll(lp, 1)] + P[np.roll(lp, -1)]) / 2
                P[lp] = P[lp] + step * (mid - P[lp])
    mesh.P = P
    return {"corners": int(sum(len(lp) for lp in loops)), "rounds": rounds,
            "most_before_mm": round(before, 1), "most_after_mm": round(worst(P), 1)}


class _Surface:
    """Corners and triangles (a collider: the body and the layers under)."""

    def __init__(self, P, tris):
        self.P, self.tris = P, tris


def _with_layers(body, under):
    """The body and the layers the item is worn over as one surface."""
    Ps, tris, offset = [], [], 0
    for part in [body, *under]:
        Ps.append(np.asarray(part.P, float))
        tris.append(_triangles(part) + offset)
        offset += len(part.P)
    return _Surface(np.vstack(Ps), np.vstack(tris))


def under_layers(spec, only=None):
    """The layers an item is worn over (`fit.under`, item ids, or those of
    them in `only`) as they sit on the default body at rest: each item's
    model object read from its GLB (`../items/<name>.item.json`), so the
    drape hangs the cloth over them, not into the hollows they bridge
    (review fixes 2)."""
    import bodyglb
    items = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "items")
    layers = []
    for item_id in spec.get("fit", {}).get("under", []):
        if only is not None and item_id not in only:
            continue
        with open(os.path.join(items, item_id.split("/")[-1] + ".item.json"), encoding="utf-8") as f:
            model = json.load(f)["model"]
        doc, binary = bodyglb.read_glb(os.path.normpath(os.path.join(items, model["file"])))
        name = model["object"].split("/")[-1]
        node = next(n for n in doc["nodes"] if n.get("name") == name and "mesh" in n)
        Ps, tris, offset = [], [], 0
        for prim in doc["meshes"][node["mesh"]]["primitives"]:
            P = bodyglb.accessor(doc, binary, prim["attributes"]["POSITION"])
            Ps.append(P)
            tris.append(bodyglb.accessor(doc, binary, prim["indices"]).reshape(-1, 3) + offset)
            offset += len(P)
        layers.append(_Surface(np.vstack(Ps), np.vstack(tris).astype(np.int64)))
    return layers


def drape(mesh, body, spec, under=()):
    """Hangs the item on the body by Blender's cloth simulation (see the
    module's note); returns what it did. `under`: the layers it is worn
    over (surfaces with `P` and `tris`, at rest on the same body), which
    collide with it and which it is first pushed out of, like the body."""
    import bpy
    sys.path.append(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "clothing"))
    import drape as cloth  # the realistic wardrobe's cloth simulation
    import checks

    def to_blender(P):
        return np.stack([P[:, 0], -P[:, 2], P[:, 1]], axis=1)

    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    skin = body
    tris = _triangles(body)
    if under:
        body = _with_layers(body, under)
    me = bpy.data.meshes.new("Body")
    me.from_pydata(to_blender(body.P).tolist(), [], _triangles(body).tolist())
    me.update()
    collider = bpy.data.objects.new("Body", me)
    scene.collection.objects.link(collider)
    collider.modifiers.new("Collision", "COLLISION")
    collider.collision.thickness_outer = 0.002
    collider.collision.thickness_inner = 0.02
    collider.collision.damping = 0.5
    collider.collision.cloth_friction = 5.0
    pushed = push_out(mesh, body, float(spec.get("start_clearance", 0.004)), int(spec.get("smooth_push", 0)))
    top = mesh.P[:, 1].max()
    held = mesh.P[:, 1] >= top - float(spec.get("pin_top", 0.05))
    band = float(spec.get("pin_band", 0.0))
    if band > 0:
        # The whole neck band held where the grade put it: every corner
        # within `pin_band` of the highest open loop (a crew neck's front
        # is lower than its back, below `pin_top`). Left free, its front
        # crumpled into notches against the body (review fixes 1).
        from refine import pick_loop
        loop = mesh.P[pick_loop(mesh, "highest")]
        near = np.min(np.linalg.norm(mesh.P[:, None, :] - loop[None, :, :], axis=2), axis=1)
        held |= near <= band
    pins = np.where(held)[0]
    settings = cloth.Settings(**spec.get("settings", {}))
    before = mesh.P.copy()
    out = cloth.simulate(to_blender(mesh.P), [list(map(int, f)) for f in mesh.faces()], [],
                         settings, pins=pins, name="Item")
    mesh.P = np.stack([out[:, 0], out[:, 2], -out[:, 1]], axis=1)
    relaxed = relax_edges(mesh, int(spec["relax_edges"])) if spec.get("relax_edges") else None
    moved = np.linalg.norm(mesh.P - before, axis=1)
    d = checks.distances(mesh.P, skin.P, tris, signed=True)
    # Over the layers too: corners left between a layer and the skin.
    inside_layers = int((checks.distances(mesh.P, body.P, _triangles(body), signed=True) < 0).sum())         if under else 0
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
        **({"edges_relaxed": relaxed} if relaxed else {}),
        "pinned": int(len(pins)),
        "frames": settings.frames,
        "moved_m": {"most": round(float(moved.max()), 4), "median": round(float(np.median(moved)), 4)},
        "clearance_m": {"least": round(float(d.min()), 4), "median": round(float(np.median(d)), 4),
                        "inside_body": int((d < 0).sum()),
                        **({"inside_layers": inside_layers} if under else {})},
        **({"over_layers": len(under)} if under else {}),
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
        # `drape.over_layers`: hang it over the items it is worn over
        # (`fit.under`: all of them with `true`, or those listed) as well
        # as the body.
        over = options["drape"].get("over_layers")
        under = under_layers(spec, over if isinstance(over, list) else None) if over else ()
        report["drape"] = drape(mesh, body, options["drape"], under=under)
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
