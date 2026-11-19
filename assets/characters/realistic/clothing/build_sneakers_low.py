"""Builds the low-top sneakers' sources (Blender's Python, bpy 5.x): the
first sneakers made from scratch (wave 5, Paul's decision 14), modelled
over a last made from the realistic foot (`sneakers_last.py`) as a
shoemaker builds a shoe (`sneakers_low.py`), against reference photographs
found through Openverse (docs/development/character-studio/references/
clothing/README.md; looked at, never copied or shipped).

    python -I assets/characters/realistic/clothing/build_sneakers_low.py \\
        --body assets/characters/realistic/RealisticParts.glb --tiles <folder> --out <folder>

Writes into `--out`:

- `SneakersLow_LOD<k>.obj` (+ `.mtl`): each level, both shoes, in glTF's
  axes (Y up, facing +Z), a vertex per place and texture coordinate, with
  normals; `SneakersLow_LOD<k>.npz`: per OBJ vertex its weights (the
  foot's bones by name, four at most) and its offsets for each of the
  body's shapes it follows (`fit_sneakers_low.py` puts them on the
  engine's fitted model).
- `SneakersLow_color.jpg`, `_normal.jpg`, `_orm.jpg`: the 2048 px maps
  (`sneakers_maps.py`), shared by the three levels.
- `SneakersLow.json`: what was built and measured (the foot of both
  frames, the last, triangles per level, texel density, clearances).

**Skinning (a shoe is not cloth).** Every corner follows its side's foot
bones only: the toes (`<Side>Toes`) and the foot (`<Side>Foot`), split
across the ball as the skin under it is split (a Gaussian mean of the
skin's weights round the nearest skin, `SKIN_SIGMA`), and the toe box
(from `TOE_BOX` of the foot's length on) on the toes alone, so it turns
with them as one piece and the shoe bends only at the toe joint. The
upper's collar, the tongue's top and the laces near the ankle also take
the shin's share the skin under them has (`<Side>LowerLeg`,
`<Side>LowerLegTwist`), faded in from 35 to 62 mm up; round the heel
(`HEEL_FROM` to `HEEL_TO` of the length) the whole shoe, sole included,
takes it, because the body's own skinning carries the heel's skin partly
with the shin as the ankle turns and a rigid heel cup let that skin out.

**Shapes.** For each of the body's shapes that moves the foot, the shoe
takes the foot's movement as one smooth map (an affine fit of the foot's
skin under the shoe: its length, width, height and place), not the skin's
own bumps, so a shoe doesn't swell with body fat (references §5.2) and
its tread stays flat: the map's height part has no tilt (it depends on
height alone).
"""

import argparse
import json
import math
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402  (first: it provides bmesh and mathutils)
import bmesh  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

import body as bodymod  # noqa: E402
import sneakers_last  # noqa: E402
import sneakers_low  # noqa: E402
import sneakers_maps  # noqa: E402
import textures  # noqa: E402

NAME = "SneakersLow"
# The shipped materials, and what the build's own names are drawn as.
MATERIALS = ["Upper", "Lining", "Tongue", "Laces", "Eyelets", "Midsole", "Outsole"]
SHIPPED = {"SoleInner": "Midsole", "Insole": "Lining"}
INTERNAL = MATERIALS + list(SHIPPED)
# The toe box, from this share of the foot's length (foot-frame s): it
# follows the toes alone (asset-quality §3, "toe box deforms only at the
# toe joint"); the player's check takes the shoe's front quarter.
TOE_BOX = 0.74
# The shin's share fades in over these heights (m).
SHIN_FROM, SHIN_TO = 0.035, 0.062
# Round the heel (foot-frame s) the whole shoe takes the skin's shin share.
HEEL_FROM, HEEL_TO = 0.14, 0.32
SKIN_SIGMA = 0.008
# A body shape is followed when it moves the foot under the shoe by more.
SHAPE_MIN = 0.0003
UNWRAP_ITERATIONS = 400
# Each shoe's upper kept this far off its foot at least (the right shoe is
# mirrored from the left, and the body's feet are not exact mirrors).
RIGHT_GAP = 0.0040
PACK_MARGIN = 0.0025   # m between islands (about 9 texels at the atlas's density)


def clear_of(mesh, foot, gap):
    """`mesh`'s faces of the upper and the tongue moved out of `foot` where
    their middles come nearer than `gap` (the right shoe is mirrored from
    the left, and the body's feet are not exact mirrors). Returns the most
    any corner moved (m)."""
    P = np.array(mesh.P)
    part = np.array(mesh.part)
    # Not the upper's lasted edge inside the sole: it must stay between the
    # sole's walls.
    s = foot.s_of(P)
    movable = np.isin(part, ("upper", "tongue")) & (P[:, 2] > sneakers_low.sole_top(s) - 0.001)
    most = np.zeros(len(P))
    for _ in range(6):
        push = np.zeros(len(P))
        for face in mesh.faces:
            ids = list(face)
            if not movable[ids].all():
                continue
            corners = P[ids]
            for q in np.vstack([corners.mean(axis=0)[None], corners]):
                need = gap - foot._sample(foot.raw, q[None])[0]
                if need > 0:
                    push[ids] = np.maximum(push[ids], need)
        if not push.any():
            break
        sel = np.nonzero(push > 0)[0]
        P[sel] += foot.normal(P[sel]) * push[sel, None]
        most += push
    mesh.P = [p for p in P]
    return float(most.max())


