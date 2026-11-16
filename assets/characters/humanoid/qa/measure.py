"""Measures a humanoid base model package for defects and writes measurements.json.

    python -I measure.py [--package stylized|realistic] [--parts FILE] [--default FILE]
                         [--out measurements.json] [--source-ngons] [--only a,b]
                         [--check thresholds.json]

Sections: per-object stats and topology, skin weights, clothing and hair
clipping across frames and builds, joint deformation in fixed test poses,
face shapes (self-intersection, eyes and lids, teeth), proportions, and
`twist` (how a limb's cross-sections hold when it turns about its length;
not run unless asked for, or the package has twist bones). A package
without garments, hair, a default file or a given shape skips what needs
them. Every distance is in millimetres unless the key says otherwise. Read
qa/README.md for what each number means.
"""

import argparse
import math
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402  (before bmesh: the bpy module provides it)
import bmesh  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

import qa_scene as Q  # noqa: E402

GARMENTS = ["Shirt_Tee", "Jacket_Cropped", "Pants_Cargo", "Shoes_Sneakers", "Underwear_Briefs", "Underwear_Top"]
HAIR = ["Hair_Swept", "Hair_Bob", "Hair_Long", "Hair_Crop", "Hair_Buzz", "Hair_Ponytail"]
COMBOS = [(f, b) for f in (0.0, 1.0) for b in (None, "Build_Heavy", "Build_Slim", "Build_Muscular")]

# Drillis & Contini (1966), segment lengths as a fraction of stature H, as
# reproduced in D. A. Winter, Biomechanics and Motor Control of Human
# Movement, 4th ed. (Wiley, 2009), fig. 4.1.
ANTHRO = {
    "head_height_ratio": 0.130,   # vertex to chin (1.000 - 0.870)
    "heads_tall": 7.7,
    "shoulder_width_ratio": 0.259,
    "hand_length_ratio": 0.108,
    "foot_length_ratio": 0.152,
    "hip_joint_height_ratio": 0.530,  # greater trochanter
    "knee_height_ratio": 0.285,
    "shoulder_height_ratio": 0.818,
    "upper_arm_ratio": 0.186,
    "forearm_ratio": 0.146,
    "eye_height_ratio": 0.936,
}


def mm(x):
    return None if x is None else round(float(x) * 1000.0, 2)


def r3(x):
    return None if x is None else round(float(x), 3)


# -- stats and topology ---------------------------------------------------------------

def object_stats(obj):
    me = obj.data
    sk = me.shape_keys
    used_bones = set()
    names = {vg.index: vg.name for vg in obj.vertex_groups}
    for v in me.vertices:
        for g in v.groups:
            if g.weight > 0.0:
                used_bones.add(names[g.group])
    return {
        "triangles": sum(len(p.vertices) - 2 for p in me.polygons),
        "vertices_gltf": len(me.vertices),
        "materials": [m.name for m in me.materials if m],
        "uv_layers": [u.name for u in me.uv_layers],
        "shape_keys": [k.name for k in sk.key_blocks[1:]] if sk else [],
        "shape_key_count": len(sk.key_blocks) - 1 if sk else 0,
        "bones_weighted": len(used_bones),
    }


def topology(obj):
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.verts.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    boundary = sum(1 for e in bm.edges if e.is_boundary)
    multi = sum(1 for e in bm.edges if len(e.link_faces) > 2)
    wire = sum(1 for e in bm.edges if e.is_wire)
    nm_verts = sum(1 for v in bm.verts if not v.is_manifold and not v.is_boundary and v.link_faces)
    loose = sum(1 for v in bm.verts if not v.link_faces)
    zero = 0
    sliver = 0
    for f in bm.faces:
        a = f.calc_area()
        if a < 1e-9:
            zero += 1
            continue
        if len(f.verts) == 3:
            p = [v.co for v in f.verts]
            angs = []
            for k in range(3):
                u, w = p[(k + 1) % 3] - p[k], p[(k + 2) % 3] - p[k]
                if u.length < 1e-12 or w.length < 1e-12:
                    angs.append(0.0)
                else:
                    angs.append(u.angle(w))
            if min(angs) < math.radians(2.0):
                sliver += 1
    flipped = 0
    for e in bm.edges:
        if len(e.link_loops) == 2:
            l1, l2 = e.link_loops
            if l1.vert == l2.vert:
                flipped += 1
    # islands
    seen = set()
    islands = 0
    for v in bm.verts:
        if v.index in seen:
            continue
        islands += 1
        stack = [v]
        while stack:
            x = stack.pop()
            if x.index in seen:
                continue
            seen.add(x.index)
            stack.extend(e.other_vert(x) for e in x.link_edges)
    ngons = sum(1 for f in bm.faces if len(f.verts) > 4)
    out = {
        "vertices_merged": len(bm.verts), "islands": islands, "boundary_edges": boundary,
        "edges_over_2_faces": multi, "wire_edges": wire, "nonmanifold_verts": nm_verts,
        "loose_verts": loose, "zero_area_faces": zero, "sliver_tris_min_angle_lt_2deg": sliver,
        "inconsistent_normal_edges": flipped, "ngons": ngons,
    }
    bm.free()
    return out


def source_ngons():
    """Runs the build's `build()` (no export) and counts each part's
    n-gons before glTF triangulates them."""
    import tempfile
    sys.path.insert(0, Q.ASSET_DIR)
    import build  # noqa: F401
    tmp = tempfile.mkdtemp(prefix="humanoid_qa_")
    Q.reset()
    _arm, parts, _g, _c, _r, _m = build.build(tmp)
    out = {}
    for name, obj in parts.items():
        me = obj.data
        sizes = [len(p.vertices) for p in me.polygons]
        out[name] = {"faces": len(sizes), "tris": sizes.count(3), "quads": sizes.count(4),
                     "ngons": sum(1 for s in sizes if s > 4), "max_sides": max(sizes) if sizes else 0}
    sys.path.remove(Q.ASSET_DIR)
    import shutil
    shutil.rmtree(tmp, ignore_errors=True)
    return out


# -- weights -----------------------------------------------------------------------

def bone_segments(ch):
    return {b.name: (b.head_local.copy(), b.tail_local.copy()) for b in ch.arm.data.bones if b.name != "Root"}


def seg_dist(p, a, b):
    d = b - a
    t = max(0.0, min(1.0, (p - a).dot(d) / max(d.length_squared, 1e-12)))
    return (p - (a + d * t)).length


def chain_of(name):
    for f in ("Thumb", "Index", "Middle", "Ring", "Little"):
        if f in name:
            return name.split(f)[0] + f
    return name


def fold(W, custom):
    """`W` with each twist bone's weight counted as the bone it's inside
    and each helper's shared between its parent and the bone it reads, so
    "the skin of the upper arm" means the same with or without them. A
    skeleton with neither gives `W` back."""
    twists, helpers = custom.get("twists", []), custom.get("helpers", [])
    if not twists and not helpers:
        return W
    to = {}
    for side in ("Left", "Right"):
        for t in twists:
            to[side + t["bone"]] = [(side + t["inside"], 1.0)]
        for h in helpers:
            parent = h["parent"] if h["parent"] == "Hips" else side + h["parent"]
            to[side + h["bone"]] = [(parent, 0.5), (side + h["reads"], 0.5)]
    out = []
    for d in W:
        f = {}
        for b, w in d.items():
            for name, share in to.get(b, [(b, 1.0)]):
                f[name] = f.get(name, 0.0) + w * share
        out.append(f)
    return out


