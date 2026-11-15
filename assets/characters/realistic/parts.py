"""The parts always worn with the realistic body: the eyes and the teeth,
placed so nothing shows through the skin at rest.

- **Eyes**: the bundle's realistic eyeball (a sphere with the cornea's
  bulge), one per side on its eye bone, sized and placed from the body's
  eye sockets. The iris is a texture over the cornea (`Iris`), the rest
  `Sclera`: one closed surface, which is what the renderer's eye shading
  (iris parallax under a cornea lobe, CO7) expects. The bundle's separate
  iris disc is vendored in `source/eyes.obj` for a head that wants real
  depth behind a cornea shell (CO6).
- **Teeth**: the bundle's realistic jaw (teeth, gums, the palate and the
  floor of the mouth with the tongue), upper half on `Head`, lower half on
  `Jaw`, set just behind the lips with the teeth meeting on the lips'
  line. The body's mouth is a hollow bag that wasn't modelled round this
  jaw, so the bag is widened where the jaw would touch it (`make_room`;
  it is never seen from outside). It carries `Frame_Feminine`, as the
  mouth sits elsewhere in that frame.

Both are placed by search: the eyeball as large and as far forward as
leaves no skin inside it and none of it inside the lids, the jaw at the
largest size that leaves none of it inside the flesh and the bag clear of
the face's skin.
"""

import math
import os

import bmesh
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

from common import (
    FS_MUZZLE, FS_SOCKET, MOUTH_SETS, SIDES, array, fit_sphere, import_obj, new_object, select_only,
    shade_smooth,
)

# The iris and pupil, as angles from the eye's axis seen from its centre.
IRIS_DEG = 28.0
PUPIL_DEG = 9.5
# name -> (base colour (sRGB), roughness)
PALETTE = {
    "Sclera": ((0.93, 0.91, 0.88), 0.22),
    "Iris": ((1.0, 1.0, 1.0), 0.10),
    "Teeth": ((0.89, 0.86, 0.78), 0.25),
    "Gums": ((0.72, 0.36, 0.37), 0.38),
    "Tongue": ((0.70, 0.34, 0.37), 0.42),
}
IRIS_COLOUR = (0.36, 0.24, 0.14)
# The jaw's face sets: the gums, the palate, the floor with the tongue;
# every other set is a tooth.
FS_UPPER_GUM, FS_LOWER_GUM, FS_PALATE, FS_FLOOR = 93, 155, 157, 31
TEETH_TRIANGLES = 3600


def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def material(name, texture=None):
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


def eyes(centres, radius, out_dir):
    """The `Eyes` object: both eyeballs at `centres` ({1: left, -1: right})
    with radius `radius`, each on its eye bone."""
    src, faces, src_radius = _eyeball()
    scale = radius / src_radius
    iris = iris_texture(out_dir)
    tex_radius = radius * math.sin(math.radians(IRIS_DEG)) / 0.86
    verts, polys, names, uvs, groups = [], [], [], [], {}
    for side, s in SIDES:
        base = len(verts)
        for p in src:
            verts.append(Vector((p[0] * s, p[1], p[2])) * scale + centres[s])
        for f in faces:
            mid = np.mean([src[i] for i in f], axis=0)
            angle = math.degrees(math.acos(max(-1.0, min(1.0, -mid[1] / np.linalg.norm(mid)))))
            f = f if s > 0 else f[::-1]
            polys.append(tuple(base + i for i in f))
            names.append("Iris" if angle < IRIS_DEG + 3.0 else "Sclera")
            uvs.append([(0.5 + src[i][0] * s * scale / (2 * tex_radius),
                         0.5 + src[i][2] * scale / (2 * tex_radius)) for i in f])
        groups[f"{side}Eye"] = list(range(base, base + len(src)))
    me = bpy.data.meshes.new("Eyes")
    me.from_pydata([tuple(p) for p in verts], [], polys)
    layer = me.uv_layers.new(name="UVMap")
    k = 0
    for corner in uvs:
        for uv in corner:
            layer.data[k].uv = uv
            k += 1
    obj = new_object("Eyes", me)
    order = ["Sclera", "Iris"]
    me.materials.append(material("Sclera"))
    me.materials.append(material("Iris", iris))
    for poly, name in zip(me.polygons, names):
        poly.material_index = order.index(name)
    for bone, idx in groups.items():
        obj.vertex_groups.new(name=bone).add(idx, 1.0, "REPLACE")
    shade_smooth(obj)
    return obj


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


def _mouth(P, lm):
    """Where the mouth is in body positions `P`: the middle of the line
    where the lips meet, its front, and half its width."""
    opening = sorted(set(lm.ring(FS_MUZZLE, MOUTH_SETS)))
    pts = array(P[i] for i in opening)
    mid = pts[np.abs(pts[:, 0]) < 0.012]
    return {"front": float(mid[:, 1].min()), "z": float(np.median(mid[:, 2])),
            "half": float(np.abs(pts[:, 0]).max()), "opening": opening}


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
    the hull and `margin` beyond. The lips' line stays, and the bag eases into the
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
                out[i] = Vector(X[i]).lerp(core + way * edge, ease * ease * (3 - 2 * ease))
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
            out[v] = out[v] + way * (by * ease)
            moved.append(v)
    # How near the widened bag comes to the skin outside it.
    in_bag = set(bag.tolist())
    bag_faces = {p.index for p in body.data.polygons if all(v in in_bag for v in p.vertices)}
    outer = _body_tree(body, P, faces={p.index for p in body.data.polygons} - bag_faces)
    nearest = 1.0
    for i in set(moved):
        loc, normal, _, _ = outer.find_nearest(out[i])
        # How far under the skin (negative: through it).
        nearest = min(nearest, (loc - out[i]).dot(normal))
    return out, nearest