def combined(left, right_foot=None, gap=0.0):
    """Both shoes of a level as one Blender mesh object: the right mirrored
    from the left (and cleared of the right foot). Returns the object and
    per-vertex arrays."""
    if right_foot is not None:
        combined.left_moved = clear_of(left, combined.left_foot, gap)
    right = sneakers_low.mirror(left)
    if right_foot is not None:
        combined.right_moved = clear_of(right, right_foot, gap)
    nl = len(left.P)
    P = np.vstack([np.array(left.P), np.array(right.P)])
    faces = list(left.faces) + [tuple(i + nl for i in f) for f in right.faces]
    mats = list(left.material) + list(right.material)
    seams = set(left.seams) | {(a + nl, b + nl) for a, b in right.seams}
    part = np.array(list(left.part) + list(right.part))
    side = np.where(np.arange(len(P)) < nl, 1, -1)
    me = bpy.data.meshes.new(NAME)
    me.from_pydata(P.tolist(), [], faces)
    me.update()
    assert len(me.polygons) == len(faces), "faces dropped"
    ob = bpy.data.objects.new(NAME, me)
    bpy.context.scene.collection.objects.link(ob)
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    for f, mat in zip(bm.faces, mats):
        f.material_index = INTERNAL.index(mat)
    for e in bm.edges:
        a, b = e.verts[0].index, e.verts[1].index
        e.seam = (min(a, b), max(a, b)) in seams
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(me)
    bm.free()
    for poly in me.polygons:
        poly.use_smooth = True
    return ob, P, part, side, np.array([INTERNAL[p.material_index] for p in me.polygons])


def orient_upper(ob, foot, part):
    """The upper is an open shell: its outside must face away from the foot
    (recalculating normals only makes each piece agree with itself)."""
    me = ob.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    flips = []
    for side in (1, -1):
        faces = [f for f in bm.faces if INTERNAL[f.material_index] == "Upper"
                 and part[f.verts[0].index] in ("upper", "lining") and f.calc_center_median().x * side > 0]
        score = 0.0
        for f in faces:
            c = np.array(f.calc_center_median())
            c[0] *= side
            n = np.array(f.normal)
            n[0] *= side
            score += float(n @ foot.normal(c)[0])
        if score < 0:
            region = [f for f in bm.faces if part[f.verts[0].index] in ("upper", "lining")
                      and f.calc_center_median().x * side > 0]
            bmesh.ops.reverse_faces(bm, faces=region)
            flips.append(side)
    bm.to_mesh(me)
    bm.free()
    return flips


def unwrap(ob, pack_after=True):
    """Texture coordinates for the full-detail level: Blender's minimum
    stretch unwrap along the seams, islands at one scale, packed."""
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    # The UV operators act on the selected faces; with selection synced
    # those are the mesh's.
    bpy.context.scene.tool_settings.use_uv_select_sync = True
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.unwrap(method="MINIMUM_STRETCH", margin=0.002, iterations=UNWRAP_ITERATIONS)
    bpy.ops.object.mode_set(mode="OBJECT")
    ob.select_set(False)
    if pack_after:
        pack(ob)


