"""Pose-driven corrective shapes (CO5): what linear skinning loses round
the shoulder when the arm is raised, put back by shapes the engine leans
on as the upper arm turns (`crates/anim/src/correctives.rs`; the QA
scripts' `Character.drive_correctives` does the same).

**The rule** (written to `Realistic.build.json` as `correctives`, the form
architecture.md section 7 keeps for the manifest): `{shape, bone, axis,
angles: [start, full]}`. A shape's weight is how far `bone` has turned
from its rest about `axis`, from 0 at `start` degrees to 1 at `full` and
beyond. `axis` is in the space of the bone's parent at rest; the turn is
the bone's own (its place in its parent now, over its place there at
rest), and its angle about the axis is the twist part of it
(`twist_angle`), so an arm swung forward instead of raised leans on these
shapes only as far as it is also raised.

**The shapes** are a better deformation of the same pose, not a fit to
any measure. Linear skinning turns each piece of skin rigidly with a mix
of bones, which keeps the skin's detail and loses its volume where the
bones part ways: the top of the shoulder caves and the armpit folds.
Smoothing the posed skin gets the broad form right and loses the
detail. **Delta mush** takes the broad form from the one and the detail
from the other: the body is smoothed at rest and each vertex's offset
from its own smoothed place is kept, in the frame of the smoothed
surface there (its normal and a tangent); the posed body is smoothed the
same way, and each vertex is put back at its kept offset in the posed
surface's frame (`Mush`). At rest it changes nothing.

A shape is that target less the skinned pose, inside a soft region round
the shoulder girdle (`girdle`: from the root of the neck along the
collarbone and down the upper arm, wide enough for the cap, the
trapezius' slope, the armpit and the edges of the pectoral and the
latissimus), carried to the rest pose by each vertex's own skinning
matrix so the engine's skinning lands it where it was made. Two per
side, each made in its pose: `Corrective_Shoulder_Level_*` (the arm
level, 45 degrees up from rest) and `Corrective_Shoulder_Raised_*` (what
the arm 150 degrees from hanging needs on top of it).
"""

import math
import os
import sys

import numpy as np
from mathutils import Matrix, Vector

from common import array, evaluated_positions

HERE = os.path.dirname(os.path.abspath(__file__))
QA = os.path.join(os.path.dirname(HERE), "humanoid", "qa")

# Delta mush: how many times the skin is smoothed (each time halfway to
# its neighbours' mean). On this mesh (10 to 25 mm edges at the shoulder)
# it spreads a bend over about a hand's width, which is how far a
# shoulder's skin slides.
SMOOTHINGS = 24
# The girdle's region: full within NEAR metres of the line from the neck's
# root to the shoulder joint to the middle of the upper arm, none past FAR.
NEAR, FAR = 0.11, 0.21
# (shape, the QA pose it is made in, degrees up from rest where it starts and is full)
STAGES = [("Level", "shoulder_abd_90", 0.0, 45.0), ("Raised", "shoulder_abd_150", 45.0, 105.0)]


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


class Mush:
    """Delta mush on one mesh: `target(posed)` is the posed skin with its
    broad form smoothed and its rest detail put back (module docstring)."""

    def __init__(self, rest, tris, lap, smoothings=SMOOTHINGS):
        self.tris, self.lap, self.smoothings = tris, lap, smoothings
        # One neighbour per vertex fixes the tangent of its frame.
        self.toward = np.zeros(len(rest), dtype=np.int64)
        self.toward[lap.e[::-1, 0]] = lap.e[::-1, 1]
        self.toward[lap.e[::-1, 1]] = lap.e[::-1, 0]
        smooth = self._smooth(rest)
        frames = self._frames(smooth)
        self.offset = np.einsum("vij,vj->vi", frames, rest - smooth)

    def _smooth(self, X):
        for _ in range(self.smoothings):
            X = (X + self.lap.mean(X)) / 2
        return X

    def _frames(self, X):
        """Per vertex, the rows tangent, bitangent, normal of the surface `X`."""
        a, b, c = (X[self.tris[:, k]] for k in range(3))
        face = np.cross(b - a, c - a)
        n = np.zeros_like(X)
        for k in range(3):
            np.add.at(n, self.tris[:, k], face)
        n /= np.maximum(np.linalg.norm(n, axis=1, keepdims=True), 1e-12)
        t = X[self.toward] - X
        t -= n * np.einsum("ij,ij->i", t, n)[:, None]
        t /= np.maximum(np.linalg.norm(t, axis=1, keepdims=True), 1e-12)
        return np.stack([t, np.cross(n, t), n], axis=1)

    def target(self, posed):
        smooth = self._smooth(posed)
        return smooth + np.einsum("vji,vj->vi", self._frames(smooth), self.offset)


