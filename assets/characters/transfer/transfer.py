"""Step 3 of the MakeHuman transfer spike: carries MakeHuman's targets over
the correspondence onto the realistic topology and writes
`WORK/shapes.npz` (and, with `--pack`, the compact file the spike proposes
to vendor).

    python -I transfer.py --work WORK [--method full] [--pack FILE [--pack-extras [frame/name,...]]]

(`python` has the `bpy` module; run `register.py` first.)

For every realistic vertex: the nearest point of MakeHuman's wrapped
surface that faces the same way (a triangle and three weights). A target's
offset there is the weighted mean of its three corners' offsets, each
first carried from MakeHuman's own shape to the wrapped one by that
vertex's **local frame** (the rotation and the scale that best take its
neighbourhood from one to the other), so an offset that pushed MakeHuman's
belly forward by 5% of its depth pushes ours the same way and as far for
its size. Vertices inside the head (eye sockets, mouth) take the smoothest
field that meets the skin round them. Each shape is then made mirror
symmetric (the mean of itself and its mirror image), and turned from
MakeHuman's pose back to the bundle's pose by the realistic body's own
local frames between the two poses (`reframe`).

**Kept apart.** MakeHuman's artists left room at the armpits and between
the thighs for their own base; ours has less, so the heaviest shapes
would push the arm into the chest. `apart` finds the triangles that
newly cross at full weight (against the neutral's own crossings) and
scales the shape down around them, a little at a time and smoothly,
until none do.

`--method` picks how much of that is done, for the comparison in
`measure.py`:

    nearest   the nearest point of MakeHuman's mesh after the landmark
              warp only (no surface fit), offsets copied as they are
    fit       the surface fit, offsets copied as they are
    frames    the surface fit, offsets carried by local frames
    symmetric `frames`, then made symmetric
    full      `symmetric`, then kept apart (the default)

`shapes.npz` holds, per frame and shape, `posed/<frame>/<shape>` (in
MakeHuman's pose, on `posed_<frame>` of the registration) and
`bundle/<frame>/<shape>` (on the bundle's pose), float32 metres.
"""

import argparse
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import geom  # noqa: E402
import mh  # noqa: E402
import skeleton  # noqa: E402
from register import FRAMES, Surface, barycentric  # noqa: E402

METHODS = ("nearest", "fit", "frames", "symmetric", "full", "filtered")
# `filtered` (CO5, what the realistic package vendors): `symmetric`, with
# MakeHuman's offsets averaged over each realistic vertex's own patch of
# skin before they are sampled. MakeHuman's mesh is finer than ours on
# the trunk (17 mm edges, 5 mm at the navel, against our 25 mm), and a
# point sample of a finer field folds what it can't hold into something
# else: the navel became a crease across the belly, a pectoral's edge a
# hard line. The reach is this share of the vertex's mean edge length,
# so hands and faces, where ours is the finer mesh, are sampled as before.
# It isn't kept apart here: the build does that in the engine's rest pose.
FILTER = 0.6
REACH = 0.04
NEAR_SKIN = 8  # skin vertices an inside vertex follows


def shape_names(M, gender):
    """{our shape name: MakeHuman's dense offsets} for one gender."""
    out = {}
    for muscle in mh.MUSCLES:
        for weight in mh.WEIGHTS:
            if (muscle, weight) != ("averagemuscle", "averageweight"):
                out[f"{muscle}_{weight}"] = M[f"corner_{gender}_{muscle}_{weight}"].astype(np.float64)
    for key in M.files:
        if key.startswith("extra_"):
            out[key[len("extra_"):]] = M[key].astype(np.float64)
    return out


