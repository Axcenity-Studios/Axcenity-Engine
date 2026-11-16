"""The body's shapes (CO5): muscle and fat, whole and per region, on both
frames, from MakeHuman's CC0 targets carried onto this topology
(`../transfer/`, vendored as `source/body_shapes.npz`).

**The grid.** MakeHuman sculpted nine bodies per gender: muscle (min,
average, max) by weight (min, average, max). Between them it mixes
bilinearly, so its bodies are a function `F(muscle, weight)` on a square
from -1 to 1, and each of its sculpted corners is a point of it. The
sliders here walk a part of that square, and the body they make is always
`base + F(m, w) - F(m0, w0)`:

- `PLACE[frame][axis] = (at, low, high)`: where the frame's own body sits
  on MakeHuman's axis (`at`), and the part of the axis the slider reaches
  (-1 gives `low`, 0 gives `at`, 1 gives `high`). The bundle's masculine
  body is already athletic, so it doesn't sit at MakeHuman's average: its
  slim and soft ends take off the muscle and mass it starts with. The
  ends are cut inside MakeHuman's square where its bodies turn into
  caricatures (see the package README for what was cut and why).
- `fat` walks MakeHuman's weight axis: with little muscle it is fat, with
  much it is bulk. `muscle` walks its muscle axis: tone and definition.

**As the manifest's corners.** `F` is linear in four monotone pieces per
axis (how far the slider is past MakeHuman's average either way, taken
from where the base sits), so the body is eight shapes per frame: four
driven by one slider and four by the product of two, each also times its
frame's share (`manifest_entries`; architecture.md section 7).

**Per region.** `fat_<region>` and `muscle_<region>` (chest, shoulders,
arms, abdomen, back, hips, thighs, calves) add or take off up to `LOCAL`
of the step to the sliders' heavy corner inside a smooth region: for fat
the heavy and soft body (muscle -1, fat 1), for muscle the heavy and
muscular one (both 1). The regions come from the skin's bone weights and
add up to one over the body.

**Fat keeps off bone** (`bony`): the skull, collarbones, elbows, wrists,
kneecaps, the shins' front, ankles, hands and feet hold back the fat
shapes. **The eyes and mouth are held** (`face_hold`): no shape moves the
lids, the sockets, the lips' edges or the mouth's inside, so the eyes and
teeth stay fitted. **Kept apart**: the heaviest combinations are checked
in the rest pose and every shape of the frame is scaled down, smoothly,
round the places where they made the body cross itself.
"""

import os
import sys

import numpy as np

from common import (
    FS_FINGERS, FS_FOOT, FS_HEAD, FS_MOUTH_LOWER, FS_MOUTH_UPPER, FS_MUZZLE, FS_PALM, FS_SHIN, FS_SOCKET, FS_TOES,
    FS_TOE_BASE, SIDES, SOURCE, array,
)

HERE = os.path.dirname(os.path.abspath(__file__))
TRANSFER = os.path.join(os.path.dirname(HERE), "transfer")
PACK = os.path.join(SOURCE, "body_shapes.npz")

FRAMES = ("Masculine", "Feminine")
AXES = ("muscle", "fat")
MH_AXIS = {"muscle": "muscle", "fat": "weight"}
# (where the frame's body sits, what -1 reaches, what +1 reaches), on MakeHuman's axes.
PLACE = {
    "Masculine": {"muscle": (0.45, -0.6, 1.0), "fat": (0.30, -0.75, 0.85)},
    "Feminine": {"muscle": (0.0, -0.7, 0.9), "fat": (0.0, -0.75, 0.85)},
}
REGIONS = ("chest", "shoulders", "arms", "abdomen", "back", "hips", "thighs", "calves")
LOCAL = 0.6       # a region's slider at 1, as a share of the step to the sliders' heavy corner
HOLD = 0.85       # how much of a fat shape the bony places hold back
STEP = 2e-5       # the packed file's step, metres


