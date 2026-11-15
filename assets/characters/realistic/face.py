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


def soften_lids(body, topo, P, eyes, radius, maps):
    """The skin maps with the upper lids' fine detail taken out (in the
    files, then reloaded). A closing lid stretches the skin of its fold
    to four times its length, and pores and creases baked there stretch
    into streaks down the closed lid. `maps` is `skin.bake`'s result;
    `body` has its texture coordinates. Returns the texels changed."""
    from PIL import Image, ImageDraw, ImageFilter
    layer = body.data.uv_layers[0].data
    lid = np.zeros(topo.count, dtype=bool)
    for _, s in SIDES:
        eye = lids.Eye(topo, P, s, eyes[s], radius)
        lid[[i for i in eye.idx if 1 <= eye.step[i] <= 6 and eye.upper[i]]] = True
    changed = 0
    for key in ("albedo", "normal", "orm"):
        image = maps[key]
        path = bpy_path(image)
        picture = Image.open(path)
        w, h = picture.size
        mask = Image.new("L", (w, h), 0)
        draw = ImageDraw.Draw(mask)
        for poly in body.data.polygons:
            if all(lid[v] for v in poly.vertices):
                uv = [layer[k].uv for k in poly.loop_indices]
                draw.polygon([(u * w, (1.0 - v) * h) for u, v in uv], fill=255)
        mask = mask.filter(ImageFilter.GaussianBlur(w / 500.0))
        m = np.asarray(mask, dtype=np.float32)[:, :, None] / 255.0
        src = np.asarray(picture.convert("RGB"), dtype=np.float32)
        soft = np.asarray(picture.convert("RGB").filter(ImageFilter.GaussianBlur(w / 170.0)), dtype=np.float32)
        if key == "normal":
            # And most of the way to flat: a crease baked for the open lid
            # is wrong on the closed one.
            soft = 0.5 * soft + 0.5 * np.array([128.0, 128.0, 255.0], dtype=np.float32)
        out = src * (1.0 - m) + soft * m
        changed = int((m[:, :, 0] > 0.5).sum())
        result = Image.fromarray(np.clip(out + 0.5, 0, 255).astype(np.uint8))
        if path.lower().endswith((".jpg", ".jpeg")):
            result.save(path, quality=95, subsampling=0)
        else:
            result.save(path, optimize=True)
        image.reload()
    return changed


