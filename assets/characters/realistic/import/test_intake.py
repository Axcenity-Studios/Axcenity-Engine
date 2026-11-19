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

    # Review fixes 1 (2026-11-18): the licence's own text, the notice MIT
    # and Apache-2.0 ask for, and the licence checked before any file.

    def test_the_licence_url_is_the_licences_own_text(self):
        spec = self.spec("CC0-1.0")
        spec["licence_url"] = "https://polyhaven.com/license"
        record = provenance.make_record(spec, [{"name": "x", "sha256": "b" * 64}], ["Fitted"])
        self.assertEqual(record["licence"]["url"], "https://creativecommons.org/publicdomain/zero/1.0/")
        self.assertEqual(provenance.problems(record), [])
        record["licence"]["url"] = "https://polyhaven.com/license"
        self.assertIn("licence.url", " ".join(provenance.problems(record)))
        record["licence"]["url"] = "http://creativecommons.org/publicdomain/zero/1.0/legalcode.en"
        self.assertEqual(provenance.problems(record), [])

    def test_mit_and_apache_records_carry_their_notice(self):
        for licence in ("MIT", "Apache-2.0"):
            record = provenance.make_record(self.spec(licence), [{"name": "x", "sha256": "c" * 64}],
                                            ["Fitted"])
            self.assertIn("notice", " ".join(provenance.problems(record)), licence)
        mit = provenance.make_record(self.spec("MIT"), [{"name": "x", "sha256": "c" * 64}], ["Fitted"],
                                     notice="Copyright (c) 2026 A. Modeller")
        self.assertIn("permission", " ".join(provenance.problems(mit)))
        mit["notice"] = "Copyright (c) 2026 A. Modeller\n\nPermission is hereby granted, free of charge..."
        self.assertEqual(provenance.problems(mit), [])

    def test_the_notice_is_read_from_its_downloaded_file(self):
        with tempfile.TemporaryDirectory() as downloads, tempfile.TemporaryDirectory() as work:
            for name, text in (("hat.glb", "glb"), ("LICENSE", "Copyright 2026 A. Modeller\n"
                                                    "Permission is hereby granted, free of charge\n")):
                with open(os.path.join(downloads, name), "w", encoding="utf-8") as f:
                    f.write(text)
            spec = {"item": {"id": "imported/bucket-hat"},
                    "source": dict(self.spec("MIT"), files=["hat.glb", "LICENSE"], model="hat.glb",
                                   notice_file="LICENSE")}
            provenance.step(spec, downloads, work)
            with open(os.path.join(work, "provenance.json"), encoding="utf-8") as f:
                record = json.load(f)
        self.assertTrue(record["notice"].startswith("Copyright 2026 A. Modeller"))
        self.assertIn("Permission is hereby granted", record["notice"])

    def test_a_refused_licence_stops_before_any_file_is_read(self):
        spec = {"item": {"id": "imported/bucket-hat"},
                "source": dict(self.spec("CC-BY-NC-4.0"), files=["missing.glb"], model="missing.glb")}
        with tempfile.TemporaryDirectory() as work:
            # The downloads folder doesn't exist: hashing would fail first.
            with self.assertRaises(provenance.Refused):
                provenance.step(spec, os.path.join(work, "no-such-folder"), work)

    def test_the_model_is_one_of_the_hashed_files(self):
        spec = {"item": {"id": "imported/bucket-hat"},
                "source": dict(self.spec("CC0-1.0"), files=["hat.bin"], model="hat.gltf")}
        with tempfile.TemporaryDirectory() as work:
            with self.assertRaises(SystemExit) as stopped:
                provenance.step(spec, work, work)
        self.assertIn("hat.gltf", str(stopped.exception))


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

    def test_a_vertex_weighted_past_the_next_joint_fails(self):
        # Review fixes 1: section 2's "no vertex weighted to a bone more
        # than one joint away" is a pass/fail rule (it is what would have
        # caught the hat's brim tips bound to the shoulders), judged on the
        # shipped weights; a rigid item is on its one bone by design.
        bones = ["Spine", "Neck", "Head", "LeftShoulder"]
        heads = {"Spine": np.array([0.0, 1.2, 0.0]), "Neck": np.array([0.0, 1.5, 0.0]),
                 "Head": np.array([0.0, 1.6, 0.0]), "LeftShoulder": np.array([0.1, 1.45, 0.0])}
        parents = {"Spine": None, "Neck": "Spine", "Head": "Neck", "LeftShoulder": "Spine"}
        P = np.array([[0.0, 1.75, 0.0], [0.0, 1.75, 0.1]])     # both at the head
        joints = np.array([[2, 0, 0, 0], [3, 0, 0, 0]])         # the second on the shoulder
        weights = np.array([[1.0, 0, 0, 0], [1.0, 0, 0, 0]])
        report = checks.skinning(joints, weights)
        report["far_from_their_bone"] = checks.far_weights(P, joints, weights, bones, heads, parents)
        self.assertEqual(report["far_from_their_bone"], 1)
        self.assertFalse(checks.skin_limits(report)["no_weight_past_the_next_joint"])
        self.assertNotIn("no_weight_past_the_next_joint", checks.skin_limits(report, rigid_bone="Head"))
        report["far_from_their_bone"] = 0
        self.assertTrue(all(checks.skin_limits(report).values()))
        limits = checks.limits_report("garment", {"degenerate": 0, "non_manifold_edges": 0,
                                                  "flipped_edges": 0}, None, dict(report, far_from_their_bone=3))
        self.assertFalse(limits["no_weight_past_the_next_joint"])


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


