"""From a draped garment to a finished mesh at one level of detail.

1. **Carry.** Each piece is gridded again at the level's spacing; every
   corner is found in the fine (simulated) grid by its place in the
   pattern and takes the drape's position there (`Panel.locate`,
   `carry`). Seams stay closed: both pieces' corners on a seam sit at the
   same lengths along it, which the fine grid joined.
2. **Edges.** No raw edge is left showing (`asset-quality.md` §3): a
   **turned hem** folds the edge inward with a rounded fold `THICK` thick
   and lies back up inside the shell for its depth; a **band** (a rib neck
   band, a waistband, a cuff) stands past the edge, folds over and comes
   back down inside past the seam. The raw edge of either lies `THICK`
   inside the shell.
3. **Texture coordinates.** The pattern itself, in millimetres, each piece
   an island (a hem's turn-up continues its piece below the hem line; a
   band is a strip of its own), packed into one square atlas.

The mesh comes back as corners (positions, pattern coordinates, piece),
quads in a material each, and seam pairs, for `textures.py` and the
export (`write_obj`).
"""

import math

import numpy as np

from panels import Panel

# How thick a fold is (two layers of jersey or fleece, rounded): the raw
# edge lies this far inside the shell.
THICK = 0.003
# Margin round each island in the atlas (mm of pattern).
ISLAND_GAP = 14.0


class Mesh:
    def __init__(self):
        self.P = []        # positions (m)
        self.uv = []       # pattern coordinates (mm), per island
        self.island = []   # island name per corner
        self.quads = {}    # material -> list of quads
        self.welds = []    # pairs of corners at one place (seams)
        self.kind = []     # "shell", "facing", "band" per corner

    def add(self, p, uv, island, kind):
        self.P.append(np.asarray(p, dtype=float))
        self.uv.append(np.asarray(uv, dtype=float))
        self.island.append(island)
        self.kind.append(kind)
        return len(self.P) - 1

    def quad(self, material, q):
        self.quads.setdefault(material, []).append(tuple(q))

    def arrays(self):
        self.P = np.array(self.P)
        self.uv = np.array(self.uv)
        return self


def carry(fine, draped, coarse):
    """`coarse` piece corners placed on the `fine` piece's drape."""
    located = fine.locate(coarse.corners())
    return fine.carry(located, draped)


