"""The parts of FreeSewing's core the drafts here use: points, paths of
lines and cubic Bezier curves, and the few utilities the Teagan, Brian,
Titan and Paco designs call. A Python port of `@freesewing/core`'s
`Point`, `Path` and `utils`, written to keep the drafts in `teagan.py` and
`paco.py` line for line comparable with the JavaScript they follow.

FreeSewing's conventions, kept: millimetres; y grows **down** the page;
an angle is in degrees, counter-clockwise as seen on the page (so 90 is
up, toward smaller y).

Ported from FreeSewing (https://codeberg.org/freesewing/freesewing,
`packages/core/src/point.mjs`, `path.mjs`, `utils.mjs`, version 4.0.0).
Used under the MIT licence, whose notice is kept here as it requires:

    MIT License

    Copyright (c) 2022 Joost De Cock

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the
    "Software"), to deal in the Software without restriction, including
    without limitation the rights to use, copy, modify, merge, publish,
    distribute, sublicense, and/or sell copies of the Software, and to
    permit persons to whom the Software is furnished to do so, subject to
    the following conditions:

    The above copyright notice and this permission notice shall be included
    in all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
    OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
    MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
    IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
    CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
    TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
    SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

The changes (Axcenity Engine contributors): a subset, in Python; curves
are measured by sampling (FreeSewing uses Bezier.js), which agrees to a
small fraction of a millimetre at the sample count used.
"""

import math

# Samples per cubic when measuring or sampling a path.
CURVE_SAMPLES = 64


class Point:
    __slots__ = ("x", "y")

    def __init__(self, x, y):
        self.x = float(x)
        self.y = float(y)

    def __repr__(self):
        return f"Point({self.x:.2f}, {self.y:.2f})"

    def copy(self):
        return Point(self.x, self.y)

    clone = copy

    def dx(self, that):
        return that.x - self.x

    def dy(self, that):
        return that.y - self.y

    def dist(self, that):
        return math.hypot(self.dx(that), self.dy(that))

    def angle(self, that):
        """The angle from this point to `that`, 0 to 360, counter-clockwise
        on the page (y down)."""
        rad = math.atan2(-1 * self.dy(that), self.dx(that))
        while rad < 0:
            rad += 2 * math.pi
        return math.degrees(rad)

    def shift(self, angle, distance):
        rad = math.radians(angle)
        return Point(self.x + distance * math.cos(rad), self.y - distance * math.sin(rad))

    def shift_towards(self, that, distance):
        return self.shift(self.angle(that), distance)

    def shift_fraction_towards(self, that, fraction):
        return self.shift_towards(that, self.dist(that) * fraction)

    def shift_outwards(self, that, distance):
        return self.shift_towards(that, self.dist(that) + distance)

    def rotate(self, degrees, center):
        radius = center.dist(self)
        angle = center.angle(self)
        return center.shift(angle + degrees, radius)

    def flip_x(self, that=None):
        x = 0.0 if that is None else that.x
        return Point(2 * x - self.x, self.y)

    def flip_y(self, that=None):
        y = 0.0 if that is None else that.y
        return Point(self.x, 2 * y - self.y)

    def translate(self, x, y):
        return Point(self.x + x, self.y + y)


def beams_intersect(a1, a2, b1, b2):
    """Where the line through a1, a2 meets the line through b1, b2."""
    # Slope-intercept as FreeSewing writes it, with vertical lines handled.
    if a1.x == a2.x:
        if b1.x == b2.x:
            return None
        return beam_intersects_x(b1, b2, a1.x)
    if b1.x == b2.x:
        return beam_intersects_x(a1, a2, b1.x)
    sa = (a2.y - a1.y) / (a2.x - a1.x)
    sb = (b2.y - b1.y) / (b2.x - b1.x)
    if abs(sa - sb) < 1e-12:
        return None
    ia = a1.y - sa * a1.x
    ib = b1.y - sb * b1.x
    x = (ib - ia) / (sa - sb)
    return Point(x, sa * x + ia)


def beam_intersects_x(a, b, x):
    if a.x == b.x:
        return None
    t = (x - a.x) / (b.x - a.x)
    return Point(x, a.y + t * (b.y - a.y))


def beam_intersects_y(a, b, y):
    if a.y == b.y:
        return None
    t = (y - a.y) / (b.y - a.y)
    return Point(a.x + t * (b.x - a.x), y)