def teeth(body, P, P_feminine, lm, k):
    """The `Teeth` object in the mouth of body positions `P`, with
    `Frame_Feminine` to where the mouth is in `P_feminine`. `k` is the
    body's scale against the bundle (its height over the bundle's).
    Returns (the object, both frames' positions with room made in the
    mouth, a report)."""
    (jaw,) = import_obj("teeth.obj")
    me = jaw.data
    ids = [int(m.name.split("_")[1].split(".")[0]) for m in me.materials]
    sets = [ids[p.material_index] for p in me.polygons]
    co = array(v.co for v in me.vertices)
    teeth_sets = sorted(set(sets) - {FS_UPPER_GUM, FS_LOWER_GUM, FS_PALATE, FS_FLOOR})
    centres = {fs: np.mean([co[i] for p, s in zip(me.polygons, sets) if s == fs for i in p.vertices], axis=0)
               for fs in teeth_sets}
    bite = float(np.median([c[2] for c in centres.values()]))
    upper_sets = {fs for fs, c in centres.items() if c[2] > bite} | {FS_UPPER_GUM, FS_PALATE}
    # About the incisors' front, on the plane the teeth meet on.
    tooth_verts = sorted({i for p, s in zip(me.polygons, sets) if s in teeth_sets for i in p.vertices})
    front = float(co[tooth_verts][:, 1].min())
    co = (co - np.array([0.0, front, bite])) * k
    for vert, p in zip(me.vertices, co):
        vert.co = p
    # The gums, palate and floor run on four centimetres behind the last
    # teeth (the throat, for sculpting): cut off there, they'd come
    # through the skin under the ear when the jaw opens.
    back = float(co[tooth_verts][:, 1].max()) + 0.004 * k
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    gone = [f for f in bm.faces if f.calc_center_median().y > back]
    gone_ids = {f.index for f in gone}
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    bm.to_mesh(me)
    bm.free()
    sets = [s for i, s in enumerate(sets) if i not in gone_ids]
    kinds = ["Teeth" if s in teeth_sets else "Tongue" if s == FS_FLOOR else "Gums" for s in sets]
    upper_face = [s in upper_sets for s in sets]
    order = ["Teeth", "Gums", "Tongue"]
    me.materials.clear()
    for name in order:
        me.materials.append(material(name))
    for poly, kind in zip(me.polygons, kinds):
        poly.material_index = order.index(kind)
    # The upper half follows the head, the lower the jaw: kept through the
    # decimation as vertex groups.
    head = jaw.vertex_groups.new(name="Head")
    lower = jaw.vertex_groups.new(name="Jaw")
    upper_verts = {i for p, up in zip(me.polygons, upper_face) if up for i in p.vertices}
    head.add(sorted(upper_verts), 1.0, "REPLACE")
    lower.add(sorted(set(range(len(me.vertices))) - upper_verts), 1.0, "REPLACE")
    # The bundle's jaw is for sculpting (10,000 triangles).
    select_only([jaw])
    mod = jaw.modifiers.new("Decimate", "DECIMATE")
    mod.ratio = TEETH_TRIANGLES / sum(len(p.vertices) - 2 for p in me.polygons)
    bpy.ops.object.modifier_apply(modifier="Decimate")
    me = jaw.data
    for vert in me.vertices:
        weights = {jaw.vertex_groups[g.group].name: g.weight for g in vert.groups}
        to_head = weights.get("Head", 0.0) >= weights.get("Jaw", 0.0)
        (head if to_head else lower).add([vert.index], 1.0, "REPLACE")
        (lower if to_head else head).remove([vert.index])
    unit = array(v.co for v in me.vertices)
    placed, roomy, report = {}, {}, {}
    for frame, positions in (("masculine", P), ("feminine", P_feminine)):
        mouth = _mouth(positions, lm)
        best = None
        for scale in (1.0, 0.97, 0.94, 0.91, 0.88, 0.85, 0.82, 0.79, 0.76):
            for back in (0.006, 0.008, 0.010, 0.012):
                at = np.array([0.0, mouth["front"] + back * k, mouth["z"]])
                points = unit * scale + at
                room, nearest = make_room(body, positions, lm, points, mouth["opening"])
                inside = _inside(_body_tree(body, room), points)
                # None of it in the flesh, and 2 mm of flesh left outside the bag.
                bad = inside + (1000 if nearest < 0.002 else 0)
                if best is None or bad < best[0]:
                    best = (bad, points, room, {"scale": round(scale, 3), "behind_lips_mm": round(back * k * 1000, 1),
                                                "points_in_flesh": inside,
                                                "bag_to_skin_mm": round(nearest * 1000, 1)})
                if bad == 0:
                    break
            if best[0] == 0:
                break
        placed[frame], roomy[frame], report[frame] = best[1], best[2], best[3]
    for vert, p in zip(me.vertices, placed["masculine"]):
        vert.co = p
    me.update()
    jaw.name = "Teeth"
    jaw.shape_key_add(name="Basis", from_mix=False)
    key = jaw.shape_key_add(name="Frame_Feminine", from_mix=False)
    for i, p in enumerate(placed["feminine"]):
        key.data[i].co = p
    key.value = 0.0
    shade_smooth(jaw)
    return jaw, roomy["masculine"], roomy["feminine"], report