def weights_report(ch):
    out = {}
    segs = bone_segments(ch)
    added = {s + t["bone"] for s in ("Left", "Right") for t in ch.custom.get("twists", []) + ch.custom.get("helpers", [])}
    segs = {b: seg for b, seg in segs.items() if b not in added}
    for name, obj in ch.parts.items():
        names = {vg.index: vg.name for vg in obj.vertex_groups}
        over4 = unnorm = none = 0
        max_dev = 0.0
        for v in obj.data.vertices:
            ws = [g.weight for g in v.groups if g.weight > 0.0]
            if not ws:
                none += 1
                continue
            if len(ws) > 4:
                over4 += 1
            s = sum(ws)
            max_dev = max(max_dev, abs(s - 1.0))
            if abs(s - 1.0) > 1e-3:
                unnorm += 1
        out[name] = {"over_4_influences": over4, "unnormalized_sum": unnorm, "max_sum_error": round(max_dev, 5),
                     "no_weights": none}
    body = ch.parts["Body"]
    names = {vg.index: vg.name for vg in body.vertex_groups}
    per_bone = {}
    W = []
    for v in body.data.vertices:
        d = {names[g.group]: g.weight for g in v.groups if g.weight > 0.0}
        W.append(d)
        for b, w in d.items():
            if w > 0.0:
                per_bone[b] = per_bone.get(b, 0) + 1
    all_bones = [b.name for b in ch.arm.data.bones]
    out["Body"]["bones_without_weights"] = [b for b in all_bones if b not in per_bone]
    out["Body"]["vertices_per_bone"] = dict(sorted(per_bone.items()))
    W = fold(W, ch.custom)
    # Fingers: weight on a finger bone where another chain's bone is nearer.
    P = [v.co.copy() for v in body.data.vertices]
    fingers = {}
    for side in ("Left", "Right"):
        for f in ("Thumb", "Index", "Middle", "Ring", "Little"):
            bones = [f"{side}{f}{s}" for s in ("Proximal", "Intermediate", "Distal")]
            total = bleed = 0
            max_w = 0.0
            far = 0
            for i, d in enumerate(W):
                w = sum(d.get(b, 0.0) for b in bones)
                if w <= 0.1:
                    continue
                total += 1
                near = min(segs, key=lambda b: seg_dist(P[i], *segs[b]))
                if chain_of(near) != f"{side}{f}" and not near.endswith("Hand"):
                    bleed += 1
                    max_w = max(max_w, w)
                if min(seg_dist(P[i], *segs[b]) for b in bones) > 0.03:
                    far += 1
            fingers[f"{side}{f}"] = {"verts_w_gt_0.1": total, "bleed_verts_nearer_other_bone": bleed,
                                     "bleed_max_weight": round(max_w, 3), "verts_over_3cm_from_chain": far}
    out["fingers"] = fingers
    # Cross-body weights (left skin on right bones and vice versa).
    cross = 0
    cross_max = 0.0
    cross_bones = {}
    for i, d in enumerate(W):
        x = P[i].x
        for b, w in d.items():
            if w > 0.05 and ((x > 0.02 and b.startswith("Right")) or (x < -0.02 and b.startswith("Left"))):
                cross += 1
                cross_max = max(cross_max, w)
                key = b.replace("Left", "").replace("Right", "")
                cross_bones[key] = cross_bones.get(key, 0) + 1
                break
    out["cross_side_weights"] = {"verts": cross, "max_weight": round(cross_max, 3), "bones": cross_bones}
    # Lips from the shapes that move them: the upper lip should not follow
    # the jaw, the lower lip should.
    sk = body.data.shape_keys.key_blocks
    basis = sk[0].data

    def moved_by(key, frac=0.5):
        kd = sk[key].data
        d = [(kd[i].co - basis[i].co).length for i in range(len(basis))]
        top = max(d)
        return [i for i, x in enumerate(d) if x > frac * top]

    lips = {}
    if "Upper_Lip_Raise" in sk and "Lower_Lip_Lower" in sk:
        upper_lip = moved_by("Upper_Lip_Raise")
        lower_lip = moved_by("Lower_Lip_Lower")
        jaw_u = [W[i].get("Jaw", 0.0) for i in upper_lip]
        jaw_l = [W[i].get("Jaw", 0.0) for i in lower_lip]
        lips = {
            "upper_lip_verts": len(upper_lip), "upper_lip_jaw_weight_mean": round(float(np.mean(jaw_u)), 3),
            "upper_lip_jaw_weight_max": round(float(np.max(jaw_u)), 3),
            "lower_lip_verts": len(lower_lip), "lower_lip_jaw_weight_mean": round(float(np.mean(jaw_l)), 3),
            "lower_lip_jaw_weight_min": round(float(np.min(jaw_l)), 3),
        }
    face_verts = [i for i, p in enumerate(P) if p.z > ch.head("Head").z + 0.02 and p.y < ch.head("Head").y - 0.04]
    neck_on_face = [W[i].get("Neck", 0.0) for i in face_verts]
    out["face_weights"] = {
        **lips,
        "face_front_verts_above_head_joint": len(face_verts),
        "face_verts_with_neck_weight_gt_0.1": sum(1 for w in neck_on_face if w > 0.1),
        "face_neck_weight_max": round(float(max(neck_on_face)), 3),
        "neck_weighted_verts_above_eye_level": sum(1 for i, p in enumerate(P) if p.z > ch.eye_centre("Left").z and W[i].get("Neck", 0) > 0.1),
    }
    # Jaw: how far up the face its weights reach (mouth line = the upper
    # teeth's lowest point).
    teeth = ch.parts.get("Teeth")
    upper = []
    if teeth is not None:
        tn = {vg.index: vg.name for vg in teeth.vertex_groups}
        upper = [v.co.z for v in teeth.data.vertices
                 if any(tn[g.group] == "Head" and g.weight > 0.5 for g in v.groups)]
    mouth_z = min(upper) if upper else None
    jaw = [(P[i], d["Jaw"]) for i, d in enumerate(W) if d.get("Jaw", 0) > 0.05]
    above = [w for p, w in jaw if mouth_z is not None and p.z > mouth_z + 0.004 and p.y < -0.05]
    out["jaw"] = {"verts_w_gt_0.05": len(jaw), "z_range_m": [r3(min(p.z for p, _ in jaw)), r3(max(p.z for p, _ in jaw))] if jaw else None,
                  "mouth_line_z_m": r3(mouth_z), "front_verts_above_mouth_line": len(above),
                  "max_weight_above_mouth_line": round(max(above), 3) if above else 0.0}
    return out, W


# -- clipping -------------------------------------------------------------------------

def dominant(W, i):
    d = W[i]
    return max(d, key=d.get) if d else "none"


