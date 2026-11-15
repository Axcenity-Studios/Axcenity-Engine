"""The eyes of the realistic head (Character Studio overhaul CO6).

`Eyes` (on the eye bones, so it turns with the gaze), per eye:

- **Sclera**: the white, a sphere open at the front where the iris is.
- **Iris**: a shallow dish set back inside the limbus, with the pupil in
  its middle. The shape `Pupil_Size` widens (+1) or narrows (-1) the
  pupil by moving the iris's inner rings; the texture stretches with it.
- **Cornea**: the clear cap standing on the limbus, 1.2 mm proud of the
  white at its top, with a skirt that runs a little way over the white.
  Alpha-blended and nearly clear: it is there for the highlight and for
  the eye shading (CO7) to refract through.

`EyeShell` (on the head, following the lids by the body's shapes), per eye:

- **EyeOcclusion**: a film over the eyeball between the lids, dark at the
  lids' edges and clear in the middle: the shadow the lids cast on the
  eye, which a renderer without it leaves out (the eye then glows).
- **TearLine**: a thin strip where the lids' wet edge meets the eyeball,
  the line of fluid that catches the light.

Every vertex of `EyeShell` is a fixed mix of the lid's inner edge
vertices (`mix`), stood off the eyeball by a height that shrinks as the
lids come together (`follow`), so it follows any shape that moves the
lids and folds flat between them when they close.
"""

import math
import os

import bpy
import numpy as np

from facetopo import LIMBUS_DEG, SIDES, Eyeball

SEGMENTS = 24
# Angles off the eye's axis of the white's rings (from the limbus back)
# and of the cornea's (from its top out to the skirt's edge).
SCLERA_RINGS = (LIMBUS_DEG, 33.0, 38.0, 45.0, 54.0, 65.0, 78.0, 93.0, 110.0, 130.0, 152.0)
CORNEA_RINGS = (5.0, 10.5, 16.0, 21.0, 25.5, LIMBUS_DEG, 32.0, 35.0)
# The iris: its rings as shares of its radius (the pupil's edge first),
# and how far each is set back from the limbus's plane (metres).
PUPIL = 0.35
IRIS_RINGS = ((PUPIL, 0.00115), (PUPIL + 0.05, 0.00112), (0.62, 0.00095), (0.86, 0.00070), (1.0, 0.00035))
PUPIL_WIDE, PUPIL_NARROW = 1.6, 0.55
# The texture spans this much more than the iris (its limbal ring fades
# into the white outside it).
IRIS_IN_TEXTURE = 0.86
# The white's texture: the angle off the axis its edge shows.
SCLERA_SPAN = 115.0
# The occlusion film: how far each ring is from the lid's edge toward the
# line between the lids (0 to 1), and how high it stands over the white.
FILM_RINGS = ((0.0, 0.0), (0.3, 0.0008), (0.65, 0.0014), (1.0, 0.0015))
TEAR_REACH = 0.16
# How far the film and the tear line draw back behind closed lids (metres).
TUCK = 0.0009
# The film and the tear line leave out the corners of the eye, where the
# lids are nearer each other than this at rest (metres).
CORNER = 0.0035


def _dir(theta, phi):
    return np.array([math.sin(theta) * math.cos(phi), -math.cos(theta), math.sin(theta) * math.sin(phi)])


def _sclera_uv(theta_deg, phi):
    r = 0.5 * min(1.0, theta_deg / SCLERA_SPAN) ** 0.8
    return (0.5 + r * math.cos(phi), 0.5 + r * math.sin(phi))


