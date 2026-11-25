"""Builds the Axcenity human base body in Blender from MakeHuman's base
mesh, targets and system assets, by driving the installed MPFB add-on
(`mh.py`: MPFB is a tool here, none of its code is in this repository).

    blender -b --python assets/characters/human/build.py -- \
        --assets <MakeHuman system assets, unpacked> --out <folder>

(Without `--factory-startup`: Blender must find the MPFB extension.)

    --check          also compares this package's reading of the clothes
                     files with what MPFB itself fits, and stops if they
                     differ by more than 0.01 mm
    --texture-size   the largest side of a texture it writes (2048)

The steps: make MakeHuman's body at the masculine middle of its macro
controls, give it MPFB's default rig and weights, load the face's parts
(eyes, teeth, tongue, brows, lashes), sample the body at every corner of
the controls the base offers (`SHAPES`), turn everything from MakeHuman's
rest pose to the engine's 45 degree A-pose (`mh.Repose`), stand it
1.75 m tall, put it on the Axcenity Humanoid skeleton (the stylized
package's bones and rolls, so every clip plays), and export
`HumanParts.glb` with the base model manifest (`manifests.py`).

Units are metres, Blender Z up, the character faces -Y (glTF +Z after
export).
"""

import argparse
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Vector  # noqa: E402

import face  # noqa: E402
import manifests  # noqa: E402
import mh  # noqa: E402
import skins  # noqa: E402
import underwear  # noqa: E402

ARMATURE = "AxcenityHuman"
PARTS_FILE = "HumanParts.glb"
DATA_FILE = "Human.build.json"
HEIGHT = 1.75
STYLIZED = os.path.join(os.path.dirname(HERE), "humanoid")
SIDES = (("Left", 1, "L"), ("Right", -1, "R"))
FINGERS = (("Thumb", 1), ("Index", 2), ("Middle", 3), ("Ring", 4), ("Little", 5))
SEGMENTS = ("Proximal", "Intermediate", "Distal")
# (twist bone, the bone it's inside, MakeHuman's half of that limb, where
# along the limb it lies). As the realistic package's: the upper twist
# holds the half nearer the body, the lower the half nearer the hand or foot.
TWISTS = (("UpperArmTwist", "UpperArm", "upperarm01", (0.08, 0.5)),
          ("LowerArmTwist", "LowerArm", "lowerarm02", (0.5, 0.92)),
          ("UpperLegTwist", "UpperLeg", "upperleg01", (0.08, 0.5)),
          ("LowerLegTwist", "LowerLeg", "lowerleg02", (0.5, 0.92)))

# The face's parts: (object, MakeHuman asset under --assets, MPFB's kind, material).
PARTS = (
    ("Eyes", "eyes/high-poly/high-poly.mhclo", "Eyes", "Eyes"),
    ("Teeth", "teeth/teeth_base/teeth_base.mhclo", "Teeth", "Teeth"),
    ("Tongue", "tongue/tongue01/tongue01.mhclo", "Tongue", "Tongue"),
    ("Brows", "eyebrows/eyebrow001/eyebrow001.mhclo", "Eyebrows", "Brows"),
    ("Lashes", "eyelashes/eyelashes01/eyelashes01.mhclo", "Eyelashes", "Lashes"),
)
# How a part's texture is see-through: the brows' and lashes' hairs fade
# out, and the eye's outer shell (the cornea) is clear over the iris.
PART_ALPHA = {"Brows": "blend", "Lashes": "blend", "Eyes": "cut"}
SKIN = f"skins/{skins.DEFAULT}/{skins.DEFAULT}.mhmat"

# The body's shapes, each a sum of MakeHuman bodies: (weight, settings).
# `M` and `F` are the two frames at the middle of muscle and weight; a
# frame's muscle and fat shapes are that frame's own (MakeHuman has a
# target per gender, muscle and weight), and a corner shape is what the
# two together add beyond their sum, so `manifests.py` drives it by the
# product of the two sliders (architecture.md section 7).
M = {"gender": 1.0}
F = {"gender": 0.0}


def _shapes():
    out = {"Frame_Feminine": [(1, F), (-1, M)]}
    for frame, g in (("Masculine", M), ("Feminine", F)):
        level = {"Up": 1.0, "Down": 0.0}
        for way, value in level.items():
            out[f"{frame}_Muscle_{way}"] = [(1, {**g, "muscle": value}), (-1, g)]
            out[f"{frame}_Fat_{way}"] = [(1, {**g, "weight": value}), (-1, g)]
        for mw, mv in level.items():
            for fw, fv in level.items():
                out[f"{frame}_Muscle_{mw}_Fat_{fw}"] = [
                    (1, {**g, "muscle": mv, "weight": fv}), (-1, {**g, "muscle": mv}),
                    (-1, {**g, "weight": fv}), (1, g)]
        out[f"{frame}_Age_Old"] = [(1, {**g, "age": 1.0}), (-1, g)]
    return out


