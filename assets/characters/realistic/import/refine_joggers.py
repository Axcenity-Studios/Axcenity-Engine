"""The intake's refinement of BlenderKit's Grey sweat pants (CC0) into the
first licensed joggers (wave 5, Paul's decision 13: "clean cuffs,
consistent geometry and proper leg deformation"). Blender's Python (bpy
5.x, NumPy, Pillow); run by `refine.py` between `topology` and `conform`.

Decision 11 gives refinement, topology and fitting to "Blender and manual
modelling"; agents can't model by hand, so this is that work as a script
under review, every number below in the open.

The download is a drop-crotch, baggy sweat pant made on a MakeHuman body:
its crotch 13 cm below the realistic body's, its cuffs pooled on the
floor round a 52 cm opening, 6,143 irregular triangles with no turned edge
and no drawcord. What is kept is what makes it this garment: its look
(the fleece, the waistband's elastic channels, the gathered cuffs, the
fly stitch, the seams and back patch pockets, all in its maps), its back
pockets and its proportions where they suit a jogger. What is made again:

1. **Topology (retopology help, measured):** quads in rings round the
   body: the pelvis as one tube from the waistband to the crotch, each leg
   as a tube from the crotch line to the hem, sharing the crotch seam.
   Rings are 2 to 3 cm apart on the thigh and shin and 1.5 cm through the
   knee (seven loops within 6 cm of the joint), meant to let the knee bend
   without collapse (not proven: the knee test checks only the front of
   the bent knee, and the joggers have not been rebuilt without the loops
   to compare); columns follow the arc length round each ring.
2. **Shape:** each ring is the convex hull of the realistic body (with the
   boxer briefs: the joggers go over them) at its height, round each leg
   below the crotch, plus an ease (`EASE`): 8 mm at the waistband, 13 to
   17 mm over the seat, 16 to 18 mm at the thigh and knee, 15 to 17 mm down
   the shin, and 5.5 mm at the cuff, gathered to the ankle over 3 cm. Below the hips the pelvis's rings turn into the two legs' outline,
   and down each leg a column comes in at most `DRAPE` of the height it
   drops: the cloth hangs from the seat and the thigh. The hull bridges
   hollows (the back of the knee, between the buttocks) as cloth does. The
   cuff's hem sits at `HEM` (8.5 cm), over the ankle bone and above a
   shoe's collar: shortened and tapered, not pooling.
3. **Construction:** the waistband and both cuffs turned inside with 3 mm
   thickness and a rounded fold (a facing three rings deep: no single-layer
   edge shows); the back patch pockets as closed pillows 2.1 mm thick (`POCKET`) at
   the download's place; a drawcord (two cords out of metal eyelets at the
   front of the waistband, with aglets): the download has none.
4. **Maps:** new texture coordinates (Blender's minimum-stretch unwrap of
   the new panels, packed at one scale) and the download's maps baked onto
   them: each texel's point on the new surface is followed, by its height
   and its direction round the body, to the same place on the download (a
   ray from the download's own section middle), and its colour, roughness
   and normal read there; the normal turned from the old texture's frame
   into the new one. Colour is taken out of the albedo (its mean brought
   to 0.8) so a tint gives the colour; the occlusion-roughness-metal map
   is rebuilt (the download's B channel is 1, metal).

    python -I refine.py --spec <asset>.intake.json --work <folder>
"""

import json
import math
import os

import numpy as np

# -- Landmarks on the realistic body at rest (m, the masculine frame; the
# engine's fit carries the item to the feminine frame and every body). --

WAIST_TOP = 1.035  # the waistband's top: 2.5 cm under the natural waist, over the boxers' band
BAND_BOTTOM = 0.995  # a 4 cm elastic band
SPLIT = 0.775  # the pelvis tube's last ring; the legs part below it
KNEE = 0.500  # `LowerLeg`'s head
CUFF_TOP = 0.135  # the cuff's seam: a 5 cm rib cuff
HEM = 0.085  # the cuff's hem, over the ankle bone (the ankle joint 6.8 cm)

PELVIS_RINGS = [1.035, 1.025, 1.015, 1.005, 0.995, 0.975, 0.955, 0.935, 0.910, 0.885, 0.860,
                0.835, 0.810, 0.790, 0.775]
# From the crotch down: 3 cm on the thigh, 1.5 cm through the knee, 2.5 to 3
# cm down the shin, then the gather and the cuff.
LEG_RINGS = [0.735, 0.710, 0.680, 0.650, 0.620, 0.590, 0.560, 0.545, 0.530, 0.515, 0.500, 0.485,
             0.470, 0.455, 0.440, 0.420, 0.395, 0.365, 0.335, 0.305, 0.275, 0.245, 0.220, 0.195,
             0.175, 0.155, 0.135, 0.118, 0.100, 0.085]

# The radial ease over the hull (m), by height: snug elastic at the waist
# and cuff, room at the seat, thigh and knee for the walk and the bends.
EASE = [(HEM, 0.0055), (CUFF_TOP, 0.0055), (0.150, 0.012), (0.165, 0.016), (0.20, 0.017),
        (0.35, 0.015), (0.50, 0.016), (0.65, 0.018), (0.74, 0.018), (0.80, 0.017), (0.86, 0.015),
        (0.92, 0.013), (0.975, 0.009), (BAND_BOTTOM, 0.008), (WAIST_TOP, 0.008)]

TURN = 0.003  # the turned edges' thickness
POCKET = 0.0021  # a pocket's thickness
CORD_RADIUS = 0.0024
AGLET_RADIUS = 0.0029
CORD_LENGTH = 0.17
COLUMNS_HALF = 16  # pelvis columns a side (the pelvis ring has 32)
CROTCH = 7  # crotch-line corners between the back and front middles
MAP = 2048
ALBEDO_MEAN = 0.80
FADE = 0.02  # the cross-fade at the crotch's ring, either way (m)

MATERIALS = ["Fleece", "Rib", "Cord", "Aglet"]
FLEECE, RIB, CORD, AGLET = range(4)


def ease(y):
    ys, es = zip(*EASE)
    return float(np.interp(y, ys, es))


# -- Sections and hulls ----------------------------------------------------

def convex_hull(points):
    """The convex hull of 2D `points`, counter-clockwise (Andrew's monotone
    chain)."""
    pts = sorted(set(map(tuple, np.round(np.asarray(points, float), 9))))
    if len(pts) < 3:
        return np.array(pts, float)

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return np.array(lower[:-1] + upper[:-1], float)


def ray_polygon(hull, centre, angle):
    """How far from `centre` (inside the convex `hull`) its edge is along
    the direction `angle` (radians, from +x towards +y)."""
    d = np.array([math.cos(angle), math.sin(angle)])
    a = hull
    b = np.roll(hull, -1, axis=0)
    e = b - a
    # centre + t d = a + s e: solve for t, s on every edge.
    den = d[0] * e[:, 1] - d[1] * e[:, 0]
    ok = np.abs(den) > 1e-12
    w = a - centre
    t = np.where(ok, (w[:, 0] * e[:, 1] - w[:, 1] * e[:, 0]) / np.where(ok, den, 1), np.inf)
    s = np.where(ok, (w[:, 0] * d[1] - w[:, 1] * d[0]) / np.where(ok, den, 1), -1)
    hit = ok & (t > 0) & (s >= -1e-9) & (s <= 1 + 1e-9)
    return float(t[hit].min()) if hit.any() else 0.0


def slice_points(P, tris, y):
    """Where the plane at height `y` cuts the triangles' edges."""
    a, b, c = P[tris[:, 0]], P[tris[:, 1]], P[tris[:, 2]]
    out = []
    for u, v in ((a, b), (b, c), (c, a)):
        du, dv = u[:, 1] - y, v[:, 1] - y
        m = (du * dv) < 0
        t = du[m] / (du[m] - dv[m])
        out.append(u[m] + (v[m] - u[m]) * t[:, None])
    return np.vstack(out)


class Sections:
    """A mesh's horizontal sections: the convex hull round the trunk
    (`side` 0: |x| under `reach`, which leaves the hanging hands out) or
    round one leg (`side` +1 or -1: that side of the middle), its middle,
    and how far its edge is each way. Cached by height (0.5 mm)."""

    def __init__(self, P, tris, reach=0.25):
        self.P, self.tris, self.reach = np.asarray(P, float), np.asarray(tris), reach
        self._hulls = {}

    def hull(self, y, side=0):
        key = (int(round(y * 2000)), side)
        if key not in self._hulls:
            pts = slice_points(self.P, self.tris, key[0] / 2000.0)
            pts = pts[np.abs(pts[:, 0]) < self.reach]
            if side:
                pts = pts[pts[:, 0] * side > 0]
            hull = convex_hull(pts[:, [0, 2]]) if len(pts) >= 3 else None
            centre = None
            if hull is not None and len(hull) >= 3:
                centre = (hull.min(axis=0) + hull.max(axis=0)) / 2
                if side == 0:
                    centre[0] = 0.0  # the body is symmetric
            self._hulls[key] = (hull, centre)
        return self._hulls[key]

    def centre(self, y, side=0):
        return self.hull(y, side)[1]

    def radius(self, y, side, angle):
        hull, centre = self.hull(y, side)
        return ray_polygon(hull, centre, angle)


