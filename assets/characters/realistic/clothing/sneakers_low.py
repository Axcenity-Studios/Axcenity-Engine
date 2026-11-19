"""The low-top sneakers' geometry, built as a shoemaker builds a shoe over
its last (`sneakers_last.py`), at one level of detail. Blender's Python
(NumPy and mathutils); Blender's frame (metres, Z up, facing -Y).

Parts, each its own closed piece or a shell whose raw edges lie inside
another layer (asset-quality.md §3, "no single-layer edge"):

- **Sole** (`Outsole` the tread and its bevel, `Midsole` the wall and the
  lip, `Insole` the footbed): a cupsole as one closed solid. The tread is
  flat on the floor (`TREAD_Z`) and turns up at the toe and the heel in a
  bevel steeper than 25 degrees (a toe spring the footwear check doesn't
  count as tread); the wall stands `FLARE` outside the upper and rises to
  `sole_top(s)`, rounds over in a lip that meets the upper, and comes down
  inside the upper as an inner wall to the footbed under the foot. The
  upper's lower edge is lasted into the sole, between the two walls.
- **Upper** (`Upper`, leather): one shell from the featherline (inside the
  sole) to the opening, columns round the shoe and rows up it. Its
  columns meet the opening so that the vamp (over the toes) converges on
  the throat and the quarters run back to the heel; the vamp's outer
  columns from the ball to the throat's corners are the flex line across
  the toe joint. The opening's edge rolls over a padded collar (thicker at
  the heel and the sides) and comes back down inside as the lining
  (`Lining`).
- **Tongue** (`Tongue`): a padded slab under the eyestays from under the
  vamp to above the opening, thinner at its edges and its tucked end.
- **Eyelets** (`Eyelets`, LOD0 only): closed rings set into the eyestays.
- **Laces** (`Laces`): flat laces criss-crossed between the eyelets over
  the tongue, tied in a bow.

The panels a court sneaker's upper is sewn from (the vamp, the quarters,
the toe cap, the heel counter, the eyestay facings) are drawn on the one
shell by the maps: edges stepped 0.9 mm, stitched, with seams
(`sneakers_maps.py`), as the references show them.
"""

import math

import numpy as np

from sneakers_last import arc, resample_closed, smoothstep

# The sole (m).
TREAD_Z = -0.0013
FOOTBED_Z = -0.0003
FLARE = 0.0032
LIP = 0.0012
INNER = 0.0024      # the inner wall this far inside the upper
# The upper's distance off the foot (m): its base, over the toe box, and
# at the opening (the collar's padding and the eyestays over the tongue).
G_BASE = 0.0050
G_TOE = 0.0082
G_EYESTAY = 0.0078
G_THROAT = 0.0070
G_COLLAR = 0.0074
# How thick the opening's edge is, rolled over (the collar's padding).
T_COLLAR = 0.0050
T_HEEL = 0.0056
T_EYESTAY = 0.0026
# The tongue.
TONGUE_LEVEL = 0.0016
TONGUE_THICK = 0.0036
TONGUE_EDGE = 0.0016
TONGUE_HALF = 0.024
# The opening (foot-frame s, height m): the collar, lateral side; the
# medial side stands `MEDIAL_UP` higher (the inner ankle bone is higher).
COLLAR = ((0.00, 0.071), (0.05, 0.066), (0.10, 0.060), (0.16, 0.054), (0.22, 0.052), (0.27, 0.055))
MEDIAL_UP = 0.004
EYESTAY_FROM = 0.40
THROAT = 0.585
EYESTAY_W = 0.0125
EYELETS = 5
EYELET_IN = 0.0075   # from the opening's edge, along the shell
LACE_HALF_W = 0.0032
LACE_HALF_T = 0.0008


def sole_top(s):
    """The sole's top edge (m above the floor) along the foot."""
    s = np.asarray(s, dtype=float)
    return 0.030 + (0.024 - 0.030) * smoothstep(0.2, 0.6, s) + (0.0215 - 0.024) * smoothstep(0.6, 0.95, s)


class Mesh:
    """Corners, faces (quads or triangles) with a material each, the part
    each corner belongs to, and edges marked as texture seams."""

    def __init__(self):
        self.P = []
        self.part = []
        self.faces = []
        self.material = []
        self.seams = set()

    def add(self, p, part):
        self.P.append(np.asarray(p, dtype=float))
        self.part.append(part)
        return len(self.P) - 1

    def add_many(self, pts, part):
        return [self.add(p, part) for p in pts]

    def face(self, material, ids):
        self.faces.append(tuple(int(i) for i in ids))
        self.material.append(material)

    def seam(self, a, b):
        self.seams.add((min(a, b), max(a, b)))

    def seam_line(self, ids):
        for a, b in zip(ids, ids[1:]):
            self.seam(a, b)

    def strip(self, rows, material, closed, flip=False):
        """Quads between consecutive rows of corner ids (each row the same
        length; `closed`: each row wraps)."""
        for r0, r1 in zip(rows, rows[1:]):
            n = len(r0)
            for i in range(n if closed else n - 1):
                j = (i + 1) % n
                q = (r0[i], r0[j], r1[j], r1[i])
                self.face(material, q[::-1] if flip else q)

    def triangles(self):
        return sum(len(f) - 2 for f in self.faces)


