"""Lashes and brows of the realistic head, as cards (Character Studio
overhaul CO6): thin strips with hair drawn on them and cut out by the
texture's alpha, the form facial hair takes on every GPU (ADR-0037).

- `Lashes`: for each lid a strip rooted on the lid's outer edge (the
  lash line) between the corners of the eye, bent once along its length
  so the lashes curl. Its roots are the lid's own vertices; the rest of
  a lash turns with its root about the eyeball when a shape moves the
  lid, and is kept off the skin under every shape (`off_skin`).
- `Brows`: a strip laid on the skin along each brow, each vertex bound
  to the point of the skin under it, so it follows every shape that
  moves the brow; no face of it comes nearer the skin than `BROW_CLEAR`
  at rest or under a shape (`over_skin`).

Both follow the body by blend shapes of the body's shapes' names
(`follow`), which is how the engine drives a part (architecture 14).
"""

import math
import os

import bpy
import numpy as np

from facetopo import SIDES, nearest, rotate_about

# Lash length at its longest (metres), upper and lower lid.
LASH = {True: 0.0088, False: 0.0042}
# The brow card's segments along and across, and how far it floats over
# the skin (at its vertices; nowhere nearer than `BROW_CLEAR`). Its height
# is each frame's (`BROW_LOOKS`).
BROW_SEGMENTS = 22
BROW_ROWS = 5
BROW_LIFT = 0.0008
BROW_CLEAR = 0.0003
HAIR = (0.020, 0.013, 0.009)
# The brows' hair (wave 3, R4's second pass): coloured by melanin, not a
# fixed near-black. A hair fibre's absorption is the sum of its two
# melanins' (d'Eon and others 2011, the coefficients per millimetre of
# concentration below), and the colour a fibre shows from its many
# scatterings is Chiang and others' 2016 mapping, c = exp(-k sqrt(sigma)),
# k for an azimuthal roughness of 0.3, plus the light the fibres' surface
# reflects white (keratin, about 1.2% averaged over a dark brow's
# directions). Eumelanin 0.8 and pheomelanin 0.15 give a dark brown,
# about sRGB (61, 46, 36); each hair's eumelanin varies about it.
EUMELANIN_ABSORBS = (0.419, 0.697, 1.37)
PHEOMELANIN_ABSORBS = (0.187, 0.4, 1.05)
CHIANG_K = 5.888
FIBRE_SHEEN = 0.012
BROW_EUMELANIN = 0.8
BROW_PHEOMELANIN = 0.15
BROW_MELANIN_SPREAD = 0.25
# Lashes grow from the lid's outer edge between the corners of the eye,
# not within this of a corner (metres): the edge's vertices there run
# round the corner, and a strip through them stood up in zigzags.
LASH_FROM_CORNER = 0.0006
# How far a closed upper lid's lashes go from curled to straight.
LASH_STRAIGHTENS = 0.7
# How many times its lid's turn an upper lid's lashes turn as it comes down
# (a closed lid's point about 45 degrees down: seen from the front, a
# fringe and not a card on edge).
LASH_TURNS_DOWN = 1.3
# How much of an upper lid's upward turn its lashes take.
LASH_TURNS_UP = 0.3
# A lash keeps this far off the skin under every shape (metres).
LASH_OFF_SKIN = 0.0004


def _strands(path, size, count, seed, draw_one, most=1.0, gain=1.6):
    from PIL import Image, ImageDraw, ImageFilter
    rng = np.random.default_rng(seed)
    scale = 4
    w, h = size[0] * scale, size[1] * scale
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    for _ in range(count):
        draw_one(draw, rng, w, h, scale)
    mask = mask.filter(ImageFilter.GaussianBlur(scale * 0.45)).resize(size, Image.LANCZOS)
    alpha = np.asarray(mask, dtype=np.float32) / 255.0
    alpha = np.clip(alpha * gain, 0.0, 1.0) * most
    px = np.zeros((size[1], size[0], 4), dtype=np.uint8)
    px[:, :, :3] = (np.array(HAIR) ** (1 / 2.2) * 255 + 0.5).astype(np.uint8)
    px[:, :, 3] = (alpha * 255 + 0.5).astype(np.uint8)
    Image.fromarray(px, "RGBA").save(path, optimize=True)
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "sRGB"
    return img