class Mapping:
    """Where each realistic vertex lies on MakeHuman's mesh `X` (wrapped
    or only warped), and how offsets are carried there."""

    def __init__(self, N, X, tris_m, edges_m, posed, tris_r, inside, lap_r, frames, filtered=False):
        surf = Surface(X, tris_m)
        normals = geom.vertex_normals(posed, tris_r)
        hit, tri, ok = surf.nearest(posed, normals, REACH)
        ok &= ~inside
        self.ok = ok
        self.tri = tris_m[np.where(ok, tri, 0)]
        self.bary = barycentric(hit, X[self.tri[:, 0]], X[self.tri[:, 1]], X[self.tri[:, 2]])
        self.gap = np.linalg.norm(hit - posed, axis=1)
        self.lap = lap_r
        # Inside the head (eye sockets, mouth) nothing of MakeHuman's is
        # matched: each vertex there follows the skin nearest to it.
        self.inside = np.flatnonzero(inside)
        skin = np.flatnonzero(ok)
        dist = np.linalg.norm(posed[self.inside][:, None] - posed[skin][None], axis=2)
        near = np.argsort(dist, axis=1)[:, :NEAR_SKIN]
        w = 1.0 / (np.take_along_axis(dist, near, axis=1) ** 2 + 1e-8)
        self.skin, self.skin_w = skin[near], w / w.sum(axis=1, keepdims=True)
        self.lost = ~ok & ~inside
        self.patch = None
        if filtered:
            self.patch = self._patches(X, tris_m, hit, posed, lap_r)
        if frames:
            R, s = geom.local_frames(N, X, edges_m, rings=2)
            # The scale varies smoothly over a body; its noise is the fit's.
            lap_m = geom.Laplacian(len(N), edges_m)
            s = s[:, None]
            for _ in range(10):
                s = lap_m.mean(s)
            self.R, self.s = R, s[:, 0]
        else:
            self.R = None

    def _patches(self, X, tris_m, hit, posed, lap_r):
        """Per matched realistic vertex: MakeHuman's vertices within its
        reach of where it lies on MakeHuman's surface, and their weights
        (a bell over the distance, times each vertex's area)."""
        from mathutils.kdtree import KDTree
        tree = KDTree(len(X))
        for j, p in enumerate(X):
            tree.insert(tuple(p), j)
        tree.balance()
        area = geom.vertex_areas(X, tris_m)
        e = lap_r.e
        length = np.linalg.norm(posed[e[:, 0]] - posed[e[:, 1]], axis=1)
        mean = (np.bincount(e[:, 0], length, len(posed)) + np.bincount(e[:, 1], length, len(posed)))             / np.maximum(lap_r.count, 1)
        reach = FILTER * mean
        out = {}
        for i in np.flatnonzero(self.ok):
            found = tree.find_range(tuple(hit[i]), 2.0 * reach[i])
            if len(found) < 4:
                continue
            idx = np.array([j for _, j, _ in found])
            dist = np.array([d for _, _, d in found])
            w = np.exp(-0.5 * (dist / reach[i]) ** 2) * area[idx]
            out[i] = (idx, w / w.sum())
        return out

    def _spread(self, out):
        """Fills the rows with no match: unmatched skin by the smoothest
        field that meets its neighbours, the inside of the head from the
        nearest skin (then smoothed a little, the skin held)."""
        if self.lost.any():
            out = geom.harmonic_fill(self.lap, out, ~self.lost)
        out[self.inside] = np.einsum("nk,nkj->nj", self.skin_w, out[self.skin])
        for _ in range(4):
            out[self.inside] = self.lap.mean(out)[self.inside]
        return out

    def carry(self, D):
        """MakeHuman's offsets `D` (per MakeHuman vertex) as offsets per
        realistic vertex."""
        if self.R is not None:
            D = np.einsum("nij,nj->ni", self.R, D) * self.s[:, None]
        out = sum(D[self.tri[:, k]] * self.bary[:, k:k + 1] for k in range(3))
        if self.patch:
            for i, (idx, w) in self.patch.items():
                out[i] = w @ D[idx]
        return self._spread(out)

    def sample(self, values):
        """A per-vertex value of MakeHuman's mesh sampled per realistic vertex."""
        v = values if values.ndim == 2 else values[:, None]
        out = self._spread(sum(v[self.tri[:, k]] * self.bary[:, k:k + 1] for k in range(3)))
        return out if values.ndim == 2 else out[:, 0]


