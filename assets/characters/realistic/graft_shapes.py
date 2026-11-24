"""Adds the blend shapes a fresh build has, and the shipped model lacks, to
the shipped `RealisticParts.glb`, leaving everything that is in it as it is.

Why: the shipped model is no longer what `build.py` gives byte for byte.
Rebuilt unchanged on another Blender (5.2.2, 2026-11-24) the body keeps its
vertices and triangles but its positions move by up to 0.07 mm, its
texture coordinates by 3e-4 and its existing shapes by up to 1.5 mm, and
`Body_LOD2` and the teeth come out with other triangles (the README's
"Rebuilding" has the numbers). Garments are bound to the shipped body's
triangles and texture coordinates and carry its shapes by name, so a new
shape is grafted on instead of the package being rebuilt over:

    blender -b --factory-startup --python build.py -- --out BUILT
    python -I graft_shapes.py --shipped . --built BUILT --out OUT
    (then `manifests.write(OUT)`, compare, and copy what changed)

For every mesh of the shipped file, each shape the built file's mesh of
that name has and the shipped one doesn't becomes a new morph target,
**appended after the existing ones** (`extras.targetNames` and `weights`
grow with it). No existing accessor, buffer view or byte of the binary
chunk changes: the new data goes after the old.

- **Same triangles** (the body, `Body_LOD1`, the brows, lashes, eye shell
  and lip seal): the built shape's offsets and normal changes are taken
  corner by corner.
- **Other triangles** (`Body_LOD2`, the teeth): each shipped corner takes
  the offsets of the nearest point of the built reference mesh's surface
  (`REFERENCE`: the full built body for `Body_LOD2`, as `lods.py` binds a
  copy; the built mesh of the same name otherwise), among triangles of its
  own material, by that point's place in its triangle.

`Realistic.build.json` is the shipped one with the new shapes added to the
face's lists and a `grafted_shapes` record of what this did. Plain Python
with numpy.

A part built apart from the body that follows its shapes (the hairstyle,
`hair/ShortCrop.glb`, which `hair/build_hair.py` rebuilds to the same
vertices) takes its new shapes the same way, from a build of it against
the grafted body:

    python -I graft_shapes.py --shipped . --built BUILT --out OUT --model hair/ShortCrop.glb

**The record.** The build data beside each model (`Realistic.build.json`,
`hair/ShortCrop.build.json`) keeps a `grafted_shapes` record: `shipped`
(how long the binary chunk was before anything was grafted, and its
SHA-256 and CRC-32) and, per mesh, the shapes added, in order. A second
graft adds to it and keeps `shipped`. The record is what tells a grafted
model from a built one: `build.py` won't write over a folder that has
it, and

    python -I graft_shapes.py --check .

checks every recorded model against it: the chunk still begins with those
bytes, and each mesh's shapes end with the recorded ones on every
primitive. `avatar`'s `shipped_library` test checks the same from Rust.
"""

import hashlib
import json
import os
import struct
import sys
import zlib

import numpy as np

PARTS_FILE = "RealisticParts.glb"
DATA_FILE = "Realistic.build.json"
# The record's key in a model's build data, and its opening note.
RECORD = "grafted_shapes"
ABOUT = ("Shapes added to the shipped model by graft_shapes.py from a fresh build; everything else in "
         "the model is as it was shipped.")

# The built mesh a shipped mesh with other triangles takes its shapes
# from, when it isn't the one of its own name.
REFERENCE = {"Body_LOD2": "Body"}
# A mesh has "the same triangles" when its index buffers are equal and no
# corner is further than this from the built one (metres).
SAME_WITHIN = 0.001
# How many of the reference's nearest vertices a corner looks round.
NEAR = 12

TYPES = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def read_glb(path):
    with open(path, "rb") as f:
        data = f.read()
    magic, version, length = struct.unpack_from("<4sII", data, 0)
    if magic != b"glTF" or version != 2:
        raise ValueError(f"{path} isn't a binary glTF 2 file")
    at, gltf, binary = 12, None, b""
    while at < length:
        size, kind = struct.unpack_from("<II", data, at)
        chunk = data[at + 8:at + 8 + size]
        if kind == 0x4E4F534A:
            gltf = json.loads(chunk)
        elif kind == 0x004E4942:
            binary = chunk
        at += 8 + size
    return gltf, binary