def pack(ob):
    """The islands at one texel density (each scaled to its true size),
    each turned to its narrowest box, packed in shelves (tallest first)
    into the square, as large as fits with `PACK_MARGIN` between them."""
    me = ob.data
    uv = loop_uvs(ob)
    island = islands(ob)
    P = np.array([v.co for v in me.vertices])
    loop_vert = np.array([l.vertex_index for l in me.loops])
    loop_poly = np.zeros(len(me.loops), dtype=int)
    for p in me.polygons:
        loop_poly[list(p.loop_indices)] = p.index
    loop_isl = island[loop_poly]
    tri, loops, polys = triangles_of(ob)
    out = uv.copy()
    boxes = {}
    for isl in np.unique(island):
        ls = np.nonzero(loop_isl == isl)[0]
        sel = island[polys] == isl
        t3 = P[tri[sel]]
        a3 = np.linalg.norm(np.cross(t3[:, 1] - t3[:, 0], t3[:, 2] - t3[:, 0]), axis=1).sum() / 2
        tu = uv[loops[sel]]
        au = np.abs((tu[:, 1, 0] - tu[:, 0, 0]) * (tu[:, 2, 1] - tu[:, 0, 1])
                    - (tu[:, 2, 0] - tu[:, 0, 0]) * (tu[:, 1, 1] - tu[:, 0, 1])).sum() / 2
        k = math.sqrt(a3 / max(au, 1e-18))          # metres per UV unit
        q = (uv[ls] - uv[ls].mean(axis=0)) * k
        # Turned so its box is narrowest (the principal axis along x).
        cov = np.cov(q.T)
        ev, vec = np.linalg.eigh(cov)
        q = q @ vec[:, ::-1]
        q -= q.min(axis=0)
        out[ls] = q
        boxes[isl] = (ls, q.max(axis=0))
    margin = PACK_MARGIN

    def layout(width):
        order = sorted(boxes, key=lambda i: -boxes[i][1][1])
        x = y = shelf = 0.0
        place = {}
        for i in order:
            w, h = boxes[i][1] + margin
            if x + w > width and x > 0:
                x, y, shelf = 0.0, y + shelf, 0.0
            place[i] = (x, y)
            x += w
            shelf = max(shelf, h)
        return place, y + shelf

    total = sum(np.prod(b[1] + margin) for b in boxes.values())
    widest = max(b[1][0] + margin for b in boxes.values())
    best = None
    for width in np.linspace(max(widest, math.sqrt(total)), 2.5 * math.sqrt(total), 80):
        place, height = layout(width)
        side = max(width, height)
        if best is None or side < best[0]:
            best = (side, place)
    side, place = best
    for i, (ls, _) in boxes.items():
        out[ls] = (out[ls] + np.array(place[i]) + margin / 2) / side
    me.uv_layers.active.data.foreach_set("uv", out.reshape(-1))


def loop_uvs(ob):
    me = ob.data
    uv = np.zeros(len(me.loops) * 2)
    me.uv_layers.active.data.foreach_get("uv", uv)
    return uv.reshape(-1, 2)


def triangles_of(ob):
    """Triangles (corner ids), their loop ids and polygon ids."""
    me = ob.data
    tris, loops, polys = [], [], []
    for p in me.polygons:
        ls = list(p.loop_indices)
        vs = [me.loops[k].vertex_index for k in ls]
        for k in range(1, len(vs) - 1):
            tris.append((vs[0], vs[k], vs[k + 1]))
            loops.append((ls[0], ls[k], ls[k + 1]))
            polys.append(p.index)
    return np.array(tris), np.array(loops), np.array(polys)


def islands(ob):
    """Each polygon's texture island (faces joined across edges that are
    not seams)."""
    me = ob.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    island = np.full(len(bm.faces), -1)
    k = 0
    for f in bm.faces:
        if island[f.index] >= 0:
            continue
        stack = [f]
        island[f.index] = k
        while stack:
            g = stack.pop()
            for e in g.edges:
                if e.seam:
                    continue
                for h in e.link_faces:
                    if island[h.index] < 0:
                        island[h.index] = k
                        stack.append(h)
        k += 1
    bm.free()
    return island


def project_uvs(ob, ref, ref_island, ref_part, part):
    """Texture coordinates for a simpler level from the full one's: each
    face takes the island its middle is nearest to (the same part and
    shoe), and each of its corners the coordinate of the nearest place on
    that island."""
    rme = ref.data
    rP = np.array([v.co for v in rme.vertices])
    ruv = loop_uvs(ref)
    rtri, rloop, rpoly = triangles_of(ref)
    tri_island = ref_island[rpoly]
    by_island = {}
    for isl in np.unique(tri_island):
        sel = np.nonzero(tri_island == isl)[0]
        by_island[isl] = (sel, BVHTree.FromPolygons([tuple(p) for p in rP], [tuple(t) for t in rtri[sel]]))
    # Islands by part and side, for finding a face's island.
    keys = {}
    for isl, (sel, _) in by_island.items():
        t = rtri[sel[0]]
        keys.setdefault((ref_part[t[0]], rP[t[0]][0] > 0), []).append(isl)
    me = ob.data
    out = np.zeros((len(me.loops), 2))
    P = np.array([v.co for v in me.vertices])
    for poly in me.polygons:
        vs = list(poly.vertices)
        c = P[vs].mean(axis=0)
        best = None
        for isl in keys.get((part[vs[0]], c[0] > 0), []):
            sel, bvh = by_island[isl]
            loc, nor, idx, dist = bvh.find_nearest(Vector(c))
            if loc is not None and (best is None or dist < best[0]):
                best = (dist, isl)
        sel, bvh = by_island[best[1]]
        for li, vi in zip(poly.loop_indices, vs):
            loc, nor, idx, dist = bvh.find_nearest(Vector(P[vi]))
            t = sel[idx]
            a, b, cc = rP[rtri[t]]
            w = barycentric(np.array(loc), a, b, cc)
            out[li] = w[0] * ruv[rloop[t][0]] + w[1] * ruv[rloop[t][1]] + w[2] * ruv[rloop[t][2]]
    return out


