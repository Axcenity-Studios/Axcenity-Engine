"""The hair atlas (Character Studio R6, CO11): strands drawn into texture
islands, baked to the Hair model's layout (`docs/rendering/hair.md`):

- colour (sRGB) and coverage (alpha): strands with tapered ends,
  clumped, with strays and flyaways; clear texels exactly 0;
- the data map (linear): R root to tip, G a value per strand, B the
  streak mask (highlights), A the strands' ambient occlusion;
- the flow map (linear): RG each strand's direction, root to tip, in
  the island (`0.5 + 0.5 * d`, (0.5, 1.0) straight down the texture: +V);
- a normal map: the strands' relief across them, which also gives the
  engine its tangents (it makes them only where there is a normal map).

Strands are drawn as polylines at four times the size, each segment
painted with its own number; every channel is then looked up from the
segment and strand that won each sample and averaged over the samples
covered. Islands are padded and their clear texels kept at alpha 0 (the
MSAA note in hair.md); colour is spread into the clear texels so mip
levels don't darken the edges.
"""

import math
import zlib

import numpy as np
from PIL import Image, ImageDraw

SIZE = 2048
PAD = 8
SS = 4
# The atlas's natural colour: a medium brown, about sRGB (72, 51, 33) (eumelanin, pheomelanin as in
# `lashes.hair_colour`), each strand a little lighter or darker; the Hair
# model's values move it from here (`styles.COLOURS`).
BASE_EUMELANIN = 0.55
BASE_PHEOMELANIN = 0.12
MELANIN_SPREAD = 0.3
# Coverage over a texel is multiplied by this (strands overlap; a dense
# island must read as whole).
GAIN = {"cap": 3.0, "band": 1.2, "coverage": 1.9, "clump": 1.5, "detail": 1.45,
        "short": 1.6, "hairline": 1.35, "sparse": 1.3, "flyaway": 1.4}
# How much of the hair is streaked (the streak mask), by strand clusters.
STREAK_SHARE = 0.16

EUMELANIN_ABSORBS = np.array((0.419, 0.697, 1.37))
PHEOMELANIN_ABSORBS = np.array((0.187, 0.4, 1.05))
CHIANG_K = 5.888
FIBRE_SHEEN = 0.012


def hair_colour(eumelanin, pheomelanin):
    """A fibre's colour (linear) from its melanins (Chiang 2016's mapping,
    as `lashes.hair_colour`)."""
    sigma = np.outer(eumelanin, EUMELANIN_ABSORBS) + np.outer(pheomelanin, PHEOMELANIN_ABSORBS)
    return np.exp(-CHIANG_K * np.sqrt(sigma)) + FIBRE_SHEEN


class Island:
    """A rectangle of the atlas (pixels) that strands are drawn in. A
    periodic island repeats across every `period` pixels and is drawn a
    little wider than one period, so a face that crosses the seam reads
    on."""

    def __init__(self, kind, x, y, w, h, period=None):
        self.kind, self.x, self.y, self.w, self.h, self.period = kind, x, y, w, h, period

    def uv(self, u, v):
        """Atlas texture coordinates of the island's (u, v): u across (in
        periods for a periodic island), v down, 0 to 1."""
        span = self.period if self.period else self.w
        return ((self.x + np.asarray(u) * span) / SIZE, (self.y + np.asarray(v) * self.h) / SIZE)


def layout():
    """The islands, by kind."""
    out = {}
    y = PAD
    out["cap"] = [Island("cap", PAD, y, SIZE - 2 * PAD, 330, period=1900)]
    y += 330 + PAD
    out["band"] = [Island("band", PAD, y, SIZE - 2 * PAD, 130, period=1900)]
    y += 130 + PAD

    def row(kinds, height):
        nonlocal y
        x = PAD
        for kind, count, width in kinds:
            for _ in range(count):
                out.setdefault(kind, []).append(Island(kind, x, y, width, height))
                x += width + PAD
        assert x <= SIZE, kinds
        y += height + PAD

    row([("coverage", 4, 240), ("clump", 5, 200)], 600)
    row([("detail", 9, 140), ("flyaway", 10, 50)], 600)
    row([("sparse", 3, 180), ("hairline", 3, 180), ("short", 5, 170)], 320)
    assert y <= SIZE
    return out


# -- strands in an island (island coordinates: x across in pixels of a
# period or the island's width, y down in pixels; root first) ------------


