"""Baking a garment's maps at one atlas (2048 px): base colour, a
tangent-space normal map (glTF's: +X along U, +Y up the image) and
occlusion-roughness-metal (glTF's channels), all in the garment's own
texture coordinates (the pattern, `finish.pack`).

What the normal map carries, over the level of detail's own surface:

- **Folds**: the fine cloth simulation's normal at each place of the
  pattern, against the coarser mesh's (the drape's small folds the mesh
  can't hold).
- **Seams**: a soft valley along every sewn seam (the two pieces turn in
  to it), and stitching: a hem's two rows of cover stitch, a neck band's
  or waistband's topstitch, each a dashed raised thread.
- **The fabric**: a CC0 photographed fabric's height (Poly Haven), tiled
  at its real size along the straight grain, filtered to the atlas's
  texel size; a rib band's wales across it.

Base colour is the fabric's own light and shade (its colour taken out:
the item's tint gives the colour), roughness the fabric's, occlusion the
seams' and stitches' and the folds' recesses.
"""

import math

import numpy as np
from PIL import Image

SIZE = 2048
# A fold's tilt from the level's normal kept whole up to this (degrees),
# and eased to none at the most.
FOLD_TILT = 25.0
FOLD_TILT_MAX = 50.0


def load_tile(path, mm, scale, mode="L"):
    """A fabric tile `mm` across, resampled to `scale` texels per mm (a
    float array 0..1)."""
    img = Image.open(path).convert(mode)
    px = max(8, int(round(mm * scale)))
    img = img.resize((px, px), Image.LANCZOS)
    return np.asarray(img, dtype=np.float32) / 255.0


def sample_tile(tile, x_mm, y_mm, mm):
    """`tile` (one repeat `mm` across) at pattern places (mm), wrapping."""
    h, w = tile.shape[:2]
    u = np.mod(x_mm / mm, 1.0) * w
    v = np.mod(y_mm / mm, 1.0) * h
    i0 = np.floor(u).astype(np.int64) % w
    j0 = np.floor(v).astype(np.int64) % h
    fu, fv = u - np.floor(u), v - np.floor(v)
    i1, j1 = (i0 + 1) % w, (j0 + 1) % h
    if tile.ndim == 3:
        fu, fv = fu[:, None], fv[:, None]
    return ((1 - fu) * (1 - fv) * tile[j0, i0] + fu * (1 - fv) * tile[j0, i1]
            + (1 - fu) * fv * tile[j1, i0] + fu * fv * tile[j1, i1])


def raster(uv, tris, size=SIZE):
    """For each texel whose middle is inside a triangle (in texture
    space, 0..1, v down): the triangle and its barycentric weights.
    Returns (ids (size*size,), weights (size*size, 3)), -1 outside."""
    ids = np.full(size * size, -1, dtype=np.int64)
    w = np.zeros((size * size, 3), dtype=np.float32)
    px = uv * size
    for t, (a, b, c) in enumerate(tris):
        pa, pb, pc = px[a], px[b], px[c]
        x0 = int(max(0, math.floor(min(pa[0], pb[0], pc[0]))))
        x1 = int(min(size, math.ceil(max(pa[0], pb[0], pc[0]))))
        y0 = int(max(0, math.floor(min(pa[1], pb[1], pc[1]))))
        y1 = int(min(size, math.ceil(max(pa[1], pb[1], pc[1]))))
        if x1 <= x0 or y1 <= y0:
            continue
        xs, ys = np.meshgrid(np.arange(x0, x1) + 0.5, np.arange(y0, y1) + 0.5)
        det = (pb[0] - pa[0]) * (pc[1] - pa[1]) - (pc[0] - pa[0]) * (pb[1] - pa[1])
        if abs(det) < 1e-12:
            continue
        l1 = ((xs - pa[0]) * (pc[1] - pa[1]) - (pc[0] - pa[0]) * (ys - pa[1])) / det
        l2 = ((pb[0] - pa[0]) * (ys - pa[1]) - (xs - pa[0]) * (pb[1] - pa[1])) / det
        l0 = 1 - l1 - l2
        inside = (l0 >= -1e-4) & (l1 >= -1e-4) & (l2 >= -1e-4)
        if not inside.any():
            continue
        yy, xx = np.nonzero(inside)
        flat = (yy + y0) * size + (xx + x0)
        ids[flat] = t
        w[flat] = np.stack([l0[inside], l1[inside], l2[inside]], axis=1)
    return ids, w


