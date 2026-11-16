"""Tests for `pseudo_normals.py`: `python3 -m unittest test_pseudo_normals`
in this folder (needs numpy, not Blender)."""

import unittest

import numpy as np

from pseudo_normals import PseudoNormals


def tetra():
    """A closed tetrahedron, outward-facing, with a sharp edge between
    vertices 0 and 1 (faces 0 and 1 share it)."""
    co = np.array([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0], [0.5, 0.3, 1.0]])
    tris = np.array([[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]])
    return co, tris


class SignAtEdges(unittest.TestCase):
    def test_a_point_outside_the_edge_is_outside_whichever_face_was_returned(self):
        co, tris = tetra()
        pn = PseudoNormals(co, tris)
        loc = np.array([0.5, 0.0, 0.0])  # middle of the edge 0-1
        # the faces' normals are over 90 degrees apart (a sharp edge, as
        # at a lid's margin); a point outside, between them and close to
        # face 1's normal, is behind face 0's plane
        n0, n1 = pn.face[0], pn.face[1]
        self.assertLess(n0 @ n1, 0.0)
        p = loc + 0.01 * (0.05 * n0 + n1)
        # one face's own normal gets this wrong (the bug this replaces)
        signs = {face: float(np.sign((p - loc) @ pn.face[face])) for face in (0, 1)}
        self.assertIn(-1.0, signs.values())
        for face in (0, 1):
            self.assertGreater(pn.signed(p, face), 0.0)

    def test_a_point_inside_near_the_edge_is_inside(self):
        co, tris = tetra()
        pn = PseudoNormals(co, tris)
        loc = np.array([0.5, 0.0, 0.0])
        p = np.array([0.5, 0.005, 0.005])
        for face in (0, 1):
            self.assertLess(pn.signed(p, face), 0.0)

    def test_at_a_corner_the_sign_is_right_and_the_magnitude_is_the_distance(self):
        co, tris = tetra()
        pn = PseudoNormals(co, tris)
        loc = co[3]
        p = loc + np.array([0.0, 0.0, 0.02])  # straight above the apex
        for face in (1, 2, 3):
            self.assertAlmostEqual(pn.signed(p, face), 0.02)
        q = loc + np.array([0.0, 0.0, -0.02])
        self.assertLess(pn.signed(q, 1), 0.0)

    def test_inside_a_face_it_is_the_plane_distance(self):
        co, tris = tetra()
        pn = PseudoNormals(co, tris)
        loc = co[tris[0]].mean(axis=0)  # centre of the bottom face
        self.assertAlmostEqual(pn.signed(loc + [0, 0, -0.03], 0), 0.03)
        self.assertAlmostEqual(pn.signed(loc + [0, 0, 0.03], 0), -0.03)

    def test_a_point_beside_a_faces_edge_is_measured_to_the_edge_not_the_plane(self):
        # The Stage 3 diagnostic: the tree's nearest point (single
        # precision) sat on an edge, a tolerance read it as inside the
        # face, and a point 2.3 mm outside the lid read 0.06 mm inside by
        # the face's plane. The region is now worked out exactly.
        co, tris = tetra()
        pn = PseudoNormals(co, tris)
        edge_mid = np.array([0.5, 0.0, 0.0])
        p = edge_mid + 0.01 * (0.05 * pn.face[0] + pn.face[1])
        plane = (p - edge_mid) @ pn.face[0]
        self.assertLess(plane, 0.0)  # behind face 0's plane, yet outside
        self.assertAlmostEqual(pn.signed(p, 0), float(np.linalg.norm(p - edge_mid)))

    def test_a_vertex_split_at_a_seam_still_finds_both_faces(self):
        co, tris = tetra()
        # face 1 gets its own copies of vertices 0 and 1, as a UV seam does
        co = np.vstack([co, co[0], co[1]])
        tris = tris.copy()
        tris[1] = [4, 5, 3]
        pn = PseudoNormals(co, tris)
        loc = np.array([0.5, 0.0, 0.0])
        p = loc + 0.01 * (0.05 * pn.face[0] + pn.face[1])
        for face in (0, 1):
            self.assertGreater(pn.signed(p, face), 0.0)


if __name__ == "__main__":
    unittest.main()
