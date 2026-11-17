"""The realistic body's handles (Character Studio refinement R2, brief
§3.4, §4.3 and §5.3): the base manifest's `handles`, written by
`manifests.py`. Plain Python with numpy, like `anchors.py`.

A handle is a point on the skin that a person drags in the viewport. It
holds no geometry of its own: it names parameters (`drives`) and moves
them as their sliders would (plan section 11.3). The seven features come
in the plan's order: nose, eyes, jaw, mouth (the `lips` feature),
shoulders, waist and hips.

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

# The handles, by feature in the plan's order. `{side}` and `{Side}` make
# a pair (`left`/`Left` first). `at` is how its corner is found:
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
#   ("narrowest", region) / ("widest", region): the side of the region
#       where the trunk is narrowest or widest (in slices), the
#       outline's corner half-way front to back.
HANDLES = [
    {"id": "nose_tip", "feature": "nose", "label": "Nose tip",
     "at": ("forward", "Face_Nose_Length"), "drives": ["nose_length", "nose_tip_projection"]},
    {"id": "nose_bridge", "feature": "nose", "label": "Bridge",
     "at": ("middle", "nose_bridge"), "drives": ["nose_bridge"]},
    {"id": "nostril_{side}", "feature": "nose", "label": "{Side} nostril",
     "at": ("moved", "nostril_width_{side}"), "drives": ["nostril_width_{side}"]},
    {"id": "eye_{side}", "feature": "eyes", "label": "{Side} eye",
     "at": ("lid", "{Side}Eye", [0.0, 0.6, 0.8]),
     "drives": ["eye_spacing_{side}", "eye_height_{side}", "eye_depth_{side}"]},
    {"id": "eye_corner_{side}", "feature": "eyes", "label": "{Side} eye corner",
     "at": ("moved", "eye_tilt_{side}"), "drives": ["eye_size_{side}", "eye_tilt_{side}"]},
    {"id": "jaw_{side}", "feature": "jaw", "label": "{Side} jaw corner",
     "at": ("moved", "jaw_width_{side}"), "drives": ["jaw_width_{side}", "jaw_angle_{side}"]},
    {"id": "mouth_corner_{side}", "feature": "lips", "label": "{Side} mouth corner",
     "at": ("moved", "mouth_width_{side}"), "drives": ["mouth_width_{side}", "mouth_height"]},
    {"id": "upper_lip", "feature": "lips", "label": "Upper lip",
     "at": ("middle", "upper_lip_fullness"), "drives": ["upper_lip_fullness"]},
    {"id": "lower_lip", "feature": "lips", "label": "Lower lip",
     "at": ("middle", "lower_lip_fullness"), "drives": ["lower_lip_fullness"]},
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

    def corners_at(self, pid, value):
        """Every body corner with parameter `pid` at `value`."""
        body, rig = self.body, self.rig
        values = {p["id"]: p.get("default", 0.0) for p in self.manifest["parameters"]}
        values["frame"] = 0.0
        values[pid] = value
        # The shapes: each parameter's own, then the corners.
        P = body.P.copy()
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

    def ends(self, pid):
        low, high = self.parameters[pid]["range"]
        return self.corners_at(pid, low), self.corners_at(pid, high)


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

    def ends(pid):
        if pid not in cache:
            cache[pid] = model.ends(pid)
        return cache[pid]

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
                drives.append({"parameter": pid, "axis": moved / travel, "travel": travel, "screen": {}})
            for view in VIEWS:
                shown = []
                for drive in drives:
                    direction = screen_direction(drive["axis"], view)
                    length = float(np.linalg.norm(direction))
                    if length >= LIVE:
                        shown.append((length, direction / length, drive))
                # The drives that show most of their travel go first; one
                # that shares a line with a drive kept gives way.
                kept = []
                for length, direction, drive in sorted(shown, key=lambda s: -s[0]):
                    line = np.cos(np.radians(APART))
                    if all(abs(float(direction @ other)) < line for other in kept):
                        kept.append(direction)
                        drive["screen"][view] = [round(float(v), 3) for v in direction]
                        if view not in views:
                            views.append(view)
            for drive in drives:
                drive["axis"] = [round(float(v), 4) for v in drive["axis"]]
                drive["travel"] = round(drive["travel"], 4)
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
