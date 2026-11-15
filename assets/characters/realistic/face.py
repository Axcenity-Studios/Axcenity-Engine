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
# How much rougher the softened lids are than the skin round them.
LID_ROUGHER = 0.14
# How much of the masculine lids each loop round the feminine eye takes
# (by loop from the rim; the lining takes all of it).
SHARE_LIDS = {5: 0.75, 6: 0.5, 7: 0.25}


def topo_of(mesh, sets):
    return Topo([tuple(p.vertices) for p in mesh.polygons], sets, len(mesh.vertices))


# The mouth is made the same on both sides: all of it within the first of
# these of the line the lips meet on, none beyond the second (metres), and
# the whole of the mouth's lining.
MOUTH_EVEN = (0.040, 0.056)
FLIP = np.array([-1.0, 1.0, 1.0])
# The mouth's shapes that are one shape for both sides, and those that
# are a left and a right (the two of a pair are made mirror images).
EVEN_SHAPES = ("Mouth_Open", "Mouth_Wide", "Mouth_Narrow", "Mouth_Pucker", "Upper_Lip_Raise", "Lower_Lip_Lower",
               "Cheek_Puff", "Jaw_Open", "Jaw_Forward", "Face_Lip_Fullness_Upper", "Face_Lip_Fullness_Lower",
               "Face_Mouth_Height", "Face_Chin_Length", "Face_Chin_Forward", "Face_Chin_Width")
EVEN_PAIRS = (("Mouth_Smile_Left", "Mouth_Smile_Right"), ("Mouth_Frown_Left", "Mouth_Frown_Right"),
              ("Lip_Corner_Pull_Left", "Lip_Corner_Pull_Right"), ("Upper_Lip_Raise_Left", "Upper_Lip_Raise_Right"),
              ("Lower_Lip_Lower_Left", "Lower_Lip_Lower_Right"), ("Nose_Sneer_Left", "Nose_Sneer_Right"),
              ("Mouth_Left", "Mouth_Right"), ("Jaw_Left", "Jaw_Right"),
              ("Face_Mouth_Width_Left", "Face_Mouth_Width_Right"),
              ("Face_Cheek_Fullness_Left", "Face_Cheek_Fullness_Right"), ("Face_Jaw_Width_Left", "Face_Jaw_Width_Right"))


def mouth_region(topo, frames, mirror):
    """How much of each vertex is the mouth's (1 to 0; `MOUTH_EVEN`), the
    same for a vertex and its mirror image, over all `frames`' positions."""
    from facetopo import FS_MOUTH_LOWER, FS_MOUTH_UPPER
    w = np.zeros(topo.count)
    for P in frames:
        P = np.asarray(P, dtype=np.float64)
        d, _, _ = polyline_nearest(P, P[mouth.Mouth(topo, P).edge])
        w = np.maximum(w, 1.0 - smoothstep(MOUTH_EVEN[0], MOUTH_EVEN[1], d))
    w[[i for i in range(topo.count) if topo.vsets[i] and topo.vsets[i] <= {FS_MOUTH_UPPER, FS_MOUTH_LOWER}]] = 1.0
    w *= topo.is_head
    return np.maximum(w, w[mirror])


def even(X, mirror, w, other=None):
    """Positions or offsets `X` (every vertex) made mirror images of
    themselves across the body's middle, by `w` per vertex: each is moved
    toward the mean of itself and its mirror vertex's image. With
    `other`, the mirror vertex's are taken from it: a left shape against
    its right one."""
    source = X if other is None else other
    return X + w[:, None] * (0.5 * (X + source[mirror] * FLIP) - X)


