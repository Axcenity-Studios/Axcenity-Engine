"""Underwear for the realistic body, cut from the body's own surface.

Each garment is the body's skin between smooth hem lines (a scalar field
on the body, cut along its zero line through the triangles, so the hems
are curves and not the quads' stair steps), lifted off the skin along the
skin's normal by `GAP`, with a short rim folded down into the skin at
every hem so the cloth has an edge. A garment vertex is a fixed mix of
body vertices, so it takes their skin weights and every shape the body
has (the frame, muscle and fat, the correctives): the item follows the
body by shapes (`fit: shapes`, architecture.md section 9) under every
parameter, with nothing to fit at run time. Cloth under tension spans the
skin's hollows (`taut`); between the breasts and over the cleft of the
buttocks it is drawn straight across first (`bridge`).

Under a worn garment the body's skin sinks a little (the garment's mask
shape on the body, `Mask_<object>`, driven by the region the item hides),
so the body's simpler copies never show through the cloth.

Three garments, each its own object in `RealisticParts.glb`:

- `Underwear_BoxerBriefs`: from the hip bones to a third of the way down
  the thigh, with a waistband (Male's default).
- `Underwear_Briefs`: high-cut briefs (Female's default).
- `Underwear_Bra`: a sports bra with straps over the shoulders (Female's
  default), laid out on the feminine frame.

Blender only for making the objects; the geometry is NumPy. Units are
metres, Blender axes (Z up, the character faces -Y).
"""

import numpy as np

import shapes as body_shapes

# How far the cloth stands off the skin, and how far under the skin's
# resting surface its rim folds (onto the sunk skin, `RECESS`).
GAP = {"Underwear_BoxerBriefs": 0.0028, "Underwear_Briefs": 0.0024, "Underwear_Bra": 0.0030}
RIM_DEPTH = 0.004
# Half the gusset's width.
GUSSET = 0.028
# How many rounds of tension (`taut`) a garment's surface gets.
TAUT_ROUNDS = 40
# How far the skin under a worn garment sinks (its mask shape), and over
# how far past the hems it eases back up (`recess`).
RECESS = 0.005
RECESS_OUT = 0.015
# The waistband's and the bra's band's height.
BAND = {"Underwear_BoxerBriefs": 0.034, "Underwear_Briefs": 0.016, "Underwear_Bra": 0.026}
# (name, base colour (linear), roughness) of the two materials.
CLOTH = ("Underwear", (0.024, 0.024, 0.027), 0.88)
ELASTIC = ("Waistband", (0.006, 0.006, 0.007), 0.45)
# Bones whose weight keeps the cloth off a part of the body.
ARMS = ("UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little")
# Shapes a garment leaves out: they move nothing under it, or are the
# face's (driven live, moving nothing below the neck).
LEAST = 1e-5
# The push-out (CO9): how far the cloth is kept outside the skin at every
# shape's full weight on both frames, how far from the cloth a body
# vertex is looked at, and how a push fades to the neighbours (a share a
# ring, over this many rings).
MARGIN = 0.002
PUSH_REACH = 0.03
PUSH_FADE = 0.7
PUSH_RINGS = 4
PUSH_ROUNDS = 5


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


# ---------------------------------------------------------------- the body


class Body:
    """The body's rest positions on both frames, its triangles, normals,
    weights and shapes, read from the Blender object."""

    def __init__(self, obj):
        me = obj.data
        n = len(me.vertices)
        keys = me.shape_keys.key_blocks
        self.M = _co(keys["Basis"].data, n)
        self.F = _co(keys["Frame_Feminine"].data, n)
        self.shapes = {}
        for key in keys:
            if key.name == "Basis":
                continue
            self.shapes[key.name] = _co(key.data, n) - self.M
        me.calc_loop_triangles()
        self.tris = np.array([t.vertices[:] for t in me.loop_triangles], dtype=np.int64)
        self.groups = [g.name for g in obj.vertex_groups]
        W = np.zeros((n, len(self.groups)))
        for v in me.vertices:
            for g in v.groups:
                W[v.index, g.group] = g.weight
        self.W = W

    def weight(self, *parts):
        """Total weight of every group whose name contains one of `parts`."""
        cols = [k for k, name in enumerate(self.groups) if any(p in name for p in parts)]
        return self.W[:, cols].sum(axis=1) if cols else np.zeros(len(self.M))

    def normals(self, P):
        """Area-weighted vertex normals of positions `P`."""
        a, b, c = P[self.tris[:, 0]], P[self.tris[:, 1]], P[self.tris[:, 2]]
        fn = np.cross(b - a, c - a)
        N = np.zeros_like(P)
        for k in range(3):
            np.add.at(N, self.tris[:, k], fn)
        return N / np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)


