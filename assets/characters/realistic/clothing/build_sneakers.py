"""Builds the low-top sneakers' sources (Blender's Python, bpy 5.x). A
shoe is a firm shell, not a drape (references/clothing §5.1, "Shell"):
the upper is laid over the realistic foot (its last) with room at the
toes, and a sole is cemented under it. Writes into `--out` the same
files as `build_tee.py`, named `Sneakers_*`.

    python -I assets/characters/realistic/clothing/build_sneakers.py \
        --body assets/characters/realistic/RealisticParts.glb --tiles <folder> --out <folder>

Each shoe, in the foot's own frame (`s` from the heel's back, 0, to the
toe's tip, 1; `w` across, + outward):

- **Upper** (white leather, `Upper`): a ring of quads between the sole
  line (tucked 4 mm into the midsole) and the collar opening: the heel's
  topline, down under the ankle bone at the sides, along the eyestays to
  the throat over the instep. Each column runs over the foot's surface
  `GAP` off it, more at the toes (the toe box). The toe cap and heel
  counter are overlays drawn by the maps (seam and two stitch rows), the
  eyelets punched holes.
- **Collar** (a textile lining, `Lining`): the opening's edge rolled
  inward over a pad and carried down inside, so no edge is one layer.
- **Tongue** (`Tongue`): a padded slab under the laces, from the throat
  to above the opening.
- **Laces** (`Laces`): five rungs across the tongue, closed round cords.
- **Sole**: a closed foam midsole (`Midsole`) with a rubber outsole
  (`Outsole`) whose tread is flat on the floor; the foot stands inside
  it, as a shoe's footbed is (the body stands on the floor).
"""

import argparse
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np
import bpy  # noqa: F401 (loads mathutils)
from mathutils import Vector
from mathutils.bvhtree import BVHTree

import body as bodymod
import finish
import textures
from build_tee import Atlas

# Detail at each level: columns round the upper, rows up it, the sole's
# outline corners, the tongue's grid, a lace's points.
LEVELS = ((36, 8, 36, 5, 4), (24, 5, 24, 3, 3), (16, 3, 16, 2, 2))
GAP = 0.004
TOE_ROOM = 0.014
SOLE_TOP_HEEL = 0.030
SOLE_TOP_TOE = 0.022
SOLE_MARGIN = 0.008
PAD = 0.003
LINING_DOWN = 0.022
TONGUE_THICK = 0.004
LACE_RADIUS = 0.0018
# The instep's top is looked for under this height (above it, the skin
# over a point of the instep is the ankle's front).
INSTEP_MAX = 0.092
# The collar opening, as (u round the shoe from the heel's back 0, across
# the outer side to the throat 0.5 and back along the inner side; s;
# w as a share of the foot's half width there; height (m) or None for
# the top of the foot there).
OPENING = ((0.0, 0.02, 0.0, 0.082), (0.10, 0.10, 0.85, 0.078), (0.20, 0.30, 1.0, 0.060),
           (0.30, 0.42, 0.62, None), (0.40, 0.53, 0.38, None), (0.50, 0.62, 0.0, None))