def prepare(topo, M, F, eyes_m, eyes_f, radius_m, radius_f, mirror=None):
    """Both frames' positions (arrays) with the face put right at rest,
    and a report. `eyes_*` are {1: left centre, -1: right centre}. With
    `mirror` (each vertex's mirror vertex) the mouth is made even: the
    bundle's lips differ left from right by up to 0.3 mm (masculine) and
    1.3 mm (feminine), where the gap between closed lips is a tenth of a
    millimetre, so the line between them showed on one half only."""
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
    region = mouth_region(topo, (M, F), mirror) if mirror is not None else None
    for name, P in (("masculine", M), ("feminine", F)):
        if region is not None:
            P[:] = even(P, mirror, region)
        sealed, most = mouth.Mouth(topo, P).seal(P)
        P[:] = sealed
        if region is not None:
            # (The seal goes by rays and by which way each quad is cut:
            # not quite alike on the two sides.)
            P[:] = even(P, mirror, region)
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
    round_eye = np.zeros(topo.count, dtype=bool)
    rings = []
    for _, s in SIDES:
        eye = lids.Eye(topo, P, s, eyes[s], radius)
        # (To two loops above the fold: the mask's soft edge then lies
        # well outside the fold's strip, which a closed lid draws out
        # most, and outside the skin a low light grazes on a closed lid.)
        lid[[i for i in eye.idx if 1 <= eye.step[i] <= 8 and eye.upper[i]]] = True
        round_eye[[i for i in eye.idx if eye.step[i] >= 1]] = True
        rings.append(set(eye.loops[1]))
    # The skin's faces round each eye, and the opening between the lids
    # as the texture has it: no face lies there (the lining is an island
    # of its own), and what the bake left in it is darker than the lids.
    skin_faces, lid_faces, opening = [], [], [{} for _ in rings]
    for poly in body.data.polygons:
        if all(round_eye[v] for v in poly.vertices):
            uv = [tuple(layer[k].uv) for k in poly.loop_indices]
            skin_faces.append(uv)
            if all(lid[v] for v in poly.vertices):
                lid_faces.append(uv)
            for v, at in zip(poly.vertices, uv):
                for ring, found in zip(rings, opening):
                    if v in ring:
                        found[v] = at
    changed = 0
    for key in ("albedo", "normal", "orm"):
        image = maps[key]
        path = bpy_path(image)
        picture = Image.open(path)
        w, h = picture.size
        px = lambda uv: [(u * w, (1.0 - v) * h) for u, v in uv]  # noqa: E731
        mask, valid = Image.new("L", (w, h), 0), Image.new("L", (w, h), 0)
        draw, draw_valid = ImageDraw.Draw(mask), ImageDraw.Draw(valid)
        for uv in skin_faces:
            draw_valid.polygon(px(uv), fill=255)
        for uv in lid_faces:
            draw.polygon(px(uv), fill=255)
        for found in opening:
            at = np.array(list(found.values()))
            middle = at.mean(axis=0)
            order = np.argsort(np.arctan2(at[:, 1] - middle[1], at[:, 0] - middle[0]))
            draw.polygon(px(at[order]), fill=255)
            draw_valid.polygon(px(at[order]), fill=0)
        mask = mask.filter(ImageFilter.GaussianBlur(w / 400.0))
        m = np.asarray(mask, dtype=np.float32)[:, :, None] / 255.0
        src = np.asarray(picture.convert("RGB"), dtype=np.float32)
        # The lid's skin as the mean of the skin round the eye (the
        # opening's texels left out of the mean). It has to be even along
        # the lid: a closed lid's skin is a dozen texels from lash line to
        # fold, drawn out over a centimetre, and any difference between
        # neighbouring texels along the lid is a streak down it.
        weight = np.asarray(valid, dtype=np.float32)[:, :, None] / 255.0
        soft = _blurred(src * weight, w / 90.0) / np.maximum(_blurred(weight, w / 90.0), 1e-3)
        if key == "normal":
            # And nearly flat: a crease baked for the open lid is wrong
            # on the closed one.
            soft = 0.05 * soft + 0.95 * np.array([128.0, 128.0, 255.0], dtype=np.float32)
        elif key == "orm":
            # And a little rougher (the green channel): flat and as glossy
            # as the skin round it, a lowered lid caught the light as one
            # bright band.
            soft[:, :, 1] = np.minimum(soft[:, :, 1] + LID_ROUGHER * 255.0, 235.0)
        out = src * (1.0 - m) + soft * m
        changed = int((m[:, :, 0] > 0.5).sum())
        result = Image.fromarray(np.clip(out + 0.5, 0, 255).astype(np.uint8))
        if path.lower().endswith((".jpg", ".jpeg")):
            result.save(path, quality=95, subsampling=0)
        else:
            result.save(path, optimize=True)
        image.reload()
    return changed