def _polyline(rng, x0, y0, length, lean, wave, waves, curl=0.0, steps=24):
    phase = rng.uniform(0, 2 * math.pi)
    t = np.linspace(0, 1, steps + 1)
    x = x0 + lean * t * t + wave * np.sin(2 * math.pi * waves * t + phase) * np.minimum(1, t * 3)
    x = x + curl * t ** 3
    y = y0 + length * t
    return np.stack([x, y], axis=1)


def clump_strands(rng, w, h, count, clumps, pull, length=(0.6, 1.0), width=1.1, wave=2.0,
                  root_band=0.08, strays=0.04, taper=0.35):
    """Strands in clumps: roots across the island, each strand drawn toward
    its clump's line by `pull` along its length, tapering over the last
    `taper` of it; a few strays wander off."""
    centres = rng.uniform(0.15, 0.85, clumps) * w
    out = []
    for _ in range(count):
        x0 = rng.uniform(0.02, 0.98) * w
        c = centres[np.argmin(np.abs(centres - x0))]
        y0 = rng.uniform(0.0, root_band) * h
        L = rng.uniform(*length) * (h - y0) * 0.98
        drift = (c - x0) * pull * rng.uniform(0.6, 1.1)
        stray = rng.random() < strays
        lean = drift + (rng.normal(0, 0.25) * w if stray else rng.normal(0, 0.03) * w)
        pts = _polyline(rng, x0, y0, L, lean, wave * rng.uniform(0.5, 1.5), rng.uniform(0.6, 1.6),
                        curl=rng.normal(0, 4.0))
        out.append({"points": pts, "width": width * rng.uniform(0.8, 1.2), "taper": taper})
    return out


def periodic_strands(rng, period, h, count, length, width, density=None, direction_noise=0.04):
    """Short strands over a periodic island, all running down it (+V);
    `density(v)` (0 to 1) thins them toward the island's bottom."""
    out = []
    while len(out) < count:
        x0 = rng.uniform(0, period)
        y0 = rng.uniform(-0.15, 1.0) * h
        if density is not None and rng.random() > density(max(y0, 0.0) / h):
            continue
        L = rng.uniform(*length) * h
        lean = rng.normal(0, direction_noise) * L
        pts = _polyline(rng, x0, y0, L, lean, rng.uniform(0.3, 1.5), rng.uniform(0.5, 1.5), steps=8)
        out.append({"points": pts, "width": width * rng.uniform(0.75, 1.25), "taper": 0.4})
    return out


def strands_for(island, rng):
    w, h = (island.period or island.w), island.h
    k = island.kind
    if k == "cap":
        return periodic_strands(rng, w, h, 40000, (0.05, 0.16), 1.3)
    if k == "band":
        # Dense where the band meets the cap (top), thinning to single fine
        # hairs at the hairline (bottom).
        dens = lambda v: 1.0 - 0.97 * min(1.0, max(0.0, (v - 0.05) / 0.9))
        return periodic_strands(rng, w, h, 7000, (0.08, 0.25), 0.9, density=dens)
    if k == "coverage":
        return clump_strands(rng, w, h, 300, 4, 0.25, length=(0.75, 1.0), width=1.25, taper=0.25)
    if k == "clump":
        return clump_strands(rng, w, h, 150, 2, 0.55, length=(0.65, 1.0), width=1.15)
    if k == "detail":
        return clump_strands(rng, w, h, 70, 1, 0.75, length=(0.6, 1.0), width=1.05, strays=0.08, taper=0.45)
    if k == "short":
        return clump_strands(rng, w, h, 170, 3, 0.4, length=(0.55, 1.0), width=1.1, root_band=0.12)
    if k == "hairline":
        return clump_strands(rng, w, h, 70, 3, 0.25, length=(0.4, 1.0), width=0.85, root_band=0.25,
                             taper=0.5)
    if k == "sparse":
        return clump_strands(rng, w, h, 26, 3, 0.15, length=(0.35, 1.0), width=0.75, root_band=0.35,
                             taper=0.55)
    if k == "flyaway":
        n = int(rng.integers(1, 4))
        return clump_strands(rng, w, h, n, 1, 0.0, length=(0.8, 1.0), width=0.9, wave=5.0,
                             strays=0.5, taper=0.5)
    raise ValueError(k)