def normals(P, quads, welds):
    """Corner normals over the whole garment, seams welded (area-weighted
    over each quad's two triangles)."""
    N = np.zeros_like(P)
    for q in quads:
        a, b, c, d = q
        n = np.cross(P[c] - P[a], P[d] - P[b])
        for k in q:
            N[k] += n
    parent = list(range(len(P)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for a, b in welds:
        parent[find(a)] = find(b)
    groups = {}
    for k in range(len(P)):
        groups.setdefault(find(k), []).append(k)
    for g in groups.values():
        if len(g) > 1:
            N[g] = N[g].sum(axis=0)
    return N / np.maximum(np.linalg.norm(N, axis=1, keepdims=True), 1e-12)


def snap(mesh):
    """Corners welded at a seam are put at exactly one place (their
    middle): the two pieces' corners agree only to a fraction of a
    micrometre after the carry, and the import welds corners by place."""
    parent = list(range(len(mesh.P)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for a, b in mesh.welds:
        parent[find(a)] = find(b)
    groups = {}
    for k in range(len(mesh.P)):
        groups.setdefault(find(k), []).append(k)
    for g in groups.values():
        if len(g) > 1:
            mid = np.mean([mesh.P[k] for k in g], axis=0)
            for k in g:
                mesh.P[k] = mid.copy()


def column_point(grid3, gridmm, row_from, step, col, distance):
    """The point `distance` (m) along a piece's column `col`, from row
    `row_from` inward by `step` (+1 or -1) rows, and its pattern place."""
    left = distance
    r = row_from
    while True:
        nr = r + step
        if nr < 0 or nr >= grid3.shape[0]:
            return grid3[r, col], gridmm[r, col]
        seg = np.linalg.norm(grid3[nr, col] - grid3[r, col])
        if seg >= left:
            f = left / max(seg, 1e-9)
            return (grid3[r, col] + (grid3[nr, col] - grid3[r, col]) * f,
                    gridmm[r, col] + (gridmm[nr, col] - gridmm[r, col]) * f)
        left -= seg
        r = nr


class Garment:
    """A garment at one level: its pieces (gridded at `spacing` mm),
    placed on the drape of the `fine` pieces, finished by `spec`.

    `spec`: `hems` [(piece, segment, depth m)], `bands` [{runs: [(piece,
    segment, reversed)], height (m), inside (m), material, name}],
    `material` (the shell's), `seams` (the garment module's seam list),
    `flip` {piece: bool} (quads that face in when made)."""

    def __init__(self, pieces_fn, m, spacing, fine, draped_by_piece, spec):
        self.pieces, self.seam_list = pieces_fn(m, spacing)[:2]
        self.spec = spec
        self.grid3 = {}
        for name, piece in self.pieces.items():
            pos = carry(fine[name], draped_by_piece[name], piece)
            self.grid3[name] = pos.reshape(piece.rows, piece.cols, 3)
        self.build()

    def build(self):
        spec = self.spec
        mesh = Mesh()
        index = {}
        for name, piece in self.pieces.items():
            idx = np.empty((piece.rows, piece.cols), dtype=np.int64)
            for r in range(piece.rows):
                for c in range(piece.cols):
                    idx[r, c] = mesh.add(self.grid3[name][r, c], piece.uv[r, c], name, "shell")
            index[name] = idx
            for r in range(piece.rows - 1):
                for c in range(piece.cols - 1):
                    q = (idx[r, c], idx[r, c + 1], idx[r + 1, c + 1], idx[r + 1, c])
                    mesh.quad(spec["material"], q[::-1] if spec["flip"][name] else q)
        # Seams: corners at one place.
        for pa, sa, pb, sb, rev in self.seam_list:
            a = [index[pa][r, c] for r, c in self.pieces[pa].segments[sa]]
            b = [index[pb][r, c] for r, c in self.pieces[pb].segments[sb]]
            if rev:
                b = b[::-1]
            mesh.welds.extend(zip(a, b))
        snap(mesh)
        P = np.array(mesh.P)
        all_quads = [q for qs in mesh.quads.values() for q in qs]
        N = normals(P, all_quads, mesh.welds)
        self.mesh, self.index, self.N = mesh, index, N
        self._ends = []
        self.band_columns = []
        for piece, segment, depth in spec.get("hems", []):
            self.hem(piece, segment, depth)
        self._join_ends()
        for band in spec.get("bands", []):
            self.band(band)
        snap(mesh)
        mesh.arrays()

    def _run(self, piece, segment):
        """A run of corners on a piece's top or bottom row: their indices,
        positions, the direction past the edge (the surface carried on) and
        which way rows go inward."""
        p = self.pieces[piece]
        rc = p.segments[segment]
        rows = {r for r, _ in rc}
        assert len(rows) == 1, f"{piece}.{segment} isn't a row"
        r = rows.pop()
        inward = 1 if r == 0 else -1
        g = self.grid3[piece]
        out = []
        for _, c in rc:
            edge = g[r, c]
            inner = g[r + inward, c]
            past = edge - inner
            past /= max(np.linalg.norm(past), 1e-9)
            out.append((c, self.index[piece][r, c], edge, past))
        return r, inward, out

    def hem(self, piece, segment, depth):
        """A turned hem: a rounded fold at the edge, the turn-up lying
        `THICK` inside the shell for `depth` (m) back up the piece."""
        mesh, spec = self.mesh, self.spec
        r, inward, run = self._run(piece, segment)
        p = self.pieces[piece]
        flip = spec["flip"][piece]
        rows = []
        for c, k, edge, past in run:
            n = self.N[k]
            # Remove any part of `past` along the normal.
            past = past - n * (past @ n)
            past /= max(np.linalg.norm(past), 1e-9)
            uv_edge = p.uv[r, c]
            down = np.array([0.0, 1.0 if inward > 0 else -1.0])
            inner, inner_uv = column_point(self.grid3[piece], p.uv, r, inward, c, depth)
            round_ = mesh.add(edge + past * THICK * 0.5 - n * THICK * 0.5, uv_edge + down * THICK * 800, piece, "facing")
            fold = mesh.add(edge - n * THICK, uv_edge + down * THICK * 1600, piece, "facing")
            top = mesh.add(inner - n * THICK, uv_edge + down * (THICK * 1600 + depth * 1000), piece, "facing")
            rows.append((k, round_, fold, top))
        for a, b in zip(rows, rows[1:]):
            for j in range(3):
                # Rows going out from the edge: the edge is row 0, the
                # round row -1 and so on (for a bottom hem), so the quads
                # keep the piece's own turn.
                lo_a, lo_b, hi_a, hi_b = a[j + 1], b[j + 1], a[j], b[j]
                q = (lo_a, lo_b, hi_b, hi_a) if inward > 0 else (hi_a, hi_b, lo_b, lo_a)
                mesh.quad(spec["material"], q[::-1] if flip else q)
        self._weld_ends(rows, piece, segment)

    def _weld_ends(self, rows, piece, segment):
        """Keeps the hem's first and last columns: where two hems meet at
        a seam (a front and a back at the side seam, a sleeve's two ends
        at its underarm seam) they are made one in `_join_ends`."""
        self._ends.append(rows[0])
        self._ends.append(rows[-1])

    def _join_ends(self):
        """Hem columns whose edge corners are sewn together are moved to
        their middle and welded, so no crack opens at a seam."""
        parent = {}

        def find(a):
            parent.setdefault(a, a)
            while parent[a] != a:
                parent[a] = parent[parent[a]]
                a = parent[a]
            return a

        for a, b in self.mesh.welds:
            parent[find(a)] = find(b)
        groups = {}
        for col in self._ends:
            groups.setdefault(find(col[0]), []).append(col)
        for cols in groups.values():
            if len(cols) < 2:
                continue
            for j in range(1, len(cols[0])):
                ks = [c[j] for c in cols]
                mid = np.mean([self.mesh.P[k] for k in ks], axis=0)
                for k in ks:
                    self.mesh.P[k] = mid
                for k in ks[1:]:
                    self.mesh.welds.append((ks[0], k))

    def band(self, band):
        """A band round `runs` (a closed ring when its ends meet): it stands
        `height` past the edge, folds over `THICK` thick and comes back
        down inside, `inside` past the seam."""
        mesh, spec = self.mesh, self.spec
        corners = []
        for piece, segment, rev in band["runs"]:
            r, inward, run = self._run(piece, segment)
            run = run[::-1] if rev else run
            for c, k, edge, past in run:
                if corners and np.linalg.norm(corners[-1][1] - edge) < 1e-6:
                    continue
                n = self.N[k]
                inner, _ = column_point(self.grid3[piece], self.pieces[piece].uv, r, inward, c, band["inside"])
                corners.append((k, edge, past, n, inner, piece))
        closed = np.linalg.norm(corners[0][1] - corners[-1][1]) < 1e-6
        if closed:
            corners = corners[:-1]
        h, t = band["height"], THICK
        arc = 0.0
        ring = []
        for i, (k, edge, past, n, inner, piece) in enumerate(corners):
            if i:
                arc += np.linalg.norm(edge - corners[i - 1][1]) * 1000
            if "direction" in band:
                # A band that stands straight (a waistband up, a cuff down),
                # whatever the gathered edge's own slope.
                past = np.asarray(band["direction"], dtype=float)
            past = past - n * (past @ n)
            past /= max(np.linalg.norm(past), 1e-9)
            out = edge + past * h
            v = [0.0, h * 1000, h * 1000 + t * 800, h * 1000 + t * 1600, (2 * h + band["inside"]) * 1000 + t * 1600]
            pts = [edge, out, out + past * t * 0.5 - n * t * 0.5, out - n * t, inner - n * t]
            ids = [mesh.add(p, (arc, vv), band["name"], "band") for p, vv in zip(pts, v)]
            mesh.welds.append((k, ids[0]))
            ring.append(ids)
        if closed:
            # The ring's last column meets its first at the same place; a
            # copy keeps the strip's texture coordinates running on.
            first = corners[0]
            arc += np.linalg.norm(first[1] - corners[-1][1]) * 1000
            ids = []
            for j, k in enumerate(ring[0]):
                ids.append(mesh.add(mesh.P[k], (arc, mesh.uv[k][1]), band["name"], "band"))
                mesh.welds.append((k, ids[-1]))
            ring.append(ids)
        # Each row keeps the first row's lengths round (a rectangle): a
        # band that leans in or out is shorter or longer round than its
        # seam, and taking each row's own length instead shears the strip
        # (a ring cut at one seam can't lie flat otherwise).
        self.band_columns.extend(ring)
        # The turn: as the band's first run's piece goes (a top edge's
        # band runs on up the piece, so quads as the piece's own rows).
        piece0 = band["runs"][0][0]
        _, inward0, _ = self._run(piece0, band["runs"][0][1])
        flip = self.spec["flip"][piece0] ^ band["runs"][0][2]
        for a, b in zip(ring, ring[1:]):
            for j in range(4):
                q = (a[j], b[j], b[j + 1], a[j + 1]) if inward0 < 0 else (a[j + 1], b[j + 1], b[j], a[j])
                mesh.quad(band["material"], q[::-1] if flip else q)


def clear_bands(garment, nearest, margin=0.004):
    """Moves each column of each band out of the body as one (its fold
    keeps its thickness) until every corner but the one on the seam is
    `margin` over the skin: a band standing straight up from a waist or
    down from an ankle runs into a belly or a heel. `nearest(p)` gives the
    skin's closest point and normal. Returns the most a column moved (m)."""
    mesh = garment.mesh
    most = 0.0
    for column in garment.band_columns:
        need, normal = 0.0, None
        for k in column[1:]:
            q, n = nearest(np.asarray(mesh.P[k]))
            gap = (np.asarray(mesh.P[k]) - q) @ n
            if margin - gap > need:
                need, normal = margin - gap, n
        if normal is not None:
            for k in column[1:]:
                mesh.P[k] = np.asarray(mesh.P[k]) + normal * need
            most = max(most, need)
    snap(mesh)
    return most


def pack(mesh, size_px=2048, margin_px=8):
    """Texture coordinates (0..1, v down) from each island's pattern
    millimetres, packed in shelves (tallest first) into a square; one
    scale for all, so every piece has the same texel density. Returns the
    coordinates and the scale (texels per mm) and each island's offset."""
    islands = {}
    for k, name in enumerate(mesh.island):
        islands.setdefault(name, []).append(k)
    boxes = {}
    for name, ks in islands.items():
        uv = mesh.uv[ks]
        boxes[name] = (uv.min(axis=0) - ISLAND_GAP / 2, uv.max(axis=0) + ISLAND_GAP / 2)

    def layout(width):
        order = sorted(boxes, key=lambda n: -(boxes[n][1][1] - boxes[n][0][1]))
        x = y = shelf = 0.0
        place = {}
        for name in order:
            w, h = boxes[name][1] - boxes[name][0]
            if x + w > width:
                x, y, shelf = 0.0, y + shelf, 0.0
            place[name] = (x, y)
            x += w
            shelf = max(shelf, h)
        return place, y + shelf

    widest = max(b[1][0] - b[0][0] for b in boxes.values())
    total = sum(np.prod(b[1] - b[0]) for b in boxes.values())
    best = None
    for width in np.linspace(max(widest, math.sqrt(total)), 3 * math.sqrt(total), 60):
        place, height = layout(width)
        side = max(width, height)
        if best is None or side < best[0]:
            best = (side, place)
    side, place = best
    scale = (size_px - 2 * margin_px) / side
    out = np.zeros((len(mesh.uv), 2))
    offsets = {}
    for name, ks in islands.items():
        x, y = place[name]
        lo = boxes[name][0]
        offsets[name] = (x - lo[0], y - lo[1])
        out[ks] = ((mesh.uv[ks] - lo + [x, y]) * scale + margin_px) / size_px
    return out, scale, offsets, margin_px


def write_obj(path, mesh, uv, materials):
    """An OBJ of the garment for the import, in glTF's axes (Y up, the
    body facing +Z): Blender's (x, y, z) is (x, z, -y). Triangles, with
    texture coordinates (OBJ's v up) and normals."""
    P = mesh.P
    welds = mesh.welds
    all_quads = [q for qs in mesh.quads.values() for q in qs]
    N = normals(P, all_quads, welds)
    with open(path, "w", newline="\n") as f:
        f.write("# A garment of the Axcenity Engine's realistic wardrobe (assets/characters/realistic/clothing)\n")
        f.write(f"mtllib {materials}\n")
        for p in P:
            f.write(f"v {p[0]:.6f} {p[2]:.6f} {-p[1]:.6f}\n")
        for t in uv:
            f.write(f"vt {t[0]:.6f} {1 - t[1]:.6f}\n")
        for n in N:
            f.write(f"vn {n[0]:.5f} {n[2]:.5f} {-n[1]:.5f}\n")
        for material, quads in mesh.quads.items():
            f.write(f"usemtl {material}\n")
            for a, b, c, d in quads:
                for tri in ((a, b, c), (a, c, d)):
                    f.write("f " + " ".join(f"{k + 1}/{k + 1}/{k + 1}" for k in tri) + "\n")


def triangles(mesh):
    return sum(2 * len(q) for q in mesh.quads.values())
