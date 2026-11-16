"""The Axcenity Humanoid skeleton fitted to the realistic body, its twist
and helper bones, and the skin weights.

Joints come from the base mesh's face sets, as the stylized body's do: the
loop where two sets meet (upper arm and forearm) is a ring round the limb,
and its centre is the joint. The bones, their names and their rolls are
the stylized humanoid's (`../humanoid/rig.py` `build_armature`), so every
clip, retarget profile and state machine fits.

Added bones (architecture section 12; none is keyed by a clip, `anim`'s
`twist.rs` drives them after the pose and the QA scripts do the same):

- Twist bones, inside the bone they're named after: `LeftUpperArmTwist`
  and `LeftUpperLegTwist` hold the half of the limb nearer the body and
  keep half of the limb's own twist out of it; `LeftLowerArmTwist` and
  `LeftLowerLegTwist` hold the half nearer the hand or foot and take half
  of its twist.
- Helper bones at the big hinges, each turning half as far as the joint
  it sits on: `LeftShoulderHelper` (on the shoulder, from the upper arm),
  `LeftElbowHelper`, `LeftHipHelper`, `LeftKneeHelper`. Skin over a joint
  that is shared between its two bones goes to the helper instead, so it
  moves round the joint on an arc and keeps its width, where a plain
  blend of two bones cuts the corner and the joint collapses.
"""

import math

import bpy
import numpy as np
from mathutils import Vector

from common import (
    FINGERS, FS_EAR, FS_FINGERS, FS_FOOT, FS_FOREARM, FS_HEAD, FS_MOUTH_LOWER, FS_MUZZLE, FS_PALM,
    FS_PELVIS, FS_SHIN, FS_THIGH, FS_TOE_BASE, FS_TOES, FS_TORSO, FS_UPPER_ARM, FS_WAIST,
    HEAD_SETS, SEGMENTS, SIDES, array, centroid, edge_neighbours, select_only, stylized, v,
)

# (twist bone, the bone it's inside, the bone whose turn it reads, share).
# A negative share keeps that much of the limb's own twist out of the bone.
TWISTS = [
    ("UpperArmTwist", "UpperArm", "UpperArm", -0.5),
    ("LowerArmTwist", "LowerArm", "Hand", 0.5),
    ("UpperLegTwist", "UpperLeg", "UpperLeg", -0.5),
    ("LowerLegTwist", "LowerLeg", "Foot", 0.5),
]
# (helper bone, its parent, the bone it turns half as far as).
HELPERS = [
    ("ShoulderHelper", "Shoulder", "UpperArm"),
    ("ElbowHelper", "UpperArm", "LowerArm"),
    ("HipHelper", "Hips", "UpperLeg"),
    ("KneeHelper", "UpperLeg", "LowerLeg"),
]


def stylized_rig():
    return stylized("rig")


def at(P, idx):
    return centroid(P[i] for i in idx)


def added_bones():
    """[(name, parent)] of every twist and helper bone, for the manifest."""
    out = []
    for side, _ in SIDES:
        out += [(side + name, side + parent) for name, parent, _, _ in TWISTS]
        out += [(side + name, "Hips" if parent == "Hips" else side + parent)
                for name, parent, _ in HELPERS]
    return out


