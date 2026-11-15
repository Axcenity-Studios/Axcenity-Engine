"""The eyes' place and the teeth of the realistic body, set so nothing
shows through the skin at rest.

- **Where the eyes go** (`fit_eyes`): each socket is a sphere of the mesh;
  the eyeball sits a little forward in it, as large as leaves no skin
  inside the bundle's eyeball and none of its front inside the lids. The
  eyeballs themselves are made in `eyes.py` (CO6) at that place and size.
- **Teeth** (`teeth`): the bundle's realistic jaw (made ready by
  `teeth.py`): the teeth, both gums and the palate, upper half on
  `Head`, lower half on `Jaw`, a tongue made here with the tongue
  controls' shapes, and two simpler copies. The teeth meet 1.5 mm under
  the line the lips meet on and stand just behind the upper lip's inner
  side, where teeth are (CO4 had them 13 mm lower and 12 mm further
  back, behind the lower lip). The body's mouth is a hollow bag that
  wasn't modelled round this jaw, so the bag is widened where the jaw
  would touch it (`make_room`; it is never seen from outside, and never
  widened to nearer the face's skin than `UNDER_SKIN`). The teeth carry
  `Frame_Feminine`, as the mouth sits elsewhere in that frame,
  `Jaw_Forward`, and the tongue's `Tongue_Out`, `Tongue_Up` and
  `Tongue_Down`.

The jaw is placed by search: the nearest behind the lips that leaves none
of it inside the flesh, no face of it through the body, and the bag clear
of the face's skin.
"""

import math
import os

import bmesh
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

from common import FS_MUZZLE, FS_SOCKET, MOUTH_SETS, array, fit_sphere, import_obj, shade_smooth

# The iris and pupil, as angles from the eye's axis seen from its centre.
IRIS_DEG = 28.0
PUPIL_DEG = 9.5
# name -> (base colour (sRGB), roughness)
PALETTE = {
    "Sclera": ((0.93, 0.91, 0.88), 0.22),
    "Iris": ((1.0, 1.0, 1.0), 0.10),
    "Teeth": ((1.0, 1.0, 1.0), 0.16),
    "Gums": ((1.0, 1.0, 1.0), 0.34),
    "Tongue": ((1.0, 1.0, 1.0), 0.46),
}
IRIS_COLOUR = (0.36, 0.24, 0.14)
# The mouth's bag is never widened to nearer the face's skin than this (metres).
UNDER_SKIN = 0.0028
# How far the Jaw_Forward control brings the lower teeth (the body's shape
# of that name moves the jaw's skin as far: `face.JAW_FORWARD`).
JAW_FORWARD = 0.0055


def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def material(name, texture=None, occlusion=None):
    m = bpy.data.materials.get(name)
    if m is not None:
        return m
    colour, rough = PALETTE[name]
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = rough
    bsdf.inputs["Metallic"].default_value = 0.0
    bsdf.inputs["Base Color"].default_value = (*[srgb_to_linear(c) for c in colour], 1.0)
    if texture is not None:
        tex = m.node_tree.nodes.new("ShaderNodeTexImage")
        tex.image = texture
        tex.extension = "EXTEND"
        m.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    if occlusion is not None:
        import skin
        tex = m.node_tree.nodes.new("ShaderNodeTexImage")
        tex.image = occlusion
        tex.extension = "EXTEND"
        split = m.node_tree.nodes.new("ShaderNodeSeparateColor")
        m.node_tree.links.new(tex.outputs["Color"], split.inputs["Color"])
        out = m.node_tree.nodes.new("ShaderNodeGroup")
        out.node_tree = skin._gltf_output()
        m.node_tree.links.new(split.outputs["Red"], out.inputs["Occlusion"])
    return m