def poke_stats(body_geo, layer_bvh, W, idx=None, extra=None, out_window=0.05, in_window=0.03, near=0.06):
    """Skin (or inner layer) points under a layer: covered vs poking out."""
    covered = pokes = exposed = 0
    depths = []
    where = {}
    ex_where = {}
    gaps = []
    rng = range(len(body_geo.co)) if idx is None else idx
    for i in rng:
        p = Vector(body_geo.co[i])
        loc, _n, _f, d = layer_bvh.find_nearest(p, near)
        if loc is None:
            continue
        kind, dist = Q.cover_test(layer_bvh, p, body_geo.nrm[i], out_window, in_window)
        key = None
        if kind in ("poke", "exposed"):
            key = extra(i) if extra else (dominant(W, i) if W else "")
        if kind == "covered":
            covered += 1
            gaps.append(dist)
        elif kind == "poke":
            covered += 1
            pokes += 1
            depths.append(dist)
            where[key] = where.get(key, 0) + 1
        elif kind == "exposed":
            exposed += 1
            ex_where[key] = ex_where.get(key, 0) + 1
    out = {"covered": covered, "poke": pokes, "poke_pct": round(100.0 * pokes / covered, 2) if covered else 0.0,
           "poke_max_mm": mm(max(depths)) if depths else 0.0,
           "poke_p90_mm": mm(np.percentile(depths, 90)) if depths else 0.0,
           "where": dict(sorted(where.items(), key=lambda kv: -kv[1])[:6]),
           "exposed_over_opening": exposed,
           "exposed_where": dict(sorted(ex_where.items(), key=lambda kv: -kv[1])[:6])}
    if gaps:
        out["gap_median_mm"] = mm(np.median(gaps))
        out["gap_p95_mm"] = mm(np.percentile(gaps, 95))
    return out


ARM_BONES = ("UpperArm", "LowerArm", "Hand")


def arm_face(body, W):
    """A test for the body's triangles (by index) that belong to an arm."""
    def test(fi):
        tri = body.tris[fi]
        votes = sum(1 for i in tri if (dominant(W, int(i)) or "").endswith(ARM_BONES))
        return votes >= 2
    return test


def facing_out(layer, body, reach=0.1):
    """The layer's points whose normal faces away from the body (the skin's
    normal at the nearest point), leaving out openings' rolled-in bands."""
    keep = []
    for i in range(len(layer.co)):
        loc, nrm, _, _ = body.bvh.find_nearest(Vector(layer.co[i]), reach)
        if loc is not None and Vector(layer.nrm[i]).dot(nrm) > 0.2:
            keep.append(i)
    return keep


def combo_name(f, b):
    return ("feminine" if f else "masculine") + "/" + (b or "neutral")


def obj_weights(obj):
    names = {vg.index: vg.name for vg in obj.vertex_groups}
    return [{names[g.group]: g.weight for g in v.groups if g.weight > 0.0} for v in obj.data.vertices]


def clipping(ch, W, ear_idx):
    out = {"garments": {}, "layering": {}, "waist_gap_mm": {}, "tee_alone_waist_gap_mm": {}, "hair": {},
           "hair_face_shapes": {}, "hair_ears_out": {}}
    outfit = ch.custom["default"]["outfit"]
    WT = obj_weights(ch.parts["Shirt_Tee"])
    WP = obj_weights(ch.parts["Pants_Cargo"])
    for f, b in COMBOS:
        build = {b: 1.0} if b else None
        cn = combo_name(f, b)
        for g in GARMENTS:
            ch.set_look(frame=f, build=build, masks=ch.masks_for([g]))
            body = Q.Geo(ch.parts["Body"])
            gg = Q.Geo(ch.parts[g])
            out["garments"].setdefault(g, {})[cn] = poke_stats(body, gg.bvh, W)
        # The outfit's layer order, as build.py clears it: the trousers'
        # waistband under the untucked tee, the tee under the jacket. Only
        # inner points facing away from the body count: an opening's
        # rolled-in band faces the skin, can't be seen, and isn't a poke.
        ch.set_look(frame=f, build=build, masks=ch.masks_for(outfit))
        body = Q.Geo(ch.parts["Body"])
        tee = Q.Geo(ch.parts["Shirt_Tee"])
        jacket = Q.Geo(ch.parts["Jacket_Cropped"])
        pants = Q.Geo(ch.parts["Pants_Cargo"])
        out["layering"].setdefault("Shirt_Tee_under_Jacket_Cropped", {})[cn] = poke_stats(
            tee, jacket.bvh, WT, idx=facing_out(tee, body))
        out["layering"].setdefault("Pants_Cargo_under_Shirt_Tee", {})[cn] = poke_stats(
            pants, tee.bvh, WP, idx=facing_out(pants, body))
        all_g = Q.merged_bvh([Q.Geo(ch.parts[g]) for g in outfit])
        out["layering"].setdefault("Body_under_default_outfit", {})[cn] = poke_stats(body, all_g, W)
        # Bare skin between the outfit's garments round the waist (in the
        # shipped Humanoid.glb that skin is removed: a hole).
        out["waist_gap_mm"][cn] = Q.band_gaps(all_g, body.bvh, 0.88, 1.20, skip=arm_face(body, W))
        ch.set_look(frame=f, build=build, masks=ch.masks_for(["Shirt_Tee", "Pants_Cargo"]))
        body = Q.Geo(ch.parts["Body"])
        tp = Q.merged_bvh([Q.Geo(ch.parts["Shirt_Tee"]), Q.Geo(ch.parts["Pants_Cargo"])])
        out["tee_alone_waist_gap_mm"][cn] = Q.band_gaps(tp, body.bvh, 0.88, 1.20, skip=arm_face(body, W))
        # Hair (builds don't move hair; the face and neck do move a little).
        if b in (None, "Build_Heavy"):
            for h in HAIR:
                ch.set_look(frame=f, build=build)
                body = Q.Geo(ch.parts["Body"])
                hg = Q.Geo(ch.parts[h])
                out["hair"].setdefault(h, {})[cn] = poke_stats(
                    body, hg.bvh, W, extra=lambda i: region_of(body, i, ear_idx), out_window=0.06, in_window=0.03)
    for f in (0.0, 1.0):
        for h in HAIR:
            for shape in ("Face_Head_Width", "Face_Head_Length"):
                for w in (-1.0, 1.0):
                    ch.set_look(frame=f, face={shape: w})
                    body = Q.Geo(ch.parts["Body"])
                    hg = Q.Geo(ch.parts[h])
                    out["hair_face_shapes"].setdefault(h, {})[f"{combo_name(f, None)}/{shape}{w:+.0f}"] = poke_stats(
                        body, hg.bvh, W, extra=lambda i: region_of(body, i, ear_idx), out_window=0.06, in_window=0.03)
            ch.set_look(frame=f, face={"Face_Ears_Out": 1.0, "Face_Ears_Size": 1.0})
            body = Q.Geo(ch.parts["Body"])
            hg = Q.Geo(ch.parts[h])
            out["hair_ears_out"].setdefault(h, {})[combo_name(f, None)] = poke_stats(
                body, hg.bvh, W, idx=ear_idx, extra=lambda i: "ear")
    return out


def region_of(geo, i, ear_idx):
    if i in ear_idx:
        return "ear"
    p = geo.co[i]
    if p[1] < -0.06 and p[2] < 1.68:
        return "forehead/face"
    if p[2] < 1.56:
        return "neck/nape"
    return "scalp"