SHAPES = _shapes()
# A shape that moves no corner this far (metres) isn't written.
LEAST = 0.00005


def stylized_rig():
    """The stylized humanoid's `rig` module, loaded with its own `common`
    (the two packages share bones and rolls, so they can't drift apart)."""
    saved = sys.modules.get("common")

    def load(module, alias):
        spec = importlib.util.spec_from_file_location(alias, os.path.join(STYLIZED, module + ".py"))
        made = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(made)
        return made

    try:
        sys.modules["common"] = load("common", "stylized_common")
        return load("rig", "stylized_rig")
    finally:
        if saved is not None:
            sys.modules["common"] = saved
        else:
            sys.modules.pop("common", None)


def key_of(settings):
    return json.dumps(settings, sort_keys=True)


class Samples:
    """MakeHuman bodies by their settings, made once each."""

    def __init__(self):
        self.made = {}

    def at(self, settings):
        key = key_of(settings)
        if key not in self.made:
            self.made[key] = mh.sample(mh.macro(**settings))
        return self.made[key]

    def shape(self, terms):
        return sum(weight * self.at(settings) for weight, settings in terms)


# -- the skeleton ------------------------------------------------------------------

def humanoid_bone(name, parent_of):
    """The Humanoid bone that carries MakeHuman's bone `name`."""
    side = {"L": "Left", "R": "Right"}.get(name.rsplit(".", 1)[-1], "") if "." in name else ""
    stem = name.rsplit(".", 1)[0] if side else name
    fixed = {
        "root": "Hips", "pelvis": "Hips",
        "spine05": "Spine", "spine04": "Spine", "spine03": "Chest", "spine02": "Chest",
        "spine01": "UpperChest", "breast": "Chest",
        "neck01": "Neck", "neck02": "Neck", "neck03": "Neck", "head": "Head", "jaw": "Jaw",
        "eye": side + "Eye", "clavicle": side + "Shoulder", "shoulder01": side + "Shoulder",
        "upperarm01": side + "UpperArmTwist", "upperarm02": side + "UpperArm",
        "lowerarm01": side + "LowerArm", "lowerarm02": side + "LowerArmTwist", "wrist": side + "Hand",
        "upperleg01": side + "UpperLegTwist", "upperleg02": side + "UpperLeg",
        "lowerleg01": side + "LowerLeg", "lowerleg02": side + "LowerLegTwist", "foot": side + "Foot",
    }
    if stem in fixed:
        return fixed[stem]
    if stem.startswith("metacarpal"):
        return side + "Hand"
    if stem.startswith("toe"):
        return side + "Toes"
    if stem.startswith("finger"):
        finger, segment = stem[len("finger"):].split("-")
        return side + FINGERS[int(finger) - 1][0] + SEGMENTS[int(segment) - 1]
    # The face's and the tongue's bones: whatever carries their parent.
    return humanoid_bone(parent_of[name], parent_of)


