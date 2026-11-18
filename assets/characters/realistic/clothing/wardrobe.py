"""Makes the realistic wardrobe's items from their built sources: each
level of detail is fitted to the realistic body by the engine's own
import (`avatar::fit`, `cargo run -p axcenity-avatar --example
import_garment`: binding, the body's shapes, push-out, region hiding),
then the three fitted levels and the baked maps are put together into
the item's model, with its manifest (colour roles, variants, quality,
source). Plain Python (no Blender):

    python -I assets/characters/realistic/clothing/wardrobe.py --built <folder> [--only crew-tee]

`--built` is where the `build_*.py` scripts wrote each item's sources
(`<folder>/<item>/`). The items are written to
`assets/characters/realistic/clothing/<item>/`; then run
`python -I assets/characters/catalogue.py`.

The imports run in a scratch copy of the realistic library (the base,
its underwear and the items already made, so a later item is pushed
clear of an earlier one it is worn over), next to `--built`.
"""

import argparse
import json
import os
import shutil
import struct
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REALISTIC = os.path.dirname(HERE)
LIBRARY = os.path.dirname(REALISTIC)
REPOSITORY = os.path.dirname(os.path.dirname(LIBRARY))
BASE = "axcenity/realistic"
CREDIT = "Axcenity Engine contributors"

# Each item: its sources' name, the object it draws as, its manifest, the
# items it is pushed clear of, its materials (glTF material values) and
# variants. Quality stays "draft" until every check of asset-quality.md
# §2 and §3 passes and the review finds no defect (see README.md).
ITEMS = {
    "joggers": {
        "source": "Joggers",
        "object": "Pants_Joggers",
        "id": "axcenity/realistic-joggers",
        "label": "Joggers",
        "slot": "bottom",
        "under": ["axcenity/realistic-boxer-briefs", "axcenity/realistic-briefs"],
        "about": "Fleece joggers with an elastic waistband, a drawcord and rib cuffs, cut from FreeSewing's Paco.",
        "tags": ["joggers", "sweatpants", "fleece", "casual"],
        "author": (f"{CREDIT}; pattern after FreeSewing's paco design on its titan block (MIT); "
                   "fabric from Poly Haven's Jogging Melange (CC0)"),
        "url": "https://codeberg.org/freesewing/freesewing/src/branch/develop/designs/paco",
        "materials": {
            "Fleece": {"role": "primary", "label": "Joggers", "sheen": [0.35, 0.35, 0.35], "sheen_roughness": 0.5},
            "Rib": {"role": "secondary", "label": "Waistband and cuffs", "sheen": [0.3, 0.3, 0.3],
                    "sheen_roughness": 0.55},
            "Cord": {"role": "accent", "label": "Drawcord", "sheen": [0.2, 0.2, 0.2], "sheen_roughness": 0.6},
            "Aglet": {"label": "Aglets and grommets", "shading": "standard"},
        },
        "variants": [
            ("heather-grey", "Heather grey", {"Fleece": "#ffffff", "Rib": "#ffffff", "Cord": "#ffffff"}),
            ("charcoal", "Charcoal", {"Fleece": "#56585d", "Rib": "#56585d", "Cord": "#56585d"}),
            ("black", "Black", {"Fleece": "#2a2a2c", "Rib": "#2a2a2c", "Cord": "#2a2a2c"}),
            ("navy", "Navy", {"Fleece": "#2c3552", "Rib": "#2c3552", "Cord": "#ffffff"}),
            ("olive", "Olive", {"Fleece": "#5c6146", "Rib": "#5c6146", "Cord": "#5c6146"}),
        ],
    },
    "crew-tee": {
        "source": "CrewTee",
        "object": "Shirt_CrewTee",
        "id": "axcenity/realistic-crew-tee",
        "label": "Crew-neck T-shirt",
        "slot": "top",
        "under": ["axcenity/realistic-sports-bra", "axcenity/realistic-joggers"],
        "about": "A short-sleeved cotton jersey T-shirt with a rib crew neck, cut from FreeSewing's Teagan.",
        "tags": ["tee", "t-shirt", "jersey", "casual"],
        "author": (f"{CREDIT}; pattern after FreeSewing's teagan design (MIT); "
                   "fabric from Poly Haven's Cotton Jersey (CC0)"),
        "url": "https://codeberg.org/freesewing/freesewing/src/branch/develop/designs/teagan",
        "materials": {
            "Jersey": {"role": "primary", "label": "Shirt", "sheen": [0.22, 0.22, 0.22], "sheen_roughness": 0.6},
            "Rib": {"role": "secondary", "label": "Neck band", "sheen": [0.22, 0.22, 0.22], "sheen_roughness": 0.6},
        },
        "variants": [
            ("white", "White", {"Jersey": "#ffffff", "Rib": "#ffffff"}),
            ("heather-grey", "Heather grey", {"Jersey": "#a9a9a6", "Rib": "#a9a9a6"}),
            ("navy", "Navy", {"Jersey": "#2b3550", "Rib": "#2b3550"}),
            ("black", "Black", {"Jersey": "#262626", "Rib": "#262626"}),
            ("olive", "Olive", {"Jersey": "#5d6443", "Rib": "#5d6443"}),
            ("ringer", "White with a navy band", {"Jersey": "#ffffff", "Rib": "#2b3550"}),
        ],
    },
    "sneakers": {
        "source": "Sneakers",
        "object": "Shoes_CourtSneakers",
        "id": "axcenity/realistic-sneakers",
        "label": "Court sneakers",
        "slot": "shoes",
        "under": [],
        "about": "Low-top leather sneakers with a padded collar, laces and a cupsole.",
        "tags": ["sneakers", "trainers", "leather", "casual"],
        "author": (f"{CREDIT}; a shell built over the realistic foot; leather and textile from Poly Haven's "
                   "Leather White and Cotton Jersey (CC0), outsole rubber from ambientCG's Rubber004 (CC0)"),
        "url": "https://ambientcg.com/view?id=Rubber004",
        "materials": {
            "Upper": {"role": "primary", "label": "Leather", "shading": "standard"},
            "Lining": {"role": "secondary", "label": "Lining", "sheen": [0.2, 0.2, 0.2], "sheen_roughness": 0.6},
            "Tongue": {"role": "secondary", "label": "Tongue", "sheen": [0.2, 0.2, 0.2], "sheen_roughness": 0.6},
            "Laces": {"role": "accent", "label": "Laces", "sheen": [0.25, 0.25, 0.25], "sheen_roughness": 0.55},
            "Midsole": {"label": "Midsole", "shading": "standard"},
            "Outsole": {"role": "accent", "label": "Outsole", "shading": "standard"},
        },
        "variants": [
            ("white", "White", {"Upper": "#ffffff", "Lining": "#ffffff", "Tongue": "#ffffff", "Laces": "#ffffff"}),
            ("black", "Black", {"Upper": "#2c2c2c", "Lining": "#2c2c2c", "Tongue": "#2c2c2c", "Laces": "#2c2c2c",
                                "Midsole": "#3a3a3a"}),
            ("navy", "Navy", {"Upper": "#2d3754", "Lining": "#2d3754", "Tongue": "#2d3754", "Laces": "#ffffff"}),
            ("white-gum", "White with a gum sole", {"Upper": "#ffffff", "Outsole": "#ffd2a0"}),
        ],
    },
}


