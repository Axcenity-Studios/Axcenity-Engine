"""Surface landmarks of a standing body, found the same way on any mesh:
the places a wrapped surface must not slide past (CO5; the spike's
correspondence was tight across the skin and 1 to 3 cm loose along it,
which put MakeHuman's pectoral edge beside our body's own and its navel on
smooth skin).

The joints pin a body's lengths; these pin its surface along the trunk
and the legs. Each is read off the body's depth as seen from straight in
front or behind (rays, so the mesh's own density doesn't matter), in the
pose the registration compares bodies in (arms clear of the trunk):

- `navel`: the deepest dent on the middle line of the belly.
- `notch`: the dent between the collarbones, on the middle line.
- `chest`: each side's most forward point of the chest (the pectoral's
  or the breast's peak); `underchest`: the crease below it (the
  pectoral's lower edge, the fold under the breast).
- `kneecap`: the most forward point over each knee.
- `buttock`: each side's rearmost point; `fold`: the crease under it.
- `calf`: the rearmost point of each lower leg.

`surface_landmarks` takes the joints' landmarks (`skeleton.py`) to know
where to look, and returns only the features it found clearly (`found`
says how clearly, for the log).
"""

import numpy as np

SIDES = (("l", 1), ("r", -1))
STEP = 0.004      # metres between rays
FRONT, BACK = -1.0, 1.0   # which way the surface faces along Y


class Depth:
    """A body's depth from the front or the back: `at(x, z)` is the Y of
    the first surface a ray from that side meets (None if it misses)."""

    def __init__(self, V, tris):
        import bpy  # noqa: F401 (the bpy module brings mathutils)
        from mathutils.bvhtree import BVHTree
        self.tree = BVHTree.FromPolygons([tuple(v) for v in V], [tuple(int(i) for i in t) for t in tris])

    def at(self, x, z, side):
        hit = self.tree.ray_cast((x, 2.0 * side, z), (0.0, -side, 0.0))[0]
        return None if hit is None else hit[1]

    def column(self, x, zs, side):
        """Depths down a vertical line, as how far the surface stands out
        toward the viewer (larger is nearer); NaN where the rays miss."""
        out = np.full(len(zs), np.nan)
        for i, z in enumerate(zs):
            y = self.at(x, z, side)
            if y is not None:
                out[i] = y * side
        return out


def _smooth(values, half):
    """A moving mean over `half` samples each way (NaN-safe)."""
    out = np.empty_like(values)
    for i in range(len(values)):
        out[i] = np.nanmean(values[max(0, i - half):i + half + 1])
    return out


def _dent(depth, half):
    """How far each sample lies under the smoothed line through it."""
    return _smooth(depth, half) - depth


def _peak(depth_map, xs, zs):
    """The grid point standing out most, as (x, z, value)."""
    i, j = np.unravel_index(np.nanargmax(depth_map), depth_map.shape)
    return xs[i], zs[j], depth_map[i, j]


def _grid(depth, xs, zs, side):
    return np.array([depth.column(x, zs, side) for x in xs])


def _crease(column, zs, top, low, high):
    """The sharpest inward bend of a vertical profile between `low` and
    `high` metres below sample `top`: (index, bend in metres per step squared)."""
    best, at = 0.0, None
    k = 4
    for i in range(k, len(zs) - k):
        drop = zs[top] - zs[i]
        if not low <= drop <= high or np.isnan(column[i - k:i + k + 1]).any():
            continue
        bend = column[i - k] + column[i + k] - 2 * column[i]
        if bend > best:
            best, at = bend, i
    return at, best


