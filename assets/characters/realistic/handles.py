"""The realistic body's handles (Character Studio refinement R2, brief
§3.4, §4.3 and §5.3): the base manifest's `handles`, written by
`manifests.py`. Plain Python with numpy, like `anchors.py`.

A handle is a point on the skin that a person drags in the viewport. It
holds no geometry of its own: it names parameters (`drives`) and moves
them as their sliders would (plan section 11.3). The first seven features
came in the plan's order: nose, eyes, jaw, mouth (the `lips` feature),
shoulders, waist and hips. Since 2026-11-24 the face has more: a ring of
handles round its outline (the `face_shape` feature: the forehead's top,
the temples, the cheekbones' outer edges, the cheeks, the jaw's corners,
the jawline, the chin's corners and the chin), which drive the outline's
own parameters and those of the features the outline passes (the
forehead's height, the temples' width, the jaw's angle, the chin's width),
more on the nose (its root, the hump, the bridge's and the tip's sides, the
columella), and handles for the cheeks, the chin, the jawline, the
forehead and the skin round the eyes. A handle's features are kept
together in the list, in the order the studio lists the features.

Everything is measured on the built model, on the masculine frame (the
model's own rest shape), with every other parameter at its default:

- **Where it sits.** A corner of the body's skin, chosen by a rule per
  handle (`HANDLES`): the corner its first parameter moves furthest (a
  mouth corner, a jaw angle), the nose's most forward corner, the waist's
  narrowest side, the hips' widest. Its `anchor` is that corner as an
  offset from the head of the bone that carries most of it (model space at
  rest, metres, glTF axes: Y up, +Z front, the character's left is +X), so
  the studio finds the same corner again and draws the handle on it, as
  the skin is posed and shaped.
- **What a drag does.** Each drive's `axis` and `travel` are how that
  corner moves from the parameter's lowest value to its highest: the
  direction, and the distance in metres. Shapes are added at their
  weights (corners included); the proportion sliders' bone changes are
  worked out as `anim`'s body sliders make them (`body_shape.rs`: a length
  scales a bone's own corners along its Y and moves its children out, a
  width the same along X), and the face's bone drivers move their bones.
  So the studio can turn a drag of one centimetre on the screen into the
  change that moves the surface one centimetre (`R2` test in `player`
  holds the measured travel to the engine's).
- **Where a drive works.** `screen` gives, for each of the studio's
  framings (`front`, `three_quarter`, `side` and `back`, the camera as its
  view buttons put it), the direction the corner moves on the screen (x
  right, y up), only where at least `LIVE` of the travel shows; a drag
  there means what the view shows (ux patterns P5). A handle's `views`
  are the framings where any of its drives works.
- **Pairs.** A left handle and its right twin name each other in
  `mirror`; the right one is the corner mirrored across the middle.
"""

import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import anchors  # noqa: E402 (beside this file)

# How far (metres) a drive's move on the feminine frame may differ from the
# masculine one's before the drive lists it on its own (`frames`).
FRAME_APART = 0.001

# The share of a drive's travel that must show across the screen in a
# framing for the drive to work there.
LIVE = 0.35
# Two drives of one handle whose directions on the screen are closer than
# this (either way along a line) can't both follow one drag: the one that
# shows less of its travel there gives way.
APART = 25.0

# The studio's framings (`stage.rs`): the camera's turn round the
# character from its front, in degrees. A side view is the character's
# right profile, as the Side button frames it.
VIEWS = {"front": 0.0, "three_quarter": 45.0, "side": 90.0, "back": 180.0}

# The regions that make the trunk's outline, for the waist's and the hips'
# handles.
TRUNK = ["torso", "waist", "hips", "thighs_top"]

# Where the top of the forehead is from the head's joint (model space,
# glTF axes): up and forward, about 54 degrees above level.
FOREHEAD_TOP = [0.0, 0.81, 0.59]

# The jawline's handle: half-way from the jaw's corner to the chin's.
JAWLINE = ("between", ("moved", "jaw_width_{side}"), ("moved", "chin_shape"), 0.5)