def iris_texture(out_dir, size=512):
    """The iris seen through the cornea: fibres radiating from the pupil,
    a lighter collar round it, a dark limbal ring fading into the white."""
    rng = np.random.default_rng(11)
    y, x = np.mgrid[0:size, 0:size]
    u = (x + 0.5) / size * 2 - 1
    v = (y + 0.5) / size * 2 - 1
    r = np.sqrt(u * u + v * v)
    a = np.arctan2(v, u)
    # The texture spans a little more than the iris.
    iris_r = 0.86
    pupil_r = iris_r * math.sin(math.radians(PUPIL_DEG)) / math.sin(math.radians(IRIS_DEG))
    t = np.clip((r - pupil_r) / (iris_r - pupil_r), 0, 1)
    fibres = np.zeros_like(r)
    for k, amp in ((29, 0.10), (47, 0.08), (83, 0.06), (131, 0.04)):
        phase = rng.uniform(0, 2 * np.pi)
        fibres += amp * np.sin(a * k + phase + 2.5 * np.sin(6.0 * t + phase))
    crypts = 0.06 * np.sin(a * 9 + 4 * t) * np.exp(-((t - 0.45) / 0.25) ** 2)
    value = 0.80 + fibres * (0.4 + 0.6 * t) + crypts
    value *= 1.0 + 0.30 * np.exp(-((t - 0.16) / 0.10) ** 2)        # the collar
    value *= 1.0 - 0.65 * np.clip((t - 0.72) / 0.28, 0, 1) ** 1.6   # the limbal ring
    value = np.clip(value, 0, 1.2)
    warm = np.exp(-((t - 0.2) / 0.2) ** 2)[:, :, None]
    base = np.array(IRIS_COLOUR)
    inner = np.array((0.50, 0.33, 0.14))
    rgb = (base * (1 - warm) + inner * warm) * value[:, :, None] * 1.6
    rgb[r < pupil_r] = 0.012
    # A soft edge on the pupil and out into the white.
    edge = np.clip((r - pupil_r) / 0.02, 0, 1)[:, :, None]
    rgb = rgb * edge + 0.012 * (1 - edge)
    white = np.array(PALETTE["Sclera"][0])
    out = np.clip((r - iris_r) / 0.05, 0, 1)[:, :, None]
    rgb = np.clip(rgb * (1 - out) + white * out, 0, 1)
    from PIL import Image
    path = os.path.join(out_dir, "RealisticIris.png")
    Image.fromarray((rgb[::-1] * 255 + 0.5).astype(np.uint8)).save(path, optimize=True)
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "sRGB"
    return img


def _eyeball():
    """The bundle's eyeball about its centre (points, faces) and its
    radius: the file holds the eyeball and the iris disc, and the eyeball
    is the larger piece."""
    (both,) = import_obj("eyes.obj")
    piece = _largest_piece(both.data)
    index = {old: new for new, old in enumerate(piece)}
    src = array(both.data.vertices[i].co for i in piece)
    faces = [tuple(index[i] for i in p.vertices) for p in both.data.polygons if p.vertices[0] in index]
    bpy.data.objects.remove(both)
    # Its back half is a sphere.
    radius = float(np.median(np.linalg.norm(src[src[:, 1] > 0], axis=1)))
    return src, faces, radius


def fit_eyes(body, P, lm):
    """Where the eyes go in body positions `P`: ({1: left centre, -1:
    right centre}, radius, report). Each socket is a sphere of the mesh;
    the eyeball sits a little forward in it, as large as leaves no skin
    inside the eyeball and none of the eyeball's front inside the lids."""
    src, faces, src_radius = _eyeball()
    tree = _body_tree(body, P)
    X = array(P)
    sockets = {s: fit_sphere(P[i] for i in lm.verts(FS_SOCKET[s])) for s in (1, -1)}
    socket_r = (sockets[1][1] + sockets[-1][1]) / 2
    # One height, depth and distance from the middle for both eyes.
    mid = Vector(((sockets[1][0].x - sockets[-1][0].x) / 2, (sockets[1][0].y + sockets[-1][0].y) / 2,
                  (sockets[1][0].z + sockets[-1][0].z) / 2))
    front = src[-src[:, 1] / np.maximum(np.linalg.norm(src, axis=1), 1e-9) > math.cos(math.radians(60))]
    triangles = []
    for f in faces:
        triangles += [(f[0], f[k], f[k + 1]) for k in range(1, len(f) - 1)]
    tries = []
    for size in (0.892, 0.88, 0.868, 0.856, 0.844, 0.832, 0.82):
        for forward in (0.22, 0.19, 0.16, 0.13, 0.10):
            tries.append(((0.892 - size) * 12 + (0.22 - forward), size, forward))
    best = None
    for _, size, forward in sorted(tries):
        radius = socket_r * size
        scale = radius / src_radius
        skin = flesh = 0
        centres = {}
        for s in (1, -1):
            c = Vector((mid.x * s, mid.y - forward * socket_r, mid.z))
            centres[s] = c
            mirror = np.array([s, 1.0, 1.0])
            ball = BVHTree.FromPolygons([tuple(p * mirror * scale + np.array(c)) for p in src],
                                        triangles if s > 0 else [t[::-1] for t in triangles], all_triangles=True)
            near = np.nonzero(np.linalg.norm(X - np.array(c), axis=1) < radius * 1.6)[0]
            for i in near:
                loc, normal, _, _ = ball.find_nearest(Vector(X[i]), 0.03)
                if loc is not None and (Vector(X[i]) - loc).dot(normal) < -0.0001:
                    skin += 1
            flesh += _inside(tree, front * mirror * scale + np.array(c), 0.0001)
        if best is None or skin + flesh < best[2]["skin_in_eyeball"] + best[2]["eyeball_front_in_flesh"]:
            best = (centres, radius, {
                "skin_in_eyeball": skin, "eyeball_front_in_flesh": flesh,
                "radius": round(radius, 5), "of_socket": size, "forward_of_socket": forward})
        if skin + flesh == 0:
            break
    return best


