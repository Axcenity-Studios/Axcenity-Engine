"""Twice the vertices round the lids (Character Studio overhaul CO6).

The bundle's lids have a ring of 20 vertices at the edge of each eye's
opening, 8 of them along the upper lid. Seen close, the opening is a
polygon: most with the eye wide and in a laugh, whose arch was three or
four straight pieces. `split` cuts every quad of the strips that run out
from that ring in two, along the strip: across the first loops of the
socket's lining behind the lid's edge (`LINING_LOOPS`) and out over the
lid's skin past the fold (`SKIN_LOOPS`), so each of those loops has twice
the vertices. Where a strip stops, the quad beyond it takes the one new
vertex on its edge as a fan of three triangles: on the lining behind the
eyeball, and on the skin under the brow and on the cheek, not on the lid.

A new vertex does not sit at the middle of its edge (the polygon would be
the same polygon): it is put on the curve through the four vertices of
its loop round it (`Rims.grow`). That is a fixed mix of those four, the
same for both frames, every pose and every vendored shape, so whatever is
kept by the bundle's vertex index still fits: the bundle's vertices keep
their indices and their places, the new ones follow them.
"""

import bmesh
import numpy as np

from facetopo import FS_HEAD, FS_SOCKET, SIDES, Topo

# How far the strips are cut: the edges of the loops out to this one over
# the skin (the loops as `lids.Eye` counts them: 1 and 2 are the lid's
# rim, 5 and 6 the fold over the upper lid) and back to this one over the
# lining.
SKIN_LOOPS = 7
LINING_LOOPS = 5
# How a new vertex's place along its loop is measured between its
# neighbours: by their distances to this power (1 by the distance itself,
# 0.5 the centripetal measure, which does not overshoot where a loop turns
# a corner between a long edge and a short one).
SPACING = 0.5


class Rims:
    """What `split` made: `count` vertices before, the edge each new
    vertex was put on (`edges`, in the new vertices' order) and the mix of
    old vertices each is (`mix`: indices and shares, four per vertex)."""

    def __init__(self, count, edges, mix):
        self.count = count
        self.edges = list(edges)
        self.index = np.array([m[0] for m in mix], dtype=np.int64).reshape(-1, 4)
        self.share = np.array([m[1] for m in mix], dtype=np.float64).reshape(-1, 4)

    @property
    def total(self):
        return self.count + len(self.edges)

    def grow(self, X):
        """Per-vertex rows `X` of the bundle's vertices (positions or
        offsets) with the new vertices' rows after them."""
        X = np.asarray(X, dtype=np.float64)
        if len(X) == self.total:
            return X
        assert len(X) == self.count, f"{len(X)} rows for {self.count} vertices"
        new = (X[self.index] * self.share[:, :, None]).sum(axis=1)
        return np.concatenate([X, new])

    def grow_points(self, points):
        """`grow` for a list of `mathutils.Vector`s."""
        from mathutils import Vector
        return [Vector(p) for p in self.grow(np.array([tuple(p) for p in points]))]

    def mirror(self, base):
        """The mirror map of all the vertices from the bundle's (`base`:
        each vertex's mirror vertex): a new vertex's is the one on the
        mirror image of its edge, exactly."""
        base = np.asarray(base, dtype=np.int64)[:self.count]
        where = {frozenset(e): self.count + k for k, e in enumerate(self.edges)}
        new = [where[frozenset((int(base[a]), int(base[b])))] for a, b in self.edges]
        out = np.concatenate([base, np.array(new, dtype=np.int64)])
        assert np.array_equal(out[out], np.arange(len(out))), "the mirror map doesn't pair the new vertices up"
        return out


def _strips(topo, P):
    """The edges to cut, in order: for each eye, each edge of the ring
    where the lining meets the face and the edges across from it, quad by
    quad, out over the skin and back over the lining. A strip stops where
    its next edge is not one of a loop round the eye (its ends a different
    number of steps from the ring: the loops have run into the face's
    grid) or at the last loop."""
    edge_faces = {}
    for k, p in enumerate(topo.polys):
        for a, b in zip(p, p[1:] + p[:1]):
            edge_faces.setdefault(frozenset((a, b)), []).append(k)

    def across(face, edge):
        p = topo.polys[face]
        if len(p) != 4:
            return None
        k = next(k for k in range(4) if frozenset((p[k], p[(k + 1) % 4])) == edge)
        return frozenset((p[(k + 2) % 4], p[(k + 3) % 4]))

    out, seen = [], set()

    def take(edge):
        if edge not in seen:
            seen.add(edge)
            out.append(tuple(sorted(edge)))

    for _, s in SIDES:
        fs = FS_SOCKET[s]
        ring = topo.ring(fs, FS_HEAD)
        on_ring = set(ring)
        step = {}
        for k, loop in enumerate(topo.levels(ring, lambda i: topo.vsets[i] == {FS_HEAD} and P[i][0] * s > 0,
                                             most=SKIN_LOOPS + 2)):
            step.update((i, k) for i in loop)
        for k, loop in enumerate(topo.levels(ring, lambda i: topo.vsets[i] == {fs}, most=LINING_LOOPS + 2)):
            step.update((i, -k) for i in loop if k)
        first = sorted((e for e in edge_faces if e <= on_ring), key=lambda e: sorted(e))
        for start in first:
            take(start)
            for way, last in ((1, SKIN_LOOPS), (-1, LINING_LOOPS)):
                edge = start
                face = next(k for k in edge_faces[edge] if (topo.sets[k] == FS_HEAD) == (way > 0))
                for depth in range(1, last + 1):
                    edge = across(face, edge)
                    if edge is None or any(step.get(i) != way * depth for i in edge):
                        break
                    take(edge)
                    face = next((k for k in edge_faces[edge] if k != face), None)
                    if face is None:
                        break
    return out, edge_faces


