"""Measures a built hairstyle against asset-quality.md §2 and §4 (Character
Studio R6, CO11), on the built files (`RealisticParts.glb` and the style's
`hair/<File>.glb`), with the bpy module:

    python -I assets/characters/realistic/hair/qa_hair.py --style short-crop --out measurements.json

- **counts**: triangles and vertices per level (hero, gameplay, crowd).
- **volume**: from points inside the hairline (past its ramp), a ray
  down each scalp normal from 6 cm out: how far the outermost hair stands
  off the scalp; the share under the style's declared thickness.
- **coverage**: from points on the scalp, a ray out along the normal
  through the hair, each card it meets taking its share of the light by
  the atlas's alpha at that point (the Hair model's cut: hashed from 0.2
  to 0.8 under TAA): the scalp's show-through inside the ramp, and the
  hairline's ramp (the distance over which coverage climbs from 10% to
  90%, per side of the head).
- **texels**: the cap's texel density (pixels per centimetre at the
  atlas's size) and the cards' along and across the strands.
- **inside**: hair vertices inside the body (the most, and how many more
  than 1 mm in), at rest on both frames, at the ends of every head
  parameter the style says it holds, at the build's corners of muscle
  and fat, and in sampled frames of Idle, Walk and Run (the starter
  animations, retargeted as the engine does), per level.
"""

import argparse
import json
import math
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PACKAGE = os.path.dirname(HERE)
QA = os.path.join(os.path.dirname(PACKAGE), "humanoid", "qa")
sys.path.insert(0, HERE)
sys.path.insert(0, QA)

import bpy  # noqa: E402
import numpy as np  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402
from PIL import Image  # noqa: E402

import glb  # noqa: E402
import scalp  # noqa: E402
import styles  # noqa: E402
from groom import Groom  # noqa: E402

LEVELS = ("", "_LOD1", "_LOD2")
ANIMATIONS = os.path.join(os.path.dirname(PACKAGE), "humanoid", "HumanoidAnimations.glb")


def mesh_arrays(obj):
    me = obj.data
    n = len(me.vertices)
    co = np.zeros(n * 3)
    me.vertices.foreach_get("co", co)
    me.calc_loop_triangles()
    tris = np.zeros(len(me.loop_triangles) * 3, dtype=np.int64)
    me.loop_triangles.foreach_get("vertices", tris)
    uv = np.zeros(len(me.loops) * 2)
    me.uv_layers[0].data.foreach_get("uv", uv)
    loops = np.zeros(len(me.loop_triangles) * 3, dtype=np.int64)
    me.loop_triangles.foreach_get("loops", loops)
    return co.reshape(n, 3), tris.reshape(-1, 3), uv.reshape(-1, 2)[loops].reshape(-1, 3, 2)


def import_hair(path):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path, merge_vertices=False)
    new = [o for o in bpy.data.objects if o not in before]
    hair = {o.name.split(".")[0]: o for o in new if o.type == "MESH" and o.name.startswith("Hair_")}
    for o in new:
        if o.type == "ARMATURE" or (o.type == "MESH" and not o.name.startswith("Hair_")):
            o.hide_render = True
    return hair


def alpha_map(style):
    """The atlas's alpha, read from the style's file."""
    doc, binary = glb.read_glb(os.path.join(PACKAGE, "hair", style["file"]))
    mat = doc["materials"][0]
    image = doc["images"][doc["textures"][mat["pbrMetallicRoughness"]["baseColorTexture"]["index"]]["source"]]
    view = doc["bufferViews"][image["bufferView"]]
    start = view.get("byteOffset", 0)
    import io
    png = Image.open(io.BytesIO(binary[start:start + view["byteLength"]]))
    return np.asarray(png.convert("RGBA"))[..., 3].astype(np.float64) / 255.0


def smoothstep(e0, e1, x):
    t = min(1.0, max(0.0, (x - e0) / (e1 - e0)))
    return t * t * (3 - 2 * t)


# -- volume, coverage, texels (the build's own scalp) -----------------------