# -- Heights between the made body and the download -------------------------

class HeightMap:
    """A piecewise-linear map between target heights and the download's,
    through matching landmarks (both lists rising)."""

    def __init__(self, target, source):
        self.t = np.asarray(target, float)
        self.s = np.asarray(source, float)

    def to_source(self, y):
        return np.interp(y, self.t, self.s) if np.ndim(y) else float(np.interp(y, self.t, self.s))

    def to_target(self, y):
        return np.interp(y, self.s, self.t) if np.ndim(y) else float(np.interp(y, self.s, self.t))


# -- Closed parts ------------------------------------------------------------

def _frame(direction):
    d = direction / np.linalg.norm(direction)
    a = np.array([1.0, 0, 0]) if abs(d[0]) < 0.9 else np.array([0, 1.0, 0])
    u = np.cross(d, a)
    u /= np.linalg.norm(u)
    return u, np.cross(d, u)


def tube(path, radius, sides=6, caps=True):
    """A closed tube along `path` (rings of `sides` corners; each end capped
    by a fan to a corner on the path). Returns positions and faces (quads
    and the caps' triangles), outward."""
    path = np.asarray(path, float)
    radius = np.broadcast_to(np.asarray(radius, float), (len(path),))
    P, faces = [], []
    prev_u = None
    for k, p in enumerate(path):
        tangent = path[min(k + 1, len(path) - 1)] - path[max(k - 1, 0)]
        u, v = _frame(tangent)
        if prev_u is not None:
            # Keep the rings from twisting: the previous ring's u, turned
            # square to the new tangent.
            t = tangent / np.linalg.norm(tangent)
            u = prev_u - t * prev_u.dot(t)
            u /= np.linalg.norm(u)
            v = np.cross(t, u)
        prev_u = u
        for s in range(sides):
            a = 2 * math.pi * s / sides
            P.append(p + radius[k] * (math.cos(a) * u + math.sin(a) * v))
    n = len(path)
    for k in range(n - 1):
        for s in range(sides):
            a, b = k * sides + s, k * sides + (s + 1) % sides
            faces.append([a, b, b + sides, a + sides])
    if caps:
        start = len(P)
        P.append(path[0])
        P.append(path[-1])
        for s in range(sides):
            faces.append([start, (s + 1) % sides, s])
            faces.append([start + 1, (n - 1) * sides + s, (n - 1) * sides + (s + 1) % sides])
    return np.array(P), faces


def torus(centre, axis, major, minor, around=8, across=4):
    """A closed ring (an eyelet) round `axis` through `centre`."""
    u, v = _frame(np.asarray(axis, float))
    axis = np.asarray(axis, float) / np.linalg.norm(axis)
    P, faces = [], []
    for i in range(around):
        a = 2 * math.pi * i / around
        radial = math.cos(a) * u + math.sin(a) * v
        for j in range(across):
            b = 2 * math.pi * j / across
            P.append(centre + radial * (major + minor * math.cos(b)) + axis * minor * math.sin(b))
    for i in range(around):
        for j in range(across):
            a = i * across + j
            b = ((i + 1) % around) * across + j
            c = ((i + 1) % around) * across + (j + 1) % across
            d = i * across + (j + 1) % across
            faces.append([a, b, c, d])
    return np.array(P), faces


def pillow(front, thickness, normal):
    """A closed pocket: the `front` grid (rows, cols, 3), a back layer
    `thickness` behind it (along -`normal`, one vector or one per corner)
    and a rim joining their edges. Returns positions (front first),
    faces and the rim's faces."""
    rows, cols = front.shape[:2]
    normal = np.broadcast_to(np.asarray(normal, float), front.shape)
    back = front - normal * thickness
    P = np.vstack([front.reshape(-1, 3), back.reshape(-1, 3)])
    nb = rows * cols

    def f(i, j):
        return i * cols + j

    faces = []
    for i in range(rows - 1):
        for j in range(cols - 1):
            faces.append([f(i, j), f(i, j + 1), f(i + 1, j + 1), f(i + 1, j)])
            faces.append([nb + f(i, j), nb + f(i + 1, j), nb + f(i + 1, j + 1), nb + f(i, j + 1)])
    loop = ([f(0, j) for j in range(cols)] + [f(i, cols - 1) for i in range(1, rows)]
            + [f(rows - 1, j) for j in range(cols - 2, -1, -1)] + [f(i, 0) for i in range(rows - 2, 0, -1)])
    rim = []
    for a, b in zip(loop, loop[1:] + loop[:1]):
        rim.append([b, a, nb + a, nb + b])
    faces += rim
    return P, faces, rim


def turned(rings, thickness, centre, fold_out):
    """A turned edge: `rings` (rings, cols, 3), the edge's ring first, each
    copied `thickness` in towards the axis through `centre(y)` (the
    facing), and the fold's ring between the edge and the facing's first
    ring, rounded `thickness / 2` past the edge (up for `fold_out` +1, down
    for -1). Returns the facing's rings and the fold's ring."""
    rings = np.asarray(rings, float)
    inner = np.empty_like(rings)
    for k in range(rings.shape[0]):
        for j in range(rings.shape[1]):
            p = rings[k, j]
            d = p - centre(p[1])
            d[1] = 0.0
            d /= max(np.linalg.norm(d), 1e-9)
            inner[k, j] = p - d * thickness
    fold = (rings[0] + inner[0]) / 2
    fold[:, 1] += fold_out * thickness / 2
    return inner, fold


# -- The garment ---------------------------------------------------------------

class Mesh:
    """Positions, faces with a material and a kind each, the corner each
    vertex shows the download's texture of (`sample`), and UV seams."""

    def __init__(self):
        self.P, self.sample, self.faces, self.material, self.kind, self.mode = [], [], [], [], [], []
        self.seams = set()

    def add(self, p, sample=None):
        self.P.append(np.asarray(p, float))
        self.sample.append(np.asarray(p if sample is None else sample, float))
        return len(self.P) - 1

    def face(self, f, material, kind, mode=0):
        self.faces.append(list(f))
        self.material.append(material)
        self.kind.append(kind)
        self.mode.append(mode)

    def seam(self, a, b):
        self.seams.add((min(a, b), max(a, b)))


def strip(mesh, a, b, material, kind, mode=0, closed=True):
    """Quads between two rings of corner indices."""
    n = len(a)
    for j in range(n if closed else n - 1):
        k = (j + 1) % n
        mesh.face([a[j], a[k], b[k], b[j]], material, kind, mode)


def section_curve(sections, y, side, start, count, samples=720, clamp=None):
    """`count` points round the section at `y` (`side` 0 the trunk, +-1 a
    leg), the hull plus the ease, starting at direction `start` (radians)
    and going clockwise in (x, z) (decreasing angle), at the arc-length
    fractions `count` gives (a list of fractions, or a number for even)."""
    centre = sections.centre(y, side)
    angles = start - np.linspace(0, 2 * math.pi, samples, endpoint=False)
    r = np.array([sections.radius(y, side, a) for a in angles]) + ease(y)
    pts = np.stack([centre[0] + r * np.cos(angles), centre[1] + r * np.sin(angles)], 1)
    if clamp is not None:
        pts[:, 0] = np.maximum(pts[:, 0] * side, clamp) * side
    closed = np.vstack([pts, pts[:1]])
    seg = np.linalg.norm(np.diff(closed, axis=0), axis=1)
    cum = np.concatenate([[0], np.cumsum(seg)])
    fractions = np.arange(count) / count if np.isscalar(count) else np.asarray(count)
    target = fractions * cum[-1]
    x = np.interp(target, cum, closed[:, 0])
    z = np.interp(target, cum, closed[:, 1])
    return np.stack([x, np.full_like(x, y), z], 1)


def fractions_round(points):
    closed = np.vstack([points, points[:1]])
    seg = np.linalg.norm(np.diff(closed, axis=0), axis=1)
    return np.concatenate([[0], np.cumsum(seg)[:-1]]) / seg.sum()


# Where the pelvis starts hugging the legs: above it a ring is the hull
# round the hips; from here down to `SPLIT` it turns into the two legs'
# outline (their hulls with the ease, the gap between them bridged
# straight), so the pelvis meets the legs without a ledge.
LEGS_FROM = 0.870
# The least a pelvis ring stands off the made body anywhere (m): over the
# boxer briefs' front, which the legs' outline would cut through.
PELVIS_CLEAR = 0.008
_pelvis_cache = {}