def lash_texture(out_dir):
    """Lashes from the lid (the image's bottom) to their tips: curved,
    thinning, of uneven length, some in pairs."""
    def one(draw, rng, w, h, scale):
        # A lash: thick at the lid, a point at its tip, leaning with its
        # neighbours (lashes stand in tufts) and a little on its own.
        x = rng.uniform(0.01, 0.99) * w
        length = rng.uniform(0.45, 0.98) * h
        lean = (0.07 * math.sin(x / w * 61.0) + rng.normal(0.0, 0.06)) * h
        steps = 12
        last = (x, h - 1)
        for k in range(1, steps + 1):
            t = k / steps
            at = (x + lean * t * t, h - 1 - length * t)
            draw.line([last, at], fill=255, width=max(1, int(round(scale * (4.6 - 3.4 * t)))))
            last = at

    def fringe(draw, rng, w, h, scale):
        # The lash line itself, a thin dark edge: where the lashes leave
        # the lid they stand too close to tell apart. (Kept low: a tall
        # fringe made a closed eye's lashes one grey band.)
        x = rng.uniform(0.0, 1.0) * w
        draw.line([(x, h - 1), (x + rng.normal(0, 0.006) * w, h - 1 - rng.uniform(0.05, 0.10) * h)], fill=255,
                  width=int(scale * 4))

    def both(draw, rng, w, h, scale):
        one(draw, rng, w, h, scale)
        for _ in range(2):
            fringe(draw, rng, w, h, scale)

    return _strands(os.path.join(out_dir, "RealisticLashes.png"), (512, 128), 120, 5, both, gain=2.6)


def hair_colour(eumelanin, pheomelanin=0.0):
    """A hair's colour (linear RGB) from its melanins (`EUMELANIN_ABSORBS`)."""
    sigma = eumelanin * np.array(EUMELANIN_ABSORBS) + pheomelanin * np.array(PHEOMELANIN_ABSORBS)
    return np.exp(-CHIANG_K * np.sqrt(sigma)) + FIBRE_SHEEN


# The brow's hair on its card (texture shares: u from the inner end 0 to
# the tail 1, v from the card's lower edge 0 to its upper edge 1). The
# roots lie in a band about `BROW_BAND_MIDDLE`, as wide as `brow_band`
# says along it; hairs grow up and out at the inner end and lie over
# toward the tail further on, those from the band's lower half up and
# those from its upper half a little down, so they meet along the middle
# (the herringbone of a real brow). Density, by `brow_density`: sparse at
# the inner end, densest over the first half, thinning to the tail, and
# falling off softly to the band's edges, where single hairs stand alone.
BROW_BAND_MIDDLE = 0.47
BROW_HAIRS = 6000
BROW_TEXTURE = (1024, 256)


def brow_band(u):
    """The roots' half-width (a share of the card's height) at `u`: a
    rounded head, the full body, a tail drawn to a point."""
    head = 0.55 + 0.45 * min(1.0, max(0.0, u) / 0.14) ** 0.6
    tail = (1.0 - min(1.0, max(0.0, u - 0.50) / 0.55)) ** 0.85
    return 0.26 * head * max(tail, 0.04)


def brow_density(u):
    """How densely the hair grows at `u`, 0 to 1."""
    if u < 0.0 or u > 1.0:
        return 0.0
    rise = 0.35 + 0.65 * min(1.0, u / 0.20)
    fall = 1.0 - max(0.0, u - 0.62) / 0.45
    return rise * max(fall, 0.0) ** 1.3