def fit_islands(ob, target, density):
    """A simpler level's own unwrap (minimum stretch, its own seams: so its
    texture coordinates are as even as the full level's), each island then
    moved, turned and scaled (mirrored if its unwrap came out mirrored)
    onto where `target` (the projection onto the full level) puts it, so
    it reads the same texels."""
    unwrap(ob, pack_after=False)
    own = loop_uvs(ob)
    island = islands(ob)
    me = ob.data
    loop_poly = np.zeros(len(me.loops), dtype=int)
    for p in me.polygons:
        loop_poly[list(p.loop_indices)] = p.index
    P = np.array([v.co for v in me.vertices])
    tri, loops, polys = triangles_of(ob)
    out = target.copy()
    for isl in np.unique(island):
        ls = np.nonzero(island[loop_poly] == isl)[0]
        # The island at the full level's density (UV units per metre),
        # from its own area.
        sel = island[polys] == isl
        t3 = P[tri[sel]]
        a3 = np.linalg.norm(np.cross(t3[:, 1] - t3[:, 0], t3[:, 2] - t3[:, 0]), axis=1).sum() / 2
        tu = own[loops[sel]]
        au = np.abs((tu[:, 1, 0] - tu[:, 0, 0]) * (tu[:, 2, 1] - tu[:, 0, 1])
                    - (tu[:, 2, 0] - tu[:, 0, 0]) * (tu[:, 1, 1] - tu[:, 0, 1])).sum() / 2
        scale = density * math.sqrt(a3 / max(au, 1e-18))
        a, b = own[ls] * scale, target[ls]
        ca, cb = a.mean(axis=0), b.mean(axis=0)
        A, Bm = a - ca, b - cb
        # Turned (or mirrored) onto the target: orthogonal Procrustes.
        u, sv, vt = np.linalg.svd(A.T @ Bm)
        out[ls] = A @ (u @ vt) + cb
    me.uv_layers.active.data.foreach_set("uv", out.reshape(-1))
    return out


def barycentric(p, a, b, c):
    v0, v1, v2 = b - a, c - a, p - a
    d00, d01, d11 = v0 @ v0, v0 @ v1, v1 @ v1
    d20, d21 = v2 @ v0, v2 @ v1
    den = d00 * d11 - d01 * d01
    if abs(den) < 1e-20:
        return np.array([1.0, 0.0, 0.0])
    v = (d11 * d20 - d01 * d21) / den
    w = (d00 * d21 - d01 * d20) / den
    return np.clip(np.array([1 - v - w, v, w]), 0, 1)


def evenness(ob, P, uv, fmat):
    """asset-quality §2's texture measures per shipped material, as the
    avatar test reads them: the share of the area whose stretch (both of
    the map's scales) is within 10% of the material's median, and the
    share at 8 px/cm or more at 2048."""
    tri, loops, polys = triangles_of(ob)
    mats = np.array([SHIPPED.get(m, m) for m in fmat[polys]])
    out = {}
    for name in MATERIALS:
        sel = mats == name
        if not sel.any():
            continue
        p = P[tri[sel]]
        t = uv[loops[sel]] * textures.SIZE
        e1, e2 = p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]
        area = np.linalg.norm(np.cross(e1, e2), axis=1) / 2
        x = e1 / np.maximum(np.linalg.norm(e1, axis=1, keepdims=True), 1e-12)
        y = np.cross(np.cross(e1, e2), e1)
        y /= np.maximum(np.linalg.norm(y, axis=1, keepdims=True), 1e-12)
        p1 = np.stack([(e1 * x).sum(1), np.zeros(len(x))], 1)
        p2 = np.stack([(e2 * x).sum(1), (e2 * y).sum(1)], 1)
        t1, t2 = t[:, 1] - t[:, 0], t[:, 2] - t[:, 0]
        det = p1[:, 0] * p2[:, 1] - p2[:, 0] * p1[:, 1]
        det = np.where(np.abs(det) < 1e-15, 1e-15, det)
        a = (t1[:, 0] * p2[:, 1] - t2[:, 0] * p1[:, 1]) / det
        b = (-t1[:, 0] * p2[:, 0] + t2[:, 0] * p1[:, 0]) / det
        c = (t1[:, 1] * p2[:, 1] - t2[:, 1] * p1[:, 1]) / det
        d = (-t1[:, 1] * p2[:, 0] + t2[:, 1] * p1[:, 0]) / det
        s1 = a * a + b * b + c * c + d * d
        s2 = np.sqrt((a * a + b * b - c * c - d * d) ** 2 + 4 * (a * c + b * d) ** 2)
        hi = np.sqrt(np.maximum((s1 + s2) / 2, 0))
        lo = np.sqrt(np.maximum((s1 - s2) / 2, 0))
        ok = area > 1e-12
        med = np.median(np.sqrt(hi[ok] * lo[ok]))
        even = area[ok & (hi / med - 1 < 0.1) & (1 - lo / med < 0.1)].sum() / area[ok].sum()
        dense = area[ok & (lo >= 800)].sum() / area[ok].sum()
        out[name] = [round(float(even), 4), round(float(dense), 4)]
        if os.environ.get("AXCENITY_UV_DEBUG"):
            bad = np.nonzero(ok & (lo < 800))[0]
            centre = p.mean(axis=1)
            print(name, "sparse", [(np.round(centre[k] * 1000).tolist(), round(float(lo[k])), round(float(area[k] * 1e6), 2))
                                   for k in bad[:6]])
            uneven = ok & ~((hi / med - 1 < 0.1) & (1 - lo / med < 0.1))
            print(name, "uneven at", np.round(centre[uneven][:10] * 1000).tolist())
    return out


