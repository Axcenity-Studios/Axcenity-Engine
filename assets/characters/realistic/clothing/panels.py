"""Pattern pieces as meshes: each piece is a grid of quads spanned by its
four sides (a Coons patch), in the pattern's own millimetres, so a
garment is made of quads whose texture coordinates are the pattern
itself (straight grain up the piece, no stretch but the drape's).

A piece's four sides run, as a Coons patch has them: `bottom` and `top`
from the left corner to the right, `left` and `right` from the bottom
corner to the top. A side may be several seams in a row (`top` of a
T-shirt front is armhole, shoulder, neckline, neckline, shoulder,
armhole); `segments` names each run of corners so seams can be paired.

The same piece is gridded several times at different spacings: finely
for the cloth simulation, then at each level of detail's spacing. Every
grid of a piece keeps the same seams' corner counts in proportion, and a
corner of a coarse grid is found on the fine one by its place in the
pattern (`Panel.locate`), which is how the drape is carried to each level.
"""

import math

import numpy as np

from freesewing import Path, Point


def resample(points, n):
    """`points` (a polyline, mm) at `n + 1` corners evenly along it."""
    pts = np.asarray(points, dtype=float)
    seg = np.linalg.norm(np.diff(pts, axis=0), axis=1)
    s = np.concatenate([[0.0], np.cumsum(seg)])
    targets = np.linspace(0.0, s[-1], n + 1)
    out = np.empty((n + 1, 2))
    for k, t in enumerate(targets):
        i = min(np.searchsorted(s, t, side="right") - 1, len(seg) - 1)
        f = 0.0 if seg[i] == 0 else (t - s[i]) / seg[i]
        out[k] = pts[i] + (pts[i + 1] - pts[i]) * f
    return out


def path_points(path):
    if isinstance(path, Path):
        return np.array([[p.x, p.y] for p in path.sample(128)])
    return np.asarray(path, dtype=float)


def polyline_length(points):
    return float(np.linalg.norm(np.diff(np.asarray(points), axis=0), axis=1).sum())


def mirror(points):
    p = np.array(points, dtype=float)
    p[:, 0] *= -1
    return p


def chain(runs):
    """Runs of corners joined end to start (each run's first corner is the
    previous run's last), and where each run starts and ends."""
    out = [runs[0][1][0]]
    marks = {}
    for name, pts in runs:
        start = len(out) - 1
        out.extend(pts[1:])
        marks[name] = (start, len(out) - 1)
    return np.array(out), marks


def coons(bottom, top, left, right):
    """The grid spanned by four sides: an array (rows, columns, 2), row 0
    the bottom, row -1 the top, column 0 the left, column -1 the right."""
    ns = len(bottom) - 1
    nt = len(left) - 1
    assert len(top) == ns + 1 and len(right) == nt + 1, (len(bottom), len(top), len(left), len(right))
    # Arc-length parameters along each side, so corners spread as the
    # sides' own corners do.
    def params(pts):
        d = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(pts, axis=0), axis=1))])
        return d / d[-1]
    sb, st = params(bottom), params(top)
    tl, tr = params(left), params(right)
    grid = np.empty((nt + 1, ns + 1, 2))
    p00, p10, p01, p11 = bottom[0], bottom[-1], top[0], top[-1]
    for j in range(nt + 1):
        for i in range(ns + 1):
            # The corner's own (s, t), blending the two sides' spacings.
            t = (1 - (sb[i] + st[i]) / 2) * tl[j] + ((sb[i] + st[i]) / 2) * tr[j]
            s = (1 - t) * sb[i] + t * st[i]
            grid[j, i] = ((1 - t) * bottom[i] + t * top[i] + (1 - s) * left[j] + s * right[j]
                          - ((1 - s) * (1 - t) * p00 + s * (1 - t) * p10 + (1 - s) * t * p01 + s * t * p11))
    # Sides exact.
    grid[0], grid[-1], grid[:, 0], grid[:, -1] = bottom, top, left, right
    return relax(grid)


