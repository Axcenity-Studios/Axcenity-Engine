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
driving its two targets as shapes. Each has a name written by hand
(`NAMES`: label, what its two ends are, the level that shows it); a slider
MakeHuman adds later falls back to a label made from its target's name
(`label_of`).

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


Q, D, A = "quick", "detailed", "advanced"
# The identity sliders' names, by hand: slider (its ID without a side) ->
# (label, what its low end is, what its high end is, level), in the order
# each page shows them. A paired slider's label is written without its
# side ("eye size": "Left eye size"), and with it is at most 22 characters,
# which is what Character Studio's panel shows without cutting it short
# (`shipped_library.rs` checks). What each one moves was measured on
# MakeHuman's targets on 2026-11-24 (where on the head, which way), since
# their names alone mislead: `eye-push1` is the outer corner, `nose-width1`
# the bridge between the eyes, `chin-bones` the corners of the jaw,
# `head-triangular` widest at the jaw and `head-invertedtriangular` at the
# temples, `eyebrows-angle-up` lowers the brows' outer ends.
NAMES = {
    # Face shape
    "face_shape_scale_horiz": ("Head width", "narrow", "wide", Q),
    "face_shape_scale_vert": ("Head height", "short", "tall", Q),
    "face_shape_scale_depth": ("Head depth", "shallow", "deep", D),
    "face_shape_fat": ("Face fat", "gaunt", "full", Q),
    "face_shape_age": ("Face age", "younger", "older", D),
    "face_shape_oval": ("Oval face", None, "an oval face, wide at the cheeks", D),
    "face_shape_round": ("Round face", None, "a round face", D),
    "face_shape_square": ("Square face", None, "a square face with a wide jaw", D),
    "face_shape_rectangular": ("Long, square face", None, "a long face with a square jaw", D),
    "face_shape_invertedtriangular": ("Heart-shaped face", None, "a face wide at the temples and narrow at the jaw", D),
    "face_shape_triangular": ("Pear-shaped face", None, "a face widest at the jaw", D),
    "face_shape_diamond": ("Diamond face", None, "a face narrow at the temples and the jaw", D),
    "face_shape_angle_in_out": ("Lower face slope", "receding", "projecting", D),
    "face_shape_back_scale_depth": ("Back of the head", "flat", "deep", D),
    "face_shape_trans_backward_forward": ("Head set, depth", "back", "forward", D),
    "face_shape_trans_down_up": ("Head set, height", "lower", "higher", A),
    "face_shape_trans_in_out": ("Head set, sideways", "toward its right", "toward its left", A),
    # Forehead
    "forehead_scale_vert": ("Forehead height", "low", "high", D),
    "forehead_trans_backward_forward": ("Forehead slope", "sloped back", "full", D),
    "forehead_temple": ("Temple width", "narrow", "wide", D),
    "forehead_nubian": ("Crown", "flat", "high and long", A),
    # Brows
    "brows_trans_down_up": ("Brow height", "low", "high", Q),
    "brows_angle_down_up": ("Brow slant", "outer ends raised", "outer ends lowered", D),
    "brows_trans_backward_forward": ("Brow ridge", "flat", "prominent", D),
    # Eyes
    "eyes_scale": ("eye size", "small", "large", Q),
    "eyes_trans_in_out": ("eye spacing", "nearer the nose", "farther from it", Q),
    "eyes_trans_down_up": ("eye height", "lower", "higher", D),
    "eyes_height2": ("eye opening", "narrow", "wide", D),
    "eyes_corner1_down_up": ("outer eye corner", "lower", "higher", D),
    "eyes_corner2_down_up": ("inner eye corner", "lower", "higher", D),
    "eyes_eyefold_down_up": ("upper lid crease", "lower", "higher", D),
    "eyes_eyefold_concave_convex": ("lid fullness", "hollow", "full", D),
    "eyes_epicanthus_in_out": ("inner eye fold", "folded in", "open", D),
    "eyes_bag": ("under-eye bag", "none", "heavy", D),
    "eyes_height1": ("inner lid gap", "narrow", "wide", A),
    "eyes_height3": ("outer lid gap", "narrow", "wide", A),
    "eyes_push1_in_out": ("outer eye reach", "drawn in", "drawn out", A),
    "eyes_push2_in_out": ("inner eye reach", "drawn in", "drawn out", A),
    "eyes_eyefold_angle_down_up": ("lid crease slant", "down", "up", A),
    "eyes_bag_height": ("eye bag height", "short", "tall", A),
    "eyes_bag_in_out": ("eye bag reach", "in", "out", A),
    # Nose
    "nose_scale_horiz": ("Nose width", "narrow", "wide", Q),
    "nose_scale_vert": ("Nose length", "short", "long", Q),
    "nose_scale_depth": ("Nose projection", "flat", "prominent", Q),
    "nose_point_down_up": ("Nose tip tilt", "turned down", "turned up", D),
    "nose_volume": ("Nose tip size", "small", "bulbous", D),
    "nose_point_width": ("Nose tip width", "narrow", "wide", D),
    "nose_hump": ("Bridge hump", "scooped", "humped", D),
    "nose_curve_concave_convex": ("Bridge curve", "concave", "convex", D),
    "nose_greek": ("Nose root", "deep between the eyes", "straight from the forehead", D),
    "nose_width3": ("Nose width, lower part", "narrow", "wide", D),
    "nose_nostrils_width": ("Nostril width", "narrow", "wide", D),
    "nose_flaring": ("Nostril flare", "tight", "flared", D),
    "nose_base_down_up": ("Nose base height", "lower", "higher", D),
    "nose_trans_down_up": ("Nose set, height", "lower", "higher", D),
    "nose_width1": ("Bridge width, upper", "narrow", "wide", A),
    "nose_width2": ("Bridge width, middle", "narrow", "wide", A),
    "nose_nostrils_angle_down_up": ("Nostril angle", "down", "up", A),
    "nose_septumangle": ("Septum angle", "level", "raised", A),
    "nose_compression_compress_uncompress": ("Bridge length", "compressed", "stretched", A),
    "nose_trans_backward_forward": ("Nose set, depth", "back", "forward", A),
    "nose_trans_in_out": ("Nose set, sideways", "toward its right", "toward its left", A),
    # Cheeks
    "cheeks_bones": ("cheekbone width", "narrow", "wide", Q),
    "cheeks_volume": ("cheek fullness", "hollow", "full", Q),
    "cheeks_trans_down_up": ("cheekbone height", "lower", "higher", D),
    "cheeks_inner": ("inner cheek", "flat", "full", A),
    # Lips
    "lips_scale_horiz": ("Mouth width", "narrow", "wide", Q),
    "lips_upperlip_volume": ("Upper lip fullness", "thin", "full", Q),
    "lips_lowerlip_volume": ("Lower lip fullness", "thin", "full", Q),
    "lips_angles_down_up": ("Mouth corners", "turned down", "turned up", Q),
    "lips_scale_vert": ("Mouth height", "thin", "tall", D),
    "lips_upperlip_height": ("Upper lip height", "short", "tall", D),
    "lips_lowerlip_height": ("Lower lip height", "short", "tall", D),
    "lips_upperlip_width": ("Upper lip width", "narrow", "wide", D),
    "lips_lowerlip_width": ("Lower lip width", "narrow", "wide", D),
    "lips_cupidsbow": ("Cupid's bow", "flat", "pronounced", D),
    "lips_philtrum_volume": ("Philtrum", "flat", "full", D),
    "lips_laugh_lines_in_out": ("Laugh lines", "creased", "smooth", D),
    "lips_dimples_in_out": ("Dimples", "dimpled", "smooth", D),
    "lips_trans_down_up": ("Mouth set, height", "lower", "higher", D),
    "lips_trans_backward_forward": ("Mouth set, depth", "back", "forward", D),
    "lips_scale_depth": ("Mouth depth", "shallow", "deep", A),
    "lips_cupidsbow_width": ("Cupid's bow width", "narrow", "wide", A),
    "lips_upperlip_middle_down_up": ("Upper lip, middle", "lower", "higher", A),
    "lips_lowerlip_middle_down_up": ("Lower lip, middle", "lower", "higher", A),
    "lips_upperlip_ext_down_up": ("Upper lip, outer parts", "lower", "higher", A),
    "lips_lowerlip_ext_down_up": ("Lower lip, outer parts", "lower", "higher", A),
    "lips_trans_in_out": ("Mouth set, sideways", "toward its right", "toward its left", A),
    # Chin and jaw
    "chin_height": ("Chin height", "short", "long", Q),
    "chin_width": ("Chin width", "narrow", "wide", Q),
    "chin_prominent": ("Chin projection", "receding", "prominent", Q),
    "chin_bones": ("Jaw corners", "soft", "strong", Q),
    "chin_prognathism": ("Lower jaw", "set back", "set forward", D),
    "chin_jaw_drop": ("Under the chin", "tight", "dropped", D),
    "chin_cleft": ("Chin cleft", "none", "deep", D),
    "chin_triangle": ("Pointed chin", None, "a pointed chin", D),
    # Ears
    "ears_scale": ("ear size", "small", "large", Q),
    "ears_wing": ("ear protrusion", "flat to the head", "standing out", Q),
    "ears_scale_vert": ("ear height", "short", "tall", D),
    "ears_scale_depth": ("ear width", "narrow", "wide", D),
    "ears_lobe": ("earlobe", "small", "long", D),
    "ears_rot_backward_forward": ("ear tilt", "back", "forward", D),
    "ears_trans_down_up": ("ear set, height", "lower", "higher", D),
    "ears_flap": ("ear rim", "flat", "turned out", A),
    "ears_shape_square_round": ("ear roundness", "square", "round", A),
    "ears_shape_pointed_triangle": ("ear tip shape", "pointed", "triangular", A),
    "ears_trans_backward_forward": ("ear set, depth", "back", "forward", A),
    # Neck
    "neck_neck_circ": ("Neck girth", "thin", "thick", Q),
    "neck_neck_height": ("Neck length", "short", "long", Q),
    "neck_scale_horiz": ("Neck width", "narrow", "wide", D),
    "neck_scale_depth": ("Neck depth", "shallow", "deep", D),
    "neck_back_scale_depth": ("Nape", "flat", "full", D),
    "neck_double": ("Double chin", "none", "full", D),
    "neck_trans_backward_forward": ("Neck set, depth", "back", "forward", D),
    "neck_scale_vert": ("Neck stretch", "compressed", "stretched", A),
    "neck_trans_down_up": ("Neck set, height", "lower", "higher", A),
    "neck_trans_in_out": ("Neck set, sideways", "toward its right", "toward its left", A),
}


