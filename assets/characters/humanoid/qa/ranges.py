"""Finds how far each proportion slider of the realistic body can go (R3,
plan section 0 decision 8): each end is the last step that still deforms
cleanly or the population's 0.5th / 99.5th percentile, whichever is
nearer to 1.

    python -I ranges.py [--package realistic] [--out ranges.json] [--only hip_width,height]
                        [--step 0.025] [--hips-by-bone]

For every proportion (`anim`'s nine body sliders), on both frames, the
value is stepped outward from 1 in both directions. At each step five
bodies per frame are checked: the frame's own body and the four corners
of `muscle` and `fat` at their ends. A step fails when, on any of them:

- **Crossings**: the skin newly crosses itself in the rest pose (the
  sliders pass's method, `measure.slider_pairs`: pairs of triangles that
  cross where neither the frame's neutral body nor the same body at 1
  crosses, and that don't already lie within 4 mm of touching there).
- **Joints**: in the deformation poses the slider bears on (`POSES`), a
  joint's ring area ratio, or its local volume ratio where the package
  holds one (posed over rest, as `measure.deformation` measures them),
  leaves the package's limit (`qa/thresholds.json`). Where the package
  has no limit for that pose, or the body at 1 is already past it, it
  may not fall below `KEEP` of what the same body holds at 1 (or of its
  rest size, where it swells); a capped ratio (the lowered shoulder's
  bulge) may rise `BULGE` past the body's own at 1 (`joint_ceiling`).
- **Eyes and teeth**: skin in an eyeball, an eyeball's front in the lids'
  flesh or a tooth in the lips' or gums' flesh (`measure.eye_metrics`; on
  the frame's own body).
- **Collapse**: the narrowest breadth of a cross-section of the trunk or
  a limb (`SECTIONS`), or a region's skin area (`VOLUMES`), falls below
  `COLLAPSE` of the same body's at 1.
- **A step in the outline** (brief §4.4, "no abrupt deformation
  transitions"): the trunk's breadth, at `SLABS` + 1 levels from below
  the hips to above the chest, changes against the same body's at 1 by
  more than `OUTLINE` more at one level than at the next.

The population ends come from ANSUR II (docs/development/character-
studio/references/body/overall/README.md): each slider has a dimension
measured on the built body at every step (`dimension`), and that
dimension's mean and SD per sex (`POPULATION`). The 0.5th and 99.5th
percentiles are mean -/+ 2.576 SD on a normal curve. Where the body's own
dimension is comparable with the survey's (stature, hip joint height
against trochanter height, hand and foot length), the bound is where the
body's own value reaches it; where it isn't (joint centres against bony
landmarks), the body is taken to sit at the mean and the bound is the
relative spread (`ANCHOR`).

Body sliders are applied as `anim::BodyShape::apply` applies them, to the
pose (`shaped`, which keeps each bone's turn from the QA pose and checks
itself against `Character.set_body_sliders` at rest). `hip_width` below 1
moves no bone (the manifest's attribute driver starts at 1): the hips'
narrowing shape does it, by the manifest's corner. `--hips-by-bone` sweeps
the old way, `HipWidth` below 1, for the record.
"""

import argparse
import json
import math
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import measure as M  # noqa: E402
import qa_scene as Q  # noqa: E402

THRESHOLDS = os.path.join(Q.LIBRARY_DIR, "realistic", "qa", "thresholds.json")

# The sliders: (parameter, attribute).
SLIDERS = [("height", "BodyHeight"), ("leg_length", "LegLength"), ("arm_length", "ArmLength"),
           ("shoulder_width", "ShoulderWidth"), ("waist_width", "WaistWidth"), ("hip_width", "HipWidth"),
           ("head_size", "HeadSize"), ("hand_size", "HandSize"), ("foot_size", "FootSize")]
# `anim`'s clamp (crates/anim/src/body_shape.rs, RANGE): the sweep never goes past it.
BOUND = (0.5, 1.5)
COMPOSITIONS = [("own body", {}), ("slight", {"muscle": -1.0, "fat": -1.0}),
                ("lean and muscular", {"muscle": 1.0, "fat": -1.0}),
                ("heavy and soft", {"muscle": -1.0, "fat": 1.0}),
                ("heavy and muscular", {"muscle": 1.0, "fat": 1.0})]
# The QA poses each slider bears on (`Q.POSES`): the joints whose bones it
# moves or scales. Head, hand (beyond the wrist) and foot sizes scale a
# whole part and move no joint of the trunk or limbs.
POSES = {
    "height": ["knee_135", "hip_flex_100", "spine_twist_45", "shoulder_abd_90", "shoulder_abd_150"],
    "leg_length": ["knee_135", "hip_flex_100"],
    "arm_length": ["elbow_135", "wrist_70", "shoulder_abd_10", "shoulder_abd_90", "shoulder_abd_150"],
    "shoulder_width": ["shoulder_abd_10", "shoulder_abd_90", "shoulder_abd_150"],
    "waist_width": ["spine_twist_45", "hip_flex_100"],
    "hip_width": ["hip_flex_100", "knee_135"],
    "head_size": [],
    "hand_size": ["wrist_70"],
    "foot_size": [],
}
KEEP = 0.9        # a joint without a package limit keeps this share of its ratio at 1
COLLAPSE = 0.7    # a cross-section breadth or region skin area keeps this share of its size at 1
OUTLINE = 0.05    # the most the trunk's breadth ratio may change from one level to the next
SLABS = 16        # those levels, over (shares of the hip joints' to the chest joint's height, from the hips):
OUTLINE_SPAN = (-0.7, 1.45)  # about 15 cm below the hips to 10 cm above the chest, 3 cm apart
BULGE = 0.01      # what a step may add to a capped ratio (the lowered shoulder's) past its body's own
TOUCHING = 0.004  # pairs this close in the body at 1 aren't counted (the sliders pass's 4 mm)
Z99 = 2.576       # the 99.5th percentile of a normal curve, in SD