def _co(data, n):
    out = np.zeros(n * 3)
    data.foreach_get("co", out)
    return out.reshape(n, 3)


# ------------------------------------------------- cutting along a field


class Cut:
    """A piece of the body's surface: its vertices as mixes of body
    vertices ({body vertex: share}) and its triangles."""

    def __init__(self, tris, mixes):
        self.tris = tris
        self.mixes = mixes

    def matrix(self, n):
        """The mixes as a (pieces' vertices x body vertices) array."""
        A = np.zeros((len(self.mixes), n))
        for row, mix in enumerate(self.mixes):
            for i, w in mix.items():
                A[row, i] += w
        return A


def split(tris, mixes, V, k):
    """Splits triangles `tris` (over vertices `mixes`, with field values
    `V`, a row per vertex) along the zero line of field `k`, which varies
    linearly along each edge. Returns (the triangles, the vertices, the
    values): the points on the line are added with every field's value
    mixed as they are, and shared between the two sides."""
    mixes = list(mixes)
    V = [row for row in V]
    # (A vertex on the line is moved just inside: no slivers of no area.)
    for row in V:
        if abs(row[k]) < 1e-6:
            row[k] = 1e-6
    points = {}

    def at(i, j):
        key = (min(i, j), max(i, j))
        if key not in points:
            t = V[i][k] / (V[i][k] - V[j][k])
            mix = {}
            for v, w in mixes[i].items():
                mix[v] = mix.get(v, 0.0) + w * (1.0 - t)
            for v, w in mixes[j].items():
                mix[v] = mix.get(v, 0.0) + w * t
            points[key] = len(mixes)
            mixes.append(mix)
            row = V[i] * (1.0 - t) + V[j] * t
            row[k] = 0.0
            V.append(row)
        return points[key]

    out = []
    for tri in tris:
        flags = [V[v][k] >= 0 for v in tri]
        if all(flags) or not any(flags):
            out.append(tuple(tri))
            continue
        # Turn the triangle so the odd one out comes first.
        lone = flags.index(True) if flags.count(True) == 1 else flags.index(False)
        a, b, c = tri[lone], tri[(lone + 1) % 3], tri[(lone + 2) % 3]
        ab, ac = at(a, b), at(a, c)
        out += [(a, ab, ac), (ab, b, c), (ab, c, ac)]
    return out, mixes, np.array(V)


def largest_piece(tris):
    """The triangles of the largest connected piece (by shared vertices)."""
    parent = {}

    def find(x):
        while parent.setdefault(x, x) != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for a, b, c in tris:
        ra, rb, rc = find(a), find(b), find(c)
        parent[rb] = ra
        parent[find(rc)] = ra
    count = {}
    for tri in tris:
        r = find(tri[0])
        count[r] = count.get(r, 0) + 1
    if not count:
        return []
    keep = max(count, key=count.get)
    return [tri for tri in tris if find(tri[0]) == keep]