def brow_texture(out_dir):
    """A brow from its inner end (the image's left) to its tail, hair by
    hair (`brow_band`, `brow_density`), each hair a tapering curve with
    its own melanin; drawn four times over and filtered down, so a hair
    is about one texel wide and the alpha is graded."""
    from PIL import Image, ImageDraw, ImageFilter
    rng = np.random.default_rng(11)
    scale = 4
    w, h = BROW_TEXTURE[0] * scale, BROW_TEXTURE[1] * scale
    colour = Image.new("RGB", (w, h), (0, 0, 0))
    mask = Image.new("L", (w, h), 0)
    hit = Image.new("L", (w, h), 0)
    paint, cut, mark = ImageDraw.Draw(colour), ImageDraw.Draw(mask), ImageDraw.Draw(hit)
    drawn = 0
    while drawn < BROW_HAIRS:
        u = rng.uniform(-0.02, 1.0)
        if rng.uniform() > brow_density(u):
            continue
        band = brow_band(u)
        # Roots across the band: most near the middle, a few out at its
        # edges (a soft edge, single hairs).
        across = float(np.clip(rng.normal(0.0, 0.55), -1.6, 1.6))
        if abs(across) > 1.0 and rng.uniform() > 0.35:
            continue
        v = BROW_BAND_MIDDLE + across * band
        drawn += 1
        # The hair's direction (degrees from along the brow toward the
        # tail, up positive): up at the inner end, lying over by a third
        # of the way; from the band's lower half up into the middle, from
        # its upper half down onto it.
        lie = min(1.0, max(0.0, u) / 0.32)
        angle = 78.0 * (1.0 - lie) + (16.0 if across < 0 else -10.0) * lie + rng.normal(0.0, 8.0)
        # (Shorter at the inner end, where they stand up, and at the tail.)
        length = rng.uniform(0.045, 0.085) * (0.6 + 0.4 * lie) * (1.0 - 0.45 * max(0.0, u - 0.6) / 0.4) * w
        bend = rng.normal(0.0, 0.15) - 0.2 * lie
        tint = hair_colour(BROW_EUMELANIN * float(np.exp(rng.normal(0.0, BROW_MELANIN_SPREAD))),
                           BROW_PHEOMELANIN)
        rgb = tuple(int(c) for c in np.clip(np.power(tint, 1 / 2.2) * 255 + 0.5, 0, 255))
        x, y = u * w, (1.0 - v) * h
        a = math.radians(angle)
        steps = 8
        last = (x, y)
        for k in range(1, steps + 1):
            t = k / steps
            turn = a + bend * t
            # (Image rows run down: up on the card is minus y.)
            at = (last[0] + math.cos(turn) * length / steps, last[1] - math.sin(turn) * length / steps)
            # Thick at the root, a point at the tip, darker at the root.
            width = max(1, int(round(scale * (1.15 - 0.75 * t))))
            strength = int(255 * (0.95 - 0.35 * t))
            paint.line([last, at], fill=rgb, width=width)
            cut.line([last, at], fill=strength, width=width)
            mark.line([last, at], fill=255, width=width)
            last = at
    size = BROW_TEXTURE
    # (The colour where no hair is drawn takes its neighbours', so the
    # filtered edges don't darken toward black.)
    alpha = np.asarray(mask.filter(ImageFilter.GaussianBlur(scale * 0.35)).resize(size, Image.LANCZOS),
                       dtype=np.float32) / 255.0
    rgb = np.asarray(colour.resize(size, Image.BOX), dtype=np.float32)
    cover = np.asarray(hit.resize(size, Image.BOX), dtype=np.float32)[:, :, None] / 255.0
    plain = (np.power(hair_colour(BROW_EUMELANIN, BROW_PHEOMELANIN), 1 / 2.2) * 255).astype(np.float32)
    rgb = np.where(cover > 0.02, rgb / np.maximum(cover, 0.02), plain)
    px = np.zeros((size[1], size[0], 4), dtype=np.uint8)
    px[:, :, :3] = np.clip(rgb + 0.5, 0, 255).astype(np.uint8)
    px[:, :, 3] = np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)
    path = os.path.join(out_dir, "RealisticBrows.png")
    Image.fromarray(px, "RGBA").save(path, optimize=True)
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "sRGB"
    return img


def _material(name, texture, cut=True, matte=False):
    """Hair drawn on a card: blended by the texture's alpha, or with
    `cut` cut out by it (glTF alphaMode MASK; not used: see `lashes`).
    `matte`: a card shaded as a smooth dielectric reflects 4% of the sky
    whatever its colour, which on a card this dark is most of its light
    (a closed eye's lashes read grey, and a raised brow, turned up to
    the sky, went pale). As a rough metal the card reflects only its own
    colour: dark, without sheen."""
    m = bpy.data.materials.get(name)
    if m is not None:
        return m
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 1.0 if matte else 0.55
    bsdf.inputs["Metallic"].default_value = 1.0 if matte else 0.0
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = texture
    tex.extension = "EXTEND"
    nt.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    if cut:
        step = nt.nodes.new("ShaderNodeMath")
        step.operation = "GREATER_THAN"
        step.inputs[1].default_value = 0.33
        nt.links.new(tex.outputs["Alpha"], step.inputs[0])
        nt.links.new(step.outputs[0], bsdf.inputs["Alpha"])
    else:
        nt.links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
        m.surface_render_method = "BLENDED"
    m.use_backface_culling = False
    return m


