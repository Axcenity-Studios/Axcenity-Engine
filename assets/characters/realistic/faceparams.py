"""The realistic head's face parameters for the base model manifest
(Character Studio overhaul CO6). Plain Python: `manifests.py` reads it.

Every face identity shape (`face.Face.identity`) is one parameter from -1
to 1, 0 being the face as built, but for the brows' height, arch and
length, which have a shape for each end (`SPLIT`): a shape moves its
vertices along one straight line, and a lowered brow's fold has to come
forward over the lid where a raised one lies back on the brow bone (and a
brow's tail follows the brow's curve, `lashes.brow_lengths`). The length
shapes move the `Brows` card only; the body carries them empty. A paired
feature has a parameter a side
(`eye_height_left`, `eye_height_right`), each naming the other as its
`mirror`, so the studio can edit both together or one alone (asymmetry).
The middle of the face has single parameters, and three that push it off
centre (`nose_tilt`, `jaw_shift`, in the advanced view).

Each parameter has an `about` sentence (`ABOUT`), its group as its
`feature` and a `level` (R1: `quick` for the main sizes, `QUICK`;
`advanced` where it is flagged so; else `detailed`). `features()` names
the shapes that find each face feature for `anchors.py`.

The pupil has no parameter: `Pupil_Size` is a shape of the `Eyes` part
that moves 1.3 mm of the eye, which is not a slider's worth (a parameter
has to be seen on the body); the eye shading's `pupil_size` (CO7) or a
script setting the part's shape is what should drive it.
"""

# (group id, label): all in the studio's `face` category.
GROUPS = [
    ("eyes", "Eyes"),
    ("brows", "Brows"),
    ("nose", "Nose"),
    ("cheeks", "Cheeks"),
    ("jaw", "Jaw"),
    ("chin", "Chin"),
    ("lips", "Lips"),
    ("forehead", "Forehead"),
    ("ears", "Ears"),
]

# Paired: (shape stem, id stem, label, group, advanced).
PAIRED = [
    ("Face_Eye_Height", "eye_height", "eye height", "eyes", False),
    ("Face_Eye_Spacing", "eye_spacing", "eye spacing", "eyes", False),
    ("Face_Eye_Depth", "eye_depth", "eye depth", "eyes", True),
    ("Face_Eye_Size", "eye_size", "eye size", "eyes", False),
    ("Face_Eye_Tilt", "eye_tilt", "eye tilt", "eyes", False),
    ("Face_Eye_Corner", "eye_corner", "eye corner", "eyes", False),
    ("Face_Lid_Fold", "upper_lid_fold", "upper lid fold", "eyes", False),
    ("Face_Lower_Lid", "lower_lid", "lower lid", "eyes", False),
    ("Face_Brow_Height", "brow_height", "brow height", "brows", False),
    ("Face_Brow_Arch", "brow_arch", "brow arch", "brows", False),
    ("Face_Brow_Length", "brow_length", "brow length", "brows", False),
    ("Face_Nose_Width", "nostril_width", "nostril width", "nose", False),
    ("Face_Cheekbone", "cheekbone", "cheekbone", "cheeks", False),
    ("Face_Cheek_Fullness", "cheek_fullness", "cheek fullness", "cheeks", False),
    ("Face_Jaw_Width", "jaw_width", "jaw width", "jaw", False),
    ("Face_Jaw_Angle", "jaw_angle", "jaw angle", "jaw", False),
    ("Face_Mouth_Width", "mouth_width", "mouth width", "lips", False),
    ("Face_Ear_Size", "ear_size", "ear size", "ears", False),
    ("Face_Ear_Height", "ear_height", "ear height", "ears", True),
    ("Face_Ear_Out", "ear_out", "ear protrusion", "ears", False),
    ("Face_Earlobe", "earlobe", "earlobe", "ears", False),
]

