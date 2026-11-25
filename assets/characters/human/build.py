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
    # The fullest of MakeHuman's natural brows: the thin ones (001) are a
    # pencil line at a head's distance.
    ("Brows", "eyebrows/eyebrow008/eyebrow008.mhclo", "Eyebrows", "Brows"),
    ("Lashes", "eyelashes/eyelashes01/eyelashes01.mhclo", "Eyelashes", "Lashes"),
)
# How a part's texture is see-through: the brows' and lashes' hairs fade
# out, and the eye's outer shell (the cornea) is clear over the iris.
# Brows and lashes are hair: cut out by their textures, with soft edges
# under TAA and a shadow through their holes (`docs/rendering/hair.md`).
PART_ALPHA = {"Brows": "cut", "Lashes": "cut", "Eyes": "cut"}
# What tells the engine a material's shading model (glTF `extras`, read by
# `assets`: `docs/rendering/skin.md`); the manifest says the same
# (`manifests.MATERIAL_SHADING`).
MATERIAL_EXTRAS = {
    "Skin": {"axcenity_shading": "skin"},
    "Brows": {"axcenity_shading": "hair", "axcenity_hair_shell": True},
    "Lashes": {"axcenity_shading": "hair"},
}
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


def bone_heads(joints):
    """{bone: its head} for the contract's `joints` and the twist bones
    `build_armature` adds inside them."""
    out = {name: head for name, (head, _, _) in joints.items()}
    for side, _, _ in SIDES:
        for name, inside, _, (a, _) in TWISTS:
            head, tail, _ = joints[side + inside]
            out[side + name] = head.lerp(tail, a)
    return out


def head_moves(joints, theirs):
    """How far each bone's head is in `theirs` from where it is in
    `joints` (glTF axes), for a parameter's `bones`. The twist bones are
    among them: the engine leaves a bone no change names where it is
    whatever its parent does (`assets` `identity.rs`), and a twist bone
    left on the default body's arm turns the forearm's skin about a line
    that isn't in the arm (a notch at the elbow of the feminine frame,
    whose arms are 12 cm lower)."""
    ours, theirs = bone_heads(joints), bone_heads(theirs)
    out = {}
    for name, head in ours.items():
        d = theirs[name] - head
        if name != "Root" and d.length > 0.0005:
            out[name] = to_gltf(d)
    return out


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


def simple_material(name, colour=(0.8, 0.8, 0.8, 1.0), texture=None, normal=None, alpha=None, roughness=0.6,
                    extras=None):
    """A plain material; `alpha` is None (solid), `"cut"` (cut out where
    the texture is see-through, if it is anywhere) or `"blend"`. `extras`
    are written into the file with it (`export(..., extras=True)`): what
    tells the engine its shading model (`axcenity_shading`)."""
    material = bpy.data.materials.new(name)
    for key, value in (extras or {}).items():
        material[key] = value
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


def material_of(name, mhmat, alpha=None, extras=None):
    values = read_mhmat(mhmat) if mhmat and os.path.exists(mhmat) else {}
    colour = (0.8, 0.8, 0.8, 1.0)
    if "diffuseColor" in values:
        try:
            colour = tuple(float(x) for x in values["diffuseColor"].split()[:3]) + (1.0,)
        except ValueError:
            pass
    return simple_material(name, colour, values.get("diffuseTexture"), values.get("normalmapTexture"), alpha,
                           extras=extras)


#: The attribute a fitted garment's corners carry: the body's own texture
#: coordinates at the skin each lies over (`axcenity_assets`'
#: `SKIN_UV_ATTRIBUTE`). With it what is worn over the garment hides it by
#: its cover map as it hides one the engine fitted: the triangles under
#: the cover go, and the ones that reach out from under it dip their
#: covered corners into the body, so the garment's edge goes under the
#: outer one's hem and doesn't stand through it.
SKIN_UV = "_AXCENITY_SKIN_UV"


def skin_uvs_of(body, body_points, points):
    """Each of a garment's corners' place on the skin: the body's texture
    coordinates (glTF's, v down) at the skin point nearest it."""
    from mathutils.bvhtree import BVHTree
    from mathutils.interpolate import poly_3d_calc
    tree = BVHTree.FromPolygons([Vector(p) for p in body_points], [tuple(int(c) for c in face) for face in body.faces])
    out = np.zeros((len(points), 2))
    for v, p in enumerate(points):
        at, _, index, _ = tree.find_nearest(Vector(p))
        corners = [Vector(body_points[c]) for c in body.faces[index]]
        shares = poly_3d_calc(corners, at)
        u = sum(w * uv[0] for w, uv in zip(shares, body.uvs[index]))
        t = sum(w * uv[1] for w, uv in zip(shares, body.uvs[index]))
        out[v] = (u, 1.0 - t)
    return out


