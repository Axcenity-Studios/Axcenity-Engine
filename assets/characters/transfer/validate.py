"""A check of the correspondence where the right answer is known.

    python -I validate.py --work WORK

(`python` has the `bpy` module; run `extract.py` first.)

MakeHuman's male neutral is wrapped onto MakeHuman's **female** neutral
with the same warp and fit `register.py` uses, treating the female mesh as
a surface of unknown topology. The two share one topology, so the truth is
known: wrapped vertex `i` should land on the female mesh's vertex `i`.
The distance between them is the correspondence error: how far along the
skin a transferred offset can be misplaced.

This is an easier case than the real one (the landmarks are the same kind
on both sides, the pose is the same, and both are MakeHuman's style of
body), so read the result as a lower bound on the real error. It still
has a male-to-female change of shape: breasts, hips, shoulders, height.
"""

import argparse
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import geom  # noqa: E402
import skeleton  # noqa: E402
from register import fit  # noqa: E402


def parts(V, lm):
    """A body part per vertex, by where it is."""
    out = np.full(len(V), "trunk", dtype=object)
    x, z = np.abs(V[:, 0]), V[:, 2]
    wrist, elbow, shoulder = (lm[f"l-{k}"] for k in ("wrist", "elbow", "shoulder"))
    along = (elbow - shoulder) / np.linalg.norm(elbow - shoulder)
    mirrored = V * np.array([1.0, 1.0, 1.0])
    mirrored[:, 0] = x
    t = (mirrored - shoulder) @ along
    upper = np.linalg.norm(elbow - shoulder)
    lower = np.linalg.norm(wrist - elbow)
    lateral = x > shoulder[0] + 0.02
    out[lateral & (t > 0.03) & (z < shoulder[2] + 0.03)] = "arms"
    out[lateral & (t > upper + lower - 0.01)] = "hands"
    out[z < lm["crotch"][2]] = "legs"
    out[z < lm["l-ankle"][2]] = "feet"
    out[(z > lm["l-shoulder"][2] + 0.05) & (x < 0.12)] = "head and neck"
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    M = np.load(os.path.join(a.work, "makehuman.npz"))
    names = M["joint_names"].tolist()
    A, B = M["neutral_male"], M["neutral_female"]
    lm_a = skeleton.makehuman_landmarks(dict(zip(names, M["joints_male"])), A)
    lm_b = skeleton.makehuman_landmarks(dict(zip(names, M["joints_female"])), B)
    src, dst, _ = skeleton.control_pairs(lm_a, lm_b)
    warped = geom.rbf_warp(src, dst, A)
    tris, edges = geom.triangles(M["faces"]), geom.edges(M["faces"])
    wrapped = fit(warped, tris, edges, B, tris, log=lambda *_: None)
    part = parts(B, lm_b)
    print("| Part | Vertices | Before any alignment: median mm | After the landmark warp: median, 95% mm | "
          "After the fit: median, 95%, largest mm |")
    print("|---|---|---|---|---|")
    for name in ("all", "trunk", "head and neck", "arms", "hands", "legs", "feet"):
        pick = np.ones(len(B), dtype=bool) if name == "all" else part == name
        e0 = np.linalg.norm(A - B, axis=1)[pick] * 1000
        e1 = np.linalg.norm(warped - B, axis=1)[pick] * 1000
        e2 = np.linalg.norm(wrapped - B, axis=1)[pick] * 1000
        print(f"| {name} | {int(pick.sum())} | {np.median(e0):.1f} | {np.median(e1):.1f}, {np.percentile(e1, 95):.1f} "
              f"| {np.median(e2):.1f}, {np.percentile(e2, 95):.1f}, {e2.max():.1f} |")


if __name__ == "__main__":
    main()