# Single: (shape, id, label, group, advanced).
SINGLE = [
    ("Face_Brow_Ridge", "brow_ridge", "Brow ridge", "brows", False),
    ("Face_Nose_Length", "nose_length", "Nose length", "nose", False),
    ("Face_Nose_Bridge", "nose_bridge", "Nose bridge", "nose", False),
    ("Face_Nose_Tip", "nose_tip", "Nose tip", "nose", False),
    ("Face_Nose_Tip_Projection", "nose_tip_projection", "Nose tip projection", "nose", False),
    ("Face_Nose_Tip_Rotation", "nose_tip_rotation", "Nose tip rotation", "nose", False),
    ("Face_Nose_Width", "nose_width", "Nose width", "nose", False),
    ("Face_Nose_Tilt", "nose_tilt", "Nose to one side", "nose", True),
    ("Face_Jaw_Shift", "jaw_shift", "Jaw to one side", "jaw", True),
    ("Face_Chin_Length", "chin_length", "Chin height", "chin", False),
    ("Face_Chin_Forward", "chin_forward", "Chin forward", "chin", False),
    ("Face_Chin_Width", "chin_width", "Chin width", "chin", False),
    ("Face_Lip_Fullness_Upper", "upper_lip_fullness", "Upper lip fullness", "lips", False),
    ("Face_Lip_Fullness_Lower", "lower_lip_fullness", "Lower lip fullness", "lips", False),
    ("Face_Mouth_Height", "mouth_height", "Mouth height", "lips", False),
    ("Face_Philtrum_Length", "philtrum_length", "Philtrum length", "lips", False),
    ("Face_Forehead_Height", "forehead_height", "Forehead height", "forehead", False),
    ("Face_Forehead_Slope", "forehead_slope", "Forehead fullness", "forehead", False),
    ("Face_Head_Width", "temple_width", "Temple width", "forehead", False),
]


# Paired features with a shape for each end: {shape stem: the raising
# shape's stem}. The stem's own shape is the lowering one, driven from 0
# to -1 (weights 0 to -1, as one shape for both ends would be); the
# raising one is driven from 0 to 1.
SPLIT = {"Face_Brow_Height": "Face_Brow_Height_Up", "Face_Brow_Arch": "Face_Brow_Arch_Up",
         "Face_Brow_Length": "Face_Brow_Length_Long"}


# What each parameter does, in a sentence (its tooltip, R1). A pair's
# sentence has `{side}` where the side goes ("the left eye").
ABOUT = {
    "eye_height": "How high the {side} eye sits on the face.",
    "eye_spacing": "How far the {side} eye sits from the middle of the face.",
    "eye_depth": "How deep the {side} eye sits in its socket.",
    "eye_size": "How large the {side} eye opening is.",
    "eye_tilt": "How far the {side} eye turns about its middle: the outer corner up and the inner corner down.",
    "eye_corner": "How far the outer corner of the {side} eye is raised or lowered, with the inner corner held.",
    "upper_lid_fold": "How far the fold over the {side} upper lid hangs down over it (hooded) or sits up and back.",
    "lower_lid": "How full and high the {side} lower lid sits, from a flat, long lid to a full, short one.",
    "brow_height": "How high the {side} brow sits above the eye.",
    "brow_arch": "How high the {side} brow arches over its ends.",
    "brow_length": "How far the tail of the {side} brow reaches.",
    "nostril_width": "How far the {side} nostril flares.",
    "cheekbone": "How prominent the {side} cheekbone is.",
    "cheek_fullness": "How full the soft {side} cheek under the cheekbone is.",
    "jaw_width": "How wide the jaw is on the {side} side.",
    "jaw_angle": "How square the corner of the jaw is on the {side} side, from sloped to square.",
    "mouth_width": "How far the {side} corner of the mouth reaches.",
    "ear_size": "How large the {side} ear is.",
    "ear_height": "How high the {side} ear sits on the head.",
    "ear_out": "How far the {side} ear stands out from the head.",
    "earlobe": "How long the lobe of the {side} ear hangs.",
    "brow_ridge": "How far the bony ridge over the eyes stands forward.",
    "nose_length": "How long the nose is, from the bridge to the tip.",
    "nose_bridge": "How high the bridge of the nose stands.",
    "nose_tip": "How far the tip of the nose projects and turns up.",
    "nose_tip_projection": "How far the tip of the nose stands out from the face.",
    "nose_tip_rotation": "How far the tip of the nose turns up, or droops.",
    "nose_width": "How wide the whole nose is, from the bridge to the nostrils.",
    "nose_tilt": "How far the nose leans to one side.",
    "jaw_shift": "How far the jaw sits to one side.",
    "chin_length": "How tall the chin is below the lower lip.",
    "chin_forward": "How far the chin projects forward.",
    "chin_width": "How wide the chin is.",
    "upper_lip_fullness": "How full the upper lip is.",
    "lower_lip_fullness": "How full the lower lip is.",
    "mouth_height": "How high the mouth sits between the nose and the chin.",
    "philtrum_length": "How long the upper lip is from the nose down to the lip's edge.",
    "forehead_height": "How tall the forehead is, from the brows to the hairline.",
    "forehead_slope": "How full and upright the forehead is.",
    "temple_width": "How wide the head is at the temples.",
}

# The quick level (brief §3.3, broad facial adjustments): the face's main
# sizes. Everything else is detailed, or advanced where it is flagged so.
QUICK = {"eye_size", "nose_length", "mouth_width", "jaw_width"}

