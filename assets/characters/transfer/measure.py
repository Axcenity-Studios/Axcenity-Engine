"""Step 4 of the MakeHuman transfer spike: the numbers.

    python -I measure.py --work WORK [--out measurements.json] [--methods]

(`python` has the `bpy` module; run `transfer.py` first.) Prints Markdown
tables and writes the same as JSON.

Per transferred shape, at full weight, in MakeHuman's pose:

- the largest move and where it is (the realistic body's face set and
  side), beside MakeHuman's own largest move for that target, and how far
  apart the two places are once MakeHuman's is carried to our body;
- self-intersection: pairs of triangles the shape makes cross (sharing no
  vertex, not already crossing in the neutral, and not touching in the
  neutral: closed lips, eyelids and neighbouring toes rest within 4 mm):
  before the keep-apart step, after it, and on MakeHuman's own mesh with
  the same target;
- how much of the shape is asymmetric before it is made symmetric;
- noise: the offsets' roughness (each vertex's offset against the mean of
  its neighbours', over the squared edge length, which estimates the
  field's second derivative whatever the mesh's density), ours over
  MakeHuman's, at the median and the 95th percentile of the vertices the
  target moves by more than a millimetre. 1.0 is "as smooth as the
  source"; below 1 is smoother (detail lost), above is noise added;
- whether the body changes as MakeHuman's does: the change in volume, and
  in girth (the tape length round the cross-section) at the chest, waist,
  hips, thigh and upper arm, all as percentages. The trunk's three levels
  are set on MakeHuman's mesh and carried to ours by the correspondence.

`--methods` adds the comparison of the transfer's variants
(`transfer.py --method`).
"""

import argparse
import json
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import geom  # noqa: E402
import skeleton  # noqa: E402
import transfer  # noqa: E402
from register import FRAMES  # noqa: E402

REGIONS = {1: "chest and upper back", 17: "head and neck", 18: "pelvis", 19: "waist", 22: "mouth",
           20: "upper arm", 21: "upper arm", 11: "forearm", 12: "forearm", 9: "hand", 10: "hand",
           23: "thigh", 24: "thigh", 15: "shin", 16: "shin", 13: "foot", 14: "foot",
           25: "toes", 26: "toes", 2: "eye socket", 3: "eye socket", 4: "ear", 5: "ear",
           7: "inside mouth", 8: "inside mouth"}
GIRTHS = ("chest", "waist", "hips", "thigh", "upper arm")
# Trunk levels as shares of the way from the crotch to the shoulder joints.
TRUNK = {"chest": 0.74, "waist": 0.36, "hips": 0.08}


def region(sets, P, normals, i):
    s = int(sets.vertex_set[i])
    if 27 <= s <= 63:
        name = "toes"
    elif s >= 64:
        name = "fingers"
    else:
        name = REGIONS.get(s, f"set {s}")
    if name in ("chest and upper back", "waist", "pelvis"):
        name += ", front" if normals[i, 1] < 0 else ", back"
    if abs(P[i, 0]) > 0.01 and name not in ("head and neck", "mouth"):
        name += " (left)" if P[i, 0] > 0 else " (right)"
    return name


def where(pairs, V, tris, height):
    """A short description of where crossing pairs sit."""
    names = {}
    for a, b in pairs:
        c = (V[tris[a]].mean(axis=0) + V[tris[b]].mean(axis=0)) / 2
        z, x = c[2] / height, abs(c[0])
        if z > 0.86:
            key = "head"
        elif z > 0.62 and 0.10 < x < 0.30:
            key = "armpit"
        elif 0.36 < z < 0.56 and x < 0.10:
            key = "crotch and inner thigh"
        elif x > 0.3:
            key = "hand"
        elif z < 0.08:
            key = "foot"
        else:
            key = "trunk"
        names[key] = names.get(key, 0) + 1
    return ", ".join(f"{k} {v}" for k, v in sorted(names.items(), key=lambda kv: -kv[1]))


def roughness(d, lap, h2):
    """Per vertex: |offset - neighbours' mean offset| / mean squared edge length."""
    return np.linalg.norm(d - lap.mean(d), axis=1) / h2


def edge_len2(V, e, n):
    l2 = ((V[e[:, 0]] - V[e[:, 1]]) ** 2).sum(axis=1)
    return (np.bincount(e[:, 0], l2, n) + np.bincount(e[:, 1], l2, n)) / \
        np.maximum(np.bincount(e[:, 0], minlength=n) + np.bincount(e[:, 1], minlength=n), 1)


