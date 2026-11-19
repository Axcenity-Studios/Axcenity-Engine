"""Step 2 of the intake: import the downloaded mesh (Blender's Python).

glTF (`.gltf`, `.glb`), FBX and OBJ are read with Blender's importers; a
`.blend` only with its scripts off (auto-execution disabled before it is
opened, `use_scripts=False`, `load_ui=False`): a downloaded file is data,
never code. Every mesh object is taken with its modifiers and transforms
applied and joined into one work mesh in the engine's space (metres, Y
up, facing +Z; Blender's (x, y, z) is glTF's (x, z, -y)). Each
material's maps (base colour, normal, and the occlusion-roughness-metal
image glTF calls metallicRoughness) are copied into `work/textures/`.

Units: `"units": "auto"` takes the first of metres, centimetres,
decimetres, millimetres and inches that makes the item between 3 cm and
2.2 m at its largest (Blender's glTF and FBX importers already give
metres); a number is a scale to metres. `"up": "z"` reads an OBJ made Z
up (OBJ's usual is Y up).

    python -I load.py --spec <asset>.intake.json --downloads <folder> --work <folder>
"""

import argparse
import json
import os
import shutil

import bpy
import numpy as np

from workmesh import WorkMesh

UNITS = [1.0, 0.01, 0.1, 0.001, 0.0254]