def surface_measures(style):
    body = scalp.Body(os.path.join(PACKAGE, "RealisticParts.glb"))
    groom = Groom(body, style)
    hair = import_hair(os.path.join(PACKAGE, "hair", style["file"]))
    hero = hair[style["object"]]
    co, tris, uv = mesh_arrays(hero)
    bvh = BVHTree.FromPolygons([Vector(p) for p in co], tris.tolist(), all_triangles=True)
    alpha = alpha_map(style)
    size = alpha.shape[0]
    rng = np.random.default_rng(7)
    places, normals = groom.surface_points(30000, rng)
    density, f = groom.hairline.density(places)
    ramp = groom.hairline.ramp(places)
    phi, theta, _ = groom.head.angles(places)

    def alpha_at(face, loc):
        a, b, c = co[tris[face]]
        bary = scalp.barycentric(np.array(loc), a, b, c)
        t = bary @ uv[face]
        x = int(np.clip(t[0] * size, 0, size - 1))
        y = int(np.clip((1.0 - t[1]) * size, 0, size - 1))  # Blender's V is up
        return alpha[y, x]

    # Volume: the outermost hair along each normal, inside the ramp.
    thick = []
    for p, n, ff, rr in zip(places, normals, f, ramp):
        if ff < rr:
            continue
        start = p + n * 0.06
        hit, _, _, dist = bvh.ray_cast(Vector(start), Vector(-n), 0.07)
        thick.append(0.0 if hit is None else 0.06 - dist)
    thick = np.array(thick) * 1000.0
    # Coverage: light through the hair along the normal.
    cover = np.zeros(len(places))
    for i, (p, n) in enumerate(zip(places, normals)):
        if f[i] < -0.004:
            continue
        origin = Vector(p + n * 0.0001)
        through = 1.0
        travelled = 0.0
        for _ in range(12):
            hit, _, face, dist = bvh.ray_cast(origin, Vector(n), 0.08 - travelled)
            if hit is None:
                break
            through *= 1.0 - smoothstep(0.2, 0.8, alpha_at(face, hit))
            travelled += dist + 1e-5
            origin = hit + Vector(n) * 1e-5
            if through < 1e-3:
                break
        cover[i] = 1.0 - through
    inside = f >= ramp
    show = cover[inside] < 0.5
    out = {
        "volume": {
            "declared_min_mm": style["thickness_min"] * 1000,
            "points": int(len(thick)),
            "min_mm": round(float(thick.min()), 2),
            "p5_mm": round(float(np.percentile(thick, 5)), 2),
            "median_mm": round(float(np.median(thick)), 2),
            "below_declared_share": round(float((thick < style["thickness_min"] * 1000).mean()), 4),
        },
        "coverage": {
            "points_inside_ramp": int(inside.sum()),
            "show_through_share": round(float(show.mean()), 4),
            "mean_cover_inside": round(float(cover[inside].mean()), 4),
        },
    }
    # The hairline's ramp: coverage against distance in from the line,
    # front (|phi| < 50) and sides/back, in 1 mm bins.
    for name, sel in (("front", np.abs(phi) < 50), ("temples", (np.abs(phi) >= 50) & (np.abs(phi) < 80)),
                      ("sides_back", np.abs(phi) >= 80)):
        bins = np.arange(-3, 21)
        profile = []
        for b in bins:
            m = sel & (f * 1000 >= b) & (f * 1000 < b + 1)
            profile.append(float(cover[m].mean()) if m.sum() > 5 else float("nan"))
        profile = np.array(profile)
        valid = ~np.isnan(profile)
        smooth = np.maximum.accumulate(np.where(valid, profile, 0.0))
        lo = bins[np.argmax(smooth >= 0.1)] if (smooth >= 0.1).any() else None
        hi = bins[np.argmax(smooth >= 0.9)] if (smooth >= 0.9).any() else None
        out["coverage"]["ramp_" + name] = {
            "cover_by_mm": {int(b): (round(float(c), 3) if not math.isnan(c) else None) for b, c in zip(bins, profile)},
            "from_10_to_90_mm": (int(hi - lo) if lo is not None and hi is not None else None),
        }
    # Texel density.
    cap = []
    along, across = [], []
    # Where the cap is mapped: the wave-4 style's islands at the atlas's top
    # (rows above 484 px), a guide-groomed style's cap patch (its build
    # report's `atlas.cap`, pixels).
    cap_rect = (0, 0, size, 484)
    report = os.path.join(PACKAGE, "hair", style["file"].replace(".glb", ".build.json"))
    if style.get("method") == "guides" and os.path.exists(report):
        with open(report) as f:
            c = json.load(f)["atlas"]["cap"]
        cap_rect = (c["x"], c["y"], c["x"] + c["w"], c["y"] + c["h"])
    for k, tri in enumerate(tris):
        a, b, c = co[tri]
        area = 0.5 * np.linalg.norm(np.cross(b - a, c - a)) * 1e4  # cm²
        if area < 1e-6:
            continue
        ta, tb, tc = uv[k] * size
        uv_area = 0.5 * abs((tb[0] - ta[0]) * (tc[1] - ta[1]) - (tc[0] - ta[0]) * (tb[1] - ta[1]))
        density_px = math.sqrt(uv_area / area)
        # Blender's V is up.
        px, py = uv[k][:, 0] * size, (1.0 - uv[k][:, 1]) * size
        if ((px >= cap_rect[0]) & (px <= cap_rect[2]) & (py >= cap_rect[1]) & (py <= cap_rect[3])).all():
            cap.append(density_px)
            continue
        # Along: the edge whose UV moves most in V.
        e = [(b - a, tb - ta), (c - a, tc - ta), (c - b, tc - tb)]
        dv = max(e, key=lambda x: abs(x[1][1]))
        du = max(e, key=lambda x: abs(x[1][0]))
        if np.linalg.norm(dv[0]) > 1e-6:
            along.append(abs(dv[1][1]) / (np.linalg.norm(dv[0]) * 100))
        if np.linalg.norm(du[0]) > 1e-6:
            across.append(abs(du[1][0]) / (np.linalg.norm(du[0]) * 100))
    out["texels"] = {
        "atlas": size,
        "cap_px_per_cm_median": round(float(np.median(cap)), 1) if cap else None,
        "cap_px_per_cm_p5": round(float(np.percentile(cap, 5)), 1) if cap else None,
        "cards_along_px_per_cm_median": round(float(np.median(along)), 1),
        "cards_across_px_per_cm_median": round(float(np.median(across)), 1),
    }
    out["counts"] = {}
    for level, suffix in enumerate(LEVELS):
        c2, t2, _ = mesh_arrays(hair[style["object"] + suffix])
        out["counts"][("hero", "gameplay", "crowd")[level]] = {"triangles": int(len(t2)), "vertices": int(len(c2))}
    return out


