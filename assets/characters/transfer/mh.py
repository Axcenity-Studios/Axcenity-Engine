"""Reads MakeHuman's CC0 data files (the base mesh and `.target` files) and
mixes its macro targets. Plain numpy; no MakeHuman or MPFB code is used or
adapted: the two formats are simple text and were read from the files
themselves.

- `base.obj`: an OBJ file, Y up, decimetres, the figure facing +Z. Its
  `g` groups split the faces into `body` (the skin, the first 13,380
  vertices), `helper-*` (clothes helpers, hair, a skirt, genitals, eyes,
  teeth, tongue, lashes) and `joint-*` (one small cube per joint: its
  centre is the joint).
- `*.target`: after `#` comment lines, one line per moved vertex:
  `index dx dy dz`, in the base mesh's vertex order and units.

Everything this module returns is in Blender space: metres, Z up, the
figure facing -Y, its left side at +X.
"""

import os

import numpy as np

SCALE = 0.1  # decimetres to metres
BODY_VERTS = 13380  # checked in `load_base`

GENDERS = ("male", "female")
MUSCLES = ("minmuscle", "averagemuscle", "maxmuscle")
WEIGHTS = ("minweight", "averageweight", "maxweight")
RACES = ("african", "asian", "caucasian")


def to_blender(a):
    """MakeHuman (x, y up, z front) decimetres to Blender (x, -z, y) metres."""
    a = np.asarray(a, dtype=np.float64)
    return np.stack([a[..., 0], -a[..., 2], a[..., 1]], axis=-1) * SCALE


def data_dir(root):
    """`root` is a checkout of github.com/makehumancommunity/makehuman."""
    d = os.path.join(root, "makehuman", "data")
    if not os.path.isdir(d):
        raise SystemExit(f"no MakeHuman data under {root} (run fetch_makehuman.py)")
    return d


def load_base(root):
    """The base mesh: `verts` (all 19,158, Blender space), `faces` (the
    body's quads as vertex indices), `joints` ({name: centre} from the
    joint cubes) and `groups` ({group: its vertex indices})."""
    path = os.path.join(data_dir(root), "3dobjs", "base.obj")
    verts, group, faces_by_group = [], None, {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("v "):
                verts.append([float(x) for x in line.split()[1:4]])
            elif line.startswith("g "):
                group = line.split()[1]
            elif line.startswith("f "):
                face = [int(t.split("/")[0]) - 1 for t in line.split()[1:]]
                faces_by_group.setdefault(group, []).append(face)
    verts = to_blender(np.array(verts))
    body = faces_by_group["body"]
    assert all(len(f) == 4 for f in body), "the body is expected to be all quads"
    faces = np.array(body, dtype=np.int64)
    assert faces.max() == BODY_VERTS - 1 and len(np.unique(faces)) == BODY_VERTS, \
        "the body's faces don't use exactly the first 13,380 vertices"
    groups = {g: np.unique(np.concatenate(fs)) for g, fs in faces_by_group.items()}
    joints = {g[len("joint-"):]: verts[idx].mean(axis=0)
              for g, idx in groups.items() if g.startswith("joint-")}
    return {"verts": verts, "faces": faces, "joints": joints, "groups": groups}


def load_target(root, name, count=None):
    """A target as a dense (count, 3) array of offsets in Blender space.
    `name` is relative to `data/targets`, without `.target`."""
    path = os.path.join(data_dir(root), "targets", name + ".target")
    idx, off = [], []
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            p = line.split()
            idx.append(int(p[0]))
            off.append((float(p[1]), float(p[2]), float(p[3])))
    n = count or 19158
    out = np.zeros((n, 3))
    if idx:
        idx = np.array(idx)
        keep = idx < n
        out[idx[keep]] = to_blender(np.array(off))[keep]
    return out


def corner_name(gender, muscle, weight, age="young"):
    return f"macrodetails/universal-{gender}-{age}-{muscle}-{weight}"


def neutral_offset(root, gender, count=None, age="young"):
    """What MakeHuman adds to the base mesh for `gender` at its default
    ethnicity (one third of each ethnic target), muscle and weight."""
    return sum(load_target(root, f"macrodetails/{race}-{gender}-{age}", count)
               for race in RACES) / len(RACES)


def factors(value):
    """MakeHuman's split of a macro slider (0..1, 0.5 average) into the
    weights of its min, average and max targets (they add to 1): piecewise
    linear, average at 0.5. Written from the rule as the Stage 1 research
    note states it (notes/body-face-skin-cloth.md, part C); no MakeHuman
    code was read for this spike."""
    lo = max(0.0, 1.0 - 2.0 * value)
    hi = max(0.0, 2.0 * value - 1.0)
    return (lo, 1.0 - lo - hi, hi)


def corner_weights(muscle, weight):
    """{(muscle name, weight name): weight} of the nine corner targets of
    one gender and age: each the product of its factors."""
    fm, fw = factors(muscle), factors(weight)
    return {(m, w): fm[i] * fw[j]
            for i, m in enumerate(MUSCLES) for j, w in enumerate(WEIGHTS)}
