"""Turns MakeHuman clothes (`.mhclo` assets from the MakeHuman community's
asset packs) into items of the human base, by driving the installed MPFB
add-on (`mh.py`; none of its code is in this repository).

    blender -b --python assets/characters/human/wardrobe.py -- \
        --assets <an unpacked asset pack>... --system <MakeHuman system assets, unpacked> \
        --out <library>/human/wardrobe [--only <text>]... [--list]

Each garment is fitted to the body by MPFB, brought into the engine's pose
on the body's skeleton with MakeHuman's own weights, and given the body's
shapes (what its clothes file makes of each of them), so it follows every
body slider with nothing to fit at run time (`fit: shapes`). Its cover
map (ADR-0045, over the body's texture coordinates) says what it hides:
red, the skin left out under it, is the body corners its file hides
(`delete_verts`); green and blue, where a garment worn beneath it leaves
its own cloth out, are where its cloth lies over the skin
(`build.cloth_cover`). Each of its corners carries its place on the skin
(`build.skin_uvs_of`), so worn under another garment it is cut and its
edge sunk as a garment the engine fitted.

**Licence gate.** (This converter's, for a library outside the
repository; the repository itself takes only the CC0 ones of what is
converted: `take_library.py`.) A garment is taken only when its own file's header and
its pack's metadata both say CC0 or CC BY, and agree (`vet`). Anything
else is left out with the reason (some packs listed as CC0 hold files
whose headers still say AGPL3, and outfits of characters that aren't
their authors' to release). What is taken is recorded per item, in its
manifest's `source` and in `CREDITS.md` beside the items.

Writes, per garment, `<name>/<Name>.glb`, `<Name>.cover.png`,
`<Name>.png` (its thumbnail, when the pack has one) and
`<Name>.item.json`; and `CREDITS.md` and `wardrobe.report.json` for all.
"""

import argparse
import json
import os
import re
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402
import numpy as np  # noqa: E402

import build as body_build  # noqa: E402
import manifests  # noqa: E402
import mh  # noqa: E402

SOURCE_URL = "https://static.makehumancommunity.org/assets/assetpacks.html"
LICENCE_ID = {"CC0": "CC0-1.0", "CC-BY": "CC-BY-4.0"}
# Names that are someone else's character or brand: never taken, whatever
# the file says of its licence.
NOT_OURS = ("spider", "gwen", "ayanami", "batman", "superman", "mario", "zelda", "disney", "marvel")

# How a garment is worn, by words in its name, its tags or its pack's
# name: (words, slot, layer, also fills). First match wins.
KINDS = (
    (("shoe", "boot", "sneaker", "sandal", "heel", "slipper", "loafer"), "shoes", 1, ()),
    (("stocking", "pantyhose", "sock", "tights", "legwarmer", "leg_warmer"), "legwear", 0, ()),
    (("glove", "mitten", "armsleeve", "arm_sleeve"), "gloves", 1, ()),
    (("hat", "fedora", "cap", "helmet", "hood", "beanie", "crown", "headband"), "headwear", None, ()),
    # Underwear and swimwear are a top or a bottom of layer 0, worn over
    # the underwear cut from the body, which they hide where they cover
    # it: the base's own underwear slots are the body type's
    # (`avatar::chosen_slots`), and nothing else can be worn in them.
    (("bra", "bikini_top", "bikinitop", "camisole", "bandeau"), "top", 0, ()),
    (("underwear", "brief", "boxer", "panties", "panty", "thong", "lingerie", "bikini", "swim", "slip"),
     "bottom", 0, ()),
    (("dress", "gown", "robe", "tunic", "jumpsuit", "overall", "suit", "uniform", "costume", "outfit",
      "romper", "babydoll", "teddy"), "top", 1, ("bottom",)),
    (("skirt", "kilt"), "bottom", 1, ()),
    (("pants", "trouser", "jeans", "shorts", "legging"), "bottom", 1, ()),
    (("jacket", "coat", "blazer", "hoodie", "vest", "cardigan", "cape", "cloak"), "outerwear", 2, ()),
    (("shirt", "top", "tee", "sweater", "blouse", "tank", "polo", "pullover", "jersey", "turtleneck",
      "bodice", "halter"), "top", 1, ()),
)


