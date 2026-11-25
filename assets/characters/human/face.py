"""The human base's face: what it can do (expression shapes for the
engine's standard facial controls) and who it is (identity parameters),
both from MakeHuman's targets (CC0 data read from the installed MPFB
add-on's `data/targets`; `mh.read_target`).

**Expressions.** MakeHuman's expression units (`expression/units`, a set
per ethnicity, mixed in equal thirds as the body's rest is) become blend
shapes named after the controls they play, which is what the built-in
`axcenity` face profile looks for (`anim::face`, architecture.md section
13). A unit that moves both sides of the mouth is split into a left and a
right shape across the middle of the face. `CONTROLS` is the mapping,
chosen from what each unit moves (measured on 2026-11-24: the corner
puller takes the mouth's corners out and up 6.5 mm, a smile; the
depression takes them down, a frown; pursing draws the lips in and
9 mm forward). The jaw opens by its bone, so `Jaw_Open` has no shape, and
`Mouth_Open` is the lips parting. Controls with no unit behind them are
in `UNMAPPED`.

**Identity.** MakeHuman's own list of sliders (`target.json`: a pair of
opposite targets each, one for each side where the feature is paired) for
the head, forehead, brows, eyes, nose, mouth, cheeks, chin, ears and
neck becomes one parameter a slider (two, mirrored, for a pair), each
driving its two targets as shapes. Labels are made from the targets'
names (`label_of`).

Plain Python and NumPy.
"""

import json
import os
import re

import numpy as np

RACES = ("african", "asian", "caucasian")
# How far either side of the face's middle a both-sided unit fades from
# one side's shape to the other's (metres).
SPLIT = 0.012

# control -> [(unit, weight, side)]: side "" for the whole unit, "L" or
# "R" for that half of a unit that moves both sides.
CONTROLS = {}
for _side, _name in (("left", "Left"), ("right", "Right")):
    _half = _name[0]
    CONTROLS.update({
        f"Blink_{_name}": [(f"eye-{_side}-closure", 1.0, "")],
        f"Eye_Closed_{_name}": [(f"eye-{_side}-closure", 1.0, "")],
        f"Eye_Squint_{_name}": [(f"eye-{_side}-slit", 1.0, "")],
        f"Eye_Wide_{_name}": [(f"eye-{_side}-opened-up", 1.0, "")],
        f"Brow_Raise_{_name}": [(f"eyebrows-{_side}-up", 1.0, "")],
        f"Brow_Lower_{_name}": [(f"eyebrows-{_side}-down", 1.0, "")],
        f"Nose_Sneer_{_name}": [(f"nose-{_side}-elevation", 1.0, "")],
        f"Mouth_Smile_{_name}": [("mouth-corner-puller", 1.0, _half)],
        f"Mouth_Frown_{_name}": [("mouth-depression", 1.0, _half)],
        f"Lip_Corner_Pull_{_name}": [("mouth-retraction", 1.0, _half)],
        f"Upper_Lip_Raise_{_name}": [("mouth-upward-retraction", 1.0, _half)],
        f"Lower_Lip_Lower_{_name}": [("mouth-depression-retraction", 1.0, _half)],
    })
CONTROLS.update({
    "Mouth_Open": [("mouth-parling", 1.0, "")],
    "Mouth_Wide": [("mouth-retraction", 1.0, "")],
    "Mouth_Narrow": [("mouth-pursing", 0.5, "")],
    "Mouth_Pucker": [("mouth-pursing", 1.0, "")],
    "Upper_Lip_Raise": [("mouth-upward-retraction", 1.0, "")],
    "Lower_Lip_Lower": [("mouth-depression-retraction", 1.0, "")],
})
#: Standard controls MakeHuman has no unit for (the eyes look by their bones).
UNMAPPED = ("Brow_Inward_Left", "Brow_Inward_Right", "Mouth_Left", "Mouth_Right", "Cheek_Raise_Left",
            "Cheek_Raise_Right", "Cheek_Puff", "Jaw_Forward", "Jaw_Left", "Jaw_Right", "Tongue_Out",
            "Tongue_Up", "Tongue_Down")

# The identity sections, in page order: (MakeHuman's section, group ID, label).
SECTIONS = (
    ("head", "face_shape", "Face shape"), ("forehead", "forehead", "Forehead"), ("eyebrows", "brows", "Brows"),
    ("eyes", "eyes", "Eyes"), ("nose", "nose", "Nose"), ("cheek", "cheeks", "Cheeks"),
    ("mouth", "lips", "Lips"), ("chin", "chin", "Chin and jaw"), ("ears", "ears", "Ears"), ("neck", "neck", "Neck"),
)
# The ends of a slider, by how MakeHuman names its pair: (low word, high word).
PAIRS = {
    ("decr", "incr"): ("less", "more"), ("in", "out"): ("in", "out"), ("down", "up"): ("lower", "higher"),
    ("backward", "forward"): ("back", "forward"), ("concave", "convex"): ("concave", "convex"),
    ("compress", "uncompress"): ("pinched", "open"), ("square", "round"): ("square", "round"),
    ("pointed", "triangle"): ("pointed", "triangular"),
}
WORDS = {
    "scale-horiz": "width", "scale-vert": "height", "scale-depth": "depth", "scale": "size",
    "trans": "position", "rot": "tilt", "circ": "girth", "eyefold": "lid fold", "cupidsbow": "cupid's bow",
    "upperlip": "upper lip", "lowerlip": "lower lip", "septumangle": "septum angle", "ext": "outer part",
    "invertedtriangular": "heart-shaped", "prognathism": "jaw forward",
}
SECTION_WORD = {"head": "Head", "forehead": "Forehead", "eyebrows": "Brow", "eyes": "Eye", "nose": "Nose",
                "cheek": "Cheek", "mouth": "Mouth", "chin": "Chin", "ears": "Ear", "neck": "Neck"}