def cut(body, fields, keep, band):
    """The body's surface where `keep` holds, as a `Cut`, the material of
    each of its triangles (1 where `band` >= 0) and the body vertices it
    covers. `fields` (a column per field, a row per body vertex) are
    simple, nearly linear fields; the surface is split along each one's
    zero line in turn, so a strip narrower than the body's triangles (a
    strap) is cut exactly, and `keep` then says, from the fields' values
    in the middle of each piece, which pieces are the garment."""
    n = len(body.M)
    mixes = [{i: 1.0} for i in range(n)]
    V = np.column_stack([fields, band]).astype(np.float64)
    tris = [tuple(t) for t in body.tris]
    # Only the triangles near the garment are cut.
    near = np.array([keep(V[list(t), :-1].max(axis=0)[None, :] + 0.05)[0] for t in tris])
    tris = [t for t, close in zip(tris, near) if close]
    for k in range(V.shape[1]):
        tris, mixes, V = split(tris, mixes, V, k)
    middles = np.array([V[list(t), :-1].mean(axis=0) for t in tris])
    kept = keep(middles)
    inside = largest_piece([t for t, yes in zip(tris, kept) if yes])
    band_k = V.shape[1] - 1
    banded = [t for t in inside if V[list(t), band_k].mean() >= 0]
    plain = [t for t in inside if V[list(t), band_k].mean() < 0]
    # Only the vertices used, renumbered.
    used = sorted({v for tri in banded + plain for v in tri})
    renumber = {old: new for new, old in enumerate(used)}
    tris = [tuple(renumber[v] for v in tri) for tri in plain + banded]
    material = [0] * len(plain) + [1] * len(banded)
    return Cut(tris, [mixes[v] for v in used]), material, [v for v in used if v < n]


# ------------------------------------------------------------ the garment


def boundary_edges(tris):
    """Edges used by one triangle, in that triangle's winding."""
    seen = {}
    for a, b, c in tris:
        for u, v in ((a, b), (b, c), (c, a)):
            key = (min(u, v), max(u, v))
            seen.setdefault(key, []).append((u, v))
    return [uses[0] for uses in seen.values() if len(uses) == 1]


def bridge(P, select, across, along, width):
    """`P` with the selected points drawn out to a straight line across a
    hollow: each takes the most outward (`across` points out of the body)
    of itself and the points near it in height whose distance from the
    middle (`along`) is `width`, as cloth over a cleft lies."""
    out = P.copy()
    rows = np.nonzero(select)[0]
    if len(rows) == 0:
        return out
    side = np.abs(P @ along)
    rim = np.nonzero(np.abs(side - width) < 0.012)[0]
    if len(rim) == 0:
        return out
    z = P[:, 2]
    for i in rows:
        near = rim[np.abs(z[rim] - z[i]) < 0.010]
        if len(near) == 0:
            continue
        reach = float(np.max(P[near] @ across))
        mine = float(P[i] @ across)
        if reach > mine:
            # Fades off toward the edge of the hollow.
            t = 1.0 - smoothstep(0.6 * width, width, float(side[i]))
            out[i] += across * (reach - mine) * t
    return out


def taut(P, N, tris, rounds=TAUT_ROUNDS):
    """`P` (a garment's points, `N` their normals) as cloth under tension
    lies: smoothed over its triangles, but each point only ever moved out
    along its normal, so it spans the skin's hollows (a strap across the
    groove by the shoulder blade, where skin beside it stood above it)
    and keeps to the skin where it is convex. The hems stay put."""
    P = P.copy()
    n = len(P)
    neighbours = [set() for _ in range(n)]
    for a, b, c in tris:
        neighbours[a].update((b, c))
        neighbours[b].update((a, c))
        neighbours[c].update((a, b))
    hem = {v for edge in boundary_edges(tris) for v in edge}
    rows = np.array([v for v in range(n) if v not in hem and neighbours[v]], dtype=np.int64)
    if len(rows) == 0:
        return P
    lists = [np.array(sorted(neighbours[v]), dtype=np.int64) for v in rows]
    for _ in range(rounds):
        mean = np.array([P[nb].mean(axis=0) for nb in lists])
        out = np.einsum("ij,ij->i", mean - P[rows], N[rows])
        P[rows] += N[rows] * (np.maximum(out, 0.0) * 0.5)[:, None]
    return P