def _object(name, pts, faces, uvs, material):
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
    me.materials.append(material)
    for poly in me.polygons:
        poly.use_smooth = True
    obj.vertex_groups.new(name="Head").add(list(range(len(pts))), 1.0, "REPLACE")
    return obj


def lashes(face, out_dir):
    """The `Lashes` object for `face`. Returns (the object, `roots`: for
    each vertex (the lid vertex it grows from, its eye's side))."""
    P = face.P
    up = np.array([0.0, 0.0, 1.0])
    pts, faces, uvs, roots = [], [], [], []
    for _, s in SIDES:
        eye = face.eye[s]
        for upper in (True, False):
            loop = [i for i in eye.loops[2] if eye.upper[i] == upper
                    and eye.t_in + LASH_FROM_CORNER < float(eye.along(P[i])[0]) < eye.t_out - LASH_FROM_CORNER]
            loop.sort(key=lambda i: float(eye.along(P[i])[0]))
            t = np.array([float(eye.along(P[i])[0]) for i in loop])
            share = np.clip((t - eye.t_in) / (eye.t_out - eye.t_in), 0.0, 1.0)
            rows = []
            for i, u in zip(loop, share):
                # Longest past the middle, short at the strip's ends.
                length = LASH[upper] * (0.04 + 0.96 * max(0.0, math.sin(math.pi * min(1.0, u ** 0.8))) ** 0.7)
                radial = P[i] - eye.c
                radial /= np.linalg.norm(radial)
                fan = eye.axis * ((u - 0.42) * 0.9)
                if upper:
                    first = 0.35 * radial + 0.75 * eye.fwd + fan
                    second = first / np.linalg.norm(first) + 0.9 * up
                else:
                    # (Out of the lid along the normal of the skin just
                    # under its edge, which slopes forward: a lash set by
                    # the eye's axes alone lay in that skin.)
                    below = [j for j in face.topo.nbr[i] if eye.step.get(j) == 3] or [i]
                    out = face.N[below].mean(axis=0)
                    first = 0.80 * out / np.linalg.norm(out) + 0.25 * eye.fwd + fan - 0.45 * up
                    second = first / np.linalg.norm(first) - 0.25 * up
                first /= np.linalg.norm(first)
                second /= np.linalg.norm(second)
                root = P[i] + radial * 0.00005
                mid = root + first * length * 0.55
                rows.append((root, mid, mid + second * length * 0.5, i, u))
            base = len(pts)
            for root, mid, tip, i, _ in rows:
                pts += [root, mid, tip]
                roots += [(int(i), s)] * 3
            for k in range(len(rows) - 1):
                a, b = base + 3 * k, base + 3 * (k + 1)
                ua, ub = rows[k][4], rows[k + 1][4]
                for row, (v0, v1) in enumerate(((0.0, 0.55), (0.55, 1.0))):
                    quad = (a + row, b + row, b + row + 1, a + row + 1)
                    uv = [(ua, v0), (ub, v0), (ub, v1), (ua, v1)]
                    if (s > 0) != upper:
                        quad, uv = quad[::-1], uv[::-1]
                    faces.append(quad)
                    uvs.append(uv)
    # Blended, not cut out: in the engine a cut-out card on the face
    # shows the backdrop through the skin behind its clear parts (seen
    # as a grey band under the eye in CO6's first captures).
    # No lash lies in the skin at rest.
    pts = list(off_skin(face, face.P, np.array(pts), roots))
    obj = _object("Lashes", pts, faces, uvs, _material("Lashes", lash_texture(out_dir), cut=False, matte=True))
    return obj, roots