def _ray_out(hull, centre, angle):
    """How far along `angle` from `centre` a ray leaves the convex `hull`
    (centre may be outside it), or nan when it misses."""
    d = np.array([math.cos(angle), math.sin(angle)])
    a, b = hull, np.roll(hull, -1, axis=0)
    e = b - a
    den = d[0] * e[:, 1] - d[1] * e[:, 0]
    ok = np.abs(den) > 1e-12
    w = a - centre
    t = np.where(ok, (w[:, 0] * e[:, 1] - w[:, 1] * e[:, 0]) / np.where(ok, den, 1), -np.inf)
    s = np.where(ok, (w[:, 0] * d[1] - w[:, 1] * d[0]) / np.where(ok, den, 1), -1)
    hit = ok & (t > 0) & (s >= -1e-9) & (s <= 1 + 1e-9)
    return float(t[hit].max()) if hit.any() else float("nan")


def pelvis_radius(sections, y, angles=None):
    """The pelvis ring's radius about the trunk's middle at height `y`, on
    720 directions (from +x towards +z), or at `angles`: the hips' hull
    plus the ease, turning towards the legs' outline below `LEGS_FROM`,
    and never under `PELVIS_CLEAR` off the made body's own outline."""
    key = int(round(y * 2000))
    if key not in _pelvis_cache:
        yy = key / 2000.0
        grid = np.linspace(-math.pi, math.pi, 720, endpoint=False)
        hull, centre = sections.hull(yy, 0)
        r = np.array([ray_polygon(hull, centre, a) for a in grid]) + ease(yy)
        w = min(max((LEGS_FROM - yy) / (LEGS_FROM - SPLIT), 0.0), 1.0)
        w = w * w * (3 - 2 * w)
        if w > 0:
            legs = np.full(len(grid), np.nan)
            for side in (1, -1):
                leg, mid = sections.hull(yy, side)
                if leg is None:
                    continue
                # The leg's outline grown by the ease (about its own middle).
                grown = mid + (leg - mid) * (1 + ease(yy) / np.maximum(np.linalg.norm(leg - mid, axis=1), 1e-6))[:, None]
                out = np.array([_ray_out(grown, centre, a) for a in grid])
                legs = np.fmax(legs, out)
            # Between the legs (the front and back middles) the outline is
            # bridged straight: the gaps filled round the circle.
            ok = ~np.isnan(legs)
            if ok.sum() > 2:
                idx = np.arange(len(grid))
                legs = np.interp(idx, np.concatenate([idx[ok] - len(grid), idx[ok], idx[ok] + len(grid)]),
                                 np.tile(legs[ok], 3))
                r = (1 - w) * r + w * legs
        # The made body's own outline (not its hull) plus the clearance.
        pts = slice_points(sections.P, sections.tris, yy)
        pts = pts[np.abs(pts[:, 0]) < sections.reach]
        own = np.zeros(len(grid))
        if len(pts):
            a = np.arctan2(pts[:, 2] - centre[1], pts[:, 0] - centre[0])
            d = np.hypot(pts[:, 2] - centre[1], pts[:, 0] - centre[0])
            bins = ((a + math.pi) / (2 * math.pi) * len(grid)).astype(int) % len(grid)
            np.maximum.at(own, bins, d)
            for k in range(1, 3):  # a bin's neighbours too: no gaps
                own = np.maximum(own, np.maximum(np.roll(own, k), np.roll(own, -k)))
        r = np.maximum(r, own + PELVIS_CLEAR)
        # Smoothed round the ring (7.5 degrees either way), so the
        # underwear's front and the bridged gaps turn without a kink.
        k = 15
        r = np.convolve(np.concatenate([r[-k:], r, r[:k]]), np.ones(2 * k + 1) / (2 * k + 1), "valid")
        r = np.maximum(r, own + 0.75 * PELVIS_CLEAR)
        _pelvis_cache[key] = (grid, r, centre)
    grid, r, centre = _pelvis_cache[key]
    if angles is None:
        return grid, r, centre
    a = (np.asarray(angles) + math.pi) % (2 * math.pi) - math.pi
    return np.interp(a, grid, r, period=2 * math.pi), centre


def pelvis_angles(sections, y):
    """The pelvis's left-half columns (front middle to back middle round the
    outside), evenly spaced by arc length on the ring at `y`: angles about
    the trunk's middle."""
    a = np.linspace(math.pi / 2, -math.pi / 2, 721)
    r, centre = pelvis_radius(sections, y, a)
    pts = np.stack([r * np.cos(a), r * np.sin(a)], 1)
    cum = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(pts, axis=0), axis=1))])
    want = np.linspace(0, cum[-1], COLUMNS_HALF + 1)
    return np.interp(want, cum, a), centre


def pelvis_point(sections, y, angle):
    r, centre = pelvis_radius(sections, y, [angle])
    return np.array([centre[0] + r[0] * math.cos(angle), y, centre[1] + r[0] * math.sin(angle)])


# Below the crotch each leg's columns keep their directions about the leg's
# middle from the top ring for `SETTLE` m, then share the ring out evenly
# by length; and cloth hangs: going down, a column comes in towards the
# leg's middle by at most `DRAPE` of the height it drops (it falls from the
# seat and the thigh instead of following the skin under them), except in
# the cuff's gather.
SETTLE = 0.15
DRAPE = 0.35
GATHER_FROM = 0.170


def leg_rings(sections, top_points):
    """The left leg's rings under its top ring (`top_points`, from the
    inseam clockwise seen from above), at `LEG_RINGS`."""
    centre = sections.centre(LEG_RINGS[0], 1)
    rel = top_points[:, [0, 2]] - centre
    top_angles = np.unwrap(np.arctan2(rel[:, 1], rel[:, 0]))
    fr = fractions_round(top_points[:, [0, 2]])
    out = []
    prev = top_points
    prev_y = SPLIT
    for y in LEG_RINGS:
        mid = sections.centre(y, 1)
        even = section_curve(sections, y, 1, top_angles[0], fr, clamp=0.004)
        even_angles = np.unwrap(np.arctan2(even[:, 2] - mid[1], even[:, 0] - mid[0]))
        even_angles += np.round((top_angles[0] - even_angles[0]) / (2 * math.pi)) * 2 * math.pi
        t = min(1.0, (LEG_RINGS[0] - y) / SETTLE)
        angles = (1 - t) * top_angles + t * even_angles
        r = np.array([sections.radius(y, 1, a) for a in angles]) + ease(y)
        pts = np.stack([mid[0] + r * np.cos(angles), np.full(len(r), y), mid[1] + r * np.sin(angles)], 1)
        if y > GATHER_FROM:
            # Drape: no closer to the leg's middle than the column above,
            # less what it may come in over the drop.
            d = prev[:, [0, 2]] - mid
            least = np.linalg.norm(d, axis=1) - DRAPE * (prev_y - y)
            here = pts[:, [0, 2]] - mid
            have = np.linalg.norm(here, axis=1)
            grow = np.maximum(least, have) / np.maximum(have, 1e-9)
            pts[:, 0] = mid[0] + here[:, 0] * grow
            pts[:, 2] = mid[1] + here[:, 1] * grow
        pts[:, 0] = np.maximum(pts[:, 0], 0.004)
        out.append(pts)
        prev, prev_y = pts, y
    return out


def crotch_line(union, front, back):
    """The crotch seam's corners between the back middle and the front
    middle of the last pelvis ring, under the body (with the underwear) by
    1.2 cm along the middle plane, and above the first leg ring."""
    # The middle plane's section of the body: (z, y) points.
    P, tris = union
    a, b, c = P[tris[:, 0]], P[tris[:, 1]], P[tris[:, 2]]
    pts = []
    for u, v in ((a, b), (b, c), (c, a)):
        du, dv = u[:, 0], v[:, 0]
        m = (du * dv) < 0
        t = du[m] / (du[m] - dv[m])
        pts.append((u[m] + (v[m] - u[m]) * t[:, None])[:, [2, 1]])
    pts = np.vstack(pts)
    pts = pts[(pts[:, 1] < SPLIT + 0.03) & (pts[:, 1] > 0.6)]
    out = []
    for k in range(1, CROTCH + 1):
        t = k / (CROTCH + 1)
        z = back[2] + (front[2] - back[2]) * t
        near = pts[np.abs(pts[:, 0] - z) < 0.012]
        low = near[:, 1].min() if len(near) else SPLIT
        y = min(low - 0.018, SPLIT - 0.006)
        y = max(y, LEG_RINGS[0] + 0.008)
        out.append([0.0, y, z])
    out = np.array(out)
    # Smoothed, the ends held by the middles.
    ys = np.concatenate([[back[1]], out[:, 1], [front[1]]])
    for _ in range(6):
        ys[1:-1] = np.minimum(ys[1:-1], 0.25 * ys[:-2] + 0.5 * ys[1:-1] + 0.25 * ys[2:])
    out[:, 1] = ys[1:-1]
    return out