def named(stem, side):
    """(label, about, level) of the slider `stem` from `NAMES`, on `side`
    ("" or "left" or "right"); None when it isn't there."""
    if stem not in NAMES:
        return None
    label, low, high, level = NAMES[stem]
    if side:
        label = f"{side.capitalize()} {label}"
    else:
        label = label[0].upper() + label[1:]
    if low is None:
        about = f"How far the face goes toward {high}."
    else:
        about = f"{label}: from {low} to {high}."
    return label, about, level


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
                         "about": about if not suffix else about[:-1] + f", on the {suffix} side.",
                         "level": "detailed", "stem": stem}
                # A name written by hand, where there is one (`NAMES`).
                by_hand = named(stem, suffix)
                if by_hand:
                    entry["label"], entry["about"], entry["level"] = by_hand
                if low:
                    entry["low"] = low
                if high:
                    entry["high"] = high
                if suffix:
                    entry["mirror"] = f"{stem}_{'right' if suffix == 'left' else 'left'}"
                parameters.append(entry)
    # Each page in `NAMES`'s order, the unnamed after them as listed.
    order = {stem: k for k, stem in enumerate(NAMES)}
    groups = [group for _, group, _ in SECTIONS]
    parameters.sort(key=lambda p: (groups.index(p["group"]), order.get(p["stem"], len(order))))
    seen = {}
    for p in parameters:
        seen[p["id"]] = seen.get(p["id"], 0) + 1
    assert all(count == 1 for count in seen.values()), [k for k, c in seen.items() if c > 1]
    return parameters, shapes


