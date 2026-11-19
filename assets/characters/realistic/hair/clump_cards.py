"""Cards from a guide groom's clumps, and the atlas drawn from their strands
(Character Studio wave 5, decision 14).

- **A card per clump.** It follows the clump's centre strand (curved over
  the head, rising and lifting or settling at its tip), as wide as the
  clump's roots, a little narrower toward the tip, and bowed across (its
  middle row raised off the scalp), so it isn't a flat plank. A clump of
  the outer layer has a second card turned across the first, so it has
  depth from the side. Normals are the scalp's (cards shade as one
  surface, highlights run over the head).
- **Tiles drawn from strands.** Each layer has a few tiles. A tile is a
  clump's strands drawn in the clump's own frame (`guides.clump_strands`):
  across the card and along it, root at the top. So a tile shows what the
  card carries: strands spread at the root, converging to pointed tips that
  end at staggered lengths, frizz and strays. No strand touches the tile's
  sides and no tile ends on one row: no card shows a straight alpha edge.
- **No cap.** The wave-4 crop's cap (the scalp lifted and mapped round the
  crown) made the crown's pinwheel and the grey band at the hairline. Here
  a low, wide `under` layer of clumps covers the scalp, combed like the
  rest, so the crown is cards spiralling out of the whorl.

The atlas is baked by `atlas.draw` (the Hair model's layout: colour and
coverage, the data map, the flow map, a normal map of the strands'
relief).
"""

import os
import zlib

import numpy as np
from PIL import Image

import atlas
from atlas import SIZE, PAD, Island
from cards import Mesh, keep_clear
from groom import rotate, unit
from guides import clump_strands


# The normal map's relief: its strength, and how much of it is each
# strand's own height. Strands lighter and darker than their neighbours
# (melanin spread, as `atlas.MELANIN_SPREAD`, wider than wave 4's so a
# clump reads as strands at arm's length).
RELIEF = 2.2
RELIEF_OWN = 0.8
MELANIN_SPREAD = 0.45


def layout(style):
    """The tiles of every layer, packed in rows: {layer: [Island]}."""
    out = {}
    x, y, row = PAD, PAD, 0
    for layer in style["layers"]:
        if "tiles_from" in layer:
            continue
        w, h = layer["tile"]
        for _ in range(layer["tiles"]):
            if x + w + PAD > SIZE:
                x, y, row = PAD, y + row + PAD, 0
            out.setdefault(layer["name"], []).append(Island(layer["name"], x, y, w, h))
            x += w + PAD
            row = max(row, h)
    if "cap" in style:
        w, h = style["cap"]["tile"]
        if x + w + PAD > SIZE:
            x, y, row = PAD, y + row + PAD, 0
        out["cap"] = [Island("cap", x, y, w, h)]
        row = max(row, h)
    assert y + row + PAD <= SIZE, "the tiles don't fit the atlas"
    return out


def tile_strands(style, layer, index):
    """The strands drawn in one tile, deepest first, in pixels of the tile
    (x across, y along, root at the top)."""
    rng = np.random.default_rng([style["seed"], zlib.crc32(layer["name"].encode()), index])
    count = int(rng.integers(*layer["strands"]))
    strands = clump_strands(None, layer, count, rng)
    strands.sort(key=lambda s: s["up"].mean())
    return strands, rng


