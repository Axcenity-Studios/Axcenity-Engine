"""The realistic base model's shipped presets (Character Studio refinement
R1; plan section 11.3): each a format 2 character description with a
`preset` block, written to `presets/<name>.preset.json`. Plain Python:

    python -I assets/characters/realistic/presets.py
    python -I assets/characters/catalogue.py

- **Body** (brief §4.1): Slim, Average, Athletic, Muscular, Heavyset, Lean
  and Broad, one file per body type, because the values differ per frame
  (docs/development/character-studio/references/body/README.md, section 6).
  A body preset doesn't name `frame`, so applying one never changes the
  body type; its tags say which body type it is made for (`male`,
  `female`, the frame choices' labels). "Custom" is no file: it is what the
  studio shows when no preset matches.
- **Face** (brief §5.1): the nine shapes of the face references
  (references/face/README.md, section 3), both sides of a pair alike.
- **Character**: the six starting characters Character Studio had in code
  (`crates/editor/src/character_workspace.rs`, `presets()`), as data.
- **Feature** (2026-11-24; architecture.md section 15a): a preset of one
  feature, which sets that feature's parameters and nothing else. Ten nose
  shapes (`NOSES`, feature `nose`: the nose charts' types, each read in
  profile and from the front) and six face shapes (`FACE_SHAPES`, feature
  `face_shape`: the outline alone, so a nose or a pair of eyes is kept).

These are starting values to tune against the references and the
captures, not types of people.
"""

import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
FOLDER = os.path.join(HERE, "presets")
BASE = "axcenity/realistic"
MALE_WEARS = ["axcenity/realistic-boxer-briefs"]
FEMALE_WEARS = ["axcenity/realistic-briefs", "axcenity/realistic-sports-bra"]

# (name, label, about, {frame label: {parameter: value}}): body README §6.
BODY = [
    ("slim", "Slim", "Slight: a small frame with little muscle and little fat.",
     {"male": {"muscle": -0.3, "fat": -0.6, "shoulder_width": 0.97},
      "female": {"muscle": -0.2, "fat": -0.5, "height": 0.95}}),
    ("average", "Average", "An ordinary adult who doesn't train.",
     {"male": {"muscle": -0.5, "fat": 0.5},
      "female": {"muscle": -0.4, "fat": 0.4, "height": 0.95}}),
    ("athletic", "Athletic", "Trained but not bulky.",
     {"male": {"muscle": 0.4, "fat": -0.1},
      "female": {"muscle": 0.6, "fat": -0.1, "height": 0.97}}),
    ("muscular", "Muscular", "Visible mass in the arms, chest and thighs.",
     {"male": {"muscle": 0.9, "fat": 0.5, "muscle_shoulders": 0.3},
      "female": {"muscle": 0.9, "fat": 0.4}}),
    ("heavyset", "Heavyset", "Soft and heavy.",
     {"male": {"muscle": -0.7, "fat": 0.9, "fat_abdomen": 0.4, "waist_width": 1.05},
      "female": {"muscle": -0.6, "fat": 1.0, "fat_hips": 0.3, "hip_width": 1.05, "height": 0.95}}),
    ("lean", "Lean", "Low fat with the muscle tone showing: a runner.",
     {"male": {"muscle": 0.3, "fat": -0.7},
      "female": {"muscle": 0.3, "fat": -0.6, "height": 0.97}}),
    ("broad", "Broad", "A wide, solid frame: wider shoulders, waist and hips.",
     {"male": {"muscle": 0.3, "fat": 0.4, "shoulder_width": 1.10, "waist_width": 1.05, "hip_width": 1.05},
      "female": {"muscle": 0.2, "fat": 0.3, "shoulder_width": 1.08, "hip_width": 1.08, "waist_width": 1.04}}),
]