def girdle(rest, armature, side):
    """Per vertex, 0 to 1: the soft region round side `side`'s shoulder
    girdle, by distance from the line neck's root, shoulder joint, middle
    of the upper arm, and only on that side of the body."""
    head = lambda name: np.array(armature.data.bones[name].head_local)  # noqa: E731
    joint, elbow = head(side + "UpperArm"), head(side + "LowerArm")
    path = [head(side + "Shoulder"), joint, joint + (elbow - joint) * 0.55]
    away = np.full(len(rest), np.inf)
    for a, b in zip(path, path[1:]):
        t = np.clip(((rest - a) @ (b - a)) / ((b - a) @ (b - a)), 0.0, 1.0)
        away = np.minimum(away, np.linalg.norm(rest - (a + t[:, None] * (b - a)), axis=1))
    m = np.clip((FAR - away) / (FAR - NEAR), 0.0, 1.0)
    m = m * m * (3.0 - 2.0 * m)
    # Nothing across the middle of the body, and nothing on the head.
    s = 1.0 if side == "Left" else -1.0
    m *= np.clip(rest[:, 0] * s / 0.03, 0.0, 1.0)
    m *= np.clip((head("Head")[2] - 0.02 - rest[:, 2]) / 0.04, 0.0, 1.0)
    return m


def build(body, armature, weights, P, custom, mirror, lap, tris, log=print):
    """The corrective shapes as rest-pose offsets, and their rules:
    ({shape: offsets}, [rule], report). `custom` is the build data's
    twist and helper lists; `mirror` each vertex's mirror vertex."""
    Q, rig = posed_rig(armature, custom)
    rest = array(P)
    side = "Left"
    mush = Mush(rest, tris, lap)
    region = girdle(rest, armature, side)

    def raise_axis(s):
        """The axis side `s`'s arm is raised about, in its parent's rest
        space, turning upward."""
        bone = armature.data.bones[s + "UpperArm"]
        axis = Vector((0.0, 1.0, 0.0))
        along = bone.tail_local - bone.head_local
        if (Matrix.Rotation(math.radians(1.0), 3, axis) @ along).z < along.z:
            axis = -axis
        return bone.parent.matrix_local.to_3x3().inverted() @ axis

    shapes, rules, report = {}, [], {"method": "delta mush", "smoothings": SMOOTHINGS, "region_m": [NEAR, FAR]}
    so_far = np.zeros_like(rest)
    for name, pose, start, full in STAGES:
        Q.apply_pose(rig, pose)
        posed = evaluated_positions(body)
        delta = (mush.target(posed) - posed) * region[:, None]
        M = skin_matrices(armature, weights)
        total = np.einsum("vij,vj->vi", np.linalg.inv(M), delta)
        left = total - so_far
        left[np.linalg.norm(left, axis=1) < 2e-5] = 0.0
        so_far = so_far + left
        right = left[mirror] * np.array([-1.0, 1.0, 1.0])
        for s, d in (("Left", left), ("Right", right)):
            shape = f"Corrective_Shoulder_{name}_{s}"
            shapes[shape] = d
            rules.append({"shape": shape, "bone": s + "UpperArm", "axis": [round(v, 5) for v in raise_axis(s)],
                          "angles": [start, full]})
        moved = np.linalg.norm(delta, axis=1)
        report[name] = {"pose": pose, "largest_move_mm": round(float(moved.max()) * 1000, 1),
                        "vertices": int((moved > 2e-5).sum())}
        log(f"  corrective {name}: {pose}, {report[name]['vertices']} vertices, "
            f"largest move {report[name]['largest_move_mm']} mm")
    rig.pose_reset()
    return shapes, rules, report


def add_keys(body, shapes, P):
    base = array(P)
    for name in sorted(shapes):
        key = body.shape_key_add(name=name, from_mix=False)
        key.data.foreach_set("co", (base + shapes[name]).ravel())
        key.value = 0.0
