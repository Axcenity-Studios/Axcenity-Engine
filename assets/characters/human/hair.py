"""Turns MakeHuman hairstyles (`hair/*/*.mhclo` in the system assets and
the community's hair packs) into hair items of the human base, by driving
the installed MPFB add-on (`mh.py`; none of its code is in this
repository).

    blender -b --python assets/characters/human/hair.py -- \
        --assets <an unpacked asset pack>... --system <MakeHuman system assets, unpacked> \
        --out <library>/human/hair [--only <text>]... [--list] [--shading hair|standard]

A MakeHuman hairstyle is a clothes-type asset: a mesh of a few thousand
faces with a painted texture, hung on the head and shoulders by its
`.mhclo` file. It is converted exactly as a garment is (`wardrobe.py`):
fitted by MPFB, brought into the engine's pose on the body's skeleton with
MakeHuman's weights, and given the body's shapes, so it follows every body
and face slider with nothing fitted at run time (`fit: shapes`). It isn't
given the face's expression shapes. What differs is the item it becomes: a `groom` in the `hair` slot, with its mesh
as the groom's `mesh` tier, no layer and no cover map.

**Shading.** The material is marked for the engine's Hair shading model
(`extras.axcenity_shading`, `docs/rendering/hair.md`) and cut out by its
texture's alpha: under TAA a hair cut-out's edge is hashed and soft, and it
alone casts its shadow through its holes. The texture is MakeHuman's
painted one, colour and all; the item's colour variants move it along the
Hair model's pigment curve (`VARIANTS`).

**Licence gate.** As for garments (`wardrobe.vet`): the file's header and
its pack's metadata both say CC0 or CC BY, and agree.

Writes, per hairstyle, `<name>/<Name>.glb`, `<Name>.png` (its thumbnail)
and `<Name>.item.json`; and `CREDITS.md` and `hair.report.json` for all.
"""

import argparse
import json
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402

import build as body_build  # noqa: E402
import manifests  # noqa: E402
import mh  # noqa: E402
import wardrobe  # noqa: E402

MATERIAL = "Hair"
# Names for people, where the asset's own name is a file name, and what a
# person would search for.
LABELS = {
    "afro01": ("Afro", ["short", "coily"]),
    "bob01": ("Bob", ["medium", "straight"]),
    "bob02": ("Blunt bob with a fringe", ["medium", "straight", "fringe"]),
    "braid01": ("Long braid", ["long", "braided", "tied"]),
    "long01": ("Long and straight", ["long", "straight"]),
    "ponytail01": ("Ponytail", ["long", "tied"]),
    "short01": ("Short crop", ["short"]),
    "short02": ("Short and tousled", ["short"]),
    "short03": ("Short, swept to the side", ["short"]),
    "short04": ("Short, brushed forward", ["short"]),
}
# The Hair model's values a person may set on a painted texture: the ones
# that need a data map (roots, tips, streaks) have nothing to read.
VALUES = ["melanin", "redness", "dye", "dye_amount", "tint", "highlight", "shift_scale", "roughness_scale"]
# Colours. The texture is painted in one colour (black, brown, platinum),
# so the first five are moves from it along the pigment curve and are
# named as moves; the last three mix in a dye and land near the same
# colour on every hairstyle (`melanin` first, so a dark texture takes it).
VARIANTS = [
    ("as-painted", "As painted", {}),
    ("darker", "Darker", {"melanin": 0.45}),
    ("darkest", "Darkest", {"melanin": 1.0, "redness": -0.3}),
    ("lighter", "Lighter", {"melanin": -0.45}),
    ("lightest", "Lightest", {"melanin": -1.0}),
    ("dyed-blonde", "Dyed blonde", {"melanin": -1.0, "redness": 0.35, "dye": "#d9b877", "dye_amount": 0.6}),
    ("dyed-auburn", "Dyed auburn", {"melanin": -0.5, "redness": 1.0, "dye": "#8a3a1c", "dye_amount": 0.55}),
    ("dyed-grey", "Dyed grey", {"melanin": -0.75, "redness": -1.0, "dye": "#bdbcb8", "dye_amount": 0.6}),
]


