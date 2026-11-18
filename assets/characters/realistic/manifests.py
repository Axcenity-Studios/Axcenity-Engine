"""The realistic base body's Character Data manifest (Character Studio
overhaul CO4; formats in docs/development/character-studio/architecture.md).

Writes `Realistic.base.json` (the base model manifest: parameters,
regions, slots, sockets, materials, skeleton, face) from
`Realistic.build.json` (what the build measured: the frame's bone offsets,
the added bones, the regions) and the objects of `RealisticParts.glb`.
Plain Python, no Blender: `build.py` runs it after writing those files,
and it can be run alone:

    python -I assets/characters/realistic/manifests.py [folder]

Then rebuild the library's catalogue: python -I assets/characters/catalogue.py

Every parameter carries its `about` (tooltip), `feature` and `level`
(R1; the face's come from `faceparams.py`), and the manifest lists the
body's and the face's `features`, with anchors measured on the model by
`anchors.py` (numpy), and the `handles` a person drags on the character
in the viewport (R2), measured by `handles.py`. The presets are
`presets.py`'s.

Slots, sockets and the proportion sliders are the stylized humanoid's
(`../humanoid/manifests.py`), read from there, so an item or a script
that names one fits both bodies. Its items so far are the underwear cut
from the body (`underwear.py`), written as `items/<name>.item.json`: the
`frame` parameter is a choice of Male or Female, and each choice wears its
underwear.
"""

import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
BASE_ID = "axcenity/realistic"
PARTS_FILE = "RealisticParts.glb"
DATA_FILE = "Realistic.build.json"
MANIFEST_FILE = "Realistic.base.json"
FACE_PROFILE = "axcenity"

# The parts always worn with the body.
FACE_PARTS = ["Eyes", "Teeth", "EyeShell", "Lashes", "Brows", "LipSeal"]

GROUPS = [
    ("frame", "Frame", "body"),
    ("build", "Build", "body"),
    # The whole body's proportions before the per-region build: Character
    # Studio draws groups in this order.
    ("proportions", "Proportions", "body"),
    ("build_regions", "Build by region", "body"),
]
BODY_GROUPS = {gid for gid, _, _ in GROUPS}

# How far each proportion slider goes on this body (R3, plan section 0
# decision 8; CO5 had cut them all to 0.85 to 1.15 and the hips to 1 to
# 1.15). Each end is where the ranges sweep (`../humanoid/qa/ranges.py`)
# found the last step that still deforms cleanly on both frames and at the
# corners of muscle and fat, or the population's 0.5th or 99.5th
# percentile (ANSUR II), whichever is nearer to 1. What set each end, with
# the numbers: docs/development/character-studio/refinement/r3/ranges.md,
# whose table a test holds this one to (`avatar`'s `realistic_ranges`).
PROPORTION_RANGE = [0.85, 1.15]  # (a proportion the table below doesn't name)
PROPORTION_RANGES = {
    "height": [0.82, 1.13],
    "leg_length": [0.85, 1.125],
    "arm_length": [0.95, 1.08],
    "shoulder_width": [0.88, 1.12],
    "waist_width": [0.8, 1.175],
    "hip_width": [0.7, 1.2],
    "head_size": [0.88, 1.12],
    "hand_size": [0.82, 1.07],
    "foot_size": [0.96, 1.21],
}
# Sliders whose attribute (`anim`'s body slider) starts at 1: below 1 they
# move no bone and a shape does the work. The hips: `HipWidth` below 1
# pulls the legs in and the thighs cross; `<Frame>_Hips_Narrow`
# (`shapes.py`) narrows the surface instead.
ATTRIBUTE_FROM_ONE = {"hip_width"}

