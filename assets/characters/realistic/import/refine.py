"""Step 5 of the intake (optional, with a `refine` block): refine a
garment in Blender by script, under review (asset-quality.md section 6, "Blender and manual modelling":
refinement by scripts a person reviews). Blender's Python.

Each operation is a key of the spec's `refine` block, run in this order;
every one is counted in `refined.json`:

- `unwrap`: `{"method": "ANGLE_BASED", "margin": uv}`: the texture
  coordinates laid out again along the source's own island seams, so the
  maps aren't stretched (`unwrap`).
- `rename`: `{"<source material>": "<name>"}` (`LowPolyTShirt` is
  `Jersey`).
- `materials`: `{"<name>": {"from": "<source material>", "near_edge":
  {"loop": "highest"|"lowest", "within": m}}}`: faces whose every corner
  is within `within` (along the surface) of the boundary loop named take
  the new material (a tee's rib neck band from its body's jersey).
- `hems`: `[{"loop": "highest"|"lowest"|"left"|"right", "thickness": m,
  "turn_up": m, "rows": n, "material": name}]`: each open edge loop
  named is turned under: the cloth rolls over the edge in a rounded fold
  `thickness` thick and lies back inside for `turn_up` (asset-quality.md
  section 3: no single-layer edge, at least 2 mm visible thickness). The
  turned strips' texture coordinates are packed into the free part of
  the atlas at the panels' own density (`Atlas`).
- `maps`: the maps baked on the refined texture coordinates from a CC0
  photographed fabric (`bake`): jersey, a rib's wales, the seams and the
  hems' and band's stitching. Every material then uses them; the source's
  own maps are dropped (a drape baked on another body, and a roughness map
  glTF would read as metal).

    python -I refine.py --spec <asset>.intake.json --work <folder>
"""

import argparse
import json
import math
import os

import numpy as np

from workmesh import WorkMesh


def edges_of(mesh):
    """Unique undirected edges of a work mesh, (n, 2)."""
    seen = set()
    for f in mesh.faces():
        n = len(f)
        for k in range(n):
            a, b = int(f[k]), int(f[(k + 1) % n])
            seen.add((min(a, b), max(a, b)))
    return np.array(sorted(seen), np.int64)


def boundary_loops(mesh):
    """The open edge loops (each a list of vertex indices, in order)."""
    count = {}
    for f in mesh.faces():
        n = len(f)
        for k in range(n):
            a, b = int(f[k]), int(f[(k + 1) % n])
            key = (min(a, b), max(a, b))
            count[key] = count.get(key, 0) + 1
    # Directed boundary edges, as the faces run (outside on the right).
    nxt = {}
    for f in mesh.faces():
        n = len(f)
        for k in range(n):
            a, b = int(f[k]), int(f[(k + 1) % n])
            if count[(min(a, b), max(a, b))] == 1:
                nxt[a] = b
    loops, seen = [], set()
    for start in nxt:
        if start in seen:
            continue
        loop, v = [], start
        while v not in seen:
            seen.add(v)
            loop.append(v)
            v = nxt[v]
        loops.append(loop)
    return loops


def pick_loop(mesh, which):
    loops = boundary_loops(mesh)
    if which == "highest":
        return max(loops, key=lambda lp: mesh.P[lp, 1].mean())
    if which == "lowest":
        return min(loops, key=lambda lp: mesh.P[lp, 1].mean())
    if which in ("left", "right"):
        side = 1 if which == "left" else -1
        return max(loops, key=lambda lp: side * mesh.P[lp, 0].mean())
    raise SystemExit(f"no loop `{which}`: highest, lowest, left or right")


def welded_neighbours(mesh):
    """Each vertex's neighbours, by edges."""
    nb = [set() for _ in range(len(mesh.P))]
    for a, b in edges_of(mesh):
        nb[a].add(b)
        nb[b].add(a)
    return [sorted(s) for s in nb]


# -- unwrap ------------------------------------------------------------------

