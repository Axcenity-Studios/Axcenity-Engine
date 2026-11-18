"""Face gate (1): the tooth vertex face_checks (as it was) called in flesh, measured independently.

    bpy-python -I probe1.py REPO PARTS.glb "POSE NAME" OUT.json
"""
import sys, os, json, math
import bpy  # noqa: F401
import numpy as np
from mathutils import Vector

repo, parts, want, out = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
sys.path.insert(0, os.path.join(repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q, face_checks as FC  # noqa: E402
from pseudo_normals import PseudoNormals  # noqa: E402

_, _, custom_path = Q.package_paths("realistic")
custom = Q.load_custom(custom_path)
with open(custom_path, encoding="utf-8") as f:
    custom["face"] = json.load(f).get("face", {}).get("face_identity", custom.get("face", {}))
Q.reset()
ch = Q.Character(parts, merge_vertices=True, custom=custom)
shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
params = FC.face_parameters(os.path.join(os.path.dirname(parts), "Realistic.base.json"))


def fib(n):
    i = np.arange(n) + 0.5
    phi = np.arccos(1 - 2 * i / n)
    th = math.pi * (1 + 5 ** 0.5) * i
    return np.stack([np.cos(th) * np.sin(phi), np.sin(th) * np.sin(phi), np.cos(phi)], axis=1)


def parity(bvh, p, way):
    at, n, hits = Vector(p), 0, []
    way = Vector(way)
    for _ in range(64):
        hit = bvh.ray_cast(at, way, 3.0)
        if hit[0] is None:
            break
        n += 1
        hits.append((float((hit[0] - Vector(p)).length), int(hit[2])))
        at = hit[0] + way * 1e-6
    return n, hits


def winding(co, tris, p):
    a, b, c = (co[tris[:, k]] - p for k in range(3))
    la, lb, lc = (np.linalg.norm(x, axis=1) for x in (a, b, c))
    num = np.einsum("ij,ij->i", a, np.cross(b, c))
    den = la * lb * lc + np.einsum("ij,ij->i", a, b) * lc + np.einsum("ij,ij->i", a, c) * lb + np.einsum("ij,ij->i", b, c) * la
    return float((2 * np.arctan2(num, den)).sum() / (4 * math.pi))


def old_inside(piece, p):
    """face_checks' inside test as it was (a re-hit of the same triangle counted)."""
    votes = 0
    for way in (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60))):
        way.normalize()
        votes += parity(piece.bvh, p, way)[0] % 2
    return votes >= 2


report = {}
for group, name, w, ident, req in FC.poses(shapes, params):
    if name != want:
        continue
    FC.pose(ch, 0.0, w, ident)
    body = FC.Piece(ch.parts["Body"])
    skin = FC.Piece(ch.parts["Body"], ("Skin",))
    teeth = FC.Piece(ch.parts["Teeth"], ("Teeth", "Gums"))
    pn = PseudoNormals(body.co, body.tris)
    dirs = fib(201)
    for k, p in enumerate(teeth.co):
        loc, normal, face, dist = body.bvh.find_nearest(Vector(p), 0.02)
        d = dist if loc is not None and old_inside(body, p) else 0.0
        if d <= 0.0005:
            continue
        three = []
        for way in (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60))):
            way.normalize()
            n, hits = parity(body.bvh, p, way)
            three.append({"dir": [round(x, 3) for x in way], "crossings": n, "hits_mm": [(round(h[0] * 1000, 3), h[1]) for h in hits[:8]]})
        odd = sum(parity(body.bvh, p, way)[0] % 2 for way in dirs)
        wn = winding(body.co, body.tris, np.asarray(p, dtype=np.float64))
        signed = pn.signed(np.asarray(p, dtype=np.float64), face)
        # which material is the nearest face?
        me = ch.parts["Body"].evaluated_get(bpy.context.evaluated_depsgraph_get()).data
        rec = {"pose": f"{group}/{name}", "tooth_vertex": k, "at_mm": [round(float(x) * 1000, 2) for x in p],
               "sunk_mm_by_check": round(d * 1000, 3), "nearest_mm": round(dist * 1000, 3),
               "nearest_point_mm": [round(x * 1000, 2) for x in loc],
               "check_three_rays": three, "odd_of_201_rays": int(odd), "winding_number": round(wn, 4),
               "pseudo_normal_signed_mm": round(signed * 1000, 3)}
        # also: the same for its neighbours in the teeth (points 1 mm around)
        print(json.dumps(rec, indent=1))
        report.setdefault("flagged", []).append(rec)
    # Sanity of the independent tests: other tooth vertices, all winding vs 3-ray vote
    agree = dis = 0
    flips = []
    near = [k for k, p in enumerate(teeth.co) if body.bvh.find_nearest(Vector(p), 0.006)[0] is not None]
    rng = np.random.default_rng(0)
    sample = sorted(rng.choice(near, size=min(300, len(near)), replace=False).tolist())
    report["sampled"] = len(sample)
    for k in sample:
        p = teeth.co[k]
        wn = winding(body.co, body.tris, np.asarray(p, dtype=np.float64))
        vote = old_inside(body, p)
        if (wn > 0.5) == vote:
            agree += 1
        else:
            dis += 1
            flips.append((k, round(wn, 3), vote, round(float(body.bvh.find_nearest(Vector(p), 0.05)[3]) * 1000, 2)))
    report["all_teeth_winding_vs_vote"] = {"agree": agree, "disagree": dis, "disagreements": flips[:40]}
    print("winding vs vote over", len(sample), "teeth vertices within 6 mm of the body: agree", agree, "disagree", dis, flips[:40])
with open(out, "w") as f:
    json.dump(report, f, indent=1)
