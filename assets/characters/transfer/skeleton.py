"""Landmarks of both bodies under one set of names, and the re-pose of the
realistic body into MakeHuman's pose.

The realistic body's landmarks come from its sculpt face sets (the rings
where two sets meet); MakeHuman's from the joint cubes its base mesh
carries. Names: `crown`, `crotch`, `l-eye`, and per side (`l-`, `r-`)
`shoulder`, `elbow`, `wrist`, `hip`, `knee`, `ankle`, `ball`, `toe`, and
`f<1..5>-<1..4>` (thumb to little finger; knuckle, two joints, tip).

Used by `register.py`; not run on its own.
"""

import math

import numpy as np

import geom

# Sculpt face sets of the realistic bodies. The left side is +X.
FS_TORSO = 1
FS_SOCKET = {1: 2, -1: 3}
FS_EAR = {1: 4, -1: 5}
FS_MOUTH = (7, 8)
FS_PALM = {1: 9, -1: 10}
FS_FOREARM = {1: 12, -1: 11}
FS_FOOT = {1: 14, -1: 13}
FS_SHIN = {1: 15, -1: 16}
FS_HEAD = 17
FS_PELVIS = 18
FS_WAIST = 19
FS_UPPER_ARM = {1: 21, -1: 20}
FS_MUZZLE = 22
FS_THIGH = {1: 24, -1: 23}
FS_TOE_BASE = {1: 25, -1: 26}
FS_TOES = {1: tuple(range(46, 64)), -1: tuple(range(27, 46))}
# Fingers, thumb first: (first, second, third, nail) sets.
FS_FINGERS = {
    1: ((80, 81, 82, 83), (76, 77, 78, 79), (72, 73, 74, 75), (68, 69, 70, 71), (64, 65, 66, 67)),
    -1: ((84, 85, 86, 87), (88, 89, 90, 91), (92, 93, 94, 95), (96, 97, 98, 99), (100, 101, 102, 103)),
}
# Inside the head: the eye sockets and the mouth. Not skin; never matched.
FS_INSIDE = (2, 3, 7, 8)
SIDES = (("l", 1), ("r", -1))


class Sets:
    """Vertex indices of each face set and of the rings between sets."""

    def __init__(self, faces, face_sets):
        self.of = {}
        for s in np.unique(face_sets):
            idx = np.unique(faces[face_sets == s])
            self.of[int(s)] = idx[idx >= 0]
        n = faces.shape[1]
        pairs = {}
        for k in range(n):
            a, b = faces[:, k], faces[:, (k + 1) % n]
            b = np.where(b < 0, faces[:, 0], b)
            ok = (a >= 0) & (a != b)
            lo, hi = np.minimum(a, b)[ok], np.maximum(a, b)[ok]
            for e0, e1, s in zip(lo.tolist(), hi.tolist(), face_sets[ok].tolist()):
                pairs.setdefault((e0, e1), []).append(s)
        self.border = {}
        for (e0, e1), ss in pairs.items():
            if len(ss) == 2 and ss[0] != ss[1]:
                self.border.setdefault(frozenset(ss), set()).update((e0, e1))
        self.vertex_set = np.zeros(int(faces.max()) + 1, dtype=np.int64)
        for s, idx in self.of.items():
            self.vertex_set[idx] = s

    def ring(self, a, b):
        bs = b if isinstance(b, (tuple, list)) else (b,)
        out = set()
        for bb in bs:
            out |= self.border.get(frozenset((a, bb)), set())
        assert out, f"face sets {a} and {b} don't meet"
        return np.array(sorted(out))

    def verts(self, *sets):
        return np.unique(np.concatenate([self.of[s] for s in sets]))


def crotch_of(P, tol):
    mid = P[np.abs(P[:, 0]) < tol]
    return mid[mid[:, 2].argmin()]


def hip_rule(mh_joints, mh_verts):
    """Where MakeHuman's hip joint sits, as shares of the figure's height
    (out from the middle; above the crotch), so the realistic body's hip
    can be placed by the same rule."""
    height = np.ptp(mh_verts[:, 2])
    crotch = crotch_of(mh_verts, 1e-4)
    hip = mh_joints["l-upper-leg"]
    return hip[0] / height, (hip[2] - crotch[2]) / height


