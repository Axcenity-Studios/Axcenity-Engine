"""Writes a hairstyle's glTF binary (Character Studio R6, CO11): its card
meshes (full, `_LOD1`, `_LOD2`), skinned to a copy of the realistic
body's skeleton (the same joints, rest and inverse bind matrices as
`RealisticParts.glb`, so the engine poses it by bone name with the body),
with the body's shapes it carries as sparse morph targets named as the
body's, and its Hair material and atlas.

Written directly rather than through Blender's exporter: the material's
`extras` name the data and flow maps, which the exporter would leave out
(`skin.mark` patches the body's file for the same reason), and the
shapes are only the moves each mesh has.

Positions arrive in Blender's space (Z up, -Y forward) and are written in
glTF's (Y up, +Z forward).
"""

import json
import struct

import numpy as np

FLOAT, U16, U32 = 5126, 5123, 5125
ARRAY, ELEMENTS = 34962, 34963


def read_glb(path):
    with open(path, "rb") as f:
        data = f.read()
    at, chunks = 12, []
    while at < len(data):
        length, kind = struct.unpack_from("<II", data, at)
        chunks.append(data[at + 8:at + 8 + length])
        at += 8 + length
    return json.loads(chunks[0]), chunks[1] if len(chunks) > 1 else b""


def to_gltf(v):
    v = np.asarray(v, dtype=np.float64)
    return np.stack([v[..., 0], v[..., 2], -v[..., 1]], axis=-1)


class Skeleton:
    """The body's skeleton as its file has it: the joint nodes (the
    subtree under the armature's root bone), the skin's joint order and
    its inverse bind matrices."""

    def __init__(self, path, armature="AxcenityRealistic"):
        doc, binary = read_glb(path)
        nodes = doc["nodes"]
        skin = doc["skins"][0]
        root = next(i for i, n in enumerate(nodes) if n.get("name") == armature)
        joints = set(skin["joints"])
        top = [c for c in nodes[root].get("children", []) if c in joints]
        assert len(top) == 1, top
        order = []

        def walk(i):
            order.append(i)
            for c in nodes[i].get("children", []):
                walk(c)

        walk(top[0])
        self.old_to_new = {old: new for new, old in enumerate(order)}
        self.nodes = []
        for old in order:
            node = {k: v for k, v in nodes[old].items() if k in ("name", "translation", "rotation", "scale")}
            kids = [self.old_to_new[c] for c in nodes[old].get("children", [])]
            if kids:
                node["children"] = kids
            self.nodes.append(node)
        self.armature = armature
        self.root_node = {k: v for k, v in nodes[root].items() if k in ("translation", "rotation", "scale")}
        self.joints = [self.old_to_new[j] for j in skin["joints"]]
        self.joint_names = [nodes[j]["name"] for j in skin["joints"]]
        acc = doc["accessors"][skin["inverseBindMatrices"]]
        view = doc["bufferViews"][acc["bufferView"]]
        start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
        self.inverse_binds = bytes(binary[start:start + acc["count"] * 64])
        self.joint_count = acc["count"]


class Writer:
    def __init__(self):
        self.doc = {
            "asset": {"version": "2.0", "generator": "Axcenity hair build (assets/characters/realistic/hair)"},
            "scene": 0, "scenes": [{"name": "Scene", "nodes": []}], "nodes": [], "meshes": [],
            "materials": [], "textures": [], "images": [], "samplers": [], "accessors": [],
            "bufferViews": [], "buffers": [{"byteLength": 0}], "skins": [],
        }
        self.bin = bytearray()

    def view(self, data, target=None):
        self.bin.extend(b"\0" * (-len(self.bin) % 4))
        view = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target:
            view["target"] = target
        self.bin.extend(data)
        self.doc["bufferViews"].append(view)
        return len(self.doc["bufferViews"]) - 1

    def accessor(self, array, kind, component, target=None, bounds=False):
        array = np.ascontiguousarray(array)
        acc = {"bufferView": self.view(array.tobytes(), target), "componentType": component,
               "count": int(array.shape[0]), "type": kind}
        if bounds:
            acc["min"] = [float(x) for x in array.min(0)]
            acc["max"] = [float(x) for x in array.max(0)]
        self.doc["accessors"].append(acc)
        return len(self.doc["accessors"]) - 1

    def sparse(self, count, indices, values):
        """A VEC3 float accessor of `count` zeros but at `indices`."""
        if len(indices) == 0:
            indices, values = np.array([0]), np.zeros((1, 3))
        full = np.zeros((count, 3))
        full[indices] = values
        acc = {"componentType": FLOAT, "count": int(count), "type": "VEC3",
               "min": [float(x) for x in full.min(0)], "max": [float(x) for x in full.max(0)],
               "sparse": {"count": int(len(indices)),
                          "indices": {"bufferView": self.view(np.asarray(indices, dtype=np.uint32).tobytes()),
                                      "componentType": U32},
                          "values": {"bufferView": self.view(np.asarray(values, dtype=np.float32).tobytes())}}}
        self.doc["accessors"].append(acc)
        return len(self.doc["accessors"]) - 1

    def texture(self, png_path, name):
        with open(png_path, "rb") as f:
            png = f.read()
        self.doc["images"].append({"bufferView": self.view(png), "mimeType": "image/png", "name": name})
        if not self.doc["samplers"]:
            self.doc["samplers"].append({"magFilter": 9729, "minFilter": 9987})
        self.doc["textures"].append({"sampler": 0, "source": len(self.doc["images"]) - 1})
        return len(self.doc["textures"]) - 1

    def save(self, path):
        self.bin.extend(b"\0" * (-len(self.bin) % 4))
        self.doc["buffers"][0]["byteLength"] = len(self.bin)
        text = json.dumps(self.doc, separators=(",", ":")).encode()
        text += b" " * (-len(text) % 4)
        body = struct.pack("<II", len(text), 0x4E4F534A) + text + struct.pack("<II", len(self.bin), 0x004E4942) + bytes(self.bin)
        with open(path, "wb") as f:
            f.write(struct.pack("<III", 0x46546C67, 2, 12 + len(body)) + body)


