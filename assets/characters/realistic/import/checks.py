"""The asset-quality.md section 2 checks the intake runs on every level of
detail (NumPy, and `mathutils`'s BVH for distances: Blender's Python):

| Area | Measured here |
|---|---|
| Geometry | degenerate faces (area under 1e-10 m^2 or a repeated corner), non-manifold edges (three or more faces), open edges, flipped edges (two faces running an edge the same way), loose vertices; corners split at seams are welded first |
| Topology | faces, quads, triangles, n-gons, the quad share |
| UVs | overlap (texels covered twice, at 1024 x 1024), stretch (each triangle's anisotropy: its longer singular value over its shorter, minus 1; the area share over 10%), texel density (px/cm at the map's size: median and 5th percentile by area) |
| Skinning | influences per vertex (at most four), weights summing to 1, weights on a bone more than one joint from the nearest bone |
| Clearance | signed distances to the body (`distances`), and `settle` for placing an item down onto it |

    python -I checks.py --spec <asset>.intake.json --work <folder>

writes `work/checks.json` for the levels the `levels` step made.
"""

import argparse
import json
import os

import numpy as np

# The section 2 limits (asset-quality.md): those marked proposed there.
STRETCH = 0.10
STRETCH_AREA = 0.95
DENSITY = {"garment": 8.0, "accessory": 8.0, "groom": 10.0}


def weld(P, tol=1e-6):
    """An index per vertex, the same for vertices closer than `tol`."""
    keys = np.round(np.asarray(P) / tol).astype(np.int64)
    _, inverse = np.unique(keys, axis=0, return_inverse=True)
    return inverse.reshape(-1)


def polygon_area(points):
    total = np.zeros(3)
    for k in range(1, len(points) - 1):
        total += np.cross(points[k] - points[0], points[k + 1] - points[0])
    return 0.5 * float(np.linalg.norm(total))


def geometry(mesh):
    """Counts of section 2's geometry and topology rules."""
    ids = weld(mesh.P)
    faces = mesh.faces()
    edges = {}
    degenerate = 0
    for f, face in enumerate(faces):
        welded = ids[face]
        if len(set(welded.tolist())) < len(welded) or polygon_area(mesh.P[face]) < 1e-10:
            degenerate += 1
        for a, b in zip(welded, np.roll(welded, -1)):
            if a == b:
                continue
            edges.setdefault((min(a, b), max(a, b)), []).append((a, b))
    non_manifold = sum(1 for runs in edges.values() if len(runs) > 2)
    boundary = sum(1 for runs in edges.values() if len(runs) == 1)
    flipped = sum(1 for runs in edges.values() if len(runs) == 2 and runs[0] == runs[1])
    sizes = mesh.sizes
    used = np.zeros(ids.max() + 1 if len(ids) else 0, dtype=bool)
    used[ids[mesh.corners]] = True
    return {
        "vertices": int(ids.max() + 1) if len(ids) else 0,
        "faces": int(len(sizes)),
        "quads": int((sizes == 4).sum()),
        "triangle_faces": int((sizes == 3).sum()),
        "ngons": int((sizes > 4).sum()),
        "quad_share": round(float((sizes == 4).mean()) if len(sizes) else 0.0, 4),
        "triangles": int(np.maximum(sizes - 2, 0).sum()),
        "degenerate": degenerate,
        "non_manifold_edges": non_manifold,
        "boundary_edges": boundary,
        "flipped_edges": flipped,
        "loose_vertices": int((~used).sum()),
    }


def _weighted_percentile(values, weights, q):
    order = np.argsort(values)
    v, w = values[order], weights[order]
    cum = np.cumsum(w)
    if cum[-1] <= 0:
        return float("nan")
    return float(v[np.searchsorted(cum, q / 100 * cum[-1])].item()) if len(v) else float("nan")