def realistic_landmarks(P, sets, hip_shares):
    """The realistic body's landmarks from its face sets."""
    at = lambda idx: P[idx].mean(axis=0)  # noqa: E731
    height = np.ptp(P[:, 2])
    k = height / 1.75
    lm = {"crown": P[P[:, 2].argmax()].copy(), "crotch": crotch_of(P, 0.004)}
    lm["crown"][0] = 0.0
    lm["neck"] = at(sets.ring(FS_TORSO, FS_HEAD))
    lm["waist"] = at(sets.ring(FS_WAIST, FS_TORSO))
    lm["pelvis"] = at(sets.ring(FS_PELVIS, FS_WAIST))
    for side, s in SIDES:
        lm[f"{side}-eye"] = fit_sphere(P[sets.of[FS_SOCKET[s]]])[0]
        lm[f"{side}-ear"] = at(sets.of[FS_EAR[s]])
        elbow = at(sets.ring(FS_UPPER_ARM[s], FS_FOREARM[s]))
        wrist = at(sets.ring(FS_FOREARM[s], FS_PALM[s]))
        ring = at(sets.ring(FS_TORSO, FS_UPPER_ARM[s]))
        # The joint sits inside the ring where the arm leaves the torso.
        axis = (ring - elbow) / np.linalg.norm(ring - elbow)
        lm[f"{side}-shoulder"] = ring + axis * 0.03 * k
        lm[f"{side}-elbow"], lm[f"{side}-wrist"] = elbow, wrist
        for f, (a, b, c, nail) in enumerate(FS_FINGERS[s], start=1):
            base = at(sets.ring(FS_PALM[s], a))
            j2 = at(sets.ring(a, b))
            j3 = at(sets.ring(b, c))
            d = (j3 - j2) / np.linalg.norm(j3 - j2)
            tips = P[sets.verts(c, nail)]
            tip = j3 + d * ((tips - j3) @ d).max() * 0.9
            if f == 1:
                # The thumb's first set is its metacarpal: the joint is by the wrist.
                j1 = wrist + (base - wrist) * 0.45
            else:
                # The knuckle sits inside the hand, behind where the finger shows.
                j1 = base - (j2 - base) * 0.22
            for m, p in enumerate((j1, j2, j3, tip), start=1):
                lm[f"{side}-f{f}-{m}"] = p
        thigh = at(sets.ring(FS_PELVIS, FS_THIGH[s]))
        hip = np.array([s * hip_shares[0] * height, thigh[1] + 0.008 * k,
                        lm["crotch"][2] + hip_shares[1] * height])
        knee_ring = at(sets.ring(FS_THIGH[s], FS_SHIN[s]))
        # The ring runs under the kneecap; the joint is above it.
        knee = knee_ring + (hip - knee_ring) / np.linalg.norm(hip - knee_ring) * 0.03 * k
        ankle = at(sets.ring(FS_SHIN[s], FS_FOOT[s]))
        ball = at(sets.ring(FS_FOOT[s], FS_TOE_BASE[s]))
        toes = P[sets.verts(*FS_TOES[s])]
        toe = toes[toes[:, 1].argmin()].copy()
        heel = P[sets.of[FS_FOOT[s]]]
        lm[f"{side}-hip"], lm[f"{side}-knee"], lm[f"{side}-ankle"] = hip, knee, ankle
        lm[f"{side}-ball"], lm[f"{side}-toe"] = ball, toe
        lm[f"{side}-heel"] = heel[heel[:, 1].argmax()].copy()
    return lm


def makehuman_landmarks(joints, verts):
    """MakeHuman's landmarks: `joints` is {MakeHuman joint name: centre}
    for the shape `verts` (the body's vertices) is in."""
    lm = {"crown": verts[verts[:, 2].argmax()].copy(), "crotch": crotch_of(verts, 1e-4)}
    lm["crown"][0] = 0.0
    names = {"shoulder": "shoulder", "elbow": "elbow", "wrist": "hand", "hip": "upper-leg",
             "knee": "knee", "ankle": "ankle", "eye": "eye"}
    for side, s in SIDES:
        for ours, theirs in names.items():
            lm[f"{side}-{ours}"] = joints[f"{side}-{theirs}"]
        for f in range(1, 6):
            for m in range(1, 5):
                lm[f"{side}-f{f}-{m}"] = joints[f"{side}-finger-{f}-{m}"]
        foot = verts[(verts[:, 0] * s > 0.02) & (verts[:, 2] < joints[f"{side}-ankle"][2])]
        lm[f"{side}-toe"] = foot[foot[:, 1].argmin()].copy()
        lm[f"{side}-heel"] = foot[foot[:, 1].argmax()].copy()
        lm[f"{side}-ball"] = joints[f"{side}-foot-1"]
    return lm