def off_skin(face, P, points, roots, moved=None):
    """`points` (the lashes' vertices) with those that lie in or on the
    head's skin at positions `P` moved out to `LASH_OFF_SKIN` over it. A
    root (the first of each three) stays on its lid; only vertices whose
    root is in `moved` (or all) are looked at."""
    from mathutils import Vector
    from facetopo import FS_HEAD, FS_MUZZLE
    outer = {k for k, fs in enumerate(face.topo.sets) if fs in (FS_HEAD, FS_MUZZLE)}
    tree = face.topo.bvh(P, outer)
    out = np.array(points, dtype=np.float64)
    for k in range(len(out)):
        if k % 3 == 0 or (moved is not None and roots[k][0] not in moved):
            continue
        loc, normal, _, _ = tree.find_nearest(Vector(out[k]), 0.006)
        if loc is None:
            continue
        height = float((Vector(out[k]) - loc).dot(normal))
        if height < LASH_OFF_SKIN:
            out[k] = np.array(loc) + np.array(normal) * LASH_OFF_SKIN
    return out


def follow_lashes(face, points, roots, shape, lids=True):
    """Offsets of the lashes' vertices under a body shape ({vertex:
    offset}): each moves with its root and, under a shape that works the
    lids (`lids`), turns about the eyeball by as much as its root does."""
    P = face.P
    out = np.zeros((len(points), 3))
    for k, (i, s) in enumerate(roots):
        d = shape.get(i)
        if d is None:
            continue
        if not lids:
            out[k] = d
            continue
        eye = face.eye[s]
        before, after = P[i], P[i] + d
        reach = max(float(eye.reach(before[None])[0]), 1e-6)
        angle = (math.asin(max(-1.0, min(1.0, float(eye.rise(after[None])[0]) / reach)))
                 - math.asin(max(-1.0, min(1.0, float(eye.rise(before[None])[0]) / reach))))
        if angle > 0 and eye.upper[i]:
            # An upper lid drawn up slides under the fold above it; its
            # lashes keep most of their own direction (turned all the
            # way with the lid they lay flat on the fold).
            angle *= LASH_TURNS_UP
        elif angle < 0 and eye.upper[i]:
            # An upper lid coming down turns its lashes down a little
            # further than itself, so a closed lid's are seen from the
            # front as a fringe. (Not much further: on the straight path
            # to there a half-shut eye's lashes would hang across the
            # opening.)
            angle *= LASH_TURNS_DOWN
        turned = rotate_about(points[k][None], eye.c, eye.turn_axis, angle)[0]
        carried = rotate_about(before[None], eye.c, eye.turn_axis, angle)[0]
        out[k] = turned + (after - carried) - points[k]
        if k % 3 == 2 and angle < 0 and eye.upper[i]:
            # An upper lid coming down lays its lashes out straighter: with
            # their full curl a closed lid's lashes point their outer halves
            # straight at the viewer, and the fringe ends in a cut line.
            root, mid, tip = (points[j] + out[j] for j in (k - 2, k - 1, k))
            length = np.linalg.norm(tip - mid)
            along, curled = (mid - root) / max(np.linalg.norm(mid - root), 1e-9), (tip - mid) / max(length, 1e-9)
            way = curled + (along - curled) * (LASH_STRAIGHTENS * min(1.0, -angle / 0.9))
            out[k] = mid + way / np.linalg.norm(way) * length - points[k]
    if lids and any(i in shape for i, _ in roots):
        # And off the skin as the shape leaves it: an upper lid drawn up
        # (eyes wide, a raised brow) turns its lashes into the fold above
        # it, a lower lid pushed up turns its own into the lid.
        posed = P.copy()
        for i, d in shape.items():
            posed[i] += d
        out = off_skin(face, posed, points + out, roots, moved=shape) - points
    return out


# How far along the brow's line its card runs (0 the inner end, 1 the
# tail), and where the tail ends at each end of the brows' length
# parameter (`Face_Brow_Length_*`): drawn in, or carried on down the
# temple.
BROW_START = -0.20
BROW_END = 1.38
BROW_END_SHORT = 1.06
BROW_END_LONG = 1.60
# Each frame's brows (wave 3, R4's second pass; references/face/README.md,
# F-29): the card's half-height at its head (`half`, metres) and the share
# of it left at the tail (`tail`); where the tail ends along the brow's
# line (`end`); how far the card's middle is raised along the card's
# height at the arch (`arch`, about 62% along) and at the tail
# (`tail_up`), and how far the whole card is lowered (`drop`), metres.
# The masculine brow is fuller and flatter (its arch
# taken down a little); the feminine one thinner, shorter (F-29: 61 mm
# along its lower edge against 70) and more arched, its tail higher (the
# brows' lower edge stands 2.6 mm higher over the outer eye corner in
# women, about the same over the inner one). The feminine card is the
# brows' `Frame_Feminine` shape's own part, beside the skin's move.
BROW_LOOKS = {
    "masculine": {"half": 0.0080, "tail": 0.50, "end": BROW_END, "arch": -0.0006, "tail_up": 0.0, "drop": 0.0},
    "feminine": {"half": 0.0072, "tail": 0.38, "end": 1.26, "arch": 0.0016, "tail_up": 0.0018, "drop": 0.0025},
}