def kind_of(name, tags, entry):
    """(slot, layer, also fills) from the garment's name and tags and its
    pack's category, or None when none of them says."""
    category = (entry or {}).get("category", "").lower()
    words_in = "_" + re.sub(r"[^a-z0-9]+", "_", " ".join([name] + tags).lower()) + "_"
    for words, slot, layer, fills in KINDS:
        if any(w in words_in for w in words):
            # In an underwear pack a top is worn as underwear: layer 0.
            if category == "underwear" and slot == "top" and not fills:
                return "top", 0, []
            # A top made tucked in is worn under the bottoms (layer 0:
            # `ItemManifest::wearing_order`), so their waistband hides it.
            if slot == "top" and not fills and "_tucked_" in words_in:
                return slot, 0, []
            return slot, layer, list(fills)
    by_category = {"underwear": ("bottom", 0, []), "shirts": ("top", 1, []), "pants": ("bottom", 1, []),
                   "shoes": ("shoes", 1, []), "gloves": ("gloves", 1, []), "hats": ("headwear", None, []),
                   "skirts": ("bottom", 1, []), "dress": ("top", 1, ["bottom"]), "suits": ("top", 1, ["bottom"])}
    return by_category.get(category)


def object_name(name):
    return "".join(part.capitalize() for part in re.split(r"[^A-Za-z0-9]+", name) if part)


# Names for people, where the asset's own name is a file name.
LABELS = {
    "male_casualsuit01": "Shirt and jeans", "female_casualsuit01": "T-shirt and jeans",
    "shoes01": "Brown leather shoes", "shoes04": "Black shoes",
}
# The pack's name for MakeHuman's own assets, as their files credit them.
AUTHORS = {"makehuman_system": "The MakeHuman team (Data Collection AB, Joel Palmius, Jonas Hauquier)"}


def label_of(name):
    if name in LABELS:
        return LABELS[name]
    words = [w for w in re.split(r"[^A-Za-z0-9]+", name) if w]
    return " ".join(words).capitalize()


def pack_metadata(root):
    """{asset name: its entry} from every `packs/*.json` under `root`."""
    out = {}
    folder = os.path.join(root, "packs")
    if os.path.isdir(folder):
        for name in sorted(os.listdir(folder)):
            if name.endswith(".json"):
                with open(os.path.join(folder, name), encoding="utf-8") as f:
                    for asset, entry in json.load(f).items():
                        out[asset] = dict(entry, pack=os.path.splitext(name)[0])
    return out