def hull_perimeter(points):
    """The perimeter of the convex hull of 2D points (a tape measure's length)."""
    pts = sorted(set(map(tuple, np.round(points, 6))))
    if len(pts) < 3:
        return 0.0

    def half(seq):
        out = []
        for p in seq:
            while len(out) >= 2 and ((out[-1][0] - out[-2][0]) * (p[1] - out[-2][1])
                                     - (out[-1][1] - out[-2][1]) * (p[0] - out[-2][0])) <= 0:
                out.pop()
            out.append(p)
        return out

    hull = half(pts)[:-1] + half(list(reversed(pts)))[:-1]
    h = np.array(hull)
    return float(np.linalg.norm(h - np.roll(h, 1, axis=0), axis=1).sum())


def loops(V, tris, origin, axis):
    """The closed curves where the plane through `origin` across `axis`
    cuts the mesh: a list of (n, 2) point arrays in the plane."""
    s = (V - origin) @ axis
    up = s[tris] > 0
    cut = tris[(up.sum(axis=1) % 3) != 0]
    u = np.cross(axis, [0.0, 1.0, 0.0])
    if np.linalg.norm(u) < 1e-6:
        u = np.array([1.0, 0.0, 0.0])
    u /= np.linalg.norm(u)
    w = np.cross(axis, u)
    parent, point = {}, {}

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for t in cut.tolist():
        keys = []
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            if (s[a] > 0) != (s[b] > 0):
                key = (min(a, b), max(a, b))
                if key not in parent:
                    parent[key] = key
                    p = V[a] + (V[b] - V[a]) * (s[a] / (s[a] - s[b]))
                    point[key] = ((p - origin) @ u, (p - origin) @ w)
                keys.append(key)
        if len(keys) == 2:
            parent[find(keys[0])] = find(keys[1])
    groups = {}
    for key in parent:
        groups.setdefault(find(key), []).append(point[key])
    return [np.array(g) for g in groups.values()]


def trunk_levels(lm):
    """{name: height} of the trunk's girth levels, from landmarks."""
    shoulder_z = (lm["l-shoulder"][2] + lm["r-shoulder"][2]) / 2
    crotch_z = lm["crotch"][2]
    return {k: crotch_z + share * (shoulder_z - crotch_z) for k, share in TRUNK.items()}


def carried_levels(levels, rest_z, P):
    """MakeHuman's trunk levels on our body: the middle height of our
    trunk vertices that the correspondence puts at each of MakeHuman's
    levels (`rest_z`: per realistic vertex, the height on MakeHuman's
    neutral of the point it maps to)."""
    trunk = np.abs(P[:, 0]) < 0.12
    return {k: float(np.median(P[trunk & (np.abs(rest_z - z) < 0.008), 2])) for k, z in levels.items()}


def girths(V, tris, lm, levels):
    """{name: metres}: the tape length round the trunk at `levels` and
    round the left thigh and upper arm at places set by the landmarks `lm`
    (the neutral's, so a changed body is measured at the same places)."""
    up = np.array([0.0, 0.0, 1.0])
    out = {}
    for name, z in levels.items():
        curves = loops(V, tris, np.array([0.0, 0.0, z]), up)
        # The trunk is the curve round the middle line.
        mid = [c for c in curves if c[:, 0].min() < 0 < c[:, 0].max()]
        out[name] = max(hull_perimeter(c) for c in mid)
    for name, a, b, t in (("thigh", "l-hip", "l-knee", 0.5), ("upper arm", "l-shoulder", "l-elbow", 0.6)):
        axis = lm[b] - lm[a]
        axis /= np.linalg.norm(axis)
        o = lm[a] + (lm[b] - lm[a]) * t
        curves = [c for c in loops(V, tris, o, axis) if len(c) >= 6]
        nearest = min(curves, key=lambda c: np.linalg.norm(c.mean(axis=0)))
        out[name] = hull_perimeter(nearest)
    return out


def percent(new, old):
    return 100.0 * (new / old - 1.0)


