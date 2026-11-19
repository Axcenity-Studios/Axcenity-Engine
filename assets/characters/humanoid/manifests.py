"""The stylized humanoid's Character Data manifests (Character Studio
overhaul CO1; formats in docs/development/character-studio/architecture.md).

Writes `Humanoid.base.json` (the base model manifest: parameters, regions,
slots, sockets, materials, skeleton, face) and `items/<name>.item.json`
(one per garment and hair style) from `Humanoid.customization.json` and the
objects of `HumanoidParts.glb`. Plain Python, no Blender: `build.py` runs it
after writing those files, and it can be run alone:

    python -I assets/characters/humanoid/manifests.py [folder]

Then rebuild the library's catalogue: python -I assets/characters/catalogue.py
"""

import json
import os
import struct
import sys

BASE_ID = "axcenity/stylized"
PARTS_FILE = "HumanoidParts.glb"
FACE_PROFILE = "axcenity"
SOURCE = {"author": "Axcenity Engine contributors", "licence": "CC0-1.0"}

# The parts always worn with the body.
FACE_PARTS = ["Eyes", "Teeth", "Brows", "Lashes"]

# The sockets `project::HUMANOID_SOCKETS` adds to the default character.
SOCKETS = [
    ("HeadSocket", "Head"),
    ("BackSocket", "UpperChest"),
    ("HipsSocket", "Hips"),
    ("LeftHandSocket", "LeftHand"),
    ("RightHandSocket", "RightHand"),
    ("LeftFootSocket", "LeftFoot"),
    ("RightFootSocket", "RightFoot"),
]

# `anim`'s body sliders (PR #400): MeshPart attributes, factors 0.5 to 1.5.
PROPORTIONS = [
    ("height", "Height", "BodyHeight", False),
    ("leg_length", "Leg length", "LegLength", True),
    ("arm_length", "Arm length", "ArmLength", True),
    ("shoulder_width", "Shoulder width", "ShoulderWidth", True),
    ("waist_width", "Waist width", "WaistWidth", True),
    ("hip_width", "Hip width", "HipWidth", True),
    ("head_size", "Head size", "HeadSize", True),
    ("hand_size", "Hand size", "HandSize", True),
    ("foot_size", "Foot size", "FootSize", True),
]

# How far each proportion slider goes on this body (R3, wave 3: the
# stylized family held to the realistic body's checks; plan section 0
# decision 8). Each end is where the ranges sweep
# (`qa/ranges.py --package stylized`) found the last step that still
# deforms cleanly on both frames and at the corners of `build` and
# `muscle`, or the population's 0.5th or 99.5th percentile (ANSUR II),
# whichever is nearer to 1. What set each end:
# docs/development/character-studio/refinement/r3/ranges.md ("Stylized
# shipped ranges"), which a test holds this table to (`avatar`'s
# `realistic_ranges`). Two ends are nearer 1 than the sweep's, set by
# `measure.py`'s sliders pass (every pair of sliders at their ends): the
# hips don't narrow (by the bone, at 0.975 with `build` 1, the inner thighs
# cross), and the head stops at 1.025 (at 1.05 with `build` 1, the chin's
# fat meets the neck).
PROPORTION_RANGES = {
    "height": [0.85, 1.15],
    "leg_length": [0.81, 1.275],
    "arm_length": [0.92, 1.08],
    "shoulder_width": [0.95, 1.12],
    "waist_width": [0.775, 1.225],
    "hip_width": [0.7, 1.15],
    "head_size": [0.87, 1.125],
    "hand_size": [0.81, 1.11],
    "foot_size": [0.98, 1.27],
}

# Sliders whose attribute (`anim`'s body slider) starts at 1: below 1 they
# move no bone and a shape does the work (review fixes 1, as the realistic
# body's `ATTRIBUTE_FROM_ONE`). The hips: `HipWidth` below 1 pulls the legs
# in and the heavy inner thighs cross; `<Frame>_Hips_Narrow`
# (`bodyshapes.hips_narrow`) narrows the surface instead, whole at
# `NARROW_AT`, each frame's driven with the frame (`corners`).
ATTRIBUTE_FROM_ONE = {"hip_width"}
NARROW_AT = 0.6