def glb_read(path):
    data = open(path, "rb").read()
    n = struct.unpack("<I", data[12:16])[0]
    doc = json.loads(data[20:20 + n])
    m = struct.unpack("<I", data[20 + n:24 + n])[0]
    return doc, bytearray(data[28 + n:28 + n + m])


def glb_write(path, doc, binary):
    while len(binary) % 4:
        binary.append(0)
    doc["buffers"] = [{"byteLength": len(binary)}]
    text = json.dumps(doc, separators=(",", ":")).encode()
    while len(text) % 4:
        text += b" "
    total = 12 + 8 + len(text) + 8 + len(binary)
    with open(path, "wb") as f:
        f.write(b"glTF" + struct.pack("<II", 2, total))
        f.write(struct.pack("<I", len(text)) + b"JSON" + text)
        f.write(struct.pack("<I", len(binary)) + b"BIN\0" + bytes(binary))


def append(doc, binary, other, other_bin):
    """Adds `other`'s meshes (with their accessors and buffer views) to
    `doc`; returns the new mesh indices."""
    while len(binary) % 4:
        binary.append(0)
    view_base = len(doc["bufferViews"])
    acc_base = len(doc["accessors"])
    offset = len(binary)
    binary.extend(other_bin)
    for view in other["bufferViews"]:
        v = dict(view)
        v["byteOffset"] = v.get("byteOffset", 0) + offset
        doc["bufferViews"].append(v)
    for acc in other["accessors"]:
        a = json.loads(json.dumps(acc))
        if "bufferView" in a:
            a["bufferView"] += view_base
        if "sparse" in a:
            a["sparse"]["indices"]["bufferView"] += view_base
            a["sparse"]["values"]["bufferView"] += view_base
        doc["accessors"].append(a)
    meshes = []
    for mesh in other["meshes"]:
        m = json.loads(json.dumps(mesh))
        for prim in m["primitives"]:
            prim["attributes"] = {k: v + acc_base for k, v in prim["attributes"].items()}
            prim["indices"] += acc_base
            prim["material"] = other["materials"][prim["material"]]["name"]
            for target in prim.get("targets", []):
                for k in target:
                    target[k] += acc_base
        doc["meshes"].append(m)
        meshes.append(len(doc["meshes"]) - 1)
    return meshes


