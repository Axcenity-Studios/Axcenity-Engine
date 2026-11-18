"""Hair cards from a groom (Character Studio R6, CO11): each strand of a
layer becomes a strip of triangles along it (a card), lying in the hair's
layer and textured with a tile of the atlas from root to tip; and a cap,
the scalp itself lifted a little and textured with the atlas's dense
islands, so no skin shows between cards and the hairline thins out over
its ramp instead of ending at a card's edge.

A mesh here is plain arrays: positions, normals, texture coordinates
(glTF's: (0, 0) the atlas's top left) and triangles, plus the layer of
each vertex (for the build's report).
"""

import math

import numpy as np

from groom import seeded, unit, rotate
from scalp import smoothstep

# The cap stands this far off the scalp (metres), and its band (the
# hairline's ramp, drawn from the atlas's `band` island) runs this far in
# from the hairline.
CAP_LIFT = 0.0008
BAND = 0.014
# No card vertex comes nearer the scalp than this (metres; roots excepted).
CLEAR = 0.0012


class Mesh:
    def __init__(self):
        self.positions, self.normals, self.uvs, self.tris = [], [], [], []
        self.layers, self.roots = [], []

    def add(self, positions, normals, uvs, tris, layer, roots=None):
        """Adds a piece; `layer` is a name or one per vertex, `roots` marks
        the vertices that sit at the scalp (a card's first row)."""
        base = sum(len(p) for p in self.positions)
        count = len(positions)
        self.positions.append(np.asarray(positions, dtype=np.float64))
        self.normals.append(np.asarray(normals, dtype=np.float64))
        self.uvs.append(np.asarray(uvs, dtype=np.float64))
        self.tris.append(np.asarray(tris, dtype=np.int64).reshape(-1, 3) + base)
        self.layers += [layer] * count if isinstance(layer, str) else list(layer)
        self.roots += [False] * count if roots is None else list(roots)

    def arrays(self):
        return (np.concatenate(self.positions), np.concatenate(self.normals),
                np.concatenate(self.uvs), np.concatenate(self.tris), np.array(self.layers),
                np.array(self.roots))


def card(points, normals, sides, width, island, flip, twist, across=1, bulge=0.0, taper=0.35):
    """A strip along `points` (root first), `width` wide (narrowing by
    `taper` toward the tip), turned `twist` degrees about the strand from
    lying flat, mapped root to tip down `island`. Returns positions,
    normals (the scalp's, so neighbouring cards shade as one surface, not
    as planes) and texture coordinates per vertex, and triangles."""
    m = len(points)
    pos, nrm, uv = [], [], []
    for k in range(m):
        t = k / (m - 1)
        tangent = unit(points[min(k + 1, m - 1)] - points[max(k - 1, 0)])
        side = unit(sides[k] - tangent * (sides[k] @ tangent))
        lie = unit(np.cross(tangent, side))
        side = rotate(side, tangent, twist)
        half = 0.5 * width * (1.0 - taper * t)
        for j in range(across + 1):
            s = j / across
            x = (s - 0.5) * 2.0
            p = points[k] + side * half * x + lie * bulge * width * (1 - x * x)
            pos.append(p)
            # The scalp's normal: neighbouring cards shade as one surface
            # and highlights run over the head, not card by card.
            nrm.append(normals[k])
            u = (1 - s) if flip else s
            uv.append(island.uv(u, t))
    pos = np.array(pos)
    uv = np.stack([np.array([a[0] for a in uv]), np.array([a[1] for a in uv])], axis=1)
    roots = np.zeros(len(pos), dtype=bool)
    roots[: across + 1] = True
    tris = []
    w = across + 1
    for k in range(m - 1):
        for j in range(across):
            a, b = k * w + j, k * w + j + 1
            c, d = a + w, b + w
            tris += [(a, c, b), (b, c, d)]
    return pos, np.array(nrm), uv, np.array(tris), roots


