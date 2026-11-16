"""The head's topology and small geometry helpers the face modules share
(Character Studio overhaul CO6): loops round the eyes and the mouth found
from the face sets, distances along the skin, vertex normals, the nearest
point of a surface, and the eyeball as a surface the lids slide on.

Everything here works on arrays (positions `P` as an n x 3 array in the
bundle's vertex order, polygons as tuples of vertex indices, one face set
per polygon), so the build and the QA scripts use the same code. Units
are metres, Blender axes: Z up, the face looks along -Y, the character's
left is +X.
"""

import heapq
import math

import bpy  # noqa: F401  (the bpy module brings mathutils with it)
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

FS_SOCKET = {1: 2, -1: 3}
FS_EAR = {1: 4, -1: 5}
FS_MOUTH_UPPER = 7
FS_MOUTH_LOWER = 8
FS_HEAD = 17
FS_MUZZLE = 22
HEAD_SETS = (FS_HEAD, FS_MOUTH_UPPER, FS_MOUTH_LOWER, FS_MUZZLE, 2, 3, 4, 5)
SIDES = (("Left", 1), ("Right", -1))

# The cornea: a cap of this radius (of the eyeball's) standing on the
# limbus, which lies this many degrees off the eye's axis.
LIMBUS_DEG = 29.0
CORNEA_RADIUS = 0.64


