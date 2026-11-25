"""Underwear for the human base, cut from the body's own surface: boxer
briefs (the CC0 and CC BY MakeHuman packs hold no men's underwear) and a
bandeau top to go with them on the feminine frame.

The garment is the body's skin between smooth hem lines: a number on each
body corner (`field`: above zero inside the garment) and the skin cut
along its zero line through the faces, so a hem is a curve and not the
quads' stair steps. Each garment corner is a fixed mix of two body
corners, so it takes their weights and every shape the body has: the
item follows the body by shapes (`fit: shapes`) with nothing fitted at
run time. The cloth stands `GAP` off the skin and is relaxed so it spans
the hollows (between the buttocks, at the groin) instead of following
them, each corner moving only out from the skin it lies over (along one
line, never sideways), and each hem has a short rim turned in under the
skin so the cloth has an edge.

NumPy, and Blender's `mathutils` for the nearest point on the skin.
Units are metres, Blender axes (Z up, the character faces -Y, left is +X).
"""

import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

#: How far the cloth stands off the skin, and how far its rim turns in.
GAP = 0.003
RIM = 0.005
#: How much farther off the skin the cloth stands at a hem, and how far in
#: from the hem that eases away.
HEM_LIFT = 0.002
HEM_BAND = 0.03
#: The waistband's height above the hip joints, and how far down the
#: thigh (as a share of hip to knee) the legs reach.
WAIST_ABOVE_HIPS = 0.085
LEG_SHARE = 0.36
#: Skin whose every corner is this far inside the hems (metres along the
#: field) is left out of the body when the garment is worn (its cover
#: map): all of it but the faces a hem crosses. The cloth is 3 mm off the
#: skin's corners, but a cloth face split along the other diagonal than
#: the skin's own dips to a tenth of a millimetre from it, so skin left
#: drawn under the cloth shows through in specks; the rim covers the edge.
SETBACK = 0.0005
RELAX_ROUNDS = 30
#: The farthest the cloth is let stand off its own skin where it spans a
#: hollow (between the buttocks, at the groin, under the bust).
SPAN = 0.03
#: The bones whose skin the garment may take (never a hand hanging by the hip).
BONES = ("Hips", "Spine", "LeftUpperLeg", "RightUpperLeg", "LeftUpperLegTwist", "RightUpperLegTwist")


def field(points, joints, allowed):
    """Above zero inside the garment, in metres from its nearest hem
    (`allowed`: [`on_bones`], the hem its bones make)."""
    hip_l, knee_l = (np.array(joints[n][0]) for n in ("LeftUpperLeg", "LeftLowerLeg"))
    hip_r, knee_r = (np.array(joints[n][0]) for n in ("RightUpperLeg", "RightLowerLeg"))
    waist = (hip_l[2] + hip_r[2]) / 2 + WAIST_ABOVE_HIPS - points[:, 2]

    def leg(hip, knee):
        axis = knee - hip
        length = np.linalg.norm(axis)
        along = (points - hip) @ (axis / length)
        return LEG_SHARE * length - along

    legs = np.where(points[:, 0] >= 0, leg(hip_l, knee_l), leg(hip_r, knee_r))
    return np.minimum(np.minimum(waist, legs), allowed)


def cut(faces, f):
    """The faces where `f` is above zero, cut along its zero line. Returns
    (garment faces, mixes): a garment corner is `(i, j, t)`, the point a
    share `t` of the way from body corner `i` to `j`."""
    index, mixes, out = {}, [], []

    def corner(i, j=None, t=0.0):
        key = (i, i) if j is None else (min(i, j), max(i, j))
        if key not in index:
            index[key] = len(mixes)
            if j is None:
                mixes.append((i, i, 0.0))
            else:
                a, b = key
                mixes.append((a, b, t if a == i else 1.0 - t))
        return index[key]

    for face in faces:
        values = [f[c] for c in face]
        if max(values) <= 0:
            continue
        if min(values) > 0:
            out.append([corner(c) for c in face])
            continue
        clipped = []
        for k, a in enumerate(face):
            b = face[(k + 1) % len(face)]
            fa, fb = f[a], f[b]
            if fa > 0:
                clipped.append(corner(a))
            if (fa > 0) != (fb > 0):
                clipped.append(corner(a, b, fa / (fa - fb)))
        if len(clipped) >= 3:
            out.append(clipped)
    return out, mixes


