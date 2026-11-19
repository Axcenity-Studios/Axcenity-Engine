"""Step 8 of the intake: the engine's own fit, and the item (plain Python
with NumPy; Cargo for the fit).

Each level of detail is written as an OBJ in the engine's space and
imported by the engine's own import (`cargo run -p axcenity-avatar
--release --example import_garment`: alignment, binding to the body's
surface, the body's weights, identity and corrective shapes, push-out from
the body and the items named in `fit.under`, the regions it hides), in a
scratch copy of the realistic library, as the wardrobe does
(`../clothing/wardrobe.py`, whose GLB helpers and scratch library this
reuses). The fitted levels and the source's own maps are put together into
the item's model (`<Object>.glb`: `<Object>`, `<Object>_LOD1`, ...), and
beside it go its binding, thumbnail, manifest (`quality: "draft"`: nothing
becomes Production without the review, asset-quality.md section 1) and
provenance record (`<Object>.provenance.json`, with what every step
changed and the item's ID). The model is read back and its levels'
triangles and the weights it ships are judged against section 2's
skinning rules (`export.json`), and the library's `catalogue.json` is
written again so the provenance check and a capture can read it.

The weights the item ships are the engine fit's, not the intake's
(`skinning.py` only stands in for them while the item is conformed and
checked), except for a rigid accessory: `"fit": {"rigid_bone": "Head"}`
puts every vertex on that bone alone, at the place the intake gave it,
without the body's shapes (`rigidify`).

    python -I fit.py --spec <asset>.intake.json --work <folder> --out <library folder>
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REALISTIC = os.path.dirname(HERE)
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(REALISTIC)))
sys.path.append(os.path.join(REALISTIC, "clothing"))

import wardrobe  # noqa: E402  (the realistic wardrobe's GLB helpers and scratch library)

import checks  # noqa: E402
import provenance  # noqa: E402
from workmesh import WorkMesh  # noqa: E402

BASE = "axcenity/realistic"


def object_name(label):
    """The import's object name for a label (`avatar::fit::import`'s
    `object_name`): its words run together, each capitalised."""
    words = [w for w in re.split(r"[^0-9A-Za-z]", label) if w]
    return "".join(w[0].upper() + w[1:] for w in words) or "Garment"


def smooth_normals(mesh):
    """Area-weighted vertex normals, the same across a split seam."""
    ids = checks.weld(mesh.P)
    tris = ids[mesh.triangles()]
    W = np.zeros((ids.max() + 1, 3))
    P = np.zeros((ids.max() + 1, 3))
    P[ids] = mesh.P
    face = np.cross(P[tris[:, 1]] - P[tris[:, 0]], P[tris[:, 2]] - P[tris[:, 0]])
    for k in range(3):
        np.add.at(W, tris[:, k], face)
    W /= np.maximum(np.linalg.norm(W, axis=1, keepdims=True), 1e-12)
    return W[ids]


def write_obj(path, mesh):
    """Triangles in the engine's space with texture coordinates (Blender's,
    v up, as OBJ has them) and smooth normals. A vertex is written once
    per texture coordinate it has (split at the UV seams, one index for
    `v`, `vt` and `vn`), as the wardrobe writes them: the engine's import
    keeps a garment's texture coordinates only when each vertex has one."""
    N = smooth_normals(mesh)
    mtl = os.path.splitext(os.path.basename(path))[0] + ".mtl"
    with open(os.path.join(os.path.dirname(path), mtl), "w", newline="\n") as f:
        for m in mesh.materials:
            f.write(f"newmtl {m['name']}\n")
            # The engine's OBJ reader keeps texture coordinates only for a
            # material with a texture.
            if m.get("base_color"):
                f.write(f"map_Kd textures/{m['base_color']}\n")
    # One vertex per (position, texture coordinate) pair.
    keys = np.concatenate([mesh.corners[:, None].astype(float), np.round(mesh.uv, 7)], axis=1)
    unique, vertex_of_corner = np.unique(keys, axis=0, return_inverse=True)
    vertex_of_corner = vertex_of_corner.reshape(-1)
    position = unique[:, 0].astype(np.int64)
    tc = mesh.triangle_corners()
    tf = mesh.triangle_faces()
    with open(path, "w", newline="\n") as f:
        f.write("# An item made by the Axcenity intake (assets/characters/realistic/import)\n")
        f.write(f"mtllib {mtl}\n")
        for p in mesh.P[position]:
            f.write(f"v {p[0]:.6f} {p[1]:.6f} {p[2]:.6f}\n")
        for t in unique[:, 1:]:
            f.write(f"vt {t[0]:.6f} {t[1]:.6f}\n")
        for n in N[position]:
            f.write(f"vn {n[0]:.5f} {n[1]:.5f} {n[2]:.5f}\n")
        for m in range(len(mesh.materials)):
            rows = np.nonzero(mesh.face_material[tf] == m)[0]
            if len(rows) == 0:
                continue
            f.write(f"usemtl {mesh.materials[m]['name']}\n")
            for r in rows:
                parts = [f"{vertex_of_corner[c] + 1}/{vertex_of_corner[c] + 1}/{vertex_of_corner[c] + 1}"
                         for c in tc[r]]
                f.write("f " + " ".join(parts) + "\n")