def build(union, pockets_at):
    """The joggers' mesh on the made body (`union`: the body with the
    boxer briefs, as (P, tris)). `pockets_at(sections, mesh)` gives each back
    pocket's front grid (rows, cols, 3) and the outward direction at each
    corner. Built for the left side (+x) and mirrored."""
    sections = Sections(*union)
    mesh = Mesh()
    angles, _ = pelvis_angles(sections, SPLIT)
    # The pelvis's columns for the whole ring: left half front to back,
    # then the right half back to front (mirrored angles).
    left = list(angles)  # pi/2 .. -pi/2
    right = [math.pi - a for a in angles[-2:0:-1]]  # the mirror, back to front, middles left out
    ring_angles = left + right
    n_pelvis = len(ring_angles)
    pelvis = []
    for y in PELVIS_RINGS:
        pelvis.append([mesh.add(pelvis_point(sections, y, a)) for a in ring_angles])
    for k in range(len(pelvis) - 1):
        top, bottom = PELVIS_RINGS[k], PELVIS_RINGS[k + 1]
        material = RIB if bottom >= BAND_BOTTOM - 1e-6 else FLEECE
        strip(mesh, pelvis[k], pelvis[k + 1], material, "outer", 0)
    split = pelvis[-1]
    front_mid, back_mid = split[0], split[COLUMNS_HALF]
    crotch = crotch_line(union, front=mesh.P[front_mid], back=mesh.P[back_mid])
    crotch_ids = [mesh.add(p) for p in crotch]  # back to front
    # The left leg's top ring, clockwise in (x, z) seen from above: the
    # inseam (the crotch line's middle) to the front middle, round the
    # outside to the back middle, and back to the inseam.
    mid = CROTCH // 2
    top = crotch_ids[mid:] + split[0:COLUMNS_HALF + 1] + crotch_ids[:mid]
    top_points = np.array([mesh.P[i] for i in top])
    left = [top]
    for p in leg_rings(sections, top_points):
        left.append([mesh.add(q) for q in p])
    # The right leg is the left's reflection in x: its top ring is the same
    # corners mirrored (the crotch line and the middles are their own
    # mirror; a pelvis column j is column n - j), the rest new corners.
    def mirror(i):
        if i in crotch_ids or i in (front_mid, back_mid):
            return i
        return split[(n_pelvis - split.index(i)) % n_pelvis]

    right = [[mirror(i) for i in top]]
    for ring in left[1:]:
        right.append([mesh.add(mesh.P[i] * [-1.0, 1.0, 1.0]) for i in ring])
    legs = {1: left, -1: right}
    heights = [SPLIT] + LEG_RINGS
    for side in (1, -1):
        rings = legs[side]
        for k in range(len(rings) - 1):
            material = RIB if heights[k] <= CUFF_TOP + 1e-6 else FLEECE
            strip(mesh, rings[k], rings[k + 1], material, "outer", side)
    # The crotch's transition relaxed: the pelvis's hull spans the thighs'
    # fronts and backs where the legs' hulls don't, which left a ledge.
    if RELAX_ZONE[1] > RELAX_ZONE[0]:
        relax(mesh, union, faces=len(mesh.faces))
    # Turned edges: the waistband's top and each cuff's hem.
    # Two rows deep: the facing ends over the boxer briefs' band, not on it.
    waist_rings = np.array([[mesh.P[i] for i in pelvis[k]] for k in range(3)])
    centre_t = lambda y: np.array([0.0, y, sections.centre(y, 0)[1]])  # noqa: E731
    inner, fold = turned(waist_rings, TURN, centre_t, +1)
    _facing(mesh, pelvis[:3], inner, fold, RIB, 0)
    for side in (1, -1):
        hem_rings = legs[side][::-1][:3]  # hem first, then up
        pts = np.array([[mesh.P[i] for i in r] for r in hem_rings])
        centre_l = lambda y, s=side: np.array([sections.centre(y, s)[0], y, sections.centre(y, s)[1]])  # noqa: E731
        inner, fold = turned(pts, TURN, centre_l, -1)
        _facing(mesh, hem_rings, inner, fold, RIB, side)
    # UV seams: the front and back middles, the outseams and inseams, and
    # round the turned edges (their facings are islands of their own).
    _garment_seams(mesh, pelvis, legs, n_pelvis, crotch_ids)
    # The back pockets and the drawcord.
    for front, normal in pockets_at(sections, mesh):
        _pocket(mesh, front, normal)
    _drawcord(mesh, sections)
    return mesh, sections, {"pelvis": pelvis, "legs": legs, "crotch": crotch_ids}


# The relaxed band round the crotch (m), and the least clearance kept over
# the body with the underwear while relaxing.
RELAX_ZONE = (0.0, 0.0)
RELAX_CLEAR = 0.012


def relax(mesh, union, faces, iterations=40, step=0.5):
    """Smooths the garment's corners in RELAX_ZONE (each pulled half way
    to its neighbours' middle, fading out towards the zone's ends) and keeps
    each at least RELAX_CLEAR outside the made body: a ledge where the
    pelvis's hull meets the legs' becomes a smooth turn, as cloth makes."""
    import bpy  # noqa: F401  (mathutils comes with the bpy module)
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    P0, tris = union
    tree = BVHTree.FromPolygons([tuple(p) for p in P0], [tuple(int(i) for i in t) for t in tris],
                                all_triangles=True)
    P = np.array(mesh.P)
    neighbours = [set() for _ in P]
    for f in mesh.faces[:faces]:
        for a, b in zip(f, f[1:] + f[:1]):
            neighbours[a].add(b)
            neighbours[b].add(a)
    lo, hi = RELAX_ZONE
    move = [i for i in range(len(P)) if lo < P[i, 1] < hi and neighbours[i]]
    weight = {i: min(1.0, (P[i, 1] - lo) / 0.04, (hi - P[i, 1]) / 0.04) for i in move}
    for _ in range(iterations):
        new = P.copy()
        for i in move:
            avg = P[list(neighbours[i])].mean(axis=0)
            new[i] = P[i] + step * weight[i] * (avg - P[i])
        for i in move:
            loc, normal, _, dist = tree.find_nearest(Vector(new[i]))
            if loc is None:
                continue
            away = new[i] - np.array(loc)
            side = float(np.dot(away, np.array(normal)))
            if side < RELAX_CLEAR:
                new[i] = np.array(loc) + np.array(normal) * RELAX_CLEAR
        P = new
    for i in move:
        mesh.P[i] = P[i]
        mesh.sample[i] = P[i].copy()


def _facing(mesh, outer_rings, inner, fold, material, mode):
    """Adds the fold's ring and the facing's rings for `outer_rings` (the
    edge's ring first) and their faces: edge -> fold -> facing, the facing
    showing the texture of the cloth it lies under."""
    fold_ids = [mesh.add(p, mesh.P[i]) for p, i in zip(fold, outer_rings[0])]
    inner_ids = [[mesh.add(p, mesh.P[i]) for p, i in zip(ring, outer)]
                 for ring, outer in zip(inner, outer_rings)]
    strip(mesh, outer_rings[0], fold_ids, material, "fold", mode)
    strip(mesh, fold_ids, inner_ids[0], material, "fold", mode)
    for a, b in zip(inner_ids, inner_ids[1:]):
        strip(mesh, a, b, material, "facing", mode)
    # The facing's island: cut at the edge ring.
    for a, b in zip(outer_rings[0], outer_rings[0][1:] + outer_rings[0][:1]):
        mesh.seam(a, b)
    # And one cut across, so it unrolls.
    mesh.seam(outer_rings[0][0], fold_ids[0])
    mesh.seam(fold_ids[0], inner_ids[0][0])
    for a, b in zip(inner_ids, inner_ids[1:]):
        mesh.seam(a[0], b[0])


def _garment_seams(mesh, pelvis, legs, n_pelvis, crotch_ids):
    """The panels' cuts: the front and back middles and the outseams on the
    pelvis; the inseam and outseam down each leg; the crotch seam between
    the legs. Four panels, as the download's pattern (front and back a
    side)."""
    half = COLUMNS_HALF
    lateral = half // 2
    for col in (0, half, lateral, n_pelvis - lateral):
        for a, b in zip(pelvis, pelvis[1:]):
            mesh.seam(a[col], b[col])
    for side in (1, -1):
        rings = legs[side]
        lateral_id = pelvis[-1][lateral] if side > 0 else pelvis[-1][n_pelvis - lateral]
        out = rings[0].index(lateral_id)
        for col in (0, out):
            for a, b in zip(rings, rings[1:]):
                mesh.seam(a[col], b[col])
    line = [pelvis[-1][half]] + list(crotch_ids) + [pelvis[-1][0]]
    for a, b in zip(line, line[1:]):
        mesh.seam(a, b)


def _pocket(mesh, front, normal):
    P, faces, rim = pillow(front, POCKET, normal)
    base = len(mesh.P)
    rows, cols = front.shape[:2]
    nb = rows * cols
    # The texture is read 10% in from the pocket's edge: the download's
    # pocket island ends in its atlas's dark padding.
    flat = front.reshape(-1, 3)
    inset = flat.mean(axis=0) + 0.9 * (flat - flat.mean(axis=0))
    for k, p in enumerate(P):
        mesh.add(p, inset[k % nb])
    rim_set = {tuple(f) for f in rim}
    for f in faces:
        kind = "pocket_rim" if tuple(f) in rim_set else ("pocket" if max(f) < nb else "pocket_back")
        mesh.face([base + i for i in f], FLEECE, kind, 0)
    # Islands: the front, and the back with its rim cut once across.
    loop = ([(0, j) for j in range(cols)] + [(i, cols - 1) for i in range(1, rows)]
            + [(rows - 1, j) for j in range(cols - 2, -1, -1)] + [(i, 0) for i in range(rows - 2, 0, -1)])
    ids = [base + i * cols + j for i, j in loop]
    for a, b in zip(ids, ids[1:] + ids[:1]):
        mesh.seam(a, b)
        mesh.seam(a + nb, b + nb)
    mesh.seam(ids[0], ids[0] + nb)