# The underwear (`underwear.py`): object -> (item ID, label, slot, tags).
GARMENTS = {
    "Underwear_BoxerBriefs": ("axcenity/realistic-boxer-briefs", "Boxer briefs", "underwear_bottom",
                              ["underwear"]),
    "Underwear_Briefs": ("axcenity/realistic-briefs", "Briefs", "underwear_bottom", ["underwear"]),
    "Underwear_Bra": ("axcenity/realistic-sports-bra", "Sports bra", "underwear_top", ["underwear"]),
}
# The region each one hides: its mask shape sinks the skin under it.
# The body's region map (`regions.py`'s `MAP_FILE`), beside the model.
REGIONS_MAP = "RealisticRegions.png"
GARMENT_REGIONS = {"Underwear_BoxerBriefs": "under_boxer_briefs", "Underwear_Briefs": "under_briefs",
                   "Underwear_Bra": "under_sports_bra"}
GARMENT_SHADING = {"Underwear": "cloth", "Waistband": "cloth"}
# What each frame wears to start with.
MALE_WEARS = ["axcenity/realistic-boxer-briefs"]
FEMALE_WEARS = ["axcenity/realistic-briefs", "axcenity/realistic-sports-bra"]
SOURCE = {"author": "Axcenity Engine contributors", "licence": "CC0-1.0"}

# Every material of the body and its parts, and how it is lit.
SHADING = {
    "Skin": "skin", "Mouth": "standard", "Iris": "eye", "Sclera": "eye",
    "Teeth": "standard", "Gums": "standard", "Tongue": "standard", "LipSeal": "standard",
    # The face's parts (CO6; eyes.py, lashes.py).
    "Cornea": "eye", "EyeOcclusion": "standard", "TearLine": "standard", "Lashes": "hair", "Brows": "hair",
}


# How much of the skin's defined normal map shows (R5, OPEN-07): muscle
# brings the definition, fat covers it, and the renderer shows
# `definition * (1 - softness)` of it, so abdominal lines show only on
# lean, muscular bodies (`skin.py`).
SKIN_DRIVERS = {
    "muscle": {"material": "Skin", "value": "definition", "from": [-0.5, 0.8], "to": [0, 1]},
    "fat": {"material": "Skin", "value": "softness", "from": [-0.5, 0.5], "to": [0, 1]},
}


# Makeup and markings the skin draws (R5, ADR-0042): each a channel of
# the masks `makeup.py` bakes into the model file, turned on by a
# description's `skin` block. The colours, roughness and opacity a new
# layer starts at; a layer without a colour takes the character's own
# tone (foundation). IDs are the renderer's (`docs/rendering/skin.md`).
SKIN_LAYERS = [
    {"id": "foundation", "label": "Foundation", "kind": "makeup",
     "about": "Evens the skin's tone over the face and makes it more matte; in the character's own tone unless a "
              "colour is set.", "roughness": 0.55, "opacity": 0.6},
    {"id": "blush", "label": "Blush", "kind": "makeup",
     "about": "Colour on the cheeks, swept up toward the temples.",
     "colour": "#c85a62", "roughness": 0.5, "opacity": 0.45},
    {"id": "eyeshadow", "label": "Eyeshadow", "kind": "makeup",
     "about": "Colour on the upper lids, deepest by the lashes and toward the outer corners.",
     "colour": "#6e4c58", "roughness": 0.45, "opacity": 0.6},
    {"id": "eyeliner", "label": "Eyeliner", "kind": "makeup",
     "about": "A line along the upper lashes with a short wing, and a fainter one under the outer lower lid.",
     "colour": "#1a1414", "roughness": 0.4, "opacity": 0.9},
    {"id": "lips", "label": "Lip colour", "kind": "makeup",
     "about": "Colour on the lips; lower roughness is glossier.",
     "colour": "#a8434b", "roughness": 0.3, "opacity": 0.6},
    {"id": "brow_fill", "label": "Brow fill", "kind": "makeup",
     "about": "Colour on the skin under the brows, so they read fuller.",
     "colour": "#4a3428", "roughness": 0.6, "opacity": 0.45},
    {"id": "freckles", "label": "Freckles", "kind": "marking",
     "about": "Small brown spots where the sun falls: the nose and cheeks, shoulders, upper back and forearms. "
              "Amount shows more of them; a new seed shows others.", "amount": 0.45, "opacity": 0.75},
    {"id": "moles", "label": "Moles", "kind": "marking",
     "about": "A few darker spots over the face and body. Amount shows more of them; a new seed shows others.",
     "amount": 0.3, "opacity": 0.9},
    {"id": "scar", "label": "Scar", "kind": "marking",
     "about": "A healed scar down the left cheek: paler and smoother than the skin round it.", "opacity": 0.8},
]