def face_parts_report(ch, configs):
    """Brows and lashes against the skin, lashes against the eyeballs."""
    out = {}
    for label, look, pose in configs:
        ch.set_look(**look)
        ch.pose_reset()
        if pose:
            pose(ch)
            Q.update()
        body = Q.Geo(ch.parts["Body"])
        eyes = Q.Geo(ch.parts["Eyes"])
        res = {}
        for part in ("Brows", "Lashes"):
            g = Q.Geo(ch.parts[part])
            sd = [Q.signed_distance(body.bvh, p, 0.05) for p in g.co]
            sd = np.array([s for s in sd if s is not None])
            res[part] = {"min_mm": mm(sd.min()), "median_mm": mm(np.median(sd)), "max_mm": mm(sd.max()),
                         "pct_below_skin_1mm": round(100.0 * float((sd < -0.001).mean()), 1),
                         "pct_floating_over_3mm": round(100.0 * float((sd > 0.003).mean()), 1)}
            if part == "Lashes":
                se = [Q.signed_distance(eyes.bvh, p, 0.03) for p in g.co]
                se = np.array([s for s in se if s is not None])
                res[part]["inside_eyeball_verts"] = int((se < -0.0002).sum())
                res[part]["inside_eyeball_max_mm"] = mm(-se.min()) if (se < 0).any() else 0.0
        out[label] = res
    return out


# -- eyes, lids, teeth, self-intersection ----------------------------------------------

def head_tris(geo, z_min):
    keep = (geo.co[geo.tris][:, :, 2].min(axis=1) > z_min)
    return geo.tris[keep]


def self_intersections(geo, tris):
    bvh = BVHTree.FromPolygons([tuple(p) for p in geo.co], [tuple(t) for t in tris], all_triangles=True)
    pairs = bvh.overlap(bvh)
    n = 0
    for i, j in pairs:
        if i >= j:
            continue
        if set(tris[i]) & set(tris[j]):
            continue
        n += 1
    return n


def eye_metrics(ch, body, eyes, teeth=None, z_neck=None):
    res = {}
    pen = 0
    pen_max = 0.0
    eye_in_flesh = 0
    eye_in_flesh_max = 0.0
    uncovered = 0
    front_total = 0
    gaps = []
    for side, s in Q.SIDES:
        c = ch.eye_centre(side)
        er = np.linalg.norm(eyes.co[eyes.co[:, 0] * s > 0] - np.array(c), axis=1)
        r = float(np.median(er))
        near = np.where(np.linalg.norm(body.co - np.array(c), axis=1) < r * 1.6)[0]
        sd_lid = []
        for i in near:
            sd = Q.signed_distance(eyes.bvh, body.co[i], 0.03)
            if sd is None:
                continue
            if body.co[i][1] < c.y:  # in front of the eye's centre: the lids
                sd_lid.append(sd)
            if sd < -0.0002:
                pen += 1
                pen_max = max(pen_max, -sd)
        if sd_lid:
            sd_lid = np.sort(np.array(sd_lid))
            pos = sd_lid[sd_lid > -0.0002]
            gaps.append(float(np.median(pos[:30])) if len(pos) else 0.0)
        idx = np.where((eyes.co[:, 0] * s > 0))[0]
        for i in idx:
            p = Vector(eyes.co[i])
            d = (p - c)
            if d.length < 1e-9:
                continue
            if (-d.y / d.length) < math.cos(math.radians(60)):
                continue  # only the front of the eye
            front_total += 1
            sd = Q.signed_distance(body.bvh, p, 0.03)
            if sd is not None and sd < -0.0002:
                eye_in_flesh += 1
                eye_in_flesh_max = max(eye_in_flesh_max, -sd)
            n = Vector(eyes.nrm[i])
            hit = body.bvh.ray_cast(p + n * 1e-4, n, 0.03)
            if hit[0] is None:
                uncovered += 1
    res["skin_inside_eyeball_verts"] = pen
    res["skin_inside_eyeball_max_mm"] = mm(pen_max)
    res["front_eye_inside_lid_flesh_verts"] = eye_in_flesh
    res["front_eye_inside_lid_flesh_max_mm"] = mm(eye_in_flesh_max)
    res["front_eye_visible_pct"] = round(100.0 * uncovered / max(front_total, 1), 1)
    res["lid_margin_gap_mm"] = [mm(g) for g in gaps]
    if teeth is not None:
        emb = []
        for p in teeth.co:
            sd = Q.signed_distance(body.bvh, p, 0.02)
            if sd is not None and sd < -0.0005:
                emb.append(-sd)
        res["teeth_verts_inside_lips_or_gums_flesh"] = len(emb)
        res["teeth_inside_flesh_max_mm"] = mm(max(emb)) if emb else 0.0
    return res


FACE_SHAPES = ["Face_Eyes_Spacing", "Face_Eyes_Height", "Face_Eyes_Size", "Face_Eyes_Tilt", "Face_Brows_Height",
               "Face_Nose_Width", "Face_Nose_Length", "Face_Nose_Bridge", "Face_Nose_Tip", "Face_Mouth_Width",
               "Face_Lips_Full", "Face_Chin_Length", "Face_Chin_Forward", "Face_Jaw_Width", "Face_Cheekbones",
               "Face_Head_Width", "Face_Head_Length", "Face_Ears_Size", "Face_Ears_Out"]


def both(name, w=1.0):
    return {f"{name}_Left": w, f"{name}_Right": w}


EXPRESSIONS = {
    "neutral": ({}, None),
    "smile": (both("Mouth_Smile"), None),
    "frown": (both("Mouth_Frown"), None),
    "jaw_open": ({"Jaw_Open": 1.0}, Q.jaw_open),
    "jaw_open+smile": ({"Jaw_Open": 1.0, **both("Mouth_Smile")}, Q.jaw_open),
    "blink": (both("Blink"), None),
    "brow_raise": (both("Brow_Raise"), None),
    "sneer": (both("Nose_Sneer"), None),
}


def face_report(ch):
    out = {}
    z_neck = ch.head("Head").z - 0.02
    has = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
    for f in (0.0, 1.0):
        fr = {}
        base = None
        configs = [("neutral", {}, {}, None)]
        for name in FACE_SHAPES:
            if name not in has:
                continue
            for w in (-1.0, 1.0):
                configs.append((f"{name}{w:+.0f}", {name: w}, {}, None))
        for name, (expr, pose) in EXPRESSIONS.items():
            # An expression needs its shapes; one with a pose (the jaw's
            # bone) is still measured on a body without them.
            if name != "neutral" and (all(k in has for k in expr) or pose):
                configs.append((name, {}, {k: w for k, w in expr.items() if k in has}, pose))
        for label, face, expr, pose in configs:
            ch.set_look(frame=f, face=face, expr=expr)
            ch.pose_reset()
            if pose:
                pose(ch)
                Q.update()
            body = Q.Geo(ch.parts["Body"])
            eyes = Q.Geo(ch.parts["Eyes"])
            teeth = Q.Geo(ch.parts["Teeth"])
            si = self_intersections(body, head_tris(body, z_neck))
            m = eye_metrics(ch, body, eyes, teeth)
            m["head_self_intersecting_tri_pairs"] = si
            if base is None:
                base = si
            m["self_intersections_vs_neutral"] = si - base
            fr[label] = m
        ch.pose_reset()
        out["feminine" if f else "masculine"] = fr
    # The same without the eye bones following the frame (what the engine
    # shows until CS3 applies bone offsets).
    ch.set_look(frame=1.0)
    for pb in ch.arm.pose.bones:
        if pb.name.endswith("Eye"):
            pb.matrix_basis = Q.Matrix()
    Q.update()
    saved = ch.eye_transform
    ch.eye_transform = lambda side: (Vector(), 1.0)
    out["feminine_eyes_not_moved"] = eye_metrics(ch, Q.Geo(ch.parts["Body"]), Q.Geo(ch.parts["Eyes"]))
    ch.eye_transform = saved
    ch.set_look()
    return out