def _surface_at_x(sections, y, x):
    """The pelvis ring's point at height `y` on the front with this x."""
    lo, hi = 0.25 * math.pi, 0.5 * math.pi
    for _ in range(40):
        mid = (lo + hi) / 2
        if pelvis_point(sections, y, mid)[0] > x:
            lo = mid
        else:
            hi = mid
    a = (lo + hi) / 2
    return pelvis_point(sections, y, a), a


def _drawcord(mesh, sections):
    """Two cords out of eyelets 1.5 cm either side of the front middle,
    3 cm under the waistband's top, hanging 17 cm down the front with a
    little spread, each ending in an aglet."""
    y0 = WAIST_TOP - 0.018
    for side in (1, -1):
        path = []
        steps = 13
        for k in range(steps + 1):
            t = k / steps
            y = y0 - t * CORD_LENGTH
            x = side * (0.015 + 0.012 * t)
            p, a = _surface_at_x(sections, y, abs(x))
            p[0] = x
            out = np.array([math.cos(a), 0.0, math.sin(a)])
            lift = (CORD_RADIUS + 0.0012) if k else -0.002
            path.append(p + out * lift)
        path = np.array(path)
        P, faces = tube(path, CORD_RADIUS, sides=6)
        base = len(mesh.P)
        for p in P:
            mesh.add(p)
        for f in faces:
            # The caps (triangles) are hidden in the band and the aglet: no
            # texture space for them.
            if len(f) == 3:
                mesh.face([base + i for i in f], AGLET, "aglet", 0)
            else:
                mesh.face([base + i for i in f], CORD, "cord", 0)
        sides = 6
        n = len(path)
        for k in range(n - 1):
            mesh.seam(base + k * sides, base + (k + 1) * sides)
        for s in range(sides):
            mesh.seam(base + s, base + (s + 1) % sides)
            mesh.seam(base + (n - 1) * sides + s, base + (n - 1) * sides + (s + 1) % sides)
        # The aglet: the last 2 cm, a little thicker.
        direction = path[-1] - path[-2]
        direction /= np.linalg.norm(direction)
        aglet = np.array([path[-1] - direction * 0.02, path[-1] - direction * 0.01, path[-1] + direction * 0.002])
        P, faces = tube(aglet, AGLET_RADIUS, sides=6)
        base = len(mesh.P)
        for p in P:
            mesh.add(p)
        for f in faces:
            mesh.face([base + i for i in f], AGLET, "aglet", 0)
        # The eyelet: a metal ring round the cord's way out.
        p, a = _surface_at_x(sections, y0, abs(side * 0.015))
        p[0] = side * 0.015
        out = np.array([math.cos(a), 0.0, math.sin(a)])
        P, faces = torus(p + out * 0.0009, out, 0.0036, 0.0009)
        base = len(mesh.P)
        for q in P:
            mesh.add(q)
        for f in faces:
            mesh.face([base + i for i in f], AGLET, "aglet", 0)


def changes():
    """What the refinement changed, for the provenance record."""
    return [
        "retopologized as quads in rings round the body, with seven edge loops within 6 cm of the knee",
        f"refitted to the Axcenity realistic body (with its boxer briefs): shortened and tapered to the ankle, "
        f"the hem at {HEM * 100:.1f} cm with a {100 * (CUFF_TOP - HEM):.0f} cm cuff gathered round it, "
        "the drop crotch raised to the body's",
        "the waistband raised over the underwear's band and closed round the waist (8 mm over the body and underwear), "
        "turned inside with 3 mm thickness, "
        "the cuffs turned likewise",
        f"back patch pockets rebuilt as closed pockets {POCKET * 1000:.1f} mm thick at the download's place",
        "a drawcord added (two cords out of metal eyelets, with aglets)",
        "new texture coordinates and the download's maps baked onto them; colour taken out of the albedo for tints; "
        "the roughness map rebuilt as occlusion-roughness-metal",
    ]


# -- The download ----------------------------------------------------------------

def components(n, triangles):
    """Connected pieces: a label per vertex."""
    parent = np.arange(n)

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for t in triangles:
        ra, rb, rc = find(t[0]), find(t[1]), find(t[2])
        parent[rb] = ra
        parent[find(rc)] = ra
    return np.array([find(i) for i in range(n)])


class Download:
    """The download as data: its triangles and texture coordinates, its
    pieces (the four panels and the two back pockets), its sections, its
    crotch, waist and hem heights, and how heights on the made garment map
    to heights on it."""

    def __init__(self, mesh):
        self.P = mesh.P
        tc = mesh.triangle_corners()
        self.tris = mesh.corners[tc]
        self.tri_uv = mesh.uv[tc]  # (n, 3, 2)
        label = components(len(self.P), self.tris)
        sizes = {lab: int((label == lab).sum()) for lab in np.unique(label)}
        big = sorted(sizes, key=sizes.get)
        # The back pockets: the two smallest pieces (155 corners each).
        self.pocket_labels = big[:2]
        pocket_vertex = np.isin(label, self.pocket_labels)
        tri_pocket = pocket_vertex[self.tris].all(axis=1)
        self.panel_tris = np.nonzero(~tri_pocket)[0]
        self.pocket_tris = np.nonzero(tri_pocket)[0]
        self.pockets = [np.nonzero(label == lab)[0] for lab in self.pocket_labels]
        self.sections = Sections(self.P, self.tris[self.panel_tris], reach=0.4)
        self.split = self._split()
        # The made crotch's ring reads the download a little clear of its own
        # crotch either way: right at it, rays from the middles graze the
        # inside of the fork and smear its texture across the seat.
        self.pelvis_from = self.split + 0.025
        self.legs_to = self.split - 0.008
        self._edges()

    def _split(self):
        """Where the legs part: the highest height under 0.9 m with no
        panel within 6 mm of the middle."""
        for y in np.arange(0.9, 0.4, -0.002):
            pts = slice_points(self.P, self.tris[self.panel_tris], y)
            if len(pts) and not (np.abs(pts[:, 0]) < 0.006).any():
                return float(y + 0.006)
        raise SystemExit("the download's legs never part")

    def _edges(self):
        """The waist's top edge and each hem's, as heights by direction."""
        edges = {}
        for t in self.tris[self.panel_tris]:
            for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                key = (min(a, b), max(a, b))
                edges[key] = edges.get(key, 0) + 1
        open_v = np.unique([v for e, n in edges.items() if n == 1 for v in e])
        P = self.P[open_v]
        top = P[P[:, 1] > 0.85]
        c = self.sections.centre(0.93, 0)
        self.top_angles = np.arctan2(top[:, 2] - c[1], top[:, 0] - c[0])
        self.top_heights = top[:, 1]
        self.hems = {}
        for side in (1, -1):
            hem = P[(P[:, 1] < 0.08) & (P[:, 0] * side > 0)]
            c = self.sections.centre(0.06, side)
            self.hems[side] = (np.arctan2(hem[:, 2] - c[1], hem[:, 0] - c[0]), hem[:, 1])

    @staticmethod
    def _by_angle(angles, heights, at, pick):
        """The edge's height in direction `at` (the corners within 12
        degrees, `pick` of them)."""
        d = np.abs((angles[None, :] - np.asarray(at)[:, None] + np.pi) % (2 * np.pi) - np.pi)
        near = d < math.radians(12)
        out = np.where(near, heights[None, :], np.nan)
        with np.errstate(all="ignore"):
            v = pick(out, axis=1)
        return np.where(np.isnan(v), pick(heights), v)

    def pelvis_height(self, y, angle):
        """A made pelvis height in a direction, as the download's: the crotch,
        the band's foot and the top edge (1 mm in) matched."""
        top = self._by_angle(self.top_angles, self.top_heights, np.atleast_1d(angle), np.nanmin) - 0.002
        band = top - 0.05
        s = (np.atleast_1d(y) - SPLIT) / (WAIST_TOP - SPLIT)
        fb_t = (BAND_BOTTOM - SPLIT) / (WAIST_TOP - SPLIT)
        fb_s = (band - self.pelvis_from) / (top - self.pelvis_from)
        s_src = np.where(s <= fb_t, s / fb_t * fb_s, fb_s + (s - fb_t) / (1 - fb_t) * (1 - fb_s))
        return self.pelvis_from + s_src * (top - self.pelvis_from)

    def pelvis_height_back(self, y_s, angle):
        """The inverse of `pelvis_height`."""
        top = self._by_angle(self.top_angles, self.top_heights, np.atleast_1d(angle), np.nanmin) - 0.002
        band = top - 0.05
        fb_t = (BAND_BOTTOM - SPLIT) / (WAIST_TOP - SPLIT)
        fb_s = (band - self.pelvis_from) / (top - self.pelvis_from)
        s_src = (np.atleast_1d(y_s) - self.pelvis_from) / (top - self.pelvis_from)
        s = np.where(s_src <= fb_s, s_src / fb_s * fb_t, fb_t + (s_src - fb_s) / (1 - fb_s) * (1 - fb_t))
        return SPLIT + s * (WAIST_TOP - SPLIT)

    def leg_height(self, y, angle, side):
        """A made leg height as the download's: the hem, the cuff's seam (5
        cm up the download's gathered cuff) and the crotch matched."""
        angles, heights = self.hems[side]
        hem = self._by_angle(angles, heights, np.atleast_1d(angle), np.nanmax) + 0.002
        cuff = hem + 0.05
        y = np.atleast_1d(y)
        lo = np.where(y <= CUFF_TOP, hem + (y - HEM) / (CUFF_TOP - HEM) * (cuff - hem), np.nan)
        hi = cuff + (y - CUFF_TOP) / (SPLIT - CUFF_TOP) * (self.legs_to - cuff)
        return np.where(y <= CUFF_TOP, lo, hi)