def run_import(spec, obj_path, scratch, label, folder):
    item = spec["item"]
    fit = spec.get("fit", {})
    cmd = ["cargo", "run", "-q", "-p", "axcenity-avatar", "--release", "--example", "import_garment", "--",
           obj_path, scratch, BASE, f"build/{folder}", label, f"build/{folder}",
           "--slot", item["slot"], "--margin", str(fit.get("margin", 0.002)),
           "--author", spec["source"]["author"], "--licence", spec["source"]["licence"]]
    # The intake placed the item on the body itself (conform.py): the
    # import keeps that place instead of aligning it again.
    if spec.get("fit", {}).get("keep_placement", True):
        cmd.append("--placed")
    for u in fit.get("under", []):
        cmd += ["--under", u]
    out = subprocess.run(cmd, cwd=REPOSITORY, capture_output=True, text=True)
    if out.returncode:
        raise SystemExit(out.stdout + out.stderr)
    return out.stdout


def _add_accessor(doc, binary, array, component, kind):
    """Appends `array` as a new accessor; returns its index."""
    dtype = {5126: np.float32, 5123: np.uint16}[component]
    data = np.ascontiguousarray(array, dtype=dtype).tobytes()
    while len(binary) % 4:
        binary.append(0)
    doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(data)})
    binary.extend(data)
    accessor = {"bufferView": len(doc["bufferViews"]) - 1, "componentType": component,
                "count": int(len(array)), "type": kind}
    if kind == "VEC3" and component == 5126:
        accessor["min"] = np.asarray(array).min(axis=0).astype(float).tolist()
        accessor["max"] = np.asarray(array).max(axis=0).astype(float).tolist()
    doc["accessors"].append(accessor)
    return len(doc["accessors"]) - 1


def rigidify(doc, binary, bone, placed):
    """A rigid accessory (a hat on the head): every vertex of every level
    on `bone` alone, at its place as the intake put it (the engine fit's
    push-out undone: it binds a far vertex, such as a brim's tip, to the
    skin along its normal, which can be a shoulder), and without the
    body's shapes (a rigid item doesn't take the face's or the build's
    shape). `placed[k]` is level k's work mesh. Returns what moved."""
    import bodyglb
    skin = doc["skins"][0]
    names = [doc["nodes"][j]["name"] for j in skin["joints"]]
    joint = names.index(bone)
    moved = []
    for k, mesh in enumerate(doc["meshes"]):
        level = placed[min(k, len(placed) - 1)]
        # The placed corners by their texture coordinate (glTF's v is
        # Blender's 1 - v).
        by_uv = {}
        for corner, vertex in enumerate(level.corners):
            u, v = level.uv[corner]
            by_uv.setdefault((round(u, 4), round(1 - v, 4)), []).append(level.P[vertex])
        for prim in mesh["primitives"]:
            a = prim["attributes"]
            P = bodyglb.accessor(doc, binary, a["POSITION"])
            uv = bodyglb.accessor(doc, binary, a["TEXCOORD_0"]) if "TEXCOORD_0" in a else None
            out = P.copy()
            for i in range(len(P)):
                found = by_uv.get((round(uv[i, 0], 4), round(uv[i, 1], 4))) if uv is not None else None
                if found:
                    candidates = np.array(found)
                    out[i] = candidates[np.argmin(((candidates - P[i]) ** 2).sum(axis=1))]
            moved.append(float(np.linalg.norm(out - P, axis=1).max()))
            a["POSITION"] = _add_accessor(doc, binary, out, 5126, "VEC3")
            joints = np.zeros((len(P), 4), np.uint16)
            joints[:, 0] = joint
            weights = np.zeros((len(P), 4), np.float32)
            weights[:, 0] = 1.0
            a["JOINTS_0"] = _add_accessor(doc, binary, joints, 5123, "VEC4")
            a["WEIGHTS_0"] = _add_accessor(doc, binary, weights, 5126, "VEC4")
            prim.pop("targets", None)
        mesh.pop("weights", None)
        mesh.get("extras", {}).pop("targetNames", None)
    return {"bone": bone, "push_out_undone_m": round(max(moved), 4) if moved else 0.0,
            "shapes_dropped": True}


