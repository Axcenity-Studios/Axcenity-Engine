"""The body's simpler copies, `Body_LOD1` and `Body_LOD2`: the engine
draws them in the body's place when it's small on screen (it reads a
file's `_LOD1`/`_LOD2` objects as a model's levels of detail and pairs
them with the full primitives by material, `docs/rendering/models.md`).

Each is the body decimated toward a triangle budget, left and right kept
mirror images. The first copy holds back what a character can't lose up
close: the face (eyes, nose and lips keep their loops longest), the
hands, and the rings of skin round each joint, which need their edges to
bend. The crowd copy is too small for that (see `LEVELS`). The copies keep
the body's skeleton, weights (four per vertex), texture coordinates, both
materials and regions, and carry every shape the body has, moved across
by where each of their vertices lies on the body's surface. So a
character's level can change without its pose, frame or clothing
changing.
"""

import bmesh
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

import regions
import rig
from common import FS_FINGERS, FS_PALM, HEAD_SETS, array, duplicate, select_only

# (name, triangles aimed for, how strongly the kept places are held back):
# gameplay distance and crowds (plan section 7). The crowd copy holds
# nothing back: at 5,000 triangles the face's loops alone would take most
# of the budget and the skull, shoulders and limbs would lose their shape,
# so there the error measure alone decides, which keeps the silhouette.
LEVELS = [("Body_LOD1", 10500, 1.0), ("Body_LOD2", 5000, 0.0)]
# Joints whose rings are kept: (bone at the joint, reach along the limb in metres).
JOINTS = [("UpperArm", 0.07), ("LowerArm", 0.06), ("Hand", 0.04), ("UpperLeg", 0.09), ("LowerLeg", 0.07),
          ("Foot", 0.05)]


def protect(body, lm, P, joints):
    """A vertex group `Keep` on `body`: 1 where detail is held back
    longest, 0 where it can go first."""
    X = array(P)
    keep = np.full(len(X), 0.3)
    of_set = {fs: np.array(sorted(idx), dtype=np.int64) for fs, idx in lm.of_set.items()}
    head = np.unique(np.concatenate([of_set[fs] for fs in HEAD_SETS]))
    head_joint = np.array(joints["Head"][0])
    # The face: in front of the ears, from the chin up to the brow.
    front = head[(X[head][:, 1] < head_joint[1] - 0.045) & (X[head][:, 2] > head_joint[2] - 0.08) &
                 (X[head][:, 2] < head_joint[2] + 0.11)]
    keep[head] = 0.5
    keep[front] = 1.0
    for s in (1, -1):
        hand = np.unique(np.concatenate([of_set[FS_PALM[s]]] + [of_set[fs] for g in FS_FINGERS[s].values() for fs in g]))
        keep[hand] = np.maximum(keep[hand], 0.6)
        side = "Left" if s > 0 else "Right"
        for bone, reach in JOINTS:
            at = np.array(joints[side + bone][0])
            near = np.linalg.norm(X - at, axis=1)
            keep = np.maximum(keep, 0.7 * np.clip(1.0 - near / (reach * 1.8), 0.0, 1.0) ** 0.5 * (near < reach * 1.8))
    group = body.vertex_groups.get("Keep") or body.vertex_groups.new(name="Keep")
    for i, value in enumerate(keep):
        group.add([i], float(value), "REPLACE")
    return group


def _bind(points, positions, triangles):
    """For each of `points`, the triangle of the body nearest it and where
    it lies on it (barycentric), plus what's left over."""
    tree = BVHTree.FromPolygons([tuple(p) for p in positions], [tuple(t) for t in triangles], all_triangles=True)
    out = []
    for p in points:
        loc, _, index, _ = tree.find_nearest(Vector(p))
        a, b, c = (positions[i] for i in triangles[index])
        v0, v1, v2 = b - a, c - a, np.array(loc) - a
        d00, d01, d11, d20, d21 = v0 @ v0, v0 @ v1, v1 @ v1, v2 @ v0, v2 @ v1
        den = d00 * d11 - d01 * d01
        v = (d11 * d20 - d01 * d21) / den if abs(den) > 1e-18 else 0.0
        w = (d00 * d21 - d01 * d20) / den if abs(den) > 1e-18 else 0.0
        out.append((index, (1.0 - v - w, v, w), np.array(p) - np.array(loc)))
    return out


def build(body, armature, lm, P, joints, face_regions):
    """The copies of `body` (which has its weights, materials, texture
    coordinates, regions and shapes), unparented. Returns the objects."""
    me = body.data
    me.calc_loop_triangles()
    triangles = [tuple(t.vertices) for t in me.loop_triangles]
    tri_face = [t.polygon_index for t in me.loop_triangles]
    positions = array(P)
    shapes = {}
    if me.shape_keys:
        for key in me.shape_keys.key_blocks[1:]:
            co = np.empty(len(me.vertices) * 3, dtype=np.float64)
            key.data.foreach_get("co", co)
            shapes[key.name] = co.reshape(-1, 3) - positions
    protect(body, lm, P, joints)
    full = sum(len(p.vertices) - 2 for p in me.polygons)
    copies = []
    for name, budget, hold in LEVELS:
        copy = duplicate(body, name)
        copy.parent = None
        copy.shape_key_clear()
        for mod in list(copy.modifiers):
            copy.modifiers.remove(mod)
        select_only([copy])
        mod = copy.modifiers.new("Decimate", "DECIMATE")
        mod.decimate_type = "COLLAPSE"
        mod.ratio = budget / full
        mod.use_symmetry = True
        mod.symmetry_axis = "X"
        mod.use_collapse_triangulate = True
        if hold:
            mod.vertex_group = "Keep"
            mod.invert_vertex_group = True
            mod.vertex_group_factor = hold
        bpy.ops.object.modifier_apply(modifier="Decimate")
        made = copy.data
        copy.vertex_groups.remove(copy.vertex_groups["Keep"])
        # Weights: four per vertex again (collapsing edges mixes them).
        weights = rig.Weights(copy, armature)
        weights.limit(4)
        weights.write(copy)
        # Regions: each face takes the body's nearest face's.
        points = array(v.co for v in made.vertices)
        centres = [np.mean([points[i] for i in p.vertices], axis=0) for p in made.polygons]
        bound = _bind(centres, positions, triangles)
        regions.write_layer(copy, [face_regions[tri_face[index]] for index, _, _ in bound])
        # Shapes: each vertex moves as the point of the body under it does.
        bound = _bind(points, positions, triangles)
        if shapes:
            copy.shape_key_add(name="Basis", from_mix=False)
            for shape, delta in shapes.items():
                key = copy.shape_key_add(name=shape, from_mix=False)
                for i, (index, (u, v, w), _) in enumerate(bound):
                    a, b, c = triangles[index]
                    key.data[i].co = Vector(points[i] + u * delta[a] + v * delta[b] + w * delta[c])
                key.value = 0.0
        for poly in made.polygons:
            poly.use_smooth = True
        copies.append(copy)
    body.vertex_groups.remove(body.vertex_groups["Keep"])
    return copies


def triangles_of(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


del bmesh
