"""Measures the open mouth as seen straight from the front (R9, OPEN-14).

    bpy-python -I mouth_measure.py --repo REPO [--parts GLB] --out FILE.json [--png DIR]

Rays from the front (along +Y, Blender space) on a 0.25 mm grid over the
mouth: a ray is in the opening when it meets no skin within 30 mm of the
lips' front; in the opening, what it meets first (teeth, gums, tongue,
the mouth's lining) is what is seen there. Per pose, both frames:

- width, height, area of the opening (mm, mm2);
- widths at 10, 25, 50, 75 and 90% of the height from its top, and
  `bell` = width at 75% / width at 25% (a bell or vase narrows downward:
  well under 1; a broad U keeps its width: near 1);
- `corner_round`: the opening's height 2 mm in from each end over its
  largest height (a pointed corner, a thin wedge, is near 0; a rounded
  end is larger), the mean of both ends;
- `teeth_mm2`, `tongue_mm2`, `gums_mm2`, `lining_mm2`: the area of each
  seen in the opening;
- `upper_band_mm`: in the middle 10 mm, the mean height of lining seen
  between the upper lip and the first tooth below it (the dark band);
- `fv_gap_mm`: at the middle, the lower lip's top less the upper incisors'
  edge, in mm (negative: the lip is under the edge, as for F and V).
"""

import argparse
import json
import os
import sys

import bpy  # noqa: F401
import numpy as np
from mathutils import Vector

ap = argparse.ArgumentParser()
ap.add_argument("--repo", required=True)
ap.add_argument("--parts", default="")
ap.add_argument("--out", required=True)
ap.add_argument("--png", default="")
ap.add_argument("--only", default="")
ap.add_argument("--extra", default="")
ap.add_argument("--no-default", action="store_true")
a = ap.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])

sys.path.insert(0, os.path.join(a.repo, "assets", "characters", "humanoid", "qa"))
import qa_scene as Q  # noqa: E402
import face_checks as FC  # noqa: E402

STEP = 0.00025


def poses():
    extra = []
    if a.extra:
        with open(a.extra, encoding="utf-8") as f:
            extra = list(json.load(f).items())
    if a.no_default:
        return extra
    vis = FC.visemes()
    ex = FC.expressions()
    out = [("rest", {}), ("Jaw_Open 0.5", {"Jaw_Open": 0.5}), ("Jaw_Open 1", {"Jaw_Open": 1.0}),
           ("Smile closed", {"Mouth_Smile_Left": 1.0, "Mouth_Smile_Right": 1.0})]
    for name in ("Happy", "Surprised", "Laughing", "Excited"):
        out.append((name, ex[name]))
    for name in ("U", "WQ", "TH", "FV", "O", "AI"):
        out.append((f"viseme {name}", vis[name]))
    return out + extra


def grid(ch, seal_rest):
    skin = FC.Piece(ch.parts["Body"], ("Skin",))
    lining = FC.Piece(ch.parts["Body"], ("Mouth",))
    teeth_obj = ch.parts["Teeth"]
    pieces = {"teeth": FC.Piece(teeth_obj, ("Teeth",)), "gums": FC.Piece(teeth_obj, ("Gums",)),
              "tongue": FC.Piece(teeth_obj, ("Tongue",)), "lining": lining}
    if "LipSeal" in ch.parts:
        pieces["seal"] = FC.Piece(ch.parts["LipSeal"])
    y0 = float(seal_rest[:, 1].min()) - 0.05
    zc = float(seal_rest[:, 2].mean())
    xs = np.arange(-0.036, 0.036 + 1e-9, STEP)
    zs = np.arange(zc + 0.012, zc - 0.05, -STEP)
    seen = np.zeros((len(zs), len(xs)), dtype=np.int8)  # 0 skin/none, 1.. what
    names = ["skin", "teeth", "gums", "tongue", "lining", "seal", "none"]
    front = None
    for r, z in enumerate(zs):
        for c, x in enumerate(xs):
            o = Vector((float(x), y0, float(z)))
            h = skin.bvh.ray_cast(o, Vector((0.0, 1.0, 0.0)), 0.2)
            d = None if h[0] is None else float(h[3])
            if d is not None and front is None:
                front = d
            if d is not None and d < 0.05 + 0.03:
                continue
            best, what = d if d is not None else 9.0, "none"
            for name, piece in pieces.items():
                if piece.bvh is None:
                    continue
                hh = piece.bvh.ray_cast(o, Vector((0.0, 1.0, 0.0)), 0.2)
                if hh[0] is not None and hh[3] < best:
                    best, what = hh[3], name
            seen[r, c] = names.index(what)
    return xs, zs, seen, names, pieces


def component(seen, names):
    """The open cells joined to the mouth's inside (a flood fill from the
    lining, tongue or teeth cell nearest the middle): what lies past the
    face's outline is not the opening."""
    inside = np.isin(seen, [names.index(n) for n in ("teeth", "gums", "tongue", "lining", "seal")])
    keep = np.zeros(seen.shape, dtype=bool)
    if not inside.any():
        return keep
    rr, cc = np.where(inside)
    mid_c = seen.shape[1] // 2
    k = int(np.argmin(np.abs(cc - mid_c) * 4 + np.abs(rr - rr.mean())))
    stack = [(int(rr[k]), int(cc[k]))]
    open_ = seen > 0
    while stack:
        r, c = stack.pop()
        if r < 0 or c < 0 or r >= seen.shape[0] or c >= seen.shape[1] or keep[r, c] or not open_[r, c]:
            continue
        keep[r, c] = True
        stack.extend(((r + 1, c), (r - 1, c), (r, c + 1), (r, c - 1)))
    return keep