def garment(body, name, fields, keep, band, bridges=(), parents=None):
    """The garment `name`: (positions on the masculine frame, triangles,
    material per triangle, {shape: offsets}, weights (vertices x groups)).
    `bridges` are (frame, select, across, along, width): hollows to draw
    the cloth across on that frame (`select` takes the piece's positions
    on that frame)."""
    piece, material, covered = cut(body, fields, keep, band)
    n = len(body.M)
    A = piece.matrix(n)
    frames = {}
    for frame, P in (("M", body.M), ("F", body.F)):
        normals = body.normals(P)
        N = A @ normals
        N /= np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)
        on = A @ P
        for which, select, across, along, width in bridges:
            if which == frame:
                on = bridge(on, select(on), across, along, width)
        on = taut(on, N, piece.tris)
        frames[frame] = (on, N)
    gap = GAP[name]
    # The rim: each hem vertex again, folded back to the skin.
    hem = boundary_edges(piece.tris)
    hem_verts = sorted({v for edge in hem for v in edge})
    inner = {v: len(piece.mixes) + k for k, v in enumerate(hem_verts)}
    source = list(range(len(piece.mixes))) + hem_verts
    lift = np.array([gap] * len(piece.mixes) + [-RIM_DEPTH] * len(hem_verts))
    tris = list(piece.tris)
    material = list(material)
    rim_material = {}
    for tri, m in zip(piece.tris, material):
        for u, v in ((tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])):
            rim_material[(u, v)] = m
    for u, v in hem:
        m = rim_material.get((u, v), 0)
        tris.append((v, u, inner[u]))
        tris.append((v, inner[u], inner[v]))
        material += [m, m]
    placed = {}
    for frame, (on, N) in frames.items():
        placed[frame] = on[source] + N[source] * lift[:, None]
    rest = placed["M"]
    shapes = {"Frame_Feminine": placed["F"] - rest}
    for shape, delta in body.shapes.items():
        if shape == "Frame_Feminine":
            continue
        moved = (A @ delta)[source]
        if float(np.abs(moved).max()) > LEAST:
            shapes[shape] = moved
    pushes = push_out(body, rest, piece.tris, len(piece.mixes), hem_verts, shapes)
    W, moved = limit((A @ body.W)[source], body.groups, parents or {})
    hem_at = A @ body.M
    mask = recess(body, covered, [(hem_at[u], hem_at[v]) for u, v in hem])
    report = {"triangles": len(tris), "shapes": len(shapes), "weight_moved_most": round(moved, 3),
              "shapes_pushed": pushes[0], "push_most": round(pushes[1], 4)}
    return rest, tris, material, shapes, W, mask, report


