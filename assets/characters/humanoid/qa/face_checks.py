"""Checks a face control by control (Character Studio overhaul CO6): for
every standard facial control at full weight, every viseme, every built-in
expression and every face identity parameter's two ends, on both frames:
does the eyeball come through a lid, do the teeth or the tongue come
through the lips, and does a full blink close the eye.

    python -I face_checks.py --package realistic --out FILE [--check] [--only TEXT]

(`python` has the `bpy` module, as for `measure.py`.) It poses the parts
file as the engine does: a control's blend shape on the body and on every
part that has a shape of that name (parts follow the body's live shapes),
the jaw and eye bones turned by the facial profile's limits, the frame's
bone offsets and eye scale.

What is measured, per pose (millimetres; counts are vertices or rays):

- `lid_in_eyeball`: skin vertices under the surface of the opaque eyeball
  (materials `Sclera` and `Iris`), by a ray from the eye's centre through
  the vertex: the white or the iris would show through the lid there. `lid_in_cornea` is the same against the clear
  cornea standing above the white, where the eyeball has one.
- `eyeball_in_lid`: vertices of the opaque eyeball's front inside the
  lids' flesh. "Inside the flesh" is inside the body's closed surface (a
  ray from the point crosses it an odd number of times) and more than a
  margin from it: 0.2 mm for the eyeball, 0.5 mm for the teeth.
- `open_mm`: the height of the eyeball left bare between the lids, per eye
  (the largest over the eye's width), by rays from the eye's centre.
  `bare_rays` counts them; a closed eye has none.
- `teeth_in_flesh`, `tongue_in_flesh`: vertices of the teeth and gums, and
  of the tongue, inside the body's flesh by more than 0.5 mm.
- `lips_in_teeth`: skin and lining vertices inside the teeth or gums.
- `lining_through_skin`: pairs of triangles, one of the mouth's lining
  (material `Mouth`) and one of the skin, that cross, more than 8 mm from
  the lips' edges (where closed lips touch): the lining would show
  outside the face.
- `self_crossing`: pairs of the head's triangles (those wholly above the
  base of the neck at the frame's rest, the same in every pose) that
  cross, less the resting face's own.
- `mouth_asymmetry_mm`, `teeth_asymmetry_mm`, `tongue_asymmetry_mm`: how
  far the mouth's skin (the vertices `Mouth_Pucker` moves by more than
  half a millimetre), the teeth and gums, and the tongue are from their
  own mirror images across the body's middle: the furthest any vertex's
  mirror image lies from the nearest vertex. Measured in poses that are
  the same on both sides (rest, the controls that have no left and
  right, each left and right pair together, the visemes, and the
  expressions whose recipes are even). The gap between closed lips is a
  tenth of a millimetre: half a millimetre's difference between the two
  halves is a dark line on one side of the mouth and none on the other.
- `lips_open_mm`, `seal_in_view`: how far the lips stand apart at the
  middle of the face (the height through which rays from the front pass
  between the upper lip's skin and the lower lip's; resting lips overlap
  and measure under a millimetre), and how many vertices of `LipSeal`
  (the flaps that hide the teeth behind lips a hair apart) are then out
  of the body's flesh, where they can be seen. With the lips more than
  `SEAL_GONE_MM` apart a flap in view is a line round the open mouth.

`--check` exits 1 unless: no control, viseme or expression at full weight
puts a lid in the opaque eyeball or the eyeball in a lid, or the teeth or
tongue in the flesh, or the lips in the teeth, or the mouth's lining
through the skin; the half-way blinks keep
the lids out of the opaque eyeball; `Blink` and `Eye_Closed` leave no
ray bare; and in every pose that is the same on both sides the mouth, the
teeth and the tongue are within `EVEN_MM` of their mirror images, on both
frames; and no vertex of `LipSeal` is in view with the lips more than
`SEAL_GONE_MM` apart. The tongue controls are posed with the jaw
half open: a tongue doesn't leave a closed mouth. The identity parameters
in `HELD_IDENTITY` (R4's) are held to the same at both ends: alone, in a
full blink (no ray bare) and under every expression and viseme
(`identity_faces`); the others' ends are reported. Those parameters'
ends also make no pair of the head's skin triangles cross that doesn't
cross at rest, alone or on top of any other face parameter's end
(`identity_pairs`: the pairs that cross with both, less those that cross
with the other alone or at rest; on top of an older parameter's end that
crosses on its own, reported). Unlike `measure.py`'s sliders check,
this counts the pairs that lie within 4 mm at rest: the upper lid's
crease lies on itself there, and its fold once came down through it.
"""

