"""The last the low-top sneakers are built over (Blender's Python with
NumPy): a smooth form made from the realistic foot, as a shoemaker's last
is made from a measured foot.

A last is not the foot: it is the foot's volume made smooth (no toes, no
creases), with room in front of the toes and over them (the toe box) and
the allowance a shoe needs round the rest. Here it is an implicit surface:

1. The foot's signed distance on a 2 mm grid (Blender's BVH of the skin,
   the sign from the nearest point's normal).
2. The toe box: the distance taken as the smaller of the foot's and the
   foot's moved forward along its length (`TOE_ROOM` at the toes, nothing
   behind the ball), so the form runs on past the toe tips.
3. A Gaussian blur (`BLUR` mm): the toes' gaps and the skin's small
   creases go, the form stays the foot's.

A surface `level` mm off the foot is then `{p: last(p) = level}`; points
are put on it by Newton steps along the field's gradient (`onto`), and
outlines at a height are traced round it (`outline`).

Coordinates are Blender's (metres; Z up, the body facing -Y). The foot's
frame: `s` along its length from the back of the heel (0) to the toe tip
(1), `w` across it (+ outward, to the little toe's side), `h` up.
"""

import math

import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

VOXEL = 0.002
BLUR = 4.0          # mm, the Gaussian's sigma
TOE_ROOM = 0.011    # m the toe box runs on past the toes
PAD = 0.03          # m round the foot's box
SOFT = 0.0015       # m, the smooth minimum's width