def push_out(body, rest, cloth_tris, count, hem_verts, shapes, margin=MARGIN):
    """Keeps the cloth `margin` outside the skin at each of its shapes' full
    weight, on both frames (the same push-out `avatar::fit` gives a
    garment it fits, CO9). A garment vertex moves with the body vertices it
    was cut from, but where the cloth bridges a hollow (the gusset under the
    crotch, the cleft of the buttocks) the skin beside it can swell past
    it: heavy and soft, the inner thighs come out under the gusset. Each
    shape that brings skin within `margin` of the cloth, or through it,
    takes the move out along the skin's normal (spread to the neighbours,
    and to each hem vertex's rim). Both ways round are looked at: cloth
    vertices inside the skin, and skin vertices under the cloth that come
    out between its vertices. `rest` and `shapes` are of the whole
    garment, the cloth's `count` vertices first and then the rim's (one per
    vertex of `hem_verts`). Returns (shapes pushed, the farthest push)."""
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree

    neighbours = [set() for _ in range(count)]
    for a, b, c in cloth_tris:
        for u, v in ((a, b), (b, c), (c, a)):
            neighbours[u].add(v)
            neighbours[v].add(u)
    cloth_tris = np.asarray(cloth_tris, dtype=np.int64)
    rim = {v: count + k for k, v in enumerate(hem_verts)}
    # The cloth away from its hems (two rings in): skin by a hem is beside
    # the cloth, not under it.
    near_hem = np.zeros(count, dtype=bool)
    near_hem[hem_verts] = True
    for _ in range(2):
        touching = near_hem[cloth_tris].any(axis=1)
        near_hem[cloth_tris[touching].ravel()] = True
    inner = cloth_tris[~near_hem[cloth_tris].any(axis=1)]

    def over(tree, p, normal):
        """The inner cloth's point over `p` (whose skin normal is
        `normal`): nearest, within reach, along the cloth's normal from it
        and facing the skin's way; or None."""
        loc, n, face, _ = tree.find_nearest(Vector(p), PUSH_REACH)
        if loc is None or np.dot(n, normal) <= 0.5:
            return None
        away = p - np.array(loc)
        along = float(np.dot(away, n))
        across = np.linalg.norm(away - np.array(n) * along)
        if across > 0.85 * abs(along) + 0.0005:
            return None
        return np.array(loc), np.array(n), face, along

    # The skin near the cloth (within 5 cm of its box at rest): which of it
    # the cloth covers is found in each state, as the body's shapes move
    # skin under it (a heavy thigh under a gusset).
    low, high = rest[:count].min(axis=0) - 0.05, rest[:count].max(axis=0) + 0.05
    near = np.nonzero(((body.M >= low) & (body.M <= high)).all(axis=1))[0]

    def spread(need):
        amount = need.copy()
        for _ in range(PUSH_RINGS):
            before = amount.copy()
            lengths = np.linalg.norm(before, axis=1)
            for v in range(count):
                for u in neighbours[v]:
                    if lengths[u] * PUSH_FADE > np.linalg.norm(amount[v]):
                        amount[v] = before[u] * PUSH_FADE
        return amount

    frame_body = body.shapes.get("Frame_Feminine", np.zeros_like(body.M))
    frame_cloth = shapes.get("Frame_Feminine", np.zeros_like(rest))
    own = [name for name in sorted(shapes)
           if name != "Frame_Feminine" and name in body.shapes and not name.startswith("Mask_")]
    # The states: each shape at its full weight, and the build's corners
    # (muscle and fat at either end: heavy and soft among them), on both
    # frames.
    states = [({name: 1.0}, feminine) for name in own for feminine in (0.0, 1.0)]
    for feminine in (0.0, 1.0):
        for muscle in (-1.0, 1.0):
            for fat in (-1.0, 1.0):
                mix = body_shapes.weights_of({"muscle": muscle, "fat": fat}, feminine)
                states.append(({k: w for k, w in mix.items() if k in shapes and abs(w) > 1e-6}, feminine))
    pushed, most = set(), 0.0
    for mix, feminine in states:
        if not mix:
            continue
        for _ in range(PUSH_ROUNDS):
            P = body.M + feminine * frame_body + sum(w * body.shapes[k] for k, w in mix.items())
            cloth = rest + feminine * frame_cloth + sum(w * shapes[k] for k, w in mix.items())
            skin = BVHTree.FromPolygons(P.tolist(), body.tris.tolist(), all_triangles=True)
            need = np.zeros((count, 3))
            for v in range(count):
                loc, normal, _, _ = skin.find_nearest(Vector(cloth[v]), PUSH_REACH)
                if loc is None:
                    continue
                out = float(np.dot(cloth[v] - np.array(loc), normal))
                if out < margin:
                    need[v] = np.array(normal) * (margin - out)
            tree = BVHTree.FromPolygons(cloth[:count].tolist(), inner.tolist(), all_triangles=True)
            normals = body.normals(P)
            for v in near:
                found = over(tree, P[v], normals[v])
                if found is None:
                    continue
                _, normal, face, out = found
                if out <= -margin:
                    continue
                push = normal * (out + margin)
                for c in inner[face]:
                    if np.linalg.norm(push) > np.linalg.norm(need[c]):
                        need[c] = push
            if not need.any():
                break
            most = max(most, float(np.linalg.norm(need, axis=1).max()))
            amount = spread(need)
            # Shared by the state's shapes, each by its weight's share, so
            # that together, at these weights, they move the cloth by it.
            total = sum(w * w for w in mix.values())
            for k, w in mix.items():
                share = amount * (w / total)
                shapes[k][:count] += share
                for v, r in rim.items():
                    shapes[k][r] += share[v]
                pushed.add(k)
    return len(pushed), most


