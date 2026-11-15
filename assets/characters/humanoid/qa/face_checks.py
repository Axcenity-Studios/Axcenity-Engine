"""Checks a face control by control (Character Studio overhaul CO6): for
every standard facial control at full weight, every viseme, every built-in
expression and every face identity parameter's two ends, on both frames:
does the eyeball come through a lid, do the teeth or the tongue come
through the lips, and does a full blink close the eye.

    python -I face_checks.py --package realistic --out FILE [--check]

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
- `self_crossing`: pairs of the head's triangles that cross, less the
  resting face's own.

`--check` exits 1 unless: no control, viseme or expression at full weight
puts a lid in the opaque eyeball or the eyeball in a lid, or the teeth or
tongue in the flesh, or the lips in the teeth, or the mouth's lining
through the skin; the half-way blinks keep
the lids out of the opaque eyeball; and `Blink` and `Eye_Closed` leave no
ray bare, on both frames. The tongue controls are posed with the jaw
half open: a tongue doesn't leave a closed mouth.
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
# Crossings of the mouth's lining and the skin this near the lips' edges
# are the closed lips touching, not the lining coming through the face.
LIPS_EDGE = 0.008


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
        times. Three rays vote, as one can graze an edge."""
        votes = 0
        for way in (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60))):
            way.normalize()
            at, crossings = Vector(p), 0
            for _ in range(64):
                hit = self.bvh.ray_cast(at, way, 3.0)
                if hit[0] is None:
                    break
                crossings += 1
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


def measure(ch, rest_crossing=None):
    """The measures of the pose the character is in."""
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
    z_neck = ch.head("Head").z - 0.02
    tris = body.tris[body.co[body.tris][:, :, 2].min(axis=1) > z_neck]
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
    # (reported; a blink on a reshaped eye is not held to zero).
    blink = {"Blink_Left": 1.0, "Blink_Right": 1.0}
    for pid, drivers in parameters:
        for end in (-1.0, 1.0):
            identity = {}
            for shape, value in drivers(end).items():
                if shape in shapes:
                    identity[shape] = value
            if identity:
                out.append(("identity", f"{pid}{end:+.0f}", {}, identity, False))
                out.append(("identity_blink", f"{pid}{end:+.0f}", blink, identity, False))
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
                if group == "closed" and m["bare_rays"]:
                    bad.append(f"{frame}/{group}/{name}: the eye isn't closed: {m['bare_rays']} rays bare, "
                               f"open {m['open_mm']} mm")
        for name in ("Blink", "Eye_Closed"):
            if name not in groups.get("closed", {}):
                bad.append(f"{frame}: no {name} pose was measured")
    return bad


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="realistic", choices=sorted(Q.PACKAGES))
    ap.add_argument("--parts", default="")
    ap.add_argument("--custom", default="")
    ap.add_argument("--out", default="face_checks.json")
    ap.add_argument("--check", action="store_true")
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
    listed = poses(shapes, face_parameters(manifest))
    pose(ch, 0.0, {})
    report = {"file": os.path.basename(a.parts), "limits_deg": LIMITS, "tongue_posed_with_jaw_open": TONGUE_JAW,
              "body_open_edges": Piece(ch.parts["Body"]).open_edges(),
              "controls_with_a_body_shape": sorted(shapes & set(CONTROLS)), "frames": {}}
    for frame_name, frame in (("masculine", 0.0), ("feminine", 1.0)):
        groups = {}
        rest_crossing = None
        for group, name, weights, identity, required in listed:
            pose(ch, frame, weights, identity)
            m, crossing = measure(ch, rest_crossing)
            if rest_crossing is None:
                rest_crossing = crossing
                m["self_crossing_at_rest"] = crossing
                m["self_crossing"] = 0
            m["required"] = required
            groups.setdefault(group, {})[name] = m
        report["frames"][frame_name] = groups
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