def fit_joints(P, lm, eyes):
    """Bone name -> (head, tail, parent) of the contract's bones, from body
    positions `P` and the eye centres `eyes` ({1: left, -1: right})."""
    j = {}
    crown = max(p.z for p in P)
    height = crown - min(p.z for p in P)
    k = height / 1.75
    # Centre line. The spine runs behind the body's middle.
    pelvis_top = at(P, lm.ring(FS_PELVIS, FS_WAIST))
    waist_top = at(P, lm.ring(FS_WAIST, FS_TORSO))
    neck_base = at(P, lm.ring(FS_TORSO, FS_HEAD))
    centre = [P[i] for i in lm.verts(FS_PELVIS) if abs(P[i].x) < 0.004]
    crotch_z = min(p.z for p in centre)
    hip_z = crotch_z + 0.058 * height
    ears = centroid(P[i] for s in (1, -1) for i in lm.verts(FS_EAR[s]))
    j["Root"] = (v(0, 0, 0), v(0, -0.12, 0), None)
    back = 0.022 * k
    hips = v(0, pelvis_top.y + back, hip_z + 0.025 * k)
    spine = v(0, pelvis_top.y + back, pelvis_top.z)
    chest = v(0, waist_top.y + back, waist_top.z)
    neck = v(0, neck_base.y + 0.012 * k, neck_base.z + 0.01 * k)
    upper = chest.lerp(neck, 0.52)
    j["Hips"] = (hips, spine, "Root")
    j["Spine"] = (spine, chest, "Hips")
    j["Chest"] = (chest, upper, "Spine")
    j["UpperChest"] = (upper, neck, "Chest")
    # The head turns on the top of the spine: between the ears, below them.
    head = v(0, ears.y + 0.006 * k, ears.z - 0.028 * k)
    j["Neck"] = (neck, head, "UpperChest")
    j["Head"] = (head, v(0, head.y, crown), "Neck")
    # The jaw hinges just in front of the ears.
    chin_line = [P[i] for i in lm.verts(FS_HEAD) if abs(P[i].x) < 0.004 and P[i].y < head.y - 0.03]
    muzzle_low = min(P[i].z for i in lm.verts(FS_MUZZLE))
    chin = min((p for p in chin_line if p.z < muzzle_low), key=lambda p: p.y)
    hinge = v(0, ears.y - 0.018 * k, ears.z - 0.006 * k)
    j["Jaw"] = (hinge, v(0, chin.y + 0.02, chin.z + 0.012), "Head")
    for side, s in SIDES:
        eye = eyes[s]
        j[f"{side}Eye"] = (eye, eye + v(0, -0.03, 0), "Head")
        elbow = at(P, lm.ring(FS_UPPER_ARM[s], FS_FOREARM[s]))
        wrist = at(P, lm.ring(FS_FOREARM[s], FS_PALM[s]))
        # The shoulder joint: inside the ring where the arm leaves the
        # torso, toward the body along the arm.
        ring = at(P, lm.ring(FS_TORSO, FS_UPPER_ARM[s]))
        axis = (ring - elbow).normalized()
        top = max((P[i] for i in lm.ring(FS_TORSO, FS_UPPER_ARM[s])), key=lambda p: p.z)
        shoulder = ring + axis * 0.03 * k
        shoulder.z = max(shoulder.z, top.z - 0.055 * k)
        clav = v(s * 0.022 * k, neck.y - 0.045 * k, neck.z - 0.035 * k)
        j[f"{side}Shoulder"] = (clav, shoulder, "UpperChest")
        j[f"{side}UpperArm"] = (shoulder, elbow, f"{side}Shoulder")
        j[f"{side}LowerArm"] = (elbow, wrist, f"{side}UpperArm")
        knuckles = {}
        for finger in FINGERS:
            p_set, i_set, d_set, nail = FS_FINGERS[s][finger]
            base = at(P, lm.ring(FS_PALM[s], p_set))
            pip = at(P, lm.ring(p_set, i_set))
            dip = at(P, lm.ring(i_set, d_set))
            tip_verts = [P[i] for i in set(lm.verts(d_set)) | set(lm.verts(nail))]
            direction = (dip - pip).normalized()
            tip = max(tip_verts, key=lambda p: (p - dip).dot(direction))
            tip = dip + direction * (tip - dip).dot(direction) * 0.9
            if finger == "Thumb":
                # The thumb's first bone is its metacarpal, from by the wrist.
                mcp = wrist.lerp(base, 0.45)
            else:
                # The knuckle sits inside the hand, behind where the finger shows.
                along = (pip - base).normalized()
                mcp = base - along * 0.22 * (pip - base).length
            knuckles[finger] = mcp
            parent = f"{side}Hand"
            for seg, (a, b) in zip(SEGMENTS, [(mcp, pip), (pip, dip), (dip, tip)]):
                bone = f"{side}{finger}{seg}"
                j[bone] = (a, b, parent)
                parent = bone
        j[f"{side}Hand"] = (wrist, knuckles["Middle"], f"{side}LowerArm")
        thigh = at(P, lm.ring(FS_PELVIS, FS_THIGH[s]))
        knee_ring = at(P, lm.ring(FS_THIGH[s], FS_SHIN[s]))
        ankle_ring = at(P, lm.ring(FS_SHIN[s], FS_FOOT[s]))
        hip = v(s * 0.0525 * height, thigh.y + 0.008 * k, hip_z)
        # The knee turns above the ring where the sets meet (the ring runs
        # under the kneecap), and behind the leg's middle.
        knee = knee_ring + (hip - knee_ring).normalized() * 0.03 * k + v(0, 0.006 * k, 0)
        ankle = v(ankle_ring.x, ankle_ring.y + 0.004 * k, ankle_ring.z - 0.004 * k)
        ball = at(P, lm.ring(FS_FOOT[s], FS_TOE_BASE[s]))
        toe_verts = [P[i] for fs in FS_TOES[s] for i in lm.verts(fs)]
        toe_tip = min(toe_verts, key=lambda p: p.y)
        # Along the foot, which may point a little outward.
        forward = Vector((ball.x - ankle.x, ball.y - ankle.y, 0.0)).normalized()
        ball = v(ball.x, ball.y, 0.022 * k)
        reach = (toe_tip - ball).dot(forward) * 0.92
        j[f"{side}UpperLeg"] = (hip, knee, "Hips")
        j[f"{side}LowerLeg"] = (knee, ankle, f"{side}UpperLeg")
        j[f"{side}Foot"] = (ankle, ball, f"{side}LowerLeg")
        j[f"{side}Toes"] = (ball, ball + forward * reach, f"{side}Foot")
    return j


