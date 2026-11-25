"""Writes the human base's manifest (`Human.base.json`) from what `build.py`
recorded (`Human.build.json`). Plain Python; `build.py` calls it, and it
runs alone after a change here:

    python -I assets/characters/human/manifests.py [--folder <built folder>]

The schema is `docs/development/character-studio/architecture.md` section 6.
"""

import argparse
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
BASE_ID = "axcenity/human"
ARMATURE = "AxcenityHuman"
PARTS_FILE = "HumanParts.glb"
DATA_FILE = "Human.build.json"
MANIFEST = "Human.base.json"
REGIONS_MAP = "HumanRegions.png"
CREDIT = "MakeHuman (makehumancommunity.org), fitted with MPFB; Axcenity Engine contributors"

# (id, label), by map index from 1: the realistic family's regions, so an
# item's `hides` means the same on this body.
REGIONS = [
    ("head", "Head"), ("neck", "Neck"), ("torso", "Torso"), ("waist", "Waist"), ("hips", "Hips"),
    ("upper_arms_top", "Upper arms (top)"), ("upper_arms_low", "Upper arms (low)"),
    ("forearms", "Forearms"), ("hands", "Hands"), ("thighs_top", "Thighs (top)"),
    ("thighs_low", "Thighs (low)"), ("shins_top", "Shins (top)"), ("shins_low", "Shins (low)"),
    ("feet", "Feet"),
]
REGION_INDEX = {name: k + 1 for k, (name, _) in enumerate(REGIONS)}

GROUPS = [
    {"id": "frame", "label": "Frame", "category": "body"},
    {"id": "build", "label": "Build", "category": "body"},
    {"id": "proportions", "label": "Proportions", "category": "body"},
]
# The Skin and Makeup page: the skin choice (`skins.py`).
SKIN_GROUP = {"id": "skin", "label": "Skin", "category": "skin",
              "about": "The photographed skin the body wears."}
# The face's pages, in order (`face.SECTIONS`).
FACE_GROUPS = [
    {"id": "face_shape", "label": "Face shape", "category": "face"},
    {"id": "forehead", "label": "Forehead", "category": "face"},
    {"id": "brows", "label": "Brows", "category": "face"},
    {"id": "eyes", "label": "Eyes", "category": "face"},
    {"id": "nose", "label": "Nose", "category": "face"},
    {"id": "cheeks", "label": "Cheeks", "category": "face"},
    {"id": "lips", "label": "Lips", "category": "face"},
    {"id": "chin", "label": "Chin and jaw", "category": "face"},
    {"id": "ears", "label": "Ears", "category": "face"},
    {"id": "neck", "label": "Neck", "category": "face"},
]
SLOTS = [
    # Every hairstyle is still a draft (`hair.py`), and Randomize draws no
    # draft unless the slot says so: without this a random character is bald.
    {"id": "hair", "label": "Hairstyle", "category": "hair", "empty_label": "Bald", "random_drafts": True},
    {"id": "underwear_top", "label": "Underwear top", "category": "clothing"},
    {"id": "underwear_bottom", "label": "Underwear", "category": "clothing"},
    {"id": "top", "label": "Top", "category": "clothing"},
    {"id": "outerwear", "label": "Outerwear", "category": "clothing"},
    {"id": "bottom", "label": "Bottoms", "category": "clothing"},
    {"id": "shoes", "label": "Shoes", "category": "clothing"},
    {"id": "legwear", "label": "Socks and stockings", "category": "clothing"},
    {"id": "gloves", "label": "Gloves", "category": "clothing"},
    {"id": "headwear", "label": "Headwear", "category": "accessories"},
]
SOCKETS = [
    {"name": "HeadSocket", "bone": "Head"}, {"name": "BackSocket", "bone": "UpperChest"},
    {"name": "HipsSocket", "bone": "Hips"}, {"name": "LeftHandSocket", "bone": "LeftHand"},
    {"name": "RightHandSocket", "bone": "RightHand"}, {"name": "LeftFootSocket", "bone": "LeftFoot"},
    {"name": "RightFootSocket", "bone": "RightFoot"},
]
# The body sliders `anim` applies after the pose (architecture section 6,
# `attributes`), as the realistic family offered them.
PROPORTIONS = [
    ("height", "Height", "BodyHeight", (0.82, 1.13), "quick", "How tall the character stands."),
    ("leg_length", "Leg length", "LegLength", (0.85, 1.125), "detailed", "How long the legs are for the body."),
    ("arm_length", "Arm length", "ArmLength", (0.95, 1.08), "detailed", "How long the arms are for the body."),
    ("shoulder_width", "Shoulder width", "ShoulderWidth", (0.88, 1.12), "detailed", "How wide the shoulders are."),
    ("waist_width", "Waist width", "WaistWidth", (0.8, 1.175), "detailed", "How wide the waist is."),
    ("hip_width", "Hip width", "HipWidth", (0.85, 1.15), "detailed", "How wide the hips are."),
    ("head_size", "Head size", "HeadSize", (0.9, 1.1), "detailed", "How large the head is for the body."),
]
MATERIAL_SHADING = {"Skin": "skin", "Eyes": "standard", "Teeth": "standard", "Tongue": "standard",
                    "Brows": "hair", "Lashes": "hair"}