def surface_landmarks(V, tris, lm, shaped=None):
    """{name: position} of the features found on the body `V`, and
    {name: how clear, in millimetres} for the log. `lm` holds the joints'
    landmarks (crotch, neck or shoulders, hips, knees, ankles).

    `shaped` is the same mesh in a shape that shows the chest's lower
    edge better than `V` does (MakeHuman's most muscular body: its
    neutral's pectorals have next to no edge). The crease is then found
    on it and carried back to `V` by the vertex nearest to it."""
    depth = Depth(V, tris)
    sharp = Depth(shaped, tris) if shaped is not None else None
    out, found = {}, {}
    crotch = lm["crotch"][2]
    shoulders = (lm["l-shoulder"][2] + lm["r-shoulder"][2]) / 2
    trunk = shoulders - crotch
    half_width = abs(lm["l-shoulder"][0])
    point = lambda x, z, side: np.array([x, depth.at(x, z, side * 1.0) , z])  # noqa: E731

    # The middle line of the front: the navel and the notch between the collarbones.
    zs = np.arange(crotch + 0.12 * trunk, shoulders + 0.08 * trunk, STEP)[::-1]
    middle = depth.column(0.0, zs, FRONT) * 1.0
    dent = _dent(middle, int(0.035 / STEP))
    belly = (zs > crotch + 0.18 * trunk) & (zs < crotch + 0.50 * trunk)
    if np.isfinite(dent[belly]).any():
        i = np.flatnonzero(belly)[np.nanargmax(dent[belly])]
        out["navel"], found["navel"] = point(0.0, zs[i], FRONT), dent[i] * 1000
    top = (zs > crotch + 0.88 * trunk) & (zs < shoulders + 0.06 * trunk)
    if np.isfinite(dent[top]).any():
        i = np.flatnonzero(top)[np.nanargmax(dent[top])]
        out["notch"], found["notch"] = point(0.0, zs[i], FRONT), dent[i] * 1000

    for side, s in SIDES:
        # The chest's peak and the crease under it.
        xs = np.arange(0.25, 0.80, 0.03) * half_width * s
        zs = np.arange(crotch + 0.50 * trunk, crotch + 0.90 * trunk, STEP)[::-1]
        chest = _grid(depth, xs, zs, FRONT)
        x, z, _ = _peak(chest, xs, zs)
        out[f"{side}-chest"] = point(x, z, FRONT)
        column = depth.column(x, zs, FRONT)
        top = int(np.argmin(np.abs(zs - z)))
        i, bend = _crease(column, zs, top, 0.02, 0.13)
        found[f"{side}-chest"] = (np.nanmax(chest) - np.nanmedian(chest)) * 1000
        if i is not None:
            out[f"{side}-underchest"], found[f"{side}-underchest"] = point(x, zs[i], FRONT), bend * 1000
        if sharp is not None:
            j, bend2 = _crease(sharp.column(x, zs, FRONT), zs, top, 0.02, 0.13)
            if j is not None and bend2 * 1000 > found.get(f"{side}-underchest", 0.0):
                at = np.array([x, sharp.at(x, zs[j], FRONT), zs[j]])
                nearest = int(np.argmin(((shaped - at) ** 2).sum(axis=1)))
                out[f"{side}-underchest"], found[f"{side}-underchest"] = V[nearest].copy(), bend2 * 1000

        # The kneecap: the front of the leg over the knee joint.
        knee, hip, ankle = lm[f"{side}-knee"], lm[f"{side}-hip"], lm[f"{side}-ankle"]
        zs = np.arange(knee[2] - 0.07, knee[2] + 0.07, STEP)[::-1]
        xs = knee[0] + np.arange(-0.02, 0.021, 0.01)
        cap = _grid(depth, xs, zs, FRONT)
        # Against the leg's own lean: the line from the top to the bottom of the window.
        lean = np.linspace(np.nanmean(cap[:, :3]), np.nanmean(cap[:, -3:]), len(zs))
        x, z, value = _peak(cap - lean, xs, zs)
        out[f"{side}-kneecap"], found[f"{side}-kneecap"] = point(x, z, FRONT), value * 1000

        # The buttock's rearmost point and the fold under it.
        zs = np.arange(crotch - 0.10 * trunk, crotch + 0.30 * trunk, STEP)[::-1]
        xs = np.arange(0.3, 1.31, 0.1) * hip[0]
        rear = _grid(depth, xs, zs, BACK)
        x, z, _ = _peak(rear, xs, zs)
        out[f"{side}-buttock"] = point(x, z, BACK)
        found[f"{side}-buttock"] = (np.nanmax(rear) - np.nanmedian(rear)) * 1000
        column = depth.column(x, zs, BACK)
        top = int(np.argmin(np.abs(zs - z)))
        i, bend = _crease(column, zs, top, 0.03, 0.16)
        if i is not None:
            out[f"{side}-fold"], found[f"{side}-fold"] = point(x, zs[i], BACK), bend * 1000

        # The calf's rearmost point, along the line from knee to ankle.
        ts = np.arange(0.12, 0.6, 0.02)
        best = None
        for t in ts:
            at = knee + (ankle - knee) * t
            for dx in (-0.01, 0.0, 0.01):
                y = depth.at(at[0] + dx, at[2], BACK)
                if y is not None and (best is None or y - at[1] > best[0]):
                    best = (y - at[1], np.array([at[0] + dx, y, at[2]]))
        if best is not None:
            out[f"{side}-calf"], found[f"{side}-calf"] = best[1], best[0] * 1000
    return out, found