class Foot:
    def __init__(self, b, s):
        side = "Left" if s > 0 else "Right"
        self.s = s
        P = b.P
        obj = b.object
        groups = {g.index: g.name for g in obj.vertex_groups}
        foot = np.zeros(len(P), dtype=bool)
        for v in obj.data.vertices:
            for g in v.groups:
                if groups[g.group] in (f"{side}Foot", f"{side}Toes") and g.weight > 0.5:
                    foot[v.index] = True
        foot &= P[:, 2] < 0.13
        self.P = P[foot]
        self.bvh = BVHTree.FromPolygons([tuple(p) for p in b.P], [tuple(t) for t in b.tris])
        low = self.P[self.P[:, 2] < 0.04]
        heel = low[np.argmax(low[:, 1])]
        toe = low[np.argmin(low[:, 1])]
        d = toe[:2] - heel[:2]
        self.length = float(np.linalg.norm(d))
        self.d = d / self.length
        self.l = np.array([self.d[1], -self.d[0]]) * (1 if s > 0 else -1)
        if (self.l @ np.array([s, 0.0])) < 0:
            self.l = -self.l
        self.heel = heel[:2]
        # The footprint's half widths along the foot (outward and inward).
        rel = low[:, :2] - self.heel
        self.fs = rel @ self.d / self.length
        self.fw = rel @ self.l
        self.hull = bodymod.hull_of(low[:, :2])

    def frame(self, s, w):
        return self.heel + self.d * s * self.length + self.l * w

    def half_width(self, s, outward):
        near = np.abs(self.fs - s) < 0.06
        w = self.fw[near]
        if not len(w):
            return 0.03
        return float(w.max() if outward else -w.min())

    def axis(self, s):
        """A point inside the foot at `s` (its section's middle)."""
        xy = self.frame(s, 0.0)
        near = self.P[np.linalg.norm(self.P[:, :2] - xy, axis=1) < 0.02]
        z = 0.5 * (near[:, 2].min() + near[:, 2].max()) if len(near) else 0.04
        return np.array([xy[0], xy[1], z])

    def onto(self, target, s, gap):
        """`target` moved out of the foot until it is `gap` over the skin
        (a point already that far out stays where it is)."""
        p = np.asarray(target, dtype=float)
        for _ in range(3):
            loc, nor, _, _ = self.bvh.find_nearest(Vector(p))
            q, n = np.array(loc), np.array(nor)
            if (p - q) @ n >= gap - 1e-5:
                break
            p = q + n * gap
        return p

    def top_z(self, xy):
        """The top of the foot over `xy`."""
        near = self.P[(np.linalg.norm(self.P[:, :2] - xy, axis=1) < 0.012) & (self.P[:, 2] < INSTEP_MAX)]
        return float(near[:, 2].max()) if len(near) else 0.06

    def s_of(self, p):
        return float((p[:2] - self.heel) @ self.d / self.length)


def resample_closed(points, n):
    pts = np.vstack([points, points[:1]])
    seg = np.linalg.norm(np.diff(pts, axis=0), axis=1)
    acc = np.concatenate([[0], np.cumsum(seg)])
    out = []
    for t in np.linspace(0, acc[-1], n, endpoint=False):
        i = min(np.searchsorted(acc, t, side="right") - 1, len(seg) - 1)
        f = (t - acc[i]) / max(seg[i], 1e-12)
        out.append(pts[i] + (pts[i + 1] - pts[i]) * f)
    return np.array(out)