# -- deformation --------------------------------------------------------------------

def ring_points(ch, P, W, pivot, parent, radius):
    """Edges of the rest mesh crossing the plane through the pivot bone's
    head, normal to the parent bone, near the pivot and on the joint's skin."""
    c = ch.head(pivot)
    n = (ch.head(pivot) - ch.head(parent)).normalized() if parent not in ("Hips", "LeftShoulder") else \
        (ch.tail(pivot) - ch.head(pivot)).normalized()
    if pivot == "Chest":
        n = (ch.head("Chest") - ch.head("Spine")).normalized()
    if pivot == "LeftMiddleProximal":
        n = (ch.tail(pivot) - ch.head(pivot)).normalized()
    joint_bones = {pivot, parent}
    pts = []
    me = ch.parts["Body"].data
    for e in me.edges:
        i, j = e.vertices
        a, b = P[i], P[j]
        da, db = (a - c).dot(n), (b - c).dot(n)
        if da * db > 0 or da == db:
            continue
        t = da / (da - db)
        x = a.lerp(b, t)
        if (x - c).length > radius:
            continue
        wa = sum(W[i].get(bb, 0) for bb in joint_bones)
        wb = sum(W[j].get(bb, 0) for bb in joint_bones)
        if wa + wb < 1.0:
            continue
        pts.append((i, j, t))
    # order by angle round the plane
    u = n.orthogonal().normalized()
    w = n.cross(u)
    pts.sort(key=lambda q: math.atan2((P[q[0]].lerp(P[q[1]], q[2]) - c).dot(w), (P[q[0]].lerp(P[q[1]], q[2]) - c).dot(u)))
    return pts


def ring_area(co, pts):
    xs = [Vector(co[i]).lerp(Vector(co[j]), t) for i, j, t in pts]
    if len(xs) < 3:
        return 0.0
    s = Vector()
    for k in range(len(xs)):
        s += xs[k].cross(xs[(k + 1) % len(xs)])
    return s.length / 2.0


def local_volume(co, tris, c):
    a = co[tris[:, 0]] - c
    b = co[tris[:, 1]] - c
    d = co[tris[:, 2]] - c
    return float(np.einsum("ij,ij->i", a, np.cross(b, d)).sum() / 6.0)


HAND_BONES = ("Hand", "Thumb", "Index", "Middle", "Ring", "Little")


def deformation(ch, W, default_path, package="stylized"):
    out = {}
    outfit = ch.custom["default"]["outfit"]
    WT = obj_weights(ch.parts["Shirt_Tee"]) if "Shirt_Tee" in ch.parts else None
    for f in (0.0, 1.0):
        fr = {}
        ch.set_look(frame=f)
        ch.pose_reset()
        rest = Q.Geo(ch.parts["Body"])
        P = [Vector(p) for p in rest.co]
        for pose, (_fn, joint) in Q.POSES.items():
            res = {}
            if pose == "fist":
                joint = None  # several pivots: the cone volume isn't meaningful
            if joint:
                pivot, parent, radius = joint
                c_rest = np.array(ch.head(pivot))
                region = np.linalg.norm(rest.co - c_rest, axis=1) < radius
                tris = rest.tris[region[rest.tris].all(axis=1)]
                v0 = local_volume(rest.co, tris, c_rest)
                ring = ring_points(ch, P, W, pivot, parent, radius * 0.8)
                a0 = ring_area(rest.co, ring)
            ch.set_look(frame=f)
            Q.apply_pose(ch, pose)
            body = Q.Geo(ch.parts["Body"])
            if joint:
                c = np.array(ch.arm.pose.bones[pivot].head)
                v1 = local_volume(body.co, tris, c)
                a1 = ring_area(body.co, ring)
                res["local_volume_ratio"] = round(v1 / v0, 3) if v0 else None
                res["ring_area_ratio"] = round(a1 / a0, 3) if a0 else None
                res["ring_points"] = len(ring)
                res["region_radius_m"] = radius
            if pose == "fist":
                hand = (rest.co[:, 0] > 0.3) & (rest.co[:, 2] < 1.1)
                if f:
                    hand = (rest.co[:, 0] > 0.25) & (rest.co[:, 2] < 1.12)
                if package != "stylized":
                    # The left hand, by what it's weighted to.
                    hand = np.array([sum(w for b, w in d.items() if b.startswith("Left") and
                                         any(k in b for k in HAND_BONES)) > 0.5 for d in W])
                ht = body.tris[hand[body.tris].all(axis=1)]
                res["hand_self_intersecting_tri_pairs"] = self_intersections(body, ht)
                res["hand_self_intersections_at_rest"] = self_intersections(rest, ht)
            # The default outfit in this pose.
            if outfit:
                ch.set_look(frame=f, masks=ch.masks_for(outfit))
                Q.apply_pose(ch, pose)
                body = Q.Geo(ch.parts["Body"])
                gs = {g: Q.Geo(ch.parts[g]) for g in outfit}
                merged = Q.merged_bvh(list(gs.values()))
                res["body_through_outfit"] = poke_stats(body, merged, W)
                res["tee_through_jacket"] = poke_stats(gs["Shirt_Tee"], gs["Jacket_Cropped"].bvh, WT,
                                                       idx=facing_out(gs["Shirt_Tee"], body))
                wg = Q.band_gaps(merged, body.bvh, 0.88, 1.20, skip=arm_face(body, W))
                res["waist_gap_mm_max"] = max(wg.values())
                res["waist_gap_mm_sum_16_dirs"] = round(sum(wg.values()), 1)
            fr[pose] = res
            ch.set_look(frame=f)
            ch.pose_reset()
        out["feminine" if f else "masculine"] = fr
    if not default_path:
        return out
    # Humanoid.glb (the shipped default, its hidden skin removed): do the
    # trimmed body's cut edges stay under the clothes?
    holes = {}
    sel = set(bpy.data.objects)
    Q.reset()
    d = Q.Character(default_path, merge_vertices=True)
    d.set_look(masks=d.masks_for(outfit))
    bm = bmesh.new()
    bm.from_mesh(d.parts["Body"].data)
    edge_verts = sorted({v.index for e in bm.edges if e.is_boundary for v in e.verts})
    bm.free()
    for pose in Q.POSES:
        Q.apply_pose(d, pose)
        body = Q.Geo(d.parts["Body"])
        allg = Q.merged_bvh([Q.Geo(d.parts[g]) for g in outfit])
        unc = 0
        for i in edge_verts:
            # Uncovered: nothing of the outfit within 6 cm outward along
            # the normal, nor any garment surface within 4 mm.
            kind, _ = Q.cover_test(allg, body.co[i], body.nrm[i], 0.06, 0.03)
            loc = allg.find_nearest(Vector(body.co[i]), 0.004)[0]
            if kind is None and loc is None:
                unc += 1
        holes[pose] = {"cut_edge_verts": len(edge_verts), "uncovered": unc}
    out["default_file_cut_edges"] = holes
    del sel
    return out


# -- twist -------------------------------------------------------------------------

