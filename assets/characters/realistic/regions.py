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
]
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
            kind = "upper_arms_top" if along(side + "UpperArm", c) < 0.5 else "upper_arms_low"
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
