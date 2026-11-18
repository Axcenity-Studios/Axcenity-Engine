"""The mesh the intake steps pass to each other (NumPy only): positions in
metres in glTF's axes (Y up, the body facing +Z, the engine's space),
polygons as they were authored (quads stay quads until export), texture
coordinates per corner, a material per face and, once weighted, four
joints and weights per vertex on the realistic skeleton.

A step saves it as `<name>.npz` beside a `<name>.json` of what it did;
the next step reads it. Nothing here needs Blender.
"""

import json

import numpy as np


class WorkMesh:
    def __init__(self, P, corners, sizes, uv=None, face_material=None, materials=None):
        self.P = np.asarray(P, dtype=float).reshape(-1, 3)
        # Polygons: `corners` holds each face's vertex indices in turn,
        # `sizes` how many each face has.
        self.corners = np.asarray(corners, dtype=np.int64)
        self.sizes = np.asarray(sizes, dtype=np.int64)
        n = len(self.corners)
        self.uv = np.zeros((n, 2)) if uv is None else np.asarray(uv, dtype=float).reshape(n, 2)
        self.face_material = (np.zeros(len(self.sizes), dtype=np.int64) if face_material is None
                              else np.asarray(face_material, dtype=np.int64))
        # Each material: {"name", and its maps' file names when it has them:
        # "base_color", "normal", "orm"}.
        self.materials = materials or [{"name": "Material"}]
        self.bones = []
        self.joints = None
        self.weights = None

    @classmethod
    def from_faces(cls, P, faces, uv=None, face_material=None, materials=None):
        corners = [k for face in faces for k in face]
        return cls(P, corners, [len(face) for face in faces], uv, face_material, materials)

    # -- faces --------------------------------------------------------------

    @property
    def starts(self):
        return np.concatenate([[0], np.cumsum(self.sizes)[:-1]]).astype(np.int64)

    def faces(self):
        """Each face's vertex indices (a list of arrays)."""
        return [self.corners[s:s + n] for s, n in zip(self.starts, self.sizes)]

    def triangle_corners(self):
        """Each face fanned into triangles, as indices into `corners`
        (so texture coordinates come along)."""
        out = []
        for s, n in zip(self.starts, self.sizes):
            for k in range(1, n - 1):
                out.append((s, s + k, s + k + 1))
        return np.array(out, dtype=np.int64).reshape(-1, 3)

    def triangles(self):
        return self.corners[self.triangle_corners()]

    def triangle_faces(self):
        """The face each fanned triangle belongs to."""
        return np.repeat(np.arange(len(self.sizes)), np.maximum(self.sizes - 2, 0))

    def reverse_face(self, f):
        s, n = self.starts[f], self.sizes[f]
        self.corners[s:s + n] = self.corners[s:s + n][::-1].copy()
        self.uv[s:s + n] = self.uv[s:s + n][::-1].copy()

    def split_corners(self):
        """A copy with a vertex per corner (as a file split at every seam
        would have it)."""
        out = WorkMesh(self.P[self.corners], np.arange(len(self.corners)), self.sizes, self.uv.copy(),
                       self.face_material.copy(), [dict(m) for m in self.materials])
        if self.weights is not None:
            out.bones = list(self.bones)
            out.joints = self.joints[self.corners]
            out.weights = self.weights[self.corners]
        return out

    def copy(self):
        out = WorkMesh(self.P.copy(), self.corners.copy(), self.sizes.copy(), self.uv.copy(),
                       self.face_material.copy(), [dict(m) for m in self.materials])
        out.bones = list(self.bones)
        out.joints = None if self.joints is None else self.joints.copy()
        out.weights = None if self.weights is None else self.weights.copy()
        return out

    # -- files --------------------------------------------------------------

    def save(self, path, report=None):
        arrays = dict(P=self.P, corners=self.corners, sizes=self.sizes, uv=self.uv,
                      face_material=self.face_material,
                      materials=np.array(json.dumps(self.materials)),
                      bones=np.array(json.dumps(self.bones)))
        if self.weights is not None:
            arrays["joints"] = self.joints
            arrays["weights"] = self.weights
        np.savez_compressed(path, **arrays)
        if report is not None:
            with open(path[:-4] + ".json" if path.endswith(".npz") else path + ".json", "w",
                      newline="\n") as f:
                json.dump(report, f, indent=1)
                f.write("\n")

    @classmethod
    def load(cls, path):
        data = np.load(path, allow_pickle=False)
        mesh = cls(data["P"], data["corners"], data["sizes"], data["uv"], data["face_material"],
                   json.loads(str(data["materials"])))
        mesh.bones = json.loads(str(data["bones"]))
        if "weights" in data:
            mesh.joints = data["joints"]
            mesh.weights = data["weights"]
        return mesh