def corner_normals(ob):
    """Smooth normals per corner, the same for every corner at one place of
    one piece (the mesh's own vertex normals)."""
    me = ob.data
    n = np.zeros(len(me.vertices) * 3)
    me.vertices.foreach_get("normal", n)
    return n.reshape(-1, 3)


class Skin:
    """The body's skin weights for the shin's share and its shapes, by the
    nearest skin of each foot."""

    def __init__(self, b):
        self.b = b
        obj = b.object
        groups = {g.index: g.name for g in obj.vertex_groups}
        n = len(b.P)
        self.shin = {1: np.zeros(n), -1: np.zeros(n)}
        self.twist = {1: np.zeros(n), -1: np.zeros(n)}
        self.toes = {1: np.zeros(n), -1: np.zeros(n)}
        for v in obj.data.vertices:
            for g in v.groups:
                name = groups[g.group]
                for side, prefix in ((1, "Left"), (-1, "Right")):
                    if name == f"{prefix}LowerLeg":
                        self.shin[side][v.index] += g.weight
                    elif name == f"{prefix}LowerLegTwist":
                        self.twist[side][v.index] += g.weight
                    elif name == f"{prefix}Toes":
                        self.toes[side][v.index] += g.weight
        self.trees = {}
        for side in (1, -1):
            sel = np.nonzero((b.P[:, 0] * side > 0.02) & (b.P[:, 2] < 0.2))[0]
            self.trees[side] = sel

    def share(self, side, p):
        """The shin's and its twist bone's share of the skin near `p`: a
        Gaussian mean (`SKIN_SIGMA`) round the nearest skin, so the shoe's
        weights are smooth where the skin's change from one corner to the
        next."""
        sel = self.trees[side]
        d = np.linalg.norm(self.b.P[sel] - p, axis=1)
        near = self.b.P[sel[int(np.argmin(d))]]
        r = np.linalg.norm(self.b.P[sel] - near, axis=1)
        w = np.exp(-0.5 * (r / SKIN_SIGMA) ** 2)
        w /= w.sum()
        return (float(w @ self.shin[side][sel]), float(w @ self.twist[side][sel]),
                float(w @ self.toes[side][sel]))

    def shapes(self, foot_points_side):
        """For each shape that moves the foot: per side an affine map
        (3x3, 3) of the foot's skin under the shoe, the height part with no
        tilt."""
        b = self.b
        me = b.object.data
        keys = me.shape_keys.key_blocks
        n = len(b.P)
        world = np.array(b.object.matrix_world)
        basis = np.zeros(n * 3)
        keys["Basis"].data.foreach_get("co", basis)
        basis = basis.reshape(n, 3)
        out = {}
        for k in keys:
            if k.name == "Basis":
                continue
            c = np.zeros(n * 3)
            k.data.foreach_get("co", c)
            delta = (c.reshape(n, 3) - basis) @ world[:3, :3].T
            maps = {}
            moved = 0.0
            for side in (1, -1):
                sel = foot_points_side[side]
                X = b.P[sel]
                D = delta[sel]
                moved = max(moved, float(np.abs(D).max()))
                A1 = np.column_stack([X, np.ones(len(X))])
                sol, *_ = np.linalg.lstsq(A1, D[:, :2], rcond=None)
                # Height: from height alone (no tilt across or along).
                Az = np.column_stack([X[:, 2], np.ones(len(X))])
                solz, *_ = np.linalg.lstsq(Az, D[:, 2], rcond=None)
                A = np.zeros((3, 3))
                A[:2, :] = sol[:3].T
                A[2, 2] = solz[0]
                # The height's offset: the foot's lowest point's move (the
                # sole stands where the foot stands; the frame's bones move
                # the shoe as they move the foot).
                low = (X[:, 2] + D[:, 2]).min() - X[:, 2].min()
                t = np.array([sol[3, 0], sol[3, 1], low])
                maps[side] = (A, t)
            if moved > SHAPE_MIN:
                out[k.name] = maps
        return out


