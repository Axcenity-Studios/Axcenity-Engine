"""Sewing and draping pattern pieces on the realistic body with Blender's
cloth simulation.

The pieces (`panels.Panel` grids, placed round the body by the garment's
own module) become one cloth mesh; each seam is a row of sewing springs
(loose edges between the two pieces' corners, which Blender's cloth pulls
closed when `use_sewing_springs` is on). The body is a collision object
in its rest pose. Gravity is eased in over the first frames so the seams
close before the cloth falls, then the cloth settles. What comes back is
each corner's place, with sewn corners joined at their middle.
"""

import bpy
import numpy as np


class Settings:
    """Cloth settings (Blender's `ClothSettings`, SI units). Jersey is
    light and soft; fleece a little heavier; both stretch little along
    the grain at these stiffnesses."""

    def __init__(self, **kw):
        self.frames = 90
        self.gravity_from = 1
        self.gravity_full = 30
        self.quality = 10
        self.mass = 0.15
        self.air = 1.0
        self.tension = 25.0
        self.compression = 25.0
        self.shear = 8.0
        self.bending = 0.08
        self.tension_damping = 5.0
        self.sewing_force = 30.0
        self.distance = 0.003
        self.thickness = 0.002
        self.friction = 3.0
        self.self_collision = False
        self.self_distance = 0.003
        self.pin_stiffness = 1.0
        for k, v in kw.items():
            setattr(self, k, v)


def collision_body(frame):
    """The body as a collision object in its rest pose (the feminine frame
    with `Frame_Feminine` at full weight); everything else imported is
    removed."""
    body = bpy.data.objects["Body"]
    for obj in list(bpy.data.objects):
        if obj.type == "MESH" and obj is not body:
            bpy.data.objects.remove(obj)
    keys = body.data.shape_keys
    if keys:
        for key in keys.key_blocks:
            key.value = 0.0
        if frame == "feminine":
            keys.key_blocks["Frame_Feminine"].value = 1.0
    mod = body.modifiers.new("Collision", "COLLISION")
    body.collision.thickness_outer = 0.002
    body.collision.thickness_inner = 0.02
    body.collision.damping = 0.5
    body.collision.cloth_friction = 5.0
    return body


def simulate(verts, faces, seams, settings, pins=None, name="Garment"):
    """Drapes the cloth: `verts` (n, 3) in Blender's world, `faces` (quads),
    `seams` pairs of corner indices to sew, `pins` corners held where they
    are. Returns the settled corners (n, 3); `join` closes the seams.

    The cloth takes its springs' rest lengths and its folds' rest angles
    from `verts` as given, so the pieces must start as long as they were
    cut: rolled, not stretched (a piece wrapped round a cylinder or an
    ellipse along its length is). Neither a rest shape key
    (`rest_shape_key`: the springs kept the starting lengths here) nor a
    base mesh unbending to the flat pieces (`use_dynamic_mesh`: it dragged
    the sleeves into the armholes) did better."""
    scene = bpy.context.scene
    me = bpy.data.meshes.new(name)
    me.from_pydata([tuple(v) for v in verts], [tuple(e) for e in seams], [tuple(f) for f in faces])
    me.update()
    obj = bpy.data.objects.new(name, me)
    scene.collection.objects.link(obj)
    if pins is not None and len(pins):
        group = obj.vertex_groups.new(name="Pin")
        group.add([int(k) for k in pins], 1.0, "REPLACE")
    mod = obj.modifiers.new("Cloth", "CLOTH")
    cs = mod.settings
    cs.quality = settings.quality
    cs.mass = settings.mass
    cs.air_damping = settings.air
    cs.tension_stiffness = settings.tension
    cs.compression_stiffness = settings.compression
    cs.shear_stiffness = settings.shear
    cs.bending_stiffness = settings.bending
    cs.tension_damping = settings.tension_damping
    cs.compression_damping = settings.tension_damping
    cs.use_sewing_springs = True
    cs.sewing_force_max = settings.sewing_force
    if pins is not None and len(pins):
        cs.vertex_group_mass = "Pin"
        cs.pin_stiffness = settings.pin_stiffness
    cc = mod.collision_settings
    cc.distance_min = settings.distance
    cc.collision_quality = 4
    cc.friction = settings.friction
    cc.use_self_collision = settings.self_collision
    cc.self_distance_min = settings.self_distance
    # Gravity eased in, so the seams close before the cloth falls.
    weights = cs.effector_weights
    weights.gravity = 0.0
    weights.keyframe_insert("gravity", frame=settings.gravity_from)
    weights.gravity = 1.0
    weights.keyframe_insert("gravity", frame=settings.gravity_full)
    scene.frame_start = 1
    scene.frame_end = settings.frames
    mod.point_cache.frame_start = 1
    mod.point_cache.frame_end = settings.frames
    for frame in range(1, settings.frames + 1):
        scene.frame_set(frame)
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = obj.evaluated_get(depsgraph)
    em = evaluated.to_mesh()
    out = np.zeros(len(em.vertices) * 3)
    em.vertices.foreach_get("co", out)
    evaluated.to_mesh_clear()
    out = out.reshape(-1, 3) @ np.array(obj.matrix_world)[:3, :3].T + np.array(obj.matrix_world)[:3, 3]
    bpy.data.objects.remove(obj)
    return out


def join(points, seams):
    """Sewn corners moved to their middle (a corner sewn twice, as where
    four pieces meet under the arm, to the middle of all of them)."""
    out = points.copy()
    parent = list(range(len(points)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for a, b in seams:
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[ra] = rb
    groups = {}
    for k in range(len(points)):
        groups.setdefault(find(k), []).append(k)
    for members in groups.values():
        if len(members) > 1:
            out[members] = points[members].mean(axis=0)
    return out


def gap_report(points, seams):
    """How far apart the sewn corners were left (m): the largest and the
    mean, to see the seams closed."""
    if not seams:
        return 0.0, 0.0
    d = np.array([np.linalg.norm(points[a] - points[b]) for a, b in seams])
    return float(d.max()), float(d.mean())

