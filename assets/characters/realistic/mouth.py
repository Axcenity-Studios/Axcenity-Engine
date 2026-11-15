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
SEAL = {0: 0.0, 1: 0.55, 2: 1.0, 3: 1.0, 4: 0.8, 5: 0.45, 6: 0.15}
# The loop of the lower lip that meets the upper lip: its top's front.
CREST = 3
# The lips touch with this much room (metres).
TOUCH = 0.00015


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
        out[idx, 2] += rise * share
        return out, float((rise * share).max())