def read_node(name, path=None):
    """A part of the realistic body's file (an underwear item) at rest:
    positions and triangles."""
    import bodyglb
    doc, binary = bodyglb.read_glb(path or bodyglb.DEFAULT)
    node = next(n for n in doc["nodes"] if n.get("name") == name and "mesh" in n)
    Ps, tris, offset = [], [], 0
    for prim in doc["meshes"][node["mesh"]]["primitives"]:
        P = bodyglb.accessor(doc, binary, prim["attributes"]["POSITION"])
        Ps.append(P)
        tris.append(bodyglb.accessor(doc, binary, prim["indices"]).reshape(-1, 3) + offset)
        offset += len(P)
    return np.vstack(Ps), np.vstack(tris).astype(np.int64)


def made_body():
    """The made body: the realistic masculine body with its boxer briefs
    (the joggers go over them), as one (positions, triangles)."""
    import bodyglb
    body = bodyglb.load()
    P, tris = read_node("Underwear_BoxerBriefs")
    return np.vstack([body.P, P]), np.vstack([body.tris, tris + len(body.P)])


def pockets_from(download):
    """Each download pocket's place on the made garment: a 7 x 6 grid over
    the same directions and (mapped) heights, laid on the panel as built
    (rays from the trunk's middle onto its faces) and standing `POCKET`
    plus 1.2 mm off it, so no panel corner or face comes through it."""
    def at(sections, mesh):
        import bpy  # noqa: F401  (mathutils comes with the bpy module)
        from mathutils import Vector
        from mathutils.bvhtree import BVHTree
        tris = [(f[0], f[k], f[k + 1]) for f in mesh.faces for k in range(1, len(f) - 1)]
        panel = BVHTree.FromPolygons([tuple(p) for p in mesh.P], tris, all_triangles=True)
        out = []
        for verts in download.pockets:
            P = download.P[verts]
            c = np.array([download.sections.centre(y, 0) for y in P[:, 1]])
            angle = np.arctan2(P[:, 2] - c[:, 1], P[:, 0] - c[:, 0])
            y = download.pelvis_height_back(P[:, 1], angle)
            a0, a1 = np.percentile(angle, [2, 98])
            y0, y1 = np.percentile(y, [2, 98])
            rows, cols = 7, 6
            front = np.zeros((rows, cols, 3))
            normal = np.zeros((rows, cols, 3))
            for i in range(rows):
                for j in range(cols):
                    yy = y0 + (y1 - y0) * i / (rows - 1)
                    aa = a0 + (a1 - a0) * j / (cols - 1)
                    mid = sections.centre(yy, 0)
                    ray = Vector((math.cos(aa), 0.0, math.sin(aa)))
                    loc, n, _, _ = panel.ray_cast(Vector((mid[0], yy, mid[1])), ray, 0.6)
                    n = np.array(n) if np.dot(n, ray) > 0 else -np.array(n)
                    front[i, j] = np.array(loc) + n * (POCKET + 0.0012)
                    normal[i, j] = n
            out.append((front, normal))
        return out
    return at


# -- In Blender: orientation and texture coordinates ---------------------------

def finish(mesh, sections):
    """The mesh made a Blender object: every piece's faces turned one way
    and outward, the panels, facings, pockets and cords unwrapped
    (minimum stretch) along the seams, at one scale, and packed. The aglets
    and eyelets (metal, no map) take no texture space. Returns the faces
    (as Blender wrote them) and their corners' texture coordinates."""
    import bpy
    import bmesh
    bpy.ops.wm.read_factory_settings(use_empty=True)
    P = np.array(mesh.P)
    me = bpy.data.meshes.new("Joggers")
    me.from_pydata(np.stack([P[:, 0], -P[:, 2], P[:, 1]], 1).tolist(), [], mesh.faces)
    me.polygons.foreach_set("material_index", np.array(mesh.material, np.int32))
    me.update()
    obj = bpy.data.objects.new("Joggers", me)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    for a, b in mesh.seams:
        e = bm.edges.get((bm.verts[a], bm.verts[b]))
        if e is not None:
            e.seam = True
    # One way round per piece, then outward: the garment's outer faces
    # away from the body's axis (the trunk's, a leg's).
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    label = components(len(P), [list(f)[:3] for f in mesh.faces])
    flips = 0
    for lab in np.unique(label):
        faces = [f for f in bm.faces if label[f.verts[0].index] == lab]
        votes = 0.0
        for f in faces:
            k = f.index
            if mesh.kind[k] not in ("outer", "pocket", "cord", "aglet"):
                continue
            c = np.array(f.calc_center_median())
            centre_y = c[2]
            side = mesh.mode[k] if mesh.kind[k] == "outer" else 0
            mid = sections.centre(min(max(centre_y, 0.07), 1.0), side)
            if mesh.kind[k] in ("cord", "aglet"):
                continue  # closed: recalc already made them outward
            radial = np.array([c[0] - mid[0], -(c[1]) - mid[1]])  # (x, z): Blender -y is z
            n = np.array(f.normal)
            votes += float(np.dot([n[0], -n[1]], radial))
        if votes < 0:
            bmesh.ops.reverse_faces(bm, faces=faces)
            flips += 1
    bm.to_mesh(me)
    bm.free()
    me.update()
    # Texture coordinates for the faces with maps.
    bpy.ops.object.mode_set(mode="EDIT")
    bm = bmesh.from_edit_mesh(me)
    for f in bm.faces:
        f.select = mesh.material[f.index] != AGLET
    bmesh.update_edit_mesh(me)
    bpy.ops.uv.unwrap(method="MINIMUM_STRETCH", margin=0.0)
    bpy.ops.uv.average_islands_scale()
    bpy.ops.uv.pack_islands(margin=0.004, rotate=True)
    bpy.ops.object.mode_set(mode="OBJECT")
    uv = np.zeros(len(me.loops) * 2)
    me.uv_layers.active.data.foreach_get("uv", uv)
    uv = uv.reshape(-1, 2)
    faces = [list(p.vertices) for p in me.polygons]
    corners = [list(p.loop_indices) for p in me.polygons]
    for k, p in enumerate(me.polygons):
        if mesh.material[k] == AGLET:
            uv[list(p.loop_indices)] = 0.0
    return faces, corners, uv, flips


# -- The maps ------------------------------------------------------------------

def _image(path):
    from PIL import Image
    with Image.open(path) as im:
        return np.asarray(im.convert("RGB"), np.float32) / 255.0


# A tangent-space normal leaning further than this from the surface's own
# (z, its cosine) faces into the cloth. The download's normal map has such
# texels (71,519 in its islands, z <= 0); baked as they were, they shade a
# texel as if lit from behind the cloth, a fault of the map. (They were not
# the dark notch on the seat with the hips flexed: that is the boxer briefs
# through the joggers, Review fixes 2.)
MIN_NORMAL_Z = 0.2


def outward(xyz):
    """Tangent-space normals (n, 3) with each one facing into the cloth
    (z under `MIN_NORMAL_Z`) read as flat: such a texel is a fault of the
    map, not a fold."""
    out = np.array(xyz, np.float32, copy=True)
    out[out[:, 2] < MIN_NORMAL_Z] = (0.0, 0.0, 1.0)
    return out


def _bilinear(img, uv):
    """`img` read at texture coordinates `uv` (Blender's: v up)."""
    h, w = img.shape[:2]
    x = np.clip(uv[:, 0] * w - 0.5, 0, w - 1.001)
    y = np.clip((1.0 - uv[:, 1]) * h - 0.5, 0, h - 1.001)
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    fx, fy = (x - x0)[:, None], (y - y0)[:, None]
    a = img[y0, x0] * (1 - fx) + img[y0, x0 + 1] * fx
    b = img[y0 + 1, x0] * (1 - fx) + img[y0 + 1, x0 + 1] * fx
    return a * (1 - fy) + b * fy