def contract_joints(centres, pose, stand):
    """Bone name -> (head, tail, parent) of the Humanoid contract's bones,
    from MakeHuman's joint centres: each carried into the engine's pose by
    the bone that holds it, then stood on the floor (`stand`)."""

    def at(name, bone=None):
        return Vector(stand(pose.carry(bone, centres[name])))

    j = {"Root": (Vector((0, 0, 0)), Vector((0, -0.12, 0)), None)}
    pelvis = at("pelvis")
    j["Hips"] = (pelvis - Vector((0, 0, 0.03)), pelvis, "Root")
    j["Spine"] = (pelvis, at("spine-3"), "Hips")
    j["Chest"] = (at("spine-3"), at("spine-1"), "Spine")
    j["UpperChest"] = (at("spine-1"), at("neck"), "Chest")
    j["Neck"] = (at("neck"), at("head"), "UpperChest")
    head, crown = at("head"), at("head-2")
    j["Head"] = (head, Vector((head.x, head.y, crown.z)), "Neck")
    # MakeHuman's `mouth` cube is the jaw's hinge, its `jaw` cube the chin.
    j["Jaw"] = (at("mouth"), at("jaw"), "Head")
    for side, s, mhs in SIDES:
        c = mhs.lower()
        upper, lower = f"upperarm01.{mhs}", f"lowerarm01.{mhs}"
        eye = at(f"{c}-eye")
        j[f"{side}Eye"] = (eye, eye + Vector((0, -0.03, 0)), "Head")
        j[f"{side}Shoulder"] = (at(f"{c}-clavicle"), at(f"{c}-shoulder"), "UpperChest")
        j[f"{side}UpperArm"] = (at(f"{c}-shoulder"), at(f"{c}-elbow", upper), f"{side}Shoulder")
        j[f"{side}LowerArm"] = (at(f"{c}-elbow", upper), at(f"{c}-hand", lower), f"{side}UpperArm")
        j[f"{side}Hand"] = (at(f"{c}-hand", lower), at(f"{c}-finger-3-1", lower), f"{side}LowerArm")
        for finger, number in FINGERS:
            parent = f"{side}Hand"
            for k, segment in enumerate(SEGMENTS):
                a, b = (at(f"{c}-finger-{number}-{n}", lower) for n in (k + 1, k + 2))
                j[f"{side}{finger}{segment}"] = (a, b, parent)
                parent = f"{side}{finger}{segment}"
        ankle, ball, tip = at(f"{c}-ankle"), at(f"{c}-foot-1"), at(f"{c}-foot-2")
        ball.z = tip.z = 0.022
        j[f"{side}UpperLeg"] = (at(f"{c}-upper-leg"), at(f"{c}-knee"), "Hips")
        j[f"{side}LowerLeg"] = (at(f"{c}-knee"), ankle, f"{side}UpperLeg")
        j[f"{side}Foot"] = (ankle, ball, f"{side}LowerLeg")
        j[f"{side}Toes"] = (ball, tip, f"{side}Foot")
    return j


def added_bones():
    return [(side + name, side + inside) for side, _, _ in SIDES for name, inside, _, _ in TWISTS]


def build_armature(joints):
    """The stylized humanoid's armature for `joints`, plus the twist bones."""
    rig = stylized_rig()
    armature = rig.build_armature(joints, ARMATURE)
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    ebs = armature.data.edit_bones
    for side, _, _ in SIDES:
        for name, inside, _, (a, b) in TWISTS:
            head, tail, _ = joints[side + inside]
            eb = ebs.new(side + name)
            eb.head, eb.tail = head.lerp(tail, a), head.lerp(tail, b)
            eb.parent = ebs[side + inside]
            eb.use_connect = False
            eb.roll = 0.0
            eb.align_roll(ebs[side + inside].z_axis)
            eb.use_deform = True
    bpy.ops.object.mode_set(mode="OBJECT")
    armature.select_set(False)
    return armature


def humanoid_weights(weights, names, parent_of):
    """MakeHuman's weights gathered onto the Humanoid bones: at most four a
    corner, adding to one. Returns {bone: (corner indices, weights)}."""
    targets = [humanoid_bone(n, parent_of) for n in names]
    order = sorted(set(targets))
    column = {n: k for k, n in enumerate(order)}
    gathered = np.zeros((weights.shape[0], len(order)))
    for k, target in enumerate(targets):
        gathered[:, column[target]] += weights[:, k]
    # The four heaviest.
    if gathered.shape[1] > 4:
        cut = np.partition(gathered, -4, axis=1)[:, -4][:, None]
        gathered = np.where(gathered >= cut, gathered, 0.0)
    total = gathered.sum(axis=1, keepdims=True)
    loose = total[:, 0] <= 1e-9
    gathered = gathered / np.maximum(total, 1e-9)
    out = {}
    for name, k in column.items():
        idx = np.nonzero(gathered[:, k] > 0)[0]
        if len(idx):
            out[name] = (idx, gathered[idx, k])
    return out, loose


# -- objects -----------------------------------------------------------------------

class Piece:
    """A mesh taken from Blender: where its corners are, its faces and its
    texture coordinates, and MakeHuman's weights on it."""

    def __init__(self, obj, names, keep=None):
        mesh = obj.data
        self.count = len(mesh.vertices)
        self.keep = np.arange(self.count) if keep is None else np.asarray(keep)
        renumber = np.full(self.count, -1, dtype=np.int64)
        renumber[self.keep] = np.arange(len(self.keep))
        self.faces, self.uvs = [], []
        uv = mesh.uv_layers.active.data if mesh.uv_layers.active else None
        for poly in mesh.polygons:
            corners = [renumber[v] for v in poly.vertices]
            if min(corners) < 0:
                continue
            self.faces.append(corners)
            self.uvs.append([tuple(uv[l].uv) if uv else (0.0, 0.0) for l in poly.loop_indices])
        self.weights = mh.weights(obj, names)[self.keep]


