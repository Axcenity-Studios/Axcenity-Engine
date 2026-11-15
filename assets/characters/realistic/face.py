"""The realistic head's face (Character Studio overhaul CO6): the repairs
to both frames' resting faces, the expression shapes of the Axcenity
Humanoid Facial Profile, and the face identity shapes.

`build.py` calls, in order:

    prepare()   both frames' positions with the face put right at rest
                (below), before the teeth are placed and the body weighted
    Face(...)   the landmarks; `.expressions()` and `.identity()` give the
                shapes as {name: {vertex: offset}} on the masculine frame

**At rest.** Three things are changed in the positions the body is built
from (the vertex order is the bundle's, untouched):

- The feminine frame's lids are the masculine frame's, carried to the
  feminine eye: as sculpted the feminine opening is the same size but
  sits 1.5 mm higher on the eyeball, which bares the whole iris (a
  stare). One set of lids on both frames is also what lets one blink
  shape close both exactly.
- The lids' lining is lifted off the eyeball where it dipped under it.
- The lips are closed (`mouth.Mouth.seal`).
"""

import numpy as np

import lids
import mouth
from facetopo import SIDES, Topo, nearest, smoothstep

# The textures the face's parts are built with (embedded in the model
# file; the build removes the loose files).
IMAGES = ("RealisticSclera.png", "RealisticEyeOcclusion.png", "RealisticLashes.png", "RealisticBrows.png")
# How much of the masculine lids each loop round the feminine eye takes
# (by loop from the rim; the lining takes all of it).
SHARE_LIDS = {5: 0.75, 6: 0.5, 7: 0.25}


def topo_of(mesh, sets):
    return Topo([tuple(p.vertices) for p in mesh.polygons], sets, len(mesh.vertices))


def prepare(topo, M, F, eyes_m, eyes_f, radius_m, radius_f):
    """Both frames' positions (arrays) with the face put right at rest,
    and a report. `eyes_*` are {1: left centre, -1: right centre}."""
    M, F = np.array(M, dtype=np.float64), np.array(F, dtype=np.float64)
    report = {}
    moved = 0.0
    for _, s in SIDES:
        eye = lids.Eye(topo, M, s, eyes_m[s], radius_m)
        offset = np.asarray(eyes_f[s], dtype=np.float64) - np.asarray(eyes_m[s], dtype=np.float64)
        for i in eye.idx:
            step = eye.step[i]
            share = 1.0 if step <= 4 else SHARE_LIDS.get(step, 0.0)
            if share:
                to = M[i] + offset
                moved = max(moved, float(np.linalg.norm(to - F[i])) * share)
                F[i] += (to - F[i]) * share
    report["feminine_lids_moved_mm"] = round(moved * 1000, 2)
    lifted = 0
    for P, centres, radius in ((M, eyes_m, radius_m), (F, eyes_f, radius_f)):
        for _, s in SIDES:
            eye = lids.Eye(topo, P, s, centres[s], radius)
            seated = eye.seat(P)
            lifted += int((np.linalg.norm(seated - P, axis=1) > 1e-7).sum())
            P[:] = seated
    report["lining_vertices_lifted"] = lifted
    rise = {}
    for name, P in (("masculine", M), ("feminine", F)):
        sealed, most = mouth.Mouth(topo, P).seal(P)
        P[:] = sealed
        rise[name] = round(most * 1000, 2)
    report["lips_closed_by_mm"] = rise
    return M, F, report


# -- the face's landmarks and the tools its shapes are made with --------------

JAW_OPEN_DEG = 25.0   # the face profile's jaw_open limit (crates/assets/src/face_profile.rs)
JAW_SIDE_DEG = 8.0    # and its jaw_sideways limit
# Lips, the mouth's lining and the tongue stay this far off the teeth.
OFF_TEETH = 0.0006
# A point within this of the teeth and behind their nearest face is inside them.
DEEPEST = 0.004
# How far from the lips' line the lips are kept off the teeth vertex by vertex.
NEAR_LIPS = 0.034


# The expression shapes, in the order the body carries them: the facial
# profile's control names (crates/core/src/face.rs). The four Eye_Look_*
# controls turn the eye bones and have no shape.
EXPRESSIONS = [
    "Blink_Left", "Blink_Right", "Eye_Closed_Left", "Eye_Closed_Right",
    "Eye_Squint_Left", "Eye_Squint_Right", "Eye_Wide_Left", "Eye_Wide_Right",
    "Brow_Raise_Left", "Brow_Raise_Right", "Brow_Lower_Left", "Brow_Lower_Right",
    "Brow_Inward_Left", "Brow_Inward_Right",
    "Mouth_Smile_Left", "Mouth_Smile_Right", "Mouth_Frown_Left", "Mouth_Frown_Right",
    "Mouth_Open", "Mouth_Wide", "Mouth_Narrow", "Mouth_Pucker", "Mouth_Left", "Mouth_Right",
    "Upper_Lip_Raise", "Lower_Lip_Lower", "Upper_Lip_Raise_Left", "Upper_Lip_Raise_Right",
    "Lower_Lip_Lower_Left", "Lower_Lip_Lower_Right", "Lip_Corner_Pull_Left", "Lip_Corner_Pull_Right",
    "Cheek_Raise_Left", "Cheek_Raise_Right", "Cheek_Puff", "Nose_Sneer_Left", "Nose_Sneer_Right",
    "Jaw_Open", "Jaw_Forward", "Jaw_Left", "Jaw_Right",
    "Tongue_Out", "Tongue_Up", "Tongue_Down",
]
# How far Jaw_Forward brings the jaw (metres).
JAW_FORWARD = 0.0055


def add(*shapes):
    """Shapes ({vertex: offset}) added together."""
    out = {}
    for shape in shapes:
        for i, d in shape.items():
            out[i] = out[i] + d if i in out else np.array(d, dtype=np.float64)
    return out


def scale(shape, by):
    return {i: d * by for i, d in shape.items()}


def gauss(distance, radius):
    return np.exp(-(np.asarray(distance) / radius) ** 2)


