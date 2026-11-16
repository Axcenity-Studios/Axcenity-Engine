"""Step 5 of the MakeHuman transfer spike: the comparison sheets.

    python -I sheets.py --work WORK --out ../../../docs/development/character-studio/co5-spike
        [--only masculine,feminine,details,grid,bundle,extras] [--samples 24]

(`python` has the `bpy` module and Pillow; run `transfer.py` first.)

- `comparison_masculine.png`, `comparison_feminine.png`: six settings in
  columns (neutral; heavy; heavy and soft; slim; muscular; heavy and
  muscular: MakeHuman's macro sliders at their ends), and in rows
  MakeHuman's own mesh and the realistic body with the transferred shapes,
  from the front and three-quarter. Both in MakeHuman's pose.
- `details.png`: the trunk close up, front and back, for the settings
  where anatomy has most to show; MakeHuman's mesh above ours, both frames.
- `grid.png`: the realistic body over the muscle x weight plane (five
  steps each way, so most tiles are mixtures of corner shapes), both
  frames, three-quarter view.
- `bundle_pose.png`: the same shapes turned to the bundle's own pose (arms
  about 70 degrees below level), where the realistic body is vendored.
  The keep-apart step ran in MakeHuman's pose, so here the heavy arms
  meet the chest again: it has to run in the pose the engine rests in.
- `extras.png`: regional targets carried the same way (pregnant belly,
  buttocks, pectorals, toned stomach, breast size).

Clay, the humanoid QA studio's lights (Cycles, fixed seed) with a harder
key, orthographic, on flat grey.
"""

import argparse
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import mix  # noqa: E402
import render  # noqa: E402

# (label, muscle, weight) in -1..1.
SETTINGS = (
    ("neutral", 0, 0),
    ("heavy (weight max)", 0, 1),
    ("heavy + soft (muscle min, weight max)", -1, 1),
    ("slim (weight min)", 0, -1),
    ("muscular (muscle max)", 1, 0),
    ("heavy + muscular (both max)", 1, 1),
)
VIEWS = (("front", 0), ("three-quarter", 35))
TILE = (360, 660)
LABEL = 26