def fit_sphere(points):
    """Least-squares sphere through `points`: (centre, radius)."""
    A = np.c_[2 * points, np.ones(len(points))]
    b = (points ** 2).sum(axis=1)
    x = np.linalg.lstsq(A, b, rcond=None)[0]
    centre = x[:3]
    return centre, math.sqrt(max(x[3] + centre @ centre, 0.0))


def rotation_between(a, b):
    """The smallest rotation taking direction `a` to direction `b`."""
    a = a / np.linalg.norm(a)
    b = b / np.linalg.norm(b)
    v = np.cross(a, b)
    c = float(a @ b)
    if np.linalg.norm(v) < 1e-9:
        return np.eye(3)
    vx = np.array([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])
    return np.eye(3) + vx + vx @ vx / (1.0 + c)


def hand_frame(lm, side, s):
    """The hand's axes: along it (wrist to the middle knuckle), its
    normal, and across."""
    along = lm[f"{side}-f3-1"] - lm[f"{side}-wrist"]
    along /= np.linalg.norm(along)
    across = lm[f"{side}-f2-1"] - lm[f"{side}-f5-1"]
    normal = np.cross(along, across)
    normal /= np.linalg.norm(normal)
    return np.stack([along, normal, np.cross(normal, along)], axis=1)


# The bones the re-pose turns: name -> (head landmark, tail landmark,
# parent, the landmarks that ride on it).
def _bones(lm):
    fingers = [f"f{f}-{m}" for f in range(1, 6) for m in range(1, 5)]
    bones = {
        "Hips": ("crotch", "pelvis", None, []),
        "Spine": ("pelvis", "neck", "Hips", []),
        "Head": ("neck", "crown", "Spine", []),
    }
    for side, _ in SIDES:
        p = lambda n: f"{side}-{n}"  # noqa: E731
        bones[p("Clavicle")] = ("neck", p("shoulder"), "Spine", [])
        bones[p("UpperArm")] = (p("shoulder"), p("elbow"), p("Clavicle"), [p("elbow")])
        bones[p("LowerArm")] = (p("elbow"), p("wrist"), p("UpperArm"), [p("wrist")])
        bones[p("Hand")] = (p("wrist"), p("f3-1"), p("LowerArm"), [p(f) for f in fingers])
        bones[p("UpperLeg")] = (p("hip"), p("knee"), "Hips", [p("knee")])
        bones[p("LowerLeg")] = (p("knee"), p("ankle"), p("UpperLeg"), [p("ankle")])
        bones[p("Foot")] = (p("ankle"), p("toe"), p("LowerLeg"), [p("ball"), p("toe"), p("heel")])
    return bones


