"""Face gate (2b): how deep the four pairs cross, how far apart the same triangles are in
the neighbouring poses, and whether the crossing can be seen from in front.

    bpy-python -I probe2b.py REPO PKGDIR
"""
import sys, os, json, math
import bpy  # noqa: F401
import numpy as np
from mathutils import Vector

repo, pkg = sys.argv[1:3]
sys.path.insert(0, os.path.join(repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q, face_checks as FC  # noqa: E402

parts = os.path.join(pkg, "RealisticParts.glb")
custom_path = os.path.join(pkg, "Realistic.build.json")
custom = Q.load_custom(custom_path)
with open(custom_path, encoding="utf-8") as f:
    custom["face"] = json.load(f).get("face", {}).get("face_identity", custom.get("face", {}))
Q.reset()
ch = Q.Character(parts, merge_vertices=True, custom=custom)
shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
params = dict(FC.face_parameters(os.path.join(pkg, "Realistic.base.json")))
PAIRS = [(8524, 14094), (8525, 14094), (10419, 14589), (10420, 14589)]


def ident(*ends):
    o = {}
    for pid, end in ends:
        for s, v in params[pid](end).items():
            if s in shapes:
                o[s] = o.get(s, 0.0) + v
    return o


def tri_gap(a, b):
    """Signed: minimum over vertices of one triangle of distance to the other's plane,
    for vertices whose projection lies inside the other triangle; negative = through."""
    best = None
    for P, T in ((a, b), (b, a)):
        n = np.cross(T[1] - T[0], T[2] - T[0]); n /= np.linalg.norm(n)
        side = [(p - T[0]) @ n for p in P]
        for p, d in zip(P, side):
            x = p - n * d
            inside = all(np.cross(T[(k + 1) % 3] - T[k], x - T[k]) @ n >= 0 for k in range(3))
            if inside:
                # sign relative to the majority side of P
                s = d if np.sign(d) == np.sign(np.median(side)) else -abs(d)
                best = s if best is None else min(best, abs(s) if s >= 0 else s)
    return best


def crossing_points(a, b):
    pts = []
    for P, T in ((a, b), (b, a)):
        n = np.cross(T[1] - T[0], T[2] - T[0])
        for i in range(3):
            p, q = P[i], P[(i + 1) % 3]
            dp, dq = (p - T[0]) @ n, (q - T[0]) @ n
            if dp * dq < 0:
                x = p + (q - p) * (dp / (dp - dq))
                if all(np.cross(T[(k + 1) % 3] - T[k], x - T[k]) @ n >= -1e-15 for k in range(3)):
                    pts.append(x)
    return pts


def fib_front(n):
    i = np.arange(n) + 0.5
    phi = np.arccos(1 - 2 * i / n)
    th = math.pi * (1 + 5 ** 0.5) * i
    d = np.stack([np.cos(th) * np.sin(phi), np.sin(th) * np.sin(phi), np.cos(phi)], axis=1)
    return d[-d[:, 1] > math.cos(math.radians(80))]  # within 80 degrees of straight ahead (-Y)


views = fib_front(400)
for label, ends in (("rest", ()), ("mouth_height+1", (("mouth_height", 1.0),)),
                    ("philtrum_length-1", (("philtrum_length", -1.0),)),
                    ("both", (("philtrum_length", -1.0), ("mouth_height", 1.0)))):
    FC.pose(ch, 0.0, {}, ident(*ends) if ends else None)
    skin = FC.Piece(ch.parts["Body"], ("Skin",))
    body = FC.Piece(ch.parts["Body"])
    print("==", label)
    for pair in PAIRS:
        a, b = (skin.co[skin.tris[t]] for t in pair)
        size = max(np.linalg.norm(a[1] - a[0]), np.linalg.norm(b[1] - b[0])) * 1000
        vmin = float(np.linalg.norm(a[:, None] - b[None], axis=2).min()) * 1000
        pts = crossing_points(a, b)
        line = ""
        if pts:
            # visibility of each crossing point from 0.5 m in the front cone
            seen = 0
            for p in pts:
                for d in views:
                    cam = Vector(p + d * 0.5)
                    way = Vector(p) - cam
                    L = way.length
                    hit = body.bvh.ray_cast(cam, way / L, L + 0.001)
                    if hit[0] is not None and hit[3] >= L - 0.00005:
                        seen += 1
            line = f" crossing pts {len(pts)}, seen from {seen} of {len(pts) * len(views)} front views"
        g = tri_gap(a, b)
        print(f"  pair {pair}: edge ~{size:.2f} mm, nearest vertices {vmin:.3f} mm, plane gap "
              f"{(g * 1000 if g is not None else float('nan')):.4f} mm{line}")