# The face's features (R1), one per group, found by the shapes that move
# them (`anchors.py`); `{side}` makes a pair. The anchors sit on the skin.
FEATURES = [
    ("eyes", "Eyes", "Their height, spacing, depth, size, tilt and corners, and the lids' shape.",
     ["Face_Eye_Height_{side}", "Face_Eye_Spacing_{side}", "Face_Eye_Size_{side}", "Face_Eye_Tilt_{side}"]),
    ("brows", "Brows", "Their height, arch and length, and the ridge under them.",
     ["Face_Brow_Height_{side}", "Face_Brow_Arch_{side}"]),
    ("nose", "Nose", "Its length, width, bridge, tip and nostrils.",
     ["Face_Nose_Length", "Face_Nose_Bridge", "Face_Nose_Tip", "Face_Nose_Width_Left", "Face_Nose_Width_Right"]),
    ("cheeks", "Cheeks", "The cheekbones and the soft cheeks under them.",
     ["Face_Cheekbone_{side}", "Face_Cheek_Fullness_{side}"]),
    ("jaw", "Jaw", "How wide and how square the jaw is, and where it sits.", ["Face_Jaw_Width_{side}"]),
    ("chin", "Chin", "Its height, width and projection.",
     ["Face_Chin_Length", "Face_Chin_Forward", "Face_Chin_Width"]),
    ("lips", "Lips", "The mouth's width and height, the upper lip's length and the lips' fullness.",
     ["Face_Lip_Fullness_Upper", "Face_Lip_Fullness_Lower", "Face_Mouth_Height"]),
    ("forehead", "Forehead", "Its height and slope, and the width at the temples.",
     ["Face_Forehead_Height", "Face_Forehead_Slope"]),
    ("ears", "Ears", "Their size, height, lobes and how far they stand out.",
     ["Face_Ear_Size_{side}", "Face_Ear_Height_{side}", "Face_Ear_Out_{side}"]),
]


def features():
    """The face's feature definitions for `anchors.measure`."""
    return [{"id": fid, "label": label, "category": "face", "about": about, "icon": fid,
             "shapes": shapes, "paired": any("{side}" in s for s in shapes), "on_skin": True}
            for fid, label, about, shapes in FEATURES]


def shapes():
    """Every identity shape the parameters drive."""
    out = []
    for stem, *_ in PAIRED:
        for side in ("Left", "Right"):
            out.append(f"{stem}_{side}")
            if stem in SPLIT:
                out.append(f"{SPLIT[stem]}_{side}")
    return out + [s for s, *_ in SINGLE]


def groups():
    return [{"id": gid, "label": label, "category": "face"} for gid, label in GROUPS]


def parameters(face_identity):
    """The face parameters, in display order. `face_identity` is the
    build data's `face.face_identity`: {shape: {"bones": {bone: change}}}
    (offsets in glTF axes)."""
    out = []

    def parameter(pid, label, group, shape, advanced, mirror=None, raising=None, stem=None, side=None):
        stem = stem or pid
        level = "advanced" if advanced else "quick" if stem in QUICK else "detailed"
        p = {"id": pid, "label": label, "group": group, "range": [-1, 1],
             "about": ABOUT[stem].format(side=side), "feature": group, "level": level}
        if advanced:
            p["advanced"] = True
        if mirror:
            p["mirror"] = mirror
        if raising:
            p["shapes"] = [{"shape": shape, "from": [-1, 0], "to": [-1, 0]}, {"shape": raising, "from": [0, 1]}]
        else:
            p["shapes"] = [{"shape": shape, "from": [-1, 1]}]
        bones = []
        for bone, change in sorted(face_identity.get(shape, {}).get("bones", {}).items()):
            bones.append(dict({"bone": bone}, **change, **{"from": [-1, 1]}))
        if bones:
            p["bones"] = bones
        return p

    order = {gid: k for k, (gid, _) in enumerate(GROUPS)}
    rows = []
    for stem, pid, label, group, advanced in PAIRED:
        for side, other in (("left", "right"), ("right", "left")):
            raising = f"{SPLIT[stem]}_{side.capitalize()}" if stem in SPLIT else None
            rows.append((order[group], parameter(f"{pid}_{side}", f"{side.capitalize()} {label}", group,
                                                 f"{stem}_{side.capitalize()}", advanced, f"{pid}_{other}", raising,
                                                 stem=pid, side=side)))
    for shape, pid, label, group, advanced in SINGLE:
        rows.append((order[group], parameter(pid, label, group, shape, advanced)))
    rows.sort(key=lambda row: row[0])
    out = [p for _, p in rows]
    return out