def _largest_piece(mesh):
    """The vertex indices of the mesh's largest connected piece, in order."""
    nbr = [[] for _ in mesh.vertices]
    for e in mesh.edges:
        a, b = e.vertices
        nbr[a].append(b)
        nbr[b].append(a)
    seen, best = set(), []
    for start in range(len(mesh.vertices)):
        if start in seen:
            continue
        stack, piece = [start], []
        while stack:
            i = stack.pop()
            if i in seen:
                continue
            seen.add(i)
            piece.append(i)
            stack.extend(nbr[i])
        if len(piece) > len(best):
            best = piece
    return sorted(best)


# The teeth meet this far under the line the lips meet on, and their front
# is tried these distances behind the upper lip's inner side (metres at
# 1.75 m; scaled with the body).
BITE_UNDER_LIPS = 0.0015
BEHIND_LIP = (0.0008, 0.0016, 0.0026, 0.004, 0.006)


def _mouth(P, lm):
    """Where the mouth is in body positions `P`: the height of the line
    the lips meet on (the upper lip's edge at the middle), how far back
    the upper lip's inner side is there (the teeth go behind it), half
    the mouth's width, and the vertices that must stay where they are
    when room is made for the jaw (the lips' edges)."""
    from common import FS_HEAD, FS_MOUTH_UPPER
    edge = sorted(lm.ring(FS_MOUTH_UPPER, FS_HEAD))
    pts = array(P[i] for i in edge)
    mid = pts[np.abs(pts[:, 0]) < 0.006]
    z, front = float(np.median(mid[:, 2])), float(mid[:, 1].min())
    lining = array(P[i] for i in lm.verts(FS_MOUTH_UPPER))
    behind = lining[(np.abs(lining[:, 0]) < 0.008) & (lining[:, 2] > z + 0.002) & (lining[:, 2] < z + 0.010)
                    & (lining[:, 1] < front + 0.014)]
    opening = sorted(set(lm.ring(FS_MUZZLE, MOUTH_SETS)) | set(edge))
    return {"front": float(behind[:, 1].max()), "z": z, "half": float(np.abs(pts[:, 0]).max()),
            "opening": opening}


def _body_tree(body, P, faces=None):
    """A BVH of the body's triangles at positions `P` (all of them, or
    those of the polygons `faces`)."""
    me = body.data
    me.calc_loop_triangles()
    triangles = [tuple(t.vertices) for t in me.loop_triangles if faces is None or t.polygon_index in faces]
    return BVHTree.FromPolygons([tuple(p) for p in P], triangles, all_triangles=True)


def _inside(tree, points, margin=0.0005):
    """How many of `points` lie inside the flesh: behind the nearest
    surface by more than `margin` (as the QA measures it)."""
    count = 0
    for p in points:
        loc, normal, _, _ = tree.find_nearest(Vector(p), 0.02)
        if loc is not None and (Vector(p) - loc).dot(normal) < -margin:
            count += 1
    return count


