"""The mouth of the realistic head (Character Studio overhaul CO6): its
landmarks on the bundle's topology, and the lips brought together at rest.

Loops round the mouth, as the face sets give them (all positions below are
the masculine frame's, from the upper lip's edge at the middle; z up, y
back):

    upper lip   `edge`: the line where the upper lip's skin (set 17) meets
                the mouth's lining (set 7), 23 vertices from corner to
                corner; `upper[k]`: loops of skin above it (1: the red
                lip's front, 2: its border, 3...: toward the nose);
                `under[k]`: loops of lining behind it (1: the lip's
                underside, 2...: its inner side, up to the roof)
    lower lip   `floor`: where set 22 meets the lining (set 8), deep
                behind the lip; `lower[k]`: loops from it up the lip's
                inner side (1), over its top (2, 3), down its front (4,
                5) to the fold over the chin (6) and on
    corners     the two vertices where sets 7, 17 and 22 meet

As sculpted the lips rest 2 to 3 mm apart, which shows as a dark slit.
`seal` raises the lower lip's top onto the upper lip's edge.
"""

import numpy as np
from mathutils import Vector

from facetopo import FS_HEAD, FS_MOUTH_LOWER, FS_MOUTH_UPPER, FS_MUZZLE, smoothstep

# How far each loop of the lower lip rises with its top when the lips are
# sealed (by loop, from the floor of the mouth).
SEAL = {0: 0.0, 1: 0.35, 2: 0.75, 3: 1.0, 4: 0.8, 5: 0.45, 6: 0.15}
# The loop of the lower lip that meets the upper lip: its top's front.
CREST = 3
# The lips touch with this much room (metres).
TOUCH = 0.0003


