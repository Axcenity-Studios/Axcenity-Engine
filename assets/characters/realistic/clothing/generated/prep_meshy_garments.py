#!/usr/bin/env python3
"""Headless Blender prep script for raw AI-generated (Meshy) garment `.glb`s.

Performs the one-time upstream canonical source preparation required by ADR-0046
so raw closed-shell, hanging-arm, origin-normalized Meshy `.glb` files pass
Axcenity Engine's read-only `validate_garment` gate and sit cleanly on the
realistic body without any destructive in-engine rest-mesh mutation:

  1. Outer-shell extraction (removes interior walls and recessed pit floors by
     multi-ray exterior visibility voting + connected-component scrap removal).
  2. 45-degree A-pose sleeve placement (smoothly rotates hanging sleeves outward
     around shoulder pivots to match `RealisticParts.glb`'s 45-degree A-pose).
  3. Canonical body-frame alignment (scales and places the garment onto the
     realistic body's Masculine or Feminine rest frame using `garments.json`
     metadata, plus smooth outward-only clearance to Layer Envelope `layer`).
  4. Boundary-locked decimation & chord-sag enforcement (locks all hem/cuff/
     collar boundary loops via vertex group + UV/SEAM/SHARP delimiting, removes
     non-manifold fins and degenerate faces, subdivides any opening boundary
     edge exceeding L < sqrt(8 * r * m), and preserves PBR materials/UVs).

Usage:
  blender --background --python \
    assets/characters/realistic/clothing/generated/prep_meshy_garments.py -- \
    --input-dir "/home/pauljanota/Projects/Character Creator Assets/3D Clothing Assets" \
    --output-dir "/home/pauljanota/Projects/Character Creator Assets/Prepped_ADR0046" \
    [--body assets/characters/realistic/RealisticParts.glb] \
    [--manifest assets/characters/realistic/clothing/generated/garments.json] \
    [--opening-edge-mm 16.0] [--arm-angle-deg 45.0] [--only bomber]
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

LAYER_ENVELOPE_OFFSETS = (0.002, 0.006, 0.012, 0.020)
HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]


def clear_scene() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def parse_args() -> argparse.Namespace:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description="Prep raw Meshy GLB garments for ADR-0046.")
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=Path("/home/pauljanota/Projects/Character Creator Assets/3D Clothing Assets"),
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("/home/pauljanota/Projects/Character Creator Assets/Prepped_ADR0046"),
    )
    parser.add_argument(
        "--body",
        type=Path,
        default=REPO / "assets/characters/realistic/RealisticParts.glb",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=HERE / "garments.json",
    )
    parser.add_argument("--opening-edge-mm", type=float, default=16.0)
    parser.add_argument("--arm-angle-deg", type=float, default=45.0)
    parser.add_argument("--only", type=str, default=None, help="Substring filter for filename or ID")
    return parser.parse_args(argv)


def load_reference_body_data(body_path: Path, feminine: bool) -> dict:
    """Import `RealisticParts.glb`, evaluate `Body` at Masculine or Feminine frame,
    and return world-space vertices, triangles, and anatomical span stats."""
    clear_scene()
    bpy.ops.import_scene.gltf(filepath=str(body_path))
    body_obj = next((o for o in bpy.context.scene.objects if o.type == "MESH" and o.name == "Body"), None)
    if body_obj is None:
        raise RuntimeError(f"Mesh `Body` not found in {body_path}")

    if body_obj.data.shape_keys:
        for kb in body_obj.data.shape_keys.key_blocks:
            if kb.name == "Frame_Feminine":
                kb.value = 1.0 if feminine else 0.0
            else:
                kb.value = 0.0

    depsgraph = bpy.context.evaluated_depsgraph_get()
    eval_obj = body_obj.evaluated_get(depsgraph)
    eval_mesh = eval_obj.to_mesh()
    mw = eval_obj.matrix_world.copy()
    verts = [mw @ v.co for v in eval_mesh.vertices]
    eval_mesh.calc_loop_triangles()
    tris = [(lt.vertices[0], lt.vertices[1], lt.vertices[2]) for lt in eval_mesh.loop_triangles]
    eval_obj.to_mesh_clear()

    def band_bounds(z0: float, z1: float, max_abs_x: float = 0.30) -> tuple[float, float, float, float]:
        pts = [v for v in verts if z0 <= v.z <= z1 and abs(v.x) <= max_abs_x]
        if not pts:
            return (-0.18, 0.18, -0.10, 0.10)
        return (
            min(v.x for v in pts),
            max(v.x for v in pts),
            min(v.y for v in pts),
            max(v.y for v in pts),
        )

    torso = band_bounds(1.00, 1.38, 0.24)
    hips = band_bounds(0.84, 1.04, 0.24)
    bvh = BVHTree.FromPolygons(verts, tris)
    return {
        "verts": verts,
        "tris": tris,
        "bvh": bvh,
        "torso_width": torso[1] - torso[0],
        "torso_depth": torso[3] - torso[2],
        "torso_y": 0.5 * (torso[2] + torso[3]),
        "hip_width": hips[1] - hips[0],
        "hip_depth": hips[3] - hips[2],
        "hip_y": 0.5 * (hips[2] + hips[3]),
    }


def join_mesh_objects() -> bpy.types.Object:
    meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    if not meshes:
        raise RuntimeError("No mesh objects found in imported GLB")
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return obj


def clean_non_manifold_and_degen(bm: bmesh.types.BMesh, degen_area: float = 1e-10) -> None:
    """Collapse degenerate slivers via `dissolve_degenerate` (without punching holes),
    then remove non-manifold fin faces (>2 linked faces) and loose geometry."""
    bmesh.ops.dissolve_degenerate(bm, edges=bm.edges, dist=2e-4)
    for _ in range(8):
        bm.edges.ensure_lookup_table()
        bm.faces.ensure_lookup_table()
        bad_faces = set()
        for e in bm.edges:
            if len(e.link_faces) > 2:
                ranked = sorted(e.link_faces, key=lambda f: f.calc_area(), reverse=True)
                for extra in ranked[2:]:
                    bad_faces.add(extra)
        for f in bm.faces:
            if f.calc_area() < degen_area:
                bad_faces.add(f)
        if not bad_faces:
            break
        bmesh.ops.delete(bm, geom=list(bad_faces), context="FACES")
        bmesh.ops.dissolve_degenerate(bm, edges=bm.edges, dist=2e-4)

    bm.verts.ensure_lookup_table()
    loose_verts = [v for v in bm.verts if not v.link_faces]
    if loose_verts:
        bmesh.ops.delete(bm, geom=loose_verts, context="VERTS")


def count_boundary_loops(bm: bmesh.types.BMesh) -> list[list[bmesh.types.BMEdge]]:
    """Return all connected boundary edge loops sorted from largest to smallest."""
    bm.edges.ensure_lookup_table()
    bnd = [e for e in bm.edges if e.is_boundary]
    visited = set()
    loops: list[list[bmesh.types.BMEdge]] = []
    for start in bnd:
        if start in visited:
            continue
        loop = []
        stack = [start]
        visited.add(start)
        while stack:
            e = stack.pop()
            loop.append(e)
            for v in e.verts:
                for ne in v.link_edges:
                    if ne.is_boundary and ne not in visited:
                        visited.add(ne)
                        stack.append(ne)
        loops.append(loop)
    loops.sort(key=len, reverse=True)
    return loops


def fill_pinholes(bm: bmesh.types.BMesh, max_loop_edges: int = 48, keep_largest: int = 6) -> None:
    """Fill small accidental pinholes while keeping real anatomical openings
    (neck, cuffs, waist, hems) completely open."""
    for _ in range(6):
        loops = count_boundary_loops(bm)
        if not loops:
            return
        targets = []
        for idx, loop in enumerate(loops):
            if len(loop) <= max_loop_edges or (idx >= keep_largest and len(loop) <= 120):
                targets.append(loop)
        if not targets:
            return
        filled_any = False
        for loop in targets:
            alive = [e for e in loop if e.is_valid and e.is_boundary]
            if len(alive) < 3:
                continue
            res = bmesh.ops.holes_fill(bm, edges=alive, sides=max(len(alive) + 4, 64))
            if res.get("faces"):
                filled_any = True
            else:
                res2 = bmesh.ops.triangle_fill(bm, edges=alive, use_beauty=True, use_dissolve=False)
                if res2.get("geom"):
                    filled_any = True
        ngons = [f for f in bm.faces if len(f.verts) > 3]
        if ngons:
            bmesh.ops.triangulate(bm, faces=ngons)
        clean_non_manifold_and_degen(bm)
        # Any tiny pinhole/slit (<= 16 edges) that survived holes_fill + non-manifold
        # cleanup is a pinched non-manifold slit: weld its vertices to its centroid.
        rem_loops = count_boundary_loops(bm)
        for idx, loop in enumerate(rem_loops):
            if idx >= keep_largest or len(loop) <= 16:
                if len(loop) <= 16:
                    verts = list({v for e in loop if e.is_valid for v in e.verts if v.is_valid})
                    if len(verts) >= 2:
                        center = sum((v.co for v in verts), Vector((0.0, 0.0, 0.0))) / len(verts)
                        bmesh.ops.pointmerge(bm, verts=verts, merge_co=center)
                        filled_any = True
        clean_non_manifold_and_degen(bm)
        if not filled_any:
            return


def _hemisphere_dirs(n: Vector, count: int = 18) -> list[Vector]:
    """Deterministic Fibonacci hemisphere ray directions around unit normal `n`."""
    up = Vector((0.0, 0.0, 1.0)) if abs(n.z) < 0.9 else Vector((1.0, 0.0, 0.0))
    t1 = n.cross(up).normalized()
    t2 = n.cross(t1).normalized()
    golden = math.pi * (3.0 - math.sqrt(5.0))
    dirs = []
    for i in range(count):
        # Cos theta from 0.15 to 0.98 (avoid grazing self-intersection at 0)
        z = 0.15 + 0.83 * (i + 0.5) / count
        r = math.sqrt(max(0.0, 1.0 - z * z))
        phi = golden * i
        d = (t1 * (r * math.cos(phi)) + t2 * (r * math.sin(phi)) + n * z).normalized()
        dirs.append(d)
    return dirs


def extract_outer_shell(obj: bpy.types.Object, max_boundary_loops: int = 6) -> None:
    """Remove strictly enclosed interior cavity walls via 18-ray hemisphere
    sky-visibility voting so outer-wall faces (including inner thighs, wrinkles,
    and thin hems) are NEVER deleted. Fails hard if > `max_boundary_loops` remain."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-4)
    clean_non_manifold_and_degen(bm)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.normal_update()
    bm.verts.ensure_lookup_table()
    bm.faces.ensure_lookup_table()

    bvh = BVHTree.FromBMesh(bm)
    bbox = [Vector(c) for c in obj.bound_box]
    min_v = Vector((min(c.x for c in bbox), min(c.y for c in bbox), min(c.z for c in bbox)))
    max_v = Vector((max(c.x for c in bbox), max(c.y for c in bbox), max(c.z for c in bbox)))
    diag = (max_v - min_v).length
    eps = max(diag * 2e-4, 2e-4)
    ray_len = diag * 2.0

    # A true interior cavity wall is enclosed in all directions (0 or at most 1
    # ray out of 18 escapes to the sky). Any outer face—even between trouser legs
    # or inside a fold—has multiple rays that escape cleanly to open sky.
    inner_set = set()
    for f in bm.faces:
        n = f.normal
        if n.length < 1e-6:
            inner_set.add(f.index)
            continue
        n = n.normalized()
        origin = f.calc_center_median() + n * eps
        escaped = 0
        for d in _hemisphere_dirs(n, 18):
            hit_loc, _hit_norm, hit_idx, _hit_dist = bvh.ray_cast(origin, d, ray_len)
            if hit_loc is None or hit_idx == f.index:
                escaped += 1
                if escaped >= 2:
                    break
        if escaped <= 1:
            inner_set.add(f.index)

    # Only delete large contiguous interior patches (>= 40 faces) so narrow outer
    # creases are never punctured.
    visited_inner = set()
    inner_faces = []
    for f in bm.faces:
        if f.index not in inner_set or f.index in visited_inner:
            continue
        patch = []
        stack = [f]
        visited_inner.add(f.index)
        while stack:
            cur = stack.pop()
            patch.append(cur)
            for e in cur.edges:
                for lf in e.link_faces:
                    if lf.index in inner_set and lf.index not in visited_inner:
                        visited_inner.add(lf.index)
                        stack.append(lf)
        if len(patch) >= 40:
            inner_faces.extend(patch)

    if inner_faces and (len(bm.faces) - len(inner_faces)) >= int(len(bm.faces) * 0.45):
        bmesh.ops.delete(bm, geom=inner_faces, context="FACES")

    # Drop tiny disconnected floating scraps (< 5% of total faces) before filling pinholes.
    bm.faces.ensure_lookup_table()
    visited = set()
    components = []
    for f in bm.faces:
        if f.index in visited:
            continue
        comp = []
        stack = [f]
        visited.add(f.index)
        while stack:
            cur = stack.pop()
            comp.append(cur)
            for e in cur.edges:
                for lf in e.link_faces:
                    if lf.index not in visited:
                        visited.add(lf.index)
                        stack.append(lf)
        components.append(comp)
    if len(components) > 1:
        total_f = len(bm.faces)
        largest_len = max(len(c) for c in components)
        scraps = []
        for c in components:
            if len(c) < largest_len and len(c) < max(50, int(total_f * 0.05)):
                scraps.extend(c)
        if scraps:
            bmesh.ops.delete(bm, geom=scraps, context="FACES")

    clean_non_manifold_and_degen(bm)
    fill_pinholes(bm, max_loop_edges=48, keep_largest=max_boundary_loops)
    clean_non_manifold_and_degen(bm)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    loops = count_boundary_loops(bm)
    if len(loops) > max_boundary_loops:
        sizes = [len(lp) for lp in loops[:10]]
        bm.free()
        raise RuntimeError(
            f"HARD GATE FAILED for {obj.name}: {len(loops)} boundary loops after "
            f"extract_outer_shell (max {max_boundary_loops}); top loop sizes={sizes}"
        )

    bm.normal_update()
    bm.to_mesh(me)
    bm.free()
    me.update()