# -- inside the body, posed --------------------------------------------------


def skin_arrays(obj, bones):
    """Each vertex's bone indices (into `bones`) and weights, 4 at most."""
    groups = {g.index: g.name for g in obj.vertex_groups}
    n = len(obj.data.vertices)
    idx = np.zeros((n, 4), dtype=np.int64)
    wts = np.zeros((n, 4))
    for v in obj.data.vertices:
        pairs = sorted(((e.weight, bones.index(groups[e.group])) for e in v.groups), reverse=True)[:4]
        for k, (w, b) in enumerate(pairs):
            idx[v.index, k] = b
            wts[v.index, k] = w
    wts /= np.maximum(wts.sum(1, keepdims=True), 1e-9)
    return idx, wts


def shaped(obj):
    """The object's shape-mixed rest positions (its keys' current values)."""
    me = obj.data
    n = len(me.vertices)
    base = np.zeros(n * 3)
    me.shape_keys.key_blocks[0].data.foreach_get("co", base)
    base = base.reshape(n, 3)
    out = base.copy()
    for kb in me.shape_keys.key_blocks[1:]:
        if abs(kb.value) < 1e-9:
            continue
        d = np.zeros(n * 3)
        kb.data.foreach_get("co", d)
        out += kb.value * (d.reshape(n, 3) - base)
    return out


def posed_points(arm, bones, rest, idx, wts):
    mats = np.zeros((len(bones), 4, 4))
    for i, name in enumerate(bones):
        pb = arm.pose.bones[name]
        mats[i] = np.array(arm.matrix_world @ pb.matrix @ pb.bone.matrix_local.inverted())
    h = np.concatenate([rest, np.ones((len(rest), 1))], axis=1)
    out = np.zeros((len(rest), 3))
    for k in range(4):
        m = mats[idx[:, k]]
        out += wts[:, k:k + 1] * np.einsum("nij,nj->ni", m, h)[:, :3]
    return out


def inside_depths(body_obj, points):
    deps = bpy.context.evaluated_depsgraph_get()
    ev = body_obj.evaluated_get(deps)
    me = ev.to_mesh()
    co = np.array([v.co for v in me.vertices])
    co = np.array([body_obj.matrix_world @ Vector(p) for p in co])
    me.calc_loop_triangles()
    tris = [tuple(t.vertices) for t in me.loop_triangles]
    ev.to_mesh_clear()
    bvh = BVHTree.FromPolygons([Vector(p) for p in co], tris, all_triangles=True)
    depth = np.zeros(len(points))
    for i, p in enumerate(points):
        loc, nrm, _, _ = bvh.find_nearest(Vector(p))
        d = (Vector(p) - loc).dot(nrm)
        depth[i] = max(0.0, -d)
    return depth