def _look_at(look, u):
    """(The card's half-height, its middle's lift) at `u` along the card
    (0 its inner end, 1 its tail) for `look` (`BROW_LOOKS`)."""
    half = look["half"] * (1.0 - (1.0 - look["tail"]) * smoothstep_scalar(0.40, 1.05, u))
    lift = look["arch"] * math.exp(-0.5 * ((u - 0.62) / 0.24) ** 2) \
        + look["tail_up"] * smoothstep_scalar(0.35, 1.0, u) - look["drop"]
    return half, lift


def smoothstep_scalar(e0, e1, x):
    t = min(1.0, max(0.0, (x - e0) / (e1 - e0)))
    return t * t * (3.0 - 2.0 * t)


def _skin_tree(face):
    """The head's outer skin as (BVH tree, its triangles)."""
    from mathutils.bvhtree import BVHTree
    from facetopo import FS_HEAD
    P, topo = face.P, face.topo
    outer = [k for k, fs in enumerate(topo.sets) if fs == FS_HEAD]
    tris = []
    for k in outer:
        p = topo.polys[k]
        tris += [(p[0], p[j], p[j + 1]) for j in range(1, len(p) - 1)]
    return BVHTree.FromPolygons([tuple(p) for p in P], tris, all_triangles=True), tris


def _brow_card(face, tree, tris, ends, look=None):
    """The brows' card laid on the skin: (points, faces, uvs, bound), with
    each side's tail at `ends[s]` of the way along its brow's line, shaped
    as `look` says (`BROW_LOOKS`; the masculine one by default)."""
    from mathutils import Vector
    look = look or BROW_LOOKS["masculine"]
    P = face.P
    pts, faces, uvs, bound = [], [], [], []
    rows = np.linspace(-1.0, 1.0, BROW_ROWS)
    for _, s in SIDES:
        # From a little before the inner end to a little past the tail.
        curve = face.brow_curve(s, BROW_SEGMENTS, BROW_START, ends.get(s, look["end"]))
        base = len(pts)
        for k, at in enumerate(curve):
            ahead = curve[min(k + 1, BROW_SEGMENTS)] - curve[max(k - 1, 0)]
            ahead /= np.linalg.norm(ahead)
            hit = nearest(tree, at, 0.05)
            across = np.cross(ahead, hit[1]) * (1.0 if s < 0 else -1.0)
            if across[2] < 0:
                across = -across
            across /= np.linalg.norm(across)
            half, lift = _look_at(look, k / BROW_SEGMENTS)
            for row in rows:
                loc, normal, index, _ = tree.find_nearest(Vector(at + across * (lift + row * half)), 0.05)
                tri = tris[index]
                va, vb, vc = (P[i] for i in tri)
                v0, v1, v2 = vb - va, vc - va, np.array(loc) - va
                d00, d01, d11, d20, d21 = v0 @ v0, v0 @ v1, v1 @ v1, v2 @ v0, v2 @ v1
                den = d00 * d11 - d01 * d01
                v = (d11 * d20 - d01 * d21) / den
                w = (d00 * d21 - d01 * d20) / den
                bound.append((tri, (1.0 - v - w, v, w)))
                pts.append(np.array(loc) + np.array(normal) * BROW_LIFT)
        for k in range(BROW_SEGMENTS):
            for row in range(BROW_ROWS - 1):
                i0, i1 = base + BROW_ROWS * k + row, base + BROW_ROWS * (k + 1) + row
                quad = (i0, i1, i1 + 1, i0 + 1)
                u0, u1 = k / BROW_SEGMENTS, (k + 1) / BROW_SEGMENTS
                v0, v1 = row / (BROW_ROWS - 1), (row + 1) / (BROW_ROWS - 1)
                uv = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
                if s < 0:
                    quad, uv = quad[::-1], uv[::-1]
                faces.append(quad)
                uvs.append(uv)
    return np.array(over_skin(tree, np.array(pts), faces)), faces, uvs, bound


