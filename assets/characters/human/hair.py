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
alone casts its shadow through its holes. Its maps are made from
MakeHuman's painted texture (`maps`): the colour map is the painted
strands brought to one authored brown, so the colour variants (`VARIANTS`)
mean the same on every hairstyle, and the flow map is the direction the
painted strands run, which is where the Hair model puts its highlights.

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
import cut  # noqa: E402
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
# The colour every hairstyle's texture is brought to (sRGB): the realistic
# hairstyle's authored brown, the pigment curve's middle (`hair.md`), so
# the colour variants mean the same on every hairstyle.
AUTHORED = (0.283, 0.203, 0.137)
# How much of the painted strands' light and dark is kept round that
# colour (a share of it, lowest and highest), and over how many texels of
# a 2048 map the painting's own shading is taken out.
DETAIL = (0.7, 1.3)
SHADING_RADIUS = 40
FLOW_RADIUS = 14
ROUGHNESS = 0.45
# Colours: the realistic hairstyle's own variants, which are absolute
# because the texture is brought to that hairstyle's authored brown first
# (`maps`): black is black on the hairstyle MakeHuman painted platinum.
VARIANTS = [
    ("black", "Black", {"melanin": 0.95, "redness": -0.3}),
    ("dark-brown", "Dark brown", {"melanin": 0.5, "redness": 0.0}),
    ("brown", "Brown", {"melanin": 0.1, "redness": 0.15}),
    ("auburn", "Auburn", {"melanin": 0.0, "redness": 0.9}),
    ("ginger", "Ginger", {"melanin": -0.45, "redness": 1.0}),
    ("blonde", "Blonde", {"melanin": -0.85, "redness": 0.35}),
    ("platinum", "Platinum", {"melanin": -1.0, "redness": -0.5, "dye": "#e9e2d0", "dye_amount": 0.35}),
    ("grey", "Grey", {"melanin": -0.75, "redness": -1.0, "dye": "#bdbcb8", "dye_amount": 0.55}),
    ("blonde-dark-roots", "Blonde, dark roots", {"melanin": -0.85, "redness": 0.35, "root_melanin": 0.9, "root_length": 0.3}),
    ("brown-highlights", "Brown, highlights", {"melanin": 0.1, "redness": 0.15, "streaks": 1.0, "streak_melanin": -0.8}),
]
# The pigment curve alone hardly moves the hue (`hair.md`, `redness`): on
# these flat-lit meshes auburn, ginger and blonde came out brown, beige
# and grey (captured 2026-11-24). A dye of the colour meant brings it.
HUES = {
    "auburn": {"melanin": -0.2, "dye": "#8a3a1c", "dye_amount": 0.5},
    "ginger": {"melanin": -0.5, "dye": "#b5562a", "dye_amount": 0.5},
    "blonde": {"melanin": -0.9, "dye": "#d9b877", "dye_amount": 0.55},
}


