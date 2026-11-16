"""MakeHuman's corner mixing, and the same thing in the form the engine's
base model manifest takes (architecture.md, sections 6 and 7).

MakeHuman's rule: a macro slider in 0..1 splits into three factors (min,
average, max; `mh.factors`), and each corner target (gender x muscle x
weight) is weighted by the product of its factors.

The engine's rule: a shape's weight is the sum of its drivers' spans plus
its corners', a corner being the product of its factors' spans, each span
monotone. "Average" (1 - min - max) is not a monotone span, so the corner
targets are re-expressed. Per frame, with D[m][w] a corner's offsets from
that frame's neutral (D[average][average] = 0):

    single-axis shapes     Muscle_Min, Muscle_Max, Weight_Min, Weight_Max
                           = D[min][avg], D[max][avg], D[avg][min], D[avg][max]
    interaction shapes     X[m][w] = D[m][w] - D[m][avg] - D[avg][w]
                           for m, w in {min, max}

and the body is  neutral + sum a_m * Muscle_m + sum b_w * Weight_w
+ sum a_m * b_w * X[m][w], with a_min, a_max, b_min, b_max MakeHuman's
min and max factors (monotone). Expanding MakeHuman's products with
a_avg = 1 - a_min - a_max gives exactly this; `check` confirms it
numerically. Each frame's eight shapes are scaled by that frame's share
(masculine: 1 - frame; feminine: frame), which is one more factor.

Parameters here: `frame` 0 (masculine) to 1 (feminine); `muscle` and
`weight` -1 to 1 with 0 the average (MakeHuman's slider is (v + 1) / 2).

    python -I mix.py --work WORK     checks the two rules agree, counts the
                                     crossings mixtures add, and prints the
                                     shapes' sizes and the manifest entries
"""

import argparse
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import mh  # noqa: E402

FRAMES = ("masculine", "feminine")
AXIS = {"min": "Min", "max": "Max"}


def makehuman_weights(frame, muscle, weight):
    """{(frame name, corner name): weight} by MakeHuman's rule."""
    out = {}
    corners = mh.corner_weights((muscle + 1) / 2, (weight + 1) / 2)
    for name, share in zip(FRAMES, (1.0 - frame, frame)):
        for (m, w), value in corners.items():
            if (m, w) != ("averagemuscle", "averageweight"):
                out[(name, f"{m}_{w}")] = share * value
    return out


def engine_shapes(shapes, space="posed"):
    """{engine shape name: offsets} from the transferred corner shapes
    (`transfer.py`'s `shapes.npz`), in `space` (`posed` or `bundle`)."""
    out = {}
    for frame in FRAMES:
        D = lambda m, w: shapes[f"{space}/{frame}/{m}muscle_{w}weight"].astype(np.float64)  # noqa: E731
        F = frame.capitalize()
        for k, K in AXIS.items():
            out[f"{F}_Muscle_{K}"] = D(k, "average")
            out[f"{F}_Weight_{K}"] = D("average", k)
        for m, Mn in AXIS.items():
            for w, Wn in AXIS.items():
                out[f"{F}_Muscle_{Mn}_Weight_{Wn}"] = D(m, w) - D(m, "average") - D("average", w)
    return out


def manifest():
    """The manifest entries: parameters and corners (architecture.md 6)."""
    span = {"min": {"from": [0, -1], "to": [0, 1]}, "max": {"from": [0, 1]}}
    share = {"masculine": {"parameter": "frame", "from": [1, 0], "to": [0, 1]},
             "feminine": {"parameter": "frame", "from": [0, 1]}}
    corners = []
    for frame in FRAMES:
        F = frame.capitalize()
        for k, K in AXIS.items():
            corners.append({"shape": f"{F}_Muscle_{K}",
                            "factors": [share[frame], {"parameter": "muscle", **span[k]}]})
            corners.append({"shape": f"{F}_Weight_{K}",
                            "factors": [share[frame], {"parameter": "weight", **span[k]}]})
        for m, Mn in AXIS.items():
            for w, Wn in AXIS.items():
                corners.append({"shape": f"{F}_Muscle_{Mn}_Weight_{Wn}",
                                "factors": [share[frame], {"parameter": "muscle", **span[m]},
                                            {"parameter": "weight", **span[w]}]})
    parameters = [
        {"id": "frame", "label": "Frame", "group": "build", "range": [0, 1], "default": 0,
         "shapes": [{"shape": "Frame_Feminine"}]},
        {"id": "muscle", "label": "Muscle", "group": "build", "range": [-1, 1], "default": 0},
        {"id": "weight", "label": "Weight", "group": "build", "range": [-1, 1], "default": 0},
    ]
    return {"parameters": parameters, "corners": corners}


def span(value, source, to=None):
    """`avatar::span`: `value` through `source` onto `to` (clamped)."""
    to = source if to is None else to
    t = min(max((value - source[0]) / (source[1] - source[0]), 0.0), 1.0)
    return to[0] + (to[1] - to[0]) * t