def round_tube(radius, y0, y1, sides=24, rings=9, closed=False):
    """A tube of quads round Y (outward), from y0 to y1; with `closed`, a
    fan at each end."""
    P = []
    for j in range(rings):
        y = y0 + (y1 - y0) * j / (rings - 1)
        for i in range(sides):
            a = 2 * math.pi * i / sides
            P.append([radius * math.cos(a), y, radius * math.sin(a)])
    faces = []
    for j in range(rings - 1):
        for i in range(sides):
            a, b = j * sides + i, j * sides + (i + 1) % sides
            faces.append((a, a + sides, b + sides, b))
    if closed:
        for y, ring, flip in ((y0, 0, True), (y1, rings - 1, False)):
            P.append([0.0, y, 0.0])
            c = len(P) - 1
            for i in range(sides):
                a, b = ring * sides + i, ring * sides + (i + 1) % sides
                faces.append((c, b, a) if flip else (c, a, b))
    return WorkMesh.from_faces(np.array(P, float), faces)


class DrapeTest(unittest.TestCase):
    """Wave 5 stage 3 (`refinement/wave5/fit-drape.md`): a garment is graded
    to the body by measured girths, then draped onto it by Blender's cloth
    simulation, hanging from its pins, keeping its ease."""

    def test_a_girth_is_the_slice_round_the_outside(self):
        body = round_tube(0.15, 0.0, 1.0, sides=48)
        self.assertAlmostEqual(conform.girth(body.P, 0.5, 0.02), 2 * math.pi * 0.15, delta=0.005)

    def test_a_small_garment_is_graded_to_its_least_ease(self):
        body = round_tube(0.15, 0.0, 1.0, sides=48, closed=True)
        shirt = round_tube(0.14, 0.4, 0.9, sides=48, rings=11)
        report = conform.grade(shirt, body, {"at": [0.5, 0.8], "least_ease": 0.06})
        want = 2 * math.pi * 0.15 + 0.06
        self.assertAlmostEqual(conform.girth(shirt.P, 0.5, 0.02), want, delta=0.006)
        self.assertGreater(report["scale"], 1.1)
        # A garment with enough ease isn't touched.
        roomy = round_tube(0.2, 0.4, 0.9, sides=48, rings=11)
        before = roomy.P.copy()
        report = conform.grade(roomy, body, {"at": [0.5], "least_ease": 0.06})
        self.assertEqual(report["scale"], 1.0)
        np.testing.assert_allclose(roomy.P, before)

    def test_a_loose_tube_hangs_from_its_pins_and_keeps_its_ease(self):
        body = round_tube(0.15, 0.0, 1.2, sides=32, rings=13, closed=True)
        # A loose tube round it, its top pinned: it hangs, it doesn't
        # stretch down, it isn't drawn onto the body, and nothing of it is
        # inside the body.
        shirt = round_tube(0.2, 0.5, 1.0, sides=32, rings=11)
        report = conform.drape(shirt, body, {"pin_top": 0.06, "settings": {"frames": 30, "quality": 5}})
        bottom = shirt.P[:32]
        self.assertGreater(bottom[:, 1].min(), 0.47)
        radius = np.hypot(shirt.P[:, 0], shirt.P[:, 2])
        self.assertGreater(np.median(radius), 0.18)
        self.assertGreater(radius.min(), 0.15)
        self.assertEqual(report["pinned"], 64)
        self.assertIn("moved_m", report)

    def test_the_cloth_hangs_over_the_layers_it_is_worn_over(self):
        """Review fixes 2: draped against the body alone, the tee's hem fell
        into the hollow the underwear bridges (the cleft), and the engine's
        fit then pushed it out over the underwear, pulling the centre back
        down into a point. Draped over the layers it is worn over too, it
        hangs outside them."""
        body = round_tube(0.15, 0.0, 1.2, sides=32, rings=13, closed=True)
        # Underwear: a band from 0.4 to 0.7 m standing 2 cm off the skin.
        under = round_tube(0.17, 0.4, 0.7, sides=32, rings=7)
        # A tube 1.5 cm off the skin, its lower part inside the band.
        shirt = round_tube(0.165, 0.5, 1.0, sides=32, rings=11)
        report = conform.drape(shirt, body, {"pin_top": 0.06, "settings": {"frames": 30, "quality": 5}},
                               under=[under])
        low = shirt.P[shirt.P[:, 1] < 0.68]
        self.assertGreater(len(low), 32)
        # Every corner over the band is outside it (its collision margin
        # less a little), none left between it and the skin.
        self.assertGreater(np.hypot(low[:, 0], low[:, 2]).min(), 0.17 - 0.001)
        self.assertEqual(report["clearance_m"]["inside_layers"], 0)
        self.assertGreater(report["pushed_out_first"]["pushed"], 0)