def metrics(xs, zs, seen, names, pieces):
    open_ = component(seen, names)
    seen = np.where(open_, seen, 0)
    cell = (STEP * 1000) ** 2
    out = {"area_mm2": round(float(open_.sum()) * cell, 1)}
    for name in ("teeth", "gums", "tongue", "lining", "seal"):
        out[f"{name}_mm2"] = round(float((seen == names.index(name)).sum()) * cell, 1)
    rows = np.where(open_.any(axis=1))[0]
    cols = np.where(open_.any(axis=0))[0]
    if not len(rows):
        out.update({"width_mm": 0.0, "height_mm": 0.0})
        return out
    top, bottom = rows.min(), rows.max()
    h = bottom - top + 1
    out["height_mm"] = round(h * STEP * 1000, 2)
    out["width_mm"] = round((cols.max() - cols.min() + 1) * STEP * 1000, 2)
    widths = {}
    for f in (0.1, 0.25, 0.5, 0.75, 0.9):
        r = int(top + f * (h - 1))
        widths[f"{int(f * 100)}"] = round(float(open_[r].sum()) * STEP * 1000, 2)
    out["widths_mm"] = widths
    out["bell"] = round(widths["75"] / max(widths["25"], 1e-6), 3)
    heights = open_.sum(axis=0)
    ends = []
    for c, way in ((cols.min(), 1), (cols.max(), -1)):
        k = c + way * int(round(0.002 / STEP))
        ends.append(float(heights[k]) / max(float(heights.max()), 1.0))
    out["corner_round"] = round(float(np.mean(ends)), 3)
    # The dark band: middle 10 mm, rows from the opening's top down to the
    # first tooth, of lining or seal.
    mid = np.where(np.abs(xs) < 0.005)[0]
    band = []
    for c in mid:
        col = seen[:, c]
        idx = np.where(col > 0)[0]
        if not len(idx):
            continue
        n = 0
        for r in range(idx.min(), len(col)):
            if col[r] in (names.index("lining"), names.index("seal")):
                n += 1
            elif col[r] == 0:
                break
            else:
                break
        band.append(n * STEP * 1000)
    out["upper_band_mm"] = round(float(np.mean(band)), 2) if band else 0.0
    # F and V: the upper incisors' edge against the lower lip's top, at the middle.
    teeth = pieces["teeth"].co
    centre = teeth[np.abs(teeth[:, 0]) < 0.004]
    upper = centre[centre[:, 2] > np.median(teeth[:, 2])]
    if len(upper):
        edge = float(upper[:, 2].min())
        c = int(np.argmin(np.abs(xs)))
        col = seen[:, c]
        # The lower lip's top: the first skin row below the incisors' edge
        # going down (rows go down in z).
        below = [r for r in range(len(zs)) if zs[r] < edge + 0.004 and col[r] == 0]
        # skip the upper lip: the first skin row after some non-skin, or
        # the first one under the edge.
        top_lip = None
        seen_open = False
        for r in range(len(zs)):
            if zs[r] > edge + 0.006:
                continue
            if col[r] > 0:
                seen_open = True
            elif seen_open or zs[r] < edge:
                top_lip = float(zs[r])
                break
        if top_lip is not None:
            out["fv_gap_mm"] = round((top_lip - edge) * 1000, 2)
        del below
    return out


def save_png(path, seen, names):
    from PIL import Image
    colors = {"skin": (200, 160, 140), "teeth": (250, 250, 240), "gums": (220, 110, 120),
              "tongue": (230, 140, 160), "lining": (90, 20, 30), "seal": (160, 40, 200), "none": (0, 0, 0)}
    img = np.zeros(seen.shape + (3,), dtype=np.uint8)
    for k, name in enumerate(names):
        img[seen == k] = colors[name]
    Image.fromarray(img).resize((seen.shape[1] * 2, seen.shape[0] * 2), Image.NEAREST).save(path)


def main():
    parts_path, _, custom_path = Q.package_paths("realistic")
    parts_path = a.parts or parts_path
    custom = Q.load_custom(custom_path)
    Q.reset()
    ch = Q.Character(parts_path, merge_vertices=True, custom=custom)
    report = {}
    for frame_name, frame in (("masculine", 0.0), ("feminine", 1.0)):
        FC.pose(ch, frame, {})
        seal_rest = FC.Piece(ch.parts["LipSeal"]).co
        for name, w in poses():
            if a.only and a.only not in name:
                continue
            FC.pose(ch, frame, w)
            xs, zs, seen, names, pieces = grid(ch, seal_rest)
            m = metrics(xs, zs, seen, names, pieces)
            report.setdefault(frame_name, {})[name] = m
            print(frame_name, name, json.dumps(m), flush=True)
            if a.png:
                os.makedirs(a.png, exist_ok=True)
                save_png(os.path.join(a.png, f"{frame_name}-" + "".join(ch if ch.isalnum() else "_" for ch in name) + ".png"), seen, names)
    with open(a.out, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=1, sort_keys=True)


main()
