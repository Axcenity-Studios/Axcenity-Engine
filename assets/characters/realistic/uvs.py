"""The body's texture coordinates: one square for a 2K or 4K texture set.

The base mesh comes with a sculpting layout: forty islands over a grid of
UDIM tiles, with a seam down the middle of the face and the chest. A game
character wants few seams, out of sight, so the body is cut again here
and unwrapped (Blender's minimum-stretch unwrap):

- the head and neck as one island, cut up the back of the head;
- the torso as a front and a back, cut down the sides, over the tops of
  the shoulders and through the crotch;
- each arm and each leg as one island, cut along its inner side.

The hands and feet (a back and a palm, a whole foot), the ears, the eye
sockets and the inside of the mouth keep the bundle's own islands. Every island is
then resized to one texel density (how much skin a texel covers), the
face given more and the hidden insides less, and all are packed into the
square with a margin for the bakes' and mip maps' bleed. Nothing
overlaps: left and right have their own texels.
"""

import heapq

import bmesh
import bpy
import numpy as np

from common import (
    FS_FINGERS, FS_FOOT, FS_FOREARM, FS_HEAD, FS_MUZZLE, FS_PALM, FS_PELVIS, FS_SHIN, FS_SOCKET,
    FS_THIGH, FS_TORSO, FS_UPPER_ARM, FS_WAIST, HEAD_SETS, MOUTH_SETS, array, face_sets, select_only,
)

# Texel density of a face set, as a multiple of the body's.
FACE_DENSITY = 2.2
HAND_DENSITY = 1.15
HIDDEN_DENSITY = 0.4
# The gap between islands, as a share of the square (about 6 texels at 2K).
MARGIN = 0.003
# Blender's minimum-stretch unwrap and how long it's given to settle.
UNWRAP = "MINIMUM_STRETCH"
ITERATIONS = 200


def _density(sets):
    hidden = {*MOUTH_SETS, *FS_SOCKET.values()}
    hands = {*FS_PALM.values(), *(fs for side in FS_FINGERS.values() for group in side.values() for fs in group)}
    out = []
    for fs in sets:
        if fs in hidden:
            out.append(HIDDEN_DENSITY)
        elif fs in HEAD_SETS:
            out.append(FACE_DENSITY)
        elif fs in hands:
            out.append(HAND_DENSITY)
        else:
            out.append(1.0)
    return out


def islands(bm, uv):
    """The mesh's UV islands as lists of faces: faces joined across edges
    whose two sides share their texture coordinates."""
    seen, out = set(), []
    for start in bm.faces:
        if start.index in seen:
            continue
        seen.add(start.index)
        stack, island = [start], []
        while stack:
            face = stack.pop()
            island.append(face)
            for loop in face.loops:
                for other in loop.link_loops:
                    f = other.face
                    if f.index in seen:
                        continue
                    # `other` runs the shared edge the opposite way.
                    a, b = loop[uv].uv, loop.link_loop_next[uv].uv
                    c, d = other.link_loop_next[uv].uv, other[uv].uv
                    if (a - c).length < 1e-6 and (b - d).length < 1e-6:
                        seen.add(f.index)
                        stack.append(f)
        out.append(island)
    return out


def _path(bm, X, start, goals, allowed, pull=40.0, line=None):
    """The vertices of a path along edges from `start` to the nearest of
    `goals`, through `allowed` vertices only, staying near the straight
    line between its ends (or the function `line(point)` giving how far
    off course a point is, in metres)."""
    goals = set(goals)
    if line is None:
        a = X[start]
        b = X[min(goals, key=lambda g: np.linalg.norm(X[g] - a))]
        d = (b - a) / max(np.linalg.norm(b - a), 1e-9)

        def line(p):
            rel = p - a
            return float(np.linalg.norm(rel - d * (rel @ d)))
    dist = {start: 0.0}
    prev = {}
    heap = [(0.0, start)]
    while heap:
        cost, i = heapq.heappop(heap)
        if cost > dist.get(i, np.inf):
            continue
        if i in goals:
            out = [i]
            while out[-1] in prev:
                out.append(prev[out[-1]])
            return out[::-1]
        for e in bm.verts[i].link_edges:
            j = e.other_vert(bm.verts[i]).index
            if j not in allowed:
                continue
            mid = (X[i] + X[j]) / 2
            step = float(np.linalg.norm(X[i] - X[j])) * (1.0 + pull * line(mid))
            if cost + step < dist.get(j, np.inf):
                dist[j] = cost + step
                prev[j] = i
                heapq.heappush(heap, (cost + step, j))
    raise RuntimeError("no seam path between the landmarks")