# A root may sit this far inside the scalp (hair grows out of it), and
# is put back to `ROOT_DEPTH` when it is deeper.
ROOT_DEEPEST = 0.0007
ROOT_DEPTH = 0.0003


def keep_clear(groom, positions, roots):
    """Pushes every vertex but the roots at least `CLEAR` off the body's
    surface (along the nearest triangle's normal), and roots no deeper
    than `ROOT_DEEPEST` into it."""
    body = groom.body
    from mathutils import Vector
    out = positions.copy()
    for i, p in enumerate(positions):
        loc, _, index, _ = body.bvh.find_nearest(Vector(p))
        n = body.tri_normal[index]
        d = float((p - np.array(loc)) @ n)
        if roots[i]:
            if d < -ROOT_DEEPEST:
                out[i] = p + n * (-ROOT_DEPTH - d)
        elif d < CLEAR:
            out[i] = p + n * (CLEAR - d)
    return out


def layer_cards(groom, layer, islands, level):
    """The cards of one layer at `level` (0 hero, 1 gameplay, 2 crowd)."""
    style = groom.style
    name, spacing, width, share, kind, where = layer
    rng = seeded(style, name)
    keep = 1.0 if level == 0 else style["keep"].get(name, 1.0)
    if level == 2:
        if name != "coverage":
            return None
        spacing *= 1.9
        width *= 1.7
    if keep <= 0.0:
        return None
    roots, normals, density, f = groom.roots(spacing, where, rng)
    mesh = Mesh()
    pick = np.random.default_rng([style["seed"], level, len(roots)])
    tiles = islands[kind]
    for i in range(len(roots)):
        if level == 1 and pick.random() > keep:
            continue
        p, n = roots[i], normals[i]
        length = groom.length(p)
        height = groom.thickness(p, length)
        front = abs(float(groom.head.angles(p[None])[0][0])) < 45.0
        if name == "hairline":
            overhang = style["overhang_hairline"]
        elif front and name in ("top", "mid", "flyaway"):
            overhang = style["overhang_front"]
        else:
            # Staggered, so the tips don't end along one line.
            overhang = style["overhang"] - rng.uniform(0.0, 0.007)
        sub_kind = kind
        if name == "hairline":
            length = float(np.clip(0.65 * length, 0.010, 0.028))
            height = max(0.0015, height * share)
            if density[i] < 0.4:
                sub_kind = "sparse"
        elif name == "flyaway":
            length *= rng.uniform(1.05, 1.25)
            height *= share * rng.uniform(0.9, 1.3)
        else:
            height *= share * rng.uniform(0.85, 1.15)
            # Piecey: clumps of uneven length, so no edge of the style is
            # one line (the fringe, above all).
            if name in ("top", "mid"):
                length *= rng.uniform(0.72, 1.12)
        flick = style["flick"] if (name == "top" and rng.random() < style["flick"]) else 0.0
        segments = 3 if level == 2 else None
        pts, nrms, sides = groom.strand(p, n, length, height, rng, flick=flick, overhang=overhang,
                                        segments=segments,
                                        wave=style["wave"] * (2.5 if name == "flyaway" else 1.0))
        if len(pts) < 3:
            continue
        island = tiles if sub_kind == kind else islands[sub_kind]
        island = island[int(rng.integers(len(island)))]
        across = 2 if name in ("mid", "top") and level < 2 else 1
        bulge = 0.08 if across == 2 else 0.0
        twist = rng.normal(0.0, 6.0)
        w = width * rng.uniform(0.85, 1.15)
        pos, nrm, uv, tris, root = card(pts, nrms, sides, w, island, rng.random() < 0.5, twist,
                                        across, bulge)
        mesh.add(pos, nrm, uv, tris, name, root)
        # A clump of the top is two cards, the second turned up across the
        # first, so it has depth from the side.
        if name == "top" and level == 0:
            pos, nrm, uv, tris, root = card(pts + nrms * 0.0015, nrms, sides, w * 0.8,
                                            tiles[int(rng.integers(len(tiles)))], rng.random() < 0.5,
                                            twist + rng.choice([-1, 1]) * rng.uniform(18, 30), 1, 0.0)
            mesh.add(pos, nrm, uv, tris, name, root)
    return mesh


