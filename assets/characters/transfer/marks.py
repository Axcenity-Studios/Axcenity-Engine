"""The navel and the nipples of both bodies, for the registration to pin
(CO5): three marks the realistic body's base mesh can't show (its belly
quads are 25 mm wide) but its sculpt and its normal map do, and that
MakeHuman's shapes fold and swell around.

    PY -I marks.py --work WORK        (after extract.py; writes WORK/marks.npz)

- On MakeHuman's mesh they are read from its topology: its navel and
  nipples are the centres of small dense rings, so each is the vertex
  with the shortest edges in its part of the trunk's front.
- On ours they are read from the vendored sculpt
  (`realistic/source/sculpt_*.npz`, rebuilt as the build does): the navel
  is the deepest narrow dent on the belly's middle line, a nipple the
  highest narrow bump on its side of the chest.

Both are in the bundle's pose, where the trunk is as the registration's
pose has it (only the limbs are turned).
"""

import argparse
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import features  # noqa: E402
import geom  # noqa: E402
import skeleton  # noqa: E402

FRAMES = (("masculine", "male"), ("feminine", "female"))


def _trunk(lm):
    crotch = lm["crotch"][2]
    return crotch, (lm["l-shoulder"][2] + lm["r-shoulder"][2]) / 2 - crotch


def by_density(V, faces, lm):
    """{navel, l-nipple, r-nipple} on a mesh that models them as dense
    rings, and each one's edge length over the typical one there."""
    e = geom.edges(faces)
    length = np.linalg.norm(V[e[:, 0]] - V[e[:, 1]], axis=1)
    total, count = np.zeros(len(V)), np.zeros(len(V))
    for k in (0, 1):
        np.add.at(total, e[:, k], length)
        np.add.at(count, e[:, k], 1)
    mean = total / np.maximum(count, 1)
    crotch, trunk = _trunk(lm)
    up = (V[:, 2] - crotch) / trunk
    front = V[:, 1] < lm["crotch"][1] - 0.02
    half = abs(lm["l-shoulder"][0])
    out, clear = {}, {}

    def pick(name, mask):
        idx = np.flatnonzero(mask & front)
        i = idx[np.argmin(mean[idx])]
        out[name], clear[name] = V[i].copy(), float(mean[i] / np.median(mean[idx]))

    pick("navel", (np.abs(V[:, 0]) < 0.002) & (up > 0.25) & (up < 0.6))
    for side, s in features.SIDES:
        x = V[:, 0] * s
        pick(f"{side}-nipple", (x > 0.25 * half) & (x < 0.85 * half) & (up > 0.55) & (up < 0.9))
    return out, clear


def on_sculpt(V, tris, lm):
    """{navel, l-nipple, r-nipple} on a sculpted body, and how far each
    stands in or out of the smooth skin round it, in millimetres."""
    depth = features.Depth(V, tris)
    crotch, trunk = _trunk(lm)
    half = abs(lm["l-shoulder"][0])
    out, clear = {}, {}
    zs = np.arange(crotch + 0.25 * trunk, crotch + 0.6 * trunk, 0.001)
    line = depth.column(0.0, zs, features.FRONT)
    dent = features._smooth(line, 15) - line
    i = int(np.nanargmax(dent[20:-20])) + 20
    out["navel"], clear["navel"] = np.array([0.0, depth.at(0.0, zs[i], features.FRONT), zs[i]]), dent[i] * 1000
    for side, s in features.SIDES:
        xs = np.arange(0.25, 0.85, 0.01) * half * s
        zs = np.arange(crotch + 0.55 * trunk, crotch + 0.9 * trunk, 0.002)
        grid = np.array([depth.column(x, zs, features.FRONT) for x in xs])
        # Against the skin round it: a box mean about 12 mm each way.
        k = 6
        smooth = np.full_like(grid, np.nan)
        kx = max(1, int(round(0.012 / abs(xs[1] - xs[0]))))
        for a in range(grid.shape[0]):
            for b in range(grid.shape[1]):
                smooth[a, b] = np.nanmean(grid[max(0, a - kx):a + kx + 1, max(0, b - k):b + k + 1])
        bump = (grid - smooth)[kx:-kx, k:-k]
        a, b = np.unravel_index(np.nanargmax(bump), bump.shape)
        x, z = xs[a + kx], zs[b + k]
        out[f"{side}-nipple"] = np.array([x, depth.at(x, z, features.FRONT), z])
        clear[f"{side}-nipple"] = bump[a, b] * 1000
    return out, clear


def sculpts():
    """Both frames' sculpts in the bundle's pose: {frame: (positions, triangles)}."""
    sys.path.insert(0, os.path.join(HERE, "..", "realistic"))
    import body
    import skin
    from common import reset_scene, set_positions
    reset_scene()
    low, _, _, source = body.load()
    out = {}
    for frame in ("Masculine", "Feminine"):
        set_positions(low, source[frame])
        high = skin.sculpt(low, frame)
        mesh = high.data
        co = np.empty(len(mesh.vertices) * 3)
        mesh.vertices.foreach_get("co", co)
        mesh.calc_loop_triangles()
        tris = np.empty(len(mesh.loop_triangles) * 3, dtype=np.int32)
        mesh.loop_triangles.foreach_get("vertices", tris)
        out[frame.lower()] = (co.reshape(-1, 3), tris.reshape(-1, 3))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    M = np.load(os.path.join(a.work, "makehuman.npz"))
    R = np.load(os.path.join(a.work, "realistic.npz"))
    sets = skeleton.Sets(R["faces"], R["face_sets"])
    names = M["joint_names"].tolist()
    high = sculpts()
    out = {}
    for frame, gender in FRAMES:
        N = M[f"neutral_{gender}"]
        joints = dict(zip(names, M[f"joints_{gender}"]))
        lm_m = skeleton.makehuman_landmarks(joints, N)
        lm_r = skeleton.realistic_landmarks(R[frame], sets, skeleton.hip_rule(joints, N))
        theirs, clear_m = by_density(N, M["faces"], lm_m)
        V, tris = high[frame]
        # The sculpt is the vendored one (the build's scale may differ from the extract's).
        scale = np.ptp(R[frame][:, 2]) / np.ptp(V[:, 2])
        ours, clear_r = on_sculpt(V * scale, tris, lm_r)
        for key in sorted(theirs):
            out[f"makehuman_{frame}_{key}"] = theirs[key]
            out[f"realistic_{frame}_{key}"] = ours[key]
            for tag, lm, p, c in (("MakeHuman", lm_m, theirs[key], clear_m[key]), ("ours", lm_r, ours[key], clear_r[key])):
                crotch, trunk = _trunk(lm)
                print(f"{frame:9s} {key:9s} {tag:9s} x {p[0]:+.3f}  up {(p[2] - crotch) / trunk:.3f} trunks  clear {c:.2f}")
    np.savez(os.path.join(a.work, "marks.npz"), **out)


if __name__ == "__main__":
    main()
