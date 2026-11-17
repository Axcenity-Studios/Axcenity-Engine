"""Where the realistic body's features are (Character Studio refinement R1):
the anchors of the base manifest's `features`, measured on the built model,
not guessed. Plain Python with numpy: `manifests.py` calls `measure()`.

A feature is found by what moves it or where it is:

- `shapes`: the body vertices its identity or region shapes move, weighted
  by how far each moves (`Face_Nose_*` for the nose, `Masculine_Fat_Chest`
  and `Masculine_Muscle_Chest` for the chest);
- `region`: the body vertices of a region of the region map (`TEXCOORD_1`,
  `u = (index + 0.5) / 16`; README "Regions").

A paired feature splits its vertices by side (the character's left is +X)
and has two anchors, left first. Each anchor is the weighted centre of its
vertices, put on the skin (the nearest of them) when the feature is a patch
of the face's surface, and left at the centre for a part of the body (the
centre of a limb is inside it). It follows the bone that carries most of
those vertices (their skin weights), as an offset from the bone's head in
model space at rest (glTF axes: Y up, +Z front), on the masculine frame,
which is the model's own rest shape. `radius` is the vertices' weighted
root-mean-square distance from the anchor: how far around it counts as the
feature. Everything is rounded to a tenth of a millimetre.
"""

import json
import struct

import numpy as np

COMPONENTS = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


class Glb:
    """The parts of a binary glTF file the anchors need."""

    def __init__(self, path):
        with open(path, "rb") as f:
            data = f.read()
        length = struct.unpack("<I", data[12:16])[0]
        self.gltf = json.loads(data[20:20 + length])
        at = 20 + length
        bin_length = struct.unpack("<I", data[at:at + 4])[0]
        self.bin = data[at + 8:at + 8 + bin_length]

    def rows(self, view_index, offset, count, width, component):
        view = self.gltf["bufferViews"][view_index]
        dtype = np.dtype(COMPONENTS[component])
        start = view.get("byteOffset", 0) + offset
        stride = view.get("byteStride", dtype.itemsize * width)
        return np.ndarray((count, width), dtype=dtype, buffer=self.bin, offset=start,
                          strides=(stride, dtype.itemsize)), dtype

    def accessor(self, index):
        """An accessor's values as floats, one row per element. One with no
        buffer view is zeros; sparse values go over it."""
        acc = self.gltf["accessors"][index]
        width = WIDTH[acc["type"]]
        if "bufferView" in acc:
            rows, dtype = self.rows(acc["bufferView"], acc.get("byteOffset", 0), acc["count"], width,
                                    acc["componentType"])
            out = rows.astype(np.float64)
            if acc.get("normalized"):
                out /= np.iinfo(dtype).max
        else:
            out = np.zeros((acc["count"], width))
        sparse = acc.get("sparse")
        if sparse:
            ix, iv = sparse["indices"], sparse["values"]
            at, _ = self.rows(ix["bufferView"], ix.get("byteOffset", 0), sparse["count"], 1, ix["componentType"])
            values, _ = self.rows(iv["bufferView"], iv.get("byteOffset", 0), sparse["count"], width,
                                  acc["componentType"])
            out[at[:, 0].astype(int)] = values
        return out

    def node(self, name):
        return next(n for n in self.gltf["nodes"] if n["name"] == name)


