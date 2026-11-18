"""The joggers: Paco's pieces (`paco.py`) gridded, placed round the
realistic body's hips and legs, sewn and draped (`drape.py`).

Pieces: a front and a back for each leg. Seams: each leg's outseam and
inseam, the centre front (the two fronts' rises) and the centre back.
The waist edge and the hems are gathered as Paco gathers them, by elastic:
their corners start on a ring round the waist and round each ankle (the
elastic's length) and are held there through the drape (pinned), so the
cloth above them bunches into the gathers a jogger has. The waistband
casing and the cuffs are made after (`finish.py` bands).

Blender's axes: Z up, the body facing -Y. A piece's grid: columns from
the outseam (column 0) to the inseam and rise (the last column), rows
from the hem (row 0) to the waist (the top row).
"""

import math

import numpy as np

import paco
from panels import Panel, chain, path_points, polyline_length, resample

# How far the elastic rings stand off the skin (m): the waist's elastic
# lies on it; a cuff stands a little off the ankle.
WAIST_EASE = 0.004
CUFF_EASE = 0.012
# How far out of the skin the free cloth starts (m).
START_OUT = 0.02


def pieces(m, spacing, opts=paco.OPTIONS):
    d = paco.draft(m, opts)
    F = {k: path_points(v) for k, v in d["front"].items()}
    B = {k: path_points(v) for k, v in d["back"].items()}
    L = polyline_length
    n_in = max(4, round(L(F["inseam"]) / spacing))
    n_rise = max(3, round((L(F["rise"]) + L(B["rise"])) / 2 / spacing))
    rows = n_in + n_rise
    out = {}
    for side in ("left", "right"):
        for name, E in (("front", F), ("back", B)):
            if name == "front":
                bottom_src, left_src = E["hem"], E["outseam"][::-1]
                right_runs = [("inseam", resample(E["inseam"], n_in)), ("rise", resample(E["rise"], n_rise))]
                top_src = E["waist"][::-1]
            else:
                bottom_src, left_src = E["hem"][::-1], E["outseam"]
                right_runs = [("inseam", resample(E["inseam"][::-1], n_in)), ("rise", resample(E["rise"][::-1], n_rise))]
                top_src = E["waist"]
            cols = max(4, round((L(bottom_src) + L(top_src)) / 2 / spacing))
            right, marks = chain(right_runs)
            piece = f"{name}_{side}"
            segments = {
                "hem": [(0, c) for c in range(cols + 1)],
                "waist": [(rows, c) for c in range(cols + 1)],
                "outseam": [(r, 0) for r in range(rows + 1)],
                "inseam": [(r, cols) for r in range(marks["inseam"][0], marks["inseam"][1] + 1)],
                "rise": [(r, cols) for r in range(marks["rise"][0], marks["rise"][1] + 1)],
            }
            out[piece] = Panel(piece, (resample(bottom_src, cols), resample(top_src, cols),
                                       resample(left_src, rows), right), segments)
    seams = [
        ("front_left", "outseam", "back_left", "outseam", False),
        ("front_right", "outseam", "back_right", "outseam", False),
        ("front_left", "inseam", "back_left", "inseam", False),
        ("front_right", "inseam", "back_right", "inseam", False),
        ("front_left", "rise", "front_right", "rise", False),
        ("back_left", "rise", "back_right", "rise", False),
    ]
    return out, seams, d


def seam_pairs(panels, seams, offsets):
    pairs = []
    for pa, sa, pb, sb, rev in seams:
        a = panels[pa].segment(sa)
        b = panels[pb].segment(sb)
        if rev:
            b = b[::-1]
        assert len(a) == len(b), (pa, sa, pb, sb, len(a), len(b))
        pairs.extend((offsets[pa] + i, offsets[pb] + j) for i, j in zip(a, b))
    return pairs


def arc_fractions(points):
    d = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(points, axis=0), axis=1))])
    return d / d[-1]