class Context:
    def __init__(self, work):
        self.M = np.load(os.path.join(work, "makehuman.npz"))
        self.R = np.load(os.path.join(work, "realistic.npz"))
        self.G = np.load(os.path.join(work, "registration.npz"))
        faces_r = self.R["faces"]
        self.tris_r, self.edges_r = geom.triangles(faces_r), geom.edges(faces_r)
        self.tris_m, self.edges_m = geom.triangles(self.M["faces"]), geom.edges(self.M["faces"])
        self.sets = skeleton.Sets(faces_r, self.R["face_sets"])
        self.lap_r = geom.Laplacian(len(self.R["masculine"]), self.edges_r)
        self.lap_m = geom.Laplacian(len(self.M["base"]), self.edges_m)
        self.mirror = geom.mirror_map([self.R["masculine"], self.R["feminine"]])
        self.joint_names = self.M["joint_names"].tolist()
        self.lm_names = self.G["landmark_names"].tolist()

    def frame(self, frame, gender, mapping):
        f = type("Frame", (), {})()
        f.P, f.N, f.X = self.G[f"posed_{frame}"], self.M[f"neutral_{gender}"], self.G[f"wrapped_{frame}"]
        f.lm_r = dict(zip(self.lm_names, self.G[f"landmarks_{frame}"]))
        f.lm_m = skeleton.makehuman_landmarks(dict(zip(self.joint_names, self.M[f"joints_{gender}"])), f.N)
        f.h2_r = edge_len2(f.P, self.edges_r, len(f.P))
        f.h2_m = edge_len2(f.N, self.edges_m, len(f.N))
        f.vol_r, f.vol_m = geom.volume(f.P, self.tris_r), geom.volume(f.N, self.tris_m)
        f.levels_m = trunk_levels(f.lm_m)
        f.levels_r = carried_levels(f.levels_m, mapping.sample(f.N[:, 2]), f.P)
        f.g_r = girths(f.P, self.tris_r, f.lm_r, f.levels_r)
        f.g_m = girths(f.N, self.tris_m, f.lm_m, f.levels_m)
        f.known_r = transfer.crossing_pairs(f.P, self.tris_r)
        f.known_m = transfer.crossing_pairs(f.N, self.tris_m)
        return f


