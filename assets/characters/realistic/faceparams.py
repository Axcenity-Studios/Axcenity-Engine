"""The realistic head's face parameters for the base model manifest
(Character Studio overhaul CO6). Plain Python: `manifests.py` reads it.

Every face identity shape (`face.Face.identity`) is one parameter from -1
to 1, 0 being the face as built. A paired feature has a parameter a side
(`eye_height_left`, `eye_height_right`), each naming the other as its
`mirror`, so the studio can edit both together or one alone (asymmetry).
The middle of the face has single parameters, and three that push it off
centre (`nose_tilt`, `jaw_shift`, in the advanced view).

`pupil_size` drives `Pupil_Size`, a shape of the `Eyes` part (the body
carries an empty shape of that name).
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
    ("Face_Brow_Height", "brow_height", "brow height", "brows", False),
    ("Face_Brow_Arch", "brow_arch", "brow arch", "brows", False),
    ("Face_Nose_Width", "nostril_width", "nostril width", "nose", False),
    ("Face_Cheekbone", "cheekbone", "cheekbone", "cheeks", False),
    ("Face_Cheek_Fullness", "cheek_fullness", "cheek fullness", "cheeks", False),
    ("Face_Jaw_Width", "jaw_width", "jaw width", "jaw", False),
    ("Face_Mouth_Width", "mouth_width", "mouth width", "lips", False),
    ("Face_Ear_Size", "ear_size", "ear size", "ears", False),
    ("Face_Ear_Height", "ear_height", "ear height", "ears", True),
    ("Face_Ear_Out", "ear_out", "ear protrusion", "ears", False),
]

# Single: (shape, id, label, group, advanced).
SINGLE = [
    ("Pupil_Size", "pupil_size", "Pupil size", "eyes", False),
    ("Face_Brow_Ridge", "brow_ridge", "Brow ridge", "brows", False),
    ("Face_Nose_Length", "nose_length", "Nose length", "nose", False),
    ("Face_Nose_Bridge", "nose_bridge", "Nose bridge", "nose", False),
    ("Face_Nose_Tip", "nose_tip", "Nose tip", "nose", False),
    ("Face_Nose_Tilt", "nose_tilt", "Nose to one side", "nose", True),
    ("Face_Jaw_Shift", "jaw_shift", "Jaw to one side", "jaw", True),
    ("Face_Chin_Length", "chin_length", "Chin length", "chin", False),
    ("Face_Chin_Forward", "chin_forward", "Chin forward", "chin", False),
    ("Face_Chin_Width", "chin_width", "Chin width", "chin", False),
    ("Face_Lip_Fullness_Upper", "upper_lip_fullness", "Upper lip fullness", "lips", False),
    ("Face_Lip_Fullness_Lower", "lower_lip_fullness", "Lower lip fullness", "lips", False),
    ("Face_Mouth_Height", "mouth_height", "Mouth height", "lips", False),
    ("Face_Forehead_Height", "forehead_height", "Forehead height", "forehead", False),
    ("Face_Forehead_Slope", "forehead_slope", "Forehead fullness", "forehead", False),
    ("Face_Head_Width", "temple_width", "Temple width", "forehead", False),
]


def shapes():
    """Every identity shape the parameters drive."""
    return [f"{stem}_{side}" for stem, *_ in PAIRED for side in ("Left", "Right")] + [s for s, *_ in SINGLE]


def groups():
    return [{"id": gid, "label": label, "category": "face"} for gid, label in GROUPS]


def parameters(face_identity):
    """The face parameters, in display order. `face_identity` is the
    build data's `face.face_identity`: {shape: {"bones": {bone: change}}}
    (offsets in glTF axes)."""
    out = []

    def parameter(pid, label, group, shape, advanced, mirror=None):
        p = {"id": pid, "label": label, "group": group, "range": [-1, 1]}
        if advanced:
            p["advanced"] = True
        if mirror:
            p["mirror"] = mirror
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
            rows.append((order[group], parameter(f"{pid}_{side}", f"{side.capitalize()} {label}", group,
                                                 f"{stem}_{side.capitalize()}", advanced, f"{pid}_{other}")))
    for shape, pid, label, group, advanced in SINGLE:
        rows.append((order[group], parameter(pid, label, group, shape, advanced)))
    rows.sort(key=lambda row: row[0])
    out = [p for _, p in rows]
    return out