def _uv_frame(P3, UV, centre_xz):
    """For triangles (n, 3, 3) with texture coordinates (n, 3, 2): the
    texture directions (n, 2, 2) of going round the body (counter-clockwise
    seen from above, about the middle `centre_xz` (n, 2)) and of going up,
    on each triangle's plane (columns: round, up)."""
    e1, e2 = P3[:, 1] - P3[:, 0], P3[:, 2] - P3[:, 0]
    n = np.cross(e1, e2)
    n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)
    mid = P3.mean(axis=1)
    radial = np.stack([mid[:, 0] - centre_xz[:, 0], np.zeros(len(mid)), mid[:, 2] - centre_xz[:, 1]], 1)
    up = np.array([0.0, 1.0, 0.0])
    around = np.cross(up, radial)
    out = []
    for t in (around, np.broadcast_to(up, around.shape)):
        t = t - n * np.sum(t * n, axis=1, keepdims=True)
        t /= np.maximum(np.linalg.norm(t, axis=1, keepdims=True), 1e-12)
        # t = a e1 + b e2 (least squares on the plane)
        g11, g12, g22 = np.sum(e1 * e1, 1), np.sum(e1 * e2, 1), np.sum(e2 * e2, 1)
        r1, r2 = np.sum(t * e1, 1), np.sum(t * e2, 1)
        det = np.maximum(g11 * g22 - g12 * g12, 1e-20)
        a = (r1 * g22 - r2 * g12) / det
        b = (r2 * g11 - r1 * g12) / det
        d = a[:, None] * (UV[:, 1] - UV[:, 0]) + b[:, None] * (UV[:, 2] - UV[:, 0])
        out.append(d / np.maximum(np.linalg.norm(d, axis=1, keepdims=True), 1e-12))
    return np.stack(out, axis=2)


def _orthogonal(M):
    """The orthogonal part of 2x2 matrices (n, 2, 2) (polar decomposition),
    reflections kept."""
    U, _, Vt = np.linalg.svd(M)
    return U @ Vt


def _rasterize(tri_uv, size):
    """Every texel whose middle is in a triangle of `tri_uv` (n, 3, 2):
    (flat texel index, triangle, barycentric weights). Edges count, so
    neighbouring triangles leave no gap."""
    texels, tris, bary = [], [], []
    px = tri_uv * size
    px[..., 1] = size - px[..., 1]  # image rows go down
    for k, t in enumerate(px):
        x0, y0 = np.floor(t.min(axis=0)).astype(int)
        x1, y1 = np.ceil(t.max(axis=0)).astype(int)
        x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, size), min(y1, size)
        if x1 <= x0 or y1 <= y0:
            continue
        xs, ys = np.meshgrid(np.arange(x0, x1) + 0.5, np.arange(y0, y1) + 0.5)
        xs, ys = xs.ravel(), ys.ravel()
        (ax, ay), (bx, by), (cx, cy) = t
        den = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
        if abs(den) < 1e-12:
            continue
        w0 = ((by - cy) * (xs - cx) + (cx - bx) * (ys - cy)) / den
        w1 = ((cy - ay) * (xs - cx) + (ax - cx) * (ys - cy)) / den
        w2 = 1 - w0 - w1
        inside = (w0 >= -1e-6) & (w1 >= -1e-6) & (w2 >= -1e-6)
        if not inside.any():
            continue
        idx = (ys[inside] - 0.5).astype(int) * size + (xs[inside] - 0.5).astype(int)
        texels.append(idx)
        tris.append(np.full(inside.sum(), k))
        bary.append(np.stack([w0[inside], w1[inside], w2[inside]], 1))
    return np.concatenate(texels), np.concatenate(tris), np.concatenate(bary)


def _dilate(img, filled, passes=12):
    """Spreads the filled texels outward so mip levels don't bleed the
    background into the islands' edges."""
    img, filled = img.copy(), filled.copy()
    for _ in range(passes):
        acc = np.zeros_like(img)
        cnt = np.zeros(filled.shape, np.float32)
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            cnt += np.roll(np.roll(filled, dy, 0), dx, 1)
            acc += np.roll(np.roll(img * filled[..., None], dy, 0), dx, 1)
        grow = (~filled) & (cnt > 0)
        img[grow] = acc[grow] / cnt[grow][:, None]
        filled = filled | grow
    return img, filled