def open_file(path, up=None):
    """Reads `path` into an empty scene; returns its mesh objects."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    ext = os.path.splitext(path)[1].lower()
    if ext in (".gltf", ".glb"):
        bpy.ops.import_scene.gltf(filepath=path)
    elif ext == ".fbx":
        bpy.ops.import_scene.fbx(filepath=path)
    elif ext == ".obj":
        if up == "z":
            bpy.ops.wm.obj_import(filepath=path, forward_axis="Y", up_axis="Z")
        else:
            bpy.ops.wm.obj_import(filepath=path, forward_axis="NEGATIVE_Z", up_axis="Y")
    elif ext == ".blend":
        # Never run what a downloaded file carries.
        bpy.context.preferences.filepaths.use_scripts_auto_execute = False
        bpy.ops.wm.open_mainfile(filepath=path, load_ui=False, use_scripts=False)
    else:
        raise SystemExit(f"can't read `{ext}`: use glTF, FBX, OBJ or .blend")
    return [o for o in bpy.context.scene.objects if o.type == "MESH"]


def _upstream_image(socket, seen=None):
    """The first image texture feeding `socket`, through any nodes."""
    seen = seen or set()
    for link in socket.links:
        node = link.from_node
        if node.name in seen:
            continue
        seen.add(node.name)
        if node.type == "TEX_IMAGE" and node.image:
            return node.image
        for inp in node.inputs:
            found = _upstream_image(inp, seen)
            if found:
                return found
    return None


def _save_image(image, folder, name):
    """Copies `image` into `folder` as `name` plus its extension. The
    engine reads PNG and JPEG, so any other format (a glTF's packed WebP,
    a TGA beside an FBX) is written as a PNG (Pillow), never its bytes
    under a `.png` name."""
    os.makedirs(folder, exist_ok=True)
    source = bpy.path.abspath(image.filepath) if image.filepath else ""
    if image.packed_file is None and source and os.path.isfile(source):
        ext = os.path.splitext(source)[1].lower()
        if ext in (".png", ".jpg", ".jpeg"):
            out = os.path.join(folder, name + ext)
            shutil.copyfile(source, out)
            return os.path.basename(out)
        with open(source, "rb") as f:
            return _as_png(f.read(), folder, name)
    if image.packed_file is not None:
        data = image.packed_file.data
        ext = {"JPEG": ".jpg", "PNG": ".png"}.get(image.file_format)
        if ext is None:
            return _as_png(data, folder, name)
        out = os.path.join(folder, name + ext)
        with open(out, "wb") as f:
            f.write(data)
        return os.path.basename(out)
    return None


def _as_png(data, folder, name):
    """`data` (an image file's bytes in any format Pillow reads) written as
    `name.png` in `folder`."""
    import io

    from PIL import Image
    out = os.path.join(folder, name + ".png")
    Image.open(io.BytesIO(data)).save(out, "PNG")
    return os.path.basename(out)


def material_maps(material, folder):
    """A material's base colour, normal and ORM maps, copied into `folder`."""
    out = {"name": material.name}
    if not material.use_nodes:
        return out
    bsdf = next((n for n in material.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if bsdf is None:
        return out
    stem = bpy.path.clean_name(material.name)
    for key, socket in (("base_color", "Base Color"), ("normal", "Normal"), ("orm", "Roughness")):
        image = _upstream_image(bsdf.inputs[socket])
        if image is not None:
            saved = _save_image(image, folder, f"{stem}_{key}")
            if saved:
                out[key] = saved
    return out


def to_workmesh(objects, folder):
    """The objects joined, in the engine's space, with their materials."""
    depsgraph = bpy.context.evaluated_depsgraph_get()
    Ps, corners, sizes, uvs, face_material = [], [], [], [], []
    materials, by_name = [], {}
    offset = 0
    for obj in objects:
        evaluated = obj.evaluated_get(depsgraph)
        me = evaluated.to_mesh()
        M = np.array(obj.matrix_world)
        co = np.zeros(len(me.vertices) * 3)
        me.vertices.foreach_get("co", co)
        P = co.reshape(-1, 3) @ M[:3, :3].T + M[:3, 3]
        # A mirrored transform turns the faces inside out: put them back.
        mirrored = np.linalg.det(M[:3, :3]) < 0
        Ps.append(np.stack([P[:, 0], P[:, 2], -P[:, 1]], axis=1))
        uv_layer = me.uv_layers.active
        slots = [s.material for s in obj.material_slots]
        for poly in me.polygons:
            loops = list(poly.loop_indices)
            verts = [me.loops[k].vertex_index + offset for k in loops]
            uv = [tuple(uv_layer.data[k].uv) if uv_layer else (0.0, 0.0) for k in loops]
            if mirrored:
                verts, uv = verts[::-1], uv[::-1]
            corners += verts
            uvs += uv
            sizes.append(len(verts))
            mat = slots[poly.material_index] if poly.material_index < len(slots) else None
            name = mat.name if mat else "Material"
            if name not in by_name:
                by_name[name] = len(materials)
                materials.append(material_maps(mat, folder) if mat else {"name": name})
            face_material.append(by_name[name])
        offset += len(P)
        evaluated.to_mesh_clear()
    return WorkMesh(np.vstack(Ps), corners, sizes, np.array(uvs), face_material, materials)


def units_of(mesh, units):
    if units != "auto":
        return float(units)
    size = float((mesh.P.max(axis=0) - mesh.P.min(axis=0)).max())
    for u in UNITS:
        if 0.03 <= size * u <= 2.2:
            return u
    return 1.0


def step(spec, downloads, work):
    options = spec.get("load", {})
    path = os.path.join(downloads, *spec["source"]["model"].split("/"))
    objects = open_file(path, options.get("up"))
    if not objects:
        raise SystemExit(f"`{spec['source']['model']}` has no mesh")
    mesh = to_workmesh(objects, os.path.join(work, "textures"))
    units = units_of(mesh, options.get("units", "auto"))
    mesh.P = mesh.P * units
    size = mesh.P.max(axis=0) - mesh.P.min(axis=0)
    report = {
        "file": spec["source"]["model"],
        "objects": [o.name for o in objects],
        "units_to_metres": units,
        "size_m": size.round(4).tolist(),
        "vertices": int(len(mesh.P)),
        "faces": int(len(mesh.sizes)),
        "materials": mesh.materials,
    }
    mesh.save(os.path.join(work, "loaded.npz"), report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--downloads", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.downloads, args.work), indent=1))


if __name__ == "__main__":
    main()