def has_holes(image):
    """Whether a texture is see-through anywhere (a tenth of a percent of
    it or more: a stray texel isn't a hole)."""
    if image.channels < 4 or not image.size[0]:
        return False
    alpha = np.empty(image.size[0] * image.size[1] * 4, dtype=np.float32)
    image.pixels.foreach_get(alpha)
    return float((alpha[3::4] < 0.5).mean()) > 0.001


_compact = {}


def compact(path, keep_alpha):
    """A texture file to load: the file itself, or, for a solid PNG, a
    JPEG copy of it (a tenth of the size in the model; the exporter keeps
    each image's own format)."""
    if not path.lower().endswith(".png") or keep_alpha:
        return path
    if path not in _compact:
        import tempfile
        from PIL import Image
        folder = _compact.setdefault("", tempfile.mkdtemp(prefix="human-textures-"))
        out = os.path.join(folder, f"{len(_compact)}-" + os.path.splitext(os.path.basename(path))[0] + ".jpg")
        try:
            Image.open(path).convert("RGB").save(out, quality=90)
        except (OSError, ValueError):
            out = path  # a file Pillow can't read is left to Blender
        _compact[path] = out
    return _compact[path]


def simple_material(name, colour=(0.8, 0.8, 0.8, 1.0), texture=None, normal=None, alpha=None, roughness=0.6):
    """A plain material; `alpha` is None (solid), `"cut"` (cut out where
    the texture is see-through, if it is anywhere) or `"blend"`."""
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    nodes, links = material.node_tree.nodes, material.node_tree.links
    shader = nodes["Principled BSDF"]
    shader.inputs["Base Color"].default_value = colour
    shader.inputs["Roughness"].default_value = roughness
    if texture and os.path.exists(texture):
        image = nodes.new("ShaderNodeTexImage")
        image.image = bpy.data.images.load(texture, check_existing=True)
        holes = alpha is not None and has_holes(image.image)
        if not holes:
            image.image = bpy.data.images.load(compact(texture, False), check_existing=True)
        links.new(image.outputs["Color"], shader.inputs["Base Color"])
        if alpha == "blend" and holes:
            links.new(image.outputs["Alpha"], shader.inputs["Alpha"])
            material.blend_method = "BLEND"
        elif alpha == "cut" and holes:
            # Cut out, not blended (glTF's MASK, which the exporter reads
            # from a rounded alpha): the cloth still casts its shadow and
            # sorts like any solid surface.
            cut = nodes.new("ShaderNodeMath")
            cut.operation = "ROUND"
            links.new(image.outputs["Alpha"], cut.inputs[0])
            links.new(cut.outputs[0], shader.inputs["Alpha"])
    if normal and os.path.exists(normal):
        image = nodes.new("ShaderNodeTexImage")
        image.image = bpy.data.images.load(compact(normal, False), check_existing=True)
        image.image.colorspace_settings.name = "Non-Color"
        bump = nodes.new("ShaderNodeNormalMap")
        links.new(image.outputs["Color"], bump.inputs["Color"])
        links.new(bump.outputs["Normal"], shader.inputs["Normal"])
    return material


def read_mhmat(path):
    """A `.mhmat` file's keys (MakeHuman's material format: a key and its
    value a line), texture paths made absolute."""
    out = {}
    folder = os.path.dirname(path)
    with open(path, encoding="utf-8", errors="replace") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#") or line.startswith("//"):
                continue
            words = line.split(None, 1)
            if len(words) == 2:
                value = words[1].strip()
                out[words[0]] = os.path.join(folder, value) if words[0].endswith("Texture") else value
    return out


def material_of(name, mhmat, alpha=None):
    values = read_mhmat(mhmat) if mhmat and os.path.exists(mhmat) else {}
    colour = (0.8, 0.8, 0.8, 1.0)
    if "diffuseColor" in values:
        try:
            colour = tuple(float(x) for x in values["diffuseColor"].split()[:3]) + (1.0,)
        except ValueError:
            pass
    return simple_material(name, colour, values.get("diffuseTexture"), values.get("normalmapTexture"), alpha)


