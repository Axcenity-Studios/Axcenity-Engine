"""Face gate (2): philtrum_length -1 with mouth_height +1 on the masculine frame,
the new crossing pairs (face_checks as it was) with no exclusion, where they are, and what is near.

    bpy-python -I probe2.py REPO PKGDIR OUT.json [frame]
"""
import sys, os, json
import bpy  # noqa: F401
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

repo, pkg, out = sys.argv[1:4]
frame = float(sys.argv[4]) if len(sys.argv) > 4 else 0.0
sys.path.insert(0, os.path.join(repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q, face_checks as FC  # noqa: E402


def old_head_crossings(ch):
    """face_checks.head_crossings as it was: the triangles over a line 20 mm under the
    Head bone, chosen in each pose."""
    body = FC.Piece(ch.parts["Body"], ("Skin",))
    keep = np.where(body.co[body.tris][:, :, 2].min(axis=1) > ch.head("Head").z - 0.02)[0]
    return FC.head_crossings(ch, keep)


parts = os.path.join(pkg, "RealisticParts.glb")
custom_path = os.path.join(pkg, "Realistic.build.json")
custom = Q.load_custom(custom_path)
with open(custom_path, encoding="utf-8") as f:
    custom["face"] = json.load(f).get("face", {}).get("face_identity", custom.get("face", {}))
Q.reset()
ch = Q.Character(parts, merge_vertices=True, custom=custom)
shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
params = dict(FC.face_parameters(os.path.join(pkg, "Realistic.base.json")))


def ident(*ends):
    o = {}
    for pid, end in ends:
        for s, v in params[pid](end).items():
            if s in shapes:
                o[s] = o.get(s, 0.0) + v
    return o


def tri_tri_segment(a, b):
    """Rough crossing points: edges of a through b and edges of b through a."""
    pts = []
    for P, T in ((a, b), (b, a)):
        n = np.cross(T[1] - T[0], T[2] - T[0])
        for i in range(3):
            p, q = P[i], P[(i + 1) % 3]
            dp, dq = (p - T[0]) @ n, (q - T[0]) @ n
            if dp * dq < 0:
                x = p + (q - p) * (dp / (dp - dq))
                # inside T?
                ok = True
                for k in range(3):
                    e = T[(k + 1) % 3] - T[k]
                    if np.cross(e, x - T[k]) @ n < -1e-12:
                        ok = False
                if ok:
                    pts.append(x)
    return pts


FC.pose(ch, frame, {})
rest = old_head_crossings(ch)
skin = FC.Piece(ch.parts["Body"], ("Skin",))
lining = FC.Piece(ch.parts["Body"], ("Mouth",))
seal = FC.Piece(ch.parts["LipSeal"]).co
seen = {tuple(np.round(p, 6)) for p in lining.co}
edge = np.array([p for p in skin.co if tuple(np.round(p, 6)) in seen])
FC.pose(ch, frame, {}, ident(("mouth_height", 1.0)))
alone = old_head_crossings(ch)
FC.pose(ch, frame, {}, ident(("philtrum_length", -1.0)))
alone_p = old_head_crossings(ch)
FC.pose(ch, frame, {}, ident(("philtrum_length", -1.0), ("mouth_height", 1.0)))
both = old_head_crossings(ch)
posed = FC.Piece(ch.parts["Body"], ("Skin",))
new = both - alone - rest
res = {"rest_pairs": len(rest), "mouth_height+1_pairs": len(alone), "philtrum-1_pairs": len(alone_p),
       "both_pairs": len(both), "new": []}
for pair in sorted(new):
    a, b = (skin.co[skin.tris[t]] for t in pair)
    pa, pb = (posed.co[posed.tris[t]] for t in pair)
    cross = tri_tri_segment(pa, pb)
    rec = {"pair": list(pair),
           "centre_rest_mm": [round(float(v) * 1000, 2) for v in (a.mean(0) + b.mean(0)) / 2],
           "rest_gap_mm": round(float(np.linalg.norm(a[:, None] - b[None], axis=2).min()) * 1000, 2),
           "to_seal_mm": [round(float(np.linalg.norm(seal - t.mean(0), axis=1).min()) * 1000, 2) for t in (a, b)],
           "to_lips_inner_edge_mm": [round(float(np.linalg.norm(edge - t.mean(0), axis=1).min()) * 1000, 2) for t in (a, b)],
           "crossing_points_posed_mm": [[round(float(v) * 1000, 2) for v in x] for x in cross],
           "in_alone_mouth_height": pair in alone, "in_alone_philtrum": pair in alone_p}
    res["new"].append(rec)
    print(json.dumps(rec))
print({k: v for k, v in res.items() if k != "new"})
print("seal x extent mm", round(float(seal[:, 0].min()) * 1000, 1), round(float(seal[:, 0].max()) * 1000, 1),
      "Skin/Mouth border x extent mm (it also rings the eye sockets)", round(float(edge[:, 0].min()) * 1000, 1), round(float(edge[:, 0].max()) * 1000, 1))
with open(out, "w") as f:
    json.dump(res, f, indent=1)