# (name, bone turned about its own length, degrees, the limb's bones from the
# body outward). Turning a hand or an arm about its length is what a plain
# two-bone blend handles worst: the skin between the bones winds up like a
# sweet wrapper. Twist bones spread the turn along the limb.
TWIST_POSES = [
    ("forearm_twist_90", "LeftHand", 90.0, ("LeftLowerArm", "LeftHand")),
    ("upper_arm_twist_70", "LeftUpperArm", 70.0, ("LeftShoulder", "LeftUpperArm", "LeftLowerArm")),
    ("thigh_twist_45", "LeftUpperLeg", 45.0, ("Hips", "LeftUpperLeg", "LeftLowerLeg")),
]


def twist_report(ch, W):
    """For each twist pose: the limb's cross-sections (rings of the rest
    mesh's edges crossing planes along the turned bone's parent segment
    and its own), posed area over rest. A wound-up limb's smallest ring
    falls far under 1."""
    out = {}
    me = ch.parts["Body"].data
    for f in (0.0, 1.0):
        fr = {}
        ch.set_look(frame=f)
        ch.pose_reset()
        rest = Q.Geo(ch.parts["Body"])
        P = [Vector(p) for p in rest.co]
        for name, bone, deg, chain in TWIST_POSES:
            # Planes along the segment the twist is spread over: the turned
            # bone's parent for a hand, the bone itself for an arm or thigh.
            seg = chain[-2] if bone == chain[-1] else bone
            a, b = ch.head(seg), ch.tail(seg)
            n = (b - a).normalized()
            limb = set(chain)
            rings = []
            for k in range(1, 12):
                c = a.lerp(b, k / 12.0)
                pts = []
                for e in me.edges:
                    i, j = e.vertices
                    da, db = (P[i] - c).dot(n), (P[j] - c).dot(n)
                    if da * db > 0 or da == db:
                        continue
                    t = da / (da - db)
                    x = P[i].lerp(P[j], t)
                    if (x - c).length > 0.16:
                        continue
                    if sum(W[i].get(bb, 0) for bb in limb) + sum(W[j].get(bb, 0) for bb in limb) < 1.6:
                        continue
                    pts.append((i, j, t))
                u = n.orthogonal().normalized()
                w = n.cross(u)
                pts.sort(key=lambda q: math.atan2((P[q[0]].lerp(P[q[1]], q[2]) - c).dot(w),
                                                  (P[q[0]].lerp(P[q[1]], q[2]) - c).dot(u)))
                rings.append(pts)
            a0 = [ring_area(rest.co, r) for r in rings]
            ch.pose_reset()
            ch.bend(bone, n, deg)
            ch.drive()
            Q.update()
            posed = Q.Geo(ch.parts["Body"])
            ratios = [round(ring_area(posed.co, r) / x, 3) for r, x in zip(rings, a0) if x > 0]
            fr[name] = {"ring_area_ratio_min": min(ratios), "ring_area_ratios": ratios}
            ch.pose_reset()
        out["feminine" if f else "masculine"] = fr
    return out


# -- proportions ---------------------------------------------------------------------

def proportions(ch, W):
    out = {}
    for f in (0.0, 1.0):
        ch.set_look(frame=f)
        ch.pose_reset()
        g = Q.Geo(ch.parts["Body"])
        co = g.co
        H = float(co[:, 2].max())
        head_z = ch.head("Head").z
        prof = co[(np.abs(co[:, 0]) < 0.006) & (co[:, 2] > ch.head("Neck").z)]
        nose = prof[np.argmin(prof[:, 1])]
        front = prof[(prof[:, 2] < nose[2] - 0.03) & (prof[:, 2] > nose[2] - 0.12)]
        chin_front_y = float(front[:, 1].min())
        chin_zone = prof[(prof[:, 1] < chin_front_y + 0.015) & (prof[:, 2] < nose[2] - 0.03)]
        menton = float(chin_zone[:, 2].min())
        head_h = H - menton
        # shoulders: joint centres and skin over the deltoids
        lsh, rsh = ch.head("LeftUpperArm"), ch.head("RightUpperArm")
        el = ch.head("LeftLowerArm")
        P = [Vector(p) for p in co]
        delt = 0.0
        for i, p in enumerate(P):
            if W[i].get("LeftUpperArm", 0) + W[i].get("LeftShoulder", 0) < 0.5:
                continue
            t = (p - lsh).dot(el - lsh) / (el - lsh).length_squared
            if t < 0.12:
                delt = max(delt, p.x)
        # hand: wrist (Hand head) to the farthest point along the hand
        wr = ch.head("LeftHand")
        along = (ch.head("LeftMiddleProximal") - wr).normalized()
        hand =[p for i, p in enumerate(P) if p.x > wr.x - 0.03 and
                sum(w for b, w in W[i].items() if b.startswith("Left") and (b.endswith("Hand") or any(k in b for k in ("Thumb", "Index", "Middle", "Ring", "Little")))) > 0.5]
        hand_len = max((p - wr).dot(along) for p in hand)
        foot = [p for i, p in enumerate(P) if p.x > 0 and p.z < 0.14 and
                W[i].get("LeftFoot", 0) + W[i].get("LeftToes", 0) + W[i].get("LeftLowerLeg", 0) > 0.5]
        foot_len = max(p.y for p in foot) - min(p.y for p in foot)
        hip = ch.head("LeftUpperLeg").z
        knee = ch.head("LeftLowerLeg").z
        eye = ch.eye_centre("Left").z
        upper_arm = (ch.head("LeftLowerArm") - lsh).length
        forearm = (ch.head("LeftHand") - ch.head("LeftLowerArm")).length
        r = {
            "height_m": r3(H), "menton_z_m": r3(menton), "head_height_m": r3(head_h),
            "heads_tall": round(H / head_h, 2), "head_height_ratio": round(head_h / H, 3),
            "shoulder_joint_width_m": r3(lsh.x - rsh.x),
            "bideltoid_width_m": r3(2 * delt), "shoulder_width_ratio_bideltoid": round(2 * delt / H, 3),
            "hand_length_m": r3(hand_len), "hand_length_ratio": round(hand_len / H, 3),
            "foot_length_m": r3(foot_len), "foot_length_ratio": round(foot_len / H, 3),
            "hip_joint_height_ratio": round(hip / H, 3), "knee_height_ratio": round(knee / H, 3),
            "shoulder_joint_height_ratio": round(lsh.z / H, 3), "eye_height_ratio": round(eye / H, 3),
            "upper_arm_ratio": round(upper_arm / H, 3), "forearm_ratio": round(forearm / H, 3),
            "head_bone_z_m": r3(head_z),
        }
        out["feminine" if f else "masculine"] = r
    out["reference"] = dict(ANTHRO, source="Drillis & Contini (1966) segment lengths as fractions of stature, "
                                           "reproduced in D. A. Winter, Biomechanics and Motor Control of Human "
                                           "Movement, 4th ed., 2009, fig. 4.1; heads tall = 1/0.130")
    return out


# -- shape ranges, eyes, shoes -----------------------------------------------------------