def eyeball(radius):
    """One eyeball about the origin looking along -Y: (points, faces,
    material of each face, texture coordinates per face corner, the
    pupil's shape as offsets per point)."""
    ball = Eyeball((0.0, 0.0, 0.0), radius)
    pts, faces, mats, uvs = [], [], [], []
    pupil = []
    limbus = math.radians(LIMBUS_DEG)
    iris_r = radius * math.sin(limbus)
    plane = radius * math.cos(limbus)

    def ring(make):
        first = len(pts)
        for k in range(SEGMENTS):
            p, move = make(2.0 * math.pi * k / SEGMENTS)
            pts.append(p)
            pupil.append(move)
        return first

    def band(a, b, material, uv_a, uv_b):
        for k in range(SEGMENTS):
            j = (k + 1) % SEGMENTS
            p0, p1 = 2.0 * math.pi * k / SEGMENTS, 2.0 * math.pi * (k + 1) / SEGMENTS
            faces.append((a + k, a + j, b + j, b + k))
            mats.append(material)
            uvs.append([uv_a(p0), uv_a(p1), uv_b(p1), uv_b(p0)])

    def fan(centre, a, material, uv_c, uv_a, flip=False):
        # `flip` for a fan seen from the front (the rings run anticlockwise there).
        for k in range(SEGMENTS):
            j = (k + 1) % SEGMENTS
            p0, p1 = 2.0 * math.pi * k / SEGMENTS, 2.0 * math.pi * (k + 1) / SEGMENTS
            face, uv = (centre, a + j, a + k), [uv_c, uv_a(p1), uv_a(p0)]
            if flip:
                face, uv = (centre, a + k, a + j), [uv_c, uv_a(p0), uv_a(p1)]
            faces.append(face)
            mats.append(material)
            uvs.append(uv)

    zero = np.zeros(3)
    # The white.
    rings = [ring(lambda phi, t=t: (_dir(math.radians(t), phi) * radius, zero)) for t in SCLERA_RINGS]
    for (a, ta), (b, tb) in zip(zip(rings, SCLERA_RINGS), zip(rings[1:], SCLERA_RINGS[1:])):
        band(b, a, "Sclera", lambda phi, t=tb: _sclera_uv(t, phi), lambda phi, t=ta: _sclera_uv(t, phi))
    pole = len(pts)
    pts.append(np.array([0.0, radius, 0.0]))
    pupil.append(zero)
    fan(pole, rings[-1], "Sclera", (0.5, 0.5), lambda phi: _sclera_uv(SCLERA_RINGS[-1], phi))
    # The iris, inside the limbus.
    iris_uv = lambda share: (lambda phi: (0.5 + 0.5 * IRIS_IN_TEXTURE * share * math.cos(phi),  # noqa: E731
                                          0.5 + 0.5 * IRIS_IN_TEXTURE * share * math.sin(phi)))

    def iris_ring(share, depth):
        def make(phi):
            p = np.array([iris_r * share * math.cos(phi), -(plane - depth), iris_r * share * math.sin(phi)])
            # The pupil's edge and the ring beside it move with the pupil.
            grow = (PUPIL_WIDE - 1.0) * max(0.0, 1.0 - (share - PUPIL) / 0.3) if share < 0.6 else 0.0
            return p, np.array([p[0] * grow, 0.0, p[2] * grow])
        return ring(make)

    irises = [iris_ring(share, depth) for share, depth in IRIS_RINGS]
    centre = len(pts)
    pts.append(np.array([0.0, -(plane - IRIS_RINGS[0][1]), 0.0]))
    pupil.append(zero)
    fan(centre, irises[0], "Iris", (0.5, 0.5), iris_uv(IRIS_RINGS[0][0]), flip=True)
    for (a, (sa, _)), (b, (sb, _)) in zip(zip(irises, IRIS_RINGS), zip(irises[1:], IRIS_RINGS[1:])):
        band(b, a, "Iris", iris_uv(sb), iris_uv(sa))
    # The iris's rim meets the white's.
    band(rings[0], irises[-1], "Iris", iris_uv(1.04), iris_uv(1.0))
    # The cornea.
    top = len(pts)
    pts.append(ball.f * ball.apex)
    pupil.append(zero)

    def cornea_ring(t):
        def make(phi):
            d = _dir(math.radians(t), phi)
            return d * (float(ball.height(d[None])[0]) + (0.00004 if t > LIMBUS_DEG else 0.0)), zero
        return ring(make)

    caps = [cornea_ring(t) for t in CORNEA_RINGS]
    fan(top, caps[0], "Cornea", (0.5, 0.5), lambda phi: _sclera_uv(CORNEA_RINGS[0], phi), flip=True)
    for (a, ta), (b, tb) in zip(zip(caps, CORNEA_RINGS), zip(caps[1:], CORNEA_RINGS[1:])):
        band(b, a, "Cornea", lambda phi, t=tb: _sclera_uv(t, phi), lambda phi, t=ta: _sclera_uv(t, phi))
    return np.array(pts), faces, mats, uvs, np.array(pupil)