def uvs(mesh, size=2048, resolution=1024):
    """Overlap, stretch and texel density of the first UV set."""
    tc = mesh.triangle_corners()
    if len(tc) == 0:
        return {}
    P = mesh.P[mesh.corners[tc]]           # (t, 3, 3)
    U = mesh.uv[tc]                        # (t, 3, 2)
    e1, e2 = P[:, 1] - P[:, 0], P[:, 2] - P[:, 0]
    d1, d2 = U[:, 1] - U[:, 0], U[:, 2] - U[:, 0]
    area3 = 0.5 * np.linalg.norm(np.cross(e1, e2), axis=1)
    det = d1[:, 0] * d2[:, 1] - d1[:, 1] * d2[:, 0]
    area_uv = 0.5 * np.abs(det)
    ok = (area_uv > 1e-14) & (area3 > 1e-14)
    # The map from UV to the surface: J = [e1 e2] [d1 d2]^-1, its singular
    # values the surface's metres per UV unit along the two directions.
    stretch = np.full(len(tc), np.inf)
    inv = np.zeros((len(tc), 2, 2))
    inv[ok, 0, 0] = d2[ok, 1] / det[ok]
    inv[ok, 0, 1] = -d2[ok, 0] / det[ok]
    inv[ok, 1, 0] = -d1[ok, 1] / det[ok]
    inv[ok, 1, 1] = d1[ok, 0] / det[ok]
    E = np.stack([e1, e2], axis=2)         # (t, 3, 2)
    J = E @ inv
    s = np.linalg.svd(J[ok], compute_uv=False)
    stretch[ok] = s[:, 0] / np.maximum(s[:, 1], 1e-30) - 1
    density = np.zeros(len(tc))
    density[ok] = np.sqrt(area_uv[ok] * size * size / (area3[ok] * 1e4))
    weights = area3 / max(area3.sum(), 1e-30)
    stretched = float(weights[stretch > STRETCH].sum())
    finite = np.where(np.isfinite(stretch), stretch, 1e9)

    # Overlap: count the texels each triangle covers (pixel centres nudged
    # off the grid so an edge two triangles share is never on one).
    count = np.zeros((resolution, resolution), dtype=np.int32)
    nudge = np.array([0.000123, 0.0000731])
    for k in np.nonzero(area_uv > 1e-14)[0]:
        tri = U[k] * resolution
        lo = np.clip(np.floor(tri.min(axis=0)).astype(int), 0, resolution - 1)
        hi = np.clip(np.ceil(tri.max(axis=0)).astype(int), 0, resolution - 1)
        xs, ys = np.meshgrid(np.arange(lo[0], hi[0] + 1), np.arange(lo[1], hi[1] + 1))
        px = np.stack([xs.ravel(), ys.ravel()], axis=1) + 0.5 + nudge
        a, b, c = tri
        v0, v1, v2 = b - a, c - a, px - a
        dd = v0[0] * v1[1] - v0[1] * v1[0]
        if abs(dd) < 1e-18:
            continue
        w1 = (v2[:, 0] * v1[1] - v2[:, 1] * v1[0]) / dd
        w2 = (v0[0] * v2[:, 1] - v0[1] * v2[:, 0]) / dd
        inside = (w1 > 0) & (w2 > 0) & (w1 + w2 < 1)
        count[px[inside, 1].astype(int), px[inside, 0].astype(int)] += 1
    covered = int((count > 0).sum())
    return {
        "map_size": size,
        "overlap_share": round(float((count > 1).sum() / covered) if covered else 0.0, 4),
        "covered_share": round(covered / resolution ** 2, 4),
        "stretched_share": round(stretched, 4),
        "stretch_p95": round(min(_weighted_percentile(finite, weights, 95), 1e9), 4),
        "texel_density_px_per_cm": {
            "median": round(_weighted_percentile(density[ok], weights[ok], 50), 2),
            "p5": round(_weighted_percentile(density[ok], weights[ok], 5), 2),
        },
        "degenerate_uv_triangles": int((~ok).sum()),
    }


def skinning(joints, weights, influences=None):
    """Influences per vertex (at most four), weights summing to 1."""
    weights = np.asarray(weights, dtype=float)
    if influences is None:
        influences = (weights > 1e-4).sum(axis=1)
    influences = np.asarray(influences)
    sums = weights.sum(axis=1)
    return {
        "vertices": int(len(weights)),
        "most_influences": int(influences.max()) if len(influences) else 0,
        "over_four": int((influences > 4).sum()),
        "not_normalized": int((np.abs(sums - 1) > 1e-3).sum()),
        "unweighted": int((sums <= 1e-6).sum()),
    }


def hops(parents, a, b):
    """How many joints apart bones `a` and `b` are in the hierarchy."""
    def chain(bone):
        out = []
        while bone is not None:
            out.append(bone)
            bone = parents.get(bone)
        return out
    ca, cb = chain(a), chain(b)
    common = next((x for x in ca if x in cb), None)
    if common is None:
        return 99
    return ca.index(common) + cb.index(common)


def far_weights(P, joints, weights, bones, heads, parents, least=0.05):
    """Vertices weighted (more than `least`) to a bone more than one joint
    from the bone nearest them (by its head)."""
    names = [b for b in bones if b in heads]
    H = np.array([heads[b] for b in names])
    nearest = np.argmin(((P[:, None, :] - H[None]) ** 2).sum(axis=2), axis=1)
    far = 0
    for v in range(len(P)):
        near = names[nearest[v]]
        for j, w in zip(joints[v], weights[v]):
            if w > least and hops(parents, bones[j], near) > 1:
                far += 1
                break
    return far


def _tree(P, tris):
    import bpy  # noqa: F401  (mathutils comes with the bpy module)
    from mathutils.bvhtree import BVHTree
    return BVHTree.FromPolygons([tuple(p) for p in np.asarray(P, float)],
                                [tuple(int(i) for i in t) for t in tris], all_triangles=True)