def assemble(spec, fitted, maps, out, placed=None):
    """LOD0's fitted model with the further levels added as
    `<object>_LOD<n>` and the source's maps on the item's materials; a
    rigid item (`fit.rigid_bone`) is then made rigid ([`rigidify`])."""
    item = spec["item"]
    doc, binary = wardrobe.glb_read(fitted[0])
    node0 = next(k for k, n in enumerate(doc["nodes"]) if "mesh" in n)
    doc["nodes"][node0]["name"] = item["object"]
    doc["meshes"][0]["name"] = item["object"]
    for prim in doc["meshes"][0]["primitives"]:
        prim["material"] = doc["materials"][prim["material"]]["name"]
    armature = next(k for k, n in enumerate(doc["nodes"]) if node0 in n.get("children", []))
    for level, path in enumerate(fitted[1:], start=1):
        other, other_bin = wardrobe.glb_read(path)
        (mesh,) = wardrobe.append(doc, binary, other, other_bin)
        doc["meshes"][mesh]["name"] = f"{item['object']}_LOD{level}"
        doc["nodes"].append({"name": f"{item['object']}_LOD{level}", "mesh": mesh, "skin": 0})
        doc["nodes"][armature]["children"].append(len(doc["nodes"]) - 1)
    names = [m["name"] for m in spec["_materials"]]
    doc["materials"] = []
    for m in spec["_materials"]:
        options = item.get("materials", {}).get(m["name"], {})
        # Section 2: no surface is seen from both sides unless it is a
        # card; a material opts in (`"double_sided": true`), so a flipped
        # face isn't hidden.
        mat = {"name": m["name"], "doubleSided": bool(options.get("double_sided", False)),
               "pbrMetallicRoughness": {"metallicFactor": 1.0, "roughnessFactor": 1.0}}
        found = maps.get(m["name"], {})
        mime = {".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".png": "image/png"}
        for key in ("base_color", "normal", "orm"):
            if key in found:
                tex = wardrobe.image(doc, binary, found[key],
                                     mime[os.path.splitext(found[key])[1].lower()])
                if key == "base_color":
                    mat["pbrMetallicRoughness"]["baseColorTexture"] = {"index": tex}
                elif key == "normal":
                    mat["normalTexture"] = {"index": tex}
                else:
                    mat["pbrMetallicRoughness"]["metallicRoughnessTexture"] = {"index": tex}
                    mat["occlusionTexture"] = {"index": tex}
        if "orm" not in found:
            mat["pbrMetallicRoughness"]["metallicFactor"] = 0.0
            mat["pbrMetallicRoughness"]["roughnessFactor"] = 0.8
        if options.get("shading", "standard") != "standard":
            mat["extras"] = {"axcenity_shading": options["shading"]}
        doc["materials"].append(mat)
    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            if isinstance(prim["material"], str):
                prim["material"] = names.index(prim["material"]) if prim["material"] in names else 0
    doc["asset"]["generator"] = "Axcenity Engine licensed-asset intake (assets/characters/realistic/import)"
    binary = bytearray(binary)
    rigid = None
    bone = spec.get("fit", {}).get("rigid_bone")
    if bone and placed:
        rigid = rigidify(doc, binary, bone, placed)
    wardrobe.glb_write(out, doc, binary)
    return rigid


def read_back(path, body=None, rigid_bone=None):
    """Each level's triangles and skinning, from the written model: the
    weights it ships (the engine fit's, or the rigid bone's), judged
    against section 2's skinning rules, with the far-bone rule measured
    against `body`'s joints when it is given."""
    import bodyglb
    doc, binary = bodyglb.read_glb(path)
    out = {}
    for node in doc["nodes"]:
        if "mesh" not in node:
            continue
        tris, points, joints, weights, uvs = 0, [], [], [], True
        for prim in doc["meshes"][node["mesh"]]["primitives"]:
            tris += doc["accessors"][prim["indices"]]["count"] // 3
            a = prim["attributes"]
            uvs &= "TEXCOORD_0" in a
            if "WEIGHTS_0" in a:
                points.append(bodyglb.accessor(doc, binary, a["POSITION"]))
                joints.append(bodyglb.accessor(doc, binary, a["JOINTS_0"]).astype(np.int64))
                weights.append(bodyglb.accessor(doc, binary, a["WEIGHTS_0"]))
        entry = {"triangles": int(tris), "texture_coordinates": uvs}
        if weights:
            J, W = np.vstack(joints), np.vstack(weights)
            skin = checks.skinning(J, W)
            if body is not None and "skin" in node:
                # The skin's joints by name, in the order JOINTS_0 counts.
                bones = [doc["nodes"][j]["name"] for j in doc["skins"][node["skin"]]["joints"]]
                skin["far_from_their_bone"] = checks.far_weights(np.vstack(points), J, W, bones,
                                                                 body.heads, body.parents)
            entry["skinning"] = skin
            entry["limits"] = checks.skin_limits(skin, rigid_bone)
        out[node["name"]] = entry
    return out


def write_catalogue(root):
    """`catalogue.json` of the library at `root`, by the library's own
    script (`assets/characters/catalogue.py`), so the provenance check and
    a capture can read the library the item went into."""
    sys.path.insert(0, os.path.join(REPOSITORY, "assets", "characters"))
    try:
        import catalogue
    finally:
        sys.path.pop(0)
    catalogue.ROOT = os.path.abspath(root)
    catalogue.main()
    with open(os.path.join(root, "catalogue.json"), encoding="utf-8") as f:
        written = json.load(f)
    return {"bases": len(written["bases"]), "items": len(written["items"])}


def changes_of(spec, work):
    """What the steps changed, from their reports, for the record."""
    def report(name):
        path = os.path.join(work, name)
        if not os.path.exists(path):
            return {}
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    out = []
    loaded = report("loaded.json")
    if loaded.get("units_to_metres", 1.0) != 1.0:
        out.append(f"scaled from its file's units to metres (x{loaded['units_to_metres']})")
    topo = report("topology.json")
    fixes = topo.get("fixes", {})
    done = [f"{v} {k.replace('_', ' ')}" for k, v in fixes.items()
            if k in ("welded", "degenerate_dissolved", "loose_removed", "faces_turned", "ngons_cut") and v]
    if done:
        out.append("topology repaired: " + ", ".join(done))
    if "decimate" in topo:
        out.append(f"decimated to {topo['decimate']['triangles_after']} triangles")
    if "remesh" in topo:
        out.append("remeshed (voxel)")
    conformed = report("conformed.json")
    if "place" in conformed:
        place = conformed["place"]
        how = [f"scaled x{place['scale']}"] if place.get("scale", 1.0) != 1.0 else []
        how += [f"turned {d} degrees about {axis}" for axis, d in zip("XYZ", place.get("rotate_degrees", []))
                if d]
        on = f"'s {place['centred_on'].lower()}" if place.get("centred_on") else ""
        out.append(f"placed on the Axcenity realistic body{on}" + (f" ({', '.join(how)})" if how else ""))
    if "pose" in conformed:
        out.append("posed to the realistic body's rest pose")
    refined = report("refined.json") if os.path.exists(os.path.join(work, "refined.npz")) else {}
    if refined.get("unwrap"):
        method = refined["unwrap"].get("method", "ANGLE_BASED").lower().replace("_", "-")
        out.append(f"texture coordinates laid out again (Blender's {method} unwrap along the source's own seams)")
    for name in refined.get("materials", {}):
        out.append(f"a {name.lower()} band made from the faces along its edge")
    hems = refined.get("hems", [])
    if hems:
        out.append("turned hems added at every open edge (" + ", ".join(
            f"{h['loop']}: {h['thickness_m'] * 1000:g} mm thick, {h['turn_up_m'] * 1000:g} mm turned up"
            for h in hems) + ")")
    if "maps" in refined:
        out.append(f"maps baked again from Poly Haven's {refined['maps']['fabric'].replace('_', ' ')} (CC0) "
                   "with seams and stitching; the source's own maps dropped")
    if os.path.exists(os.path.join(work, "lod1.npz")):
        out.append("simpler copies made by decimation (levels of detail)")
    if spec.get("fit", {}).get("rigid_bone"):
        out.append(f"fitted by the Axcenity engine's fit, then made rigid on the {spec['fit']['rigid_bone']} bone")
    else:
        out.append("fitted, rigged and given the body's shapes by the Axcenity engine's fit")
    out.append("materials rebuilt for the Axcenity engine")
    return [c for c in out if c]


def step(spec, work, out_root):
    import bodyglb
    item = spec["item"]
    loaded = WorkMesh.load(os.path.join(work, "lod0.npz"))
    spec = dict(spec)
    spec["_materials"] = loaded.materials
    maps = {m["name"]: {k: os.path.join(work, "textures", m[k]) for k in ("base_color", "normal", "orm")
                        if m.get(k)} for m in loaded.materials}
    scratch = os.path.join(work, "library")
    wardrobe.library(scratch)
    reports, fitted = [], []
    level = 0
    while os.path.exists(os.path.join(work, f"lod{level}.npz")):
        mesh = WorkMesh.load(os.path.join(work, f"lod{level}.npz"))
        obj_path = os.path.join(work, f"lod{level}.obj")
        write_obj(obj_path, mesh)
        label = f"{item['object']} lod{level}"
        folder = f"intake-lod{level}"
        reports.append(run_import(spec, obj_path, scratch, label, folder))
        fitted.append(os.path.join(scratch, "build", folder, object_name(label) + ".glb"))
        level += 1
    folder = os.path.join(out_root, *item["folder"].split("/"))
    os.makedirs(folder, exist_ok=True)
    obj = item["object"]
    placed = [WorkMesh.load(os.path.join(work, f"lod{k}.npz")) for k in range(len(fitted))]
    rigid = assemble(spec, fitted, maps, os.path.join(folder, f"{obj}.glb"), placed)
    lod0 = os.path.join(scratch, "build", "intake-lod0")
    src = object_name(f"{obj} lod0")
    shutil.copy(os.path.join(lod0, f"{src}.binding.json"), os.path.join(folder, f"{obj}.binding.json"))
    shutil.copy(os.path.join(lod0, f"{src}.png"), os.path.join(folder, f"{obj}.png"))
    with open(os.path.join(lod0, f"{src}.item.json"), encoding="utf-8") as f:
        imported = json.load(f)
    record = provenance.finish(work, [item["id"]], changes_of(spec, work))
    with open(os.path.join(folder, f"{obj}.provenance.json"), "w", newline="\n", encoding="utf-8") as f:
        json.dump(record, f, indent=2)
        f.write("\n")
    source = spec["source"]
    manifest = {
        "format_version": 1,
        "id": item["id"],
        "label": item["label"],
        "kind": item["kind"],
        "slot": item["slot"],
        "tier": item.get("tier", "built_in"),
        "bases": [BASE],
        "model": {"file": f"{obj}.glb", "object": f"AxcenityRealistic/{obj}"},
        "hides": item["hides"] if isinstance(item.get("hides"), list) else imported.get("hides", []),
        "materials": [dict({"name": m["name"],
                            "shading": item.get("materials", {}).get(m["name"], {}).get("shading", "standard"),
                            "label": item.get("materials", {}).get(m["name"], {}).get("label", m["name"])},
                           **({"role": item["materials"][m["name"]]["role"]}
                              if "role" in item.get("materials", {}).get(m["name"], {}) else {}))
                      for m in loaded.materials],
        "fit": {"method": "binding", "data": f"{obj}.binding.json",
                "margin": spec.get("fit", {}).get("margin", 0.002)},
        "tags": item.get("tags", []),
        "thumbnail": f"{obj}.png",
        "source": {"author": f"{source['author']}; fitted by {provenance.CREDIT}",
                   "licence": source["licence"], "url": source["source_page"],
                   "origin": "imported", "provenance": f"{obj}.provenance.json"},
        "about": item.get("about"),
        "quality": "draft",
    }
    if item["kind"] == "garment":
        manifest["layer"] = item.get("layer", 1)
    if imported.get("also_fills"):
        manifest["also_fills"] = imported["also_fills"]
    manifest = {k: v for k, v in manifest.items() if v is not None}
    with open(os.path.join(folder, f"{obj}.item.json"), "w", newline="\n", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")
    with open(os.path.join(work, "import.txt"), "w", newline="\n") as f:
        f.write("\n\n".join(reports))
    export = {
        "folder": item["folder"],
        "files": sorted(os.listdir(folder)),
        "levels": read_back(os.path.join(folder, f"{obj}.glb"), bodyglb.load(spec.get("body")),
                            spec.get("fit", {}).get("rigid_bone")),
        "rigid": rigid,
        "import": [[line for line in r.splitlines()
                    if line.startswith(("imported", "slot", "alignment", "coverage"))] for r in reports],
    }
    export["catalogue"] = write_catalogue(out_root)
    with open(os.path.join(work, "export.json"), "w", newline="\n") as f:
        json.dump(export, f, indent=1)
        f.write("\n")
    return export


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    parser.add_argument("--out", required=True, help="the character library folder the item goes into")
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work, args.out), indent=1))


if __name__ == "__main__":
    main()