# ANSUR II (references/body/overall/README.md), men and women: mean and SD
# of the dimension `dimension` measures (stature in metres, the rest over
# stature).
POPULATION = {
    "height": {"masculine": (1.756, 0.069), "feminine": (1.628, 0.064), "what": "stature"},
    "leg_length": {"masculine": (0.513, 0.015), "feminine": (0.519, 0.014),
                   "what": "trochanter height over stature (the hip joint's height over stature here)"},
    "arm_length": {"masculine": (0.343, 0.010), "feminine": (0.339, 0.011),
                   "what": "upper arm and forearm over stature (joint to joint here)"},
    "shoulder_width": {"masculine": (0.237, 0.010), "feminine": (0.224, 0.010),
                       "what": "biacromial breadth over stature (shoulder joint width here)"},
    # The waist slider changes the waist and nothing else, so its
    # dimension is the waist against the hips (not against stature, which
    # varies three times as much and would let it reach a wasp waist).
    "waist_width": {"masculine": (0.94, 0.06), "feminine": (0.85, 0.06),
                    "what": "waist breadth over hip breadth (at the spine's middle here)"},
    "hip_width": {"masculine": (0.197, 0.013), "feminine": (0.217, 0.015), "what": "hip breadth over stature"},
    "head_size": {"masculine": (0.327, 0.013), "feminine": (0.345, 0.015),
                  "what": "head circumference over stature (head breadth here, which scales alike)"},
    "hand_size": {"masculine": (0.1101, 0.0042), "feminine": (0.1112, 0.0047), "what": "hand length over stature"},
    "foot_size": {"masculine": (0.1544, 0.0052), "feminine": (0.1513, 0.0053), "what": "foot length over stature"},
}
# "own": the body's own value is compared with the survey's. "mean": the
# landmarks differ, so the body is taken to sit at the mean and only the
# relative spread is used.
ANCHOR = {"height": "own", "leg_length": "own", "hand_size": "own", "foot_size": "own"}

# Cross-sections for collapse: (bone, share along it from its head).
SECTIONS = [("Hips", 0.5), ("Spine", 0.0), ("Spine", 0.5), ("Chest", 0.5), ("UpperChest", 0.5), ("Neck", 0.5),
            ("LeftUpperLeg", 0.15), ("LeftUpperLeg", 0.5), ("LeftLowerLeg", 0.5), ("LeftUpperArm", 0.2),
            ("LeftUpperArm", 0.6), ("LeftLowerArm", 0.5)]
# The trunk's sections take every vertex but the arms' (a trunk vertex is
# often weighted to three bones); a limb's those of the bone and its parent.
TRUNK = ("Hips", "Spine", "Chest", "UpperChest", "Neck")
# Regions for collapse: (name, the bones whose weight puts a vertex in it).
# (Weights folded: `context`.)
VOLUMES = [("pelvis", ("Hips",)), ("waist", ("Spine",)), ("chest", ("Chest", "UpperChest")),
           ("thigh", ("LeftUpperLeg",)), ("shin", ("LeftLowerLeg",)), ("upper arm", ("LeftUpperArm",)),
           ("forearm", ("LeftLowerArm",)), ("neck", ("Neck",))]
ARM_WORDS = ("Shoulder", "UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little", "Elbow")


# -- the body sliders on a pose ----------------------------------------------------------