class Foot:
    """One foot of the body in the rest pose: its points, its frame and its
    measures, and the last made from it."""

    def __init__(self, body, side):
        """`body`: clothing/body.py's Body (skin positions `P`, triangles
        `tris`, joints); `side` +1 the left foot (+X), -1 the right."""
        self.side = side
        P = body.P
        leg = (P[:, 0] * side > 0.02) & (P[:, 2] < 0.32)
        self.leg = leg
        tris = body.tris[leg[body.tris].all(axis=1)]
        self.bvh = BVHTree.FromPolygons([tuple(p) for p in P], [tuple(t) for t in tris])
        self.points = P[leg & (P[:, 2] < 0.13)]
        low = self.points[self.points[:, 2] < 0.03]
        heel = low[np.argmax(low[:, 1])]
        toe = low[np.argmin(low[:, 1])]
        d = toe[:2] - heel[:2]
        self.length = float(np.linalg.norm(d))
        self.d = d / self.length
        lateral = np.array([-self.d[1], self.d[0]])
        if lateral[0] * side < 0:
            lateral = -lateral
        self.l = lateral
        self.heel = heel[:2].copy()
        # Width: the footprint's widest across (at the ball), and where.
        rel = low[:, :2] - self.heel
        s = rel @ self.d / self.length
        w = rel @ self.l
        best = (0.0, 0.0)
        for sc in np.linspace(0.5, 0.85, 36):
            near = np.abs(s - sc) < 0.02
            if near.sum() > 3:
                width = float(w[near].max() - w[near].min())
                if width > best[0]:
                    best = (width, float(sc))
        self.width, self.width_at = best
        self.toe_joint = None
        bones = body.joints
        name = "LeftToes" if side > 0 else "RightToes"
        if name in bones:
            self.toe_joint = float(self.s_of(np.asarray(bones[name])))
        self.ankle = np.asarray(bones["LeftFoot" if side > 0 else "RightFoot"])
        # The foot's middle line across (w), from heel to toe: halfway
        # between its outer and inner sides 2 to 4 cm up (the heel-to-toe
        # line runs along the big toe, the inner side).
        allp = self.points
        sa, wa = self.s_of(allp), self.w_of(allp)
        band = (allp[:, 2] > 0.02) & (allp[:, 2] < 0.04)
        mids = []
        grid_s = np.linspace(0.0, 1.0, 41)
        for sc in grid_s:
            near = band & (np.abs(sa - sc) < 0.03)
            mids.append(0.5 * (wa[near].max() + wa[near].min()) if near.sum() > 3 else np.nan)
        mids = np.array(mids)
        ok = ~np.isnan(mids)
        mids = np.interp(grid_s, grid_s[ok], mids[ok])
        for _ in range(3):
            mids[1:-1] = 0.25 * mids[:-2] + 0.5 * mids[1:-1] + 0.25 * mids[2:]
        self.mid_s, self.mid_w_table = grid_s, mids
        self._grid()

    # The foot's frame.
    def s_of(self, p):
        p = np.asarray(p, dtype=float)
        return (p[..., :2] - self.heel) @ self.d / self.length

    def w_of(self, p):
        p = np.asarray(p, dtype=float)
        return (p[..., :2] - self.heel) @ self.l

    def at(self, s, w, h):
        xy = self.heel + self.d * (s * self.length) + self.l * w
        return np.array([xy[0], xy[1], h])

    def mid_w(self, s):
        return float(np.interp(s, self.mid_s, self.mid_w_table))

    def mid(self, s, w, h):
        """The point `w` across from the foot's middle line at `s`, at
        height `h`."""
        return self.at(s, self.mid_w(s) + w, h)

    def axis(self, s):
        """A point well inside the foot at `s` (the section's middle, low)."""
        near = self.points[np.abs(self.s_of(self.points) - s) < 0.03]
        near = near[near[:, 2] < 0.06]
        if len(near) == 0:
            return self.at(s, 0.0, 0.03)
        c = near.mean(axis=0)
        return np.array([c[0], c[1], min(max(c[2], 0.02), 0.04)])

    def _grid(self):
        lo = self.points.min(axis=0) - PAD
        hi = self.points.max(axis=0) + PAD
        lo[2] = -0.012
        hi[2] = 0.15
        n = np.ceil((hi - lo) / VOXEL).astype(int) + 1
        self.lo, self.n = lo, n
        xs = lo[0] + np.arange(n[0]) * VOXEL
        ys = lo[1] + np.arange(n[1]) * VOXEL
        zs = lo[2] + np.arange(n[2]) * VOXEL
        X, Y, Z = np.meshgrid(xs, ys, zs, indexing="ij")
        pts = np.stack([X.ravel(), Y.ravel(), Z.ravel()], axis=1)
        dist = np.empty(len(pts))
        for k, p in enumerate(pts):
            loc, nor, _, dd = self.bvh.find_nearest(Vector(p))
            if loc is None:
                dist[k] = 0.05
                continue
            sign = 1.0 if (p - np.array(loc)) @ np.array(nor) >= 0 else -1.0
            dist[k] = sign * dd
        field = dist.reshape(n)
        self.raw = field
        # The toe box: the field moved forward along the foot ahead of the
        # ball, the smaller of the two kept (both forms' union).
        s = self.s_of(pts)
        shift = TOE_ROOM * np.clip((s - 0.62) / 0.3, 0, 1)
        shift = shift * shift * (3 - 2 * shift)
        back = pts.copy()
        back[:, 0] -= self.d[0] * shift
        back[:, 1] -= self.d[1] * shift
        moved = self._sample(field, back)
        field = np.minimum(field, moved.reshape(n))
        # Smooth (a separable Gaussian, BLUR mm).
        sigma = BLUR / (VOXEL * 1000)
        radius = int(math.ceil(3 * sigma))
        k = np.exp(-0.5 * (np.arange(-radius, radius + 1) / sigma) ** 2)
        k /= k.sum()
        for axis in range(3):
            padded = np.pad(field, [(radius, radius) if a == axis else (0, 0) for a in range(3)], mode="edge")
            out = np.zeros_like(field)
            for j, kj in enumerate(k):
                sl = [slice(None)] * 3
                sl[axis] = slice(j, j + field.shape[axis])
                out += kj * padded[tuple(sl)]
            field = out
        # Never nearer the foot than the field says: where the blur rounds a
        # convex end in (the toe tip), the foot's own distance is kept, by
        # a smooth minimum (`SOFT` m wide) so no crease is left.
        a, b = field, self.raw
        soft = SOFT
        lo = np.minimum(a, b)
        field = lo - soft * np.log(np.exp(-(a - lo) / soft) + np.exp(-(b - lo) / soft))
        self.field = field
        self.gx, self.gy, self.gz = np.gradient(field, VOXEL)

    def _sample(self, grid, pts):
        """Trilinear samples of a grid at points (outside the grid: its edge)."""
        q = (np.asarray(pts, dtype=float) - self.lo) / VOXEL
        q = np.clip(q, 0, np.array(self.n) - 1.001)
        i = np.floor(q).astype(int)
        f = q - i
        out = np.zeros(len(q))
        for dx in (0, 1):
            for dy in (0, 1):
                for dz in (0, 1):
                    w = (f[:, 0] if dx else 1 - f[:, 0]) * (f[:, 1] if dy else 1 - f[:, 1]) * \
                        (f[:, 2] if dz else 1 - f[:, 2])
                    out += w * grid[i[:, 0] + dx, i[:, 1] + dy, i[:, 2] + dz]
        return out

    def value(self, pts):
        pts = np.atleast_2d(pts)
        return self._sample(self.field, pts)

    def gradient(self, pts):
        pts = np.atleast_2d(pts)
        g = np.stack([self._sample(self.gx, pts), self._sample(self.gy, pts), self._sample(self.gz, pts)], axis=1)
        return g

    def normal(self, pts):
        g = self.gradient(pts)
        return g / np.maximum(np.linalg.norm(g, axis=1, keepdims=True), 1e-9)

    def onto(self, pts, level):
        """Points moved onto the surface `level` (m, a number or one per
        point) off the foot, along the field's gradient."""
        p = np.atleast_2d(np.asarray(pts, dtype=float)).copy()
        level = np.broadcast_to(np.asarray(level, dtype=float), (len(p),))
        for _ in range(12):
            v = self.value(p) - level
            g = self.gradient(p)
            gg = np.maximum((g * g).sum(axis=1), 1e-6)
            step = (v / gg)[:, None] * g
            # At most 6 mm a step: far points come in over several.
            m = np.linalg.norm(step, axis=1, keepdims=True)
            step *= np.minimum(1.0, 0.006 / np.maximum(m, 1e-12))
            p -= step
        return p

    def march(self, origin, direction, level, reach=0.15):
        """The first point from `origin` (inside) along `direction` where
        the field reaches `level`."""
        direction = np.asarray(direction, dtype=float)
        direction = direction / np.linalg.norm(direction)
        ts = np.arange(0.0, reach, 0.0005)
        pts = np.asarray(origin)[None] + ts[:, None] * direction[None]
        v = self.value(pts) - level
        out = np.nonzero(v >= 0)[0]
        if len(out) == 0:
            return pts[-1]
        k = out[0]
        if k == 0:
            return pts[0]
        a, b = v[k - 1], v[k]
        f = -a / (b - a)
        return pts[k - 1] + (pts[k] - pts[k - 1]) * f

    def outline(self, height, level, n=720):
        """The surface `level` off the foot cut at `height` (m): a closed
        loop round the foot, started at the back of the heel and going
        round the outer side first, resampled to `n` points evenly."""
        centre = self.at(0.5, 0.0, height)
        # The heel's and the toe's middles, to keep rays inside the form.
        pts = []
        for k in range(n):
            ang = 2 * math.pi * k / n
            dirn = np.array([math.cos(ang), math.sin(ang), 0.0])
            pts.append(self.march(centre, dirn, level, reach=0.25))
        loop = np.array(pts)
        return order_loop(self, resample_closed(loop, n))


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


def order_loop(foot, loop):
    """`loop` started at its point farthest back (the heel's middle) and
    going round the outer side first."""
    s = foot.s_of(loop)
    w = foot.w_of(loop)
    k0 = int(np.argmin(s - 0.02 * np.abs(w)))
    loop = np.roll(loop, -k0, axis=0)
    if foot.w_of(loop[len(loop) // 8]) < 0:
        loop = np.vstack([loop[:1], loop[1:][::-1]])
    return loop


def arc(points, closed=False):
    pts = np.vstack([points, points[:1]]) if closed else points
    seg = np.linalg.norm(np.diff(pts, axis=0), axis=1)
    return np.concatenate([[0], np.cumsum(seg)])


def smoothstep(a, b, x):
    t = np.clip((np.asarray(x, dtype=float) - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)