def bake(style, out_dir, name):
    """Draws every tile and writes the four maps (as `atlas.bake`)."""
    islands = layout(style)
    colour = np.zeros((SIZE, SIZE, 3))
    alpha = np.zeros((SIZE, SIZE))
    data = np.zeros((SIZE, SIZE, 4))
    data[..., 3] = 1.0
    flow = np.zeros((SIZE, SIZE, 2))
    flow[..., 1] = 1.0
    normal = np.zeros((SIZE, SIZE, 3))
    normal[..., 2] = 1.0
    atlas.MELANIN_SPREAD = MELANIN_SPREAD
    gains = {layer["name"]: layer["gain"] for layer in style["layers"] if "gain" in layer}
    atlas.GAIN.update(gains)
    tiles = []

    def put(island, ch):
        ys = slice(island.y, island.y + island.h)
        xs = slice(island.x, island.x + island.w)
        colour[ys, xs] = atlas.fill_clear(ch["colour"], ch["alpha"])
        alpha[ys, xs] = ch["alpha"]
        data[ys, xs, 0] = ch["root_to_tip"]
        data[ys, xs, 1] = ch["own"]
        data[ys, xs, 2] = ch["streak"]
        data[ys, xs, 3] = np.where(ch["alpha"] > 0, ch["ao"], 1.0)
        f = ch["flow"]
        ln = np.linalg.norm(f, axis=-1, keepdims=True)
        flow[ys, xs] = np.where(ln > 1e-6, f / np.maximum(ln, 1e-9), np.array([0.0, 1.0]))
        # Relief: the coverage, plus a height of each strand's own (its
        # per-strand value), so the normal map turns from strand to strand
        # inside a dense clump and the highlight breaks into strands
        # instead of lying over the clump as one sheet.
        height = ch["alpha"] * (0.6 + RELIEF_OWN * ch["own"])
        gx = np.zeros_like(height)
        gy = np.zeros_like(height)
        gx[:, 1:-1] = (height[:, 2:] - height[:, :-2]) * 0.5
        gy[1:-1, :] = (height[2:, :] - height[:-2, :]) * 0.5
        nrm = np.stack([-gx * RELIEF, gy * RELIEF, np.ones_like(gx)], axis=-1)
        normal[ys, xs] = nrm / np.linalg.norm(nrm, axis=-1, keepdims=True)

    for layer in style["layers"]:
        if "tiles_from" in layer:
            continue
        for index, island in enumerate(islands[layer["name"]]):
            strands, rng = tile_strands(style, layer, index)
            drawn = [{"points": np.stack([(0.5 + s["across"]) * island.w, s["along"] * island.h], axis=1),
                      "width": s["width"], "taper": s["taper"]} for s in strands]
            put(island, atlas.draw(island, drawn, rng))
            tiles.append({"kind": layer["name"], "x": island.x, "y": island.y, "w": island.w, "h": island.h})
    cap_tile = None
    if "cap" in islands:
        # The cap's patch: a dense mat of short strands running down (+V),
        # whole (alpha 1) inside a 3-texel clear border and darker than the
        # cards (it is the shadowed depth between clumps).
        island = islands["cap"][0]
        rng = np.random.default_rng([style["seed"], zlib.crc32(b"cap")])
        atlas.GAIN["cap"] = 3.0
        strands = atlas.periodic_strands(rng, island.w, island.h, style["cap"]["strands"], (0.05, 0.16), 1.3)
        ch = atlas.draw(island, strands, rng)
        inner = np.zeros_like(ch["alpha"], dtype=bool)
        inner[3:-3, 3:-3] = True
        ch["alpha"] = np.where(inner, 1.0, 0.0)
        ch["colour"] = ch["colour"] * style["cap"]["shade"]
        put(island, ch)
        cap_tile = {"x": island.x, "y": island.y, "w": island.w, "h": island.h}
    srgb = np.where(colour <= 0.0031308, colour * 12.92,
                    1.055 * np.power(np.maximum(colour, 0), 1 / 2.4) - 0.055)
    rgba = np.zeros((SIZE, SIZE, 4), dtype=np.uint8)
    rgba[..., :3] = np.clip(srgb * 255 + 0.5, 0, 255).astype(np.uint8)
    rgba[..., 3] = np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)
    covered = rgba[..., 3] > 0
    rgba[colour.sum(-1) == 0, :3] = rgba[covered][:, :3].mean(0).astype(np.uint8)
    files = {
        "colour": os.path.join(out_dir, name + "_Colour.png"),
        "data": os.path.join(out_dir, name + "_Data.png"),
        "flow": os.path.join(out_dir, name + "_Flow.png"),
        "normal": os.path.join(out_dir, name + "_Normal.png"),
    }
    Image.fromarray(rgba, "RGBA").save(files["colour"], optimize=True)
    Image.fromarray(np.clip(data * 255 + 0.5, 0, 255).astype(np.uint8), "RGBA").save(files["data"], optimize=True)
    fl = np.zeros((SIZE, SIZE, 3), dtype=np.uint8)
    fl[..., :2] = np.clip((0.5 + 0.5 * flow) * 255 + 0.5, 0, 255).astype(np.uint8)
    Image.fromarray(fl, "RGB").save(files["flow"], optimize=True)
    nm = np.clip((0.5 + 0.5 * normal) * 255 + 0.5, 0, 255).astype(np.uint8)
    Image.fromarray(nm, "RGB").save(files["normal"], optimize=True)
    return islands, files, tiles, cap_tile