def font(size=15):
    for name in ("arial.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


class Sheet:
    def __init__(self, studio, tmp):
        self.st, self.tmp, self.n = studio, tmp, 0

    def tile(self, verts, faces, azimuth, label, colors=None, centre=(0, 0, 0.9), span=1.9, relative=False):
        """One labelled render. `relative`: `centre`'s height and `span`
        are shares of the figure's height, so two bodies of different
        heights are framed alike."""
        verts = render.stand(verts)
        if relative:
            h = verts[:, 2].max()
            centre, span = (centre[0], centre[1], centre[2] * h), span * h
        self.st.mesh(verts, faces, colors)
        path = os.path.join(self.tmp, f"tile_{self.n}.png")
        self.n += 1
        self.st.shoot(path, azimuth=azimuth, centre=centre, span=span)
        im = render.flat(path)
        ImageDraw.Draw(im).text((8, 6), label, fill=(20, 20, 20), font=font())
        return im


def paste(rows, path, title=None):
    w, h = rows[0][0].size
    top = LABEL + 8 if title else 0
    sheet = Image.new("RGB", (w * max(len(r) for r in rows), h * len(rows) + top), (235, 235, 235))
    if title:
        ImageDraw.Draw(sheet).text((10, 6), title, fill=(0, 0, 0), font=font(18))
    for r, row in enumerate(rows):
        for c, im in enumerate(row):
            sheet.paste(im, (c * w, top + r * h))
    # Clay is close to one ramp of colour: 256 picked colours keep it and
    # make the file a third of the size.
    sheet = sheet.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    sheet.save(path, optimize=True)
    print(f"{path}: {sheet.size[0]}x{sheet.size[1]}, {os.path.getsize(path) / 1e6:.2f} MB")


def comparison(sh, M, R, G, shapes, frame, out):
    f = 0.0 if frame == "masculine" else 1.0
    engine = mix.engine_shapes(shapes)
    rows = []
    for view, azimuth in VIEWS:
        for who in ("MakeHuman", "ours"):
            row = []
            for label, muscle, weight in SETTINGS:
                if who == "MakeHuman":
                    V, F = mix.makehuman_body(M, f, muscle, weight), M["faces"]
                else:
                    V, F = mix.body(G, shapes, f, muscle, weight, engine=engine), R["faces"]
                row.append(sh.tile(V, F, azimuth, f"{who}, {view}: {label}"))
            rows.append(row)
    paste(rows, os.path.join(out, f"comparison_{frame}.png"),
          f"{frame}: MakeHuman's own mesh (rows 1, 3) against the realistic body with the "
          f"transferred shapes (rows 2, 4), same macro settings")


def details(sh, M, R, G, shapes, out):
    engine = mix.engine_shapes(shapes)
    picks = (("heavy + soft", -1, 1, 0), ("heavy + soft", -1, 1, 180), ("heavy + muscular", 1, 1, 0),
             ("heavy + muscular", 1, 1, 180), ("muscular", 1, 0, 0), ("slim", 0, -1, 0))
    rows = []
    for frame, f in (("masculine", 0.0), ("feminine", 1.0)):
        for who in ("MakeHuman", "ours"):
            row = []
            for label, muscle, weight, azimuth in picks:
                if who == "MakeHuman":
                    V, F = mix.makehuman_body(M, f, muscle, weight), M["faces"]
                else:
                    V, F = mix.body(G, shapes, f, muscle, weight, engine=engine), R["faces"]
                view = "front" if azimuth == 0 else "back"
                row.append(sh.tile(V, F, azimuth, f"{who}, {frame}: {label}, {view}",
                                   centre=(0, 0, 0.66), span=0.50, relative=True))
            rows.append(row)
    paste(rows, os.path.join(out, "details.png"),
          "The trunk close up: MakeHuman's own mesh (rows 1, 3) against the realistic body (rows 2, 4)")


def grid(sh, R, G, shapes, out):
    engine = mix.engine_shapes(shapes)
    steps = (-1.0, -0.5, 0.0, 0.5, 1.0)
    rows = []
    for muscle in reversed(steps):
        row = []
        for frame, f in (("masculine", 0.0), ("feminine", 1.0)):
            for weight in steps:
                V = mix.body(G, shapes, f, muscle, weight, engine=engine)
                row.append(sh.tile(V, R["faces"], 35, f"{frame[0].upper()} muscle {muscle:+.1f} weight {weight:+.1f}",
                                   span=1.86))
        rows.append(row)
    paste(rows, os.path.join(out, "grid.png"),
          "The realistic body over muscle (rows, max at the top) x weight (columns, min at the left): "
          "masculine left, feminine right; every tile at a half step is a mixture of corner shapes")


def bundle(sh, R, shapes, out):
    engine = mix.engine_shapes(shapes, "bundle")
    rows = []
    for frame, f in (("masculine", 0.0), ("feminine", 1.0)):
        row = []
        for label, muscle, weight in SETTINGS:
            V = mix.body(R, shapes, f, muscle, weight, "bundle", engine)
            row.append(sh.tile(V, R["faces"], 35, f"{frame}: {label}"))
        rows.append(row)
    paste(rows, os.path.join(out, "bundle_pose.png"),
          "The same shapes turned back to the bundle's pose (the pose the realistic body is vendored in)")


def extras(sh, M, R, G, shapes, out):
    picks = (("pregnant", "feminine"), ("buttocks_volume", "feminine"), ("breast_maxcup", "feminine"),
             ("breast_mincup", "feminine"), ("pectoral", "masculine"), ("stomach_tone", "masculine"))
    rows = [[], []]
    for name, frame in picks:
        gender = "male" if frame == "masculine" else "female"
        rows[0].append(sh.tile(M[f"neutral_{gender}"] + M[f"extra_{name}"], M["faces"], 35,
                               f"MakeHuman: {name} ({frame})"))
        rows[1].append(sh.tile(G[f"posed_{frame}"] + shapes[f"posed/{frame}/{name}"], R["faces"], 35,
                               f"ours: {name} ({frame})"))
    paste(rows, os.path.join(out, "extras.png"),
          "Regional targets carried the same way: MakeHuman's mesh (top) and the realistic body (bottom)")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--only", default="masculine,feminine,details,grid,bundle,extras")
    ap.add_argument("--samples", type=int, default=24)
    ap.add_argument("--tile", default=None, help="WxH of a tile")
    a = ap.parse_args()
    only = a.only.split(",")
    M = np.load(os.path.join(a.work, "makehuman.npz"))
    R = np.load(os.path.join(a.work, "realistic.npz"))
    G = np.load(os.path.join(a.work, "registration.npz"))
    shapes = np.load(os.path.join(a.work, "shapes.npz"))
    os.makedirs(a.out, exist_ok=True)
    tmp = os.path.join(a.work, "tiles")
    os.makedirs(tmp, exist_ok=True)
    size = tuple(int(x) for x in a.tile.split("x")) if a.tile else TILE
    for frame in ("masculine", "feminine"):
        if frame in only:
            comparison(Sheet(render.Studio(*size, a.samples), tmp), M, R, G, shapes, frame, a.out)
    if "details" in only:
        details(Sheet(render.Studio(400, 400, a.samples), tmp), M, R, G, shapes, a.out)
    if "grid" in only:
        grid(Sheet(render.Studio(236, 430, a.samples), tmp), R, G, shapes, a.out)
    if "bundle" in only:
        bundle(Sheet(render.Studio(300, 620, a.samples), tmp), R, shapes, a.out)
    if "extras" in only:
        extras(Sheet(render.Studio(340, 640, a.samples), tmp), M, R, G, shapes, a.out)


if __name__ == "__main__":
    main()