def label_of(name):
    if name in LABELS:
        return LABELS[name]
    return wardrobe.label_of(name), []


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--assets", required=True, nargs="+")
    parser.add_argument("--system", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--only", action="append", help="a hairstyle whose file name has this text (may repeat)")
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--shading", choices=("hair", "standard"), default="hair")
    parser.add_argument("--texture-size", type=int, default=2048)
    args = parser.parse_args(argv)

    found = []
    for root in args.assets:
        metadata = wardrobe.pack_metadata(root)
        for folder, _, files in sorted(os.walk(os.path.join(root, "hair"))):
            for name in sorted(files):
                if name.endswith(".mhclo"):
                    found.append((os.path.join(folder, name), metadata))
    if args.only:
        found = [f for f in found if any(o.lower() in os.path.basename(f[0]).lower() for o in args.only)]

    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    build = body_build.Build(args.system)
    joints = build.joints()
    carried = set(build.body_shapes())

    taken, left, work = [], [], []
    for path, metadata in found:
        clothes = mh.Clothes(path)
        entry = metadata.get(clothes.name) or metadata.get(os.path.basename(os.path.dirname(path)))
        ok, reason = wardrobe.vet(clothes, entry)
        if not ok:
            left.append({"file": os.path.relpath(path, os.path.dirname(os.path.dirname(os.path.dirname(path)))),
                         "reason": reason})
            continue
        if args.list:
            taken.append({"name": clothes.name, "licence": ok[0], "author": ok[1]})
            continue
        obj = mh.add_asset(build.base, path, "Hair")
        piece, points, shapes = build.hung(obj, clothes)
        # Hair keeps to the head's shape, not to its expressions: a fringe
        # bound to the brow would rise with it, and a bob's sides part with
        # a smile.
        shapes = {name: moves for name, moves in shapes.items() if name not in build.expressions}
        groups, loose = body_build.humanoid_weights(piece.weights, build.names, build.parent_of)
        if loose.any():
            left.append({"file": path, "reason": f"{int(loose.sum())} corners with no weights"})
            continue
        mhmat = os.path.join(clothes.folder, clothes.fields.get("material", ""))
        work.append((clothes, entry, ok, piece, points, shapes, groups, mhmat))

    if not args.list:
        mh.remove(build.base)
        for o in list(bpy.data.objects):
            bpy.data.objects.remove(o)
        os.makedirs(args.out, exist_ok=True)
        # A hairstyle lies on the head: a shell, lit as one (`hair.md`).
        extras = {"axcenity_shading": "hair", "axcenity_hair_shell": True} if args.shading == "hair" else None
        for clothes, entry, (licence, author, url), piece, points, shapes, groups, mhmat in work:
            key = clothes.name.lower().replace(" ", "-")
            name = "Hair" + wardrobe.object_name(clothes.name)
            folder = os.path.join(args.out, key)
            os.makedirs(folder, exist_ok=True)
            armature = body_build.build_armature(joints)
            try:
                material = body_build.material_of(MATERIAL, mhmat, alpha="cut", extras=extras)
                obj = body_build.make_object(name, piece, points, shapes, groups, material, armature)
                # No normals with the shapes: they are half of a hairstyle's
                # file, and a head's shape barely turns its hair's.
                body_build.export(os.path.join(folder, name + ".glb"), armature, [obj], extras=bool(extras),
                                  morph_normals=False)
            except Exception as error:  # one bad file mustn't stop the rest
                left.append({"file": clothes.path, "reason": f"it couldn't be made: {error}"})
                shutil.rmtree(folder, ignore_errors=True)
                continue
            finally:
                for o in list(bpy.data.objects):
                    bpy.data.objects.remove(o)
                for m in list(bpy.data.materials):
                    bpy.data.materials.remove(m)
            label, words = label_of(clothes.name)
            item = {
                "format_version": 1,
                "id": "makehuman/hair-" + key.replace("_", "-"),
                "label": label,
                "kind": "groom",
                "slot": "hair",
                "tier": "built_in",
                "bases": [manifests.BASE_ID],
                "materials": [{"name": MATERIAL, "shading": args.shading, "label": "Hair", "role": "primary",
                               **({"values": VALUES} if args.shading == "hair" else {})}],
                "fit": {"method": "shapes", "follows": sorted(set(shapes) & carried)},
                "groom": {"mesh": {"file": name + ".glb", "object": f"{manifests.ARMATURE}/{name}"}},
                "tags": sorted(set(words) | {"makehuman"}),
            }
            if args.shading == "hair":
                item["variants"] = [{"id": vid, "label": vlabel, **({"materials": {MATERIAL: values}} if values else {})}
                                    for vid, vlabel, values in VARIANTS]
            thumb = os.path.join(clothes.folder, os.path.splitext(os.path.basename(clothes.path))[0] + ".thumb")
            if os.path.exists(thumb):
                shutil.copyfile(thumb, os.path.join(folder, name + ".png"))  # a `.thumb` is a PNG
                item["thumbnail"] = name + ".png"
            item["source"] = {"author": author, "licence": wardrobe.LICENCE_ID[licence], "url": url}
            item["quality"] = "draft"
            with open(os.path.join(folder, name + ".item.json"), "w", newline="\n") as f:
                json.dump(item, f, indent=2)
                f.write("\n")
            taken.append({"id": item["id"], "label": label, "name": clothes.name, "licence": licence,
                          "author": author, "url": url, "pack": entry.get("pack", ""), "slot": "hair",
                          "corners": int(len(points)), "faces": len(piece.faces), "shapes": len(shapes)})
        wardrobe.credits(os.path.join(args.out, "CREDITS.md"), taken, what="hairstyle")

    report = {"mpfb": mh.version(), "shading": args.shading, "taken": taken, "left_out": left}
    if not args.list:
        with open(os.path.join(args.out, "hair.report.json"), "w") as f:
            json.dump(report, f, indent=1)
    print(f"human hair: {len(taken)} taken, {len(left)} left out")
    for entry in left:
        print("  left out:", entry["file"], "-", entry["reason"])


if __name__ == "__main__":
    main()