# Face presets: (name, label, about); face README §3's table, by column.
FACES = [
    ("round", "Round", "Width and length close to equal, full cheeks, a short wide chin, no corners."),
    ("oval", "Oval", "Slightly longer than wide, widest at the cheekbones, tapering gently to the chin."),
    ("square", "Square", "A jaw as wide as the forehead, a wide flat chin and straight sides."),
    ("heart", "Heart-shaped", "A wide forehead and cheekbones, a narrow jaw and a small chin."),
    ("long", "Long", "A tall forehead, a long nose and chin, narrow across."),
    ("broad", "Broad", "Wide across the temples, cheekbones and jaw, with the features set wider."),
    ("narrow", "Narrow", "Narrow across the temples, cheekbones and jaw, with the features set closer."),
    ("soft", "Soft features", "Smooth transitions: a low brow ridge, full cheeks and lips, a rounded chin."),
    ("angular", "Angular features", "Strong planes: high cheekbones over hollow cheeks, a defined ridge and chin."),
]
# Rows: parameter (a pair's stem gets `_left` and `_right`), then one value
# per face above, in that order. Started from the face README's §3 table
# (its strengths a first guess); wave 3 (R4's second pass) strengthened
# them, each toward its sentence and none past 0.7, until every preset
# changes at least 3% of the face's pixels against the default face in
# the studio light's front and side pictures on both frames (`player`'s
# `face_preset_tests`; the lead: square, heart and soft barely differed
# from the default on the masculine frame, and in profile none reached
# 3%). The profile is what the chin, the nose, the brow ridge, the
# forehead's slope and the jaw's angle change. (Review fixes 2: square's
# brow ridge 0.7 on a forehead tipped back by 0.6 drew a hard band across
# the forehead, a shelf with a hollow over it in profile; it keeps a
# moderate ridge, an upright forehead and the chin further forward.
# `avatar`'s `no_face_preset_puts_a_shelf_on_the_forehead` holds it.)
FACE_TABLE = [
    ("forehead_height", [-0.2, 0.6, 0, 0.6, 0.7, -0.6, 0.6, 0, 0]),
    ("forehead_slope", [0.6, 0.2, 0.1, 0.7, -0.4, 0, 0, 0.5, -0.2]),
    ("temple_width", [0.3, -0.3, 0.5, 0.7, -0.3, 0.6, -0.6, 0, 0]),
    ("brow_ridge", [-0.2, 0, 0.35, 0, 0, 0, 0, -0.7, 0.6]),
    ("cheekbone*", [-0.2, 0.7, 0, 0.7, -0.2, 0.6, -0.4, -0.4, 0.7]),
    ("cheek_fullness*", [0.7, -0.3, -0.6, -0.2, -0.3, 0.6, -0.7, 0.7, -0.7]),
    ("jaw_width*", [0.3, -0.6, 0.7, -0.6, -0.3, 0.5, -0.5, -0.5, 0.3]),
    ("chin_width", [0.4, -0.6, 0.7, -0.7, 0, 0.3, -0.3, 0.6, -0.2]),
    ("chin_length", [-0.6, 0.6, -0.5, 0.6, 0.7, -0.7, 0.6, -0.1, 0.1]),
    ("chin_forward", [-0.6, 0.3, 0.7, -0.6, 0.7, 0, 0, -0.4, 0.5]),
    ("nose_length", [-0.1, 0.4, 0, 0, 0.7, -0.5, 0.5, -0.1, 0.1]),
    ("nose_bridge", [0, 0, 0.5, 0, 0, 0, 0, -0.6, 0.6]),
    ("nose_tip", [0, 0, 0, 0, 0, 0, 0, 0.4, 0]),
    ("nostril_width*", [0, 0, 0, 0, -0.1, 0.2, -0.2, 0, 0]),
    ("eye_spacing*", [0, 0, 0, 0, 0, 0.3, -0.3, 0, 0]),
    ("mouth_width*", [0, 0, 0.1, -0.1, 0, 0.3, -0.2, 0, 0]),
    ("upper_lip_fullness", [0.1, 0, 0, 0, 0, 0, 0, 0.7, -0.2]),
    ("lower_lip_fullness", [0.1, 0, 0, 0, 0, 0, 0, 0.7, -0.2]),
    # R4's controls, where they serve a preset (face README §3: "with the
    # proposed parameters, add jaw_angle, + for square and angular, - for
    # round, heart and soft"; the rest by the presets' sentences).
    ("jaw_angle*", [-0.6, -0.5, 0.7, -0.5, 0, 0.6, -0.5, -0.7, 0.7]),
    ("nose_width", [0.2, 0, 0, 0, -0.1, 0.6, -0.4, 0, 0]),
    ("nose_tip_projection", [0, 0, 0, 0, 0.7, 0, 0.5, -0.4, 0.3]),
    ("nose_tip_rotation", [0, 0, 0, 0, 0, 0, 0, 0.2, -0.1]),
    ("philtrum_length", [-0.3, 0, 0, 0, 0.7, -0.6, 0.5, 0, 0]),
    ("lower_lid*", [0.3, 0, 0, 0, 0, 0, 0, 0.7, -0.2]),
    ("upper_lid_fold*", [0, 0, 0, 0, 0, 0, 0, 0.2, -0.3]),
    ("eye_corner*", [0, 0, 0, 0.2, 0, 0, 0, 0, 0.3]),
]