def measure(work, log=print):
    c = Context(work)
    M, R = c.M, c.R
    shapes = np.load(os.path.join(work, "shapes.npz"))
    raw, maps = transfer.run(work, "frames", log=lambda *_: None)
    sym, _ = transfer.run(work, "symmetric", log=lambda *_: None)
    flip = np.array([-1.0, 1.0, 1.0])
    rows, neutral = [], {}
    for frame, gender in FRAMES:
        f = c.frame(frame, gender, maps[frame])
        P, N, X = f.P, f.N, f.X
        normals = geom.vertex_normals(P, c.tris_r)
        height = np.ptp(P[:, 2])
        bundle = transfer.crossing_pairs(R[frame], c.tris_r)
        # Turning a shape to the bundle's pose and back should change nothing.
        d0 = shapes[f"posed/{frame}/minmuscle_maxweight"].astype(np.float64)
        there = transfer.reframe(d0, P, R[frame], c.edges_r)
        back = transfer.reframe(there, R[frame], P, c.edges_r)
        turn, _ = geom.local_frames(P, R[frame], c.edges_r, rings=2)
        angle = np.degrees(np.arccos(np.clip((np.trace(turn, axis1=1, axis2=2) - 1) / 2, -1, 1)))
        neutral[frame] = {
            "pose_turn_degrees_largest": float(angle.max()),
            "reframe_round_trip_largest_mm": float(np.linalg.norm(back - d0, axis=1).max()) * 1000,
            "crossings": len(f.known_r), "where": where(f.known_r, P, c.tris_r, height),
            "crossings_in_bundle_pose": len(bundle),
            "makehuman_crossings": len(f.known_m),
            "volume_litres": f.vol_r * 1000, "makehuman_volume_litres": f.vol_m * 1000,
            "girths_mm": {k: v * 1000 for k, v in f.g_r.items()},
            "makehuman_girths_mm": {k: v * 1000 for k, v in f.g_m.items()}}
        log(f"{frame} neutral: ours {f.vol_r * 1000:.1f} l, {len(f.known_r)} crossing pairs of its own "
            f"({neutral[frame]['where']}; {len(bundle)} in the bundle's pose); "
            f"MakeHuman {f.vol_m * 1000:.1f} l, {len(f.known_m)}")
        log(f"  to the bundle's pose and back: the pose turns vertices by up to {angle.max():.0f} degrees; "
            f"round trip changes the largest shape by at most "
            f"{neutral[frame]['reframe_round_trip_largest_mm']:.3f} mm")
        log("  girths mm, ours: " + ", ".join(f"{k} {f.g_r[k] * 1000:.0f}" for k in GIRTHS)
            + "; MakeHuman: " + ", ".join(f"{k} {f.g_m[k] * 1000:.0f}" for k in GIRTHS))
        for name, D in transfer.shape_names(M, gender).items():
            key = f"posed/{frame}/{name}"
            d = shapes[key].astype(np.float64)
            before, mid = raw[key].astype(np.float64), sym[key].astype(np.float64)
            size, size_m = np.linalg.norm(d, axis=1), np.linalg.norm(D, axis=1)
            i, j = int(size.argmax()), int(size_m.argmax())
            asym = np.linalg.norm(before - before[c.mirror] * flip, axis=1) / 2
            ro, rm = roughness(d, c.lap_r, f.h2_r), roughness(D, c.lap_m, f.h2_m)
            mo, mm = size > 1e-3, size_m > 1e-3
            cross_before = transfer.new_crossings(P, P + mid, c.tris_r, f.known_r)
            cross_after = transfer.new_crossings(P, P + d, c.tris_r, f.known_r)
            cross_m = transfer.new_crossings(N, N + D, c.tris_m, f.known_m)
            go = girths(P + d, c.tris_r, f.lm_r, f.levels_r)
            gm = girths(N + D, c.tris_m, f.lm_m, f.levels_m)
            cut = np.linalg.norm(d - mid, axis=1)
            rows.append({
                "frame": frame, "shape": name,
                "largest_move_mm": size[i] * 1000, "where": region(c.sets, P, normals, i),
                "makehuman_largest_move_mm": size_m[j] * 1000,
                "largest_move_places_apart_mm": float(np.linalg.norm(X[j] - P[i])) * 1000,
                "moved_vertices": int(mo.sum()),
                "new_crossings_before_keeping_apart": len(cross_before),
                "new_crossings_where": where(cross_before, P + mid, c.tris_r, height),
                "new_crossings": len(cross_after),
                "kept_apart_vertices": int((cut > 1e-4).sum()),
                "kept_apart_largest_cut_mm": float(cut.max()) * 1000,
                "makehuman_new_crossings": len(cross_m),
                "asymmetry_largest_mm": asym.max() * 1000,
                "asymmetry_rms_share": float(np.sqrt((asym ** 2).mean() / (size ** 2).mean())),
                "roughness_ratio_median": float(np.median(ro[mo]) / np.median(rm[mm])),
                "roughness_ratio_p95": float(np.percentile(ro[mo], 95) / np.percentile(rm[mm], 95)),
                "volume_change_percent": percent(geom.volume(P + d, c.tris_r), f.vol_r),
                "makehuman_volume_change_percent": percent(geom.volume(N + D, c.tris_m), f.vol_m),
                "girth_change_percent": {k: percent(go[k], f.g_r[k]) for k in GIRTHS},
                "makehuman_girth_change_percent": {k: percent(gm[k], f.g_m[k]) for k in GIRTHS},
            })
    return {"neutral": neutral, "shapes": rows}


def methods(work, log=print):
    """The variants of the transfer side by side, over the 16 corner shapes."""
    c = Context(work)
    flip = np.array([-1.0, 1.0, 1.0])
    out = []
    for method in transfer.METHODS:
        shapes, maps = transfer.run(work, method, log=lambda *_: None)
        acc = {"rough50": [], "rough95": [], "asym": [], "vol": [], "girth": [], "cross": 0, "gap": []}
        for frame, gender in FRAMES:
            f = c.frame(frame, gender, maps[frame])
            acc["gap"].append(np.median(maps[frame].gap[maps[frame].ok]) * 1000)
            for name, D in transfer.shape_names(c.M, gender).items():
                if "muscle" not in name:
                    continue
                d = shapes[f"posed/{frame}/{name}"].astype(np.float64)
                size, size_m = np.linalg.norm(d, axis=1), np.linalg.norm(D, axis=1)
                ro, rm = roughness(d, c.lap_r, f.h2_r), roughness(D, c.lap_m, f.h2_m)
                mo, mm = size > 1e-3, size_m > 1e-3
                acc["rough50"].append(np.median(ro[mo]) / np.median(rm[mm]))
                acc["rough95"].append(np.percentile(ro[mo], 95) / np.percentile(rm[mm], 95))
                asym = np.linalg.norm(d - d[c.mirror] * flip, axis=1) / 2
                acc["asym"].append(np.sqrt((asym ** 2).mean() / (size ** 2).mean()))
                acc["vol"].append(abs(percent(geom.volume(f.P + d, c.tris_r), f.vol_r)
                                      - percent(geom.volume(f.N + D, c.tris_m), f.vol_m)))
                go = girths(f.P + d, c.tris_r, f.lm_r, f.levels_r)
                gm = girths(f.N + D, c.tris_m, f.lm_m, f.levels_m)
                acc["girth"] += [abs(percent(go[k], f.g_r[k]) - percent(gm[k], f.g_m[k])) for k in GIRTHS]
                acc["cross"] += len(transfer.new_crossings(f.P, f.P + d, c.tris_r, f.known_r))
        row = {"method": method,
               "surface_gap_median_mm": float(np.mean(acc["gap"])),
               "roughness_ratio_median": float(np.mean(acc["rough50"])),
               "roughness_ratio_p95": float(np.mean(acc["rough95"])),
               "asymmetry_rms_share": float(np.mean(acc["asym"])),
               "volume_change_error_points_mean": float(np.mean(acc["vol"])),
               "volume_change_error_points_worst": float(np.max(acc["vol"])),
               "girth_change_error_points_mean": float(np.mean(acc["girth"])),
               "girth_change_error_points_worst": float(np.max(acc["girth"])),
               "new_crossings_total": int(acc["cross"])}
        log(json.dumps(row))
        out.append(row)
    return out