def smoothstep(e0, e1, x):
    x = np.asarray(x, dtype=np.float64)
    if e0 == e1:
        return (x >= e1).astype(np.float64)
    t = np.clip((x - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


class Topo:
    """Neighbours, face sets by vertex and the borders between sets."""

    def __init__(self, polys, sets, count):
        self.polys = [tuple(p) for p in polys]
        self.sets = list(sets)
        self.count = count
        self.nbr = [set() for _ in range(count)]
        self.vsets = [set() for _ in range(count)]
        self.of_set = {}
        edge_sets = {}
        for p, s in zip(self.polys, self.sets):
            self.of_set.setdefault(s, set()).update(p)
            for a, b in zip(p, p[1:] + p[:1]):
                self.nbr[a].add(b)
                self.nbr[b].add(a)
                edge_sets.setdefault((min(a, b), max(a, b)), []).append(s)
            for i in p:
                self.vsets[i].add(s)
        self.border = {}
        for (a, b), ss in edge_sets.items():
            if len(ss) == 2 and ss[0] != ss[1]:
                self.border.setdefault(frozenset(ss), set()).update((a, b))
        self.edges = np.array(sorted(edge_sets), dtype=np.int64)
        tris = []
        for p in self.polys:
            tris += [(p[0], p[k], p[k + 1]) for k in range(1, len(p) - 1)]
        self.tris = np.array(tris, dtype=np.int64)
        self.head = np.array(sorted(set().union(*(self.of_set.get(s, set()) for s in HEAD_SETS))), dtype=np.int64)
        self.is_head = np.zeros(count, dtype=bool)
        self.is_head[self.head] = True

    def ring(self, a, b):
        return sorted(self.border.get(frozenset((a, b)), set()))

    def verts(self, *sets):
        return np.array(sorted(set().union(*(self.of_set.get(s, set()) for s in sets))), dtype=np.int64)

    def levels(self, start, allowed, most=40):
        """Loops by steps along edges from `start`: [start, 1 step, ...]
        through vertices `allowed(i)` accepts."""
        seen = set(start)
        front, out = list(start), [list(start)]
        for _ in range(most):
            nxt = []
            for i in front:
                for j in self.nbr[i]:
                    if j not in seen and allowed(j):
                        seen.add(j)
                        nxt.append(j)
            if not nxt:
                break
            out.append(nxt)
            front = nxt
        return out

    def steps(self, start, allowed=None, most=60):
        """Each vertex's number of edge steps from `start` (a large number
        where it can't be reached)."""
        out = np.full(self.count, 10 ** 6, dtype=np.int64)
        for k, level in enumerate(self.levels(start, allowed or (lambda i: True), most)):
            out[level] = k
        return out

    def geodesic(self, P, sources, reach=1.0, allowed=None):
        """Distance along the mesh's edges from the nearest of `sources`
        (index -> starting distance, or a list at 0), out to `reach`."""
        dist = np.full(self.count, np.inf)
        heap = []
        items = sources.items() if isinstance(sources, dict) else ((i, 0.0) for i in sources)
        for i, d in items:
            dist[i] = d
            heap.append((d, i))
        heapq.heapify(heap)
        while heap:
            d, i = heapq.heappop(heap)
            if d > dist[i] or d > reach:
                continue
            for j in self.nbr[i]:
                if allowed is not None and not allowed[j]:
                    continue
                nd = d + float(np.linalg.norm(P[i] - P[j]))
                if nd < dist[j]:
                    dist[j] = nd
                    heapq.heappush(heap, (nd, j))
        return dist

    def normals(self, P):
        """Area-weighted vertex normals."""
        a, b, c = (P[self.tris[:, k]] for k in range(3))
        face = np.cross(b - a, c - a)
        out = np.zeros_like(P)
        for k in range(3):
            np.add.at(out, self.tris[:, k], face)
        return out / np.maximum(np.linalg.norm(out, axis=1, keepdims=True), 1e-12)

    def smooth(self, values, rounds, keep=None, among=None):
        """`values` (per vertex, any width) averaged with the neighbours
        `rounds` times; vertices in `keep` hold theirs; only vertices in
        `among` (default: the head) change."""
        values = np.array(values, dtype=np.float64)
        idx = self.head if among is None else np.asarray(among)
        held = np.zeros(self.count, dtype=bool)
        if keep is not None:
            held[np.asarray(keep)] = True
        idx = idx[~held[idx]]
        for _ in range(rounds):
            new = values.copy()
            for i in idx:
                nb = list(self.nbr[i])
                new[i] = 0.5 * values[i] + 0.5 * values[nb].mean(axis=0)
            values = new
        return values

    def bvh(self, P, faces=None):
        """A BVH over the triangles at `P` (of polygons `faces`, a set of
        polygon indices, or all)."""
        if faces is None:
            tris = self.tris
        else:
            tris = []
            for k in faces:
                p = self.polys[k]
                tris += [(p[0], p[j], p[j + 1]) for j in range(1, len(p) - 1)]
        return BVHTree.FromPolygons([tuple(p) for p in P], [tuple(int(i) for i in t) for t in tris], all_triangles=True)


def nearest(tree, p, reach=0.05):
    """(point, normal, signed distance) of the nearest point of `tree` to
    `p`: the distance is negative behind the surface. None out of reach."""
    loc, normal, _, _ = tree.find_nearest(Vector(p), reach)
    if loc is None:
        return None
    return np.array(loc), np.array(normal), float((Vector(p) - loc).dot(normal))


class Eyeball:
    """The eyeball as the lids know it: a sphere of radius `r` about `c`
    with the cornea's cap about its axis (`forward`, the way it looks at
    rest). `height(d)` is how far the surface is from the centre along
    the direction `d`."""

    def __init__(self, c, r, forward=(0.0, -1.0, 0.0)):
        self.c = np.asarray(c, dtype=np.float64)
        self.r = float(r)
        self.f = np.asarray(forward, dtype=np.float64)
        limbus = math.radians(LIMBUS_DEG)
        self.limbus_cos = math.cos(limbus)
        self.rc = CORNEA_RADIUS * self.r
        ring = self.r * math.sin(limbus)
        # The cap's centre on the axis, and its top.
        self.zc = self.r * math.cos(limbus) - math.sqrt(self.rc ** 2 - ring ** 2)
        self.apex = self.zc + self.rc

    def height(self, d):
        """Surface distance from the centre along unit directions `d`."""
        d = np.atleast_2d(d)
        cosine = d @ self.f
        # Along d: |t d - zc f| = rc  ->  t = zc cos + sqrt(rc^2 - zc^2 sin^2)
        under = self.rc ** 2 - self.zc ** 2 * (1.0 - cosine ** 2)
        cap = self.zc * cosine + np.sqrt(np.maximum(under, 0.0))
        return np.where((cosine > self.limbus_cos) & (under > 0), np.maximum(cap, self.r), self.r)

    def above(self, P):
        """How far points `P` are above the eyeball's surface (negative
        inside it)."""
        d = np.atleast_2d(P) - self.c
        length = np.linalg.norm(d, axis=1)
        return length - self.height(d / np.maximum(length, 1e-12)[:, None])

    def lift(self, P, least):
        """`P` moved straight out from the centre where they are nearer
        the surface than `least` (a number or one per point)."""
        P = np.atleast_2d(np.array(P, dtype=np.float64))
        d = P - self.c
        length = np.linalg.norm(d, axis=1)
        unit = d / np.maximum(length, 1e-12)[:, None]
        want = self.height(unit) + least
        return self.c + unit * np.maximum(length, want)[:, None]


def rotate_about(P, origin, axis, angles):
    """Points `P` turned about the line through `origin` along unit
    `axis`, each by its own angle (radians)."""
    P = np.atleast_2d(P)
    d = P - origin
    angles = np.broadcast_to(np.asarray(angles, dtype=np.float64), (len(P),))
    cos, sin = np.cos(angles)[:, None], np.sin(angles)[:, None]
    along = (d @ axis)[:, None] * axis
    return origin + d * cos + np.cross(axis, d) * sin + along * (1.0 - cos)