import argparse
import json
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qa_scene as Q  # noqa: E402

REPO = os.path.abspath(os.path.join(Q.LIBRARY_DIR, "..", ".."))
FACES = os.path.join(REPO, "crates", "anim", "faces")
# The facial profile's default limits (crates/assets/src/face_profile.rs).
LIMITS = {"look_sideways": 30.0, "look_up": 25.0, "look_down": 20.0, "jaw_open": 25.0, "jaw_sideways": 8.0}
CONTROLS = [
    "Blink_Left", "Blink_Right", "Eye_Closed_Left", "Eye_Closed_Right", "Eye_Squint_Left", "Eye_Squint_Right",
    "Eye_Wide_Left", "Eye_Wide_Right", "Eye_Look_Up", "Eye_Look_Down", "Eye_Look_Left", "Eye_Look_Right",
    "Brow_Raise_Left", "Brow_Raise_Right", "Brow_Lower_Left", "Brow_Lower_Right", "Brow_Inward_Left",
    "Brow_Inward_Right", "Mouth_Smile_Left", "Mouth_Smile_Right", "Mouth_Frown_Left", "Mouth_Frown_Right",
    "Mouth_Open", "Mouth_Wide", "Mouth_Narrow", "Mouth_Pucker", "Mouth_Left", "Mouth_Right", "Upper_Lip_Raise",
    "Lower_Lip_Lower", "Upper_Lip_Raise_Left", "Upper_Lip_Raise_Right", "Lower_Lip_Lower_Left",
    "Lower_Lip_Lower_Right", "Lip_Corner_Pull_Left", "Lip_Corner_Pull_Right", "Cheek_Raise_Left",
    "Cheek_Raise_Right", "Cheek_Puff", "Nose_Sneer_Left", "Nose_Sneer_Right", "Jaw_Open", "Jaw_Forward",
    "Jaw_Left", "Jaw_Right", "Tongue_Out", "Tongue_Up", "Tongue_Down",
]
TONGUE = ("Tongue_Out", "Tongue_Up", "Tongue_Down")
# The jaw's opening the tongue controls are posed with.
TONGUE_JAW = 0.5
OPAQUE = ("Sclera", "Iris")
# In a pose that is the same on both sides, no vertex of the mouth, the
# teeth or the tongue is further than this from a mirror image (millimetres).
EVEN_MM = 0.1
# The left and right controls checked together as pairs.
PAIRS = ("Mouth_Smile", "Mouth_Frown", "Lip_Corner_Pull", "Upper_Lip_Raise", "Lower_Lip_Lower", "Nose_Sneer",
         "Cheek_Raise")
# Crossings of the mouth's lining and the skin this near the lips' edges
# are the closed lips touching, not the lining coming through the face.
LIPS_EDGE = 0.008
# With the lips further apart than this at the middle (millimetres), no
# part of `LipSeal` may be in view.
SEAL_GONE_MM = 6.0
# The face identity parameters held to the checks at their ends, alone, in
# a full blink and with every expression and viseme on top (R4's controls,
# by id or id stem; the older parameters' ends are reported only, as
# before: some of them are not clean, `co6/README.md`).
HELD_IDENTITY = ("nose_tip_projection", "nose_tip_rotation", "nose_width", "philtrum_length", "jaw_angle",
                 "upper_lid_fold", "lower_lid", "eye_corner", "earlobe")


def head_triangles(piece, ch):
    """Which of `piece`'s triangles are the head's: those wholly above the
    base of the neck (the `Neck` bone's head) in the pose the character is
    in, a frame's rest. Chosen once and kept for every pose of the frame:
    a line drawn through the posed face let a pose move skin across it.
    (It was 20 mm under the `Head` bone, which is 5 mm over the masculine
    mouth's corners: the philtrum's and the mouth's height's ends lifted
    the corners' resting contact over it, where it counted as new.)"""
    return np.where(piece.co[piece.tris][:, :, 2].min(axis=1) > ch.head("Neck").z)[0]


def head_crossings(ch, keep):
    """The pairs of the head's posed skin triangles (`keep`, from
    `head_triangles` at the frame's rest) that cross and share no vertex,
    as pairs of their numbers. (Of the skin: the mouth's lining lies
    folded on itself inside the mouth, where no one sees it, and its
    crossing the skin is `lining_through_skin`.)"""
    body = Piece(ch.parts["Body"], ("Skin",))
    tris = body.tris[keep]
    tree = BVHTree.FromPolygons([tuple(p) for p in body.co], [tuple(int(i) for i in t) for t in tris], all_triangles=True)
    return {(int(keep[i]), int(keep[j])) for i, j in tree.overlap(tree) if i < j and not set(tris[i]) & set(tris[j])}


