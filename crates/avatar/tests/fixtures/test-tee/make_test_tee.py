"""A test garment for the import flow (CO9): a short-sleeved T-shirt made
from simple solids, NOT from the body's surface.

The fitting tests import it as a third-party garment would be imported:
an OBJ in the realistic body's rest pose, roughly aligned. Only a
tailor's measurements are taken from the body (a girth every 2 cm, the
upper arm's girth along it), as a person modelling a shirt would take
them; the shirt itself is a loft of those girths with the ease (the
torso) and two tubes (the sleeves), joined by a voxel remesh, smoothed,
with the hem and sleeve ends cut by planes and a crew neck cut by a
cylinder round the neck. It is a **draft** by
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
SLEEVE_EASE = 0.014
# How far down the neck (from its joint) the crew neck's cylinder starts.
NECK_DROP = 0.08
# The voxel remesh's size: about 6k triangles for the shirt.
VOXEL = 0.019
ARM_PARTS = ("UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little")
# Below the armpit a slice of the body is the trunk in the middle and an
# arm either side, apart from it: a girth is the trunk's, out to the first
# gap this wide. (Skin of the arm weighted to the shoulder's helper bones,
# not the arm's, would otherwise widen a girth into a flange.)
GAP = 0.03
# How far past the trunk's side at the armpit a girth above it reaches
# (the arm's root, under the front and back of the shoulder).
ARM_ROOT = 0.05
# The sleeve's length, as a share of the upper arm.
SLEEVE = 0.55
COLOUR = (0.30, 0.38, 0.52, 1.0)


def body_data():
    """The body's rest positions (world), its arm weight per vertex, its
    left upper arm's weight (with the deltoid's helper), and the
    armature."""
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
            if "LeftUpperArm" in name or name == "LeftShoulderHelper":
                upper_arm[v.index] += g.weight
    return P, arm_weight, upper_arm, arm


def body_edges():
    me = bpy.data.objects["Body"].data
    edges = np.zeros(len(me.edges) * 2, dtype=np.int64)
    me.edges.foreach_get("vertices", edges)
    return edges.reshape(-1, 2)


def joint(arm, name):
    return np.array(arm.matrix_world @ arm.data.bones[name].head_local)


def section(P, edges, z, keep):
    """Where the body's edges cross the height `z` (among the edges whose
    both corners `keep` holds): its outline there."""
    a, b = P[edges[:, 0]], P[edges[:, 1]]
    crossing = ((a[:, 2] - z) * (b[:, 2] - z) <= 0) & (a[:, 2] != b[:, 2])
    crossing &= keep[edges[:, 0]] & keep[edges[:, 1]]
    a, b = a[crossing], b[crossing]
    t = (z - a[:, 2]) / (b[:, 2] - a[:, 2])
    return a + (b - a) * t[:, None]


def trunk(points):
    """The trunk's part of an outline: out from the middle to the first gap
    of `GAP` across (an arm, apart from it, is beyond)."""
    xs = np.sort(np.abs(points[:, 0]))
    reach = xs[0]
    for x in xs[1:]:
        if x - reach > GAP:
            break
        reach = x
    return points[np.abs(points[:, 0]) <= reach + 1e-9]


def hull(points):
    """The convex hull of 2D `points`, counter-clockwise (Andrew's monotone
    chain)."""
    pts = sorted(set(map(tuple, np.round(points, 6))))

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return np.array(lower[:-1] + upper[:-1])


def girth_ring(outline, z, ease, count=48):
    """A girth as a tape takes it: round the outline's convex hull (made
    the same on both sides), `ease` out from it, as `count` points round
    its middle."""
    xy = outline[:, :2]
    xy = np.vstack([xy, xy * np.array([-1.0, 1.0])])
    poly = hull(xy)
    centre = np.array([0.0, poly[:, 1].mean()])
    edges = [(poly[k], poly[(k + 1) % len(poly)]) for k in range(len(poly))]
    ring = []
    for t in (2 * math.pi * k / count for k in range(count)):
        d = np.array([math.cos(t), math.sin(t)])
        best = None
        for a, b in edges:
            e = b - a
            # centre + d * r = a + e * u
            m = np.array([[d[0], -e[0]], [d[1], -e[1]]])
            if abs(np.linalg.det(m)) < 1e-12:
                continue
            r, u = np.linalg.solve(m, a - centre)
            if r > 0 and -1e-9 <= u <= 1 + 1e-9 and (best is None or r < best[0]):
                normal = np.array([e[1], -e[0]]) / np.linalg.norm(e)
                best = (r, normal)
        r, normal = best
        # Out along `d` far enough to be `ease` from the hull's side.
        r += ease / max(abs(d @ normal), 0.3)
        ring.append((centre[0] + d[0] * r, centre[1] + d[1] * r, z))
    return ring


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


def sleeve(start, axis, steps, radii, count=32):
    """A tube along `axis` from `start`: a ring of radius `radii[k]` at
    `steps[k]` along."""
    side = np.cross(axis, [0.0, 0.0, 1.0])
    side /= np.linalg.norm(side)
    up = np.cross(side, axis)
    rings = []
    for t, r in zip(steps, radii):
        centre = start + axis * t
        rings.append([tuple(centre + r * (math.cos(a) * side + math.sin(a) * up))
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

    # The torso: a girth every 2 cm from the hem to the neck. Below the
    # armpit a girth is the trunk's (the arms apart from it left out);
    # above it, the body's outline no wider than the trunk at the armpit
    # and the arm's root (the shoulder past that is the sleeve's).
    edges = body_edges()
    not_arm = arm_weight < 0.5
    # The armpit: the highest the arm is still apart from the trunk
    # (from the elbow up, where the upper arm is beside it).
    armpit = elbow_l[2]
    while True:
        z = armpit + 0.005
        outline = section(P, edges, z, not_arm)
        if len(trunk(outline)) == len(outline):
            break
        armpit = z
    side = np.abs(trunk(section(P, edges, armpit, not_arm))[:, 0]).max()
    print(f"armpit at {armpit:.3f} m (the shoulder joint at {shoulder_l[2]:.3f} m), "
          f"the trunk {side:.3f} m wide there")
    # No girth is wider than this: one past it has caught an arm (and the
    # trunk at the armpit is no wider than the shoulder joint and a hand).
    widest = side + ARM_ROOT
    assert side < abs(shoulder_l[0]) + 0.05, f"the trunk at the armpit is {side:.3f} m wide"
    # Up the neck (the neck's cylinder cuts the opening).
    top = neck[2] + 0.04
    rings = []
    for z in np.arange(hem, top + 1e-6, 0.02):
        if z <= armpit:
            s = trunk(section(P, edges, z, not_arm))
        else:
            s = section(P, edges, z, not_arm)
            s = s[np.abs(s[:, 0]) <= side + ARM_ROOT]
        ease = NECK_EASE if z > top - 0.03 else EASE
        ring = girth_ring(s, z, ease)
        a = max(abs(p[0]) for p in ring)
        # A flange would be wider.
        assert a <= widest + EASE + 1e-6, f"the girth at {z:.3f} m is {a:.3f} m wide"
        rings.append(ring)
    parts = [solid(rings)]

    # The sleeves: round the upper arm from the shoulder joint to a little
    # past half way to the elbow, measured on the left arm (mirrored): at
    # each step the arm's girth there (the deltoid's near the shoulder)
    # and the ease.
    length = np.linalg.norm(elbow_l - shoulder_l)
    axis_l = (elbow_l - shoulder_l) / length
    rel = P[upper_arm > 0.5] - shoulder_l
    along = rel @ axis_l
    across = np.linalg.norm(rel - np.outer(along, axis_l), axis=1)
    steps = np.linspace(0.0, SLEEVE * length, 8)
    radii = [np.percentile(across[np.abs(along - t) < 0.015], 80) + SLEEVE_EASE for t in steps]
    # No wider at the shoulder than a little over the sleeve's middle: a
    # wider start stands up off the shoulder's line.
    radii = [min(r, 1.12 * radii[len(radii) // 2]) for r in radii]
    radius = max(radii)
    print("sleeve radii " + " ".join(f"{r:.3f}" for r in radii))
    for sign in (1.0, -1.0):
        s = shoulder_l * np.array([sign, 1.0, 1.0])
        axis = axis_l * np.array([sign, 1.0, 1.0])
        parts.append(sleeve(s, axis, steps, radii))

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

    # The neck: a crew neck, cut by an upright elliptic cylinder round the
    # neck (from below its base up past the head), a little wider than the
    # shell's neck: the opening is where it meets the shoulders' slope,
    # lower in front. The cylinder's faces (a pit into the closed shell)
    # are deleted after, which leaves the opening.
    outline = section(P, edges, neck[2] + 0.02, arm_weight < 0.1)
    neck_a = np.abs(outline[:, 0]).max() + NECK_EASE + 0.008
    neck_cy = (outline[:, 1].max() + outline[:, 1].min()) / 2
    neck_b = (outline[:, 1].max() - outline[:, 1].min()) / 2 + NECK_EASE + 0.008
    up = Vector((0.0, 0.0, 1.0))
    base = Vector((0.0, neck_cy, neck[2])) - up * NECK_DROP
    bpy.ops.mesh.primitive_cylinder_add(vertices=96, radius=1.0, depth=1.0)
    cutter = bpy.context.active_object
    cutter.scale = (neck_a, neck_b, 0.5)
    cutter.rotation_mode = "QUATERNION"
    cutter.rotation_quaternion = up.to_track_quat("Z", "Y")
    cutter.location = base + up * 0.25
    shell = bpy.data.materials.new("Shell")
    hole = bpy.data.materials.new("Hole")
    tee.data.materials.append(shell)
    cutter.data.materials.append(hole)
    boolean = tee.modifiers.new("neck", "BOOLEAN")
    boolean.operation = "DIFFERENCE"
    boolean.solver = "EXACT"
    boolean.material_mode = "TRANSFER"
    boolean.object = cutter
    bpy.context.view_layer.objects.active = tee
    bpy.ops.object.modifier_apply(modifier="neck")
    bpy.data.objects.remove(cutter)
    hole_slot = [m.name for m in tee.data.materials].index(hole.name)

    # Open the hem and the sleeve ends with straight cuts: a
    # plane through the triangles, so each edge is a clean line rather
    # than the remesh's stair-steps.
    import bmesh
    me = tee.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.material_index == hole_slot], context="FACES")

    def cut(co, no, chosen=lambda c: True):
        """Cuts the faces `chosen` picks (by their centres) along the plane
        at `co` with normal `no`, and deletes what is on the normal's side."""
        faces = [f for f in bm.faces if chosen(np.array(f.calc_center_median()))]
        edges = list({e for f in faces for e in f.edges})
        verts = list({v for f in faces for v in f.verts})
        bmesh.ops.bisect_plane(bm, geom=verts + edges + faces, dist=1e-6,
                               plane_co=tuple(co), plane_no=tuple(no), clear_outer=True)

    # Each a little inside the solid's end, past where the remesh rounded
    # its cap.
    cut((0.0, 0.0, hem + 0.02), (0.0, 0.0, -1.0))
    for sign in (1.0, -1.0):
        s = shoulder_l * np.array([sign, 1.0, 1.0])
        axis = axis_l * np.array([sign, 1.0, 1.0])

        def on_sleeve(c, s=s, axis=axis):
            # The sleeve, past the torso's side (where they meet under the
            # arm the torso is left whole).
            along = (c - s) @ axis
            off_axis = np.linalg.norm((c - s) - along * axis)
            return along > 0.3 * length and off_axis < 1.6 * radius and abs(c[0]) > side + EASE

        cut(s + axis * (SLEEVE * length - 0.02), axis, on_sleeve)
    # Keep the largest piece (a cut can leave a stray ring).
    bm.faces.ensure_lookup_table()
    bm.faces.index_update()
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