def distances(points, P, tris, signed=False, tree=None):
    """Each point's distance to the surface (P, tris); with `signed`,
    negative inside it (behind its nearest face)."""
    tree = tree or _tree(P, tris)
    out = np.empty(len(points))
    for k, p in enumerate(np.asarray(points, float)):
        found = tree.find_nearest(tuple(p))
        if found[0] is None:
            out[k] = np.inf
            continue
        loc, normal, _, dist = found
        # Behind the face: a point straight over an edge is level with the
        # face beside it, so only clearly behind counts (float32 noise).
        if signed and np.dot(p - np.array(loc), np.array(normal)) < -1e-7:
            dist = -dist
        out[k] = dist
    return out


def settle(points, P, tris, direction=(0, -1, 0), clearance=0.004, step=0.002, max_drop=1.0):
    """How far `points` can move along `direction` before any comes
    within `clearance` of the surface (or behind it): an item lowered onto
    the body until it sits."""
    tree = _tree(P, tris)
    direction = np.asarray(direction, float) / np.linalg.norm(direction)
    points = np.asarray(points, float)

    def fits(d):
        return distances(points + d * direction, P, tris, signed=True, tree=tree).min() >= clearance

    if not fits(0.0):
        return 0.0
    lo, hi = 0.0, None
    d = step
    while d <= max_drop:
        if fits(d):
            lo = d
            d += step
        else:
            hi = d
            break
    if hi is None:
        return lo
    for _ in range(30):
        mid = (lo + hi) / 2
        if fits(mid):
            lo = mid
        else:
            hi = mid
    return lo


def skin_limits(skin_report, rigid_bone=None):
    """Section 2's skinning rules, pass or fail: four influences at most,
    normalized, and (when the report counts it) no vertex weighted to a
    bone more than one joint from the bone nearest it. A rigid item
    (`fit.rigid_bone`) is on its one bone by design, so that rule isn't
    judged for it."""
    out = {
        "four_influences_at_most": skin_report["over_four"] == 0,
        "weights_normalized": skin_report["not_normalized"] == 0 and skin_report["unweighted"] == 0,
    }
    if "far_from_their_bone" in skin_report and not rigid_bone:
        out["no_weight_past_the_next_joint"] = skin_report["far_from_their_bone"] == 0
    return out


def limits_report(kind, geometry_report, uv_report, skin_report, rigid_bone=None):
    """Pass or fail against section 2's limits, rule by rule."""
    out = {
        "no_degenerate_faces": geometry_report["degenerate"] == 0,
        "no_non_manifold_edges": geometry_report["non_manifold_edges"] == 0,
        "normals_consistent": geometry_report["flipped_edges"] == 0,
    }
    if uv_report:
        out["no_uv_overlap"] = uv_report["overlap_share"] == 0
        out["stretch_under_10_percent_on_95_percent"] = uv_report["stretched_share"] <= 1 - STRETCH_AREA
        out["texel_density"] = uv_report["texel_density_px_per_cm"]["p5"] >= DENSITY.get(kind, 8.0)
    if skin_report:
        out.update(skin_limits(skin_report, rigid_bone))
    return out


def step(spec, work):
    """Checks every level the `levels` step wrote; `work/checks.json`."""
    from workmesh import WorkMesh
    import bodyglb
    body = bodyglb.load(spec.get("body"))
    kind = spec["item"]["kind"]
    size = spec.get("map_size", 2048)
    out = {}
    level = 0
    while os.path.exists(os.path.join(work, f"lod{level}.npz")):
        mesh = WorkMesh.load(os.path.join(work, f"lod{level}.npz"))
        g = geometry(mesh)
        u = uvs(mesh, size)
        s = skinning(mesh.joints, mesh.weights) if mesh.weights is not None else {}
        if s:
            s["far_from_their_bone"] = far_weights(mesh.P, mesh.joints, mesh.weights, mesh.bones,
                                                   body.heads, body.parents)
        inside = distances(mesh.P, body.P, body.tris, signed=True)
        out[f"lod{level}"] = {
            "geometry": g, "uvs": u, "skinning": s,
            "clearance_m": {"inside_body": int((inside < 0).sum()),
                            "least": round(float(inside.min()), 4),
                            "median": round(float(np.median(inside)), 4)},
            # These are the intake's own weights (skinning.py), before the
            # engine's fit; the export step judges the shipped ones again.
            "limits": limits_report(kind, g, u, s, spec.get("fit", {}).get("rigid_bone")),
        }
        level += 1
    with open(os.path.join(work, "checks.json"), "w", newline="\n") as f:
        json.dump(out, f, indent=1)
        f.write("\n")
    return {k: {"triangles": v["geometry"]["triangles"],
                "failed": [rule for rule, ok in v["limits"].items() if not ok]} for k, v in out.items()}


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