def shaped(ch, sliders, basis=None):
    """Places every bone as `anim::BodyShape::apply` reshapes a pose:
    `Character.set_body_sliders` (same rules), but each bone keeps its own
    local transform from the pose (`basis`: {bone: 4x4 matrix_basis}, as
    `Q.apply_pose` and `set_eyes` left them), and the bones' pose matrices
    are set straight from the parents' (no scene update per bone)."""
    get = lambda name: float(sliders.get(name, 1.0))  # noqa: E731
    bones = ch.arm.data.bones
    scale = {b.name: Vector((1.0, 1.0, 1.0)) for b in bones}
    mult = {b.name: Vector((1.0, 1.0, 1.0)) for b in bones}

    def stretch(name, factor, axis=1):
        if name in scale:
            scale[name][axis] *= factor
            for child in bones[name].children:
                mult[child.name][axis] *= factor

    def grow(name, factor):
        if name not in scale or factor == 1.0:
            return
        scale[name] *= factor
        for child in bones[name].children_recursive:
            scale[child.name] *= factor
            mult[child.name] *= factor

    height, arms, shoulders = get("BodyHeight"), get("ArmLength"), get("ShoulderWidth")
    legs = get("LegLength") * height
    for name in ("Spine", "Chest", "UpperChest"):
        stretch(name, height)
    for side, _ in Q.SIDES:
        for name in ("UpperLeg", "LowerLeg"):
            stretch(side + name, legs)
        for name in ("UpperArm", "LowerArm"):
            stretch(side + name, arms)
        stretch(side + "Shoulder", shoulders)
    stretch("UpperChest", math.sqrt(shoulders), axis=0)
    stretch("Spine", get("WaistWidth"), axis=0)
    stretch("Hips", get("HipWidth"), axis=0)
    grow("Head", get("HeadSize"))
    for side, _ in Q.SIDES:
        grow(side + "Hand", get("HandSize"))
        grow(side + "Foot", get("FootSize"))
    local = {b.name: (b.parent.matrix_local.inverted() @ b.matrix_local if b.parent else b.matrix_local.copy())
             for b in bones}
    # A pose's own move of a bone (the eyes' for the feminine frame) is part
    # of its place in its parent, which the sliders stretch and grow with
    # the rest: `anim` adds identity's bone moves to the pose's
    # translations before it reshapes (`Model::settle`).
    own = {b.name: (basis.get(b.name, Matrix()) if basis else Matrix()) for b in bones}
    moved = {name: Vector([(m.to_translation() + m.to_3x3() @ own[name].to_translation())[k] * mult[name][k]
                           for k in range(3)]) for name, m in local.items()}
    for t in ch.custom.get("twists", []):
        for side, _ in Q.SIDES:
            if side + t["bone"] in scale:
                scale[side + t["bone"]] = scale[side + t["inside"]].copy()
    for h in ch.custom.get("helpers", []):
        for side, _ in Q.SIDES:
            name, reads = side + h["bone"], side + h["reads"]
            if name in scale:
                scale[name] = scale[reads].copy()
                moved[name] = moved[reads] + (local[name].to_translation() - local[reads].to_translation())
    if legs != 1.0:
        length = lambda a, b: (bones[b].head_local - bones[a].head_local).length  # noqa: E731
        leg = length("LeftUpperLeg", "LeftLowerLeg") + length("LeftLowerLeg", "LeftFoot")
        up = bones["Hips"].parent.matrix_local.to_3x3().inverted() @ Q.UP if bones["Hips"].parent else Q.UP
        moved["Hips"] = moved["Hips"] + up * leg * (legs - 1.0)
    frame, pose = {}, {}
    for b in bones:
        parent = frame[b.parent.name] if b.parent else Matrix()
        turn = own[b.name].to_3x3().to_4x4()
        frame[b.name] = parent @ Matrix.Translation(moved[b.name]) @ local[b.name].to_3x3().to_4x4() @ turn
        pose[b.name] = frame[b.name] @ Matrix.Diagonal(scale[b.name]).to_4x4()
    # A bone's scale isn't passed on to its children's turn and scale
    # (`Model::pose_shaped`), which is Blender's `inherit_scale` NONE. (With
    # FULL, a child of a bone scaled along one axis would need a sheared
    # basis, which Blender's location, rotation and scale can't hold.) Under
    # NONE Blender turns and scales a child by its parent's pose without the
    # scale, but places it by the whole of it:
    #   pose 3x3 = parent's turn @ rest 3x3 @ basis 3x3
    #   pose head = parent's pose @ (rest offset + rest 3x3 @ basis location)
    for b in bones:
        b.inherit_scale = "NONE"
        parent_turn = frame[b.parent.name].to_3x3() if b.parent else Matrix.Identity(3)
        parent_pose = pose[b.parent.name] if b.parent else Matrix()
        rest = local[b.name].to_3x3()
        turn = rest.inverted() @ parent_turn.inverted() @ pose[b.name].to_3x3()
        at = rest.inverted() @ (parent_pose.inverted() @ pose[b.name].to_translation() - local[b.name].to_translation())
        ch.arm.pose.bones[b.name].matrix_basis = Matrix.Translation(at) @ turn.to_4x4()
    Q.update()
    return pose


def look(ch, frame, values, sliders, pose=None):
    """The body with `values` (manifest parameters), `sliders` ({attribute:
    value}) and a QA pose (or the rest pose), as a `Q.Geo`."""
    ch.set_sliders(dict(values, frame=frame))
    if pose and pose != "rest":
        Q.apply_pose(ch, pose)
    else:
        ch.pose_reset()
    basis = {pb.name: pb.matrix_basis.copy() for pb in ch.arm.pose.bones}
    shaped(ch, sliders, basis)
    return Q.Geo(ch.parts["Body"])


def heads(ch):
    """Every bone's head and tail as posed (armature space; the armature
    is at the origin)."""
    return ({pb.name: Vector(pb.head) for pb in ch.arm.pose.bones},
            {pb.name: Vector(pb.tail) for pb in ch.arm.pose.bones})


# -- measures ------------------------------------------------------------------------------

def arm_share(W):
    return np.array([sum(w for b, w in d.items() if any(k in b for k in ARM_WORDS)) for d in W])


def bone_share(W, names):
    return np.array([sum(d.get(n, 0.0) for n in names) for d in W])


def breadth(co, keep, z0, z1, slab=0.01):
    """The widest side-to-side extent of the `keep` vertices in `slab`
    slabs between heights z0 and z1."""
    out = []
    z = z0
    while z < z1:
        sel = keep & (co[:, 2] >= z) & (co[:, 2] < z + slab)
        if sel.sum() > 8:
            out.append(float(co[sel, 0].max() - co[sel, 0].min()))
        z += slab
    if not out:
        return None
    return max(out)


