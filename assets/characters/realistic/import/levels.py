"""Step 6 of the intake: the simpler copies (Blender's Python).

LOD0 is the weighted item; each further level is Blender's collapse
decimation of it to `levels.ratios` (by default 0.5 and 0.25 of its
triangles), weighted again from the body (`skinning.transfer`, the same
split), and measured: its triangles and how far its surface strays from
LOD0's (median, 95th percentile, most), so a level that loses the
silhouette is seen. The engine's fit then pushes every level clear of the
body on its own (`fit.py`).

    python -I levels.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import json
import os

import bpy  # noqa: F401  (Blender's Python)
import numpy as np

import blendmesh
import checks
from workmesh import WorkMesh


def step(spec, work, source="weighted.npz"):
    import bodyglb
    import skinning
    body = bodyglb.load(spec.get("body"))
    split = spec.get("weights", {}).get("split")
    ratios = spec.get("levels", {}).get("ratios", [0.5, 0.25])
    lod0 = WorkMesh.load(os.path.join(work, source))
    report = {"lod0": {"triangles": int(len(lod0.triangles()))}}
    lod0.save(os.path.join(work, "lod0.npz"), report["lod0"])
    for level, ratio in enumerate(ratios, start=1):
        obj = blendmesh.to_object(lod0)
        mod = obj.modifiers.new("Decimate", "DECIMATE")
        mod.decimate_type = "COLLAPSE"
        mod.ratio = float(ratio)
        mesh = blendmesh.from_object(obj, lod0.materials)
        mesh.joints, mesh.weights = skinning.transfer(body, mesh.P, split=split)
        mesh.bones = list(body.bones)
        away = checks.distances(mesh.P, lod0.P, lod0.triangles())
        back = checks.distances(lod0.P, mesh.P, mesh.triangles())
        entry = {
            "ratio": ratio,
            "triangles": int(len(mesh.triangles())),
            "from_lod0_m": {"median": round(float(np.median(away)), 5),
                            "p95": round(float(np.percentile(away, 95)), 5),
                            "most": round(float(away.max()), 5)},
            "lod0_from_it_m": {"p95": round(float(np.percentile(back, 95)), 5),
                               "most": round(float(back.max()), 5)},
        }
        report[f"lod{level}"] = entry
        mesh.save(os.path.join(work, f"lod{level}.npz"), entry)
    # Levels a previous run left beyond these are removed.
    level = len(ratios) + 1
    while os.path.exists(os.path.join(work, f"lod{level}.npz")):
        os.remove(os.path.join(work, f"lod{level}.npz"))
        level += 1
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work), indent=1))


if __name__ == "__main__":
    main()