def identity_pairs(ch, frame, parameters, shapes, only=""):
    """Each `HELD_IDENTITY` parameter's ends alone and on top of every
    other face parameter's ends (of other parameters): how many pairs of
    the head's skin triangles cross that cross neither at rest nor with the
    other parameter alone. {"poses": n, "offenders": [{"sliders",
    "new_crossings", "required"}]}: held (`required`) alone and on top of
    an end that crosses nothing of its own; on top of an end that already
    crosses (an older parameter's, reported in `identity`), reported.

    Left out, as `measure.py`'s sliders check leaves them out: pairs that
    touch at rest (within 4 mm) at the closed lips (within `LIPS_EDGE`
    of `LipSeal`), where any move makes or unmakes a crossing. Elsewhere,
    the lids' crease among them, every pair counts. The head's triangles
    are the same in every pose (`head_triangles`, at the frame's rest):
    the philtrum's and the mouth's height's ends together once lifted the
    corners' resting contact (four pairs crossing by 0.03 mm in every
    pose) over a line drawn through the posed mouth, where it counted as
    new (wave 4)."""
    def identity(*ends):
        out = {}
        for drivers, end in ends:
            for shape, value in drivers(end).items():
                if shape in shapes:
                    out[shape] = out.get(shape, 0.0) + value
        return out

    pose(ch, frame, {})
    skin = Piece(ch.parts["Body"], ("Skin",))
    head = head_triangles(skin, ch)
    rest = head_crossings(ch, head)
    seal = Piece(ch.parts["LipSeal"]).co if "LipSeal" in ch.parts else np.zeros((0, 3))

    def at_lips(pair):
        a, b = (skin.co[skin.tris[t]] for t in pair)
        if np.linalg.norm(a[:, None] - b[None], axis=2).min() > 0.004 or not len(seal):
            return False
        return all(np.linalg.norm(seal - t.mean(axis=0), axis=1).min() < LIPS_EDGE for t in (a, b))

    def counted(pairs):
        return sum(1 for pair in pairs if not at_lips(pair))

    alone = {}
    for pid, drivers in parameters:
        for end in (-1.0, 1.0):
            pose(ch, frame, {}, identity((drivers, end)))
            alone[(pid, end)] = head_crossings(ch, head)
    poses, offenders = 0, []
    for pid, drivers in parameters:
        if not held_identity(pid):
            continue
        for end in (-1.0, 1.0):
            name = f"{pid}{end:+.0f}"
            if only and only not in name:
                continue
            new = counted(alone[(pid, end)] - rest)
            poses += 1
            if new:
                offenders.append({"sliders": name, "new_crossings": new, "required": True})
            for other, other_drivers in parameters:
                if other == pid:
                    continue
                for other_end in (-1.0, 1.0):
                    pose(ch, frame, {}, identity((drivers, end), (other_drivers, other_end)))
                    new = counted(head_crossings(ch, head) - alone[(other, other_end)] - rest)
                    poses += 1
                    if new:
                        clean = not counted(alone[(other, other_end)] - rest)
                        offenders.append({"sliders": f"{name}, {other}{other_end:+.0f}", "new_crossings": new,
                                          "required": clean})
    return {"poses": poses, "offenders": offenders}


def held_identity(pid):
    return any(pid == stem or pid in (f"{stem}_left", f"{stem}_right") for stem in HELD_IDENTITY)


def mm(x):
    return round(float(x) * 1000.0, 2)