def extra_joints(joints):
    """The twist and helper bones' (head, tail, parent), from the fitted
    contract bones."""
    extra = {}
    for side, _ in SIDES:
        for name, inside, _, share in TWISTS:
            head, tail, _ = joints[side + inside]
            # A twist bone lies along its bone, in the half it holds.
            a, b = (0.08, 0.5) if share < 0 else (0.5, 0.92)
            extra[side + name] = (head.lerp(tail, a), head.lerp(tail, b), side + inside)
        for name, parent, source in HELPERS:
            head, tail, _ = joints[side + source]
            parent = "Hips" if parent == "Hips" else side + parent
            extra[side + name] = (head, head.lerp(tail, 0.3), parent)
    return extra


def build_armature(joints, name):
    """The stylized humanoid's armature for `joints` plus the twist and
    helper bones, each with its source bone's roll."""
    armature = stylized_rig().build_armature(joints, name)
    extra = extra_joints(joints)
    select_only([armature])
    bpy.ops.object.mode_set(mode="EDIT")
    ebs = armature.data.edit_bones
    for side, _ in SIDES:
        for bone, like in [(side + n, side + inside) for n, inside, _, _ in TWISTS] + \
                          [(side + n, side + source) for n, _, source in HELPERS]:
            head, tail, parent = extra[bone]
            eb = ebs.new(bone)
            eb.head, eb.tail = head, tail
            eb.parent = ebs[parent]
            eb.use_connect = False
            eb.roll = 0.0
            eb.align_roll(ebs[like].z_axis)
            eb.use_deform = True
    bpy.ops.object.mode_set(mode="OBJECT")
    armature.select_set(False)
    return armature


# -- weights -------------------------------------------------------------------------

def _smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


class Weights:
    """Per-vertex bone weights as an array, by bone name."""

    def __init__(self, obj, armature):
        self.bones = [b.name for b in armature.data.bones]
        self.index = {name: k for k, name in enumerate(self.bones)}
        self.W = np.zeros((len(obj.data.vertices), len(self.bones)))
        names = {vg.index: vg.name for vg in obj.vertex_groups}
        for vert in obj.data.vertices:
            for g in vert.groups:
                name = names[g.group]
                if name in self.index:
                    self.W[vert.index, self.index[name]] = g.weight
        self.normalize()

    def col(self, name):
        return self.W[:, self.index[name]]

    def normalize(self):
        total = self.W.sum(axis=1, keepdims=True)
        self.W = self.W / np.maximum(total, 1e-12)

    def limit(self, keep=4, floor=0.004):
        """At most `keep` influences per vertex, normalized."""
        order = np.argsort(-self.W, axis=1)
        mask = np.zeros_like(self.W, dtype=bool)
        np.put_along_axis(mask, order[:, :keep], True, axis=1)
        self.W = np.where(mask & (self.W > floor), self.W, 0.0)
        self.normalize()

    def write(self, obj):
        for vg in list(obj.vertex_groups):
            obj.vertex_groups.remove(vg)
        groups = [obj.vertex_groups.new(name=name) for name in self.bones]
        for k, group in enumerate(groups):
            col = self.W[:, k]
            for i in np.nonzero(col > 0.0)[0]:
                group.add([int(i)], float(col[i]), "REPLACE")
        # Bones nothing is weighted to keep no group.
        for group in list(obj.vertex_groups):
            if not (self.W[:, self.index[group.name]] > 0).any():
                obj.vertex_groups.remove(group)

    def as_dicts(self):
        out = []
        for row in self.W:
            out.append({self.bones[k]: float(row[k]) for k in np.nonzero(row > 0)[0]})
        return out


def _smooth(values, nbr, iterations, keep=None):
    """Laplacian smoothing of per-vertex rows (an array [n] or [n, k])."""
    vals = np.array(values, dtype=np.float64)
    count = np.array([max(1, len(n)) for n in nbr], dtype=np.float64)
    src = np.concatenate([np.full(len(n), i) for i, n in enumerate(nbr)])
    dst = np.concatenate([np.array(n, dtype=np.int64) for n in nbr])
    for _ in range(iterations):
        total = np.zeros_like(vals)
        np.add.at(total, src, vals[dst])
        mean = total / (count[:, None] if vals.ndim == 2 else count)
        new = 0.5 * vals + 0.5 * mean
        if keep is not None:
            new[keep] = vals[keep]
        vals = new
    return vals


