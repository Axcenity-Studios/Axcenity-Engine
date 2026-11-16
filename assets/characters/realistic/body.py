"""The base body: both frames loaded from `source/`, re-posed to the
skeleton contract's A-pose (arms 45 degrees below level, legs under the
hips), stood on the ground with the masculine frame 1.75 m tall, and each
frame's eye centres.

The bundle's bodies stand with the arms about 70 degrees below level and
the masculine one with its feet 40 cm apart. The re-pose turns the limbs
on a fitted skeleton with smooth weights, then takes out what linear
skinning does to the skin near the joints (a corrective smooth, which
keeps each vertex where it sat relative to its neighbours).
"""

import math

import bpy
from mathutils import Matrix, Vector

import rig
from common import (
    ARM_DOWN, FS_SOCKET, HEIGHT, SIDES, face_sets, fit_sphere, import_obj, positions, select_only,
    set_positions, v,
)

# How far the legs lean out from straight down, hip to ankle, at rest.
LEG_OUT = math.radians(2.0)
# The eyeball's centre sits this far in front of its socket's (the bundle's
# own placement, as a share of the socket's radius), and is this big.
EYE_FORWARD = 0.22
EYE_RADIUS = 0.892


class Frame:
    """One frame of the body: its positions and its eyes' centres and
    radius (found from the eye sockets, which are part of the mesh)."""

    def __init__(self, body_positions, lm):
        self.P = body_positions
        self.lm = lm

    def eyes(self):
        """{1: left centre, -1: right centre} and the eyeball's radius."""
        centres, radius = {}, 0.0
        for _, s in SIDES:
            centre, r = fit_sphere(self.P[i] for i in self.lm.verts(FS_SOCKET[s]))
            centres[s] = centre + v(0, -EYE_FORWARD * r, 0)
            radius += r * EYE_RADIUS / 2
        # One height, depth and distance from the middle for both eyes.
        x = (centres[1].x - centres[-1].x) / 2
        y = (centres[1].y + centres[-1].y) / 2
        z = (centres[1].z + centres[-1].z) / 2
        return {1: v(x, y, z), -1: v(-x, y, z)}, radius

    def eye_centres(self):
        return self.eyes()[0]


def repose(body, P, lm):
    """`P` with the arms turned to the 45 degree A-pose (the elbow's droop
    straightened) and the legs brought under the hips, the feet kept flat.
    `body` is left as it was."""
    stylized = rig.stylized_rig()
    eyes = Frame(P, lm).eye_centres()
    joints = rig.fit_joints(P, lm, eyes)
    arm = stylized.build_armature(joints, "Repose")
    work = body.copy()
    work.data = body.data.copy()
    bpy.context.scene.collection.objects.link(work)
    for vg in list(work.vertex_groups):
        work.vertex_groups.remove(vg)
    set_positions(work, P)
    stylized.auto_weights(work, arm)
    smooth = work.modifiers.new("Corrective", "CORRECTIVE_SMOOTH")
    smooth.rest_source = "ORCO"
    smooth.smooth_type = "LENGTH_WEIGHTED"
    smooth.iterations = 12
    smooth.factor = 0.6
    smooth.use_only_smooth = False
    select_only([arm])
    bpy.ops.object.mode_set(mode="POSE")

    def turn(bone, rotation):
        pb = arm.pose.bones[bone]
        bpy.context.view_layer.update()
        m = pb.matrix.copy()
        head = m.to_translation()
        pb.matrix = Matrix.Translation(head) @ rotation.to_4x4() @ Matrix.Translation(-head) @ m

    for side, s in SIDES:
        for bone in ("UpperArm", "LowerArm"):
            pb = arm.pose.bones[f"{side}{bone}"]
            bpy.context.view_layer.update()
            d = (pb.tail - pb.head).normalized()
            r = math.sqrt(max(0.0, 1.0 - d.y * d.y))
            target = Vector((s * math.cos(ARM_DOWN) * r, d.y, -math.sin(ARM_DOWN) * r)).normalized()
            turn(f"{side}{bone}", d.rotation_difference(target).to_matrix())
        # The leg as a whole, from the hip, in the frontal plane; the foot
        # turned back so its sole stays on the ground.
        hip, ankle = joints[f"{side}UpperLeg"][0], joints[f"{side}Foot"][0]
        out = math.atan2((ankle.x - hip.x) * s, hip.z - ankle.z)
        swing = Matrix.Rotation((out - LEG_OUT) * s, 3, "Y")
        turn(f"{side}UpperLeg", swing)
        turn(f"{side}Foot", swing.inverted())
    bpy.context.view_layer.update()
    bpy.ops.object.mode_set(mode="OBJECT")
    deps = bpy.context.evaluated_depsgraph_get()
    evaluated = work.evaluated_get(deps)
    out = [vert.co.copy() for vert in evaluated.data.vertices]
    bpy.data.objects.remove(work)
    bpy.data.objects.remove(arm)
    return out


