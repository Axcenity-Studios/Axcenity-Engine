"""Face gate (1b): the fixed inside test against the winding number, over many poses, and
a tooth vertex pushed 2 mm into the lip must still fail the check.

    bpy-python -I probe_inside.py REPO PKGDIR OUT.json
"""
import sys, os, json, math
import bpy
import numpy as np
from mathutils import Vector

repo, pkg, out = sys.argv[1:4]
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
params = FC.face_parameters(os.path.join(pkg, "Realistic.base.json"))
DIRS = (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60)))


def old_inside(piece, p):
    """face_checks' inside test as it was (a re-hit counted)."""
    votes = 0
    for way in DIRS:
        way = way.normalized()
        at, n = Vector(p), 0
        for _ in range(64):
            hit = piece.bvh.ray_cast(at, way, 3.0)
            if hit[0] is None:
                break
            n += 1
            at = hit[0] + way * 1e-6
        votes += n % 2
    return votes >= 2


def winding(co, tris, p):
    a, b, c = (co[tris[:, k]] - p for k in range(3))
    la, lb, lc = (np.linalg.norm(x, axis=1) for x in (a, b, c))
    num = np.einsum("ij,ij->i", a, np.cross(b, c))
    den = (la * lb * lc + np.einsum("ij,ij->i", a, b) * lc + np.einsum("ij,ij->i", a, c) * lb
           + np.einsum("ij,ij->i", b, c) * la)
    return float((2 * np.arctan2(num, den)).sum() / (4 * math.pi))


listed = {(g, n): (w, i) for g, n, w, i, r in FC.poses(shapes, params)}
want = [("rest", "neutral")] + [k for k in listed if k[0] in ("visemes", "expressions", "controls")]
want += [k for k in listed if k[0] == "identity_faces" and k[1].startswith(("eye_corner_right-1", "philtrum_length-1",
                                                                                "jaw_angle+1"))]
rng = np.random.default_rng(3)
report = {"poses": 0, "points": 0, "old_vs_winding_disagree": [], "new_vs_winding_disagree": []}
for frame_name, frame in (("masculine", 0.0), ("feminine", 1.0)):
    for key in want:
        w, ident = listed[key]
        FC.pose(ch, frame, w, ident)
        body = FC.Piece(ch.parts["Body"])
        teeth = FC.Piece(ch.parts["Teeth"], ("Teeth", "Gums"))
        pts = [("teeth", p) for p in teeth.co if body.bvh.find_nearest(Vector(p), 0.02)[0] is not None]
        if "LipSeal" in ch.parts:
            pts += [("seal", p) for p in FC.Piece(ch.parts["LipSeal"]).co]
        report["poses"] += 1
        for kind, p in pts:
            old, new = old_inside(body, p), body.inside(p)
            # The winding number for every point either test calls inside, and 1 in 10 of the rest.
            if not (old or new) and rng.random() > 0.1:
                continue
            wn = winding(body.co, body.tris, np.asarray(p, dtype=np.float64))
            report["points"] += 1
            truth = wn > 0.5
            rec = [frame_name, f"{key[0]}/{key[1]}", kind, [round(float(x) * 1000, 2) for x in p], round(wn, 3)]
            if old != truth:
                report["old_vs_winding_disagree"].append(rec + [old])
            if new != truth:
                report["new_vs_winding_disagree"].append(rec + [new])
    print(frame_name, "done", report["poses"], "poses", report["points"], "points; old wrong",
          len(report["old_vs_winding_disagree"]), "new wrong", len(report["new_vs_winding_disagree"]), flush=True)

# A tooth vertex pushed 2 mm into the lip: the check must fail.
pushed = []
for label, key in (("rest", ("rest", "neutral")), ("the failing pose", ("identity_faces", "eye_corner_right-1 viseme L"))):
    w, ident = listed[key]
    FC.pose(ch, 0.0, w, ident)
    obj = ch.parts["Teeth"]
    dg = bpy.context.evaluated_depsgraph_get()
    body = FC.Piece(ch.parts["Body"])
    teeth = FC.Piece(obj, ("Teeth", "Gums"))
    # The teeth vertex nearest the lips (in front of the incisors), and the point 2 mm past the lip's inner surface.
    near = [(body.bvh.find_nearest(Vector(p), 0.02), p) for p in teeth.co]
    near = [(n[3], n[0], p) for n, p in near if n[0] is not None and p[1] < teeth.co[:, 1].min() + 0.004]
    d, q, p = min(near, key=lambda t: t[0])
    way = (np.array(q) - p) / np.linalg.norm(np.array(q) - p)
    target = np.array(q) + way * 0.002
    wn_target = winding(body.co, body.tris, target)
    # The mesh vertex there, moved by a shape key (fixed-point on the posed position).
    ev = obj.evaluated_get(dg).to_mesh()
    M = np.array(obj.matrix_world)
    co = np.array([v.co[:] for v in ev.vertices]) @ M[:3, :3].T + M[:3, 3]
    obj.evaluated_get(dg).to_mesh_clear()
    vi = int(np.argmin(np.linalg.norm(co - p, axis=1)))
    same = np.where(np.linalg.norm(co - p, axis=1) < 1e-7)[0]
    if obj.data.shape_keys is None:
        obj.shape_key_add(name="Basis")
    key_block = obj.shape_key_add(name="QA_Pushed", from_mix=False)
    base = np.array([v.co[:] for v in obj.data.shape_keys.reference_key.data])
    offset = np.zeros(3)
    for _ in range(6):
        for i in same:
            key_block.data[int(i)].co = Vector(base[int(i)] + offset)
        key_block.value = 1.0
        Q.update()
        ev = obj.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
        now = np.array(ev.vertices[vi].co[:]) @ M[:3, :3].T + M[:3, 3]
        obj.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh_clear()
        offset += np.linalg.inv(M[:3, :3]) @ (target - now)
    body_now = FC.Piece(ch.parts["Body"])
    head = FC.head_triangles(body_now, ch)
    m, _ = FC.measure(ch, head)
    old = sum(1 for x in FC.Piece(obj, ("Teeth", "Gums")).co if body_now.bvh.find_nearest(Vector(x), 0.02)[0] is not None
              and old_inside(body_now, x) and body_now.bvh.find_nearest(Vector(x), 0.02)[3] > 0.0005)
    rec = {"pose": label, "vertex_mm": [round(float(x) * 1000, 2) for x in p], "to_lip_mm": round(d * 1000, 2),
           "moved_to_mm": [round(float(x) * 1000, 2) for x in now], "miss_mm": round(float(np.linalg.norm(now - target)) * 1000, 4),
           "winding_at_target": round(wn_target, 3),
           "depth_mm": round(float(body_now.bvh.find_nearest(Vector(now), 0.02)[3]) * 1000, 3),
           "teeth_in_flesh": m["teeth_in_flesh"], "teeth_in_flesh_max_mm": m["teeth_in_flesh_max_mm"],
           "old_test_count": old}
    fake = {"frames": {"masculine": {"identity_faces": {"pushed": dict(m, required=True)},
                                     "closed": {"Blink": {"bare_rays": 0}, "Eye_Closed": {"bare_rays": 0}}}}}
    rec["failures"] = [b for b in FC.failures(fake) if "teeth" in b]
    print(rec, flush=True)
    pushed.append(rec)
    obj.shape_key_remove(key_block)
    Q.update()
report["pushed_tooth"] = pushed
with open(out, "w") as f:
    json.dump(report, f, indent=1)
print(json.dumps({k: v for k, v in report.items() if k != "pushed_tooth"}, indent=0)[:3000])