def sclera_texture(out_dir, size=512):
    """The white of the eye: paler at the front, pinker toward the lids
    and the back, fine vessels running in from the edge, a soft grey ring
    where it meets the iris."""
    from PIL import Image, ImageDraw, ImageFilter
    rng = np.random.default_rng(23)
    y, x = np.mgrid[0:size, 0:size]
    u, v = (x + 0.5) / size * 2 - 1, (y + 0.5) / size * 2 - 1
    r = np.sqrt(u * u + v * v)
    theta = np.clip(r, 0, 1) ** (1 / 0.8) * SCLERA_SPAN
    white, pink = np.array([0.93, 0.915, 0.89]), np.array([0.88, 0.76, 0.72])
    t = np.clip((theta - 42.0) / 55.0, 0, 1)[:, :, None] ** 1.3
    rgb = white * (1 - t) + pink * t
    rim = np.exp(-((theta - LIMBUS_DEG) / 4.5) ** 2)[:, :, None]
    rgb = rgb * (1 - 0.30 * rim) + np.array([0.62, 0.64, 0.66]) * 0.30 * rim
    base = Image.fromarray((np.clip(rgb, 0, 1) * 255 + 0.5).astype(np.uint8))
    veins = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(veins)
    edge = lambda th: 0.5 * min(1.0, th / SCLERA_SPAN) ** 0.8  # noqa: E731
    for _ in range(46):
        a = rng.uniform(0, 2 * math.pi)
        th = rng.uniform(70, 100)
        points = []
        for _ in range(int(rng.uniform(8, 16))):
            rr = edge(th)
            points.append((size * (0.5 + rr * math.cos(a)), size * (0.5 + rr * math.sin(a))))
            th -= rng.uniform(2.0, 5.0)
            a += rng.normal(0, 0.05)
            if th < LIMBUS_DEG + 6:
                break
        if len(points) > 1:
            draw.line(points, fill=int(rng.uniform(70, 150)), width=1)
    veins = np.asarray(veins.filter(ImageFilter.GaussianBlur(0.7)), dtype=np.float32)[:, :, None] / 255.0
    rgb = np.asarray(base, dtype=np.float32) / 255.0
    rgb = rgb * (1 - 0.55 * veins) + np.array([0.70, 0.22, 0.20]) * 0.55 * veins
    path = os.path.join(out_dir, "RealisticSclera.png")
    Image.fromarray((np.clip(rgb[::-1], 0, 1) * 255 + 0.5).astype(np.uint8)).save(path, optimize=True)
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "sRGB"
    return img


def film_texture(out_dir):
    """The occlusion film's gradient: black, dense at the lids' edge (v =
    0), clear by the middle of the eye (v = 1)."""
    from PIL import Image
    w, h = 4, 64
    v = (np.arange(h) + 0.5) / h
    alpha = 0.46 * (1.0 - v) ** 2.2
    px = np.zeros((h, w, 4), dtype=np.uint8)
    px[:, :, 3] = (alpha[::-1, None] * 255 + 0.5).astype(np.uint8)
    path = os.path.join(out_dir, "RealisticEyeOcclusion.png")
    Image.fromarray(px, "RGBA").save(path)
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "sRGB"
    return img