def dimension(pid, ch, geo, W, masks):
    """The dimension of `POPULATION[pid]` on the body as it stands (rest
    pose, sliders applied): stature in metres, else over stature."""
    co = geo.co
    H = float(co[:, 2].max() - co[:, 2].min())
    floor = float(co[:, 2].min())
    h, _t = heads(ch)
    if pid == "height":
        return H
    if pid == "leg_length":
        return (h["LeftUpperLeg"].z - floor) / H
    if pid == "arm_length":
        return ((h["LeftLowerArm"] - h["LeftUpperArm"]).length + (h["LeftHand"] - h["LeftLowerArm"]).length) / H
    if pid == "shoulder_width":
        return (h["LeftUpperArm"].x - h["RightUpperArm"].x) / H
    trunk = masks["trunk"]
    hips = breadth(co, trunk, h["LeftUpperLeg"].z - 0.12, h["Spine"].z)
    if pid == "waist_width":
        middle = (h["Spine"].z + h["Chest"].z) / 2.0
        return breadth(co, trunk, middle - 0.015, middle + 0.015, slab=0.03) / hips
    if pid == "hip_width":
        return hips / H
    if pid == "head_size":
        sel = masks["head"]
        return float(co[sel, 0].max() - co[sel, 0].min()) / H
    if pid == "hand_size":
        wr = h["LeftHand"]
        along = (h["LeftMiddleProximal"] - wr).normalized()
        P = co[masks["hand"]] - np.array(wr)
        return float((P @ np.array(along)).max()) / H
    if pid == "foot_size":
        P = co[masks["foot"]]
        return float(P[:, 1].max() - P[:, 1].min()) / H
    raise KeyError(pid)


def ring(co, edges, W_joint, c, n, radius):
    """The closed ring where the plane through `c` with normal `n` cuts the
    skin's edges near `c` whose two ends belong to the joint's bones
    (`W_joint`, each vertex's weight to them, at least 1 between the two
    ends): (vertex i, vertex j, t) ordered round the plane."""
    a, b = co[edges[:, 0]], co[edges[:, 1]]
    da, db = (a - c) @ n, (b - c) @ n
    cross = (da * db <= 0) & (da != db)
    t = np.where(cross, da / np.where(da == db, 1.0, da - db), 0.0)
    x = a + (b - a) * t[:, None]
    near = np.linalg.norm(x - c, axis=1) <= radius
    held = (W_joint[edges[:, 0]] + W_joint[edges[:, 1]]) >= 1.0
    pick = np.where(cross & near & held)[0]
    u = np.array(Vector(n).orthogonal().normalized())
    w = np.cross(n, u)
    # Ordered round the points' own middle: the spine's joints lie near the
    # back's skin, and ordered round the joint the polygon could fold on
    # itself when one point passed another (`measure.ring_points` orders
    # round the joint; for the limbs the two agree).
    xs = x[pick] - x[pick].mean(axis=0) if len(pick) else x[pick]
    order = np.argsort(np.arctan2(xs @ w, xs @ u))
    return [(int(edges[k, 0]), int(edges[k, 1]), float(t[k])) for k in pick[order]]


def ring_area(co, pts):
    if len(pts) < 3:
        return 0.0
    xs = np.array([co[i] + (co[j] - co[i]) * t for i, j, t in pts])
    s = np.cross(xs, np.roll(xs, -1, axis=0)).sum(axis=0)
    return float(np.linalg.norm(s) / 2.0)


def joint_frame(h, t, pivot, parent):
    """The plane `measure.ring_points` cuts a joint with: through the
    pivot's head, normal to the parent bone (along the pivot from the
    hips and the collarbone)."""
    if parent in ("Hips", "LeftShoulder") or pivot == "LeftMiddleProximal":
        n = (t[pivot] - h[pivot]).normalized()
    elif pivot == "Chest":
        n = (h["Chest"] - h["Spine"]).normalized()
    else:
        n = (h[pivot] - h[parent]).normalized()
    return np.array(h[pivot]), np.array(n)


class Body:
    """What a body at 1 measures, to compare a step with: its crossings,
    its joints in each pose, its cross-sections and region areas."""

    def __init__(self, ch, frame, values, poses, ctx):
        self.frame, self.values = frame, values
        rest = look(ch, frame, values, {})
        self.rest_co = rest.co
        self.known = M.crossing_pairs(rest)
        self.sections = sections(ch, rest, ctx)
        self.outline = outline(ch, rest, ctx)
        self.volumes = volumes(rest, ctx)
        self.joints = {pose: joint(ch, frame, values, {}, pose, ctx) for pose in poses}


def sections(ch, geo, ctx):
    h, t = heads(ch)
    out = {}
    for bone, share in SECTIONS:
        c = h[bone] + (t[bone] - h[bone]) * share
        n = (t[bone] - h[bone]).normalized()
        if bone in TRUNK:
            held = ctx["masks"]["trunk"].astype(float)
        else:
            held = bone_share(ctx["W"], (bone, ch.arm.data.bones[bone].parent.name))
        pts = ring(geo.co, ctx["edges"], held, np.array(c), np.array(n), 0.3)
        out[f"{bone}@{share:g}"] = least_breadth(geo.co, pts, np.array(n))
    return out


def least_breadth(co, pts, n):
    """A cross-section's narrowest breadth: the least, over 36 directions
    in its plane, of how far its points spread. (Not its area: by the
    trunk's sides a plane picks up a few points of the armpit or the
    groin, and a polygon through them ordered by angle can fold on
    itself, while the narrowest breadth is what a pinch makes small.)"""
    if len(pts) < 3:
        return 0.0
    xs = np.array([co[i] + (co[j] - co[i]) * t for i, j, t in pts])
    u = np.array(Vector(n).orthogonal().normalized())
    w = np.cross(n, u)
    angles = np.linspace(0.0, math.pi, 36, endpoint=False)
    dirs = np.cos(angles)[:, None] * u + np.sin(angles)[:, None] * w
    proj = xs @ dirs.T
    return float((proj.max(axis=0) - proj.min(axis=0)).min())


