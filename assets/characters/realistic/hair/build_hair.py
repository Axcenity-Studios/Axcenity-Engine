"""Builds a realistic hairstyle (Character Studio R6, CO11) from its data
in `styles.py`: the groom on the realistic scalp, its cards and cap at
three levels (hero, gameplay, crowd), the atlas, the binding that carries
the head's shapes and skinning onto every vertex, the model file and the
item manifest. Needs the `bpy` module (Python 3.13 for bpy 5.x), NumPy
and Pillow; reads the built body (`../RealisticParts.glb`), so build the
body first.

    python -I assets/characters/realistic/hair/build_hair.py --style short-crop

Writes `hair/<File>.glb`, `hair/<File>.build.json` (what the build made
and measured) and `items/<id>.item.json`. Deterministic: the same body
and style give the same files.
"""

import argparse
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402,F401  (first: it makes `mathutils` importable)
import numpy as np  # noqa: E402

import atlas  # noqa: E402
import cards  # noqa: E402
import glb  # noqa: E402
import scalp  # noqa: E402
import styles  # noqa: E402
from groom import Groom  # noqa: E402

PACKAGE = os.path.dirname(HERE)
LEVELS = ("", "_LOD1", "_LOD2")


def manifest(style, follows):
    variants = []
    for vid, label, values in styles.COLOURS + styles.COLOURINGS:
        variants.append({"id": vid, "label": label, "materials": {"Hair": dict(values)}})
    # Each range's note is what was measured over it (styles.py
    # `measured`), not a fixed claim: a draft's notes say where it fails.
    limitations = [{"note": style["measured"][parameter], "parameter": parameter, "range": span}
                   for parameter, span in style["holds"].items()]
    return {
        "format_version": 1,
        "id": style["item"],
        "label": style["label"],
        "kind": "groom",
        "slot": "hair",
        "tier": "built_in",
        "bases": ["axcenity/realistic"],
        "materials": [{"name": "Hair", "shading": "hair", "label": "Hair", "values": styles.CARD_VALUES,
                       "role": "primary"}],
        "variants": variants,
        "fit": {"method": "shapes", "follows": sorted(follows)},
        "groom": {"cards": {"file": "../hair/" + style["file"],
                            "object": "AxcenityRealistic/" + style["object"]}},
        "tags": style["tags"],
        "source": {"author": "Axcenity Engine contributors", "licence": "CC0-1.0"},
        "about": style["about"],
        "limitations": limitations,
        # A draft until it passes every check of asset-quality.md §2 and §4
        # and Paul signs the first style off (§1).
        "quality": style.get("quality", "draft"),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--style", default="short-crop", choices=sorted(styles.STYLES))
    parser.add_argument("--parts", default=os.path.join(PACKAGE, "RealisticParts.glb"))
    parser.add_argument("--out", default=PACKAGE)
    args = parser.parse_args()
    style = styles.STYLES[args.style]
    body = scalp.Body(args.parts)
    groom = Groom(body, style)
    skeleton = glb.Skeleton(args.parts)
    report = {"style": style["id"], "levels": {}, "head_centre": [round(float(x), 5) for x in groom.head.centre],
              "crown": [round(float(x), 5) for x in groom.crown]}
    with tempfile.TemporaryDirectory() as tmp:
        islands, maps = atlas.bake(style["seed"], tmp, style["object"])
        built = []
        for level in range(3):
            pos, nrm, uv, tris, layers, roots = cards.build(groom, islands, level).arrays()
            binding = scalp.Binding(body, pos)
            order, weights = binding.weights()
            built.append((level, pos, nrm, uv, tris, layers, binding, order, weights))
        # Every level carries the same shapes: those that move any of them.
        names = set()
        for *_, binding, _, _ in built:
            names |= set(binding.shapes())
        meshes = []
        for level, pos, nrm, uv, tris, layers, binding, order, weights in built:
            shapes = {name: binding.shape(name) for name in sorted(names)}
            joint_names = [[body.bones[i] for i in row] for row in order]
            meshes.append((style["object"] + LEVELS[level], pos, nrm, uv, tris, joint_names, weights, shapes))
            counts = {}
            for name in dict.fromkeys(layers):
                counts[str(name)] = int((layers[tris[:, 0]] == name).sum())
            report["levels"][str(level)] = {"triangles": int(len(tris)), "vertices": int(len(pos)),
                                            "by_layer": counts,
                                            "roots_deepest_mm": round(float(-min(0.0, binding_depth(body, pos).min())) * 1000, 3)}
        material = {"name": "Hair", "alphaMode": "MASK", "alphaCutoff": 0.5, "doubleSided": True,
                    "pbrMetallicRoughness": {"metallicFactor": 0.0, "roughnessFactor": style["roughness"]},
                    "normal_scale": 0.5}
        out_glb = os.path.join(args.out, "hair", style["file"])
        glb.write(out_glb, skeleton, meshes, material, maps)
    follows = [n for n in names if n == "Frame_Feminine" or not is_expression(n)]
    report["shapes"] = sorted(names)
    report["follows"] = sorted(follows)
    report["atlas"] = {"size": atlas.SIZE, "islands": {k: len(v) for k, v in islands.items()}}
    with open(os.path.join(args.out, "hair", style["file"].replace(".glb", ".build.json")), "w", newline="\n") as f:
        json.dump(report, f, indent=2)
        f.write("\n")
    item = manifest(style, follows)
    path = os.path.join(args.out, "items", style["item"].split("/")[-1] + ".item.json")
    with open(path, "w", newline="\n") as f:
        json.dump(item, f, indent=2)
        f.write("\n")
    print(json.dumps(report["levels"], indent=1))


def is_expression(name):
    """Shapes the face's controls or the correctives drive (not identity):
    the hair carries them so it moves with the skin, but they aren't what
    it is fitted to."""
    return not (name.startswith("Face_") or name.startswith("Frame_") or name.startswith("Masculine_")
                or name.startswith("Feminine_"))


def binding_depth(body, positions):
    return body.nearest(positions)[4]


if __name__ == "__main__":
    main()
