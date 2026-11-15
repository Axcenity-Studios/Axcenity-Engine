"""Step 2 of the MakeHuman transfer spike: the correspondence. Wraps
MakeHuman's base mesh onto the realistic body, once per frame (MakeHuman's
male neutral onto the masculine body, its female neutral onto the feminine
one), and writes `WORK/registration.npz`.

    python -I register.py --work WORK

(`python` has the `bpy` module; run `extract.py` first.)

Per frame:

1. **Pose.** The realistic body stands with its arms about 70 degrees
   below level; MakeHuman's about 50. The realistic body's arms, hands
   and legs are turned to MakeHuman's directions (`skeleton.repose`), so
   the two are compared in one pose. All later steps work in that pose;
   `transfer.py` turns the result back.
2. **Warp.** MakeHuman's mesh is moved by a smooth space warp that
   carries its landmarks (joints, crown, crotch, eyes, finger joints,
   heels, toes) onto the realistic body's.
3. **Fit.** A non-rigid fit pulls MakeHuman's surface onto the realistic
   one: each vertex is drawn to the nearest point of the realistic
   surface that faces the same way, while the field of moves is kept
   smooth; the smoothness is relaxed in stages. Both directions pull (the
   realistic surface's vertices also draw the nearest MakeHuman vertices),
   so parts MakeHuman's nearest-point search misses (finger tips, ears)
   are still covered.

The result per frame: `posed_<frame>` (the realistic body in MakeHuman's
pose), `wrapped_<frame>` (MakeHuman's 13,380 vertices on it) and
`warped_<frame>` (after step 2 only, for the measurements).
"""

import argparse
import os
import sys
import time

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import geom  # noqa: E402
import skeleton  # noqa: E402

FRAMES = (("masculine", "male"), ("feminine", "female"))
# Stages of the fit: (stiffness, iterations, reach in metres).
STAGES = ((200.0, 4, 0.06), (60.0, 4, 0.05), (20.0, 5, 0.04), (6.0, 6, 0.03), (2.0, 6, 0.02))
FACING = 0.35  # the least cosine between normals for a match


class Surface:
    """Nearest points on a triangle mesh that face a given way."""

    def __init__(self, V, tris):
        import bpy  # noqa: F401 (the bpy module brings mathutils)
        from mathutils.bvhtree import BVHTree
        self.V, self.tris = V, tris
        self.tree = BVHTree.FromPolygons([tuple(v) for v in V], [tuple(int(i) for i in t) for t in tris])
        fn = geom.face_normals(V, tris)
        self.fn = fn / np.maximum(np.linalg.norm(fn, axis=1, keepdims=True), 1e-12)

    def nearest(self, points, normals, reach):
        """Per point: the nearest surface point whose face normal agrees
        with the point's, its triangle, and whether one was found."""
        hit = np.zeros((len(points), 3))
        tri = np.full(len(points), -1, dtype=np.int64)
        for i, (p, n) in enumerate(zip(points, normals)):
            loc, _, idx, dist = self.tree.find_nearest(tuple(p), reach)
            if loc is None:
                continue
            if self.fn[idx] @ n < FACING:
                best = None
                for loc2, _, idx2, dist2 in self.tree.find_nearest_range(tuple(p), reach):
                    if self.fn[idx2] @ n >= FACING and (best is None or dist2 < best[2]):
                        best = (loc2, idx2, dist2)
                if best is None:
                    continue
                loc, idx = best[0], best[1]
            hit[i] = loc
            tri[i] = idx
        return hit, tri, tri >= 0


def barycentric(p, a, b, c):
    v0, v1, v2 = b - a, c - a, p - a
    d00 = np.einsum("ij,ij->i", v0, v0)
    d01 = np.einsum("ij,ij->i", v0, v1)
    d11 = np.einsum("ij,ij->i", v1, v1)
    d20 = np.einsum("ij,ij->i", v2, v0)
    d21 = np.einsum("ij,ij->i", v2, v1)
    den = np.maximum(d00 * d11 - d01 * d01, 1e-18)
    v = (d11 * d20 - d01 * d21) / den
    w = (d00 * d21 - d01 * d20) / den
    return np.stack([1 - v - w, v, w], axis=1)


