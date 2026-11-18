"""The skin's makeup and marking masks (Character Studio R5, ADR-0042):
three RGBA images at the size of the skin's maps, placed like its albedo,
one layer to a channel, which the engine's Skin shading model draws by
the material values a description's `skin` block turns on. No texture is
made per character: a colour, an opacity or a seed is a value, so a
change re-applies live.

- `RealisticSkin_MakeupA.png`: foundation, blush, eyeshadow, eyeliner.
- `RealisticSkin_MakeupB.png`: lip colour, brow fill, a scar, stubble
  (R6: the spare channel, ADR-0042).
- `RealisticSkin_Markings.png`: freckles, the freckles' ranks, moles,
  the moles' ranks.

Every mask is worked out per texel from where the texel lies on the
body at rest (a position map rasterised from the texture coordinates),
so a mark is measured in millimetres on the skin, not in texels, and a
spot crosses no seam it shouldn't. The makeup masks follow the face's
landmarks (`face.Face`: the lids' rims, the brows' line, the cheeks,
the mouth); the lips are the albedo's own lip field (`skin.fields`), so
lip colour covers exactly the lips the skin shows.

Freckles and moles are **seeded spots**: candidates placed once by the
build over a density field (freckles where the sun falls: the nose and
cheeks, the shoulders, the upper back and chest, the forearms; moles
anywhere, more on the face and back), each with a soft disc of coverage
and a **rank** (0 to 1, the same over the whole spot). The engine shows
the spots whose rank, offset by the description's seed, is below its
amount: another seed shows other spots, a larger amount more of them,
all live. Ranks are spread a few texels past each disc so filtering
never mixes two spots' ranks at an edge that shows.

`mark` adds the three images to the exported file and names them in
the `Skin` material's `extras` (`axcenity_skin_masks`).
"""

import os

import numpy as np

from common import FS_HEAD, FS_MUZZLE, FS_SOCKET, FS_TORSO, SIDES

FILES = ("RealisticSkin_MakeupA.png", "RealisticSkin_MakeupB.png", "RealisticSkin_Markings.png")
# How far (texels) each mask is grown past its island's edge, so filtering
# and smaller levels at a seam read the layer, not the empty background.
MARGIN = 6
# Freckles: candidates per square centimetre where the sun field is 1,
# and their radius range (metres). Freckles are under 3 mm across
# (references/skin, DermNet).
FRECKLE_DENSITY = 9.0
FRECKLE_RADIUS = (0.0005, 0.00125)
# Moles: how many candidates over the body (and how many of them on the
# face), and their radius range (metres): 2 to 6 mm across.
MOLES = 70
FACE_MOLES = 9
MOLE_RADIUS = (0.0011, 0.0026)
SEED = 4207