def make_room(body, P, lm, jaw_points, opening, margin=0.002):
    """Body positions `P` with the mouth's bag moved clear of the jaw at
    `jaw_points`: each of the bag's vertices inside the jaw's hull (or
    nearer it than `margin`) goes straight out from the jaw's middle to
    the hull and `margin` beyond, but never nearer the face's skin than
    `UNDER_SKIN`. The lips' line stays, and the bag eases into the
    change over the first centimetre behind it. Returns (positions, how
    near the bag now comes to the face's skin in metres)."""
    import rig
    bm = bmesh.new()
    for p in jaw_points:
        bm.verts.new(tuple(p))
    hull = bmesh.ops.convex_hull(bm, input=bm.verts[:])
    keep = {v for v in hull["geom"] if isinstance(v, bmesh.types.BMVert)}
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if v not in keep], context="VERTS")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    tree = BVHTree.FromBMesh(bm)
    bm.free()
    X = array(P)
    pts = np.asarray(jaw_points)
    dist, _ = rig.surface_distance(body.data, X, opening)
    in_bag = set().union(*(lm.of_set[fs] for fs in MOUTH_SETS))
    bag_faces = {p.index for p in body.data.polygons if all(v in in_bag for v in p.vertices)}
    outer = _body_tree(body, P, faces={p.index for p in body.data.polygons} - bag_faces)

    def under(p):
        """How far `p` is under the skin round the bag (negative: through
        it), the lips' inner sides included."""
        loc, normal, _, _ = outer.find_nearest(p, 0.03)
        return 1.0 if loc is None else (loc - p).dot(normal)

    def held(i, to):
        """`to`, or as far toward it from where vertex `i` rests as
        leaves it `UNDER_SKIN` under the face's skin: the bag stays
        inside the head whatever the teeth ask for."""
        start = Vector(X[i])
        if under(to) >= UNDER_SKIN:
            return to
        lo, hi = 0.0, 1.0
        for _ in range(8):
            mid = 0.5 * (lo + hi)
            if under(start.lerp(to, mid)) >= UNDER_SKIN:
                lo = mid
            else:
                hi = mid
        return start.lerp(to, lo)
    out = [p.copy() for p in P]
    moved = []
    # The roof is pushed out from a point under the plane the teeth meet
    # on, the floor from one over it: up and out, down and out.
    middle = np.array([0.0, (pts[:, 1].min() + pts[:, 1].max()) / 2, np.median(pts[:, 2])])
    bag, ways = [], {}
    for fs, lift in zip(MOUTH_SETS, (-0.006, 0.006)):
        core = Vector(middle + np.array([0.0, 0.0, lift]))
        for i in sorted(lm.of_set[fs]):
            bag.append(i)
            ease = min(1.0, max(0.0, (dist[i] - 0.0015) / 0.006))
            if ease <= 0.0:
                continue
            way = Vector(X[i]) - core
            reach = way.length
            if reach < 1e-6:
                continue
            way /= reach
            hit = tree.ray_cast(core + way * 0.2, -way, 0.2)
            if hit[0] is None:
                continue
            ways[i] = (way, ease)
            edge = (hit[0] - core).length + margin
            if reach < edge:
                out[i] = held(i, Vector(X[i]).lerp(core + way * edge, ease * ease * (3 - 2 * ease)))
                moved.append(i)
    bag = np.unique(np.array(bag, dtype=np.int64))
    # The bag has few faces, and a flat face between vertices that are
    # clear can still cut a corner of the jaw: push the vertices of any
    # face a point of the jaw is behind a little further, until none is.
    me = body.data
    me.calc_loop_triangles()
    triangles = [tuple(t.vertices) for t in me.loop_triangles]
    for _ in range(16):
        tree_now = BVHTree.FromPolygons([tuple(p) for p in out], triangles, all_triangles=True)
        push = {}
        for p in pts:
            loc, normal, index, _ = tree_now.find_nearest(Vector(p), 0.02)
            if loc is None:
                continue
            depth = -(Vector(p) - loc).dot(normal)
            if depth > 0.0003:
                for v in triangles[index]:
                    if v in ways:
                        push[v] = max(push.get(v, 0.0), depth + 0.0008)
        if not push:
            break
        for v, by in push.items():
            way, ease = ways[v]
            out[v] = held(v, out[v] + way * (by * ease))
            moved.append(v)
    # How near the widened bag comes to the skin outside it.
    # (Of the vertices that moved: one that could not, because it already
    # lies nearer the skin than that or before the lower lip's inner
    # side, is where the bundle has it and says nothing about the jaw's
    # place. Counted, it turned down every place for the feminine jaw
    # nearer than 6 mm behind the lips.)
    nearest = 1.0
    for i in set(moved):
        if (out[i] - Vector(X[i])).length > 1e-7:
            nearest = min(nearest, under(out[i]))
    return out, nearest