def axis_tube(radius, origin, axis, t0, t1, sides=24, rings=8, closed=False):
    """A tube of quads round `axis` (a unit vector through `origin`), from
    t0 to t1 along it (an arm, or a sleeve round it)."""
    axis = np.asarray(axis, float) / np.linalg.norm(axis)
    u = np.cross(axis, [0.0, 0.0, 1.0])
    u = u / np.linalg.norm(u)
    v = np.cross(axis, u)
    tube = round_tube(radius, t0, t1, sides=sides, rings=rings, closed=closed)
    # round_tube runs along Y with X and Z round it: map Y to the axis.
    local = tube.P
    tube.P = (np.asarray(origin, float) + np.outer(local[:, 1], axis)
              + np.outer(local[:, 0], u) + np.outer(local[:, 2], v))
    return tube


def ring_girth(points, origin, axis, t, band=0.006):
    """The girth round `points` across `axis` at `t` along it."""
    axis = np.asarray(axis, float) / np.linalg.norm(axis)
    r = np.asarray(points, float) - origin
    along = r @ axis
    sel = np.abs(along - t) <= band
    u = np.cross(axis, [0.0, 0.0, 1.0])
    u = u / np.linalg.norm(u)
    v = np.cross(axis, u)
    return conform._hull_perimeter(np.stack([r[sel] @ u, r[sel] @ v], axis=1))


def joined(*meshes):
    """Meshes as one (the body's trunk and arm)."""
    P, faces = [], []
    for m in meshes:
        faces += [[int(k) + len(P) for k in f] for f in m.faces()]
        P += m.P.tolist()
    return WorkMesh.from_faces(np.array(P), faces)