def markdown(result):
    lines = ["| Frame | Shape | Largest move mm (MakeHuman's) | Where | Places apart mm | New crossings: "
             "before, after keeping apart (MakeHuman's own) | Kept apart: vertices, largest cut mm | "
             "Asymmetry before the fix: largest mm, share | Roughness / source: median, 95% | "
             "Volume % (MakeHuman's) | Girth %: chest, waist, hips, thigh, upper arm (MakeHuman's) |",
             "|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in result["shapes"]:
        g, gm = r["girth_change_percent"], r["makehuman_girth_change_percent"]
        spots = f": {r['new_crossings_where']}" if r["new_crossings_before_keeping_apart"] else ""
        lines.append(
            f"| {r['frame']} | {r['shape']} | {r['largest_move_mm']:.1f} ({r['makehuman_largest_move_mm']:.1f}) "
            f"| {r['where']} | {r['largest_move_places_apart_mm']:.0f} "
            f"| {r['new_crossings_before_keeping_apart']}{spots}, {r['new_crossings']} "
            f"({r['makehuman_new_crossings']}) "
            f"| {r['kept_apart_vertices']}, {r['kept_apart_largest_cut_mm']:.1f} "
            f"| {r['asymmetry_largest_mm']:.2f}, {r['asymmetry_rms_share'] * 100:.1f}% "
            f"| {r['roughness_ratio_median']:.2f}, {r['roughness_ratio_p95']:.2f} "
            f"| {r['volume_change_percent']:+.1f} ({r['makehuman_volume_change_percent']:+.1f}) "
            f"| {', '.join(f'{g[k]:+.0f}' for k in GIRTHS)} ({', '.join(f'{gm[k]:+.0f}' for k in GIRTHS)}) |")
    return "\n".join(lines)


def methods_markdown(rows):
    lines = ["| Method | Gap to MakeHuman's surface, median mm | Roughness / source: median, 95% | "
             "Asymmetry share | Volume change error, points: mean, worst | "
             "Girth change error, points: mean, worst | New crossings, 16 shapes |",
             "|---|---|---|---|---|---|---|"]
    for r in rows:
        lines.append(
            f"| {r['method']} | {r['surface_gap_median_mm']:.2f} "
            f"| {r['roughness_ratio_median']:.2f}, {r['roughness_ratio_p95']:.2f} "
            f"| {r['asymmetry_rms_share'] * 100:.1f}% "
            f"| {r['volume_change_error_points_mean']:.2f}, {r['volume_change_error_points_worst']:.2f} "
            f"| {r['girth_change_error_points_mean']:.2f}, {r['girth_change_error_points_worst']:.1f} "
            f"| {r['new_crossings_total']} |")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--out")
    ap.add_argument("--methods", action="store_true")
    a = ap.parse_args()
    result = measure(a.work)
    print(markdown(result))
    if a.methods:
        result["methods"] = methods(a.work, log=lambda *_: None)
        print()
        print(methods_markdown(result["methods"]))
    if a.out:
        with open(a.out, "w", encoding="utf-8") as f:
            json.dump(result, f, indent=1)


if __name__ == "__main__":
    main()