# What each body parameter does, in a sentence (its tooltip, R1), and the
# feature it belongs to. A regional slider's sentence is made from its
# region's words (`REGION_WORDS`).
BODY_ABOUT = {
    "frame": "The body type the character is built on: male or female.",
    "height": "How tall the whole body is.",
    "leg_length": "How long the legs are against the body.",
    "arm_length": "How long the arms are against the body.",
    "shoulder_width": "How wide the shoulders are.",
    "waist_width": "How wide the waist is.",
    "hip_width": "How wide the hips are.",
    "head_size": "How large the head is against the body.",
    "hand_size": "How large the hands are.",
    "foot_size": "How large the feet are.",
    "muscle": "How much muscle the whole body carries, from slight to strong.",
    "fat": "How much fat the whole body carries, from lean to heavy.",
}
BODY_FEATURE = {
    "frame": "body", "height": "body", "muscle": "body", "fat": "body",
    "leg_length": "thighs", "arm_length": "arms", "shoulder_width": "shoulders",
    "waist_width": "waist", "hip_width": "hips", "head_size": "head", "hand_size": "hands",
    "foot_size": "feet",
}
REGION_WORDS = {"chest": "the chest", "shoulders": "the shoulders", "arms": "the arms",
                "abdomen": "the belly", "back": "the back", "hips": "the hips and buttocks",
                "thighs": "the thighs", "calves": "the calves"}
# The quick level (brief §3.3): the body's overall shape. The rest is
# detailed, or advanced where the slider is flagged advanced.
BODY_QUICK = {"frame", "height", "muscle", "fat"}
# The detailed level holds the regional body controls and the main
# proportions (brief §3.3, Level 2 "regional body adjustments"); only the
# hands' and feet's sizes stay advanced. These lose the shared generators'
# `advanced` flag here, so `advanced` and `level` still agree.
BODY_DETAILED = {"leg_length", "arm_length", "shoulder_width", "waist_width", "hip_width", "head_size"}

# The body's features (R1): the whole body, then its regions, found by the
# shapes that move them or by the region map (`anchors.py`). Their anchors
# are at the region's centre (inside the body); `paired` gives a left and
# a right one.
BODY_FEATURES = [
    {"id": "body", "label": "Whole body", "about": "Body type, height, muscle and fat.",
     "shapes": ["Masculine_Fat_Up", "Masculine_Muscle_Up"]},
    {"id": "head", "label": "Head", "about": "The head's size against the body.", "region": "head",
     "regions": ["head"]},
    {"id": "neck", "label": "Neck", "about": "The neck.", "region": "neck", "regions": ["neck"]},
    {"id": "shoulders", "label": "Shoulders", "about": "Their width, muscle and fat.",
     "shapes": ["Masculine_Fat_Shoulders", "Masculine_Muscle_Shoulders"], "paired": True,
     "regions": ["upper_arms_top"]},
    {"id": "chest", "label": "Chest", "about": "Its muscle and fat.",
     "shapes": ["Masculine_Fat_Chest", "Masculine_Muscle_Chest"]},
    {"id": "arms", "label": "Arms", "about": "Their length, muscle and fat.",
     "shapes": ["Masculine_Fat_Arms", "Masculine_Muscle_Arms"], "paired": True,
     "regions": ["upper_arms_mid", "upper_arms_low", "forearms"]},
    {"id": "hands", "label": "Hands", "about": "Their size.", "region": "hands", "paired": True,
     "regions": ["hands"]},
    {"id": "abdomen", "label": "Abdomen", "about": "The belly's muscle and fat.",
     "shapes": ["Masculine_Fat_Abdomen", "Masculine_Muscle_Abdomen"]},
    {"id": "waist", "label": "Waist", "about": "Its width.", "region": "waist", "regions": ["waist"]},
    {"id": "back", "label": "Back", "about": "Its muscle and fat.",
     "shapes": ["Masculine_Fat_Back", "Masculine_Muscle_Back"]},
    {"id": "hips", "label": "Hips", "about": "Their width, and the muscle and fat of the hips and buttocks.",
     "shapes": ["Masculine_Fat_Hips", "Masculine_Muscle_Hips"], "regions": ["hips"]},
    {"id": "thighs", "label": "Thighs", "about": "The legs' length, and the thighs' muscle and fat.",
     "shapes": ["Masculine_Fat_Thighs", "Masculine_Muscle_Thighs"], "paired": True,
     "regions": ["thighs_top", "thighs_low"]},
    {"id": "calves", "label": "Calves", "about": "Their muscle and fat.",
     "shapes": ["Masculine_Fat_Calves", "Masculine_Muscle_Calves"], "paired": True,
     "regions": ["shins_top", "shins_low"]},
    {"id": "feet", "label": "Feet", "about": "Their size.", "region": "feet", "paired": True,
     "regions": ["feet"]},
]


