"""Clay close-ups of the trunk for the proportion ranges (R3): bodies set
by parameters and body sliders as `ranges.py` sets them (`ranges.look`),
under one hard lamp from the side so steps, notches and creases show.

    python -I render_ranges.py --cases cases.json --out FOLDER [--samples 24] [--parts FILE] [--aim body]

`cases.json` is a list of {"label", "frame" (0 or 1), "values"
({parameter: value}), "sliders" ({attribute: value}), "pose" (a QA pose
or "rest"), "views" (["front", "three-quarter", "side", "back"])}. Writes
one PNG per case and view and a sheet per case list (`sheet.png`).
"""

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402,F401
from mathutils import Vector  # noqa: E402

import qa_scene as Q  # noqa: E402
import ranges as R  # noqa: E402

# view -> (camera direction from the aim, the lamp from behind)
VIEWS = {"front": ((0.0, -1.0, 0.0), False), "three-quarter": ((0.7, -0.7, 0.0), False),
         "side": ((1.0, 0.0, 0.0), False), "back": ((0.0, 1.0, 0.0), True)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cases", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--samples", type=int, default=24)
    ap.add_argument("--size", type=int, default=520)
    ap.add_argument("--title", default="")
    ap.add_argument("--package", default="realistic", choices=sorted(Q.PACKAGES))
    ap.add_argument("--parts", default="", help="another build's parts file (its build data beside it)")
    ap.add_argument("--aim", default="trunk", choices=["trunk", "body"],
                    help="frame the trunk (the default) or the whole body")
    a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
    cases = json.load(open(a.cases))
    os.makedirs(a.out, exist_ok=True)
    parts, _, custom = Q.package_paths(a.package)
    if a.parts:
        parts, custom = a.parts, os.path.join(os.path.dirname(a.parts), os.path.basename(custom))
    Q.reset()
    ch = Q.Character(parts, merge_vertices=True, custom=Q.load_custom(custom))
    for p in ch.manifest["parameters"]:
        if p["group"] == "proportions":
            p["range"] = list(R.BOUND)
    ch.show_only(["Body", "Eyes", "Teeth"])
    cam = Q.setup_studio(a.size, a.size, a.samples)
    Q.clay_parts(ch.parts.values(), Q.clay_kind)
    tiles, labels = [], []
    for case in cases:
        R.look(ch, case.get("frame", 0.0), case.get("values", {}), case.get("sliders", {}), case.get("pose"))
        h, _ = R.heads(ch)
        aim = Vector((0.0, 0.0, (h["LeftUpperLeg"].z + h["Chest"].z) / 2.0))
        span = 0.75
        if a.aim == "body":
            aim, span = Vector((0.0, 0.0, h["Hips"].z - 0.05)), 1.9
        for view in case.get("views", ["front", "three-quarter", "side"]):
            direction, back = VIEWS[view]
            for o in list(bpy.data.objects):
                if o.name.startswith("Raking"):
                    bpy.data.objects.remove(o)
            lamp = Q.raking_light(aim, back=back)
            if view == "side":
                lamp.location = aim + Vector((1.2, -1.6, 0.9))
                Q.look_at(lamp, aim)
            path = os.path.join(a.out, f"{case['label']}-{view}.png".replace(" ", "_"))
            # (Inside the cyclorama, whose back wall is 3.2 m behind the figure.)
            Q.shot(cam, path, aim + Vector(direction) * 2.6, aim, ortho=span)
            tiles.append(path)
            labels.append(f"{case['label']}, {view}")
    cols = max(len(c.get("views", ["front", "three-quarter", "side"])) for c in cases)
    Q.contact_sheet(os.path.join(a.out, "sheet.png"), tiles, cols, labels, a.title or None, max_kb=1500, grey=True)


if __name__ == "__main__":
    main()
