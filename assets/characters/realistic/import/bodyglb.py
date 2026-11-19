"""The realistic body read straight from `RealisticParts.glb` (NumPy, no
Blender): its skin at rest in glTF's axes (metres, Y up, facing +Z), its
triangles, four joints and weights per vertex, and the skeleton's joints
(each bone's head at rest, from the skin's inverse bind matrices) and
parents. `frame="feminine"` puts `Frame_Feminine` on at full weight.
"""

import json
import os
import struct

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT = os.path.join(os.path.dirname(HERE), "RealisticParts.glb")
BODY = "Body"

_COMPONENTS = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32,
               5126: np.float32}
_SIZES = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
_cache = {}


def read_glb(path):
    with open(path, "rb") as f:
        data = f.read()
    n = struct.unpack("<I", data[12:16])[0]
    doc = json.loads(data[20:20 + n])
    m = struct.unpack("<I", data[20 + n:24 + n])[0]
    return doc, data[28 + n:28 + n + m]


def accessor(doc, binary, index):
    """An accessor's values as an array (rows of its type's width);
    normalized integers are scaled to 0..1; sparse accessors are applied."""
    acc = doc["accessors"][index]
    dtype = np.dtype(_COMPONENTS[acc["componentType"]])
    width = _SIZES[acc["type"]]
    count = acc["count"]
    if "bufferView" in acc:
        view = doc["bufferViews"][acc["bufferView"]]
        start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
        stride = view.get("byteStride", width * dtype.itemsize)
        if stride == width * dtype.itemsize:
            out = np.frombuffer(binary, dtype, count * width, start).reshape(count, width)
        else:
            rows = [np.frombuffer(binary, dtype, width, start + k * stride) for k in range(count)]
            out = np.array(rows)
        out = out.astype(np.float64 if dtype.kind == "f" or acc.get("normalized") else np.int64)
    else:
        out = np.zeros((count, width))
    if acc.get("normalized") and dtype.kind in "ui":
        out = out / np.iinfo(dtype).max
    sparse = acc.get("sparse")
    if sparse:
        out = out.astype(np.float64).copy()
        idx_view = doc["bufferViews"][sparse["indices"]["bufferView"]]
        idx_type = np.dtype(_COMPONENTS[sparse["indices"]["componentType"]])
        idx = np.frombuffer(binary, idx_type, sparse["count"],
                            idx_view.get("byteOffset", 0) + sparse["indices"].get("byteOffset", 0))
        val_view = doc["bufferViews"][sparse["values"]["bufferView"]]
        vals = np.frombuffer(binary, dtype, sparse["count"] * width,
                             val_view.get("byteOffset", 0) + sparse["values"].get("byteOffset", 0))
        out[idx.astype(np.int64)] = vals.reshape(-1, width)
    return out


def load(path=None, frame="masculine"):
    """The body as a `skinning.Body` (cached per file and frame)."""
    import skinning
    path = path or DEFAULT
    key = (os.path.abspath(path), frame)
    if key in _cache:
        return _cache[key]
    doc, binary = read_glb(path)
    nodes = doc["nodes"]
    node = next(n for n in nodes if n.get("name") == BODY and "mesh" in n)
    mesh = doc["meshes"][node["mesh"]]
    skin = doc["skins"][node["skin"]]
    bones = [nodes[j]["name"] for j in skin["joints"]]
    ibm = accessor(doc, binary, skin["inverseBindMatrices"]).reshape(-1, 4, 4)
    # glTF matrices are column-major: transpose to row-major, then the
    # joint's world matrix at rest is the inverse of its bind matrix.
    heads = {}
    for name, m in zip(bones, ibm):
        world = np.linalg.inv(m.T)
        heads[name] = world[:3, 3]
    parent_of = {}
    for i, n in enumerate(nodes):
        for c in n.get("children", []):
            parent_of[c] = i
    parents = {}
    for j in skin["joints"]:
        p = parent_of.get(j)
        parents[nodes[j]["name"]] = nodes[p]["name"] if p is not None and p in skin["joints"] else None
    names = mesh.get("extras", {}).get("targetNames", [])
    Ps, tris, joints, weights = [], [], [], []
    offset = 0
    for prim in mesh["primitives"]:
        a = prim["attributes"]
        P = accessor(doc, binary, a["POSITION"])
        if frame == "feminine" and "Frame_Feminine" in names:
            t = prim["targets"][names.index("Frame_Feminine")]
            P = P + accessor(doc, binary, t["POSITION"])
        Ps.append(P)
        tris.append(accessor(doc, binary, prim["indices"]).reshape(-1, 3) + offset)
        joints.append(accessor(doc, binary, a["JOINTS_0"]).astype(np.int64))
        weights.append(accessor(doc, binary, a["WEIGHTS_0"]))
        offset += len(P)
    body = skinning.Body(np.vstack(Ps), np.vstack(tris), np.vstack(joints), np.vstack(weights),
                         bones, heads, parents)
    _cache[key] = body
    return body


def bone_region(body, bone, least=0.5):
    """The skin weighted mostly to `bone` (its vertices' positions)."""
    j = body.bones.index(bone)
    w = (body.weights * (body.joints == j)).sum(axis=1)
    return body.P[w >= least]
