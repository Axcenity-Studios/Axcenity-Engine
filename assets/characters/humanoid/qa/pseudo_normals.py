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
    can't at a sharp edge (the CO6 eye check's three false counts).
    Corners are welded by position, so a vertex split at a UV seam still
    finds its neighbours. Plain numpy, so it can be tested without Blender
    (`test_pseudo_normals.py`)."""

    WELD = 1e-7  # m: corners closer than this are one
    EDGE = 1e-6  # barycentric weight under which a point is on an edge

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

    def normal(self, loc, face):
        """The pseudo-normal at `loc`, a point of triangle `face`."""
        a, b, c = (self.co[self.tris[face, k]] for k in range(3))
        v0, v1, v2 = b - a, c - a, loc - a
        d00, d01, d11 = v0 @ v0, v0 @ v1, v1 @ v1
        d20, d21 = v2 @ v0, v2 @ v1
        den = d00 * d11 - d01 * d01
        if abs(den) < 1e-30:
            return self.face[face]
        w1 = (d11 * d20 - d01 * d21) / den
        w2 = (d00 * d21 - d01 * d20) / den
        w = (1.0 - w1 - w2, w1, w2)
        on_edge = [j for j in range(3) if w[j] < self.EDGE]
        if len(on_edge) >= 2:  # at the corner the other weight belongs to
            corner = next(j for j in range(3) if j not in on_edge)
            return self.corner[self.corner_of[face, corner]]
        if len(on_edge) == 1:  # on the edge opposite that corner
            return self.edge[self.edge_of[face, (on_edge[0] + 1) % 3]]
        return self.face[face]

    def signed(self, p, loc, face):
        """|p - loc|, negative when `p` is inside: `loc` is the nearest
        point to `p`, on triangle `face`."""
        way = p - loc
        dist = float(np.linalg.norm(way))
        return -dist if way @ self.normal(loc, face) < 0 else dist