# The tongue's mesh: rings from the tip to the root, points round each.
TONGUE_ROWS, TONGUE_AROUND = 14, 16


def _tongue(lower_teeth, k):
    """A tongue in the jaw's own space (the incisors' front on the plane
    the teeth meet on; y back, z up): (points, quads and triangles, how
    far each point is from the root 0 to the tip 1). It lies inside the
    lower teeth's arch and fills it from side to side, its tip round and
    thin behind the lower incisors and well under their tips, its back rising
    toward the throat.
    The points are in rings of `TONGUE_AROUND` from the tip, then the
    tip's own point."""
    lower_teeth = np.asarray(lower_teeth)
    tip_y = float(lower_teeth[np.abs(lower_teeth[:, 0]) < 0.006 * k][:, 1].max()) + 0.0020 * k
    root_y = tip_y + 0.048 * k
    rows, around = TONGUE_ROWS, TONGUE_AROUND

    def room(y):
        """Half the width between the lower teeth at `y`."""
        near = lower_teeth[np.abs(lower_teeth[:, 1] - y) < 0.004 * k]
        return float(np.abs(near[:, 0]).min()) if len(near) else 0.02 * k

    points, along, around_of = [], [], []
    for row in range(rows):
        # Rings closer together toward the tip, where the outline turns.
        t = (row / (rows - 1.0)) ** 1.5 * 0.97 + 0.03          # 0 at the tip
        y = tip_y + (root_y - tip_y) * t
        # The outline from above: a round tip, then the arch's width.
        half = 0.0200 * k * math.sqrt(max(0.0, 1.0 - (1.0 - min(1.0, t / 0.42)) ** 2))
        half *= 1.0 - 0.10 * max(0.0, t - 0.75) / 0.25
        half = max(0.002 * k, min(half, room(y) - 0.0018 * k))
        thick = k * (0.0030 + 0.0111 * min(1.0, t * 2.2) ** 0.8)
        top = k * (-0.0065 + 0.0022 * t + 0.0040 * max(0.0, t - 0.6) / 0.4)
        for j in range(around):
            a = 2.0 * math.pi * j / around
            x, lift = math.sin(a), math.cos(a)        # round the section, from its top
            # A rounded section with a furrow down the middle of its top.
            z = top - thick * 0.5 + thick * 0.5 * (abs(lift) ** 0.85) * (1 if lift >= 0 else -1)
            if lift > 0:
                z -= 0.0007 * k * math.exp(-(x / 0.35) ** 2) * min(1.0, t * 3.0)
            points.append((half * (abs(x) ** 0.9) * (1 if x >= 0 else -1), y, z))
            along.append(1.0 - t)
            around_of.append((abs(x), lift))
    faces = []
    for row in range(rows - 1):
        for j in range(around):
            a, b = row * around + j, row * around + (j + 1) % around
            faces.append((a, b, b + around, a + around))
    # The tip is closed with a fan; the root is hidden in the throat.
    tip = len(points)
    points.append((0.0, tip_y - 0.0004 * k, k * (-0.0065 - 0.0015)))
    along.append(1.0)
    around_of.append((0.0, 0.0))
    for j in range(around):
        faces.append((tip, (j + 1) % around, j))
    return np.array(points), faces, np.array(along), np.array(around_of)


def _tongue_shapes(points, along, k):
    """The tongue's shapes, as {name: positions}: `Tongue_Out` (forward
    between the teeth, flatter and narrower), `Tongue_Up` (the tip to the
    ridge behind the upper teeth) and `Tongue_Down` (the tip curled down
    behind the lower teeth). `along` is 0 at the root and 1 at the tip."""
    out = {}
    reach = np.clip(along, 0.0, 1.0)
    front = np.clip((reach - 0.35) / 0.65, 0.0, 1.0) ** 1.4
    P = np.array(points)
    forward = P.copy()
    forward[:, 1] -= 0.017 * k * reach ** 0.8
    forward[:, 2] += 0.0030 * k * reach
    forward[:, 0] *= 1.0 - 0.22 * reach
    mid = np.median(P[:, 2])
    # (From its low rest over the lower teeth and the lower lip, which
    # stands higher on an open jaw.)
    forward[:, 2] = mid + (forward[:, 2] - mid) * (1.0 - 0.25 * reach) + 0.0105 * k * reach
    out["Tongue_Out"] = forward
    for name, lift, back in (("Tongue_Up", 0.0105, 0.0030), ("Tongue_Down", -0.0045, 0.0040)):
        Q = P.copy()
        Q[:, 2] += lift * k * front
        Q[:, 1] += back * k * front
        out[name] = Q
    return out