def outline(foot, n, grow):
    """The footprint's outline grown by `grow`, `n` corners, starting at the
    heel's back and going round the outer side first."""
    hull = foot.hull
    c = hull.mean(axis=0)
    grown = []
    for p in hull:
        d = p - c
        grown.append(p + d / np.linalg.norm(d) * grow)
    ring = resample_closed(np.array(grown), 4 * n)
    # Start at the heel's back; go round the outer side first.
    k0 = int(np.argmax((ring - c) @ -foot.d))
    ring = np.roll(ring, -k0, axis=0)
    if ((ring[len(ring) // 8] - c) @ foot.l) < 0:
        ring = np.vstack([ring[:1], ring[1:][::-1]])
    return resample_closed(ring, n)


def sole_top(s):
    return SOLE_TOP_HEEL + (SOLE_TOP_TOE - SOLE_TOP_HEEL) * min(max(s, 0), 1)


def opening(foot, u):
    """The collar opening's point at `u` (0 to 1 round), on the foot."""
    us = [o[0] for o in OPENING]
    mirror = u > 0.5
    uu = 1 - u if mirror else u
    k = min(np.searchsorted(us, uu, side="right") - 1, len(OPENING) - 2)
    a, b = OPENING[k], OPENING[k + 1]
    f = (uu - a[0]) / (b[0] - a[0])
    f = f * f * (3 - 2 * f)
    s = a[1] + (b[1] - a[1]) * f
    share = a[2] + (b[2] - a[2]) * f
    w = share * foot.half_width(s, not mirror) * (-1 if mirror else 1)
    xy = foot.frame(s, w)
    za, zb = a[3], b[3]
    if za is None or zb is None:
        z_top = foot.top_z(xy)
        za_ = z_top if za is None else za
        zb_ = z_top if zb is None else zb
        z = za_ + (zb_ - za_) * f
    else:
        z = za + (zb - za) * f
    return s, foot.onto(np.array([xy[0], xy[1], z]), s, GAP + PAD)


class Shoe:
    def __init__(self, foot, mesh, level, tag):
        self.foot, self.mesh, self.tag = foot, mesh, tag
        U, V, NS, NT, NL = LEVELS[level]
        self.upper(U, V)
        self.sole(NS)
        self.tongue(NT)
        self.laces(NL)

    def add(self, p, uv, island, kind="shell"):
        return self.mesh.add(p, uv, f"{island}_{self.tag}", kind)

    def upper(self, U, V):
        f, mesh = self.foot, self.mesh
        bottom = outline(f, U, SOLE_MARGIN - 0.003)
        # The opening's loop, smoothed round (a topline is a smooth curve)
        # and kept clear of the skin.
        tops = np.array([opening(f, i / U)[1] for i in range(U)])
        for _ in range(4):
            tops = 0.5 * tops + 0.25 * (np.roll(tops, 1, axis=0) + np.roll(tops, -1, axis=0))
            tops = np.array([f.onto(p, f.s_of(p), GAP + PAD) for p in tops])
        cols = []
        for i in range(U):
            u = i / U
            top = tops[i]
            b2 = bottom[i]
            s_b = f.s_of(np.array([b2[0], b2[1], 0]))
            b3 = np.array([b2[0], b2[1], sole_top(s_b) - 0.004])
            col = [b3]
            for j in range(1, V):
                v = j / V
                t = b3 + (top - b3) * v
                s = f.s_of(t)
                gap = GAP + PAD * v + TOE_ROOM * max(0.0, (s - 0.72) / 0.28) * (1 - v * 0.5)
                p = f.onto(t, s, gap)
                p[2] = max(p[2], sole_top(s) + 0.002 * v)
                col.append(p)
            col.append(top)
            cols.append(np.array(col))
        cols = np.array(cols)                                   # (U, V+1, 3)
        # Smoothed round and up (the clearance push leaves dents where the
        # foot's bones stand out), cleared again, a few times.
        for _ in range(6):
            inner = cols[:, 1:V].copy()
            mean = (np.roll(cols, 1, axis=0)[:, 1:V] + np.roll(cols, -1, axis=0)[:, 1:V]
                    + cols[:, 0:V - 1] + cols[:, 2:V + 1]) / 4
            inner = 0.5 * inner + 0.5 * mean
            for i in range(U):
                for j in range(V - 1):
                    p = inner[i, j]
                    s = f.s_of(p)
                    v = (j + 1) / V
                    gap = GAP + PAD * v + TOE_ROOM * max(0.0, (s - 0.72) / 0.28) * (1 - v * 0.5)
                    p = f.onto(p, s, gap)
                    p[2] = max(p[2], sole_top(s) + 0.002 * v)
                    inner[i, j] = p
            cols[:, 1:V] = inner
        cols = np.concatenate([cols, cols[:1]])
        # The collar: rolled over the pad, then down inside.
        rows = [cols[:, j] for j in range(V + 1)]
        normals = []
        for i in range(U + 1):
            o = cols[i, V]
            n = o - f.axis(min(max(f.s_of(o), 0.05), 0.95))
            n[2] = max(n[2], 0.0) * 0.3
            normals.append(n / np.linalg.norm(n))
        normals = np.array(normals)
        past = cols[:, V] - cols[:, V - 1]
        past /= np.linalg.norm(past, axis=1, keepdims=True)
        roll = cols[:, V] + past * PAD * 0.6 - normals * PAD * 0.6
        fold = cols[:, V] - normals * PAD * 1.2
        down = []
        for i in range(U + 1):
            # The lining runs down inside the upper, `LINING_DOWN` along it.
            k = V
            left = LINING_DOWN
            p = cols[i, V]
            while k > 0 and left > 0:
                step = np.linalg.norm(cols[i, k - 1] - cols[i, k])
                f_ = min(1.0, left / step)
                p = cols[i, k] + (cols[i, k - 1] - cols[i, k]) * f_
                left -= step
                k -= 1
            down.append(p - normals[i] * PAD * 1.2)
        rows += [roll, fold, np.array(down)]
        # Texture coordinates: along each row (mm) and up the columns.
        ids = []
        vacc = np.zeros(U + 1)
        for j, row in enumerate(rows):
            if j:
                vacc += np.linalg.norm(row - rows[j - 1], axis=1) * 1000
            if j <= V:
                arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(row, axis=0), axis=1))]) * 1000
                # Every row lined up at the toe's column (the middle of
                # the strip): rows of different lengths shear the strip,
                # and lined up at one end the shear piles up at the other
                # until neighbouring rows fold over in the texture.
                arc = arc - arc[U // 2] + 400.0
            # The collar's roll and lining keep the collar row's lengths:
            # rows a few millimetres apart with lengths of their own would
            # fold over each other in the texture.
            kind = "shell" if j <= V else "lining"
            ids.append([self.add(row[i], (arc[i], vacc[i]), "upper", kind) for i in range(U + 1)])
        ids = np.array(ids)
        for j in range(len(rows)):
            mesh.welds.append((ids[j, 0], ids[j, U]))
        for j in range(len(rows) - 1):
            material = "Upper" if j < V else "Lining"
            for i in range(U):
                q = (ids[j, i], ids[j, i + 1], ids[j + 1, i + 1], ids[j + 1, i])
                mesh.quad(material, q[::-1] if self.foot.s > 0 else q)
        self.upper_ids = ids
        self.V = V
        self.U = U
        self.top = cols[:, V]

    def sole(self, N):
        f, mesh = self.foot, self.mesh
        ring = outline(f, N, SOLE_MARGIN)
        c = ring.mean(axis=0)
        s_ring = np.array([f.s_of(np.array([p[0], p[1], 0])) for p in ring])
        tops = np.array([sole_top(s) for s in s_ring])

        def grown(by):
            d = ring - c
            return ring + d / np.linalg.norm(d, axis=1, keepdims=True) * by

        layers = [(grown(-0.0025), np.zeros(N)), (grown(0), np.full(N, 0.0025)), (grown(0.0004), np.full(N, 0.008)),
                  (grown(0.0008), np.full(N, 0.012)), (grown(0.0), tops), (grown(-0.004), tops)]
        ids = []
        vacc = 0.0
        prev = None
        for j, (xy, z) in enumerate(layers):
            pts = np.column_stack([xy, z])
            if prev is not None:
                vacc += float(np.mean(np.linalg.norm(pts - prev, axis=1))) * 1000
            prev = pts
            closed = np.vstack([pts, pts[:1]])
            arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(closed, axis=0), axis=1))]) * 1000
            ids.append([self.add(closed[i], (arc[i], vacc), "soleside") for i in range(N + 1)])
        ids = np.array(ids)
        for j in range(len(layers)):
            mesh.welds.append((ids[j, 0], ids[j, N]))
        for j in range(len(layers) - 1):
            material = "Outsole" if j < 2 else "Midsole"
            for i in range(N):
                q = (ids[j, i], ids[j, i + 1], ids[j + 1, i + 1], ids[j + 1, i])
                mesh.quad(material, q[::-1] if self.foot.s > 0 else q)
        # Caps: a ring at 60% toward the middle, then quads fanned.
        for j, material, island, up in ((0, "Outsole", "tread", True), (len(layers) - 1, "Midsole", "insole", False)):
            xy, z = layers[j]
            mid = xy.mean(axis=0)
            inner = mid + (xy - mid) * 0.55
            zc = float(np.mean(z))
            edge = [self.add(np.array([p[0], p[1], zz]), (p[0] * 1000, p[1] * 1000), island) for p, zz in zip(xy, z)]
            for k in range(N):
                mesh.welds.append((edge[k], ids[j, k]))
            inn = [self.add(np.array([p[0], p[1], zc]), (p[0] * 1000, p[1] * 1000), island) for p in inner]
            centre = self.add(np.array([mid[0], mid[1], zc]), (mid[0] * 1000, mid[1] * 1000), island)
            quads = []
            for k in range(N):
                k2 = (k + 1) % N
                quads.append((edge[k], edge[k2], inn[k2], inn[k]))
            for k in range(0, N, 2):
                quads.append((centre, inn[k], inn[(k + 1) % N], inn[(k + 2) % N]))
            for q in quads:
                face = q if (up ^ (self.foot.s < 0)) else q[::-1]
                mesh.quad(material, face)

    def eyestay(self, a, outer):
        """A point of the eyestay at `a` (0 at the throat, 1 at the top),
        on the outer side or the inner."""
        u = 0.5 - a * 0.25 if outer else 0.5 + a * 0.25
        return opening(self.foot, u)

    def tongue(self, N):
        f, mesh = self.foot, self.mesh
        grid = []
        for i in range(N + 1):
            a = -0.05 + 1.12 * i / N
            so, po = self.eyestay(min(a, 1.0), True)
            si, pi = self.eyestay(min(a, 1.0), False)
            row = []
            for j in range(N + 1):
                b = j / N
                t = po + (pi - po) * b
                s = f.s_of(t)
                p = f.onto(t, s, GAP + PAD + 0.0005)
                if a > 1.0:
                    # Past the opening's top the tongue stands up the ankle.
                    p = p + np.array([0.0, 0.0, (a - 1.0) * 0.12])
                row.append(p)
            grid.append(row)
        grid = np.array(grid)
        top = grid + np.array([0, 0, TONGUE_THICK])
        # Texture coordinates: across each row and along each column at
        # their own lengths (mm), the top beside the bottom.
        def layout(g):
            across = np.concatenate([np.zeros((N + 1, 1)), np.cumsum(np.linalg.norm(np.diff(g, axis=1), axis=2), axis=1)], axis=1)
            along = np.concatenate([np.zeros((1, N + 1)), np.cumsum(np.linalg.norm(np.diff(g, axis=0), axis=2), axis=0)], axis=0)
            return across * 1000, along * 1000

        ab, lb = layout(grid)
        at, lt = layout(top)
        width = float(ab.max()) + 10.0
        ids_b, ids_t = [], []
        for i in range(N + 1):
            ids_b.append([self.add(grid[i, j], (ab[i, j], lb[i, j]), "tongue") for j in range(N + 1)])
            ids_t.append([self.add(top[i, j], (at[i, j] + width, lt[i, j]), "tongue") for j in range(N + 1)])
        ids_b, ids_t = np.array(ids_b), np.array(ids_t)
        flip = self.foot.s > 0
        for i in range(N):
            for j in range(N):
                q = (ids_t[i, j], ids_t[i, j + 1], ids_t[i + 1, j + 1], ids_t[i + 1, j])
                mesh.quad("Tongue", q[::-1] if flip else q)
                q = (ids_b[i, j], ids_b[i + 1, j], ids_b[i + 1, j + 1], ids_b[i, j + 1])
                mesh.quad("Tongue", q[::-1] if flip else q)
        # The rim round the slab.
        rim = [(i, 0) for i in range(N)] + [(N, j) for j in range(N)] + \
              [(i, N) for i in range(N, 0, -1)] + [(0, j) for j in range(N, 0, -1)]
        # The rim: a strip of its own (corners welded to the slab's).
        loop = rim + rim[:1]
        along = 0.0
        strip = []
        for n_, (i, j) in enumerate(loop):
            if n_:
                along += float(np.linalg.norm(grid[i, j] - grid[loop[n_ - 1][0], loop[n_ - 1][1]])) * 1000
            b_ = self.add(grid[i, j], (along, 0.0), "tonguerim")
            t_ = self.add(top[i, j], (along, TONGUE_THICK * 1000), "tonguerim")
            mesh.welds += [(b_, ids_b[i, j]), (t_, ids_t[i, j])]
            strip.append((b_, t_))
        for (b0, t0), (b1, t1) in zip(strip, strip[1:]):
            q = (b0, b1, t1, t0)
            mesh.quad("Tongue", q if flip else q[::-1])
        self.tongue_top = top

    def laces(self, points):
        f = self.foot
        for k, a in enumerate(np.linspace(0.1, 0.8, 5)):
            _, po = self.eyestay(a, True)
            _, pi = self.eyestay(a, False)
            path = []
            for t in np.linspace(0, 1, points + 1):
                q = po + (pi - po) * t
                s = f.s_of(q)
                p = f.onto(q, s, GAP + PAD + 0.0005 + TONGUE_THICK + LACE_RADIUS)
                path.append(p)
            tube(self.mesh, np.array(path), LACE_RADIUS, "Laces", f"lace{k}_{self.tag}")