def write(path, skeleton, meshes, material, maps):
    """`meshes`: [(name, positions, normals, uvs, tris, joints (names per
    vertex, 4), weights (4), shapes {name: offsets})] in Blender's space;
    `material`: the material's glTF fields without textures; `maps`: PNG
    paths of `colour`, `data`, `flow`, `normal`."""
    w = Writer()
    w.doc["nodes"] = [dict(n) for n in skeleton.nodes]
    hips = 0
    tex = {k: w.texture(p, f"{material['name']}_{k.title()}") for k, p in maps.items()}
    mat = json.loads(json.dumps(material))
    mat.setdefault("pbrMetallicRoughness", {})["baseColorTexture"] = {"index": tex["colour"]}
    mat["normalTexture"] = {"index": tex["normal"], "scale": material.get("normal_scale", 0.6)}
    mat.pop("normal_scale", None)
    extras = dict(mat.get("extras", {}))
    extras["axcenity_shading"] = "hair"
    extras["axcenity_hair_data"] = {"index": tex["data"]}
    extras["axcenity_hair_flow"] = {"index": tex["flow"]}
    mat["extras"] = extras
    w.doc["materials"].append(mat)
    index_of = {name: i for i, name in enumerate(skeleton.joint_names)}
    mesh_nodes = []
    for name, pos, nrm, uv, tris, joint_names, weights, shapes in meshes:
        p = to_gltf(pos).astype(np.float32)
        n = to_gltf(nrm).astype(np.float32)
        joints = np.array([[index_of.get(b, 0) for b in row] for row in joint_names], dtype=np.uint16)
        attrs = {
            "POSITION": w.accessor(p, "VEC3", FLOAT, ARRAY, bounds=True),
            "NORMAL": w.accessor(n, "VEC3", FLOAT, ARRAY),
            "TEXCOORD_0": w.accessor(np.asarray(uv, dtype=np.float32), "VEC2", FLOAT, ARRAY),
            "JOINTS_0": w.accessor(joints, "VEC4", U16, ARRAY),
            "WEIGHTS_0": w.accessor(np.asarray(weights, dtype=np.float32), "VEC4", FLOAT, ARRAY),
        }
        targets, target_names = [], []
        for shape, delta in sorted(shapes.items()):
            d = to_gltf(delta)
            moved = np.where(np.linalg.norm(d, axis=1) > 1e-5)[0]
            targets.append({"POSITION": w.sparse(len(p), moved, d[moved])})
            target_names.append(shape)
        prim = {"attributes": attrs, "indices": w.accessor(np.asarray(tris, dtype=np.uint32).reshape(-1),
                                                            "SCALAR", U32, ELEMENTS),
                "material": 0, "mode": 4}
        mesh = {"name": name, "primitives": [prim]}
        if targets:
            prim["targets"] = targets
            mesh["extras"] = {"targetNames": target_names}
            mesh["weights"] = [0.0] * len(targets)
        w.doc["meshes"].append(mesh)
        w.doc["nodes"].append({"name": name, "mesh": len(w.doc["meshes"]) - 1, "skin": 0})
        mesh_nodes.append(len(w.doc["nodes"]) - 1)
    root = dict(skeleton.root_node, name=skeleton.armature, children=[hips] + mesh_nodes)
    w.doc["nodes"].append(root)
    w.doc["scenes"][0]["nodes"] = [len(w.doc["nodes"]) - 1]
    ibm = w.view(skeleton.inverse_binds)
    w.doc["accessors"].append({"bufferView": ibm, "componentType": FLOAT, "count": skeleton.joint_count,
                               "type": "MAT4"})
    w.doc["skins"].append({"name": skeleton.armature, "joints": skeleton.joints,
                           "inverseBindMatrices": len(w.doc["accessors"]) - 1})
    w.save(path)