# Detail at each level: sole rays, upper (quarter columns a side, vamp
# columns, rows), lining rows, tongue (along, across), eyelet segments
# (0: none), points along a lace segment, bow points.
LEVELS = (
    {"rays": 30, "quarter": 10, "vamp": 12, "rows": 6, "lining": 2, "tongue": (5, 6), "eyelet": 5,
     "lace": 3, "bow": 7, "lace_sides": 3, "bevel": True},
    {"rays": 24, "quarter": 7, "vamp": 8, "rows": 4, "lining": 1, "tongue": (4, 4), "eyelet": 0,
     "lace": 2, "bow": 5, "lace_sides": 3, "bevel": True},
    {"rays": 16, "quarter": 5, "vamp": 6, "rows": 3, "lining": 1, "tongue": (3, 3), "eyelet": 0,
     "lace": 1, "bow": 0, "lace_sides": 3, "bevel": False},
)


class Shoe:
    def __init__(self, foot, level):
        self.f = foot
        self.cfg = LEVELS[level]
        self.mesh = Mesh()
        self.report = {}
        self.build_sole()
        self.build_upper()
        self.build_tongue()
        if self.cfg["eyelet"]:
            self.build_eyelets()
        else:
            self.build_studs()
        self.build_laces()
        if level > 0:
            self.report["pushed_clear_mm"] = self.clear_faces()

    def clear_faces(self, gap=0.0018, rounds=6):
        """A simpler level's faces are flat across corners that lie on the
        curved last: where a face's middle comes nearer the foot than
        `gap`, its corners are moved out along the last's normal (the
        upper and the tongue; the sole stands off the foot by its own
        walls, and the full level is fine as built).
        Returns how far the most moved (mm)."""
        f, m = self.f, self.mesh
        P = np.array(m.P)
        part = np.array(m.part)
        movable = np.isin(part, ("upper", "tongue")) & (P[:, 2] > sole_top(f.s_of(P)) + 0.004)
        most = np.zeros(len(P))
        for _ in range(rounds):
            push = np.zeros(len(P))
            for face in m.faces:
                ids = list(face)
                if not movable[ids].all():
                    continue
                c = P[ids].mean(axis=0)
                d = f._sample(f.raw, c[None])[0]
                need = gap - d
                if need > 0:
                    push[ids] = np.maximum(push[ids], need)
            if not push.any():
                break
            sel = np.nonzero(push > 0)[0]
            P[sel] += f.normal(P[sel]) * push[sel, None]
            most += push
        m.P = [p for p in P]
        return round(float(most.max() * 1000), 2)

    # The sole.
    def ray_dirs(self, n):
        """`n` horizontal directions from the sole's middle, evenly spaced
        along the wall's outline."""
        f = self.f
        centre = f.mid(0.5, 0.0, 0.008)
        fine = f.outline(0.008, G_BASE + FLARE, 720)
        acc = arc(fine, closed=True)
        ts = np.linspace(0, acc[-1], n, endpoint=False)
        pts = np.array([np.interp(ts, acc[:-1], fine[:, k]) for k in range(3)]).T
        dirs = pts[:, :2] - centre[:2]
        dirs /= np.linalg.norm(dirs, axis=1, keepdims=True)
        return centre, np.column_stack([dirs, np.zeros(n)])

    def ring(self, centre, dirs, height, level):
        """Along each direction from `centre` at `height` (one per direction
        or a number), where the field reaches `level`."""
        height = np.broadcast_to(np.asarray(height, dtype=float), (len(dirs),))
        level = np.broadcast_to(np.asarray(level, dtype=float), (len(dirs),))
        out = []
        for d, h, lv in zip(dirs, height, level):
            o = centre.copy()
            o[2] = h
            out.append(self.f.march(o, d, lv, reach=0.25))
        return np.array(out)

    def build_sole(self):
        f, m, cfg = self.f, self.mesh, self.cfg
        n = cfg["rays"]
        centre, dirs = self.ray_dirs(n)
        # The wall stands outside the upper at every height it covers: along
        # each direction, the farthest of the upper's outlines from the
        # floor to the sole's top, `FLARE` out (the arch's inner side is
        # filled, as a cupsole's is).
        c2 = centre[:2]
        first = self.ring(centre, dirs, 0.008, G_BASE + FLARE)
        top = sole_top(f.s_of(first))
        reach = np.zeros(n)
        for z in (0.004, 0.008, 0.014):
            r = self.ring(centre, dirs, z, G_BASE + FLARE)
            reach = np.maximum(reach, np.linalg.norm(r[:, :2] - c2, axis=1))
        for frac in (0.75, 1.0):
            r = self.ring(centre, dirs, top * frac, self.g_at_sole(f.s_of(first)) + FLARE)
            reach = np.maximum(reach, np.linalg.norm(r[:, :2] - c2, axis=1))
        wall = np.column_stack([c2[None] + dirs[:, :2] * reach[:, None], np.full(n, 0.008)])
        s = f.s_of(wall)
        top = sole_top(s)

        def grown(ring, by):
            d = ring[:, :2] - c2
            d /= np.linalg.norm(d, axis=1, keepdims=True)
            out = ring.copy()
            out[:, :2] += d * by[:, None] if np.ndim(by) else d * by
            return out

        def at(ring, z):
            out = ring.copy()
            out[:, 2] = z
            return out

        # The bevel at the floor: wider at the toe (the toe spring) and the
        # heel, narrow at the sides.
        toe = smoothstep(0.86, 1.0, s)
        heel = 1 - smoothstep(0.0, 0.1, s)
        inset = 0.0022 + 0.0085 * toe + 0.0022 * heel
        rise = 0.0021 + 0.0042 * toe + 0.0006 * heel
        rings = []
        if cfg["bevel"]:
            rings.append(("Outsole", at(grown(wall, -inset), TREAD_Z)))
            rings.append(("Outsole", at(grown(wall, -0.0004), TREAD_Z + rise)))
            # The outsole's edge meets the wall in a groove 3.5 mm up.
            rings.append(("Midsole", at(grown(wall, -0.0002), TREAD_Z + rise + 0.0016)))
        else:
            rings.append(("Outsole", at(grown(wall, -inset * 0.6), TREAD_Z)))
            rings.append(("Midsole", at(wall, TREAD_Z + rise + 0.001)))
        rings.append(("Midsole", at(grown(wall, 0.0003), top - 0.0028)))
        rings.append(("Midsole", at(grown(wall, -LIP), top)))
        # The lip meets the upper: the upper's surface at the top, just out.
        lip_in = self.ring(centre, dirs, top - 0.0006, self.g_at_sole(s) + 0.0004)
        rings.append(("Midsole", lip_in))
        # Inside the upper: the inner wall down to the footbed.
        inner_top = self.ring(centre, dirs, top - 0.0016, self.g_at_sole(s) - INNER)
        # Straight down to the footbed (a ray along the floor from the
        # sole's middle can start under the arch, outside the foot).
        inner_low = inner_top.copy()
        rings.append(("Midsole", inner_top))
        rings.append(("Insole", at(inner_low, FOOTBED_Z)))
        ids = [m.add_many(r, "sole") for _, r in rings]
        for k in range(len(ids) - 1):
            mat = rings[k][0] if rings[k][0] == rings[k + 1][0] else rings[k + 1][0]
            if rings[k][0] == "Outsole":
                mat = "Outsole"
            # Past the lip, inside the upper: the inner wall (`SoleInner`,
            # drawn as the midsole; told apart for the clearance checks).
            if k >= len(ids) - 3:
                mat = "SoleInner"
            m.strip([ids[k], ids[k + 1]], mat, closed=True, flip=f.side < 0)
        # Texture seams: the wall cut at the heel and the toe; the caps, the
        # bevel, the outer wall, the lip and the inner wall each a band of
        # its own (each nearly developable, so it lies flat unstretched).
        m.seam_line([ids[k][0] for k in range(len(ids))])
        m.seam_line([ids[k][n // 2] for k in range(len(ids))])
        cuts = [0, len(ids) - 1, len(ids) - 3, len(ids) - 4]
        if cfg["bevel"]:
            cuts += [2]
        for k in cuts:
            r = ids[k]
            for i in range(n):
                m.seam(r[i], r[(i + 1) % n])
        self.cap(ids[0], "Outsole", down=True)
        self.cap(ids[-1], "Insole", down=False)
        self.sole_rings = rings
        self.report["sole"] = {"rays": n, "top_heel_mm": float(top.max() * 1000),
                               "top_toe_mm": float(top.min() * 1000)}

    def cap(self, ring, material, down):
        """A flat cap over a ring: quads laddered across the foot (each ring
        point paired with its mirror over the long axis), triangles at the
        ends."""
        m, f = self.mesh, self.f
        n = len(ring)
        # Ring 0 is the heel's middle and n/2 the toe's: pair i with n - i.
        quads = []
        for i in range(0, n // 2 - 1):
            a, b = ring[i], ring[i + 1]
            c, d = ring[(n - i - 1) % n], ring[(n - i) % n]
            if i == 0:
                quads.append((a, b, c))
            else:
                quads.append((a, b, c, d))
        half = n // 2
        quads.append((ring[half - 1], ring[half], ring[half + 1]))
        outward = f.side > 0
        for q in quads:
            # A cap facing down runs the other way to one facing up.
            q2 = q if (down ^ (not outward)) else q[::-1]
            m.face(material, q2)

    def g_at_sole(self, s):
        """The upper's distance off the foot where the sole meets it."""
        return G_BASE + (G_TOE - G_BASE) * smoothstep(0.7, 0.9, np.asarray(s))

    # The upper.
    def featherline(self, n):
        """The upper's lower edge, lasted 4 mm into the sole: n points from
        the heel's middle round the outer side."""
        f = self.f
        centre, dirs = self.ray_dirs(720)
        pts = self.ring(centre, dirs, 0.01, G_BASE)
        s = f.s_of(pts)
        z = sole_top(s) - 0.004
        pts = self.ring(centre, dirs, z, self.g_at_sole(s))
        return pts

    def opening(self):
        """The opening's edge as a dense closed loop from the heel's middle
        round the outer side, on the upper's surface, with each point's
        kind (0 collar, 1 eyestay, 2 throat)."""
        f = self.f

        def side_point(s, h, side, level):
            o = f.mid(max(s, 0.1), 0.0, h)
            if s < 0.08:
                # Round the heel: the direction turns from back to side.
                a = (s / 0.08) * math.pi / 2
                d2 = -f.d * math.cos(a) + f.l * side * math.sin(a)
            else:
                d2 = f.l * side
            return f.march(o, np.array([d2[0], d2[1], 0.0]), level)

        def top_point(s, w, level):
            return f.march(f.mid(s, w, 0.02), np.array([0.0, 0.0, 1.0]), level)

        def toward(s, w, h, level):
            """Onto the surface from a point `w` across the middle line."""
            return f.onto(f.mid(s, w, h), level)[0]

        pts, kinds = [], []
        lateral, medial = [], []
        for side, out in ((1, lateral), (-1, medial)):
            up = MEDIAL_UP if side < 0 else 0.0
            for s, h in COLLAR[1:]:
                out.append((side_point(s, h + up, side, G_COLLAR), 0))
            # The collar rises at the front of the ankle to the top eyelet.
            out.append((toward(0.33, side * 0.032, 0.064 + up, G_COLLAR), 0))
            out.append((toward(0.37, side * 0.022, 0.078 + up * 0.5, G_EYESTAY), 1))
            for s in np.linspace(EYESTAY_FROM, THROAT, 6):
                out.append((top_point(s, EYESTAY_W * side, G_EYESTAY), 1))
        heel = side_point(0.0, COLLAR[0][1], 1, G_COLLAR)
        seq = [(heel, 0)] + lateral
        for w in (0.006, 0.0, -0.006):
            seq.append((top_point(THROAT + 0.008, w, G_THROAT), 2))
        seq += medial[::-1]
        pts = np.array([p for p, _ in seq])
        kinds = np.array([k for _, k in seq])
        # A smooth closed curve through them (Catmull-Rom), dense.
        dense, dk = [], []
        n = len(pts)
        for i in range(n):
            p0, p1, p2, p3 = pts[(i - 1) % n], pts[i], pts[(i + 1) % n], pts[(i + 2) % n]
            for t in np.linspace(0, 1, 24, endpoint=False):
                t2, t3 = t * t, t * t * t
                q = 0.5 * ((2 * p1) + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2
                           + (-p0 + 3 * p1 - 3 * p2 + p3) * t3)
                dense.append(q)
                dk.append(kinds[i] if t < 0.5 else kinds[(i + 1) % n])
        return np.array(dense), np.array(dk)

    def build_upper(self):
        f, m, cfg = self.f, self.mesh, self.cfg
        nq, nv, V = cfg["quarter"], cfg["vamp"], cfg["rows"]
        U = 2 * nq + nv
        feather = self.featherline(720)
        top, kind = self.opening()
        fs, ts = f.s_of(feather), f.s_of(top)
        fw, tw = f.w_of(feather), f.w_of(top)
        # Key places: the heel (index 0 of both), the ball on each side,
        # the toe tip; on the opening the throat's corners and middle.
        ball = f.toe_joint if f.toe_joint else 0.66
        n_f = len(feather)
        lat = [i for i in range(n_f // 2) if fw[i] > 0]
        k_ball_l = min(lat, key=lambda i: abs(fs[i] - ball))
        k_toe = int(np.argmax(fs))
        med = [i for i in range(n_f // 2, n_f) if fw[i] < 0]
        k_ball_m = min(med, key=lambda i: abs(fs[i] - ball))
        n_t = len(top)
        throat = np.nonzero(kind == 2)[0]
        k_th_l, k_th_m = int(throat.min()), int(throat.max())
        k_th_c = int(throat[len(throat) // 2])
        acc_f = arc(feather, closed=True)
        acc_t = arc(top, closed=True)

        def spread(acc, a, b, count, closed_len):
            """`count` arc positions from index a to b (b may wrap)."""
            la, lb = acc[a], acc[b] if b > a else acc[b] + closed_len
            return np.linspace(la, lb, count, endpoint=False) % closed_len

        def place(points, acc, positions):
            L = acc[-1]
            closed = np.vstack([points, points[:1]])
            return np.array([[np.interp(p, acc, closed[:, k]) for k in range(3)] for p in positions])

        Lf, Lt = acc_f[-1], acc_t[-1]
        fpos = np.concatenate([spread(acc_f, 0, k_ball_l, nq, Lf), spread(acc_f, k_ball_l, k_ball_m, nv, Lf),
                               spread(acc_f, k_ball_m, 0, nq, Lf)])
        tpos = np.concatenate([spread(acc_t, 0, k_th_l, nq, Lt), spread(acc_t, k_th_l, k_th_m, nv, Lt),
                               spread(acc_t, k_th_m, 0, nq, Lt)])
        F = place(feather, acc_f, fpos)
        T = place(top, acc_t, tpos)
        tkind = np.array([kind[int(np.argmin(np.abs(acc_t[:-1] - p)))] for p in tpos])
        # Per column: the opening's distance off the foot and thickness.
        g_top = np.where(tkind == 0, G_COLLAR, np.where(tkind == 1, G_EYESTAY, G_THROAT))
        t_edge = np.where(tkind == 0, T_COLLAR, T_EYESTAY)
        s_top = f.s_of(T)
        t_edge = np.where((tkind == 0) & (s_top < 0.08), T_HEEL, t_edge)
        vamp = np.zeros(U, dtype=bool)
        vamp[nq:nq + nv] = True

        def level(i, v, p):
            s = f.s_of(p)
            base = self.g_at_sole(s) if v < 0.5 else G_BASE + (G_TOE - G_BASE) * smoothstep(0.7, 0.9, s)
            start = 0.62 if vamp[i] else 0.3
            return base + (g_top[i] - base) * smoothstep(start, 1.0, v)

        grid = np.zeros((U, V + 1, 3))
        levels = np.zeros((U, V + 1))
        for i in range(U):
            a, b = F[i], T[i]
            na = f.normal(a)[0]
            nb = f.normal(b)[0]
            nm = na + nb + np.array([0, 0, 0.6])
            nm /= np.linalg.norm(nm)
            chord = np.linalg.norm(b - a)
            vs = np.linspace(0, 1, 40)
            curve = np.array([a + (b - a) * v + nm * chord * 0.55 * 4 * v * (1 - v) for v in vs])
            lv = np.array([level(i, v, p) for v, p in zip(vs, curve)])
            curve = f.onto(curve, lv)
            curve[0], curve[-1] = a, b
            acc = arc(curve)
            # Rows evenly spaced along the column.
            pos = np.linspace(0, acc[-1], V + 1)
            col = np.array([[np.interp(p, acc, curve[:, k]) for k in range(3)] for p in pos])
            grid[i] = col
            levels[i] = [level(i, v, p) for v, p in zip(np.linspace(0, 1, V + 1), col)]
        # Smoothing round and up, then onto the surface again.
        for _ in range(8):
            inner = grid[:, 1:V].copy()
            mean = (np.roll(grid, 1, axis=0)[:, 1:V] + np.roll(grid, -1, axis=0)[:, 1:V]
                    + grid[:, 0:V - 1] + grid[:, 2:V + 1]) / 4
            inner = 0.5 * inner + 0.5 * mean
            flat = inner.reshape(-1, 3)
            flat = f.onto(flat, levels[:, 1:V].reshape(-1))
            grid[:, 1:V] = flat.reshape(U, V - 1, 3)
        # The opening's edge rolled over and the lining down inside.
        up = grid[:, V] - grid[:, V - 1]
        up /= np.linalg.norm(up, axis=1, keepdims=True)
        N = f.normal(grid[:, V])
        t = t_edge[:, None]
        roll1 = grid[:, V] + up * (0.42 * t) - N * (0.45 * t)
        roll2 = grid[:, V] - N * t + up * (0.08 * t)
        rows = [grid[:, j] for j in range(V + 1)] + [roll1, roll2]
        kinds = ["Upper"] * V + ["Lining", "Lining"]
        lining_rows = [V - 1, max(V - 3, 1)][:cfg["lining"]] if cfg["lining"] > 1 else [max(V - 2, 1)]
        for j in lining_rows:
            lv = np.maximum(levels[:, j] - t_edge, 0.0014)
            rows.append(f.onto(grid[:, j], lv))
            kinds.append("Lining")
        # The shell's rows and the roll are the upper; the rows down inside
        # are the lining (one piece with it).
        ids = [m.add_many(r, "upper" if k <= V + 2 else "lining") for k, r in enumerate(rows)]
        for k in range(len(ids) - 1):
            m.strip([ids[k], ids[k + 1]], "Upper" if k < V else "Lining", closed=True, flip=f.side < 0)
        # Seams: the heel's middle column, and the vamp's edges (the flex
        # line at the ball), each down the whole strip.
        for c in (0, nq, nq + nv):
            m.seam_line([ids[k][c] for k in range(len(ids))])
        # The vamp's middle (over the toe box's crown, a dome that doesn't
        # lie flat in one piece), and the opening's edge: the rolled collar
        # and lining a band of their own.
        m.seam_line([ids[k][nq + nv // 2] for k in range(V + 1)])
        for row in (V, V + 2):
            for i in range(U):
                m.seam(ids[row][i], ids[row][(i + 1) % U])
        # The collar's roll and lining are long curved bands: cut at the
        # quarters' middles too.
        for c in (nq // 2, U - nq // 2):
            m.seam_line([ids[k][c] for k in range(V, len(ids))])
        self.upper_grid = grid
        self.upper_ids = ids
        self.upper_top = grid[:, V]
        self.upper_up = up
        self.upper_kind = tkind
        self.upper_levels = levels
        self.U, self.V = U, V
        self.opening_dense = (top, kind)
        self.report["upper"] = {"columns": U, "rows": V, "featherline_mm": float(Lf * 1000),
                                "opening_mm": float(Lt * 1000)}

    # The tongue.
    def build_tongue(self):
        f, m, cfg = self.f, self.mesh, self.cfg
        na, nb = cfg["tongue"]
        # Its middle line: from under the vamp, up the instep, and on up
        # the front of the ankle past the opening.
        mids = []
        ss = np.linspace(THROAT + 0.035, 0.36, na + 1)
        for s in ss:
            mids.append(f.march(f.mid(s, 0.0, 0.02), np.array([0, 0, 1.0]), TONGUE_LEVEL))
        mids = np.array(mids)
        A = len(mids)
        across = np.linspace(-1, 1, nb + 1)
        bottom = np.zeros((A, nb + 1, 3))
        thick = np.zeros((A, nb + 1))
        for a in range(A):
            c = mids[a]
            n = f.normal(c)[0]
            lat = np.array([f.l[0], f.l[1], 0.0])
            lat -= n * (lat @ n)
            lat /= np.linalg.norm(lat)
            # Narrower where it is tucked under the vamp.
            half = TONGUE_HALF * (0.75 + 0.25 * min(1.0, a / 2))
            pts = np.array([c + lat * half * x + n * 0.01 for x in across])
            bottom[a] = f.onto(pts, TONGUE_LEVEL)
            prof = np.cos(across * math.pi / 2) ** 0.7
            tk = TONGUE_EDGE + (TONGUE_THICK - TONGUE_EDGE) * prof
            if a == 0:
                tk = np.minimum(tk, 0.0013)
            if a == A - 1:
                tk = tk * 0.8
            thick[a] = tk
        normals = f.normal(bottom.reshape(-1, 3)).reshape(A, nb + 1, 3)
        topg = bottom + normals * thick[:, :, None]
        bi = [m.add_many(bottom[a], "tongue") for a in range(A)]
        ti = [m.add_many(topg[a], "tongue") for a in range(A)]
        flip = f.side < 0
        for a in range(A - 1):
            for b in range(nb):
                q = (ti[a][b], ti[a][b + 1], ti[a + 1][b + 1], ti[a + 1][b])
                m.face("Tongue", q if not flip else q[::-1])
                q = (bi[a][b], bi[a + 1][b], bi[a + 1][b + 1], bi[a][b + 1])
                m.face("Tongue", q if not flip else q[::-1])
        # The rim between them, its edges texture seams (so the top, the
        # bottom and the rim are islands of their own; one closed piece).
        loop = [(a, 0) for a in range(A - 1)] + [(A - 1, b) for b in range(nb)] +                [(a, nb) for a in range(A - 1, 0, -1)] + [(0, b) for b in range(nb, 0, -1)]
        rb = [bi[a][b] for a, b in loop]
        rt = [ti[a][b] for a, b in loop]
        L = len(loop)
        for i in range(L):
            j = (i + 1) % L
            q = (rb[i], rb[j], rt[j], rt[i])
            m.face("Tongue", q[::-1] if not flip else q)
        for r in (rb, rt):
            for i in range(L):
                m.seam(r[i], r[(i + 1) % L])
        m.seam(rb[0], rt[0])
        self.tongue_top = topg
        self.tongue_mid = mids

    # Eyelets and laces.
    def eyelet_places(self):
        """Each side's eyelets, from the throat up: (centre on the shell,
        its normal)."""
        f = self.f
        top, kind = self.opening_dense
        out = {}
        for side in (1, -1):
            idx = [i for i in range(len(top)) if kind[i] == 1
                   and (f.w_of(top[i]) - f.mid_w(float(f.s_of(top[i])))) * side > 0]
            edge = top[idx]
            order = np.argsort(-f.s_of(edge))
            edge = edge[order]
            acc = arc(edge)
            # From 6 mm short of the throat's corner up, evenly.
            pos = np.linspace(acc[-1] * 0.06, acc[-1] * 0.86, EYELETS)
            pts = np.array([[np.interp(p, acc, edge[:, k]) for k in range(3)] for p in pos])
            places = []
            for p in pts:
                # In from the edge, down the upper's nearest column (its
                # rows, near the top, are the shell's surface).
                i = int(np.argmin(np.linalg.norm(self.upper_top - p, axis=1)))
                col = self.upper_grid[i][::-1]
                acc = arc(col)
                q = np.array([np.interp(EYELET_IN, acc, col[:, k]) for k in range(3)])
                q += p - col[0]
                q = f.onto(q, self.upper_levels[i, -1])[0]
                places.append((q, f.normal(q)[0]))
            out[side] = places
        return out

    def build_studs(self):
        """The simpler levels' eyelets: a low closed stud each (a square
        pyramid, its base inside the shell), so the level has its own
        eyelets rather than the renderer simplifying the full ones."""
        m = self.mesh
        for side, places in self.eyelet_places().items():
            for c, n in places:
                a = np.cross(n, [0.0, 0.0, 1.0])
                a /= np.linalg.norm(a)
                b = np.cross(n, a)
                base = [c - n * 0.0010 + 0.0024 * (math.cos(k * math.pi / 2) * a + math.sin(k * math.pi / 2) * b)
                        for k in range(4)]
                ids = m.add_many(base, "eyelet")
                apex = m.add(c + n * 0.0007, "eyelet")
                for k in range(4):
                    m.face("Eyelets", (ids[k], ids[(k + 1) % 4], apex))
                    m.seam(ids[k], apex)
                m.face("Eyelets", ids[::-1])

    def build_eyelets(self):
        m, seg = self.mesh, self.cfg["eyelet"]
        for side, places in self.eyelet_places().items():
            for c, n in places:
                a = np.cross(n, [0.0, 0.0, 1.0])
                if np.linalg.norm(a) < 1e-6:
                    a = np.cross(n, [1.0, 0.0, 0.0])
                a /= np.linalg.norm(a)
                b = np.cross(n, a)
                prof = ((0.0029, 0.0004), (0.0017, 0.0007), (0.0022, -0.0016))
                rings = []
                for r, h in prof:
                    ring = [c + n * h + r * (math.cos(2 * math.pi * k / seg) * a + math.sin(2 * math.pi * k / seg) * b)
                            for k in range(seg)]
                    rings.append(m.add_many(ring, "eyelet"))
                rings.append(rings[0])
                m.strip(rings, "Eyelets", closed=True, flip=(self.f.side < 0))
                # A ring is a torus: cut across its profile and round it.
                m.seam(rings[0][0], rings[1][0])
                m.seam(rings[1][0], rings[2][0])
                m.seam(rings[2][0], rings[0][0])
                # Each face of the profile a band of its own (each flat or a
                # cone, so it lies flat).
                for r in rings[:3]:
                    for k in range(seg):
                        m.seam(r[k], r[(k + 1) % seg])
        self.eyelets = self.eyelet_places()

    def lace_level(self, p):
        """How far off the foot a lace lies: over the tongue, or over the
        eyestay it comes up through (past the opening's edge)."""
        over_tongue = TONGUE_LEVEL + TONGUE_THICK + LACE_HALF_T + 0.0003
        if p is None:
            return over_tongue
        f = self.f
        s = float(f.s_of(p))
        w = abs(float(f.w_of(p)) - f.mid_w(s))
        t = float(smoothstep(EYESTAY_W - 0.003, EYESTAY_W + 0.0005, w))
        return over_tongue * (1 - t) + (G_EYESTAY + LACE_HALF_T + 0.0006) * t

    def lace(self, path, normals):
        """A flat lace along `path` (its surface's normals beside it): a
        closed tube of `lace_sides` sides, capped."""
        m = self.mesh
        sides = self.cfg["lace_sides"]
        rings = []
        for i, p in enumerate(path):
            t = path[min(i + 1, len(path) - 1)] - path[max(i - 1, 0)]
            t /= np.linalg.norm(t)
            n = normals[i] - t * (normals[i] @ t)
            n /= np.linalg.norm(n)
            w = np.cross(t, n)
            if sides == 3:
                prof = ((-LACE_HALF_W, -LACE_HALF_T), (LACE_HALF_W, -LACE_HALF_T), (0.0, LACE_HALF_T))
            else:
                prof = ((-LACE_HALF_W, -LACE_HALF_T), (LACE_HALF_W, -LACE_HALF_T),
                        (LACE_HALF_W, LACE_HALF_T), (-LACE_HALF_W, LACE_HALF_T))
            rings.append(m.add_many([p + w * x + n * y for x, y in prof], "lace"))
        flip = self.f.side < 0
        m.strip(rings, "Laces", closed=True, flip=not flip)
        k = len(rings[0])
        first, last = rings[0], rings[-1]
        m.face("Laces", first if not flip else first[::-1])
        m.face("Laces", last[::-1] if not flip else last)
        # Each side a strip of its own (a tube bends: its sides differ in
        # length), the caps apart.
        for j in range(k):
            m.seam_line([r[j] for r in rings])
        for r in (first, last):
            for i in range(k):
                m.seam(r[i], r[(i + 1) % k])

    def build_laces(self):
        f, cfg = self.f, self.cfg
        places = self.eyelet_places()
        lat, med = places[1], places[-1]
        n_pts = cfg["lace"] + 1

        def segment(a, b):
            (pa, na), (pb, nb) = a, b
            path, normals = [], []
            for t in np.linspace(0, 1, n_pts + 1):
                q = pa + (pb - pa) * t
                if 0 < t < 1:
                    q = f.onto(q + f.normal(q)[0] * 0.01, self.lace_level(q))[0]
                else:
                    q = q + (na if t == 0 else nb) * 0.0004
                path.append(q)
                normals.append(f.normal(q)[0])
            self.lace(np.array(path), np.array(normals))

        segment(lat[0], med[0])
        for k in range(EYELETS - 1):
            segment(lat[k], med[k + 1])
            segment(med[k], lat[k + 1])
        # The bow at the top: from each top eyelet to the knot, a loop each
        # way and two ends.
        if cfg["bow"]:
            knot = f.march(f.mid(EYESTAY_FROM + 0.012, 0.0, 0.02), np.array([0.0, 0.0, 1.0]),
                           self.lace_level(None) + 0.0015)
            nk = f.normal(knot)[0]
            for side, k in ((1, lat[-1]), (-1, med[-1])):
                pa, na = k
                path = [pa + na * 0.0004]
                for t in np.linspace(0.3, 0.9, max(1, cfg["lace"] - 1)):
                    q = pa + (knot - pa) * t
                    q = f.onto(q + f.normal(q)[0] * 0.01, self.lace_level(q) + 0.0008 * t)[0]
                    path.append(q)
                path.append(knot + np.array([f.l[0], f.l[1], 0.0]) * 0.0035 * side)
                self.lace(np.array(path), np.array([f.normal(q)[0] for q in path]))
            # The loops lie over the instep to each side, the ends hang
            # forward over the laces.
            nb = cfg["bow"]
            for side in (1, -1):
                lat2 = np.array([f.l[0], f.l[1], 0.0]) * side
                fwd = np.array([f.d[0], f.d[1], 0.0])
                path = []
                for k in range(nb):
                    a = math.pi * k / (nb - 1)
                    r = 0.016
                    q = knot + lat2 * (r * (0.5 - 0.5 * math.cos(a)) + 0.003) + fwd * (0.008 * math.sin(a) - 0.002) \
                        + nk * 0.003
                    q = f.onto(q, max(self.lace_level(q) + 0.0012, f.value(q)[0]))[0] if k not in (0, nb - 1) else q
                    path.append(q)
                self.lace(np.array(path), np.array([f.normal(q)[0] for q in path]))
                end = []
                for k in range(max(3, nb - 3)):
                    t = k / max(2, nb - 4)
                    q = knot + fwd * (0.004 + 0.024 * t) + lat2 * (0.003 + 0.004 * t) + nk * 0.0025
                    q = f.onto(q, self.lace_level(q) + 0.0009)[0]
                    end.append(q)
                self.lace(np.array(end), np.array([f.normal(q)[0] for q in end]))


def mirror(mesh):
    """The right shoe from the left: x mirrored, every face turned round
    (so its outside still faces out)."""
    out = Mesh()
    out.P = [np.array([-p[0], p[1], p[2]]) for p in mesh.P]
    out.part = list(mesh.part)
    out.faces = [tuple(f[::-1]) for f in mesh.faces]
    out.material = list(mesh.material)
    out.seams = set(mesh.seams)
    return out


def on_upper(shoe, i, v):
    """A point of the upper's shell at column `i` (fractional, wrapping)
    and `v` (0 the featherline, 1 the opening's edge), on its surface."""
    U, V = shoe.U, shoe.V
    i = i % U
    i0 = int(math.floor(i))
    i1 = (i0 + 1) % U
    fi = i - i0
    r = min(max(v, 0.0), 1.0) * V
    j0 = min(int(math.floor(r)), V - 1)
    fj = r - j0
    g, lv = shoe.upper_grid, shoe.upper_levels
    p = ((1 - fi) * ((1 - fj) * g[i0, j0] + fj * g[i0, j0 + 1]) + fi * ((1 - fj) * g[i1, j0] + fj * g[i1, j0 + 1]))
    level = ((1 - fi) * ((1 - fj) * lv[i0, j0] + fj * lv[i0, j0 + 1]) + fi * ((1 - fj) * lv[i1, j0] + fj * lv[i1, j0 + 1]))
    return shoe.f.onto(p, level)[0]


def features(shoe):
    """The lines the maps draw on the left shoe (metres): each overlay's
    edge with the side it covers (a unit vector along the shell at each
    point of the line), the topline, the tongue's bound edge, the sole's
    stitch row and the tread's outline. Built on the full-detail shoe."""
    nq, nv, U = shoe.cfg["quarter"], shoe.cfg["vamp"], shoe.U
    out = {"covers": {}}

    def curve(params, cover_dir):
        pts = np.array([on_upper(shoe, i, v) for i, v in params])
        dirs = []
        for k, (i, v) in enumerate(params):
            a = on_upper(shoe, i + cover_dir[0] * 0.3, v + cover_dir[1] * 0.03)
            d = a - pts[k]
            dirs.append(d / max(np.linalg.norm(d), 1e-9))
        return pts, np.array(dirs)

    t = np.linspace(0, 1, 60)
    # The toe cap: across the vamp, from the ball down at each side, over
    # the toe box 45% of the way up its middle; it covers the toe.
    out["toe_cap"] = curve([(nq + nv * x, 0.47 * math.sin(math.pi * x) ** 0.55) for x in t], (0, -1))
    # The heel counters: from the featherline 55% of the way to the ball up
    # and back to the collar; they cover the heel.
    out["counter_l"] = curve([(nq * (0.55 - 0.33 * x), x) for x in t], (-1, 0))
    out["counter_m"] = curve([(U - nq * (0.55 - 0.33 * x), x) for x in t], (1, 0))
    # The eyestay facings: 17 mm under the opening's edge from the collar's
    # front to the throat's corner, then down the vamp's edge to the sole;
    # they cover the opening's side and the quarter.
    for name, sgn in (("eyestay_l", 1), ("eyestay_m", -1)):
        top_cols = np.linspace(0.62, 0.985, 30)
        params = []
        for x in top_cols:
            i = nq * x if sgn > 0 else U - nq * x
            col_len = np.linalg.norm(np.diff(shoe.upper_grid[int(round(i)) % U], axis=0), axis=1).sum()
            params.append((i, 1 - 0.017 / col_len))
        # From the throat's corner the facing's edge runs down and back
        # along the side to the sole behind the ball (the vamp's columns
        # fan out from the throat, so following one would cross the toe
        # box's top).
        i_edge = nq - 0.15 if sgn > 0 else U - nq + 0.15
        i_low = nq * 0.8 if sgn > 0 else U - nq * 0.8
        v_from = params[-1][1]
        for t_ in np.linspace(0.0, 1.0, 30)[1:]:
            params.append((i_edge + (i_low - i_edge) * t_ ** 0.8, v_from * (1 - t_)))
        pts, dirs = curve(params, (0, 1))
        # Down the vamp's edge the facing covers the quarter's side.
        k0 = len(top_cols)
        side_pts, side_dirs = curve(params[k0:], (-sgn, 0))
        dirs[k0:] = side_dirs
        out[name] = (pts, dirs)
    for name in ("toe_cap", "counter_l", "counter_m", "eyestay_l", "eyestay_m"):
        pts, dirs = out[name]
        out[name] = pts
        out["covers"][name] = (pts, dirs)
    out["topline"] = np.vstack([shoe.upper_top, shoe.upper_top[:1]])
    top = shoe.tongue_top
    A, B = top.shape[0], top.shape[1]
    edge = [top[a, 0] for a in range(A)] + [top[A - 1, b] for b in range(B)] + \
           [top[a, B - 1] for a in range(A - 1, -1, -1)]
    out["tongue_edge"] = np.array(edge)
    # The sole's stitch row, 5 mm under the lip, on the wall.
    rings = shoe.sole_rings
    wall_top = [r for mat, r in rings][-5].copy()
    wall_top[:, 2] -= 0.0022
    out["sole_stitch"] = np.vstack([wall_top, wall_top[:1]])
    out["tread"] = rings[0][1][:, :2].copy()
    out["eyelets"] = [c for side in (1, -1) for c, _ in shoe.eyelet_places()[side]]
    return out