def pose_sleeves_to_a_pose(obj: bpy.types.Object, target_angle_deg: float, has_sleeves: bool) -> None:
    """Measure each sleeve's actual hang angle from vertical and gently rotate
    ONLY upper-arm sleeve vertices (strictly outside the torso core and above
    the underarm crease) toward `target_angle_deg`, smoothing the shoulder seam
    so zero downward underarm wings are ever created."""
    if not has_sleeves:
        return
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    if not bm.verts:
        bm.free()
        return

    zs = [v.co.z for v in bm.verts]
    xs = [v.co.x for v in bm.verts]
    z_min, z_max = min(zs), max(zs)
    x_min, x_max = min(xs), max(xs)
    height = z_max - z_min
    width = x_max - x_min
    if height < 1e-4 or width < 1e-4:
        bm.free()
        return

    is_full_body = (width / height) < 0.72
    # Shoulder joint height fraction from top of garment.
    shoulder_z = z_max - height * (0.10 if is_full_body else 0.16)
    armpit_z = z_max - height * (0.24 if is_full_body else 0.40)

    # Estimate torso half-width from the lower-chest / waist band (below sleeves).
    waist_lo = z_max - height * (0.45 if is_full_body else 0.85)
    waist_hi = z_max - height * (0.26 if is_full_body else 0.52)
    waist_xs = sorted(abs(v.co.x) for v in bm.verts if waist_lo <= v.co.z <= waist_hi)
    if waist_xs:
        torso_half_w = waist_xs[int(len(waist_xs) * 0.88)]
    else:
        torso_half_w = width * 0.24
    # Clamp torso half-width to sensible range.
    torso_half_w = max(width * 0.18, min(width * 0.34, torso_half_w))

    # Measure actual current sleeve angle from vertical using outer sleeve verts.
    upper_sleeve = [
        v.co
        for v in bm.verts
        if abs(v.co.x) > torso_half_w + width * 0.08
        and v.co.z >= z_max - height * (0.55 if is_full_body else 0.90)
    ]
    if len(upper_sleeve) < 10:
        bm.free()
        return
    outer_sorted = sorted(upper_sleeve, key=lambda p: abs(p.x), reverse=True)
    cuff_pts = outer_sorted[: max(8, len(outer_sorted) // 10)]
    cuff_x = sum(abs(p.x) for p in cuff_pts) / len(cuff_pts)
    cuff_z = sum(p.z for p in cuff_pts) / len(cuff_pts)
    dx = max(cuff_x - torso_half_w, 1e-4)
    dz = max(shoulder_z - cuff_z, 1e-4)
    current_angle_deg = math.degrees(math.atan2(dx, dz))

    delta_deg = max(0.0, min(20.0, target_angle_deg - current_angle_deg))
    if delta_deg < 4.0:
        bm.free()
        return
    delta_rad = math.radians(delta_deg)

    y_mid = sum(v.co.y for v in bm.verts) / len(bm.verts)
    seam_verts = []
    for v in bm.verts:
        side = 1.0 if v.co.x >= 0.0 else -1.0
        out_x = abs(v.co.x) - torso_half_w
        if out_x <= 0.0:
            continue
        # Only rotate vertices along the sleeve envelope, never flared lower hem/coat skirt.
        sleeve_z_floor = armpit_z - out_x * (dz / dx) - height * 0.10
        if v.co.z < sleeve_z_floor:
            continue
        # Smooth horizontal ramp + vertical fade near underarm floor so zero wings form.
        tx = min(1.0, out_x / max(width * 0.16, 1e-4))
        wx = tx * tx * (3.0 - 2.0 * tx)
        tz = min(1.0, (v.co.z - sleeve_z_floor) / max(height * 0.12, 1e-4))
        wz = tz * tz * (3.0 - 2.0 * tz)
        w = wx * wz
        if w <= 1e-4:
            continue
        pivot = Vector((side * torso_half_w, y_mid, shoulder_z))
        rot = Matrix.Rotation(-side * delta_rad * w, 3, "Y")
        v.co = pivot + rot @ (v.co - pivot)
        if 0.02 < w < 0.98:
            seam_verts.append(v)

    # Gentle smoothing on the shoulder transition band only to eliminate creases.
    for _ in range(3):
        updates = {}
        for v in seam_verts:
            nbrs = [e.other_vert(v).co for e in v.link_edges]
            if nbrs:
                avg = sum(nbrs, Vector((0.0, 0.0, 0.0))) / len(nbrs)
                updates[v] = v.co * 0.65 + avg * 0.35
        for v, pos in updates.items():
            v.co = pos

    bm.normal_update()
    bm.to_mesh(me)
    bm.free()
    me.update()


def align_to_body_frame(obj: bpy.types.Object, body_data: dict, entry: dict) -> None:
    """Scale and place the garment onto the realistic body's rest frame, then apply
    a smooth outward-only clearance nudge so zero rest vertices sit inside the body."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    if not bm.verts:
        bm.free()
        return

    xs = [v.co.x for v in bm.verts]
    ys = [v.co.y for v in bm.verts]
    zs = [v.co.z for v in bm.verts]
    x_min, x_max = min(xs), max(xs)
    y_min, y_max = min(ys), max(ys)
    z_min, z_max = min(zs), max(zs)
    raw_h = max(z_max - z_min, 1e-4)
    raw_w = max(x_max - x_min, 1e-4)
    raw_d = max(y_max - y_min, 1e-4)

    z_bot_target, z_top_target = entry["z_range"]
    target_h = max(z_top_target - z_bot_target, 0.15)
    scale_z = target_h / raw_h

    layer = int(entry.get("layer", 1))
    envelope = LAYER_ENVELOPE_OFFSETS[min(max(layer, 0), len(LAYER_ENVELOPE_OFFSETS) - 1)]
    slot = entry.get("slot", "top")
    has_sleeves = bool(entry.get("has_sleeves", False))

    # Layer-scaled ease so tight Layer 0/1 garments hug the canonical body closely
    # and achieve >75% confident proxy binding.
    extra_ease = (0.008, 0.015, 0.024, 0.032)[min(max(layer, 0), 3)]

    # Reference width & depth based on slot (torso vs hips) with gentle anisotropy clamping.
    if slot == "bottom" and "also_fills" not in entry and entry["id"] != "generated/overalls":
        ref_w = body_data["hip_width"] + 2.0 * (envelope + extra_ease)
        ref_d = body_data["hip_depth"] + 2.0 * (envelope + extra_ease)
        target_y = body_data["hip_y"]
        # Measure garment's upper-band width/depth (hips/waist) for scale ratio.
        band = [v for v in bm.verts if v.co.z >= z_max - raw_h * 0.45]
    else:
        ref_w = body_data["torso_width"] + 2.0 * (envelope + extra_ease)
        ref_d = body_data["torso_depth"] + 2.0 * (envelope + extra_ease)
        target_y = body_data["torso_y"]
        # Measure core torso band (excluding outstretched sleeves if present).
        z_lo = z_max - raw_h * (0.45 if has_sleeves else 0.70)
        z_hi = z_max - raw_h * 0.15
        band = [v for v in bm.verts if z_lo <= v.co.z <= z_hi]
        if has_sleeves and band:
            # Exclude outer sleeve vertices when measuring torso width.
            band_xs = sorted(abs(v.co.x) for v in band)
            cut_x = band_xs[int(len(band_xs) * 0.65)]
            core = [v for v in band if abs(v.co.x) <= cut_x]
            if core:
                band = core

    if band:
        bw = max(max(v.co.x for v in band) - min(v.co.x for v in band), 1e-4)
        bd = max(max(v.co.y for v in band) - min(v.co.y for v in band), 1e-4)
    else:
        bw, bd = raw_w, raw_d

    # Clamp X/Y scale relative to Z scale so silhouettes stay faithful and never pancake.
    scale_x = max(scale_z * 0.78, min(scale_z * 1.28, ref_w / bw))
    scale_y = max(scale_z * 0.78, min(scale_z * 1.28, ref_d / bd))
    if has_sleeves:
        # Preserve arm proportions closely to Z scale for sleeved garments.
        scale_x = 0.5 * scale_z + 0.5 * scale_x

    cx = 0.5 * (x_min + x_max)
    cy = 0.5 * (y_min + y_max)
    for v in bm.verts:
        v.co.x = (v.co.x - cx) * scale_x
        v.co.y = (v.co.y - cy) * scale_y + target_y
        v.co.z = (v.co.z - z_min) * scale_z + z_bot_target

    # Smooth outward-only clearance nudge so zero rest vertices sit inside the body.
    bvh: BVHTree = body_data["bvh"]
    min_clear = envelope + 0.004
    pushes = [Vector((0.0, 0.0, 0.0)) for _ in bm.verts]
    for i, v in enumerate(bm.verts):
        loc, norm, _idx, dist = bvh.find_nearest(v.co)
        if loc is None or dist > 0.14:
            continue
        signed = (v.co - loc).dot(norm)
        if signed < min_clear:
            pushes[i] = norm * (min_clear - signed)

    # 4 rounds of Laplacian smoothing on the push field only (preserves rest detail).
    for _ in range(4):
        next_pushes = []
        for i, v in enumerate(bm.verts):
            nbrs = [e.other_vert(v).index for e in v.link_edges]
            if not nbrs:
                next_pushes.append(pushes[i])
                continue
            avg = Vector((0.0, 0.0, 0.0))
            for ni in nbrs:
                avg += pushes[ni]
            avg /= len(nbrs)
            # Keep at least the required push where vertex was inside body.
            blended = pushes[i] * 0.55 + avg * 0.45
            if pushes[i].length > 1e-6 and blended.length < pushes[i].length:
                blended = pushes[i]
            next_pushes.append(blended)
        pushes = next_pushes

    for i, v in enumerate(bm.verts):
        v.co += pushes[i]

    bm.normal_update()
    bm.to_mesh(me)
    bm.free()
    me.update()


def enforce_manifold_and_boundary_budget(
    obj: bpy.types.Object, target_faces: int, opening_edge_mm: float
) -> None:
    """Lock hem/cuff/collar boundary loops, decimate interior preserving UV seams,
    clean non-manifold/degenerate faces, and subdivide any boundary edge exceeding
    the ADR-0046 chord-sag opening budget."""
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    clean_non_manifold_and_degen(bm)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.verts.ensure_lookup_table()

    # Collect boundary vertices + 1-ring so DECIMATE never collapses openings.
    boundary_ids = set()
    for e in bm.edges:
        if e.is_boundary:
            boundary_ids.add(e.verts[0].index)
            boundary_ids.add(e.verts[1].index)
    ring_ids = set(boundary_ids)
    for vid in boundary_ids:
        for e in bm.verts[vid].link_edges:
            ring_ids.add(e.other_vert(bm.verts[vid]).index)

    bm.to_mesh(me)
    bm.free()
    me.update()

    # 1. Simplify interior to target_faces while locking boundary loops & UV seams.
    cur_faces = len(me.polygons)
    if cur_faces > target_faces > 0:
        vg = obj.vertex_groups.get("ADR0046_Boundary") or obj.vertex_groups.new(name="ADR0046_Boundary")
        if ring_ids:
            vg.add(list(ring_ids), 1.0, "REPLACE")
        mod = obj.modifiers.new(name="ADR0046_Decimate", type="DECIMATE")
        mod.decimate_type = "COLLAPSE"
        mod.ratio = max(0.005, min(1.0, target_faces / float(cur_faces)))
        mod.use_collapse_triangulate = True
        mod.delimit = {"UV", "SHARP", "SEAM"}
        if ring_ids:
            mod.vertex_group = vg.name
            mod.invert_vertex_group = True
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
        vg_ref = obj.vertex_groups.get("ADR0046_Boundary")
        if vg_ref is not None:
            obj.vertex_groups.remove(vg_ref)

    # 2. Enforce chord-sag opening edge limit L < sqrt(8 * r * m) and triangulate.
    max_edge_m = opening_edge_mm / 1000.0
    bm = bmesh.new()
    bm.from_mesh(me)
    clean_non_manifold_and_degen(bm)
    for _ in range(10):
        long_open = [e for e in bm.edges if e.is_boundary and e.calc_length() > max_edge_m]
        if not long_open:
            break
        bmesh.ops.subdivide_edges(
            bm, edges=long_open, cuts=1, use_grid_fill=False, use_single_edge=True
        )
        ngons = [f for f in bm.faces if len(f.verts) > 3]
        if ngons:
            bmesh.ops.triangulate(bm, faces=ngons)
        clean_non_manifold_and_degen(bm)
    # Triangulate entire mesh cleanly so glTF export has deterministic triangles.
    ngons = [f for f in bm.faces if len(f.verts) > 3]
    if ngons:
        bmesh.ops.triangulate(bm, faces=ngons)
    # Collapse any remaining sliver triangles (< 1e-6 m^2, validate_garment threshold)
    # by collapsing their shortest edge instead of deleting faces (never punches holes).
    for _ in range(6):
        sliver_edges = set()
        for f in bm.faces:
            if f.calc_area() < 1e-6:
                shortest = min(f.edges, key=lambda e: e.calc_length())
                sliver_edges.add(shortest)
        if not sliver_edges:
            break
        bmesh.ops.collapse(bm, edges=list(sliver_edges), uvs=True)
        clean_non_manifold_and_degen(bm)
    clean_non_manifold_and_degen(bm)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    final_loops = count_boundary_loops(bm)
    bm.to_mesh(me)
    bm.free()
    me.update()
    return len(final_loops)


def prep_one(
    src: Path,
    dst: Path,
    body_data: dict,
    entry: dict,
    opening_edge_mm: float,
    arm_angle_deg: float,
) -> None:
    clear_scene()
    bpy.ops.import_scene.gltf(filepath=str(src))
    obj = join_mesh_objects()
    extract_outer_shell(obj)
    pose_sleeves_to_a_pose(obj, arm_angle_deg, bool(entry.get("has_sleeves", False)))
    align_to_body_frame(obj, body_data, entry)
    target_faces = int(entry.get("triangles", 8000))
    n_loops = enforce_manifold_and_boundary_budget(obj, target_faces, opening_edge_mm)

    dst.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(dst),
        export_format="GLB",
        export_texcoords=True,
        export_normals=True,
        export_materials="EXPORT",
        export_yup=True,
    )
    print(f"[prep] {src.name} -> {dst.name} ({len(obj.data.polygons)} tris, {len(obj.data.vertices)} verts, {n_loops} loops)")


def main() -> int:
    args = parse_args()
    if not args.input_dir.exists():
        print(f"Input directory not found: {args.input_dir}", file=sys.stderr)
        return 1
    manifest = json.loads(args.manifest.read_text())

    print("Loading realistic reference body (Masculine + Feminine)...")
    body_masc = load_reference_body_data(args.body, feminine=False)
    body_fem = load_reference_body_data(args.body, feminine=True)

    only_terms = [t.strip() for t in args.only.split(",") if t.strip()] if args.only else []
    count = 0
    for entry in manifest["garments"]:
        if only_terms and not any(t in entry["file"] or t in entry["id"] for t in only_terms):
            continue
        matches = sorted(args.input_dir.glob(entry["file"] + "*.glb"))
        if not matches:
            print(f"[warn] no source file matching {entry['file']}*.glb", file=sys.stderr)
            continue
        src = matches[0]
        frame = entry.get("frame", "masculine")
        if frame == "unisex":
            # Generate both Masculine (primary) and Feminine (_feminine) canonical sources.
            prep_one(src, args.output_dir / src.name, body_masc, entry, args.opening_edge_mm, args.arm_angle_deg)
            fem_name = src.stem + "_feminine" + src.suffix
            prep_one(src, args.output_dir / fem_name, body_fem, entry, args.opening_edge_mm, args.arm_angle_deg)
            count += 2
        elif frame == "feminine":
            prep_one(src, args.output_dir / src.name, body_fem, entry, args.opening_edge_mm, args.arm_angle_deg)
            count += 1
        else:
            prep_one(src, args.output_dir / src.name, body_masc, entry, args.opening_edge_mm, args.arm_angle_deg)
            count += 1

    print(f"Prepped {count} canonical garment .glb files into {args.output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
