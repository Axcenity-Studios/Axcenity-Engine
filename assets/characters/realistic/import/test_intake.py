"""Tests of the licensed-asset intake (Paul's decision 11), on small meshes
made here: the provenance record, the section 2 checks, the weight
transfer and the conform to the rest pose. Blender's Python (bpy with
NumPy, for `mathutils`):

    python -I assets/characters/realistic/import/test_intake.py
"""

import json
import math
import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import numpy as np  # noqa: E402

import checks  # noqa: E402
import conform  # noqa: E402
import provenance  # noqa: E402
import skinning  # noqa: E402
from workmesh import WorkMesh  # noqa: E402


def cube(size=1.0):
    """A closed cube of six quads, outward, with each face mapped onto
    its own sixth of the UV square (no overlaps)."""
    s = size / 2
    P = np.array([[-s, -s, -s], [s, -s, -s], [s, s, -s], [-s, s, -s],
                  [-s, -s, s], [s, -s, s], [s, s, s], [-s, s, s]], float)
    faces = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4), (2, 3, 7, 6), (1, 2, 6, 5), (0, 4, 7, 3)]
    uv = []
    for k in range(6):
        u0, v0 = (k % 3) / 3, (k // 3) / 2
        uv += [(u0, v0), (u0 + 1 / 3, v0), (u0 + 1 / 3, v0 + 0.5), (u0, v0 + 0.5)]
    return WorkMesh.from_faces(P, faces, uv=np.array(uv) * 0.98 + 0.005)


def plane(width=1.0, depth=1.0, n=4, uv_scale=(1.0, 1.0)):
    """A flat grid of n x n quads in X and Z (Y up), mapped onto the UV
    square scaled by `uv_scale`."""
    xs = np.linspace(-width / 2, width / 2, n + 1)
    zs = np.linspace(-depth / 2, depth / 2, n + 1)
    P = np.array([[x, 0.0, z] for z in zs for x in xs])
    faces, uv = [], []
    for j in range(n):
        for i in range(n):
            a = j * (n + 1) + i
            quad = (a, a + n + 1, a + n + 2, a + 1)
            faces.append(quad)
            for k in quad:
                uv.append(((k % (n + 1)) / n * uv_scale[0], (k // (n + 1)) / n * uv_scale[1]))
    return WorkMesh.from_faces(P, faces, uv=np.array(uv))


class ProvenanceTest(unittest.TestCase):
    def spec(self, licence="CC-BY-4.0", attribution=None):
        return {
            "asset": "Bucket Hat", "source_page": "https://example.org/hat", "author": "A. Modeller",
            "licence": licence, "retrieved": "2026-11-18", "items": ["imported/bucket-hat"],
            "evidence": "The page says CC BY 4.0", "attribution": attribution,
        }

    def test_hashes_every_file_as_downloaded(self):
        with tempfile.TemporaryDirectory() as folder:
            with open(os.path.join(folder, "hat.glb"), "wb") as f:
                f.write(b"abc")
            files = provenance.hash_files(folder, ["hat.glb"])
        self.assertEqual(files, [{
            "name": "hat.glb", "bytes": 3,
            "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}])

    def test_refuses_share_alike_non_commercial_no_derivatives_and_unknown(self):
        for licence, why in [("CC-BY-SA-4.0", "share-alike"), ("CC-BY-NC-4.0", "non-commercial"),
                             ("CC-BY-ND-4.0", "no-derivatives"), ("Custom", "isn't one"),
                             ("", "unknown")]:
            with self.assertRaises(provenance.Refused) as refused:
                provenance.terms(licence)
            self.assertIn(why, str(refused.exception), licence)
        self.assertEqual(provenance.terms("CC0-1.0"), "public_domain")
        self.assertEqual(provenance.terms("CC-BY-4.0"), "attribution")

    def test_a_cc_by_record_gets_its_attribution_line(self):
        record = provenance.make_record(self.spec(), [{"name": "hat.glb", "sha256": "a" * 64, "bytes": 1}],
                                        ["Fitted to the realistic body"])
        line = record["attribution"]
        for part in ["Bucket Hat", "A. Modeller", "https://example.org/hat", "CC BY 4.0",
                     "https://creativecommons.org/licenses/by/4.0/", "changed"]:
            self.assertIn(part, line)
        self.assertEqual(record["licence"], {"id": "CC-BY-4.0",
                                             "url": "https://creativecommons.org/licenses/by/4.0/"})
        self.assertEqual(provenance.problems(record), [])
        # The engine's own schema: these keys, nothing else.
        self.assertEqual(set(record), {"format_version", "asset", "source_page", "author", "licence",
                                       "retrieved", "files", "changes", "attribution", "items", "evidence"})

    def test_a_given_line_is_kept_exactly(self):
        given = "Hat by A. Modeller, CC BY 4.0, changed"
        record = provenance.make_record(self.spec(attribution=given), [{"name": "x", "sha256": "a" * 64}],
                                        ["Fitted"])
        self.assertEqual(record["attribution"], given)

    def test_a_cc0_record_needs_no_line_but_says_what_changed(self):
        record = provenance.make_record(self.spec("CC0-1.0"), [{"name": "x", "sha256": "b" * 64}], [])
        self.assertEqual(provenance.problems(record), ["changes: an imported asset is always changed; say how"])
        record["changes"] = ["Fitted"]
        self.assertEqual(provenance.problems(record), [])

    def test_a_record_with_holes_says_where(self):
        record = provenance.make_record(self.spec("CC0-1.0"), [{"name": "x", "sha256": "xyz"}], ["Fitted"])
        record["source_page"] = "example.org"
        record["retrieved"] = "yesterday"
        found = " ".join(provenance.problems(record))
        for where in ["source_page", "retrieved", "files[0].sha256"]:
            self.assertIn(where, found)


class GeometryTest(unittest.TestCase):
    def test_a_closed_cube_is_sound(self):
        report = checks.geometry(cube())
        self.assertEqual(report["faces"], 6)
        self.assertEqual(report["quads"], 6)
        self.assertEqual(report["quad_share"], 1.0)
        self.assertEqual(report["triangles"], 12)
        for key in ("degenerate", "non_manifold_edges", "flipped_edges", "boundary_edges", "ngons"):
            self.assertEqual(report[key], 0, key)

    def test_a_flipped_face_is_found(self):
        mesh = cube()
        mesh.reverse_face(1)
        report = checks.geometry(mesh)
        self.assertEqual(report["flipped_edges"], 4)

    def test_degenerate_faces_ngons_and_open_edges(self):
        P = np.array([[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0], [0.5, 1.5, 0], [2, 0, 0], [3, 0, 0]], float)
        mesh = WorkMesh.from_faces(P, [(0, 1, 2, 4, 3), (1, 5, 6)])
        report = checks.geometry(mesh)
        self.assertEqual(report["ngons"], 1)
        self.assertEqual(report["degenerate"], 1)
        self.assertGreater(report["boundary_edges"], 0)

    def test_three_faces_on_an_edge_is_non_manifold(self):
        P = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1]], float)
        mesh = WorkMesh.from_faces(P, [(0, 1, 2), (1, 0, 3), (0, 1, 4)])
        self.assertEqual(checks.geometry(mesh)["non_manifold_edges"], 1)

    def test_split_seams_are_welded_before_counting(self):
        # A glTF file splits vertices at UV seams: the cube's 24 corners
        # are still a closed cube.
        mesh = cube().split_corners()
        self.assertEqual(len(mesh.P), 24)
        report = checks.geometry(mesh)
        self.assertEqual(report["boundary_edges"], 0)
        self.assertEqual(report["non_manifold_edges"], 0)


class UvTest(unittest.TestCase):
    def test_density_overlap_and_stretch_of_a_square_metre(self):
        report = checks.uvs(plane(1.0, 1.0, n=8), size=2048)
        # 2048 px across 100 cm.
        self.assertAlmostEqual(report["texel_density_px_per_cm"]["median"], 20.48, places=2)
        self.assertEqual(report["overlap_share"], 0.0)
        self.assertEqual(report["stretched_share"], 0.0)

    def test_overlap_is_found(self):
        mesh = plane(1.0, 1.0, n=2)
        mesh.uv[:] = np.tile([[0.1, 0.1], [0.1, 0.9], [0.9, 0.9], [0.9, 0.1]], (4, 1))
        self.assertGreater(checks.uvs(mesh)["overlap_share"], 0.9)

    def test_stretch_is_measured(self):
        # Twice as many texels across as along: every face 100% stretched.
        report = checks.uvs(plane(1.0, 1.0, n=4, uv_scale=(1.0, 0.5)))
        self.assertEqual(report["stretched_share"], 1.0)
        self.assertAlmostEqual(report["stretch_p95"], 1.0, places=3)


class SkinningTest(unittest.TestCase):
    """A two-bone 'body': a trunk column on `Spine` and an arm out along
    +X on `LeftUpperArm`, each a box of skin."""

    def body(self):
        trunk = cube(1.0)
        trunk.P = trunk.P * [0.3, 1.0, 0.2]
        arm = cube(1.0)
        arm.P = arm.P * [0.6, 0.1, 0.1] + [0.45, 0.4, 0.0]
        P = np.vstack([trunk.P, arm.P])
        tris = np.vstack([trunk.triangles(), arm.triangles() + len(trunk.P)])
        joints = np.zeros((len(P), 4), int)
        weights = np.zeros((len(P), 4))
        weights[:8, 0] = 1.0           # Spine
        joints[8:, 0] = 1              # LeftUpperArm
        weights[8:, 0] = 1.0
        bones = ["Spine", "LeftUpperArm"]
        heads = {"Spine": np.array([0.0, -0.5, 0.0]), "LeftUpperArm": np.array([0.15, 0.4, 0.0]),
                 "LeftLowerArm": np.array([0.75, 0.4, 0.0])}
        return skinning.Body(P, tris, joints, weights, bones, heads)

    def test_weights_are_four_at_most_and_normalized(self):
        body = self.body()
        points = np.array([[0.0, 0.0, 0.12], [0.5, 0.46, 0.0], [0.17, 0.3, 0.0]])
        joints, weights = skinning.transfer(body, points)
        self.assertEqual(joints.shape, (3, 4))
        np.testing.assert_allclose(weights.sum(axis=1), 1.0, atol=1e-6)
        self.assertEqual(body.bones[joints[1, 0]], "LeftUpperArm")
        self.assertAlmostEqual(weights[1, 0], 1.0)
        self.assertEqual(body.bones[joints[0, 0]], "Spine")

    def test_the_arm_and_the_side_are_kept_apart(self):
        body = self.body()
        # Under the armpit, on the trunk's side but nearer the arm's skin.
        side = np.array([[0.2, 0.33, 0.0]])
        joints, weights = skinning.transfer(body, side)
        self.assertEqual(body.bones[joints[0, 0]], "LeftUpperArm")
        joints, weights = skinning.transfer(body, side, split="arm-side")
        arm = body.bones.index("LeftUpperArm")
        self.assertEqual(weights[0][joints[0] == arm].sum(), 0.0)

    def test_the_skinning_check_counts_what_breaks_the_rules(self):
        joints = np.array([[0, 1, 0, 0], [0, 1, 2, 3]])
        weights = np.array([[0.5, 0.4, 0, 0], [0.4, 0.3, 0.2, 0.1]])
        report = checks.skinning(joints, weights, influences=np.array([2, 5]))
        self.assertEqual(report["most_influences"], 5)
        self.assertEqual(report["over_four"], 1)
        self.assertEqual(report["not_normalized"], 1)


class ConformTest(unittest.TestCase):
    def test_a_t_pose_sleeve_comes_down_to_the_rest_pose(self):
        # The rest pose's arm hangs 45 degrees below level; the sleeve was
        # made with the arm level (a T-pose).
        heads = {"Spine": np.array([0.0, 1.0, 0.0]), "LeftUpperArm": np.array([0.2, 1.4, 0.0]),
                 "LeftLowerArm": np.array([0.2 + 0.3 * math.cos(math.radians(45)),
                                           1.4 - 0.3 * math.sin(math.radians(45)), 0.0])}
        parents = {"Spine": None, "LeftUpperArm": "Spine", "LeftLowerArm": "LeftUpperArm"}
        self.assertAlmostEqual(conform.arm_angle(heads, "Left"), 45.0, places=4)
        pose = conform.arm_pose(heads, parents, authored=0.0)
        point = np.array([[0.5, 1.4, 0.0]])      # 0.3 m out along a level arm
        joints = np.array([[1, 0, 0, 0]])
        weights = np.array([[1.0, 0, 0, 0]])
        bones = ["Spine", "LeftUpperArm", "LeftLowerArm"]
        rest = conform.to_rest(point, joints, weights, bones, pose)
        expected = heads["LeftUpperArm"] + 0.3 * np.array([math.cos(math.radians(45)), -math.sin(math.radians(45)), 0])
        np.testing.assert_allclose(rest[0], expected, atol=1e-9)
        # The trunk doesn't move.
        still = conform.to_rest(np.array([[0.0, 1.2, 0.1]]), np.array([[0, 0, 0, 0]]), weights, bones, pose)
        np.testing.assert_allclose(still[0], [0.0, 1.2, 0.1], atol=1e-12)


class SettleTest(unittest.TestCase):
    def test_a_hat_comes_down_until_it_sits_its_clearance_off_the_head(self):
        head = cube(0.2)
        head.P = head.P + [0, 1.6, 0]
        ring = plane(0.3, 0.3, n=6)
        ring.P = ring.P + [0, 2.0, 0]
        ring.P = ring.P[[True] * len(ring.P)]
        drop = checks.settle(ring.P, head.P, head.triangles(), direction=(0, -1, 0), clearance=0.004)
        # The plate stops 4 mm over the head's top at 1.7 m.
        self.assertAlmostEqual(2.0 - drop, 1.704, places=3)


class DistanceTest(unittest.TestCase):
    def test_a_copy_lies_on_itself(self):
        mesh = cube()
        d = checks.distances(mesh.P, mesh.P, mesh.triangles())
        self.assertLess(float(np.max(np.abs(d))), 1e-9)


if __name__ == "__main__":
    unittest.main(verbosity=1)
