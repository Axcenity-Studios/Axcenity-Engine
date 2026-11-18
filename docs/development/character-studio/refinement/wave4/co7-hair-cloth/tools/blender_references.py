"""Blender reference renders for CO7's Hair and Cloth models (wave 4).

The engine's comparison scenes (render crate,
tests/character_models_evidence.rs) drawn in Cycles with Blender's own
models: a ball of radius 1 covered in strands that run down its meridians
from a root at the top (Principled Hair BSDF, Chiang, direct colouring),
and a ball of cloth (Principled BSDF with sheen; with anisotropy).

Engine coordinates (Y up, camera on +Z) become Blender's (Z up) as
(x, y, z) -> (x, -z, y).

Usage (Blender 5 Python with bpy and numpy): python blender_references.py <out folder>
"""
import math
import os
import random
import sys

import bpy
import numpy as np

OUT = sys.argv[1]
SIZE = 512
SAMPLES = 96


def linear(c):
    return [((x + 0.055) / 1.055) ** 2.4 if x > 0.04045 else x / 12.92 for x in c]


def engine_to_blender(v):
    x, y, z = v
    return (x, -z, y)


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = SAMPLES
    scene.cycles.use_denoising = True
    scene.render.resolution_x = SIZE
    scene.render.resolution_y = SIZE
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    scene.render.film_transparent = False
    world = bpy.data.worlds.new("Dark")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.0
    scene.world = world
    cam_data = bpy.data.cameras.new("Camera")
    cam_data.sensor_fit = "VERTICAL"
    cam_data.angle_y = math.radians(30.0)
    cam = bpy.data.objects.new("Camera", cam_data)
    cam.location = engine_to_blender((0.0, 0.0, 6.0))
    # Looking along +Y (engine -Z), Z up.
    cam.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    scene.collection.objects.link(cam)
    scene.camera = cam
    return scene


def sun(scene, toward, strength):
    data = bpy.data.lights.new("Sun", "SUN")
    data.energy = strength
    data.angle = math.radians(0.5)
    obj = bpy.data.objects.new("Sun", data)
    d = np.array(engine_to_blender(toward), dtype=float)
    d /= np.linalg.norm(d)
    # A sun shines along its -Z: point -Z at -d (from the sun to the ball).
    z = d
    x = np.cross([0.0, 0.0, 1.0], z)
    if np.linalg.norm(x) < 1e-6:
        x = np.array([1.0, 0.0, 0.0])
    x /= np.linalg.norm(x)
    y = np.cross(z, x)
    from mathutils import Matrix

    m = Matrix(((x[0], y[0], z[0]), (x[1], y[1], z[1]), (x[2], y[2], z[2])))
    obj.rotation_euler = m.to_euler()
    scene.collection.objects.link(obj)


def sphere(scene, material, radius=1.0):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=128, ring_count=64, radius=radius)
    obj = bpy.context.active_object
    bpy.ops.object.shade_smooth()
    obj.data.materials.append(material)
    return obj


def hair_material(colour, roughness):
    m = bpy.data.materials.new("Hair")
    m.use_nodes = True
    nodes = m.node_tree.nodes
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    h = nodes.new("ShaderNodeBsdfHairPrincipled")
    h.model = "CHIANG"
    h.parametrization = "COLOR"
    h.inputs["Color"].default_value = (*linear(colour), 1.0)
    h.inputs["Roughness"].default_value = roughness
    h.inputs["Radial Roughness"].default_value = 0.3
    h.inputs["Coat"].default_value = 0.0
    h.inputs["IOR"].default_value = 1.55
    h.inputs["Offset"].default_value = math.radians(2.0)
    for name in ("Random Color", "Random Roughness"):
        if h.inputs.get(name) is not None:
            h.inputs[name].default_value = 0.0
    m.node_tree.links.new(h.outputs[0], out.inputs["Surface"])
    return m