def outline(ch, geo, ctx):
    """The trunk's breadth (arms left out) at `SLABS` heights from below
    the hip joints to above the chest's joint, placed by those two joints'
    heights, so a height is the same place on a taller or longer-legged
    body. Each is where a level plane cuts the skin's edges (not the
    vertices in a slab, whose breadth jumps as a ring of vertices slips in
    or out of it)."""
    h, _t = heads(ch)
    hip, chest = h["LeftUpperLeg"].z, h["Chest"].z
    span = chest - hip
    low, high = OUTLINE_SPAN
    edges = ctx["trunk_edges"]
    a, b = geo.co[edges[:, 0]], geo.co[edges[:, 1]]
    out = {}
    for k in range(SLABS + 1):
        z = hip + (low + k * (high - low) / SLABS) * span
        cross = (a[:, 2] - z) * (b[:, 2] - z) <= 0
        cross &= a[:, 2] != b[:, 2]
        if cross.sum() < 8:
            out[k] = None
            continue
        t = (z - a[cross, 2]) / (b[cross, 2] - a[cross, 2])
        x = a[cross, 0] + (b[cross, 0] - a[cross, 0]) * t
        out[k] = float(x.max() - x.min())
    return out


def outline_jump(now, then):
    """(the largest change between neighbouring levels of the breadth's
    ratio to the body's at 1, the level halfway between the two)."""
    keys = [k for k in sorted(now) if k in then and now[k] and then[k]]
    ratio = [now[k] / then[k] for k in keys]
    jumps = [(abs(a - b), keys[i]) for i, (a, b) in enumerate(zip(ratio, ratio[1:]))]
    worst = max(jumps, default=(0.0, 0))
    low, high = OUTLINE_SPAN
    return [round(worst[0], 4), round(low + (worst[1] + 0.5) * (high - low) / SLABS, 3)]


def volumes(geo, ctx):
    """Each region's skin area (m^2). (A region is an open patch, so its
    enclosed volume isn't defined, and a cone volume from its middle
    swings with the patch's outline; its area falls when it is pinched or
    squashed.)"""
    out = {}
    for name, tris in ctx["regions"].items():
        a, b, c = (geo.co[tris[:, k]] for k in range(3))
        out[name] = float(np.linalg.norm(np.cross(b - a, c - a), axis=1).sum() / 2.0)
    return out


def joint(ch, frame, values, sliders, pose, ctx):
    """(ring area ratio, local volume ratio) of a QA pose's joint on the
    body with `sliders`, measured as `measure.deformation` does, on that
    body's own rest shape."""
    _fn, (pivot, parent, radius) = Q.POSES[pose]
    rest = look(ch, frame, values, sliders)
    h, t = heads(ch)
    c, n = joint_frame(h, t, pivot, parent)
    region = np.linalg.norm(rest.co - c, axis=1) < radius
    tris = rest.tris[region[rest.tris].all(axis=1)]
    v0 = M.local_volume(rest.co, tris, c)
    pts = ring(rest.co, ctx["edges"], bone_share(ctx["W"], (pivot, parent)), c, n, radius * 0.8)
    a0 = ring_area(rest.co, pts)
    posed = look(ch, frame, values, sliders, pose)
    c1 = np.array(ch.arm.pose.bones[pivot].head)
    v1 = M.local_volume(posed.co, tris, c1)
    a1 = ring_area(posed.co, pts)
    return {"ring_area_ratio": round(a1 / a0, 3) if a0 else None,
            "local_volume_ratio": round(v1 / v0, 3) if v0 else None}


def joint_floor(limits, label, pose, key, at_one):
    """The least a pose's ratio may be: the package's limit, or `KEEP` of
    the body's own at 1 where there's no limit or the body at 1 is
    already under it. A ratio over 1 is a joint that swells as it bends;
    only what it loses against its rest size counts, so the floor is taken
    from at most 1."""
    limit = limits.get("at_least", {}).get(f"deformation/{label}/{pose}/{key}")
    if limit is not None and at_one is not None and at_one >= limit:
        return limit
    return None if at_one is None else round(KEEP * min(at_one, 1.0), 3)


def joint_ceiling(limits, label, pose, key, at_one):
    """The most a pose's ratio may be (the lowered shoulder's bulge): the
    package's limit, or `BULGE` over the body's own at 1 where that is
    within `BULGE` of the limit already (the limits were set at what the
    default body reached, so a slider's millimetre would fail them)."""
    limit = limits.get("at_most", {}).get(f"deformation/{label}/{pose}/{key}")
    if limit is None or at_one is None:
        return None
    return round(max(limit, at_one + BULGE), 3)


def eyes_and_teeth(ch, geo):
    """`measure.eye_metrics` on the body as it stands, with each eye's
    centre where its bone is now (the sliders move the head)."""
    saved = ch.eye_centre
    ch.eye_centre = lambda side: Vector(ch.arm.pose.bones[f"{side}Eye"].head)
    try:
        m = M.eye_metrics(ch, geo, Q.Geo(ch.parts["Eyes"]), Q.Geo(ch.parts["Teeth"]))
    finally:
        ch.eye_centre = saved
    return {k: m.get(k) for k in ("skin_inside_eyeball_verts", "front_eye_inside_lid_flesh_verts",
                                  "teeth_verts_inside_lips_or_gums_flesh")}