# The values of its shading model a slot offers a person. The skin: its
# tone over whichever photographed skin is drawn, and how it takes light.
# Not the Skin model's `definition`, `softness` and `detail_strength`: they
# need the normal maps MakeHuman's skins don't have.
MATERIAL_VALUES = {
    "Skin": ["melanin", "undertone", "redness", "tint", "roughness_scale", "scatter_scale"],
    # Brows and lashes are painted cards: their colour along the pigment
    # curve, or dyed.
    "Brows": ["melanin", "redness", "dye", "dye_amount", "tint"],
    "Lashes": ["melanin", "redness", "dye", "dye_amount", "tint"],
}
MATERIAL_LABELS = {"Skin": "Skin", "Brows": "Brows", "Lashes": "Lashes"}

# What a new character wears: the boxer briefs cut from the body
# (`underwear.py`; MakeHuman's packs have no men's underwear).
BOXER_BRIEFS = "axcenity/human-boxer-briefs"
BANDEAU = "axcenity/human-bandeau"
MALE_WEARS = [BOXER_BRIEFS]
FEMALE_WEARS = [BOXER_BRIEFS, BANDEAU]
#: The garments shipped beside the base (the rest are downloaded: README).
SHIPPED = ("male_casualsuit01", "shoes01", "female_casualsuit01", "shoes04")

MASCULINE = {"parameter": "frame", "from": [0, 1], "to": [1, 0]}
FEMININE = {"parameter": "frame", "from": [0, 1], "to": [0, 1]}
UP = {"from": [0, 1], "to": [0, 1]}
DOWN = {"from": [0, -1], "to": [0, 1]}


def corners(shapes):
    """The corner drivers of the frame's own muscle, fat and age shapes:
    MakeHuman mixes a target per gender, muscle and weight, so each is
    driven by the product of the sliders it belongs to (section 7)."""
    out = []
    for frame, factor in (("Masculine", MASCULINE), ("Feminine", FEMININE)):
        for way, span in (("Up", UP), ("Down", DOWN)):
            out.append((f"{frame}_Muscle_{way}", [factor, {"parameter": "muscle", **span}]))
            out.append((f"{frame}_Fat_{way}", [factor, {"parameter": "fat", **span}]))
        for mw, ms in (("Up", UP), ("Down", DOWN)):
            for fw, fs in (("Up", UP), ("Down", DOWN)):
                out.append((f"{frame}_Muscle_{mw}_Fat_{fw}",
                            [factor, {"parameter": "muscle", **ms}, {"parameter": "fat", **fs}]))
        out.append((f"{frame}_Age_Old", [factor, {"parameter": "age", **UP}]))
    return [{"shape": shape, "factors": factors} for shape, factors in out if shape in shapes]