def _material(name, colour, roughness, alpha=1.0, texture=None, texture_alpha=False):
    m = bpy.data.materials.get(name)
    if m is not None:
        return m
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = 0.0
    bsdf.inputs["Base Color"].default_value = (*colour, 1.0)
    bsdf.inputs["Alpha"].default_value = alpha
    if texture is not None:
        tex = m.node_tree.nodes.new("ShaderNodeTexImage")
        tex.image = texture
        tex.extension = "EXTEND"
        m.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
        if texture_alpha:
            m.node_tree.links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
    if alpha < 1.0 or texture_alpha:
        m.surface_render_method = "BLENDED"
    return m


def _object(name, pts, faces, mats, uvs, order, materials):
    me = bpy.data.meshes.new(name)
    me.from_pydata([tuple(p) for p in pts], [], faces)
    layer = me.uv_layers.new(name="UVMap")
    k = 0
    for corner in uvs:
        for uv in corner:
            layer.data[k].uv = uv
            k += 1
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    for material in materials:
        me.materials.append(material)
    for poly, material in zip(me.polygons, mats):
        poly.material_index = order.index(material)
        poly.use_smooth = True
    return obj


def eyes(centres, radius, iris_image, out_dir):
    """The `Eyes` object: both eyeballs at `centres` ({1: left, -1:
    right}) with radius `radius`, each on its eye bone, with the pupil's
    shape."""
    src, src_faces, src_mats, src_uvs, pupil = eyeball(radius)
    pts, faces, mats, uvs, groups, moves = [], [], [], [], {}, []
    for side, s in SIDES:
        base = len(pts)
        mirror = np.array([s, 1.0, 1.0])
        pts += [p * mirror + np.asarray(centres[s]) for p in src]
        moves += [d * mirror for d in pupil]
        for f, uv in zip(src_faces, src_uvs):
            f, uv = (f, uv) if s > 0 else (f[::-1], uv[::-1])
            faces.append(tuple(base + i for i in f))
            uvs.append([(0.5 + (u - 0.5) * s, v) for u, v in uv])
        mats += src_mats
        groups[f"{side}Eye"] = list(range(base, base + len(src)))
    order = ["Sclera", "Iris", "Cornea"]
    materials = [
        _material("Sclera", (1.0, 1.0, 1.0), 0.20, texture=sclera_texture(out_dir)),
        _material("Iris", (1.0, 1.0, 1.0), 0.30, texture=iris_image),
        _material("Cornea", (0.0, 0.0, 0.0), 0.03, alpha=0.06),
    ]
    obj = _object("Eyes", pts, faces, mats, uvs, order, materials)
    for bone, idx in groups.items():
        obj.vertex_groups.new(name=bone).add(idx, 1.0, "REPLACE")
    obj.shape_key_add(name="Basis", from_mix=False)
    key = obj.shape_key_add(name="Pupil_Size", from_mix=False)
    for i, d in enumerate(moves):
        key.data[i].co = pts[i] + d
    key.value = 0.0
    return obj


def follow(face, mix, shape, lids):
    """Offsets of `EyeShell`'s vertices under a body shape ({vertex:
    offset}). With `lids` (a shape that works the lids) a vertex keeps
    its height over the eyeball only in proportion to how far apart the
    lids still are where it is, so a closed eye has the film flat
    between its lids, not standing through them."""
    P = face.P
    at = lambda i: P[i] + shape.get(i, 0.0)  # noqa: E731
    out = np.zeros((len(mix), 3))
    for k, m in enumerate(mix):
        rest = sum(P[i] * w for i, w in m["pieces"])
        moved = sum(at(i) * w for i, w in m["pieces"])
        if not m["lift"]:
            out[k] = moved - rest
            continue
        ball = face.eye[m["side"]].ball
        lifted = ball.lift(rest[None], m["lift"])[0]
        if not lids:
            out[k] = moved - rest
            continue
        gap_rest = np.linalg.norm(P[m["own"]] - sum(P[i] * w for i, w in m["across"]))
        gap = np.linalg.norm(at(m["own"]) - sum(at(i) * w for i, w in m["across"]))
        share = min(1.0, gap / max(gap_rest, 1e-9))
        # (And drawn back toward the eyeball as they meet: nothing of
        # the film or the tear line is left in the closed lids' seam.)
        away = moved - ball.c
        tucked = moved - away / max(np.linalg.norm(away), 1e-9) * TUCK
        out[k] = ball.lift(moved[None], m["lift"] * share)[0] * share + tucked * (1.0 - share) - lifted
    return out