# -- the sweep ---------------------------------------------------------------------------

def setting(pid, attribute, value, by_bone):
    """(parameter values, attribute values) for a slider at `value`. The
    hips below 1 move no bone (the manifest's driver starts at 1); the
    parameter drives the narrowing shape. `by_bone` narrows by the bone."""
    if pid == "hip_width" and value < 1.0 and not by_bone:
        return {pid: value}, {}
    return {pid: value}, {attribute: value}


def check_step(ch, label, frame, pid, attribute, value, bodies, limits, ctx, by_bone):
    """Every check of one step on one frame: (failures, details)."""
    failures, details = [], {}
    for name, values in COMPOSITIONS:
        at_one = bodies[name]
        params, sliders = setting(pid, attribute, value, by_bone)
        geo = look(ch, frame, dict(values, **params), sliders)
        P = at_one.rest_co
        tris = geo.tris
        n = 0
        for a, b in M.crossing_pairs(geo) - at_one.known - ctx["neutral_known"][label]:
            gap = np.linalg.norm(P[tris[a]][:, None] - P[tris[b]][None], axis=2).min()
            n += gap > TOUCHING
        d = {"new_crossings": int(n)}
        if n:
            failures.append(f"{name}: {n} new crossings")
        sec = sections(ch, geo, ctx)
        worst = min(((sec[k] / at_one.sections[k], k) for k in sec if at_one.sections[k] > 0), default=(1.0, ""))
        vol = volumes(geo, ctx)
        worst_v = min(((vol[k] / at_one.volumes[k], k) for k in vol if at_one.volumes[k] > 0), default=(1.0, ""))
        d["least_section"] = [round(worst[0], 3), worst[1]]
        d["least_region_area"] = [round(worst_v[0], 3), worst_v[1]]
        if worst[0] < COLLAPSE:
            failures.append(f"{name}: section {worst[1]} at {worst[0]:.2f} of its size at 1")
        if worst_v[0] < COLLAPSE:
            failures.append(f"{name}: {worst_v[1]} skin area at {worst_v[0]:.2f} of its size at 1")
        jump = outline_jump(outline(ch, geo, ctx), at_one.outline)
        d["outline_jump"] = jump
        if jump[0] > OUTLINE:
            failures.append(f"{name}: the trunk's outline steps by {jump[0]:.3f} at {jump[1]} of the hips to chest")
        # Joints: the ring area always; the local volume where the package
        # holds it (the shoulder at 90 and lowered: its 150 degree volume is
        # reported, not held: the decision of 2026-11-16 in thresholds.json).
        if name == "own body":
            et = eyes_and_teeth(ch, geo)
            d["eyes_and_teeth"] = et
            if any(et.values()):
                failures.append(f"{name}: eyes or teeth in flesh {et}")
        for pose in POSES[pid]:
            got = joint(ch, frame, dict(values, **params), sliders, pose, ctx)
            d[pose] = got
            for key in ("ring_area_ratio", "local_volume_ratio"):
                held = any(f"deformation/{label}/{pose}/{key}" in limits.get(k, {}) for k in ("at_least", "at_most"))
                if got[key] is None or (key == "local_volume_ratio" and not held):
                    continue
                low = joint_floor(limits, label, pose, key, at_one.joints[pose][key])
                high = joint_ceiling(limits, label, pose, key, at_one.joints[pose][key])
                if low is not None and got[key] < low:
                    failures.append(f"{name}: {pose} {key} {got[key]} under {low}")
                if high is not None and got[key] > high:
                    failures.append(f"{name}: {pose} {key} {got[key]} over {high}")
        details[name] = d
    return failures, details


def population_bounds(pid, label, base):
    """(low, high) of the slider's dimension at the 0.5th and 99.5th
    percentiles, in the dimension's units on this body: absolute for an
    `own` anchor, the body's own value times the relative spread for a
    `mean` one."""
    mean, sd = POPULATION[pid][label]
    lo, hi = mean - Z99 * sd, mean + Z99 * sd
    if ANCHOR.get(pid) == "own":
        return lo, hi
    return base * lo / mean, base * hi / mean


def crossing_value(steps, bound, base, sign):
    """The slider value where the measured dimension reaches `bound`
    (linear between the steps that straddle it), or None when no step
    does. 1 when the body at 1 is already past it (the masculine foot is
    shorter than the 0.5th percentile's: a smaller foot leaves the
    population at once)."""
    if (base - bound) * sign >= 0:
        return 1.0
    prev = (1.0, base)
    for value, dim in steps:
        if (prev[1] - bound) * (dim - bound) <= 0 and dim != prev[1]:
            return prev[0] + (value - prev[0]) * (bound - prev[1]) / (dim - prev[1])
        prev = (value, dim)
    return None


