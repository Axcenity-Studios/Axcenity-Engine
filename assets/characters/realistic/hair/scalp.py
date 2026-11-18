"""The realistic head's scalp, read from the built body (`RealisticParts.glb`),
for the hair build (Character Studio R6, CO11): its surface and skinning,
the head's frame, the ears, the hairline and density map a style asks
for, and the binding that carries the body's shapes onto hair.

Everything is in Blender's space as the glTF importer gives it (Z up, the
face toward -Y), in metres, on the masculine frame at rest.
"""

import math

import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

# A shape the hair carries moves the scalp under some hair by more than
# this (metres).
SHAPE_MOVES = 0.0002


def smoothstep(e0, e1, x):
    t = np.clip((np.asarray(x, dtype=np.float64) - e0) / (e1 - e0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def interp_periodic(table, x):
    """Linear interpolation over (azimuth, value) pairs from 0 to 180
    degrees, mirrored (the head is even)."""
    xs = np.array([a for a, _ in table], dtype=np.float64)
    ys = np.array([v for _, v in table], dtype=np.float64)
    return np.interp(np.abs(x), xs, ys)


def smooth_table(table, x):
    """A smooth (cosine-eased) interpolation over (key, value) pairs."""
    xs = np.array([a for a, _ in table], dtype=np.float64)
    ys = np.array([v for _, v in table], dtype=np.float64)
    x = np.clip(np.asarray(x, dtype=np.float64), xs[0], xs[-1])
    i = np.clip(np.searchsorted(xs, x) - 1, 0, len(xs) - 2)
    t = (x - xs[i]) / (xs[i + 1] - xs[i])
    t = 0.5 - 0.5 * np.cos(np.pi * t)
    return ys[i] * (1 - t) + ys[i + 1] * t


class Body:
    """The body mesh at rest: positions, smooth normals, triangles, skin
    weights by bone, and every shape key as offsets."""

    def __init__(self, glb):
        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=glb, merge_vertices=False)
        obj = bpy.data.objects["Body"]
        me = obj.data
        n = len(me.vertices)
        co = np.zeros(n * 3)
        me.vertices.foreach_get("co", co)
        self.co = co.reshape(n, 3)
        me.calc_loop_triangles()
        tris = np.zeros(len(me.loop_triangles) * 3, dtype=np.int64)
        me.loop_triangles.foreach_get("vertices", tris)
        self.tris = tris.reshape(-1, 3)
        # The importer keeps glTF's split vertices (seams); weld them by
        # place for normals and neighbourhoods.
        keys = np.round(self.co * 1e5).astype(np.int64)
        _, inverse = np.unique(keys, axis=0, return_inverse=True)
        self.weld = inverse.reshape(-1)
        self.normals = self._normals(self.co)
        groups = {g.index: g.name for g in obj.vertex_groups}
        self.bones = [groups[i] for i in sorted(groups)]
        self.weights = np.zeros((n, len(self.bones)), dtype=np.float32)
        for v in me.vertices:
            for e in v.groups:
                self.weights[v.index, e.group] = e.weight
        sums = self.weights.sum(1, keepdims=True)
        self.weights /= np.maximum(sums, 1e-9)
        self.shapes = {}
        if me.shape_keys:
            for kb in me.shape_keys.key_blocks[1:]:
                d = np.zeros(n * 3)
                kb.data.foreach_get("co", d)
                self.shapes[kb.name] = d.reshape(n, 3) - self.co
        self.bvh = BVHTree.FromPolygons([Vector(p) for p in self.co], self.tris.tolist(), all_triangles=True)
        self.tri_normal = self._tri_normals(self.co)
        # Bones by name, at rest.
        arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
        self.bone_heads = {b.name: np.array(b.head_local) for b in arm.data.bones}
        self.objects = {o.name: o for o in bpy.data.objects}

    def bone(self, name):
        return self.bones.index(name)

    def _tri_normals(self, co):
        a, b, c = co[self.tris[:, 0]], co[self.tris[:, 1]], co[self.tris[:, 2]]
        n = np.cross(b - a, c - a)
        return n / np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)

    def _normals(self, co):
        a, b, c = co[self.tris[:, 0]], co[self.tris[:, 1]], co[self.tris[:, 2]]
        fn = np.cross(b - a, c - a)
        acc = np.zeros((self.weld.max() + 1, 3))
        for k in range(3):
            np.add.at(acc, self.weld[self.tris[:, k]], fn)
        acc /= np.maximum(np.linalg.norm(acc, axis=1, keepdims=True), 1e-12)
        return acc[self.weld]

    def shaped(self, name, weight=1.0):
        """Positions and normals with one shape at `weight`."""
        co = self.co + weight * self.shapes[name]
        return co, self._normals(co)

    def nearest(self, points):
        """The nearest surface point to each of `points`: (place, triangle,
        barycentric, normal there, signed distance along it)."""
        out_p, out_t, out_b, out_n, out_d = [], [], [], [], []
        for q in points:
            loc, _, index, _ = self.bvh.find_nearest(Vector(q))
            p = np.array(loc)
            tri = self.tris[index]
            bary = barycentric(p, self.co[tri[0]], self.co[tri[1]], self.co[tri[2]])
            nrm = bary @ self.normals[tri]
            nrm /= max(np.linalg.norm(nrm), 1e-12)
            out_p.append(p)
            out_t.append(index)
            out_b.append(bary)
            out_n.append(nrm)
            out_d.append(float(np.dot(np.asarray(q) - p, self.tri_normal[index])))
        return (np.array(out_p), np.array(out_t), np.array(out_b), np.array(out_n), np.array(out_d))


