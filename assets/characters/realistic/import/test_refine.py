"""Tests of the refinement step and the loader's texture handling (wave 5,
the first licensed joggers). Blender's Python (bpy 5.x, NumPy, Pillow):

    python -I test_refine.py
"""

import io
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import numpy as np  # noqa: E402

import refine_joggers as rj  # noqa: E402


def open_edges(faces):
    """Edges with one face (as sorted vertex pairs)."""
    count = {}
    for f in faces:
        for a, b in zip(f, list(f[1:]) + [f[0]]):
            key = (min(a, b), max(a, b))
            count[key] = count.get(key, 0) + 1
    return [e for e, n in count.items() if n == 1]


class SectionTest(unittest.TestCase):
    def test_the_hull_of_a_square_and_its_radius_each_way(self):
        pts = np.array([[1, 1], [-1, 1], [-1, -1], [1, -1], [0.2, 0.3], [0, 0]], float)
        hull = rj.convex_hull(pts)
        self.assertEqual(len(hull), 4)
        self.assertAlmostEqual(rj.ray_polygon(hull, np.zeros(2), 0.0), 1.0, places=6)
        self.assertAlmostEqual(rj.ray_polygon(hull, np.zeros(2), np.pi / 4), np.sqrt(2), places=6)
        self.assertAlmostEqual(rj.ray_polygon(hull, np.array([0.5, 0.0]), np.pi), 1.5, places=6)

    def test_a_hull_bridges_a_hollow(self):
        # A "C": the hull spans its mouth, as cloth bridges the back of the knee.
        t = np.linspace(0.3, 2 * np.pi - 0.3, 60)
        pts = np.stack([np.cos(t), np.sin(t)], 1)
        pts = np.vstack([pts, [[0.4, 0.0]]])
        hull = rj.convex_hull(pts)
        self.assertGreater(rj.ray_polygon(hull, np.zeros(2), 0.0), 0.9)


class HeightMapTest(unittest.TestCase):
    def test_landmarks_map_both_ways(self):
        m = rj.HeightMap([0.085, 0.135, 0.8, 1.0], [0.0, 0.05, 0.68, 0.97])
        for t, s in ((0.085, 0.0), (0.135, 0.05), (0.8, 0.68), (1.0, 0.97)):
            self.assertAlmostEqual(m.to_source(t), s, places=9)
            self.assertAlmostEqual(m.to_target(s), t, places=9)
        y = np.linspace(0.085, 1.0, 17)
        np.testing.assert_allclose(m.to_target(m.to_source(y)), y, atol=1e-12)


class RingsTest(unittest.TestCase):
    def test_the_knee_has_edge_loops_close_together(self):
        heights = np.array(rj.LEG_RINGS)
        self.assertTrue(np.all(np.diff(heights) < 0), "from the crotch down")
        knee = heights[(heights > rj.KNEE - 0.06) & (heights < rj.KNEE + 0.06)]
        self.assertGreaterEqual(len(knee), 7, "rings round the knee")
        self.assertLessEqual(np.max(-np.diff(knee)), 0.016)
        # Elsewhere on the thigh the rings are farther apart.
        thigh = heights[(heights > 0.6) & (heights < 0.75)]
        self.assertGreater(np.min(-np.diff(thigh)), 0.016)

    def test_the_cuff_and_the_waistband_sit_where_they_should(self):
        self.assertAlmostEqual(min(rj.LEG_RINGS), rj.HEM)
        self.assertGreaterEqual(rj.HEM, 0.075, "the hem at the ankle, above the shoe's collar")
        self.assertLessEqual(rj.HEM, 0.10)
        self.assertAlmostEqual(max(rj.PELVIS_RINGS), rj.WAIST_TOP)
        self.assertIn(rj.CUFF_TOP, rj.LEG_RINGS)
        self.assertIn(rj.BAND_BOTTOM, rj.PELVIS_RINGS)

    def test_the_cuff_is_gathered_close_and_the_leg_above_it_is_not(self):
        self.assertLess(rj.ease(rj.HEM + 0.01), rj.ease(0.30))
        self.assertLessEqual(rj.ease(rj.HEM + 0.01), 0.007)
        self.assertGreaterEqual(rj.ease(rj.KNEE), 0.015, "room for the knee to bend")


