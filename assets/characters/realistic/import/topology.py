"""Step 3 of the intake: report the topology and, when asked, fix it or
help retopologize it (Blender's Python). Nothing happens silently: every
change is counted and every retopology is measured.

- **Report** (`checks.geometry`): degenerate faces, non-manifold, open and
  flipped edges, loose vertices, n-gons and the quad share, before and
  after.
- **Fix** (`"fix": true`): corners closer than `weld` (1e-6 m) are joined
  (a file split at its seams becomes one surface again; texture
  coordinates stay per corner); degenerate faces are dissolved; loose
  vertices removed; when edges run against each other, the faces are made
  consistent (`recalc_face_normals`) and the share that turned is
  reported; n-gons are cut into triangles.
- **Retopology help**: `"decimate": <ratio>` (Blender's collapse) or
  `"remesh": <voxel size in metres>` (voxel remesh: the texture
  coordinates are lost, which the report says, so the maps must be
  baked again by hand). Either is measured against the original surface
  (the distance of the new surface from the old: median, 95th
  percentile, most) and the triangle counts. It is help, not
  retopology: an item that needs clean loops round its joints is
  retopologized by a person (asset-quality.md section 6).

    python -I topology.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import json
import os

import bmesh
import bpy
import numpy as np

import blendmesh
import checks
from workmesh import WorkMesh


def fix(obj, weld=1e-6):
    """The repairs, each counted."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    counts = {"vertices_before": len(bm.verts)}
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=weld)
    counts["welded"] = counts["vertices_before"] - len(bm.verts)
    before = len(bm.faces)
    bmesh.ops.dissolve_degenerate(bm, edges=bm.edges, dist=1e-7)
    counts["degenerate_dissolved"] = before - len(bm.faces)
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    counts["loose_removed"] = len(loose)
    # Only when edges disagree: recalc_face_normals picks an outside per
    # connected piece, which an open shell (a hat's crown) may not want.
    flipped = sum(1 for e in bm.edges if len(e.link_faces) == 2 and not e.is_contiguous)
    counts["flipped_edges_found"] = flipped
    if flipped:
        normals = [f.normal.copy() for f in bm.faces]
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        counts["faces_turned"] = sum(1 for f, n in zip(bm.faces, normals) if f.normal.dot(n) < 0)
    ngons = [f for f in bm.faces if len(f.verts) > 4]
    counts["ngons_cut"] = len(ngons)
    if ngons:
        bmesh.ops.triangulate(bm, faces=ngons, quad_method="BEAUTY", ngon_method="BEAUTY")
    bm.to_mesh(me)
    bm.free()
    me.update()
    return counts


def measure(new, old):
    """How far the new surface lies from the old (metres)."""
    d = checks.distances(new.P, old.P, old.triangles())
    return {"median": round(float(np.median(d)), 5), "p95": round(float(np.percentile(d, 95)), 5),
            "most": round(float(d.max()), 5), "triangles_before": int(len(old.triangles())),
            "triangles_after": int(len(new.triangles()))}


def step(spec, work, source="loaded.npz", target="topology.npz"):
    options = spec.get("topology", {})
    mesh = WorkMesh.load(os.path.join(work, source))
    report = {"before": checks.geometry(mesh)}
    obj = blendmesh.to_object(mesh)
    if options.get("fix", True):
        report["fixes"] = fix(obj, options.get("weld", 1e-6))
    fixed = blendmesh.from_object(obj, mesh.materials)
    out = fixed
    if options.get("decimate"):
        mod = obj.modifiers.new("Decimate", "DECIMATE")
        mod.decimate_type = "COLLAPSE"
        mod.ratio = float(options["decimate"])
        out = blendmesh.from_object(obj, mesh.materials)
        report["decimate"] = dict(ratio=mod.ratio, **measure(out, fixed))
    elif options.get("remesh"):
        mod = obj.modifiers.new("Remesh", "REMESH")
        mod.mode = "VOXEL"
        mod.voxel_size = float(options["remesh"])
        out = blendmesh.from_object(obj, mesh.materials)
        out.uv[:] = 0.0
        report["remesh"] = dict(voxel_size=mod.voxel_size, uvs_lost=True, **measure(out, fixed))
    report["after"] = checks.geometry(out)
    out.save(os.path.join(work, target), report)
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