def _stand(P, scale):
    """`P` scaled, with its lowest point on the ground."""
    lo = min(p.z for p in P)
    return [Vector((p.x * scale, p.y * scale, (p.z - lo) * scale)) for p in P]


def _symmetric(P, mirror):
    """`P` averaged with its mirror image, so both sides match exactly
    (the re-pose's solver leaves a fraction of a millimetre between them)."""
    return [(P[i] + Vector((-P[j].x, P[j].y, P[j].z))) / 2 for i, j in enumerate(mirror)]


def mirror_map(P):
    """Each vertex's mirror vertex across the body's middle (itself on the
    middle line), by position. None if the mesh isn't symmetric."""
    from mathutils.kdtree import KDTree
    tree = KDTree(len(P))
    for i, p in enumerate(P):
        tree.insert(p, i)
    tree.balance()
    out = []
    for p in P:
        _, j, d = tree.find(Vector((-p.x, p.y, p.z)))
        if d > 0.0015:
            return None
        out.append(j)
    if any(out[out[i]] != i for i in range(len(P))):
        return None
    return out


def unfold(mesh, P, sets, rounds=40):
    """`P` with the eye sockets' folds taken out: the bundle's feminine
    body has the skin lining each socket pass through itself at the lids'
    upper inner corner (it shows as a dark nick above the eye). The
    vertices of the faces that cross are moved toward the middle of their
    neighbours, a little at a time, until none do. Only vertices wholly
    inside a socket move: the lids' edge stays as sculpted. Returns the
    positions and how many pairs of faces crossed."""
    from mathutils.bvhtree import BVHTree

    from common import edge_neighbours
    socket = {fs for fs in FS_SOCKET.values()}
    inside = [True] * len(P)
    faces = []
    for poly in mesh.polygons:
        if sets[poly.index] in socket:
            faces.append(tuple(poly.vertices))
        else:
            for i in poly.vertices:
                inside[i] = False
    tris = [(f[0], f[k], f[k + 1]) for f in faces for k in range(1, len(f) - 1)]
    nbr = edge_neighbours(mesh)
    P = [p.copy() for p in P]
    found = None
    for _ in range(rounds):
        tree = BVHTree.FromPolygons(P, tris, all_triangles=True)
        crossing = [(i, j) for i, j in tree.overlap(tree) if i < j and not set(tris[i]) & set(tris[j])]
        if found is None:
            found = len(crossing)
        if not crossing:
            break
        move = {v for pair in crossing for t in pair for v in tris[t] if inside[v]}
        move |= {n for v in move for n in nbr[v] if inside[n]}
        new = {v: P[v].lerp(sum((P[n] for n in nbr[v]), Vector()) / len(nbr[v]), 0.5) for v in move}
        for v, at in new.items():
            P[v] = at
    return P, found or 0


# The fold under the chest: the sharpest inward bend this mesh's 25 mm
# quads can carry without stepping, in degrees between neighbouring faces.
FOLD = 24.0
# And the sharpest outward bend, for the ridge of the breast's underside
# just above the fold: sharper than this it shows as facets from below.
RIDGE = 26.0