def maps(texture, folder, name, size):
    """A hairstyle's maps from MakeHuman's painted texture `texture`,
    written into `folder`: (colour map, flow map). Plain image work
    (Pillow, NumPy), no MakeHuman code.

    **Colour.** MakeHuman paints each hairstyle in one colour with its
    shading baked in (a dark cap under the afro's curls, highlights on the
    bob). The colour variants need one known colour to start from, and the
    Hair model lights the strands itself, so the painting is reduced to
    what the strands are: its brightness against the brightness round it
    (the baked shading goes), as light and dark round `AUTHORED`. Clear
    texels take the colour too, so nothing dark bleeds in at an edge.

    **Flow.** The Hair model shines along the strands, which it takes to
    run up the texture unless a flow map says otherwise; MakeHuman's
    don't. The painted strands show their own direction: across a strand
    the brightness changes, along it it doesn't. The flow map is that
    direction at each texel (the structure tensor's quiet axis, smoothed),
    pointing down the texture where it can, since nothing in the painting
    says which end is the root."""
    import numpy as np
    from PIL import Image, ImageFilter

    image = Image.open(texture).convert("RGBA")
    if max(image.size) > size:
        image = image.resize((size, size), Image.LANCZOS)
    rgba = np.asarray(image, dtype=np.float32) / 255.0
    alpha = rgba[..., 3]
    light = rgba[..., :3] @ np.array([0.2126, 0.7152, 0.0722], dtype=np.float32)

    def blurred(values, radius):
        out = Image.fromarray(np.clip(values * 255.0, 0, 255).astype(np.uint8)).filter(ImageFilter.BoxBlur(radius))
        return np.asarray(out, dtype=np.float32) / 255.0

    radius = max(2, SHADING_RADIUS * image.size[0] // 2048)
    cover = np.maximum(blurred(alpha, radius), 1e-3)
    around = blurred(light * alpha, radius) / cover
    detail = np.clip(light / np.maximum(around, 0.02), DETAIL[0], DETAIL[1])
    detail = np.where(alpha > 0.02, detail, 1.0)
    colour = np.clip(np.array(AUTHORED, dtype=np.float32)[None, None, :] * detail[..., None], 0.0, 1.0)
    out = np.dstack([colour, alpha[..., None]])
    colour_path = os.path.join(folder, name + "_Colour.png")
    Image.fromarray((out * 255.0 + 0.5).astype(np.uint8), "RGBA").save(colour_path, optimize=True)

    # The direction the brightness changes least in, smoothed.
    gy, gx = np.gradient(light * alpha)
    radius = max(1, FLOW_RADIUS * image.size[0] // 2048)
    # Blurring signed products: shift them into 0 to 1 and back.
    def smooth(values):
        span = max(float(np.abs(values).max()), 1e-6)
        return (blurred(values / span * 0.5 + 0.5, radius) - 0.5) * 2.0 * span
    xx, yy, xy = smooth(gx * gx), smooth(gy * gy), smooth(gx * gy)
    # The tensor's major axis is across the strands, at this angle:
    across = 0.5 * np.arctan2(2.0 * xy, xx - yy)
    dx, dy = -np.sin(across), np.cos(across)
    flip = (dy < 0) | ((dy == 0) & (dx < 0))
    dx, dy = np.where(flip, -dx, dx), np.where(flip, -dy, dy)
    # Where the painting says nothing (flat or clear), straight down.
    strength = np.sqrt((xx - yy) ** 2 + 4.0 * xy ** 2)
    quiet = strength < strength.max() * 1e-4
    dx, dy = np.where(quiet, 0.0, dx), np.where(quiet, 1.0, dy)
    flow = np.dstack([0.5 + 0.5 * dx, 0.5 + 0.5 * dy, np.zeros_like(dx)])
    flow_path = os.path.join(folder, name + "_Flow.png")
    Image.fromarray((flow * 255.0 + 0.5).astype(np.uint8), "RGB").save(flow_path, optimize=True)
    return colour_path, flow_path


def add_flow(path, flow_path):
    """Puts the flow map into the model file `path` and names it in its
    material's `extras` (`axcenity_hair_flow`, `docs/rendering/hair.md`):
    the exporter writes a material's colour map, not a map only the engine
    reads. A glTF binary is a JSON chunk and one buffer; this appends the
    image to the buffer."""
    import struct
    with open(path, "rb") as f:
        data = f.read()
    length = struct.unpack("<I", data[12:16])[0]
    doc = json.loads(data[20:20 + length])
    start = 20 + length + 8
    binary = bytearray(data[start:start + struct.unpack("<I", data[20 + length:24 + length])[0]])
    with open(flow_path, "rb") as f:
        png = f.read()
    while len(binary) % 4:
        binary.append(0)
    doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(png)})
    binary += png
    while len(binary) % 4:
        binary.append(0)
    doc["buffers"][0]["byteLength"] = len(binary)
    doc.setdefault("images", []).append({"name": "Hair_Flow", "mimeType": "image/png",
                                         "bufferView": len(doc["bufferViews"]) - 1})
    texture = {"source": len(doc["images"]) - 1}
    if doc.get("samplers"):
        texture["sampler"] = 0
    doc.setdefault("textures", []).append(texture)
    for material in doc["materials"]:
        if material.get("extras", {}).get("axcenity_shading") == "hair":
            material["extras"]["axcenity_hair_flow"] = {"index": len(doc["textures"]) - 1}
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    body = (struct.pack("<II", len(text), 0x4E4F534A) + text
            + struct.pack("<II", len(binary), 0x004E4942) + bytes(binary))
    with open(path, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, 2, 12 + len(body)) + body)


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
                painted = body_build.read_mhmat(mhmat).get("diffuseTexture") if os.path.exists(mhmat) else None
                if extras and painted and os.path.exists(painted):
                    import tempfile
                    with tempfile.TemporaryDirectory(prefix="human-hair-") as scratch:
                        colour, flow = maps(painted, scratch, name, args.texture_size)
                        material = body_build.simple_material(MATERIAL, (1.0, 1.0, 1.0, 1.0), colour, None, "cut",
                                                              roughness=ROUGHNESS, extras=extras)
                        obj = body_build.make_object(name, piece, points, shapes, groups, material, armature)
                        body_build.export(os.path.join(folder, name + ".glb"), armature, [obj], extras=True,
                                          morph_normals=False)
                        add_flow(os.path.join(folder, name + ".glb"), flow)
                    raise StopIteration
                material = body_build.material_of(MATERIAL, mhmat, alpha="cut", extras=extras)
                obj = body_build.make_object(name, piece, points, shapes, groups, material, armature)
                # No normals with the shapes: they are half of a hairstyle's
                # file, and a head's shape barely turns its hair's.
                body_build.export(os.path.join(folder, name + ".glb"), armature, [obj], extras=bool(extras),
                                  morph_normals=False)
            except StopIteration:  # made above, with its own maps
                pass
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
            }
            # Its own tags, and who it is made for (`cut.py`).
            item["tags"] = cut.tags_with_cut(item["id"], set(words) | {"makehuman"})
            if args.shading == "hair":
                # Not the variants that need a data map (dark roots, streaks).
                item["variants"] = [{"id": vid, "label": vlabel,
                                     "materials": {MATERIAL: {**values, **HUES.get(vid, {})}}}
                                    for vid, vlabel, values in VARIANTS if set(values) <= set(VALUES)]
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
