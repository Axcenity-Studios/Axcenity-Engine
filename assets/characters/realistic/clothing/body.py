"""The realistic body as a tailor measures it, for the drafts (FreeSewing's
measurements, in millimetres and degrees), and the landmarks the drape
places pieces by (metres, Blender's axes: Z up, the body facing -Y).

`load(glb, frame)` imports `RealisticParts.glb` and returns a `Body` with
the rest pose's skin (the masculine frame, or the feminine with
`Frame_Feminine` at full weight), its joints and these measurements:

| FreeSewing name | How it is taken here |
|---|---|
| neck | the convex outline of the neck 3 cm over its joint |
| chest | the fullest convex trunk outline between 3 and 14 cm under the armpit (the bust on the feminine frame) |
| waist | the narrowest trunk outline between the hips' joint + 8 cm and the chest |
| hips | the trunk outline a third of the way from the waist to the crotch (where trousers sit) |
| seat | the fullest outline round both legs between the crotch + 3 cm and the hips |
| biceps | the upper arm's outline across it, 40% of the way to the elbow |
| knee | one leg's outline at the knee joint |
| ankle | one leg's narrowest outline over the ankle joint |
| heel | 1.3 times the foot's length (the heel-and-instep girth isn't on a mesh's sections; 1.3 is FreeSewing's sample bodies' ratio) |
| shoulderToShoulder | across the back between the shoulder points (3 cm past where the top of the shoulder turns down into the arm, the deltoid's outer edge), along the back's outline |
| shoulderSlope | the shoulder line's fall from the high point (HPS) to the shoulder point |
| hpsToBust, hpsToWaistBack | from the high point down to the chest's height; down the back's middle to the waist |
| waistToArmpit, waistToHips, waistToSeat, waistToUpperLeg, waistToKnee, waistToFloor | heights |
| waist/seat front and back arcs | the outline's length in front of and behind its widest points |
| crossSeamFront, crossSeamBack | the middle section's outline from the waist at the front (back) down to the crotch |

A tape bridges hollows, so girths are convex outlines. Run alone to print
both frames' measurements (Blender's Python):

    python -I assets/characters/realistic/clothing/body.py --body assets/characters/realistic/RealisticParts.glb
"""

import argparse
import json
import math
import os

import bpy
import numpy as np

ARM_PARTS = ("UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little", "ShoulderHelper",
             "ElbowHelper")
LEG_PARTS = ("UpperLeg", "LowerLeg", "Foot", "Toes", "HipHelper", "KneeHelper")
# A slice of the body below the armpit is the trunk with an arm apart
# either side: the trunk reaches out to the first gap this wide.
GAP = 0.03
# The shoulder point is where the shoulder's top turns down into the arm
# more steeply than this (degrees): in the A-pose the deltoid's top falls
# at about the arm's 45 degrees, so a little past it.
SHOULDER_TURN = 52
ACROMION = 0.03