# The six characters the editor had in code (`presets()`), in its order:
# (name, label, about, parameters). The first is the default character.
CHARACTERS = [
    ("male", "Male", "The realistic male body, as it's built: a start for any character.", {}),
    ("female", "Female", "The realistic female body, as it's built.", {"frame": 1}),
    ("athletic-male", "Athletic male", "Lean and muscular.", {"muscle": 0.7, "fat": -0.4}),
    ("athletic-female", "Athletic female", "Lean and toned.", {"frame": 1, "muscle": 0.6, "fat": -0.4}),
    ("heavyset-male", "Heavyset male", "A heavier build.", {"fat": 0.8, "muscle": 0.2}),
    ("heavyset-female", "Heavyset female", "A heavier build.", {"frame": 1, "fat": 0.8, "muscle": 0.2}),
]


# Nose shapes: (name, label, about, parameters; `*` is a pair, both sides
# alike). Each is the nose parameters that read as that type in profile
# and from the front; what a type doesn't name is the nose as built.
NOSES = [
    ("straight", "Straight", "A straight bridge that runs from the brow to the tip without a dip or a hump.",
     {"nose_root": 0.8, "nose_bridge": 0.2, "nose_tip_width": -0.2, "nose_width": -0.2}),
    ("aquiline", "Aquiline", "A high bridge that bows out along its length, the tip turned a little down.",
     {"nose_hump": 0.7, "nose_root": 0.3, "nose_bridge": 0.3, "nose_tip_rotation": -0.35, "nose_length": 0.3}),
    ("hooked", "Hooked", "A strong hump and a sharp tip that hangs below the nostrils.",
     {"nose_hump": 1.0, "nose_bridge": 0.4, "nose_tip_rotation": -0.9, "nose_length": 0.5,
      "nose_tip_projection": 0.2, "nose_columella": 0.5, "nose_tip_width": -0.3}),
    ("snub", "Snub", "Short, with a scooped bridge and a tip that turns up.",
     {"nose_length": -0.6, "nose_hump": -0.6, "nose_tip_rotation": 0.6, "nose_tip_projection": -0.3,
      "nose_root": -0.3, "nose_tip_width": 0.3}),
    ("button", "Button", "Small and round: a short nose with a low bridge and a round tip.",
     {"nose_length": -0.8, "nose_bridge": -0.3, "nose_hump": -0.3, "nose_tip_rotation": 0.35,
      "nose_tip_width": 0.5, "nose_width": -0.2, "nostril_width*": -0.2, "nose_tip_projection": -0.2}),
    ("upturned", "Upturned", "A tip that turns well up, so the nostrils show from the front.",
     {"nose_tip_rotation": 0.9, "nose_hump": -0.4, "nose_length": -0.2, "nose_columella": -0.3,
      "nose_tip_projection": 0.2, "nostril_height*": 0.3}),
    ("bulbous", "Bulbous", "A wide, round tip over full nostrils.",
     {"nose_tip_width": 1.0, "nose_tip_projection": 0.2, "nostril_flare*": 0.6, "nostril_width*": 0.3,
      "nose_bridge_width": 0.3}),
    ("long-thin", "Long and thin", "Long from the bridge to the tip and narrow all the way down.",
     {"nose_length": 0.9, "nose_width": -0.7, "nose_bridge_width": -0.6, "nose_tip_width": -0.6,
      "nostril_width*": -0.5, "nose_tip_projection": 0.3, "nose_root": 0.3}),
    ("wide", "Wide", "A low, wide bridge over wide, flared nostrils.",
     {"nose_width": 0.8, "nostril_width*": 0.8, "nostril_flare*": 0.7, "nose_bridge": -0.6,
      "nose_bridge_width": 0.8, "nose_root": -0.5, "nose_tip_width": 0.5, "nose_tip_projection": -0.4}),
    ("low-bridge", "Low bridge", "A shallow root between the eyes and a low bridge under it.",
     {"nose_root": -1.0, "nose_bridge": -0.6, "nose_hump": -0.2, "nose_bridge_width": 0.3}),
]