def make_object(name, piece, points, shapes, groups, material, armature):
    """A skinned mesh object with its shapes, under `armature`."""
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([tuple(p) for p in points], [], piece.faces)
    layer = mesh.uv_layers.new(name="UVMap")
    flat = [uv for face in piece.uvs for uv in face]
    layer.data.foreach_set("uv", [c for uv in flat for c in uv])
    mesh.polygons.foreach_set("use_smooth", [True] * len(mesh.polygons))
    mesh.materials.append(material)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    for bone, (idx, w) in groups.items():
        group = obj.vertex_groups.new(name=bone)
        for i, weight in zip(idx.tolist(), w.tolist()):
            group.add([i], weight, "REPLACE")
    if shapes:
        obj.shape_key_add(name="Basis")
        for shape, moves in shapes.items():
            key = obj.shape_key_add(name=shape)
            key.data.foreach_set("co", (points + moves).ravel())
            # Blender 5 adds a key at full value, and the exporter writes
            # that as the shape's resting weight: a shape nothing drives
            # (an expression) would be worn whole.
            key.value = 0.0
    obj.parent = armature
    modifier = obj.modifiers.new("Armature", "ARMATURE")
    modifier.object = armature
    return obj


def region_of(bone):
    """The body region a Humanoid bone's skin belongs to."""
    side_less = bone[4:] if bone.startswith("Left") else bone[5:] if bone.startswith("Right") else bone
    fixed = {
        "Head": "head", "Jaw": "head", "Eye": "head", "Neck": "neck", "UpperChest": "torso", "Chest": "torso",
        "Shoulder": "torso", "Spine": "waist", "Hips": "hips", "UpperArmTwist": "upper_arms_top",
        "UpperArm": "upper_arms_low", "LowerArm": "forearms", "LowerArmTwist": "forearms",
        "UpperLegTwist": "thighs_top", "UpperLeg": "thighs_low", "LowerLeg": "shins_top",
        "LowerLegTwist": "shins_low", "Foot": "feet", "Toes": "feet",
    }
    return fixed.get(side_less, "hands")


def face_regions(piece, groups):
    """Each face's region index (`manifests.REGIONS`): the region of the
    bone that weighs most on most of its corners."""
    count = len(piece.keep)
    best = np.zeros(count)
    region = np.zeros(count, dtype=np.int64)
    for bone, (idx, w) in groups.items():
        r = manifests.REGION_INDEX[region_of(bone)]
        better = w > best[idx]
        best[idx[better]] = w[better]
        region[idx[better]] = r
    out = []
    for face in piece.faces:
        values, counts = np.unique(region[face], return_counts=True)
        out.append(int(values[counts.argmax()]))
    return out


def write_region_map(path, piece, regions, size=1024):
    """The region index of the skin under each texel of the body's texture
    coordinates (the manifest's `regions_map`), bled a few texels past
    each island's edge so a sample on an edge still finds its region."""
    from PIL import Image, ImageDraw
    image = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(image)
    for uvs, region in zip(piece.uvs, regions):
        draw.polygon([(u * size, (1 - v) * size) for u, v in uvs], fill=region, outline=region)
    a = np.asarray(image).copy()
    for _ in range(3):
        grown = a.copy()
        for shift in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            grown = np.where(grown == 0, np.roll(a, shift, axis=(0, 1)), grown)
        a = grown
    Image.fromarray(a).save(path, optimize=True)


def write_cover(path, body, hidden, size=1024):
    """An item's cover map (ADR-0045): red where the body's skin is left
    out under the garment, over the body's texture coordinates. A face
    goes only when every corner of it is hidden. Returns the faces lit."""
    from PIL import Image, ImageDraw
    image = Image.new("RGB", (size, size), (0, 0, 0))
    draw = ImageDraw.Draw(image)
    lit = 0
    for corners, uvs in zip(body.faces, body.uvs):
        if all(hidden[c] for c in corners):
            draw.polygon([(u * size, (1 - v) * size) for u, v in uvs], fill=(255, 0, 0))
            lit += 1
    image.save(path, optimize=True)
    return lit


def skin_triangles(points, faces):
    """The triangles Blender splits these faces into (what the glTF
    exporter writes for the body made of them)."""
    mesh = bpy.data.meshes.new("Triangles")
    mesh.from_pydata([tuple(p) for p in points], [], faces)
    mesh.update()
    mesh.calc_loop_triangles()
    out = [list(t.vertices) for t in mesh.loop_triangles]
    bpy.data.meshes.remove(mesh)
    return out


class Cut:
    """A garment cut from the body (`underwear.py`), as `make_object` takes a piece."""

    def __init__(self, garment):
        self.faces, self.uvs = garment["faces"], garment["uvs"]


# The garments cut from the body: (object, folder, item ID, label, slot,
# the cut, the body shape it is laid out on).
def cut_items():
    return (
        ("BoxerBriefs", "boxer-briefs", manifests.BOXER_BRIEFS, "Boxer briefs", "underwear_bottom",
         underwear.boxer_briefs, None),
        ("Bandeau", "bandeau", manifests.BANDEAU, "Bandeau top", "underwear_top", underwear.bandeau,
         "Frame_Feminine"),
    )