def corners():
    """The manifest's `corners`: each frame's hips narrowing."""
    out = []
    for frame, to in (("Masculine", [1, 0]), ("Feminine", [0, 1])):
        out.append({"shape": f"{frame}_Hips_Narrow",
                    "factors": [{"parameter": "frame", "from": [0, 1], "to": to},
                                {"parameter": "hip_width", "from": [1, NARROW_AT], "to": [0, 1]}]})
    return out


# Face identity shapes: (shape, label, group). Their range is -1 to 1.
FACE = [
    ("Face_Eyes_Spacing", "Eye spacing", "eyes"),
    ("Face_Eyes_Height", "Eye height", "eyes"),
    ("Face_Eyes_Size", "Eye size", "eyes"),
    ("Face_Eyes_Tilt", "Eye tilt", "eyes"),
    ("Face_Brows_Height", "Brow height", "brows"),
    ("Face_Nose_Width", "Nose width", "nose"),
    ("Face_Nose_Length", "Nose length", "nose"),
    ("Face_Nose_Bridge", "Nose bridge", "nose"),
    ("Face_Nose_Tip", "Nose tip", "nose"),
    ("Face_Mouth_Width", "Mouth width", "mouth"),
    ("Face_Lips_Full", "Lip fullness", "mouth"),
    ("Face_Chin_Length", "Chin length", "jaw"),
    ("Face_Chin_Forward", "Chin forward", "jaw"),
    ("Face_Jaw_Width", "Jaw width", "jaw"),
    ("Face_Cheekbones", "Cheekbones", "jaw"),
    ("Face_Head_Width", "Head width", "head"),
    ("Face_Head_Length", "Head length", "head"),
    ("Face_Ears_Size", "Ear size", "head"),
    ("Face_Ears_Out", "Ears out", "head"),
]

GROUPS = [
    ("frame", "Frame", "body"),
    ("build", "Build", "body"),
    ("proportions", "Proportions", "body"),
    ("eyes", "Eyes", "face"),
    ("brows", "Brows", "face"),
    ("nose", "Nose", "face"),
    ("mouth", "Mouth", "face"),
    ("jaw", "Jaw, chin and cheeks", "face"),
    ("head", "Head and ears", "face"),
]

SLOTS = [
    {"id": "hair", "label": "Hairstyle", "category": "hair", "empty_label": "Bald"},
    {"id": "underwear_top", "label": "Underwear top", "category": "clothing"},
    {"id": "underwear_bottom", "label": "Underwear", "category": "clothing"},
    {"id": "top", "label": "Top", "category": "clothing"},
    {"id": "outerwear", "label": "Outerwear", "category": "clothing"},
    {"id": "bottom", "label": "Bottoms", "category": "clothing"},
    {"id": "shoes", "label": "Shoes", "category": "clothing"},
]

# Every material of the parts file and how it is lit.
SHADING = {
    "Skin": "skin", "Mouth": "standard", "Iris": "eye", "Sclera": "eye",
    "Teeth": "standard", "Gums": "standard", "Brow": "hair", "Lash": "hair",
    "Shirt": "cloth", "Jacket": "cloth", "JacketRib": "cloth", "JacketTrim": "cloth",
    "Trousers": "cloth", "Shoes": "cloth", "ShoeAccent": "standard", "Sole": "standard",
    "Laces": "cloth", "Underwear": "cloth", "Hair": "hair", "HairSecondary": "hair",
}