def polyline_distance(points, lines, chunk=200000):
    """For each point (mm), the distance to the nearest of `lines` (each a
    polyline, mm) and how far along that line the nearest place is."""
    segs, starts = [], []
    for line in lines:
        line = np.asarray(line, dtype=float)
        acc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(line, axis=0), axis=1))])
        for k in range(len(line) - 1):
            segs.append((line[k], line[k + 1]))
            starts.append(acc[k])
    if not segs:
        return np.full(len(points), 1e9), np.zeros(len(points))
    A = np.array([s[0] for s in segs])
    B = np.array([s[1] for s in segs])
    S = np.array(starts)
    AB = B - A
    L2 = np.maximum((AB ** 2).sum(axis=1), 1e-12)
    dist = np.empty(len(points))
    along = np.empty(len(points))
    for c0 in range(0, len(points), chunk):
        p = points[c0:c0 + chunk]
        best = np.full(len(p), np.inf)
        best_s = np.zeros(len(p))
        for k0 in range(0, len(A), 64):
            a, ab, l2, s = A[k0:k0 + 64], AB[k0:k0 + 64], L2[k0:k0 + 64], S[k0:k0 + 64]
            t = np.clip(((p[:, None, :] - a[None]) * ab[None]).sum(axis=2) / l2[None], 0, 1)
            q = a[None] + t[:, :, None] * ab[None]
            d = np.linalg.norm(p[:, None, :] - q, axis=2)
            j = d.argmin(axis=1)
            dj = d[np.arange(len(p)), j]
            better = dj < best
            best[better] = dj[better]
            best_s[better] = (s[j] + t[np.arange(len(p)), j] * np.sqrt(l2[j]))[better]
        dist[c0:c0 + chunk] = best
        along[c0:c0 + chunk] = best_s
    return dist, along


def dilate(img, valid, rounds=12):
    """Spreads each island's edge texels outward, so mip levels and
    filtering across an island's edge read its own colour."""
    img = img.copy()
    valid = valid.copy()
    for _ in range(rounds):
        acc = np.zeros_like(img)
        cnt = np.zeros(valid.shape, dtype=np.float32)
        for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            sv = np.roll(valid, (dy, dx), axis=(0, 1))
            si = np.roll(img, (dy, dx), axis=(0, 1))
            acc[sv] += si[sv]
            cnt[sv] += 1
        grow = (~valid) & (cnt > 0)
        img[grow] = acc[grow] / cnt[grow][..., None] if img.ndim == 3 else acc[grow] / cnt[grow]
        valid = valid | grow
    return img