def _blurred(a, sigma):
    """`a` (rows x columns x channels, floats) blurred to about a
    Gaussian of `sigma` texels: three box blurs along each axis."""
    half = max(1, int(round(sigma * 0.5 * (12.0 / 3.0 + 1.0) ** 0.5 - 0.5)))
    out = np.asarray(a, dtype=np.float64)
    for axis in (0, 1):
        for _ in range(3):
            pad = [(0, 0)] * out.ndim
            pad[axis] = (half + 1, half)
            total = np.cumsum(np.pad(out, pad, mode="edge"), axis=axis)
            n = out.shape[axis]
            upper = np.take(total, np.arange(2 * half + 1, 2 * half + 1 + n), axis=axis)
            lower = np.take(total, np.arange(0, n), axis=axis)
            out = (upper - lower) / (2 * half + 1)
    return out.astype(np.float32)


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
# A squint: how far its lids go toward the line they would meet on, and
# where in the opening that line lies (from the lower rim); how far a
# raised cheek pushes the lower lid toward the same line.
SQUINT = 0.60
SQUINT_MEET = 0.56
# The line is arched: this much higher at the middle of the eye than a
# line that followed the opening (metres), so a narrowed eye is a crescent,
# its lower lid pushed up most at the middle. And the upper lid comes this
# far of its way down onto it in a squint.
SQUINT_ARCH = 0.0012
SQUINT_UPPER = 0.80
CHEEK_LID = 0.40
# The brow's skin moves whole within this of the brow's line (metres), and
# each loop of the upper lid's skin takes this much of the brow's move
# (the loops from the lash line up to the fold under the brow).
BROW_WHOLE = 0.0065
BROW_ON_LID = {0: 0.0, 1: 0.0, 2: 0.0, 3: 0.08, 4: 0.20, 5: 0.42, 6: 0.50, 7: 0.80}
# How much of a raised cheek and of a lowered brow a blink has.
BLINK_CHEEK = 0.22
BLINK_BROW = 0.14
# The loop round the eye from which a brow's shapes begin to put the skin
# back on its resting surface (all the way three loops on). The loops
# under it are the fold and the brow bone's overhang: slid on the resting
# surface, a lowered brow's skin went over the edge of the bone into the
# hollow above the lid, the fold was pressed flat and its loops lay as
# thin strips turned up to the light (streaks across the upper lid).
BROW_SLIDES_FROM = 8.0
# How far a lowered brow brings the upper lid down (a share of the way to
# closed).
BROW_PRESSES = 0.22
# How much of the cheek's lift each loop of the lower lid's skin takes.
CHEEK_ON_LID = {3: 0.15, 4: 0.40, 5: 0.65, 6: 0.85, 7: 1.0}
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
JAW_LIP_UP = 0.0012
JAW_LIP_BACK = 0.0020
# How much of `JAW_LIP_UP` each loop of the lower lip takes (by loop from
# the fold behind the lip: `mouth.Mouth.lower`).
LIP_RISES = {1: 0.8, 2: 1.0, 3: 1.0, 4: 0.9, 5: 0.65, 6: 0.35, 7: 0.12}
# The mouth's corners come in round the opening by this much, and
# forward along the teeth by this much.
JAW_CORNER_IN = 0.0025
JAW_CORNER_FORWARD = 0.0025
# The upper lip is drawn up off the upper teeth as the jaw opens (its
# edge by this much, the skin above it less by loop: the lip thins), so
# the upper row shows as an arc and not under a curtain.
JAW_UPPER_LIP_UP = 0.0038
UPPER_LIP_RISES = {0: 1.0, 1: 0.8, 2: 0.5, 3: 0.25, 4: 0.1}
UPPER_LINING_RISES = {1: 1.0, 2: 0.9, 3: 0.6, 4: 0.3}
# A smile: where along a lip (0 the middle, 1 the corner) its own share
# of the corner's pull begins to give way to the skin's, and where the
# skin's has all of it (`Face.smile`).
SMILE_LIPS_OWN = (0.45, 0.95)
# The power of the way to the corner a lip's own share rises by.
SMILE_LIPS_CURVE = 2.6
# The tongue with the jaw open (metres, in the jaw's own space). Turned
# with the jaw as it rests, it is a ramp from the lower incisors up to
# the upper molars: it fills the opening and hides the back of the mouth.
# A tongue lies low in an open mouth, so toward the root it sinks by
# `TONGUE_SINKS` (most of what the jaw's turn raises it there: it lies
# nearly level), is hollowed by `TONGUE_HOLLOW` along its middle, is thinner
# from below by `TONGUE_THINS` of itself (it stays off the mouth's
# floor), and spreads to `TONGUE_ROOM` from the lower teeth.
TONGUE_SINKS = 0.006
TONGUE_HOLLOW = 0.0008
TONGUE_THINS = 0.30
TONGUE_ROOM = 0.0006


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

    def lips_apart(self, X, rest, step=0.00004, rounds=240):
        """Posed positions `X` with the lips moved off each other where a
        face of one cuts a face of the other (and didn't in `rest`, the
        frame as it rests): the upper lip's corners of the pair go up
        and the lower lip's down, a little at a time, until none does.
        A shape moves both lips by one rule made on the masculine frame;
        the feminine lips lie differently on each other, and the same
        offsets drove one through the other (163 pairs of faces in
        `Mouth_Pucker`, none on the masculine frame). (The step is small
        beside the room the lips rest with: at 0.12 mm a lip a side, the
        middle of a closed smile was left 0.3 mm further apart than the
        lips beside it, a notch in the lips' line.)"""
        from mathutils.bvhtree import BVHTree
        from facetopo import FS_HEAD, FS_MOUTH_LOWER, FS_MOUTH_UPPER, FS_MUZZLE
        near = self.d_line < 0.018
        topo = self.topo
        # (By face, so the faces at the corners count, where the two
        # lips share vertices: a pair that shares one is left alone.)
        of = [fs for p, fs in zip(topo.polys, topo.sets) for _ in range(len(p) - 2)]

        def faces(sets):
            return [tuple(int(i) for i in t) for t, fs in zip(topo.tris, of)
                    if fs in sets and near[t[0]] and near[t[1]] and near[t[2]]]

        upper, lower = faces((FS_HEAD, FS_MOUTH_UPPER)), faces((FS_MUZZLE, FS_MOUTH_LOWER))

        def crossing(Q):
            pts = [tuple(q) for q in Q]
            a = BVHTree.FromPolygons(pts, upper, all_triangles=True)
            b = BVHTree.FromPolygons(pts, lower, all_triangles=True)
            return {(i, j) for i, j in a.overlap(b) if not set(upper[i]) & set(lower[j])}

        resting = crossing(rest)
        out = X.copy()
        last = None
        for _ in range(rounds + 1):
            pairs = crossing(out) - resting
            if not pairs:
                # (Once more for the last that crossed: off each other by
                # a step, not by nothing.)
                pairs, last = last, None
                if not pairs:
                    break
            else:
                last = pairs
            out[sorted({v for i, _ in pairs for v in upper[i]}), 2] += step
            out[sorted({v for _, j in pairs for v in lower[j]}), 2] -= step
        return out

    def lips_together(self, X, rest, reach=0.0015):
        """Posed positions `X` with the lower lip's top raised back to the
        upper lip where they stand further apart than in `rest` (the
        frame as it rests), by up to `reach`: a shape that means the
        mouth to stay closed (a smile with the lips together) parts the
        lips a hair where they are kept off the teeth, and the teeth
        show through as a pale line. The loops beside the lip's crest
        rise with it by `mouth.SEAL`'s shares."""
        from mathutils import Vector
        from facetopo import FS_HEAD, FS_MOUTH_UPPER
        topo, lips = self.topo, self.mouth
        over = {k for k, fs in enumerate(topo.sets) if fs in (FS_HEAD, FS_MOUTH_UPPER)}
        now, before = topo.bvh(X, over), topo.bvh(rest, over)
        up = Vector((0.0, 0.0, 1.0))
        xs, rises = [], []
        for i in lips.lower[mouth.CREST]:
            hit, hit0 = now.ray_cast(Vector(X[i]), up, 0.01), before.ray_cast(Vector(rest[i]), up, 0.01)
            if hit[0] is None or hit0[0] is None:
                continue
            more = float(hit[3]) - max(float(hit0[3]), mouth.TOUCH)
            xs.append(float(rest[i][0]))
            rises.append(more if 0.0 < more < reach else 0.0)
        out = X.copy()
        if not xs or max(rises) <= 0.0:
            return out
        order = np.argsort(xs)
        xs, rises = np.array(xs)[order], np.array(rises)[order]
        for i, k in lips.lower_step.items():
            share = mouth.SEAL.get(k, 0.0)
            if share:
                out[i, 2] += share * float(np.interp(rest[i][0], xs, rises, left=0.0, right=0.0))
        return out

    def evened(self, shapes, mirror, w):
        """`shapes` ({name: {vertex: offset}}) with the mouth's shapes made
        even (`EVEN_SHAPES`, `EVEN_PAIRS`; `mirror` is each vertex's
        mirror vertex, `w` how much of each is the mouth's): a shape is
        made by rays and nearest points on a face that is not quite the
        same on both sides, and comes out a little different on each.
        The mean of two halves can put the lips through each other where
        neither half did, so they are taken off each other again, alone
        and for a pair together, and the result made even once more."""
        count = len(self.P)
        frames = [self.P] + ([self.other["P"]] if self.other else [])

        def dense(shape):
            out = np.zeros((count, 3))
            for i, d in shape.items():
                out[i] = d
            return out

        def sparse(D):
            keep = np.where((np.linalg.norm(D, axis=1) > 2e-5) & self.topo.is_head)[0]
            return {int(i): D[i].copy() for i in keep}

        def apart(D):
            for rest in frames:
                D = D + self.lips_apart(rest + D, rest) - (rest + D)
            return D

        out = dict(shapes)
        for name in EVEN_SHAPES:
            if name in out:
                D = even(dense(out[name]), mirror, w)
                if not name.startswith("Jaw"):
                    D = even(apart(D), mirror, w)
                out[name] = sparse(D)
        for left, right in EVEN_PAIRS:
            if left in out and right in out:
                L, R = dense(out[left]), dense(out[right])
                L, R = even(L, mirror, w, R), even(R, mirror, w, L)
                if not left.startswith("Jaw"):
                    L, R = apart(L), apart(R)
                    more = apart(L + R) - (L + R)
                    L, R = L + more * self.side(1)[:, None], R + more * self.side(-1)[:, None]
                    L, R = even(L, mirror, w, R), even(R, mirror, w, L)
                out[left], out[right] = sparse(L), sparse(R)
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

    def shape(self, X, open_deg=0.0, side_deg=0.0, slide=None, teeth=True, shift=None, bag=False, sweep=(1.0,),
              sealed=False):
        """The shape that takes the resting face to positions `X` (every
        vertex; with the jaw turned, `X` is where the *posed* skin should
        be). Skin is slid on the resting surface by `slide`, lips are
        kept off the teeth and off each other in both frames, and with
        `sealed` brought back together. Returns {vertex: offset}."""
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
                    # The lips off each other first, then off the teeth.
                    posed_f = jaw.pose(frame["P"] + offsets, open_deg, side_deg)
                    more = self.lips_apart(posed_f, frame["P"]) - posed_f
                    moved = np.where(np.linalg.norm(more, axis=1) > 1e-7)[0]
                    if len(moved):
                        offsets[moved] += jaw.unpose(more[moved], open_deg, side_deg, moved)
                    posed_f = jaw.pose(frame["P"] + offsets, open_deg, side_deg)
                    fixed = self.off_teeth(posed_f, open_deg, side_deg, teeth=frame["teeth"], jaw=jaw, shift=shift,
                                           bag=bag, sweep=sweep)
                    more = self.spread(fixed - posed_f)
                    moved = np.where(np.linalg.norm(more, axis=1) > 1e-7)[0]
                    if len(moved):
                        offsets[moved] += jaw.unpose(more[moved], open_deg, side_deg, moved)
            # A shape that keeps the mouth closed (`sealed`): the lips
            # back together where keeping them off the teeth parted them,
            # and off each other again.
            if sealed and not open_deg:
                for frame in frames:
                    jaw = frame["jaw"]
                    posed_f = jaw.pose(frame["P"] + offsets, open_deg, side_deg)
                    more = self.lips_together(posed_f, frame["P"])
                    more = self.lips_apart(more, frame["P"]) - posed_f
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

    def _lids(self, s, upper=None, lower=None, meet=lids.MEET, ease=False, arch=0.0):
        return lids.deltas(self.P, self.eye[s].lid(self.P, upper_to=upper, lower_to=lower, meet=meet, ease=ease,
                                                   arch=arch))

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
        # (By a power of the way to the corner, with a little that passes
        # from one side to the other across the middle: the two sides'
        # shares of that add up to the same from side to side, so
        # together they leave the lips' line one curve, level in the
        # middle. A share that rose faster on its own side than it fell
        # on the other left the middle vertex 0.4 mm under its
        # neighbours, and by the square of the way the lower lip, whose
        # vertices are 7 mm apart, rose 0.4 mm over its first one: a
        # point in the middle again, at twice the size.)
        own = (0.85 * np.clip(along, 0.0, 1.0) ** SMILE_LIPS_CURVE
               + 0.075 * (1.0 + np.sin(np.clip(along / 0.6, -1.0, 1.0) * np.pi / 2)))
        round_corner = gauss(d, 0.021) * self.side(s)
        # The two are blended along the lip: a lip goes by its own share
        # from the middle out to `SMILE_LIPS_OWN`'s first place and by the
        # skin's from the second to the corner, where lip and skin are one.
        # (The larger of the two was taken before, and over most of the lip
        # that is the skin's, which rises evenly from the middle: the
        # lips' line was two straight halves that met in a point. The
        # lips' own share all the way to the corner left them behind the
        # skin beside them there, and the teeth came through the fold.)
        theirs = self.lipness * (1.0 - smoothstep(SMILE_LIPS_OWN[0], SMILE_LIPS_OWN[1], along))
        w = round_corner + (own - round_corner) * theirs
        w[self.lid | self.socket] = 0.0
        move = w[:, None] * pull * (0.0128 * amount)
        # The upper lip lifts a little with it.
        # The cheek, pushed up and out ahead of the corner.
        bulge_at = self.on_skin(c + pull * 0.027 + np.array([0.0, 0.0, 0.004]))
        cheek = self.near(bulge_at, 0.019) * self.side(s) * self.skin * ~self.lid
        move += (cheek * 0.0072 * amount)[:, None] * self.N + (cheek * 0.0062 * amount)[:, None] * np.array([0.0, 0.0, 1.0])
        # The fold between the nose's wing and the corner sinks.
        wing = self.on_skin(self.nose_tip + np.array([s * 0.021, 0.018, -0.006]))
        low = self.on_skin(c + np.array([s * 0.009, -0.004, 0.004]))
        knee = self.on_skin(0.5 * (wing + low) + np.array([s * 0.004, 0.0, 0.0]))
        fold_d, _, _ = polyline_nearest(P, np.array([wing, knee, low]))
        move -= (gauss(fold_d, 0.0045) * self.skin * (1.0 - self.lipness) * 0.0042 * amount)[:, None] * self.N
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

    def brow_curve(self, s, count=24, start=0.0, end=1.0):
        """`count` + 1 points of the brow's line (through the three of
        `brow_line`), from `start` to `end` of the way along it (0 the
        inner end, 1 the tail; the brows' card runs a little past both)."""
        a, b, c = self.brow_line(s)
        ctrl = 2.0 * b - 0.5 * (a + c)        # the curve passes through the arch
        t = np.linspace(start, end, count + 1)[:, None]
        return (1 - t) ** 2 * a + 2 * (1 - t) * t * ctrl + t ** 2 * c

    def brow(self, s):
        """(weight, how far along the brow from its inner end 0 to its
        tail 1): the brow's skin. All of it under the brow's hair
        (`BROW_WHOLE` either side of the brow's line), so the hair's card
        rides the skin without being stretched; easing off further up the
        forehead than down toward the eye, where the fold over the upper
        lid takes less of it loop by loop and the lid's rim none."""
        d, on, along = polyline_nearest(self.P, self.brow_curve(s))
        above = smoothstep(-0.004, 0.004, self.P[:, 2] - on[:, 2])
        past = np.maximum(d / (1.0 + 0.6 * above) - BROW_WHOLE, 0.0)
        w = gauss(past, 0.012) * self.skin * self.side(s, 0.006)
        eye = self.eye[s]
        for i in eye.idx:
            if eye.upper[i] and eye.step[i] in BROW_ON_LID:
                w[i] *= BROW_ON_LID[eye.step[i]]
        lower = np.array([i for i in eye.idx if not eye.upper[i] and eye.step[i] <= 4], dtype=np.int64)
        w[lower] = 0.0
        return w, along

    def brow_slide(self):
        """How much the brows' shapes put the skin back on its resting
        surface: all the way over the skull, not at all on the lids and
        the fold over them (the nearest point of a fold is across it)."""
        amount = self.skin.astype(np.float64)
        for eye in self.eye.values():
            for i in eye.idx:
                amount[i] = min(amount[i], float(smoothstep(BROW_SLIDES_FROM, BROW_SLIDES_FROM + 3.0, eye.step[i])))
        return amount

    def glabella(self):
        """The skin between the brows' inner ends, as a weight."""
        ends = [self.brow_line(s)[0] for _, s in SIDES]
        at = self.on_skin(0.5 * (ends[0] + ends[1]) + np.array([0.0, -0.01, 0.0]))
        return self.near(at, 0.009) * self.skin

    def cheek(self, s):
        """The cheek under the eye (the skin a squint or a smile lifts)."""
        at = self.on_skin(self.eye[s].c + np.array([s * 0.010, -0.012, -0.026]))
        w = self.near(at, 0.019) * self.skin * self.side(s)
        w[self.lid] = 0.0
        # The lower lid's skin takes more of it loop by loop from the
        # lash line down (the lid's own turn moves the rim): cut off at
        # the lid's border, the cheek rose as a shelf under the eye.
        eye = self.eye[s]
        for i in eye.idx:
            if not eye.upper[i] and eye.step[i] in CHEEK_ON_LID:
                w[i] = float(gauss(np.linalg.norm(self.P[i] - at), 0.019)) * CHEEK_ON_LID[eye.step[i]]
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
            # A squint's lids go part of the way to a line of their own
            # just above the middle of the opening: the lower lid rises a
            # third of the opening, the upper comes down a quarter. A raised
            # cheek pushes the lower lid a quarter of the opening up. The
            # two together (a laugh) bring the lower lid to that line, and
            # each has its share of that one straight path: shapes add in
            # straight lines, and two arcs' own chords added together
            # carried the lid 3 mm forward off the eyeball, its skin
            # twisted at the inner corner.
            narrowed = self._lids(s, None, 1.0, SQUINT_MEET, ease=True, arch=SQUINT_ARCH)
            S[f"Eye_Squint_{side}"] = add(self._lids(s, SQUINT_UPPER, None, SQUINT_MEET, ease=True, arch=SQUINT_ARCH),
                                          scale(narrowed, SQUINT),
                                          scale(cheek_up, 0.55))
            # (Wide: the upper lid's edge clears the top of the iris by
            # about 2 mm of white.)
            S[f"Eye_Wide_{side}"] = self._lids(s, -0.50, -0.10)
            S[f"Cheek_Raise_{side}"] = add(cheek_up, scale(narrowed, CHEEK_LID))
            # Brows.
            w, along = self.brow(s)
            # (The skin between the brows bunches forward as they come
            # down or together; it stays off its resting surface there.)
            bunch = self.glabella() * self.side(s, 0.006)
            on_skull = self.brow_slide() * (1.0 - np.clip(bunch * 2.0, 0.0, 1.0))
            # Raised (frontalis): the whole brow, its inner end a little
            # less; the fold under it is drawn up and the lid follows.
            profile = 0.75 + 0.25 * smoothstep(0.0, 0.45, along)
            S[f"Brow_Raise_{side}"] = add(self._moved(vec(w * profile, up * 0.0105), slide=on_skull, teeth=False),
                                          self._lids(s, -0.12, None))
            # Lowered (the depressors): the inner end down and toward the
            # other, the tail less; the fold comes down on the upper lid,
            # which narrows the eye from above.
            inner = 1.0 - smoothstep(0.0, 0.55, along)
            # (No vertex is drawn more than part of its way to the middle
            # of the face: the two sides' skin met there and folded over
            # in a hairline crease.)
            together = lambda reach: vec(np.minimum(w * inner * reach, 0.4 * np.abs(P[:, 0])), out * -1.0)  # noqa: E731
            lowered = self._moved(vec(w * (0.45 + 0.55 * inner), up * -0.0078) + together(0.0036)
                                  + (bunch * 0.0016)[:, None] * N, slide=on_skull, teeth=False)
            S[f"Brow_Lower_{side}"] = add(lowered, self._lids(s, BROW_PRESSES, None))
            # The muscle that shuts an eye also draws the cheek up and the
            # brow down a little: one eye shut alone is a wink, not a lid
            # dropped on a still face. (`Eye_Closed`, an eye at rest, has
            # the lids alone.)
            S[f"Blink_{side}"] = add(S[f"Blink_{side}"], scale(cheek_up, BLINK_CHEEK), scale(lowered, BLINK_BROW))
            # The inner end in and up, as in worry (the control's meaning in
            # the engine, and what Sad, Worried, Fearful and Crying use it
            # for): the inner third rises and comes toward the middle, the
            # rest of the brow stays nearly where it was (the brow bends
            # there; raised over its whole inner half and dipped at the
            # tail it tilted as one straight bar), and the skin between
            # bunches forward.
            kink = 1.0 - smoothstep(0.0, 0.42, along)
            outer = smoothstep(0.45, 1.0, along)
            S[f"Brow_Inward_{side}"] = self._moved(
                vec(w * kink, up * 0.0082) + together(0.0034) + vec(w * outer, up * -0.0010)
                + (bunch * 0.0015)[:, None] * N, slide=on_skull, teeth=False)
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
            # (These keep the mouth closed; the lip and nose shapes part it.)
            sealed = name in ("Mouth_Smile", "Mouth_Frown", "Lip_Corner_Pull")
            both = np.zeros_like(P)
            for i, d in self._moved(sides[1][0] + sides[-1][0], slide=np.maximum(sides[1][1], sides[-1][1]),
                                    sealed=sealed).items():
                both[i] = d
            for side, s in SIDES:
                share = self.side(s)
                # (Kept off the teeth once more alone: one side's share of
                # the middle is less than the whole it was made as.)
                S[f"{name}_{side}"] = self._moved(both * share[:, None], sealed=sealed)
            if name in ("Upper_Lip_Raise", "Lower_Lip_Lower"):
                S[name] = {int(i): both[i].copy() for i in np.where(np.linalg.norm(both, axis=1) > 2e-5)[0]}
        ul, ll = self.upper_lip(), self.lower_lip()
        S["Mouth_Open"] = self._moved(vec(ul, up * 0.0020) + vec(ll, up * -0.0058 + back * -0.0006), slide=slide * 0.6)
        wide = sum(self.corner_pull(s, (s * 1.0, 0.5, -0.12), 0.0066, radius=0.019, start=-0.6, power=1.2) for _, s in SIDES)
        S["Mouth_Wide"] = self._moved(wide, slide=slide, sealed=True)
        # Lips drawn together: toward the middle, and forward as they
        # come round the teeth's arch.
        lips = gauss(self.d_line, 0.019) * (self.skin | self.lining)
        gather = lambda squeeze, forward, corners: (  # noqa: E731
            np.stack([-P[:, 0] * squeeze * lips, -(forward + corners * np.abs(self.u)) * lips, np.zeros(len(P))], axis=1))
        narrow = gather(0.20, 0.0032, 0.0042) + vec(ul, up * 0.0012) + vec(ll, up * -0.0020)
        S["Mouth_Narrow"] = self._moved(narrow, slide=None)
        pucker = gather(0.31, 0.0054, 0.0056) + (lips * self.skin * 0.0009)[:, None] * N
        S["Mouth_Pucker"] = self._moved(pucker, slide=None, sealed=True)
        for side, s in SIDES:
            mouth_w = gauss(self.d_line, 0.028) * (self.skin | self.lining)
            shift = np.stack([s * 0.0062 * mouth_w, 0.0030 * mouth_w * self.u * s, np.zeros(len(P))], axis=1)
            S[f"Mouth_{side}"] = self._moved(shift, slide=slide * 0.7, sealed=True)
        puff = np.zeros_like(P)
        for _, s in SIDES:
            at = self.on_skin(np.array([s * 0.047, self.corner[s][1] + 0.028, self.corner[s][2] - 0.001]))
            w = self.near(at, 0.026) * (self.skin | self.lining) * self.side(s)
            way = P - np.array([0.0, at[1] + 0.02, at[2]])
            way[:, 2] *= 0.3
            way /= np.maximum(np.linalg.norm(way, axis=1, keepdims=True), 1e-9)
            puff += (w * 0.0095)[:, None] * way
        puff += vec(gauss(self.d_line, 0.016) * self.skin, back * -0.0016)
        S["Cheek_Puff"] = self._moved(puff, slide=None, sealed=True)
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
        teeth, the upper lip drawn up off the upper ones, the corners of
        the mouth drawn in and forward round the opening, the cheeks
        hollowed a little between the jaws, and the lips off the teeth."""
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
        lift = np.zeros(len(P))
        for shares, loops in ((UPPER_LIP_RISES, self.mouth.upper), (UPPER_LINING_RISES, self.mouth.under)):
            for k, share in shares.items():
                if k < len(loops):
                    lift[loops[k]] = share
        X[:, 2] += JAW_UPPER_LIP_UP * lift * self.corner_taper(0.6)
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
    the tip's point; `lower_teeth` are the lower teeth's points): low
    and level behind the lower teeth, hollow along its middle, with the
    back of the mouth open over it (the `TONGUE_*` numbers above)."""
    from parts import TONGUE_AROUND, TONGUE_ROWS
    out = points.copy()
    for row in range(1, TONGUE_ROWS):
        ring = slice(row * TONGUE_AROUND, (row + 1) * TONGUE_AROUND)
        R = points[ring]
        t = row / (TONGUE_ROWS - 1.0)
        top, low, half = R[:, 2].max(), R[:, 2].min(), max(float(np.abs(R[:, 0]).max()), 1e-6)
        near = lower_teeth[np.abs(lower_teeth[:, 1] - R[:, 1].mean()) < 0.004]
        room = max(float(np.abs(near[:, 0]).min()) - TONGUE_ROOM, half) if len(near) else half
        across = np.clip(np.abs(R[:, 0]) / half, 0.0, 1.0)
        high = (R[:, 2] - low) / max(top - low, 1e-6)
        z = top - (top - R[:, 2]) * (1.0 - TONGUE_THINS * t) - TONGUE_SINKS * t
        z -= TONGUE_HOLLOW * (1.0 - across ** 2) * high * float(smoothstep(0.1, 0.5, t))
        out[ring, 0], out[ring, 2] = R[:, 0] * (room / half), z
    return out - points