def _transfer():
    if TRANSFER not in sys.path:
        sys.path.insert(0, TRANSFER)
    import geom
    import transfer
    return geom, transfer


# -- MakeHuman's grid -------------------------------------------------------------------

class Grid:
    """One frame's nine bodies as offsets from MakeHuman's average, in
    the rest pose: `A[s]`, `B[s]` (muscle and weight alone, s = +1 or
    -1) and `I[(sm, sw)]` (what the corner adds to the two)."""

    def __init__(self, corners):
        name = {1: "max", -1: "min"}
        c = lambda m, w: corners[f"{m}muscle_{w}weight"]  # noqa: E731
        self.A = {s: c(name[s], "average") for s in (1, -1)}
        self.B = {s: c("average", name[s]) for s in (1, -1)}
        self.I = {(a, b): c(name[a], name[b]) - self.A[a] - self.B[b] for a in (1, -1) for b in (1, -1)}

    def at(self, m, w):
        sm, sw = (1 if m >= 0 else -1), (1 if w >= 0 else -1)
        return abs(m) * self.A[sm] + abs(w) * self.B[sw] + abs(m) * abs(w) * self.I[(sm, sw)]


def pieces(place):
    """An axis's monotone pieces for a frame placed at `place`: a list of
    (key, from, to), where key is `up` (how far past MakeHuman's average
    upward, less where the base sits) or `down` (how far past it
    downward), `from` the slider's span and `to` the piece's."""
    at, low, high = place
    out = [("up", [0.0, 1.0], [0.0, high - at])]
    if at > 0:
        # Below the base the slider first takes off what the base has, down to
        # MakeHuman's average at `knee`, then goes on down MakeHuman's lower half.
        knee = -at / (at - low)
        out.append(("up", [knee, 0.0], [-at, 0.0]))
        if low < 0:
            out.append(("down", [knee, -1.0], [0.0, -low]))
    else:
        out.append(("down", [0.0, -1.0], [0.0, -low]))
    return out


def span(value, source, to):
    t = min(1.0, max(0.0, (value - source[0]) / (source[1] - source[0])))
    return to[0] + (to[1] - to[0]) * t


def engine_shapes(grid, frame):
    """{shape name: offsets} for one frame: the four single-slider shapes
    and the four products (module docstring)."""
    m0, w0 = PLACE[frame]["muscle"][0], PLACE[frame]["fat"][0]
    assert m0 >= 0 and w0 >= 0, "a base under MakeHuman's average isn't handled"
    out = {
        f"{frame}_Muscle_Up": grid.A[1] + w0 * grid.I[(1, 1)],
        f"{frame}_Muscle_Down": grid.A[-1] + w0 * grid.I[(-1, 1)],
        f"{frame}_Fat_Up": grid.B[1] + m0 * grid.I[(1, 1)],
        f"{frame}_Fat_Down": grid.B[-1] + m0 * grid.I[(1, -1)],
    }
    way = {1: "Up", -1: "Down"}
    for (a, b), offsets in grid.I.items():
        out[f"{frame}_Muscle_{way[a]}_Fat_{way[b]}"] = offsets
    return out


def frame_factor(frame):
    return {"parameter": "frame", "from": [0, 1], "to": [1, 0] if frame == "Masculine" else [0, 1]}