class Jaw:
    """The jaw bone as the engine turns it: about the body's left by
    `open_deg`, about its up by `side_deg` (to the character's left
    positive), at the hinge."""

    def __init__(self, hinge, weights):
        self.hinge = np.asarray(hinge, dtype=np.float64)
        self.w = np.asarray(weights, dtype=np.float64)

    @staticmethod
    def turn(open_deg=0.0, side_deg=0.0):
        a, b = np.radians(open_deg), np.radians(side_deg)
        # Blender axes: left is +X, up is +Z. Down is a positive turn about
        # left for a point in front (-Y) of the hinge.
        rx = np.array([[1, 0, 0], [0, np.cos(a), -np.sin(a)], [0, np.sin(a), np.cos(a)]])
        rz = np.array([[np.cos(b), -np.sin(b), 0], [np.sin(b), np.cos(b), 0], [0, 0, 1]])
        return rz @ rx

    def pose(self, P, open_deg=0.0, side_deg=0.0, idx=None):
        """Positions as the skin is posed with the jaw turned."""
        R = self.turn(open_deg, side_deg)
        w = (self.w if idx is None else self.w[idx])[:, None]
        return P + w * ((P - self.hinge) @ R.T + self.hinge - P)

    def rigid(self, X, open_deg=0.0, side_deg=0.0):
        return (np.asarray(X) - self.hinge) @ self.turn(open_deg, side_deg).T + self.hinge

    def unpose(self, offsets, open_deg=0.0, side_deg=0.0, idx=None):
        """Offsets wanted on the posed skin, as offsets of the resting
        skin (a shape is added before the skin is posed)."""
        R = self.turn(open_deg, side_deg)
        w = self.w if idx is None else self.w[idx]
        out = np.empty_like(offsets)
        eye = np.eye(3)
        for k in range(len(offsets)):
            out[k] = np.linalg.solve(eye + w[k] * (R - eye), offsets[k])
        return out


class Teeth:
    """The teeth and gums as two solids (the upper on the head, the lower
    on the jaw) the lips are kept off."""

    def __init__(self, points, polys, upper):
        from mathutils.bvhtree import BVHTree
        self.points = np.asarray(points, dtype=np.float64)
        upper = np.asarray(upper, dtype=bool)
        self.upper = upper
        self.polys = {True: [p for p in polys if upper[p[0]]], False: [p for p in polys if not upper[p[0]]]}
        self.used = np.unique(np.concatenate([np.array(p) for p in polys]))
        self._tree = BVHTree

    def posed(self, jaw=None, open_deg=0.0, side_deg=0.0, shift=None):
        """The points with the lower half moved by `shift` (the teeth's
        own Jaw_Forward shape) and turned with the jaw."""
        pts = self.points.copy()
        if shift is not None:
            pts[~self.upper] += shift
        if jaw is not None and (open_deg or side_deg):
            pts[~self.upper] = jaw.rigid(pts[~self.upper], open_deg, side_deg)
        return pts

    def trees(self, jaw=None, open_deg=0.0, side_deg=0.0, shift=None):
        pts = self.posed(jaw, open_deg, side_deg, shift)
        return [self._tree.FromPolygons([tuple(p) for p in pts], self.polys[half]) for half in (True, False)]


def _inside(tree, p):
    """Whether `p` is inside the closed surface in `tree`: rays from it
    cross the surface an odd number of times (three vote)."""
    from mathutils import Vector
    votes = 0
    for way in (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60))):
        way.normalize()
        at, crossings = Vector(p), 0
        for _ in range(64):
            hit = tree.ray_cast(at, way, 3.0)
            if hit[0] is None:
                break
            crossings += 1
            at = hit[0] + way * 1e-6
        votes += crossings % 2
    return votes >= 2


def polyline_nearest(points, line):
    """For each point: (distance to the polyline `line`, the nearest
    point on it, how far along it that is from 0 to 1)."""
    a, b = line[:-1], line[1:]
    seg = b - a
    length = np.linalg.norm(seg, axis=1)
    start = np.concatenate([[0.0], np.cumsum(length)])
    best_d = np.full(len(points), np.inf)
    best_p = np.zeros((len(points), 3))
    best_u = np.zeros(len(points))
    for k in range(len(seg)):
        t = np.clip(((points - a[k]) @ seg[k]) / max(length[k] ** 2, 1e-18), 0.0, 1.0)
        q = a[k] + t[:, None] * seg[k]
        d = np.linalg.norm(points - q, axis=1)
        closer = d < best_d
        best_d[closer], best_p[closer] = d[closer], q[closer]
        best_u[closer] = (start[k] + t[closer] * length[k]) / start[-1]
    return best_d, best_p, best_u


