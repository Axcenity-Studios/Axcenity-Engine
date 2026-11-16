"""Angle-weighted pseudo-normals, to sign a distance to a mesh. Plain
numpy (no Blender), so `test_pseudo_normals.py` runs anywhere:
`python3 -m unittest test_pseudo_normals` in this folder."""

import numpy as np


class PseudoNormals:
    """Angle-weighted pseudo-normals (Baerentzen and Aanaes, 2005) of a
    triangle mesh: the face's normal inside a face, the sum of its faces'
    normals on an edge, and the angle-weighted sum of its faces' normals
    at a corner. The sign of (p - nearest) . pseudo-normal tells inside
    from outside wherever the nearest point is, where one face's normal
    can't at a sharp edge (the CO6 eye check's false counts).
    Corners are welded by position, so a vertex split at a UV seam still
    finds its neighbours. Plain numpy, so it can be tested without Blender
    (`test_pseudo_normals.py`)."""

    WELD = 1e-7  # m: corners closer than this are one

    def __init__(self, co, tris):
        co = np.asarray(co, dtype=np.float64)
        tris = np.asarray(tris, dtype=np.int64)
        self.co, self.tris = co, tris
        weld = np.unique(np.round(co / self.WELD).astype(np.int64), axis=0, return_inverse=True)[1].ravel()
        t = weld[tris]
        a, b, c = (co[tris[:, k]] for k in range(3))
        fn = np.cross(b - a, c - a)
        fn /= np.maximum(np.linalg.norm(fn, axis=1, keepdims=True), 1e-18)
        self.face = fn

        def angle(at, q, r):
            u, v = q - at, r - at
            return np.arctan2(np.linalg.norm(np.cross(u, v), axis=1), (u * v).sum(axis=1))

        self.corner = np.zeros((int(weld.max()) + 1, 3))
        for k, (at, q, r) in enumerate(((a, b, c), (b, c, a), (c, a, b))):
            np.add.at(self.corner, t[:, k], fn * angle(at, q, r)[:, None])
        self.corner_of = t
        # edge k of a triangle joins its corners k and k + 1
        pairs = np.stack([np.sort(np.stack([t[:, k], t[:, (k + 1) % 3]], axis=1), axis=1) for k in range(3)], axis=1)
        _, edge_id = np.unique(pairs.reshape(-1, 2), axis=0, return_inverse=True)
        self.edge_of = edge_id.ravel().reshape(-1, 3)
        self.edge = np.zeros((int(self.edge_of.max()) + 1, 3))
        np.add.at(self.edge, self.edge_of.ravel(), np.repeat(fn, 3, axis=0))

    def closest(self, p, face):
        """The point of triangle `face` nearest to `p`, worked out again in
        double precision (Ericson, Real-Time Collision Detection, 5.1.5),
        its pseudo-normal (of the corner, the edge or the face that point
        is on) and which that is: ("corner" | "edge" | "face", index). A tree's nearest point is in single precision, so a
        tolerance on it reads a point on an edge as inside the face (the
        Stage 3 diagnostic, `docs/development/character-studio/stage3/`);
        the regions here are exact."""
        a, b, c = (self.co[self.tris[face, k]] for k in range(3))
        corner = lambda j: (self.corner[self.corner_of[face, j]], ("corner", int(self.corner_of[face, j])))  # noqa: E731
        edge = lambda k: (self.edge[self.edge_of[face, k]], ("edge", int(self.edge_of[face, k])))  # noqa: E731
        ab, ac, ap = b - a, c - a, p - a
        d1, d2 = ab @ ap, ac @ ap
        if d1 <= 0 and d2 <= 0:
            return (a, *corner(0))
        bp = p - b
        d3, d4 = ab @ bp, ac @ bp
        if d3 >= 0 and d4 <= d3:
            return (b, *corner(1))
        vc = d1 * d4 - d3 * d2
        if vc <= 0 and d1 >= 0 and d3 <= 0:
            return (a + ab * (d1 / (d1 - d3)), *edge(0))
        cp = p - c
        d5, d6 = ab @ cp, ac @ cp
        if d6 >= 0 and d5 <= d6:
            return (c, *corner(2))
        vb = d5 * d2 - d1 * d6
        if vb <= 0 and d2 >= 0 and d6 <= 0:
            return (a + ac * (d2 / (d2 - d6)), *edge(2))
        va = d3 * d6 - d5 * d4
        if va <= 0 and d4 - d3 >= 0 and d5 - d6 >= 0:
            return (b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6))), *edge(1))
        denom = va + vb + vc
        if abs(denom) < 1e-30:  # a degenerate triangle
            return (a, *corner(0))
        return a + ab * (vb / denom) + ac * (vc / denom), self.face[face], ("face", int(face))

    def signed(self, p, face):
        """Distance from `p` to triangle `face` (the nearest one a tree
        found), negative when `p` is inside the mesh."""
        q, normal, _ = self.closest(np.asarray(p, dtype=np.float64), face)
        way = p - q
        dist = float(np.linalg.norm(way))
        return -dist if way @ normal < 0 else dist