# Garments: object -> (item id, label, slot, layer, tags). Hair styles:
# object -> (item id, label, tags). The IDs are `avatar::V1_ITEMS`'.
GARMENTS = {
    "Shirt_Tee": ("axcenity/tee", "T-shirt", "top", 1, ["casual", "cotton", "short-sleeve"]),
    "Jacket_Cropped": ("axcenity/cropped-jacket", "Cropped jacket", "outerwear", 3,
                       ["casual", "cropped", "zip", "ribbed"]),
    "Pants_Cargo": ("axcenity/cargo-pants", "Cargo pants", "bottom", 1,
                    ["casual", "joggers", "pockets"]),
    "Shoes_Sneakers": ("axcenity/sneakers", "Sneakers", "shoes", 1, ["casual", "trainers"]),
    "Underwear_Briefs": ("axcenity/briefs", "Briefs", "underwear_bottom", 0, ["underwear"]),
    "Underwear_Top": ("axcenity/sports-top", "Sports top", "underwear_top", 0,
                      ["underwear", "athletic"]),
}
HAIR = {
    "Hair_Swept": ("axcenity/hair-swept", "Swept", ["short", "straight", "side-part"]),
    "Hair_Bob": ("axcenity/hair-bob", "Bob", ["medium", "straight"]),
    "Hair_Long": ("axcenity/hair-long", "Long", ["long", "straight"]),
    "Hair_Crop": ("axcenity/hair-crop", "Crop", ["short"]),
    "Hair_Buzz": ("axcenity/hair-buzz", "Buzz", ["very-short"]),
    "Hair_Ponytail": ("axcenity/hair-ponytail", "Ponytail", ["long", "tied"]),
}


def glb_json(path):
    with open(path, "rb") as f:
        data = f.read()
    length = struct.unpack("<I", data[12:16])[0]
    return json.loads(data[20:20 + length])


def objects(path):
    """Each mesh object's path in the file (parents' names joined by `/`,
    as a `MeshId` pick), its shapes and its materials, by name."""
    gltf = glb_json(path)
    nodes = gltf["nodes"]
    parent = {}
    for index, node in enumerate(nodes):
        for child in node.get("children", []):
            parent[child] = index
    out = {}
    for index, node in enumerate(nodes):
        if "mesh" not in node:
            continue
        names, at = [], index
        while at is not None:
            names.append(nodes[at]["name"])
            at = parent.get(at)
        mesh = gltf["meshes"][node["mesh"]]
        materials = sorted({gltf["materials"][p["material"]]["name"]
                            for p in mesh["primitives"] if "material" in p})
        out[node["name"]] = {
            "path": "/".join(reversed(names)),
            "shapes": (mesh.get("extras") or {}).get("targetNames", []),
            "materials": materials,
        }
    return out


def snake(name):
    out = ""
    for i, c in enumerate(name):
        if c.isupper() and i and name[i - 1] != "_":
            out += "_"
        out += c.lower()
    return out


def region_id(mask):
    return snake(mask[len("Mask_"):])


def region_label(mask):
    words = mask[len("Mask_"):].split("_")
    label = {"UpperArms": "Upper arms"}.get(words[0], words[0])
    return label + (f" ({words[1].lower()})" if len(words) > 1 else "")


def bone_drivers(bones, extra_scale=None):
    """Customization bones ({bone: {"offset": [...]} or {"scale": s}}) as
    bone drivers; `extra_scale` scales the eye bones too."""
    drivers = []
    for bone in sorted(bones):
        change = bones[bone]
        driver = {"bone": bone}
        if "offset" in change:
            driver["offset"] = change["offset"]
        if "scale" in change:
            driver["scale"] = change["scale"]
        if extra_scale and bone in ("LeftEye", "RightEye"):
            driver["scale"] = extra_scale
        drivers.append(driver)
    if extra_scale:
        for bone in ("LeftEye", "RightEye"):
            if bone not in bones:
                drivers.append({"bone": bone, "scale": extra_scale})
    return drivers