def relax(grid, rounds=60, keep=0.5):
    """Evens the inside corners (Laplacian smoothing that keeps the sides
    still): a Coons grid under a deep neckline bunches; this spreads it."""
    g = grid.copy()
    for _ in range(rounds):
        mean = (g[:-2, 1:-1] + g[2:, 1:-1] + g[1:-1, :-2] + g[1:-1, 2:]) / 4
        g[1:-1, 1:-1] = keep * g[1:-1, 1:-1] + (1 - keep) * mean
    return g


class Panel:
    """A pattern piece gridded: `uv` (rows, columns, 2) in mm and the
    names of runs of its outline (`segments`: name -> list of (row,
    column) corners in order)."""

    def __init__(self, name, sides, segments):
        self.name = name
        self.uv = coons(*sides)
        self.segments = segments
        rows, cols = self.uv.shape[:2]
        self.rows, self.cols = rows, cols

    def index(self, row, col):
        return row * self.cols + col

    def corners(self):
        return self.uv.reshape(-1, 2)

    def quads(self):
        """Quads, counter-clockwise in the pattern (x right, y down: so
        clockwise on paper, which is the outside seen from the front)."""
        out = []
        for j in range(self.rows - 1):
            for i in range(self.cols - 1):
                out.append((self.index(j, i), self.index(j, i + 1), self.index(j + 1, i + 1), self.index(j + 1, i)))
        return np.array(out, dtype=np.int64)

    def segment(self, name):
        return [self.index(r, c) for r, c in self.segments[name]]

    def locate(self, points):
        """For each point of the pattern (mm), the quad of this grid it is
        in and its place there (bilinear, by Newton's method): (quad
        row, quad column, s, t). Points outside the piece get the nearest
        quad, clamped."""
        uv = self.uv
        centres = (uv[:-1, :-1] + uv[1:, :-1] + uv[:-1, 1:] + uv[1:, 1:]) / 4
        flat = centres.reshape(-1, 2)
        out = []
        for p in np.asarray(points, dtype=float):
            order = np.argsort(np.linalg.norm(flat - p, axis=1))[:8]
            best = None
            for q in order:
                j, i = divmod(int(q), self.cols - 1)
                a, b, c, d = uv[j, i], uv[j, i + 1], uv[j + 1, i + 1], uv[j + 1, i]
                s, t = 0.5, 0.5
                for _ in range(12):
                    f = (1 - s) * (1 - t) * a + s * (1 - t) * b + s * t * c + (1 - s) * t * d - p
                    ds = (1 - t) * (b - a) + t * (c - d)
                    dt = (1 - s) * (d - a) + s * (c - b)
                    m = np.array([[ds[0], dt[0]], [ds[1], dt[1]]])
                    try:
                        step = np.linalg.solve(m, f)
                    except np.linalg.LinAlgError:
                        break
                    s, t = s - step[0], t - step[1]
                outside = max(0.0, -s, s - 1, -t, t - 1)
                if best is None or outside < best[0]:
                    best = (outside, j, i, min(max(s, 0.0), 1.0), min(max(t, 0.0), 1.0))
                if outside < 1e-6:
                    break
            out.append(best[1:])
        return out

    def carry(self, located, values):
        """Values per corner of this grid (rows*cols, k) interpolated at
        `located` places (from `locate`)."""
        v = np.asarray(values).reshape(self.rows, self.cols, -1)
        out = []
        for j, i, s, t in located:
            out.append((1 - s) * (1 - t) * v[j, i] + s * (1 - t) * v[j, i + 1]
                       + s * t * v[j + 1, i + 1] + (1 - s) * t * v[j + 1, i])
        return np.array(out)


def counts(lengths, spacing, least=1):
    return {k: max(least, int(round(v / spacing))) for k, v in lengths.items()}
