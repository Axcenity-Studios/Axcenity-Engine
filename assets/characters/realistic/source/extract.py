"""Extracts what the realistic base body is built on from Blender Studio's
Human Base Meshes bundle (CC0 1.0) into plain files here.

Only needed to refresh the vendored files; `build.py` reads these files.

    python -I extract.py path/to/human_base_meshes_bundle.blend

The bundle: https://download.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip
(v1.4.1, SHA-256 811f43accbb31a88266d932f8f5563b2d13586fca0ba2693aad1f5fe582b3515).

What it writes:

- `body_masculine.obj`, `body_feminine.obj`: the realistic male and female
  bodies' base meshes (10,582 vertices, one topology: vertex `i` of one is
  vertex `i` of the other, checked here; the files list their faces in
  different orders). The base vertices lie on the bundle's sculpt (its
  multiresolution level 3). Sculpt face sets are written as OBJ materials
  `fs_<id>`; the two bodies' sets differ on a few faces, so the build
  reads the masculine one's.
- `sculpt_masculine.npz`, `sculpt_feminine.npz`: the sculpt itself, as
  what it adds to the vendored base mesh subdivided three times
  (`subdivide`, below): `displacement`, one vector per subdivided vertex
  (in the order the vendored OBJ file subdivides in), in metres.
  The build rebuilds the sculpt from them and bakes it into the normal
  map. `check_index` and `check_position` let the build confirm its
  Blender subdivides in the same vertex order.
- `eyes.obj`: the realistic animation head's left eye, about its centre:
  the eyeball with its cornea bulge (`sclera`) and the iris disc behind
  the cornea (`iris`), as two objects.
- `teeth.obj`: the realistic jaw (teeth, gums, tongue), face sets as
  materials.
- `head_animation.obj`: the realistic animation head, kept for reference
  (the body's own head is used; see the package README).
"""

import os
import sys

import bpy
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))

BODIES = {
    "GEO-body_male_realistic": "masculine",
    "GEO-body_female_realistic": "feminine",
}
EYE = ("GEO-head_animation_realistic.sclera.L", "GEO-head_animation_realistic.iris.L")
TEETH = "Jaw - Realistic"
HEAD = "GEO-head_animation_realistic"
SCULPT_LEVEL = 3


def subdivide(obj, levels=SCULPT_LEVEL):
    """A Catmull-Clark subdivision modifier with the settings the sculpt's
    displacement is stored against. `build.py` (`skin.py`) uses the same."""
    mod = obj.modifiers.new("Subdivide", "SUBSURF")
    mod.subdivision_type = "CATMULL_CLARK"
    mod.levels = levels
    mod.render_levels = levels
    mod.quality = 3
    mod.use_limit_surface = True
    mod.boundary_smooth = "ALL"
    mod.uv_smooth = "PRESERVE_BOUNDARIES"
    return mod


def evaluated_positions(obj):
    deps = bpy.context.evaluated_depsgraph_get()
    mesh = obj.evaluated_get(deps).to_mesh()
    co = np.empty(len(mesh.vertices) * 3, dtype=np.float64)
    mesh.vertices.foreach_get("co", co)
    return co.reshape(-1, 3).copy()


def face_set_materials(obj):
    me = obj.data
    sets = [d.value for d in me.attributes[".sculpt_face_set"].data]
    ids = sorted(set(sets))
    me.materials.clear()
    for i in ids:
        me.materials.append(bpy.data.materials.get(f"fs_{i}") or bpy.data.materials.new(f"fs_{i}"))
    for poly, s in zip(me.polygons, sets):
        poly.material_index = ids.index(s)


def faces_of(mesh):
    """The mesh's faces as a set, each from its lowest vertex index round."""
    faces = set()
    for poly in mesh.polygons:
        verts = tuple(poly.vertices)
        first = verts.index(min(verts))
        faces.add(verts[first:] + verts[:first])
    return faces


def export(objects, name):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objects:
        o.hide_set(False)
        o.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.wm.obj_export(
        filepath=os.path.join(HERE, name), export_selected_objects=True,
        export_materials=True, export_uv=True, export_normals=False,
        export_triangulated_mesh=False, apply_modifiers=False,
        export_object_groups=True, path_mode="STRIP")


def show_everything():
    def include(layer):
        layer.exclude = False
        layer.hide_viewport = False
        for child in layer.children:
            include(child)
    include(bpy.context.view_layer.layer_collection)
    for collection in bpy.data.collections:
        collection.hide_viewport = False
        collection.hide_select = False
    for obj in bpy.data.objects:
        obj.hide_viewport = False
        obj.hide_select = False


