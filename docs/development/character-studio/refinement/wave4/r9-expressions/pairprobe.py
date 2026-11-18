"""R9 review 2: philtrum_length -1 with mouth_height +1, the new skin crossings
with and without face_checks' closed-lips exclusion (pairs within LIPS_EDGE of LipSeal).

    bpy-python -I pairprobe.py REPO PARTS.glb
"""
import sys, os, json
import bpy  # noqa: F401
import numpy as np
repo, parts = sys.argv[1], sys.argv[2]
sys.path.insert(0, os.path.join(repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q, face_checks as FC
_, _, custom_path = Q.package_paths("realistic")
custom = Q.load_custom(custom_path)
Q.reset()
ch = Q.Character(parts, merge_vertices=True, custom=custom)
shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
params = dict(FC.face_parameters(os.path.join(repo, "assets/characters/realistic/Realistic.base.json")))
def ident(*ends):
    out = {}
    for pid, end in ends:
        for s, v in params[pid](end).items():
            if s in shapes: out[s] = out.get(s, 0.0) + v
    return out
FC.pose(ch, 0.0, {})
rest = FC.head_crossings(ch)
skin = FC.Piece(ch.parts["Body"], ("Skin",))
seal = FC.Piece(ch.parts["LipSeal"]).co
FC.pose(ch, 0.0, {}, ident(("mouth_height", 1.0)))
alone = FC.head_crossings(ch)
FC.pose(ch, 0.0, {}, ident(("philtrum_length", -1.0), ("mouth_height", 1.0)))
both = FC.head_crossings(ch)
new = both - alone - rest
def at_lips(pair):
    a, b = (skin.co[skin.tris[t]] for t in pair)
    if np.linalg.norm(a[:, None] - b[None], axis=2).min() > 0.004: return False
    return all(np.linalg.norm(seal - t.mean(axis=0), axis=1).min() < FC.LIPS_EDGE for t in (a, b))
print("new pairs", sorted(tuple(sorted(p)) for p in new)[:10], "raw", len(new), "counted", sum(1 for p in new if not at_lips(p)))
for p in new:
    a = skin.co[skin.tris[list(p)[0]]].mean(axis=0)
    print("  pair at mm", np.round(a * 1000, 1), "nearest seal mm", round(float(np.linalg.norm(seal - a, axis=1).min()) * 1000, 2))