def shell(face, out_dir):
    """The `EyeShell` object for `face` (a `face.Face`): the occlusion
    film and the tear line of both eyes. Returns (the object, `mix`: for
    each vertex the lid vertices it follows as [(vertex, share), ...])."""
    P = face.P
    pts, faces, mats, uvs, mix = [], [], [], [], []
    for _, s in SIDES:
        eye = face.eye[s]
        ring = np.array(eye.loops[1])
        t, h = eye.along(P[ring]), eye.rise(P[ring])
        order = np.argsort(np.arctan2(h - h.mean(), t - t.mean()))
        if s < 0:
            order = order[::-1]
        ring, t = ring[order], t[order]
        count = len(ring)
        upper = np.array([eye.upper[int(i)] for i in ring])

        def partner(k):
            """The point of the other lid's edge across from ring vertex
            `k`, as a mix of two of its vertices."""
            if ring[k] in (eye.canthus_in, eye.canthus_out):
                return [(int(ring[k]), 1.0)]
            other = [j for j in range(count) if upper[j] != upper[k] or ring[j] in (eye.canthus_in, eye.canthus_out)]
            other.sort(key=lambda j: t[j])
            for a, b in zip(other, other[1:]):
                if t[a] <= t[k] <= t[b]:
                    w = (t[k] - t[a]) / max(t[b] - t[a], 1e-9)
                    return [(int(ring[a]), 1.0 - w), (int(ring[b]), w)]
            end = other[0] if t[k] < t[other[0]] else other[-1]
            return [(int(ring[end]), 1.0)]

        room = [float(np.linalg.norm(P[ring[k]] - sum(P[i] * w for i, w in partner(k)))) for k in range(count)]

        def vertex(k, toward, lift):
            pieces = [(int(ring[k]), 1.0 - 0.5 * toward)] + [(i, w * 0.5 * toward) for i, w in partner(k)]
            at = sum(P[i] * w for i, w in pieces)
            if lift:
                at = eye.ball.lift(at[None], lift)[0]
            mix.append({"pieces": pieces, "lift": lift, "own": int(ring[k]), "across": partner(k), "side": s})
            pts.append(at)
            return len(pts) - 1

        def strip(rings, values, material):
            for (ra, va), (rb, vb) in zip(zip(rings, values), zip(rings[1:], values[1:])):
                for k in range(count):
                    j = (k + 1) % count
                    # None in the corners, where the lids hardly part: a
                    # film there has nowhere to go when they close.
                    if min(room[k], room[j]) < CORNER:
                        continue
                    faces.append((ra[k], ra[j], rb[j], rb[k]))
                    mats.append(material)
                    uvs.append([(k / count, va), ((k + 1) / count, va), ((k + 1) / count, vb), (k / count, vb)])

        film = [[vertex(k, toward, lift) for k in range(count)] for toward, lift in FILM_RINGS]
        strip(film, [toward for toward, _ in FILM_RINGS], "EyeOcclusion")
        tear = [[vertex(k, 0.0, 0.00012) for k in range(count)], [vertex(k, TEAR_REACH, 0.00006) for k in range(count)]]
        strip(tear, [0.0, 1.0], "TearLine")
    order = ["EyeOcclusion", "TearLine"]
    materials = [
        _material("EyeOcclusion", (0.0, 0.0, 0.0), 0.9, texture=film_texture(out_dir), texture_alpha=True),
        _material("TearLine", (1.0, 1.0, 1.0), 0.04, alpha=0.22),
    ]
    obj = _object("EyeShell", pts, faces, mats, uvs, order, materials)
    obj.vertex_groups.new(name="Head").add(list(range(len(pts))), 1.0, "REPLACE")
    return obj, mix
