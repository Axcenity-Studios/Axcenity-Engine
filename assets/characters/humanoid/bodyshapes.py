"""The body's own blend shapes: the feminine frame, the build shapes and
the clothing masks, each a {vertex: delta} field over the body.

Masks: a clothing item names the masks it needs (`MASKS`); at weight 1 a
mask pulls the covered skin about a centimetre inside the clothes, so it
can't poke through, and feathers out where the weights of the next bone
take over, so the visible edge stays smooth. The body stays one seamless
mesh at every level of detail.
"""

import math

from mathutils import Vector

from common import (
    FS_FACE, FS_FOREARM, FS_JAW_SIDE, FS_PELVIS, FS_SHIN, FS_SHOULDERS, FS_THIGH, FS_TORSO,
    FS_UNDER_JAW, FS_UPPER_ARM, SIDES, smooth_field, smoothstep, v,
)

# Mask name -> (bones, part of the bone's length (from, to) or None).
MASKS = {
    "Mask_Torso": (("Spine", "Chest", "UpperChest", "LeftShoulder", "RightShoulder"), None),
    "Mask_Hips": (("Hips",), None),
    "Mask_UpperArms_Top": (("LeftUpperArm", "RightUpperArm"), (0.0, 0.5)),
    "Mask_UpperArms_Low": (("LeftUpperArm", "RightUpperArm"), (0.5, 1.0)),
    "Mask_Forearms": (("LeftLowerArm", "RightLowerArm"), None),
    "Mask_Thighs_Top": (("LeftUpperLeg", "RightUpperLeg"), (0.0, 0.5)),
    "Mask_Thighs_Low": (("LeftUpperLeg", "RightUpperLeg"), (0.5, 1.0)),
    "Mask_Shins_Top": (("LeftLowerLeg", "RightLowerLeg"), (0.0, 0.6)),
    "Mask_Shins_Low": (("LeftLowerLeg", "RightLowerLeg"), (0.6, 1.0)),
    "Mask_Feet": (("LeftFoot", "RightFoot", "LeftToes", "RightToes"), None),
}

MASK_DEPTH = 0.011


def bone_param(p, head, tail):
    d = tail - head
    return (p - head).dot(d) / d.length_squared


def masks(P, normals, weights, joints):
    """Mask name -> field. `weights[i]` is {bone: weight} of vertex i."""
    out = {}
    for name, (bones, part) in MASKS.items():
        field = {}
        depth = MASK_DEPTH * (0.7 if name == "Mask_Feet" else 1.0)
        for i, p in enumerate(P):
            m = 0.0
            for b in bones:
                w = weights[i].get(b, 0.0)
                if w <= 0.0:
                    continue
                if part is not None:
                    head, tail, _ = joints[b]
                    t = bone_param(p, head, tail)
                    lo, hi = part
                    edge_lo = 1.0 if lo <= 0.0 else smoothstep(lo - 0.08, lo + 0.08, t)
                    edge_hi = 1.0 if hi >= 1.0 else 1.0 - smoothstep(hi - 0.08, hi + 0.08, t)
                    w *= edge_lo * edge_hi
                m += w
            if m > 1e-3:
                field[i] = -normals[i] * depth * min(1.0, m)
        out[name] = field
    return out


def frame_feminine(M, F):
    """The feminine frame: the feminine base body's positions over the
    masculine one's (the same topology)."""
    return {i: f - m for i, (m, f) in enumerate(zip(M, F)) if (f - m).length_squared > 1e-12}


def _amount_field(P, lm, nbr, per_set, bumps, smooth=6):
    """A per-vertex amount: a value per face set, plus gaussian bumps
    (centre, radius, amount, front-only), smoothed over the mesh."""
    amount = [0.0] * len(P)
    for s, a in per_set.items():
        for i in lm.verts(s):
            amount[i] = a
    amount = smooth_field(amount, nbr, smooth)
    for i, p in enumerate(P):
        for c, r, a, facing in bumps:
            d2 = (p - c).length_squared
            if d2 > 9 * r * r:
                continue
            g = math.exp(-d2 / (2 * r * r))
            amount[i] += a * g * (facing(i) if facing else 1.0)
    return amount


