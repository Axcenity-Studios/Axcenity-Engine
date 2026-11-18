"""Close-up renders of one face_checks pose with a marker at a point.

    bpy-python -I render_probe.py REPO PARTS.glb POSESPEC OUTPREFIX X Y Z [frame]

POSESPEC is a face_checks pose name (e.g. "eye_corner_right-1 viseme L") or
"pair:pidA=end,pidB=end" for an identity pair at rest. Point in mm.
Writes OUTPREFIX_front.png (studio materials), _section.png (a cut through
the plane x = X seen from +X: flesh-side surfaces are red where the cut
looks into a closed part's inside), _from_point.png (a camera at the point
looking at the nearest body surface; red = back faces, i.e. inside flesh).
"""
import sys, os, json, math
import bpy
import numpy as np
from mathutils import Vector

repo, parts, spec, outp = sys.argv[1:5]
X, Y, Z = (float(v) / 1000.0 for v in sys.argv[5:8])
frame = float(sys.argv[8]) if len(sys.argv) > 8 else 0.0
sys.path.insert(0, os.path.join(repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q, face_checks as FC  # noqa: E402

_, _, custom_path = Q.package_paths("realistic")
custom = Q.load_custom(custom_path)
with open(custom_path, encoding="utf-8") as f:
    custom["face"] = json.load(f).get("face", {}).get("face_identity", custom.get("face", {}))
Q.reset()
ch = Q.Character(parts, merge_vertices=True, custom=custom)
shapes = {kb.name for kb in ch.parts["Body"].data.shape_keys.key_blocks}
params = FC.face_parameters(os.path.join(os.path.dirname(parts), "Realistic.base.json"))
pd = dict(params)
if spec.startswith("pair:"):
    ident = {}
    for item in spec[5:].split(","):
        pid, end = item.split("=")
        for s, v in pd[pid](float(end)).items():
            if s in shapes:
                ident[s] = ident.get(s, 0.0) + v
    FC.pose(ch, frame, {}, ident)
else:
    for group, name, w, ident, req in FC.poses(shapes, params):
        if name == spec:
            FC.pose(ch, frame, w, ident)
            break
    else:
        raise SystemExit("no pose " + spec)

P = Vector((X, Y, Z))
cam = Q.setup_studio(900, 900, 64)
ch.show_only([p for p in ("Body", "Eyes", "Teeth", "EyeShell", "Lashes", "Brows", "LipSeal") if p in ch.parts])
scene = bpy.context.scene
bpy.ops.mesh.primitive_uv_sphere_add(radius=0.00035, location=P)
marker = bpy.context.active_object
mm = bpy.data.materials.new("Marker")
mm.use_nodes = True
em = mm.node_tree.nodes.new("ShaderNodeEmission")
em.inputs[0].default_value = (0.0, 1.0, 0.1, 1.0)
em.inputs[1].default_value = 4.0
mm.node_tree.links.new(em.outputs[0], mm.node_tree.nodes["Material Output"].inputs["Surface"])
marker.data.materials.append(mm)
if os.environ.get("NOMARKER"):
    marker.hide_render = True


def shoot(name, loc, target, lens=None, ortho=None, clip=0.001):
    cam.location = loc
    Q.look_at(cam, target)
    if ortho:
        cam.data.type = "ORTHO"
        cam.data.ortho_scale = ortho
    else:
        cam.data.type = "PERSP"
        cam.data.lens = lens or 50
    cam.data.clip_start = clip
    cam.data.clip_end = 5.0
    scene.render.filepath = f"{outp}_{name}.png"
    bpy.ops.render.render(write_still=True)


# 1. Front, studio materials, about 60 mm across.
zoom = float(os.environ.get("ZOOM", "280"))
shoot("front", P + Vector((0.0, -0.35, 0.02)), P, lens=zoom)
side = -1.0 if X < 0 else 1.0
shoot("front34", P + Vector((0.22 * side, -0.28, 0.03)), P, lens=zoom)

# Flat shading for the cut views: emission per part, red back faces.
def flat(color):
    m = bpy.data.materials.new("Flat")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    geo = nt.nodes.new("ShaderNodeNewGeometry")
    lw = nt.nodes.new("ShaderNodeLayerWeight")
    ramp = nt.nodes.new("ShaderNodeMixRGB")
    ramp.blend_type = "MIX"
    ramp.inputs[1].default_value = color
    ramp.inputs[2].default_value = tuple(c * 0.35 for c in color[:3]) + (1.0,)
    nt.links.new(lw.outputs["Facing"], ramp.inputs[0])
    back = nt.nodes.new("ShaderNodeMixRGB")
    back.inputs[2].default_value = (1.0, 0.0, 0.0, 1.0)
    nt.links.new(geo.outputs["Backfacing"], back.inputs[0])
    nt.links.new(ramp.outputs[0], back.inputs[1])
    e = nt.nodes.new("ShaderNodeEmission")
    nt.links.new(back.outputs[0], e.inputs[0])
    nt.links.new(e.outputs[0], out.inputs["Surface"])
    return m


colors = {"Body": (0.85, 0.7, 0.6, 1), "Teeth": (0.95, 0.95, 0.85, 1), "LipSeal": (0.2, 0.4, 1.0, 1),
          "Eyes": (0.9, 0.9, 0.9, 1), "EyeShell": (0.9, 0.9, 0.9, 1), "Lashes": (0.1, 0.1, 0.1, 1),
          "Brows": (0.1, 0.1, 0.1, 1)}
for name, o in ch.parts.items():
    if name in colors:
        m = flat(colors[name])
        for slot in o.material_slots:
            slot.material = m
for o in scene.objects:
    if o.type == "LIGHT":
        o.hide_render = True
scene.cycles.samples = 16
# 2. Section through x = X seen from +X (the body's left), 40 mm across.
shoot("section", Vector((X + 0.25, Y, Z)), P, ortho=0.04, clip=0.25)
# 3. From the point toward the nearest body surface.
dg = bpy.context.evaluated_depsgraph_get()
body = FC.Piece(ch.parts["Body"])
loc, normal, face, dist = body.bvh.find_nearest(P, 0.05)
cam.data.angle = math.radians(100)
cam.location = P
Q.look_at(cam, loc)
cam.data.type = "PERSP"
cam.data.lens_unit = "FOV"
cam.data.angle = math.radians(100)
cam.data.clip_start = 0.00005
marker.hide_render = True
if 'Teeth' in ch.parts:
    ch.parts['Teeth'].hide_render = True
scene.render.filepath = f"{outp}_from_point.png"
bpy.ops.render.render(write_still=True)
print("nearest body point mm", [round(v * 1000, 2) for v in loc], "dist mm", round(dist * 1000, 3))