def body(name, frame):
    obj = bpy.data.objects[name]
    obj.location = (0, 0, 0)
    me = obj.data
    count = len(me.vertices)
    multires = obj.modifiers["Multires"]
    multires.levels = SCULPT_LEVEL
    bpy.context.view_layer.update()
    sculpt = evaluated_positions(obj)
    cage = np.array([v.co[:] for v in me.vertices])
    # A subdivided mesh lists the base mesh's vertices first: check, by how
    # far each moved and by the lengths of the base mesh's edges.
    moved = np.linalg.norm(sculpt[:count] - cage, axis=1)
    edges = np.array([e.vertices[:] for e in me.edges])
    before = np.linalg.norm(cage[edges[:, 0]] - cage[edges[:, 1]], axis=1)
    after = np.linalg.norm(sculpt[edges[:, 0]] - sculpt[edges[:, 1]], axis=1)
    ratio = after / np.maximum(before, 1e-9)
    assert moved.max() < 0.02 and 0.5 < np.median(ratio) < 1.5 and ratio.max() < 4.0, \
        f"{name}: the subdivided mesh doesn't list the base vertices first"
    obj.modifiers.remove(multires)
    for vert, p in zip(me.vertices, sculpt[:count]):
        vert.co = p
    me.update()
    mod = subdivide(obj)
    bpy.context.view_layer.update()
    smooth = evaluated_positions(obj)
    obj.modifiers.remove(mod)
    assert len(smooth) == len(sculpt)
    face_set_materials(obj)
    export([obj], f"body_{frame}.obj")
    # A subdivided mesh lists its new vertices by the base mesh's edges and
    # faces, and the OBJ file doesn't keep their order. The build makes
    # both frames from the masculine file's faces (the feminine one gives
    # positions only), so the displacement is stored in the order that
    # mesh subdivides in: read the masculine file back, give it this
    # frame's positions, subdivide it, and match the two by position.
    before = set(bpy.data.objects)
    bpy.ops.wm.obj_import(filepath=os.path.join(HERE, "body_masculine.obj"))
    (again,) = [o for o in bpy.data.objects if o not in before]
    again.data.transform(again.matrix_world)
    again.matrix_world.identity()
    for vert, point in zip(again.data.vertices, sculpt[:count]):
        vert.co = point
    again.data.update()
    mod = subdivide(again)
    bpy.context.view_layer.update()
    stored = evaluated_positions(again)
    bpy.data.objects.remove(again)
    from mathutils.kdtree import KDTree
    tree = KDTree(len(smooth))
    for i, point in enumerate(smooth):
        tree.insert(point, i)
    tree.balance()
    match = np.empty(len(stored), dtype=np.int64)
    worst = 0.0
    for i, point in enumerate(stored):
        _, j, d = tree.find(point)
        match[i] = j
        worst = max(worst, d)
    assert worst < 2e-5 and len(set(match.tolist())) == len(match),         f"{name}: the vendored file doesn't subdivide to the bundle's mesh ({worst})"
    displacement = (sculpt - smooth)[match]
    smooth = stored
    pick = np.linspace(0, len(smooth) - 1, 512).astype(np.int64)
    np.savez_compressed(
        os.path.join(HERE, f"sculpt_{frame}.npz"),
        displacement=displacement.astype(np.float16),
        check_index=pick, check_position=smooth[pick].astype(np.float32),
        levels=np.int64(SCULPT_LEVEL))
    size = np.linalg.norm(displacement, axis=1) * 1000
    print(f"{frame}: {count} vertices, {len(me.polygons)} faces, sculpt {len(sculpt)} vertices; "
          f"base moved onto the sculpt by up to {moved.max() * 1000:.1f} mm; "
          f"displacement median {np.median(size):.2f} mm, largest {size.max():.1f} mm")
    return obj


def main(blend):
    bpy.ops.wm.open_mainfile(filepath=blend)
    show_everything()
    for obj in bpy.data.objects:
        obj.animation_data_clear()
    made = [body(name, frame) for name, frame in BODIES.items()]
    a, b = (o.data for o in made)
    # The same faces over the same vertex indices (the files list them in
    # different orders, so the build takes faces, face sets and texture
    # coordinates from the masculine body and only positions from the other).
    assert faces_of(a) == faces_of(b), "the two bodies don't share one topology"
    print("one topology:", len(a.vertices), "vertices,",
          sum(len(p.vertices) - 2 for p in a.polygons), "triangles")

    # The eye, about the eyeball's centre, at its size in the bundle's head.
    sclera, iris = (bpy.data.objects[n] for n in EYE)
    centre = sclera.matrix_world.to_translation()
    for obj, name in ((sclera, "sclera"), (iris, "iris")):
        obj.data = obj.data.copy()
        world = obj.matrix_world.copy()
        obj.parent = None
        obj.matrix_world = world
        obj.data.transform(obj.matrix_world)
        obj.matrix_world.identity()
        for vert in obj.data.vertices:
            vert.co -= centre
        obj.data.materials.clear()
        obj.name = name
        obj.data.name = name
    export([sclera, iris], "eyes.obj")

    teeth = bpy.data.objects[TEETH]
    teeth.location = (0, 0, 0)
    face_set_materials(teeth)
    export([teeth], "teeth.obj")

    head = bpy.data.objects[HEAD]
    for mod in list(head.modifiers):
        head.modifiers.remove(mod)
    head.data.transform(head.matrix_world)
    head.matrix_world.identity()
    face_set_materials(head)
    export([head], "head_animation.obj")


if __name__ == "__main__":
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    main(args[0])