def tube(mesh, path, radius, material, island, sides=4):
    rings = []
    along = 0.0
    for i, p in enumerate(path):
        t = path[min(i + 1, len(path) - 1)] - path[max(i - 1, 0)]
        t /= np.linalg.norm(t)
        u = np.cross(t, [0.0, 0.0, 1.0])
        if np.linalg.norm(u) < 1e-6:
            u = np.cross(t, [1.0, 0.0, 0.0])
        u /= np.linalg.norm(u)
        v = np.cross(t, u)
        if i:
            along += np.linalg.norm(path[i] - path[i - 1]) * 1000
        ring = []
        for k in range(sides + 1):
            ang = 2 * math.pi * k / sides
            ring.append(mesh.add(p + radius * (math.cos(ang) * u + math.sin(ang) * v),
                                 (k * 2 * math.pi * radius * 1000 / sides, along), island, "cord"))
        mesh.welds.append((ring[0], ring[-1]))
        rings.append(ring)
    for a, b in zip(rings, rings[1:]):
        for k in range(sides):
            mesh.quad(material, (a[k], a[k + 1], b[k + 1], b[k]))
    caps = [(0, k, k + 1, k + 2) for k in range(1, sides - 1, 2)]
    for ring, rev in ((rings[0], True), (rings[-1], False)):
        for q in caps:
            qq = tuple(ring[i] for i in q)
            mesh.quad(material, qq[::-1] if rev else qq)


