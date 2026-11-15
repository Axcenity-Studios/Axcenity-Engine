"""Lashes and brows of the realistic head, as cards (Character Studio
overhaul CO6): thin strips with hair drawn on them and cut out by the
texture's alpha, the form facial hair takes on every GPU (ADR-0037).

- `Lashes`: for each lid a strip rooted on the lid's outer edge (the
  lash line), bent once along its length so the lashes curl. Its roots
  are the lid's own vertices; the rest of a lash turns with its root
  about the eyeball when a shape moves the lid.
- `Brows`: a strip laid on the skin along each brow, each vertex bound
  to the skin under it, so it follows every shape that moves the brow.

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
# Half the brow card's height, its segments along and how far it floats
# over the skin.
BROW_HALF = 0.0078
BROW_SEGMENTS = 14
BROW_LIFT = 0.0005
HAIR = (0.020, 0.013, 0.009)


def _strands(path, size, count, seed, draw_one, most=1.0):
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
    alpha = np.clip(alpha * 1.6, 0.0, 1.0) * most
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
        x = rng.uniform(0.01, 0.99) * w
        length = rng.uniform(0.55, 0.98) * h
        lean = rng.normal(0.0, 0.05) * h
        steps = 10
        last = (x, h - 1)
        for k in range(1, steps + 1):
            t = k / steps
            at = (x + lean * t * t, h - 1 - length * t)
            draw.line([last, at], fill=255, width=max(1, int(round(scale * (4.2 - 3.0 * t)))))
            last = at

    def fringe(draw, rng, w, h, scale):
        # The lash line itself: where the lashes leave the lid they
        # stand too close to tell apart.
        x = rng.uniform(0.0, 1.0) * w
        draw.line([(x, h - 1), (x + rng.normal(0, 0.01) * w, h - 1 - rng.uniform(0.16, 0.30) * h)], fill=255,
                  width=int(scale * 5))

    def both(draw, rng, w, h, scale):
        one(draw, rng, w, h, scale)
        for _ in range(3):
            fringe(draw, rng, w, h, scale)

    return _strands(os.path.join(out_dir, "RealisticLashes.png"), (256, 64), 110, 5, both)


def brow_texture(out_dir):
    """A brow from its inner end (the image's left) to its tail: hairs
    growing up at the inner end and lying over toward the tail, thick in
    the first two thirds, thinning out."""
    def one(draw, rng, w, h, scale):
        u = rng.uniform(0.02, 0.98) ** 0.9
        thick = 0.40 * (1.0 - 0.70 * max(0.0, u - 0.3) / 0.7) * min(1.0, 0.45 + u * 4.0)
        centre = 0.52 - 0.10 * math.sin(u * math.pi * 0.9)
        v = centre + rng.uniform(-1.0, 1.0) * thick
        angle = math.radians(80.0 - 78.0 * min(1.0, u * 1.35) + rng.normal(0.0, 9.0))
        if v < centre:
            angle = -angle * 0.45
        length = rng.uniform(0.10, 0.20) * h * 2.2
        x0, y0 = u * w, v * h
        x1, y1 = x0 + math.cos(angle) * length, y0 - math.sin(angle) * length
        draw.line([(x0, y0), (x1, y1)], fill=255, width=max(1, int(scale * 2.2)))
    return _strands(os.path.join(out_dir, "RealisticBrows.png"), (512, 128), 1600, 9, one, most=0.95)


def _material(name, texture, cut=True):
    """Hair drawn on a card: blended by the texture's alpha, or with
    `cut` cut out by it (glTF alphaMode MASK; not used: see `lashes`)."""
    m = bpy.data.materials.get(name)
    if m is not None:
        return m
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.55
    bsdf.inputs["Metallic"].default_value = 0.0
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
            loop = [i for i in eye.loops[2] if eye.upper[i] == upper]
            loop.sort(key=lambda i: float(eye.along(P[i])[0]))
            t = np.array([float(eye.along(P[i])[0]) for i in loop])
            share = np.clip((t - eye.t_in) / (eye.t_out - eye.t_in), 0.0, 1.0)
            rows = []
            for i, u in zip(loop, share):
                # Longest past the middle, nothing at the corners.
                length = LASH[upper] * (0.04 + 0.96 * max(0.0, math.sin(math.pi * min(1.0, u ** 0.8))) ** 0.7)
                radial = P[i] - eye.c
                radial /= np.linalg.norm(radial)
                fan = eye.axis * ((u - 0.42) * 0.9)
                if upper:
                    first = 0.35 * radial + 0.75 * eye.fwd + fan - 0.15 * up
                    second = first / np.linalg.norm(first) + 0.9 * up
                else:
                    first = 0.35 * radial + 0.70 * eye.fwd + fan - 0.40 * up
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
    obj = _object("Lashes", pts, faces, uvs, _material("Lashes", lash_texture(out_dir), cut=False))
    return obj, roots


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
        turned = rotate_about(points[k][None], eye.c, eye.turn_axis, angle)[0]
        carried = rotate_about(before[None], eye.c, eye.turn_axis, angle)[0]
        out[k] = turned + (after - carried) - points[k]
    return out


def brows(face, out_dir):
    """The `Brows` object for `face`. Returns (the object, `bound`: for
    each vertex the skin it lies on as ([three vertices], [their shares]))."""
    P, topo = face.P, face.topo
    from facetopo import FS_HEAD
    outer = [k for k, fs in enumerate(topo.sets) if fs == FS_HEAD]
    tris = []
    for k in outer:
        p = topo.polys[k]
        tris += [(p[0], p[j], p[j + 1]) for j in range(1, len(p) - 1)]
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    tree = BVHTree.FromPolygons([tuple(p) for p in P], tris, all_triangles=True)
    pts, faces, uvs, bound = [], [], [], []
    for _, s in SIDES:
        a, b, c = face.brow_line(s)
        # From a little before the inner end to a little past the tail.
        curve = []
        for k in range(BROW_SEGMENTS + 1):
            t = -0.06 + 1.14 * k / BROW_SEGMENTS
            ctrl = 2.0 * b - 0.5 * (a + c)        # the curve passes through the arch
            curve.append((1 - t) ** 2 * a + 2 * (1 - t) * t * ctrl + t ** 2 * c)
        base = len(pts)
        for k, at in enumerate(curve):
            ahead = curve[min(k + 1, BROW_SEGMENTS)] - curve[max(k - 1, 0)]
            ahead /= np.linalg.norm(ahead)
            hit = nearest(tree, at, 0.05)
            across = np.cross(ahead, hit[1]) * (1.0 if s < 0 else -1.0)
            if across[2] < 0:
                across = -across
            across /= np.linalg.norm(across)
            for row in (-1.0, 0.0, 1.0):
                loc, normal, index, _ = tree.find_nearest(Vector(at + across * row * BROW_HALF), 0.05)
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
            for row in range(2):
                i0, i1 = base + 3 * k + row, base + 3 * (k + 1) + row
                quad = (i0, i1, i1 + 1, i0 + 1)
                u0, u1 = k / BROW_SEGMENTS, (k + 1) / BROW_SEGMENTS
                uv = [(u0, row / 2), (u1, row / 2), (u1, (row + 1) / 2), (u0, (row + 1) / 2)]
                if s < 0:
                    quad, uv = quad[::-1], uv[::-1]
                faces.append(quad)
                uvs.append(uv)
    obj = _object("Brows", pts, faces, uvs, _material("Brows", brow_texture(out_dir), cut=False))
    return obj, bound


def follow_bound(bound, shape, rows=3):
    """Offsets of the brows' vertices under a body shape. A card's `rows`
    vertices across the brow all move as the skin under its middle one
    does: the card rides the brow whole, it is not pulled apart where the
    skin above the brow moves less than the skin below it."""
    out = np.zeros((len(bound), 3))
    for k in range(len(bound)):
        tri, shares = bound[rows * (k // rows) + rows // 2]
        for i, w in zip(tri, shares):
            d = shape.get(int(i))
            if d is not None:
                out[k] += w * d
    return out


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