def make_object(name, piece, points, shapes, groups, material, armature, skin_uvs=None):
    """A skinned mesh object with its shapes, under `armature`. With
    `skin_uvs` (one per corner) it carries them as `SKIN_UV`."""
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([tuple(p) for p in points], [], piece.faces)
    layer = mesh.uv_layers.new(name="UVMap")
    flat = [uv for face in piece.uvs for uv in face]
    layer.data.foreach_set("uv", [c for uv in flat for c in uv])
    mesh.polygons.foreach_set("use_smooth", [True] * len(mesh.polygons))
    mesh.materials.append(material)
    if skin_uvs is not None:
        # After the texture coordinates: made before them, the exporter
        # no longer finds which set a material's textures use.
        attribute = mesh.attributes.new(name=SKIN_UV, type="FLOAT2", domain="POINT")
        attribute.data.foreach_set("vector", np.asarray(skin_uvs, dtype=np.float32).ravel())
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


#: A cover map's green and blue (ADR-0045: the skin over which a garment
#: worn beneath leaves its own cloth out). Cloth is "over" a skin corner
#: when a line out along the corner's normal meets it within `CLOTH_REACH`
#: (started `CLOTH_BELOW` under the skin: MakeHuman's clothes dip into
#: skin they hide); a loose skirt far off the thigh covers nothing, and
#: what is worn under it is far from it too. From the garment's openings
#: the cover is set back by the cloth's distance from the skin there,
#: between `CLOTH_LEAST` and `CLOTH_MOST` (the engine's own setback of
#: cloth worn beneath, `avatar::fit::cover`): one sees a little way under
#: a loose hem.
CLOTH_REACH = 0.10
CLOTH_BELOW = 0.005
CLOTH_GAIN, CLOTH_LEAST, CLOTH_MOST = 1.0, 0.003, 0.040
#: Round the hips a bottom's blue has no setback: a top tucked into it
#: isn't seen under its waistband.
HIP_REGIONS = ("waist", "hips", "thighs_top")
#: How far a cover's green and blue are grown (texels of 1024). Over the
#: skin by one, for the rounding of a face's edge. Past the edge of the
#: body's texture islands by `COVER_PAST`, where there is no skin: a
#: garment's corner is placed on the map by the skin nearest it, which at
#: a seam of the texture coordinates (the middle of the crotch) is on an
#: island's very edge, and the engine counts a corner covered when most
#: of the map round it is lit, a hundredth of the map each way
#: (`axcenity_assets` `PINHOLE`): without this the underwear kept a strip
#: of 40 triangles along that seam under any trousers.
COVER_GROW = 1
COVER_PAST = 12


def cloth_over(body_points, body_faces, points, faces):
    """(covered, set back) for each of the body's corners under the
    garment whose corners are `points` and faces `faces`: whether cloth is
    over it, and whether it is over it clear of the garment's openings
    (`CLOTH_*`)."""
    from mathutils.bvhtree import BVHTree
    from mathutils.kdtree import KDTree
    tree = BVHTree.FromPolygons([Vector(p) for p in points], [tuple(int(c) for c in face) for face in faces])
    normals = underwear.normals_of(body_points, body_faces)
    covered = np.zeros(len(body_points), dtype=bool)
    off = np.zeros(len(body_points))
    for v, (p, n) in enumerate(zip(body_points, normals)):
        start = Vector(p - n * CLOTH_BELOW)
        hit, _, _, distance = tree.ray_cast(start, Vector(n), CLOTH_REACH + CLOTH_BELOW)
        if hit is not None:
            covered[v] = True
            off[v] = max(distance - CLOTH_BELOW, 0.0)
    bare = np.nonzero(~covered)[0]
    clear = covered.copy()
    if len(bare):
        near = KDTree(len(bare))
        for k, v in enumerate(bare):
            near.insert(Vector(body_points[v]), k)
        near.balance()
        for v in np.nonzero(covered)[0]:
            setback = min(max(CLOTH_GAIN * off[v], CLOTH_LEAST), CLOTH_MOST)
            _, _, distance = near.find(Vector(body_points[v]))
            clear[v] = distance > setback
    return covered, clear