class Piece:
    """Some of an object's posed triangles (those of the named materials)."""

    def __init__(self, obj, materials=None):
        dg = bpy.context.evaluated_depsgraph_get()
        ev = obj.evaluated_get(dg)
        me = ev.to_mesh()
        M = np.array(obj.matrix_world)
        co = np.array([v.co[:] for v in me.vertices]) @ M[:3, :3].T + M[:3, 3]
        names = [m.name.split(".")[0] if m else "" for m in me.materials]
        me.calc_loop_triangles()
        keep = [t for t in me.loop_triangles
                if materials is None or (names and names[me.polygons[t.polygon_index].material_index] in materials)]
        tris = np.array([tuple(t.vertices) for t in keep], dtype=np.int64).reshape(-1, 3)
        used = np.unique(tris)
        remap = np.full(len(co), -1, dtype=np.int64)
        remap[used] = np.arange(len(used))
        self.co = co[used]
        self.tris = remap[tris]
        ev.to_mesh_clear()
        self.bvh = (BVHTree.FromPolygons([tuple(p) for p in self.co], [tuple(int(i) for i in t) for t in self.tris],
                                         all_triangles=True) if len(self.tris) else None)

    def beyond(self, centre, p):
        """How far `p` is inside this piece seen from `centre`: the
        length of the ray from `centre` through `p` that lies past `p`
        before it leaves the piece (0 when `p` is outside it). For a
        piece that wraps `centre` (an eyeball about its own middle),
        whether or not it is closed."""
        if self.bvh is None:
            return 0.0
        way = Vector(p) - Vector(centre)
        length = way.length
        if length < 1e-9:
            return 0.0
        hit = self.bvh.ray_cast(Vector(centre), way / length, 0.03)
        if hit[0] is None:
            return 0.0
        return max(0.0, hit[3] - length)

    def inside(self, p):
        """Whether `p` is inside this piece, which must be closed (the
        body is): a ray from `p` crosses its surface an odd number of
        times. Three rays vote, as one can graze an edge. A ray goes on
        from each hit a micrometre past it, and at a grazing angle that
        can land back on the triangle it just crossed, which the tree
        then reports again; a ray crosses a flat triangle once, so a hit
        on the same triangle as the last is not counted. (Two of the three
        rays counted one triangle twice at a tooth 4 mm behind the lower
        lip, R9's wave 4, and called it in the flesh.)"""
        votes = 0
        for way in (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60))):
            way.normalize()
            at, crossings, last = Vector(p), 0, None
            for _ in range(64):
                hit = self.bvh.ray_cast(at, way, 3.0)
                if hit[0] is None:
                    break
                if hit[2] != last:
                    crossings += 1
                last = hit[2]
                at = hit[0] + way * 1e-6
            votes += crossings % 2
        return votes >= 2

    def sunk(self, p, reach):
        """How deep `p` is inside this closed piece: its distance to the
        surface when it is inside, else 0."""
        loc, _, _, distance = self.bvh.find_nearest(Vector(p), reach)
        if loc is None or not self.inside(p):
            return 0.0
        return distance

    def open_edges(self):
        """Edges with one face only: 0 for a closed piece."""
        count = {}
        for t in self.tris:
            for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                key = (min(a, b), max(a, b))
                count[key] = count.get(key, 0) + 1
        return sum(1 for n in count.values() if n == 1)

    def depth(self, p, reach):
        """How far `p` is behind this piece's nearest face (0 in front of
        it or out of reach). For an open piece (the teeth and gums)."""
        if self.bvh is None:
            return 0.0
        loc, normal, _, _ = self.bvh.find_nearest(Vector(p), reach)
        if loc is None:
            return 0.0
        return max(0.0, -(Vector(p) - loc).dot(normal))


def has_material(obj, name):
    return any(m and m.name.split(".")[0] == name for m in obj.data.materials)


def pose(ch, frame, weights, identity=None):
    """The character in `frame` with control `weights` ({control: 0..1})
    and identity shapes: shapes on every part, the jaw and eye bones
    turned."""
    ch.set_look(frame=frame, face=identity or {}, expr=weights)
    ch.pose_reset()
    w = lambda name: weights.get(name, 0.0)  # noqa: E731
    if w("Jaw_Open"):
        ch.bend("Jaw", (1, 0, 0), LIMITS["jaw_open"] * w("Jaw_Open"), toward=Q.DOWN)
    side = w("Jaw_Left") - w("Jaw_Right")
    if side:
        ch.bend("Jaw", (0, 0, 1), LIMITS["jaw_sideways"] * abs(side), toward=Vector((1 if side > 0 else -1, 0, 0)))
    yaw = LIMITS["look_sideways"] * (w("Eye_Look_Left") - w("Eye_Look_Right"))
    pitch = LIMITS["look_up"] * w("Eye_Look_Up") - LIMITS["look_down"] * w("Eye_Look_Down")
    for side, _ in Q.SIDES:
        eye = f"{side}Eye"
        if eye not in ch.arm.pose.bones or not (yaw or pitch):
            continue
        # About the eye's own centre in this look (the frame and the
        # identity move it from where the bone rests): left is a positive
        # turn about up, up a positive turn about the body's left (+X)
        # for an eye that looks along -Y.
        centre = ch.eye_centre(side)
        turn = Matrix.Rotation(math.radians(yaw), 4, "Z") @ Matrix.Rotation(math.radians(-pitch), 4, "X")
        about = Matrix.Translation(centre) @ turn @ Matrix.Translation(-centre)
        rest = ch.arm.data.bones[eye].matrix_local
        pb = ch.arm.pose.bones[eye]
        pb.matrix_basis = (rest.inverted() @ about @ rest) @ pb.matrix_basis
    Q.update()