def card(points, normals, sides, width, island, flip, twist, bow, taper):
    """A card along a clump's centre strand (root first): three vertices
    across, the middle raised `bow` × width off the layer, `width` wide at
    the root and `taper` narrower at the tip, turned `twist` degrees about
    the strand; mapped root to tip down `island`."""
    m = len(points)
    lengths = np.concatenate([[0.0], np.cumsum(np.linalg.norm(np.diff(points, axis=0), axis=1))])
    along = lengths / max(lengths[-1], 1e-9)
    pos, nrm, uv = [], [], []
    for k in range(m):
        t = along[k]
        tangent = unit(points[min(k + 1, m - 1)] - points[max(k - 1, 0)])
        side = unit(sides[k] - tangent * (sides[k] @ tangent))
        side = rotate(side, tangent, twist)
        lie = unit(np.cross(side, tangent))
        if lie @ normals[k] < 0:
            lie = -lie
        half = 0.5 * width * (1.0 - taper * t)
        for j, x in enumerate((-1.0, 0.0, 1.0)):
            pos.append(points[k] + side * half * x + lie * bow * width * (1.0 - x * x))
            nrm.append(normals[k])
            u = 0.5 * (x + 1.0)
            u = 1.0 - u if flip else u
            uv.append(island.uv(u, t))
    uv = np.array([[a[0], a[1]] for a in uv])
    roots = np.zeros(len(pos), dtype=bool)
    roots[:3] = True
    tris = []
    for k in range(m - 1):
        for j in range(2):
            a, b = k * 3 + j, k * 3 + j + 1
            c, d = a + 3, b + 3
            tris += [(a, c, b), (b, c, d)]
    return np.array(pos), np.array(nrm), uv, np.array(tris), roots


def layer_cards(groom, layer, islands, level):
    """The cards of one layer at `level` (0 hero, 1 gameplay, 2 crowd)."""
    name = layer["name"]
    if level == 2 and not layer.get("crowd"):
        return None
    keep = 1.0 if level == 0 else (layer["keep"] if level == 1 else 1.0)
    if keep <= 0.0:
        return None
    clumps = groom.clumps(layer, level)
    pick = np.random.default_rng([groom.style["seed"], level, zlib.crc32(name.encode())])
    tiles = islands[layer.get("tiles_from", name)]
    mesh = Mesh()
    for c in clumps:
        if level == 1 and pick.random() > keep:
            continue
        rng = np.random.default_rng(c["seed"])
        width = c["width"] * (layer.get("crowd_width", 1.6) if level == 2 else 1.0)
        island = tiles[int(rng.integers(len(tiles)))]
        twist = rng.normal(0.0, 7.0)
        bow = layer["bow"] if level < 2 else 0.0
        pos, nrm, uv, tris, root = card(c["points"], c["normals"], c["sides"], width, island,
                                        rng.random() < 0.5, twist, bow, layer["card_taper"])
        mesh.add(pos, nrm, uv, tris, name, root)
        # The outer layer's second card, turned across the first.
        cross = layer.get("cross", 0.0)
        if level == 0 and cross > 0 and rng.random() < cross:
            other = tiles[int(rng.integers(len(tiles)))]
            pos, nrm, uv, tris, root = card(c["points"] + c["normals"] * 0.001, c["normals"], c["sides"],
                                            width * 0.75, other, rng.random() < 0.5,
                                            twist + rng.choice([-1, 1]) * rng.uniform(20, 35), 0.0,
                                            layer["card_taper"])
            mesh.add(pos, nrm, uv, tris, name, root)
    return mesh