# What a person can point at on the face (the manifest's `features`): the
# page's group ID -> (label, a sentence, the bone it rides, a pair?).
FEATURES = {
    "face_shape": ("Face shape", "The head's size and outline.", "Head", False),
    "forehead": ("Forehead", "The forehead's height and slope, the temples.", "Head", False),
    "brows": ("Brows", "How high the brows sit and how they slant.", "Head", False),
    "eyes": ("Eyes", "Each eye's size, place, corners and lids.", "Head", True),
    "nose": ("Nose", "The nose's size, bridge, tip and nostrils.", "Head", False),
    "cheeks": ("Cheeks", "The cheekbones and the soft cheeks under them.", "Head", True),
    "lips": ("Lips", "The mouth's width, the lips' fullness and shape.", "Head", False),
    "chin": ("Chin and jaw", "The chin's size and the jaw's corners.", "Head", False),
    "ears": ("Ears", "Each ear's size, shape and how far it stands out.", "Head", True),
    "neck": ("Neck", "The neck's length and girth.", "Neck", False),
}
# Handles on the face (the manifest's `handles`): (ID, feature, label, the
# shape whose most-moved corner is the handle's place, where to look for
# it (0 the middle, 1 the left half, None anywhere, 2 the outline: of the
# corners the shape moves at least `OUTLINE` of its most, the one farthest
# out to its side, so an ear's or the neck's handle is seen from the front
# and not behind the ear's rim or under the jaw), the sliders it drives).
# `{side}` and `{Side}` make a left and a right one.
HANDLES = (
    ("head_width", "face_shape", "Head width", "Face_Head_Scale_Horiz_Incr", 1, ("face_shape_scale_horiz",)),
    ("head_height", "face_shape", "Head height", "Face_Head_Scale_Vert_Incr", 0, ("face_shape_scale_vert",)),
    ("forehead", "forehead", "Forehead", "Face_Forehead_Trans_Forward", 0,
     ("forehead_trans_backward_forward", "forehead_scale_vert")),
    ("brow", "brows", "Brows", "Face_Eyebrows_Trans_Up", 1, ("brows_trans_down_up", "brows_trans_backward_forward")),
    ("eye_{side}", "eyes", "{Side} eye", "Face_Eye_Trans_Up_{Side}", None,
     ("eyes_trans_down_up_{side}", "eyes_trans_in_out_{side}")),
    ("eye_corner_{side}", "eyes", "{Side} outer corner", "Face_Eye_Corner1_Up_{Side}", None,
     ("eyes_corner1_down_up_{side}",)),
    ("nose_tip", "nose", "Nose tip", "Face_Nose_Point_Up", 0, ("nose_scale_depth", "nose_point_down_up")),
    ("nose_bridge", "nose", "Bridge", "Face_Nose_Hump_Incr", 0, ("nose_hump",)),
    ("nose_wing", "nose", "Nostril", "Face_Nose_Scale_Horiz_Incr", 1, ("nose_scale_horiz",)),
    ("cheekbone_{side}", "cheeks", "{Side} cheekbone", "Face_Cheek_Bones_Incr_{Side}", None,
     ("cheeks_bones_{side}", "cheeks_trans_down_up_{side}")),
    ("cheek_{side}", "cheeks", "{Side} cheek", "Face_Cheek_Volume_Incr_{Side}", None, ("cheeks_volume_{side}",)),
    ("mouth_corner", "lips", "Mouth corner", "Face_Mouth_Scale_Horiz_Incr", 1,
     ("lips_scale_horiz", "lips_angles_down_up")),
    ("upper_lip", "lips", "Upper lip", "Face_Mouth_Upperlip_Volume_Incr", 0,
     ("lips_upperlip_volume", "lips_upperlip_height")),
    ("lower_lip", "lips", "Lower lip", "Face_Mouth_Lowerlip_Volume_Incr", 0,
     ("lips_lowerlip_volume", "lips_lowerlip_height")),
    ("chin_tip", "chin", "Chin", "Face_Chin_Prominent_Incr", 0, ("chin_prominent", "chin_height")),
    ("jaw_corner", "chin", "Jaw corner", "Face_Chin_Bones_Incr", 1, ("chin_bones",)),
    ("chin_side", "chin", "Chin width", "Face_Chin_Width_Incr", 1, ("chin_width",)),
    ("ear_{side}", "ears", "{Side} ear", "Face_Ear_Wing_Incr_{Side}", 2, ("ears_wing_{side}", "ears_scale_{side}")),
    ("neck_side", "neck", "Neck", "Face_Measure_Neck_Circ_Incr", 2, ("neck_neck_circ",)),
)
#: The share of a shape's largest move a corner must make to be on its
#: outline (a handle's place of kind 2).
OUTLINE = 0.6
#: The least a handle's corner must move for a slider to be driven by it
#: (metres), and how alike two drives' directions may be (their cosine).
LEAST_TRAVEL = 0.0008
ALIKE = 0.8
#: Character Studio's framings (`stage.rs`): the camera's turn round the
#: character from its front, in degrees (a side view is its right
#: profile). A handle's drive works in a framing when at least `LIVE` of
#: its travel shows across the screen there, and two drives of one handle
#: whose lines on the screen are closer than `APART` degrees can't both
#: follow one drag: the one that shows less gives way. The same rules as
#: the realistic base's handles (`realistic/handles.py`).
VIEWS = {"front": 0.0, "three_quarter": 45.0, "side": 90.0, "back": 180.0}
LIVE = 0.35
APART = 25.0


