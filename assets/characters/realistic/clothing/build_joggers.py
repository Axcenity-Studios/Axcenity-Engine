"""Builds the joggers' sources (Blender's Python, bpy 5.x): Paco drafted
at the realistic masculine body's measurements, sewn and draped with the
waist and hems gathered on elastic (`joggers.py`, `drape.py`), finished
at three levels of detail with a waistband casing, rib cuffs and a
drawcord (`finish.py`), and baked (`textures.py`). Writes into `--out`
the same files as `build_tee.py`, named `Joggers_*`.

    python -I assets/characters/realistic/clothing/build_joggers.py \
        --body assets/characters/realistic/RealisticParts.glb --tiles <folder> --out <folder>

Why joggers and not jeans first: the clothing references' first wave
(references/clothing §5.3) orders them so, and the binding fit suits
them. Joggers are soft fleece gathered at the waist and the ankles, so
the cloth stays near the body and moves with it, and the gathered cuff
sits clear of the shoe. Jeans are stiff denim that stacks at the shoe and
holds its own shape through a bend, which needs cloth simulation at run
time (ADR-0035's fallback) more than binding. Joggers also prove the
waist against the tee's hem, the layering this wave must show.
"""

import argparse
import json
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np
import bpy  # noqa: F401 (loads mathutils)
from mathutils.bvhtree import BVHTree

import body as bodymod
import drape
import finish
import joggers
import paco
import textures
from build_tee import Atlas, START_CLEAR

LEVELS = (24.0, 40.0, 70.0)
FINE = 11.0
# Paco's options for these joggers: its own (`paco.OPTIONS`), cropped to
# the ankle as joggers are (lengthBonus -11%, within its -15 to 10%: the
# cuff ends over the ankle bone, above the foot and a shoe's collar), and a
# relaxed leg: Paco's leg below the seat is the heel's girth plus
# `heelEase` (5%), which on the realistic body is narrower than its knee
# (a legging); 35% (of its 0 to 50%) is a jogger's 45 cm.
OPTIONS = dict(paco.OPTIONS, lengthBonus=-0.11, heelEase=0.35)
WAISTBAND_INSIDE = 0.012
CUFF_INSIDE = 0.012
# The drawcord: its round section, how far it hangs below the waistband's
# top, and the aglet at each end.
CORD_RADIUS = 0.0025
CORD_HANG = 0.24
AGLET = 0.022
GROMMET_APART = 0.024


def pieces(m, spacing):
    return joggers.pieces(m, spacing, OPTIONS)


def widths_of(panels):
    out = {}
    for name, p in panels.items():
        d = np.linalg.norm(np.diff(p.uv, axis=1), axis=2).sum(axis=1)
        for r in range(p.rows):
            out[(name, r)] = d[r]
    return out