def unwrap(mesh, options):
    """The texture coordinates laid out again (Blender's unwrap), island by
    island along the source's own seams, so the texture isn't stretched
    (asset-quality.md section 2: under 10% on 95% of the area): a base
    whose islands were flattened from a pattern but stretched over its
    drape. `method` is Blender's (`ANGLE_BASED` or `CONFORMAL`); the islands
    are packed again with `margin` (UV units) round each, and keep the
    upright grain of the source (rotation off)."""
    import blendmesh  # (imports bpy first)
    import bpy
    obj = blendmesh.to_object(mesh)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.select_all(action="SELECT")
    bpy.ops.uv.seams_from_islands(mark_seams=True)
    bpy.ops.uv.unwrap(method=options.get("method", "ANGLE_BASED"), margin=0.0)
    bpy.ops.uv.pack_islands(rotate=False, margin=float(options.get("margin", 0.01)))
    bpy.ops.object.mode_set(mode="OBJECT")
    out = blendmesh.from_object(obj, mesh.materials)
    return out, {"method": options.get("method", "ANGLE_BASED"), "islands_kept": True}


# -- materials ---------------------------------------------------------------

def surface_distance(mesh, sources):
    """Each vertex's distance along the mesh's edges from `sources`
    (Dijkstra)."""
    import heapq
    nb = welded_neighbours(mesh)
    dist = np.full(len(mesh.P), np.inf)
    heap = []
    for s in sources:
        dist[s] = 0.0
        heap.append((0.0, s))
    heapq.heapify(heap)
    while heap:
        d, v = heapq.heappop(heap)
        if d > dist[v]:
            continue
        for w in nb[v]:
            nd = d + float(np.linalg.norm(mesh.P[w] - mesh.P[v]))
            if nd < dist[w]:
                dist[w] = nd
                heapq.heappush(heap, (nd, w))
    return dist


def split_materials(mesh, options):
    report = {}
    for name, rule in options.items():
        source = next(k for k, m in enumerate(mesh.materials) if m["name"] == rule["from"])
        if all(m["name"] != name for m in mesh.materials):
            new = dict(mesh.materials[source])
            new["name"] = name
            mesh.materials.append(new)
        index = next(k for k, m in enumerate(mesh.materials) if m["name"] == name)
        edge = rule["near_edge"]
        dist = surface_distance(mesh, pick_loop(mesh, edge["loop"]))
        count = 0
        for f, face in enumerate(mesh.faces()):
            if mesh.face_material[f] == source and np.all(dist[face] <= float(edge["within"]) + 1e-6):
                mesh.face_material[f] = index
                count += 1
        report[name] = {"faces": count}
    return report


# -- hems --------------------------------------------------------------------

def vertex_normals(mesh):
    T = mesh.triangles()
    P = mesh.P
    face = np.cross(P[T[:, 1]] - P[T[:, 0]], P[T[:, 2]] - P[T[:, 0]])
    N = np.zeros_like(P)
    for k in range(3):
        np.add.at(N, T[:, k], face)
    return N / np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)


class Atlas:
    """The free texture space for the turned strips: the part of the UV
    square the panels leave free (rasterized), filled in rows."""

    def __init__(self, mesh, density, resolution=1024, margin=0.004):
        self.density = density  # UV units per metre
        self.margin = margin
        r = resolution
        taken = np.zeros((r, r), bool)
        for t in mesh.triangle_corners():
            uv = mesh.uv[t]
            lo = np.floor(uv.min(axis=0) * r).astype(int) - 3
            hi = np.ceil(uv.max(axis=0) * r).astype(int) + 3
            lo, hi = np.clip(lo, 0, r), np.clip(hi, 0, r)
            taken[lo[1]:hi[1], lo[0]:hi[0]] = True
        self.taken = taken
        self.resolution = r

    def runs(self, height):
        """Free runs a strip `height` (UV) tall fits in, row by row:
        [(v0, u0, u1)]."""
        r = self.resolution
        rows = int(math.ceil((height + self.margin) * r)) + 1
        out = []
        for v in range(0, r - rows + 1, rows):
            band = self.taken[v:v + rows].any(axis=0)
            u = 0
            while u < r:
                if band[u]:
                    u += 1
                    continue
                start = u
                while u < r and not band[u]:
                    u += 1
                out.append((v / r, start / r + self.margin, u / r - self.margin))
        # The longest first, so a strip is cut as few times as can be.
        return sorted(out, key=lambda run: run[1] - run[2])

    def take(self, u0, v0, u1, v1):
        r = self.resolution
        c0, c1 = int(u0 * r) - 2, int(math.ceil(u1 * r)) + 2
        r0, r1 = int(v0 * r) - 2, int(math.ceil(v1 * r)) + 2
        self.taken[max(r0, 0):r1, max(c0, 0):c1] = True