def draw(island, strands, rng):
    """The island's channels at its own size: colour (linear RGB), alpha,
    root to tip, per strand, streak, occlusion and flow, from `strands`
    drawn first (deepest) to last."""
    w, h = island.w, island.h
    period = island.period
    canvas = Image.new("I", (w * SS, h * SS), 0)
    pen = ImageDraw.Draw(canvas)
    seg_strand, seg_t, seg_dir = [0], [0.0], [(0.0, 1.0)]
    n = len(strands)
    # Strand attributes: melanin, a value of its own, streak (by clusters
    # across the island, so highlights come in pieces), depth.
    streak_centres = rng.uniform(0, period or w, max(1, int(6 * STREAK_SHARE * (period or w) / 100)))
    attrs = []
    for index, s in enumerate(strands):
        x_root = s["points"][0, 0]
        near = np.min(np.abs(((x_root - streak_centres + (period or 1e9) / 2) % (period or 1e9))
                             - (period or 1e9) / 2)) if period else np.min(np.abs(x_root - streak_centres))
        streak = 1.0 if (near < 9 and rng.random() < 0.8) or rng.random() < 0.02 else 0.0
        eu = BASE_EUMELANIN * math.exp(rng.normal(0, MELANIN_SPREAD))
        ph = BASE_PHEOMELANIN * math.exp(rng.normal(0, 0.15))
        attrs.append((eu, ph, rng.random(), streak, (index + 1) / n))
        pts = s["points"]
        m = len(pts) - 1
        for k in range(m):
            t0 = k / m
            t1 = (k + 1) / m
            tip = max(0.0, (t1 - (1 - s["taper"])) / s["taper"])
            width = max(1, int(round(s["width"] * SS * (1.0 - 0.8 * tip))))
            a, b = pts[k] * SS, pts[k + 1] * SS
            d = b - a
            ln = max(np.linalg.norm(d), 1e-9)
            seg_strand.append(index)
            seg_t.append(0.5 * (t0 + t1))
            seg_dir.append((d[0] / ln, d[1] / ln))
            fill = len(seg_strand) - 1
            shifts = [0.0]
            if period:
                shifts = [-period * SS, 0.0, period * SS]
            for dx in shifts:
                pen.line([(a[0] + dx, a[1]), (b[0] + dx, b[1])], fill=fill, width=width)
    ids = np.asarray(canvas, dtype=np.int64)
    covered = ids > 0
    seg_strand = np.array(seg_strand)
    seg_t = np.array(seg_t)
    seg_dir = np.array(seg_dir)
    attrs = np.array(attrs)
    strand = seg_strand[ids]
    colour = hair_colour(attrs[:, 0], attrs[:, 1])[strand]
    t = seg_t[ids]
    own = attrs[strand, 2]
    streak = attrs[strand, 3]
    depth = attrs[strand, 4]
    ao = (0.3 + 0.55 * depth) * (0.55 + 0.45 * np.clip(t / 0.4, 0, 1))
    flow = seg_dir[ids]

    def down(x, weights=covered):
        """The mean over each texel's covered samples."""
        wgt = weights.astype(np.float64)
        if x.ndim == 3:
            num = (x * wgt[..., None]).reshape(h, SS, w, SS, -1).sum((1, 3))
            den = wgt.reshape(h, SS, w, SS).sum((1, 3))[..., None]
        else:
            num = (x * wgt).reshape(h, SS, w, SS).sum((1, 3))
            den = wgt.reshape(h, SS, w, SS).sum((1, 3))
        return num / np.maximum(den, 1e-9)

    coverage = covered.reshape(h, SS, w, SS).mean((1, 3))
    alpha = np.clip(coverage * GAIN[island.kind], 0.0, 1.0)
    out = {
        "alpha": alpha,
        "colour": down(colour),
        "root_to_tip": down(t),
        "own": down(own),
        "streak": down(streak),
        "ao": down(ao),
        "flow": down(flow),
    }
    # Relief across the strands for the normal map: coverage as height.
    height = coverage
    gx = np.zeros_like(height)
    gy = np.zeros_like(height)
    gx[:, 1:-1] = (height[:, 2:] - height[:, :-2]) * 0.5
    gy[1:-1, :] = (height[2:, :] - height[:-2, :]) * 0.5
    out["relief"] = (gx, gy)
    return out