def corners():
    """The manifest's `corners` for the whole-body shapes."""
    out = []
    way = {"up": "Up", "down": "Down"}
    for frame in FRAMES:
        P = {axis: pieces(PLACE[frame][axis]) for axis in AXES}
        for axis, title in (("muscle", "Muscle"), ("fat", "Fat")):
            for key, source, to in P[axis]:
                out.append({"shape": f"{frame}_{title}_{way[key]}",
                            "factors": [frame_factor(frame), {"parameter": axis, "from": source, "to": to}]})
        for km, sm, tm in P["muscle"]:
            for kf, sf, tf in P["fat"]:
                out.append({"shape": f"{frame}_Muscle_{way[km]}_Fat_{way[kf]}",
                            "factors": [frame_factor(frame), {"parameter": "muscle", "from": sm, "to": tm},
                                        {"parameter": "fat", "from": sf, "to": tf}]})
        for region in REGIONS:
            for title in ("Muscle", "Fat"):
                out.append({"shape": f"{frame}_{title}_{region.capitalize()}",
                            "factors": [frame_factor(frame),
                                        {"parameter": f"{title.lower()}_{region}", "from": [-1, 1], "to": [-LOCAL, LOCAL]}]})
    return out


def parameters():
    """The manifest's parameters for the body's shapes (they drive
    nothing themselves: every shape is a corner with the frame)."""
    out = [
        {"id": "muscle", "label": "Muscle", "group": "build", "range": [-1, 1]},
        {"id": "fat", "label": "Body fat", "group": "build", "range": [-1, 1]},
    ]
    for region in REGIONS:
        for title in ("Muscle", "Fat"):
            out.append({"id": f"{title.lower()}_{region}", "label": f"{region.capitalize()} {title.lower()}",
                        "group": "build_regions", "range": [-1, 1], "advanced": True})
    return out


def weights_of(values, frame_value):
    """{shape: weight} for slider `values` ({parameter: value}) at a frame
    value, by the manifest's rule (what `avatar::resolve` does)."""
    out = {}
    for corner in corners():
        w = 1.0
        for factor in corner["factors"]:
            value = frame_value if factor["parameter"] == "frame" else values.get(factor["parameter"], 0.0)
            w *= span(value, factor.get("from", [0, 1]), factor.get("to", factor.get("from", [0, 1])))
        out[corner["shape"]] = out.get(corner["shape"], 0.0) + w
    return out


# -- regions, bone, the face ------------------------------------------------------------

def region_weights(weights, normals, lap):
    """{region: per-vertex share}, smooth, from the skin's bone weights
    (and, on the trunk, which way the skin faces). They add up to one
    with what is left over (the head, neck, hands and feet)."""
    col = lambda *names: sum(weights.col(side + n) for n in names for side in ("Left", "Right"))  # noqa: E731
    front = np.clip((-normals[:, 1] + 0.25) / 0.6, 0.0, 1.0)
    spine, chest, upper = weights.col("Spine"), weights.col("Chest"), weights.col("UpperChest")
    out = {
        "chest": (upper + 0.5 * chest) * front,
        "abdomen": (spine + 0.5 * chest) * front,
        "back": (spine + chest + upper) * (1.0 - front),
        "shoulders": col("Shoulder", "ShoulderHelper"),
        "arms": col("UpperArm", "UpperArmTwist", "LowerArm", "LowerArmTwist", "ElbowHelper"),
        "hips": weights.col("Hips") + 0.5 * col("HipHelper"),
        "thighs": col("UpperLeg", "UpperLegTwist") + 0.5 * col("HipHelper", "KneeHelper"),
        "calves": col("LowerLeg", "LowerLegTwist") + 0.5 * col("KneeHelper"),
    }
    for name, value in out.items():
        v = value[:, None]
        for _ in range(3):
            v = (v + lap.mean(v)) / 2
        out[name] = v[:, 0]
    total = sum(out.values())
    over = np.maximum(total, 1.0)
    return {name: value / over for name, value in out.items()}


def _near(X, point, radius):
    return np.exp(-((X - np.asarray(point)) ** 2).sum(axis=1) / (radius * radius))