def even_weights(weights):
    """Whether a pose's controls are the same on both sides: every left
    control has its right one at the same weight."""
    for name, w in weights.items():
        for side, other in (("Left", "Right"), ("Right", "Left")):
            if name.endswith(side) and abs(weights.get(name[:-len(side)] + other, 0.0) - w) > 1e-9:
                return False
    return True


def asymmetry(co):
    """The furthest any of the points' mirror images (across x = 0) lies
    from the nearest of the points, in metres."""
    from mathutils.kdtree import KDTree
    if not len(co):
        return 0.0
    tree = KDTree(len(co))
    for i, p in enumerate(co):
        tree.insert(p, i)
    tree.balance()
    return max(tree.find((-p[0], p[1], p[2]))[2] for p in co)


def mouth_vertices(ch, frame):
    """Which of the body's posed vertices are the mouth's skin: those
    `Mouth_Pucker` moves by more than half a millimetre."""
    pose(ch, frame, {})
    rest = Piece(ch.parts["Body"]).co
    pose(ch, frame, {"Mouth_Pucker": 1.0})
    moved = Piece(ch.parts["Body"]).co
    return np.where(np.linalg.norm(moved - rest, axis=1) > 0.0005)[0]


def lips_open(skin, seal_rest):
    """How far the lips stand apart at the middle of the face, in metres:
    rays from the front down the middle, from the upper lip's red
    downward; the height of those that pass under the upper lip's skin
    before one meets the lower lip's. `seal_rest` is where the lips' line
    rests. (The lips overlap as they rest, the lower one's top in front
    of the upper one's edge, so resting lips measure some tenths of a
    millimetre.)"""
    y, z0 = float(seal_rest[:, 1].min()) - 0.05, float(seal_rest[:, 2].mean()) + 0.008
    step, reach = 0.0001, 0.11
    lip, gap = None, 0
    for k in range(700):
        hit = skin.bvh.ray_cast(Vector((0.0, y, z0 - k * step)), Vector((0.0, 1.0, 0.0)), reach)
        depth = None if hit[0] is None else float(hit[3])
        if lip is None:
            lip = depth
        if depth is not None and (gap == 0 and depth < (lip or 0.0) + 0.006):
            continue
        if depth is not None:
            break
        gap += 1
    return step * gap