def shapes_report(ch):
    """How far each body shape moves the skin (customization depth), the
    eyeball and iris size, and the sneakers' sole against the upper."""
    out = {"body_shape_max_move_mm": {}, "body_shape_verts_moved_over_0.5mm": {}}
    me = ch.parts["Body"].data
    sk = me.shape_keys.key_blocks
    n = len(me.vertices)
    basis = np.empty(n * 3)
    sk[0].data.foreach_get("co", basis)
    for kb in sk[1:]:
        co = np.empty(n * 3)
        kb.data.foreach_get("co", co)
        d = np.linalg.norm((co - basis).reshape(n, 3), axis=1)
        out["body_shape_max_move_mm"][kb.name] = mm(d.max())
        out["body_shape_verts_moved_over_0.5mm"][kb.name] = int((d > 0.0005).sum())
    eyes = ch.parts["Eyes"].data
    mats = [m.name for m in eyes.materials]
    co = np.array([v.co[:] for v in eyes.vertices])
    left = co[:, 0] > 0
    iris = set()
    for p in eyes.polygons:
        if mats[p.material_index] == "Iris":
            iris.update(i for i in p.vertices if co[i, 0] > 0)
    ic = co[sorted(iris)]
    out["eyeball_diameter_mm"] = mm((co[left].max(0) - co[left].min(0)).max())
    out["iris_diameter_mm"] = mm(max(ic[:, 0].max() - ic[:, 0].min(), ic[:, 2].max() - ic[:, 2].min()))
    out["reference_eye"] = "adult eyeball about 24 mm, iris about 11.7 mm (scale by the head height ratio for this head)"
    if "Shoes_Sneakers" not in ch.parts:
        return out
    shoes = ch.parts["Shoes_Sneakers"].data
    smats = [m.name for m in shoes.materials]
    sco = np.array([v.co[:] for v in shoes.vertices])
    sole, upper = set(), set()
    for p in shoes.polygons:
        (sole if smats[p.material_index] == "Sole" else upper if smats[p.material_index] in ("Shoes", "ShoeAccent") else set()).update(p.vertices)
    sole = sco[[i for i in sole if sco[i, 0] > 0]]
    upper = sco[[i for i in upper if sco[i, 0] > 0]]
    c = sole[:, :2].mean(0)
    sole_top = sole[:, 2].max()
    low_upper = upper[upper[:, 2] < sole_top + 0.015]
    over = []
    for k in range(36):
        a = 2 * math.pi * k / 36
        dvec = np.array([math.cos(a), math.sin(a)])
        sr = ((sole[:, :2] - c) @ dvec).max()
        ur = ((low_upper[:, :2] - c) @ dvec).max()
        over.append(sr - ur)
    out["sole_overhang_mm"] = {"min": mm(min(over)), "median": mm(np.median(over)), "max": mm(max(over))}
    out["sole_top_z_mm"] = mm(sole_top)
    out["upper_lowest_z_mm"] = mm(upper[:, 2].min())
    return out


# -- main ----------------------------------------------------------------------------

def file_report(path, custom=None):
    Q.reset()
    raw = Q.Character(path, merge_vertices=False, custom=custom)
    stats = {n: object_stats(o) for n, o in sorted(raw.parts.items())}
    bones = len(raw.arm.data.bones)
    Q.reset()
    merged = Q.Character(path, merge_vertices=True, custom=custom)
    for n, o in merged.parts.items():
        stats[n]["topology"] = topology(o)
    return {"bones": bones, "objects": stats,
            "total_triangles": sum(s["triangles"] for s in stats.values()),
            "total_vertices_gltf": sum(s["vertices_gltf"] for s in stats.values())}


def crossing_pairs(geo):
    """The pairs of the body's triangles that cross and share no vertex."""
    tris = geo.tris
    tree = BVHTree.FromPolygons([tuple(p) for p in geo.co], [tuple(t) for t in tris], all_triangles=True)
    return {(i, j) for i, j in tree.overlap(tree) if i < j and not set(tris[i]) & set(tris[j])}


