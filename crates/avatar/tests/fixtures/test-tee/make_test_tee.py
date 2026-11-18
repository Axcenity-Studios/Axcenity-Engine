"""A test garment for the import flow (CO9): a short-sleeved T-shirt made
from simple solids, NOT from the body's surface.

The fitting tests import it as a third-party garment would be imported:
an OBJ in the realistic body's rest pose, roughly aligned. Only a
tailor's measurements are taken from the body (girths at a few heights,
the shoulder, neck and upper arm), as a person modelling a shirt would
take them; the shirt itself is a stack of ellipses (the torso) and two
cylinders (the sleeves), joined by a voxel remesh, smoothed, with the hem,
neck and sleeve ends cut open. It is a **draft** by
`docs/development/character-studio/asset-quality.md` (one layer, no
construction, seams or folds): a fixture, not a wardrobe item.

Made by the Axcenity Engine contributors and dedicated to the public
domain (CC0 1.0).

Run with Blender's Python (bpy 5.x):

    python -I crates/avatar/tests/fixtures/test-tee/make_test_tee.py \
        --body assets/characters/realistic/RealisticParts.glb \
        --out crates/avatar/tests/fixtures/test-tee

Blender axes: Z up, the character faces -Y. The OBJ is written Y up, the
character facing +Z (glTF's axes).
"""

import argparse
import math
import os

import bpy
import numpy as np
from mathutils import Vector

# How far the shirt stands off the body (a loose fit), and at the neck.
EASE = 0.022
NECK_EASE = 0.018
# The voxel remesh's size: about 6k triangles for the shirt.
VOXEL = 0.019
ARM_PARTS = ("UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little")
COLOUR = (0.30, 0.38, 0.52, 1.0)


def body_data():
    """The body's rest positions (world), its arm weight per vertex, and
    the armature."""
    body = bpy.data.objects["Body"]
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    me = body.data
    n = len(me.vertices)
    co = np.zeros(n * 3)
    me.vertices.foreach_get("co", co)
    co = co.reshape(n, 3)
    world = np.array(body.matrix_world)
    P = co @ world[:3, :3].T + world[:3, 3]
    groups = {g.index: g.name for g in body.vertex_groups}
    arm_weight = np.zeros(n)
    upper_arm = np.zeros(n)
    for v in me.vertices:
        for g in v.groups:
            name = groups[g.group]
            if any(part in name for part in ARM_PARTS):
                arm_weight[v.index] += g.weight
            if "LeftUpperArm" in name:
                upper_arm[v.index] += g.weight
    return P, arm_weight, upper_arm, arm


def joint(arm, name):
    return np.array(arm.matrix_world @ arm.data.bones[name].head_local)


def ellipse_ring(cx, cy, z, a, b, count=48):
    return [(cx + a * math.cos(t), cy + b * math.sin(t), z)
            for t in (2 * math.pi * k / count for k in range(count))]


