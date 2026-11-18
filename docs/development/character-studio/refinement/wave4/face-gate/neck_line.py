"""Face gate (2f): the neck cut against the four pairs' triangles per pose."""
import sys, os, json
import bpy  # noqa: F401
import numpy as np
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
T = [8524, 8525, 14094, 10419, 10420, 14589]
def ident(*ends):
    o = {}
    for pid, end in ends:
        for s, v in params[pid](end).items():
            if s in shapes: o[s] = o.get(s, 0.0) + v
    return o
for label, ends in (("rest", ()), ("mouth_height+1", (("mouth_height", 1.0),)), ("philtrum_length-1", (("philtrum_length", -1.0),)), ("both", (("philtrum_length", -1.0), ("mouth_height", 1.0)))):
    FC.pose(ch, 0.0, {}, ident(*ends) if ends else None)
    body = FC.Piece(ch.parts["Body"], ("Skin",))
    z_neck = ch.head("Head").z - 0.02
    mins = body.co[body.tris[T]][:, :, 2].min(axis=1)
    print(f"{label}: neck cut z {z_neck*1000:.2f} mm; triangles' lowest z {np.round(mins*1000, 2).tolist()} -> kept {(mins > z_neck).tolist()}")