class GradeReviewTest(unittest.TestCase):
    """Wave 5 stage 3, review fixes 1: the grade replaces the push-out
    where the garment is too small (sleeves and shoulders too, not only the
    torso), and the drape keeps the neck band where the grade put it."""

    SHOULDER = np.array([0.16, 1.38, 0.0])
    ARM = np.array([math.cos(math.radians(45)), -math.sin(math.radians(45)), 0.0])

    def body(self):
        trunk = round_tube(0.12, 0.9, 1.42, sides=32, rings=14, closed=True)
        arm = axis_tube(0.05, self.SHOULDER, self.ARM, -0.02, 0.3, sides=24, rings=12, closed=True)
        body = joined(trunk, arm)
        body.heads = {"LeftUpperArm": self.SHOULDER, "LeftLowerArm": self.SHOULDER + 0.3 * self.ARM}
        # The arm's skin is the arm's (its weights), the trunk's isn't.
        body.armness = {"Left": np.r_[np.zeros(len(trunk.P)), np.ones(len(arm.P))]}
        return body

    def test_a_small_sleeve_is_graded_round_its_arm(self):
        body = self.body()
        # A sleeve too small for the arm and 1 cm off its axis (towards the
        # armpit), and a patch of the side below the armpit.
        off = np.array([-0.007, -0.007, 0.0])
        sleeve = axis_tube(0.042, self.SHOULDER + off, self.ARM, 0.06, 0.2, sides=24, rings=8)
        side = round_tube(0.135, 1.05, 1.15, sides=32, rings=3)
        item = joined(sleeve, side)
        n_sleeve = len(sleeve.P)
        before = item.P.copy()
        report = conform.grade(item, body, {"at": [], "sleeves": {"least_ease": 0.04}})
        P = item.P[:n_sleeve]
        # Round the arm, not through it: every corner the ease's share of
        # radius (4 cm / 2 pi, 6.4 mm) off the arm's skin, or nearly (the
        # arm is a 24-sided tube), and the girth its ease over the arm's,
        # not much more (the side that was off the arm moves least).
        r = P - self.SHOULDER
        radial = np.linalg.norm(r - np.outer(r @ self.ARM, self.ARM), axis=1)
        self.assertGreater(radial.min(), 0.05 + 0.9 * 0.04 / (2 * math.pi))
        want = 2 * math.pi * 0.05 + 0.04
        for t in (0.1, 0.14, 0.18):
            g = ring_girth(P, self.SHOULDER, self.ARM, t)
            self.assertGreater(g, want - 0.008)
            self.assertLess(g, want + 0.03)
        self.assertGreater(report["sleeves"]["Left"]["scale"], 1.1)
        # The side below the armpit isn't moved by the sleeve's grade.
        np.testing.assert_allclose(item.P[n_sleeve:], before[n_sleeve:], atol=1e-9)

    def test_the_neck_keeps_the_grade_of_the_highest_height(self):
        body = round_tube(0.15, 0.0, 1.0, sides=48, rings=11, closed=True)
        shirt = round_tube(0.14, 0.4, 0.9, sides=48, rings=11)
        conform.grade(shirt, body, {"at": [0.5, 0.8], "least_ease": 0.06})
        # The top (the neck) isn't let back to its source's size, inside
        # the body: it keeps the scale of the highest height graded.
        want = 2 * math.pi * 0.15 + 0.06
        self.assertAlmostEqual(conform.girth(shirt.P, 0.9, 0.02), want, delta=0.006)

    def test_a_height_can_ask_its_own_ease(self):
        body = round_tube(0.15, 0.0, 1.0, sides=48, rings=11, closed=True)
        shirt = round_tube(0.14, 0.4, 0.9, sides=48, rings=11)
        report = conform.grade(shirt, body, {"at": [[0.5, 0.1], [0.8, 0.02]], "least_ease": 0.06})
        self.assertAlmostEqual(conform.girth(shirt.P, 0.5, 0.02), 2 * math.pi * 0.15 + 0.1, delta=0.006)
        self.assertAlmostEqual(conform.girth(shirt.P, 0.8, 0.02), 2 * math.pi * 0.15 + 0.02, delta=0.006)
        self.assertEqual([row["least_ease_m"] for row in report["girths"]], [0.1, 0.02])

    def test_what_is_left_to_push_out_moves_as_one_smooth_patch(self):
        """The corners still inside after the grade move out as a smooth
        field over the cloth (their neighbours carried along), not each on
        its own along the body's normal: one row pushed out and the next
        not is the step that notched the stage-3 neckline."""
        body = round_tube(0.15, 0.0, 1.2, sides=48, rings=13, closed=True)
        # A tube set 3 cm to one side: up to 2 cm inside the body there.
        shirt = round_tube(0.16, 0.5, 1.0, sides=48, rings=21)
        shirt.P[:, 0] += 0.03
        edges = np.array(sorted({(min(a, b), max(a, b)) for f in shirt.faces()
                                 for a, b in zip(f, np.roll(f, -1))}))

        def lengths(P):
            return np.linalg.norm(P[edges[:, 0]] - P[edges[:, 1]], axis=1)

        before = lengths(shirt.P)
        plain = WorkMesh.from_faces(shirt.P.copy(), shirt.faces())
        hard = conform.push_out(plain, body, 0.004)
        report = conform.push_out(shirt, body, 0.004, rounds=30)
        self.assertEqual(report["pushed"], hard["pushed"])
        self.assertGreater(report["carried"], 0)
        # Clear of the body everywhere,
        self.assertGreater(np.hypot(shirt.P[:, 0], shirt.P[:, 2]).min(), 0.15 + 0.0035)
        # and the cloth keeps its lengths (the drape's springs' rest
        # lengths) far better: the hard push stretches an edge by 30%
        # where a pushed corner meets a free one.
        self.assertGreater(np.abs(lengths(plain.P) / before - 1).max(), 0.25)
        self.assertLess(np.abs(lengths(shirt.P) / before - 1).max(), 0.2)

    def test_a_jagged_free_edge_is_relaxed_along_itself(self):
        """A free hem the simulation left in a zig-zag (the review fixes 1
        sleeve hems: corners 10 to 15 mm off their neighbours' line) is
        relaxed along its own loop; the cloth away from the edge is left
        where the drape put it."""
        shirt = round_tube(0.2, 0.5, 1.0, sides=32, rings=11)
        bottom = np.arange(32)
        # Every other corner of the lowest ring 1.5 cm lower: a zig-zag.
        shirt.P[bottom[::2], 1] -= 0.015
        inner = shirt.P[32:320].copy()
        report = conform.relax_edges(shirt, rounds=6)
        y = shirt.P[bottom, 1]
        self.assertLess(y.max() - y.min(), 0.003)
        np.testing.assert_allclose(shirt.P[32:320], inner)
        # A smooth edge (the top ring) isn't drawn in: within 1 mm of its
        # radius (plain averaging would draw it 11 mm in).
        r = np.hypot(shirt.P[320:, 0], shirt.P[320:, 2])
        self.assertLess(np.abs(r - 0.2).max(), 0.001)
        self.assertEqual(report["corners"], 64)
        self.assertGreater(report["most_before_mm"], 7.0)
        # What is left is the round loops' own sag between corners (3.9 mm).
        self.assertLess(report["most_after_mm"], 4.5)

    def test_the_whole_neck_band_is_pinned_where_it_was_graded(self):
        body = round_tube(0.15, 0.0, 1.2, sides=32, rings=13, closed=True)
        shirt = round_tube(0.2, 0.5, 1.0, sides=32, rings=11)
        # A scooped neck: the front of the top ring 6 cm lower than the back.
        top = np.arange(10 * 32, 11 * 32)
        shirt.P[top, 1] -= 0.03 * (1 + shirt.P[top, 2] / 0.2)
        before = shirt.P.copy()
        report = conform.drape(shirt, body, {"pin_top": 0.01, "pin_band": 0.03,
                                             "settings": {"frames": 20, "quality": 5}})
        # Every corner of the neck's open edge is held, the front too.
        np.testing.assert_allclose(shirt.P[top], before[top], atol=1e-4)
        self.assertGreaterEqual(report["pinned"], 32)
        self.assertEqual(report["pushed_out_first"]["pushed"], 0)


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