def drape_joggers(b, frame, frames):
    panels, seams, d = pieces(b.measurements, FINE)
    place = joggers.Placement(b, d)
    widths = widths_of(panels)
    verts, faces, offsets, flip, pins = [], [], {}, {}, []
    for name, p in panels.items():
        offsets[name] = len(verts)
        pos, pinned = place.place(p, widths)
        q = p.quads()
        # A piece faces out when its quads' normals point away from the
        # leg's (or hips') middle.
        side = 1 if name.endswith("left") else -1
        total = 0.0
        for a, bq, c, dd in q[:: max(1, len(q) // 64)]:
            n = np.cross(pos[c] - pos[a], pos[dd] - pos[bq])
            mid = (pos[a] + pos[c]) / 2
            leg = b.joints["LeftUpperLeg" if side > 0 else "RightUpperLeg"]
            radial = mid - np.array([leg[0] if mid[2] < place.crotch_z else 0.0, leg[1], mid[2]])
            total += float(n @ radial)
        flip[name] = total < 0
        if flip[name]:
            q = q[:, ::-1]
        verts.extend(pos)
        faces.extend((q + offsets[name]).tolist())
        pins.extend(k + offsets[name] for k in pinned)
    verts = np.array(verts)
    pairs = joggers.seam_pairs(panels, seams, offsets)
    bvh = BVHTree.FromPolygons([tuple(p) for p in b.P], [tuple(t) for t in b.tris])
    moved = 0
    pinset = set(pins)
    for k, p in enumerate(verts):
        if k in pinset:
            continue
        loc, nor, _, _ = bvh.find_nearest(p)
        if loc is not None and (np.array(p) - np.array(loc)) @ np.array(nor) < START_CLEAR:
            verts[k] = np.array(loc) + np.array(nor) * (START_CLEAR + 0.002)
            moved += 1
    drape.collision_body(frame)
    settings = drape.Settings(frames=frames, gravity_full=60, mass=0.3, tension=40.0, compression=40.0,
                              shear=10.0, bending=0.5)
    raw = drape.simulate(verts, faces, pairs, settings, pins=pins)
    gaps = drape.gap_report(raw, pairs)
    draped = drape.join(raw, pairs)
    by_piece = {name: draped[offsets[name]:offsets[name] + p.rows * p.cols] for name, p in panels.items()}
    return panels, by_piece, flip, d, {"moved_out": moved, "seam_gap_max_mm": gaps[0] * 1000,
                                       "seam_gap_mean_mm": gaps[1] * 1000, "corners": len(verts), "pinned": len(pins)}


def spec(flip, d):
    wb = d["waistband"]["width"] / 1000
    cuff = d["cuff"]["width"] / 1000
    return {
        "material": "Fleece",
        "flip": flip,
        "hems": [],
        "bands": [
            {"name": "waistband", "material": "Rib", "height": wb, "inside": WAISTBAND_INSIDE,
             "direction": (0.0, 0.0, 1.0),
             "runs": [("front_left", "waist", False), ("front_right", "waist", True),
                      ("back_right", "waist", False), ("back_left", "waist", True)]},
            {"name": "cuff_left", "material": "Rib", "height": cuff, "inside": CUFF_INSIDE,
             "direction": (0.0, 0.0, -1.0),
             "runs": [("front_left", "hem", False), ("back_left", "hem", True)]},
            {"name": "cuff_right", "material": "Rib", "height": cuff, "inside": CUFF_INSIDE,
             "direction": (0.0, 0.0, -1.0),
             "runs": [("front_right", "hem", False), ("back_right", "hem", True)]},
        ],
    }


def tube(mesh, path, radius, material, island, sides=6, loop=False):
    """A closed round tube along `path` (its ends capped flat with two
    quads each; with `loop` a ring, the path's end joined to its start),
    texture coordinates round it and along it (mm)."""
    path = np.asarray(path)
    if loop:
        path = np.vstack([path, path[:1]])
    rings = []
    along = 0.0
    for i, p in enumerate(path):
        t = path[min(i + 1, len(path) - 1)] - path[max(i - 1, 0)]
        t /= np.linalg.norm(t)
        if loop and i in (0, len(path) - 1):
            t = path[1] - path[-2]
            t /= np.linalg.norm(t)
        u = np.cross(t, [0.0, 1.0, 0.0])
        if np.linalg.norm(u) < 1e-6:
            u = np.cross(t, [1.0, 0.0, 0.0])
        u /= np.linalg.norm(u)
        v = np.cross(t, u)
        if i:
            along += np.linalg.norm(path[i] - path[i - 1]) * 1000
        ring = []
        for k in range(sides + 1):
            a = 2 * math.pi * k / sides
            q = p + radius * (math.cos(a) * u + math.sin(a) * v)
            ring.append(mesh.add(q, (k * 2 * math.pi * radius * 1000 / sides, along), island, "cord"))
        mesh.welds.append((ring[0], ring[-1]))
        rings.append(ring)
    for a, b in zip(rings, rings[1:]):
        for k in range(sides):
            mesh.quad(material, (a[k], a[k + 1], b[k + 1], b[k]))
    if loop:
        for a, b in zip(rings[0], rings[-1]):
            mesh.welds.append((a, b))
        return
    # Caps: an even polygon as quads fanned from its first corner (a
    # hexagon is two quads, a square one).
    caps = [(0, k, k + 1, k + 2) for k in range(1, sides - 1, 2)]
    for ring, rev in ((rings[0], True), (rings[-1], False)):
        for q in caps:
            qq = tuple(ring[i] for i in q)
            mesh.quad(material, qq[::-1] if rev else qq)


# The drawcord's detail at each level: its step along (m) and its sides.
CORD_DETAIL = ((0.01, 6), (0.025, 4), (0.05, 4))


def add_drawcord(g, level=0):
    """Two cords out of grommets either side of the waistband's front
    middle, hanging down the front over the cloth, each ending in an
    aglet; a grommet round each (a ring). Written into the level's mesh
    (`g.mesh`) before the arrays are made final."""
    mesh = g.mesh
    P = np.asarray(mesh.P)
    kind = np.array(mesh.kind)
    island = np.array(mesh.island)
    band = np.nonzero(island == "waistband")[0]
    shell = np.nonzero(kind == "shell")[0]
    # The band's front middle: its corners nearest x = 0 in front.
    front = band[P[band, 1] < np.median(P[band, 1])]
    top = P[front, 2].max()
    bottom = P[front, 2].min()
    mid_z = (top + bottom) / 2
    step, sides = CORD_DETAIL[level]
    for side in (1, -1):
        x0 = side * GROMMET_APART / 2
        near = front[np.argsort(np.abs(P[front, 0] - x0) + np.abs(P[front, 2] - mid_z))[:4]]
        start = P[near].mean(axis=0)
        path = [start + np.array([0, -0.003, 0])]
        z = start[2]
        while z > top - CORD_HANG:
            z -= step
            x = x0 + side * 0.002 * (start[2] - z) / 0.01
            cand = shell[(np.abs(P[shell, 0] - x) < 0.02) & (np.abs(P[shell, 2] - z) < 0.02)]
            y = P[cand, 1].min() - 0.0045 if len(cand) else path[-1][1]
            path.append(np.array([x, min(y, path[-1][1] + 0.004), z]))
        path = np.array(path)
        # The aglet: the cord's last `AGLET`, a little thicker.
        end = path[-1]
        direction = (path[-1] - path[-2]) / np.linalg.norm(path[-1] - path[-2])
        aglet = np.array([end - direction * AGLET, end])
        tube(mesh, path, CORD_RADIUS, "Cord", f"cord_{'l' if side > 0 else 'r'}", sides=sides)
        tube(mesh, aglet + direction * 0.002, CORD_RADIUS * 1.25, "Aglet", f"aglet_{'l' if side > 0 else 'r'}",
             sides=sides)
        # The grommet: a flat ring round the cord's exit.
        n = np.array([0.0, -1.0, 0.0])
        ring = []
        for k in range(12):
            a = 2 * math.pi * k / 12
            ring.append(start + 0.0045 * np.array([math.cos(a), 0, math.sin(a)]) + n * 0.0012)
        tube(mesh, ring[:: 12 // (12 if level == 0 else 6)], 0.0011, "Aglet", f"grommet_{'l' if side > 0 else 'r'}",
             sides=sides, loop=True)


def fabric(tiles, panels):
    def tile(name, kind):
        return os.path.join(tiles, name, f"{name}_{kind}_2k.jpg")

    def rows(piece, segment):
        p = panels[piece]
        return np.array([p.uv[r, c] for r, c in p.segments[segment]])

    def pocket(piece):
        """Paco's front pocket opening: from the waist, 45 mm in from the
        side, down to the outseam 150 mm under the waist."""
        p = panels[piece]
        top = p.uv[-1]
        d = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(top, axis=0), axis=1))])
        a = np.array([np.interp(45, d, top[:, 0]), np.interp(45, d, top[:, 1])])
        side = p.uv[::-1, 0]
        e = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(side, axis=0), axis=1))])
        b = np.array([np.interp(150, e, side[:, 0]), np.interp(150, e, side[:, 1])])
        return np.linspace(a, b, 24)

    def melange(scale, rough_lo=0.78, rough_hi=0.95, **extra):
        spec = {"mm": 273.5, "stretch": 1.0, "amplitude": 0.22, "contrast": 0.9,
                "height": textures.load_tile(tile("jogging_melange", "disp"), 273.5, scale),
                "light": textures.load_tile(tile("jogging_melange", "diff"), 273.5, scale),
                "light_mean": 0.62,
                "rough": textures.load_tile(tile("jogging_melange", "rough"), 273.5, scale),
                "rough_lo": rough_lo, "rough_hi": rough_hi}
        spec.update(extra)
        return spec

    def make(scale):
        seams, stitches = {}, {}
        for side in ("left", "right"):
            for name in ("front", "back"):
                piece = f"{name}_{side}"
                seams[piece] = [rows(piece, "outseam"), rows(piece, "inseam"), rows(piece, "rise")]
            seams[f"front_{side}"].append(pocket(f"front_{side}"))
            stitches[f"front_{side}"] = [(pocket(f"front_{side}"), 6.0)]
        return {
            "base": [0.78, 0.78, 0.78],
            "stitch_length": 3.2,
            "thread_rough": 0.6,
            "islands": {
                "*": melange(scale),
                "waistband": melange(scale, amplitude=0.18, wales=2.6),
                "cuff_left": melange(scale, amplitude=0.18, wales=2.6),
                "cuff_right": melange(scale, amplitude=0.18, wales=2.6),
            },
            "seams": seams,
            "stitches": stitches,
        }
    return make


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--body", required=True)
    parser.add_argument("--tiles", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--frame", default="masculine")
    parser.add_argument("--frames", type=int, default=200)
    args = parser.parse_args()
    os.makedirs(args.out, exist_ok=True)
    b = bodymod.load(args.body, args.frame)
    fine, draped, flip, draft, report = drape_joggers(b, args.frame, args.frames)
    # The fine drape, kept for looking at (not shipped).
    np.savez(os.path.join(args.out, "Joggers_fine.npz"),
             **{name: draped[name].reshape(p.rows, p.cols, 3) for name, p in fine.items()})
    report["measurements"] = b.measurements
    report["draft"] = {k: {s: round(v.length(), 1) for s, v in draft[k].items()} for k in ("front", "back")}
    report["waistband_mm"] = draft["waistband"]
    report["cuff_mm"] = draft["cuff"]
    bvh = BVHTree.FromPolygons([tuple(p) for p in b.P], [tuple(t) for t in b.tris])

    def skin(p):
        loc, nor, _, _ = bvh.find_nearest(p)
        return np.array(loc), np.array(nor)

    levels = []
    for level, spacing in enumerate(LEVELS):
        g = finish.Garment(pieces, b.measurements, spacing, fine, draped, spec(flip, draft))
        mesh = g.mesh
        # The cords go in as lists (the mesh's arrays were made final).
        mesh.P = list(mesh.P)
        report.setdefault("band_moved_mm", []).append(1000 * finish.clear_bands(g, skin))
        mesh.uv = list(mesh.uv)
        add_drawcord(g, level)
        mesh.arrays()
        levels.append(g)
    uv0, scale, offsets, margin = finish.pack(levels[0].mesh)
    atlas = Atlas(scale, offsets, margin)
    report["atlas"] = {"texels_per_mm": scale, "size": textures.SIZE}
    with open(os.path.join(args.out, "Joggers.mtl"), "w", newline="\n") as f:
        for name in ("Fleece", "Rib", "Cord", "Aglet"):
            f.write(f"newmtl {name}\nKd 0.78 0.78 0.78\nmap_Kd Joggers_color.jpg\n")
    report["levels"] = []
    for k, g in enumerate(levels):
        mesh = g.mesh
        mesh.uv01 = np.array([atlas.to_uv(i, p) for i, p in zip(mesh.island, mesh.uv)])
        all_quads = [q for qs in mesh.quads.values() for q in qs]
        mesh.normals = finish.normals(mesh.P, all_quads, mesh.welds)
        finish.write_obj(os.path.join(args.out, f"Joggers_LOD{k}.obj"), mesh, mesh.uv01, "Joggers.mtl")
        report["levels"].append({"spacing_mm": LEVELS[k], "corners": len(mesh.P), "triangles": finish.triangles(mesh)})
        print(f"LOD{k}: {len(mesh.P)} corners, {finish.triangles(mesh)} triangles")
    fine_data = {}
    for name, p in fine.items():
        pos = draped[name]
        q = p.quads()
        if flip[name]:
            q = q[:, ::-1]
        tris = np.concatenate([q[:, [0, 1, 2]], q[:, [0, 2, 3]]])
        nor = finish.normals(pos, [tuple(x) for x in q], [])
        fine_data[name] = (p.corners(), pos, nor, tris)
    report["bake"] = textures.bake(levels[0].mesh, fine_data, atlas, fabric(args.tiles, levels[0].pieces)(scale),
                                   os.path.join(args.out, "Joggers"))
    with open(os.path.join(args.out, "Joggers.json"), "w", newline="\n") as f:
        json.dump(report, f, indent=1, default=float)
    print(json.dumps({k: v for k, v in report.items() if k not in ("measurements",)}, default=float, indent=1))


if __name__ == "__main__":
    main()