def builds(P, lm, normals, nbr, joints):
    """Build_Heavy, Build_Slim and Build_Muscular."""
    J = {k: (h, t) for k, (h, t, _) in joints.items()}
    front = lambda i: smoothstep(0.1, 0.6, -normals[i].y)
    back = lambda i: smoothstep(0.1, 0.6, normals[i].y)
    waist = J["Spine"][0]
    chest = J["Chest"][1]
    heavy_bumps = [
        (waist + v(0, -0.09, 0.02), 0.09, 0.035, front),       # belly
        (waist + v(0, -0.07, -0.08), 0.07, 0.015, front),      # lower belly
        (chest + v(0, -0.06, -0.06), 0.08, 0.010, front),      # chest
        (J["Neck"][0] + v(0, -0.05, 0.13), 0.035, 0.012, None),  # under the chin
    ]
    for _, s in SIDES:
        heavy_bumps.append((waist + v(s * 0.13, 0.0, -0.02), 0.06, 0.018, None))  # flanks
        heavy_bumps.append((J["Hips"][0] + v(s * 0.1, 0.06, -0.08), 0.07, 0.014, back))  # seat
    heavy = _amount_field(P, lm, nbr, {
        FS_TORSO: 0.012, FS_PELVIS: 0.014, FS_SHOULDERS: 0.008, FS_UPPER_ARM: 0.011,
        FS_FOREARM: 0.006, FS_THIGH: 0.018, FS_SHIN: 0.009, FS_FACE: 0.003, FS_JAW_SIDE: 0.006,
        FS_UNDER_JAW: 0.008,
    }, heavy_bumps)
    slim = _amount_field(P, lm, nbr, {
        FS_TORSO: -0.008, FS_PELVIS: -0.008, FS_SHOULDERS: -0.004, FS_UPPER_ARM: -0.005,
        FS_FOREARM: -0.003, FS_THIGH: -0.009, FS_SHIN: -0.004, FS_JAW_SIDE: -0.002,
    }, [(waist + v(0, -0.08, 0.0), 0.08, -0.012, front)])
    muscle_bumps = []
    for side, s in SIDES:
        sh, el = J[f"{side}UpperArm"]
        wr = J[f"{side}LowerArm"][1]
        hip, knee = J[f"{side}UpperLeg"]
        ankle = J[f"{side}LowerLeg"][1]
        lat = v(s, 0, 0)
        muscle_bumps += [
            (sh + lat * 0.035 + v(0, 0, 0.0), 0.045, 0.014, None),           # deltoid
            (sh.lerp(el, 0.5) + v(0, -0.03, 0), 0.04, 0.012, None),           # biceps
            (sh.lerp(el, 0.5) + v(0, 0.03, 0), 0.04, 0.010, None),            # triceps
            (el.lerp(wr, 0.25), 0.04, 0.008, None),                           # forearm
            (chest + v(s * 0.075, -0.07, -0.02), 0.06, 0.016, front),         # pectoral
            (J["Neck"][0] + v(s * 0.07, 0.03, -0.01), 0.045, 0.012, None),    # trapezius
            (chest + v(s * 0.11, 0.05, -0.06), 0.07, 0.012, back),            # lats
            (hip.lerp(knee, 0.45) + v(0, -0.04, 0), 0.07, 0.014, front),      # quadriceps
            (knee.lerp(ankle, 0.3) + v(0, 0.04, 0), 0.05, 0.013, back),       # calf
            (J["Hips"][0] + v(s * 0.08, 0.08, -0.07), 0.06, 0.010, back),     # glutes
        ]
    muscle_bumps.append((waist + v(0, -0.1, 0.06), 0.06, 0.006, front))       # abdominals
    muscle = _amount_field(P, lm, nbr, {}, muscle_bumps, smooth=0)
    out = {}
    for name, amount in (("Build_Heavy", heavy), ("Build_Slim", slim), ("Build_Muscular", muscle)):
        out[name] = {i: normals[i] * a for i, a in enumerate(amount) if abs(a) > 1e-5}
    return out