def barycentric(p, a, b, c):
    v0, v1, v2 = b - a, c - a, p - a
    d00, d01, d11 = v0 @ v0, v0 @ v1, v1 @ v1
    d20, d21 = v2 @ v0, v2 @ v1
    den = d00 * d11 - d01 * d01
    if abs(den) < 1e-20:
        return np.array([1.0, 0.0, 0.0])
    v = (d11 * d20 - d01 * d21) / den
    w = (d00 * d21 - d01 * d20) / den
    u = 1.0 - v - w
    bary = np.clip(np.array([u, v, w]), 0.0, 1.0)
    return bary / bary.sum()


class HeadFrame:
    """The head's centre (between the ears, a little above them) and the
    ears' vertices (those `Face_Ear_Size_*` moves)."""

    def __init__(self, body):
        ear = np.zeros(len(body.co), dtype=bool)
        for side in ("Left", "Right"):
            for name in ("Face_Ear_Size_" + side, "Face_Ear_Out_" + side):
                ear |= np.linalg.norm(body.shapes[name], axis=1) > 0.0004
        self.ear = ear
        left = body.co[ear & (body.co[:, 0] > 0)]
        right = body.co[ear & (body.co[:, 0] < 0)]
        self.ear_centres = (left.mean(0), right.mean(0))
        mid = 0.5 * (self.ear_centres[0] + self.ear_centres[1])
        self.centre = np.array([0.0, mid[1], mid[2] + 0.02])
        self.ear_points = body.co[ear]

    def angles(self, points):
        """Azimuth (0 at the front, +-180 at the back, positive to the
        character's left, +X) and elevation, in degrees, and the distance
        from the centre."""
        d = np.asarray(points) - self.centre
        r = np.linalg.norm(d, axis=-1)
        phi = np.degrees(np.arctan2(d[..., 0], -d[..., 1]))
        theta = np.degrees(np.arcsin(np.clip(d[..., 2] / np.maximum(r, 1e-9), -1, 1)))
        return phi, theta, r

    def direction(self, phi, theta):
        p, t = np.radians(phi), np.radians(theta)
        return np.stack([np.sin(p) * np.cos(t), -np.cos(p) * np.cos(t), np.sin(t)], axis=-1)


class Hairline:
    """A style's hairline as a signed distance on the scalp (metres; 0 on
    the line, positive inside the hair) and its density: 0 at the line,
    rising over the ramp (`ramp_front` at the front and temples,
    `ramp_side` elsewhere) to 1, held to `ear_clear` off each ear.

    The line is where an angular field changes sign (elevation above the
    style's hairline at each azimuth, or the distance to the ear less its
    clearance); the distance is to the nearest point of that line, found
    on the body's triangles, so it is a distance on the skin even where
    the line runs up and down (the temples, the sideburns)."""

    def __init__(self, style, head, body=None):
        self.style = style
        self.head = head
        self.line = None
        if body is not None:
            self.line = self._line(body)

    def field(self, points):
        """The angular field: its sign is inside or outside, its size only
        roughly a distance (metres)."""
        phi, theta, r = self.head.angles(points)
        line = interp_periodic(self.style["hairline"], phi)
        f = np.radians(theta - line) * r
        pts = np.asarray(points)
        d_ear = np.full(len(pts), np.inf)
        for k in range(0, len(self.head.ear_points), 256):
            chunk = self.head.ear_points[k:k + 256]
            d = np.linalg.norm(pts[:, None, :] - chunk[None, :, :], axis=2).min(1)
            d_ear = np.minimum(d_ear, d)
        return np.minimum(f, d_ear - self.style["ear_clear"])

    def _line(self, body):
        """Points along the line, about every millimetre: on each triangle
        of the head whose corners lie on both sides, the segment between
        its two crossings, sampled."""
        near = np.linalg.norm(body.co - self.head.centre, axis=1) < 0.17
        tris = body.tris[near[body.tris].all(1)]
        used = np.unique(tris)
        f = np.zeros(len(body.co))
        f[used] = self.field(body.co[used])
        out = []
        for tri in tris:
            v = f[tri]
            if (v > 0).all() or (v <= 0).all():
                continue
            cross = []
            for a, b in ((0, 1), (1, 2), (2, 0)):
                if (v[a] > 0) != (v[b] > 0):
                    t = v[a] / (v[a] - v[b])
                    cross.append(body.co[tri[a]] * (1 - t) + body.co[tri[b]] * t)
            if len(cross) == 2:
                steps = max(2, int(np.linalg.norm(cross[1] - cross[0]) / 0.001) + 1)
                for t in np.linspace(0, 1, steps):
                    out.append(cross[0] * (1 - t) + cross[1] * t)
        return np.array(out)

    def signed(self, points):
        pts = np.asarray(points, dtype=np.float64)
        f = self.field(pts)
        if self.line is None or not len(self.line):
            return f
        d = np.full(len(pts), np.inf)
        for k in range(0, len(self.line), 512):
            chunk = self.line[k:k + 512]
            d = np.minimum(d, np.linalg.norm(pts[:, None, :] - chunk[None, :, :], axis=2).min(1))
        return np.where(f > 0, d, -d)

    def ramp(self, points):
        phi, _, _ = self.head.angles(points)
        front = 1.0 - smoothstep(55.0, 80.0, np.abs(phi))
        return self.style["ramp_side"] + front * (self.style["ramp_front"] - self.style["ramp_side"])

    def density(self, points):
        f = self.signed(points)
        return smoothstep(0.0, self.ramp(points), f), f