def image(doc, binary, path, mime="image/jpeg"):
    while len(binary) % 4:
        binary.append(0)
    data = open(path, "rb").read()
    doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(data)})
    binary.extend(data)
    doc.setdefault("images", []).append({"bufferView": len(doc["bufferViews"]) - 1, "mimeType": mime})
    doc.setdefault("samplers", [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}])
    doc.setdefault("textures", []).append({"sampler": 0, "source": len(doc["images"]) - 1})
    return len(doc["textures"]) - 1


def assemble(item, built, fitted, out):
    """The item's model: LOD0's fitted file with LOD1 and LOD2 added as
    `<object>_LOD1`, `_LOD2` nodes skinned to the same skeleton, and the
    baked maps on cloth materials (`docs/rendering/cloth.md`)."""
    doc, binary = glb_read(fitted[0])
    source = item["source"]
    # The garment's node, renamed.
    node0 = next(k for k, n in enumerate(doc["nodes"]) if "mesh" in n)
    doc["nodes"][node0]["name"] = item["object"]
    doc["meshes"][0]["name"] = item["object"]
    for prim in doc["meshes"][0]["primitives"]:
        prim["material"] = doc["materials"][prim["material"]]["name"]
    armature = next(k for k, n in enumerate(doc["nodes"]) if node0 in n.get("children", []))
    for level, path in enumerate(fitted[1:], start=1):
        other, other_bin = glb_read(path)
        (mesh,) = append(doc, binary, other, other_bin)
        doc["meshes"][mesh]["name"] = f"{item['object']}_LOD{level}"
        doc["nodes"].append({"name": f"{item['object']}_LOD{level}", "mesh": mesh, "skin": 0})
        doc["nodes"][armature]["children"].append(len(doc["nodes"]) - 1)
    colour = image(doc, binary, os.path.join(built, f"{source}_color.jpg"))
    normal = image(doc, binary, os.path.join(built, f"{source}_normal.jpg"))
    orm = image(doc, binary, os.path.join(built, f"{source}_orm.jpg"))
    names = list(item["materials"])
    doc["materials"] = []
    for name in names:
        m = item["materials"][name]
        mat = {
            "name": name,
            # The inside of the garment is the same cloth, seen up a sleeve
            # or under a hem.
            "doubleSided": True,
            "pbrMetallicRoughness": {
                "baseColorTexture": {"index": colour},
                "metallicRoughnessTexture": {"index": orm},
                "metallicFactor": 1.0,
                "roughnessFactor": 1.0,
            },
            "normalTexture": {"index": normal},
            "occlusionTexture": {"index": orm},
        }
        if m.get("shading", "cloth") != "standard":
            mat["extras"] = {"axcenity_shading": m.get("shading", "cloth")}
        if "sheen" in m:
            mat["extensions"] = {"KHR_materials_sheen": {
                "sheenColorFactor": m["sheen"], "sheenRoughnessFactor": m["sheen_roughness"]}}
        doc["materials"].append(mat)
    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            prim["material"] = names.index(prim["material"])
    if any("sheen" in m for m in item["materials"].values()):
        doc["extensionsUsed"] = ["KHR_materials_sheen"]
    doc["asset"]["generator"] = "Axcenity Engine realistic wardrobe (assets/characters/realistic/clothing)"
    glb_write(out, doc, binary)