class Body:
    """The body mesh's rest positions, skin, regions and shapes, every
    primitive together."""

    def __init__(self, path, body="Body"):
        glb = Glb(path)
        node = glb.node(body)
        mesh = glb.gltf["meshes"][node["mesh"]]
        skin = glb.gltf["skins"][node["skin"]]
        self.bones = [glb.gltf["nodes"][j]["name"] for j in skin["joints"]]
        # A joint's head at bind: the translation of its inverse bind
        # matrix's inverse (glTF matrices are column-major).
        ibm = glb.accessor(skin["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
        self.heads = {name: np.linalg.inv(m)[:3, 3] for name, m in zip(self.bones, ibm)}
        names = mesh["extras"]["targetNames"]
        parts = {"P": [], "joints": [], "weights": [], "region": []}
        shapes = {name: [] for name in names}
        for primitive in mesh["primitives"]:
            a = primitive["attributes"]
            parts["P"].append(glb.accessor(a["POSITION"]))
            parts["joints"].append(glb.accessor(a["JOINTS_0"]).astype(int))
            parts["weights"].append(glb.accessor(a["WEIGHTS_0"]))
            parts["region"].append(np.floor(glb.accessor(a["TEXCOORD_1"])[:, 0] * 16).astype(int))
            for name, target in zip(names, primitive["targets"]):
                shapes[name].append(glb.accessor(target["POSITION"]))
        self.P = np.concatenate(parts["P"])
        self.joints = np.concatenate(parts["joints"])
        self.weights = np.concatenate(parts["weights"])
        self.region = np.concatenate(parts["region"])
        self.shapes = {name: np.concatenate(deltas) for name, deltas in shapes.items()}

    def moved(self, names):
        """How far the shapes `names` move each vertex, added."""
        return sum(np.linalg.norm(self.shapes[name], axis=1) for name in names)

    def in_region(self, index):
        return (self.region == index).astype(np.float64)

    def bone_of(self, weight):
        """The bone carrying the most of the vertices weighted `weight`."""
        totals = np.zeros(len(self.bones))
        for k in range(4):
            np.add.at(totals, self.joints[:, k], self.weights[:, k] * weight)
        return self.bones[int(np.argmax(totals))]


def anchor(body, weight, on_skin):
    """An anchor and a radius for the vertices weighted `weight`."""
    w = weight / weight.sum()
    centre = (body.P * w[:, None]).sum(axis=0)
    if on_skin:
        picked = np.where(weight > 0.0)[0]
        centre = body.P[picked[np.argmin(np.linalg.norm(body.P[picked] - centre, axis=1))]]
    radius = float(np.sqrt((w * np.sum((body.P - centre) ** 2, axis=1)).sum()))
    bone = body.bone_of(weight)
    offset = centre - body.heads[bone]
    return {"bone": bone, "offset": [round(float(v), 4) for v in offset]}, radius


def measure(path, definitions, regions):
    """The `features` list: `definitions` are dicts with the feature's
    `id`, `label`, `category`, `about`, optional `icon` and `regions`, and
    how to find it: `shapes` (names; with `{side}` in a name for a pair, as
    `Left`/`Right`), or `region` (a region ID, split by side when `paired`).
    `regions` is the build's region list ([{id, map_index}])."""
    body = Body(path)
    index = {r["id"]: r["map_index"] for r in regions}
    features = []
    for d in definitions:
        sides = ("Left", "Right") if d.get("paired") else (None,)
        anchors, radii = [], []
        for side in sides:
            if "shapes" in d:
                names = [s.format(side=side) for s in d["shapes"]]
                weight = body.moved(names)
                if side and not any("{side}" in s for s in d["shapes"]):
                    weight = weight * ((body.P[:, 0] > 0) == (side == "Left"))
            else:
                weight = body.in_region(index[d["region"]])
                if side:
                    weight = weight * ((body.P[:, 0] > 0) == (side == "Left"))
            weight = np.where(weight > 1e-5, weight, 0.0)
            a, r = anchor(body, weight, d.get("on_skin", False))
            anchors.append(a)
            radii.append(r)
        feature = {"id": d["id"], "label": d["label"], "category": d["category"], "about": d["about"]}
        if d.get("icon"):
            feature["icon"] = d["icon"]
        feature["anchors"] = anchors
        feature["radius"] = round(max(radii), 4)
        if d.get("regions"):
            feature["regions"] = d["regions"]
        features.append(feature)
    return features