def turn_hem(mesh, loop, normals, options, atlas, material):
    """Adds the turned hem of one boundary loop (the module's note)."""
    thick = float(options.get("thickness", 0.003))
    turn = float(options.get("turn_up", 0.018))
    P = mesh.P
    n = len(loop)
    # Into the panel, along the surface, at each loop corner.
    inward = np.zeros((n, 3))
    nb = welded_neighbours(mesh)
    on_loop = set(loop)
    for i, v in enumerate(loop):
        inner = [w for w in nb[v] if w not in on_loop]
        d = P[inner].mean(axis=0) - P[v] if inner else -normals[v]
        d = d - normals[v] * np.dot(d, normals[v])
        inward[i] = d / max(np.linalg.norm(d), 1e-9)
    for _ in range(3):
        inward = (np.roll(inward, 1, axis=0) + 2 * inward + np.roll(inward, -1, axis=0)) / 4
        inward /= np.maximum(np.linalg.norm(inward, axis=1, keepdims=True), 1e-9)
    N = normals[loop]
    edge = P[loop]
    # The fold: a half-round of diameter `thick` from the outer face, out
    # past the edge, to the inner layer `thick` behind it; then the turn-up
    # back along the inside.
    radius = thick / 2
    centre = edge - N * radius
    rows = int(options.get("rows", 3))
    rings = []
    for r in range(1, rows + 1):
        a = math.pi * r / (rows + 1)
        rings.append(centre + N * radius * math.cos(a) - inward * radius * math.sin(a))
    rings.append(edge - N * thick)
    rings.append(edge - N * thick + inward * turn)
    start = len(P)
    mesh.P = np.vstack([P] + rings)
    ring_index = [np.array(loop)] + [start + k * n + np.arange(n) for k in range(len(rings))]
    # Texture coordinates: the strip unrolled, u along the loop and v
    # across the fold, its columns laid in the atlas's free runs (a strip
    # longer than a run goes on in the next one).
    seg = np.linalg.norm(np.roll(edge, -1, axis=0) - edge, axis=1)
    across = [0.0]
    for k in range(1, len(ring_index)):
        step_ = np.linalg.norm(mesh.P[ring_index[k]] - mesh.P[ring_index[k - 1]], axis=1)
        across.append(across[-1] + float(np.median(step_)))
    height = across[-1] * atlas.density
    runs = atlas.runs(height)
    at, u = 0, None
    faces, uvs, mats, used = [], [], [], []
    for i in range(n):
        j = (i + 1) % n
        width = seg[i] * atlas.density
        while at < len(runs) and (u if u is not None else runs[at][1]) + width > runs[at][2]:
            at, u = at + 1, None
        if at >= len(runs):
            raise SystemExit("the atlas has no room left for the turned hems")
        v0, u0, _ = runs[at]
        if u is None:
            u = u0
            used.append([u0, v0, u0, v0 + height])
        used[-1][2] = u + width
        for k in range(len(ring_index) - 1):
            a, b = ring_index[k][i], ring_index[k][j]
            c, d = ring_index[k + 1][j], ring_index[k + 1][i]
            # The panel's faces run a -> b along the loop (outside on the
            # left, seen from outside); the strip goes on past the loop, so
            # its faces run b -> a.
            faces.append([b, a, d, c])
            va, vb = v0 + across[k] * atlas.density, v0 + across[k + 1] * atlas.density
            uvs += [(u + width, va), (u, va), (u, vb), (u + width, vb)]
            mats.append(material)
        u += width
    for u0, v0, u1, v1 in used:
        atlas.take(u0, v0, u1, v1)
    mesh.corners = np.concatenate([mesh.corners, np.array(faces, np.int64).ravel()])
    mesh.sizes = np.concatenate([mesh.sizes, np.full(len(faces), 4, np.int64)])
    mesh.uv = np.vstack([mesh.uv, np.array(uvs)])
    mesh.face_material = np.concatenate([mesh.face_material, np.array(mats, np.int64)])
    return {"corners": n, "length_m": round(float(seg.sum()), 3), "strip_width_m": round(across[-1], 4),
            "thickness_m": thick, "turn_up_m": turn, "atlas_runs": len(used)}