def symmetric(d, mirror):
    """The mean of a shape and its mirror image."""
    return (d + d[mirror] * np.array([-1.0, 1.0, 1.0])) / 2


def crossing_pairs(V, tris):
    """The pairs of triangles that intersect and share no vertex."""
    import bpy  # noqa: F401 (the bpy module brings mathutils)
    from mathutils.bvhtree import BVHTree
    tree = BVHTree.FromPolygons([tuple(v) for v in V], [tuple(int(i) for i in t) for t in tris])
    return {(a, b) for a, b in tree.overlap(tree) if a < b and not set(tris[a]) & set(tris[b])}


def new_crossings(P, V, tris, known=None, touching=0.004):
    """The crossings of the body `V` that its neutral `P` doesn't have,
    leaving out pairs of triangles that already touch in the neutral
    (closed lips, eyelids, neighbouring toes: within `touching` metres),
    where any move makes or unmakes a crossing."""
    known = crossing_pairs(P, tris) if known is None else known
    out = set()
    for a, b in crossing_pairs(V, tris) - known:
        gap = np.linalg.norm(P[tris[a]][:, None] - P[tris[b]][None], axis=2).min()
        if gap > touching:
            out.add((a, b))
    return out


def apart(P, d, tris, lap, mirror, known=None, rounds=60, step=0.8):
    """The shape `d` on the body `P` scaled down, smoothly, round the
    places where it makes the body cross itself, until it adds no
    crossing (`new_crossings`). Returns the shape, the per-vertex factor
    and the number of crossings left."""
    known = crossing_pairs(P, tris) if known is None else known
    factor = np.ones(len(P))
    left = set()
    for _ in range(rounds):
        left = new_crossings(P, P + d * factor[:, None], tris, known)
        if not left:
            break
        hit = np.unique(tris[np.array(sorted({t for pair in left for t in pair}))])
        g = np.ones((len(P), 1))
        g[hit] = step
        # Spread the cut over the neighbours so it leaves no dent.
        for _ in range(3):
            g = np.minimum(g, (g + lap.mean(g)) / 2)
        g = np.minimum(g[:, 0], g[mirror, 0])
        factor *= g
    return d * factor[:, None], factor, len(left)


def reframe(d, P_from, P_to, edges):
    """Offsets `d` made on the pose `P_from` turned to suit the pose
    `P_to` of the same mesh: each by its vertex's local rotation between
    the two (and the local scale, if the mesh was also resized)."""
    R, s = geom.local_frames(P_from, P_to, edges, rings=2)
    return np.einsum("nij,nj->ni", R, d) * s[:, None]


def run(work, method="full", log=print):
    M = np.load(os.path.join(work, "makehuman.npz"))
    R = np.load(os.path.join(work, "realistic.npz"))
    G = np.load(os.path.join(work, "registration.npz"))
    faces_r = R["faces"]
    tris_r, edges_r = geom.triangles(faces_r), geom.edges(faces_r)
    tris_m, edges_m = geom.triangles(M["faces"]), geom.edges(M["faces"])
    sets = skeleton.Sets(faces_r, R["face_sets"])
    inside = np.zeros(len(R["masculine"]), dtype=bool)
    inside[sets.verts(*skeleton.FS_INSIDE)] = True
    lap_r = geom.Laplacian(len(inside), edges_r)
    # One topology: one pairing of left and right vertices for both frames.
    mirror = geom.mirror_map([R["masculine"], R["feminine"]])
    out, maps = {}, {}
    for frame, gender in FRAMES:
        N, posed, P = M[f"neutral_{gender}"], G[f"posed_{frame}"], R[frame]
        X = G[f"warped_{frame}"] if method == "nearest" else G[f"wrapped_{frame}"]
        m = Mapping(N, X, tris_m, edges_m, posed, tris_r, inside, lap_r, method not in ("nearest", "fit"),
                    filtered=method == "filtered")
        known = crossing_pairs(posed, tris_r) if method == "full" else None
        maps[frame] = m
        log(f"{frame} ({method}): {m.ok.sum()} of {len(posed)} vertices mapped "
            f"({inside.sum()} inside the head follow the skin, {m.lost.sum()} unmatched); "
            f"distance to MakeHuman's surface median {np.median(m.gap[m.ok]) * 1000:.2f} mm, "
            f"largest {m.gap[m.ok].max() * 1000:.1f} mm")
        for name, D in shape_names(M, gender).items():
            d = m.carry(D)
            if method in ("symmetric", "full", "filtered"):
                d = symmetric(d, mirror)
            if method == "full":
                d, factor, left = apart(posed, d, tris_r, lap_r, mirror, known)
                cut = factor < 0.999
                if cut.any():
                    log(f"  {name}: kept apart at {int(cut.sum())} vertices "
                        f"(scaled down to {factor.min():.2f} at most), {left} crossings left")
            out[f"posed/{frame}/{name}"] = d.astype(np.float32)
            out[f"bundle/{frame}/{name}"] = reframe(d, posed, P, edges_r).astype(np.float32)
    return out, maps