def normals_of(points, faces):
    out = np.zeros_like(points)
    for face in faces:
        p = points[face]
        n = np.zeros(3)
        for k in range(len(face)):
            n += np.cross(p[k], p[(k + 1) % len(face)])
        out[face] += n
    length = np.linalg.norm(out, axis=1, keepdims=True)
    return out / np.maximum(length, 1e-12)


def boundary_of(faces, count):
    """(is on an open edge, {corner: its two neighbours along the edge})."""
    uses = {}
    for face in faces:
        for k, a in enumerate(face):
            b = face[(k + 1) % len(face)]
            uses[(min(a, b), max(a, b))] = uses.get((min(a, b), max(a, b)), 0) + 1
    along = {}
    for (a, b), n in uses.items():
        if n == 1:
            along.setdefault(a, []).append(b)
            along.setdefault(b, []).append(a)
    on = np.zeros(count, dtype=bool)
    on[list(along)] = True
    return on, along, [edge for edge, n in uses.items() if n == 1]


#: How far (metres) inside a garment a corner wholly on its bones counts
#: as being, by its weights alone: what makes "mostly on these bones" a
#: number that crosses zero inside a face, like a hem's.
BONE_REACH = 0.1


def on_bones(weights, bones, count):
    """How much each body corner is on `bones`, as a number like a hem's:
    above zero where it is mostly on them (`BONE_REACH` where wholly),
    below where it is mostly on others. A garment's edge by its bones (the
    bandeau's under the arm) is then a line cut through the faces, not the
    faces' own stair steps."""
    total = np.zeros(count)
    for bone in bones:
        if bone in weights:
            idx, w = weights[bone]
            total[idx] += w
    return (total - 0.5) * 2 * BONE_REACH


def boxer_briefs(points, faces, weights, joints):
    """Boxer briefs on a body whose corners are `points` ([`cut_garment`])."""
    return cut_garment(points, faces, weights, field(points, joints, on_bones(weights, BONES, len(points))))


#: The bandeau: how far above and below the bust's most forward point it
#: reaches, and the bones whose skin it may take.
BAND_ABOVE = 0.06
BAND_BELOW = 0.085
BAND_BONES = ("Chest", "UpperChest", "Spine", "LeftShoulder", "RightShoulder")


def bandeau(points, faces, weights, joints):
    """A bandeau top round the chest of a body whose corners are `points`
    (laid out on the feminine frame, which it is for)."""
    allowed = on_bones(weights, BAND_BONES, len(points))
    chest = np.array(joints["Chest"][0])
    neck = np.array(joints["Neck"][0])
    front = (allowed > 0) & (points[:, 2] > chest[2]) & (points[:, 2] < neck[2] - 0.08) & (np.abs(points[:, 0]) < 0.12)
    apex = points[front][points[front][:, 1].argmin(), 2]
    f = np.minimum(apex + BAND_ABOVE - points[:, 2], points[:, 2] - (apex - BAND_BELOW))
    return cut_garment(points, faces, weights, np.minimum(f, allowed))