def bony(P, lm, joints, eyes, normals, lap):
    """Per vertex, 0 to 1: how near the bone lies under the skin, where a
    fat body keeps its shape (module docstring)."""
    X = array(P)
    k = (X[:, 2].max() - X[:, 2].min()) / 1.75
    of = lambda *sets: np.array(sorted({i for fs in sets for i in lm.of_set[fs]}), dtype=np.int64)  # noqa: E731
    J = {name: np.array(j[0]) for name, j in joints.items()}
    m = np.zeros(len(X))
    # The skull: the head above the eyes.
    head = of(FS_HEAD)
    eye_z = eyes[1][2]
    m[head] = np.clip((X[head][:, 2] - (eye_z + 0.012 * k)) / (0.02 * k), 0.0, 1.0)
    for _, s in SIDES:
        side = "Left" if s > 0 else "Right"
        m[of(FS_PALM[s], FS_FOOT[s], FS_TOE_BASE[s], *FS_TOES[s], *[fs for g in FS_FINGERS[s].values() for fs in g])] = 1.0
        # Collarbone: from the pit of the neck out to the shoulder, on the front and top.
        inner = J["Neck"] + np.array([s * 0.02 * k, -0.045 * k, -0.035 * k])
        outer = J[side + "UpperArm"] + np.array([-s * 0.02 * k, -0.02 * k, 0.035 * k])
        t = np.clip(((X - inner) @ (outer - inner)) / ((outer - inner) @ (outer - inner)), 0.0, 1.0)
        away = np.linalg.norm(X - (inner + t[:, None] * (outer - inner)), axis=1)
        m = np.maximum(m, 0.9 * np.exp(-(away / (0.022 * k)) ** 2))
        # Elbow's point, wrist, kneecap, ankle.
        m = np.maximum(m, _near(X, J[side + "LowerArm"] + np.array([0.0, 0.03 * k, 0.0]), 0.035 * k))
        m = np.maximum(m, _near(X, J[side + "Hand"], 0.04 * k))
        m = np.maximum(m, _near(X, J[side + "LowerLeg"] + np.array([0.0, -0.045 * k, 0.005 * k]), 0.045 * k))
        m = np.maximum(m, _near(X, J[side + "Foot"], 0.05 * k))
        # The shin's front: the skin of the lower leg that faces forward.
        shin = of(FS_SHIN[s])
        knee, ankle = J[side + "LowerLeg"], J[side + "Foot"]
        along = np.clip((knee[2] - X[shin][:, 2]) / max(knee[2] - ankle[2], 1e-6), 0.0, 1.0)
        facing = np.clip((-normals[shin][:, 1] - 0.55) / 0.3, 0.0, 1.0)
        m[shin] = np.maximum(m[shin], facing * np.clip(along / 0.15, 0.0, 1.0))
    v = m[:, None]
    for _ in range(2):
        v = (v + lap.mean(v)) / 2
    return np.clip(v[:, 0], 0.0, 1.0)


def face_hold(P, lm, eyes, eye_radius, lap):
    """Per vertex, 1 where shapes may move the skin, 0 where they may not
    (the lids and sockets, the lips' edges, the mouth's inside)."""
    X = array(P)
    k = (X[:, 2].max() - X[:, 2].min()) / 1.75
    free = np.ones(len(X))
    for s in (1, -1):
        d = np.linalg.norm(X - np.array(eyes[s]), axis=1)
        free = np.minimum(free, np.clip((d - 1.5 * eye_radius) / (1.6 * eye_radius), 0.0, 1.0))
    edge = sorted(set(lm.ring(FS_HEAD, FS_MOUTH_UPPER)) | set(lm.ring(FS_MUZZLE, FS_MOUTH_UPPER))
                  | set(lm.ring(FS_MUZZLE, FS_MOUTH_LOWER)))
    if edge:
        E = X[edge]
        d = np.sqrt(((X[:, None, :] - E[None]) ** 2).sum(axis=2)).min(axis=1)
        free = np.minimum(free, np.clip((d - 0.006 * k) / (0.016 * k), 0.0, 1.0))
    inside = sorted({i for fs in (FS_SOCKET[1], FS_SOCKET[-1], FS_MOUTH_UPPER, FS_MOUTH_LOWER) for i in lm.of_set[fs]})
    free[inside] = 0.0
    v = free[:, None]
    for _ in range(2):
        v = np.minimum(v, (v + lap.mean(v)) / 2)
    v[inside] = 0.0
    return v[:, 0]