def _cubic(p0, c1, c2, p1, t):
    u = 1 - t
    return Point(u ** 3 * p0.x + 3 * u * u * t * c1.x + 3 * u * t * t * c2.x + t ** 3 * p1.x,
                 u ** 3 * p0.y + 3 * u * u * t * c1.y + 3 * u * t * t * c2.y + t ** 3 * p1.y)


class Path:
    """Operations: ("move", p), ("line", p), ("curve", cp1, cp2, p)."""

    def __init__(self, ops=None):
        self.ops = list(ops or [])

    def move(self, p):
        self.ops.append(("move", p.copy()))
        return self

    def line(self, p):
        self.ops.append(("line", p.copy()))
        return self

    def curve(self, cp1, cp2, p):
        self.ops.append(("curve", cp1.copy(), cp2.copy(), p.copy()))
        return self

    def curve_(self, cp1, p):
        """A curve whose second control point is its end."""
        return self.curve(cp1, p, p)

    def _curve(self, cp2, p):
        """A curve whose first control point is where it starts."""
        return self.curve(self.end(), cp2, p)

    def close(self):
        if self.ops and self.end().dist(self.start()) > 1e-9:
            self.line(self.start())
        return self

    def start(self):
        return self.ops[0][1]

    def end(self):
        return self.ops[-1][-1]

    def join(self, that):
        """This path, then `that` without its opening move."""
        ops = list(self.ops)
        ops.extend(op for op in that.ops if op[0] != "move")
        return Path(ops)

    def reverse(self):
        segments = self.segments()
        out = Path().move(self.end())
        for seg in reversed(segments):
            if seg[0] == "line":
                out.line(seg[1])
            else:
                _, p0, c1, c2, p1 = seg
                out.curve(c2, c1, p0)
        return out

    def segments(self):
        """("line", from, to) and ("curve", from, cp1, cp2, to)."""
        out = []
        at = None
        for op in self.ops:
            if op[0] == "move":
                at = op[1]
            elif op[0] == "line":
                out.append(("line", at, op[1]))
                at = op[1]
            else:
                out.append(("curve", at, op[1], op[2], op[3]))
                at = op[3]
        return out

    def sample(self, per_curve=CURVE_SAMPLES):
        """Points along the path (the first, then each segment's)."""
        segments = self.segments()
        pts = [segments[0][1]]
        for seg in segments:
            if seg[0] == "line":
                pts.append(seg[2])
            else:
                _, p0, c1, c2, p1 = seg
                for k in range(1, per_curve + 1):
                    pts.append(_cubic(p0, c1, c2, p1, k / per_curve))
        return pts

    def length(self):
        pts = self.sample()
        return sum(a.dist(b) for a, b in zip(pts, pts[1:]))

    def shift_along(self, distance):
        pts = self.sample(256)
        walked = 0.0
        for a, b in zip(pts, pts[1:]):
            step = a.dist(b)
            if walked + step >= distance and step > 0:
                return a.shift_fraction_towards(b, (distance - walked) / step)
            walked += step
        return pts[-1].copy()

    def edge(self, side):
        pts = self.sample()
        if side == "top":
            return min(pts, key=lambda p: p.y)
        if side == "bottom":
            return max(pts, key=lambda p: p.y)
        if side == "left":
            return min(pts, key=lambda p: p.x)
        return max(pts, key=lambda p: p.x)

    def intersects_y(self, y):
        out = []
        pts = self.sample(256)
        for a, b in zip(pts, pts[1:]):
            if (a.y - y) * (b.y - y) <= 0 and a.y != b.y:
                out.append(beam_intersects_y(a, b, y))
        return out

    def split(self, point):
        """The path cut at the sampled point nearest `point`, as two
        polylines (used only to find control points' replacements)."""
        pts = self.sample(256)
        k = min(range(len(pts)), key=lambda i: pts[i].dist(point))
        first = Path().move(pts[0])
        for p in pts[1:k + 1]:
            first.line(p)
        second = Path().move(pts[k])
        for p in pts[k + 1:]:
            second.line(p)
        return first, second


def polyline(path, spacing):
    """`path` resampled at about `spacing` mm along it, both ends kept."""
    pts = path.sample(256)
    total = sum(a.dist(b) for a, b in zip(pts, pts[1:]))
    n = max(1, int(round(total / spacing)))
    return [path.shift_along(total * k / n) for k in range(n + 1)]