def cut(body, lm):
    """Marks the seams (see the module's text) on `body` and returns the
    indices of the faces to unwrap again."""
    me = body.data
    sets = face_sets(body)
    X = array(v.co for v in me.vertices)
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    bm.edges.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    for e in bm.edges:
        e.seam = False

    def seam_path(verts):
        for a, b in zip(verts, verts[1:]):
            bm.edges.get((bm.verts[a], bm.verts[b])).seam = True

    def verts_of(face_ids):
        return {v.index for f in face_ids for v in bm.faces[f].verts}

    def region(set_ids):
        set_ids = set(set_ids)
        return [f.index for f in bm.faces if sets[f.index] in set_ids]

    def outline(face_ids):
        """Seams round a group of faces."""
        inside = set(face_ids)
        for f in face_ids:
            for e in bm.faces[f].edges:
                if any(g.index not in inside for g in e.link_faces) or len(e.link_faces) < 2:
                    e.seam = True

    def far(ring, centre, direction):
        return max(ring, key=lambda i: (X[i] - centre) @ direction)

    again = []
    # The inside of the mouth keeps the bundle's islands, which start at the
    # line where the lips meet (they take the lips' inner sides with them).
    uv = bm.loops.layers.uv.verify()
    inside_mouth = {f.index for island in islands(bm, uv)
                    if any(sets[f.index] in MOUTH_SETS for f in island) for f in island}
    # The head and neck: cut up the back's middle line from the neck's base
    # to the crown.
    head = [f for f in region([FS_HEAD, FS_MUZZLE]) if f not in inside_mouth]
    outline(head)
    hv = verts_of(head)
    neck_ring = lm.ring(FS_TORSO, FS_HEAD)
    nape = max((i for i in neck_ring if abs(X[i][0]) < 0.003), key=lambda i: X[i][1])
    crown = max((i for i in hv if abs(X[i][0]) < 0.003), key=lambda i: X[i][2])
    seam_path(_path(bm, X, nape, [crown], hv, line=lambda p: abs(float(p[0])) + max(0.0, float(X[crown][1] - p[1]))))
    again += head
    # The torso: a front and a back.
    torso = region([FS_TORSO, FS_WAIST, FS_PELVIS])
    outline(torso)
    tv = verts_of(torso)
    crotch = {}
    for s in (1, -1):
        medial = np.array([-s, 0.0, 0.0])
        arm_ring = lm.ring(FS_TORSO, FS_UPPER_ARM[s])
        arm_centre = X[arm_ring].mean(axis=0)
        leg_ring = lm.ring(FS_PELVIS, FS_THIGH[s])
        leg_centre = X[leg_ring].mean(axis=0)
        armpit = min(arm_ring, key=lambda i: X[i][2])
        hip = far(leg_ring, leg_centre, -medial)
        seam_path(_path(bm, X, armpit, [hip], tv))
        top = max(arm_ring, key=lambda i: X[i][2])
        side_ring = [i for i in neck_ring if X[i][0] * s > 0]
        neck_side = far(side_ring, X[neck_ring].mean(axis=0), -medial)
        seam_path(_path(bm, X, top, [neck_side], tv))
        crotch[s] = far(leg_ring, leg_centre, medial)
    seam_path(_path(bm, X, crotch[1], [crotch[-1]], tv))
    again += torso
    for s in (1, -1):
        medial = np.array([-s, 0.0, 0.0])
        # An arm: cut along the side that faces the body.
        arm = region([FS_UPPER_ARM[s], FS_FOREARM[s]])
        outline(arm)
        arm_ring = lm.ring(FS_TORSO, FS_UPPER_ARM[s])
        wrist_ring = lm.ring(FS_FOREARM[s], FS_PALM[s])
        a, b = X[arm_ring].mean(axis=0), X[wrist_ring].mean(axis=0)
        axis = (b - a) / np.linalg.norm(b - a)
        inner = medial - axis * (medial @ axis)
        inner /= np.linalg.norm(inner)
        seam_path(_path(bm, X, far(arm_ring, a, inner), [far(wrist_ring, b, inner)], verts_of(arm)))
        again += arm
        # A leg: cut along the inseam.
        leg = region([FS_THIGH[s], FS_SHIN[s]])
        outline(leg)
        leg_ring = lm.ring(FS_PELVIS, FS_THIGH[s])
        ankle_ring = lm.ring(FS_SHIN[s], FS_FOOT[s])
        seam_path(_path(bm, X, far(leg_ring, X[leg_ring].mean(axis=0), medial),
                        [far(ankle_ring, X[ankle_ring].mean(axis=0), medial)], verts_of(leg)))
        again += leg
    bm.to_mesh(me)
    bm.free()
    return again, head