def skin(body, armature, P, lm, joints, even=None):
    """Weights `body` to `armature`: bone-heat weights from the contract's
    bones, then the head and neck, the jaw, the fingers and toes from the
    face sets, each limb's twist bones, and the helper bones at the big
    joints. At most four per vertex. Returns the `Weights`."""
    rigs = stylized_rig()
    added = {name for name, _ in added_bones()}
    skip = ("Root", "Jaw", "LeftEye", "RightEye", *added)
    rigs.auto_weights(body, armature, skip=skip)
    for b in armature.data.bones:
        b.use_deform = b.name != "Root"
    w = Weights(body, armature)
    X = array(P)
    nbr = edge_neighbours(body.data)
    of_set = {fs: np.array(sorted(idx), dtype=np.int64) for fs, idx in lm.of_set.items()}
    in_sets = lambda sets: np.unique(np.concatenate([of_set[fs] for fs in sets]))  # noqa: E731
    J = {name: (np.array(h), np.array(t)) for name, (h, t, _) in joints.items()}

    # No weight from the other side's bones (the thighs touch, and bone
    # heat lets a little through).
    for side, s in SIDES:
        other = "Right" if side == "Left" else "Left"
        mine = X[:, 0] * s > 0.012
        for name in w.bones:
            if name.startswith(other):
                w.col(name)[mine] = 0.0
    w.normalize()

    _head_and_neck(w, X, in_sets, J)
    _jaw(w, X, body.data, lm, in_sets, of_set, J, nbr)
    if even is not None:
        # The jaw's hold the same on both sides of the mouth (`even`: each
        # vertex's mirror vertex, and how much of it is the mouth's), as
        # the mouth's form is (`face.prepare`).
        mirror, share = even
        head, jaw = w.index["Head"], w.index["Jaw"]
        held = w.W[:, head] + w.W[:, jaw]
        amount = w.W[:, jaw] + share * (0.5 * (w.W[:, jaw] + w.W[mirror, jaw]) - w.W[:, jaw])
        amount = np.minimum(amount, held)
        w.W[:, head], w.W[:, jaw] = held - amount, amount
    for side, s in SIDES:
        _shoulder(w, X, body.data, lm, of_set, side, s)
        _hand(w, X, of_set, J, side, s)
        _toes(w, X, of_set, in_sets, J, side, s)
    w.normalize()
    # Soften what the rules above cut, away from the fingers and the face.
    soft = np.ones(len(X), dtype=bool)
    soft[in_sets([fs for s in (1, -1) for sets in FS_FINGERS[s].values() for fs in sets])] = False
    soft[in_sets(HEAD_SETS)] = False
    w.W[soft] = _smooth(w.W, nbr, 2)[soft]
    w.normalize()
    for side, s in SIDES:
        for name, inside, _, share in TWISTS:
            _twist(w, X, J, side + name, side + inside, share)
        _helper(w, X, J, side + "ElbowHelper", [side + "UpperArm", side + "UpperArmTwist"],
                [side + "LowerArm", side + "LowerArmTwist"], side + "LowerArm",
                toward=np.array([0.0, -1.0, 0.0]), inner=0.35)
        _helper(w, X, J, side + "KneeHelper", [side + "UpperLeg", side + "UpperLegTwist"],
                [side + "LowerLeg", side + "LowerLegTwist"], side + "LowerLeg",
                toward=np.array([0.0, 1.0, 0.0]), inner=0.35)
        _helper(w, X, J, side + "ShoulderHelper", [side + "Shoulder", "UpperChest", "Chest"],
                [side + "UpperArm", side + "UpperArmTwist"], side + "UpperArm",
                toward=np.array([0.0, 0.0, -1.0]), inner=0.0, flat=0.45)
        _helper(w, X, J, side + "HipHelper", ["Hips", "Spine"],
                [side + "UpperLeg", side + "UpperLegTwist"], side + "UpperLeg",
                toward=np.array([0.0, -1.0, 0.0]), inner=0.3)
    w.limit(4)
    w.write(body)
    return w


def _head_and_neck(w, X, in_sets, J):
    """The head is the `Head` bone's alone above a plane from under the
    chin to the nape (so a turn of the neck doesn't bend the face), and
    blends into the neck below it."""
    head = J["Head"][0]
    neck = J["Neck"][0]
    chin = J["Jaw"][1]
    idx = in_sets(HEAD_SETS)
    # The throat: the deepest point of the front's middle line between the
    # chin and the neck's base, where the underside of the jaw ends. The
    # nape: the back's middle line at the height of the head's joint.
    mid = X[idx][np.abs(X[idx][:, 0]) < 0.004]
    throat = mid[(mid[:, 1] < head[1]) & (mid[:, 2] < chin[2] - 0.01) & (mid[:, 2] > neck[2] + 0.02)]
    throat = throat[np.argmax(throat[:, 1])]
    back = mid[(mid[:, 1] > head[1]) & (np.abs(mid[:, 2] - head[2]) < 0.012)]
    front = np.array([0.0, throat[1], throat[2] - 0.006])
    nape = np.array([0.0, back[:, 1].max(), head[2] - 0.004])
    along = nape - front
    normal = np.array([0.0, -along[2], along[1]])
    normal /= np.linalg.norm(normal)
    d = (X[idx] - front) @ normal
    reach = max(0.03, 0.55 * (head[2] - neck[2]))
    h = _smoothstep(-reach, 0.0, d)
    rest = w.W[idx].copy()
    rest[:, w.index["Head"]] = 0.0
    # Under the plane, what isn't the head's is the neck's and the chest's.
    lower = rest.sum(axis=1)
    none = lower < 1e-6
    rest[none, w.index["Neck"]] = 1.0
    rest /= rest.sum(axis=1, keepdims=True)
    w.W[idx] = rest * (1.0 - h)[:, None]
    w.W[idx, w.index["Head"]] = h
    for name in ("LeftShoulder", "RightShoulder", "LeftUpperArm", "RightUpperArm"):
        # A shoulder doesn't reach above the neck's middle.
        w.W[idx, w.index[name]] *= 1.0 - _smoothstep(-reach, -0.4 * reach, d)
    total = w.W[idx].sum(axis=1, keepdims=True)
    w.W[idx] /= np.maximum(total, 1e-12)