def library(scratch):
    """A scratch copy of the realistic library with the items already
    made, and a catalogue."""
    if os.path.isdir(scratch):
        shutil.rmtree(scratch)
    os.makedirs(os.path.join(scratch, "realistic", "items"))
    for name in ("Realistic.base.json", "RealisticParts.glb", "RealisticRegions.png"):
        shutil.copy(os.path.join(REALISTIC, name), os.path.join(scratch, "realistic", name))
    for name in os.listdir(os.path.join(REALISTIC, "items")):
        if name.endswith(".item.json"):
            shutil.copy(os.path.join(REALISTIC, "items", name), os.path.join(scratch, "realistic", "items", name))
    for key in ITEMS:
        made = os.path.join(HERE, key)
        if os.path.isdir(made):
            shutil.copytree(made, os.path.join(scratch, "realistic", "clothing", key))
    shutil.copy(os.path.join(LIBRARY, "catalogue.py"), scratch)
    subprocess.run([sys.executable, "-I", os.path.join(scratch, "catalogue.py")], check=True)


def run_import(item, obj, scratch, label, folder, under):
    cmd = ["cargo", "run", "-q", "-p", "axcenity-avatar", "--release", "--example", "import_garment", "--",
           obj, scratch, BASE, f"build/{folder}", label, f"build/{folder}",
           "--slot", item["slot"], "--layer", "1", "--author", CREDIT, "--licence", "CC0-1.0"]
    for u in under:
        cmd += ["--under", u]
    out = subprocess.run(cmd, cwd=REPOSITORY, capture_output=True, text=True)
    if out.returncode:
        raise SystemExit(out.stdout + out.stderr)
    return out.stdout


def make(key, built_root, scratch):
    item = ITEMS[key]
    built = os.path.join(built_root, key)
    library(scratch)
    have = set()
    with open(os.path.join(scratch, "catalogue.json")) as f:
        have = {e["id"] for e in json.load(f)["items"]}
    under = [u for u in item["under"] if u in have]
    reports = []
    fitted = []
    for level in range(3):
        folder = f"{key}-lod{level}"
        obj = os.path.join(built, f"{item['source']}_LOD{level}.obj")
        # The import names the object from the label: `CrewTee lod0` is
        # `CrewTeeLod0`.
        report = run_import(item, obj, scratch, f"{item['source']} lod{level}", folder, under)
        reports.append(report)
        fitted.append(os.path.join(scratch, "build", folder, f"{item['source']}Lod{level}.glb"))
    out_dir = os.path.join(HERE, key)
    os.makedirs(out_dir, exist_ok=True)
    obj = item["object"]
    assemble(item, built, fitted, os.path.join(out_dir, f"{obj}.glb"))
    lod0 = os.path.join(scratch, "build", f"{key}-lod0")
    src = f"{item['source']}Lod0"
    shutil.copy(os.path.join(lod0, f"{src}.binding.json"), os.path.join(out_dir, f"{obj}.binding.json"))
    shutil.copy(os.path.join(lod0, f"{src}.png"), os.path.join(out_dir, f"{obj}.png"))
    with open(os.path.join(lod0, f"{src}.item.json")) as f:
        imported = json.load(f)
    manifest = {
        "format_version": 1,
        "id": item["id"],
        "label": item["label"],
        "kind": "garment",
        "slot": item["slot"],
        "layer": 1,
        "tier": "built_in",
        "bases": [BASE],
        "model": {"file": f"{obj}.glb", "object": f"AxcenityRealistic/{obj}"},
        "hides": imported.get("hides", []),
        "materials": [dict({"name": n, "shading": m.get("shading", "cloth"), "label": m["label"]},
                           **({"role": m["role"]} if "role" in m else {}))
                      for n, m in item["materials"].items()],
        "variants": [{"id": vid, "label": label,
                      "materials": {n: {"tint": tint} for n, tint in tints.items()}}
                     for vid, label, tints in item["variants"]],
        "fit": {"method": "binding", "data": f"{obj}.binding.json", "margin": 0.002},
        "tags": item["tags"],
        "thumbnail": f"{obj}.png",
        "source": {"author": item["author"], "licence": "CC0-1.0", "url": item["url"]},
        "about": item["about"],
        "quality": item.get("quality", "draft"),
    }
    if imported.get("also_fills"):
        manifest["also_fills"] = imported["also_fills"]
    with open(os.path.join(out_dir, f"{obj}.item.json"), "w", newline="\n") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")
    with open(os.path.join(built, "import.txt"), "w", newline="\n") as f:
        f.write("\n\n".join(reports))
    return reports


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--built", required=True)
    parser.add_argument("--only", action="append")
    args = parser.parse_args()
    scratch = os.path.join(args.built, "library")
    for key in ITEMS:
        if args.only and key not in args.only:
            continue
        reports = make(key, args.built, scratch)
        for r in reports:
            print("\n".join(line for line in r.splitlines() if line.startswith(("imported", "slot", "alignment", "coverage"))))


if __name__ == "__main__":
    main()