def vet(clothes, entry):
    """(licence, author, url) when the garment may be taken, else a reason."""
    name = clothes.name.lower()
    if any(word in name for word in NOT_OURS):
        return None, "names a character or brand that isn't the author's to release"
    header = clothes.licence()
    listed = (entry or {}).get("license", "").upper().replace(" ", "-").replace("CC-BY-4.0", "CC-BY")
    listed = "CC-BY" if listed.startswith("CC-BY") else listed
    if header == "AGPL":
        return None, "its file header says AGPL3"
    if entry is None:
        return None, "it is in no pack's metadata"
    if listed not in LICENCE_ID:
        return None, f"its pack lists the licence `{entry.get('license', '')}`"
    if header != "unknown" and header != listed:
        return None, f"its file header says {header}, its pack {listed}"
    author = entry.get("author") or clothes.author() or "unknown"
    author = AUTHORS.get(author, author)
    if listed == "CC-BY" and author == "unknown":
        return None, "CC BY with no author to credit"
    url = entry.get("source") or SOURCE_URL
    return (listed, author, url), None


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--assets", required=True, nargs="+")
    parser.add_argument("--system", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--only", action="append", help="a garment whose file name has this text (may repeat)")
    parser.add_argument("--list", action="store_true")
    args = parser.parse_args(argv)

    found = []
    for root in args.assets:
        metadata = pack_metadata(root)
        for folder, _, files in sorted(os.walk(os.path.join(root, "clothes"))):
            for name in sorted(files):
                if name.endswith(".mhclo"):
                    found.append((os.path.join(folder, name), metadata))
    if args.only:
        found = [f for f in found if any(o.lower() in os.path.basename(f[0]).lower() for o in args.only)]

    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    build = body_build.Build(args.system)
    joints = build.joints()
    body_hidden_of = np.full(len(build.rest), -1, dtype=np.int64)
    body_hidden_of[build.skin] = np.arange(len(build.skin))
    carried = set(build.body_shapes())
    body_points = build.body_points()
    body_groups, _ = body_build.humanoid_weights(build.body.weights, build.names, build.parent_of)

    taken, left = [], []
    work = []
    for path, metadata in found:
        clothes = mh.Clothes(path)
        entry = metadata.get(clothes.name) or metadata.get(os.path.basename(os.path.dirname(path)))
        ok, reason = vet(clothes, entry)
        kind = kind_of(clothes.name, clothes.tags, entry)
        if ok and kind is None:
            ok, reason = None, "no slot could be told from its name, tags or pack"
        if not ok:
            left.append({"file": os.path.relpath(path, os.path.dirname(os.path.dirname(os.path.dirname(path)))),
                         "reason": reason})
            continue
        if args.list:
            taken.append({"name": clothes.name, "licence": ok[0], "author": ok[1], "slot": kind[0]})
            continue
        obj = mh.add_asset(build.base, path, "Clothes")
        piece, points, shapes = build.hung(obj, clothes)
        groups, loose = body_build.humanoid_weights(piece.weights, build.names, build.parent_of)
        if loose.any():
            left.append({"file": path, "reason": f"{int(loose.sum())} corners with no weights"})
            continue
        hidden = np.zeros(len(build.skin), dtype=bool)
        deleted = body_hidden_of[[d for d in clothes.delete if d < len(body_hidden_of)]]
        hidden[deleted[deleted >= 0]] = True
        mhmat = os.path.join(clothes.folder, clothes.fields.get("material", ""))
        work.append((clothes, entry, ok, kind, piece, points, shapes, groups, hidden, mhmat))

    if not args.list:
        mh.remove(build.base)
        for o in list(bpy.data.objects):
            bpy.data.objects.remove(o)
        os.makedirs(args.out, exist_ok=True)
        for clothes, entry, (licence, author, url), (slot, layer, fills), piece, points, shapes, groups, hidden, mhmat in work:
            name = object_name(clothes.name)
            folder = os.path.join(args.out, clothes.name.lower().replace(" ", "-"))
            os.makedirs(folder, exist_ok=True)
            armature = body_build.build_armature(joints)
            try:
                material = body_build.material_of(name, mhmat, alpha="cut")
                obj = body_build.make_object(name, piece, points, shapes, groups, material, armature,
                                             body_build.skin_uvs_of(build.body, body_points, points))
                body_build.export(os.path.join(folder, name + ".glb"), armature, [obj])
            except Exception as error:  # one bad file mustn't stop the rest
                left.append({"file": clothes.path, "reason": f"it couldn't be made: {error}"})
                shutil.rmtree(folder, ignore_errors=True)
                continue
            finally:
                for o in list(bpy.data.objects):
                    bpy.data.objects.remove(o)
            # What is worn beneath it is left out under its cloth (the
            # cover map's green; blue for a top tucked into a bottom).
            under, whole = body_build.cloth_cover(build.body, body_points, body_groups, points, piece.faces,
                                                  hidden, bottom=slot == "bottom")
            lit = body_build.write_cover(os.path.join(folder, name + ".cover.png"), build.body, hidden, under,
                                         whole)
            item = {
                "format_version": 1,
                "id": "makehuman/" + clothes.name.lower().replace("_", "-").replace(" ", "-"),
                "label": label_of(clothes.name),
                "kind": "garment" if layer is not None else "accessory",
                "slot": slot,
            }
            if layer is not None:
                item["layer"] = layer
            item.update({
                "tier": "built_in",
                "bases": [manifests.BASE_ID],
                "model": {"file": name + ".glb", "object": f"{manifests.ARMATURE}/{name}"},
                "cover_map": name + ".cover.png",
                "materials": [{"name": name, "shading": "standard"}],
                "fit": {"method": "shapes", "follows": sorted(set(shapes) & carried)},
                "tags": sorted({t.lower() for t in clothes.tags if "makehuman" not in t.lower()} | {"makehuman"}),
            })
            thumb = os.path.join(clothes.folder, os.path.splitext(os.path.basename(clothes.path))[0] + ".thumb")
            if os.path.exists(thumb):
                # MakeHuman's `.thumb` files are PNG images.
                shutil.copyfile(thumb, os.path.join(folder, name + ".png"))
                item["thumbnail"] = name + ".png"
            item["source"] = {"author": author, "licence": LICENCE_ID[licence], "url": url}
            if fills:
                item["also_fills"] = fills
            item["quality"] = "draft"
            with open(os.path.join(folder, name + ".item.json"), "w", newline="\n") as f:
                json.dump(item, f, indent=2)
                f.write("\n")
            taken.append({"id": item["id"], "label": item["label"], "name": clothes.name, "licence": licence,
                          "author": author, "url": url, "pack": entry.get("pack", ""), "slot": slot,
                          "corners": int(len(points)), "shapes": len(shapes), "hidden_faces": lit})
        credits(os.path.join(args.out, "CREDITS.md"), taken)

    report = {"mpfb": mh.version(), "taken": taken, "left_out": left}
    if not args.list:
        with open(os.path.join(args.out, "wardrobe.report.json"), "w") as f:
            json.dump(report, f, indent=1)
    print(f"human wardrobe: {len(taken)} taken, {len(left)} left out")
    for entry in left:
        print("  left out:", entry["file"], "-", entry["reason"])


def credits(path, taken, what="garment"):
    """`CREDITS.md`: who made each garment (or hairstyle: `what`) and
    under which licence. CC BY asks for this; CC0 doesn't, and it is given
    all the same."""
    script = "hair.py" if what == "hairstyle" else "wardrobe.py"
    lines = [
        "# Hair credits" if what == "hairstyle" else "# Wardrobe credits",
        "",
        f"Every {what} here is a MakeHuman community asset, converted for Axcenity's human base by",
        f"`assets/characters/human/{script}` with the MPFB add-on (https://static.makehumancommunity.org/mpfb.html).",
        "The meshes were re-posed, re-skinned to the Axcenity Humanoid skeleton and given the body's shapes;",
        "textures are as their authors made them. Source: " + SOURCE_URL,
        "",
        "| Item | Author | Licence | Pack |",
        "|---|---|---|---|",
    ]
    for t in sorted(taken, key=lambda t: t["id"]):
        licence = "CC0 1.0" if t["licence"] == "CC0" else "CC BY 4.0"
        lines.append(f"| {t['label']} (`{t['id']}`) | {t['author']} | {licence} | {t['pack']} |")
    lines += ["", "CC BY 4.0: https://creativecommons.org/licenses/by/4.0/ . CC0 1.0: https://creativecommons.org/publicdomain/zero/1.0/", ""]
    with open(path, "w", newline="\n") as f:
        f.write("\n".join(lines))


if __name__ == "__main__":
    main()
