"""The lips' line from the front in a smile: is its middle a point? (R9 review 2)

    bpy-python -I lipline.py --repo REPO [--parts GLB] --out FILE.json [--happy EXTRA.json]

The lips' line is the upper lip's edge: the ring of the body's vertices
shared by skin and the mouth's lining, walked from the frontmost middle
vertex out to each corner. Per pose (rest, the closed smile, Happy, and
any in EXTRA.json, {name: {control: weight}}), both frames: the middle
vertex's height over the chord of its two neighbours (`mid_over_chord_mm`;
negative, it sags under them: a point downward), and the line's turn at
each of the seven middle vertices (`turns_deg`, the change of slope seen
from the front; positive bends up, as a smile's curve does). A smooth
smile turns a little and alike at the middle and its neighbours; a point
turns hard at the middle and back at each neighbour.
"""
import argparse
import json
import math
import os
import sys

import bpy  # noqa: F401
import numpy as np

ap = argparse.ArgumentParser()
ap.add_argument("--repo", required=True)
ap.add_argument("--parts", default="")
ap.add_argument("--out", required=True)
ap.add_argument("--happy", default="", help="JSON of extra poses {name: weights}")
a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
sys.path.insert(0, os.path.join(a.repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q  # noqa: E402
import face_checks as FC  # noqa: E402


def rings(obj):
    me = obj.data
    names = [m.name.split(".")[0] if m else "" for m in me.materials]
    skin_v, mouth_v = set(), set()
    mouth_faces = {}
    for p in me.polygons:
        n = names[p.material_index] if names else ""
        if n == "Skin":
            skin_v.update(p.vertices)
        elif n == "Mouth":
            mouth_v.update(p.vertices)
            for v in p.vertices:
                mouth_faces.setdefault(v, []).append(p.index)
    both = sorted(skin_v & mouth_v)
    return both, mouth_faces


def main():
    parts_path, _, custom_path = Q.package_paths("realistic")
    parts_path = a.parts or parts_path
    custom = Q.load_custom(custom_path)
    Q.reset()
    ch = Q.Character(parts_path, merge_vertices=True, custom=custom)
    body = ch.parts["Body"]
    both, mouth_faces = rings(body)
    ex = FC.expressions()
    poses = [("rest", {}), ("Smile closed", {"Mouth_Smile_Left": 1.0, "Mouth_Smile_Right": 1.0}),
             ("Happy", ex["Happy"])]
    if a.happy:
        with open(a.happy, encoding="utf-8") as f:
            poses += list(json.load(f).items())
    report = {}
    for frame_name, frame in (("masculine", 0.0), ("feminine", 1.0)):
        FC.pose(ch, frame, {})
        rest = FC.Piece(body).co
        me = body.data
        # The lips' line: from the frontmost middle vertex of the ring
        # (skin and lining both), walk out each way along the ring's
        # edges, |x| growing, keeping to the upper of two choices.
        bset = set(both)
        adj = {v: set() for v in both}
        for e in me.edges:
            i, j = e.vertices
            if i in bset and j in bset:
                adj[i].add(j)
                adj[j].add(i)
        mids = [v for v in both if abs(rest[v][0]) < 1e-4 and rest[v][2] > 1.0]
        start = min(mids, key=lambda v: rest[v][1])
        line = [start]
        for s in (-1.0, 1.0):
            at, path = start, []
            while True:
                nxt = [n for n in adj[at] if s * (rest[n][0] - rest[at][0]) > 1e-5 and n not in path]
                if not nxt:
                    break
                at = max(nxt, key=lambda n: rest[n][2])
                path.append(at)
            line = (path[::-1] + line) if s < 0 else (line + path)
        upper = sorted(line, key=lambda v: rest[v][0])
        lower = upper
        for name, w in poses:
            FC.pose(ch, frame, w)
            co = FC.Piece(body).co
            out = {}
            for ring_name, idx in (("line", upper),):
                xz = np.array([[co[v][0], co[v][2]] for v in idx])
                k = int(np.argmin(np.abs(xz[:, 0])))
                turns = []
                for j in range(max(1, k - 3), min(len(xz) - 1, k + 4)):
                    s1 = math.atan2(xz[j][1] - xz[j - 1][1], xz[j][0] - xz[j - 1][0])
                    s2 = math.atan2(xz[j + 1][1] - xz[j][1], xz[j + 1][0] - xz[j][0])
                    turns.append(round(math.degrees(s2 - s1), 2))
                chord = 0.5 * (xz[k - 1][1] + xz[k + 1][1])
                out[ring_name] = {"n": len(idx), "mid_over_chord_mm": round((xz[k][1] - chord) * 1000, 3),
                                  "turns_deg": turns, "mid_turn_deg": turns[min(3, k - 1)] if turns else None,
                                  "x_mm": [round(x * 1000, 1) for x in xz[:, 0]], "dz_rest_mm": [round(float(co[v][2] - rest[v][2]) * 1000, 2) for v in idx], "dy_rest_mm": [round(float(co[v][1] - rest[v][1]) * 1000, 2) for v in idx],
                                  "z_mm": [round((z - xz[k][1]) * 1000, 2) for z in xz[max(0, k - 3):k + 4, 1]]}
            report.setdefault(frame_name, {})[name] = out
            print(frame_name, name, json.dumps(out), flush=True)
    with open(a.out, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=1)


main()