def hair_ball(scene, colour, roughness, strands=6000, points=96):
    """Strands down the meridians of a ball of radius 1, from latitude 86°
    to -86°, over a black scalp."""
    scalp = bpy.data.materials.new("Scalp")
    scalp.use_nodes = True
    scalp.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0, 0, 0, 1)
    sphere(scene, scalp, 0.995)
    rng = random.Random(7)
    curves = bpy.data.hair_curves.new("Hair")
    curves.add_curves([points] * strands)
    positions = np.zeros((strands * points, 3))
    for i in range(strands):
        lon = rng.random() * 2.0 * math.pi
        r = 1.0 + 0.002 + 0.012 * rng.random()
        for j in range(points):
            lat = math.radians(86.0) - math.radians(172.0) * j / (points - 1)
            # Engine (x, y, z) on the ball, root at +Y.
            ex = r * math.cos(lat) * math.cos(lon)
            ey = r * math.sin(lat)
            ez = r * math.cos(lat) * math.sin(lon)
            positions[i * points + j] = engine_to_blender((ex, ey, ez))
    curves.attributes["position"].data.foreach_set("vector", positions.ravel())
    radius = curves.attributes.get("radius") or curves.attributes.new("radius", "FLOAT", "POINT")
    radius.data.foreach_set("value", np.full(strands * points, 0.0025))
    obj = bpy.data.objects.new("Hair", curves)
    scene.collection.objects.link(obj)
    curves.materials.append(hair_material(colour, roughness))
    return obj


def cloth_material(base, roughness, sheen, sheen_roughness, anisotropy):
    m = bpy.data.materials.new("Cloth")
    m.use_nodes = True
    nodes = m.node_tree.nodes
    p = nodes["Principled BSDF"]
    p.inputs["Base Color"].default_value = (*linear(base), 1.0)
    p.inputs["Roughness"].default_value = roughness
    p.inputs["Sheen Weight"].default_value = sheen
    p.inputs["Sheen Roughness"].default_value = sheen_roughness
    p.inputs["Sheen Tint"].default_value = (1, 1, 1, 1)
    p.inputs["Anisotropic"].default_value = anisotropy
    if anisotropy > 0.0:
        # Along the engine sphere's U: around its vertical axis (Blender Z).
        tangent = nodes.new("ShaderNodeTangent")
        tangent.direction_type = "RADIAL"
        tangent.axis = "Z"
        m.node_tree.links.new(tangent.outputs[0], p.inputs["Tangent"])
    return m


def render(scene, name):
    scene.render.filepath = os.path.join(OUT, f"blender-{name}.png")
    bpy.ops.render.render(write_still=True)
    print("wrote", scene.render.filepath, flush=True)


FRONT = (0.0, 0.15, 1.0)
BROWN = [0.45, 0.3, 0.18]
BLONDE = [0.75, 0.6, 0.4]
SUN = 3.0

only = sys.argv[2:] or None
jobs = {
    "hair-brown": lambda s: (hair_ball(s, BROWN, 0.2), sun(s, FRONT, SUN)),
    "hair-blonde": lambda s: (hair_ball(s, BLONDE, 0.2), sun(s, FRONT, SUN)),
    "hair-blonde-backlit": lambda s: (hair_ball(s, BLONDE, 0.2), sun(s, (0.0, 0.5, -1.0), SUN)),
    "cloth-standard": lambda s: (
        sphere(s, cloth_material([0.45] * 3, 0.8, 0.0, 0.5, 0.0)),
        sun(s, (0.0, 0.0, 1.0), SUN),
    ),
    "cloth-sheen": lambda s: (
        sphere(s, cloth_material([0.45] * 3, 0.8, 1.0, 0.5, 0.0)),
        sun(s, (0.0, 0.0, 1.0), SUN),
    ),
    "cloth-satin": lambda s: (
        sphere(s, cloth_material([0.05] * 3, 0.3, 0.0, 0.5, 0.8)),
        sun(s, (0.0, 0.0, 1.0), SUN),
    ),
}
for name, make in jobs.items():
    if only and name not in only:
        continue
    scene = reset()
    make(scene)
    render(scene, name)