class Placement:
    """Where each corner starts: the waist edge on the waist's elastic
    ring, the hem on the ankle's, the rest round the hips (above the
    crotch) and round each leg (below it), blended through the crotch."""

    def __init__(self, body, draft):
        self.body = body
        lm = body.landmarks
        self.waist_z = lm["waist_z"]
        self.crotch_z = lm["crotch_z"]
        f, b = draft["points"]["front"], draft["points"]["back"]
        top_y = (f["styleWaistOut"].y + b["styleWaistOut"].y) / 2 / 1000
        self.ring_z = self.waist_z - top_y
        self.hem_z = self.waist_z - (f["floorOut"].y + b["floorOut"].y) / 2 / 1000

    def ellipse(self, z):
        pts = self.body.section(z, self.body.arm < 0.2)
        pts = pts[np.abs(pts[:, 0]) < 0.3]
        a = np.abs(pts[:, 0]).max()
        y0, y1 = pts[:, 1].min(), pts[:, 1].max()
        return a, (y1 - y0) / 2, (y0 + y1) / 2

    def leg(self, z, s):
        pts = self.body.leg(z, s)
        c = pts.mean(axis=0)
        r = np.linalg.norm(pts[:, :2] - c[:2], axis=1).max()
        return np.array([c[0], c[1], z]), r

    def ring_point(self, phi, z, ease):
        """A point of the hips' ring: phi 0 at the front's middle, pi/2 the
        body's left side (+X), pi the back's middle; on the body's own
        convex outline there, `ease` out (an ellipse through its widest
        points cuts inside the hips at the back's sides)."""
        hull = self._hull(z)
        a, b, cy = self.ellipse(z)
        d = np.array([math.sin(phi), -math.cos(phi)])
        centre = np.array([0.0, cy])
        # The ray from the middle along `d` to the outline's edge.
        best = 0.0
        for p0, p1 in zip(hull, np.roll(hull, -1, axis=0)):
            e = p1 - p0
            m = np.array([[d[0], -e[0]], [d[1], -e[1]]])
            if abs(np.linalg.det(m)) < 1e-12:
                continue
            t, u = np.linalg.solve(m, p0 - centre)
            if t > 0 and -1e-9 <= u <= 1 + 1e-9:
                best = max(best, t)
        xy = centre + d * (best + ease)
        return np.array([xy[0], xy[1], z])

    def _hull(self, z):
        key = round(z, 4)
        cache = self.__dict__.setdefault("_hulls", {})
        if key not in cache:
            import body as bodymod
            pts = self.body.section(z, self.body.arm < 0.2)
            pts = pts[np.abs(pts[:, 0]) < 0.3]
            cache[key] = bodymod.hull_of(pts[:, :2])
        return cache[key]

    def leg_point(self, alpha, z, s, extra):
        """A point round a leg: alpha 0 at its outer side, pi/2 the front,
        pi the inner side, -pi/2 the back."""
        c, r = self.leg(z, s)
        r += extra
        return c + np.array([s * r * math.cos(alpha), -r * math.sin(alpha), 0.0])

    def row_arcs(self, panel):
        """Each corner's length along its row from the outseam (mm), and
        each row's whole width."""
        d = np.linalg.norm(np.diff(panel.uv, axis=1), axis=2)
        arcs = np.concatenate([np.zeros((panel.rows, 1)), np.cumsum(d, axis=1)], axis=1)
        return arcs, arcs[:, -1]

    def ellipse_at(self, a, b, cy, s_from_front, perimeter, side):
        """The point `s_from_front` (m) along an ellipse of `perimeter`
        (the body's section's shape, scaled), from the front's middle
        toward the body's `side` (+1 left)."""
        k = perimeter / (math.pi * (3 * (a + b) - math.sqrt((3 * a + b) * (a + 3 * b))))
        a, b = a * k, b * k
        th = np.linspace(0, math.pi, 721)
        xy = np.stack([a * np.sin(th), -b * np.cos(th)], axis=1)
        arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(xy, axis=0), axis=1))])
        t = np.interp(s_from_front, arc, th)
        return np.array([side * a * math.sin(t), cy - b * math.cos(t)])

    def place(self, panel, widths):
        """`widths[(piece, row)]`: each piece's row widths (mm), so a row
        laid round the hips or a leg is as long as it was cut (the cloth
        takes its starting lengths as its rest)."""
        name, side = panel.name.split("_")
        s = 1 if side == "left" else -1
        front = name == "front"
        rows, cols = panel.rows, panel.cols
        arcs, _ = self.row_arcs(panel)
        out = np.zeros((rows, cols, 3))
        pins = []
        waist_u = arc_fractions(panel.uv[-1])
        hem_u = arc_fractions(panel.uv[0])
        for r in range(rows):
            wf = widths[(f"front_{side}", r)] / 1000
            wb = widths[(f"back_{side}", r)] / 1000
            for c in range(cols):
                z = self.waist_z - panel.uv[r, c][1] / 1000
                along = arcs[r, c] / 1000          # from the outseam
                if r == rows - 1:
                    # On the waist's elastic: from the side to the middle.
                    q = waist_u[c]
                    phi = (math.pi / 2) * (1 - q) if front else math.pi / 2 + (math.pi / 2) * q
                    out[r, c] = self.ring_point(phi * s, self.ring_z, WAIST_EASE)
                    pins.append(panel.index(r, c))
                    continue
                if r == 0:
                    q = hem_u[c]
                    alpha = math.pi * q * (1 if front else -1)
                    out[r, c] = self.leg_point(alpha, self.hem_z, s, CUFF_EASE)
                    pins.append(panel.index(r, c))
                    continue
                hips = legs = None
                if z > self.crotch_z - 0.06:
                    a, b, cy = self.ellipse(min(max(z, self.crotch_z + 0.02), self.ring_z))
                    perimeter = 2 * (wf + wb)
                    # From the front's middle: a front corner is its row's
                    # width less its way from the outseam; a back corner is
                    # past the side seam.
                    from_front = (wf - along) if front else (wf + along)
                    xy = self.ellipse_at(a + START_OUT, b + START_OUT, cy, from_front, perimeter, s)
                    hips = np.array([xy[0], xy[1], z])
                if z < self.crotch_z + 0.08:
                    c0, _ = self.leg(min(max(z, self.hem_z), self.crotch_z - 0.01), s)
                    radius = (wf + wb) / (2 * math.pi)
                    # The inseam at the inner side (pi); the outseam where
                    # the front's width ends round from it.
                    if front:
                        alpha = math.pi - (wf - along) / radius
                    else:
                        alpha = -(math.pi - (wb - along) / radius)
                    legs = c0 + np.array([s * radius * math.cos(alpha), -radius * math.sin(alpha), 0.0])
                    legs[2] = z
                if hips is None:
                    out[r, c] = legs
                elif legs is None:
                    out[r, c] = hips
                else:
                    t = (z - (self.crotch_z - 0.06)) / 0.14
                    t = t * t * (3 - 2 * t)
                    out[r, c] = legs * (1 - t) + hips * t
        return out.reshape(-1, 3), pins