def engine_weights(values, entries=None):
    """{shape: weight} by the engine's rule (`avatar/src/resolve.rs`):
    each corner is the product of its factors' spans."""
    entries = entries or manifest()
    out = {}
    for corner in entries["corners"]:
        w = 1.0
        for f in corner["factors"]:
            w *= span(values[f["parameter"]], f.get("from", [0, 1]), f.get("to"))
        out[corner["shape"]] = out.get(corner["shape"], 0.0) + w
    return out


def body(G, shapes, frame, muscle, weight, space="posed", engine=None):
    """The realistic body at a point of the space, by the engine's rule.
    `G` is the registration (for the posed frames) or {frame: positions}
    (`space` "bundle")."""
    key = (lambda f: f"posed_{f}") if space == "posed" else (lambda f: f)
    P = G[key("masculine")] * (1.0 - frame) + G[key("feminine")] * frame
    engine = engine or engine_shapes(shapes, space)
    for name, w in engine_weights({"frame": frame, "muscle": muscle, "weight": weight}).items():
        if w:
            P = P + engine[name] * w
    return P


def body_makehuman_rule(G, shapes, frame, muscle, weight, space="posed"):
    """The same body by MakeHuman's rule over the 16 corner shapes."""
    key = (lambda f: f"posed_{f}") if space == "posed" else (lambda f: f)
    P = G[key("masculine")] * (1.0 - frame) + G[key("feminine")] * frame
    for (name, corner), w in makehuman_weights(frame, muscle, weight).items():
        if w:
            P = P + shapes[f"{space}/{name}/{corner}"].astype(np.float64) * w
    return P


def makehuman_body(M, gender_value, muscle, weight):
    """MakeHuman's own mesh at the same point: `gender_value` 0 male to 1
    female (our `frame`)."""
    P = M["neutral_male"] * (1.0 - gender_value) + M["neutral_female"] * gender_value
    corners = mh.corner_weights((muscle + 1) / 2, (weight + 1) / 2)
    for gender, share in (("male", 1.0 - gender_value), ("female", gender_value)):
        for (m, w), value in corners.items():
            if share * value:
                P = P + M[f"corner_{gender}_{m}_{w}"] * (share * value)
    return P


def check(G, shapes):
    """The largest difference between the two rules over a grid of the
    space, in metres."""
    worst = 0.0
    engine = engine_shapes(shapes)
    for frame in (0.0, 0.3, 1.0):
        for muscle in (-1.0, -0.4, 0.0, 0.5, 1.0):
            for weight in (-1.0, -0.6, 0.0, 0.25, 1.0):
                a = body(G, shapes, frame, muscle, weight, engine=engine)
                b = body_makehuman_rule(G, shapes, frame, muscle, weight)
                worst = max(worst, float(np.abs(a - b).max()))
    return worst


def mixture_crossings(work, G, shapes, steps=(-1.0, -0.5, 0.0, 0.5, 1.0)):
    """The crossings each point of a muscle x weight grid adds to its
    frame's neutral (`transfer.new_crossings`), and at frame 0.5 against
    the half-way neutral: {(frame, muscle, weight): count} of the points
    that add any."""
    import geom
    import transfer
    tris = geom.triangles(np.load(os.path.join(work, "realistic.npz"))["faces"])
    engine = engine_shapes(shapes)
    out = {}
    for frame in (0.0, 0.5, 1.0):
        P = body(G, shapes, frame, 0.0, 0.0, engine=engine)
        known = transfer.crossing_pairs(P, tris)
        for muscle in steps:
            for weight in steps:
                V = body(G, shapes, frame, muscle, weight, engine=engine)
                n = len(transfer.new_crossings(P, V, tris, known))
                if n:
                    out[(frame, muscle, weight)] = n
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    G = np.load(os.path.join(a.work, "registration.npz"))
    shapes = np.load(os.path.join(a.work, "shapes.npz"))
    print(f"engine rule against MakeHuman's rule over 75 points: largest difference "
          f"{check(G, shapes) * 1e6:.3f} micrometres")
    added = mixture_crossings(a.work, G, shapes)
    print(f"points of the 5 x 5 x 3 grid (muscle x weight x frame 0, 0.5, 1) that add crossings: "
          f"{len(added)} of 75")
    for (frame, muscle, weight), n in added.items():
        print(f"  frame {frame}, muscle {muscle:+.1f}, weight {weight:+.1f}: {n} pairs")
    engine = engine_shapes(shapes)
    for name, d in engine.items():
        size = np.linalg.norm(d, axis=1)
        print(f"  {name:38s} largest move {size.max() * 1000:6.1f} mm, moves {int((size > 1e-4).sum()):5d} vertices")
    print(json.dumps(manifest(), indent=1))


if __name__ == "__main__":
    main()
