"""The human base's skins: MakeHuman's photographed skin textures (the
system assets' `skins/` folder, CC0), one for each age, ethnicity and sex,
written as library images beside the manifest and never into the model
file (ADR-0048).

A skin is **not** a slider: the base's manifest lists every skin with the
parameter values it was made for (`material_maps`: the `skin` choice, the
body type, the age), and a character draws the one nearest its own values.
So the sex of the skin follows the body type, its age follows the Age
slider, and the one thing a person picks is the `skin` choice (`TONES`).
Between two ages the skin is a mix of the two photographs (each map's
`blend`: the Skin shading model draws both), so nothing snaps as the Age
slider moves.

Every MakeHuman skin is laid out on the same texture coordinates (the base
mesh's own), which is what lets one material draw any of them.

Plain Python and Pillow; `build.py` calls `write`.
"""

import os

from PIL import Image

FOLDER = "skins"
MATERIAL = "Skin"
#: The model file's own colour map, drawn until a character's skin is read
#: (and by a character with no skin named): the default character's.
DEFAULT = "young_caucasian_male"

#: The `skin` choice, in order: (id, label, MakeHuman's name for it). The
#: labels are MakeHuman's own names for the people photographed.
TONES = (
    ("african", "African", "african"),
    ("asian", "Asian", "asian"),
    ("caucasian", "Caucasian", "caucasian"),
    ("caucasian_2", "Caucasian 2", "caucasian"),
    ("bodysuit", "Bodysuit", "caucasian"),
)
DEFAULT_TONE = "caucasian"
#: The last choice Randomize draws: the bodysuit is a costume painted on
#: the skin, offered and never drawn at random.
LAST_RANDOM = "caucasian_2"
#: MakeHuman's three ages, at the Age slider's values they stand for (0 is
#: a young adult, 1 old), and its two sexes at the body type's.
AGES = (("young", 0.0), ("middleage", 0.5), ("old", 1.0))
SEXES = (("male", 0.0), ("female", 1.0))
CC0 = "explicitly released as CC0"


def folders():
    """Every skin as (tone id, age value, body type value, MakeHuman's
    folder name). MakeHuman has one skin for each age, ethnicity and sex,
    a second young Caucasian pair, and a young pair with a bodysuit
    painted on: the second pair is listed at the older ages with the first
    Caucasian pair's skins (so it ages into them), and the bodysuit at
    every age, so the costume doesn't come off as the Age slider moves."""
    out = []
    for tone, _, name in TONES[:3]:
        for age, at in AGES:
            for sex, frame in SEXES:
                out.append((tone, at, frame, f"{age}_{name}_{sex}"))
    for sex, frame in SEXES:
        for age, at in AGES:
            out.append(("caucasian_2", at, frame,
                        f"young_caucasian_{sex}2" if age == "young" else f"{age}_caucasian_{sex}"))
            out.append(("bodysuit", at, frame, f"young_caucasian_{sex}_special_suit"))
    return out


def read(assets, name):
    """(the texture's path, the licence line) of the MakeHuman skin
    `name`, from its material file."""
    folder = os.path.join(assets, "skins", name)
    with open(os.path.join(folder, name + ".mhmat"), encoding="utf-8", errors="replace") as f:
        text = f.read()
    texture = None
    for line in text.splitlines():
        words = line.split()
        if len(words) == 2 and words[0] == "diffuseTexture":
            texture = os.path.join(folder, words[1])
    return texture, CC0 in text


def write(assets, out, size=2048, quality=90):
    """Writes every skin as `skins/<name>.jpg` in `out` and returns what
    the manifest needs: {"choices": [...], "maps": [...], "left_out":
    [...]}. A skin whose file doesn't say CC0, or has no texture, is left
    out with the reason."""
    os.makedirs(os.path.join(out, FOLDER), exist_ok=True)
    index = {tone: k for k, (tone, _, _) in enumerate(TONES)}
    written, maps, left_out = {}, [], []
    for tone, age, frame, name in folders():
        if name not in written:
            texture, cc0 = read(assets, name)
            if not cc0:
                left_out.append({"skin": name, "why": "its material file doesn't say CC0"})
                written[name] = None
            elif not texture or not os.path.exists(texture):
                left_out.append({"skin": name, "why": "it has no texture"})
                written[name] = None
            else:
                image = Image.open(texture).convert("RGB")
                if max(image.size) > size:
                    image = image.resize((size, size), Image.LANCZOS)
                file = f"{FOLDER}/{name}.jpg"
                image.save(os.path.join(out, file), quality=quality, optimize=True)
                written[name] = file
        if written[name]:
            maps.append({"material": MATERIAL, "colour_map": written[name],
                         "at": {"skin": index[tone], "frame": frame, "age": age},
                         # Between two ages the skin is a mix of both.
                         "blend": "age"})
    # A file a rebuild no longer writes doesn't stay behind.
    kept = {os.path.basename(file) for file in written.values() if file}
    for file in os.listdir(os.path.join(out, FOLDER)):
        if file.endswith(".jpg") and file not in kept:
            os.remove(os.path.join(out, FOLDER, file))
    return {
        "choices": [{"value": k, "id": tone, "label": label} for k, (tone, label, _) in enumerate(TONES)
                    if any(m["at"]["skin"] == k for m in maps)],
        "default": index[DEFAULT_TONE],
        "last_random": index[LAST_RANDOM],
        "maps": maps,
        "files": sorted(file for file in written.values() if file),
        "left_out": left_out,
    }
