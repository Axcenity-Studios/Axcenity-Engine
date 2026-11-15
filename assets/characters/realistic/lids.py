"""The eyelids of the realistic head (Character Studio overhaul CO6): the
loops round each eye as the bundle's topology has them, and lid shapes
that turn the lids about the eyeball so they slide on it.

Round each eye the head has concentric loops. Counting out from the
border of the socket's lining (face sets 2 and 3) with the face:

    0   on the eyeball, under the lid (where the lining starts)
    1   the lid's inner edge (the wet line; 20 vertices)
    2   the lid's outer edge (the lash line; the lid's rim is the strip
        between 1 and 2)
    3+  the lid's skin, out to the brow and the cheek

A lid shape turns every vertex about the line through the eyeball's
centre along the eye's own corner-to-corner direction. A vertex is at an
angle about that line (its *elevation*: 0 straight ahead, up positive)
and a position along it. Each loop's upper and lower arcs give an
elevation for every position along the line, so "close" can be said
exactly: every rim vertex of both lids ends on one closing line, a hair
apart. Both edges of a rim go the same share of the way to that line (a
squint's line lies higher in the opening than a blink's, `meet`), the
lining by the inner edge's turn and the skin by the outer edge's, each
loop of skin trailing the one before. Every vertex keeps at least its
resting height over the eyeball (the cornea's cap included), so a closed
lid lies over the cornea and nothing passes through it; the rim's outer
edge ends outside its inner edge by the margin's width, so a closed
lid's margin lies flat on the other's.

A blend shape moves a vertex in a straight line, and a lid's path is an
arc: half way through a blink the rim would sink under the eyeball's
surface by up to a millimetre. The closed lid is therefore raised, whole
(`RAISED`), by just enough that the straight path's middle clears the
white of the eye at the rim (`CLEAR`); the clear cornea standing above
the white is the one thing a half blink still dips into. For the same
reason two shapes that move one lid (a squint and a raised cheek) are
made as shares of one path (`face.Face.expressions`): two arcs' chords
added together leave the eyeball.
"""


import numpy as np

from facetopo import FS_HEAD, FS_SOCKET, Eyeball, rotate_about, smoothstep

# How much each loop of skin beyond the rim's outer edge turns with it. Each
# loop trails the one before, so a closing lid's skin is paid out evenly
# from the lash line up to the fold (with loops 3 and 4 turning all the way
# the lash line's strip folded shut and the fold's one strip took all the
# stretch: streaks down the closed lid).
SKIN = {3: 0.90, 4: 0.78, 5: 0.55, 6: 0.28, 7: 0.10}
# The same for an upper lid drawn up (eyes wide, a raised brow).
SKIN_UP = {3: 0.85, 4: 0.55, 5: 0.36, 6: 0.50, 7: 0.30}
# A closed lid's rims end this far apart (metres): on one line. (The two
# rims' vertices lie at different places along it, so their edges already
# part by up to 0.15 mm where the line bends; with a seam besides, the
# eyeball showed between them at the inner corner.) No vertex of a lid
# comes nearer the eyeball than `CLEARANCE` (the lining's flat faces
# between its vertices then stay off the round eyeball too).
SEAM = 0.0
CLEARANCE = 0.0005
# The lining under a lid (hidden) keeps this far off the eyeball: its flat
# faces, stretched by a lid's turn, then stay off the round eyeball.
UNDER_LID = 0.0009
# A turned lid's skin (from its outer rim out) ends at least this far
# above the height its lining keeps: the lining never shows through it.
OVER_LINING = 0.0005
# How many times the lid's skin has its turn averaged with its neighbours'.
SMOOTH = 3
# Half way along a shape's path a lid vertex is at least this far above
# the white of the eye.
CLEAR = 0.0005
# How much of what the rim is raised by for that (below) each loop of the
# lid takes: the lid is raised whole, its skin outside its inner edge, and
# the raise dies out toward the fold.
RAISED = {1: 1.0, 2: 1.0, 3: 1.0, 4: 0.95, 5: 0.70, 6: 0.35, 7: 0.10}
# How far along the opening the lids meet, from the lower lid's rim: the
# lower lid rises a fifth of the way, the upper comes down the rest. (The
# further the upper lid travels, the further its straight path cuts under
# the eyeball's curve and the higher the closed lid must stand to clear
# it: its inner edge 2.2 mm off the eyeball at a tenth, 1.3 mm at a fifth.)
MEET = 0.20
# How far inside each corner of the eye the closing line starts running
# straight to the corner (metres).
CORNER_RUN = 0.0022
# A lid that narrows the eye without closing it (`ease`) reaches its full
# share this far from the inner and the outer corner (metres), for the
# upper lid (True) and the lower. The lower lid is held at the inner
# corner (the tendon there): pushed up at full share 3 mm from it, its
# edge stood as a ramp over the still skin beside the nose, which lit as
# a pale patch.
CORNER_EASE = {True: (0.0025, 0.0025), False: (0.0050, 0.0025)}