def solid(rings):
    """A closed mesh: the rings joined side by side, capped at both ends."""
    count = len(rings[0])
    verts = [p for ring in rings for p in ring]
    faces = []
    for r in range(len(rings) - 1):
        for k in range(count):
            a, b = r * count + k, r * count + (k + 1) % count
            faces.append((a, b, b + count, a + count))
    faces.append(tuple(reversed(range(count))))
    last = (len(rings) - 1) * count
    faces.append(tuple(range(last, last + count)))
    me = bpy.data.meshes.new("part")
    me.from_pydata(verts, [], faces)
    me.update()
    me.validate()
    obj = bpy.data.objects.new("part", me)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def cylinder(start, end, radius, count=32):
    axis = (end - start) / np.linalg.norm(end - start)
    side = np.cross(axis, [0.0, 0.0, 1.0])
    side /= np.linalg.norm(side)
    up = np.cross(side, axis)
    rings = []
    for t in np.linspace(0.0, 1.0, 8):
        centre = start + (end - start) * t
        rings.append([tuple(centre + radius * (math.cos(a) * side + math.sin(a) * up))
                      for a in (2 * math.pi * k / count for k in range(count))])
    return solid(rings)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--body", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.body))
    P, arm_weight, upper_arm, arm = body_data()

    hips = joint(arm, "Hips")
    shoulder_l = joint(arm, "LeftUpperArm")
    elbow_l = joint(arm, "LeftLowerArm")
    neck = joint(arm, "Neck")
    hem = hips[2] - 0.10
    armpit = shoulder_l[2] - 0.11
    shoulder_top = shoulder_l[2] + 0.045
    # The neck's girth just above its base.
    ring = P[(np.abs(P[:, 2] - (neck[2] + 0.02)) < 0.01) & (arm_weight < 0.1)]
    neck_a = np.abs(ring[:, 0]).max() + NECK_EASE
    neck_b = (ring[:, 1].max() - ring[:, 1].min()) / 2 + NECK_EASE
    neck_cy = (ring[:, 1].max() + ring[:, 1].min()) / 2

    # The torso: a girth every 2 cm from the hem to the armpit (the arms
    # left out), then out to the shoulders and in to the neck.
    rings = []
    last = None
    for z in np.arange(hem, armpit + 1e-6, 0.02):
        s = P[(np.abs(P[:, 2] - z) < 0.012) & (arm_weight < 0.2)]
        a = np.abs(s[:, 0]).max() + EASE
        cy = (s[:, 1].max() + s[:, 1].min()) / 2
        b = (s[:, 1].max() - s[:, 1].min()) / 2 + EASE
        last = (cy, a, b)
        rings.append(ellipse_ring(0.0, cy, z, a, b))
    cy, a, b = last
    shoulder_a = abs(shoulder_l[0]) + EASE
    for z in np.linspace(armpit + 0.02, shoulder_top, 4):
        t = (z - armpit) / (shoulder_top - armpit)
        rings.append(ellipse_ring(0.0, cy, z, a + (shoulder_a - a) * t, b))
    top = neck[2] + 0.015
    for t in np.linspace(0.25, 1.0, 4):
        z = shoulder_top + (top - shoulder_top) * t
        rings.append(ellipse_ring(0.0, cy + (neck_cy - cy) * t, z,
                                  shoulder_a + (neck_a - shoulder_a) * t ** 0.7,
                                  b + (neck_b - b) * t))
    parts = [solid(rings)]

    # The sleeves: round the upper arm, from inside the shoulder to about
    # half way to the elbow (measured on the left arm, mirrored).
    length = np.linalg.norm(elbow_l - shoulder_l)
    axis_l = (elbow_l - shoulder_l) / length
    rel = P[upper_arm > 0.5] - shoulder_l
    along = rel @ axis_l
    across = np.linalg.norm(rel - np.outer(along, axis_l), axis=1)
    radius = np.percentile(across[(along > 0.2 * length) & (along < 0.5 * length)], 50) + 0.014
    print(f"upper arm: {length:.3f} m long, sleeve radius {radius:.3f} m")
    for sign in (1.0, -1.0):
        s = shoulder_l * np.array([sign, 1.0, 1.0])
        axis = axis_l * np.array([sign, 1.0, 1.0])
        parts.append(cylinder(s - axis * 0.02, s + axis * 0.48 * length, radius))

    # One shell: the solids' union, smoothed.
    for obj in bpy.data.objects:
        obj.select_set(obj in parts)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    tee = parts[0]
    tee.name = "TestTee"
    tee.data.name = "TestTee"
    remesh = tee.modifiers.new("remesh", "REMESH")
    remesh.mode = "VOXEL"
    remesh.voxel_size = VOXEL
    smooth = tee.modifiers.new("smooth", "SMOOTH")
    smooth.factor = 0.5
    smooth.iterations = 4
    bpy.ops.object.modifier_apply(modifier="remesh")
    bpy.ops.object.modifier_apply(modifier="smooth")

    # Open the hem, the neck and the sleeve ends.
    me = tee.data
    cut = set()
    for poly in me.polygons:
        c = np.array(poly.center)
        if c[2] < hem + 0.012:
            cut.add(poly.index)
        elif c[2] > top - 0.012 and abs(c[0]) < neck_a + 0.01:
            cut.add(poly.index)
        for sign in (1.0, -1.0):
            s = shoulder_l * np.array([sign, 1.0, 1.0])
            axis = axis_l * np.array([sign, 1.0, 1.0])
            along = (c - s) @ axis
            off_axis = np.linalg.norm((c - s) - along * axis)
            # Past the sleeve's end, and on the sleeve (not the torso).
            if along > 0.48 * length - 0.012 and off_axis < 1.5 * radius:
                cut.add(poly.index)
    import bmesh
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    bmesh.ops.delete(bm, geom=[bm.faces[i] for i in sorted(cut)], context="FACES")
    # Keep the largest piece (a cut can leave a stray ring).
    bm.faces.ensure_lookup_table()
    seen = set()
    pieces = []
    for f in bm.faces:
        if f.index in seen:
            continue
        stack, piece = [f], []
        seen.add(f.index)
        while stack:
            g = stack.pop()
            piece.append(g)
            for e in g.edges:
                for h in e.link_faces:
                    if h.index not in seen:
                        seen.add(h.index)
                        stack.append(h)
        pieces.append(piece)
    keep = max(pieces, key=len)
    bmesh.ops.delete(bm, geom=[f for piece in pieces if piece is not keep for f in piece], context="FACES")
    bm.to_mesh(me)
    bm.free()
    me.update()
    for poly in me.polygons:
        poly.use_smooth = True

    mat = bpy.data.materials.new("TeeCotton")
    mat.diffuse_color = COLOUR
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = COLOUR
    bsdf.inputs["Roughness"].default_value = 0.85
    me.materials.clear()
    me.materials.append(mat)

    # Texture coordinates, for completeness.
    for obj in bpy.data.objects:
        obj.select_set(obj == tee)
    bpy.context.view_layer.objects.active = tee
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(island_margin=0.01)
    bpy.ops.object.mode_set(mode="OBJECT")

    triangles = sum(len(p.vertices) - 2 for p in me.polygons)
    print(f"TestTee: {len(me.vertices)} vertices, {triangles} triangles")
    os.makedirs(args.out, exist_ok=True)
    bpy.ops.wm.obj_export(filepath=os.path.join(os.path.abspath(args.out), "TestTee.obj"),
                          export_selected_objects=True, forward_axis="NEGATIVE_Z", up_axis="Y",
                          export_materials=True, export_normals=True, export_uv=True,
                          apply_modifiers=True, path_mode="STRIP")


if __name__ == "__main__":
    main()