def uv_density(mesh):
    """The panels' texture density: UV units per metre (median over
    triangles)."""
    T = mesh.triangle_corners()
    P = mesh.P[mesh.corners[T]]
    UV = mesh.uv[T]
    area3 = np.linalg.norm(np.cross(P[:, 1] - P[:, 0], P[:, 2] - P[:, 0]), axis=1) / 2
    e1, e2 = UV[:, 1] - UV[:, 0], UV[:, 2] - UV[:, 0]
    area2 = np.abs(e1[:, 0] * e2[:, 1] - e1[:, 1] * e2[:, 0]) / 2
    ok = area3 > 1e-8
    return float(np.median(np.sqrt(area2[ok] / area3[ok])))


def edge_uvs(mesh):
    """Each directed face edge (a, b) and the texture coordinates its face
    gives a and b."""
    out = {}
    for s0, n in zip(mesh.starts, mesh.sizes):
        for k in range(n):
            i, j = s0 + k, s0 + (k + 1) % n
            out[(int(mesh.corners[i]), int(mesh.corners[j]))] = (mesh.uv[i], mesh.uv[j])
    return out


def hems(mesh, options):
    """Turns every hem named; returns the report and each hemmed edge's
    texture coordinates on the panel (where its stitching goes)."""
    normals = vertex_normals(mesh)
    atlas = Atlas(mesh, uv_density(mesh))
    report, edges = [], []
    loops = {hem["loop"]: pick_loop(mesh, hem["loop"]) for hem in options}
    panel = edge_uvs(mesh)
    for hem in options:
        loop = loops[hem["loop"]]
        line = [panel[(loop[i], loop[(i + 1) % len(loop)])] for i in range(len(loop))]
        edges.append(np.array([[a, b] for a, b in line]))
        name = hem.get("material")
        material = (next(k for k, m in enumerate(mesh.materials) if m["name"] == name)
                    if name else 0)
        r = turn_hem(mesh, loop, normals, hem, atlas, material)
        r["loop"] = hem["loop"]
        report.append(r)
    return report, edges


# -- maps --------------------------------------------------------------------

def seams(mesh, panels):
    """Where the texture's islands meet on the surface (a sewn seam):
    each face edge whose neighbour across it maps it elsewhere; and where
    two materials meet (a band's seam). Texture segments."""
    uvs = edge_uvs(mesh)
    face_of = {}
    for f, (s0, n) in enumerate(zip(mesh.starts, mesh.sizes)):
        for k in range(n):
            face_of[(int(mesh.corners[s0 + k]), int(mesh.corners[s0 + (k + 1) % n]))] = f
    island, band = [], []
    for (a, b), (ua, ub) in uvs.items():
        other = uvs.get((b, a))
        # Only between the panels' own faces (`panels` of them): a turned
        # strip's cuts and its fold are no seams.
        if other is None or face_of[(a, b)] >= panels or face_of[(b, a)] >= panels:
            continue
        if np.abs(other[1] - ua).max() > 1e-5 or np.abs(other[0] - ub).max() > 1e-5:
            island.append([ua, ub])
        elif mesh.face_material[face_of[(a, b)]] != mesh.face_material[face_of[(b, a)]]:
            band.append([ua, ub])
    return np.array(island), np.array(band)