def tube(radius=0.1, height=0.3, around=24, rings=6):
    """An open tube round Y (a sleeve): quads, outward, mapped onto the
    lower half of the UV square (the upper half left free)."""
    P, faces, uv = [], [], []
    for j in range(rings + 1):
        for i in range(around):
            a = 2 * math.pi * i / around
            P.append([radius * math.cos(a), height * j / rings, radius * math.sin(a)])
    for j in range(rings):
        for i in range(around):
            a, b = j * around + i, j * around + (i + 1) % around
            faces.append((a, a + around, b + around, b))
            u0, u1 = i / around * 0.9, (i + 1) / around * 0.9
            v0, v1 = j / rings * 0.4, (j + 1) / rings * 0.4
            uv += [(u0, v0), (u0, v1), (u1, v1), (u1, v0)]
    return WorkMesh.from_faces(np.array(P), faces, uv=np.array(uv))


class RefineTest(unittest.TestCase):
    def test_a_turned_hem_has_its_thickness_and_its_own_texture_space(self):
        """Wave 5: an open edge turned under is a fold `thickness` thick
        that lies back up inside for `turn_up`; its strip takes free texture
        space (no overlap) and the edge is no longer a single layer."""
        import refine
        mesh = tube()
        before = checks.geometry(mesh)["boundary_edges"]
        report, edges = refine.hems(mesh, [{"loop": "lowest", "thickness": 0.003, "turn_up": 0.018,
                                            "rows": 3}])
        self.assertEqual(report[0]["corners"], 24)
        self.assertEqual(len(edges), 1)
        # The inner layer: 3 mm inside the cloth all round, up to 18 mm.
        r = np.linalg.norm(mesh.P[:, [0, 2]], axis=1)
        inner = mesh.P[r < 0.0995]
        self.assertAlmostEqual(float(r[r < 0.0995].mean()), 0.097, places=3)
        self.assertAlmostEqual(float(inner[:, 1].max()), 0.018, places=3)
        # The fold reaches 1.5 mm below the old edge.
        self.assertAlmostEqual(float(mesh.P[:, 1].min()), -0.0015, places=4)
        # Still one open loop at the top and one (the turn-up's end) inside.
        self.assertEqual(checks.geometry(mesh)["boundary_edges"], before)
        self.assertEqual(checks.uvs(mesh)["overlap_share"], 0.0)

    def test_an_unwrap_takes_the_stretch_out_of_the_texture(self):
        """A tube mapped with its texture squashed to a third along it is
        stretched everywhere; laid out again along its own seams, not."""
        import refine
        mesh = tube()
        mesh.uv[:, 1] *= 0.33
        self.assertGreater(checks.uvs(mesh)["stretched_share"], 0.9)
        out, _ = refine.unwrap(mesh, {"method": "CONFORMAL", "margin": 0.01})
        self.assertLess(checks.uvs(out)["stretched_share"], 0.05)

    def test_the_record_lists_the_unwrap(self):
        """Wave 5 review: the provenance record lists every change the
        refine step made, the texture coordinates laid out again too
        (asset-quality.md section 2: 'what was changed')."""
        import fit
        with tempfile.TemporaryDirectory() as work:
            with open(os.path.join(work, "refined.json"), "w", encoding="utf-8") as f:
                json.dump({"unwrap": {"method": "ANGLE_BASED", "islands_kept": True}}, f)
            open(os.path.join(work, "refined.npz"), "wb").close()
            changes = fit.changes_of({}, work)
        unwrap = [c for c in changes if "texture coordinates" in c]
        self.assertEqual(len(unwrap), 1, changes)
        self.assertIn("angle-based", unwrap[0])
        self.assertIn("the source's own seams", unwrap[0])