def soften_folds(mesh, P, sets, mirror, rounds=60):
    """`P` with the fold under each side of the chest softened to what
    the mesh can hold. The bundle's feminine body has a sharp fold under
    the breast (up to 58 degrees between faces) that runs askew over quads
    25 mm wide, so it shows as stair-steps and a notch at its outer end.
    The vertices of the edges that fold inward by more than `FOLD`
    degrees there (or bend outward by more than `RIDGE`, on the ridge of
    the breast's underside above the fold), and their neighbours, are moved toward the middle of
    their own neighbours, a little at a time, until no edge does: the
    breast keeps its form and meets the ribs in a curve. Both sides are
    kept mirror images. Returns the positions, how many edges were over
    the limit and the sharpest of them (degrees)."""
    import bmesh

    from common import FS_TORSO, edge_neighbours
    nbr = edge_neighbours(mesh)
    P = [p.copy() for p in P]
    low = min(p.z for p in P)
    height = max(p.z for p in P) - low
    found, sharpest = None, 0.0
    for _ in range(rounds):
        bm = bmesh.new()
        bm.from_mesh(mesh)
        bm.verts.ensure_lookup_table()
        for v in bm.verts:
            v.co = P[v.index]
        bm.normal_update()
        over = set()
        worst = 0.0
        for e in bm.edges:
            if len(e.link_faces) != 2:
                continue
            f, g = e.link_faces
            if sets[f.index] != FS_TORSO or sets[g.index] != FS_TORSO:
                continue
            mid = (e.verts[0].co + e.verts[1].co) / 2
            down = (height - (mid.z - low)) / height
            # Under the chest, on the front and round to the side (the fold's
            # outer end), off the middle line and below the armpit.
            if not (0.265 < down < 0.355 and 0.03 < abs(mid.x) < 0.21 * height / 1.75 and mid.y < -0.005):
                continue
            inward = (g.calc_center_median() - f.calc_center_median()).dot(f.normal) > 0
            angle = math.degrees(f.normal.angle(g.normal, 0.0))
            if angle > (FOLD if inward else RIDGE):
                over.update(v.index for v in e.verts)
                worst = max(worst, angle)
        bm.free()
        if found is None:
            found, sharpest = len(over), worst
        if not over:
            break
        move = set(over)
        move |= {n for v in over for n in nbr[v]}
        if mirror is not None:
            move |= {mirror[v] for v in move}
        new = {v: P[v].lerp(sum((P[n] for n in nbr[v]), Vector()) / len(nbr[v]), 0.35) for v in move}
        for v, at in new.items():
            P[v] = at
        if mirror is not None:
            for v in move:
                m = P[mirror[v]]
                mean = (P[v] + Vector((-m.x, m.y, m.z))) / 2
                P[v] = mean
                P[mirror[v]] = Vector((-mean.x, mean.y, mean.z))
    return P, found or 0, sharpest


def load():
    """The masculine body object (faces, face sets and texture coordinates
    as vendored, in the bundle's pose), its landmarks, both frames re-posed
    and stood, and both frames' positions as vendored (for the bakes)."""
    masc = import_obj("body_masculine.obj")[0]
    masc.name = "Body"
    fem = import_obj("body_feminine.obj")[0]
    assert len(masc.data.vertices) == len(fem.data.vertices)
    stylized = rig.stylized_rig()
    lm = stylized.Landmarks(masc)
    source = {"Masculine": positions(masc), "Feminine": positions(fem)}
    sets = face_sets(masc)
    bpy.data.objects.remove(fem)
    mirror = mirror_map(source["Masculine"])
    frames = {}
    scale = None
    for name, P in source.items():
        # Twice: the second pass takes out what the first left, since the
        # joints sit a little differently once the limbs have moved.
        Q = P
        for _ in range(2):
            Q = repose(masc, Q, lm)
        if mirror is not None:
            Q = _symmetric(Q, mirror)
        if scale is None:
            scale = HEIGHT / (max(p.z for p in Q) - min(p.z for p in Q))
        Q, _ = unfold(masc.data, _stand(Q, scale), sets)
        # Only the feminine frame: the masculine body's sharpest fold there is
        # its pectoral's lower edge (36 degrees), which holds its line.
        folds = 0
        if name == "Feminine":
            Q, folds, sharpest = soften_folds(masc.data, Q, sets, mirror)
        if folds:
            print(f"  {name}: {folds} vertices on folds under the chest sharper than {FOLD:g} degrees "
                  f"(the sharpest {sharpest:.0f}), softened")
        frames[name] = Frame(Q, lm)
    return masc, lm, frames, source