def measure(ch, head, rest_crossing=None, mouth_skin=None, seal_rest=None):
    """The measures of the pose the character is in (`head`: the body's
    head triangles, `head_triangles` at the frame's rest; with `mouth_skin`,
    the mouth's vertices: how far it and the teeth are from even; with
    `seal_rest`, the resting `LipSeal`: how far the lips are apart and
    how much of it is in view)."""
    body = Piece(ch.parts["Body"])
    eyes_obj, teeth_obj = ch.parts["Eyes"], ch.parts["Teeth"]
    opaque = Piece(eyes_obj, OPAQUE)
    cornea = Piece(eyes_obj, ("Cornea",)) if has_material(eyes_obj, "Cornea") else None
    teeth = Piece(teeth_obj, ("Teeth", "Gums"))
    tongue = Piece(teeth_obj, ("Tongue",)) if has_material(teeth_obj, "Tongue") else None
    out = {}
    lid_in, lid_in_max, cornea_in, cornea_max, eye_in, eye_in_max = 0, 0.0, 0, 0.0, 0, 0.0
    open_mm, bare = [], 0
    for side, s in Q.SIDES:
        c = np.array(ch.eye_centre(side))
        mine = opaque.co[opaque.co[:, 0] * s > 0]
        r = float(np.median(np.linalg.norm(mine - c, axis=1)))
        near = np.where(np.linalg.norm(body.co - c, axis=1) < r * 1.7)[0]
        for i in near:
            d = opaque.beyond(c, body.co[i])
            if d > 0.0002:
                lid_in += 1
                lid_in_max = max(lid_in_max, d)
            elif cornea is not None:
                d = cornea.beyond(c, body.co[i])
                if d > 0.0002:
                    cornea_in += 1
                    cornea_max = max(cornea_max, d)
        front = mine[-(mine - c)[:, 1] / np.maximum(np.linalg.norm(mine - c, axis=1), 1e-9) > math.cos(math.radians(60))]
        for p in front:
            d = body.sunk(p, 0.03)
            if d > 0.0002:
                eye_in += 1
                eye_in_max = max(eye_in_max, d)
        # Rays from the eye's centre: which leave between the lids.
        tallest = 0.0
        step = 0.5
        for az in np.arange(-48.0, 48.1, 3.0):
            column = 0
            for el in np.arange(-50.0, 50.1, step):
                a, e = math.radians(az), math.radians(el)
                way = Vector((math.sin(a) * math.cos(e), -math.cos(a) * math.cos(e), math.sin(e)))
                if body.bvh.ray_cast(Vector(c) + way * (r * 0.5), way, 0.06)[0] is None:
                    column += 1
            bare += column
            tallest = max(tallest, column * math.radians(step) * r)
        open_mm.append(mm(tallest))
    out["lid_in_eyeball"] = lid_in
    out["lid_in_eyeball_max_mm"] = mm(lid_in_max)
    out["lid_in_cornea"] = cornea_in
    out["lid_in_cornea_max_mm"] = mm(cornea_max)
    out["eyeball_in_lid"] = eye_in
    out["eyeball_in_lid_max_mm"] = mm(eye_in_max)
    out["open_mm"] = open_mm
    out["bare_rays"] = bare
    for key, piece in (("teeth", teeth), ("tongue", tongue)):
        deep = [body.sunk(p, 0.02) for p in piece.co] if piece is not None else []
        deep = [d for d in deep if d > 0.0005]
        out[f"{key}_in_flesh"] = len(deep)
        out[f"{key}_in_flesh_max_mm"] = mm(max(deep)) if deep else 0.0
    mouth = teeth.co.mean(axis=0)
    close = np.where(np.linalg.norm(body.co - mouth, axis=1) < 0.07)[0]
    deep = [teeth.depth(body.co[i], 0.004) for i in close]
    deep = [d for d in deep if d > 0.0002]
    out["lips_in_teeth"] = len(deep)
    out["lips_in_teeth_max_mm"] = mm(max(deep)) if deep else 0.0
    if seal_rest is not None:
        out["lips_open_mm"] = mm(lips_open(Piece(ch.parts["Body"], ("Skin",)), seal_rest))
        out["seal_in_view"] = sum(1 for p in Piece(ch.parts["LipSeal"]).co if not body.inside(p))
    if mouth_skin is not None:
        out["mouth_asymmetry_mm"] = mm(asymmetry(body.co[mouth_skin]))
        out["teeth_asymmetry_mm"] = mm(asymmetry(teeth.co))
        out["tongue_asymmetry_mm"] = mm(asymmetry(tongue.co)) if tongue is not None else 0.0
    # The mouth's lining through the face's skin: pairs of their triangles
    # that cross, away from the lips' own edges (closed lips touch there).
    lining = Piece(ch.parts["Body"], ("Mouth",))
    crossed = 0
    if lining.bvh is not None:
        skin = Piece(ch.parts["Body"], ("Skin",))
        near = skin.co[np.linalg.norm(skin.co - lining.co.mean(axis=0), axis=1) < 0.12]
        seen = {tuple(np.round(p, 6)) for p in near}
        edge = np.array([p for p in lining.co if tuple(np.round(p, 6)) in seen])
        for i, j in lining.bvh.overlap(skin.bvh):
            a, b = lining.co[lining.tris[i]], skin.co[skin.tris[j]]
            if np.linalg.norm(a[:, None, :] - b[None, :, :], axis=2).min() <= 1e-6:
                continue
            if len(edge) and np.linalg.norm(edge - a.mean(axis=0), axis=1).min() < LIPS_EDGE:
                continue
            crossed += 1
    out["lining_through_skin"] = crossed
    tris = body.tris[head]
    tree = BVHTree.FromPolygons([tuple(p) for p in body.co], [tuple(int(i) for i in t) for t in tris], all_triangles=True)
    crossing = sum(1 for i, j in tree.overlap(tree) if i < j and not set(tris[i]) & set(tris[j]))
    out["self_crossing"] = crossing if rest_crossing is None else crossing - rest_crossing
    return out, crossing


def visemes():
    """Each viseme's controls as the engine's lip sync gives them (the
    built-in viseme map, its jaw share moved from Mouth_Open to Jaw_Open)."""
    with open(os.path.join(FACES, "visemes", "default.visemes.json"), encoding="utf-8") as f:
        data = json.load(f)
    share = max(0.0, min(1.0, data.get("jaw_share", 0.0)))
    out = {}
    for name, values in sorted(data["visemes"].items()):
        w = dict(values)
        if share and "Mouth_Open" in w:
            opened = w["Mouth_Open"]
            w["Mouth_Open"] = opened * (1.0 - share)
            w["Jaw_Open"] = min(1.0, w.get("Jaw_Open", 0.0) + opened * share)
        out[name] = w
    return out