def fit(X0, tris_x, edges_x, target, tris_t, log=print):
    """`X0` (MakeHuman's mesh after the warp) fitted to the surface
    `target`. Returns the fitted positions."""
    n = len(X0)
    lap = geom.Laplacian(n, edges_x)
    area = geom.vertex_areas(X0, tris_x)
    area /= area.mean()
    surf = Surface(target, tris_t)
    nt = geom.vertex_normals(target, tris_t)
    area_t = geom.vertex_areas(target, tris_t)
    area_t /= area_t.mean()
    U = np.zeros_like(X0)
    for stiffness, iterations, reach in STAGES:
        for _ in range(iterations):
            X = X0 + U
            nx = geom.vertex_normals(X, tris_x)
            hit, _, ok = surf.nearest(X, nx, reach)
            w = np.where(ok, area, 0.0)
            goal = np.where(ok[:, None], hit - X0, 0.0)
            # The other way: each target vertex draws the corners of the
            # nearest triangle of the fitted mesh toward itself.
            mine = Surface(X, tris_x)
            back, tri, ok_t = mine.nearest(target, nt, reach)
            t = tris_x[tri[ok_t]]
            bary = barycentric(back[ok_t], X[t[:, 0]], X[t[:, 1]], X[t[:, 2]])
            pull = target[ok_t] - back[ok_t]
            wb = np.zeros(n)
            gb = np.zeros((n, 3))
            for k in range(3):
                wk = bary[:, k] * area_t[ok_t] * 0.5
                np.add.at(wb, t[:, k], wk)
                np.add.at(gb, t[:, k], wk[:, None] * (X[t[:, k]] + pull - X0[t[:, k]]))
            total = w + wb
            goal = (goal * w[:, None] + gb) / np.maximum(total, 1e-12)[:, None]
            U = geom.smooth_fit(lap, total, goal, stiffness, U, 400)
        gap = np.linalg.norm(hit - X, axis=1)[ok]
        log(f"    stiffness {stiffness:6.1f}: matched {ok.mean() * 100:5.1f}% of MakeHuman's vertices, "
            f"{ok_t.mean() * 100:5.1f}% of ours; gap median {np.median(gap) * 1000:.2f} mm, "
            f"95% {np.percentile(gap, 95) * 1000:.2f} mm")
    return X0 + U


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    M = np.load(os.path.join(a.work, "makehuman.npz"))
    R = np.load(os.path.join(a.work, "realistic.npz"))
    faces_r, face_sets = R["faces"], R["face_sets"]
    tris_r = geom.triangles(faces_r)
    tris_m = geom.triangles(M["faces"])
    edges_m = geom.edges(M["faces"])
    sets = skeleton.Sets(faces_r, face_sets)
    names = M["joint_names"].tolist()
    out = {}
    for frame, gender in FRAMES:
        start = time.time()
        P = R[frame]
        N = M[f"neutral_{gender}"]
        joints = dict(zip(names, M[f"joints_{gender}"]))
        lm_m = skeleton.makehuman_landmarks(joints, N)
        lm_r = skeleton.realistic_landmarks(P, sets, skeleton.hip_rule(joints, N))
        posed, lm_p = skeleton.repose(P, faces_r, lm_r, lm_m)
        src, dst, keys = skeleton.control_pairs(lm_m, lm_p)
        warped = geom.rbf_warp(src, dst, N)
        print(f"{frame}: posed (moved up to {np.linalg.norm(posed - P, axis=1).max() * 1000:.0f} mm); "
              f"landmarks moved MakeHuman by a median {np.median(np.linalg.norm(dst - src, axis=1)) * 1000:.0f} mm")
        wrapped = fit(warped, tris_m, edges_m, posed, tris_r)
        out[f"posed_{frame}"] = posed
        out[f"warped_{frame}"] = warped
        out[f"wrapped_{frame}"] = wrapped
        out[f"landmarks_{frame}"] = np.array([lm_p[k] for k in sorted(lm_p)])
        out["landmark_names"] = np.array(sorted(lm_p))
        print(f"  {time.time() - start:.0f} s")
    np.savez_compressed(os.path.join(a.work, "registration.npz"), **out)


if __name__ == "__main__":
    main()