# The jaw's hold on the face (`_jaw`).
# How much of the jaw's turn the corners of the mouth take.
JAW_CORNER = 0.30
# The lips' curve between the middle and a corner (`_jaw`): the power of
# the way along the lip the hold changes by. High, a lip is level across
# its middle and turns to the corner late (a broad U); it still meets
# the corner at a slope, so its end stays by the corner (an ellipse's
# upright end hung the lower lip's red a centimetre under it).
JAW_UPPER_CURVE = 2.5
JAW_LOWER_CURVE = 2.5
# The cheeks: from none of the turn this far above the line from the hinge
# to the mouth's corner to all of it this far under it (radians about the
# hinge, so the band is as wide as the jaw's travel is long).
JAW_BAND = (-0.20, 0.18)
# Round the mouth the lips' own rule holds inside this ring and has
# eased into the cheeks' by this one (elliptic coordinate about the
# corners: 0 is the lips' line, 1 an ellipse 85 mm wide and 64 mm high).
JAW_LIPS = (0.45, 1.0)
# Inside the mouth the skin's rule holds to this far from the lips' line
# and has given way by the second distance to the lining's own (metres):
# the roof with the head, the floor with the jaw.
JAW_LINING = (0.006, 0.016)


def _jaw(w, X, mesh, lm, in_sets, of_set, J, nbr):
    """The jaw's hold on the head: one smooth field over the skin and the
    mouth's lining alike, so a cheek and the lining behind it move
    together.

    - **The cheeks**, by the angle about the hinge under the line to the
      mouth's corner (`JAW_BAND`): a wide band, so the jaw's drop is
      spread over the whole cheek and not taken by one strip of skin.
    - **The lips.** The upper lip's middle stays with the head and the
      lower lip's middle goes with the jaw, though they touch; the
      corners, where they are one skin, take `JAW_CORNER`. Between, the
      hold follows elliptic coordinates about the two corners: on the
      lips' line, a fraction `u` of the way from the middle to a corner,
      the upper lip takes `JAW_CORNER * u ** JAW_UPPER_CURVE` and the
      lower `1 - (1 - JAW_CORNER) * u ** JAW_LOWER_CURVE`, so an open
      mouth is a shallow arch over a broad U and not a box. The same
      coordinates carry the rule off the lips smoothly, to none of the
      turn under the nose and all of it on the chin.
    - **Inside the mouth** the lips' inner sides go by the same rule as
      their outer sides, so a lip is moved whole. Deeper (`JAW_LINING`),
      where the lining lies on the gums, the upper half goes with the
      head and the lower with the jaw, as the teeth do, and the seam
      between the two along the inside of the cheek is shared."""
    from common import FS_MOUTH_UPPER
    hinge, _ = J["Jaw"]
    idx = in_sets(HEAD_SETS)
    edge = np.array(lm.ring(FS_MOUTH_UPPER, FS_HEAD), dtype=np.int64)
    edge = edge[np.argsort(X[edge][:, 0])]
    ends = X[edge[[0, -1]]]
    half = float(np.abs(ends[:, 0]).mean())
    corner = np.array([0.0, ends[:, 1].mean(), ends[:, 2].mean()])
    # The cheeks' rule: each vertex's angle about the hinge, under the
    # line to the mouth's corners.
    forward = -(X[:, 1] - hinge[1])
    up = X[:, 2] - hinge[2]
    line = np.arctan2(corner[2] - hinge[2], -(corner[1] - hinge[1]))
    under = line - np.arctan2(up, forward)
    cheeks = np.where(under < 0.0, JAW_CORNER * (1.0 - _smoothstep(0.0, -JAW_BAND[0], -under)),
                      JAW_CORNER + (1.0 - JAW_CORNER) * _smoothstep(0.0, JAW_BAND[1], under))
    # The lips' rule, in elliptic coordinates about the corners on the
    # face seen from the front, the lips' line made level.
    x = X[:, 0]
    inside = np.clip(x, -half, half)
    height = X[:, 2] - np.interp(inside, X[edge][:, 0], X[edge][:, 2])
    r_left, r_right = np.hypot(x - half, height), np.hypot(x + half, height)
    ring = np.arccosh(np.maximum((r_left + r_right) / (2.0 * half), 1.0))
    along = np.clip((r_right - r_left) / (2.0 * half), -1.0, 1.0)
    # Which lip: over or under the line; on it (the lips touch) by the
    # face set, the lower lip's being the muzzle's and the floor's.
    upper = height > 0.0
    of = np.zeros(len(X), dtype=np.int8)
    of[of_set[FS_HEAD]] = 1
    of[of_set[FS_MOUTH_UPPER]] = 1
    of[of_set[FS_MUZZLE]] = -1
    of[of_set[FS_MOUTH_LOWER]] = -1
    of[edge] = 1
    close = (np.abs(x) < half) & (np.abs(height) < 0.006) & (of != 0)
    upper[close] = of[close] > 0
    way = np.abs(along)
    lips = np.where(upper, JAW_CORNER * way ** JAW_UPPER_CURVE, 1.0 - (1.0 - JAW_CORNER) * way ** JAW_LOWER_CURVE)
    # How far the lips' rule reaches round the mouth.
    reach = 1.0 - _smoothstep(JAW_LIPS[0], JAW_LIPS[1], ring)
    field = cheeks + (lips - cheeks) * reach
    # Nothing at the hinge itself, nor behind it.
    field *= _smoothstep(0.015, 0.045, np.hypot(forward, up))
    field *= _smoothstep(hinge[1] + 0.004, hinge[1] - 0.02, X[:, 1])
    # The mouth's lining, away from the lips.
    upper_bag, lower_bag = of_set[FS_MOUTH_UPPER], of_set[FS_MOUTH_LOWER]
    own = np.zeros(len(X))
    own[lower_bag] = 1.0
    own[np.intersect1d(upper_bag, lower_bag)] = 0.5
    bag = np.union1d(upper_bag, lower_bag)
    lip_line = X[edge]
    away = np.full(len(X), np.inf)
    for k in range(len(lip_line) - 1):
        a, b = lip_line[k], lip_line[k + 1]
        t = np.clip(((X[bag] - a) @ (b - a)) / max(float((b - a) @ (b - a)), 1e-18), 0.0, 1.0)
        away[bag] = np.minimum(away[bag], np.linalg.norm(X[bag] - (a + t[:, None] * (b - a)), axis=1))
    deep = _smoothstep(JAW_LINING[0], JAW_LINING[1], away[bag])
    field[bag] += (own[bag] - field[bag]) * deep
    out = np.zeros(len(X))
    out[idx] = field[idx]
    # Only what the head holds turns with the jaw.
    held = w.col("Head")
    amount = np.clip(out, 0.0, 1.0) * held
    w.W[:, w.index["Head"]] = held - amount
    w.W[:, w.index["Jaw"]] = amount