def _gltf(d):
    """A Blender-space vector (Z up, front -Y) in glTF space (Y up, front +Z)."""
    return [round(float(d[0]), 5), round(float(d[2]), 5), round(float(-d[1]), 5)]


def _screen(axis, view):
    """Where `axis` (glTF space) goes on the screen (x right, y up) in
    framing `view`: in front the character's left (+X) is to the right,
    from its right side its front (+Z) is."""
    turn = np.radians(VIEWS[view])
    return np.array([axis[0] * np.cos(turn) + axis[2] * np.sin(turn), axis[1]])


def _framings(drives):
    """Gives each of a handle's `drives` its `screen` direction in every
    framing it works in, and returns the framings the handle shows in."""
    views = []
    for view in VIEWS:
        shown = []
        for drive in drives:
            direction = _screen(np.array(drive["axis"]), view)
            length = float(np.linalg.norm(direction))
            if length >= LIVE:
                shown.append((length, direction / length, drive))
        kept = []
        for _, direction, drive in sorted(shown, key=lambda s: -s[0]):
            if all(abs(float(direction @ other)) < np.cos(np.radians(APART)) for other in kept):
                kept.append(direction)
                drive["screen"][view] = [round(float(v), 3) for v in direction]
                if view not in views:
                    views.append(view)
    return views


