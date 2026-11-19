"""The low-top sneakers' maps (2048 px): base colour, a tangent-space
normal map (glTF's: +X along U, +Y up the image) and occlusion-roughness-
metal, baked from the full-detail geometry (`build_sneakers_low.py`) and
CC0 material photographs. Nothing is taken from a reference photograph:
the references were looked at for proportions and construction only.

Each texel is found on its triangle (`textures.raster`) and so has a place
on the shoe; what it carries is worked out there:

- **Upper** (leather, Poly Haven's Leather White, CC0, tiled at its
  photographed 300 mm): the panels a court shoe is sewn from, as overlays
  on the one shell: the toe cap, the eyestay facings and the heel counter
  each stand 0.9 mm proud with a rounded edge, stitched with two rows
  (1.6 and 3.8 mm in, 3.2 mm stitches); a topline stitch 4 mm under the
  collar; the punched eyelet holes dark under the rings.
- **Lining, tongue, footbed** (textile: Poly Haven's Cotton Jersey, CC0):
  the knit; the tongue's bound edge stitched 3 mm in.
- **Laces** (the jersey, finer): a woven rib along the lace.
- **Eyelets**: painted metal (metalness 1, roughness 0.35).
- **Midsole** (the cupsole's wall, rubber, ambientCG's Rubber004, CC0, at
  low amplitude): the sole's stitch row 5 mm under the lip and a fine
  vertical texture.
- **Outsole** (the tread): our own pattern in the footprint's frame: a
  5 mm plain rim, a field of diamond lugs (7 mm pitch, 1.3 mm grooves)
  with two flex grooves across the ball, and fine bars under the arch;
  grooves 1.2 mm deep, dark in occlusion.

Base colour is near white (the variants' tints give the colour), with the
material's own light and shade from the photographed tile, no lighting.
"""

import math

import numpy as np
from PIL import Image

import textures

SIZE = textures.SIZE


def tile(path, mm, scale, mode="L"):
    return textures.load_tile(path, mm, scale, mode)


