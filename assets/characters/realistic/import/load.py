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

Texture coordinates: the active UV layer, or with `"uv": "textures"` the
layer the base colour map is drawn with (a glTF can carry a second set
that its maps use, as BlenderKit's Grey sweat pants do). A map in a
format the engine doesn't read (WebP) is written as PNG.

    python -I load.py --spec <asset>.intake.json --downloads <folder> --work <folder>
"""

import argparse
import io
import json
import os

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


# The map formats the engine reads, as written.
KEPT = {"JPEG": ".jpg", "PNG": ".png"}


def _save_image(image, folder, name):
    """The image's file as it came when the engine reads its format;
    anything else (WebP) decoded and written as PNG (Pillow)."""
    os.makedirs(folder, exist_ok=True)
    source = bpy.path.abspath(image.filepath) if image.filepath else ""
    if image.packed_file is not None:
        data = image.packed_file.data
    elif source and os.path.isfile(source):
        with open(source, "rb") as f:
            data = f.read()
    else:
        return None
    from PIL import Image
    with Image.open(io.BytesIO(data)) as decoded:
        if decoded.format in KEPT:
            out = os.path.join(folder, name + KEPT[decoded.format])
            with open(out, "wb") as f:
                f.write(data)
        else:
            out = os.path.join(folder, name + ".png")
            decoded.save(out, "PNG")
    return os.path.basename(out)


def texture_uv_layer(material):
    """The UV layer the material's base colour map is drawn with (a UV Map
    node feeding the image's vector), or None for the active one."""
    if material is None or not material.use_nodes:
        return None
    bsdf = next((n for n in material.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if bsdf is None:
        return None
    image_node = None
    for link in bsdf.inputs["Base Color"].links:
        node = link.from_node
        if node.type == "TEX_IMAGE":
            image_node = node
    if image_node is None:
        return None
    for link in image_node.inputs["Vector"].links:
        node = link.from_node
        # glTF's importer puts a Mapping node between when the texture is
        # transformed; follow it to the UV Map node.
        while node.type != "UVMAP" and node.inputs and node.inputs[0].links:
            node = node.inputs[0].links[0].from_node
        if node.type == "UVMAP" and node.uv_map:
            return node.uv_map
    return None


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


def to_workmesh(objects, folder, uv_from=None):
    """The objects joined, in the engine's space, with their materials.
    `uv_from="textures"` reads the UV layer the maps are drawn with."""
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
        slots = [s.material for s in obj.material_slots]
        uv_layer = me.uv_layers.active
        if uv_from == "textures":
            named = next((texture_uv_layer(m) for m in slots if texture_uv_layer(m)), None)
            if named and named in me.uv_layers:
                uv_layer = me.uv_layers[named]
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
    mesh = to_workmesh(objects, os.path.join(work, "textures"), options.get("uv"))
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