def sweep(ch, pid, attribute, step, limits, ctx, frames, by_bone, log):
    out = {"attribute": attribute, "poses": POSES[pid], "frames": {}, "sides": {}}
    bodies, base_dim = {}, {}
    for label, frame in frames:
        bodies[label] = {name: Body(ch, frame, values, POSES[pid], ctx) for name, values in COMPOSITIONS}
        look(ch, frame, {}, {})
        base_dim[label] = dimension(pid, ch, Q.Geo(ch.parts["Body"]), ctx["W"], ctx["masks"])
        lo, hi = population_bounds(pid, label, base_dim[label])
        out["frames"][label] = {"dimension_at_1": round(base_dim[label], 4),
                                "population": {"what": POPULATION[pid]["what"], "mean_sd": POPULATION[pid][label],
                                               "anchor": ANCHOR.get(pid, "mean"),
                                               "bounds": [round(lo, 4), round(hi, 4)]},
                                "at_one": {name: {"joints": b.joints} for name, b in bodies[label].items()}}
    for side, sign in (("low", -1), ("high", 1)):
        steps, dims = [], {label: [] for label, _ in frames}
        last_pass, fail = 1.0, None
        k = 0
        floor = narrowest(ch) if pid == "hip_width" and side == "low" and not by_bone else BOUND[0]
        while True:
            k += 1
            value = round(1.0 + sign * k * step, 4)
            if value < floor - 1e-9 or value > BOUND[1] + 1e-9:
                break
            record = {"value": value, "frames": {}}
            failed = []
            for label, frame in frames:
                params, sliders = setting(pid, attribute, value, by_bone)
                look(ch, frame, params, sliders)
                dim = dimension(pid, ch, Q.Geo(ch.parts["Body"]), ctx["W"], ctx["masks"])
                dims[label].append((value, dim))
                failures, details = check_step(ch, label, frame, pid, attribute, value, bodies[label], limits, ctx,
                                               by_bone)
                record["frames"][label] = {"dimension": round(dim, 4), "failures": failures, "checks": details}
                failed += [f"{label} {f}" for f in failures]
            record["passed"] = not failed
            steps.append(record)
            log(f"{pid} {value:.3f}: {'pass' if not failed else 'FAIL ' + '; '.join(failed[:3])}")
            if failed:
                fail = {"value": value, "failures": failed}
                break
            last_pass = value
            # Past every frame's population bound and nothing failed: stop.
            # (Every dimension grows with its slider, so the high side meets
            # the 99.5th percentile and the low side the 0.5th.)
            beyond = True
            for label, _ in frames:
                lo, hi = population_bounds(pid, label, base_dim[label])
                if crossing_value(dims[label], hi if sign > 0 else lo, base_dim[label], sign) is None:
                    beyond = False
            if beyond:
                break
        population = {}
        for label, _ in frames:
            lo, hi = population_bounds(pid, label, base_dim[label])
            bound = hi if sign > 0 else lo
            at = crossing_value(dims[label], bound, base_dim[label], sign)
            population[label] = {"bound": round(bound, 4), "slider": None if at is None else round(at, 4)}
        out["sides"][side] = {"last_passing_step": last_pass, "first_failing_step": fail, "population": population,
                              "steps": steps}
        clamp = "shape" if pid == "hip_width" and side == "low" and not by_bone else "clamp"
        out["sides"][side]["end"] = end_of(side, sign, last_pass, fail, population, floor, clamp)
    return out


def narrowest(ch):
    """Where the hips' narrowing shape is whole (its corner's span): the
    sweep can't go past it."""
    for corner in ch.manifest.get("corners", []):
        if corner["shape"].endswith("_Hips_Narrow"):
            return min(f["from"][1] for f in corner["factors"] if f["parameter"] == "hip_width")
    return 1.0


def end_of(side, sign, last_pass, fail, population, clamp, clamp_name="clamp"):
    """Where the shipped range ends on one side, and what set it. The
    candidates: the last passing step when a step failed (on any frame or
    body: the range is one for both frames); the population's percentile,
    the farther of the two frames' (a parameter has one range, and each
    frame's own people reach their own end: a range that stopped at the
    nearer would hold tall men to the 99.5th percentile of women), rounded
    to 0.01 toward 1; and `anim`'s clamp (or where the hips' narrowing
    shape is whole: `shape`).
    The nearest to 1 wins."""
    found = [population[label]["slider"] for label in population if population[label]["slider"] is not None]
    candidates = []
    if fail:
        candidates.append((last_pass, "deformation"))
    if found:
        far = max(found) if sign > 0 else min(found)
        rounded = math.floor(far * 100 + 1e-6) / 100 if sign > 0 else math.ceil(far * 100 - 1e-6) / 100
        candidates.append((rounded, "population"))
    candidates.append((clamp if sign < 0 else BOUND[1], clamp_name if sign < 0 else "clamp"))
    value = min(candidates, key=lambda c: abs(c[0] - 1.0))[0]
    return {"value": round(value, 3), "set_by": [why for v, why in candidates if abs(v - value) < 1e-9],
            "candidates": {why: round(v, 4) for v, why in candidates}}


def context(ch):
    # Twist and helper weights counted as their limb's, as `measure.py` does.
    W = M.fold(M.obj_weights(ch.parts["Body"]), ch.custom)
    me = ch.parts["Body"].data
    edges = np.array([tuple(e.vertices) for e in me.edges], dtype=np.int64)
    arms = arm_share(W)
    head = bone_share(W, ("Head",)) > 0.5
    hand = np.array([sum(w for b, w in d.items() if b.startswith("Left") and any(
        k in b for k in M.HAND_BONES)) > 0.5 for d in W])
    foot = bone_share(W, ("LeftFoot", "LeftToes")) > 0.5
    trunk = arms < 0.5
    trunk_edges = edges[trunk[edges[:, 0]] & trunk[edges[:, 1]]]
    rest = Q.Geo(ch.parts["Body"])
    regions = {}
    for name, names in VOLUMES:
        own = bone_share(W, names) > 0.5
        tris = rest.tris[own[rest.tris].all(axis=1)]
        regions[name] = tris
    masks = {"trunk": trunk, "head": head, "hand": hand, "foot": foot}
    return {"W": W, "edges": edges, "trunk_edges": trunk_edges, "masks": masks,
            "regions": regions}