def joint_offsets(joints_from, joints_to, scale=1.0):
    """Bone name -> {"offset": [x, y, z]}: how far each bone's head moves
    between two fittings (the skeleton following a body shape)."""
    out = {}
    for name, (head, _, _) in joints_from.items():
        d = (joints_to[name][0] - head) * scale
        if d.length > 1e-4:
            out[name] = {"offset": [round(d.x, 5), round(d.y, 5), round(d.z, 5)]}
    return out


def _transfer():
    """The realistic pipeline's mesh helpers (`../transfer/`: `geom`,
    `transfer`) and numpy."""
    import os
    import sys

    import numpy as np
    here = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "transfer")
    if here not in sys.path:
        sys.path.insert(0, here)
    import geom
    import transfer
    return geom, transfer, np


# Narrower hips (review fixes 1 of R3 wave 3; plan section 0 decision 8,
# "hips go both ways"), the realistic body's mechanism
# (`../realistic/shapes.py`, `hips_narrow`). `anim`'s `HipWidth` below 1
# pulls the legs' roots in with the pelvis and the heavy inner thighs cross
# at the crotch, so below 1 no bone moves (the manifest's attribute driver
# starts at 1) and `<Frame>_Hips_Narrow` takes the surface in: every vertex
# of the pelvis and the thighs' tops brought toward the middle by
# `1 - NARROW_SCALE` of its distance from it, the inner thighs held (they
# are near the middle, and are let go only past `NARROW_HELD`), fading out
# above the hip crest and down the thigh. It is whole at `hip_width`
# `manifests.NARROW_AT` (0.6); `NARROW_SCALE` makes the hips' breadth fall
# below 1 about as fast as `HipWidth` widens them above it (0.52 to 0.57 of
# the slider's change on this body), so the slider has no kink at 1.
NARROW_SCALE = 0.78
NARROW_HELD = (0.012, 0.07)   # metres from the middle: held, then wholly narrowed
# Where it is whole and where it fades, in metres from the hip joints'
# height (up: the hip crest, then the waist; down: the crotch, then mid-thigh).
NARROW_UP = (0.08, 0.24)
NARROW_DOWN = (0.1, 0.25)
# The bones whose skin narrows (a hand hanging by the hip doesn't).
NARROW_BONES = ("Hips", "Spine", "Chest")
NARROW_SIDE_BONES = ("UpperLeg", "LowerLeg")
NARROW_SHAPES = {"masculine": "Masculine_Hips_Narrow", "feminine": "Feminine_Hips_Narrow"}