def bake(mesh, faces, corners, uv, download, sections, maps, out_folder):
    """The download's maps on the new texture coordinates (see the module's
    note, step 4). Writes `<out>/Joggers_{base_color,normal,orm}.jpg`;
    returns their names and what the bake measured."""
    import bpy  # noqa: F401  (mathutils comes with the bpy module)
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    from PIL import Image
    albedo = _image(maps["base_color"])
    normal_map = _image(maps["normal"])
    rough = _image(maps["orm"])[..., 1]
    S = np.array(mesh.sample)
    # Fan every face with a map into triangles: their corners' sample
    # points, texture coordinates, and the face they belong to.
    rows_s, rows_uv, rows_face = [], [], []
    for k, (f, cs) in enumerate(zip(faces, corners)):
        if mesh.material[k] == AGLET:
            continue
        for j in range(1, len(f) - 1):
            idx = [0, j, j + 1]
            rows_s.append(S[[f[i] for i in idx]])
            rows_uv.append(uv[[cs[i] for i in idx]])
            rows_face.append(k)
    tri_s, tri_uv, tri_face = np.array(rows_s), np.array(rows_uv), np.array(rows_face)
    texel, tri, bary = _rasterize(tri_uv, MAP)
    kind = np.array(mesh.kind)[tri_face][tri]
    mode = np.array(mesh.mode)[tri_face][tri]
    point = np.einsum("nk,nkd->nd", bary, tri_s[tri])
    # The middles by height (cached sections, 1 mm steps).
    grid = np.arange(0.05, 1.04, 0.001)

    def centres(sec, side):
        out = []
        for y in grid:
            c = sec.centre(y, side)
            out.append(c if c is not None else (out[-1] if out else np.zeros(2)))
        return np.array(out)

    made = {side: centres(sections, side) for side in (0, 1, -1)}
    src = {side: centres(download.sections, side) for side in (0, 1, -1)}

    def at(table, side, y):
        return np.stack([np.interp(y, grid, table[side][:, 0]), np.interp(y, grid, table[side][:, 1])], 1)

    n = len(texel)
    pocketish = np.isin(kind, ["pocket", "pocket_back", "pocket_rim"])
    cord = kind == "cord"
    verts = [tuple(p) for p in download.P]
    panels = BVHTree.FromPolygons(verts, [tuple(int(i) for i in t) for t in download.tris[download.panel_tris]],
                                  all_triangles=True)
    pockets = BVHTree.FromPolygons(verts, [tuple(int(i) for i in t) for t in download.tris[download.pocket_tris]],
                                   all_triangles=True)
    turn_kinds = np.isin(kind, ["outer", "pocket"])

    def lookup(rows, side):
        """The download's colour, roughness and (turned) normal for the
        texels `rows`, read about the trunk's middle (`side` 0) or a leg's:
        a ray from the download's own middle at the matching height, the
        way the texel faces. Returns them and how many rays missed."""
        m = len(rows)
        p = point[rows]
        mid = np.zeros((m, 2))
        smid = np.zeros((m, 2))
        y_s = np.zeros(m)
        for s_ in (0, 1, -1):
            k = side == s_
            mid[k] = at(made, s_, p[k, 1])
        angle = np.arctan2(p[:, 2] - mid[:, 1], p[:, 0] - mid[:, 0])
        k = side == 0
        y_s[k] = download.pelvis_height(p[k, 1], angle[k])
        for s_ in (1, -1):
            k = side == s_
            y_s[k] = download.leg_height(p[k, 1], angle[k], s_)
        for s_ in (0, 1, -1):
            k = side == s_
            smid[k] = at(src, s_, y_s[k])
        hit_tri = np.zeros(m, int)
        hit_at = np.zeros((m, 3))
        missed = 0
        cos, sin = np.cos(angle), np.sin(angle)
        for j, i in enumerate(rows):
            origin = Vector((smid[j, 0], y_s[j], smid[j, 1]))
            direction = Vector((cos[j], 0.0, sin[j]))
            tree, table = (pockets, download.pocket_tris) if pocketish[i] else (panels, download.panel_tris)
            loc, _, index, _ = tree.ray_cast(origin, direction, 0.6)
            if index is None:
                missed += 1
                loc, _, index, _ = tree.find_nearest(origin + direction * 0.12)
            hit_tri[j] = table[index]
            hit_at[j] = loc
        T = download.P[download.tris[hit_tri]]
        v0, v1, v2 = T[:, 1] - T[:, 0], T[:, 2] - T[:, 0], hit_at - T[:, 0]
        d00, d01, d11 = np.sum(v0 * v0, 1), np.sum(v0 * v1, 1), np.sum(v1 * v1, 1)
        d20, d21 = np.sum(v2 * v0, 1), np.sum(v2 * v1, 1)
        den = np.maximum(d00 * d11 - d01 * d01, 1e-20)
        b1 = (d11 * d20 - d01 * d21) / den
        b2 = (d00 * d21 - d01 * d20) / den
        w = np.stack([1 - b1 - b2, b1, b2], 1)
        old_uv = np.einsum("nk,nkd->nd", w, download.tri_uv[hit_tri])
        colour = _bilinear(albedo, old_uv).astype(np.float32)
        rgh = _bilinear(rough[..., None], old_uv)[:, 0].astype(np.float32)
        xyz = outward(_bilinear(normal_map, old_uv) * 2 - 1)
        nrm = np.tile(np.array([0.0, 0.0, 1.0], np.float32), (m, 1))
        # The normal turned from the download's texture frame into the new
        # one where the texel shows the cloth's outside (the facings, folds
        # and the pockets' backs and rims are left flat).
        sel = np.nonzero(turn_kinds[rows])[0]
        new_frame = _uv_frame(tri_s[tri[rows[sel]]], tri_uv[tri[rows[sel]]], mid[sel])
        old_frame = _uv_frame(download.P[download.tris[hit_tri[sel]]], download.tri_uv[hit_tri[sel]], smid[sel])
        Q = _orthogonal(new_frame @ np.linalg.pinv(old_frame))
        nrm[sel, :2] = np.einsum("nij,nj->ni", Q, xyz[sel, :2])
        nrm[sel, 2] = xyz[sel, 2]
        return colour, rgh, nrm, missed

    side = np.where(pocketish, 0, mode)
    rows = np.nonzero(~cord)[0]
    colour = np.zeros((n, 3), np.float32)
    rgh = np.full(n, 0.8, np.float32)
    nrm = np.tile(np.array([0.0, 0.0, 1.0], np.float32), (n, 1))
    colour[rows], rgh[rows], nrm[rows], missed = lookup(rows, side[rows])
    # Across the crotch's ring the pelvis's reading and the legs' are
    # cross-faded over `FADE` either way, so the download's folds don't end
    # in a line where one takes over from the other.
    y = point[:, 1]
    band = np.nonzero(~cord & ~pocketish & (np.abs(y - SPLIT) < FADE))[0]
    other = np.where(side[band] == 0, np.where(point[band, 0] >= 0, 1, -1), 0)
    c2, r2, n2, missed2 = lookup(band, other)
    leg_share = np.clip((SPLIT + FADE - y[band]) / (2 * FADE), 0, 1)
    mine = np.where(side[band] == 0, 1 - leg_share, leg_share)
    colour[band] = mine[:, None] * colour[band] + (1 - mine[:, None]) * c2
    rgh[band] = mine * rgh[band] + (1 - mine) * r2
    nrm[band] = mine[:, None] * nrm[band] + (1 - mine[:, None]) * n2
    missed += missed2
    good = ~cord
    # The cords: a twisted cord's ridges in the fleece's mean colour.
    c = np.nonzero(cord)[0]
    tx = np.stack([texel[c] % MAP, texel[c] // MAP], 1).astype(np.float32)
    ridge = np.sin(2 * np.pi * (tx[:, 0] + tx[:, 1]) / 9.0)
    nrm[c] = np.stack([0.35 * ridge, 0.35 * ridge, np.ones(len(c))], 1)
    garment_texels = good & ~cord
    mean = float(colour[garment_texels].mean())
    colour[c] = (mean * (0.93 + 0.07 * ridge))[:, None]
    rgh[c] = 0.7
    # Colour out: the mean brought to ALBEDO_MEAN, so a tint gives the colour.
    colour = np.clip(colour / mean * ALBEDO_MEAN, 0, 1)
    nrm /= np.maximum(np.linalg.norm(nrm, axis=1, keepdims=True), 1e-9)
    filled = np.zeros(MAP * MAP, bool)
    filled[texel] = True
    filled = filled.reshape(MAP, MAP)
    images = {}
    mean_rough = float(rgh[garment_texels].mean())
    for name, values, background in (
            ("base_color", colour, [ALBEDO_MEAN] * 3),
            ("normal", nrm * 0.5 + 0.5, [0.5, 0.5, 1.0]),
            ("orm", np.stack([np.ones(n), rgh, np.zeros(n)], 1), [1.0, mean_rough, 0.0])):
        img = np.zeros((MAP * MAP, 3), np.float32)
        img[texel] = values
        img = img.reshape(MAP, MAP, 3)
        img, grown = _dilate(img, filled)
        img[~grown] = background
        file = f"Joggers_{name}.jpg"
        Image.fromarray(np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)).save(
            os.path.join(out_folder, file), quality=93)
        images[name] = file
    report = {"texels": int(n), "atlas_used": round(float(filled.mean()), 4), "rays_missed": int(missed),
              "source_albedo_mean": round(mean, 4), "roughness_mean": round(mean_rough, 3),
              "heather_grey_tint": "#{0:02x}{0:02x}{0:02x}".format(int(round(255 * mean / ALBEDO_MEAN)))}
    return images, report


# -- The step ------------------------------------------------------------------

def measure(mesh, sections, parts):
    """What the refinement made, for its report: counts, the loops round
    the knee, the cuff and the waistband against the body."""
    sizes = np.array([len(f) for f in mesh.faces])
    P = np.array(mesh.P)
    left = parts["legs"][1]
    heights = [SPLIT] + LEG_RINGS

    def girth(ids):
        ring = P[ids][:, [0, 2]]
        return float(np.linalg.norm(np.diff(np.vstack([ring, ring[:1]]), axis=0), axis=1).sum())

    def hull_girth(y, side):
        hull = sections.hull(y, side)[0]
        return float(np.linalg.norm(np.diff(np.vstack([hull, hull[:1]]), axis=0), axis=1).sum())

    rings = {}
    for y, ids in zip(heights, left):
        if y in (HEM, CUFF_TOP, 0.155, 0.20, 0.30, KNEE, 0.62):
            rings[f"{y:.3f}"] = {"cloth_mm": round(girth(ids) * 1000), "body_hull_mm": round(hull_girth(y, 1) * 1000)}
    for y, ids in zip(PELVIS_RINGS, parts["pelvis"]):
        if y in (WAIST_TOP, BAND_BOTTOM, 0.875):
            rings[f"{y:.3f}"] = {"cloth_mm": round(girth(ids) * 1000), "body_hull_mm": round(hull_girth(y, 0) * 1000)}
    knee = [y for y in LEG_RINGS if abs(y - KNEE) < 0.06]
    return {
        "faces": int(len(sizes)),
        "quads": int((sizes == 4).sum()),
        "quad_share": round(float((sizes == 4).mean()), 4),
        "triangles": int((sizes - 2).sum()),
        "pelvis_columns": len(parts["pelvis"][0]),
        "leg_columns": len(left[0]),
        "rings": {"pelvis": len(PELVIS_RINGS), "leg": len(LEG_RINGS) + 1},
        "knee_loops_within_6_cm": len(knee),
        "knee_loop_spacing_mm": round(1000 * float(max(-np.diff(knee))), 1),
        "girths_left_leg_and_pelvis": rings,
        "ease_mm": {f"{y:.3f}": round(e * 1000, 1) for y, e in EASE},
        "hem_m": HEM, "cuff_top_m": CUFF_TOP, "waist_top_m": WAIST_TOP,
        "crotch_line_lowest_m": round(float(P[parts["crotch"]][:, 1].min()), 4),
    }


def step(spec, work, source="topology.npz", target="refined.npz"):
    """The refinement: reads the download as the topology step left it,
    writes the joggers on the realistic body (`refined.npz`), their maps
    (`textures/Joggers_*.jpg`) and the report (`refined.json`)."""
    from workmesh import WorkMesh
    loaded = WorkMesh.load(os.path.join(work, source))
    if loaded.materials[0].get("base_color") is None:
        raise SystemExit("the download's maps weren't loaded (load: \"uv\": \"textures\")")
    folder = os.path.join(work, "textures")
    maps = {k: os.path.join(folder, loaded.materials[0][k]) for k in ("base_color", "normal", "orm")}
    download = Download(loaded)
    union = made_body()
    mesh, sections, parts = build(union, pockets_from(download))
    faces, corners, uv, flips = finish(mesh, sections)
    images, baked = bake(mesh, faces, corners, uv, download, sections, maps, folder)
    flat_corners = [i for f in faces for i in f]
    flat_uv = uv[[c for cs in corners for c in cs]]
    materials = []
    for k, name in enumerate(MATERIALS):
        entry = {"name": name}
        if k != AGLET:
            entry.update({key: images[key] for key in ("base_color", "normal", "orm")})
        materials.append(entry)
    out = WorkMesh(np.array(mesh.P), flat_corners, [len(f) for f in faces], flat_uv,
                   np.array(mesh.material), materials)
    report = {
        "download": {"split_m": round(download.split, 4), "pockets": [int(len(p)) for p in download.pockets],
                     "panel_triangles": int(len(download.panel_tris))},
        "made": measure(mesh, sections, parts),
        "pieces_turned_outward": flips,
        "bake": baked,
        "changes": changes(),
    }
    out.save(os.path.join(work, target), report)
    return report
