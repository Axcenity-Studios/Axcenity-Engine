"""Fits the low-top sneakers (`build_sneakers_low.py`'s sources) to the
realistic body and writes the item. Plain Python with NumPy (no Blender):

    python -I assets/characters/realistic/clothing/fit_sneakers_low.py --built <folder>
    python -I assets/characters/catalogue.py

Each level goes through the engine's own import (`cargo run -p
axcenity-avatar --release --example import_garment -- ... --placed`, in a
scratch copy of the realistic library as the wardrobe does): it gives the
model its skeleton (the realistic rig's joints and bind matrices), the
binding kept for refitting, the regions it may hide (none: the `feet`
region reaches above the collar) and a thumbnail. Then the shoe is made a
shoe again (a firm shell, not cloth, references/clothing §5.1): its
corners go back where the build put them (the import's push-out is for
cloth, and pushing a sole's footbed out of the foot is what crumpled the
wave-4 sneakers), its weights are the build's (the foot's bones only, the
sole and the toe box rigid) and its shapes the build's smooth maps of the
foot's own (so its tread stays flat). The fitted file's other data is
dropped: the model is written afresh with only what it uses.

Writes `assets/characters/realistic/clothing/sneakers-low/`:
`Shoes_LowSneakers.glb` (`Shoes_LowSneakers`, `_LOD1`, `_LOD2`), its
binding, thumbnail and manifest (`quality: "draft"`, original work, CC0).
"""

import argparse
import json
import os
import shutil
import subprocess
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import wardrobe  # noqa: E402

NAME = "SneakersLow"
OBJECT = "Shoes_LowSneakers"
FOLDER = "sneakers-low"
ID = "axcenity/realistic-sneakers-low"
MATERIALS = {
    # name: (shading, label, role, sheen)
    "Upper": ("standard", "Leather", "primary", None),
    "Lining": ("cloth", "Lining and footbed", "secondary", (0.2, 0.6)),
    "Tongue": ("cloth", "Tongue", "secondary", (0.2, 0.6)),
    "Laces": ("cloth", "Laces", "accent", (0.25, 0.55)),
    "Eyelets": ("standard", "Eyelets", None, None),
    "Midsole": ("standard", "Sole", None, None),
    "Outsole": ("standard", "Tread", None, None),
}
VARIANTS = [
    ("white", "White", {"Upper": "#ffffff", "Lining": "#ffffff", "Tongue": "#ffffff", "Laces": "#ffffff"}),
    ("black", "Black", {"Upper": "#2b2b2b", "Lining": "#2b2b2b", "Tongue": "#2b2b2b", "Laces": "#2b2b2b",
                        "Midsole": "#e9e6df", "Outsole": "#e9e6df"}),
    ("navy", "Navy", {"Upper": "#2d3754", "Lining": "#2d3754", "Tongue": "#2d3754", "Laces": "#ffffff"}),
    ("tan", "Tan suede-look", {"Upper": "#b08a62", "Tongue": "#b08a62", "Laces": "#e9e2d4",
                               "Outsole": "#c79a62"}),
]


