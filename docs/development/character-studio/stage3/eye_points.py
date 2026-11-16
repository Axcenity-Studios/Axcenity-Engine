"""Stage 3 diagnostic: the eyeball points the sliders check counts in the
lid's flesh, one by one. For each parameter end the check flagged, and
each front eyeball point it signs inside the body, prints how deep, what
the nearest feature of the body is (a face, an edge two faces share, an
open edge with one face, or a corner), whether that feature is on the
body's open rim, and what four rays' parity says (an odd count of
crossings is inside, on a closed surface).

    PY -I docs/development/character-studio/stage3/eye_points.py

Writes eye_points.json beside itself.
"""

import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
QA = os.path.join(HERE, "..", "..", "..", "..", "assets", "characters", "humanoid", "qa")
sys.path.insert(0, os.path.abspath(QA))

import numpy as np  # noqa: E402
from mathutils import Vector  # noqa: E402

import qa_scene as Q  # noqa: E402

ENDS = [("masculine", 0.0, "eye_size_left", -1.0), ("masculine", 0.0, "eye_tilt_left", 1.0),
        ("masculine", 0.0, "eye_tilt_right", 1.0), ("feminine", 1.0, "eye_tilt_right", 1.0)]
RAYS = [Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60)),
        Vector((-0.55, -0.62, -0.56))]


def parity(bvh, p):
    votes = []
    for way in RAYS:
        way = way.normalized()
        at, n = Vector(p), 0
        for _ in range(64):
            hit = bvh.ray_cast(at, way, 3.0)
            if hit[0] is None:
                break
            n += 1
            at = hit[0] + way * 1e-6
        votes.append(n % 2)
    return votes


def main():
    parts, _default, custom_path = Q.package_paths("realistic")
    custom = Q.load_custom(custom_path)
    ch = Q.Character(parts, merge_vertices=True, custom=custom)
    ch.show_only(["Body", "Eyes", "Teeth"])
    out = []
    for frame_label, frame, pid, end in ENDS:
        ch.pose_reset()
        ch.set_sliders({pid: end, "frame": frame})
        Q.update()
        body, eyes = Q.Geo(ch.parts["Body"]), Q.Geo(ch.parts["Eyes"])
        pn = Q.PseudoNormals(body.co, body.tris)
        # edges with one face: the body's open rims
        counts = np.bincount(pn.edge_of.ravel(), minlength=len(pn.edge))
        for side, s in Q.SIDES:
            c = ch.eye_centre(side)
            for i in np.where(eyes.co[:, 0] * s > 0)[0]:
                p = Vector(eyes.co[i])
                d = p - c
                if d.length < 1e-9 or (-d.y / d.length) < math.cos(math.radians(60)):
                    continue
                loc, nrm, f, dist = body.bvh.find_nearest(p, 0.03)
                if loc is None:
                    continue
                new = pn.signed(np.array(p), f)
                old = (p - loc).dot(nrm)
                if min(new, old) >= -0.0002:
                    continue
                _q, _n, (kind, index) = pn.closest(np.array(p), f)
                edge = index if kind == "edge" else None
                out.append({"end": f"{frame_label} {pid}={end:g}", "side": side, "vertex": int(i),
                            "pseudo_normal_mm": round(new * 1000, 3), "face_normal_mm": round(old * 1000, 3),
                            "nearest": kind, "open_edge": bool(edge is not None and counts[edge] == 1),
                            "faces_on_edge": int(counts[edge]) if edge is not None else None,
                            "ray_parity": parity(body.bvh, p)})
                print(out[-1], flush=True)
    with open(os.path.join(HERE, "eye_points.json"), "w") as fh:
        json.dump(out, fh, indent=1)


main()
