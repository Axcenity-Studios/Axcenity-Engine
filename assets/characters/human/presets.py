"""The human base's shipped presets, written to `presets/<name>.preset.json`.
Plain Python:

    python -I assets/characters/human/presets.py

A preset is a character description with a `preset` block (architecture.md
section 15a). Kinds here:

- **character**: a whole character to start from (Male, Female), the
  body type alone, so "Start from this" on the start page has the base.
- **body**: a build (slim, athletic, heavyset). It names no body type.
- **face**: a whole face. Every face slider it doesn't name goes back to
  its default when it is applied.
- **feature**: one feature's sliders (the face's outline, the nose), and
  no other feature's.

Values are on the sliders `face.NAMES` names; none is past 0.8, and each
preset moves only what its sentence says. They were set by eye from engine
captures on 2026-11-24 and are starting points, not likenesses.
"""

import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
FOLDER = os.path.join(HERE, "presets")
BASE = "axcenity/human"


def both(values):
    """`values` with each `*_side` slider given to the left and the right."""
    out = {}
    for key, value in values.items():
        if key.endswith("_side"):
            out[key[:-5] + "_left"] = value
            out[key[:-5] + "_right"] = value
        else:
            out[key] = value
    return out


# (file, label, about, values, the underwear its body type wears).
CHARACTERS = [
    ("male", "Male", "The human base's male body, as it's built: a start for any character.", {},
     ["axcenity/human-boxer-briefs"]),
    ("female", "Female", "The human base's female body, as it's built.", {"frame": 1},
     ["axcenity/human-boxer-briefs", "axcenity/human-bandeau"]),
]
# Builds: a body preset names no body type, so it never changes it.
BODIES = [
    ("average", "Average", "The body as built.", {}),
    ("slim", "Slim", "Little fat and little muscle.", {"fat": -0.6, "muscle": -0.2}),
    ("athletic", "Athletic", "Lean, with trained muscle.", {"muscle": 0.65, "fat": -0.4}),
    ("muscular", "Muscular", "Heavy muscle and broad shoulders.", {"muscle": 0.9, "fat": 0.1, "shoulder_width": 1.05}),
    ("heavyset", "Heavyset", "A heavy body with some muscle under it.", {"fat": 0.8, "muscle": 0.2}),
    ("elder", "Elderly", "An old, spare body.", {"age": 0.9, "fat": -0.3, "muscle": -0.3}),
]
FACES = [
    ("soft", "Soft features", "Full cheeks, a short round chin and a small nose.", {
        "face_shape_round": 0.4, "cheeks_volume_side": 0.5, "chin_height": -0.3, "chin_width": 0.2,
        "chin_bones": -0.5, "nose_scale_depth": -0.3, "nose_volume": 0.2, "lips_upperlip_volume": 0.3,
        "lips_lowerlip_volume": 0.3, "brows_trans_backward_forward": -0.3}),
    ("angular", "Angular features", "High cheekbones over hollow cheeks, a strong jaw and chin.", {
        "cheeks_bones_side": 0.6, "cheeks_volume_side": -0.6, "chin_bones": 0.7, "chin_prominent": 0.4,
        "brows_trans_backward_forward": 0.5, "nose_hump": 0.3, "face_shape_fat": -0.3}),
    ("broad", "Broad face", "A wide head and jaw, a wide nose and mouth.", {
        "face_shape_scale_horiz": 0.6, "chin_width": 0.5, "chin_bones": 0.4, "nose_scale_horiz": 0.4,
        "lips_scale_horiz": 0.3, "cheeks_bones_side": 0.3}),
    ("narrow", "Narrow face", "A narrow head and chin, a long thin nose.", {
        "face_shape_scale_horiz": -0.6, "chin_width": -0.5, "nose_scale_horiz": -0.4, "nose_scale_vert": 0.3,
        "lips_scale_horiz": -0.3, "cheeks_bones_side": -0.3}),
    ("long", "Long face", "A tall head, a high forehead and a long chin.", {
        "face_shape_scale_vert": 0.5, "forehead_scale_vert": 0.4, "chin_height": 0.5, "nose_scale_vert": 0.3}),
    ("heavy", "Heavy face", "A full face with a double chin and a thick neck.", {
        "face_shape_fat": 0.8, "neck_double": 0.7, "neck_neck_circ": 0.5, "cheeks_volume_side": 0.6,
        "chin_jaw_drop": 0.4}),
    ("gaunt", "Gaunt face", "Little fat on the face: hollow cheeks and temples, a thin neck.", {
        "face_shape_fat": -0.8, "cheeks_volume_side": -0.7, "forehead_temple": -0.4, "neck_neck_circ": -0.4,
        "eyes_bag_side": 0.3}),
    ("weathered", "Older face", "An aged face: lines, heavier lids and a little sag under the chin.", {
        "face_shape_age": 0.8, "eyes_bag_side": 0.5, "eyes_eyefold_down_up_side": -0.4,
        "lips_laugh_lines_in_out": -0.6, "chin_jaw_drop": 0.3, "lips_upperlip_volume": -0.3}),
]
# One feature each: (file, label, about, feature, values).
FEATURES = [
    ("face-shape-oval", "Oval", "Widest at the cheeks, a rounded jaw.", "face_shape", {"face_shape_oval": 0.7}),
    ("face-shape-round", "Round", "As wide as it is long, a soft jaw.", "face_shape", {"face_shape_round": 0.7}),
    ("face-shape-square", "Square", "A wide jaw with strong corners.", "face_shape", {"face_shape_square": 0.7}),
    ("face-shape-long", "Long", "A long face with a square jaw.", "face_shape", {"face_shape_rectangular": 0.7}),
    ("face-shape-heart", "Heart", "Wide at the temples, narrow at the chin.", "face_shape",
     {"face_shape_invertedtriangular": 0.7}),
    ("face-shape-pear", "Pear", "Widest at the jaw.", "face_shape", {"face_shape_triangular": 0.7}),
    ("face-shape-diamond", "Diamond", "Narrow temples and jaw, wide cheekbones.", "face_shape",
     {"face_shape_diamond": 0.7}),
    ("nose-straight", "Straight", "A straight bridge of middling width.", "nose", {"nose_greek": 0.3}),
    ("nose-aquiline", "Aquiline", "A high bridge that bows out, the tip turned a little down.", "nose",
     {"nose_hump": 0.7, "nose_curve_concave_convex": 0.4, "nose_point_down_up": -0.4, "nose_scale_depth": 0.3}),
    ("nose-button", "Button", "Short and small, the tip turned up.", "nose",
     {"nose_scale_vert": -0.5, "nose_scale_depth": -0.3, "nose_point_down_up": 0.5, "nose_volume": -0.2}),
    ("nose-wide", "Wide", "A wide nose with flared nostrils and a low bridge.", "nose",
     {"nose_scale_horiz": 0.6, "nose_flaring": 0.5, "nose_nostrils_width": 0.4, "nose_hump": -0.4,
      "nose_width2": 0.4}),
    ("nose-long-thin", "Long and thin", "A long, narrow nose.", "nose",
     {"nose_scale_vert": 0.6, "nose_scale_horiz": -0.5, "nose_point_width": -0.4}),
    ("nose-bulbous", "Bulbous", "A wide, round tip.", "nose",
     {"nose_volume": 0.8, "nose_point_width": 0.6, "nose_nostrils_width": 0.3}),
    ("nose-snub", "Snub", "A scooped bridge and a short upturned tip.", "nose",
     {"nose_hump": -0.6, "nose_curve_concave_convex": -0.5, "nose_point_down_up": 0.6, "nose_scale_vert": -0.3}),
]