def scalp_triangles(body, head, hairline, reach=0.004):
    """The triangles of the head and the nape whose corners are inside the
    hairline or within `reach` of it (hair at the line roots there)."""
    top = body.co[:, 2] > head.centre[2] - 0.16
    near = np.linalg.norm(body.co - head.centre, axis=1) < 0.16
    keep = top & near & ~head.ear
    candidate = np.where(keep[body.tris].all(1))[0]
    corners = np.unique(body.tris[candidate])
    f = np.full(len(body.co), -1.0)
    f[corners] = hairline.signed(body.co[corners])
    inside = (f[body.tris[candidate]] > -reach).all(1)
    # Leave the face out (in front of the ears and below the hairline
    # nothing grows): the signed distance does that, and the lips and
    # nose are far from any hairline.
    return candidate[inside], f


class Binding:
    """Carries the body's skinning and shapes onto points near the scalp.

    Each point is bound to the nearest point of the body's surface (a
    triangle and its barycentric weights) with its offset in that point's
    frame. Under a shape, the surface point moves with the triangle's
    corners and the offset turns with the surface's normal, so hair
    standing off the scalp keeps its height (fit by the head's own
    shapes, as the realistic underwear: `fit: shapes`)."""

    def __init__(self, body, points):
        self.body = body
        self.points = np.asarray(points, dtype=np.float64)
        place, tri, bary, normal, _ = body.nearest(self.points)
        self.tri, self.bary = tri, bary
        self.corners = body.tris[tri]
        self.place, self.normal = place, normal
        self.offset = self.points - place

    def weights(self, most=4):
        """Up to `most` bone weights per point, adding up to 1."""
        w = (self.body.weights[self.corners] * self.bary[:, :, None]).sum(1)
        order = np.argsort(-w, axis=1)[:, :most]
        top = np.take_along_axis(w, order, axis=1)
        top = np.where(top > 1e-4, top, 0.0)
        top /= np.maximum(top.sum(1, keepdims=True), 1e-9)
        return order, top

    def shape(self, name):
        """The points' offsets under the body's shape `name` at 1."""
        co, normals = self.body.shaped(name)
        place = (co[self.corners] * self.bary[:, :, None]).sum(1)
        n_new = (normals[self.corners] * self.bary[:, :, None]).sum(1)
        n_new /= np.maximum(np.linalg.norm(n_new, axis=1, keepdims=True), 1e-12)
        offset = rotate_between(self.normal, n_new, self.offset)
        return place + offset - self.points

    def shapes(self, threshold=SHAPE_MOVES):
        """Every body shape that moves some point by more than `threshold`,
        as {name: offsets}."""
        out = {}
        for name, delta in self.body.shapes.items():
            # Quick reject: no corner under these points moves.
            if np.abs(delta[np.unique(self.corners)]).max() < threshold * 0.5:
                continue
            moved = self.shape(name)
            if np.linalg.norm(moved, axis=1).max() > threshold:
                out[name] = moved
        return out


def rotate_between(a, b, v):
    """Turns each `v` by the shortest rotation taking unit `a` to unit `b`
    (Rodrigues)."""
    axis = np.cross(a, b)
    s = np.linalg.norm(axis, axis=1, keepdims=True)
    c = np.sum(a * b, axis=1, keepdims=True)
    k = axis / np.maximum(s, 1e-12)
    small = s[:, 0] < 1e-9
    out = v * c + np.cross(k, v) * s + k * np.sum(k * v, axis=1, keepdims=True) * (1 - c)
    out[small] = v[small]
    return out