def expressions():
    out = {}
    folder = os.path.join(FACES, "expressions")
    for file in sorted(os.listdir(folder)):
        with open(os.path.join(folder, file), encoding="utf-8") as f:
            out[file.split(".")[0]] = json.load(f)["controls"]
    return out


def poses(shapes, parameters):
    """(group, name, control weights, identity shapes, required) for
    every pose checked."""
    out = [("rest", "neutral", {}, {}, True)]
    for control in CONTROLS:
        w = {control: 1.0}
        if control in TONGUE:
            w["Jaw_Open"] = TONGUE_JAW
        out.append(("controls", control, w, {}, True))
    for name in PAIRS:
        out.append(("pairs", name, {f"{name}_Left": 1.0, f"{name}_Right": 1.0}, {}, True))
    for name in ("Blink", "Eye_Closed"):
        both = {f"{name}_Left": 1.0, f"{name}_Right": 1.0}
        out.append(("closed", name, both, {}, True))
    for share in (0.25, 0.5, 0.75):
        out.append(("part_blink", f"Blink_{share:g}", {"Blink_Left": share, "Blink_Right": share}, {}, True))
    for name, w in visemes().items():
        out.append(("visemes", name, w, {}, True))
    for name, w in expressions().items():
        out.append(("expressions", name, w, {}, True))
    # Identity: each face parameter's two ends, at rest and in a blink
    # (reported; a blink on a reshaped eye is not held to zero), but for
    # the `HELD_IDENTITY` ones: held, and also with every expression and
    # viseme on top (`identity_faces`).
    blink = {"Blink_Left": 1.0, "Blink_Right": 1.0}
    faces = list(expressions().items()) + [(f"viseme {name}", w) for name, w in visemes().items()]
    for pid, drivers in parameters:
        held = held_identity(pid)
        for end in (-1.0, 1.0):
            identity = {}
            for shape, value in drivers(end).items():
                if shape in shapes:
                    identity[shape] = value
            if identity:
                out.append(("identity", f"{pid}{end:+.0f}", {}, identity, held))
                out.append(("identity_blink", f"{pid}{end:+.0f}", blink, identity, held))
                if held:
                    for name, w in faces:
                        out.append(("identity_faces", f"{pid}{end:+.0f} {name}", w, identity, True))
    return out


def face_parameters(manifest_path):
    """The manifest's face parameters: (id, value -> {shape: weight})."""
    if not os.path.exists(manifest_path):
        return []
    with open(manifest_path, encoding="utf-8") as f:
        manifest = json.load(f)
    face_groups = {g["id"] for g in manifest.get("groups", []) if g.get("category") == "face"}
    out = []

    def span(v, a, b):
        t = max(0.0, min(1.0, (v - a[0]) / (a[1] - a[0])))
        return b[0] + (b[1] - b[0]) * t

    for p in manifest.get("parameters", []):
        if p.get("group") not in face_groups or not p.get("shapes"):
            continue
        lo, hi = p["range"]

        def drivers(end, p=p, lo=lo, hi=hi):
            v = lo if end < 0 else hi
            res = {}
            for d in p["shapes"]:
                a = d.get("from", [0, 1])
                b = d.get("to", a)
                res[d["shape"]] = res.get(d["shape"], 0.0) + span(v, a, b)
            return {k: w for k, w in res.items() if abs(w) > 1e-9}

        out.append((p["id"], drivers))
    return out


ZERO = ("lid_in_eyeball", "eyeball_in_lid", "teeth_in_flesh", "tongue_in_flesh", "lips_in_teeth",
        "lining_through_skin")