def weights_of(P, part, side, foot, skin):
    """Each corner's bones and weights (see the module's notes)."""
    names, W = [], []
    for p, pt, sd in zip(P, part, side):
        q = p.copy()
        q[0] *= sd
        s = float(foot.s_of(q))
        prefix = "Left" if sd > 0 else "Right"
        # The toes' share: the skin's own across the ball (so the vamp's
        # sides bend as the foot's do when the toes turn up), all of it in
        # the toe box (`TOE_BOX`: it turns with the toes as one piece).
        a_, b_, t_ = skin.share(sd, p)
        box = float(sneakers_last.smoothstep(TOE_BOX - 0.04, TOE_BOX, s))
        behind = 1 - float(sneakers_last.smoothstep(foot.toe_joint - 0.12, foot.toe_joint - 0.06, s))
        toes = (t_ * (1 - behind)) * (1 - box) + box
        # The shin's share: the skin's under the collar (faded in with
        # height), and all the skin's round the heel, where the body's own
        # skinning carries the heel's skin with the shin as the ankle turns
        # (a heel cup that didn't would let the heel's skin out of it).
        ramp = float(sneakers_last.smoothstep(SHIN_FROM, SHIN_TO, p[2]))
        heel = 1 - float(sneakers_last.smoothstep(HEEL_FROM, HEEL_TO, s))
        follow = max(ramp if pt != "sole" else 0.0, heel)
        shin = twist = 0.0
        if follow > 0:
            shin, twist = a_ * follow * (1 - toes), b_ * follow * (1 - toes)
        rest = 1 - shin - twist
        w = {f"{prefix}Foot": rest * (1 - toes), f"{prefix}Toes": rest * toes,
             f"{prefix}LowerLeg": shin, f"{prefix}LowerLegTwist": twist}
        w = {k: v for k, v in w.items() if v > 1e-4}
        total = sum(w.values())
        names.append(list(w))
        W.append([v / total for v in w.values()])
    return names, W


def export(ob, P, part, side, uvs, foot, skin, shapes, path):
    """The level as an OBJ in glTF's axes (a vertex per place and texture
    coordinate) and its sidecar (weights by bone, shape offsets)."""
    me = ob.data
    normals = corner_normals(ob)
    corner_vertex = np.array([l.vertex_index for l in me.loops])
    keys = np.concatenate([corner_vertex[:, None].astype(float), np.round(uvs, 6)], axis=1)
    unique, of_corner = np.unique(keys, axis=0, return_inverse=True)
    of_corner = of_corner.reshape(-1)
    vert = unique[:, 0].astype(int)
    stem = os.path.splitext(path)[0]
    mtl = os.path.basename(stem) + ".mtl"
    with open(stem + ".mtl", "w", newline="\n") as f:
        for name in MATERIALS:
            f.write(f"newmtl {name}\nKd 0.9 0.9 0.9\nmap_Kd {NAME}_color.jpg\n")
    eng = lambda a: np.column_stack([a[:, 0], a[:, 2], -a[:, 1]])  # noqa: E731
    with open(path, "w", newline="\n") as f:
        f.write("# The Axcenity Engine's low-top sneakers (assets/characters/realistic/clothing/build_sneakers_low.py)\n")
        f.write(f"mtllib {mtl}\n")
        for p in eng(P[vert]):
            f.write(f"v {p[0]:.6f} {p[1]:.6f} {p[2]:.6f}\n")
        for t in unique[:, 1:]:
            f.write(f"vt {t[0]:.6f} {t[1]:.6f}\n")
        for n in eng(normals[vert]):
            f.write(f"vn {n[0]:.5f} {n[1]:.5f} {n[2]:.5f}\n")
        by_mat = {}
        for poly in me.polygons:
            mat = SHIPPED.get(INTERNAL[poly.material_index], INTERNAL[poly.material_index])
            ls = list(poly.loop_indices)
            for k in range(1, len(ls) - 1):
                by_mat.setdefault(mat, []).append((ls[0], ls[k], ls[k + 1]))
        for mat in MATERIALS:
            if mat not in by_mat:
                continue
            f.write(f"usemtl {mat}\n")
            for tri in by_mat[mat]:
                f.write("f " + " ".join(f"{of_corner[c] + 1}/{of_corner[c] + 1}/{of_corner[c] + 1}" for c in tri) + "\n")
    names, W = weights_of(P[vert], part[vert], side[vert], foot, skin)
    bones = sorted({n for ns in names for n in ns})
    J = np.zeros((len(vert), 4), dtype=np.int32)
    WW = np.zeros((len(vert), 4), dtype=np.float32)
    for i, (ns, ws) in enumerate(zip(names, W)):
        for k, (n_, w_) in enumerate(zip(ns, ws)):
            J[i, k] = bones.index(n_)
            WW[i, k] = w_
    shape_names = sorted(shapes)
    D = np.zeros((len(shape_names), len(vert), 3), dtype=np.float32)
    Pv = P[vert]
    for k, name in enumerate(shape_names):
        for sd in (1, -1):
            sel = side[vert] == sd
            A, t = shapes[name][sd]
            D[k, sel] = Pv[sel] @ A.T + t
        D[k] = eng(D[k])
    np.savez_compressed(stem + ".npz", positions=eng(Pv).astype(np.float32), uvs=unique[:, 1:].astype(np.float32),
                        normals=eng(normals[vert]).astype(np.float32), bones=np.array(bones), joints=J,
                        weights=WW, shape_names=np.array(shape_names), shape_deltas=D,
                        part=part[vert], side=side[vert])
    return {"vertices": int(len(vert)), "triangles": int(sum(len(v) for v in by_mat.values())),
            "bones": bones, "shapes": len(shape_names)}


