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
bones, weight the body, add `Frame_Feminine`, the eyes and the teeth, cut
the underwear from the body (`underwear.py`), mark the body regions,
make the simpler copies (`Body_LOD1`, `Body_LOD2`) and export
`RealisticParts.glb`, then the base model manifest
(`manifests.py`; afterwards rebuild the library's catalogue with
`../catalogue.py`). Units are metres, Blender Z up, the character faces -Y
(glTF +Z after export).
"""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

import body  # noqa: E402
import correctives  # noqa: E402
import eyes as eyelib  # noqa: E402
import face  # noqa: E402
import lods  # noqa: E402
import makeup  # noqa: E402
import manifests  # noqa: E402
import parts  # noqa: E402
import regions  # noqa: E402
import rig  # noqa: E402
import rims  # noqa: E402
import shapes  # noqa: E402
import skin  # noqa: E402
import teeth as jawlib  # noqa: E402
import underwear  # noqa: E402
import uvs  # noqa: E402
from common import array, duplicate, face_sets, reset_scene, set_positions, shade_smooth, tri_count  # noqa: E402

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

    # Eyes and teeth, on the frames as they stand. The eyes follow the
    # feminine frame on their bones (the frame's bone offsets and eye
    # scale), the teeth by a shape; the mouth's bag is widened round them.
    # (The eyes are fitted to the bundle's own sockets, before the lids
    # get their new vertices.)
    set_positions(body_obj, M.P)
    eyes_m, eye_radius, eye_fit = parts.fit_eyes(body_obj, M.P, lm)
    eyes_f, eye_radius_f, eye_fit_f = parts.fit_eyes(body_obj, F.P, lm)
    # Twice the vertices round the lids (`rims.py`). The bundle's vertices
    # keep their indices; the new ones follow in every set of positions,
    # and whatever is kept by polygon is read again. The sculpts are baked
    # from the bundle's own topology (`cage`).
    cage = duplicate(body_obj, "Cage") if bakes else None
    fine = rims.split(body_obj, sets)
    sets = face_sets(body_obj)
    lm = rig.stylized_rig().Landmarks(body_obj)
    source = {name: fine.grow_points(P) for name, P in source.items()}
    M.P, F.P = fine.grow_points(M.P), fine.grow_points(F.P)
    set_positions(body_obj, source["Masculine"])
    maps = skin.bake(body_obj, source, lm, texture_size, out_dir, cage) if bakes else None
    if cage is not None:
        bpy.data.objects.remove(cage)
    set_positions(body_obj, M.P)
    # The face put right at rest (the feminine lids, the lids' lining, the
    # lips; `face.prepare`), before the teeth are placed and the body weighted.
    topo = face.topo_of(body_obj.data, sets)
    # (Each vertex's mirror vertex: the mouth is made even by it.)
    mirror = fine.mirror(shapes._transfer()[0].mirror_map(
        [array(source["Masculine"])[:fine.count], array(source["Feminine"])[:fine.count]]))
    m_face, f_face, face_rest = face.prepare(topo, array(M.P), array(F.P), eyes_m, eyes_f, eye_radius, eye_radius_f,
                                             mirror)
    M.P, F.P = [Vector(p) for p in m_face], [Vector(p) for p in f_face]
    if maps:
        face_rest["lid_texels_softened"] = face.soften_lids(body_obj, topo, m_face, eyes_m, eye_radius, maps)
        # The makeup and marking masks (R5, `makeup.py`), measured on the
        # face at rest; the lips are the albedo's own lip field.
        lips = skin.fields(body_obj, source["Masculine"], lm)["lips"]
        maps["report"]["makeup"] = makeup.masks(body_obj, topo, m_face, M.P, lm, eyes_m, eye_radius, lips,
                                                texture_size, out_dir)
    eyes = eyelib.eyes(eyes_m, eye_radius, parts.iris_texture(out_dir), out_dir)
    height = max(p.z for p in M.P) - min(p.z for p in M.P)
    bundle = max(p.z for p in source["Masculine"]) - min(p.z for p in source["Masculine"])
    teeth, teeth_copies, M.P, F.P, teeth_fit = parts.teeth(body_obj, M.P, F.P, lm, height / bundle, out_dir)
    # (Room for the jaw is made vertex by vertex: the mouth even again.)
    mouth_share = face.mouth_region(topo, (array(M.P), array(F.P)), mirror)
    M.P = [Vector(p) for p in face.even(array(M.P), mirror, mouth_share)]
    F.P = [Vector(p) for p in face.even(array(F.P), mirror, mouth_share)]

    # The skeleton and the weights, on the masculine frame.
    P = M.P
    set_positions(body_obj, P)
    joints = rig.fit_joints(P, lm, eyes_m)
    joints_f = rig.fit_joints(F.P, lm, eyes_f)
    armature = rig.build_armature(joints, ARMATURE)
    weights = rig.skin(body_obj, armature, P, lm, joints, even=(mirror, mouth_share))
    for obj in (body_obj, eyes, teeth, *teeth_copies):
        skin_to(obj, armature)

    # Materials (the face sets' are replaced), the frame shape, the body
    # regions (for clothing to hide skin) and the simpler copies.
    skin.materials(body_obj, sets, maps)
    shade_smooth(body_obj)
    add_shape(body_obj, "Frame_Feminine", P, F.P)
    # The body's muscle and fat shapes (CO5), on both frames.
    both = lambda m, f: {"Masculine": m, "Feminine": f}  # noqa: E731
    body_shapes, shape_report, mesh = shapes.build(
        body_obj, source, both(P, F.P), lm, both(joints, joints_f), weights, both(eyes_m, eyes_f),
        both(eye_radius, eye_radius_f), mirror=mirror, grow=fine.grow)
    shapes.add_keys(body_obj, body_shapes, both(P, F.P))
    # The shapes the engine leans on as the arms are raised (CO5).
    driven = {"twists": [{"bone": n, "inside": i, "reads": r, "share": s} for n, i, r, s in rig.TWISTS],
              "helpers": [{"bone": n, "parent": p, "reads": r, "share": 0.5} for n, p, r in rig.HELPERS]}
    corrective_shapes, corrective_rules, corrective_report = correctives.build(
        body_obj, armature, weights, P, driven, mesh["mirror"], mesh["lap"], mesh["tris"])
    correctives.add_keys(body_obj, corrective_shapes, P)
    # The face: its expression and identity shapes, and the parts that
    # follow the lids and brows (before the simpler copies, which take
    # the body's shapes).
    face_parts, face_data = face.build(body_obj, topo, array(P), array(F.P), eyes_m, eyes_f, eye_radius,
                                       joints, joints_f, weights, teeth, out_dir, teeth_copies, mirror)
    for obj in face_parts.values():
        skin_to(obj, armature)
    # Underwear cut from the body's surface, with its weights and shapes
    # (`underwear.py`; the items are written by `manifests.py`).
    garments, garment_report = underwear.make(body_obj, armature, joints, joints_f)
    for obj in garments.values():
        skin_to(obj, armature)
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
        "correctives": corrective_rules,
        "corrective_shapes": corrective_report,
        "face": dict(face_data, rest=face_rest),
        "underwear": garment_report,
    }
    objects = {"Body": body_obj, "Eyes": eyes, "Teeth": teeth}
    objects.update(face_parts)
    objects.update(garments)
    objects.update({c.name: c for c in (*copies, *teeth_copies)})
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
    # The skin marked skin, with its defined normal map (R5).
    skin.mark(os.path.join(out_dir, PARTS_FILE), out_dir)
    # And its makeup and marking masks (R5, ADR-0042).
    makeup.mark(os.path.join(out_dir, PARTS_FILE), out_dir)
    with open(os.path.join(out_dir, DATA_FILE), "w", newline="\n") as f:
        json.dump(data, f, indent=1, sort_keys=True)
        f.write("\n")
    manifests.write(out_dir)
    skin.clean_up(out_dir)
    makeup.clean_up(out_dir)
    for image in ("RealisticIris.png", *face.IMAGES, *jawlib.IMAGES):
        if os.path.exists(os.path.join(out_dir, image)):
            os.remove(os.path.join(out_dir, image))
    return report


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.dirname(os.path.abspath(__file__))
    size = int(argv[argv.index("--texture-size") + 1]) if "--texture-size" in argv else 2048
    rep = main(out, size, bakes="--skip-bakes" not in argv)
    for k in sorted(rep):
        print(f"{k:24s} {rep[k]:6d}")