def recess(body, covered, hem):
    """The garment's mask shape for the body (offsets per body vertex):
    the skin it covers sunk `RECESS` under its rest surface, and the skin
    round it easing back up over `RECESS_OUT` past the hems (`hem`: the
    hems as segments, on the masculine frame). Applied while the garment
    is worn, it keeps the body's simpler copies, whose surface is a few
    millimetres off the full one, and the skin at the hems in a bent pose
    (a narrow strap crosses the body's large triangles over the shoulder,
    whose far corners would otherwise stay up) from showing through the
    cloth. The skin dips into the cloth's edge, as under elastic."""
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    points = [a + (b - a) * k / 8.0 for a, b in hem for k in range(8)]
    tree = KDTree(len(points))
    for k, p in enumerate(points):
        tree.insert(Vector(p), k)
    tree.balance()
    normals = body.normals(body.M)
    inside = np.zeros(len(body.M), dtype=bool)
    inside[covered] = True
    out = np.zeros_like(body.M)
    for i in range(len(body.M)):
        _, _, far = tree.find(Vector(body.M[i]))
        if inside[i]:
            depth = 1.0
        elif far < RECESS_OUT:
            depth = 1.0 - float(smoothstep(0.0, RECESS_OUT, far))
        else:
            continue
        out[i] = -normals[i] * RECESS * depth
    return out


def limit(W, groups, parents, keep=4, floor=0.004):
    """At most `keep` influences per vertex, normalized. A point on a hem
    mixes two body vertices, so it can have up to eight; the engine skins
    with four. Each influence over four (the smallest first) goes to its
    nearest ancestor among the ones kept (a twist or helper bone's weight
    to the bone it turns with), so the point moves as nearly as it can as
    the skin under it. Returns (weights, the most weight any vertex had
    to move to another bone)."""
    W = W.copy()
    index = {name: k for k, name in enumerate(groups)}
    moved = 0.0
    for row in range(len(W)):
        while np.count_nonzero(W[row] > 0.0) > keep:
            on = np.nonzero(W[row] > 0.0)[0]
            least = on[np.argmin(W[row, on])]
            weight = W[row, least]
            W[row, least] = 0.0
            target = None
            name = parents.get(groups[least])
            while name is not None:
                k = index.get(name)
                if k is not None and W[row, k] > 0.0:
                    target = k
                    break
                name = parents.get(name)
            if target is None:
                target = on[np.argmax(W[row, on])]
            W[row, target] += weight
            moved = max(moved, float(weight))
    W[W < floor] = 0.0
    return W / np.maximum(W.sum(axis=1, keepdims=True), 1e-12), moved


# ----------------------------------------------------- the three fields