def cut_garment(points, faces, weights, f):
    """The garment where the number `f` on each body corner is above zero,
    on a body whose corners are `points` (the engine's pose), with faces
    `faces` and weights `weights` ({bone: (corners, weights)}). Returns a
    dict: `faces`, `points`, `mix` (i, j, t arrays) and `offset` (each
    corner's own stand-off from the mixed skin), `uvs` per face, `groups`
    and `hidden` (the body corners it covers inside its hems)."""
    count = len(points)
    garment_faces, mixes = cut(faces, f)
    i = np.array([m[0] for m in mixes])
    j = np.array([m[1] for m in mixes])
    t = np.array([m[2] for m in mixes])[:, None]
    skin = points[i] * (1 - t) + points[j] * t

    tree = BVHTree.FromPolygons([Vector(p) for p in points], [tuple(face) for face in faces])
    # A little more room at the hems, where the skin under the cloth is
    # still drawn (the faces a hem crosses).
    inside = f[i] * (1 - t[:, 0]) + f[j] * t[:, 0]
    ease = np.clip(inside / HEM_BAND, 0.0, 1.0)
    gap = GAP + HEM_LIFT * (1 - ease * ease * (3 - 2 * ease))
    # Out of the skin at each corner: the body's own normal there, which
    # is the same on both sides of a hem (the garment's own faces stop at
    # it, and their normal at a hem leans along the skin).
    body_normals = normals_of(points, faces)
    out_of_skin = body_normals[i] * (1 - t) + body_normals[j] * t
    out_of_skin /= np.maximum(np.linalg.norm(out_of_skin, axis=1, keepdims=True), 1e-12)
    on_edge, along, open_edges = boundary_of(garment_faces, len(skin))
    neighbours = [set() for _ in skin]
    for face in garment_faces:
        for k, a in enumerate(face):
            b = face[(k + 1) % len(face)]
            neighbours[a].add(b)
            neighbours[b].add(a)
    rings = [np.array(along[v] if on_edge[v] else sorted(neighbours[v]), dtype=np.int64) for v in range(len(skin))]
    # Each corner stays on the line out of its own skin and only its
    # height on it changes: a corner that left that line (even to keep
    # its distance from the nearest skin, which near a ridge is another
    # face's) walked sideways a little each round, up to 2 cm, and its
    # faces then lay across skin they weren't cut from, with the skin
    # through them at the leg hems and the hems themselves ragged.
    height = gap.copy()
    for _ in range(RELAX_ROUNDS):
        cloth = skin + out_of_skin * height[:, None]
        pull = np.array([
            0.5 * (cloth[ring].mean(axis=0) - cloth[v]) @ out_of_skin[v] if len(ring) else 0.0
            for v, ring in enumerate(rings)])
        # Out over a hollow as far as its neighbours span it, in over a
        # ridge no nearer its own skin than the gap.
        height = np.clip(height + pull, gap, gap + SPAN)
        # And no nearer any other skin than the gap (a crease's far wall).
        for v in range(len(skin)):
            for _ in range(8):
                at = skin[v] + out_of_skin[v] * height[v]
                near, normal, _, distance = tree.find_nearest(Vector(at))
                if near is None:
                    break
                outside = (at - np.array(near)) @ np.array(normal) >= 0
                if distance >= gap[v] - 1e-5 and outside:
                    break
                height[v] += 0.0005
    cloth = skin + out_of_skin * height[:, None]

    # The rim: each hem's corners again, turned in under the skin's own
    # surface along the same line.
    rim_of = {}
    extra_points, extra_mix = [], []
    for v in np.nonzero(on_edge)[0]:
        rim_of[int(v)] = len(cloth) + len(extra_points)
        extra_points.append(skin[v] - out_of_skin[v] * RIM)
        extra_mix.append(v)
    all_points = np.vstack([cloth, np.array(extra_points)]) if extra_points else cloth
    all_faces = [list(face) for face in garment_faces]
    orientation = {}
    for face in garment_faces:
        for k, a in enumerate(face):
            orientation[(a, face[(k + 1) % len(face)])] = True
    for a, b in open_edges:
        if (a, b) not in orientation:
            a, b = b, a
        # Wound against the cloth's own edge, so the rim faces outward.
        all_faces.append([b, a, rim_of[a], rim_of[b]])
    source = np.concatenate([np.arange(len(cloth)), np.array(extra_mix, dtype=np.int64)])
    mi, mj, mt = i[source], j[source], t[source]
    offset = all_points - (points[mi] * (1 - mt) + points[mj] * mt)

    dense = np.zeros((count, 0))
    bones = sorted(weights)
    dense = np.zeros((count, len(bones)))
    for k, bone in enumerate(bones):
        idx, w = weights[bone]
        dense[idx, k] = w
    mixed = dense[mi] * (1 - mt) + dense[mj] * mt
    if mixed.shape[1] > 4:
        keep = np.partition(mixed, -4, axis=1)[:, -4][:, None]
        mixed = np.where(mixed >= keep, mixed, 0.0)
    mixed /= np.maximum(mixed.sum(axis=1, keepdims=True), 1e-9)
    groups = {}
    for k, bone in enumerate(bones):
        idx = np.nonzero(mixed[:, k] > 0)[0]
        if len(idx):
            groups[bone] = (idx, mixed[idx, k])
    # Flat texture coordinates round the body: the cloth is one colour.
    angle = np.arctan2(all_points[:, 0], -all_points[:, 1]) / (2 * np.pi) + 0.5
    height = (all_points[:, 2] - all_points[:, 2].min()) / max(np.ptp(all_points[:, 2]), 1e-6)
    uvs = [[(float(angle[c]), float(height[c])) for c in face] for face in all_faces]
    return {"faces": all_faces, "points": all_points, "mix": (mi, mj, mt), "offset": offset, "uvs": uvs,
            "groups": groups, "hidden": f > SETBACK}


def shape_moves(garment, body_moves):
    """A body shape's moves, carried to the garment's corners."""
    mi, mj, mt = garment["mix"]
    return body_moves[mi] * (1 - mt) + body_moves[mj] * mt