def fill(img, valid):
    """Every texel outside the islands takes its nearest island's colour
    (spread out ring by ring until the atlas is full), so no filtered or
    mip-mapped read near an island's edge picks up the empty background
    (which drew dark lines along the texture seams)."""
    img = textures.dilate(img, valid, rounds=16)
    grown = valid.copy()
    for _ in range(16):
        grown = grown | np.roll(grown, 1, 0) | np.roll(grown, -1, 0) | np.roll(grown, 1, 1) | np.roll(grown, -1, 1)
    # Further out, coarser: a mean of each 16 x 16 block's filled texels,
    # spread the same way, then everything left the islands' mean.
    small = img.reshape(SIZE // 16, 16, SIZE // 16, 16, -1)
    mask = grown.reshape(SIZE // 16, 16, SIZE // 16, 16)
    total = (small * mask[..., None]).sum(axis=(1, 3))
    count = mask.sum(axis=(1, 3))
    have = count > 0
    block = np.where(have[..., None], total / np.maximum(count, 1)[..., None], 0)
    block = textures.dilate(block, have, rounds=SIZE // 16)
    mean = img[valid].mean(axis=0)
    block[~have & ~np.isfinite(block).all(axis=-1)] = mean
    up = np.repeat(np.repeat(block, 16, axis=0), 16, axis=1)
    return np.where(grown[..., None], img, up) if img.ndim == 3 else np.where(grown, img, up[..., 0])


def rim_distance(points, outline):
    """Each 2D point's distance (mm) from a closed outline (mm)."""
    loop = np.vstack([outline, outline[:1]])
    d, _ = textures.polyline_distance(np.column_stack([points, np.zeros(len(points))]),
                                      [np.column_stack([loop, np.zeros(len(loop))])])
    return d


def bake(tris, uv, pos, nor, mat, side_of, foot_frame, features, tiles, scale, out_prefix):
    """`tris` (t, 3) corner ids, `uv` (n, 2) 0..1 (v down), `pos` and `nor`
    (n, 3) metres, `mat` (t,) material names, `side_of` (n,) +1 left -1
    right, `foot_frame(p)` -> (s, w across the middle line, h) of points
    on the left foot (a right shoe's points are mirrored first),
    `features`: `sneakers_low.features` (polylines in metres on the left
    shoe, each overlay's with the side it covers) plus `foot_length_mm`,
    `tiles`: the CC0 tiles loaded (`leather`, `jersey`, `rubber`), `scale`
    texels per mm."""
    ids, w = textures.raster(uv, tris)
    valid = ids >= 0
    vi = np.nonzero(valid)[0]
    t = tris[ids[vi]]
    ww = w[vi]
    P = (pos[t] * ww[:, :, None]).sum(axis=1)
    N = (nor[t] * ww[:, :, None]).sum(axis=1)
    N /= np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)
    M = mat[ids[vi]]
    side = side_of[t[:, 0]]
    left = P.copy()
    left[:, 0] *= side          # every texel as on the left shoe
    s, wl, h = foot_frame(left)
    # Tangent frames from each triangle's texture map.
    e1, e2 = pos[t[:, 1]] - pos[t[:, 0]], pos[t[:, 2]] - pos[t[:, 0]]
    d1, d2 = uv[t[:, 1]] - uv[t[:, 0]], uv[t[:, 2]] - uv[t[:, 0]]
    det = d1[:, 0] * d2[:, 1] - d2[:, 0] * d1[:, 1]
    det = np.where(np.abs(det) < 1e-12, 1e-12, det)
    T = (e1 * d2[:, 1:2] - e2 * d1[:, 1:2]) / det[:, None]
    Bv = (e2 * d1[:, 0:1] - e1 * d2[:, 0:1]) / det[:, None]
    T = T - N * (T * N).sum(axis=1, keepdims=True)
    T /= np.maximum(np.linalg.norm(T, axis=1, keepdims=True), 1e-12)
    Bup = -Bv
    Bup = Bup - N * (Bup * N).sum(axis=1, keepdims=True) - T * (Bup * T).sum(axis=1, keepdims=True)
    Bup /= np.maximum(np.linalg.norm(Bup, axis=1, keepdims=True), 1e-12)

    n = len(vi)
    height = np.zeros(n)
    ao = np.ones(n)
    rough = np.full(n, 0.6)
    metal = np.zeros(n)
    colour = np.ones((n, 3))
    # Texel places in mm along the image (for the photographed tiles: the
    # atlas is one scale, so texels are a true size).
    px = (vi % SIZE).astype(float) / scale
    py = (vi // SIZE).astype(float) / scale
    mm = left * 1000.0

    def stitch(sel, d, along, offsets, length=3.2, gap=0.9, width=0.42, depth=0.22):
        for off in offsets:
            across = np.abs(d - off)
            phase = np.mod(along, length)
            on = (across < width) & (phase < length - gap)
            prof = np.where(on, np.cos(np.clip(across / width, 0, 1) * math.pi / 2), 0.0)
            height[sel] += depth * prof
            # The needle holes and the thread's shadow.
            hole = (across < width + 0.15) & (phase >= length - gap)
            ao[sel] *= np.where(hole, 0.72, 1.0) * (1 - 0.12 * prof)
            rough[sel] = np.where(prof > 0.3, 0.55, rough[sel])

    # Materials.
    is_ = {name: M == name for name in np.unique(M)}

    def where(*names):
        out = np.zeros(n, dtype=bool)
        for name in names:
            if name in is_:
                out |= is_[name]
        return out

    leather = where("Upper")
    textile = where("Lining", "Tongue", "Insole")
    laces = where("Laces")
    eyelets = where("Eyelets")
    wall = where("Midsole", "SoleInner")
    tread = where("Outsole")

    # Leather.
    lt = tiles["leather"]
    if leather.any():
        g = textures.sample_tile(lt["height"], px[leather], py[leather], lt["mm"])
        height[leather] += 0.06 * (g - 0.5)
        lum = textures.sample_tile(lt["light"], px[leather], py[leather], lt["mm"])
        colour[leather] *= (0.86 + 0.10 * (lum - lt["light_mean"]) / lt["light_mean"])[:, None]
        r = textures.sample_tile(lt["rough"], px[leather], py[leather], lt["mm"])
        rough[leather] = 0.40 + 0.16 * r
        # Overlays: each its edge as a polyline on the shell, and a test of
        # which side it covers (in the foot's frame).
        for name in ("toe_cap", "counter_l", "counter_m", "eyestay_l", "eyestay_m"):
            line = features.get(name)
            if line is None:
                continue
            d, along = textures.polyline_distance(mm[leather], [np.asarray(line) * 1000.0])
            # Which side: the texel against the line's cover direction at its
            # nearest point.
            lpts, ldirs = features["covers"][name]
            lmm = np.asarray(lpts) * 1000.0
            acc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(lmm, axis=0), axis=1))])
            q = np.stack([np.interp(along, acc, lmm[:, k]) for k in range(3)], axis=1)
            c = np.stack([np.interp(along, acc, ldirs[:, k]) for k in range(3)], axis=1)
            covered = ((mm[leather] - q) * c).sum(axis=1) > 0
            # 0.9 mm proud on the covered side, its edge rounded over about
            # 0.9 mm, easing back to the shell's own level 4 to 9 mm in (the
            # panel's thickness is drawn at its edge only: far from the line
            # the side it covers isn't known well, and a step there would
            # draw a crease where no seam is); a shadow line outside it.
            signed = np.where(covered, d, -d)
            ease = 1 - np.clip((d - 4.0) / 5.0, 0, 1)
            ease = ease * ease * (3 - 2 * ease)
            height[leather] += 0.9 * np.clip((signed + 0.3) / 0.9, 0, 1) ** 0.7 * ease
            ao[leather] *= np.where(~covered & (d < 1.2), 0.62 + 0.38 * d / 1.2, 1.0)
            stitch(leather, np.where(covered, d, 99.0), along, (1.6, 3.8))
        # The topline stitch under the collar.
        d, along = textures.polyline_distance(mm[leather], [np.asarray(features["topline"]) * 1000.0])
        stitch(leather, d, along, (4.0,))
        # Eyelet holes: dark and recessed under each ring.
        for c in features["eyelets"]:
            dist = np.linalg.norm(mm[leather] - np.asarray(c) * 1000.0, axis=1)
            height[leather] -= np.where(dist < 1.7, 0.8, 0.0)
            ao[leather] *= np.where(dist < 1.7, 0.15, np.where(dist < 3.4, 0.82, 1.0))

    # Textiles.
    jt = tiles["jersey"]
    if textile.any() or laces.any():
        sel = textile | laces
        k = np.where(laces[sel], 2.5, 1.0)
        g = textures.sample_tile(jt["height"], px[sel] * k, py[sel] * k, jt["mm"])
        height[sel] += np.where(laces[sel], 0.10, 0.16) * (g - 0.5)
        lum = textures.sample_tile(jt["light"], px[sel] * k, py[sel] * k, jt["mm"])
        colour[sel] *= (0.84 + 0.12 * (lum - jt["light_mean"]) / jt["light_mean"])[:, None]
        rough[sel] = 0.78 + 0.12 * textures.sample_tile(jt["rough"], px[sel] * k, py[sel] * k, jt["mm"])
    if laces.any():
        # A woven rib: fine diagonal ridges, 1.1 mm apart.
        ph = 2 * math.pi * (px[laces] + py[laces]) / 1.1
        height[laces] += 0.12 * np.sin(ph)
        ao[laces] *= 1 - 0.08 * (1 - np.sin(ph)) / 2
    tongue = where("Tongue")
    if tongue.any() and "tongue_edge" in features:
        d, along = textures.polyline_distance(mm[tongue], [np.asarray(features["tongue_edge"]) * 1000.0])
        stitch(tongue, d, along, (3.0,), length=2.8)
    # Eyelets: painted metal.
    colour[eyelets] = 0.62
    rough[eyelets] = 0.35
    metal[eyelets] = 1.0

    # The cupsole's wall.
    rt = tiles["rubber"]
    if wall.any():
        g = textures.sample_tile(rt["height"], px[wall], py[wall], rt["mm"])
        height[wall] += 0.05 * (g - 0.5)
        colour[wall] *= 0.92
        rough[wall] = 0.62
        if "sole_stitch" in features:
            d, along = textures.polyline_distance(mm[wall], [np.asarray(features["sole_stitch"]) * 1000.0])
            stitch(wall, d, along, (0.0,), length=3.6, gap=1.0, width=0.5, depth=0.25)
            # The thread a little darker than the rubber.
            colour[wall] *= np.where(d < 0.5, 0.88, 1.0)[:, None]
    # The tread: our own pattern in the footprint's frame.
    if tread.any():
        ts, tw = s[tread], wl[tread]
        x_mm, y_mm = mm[tread][:, 0], mm[tread][:, 1]
        rim = rim_distance(left[tread][:, :2] * 1000.0, np.asarray(features["tread"]) * 1000.0)
        groove = np.zeros(tread.sum())
        # Diamond lugs over the forefoot and the heel.
        u = (x_mm + y_mm) / 7.0
        v = (x_mm - y_mm) / 7.0
        du = np.abs(u - np.round(u)) * 7.0
        dv = np.abs(v - np.round(v)) * 7.0
        diamonds = np.minimum(du, dv) < 0.65
        # Fine bars under the arch.
        length = features["foot_length_mm"]
        bars = np.abs(np.mod(ts * length, 3.0) - 1.5) < 0.5
        arch = (ts > 0.33) & (ts < 0.55)
        groove = np.where(arch, bars, diamonds)
        # Two flex grooves across the ball.
        for sc in (0.62, 0.71):
            groove |= np.abs((ts - sc) * length) < 1.0
        groove &= rim > 5.0
        # The rim's inner edge.
        groove |= np.abs(rim - 5.0) < 0.6
        height[tread] += np.where(groove, -1.2, 0.0)
        ao[tread] *= np.where(groove, 0.55, 1.0)
        g = textures.sample_tile(rt["height"], px[tread], py[tread], rt["mm"])
        height[tread] += 0.08 * (g - 0.5)
        colour[tread] *= 0.90
        rough[tread] = np.where(groove, 0.8, 0.68)

    # Height to the normal map: finite differences in the atlas, texels of
    # other islands ignored.
    H = np.zeros(SIZE * SIZE)
    H[vi] = height
    Hm = H.reshape(SIZE, SIZE)
    V = valid.reshape(SIZE, SIZE)
    px_mm = 1.0 / scale
    right, leftn = np.roll(Hm, -1, axis=1), np.roll(Hm, 1, axis=1)
    rv, lv = np.roll(V, -1, axis=1), np.roll(V, 1, axis=1)
    gx = np.where(rv & lv, (right - leftn) / (2 * px_mm),
                  np.where(rv, (right - Hm) / px_mm, np.where(lv, (Hm - leftn) / px_mm, 0)))
    down, up = np.roll(Hm, -1, axis=0), np.roll(Hm, 1, axis=0)
    dv_, uv_ = np.roll(V, -1, axis=0), np.roll(V, 1, axis=0)
    gy = np.where(dv_ & uv_, (down - up) / (2 * px_mm),
                  np.where(dv_, (down - Hm) / px_mm, np.where(uv_, (Hm - up) / px_mm, 0)))
    gx, gy = gx.reshape(-1)[vi], gy.reshape(-1)[vi]
    nt = np.stack([-gx, gy, np.ones(n)], axis=1)
    nt /= np.linalg.norm(nt, axis=1, keepdims=True)

    def image(values, channels):
        img = np.zeros((SIZE * SIZE, channels), dtype=np.float32)
        img[vi] = values
        img = img.reshape(SIZE, SIZE, channels)
        return fill(img, V)

    nimg = image(nt * 0.5 + 0.5, 3)
    cimg = image(np.clip(colour * (0.6 + 0.4 * ao[:, None]), 0, 1), 3)
    oimg = image(np.stack([ao, rough, metal], axis=1), 3)

    def save(img, path, srgb):
        if srgb:
            img = np.where(img <= 0.0031308, img * 12.92, 1.055 * np.power(np.clip(img, 0, 1), 1 / 2.4) - 0.055)
        Image.fromarray(np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)).save(path, quality=92, subsampling=0)

    save(cimg, out_prefix + "_color.jpg", True)
    save(nimg, out_prefix + "_normal.jpg", False)
    save(oimg, out_prefix + "_orm.jpg", False)
    return {"texels": int(valid.sum()), "texels_per_mm": float(scale)}