class LoadTest(unittest.TestCase):
    def test_a_packed_webp_map_is_saved_as_a_png(self):
        """A glTF may pack its maps as WebP (BlenderKit's do); the engine
        reads PNG and JPEG, so the copy must be a PNG, not WebP bytes
        under a `.png` name (wave 5, the licensed tee)."""
        import bpy

        import load
        bpy.ops.wm.read_factory_settings(use_empty=True)
        with tempfile.TemporaryDirectory() as folder:
            image = bpy.data.images.new("Map", 8, 8)
            image.pixels.foreach_set(np.tile([0.2, 0.4, 0.6, 1.0], 64).astype(np.float32))
            image.file_format = "WEBP"
            image.filepath_raw = os.path.join(folder, "map.webp")
            image.save()
            image.pack()
            saved = load._save_image(image, os.path.join(folder, "out"), "Map_base_color")
            self.assertEqual(saved, "Map_base_color.png")
            with open(os.path.join(folder, "out", saved), "rb") as f:
                self.assertEqual(f.read(8), b"\x89PNG\r\n\x1a\n")
            bpy.data.images.remove(image)


class DistanceTest(unittest.TestCase):
    def test_a_copy_lies_on_itself(self):
        mesh = cube()
        d = checks.distances(mesh.P, mesh.P, mesh.triangles())
        self.assertLess(float(np.max(np.abs(d))), 1e-9)


if __name__ == "__main__":
    unittest.main(verbosity=1)