class Eye:
    """One eye's lids on positions `P`."""

    def __init__(self, topo, P, side, centre, radius):
        self.topo, self.s = topo, side
        self.ball = Eyeball(centre, radius)
        self.c = np.asarray(centre, dtype=np.float64)
        self.r = float(radius)
        fs = FS_SOCKET[side]
        ring = topo.ring(fs, FS_HEAD)
        skin = lambda i: topo.vsets[i] == {FS_HEAD} and P[i][0] * side > 0  # noqa: E731
        self.loops = topo.levels(ring, skin, most=12)
        self.lining = topo.levels(ring, lambda i: topo.vsets[i] == {fs}, most=20)
        # Steps from the rim's inner edge, outward positive, into the
        # socket negative.
        self.step = {}
        for k, loop in enumerate(self.loops):
            for i in loop:
                self.step[i] = k
        for k, loop in enumerate(self.lining[1:], 1):
            for i in loop:
                self.step[i] = -k
        inner = np.array(self.loops[1])
        x = P[inner][:, 0] * side
        self.canthus_in, self.canthus_out = int(inner[np.argmin(x)]), int(inner[np.argmax(x)])
        a = P[self.canthus_out] - P[self.canthus_in]
        a[1] = 0.0          # the line the lids turn about stays in the face's plane
        self.axis = a / np.linalg.norm(a)
        f = np.array([0.0, -1.0, 0.0])
        f -= self.axis * (f @ self.axis)
        self.fwd = f / np.linalg.norm(f)
        self.up = np.cross(self.axis, self.fwd)
        if self.up[2] < 0:
            self.up = -self.up
        # The turn that raises a point in front of the eye is about -axis
        # or +axis, whichever lifts `fwd` toward `up`.
        self.turn_axis = self.axis if np.cross(self.axis, self.fwd) @ self.up > 0 else -self.axis
        self.t_in = float((P[self.canthus_in] - self.c) @ self.axis)
        self.t_out = float((P[self.canthus_out] - self.c) @ self.axis)
        # Each loop's arcs: elevation by position along the axis.
        self.arcs = {}
        for k in (0, 1, 2):
            self.arcs[k] = self._arcs(P, self.loops[k])
        self.idx = np.array(sorted(self.step), dtype=np.int64)
        self._classify(P)
        # The lid margin: how far each vertex of the rim's outer edge is
        # from the inner edge, and the inner edge's height over the eyeball
        # by position along the axis, for each lid.
        self.margin, self.inner_above = {}, {}
        for upper in (True, False):
            ends = (self.canthus_in, self.canthus_out)
            inner = np.array([i for i in self.loops[1] if self.upper[i] == upper or i in ends])
            inner = inner[np.argsort(self.along(P[inner]))]
            self.inner_above[upper] = (self.along(P[inner]), np.maximum(self.ball.above(P[inner]), CLEARANCE))
            for i in self.loops[2]:
                if self.upper[i] == upper:
                    self.margin[i] = float(_to_polyline(P[i], P[inner]))

    # -- coordinates -----------------------------------------------------

    def along(self, P):
        return (np.atleast_2d(P) - self.c) @ self.axis

    def rise(self, P):
        """Height above the turning line, along the eye's own up."""
        return (np.atleast_2d(P) - self.c) @ self.up

    def reach(self, P):
        """Distance from the turning line."""
        d = np.atleast_2d(P) - self.c
        return np.hypot(d @ self.up, d @ self.fwd)

    def _arcs(self, P, loop):
        """(upper, lower): each an (along, rise) table of a loop's arc
        from corner to corner, sorted along the axis."""
        loop = np.array(loop)
        t, h = self.along(P[loop]), self.rise(P[loop])
        # Round the loop, by the angle about its middle seen from the front.
        angle = np.arctan2(h - h.mean(), t - t.mean())
        order = np.argsort(angle)
        t, h = t[order], h[order]
        first, last = int(np.argmin(t)), int(np.argmax(t))
        n = len(loop)
        walk = lambda a, b: [(a + k) % n for k in range((b - a) % n + 1)]  # noqa: E731
        one, two = walk(first, last), walk(last, first)
        if h[one].mean() < h[two].mean():
            one, two = two, one
        out = []
        for arc in (one, two):
            arc = sorted(arc, key=lambda k: t[k])
            out.append((t[arc], h[arc]))
        return out

    def arc(self, loop, upper, t):
        table = self.arcs[loop][0 if upper else 1]
        return np.interp(t, table[0], table[1])

    def _classify(self, P):
        """Which lid each vertex belongs to, and how much it turns with
        its rim: 1 at the rim and under it, easing off over the skin
        beyond it and over the lining behind it."""
        idx = self.idx
        t, h = self.along(P[idx]), self.rise(P[idx])
        tc = np.clip(t, self.t_in, self.t_out)
        mid = 0.5 * (self.arc(1, True, tc) + self.arc(1, False, tc))
        self.upper = dict(zip(idx.tolist(), (h > mid).tolist()))
        step = np.array([self.step[i] for i in idx])
        w = np.array([SKIN.get(int(k), 0.0) if k > 2 else 1.0 for k in step])
        back = step < 0
        w[back] = 1.0 - smoothstep(0.0, 1.0, (-step[back]) / 5.0)
        self.weight = dict(zip(idx.tolist(), w.tolist()))

    # -- shapes ----------------------------------------------------------

    def opening(self, t):
        """The height between the rims' inner edges at `t` along the axis."""
        return self.arc(1, True, t) - self.arc(1, False, t)

    def closing_line(self, t, meet=MEET, arch=0.0):
        """The height of the line the lids meet on at `t` along the axis:
        `meet` of the way up the opening, and from `CORNER_RUN` inside
        each corner straight to the corner. (Both corners are blunt: the
        rims part by 2 to 4 mm within half a millimetre of them. A line
        that followed the opening there dropped away from the corner
        faster than the two rims' vertices could follow alike, and a
        closed eye kept a small three-cornered hole at its inner corner
        with the eyeball showing in it.)"""
        knots = np.linspace(self.t_in + CORNER_RUN, self.t_out - CORNER_RUN, 12)
        height = self.arc(1, False, knots) + meet * self.opening(knots)
        # (`arch`: raised by that much at the middle of the eye, by nothing
        # at its corners: the line of an eye a raised cheek has narrowed.)
        height = height + arch * np.sin(np.pi * (knots - self.t_in) / (self.t_out - self.t_in))
        ends = self.arc(1, False, np.array([self.t_in, self.t_out]))
        return np.interp(t, np.concatenate([[self.t_in], knots, [self.t_out]]),
                         np.concatenate([[ends[0]], height, [ends[1]]]))

    def _to_line(self, X, t, upper, share, loop=1, meet=MEET, arch=0.0):
        """The turn that takes rim points `X` `share` of the way from
        their loop's resting arc to the closing line."""
        seam = 0.5 * SEAM if upper else -0.5 * SEAM
        rest = self.arc(loop, upper, t)
        target = rest + share * (self.closing_line(t, meet, arch) + seam - rest)
        reach = np.maximum(self.reach(X), 1e-6)
        # A point keeps its distance from the turning line; its height
        # above the line says its angle.
        now = self.rise(X)
        return np.arcsin(np.clip(target / reach, -1.0, 1.0)) - np.arcsin(np.clip(now / reach, -1.0, 1.0))

    def _turns(self, P, upper, share, meet=MEET, loop=1, ease=False, arch=0.0):
        """The turn (radians, up positive) of one lid's rim by position
        along the axis: a table (along, angle) from the rim's inner edge
        (`loop` 1) or its outer edge (2). Nothing turns at the corners."""
        ends = (self.canthus_in, self.canthus_out)
        if loop == 1:
            ring = np.array([i for i in self.loops[1] if self.upper[i] == upper or i in ends])
        else:
            ring = np.array([i for i in self.loops[2] if self.upper[i] == upper
                             and self.t_in < float(self.along(P[i])[0]) < self.t_out])
        t = self.along(P[ring])
        order = np.argsort(t)
        ring, t = ring[order], t[order]
        if ease:
            # A lid that only narrows the eye does it less toward the
            # corners (a lid that closes goes all the way everywhere):
            # at its full share right up to a corner, the lower lid's
            # edge ran down to the still corner as a lit ridge.
            inner, outer = CORNER_EASE[bool(upper)]
            share = share * np.minimum(smoothstep(0.0, inner, t - self.t_in), smoothstep(0.0, outer, self.t_out - t))
        angle = self._to_line(P[ring], t, upper, share, loop, meet, arch)
        if loop == 1:
            angle[0] = angle[-1] = 0.0
            return t, angle
        return np.concatenate([[self.t_in], t, [self.t_out]]), np.concatenate([[0.0], angle, [0.0]])

    def lid(self, P, upper_to=None, lower_to=None, clear=True, meet=MEET, ease=False, arch=0.0):
        """Positions with the lids turned. `upper_to` and `lower_to` say
        where a lid's rim goes, as a share of the way from its resting
        place to the closing line (1 closes it, negative opens it wider);
        None leaves that lid alone. `meet` is where the closing line lies
        in the opening (0 the lower rim, 1 the upper) and `arch` how far
        it is raised at the middle of the eye: a blink's is low and
        follows the opening, a squint's is near the middle and arched. Both edges of a rim go the same share of
        the way to one line, so they keep their order at every share (a
        share past 1 would carry the outer edge, which starts further
        from the line near the corners, over the inner one). With `ease`
        the lids go less far toward the corners (a squint). Returns
        {vertex: new position}."""
        idx = self.idx
        X = P[idx]
        along = self.along(X)
        t = np.clip(along, self.t_in, self.t_out)
        step = np.array([self.step[i] for i in idx])
        angles = np.zeros(len(idx))
        closing = np.zeros(len(idx))
        lid_of = np.array([self.upper[i] for i in idx])
        for upper, share in ((True, upper_to), (False, lower_to)):
            if share is None:
                continue
            mine = lid_of == upper
            # The lining and the inner edge go by the inner edge's turn,
            # the outer edge and the skin beyond it by the outer edge's:
            # the skin then trails its own rim, not the other edge.
            for loop, which in ((1, mine & (step <= 1)), (2, mine & (step >= 2))):
                table = self._turns(P, upper, share, meet, loop, ease, arch)
                angles[which] = np.interp(t[which], table[0], table[1])
            closing[mine] = min(max(share, 0.0), 1.0)
        angles *= np.array([self.weight[i] for i in idx])
        # An upper lid drawn up gathers its skin under the fold: the fold's
        # two loops go up together (by `SKIN`, made for a lid coming down,
        # the lower of them passed the upper and the crease turned inside
        # out: lit slivers over a wide eye).
        gathers = (angles > 0) & lid_of & (step >= 3)
        angles[gathers] *= np.array([SKIN_UP.get(int(k), 1.0) / max(SKIN.get(int(k), 1.0), 1e-9) if SKIN.get(int(k)) else 1.0
                                     for k in step[gathers]])
        turned = rotate_about(X, self.c, self.turn_axis, angles)
        # Every vertex keeps its resting height over the eyeball, and the
        # lining stays off it. The rim's outer edge stands off the inner
        # one by the margin's width as the lids meet: a closed lid's
        # margin lies flat against the other's, its outer edge outermost
        # (on one height with the inner edge the margin's strip has no
        # area, and nearer the eyeball than it the lining shows).
        rest_height = np.maximum(self.ball.above(X), np.where(step <= 0, UNDER_LID, CLEARANCE))
        for k in np.where((step == 2) & (closing > 0))[0]:
            i = int(idx[k])
            table = self.inner_above[self.upper[i]]
            flat = float(np.interp(t[k], table[0], table[1])) + self.margin.get(i, 0.0)
            # (Not at the corners, where the edges run into each other.)
            inside = min(along[k] - self.t_in, self.t_out - along[k]) / 0.003
            rest_height[k] += max(flat - rest_height[k], 0.0) * closing[k] * min(max(inside, 0.0), 1.0)
        d = turned - self.c
        length = np.linalg.norm(d, axis=1)
        unit = d / np.maximum(length, 1e-12)[:, None]
        turned = self.c + unit * (self.ball.height(unit) + rest_height)[:, None]
        # The lid's skin beyond its rim takes the turn smoothly: each
        # vertex's move is averaged with its neighbours' (the rim and the
        # lining keep theirs), so no loop folds against the next where
        # the turn dies out at the corners.
        where = {int(i): k for k, i in enumerate(idx)}
        move = turned - X
        # (And the outer edge's vertices that run round a corner, outside
        # the span of the rims: they have no turn of their own.)
        loose = [k for k in range(len(idx)) if step[k] >= 3
                 or (step[k] == 2 and not self.t_in < along[k] < self.t_out)]
        for _ in range(SMOOTH):
            new = move.copy()
            for k in loose:
                nb = [move[where[j]] if j in where else np.zeros(3) for j in self.topo.nbr[int(idx[k])]]
                new[k] = 0.5 * move[k] + 0.5 * np.mean(nb, axis=0)
            move = new
        moved = np.linalg.norm(move, axis=1) > 1e-7
        # A lid's skin stays over its own lining as it turns: the further
        # a vertex has come, the nearer it is held to that height.
        far = np.clip(np.linalg.norm(move, axis=1) / 0.002, 0.0, 1.0)
        over = np.maximum(rest_height, np.where(step >= 2, UNDER_LID + OVER_LINING, 0.0))
        rest_height = rest_height + (over - rest_height) * far
        turned = self.ball.lift(X + move, rest_height)
        if clear:
            # The straight path's middle clears the white of the eye:
            # raise the far end where it wouldn't. A lid is raised whole,
            # by what its rim needs at each place along the eye: raised
            # vertex by vertex, the inner edge (which travels furthest)
            # ended outside the skin and the lining showed as specks.
            profile = np.array([RAISED.get(int(k), 0.0) for k in step])
            for _ in range(4):
                middle = 0.5 * (X + turned)
                short = (self.r + CLEAR) - np.linalg.norm(middle - self.c, axis=1)
                short = np.where(moved, np.maximum(short, 0.0), 0.0)
                if short.max() <= 1e-7:
                    break
                lift = short.copy()
                for upper in (True, False):
                    rim = np.where((lid_of == upper) & ((step == 1) | (step == 2)))[0]
                    if not len(rim) or short[rim].max() <= 0:
                        continue
                    rim = rim[np.argsort(along[rim])]
                    need = short[rim]
                    # (An envelope: each place takes the most its neighbours need.)
                    need = np.maximum.reduce([need, np.concatenate([need[1:], [0.0]]), np.concatenate([[0.0], need[:-1]])])
                    mine = np.where((lid_of == upper) & (step >= 1) & moved)[0]
                    lift[mine] = np.maximum(lift[mine], np.interp(along[mine], along[rim], need) * profile[mine])
                d = turned - self.c
                length = np.linalg.norm(d, axis=1)
                turned = self.c + d * ((length + 2.0 * lift) / length)[:, None]
        # Raising the lining for the half-way path must not put it over
        # the lid's own skin: skin near any lining stands above it.
        lining = np.where(step <= 0)[0]
        if len(lining):
            height = np.linalg.norm(turned[lining] - self.c, axis=1)
            for k in np.where((step >= 2) & moved)[0]:
                near = np.linalg.norm(turned[lining] - turned[k], axis=1) < 0.004
                if near.any():
                    need = height[near].max() + OVER_LINING
                    have = np.linalg.norm(turned[k] - self.c)
                    if have < need:
                        turned[k] = self.c + (turned[k] - self.c) * (need / have)
        return {int(i): turned[k] for k, i in enumerate(idx) if moved[k]}

    def seat(self, P):
        """`P` with the lids' lining lifted off the eyeball where the
        sculpt has it under the surface (it was shaped round the bundle's
        eyeball, which isn't quite this one). Returns new positions."""
        out = P.copy()
        step = np.array([self.step[i] for i in self.idx])
        out[self.idx] = self.ball.lift(P[self.idx], np.where(step <= 0, UNDER_LID, CLEARANCE))
        return out


def _to_polyline(p, line):
    """The distance from point `p` to the polyline `line`."""
    a, b = line[:-1], line[1:]
    seg = b - a
    u = np.clip(((p - a) * seg).sum(axis=1) / np.maximum((seg * seg).sum(axis=1), 1e-18), 0.0, 1.0)
    return np.linalg.norm(a + u[:, None] * seg - p, axis=1).min()


def deltas(P, moved):
    """{vertex: new position} as {vertex: offset from `P`}."""
    return {i: p - P[i] for i, p in moved.items()}