def base_manifest(data):
    shapes = set(data["shapes"])
    frame = {
        "id": "frame", "label": "Body type", "group": "frame", "range": [0, 1], "level": "quick",
        "about": "The body type the character is built on: male or female.",
        "shapes": [{"shape": "Frame_Feminine"}],
        "bones": [{"bone": bone, "offset": offset} for bone, offset in sorted(data["frame_bones"].items())],
        "choices": [{"value": 0, "label": "Male", "items": MALE_WEARS},
                    {"value": 1, "label": "Female", "items": FEMALE_WEARS}],
    }
    parameters = [
        frame,
        {"id": "muscle", "label": "Muscle", "group": "build", "range": [-1, 1], "level": "quick",
         "about": "How much muscle the whole body carries, from slight to strong."},
        {"id": "fat", "label": "Body fat", "group": "build", "range": [-1, 1], "level": "quick",
         "about": "How much fat the whole body carries, from lean to heavy."},
        # Randomize draws any age (by default it would keep to the middle half
        # of the slider, and so to the middle-aged skins: `skins.AGES`).
        {"id": "age", "label": "Age", "group": "build", "range": [0, 1], "level": "quick", "random": [0, 1],
         "about": "How old the body looks, from a young adult to an old one. The skin ages with it."},
    ]
    for pid, label, attribute, span, level, about in PROPORTIONS:
        parameters.append({"id": pid, "label": label, "group": "proportions", "range": list(span),
                           "default": 1, "level": level, "about": about,
                           "attributes": [{"attribute": attribute, "from": list(span)}]})
    face = data.get("face", {})
    for spec in face.get("parameters", []):
        low, high = spec.get("low"), spec.get("high")
        p = {"id": spec["id"], "label": spec["label"], "group": spec["group"],
             "range": [-1 if low else 0, 1 if high else 0], "level": spec.get("level", "detailed"),
             "about": spec["about"],
             "random": [-0.3 if low else 0, 0.3 if high else 0]}
        if "mirror" in spec:
            p["mirror"] = spec["mirror"]
        p["shapes"], bones = [], []
        for shape, span in ((low, {"from": [0, -1], "to": [0, 1]}), (high, {"from": [0, 1]})):
            if shape:
                p["shapes"].append({"shape": shape, **span})
                bones += [{"bone": bone, **span, "offset": offset}
                          for bone, offset in sorted(face.get("bones", {}).get(shape, {}).items())]
        if bones:
            p["bones"] = bones
        parameters.append(p)
    # The skin (ADR-0048): one choice a person makes. Which of its skins
    # is drawn follows the body type and the age (`material_maps`).
    skins = data.get("skins", {})
    if skins.get("maps"):
        parameters.append({
            "id": "skin", "label": "Skin", "group": "skin", "level": "quick",
            "range": [skins["choices"][0]["value"], skins["choices"][-1]["value"]],
            "default": skins["default"],
            # Randomize draws the photographed skins, not the painted bodysuit.
            "random": [skins["choices"][0]["value"], skins["last_random"]],
            "about": "The skin the body wears, one of MakeHuman's photographed skins. "
                     "It is the male or the female one with the body type, and the young, "
                     "middle-aged or old one with Age.",
            "choices": [{"value": c["value"], "label": c["label"]} for c in skins["choices"]],
        })
    for p in parameters:
        p["advanced"] = p["level"] == "advanced"
    parts = [f"{ARMATURE}/{name}" for name in data["objects"] if name != "Body"]
    return {
        "format_version": 1,
        "id": BASE_ID,
        "label": "Human",
        # Its own family while the realistic base it replaces still ships
        # (ADR-0047): benchmarks and presets are named by family.
        "family": "human",
        "model": {"file": PARTS_FILE, "body": f"{ARMATURE}/Body", "parts": parts, "regions_map": REGIONS_MAP},
        "skeleton": {"convention": "humanoid", "height": 1.75, "pose": "a-pose-45",
                     "added_bones": data["added_bones"]},
        "groups": GROUPS + [g for g in FACE_GROUPS + [SKIN_GROUP]
                            if any(p["group"] == g["id"] for p in parameters)],
        "parameters": parameters,
        "corners": corners(shapes),
        "regions": [{"id": rid, "label": label, "map_index": k + 1} for k, (rid, label) in enumerate(REGIONS)],
        "slots": SLOTS,
        "sockets": SOCKETS,
        "materials": [{"name": name, "shading": MATERIAL_SHADING.get(name, "standard"),
                       **({"label": MATERIAL_LABELS[name]} if name in MATERIAL_LABELS else {}),
                       **({"values": MATERIAL_VALUES[name]} if name in MATERIAL_VALUES else {})}
                      for name in sorted(data["materials"])],
        "face": {"profile": "axcenity", "eye_bones": ["LeftEye", "RightEye"], "jaw_bone": "Jaw"},
        "defaults": {"items": MALE_WEARS},
        **({"material_maps": skins["maps"]} if skins.get("maps") else {}),
        **({"features": face["features"]} if face.get("features") else {}),
        **({"handles": face["handles"]} if face.get("handles") else {}),
    }


def write(folder):
    with open(os.path.join(folder, DATA_FILE)) as f:
        data = json.load(f)
    with open(os.path.join(folder, MANIFEST), "w", newline="\n") as f:
        json.dump(base_manifest(data), f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--folder", default=HERE)
    write(parser.parse_args().folder)
