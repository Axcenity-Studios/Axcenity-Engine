"""Builds `HumanoidAnimations.glb`, the starter animations: Quaternius's
Universal Animation Library (CC0) keyed on the human base's skeleton
(`mocap.py`), one glTF animation a take, with the skeleton and no meshes.

    blender -b --python animations.py -- --out DIR \\
        --mocap <UAL1_Standard.glb> <UAL2_Standard.glb>

The skeleton is read out of `HumanParts.glb` and rebuilt bone for bone
(each bone's own axes as the file has them), so a take's turns are the
model's own and the engine plays them without retargeting. It prints, for
each take, its frames, how far the feet were from the library's before
the hips were corrected, how high the hips ride and how far the take was
lifted to keep the joints that lie on the ground out of it. Copy the file
here and into `crates/samples/projects/humanoid/assets/models/Humanoid/`.
"""

import argparse
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bpy  # noqa: E402
from mathutils import Matrix, Quaternion, Vector  # noqa: E402

import mocap  # noqa: E402
from glb import Glb  # noqa: E402

ARMATURE = "AxcenityHuman"
PARTS = "HumanParts.glb"
OUT = "HumanoidAnimations.glb"
#: glTF's axes (Y up, facing +Z) in Blender's (Z up, facing -Y).
TO_BLENDER = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))


def _local(node):
    if "matrix" in node:
        m = node["matrix"]
        return Matrix([m[0:16:4], m[1:16:4], m[2:16:4], m[3:16:4]])
    t = Matrix.Translation(Vector(node.get("translation", (0, 0, 0))))
    x, y, z, w = node.get("rotation", (0, 0, 0, 1))
    r = Quaternion((w, x, y, z)).to_matrix().to_4x4()
    sx, sy, sz = node.get("scale", (1, 1, 1))
    return t @ r @ Matrix.Diagonal((sx, sy, sz, 1))


def skeleton(path=None):
    """The armature of the model file at `path` (the base's parts file):
    every joint of its skin a bone with the axes the file gives it."""
    model = Glb(path or os.path.join(HERE, PARTS))
    nodes = model.json["nodes"]
    joints = model.json["skins"][0]["joints"]
    parent = {child: i for i, node in enumerate(nodes) for child in node.get("children", [])}
    world = {}

    def placed(index):
        if index not in world:
            above = placed(parent[index]) if index in parent else Matrix.Identity(4)
            world[index] = above @ _local(nodes[index])
        return world[index]

    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj)
    data = bpy.data.armatures.new(ARMATURE)
    armature = bpy.data.objects.new(ARMATURE, data)
    bpy.context.scene.collection.objects.link(armature)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    made = {}
    for index in joints:
        name = nodes[index]["name"]
        # Only the world's axes change: a bone keeps its own, as the
        # exporter wrote them.
        at = TO_BLENDER @ placed(index)
        children = [c for c in nodes[index].get("children", []) if c in joints]
        reach = max([(placed(c).to_translation() - placed(index).to_translation()).length for c in children],
                    default=0.0)
        bone = data.edit_bones.new(name)
        bone.head, bone.tail = (0, 0, 0), (0, max(reach, 0.03), 0)
        bone.matrix = at
        bone.use_deform = True
        made[index] = bone
    for index, bone in made.items():
        if parent.get(index) in made:
            bone.parent = made[parent[index]]
            bone.use_connect = False
    bpy.ops.object.mode_set(mode="OBJECT")
    return armature


def export(path, armature):
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    bpy.context.view_layer.objects.active = armature
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", use_selection=True,
        export_skins=True, export_morph=False, export_animations=True,
        export_animation_mode="ACTIONS", export_force_sampling=True,
        export_optimize_animation_size=True, export_anim_slide_to_zero=True,
        export_yup=True, export_def_bones=False, export_leaf_bone=False)


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--mocap", nargs=2, required=True, metavar=("UAL1", "UAL2"))
    options = parser.parse_args(args)
    os.makedirs(options.out, exist_ok=True)
    for action in list(bpy.data.actions):
        bpy.data.actions.remove(action)
    armature = skeleton()
    taken = mocap.bake(armature, options.mocap)
    export(os.path.join(os.path.abspath(options.out), OUT), armature)
    print(f"{len(taken)} takes from the library")
    for name, made in taken.items():
        print(f"TAKE {name:20s} {made['frames']:4d} frames  feet {made['feet_off_mm']:6.1f} mm  "
              f"hips {made['hips_mean_m']:+.3f} m  lifted {made['lifted_mm']:6.1f} mm  ({made['from']})")


if __name__ == "__main__":
    main()