def write_glb(path, gltf, binary):
    text = json.dumps(gltf, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    text += b" " * (-len(text) % 4)
    binary = bytes(binary) + b"\0" * (-len(binary) % 4)
    with open(path, "wb") as f:
        f.write(struct.pack("<4sII", b"glTF", 2, 12 + 8 + len(text) + 8 + len(binary)))
        f.write(struct.pack("<II", len(text), 0x4E4F534A))
        f.write(text)
        f.write(struct.pack("<II", len(binary), 0x004E4942))
        f.write(binary)


def accessor(gltf, binary, index):
    """An accessor's values, one row per element (zeros where it has no
    buffer view; sparse values go over them)."""
    a = gltf["accessors"][index]
    width, dtype = WIDTH[a["type"]], np.dtype(TYPES[a["componentType"]])
    out = np.zeros((a["count"], width), dtype=dtype)
    if "bufferView" in a:
        view = gltf["bufferViews"][a["bufferView"]]
        start = view.get("byteOffset", 0) + a.get("byteOffset", 0)
        stride = view.get("byteStride") or dtype.itemsize * width
        out = np.ndarray((a["count"], width), dtype=dtype, buffer=binary, offset=start,
                         strides=(stride, dtype.itemsize)).copy()
    sparse = a.get("sparse")
    if sparse:
        ix, values = sparse["indices"], sparse["values"]
        view = gltf["bufferViews"][ix["bufferView"]]
        at = np.frombuffer(binary, dtype=TYPES[ix["componentType"]], count=sparse["count"],
                           offset=view.get("byteOffset", 0) + ix.get("byteOffset", 0))
        view = gltf["bufferViews"][values["bufferView"]]
        rows = np.frombuffer(binary, dtype=dtype, count=sparse["count"] * width,
                             offset=view.get("byteOffset", 0) + values.get("byteOffset", 0))
        out[at.astype(np.int64)] = rows.reshape(-1, width)
    return out


class Meshes:
    """A file's meshes by node name: per primitive its material, indices,
    rest positions and targets by name (accessor indices)."""

    def __init__(self, gltf, binary):
        self.gltf, self.binary = gltf, binary
        self.by_name = {}
        for node in gltf["nodes"]:
            if "mesh" in node:
                self.by_name[node["name"]] = gltf["meshes"][node["mesh"]]

    def names(self, mesh):
        return self.by_name[mesh].get("extras", {}).get("targetNames", [])

    def primitives(self, mesh):
        out = []
        names = self.names(mesh)
        for primitive in self.by_name[mesh]["primitives"]:
            material = self.gltf["materials"][primitive["material"]]["name"] if "material" in primitive else None
            out.append({"material": material,
                        "indices": accessor(self.gltf, self.binary, primitive["indices"]).reshape(-1),
                        "P": accessor(self.gltf, self.binary, primitive["attributes"]["POSITION"]).astype(np.float64),
                        "targets": dict(zip(names, primitive.get("targets", []))),
                        "primitive": primitive})
        return out

    def target(self, primitive, name, attribute):
        """A target's dense rows (zeros when the target has no such
        attribute)."""
        index = primitive["targets"][name].get(attribute)
        if index is None:
            return np.zeros((len(primitive["P"]), 3), dtype=np.float32)
        return accessor(self.gltf, self.binary, index)


def closest_on_triangles(p, a, b, c):
    """The nearest point to `p` of each triangle (`a`, `b`, `c`: rows), as
    (distance, weights of a, b and c)."""
    ab, ac, ap = b - a, c - a, p - a
    d00, d01, d11 = (ab * ab).sum(1), (ab * ac).sum(1), (ac * ac).sum(1)
    d20, d21 = (ap * ab).sum(1), (ap * ac).sum(1)
    den = d00 * d11 - d01 * d01
    safe = np.where(np.abs(den) > 1e-24, den, 1.0)
    v = (d11 * d20 - d01 * d21) / safe
    w = (d00 * d21 - d01 * d20) / safe
    inside = (np.abs(den) > 1e-24) & (v >= 0) & (w >= 0) & (v + w <= 1)
    best = np.stack([1 - v - w, v, w], axis=1)
    distance = np.where(inside, np.linalg.norm(p - (a + v[:, None] * ab + w[:, None] * ac), axis=1), np.inf)

    def edge(x, y, order):
        nonlocal best, distance
        xy = y - x
        t = np.clip(((p - x) * xy).sum(1) / np.maximum((xy * xy).sum(1), 1e-24), 0.0, 1.0)
        d = np.linalg.norm(p - (x + t[:, None] * xy), axis=1)
        weights = np.zeros((len(a), 3))
        weights[:, order[0]], weights[:, order[1]] = 1 - t, t
        closer = d < distance
        best = np.where(closer[:, None], weights, best)
        distance = np.where(closer, d, distance)

    edge(a, b, (0, 1))
    edge(b, c, (1, 2))
    edge(c, a, (2, 0))
    return distance, best


def bind(points, positions, triangles):
    """For each of `points`, the triangle of the surface nearest it (of
    those round the `NEAR` nearest vertices), the weights of its corners
    there, and the distance."""
    around = [[] for _ in range(len(positions))]
    for k, triangle in enumerate(triangles):
        for i in triangle:
            around[i].append(k)
    tri, weights, far = np.zeros(len(points), np.int64), np.zeros((len(points), 3)), np.zeros(len(points))
    for start in range(0, len(points), 512):
        chunk = points[start:start + 512]
        d = np.linalg.norm(chunk[:, None, :] - positions[None, :, :], axis=2)
        nearest = np.argpartition(d, min(NEAR, len(positions) - 1), axis=1)[:, :NEAR]
        for row, p in enumerate(chunk):
            candidates = np.array(sorted({k for i in nearest[row] for k in around[i]}), dtype=np.int64)
            a, b, c = (positions[triangles[candidates, k]] for k in range(3))
            distance, w = closest_on_triangles(p, a, b, c)
            pick = int(np.argmin(distance))
            tri[start + row], weights[start + row], far[start + row] = candidates[pick], w[pick], distance[pick]
    return tri, weights, far


class Grower:
    """The shipped file with data added after what it has."""

    def __init__(self, gltf, binary):
        self.gltf = gltf
        self.binary = bytearray(binary)

    def view(self, data):
        self.binary += b"\0" * (-len(self.binary) % 4)
        self.gltf["bufferViews"].append({"buffer": 0, "byteLength": len(data), "byteOffset": len(self.binary)})
        self.binary += data
        return len(self.gltf["bufferViews"]) - 1

    def sparse(self, rows, bounds):
        """A sparse VEC3 accessor of `rows` (as the exporter writes a
        shape: only the rows that aren't zero, one zero row when none
        is). Returns its index."""
        rows = np.ascontiguousarray(rows, dtype=np.float32)
        at = np.nonzero(np.any(rows != 0.0, axis=1))[0]
        if len(at) == 0:
            at = np.array([0])
        kind = 5121 if at.max() < 256 else 5123 if at.max() < 65536 else 5125
        out = {"componentType": 5126, "count": len(rows), "type": "VEC3"}
        if bounds:
            out["max"] = [float(v) for v in rows.max(axis=0)]
            out["min"] = [float(v) for v in rows.min(axis=0)]
        out["sparse"] = {"count": len(at),
                         "indices": {"bufferView": self.view(at.astype(TYPES[kind]).tobytes()), "componentType": kind},
                         "values": {"bufferView": self.view(rows[at].tobytes())}}
        self.gltf["accessors"].append(out)
        return len(self.gltf["accessors"]) - 1


def graft(shipped_path, built_path, out_path):
    """Writes `out_path`: the shipped file with the built file's new
    shapes. Returns the record of what was done."""
    shipped_gltf, shipped_binary = read_glb(shipped_path)
    built_gltf, built_binary = read_glb(built_path)
    before = json.loads(json.dumps(shipped_gltf))
    shipped, built = Meshes(shipped_gltf, shipped_binary), Meshes(built_gltf, built_binary)
    grow = Grower(shipped_gltf, shipped_binary)
    record = {}
    for mesh in shipped.by_name:
        if mesh not in built.by_name:
            continue
        have = shipped.names(mesh)
        new = [name for name in built.names(mesh) if name not in have]
        if not new:
            continue
        ours, theirs = shipped.primitives(mesh), built.primitives(mesh)
        same = len(ours) == len(theirs) and all(
            a["material"] == b["material"] and len(a["P"]) == len(b["P"]) and np.array_equal(a["indices"], b["indices"])
            and float(np.abs(a["P"] - b["P"]).max()) <= SAME_WITHIN for a, b in zip(ours, theirs))
        entry = {"shapes": new, "by": "corner" if same else "nearest point"}
        if same:
            entry["rest_apart_most_mm"] = round(max(float(np.linalg.norm(a["P"] - b["P"], axis=1).max())
                                                    for a, b in zip(ours, theirs)) * 1000, 4)
        else:
            reference = REFERENCE.get(mesh, mesh)
            entry["from"] = reference
            source = built.primitives(reference)
        moved_most = {name: 0.0 for name in new}
        far_all, far_moved = 0.0, 0.0
        for k, primitive in enumerate(ours):
            if same:
                rows = {name: (built.target(theirs[k], name, "POSITION"), built.target(theirs[k], name, "NORMAL"))
                        for name in new}
            else:
                # The reference's primitives of this material, as one surface.
                parts = [p for p in source if p["material"] == primitive["material"]] or source
                offsets = np.cumsum([0] + [len(p["P"]) for p in parts])
                positions = np.concatenate([p["P"] for p in parts])
                triangles = np.concatenate([p["indices"].reshape(-1, 3).astype(np.int64) + offsets[i]
                                            for i, p in enumerate(parts)])
                tri, weights, far = bind(primitive["P"], positions, triangles)
                far_all = max(far_all, float(far.max()))
                rows = {}
                for name in new:
                    pair = []
                    for attribute in ("POSITION", "NORMAL"):
                        dense = np.concatenate([built.target(p, name, attribute) for p in parts]).astype(np.float64)
                        taken = (dense[triangles[tri]] * weights[:, :, None]).sum(axis=1)
                        taken[np.linalg.norm(taken, axis=1) < 1e-7] = 0.0
                        pair.append(taken.astype(np.float32))
                    rows[name] = tuple(pair)
                    moves = np.linalg.norm(pair[0], axis=1) > 1e-5
                    if moves.any():
                        far_moved = max(far_moved, float(far[moves].max()))
            # Each new target has what this primitive's own targets have.
            wanted = set()
            for target in primitive["primitive"].get("targets", []):
                wanted |= set(target)
            wanted = wanted or {"POSITION", "NORMAL"}
            targets = primitive["primitive"].setdefault("targets", [])
            for name in new:
                position, normal = rows[name]
                target = {"POSITION": grow.sparse(position, True)}
                if "NORMAL" in wanted:
                    target["NORMAL"] = grow.sparse(normal, False)
                targets.append(target)
                moved_most[name] = max(moved_most[name], float(np.linalg.norm(position, axis=1).max()))
        data = shipped.by_name[mesh]
        data.setdefault("extras", {})["targetNames"] = have + new
        data["weights"] = list(data.get("weights", [0.0] * len(have))) + [0.0] * len(new)
        entry["moved_most_mm"] = {name: round(v * 1000, 2) for name, v in moved_most.items()}
        if not same:
            entry["bound_furthest_mm"] = round(far_all * 1000, 3)
            entry["bound_furthest_where_moved_mm"] = round(far_moved * 1000, 3)
        record[mesh] = entry
    shipped_gltf["buffers"][0]["byteLength"] = len(grow.binary) + (-len(grow.binary) % 4)
    # Nothing that was there has changed: the binary chunk begins with the
    # shipped one, and every accessor and buffer view it had is as it was.
    assert bytes(grow.binary[:len(shipped_binary)]) == bytes(shipped_binary)
    assert shipped_gltf["accessors"][:len(before["accessors"])] == before["accessors"]
    assert shipped_gltf["bufferViews"][:len(before["bufferViews"])] == before["bufferViews"]
    for mesh_before, mesh_now in zip(before["meshes"], shipped_gltf["meshes"]):
        for a, b in zip(mesh_before["primitives"], mesh_now["primitives"]):
            assert a["attributes"] == b["attributes"] and a["indices"] == b["indices"]
            assert a.get("targets", []) == b.get("targets", [])[:len(a.get("targets", []))]
    for key in before:
        if key not in ("meshes", "accessors", "bufferViews", "buffers"):
            assert before[key] == shipped_gltf[key], key
    write_glb(out_path, shipped_gltf, grow.binary)
    return record, measure(shipped_binary)


def measure(binary):
    """What tells a binary chunk from another: its length and checksums."""
    binary = bytes(binary)
    return {"binary_bytes": len(binary), "binary_sha256": hashlib.sha256(binary).hexdigest(),
            "binary_crc32": zlib.crc32(binary)}


def data_file(model):
    """The build data beside `model` (a path in the package)."""
    return DATA_FILE if model == PARTS_FILE else model.replace(".glb", ".build.json")


def recorded(data, record, shipped):
    """`data`'s record with this graft added: `shipped` stays as the first
    graft found it, and each mesh's shapes go after those grafted before."""
    was = data.get(RECORD, {})
    meshes = dict(was.get("meshes", {}))
    for mesh, entry in record.items():
        before = meshes.get(mesh)
        if before:
            entry = dict(entry, shapes=before["shapes"] + entry["shapes"],
                         moved_most_mm=dict(before.get("moved_most_mm", {}), **entry["moved_most_mm"]))
        meshes[mesh] = entry
    return {"about": ABOUT, "shipped": was.get("shipped", shipped), "meshes": meshes}


def build_data(shipped_path, built_path, out_path, record, shipped):
    """The shipped build data with the record of this graft and, for the
    body, the new shapes in the face's lists. A part's build data (the
    hairstyle's) is the fresh build's, which already lists its shapes."""
    with open(shipped_path) as f:
        data = json.load(f)
    with open(built_path) as f:
        built = json.load(f)
    new = None
    if "face" in data:
        face, new_face = data["face"], built["face"]
        new = [name for name in new_face["identity"] if name not in face["identity"]]
        face["identity"] = face["identity"] + new
        for name in new:
            face["largest_move_mm"][name] = new_face["largest_move_mm"][name]
            if name in new_face.get("face_identity", {}):
                face["face_identity"][name] = new_face["face_identity"][name]
    else:
        was, data = data, built
        if RECORD in was:
            data[RECORD] = was[RECORD]
    data[RECORD] = recorded(data, record, shipped)
    with open(out_path, "w", newline="\n") as f:
        # Each as its own build writes it.
        if "face" in data:
            json.dump(data, f, indent=1, sort_keys=True)
        else:
            json.dump(data, f, indent=2)
        f.write("\n")
    return new


def check(package):
    """Checks every model of `package` with a record against it. Returns
    what is wrong, one line each (none: every grafted model is the shipped
    one with its recorded shapes after the others)."""
    wrong, found = [], 0
    for folder, _, files in os.walk(package):
        for name in sorted(files):
            if not name.endswith(".build.json"):
                continue
            with open(os.path.join(folder, name)) as f:
                record = json.load(f).get(RECORD)
            if record is None:
                continue
            found += 1
            model = PARTS_FILE if name == DATA_FILE else name.replace(".build.json", ".glb")
            path = os.path.join(folder, model)
            label = os.path.relpath(path, package)
            gltf, binary = read_glb(path)
            shipped = record.get("shipped")
            if not shipped:
                wrong.append(f"{label}: the record doesn't say what was shipped")
            elif measure(binary[:shipped["binary_bytes"]]) != shipped:
                wrong.append(f"{label}: its first {shipped['binary_bytes']} bytes of data are not the shipped ones")
            by_name = {mesh["name"]: mesh for mesh in gltf["meshes"]}
            count = 0
            for mesh, entry in record["meshes"].items():
                names = by_name.get(mesh, {}).get("extras", {}).get("targetNames", [])
                shapes = entry["shapes"]
                count += len(shapes)
                if names[len(names) - len(shapes):] != shapes:
                    wrong.append(f"{label}: `{mesh}`'s shapes don't end with the {len(shapes)} grafted ones")
                elif any(len(p.get("targets", [])) != len(names) for p in by_name[mesh]["primitives"]):
                    wrong.append(f"{label}: a primitive of `{mesh}` hasn't every shape")
            print(f"{label}: grafted, {count} shapes on {len(record['meshes'])} meshes after "
                  f"{shipped['binary_bytes'] if shipped else '?'} shipped bytes")
    if not found:
        print("no grafted model: nothing to check")
    return wrong


def main(argv):
    def arg(name):
        return argv[argv.index(name) + 1]

    if "--check" in argv:
        wrong = check(arg("--check"))
        for line in wrong:
            print("WRONG:", line)
        sys.exit(1 if wrong else 0)
    shipped, built, out = arg("--shipped"), arg("--built"), arg("--out")
    model = arg("--model") if "--model" in argv else PARTS_FILE
    os.makedirs(os.path.dirname(os.path.join(out, model)), exist_ok=True)
    record, was = graft(os.path.join(shipped, model), os.path.join(built, model), os.path.join(out, model))
    data = data_file(model)
    new = build_data(os.path.join(shipped, data), os.path.join(built, data), os.path.join(out, data), record, was)
    for mesh, entry in record.items():
        how = entry["by"] + (f" of {entry['from']}" if "from" in entry else "")
        extra = (f", bound at most {entry['bound_furthest_mm']} mm away "
                 f"({entry['bound_furthest_where_moved_mm']} mm where a shape moves)") if "from" in entry else \
            f", rest positions at most {entry['rest_apart_most_mm']} mm apart"
        print(f"{mesh:24s} {len(entry['shapes']):3d} shapes by {how}{extra}")
    if new is not None:
        print(f"{len(new)} new face identity shapes in {DATA_FILE}")


if __name__ == "__main__":
    main(sys.argv[1:])