def pack(shapes, path, extras=False):
    """The compact form vendored as `realistic/source/body_shapes.npz`:
    the 16 corner shapes and the regional ones `extras` names, on the
    bundle's pose as 16-bit steps of 0.02 mm, only the vertices a shape
    moves by more than 0.05 mm, compressed. `extras` is True (every
    regional shape) or a list of `frame/name` (`masculine/m_thyroid`)."""
    step = 2e-5
    data = {"step": np.float64(step)}
    for key, d in shapes.items():
        if not key.startswith("bundle/"):
            continue
        name = key[len("bundle/"):].replace("/", "_")
        wanted = extras is True or (extras and key[len("bundle/"):] in extras)
        if "muscle" not in key and not wanted:
            continue
        moved = np.flatnonzero(np.linalg.norm(d, axis=1) > 5e-5)
        q = np.round(d[moved] / step)
        assert np.abs(q).max() < 32767
        data[name + "_index"] = moved.astype(np.uint16)
        data[name + "_offset"] = q.astype(np.int16)
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    np.savez_compressed(path, **data)
    return os.path.getsize(path)


def unpack(path, count=10582):
    """{shape: (count, 3) offsets in metres} from a packed file."""
    data = np.load(path)
    step = float(data["step"])
    out = {}
    for key in data.files:
        if key.endswith("_index"):
            name = key[:-len("_index")]
            d = np.zeros((count, 3), dtype=np.float32)
            d[data[key]] = data[name + "_offset"].astype(np.float32) * step
            out[name] = d
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--method", default="full", choices=METHODS)
    ap.add_argument("--pack")
    # Bare: every regional shape. With a value: the ones it lists, as
    # `frame/name` separated by commas (what the realistic build reads:
    # `realistic/shapes.py`'s `SOFT` and `FLAT`).
    ap.add_argument("--pack-extras", nargs="?", const="all")
    a = ap.parse_args()
    shapes, _ = run(a.work, a.method)
    name = "shapes.npz" if a.method == "full" else f"shapes_{a.method}.npz"
    np.savez_compressed(os.path.join(a.work, name), **shapes)
    print(f"{sum(k.startswith('posed/') for k in shapes)} shapes -> {name}")
    if a.pack:
        extras = a.pack_extras == "all" or (a.pack_extras.split(",") if a.pack_extras else False)
        size = pack(shapes, a.pack, extras)
        back = unpack(a.pack)
        worst = max(float(np.abs(back[k[len("bundle/"):].replace("/", "_")] - shapes[k]).max())
                    for k in shapes if k[len("bundle/"):].replace("/", "_") in back and k.startswith("bundle/"))
        print(f"packed: {a.pack}, {len(back)} shapes, {size / 1e6:.2f} MB; "
              f"largest difference from the unpacked shapes {worst * 1000:.3f} mm")


if __name__ == "__main__":
    main()