def _teeth_follow(teeth, lift):
    """The shapes a `Teeth` object follows the body's with: the mouth's
    height and the jaw's shift (identity), and `Jaw_Open`: the lower
    teeth glide forward with the opening jaw, and the tongue settles
    into the floor of the mouth."""
    me = teeth.data
    points = np.array([v.co[:] for v in me.vertices])
    polys = [tuple(p.vertices) for p in me.polygons]
    head_group = teeth.vertex_groups["Head"].index
    upper = np.array([any(g.group == head_group for g in v.groups) for v in me.vertices])
    key = teeth.shape_key_add(name="Face_Mouth_Height", from_mix=False)
    for i, p in enumerate(points):
        key.data[i].co = p + lift
    key.value = 0.0
    key = teeth.shape_key_add(name="Face_Jaw_Shift", from_mix=False)
    for i in np.nonzero(~upper)[0]:
        key.data[i].co = points[i] + np.array([JAW_SHIFTS, 0.0, 0.0])
    key.value = 0.0
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


# The lips' wet inner faces lie on each other over some millimetres of
# depth in front of the teeth, so a mouth parted by a hair shows dark
# between the lips, not enamel. The body's lips meet along a line. `LipSeal`
# is that depth: a flap hanging this far (metres) from the upper lip's
# underside and one standing this far up from the lower lip's top, each
# riding its own lip a few millimetres behind its edge, behind the other
# lip while the mouth is closed. (Seen from the front it is the upper
# flap that closes the parting: the lower one stands behind the lower
# lip's crest, and is what a view from above sees.)
SEAL_HANGS = 0.0022
SEAL_STANDS = 0.0014
# A flap stands out of its lip along the lip's own middle (`_flesh_axis`),
# and the controls that part the lips draw it back along that line into
# the lip: control -> how many times a flap's height it is drawn back at
# full weight. A shape moves a vertex in a straight line by its weight,
# so a flap cannot stop when it is gone: past its own height it goes on
# into the flesh of its lip, where nothing sees it. The numbers are what
# that leaves room for. A flap must still cover the teeth with the lips
# 1.5 mm apart and be gone soon after, and it shrinks evenly from the
# first, so the lip controls have it gone by 3 mm of parting
# (`Mouth_Open` parts the lips 7.8 mm at full weight: 2.6 times; and so
# on) and the jaw, which parts them 37 mm, by 4.6 mm: sooner would take
# a flap further into the face at full opening than a lip is deep (a
# laugh takes the upper one 15 mm in as it is). (The left and right
# controls take their own side's.)
SEAL_OPENS = {"Jaw_Open": 8.0, "Mouth_Open": 2.6, "Upper_Lip_Raise": 1.55, "Lower_Lip_Lower": 1.75, "Nose_Sneer": 0.85,
              "Mouth_Narrow": 1.15}