def fields(body, joints, joints_f):
    """{object name: (fields, keep, band, bridges)} for the three
    garments: the fields as columns (a row per body vertex), `keep` saying
    from their values which pieces are the garment (`cut`).
    `joints` and `joints_f` are the two frames' fitted joints (`rig`:
    name -> (head, tail, parent))."""
    M, F = body.M, body.F
    x, y, z = M[:, 0], M[:, 1], M[:, 2]
    head = lambda name, j=joints: np.array(j[name][0][:])  # noqa: E731
    hips, spine = head("Hips"), head("Spine")
    hip_l, knee_l = head("LeftUpperLeg"), head("LeftLowerLeg")
    hip_r, knee_r = head("RightUpperLeg"), head("RightLowerLeg")
    off_arms = 0.25 - body.weight(*ARMS)
    # Round each thigh: how far down it from the hip joint (a share of the
    # thigh), and which way from its axis (out to the side, to the front),
    # on the vertex's own side.
    left = x * np.sign(hip_l[0]) > 0
    t = np.zeros(len(M))
    outward = np.zeros(len(M))
    forward = np.zeros(len(M))
    for side, (hj, kn) in ((True, (hip_l, knee_l)), (False, (hip_r, knee_r))):
        rows = left == side
        d = kn - hj
        t[rows] = ((M[rows] - hj) @ d) / (d @ d)
        r = (M[rows] - hj) - np.outer(t[rows], d)
        r /= np.maximum(np.linalg.norm(r, axis=1, keepdims=True), 1e-9)
        outward[rows] = r[:, 0] * np.sign(hj[0])
        forward[rows] = -r[:, 1]
    # Front (-1) to back (+1) of the hips.
    back = 2.0 * smoothstep(hips[1] - 0.05, hips[1] + 0.05, y) - 1.0
    out = {}
    cleft = buttocks_cleft(hips, crotch_height(M))

    # The gusset: a strip along the middle under the crotch, so the cloth
    # spans it from front to back (the leg openings stop either side).
    # (Bounded by the thighs' weights, not a height: on this body the
    # thighs meet within the gusset's width of the middle.)
    # (And no further back than under the crotch: behind, it hung below
    # the hem between the buttocks.)
    gusset = [GUSSET - np.abs(x), 0.7 - body.weight("UpperLeg"), (hip_l[1] + 0.045) - y]

    def legs_or_gusset(v):
        """Inside the waist's and the arms' fields (columns 0 and 2) and
        either the leg openings' (1) or the gusset's (3 to 5)."""
        inside = v >= 0
        return inside[:, 0] & inside[:, 2] & (inside[:, 1] | (inside[:, 3] & inside[:, 4] & inside[:, 5]))

    # Boxer briefs: from just below the hip bones' crest to a third of the
    # way down the thigh, the waistband a little lower in front.
    top = spine[2] + 0.004 + 0.012 * back
    band = z - (top - BAND["Underwear_BoxerBriefs"])
    out["Underwear_BoxerBriefs"] = (np.column_stack([top - z, 0.31 - t, off_arms, *gusset]), legs_or_gusset,
                                    band, cleft)

    # Briefs: lower at the waist; the leg opening runs round the thigh from
    # the crotch, up the front to above the hip joint at the side, and back
    # down across the buttock, which it mostly covers.
    # (Linear in the direction round the thigh, so the hem is one smooth
    # curve: down to the crotch on the inside, above the hip joint at the
    # side, lower behind than in front.)
    # (The last term takes the inside of the back in, so the hem crosses
    # the buttocks instead of running down between them.)
    opening = 0.05 - 0.20 * outward - 0.05 * forward - 0.12 * outward * forward
    top = spine[2] - 0.022 + 0.010 * back
    band = z - (top - BAND["Underwear_Briefs"])
    out["Underwear_Briefs"] = (np.column_stack([top - z, opening - t, off_arms, *gusset]), legs_or_gusset,
                               band, cleft)

    # The sports bra, laid out on the feminine frame: under the bust to a
    # scooped neckline in front and across the shoulder blades behind,
    # cut away under each arm, with a strap over each shoulder.
    fx, fy, fz = F[:, 0], F[:, 1], F[:, 2]
    ax_ = np.abs(fx)
    chest_f = np.array(joints_f["Chest"][0][:])
    apex = bust_apex(F, body, chest_f)
    ax, ay, az = apex
    low = az - 0.088
    strap_x = 0.55 * abs(np.array(joints_f["LeftUpperArm"][0][:])[0])
    strap_w = 0.013
    front = 1.0 - smoothstep(chest_f[1] - 0.03, chest_f[1] + 0.03, fy)
    neckline = az + 0.040 + 0.060 * smoothstep(0.0, strap_x, ax_)
    behind = az + 0.050
    upper = front * neckline + (1.0 - front) * behind
    # The armhole: past the strap the top drops to the side seam.
    upper = upper - 1.2 * np.maximum(ax_ - (strap_x + strap_w), 0.0)
    upper = np.maximum(upper, az + 0.015)
    reach = np.array(joints_f["Head"][0][:])[2]
    columns = np.column_stack([fz - low, upper - fz, off_arms, ax_ - (strap_x - strap_w),
                               (strap_x + strap_w) - ax_, reach - fz])

    def bra(v):
        under, over, arms, inner, outer, high = (v[:, k] >= 0 for k in range(6))
        return under & ((over & arms) | (inner & outer & high))

    band = (low + BAND["Underwear_Bra"]) - fz
    across = np.array([0.0, -1.0, 0.0])
    along = np.array([1.0, 0.0, 0.0])
    cleavage = (("F", lambda P: (P[:, 1] < ay + 0.06) & (np.abs(P[:, 0]) < ax) & (P[:, 2] > low - 0.01),
                 across, along, ax),)
    out["Underwear_Bra"] = (columns, bra, band, cleavage)
    return out


