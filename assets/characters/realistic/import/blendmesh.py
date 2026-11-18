"""A work mesh to a Blender mesh object and back (Blender's Python), for
the steps that use Blender's own tools (the topology fixes, decimation,
remeshing). Positions go from glTF's axes to Blender's and back; texture
coordinates per corner and a material per face come along.
"""

import bpy
import numpy as np

from workmesh import WorkMesh


def to_object(mesh, name="Item"):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    P = mesh.P
    verts = np.stack([P[:, 0], -P[:, 2], P[:, 1]], axis=1)
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts.tolist(), [], [list(map(int, f)) for f in mesh.faces()])
    uv = me.uv_layers.new(name="UVMap")
    uv.data.foreach_set("uv", mesh.uv.astype(np.float32).ravel())
    for m in mesh.materials:
        me.materials.append(bpy.data.materials.new(m["name"]))
    me.polygons.foreach_set("material_index", mesh.face_material.astype(np.int32))
    me.update()
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    return obj


def from_object(obj, materials):
    """The object, modifiers applied, as a work mesh with `materials`."""
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = obj.evaluated_get(depsgraph)
    me = evaluated.to_mesh()
    co = np.zeros(len(me.vertices) * 3)
    me.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    P = np.stack([co[:, 0], co[:, 2], -co[:, 1]], axis=1)
    sizes = np.zeros(len(me.polygons), dtype=np.int64)
    me.polygons.foreach_get("loop_total", sizes)
    corners = np.zeros(len(me.loops), dtype=np.int64)
    me.loops.foreach_get("vertex_index", corners)
    uv = np.zeros(len(me.loops) * 2)
    if me.uv_layers.active:
        me.uv_layers.active.data.foreach_get("uv", uv)
    mat = np.zeros(len(me.polygons), dtype=np.int64)
    me.polygons.foreach_get("material_index", mat)
    out = WorkMesh(P, corners, sizes, uv.reshape(-1, 2), mat, [dict(m) for m in materials])
    evaluated.to_mesh_clear()
    return out