def bpy_path(image):
    import bpy
    return bpy.path.abspath(image.filepath)


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
# Identity: how much the eyeball and the lids' rims grow at eye size +1,
# how much the skin further out does, and how far the mouth rises at
# mouth height +1 (metres).
EYE_GROWS = 1.12
SKIN_GROWS = 1.20
MOUTH_RISES = 0.0045
JAW_SHIFTS = 0.0050
# How far Jaw_Forward brings the jaw (metres).
JAW_FORWARD = 0.0055
# The jaw opening wide (`Jaw_Open` at 1, metres). The bone only turns
# about its hinge, which carries the chin and the lower lip 29 mm back
# under the upper teeth; a jaw's joint also slides forward as it opens,
# so the shape brings all the jaw holds forward by `JAW_GLIDE` (the
# lower teeth and the tongue carry the same in their own `Jaw_Open`).
JAW_GLIDE = 0.019
# The lower lip stands this much higher than the turn leaves it: it
# covers the lower teeth, whose tips the turn alone raises over it.
JAW_LIP_UP = 0.0050
JAW_LIP_BACK = 0.0020
# How much of `JAW_LIP_UP` each loop of the lower lip takes (by loop from
# the fold behind the lip: `mouth.Mouth.lower`).
LIP_RISES = {1: 0.8, 2: 1.0, 3: 1.0, 4: 0.9, 5: 0.65, 6: 0.35, 7: 0.12}
# The mouth's corners come in round the opening by this much, and
# forward along the teeth by this much.
JAW_CORNER_IN = 0.0045
JAW_CORNER_FORWARD = 0.0025
# The tongue with the jaw open (metres, in the jaw's own space). Turned
# with the jaw as it rests, it is a ramp from the lower incisors to the
# upper molars: it hides the back of the mouth, and the far row of lower
# teeth stands bare beside it. So:
# - its front half stands `TONGUE_RISES` higher, level with the lower
#   teeth's tops, domed by `TONGUE_DOME` along its middle, and its tip
#   comes forward and up onto the lower incisors (`TONGUE_TIP`);
# - its upper side spreads flat (`TONGUE_FLAT` of the way) out to
#   `TONGUE_ROOM` from the teeth and `TONGUE_OVER` onto their inner cusps;
# - its back half sinks toward the throat, by `TONGUE_SINKS` at the root
#   along its middle and `TONGUE_EDGES` less of that at its edges, which
#   stay up along the teeth, and is thinner by `TONGUE_THINS` of itself
#   at the root, so it stays off the mouth's floor.
TONGUE_RISES = 0.0015
TONGUE_DOME = 0.0020
TONGUE_TIP = (0.0060, 0.0027)
TONGUE_FLAT = 0.75
TONGUE_ROOM = 0.0006
TONGUE_OVER = 0.0030
TONGUE_SINKS = 0.018
TONGUE_EDGES = 0.55
TONGUE_THINS = 0.60


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

    @staticmethod
    def glide():
        """`JAW_GLIDE` as an offset of what rests on the jaw (a shape is
        added before the bone turns): forward once the jaw is open."""
        return Jaw.turn(JAW_OPEN_DEG).T @ np.array([0.0, -JAW_GLIDE, 0.0])

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
        own Jaw_Forward shape) and by their `Jaw_Open` shape's glide,
        and turned with the jaw."""
        pts = self.points.copy()
        if shift is not None:
            pts[~self.upper] += shift
        if open_deg:
            pts[~self.upper] += Jaw.glide() * (open_deg / JAW_OPEN_DEG)
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

    def spread(self, push, reach=3, rounds=4):
        """What kept single vertices off the teeth, made into one smooth
        lift of the lip: held off the teeth point by point a lip takes
        the print of each tooth. Every vertex first takes the largest
        push within `reach` edges of it (the lip clears the highest
        tooth near it), then the pushes are averaged with their
        neighbours', never to less than a vertex needs itself."""
        near = np.where((self.skin | self.lining) & (self.d_line < NEAR_LIPS))[0]
        need = np.linalg.norm(push, axis=1)
        if need.max() < 1e-7:
            return push
        out = push.copy()
        size = need.copy()
        for _ in range(reach):
            new, new_size = out.copy(), size.copy()
            for i in near:
                for j in self.topo.nbr[i]:
                    if size[j] > new_size[i]:
                        new[i], new_size[i] = out[j], size[j]
            out, size = new, new_size
        for _ in range(rounds):
            new = out.copy()
            for i in near:
                nb = list(self.topo.nbr[i])
                mixed = 0.5 * out[i] + 0.5 * out[nb].mean(axis=0)
                if np.linalg.norm(mixed) >= need[i]:
                    new[i] = mixed
            out = new
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
            # What the smoothing left a hair inside the teeth, as it is.
            for frame in frames:
                jaw = frame["jaw"]
                posed_f = jaw.pose(frame["P"] + offsets, open_deg, side_deg)
                more = self.off_teeth(posed_f, open_deg, side_deg, teeth=frame["teeth"], jaw=jaw, shift=shift,
                                      bag=bag, sweep=sweep) - posed_f
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
        pull = pull + np.array([s * 0.25, 0.45, 0.0])
        pull /= np.linalg.norm(pull)
        d = np.linalg.norm(P - c, axis=1)
        # The lips follow their corner along their length; the skin round
        # the corner follows by distance.
        along = np.clip(self.u * s, -1.0, 1.0)
        lips = self.lipness * smoothstep(-0.35, 1.0, along) ** 1.15
        round_corner = gauss(d, 0.021) * self.side(s)
        w = np.maximum(lips, round_corner)
        w[self.lid | self.socket] = 0.0
        move = w[:, None] * pull * (0.0128 * amount)
        # The upper lip lifts a little with it.
        # The cheek, pushed up and out ahead of the corner.
        bulge_at = self.on_skin(c + pull * 0.027 + np.array([0.0, 0.0, 0.004]))
        cheek = self.near(bulge_at, 0.019) * self.side(s) * self.skin * ~self.lid
        move += (cheek * 0.0060 * amount)[:, None] * self.N + (cheek * 0.0052 * amount)[:, None] * np.array([0.0, 0.0, 1.0])
        # The fold between the nose's wing and the corner sinks.
        wing = self.on_skin(self.nose_tip + np.array([s * 0.021, 0.018, -0.006]))
        low = self.on_skin(c + np.array([s * 0.009, -0.004, 0.004]))
        knee = self.on_skin(0.5 * (wing + low) + np.array([s * 0.004, 0.0, 0.0]))
        fold_d, _, _ = polyline_nearest(P, np.array([wing, knee, low]))
        move -= (gauss(fold_d, 0.0045) * self.skin * (1.0 - self.lipness) * 0.0030 * amount)[:, None] * self.N
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
        w = gauss(d, 0.013 + 0.016 * above) * self.skin * self.side(s, 0.006)
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
            cheek_up = self._moved(vec(cheek, up * 0.0072) + (cheek * 0.0034)[:, None] * N, slide=None, teeth=False)
            # Eyes.
            S[f"Blink_{side}"] = self._lids(s, 1.0, 1.0)
            S[f"Eye_Closed_{side}"] = self._lids(s, 1.0, 1.0)
            # (A lower lid's share is of the way to the closing line, a
            # tenth of the opening above it: past 1 it goes on up.)
            S[f"Eye_Squint_{side}"] = add(self._lids(s, 0.30, 3.4), scale(cheek_up, 0.55))
            S[f"Eye_Wide_{side}"] = self._lids(s, -0.32, -0.10)
            S[f"Cheek_Raise_{side}"] = add(cheek_up, self._lids(s, None, 2.3))
            # Brows.
            w, along = self.brow(s)
            profile = 0.75 + 0.25 * smoothstep(0.0, 0.45, along)
            S[f"Brow_Raise_{side}"] = add(self._moved(vec(w * profile, up * 0.0105), slide=whole * self.skin, teeth=False),
                                          self._lids(s, -0.10, None))
            inner = 1.0 - smoothstep(0.0, 0.55, along)
            S[f"Brow_Lower_{side}"] = add(
                self._moved(vec(w * (0.45 + 0.55 * inner), up * -0.0078) + vec(w * inner, out * -0.0036),
                            slide=whole * self.skin, teeth=False),
                self._lids(s, 0.14, None))
            outer = smoothstep(0.45, 1.0, along)
            S[f"Brow_Inward_{side}"] = self._moved(
                vec(w * inner, up * 0.0082 + out * -0.0034) + vec(w * outer, up * -0.0026),
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
            sneer = vec(nose, up * 0.0050 + back * 0.0010) + vec(ul * 0.55, up * 0.0046)
            sneer += vec(w * (1.0 - smoothstep(0.0, 0.5, along)), up * -0.0030)
            flank = self.near(self.on_skin(wing + np.array([s * 0.005, 0.0, 0.013])), 0.012) * self.skin * self.side(s)
            sneer += vec(flank, up * 0.0034) + (flank * 0.0020)[:, None] * N
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
        S["Jaw_Open"] = self.clear_with(self.jaw_open(), add(S["Mouth_Smile_Left"], S["Mouth_Smile_Right"]))
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
        """With the jaw open all the way, what the bone's turn and the
        skin's weights don't give (`rig._jaw` makes the opening an oval):
        the jaw's glide forward, the lower lip standing over the lower
        teeth, the corners of the mouth drawn in and forward round the
        opening, the cheeks hollowed a little between the jaws, and the
        lips off the teeth."""
        P = self.P
        posed = self.jaw.pose(P, JAW_OPEN_DEG)
        X = posed.copy()
        held = self.jaw.w * self.topo.is_head
        X[:, 1] -= JAW_GLIDE * held
        # The lower lip, by its loops from the fold behind it over its
        # top to the fold above the chin; nothing at the corners.
        rise = np.zeros(len(P))
        for i, k in self.mouth.lower_step.items():
            rise[i] = LIP_RISES.get(k, 0.0)
        rise *= self.corner_taper(0.8)
        X[:, 2] += JAW_LIP_UP * rise
        # (And a little back: turned with the jaw the lip leans out.)
        X[:, 1] += JAW_LIP_BACK * rise
        for _, s in SIDES:
            corner = X[self.mouth.corner[s]].copy()
            w = gauss(np.linalg.norm(X - corner, axis=1), 0.017) * (self.skin | self.lining)
            X[:, 0] -= s * JAW_CORNER_IN * w
            X[:, 1] -= JAW_CORNER_FORWARD * w
        share = self.jaw.w * (1.0 - self.jaw.w) * 4.0
        hollow = share * smoothstep(0.024, 0.04, np.abs(P[:, 0])) * self.skin * (1.0 - self.lipness)
        X -= (hollow * 0.0022)[:, None] * self.N
        return self.shape(X, open_deg=JAW_OPEN_DEG)

    def clear_with(self, shape, other, parts=(1.0, 0.6), most=0.002):
        """The jaw's shape with what it takes to keep the lips off the
        teeth when `other` (both smiles) is on as well, with the jaw all
        the way and part of the way open: a laugh is the two together.
        What is added per vertex is held to `most` (a small need part of
        the way open is not multiplied into a spike at full)."""
        P = self.P
        offsets = np.zeros_like(P)
        for i, d in shape.items():
            offsets[i] = d
        extra = np.zeros_like(P)
        for i, d in other.items():
            extra[i] = d
        frames = [{"P": P, "jaw": self.jaw, "teeth": self.teeth}] + ([self.other] if self.other else [])
        for part in parts:
            for frame in frames:
                jaw = frame["jaw"]
                posed = jaw.pose(frame["P"] + offsets * part + extra, JAW_OPEN_DEG * part)
                more = self.spread(self.off_teeth(posed, JAW_OPEN_DEG * part, teeth=frame["teeth"], jaw=jaw) - posed)
                moved = np.where(np.linalg.norm(more, axis=1) > 1e-7)[0]
                if not len(moved):
                    continue
                add_on = jaw.unpose(more[moved], JAW_OPEN_DEG * part, 0.0, moved) / part
                size = np.linalg.norm(add_on, axis=1)
                add_on *= np.minimum(1.0, most / np.maximum(size, 1e-12))[:, None]
                offsets[moved] += add_on
        return {int(i): offsets[i].copy() for i in np.where(np.linalg.norm(offsets, axis=1) > 2e-5)[0]}


    # -- identity (Character Studio's face parameters) -----------------------

    def eye_region(self, s):
        """How much each vertex belongs to eye `s`: the lids, their lining
        and the skin round them, easing off over the loops beyond."""
        eye = self.eye[s]
        w = np.zeros(len(self.P))
        for i in eye.idx:
            step = eye.step[i]
            w[i] = 1.0 if step <= 6 else max(0.0, 1.0 - (step - 6) / 6.0)
        return self.topo.smooth(w, 2, keep=[i for i in eye.idx if eye.step[i] <= 6])

    def identity(self):
        """The face identity shapes: {name: ({vertex: offset}, {bone:
        change})}. Each is the +1 end of a parameter that runs from -1 to
        1; a bone change is {"offset": [x, y, z]} (metres, Blender axes)
        or {"scale": s} at +1. Paired features have a shape a side.

        An end is meant to be seen: it moves the face 4 mm or more at its
        most (the body's simpler copies carry the shapes too, a little
        less). The eye shapes move a lid's two rims alike at every place
        along the eye, so a blink still closes on a reshaped eye."""
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
            put(f"Face_Eye_Height_{side}", vec(w, up * 0.0045), {bone: {"offset": [0.0, 0.0, 0.0045]}}, teeth=False)
            put(f"Face_Eye_Spacing_{side}", vec(w, lateral * 0.0042), {bone: {"offset": [s * 0.0042, 0.0, 0.0]}}, teeth=False)
            put(f"Face_Eye_Depth_{side}", vec(w, back * 0.0042), {bone: {"offset": [0.0, 0.0042, 0.0]}}, teeth=False)
            # Size: the eyeball and all round it grow about the eye's
            # centre. The lids' rims and lining grow in width and depth
            # only (they stay on the larger eyeball), not in the height
            # of the opening: both rims keep the travel a blink gives them.
            d = P - eye.c
            more = np.zeros(len(P))
            for i in eye.idx:
                more[i] = float(smoothstep(3.0, 6.0, eye.step[i]))
            grow = d * (w * (EYE_GROWS - 1.0 + (SKIN_GROWS - EYE_GROWS) * more))[:, None]
            rims = np.array([i for i in eye.idx if 1 <= eye.step[i] <= 3], dtype=np.int64)
            grow[rims] -= (grow[rims] @ eye.up)[:, None] * eye.up
            put(f"Face_Eye_Size_{side}", grow, {bone: {"scale": EYE_GROWS}}, teeth=False)
            # Tilt: the outer corner up, about the eye's own axis ahead.
            # (A turn made straight: each place along the eye rises or
            # falls as one, so both rims still meet in a blink.)
            angle = np.radians(13.0) * w
            along, rise = d @ eye.axis, d @ eye.up
            tilt = (-rise * angle)[:, None] * eye.axis + (along * angle)[:, None] * eye.up
            # (The rims only rise and fall: slid sideways against each
            # other they would not meet at the corners.)
            tilt[rims] = (along[rims] * angle[rims])[:, None] * eye.up
            put(f"Face_Eye_Tilt_{side}", tilt, teeth=False)
            # Brows.
            bw, along_brow = self.brow(s)
            put(f"Face_Brow_Height_{side}", vec(bw, up * 0.0052), slide=skin, teeth=False)
            # (One broad rise about the arch: with the inner end pushed
            # the other way the brow bent into a hook.)
            arch = gauss(along_brow - 0.60, 0.50)
            put(f"Face_Brow_Arch_{side}", vec(bw * arch, up * 0.0064), slide=skin, teeth=False)
            # The nose's wing, the cheek, the jaw's corner, the mouth's corner, the ear.
            wing = self.on_skin(self.nose_tip + np.array([s * 0.019, 0.016, -0.004]))
            nw = self.near(wing, 0.013) * skin * self.side(s, 0.004)
            put(f"Face_Nose_Width_{side}", vec(nw, lateral * 0.0046), teeth=False)
            bone_at = self.cheek_point(s)
            put(f"Face_Cheekbone_{side}", (self.near(bone_at, 0.024) * skin * ~self.lid * 0.0048)[:, None] * N, teeth=False)
            hollow_at = self.on_skin(np.array([s * 0.050, self.corner[s][1] + 0.022, z_mouth + 0.012]))
            put(f"Face_Cheek_Fullness_{side}", (self.near(hollow_at, 0.026) * skin * (1.0 - self.lipness) * 0.0060)[:, None] * N)
            angle_at = self.on_skin(np.array([s * 0.062, self.jaw.hinge[1] + 0.012, z_chin + 0.022]))
            jw = self.near(angle_at, 0.030) * skin * self.side(s)
            put(f"Face_Jaw_Width_{side}", vec(jw, lateral * 0.0062) + vec(jw, up * -0.0014))
            corner = self.corner_pull(s, (s * 1.0, 0.45, 0.0), 0.0048, radius=0.017, start=-0.2, power=1.3)
            put(f"Face_Mouth_Width_{side}", corner, slide=slide)
            ear = self.ear.astype(np.float64) * self.side(s, 0.001)
            root = self.ear_at[s] + np.array([-s * 0.012, -0.004, 0.0])
            # The ear eases into the head over the skin round it.
            near_ear = self.topo.smooth(ear, 3, keep=np.where(self.ear)[0])
            put(f"Face_Ear_Size_{side}", (P - root) * (near_ear * 0.16)[:, None], teeth=False)
            put(f"Face_Ear_Height_{side}", vec(near_ear, up * 0.0048), teeth=False)
            reach = np.clip((P[:, 0] * s - root[0] * s) / 0.02, 0.0, 1.5)
            put(f"Face_Ear_Out_{side}", vec(ear * reach, lateral * 0.0048 + back * -0.0016), teeth=False)
        # The middle of the face.
        mid_nose = self.near(self.nose_tip + np.array([0.0, 0.012, 0.004]), 0.024) * skin
        low_nose = smoothstep(z_nose + 0.022, z_nose - 0.006, P[:, 2])
        put("Face_Nose_Length", vec(mid_nose * low_nose, up * -0.0050 + back * -0.0014), teeth=False)
        tip = self.near(self.nose_tip, 0.014) * skin
        put("Face_Nose_Tip", vec(tip, up * 0.0038 + back * -0.0030), teeth=False)
        eye_z = self.eye[1].c[2]
        bridge_at = self.on_skin(np.array([0.0, self.nose_tip[1], 0.5 * (z_nose + eye_z) + 0.004]))
        bridge = self.near(bridge_at, 0.015) * skin * (1.0 - smoothstep(0.009, 0.022, np.abs(P[:, 0])))
        put("Face_Nose_Bridge", vec(bridge, back * -0.0046), teeth=False)
        put("Face_Nose_Tilt", vec(mid_nose * low_nose, np.array([0.0052, 0.0, 0.0])), teeth=False)
        glabella = self.on_skin(np.array([0.0, self.nose_tip[1], eye_z + 0.019]))
        ridge = gauss(P[:, 2] - glabella[2], 0.012) * (1.0 - smoothstep(0.035, 0.06, np.abs(P[:, 0]))) * skin * ~self.lid
        ridge *= P[:, 1] < glabella[1] + 0.03
        put("Face_Brow_Ridge", (ridge * 0.0046)[:, None] * N, teeth=False)
        crown = self.crown
        put("Face_Forehead_Height", vec(smoothstep(eye_z + 0.025, crown - 0.01, P[:, 2]) * self.topo.is_head, up * 0.0080), teeth=False)
        front = gauss(P[:, 2] - (eye_z + 0.055), 0.03) * smoothstep(-0.06, -0.11, P[:, 1]) * self.topo.is_head * ~self.lid
        put("Face_Forehead_Slope", vec(front, back * -0.0055), teeth=False)
        temple = gauss(P[:, 2] - (eye_z + 0.02), 0.035) * smoothstep(0.04, 0.07, np.abs(P[:, 0])) * (~self.ear) * self.topo.is_head
        put("Face_Head_Width", np.stack([np.sign(P[:, 0]) * temple * 0.0052, np.zeros(len(P)), np.zeros(len(P))], axis=1), teeth=False)
        # The chin and the jaw.
        # (The mouth's lining goes less far than the chin's skin: the
        # lower teeth stand in it.)
        chin = self.near(self.chin + np.array([0.0, 0.008, -0.004]), 0.026) * (1.0 - self.lipness)             * (self.skin + 0.15 * self.lining)
        put("Face_Chin_Length", vec(chin, up * -0.0062))
        put("Face_Chin_Forward", vec(chin, back * -0.0056))
        put("Face_Chin_Width", np.stack([P[:, 0] * chin * 0.36, np.zeros(len(P)), np.zeros(len(P))], axis=1))
        # (The mouth's lining and the lower teeth go with the jaw's skin:
        # `build` gives the teeth the same shape.)
        jaw_all = self.jaw.w * self.topo.is_head * (1.0 - self.lipness) * (self.skin | self.lining)
        put("Face_Jaw_Shift", vec(jaw_all, np.array([JAW_SHIFTS, 0.0, 0.0])), teeth=False)
        # The lips. A point of one lip and the point of the other it
        # touches move alike, so the mouth stays closed.
        red = gauss(self.d_line - 0.0055, 0.0042) * skin * self.corner_taper(0.55)
        border = gauss(self.d_line - 0.0095, 0.0045) * skin * self.corner_taper(0.55)
        # (Only the lips' outer faces: their inner sides lie on the teeth.)
        red = red * (N[:, 1] < -0.2)
        full = N * np.array([1.0, 1.0, 0.35])
        put("Face_Lip_Fullness_Upper", (red * self.upper_side * 0.0046)[:, None] * full
            + vec(border * self.upper_side, up * 0.0040))
        put("Face_Lip_Fullness_Lower", (red * self.lower_side * 0.0046)[:, None] * full
            + vec(border * self.lower_side, up * -0.0040))
        # The mouth up and down the face: the lips, the skin round them
        # and the whole of the mouth's bag, with the teeth (which carry
        # the same shape).
        mouth_all = np.maximum(gauss(self.d_line, 0.028) * self.skin, self.lining.astype(np.float64))
        put("Face_Mouth_Height", vec(mouth_all, up * MOUTH_RISES), teeth=False)
        return out


def tongue_open(points, lower_teeth):
    """The tongue's offsets at `Jaw_Open` 1 beyond the jaw's glide (its
    points at rest, in `parts._tongue`'s order: rings from the tip, then
    the tip's point; `lower_teeth` are the lower teeth's points). A
    tongue lies in the floor of the open mouth from one row of teeth to
    the other, level with their tops, and falls away toward the throat
    (the `TONGUE_*` numbers above)."""
    from parts import TONGUE_AROUND, TONGUE_ROWS
    out = points.copy()
    forward, up = TONGUE_TIP
    for row in range(TONGUE_ROWS + 1):
        # (The last "ring" is the tip's own point.)
        ring = slice(row * TONGUE_AROUND, (row + 1) * TONGUE_AROUND) if row < TONGUE_ROWS else slice(-1, None)
        R = points[ring]
        t = row / (TONGUE_ROWS - 1.0) if row < TONGUE_ROWS else 0.0
        tip = 1.0 - float(smoothstep(0.0, 0.4, t))
        out[ring, 1] = R[:, 1] - forward * tip
        if row == TONGUE_ROWS:
            out[ring, 2] = R[:, 2] + TONGUE_RISES + up
            break
        top, low, half = R[:, 2].max(), R[:, 2].min(), max(float(np.abs(R[:, 0]).max()), 1e-6)
        near = lower_teeth[np.abs(lower_teeth[:, 1] - R[:, 1].mean()) < 0.004]
        room = float(np.abs(near[:, 0]).min()) - TONGUE_ROOM if len(near) and row else half
        room = max(room, half)
        over = TONGUE_OVER * float(smoothstep(0.25, 0.6, t)) if len(near) else 0.0
        across = np.clip(np.abs(R[:, 0]) / half, 0.0, 1.0)
        high = (R[:, 2] - low) / max(top - low, 1e-6)
        sink = float(smoothstep(0.25, 1.0, t))
        z = top - (top - R[:, 2]) * (1.0 - TONGUE_THINS * sink)
        flat = TONGUE_FLAT * float(smoothstep(0.0, 0.3, t)) * np.clip(2.0 * high - 1.0, 0.0, 1.0)
        z = top - (top - z) * (1.0 - flat)
        z += TONGUE_RISES * (1.0 - sink) + up * tip - TONGUE_SINKS * sink * (1.0 - TONGUE_EDGES * across ** 2)
        z += TONGUE_DOME * (1.0 - across ** 2) * high * float(smoothstep(0.0, 0.3, t))
        # The upper side out over the teeth's inner cusps, and clear of them.
        upper = np.clip(4.0 * high - 1.0, 0.0, 1.0)
        x = R[:, 0] * (room + over * upper) / half
        if over:
            cusps = near[np.abs(near[:, 0]) < room + TONGUE_ROOM + over + 0.002]
            on_teeth = np.abs(x) > room - 0.0005
            if len(cusps):
                z = np.where(on_teeth, np.maximum(z, cusps[:, 2].max() + 0.0006), z)
        out[ring, 0], out[ring, 2] = x, z
    return out - points


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
    key = teeth.shape_key_add(name="Face_Jaw_Shift", from_mix=False)
    for i in np.nonzero(~upper)[0]:
        key.data[i].co = points[i] + np.array([JAW_SHIFTS, 0.0, 0.0])
    key.value = 0.0
    # The lower teeth glide forward with the opening jaw, and the tongue
    # settles into the floor of the mouth.
    tongue = np.unique([i for p, poly in zip(polys, me.polygons)
                        if me.materials[poly.material_index].name == "Tongue" for i in p])
    opened = points.copy()
    opened[~upper] += Jaw.glide()
    is_tooth = np.zeros(len(points), dtype=bool)
    is_tooth[np.unique([i for p, poly in zip(polys, me.polygons)
                        if me.materials[poly.material_index].name == "Teeth" for i in p])] = True
    opened[tongue] += tongue_open(points[tongue], points[is_tooth & ~upper])
    key = teeth.shape_key_add(name="Jaw_Open", from_mix=False)
    for i in np.nonzero(~upper)[0]:
        key.data[i].co = opened[i]
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