def repose(P, faces, lm, target, smooth=True):
    """The realistic body `P` with its arms, hands and legs turned to the
    directions of the landmarks `target` (MakeHuman's), by a Blender
    armature with bone-heat weights and a corrective smooth. Returns the
    new positions and the landmarks moved with their bones."""
    import bpy
    from mathutils import Matrix, Vector

    bones = _bones(lm)
    me = bpy.data.meshes.new("Repose")
    me.from_pydata([tuple(p) for p in P], [], [tuple(int(i) for i in f if i >= 0) for f in faces])
    me.update()
    body = bpy.data.objects.new("Repose", me)
    bpy.context.scene.collection.objects.link(body)
    arm_data = bpy.data.armatures.new("ReposeRig")
    arm = bpy.data.objects.new("ReposeRig", arm_data)
    bpy.context.scene.collection.objects.link(arm)
    bpy.ops.object.select_all(action="DESELECT")
    arm.select_set(True)
    bpy.context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    for name, (head, tail, _, _) in bones.items():
        eb = arm_data.edit_bones.new(name)
        eb.head, eb.tail = Vector(lm[head]), Vector(lm[tail])
    for name, (_, _, parent, _) in bones.items():
        if parent:
            arm_data.edit_bones[name].parent = arm_data.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    body.select_set(True)
    arm.select_set(True)
    bpy.context.view_layer.objects.active = arm
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    if smooth:
        mod = body.modifiers.new("Corrective", "CORRECTIVE_SMOOTH")
        mod.rest_source = "ORCO"
        mod.smooth_type = "LENGTH_WEIGHTED"
        mod.iterations = 12
        mod.factor = 0.6
        mod.use_only_smooth = False
    bpy.ops.object.select_all(action="DESELECT")
    arm.select_set(True)
    bpy.context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="POSE")

    moved = {k: np.array(v, dtype=np.float64) for k, v in lm.items()}
    children = {}
    for name, (_, _, parent, _) in bones.items():
        children.setdefault(parent, []).append(name)

    turned = {name: np.eye(3) for name in bones}

    def below(name):
        out = [name]
        for child in children.get(name, []):
            out += below(child)
        return out

    def turn(name, R):
        """Turns bone `name` (and all that hangs on it) by `R` about its head."""
        pivot = moved[bones[name][0]].copy()
        for bone in below(name):
            turned[bone] = R @ turned[bone]
        for key in {k for bone in below(name) for k in bones[bone][3]}:
            moved[key] = pivot + R @ (moved[key] - pivot)
        pb = arm.pose.bones[name]
        bpy.context.view_layer.update()
        m = pb.matrix.copy()
        T = Matrix.Translation(Vector(pivot))
        pb.matrix = T @ Matrix(R.tolist()).to_4x4() @ T.inverted() @ m
        bpy.context.view_layer.update()

    for side, s in SIDES:
        p = lambda n: f"{side}-{n}"  # noqa: E731
        for bone, a, b in (("UpperArm", "shoulder", "elbow"), ("LowerArm", "elbow", "wrist"),
                           ("UpperLeg", "hip", "knee"), ("LowerLeg", "knee", "ankle")):
            turn(p(bone), rotation_between(moved[p(b)] - moved[p(a)], target[p(b)] - target[p(a)]))
        turn(p("Hand"), hand_frame(target, side, s) @ hand_frame(moved, side, s).T)
        # The foot: flat on the ground as it was, turned to point where
        # MakeHuman's points.
        def heading(q):
            d = q[p("toe")] - q[p("heel")]
            return math.atan2(d[0], -d[1])
        yaw = heading(target) - heading(lm)
        c, sn = math.cos(yaw), math.sin(yaw)
        flat = np.array([[c, -sn, 0.0], [sn, c, 0.0], [0.0, 0.0, 1.0]])
        # Undo what the leg's turns did to the foot, then apply the yaw.
        turn(p("Foot"), flat @ turned[p("Foot")].T)
    bpy.ops.object.mode_set(mode="OBJECT")
    deps = bpy.context.evaluated_depsgraph_get()
    ev = body.evaluated_get(deps)
    out = np.array([v.co[:] for v in ev.data.vertices])
    bpy.data.objects.remove(body)
    bpy.data.objects.remove(arm)
    # Stand both feet on the ground again.
    drop = out[:, 2].min() - P[:, 2].min()
    out[:, 2] -= drop
    for key in moved:
        moved[key][2] -= drop
    return out, moved


def symmetrize(P, mirror):
    """`P` averaged with its mirror image."""
    Q = P[mirror] * np.array([-1.0, 1.0, 1.0])
    return (P + Q) / 2


def control_pairs(src, dst):
    """Matching landmark positions of two bodies, for the space warp:
    the ones that mean the same on both."""
    keys = ["crown", "crotch"]
    for side, _ in SIDES:
        keys += [f"{side}-{n}" for n in ("eye", "shoulder", "elbow", "wrist", "hip", "knee", "ankle",
                                         "toe", "heel")]
        keys += [f"{side}-f{f}-{m}" for f in range(1, 6) for m in range(1, 5)]
    return np.array([src[k] for k in keys]), np.array([dst[k] for k in keys]), keys
