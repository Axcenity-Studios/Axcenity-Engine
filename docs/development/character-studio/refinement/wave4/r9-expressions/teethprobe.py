"""R9 review 2: is a tooth vertex face_checks counts "in flesh" really inside the body?

    bpy-python -I teethprobe.py REPO PARTS.glb "POSE NAME"   (a face_checks pose name)

For each tooth vertex `face_checks.measure` finds sunk over 0.5 mm, the inside test
again with 21 random rays instead of three.
"""
import sys, os, json
import bpy  # noqa: F401
import numpy as np
from mathutils import Vector
repo, parts, want = sys.argv[1], sys.argv[2], sys.argv[3]
sys.path.insert(0, os.path.join(repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q, face_checks as FC
_, _, custom_path = Q.package_paths("realistic")
custom = Q.load_custom(custom_path)
with open(custom_path, encoding="utf-8") as f:
    custom["face"] = json.load(f).get("face", {}).get("face_identity", custom.get("face", {}))
Q.reset()
ch = Q.Character(parts, merge_vertices=True, custom=custom)
keys = ch.parts["Body"].data.shape_keys
shapes = {kb.name for kb in keys.key_blocks}
params = FC.face_parameters(os.path.join(os.path.dirname(parts), "Realistic.base.json"))
for group, name, w, ident, req in FC.poses(shapes, params):
    if name != want:
        continue
    FC.pose(ch, 0.0, w, ident)
    body = FC.Piece(ch.parts["Body"])
    teeth = FC.Piece(ch.parts["Teeth"], ("Teeth", "Gums"))
    rng = np.random.default_rng(1)
    for k, p in enumerate(teeth.co):
        d = body.sunk(p, 0.02)
        if d > 0.0005:
            votes = 0
            dirs = rng.normal(size=(21, 3))
            for way in dirs:
                way = Vector(way / np.linalg.norm(way)); at, n = Vector(p), 0
                for _ in range(64):
                    hit = body.bvh.ray_cast(at, way, 3.0)
                    if hit[0] is None: break
                    n += 1; at = hit[0] + way * 1e-6
                votes += n % 2
            print(name, "tooth vertex", k, np.round(p * 1000, 1), "sunk mm", round(d * 1000, 2), "inside votes of 21:", votes)