def fabric(tiles, shoes):
    def tile(name, kind):
        return os.path.join(tiles, name, f"{name}_{kind}_2k.jpg")

    def rub(kind):
        return os.path.join(tiles, "rubber004", f"Rubber004_1K-JPG_{kind}.jpg")

    def make(scale):
        leather = {"mm": 300.0, "stretch": 1.0, "amplitude": 0.08, "contrast": 0.35,
                   "height": textures.load_tile(tile("leather_white", "disp"), 300.0, scale),
                   "light": textures.load_tile(tile("leather_white", "diff"), 300.0, scale), "light_mean": 0.55,
                   "rough": textures.load_tile(tile("leather_white", "rough"), 300.0, scale),
                   "rough_lo": 0.38, "rough_hi": 0.55, "base": [0.86, 0.86, 0.84]}
        textile = {"mm": 263.6, "stretch": 1.0, "amplitude": 0.15, "contrast": 0.5,
                   "height": textures.load_tile(tile("cotton_jersey", "disp"), 263.6, scale),
                   "light": textures.load_tile(tile("cotton_jersey", "diff"), 263.6, scale), "light_mean": 0.6,
                   "rough": textures.load_tile(tile("cotton_jersey", "rough"), 263.6, scale),
                   "rough_lo": 0.75, "rough_hi": 0.9, "base": [0.80, 0.80, 0.79]}
        foam = dict(leather, amplitude=0.03, contrast=0.1, rough_lo=0.6, rough_hi=0.7, base=[0.88, 0.87, 0.84])
        rubber = {"mm": 200.0, "stretch": 1.0, "amplitude": 0.25, "contrast": 0.4,
                  "height": textures.load_tile(rub("Displacement"), 200.0, scale),
                  "light": textures.load_tile(rub("Color"), 200.0, scale), "light_mean": 0.12,
                  "rough": textures.load_tile(rub("Roughness"), 200.0, scale),
                  "rough_lo": 0.6, "rough_hi": 0.85, "base": [0.11, 0.11, 0.115]}
        islands = {"upper": leather, "tongue": textile, "insole": textile, "soleside": foam, "tread": rubber,
                   "*": textile}
        seams, stitches = {}, {}
        for shoe in shoes:
            island = f"upper_{shoe.tag}"
            ids = shoe.upper_ids
            uv = np.asarray(shoe.mesh.uv)
            P = np.asarray(shoe.mesh.P)
            V = shoe.V
            # Overlays: the toe cap (where the columns cross s = 0.80) and
            # the heel counter (s = 0.22 on the back), each a seam with two
            # stitch rows; the topline's stitch row 5 mm under the collar.
            lines = []
            for s_cut, back in ((0.80, False), (0.22, True)):
                line = []
                for i in range(ids.shape[1]):
                    col = ids[:V + 1, i]
                    ss = np.array([shoe.foot.s_of(P[k]) for k in col])
                    if back:
                        ss = -ss
                        target = -s_cut
                    else:
                        target = s_cut
                    for j in range(V):
                        if (ss[j] - target) * (ss[j + 1] - target) <= 0 and ss[j] != ss[j + 1]:
                            f = (target - ss[j]) / (ss[j + 1] - ss[j])
                            line.append(uv[col[j]] + (uv[col[j + 1]] - uv[col[j]]) * f)
                            break
                if len(line) > 2:
                    lines.append(np.array(line))
            top_row = np.array([uv[k] for k in ids[V]])
            seams[island] = lines
            stitches[island] = [(l, 2.0) for l in lines] + [(l, 4.5) for l in lines] + [(top_row, 5.0)]
            # Eyelets: punched where each lace meets the eyestay.
            holes = []
            for a in np.linspace(0.1, 0.8, 5):
                for outer in (True, False):
                    _, p = shoe.eyestay(a, outer)
                    k = int(np.argmin(np.linalg.norm(P[ids[:V + 1].reshape(-1)] - p, axis=1)))
                    holes.append((uv[ids[:V + 1].reshape(-1)[k]] + np.array([0.0, -6.0]), 2.2))
            islands[island] = dict(leather, holes=holes)
        return {"base": [0.86, 0.86, 0.84], "stitch_length": 3.0, "thread_rough": 0.5,
                "islands": islands, "seams": seams, "stitches": stitches}
    return make


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--body", required=True)
    parser.add_argument("--tiles", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    b = bodymod.load(args.body, "masculine")
    feet = {"l": Foot(b, 1), "r": Foot(b, -1)}
    report = {"foot_length_mm": feet["l"].length * 1000, "levels": []}
    levels = []
    for level in range(3):
        mesh = finish.Mesh()
        shoes = [Shoe(feet[tag], mesh, level, tag) for tag in ("l", "r")]
        finish.snap(mesh)
        mesh.arrays()
        levels.append((mesh, shoes))
    uv0, scale, offsets, margin = finish.pack(levels[0][0])
    atlas = Atlas(scale, offsets, margin)
    report["atlas"] = {"texels_per_mm": scale, "size": textures.SIZE}
    with open(os.path.join(args.out, "Sneakers.mtl"), "w", newline="\n") as f:
        for name in ("Upper", "Lining", "Tongue", "Laces", "Midsole", "Outsole"):
            f.write(f"newmtl {name}\nKd 0.86 0.86 0.84\nmap_Kd Sneakers_color.jpg\n")
    for k, (mesh, shoes) in enumerate(levels):
        mesh.uv01 = np.array([atlas.to_uv(i, p) for i, p in zip(mesh.island, mesh.uv)])
        all_quads = [q for qs in mesh.quads.values() for q in qs]
        mesh.normals = finish.normals(mesh.P, all_quads, mesh.welds)
        finish.write_obj(os.path.join(args.out, f"Sneakers_LOD{k}.obj"), mesh, mesh.uv01, "Sneakers.mtl")
        report["levels"].append({"corners": len(mesh.P), "triangles": finish.triangles(mesh)})
        print(f"LOD{k}: {len(mesh.P)} corners, {finish.triangles(mesh)} triangles")
    report["bake"] = textures.bake(levels[0][0], {}, atlas, fabric(args.tiles, levels[0][1])(scale),
                                   os.path.join(args.out, "Sneakers"))
    with open(os.path.join(args.out, "Sneakers.json"), "w", newline="\n") as f:
        json.dump(report, f, indent=1, default=float)
    print(json.dumps(report, default=float, indent=1))


if __name__ == "__main__":
    main()
