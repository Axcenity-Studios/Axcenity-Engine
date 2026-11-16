"""The realistic base body's Character Data manifest (Character Studio
overhaul CO4; formats in docs/development/character-studio/architecture.md).

Writes `Realistic.base.json` (the base model manifest: parameters,
regions, slots, sockets, materials, skeleton, face) from
`Realistic.build.json` (what the build measured: the frame's bone offsets,
the added bones, the regions) and the objects of `RealisticParts.glb`.
Plain Python, no Blender: `build.py` runs it after writing those files,
and it can be run alone:

    python -I assets/characters/realistic/manifests.py [folder]

Then rebuild the library's catalogue: python -I assets/characters/catalogue.py

Slots, sockets and the proportion sliders are the stylized humanoid's
(`../humanoid/manifests.py`), read from there, so an item or a script
that names one fits both bodies. The package has no items yet (CO9,
CO10): a new character on this base wears nothing.
"""

import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BASE_ID = "axcenity/realistic"
PARTS_FILE = "RealisticParts.glb"
DATA_FILE = "Realistic.build.json"
MANIFEST_FILE = "Realistic.base.json"
FACE_PROFILE = "axcenity"

# The parts always worn with the body.
FACE_PARTS = ["Eyes", "Teeth"]

GROUPS = [
    ("frame", "Frame", "body"),
    ("build", "Build", "body"),
    ("build_regions", "Build by region", "body"),
    ("proportions", "Proportions", "body"),
]

# How far a proportion slider goes on this body (CO5). `anim`'s body
# sliders take 0.5 to 1.5, which the stylized body's manifest offers whole;
# on this body the ends of that make the skin cross itself (shoulders at
# half width put the arms through the chest: 282 pairs of triangles), so
# each slider stops where every pair of sliders at their ends still leaves
# the body free of new crossings (`measure.py --only sliders`).
PROPORTION_RANGE = [0.85, 1.15]
# Hips only get wider: the masculine frame's thighs meet at the crotch, and
# hips 2% narrower already make the skin cross there on a heavy body.
PROPORTION_RANGES = {"hip_width": [1.0, 1.15]}

# Every material of the body and its parts, and how it is lit.
SHADING = {
    "Skin": "skin", "Mouth": "standard", "Iris": "eye", "Sclera": "eye",
    "Teeth": "standard", "Gums": "standard", "Tongue": "standard",
}


def stylized():
    """The stylized humanoid's manifest generator (for what the two share)."""
    path = os.path.join(os.path.dirname(HERE), "humanoid", "manifests.py")
    spec = importlib.util.spec_from_file_location("stylized_manifests", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def base_manifest(data, parts):
    shared = stylized()
    frame = data["frame"]
    bones = shared.bone_drivers(frame["bones"], frame.get("eye_scale"))
    parameters = [
        {"id": "frame", "label": "Masculine to feminine", "group": "frame", "range": [0, 1],
         "shapes": [{"shape": "Frame_Feminine"}], "bones": bones},
    ]
    for pid, label, attribute, advanced in shared.PROPORTIONS:
        span = PROPORTION_RANGES.get(pid, PROPORTION_RANGE)
        parameter = {"id": pid, "label": label, "group": "proportions", "range": span,
                     "default": 1, "attributes": [{"attribute": attribute, "from": span}]}
        if advanced:
            parameter["advanced"] = True
        parameters.append(parameter)
    # The body's muscle and fat sliders (CO5): every one of their shapes is a
    # corner with the frame (`shapes.py`).
    if HERE not in sys.path:
        sys.path.insert(0, HERE)
    import shapes
    parameters += shapes.parameters()
    body = parts["Body"]
    materials = sorted({m for name in ["Body", *FACE_PARTS] for m in parts[name]["materials"]})
    return {
        "format_version": 1,
        "id": BASE_ID,
        "label": "Realistic",
        "family": "realistic",
        "model": {"file": PARTS_FILE, "body": body["path"],
                  "parts": [parts[name]["path"] for name in FACE_PARTS]},
        "skeleton": {"convention": "humanoid", "height": 1.75, "pose": "a-pose-45",
                     "added_bones": data["added_bones"]},
        "groups": [{"id": i, "label": l, "category": c} for i, l, c in GROUPS],
        "parameters": parameters,
        "corners": shapes.corners(),
        "regions": [{"id": r["id"], "label": r["label"], "map_index": r["map_index"]}
                    for r in data["regions"]],
        "slots": shared.SLOTS,
        "sockets": [{"name": n, "bone": b} for n, b in shared.SOCKETS],
        "materials": [{"name": m, "shading": SHADING[m]} for m in materials],
        "face": {"profile": FACE_PROFILE, "eye_bones": ["LeftEye", "RightEye"], "jaw_bone": "Jaw"},
        "defaults": {"items": []},
    }


def write(folder):
    shared = stylized()
    with open(os.path.join(folder, DATA_FILE)) as f:
        data = json.load(f)
    parts = shared.objects(os.path.join(folder, PARTS_FILE))
    shared.write_json(os.path.join(folder, MANIFEST_FILE), base_manifest(data, parts))


if __name__ == "__main__":
    write(sys.argv[1] if len(sys.argv) > 1 else HERE)