def write_cut_item(root, spec, build, joints, body_points, body_groups, body_shapes, frame_bones):
    """An item cut from the body: its model, cover map and manifest."""
    name, folder, item_id, label, slot, make, laid_on = spec
    folder = os.path.join(root, folder)
    # Cut from the skin's triangles as the exporter will split its quads,
    # not from the quads: a cloth face then lies in the plane of the skin
    # triangle under it. Cut from a quad it is split along whichever
    # diagonal suits its new outline, and on a quad that isn't flat (most
    # of a thigh's) the other diagonal's fold comes through the cloth.
    triangles = skin_triangles(body_points, build.body.faces)
    on = body_points
    at = dict(joints)
    if laid_on:
        # Laid out on another frame's body, with its joints where that
        # frame has them; the cloth's stand-off is kept on the rest body.
        on = body_points + body_shapes[laid_on]
        for bone, offset in frame_bones.items():
            head, tail, parent = at[bone]
            move = Vector((offset[0], -offset[2], offset[1]))
            at[bone] = (head + move, tail + move, parent)
    garment = make(on, triangles, body_groups, at)
    mi, mj, mt = garment["mix"]
    garment["points"] = body_points[mi] * (1 - mt) + body_points[mj] * mt + garment["offset"]
    shapes = {}
    for shape, moves in body_shapes.items():
        if shape in build.expressions:
            continue
        d = underwear.shape_moves(garment, moves)
        if np.abs(d).max() > LEAST:
            shapes[shape] = d
    os.makedirs(folder, exist_ok=True)
    armature = build_armature(joints)
    material = simple_material("Underwear", (0.035, 0.037, 0.045, 1.0), roughness=0.85)
    obj = make_object(name, Cut(garment), garment["points"], shapes, garment["groups"], material, armature)
    export(os.path.join(folder, name + ".glb"), armature, [obj])
    for o in (obj, armature):
        bpy.data.objects.remove(o)
    for a in list(bpy.data.armatures):
        bpy.data.armatures.remove(a)
    bpy.data.materials.remove(material)  # the next item's is named the same
    lit = write_cover(os.path.join(folder, name + ".cover.png"), build.body, garment["hidden"])
    item = {
        "format_version": 1, "id": item_id, "label": label, "kind": "garment",
        "slot": slot, "layer": 0, "tier": "built_in", "bases": [manifests.BASE_ID],
        "model": {"file": name + ".glb", "object": f"{ARMATURE}/{name}"},
        "cover_map": name + ".cover.png",
        "materials": [{"name": "Underwear", "shading": "cloth", "label": "Cloth", "role": "primary"}],
        "fit": {"method": "shapes", "follows": sorted(shapes)},
        "tags": ["underwear"],
        "about": "Cut from the body's own surface, so it follows every body slider.",
        "source": {"author": "Axcenity Engine contributors, from MakeHuman's base mesh (CC0)",
                   "licence": "CC0-1.0"},
        "quality": "draft",
    }
    with open(os.path.join(folder, name + ".item.json"), "w", newline="\n") as f:
        json.dump(item, f, indent=2)
        f.write("\n")
    return {"corners": int(len(garment["points"])), "faces": len(garment["faces"]), "shapes": len(shapes),
            "hidden_faces": lit}


def export(path, armature, objects):
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    for o in objects:
        o.select_set(True)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=True,
        export_skins=True, export_morph=True, export_morph_normal=True, export_try_sparse_sk=True,
        export_animations=False, export_yup=True, export_def_bones=False, export_leaf_bone=False,
        export_tangents=False, export_image_format="AUTO")


def to_gltf(d):
    """A Blender-space vector (Z up, front -Y) in glTF space (Y up, front +Z)."""
    return [round(float(d[0]), 5), round(float(d[2]), 5), round(float(-d[1]), 5)]