def preset(name, label, about, kind, parameters, order, feature=None, items=None):
    block = {"id": f"axcenity/human-{name}", "kind": kind, "about": about}
    if feature:
        block["feature"] = feature
    block["order"] = order
    out = {"format_version": 2, "name": label, "base": BASE}
    if parameters:
        out["parameters"] = dict(sorted(both(parameters).items()))
    if items:
        out["items"] = [{"id": item} for item in items]
    out["preset"] = block
    return out


def write():
    os.makedirs(FOLDER, exist_ok=True)
    made = {}
    for k, (name, label, about, values, items) in enumerate(CHARACTERS):
        made[name] = preset(name, label, about, "character", values, 10 * (k + 1), items=items)
    for k, (name, label, about, values) in enumerate(BODIES):
        made["body-" + name] = preset("body-" + name, label, about, "body", values, 10 * (k + 1))
    for k, (name, label, about, values) in enumerate(FACES):
        made["face-" + name] = preset("face-" + name, label, about, "face", values, 10 * (k + 1))
    for k, (name, label, about, feature, values) in enumerate(FEATURES):
        made[name] = preset(name, label, about, "feature", values, 10 * (k + 1), feature)
    for name, data in made.items():
        with open(os.path.join(FOLDER, name + ".preset.json"), "w", newline="\n") as f:
            json.dump(data, f, indent=2)
            f.write("\n")
    for file in os.listdir(FOLDER):  # a preset this no longer writes doesn't stay
        if file.endswith(".preset.json") and file[:-len(".preset.json")] not in made:
            os.remove(os.path.join(FOLDER, file))
    return len(made)


if __name__ == "__main__":
    print(f"{write()} human presets in {FOLDER}")