# The handles, by feature in the plan's order. `{side}` and `{Side}` make
# a pair (`left`/`Left` first). `yields` names drives that take a framing
# only when no other drive of the handle has their line there. `at` is how
# its corner is found:
#   ("moved", parameter, region?): the corner the parameter moves furthest
#       over its range (in the region, by its ID, when one is given);
#   ("forward", shape): of the corners the shape moves, the most forward;
#   ("middle", parameter): the corner the parameter moves furthest on the
#       middle line (a lip);
#   ("top", parameter, region): of the corners the parameter moves almost
#       as far as the furthest (a whole limb moves alike), the highest;
#   ("lid", bone, direction): of the skin's corners within 2 cm of the
#       bone's head that face away from it, the one most nearly in
#       `direction` from it (the upper lid over the pupil, from the
#       eyeball's centre);
#   ("canthus", bone): the eye's outer corner. Of the skin's corners
#       within 2 cm of the eyeball's centre and within 4 mm of its height,
#       those that face into the eye are the lid's margin turning into the
#       opening; the most lateral of them is where the lids meet. The
#       handle sits on the outward-facing skin corner nearest it (within
#       3 mm), so it shows from the front;
#   ("narrowest", region) / ("widest", region): the side of the region
#       where the trunk is narrowest or widest (in slices), the
#       outline's corner half-way front to back;
#   ("outline", parameter): the face's outline, seen from the front, at
#       the height of the corner the parameter moves furthest: of the
#       skin's corners within 5 mm of that height, off the ear and in
#       front of its middle, the furthest out to the side, and of those
#       within 4 mm of as far out, the most forward (the outline's corner
#       that faces the front most);
#   ("edge", parameter, share): the outer edge of what the parameter
#       moves: of the corners it moves at least `share` of its furthest,
#       the one furthest out to the side (the cheekbone's outer edge);
#   ("between", rule, rule, share): the skin's corner nearest the point
#       `share` of the way from one rule's corner to another's (the
#       jawline, between the jaw's corner and the chin's);
#   ("toward", bone, direction): of the skin's corners on the middle line,
#       the one most nearly in `direction` from the bone's head (the top
#       of the forehead, from the head's joint).
HANDLES = [
    # The ring round the face's outline, from the top down.
    {"id": "outline_forehead", "feature": "face_shape", "label": "Forehead top",
     "at": ("toward", "Head", FOREHEAD_TOP), "drives": ["forehead_height", "forehead_slope"]},
    {"id": "outline_temple_{side}", "feature": "face_shape", "label": "{Side} temple",
     "at": ("outline", "temple_width"), "drives": ["temple_width"]},
    {"id": "outline_cheekbone_{side}", "feature": "face_shape", "label": "{Side} cheekbone edge",
     "at": ("edge", "cheekbone_{side}", 0.4), "drives": ["face_width", "cheekbone_height_{side}"]},
    {"id": "outline_cheek_{side}", "feature": "face_shape", "label": "{Side} cheek",
     "at": ("outline", "cheek_fullness_{side}"), "drives": ["face_roundness"]},
    {"id": "outline_jaw_{side}", "feature": "face_shape", "label": "{Side} jaw corner",
     "at": ("moved", "jaw_width_{side}"), "drives": ["face_taper", "jaw_angle_{side}"]},
    {"id": "outline_jawline_{side}", "feature": "face_shape", "label": "{Side} jawline",
     "at": JAWLINE, "drives": ["face_narrow", "jaw_definition"]},
    {"id": "outline_chin_{side}", "feature": "face_shape", "label": "{Side} chin corner",
     "at": ("moved", "chin_shape"), "drives": ["chin_width", "chin_shape"]},
    {"id": "outline_chin", "feature": "face_shape", "label": "Chin",
     "at": ("middle", "chin_length"), "drives": ["face_length", "chin_forward"]},
    {"id": "nose_tip", "feature": "nose", "label": "Nose tip",
     "at": ("forward", "Face_Nose_Length"), "drives": ["nose_length", "nose_tip_projection"]},
    {"id": "nose_bridge", "feature": "nose", "label": "Bridge",
     "at": ("middle", "nose_bridge"), "drives": ["nose_bridge"]},
    {"id": "nostril_{side}", "feature": "nose", "label": "{Side} nostril",
     "at": ("moved", "nostril_width_{side}"),
     "drives": ["nostril_width_{side}", "nostril_height_{side}", "nostril_flare_{side}"],
     # (The flare runs nearly along the width's line: it takes only the
     # framings the width leaves, the profile.)
     "yields": ["nostril_flare_{side}"]},
    {"id": "nose_root", "feature": "nose", "label": "Bridge root",
     "at": ("middle", "nose_root"), "drives": ["nose_root"]},
    {"id": "nose_hump", "feature": "nose", "label": "Bridge hump",
     "at": ("middle", "nose_hump"), "drives": ["nose_hump"]},
    {"id": "nose_bridge_{side}", "feature": "nose", "label": "{Side} of the bridge",
     "at": ("moved", "nose_bridge_width"), "drives": ["nose_bridge_width"]},
    {"id": "nose_tip_{side}", "feature": "nose", "label": "{Side} of the tip",
     "at": ("moved", "nose_tip_width"), "drives": ["nose_tip_width"]},
    {"id": "nose_columella", "feature": "nose", "label": "Columella",
     "at": ("middle", "nose_columella"), "drives": ["nose_columella"]},
    {"id": "eye_{side}", "feature": "eyes", "label": "{Side} eye",
     "at": ("lid", "{Side}Eye", [0.0, 0.6, 0.8]),
     "drives": ["eye_spacing_{side}", "eye_height_{side}", "eye_depth_{side}"]},
    {"id": "eye_corner_{side}", "feature": "eyes", "label": "{Side} eye corner",
     "at": ("canthus", "{Side}Eye"), "drives": ["eye_size_{side}", "eye_tilt_{side}"]},
    {"id": "under_eye_{side}", "feature": "eyes", "label": "Under the {side} eye",
     "at": ("moved", "under_eye_{side}"), "drives": ["under_eye_{side}"]},
    {"id": "eye_inner_{side}", "feature": "eyes", "label": "{Side} inner corner",
     "at": ("moved", "inner_corner_fold_{side}"), "drives": ["inner_corner_fold_{side}"]},
    {"id": "cheekbone_{side}", "feature": "cheeks", "label": "{Side} cheekbone",
     "at": ("moved", "cheekbone_{side}"), "drives": ["cheekbone_{side}", "cheekbone_height_{side}"]},
    {"id": "cheek_{side}", "feature": "cheeks", "label": "{Side} cheek",
     "at": ("moved", "cheek_fullness_{side}"), "drives": ["cheek_fullness_{side}"]},
    {"id": "cheek_hollow_{side}", "feature": "cheeks", "label": "{Side} cheek hollow",
     "at": ("moved", "cheek_hollow_{side}"), "drives": ["cheek_hollow_{side}"]},
    {"id": "jaw_{side}", "feature": "jaw", "label": "{Side} jaw corner",
     "at": ("moved", "jaw_width_{side}"), "drives": ["jaw_width_{side}", "jaw_angle_{side}"]},
    {"id": "jawline_{side}", "feature": "jaw", "label": "{Side} jawline",
     "at": JAWLINE, "drives": ["jaw_definition"]},
    {"id": "chin", "feature": "chin", "label": "Chin",
     "at": ("middle", "chin_length"), "drives": ["chin_length", "chin_forward"]},
    {"id": "chin_{side}", "feature": "chin", "label": "{Side} chin corner",
     "at": ("moved", "chin_shape"), "drives": ["chin_width", "chin_shape"]},
    {"id": "mouth_corner_{side}", "feature": "lips", "label": "{Side} mouth corner",
     "at": ("moved", "mouth_width_{side}"), "drives": ["mouth_width_{side}", "mouth_height"]},
    {"id": "upper_lip", "feature": "lips", "label": "Upper lip",
     "at": ("middle", "upper_lip_fullness"), "drives": ["upper_lip_fullness"]},
    {"id": "lower_lip", "feature": "lips", "label": "Lower lip",
     "at": ("middle", "lower_lip_fullness"), "drives": ["lower_lip_fullness"]},
    {"id": "forehead", "feature": "forehead", "label": "Forehead",
     "at": ("toward", "Head", FOREHEAD_TOP), "drives": ["forehead_height", "forehead_slope"]},
    {"id": "temple_{side}", "feature": "forehead", "label": "{Side} temple",
     "at": ("outline", "temple_width"), "drives": ["temple_width"]},
    {"id": "shoulder_{side}", "feature": "shoulders", "label": "{Side} shoulder",
     "at": ("top", "shoulder_width", "upper_arms_top"), "drives": ["shoulder_width"]},
    {"id": "waist_{side}", "feature": "waist", "label": "{Side} waist",
     "at": ("narrowest", "waist"), "drives": ["waist_width"]},
    {"id": "hip_{side}", "feature": "hips", "label": "{Side} hip",
     "at": ("widest", "hips"), "drives": ["hip_width"]},
]