class Face:
    """The head's landmarks on the masculine frame `P` and the shapes
    made from them. `eyes` is {1: left centre, -1: right centre}, `jaw`
    a `Jaw`, `teeth` a `Teeth` (or None: lips aren't kept off them)."""

    def __init__(self, topo, P, eyes, eye_radius, jaw, teeth=None, other=None):
        self.topo, self.P, self.jaw, self.teeth = topo, np.asarray(P, dtype=np.float64), jaw, teeth
        # The other frame ({"P", "teeth"}), which the same shapes must
        # also keep off its teeth.
        self.other = other
        P = self.P
        self.N = topo.normals(P)
        self.head = topo.head
        self.eye = {s: lids.Eye(topo, P, s, eyes[s], eye_radius) for _, s in SIDES}
        self.mouth = mouth.Mouth(topo, P)
        sets = topo.vsets
        from facetopo import FS_EAR, FS_HEAD, FS_MOUTH_LOWER, FS_MOUTH_UPPER, FS_MUZZLE, FS_SOCKET
        n = topo.count
        flag = lambda ok: np.array([i in self.head_set and ok(sets[i]) for i in range(n)])  # noqa: E731
        self.head_set = set(self.head.tolist())
        self.lining = flag(lambda ss: ss <= {FS_MOUTH_UPPER, FS_MOUTH_LOWER})
        self.socket = flag(lambda ss: ss <= set(FS_SOCKET.values()))
        self.ear = flag(lambda ss: bool(ss & set(FS_EAR.values())))
        self.skin = flag(lambda ss: bool(ss & {FS_HEAD, FS_MUZZLE})) & ~self.ear
        self.upper_side = flag(lambda ss: ss <= {FS_HEAD, FS_MOUTH_UPPER})
        self.lower_side = flag(lambda ss: ss <= {FS_MUZZLE, FS_MOUTH_LOWER})
        inner = self.lining.copy()
        for k in (0, 1, 2):
            inner[self.mouth.lower[k]] = True
        self.inner = inner
        lid = np.zeros(n, dtype=bool)
        for eye in self.eye.values():
            lid[[i for i in eye.idx if eye.step[i] <= 4]] = True
        self.lid = lid
        outer = {k for k, fs in enumerate(topo.sets) if fs in (FS_HEAD, FS_MUZZLE)}
        self.skin_tree = topo.bvh(P, outer)
        # The mouth: its line, corners and middle; how far every vertex is
        # from the line and where along it.
        m = self.mouth
        self.line = P[m.edge]
        self.corner = {s: P[m.corner[s]].copy() for _, s in SIDES}
        self.stomion = P[m.mid].copy()
        self.half = m.half
        self.d_line, self.on_line, u = polyline_nearest(P, self.line)
        self.u = 2.0 * u - 1.0            # -1 at the right corner, 1 at the left
        self.lipness = gauss(self.d_line, 0.012)
        mid = self.skin & (np.abs(P[:, 0]) < 1e-4)
        front = np.where(mid)[0]
        self.nose_tip = P[front[np.argmin(P[front][:, 1])]].copy()
        chin = front[(P[front][:, 2] < self.stomion[2] - 0.02)]
        self.chin = P[chin[np.argmin(P[chin][:, 1])]].copy()
        self.crown = float(P[:, 2].max())
        self.ear_at = {s: P[topo.verts(FS_EAR[s])].mean(axis=0) for _, s in SIDES}

    # -- small tools -----------------------------------------------------

    def side(self, s, soft=0.008):
        """1 on side `s` of the face, 0 on the other, easing across the
        middle."""
        return smoothstep(-soft, soft, self.P[:, 0] * s)

    def near(self, point, radius):
        return gauss(np.linalg.norm(self.P - np.asarray(point), axis=1), radius)

    def on_skin(self, point):
        """The point of the resting skin nearest `point`."""
        hit = nearest(self.skin_tree, point, 0.1)
        return hit[0] if hit else np.asarray(point, dtype=np.float64)

    def slide(self, X, amount):
        """Positions `X` (every vertex) with the face's skin put back on
        the resting skin's surface, by `amount` per vertex (0 leaves a
        vertex where `X` has it): skin slides over what is under it."""
        out = X.copy()
        for i in np.where(self.skin & (amount > 1e-3) & (np.linalg.norm(X - self.P, axis=1) > 1e-6))[0]:
            hit = nearest(self.skin_tree, X[i], 0.03)
            if hit is not None:
                out[i] = X[i] + (hit[0] - X[i]) * amount[i]
        return out

    def off_teeth(self, X, open_deg=0.0, side_deg=0.0, teeth=None, jaw=None, rounds=3, shift=None, bag=False,
                  sweep=(1.0,)):
        """Posed positions `X` with the lips, the skin round them and the
        mouth's lining moved off the teeth (posed the same): no vertex of
        theirs inside the teeth, and no point of the teeth behind one of
        their faces. The teeth and gums are open sheets, not solids, so
        "inside" is only said of a point within `DEEPEST` of them and
        behind the nearest face. A point of the teeth counts as behind a
        face only when it is also inside the body's closed surface (a
        tooth is always "behind" the cheek's outer skin).

        With `bag`, the deep of the mouth's bag is widened too where a
        jaw that moves sideways or forward carries its teeth through it:
        a point of the teeth that is inside the body's flesh and behind a
        face of the bag pushes that face out. `sweep` brings the teeth to
        where they end in steps, so no step is deeper than the tests see."""
        from mathutils import Vector
        from mathutils.bvhtree import BVHTree
        teeth = teeth or self.teeth
        if teeth is None:
            return X
        jaw = jaw or self.jaw
        out = X.copy()
        # The lips and what is near them.
        near = (self.skin | self.lining) & (self.d_line < NEAR_LIPS)
        who = np.where(near)[0]
        # Faces that look at the teeth: the lining and the lips' inner sides.
        inner = self.inner
        tris = [tuple(int(i) for i in t) for t in self.topo.tris
                if inner[t[0]] and inner[t[1]] and inner[t[2]] and (near[t[0]] or near[t[1]] or near[t[2]])]
        deep = [tuple(int(i) for i in t) for t in self.topo.tris
                if self.lining[t[0]] and self.lining[t[1]] and self.lining[t[2]]
                and not (near[t[0]] or near[t[1]] or near[t[2]])] if bag else []
        whole = [tuple(int(i) for i in t) for t in self.topo.tris]
        for part in sweep:
            turn = side_deg * part
            moved = None if shift is None else shift * part
            points = teeth.posed(jaw, open_deg, turn, moved)[teeth.used]
            for _ in range(rounds):
                for tree in teeth.trees(jaw, open_deg, turn, moved):
                    for i in who:
                        hit = nearest(tree, out[i], DEEPEST)
                        if hit is not None and hit[2] < OFF_TEETH:
                            out[i] = hit[0] + hit[1] * OFF_TEETH
                # The other way round: a point of the teeth behind a face.
                push = {}

                def behind(faces, reach, margin, confirm=None):
                    flesh = BVHTree.FromPolygons([tuple(q) for q in out], faces, all_triangles=True)
                    for q in points:
                        loc, normal, index, _ = flesh.find_nearest(Vector(q), reach)
                        if loc is None:
                            continue
                        depth = -(Vector(q) - loc).dot(normal)
                        if depth > 0.0002 and (confirm is None or confirm(q)):
                            for i in faces[index]:
                                if depth + margin > push.get(i, (0.0,))[0]:
                                    push[i] = (depth + margin, np.array(normal))

                body = BVHTree.FromPolygons([tuple(q) for q in out], whole, all_triangles=True)
                in_flesh = lambda q: _inside(body, q)  # noqa: E731
                behind(tris, DEEPEST, 0.0003, in_flesh)
                if deep:
                    behind(deep, 0.008, 0.0010, in_flesh)
                if not push:
                    break
                for i, (depth, normal) in push.items():
                    out[i] -= normal * depth
        return out

    def spread(self, push, rounds=2):
        """What kept single vertices off the teeth, shared with the
        vertices round them: a vertex takes the mean of its own push and
        its neighbours' where that is more than its own, so a lip moved
        off a tooth keeps a smooth edge, not a dent per vertex."""
        out = push.copy()
        near = (self.skin | self.lining) & (self.d_line < NEAR_LIPS)
        size = np.linalg.norm(out, axis=1)
        if size.max() < 1e-7:
            return out
        for _ in range(rounds):
            new = out.copy()
            for i in np.where(near)[0]:
                nb = list(self.topo.nbr[i])
                if not size[nb].any() and not size[i]:
                    continue
                mixed = 0.5 * out[i] + 0.5 * out[nb].mean(axis=0)
                if np.linalg.norm(mixed) > size[i]:
                    new[i] = mixed
            out = new
            size = np.linalg.norm(out, axis=1)
        return out

    def shape(self, X, open_deg=0.0, side_deg=0.0, slide=None, teeth=True, shift=None, bag=False, sweep=(1.0,)):
        """The shape that takes the resting face to positions `X` (every
        vertex; with the jaw turned, `X` is where the *posed* skin should
        be). Skin is slid on the resting surface by `slide`, lips are
        kept off the teeth in both frames. Returns {vertex: offset}."""
        P = self.P
        if slide is not None:
            X = self.slide(X, slide)
        posed = self.jaw.pose(P, open_deg, side_deg)
        offsets = X - posed
        if open_deg or side_deg:
            moved = np.where(np.linalg.norm(offsets, axis=1) > 1e-7)[0]
            offsets[moved] = self.jaw.unpose(offsets[moved], open_deg, side_deg, moved)
        if teeth:
            # Off the teeth on this frame, then the same offsets on the
            # other frame's face off its teeth, and once more round (what
            # one frame needs can undo the other's).
            frames = [{"P": P, "jaw": self.jaw, "teeth": self.teeth}] + ([self.other] if self.other else [])
            for _ in range(2 if self.other else 1):
                for frame in frames:
                    jaw = frame["jaw"]
                    posed_f = jaw.pose(frame["P"] + offsets, open_deg, side_deg)
                    fixed = self.off_teeth(posed_f, open_deg, side_deg, teeth=frame["teeth"], jaw=jaw, shift=shift,
                                           bag=bag, sweep=sweep)
                    more = self.spread(fixed - posed_f)
                    moved = np.where(np.linalg.norm(more, axis=1) > 1e-7)[0]
                    if len(moved):
                        offsets[moved] += jaw.unpose(more[moved], open_deg, side_deg, moved)
        keep = np.where((np.linalg.norm(offsets, axis=1) > 2e-5) & self.topo.is_head)[0]
        return {int(i): offsets[i].copy() for i in keep}

    # -- the expression shapes ---------------------------------------------

    def _lids(self, s, upper=None, lower=None):
        return lids.deltas(self.P, self.eye[s].lid(self.P, upper_to=upper, lower_to=lower))

    def _moved(self, offsets, **kw):
        return self.shape(self.P + offsets, **kw)

    def cheek_point(self, s):
        """The cheekbone under the eye's outer corner, on the skin."""
        c = self.eye[s].c
        return self.on_skin(c + np.array([s * 0.030, -0.01, -0.027]))

    def smile(self, s, amount=1.0):
        """The corner of the mouth pulled up and out toward the cheekbone
        (zygomaticus major): the lips stretch along the teeth, the cheek
        bunches over the pull, the fold from the nose's wing deepens."""
        P, c = self.P, self.corner[s]
        pull = self.cheek_point(s) - c
        pull /= np.linalg.norm(pull)
        d = np.linalg.norm(P - c, axis=1)
        # The lips follow their corner along their length; the skin round
        # the corner follows by distance.
        along = np.clip(self.u * s, -1.0, 1.0)
        lips = self.lipness * smoothstep(-0.35, 1.0, along) ** 1.6
        round_corner = gauss(d, 0.021) * self.side(s)
        w = np.maximum(lips, round_corner)
        w[self.lid | self.socket] = 0.0
        move = w[:, None] * pull * (0.0115 * amount)
        # The upper lip lifts a little with it.
        move[:, 2] += self.lipness * self.upper_side * smoothstep(0.0, 0.9, along) * 0.0012 * amount
        # The cheek, pushed up and out ahead of the corner.
        bulge_at = self.on_skin(c + pull * 0.027 + np.array([0.0, 0.0, 0.004]))
        cheek = self.near(bulge_at, 0.019) * self.side(s) * self.skin * ~self.lid
        move += (cheek * 0.0032 * amount)[:, None] * self.N + (cheek * 0.0030 * amount)[:, None] * np.array([0.0, 0.0, 1.0])
        # The fold between the nose's wing and the corner sinks.
        wing = self.on_skin(self.nose_tip + np.array([s * 0.021, 0.018, -0.006]))
        low = self.on_skin(c + np.array([s * 0.009, -0.004, 0.004]))
        knee = self.on_skin(0.5 * (wing + low) + np.array([s * 0.004, 0.0, 0.0]))
        fold_d, _, _ = polyline_nearest(P, np.array([wing, knee, low]))
        move -= (gauss(fold_d, 0.0045) * self.skin * (1.0 - self.lipness) * 0.0011 * amount)[:, None] * self.N
        slide = np.where(self.skin, 1.0 - np.clip(self.lipness * 1.6, 0.0, 1.0), 0.0) * 0.85
        return P + move, slide

    def corner_pull(self, s, pull, reach=0.011, lips=1.0, power=1.6, radius=0.020, start=-0.35):
        """Offsets for the corner of the mouth on side `s` pulled along
        `pull` (a direction) by `reach`: the lips follow along their
        length, the skin round the corner by distance."""
        pull = np.asarray(pull, dtype=np.float64)
        pull = pull / np.linalg.norm(pull)
        d = np.linalg.norm(self.P - self.corner[s], axis=1)
        along = np.clip(self.u * s, -1.0, 1.0)
        w = np.maximum(self.lipness * lips * smoothstep(start, 1.0, along) ** power, gauss(d, radius) * self.side(s))
        w[self.lid | self.socket] = 0.0
        return w[:, None] * pull * reach

    def lip_slide(self, scale=0.85):
        """How much the skin is put back on its resting surface: all the
        way on the cheeks, not at all on the lips (which are free)."""
        return np.where(self.skin, 1.0 - np.clip(self.lipness * 1.6, 0.0, 1.0), 0.0) * scale

    def corner_taper(self, a=0.7):
        return 1.0 - smoothstep(a, 1.0, np.abs(self.u))

    def upper_lip(self, s=0):
        """The upper lip's weight (outside and its lining), on one side
        or both, nothing at the corners."""
        w = self.upper_side * gauss(self.d_line, 0.014) * self.corner_taper()
        return w * (self.side(s) if s else 1.0)

    def lower_lip(self, s=0):
        w = self.lower_side * gauss(self.d_line, 0.017) * self.corner_taper()
        return w * (self.side(s) if s else 1.0)

    def brow_line(self, s):
        """Three points of the brow on the skin (inner end, arch, tail)."""
        eye = self.eye[s]
        span = eye.t_out - eye.t_in
        out = []
        for t, rise in ((eye.t_in - 0.10 * span, 0.0150), (eye.t_in + 0.62 * span, 0.0185), (eye.t_out + 0.22 * span, 0.0140)):
            out.append(self.on_skin(eye.c + eye.axis * t + eye.up * rise + eye.fwd * 0.03))
        return np.array(out)

    def brow(self, s):
        """(weight, how far along the brow from its inner end 0 to its
        tail 1): the brow's skin, reaching further up the forehead than
        down toward the lid."""
        line = self.brow_line(s)
        d, on, along = polyline_nearest(self.P, line)
        above = smoothstep(-0.004, 0.004, self.P[:, 2] - on[:, 2])
        w = gauss(d, 0.010 + 0.014 * above) * self.skin * self.side(s, 0.006)
        w[self.lid] *= 0.0
        return w, along

    def cheek(self, s):
        """The cheek under the eye (the skin a squint or a smile lifts)."""
        at = self.on_skin(self.eye[s].c + np.array([s * 0.010, -0.012, -0.026]))
        w = self.near(at, 0.019) * self.skin * self.side(s)
        w[self.lid] = 0.0
        return w

    def expressions(self):
        """Every expression shape: {name: {vertex: offset}}."""
        P, N = self.P, self.N
        S = {}
        up, back = np.array([0.0, 0.0, 1.0]), np.array([0.0, 1.0, 0.0])
        vec = lambda w, v: np.asarray(w)[:, None] * np.asarray(v, dtype=np.float64)  # noqa: E731
        whole = np.ones(len(P))
        slide = self.lip_slide()
        pairs = {name: {} for name in ("Mouth_Smile", "Mouth_Frown", "Lip_Corner_Pull", "Upper_Lip_Raise",
                                       "Lower_Lip_Lower", "Nose_Sneer")}
        for side, s in SIDES:
            out = np.array([float(s), 0.0, 0.0])
            cheek = self.cheek(s)
            cheek_up = self._moved(vec(cheek, up * 0.0034) + (cheek * 0.0014)[:, None] * N, slide=None, teeth=False)
            # Eyes.
            S[f"Blink_{side}"] = self._lids(s, 1.0, 1.0)
            S[f"Eye_Closed_{side}"] = self._lids(s, 1.0, 1.0)
            S[f"Eye_Squint_{side}"] = add(self._lids(s, 0.16, 0.42), scale(cheek_up, 0.45))
            S[f"Eye_Wide_{side}"] = self._lids(s, -0.32, -0.10)
            S[f"Cheek_Raise_{side}"] = add(cheek_up, self._lids(s, None, 0.22))
            # Brows.
            w, along = self.brow(s)
            profile = 0.75 + 0.25 * smoothstep(0.0, 0.45, along)
            S[f"Brow_Raise_{side}"] = add(self._moved(vec(w * profile, up * 0.0078), slide=whole * self.skin, teeth=False),
                                          self._lids(s, -0.07, None))
            inner = 1.0 - smoothstep(0.0, 0.55, along)
            S[f"Brow_Lower_{side}"] = add(
                self._moved(vec(w, up * -0.0046) + vec(w * inner, out * -0.0018), slide=whole * self.skin, teeth=False),
                self._lids(s, 0.10, None))
            S[f"Brow_Inward_{side}"] = self._moved(vec(w * inner, up * 0.0046 + out * -0.0026),
                                                   slide=whole * self.skin, teeth=False)
            # The mouth's corners, the lips and the nose: each side's
            # offsets are kept, and made into shapes below.
            X, sl = self.smile(s)
            pairs["Mouth_Smile"][s] = (X - P, sl)
            frown = self.corner_pull(s, (s * 0.25, 0.10, -1.0), 0.0072, radius=0.019, start=-0.2)
            pairs["Mouth_Frown"][s] = (frown, slide)
            dimple_at = self.on_skin(self.corner[s] + np.array([s * 0.008, 0.004, 0.0]))
            pull = self.corner_pull(s, (s * 1.0, 0.55, 0.0), 0.0052, radius=0.017, start=-0.1)
            pull -= (self.near(dimple_at, 0.0055) * self.skin * 0.0016)[:, None] * N
            pairs["Lip_Corner_Pull"][s] = (pull, slide)
            ul = self.upper_lip(s)
            peak = 0.75 + 0.25 * gauss(np.abs(self.u) - 0.45, 0.3)
            wing = self.on_skin(self.nose_tip + np.array([s * 0.019, 0.016, -0.004]))
            nose = self.near(wing, 0.011) * self.skin * self.side(s)
            pairs["Upper_Lip_Raise"][s] = (vec(ul * peak, up * 0.0046 + back * 0.0008) + vec(nose, up * 0.0010), slide * 0.6)
            ll = self.lower_lip(s)
            pairs["Lower_Lip_Lower"][s] = (vec(ll, up * -0.0052 + back * -0.0010), slide * 0.6)
            sneer = vec(nose, up * 0.0034 + back * 0.0006) + vec(ul * 0.32, up * 0.0046)
            sneer += vec(w * (1.0 - smoothstep(0.0, 0.5, along)), up * -0.0013)
            flank = self.near(self.on_skin(wing + np.array([s * 0.004, 0.0, 0.012])), 0.010) * self.skin * self.side(s)
            sneer += vec(flank, up * 0.0018) + (flank * 0.0008)[:, None] * N
            pairs["Nose_Sneer"][s] = (sneer, slide * 0.6)
        # A pair's two sides are made as one shape (both sides at once,
        # kept off the teeth together) and then divided between them
        # across the middle of the face, so the two together are what was
        # made: the middle of the lips, which both sides move, is kept
        # off the teeth for both at once, not for each alone and added.
        for name, sides in pairs.items():
            both = np.zeros_like(P)
            for i, d in self._moved(sides[1][0] + sides[-1][0], slide=np.maximum(sides[1][1], sides[-1][1])).items():
                both[i] = d
            for side, s in SIDES:
                share = self.side(s)
                # (Kept off the teeth once more alone: one side's share of
                # the middle is less than the whole it was made as.)
                S[f"{name}_{side}"] = self._moved(both * share[:, None])
            if name in ("Upper_Lip_Raise", "Lower_Lip_Lower"):
                S[name] = {int(i): both[i].copy() for i in np.where(np.linalg.norm(both, axis=1) > 2e-5)[0]}
        ul, ll = self.upper_lip(), self.lower_lip()
        S["Mouth_Open"] = self._moved(vec(ul, up * 0.0020) + vec(ll, up * -0.0058 + back * -0.0006), slide=slide * 0.6)
        wide = sum(self.corner_pull(s, (s * 1.0, 0.5, -0.12), 0.0066, radius=0.019, start=-0.6, power=1.2) for _, s in SIDES)
        S["Mouth_Wide"] = self._moved(wide, slide=slide)
        # Lips drawn together: toward the middle, and forward as they
        # come round the teeth's arch.
        lips = gauss(self.d_line, 0.019) * (self.skin | self.lining)
        gather = lambda squeeze, forward, corners: (  # noqa: E731
            np.stack([-P[:, 0] * squeeze * lips, -(forward + corners * np.abs(self.u)) * lips, np.zeros(len(P))], axis=1))
        narrow = gather(0.20, 0.0032, 0.0042) + vec(ul, up * 0.0012) + vec(ll, up * -0.0020)
        S["Mouth_Narrow"] = self._moved(narrow, slide=None)
        pucker = gather(0.31, 0.0054, 0.0056) + (lips * self.skin * 0.0009)[:, None] * N
        S["Mouth_Pucker"] = self._moved(pucker, slide=None)
        for side, s in SIDES:
            mouth_w = gauss(self.d_line, 0.028) * (self.skin | self.lining)
            shift = np.stack([s * 0.0062 * mouth_w, 0.0030 * mouth_w * self.u * s, np.zeros(len(P))], axis=1)
            S[f"Mouth_{side}"] = self._moved(shift, slide=slide * 0.7)
        puff = np.zeros_like(P)
        for _, s in SIDES:
            at = self.on_skin(np.array([s * 0.047, self.corner[s][1] + 0.028, self.corner[s][2] - 0.001]))
            w = self.near(at, 0.026) * (self.skin | self.lining) * self.side(s)
            way = P - np.array([0.0, at[1] + 0.02, at[2]])
            way[:, 2] *= 0.3
            way /= np.maximum(np.linalg.norm(way, axis=1, keepdims=True), 1e-9)
            puff += (w * 0.0095)[:, None] * way
        puff += vec(gauss(self.d_line, 0.016) * self.skin, back * -0.0016)
        S["Cheek_Puff"] = self._moved(puff, slide=None)
        # The jaw: the bone turns; these shapes put right what the turn
        # alone gets wrong.
        S["Jaw_Open"] = self.jaw_open()
        for side, s in SIDES:
            posed = self.jaw.pose(P, 0.0, s * JAW_SIDE_DEG)
            S[f"Jaw_{side}"] = self.shape(posed, side_deg=s * JAW_SIDE_DEG)
        S["Jaw_Forward"] = self._moved(vec(self.jaw.w * self.topo.is_head, back * -JAW_FORWARD),
                                       shift=back * -JAW_FORWARD, sweep=(0.5, 1.0))
        # The tongue is a part of its own (`Teeth`): the body carries the
        # controls' shapes by name so the part follows them.
        for name in ("Tongue_Out", "Tongue_Up", "Tongue_Down"):
            S[name] = {}
        return {name: S[name] for name in EXPRESSIONS}

    def jaw_open(self):
        """With the jaw open all the way: the corners of the mouth come
        in (the lips make an oval, not a diamond) and the cheeks hollow
        between the jaws, and the lips stay off the teeth."""
        P = self.P
        posed = self.jaw.pose(P, JAW_OPEN_DEG)
        X = posed.copy()
        for _, s in SIDES:
            corner = posed[self.mouth.corner[s]]
            w = gauss(np.linalg.norm(posed - corner, axis=1), 0.017) * (self.skin | self.lining)
            X[:, 0] -= s * 0.0058 * w
        share = self.jaw.w * (1.0 - self.jaw.w) * 4.0
        hollow = share * smoothstep(0.024, 0.04, np.abs(P[:, 0])) * self.skin * (1.0 - self.lipness)
        X -= (hollow * 0.0022)[:, None] * self.N
        shape = self.shape(X, open_deg=JAW_OPEN_DEG)
        # Part of the way open the same shape is at part weight: what the
        # lips then still need to stay off the teeth is added in full.
        for part in (0.5, 0.25, 0.1):
            offsets = np.zeros_like(P)
            for i, d in shape.items():
                offsets[i] = d
            for frame in ([{"P": P, "jaw": self.jaw, "teeth": self.teeth}] + ([self.other] if self.other else [])):
                jaw = frame["jaw"]
                posed = jaw.pose(frame["P"] + offsets * part, JAW_OPEN_DEG * part)
                more = self.off_teeth(posed, JAW_OPEN_DEG * part, teeth=frame["teeth"], jaw=jaw) - posed
                moved = np.where(np.linalg.norm(more, axis=1) > 1e-7)[0]
                if len(moved):
                    offsets[moved] += jaw.unpose(more[moved], JAW_OPEN_DEG * part, 0.0, moved) / part
            shape = {int(i): offsets[i].copy() for i in np.where(np.linalg.norm(offsets, axis=1) > 2e-5)[0]}
        return shape


    # -- identity (Character Studio's face parameters) -----------------------

    def eye_region(self, s):
        """How much each vertex belongs to eye `s`: the lids, their lining
        and the skin round them, easing off over the loops beyond."""
        eye = self.eye[s]
        w = np.zeros(len(self.P))
        for i in eye.idx:
            step = eye.step[i]
            w[i] = 1.0 if step <= 4 else max(0.0, 1.0 - (step - 4) / 6.0)
        return self.topo.smooth(w, 2, keep=[i for i in eye.idx if eye.step[i] <= 4])

    def identity(self):
        """The face identity shapes: {name: ({vertex: offset}, {bone:
        change})}. Each is the +1 end of a parameter that runs from -1 to
        1; a bone change is {"offset": [x, y, z]} (metres, Blender axes)
        or {"scale": s} at +1. Paired features have a shape a side."""
        P, N = self.P, self.N
        out = {}
        up, back = np.array([0.0, 0.0, 1.0]), np.array([0.0, 1.0, 0.0])
        vec = lambda w, v: np.asarray(w)[:, None] * np.asarray(v, dtype=np.float64)  # noqa: E731
        skin = self.skin.astype(np.float64)
        slide = self.lip_slide()
        put = lambda name, field, bones=None, **kw: out.__setitem__(name, (self._moved(field, **kw), bones or {}))  # noqa: E731
        z_mouth, z_nose, z_chin = self.stomion[2], self.nose_tip[2], self.chin[2]
        for side, s in SIDES:
            lateral = np.array([float(s), 0.0, 0.0])
            eye = self.eye[s]
            bone = f"{side}Eye"
            w = self.eye_region(s)
            put(f"Face_Eye_Height_{side}", vec(w, up * 0.0025), {bone: {"offset": [0.0, 0.0, 0.0025]}}, teeth=False)
            put(f"Face_Eye_Spacing_{side}", vec(w, lateral * 0.0025), {bone: {"offset": [s * 0.0025, 0.0, 0.0]}}, teeth=False)
            put(f"Face_Eye_Depth_{side}", vec(w, back * 0.0020), {bone: {"offset": [0.0, 0.0020, 0.0]}}, teeth=False)
            put(f"Face_Eye_Size_{side}", (P - eye.c) * (w * 0.07)[:, None], {bone: {"scale": 1.07}}, teeth=False)
            # Tilt: the outer corner up, about the eye's own axis ahead.
            angle = np.radians(6.0) * w
            d = P - eye.c
            along, rise = d @ eye.axis, d @ eye.up
            tilt = (along * (np.cos(angle) - 1.0) - rise * np.sin(angle))[:, None] * eye.axis \
                + (along * np.sin(angle) + rise * (np.cos(angle) - 1.0))[:, None] * eye.up
            put(f"Face_Eye_Tilt_{side}", tilt * (1.0 if eye.axis[0] * s > 0 else -1.0), teeth=False)
            # Brows.
            bw, along_brow = self.brow(s)
            put(f"Face_Brow_Height_{side}", vec(bw, up * 0.0032), slide=skin, teeth=False)
            arch = gauss(along_brow - 0.62, 0.22) - 0.6 * gauss(along_brow - 0.0, 0.2)
            put(f"Face_Brow_Arch_{side}", vec(bw * arch, up * 0.0030), slide=skin, teeth=False)
            # The nose's wing, the cheek, the jaw's corner, the mouth's corner, the ear.
            wing = self.on_skin(self.nose_tip + np.array([s * 0.019, 0.016, -0.004]))
            nw = self.near(wing, 0.011) * skin * self.side(s, 0.004)
            put(f"Face_Nose_Width_{side}", vec(nw, lateral * 0.0030), teeth=False)
            bone_at = self.cheek_point(s)
            put(f"Face_Cheekbone_{side}", (self.near(bone_at, 0.022) * skin * ~self.lid * 0.0032)[:, None] * N, teeth=False)
            hollow_at = self.on_skin(np.array([s * 0.050, self.corner[s][1] + 0.022, z_mouth + 0.012]))
            put(f"Face_Cheek_Fullness_{side}", (self.near(hollow_at, 0.024) * skin * (1.0 - self.lipness) * 0.0036)[:, None] * N)
            angle_at = self.on_skin(np.array([s * 0.062, self.jaw.hinge[1] + 0.012, z_chin + 0.022]))
            jw = self.near(angle_at, 0.030) * skin * self.side(s)
            put(f"Face_Jaw_Width_{side}", vec(jw, lateral * 0.0050) + vec(jw, up * -0.0012))
            corner = self.corner_pull(s, (s * 1.0, 0.45, 0.0), 0.0032, radius=0.016, start=-0.2, power=1.3)
            put(f"Face_Mouth_Width_{side}", corner, slide=slide)
            ear = self.ear.astype(np.float64) * self.side(s, 0.001)
            root = self.ear_at[s] + np.array([-s * 0.012, -0.004, 0.0])
            # The ear eases into the head over the skin round it.
            near_ear = self.topo.smooth(ear, 3, keep=np.where(self.ear)[0])
            put(f"Face_Ear_Size_{side}", (P - root) * (near_ear * 0.12)[:, None], teeth=False)
            put(f"Face_Ear_Height_{side}", vec(near_ear, up * 0.0035), teeth=False)
            reach = np.clip((P[:, 0] * s - root[0] * s) / 0.02, 0.0, 1.5)
            put(f"Face_Ear_Out_{side}", vec(ear * reach, lateral * 0.0045 + back * -0.0015), teeth=False)
        # The middle of the face.
        mid_nose = self.near(self.nose_tip + np.array([0.0, 0.012, 0.004]), 0.024) * skin
        low_nose = smoothstep(z_nose + 0.022, z_nose - 0.006, P[:, 2])
        put("Face_Nose_Length", vec(mid_nose * low_nose, up * -0.0034 + back * -0.0010), teeth=False)
        tip = self.near(self.nose_tip, 0.013) * skin
        put("Face_Nose_Tip", vec(tip, up * 0.0026 + back * -0.0018), teeth=False)
        eye_z = self.eye[1].c[2]
        bridge_at = self.on_skin(np.array([0.0, self.nose_tip[1], 0.5 * (z_nose + eye_z) + 0.004]))
        bridge = self.near(bridge_at, 0.014) * skin * (1.0 - smoothstep(0.008, 0.02, np.abs(P[:, 0])))
        put("Face_Nose_Bridge", vec(bridge, back * -0.0030), teeth=False)
        put("Face_Nose_Tilt", vec(mid_nose * low_nose, np.array([0.0026, 0.0, 0.0])), teeth=False)
        glabella = self.on_skin(np.array([0.0, self.nose_tip[1], eye_z + 0.019]))
        ridge = gauss(P[:, 2] - glabella[2], 0.011) * (1.0 - smoothstep(0.035, 0.06, np.abs(P[:, 0]))) * skin * ~self.lid
        ridge *= P[:, 1] < glabella[1] + 0.03
        put("Face_Brow_Ridge", (ridge * 0.0026)[:, None] * N, teeth=False)
        crown = self.crown
        fore = smoothstep(eye_z + 0.02, eye_z + 0.06, P[:, 2]) * (P[:, 1] < self.ear_at[1][1]) * self.topo.is_head
        put("Face_Forehead_Height", vec(smoothstep(eye_z + 0.025, crown - 0.01, P[:, 2]) * self.topo.is_head, up * 0.0080), teeth=False)
        front = gauss(P[:, 2] - (eye_z + 0.055), 0.03) * smoothstep(-0.06, -0.11, P[:, 1]) * self.topo.is_head * ~self.lid
        put("Face_Forehead_Slope", vec(front, back * -0.0045), teeth=False)
        temple = gauss(P[:, 2] - (eye_z + 0.02), 0.035) * smoothstep(0.04, 0.07, np.abs(P[:, 0])) * (~self.ear) * self.topo.is_head
        put("Face_Head_Width", np.stack([np.sign(P[:, 0]) * temple * 0.0040, np.zeros(len(P)), np.zeros(len(P))], axis=1), teeth=False)
        del fore
        # The chin and the jaw.
        chin = self.near(self.chin + np.array([0.0, 0.008, -0.004]), 0.026) * (self.skin | self.lining) * (1.0 - self.lipness)
        put("Face_Chin_Length", vec(chin, up * -0.0050))
        put("Face_Chin_Forward", vec(chin, back * -0.0045))
        put("Face_Chin_Width", np.stack([P[:, 0] * chin * 0.16, np.zeros(len(P)), np.zeros(len(P))], axis=1))
        jaw_all = self.jaw.w * self.topo.is_head * (1.0 - self.lipness) * skin
        put("Face_Jaw_Shift", vec(jaw_all, np.array([0.0030, 0.0, 0.0])))
        # The lips. A point of one lip and the point of the other it
        # touches move alike, so the mouth stays closed.
        red = gauss(self.d_line - 0.0055, 0.0042) * skin * self.corner_taper(0.55)
        border = gauss(self.d_line - 0.0095, 0.0040) * skin * self.corner_taper(0.55)
        full = N * np.array([1.0, 1.0, 0.35])
        put("Face_Lip_Fullness_Upper", (red * self.upper_side * 0.0022)[:, None] * full
            + vec(border * self.upper_side, up * 0.0010))
        put("Face_Lip_Fullness_Lower", (red * self.lower_side * 0.0024)[:, None] * full
            + vec(border * self.lower_side, up * -0.0010))
        mouth_all = gauss(self.d_line, 0.026) * (self.skin | self.lining)
        put("Face_Mouth_Height", vec(mouth_all, up * 0.0025), teeth=False)
        return out