def _smoothstep(np, x, a, b):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def hips_narrow(P, F, joints, joints_f, weights, edges):
    """{`Masculine_Hips_Narrow`, `Feminine_Hips_Narrow`}: each frame's
    narrowing over its own rest positions (`P`, `F`; the feminine one is
    driven with the frame, so it adds to `Frame_Feminine`). Side to side
    only, toward the middle (x = 0), so the two sides can't cross, by a
    share of each vertex's distance from it that is smooth over the skin
    and never shrinks a step of the skin by more than it narrows the whole
    (the map x -> x + dx keeps its order). Mirrored to be symmetric."""
    geom, transfer, np = _transfer()
    n = len(P)
    lap = geom.Laplacian(n, np.array(edges))
    frames = {"masculine": (np.array([tuple(p) for p in P]), joints),
              "feminine": (np.array([tuple(p) for p in F]), joints_f)}
    mirror = geom.mirror_map([X for X, _ in frames.values()])
    names = list(NARROW_BONES) + [side + b for b in NARROW_SIDE_BONES for side, _ in SIDES]
    own = np.clip(np.array([sum(w.get(b, 0.0) for b in names) for w in weights]), 0.0, 1.0)
    out = {}
    for label, (X, J) in frames.items():
        k = (X[:, 2].max() - X[:, 2].min()) / 1.75
        z = X[:, 2] - J["LeftUpperLeg"][0].z
        up = 1.0 - _smoothstep(np, z, NARROW_UP[0] * k, NARROW_UP[1] * k)
        down = _smoothstep(np, z, -NARROW_DOWN[1] * k, -NARROW_DOWN[0] * k)
        across = _smoothstep(np, np.abs(X[:, 0]), NARROW_HELD[0] * k, NARROW_HELD[1] * k)
        mask = (up * down * own)[:, None]
        for _ in range(4):
            mask = (mask + lap.mean(mask)) / 2
        d = np.zeros_like(X)
        d[:, 0] = (NARROW_SCALE - 1.0) * X[:, 0] * across * mask[:, 0]
        d = transfer.symmetric(d, mirror)
        out[NARROW_SHAPES[label]] = {i: Vector(d[i]) for i in range(n) if np.abs(d[i]).max() > 1e-7}
    return out


# The build shapes' corners the body is kept apart in (R3, wave 3: the
# stylized family held to the realistic body's checks), on both frames:
# (name, {shape: weight}). The manifest drives `Build_Heavy` with `build`
# 0 to 1, `Build_Slim` with `build` 0 to -1 and `Build_Muscular` with
# `muscle` 0 to 1, so these are the corners a character can reach.
APART = [("heavy", {"Build_Heavy": 1.0}), ("muscular", {"Build_Muscular": 1.0}),
         ("heavy and muscular", {"Build_Heavy": 1.0, "Build_Muscular": 1.0})]
# The shapes are kept apart as if this much stronger, so the proportion
# sliders' small moves of the bones don't bring the crossings back.
APART_MARGIN = 1.25
# Review fixes 1: the same corners, and the slim ones, kept apart with the
# head scaled to these (`anim`'s `HeadSize` scales the Head bone and its
# children, the jaw and the eyes, about the head's joint): the heavy
# build's double chin meets the neck as the head grows or shrinks, and the
# slim feminine jaw folds into itself as it grows. Just past the
# population's ends (0.89 and 1.11 of this body's head), so the shipped
# range can reach them.
APART_HEAD = (0.88, 1.12)
APART_HEAD_CORNERS = APART + [("slim", {"Build_Slim": 1.0}),
                              ("slim and muscular", {"Build_Slim": 1.0, "Build_Muscular": 1.0})]
# The bones `HeadSize` scales (the Head and its children).
HEAD_BONES = ("Head", "Jaw", "LeftEye", "RightEye")


def _apart(transfer, np, refs, B, d, tris, lap, mirror, known, rounds=60, step=0.8):
    """`transfer.apart` on a reshaped body `B` (a frame with the head
    scaled or the hips narrowed): the per-vertex factor that scales `d`
    down, smoothly, round the places where `B + d` crosses itself (pairs
    not in `known`) and the pair's triangles don't touch (within 4 mm) in
    one of `refs`: the frame's neutral body, as `measure.py`'s sliders
    pass counts them, and the same build at rest, as `ranges.py` does.
    Returns the factor and the crossings left."""
    factor = np.ones(len(B))
    left = set()
    for _ in range(rounds):
        left = set()
        for a, b in transfer.crossing_pairs(B + d * factor[:, None], tris) - known:
            if any(np.linalg.norm(r[tris[a]][:, None] - r[tris[b]][None], axis=2).min() > 0.004 for r in refs):
                left.add((a, b))
        if not left:
            break
        hit = np.unique(tris[np.array(sorted({t for pair in left for t in pair}))])
        g = np.ones((len(B), 1))
        g[hit] = step
        # Spread the cut over the neighbours so it leaves no dent.
        for _ in range(3):
            g = np.minimum(g, (g + lap.mean(g)) / 2)
        g = np.minimum(g[:, 0], g[mirror, 0])
        factor *= g
    return factor, len(left)


