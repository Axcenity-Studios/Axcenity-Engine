"""The Humanoid skeleton as an armature: the bones the engine's humanoid
roles name (`Hips`, `Spine`, ..., Unity's and VRM's names), with rolls
that give every bone the same axes whatever body it is fitted to: +X
swings a limb forward and bends an elbow, a knee bends with -X, finger
curl is -X. `build.py` fits it to MakeHuman's joints.
"""

import bpy
from mathutils import Vector

FINGERS = ("Thumb", "Index", "Middle", "Ring", "Little")


def palm_normal(joints, side, s):
    """The palm's normal (toward the body in the A-pose)."""
    wrist = joints[f"{side}Hand"][0]
    knuckle = joints[f"{side}Hand"][1]
    index = joints[f"{side}IndexProximal"][0]
    little = joints[f"{side}LittleProximal"][0]
    along = (knuckle - wrist).normalized()
    across = (index - little).normalized()  # toward the thumb side
    return (along.cross(across) * s).normalized()


def build_armature(joints, name):
    """An armature from `joints` ({bone: (head, tail, parent)})."""
    arm = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, arm)
    bpy.context.scene.collection.objects.link(obj)
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    for bone_name, (head, tail, _) in joints.items():
        eb = arm.edit_bones.new(bone_name)
        eb.head = head
        eb.tail = tail
        eb.use_deform = bone_name != "Root"
    for bone_name, (_, _, parent) in joints.items():
        eb = arm.edit_bones[bone_name]
        if parent:
            eb.parent = arm.edit_bones[parent]
        d = (eb.tail - eb.head).normalized()
        if any(f in bone_name for f in FINGERS) and bone_name[4:] not in ("Hand",):
            side = "Left" if bone_name.startswith("Left") else "Right"
            s = 1 if side == "Left" else -1
            eb.align_roll(-palm_normal(joints, side, s))
        elif abs(d.y) > 0.7:
            eb.align_roll(Vector((0, 0, 1)))
        else:
            eb.align_roll(Vector((0, -1, 0)))
    bpy.ops.object.mode_set(mode="OBJECT")
    obj.select_set(False)
    return obj
