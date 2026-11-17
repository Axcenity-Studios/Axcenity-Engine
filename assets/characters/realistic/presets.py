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
# per face above, in that order.
FACE_TABLE = [
    ("forehead_height", [-0.2, 0.2, 0, 0.2, 0.5, -0.2, 0.1, 0, 0]),
    ("forehead_slope", [0.2, 0, 0, 0.1, 0, 0, 0, 0.3, -0.2]),
    ("temple_width", [0.3, -0.1, 0.3, 0.5, -0.3, 0.6, -0.6, 0, 0]),
    ("brow_ridge", [-0.2, 0, 0.2, 0, 0, 0, 0, -0.5, 0.4]),
    ("cheekbone*", [-0.2, 0.2, 0, 0.5, -0.2, 0.6, -0.4, -0.2, 0.6]),
    ("cheek_fullness*", [0.6, -0.1, -0.2, -0.2, -0.3, 0.2, -0.3, 0.4, -0.6]),
    ("jaw_width*", [0.3, -0.3, 0.7, -0.6, -0.3, 0.5, -0.5, -0.2, 0.3]),
    ("chin_width", [0.4, -0.2, 0.5, -0.6, 0, 0.3, -0.3, 0.1, -0.2]),
    ("chin_length", [-0.4, 0.1, -0.1, 0.2, 0.6, -0.2, 0.1, -0.1, 0.1]),
    ("chin_forward", [-0.1, 0, 0.1, 0, 0, 0, 0, -0.2, 0.3]),
    ("nose_length", [-0.1, 0, 0, 0, 0.4, -0.1, 0.1, -0.1, 0.1]),
    ("nose_bridge", [0, 0, 0, 0, 0, 0, 0, -0.3, 0.4]),
    ("nose_tip", [0, 0, 0, 0, 0, 0, 0, 0.2, 0]),
    ("nostril_width*", [0, 0, 0, 0, -0.1, 0.2, -0.2, 0, 0]),
    ("eye_spacing*", [0, 0, 0, 0, 0, 0.3, -0.3, 0, 0]),
    ("mouth_width*", [0, 0, 0.1, -0.1, 0, 0.3, -0.2, 0, 0]),
    ("upper_lip_fullness", [0.1, 0, 0, 0, 0, 0, 0, 0.3, -0.2]),
    ("lower_lip_fullness", [0.1, 0, 0, 0, 0, 0, 0, 0.3, -0.2]),
    # R4's controls, where they serve a preset (face README §3: "with the
    # proposed parameters, add jaw_angle, + for square and angular, - for
    # round, heart and soft"; the rest by the presets' sentences).
    ("jaw_angle*", [-0.4, 0, 0.6, -0.3, 0, 0.2, 0, -0.5, 0.6]),
    ("nose_width", [0.2, 0, 0, 0, -0.1, 0.4, -0.4, 0, 0]),
    ("nose_tip_projection", [0, 0, 0, 0, 0.3, 0, 0, -0.2, 0.3]),
    ("nose_tip_rotation", [0, 0, 0, 0, 0, 0, 0, 0.2, -0.1]),
    ("philtrum_length", [-0.3, 0, 0, 0, 0.4, -0.2, 0, 0, 0]),
    ("lower_lid*", [0.3, 0, 0, 0, 0, 0, 0, 0.4, -0.2]),
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


def preset(name, label, kind, about, parameters, order, tags=None, items=None):
    block = {"id": f"axcenity/realistic-{name}", "kind": kind, "about": about}
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
    return out


def write(folder=FOLDER):
    os.makedirs(folder, exist_ok=True)
    for name, value in presets():
        with open(os.path.join(folder, f"{name}.preset.json"), "w", newline="\n") as f:
            json.dump(value, f, indent=2)
            f.write("\n")


if __name__ == "__main__":
    write()