class Mouth:
    def __init__(self, topo, P):
        self.topo = topo
        edge = topo.ring(FS_MOUTH_UPPER, FS_HEAD)
        self.edge = np.array(sorted(edge, key=lambda i: P[i][0]), dtype=np.int64)
        self.mid = int(self.edge[np.argmin(np.abs(P[self.edge][:, 0]))])
        self.corner = {1: int(self.edge[-1]), -1: int(self.edge[0])}
        self.floor = np.array(topo.ring(FS_MOUTH_LOWER, FS_MUZZLE), dtype=np.int64)
        only = lambda fs: (lambda i: topo.vsets[i] == {fs})  # noqa: E731
        self.upper = topo.levels(edge, only(FS_HEAD), most=10)
        self.under = topo.levels(edge, only(FS_MOUTH_UPPER), most=10)
        self.lower = topo.levels(list(self.floor), only(FS_MUZZLE), most=14)
        self.lower_step = {}
        for k, loop in enumerate(self.lower):
            for i in loop:
                self.lower_step[i] = k
        self.half = float(np.abs(P[self.edge][:, 0]).max())

    def line(self, P):
        """The lips' line: (x, y, z) of the upper lip's edge, by x."""
        return P[self.edge]

    def stomion(self, P):
        return P[self.mid]

    def gap(self, P):
        """How far the lower lip's top is under the upper lip, by x: a
        table (x, metres) from rays cast up from the lower lip's top loops
        at the upper lip's faces."""
        topo = self.topo
        faces = {k for k, s in enumerate(topo.sets) if s in (FS_HEAD, FS_MOUTH_UPPER)}
        tree = topo.bvh(P, faces)
        rows = {}
        for i in self.lower[CREST]:
            hit = tree.ray_cast(Vector(P[i]), Vector((0.0, 0.0, 1.0)), 0.02)
            if hit[0] is not None:
                rows[float(P[i][0])] = float(hit[3])
        xs = sorted(rows)
        return np.array(xs), np.array([rows[x] for x in xs])

    def seal(self, P):
        """`P` with the lower lip's top raised to touch the upper lip:
        each column by its own gap, less toward the chin and inside the
        mouth, nothing at the corners. Returns (positions, the largest
        rise in metres)."""
        xs, gaps = self.gap(P)
        gaps = np.maximum(gaps - TOUCH, 0.0)
        out = P.copy()
        idx = np.array(sorted(i for i, k in self.lower_step.items() if k in SEAL), dtype=np.int64)
        share = np.array([SEAL[self.lower_step[i]] for i in idx])
        x = P[idx][:, 0]
        rise = np.interp(x, xs, gaps, left=0.0, right=0.0)
        rise *= 1.0 - smoothstep(0.82 * self.half, 0.98 * self.half, np.abs(x))
        rise = rise * share
        # No vertex rises further than its own room under the upper lip
        # (the lip's top is wider than its crest, and the upper lip
        # overhangs it unevenly): the lips touch, they don't cross.
        topo = self.topo
        tree = topo.bvh(P, {k for k, s in enumerate(topo.sets) if s in (FS_HEAD, FS_MOUTH_UPPER)})
        for k, i in enumerate(idx):
            if rise[k] > 0.0:
                hit = tree.ray_cast(Vector(P[i]), Vector((0.0, 0.0, 1.0)), 0.02)
                if hit[0] is not None:
                    rise[k] = min(rise[k], max(0.0, float(hit[3]) - TOUCH))
        out[idx, 2] += rise
        # And the other way: where a face of the raised lip now lies over
        # a vertex of the upper lip's underside, its corners come down.
        # Not the lip's crest, though, while it has room of its own: the
        # upper lip's underside hangs lower behind its edge than at it,
        # so the loops behind the crest are what lie under it, and a
        # crest pulled down with them left the lips a millimetre apart
        # in front, with the teeth showing between them.
        from mathutils.bvhtree import BVHTree
        sealed = set(int(i) for i in idx)
        tris = [tuple(int(i) for i in t) for t in topo.tris if all(int(i) in sealed for i in t)]
        above = [int(i) for i in self.edge] + [int(i) for k in (1, 2) for i in self.under[k]]
        # (Toward the corners, where the lips are one skin, the crest goes with the rest.)
        crest = set(int(i) for i in self.lower[CREST] if abs(P[i][0]) < 0.7 * self.half)
        reach = 0.004
        for _ in range(12):
            lower = BVHTree.FromPolygons([tuple(q) for q in out], tris, all_triangles=True)
            down = {}
            for i in above:
                hit = lower.ray_cast(Vector(out[i]) + Vector((0.0, 0.0, reach)), Vector((0.0, 0.0, -1.0)), reach + TOUCH)
                if hit[0] is not None:
                    by = reach + TOUCH - float(hit[3])
                    corners = tris[hit[2]]
                    free = [j for j in corners if j not in crest]
                    for j in (free or corners):
                        down[j] = max(down.get(j, 0.0), by * (1.5 if len(free) < 3 else 1.0))
            if not down:
                break
            for j, by in down.items():
                out[j, 2] -= by
        # Each vertex has its room now, but a face between them can still
        # cut through a face of the upper lip (more on the feminine
        # frame, whose lips are less alike left and right): the corners
        # of any that do come down a little, until none does.
        over = [(int(p[0]), int(p[j]), int(p[j + 1])) for p, fs in zip(topo.polys, topo.sets)
                if fs in (FS_HEAD, FS_MOUTH_UPPER) for j in range(1, len(p) - 1)]
        upper = BVHTree.FromPolygons([tuple(q) for q in P], over, all_triangles=True)
        for _ in range(40):
            lower = BVHTree.FromPolygons([tuple(q) for q in out], tris, all_triangles=True)
            crossing = {j for a, b in lower.overlap(upper) if not set(tris[a]) & set(over[b]) for j in tris[a]}
            if not crossing:
                break
            for j in crossing:
                out[j, 2] -= 0.00012
        return out, float((out[idx, 2] - P[idx, 2]).max())