class Build:
    """The body in MakeHuman's pose with everything needed to bring a
    piece hung on it (a part, a garment) into the engine's."""

    def __init__(self, assets):
        self.assets = assets
        self.samples = Samples()
        self.base = mh.human(mh.macro(**M))
        self.rest = mh.positions(self.base)
        self.samples.made[key_of(M)] = self.rest
        self.cubes = mh.joint_groups()
        # MPFB's default rig, for its bones and its weights (the joints
        # themselves are the base mesh's joint cubes, `mh.joint_groups`).
        self.rig = mh.add_rig(self.base)
        self.bones = mh.rig_bones(self.rig)
        self.names = [b[0] for b in self.bones]
        self.parent_of = dict(self.bones)
        self.pose = mh.Repose(self.bones, mh.joint_centres(self.cubes, self.rest))
        groups = {g.name: g.index for g in self.base.vertex_groups}
        body = groups["body"]
        self.skin = np.array([v.index for v in self.base.data.vertices
                              if any(g.group == body for g in v.groups)])
        self.body = Piece(self.base, self.names, self.skin)
        self.body_matrix = self.pose.blend(self.body.weights)
        posed = mh.Repose.points(self.body_matrix, self.rest[self.skin])
        self.floor = posed[:, 2].min()
        self.scale = HEIGHT / (posed[:, 2].max() - self.floor)
        self.moves = {name: self.samples.shape(terms) for name, terms in SHAPES.items()}
        # The face: what it does (expression shapes, live) and who it is
        # (identity parameters), from MakeHuman's targets (`face.py`).
        count = len(self.rest)

        def read(folder, name):
            return mh.read_target(folder, name, count)

        self.expressions = face.expressions(read, self.rest)
        self.face_parameters, identity = face.identity(os.path.join(mh.data_root(), "targets"), read)
        self.identity = set(identity)
        self.moves.update(self.expressions)
        self.moves.update(identity)

    def stand(self, points):
        return (points - np.array([0.0, 0.0, self.floor])) * self.scale

    def body_points(self):
        return self.stand(mh.Repose.points(self.body_matrix, self.rest[self.skin]))

    def body_shapes(self):
        out = {}
        for name, moves in self.moves.items():
            d = mh.Repose.moves(self.body_matrix, moves[self.skin]) * self.scale
            if np.abs(d).max() > LEAST:
                out[name] = d
        return out

    def hung(self, obj, clothes):
        """A piece MPFB fitted to the body: (piece, its corners in the
        engine's pose, its shapes), each shape what the clothes file makes
        of the body's."""
        piece = Piece(obj, self.names)
        matrix = self.pose.blend(piece.weights)
        rest = clothes.place(self.rest)
        points = self.stand(mh.Repose.points(matrix, rest))
        shapes = {}
        for name, moves in self.moves.items():
            d = mh.Repose.moves(matrix, clothes.place(self.rest + moves) - rest) * self.scale
            if np.abs(d).max() > LEAST:
                shapes[name] = d
        return piece, points, shapes

    def joints(self, body=None):
        """The contract's joints for the body `body` (all of MakeHuman's
        corners; the rest body when None), in the engine's pose, standing
        where that body stands."""
        if body is None:
            return contract_joints(mh.joint_centres(self.cubes, self.rest), self.pose, self.stand)
        centres = mh.joint_centres(self.cubes, body)
        pose = mh.Repose(self.bones, centres)
        posed = mh.Repose.points(pose.blend(self.body.weights), body[self.skin])
        floor = np.array([0.0, 0.0, posed[:, 2].min()])
        return contract_joints(centres, pose, lambda p: (p - floor) * self.scale)

    def shape_bone_offsets(self, joints, shape):
        """How far a shape at full weight moves each contract bone's head
        (glTF axes): the joint cubes move with the body's corners."""
        centres = mh.joint_centres(self.cubes, self.rest + self.moves[shape])
        theirs = contract_joints(centres, self.pose, self.stand)
        out = {}
        for name, (head, _, _) in joints.items():
            d = theirs[name][0] - head
            if name != "Root" and d.length > 0.0005:
                out[name] = to_gltf(d)
        return out

    def bone_offsets(self, joints, settings):
        """How far each contract bone's head is from the rest body's on
        the body with these settings (glTF axes), for a parameter's `bones`."""
        theirs = self.joints(self.samples.at(settings))
        out = {}
        for name, (head, _, _) in joints.items():
            d = theirs[name][0] - head
            if name != "Root" and d.length > 0.0005:
                out[name] = to_gltf(d)
        return out