def _step(a, b, x):
    t = np.clip((np.asarray(x, dtype=np.float64) - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


# -- the position map -----------------------------------------------------------------

class Texels:
    """The body's texels at `size`: for each texel inside an island, the
    triangle it is on, its barycentric weights and its rest position;
    `tris` are the mesh's triangles (vertex indices), `tri_set` each
    triangle's face set."""

    def __init__(self, body, topo, P, size):
        me = body.data
        n_loops = len(me.loops)
        uv = np.empty(n_loops * 2, dtype=np.float64)
        me.uv_layers[0].data.foreach_get("uv", uv)
        uv = uv.reshape(-1, 2)
        loop_vert = np.empty(n_loops, dtype=np.int64)
        me.loops.foreach_get("vertex_index", loop_vert)
        tris, tri_uv, tri_set = [], [], []
        for poly, fs in zip(me.polygons, topo.sets):
            loops = list(poly.loop_indices)
            for k in range(1, len(loops) - 1):
                corner = [loops[0], loops[k], loops[k + 1]]
                tris.append(loop_vert[corner])
                tri_uv.append(uv[corner])
                tri_set.append(fs)
        self.size = size
        self.tris = np.array(tris, dtype=np.int64)
        # Pixel coordinates, rows from the top as images have them.
        self.tri_px = np.array(tri_uv) * size
        self.tri_px[:, :, 1] = size - self.tri_px[:, :, 1]
        self.tri_set = np.array(tri_set)
        self.P = np.asarray(P, dtype=np.float64)
        self.tri_of = np.full((size, size), -1, dtype=np.int64)
        self.bary = np.zeros((size, size, 3), dtype=np.float64)
        for t, px in enumerate(self.tri_px):
            x0, y0 = np.floor(px.min(axis=0)).astype(int)
            x1, y1 = np.ceil(px.max(axis=0)).astype(int) + 1
            x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, size), min(y1, size)
            if x1 <= x0 or y1 <= y0:
                continue
            gx, gy = np.meshgrid(np.arange(x0, x1) + 0.5, np.arange(y0, y1) + 0.5)
            (ax, ay), (bx, by), (cx, cy) = px
            det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
            if abs(det) < 1e-12:
                continue
            wa = ((by - cy) * (gx - cx) + (cx - bx) * (gy - cy)) / det
            wb = ((cy - ay) * (gx - cx) + (ax - cx) * (gy - cy)) / det
            wc = 1.0 - wa - wb
            inside = (wa >= -1e-6) & (wb >= -1e-6) & (wc >= -1e-6)
            patch = self.tri_of[y0:y1, x0:x1]
            patch[inside] = t
            self.bary[y0:y1, x0:x1][inside] = np.stack([wa, wb, wc], axis=-1)[inside]
        self.covered = self.tri_of >= 0
        self.rows, self.cols = np.nonzero(self.covered)
        tri = self.tri_of[self.rows, self.cols]
        w = self.bary[self.rows, self.cols]
        # Each covered texel's rest position and face set.
        self.X = np.einsum("nk,nkd->nd", w, self.P[self.tris[tri]])
        self.sets = self.tri_set[tri]
        self.tri = tri
        self.w = w

    def field(self, values):
        """Per-vertex `values` interpolated over the covered texels."""
        values = np.asarray(values, dtype=np.float64)
        return np.einsum("nk,nk->n", self.w, values[self.tris[self.tri]])

    def image(self, value):
        """A [size, size] image of per-covered-texel `value`, grown
        `MARGIN` texels past the islands."""
        out = np.zeros((self.size, self.size))
        out[self.rows, self.cols] = value
        return grow(out, self.covered, MARGIN)


def grow(image, covered, times):
    """`image` grown out of `covered` by `times` texels: each texel
    outside takes the largest of its neighbours'."""
    image, covered = image.copy(), covered.copy()
    for _ in range(times):
        best = np.zeros_like(image)
        got = np.zeros_like(covered)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            shifted = np.roll(np.roll(image, dy, axis=0), dx, axis=1)
            has = np.roll(np.roll(covered, dy, axis=0), dx, axis=1)
            take = has & ~covered & (shifted >= best)
            best = np.where(take, shifted, best)
            got |= has & ~covered
        image = np.where(got & ~covered, best, image)
        covered = covered | got
    return image


def polyline_distance(X, line):
    """Each point's distance to the polyline `line`, the nearest point on
    it and how far along it that is (0 to 1)."""
    line = np.asarray(line, dtype=np.float64)
    best = np.full(len(X), np.inf)
    near = np.zeros_like(X)
    along = np.zeros(len(X))
    lengths = np.linalg.norm(np.diff(line, axis=0), axis=1)
    total = max(float(lengths.sum()), 1e-12)
    start = np.concatenate([[0.0], np.cumsum(lengths)])
    for k, (a, b) in enumerate(zip(line[:-1], line[1:])):
        ab = b - a
        t = np.clip(((X - a) @ ab) / max(float(ab @ ab), 1e-18), 0.0, 1.0)
        p = a + t[:, None] * ab
        d = np.linalg.norm(X - p, axis=1)
        closer = d < best
        best = np.where(closer, d, best)
        near[closer] = p[closer]
        along = np.where(closer, (start[k] + t * lengths[k]) / total, along)
    return best, near, along


# -- the makeup masks -----------------------------------------------------------------

def lash_line(face, s, upper=True, loop=1):
    """The lid's rim where the lashes grow (the rim's second loop, which
    the lashes root on; `loop` 2 is the lid's skin just outside it), from
    the inner corner out, on the lid `upper`."""
    eye = face.eye[s]
    loop = [i for i in eye.loops[loop] if eye.upper.get(i) == upper]
    pts = face.P[loop]
    order = np.argsort(eye.along(pts))
    return pts[order]


# The eyeliner's wing: how long, and how far it rises from the line
# through the eye's corners (degrees). A wing carries the lower lid's
# line on out of the corner, rising a little toward the brow's tail
# (makeup guides: "follow the angle of the lower lash line"). 5 mm on the
# face's plane is about 6.5 mm along the skin, which turns back toward
# the temple there.
WING_MM = 5.0
WING_RISE = (15.0, 22.0)


def liner_wing(face, s):
    """The eyeliner wing of eye `s`: the outer corner of the lid's skin
    loop the liner runs along (where its upper and lower halves meet), the
    wing's points on the resting skin from there, in the eye's own frame,
    and the lower lid's own angle at the corner (degrees). Its direction is the lower lid's line at the corner, held to
    `WING_RISE` above the corners' line; each point is put on the skin
    straight back along the face's forward axis, so the wing keeps that
    direction seen from the front. (Wave 3's first wing was put on the
    skin by the nearest point, which pulled it up the curved temple: it
    rose 60 to 70 degrees, a spike beside the eye.)"""
    from mathutils import Vector

    eye = face.eye[s]
    ring = face.P[eye.loops[2]]
    corner = ring[np.argmax(eye.along(ring))]
    low = lash_line(face, s, upper=False)
    near_corner = low[eye.along(low) > eye.t_out - 0.004]
    if len(near_corner) < 2:
        near_corner = low[-2:]
    v = near_corner[-1] - near_corner[0]
    lower = float(np.degrees(np.arctan2(v @ eye.up, v @ eye.axis)))
    rise = float(np.clip(lower, *WING_RISE))
    out = eye.axis * np.cos(np.radians(rise)) + eye.up * np.sin(np.radians(rise))
    pts = []
    for k in np.linspace(0.0, WING_MM / 1000.0, 9):
        target = corner + out * k
        hit = face.skin_tree.ray_cast(Vector(target + eye.fwd * 0.03), Vector(-eye.fwd), 0.06)
        pts.append(np.array(hit[0]) if hit[0] is not None else face.on_skin(target))
    pts[0] = corner
    return corner, np.array(pts), lower


def wing_report(face):
    """Each side's wing measured on the resting skin: how far it rises
    from the corners' line on the face's plane (`rise_degrees`) and along
    the skin at the corner (`skin_degrees`, the angle to that line laid on
    the skin), its length and how far it leaves from where the lids'
    margins meet. Raises if a wing turns more than 30 degrees: the build
    doesn't ship a spike."""
    report = []
    for _, s in SIDES:
        eye = face.eye[s]
        corner, wing, lower = liner_wing(face, s)
        w = wing[-1] - wing[0]
        rise = float(np.degrees(np.arctan2(w @ eye.up, w @ eye.axis)))
        hit = face.skin_tree.find_nearest(tuple(corner), 0.01)
        n = np.array(hit[1]) if hit[0] is not None else eye.fwd
        flat = lambda v: (v - n * (v @ n)) / max(np.linalg.norm(v - n * (v @ n)), 1e-12)  # noqa: E731
        along_skin = float(np.degrees(np.arccos(np.clip(flat(w) @ flat(eye.axis), -1.0, 1.0))))
        length = float(np.sum(np.linalg.norm(np.diff(wing, axis=0), axis=1)))
        from_corner = float(np.linalg.norm(corner - face.P[eye.canthus_out]))
        report.append({"side": s, "rise_degrees": round(rise, 1), "skin_degrees": round(along_skin, 1),
                       "length_mm": round(length * 1000.0, 2), "from_corner_mm": round(from_corner * 1000.0, 2),
                       "lower_lid_degrees": round(lower, 1)})
        if not (10.0 <= rise <= 30.0 and along_skin <= 30.0):
            raise RuntimeError(f"eyeliner wing on side {s} turns {rise:.1f} degrees ({along_skin:.1f} on the skin)")
    return report


def makeup_masks(tx, face, lips):
    """Foundation, blush, eyeshadow, eyeliner, lip colour and brow fill
    over the covered texels (each 0 to 1)."""
    X = tx.X
    skin = np.isin(tx.sets, (FS_HEAD, FS_MUZZLE))
    out = {}
    lip = np.clip(tx.field(lips) * 1.15, 0.0, 1.0) * skin
    out["lips"] = lip

    # Foundation: the face, in front of the ears, from the hairline to
    # under the jaw, fading over a centimetre or two; thinner on the lids
    # and off the lips.
    ear_y = min(face.ear_at[s][1] for _, s in SIDES)
    front = _step(ear_y + 0.002, ear_y - 0.025, X[:, 1])
    brow_z = max(face.eye[s].c[2] for _, s in SIDES) + 0.02
    top = 1.0 - _step(brow_z + 0.035, brow_z + 0.055, X[:, 2])
    # Under the jaw: below the line from the chin to under the ears.
    chin = face.chin
    ear = np.mean([face.ear_at[s] for _, s in SIDES], axis=0)
    t = np.clip((X[:, 1] - chin[1]) / (ear[1] - chin[1]), 0.0, 1.0)
    jaw_z = chin[2] + t * (ear[2] - 0.035 - chin[2])
    under = _step(jaw_z - 0.022, jaw_z - 0.004, X[:, 2])
    near_eye = np.zeros(len(X))
    for _, s in SIDES:
        d, _, _ = polyline_distance(X, np.concatenate([lash_line(face, s), lash_line(face, s, False)[::-1]]))
        near_eye = np.maximum(near_eye, 1.0 - _step(0.0015, 0.004, d))
    out["foundation"] = skin * front * top * under * (1.0 - lip) * (1.0 - 0.6 * near_eye)

    # Blush: on each cheek's apple, swept up and out toward the temple.
    blush = np.zeros(len(X))
    liner = np.zeros(len(X))
    shadow = np.zeros(len(X))
    for _, s in SIDES:
        eye = face.eye[s]
        apple = face.on_skin(eye.c + eye.up * -0.031 + eye.axis * 0.013 + eye.fwd * 0.03)
        temple = face.on_skin(eye.c + eye.up * -0.006 + eye.axis * 0.040 + eye.fwd * 0.0)
        along = temple - apple
        along /= np.linalg.norm(along)
        rel = X - apple
        a = rel @ along
        across = np.linalg.norm(rel - np.outer(a, along), axis=1)
        # (It fades sooner below the apple than up toward the temple.)
        a = np.where(a < 0.0, 1.7 * a, a)
        w = np.exp(-(a ** 2) / 0.024 ** 2 - across ** 2 / 0.013 ** 2)
        blush = np.maximum(blush, w * (X[:, 0] * s > 0.01))

        # Eyeliner: along the upper lash line, thin at the inner corner and
        # wider outward, then out of the outer corner as a short wing that
        # tapers to a point (`liner_wing`); a thinner line under the outer
        # half of the lower lid. (Drawn along the lid's skin just outside
        # the lashes' roots, the line a person sees: on the roots
        # themselves the lashes and the lid's rim hid it.)
        up_line = lash_line(face, s, loop=2)
        corner, wing, _ = liner_wing(face, s)
        # The upper line runs into the corner the wing leaves from, so the
        # two are one stroke.
        line = np.concatenate([up_line[eye.along(up_line) < eye.along(corner)[0] - 0.0005], wing])
        d, near, t = polyline_distance(X, line)
        lengths = np.linalg.norm(np.diff(line, axis=0), axis=1)
        share = float(np.sum(lengths[:len(line) - len(wing)]) / np.sum(lengths))
        # Widest (2.2 mm) where the line meets the wing; the wing tapers
        # from that to 0.3 mm at its tip.
        width = np.where(t < share, 0.0009 + 0.0013 * (t / share),
                         0.0022 - 0.0019 * (t - share) / max(1.0 - share, 1e-6))
        # Held to the line's upper side, as a liner's lower edge follows the
        # lashes (and, on the wing, carries the lower lid's line on).
        above = (X - near) @ eye.up > -0.0012
        on = (1.0 - _step(width * 0.75, width + 0.0004, d)) * above
        low_line = lash_line(face, s, False)
        d_low, near_low, t_low = polyline_distance(X, low_line)
        below = (X - near_low) @ eye.up < 0.0004
        low = (1.0 - _step(0.00045, 0.0009, d_low)) * below * _step(0.35, 0.7, t_low) * 0.7
        liner = np.maximum(liner, np.maximum(on, low))

        # Eyeshadow: the upper lid, from the lash line up to under the brow,
        # deepest by the lashes and toward the outer corner.
        d_up, near_up, t_up = polyline_distance(X, up_line)
        rise = (X - near_up) @ eye.up
        lid = _step(-0.0003, 0.001, rise) * (1.0 - _step(0.005, 0.011, d_up))
        span = (X - eye.c) @ eye.axis
        sides = _step(eye.t_in - 0.002, eye.t_in + 0.003, span) * (1.0 - _step(eye.t_out + 0.004, eye.t_out + 0.011, span))
        shadow = np.maximum(shadow, lid * sides * (0.75 + 0.25 * t_up) * (X[:, 0] * s > 0))
    out["blush"] = blush * skin * (1.0 - lip)
    out["eyeliner"] = liner * skin
    out["eyeshadow"] = shadow * skin * (1.0 - 0.3 * liner)

    # Brow fill: the skin under the brows' hair, along the brows' line,
    # wider at the inner end and tapering to the tail.
    from lashes import BROW_END, BROW_START
    fill = np.zeros(len(X))
    for _, s in SIDES:
        curve = face.brow_curve(s, 40, BROW_START + 0.04, BROW_END - 0.06)
        d, _, t = polyline_distance(X, curve)
        half = 0.0048 - 0.0024 * t
        fill = np.maximum(fill, (1.0 - _step(half * 0.55, half, d)) * _step(0.0, 0.08, t))
    out["brow_fill"] = fill * skin
    return out


def scar_mask(tx, face):
    """One scar: a slightly curved line about 3.5 cm long and 3 mm wide down the
    left cheek, from below the eye's outer corner toward the corner of the
    mouth, tapering at both ends, with a faint halo."""
    eye = face.eye[1]
    a = eye.c + eye.axis * (eye.t_out - 0.002) + eye.up * -0.020 + eye.fwd * 0.03
    b = eye.c + eye.axis * (eye.t_out - 0.016) + eye.up * -0.047 + eye.fwd * 0.03
    bow = eye.axis * 0.003
    pts = [face.on_skin(a + (b - a) * t + bow * 4.0 * t * (1.0 - t)) for t in np.linspace(0.0, 1.0, 16)]
    d, _, t = polyline_distance(tx.X, pts)
    width = 0.0018 * np.sin(np.pi * np.clip(t, 0.02, 0.98)) ** 0.5
    core = 1.0 - _step(width * 0.45, width + 0.0005, d)
    halo = 0.3 * (1.0 - _step(width, width + 0.003, d))
    return np.maximum(core, halo) * np.isin(tx.sets, (FS_HEAD, FS_MUZZLE)), float(np.sum(np.linalg.norm(np.diff(pts, axis=0), axis=1)))


def stubble_mask(tx, face, lips, seed=SEED + 6):
    """Stubble's density over the beard's region (R6, ADR-0042's spare
    channel, mask 2's alpha): the cheeks below a line from the sideburn
    (in front of the ear, at its middle) to a little above the mouth's
    corner, the upper lip under the nose, the chin and jaw, and under the
    jaw onto the neck (3 cm at the front, less toward the ear), each edge
    soft over about 6 mm, off the lips; times a fine stipple, a follicle
    every millimetre or so, so shaved hair reads as dots, not a shadow."""
    X = tx.X
    skin = np.isin(tx.sets, (FS_HEAD, FS_MUZZLE))
    mouth = face.stomion
    nose = face.nose_tip
    out = np.zeros(len(X))
    for _, s in SIDES:
        ear = face.ear_at[s]
        side = X[:, 0] * s > -0.004
        # The cheek line, from the sideburn forward and down to above the
        # mouth's corner; the moustache up to the nose's base in front.
        t = np.clip((X[:, 1] - ear[1]) / (mouth[1] + 0.012 - ear[1]), 0.0, 1.0)
        top = ear[2] + 0.002 + t * (mouth[2] + 0.013 - ear[2] - 0.002)
        under_nose = (np.abs(X[:, 0]) < 0.024) & (X[:, 1] < mouth[1] + 0.012)
        top = np.where(under_nose, np.maximum(top, nose[2] - 0.016), top)
        below_line = 1.0 - _step(top - 0.006, top, X[:, 2])
        in_front = _step(ear[1] + 0.006, ear[1] - 0.004, X[:, 1])
        # Under the jaw: down the neck 3 cm at the front, 1.2 cm by the ear.
        chin = face.chin
        u = np.clip((X[:, 1] - chin[1]) / (ear[1] - chin[1]), 0.0, 1.0)
        jaw_z = chin[2] + u * (ear[2] - 0.035 - chin[2])
        low = jaw_z - 0.03 + 0.018 * u
        above_neck = _step(low - 0.008, low, X[:, 2])
        out = np.maximum(out, below_line * in_front * above_neck * side)
    lip = np.clip(tx.field(lips) * 1.6, 0.0, 1.0)
    region = out * skin * (1.0 - lip)
    # The stipple: a value per 1.2 mm cell of the rest surface (hashed),
    # most cells a follicle, smoothed over a texel by the image's filter.
    cell = np.floor(X / 0.0012).astype(np.int64).astype(np.uint64)
    h = cell[:, 0] * np.uint64(0x9E3779B97F4A7C15) + cell[:, 1] * np.uint64(0xC2B2AE3D27D4EB4F)         + cell[:, 2] * np.uint64(0x165667B19E3779F9) + np.uint64(seed)
    # splitmix64's finish, so neighbouring cells don't line up in stripes.
    h ^= h >> np.uint64(30)
    h *= np.uint64(0xBF58476D1CE4E5B9)
    h ^= h >> np.uint64(27)
    h *= np.uint64(0x94D049BB133111EB)
    h ^= h >> np.uint64(31)
    grain = (h >> np.uint64(40)).astype(np.float64) / float(1 << 24)
    follicle = _step(0.25, 0.6, grain)
    return region * (0.35 + 0.65 * follicle), region


# -- seeded spots ---------------------------------------------------------------------

def sun_field(face, J, P):
    """Per vertex, 0 to 1: where freckles come (where the sun falls): the
    nose and cheeks, a little on the forehead; the shoulders, the upper
    back and chest; the forearms' backs and the upper arms' outsides."""
    X = np.asarray(P, dtype=np.float64)

    def blob(centre, radius):
        return np.exp(-((X - np.asarray(centre)) ** 2).sum(axis=1) / radius ** 2)

    sun = np.zeros(len(X))
    nose = face.nose_tip
    sun += 0.9 * blob(nose + np.array([0.0, 0.006, 0.018]), 0.016)
    for _, s in SIDES:
        eye = face.eye[s]
        cheek = face.on_skin(eye.c + eye.up * -0.030 + eye.axis * -0.004 + eye.fwd * 0.03)
        sun += blob(cheek, 0.024)
    sun += 0.35 * blob(face.on_skin(np.array([0.0, nose[1], face.eye[1].c[2] + 0.05])), 0.03)
    for side, s in SIDES:
        shoulder = J[f"{side}UpperArm"]
        sun += 0.75 * blob(shoulder + np.array([0.0, 0.0, 0.04]), 0.085)
        sun += 0.45 * blob(0.5 * (J[f"{side}UpperArm"] + J[f"{side}LowerArm"]) + np.array([s * 0.03, 0.0, 0.0]), 0.07)
        sun += 0.6 * blob(0.5 * (J[f"{side}LowerArm"] + J[f"{side}Hand"]), 0.09)
    # The upper back and the top of the chest (the neckline).
    sun += 0.6 * blob(J["Chest"] + np.array([0.0, 0.07, 0.07]), 0.12)
    sun += 0.4 * blob(J["Chest"] + np.array([0.0, -0.06, 0.09]), 0.08)
    return np.clip(sun, 0.0, 1.0)


def _spots_on(tx, density, count_per_m2, radius, rng, fixed=None):
    """Candidate spots on the surface: (position, triangle, barycentric,
    radius) for each, area-weighted over triangles by `density` (per
    vertex) at `count_per_m2` where it is 1, none closer to another than
    the sum of their radii and a half. `fixed`: an exact count instead."""
    P, tris = tx.P, tx.tris
    a, b, c = P[tris[:, 0]], P[tris[:, 1]], P[tris[:, 2]]
    area = 0.5 * np.linalg.norm(np.cross(b - a, c - a), axis=1)
    weight = area * np.asarray(density, dtype=np.float64)[tris].mean(axis=1)
    total = float(weight.sum())
    count = fixed if fixed is not None else int(round(total * count_per_m2))
    if count <= 0 or total <= 0:
        return []
    want = rng.choice(len(tris), size=count * 3, p=weight / total)
    out, kept = [], {}
    cell = 2.5 * radius[1]
    for t in want:
        if len(out) >= count:
            break
        u, v = rng.random(2)
        if u + v > 1.0:
            u, v = 1.0 - u, 1.0 - v
        w = np.array([1.0 - u - v, u, v])
        p = w @ P[tris[t]]
        r = rng.uniform(*radius)
        key = tuple(np.floor(p / cell).astype(int))
        close = False
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    for q, rq in kept.get((key[0] + dx, key[1] + dy, key[2] + dz), ()):
                        if np.linalg.norm(p - q) < 1.5 * (r + rq):
                            close = True
        if close:
            continue
        kept.setdefault(key, []).append((p, r))
        out.append((p, t, w, r))
    return out


def spot_masks(tx, spots, ranks, rng, wobble=0.25):
    """Coverage and rank images of `spots` (soft blobs measured on the
    rest surface, their edges wobbling by up to `wobble` of their radius
    round them, so no two are the same circle); each spot's rank spread
    `MARGIN` texels past it."""
    size = tx.size
    position = np.full((size, size, 3), np.inf)
    position[tx.rows, tx.cols] = tx.X
    cover = np.zeros((size, size))
    rank = np.zeros((size, size))
    best = np.full((size, size), np.inf)
    for (p, t, w, r), q in zip(spots, ranks):
        centre = w @ tx.tri_px[t]
        # Texels across this triangle per metre: the window to look in.
        P3 = tx.P[tx.tris[t]]
        area3 = 0.5 * np.linalg.norm(np.cross(P3[1] - P3[0], P3[2] - P3[0]))
        uv = tx.tri_px[t]
        area2 = 0.5 * abs((uv[1, 0] - uv[0, 0]) * (uv[2, 1] - uv[0, 1]) - (uv[2, 0] - uv[0, 0]) * (uv[1, 1] - uv[0, 1]))
        per_m = np.sqrt(area2 / max(area3, 1e-12))
        reach = int(np.ceil(r * per_m * 2.0)) + MARGIN
        x0, y0 = int(centre[0]) - reach, int(centre[1]) - reach
        x1, y1 = int(centre[0]) + reach + 1, int(centre[1]) + reach + 1
        x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, size), min(y1, size)
        d = np.linalg.norm(position[y0:y1, x0:x1] - p, axis=-1)
        # (Texels just outside an island have no position: the margin.)
        # Round the spot (in the texture's own directions, near enough at
        # this size), its edge two waves of random phase.
        gx, gy = np.meshgrid(np.arange(x0, x1) + 0.5, np.arange(y0, y1) + 0.5)
        turn = np.arctan2(gy - centre[1], gx - centre[0])
        a, b = rng.uniform(0.0, 2.0 * np.pi, 2)
        edge = r * (1.0 + wobble * (0.6 * np.sin(2.0 * turn + a) + 0.4 * np.sin(3.0 * turn + b)))
        here = 1.0 - _step(0.55 * edge, edge, d)
        patch_c = cover[y0:y1, x0:x1]
        np.maximum(patch_c, here, out=patch_c)
        # The rank of the nearest spot, out to a few texels past it, in
        # pixels (so the margin round an island gets it too).
        px = np.hypot(gx - centre[0], gy - centre[1]) / max(r * per_m, 1e-6)
        mine = (px < 1.0 + MARGIN / max(r * per_m, 1.0)) & (px < best[y0:y1, x0:x1])
        rank[y0:y1, x0:x1][mine] = q
        best[y0:y1, x0:x1][mine] = px[mine]
    cover = grow(cover, tx.covered, MARGIN)
    return cover, rank