ORDER = ["Teeth", "Gums", "Tongue"]


def _jaw_object(level, ks, images):
    """The jaw at `level` of detail (`teeth.LEVELS`) with its tongue, in
    the jaw's own space: (object, what its vertices are)."""
    import teeth as jawlib
    jaw, info = jawlib.jaw_mesh(ks, level)
    me = jaw.data
    unit = array(v.co for v in me.vertices)
    jaw_uv = np.zeros((len(unit), 2))
    for loop in me.loops:
        jaw_uv[loop.vertex_index] = me.uv_layers[0].data[loop.index].uv
    kinds = [0 if t else 1 for t in info["is_tooth_face"]]
    me.materials.clear()
    for name in ORDER:
        me.materials.append(material(name, images[name], images["occlusion"]))
    for poly, kind in zip(me.polygons, kinds):
        poly.material_index = kind
    on_jaw = ~info["upper"]
    is_tooth = info["tooth"] >= 0
    # The tongue, on the jaw.
    tongue, tongue_faces, along, around = _tongue(unit[on_jaw & is_tooth], ks)
    first = len(unit)
    bm = bmesh.new()
    bm.from_mesh(me)
    made = [bm.verts.new(tuple(p)) for p in tongue]
    for f in tongue_faces:
        bm.faces.new([made[i] for i in f]).material_index = ORDER.index("Tongue")
    bm.to_mesh(me)
    bm.free()
    me.update()
    # The upper half follows the head, the lower the jaw.
    for name, members in (("Head", ~on_jaw), ("Jaw", on_jaw)):
        jaw.vertex_groups.new(name=name).add([int(i) for i in np.nonzero(members)[0]], 1.0, "REPLACE")
    jaw.vertex_groups["Jaw"].add(list(range(first, first + len(tongue))), 1.0, "REPLACE")
    if not all(len(v.groups) == 1 for v in me.vertices):
        raise RuntimeError("a vertex of the teeth is on neither the head nor the jaw, or on both")
    # Each vertex's place in its material's palette (`teeth.py`).
    uv = np.concatenate([jaw_uv, jawlib.paint_tongue(along, around)])
    layer = me.uv_layers[0]
    for loop in me.loops:
        layer.data[loop.index].uv = uv[loop.vertex_index]
    what = {"unit": array(v.co for v in me.vertices), "first": first, "tongue": tongue, "along": along,
            "on_jaw": np.concatenate([on_jaw, np.ones(len(tongue), dtype=bool)]),
            "tongue_triangles": sum(len(f) - 2 for f in tongue_faces)}
    return jaw, what


def _place(jaw, what, fits, ks, name):
    """`jaw` put in the mouth: where `fits` ({frame: (scale, place)}) has
    it on the masculine frame, with `Frame_Feminine` and the shapes the
    teeth and the tongue follow the body's controls with."""
    me = jaw.data
    unit, first = what["unit"], what["first"]
    scale, at = fits["masculine"]
    rest = unit * scale + at
    for vert, p in zip(me.vertices, rest):
        vert.co = p
    me.update()
    jaw.name = name
    jaw.shape_key_add(name="Basis", from_mix=False)
    key = jaw.shape_key_add(name="Frame_Feminine", from_mix=False)
    for i, p in enumerate(unit * fits["feminine"][0] + fits["feminine"][1]):
        key.data[i].co = p
    key.value = 0.0
    key = jaw.shape_key_add(name="Jaw_Forward", from_mix=False)
    for i in np.nonzero(what["on_jaw"])[0]:
        key.data[i].co = Vector(rest[i]) + Vector((0.0, -JAW_FORWARD, 0.0))
    key.value = 0.0
    for shape, moved in _tongue_shapes(what["tongue"], what["along"], ks).items():
        key = jaw.shape_key_add(name=shape, from_mix=False)
        for j, p in enumerate(moved * scale + at):
            key.data[first + j].co = p
        key.value = 0.0
    shade_smooth(jaw)


