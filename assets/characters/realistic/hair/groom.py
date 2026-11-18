"""A groom as data (Character Studio R6, CO11): roots grown from the
realistic scalp by a style's density map, combed, and each grown into a
strand (a polyline over the scalp) by the style's length, lift, wave and
clumping. `cards.py` turns the strands into cards; `atlas.py` draws the
strands the cards show.

Deterministic: every random draw comes from a generator seeded by the
style's seed and the layer's name.
"""

import math
import zlib

import numpy as np
from mathutils import Vector

from scalp import HeadFrame, Hairline, scalp_triangles, smoothstep, smooth_table


def seeded(style, name):
    return np.random.default_rng([style["seed"], zlib.crc32(name.encode())])


def unit(v):
    v = np.asarray(v, dtype=np.float64)
    return v / max(np.linalg.norm(v), 1e-12)


def rotate(v, axis, degrees):
    a = math.radians(degrees)
    axis = unit(axis)
    return v * math.cos(a) + np.cross(axis, v) * math.sin(a) + axis * (axis @ v) * (1 - math.cos(a))


class Groom:
    """A style on the body: the scalp, its hairline and density, the comb
    and the length over the head."""

    def __init__(self, body, style):
        self.body = body
        self.style = style
        self.head = HeadFrame(body)
        self.hairline = Hairline(style, self.head, body)
        self.tris = body.tris[scalp_triangles(body, self.head, self.hairline)[0]]
        a, b, c = (body.co[self.tris][:, k] for k in range(3))
        self.areas = 0.5 * np.linalg.norm(np.cross(b - a, c - a), axis=1)
        # The crown's whorl on the scalp: a ray from the head's centre.
        direction = self.head.direction(*style["crown"])
        hit, _, _, _ = body.bvh.ray_cast(Vector(self.head.centre), Vector(direction))
        self.crown = np.array(hit)

    # -- fields over the scalp ----------------------------------------------

    def length(self, p):
        """The hair's length at scalp point `p`."""
        s = self.style
        phi, theta, _ = self.head.angles(p[None])
        phi, theta = abs(float(phi[0])), float(theta[0])
        front = smooth_table(s["length_front"], theta)
        side = smooth_table(s["length_side"], theta)
        back = smooth_table(s["length_back"], theta)
        if phi <= 90:
            t = smoothstep(20.0, 90.0, phi)
            return float(front * (1 - t) + side * t)
        t = smoothstep(90.0, 160.0, phi)
        return float(side * (1 - t) + back * t)

    def comb(self, p, n):
        """The direction hair at `p` (normal `n`) is combed: away from the
        crown, down at the sides and back (and a little back over the ears),
        swept toward the character's left at the front."""
        s = self.style
        phi, theta, _ = self.head.angles(p[None])
        phi, theta = float(phi[0]), float(theta[0])
        away = p - self.crown
        away = away - n * (away @ n)
        if np.linalg.norm(away) < 1e-6:
            away = np.array([0.0, -1.0, 0.0]) - n * (-n[1])
        away = unit(away)
        side = smoothstep(40.0, 70.0, abs(phi)) * (1.0 - smoothstep(130.0, 160.0, abs(phi)))
        down = np.array([0.0, 0.35 * side, -1.0])
        down = unit(down - n * (down @ n))
        g = 1.0 - smoothstep(s["gravity_full"], s["gravity_from"], theta)
        d = unit((1.0 - g) * away + g * down)
        front = (1.0 - smoothstep(25.0, 80.0, abs(phi))) * smoothstep(15.0, 45.0, theta)
        if front > 0:
            d = rotate(d, n, s["sweep"] * front)
        return unit(d - n * (d @ n))

    def thickness(self, p, length):
        """How far the hair layer stands off the scalp at `p`."""
        s = self.style
        _, theta, _ = self.head.angles(p[None])
        top = smoothstep(0.0, 40.0, float(theta[0]))
        lift = math.radians(s["lift_side"] + top * (s["lift_top"] - s["lift_side"]))
        return float(np.clip(length * math.sin(lift) * 0.62, s["thickness_min"], s["thickness_max"]))

    # -- roots ----------------------------------------------------------------

    def surface_points(self, count, rng):
        """`count` points spread over the scalp by area: places, triangles
        and normals."""
        p = self.areas / self.areas.sum()
        pick = rng.choice(len(self.tris), size=count, p=p)
        r1, r2 = rng.random(count), rng.random(count)
        s1 = np.sqrt(r1)
        bary = np.stack([1 - s1, s1 * (1 - r2), s1 * r2], axis=1)
        corners = self.tris[pick]
        places = (self.body.co[corners] * bary[:, :, None]).sum(1)
        normals = (self.body.normals[corners] * bary[:, :, None]).sum(1)
        normals /= np.linalg.norm(normals, axis=1, keepdims=True)
        return places, normals

    def roots(self, spacing, where, rng):
        """Roots at least `spacing` apart (dart throwing), accepted by the
        density map and the layer's region: `all` (inside the ramp),
        `top`, `sides` or `hairline` (the ramp itself)."""
        area = self.areas.sum()
        candidates = int(8 * area / spacing ** 2)
        places, normals = self.surface_points(candidates, rng)
        density, f = self.hairline.density(places)
        ramp = self.hairline.ramp(places)
        _, theta, _ = self.head.angles(places)
        u = rng.random(candidates)
        if where == "hairline":
            ok = (f > -0.0005) & (f < ramp + 0.003) & (u < 0.08 + 0.92 * density ** 1.5)
        elif where == "top":
            ok = (u < density ** 2) & (theta > 12.0) & (f > ramp * 0.5)
        elif where == "sides":
            ok = (u < density ** 2) & (theta < 32.0) & (f > ramp * 0.3)
        else:
            ok = (u < density ** 2) & (f > ramp * 0.9)
        keep, cells = [], {}
        for i in np.where(ok)[0]:
            key = tuple(np.floor(places[i] / spacing).astype(int))
            close = False
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    for dz in (-1, 0, 1):
                        for j in cells.get((key[0] + dx, key[1] + dy, key[2] + dz), ()):
                            if np.linalg.norm(places[i] - places[j]) < spacing:
                                close = True
                                break
                        if close:
                            break
                    if close:
                        break
                if close:
                    break
            if not close:
                cells.setdefault(key, []).append(i)
                keep.append(i)
        keep = np.array(keep, dtype=np.int64)
        return places[keep], normals[keep], density[keep], f[keep]

    # -- strands --------------------------------------------------------------

    def project(self, q):
        loc, nrm, index, _ = self.body.bvh.find_nearest(Vector(q))
        tri = self.body.tris[index]
        from scalp import barycentric
        b = barycentric(np.array(loc), *self.body.co[tri])
        n = unit(b @ self.body.normals[tri])
        return np.array(loc), n

    def strand(self, root, normal, length, height, rng, flick=0.0, wave=None, overhang=0.002,
               segments=None, jitter=10.0):
        """A strand from `root`: walks over the scalp along the comb for
        `length`, standing `height` off it (rising from the root, settling
        toward the tip, or flicking up by `flick`), waving across. Returns
        the points (root first), the scalp normal under each and the
        direction across. Stops near an ear or past the hairline by more
        than `overhang`."""
        s = self.style
        k_count = segments or int(np.clip(round(length / 0.0065), 4, 9))
        step = length / k_count
        d = rotate(self.comb(root, normal), normal, rng.normal(0.0, jitter))
        amp = (s["wave"] if wave is None else wave) * rng.uniform(0.5, 1.3)
        phase = rng.uniform(0, 2 * math.pi)
        rise = rng.uniform(0.22, 0.38)
        points = [root - normal * 0.0004]
        normals = [normal]
        sides = [unit(np.cross(normal, d))]
        place, n = root, normal
        for k in range(1, k_count + 1):
            t = k / k_count
            q = place + d * step
            place_new, n_new = self.project(q)
            near_ear = np.linalg.norm(self.head.ear_points - place_new, axis=1).min() < 0.006
            f = self.hairline.signed(place_new[None])[0]
            if (near_ear or f < -overhang) and k > 2:
                break
            transported = unit(d - n_new * (d @ n_new))
            d = unit(0.65 * transported + 0.35 * self.comb(place_new, n_new))
            place, n = place_new, n_new
            h = height * math.sin(0.5 * math.pi * min(1.0, t / rise))
            if t > 0.55:
                fall = (t - 0.55) / 0.45
                h *= 1.0 - 0.35 * fall + flick * 1.1 * fall * fall
            side = unit(np.cross(n, d))
            w = amp * math.sin(2 * math.pi * s["waves"] * t + phase) * smoothstep(0.0, 0.3, t)
            points.append(place + n * h + side * w)
            normals.append(n)
            sides.append(side)
        return np.array(points), np.array(normals), np.array(sides)

    # -- the crowd copy's scalp -----------------------------------------------

    def lod2_scalp(self):
        """`Body_LOD2`'s triangles round the head, each corner put on the
        full body's surface and lifted `LOD2_LIFT` more (its flat triangles
        cut under the curve of the skull by up to about 1.3 mm), so the
        crowd copy's cap stays over the body at any of its levels."""
        if not hasattr(self, "_lod2"):
            me = self.body.objects["Body_LOD2"].data
            n = len(me.vertices)
            co = np.zeros(n * 3)
            me.vertices.foreach_get("co", co)
            co = co.reshape(n, 3)
            me.calc_loop_triangles()
            tris = np.zeros(len(me.loop_triangles) * 3, dtype=np.int64)
            me.loop_triangles.foreach_get("vertices", tris)
            tris = tris.reshape(-1, 3)
            near = np.linalg.norm(co - self.head.centre, axis=1) < 0.17
            tris = tris[near[tris].all(1)]
            place, _, _, normal, _ = self.body.nearest(co)
            self._lod2 = (place + normal * LOD2_LIFT, tris, normal)
        return self._lod2[0], self._lod2[1]

    def lod2_normals(self):
        self.lod2_scalp()
        return self._lod2[2]


LOD2_LIFT = 0.0016