def hull_perimeter(points):
    """The convex outline's length of 2D points (monotone chain)."""
    pts = sorted(set(map(tuple, np.round(points, 6))))
    if len(pts) < 3:
        return 0.0

    def turn(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and turn(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and turn(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    hull = np.array(lower[:-1] + upper[:-1])
    return float(np.linalg.norm(np.roll(hull, -1, axis=0) - hull, axis=1).sum()), hull


def hull_of(points):
    return hull_perimeter(points)[1]


class Body:
    """The skin in the rest pose and what was measured on it."""

    def __init__(self, frame):
        self.frame = frame
        self.object = bpy.data.objects["Body"]
        self.armature = next(o for o in bpy.data.objects if o.type == "ARMATURE")
        me = self.object.data
        n = len(me.vertices)
        co = np.zeros(n * 3)
        if frame == "feminine":
            keys = me.shape_keys.key_blocks
            basis = np.zeros(n * 3)
            fem = np.zeros(n * 3)
            keys["Basis"].data.foreach_get("co", basis)
            keys["Frame_Feminine"].data.foreach_get("co", fem)
            co = fem
        else:
            me.vertices.foreach_get("co", co)
        world = np.array(self.object.matrix_world)
        self.P = co.reshape(n, 3) @ world[:3, :3].T + world[:3, 3]
        edges = np.zeros(len(me.edges) * 2, dtype=np.int64)
        me.edges.foreach_get("vertices", edges)
        self.edges = edges.reshape(-1, 2)
        tris = []
        for poly in me.polygons:
            v = list(poly.vertices)
            for k in range(1, len(v) - 1):
                tris.append((v[0], v[k], v[k + 1]))
        self.tris = np.array(tris, dtype=np.int64)
        groups = {g.index: g.name for g in self.object.vertex_groups}
        self.arm = np.zeros(n)
        self.leg_side = np.zeros(n)
        for v in me.vertices:
            for g in v.groups:
                name = groups[g.group]
                if any(part in name for part in ARM_PARTS):
                    self.arm[v.index] += g.weight
                if any(part in name for part in LEG_PARTS):
                    self.leg_side[v.index] += g.weight * (1 if name.startswith("Left") else -1)
        self.joints = {}
        for bone in self.armature.data.bones:
            self.joints[bone.name] = np.array(self.armature.matrix_world @ bone.head_local)
        if frame == "feminine":
            self._feminine_joints()

    def _feminine_joints(self):
        """The feminine frame's joints: each moved by the mean move of the
        skin weighted to it (the build's own offsets are in the manifest;
        this is enough for placing cloth)."""
        me = self.object.data
        groups = {g.index: g.name for g in self.object.vertex_groups}
        n = len(me.vertices)
        basis = np.zeros(n * 3)
        me.vertices.foreach_get("co", basis)
        world = np.array(self.object.matrix_world)
        B = basis.reshape(n, 3) @ world[:3, :3].T + world[:3, 3]
        move = self.P - B
        sums = {}
        for v in me.vertices:
            for g in v.groups:
                if g.weight > 0.5:
                    s = sums.setdefault(groups[g.group], [np.zeros(3), 0.0])
                    s[0] += move[v.index] * g.weight
                    s[1] += g.weight
        for name, (total, w) in sums.items():
            if name in self.joints and w > 0:
                self.joints[name] = self.joints[name] + total / w

    def section(self, z, keep=None):
        """Where the body's edges cross the height `z`."""
        a, b = self.P[self.edges[:, 0]], self.P[self.edges[:, 1]]
        crossing = ((a[:, 2] - z) * (b[:, 2] - z) <= 0) & (a[:, 2] != b[:, 2])
        if keep is not None:
            crossing &= keep[self.edges[:, 0]] & keep[self.edges[:, 1]]
        a, b = a[crossing], b[crossing]
        t = (z - a[:, 2]) / (b[:, 2] - a[:, 2])
        return a + (b - a) * t[:, None]

    def trunk(self, z, everything=False):
        """The trunk's outline at `z`: the middle out to the first gap
        (with `everything`, the arms' skin is looked at too)."""
        pts = self.section(z, None if everything else self.arm < 0.5)
        if len(pts) == 0:
            return pts
        xs = np.sort(np.abs(pts[:, 0]))
        gaps = np.nonzero(np.diff(xs) > GAP)[0]
        limit = xs[gaps[0]] + 1e-6 if len(gaps) else xs[-1] + 1
        return pts[np.abs(pts[:, 0]) <= limit]

    def girth(self, pts):
        return hull_perimeter(pts[:, :2])[0] if len(pts) >= 3 else 0.0

    def leg(self, z, side=1):
        pts = self.section(z)
        return pts[pts[:, 0] * side > 0.01]

    def normals(self):
        """The skin's vertex normals (area-weighted)."""
        P, T = self.P, self.tris
        face = np.cross(P[T[:, 1]] - P[T[:, 0]], P[T[:, 2]] - P[T[:, 0]])
        N = np.zeros_like(P)
        for k in range(3):
            np.add.at(N, T[:, k], face)
        return N / np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)

    def top_z(self, x, y_range=(-0.2, 0.2), width=0.004):
        """The highest skin at `x` (the top of the shoulder there), below
        the neck's joint (the head is not a shoulder)."""
        sel = (np.abs(self.P[:, 0] - x) < width) & (self.P[:, 1] > y_range[0]) & (self.P[:, 1] < y_range[1])
        sel &= (self.P[:, 2] > self.joints["UpperChest"][2]) & (self.P[:, 2] < self.joints["Neck"][2] + 0.01)
        return self.P[sel, 2].max() if sel.any() else None


def arcs(pts):
    """A girth's outline split at its widest points (left and right): the
    front arc (toward -Y) and the back arc, each the whole front or back
    (both sides)."""
    hull = hull_of(pts[:, :2])
    left = int(np.argmax(hull[:, 0]))
    right = int(np.argmin(hull[:, 0]))
    n = len(hull)
    walk = []
    k = left
    while k != right:
        walk.append(hull[k])
        k = (k + 1) % n
    walk.append(hull[right])
    a = np.array(walk)
    first = float(np.linalg.norm(np.diff(a, axis=0), axis=1).sum())
    total = hull_perimeter(pts[:, :2])[0]
    second = total - first
    # Which run is the front: the one with the lower mean y.
    mean_first = a[:, 1].mean()
    if mean_first < hull[:, 1].mean():
        return first, second
    return second, first


def measure(body):
    """FreeSewing's measurements (mm, degrees) and the landmarks (m)."""
    J = body.joints
    P = body.P
    mm = 1000.0
    # The armpit: the highest the arm is still apart from the trunk.
    armpit = J["LeftLowerArm"][2]
    while True:
        z = armpit + 0.005
        pts = body.section(z)
        if len(body.trunk(z, everything=True)) == len(pts):
            break
        armpit = z
    # Neck.
    neck_z = J["Neck"][2] + 0.03
    neck_pts = body.section(neck_z)
    neck_pts = neck_pts[np.hypot(neck_pts[:, 0], neck_pts[:, 1] - J["Neck"][1]) < 0.09]
    neck = body.girth(neck_pts)
    neck_half = np.abs(neck_pts[:, 0]).max()
    # The high point of the shoulder (HPS): on the shoulder's top just past
    # the neck's side.
    hps_x = neck_half + 0.008
    hps_z = body.top_z(hps_x, (J["Neck"][1] - 0.06, J["Neck"][1] + 0.06))
    # The shoulder point: walking out along the shoulder's top, where it
    # turns down more steeply than 40 degrees into the arm.
    x = hps_x
    z_prev = hps_z
    shoulder_x, shoulder_z = x, hps_z
    step = 0.004
    while x < J["LeftUpperArm"][0] + 0.08:
        x += step
        z = body.top_z(x, (J["Neck"][1] - 0.1, J["Neck"][1] + 0.1))
        if z is None:
            break
        if (z_prev - z) / step > math.tan(math.radians(SHOULDER_TURN)) and x > J["LeftUpperArm"][0]:
            break
        shoulder_x, shoulder_z = x, z
        z_prev = z
    # The A-pose's deltoid falls from right over the joint, where the
    # walk stops; a tape's shoulder point is the acromion's outer edge,
    # `ACROMION` past it (with the arm down the top would run on to it).
    shoulder_x += ACROMION
    # Across the back, as a tape lies over it: the back's outline a little
    # under the shoulder points' height, from one to the other.
    band = P[(np.abs(P[:, 2] - (shoulder_z - 0.02)) < 0.004) & (P[:, 1] > J["Neck"][1]) & (np.abs(P[:, 0]) <= shoulder_x)]
    across = []
    for x0 in np.arange(-shoulder_x, shoulder_x + 1e-9, 0.01):
        near = band[np.abs(band[:, 0] - x0) < 0.006]
        if len(near):
            across.append([x0, near[:, 1].max()])
    across = np.array(across)
    s2s = max(2 * shoulder_x, float(np.linalg.norm(np.diff(across, axis=0), axis=1).sum()))
    slope = math.degrees(math.atan2(hps_z - shoulder_z, shoulder_x - hps_x))
    # Chest: the fullest trunk girth from 3 to 14 cm below the armpit.
    chest_z, chest = max(((z, body.girth(body.trunk(z))) for z in np.arange(armpit - 0.14, armpit - 0.03, 0.005)),
                         key=lambda zg: zg[1])
    # Waist: the narrowest girth between the hips' joint + 8 cm and the chest.
    waist_z, waist = min(((z, body.girth(body.trunk(z))) for z in np.arange(J["Hips"][2] + 0.08, chest_z - 0.08, 0.005)),
                         key=lambda zg: zg[1])
    # Crotch: the highest height where the legs are apart (the middle
    # section has no skin between them).
    # The crotch: the lowest skin in the middle that faces down (thighs
    # that touch there face sideways).
    N = body.normals()
    under = (np.abs(P[:, 0]) < 0.008) & (N[:, 2] < -0.6) & (P[:, 2] < J["Hips"][2]) & (P[:, 2] > J["LeftLowerLeg"][2])
    crotch_z = float(P[under, 2].min())
    hips_z = waist_z - (waist_z - crotch_z) / 3
    hips = body.girth(body.trunk(hips_z))
    seat_z, seat = max(((z, body.girth(body.section(z, body.arm < 0.2))) for z in np.arange(crotch_z + 0.03, hips_z, 0.005)),
                       key=lambda zg: zg[1])
    waist_front, waist_back = arcs(body.trunk(waist_z))
    seat_front, seat_back = arcs(body.section(seat_z, body.arm < 0.2))
    # Arms and legs.
    shoulder, elbow = J["LeftUpperArm"], J["LeftLowerArm"]
    axis = (elbow - shoulder) / np.linalg.norm(elbow - shoulder)
    at = shoulder + 0.4 * (elbow - shoulder)
    near = P[(np.abs((P - at) @ axis) < 0.004) & (np.linalg.norm(P - at, axis=1) < 0.08)]
    u = np.cross(axis, [0, 1, 0])
    u /= np.linalg.norm(u)
    v = np.cross(axis, u)
    biceps = hull_perimeter(np.stack([(near - at) @ u, (near - at) @ v], axis=1))[0]
    knee = body.girth(body.leg(J["LeftLowerLeg"][2]))
    ankle = min(body.girth(body.leg(z)) for z in np.arange(J["LeftFoot"][2] + 0.04, J["LeftFoot"][2] + 0.14, 0.005))
    foot = P[(P[:, 0] > 0.02) & (P[:, 2] < 0.04)]
    foot_length = foot[:, 1].max() - foot[:, 1].min()
    # The back's middle from the HPS's height to the waist.
    back = P[(np.abs(P[:, 0]) < 0.012) & (P[:, 2] < hps_z) & (P[:, 2] > waist_z) & (P[:, 1] > J["Spine"][1])]
    back = back[np.argsort(-back[:, 2])]
    # Keep the outermost (largest y) per 1 cm band, walked top to bottom.
    profile = []
    for z in np.arange(hps_z, waist_z, -0.01):
        band = back[(back[:, 2] <= z) & (back[:, 2] > z - 0.01)]
        if len(band):
            profile.append([band[:, 1].max(), z])
    profile = np.array(profile)
    # Down the back's curve, and never shorter than the drop itself (the
    # profile's first band starts under the neck's base).
    hps_to_waist_back = max(float(np.linalg.norm(np.diff(profile, axis=0), axis=1).sum()), hps_z - waist_z)
    # The cross seam: the middle section (|x| < 1 cm) from the waist round
    # under the crotch.
    mid = P[(np.abs(P[:, 0]) < 0.01) & (P[:, 2] < waist_z + 0.005) & (P[:, 2] > crotch_z - 0.03)]
    centre_y = (mid[:, 1].min() + mid[:, 1].max()) / 2

    def seam(front):
        side = mid[(mid[:, 1] < centre_y) if front else (mid[:, 1] >= centre_y)]
        path = []
        for z in np.arange(waist_z, crotch_z - 0.002, -0.01):
            band = side[(side[:, 2] <= z) & (side[:, 2] > z - 0.01)]
            if len(band):
                path.append([band[:, 1].min() if front else band[:, 1].max(), z])
        path.append([centre_y, crotch_z])
        path = np.array(path)
        return float(np.linalg.norm(np.diff(path, axis=0), axis=1).sum())

    m = {
        "neck": neck * mm,
        "chest": chest * mm,
        "waist": waist * mm,
        "hips": hips * mm,
        "seat": seat * mm,
        "biceps": biceps * mm,
        "knee": knee * mm,
        "ankle": ankle * mm,
        "heel": 1.3 * foot_length * mm,
        "shoulderToShoulder": s2s * mm,
        "shoulderSlope": slope,
        "hpsToBust": (hps_z - chest_z) * mm,
        "hpsToWaistBack": hps_to_waist_back * mm,
        "waistToArmpit": (armpit - waist_z) * mm,
        "waistToHips": (waist_z - hips_z) * mm,
        "waistToSeat": (waist_z - seat_z) * mm,
        "waistToUpperLeg": (waist_z - crotch_z) * mm,
        "waistToKnee": (waist_z - J["LeftLowerLeg"][2]) * mm,
        "waistToFloor": waist_z * mm,
        "waistFrontArc": waist_front / 2 * mm,
        "waistBackArc": waist_back / 2 * mm,
        "seatFrontArc": seat_front / 2 * mm,
        "seatBackArc": seat_back / 2 * mm,
        "crossSeamFront": seam(True) * mm,
        "crossSeamBack": seam(False) * mm,
    }
    landmarks = {
        "armpit_z": float(armpit), "neck_z": float(neck_z), "neck_half": float(neck_half), "neck_y": float(J["Neck"][1]),
        "hps": [float(hps_x), float(J["Neck"][1]), float(hps_z)],
        "shoulder": [float(shoulder_x), float(J["Neck"][1]), float(shoulder_z)],
        "chest_z": float(chest_z), "waist_z": float(waist_z), "hips_z": float(hips_z), "seat_z": float(seat_z),
        "crotch_z": float(crotch_z), "centre_y": float(centre_y), "foot_length": float(foot_length),
    }
    m = {k: float(v) for k, v in m.items()}
    return m, landmarks


def load(glb, frame):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=os.path.abspath(glb))
    body = Body(frame)
    body.measurements, body.landmarks = measure(body)
    return body


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--body", required=True)
    args = parser.parse_args()
    out = {}
    for frame in ("masculine", "feminine"):
        body = load(args.body, frame)
        out[frame] = {"measurements": {k: round(v, 1) for k, v in body.measurements.items()},
                      "landmarks": body.landmarks}
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