def fill_clear(rgb, alpha):
    """Colour spread into the clear texels (each takes the mean of the
    covered ones near it, then the island's mean), so filtering and mips
    don't pull dark or black in at the strands' edges."""
    covered = alpha > 0
    if not covered.any():
        return rgb
    out = rgb.copy()
    mean = rgb[covered].mean(0)
    known = covered.copy()
    for _ in range(6):
        acc = np.zeros_like(out)
        cnt = np.zeros(out.shape[:2])
        for dy, dx in ((0, 1), (0, -1), (1, 0), (-1, 0)):
            shifted = np.roll(out * known[..., None], (dy, dx), axis=(0, 1))
            k = np.roll(known, (dy, dx), axis=(0, 1))
            acc += shifted
            cnt += k
        grow = (~known) & (cnt > 0)
        out[grow] = acc[grow] / cnt[grow][:, None]
        known |= grow
    out[~known] = mean
    return out


def bake(seed, out_dir, name):
    """Draws every island and writes the atlas's four maps as PNGs in
    `out_dir` (`<name>_Colour.png`, `_Data.png`, `_Flow.png`,
    `_Normal.png`). Returns the layout and the files."""
    islands = layout()
    colour = np.zeros((SIZE, SIZE, 3))
    alpha = np.zeros((SIZE, SIZE))
    data = np.zeros((SIZE, SIZE, 4))
    data[..., 3] = 1.0
    flow = np.zeros((SIZE, SIZE, 2))
    flow[..., 1] = 1.0
    normal = np.zeros((SIZE, SIZE, 3))
    normal[..., 2] = 1.0
    for kind, group in islands.items():
        for index, island in enumerate(group):
            rng = np.random.default_rng([seed, zlib.crc32(kind.encode()), index])
            ch = draw(island, strands_for(island, rng), rng)
            ys = slice(island.y, island.y + island.h)
            xs = slice(island.x, island.x + island.w)
            colour[ys, xs] = fill_clear(ch["colour"], ch["alpha"])
            alpha[ys, xs] = ch["alpha"]
            data[ys, xs, 0] = ch["root_to_tip"]
            data[ys, xs, 1] = ch["own"]
            data[ys, xs, 2] = ch["streak"]
            data[ys, xs, 3] = np.where(ch["alpha"] > 0, ch["ao"], 1.0)
            f = ch["flow"]
            ln = np.linalg.norm(f, axis=-1, keepdims=True)
            f = np.where(ln > 1e-6, f / np.maximum(ln, 1e-9), np.array([0.0, 1.0]))
            flow[ys, xs] = f
            gx, gy = ch["relief"]
            nrm = np.stack([-gx * 1.6, gy * 1.6, np.ones_like(gx)], axis=-1)
            normal[ys, xs] = nrm / np.linalg.norm(nrm, axis=-1, keepdims=True)
    # Outside every island: the atlas's mean hair colour, alpha 0.
    srgb = np.where(colour <= 0.0031308, colour * 12.92, 1.055 * np.power(np.maximum(colour, 0), 1 / 2.4) - 0.055)
    rgba = np.zeros((SIZE, SIZE, 4), dtype=np.uint8)
    rgba[..., :3] = np.clip(srgb * 255 + 0.5, 0, 255).astype(np.uint8)
    rgba[..., 3] = np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)
    outside = rgba[..., 3] == 0
    covered_mean = rgba[~outside][:, :3].mean(0).astype(np.uint8)
    unpainted = (colour.sum(-1) == 0)
    rgba[unpainted, :3] = covered_mean
    files = {}
    import os
    files["colour"] = os.path.join(out_dir, name + "_Colour.png")
    Image.fromarray(rgba, "RGBA").save(files["colour"], optimize=True)
    d8 = np.clip(data * 255 + 0.5, 0, 255).astype(np.uint8)
    files["data"] = os.path.join(out_dir, name + "_Data.png")
    Image.fromarray(d8, "RGBA").save(files["data"], optimize=True)
    fl = np.zeros((SIZE, SIZE, 3), dtype=np.uint8)
    fl[..., :2] = np.clip((0.5 + 0.5 * flow) * 255 + 0.5, 0, 255).astype(np.uint8)
    files["flow"] = os.path.join(out_dir, name + "_Flow.png")
    Image.fromarray(fl, "RGB").save(files["flow"], optimize=True)
    nm = np.clip((0.5 + 0.5 * normal) * 255 + 0.5, 0, 255).astype(np.uint8)
    files["normal"] = os.path.join(out_dir, name + "_Normal.png")
    Image.fromarray(nm, "RGB").save(files["normal"], optimize=True)
    return islands, files