def _mix(topo, P, edges, edge_faces):
    """For each edge (a, b) to cut, the new vertex as shares of four
    vertices: the cubic through the loop's vertices before `a`, `a`, `b`
    and after `b`, at the middle between `a` and `b` (the curve through
    three, or the edge's middle, where the loop does not go on)."""
    cut = {frozenset(e) for e in edges}

    def onward(a, b):
        """The vertex before `a` on the loop that runs `... a b`."""
        faces = set(edge_faces[frozenset((a, b))])
        free = [w for w in topo.nbr[a] if w != b and not faces & set(edge_faces[frozenset((a, w))])]
        if len(topo.nbr[a]) != 4:
            # (At a vertex where five edges meet, the loop goes on along
            # the one that is cut too.)
            free = [w for w in free if frozenset((a, w)) in cut]
        return free[0] if len(free) == 1 else None

    out = []
    for a, b in edges:
        before, after = onward(a, b), onward(b, a)
        if before is None and after is not None:
            a, b, before, after = b, a, after, None
        length = float(np.linalg.norm(P[b] - P[a])) ** SPACING
        knots, verts = [0.0, length], [a, b]
        if before is not None:
            knots.insert(0, -float(np.linalg.norm(P[a] - P[before])) ** SPACING)
            verts.insert(0, before)
        if after is not None:
            knots.append(length + float(np.linalg.norm(P[after] - P[b])) ** SPACING)
            verts.append(after)
        at = 0.5 * length
        shares = [float(np.prod([(at - knots[j]) / (knots[i] - knots[j]) for j in range(len(knots)) if j != i]))
                  for i in range(len(knots))]
        while len(verts) < 4:
            verts.append(a)
            shares.append(0.0)
        out.append((verts, shares))
    return out


def split(body, sets):
    """Cuts the strips round both eyes of `body` (a mesh object on the
    bundle's topology, with its texture coordinates and its face sets as
    materials; `sets` the face set of each polygon). The bundle's vertices
    keep their indices; the new ones come after them, placed by
    `Rims.grow`. Polygons are renumbered: whatever is kept by polygon has
    to be read again. Returns the `Rims`."""
    me = body.data
    count = len(me.vertices)
    P = np.empty(count * 3, dtype=np.float64)
    me.vertices.foreach_get("co", P)
    P = P.reshape(-1, 3)
    topo = Topo([tuple(p.vertices) for p in me.polygons], sets, count)
    edges, edge_faces = _strips(topo, P)
    rims = Rims(count, edges, _mix(topo, P, edges, edge_faces))
    placed = rims.grow(P)

    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    bm.faces.ensure_lookup_table()
    by_ends = {frozenset((e.verts[0].index, e.verts[1].index)): e for e in bm.edges}
    faces = list(bm.faces)
    new_of_face = {}
    for k, (a, b) in enumerate(edges):
        edge = by_ends[frozenset((a, b))]
        touched = [f.index for f in edge.link_faces]
        _, vert = bmesh.utils.edge_split(edge, edge.verts[0], 0.5)
        vert.co = placed[count + k]
        for f in touched:
            new_of_face.setdefault(f, []).append(vert)
    for f, verts in sorted(new_of_face.items()):
        face = faces[f]
        ring = [loop.vert for loop in face.loops]
        if len(verts) == 2 and len(ring) == 6 and (ring.index(verts[0]) - ring.index(verts[1])) % 6 == 3:
            # A quad of a strip: in two, along the strip.
            bmesh.utils.face_split(face, verts[0], verts[1])
        elif len(verts) == 2 and len(face.verts) == 6:
            # Two strips end in one quad, on two of its edges that meet
            # (where the loops round the eye run into the face's grid at
            # a vertex of five edges): the corner between the new
            # vertices, and a fan from the corner across from it.
            ring = [loop.vert for loop in face.loops]
            k = next(k for k in range(6) if ring[k] in verts and ring[(k + 2) % 6] in verts)
            first, second, far = ring[k], ring[(k + 2) % 6], ring[(k + 4) % 6]
            made, _ = bmesh.utils.face_split(face, first, second)
            rest = made if far in made.verts else face
            made, _ = bmesh.utils.face_split(rest, first, far)
            rest = made if second in made.verts else rest
            bmesh.utils.face_split(rest, second, far)
        elif len(verts) == 1 and len(face.verts) == 5:
            # The quad past a strip's end: a fan from the new vertex.
            ring = [loop.vert for loop in face.loops]
            k = ring.index(verts[0])
            far = (ring[(k + 2) % 5], ring[(k + 3) % 5])
            made, _ = bmesh.utils.face_split(face, verts[0], far[0])
            rest = made if far[1] in made.verts else face
            bmesh.utils.face_split(rest, verts[0], far[1])
        else:
            raise RuntimeError(f"a face round the eye is cut in a way this doesn't know: "
                               f"{len(face.verts)} corners, {len(verts)} of them new")
    bm.to_mesh(me)
    bm.free()
    me.update()
    assert len(me.vertices) == rims.total
    return rims