# -- the build's hook -----------------------------------------------------------

def to_gltf(d):
    """A Blender-space vector (Z up, front -Y) in glTF space (Y up, front +Z)."""
    return [round(float(d[0]), 5), round(float(d[2]), 5), round(float(-d[1]), 5)]


def _shape_keys(obj, shapes, count):
    """Blend shapes on `obj` from {name: {vertex: offset}} (an empty one
    is kept: a part follows the body's shape of its name)."""
    if obj.data.shape_keys is None:
        obj.shape_key_add(name="Basis", from_mix=False)
    rest = np.array([v.co[:] for v in obj.data.vertices])
    for name, shape in shapes.items():
        key = obj.shape_key_add(name=name, from_mix=False)
        for i, d in shape.items():
            key.data[i].co = rest[i] + d
        key.value = 0.0


def build(body, topo, M, F, eyes_m, eyes_f, radius, joints, joints_f, weights, teeth, out_dir):
    """Everything the face adds once the body is weighted: the expression
    and identity shapes on `body`, the parts that follow the lids and the
    brows (`EyeShell`, `Lashes`, `Brows`) with the shapes they follow by,
    and the teeth's share of the identity. `M` and `F` are the two
    frames' positions (arrays), `teeth` the `Teeth` object. Returns
    ({name: object}, what to record in the build data)."""
    import eyes as eyelib
    import lashes as hair
    M, F = np.asarray(M, dtype=np.float64), np.asarray(F, dtype=np.float64)
    jaw_w = np.asarray(weights.col("Jaw"), dtype=np.float64)
    me = teeth.data
    points = np.array([v.co[:] for v in me.vertices])
    polys = [tuple(p.vertices) for p in me.polygons]
    head_group = teeth.vertex_groups["Head"].index
    upper = np.array([any(g.group == head_group for g in v.groups) for v in me.vertices])
    feminine = np.array([k.co[:] for k in me.shape_keys.key_blocks["Frame_Feminine"].data])
    solid = [p for p, poly in zip(polys, me.polygons) if me.materials[poly.material_index].name != "Tongue"]
    jaw = Jaw(joints["Jaw"][0], jaw_w)
    other = {"P": F, "teeth": Teeth(feminine, solid, upper), "jaw": Jaw(joints_f["Jaw"][0], jaw_w)}
    face = Face(topo, M, eyes_m, radius, jaw, Teeth(points, solid, upper), other=other)
    expressions = face.expressions()
    identity = face.identity()
    shapes = dict(expressions)
    shapes.update({name: shape for name, (shape, _) in identity.items()})
    # The pupil is the eyes' own shape; the body carries its name so the
    # parameter that drives it is the body's.
    shapes["Pupil_Size"] = {}
    _shape_keys(body, shapes, len(M))
    # Parts that follow: under the frame's shape and every shape above.
    frame = {int(i): d for i, d in enumerate(F - M) if np.linalg.norm(d) > 1e-7}
    following = dict(shapes, Frame_Feminine=frame)
    order = ["Frame_Feminine", *shapes]
    made = {}
    shell, mix = eyelib.shell(face, out_dir)
    lids = set(expressions)
    hair.add_shapes(shell, {name: eyelib.follow(face, mix, following[name], name in lids) for name in order})
    made["EyeShell"] = shell
    lash, roots = hair.lashes(face, out_dir)
    at = np.array([v.co[:] for v in lash.data.vertices])
    hair.add_shapes(lash, {name: hair.follow_lashes(face, at, roots, following[name], name in lids) for name in order})
    made["Lashes"] = lash
    brow, bound = hair.brows(face, out_dir)
    hair.add_shapes(brow, {name: hair.follow_bound(bound, following[name]) for name in order})
    made["Brows"] = brow
    # The teeth go where the mouth goes.
    lift = identity["Face_Mouth_Height"][0][face.mouth.mid]
    key = teeth.shape_key_add(name="Face_Mouth_Height", from_mix=False)
    for i, p in enumerate(points):
        key.data[i].co = p + lift
    key.value = 0.0
    bones = {}
    for name, (_, changes) in identity.items():
        if changes:
            bones[name] = {"bones": {bone: ({"offset": to_gltf(c["offset"])} if "offset" in c else {"scale": c["scale"]})
                                     for bone, c in changes.items()}}
    largest = {name: round(max((float(np.linalg.norm(d)) for d in shape.values()), default=0.0) * 1000, 2)
               for name, shape in shapes.items()}
    data = {
        "expressions": list(expressions),
        "identity": list(identity),
        "face_identity": bones,
        "largest_move_mm": largest,
        "jaw_open_deg": JAW_OPEN_DEG,
    }
    return made, data