def crotch_height(P):
    """The lowest point of the body's middle line between the legs."""
    mid = np.abs(P[:, 0]) < 0.006
    legs = P[:, 2] < 0.6 * P[:, 2].max()
    rows = np.nonzero(mid & legs & (P[:, 2] > 0.3 * P[:, 2].max()))[0]
    return float(P[rows, 2].min()) if len(rows) else 0.45 * P[:, 2].max()


def bust_apex(F, body, chest):
    """The breast's most forward point on the feminine frame (the left
    one: x, y, z, with x as a distance from the middle)."""
    torso = body.weight("Chest", "Spine") > 0.3
    rows = np.nonzero(torso & (F[:, 0] > 0.04) & (F[:, 0] < 0.16) & (F[:, 2] > chest[2] - 0.12)
                      & (F[:, 2] < chest[2] + 0.20))[0]
    i = rows[np.argmin(F[rows, 1])]
    return np.array([abs(F[i, 0]), F[i, 1], F[i, 2]])


def buttocks_cleft(hips, crotch):
    """Draw the cloth across the cleft between the buttocks, on both
    frames: behind the hips, within 4.5 cm of the middle, and not lower
    than 5 cm over the crotch (lower, the gusset was pulled back off it)."""
    across = np.array([0.0, 1.0, 0.0])
    along = np.array([1.0, 0.0, 0.0])
    width = 0.045
    select = lambda P: ((P[:, 1] > hips[1] + 0.02) & (np.abs(P[:, 0]) < width)  # noqa: E731
                        & (P[:, 2] < hips[2] + 0.06) & (P[:, 2] > crotch + 0.05))
    return (("M", select, across, along, width), ("F", select, across, along, width))


# ---------------------------------------------------------- Blender side


def _material(name, colour, roughness):
    import bpy
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*colour, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    if "Sheen Weight" in bsdf.inputs:
        bsdf.inputs["Sheen Weight"].default_value = 0.4
    return mat


def mask_name(name):
    """The body's mask shape for garment object `name`."""
    return "Mask_" + name


def make(body_obj, armature, joints, joints_f):
    """The three garments as Blender objects ({name: object}), not yet
    skinned to the armature (`build.skin_to`), and each one's mask shape
    added to the body (`mask_name`). Returns (the objects, a report for
    the build data)."""
    import bpy
    body = Body(body_obj)
    parents = {b.name: b.parent.name for b in armature.data.bones if b.parent}
    made = {}
    masks = {}
    reports = {}
    materials = [_material(*CLOTH), _material(*ELASTIC)]
    for name, (columns, keep, band, bridges) in fields(body, joints, joints_f).items():
        rest, tris, material, shapes, W, mask, reports[name] = garment(body, name, columns, keep, band, bridges,
                                                                       parents)
        masks[mask_name(name)] = mask
        me = bpy.data.meshes.new(name)
        me.from_pydata([tuple(p) for p in rest], [], [tuple(t) for t in tris])
        for mat in materials:
            me.materials.append(mat)
        me.polygons.foreach_set("material_index", material)
        me.polygons.foreach_set("use_smooth", [True] * len(tris))
        me.update()
        obj = bpy.data.objects.new(name, me)
        bpy.context.scene.collection.objects.link(obj)
        for k, group in enumerate(body.groups):
            rows = np.nonzero(W[:, k] > 0)[0]
            if len(rows) == 0:
                continue
            vg = obj.vertex_groups.new(name=group)
            for i in rows:
                vg.add([int(i)], float(W[i, k]), "REPLACE")
        obj.shape_key_add(name="Basis", from_mix=False)
        for shape, delta in shapes.items():
            key = obj.shape_key_add(name=shape, from_mix=False)
            key.data.foreach_set("co", (rest + delta).ravel())
            key.value = 0.0
        made[name] = obj
    for shape, offsets in masks.items():
        key = body_obj.shape_key_add(name=shape, from_mix=False)
        key.data.foreach_set("co", (body.M + offsets).ravel())
        key.value = 0.0
    return made, reports
