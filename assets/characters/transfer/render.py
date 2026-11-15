"""Clay renders of bare meshes for the transfer spike's sheets, in the
humanoid QA studio (`../humanoid/qa/qa_scene.py`: Cycles on the CPU, three
area lights, a grey cyclorama, a fixed seed), so they compare with the
baseline sheets.

Used by `sheets.py` and `landmarks.py`; not run on its own.
"""

import math
import os
import sys

import bpy
import numpy as np
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "humanoid", "qa"))

import qa_scene as Q  # noqa: E402

CLAY = (0.46, 0.36, 0.31)
# The QA studio with a harder key and less fill, so the surface's relief
# (muscle definition, folds) reads in clay.
LIGHTS = {"key": ((-2.6, -2.8, 3.3), 800.0, 1.1), "fill": ((3.0, -2.6, 1.7), 70.0, 3.0),
          "world": (0.025, 0.025, 0.028)}
EXPOSURE = -0.35


class Studio:
    def __init__(self, width, height, samples=24):
        Q.reset()
        Q.STUDIO.update(LIGHTS)
        self.cam = Q.setup_studio(width, height, samples)
        scene = bpy.context.scene
        scene.view_settings.exposure = EXPOSURE
        # The backdrop still lights the figure but isn't seen: tiles are
        # composited on flat grey (`flat`), which keeps the sheets small.
        bpy.data.objects["QA_Backdrop"].visible_camera = False
        scene.render.film_transparent = True
        scene.render.image_settings.color_mode = "RGBA"
        self.cam.data.type = "ORTHO"
        self.size = (width, height)
        self.obj = None

    def mesh(self, verts, faces, colors=None, smooth=True):
        """Shows one mesh (replacing the last): `verts` (n, 3), `faces`
        rows of vertex indices padded with -1, `colors` (n, 3) for a
        painted map instead of clay."""
        if self.obj is not None:
            old = self.obj.data
            bpy.data.objects.remove(self.obj)
            bpy.data.meshes.remove(old)
        me = bpy.data.meshes.new("Body")
        me.from_pydata([tuple(v) for v in verts], [],
                       [tuple(int(i) for i in f if i >= 0) for f in faces])
        me.update()
        for p in me.polygons:
            p.use_smooth = smooth
        obj = bpy.data.objects.new("Body", me)
        bpy.context.scene.collection.objects.link(obj)
        if colors is None:
            mat = Q.flat_material("Clay", CLAY, 0.55)
        else:
            attr = me.color_attributes.new("map", "FLOAT_COLOR", "POINT")
            attr.data.foreach_set("color", np.c_[colors, np.ones(len(colors))].ravel())
            mat = bpy.data.materials.get("Map") or bpy.data.materials.new("Map")
            try:
                mat.use_nodes = True
            except Exception:
                pass
            nt = mat.node_tree
            if "MapAttr" not in nt.nodes:
                node = nt.nodes.new("ShaderNodeVertexColor")
                node.name = "MapAttr"
                node.layer_name = "map"
                bsdf = nt.nodes.get("Principled BSDF")
                bsdf.inputs["Roughness"].default_value = 0.6
                nt.links.new(node.outputs["Color"], bsdf.inputs["Base Color"])
        me.materials.append(mat)
        self.obj = obj
        return obj

    def shoot(self, path, azimuth=0.0, centre=(0.0, 0.0, 0.9), span=1.9, elevation=4.0):
        """Renders to `path` with the figure turned `azimuth` degrees (0
        shows its front, 35 a three-quarter view from its left, 180 its
        back; the lights stay put), the frame `span` metres tall about
        `centre`."""
        e = math.radians(elevation)
        self.obj.rotation_euler = (0.0, 0.0, -math.radians(azimuth))
        d = Vector((0.0, -math.cos(e), math.sin(e)))
        c = Vector(centre)
        self.cam.location = c + d * 6.0
        Q.look_at(self.cam, c)
        w, h = self.size
        self.cam.data.ortho_scale = span * max(w / h, 1.0) if w > h else span
        self.cam.data.sensor_fit = "VERTICAL"
        bpy.context.scene.render.filepath = path
        bpy.ops.render.render(write_still=True)


BACKGROUND = (176, 176, 176)


def flat(path):
    """The render at `path` over the flat background, as an RGB image."""
    from PIL import Image
    im = Image.open(path).convert("RGBA")
    out = Image.new("RGBA", im.size, BACKGROUND + (255,))
    out.alpha_composite(im)
    return out.convert("RGB")


def stand(verts):
    """`verts` moved so the figure stands on the ground, centred."""
    v = np.array(verts, dtype=np.float64)
    v[:, 2] -= v[:, 2].min()
    v[:, 0] -= (v[:, 0].min() + v[:, 0].max()) / 2
    return v
