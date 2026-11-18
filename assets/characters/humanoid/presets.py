"""The stylized base model's shipped presets (Character Studio refinement,
wave 3; plan section 0 decision 7: both families are selectable, so the
stylized one has presets too). Each is a format 2 character description
with a `preset` block, written to `presets/<name>.preset.json` beside
`Humanoid.base.json`, made with the stylized parameters. Plain Python:

    python -I assets/characters/humanoid/presets.py
    python -I assets/characters/catalogue.py

- **Body**: Slim, Athletic, Heavy and Broad, one file per end of the
  stylized frame (a masculine-to-feminine slider, not a choice). A body
  preset owns every body parameter, `frame` included, so each names its
  frame; Character Studio offers a character the ones made for the end its
  frame is nearer (`presets::suits`), so applying one keeps the frame.
- **Face**: Round, Long, Angular and Soft, from the stylized face
  parameters (head, jaw, cheekbones, eyes, nose, lips).
- **Character**: Masculine (the stylized default character, as built),
  Feminine and Athletic, wearing the default outfit; New lists them.

Starting values to tune against captures, not types of people.
"""

import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
FOLDER = os.path.join(HERE, "presets")
BASE = "axcenity/stylized"
OUTFIT = ["axcenity/tee", "axcenity/cropped-jacket", "axcenity/cargo-pants", "axcenity/sneakers"]

# (name, label, about, {frame label: {parameter: value}}). Frame 0 is the
# masculine end, 1 the feminine.
BODY = [
    ("slim", "Slim", "A small frame with little muscle or fat.",
     {"masculine": {"frame": 0, "build": -0.6, "shoulder_width": 0.95},
      "feminine": {"frame": 1, "build": -0.6, "hip_width": 0.96}}),
    ("athletic", "Athletic", "Trained: more muscle, a little narrower at the waist.",
     {"masculine": {"frame": 0, "build": -0.2, "muscle": 0.7, "shoulder_width": 1.08,
                    "waist_width": 0.95},
      "feminine": {"frame": 1, "build": -0.2, "muscle": 0.6, "shoulder_width": 1.04,
                   "waist_width": 0.96}}),
    ("heavy", "Heavy", "Soft and heavy.",
     {"masculine": {"frame": 0, "build": 0.8, "waist_width": 1.1},
      "feminine": {"frame": 1, "build": 0.8, "hip_width": 1.1}}),
    ("broad", "Broad", "A wide, solid frame: wider shoulders and hips.",
     {"masculine": {"frame": 0, "build": 0.3, "muscle": 0.4, "shoulder_width": 1.12,
                    "hip_width": 1.05},
      "feminine": {"frame": 1, "build": 0.3, "muscle": 0.3, "shoulder_width": 1.1,
                   "hip_width": 1.08}}),
]

FACES = [
    ("round", "Round", "A wide, short head with full cheeks and a short chin.",
     {"head_width": 0.4, "head_length": -0.3, "cheekbones": -0.2, "jaw_width": 0.3,
      "chin_length": -0.3}),
    ("long", "Long", "A long head and chin, a longer nose, narrow across.",
     {"head_length": 0.5, "head_width": -0.2, "chin_length": 0.4, "nose_length": 0.3}),
    ("angular", "Angular", "High cheekbones, a wide jaw and a chin set forward.",
     {"cheekbones": 0.6, "jaw_width": 0.5, "chin_forward": 0.3, "brows_height": -0.2}),
    ("soft", "Soft", "Big eyes, full lips, a narrow jaw and a low bridge.",
     {"eyes_size": 0.3, "lips_full": 0.4, "jaw_width": -0.3, "cheekbones": -0.3,
      "nose_bridge": -0.2}),
]

# (name, label, about, parameters, hair). The first is the default
# character as built (its manifest's default items).
CHARACTERS = [
    ("masculine", "Masculine", "The stylized body as it's built: a start for any character.",
     {}, "axcenity/hair-swept"),
    ("feminine", "Feminine", "The stylized body at its feminine frame.",
     {"frame": 1}, "axcenity/hair-long"),
    ("athletic", "Athletic", "Lean and muscular.",
     {"build": -0.2, "muscle": 0.7, "shoulder_width": 1.08}, "axcenity/hair-crop"),
]


def preset(name, label, kind, about, parameters, order, tags=None, items=None):
    block = {"id": f"axcenity/stylized-{name}", "kind": kind, "about": about}
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
    for k, (name, label, about, values) in enumerate(FACES):
        out.append(preset(f"face-{name}", label, "face", about, values, 10 * (k + 1),
                          tags=["shape"]))
    for k, (name, label, about, values, hair) in enumerate(CHARACTERS):
        out.append(preset(name, label, "character", about, values, 10 * (k + 1),
                          items=[hair] + OUTFIT))
    return out


def write(folder=FOLDER):
    os.makedirs(folder, exist_ok=True)
    for name, value in presets():
        with open(os.path.join(folder, f"{name}.preset.json"), "w", newline="\n") as f:
            json.dump(value, f, indent=2)
            f.write("\n")


if __name__ == "__main__":
    write()
