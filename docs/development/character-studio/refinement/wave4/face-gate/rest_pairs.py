"""Face gate (2g): the pairs the rest-chosen head newly subtracts as the face's own
(crossing at rest, under the old line through the mouth), per frame."""
import sys, os, json
import bpy  # noqa: F401
import numpy as np
from mathutils.bvhtree import BVHTree
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
for name, frame in (("masculine", 0.0), ("feminine", 1.0)):
    FC.pose(ch, frame, {})
    skin = FC.Piece(ch.parts["Body"], ("Skin",))
    new = FC.head_crossings(ch, FC.head_triangles(skin, ch))
    old_keep = np.where(skin.co[skin.tris][:, :, 2].min(axis=1) > ch.head("Head").z - 0.02)[0]
    old = FC.head_crossings(ch, old_keep)
    print(name, "rest pairs: old line", len(old), "rest-chosen head", len(new), "triangles", len(old_keep), "->",
          len(FC.head_triangles(skin, ch)))
    for pair in sorted(new - old):
        a, b = (skin.co[skin.tris[t]] for t in pair)
        gap = np.linalg.norm(a[:, None] - b[None], axis=2).min()
        print("   ", pair, "at mm", np.round((a.mean(0) + b.mean(0)) / 2 * 1000, 1).tolist(), "nearest vertices mm", round(gap * 1000, 3))
