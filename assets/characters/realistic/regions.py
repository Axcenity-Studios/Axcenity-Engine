"""Body regions: which part of the body each triangle is, so a worn item
can hide the skin it covers (ADR-0035; the manifest's `regions[].map_index`).

Every body triangle has one region index. It is stored twice, from the
same per-face data:

- in the body's second texture coordinate set (`Regions`, glTF
  `TEXCOORD_1`): all three corners of a triangle sit at the centre of its
  region's cell, `u = (index + 0.5) / 16`, `v = 0.5`. This is the exact,
  per-triangle answer, and the one to hide triangles by;
- in `RealisticRegions.png`, a 1024 px image over the first texture
  coordinates whose grey value is the index (0 where no skin is), for
  anything that works in texture space.

Index 0 is no region (never hidden). The region IDs the stylized body
also has (`torso`, `hips`, `upper_arms_top`, …) mean the same here, so an
item that names them fits both; `head`, `neck`, `waist` and `hands` are
finer than its masks could be.

Two cuts follow where garments end (CO9 part 2, review fixes 1), so a
garment that covers a region whole can hide it:
- the `neck` runs down to the neck's joint: the trunk's skin above it,
  round the neck, is the neck's, which a crew neck doesn't cover (it was
  the torso's, so no top could hide the torso);
- the upper arm has three bands: `upper_arms_top` to halfway along the
  bone, `upper_arms_mid` the skin a short sleeve keeps covered (whole
  faces to three quarters of the way, farther underneath and less on top,
  as a T-shirt's hem leans), and `upper_arms_low` to the elbow.
  `upper_arms_mid` is the last of the 15 indices the second texture
  coordinates can hold.
"""

import os

import numpy as np

from common import (
    FS_EAR, FS_FINGERS, FS_FOOT, FS_FOREARM, FS_HEAD, FS_MUZZLE, FS_PALM, FS_PELVIS, FS_SHIN,
    FS_SOCKET, FS_THIGH, FS_TOE_BASE, FS_TOES, FS_TORSO, FS_UPPER_ARM, FS_WAIST, MOUTH_SETS,
)

CELLS = 16
MAP_SIZE = 1024
MAP_FILE = "RealisticRegions.png"
# (id, label), by index from 1.
REGIONS = [
    ("head", "Head"),
    ("neck", "Neck"),
    ("torso", "Torso"),
    ("waist", "Waist"),
    ("hips", "Hips"),
    ("upper_arms_top", "Upper arms (top)"),
    ("upper_arms_low", "Upper arms (low)"),
    ("forearms", "Forearms"),
    ("hands", "Hands"),
    ("thighs_top", "Thighs (top)"),
    ("thighs_low", "Thighs (low)"),
    ("shins_top", "Shins (top)"),
    ("shins_low", "Shins (low)"),
    ("feet", "Feet"),
    ("upper_arms_mid", "Upper arms (middle)"),
]
# The short sleeve's band: faces whose every corner is within this share
# of the upper arm along the bone, plus `SLEEVE_TILT` times how far the
# face faces in (towards the side, under the arm: 1) or out (on top: -1).
# Whole faces, so none reaches past a sleeve's hem: a hidden face there
# would leave a hole in the arm (the fit gate's `hidden_showing`).
SLEEVE_END = 0.8
SLEEVE_TILT = 0.1
# The neck's skin round the neck's joint: within this of the neck's line,
# above the joint less `NECK_BELOW` (the skin a crew neck leaves bare; the
# collar lies on the skin just past it).
NECK_ROUND = 0.088
NECK_BELOW = 0.005
INDEX = {name: k + 1 for k, (name, _) in enumerate(REGIONS)}