def measure_foot(P, side=1):
    """A foot's length (heel to the longest toe) and its width across the
    ball, from the skin under 3 cm (m)."""
    sel = (P[:, 0] * side > 0.03) & (P[:, 2] < 0.03)
    low = P[sel]
    heel = low[np.argmax(low[:, 1])]
    toe = low[np.argmin(low[:, 1])]
    d = (toe - heel)[:2]
    length = float(np.linalg.norm(d))
    d /= length
    lat = np.array([-d[1], d[0]])
    s = (low[:, :2] - heel[:2]) @ d / length
    w = (low[:, :2] - heel[:2]) @ lat
    width = max(float(np.ptp(w[np.abs(s - sc) < 0.02])) for sc in np.linspace(0.55, 0.8, 26)
                if (np.abs(s - sc) < 0.02).sum() > 3)
    return {"length_mm": round(length * 1000, 1), "width_mm": round(width * 1000, 1)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--body", required=True)
    parser.add_argument("--tiles", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--no-maps", action="store_true")
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    t0 = time.time()
    fem = bodymod.load(args.body, "feminine")
    report = {"foot": {"feminine": measure_foot(fem.P)}}
    b = bodymod.load(args.body, "masculine")
    report["foot"]["masculine"] = measure_foot(b.P)
    foot = sneakers_last.Foot(b, 1)
    foot_r = sneakers_last.Foot(b, -1)
    combined.left_foot = foot
    report["last"] = {"length_mm": round(foot.length * 1000, 1), "ball_width_mm": round(foot.width * 1000, 1),
                      "toe_joint_s": round(foot.toe_joint, 3), "voxel_mm": sneakers_last.VOXEL * 1000,
                      "blur_mm": sneakers_last.BLUR, "toe_room_mm": sneakers_last.TOE_ROOM * 1000}
    skin = Skin(b)
    foot_points = {sd: np.nonzero((b.P[:, 0] * sd > 0.03) & (b.P[:, 2] < 0.09))[0] for sd in (1, -1)}
    shapes = skin.shapes(foot_points)
    report["shapes"] = sorted(shapes)
    objects, data = [], []
    for level in range(3):
        shoe = sneakers_low.Shoe(foot, level)
        ob, P, part, side, fmat = combined(shoe.mesh, foot_r, RIGHT_GAP)
        shoe.report["cleared_mm"] = {"left": round(combined.left_moved * 1000, 2),
                                     "right": round(combined.right_moved * 1000, 2)}
        flips = orient_upper(ob, foot, part)
        ob.name = f"{NAME}_LOD{level}"
        objects.append(ob)
        data.append((shoe, P, part, side, fmat, flips))
    # Texture coordinates: the full level unwrapped, the others projected.
    unwrap(objects[0])
    ref_island = islands(objects[0])
    uv_levels = [loop_uvs(objects[0])]
    # The full level's density: UV units per metre.
    tri0, loops0, _ = triangles_of(objects[0])
    P0_ = data[0][1]
    a3 = np.linalg.norm(np.cross(P0_[tri0[:, 1]] - P0_[tri0[:, 0]], P0_[tri0[:, 2]] - P0_[tri0[:, 0]]), axis=1).sum()
    u0 = uv_levels[0][loops0]
    au = np.abs((u0[:, 1, 0] - u0[:, 0, 0]) * (u0[:, 2, 1] - u0[:, 0, 1])
                - (u0[:, 2, 0] - u0[:, 0, 0]) * (u0[:, 1, 1] - u0[:, 0, 1])).sum()
    density = math.sqrt(au / a3)
    for level in (1, 2):
        _, P, part, side, _, _ = data[level]
        target = project_uvs(objects[level], objects[0], ref_island, data[0][2], part)
        if not objects[level].data.uv_layers:
            objects[level].data.uv_layers.new(name="UVMap")
        uv_levels.append(fit_islands(objects[level], target, density))
    # Texel density (texels per mm at 2048).
    me0 = objects[0].data
    tri, loops, _ = triangles_of(objects[0])
    P0 = data[0][1]
    area3 = np.linalg.norm(np.cross(P0[tri[:, 1]] - P0[tri[:, 0]], P0[tri[:, 2]] - P0[tri[:, 0]]), axis=1).sum() / 2
    uv0 = uv_levels[0]
    a = uv0[loops[:, 1]] - uv0[loops[:, 0]]
    c = uv0[loops[:, 2]] - uv0[loops[:, 0]]
    area_uv = np.abs(a[:, 0] * c[:, 1] - a[:, 1] * c[:, 0]).sum() / 2
    scale = math.sqrt(area_uv * textures.SIZE ** 2 / (area3 * 1e6))
    report["atlas"] = {"texels_per_mm": round(scale, 2), "used": round(float(area_uv), 3)}
    # Each island's scale against the whole's (they should all be one).
    isl_t = ref_island[_]
    ratios = []
    for isl in np.unique(isl_t):
        sel = isl_t == isl
        a3 = np.linalg.norm(np.cross(P0[tri[sel, 1]] - P0[tri[sel, 0]], P0[tri[sel, 2]] - P0[tri[sel, 0]]),
                            axis=1).sum() / 2
        au = np.abs(a[sel, 0] * c[sel, 1] - a[sel, 1] * c[sel, 0]).sum() / 2
        ratios.append(math.sqrt(au / max(a3, 1e-12)) / math.sqrt(area_uv / area3))
    report["atlas"]["islands"] = len(ratios)
    report["atlas"]["island_scale_range"] = [round(min(ratios), 3), round(max(ratios), 3)]
    report["levels"] = []
    report["uv"] = {}
    for level in range(3):
        report["uv"][f"LOD{level}"] = evenness(objects[level], data[level][1], uv_levels[level], data[level][4])
    for level in range(3):
        shoe, P, part, side, fmat, flips = data[level]
        out = export(objects[level], P, part, side, uv_levels[level], foot, skin, shapes,
                     os.path.join(args.out, f"{NAME}_LOD{level}.obj"))
        out["upper_turned"] = flips
        tri_, _, poly_ = triangles_of(objects[level])
        ar = np.linalg.norm(np.cross(P[tri_[:, 1]] - P[tri_[:, 0]], P[tri_[:, 2]] - P[tri_[:, 0]]), axis=1) / 2
        bad = np.nonzero(ar < 1e-9)[0]
        out["degenerate"] = [[round(float(c) * 1000, 1) for c in P[tri_[k]].mean(axis=0)] + [str(fmat[poly_[k]])]
                             for k in bad[:6]]
        out.update(shoe.report)
        raw = foot._sample(foot.raw, np.column_stack([np.abs(P[:, 0]), P[:, 1], P[:, 2]]))
        out["least_off_foot_mm"] = {pt: round(float(raw[part == pt].min() * 1000), 2) for pt in np.unique(part)}
        report["levels"].append(out)
        print(f"LOD{level}: {out['triangles']} triangles, {out['vertices']} vertices")
    if not args.no_maps:
        shoe, P, part, side, fmat, _ = data[0]
        feats = sneakers_low.features(shoe)
        feats["foot_length_mm"] = foot.length * 1000

        def frame(p):
            s = foot.s_of(p)
            w = foot.w_of(p) - np.interp(s, foot.mid_s, foot.mid_w_table)
            return s, w, p[:, 2]

        tiles_dir = args.tiles

        def load(name, kind):
            return os.path.join(tiles_dir, name, f"{name}_{kind}_2k.jpg")

        tiles = {
            "leather": {"mm": 300.0, "height": textures.load_tile(load("leather_white", "disp"), 300.0, scale),
                        "light": textures.load_tile(load("leather_white", "diff"), 300.0, scale), "light_mean": 0.55,
                        "rough": textures.load_tile(load("leather_white", "rough"), 300.0, scale)},
            "jersey": {"mm": 263.6, "height": textures.load_tile(load("cotton_jersey", "disp"), 263.6, scale),
                       "light": textures.load_tile(load("cotton_jersey", "diff"), 263.6, scale), "light_mean": 0.6,
                       "rough": textures.load_tile(load("cotton_jersey", "rough"), 263.6, scale)},
            "rubber": {"mm": 200.0, "height": textures.load_tile(
                os.path.join(tiles_dir, "rubber004", "Rubber004_1K-JPG_Displacement.jpg"), 200.0, scale)},
        }
        nor = corner_normals(objects[0])
        # One texture coordinate per corner place: the bake reads per
        # triangle corner, so give it the loops' coordinates directly.
        tri, loops, polys = triangles_of(objects[0])
        n_l = len(loops) * 3
        pos_l = P[tri].reshape(-1, 3)
        nor_l = nor[tri].reshape(-1, 3)
        uv_l = uv0[loops].reshape(-1, 2).copy()
        uv_l[:, 1] = 1 - uv_l[:, 1]
        tri_l = np.arange(n_l).reshape(-1, 3)
        mat_t = fmat[polys]
        side_l = side[tri].reshape(-1)
        report["bake"] = sneakers_maps.bake(tri_l, uv_l, pos_l, nor_l, mat_t, side_l, frame, feats, tiles, scale,
                                            os.path.join(args.out, NAME))
    report["seconds"] = round(time.time() - t0, 1)
    with open(os.path.join(args.out, f"{NAME}.json"), "w", newline="\n") as f:
        json.dump(report, f, indent=1, default=float)
    print(json.dumps(report, indent=1, default=float))


if __name__ == "__main__":
    main()