def describe(parameter):
    """A body parameter with its `about`, `feature` and `level` (R1);
    `advanced` stays as it was, so the two agree."""
    pid = parameter["id"]
    if pid in BODY_ABOUT:
        about, feature = BODY_ABOUT[pid], BODY_FEATURE[pid]
    else:
        kind, region = pid.split("_", 1)
        what = "muscle" if kind == "muscle" else "fat"
        about, feature = f"How much {what} is on {REGION_WORDS[region]}, beyond the whole body's.", region
    level = "advanced" if parameter.get("advanced") else "quick" if pid in BODY_QUICK else "detailed"
    out = {}
    for key, value in parameter.items():
        out[key] = value
        if key == "range":
            out.update({"about": about, "feature": feature, "level": level})
    return out


def features(data, folder):
    """The base manifest's `features`: the body's, then the face's, with
    anchors measured on the model (`anchors.py`)."""
    import anchors
    import faceparams
    body = [dict(d, category="body", icon=d["id"]) for d in BODY_FEATURES]
    return anchors.measure(os.path.join(folder, PARTS_FILE), body + faceparams.features(), data["regions"])


def stylized():
    """The stylized humanoid's manifest generator (for what the two share)."""
    path = os.path.join(os.path.dirname(HERE), "humanoid", "manifests.py")
    spec = importlib.util.spec_from_file_location("stylized_manifests", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def base_manifest(data, parts, folder=HERE):
    shared = stylized()
    frame = data["frame"]
    bones = shared.bone_drivers(frame["bones"], frame.get("eye_scale"))
    parameters = [
        # A choice, not a slider: Character Studio shows Male and Female.
        # (Scripts may still set anything between; it blends the frames.)
        {"id": "frame", "label": "Body type", "group": "frame", "range": [0, 1],
         "shapes": [{"shape": "Frame_Feminine"}], "bones": bones,
         "choices": [{"value": 0, "label": "Male", "items": MALE_WEARS},
                     {"value": 1, "label": "Female", "items": FEMALE_WEARS}]},
    ]
    for pid, label, attribute, advanced in shared.PROPORTIONS:
        span = PROPORTION_RANGES.get(pid, PROPORTION_RANGE)
        drives = [max(span[0], 1), span[1]] if pid in ATTRIBUTE_FROM_ONE else span
        parameter = {"id": pid, "label": label, "group": "proportions", "range": span,
                     "default": 1, "attributes": [{"attribute": attribute, "from": drives}]}
        if advanced:
            parameter["advanced"] = True
        parameters.append(parameter)
    # The body's muscle and fat sliders (CO5): every one of their shapes is a
    # corner with the frame (`shapes.py`).
    if HERE not in sys.path:
        sys.path.insert(0, HERE)
    import shapes
    parameters += shapes.parameters()
    for parameter in parameters:
        if parameter["id"] in SKIN_DRIVERS:
            parameter["material_values"] = [SKIN_DRIVERS[parameter["id"]]]
    # The face's parameters (CO6; faceparams.py).
    import faceparams
    parameters += faceparams.parameters(data.get("face", {}).get("face_identity", {}))
    for parameter in parameters:
        if parameter["id"] in BODY_DETAILED or parameter["group"] == "build_regions":
            parameter.pop("advanced", None)
    parameters = [describe(p) if p["group"] in BODY_GROUPS else p for p in parameters]
    body = parts["Body"]
    materials = sorted({m for name in ["Body", *FACE_PARTS] for m in parts[name]["materials"]})
    manifest = {
        "format_version": 1,
        "id": BASE_ID,
        "label": "Realistic",
        "family": "realistic",
        "model": {"file": PARTS_FILE, "body": body["path"],
                  "parts": [parts[name]["path"] for name in FACE_PARTS],
                  # The region map hides what a worn item covers (CO9).
                  "regions_map": REGIONS_MAP},
        "skeleton": {"convention": "humanoid", "height": 1.75, "pose": "a-pose-45",
                     "added_bones": data["added_bones"]},
        "groups": [{"id": i, "label": l, "category": c} for i, l, c in GROUPS] + faceparams.groups(),
        "parameters": parameters,
        "corners": shapes.corners(),
        "regions": [{"id": r["id"], "label": r["label"], "map_index": r["map_index"]}
                    for r in data["regions"]]
        + [{"id": region, "label": f"Under {GARMENTS[name][1].lower()}", "mask_shape": "Mask_" + name}
           for name, region in GARMENT_REGIONS.items()],
        "slots": shared.SLOTS,
        "sockets": [{"name": n, "bone": b} for n, b in shared.SOCKETS],
        "materials": [{"name": m, "shading": SHADING[m]} for m in materials],
        "face": {"profile": FACE_PROFILE, "eye_bones": ["LeftEye", "RightEye"], "jaw_bone": "Jaw"},
        "defaults": {"items": MALE_WEARS},
        "features": features(data, folder),
        "skin_layers": [dict(layer, material="Skin") for layer in SKIN_LAYERS],
    }
    # The handles a person drags in the viewport (R2), measured on the
    # model with the parameters above (`handles.py`).
    import handles
    manifest["handles"] = handles.handles(manifest, os.path.join(folder, PARTS_FILE))
    return manifest


def item_manifests(parts):
    """The underwear's item manifests, by object name."""
    items = {}
    for name, (iid, label, slot, tags) in GARMENTS.items():
        part = parts[name]
        items[name] = {
            "format_version": 1, "id": iid, "label": label, "kind": "garment",
            "slot": slot, "layer": 0, "tier": "built_in", "bases": [BASE_ID],
            "model": {"file": "../" + PARTS_FILE, "object": part["path"]},
            "hides": [GARMENT_REGIONS[name]],
            "materials": [{"name": m, "shading": GARMENT_SHADING[m]} for m in part["materials"]],
            "fit": {"method": "shapes", "follows": part["shapes"]},
            "tags": tags, "source": SOURCE,
        }
    return items


def write(folder):
    shared = stylized()
    with open(os.path.join(folder, DATA_FILE)) as f:
        data = json.load(f)
    parts = shared.objects(os.path.join(folder, PARTS_FILE))
    shared.write_json(os.path.join(folder, MANIFEST_FILE), base_manifest(data, parts, folder))
    item_folder = os.path.join(folder, "items")
    os.makedirs(item_folder, exist_ok=True)
    for item in item_manifests(parts).values():
        file = item["id"].split("/", 1)[1] + ".item.json"
        shared.write_json(os.path.join(item_folder, file), item)


if __name__ == "__main__":
    write(sys.argv[1] if len(sys.argv) > 1 else HERE)
