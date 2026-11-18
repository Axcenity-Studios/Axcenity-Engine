"""Builds the crew-neck T-shirt's sources (Blender's Python, bpy 5.x):
Teagan drafted at the realistic masculine body's measurements, sewn and
draped (`tee.py`, `drape.py`), finished at three levels of detail
(`finish.py`) and baked (`textures.py`). Writes, into `--out`:

- `CrewTee_LOD0.obj`, `_LOD1.obj`, `_LOD2.obj` (+ `.mtl`): the garment
  in the body's rest pose, glTF's axes, for `import_garment`;
- `CrewTee_color.jpg`, `_normal.jpg`, `_orm.jpg`: the 2048 px maps;
- `CrewTee.json`: what was measured (the draft's lengths, the drape's
  seams, counts, the atlas).

    python -I assets/characters/realistic/clothing/build_tee.py \
        --body assets/characters/realistic/RealisticParts.glb --tiles <folder> --out <folder>

`--tiles` is the folder of the CC0 fabric tiles (`fabrics.py` fetches
them). The feminine frame wears the same garment through its binding
(the body's `Frame_Feminine` shape carried onto it, `avatar::fit`);
`--frame feminine` drapes the feminine draft instead, to compare.
"""

import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import numpy as np
import bpy
from mathutils.bvhtree import BVHTree

import body as bodymod
import drape
import finish
import tee
import teagan
import textures

# The levels' spacing (mm): LOD0 within a top's 6k triangles with its hems
# and band; each next level about a third of the one before.
LEVELS = (23.5, 39.0, 68.0)
# The simulation's spacing (mm).
FINE = 10.0
# Teagan's options for this tee: its default fit, a regular length (the
# realistic body's waist-to-hip is short, so Teagan's 15% ends at the
# hip bone; 40%, within its range of -20 to 60%, ends at the upper seat
# as a regular T-shirt does), and its shallowest
# and narrowest neckline (depth 20% of its range 20 to 40%, width 10% of
# 10 to 50%: an 18 cm crew neck sitting at the neck's base; the defaults
# are a wide scoop that leaves the chest's and shoulders' top bare, so
# the body's torso region couldn't be hidden under it).
OPTIONS = dict(teagan.OPTIONS, lengthBonus=0.40, necklineDepth=0.20, necklineWidth=0.10)
HEM = 0.022
SLEEVE_HEM = 0.020
BAND = {"height": 0.017, "inside": 0.008}
# How far out of the skin a piece starts, at least (m).
START_CLEAR = 0.010


def pieces(m, spacing):
    return tee.pieces(m, spacing, OPTIONS)