def brows(face, out_dir):
    """The `Brows` object for `face`. Returns (the object, `bound`: for
    each vertex the skin it lies on as ([three vertices], [their shares]))."""
    tree, tris = _skin_tree(face)
    pts, faces, uvs, bound = _brow_card(face, tree, tris, {})
    obj = _object("Brows", list(pts), faces, uvs, _material("Brows", brow_texture(out_dir), cut=False, matte=True))
    return obj, bound


def feminine_brows(face, bound, rest, faces, frame):
    """The brows' `Frame_Feminine` shape: the skin's move under the card
    (`follow_bound` of the frame's shape, `frame`) and the feminine card's
    own shape (`BROW_LOOKS`), its difference from the masculine card laid
    on the same skin, kept over the feminine skin."""
    from facetopo import FS_HEAD
    tree, tris = _skin_tree(face)
    own = _brow_card(face, tree, tris, {}, BROW_LOOKS["feminine"])[0] - rest
    moved = follow_bound(face, bound, rest, faces, frame)
    posed = face.P.copy()
    for i, d in frame.items():
        posed[i] += d
    skin = face.topo.bvh(posed, {k for k, fs in enumerate(face.topo.sets) if fs == FS_HEAD})
    return over_skin(skin, rest + moved + own, faces) - rest


def brow_measures(card, alpha, corners):
    """Where the brows' hair lies against the eyes, as F-29 measures it
    (metres): the lower edge of the hair (where a column of the texture
    first reaches 30% of its own most, coming up from the card's lower
    edge, in columns holding hair) over the inner and the outer eye
    corner (`corners[s]`: (inner, outer) positions), its length along
    that edge, and the card's height at its head, its arch and its tail.
    `card` is the card's vertices on a frame, `alpha` the texture's alpha
    (rows from the card's upper edge down, as the image is). The mean of
    the two sides."""
    h, w = alpha.shape
    per_side = (BROW_SEGMENTS + 1) * BROW_ROWS
    vs = np.linspace(0.0, 1.0, BROW_ROWS)
    window = max(2, int(0.015 * w))
    profiles = []
    for j in range(BROW_SEGMENTS + 1):
        x = int(round(j / BROW_SEGMENTS * (w - 1)))
        # Rows from the card's lower edge (v 0) up.
        profiles.append(alpha[::-1, max(0, x - window):x + window + 1].mean(axis=1))
    most = max(float(p.max()) for p in profiles)
    out = {"over_inner_corner_mm": [], "over_outer_corner_mm": [], "length_mm": [], "height_mm": []}
    for k, (_, s) in enumerate(SIDES):
        rows = card[k * per_side:(k + 1) * per_side].reshape(BROW_SEGMENTS + 1, BROW_ROWS, 3)
        edge = []
        for j, profile in enumerate(profiles):
            if profile.max() < 0.15 * most:
                continue
            v = float(np.argmax(profile >= 0.3 * profile.max())) / (h - 1)
            edge.append(np.array([np.interp(v, vs, rows[j, :, c]) for c in range(3)]))
        edge = np.array(edge)
        for key, corner in zip(("over_inner_corner_mm", "over_outer_corner_mm"), corners[s]):
            j = int(np.argmin(np.abs(np.abs(edge[:, 0]) - abs(float(corner[0])))))
            out[key].append(float(edge[j, 2] - corner[2]))
        out["length_mm"].append(float(np.linalg.norm(np.diff(edge, axis=0), axis=1).sum()))
        tall = np.linalg.norm(rows[:, -1] - rows[:, 0], axis=1)
        out["height_mm"].append([float(tall[1]), float(tall[int(0.62 * BROW_SEGMENTS)]), float(tall[-2])])
    report = {key: round(float(np.mean(v)) * 1000, 2) for key, v in out.items() if key != "height_mm"}
    report["height_mm"] = [round(float(x) * 1000, 2) for x in np.mean(out["height_mm"], axis=0)]
    return report