def surface_distance(mesh, X, sources):
    """Each vertex's distance to the nearest of `sources` along the
    mesh's edges (metres), and which source that is."""
    import heapq
    nbr = [[] for _ in mesh.vertices]
    for e in mesh.edges:
        a, b = e.vertices
        length = float(np.linalg.norm(X[a] - X[b]))
        nbr[a].append((b, length))
        nbr[b].append((a, length))
    dist = np.full(len(X), np.inf)
    origin = np.full(len(X), -1, dtype=np.int64)
    heap = []
    for i in sources:
        dist[i] = 0.0
        origin[i] = i
        heap.append((0.0, i))
    heapq.heapify(heap)
    while heap:
        d, i = heapq.heappop(heap)
        if d > dist[i]:
            continue
        for j, length in nbr[i]:
            nd = d + length
            if nd < dist[j]:
                dist[j] = nd
                origin[j] = origin[i]
                heapq.heappush(heap, (nd, j))
    return dist, origin


def _shoulder(w, X, mesh, lm, of_set, side, s, torso=0.08, arm=0.06, top=1.5):
    """The arm's hold on the skin round the shoulder, by distance over the
    skin from the loop where the arm leaves the torso (the face sets'
    border, which runs over the shoulder's point and through the armpit):
    half on the loop, none `torso` metres onto the chest and back, all of
    it `arm` metres down the arm. Over the top of the shoulder the blend
    is `top` times as wide: the arm turns furthest there (raised over the
    head it turns the skin through more than a right angle), and a blend
    narrower than the turn folds the skin over itself. Bone heat alone
    lets the arm pull the chest and the side of the back with it."""
    ring = lm.ring(FS_TORSO, FS_UPPER_ARM[s])
    d, origin = surface_distance(mesh, X, ring)
    heights = X[ring][:, 2]
    low, high = heights.min(), heights.max()
    wide = 1.0 + (top - 1.0) * _smoothstep(low + 0.35 * (high - low), high, X[np.maximum(origin, 0)][:, 2])
    arm_sets = [FS_UPPER_ARM[s], FS_FOREARM[s], FS_PALM[s]] + [fs for sets in FS_FINGERS[s].values() for fs in sets]
    on_arm = np.zeros(len(X), dtype=bool)
    on_arm[np.unique(np.concatenate([of_set[fs] for fs in arm_sets]))] = True
    on_arm[ring] = False
    signed = np.where(on_arm, d, -d) / wide
    ramp = np.clip((signed + torso) / (torso + arm), 0.0, 1.0)
    # Nearer a straight ramp than a smooth step: its steepest part is what folds.
    share = 0.6 * ramp + 0.4 * _smoothstep(-torso, arm, signed)
    arm_bones = [k for k, name in enumerate(w.bones) if name.startswith(side) and any(
        part in name for part in ("UpperArm", "LowerArm", "Hand", *FINGERS))]
    # Everything on this side that isn't well down the arm: beyond the
    # blend the torso keeps none of the arm's weight.
    zone = np.nonzero((signed < arm + 0.3) & (X[:, 0] * s > -0.02))[0]
    upper = w.index[side + "UpperArm"]
    clavicle = w.index[side + "Shoulder"]
    for i in zone:
        row = w.W[i]
        held = row[arm_bones].sum()
        other = row.sum() - held
        a = share[i]
        if held > 1e-9:
            row[arm_bones] *= a / held
        else:
            row[upper] = a
        if other > 1e-9:
            keep = np.ones(len(row), dtype=bool)
            keep[arm_bones] = False
            row[keep] *= (1.0 - a) / other
        elif a < 1.0:
            row[clavicle] = 1.0 - a


