"""Shared helpers for the realistic base body's build: the vendored files,
the base mesh's face sets, small maths and object plumbing.

Units are metres, Blender Z up, the character faces -Y (its left is +X).
The skeleton, its bone rolls and the weight limit are the stylized
humanoid's (`../humanoid/rig.py`), loaded by `stylized()` so the two
packages can't drift apart.
"""

import importlib.util
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
SOURCE = os.path.join(HERE, "source")
STYLIZED = os.path.join(os.path.dirname(HERE), "humanoid")

# The masculine frame's standing height in metres: 5 studs in the engine.
HEIGHT = 1.75
# The relaxed A-pose: arms 45 degrees below horizontal (default-humanoid D6).
ARM_DOWN = math.radians(45.0)

# Sculpt face sets of the realistic bodies (`source/extract.py` writes them
# as materials `fs_<id>`). The left side is +X.
FS_TORSO = 1            # the chest and upper back, up to the neck's base
FS_SOCKET = {1: 2, -1: 3}   # the bag behind each eyeball
FS_EAR = {1: 4, -1: 5}
FS_MOUTH_UPPER = 7      # inside the mouth: the roof and the upper lip's inner side
FS_MOUTH_LOWER = 8      # inside the mouth: the floor and the lower lip's inner side
FS_PALM = {1: 9, -1: 10}
FS_FOREARM = {1: 12, -1: 11}
FS_FOOT = {1: 14, -1: 13}
FS_SHIN = {1: 15, -1: 16}
FS_HEAD = 17            # the skull, the face (with the nose and the upper lip) and the neck
FS_PELVIS = 18
FS_WAIST = 19
FS_UPPER_ARM = {1: 21, -1: 20}
FS_MUZZLE = 22          # the lower lip, the chin and the skin round the mouth's corners
FS_THIGH = {1: 24, -1: 23}
FS_TOE_BASE = {1: 25, -1: 26}   # the ball of the foot
# The toes' sets: every set of the foot past the ball.
FS_TOES = {1: tuple(range(46, 64)), -1: tuple(range(27, 46))}
# Fingers: (proximal, intermediate, distal, nail) sets per side.
FS_FINGERS = {
    1: {"Little": (64, 65, 66, 67), "Ring": (68, 69, 70, 71), "Middle": (72, 73, 74, 75),
        "Index": (76, 77, 78, 79), "Thumb": (80, 81, 82, 83)},
    -1: {"Thumb": (84, 85, 86, 87), "Index": (88, 89, 90, 91), "Middle": (92, 93, 94, 95),
         "Ring": (96, 97, 98, 99), "Little": (100, 101, 102, 103)},
}
MOUTH_SETS = (FS_MOUTH_UPPER, FS_MOUTH_LOWER)
HEAD_SETS = (FS_HEAD, FS_MOUTH_UPPER, FS_MOUTH_LOWER, FS_MUZZLE, 2, 3, 4, 5)

SIDES = (("Left", 1), ("Right", -1))
FINGERS = ("Thumb", "Index", "Middle", "Ring", "Little")
SEGMENTS = ("Proximal", "Intermediate", "Distal")

_stylized = {}


def stylized(name):
    """A module of the stylized humanoid's build (`rig`, `animations`),
    loaded with its own `common` so its imports resolve."""
    if name in _stylized:
        return _stylized[name]
    saved = sys.modules.get("common")

    def load(module, alias):
        spec = importlib.util.spec_from_file_location(alias, os.path.join(STYLIZED, module + ".py"))
        made = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(made)
        return made

    try:
        sys.modules["common"] = _stylized.get("common") or load("common", "stylized_common")
        _stylized["common"] = sys.modules["common"]
        _stylized[name] = load(name, "stylized_" + name)
    finally:
        if saved is not None:
            sys.modules["common"] = saved
        else:
            sys.modules.pop("common", None)
    return _stylized[name]


def v(x, y, z):
    return Vector((x, y, z))


def smoothstep(e0, e1, x):
    if e0 == e1:
        return 1.0 if x >= e1 else 0.0
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0)))
    return t * t * (3 - 2 * t)


def centroid(points):
    points = list(points)
    total = Vector((0.0, 0.0, 0.0))
    for p in points:
        total += p
    return total / max(1, len(points))


def reset_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    return scene


def import_obj(name):
    """Imports `source/<name>`, its object transform applied to the mesh
    (the OBJ importer turns Y up into an object rotation)."""
    before = set(bpy.data.objects)
    bpy.ops.wm.obj_import(filepath=os.path.join(SOURCE, name))
    objs = [o for o in bpy.data.objects if o not in before]
    for o in objs:
        o.data.transform(o.matrix_world)
        o.matrix_world = Matrix.Identity(4)
    return objs


def face_sets(obj):
    """The face set of each polygon, from the `fs_<id>` materials."""
    ids = [int(m.name.split("_")[1].split(".")[0]) for m in obj.data.materials]
    return [ids[p.material_index] for p in obj.data.polygons]


def positions(obj):
    return [vert.co.copy() for vert in obj.data.vertices]


def set_positions(obj, points):
    for vert, p in zip(obj.data.vertices, points):
        vert.co = p
    obj.data.update()


def array(points):
    return np.array([tuple(p) for p in points], dtype=np.float64)


def new_object(name, mesh):
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def duplicate(obj, name):
    copy = obj.copy()
    copy.data = obj.data.copy()
    copy.name = name
    copy.data.name = name
    bpy.context.scene.collection.objects.link(copy)
    return copy


def shade_smooth(obj):
    for p in obj.data.polygons:
        p.use_smooth = True


def select_only(objs, active=None):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = active or objs[0]


def tri_count(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


def edge_neighbours(mesh):
    """Vertex index -> list of neighbouring vertex indices."""
    nbr = [[] for _ in mesh.vertices]
    for e in mesh.edges:
        a, b = e.vertices
        nbr[a].append(b)
        nbr[b].append(a)
    return nbr


def evaluated_positions(obj):
    """`obj`'s vertices with its modifiers and pose applied, as an array."""
    deps = bpy.context.evaluated_depsgraph_get()
    mesh = obj.evaluated_get(deps).to_mesh()
    co = np.empty(len(mesh.vertices) * 3, dtype=np.float64)
    mesh.vertices.foreach_get("co", co)
    obj.evaluated_get(deps).to_mesh_clear()
    return co.reshape(-1, 3)


def subdivide(obj, levels):
    """The subdivision the vendored sculpt is stored against (the same
    settings as `source/extract.py`'s)."""
    mod = obj.modifiers.new("Subdivide", "SUBSURF")
    mod.subdivision_type = "CATMULL_CLARK"
    mod.levels = levels
    mod.render_levels = levels
    mod.quality = 3
    mod.use_limit_surface = True
    mod.boundary_smooth = "ALL"
    mod.uv_smooth = "PRESERVE_BOUNDARIES"
    return mod


def fit_sphere(points):
    """The least-squares sphere through `points`: (centre, radius)."""
    P = array(points)
    A = np.c_[2 * P, np.ones(len(P))]
    sol, *_ = np.linalg.lstsq(A, (P ** 2).sum(1), rcond=None)
    centre = sol[:3]
    return Vector(centre), float(np.sqrt(sol[3] + (centre ** 2).sum()))