def slider_pairs(ch, touching=0.004):
    """Every pair of the manifest's parameters at every combination of
    their ranges' ends, on both frames, in the rest pose: how many pairs
    of the body's triangles newly cross. "Newly" leaves out the pairs
    that cross in the frame's neutral body and the pairs that already
    touch there (within 4 mm: closed lips, neighbouring toes), where any
    move makes or unmakes a crossing. Shapes are set by the manifest's
    rule and the body sliders as `anim` applies them
    (`Character.set_sliders`, `set_body_sliders`). Also each parameter
    alone, with the eyes and teeth checked against the skin."""
    manifest = ch.manifest
    ch.show_only(["Body", "Eyes", "Teeth"])
    params = [p for p in manifest["parameters"] if p["id"] != "frame"]

    def look(frame, values):
        ch.pose_reset()
        ch.set_sliders(dict(values, frame=frame))
        sliders = {}
        for p in params:
            for d in p.get("attributes", []):
                sliders[d["attribute"]] = Q.span(values.get(p["id"], p.get("default", 0.0)), d.get("from"), d.get("to"))
        if any(abs(v - 1.0) > 1e-6 for v in sliders.values()):
            ch.set_body_sliders(sliders)
        Q.update()
        return Q.Geo(ch.parts["Body"])

    out = {"parameters": len(params) + 1, "frames": {}}
    for label, frame in (("masculine", 0.0), ("feminine", 1.0)):
        neutral = look(frame, {})
        known = crossing_pairs(neutral)
        tris, P = neutral.tris, neutral.co

        def new(geo):
            n = 0
            for a, b in crossing_pairs(geo) - known:
                gap = np.linalg.norm(P[tris[a]][:, None] - P[tris[b]][None], axis=2).min()
                n += gap > touching
            return int(n)

        ends = {p["id"]: [e for e in p["range"] if abs(e - p.get("default", 0.0)) > 1e-9] for p in params}
        alone, worst = {}, []
        for p in params:
            for e in ends[p["id"]]:
                geo = look(frame, {p["id"]: e})
                one = {"new_crossings": new(geo)}
                if not p.get("attributes"):
                    # A shape can push skin into the eyes or the teeth. A body
                    # slider can't: it moves and scales bones, and the eyes and
                    # teeth are carried whole by the head's.
                    m = eye_metrics(ch, geo, Q.Geo(ch.parts["Eyes"]), Q.Geo(ch.parts["Teeth"]))
                    one.update({k: m.get(k) for k in ("skin_inside_eyeball_verts", "front_eye_inside_lid_flesh_verts",
                                                      "teeth_verts_inside_lips_or_gums_flesh")})
                alone[f"{p['id']}={e:g}"] = one
        pairs = bodies = crossing = 0
        for i, a in enumerate(params):
            for b in params[i + 1:]:
                pairs += 1
                for ea in ends[a["id"]]:
                    for eb in ends[b["id"]]:
                        bodies += 1
                        n = new(look(frame, {a["id"]: ea, b["id"]: eb}))
                        if n:
                            crossing += 1
                            worst.append((n, f"{a['id']}={ea:g}, {b['id']}={eb:g}"))
        worst.sort(reverse=True)
        out["frames"][label] = {
            "neutral_crossings": len(known), "alone": alone, "pairs": pairs, "bodies": bodies,
            "bodies_with_new_crossings": crossing, "new_crossings_largest": worst[0][0] if worst else 0,
            "offenders": [{"sliders": s, "new_crossings": n} for n, s in worst[:40]],
            "alone_with_new_crossings": sum(1 for v in alone.values() if v["new_crossings"]),
            "alone_eye_or_teeth_in_flesh": sum(1 for v in alone.values() if v.get("skin_inside_eyeball_verts")
                                               or v.get("front_eye_inside_lid_flesh_verts")
                                               or v.get("teeth_verts_inside_lips_or_gums_flesh")),
        }
        print(label, "pairs", pairs, "bodies", bodies, "with new crossings", crossing, flush=True)
    ch.pose_reset()
    ch.set_look()
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--package", default="stylized", choices=sorted(Q.PACKAGES))
    ap.add_argument("--parts", default="", help="the parts file (the package's, unless given)")
    ap.add_argument("--default", default="", help="the default character file (the package's, if it has one)")
    ap.add_argument("--custom", default="", help="the customization or build data file (the package's, unless given)")
    ap.add_argument("--out", default="measurements.json")
    ap.add_argument("--source-ngons", action="store_true")
    ap.add_argument("--only", default="", help="comma list of sections to run")
    ap.add_argument("--check", default="", help="thresholds file: exit 1 if a measure is worse")
    ap.add_argument("--strict", action="store_true",
                    help="with --check: also exit 1 when one of the file's `targets` isn't met")
    a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
    parts_path, default_path, custom_path = Q.package_paths(a.package)
    a.parts = a.parts or parts_path
    a.default = a.default or default_path
    custom = Q.load_custom(a.custom or custom_path)
    only = set(filter(None, a.only.split(",")))
    # `twist` is extra: run when asked for, or for a package with twist bones.
    extra = {"twist"} if not custom.get("twists") else set()
    # `sliders` takes a quarter of an hour: only when asked for.
    extra.add("sliders")
    run = lambda s: (s in only) if only else s not in extra  # noqa: E731
    out = {"files": {}, "notes": [
        "Blender space: metres, Z up, the character faces -Y. Distances in mm unless named _m.",
        "Topology counts are on position-merged vertices (glTF splits vertices at UV seams); glTF stores only "
        "triangles, so ngons are 0 in the files (see source_ngons for the build's polygons).",
        "Clipping: a skin vertex is 'covered' when a ray along its normal meets the layer within 5 cm (6 cm for "
        "hair), 'poke' when instead the layer's outward face lies up to 3 cm inside the skin. Percentages are "
        "of covered vertices.",
        "Feminine looks move the skeleton by the customization file's Frame_Feminine offsets and the eye bones "
        "by their offset and eye_scale (what CS3 is meant to do).",
    ]}
    t0 = time.time()
    if run("files"):
        out["files"][os.path.basename(a.parts)] = file_report(a.parts, custom)
        if a.default:
            out["files"][os.path.basename(a.default)] = file_report(a.default, custom)
        print("files", round(time.time() - t0, 1))
    if a.source_ngons and run("files"):
        out["source_ngons"] = source_ngons()
    Q.reset()
    ch = Q.Character(a.parts, merge_vertices=True, custom=custom)
    wr, W = weights_report(ch)
    if run("weights"):
        out["weights"] = wr
        print("weights", round(time.time() - t0, 1))
    sk = ch.parts["Body"].data.shape_keys.key_blocks
    basis = sk[0]
    ear_idx = set()
    if "Face_Ears_Size" in sk:
        ears = sk["Face_Ears_Size"]
        ear_idx = {i for i in range(len(basis.data)) if (ears.data[i].co - basis.data[i].co).length > 1e-6}
    if run("clipping") and custom["default"]["outfit"]:
        out["clipping"] = clipping(ch, W, ear_idx)
        out["face_parts"] = face_parts_report(ch, [
            ("masculine/neutral", {}, None), ("feminine/neutral", {"frame": 1.0}, None),
            ("masculine/brow_raise", {"expr": both("Brow_Raise")}, None),
            ("masculine/brow_lower", {"expr": both("Brow_Lower")}, None),
            ("masculine/blink", {"expr": both("Blink")}, None),
            ("feminine/blink", {"frame": 1.0, "expr": both("Blink")}, None),
            ("masculine/Face_Brows_Height+1", {"face": {"Face_Brows_Height": 1.0}}, None),
            ("masculine/Face_Eyes_Size+1", {"face": {"Face_Eyes_Size": 1.0}}, None),
        ])
        print("clipping", round(time.time() - t0, 1))
    if run("face"):
        out["face"] = face_report(ch)
        print("face", round(time.time() - t0, 1))
    if run("shapes"):
        out["shapes"] = shapes_report(ch)
        print("shapes", round(time.time() - t0, 1))
    if run("sliders"):
        out["sliders"] = slider_pairs(ch)
        print("sliders", round(time.time() - t0, 1))
    if run("proportions"):
        out["proportions"] = proportions(ch, W)
        print("proportions", round(time.time() - t0, 1))
    if run("deformation"):
        out["deformation"] = deformation(ch, W, a.default, a.package)
        print("deformation", round(time.time() - t0, 1))
    if run("twist"):
        if a.default:  # `deformation` ends in the default file's scene
            Q.reset()
            ch = Q.Character(a.parts, merge_vertices=True, custom=custom)
        out["twist"] = twist_report(ch, W)
        print("twist", round(time.time() - t0, 1))
    import json
    with open(a.out, "w", newline="\n") as f:
        json.dump(out, f, indent=1)
        f.write("\n")
    print("wrote", a.out, round(time.time() - t0, 1), "s")
    if a.check:
        limits = json.load(open(a.check))
        failures = check(out, limits)
        for line in failures:
            print("WORSE:", line)
        unmet = check(out, limits.get("targets", {}))
        for line in unmet:
            print("TARGET NOT MET:", line)
        failed = bool(failures) or (a.strict and bool(unmet))
        print("check:", "failed" if failed else "passed",
              f"({len(unmet)} targets not met)" if unmet else "")
        if failed:
            sys.exit(1)


def lookup(out, path):
    """The value at `path` (keys joined by `/`) in the measurements."""
    at = out
    for key in path.split("/"):
        if not isinstance(at, dict) or key not in at:
            return None
        at = at[key]
    return at


def check(out, limits):
    """Where the measurements are worse than `limits` (a thresholds file,
    see `thresholds.json`): a list of lines, empty when nothing is. A
    file with `at_least`, `at_most` or `within` holds measures by their
    path in the measurements (`deformation/masculine/elbow_135/ring_area_ratio`);
    one with `waist_gap_mm` holds the clipping section. A file's `targets`
    (the same keys: where the body is meant to get to) are checked apart,
    by `main`."""
    failures = []
    for kind, bad in (("at_least", lambda v, lim: v < lim), ("at_most", lambda v, lim: v > lim),
                      ("within", lambda v, lim: not lim[0] <= v <= lim[1])):
        for path, limit in limits.get(kind, {}).items():
            value = lookup(out, path)
            if value is None:
                failures.append(f"{path}: not measured (run every section)")
            elif bad(value, limit):
                failures.append(f"{path}: {value}, {kind.replace('_', ' ')} {limit}")
    if "waist_gap_mm" not in limits:
        return failures
    c = out.get("clipping")
    if c is None:
        return ["no clipping section (run with --only clipping or all sections)"]
    for key in ("waist_gap_mm", "tee_alone_waist_gap_mm"):
        worst = max(max(per.values()) for per in c[key].values())
        if worst > limits[key]:
            failures.append(f"{key}: {worst} mm > {limits[key]} mm")
    for group in ("garments", "layering"):
        for name, limit in limits[group].items():
            combos = c[group][name]
            points = sum(v["poke"] for v in combos.values())
            deepest = max(v["poke_max_mm"] for v in combos.values())
            if points > limit["points"] or deepest > limit["max_mm"]:
                failures.append(f"{group} {name}: {points} points, deepest {deepest} mm "
                                f"(limit {limit['points']} points, {limit['max_mm']} mm)")
    return failures


if __name__ == "__main__":
    main()