def failures(report):
    bad = []
    for frame, groups in report["frames"].items():
        for group, entries in groups.items():
            for name, m in entries.items():
                if not m.get("required"):
                    continue
                keys = ("lid_in_eyeball",) if group == "part_blink" else ZERO
                for key in keys:
                    if m[key]:
                        deep = m.get(key + "_max_mm")
                        bad.append(f"{frame}/{group}/{name}: {key} = {m[key]}" + (f" (to {deep} mm)" if deep else ""))
                for key in ("mouth_asymmetry_mm", "teeth_asymmetry_mm", "tongue_asymmetry_mm"):
                    if m.get(key, 0.0) > EVEN_MM:
                        bad.append(f"{frame}/{group}/{name}: {key} = {m[key]} (at most {EVEN_MM})")
                if m.get("lips_open_mm", 0.0) > SEAL_GONE_MM and m.get("seal_in_view"):
                    bad.append(f"{frame}/{group}/{name}: {m['seal_in_view']} vertices of LipSeal in view with the lips "
                               f"{m['lips_open_mm']} mm apart")
                if group in ("closed", "identity_blink") and m["bare_rays"]:
                    bad.append(f"{frame}/{group}/{name}: the eye isn't closed: {m['bare_rays']} rays bare, "
                               f"open {m['open_mm']} mm")
        for name in ("Blink", "Eye_Closed"):
            if name not in groups.get("closed", {}):
                bad.append(f"{frame}: no {name} pose was measured")
    for frame, pairs in report.get("identity_pairs", {}).items():
        for o in (o for o in pairs["offenders"] if o["required"]):
            bad.append(f"{frame}/identity_pairs/{o['sliders']}: {o['new_crossings']} new pairs of the head's skin "
                       "triangles cross")
    return bad


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="realistic", choices=sorted(Q.PACKAGES))
    ap.add_argument("--parts", default="")
    ap.add_argument("--custom", default="")
    ap.add_argument("--out", default="face_checks.json")
    ap.add_argument("--check", action="store_true")
    # Only the poses whose name has this text (and rest, which the others
    # are measured against): for a quick look at one control.
    ap.add_argument("--only", default="")
    a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
    parts_path, _, custom_path = Q.package_paths(a.package)
    a.parts = a.parts or parts_path
    custom = Q.load_custom(a.custom or custom_path)
    # The eye bones' share of the face identity (the build data lists it;
    # `load_custom` reads only the frame's).
    with open(a.custom or custom_path, encoding="utf-8") as f:
        custom["face"] = json.load(f).get("face", {}).get("face_identity", custom.get("face", {}))
    Q.reset()
    ch = Q.Character(a.parts, merge_vertices=True, custom=custom)
    keys = ch.parts["Body"].data.shape_keys
    shapes = {kb.name for kb in keys.key_blocks} if keys else set()
    manifest = os.path.join(os.path.dirname(a.parts), "Realistic.base.json" if a.package == "realistic" else "Humanoid.base.json")
    parameters = face_parameters(manifest)
    listed = poses(shapes, parameters)
    if a.only:
        listed = [p for p in listed if p[0] == "rest" or a.only in p[1]]
    pose(ch, 0.0, {})
    report = {"file": os.path.basename(a.parts), "limits_deg": LIMITS, "tongue_posed_with_jaw_open": TONGUE_JAW,
              "body_open_edges": Piece(ch.parts["Body"]).open_edges(),
              "controls_with_a_body_shape": sorted(shapes & set(CONTROLS)), "frames": {}}
    for frame_name, frame in (("masculine", 0.0), ("feminine", 1.0)):
        groups = {}
        rest_crossing = None
        mouth = mouth_vertices(ch, frame)
        pose(ch, frame, {})
        seal_rest = Piece(ch.parts["LipSeal"]).co if "LipSeal" in ch.parts else None
        head = head_triangles(Piece(ch.parts["Body"]), ch)
        for group, name, weights, identity, required in listed:
            pose(ch, frame, weights, identity)
            # (Evenness: of the poses that are even, and not of the
            # identity's, whose ends are a face's own.)
            even = required and not identity and even_weights(weights)
            m, crossing = measure(ch, head, rest_crossing, mouth if even else None, seal_rest)
            if rest_crossing is None:
                rest_crossing = crossing
                m["self_crossing_at_rest"] = crossing
                m["self_crossing"] = 0
            m["required"] = required
            groups.setdefault(group, {})[name] = m
        report["frames"][frame_name] = groups
        report.setdefault("identity_pairs", {})[frame_name] = identity_pairs(ch, frame, parameters, shapes, a.only)
        print(f"{frame_name}: identity_pairs {report['identity_pairs'][frame_name]['poses']} poses, "
              f"{len(report['identity_pairs'][frame_name]['offenders'])} with new crossings", flush=True)
    bad = failures(report)
    report["failures"] = bad
    with open(a.out, "w", encoding="utf-8", newline="\n") as f:
        json.dump(report, f, indent=1, sort_keys=True)
        f.write("\n")
    for frame, groups in report["frames"].items():
        rest = groups["rest"]["neutral"]
        closed = groups.get("closed", {}).get("Blink", {})
        print(f"{frame}: at rest open {rest['open_mm']} mm; full blink open {closed.get('open_mm')} mm, "
              f"{closed.get('bare_rays')} rays bare")
    print(f"{len(bad)} failures" + (":" if bad else ""))
    for line in bad[:60]:
        print("  " + line)
    if len(bad) > 60:
        print(f"  ... and {len(bad) - 60} more (in {os.path.basename(a.out)})")
    if a.check and bad:
        sys.exit(1)


if __name__ == "__main__":
    main()
