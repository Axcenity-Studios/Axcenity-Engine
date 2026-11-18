"""Side-by-side sheets and measurements: engine vs Blender."""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

EV = sys.argv[1]
OUT = sys.argv[2] if len(sys.argv) > 2 else EV
pairs = ["hair-brown", "hair-blonde", "hair-blonde-backlit", "cloth-standard", "cloth-sheen", "cloth-satin"]


def load(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)


def lin(a):
    a = a / 255.0
    return np.where(a <= 0.04045, a / 12.92, ((a + 0.055) / 1.055) ** 2.4)


def ball_mask(size=512, r=160):
    y, x = np.mgrid[0:size, 0:size]
    return ((x - 255.5) ** 2 + (y - 255.5) ** 2) < (r * 0.97) ** 2


report = []
tiles = []
mask = ball_mask()
for name in pairs:
    e = load(os.path.join(EV, f"engine-{name}.ppm"))
    b = load(os.path.join(EV, f"blender-{name}.png"))
    le, lb = lin(e), lin(b)
    # Exposure: scale Blender so the ball's mean linear luminance matches.
    lum = lambda a: (a * [0.2126, 0.7152, 0.0722]).sum(-1)
    m = mask if "backlit" not in name else np.ones_like(mask)
    se, sb = lum(le)[m].mean(), lum(lb)[m].mean()
    scale = se / max(sb, 1e-9)
    lb2 = np.clip(lb * scale, 0, 1)
    b2 = np.where(lb2 <= 0.0031308, lb2 * 12.92, 1.055 * lb2 ** (1 / 2.4) - 0.055) * 255
    # Middle column brightness profile: the brightest row.
    col_e = lum(le)[96:416, 254:258].mean(1)
    col_b = lum(lb2)[96:416, 254:258].mean(1)
    peak_e, peak_b = int(np.argmax(col_e)) + 96, int(np.argmax(col_b)) + 96
    # Chroma of the ball, linear, normalised.
    ce = le[m].mean(0); cb = lb2[m].mean(0)
    ce, cb = ce / ce.sum(), cb / cb.sum()
    # Row profile (rim vs middle) along the middle row.
    row_e = lum(le)[254:258, :].mean(0)
    row_b = lum(lb2)[254:258, :].mean(0)
    rim = lambda r: r[256 + 150] / max(r[256], 1e-6)
    # Highlight spread (satin): bright pixels' spread across and down.
    def spread(a):
        l = lum(a)
        pts = np.argwhere(l > 0.5 * l.max())
        return pts[:, 1].std(), pts[:, 0].std()
    sx_e, sy_e = spread(le)
    sx_b, sy_b = spread(lb2)
    report.append(
        f"{name}: exposure x{scale:.2f}; brightest row in the middle column engine {peak_e}, Blender {peak_b}; "
        f"chroma engine {np.round(ce, 3).tolist()} Blender {np.round(cb, 3).tolist()}; "
        f"rim/middle (middle row, 94% radius) engine {rim(row_e):.2f} Blender {rim(row_b):.2f}; "
        f"highlight spread across/down engine {sx_e:.0f}/{sy_e:.0f} Blender {sx_b:.0f}/{sy_b:.0f}"
    )
    tile = Image.new("RGB", (1024, 540), (20, 20, 20))
    tile.paste(Image.fromarray(e.astype(np.uint8)), (0, 28))
    tile.paste(Image.fromarray(np.clip(b2, 0, 255).astype(np.uint8)), (512, 28))
    d = ImageDraw.Draw(tile)
    d.text((8, 6), f"{name}: engine (left), Blender Cycles (right, exposure x{scale:.2f})", fill=(255, 255, 255))
    tiles.append(tile)

sheet = Image.new("RGB", (2048, 540 * 3), (0, 0, 0))
for i, t in enumerate(tiles):
    sheet.paste(t, ((i % 2) * 1024, (i // 2) * 540))
sheet.save(os.path.join(OUT, "engine-vs-blender.jpg"), quality=88)
print("\n".join(report))