# Where in the gums' palette the flaps are: the lining's red at the
# shade the lining has just inside the lips (`skin.INNER_LIP`), so a flap
# that shows beside an opening mouth is the lip's own inner side and not
# a black line round it.
SEAL_PLACE = (1.0, 0.55)
# A flap's root is this far inside its lip (metres).
SEAL_ROOT = 0.0006


def _flesh_axis(face, loop, skin_loop, lining_loop, lean):
    """For each vertex of `loop` (a lip's inner side just behind its
    edge), the direction from it into the middle of the lip's flesh: at
    a point between the lip's skin (`skin_loop`) and its lining
    (`lining_loop`) some way from the edge, `lean` of the way from the
    skin to the lining. {vertex: unit vector}."""
    P = face.P

    def along(ring):
        ring = sorted((int(i) for i in ring), key=lambda i: P[i][0])
        xs = np.array([P[i][0] for i in ring])
        return lambda x: np.array([x, np.interp(x, xs, [P[i][1] for i in ring]), np.interp(x, xs, [P[i][2] for i in ring])])

    skin, lining = along(skin_loop), along(lining_loop)
    out = {}
    for i in loop:
        x = float(P[i][0])
        to = skin(x) * (1.0 - lean) + lining(x) * lean - P[i]
        to[0] = 0.0
        out[int(i)] = to / np.linalg.norm(to)
    return out