def cap(groom, islands, level):
    """The cap: the scalp's triangles whose corners are all past
    `inset` of the hairline's ramp, lifted `lift` off the skin, whole and
    dark, under every card, so no skin shows between clumps. Each triangle
    is mapped on its own into the cap's patch, flat in its plane with +V
    along the comb there (the strands' direction for the Hair model), at
    `px_per_cm`, at a place drawn in the patch: no point of the map meets
    another (the wave-4 cap's polar map made a pinwheel at the crown).
    Seams between triangles fall in a uniform mat of short strands."""
    spec = groom.style["cap"]
    body = groom.body
    if level == 2:
        co, tris = groom.lod2_scalp()
        normals = groom.lod2_normals()
    else:
        co, tris, normals = body.co, groom.tris, body.normals
    used = np.unique(tris)
    f = np.full(len(co), -1.0)
    f[used] = groom.hairline.signed(co[used])
    ramp = np.zeros(len(co))
    ramp[used] = groom.hairline.ramp(co[used])
    keep = tris[(f[tris] > spec["inset"] * ramp[tris]).all(1)]
    island = islands["cap"][0]
    rng = np.random.default_rng([groom.style["seed"], level, zlib.crc32(b"cap-map")])
    scale = spec["px_per_cm"] * 100.0
    pos, nrm, uv, out = [], [], [], []
    for tri in keep:
        p = co[tri] + normals[tri] * spec["lift"]
        centre = p.mean(0)
        n = unit(normals[tri].mean(0))
        d = groom.comb(centre, n)
        side = unit(np.cross(n, d))
        local = np.stack([(p - centre) @ side, (p - centre) @ d], axis=1) * scale
        margin = np.abs(local).max() + 4
        if margin > 0.5 * min(island.w, island.h) - 1:
            # A big triangle (the crowd copy's): mapped coarser to fit.
            local *= (0.5 * min(island.w, island.h) - 5) / (margin - 4)
            margin = np.abs(local).max() + 4
        cx = rng.uniform(margin, island.w - margin)
        cy = rng.uniform(margin, island.h - margin)
        base = len(pos)
        for k in range(3):
            pos.append(p[k])
            nrm.append(normals[tri[k]])
            uv.append(((island.x + cx + local[k, 0]) / SIZE, (island.y + cy + local[k, 1]) / SIZE))
        out.append((base, base + 1, base + 2))
    mesh = Mesh()
    mesh.add(np.array(pos), np.array(nrm), np.array(uv), np.array(out), "cap")
    return mesh


def build(groom, islands, level):
    """Every card of every layer at `level`, as one mesh, with each
    non-root vertex kept clear of the body."""
    mesh = Mesh()
    if "cap" in groom.style:
        mesh.add(*cap(groom, islands, level).arrays()[:4], "cap")
    for layer in groom.style["layers"]:
        part = layer_cards(groom, layer, islands, level)
        if part is None or not part.positions:
            continue
        pos, nrm, uv, tris, layers, roots = part.arrays()
        pos = keep_clear(groom, pos, roots)
        mesh.add(pos, nrm, uv, tris, layers, roots)
    return mesh