def _unwrap_closed(body, faces, keep_open):
    """Unwraps `faces` of `body` on a copy whose holes are closed with
    faces first, and copies the result back. A hole left open is free to
    spread in the flat layout, and the mouth, a slit with no area, spreads
    into a ring that squeezes the lips round it; closed with faces of no
    area it stays a slit. The hole with a vertex of `keep_open` (the
    neck's ring) stays open: it's the island's outer edge."""
    me = body.data
    copy = body.copy()
    copy.data = me.copy()
    bpy.context.scene.collection.objects.link(copy)
    bm = bmesh.new()
    bm.from_mesh(copy.data)
    origin = bm.faces.layers.int.new("origin")
    vertex = bm.verts.layers.int.new("vertex")
    for f in bm.faces:
        f[origin] = f.index
    for v in bm.verts:
        v[vertex] = v.index
    wanted = set(faces)
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.index not in wanted], context="FACES")
    keep_open = set(keep_open)
    # Each hole: a loop of edges with one face.
    edges = {e for e in bm.edges if e.is_boundary}
    while edges:
        loop, stack = [], [next(iter(edges))]
        while stack:
            e = stack.pop()
            if e not in edges:
                continue
            edges.discard(e)
            loop.append(e)
            for v in e.verts:
                stack.extend(x for x in v.link_edges if x in edges)
        if any(v[vertex] in keep_open for e in loop for v in e.verts):
            continue
        made = bmesh.ops.edgeloop_fill(bm, edges=loop)["faces"]
        for f in made:
            f[origin] = -1
        bmesh.ops.triangulate(bm, faces=made)
    for f in bm.faces:
        f.select = True
        if f[origin] < 0:
            f[origin] = -1
    bm.to_mesh(copy.data)
    bm.free()
    for poly in copy.data.polygons:
        poly.select = True
    select_only([copy])
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.context.tool_settings.mesh_select_mode = (False, False, True)
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.unwrap(method=UNWRAP, fill_holes=False, correct_aspect=True, margin=0.001, iterations=ITERATIONS)
    bpy.ops.object.mode_set(mode="OBJECT")
    target = me.uv_layers[0].data
    source = copy.data.uv_layers[0].data
    origin = copy.data.attributes["origin"].data
    for poly in copy.data.polygons:
        k = origin[poly.index].value
        if k < 0:
            continue
        for a, b in zip(me.polygons[k].loop_indices, poly.loop_indices):
            target[a].uv = source[b].uv
    bpy.data.objects.remove(copy)