def accessor_data(doc, binary, index):
    acc = doc["accessors"][index]
    view = doc["bufferViews"][acc["bufferView"]]
    comps = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}[acc["type"]]
    dtype = {5126: np.float32, 5123: np.uint16, 5125: np.uint32, 5121: np.uint8}[acc["componentType"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    count = acc["count"] * comps
    stride = view.get("byteStride")
    item = np.dtype(dtype).itemsize * comps
    if stride and stride != item:
        raw = np.frombuffer(bytes(binary[start:start + stride * acc["count"]]), dtype=np.uint8)
        raw = raw.reshape(acc["count"], stride)[:, :item].copy()
        return np.frombuffer(raw.tobytes(), dtype=dtype).reshape(acc["count"], comps)
    return np.frombuffer(bytes(binary[start:start + count * np.dtype(dtype).itemsize]),
                         dtype=dtype).reshape(acc["count"], comps)


class Writer:
    """A glTF binary built from scratch: accessors from arrays."""

    def __init__(self):
        self.doc = {"asset": {"version": "2.0", "generator":
                              "Axcenity Engine low-top sneakers (assets/characters/realistic/clothing)"},
                    "bufferViews": [], "accessors": [], "meshes": [], "nodes": [], "materials": []}
        self.bin = bytearray()

    def view(self, data, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        v = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target:
            v["target"] = target
        self.bin.extend(data)
        self.doc["bufferViews"].append(v)
        return len(self.doc["bufferViews"]) - 1

    def accessor(self, array, kind, component, target=None, bounds=False):
        dtype = {5126: np.float32, 5123: np.uint16, 5125: np.uint32}[component]
        array = np.ascontiguousarray(array, dtype=dtype)
        acc = {"bufferView": self.view(array.tobytes(), target), "componentType": component,
               "count": int(len(array)), "type": kind}
        if bounds:
            acc["min"] = array.min(axis=0).astype(float).tolist()
            acc["max"] = array.max(axis=0).astype(float).tolist()
        self.doc["accessors"].append(acc)
        return len(self.doc["accessors"]) - 1

    def image(self, path):
        data = open(path, "rb").read()
        self.doc.setdefault("images", []).append({"bufferView": self.view(data), "mimeType": "image/jpeg"})
        self.doc.setdefault("samplers", [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}])
        self.doc.setdefault("textures", []).append({"sampler": 0, "source": len(self.doc["images"]) - 1})
        return len(self.doc["textures"]) - 1

    def write(self, path):
        wardrobe.glb_write(path, self.doc, self.bin)


def run_import(obj, scratch, level):
    folder = f"build/sneakers-low-lod{level}"
    cmd = ["cargo", "run", "-q", "-p", "axcenity-avatar", "--release", "--example", "import_garment", "--",
           obj, scratch, "axcenity/realistic", folder, f"{NAME} lod{level}", folder,
           "--slot", "shoes", "--layer", "1", "--margin", "0.002", "--author", wardrobe.CREDIT,
           "--licence", "CC0-1.0", "--placed"]
    out = subprocess.run(cmd, cwd=wardrobe.REPOSITORY, capture_output=True, text=True)
    if out.returncode:
        raise SystemExit(out.stdout + out.stderr)
    return out.stdout, os.path.join(scratch, folder, f"{NAME}Lod{level}")


def matched(fitted_pos, fitted_uv, side):
    """For each of the fitted model's corners, the build's corner it is
    (the same texture coordinate, the nearest place)."""
    uv = np.column_stack([side["uvs"][:, 0], 1 - side["uvs"][:, 1]])  # OBJ's v up to glTF's down
    key = {}
    for k, t in enumerate(np.round(uv, 4)):
        key.setdefault((t[0], t[1]), []).append(k)
    out = np.zeros(len(fitted_pos), dtype=int)
    far = 0.0
    for i, (p, t) in enumerate(zip(fitted_pos, np.round(fitted_uv, 4))):
        cands = key.get((t[0], t[1]))
        if not cands:
            d = np.linalg.norm(uv - fitted_uv[i], axis=1) + np.linalg.norm(side["positions"] - p, axis=1)
            cands = [int(np.argmin(d))]
        best = min(cands, key=lambda k: np.linalg.norm(side["positions"][k] - p))
        out[i] = best
        far = max(far, float(np.linalg.norm(side["positions"][best] - p)))
    return out, far


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--built", required=True)
    args = parser.parse_args()
    built = os.path.abspath(args.built)
    scratch = os.path.join(built, "library")
    wardrobe.library(scratch)
    w = Writer()
    report = {"levels": []}
    skeleton = None
    for level in range(3):
        stdout, stem = run_import(os.path.join(built, f"{NAME}_LOD{level}.obj"), scratch, level)
        doc, binary = wardrobe.glb_read(stem + ".glb")
        side = dict(np.load(os.path.join(built, f"{NAME}_LOD{level}.npz")))
        if skeleton is None:
            # The skeleton: every node but the fitted mesh's, as it is.
            mesh_node = next(k for k, n in enumerate(doc["nodes"]) if "mesh" in n)
            armature = next(k for k, n in enumerate(doc["nodes"]) if mesh_node in n.get("children", []))
            remap = {}
            for k, n in enumerate(doc["nodes"]):
                if k == mesh_node:
                    continue
                remap[k] = len(w.doc["nodes"])
                w.doc["nodes"].append({key: v for key, v in n.items() if key not in ("mesh", "skin", "children")})
            for k, n in enumerate(doc["nodes"]):
                if k in remap and n.get("children"):
                    kids = [remap[c] for c in n["children"] if c in remap]
                    if kids:
                        w.doc["nodes"][remap[k]]["children"] = kids
            skin = doc["skins"][0]
            ibm = accessor_data(doc, binary, skin["inverseBindMatrices"])
            joint_names = [doc["nodes"][j]["name"] for j in skin["joints"]]
            w.doc["skins"] = [{"joints": [remap[j] for j in skin["joints"]],
                               "inverseBindMatrices": w.accessor(ibm, "MAT4", 5126)}]
            if "skeleton" in skin:
                w.doc["skins"][0]["skeleton"] = remap[skin["skeleton"]]
            w.doc["scenes"] = [{"nodes": [remap[n] for n in doc["scenes"][0]["nodes"] if n in remap]}]
            w.doc["scene"] = 0
            skeleton = (remap[armature], joint_names)
            hides = json.load(open(stem + ".item.json")).get("hides", [])
            shutil.copy(stem + ".binding.json", os.path.join(built, f"{OBJECT}.binding.json"))
            shutil.copy(stem + ".png", os.path.join(built, f"{OBJECT}.png"))
        armature, joint_names = skeleton
        bone_index = np.array([joint_names.index(str(b)) for b in side["bones"]])
        names = [str(n) for n in side["shape_names"]]
        prims = []
        moved = 0.0
        for prim in doc["meshes"][0]["primitives"]:
            a = prim["attributes"]
            pos = accessor_data(doc, binary, a["POSITION"])
            uv = accessor_data(doc, binary, a["TEXCOORD_0"])
            idx = accessor_data(doc, binary, prim["indices"]).reshape(-1)
            which, far = matched(pos, uv, side)
            moved = max(moved, far)
            joints = bone_index[side["joints"][which]]
            weights = side["weights"][which]
            joints = np.where(weights > 0, joints, 0)
            attrs = {
                "POSITION": w.accessor(side["positions"][which], "VEC3", 5126, 34962, bounds=True),
                "NORMAL": w.accessor(side["normals"][which], "VEC3", 5126, 34962),
                "TEXCOORD_0": w.accessor(uv, "VEC2", 5126, 34962),
                "JOINTS_0": w.accessor(joints, "VEC4", 5123, 34962),
                "WEIGHTS_0": w.accessor(weights, "VEC4", 5126, 34962),
            }
            targets = [{"POSITION": w.accessor(side["shape_deltas"][k][which], "VEC3", 5126, 34962, bounds=True)}
                       for k in range(len(names))]
            material = doc["materials"][prim["material"]]["name"]
            prims.append({"attributes": attrs, "indices": w.accessor(idx, "SCALAR", 5125, 34963),
                          "material": list(MATERIALS).index(material), "targets": targets})
        node_name = OBJECT if level == 0 else f"{OBJECT}_LOD{level}"
        w.doc["meshes"].append({"name": node_name, "primitives": prims, "weights": [0.0] * len(names),
                                "extras": {"targetNames": names}})
        w.doc["nodes"].append({"name": node_name, "mesh": len(w.doc["meshes"]) - 1, "skin": 0})
        w.doc["nodes"][armature].setdefault("children", []).append(len(w.doc["nodes"]) - 1)
        report["levels"].append({"import": [line for line in stdout.splitlines()
                                            if line.startswith(("slot", "coverage"))],
                                 "push_out_undone_m": round(moved, 4), "shapes": len(names)})
    colour = w.image(os.path.join(built, f"{NAME}_color.jpg"))
    normal = w.image(os.path.join(built, f"{NAME}_normal.jpg"))
    orm = w.image(os.path.join(built, f"{NAME}_orm.jpg"))
    for name, (shading, _, _, sheen) in MATERIALS.items():
        mat = {"name": name, "doubleSided": False,
               "pbrMetallicRoughness": {"baseColorTexture": {"index": colour},
                                        "metallicRoughnessTexture": {"index": orm},
                                        "metallicFactor": 1.0, "roughnessFactor": 1.0},
               "normalTexture": {"index": normal}, "occlusionTexture": {"index": orm}}
        if shading != "standard":
            mat["extras"] = {"axcenity_shading": shading}
        if sheen:
            mat["extensions"] = {"KHR_materials_sheen": {"sheenColorFactor": [sheen[0]] * 3,
                                                         "sheenRoughnessFactor": sheen[1]}}
        w.doc["materials"].append(mat)
    w.doc["extensionsUsed"] = ["KHR_materials_sheen"]
    out = os.path.join(wardrobe.HERE, FOLDER)
    os.makedirs(out, exist_ok=True)
    w.write(os.path.join(out, f"{OBJECT}.glb"))
    for ext in (".binding.json", ".png"):
        shutil.copy(os.path.join(built, OBJECT + ext), os.path.join(out, OBJECT + ext))
    manifest = {
        "format_version": 1,
        "id": ID,
        "label": "Low-top sneakers",
        "kind": "garment",
        "slot": "shoes",
        "layer": 1,
        "tier": "built_in",
        "bases": ["axcenity/realistic"],
        "model": {"file": f"{OBJECT}.glb", "object": f"AxcenityRealistic/{OBJECT}"},
        "hides": hides,
        "materials": [dict({"name": n, "shading": s, "label": label}, **({"role": role} if role else {}))
                      for n, (s, label, role, _) in MATERIALS.items()],
        "variants": [{"id": vid, "label": label, "materials": {n: {"tint": t} for n, t in tints.items()}}
                     for vid, label, tints in VARIANTS],
        "fit": {"method": "binding", "data": f"{OBJECT}.binding.json", "margin": 0.002},
        "tags": ["sneakers", "trainers", "low-top", "court", "leather", "casual"],
        "thumbnail": f"{OBJECT}.png",
        "source": {"author": (f"{wardrobe.CREDIT}; modelled from scratch over a last made from the realistic foot, "
                              "against openly licensed reference photographs (looked at, not shipped); leather and "
                              "textile from Poly Haven's Leather White and Cotton Jersey (CC0), sole rubber from "
                              "ambientCG's Rubber004 (CC0)"),
                   "licence": "CC0-1.0", "url": "https://polyhaven.com/a/leather_white", "origin": "original"},
        "about": "Plain low-top court sneakers: a leather upper with a toe cap, eyestays and heel counter, "
                 "a padded collar and tongue, laces, and a stitched cupsole.",
        "quality": "draft",
    }
    with open(os.path.join(out, f"{OBJECT}.item.json"), "w", newline="\n") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")
    with open(os.path.join(built, "fit.json"), "w", newline="\n") as f:
        json.dump(report, f, indent=1)
    print(json.dumps(report, indent=1))


if __name__ == "__main__":
    main()