def posed_measures(style):
    import qa_scene as Q
    Q.reset()
    parts, _, custom = Q.package_paths("realistic")
    ch = Q.Character(parts, merge_vertices=True, custom=Q.load_custom(custom))
    hair = import_hair(os.path.join(PACKAGE, "hair", style["file"]))
    bones = [b.name for b in ch.arm.data.bones]
    skins = {name: skin_arrays(obj, bones) for name, obj in hair.items()}
    body = ch.parts["Body"]
    manifest = ch.manifest

    def set_hair_shapes():
        values = {kb.name: kb.value for kb in body.data.shape_keys.key_blocks[1:]}
        for obj in hair.values():
            for kb in obj.data.shape_keys.key_blocks[1:]:
                kb.value = values.get(kb.name, 0.0)

    def measure():
        set_hair_shapes()
        Q.update()
        result = {}
        for name, obj in hair.items():
            idx, wts = skins[name]
            pts = posed_points(ch.arm, bones, shaped(obj), idx, wts)
            depth = inside_depths(body, pts) * 1000.0
            worst = int(np.argmax(depth))
            result[name] = {"deepest_mm": round(float(depth.max()), 3), "over_1mm": int((depth > 1.0).sum()),
                            "deepest_at": [round(float(x), 4) for x in pts[worst]]}
        return result

    def look(frame, values):
        weights = Q.manifest_weights(manifest, dict(values, frame=frame))
        ch.set_look(frame=frame, build={k: v for k, v in weights.items() if k != "Frame_Feminine"},
                    bones=Q.manifest_bones(manifest, values))

    cases = {}
    head_scale = {}
    for frame in (0.0, 1.0):
        tag = "masculine" if frame == 0 else "feminine"
        ch.pose_reset()
        look(frame, {})
        cases[f"{tag}/rest"] = measure()
        for parameter, (low, high) in style["holds"].items():
            if parameter == "frame":
                continue
            for value in (low, high):
                ch.pose_reset()
                if parameter == "head_size":
                    look(frame, {})
                    ch.arm.pose.bones["Head"].scale = (value, value, value)
                else:
                    look(frame, {parameter: value})
                cases[f"{tag}/{parameter}={value}"] = measure()
                head_scale[parameter] = True
        for corner, values in (("heavy", {"fat": 1.0}), ("slim", {"fat": -1.0}), ("muscular", {"muscle": 1.0}),
                               ("heavy_muscular", {"fat": 1.0, "muscle": 1.0})):
            ch.pose_reset()
            look(frame, values)
            cases[f"{tag}/{corner}"] = measure()
    take_arm, actions = Q.load_takes(ANIMATIONS)
    motion = {}
    for frame in (0.0, 1.0):
        tag = "masculine" if frame == 0 else "feminine"
        look(frame, {})
        for clip in ("Idle", "Walk", "Run"):
            action = actions.get(clip)
            if action is None:
                continue
            worst = {}
            for k in range(8):
                Q.pose_take(ch, take_arm, action, k / 8.0)
                for name, r in measure().items():
                    w = worst.setdefault(name, {"deepest_mm": 0.0, "over_1mm": 0})
                    if r["deepest_mm"] > w["deepest_mm"]:
                        w["deepest_at"] = r["deepest_at"]
                    w["deepest_mm"] = max(w["deepest_mm"], r["deepest_mm"])
                    w["over_1mm"] = max(w["over_1mm"], r["over_1mm"])
            motion[f"{tag}/{clip}"] = worst
    return {"rest_and_parameters": cases, "motion": motion,
            "notes": "Depth: how far a hair vertex is inside the body's posed surface (nearest face, along its "
                     "normal), mm. Motion: the worst of 8 evenly spaced frames of each clip."}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--style", default="short-crop", choices=sorted(styles.STYLES))
    parser.add_argument("--out", required=True)
    parser.add_argument("--only", default="", help="surface or posed")
    args = parser.parse_args()
    style = styles.STYLES[args.style]
    out = {"style": style["id"], "file": style["file"]}
    t0 = time.time()
    if args.only in ("", "surface"):
        out.update(surface_measures(style))
        print("surface", round(time.time() - t0, 1), flush=True)
    if args.only in ("", "posed"):
        out["inside"] = posed_measures(style)
        print("posed", round(time.time() - t0, 1), flush=True)
    with open(args.out, "w", newline="\n") as f:
        json.dump(out, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
