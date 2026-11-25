"""Writes the benchmark characters (Character Studio overhaul CO2,
`docs/development/character-studio/visual-qa.md`): a fixed set of
description files per base model in the library, which the in-engine
capture and the Blender harness draw at every stage, so a change is judged
on the same characters before and after.

    python3 -I assets/characters/benchmarks/make.py

Plain Python, no Blender. Run it after a base model's parameters, slots or
items change; it rewrites every `<family>-<frame>-<case>.character.json`
here and removes the ones no longer made. The set, per base model:

- bodies: each frame (masculine, feminine, the 50/50 blend) at the default
  build, slim, heavy, muscular, heavy and muscular, the shortest and
  tallest proportions, and the narrowest and widest hips (R3: the hips
  narrow as well as widen), in underwear and bald so the body shows;
- faces: each frame neutral, and every face parameter at each end of its
  range;
- outfits: underwear only, the default outfit, every item alone, and the
  heaviest layering (one item in every slot);
- hair: every groom on each frame;
- the cast (refinement R1, brief §17.1): the same people on each base and
  frame, `<family>-<frame>-cast-<name>`: average, slim, athletic,
  muscular, heavyset, broad face, narrow face, distinctive features and
  natural asymmetry (where the base has sided face parameters). Bare:
  underwear and bald, until the family has clothes (`CAST`).

What "slim", "heavy" and "muscular" mean is per base model (`BUILDS`):
a base with fat and muscle layers names those.

What a file wears is what is drawn: where a parameter's choices fill a
slot (the human body's underwear follows Male or Female), the file
lists the choice nearest its value, ties going to the first as the
resolver's do (`avatar/src/choices.rs`), and no other item there.
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LIBRARY = os.path.dirname(HERE)

FRAMES = [("masculine", 0.0), ("feminine", 1.0), ("blend", 0.5)]

# Body cases by base model: case -> parameter values. A parameter the base
# lacks is skipped, and a case that sets nothing is left out.
BUILDS = {
    # The human base (MakeHuman's body, ADR-0047): the same corners of its
    # muscle and fat sliders, and its age.
    "axcenity/human": {
        "slim": {"fat": -1.0},
        "heavy": {"fat": 1.0},
        "muscular": {"muscle": 1.0},
        "slight": {"fat": -1.0, "muscle": -1.0},
        "heavy-soft": {"fat": 1.0, "muscle": -1.0},
        "lean-muscular": {"fat": -1.0, "muscle": 1.0},
        "heavy-muscular": {"fat": 1.0, "muscle": 1.0},
        "old": {"age": 1.0},
    },
}

# The parameters that make a body shorter or taller, wherever a base has
# them, at the ends of their ranges.
STATURE = ["height", "leg_length"]

# The cast, by base model: person -> parameter values, either one set for
# both frames or {"masculine": ..., "feminine": ...}. Sources (refinement
# Stage 0, audit-asset-pipeline.md "Benchmark plan proposal"):
# - athletic and heavyset are the workspace's Athletic and Heavyset presets
#   (`editor/src/character_workspace.rs` `presets()`), so the Presets page
#   and the benchmark show the same people;
# - broad-face and narrow-face are the Broad and Narrow face presets of
#   `docs/development/character-studio/references/face/README.md` §3;
# - asymmetry is that file's §4 "Benchmark asymmetry face" column: each
#   pair differs by that amount (on the -1 to 1 scale), the right side the
#   larger for sizes, the left the higher for heights, and the lower face
#   (jaw, nose, mouth) all to one side. That is 1.6 to 3.7 mm a feature,
#   the 1 to 3 per cent of a paired measurement healthy faces show (its
#   F-13 to F-17), just at the size people start to notice.
def sided(differences):
    """Pairs from left-minus-right differences, split evenly about 0."""
    out = {}
    for name, difference in differences.items():
        out[f"{name}_left"] = difference / 2
        out[f"{name}_right"] = -difference / 2
    return out


CAST = {}

# Bases whose neutral face is the cast's average: `face-neutral` was the
# same character as `body-default` (0 pixels differ in the refinement
# baseline), so it isn't written; `cast-average` is the face reference.
NEUTRAL_FACE_IN_CAST = {"axcenity/human"}

# The human base's cast: bodies, faces set with
# MakeHuman's own sliders (ADR-0047), and its skins (ADR-0048): `skin` is
# the choice (0 African, 1 Asian, 2 Caucasian, the default, 3 the second
# Caucasian pair, 4 the painted bodysuit); which of a choice's skins is
# drawn follows the frame and `age`, so the cast shows young, middle-aged
# and old ones of each.
CAST["axcenity/human"] = {
    "average": {},
    "slim": {"fat": -0.6, "muscle": -0.2, "skin": 1},
    "athletic": {
        "masculine": {"muscle": 0.7, "fat": -0.4, "skin": 0},
        "feminine": {"muscle": 0.6, "fat": -0.4, "skin": 0},
    },
    "muscular": {"muscle": 0.9, "fat": 0.1, "shoulder_width": 1.05, "skin": 3},
    "heavyset": {"fat": 0.8, "muscle": 0.2, "age": 0.5, "skin": 0},
    "broad-face": {"face_shape_scale_horiz": 0.6, "chin_width": 0.5, "face_shape_round": 0.4},
    "narrow-face": {"face_shape_scale_horiz": -0.6, "chin_width": -0.5, "face_shape_oval": 0.4},
    "distinctive": {"age": 0.6, "nose_hump": 0.7, "nose_scale_vert": 0.4, "chin_prominent": 0.6,
                    "brows_trans_down_up": -0.5, "skin": 1},
    # Each pair differs between its sides.
    "asymmetry": sided({
        "eyes_trans_down_up": 0.35, "eyes_scale": -0.25, "eyes_corner1_down_up": 0.25,
        "cheeks_bones": -0.6, "ears_trans_down_up": 0.7, "ears_wing": -0.7, "ears_scale": -0.7,
    }),
    "elder": {"age": 0.9, "fat": -0.3, "skin": 0},
    "elder-light": {"age": 1.0, "skin": 1},
    "bodysuit": {"skin": 4},
}
# Bases whose face parameters aren't each given a file at both ends: the
# human base's 149 are MakeHuman's own list, held by the library's random
# characters test instead.
NO_FACE_ENDS = {"axcenity/human"}


def read(path):
    with open(path, encoding="utf-8") as file:
        return json.load(file)


def library():
    catalogue = read(os.path.join(LIBRARY, "catalogue.json"))
    bases = [read(os.path.join(LIBRARY, entry["manifest"])) for entry in catalogue["bases"]]
    items = [read(os.path.join(LIBRARY, entry["manifest"])) for entry in catalogue["items"]]
    return bases, items


def description(name, base, parameters, items):
    out = {"format_version": 2, "name": name, "base": base["id"]}
    known = {p["id"]: p for p in base["parameters"]}
    values = {}
    for pid, value in parameters.items():
        if pid in known:
            low, high = known[pid]["range"]
            values[pid] = max(low, min(high, value))
    if values:
        out["parameters"] = dict(sorted(values.items()))
    if items:
        out["items"] = [{"id": item} for item in items]
    return out


def nearest_choice(parameter, value):
    """The choice nearest `value`, the first on a tie (as Rust's `min_by`)."""
    choices = parameter.get("choices", [])
    return min(choices, key=lambda choice: abs(choice["value"] - value)) if choices else None


def benchmarks(base, items):
    """(case name, description) for one base model."""
    fits = [item for item in items if base["id"] in item["bases"]]
    fit_ids = {item["id"] for item in fits}
    slot_of = {item["id"]: item["slot"] for item in fits}
    by_slot = {}
    for item in fits:
        by_slot.setdefault(item["slot"], []).append(item)
    parameters = {p["id"]: p for p in base["parameters"]}
    # The slots a parameter's choices fill: what's there follows the value.
    chosen = {
        slot_of[item]
        for p in base["parameters"]
        for choice in p.get("choices", [])
        for item in choice.get("items", [])
        if item in fit_ids
    }

    def choices_for(values):
        """The items the choices of a description with `values` wear."""
        out = []
        for p in base["parameters"]:
            choice = nearest_choice(p, values.get(p["id"], p.get("default", 0.0)))
            if choice:
                out += [item for item in choice.get("items", []) if item in fit_ids]
        return out

    def dressed(values, worn):
        """`worn` with the chosen slots' items replaced by the choices'."""
        return choices_for(values) + [item for item in worn if slot_of.get(item) not in chosen]

    underwear = [
        by_slot[slot][0]["id"]
        for slot in sorted(by_slot)
        if slot.startswith("underwear") and slot not in chosen
    ]
    groups = {g["id"]: g["category"] for g in base.get("groups", [])}
    has_frame = "frame" in parameters
    frames = FRAMES if has_frame else [("body", 0.0)]
    family = base["family"]

    def named(frame, case):
        return f"{family}-{frame}-{case}"

    def make(frame, value, case, values, worn):
        with_frame = dict(values)
        if has_frame:
            with_frame["frame"] = value
        name = named(frame, case)
        return name, description(name, base, with_frame, dressed(with_frame, worn))

    def frame_of(value):
        return {"frame": value} if has_frame else {}

    out = []
    builds = {
        case: {pid: v for pid, v in values.items() if pid in parameters}
        for case, values in BUILDS.get(base["id"], {}).items()
    }
    stature = [pid for pid in STATURE if pid in parameters]
    for frame, value in frames:
        out.append(make(frame, value, "body-default", {}, underwear))
        for case, values in builds.items():
            if values:
                out.append(make(frame, value, f"body-{case}", values, underwear))
        if stature:
            short = {pid: parameters[pid]["range"][0] for pid in stature}
            tall = {pid: parameters[pid]["range"][1] for pid in stature}
            out.append(make(frame, value, "body-shortest", short, underwear))
            out.append(make(frame, value, "body-tallest", tall, underwear))
        if "hip_width" in parameters:
            low, high = parameters["hip_width"]["range"]
            out.append(make(frame, value, "body-narrowest-hips", {"hip_width": low}, underwear))
            out.append(make(frame, value, "body-widest-hips", {"hip_width": high}, underwear))
    face = [p for p in base["parameters"] if groups.get(p["group"]) == "face"]
    for frame, value in frames[:2]:
        if base["id"] not in NEUTRAL_FACE_IN_CAST:
            out.append(make(frame, value, "face-neutral", {}, underwear))
        for parameter in [] if base["id"] in NO_FACE_ENDS else face:
            low, high = parameter["range"]
            pid = parameter["id"].replace("_", "-")
            out.append(make(frame, value, f"face-{pid}-min", {parameter["id"]: low}, underwear))
            out.append(make(frame, value, f"face-{pid}-max", {parameter["id"]: high}, underwear))
    defaults = base.get("defaults", {}).get("items", [])
    hair = [item["id"] for item in fits if item["kind"] == "groom"]
    for frame, value in frames[:2]:
        bare = dressed(frame_of(value), underwear)
        out.append(make(frame, value, "outfit-underwear", {}, underwear))
        # Outfits that would only repeat the underwear (every slot chosen)
        # are left out.
        if defaults and dressed(frame_of(value), defaults) != bare:
            out.append(make(frame, value, "outfit-default", {}, defaults))
        # One item in every slot, the outermost layer a slot offers.
        layered = [
            max(by_slot[slot], key=lambda item: item.get("layer") or 0)["id"]
            for slot in sorted(by_slot)
        ]
        if layered and dressed(frame_of(value), layered) != bare:
            out.append(make(frame, value, "outfit-layered", {}, layered))
        for item in hair:
            short = item.split("/")[-1]
            out.append(make(frame, value, short, {}, underwear + [item]))
    # Every garment and accessory alone, on the masculine frame. What a
    # choice wears can't be worn alone: its slot follows the choice.
    frame, value = frames[0]
    for item in fits:
        if item["kind"] != "groom" and item["slot"] not in chosen:
            short = item["id"].split("/")[-1]
            out.append(make(frame, value, f"item-{short}", {}, [item["id"]]))
    # The cast, on the masculine and feminine frames, bare.
    for person, values in CAST.get(base["id"], {}).items():
        for frame, value in frames[:2]:
            mine = values.get(frame, values) if "masculine" in values else values
            out.append(make(frame, value, f"cast-{person}", mine, underwear))
    return out


def main():
    bases, items = library()
    written = set()
    for base in bases:
        for name, body in benchmarks(base, items):
            path = os.path.join(HERE, f"{name}.character.json")
            text = json.dumps(body, indent=2) + "\n"
            written.add(os.path.basename(path))
            if not os.path.isfile(path) or open(path, encoding="utf-8").read() != text:
                with open(path, "w", encoding="utf-8", newline="\n") as file:
                    file.write(text)
    for name in sorted(os.listdir(HERE)):
        if name.endswith(".character.json") and name not in written:
            os.remove(os.path.join(HERE, name))
    print(f"{len(written)} benchmark characters in {os.path.relpath(HERE)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