def self_check(ch):
    """`shaped` on a QA pose with every slider moved: the largest gap, in
    millimetres, between where Blender put each bone's head and axes and
    where `anim`'s rule puts them."""
    sliders = {"BodyHeight": 1.1, "LegLength": 0.9, "ArmLength": 1.1, "ShoulderWidth": 0.9, "WaistWidth": 1.1,
               "HipWidth": 1.1, "HeadSize": 0.9, "HandSize": 1.1, "FootSize": 0.9}
    ch.set_sliders({"frame": 0.0})
    Q.apply_pose(ch, "elbow_135")
    basis = {pb.name: pb.matrix_basis.copy() for pb in ch.arm.pose.bones}
    want = shaped(ch, sliders, basis)
    gap = 0.0
    for pb in ch.arm.pose.bones:
        got = pb.matrix
        gap = max(gap, max(abs(got[i][j] - want[pb.name][i][j]) for i in range(4) for j in range(4)))
    return gap * 1000.0


WHY = {"deformation": "deformation", "population": "population", "clamp": "`anim`'s clamp",
       "shape": "the narrowing shape's end"}


def report(sweep):
    """The sweep's ends as Markdown: the shipped table, then each side's
    numbers per frame."""
    lines = ["## Shipped ranges", "", "| Parameter | Low end | Set by | High end | Set by |", "|---|---|---|---|---|"]
    detail = ["## Each end's numbers", "",
              "| Parameter | Side | Frame | Dimension at 1 | Population bound | Its slider value | "
              "Last passing step | First failing step and why |", "|---|---|---|---|---|---|---|---|"]
    for pid, d in sweep["parameters"].items():
        row = [f"`{pid}`"]
        for side in ("low", "high"):
            end = d["sides"][side]["end"]
            row += [f"{end['value']:g}", ", ".join(WHY[w] for w in end["set_by"])]
        lines.append("| " + " | ".join(row) + " |")
        for side in ("low", "high"):
            s = d["sides"][side]
            fail = s["first_failing_step"]
            why = f"{fail['value']:g}: {'; '.join(fail['failures'][:2])}" if fail else "none in the sweep"
            for label, pop in s["population"].items():
                f = d["frames"][label]
                at = "not reached" if pop["slider"] is None else f"{pop['slider']:.3f}"
                detail.append(f"| `{pid}` | {side} | {label} | {f['dimension_at_1']:g} | {pop['bound']:g} | {at} | "
                              f"{s['last_passing_step']:g} | {why} |")
    return "\n".join(lines + [""] + detail)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="realistic", choices=sorted(Q.PACKAGES))
    ap.add_argument("--out", default="ranges.json")
    ap.add_argument("--only", default="")
    ap.add_argument("--step", type=float, default=0.025)
    ap.add_argument("--frames", default="masculine,feminine")
    ap.add_argument("--hips-by-bone", action="store_true")
    ap.add_argument("--report", default="", help="a sweep's JSON: print its tables as Markdown and stop")
    a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
    if a.report:
        print(report(json.load(open(a.report))))
        return
    parts, _, custom = Q.package_paths(a.package)
    Q.reset()
    ch = Q.Character(parts, merge_vertices=True, custom=Q.load_custom(custom))
    ch.show_only(["Body", "Eyes", "Teeth"])
    # The sweep goes past the manifest's ranges: open them to `anim`'s clamp,
    # so the manifest's rule still sets the shapes (the hips' narrowing).
    for p in ch.manifest["parameters"]:
        if p["group"] == "proportions":
            p["range"] = list(BOUND)
    limits = json.load(open(THRESHOLDS))
    t0 = time.time()
    log = lambda s: print(f"[{time.time() - t0:7.1f}s] {s}", flush=True)  # noqa: E731
    gap = self_check(ch)
    log(f"self-check: bones against anim's rule, {gap:.3f} mm")
    assert gap < 0.1, gap
    ctx = context(ch)
    frames = [(label, f) for label, f in (("masculine", 0.0), ("feminine", 1.0)) if label in a.frames.split(",")]
    ctx["neutral_known"] = {}
    for label, frame in frames:
        ctx["neutral_known"][label] = M.crossing_pairs(look(ch, frame, {}, {}))
    only = set(filter(None, a.only.split(",")))
    out = {"about": __doc__.split("\n\n")[0], "step": a.step, "keep": KEEP, "collapse": COLLAPSE,
           "touching_m": TOUCHING, "z": Z99, "compositions": [n for n, _ in COMPOSITIONS],
           "hips_by_bone": a.hips_by_bone, "self_check_mm": round(gap, 4), "parameters": {}}
    for pid, attribute in SLIDERS:
        if only and pid not in only:
            continue
        out["parameters"][pid] = sweep(ch, pid, attribute, a.step, limits, ctx, frames, a.hips_by_bone, log)
        with open(a.out, "w", newline="\n") as f:
            json.dump(out, f, indent=1)
            f.write("\n")
    log(f"wrote {a.out}")


if __name__ == "__main__":
    main()