def base_manifest(custom, parts):
    frame = custom["body"]["Frame_Feminine"]
    parameters = [
        {"id": "frame", "label": "Masculine to feminine", "group": "frame", "range": [0, 1],
         "shapes": [{"shape": "Frame_Feminine"}],
         "bones": bone_drivers(frame["bones"], frame.get("eye_scale"))},
        {"id": "build", "label": "Slim to heavy", "group": "build", "range": [-1, 1],
         "shapes": [{"shape": "Build_Heavy"},
                    {"shape": "Build_Slim", "from": [0, -1], "to": [0, 1]}]},
        {"id": "muscle", "label": "Muscle", "group": "build", "range": [0, 1],
         "shapes": [{"shape": "Build_Muscular"}]},
    ]
    for pid, label, attribute, advanced in PROPORTIONS:
        span = PROPORTION_RANGES[pid]
        drives = [max(span[0], 1), span[1]] if pid in ATTRIBUTE_FROM_ONE else span
        parameter = {"id": pid, "label": label, "group": "proportions", "range": span,
                     "default": 1, "attributes": [{"attribute": attribute, "from": drives}]}
        if advanced:
            parameter["advanced"] = True
        parameters.append(parameter)
    for shape, label, group in FACE:
        parameter = {"id": snake(shape[len("Face_"):]), "label": label, "group": group,
                     "range": [-1, 1], "shapes": [{"shape": shape, "from": [-1, 1]}]}
        bones = custom["face"].get(shape, {}).get("bones", {})
        if bones:
            parameter["bones"] = [dict(d, **{"from": [-1, 1]}) for d in bone_drivers(bones)]
        parameters.append(parameter)
    body = parts["Body"]
    materials = sorted({m for name in ["Body", *FACE_PARTS] for m in parts[name]["materials"]})
    default_items = [HAIR[custom["default"]["hair"]][0]]
    default_items += [GARMENTS[name][0] for name in custom["default"]["outfit"]]
    return {
        "format_version": 1,
        "id": BASE_ID,
        "label": "Stylized",
        "family": "stylized",
        "model": {"file": PARTS_FILE, "body": body["path"],
                  "parts": [parts[name]["path"] for name in FACE_PARTS]},
        "skeleton": {"convention": "humanoid", "height": 1.75, "pose": "a-pose-45"},
        "groups": [{"id": i, "label": l, "category": c} for i, l, c in GROUPS],
        "parameters": parameters,
        "corners": corners(),
        "regions": [{"id": region_id(m), "label": region_label(m), "mask_shape": m}
                    for m in custom["masks"]],
        "slots": SLOTS,
        "sockets": [{"name": n, "bone": b} for n, b in SOCKETS],
        "materials": [{"name": m, "shading": SHADING[m]} for m in materials],
        "face": {"profile": FACE_PROFILE, "eye_bones": ["LeftEye", "RightEye"],
                 "jaw_bone": "Jaw"},
        "defaults": {"items": default_items},
    }


def ref(part):
    return {"file": "../" + PARTS_FILE, "object": part["path"]}


# The stylized family's items are Legacy (Paul's decision 12 of
# 2026-11-18, ADR-0043): older content kept for compatibility and optional
# use, hidden by default in Character Studio and never drawn by Randomize.
QUALITY = "legacy"


def item_manifests(custom, parts):
    items = {}
    for name, (iid, label, slot, layer, tags) in GARMENTS.items():
        part = parts[name]
        items[name] = {
            "format_version": 1, "id": iid, "label": label, "kind": "garment",
            "slot": slot, "layer": layer, "tier": "built_in", "bases": [BASE_ID],
            "model": ref(part),
            "hides": [region_id(m) for m in custom["parts"][name]["masks"]],
            "materials": [{"name": m, "shading": SHADING[m]} for m in part["materials"]],
            "fit": {"method": "shapes", "follows": part["shapes"]},
            "tags": tags, "source": SOURCE, "quality": QUALITY,
        }
    for name, (iid, label, tags) in HAIR.items():
        part = parts[name]
        items[name] = {
            "format_version": 1, "id": iid, "label": label, "kind": "groom",
            "slot": "hair", "tier": "built_in", "bases": [BASE_ID],
            "materials": [{"name": m, "shading": SHADING[m]} for m in part["materials"]],
            "fit": {"method": "shapes", "follows": part["shapes"]},
            "groom": {"mesh": ref(part)},
            "tags": tags, "source": SOURCE, "quality": QUALITY,
        }
    return items


def write_json(path, value):
    with open(path, "w", newline="\n") as f:
        json.dump(value, f, indent=2)
        f.write("\n")


def write(folder):
    with open(os.path.join(folder, "Humanoid.customization.json")) as f:
        custom = json.load(f)
    parts = objects(os.path.join(folder, PARTS_FILE))
    write_json(os.path.join(folder, "Humanoid.base.json"), base_manifest(custom, parts))
    item_folder = os.path.join(folder, "items")
    os.makedirs(item_folder, exist_ok=True)
    for name, item in item_manifests(custom, parts).items():
        file = item["id"].split("/", 1)[1] + ".item.json"
        write_json(os.path.join(item_folder, file), item)


if __name__ == "__main__":
    write(sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.abspath(__file__)))