def write_cover(path, body, hidden, under=None, whole=None, size=1024):
    """An item's cover map (ADR-0045), over the body's texture
    coordinates: red where the body's skin is left out under the garment,
    green where a garment worn beneath it leaves its cloth out (`under`,
    per body corner) and blue where a top tucked into it does (`whole`,
    per body **face**; as green when None). A face is lit only when every
    corner of it is. Without `under` (a garment nothing is worn beneath)
    green and blue stay dark. Returns the faces lit in red."""
    from PIL import Image, ImageChops, ImageDraw, ImageFilter
    red, green, blue, skin = (Image.new("L", (size, size), 0) for _ in range(4))
    draws = [ImageDraw.Draw(image) for image in (red, green, blue, skin)]
    lit = 0
    for index, (corners, uvs) in enumerate(zip(body.faces, body.uvs)):
        at = [(u * size, (1 - v) * size) for u, v in uvs]
        draws[3].polygon(at, fill=255, outline=255)
        if all(hidden[c] for c in corners):
            draws[0].polygon(at, fill=255)
            lit += 1
        if under is None:
            continue
        beneath = all(under[c] for c in corners)
        if beneath:
            draws[1].polygon(at, fill=255, outline=255)
        if beneath if whole is None else whole[index]:
            draws[2].polygon(at, fill=255, outline=255)
    if under is not None:
        scale = size / 1024
        near = ImageFilter.MaxFilter(2 * max(1, round(COVER_GROW * scale)) + 1)
        far = ImageFilter.MaxFilter(2 * max(1, round(COVER_PAST * scale)) + 1)
        no_skin = ImageChops.invert(skin)

        def grown(channel):
            past = ImageChops.multiply(channel.filter(far), no_skin)
            return ImageChops.lighter(channel.filter(near), past)

        green, blue = grown(green), grown(blue)
    Image.merge("RGB", (red, green, blue)).save(path, optimize=True)
    return lit


def cloth_cover(body, body_points, body_groups, points, faces, hidden, bottom=False):
    """`write_cover`'s (under, whole) for a garment on the body: the skin
    its cloth is over clear of its openings, and for a `bottom` the faces
    round the hips its cloth is over at all. Skin the garment hides
    (`hidden`, per body corner) is under it whatever the line out of it
    met: MakeHuman's clothes dip under skin their files hide, and cloth
    left drawn where the skin isn't would hang in the air."""
    covered, clear = cloth_over(body_points, body.faces, points, faces)
    covered, clear = covered | hidden, clear | hidden
    if not bottom:
        return clear, None
    hips = {manifests.REGION_INDEX[region] for region in HIP_REGIONS}
    whole = []
    for corners, region in zip(body.faces, face_regions(body, body_groups)):
        whole.append(all(covered[c] for c in corners) if region in hips else all(clear[c] for c in corners))
    return clear, whole


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
            if bone not in at:  # a twist bone: made from the bone it is inside
                continue
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
    follows = sorted(shapes)
    # Pressed onto the skin while anything is worn over it (the resolver
    # turns `Shrink_Under` on then, ADR-0046): it is never the underwear
    # that stands through a tight garment's cloth.
    shapes[underwear.SHRINK_UNDER] = garment["shrink"]
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
    under, whole = cloth_cover(build.body, body_points, body_groups, garment["points"], garment["faces"],
                               garment["hidden"])
    lit = write_cover(os.path.join(folder, name + ".cover.png"), build.body, garment["hidden"], under, whole)
    item = {
        "format_version": 1, "id": item_id, "label": label, "kind": "garment",
        "slot": slot, "layer": 0, "tier": "built_in", "bases": [manifests.BASE_ID],
        "model": {"file": name + ".glb", "object": f"{ARMATURE}/{name}"},
        "cover_map": name + ".cover.png",
        "materials": [{"name": "Underwear", "shading": "cloth", "label": "Cloth", "role": "primary"}],
        "fit": {"method": "shapes", "follows": follows},
        "tags": ["underwear"],
        "about": "Cut from the body's own surface, so it follows every body slider.",
        "source": {"author": "Axcenity Engine contributors, from MakeHuman's base mesh (CC0)",
                   "licence": "CC0-1.0"},
        "quality": "draft",
    }
    with open(os.path.join(folder, name + ".item.json"), "w", newline="\n") as f:
        json.dump(item, f, indent=2)
        f.write("\n")
    return {"corners": int(len(garment["points"])), "faces": len(garment["faces"]), "shapes": len(follows),
            "hidden_faces": lit}