def keys(landmarks):
    """The feature names in a landmark dictionary, in a fixed order."""
    names = ["navel", "notch"] + [f"{side}-{n}" for side, _ in SIDES
                                  for n in ("chest", "underchest", "kneecap", "buttock", "fold", "calf")]
    return [n for n in names if n in landmarks]


# The features the registration pins, and how clear (millimetres) each
# must be on both bodies. The chest's peak and the kneecap aren't used:
# a flat chest has no one peak, and the kneecap's bump is lost in the
# thigh's on a muscular leg.
#
# None by default. Tried on MakeHuman's male and female neutrals, which
# share a topology (so the answer is known): the depth features land 14
# to 38 mm from the vertex they should (buttock 14, chest 20, notch 26,
# crease under the chest 27, fold under the buttock 32, calf 38), no
# better than the slide they were meant to stop, and pinning them left
# the trunk's error where it was (13.3 mm before, 13.8 after). Only the
# marks read from the topology and the sculpt (`marks.py`: navel and
# nipples, 0.0 and 1.4 mm in the same test) are worth pinning.
# `{"underchest": 3.0, ...}` turns a feature on for an experiment.
PINNED = {}


def pairs(a, clear_a, b, clear_b, marks_a=None, marks_b=None, log=None):
    """The features found clearly on two bodies, as (names, positions on
    the first, positions on the second), each made symmetric (a left and
    right pair shares its height, depth and distance from the middle).
    `marks_*` are features known some other way (`marks.py`: the navel
    and the nipples), which are always used."""
    names = []
    for key in keys(a):
        need = PINNED.get(key.split("-")[-1])
        if need is None or key not in b or min(clear_a[key], clear_b[key]) < need:
            continue
        names.append(key)
    a, b = dict(a), dict(b)
    if marks_a and marks_b:
        for key in sorted(marks_a):
            a[key], b[key] = marks_a[key], marks_b[key]
            names.append(key)
    # Both sides or neither, then symmetric.
    names = [k for k in names if k[:2] not in ("l-", "r-") or {"l": "r", "r": "l"}[k[0]] + k[1:] in names]
    for lm in (a, b):
        for key in names:
            if key.startswith("l-"):
                left, right = lm[key], lm["r" + key[1:]]
                mean = (left + right * np.array([-1.0, 1.0, 1.0])) / 2
                lm[key], lm["r" + key[1:]] = mean, mean * np.array([-1.0, 1.0, 1.0])
            elif key[:2] != "r-":
                lm[key] = lm[key] * np.array([0.0, 1.0, 1.0])
    if log:
        for key in names:
            if key[:2] != "r-":
                log(f"    pinned {key:13s} moves {np.linalg.norm(a[key] - b[key]) * 1000:5.1f} mm")
    return names, np.array([a[k] for k in names]), np.array([b[k] for k in names])
