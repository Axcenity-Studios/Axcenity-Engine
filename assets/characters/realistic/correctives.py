"""Pose-driven corrective shapes (CO5): what linear skinning loses at the
shoulder when the arm is raised, put back by shapes the engine leans on
as the upper arm turns (`crates/anim/src/correctives.rs`; the QA scripts'
`Character.drive` does the same).

**The rule** (written to `Realistic.build.json` as `correctives`, the form
architecture.md section 7 keeps for the manifest): `{shape, bone, axis,
angles: [start, full]}`. A shape's weight is how far `bone` has turned
from its rest about `axis`, from 0 at `start` degrees to 1 at `full` and
beyond. `axis` is in the space of the bone's parent at rest; the turn is
the bone's own (its place in its parent now, over its place there at
rest), and its angle about the axis is the twist part of it
(`twist_angle`), so an arm swung forward instead of raised leans on these
shapes only as far as it is also raised.

**The shapes.** Raised with the whole turn at the shoulder joint, the
skin over the joint is pulled in toward it: the deltoid's cap flattens
and the top of the shoulder caves. Two shapes per side put the cap back,
`Corrective_Shoulder_Level_*` (full with the arm level, 45 degrees up
from rest) and `Corrective_Shoulder_Raised_*` (what the arm 150 degrees
from hanging needs on top of it). Each is made in its pose: the skin
within `REACH` of the joint is moved back out along the line from the
joint toward the distance it has from it at rest (times a factor found so
the joint keeps `KEEP` of its local volume, the QA scripts' measure),
smoothed, and carried to the rest pose by each vertex's own skinning
matrix, so the engine's skinning lands it where it was made.
"""

import math
import os
import sys

import numpy as np
from mathutils import Matrix, Vector

from common import array, evaluated_positions

HERE = os.path.dirname(os.path.abspath(__file__))
QA = os.path.join(os.path.dirname(HERE), "humanoid", "qa")

RADIUS = 0.09      # the QA scripts' region round the shoulder joint, metres
REACH = 0.16       # where the correction has faded out
# (shape, the QA pose it is made in, degrees up from rest where it starts and is full, local volume kept)
STAGES = [("Level", "shoulder_abd_90", 0.0, 45.0, 0.955), ("Raised", "shoulder_abd_150", 45.0, 105.0, 0.86)]


def _qa():
    if QA not in sys.path:
        sys.path.insert(0, QA)
    import qa_scene
    return qa_scene


def posed_rig(armature, custom):
    """The build's armature as the QA scripts pose a character: the same
    poses and the same twist and helper bones."""
    Q = _qa()

    class Posed(Q.Character):
        def __init__(self):
            self.arm, self.custom, self.parts = armature, custom, {}

        def set_eyes(self):
            pass

    return Q, Posed()


def skin_matrices(armature, weights):
    """Each vertex's skinning matrix in the armature's pose (its turn and
    stretch only), (n, 3, 3)."""
    T = np.array([np.array((armature.pose.bones[name].matrix @ armature.data.bones[name].matrix_local.inverted())
                           .to_3x3()) for name in weights.bones])
    return np.einsum("vb,bij->vij", weights.W, T)


def local_volume(co, tris, centre):
    a, b, c = (co[tris[:, k]] - centre for k in range(3))
    return float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0)


def _fade(distance):
    t = np.clip((REACH - distance) / (REACH - RADIUS * 1.15), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def hold(rest, posed, c_rest, c_posed, tris, keep, lap):
    """`posed` with the skin round the joint moved back out so the joint
    keeps `keep` of its local volume; and the factor on the rest
    distance that took."""
    r0 = np.linalg.norm(rest - c_rest, axis=1)
    out = posed - c_posed
    r1 = np.maximum(np.linalg.norm(out, axis=1), 1e-9)
    fade = _fade(r0)
    region = r0 < RADIUS
    inside = tris[region[tris].all(axis=1)]
    v0 = local_volume(rest, inside, c_rest)

    def at(s):
        radius = r1 + fade * np.maximum(s * r0 - r1, 0.0)
        moved = c_posed + out * (radius / r1)[:, None]
        delta = moved - posed
        # Smoothed, so the cap has no facets; twice is enough at this size.
        for _ in range(2):
            delta = (delta + lap.mean(delta)) / 2
        return posed + delta

    low, high = 0.7, 1.5
    for _ in range(24):
        mid = (low + high) / 2
        if local_volume(at(mid), inside, c_posed) / v0 < keep:
            low = mid
        else:
            high = mid
    return at(high), high, local_volume(posed, inside, c_posed) / v0


def build(body, armature, weights, P, custom, mirror, lap, tris, log=print):
    """The corrective shapes as rest-pose offsets, and their rules:
    ({shape: offsets}, [rule]). `custom` is the build data's twist and
    helper lists; `mirror` each vertex's mirror vertex."""
    Q, rig = posed_rig(armature, custom)
    rest = array(P)
    side = "Left"
    c_rest = np.array(armature.data.bones[side + "UpperArm"].head_local)

    def raise_axis(s):
        """The axis side `s`'s arm is raised about, in its parent's rest
        space, turning upward."""
        bone = armature.data.bones[s + "UpperArm"]
        axis = Vector((0.0, 1.0, 0.0))
        along = bone.tail_local - bone.head_local
        if (Matrix.Rotation(math.radians(1.0), 3, axis) @ along).z < along.z:
            axis = -axis
        return bone.parent.matrix_local.to_3x3().inverted() @ axis

    shapes, rules, report = {}, [], {}
    so_far = np.zeros_like(rest)
    for name, pose, start, full, keep in STAGES:
        Q.apply_pose(rig, pose)
        posed = evaluated_positions(body)
        c_posed = np.array(armature.pose.bones[side + "UpperArm"].head)
        target, factor, before = hold(rest, posed, c_rest, c_posed, tris, keep, lap)
        M = skin_matrices(armature, weights)
        total = np.einsum("vij,vj->vi", np.linalg.inv(M), target - posed)
        left = total - so_far
        left[np.linalg.norm(left, axis=1) < 2e-5] = 0.0
        so_far = so_far + left
        right = left[mirror] * np.array([-1.0, 1.0, 1.0])
        for s, d in (("Left", left), ("Right", right)):
            shape = f"Corrective_Shoulder_{name}_{s}"
            shapes[shape] = d
            rules.append({"shape": shape, "bone": s + "UpperArm", "axis": [round(v, 5) for v in raise_axis(s)],
                          "angles": [start, full]})
        report[name] = {"pose": pose, "local_volume_before": round(before, 3), "kept": keep,
                        "rest_distance_factor": round(factor, 3),
                        "largest_move_mm": round(float(np.linalg.norm(left, axis=1).max()) * 1000, 1),
                        "vertices": int((np.linalg.norm(left, axis=1) > 0).sum())}
        log(f"  corrective {name}: {pose} kept {before:.3f} of its local volume, now {keep}; "
            f"rest distance times {factor:.3f}, largest move {report[name]['largest_move_mm']} mm")
    rig.pose_reset()
    return shapes, rules, report


def add_keys(body, shapes, P):
    base = array(P)
    for name in sorted(shapes):
        key = body.shape_key_add(name=name, from_mix=False)
        key.data.foreach_set("co", (base + shapes[name]).ravel())
        key.value = 0.0
