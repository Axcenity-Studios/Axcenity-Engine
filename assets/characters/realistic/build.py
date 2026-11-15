"""Builds the Axcenity realistic base body in Blender from Blender Studio's
CC0 realistic base meshes (vendored in `source/`).

Run headless:  blender -b --factory-startup --python build.py -- --out DIR
(or with the `bpy` module: python -I build.py --out DIR)

    --texture-size 2048   the skin maps' size (4096 for the hero set)
    --skip-bakes          no skin maps (a plain skin colour): for quick
                          checks of the mesh and the rig

The steps: load both frames, lay out the texture coordinates, bake the
skin maps from the vendored sculpts (in the bundle's pose), re-pose both
frames to the 45 degree A-pose and stand them (the masculine one 1.75 m
tall), fit the skeleton from the face sets with its twist and helper
bones, weight the body, add `Frame_Feminine`, the eyes and the teeth, mark
the body regions, make the simpler copies (`Body_LOD1`, `Body_LOD2`) and
export `RealisticParts.glb`, then the base model manifest
(`manifests.py`; afterwards rebuild the library's catalogue with
`../catalogue.py`). Units are metres, Blender Z up, the character faces -Y
(glTF +Z after export).
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402

import body  # noqa: E402
import lods  # noqa: E402
import manifests  # noqa: E402
import parts  # noqa: E402
import regions  # noqa: E402
import rig  # noqa: E402
import shapes  # noqa: E402
import skin  # noqa: E402
import uvs  # noqa: E402
from common import face_sets, reset_scene, set_positions, shade_smooth, tri_count  # noqa: E402

ARMATURE = "AxcenityRealistic"
PARTS_FILE = "RealisticParts.glb"
DATA_FILE = "Realistic.build.json"


def to_gltf(d):
    """A Blender-space vector (Z up, front -Y) in glTF space (Y up, front +Z)."""
    return [round(d[0], 5), round(d[2], 5), round(-d[1], 5)]


def skin_to(obj, armature):
    obj.parent = armature
    for mod in list(obj.modifiers):
        if mod.type == "ARMATURE":
            obj.modifiers.remove(mod)
    mod = obj.modifiers.new("Armature", "ARMATURE")
    mod.object = armature


def add_shape(obj, name, base, target):
    """A shape key `name` on `obj` moving `base` to `target` positions."""
    if obj.data.shape_keys is None:
        obj.shape_key_add(name="Basis", from_mix=False)
    key = obj.shape_key_add(name=name, from_mix=False)
    for i, (a, b) in enumerate(zip(base, target)):
        if (b - a).length > 1e-6:
            key.data[i].co = b
    key.value = 0.0
    return key


def build(out_dir, texture_size=2048, bakes=True):
    reset_scene()
    body_obj, lm, frames, source = body.load()
    M, F = frames["Masculine"], frames["Feminine"]
    sets = face_sets(body_obj)
    layout = uvs.layout(body_obj, lm)
    maps = skin.bake(body_obj, source, lm, texture_size, out_dir) if bakes else None

    # Eyes and teeth, on the frames as they stand. The eyes follow the
    # feminine frame on their bones (the frame's bone offsets and eye
    # scale), the teeth by a shape; the mouth's bag is widened round them.
    set_positions(body_obj, M.P)
    eyes_m, eye_radius, eye_fit = parts.fit_eyes(body_obj, M.P, lm)
    eyes_f, eye_radius_f, eye_fit_f = parts.fit_eyes(body_obj, F.P, lm)
    eyes = parts.eyes(eyes_m, eye_radius, out_dir)
    height = max(p.z for p in M.P) - min(p.z for p in M.P)
    bundle = max(p.z for p in source["Masculine"]) - min(p.z for p in source["Masculine"])
    teeth, M.P, F.P, teeth_fit = parts.teeth(body_obj, M.P, F.P, lm, height / bundle)

    # The skeleton and the weights, on the masculine frame.
    P = M.P
    set_positions(body_obj, P)
    joints = rig.fit_joints(P, lm, eyes_m)
    joints_f = rig.fit_joints(F.P, lm, eyes_f)
    armature = rig.build_armature(joints, ARMATURE)
    weights = rig.skin(body_obj, armature, P, lm, joints)
    for obj in (body_obj, eyes, teeth):
        skin_to(obj, armature)

    # Materials (the face sets' are replaced), the frame shape, the body
    # regions (for clothing to hide skin) and the simpler copies.
    skin.materials(body_obj, sets, maps)
    shade_smooth(body_obj)
    add_shape(body_obj, "Frame_Feminine", P, F.P)
    # The body's muscle and fat shapes (CO5), on both frames.
    both = lambda m, f: {"Masculine": m, "Feminine": f}  # noqa: E731
    body_shapes, shape_report = shapes.build(
        body_obj, source, both(P, F.P), lm, both(joints, joints_f), weights, both(eyes_m, eyes_f),
        both(eye_radius, eye_radius_f))
    shapes.add_keys(body_obj, body_shapes, both(P, F.P))
    region_info, face_regions = regions.mark(body_obj, sets, P, joints, weights, out_dir)
    copies = lods.build(body_obj, armature, lm, P, joints, face_regions)
    for copy in copies:
        skin_to(copy, armature)

    bones = rig.joint_offsets(dict(joints, **rig.extra_joints(joints)),
                              dict(joints_f, **rig.extra_joints(joints_f)))
    data = {
        "version": 1,
        "units": "metres, glTF axes (Y up, front +Z)",
        "height": {"masculine": round(max(p.z for p in P), 4),
                   "feminine": round(max(p.z for p in F.P), 4)},
        "rest": rig.limb_angles(joints),
        "frame": {"bones": {b: {"offset": to_gltf(o)} for b, o in sorted(bones.items())},
                  "eye_scale": round(eye_radius_f / eye_radius, 4)},
        "added_bones": [{"name": n, "parent": p} for n, p in rig.added_bones()],
        "twists": [{"bone": n, "inside": i, "reads": r, "share": s} for n, i, r, s in rig.TWISTS],
        "helpers": [{"bone": n, "parent": p, "reads": r, "share": 0.5} for n, p, r in rig.HELPERS],
        "regions": region_info,
        "regions_map": {"file": regions.MAP_FILE, "size": regions.MAP_SIZE, "cells": regions.CELLS,
                        "layer": "Regions (TEXCOORD_1)"},
        "texture_coordinates": layout,
        "textures": maps["report"] if maps else {"size": 0},
        "eyes": {"masculine": eye_fit, "feminine": eye_fit_f},
        "teeth": teeth_fit,
        "body_shapes": shape_report,
    }
    objects = {"Body": body_obj, "Eyes": eyes, "Teeth": teeth}
    objects.update({c.name: c for c in copies})
    report = {name: tri_count(obj) for name, obj in objects.items()}
    return armature, objects, data, report


def export_glb(path, armature, objects):
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    for o in objects:
        o.select_set(True)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=True,
        # With the shapes' normals: the feminine frame changes the chest
        # and hips enough that the masculine normals would shade it wrong.
        # Sparse shape keys, as the stylized package's: the engine reads
        # Blender's empty sparse accessors as no moves.
        export_skins=True, export_morph=True, export_morph_normal=True,
        export_try_sparse_sk=True,
        export_animations=False, export_yup=True, export_def_bones=False,
        export_leaf_bone=False, export_tangents=False,
        export_image_format="AUTO")


def main(out_dir, texture_size=2048, bakes=True):
    out_dir = os.path.abspath(out_dir)
    os.makedirs(out_dir, exist_ok=True)
    armature, objects, data, report = build(out_dir, texture_size, bakes)
    for name, obj in objects.items():
        obj.name = name
        obj.data.name = name
    export_glb(os.path.join(out_dir, PARTS_FILE), armature, list(objects.values()))
    with open(os.path.join(out_dir, DATA_FILE), "w", newline="\n") as f:
        json.dump(data, f, indent=1, sort_keys=True)
        f.write("\n")
    manifests.write(out_dir)
    skin.clean_up(out_dir)
    iris = os.path.join(out_dir, "RealisticIris.png")
    if os.path.exists(iris):
        os.remove(iris)
    return report


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.dirname(os.path.abspath(__file__))
    size = int(argv[argv.index("--texture-size") + 1]) if "--texture-size" in argv else 2048
    rep = main(out, size, bakes="--skip-bakes" not in argv)
    for k in sorted(rep):
        print(f"{k:24s} {rep[k]:6d}")