def shape_name(target):
    """A MakeHuman target's name as a blend shape's: `l-eye-scale-incr` is
    `Face_Eye_Scale_Incr_Left`."""
    words = target.split("-")
    side = {"l": "Left", "r": "Right"}.get(words[0])
    if side:
        words = words[1:]
    return "_".join(["Face"] + [w.capitalize() for w in words] + ([side] if side else []))


def label_of(section, name):
    """(label, about) for a slider named as MakeHuman names it
    (`nose-scale-vert-decr-incr`)."""
    words = name.split("-")
    if words[0] == "measure":
        words = words[1:]
    # The section's own word leads the name ("nose-", "eye-", "ear-").
    lead = {"eyes": "eye", "ears": "ear"}.get(section, section)
    if words[0] in (lead, section):
        words = words[1:]
    ends = None
    if len(words) >= 2 and (words[-2], words[-1]) in PAIRS:
        ends = PAIRS[(words[-2], words[-1])]
        words = words[:-2]
    text = "-".join(words)
    for old, new in sorted(WORDS.items(), key=lambda kv: -len(kv[0])):
        text = re.sub(rf"(^|-){re.escape(old)}(?=-|$|\d)", lambda m: m.group(1) + new, text)
    text = re.sub(r"(\D)(\d)", r"\1 \2", text.replace("-", " ")).strip()
    label = f"{SECTION_WORD[section]} {text}".strip() if text else SECTION_WORD[section]
    if ends and ends != ("less", "more"):
        # Two sliders of one thing differ by their direction ("position").
        label += f", {ends[0]} or {ends[1]}"
    if ends:
        thing = label.split(",")[0]
        about = f"The {thing[0].lower() + thing[1:]}, from {ends[0]} to {ends[1]}."
    else:
        label = f"{text.capitalize()} {SECTION_WORD[section].lower()}"
        about = f"How far the {SECTION_WORD[section].lower()} goes toward {text}."
    return label, about


def left_weight(x):
    """How much of a both-sided unit belongs to the left side (+X) at `x`."""
    t = np.clip((x + SPLIT) / (2 * SPLIT), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def expressions(read, rest):
    """{control: every body corner's move} for the controls in `CONTROLS`.
    `read(folder, name)` returns a target's moves; `rest` is the body."""
    units = {}

    def unit(name):
        if name not in units:
            units[name] = sum(read(f"expression/units/{race}", name) for race in RACES) / len(RACES)
        return units[name]

    left = left_weight(rest[:, 0])[:, None]
    out = {}
    for control, terms in CONTROLS.items():
        total = np.zeros_like(rest)
        for name, weight, side in terms:
            moves = unit(name) * weight
            total += moves * (left if side == "L" else (1 - left) if side == "R" else 1.0)
        out[control] = total
    return out


def identity(root, read):
    """(parameters, {shape: moves}) from MakeHuman's slider list. A
    parameter here is {id, label, about, group, mirror?, low?, high?}, with
    `low` and `high` the shapes its two ends drive."""
    with open(os.path.join(root, "target.json"), encoding="utf-8") as f:
        listed = json.load(f)
    parameters, shapes = [], {}

    def shape(section, target):
        if not target or not os.path.exists(os.path.join(root, section, target + ".target.gz")):
            return None
        name = shape_name(target)
        if name not in shapes:
            shapes[name] = read(section, target)
        return name

    for section, group, _ in SECTIONS:
        for category in listed.get(section, {}).get("categories", []):
            # A shape of its own (an oval head, a triangular chin) has one
            # target and no opposite.
            name = category["name"]
            opposites = category.get("opposites") or {"positive-unsided": category["targets"][0]}
            label, about = label_of(section, name)
            stem = re.sub(r"[^a-z0-9]+", "_", f"{group} {name.split('-', 1)[-1]}".lower()).strip("_")
            stem = stem.removesuffix("_decr_incr")
            sides = [("", "unsided")] if not category["has_left_and_right"] else [("left", "left"), ("right", "right")]
            for suffix, key in sides:
                low = shape(section, opposites.get(f"negative-{key}"))
                high = shape(section, opposites.get(f"positive-{key}"))
                if not (low or high):
                    continue
                entry = {"id": f"{stem}_{suffix}" if suffix else stem, "group": group,
                         "label": f"{label}, {suffix}" if suffix else label,
                         "about": about if not suffix else about[:-1] + f", on the {suffix} side."}
                if low:
                    entry["low"] = low
                if high:
                    entry["high"] = high
                if suffix:
                    entry["mirror"] = f"{stem}_{'right' if suffix == 'left' else 'left'}"
                parameters.append(entry)
    # Two sliders can't share an ID: number a repeat.
    seen = {}
    for p in parameters:
        seen[p["id"]] = seen.get(p["id"], 0) + 1
    assert all(count == 1 for count in seen.values()), [k for k, c in seen.items() if c > 1]
    return parameters, shapes