def brow_lengths(face, rest):
    """The brows' length shapes ({name: offsets per card vertex}), for the
    `brow_length_*` parameters: each side's card with its tail drawn in to
    `BROW_END_SHORT` (`Face_Brow_Length_<side>`, made as the way back out,
    so it is driven from 0 to -1 as the brows' lowering shapes are) and
    carried on to `BROW_END_LONG` (`Face_Brow_Length_Long_<side>`). Only
    the card moves: the hair's texture runs the card's whole length, so a
    shorter brow is a whole brow, shorter, and a longer one tapers further
    down the temple. `rest` is the card as built."""
    tree, tris = _skin_tree(face)
    out = {}
    per_side = (BROW_SEGMENTS + 1) * BROW_ROWS
    for k, (side, s) in enumerate(SIDES):
        mine = np.zeros((len(rest), 1))
        mine[k * per_side:(k + 1) * per_side] = 1.0
        short = _brow_card(face, tree, tris, {s: BROW_END_SHORT})[0]
        long = _brow_card(face, tree, tris, {s: BROW_END_LONG})[0]
        out[f"Face_Brow_Length_{side}"] = (rest - short) * mine
        out[f"Face_Brow_Length_Long_{side}"] = (long - rest) * mine
    return out


def over_skin(tree, points, faces, rounds=6):
    """`points` (a card's vertices) raised where the card's faces come
    nearer the skin in `tree` than `BROW_CLEAR`: a flat face between
    vertices on a rounded brow cuts under it, and the skin showed through
    the hair as light holes. Tried at each face's middle and edges."""
    from mathutils import Vector
    out = np.array(points, dtype=np.float64)
    spots = ((0.25, 0.25, 0.25, 0.25), (0.5, 0.5, 0.0, 0.0), (0.0, 0.5, 0.5, 0.0), (0.0, 0.0, 0.5, 0.5),
             (0.5, 0.0, 0.0, 0.5), (0.5, 0.0, 0.5, 0.0), (0.0, 0.5, 0.0, 0.5))
    for _ in range(rounds):
        low = False
        for face in faces:
            corners = list(face)
            for shares in spots:
                at = sum(out[i] * w for i, w in zip(corners, shares))
                loc, normal, _, _ = tree.find_nearest(Vector(at), 0.01)
                if loc is None:
                    continue
                short = BROW_CLEAR - float((Vector(at) - loc).dot(normal))
                if short > 1e-6:
                    low = True
                    for i, w in zip(corners, shares):
                        if w:
                            out[i] += np.array(normal) * short * 1.2
        if not low:
            break
    return out


def follow_bound(face, bound, points, faces, shape):
    """Offsets of the brows' vertices under a body shape: each moves as
    the point of the skin it lies on does (hair is rooted in the skin;
    the brow's shapes move the skin under the card nearly as one, so the
    card is not pulled apart), and is then kept over the skin as the
    shape leaves it."""
    out = np.zeros((len(bound), 3))
    for k, (tri, shares) in enumerate(bound):
        for i, w in zip(tri, shares):
            d = shape.get(int(i))
            if d is not None:
                out[k] += w * d
    if not np.abs(out).max() > 1e-7:
        return out
    from facetopo import FS_HEAD
    posed = face.P.copy()
    for i, d in shape.items():
        posed[i] += d
    tree = face.topo.bvh(posed, {k for k, fs in enumerate(face.topo.sets) if fs == FS_HEAD})
    return over_skin(tree, points + out, faces) - points


def add_shapes(obj, shapes, least=2e-5):
    """Blend shapes on `obj` from {name: offsets per vertex}; a shape that
    moves nothing is left out. Returns the names added."""
    if obj.data.shape_keys is None:
        obj.shape_key_add(name="Basis", from_mix=False)
    added = []
    rest = np.array([v.co[:] for v in obj.data.vertices])
    for name, offsets in shapes.items():
        if float(np.abs(offsets).max()) < least:
            continue
        key = obj.shape_key_add(name=name, from_mix=False)
        for i in np.nonzero(np.linalg.norm(offsets, axis=1) > 1e-7)[0]:
            key.data[i].co = rest[i] + offsets[i]
        key.value = 0.0
        added.append(name)
    return added