def bake(mesh, options, hemmed, panels, work):
    """Bakes the maps on the refined mesh's own texture coordinates (wave
    5): the fabric's height, light and roughness from a CC0 photographed
    tile (`fabric`, from the folder of `../clothing/fabrics.py`'s tiles the
    environment's `AXCENITY_FABRIC_TILES` names), tiled at its real size;
    a rib's wales on the materials named in `rib`; a soft valley along
    every seam; the twin-needle cover stitch along each hem
    (`stitch_rows`, mm from the edge) and a band's topstitch
    (`band_stitch`, mm from its seam); occlusion in the seams and stitch
    holes. Writes `<name>_color.jpg`, `_normal.jpg` and `_orm.jpg` into
    `work/textures` and points every material at them."""
    import sys
    sys.path.append(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "clothing"))
    import textures as tx  # the realistic wardrobe's bake helpers
    from PIL import Image
    tiles = os.environ.get("AXCENITY_FABRIC_TILES")
    if not tiles:
        raise SystemExit("set AXCENITY_FABRIC_TILES to the folder fabrics.py fetched the CC0 tiles into")
    size = int(options.get("size", 2048))
    fabric, tile_mm = options.get("fabric", "cotton_jersey"), float(options.get("tile_mm", 263.6))
    density = uv_density(mesh)  # texture units per metre
    mm_per_uv = 1000.0 / density
    px_mm = mm_per_uv / size  # the millimetres a texel spans
    scale = 1.0 / px_mm

    def tile(kind):
        return tx.load_tile(os.path.join(tiles, fabric, f"{fabric}_{kind}_2k.jpg"), tile_mm, scale)

    def to_mm(seg):
        # Texture space with v down (the image's rows), in millimetres.
        return (np.asarray(seg) * [1, -1] + [0, 1]) * mm_per_uv

    disp, light, rough_t = tile("disp"), tile("diff"), tile("rough")
    uv_img = mesh.uv * [1, -1] + [0, 1]
    tc = mesh.triangle_corners()
    tri_mat = mesh.face_material[mesh.triangle_faces()]
    ids, w = tx.raster(uv_img, tc, size)
    valid = ids >= 0
    vi = np.nonzero(valid)[0]
    mm = (uv_img[tc[ids[vi]]] * w[vi][:, :, None]).sum(axis=1) * mm_per_uv
    mat = tri_mat[ids[vi]]
    rib_ids = [k for k, m in enumerate(mesh.materials) if m["name"] in options.get("rib", [])]
    rib = np.isin(mat, rib_ids)
    height = float(options.get("amplitude", 0.18)) * (tx.sample_tile(disp, mm[:, 0], mm[:, 1], tile_mm) - 0.5)
    lum = tx.sample_tile(light, mm[:, 0], mm[:, 1], tile_mm)
    mean = float(light.mean())
    albedo = 1 + float(options.get("contrast", 0.6)) * (lum - mean) / max(mean, 1e-3)
    rough = 0.72 + 0.20 * tx.sample_tile(rough_t, mm[:, 0], mm[:, 1], tile_mm)
    ao = np.ones(len(vi))
    thread = np.zeros(len(vi))
    phase = 2 * math.pi * mm[:, 0] / float(options.get("wales_mm", 2.4))
    height = np.where(rib, height * 0.6 + 0.22 * np.sin(phase), height)
    ao = np.where(rib, ao * (1 - 0.06 * (1 - np.sin(phase)) / 2), ao)
    island, band = seams(mesh, panels)
    if len(island):
        d, _ = tx.polyline_distance(mm, [to_mm(seg) for seg in island])
        valley = np.exp(-(d / 1.8) ** 2)
        height -= 0.45 * valley
        ao *= 1 - 0.28 * valley
    stitch = float(options.get("stitch_length", 3.0))

    def stitches(lines, offset, where):
        idx = np.nonzero(where)[0]
        d, s = tx.polyline_distance(mm[idx], lines)
        across = np.abs(d - offset)
        along = np.mod(s, stitch)
        on = (across < 0.45) & (along < stitch - 0.5)
        prof = np.where(on, np.cos(np.clip(across / 0.45, 0, 1) * math.pi / 2), 0.0)
        height[idx] += 0.28 * prof
        thread[idx] = np.maximum(thread[idx], prof)
        hole = (across < 0.5) & (along >= stitch - 0.5)
        ao[idx] *= np.where(hole, 0.8, 1.0)

    shirt = ~rib
    for loop in hemmed:
        lines = [to_mm(seg) for seg in loop]
        for offset in options.get("stitch_rows", [9.0, 15.4]):
            stitches(lines, offset, shirt)
    if len(band):
        # The band's topstitch, on the shirt's side of its seam.
        stitches([to_mm(seg) for seg in band], float(options.get("band_stitch", 3.0)), shirt)
    rough = np.where(thread > 0.3, 0.55, rough)
    # Height to a tangent-space normal (glTF's: +X along U, +Y up the
    # image), by finite differences inside each island.
    H = np.zeros(size * size)
    H[vi] = height
    Hm = H.reshape(size, size)
    V = valid.reshape(size, size)
    right, left = np.roll(Hm, -1, axis=1), np.roll(Hm, 1, axis=1)
    rv, lv = np.roll(V, -1, axis=1), np.roll(V, 1, axis=1)
    gx = np.where(rv & lv, (right - left) / (2 * px_mm),
                  np.where(rv, (right - Hm) / px_mm, np.where(lv, (Hm - left) / px_mm, 0)))
    down, up = np.roll(Hm, -1, axis=0), np.roll(Hm, 1, axis=0)
    dv, uv_ = np.roll(V, -1, axis=0), np.roll(V, 1, axis=0)
    gy = np.where(dv & uv_, (down - up) / (2 * px_mm),
                  np.where(dv, (down - Hm) / px_mm, np.where(uv_, (Hm - up) / px_mm, 0)))
    nt = np.stack([-gx.reshape(-1)[vi], gy.reshape(-1)[vi], np.ones(len(vi))], axis=1)
    nt /= np.linalg.norm(nt, axis=1, keepdims=True)

    def image(values, channels):
        img = np.zeros((size * size, channels), dtype=np.float32)
        img[vi] = values
        return tx.dilate(img.reshape(size, size, channels), V)

    base = np.array(options.get("base", [0.80, 0.80, 0.79]))
    colour = np.clip(base[None, :] * albedo[:, None] * (0.55 + 0.45 * ao[:, None]), 0, 1)
    out = os.path.join(work, "textures")
    os.makedirs(out, exist_ok=True)
    name = options.get("name", "Fabric")

    def save(img, path, srgb):
        if srgb:
            img = np.where(img <= 0.0031308, img * 12.92, 1.055 * np.power(np.clip(img, 0, 1), 1 / 2.4) - 0.055)
        Image.fromarray(np.clip(img * 255 + 0.5, 0, 255).astype(np.uint8)).save(
            os.path.join(out, path), quality=92, subsampling=0)

    files = {"base_color": f"{name}_color.jpg", "normal": f"{name}_normal.jpg", "orm": f"{name}_orm.jpg"}
    save(image(colour, 3), files["base_color"], True)
    save(image(nt * 0.5 + 0.5, 3), files["normal"], False)
    save(image(np.stack([ao, rough, np.zeros_like(ao)], axis=1), 3), files["orm"], False)
    for m in mesh.materials:
        m.update(files)
    return {"size": size, "texels_per_cm": round(10 * scale, 1), "fabric": fabric, "tile_mm": tile_mm,
            "seam_segments": int(len(island)), "band_segments": int(len(band)),
            "hems_stitched": len(hemmed), "texels": int(valid.sum())}


# -- the step ----------------------------------------------------------------

def step(spec, work, source="conformed.npz", target="refined.npz"):
    options = spec.get("refine", {})
    mesh = WorkMesh.load(os.path.join(work, source))
    report = {}
    if "unwrap" in options:
        mesh, report["unwrap"] = unwrap(mesh, options["unwrap"])
    for old, new in options.get("rename", {}).items():
        for m in mesh.materials:
            if m["name"] == old:
                m["name"] = new
    if "materials" in options:
        report["materials"] = split_materials(mesh, options["materials"])
    hemmed = []
    panels = len(mesh.sizes)
    if "hems" in options:
        report["hems"], hemmed = hems(mesh, options["hems"])
    if "maps" in options:
        report["maps"] = bake(mesh, options["maps"], hemmed, panels, work)
    report["triangles"] = int(len(mesh.triangles()))
    mesh.save(os.path.join(work, target), report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--spec", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    with open(args.spec, encoding="utf-8") as f:
        spec = json.load(f)
    print(json.dumps(step(spec, args.work), indent=1))


if __name__ == "__main__":
    main()