def span(value, start, end=None):
    """`resolve::span`: how far `value` is across `start`, clamped, then
    across `end` (which is `start` when not given)."""
    end = end or start
    length = start[1] - start[0]
    t = 0.0 if length == 0 else min(max((value - start[0]) / length, 0.0), 1.0)
    return end[0] + (end[1] - end[0]) * t


class Rig:
    """The body's bones at rest: each joint's bind (bone to model space),
    its parent, and its head."""

    def __init__(self, path, body):
        glb = anchors.Glb(path)
        node = glb.node("Body")
        skin = glb.gltf["skins"][node["skin"]]
        joints = skin["joints"]
        parent_node = {}
        for index, n in enumerate(glb.gltf["nodes"]):
            for child in n.get("children", []):
                parent_node[child] = index
        ibm = glb.accessor(skin["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
        self.names = body.bones
        self.bind = np.array([np.linalg.inv(m) for m in ibm])
        self.inverse_bind = ibm
        slot = {n: i for i, n in enumerate(joints)}
        self.parent = [slot.get(parent_node.get(n)) for n in joints]
        self.index = {name: i for i, name in enumerate(self.names)}

    def children(self, bone):
        return [j for j, p in enumerate(self.parent) if p == bone]

    def below(self, bone):
        """`bone` and every bone under it."""
        out, stack = [], [bone]
        while stack:
            b = stack.pop()
            out.append(b)
            stack.extend(self.children(b))
        return out

    def local(self, child):
        """The child's head in its parent's space at rest."""
        return (self.inverse_bind[self.parent[child]] @ self.bind[child])[:3, 3]


class Model:
    """The body at a parameter's value, the others at their defaults, on
    the masculine frame."""

    def __init__(self, path, manifest):
        self.body = anchors.Body(path)
        self.rig = Rig(path, self.body)
        # Which corners are the skin's (its material is `Skin`), and each
        # corner's normal, in `anchors.Body`'s order (the primitives one
        # after another).
        glb = anchors.Glb(path)
        mesh = glb.gltf["meshes"][glb.node("Body")["mesh"]]
        skin, normals = [], []
        for primitive in mesh["primitives"]:
            count = glb.gltf["accessors"][primitive["attributes"]["POSITION"]]["count"]
            material = glb.gltf["materials"][primitive["material"]]["name"]
            skin.append(np.full(count, material == "Skin"))
            normals.append(glb.accessor(primitive["attributes"]["NORMAL"]))
        self.skin = np.concatenate(skin)
        self.normals = np.concatenate(normals)
        self.manifest = manifest
        self.parameters = {p["id"]: p for p in manifest["parameters"]}

    def corners_at(self, pid, value, frame=0.0):
        """Every body corner with parameter `pid` at `value`, on `frame`
        (its shapes at that value; the frame's bone offsets move every
        corner alike at both ends, so they are left out)."""
        body, rig = self.body, self.rig
        values = {p["id"]: p.get("default", 0.0) for p in self.manifest["parameters"]}
        values["frame"] = frame
        values[pid] = value
        # The shapes: the frame's, each parameter's own, then the corners.
        P = body.P.copy()
        if frame and pid != "frame" and "frame" in self.parameters:
            for driver in self.parameters["frame"].get("shapes", []):
                weight = span(frame, driver.get("from", [0, 1]), driver.get("to"))
                P += weight * body.shapes[driver["shape"]]
        parameter = self.parameters[pid]
        for driver in parameter.get("shapes", []):
            weight = span(value, driver.get("from", [0, 1]), driver.get("to"))
            P += weight * body.shapes[driver["shape"]]
        for corner in self.manifest.get("corners", []):
            if corner["shape"] not in body.shapes:
                continue
            factors = corner["factors"]
            if not any(f["parameter"] == pid for f in factors):
                continue
            weight = 1.0
            for f in factors:
                weight *= span(values[f["parameter"]], f.get("from", [0, 1]), f.get("to"))
            default = 1.0
            for f in factors:
                at = values[f["parameter"]] if f["parameter"] != pid else parameter.get("default", 0.0)
                default *= span(at, f.get("from", [0, 1]), f.get("to"))
            P += (weight - default) * body.shapes[corner["shape"]]
        # The bones: each joint's matrix from rest to this value.
        count = len(rig.names)
        scale = np.tile(np.eye(4), (count, 1, 1))
        move = np.zeros((count, 3))
        for driver in parameter.get("attributes", []):
            factor = span(value, driver.get("from", [0, 1]), driver.get("to"))
            for bone, axis, f in body_slider(driver["attribute"], factor):
                b = rig.index[bone]
                scale[b][axis, axis] *= f
                for child in rig.children(b):
                    # The child's head moves out along its parent's axis,
                    # and everything under it with it.
                    shift = rig.bind[b][:3, axis] * (f - 1.0) * rig.local(child)[axis]
                    for under in rig.below(child):
                        move[under] += shift
        for driver in parameter.get("bones", []):
            weight = span(value, driver.get("from", [0, 1]), driver.get("to"))
            b = rig.index[driver["bone"]]
            for under in rig.below(b):
                move[under] += weight * np.array(driver.get("offset", [0, 0, 0]))
                if "scale" in driver:
                    grow = 1 + (driver["scale"] - 1) * weight
                    scale[under][:3, :3] *= grow
        matrices = []
        for j in range(count):
            m = rig.bind[j] @ scale[j] @ rig.inverse_bind[j]
            m[:3, 3] += move[j]
            matrices.append(m)
        matrices = np.array(matrices)
        homogeneous = np.concatenate([P, np.ones((len(P), 1))], axis=1)
        out = np.zeros_like(P)
        for k in range(4):
            moved = np.einsum("nij,nj->ni", matrices[body.joints[:, k]], homogeneous)[:, :3]
            out += body.weights[:, k:k + 1] * moved
        return out

    def ends(self, pid, frame=0.0):
        low, high = self.parameters[pid]["range"]
        return self.corners_at(pid, low, frame), self.corners_at(pid, high, frame)


def body_slider(attribute, factor):
    """`anim::BodyShape::apply` for one slider: (bone, axis, factor), axis
    0 for X (a width) and 1 for Y (a length)."""
    if attribute == "ShoulderWidth":
        return [("LeftShoulder", 1, factor), ("RightShoulder", 1, factor),
                ("UpperChest", 0, factor ** 0.5)]
    if attribute == "WaistWidth":
        return [("Spine", 0, factor)]
    if attribute == "HipWidth":
        return [("Hips", 0, factor)]
    raise ValueError(f"no handle drives {attribute}")


def sided(text, side):
    if side is None:
        return text
    return text.replace("{side}", side.lower()).replace("{Side}", side)


def screen_direction(axis, view):
    """Where `axis` (model space) goes on the screen in framing `view`:
    the studio's camera turned `VIEWS[view]` degrees round from the front.
    In front the character's left (+X) is to the right; from its right
    side (90) its front (+Z) is."""
    turn = np.radians(VIEWS[view])
    right = np.array([np.cos(turn), 0.0, np.sin(turn)])
    return np.array([axis @ right, axis[1]])


def choose(model, at, side, ends):
    """The corner a handle sits on, by its rule."""
    body = model.body
    P = body.P
    on_side = np.ones(len(P), bool) if side is None else (P[:, 0] > 0.002) == (side == "Left")
    # Only the skin's corners: the inside of the mouth and of the eye
    # sockets shares the skin's positions at their seams, facing in.
    on_side &= model.skin
    regions = {r["id"]: r.get("map_index") for r in model.manifest["regions"]}
    kind = at[0]
    if kind in ("moved", "middle", "top"):
        low, high = ends(sided(at[1], side))
        moved = np.linalg.norm(high - low, axis=1)
        allowed = on_side.copy()
        if len(at) > 2:
            allowed &= body.region == regions[at[2]]
        if kind == "middle":
            allowed &= np.abs(P[:, 0]) < 0.004
        if kind == "top":
            far = np.where(allowed, moved, -1.0).max()
            allowed &= moved >= 0.9 * far
            return int(np.argmax(np.where(allowed, P[:, 1], -np.inf)))
        # Only the front half of the skin a person sees: corners facing
        # back (inside the mouth, behind a nostril) are left out by the
        # forward rule below where it matters.
        return int(np.argmax(np.where(allowed, moved, -1.0)))
    if kind == "lid":
        offset = P - body.heads[sided(at[1], side)]
        reach = np.linalg.norm(offset, axis=1)
        out = offset / np.maximum(reach, 1e-9)[:, None]
        facing = np.einsum("ij,ij->i", model.normals, out) > 0.5
        want = np.array(at[2]) / np.linalg.norm(at[2])
        allowed = model.skin & (reach < 0.02) & facing
        return int(np.argmax(np.where(allowed, out @ want, -2.0)))
    if kind == "canthus":
        offset = P - body.heads[sided(at[1], side)]
        reach = np.linalg.norm(offset, axis=1)
        facing = np.einsum("ij,ij->i", model.normals, offset / np.maximum(reach, 1e-9)[:, None])
        lateral = offset[:, 0] * (1.0 if side == "Left" else -1.0)
        band = model.skin & (reach < 0.02) & (np.abs(offset[:, 1]) < 0.004) & (lateral > 0)
        margin = band & (facing < 0.0)
        if not margin.any():
            raise ValueError(f"no lid margin round {sided(at[1], side)}")
        rim = int(np.argmax(np.where(margin, lateral, -np.inf)))
        apart = np.linalg.norm(P - P[rim], axis=1)
        outside = model.skin & (facing > 0.5) & (apart < 0.003)
        if not outside.any():
            raise ValueError(f"no skin facing out by the corner of {sided(at[1], side)}")
        return int(np.argmin(np.where(outside, apart, np.inf)))
    if kind == "outline":
        low, high = ends(sided(at[1], side))
        moved = np.linalg.norm(high - low, axis=1)
        height = P[int(np.argmax(np.where(on_side, moved, -1.0))), 1]
        ear = np.linalg.norm(body.shapes[f"Face_Ear_Out_{side}"], axis=1) > 1e-5
        allowed = on_side & ~ear & (np.abs(P[:, 1] - height) < 0.005) & (P[:, 2] > P[ear, 2].mean())
        out = np.where(allowed, np.abs(P[:, 0]), -1.0)
        return int(np.argmax(np.where(out >= out.max() - 0.004, P[:, 2], -np.inf)))
    if kind == "edge":
        low, high = ends(sided(at[1], side))
        moved = np.where(on_side, np.linalg.norm(high - low, axis=1), -1.0)
        return int(np.argmax(np.where(moved >= at[2] * moved.max(), np.abs(P[:, 0]), -1.0)))
    if kind == "between":
        a, b = (P[choose(model, rule, side, ends)] for rule in at[1:3])
        distance = np.linalg.norm(P - (a + (b - a) * at[3]), axis=1)
        return int(np.argmin(np.where(on_side, distance, np.inf)))
    if kind == "toward":
        offset = P - body.heads[at[1]]
        out = offset / np.maximum(np.linalg.norm(offset, axis=1), 1e-9)[:, None]
        want = np.array(at[2]) / np.linalg.norm(at[2])
        allowed = model.skin & (np.abs(P[:, 0]) < 0.004)
        return int(np.argmax(np.where(allowed, out @ want, -2.0)))
    if kind == "forward":
        moved = np.linalg.norm(body.shapes[at[1]], axis=1) > 1e-5
        return int(np.argmax(np.where(moved & on_side, P[:, 2], -np.inf)))
    if kind in ("narrowest", "widest"):
        # The trunk's outline (not the arms beside it) at the heights of
        # the region, in slices 3 cm high every 5 mm (the skin's rows are
        # 1 to 3 cm apart there): the narrowest (or widest) slice wins.
        own = (body.region == regions[at[1]]) & on_side
        heights = P[own, 1]
        trunk = np.isin(body.region, [regions[r] for r in TRUNK]) & on_side
        best, chosen = None, None
        for y in np.arange(heights.min() + 0.015, heights.max() - 0.015, 0.005):
            band = trunk & (np.abs(P[:, 1] - y) < 0.015)
            if not band.any():
                continue
            out = np.where(band, np.abs(P[:, 0]), -1.0)
            width = out.max()
            better = best is None or (width < best if kind == "narrowest" else width > best)
            # Of the region's own corners within 1 cm of the outline, the
            # one half-way front to back, so the handle shows from the
            # front and a click on it picks its own feature.
            outline = band & own & (out >= width - 0.01)
            if better and outline.any():
                middle = (P[band, 2].min() + P[band, 2].max()) / 2
                best = width
                chosen = int(np.argmin(np.where(outline, np.abs(P[:, 2] - middle), np.inf)))
        return chosen
    raise ValueError(f"no rule {kind}")


def handles(manifest, path):
    """The `handles` list for the base manifest `manifest` (its parameters,
    regions and corners already written), measured on the model at `path`."""
    model = Model(path, manifest)
    cache = {}

    def ends(pid, frame=0.0):
        if (pid, frame) not in cache:
            cache[(pid, frame)] = model.ends(pid, frame)
        return cache[(pid, frame)]

    out = []
    for spec in HANDLES:
        paired = "{side}" in spec["id"]
        sides = ("Left", "Right") if paired else (None,)
        left_corner = None
        for side in sides:
            if side == "Right":
                # The left corner mirrored across the middle.
                mirrored = model.body.P[left_corner] * np.array([-1.0, 1.0, 1.0])
                distance = np.linalg.norm(model.body.P - mirrored, axis=1)
                corner = int(np.argmin(np.where(model.skin, distance, np.inf)))
            else:
                corner = choose(model, spec["at"], side, ends)
            if side == "Left":
                left_corner = corner
            weight = np.zeros(len(model.body.P))
            weight[corner] = 1.0
            bone = model.body.bone_of(weight)
            offset = model.body.P[corner] - model.body.heads[bone]
            drives, views = [], []
            for pid in spec["drives"]:
                pid = sided(pid, side)
                low, high = ends(pid)
                moved = high[corner] - low[corner]
                travel = float(np.linalg.norm(moved))
                if travel < 1e-4:
                    raise ValueError(f"{sided(spec['id'], side)}: {pid} doesn't move its corner")
                drive = {"parameter": pid, "axis": moved / travel, "travel": travel, "screen": {}}
                # On the other frame, where its own shapes (the waist's
                # `<Frame>_Waist_*`) move the corner otherwise.
                if pid != "frame" and "frame" in model.parameters:
                    low, high = ends(pid, 1.0)
                    other = high[corner] - low[corner]
                    if np.linalg.norm(other - moved) > FRAME_APART:
                        length = float(np.linalg.norm(other))
                        drive["frames"] = [{"frame": 1.0, "axis": other / length, "travel": length}]
                drives.append(drive)
            for view in VIEWS:
                shown = []
                for drive in drives:
                    direction = screen_direction(drive["axis"], view)
                    length = float(np.linalg.norm(direction))
                    if length >= LIVE:
                        shown.append((length, direction / length, drive))
                # The drives that show most of their travel go first; one
                # that shares a line with a drive kept gives way. (A drive
                # the handle says `yields` goes after the others.)
                last = [sided(pid, side) for pid in spec.get("yields", [])]
                kept = []
                for length, direction, drive in sorted(shown, key=lambda s: (s[2]["parameter"] in last, -s[0])):
                    line = np.cos(np.radians(APART))
                    if all(abs(float(direction @ other)) < line for other in kept):
                        kept.append(direction)
                        drive["screen"][view] = [round(float(v), 3) for v in direction]
                        if view not in views:
                            views.append(view)
            for drive in drives:
                for d in [drive] + drive.get("frames", []):
                    d["axis"] = [round(float(v), 4) for v in d["axis"]]
                    d["travel"] = round(d["travel"], 4)
            handle = {"id": sided(spec["id"], side), "feature": spec["feature"],
                      "label": sided(spec["label"], side)}
            if side:
                other = "Right" if side == "Left" else "Left"
                handle["mirror"] = sided(spec["id"], other)
            handle["anchor"] = {"bone": bone, "offset": [round(float(v), 4) for v in offset]}
            handle["drives"] = drives
            handle["views"] = [v for v in VIEWS if v in views]
            out.append(handle)
    return out


if __name__ == "__main__":
    import json
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "Realistic.base.json")) as f:
        base = json.load(f)
    for h in handles(base, os.path.join(here, "RealisticParts.glb")):
        print(json.dumps(h))
        if "-v" in sys.argv:
            body = anchors.Body(os.path.join(here, "RealisticParts.glb"))
            print("   at", body.heads[h["anchor"]["bone"]] + np.array(h["anchor"]["offset"]))
    sys.exit(0)
