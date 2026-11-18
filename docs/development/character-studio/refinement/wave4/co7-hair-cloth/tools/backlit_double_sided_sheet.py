"""Review fixes 2 evidence sheet: double-sided hair over a head, back-lit."""
import os
import sys

from PIL import Image, ImageDraw

R = sys.argv[1]
OUT = sys.argv[2]
BLENDER = sys.argv[3]


def labelled(path, label, box=None, scale=1):
    im = Image.open(path).convert("RGB")
    if box:
        im = im.crop(box)
    im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    tile = Image.new("RGB", (im.width, im.height + 22), (24, 24, 24))
    tile.paste(im, (0, 22))
    ImageDraw.Draw(tile).text((6, 5), label, fill=(255, 255, 255))
    return tile


def row(tiles, gap=4):
    w = sum(t.width for t in tiles) + gap * (len(tiles) - 1)
    h = max(t.height for t in tiles)
    sheet = Image.new("RGB", (w, h), (70, 70, 70))
    x = 0
    for t in tiles:
        sheet.paste(t, (x, 0))
        x += t.width + gap
    return sheet


box = (176, 85, 336, 175)
e = lambda name: f"{R}/engine-hair-blonde-backlit-double-sided-{name}.ppm"
rows = [
    row([labelled(e("loose-msaa"), "double-sided, loose: MSAA 4x", box, 3),
         labelled(e("loose-taa"), "double-sided, loose: TAA, 40 frames", box, 3)]),
    row([labelled(e("shell-msaa"), "double-sided, marked shell: MSAA 4x", box, 3),
         labelled(e("shell-taa"), "double-sided, marked shell: TAA, 40 frames", box, 3)]),
    row([labelled(f"{BLENDER}/blender-hair-blonde-backlit.png", "Blender Cycles (Principled Hair, same camera)", box, 3),
         labelled(f"{R}/engine-hair-blonde-backlit.ppm", "single-sided shell: MSAA 4x", box, 3)]),
]
w = max(r.width for r in rows)
sheet = Image.new("RGB", (w, sum(r.height for r in rows) + 4 * (len(rows) - 1)), (70, 70, 70))
y = 0
for r in rows:
    sheet.paste(r, (0, y))
    y += r.height + 4
sheet.save(os.path.join(OUT, "backlit-double-sided-hair-over-head-x3.png"))
print(sheet.size)