# Face shapes: the outline's five parameters alone (feature `face_shape`).
# Oval is the face as built.
FACE_SHAPES = [
    ("oval", "Oval", "The face as built: a little longer than wide, tapering gently to the chin.", {}),
    ("round", "Round", "As wide as long, with full sides and no corners.",
     {"face_width": 0.5, "face_length": -0.6, "face_roundness": 1.0, "face_taper": -0.2}),
    ("square", "Square", "A jaw as wide as the cheekbones, with flat sides and a square corner.",
     {"face_taper": -0.9, "face_roundness": -0.8, "face_width": 0.3, "face_length": -0.2}),
    ("heart", "Heart-shaped", "Wide at the temples and cheekbones, narrowing to a small chin.",
     {"face_taper": 1.0, "face_width": 0.2, "face_roundness": -0.1, "face_narrow": 0.3}),
    ("long", "Long", "Long from the forehead to the chin and narrow across.",
     {"face_length": 1.0, "face_width": -0.4, "face_narrow": 0.3}),
    ("narrow", "Narrow", "Narrow across the temples, the cheekbones and the jaw.",
     {"face_width": -0.8, "face_narrow": 0.7, "face_length": 0.2}),
]


def paired(values):
    """`values` with each pair's stem (`name*`) as its left and right."""
    out = {}
    for parameter, value in values.items():
        if parameter.endswith("*"):
            out[f"{parameter[:-1]}_left"] = value
            out[f"{parameter[:-1]}_right"] = value
        else:
            out[parameter] = value
    return out


def preset(name, label, kind, about, parameters, order, tags=None, items=None, feature=None):
    block = {"id": f"axcenity/realistic-{name}", "kind": kind, "about": about}
    if feature:
        block["feature"] = feature
    if tags:
        block["tags"] = tags
    block["order"] = order
    out = {"format_version": 2, "name": label, "base": BASE}
    if parameters:
        out["parameters"] = dict(sorted(parameters.items()))
    if items:
        out["items"] = [{"id": item} for item in items]
    out["preset"] = block
    return name, out


def presets():
    out = []
    for k, (name, label, about, by_frame) in enumerate(BODY):
        for frame, values in by_frame.items():
            out.append(preset(f"body-{name}-{frame}", label, "body", about, values, 10 * (k + 1),
                              tags=["build", frame]))
    for k, (name, label, about) in enumerate(FACES):
        values = {}
        for parameter, row in FACE_TABLE:
            value = row[k]
            if value == 0:
                continue
            if parameter.endswith("*"):
                stem = parameter[:-1]
                values[f"{stem}_left"] = value
                values[f"{stem}_right"] = value
            else:
                values[parameter] = value
        out.append(preset(f"face-{name}", label, "face", about, values, 10 * (k + 1), tags=["shape"]))
    for k, (name, label, about, values) in enumerate(CHARACTERS):
        wears = FEMALE_WEARS if values.get("frame") == 1 else MALE_WEARS
        out.append(preset(name, label, "character", about, values, 10 * (k + 1), items=wears))
    for prefix, feature, rows in (("nose", "nose", NOSES), ("face-shape", "face_shape", FACE_SHAPES)):
        for k, (name, label, about, values) in enumerate(rows):
            out.append(preset(f"{prefix}-{name}", label, "feature", about, paired(values), 10 * (k + 1),
                              tags=["shape"], feature=feature))
    return out


def write(folder=FOLDER):
    os.makedirs(folder, exist_ok=True)
    for name, value in presets():
        with open(os.path.join(folder, f"{name}.preset.json"), "w", newline="\n") as f:
            json.dump(value, f, indent=2)
            f.write("\n")


if __name__ == "__main__":
    import sys
    write(sys.argv[1] if len(sys.argv) > 1 else FOLDER)