# -- the build's step -------------------------------------------------------------------

def unpack(path=PACK):
    """{frame: {corner: offsets}} from the vendored file, in the bundle's pose."""
    data = np.load(path)
    step = float(data["step"])
    count = int(data["count"]) if "count" in data.files else 10582
    out = {frame: {} for frame in FRAMES}
    for key in data.files:
        if key.endswith("_index"):
            name = key[:-len("_index")]
            frame, corner = name.split("_", 1)
            if "muscle" not in corner:
                continue
            d = np.zeros((count, 3))
            d[data[key]] = data[name + "_offset"].astype(np.float64) * step
            out[frame.capitalize()][corner] = d
    return out


def composites(frame):
    """The slider settings of a frame that are checked for crossings and
    kept apart: {name: values}. The four corners of the two whole-body
    sliders with every region's slider pushed the same way, the heaviest
    first, then each slider's own ends."""
    every = lambda title, v: {f"{title}_{r}": v for r in REGIONS}  # noqa: E731
    return {
        "heavy and soft": {"muscle": -1.0, "fat": 1.0, **every("fat", 1.0)},
        "heavy and muscular": {"muscle": 1.0, "fat": 1.0, **every("fat", 1.0), **every("muscle", 1.0)},
        "slight": {"muscle": -1.0, "fat": -1.0, **every("fat", -1.0), **every("muscle", -1.0)},
        "lean and muscular": {"muscle": 1.0, "fat": -1.0, **every("fat", -1.0), **every("muscle", 1.0)},
        "muscular": {"muscle": 1.0, "fat": 0.0, **every("muscle", 1.0)},
        "heavy": {"muscle": 0.0, "fat": 1.0, **every("fat", 1.0)},
        "slim": {"muscle": 0.0, "fat": -1.0, **every("fat", -1.0)},
        "soft": {"muscle": -1.0, "fat": 0.0, **every("muscle", -1.0)},
        "slight, whole sliders only": {"muscle": -1.0, "fat": -1.0},
        "lean and muscular, whole sliders only": {"muscle": 1.0, "fat": -1.0},
    }


def mixed(shapes, values, frame):
    """The offsets a frame's shapes add at slider `values` (pure frame)."""
    w = weights_of(values, 0.0 if frame == "Masculine" else 1.0)
    return sum(shapes[name] * weight for name, weight in w.items() if name in shapes and weight)