def export(path, armature, objects, extras=False, morph_normals=True):
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    for o in objects:
        o.select_set(True)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=True,
        export_skins=True, export_morph=True, export_morph_normal=morph_normals, export_try_sparse_sk=True,
        export_animations=False, export_yup=True, export_def_bones=False, export_leaf_bone=False,
        export_tangents=False, export_image_format="AUTO", export_extras=extras,
        # Attributes of our own (`SKIN_UV`); the exporter writes those
        # whose names start with an underscore.
        export_attributes=True)


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
        self.poses = {}
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

    def pose_at(self, settings):
        """The way into the engine's pose for the MakeHuman body with
        these settings, about that body's own joints."""
        key = key_of(settings)
        if key not in self.poses:
            self.poses[key] = mh.Repose(self.bones, mh.joint_centres(self.cubes, self.samples.at(settings)))
        return self.poses[key]

    def shape_moves(self, name, weights, place, blends=None):
        """The shape `name`'s moves, in the engine's pose and size, of the
        corners with MakeHuman's `weights` that `place` puts on a
        MakeHuman body (all its corners).

        A body shape (`SHAPES`) is the difference of whole bodies, each
        brought into the engine's pose **about its own joints**. Turning
        the difference itself by the default body's bones is not the same
        thing where two bones turn differently (the elbow, which the pose
        straightens) and the joint between them moves: the feminine
        frame's elbow is 12 cm from the masculine one's, and her forearm
        came out a centimetre or two off her upper arm, a notch at every
        elbow. A face shape moves no joint to speak of and is turned as it
        is."""
        terms = SHAPES.get(name)
        if terms is None:
            rest = place(self.rest)
            return mh.Repose.moves(self.pose.blend(weights), place(self.rest + self.moves[name]) - rest) * self.scale
        blends = {} if blends is None else blends
        total = 0.0
        for weight, settings in terms:
            key = key_of(settings)
            if key not in blends:
                blends[key] = mh.Repose.points(self.pose_at(settings).blend(weights),
                                               place(self.samples.at(settings)))
            total = total + weight * blends[key]
        return total * self.scale

    def body_shapes(self):
        out, blends = {}, {}
        for name in self.moves:
            d = self.shape_moves(name, self.body.weights, lambda body: body[self.skin], blends)
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
        shapes, blends = {}, {}
        for name in self.moves:
            d = self.shape_moves(name, piece.weights, clothes.place, blends)
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
        """How far a shape at full weight moves each bone's head (glTF
        axes): the joint cubes move with the body's corners."""
        terms = SHAPES.get(shape)
        if terms is None:
            centres = mh.joint_centres(self.cubes, self.rest + self.moves[shape])
            return head_moves(joints, contract_joints(centres, self.pose, self.stand))
        # A body shape: the difference of its bodies' own joints, each
        # body brought into the pose about them (`shape_moves`).
        moved = {}
        for weight, settings in terms:
            centres = mh.joint_centres(self.cubes, self.samples.at(settings))
            heads = bone_heads(contract_joints(centres, self.pose_at(settings), self.stand))
            for name, head in heads.items():
                moved[name] = moved.get(name, Vector((0.0, 0.0, 0.0))) + weight * head
        return {name: to_gltf(d) for name, d in moved.items() if name != "Root" and d.length > 0.0005}

    def bone_offsets(self, joints, settings):
        """How far each contract bone's head is from the rest body's on
        the body with these settings (glTF axes), for a parameter's `bones`."""
        return head_moves(joints, self.joints(self.samples.at(settings)))


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
    # The feminine frame's joints, as its skin is brought into the pose
    # (the same floor, the same way: `shape_moves`).
    frame_bones = build.shape_bone_offsets(joints, "Frame_Feminine")
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
    objects = [make_object(name, piece, points, shapes, groups,
                           material_of(*material, extras=MATERIAL_EXTRAS.get(material[0])), armature)
               for name, piece, points, shapes, groups, material in pieces]
    export(os.path.join(args.out, PARTS_FILE), armature, objects, extras=True)
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
    # What a person can point at on the face, and the handles on it.
    heads = {name: np.array(head) for name, (head, _, _) in joints.items()}
    report["face"]["features"], report["face"]["handles"] = face.features_and_handles(
        face_parameters, body_shapes, pieces[0][2], heads)
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