def bake(lod, fine, atlas, fabric, out_prefix):
    """Bakes the maps. `lod`: the level's mesh (`finish.Mesh` with `uv01`,
    texture coordinates 0..1), `fine`: per piece (pattern mm (n, 2),
    positions (n, 3), normals (n, 3), triangles), `atlas`: the pattern ->
    texture transform (`to_uv(island, mm)`), `fabric`: the item's fabric
    description (tiles, stitch lines, seams, colours). Writes
    `<out_prefix>_color.jpg`, `_normal.jpg`, `_orm.jpg`."""
    P = lod.P
    uv = lod.uv01
    tris = []
    tri_mat = []
    for material, quads in lod.quads.items():
        for a, b, c, d in quads:
            tris.append((a, b, c))
            tris.append((a, c, d))
            tri_mat += [material, material]
    tris = np.array(tris)
    N = lod.normals
    ids, w = raster(uv, tris)
    valid = ids >= 0
    vi = np.nonzero(valid)[0]
    t = tris[ids[vi]]
    ww = w[vi]
    mm = (lod.uv[t] * ww[:, :, None]).sum(axis=1)
    n_lod = (N[t] * ww[:, :, None]).sum(axis=1)
    n_lod /= np.linalg.norm(n_lod, axis=1, keepdims=True)
    p_lod = (P[t] * ww[:, :, None]).sum(axis=1)
    island = np.array(lod.island)[t[:, 0]]
    kind = np.array(lod.kind)[t[:, 0]]
    # The tangent frame: U across the image (+X), up the image (+Y), from
    # each triangle's texture map.
    e1, e2 = P[t[:, 1]] - P[t[:, 0]], P[t[:, 2]] - P[t[:, 0]]
    d1, d2 = uv[t[:, 1]] - uv[t[:, 0]], uv[t[:, 2]] - uv[t[:, 0]]
    det = d1[:, 0] * d2[:, 1] - d2[:, 0] * d1[:, 1]
    det = np.where(np.abs(det) < 1e-12, 1e-12, det)
    T = (e1 * d2[:, 1:2] - e2 * d1[:, 1:2]) / det[:, None]
    Bv = (e2 * d1[:, 0:1] - e1 * d2[:, 0:1]) / det[:, None]  # along +v (down the image)
    T = T - n_lod * (T * n_lod).sum(axis=1, keepdims=True)
    T /= np.maximum(np.linalg.norm(T, axis=1, keepdims=True), 1e-12)
    Bup = -Bv
    Bup = Bup - n_lod * (Bup * n_lod).sum(axis=1, keepdims=True) - T * (Bup * T).sum(axis=1, keepdims=True)
    Bup /= np.maximum(np.linalg.norm(Bup, axis=1, keepdims=True), 1e-12)

    # The fine drape's normal and place at each texel of the shell.
    fn = np.zeros((SIZE * SIZE, 3), dtype=np.float32)
    fp = np.zeros((SIZE * SIZE, 3), dtype=np.float32)
    have = np.zeros(SIZE * SIZE, dtype=bool)
    for name, (fmm, fpos, fnor, ftris) in fine.items():
        fuv = atlas.to_uv(name, fmm)
        fid, fw = raster(fuv, ftris)
        hv = fid >= 0
        ft = ftris[fid[hv]]
        fwv = fw[hv]
        fn[hv] = (fnor[ft] * fwv[:, :, None]).sum(axis=1)
        fp[hv] = (fpos[ft] * fwv[:, :, None]).sum(axis=1)
        have |= hv
    use_fine = have[vi] & (kind == "shell")
    n_fine = np.where(use_fine[:, None], fn[vi], n_lod)
    n_fine /= np.maximum(np.linalg.norm(n_fine, axis=1, keepdims=True), 1e-12)
    # A fold the level can't hold is drawn by the map, but only so far:
    # past `FOLD_TILT` from the level's own normal (where the coarse mesh
    # cuts across a deep fold, under the arm) the map would show a crease
    # where the silhouette has none, so it eases back to the level's.
    cos = (n_fine * n_lod).sum(axis=1)
    keep = np.clip((cos - math.cos(math.radians(FOLD_TILT_MAX))) /
                   (math.cos(math.radians(FOLD_TILT)) - math.cos(math.radians(FOLD_TILT_MAX))), 0, 1)
    n_fine = n_lod + (n_fine - n_lod) * keep[:, None]
    n_fine /= np.maximum(np.linalg.norm(n_fine, axis=1, keepdims=True), 1e-12)
    depth = np.where(use_fine, ((fp[vi] - p_lod) * n_lod).sum(axis=1) * 1000, 0.0)  # mm, + out

    # A height field (mm) in pattern space: seams, stitches, fabric.
    height = np.zeros(len(vi))
    base = np.tile(np.array(fabric["base"], dtype=np.float32), (len(vi), 1))
    ao = np.ones(len(vi))
    rough = np.zeros(len(vi))
    albedo_var = np.ones(len(vi))
    thread = np.zeros(len(vi))
    for name in np.unique(island):
        sel = island == name
        pts = mm[sel]
        spec = fabric["islands"].get(name)
        if spec is None:
            # An island named `<kind>_<n>` takes `<kind>`'s fabric, else `*`.
            spec = fabric["islands"].get(name.rsplit("_", 1)[0], fabric["islands"].get("*"))
        if "base" in spec:
            base[sel] = np.array(spec["base"], dtype=np.float32)
        # Seam valleys.
        lines = fabric["seams"].get(name, [])
        if lines:
            d, _ = polyline_distance(pts, lines)
            valley = np.exp(-(d / 1.8) ** 2)
            height[sel] -= 0.45 * valley
            ao[sel] *= 1 - 0.28 * valley
        # Stitch rows: (polyline, offset from it in mm).
        for line, offset in fabric["stitches"].get(name, []):
            d, s = polyline_distance(pts, [line])
            across = np.abs(d - offset)
            along = np.mod(s, fabric["stitch_length"])
            on = (across < 0.45) & (along < fabric["stitch_length"] - 0.5)
            prof = np.where(on, np.cos(np.clip(across / 0.45, 0, 1) * math.pi / 2), 0.0)
            height[sel] += 0.28 * prof
            thread[sel] = np.maximum(thread[sel], prof)
            hole = (across < 0.5) & (along >= fabric["stitch_length"] - 0.5)
            ao[sel] *= np.where(hole, 0.8, 1.0)
        # The fabric's own height and light.
        tile = spec["height"]
        height[sel] += spec["amplitude"] * (sample_tile(tile, pts[:, 0] * spec["stretch"], pts[:, 1], spec["mm"]) - 0.5)
        # Holes punched in it (a shoe's eyelets): dark, recessed discs.
        for centre, radius in spec.get("holes", []):
            d = np.linalg.norm(pts - np.asarray(centre), axis=1)
            inside = d < radius
            height[sel] -= np.where(inside, 0.8, 0.0)
            ao[sel] *= np.where(inside, 0.25, np.where(d < radius + 1.2, 0.8, 1.0))
        lum = sample_tile(spec["light"], pts[:, 0] * spec["stretch"], pts[:, 1], spec["mm"])
        albedo_var[sel] = 1 + spec["contrast"] * (lum - spec["light_mean"]) / max(spec["light_mean"], 1e-3)
        rough[sel] = spec["rough_lo"] + (spec["rough_hi"] - spec["rough_lo"]) * sample_tile(
            spec["rough"], pts[:, 0] * spec["stretch"], pts[:, 1], spec["mm"])
        if spec.get("wales"):
            # A rib's wales: ridges along the band's height, `wales` mm apart.
            ph = 2 * math.pi * pts[:, 0] / spec["wales"]
            height[sel] += 0.22 * np.sin(ph)
            ao[sel] *= 1 - 0.06 * (1 - np.sin(ph)) / 2
    # Folds' recesses.
    ao *= np.clip(1 + 0.025 * np.minimum(depth, 0), 0.75, 1)

    # Height to normal, per texel, by finite differences in the atlas
    # (texels of other islands ignored).
    H = np.zeros(SIZE * SIZE)
    H[vi] = height
    Hm = H.reshape(SIZE, SIZE)
    V = valid.reshape(SIZE, SIZE)
    px_mm = 1.0 / atlas.scale
    gx = np.zeros_like(Hm)
    gy = np.zeros_like(Hm)
    right, left = np.roll(Hm, -1, axis=1), np.roll(Hm, 1, axis=1)
    rv, lv = np.roll(V, -1, axis=1), np.roll(V, 1, axis=1)
    gx = np.where(rv & lv, (right - left) / (2 * px_mm), np.where(rv, (right - Hm) / px_mm, np.where(lv, (Hm - left) / px_mm, 0)))
    down, up = np.roll(Hm, -1, axis=0), np.roll(Hm, 1, axis=0)
    dv, uv_ = np.roll(V, -1, axis=0), np.roll(V, 1, axis=0)
    gy = np.where(dv & uv_, (down - up) / (2 * px_mm), np.where(dv, (down - Hm) / px_mm, np.where(uv_, (Hm - up) / px_mm, 0)))
    gx, gy = gx.reshape(-1)[vi], gy.reshape(-1)[vi]
    # The fold normal in the level's tangent frame, then tilted by the
    # height's slope (+Y is up the image, against v).
    nt = np.stack([(n_fine * T).sum(axis=1), (n_fine * Bup).sum(axis=1), (n_fine * n_lod).sum(axis=1)], axis=1)
    nt[:, 0] -= gx
    nt[:, 1] += gy
    nt /= np.linalg.norm(nt, axis=1, keepdims=True)

    def image(values, channels):
        img = np.zeros((SIZE * SIZE, channels), dtype=np.float32)
        img[vi] = values
        img = img.reshape(SIZE, SIZE, channels)
        return dilate(img, V)

    nimg = image(nt * 0.5 + 0.5, 3)
    colour = np.clip(base * albedo_var[:, None] * (0.55 + 0.45 * ao[:, None]), 0, 1)
    cimg = image(colour, 3)
    rough = np.where(thread > 0.3, fabric.get("thread_rough", 0.6), rough)
    orm = np.stack([ao, rough, np.zeros_like(ao)], axis=1)
    oimg = image(orm, 3)

    def save(img, path, srgb):
        if srgb:
            img = np.where(img <= 0.0031308, img * 12.92, 1.055 * np.power(np.clip(img, 0, 1), 1 / 2.4) - 0.055)
        Image.fromarray(np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)).save(path, quality=92, subsampling=0)

    save(cimg, out_prefix + "_color.jpg", True)
    save(nimg, out_prefix + "_normal.jpg", False)
    save(oimg, out_prefix + "_orm.jpg", False)
    return {
        "texels": int(valid.sum()),
        "fold_depth_mm": [float(depth.min()), float(depth.max())],
        "texels_per_mm": float(atlas.scale),
    }