def teeth(body, P, P_feminine, lm, k, out_dir):
    """The `Teeth` object in the mouth of body positions `P`, with
    `Frame_Feminine` to where the mouth is in `P_feminine`, and its
    simpler copies `Teeth_LOD1` and `Teeth_LOD2`. `k` is the body's
    scale against the bundle (its height over the bundle's). Returns
    (the object, its copies, both frames' positions with room made in
    the mouth, a report).

    The bundle's jaw is its teeth and one bag round them: the gums of
    both jaws (sets 93, 155), a tongue on the floor (31) and an outer
    shell that closes the bag above and below (157). Kept: the teeth,
    both gums and the shell's upper half (the palate), at `jawlib.SIZE`
    of the bundle's size (`teeth.py`). The shell's lower half and the
    floor would have to stretch from the head to the jaw when it opens;
    the body's own lining is the mouth's floor instead, and the tongue
    is made here (`_tongue`), with the tongue controls' shapes. The
    upper half is cut free of the lower, so nothing spans the two.

    The copies are made here and not left to the engine, which would
    halve the teeth's triangles wherever they fall when the part is
    under 30% of the view's height: in every picture of a whole face."""
    import teeth as jawlib
    ks = k * jawlib.SIZE
    images = jawlib.palettes(out_dir)
    jaw, what = _jaw_object(0, ks, images)
    me = jaw.data
    unit = what["unit"]
    is_tongue = np.arange(len(unit)) >= what["first"]
    solid = [tuple(p.vertices) for p in me.polygons if p.material_index != ORDER.index("Tongue")]
    roomy, report, fits = {}, {}, {}
    for frame, positions in (("masculine", P), ("feminine", P_feminine)):
        mouth = _mouth(positions, lm)
        best = None
        for scale in (1.0, 0.97, 0.94, 0.91, 0.88):
            for back in BEHIND_LIP:
                at = np.array([0.0, mouth["front"] + back * k, mouth["z"] - BITE_UNDER_LIPS * k])
                points = unit * scale + at
                room, nearest = make_room(body, positions, lm, points[~is_tongue], mouth["opening"])
                # (The teeth and gums: the tongue is made to fit them, below.)
                flesh = _body_tree(body, room)
                inside = _inside(flesh, points[~is_tongue])
                # (And no face of it through a face of the body: a sheet
                # of the gum can cross the skin between its points.)
                through = len(BVHTree.FromPolygons([tuple(q) for q in points], solid).overlap(flesh))
                # None of it in the flesh, and 2 mm of flesh left outside the bag.
                bad = inside + through + (1000 if nearest < 0.002 else 0)
                if best is None or bad < best[0]:
                    best = (bad, room, {"scale": round(scale * jawlib.SIZE, 3),
                                        "behind_lips_mm": round(back * k * 1000, 1),
                                        "bite_under_lips_mm": round(BITE_UNDER_LIPS * k * 1000, 1),
                                        "points_in_flesh": inside, "faces_through_the_body": through,
                                        "bag_moved_to_skin_mm": None if nearest >= 1.0 else round(nearest * 1000, 1)},
                            scale, at)
                if bad == 0:
                    break
            if best[0] == 0:
                break
        roomy[frame], report[frame], fits[frame] = best[1], best[2], (best[3], best[4])
    _place(jaw, what, fits, ks, "Teeth")
    count = lambda obj: {name: sum(len(p.vertices) - 2 for p in obj.data.polygons if p.material_index == index)  # noqa: E731
                         for index, name in enumerate(ORDER)}
    report["tongue_triangles"] = what["tongue_triangles"]
    report["triangles"] = count(jaw)
    report["of_bundle_size"] = jawlib.SIZE
    copies = []
    for level in (1, 2):
        copy, its = _jaw_object(level, ks, images)
        _place(copy, its, fits, ks, f"Teeth_LOD{level}")
        report[f"triangles_lod{level}"] = count(copy)
        copies.append(copy)
    return jaw, copies, roomy["masculine"], roomy["feminine"], report