def keep_apart(P, fields, tris, edges, weights=None, heads=None, log=print):
    """`Build_Heavy`, `Build_Muscular` and `Build_Slim` scaled down,
    smoothly, round the places where they make the body cross itself (the
    inner thighs, the chin into the neck, the upper arm's folds), on both
    frames, as the realistic body's shapes are (`../transfer/transfer.py`,
    `apart`): in the rest pose; with the hips narrowed (`fields`'
    `<Frame>_Hips_Narrow`, whole); and with the head scaled to
    `APART_HEAD`'s ends (`weights`: each vertex's {bone: weight}; `heads`:
    {"masculine", "feminine"} -> the Head bone's joint). Returns the
    fields and a report. Elsewhere they are unchanged."""
    geom, transfer, np = _transfer()
    n = len(P)
    dense = lambda field: np.array([tuple(field.get(i, (0.0, 0.0, 0.0))) for i in range(n)])  # noqa: E731
    X = np.array([tuple(p) for p in P])
    frames = {"masculine": X, "feminine": X + dense(fields["Frame_Feminine"])}
    tris = np.array(tris)
    lap = geom.Laplacian(n, np.array(edges))
    mirror = geom.mirror_map(list(frames.values()))
    shape = {name: dense(fields[name]) for name in ("Build_Heavy", "Build_Muscular", "Build_Slim")}
    factor = {name: np.ones(n) for name in shape}
    # The skin `HeadSize` carries: a vertex moves by (p - head) (s - 1) w,
    # and a shape's offset on it grows by 1 + (s - 1) w (linear skinning).
    w_head = np.zeros(n) if weights is None else np.array(
        [min(1.0, sum(w.get(b, 0.0) for b in HEAD_BONES)) for w in weights])
    report = {}
    for label, B in frames.items():
        known = transfer.crossing_pairs(B, tris)
        # (where, the body, each vertex's offset scale, corners)
        settings = [("rest", B, None, APART)]
        narrow = NARROW_SHAPES[label]
        if narrow in fields:
            settings.append(("hips narrowed", B + dense(fields[narrow]), None, APART))
        if heads is not None:
            h = np.array(tuple(heads[label]))
            for s in APART_HEAD:
                grow = (s - 1.0) * w_head
                settings.append((f"head {s:g}", B + (B - h) * grow[:, None], 1.0 + grow, APART_HEAD_CORNERS))
        for where, body, carry, corners in settings:
            seen = known | transfer.crossing_pairs(body, tris)
            for corner, mix in corners:
                at_rest = sum(shape[name] * factor[name][:, None] * w for name, w in mix.items())
                d = APART_MARGIN * at_rest
                if carry is not None:
                    d = d * carry[:, None]
                refs = (B,) if where == "rest" else (B, B + at_rest)
                f, left = _apart(transfer, np, refs, body, d, tris, lap, mirror, seen)
                for name in mix:
                    factor[name] *= f
                cut = int((f < 0.999).sum())
                key = f"{label} {corner}" + ("" if where == "rest" else f", {where}")
                report[key] = {"left": left, "vertices_cut": cut, "least_factor": round(float(f.min()), 3)}
                if cut or left or where == "rest":
                    log(f"  kept apart, {key}: {left} left, {cut} vertices cut (to {f.min():.2f} at most)")
    out = {}
    for name, d in shape.items():
        d = d * factor[name][:, None]
        out[name] = {i: Vector(d[i]) for i in range(n) if np.abs(d[i]).max() > 1e-7}
    return out, report