def outward(name, pos, quads, place, joints):
    """Whether a piece's quads, as made, face the body (to be turned)."""
    total = 0.0
    for a, b, c, d in quads[:: max(1, len(quads) // 64)]:
        n = np.cross(pos[c] - pos[a], pos[d] - pos[b])
        mid = (pos[a] + pos[c]) / 2
        if name in ("front", "back"):
            radial = mid - np.array([0.0, place.cy, mid[2]])
        else:
            radial = mid - joints["LeftUpperArm" if name == "sleeve_left" else "RightUpperArm"]
        total += float(n @ radial)
    return total < 0


def drape_tee(b, frame, frames):
    panels, seams, d = pieces(b.measurements, FINE)
    place = tee.Placement(b, d)
    verts, faces, offsets, flip = [], [], {}, {}
    for name, p in panels.items():
        offsets[name] = len(verts)
        pos = place.place(p)
        q = p.quads()
        flip[name] = outward(name, pos, q, place, b.joints)
        if flip[name]:
            q = q[:, ::-1]
        verts.extend(pos)
        faces.extend((q + offsets[name]).tolist())
    verts = np.array(verts)
    pairs = tee.seam_pairs(panels, seams, offsets)
    bvh = BVHTree.FromPolygons([tuple(p) for p in b.P], [tuple(t) for t in b.tris])
    moved = 0
    for k, p in enumerate(verts):
        loc, nor, _, _ = bvh.find_nearest(p)
        if loc is not None and (np.array(p) - np.array(loc)) @ np.array(nor) < START_CLEAR:
            verts[k] = np.array(loc) + np.array(nor) * (START_CLEAR + 0.002)
            moved += 1
    drape.collision_body(frame)
    # Gravity eased in over 70 frames: the shoulder seams, which start a
    # body's depth apart, close before the cloth hangs from them.
    raw = drape.simulate(verts, faces, pairs, drape.Settings(frames=frames, gravity_full=70))
    gaps = drape.gap_report(raw, pairs)
    draped = drape.join(raw, pairs)
    by_piece = {name: draped[offsets[name]:offsets[name] + p.rows * p.cols] for name, p in panels.items()}
    return panels, by_piece, flip, d, {"moved_out": moved, "seam_gap_max_mm": gaps[0] * 1000,
                                       "seam_gap_mean_mm": gaps[1] * 1000, "corners": len(verts)}


def spec(flip):
    return {
        "material": "Jersey",
        "flip": flip,
        "hems": [("front", "hem", HEM), ("back", "hem", HEM),
                 ("sleeve_left", "hem", SLEEVE_HEM), ("sleeve_right", "hem", SLEEVE_HEM)],
        "bands": [dict(BAND, name="neckband", material="Rib", runs=[
            ("front", "L_neck", False), ("front", "R_neck", False),
            ("back", "R_neck", True), ("back", "L_neck", True)])],
    }


class Atlas:
    def __init__(self, scale, offsets, margin):
        self.scale, self.offsets, self.margin = scale, offsets, margin

    def to_uv(self, island, mm):
        off = np.array(self.offsets[island])
        return ((np.asarray(mm) + off) * self.scale + self.margin) / textures.SIZE


def fabric(tiles, lod0, panels):
    """The tee's fabric: Poly Haven's Cotton Jersey (CC0) for the body and
    sleeves, the same knit as a rib for the neck band; stitch rows and
    seams in each piece's pattern millimetres."""
    def tile(name, kind):
        return os.path.join(tiles, name, f"{name}_{kind}_2k.jpg")

    def rows(piece, segment):
        p = panels[piece]
        return np.array([p.uv[r, c] for r, c in p.segments[segment]])

    # Each scale is the atlas's, so the tiles are filtered to its texels.
    return lambda scale: {
        "base": [0.80, 0.80, 0.79],
        "stitch_length": 3.0,
        "thread_rough": 0.55,
        "islands": {
            "*": {"mm": 263.6, "stretch": 1.0, "amplitude": 0.18, "contrast": 0.6,
                  "height": textures.load_tile(tile("cotton_jersey", "disp"), 263.6, scale),
                  "light": textures.load_tile(tile("cotton_jersey", "diff"), 263.6, scale),
                  "light_mean": 0.60,
                  "rough": textures.load_tile(tile("cotton_jersey", "rough"), 263.6, scale),
                  "rough_lo": 0.72, "rough_hi": 0.92},
            "neckband": {"mm": 263.6, "stretch": 1.0, "amplitude": 0.12, "contrast": 0.5,
                         "height": textures.load_tile(tile("cotton_jersey", "disp"), 263.6, scale),
                         "light": textures.load_tile(tile("cotton_jersey", "diff"), 263.6, scale),
                         "light_mean": 0.60,
                         "rough": textures.load_tile(tile("cotton_jersey", "rough"), 263.6, scale),
                         "rough_lo": 0.75, "rough_hi": 0.92, "wales": 2.4},
        },
        "seams": {
            "front": [rows("front", "left"), rows("front", "right"), rows("front", "L_shoulder"),
                      rows("front", "R_shoulder"), rows("front", "L_arm"), rows("front", "R_arm"),
                      np.vstack([rows("front", "L_neck"), rows("front", "R_neck")])],
            "back": [rows("back", "left"), rows("back", "right"), rows("back", "L_shoulder"),
                     rows("back", "R_shoulder"), rows("back", "L_arm"), rows("back", "R_arm"),
                     np.vstack([rows("back", "L_neck"), rows("back", "R_neck")])],
            "sleeve_left": [rows("sleeve_left", "left"), rows("sleeve_left", "right"),
                            np.vstack([rows("sleeve_left", "cap_back"), rows("sleeve_left", "cap_front")])],
            "sleeve_right": [rows("sleeve_right", "left"), rows("sleeve_right", "right"),
                             np.vstack([rows("sleeve_right", "cap_back"), rows("sleeve_right", "cap_front")])],
        },
        # A hem's cover stitch: two rows 6.4 mm apart, 9 and 15.4 mm up
        # from the fold (a twin-needle hem); the neck band's topstitch
        # 3 mm below its seam on the shirt.
        "stitches": {
            "front": [(rows("front", "hem"), 9.0), (rows("front", "hem"), 15.4),
                      (np.vstack([rows("front", "L_neck"), rows("front", "R_neck")]), 3.0)],
            "back": [(rows("back", "hem"), 9.0), (rows("back", "hem"), 15.4),
                     (np.vstack([rows("back", "L_neck"), rows("back", "R_neck")]), 3.0)],
            "sleeve_left": [(rows("sleeve_left", "hem"), 9.0), (rows("sleeve_left", "hem"), 15.4)],
            "sleeve_right": [(rows("sleeve_right", "hem"), 9.0), (rows("sleeve_right", "hem"), 15.4)],
        },
    }


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
    fine, draped, flip, draft, report = drape_tee(b, args.frame, args.frames)
    report["measurements"] = b.measurements
    report["draft"] = {k: {s: round(v.length(), 1) for s, v in draft[k].items()} for k in ("front", "back", "sleeve")}
    report["neckband_mm"] = draft["neckband"]
    bvh = BVHTree.FromPolygons([tuple(p) for p in b.P], [tuple(t) for t in b.tris])

    def skin(p):
        loc, nor, _, _ = bvh.find_nearest(p)
        return np.array(loc), np.array(nor)

    levels = []
    for spacing in LEVELS:
        g = finish.Garment(pieces, b.measurements, spacing, fine, draped, spec(flip))
        g.mesh.P = list(g.mesh.P)
        report.setdefault("band_moved_mm", []).append(1000 * finish.clear_bands(g, skin))
        g.mesh.P = np.array(g.mesh.P)
        levels.append(g)
    # One atlas for every level: packed on LOD0.
    uv0, scale, offsets, margin = finish.pack(levels[0].mesh)
    atlas = Atlas(scale, offsets, margin)
    report["atlas"] = {"texels_per_mm": scale, "size": textures.SIZE}
    with open(os.path.join(args.out, "CrewTee.mtl"), "w", newline="\n") as f:
        # A map named keeps the texture coordinates through the import's
        # reader (it reads an OBJ's only for a textured material).
        f.write("newmtl Jersey\nKd 0.8 0.8 0.79\nmap_Kd CrewTee_color.jpg\n"
                "newmtl Rib\nKd 0.8 0.8 0.79\nmap_Kd CrewTee_color.jpg\n")
    report["levels"] = []
    for k, g in enumerate(levels):
        mesh = g.mesh
        mesh.uv01 = np.array([atlas.to_uv(i, p) for i, p in zip(mesh.island, mesh.uv)])
        all_quads = [q for qs in mesh.quads.values() for q in qs]
        mesh.normals = finish.normals(mesh.P, all_quads, mesh.welds)
        finish.write_obj(os.path.join(args.out, f"CrewTee_LOD{k}.obj"), mesh, mesh.uv01, "CrewTee.mtl")
        report["levels"].append({"spacing_mm": LEVELS[k], "corners": len(mesh.P), "triangles": finish.triangles(mesh)})
        print(f"LOD{k}: {len(mesh.P)} corners, {finish.triangles(mesh)} triangles")
    # The fine drape per piece for the bake.
    fine_data = {}
    for name, p in fine.items():
        pos = draped[name]
        q = p.quads()
        if flip[name]:
            q = q[:, ::-1]
        tris = np.concatenate([q[:, [0, 1, 2]], q[:, [0, 2, 3]]])
        nor = finish.normals(pos, [tuple(x) for x in q], [])
        fine_data[name] = (p.corners(), pos, nor, tris)
    report["bake"] = textures.bake(levels[0].mesh, fine_data, atlas, fabric(args.tiles, levels[0].mesh, levels[0].pieces)(scale),
                                   os.path.join(args.out, "CrewTee"))
    with open(os.path.join(args.out, "CrewTee.json"), "w", newline="\n") as f:
        json.dump(report, f, indent=1, default=float)
    print(json.dumps({k: v for k, v in report.items() if k not in ("measurements",)}, default=float, indent=1))


if __name__ == "__main__":
    main()