class ClosedPartsTest(unittest.TestCase):
    def test_a_cord_is_a_closed_tube(self):
        path = np.array([[0, 1, 0.1], [0.002, 0.95, 0.105], [0.004, 0.9, 0.11]], float)
        P, faces = rj.tube(path, 0.0025, sides=6)
        self.assertEqual(open_edges(faces), [])
        self.assertTrue(all(len(f) in (3, 4) for f in faces))

    def test_a_pocket_is_a_closed_pillow_with_a_turned_edge(self):
        front = np.zeros((4, 5, 3))
        for i in range(4):
            for j in range(5):
                front[i, j] = [j * 0.03, 0.8 + i * 0.03, 0.0]
        P, faces, rim = rj.pillow(front, thickness=0.0025, normal=np.array([0, 0, 1.0]))
        self.assertEqual(open_edges(faces), [])
        back = P[len(front.reshape(-1, 3)):]
        self.assertAlmostEqual(float(np.min(front[..., 2]) - np.max(back[:, 2])), 0.0025, places=6)
        self.assertGreater(len(rim), 0)

    def test_a_facing_turns_the_edge_inside_with_thickness(self):
        # A tube's last rings, turned inside by `thickness` with a fold.
        rings = []
        for y in (0.10, 0.12, 0.14, 0.16):
            a = np.linspace(0, 2 * np.pi, 12, endpoint=False)
            rings.append(np.stack([0.06 * np.cos(a), np.full(12, y), 0.06 * np.sin(a)], 1))
        rings = np.array(rings)
        inner, fold = rj.turned(rings, thickness=0.003, centre=lambda y: np.array([0.0, y, 0.0]),
                                fold_out=-1.0)
        r_in = np.linalg.norm(inner[..., [0, 2]], axis=-1)
        np.testing.assert_allclose(r_in, 0.057, atol=1e-9)
        self.assertLess(fold[:, 1].max(), 0.10, "the fold rounds below the hem")


class LoaderTest(unittest.TestCase):
    def test_the_textures_uv_layer_is_read_and_a_webp_map_becomes_png(self):
        import bpy
        from PIL import Image
        import load
        bpy.ops.wm.read_factory_settings(use_empty=True)
        me = bpy.data.meshes.new("m")
        me.from_pydata([(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)], [], [(0, 1, 2, 3)])
        first = me.uv_layers.new(name="UVMap")
        second = me.uv_layers.new(name="UVMap.001")
        for k, loop in enumerate(me.loops):
            first.data[k].uv = (0.1, 0.1)
            second.data[k].uv = [(0, 0), (1, 0), (1, 1), (0, 1)][k]
        me.uv_layers.active = first
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "map.webp")
            Image.new("RGB", (8, 8), (200, 100, 50)).save(path, "WEBP", lossless=True)
            image = bpy.data.images.load(path)
            image.pack()
            mat = bpy.data.materials.new("Cloth")
            mat.use_nodes = True
            nodes = mat.node_tree.nodes
            tex = nodes.new("ShaderNodeTexImage")
            tex.image = image
            uvnode = nodes.new("ShaderNodeUVMap")
            uvnode.uv_map = "UVMap.001"
            mat.node_tree.links.new(uvnode.outputs["UV"], tex.inputs["Vector"])
            bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
            mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
            me.materials.append(mat)
            obj = bpy.data.objects.new("o", me)
            bpy.context.scene.collection.objects.link(obj)
            out = os.path.join(tmp, "textures")
            mesh = load.to_workmesh([obj], out, uv_from="textures")
            np.testing.assert_allclose(mesh.uv, [(0, 0), (1, 0), (1, 1), (0, 1)])
            saved = mesh.materials[0]["base_color"]
            self.assertTrue(saved.endswith(".png"), saved)
            with Image.open(os.path.join(out, saved)) as im:
                self.assertEqual(im.format, "PNG")
                self.assertEqual(im.convert("RGB").getpixel((3, 3)), (200, 100, 50))


if __name__ == "__main__":
    unittest.main()
