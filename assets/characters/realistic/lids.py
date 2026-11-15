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
apart. Skin further from the rim turns less, and every vertex keeps at
least its resting height over the eyeball (the cornea's cap included),
so a closed lid lies over the cornea and nothing passes through it.

A blend shape moves a vertex in a straight line, and a lid's path is an
arc: half way through a blink the rim would sink under the eyeball's
surface by up to a millimetre. The closed lid is therefore raised where
its vertices travel furthest, by just enough that the straight path's
middle clears the white of the eye (`CLEAR`); the clear cornea standing
above the white is the one thing a half blink still dips into.
"""


import numpy as np

from facetopo import FS_HEAD, FS_SOCKET, Eyeball, rotate_about, smoothstep

# How much each loop of skin beyond the rim turns with it: the lid proper
# (loops 3 and 4) nearly all the way, the fold above it (5, 6) unrolling,
# the brow's edge hardly.
SKIN = {3: 1.0, 4: 0.92, 5: 0.62, 6: 0.30, 7: 0.10}
# A closed lid's rims end this far apart (metres), and no vertex of a lid
# comes nearer the eyeball than this (the lining's flat faces between
# its vertices then stay off the round eyeball too).
SEAM = 0.00012
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
# How far along the opening the lids meet, from the lower lid's rim: the
# lower lid rises a tenth of the way, the upper comes down the rest.
MEET = 0.10


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

    def closing_line(self, t, meet=MEET):
        return self.arc(1, False, t) + meet * self.opening(t)

    def _to_line(self, X, t, upper, share, loop=1):
        """The turn that takes rim points `X` `share` of the way from
        their loop's resting arc to the closing line."""
        seam = 0.5 * SEAM if upper else -0.5 * SEAM
        rest = self.arc(loop, upper, t)
        target = rest + share * (self.closing_line(t) + seam - rest)
        reach = np.maximum(self.reach(X), 1e-6)
        # A point keeps its distance from the turning line; its height
        # above the line says its angle.
        now = self.rise(X)
        return np.arcsin(np.clip(target / reach, -1.0, 1.0)) - np.arcsin(np.clip(now / reach, -1.0, 1.0))

    def _turns(self, P, upper, share):
        """The turn (radians, up positive) of one lid's rim by position
        along the axis: a table (along, angle) from the rim's inner edge."""
        ends = (self.canthus_in, self.canthus_out)
        loop = np.array([i for i in self.loops[1] if self.upper[i] == upper or i in ends])
        t = self.along(P[loop])
        order = np.argsort(t)
        loop, t = loop[order], t[order]
        angle = self._to_line(P[loop], t, upper, share)
        angle[0] = angle[-1] = 0.0
        return t, angle

    def lid(self, P, upper_to=None, lower_to=None, clear=True):
        """Positions with the lids turned. `upper_to` and `lower_to` say
        where a lid's rim goes, as a share of the way from its resting
        place to the closing line (1 closes it, negative opens it wider);
        None leaves that lid alone. Returns {vertex: new position}."""
        idx = self.idx
        X = P[idx]
        t = np.clip(self.along(X), self.t_in, self.t_out)
        ends = (self.canthus_in, self.canthus_out)
        angles = np.zeros(len(idx))
        for upper, share in ((True, upper_to), (False, lower_to)):
            if share is None:
                continue
            table = self._turns(P, upper, share)
            mine = np.array([self.upper[i] == upper for i in idx])
            angles[mine] = np.interp(t[mine], table[0], table[1])
            # The rim's own two edges go exactly to the line.
            for loop in (1, 2):
                rim = np.array([k for k, i in enumerate(idx)
                                if mine[k] and self.step[i] == loop and i not in ends], dtype=np.int64)
                if len(rim):
                    exact = self._to_line(X[rim], t[rim], upper, share, loop)
                    along = self.along(X[rim])
                    inside = (along > self.t_in) & (along < self.t_out)
                    angles[rim] = np.where(inside, exact, angles[rim])
        angles *= np.array([self.weight[i] for i in idx])
        moved = np.abs(angles) > 1e-7
        turned = rotate_about(X, self.c, self.turn_axis, angles)
        # Every vertex keeps its resting height over the eyeball, and the
        # lining stays off it.
        step = np.array([self.step[i] for i in idx])
        rest_height = np.maximum(self.ball.above(X), np.where(step <= 0, UNDER_LID, CLEARANCE))
        d = turned - self.c
        length = np.linalg.norm(d, axis=1)
        unit = d / np.maximum(length, 1e-12)[:, None]
        turned = self.c + unit * (self.ball.height(unit) + rest_height)[:, None]
        # The lid's skin beyond its rim takes the turn smoothly: each
        # vertex's move is averaged with its neighbours' (the rim and the
        # lining keep theirs), so no loop folds against the next where
        # the turn dies out at the corners.
        step = np.array([self.step[i] for i in idx])
        where = {int(i): k for k, i in enumerate(idx)}
        move = turned - X
        loose = [k for k in range(len(idx)) if step[k] >= 3]
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
            # raise the far end where it wouldn't.
            for _ in range(3):
                middle = 0.5 * (X + turned)
                short = (self.r + CLEAR) - np.linalg.norm(middle - self.c, axis=1)
                short = np.where(moved, np.maximum(short, 0.0), 0.0)
                if short.max() <= 0:
                    break
                d = turned - self.c
                length = np.linalg.norm(d, axis=1)
                turned = self.c + d * ((length + 2.0 * short) / length)[:, None]
        return {int(i): turned[k] for k, i in enumerate(idx) if moved[k]}

    def seat(self, P):
        """`P` with the lids' lining lifted off the eyeball where the
        sculpt has it under the surface (it was shaped round the bundle's
        eyeball, which isn't quite this one). Returns new positions."""
        out = P.copy()
        step = np.array([self.step[i] for i in self.idx])
        out[self.idx] = self.ball.lift(P[self.idx], np.where(step <= 0, UNDER_LID, CLEARANCE))
        return out


def deltas(P, moved):
    """{vertex: new position} as {vertex: offset from `P`}."""
    return {i: p - P[i] for i, p in moved.items()}