def of_faces(body, sets, P, joints, weights):
    """Each polygon's region index, from its face set and, along a limb,
    which half of the bone its centre is nearer. The neck ends where the
    skin is the head's own (by its weights)."""
    X = np.array([tuple(p) for p in P])
    me = body.data
    centres = np.array([X[list(p.vertices)].mean(axis=0) for p in me.polygons])
    J = {name: (np.array(h), np.array(t)) for name, (h, t, _) in joints.items()}

    def along(bone, points):
        head, tail = J[bone]
        axis = tail - head
        return ((points - head) @ axis) / (axis @ axis)

    held = sum(weights.col(bone) for bone in ("Head", "Jaw", "LeftEye", "RightEye"))
    neck = J["Neck"][0]

    def facing_in(i, side):
        """How far polygon `i` faces in across the upper arm (towards the
        body's middle, under the arm: 1; on top: -1)."""
        head, tail = J[side + "UpperArm"]
        axis = (tail - head) / np.linalg.norm(tail - head)
        inward = np.array([-1.0 if side == "Left" else 1.0, 0.0, 0.0])
        inward = inward - (inward @ axis) * axis
        inward /= np.linalg.norm(inward)
        v = X[list(me.polygons[i].vertices)]
        c = v.mean(axis=0)
        normal = sum(np.cross(v[k] - c, v[(k + 1) % len(v)] - c) for k in range(len(v)))
        return float(normal @ inward / max(np.linalg.norm(normal), 1e-12))
    out = np.zeros(len(me.polygons), dtype=np.int64)
    by_set = {}
    for s in (1, -1):
        side = "Left" if s > 0 else "Right"
        by_set[FS_UPPER_ARM[s]] = ("arm", side)
        by_set[FS_FOREARM[s]] = ("forearms", None)
        by_set[FS_PALM[s]] = ("hands", None)
        for group in FS_FINGERS[s].values():
            for fs in group:
                by_set[fs] = ("hands", None)
        by_set[FS_THIGH[s]] = ("thigh", side)
        by_set[FS_SHIN[s]] = ("shin", side)
        by_set[FS_FOOT[s]] = ("feet", None)
        by_set[FS_TOE_BASE[s]] = ("feet", None)
        for fs in FS_TOES[s]:
            by_set[fs] = ("feet", None)
        by_set[FS_EAR[s]] = ("head", None)
        by_set[FS_SOCKET[s]] = ("head", None)
    for fs in MOUTH_SETS:
        by_set[fs] = ("head", None)
    by_set[FS_MUZZLE] = ("head", None)
    by_set[FS_TORSO] = ("torso", None)
    by_set[FS_WAIST] = ("waist", None)
    by_set[FS_PELVIS] = ("hips", None)
    for i, fs in enumerate(sets):
        c = centres[i]
        if fs == FS_HEAD:
            on_head = held[list(me.polygons[i].vertices)].mean() > 0.5
            out[i] = INDEX["head" if on_head else "neck"]
            continue
        kind, side = by_set[fs]
        if kind == "arm":
            t = along(side + "UpperArm", c)
            farthest = along(side + "UpperArm", X[list(me.polygons[i].vertices)]).max()
            if t < 0.5:
                kind = "upper_arms_top"
            elif farthest < SLEEVE_END + SLEEVE_TILT * facing_in(i, side):
                kind = "upper_arms_mid"
            else:
                kind = "upper_arms_low"
        elif kind == "torso":
            round_neck = np.hypot(c[0] - neck[0], c[1] - neck[1])
            if c[2] > neck[2] - NECK_BELOW and round_neck < NECK_ROUND:
                kind = "neck"
        elif kind == "thigh":
            kind = "thighs_top" if along(side + "UpperLeg", c) < 0.5 else "thighs_low"
        elif kind == "shin":
            kind = "shins_top" if along(side + "LowerLeg", c) < 0.5 else "shins_low"
        out[i] = INDEX[kind]
    return out


def write_layer(obj, face_regions):
    """The `Regions` texture coordinate set on `obj` from its polygons'
    region indices."""
    me = obj.data
    layer = me.uv_layers.get("Regions") or me.uv_layers.new(name="Regions")
    for poly, region in zip(me.polygons, face_regions):
        for i in poly.loop_indices:
            layer.data[i].uv = ((int(region) + 0.5) / CELLS, 0.5)
    me.uv_layers["UVMap"].active = True
    me.uv_layers["UVMap"].active_render = True


def read_layer(obj):
    """Each polygon's region index back from the `Regions` set."""
    layer = obj.data.uv_layers["Regions"].data
    return [int(layer[p.loop_indices[0]].uv[0] * CELLS) for p in obj.data.polygons]


def write_map(obj, face_regions, out_dir):
    """`RealisticRegions.png`: the region index of the skin under each
    texel of the first texture coordinates, bled a few texels past each
    island's edge so a sample on the edge still finds its region."""
    from PIL import Image, ImageDraw
    img = Image.new("L", (MAP_SIZE, MAP_SIZE), 0)
    draw = ImageDraw.Draw(img)
    layer = obj.data.uv_layers["UVMap"].data
    for poly, region in zip(obj.data.polygons, face_regions):
        pts = [(layer[i].uv[0] * MAP_SIZE, (1 - layer[i].uv[1]) * MAP_SIZE) for i in poly.loop_indices]
        draw.polygon(pts, fill=int(region), outline=int(region))
    a = np.asarray(img).copy()
    for _ in range(3):
        grown = a.copy()
        for shift in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            moved = np.roll(a, shift, axis=(0, 1))
            grown = np.where(grown == 0, moved, grown)
        a = grown
    Image.fromarray(a).save(os.path.join(out_dir, MAP_FILE), optimize=True)


def mark(body, sets, P, joints, weights, out_dir):
    """Gives `body` its regions (both forms) and returns the manifest's
    list ([{id, label, map_index, triangles}]) and each polygon's index."""
    face_regions = of_faces(body, sets, P, joints, weights)
    write_layer(body, face_regions)
    write_map(body, face_regions, out_dir)
    counts = np.zeros(len(REGIONS) + 1, dtype=np.int64)
    for poly, region in zip(body.data.polygons, face_regions):
        counts[region] += len(poly.vertices) - 2
    assert counts[0] == 0, "a body triangle has no region"
    return [{"id": name, "label": label, "map_index": k + 1, "triangles": int(counts[k + 1])}
            for k, (name, label) in enumerate(REGIONS)], face_regions