def build(body, source, rest, lm, joints, weights, eyes, eye_radius, log=print, mirror=None, grow=None):
    """Every body shape as offsets from each frame's rest positions:
    ({shape name: (frame, offsets)}, report, the mesh's triangles,
    Laplacian and mirror pairing for whoever works on it next). `source` and `rest` are
    {frame: positions} in the bundle's pose and the rest pose; `weights`
    the skin's; `eyes` and `eye_radius` {frame: ...}. Where the body has
    more vertices than the bundle's (`rims.split`), `mirror` is its
    mirror pairing and `grow` gives the vendored offsets the new
    vertices' rows."""
    geom, transfer = _transfer()
    me = body.data
    faces = [tuple(p.vertices) for p in me.polygons]
    tris = geom.triangles(np.array([f if len(f) == 4 else f + (-1,) for f in faces]))
    edges = np.array([tuple(e.vertices) for e in me.edges])
    lap = geom.Laplacian(len(me.vertices), edges)
    if mirror is None:
        mirror = geom.mirror_map([array(source["Masculine"]), array(source["Feminine"])])
    packed = unpack()
    if grow is not None:
        packed = {frame: {name: grow(d) for name, d in corners.items()} for frame, corners in packed.items()}
    out, report = {}, {"frames": {}}
    for frame in FRAMES:
        P0, P = array(source[frame]), array(rest[frame])
        normals = geom.vertex_normals(P, tris)
        normals /= np.maximum(np.linalg.norm(normals, axis=1, keepdims=True), 1e-12)
        corner = {name: transfer.reframe(d, P0, P, edges) for name, d in packed[frame].items()}
        shapes = engine_shapes(Grid(corner), frame)
        # Per region: a share of the step to the sliders' heavy corners. Fat
        # in one place is the heavy and soft body's there (MakeHuman's fat is
        # in its low-muscle bodies: weight alone, at average muscle, is
        # mild); muscle in one place is the heavy and muscular body's (tone
        # and mass together, which is what a bigger muscle is).
        m0, m_lo, m_hi = PLACE[frame]["muscle"]
        w0, _, w_hi = PLACE[frame]["fat"]
        here = Grid(corner).at(m0, w0)
        fat_up = Grid(corner).at(m_lo, w_hi) - here
        muscle_up = Grid(corner).at(m_hi, w_hi) - here
        regions = region_weights(weights, normals, lap)
        for region, share in regions.items():
            shapes[f"{frame}_Fat_{region.capitalize()}"] = fat_up * share[:, None]
            shapes[f"{frame}_Muscle_{region.capitalize()}"] = muscle_up * share[:, None]
        # Fat keeps off bone; nothing moves the eyes' and the mouth's surroundings.
        hold = bony(rest[frame], lm, joints[frame], eyes[frame], normals, lap)
        free = face_hold(rest[frame], lm, eyes[frame], eye_radius[frame], lap)
        for name in shapes:
            if "Fat" in name:
                shapes[name] = shapes[name] * (1.0 - HOLD * hold)[:, None]
            shapes[name] = transfer.symmetric(shapes[name] * free[:, None], mirror)
        # Kept apart, in the rest pose.
        known = transfer.crossing_pairs(P, tris)
        factor = np.ones(len(P))
        cut = {}
        for name, values in composites(frame).items():
            d = mixed(shapes, values, frame)
            before = len(transfer.new_crossings(P, P + d * factor[:, None], tris, known))
            _, f, left = transfer.apart(P, d * factor[:, None], tris, lap, mirror, known)
            factor *= f
            cut[name] = {"crossings_before": before, "left": left, "vertices_cut": int((f < 0.999).sum()),
                         "least_factor": round(float(f.min()), 3)}
            log(f"  {frame} {name}: {before} new crossings, {left} left, {int((f < 0.999).sum())} vertices cut "
                f"(to {f.min():.2f} at most)")
        for name in shapes:
            shapes[name] = shapes[name] * factor[:, None]
            out[name] = (frame, shapes[name])
        report["frames"][frame] = {
            "neutral_crossings": len(known), "kept_apart": cut,
            "largest_move_mm": {n: round(float(np.linalg.norm(d, axis=1).max()) * 1000, 1) for n, d in shapes.items()},
            "bony_vertices": int((hold > 0.5).sum()), "held_face_vertices": int((free < 0.5).sum()),
        }
    report["shapes"] = len(out)
    report["place"] = {f: {a: list(PLACE[f][a]) for a in AXES} for f in FRAMES}
    report["local"] = LOCAL
    return out, report, {"tris": tris, "lap": lap, "mirror": mirror}


def add_keys(body, shapes, rest):
    """Writes the shapes as shape keys of `body` (whose basis is the
    masculine rest). A feminine shape's offsets are added as they are:
    the engine adds every shape's offsets to the same rest mesh."""
    if body.data.shape_keys is None:
        body.shape_key_add(name="Basis", from_mix=False)
    base = array(rest["Masculine"])
    for name in sorted(shapes):
        _, d = shapes[name]
        key = body.shape_key_add(name=name, from_mix=False)
        co = (base + d).ravel()
        key.data.foreach_set("co", co)
        key.value = 0.0
