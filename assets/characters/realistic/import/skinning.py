"""Weights from the body (NumPy, and `mathutils`'s BVH: Blender's Python).

Each vertex of the item takes the skin weights of the nearest point on
the realistic body's surface (interpolated across that triangle), keeps
its four largest and normalizes them: section 2's skinning rule.

`split="arm-side"` keeps the arm and the side apart, for tops: a vertex
below the shoulder takes its weights only from the arm's skin when it lies
nearer the arm's direction than straight down from the shoulder, and only
from the trunk's skin otherwise, so a sleeve never pulls the side seam
and the side never follows the arm. It is a parameter (off by default):
the weights an item ships with are the engine's fit's
(`axcenity_avatar::fit`), which these only stand in for while the item is
conformed and checked.

    python -I skinning.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import json
import os

import numpy as np

# The bones of an arm (by part of their names), as the wardrobe measures
# the body (`clothing/body.py`).
ARM_PARTS = ("UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little",
             "ShoulderHelper", "ElbowHelper")
DOWN = np.array([0.0, -1.0, 0.0])


class Body:
    """A skinned surface: positions, triangles, four joints and weights per
    vertex on `bones`, and each bone's head (`heads`, by name) and parent."""

    def __init__(self, P, tris, joints, weights, bones, heads, parents=None):
        self.P = np.asarray(P, float)
        self.tris = np.asarray(tris, np.int64)
        self.joints = np.asarray(joints, np.int64)
        self.weights = np.asarray(weights, float)
        self.bones = list(bones)
        self.heads = {k: np.asarray(v, float) for k, v in heads.items()}
        self.parents = parents or {}
        arm_bones = np.array([any(part in b for part in ARM_PARTS) for b in self.bones])
        self.arm = (self.weights * arm_bones[self.joints]).sum(axis=1)
        self._trees = {}

    def tree(self, which="all"):
        """A BVH of all the skin, the arms' (`arm`) or the trunk's
        (`side`): a triangle is an arm's when all its corners are mostly
        arm."""
        if which not in self._trees:
            import bpy  # noqa: F401  (mathutils comes with the bpy module)
            from mathutils.bvhtree import BVHTree
            arm = self.arm[self.tris] >= 0.5
            keep = {"all": np.ones(len(self.tris), bool), "arm": arm.all(axis=1),
                    "side": ~arm.any(axis=1)}[which]
            tris = self.tris[keep]
            tree = BVHTree.FromPolygons([tuple(p) for p in self.P],
                                        [tuple(int(i) for i in t) for t in tris], all_triangles=True)
            self._trees[which] = (tree, tris)
        return self._trees[which]


def classify(body, points):
    """`arm`, `side` or `all` per point: which skin a top's vertex may take
    weights from (see the module's note)."""
    out = np.array(["all"] * len(points), dtype=object)
    for side, sign in (("Left", 1.0), ("Right", -1.0)):
        shoulder = body.heads.get(f"{side}UpperArm")
        elbow = body.heads.get(f"{side}LowerArm")
        if shoulder is None or elbow is None:
            continue
        arm = (elbow - shoulder) / np.linalg.norm(elbow - shoulder)
        for k, p in enumerate(points):
            v = p - shoulder
            if v[0] * sign <= 0 or v[1] >= 0 or np.linalg.norm(v) < 1e-9:
                continue
            v = v / np.linalg.norm(v)
            out[k] = "arm" if np.dot(v, arm) > np.dot(v, DOWN) else "side"
    return out


def _barycentric(p, a, b, c):
    v0, v1, v2 = b - a, c - a, p - a
    d00, d01, d11 = v0 @ v0, v0 @ v1, v1 @ v1
    d20, d21 = v2 @ v0, v2 @ v1
    den = d00 * d11 - d01 * d01
    if abs(den) < 1e-30:
        return np.array([1.0, 0.0, 0.0])
    v = (d11 * d20 - d01 * d21) / den
    w = (d00 * d21 - d01 * d20) / den
    bary = np.clip([1 - v - w, v, w], 0, 1)
    return bary / bary.sum()


def transfer(body, points, split=None, influences=4):
    """Four joints and weights per point (indices into `body.bones`)."""
    points = np.asarray(points, float)
    groups = classify(body, points) if split == "arm-side" else np.array(["all"] * len(points), object)
    joints = np.zeros((len(points), influences), np.int64)
    weights = np.zeros((len(points), influences))
    for k, p in enumerate(points):
        tree, tris = body.tree(groups[k])
        found = tree.find_nearest(tuple(p))
        if found[0] is None:
            tree, tris = body.tree("all")
            found = tree.find_nearest(tuple(p))
        loc, _, index, _ = found
        tri = tris[index]
        bary = _barycentric(np.array(loc), *body.P[tri])
        mix = {}
        for corner, b in zip(tri, bary):
            for j, w in zip(body.joints[corner], body.weights[corner]):
                if w > 0:
                    mix[int(j)] = mix.get(int(j), 0.0) + b * w
        top = sorted(mix.items(), key=lambda jw: -jw[1])[:influences]
        total = sum(w for _, w in top) or 1.0
        for slot, (j, w) in enumerate(top):
            joints[k, slot] = j
            weights[k, slot] = w / total
    return joints, weights


def step(spec, work, source="conformed.npz", target="weighted.npz"):
    """Weights the conformed item from the body at rest."""
    import bodyglb
    from workmesh import WorkMesh
    body = bodyglb.load(spec.get("body"))
    mesh = WorkMesh.load(os.path.join(work, source))
    split = spec.get("weights", {}).get("split")
    mesh.joints, mesh.weights = transfer(body, mesh.P, split=split)
    mesh.bones = list(body.bones)
    import checks
    report = {"split": split, "skinning": checks.skinning(mesh.joints, mesh.weights)}
    mesh.save(os.path.join(work, target), report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work), indent=1))


if __name__ == "__main__":
    main()