def _hand(w, X, of_set, J, side, s):
    """Fingers from their face sets: each segment follows its own bone and
    blends into the next across the joint, with no weight from a
    neighbouring finger."""
    hand = w.index[side + "Hand"]
    palm = of_set[FS_PALM[s]]
    finger_bones = [w.index[f"{side}{f}{seg}"] for f in FINGERS for seg in SEGMENTS]
    lower = w.index[side + "LowerArm"]
    # The palm: whatever bone heat gave the fingers there goes to the hand
    # first; the knuckles are blended back in below.
    moved = w.W[np.ix_(palm, finger_bones)].sum(axis=1)
    w.W[np.ix_(palm, finger_bones)] = 0.0
    w.W[palm, hand] += moved
    for finger in FINGERS:
        sets = FS_FINGERS[s][finger]
        bones = [f"{side}{finger}{seg}" for seg in SEGMENTS]
        idx = np.unique(np.concatenate([of_set[fs] for fs in sets]))
        heads = [J[b][0] for b in bones]
        tails = [J[b][1] for b in bones]
        # Blend across each joint along the direction the finger runs there.
        widths = (0.011, 0.006, 0.005) if finger != "Thumb" else (0.016, 0.008, 0.006)
        blends = []
        for k in range(3):
            before = heads[k] - (J[side + "Hand"][0] if k == 0 else heads[k - 1])
            after = tails[k] - heads[k]
            direction = before / np.linalg.norm(before) + after / np.linalg.norm(after)
            direction /= np.linalg.norm(direction)
            blends.append((direction, widths[k]))
        b = [_smoothstep(-r, r, (X[idx] - heads[k]) @ d) for k, (d, r) in enumerate(blends)]
        row = np.zeros((len(idx), 4))
        row[:, 3] = b[2]
        row[:, 2] = b[1] * (1 - b[2])
        row[:, 1] = b[0] * (1 - b[1])
        row[:, 0] = 1 - b[0]
        w.W[idx] = 0.0
        w.W[idx, hand] = row[:, 0]
        for k, bone in enumerate(bones):
            w.W[idx, w.index[bone]] = row[:, k + 1]
        # The knuckle's skin on the palm bends with the finger's first bone.
        d, r = blends[0]
        axis = tails[0] - heads[0]
        axis /= np.linalg.norm(axis)
        rel = X[palm] - heads[0]
        along = rel @ d
        lateral = np.linalg.norm(rel - np.outer(rel @ axis, axis), axis=1)
        reach = 0.011 if finger != "Thumb" else 0.022
        share = _smoothstep(-r, r, along) * np.exp(-(lateral / reach) ** 2)
        if finger == "Thumb":
            share = np.maximum(share, _smoothstep(-0.012, 0.02, along) * np.exp(-(lateral / 0.02) ** 2))
        take = share * w.W[palm, hand]
        w.W[palm, hand] -= take
        w.W[palm, w.index[bones[0]]] += take
    # Nothing on the hand follows the upper arm.
    idx = np.unique(np.concatenate([palm] + [of_set[fs] for sets in FS_FINGERS[s].values() for fs in sets]))
    for name in (side + "UpperArm", side + "Shoulder"):
        w.W[idx, w.index[name]] = 0.0
    del lower


def _toes(w, X, of_set, in_sets, J, side, s):
    """The toes follow `Toes`, blending into the foot over the ball."""
    ball, tip = J[side + "Toes"]
    forward = (tip - ball) / np.linalg.norm(tip - ball)
    idx = in_sets([FS_FOOT[s], FS_TOE_BASE[s], *FS_TOES[s]])
    b = _smoothstep(-0.018, 0.012, (X[idx] - ball) @ forward)
    toes, foot = w.index[side + "Toes"], w.index[side + "Foot"]
    both = w.W[idx, toes] + w.W[idx, foot]
    w.W[idx, toes] = both * b
    w.W[idx, foot] = both * (1 - b)