def features_and_handles(parameters, moves, points, heads):
    """(features, handles) for the manifest. `parameters` are `identity`'s
    (those the body has shapes for), `moves` {shape: every body corner's
    move} and `points` the body's corners, both in the engine's pose and
    Blender's axes, `heads` {bone: its joint}. A feature's anchor is the
    middle of what its page's sliders move; a handle sits on the corner
    its own shape moves most, and each slider it drives moves that corner
    along `axis` by `travel` at the top of its range."""
    by_id = {p["id"]: p for p in parameters}

    def reach(p):
        """A slider's move of every corner for a rising value."""
        low, high = p.get("low"), p.get("high")
        if low in moves and high in moves:
            return (moves[high] - moves[low]) / 2
        if high in moves:
            return moves[high]
        return -moves[low] if low in moves else None

    features = []
    for group, (label, about, bone, pair) in FEATURES.items():
        own = [p for p in parameters if p["group"] == group]
        if not own:
            continue
        anchors, spread = [], 0.0
        for side in (("_left", "_right") if pair else ("",)):
            weight = np.zeros(len(points))
            for p in own:
                if p["id"].endswith(side):
                    for end in ("low", "high"):
                        if p.get(end) in moves:
                            weight += np.linalg.norm(moves[p[end]], axis=1)
            keep = weight > weight.max() * 0.5
            w = weight[keep][:, None]
            middle = (points[keep] * w).sum(axis=0) / w.sum()
            spread = max(spread, float(np.sqrt((np.linalg.norm(points[keep] - middle, axis=1) ** 2 * w[:, 0]).sum()
                                               / w.sum())))
            anchors.append({"bone": bone, "offset": _gltf(middle - heads[bone])})
        features.append({"id": group, "label": label, "category": "face", "about": about, "anchors": anchors,
                         "radius": round(max(0.02, 1.2 * spread), 4)})

    handles = []
    for hid, feature, label, pick, where, drives in HANDLES:
        sides = (("left", "Left"), ("right", "Right")) if "{side}" in hid else (("", ""),)
        made = []
        for side, cap in sides:
            shape = pick.format(side=side, Side=cap)
            if shape not in moves:
                continue
            size = np.linalg.norm(moves[shape], axis=1)
            if where == 0:
                size = np.where(np.abs(points[:, 0]) < 0.004, size, 0.0)
            elif where == 1:
                size = np.where(points[:, 0] > 0.004, size, 0.0)
            if size.max() <= 0:
                continue
            if where == 2:
                # The left one unless the shape is the right side's.
                out_there = -points[:, 0] if side == "right" else points[:, 0]
                corner = int(np.where(size >= OUTLINE * size.max(), out_there, -np.inf).argmax())
            else:
                corner = int(size.argmax())
            out = []
            for stem in drives:
                p = by_id.get(stem.format(side=side, Side=cap))
                move = reach(p)[corner] if p else None
                if move is None or np.linalg.norm(move) < LEAST_TRAVEL:
                    continue
                axis = move / np.linalg.norm(move)
                if any(abs(float(axis @ other)) > ALIKE for _, other, _ in out):
                    continue
                out.append((p["id"], axis, float(np.linalg.norm(move))))
            if not out:
                continue
            bone = FEATURES[feature][2]
            driven = [{"parameter": pid, "axis": _gltf(axis), "travel": round(travel, 5), "screen": {}}
                      for pid, axis, travel in out]
            views = _framings(driven)
            # A drive no framing shows can't be dragged: its slider stays.
            driven = [drive for drive in driven if drive["screen"]]
            if not driven:
                continue
            made.append({"id": hid.format(side=side), "feature": feature, "label": label.format(Side=cap),
                         "anchor": {"bone": bone, "offset": _gltf(points[corner] - heads[bone])},
                         "drives": driven, "views": views})
        if len(made) == 2:
            made[0]["mirror"], made[1]["mirror"] = made[1]["id"], made[0]["id"]
        handles += made
    return features, handles