# -- the build's entry ----------------------------------------------------------------

def masks(body, topo, P, PV, lm, eyes, eye_radius, lips, size, out_dir):
    """Writes the three mask images for `body` (final texture
    coordinates) at rest positions `P` (array; `PV` the same as Vectors
    for the skeleton's fit, `lm` its landmarks), with the albedo's per
    vertex lip field `lips`. Returns what to record in the build data."""
    import face as facelib
    from PIL import Image
    from skin import _joint_heads

    tx = Texels(body, topo, P, size)
    face = facelib.Face(topo, P, eyes, eye_radius, None)
    makeup = makeup_masks(tx, face, lips)
    scar, scar_length = scar_mask(tx, face)

    rng = np.random.default_rng(SEED)
    J = _joint_heads(PV, lm)
    J = {name: np.asarray(p, dtype=np.float64) for name, p in J.items()}
    sun = sun_field(face, J, P)
    # Freckles only on skin a person shows (not inside the mouth or the
    # sockets, nor the palms and soles: the sun field has none there).
    skin_sets = np.ones(len(P))
    for fs in FS_SOCKET.values():
        skin_sets[topo.verts(fs)] = 0.0
    freckles = _spots_on(tx, sun * skin_sets, FRECKLE_DENSITY * 1e4, FRECKLE_RADIUS, rng)
    # Moles: anywhere on the skin, the face and back a little more.
    anywhere = skin_sets * (1.0 + np.isin(np.arange(len(P)), topo.verts(FS_TORSO)) * 0.5)
    head = np.zeros(len(P))
    head[topo.verts(FS_HEAD, FS_MUZZLE)] = 1.0
    front = (np.asarray(P)[:, 1] < face.ear_at[1][1] - 0.02) & (np.asarray(P)[:, 2] > face.chin[2] - 0.01)
    # (Off the face's middle: a mole between the brows read as a bindi.)
    off_middle = _step(0.005, 0.012, np.abs(np.asarray(P)[:, 0]))
    face_moles = _spots_on(tx, head * front * off_middle, 0.0, MOLE_RADIUS, rng, fixed=FACE_MOLES)
    body_moles = _spots_on(tx, anywhere * (1.0 - head), 0.0, MOLE_RADIUS, rng, fixed=MOLES - FACE_MOLES)
    moles = face_moles + body_moles
    freckle_cover, freckle_rank = spot_masks(tx, freckles, rng.random(len(freckles)), rng)
    mole_cover, mole_rank = spot_masks(tx, moles, rng.random(len(moles)), rng, wobble=0.12)
    # Spots stay off the lips and the mouth's lining.
    lips_img = tx.image(makeup["lips"])
    freckle_cover *= 1.0 - lips_img
    mole_cover *= 1.0 - lips_img

    def byte(a):
        return (np.clip(a, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)

    a = np.stack([tx.image(makeup[k]) for k in ("foundation", "blush", "eyeshadow", "eyeliner")], axis=2)
    stubble, stubble_region = stubble_mask(tx, face, lips)
    b = np.stack([tx.image(makeup["lips"]), tx.image(makeup["brow_fill"]), tx.image(scar),
                  tx.image(stubble)], axis=2)
    c = np.stack([freckle_cover, freckle_rank, mole_cover, mole_rank], axis=2)
    digests = {}
    for name, image in zip(FILES, (a, b, c)):
        path = os.path.join(out_dir, name)
        Image.fromarray(byte(image), "RGBA").save(path, optimize=True)
        with open(path, "rb") as f:
            digests[name] = digest(f.read())
    return {
        "files": list(FILES),
        # FNV-1a of each file's bytes, as `mark` embeds them: a test holds
        # the model file's masks to the ones measured here.
        "digests": digests,
        "wing": wing_report(face),
        "freckles": len(freckles),
        "moles": len(moles),
        "face_moles": len(face_moles),
        "scar_mm": round(scar_length * 1000.0, 1),
        "covered_share": dict({k: round(float((tx.image(v) > 0.5).mean()), 5) for k, v in makeup.items()},
                              stubble=round(float((tx.image(stubble_region) > 0.5).mean()), 5)),
    }


def digest(data):
    """FNV-1a, 64 bits, as hex (the assets test computes the same)."""
    h = 0xCBF29CE484222325
    for byte in data:
        h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def mark(glb, out_dir):
    """Adds the three masks to the exported file `glb` and names them in
    its `Skin` material's `extras` (`axcenity_skin_masks`), as textures
    placed like its albedo (`skin.mark` does the same for the defined
    normal map)."""
    import json
    import struct
    paths = [os.path.join(out_dir, name) for name in FILES]
    if not all(os.path.exists(p) for p in paths):
        return
    with open(glb, "rb") as f:
        data = f.read()
    _, version, _ = struct.unpack_from("<III", data, 0)
    chunks, at = [], 12
    while at < len(data):
        length, kind = struct.unpack_from("<II", data, at)
        chunks.append([kind, data[at + 8:at + 8 + length]])
        at += 8 + length
    doc = json.loads(chunks[0][1])
    binary = bytearray(chunks[1][1]) if len(chunks) > 1 else bytearray()
    skin = next(m for m in doc["materials"] if m.get("name") == "Skin")
    colour = doc["textures"][skin["pbrMetallicRoughness"]["baseColorTexture"]["index"]]
    indexes = []
    for path in paths:
        with open(path, "rb") as f:
            png = f.read()
        binary.extend(b"\0" * (-len(binary) % 4))
        doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(png)})
        binary.extend(png)
        doc["images"].append({"bufferView": len(doc["bufferViews"]) - 1, "mimeType": "image/png",
                              "name": os.path.splitext(os.path.basename(path))[0]})
        doc["textures"].append(dict(colour, source=len(doc["images"]) - 1))
        indexes.append({"index": len(doc["textures"]) - 1})
    skin["extras"] = dict(skin.get("extras", {}), axcenity_skin_masks=indexes)
    binary.extend(b"\0" * (-len(binary) % 4))
    doc["buffers"][0]["byteLength"] = len(binary)
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    out = struct.pack("<II", len(text), 0x4E4F534A) + text + struct.pack("<II", len(binary), 0x004E4942) + binary
    with open(glb, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, version, 12 + len(out)) + out)


def clean_up(out_dir):
    for name in FILES:
        path = os.path.join(out_dir, name)
        if os.path.exists(path):
            os.remove(path)


