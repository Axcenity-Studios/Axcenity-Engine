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
  build, slim, heavy, muscular, heavy and muscular, and the shortest and
  tallest proportions, in underwear and bald so the body shows;
- faces: each frame neutral, and every face parameter at each end of its
  range;
- outfits: underwear only, the default outfit, every item alone, and the
  heaviest layering (one item in every slot);
- hair: every groom on each frame.

What "slim", "heavy" and "muscular" mean is per base model (`BUILDS`):
the stylized body has one slim-to-heavy axis and a muscle slider; a base
with fat and muscle layers names those.
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
    "axcenity/stylized": {
        "slim": {"build": -1.0},
        "heavy": {"build": 1.0},
        "muscular": {"muscle": 1.0},
        "heavy-muscular": {"build": 1.0, "muscle": 1.0},
    },
    # The realistic base's muscle and fat sliders (CO5): each alone at an
    # end, and the four corners of the two together. `fat` is the body's
    # weight: with little muscle it is fat, with much it is bulk.
    "axcenity/realistic": {
        "slim": {"fat": -1.0},
        "heavy": {"fat": 1.0},
        "muscular": {"muscle": 1.0},
        "slight": {"fat": -1.0, "muscle": -1.0},
        "heavy-soft": {"fat": 1.0, "muscle": -1.0},
        "lean-muscular": {"fat": -1.0, "muscle": 1.0},
        "heavy-muscular": {"fat": 1.0, "muscle": 1.0},
    },
}

# The parameters that make a body shorter or taller, wherever a base has
# them, at the ends of their ranges.
STATURE = ["height", "leg_length"]


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


def benchmarks(base, items):
    """(case name, description) for one base model."""
    fits = [item for item in items if base["id"] in item["bases"]]
    by_slot = {}
    for item in fits:
        by_slot.setdefault(item["slot"], []).append(item)
    underwear = [by_slot[slot][0]["id"] for slot in sorted(by_slot) if slot.startswith("underwear")]
    parameters = {p["id"]: p for p in base["parameters"]}
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
        return name, description(name, base, with_frame, worn)

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
    face = [p for p in base["parameters"] if groups.get(p["group"]) == "face"]
    for frame, value in frames[:2]:
        out.append(make(frame, value, "face-neutral", {}, underwear))
        for parameter in face:
            low, high = parameter["range"]
            pid = parameter["id"].replace("_", "-")
            out.append(make(frame, value, f"face-{pid}-min", {parameter["id"]: low}, underwear))
            out.append(make(frame, value, f"face-{pid}-max", {parameter["id"]: high}, underwear))
    defaults = base.get("defaults", {}).get("items", [])
    hair = [item["id"] for item in fits if item["kind"] == "groom"]
    for frame, value in frames[:2]:
        out.append(make(frame, value, "outfit-underwear", {}, underwear))
        if defaults:
            out.append(make(frame, value, "outfit-default", {}, defaults))
        # One item in every slot, the outermost layer a slot offers.
        layered = [
            max(by_slot[slot], key=lambda item: item.get("layer") or 0)["id"]
            for slot in sorted(by_slot)
        ]
        if layered:
            out.append(make(frame, value, "outfit-layered", {}, layered))
        for item in hair:
            short = item.split("/")[-1]
            out.append(make(frame, value, short, {}, underwear + [item]))
    # Every garment and accessory alone, on the masculine frame.
    frame, value = frames[0]
    for item in fits:
        if item["kind"] != "groom":
            short = item["id"].split("/")[-1]
            out.append(make(frame, value, f"item-{short}", {}, [item["id"]]))
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