def check_clothes(build, obj, clothes):
    """How far this package's reading of a clothes file is from MPFB's own fit."""
    theirs = np.array([tuple(v.co) for v in obj.data.vertices])
    ours = clothes.place(build.rest)
    return float(np.abs(theirs - ours).max())


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--assets", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--texture-size", type=int, default=2048)
    args = parser.parse_args(argv)
    os.makedirs(args.out, exist_ok=True)

    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    build = Build(args.assets)
    report = {"mpfb": mh.version(), "blender": bpy.app.version_string, "scale": round(build.scale, 5),
              "body_corners": int(len(build.skin)), "shapes": {}, "parts": {}}

    pieces = []
    body_groups, loose = humanoid_weights(build.body.weights, build.names, build.parent_of)
    assert not loose.any(), f"{int(loose.sum())} body corners have no weights"
    body_shapes = build.body_shapes()
    report["shapes"] = {n: round(float(np.linalg.norm(d, axis=1).max()) * 1000, 2) for n, d in body_shapes.items()}
    # Materials are made last: MPFB clears unused ones while it loads assets.
    pieces.append(("Body", build.body, build.body_points(), body_shapes, body_groups,
                   ("Skin", os.path.join(args.assets, SKIN), None)))

    for name, path, kind, material in PARTS:
        full = os.path.join(args.assets, path)
        if not os.path.exists(full):
            print(f"human build: no {path}; {name} left out")
            continue
        clothes = mh.Clothes(full)
        obj = mh.add_asset(build.base, full, kind)
        if args.check:
            worst = check_clothes(build, obj, clothes)
            report["parts"].setdefault(name, {})["mpfb_fit_difference_mm"] = round(worst * 1000, 4)
            assert worst < 0.00001, f"{name}: this reading of {path} is {worst * 1000:.3f} mm from MPFB's fit"
        piece, points, shapes = build.hung(obj, clothes)
        groups, loose = humanoid_weights(piece.weights, build.names, build.parent_of)
        if loose.any():
            # A part with no weights of its own rides the head.
            idx = np.nonzero(loose)[0]
            have = groups.get("Head", (np.zeros(0, dtype=np.int64), np.zeros(0)))
            groups["Head"] = (np.concatenate([have[0], idx]), np.concatenate([have[1], np.ones(len(idx))]))
        mhmat = os.path.join(clothes.folder, clothes.fields.get("material", ""))
        pieces.append((name, piece, points, shapes, groups, (material, mhmat, PART_ALPHA.get(name))))
        report["parts"].setdefault(name, {}).update(corners=int(len(points)), shapes=len(shapes))

    joints = build.joints()
    frame_bones = build.bone_offsets(joints, F)
    build_expressions = set(build.expressions)
    face_parameters = [p for p in build.face_parameters
                       if any(p.get(end) in body_shapes for end in ("low", "high"))]
    for p in face_parameters:
        for end in ("low", "high"):
            if p.get(end) not in body_shapes:
                p.pop(end, None)
    # The joints a face shape moves (the eyes with their sockets, the jaw
    # with the chin), so a bone still turns about its own middle.
    face_bones = {}
    for p in face_parameters:
        for end in ("low", "high"):
            if end in p:
                moved = build.shape_bone_offsets(joints, p[end])
                if moved:
                    face_bones[p[end]] = moved
    mh.remove(build.base)
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)

    armature = build_armature(joints)
    objects = [make_object(name, piece, points, shapes, groups, material_of(*material), armature)
               for name, piece, points, shapes, groups, material in pieces]
    export(os.path.join(args.out, PARTS_FILE), armature, objects)
    object_names = [o.name for o in objects]
    material_names = sorted({m.name for o in objects for m in o.data.materials})

    for o in list(bpy.data.objects):  # the briefs get an armature of the same name
        bpy.data.objects.remove(o)
    for a in list(bpy.data.armatures):
        bpy.data.armatures.remove(a)
    report["cut_items"] = {
        spec[0]: write_cut_item(os.path.join(args.out, "items"), spec, build, joints, pieces[0][2],
                                body_groups, body_shapes, frame_bones)
        for spec in cut_items()}
    report["joints"] = {name: to_gltf(head) for name, (head, _, _) in joints.items()}
    report["face"] = {"controls": sorted(c for c in build_expressions if c in body_shapes),
                      "unmapped": list(face.UNMAPPED), "parameters": face_parameters,
                      "bones": face_bones}
    report["frame_bones"] = frame_bones
    # Every MakeHuman skin as an image beside the manifest (ADR-0048).
    report["skins"] = skins.write(args.assets, args.out, args.texture_size)
    for left in report["skins"]["left_out"]:
        print(f"human build: skin {left['skin']} left out: {left['why']}")
    report["added_bones"] = [{"name": n, "parent": p} for n, p in added_bones()]
    report["objects"] = object_names
    report["materials"] = material_names
    write_region_map(os.path.join(args.out, manifests.REGIONS_MAP), build.body, face_regions(build.body, body_groups))
    with open(os.path.join(args.out, DATA_FILE), "w") as f:
        json.dump(report, f, indent=1)
    manifests.write(args.out)
    print("human build: wrote", os.path.join(args.out, PARTS_FILE))


if __name__ == "__main__":
    main()