def lip_seal(face, F, shapes, jaw_w):
    """The `LipSeal` object (the notes above): two strips bound vertex by
    vertex to the loop behind each lip's edge, with every shape of the
    body's that moves those vertices, `Frame_Feminine`, and the jaw's
    hold on them."""
    import bpy
    import teeth as jawlib
    from common import new_object, shade_smooth
    M, lips = face.P, face.mouth
    reach = 0.95 * lips.half
    points, bound, free, faces = [], [], [], []
    upper, lower = lips.under[1], lips.lower[mouth.CREST - 1]
    flaps = ((upper, _flesh_axis(face, upper, lips.upper[3], lips.under[4], 0.45), SEAL_HANGS),
             (lower, _flesh_axis(face, lower, lips.lower[7], lips.floor, 0.6), SEAL_STANDS))
    for flap, (loop, axis, tall) in enumerate(flaps):
        chain = [int(i) for i in sorted(loop, key=lambda i: M[i][0]) if abs(M[i][0]) < reach]
        first = len(points)
        for i in chain:
            taper = max(0.0, 1.0 - (abs(M[i][0]) / reach) ** 4)
            for is_free in (False, True):
                out = -tall * taper if is_free else SEAL_ROOT
                points.append(axis[i] * out)
                bound.append(i)
                # (What draws a free vertex back: its flap's height along the axis.)
                free.append(axis[i] * tall * taper if is_free else None)
        for k in range(len(chain) - 1):
            a = first + 2 * k
            # (Both flaps face the front: seen from its back, a flap is lit from behind.)
            faces.append((a, a + 1, a + 3, a + 2) if flap == 0 else (a, a + 2, a + 3, a + 1))
    bound = np.array(bound, dtype=np.int64)
    points = np.array(points)
    mesh = bpy.data.meshes.new("LipSeal")
    mesh.from_pydata([tuple(p) for p in M[bound] + points], [], faces)
    mesh.update()
    seal = new_object("LipSeal", mesh)
    material = bpy.data.materials.get("LipSeal")
    if material is None:
        material = bpy.data.materials["Gums"].copy()
        material.name = "LipSeal"
        material.node_tree.nodes["Principled BSDF"].inputs["Roughness"].default_value = 0.8
    mesh.materials.append(material)
    layer = mesh.uv_layers.new(name="UVMap")
    uv = jawlib.place(np.array([SEAL_PLACE[0]]), np.array([SEAL_PLACE[1]]))[0]
    for loop in mesh.loops:
        layer.data[loop.index].uv = uv
    for name, share in (("Head", 1.0 - jaw_w[bound]), ("Jaw", jaw_w[bound])):
        group = seal.vertex_groups.new(name=name)
        for k, w in enumerate(share):
            if w > 1e-4:
                group.add([k], float(w), "REPLACE")
    # The shapes: each vertex takes its lip vertex's move, and the free
    # edge of a flap is drawn back by the controls that part the lips.
    rest = M[bound] + points
    seal.shape_key_add(name="Basis", from_mix=False)
    key = seal.shape_key_add(name="Frame_Feminine", from_mix=False)
    for k, p in enumerate(F[bound] + points):
        key.data[k].co = p
    key.value = 0.0
    side = {"Left": face.side(1)[bound], "Right": face.side(-1)[bound]}
    for name, shape in shapes.items():
        offsets = np.array([shape.get(int(i), np.zeros(3)) for i in bound])
        base, part = name, None
        for tag in ("Left", "Right"):
            if name.endswith("_" + tag):
                base, part = name[:-len(tag) - 1], side[tag]
        if base in SEAL_OPENS:
            for k, back in enumerate(free):
                if back is not None:
                    offsets[k] += back * SEAL_OPENS[base] * (1.0 if part is None else part[k])
        if np.abs(offsets).max() < 2e-5:
            continue
        key = seal.shape_key_add(name=name, from_mix=False)
        for k, d in enumerate(offsets):
            key.data[k].co = rest[k] + d
        key.value = 0.0
    shade_smooth(seal)
    return seal


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


def build(body, topo, M, F, eyes_m, eyes_f, radius, joints, joints_f, weights, teeth, out_dir, teeth_copies=(),
          mirror=None):
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
    if mirror is not None:
        # The mouth's shapes even, as its resting form is (`prepare`).
        shapes = face.evened(shapes, mirror, mouth_region(topo, (M, F), mirror))
        expressions = {name: shapes[name] for name in expressions}
        identity = {name: (shapes[name], changes) for name, (_, changes) in identity.items()}
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
    at = np.array([v.co[:] for v in brow.data.vertices])
    quads = [tuple(p.vertices) for p in brow.data.polygons]
    hair.add_shapes(brow, {name: hair.follow_bound(face, bound, at, quads, following[name]) for name in order})
    made["Brows"] = brow
    made["LipSeal"] = lip_seal(face, F, shapes, jaw_w)
    # The teeth go where the mouth goes (and their simpler copies alike).
    lift = identity["Face_Mouth_Height"][0][face.mouth.mid]
    for part in (teeth, *teeth_copies):
        _teeth_follow(part, lift)
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