def _twist(w, X, J, name, inside, share):
    """Moves part of bone `inside`'s weights to its twist bone, by where
    each vertex lies along it. With a negative share (the limb's own twist
    kept out) the twist bone holds the end nearer the body; otherwise the
    end nearer the hand or foot."""
    head, tail = J[inside]
    axis = tail - head
    length = np.linalg.norm(axis)
    t = np.clip(((X - head) @ axis) / (length * length), 0.0, 1.0)
    if share < 0:
        # The twist seen along the limb rises from the body's (none) to
        # the bone's own; the twist bone carries half.
        g = _smoothstep(0.05, 0.75, t)
        part = np.clip(2.0 * (1.0 - g), 0.0, 1.0)
    else:
        part = _smoothstep(0.15, 0.85, t)
    own = w.col(inside)
    take = own * part
    w.W[:, w.index[inside]] = own - take
    w.W[:, w.index[name]] = take


def _helper(w, X, J, name, parents, children, pivot, toward, inner, flat=0.0):
    """Gives the helper bone `name` the skin that the joint at `pivot`'s
    head shares between `parents` and `children`: a vertex held half by
    each goes to the helper whole, and less the more one side holds it
    (with `flat` above 0, whole over that much of the blend round its
    middle, so the helper carries a patch of skin of its own and the
    joint's turn is split into two smaller ones either side of it).
    `toward` is the way the joint folds (model space); skin on that side
    keeps `inner` of the helper's share, so the fold still creases."""
    P = sum(w.col(b) for b in parents)
    C = sum(w.col(b) for b in children)
    both = P + C
    mix = np.where(both > 1e-9, C / np.maximum(both, 1e-9), 0.0)
    tent = np.clip((1.0 - np.abs(2.0 * mix - 1.0)) / (1.0 - flat), 0.0, 1.0)
    amount = np.ones(len(X))
    head, tail = J[pivot]
    near = np.linalg.norm(X - head, axis=1) < 0.25
    if toward is not None:
        axis = (tail - head) / np.linalg.norm(tail - head)
        fold = toward - axis * (toward @ axis)
        fold /= np.linalg.norm(fold)
        rel = X - head
        radial = rel - np.outer(rel @ axis, axis)
        radial /= np.maximum(np.linalg.norm(radial, axis=1, keepdims=True), 1e-9)
        amount = 1.0 - (1.0 - inner) * _smoothstep(-0.2, 0.8, radial @ fold)
    share = np.where(near, tent * amount, 0.0)
    # What's left stays with the two sides so that the skin still turns
    # `mix` of the way where it can: the helper's half comes off each.
    p_new = np.maximum((1.0 - mix) - share / 2.0, 0.0)
    c_new = np.maximum(mix - share / 2.0, 0.0)
    left = p_new + c_new
    fit = np.where(left > 1e-9, (1.0 - share) / np.maximum(left, 1e-9), 0.0)
    p_new, c_new = p_new * fit * both, c_new * fit * both
    take = both - p_new - c_new
    for bones, old, new in ((parents, P, p_new), (children, C, c_new)):
        ratio = np.where(old > 1e-9, new / np.maximum(old, 1e-9), 0.0)
        for bone in bones:
            w.W[:, w.index[bone]] = w.col(bone) * ratio
    # A side that held nothing can't be scaled up: its first bone takes it.
    for bones, old, new in ((parents, P, p_new), (children, C, c_new)):
        missing = np.where(old > 1e-9, 0.0, new)
        w.W[:, w.index[bones[0]]] += missing
    w.W[:, w.index[name]] = take


def joint_offsets(joints, joints_other):
    """How far each bone's head moves from `joints` to `joints_other`
    (the feminine frame's skeleton), left out under a tenth of a
    millimetre."""
    out = {}
    for name, (head, _, _) in joints.items():
        d = joints_other[name][0] - head
        if d.length > 1e-4:
            out[name] = d
    return out


def limb_angles(joints):
    """The rest pose's angles, for the build's report: how far each arm
    points below level and how far each leg leans out."""
    out = {}
    for side, s in SIDES:
        a = joints[f"{side}UpperArm"][0]
        b = joints[f"{side}LowerArm"][0]
        c = joints[f"{side}Hand"][0]
        hip, ankle = joints[f"{side}UpperLeg"][0], joints[f"{side}Foot"][0]
        out[side] = {
            "upper_arm_below_level": round(math.degrees(math.asin(-(b - a).normalized().z)), 2),
            "forearm_below_level": round(math.degrees(math.asin(-(c - b).normalized().z)), 2),
            "leg_out": round(math.degrees(math.atan2((ankle.x - hip.x) * s, hip.z - ankle.z)), 2),
        }
    return out