def layout(body, lm):
    """Replaces `body`'s texture coordinates with the packed layout.
    Returns {islands, coverage}: how many islands, and the share of the
    square they fill."""
    me = body.data
    density = _density(face_sets(body))
    again, head = cut(body, lm)
    again = set(again) - set(head)
    for poly in me.polygons:
        poly.select = poly.index in again
    select_only([body])
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.context.tool_settings.mesh_select_mode = (False, False, True)
    bpy.ops.uv.unwrap(method=UNWRAP, fill_holes=True, correct_aspect=True, margin=0.001, iterations=ITERATIONS)
    bpy.ops.object.mode_set(mode="OBJECT")
    _unwrap_closed(body, head, lm.ring(FS_TORSO, FS_HEAD))
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    uv = bm.loops.layers.uv.verify()
    found = islands(bm, uv)
    for island in found:
        area3d = sum(f.calc_area() * density[f.index] ** 2 for f in island)
        area_uv = 0.0
        for f in island:
            pts = [loop[uv].uv for loop in f.loops]
            area_uv += abs(sum(pts[i].cross(pts[(i + 1) % len(pts)]) for i in range(len(pts)))) / 2
        scale = (area3d / max(area_uv, 1e-12)) ** 0.5
        origin = island[0].loops[0][uv].uv.copy()
        for f in island:
            for loop in f.loops:
                loop[uv].uv = (loop[uv].uv - origin) * scale
    bm.to_mesh(me)
    bm.free()
    me.uv_layers[0].name = "UVMap"
    # Pack, keeping the islands' sizes relative to each other.
    select_only([body])
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.select_all(action="SELECT")
    bpy.ops.uv.pack_islands(rotate=True, rotate_method="ANY", scale=True, margin_method="FRACTION",
                            margin=MARGIN, shape_method="CONCAVE", merge_overlap=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    covered = 0.0
    for poly in me.polygons:
        pts = [me.uv_layers[0].data[i].uv for i in poly.loop_indices]
        covered += abs(sum(pts[i].cross(pts[(i + 1) % len(pts)]) for i in range(len(pts)))) / 2
    low = min(min(d.uv) for d in me.uv_layers[0].data)
    high = max(max(d.uv) for d in me.uv_layers[0].data)
    assert low >= -1e-6 and high <= 1.0 + 1e-6, "the packed texture coordinates leave the square"
    return {"islands": len(found), "coverage": round(covered, 3), "stretch": stretch(body)}


def stretch(body, faces=None):
    """How unevenly the layout treats the skin, over the faces (all, or
    those given), by area: `p90` and `max` of each face's longer-to-shorter
    stretch (1 is none), and `density` (the 5th and 95th percentile of
    texels per metre, over the median)."""
    me = body.data
    layer = me.uv_layers[0].data
    ratios, areas, scales = [], [], []
    for poly in (me.polygons if faces is None else [me.polygons[i] for i in faces]):
        ids = list(poly.loop_indices)
        p = [np.array(me.vertices[me.loops[i].vertex_index].co) for i in ids[:3]]
        t = [np.array(layer[i].uv) for i in ids[:3]]
        e1, e2 = p[1] - p[0], p[2] - p[0]
        d1, d2 = t[1] - t[0], t[2] - t[0]
        det = d1[0] * d2[1] - d1[1] * d2[0]
        if abs(det) < 1e-14:
            continue
        # The surface's derivatives along the texture's two axes.
        du = (e1 * d2[1] - e2 * d1[1]) / det
        dv = (e2 * d1[0] - e1 * d2[0]) / det
        a, b, c = du @ du, du @ dv, dv @ dv
        root = max(0.0, ((a - c) ** 2 + 4 * b * b)) ** 0.5
        big, small = ((a + c + root) / 2) ** 0.5, max(1e-12, (a + c - root) / 2) ** 0.5
        ratios.append(big / small)
        areas.append(poly.area)
        # Texels per metre: one over the root of the two stretches' product.
        scales.append(1.0 / (big * small) ** 0.5)

    def percentile(values, share):
        order = np.argsort(values)
        cum = np.cumsum(np.array(areas)[order]) / sum(areas)
        return float(np.array(values)[order][min(len(values) - 1, np.searchsorted(cum, share))])

    mid = percentile(scales, 0.5)
    return {"p90": round(percentile(ratios, 0.9), 2), "max": round(float(max(ratios)), 2),
            "density": [round(percentile(scales, 0.05) / mid, 2), round(percentile(scales, 0.95) / mid, 2)]}


def draw(body, path, size=1024):
    """Saves the layout as an image (island outlines), for review."""
    from PIL import Image, ImageDraw
    img = Image.new("RGB", (size, size), (18, 18, 20))
    d = ImageDraw.Draw(img)
    layer = body.data.uv_layers[0].data
    for poly in body.data.polygons:
        pts = [(layer[i].uv[0] * size, (1 - layer[i].uv[1]) * size) for i in poly.loop_indices]
        d.polygon(pts, outline=(150, 190, 220))
    img.save(path)