def cap(groom, islands, level):
    """The cap: the scalp's triangles inside the hairline, lifted
    `CAP_LIFT`, mapped round the head to the atlas's `band` island near
    the hairline (dense inside, single hairs at the line) and to its `cap`
    island elsewhere (strands running away from the crown)."""
    body = groom.body
    if level == 2:
        co, tris = groom.lod2_scalp()
    else:
        co, tris = body.co, groom.tris
    f = np.full(len(co), -1.0)
    used = np.unique(tris)
    f[used] = groom.hairline.signed(co[used])
    keep = tris[(f[tris] > -0.0005).all(1)]
    normals = body.normals if level != 2 else groom.lod2_normals()
    head = groom.head
    band, inner = islands["band"][0], islands["cap"][0]
    # Round the head (vertical axis): u for the band; round the crown's
    # axis: u for the cap; away from the crown: v for the cap.
    phi, _, _ = head.angles(co)
    u_band = (phi + 180.0) / 360.0
    axis = unit(groom.crown - head.centre)
    # The seam (u 0 and 1) runs down the back from the crown to the nape.
    ref = unit(np.cross(np.array([1.0, 0.0, 0.0]), axis))
    ref2 = np.cross(axis, ref)
    rel = co - head.centre
    flat = rel - np.outer(rel @ axis, axis)
    u_cap = (np.degrees(np.arctan2(flat @ ref2, flat @ ref)) + 180.0) / 360.0
    dist = np.linalg.norm(co - groom.crown, axis=1)
    far = dist[np.unique(keep)].max()
    v_cap = np.clip(dist / far, 0.0, 1.0)
    v_band = np.clip(1.0 - f / BAND, 0.0, 1.0)
    pos, nrm, uv, out = [], [], [], []
    for tri in keep:
        is_band = f[tri].min() < BAND
        if is_band:
            u = u_band[tri].copy()
            v = v_band[tri]
            island = band
        else:
            u = u_cap[tri].copy()
            v = v_cap[tri]
            island = inner
            if dist[tri].max() < 0.009:
                # Round the whorl every corner takes the face's own way
                # round, so the strands fan out from it.
                c = co[tri].mean(0) - head.centre
                c = c - axis * (c @ axis)
                u[:] = (math.degrees(math.atan2(c @ ref2, c @ ref)) + 180.0) / 360.0
        if u.max() - u.min() > 0.5:
            u = np.where(u < 0.5, u + 1.0, u)
        base = len(pos)
        for k, vi in enumerate(tri):
            pos.append(co[vi] + normals[vi] * CAP_LIFT)
            nrm.append(normals[vi])
            uv.append(island.uv(u[k], v[k]))
        out.append((base, base + 1, base + 2))
    pos = np.array(pos)
    uv = np.array([[a[0], a[1]] for a in uv])
    # Weld corners that share a place and texture coordinates.
    key = np.round(np.concatenate([pos * 1e6, uv * 1e6], axis=1)).astype(np.int64)
    _, first, inverse = np.unique(key, axis=0, return_index=True, return_inverse=True)
    inverse = inverse.reshape(-1)
    mesh = Mesh()
    mesh.add(pos[first], np.array(nrm)[first], uv[first], inverse[np.array(out)], "cap")
    return mesh


def build(groom, islands, level):
    """Every card and the cap at `level`, as one mesh, with each non-root
    vertex kept clear of the body."""
    mesh = Mesh()
    parts = [cap(groom, islands, level)]
    for layer in groom.style["layers"]:
        cards = layer_cards(groom, layer, islands, level)
        if cards is not None and cards.positions:
            parts.append(cards)
    for part in parts:
        pos, nrm, uv, tris, layers, roots = part.arrays()
        if layers[0] != "cap":
            pos = keep_clear(groom, pos, roots)
        mesh.add(pos, nrm, uv, tris, layers, roots)
    return mesh
