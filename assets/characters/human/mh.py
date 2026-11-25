"""Drives the installed MPFB add-on as a tool, and the little arithmetic
the build does on what it returns.

MPFB (https://static.makehumancommunity.org/mpfb.html, by Joel Palmius and
the MakeHuman community) is GPL-3.0-or-later. **Nothing of its code is
copied here or anywhere in this repository**: this module finds the add-on
the person installed in Blender, enables it for the session and calls its
services (`HumanService`, `TargetService`), the way a person would from
its panels. What it returns (a body, a rig, weights, fitted clothes) is
MPFB's output, which its licence leaves to the person who made it; the
meshes, targets and textures it reads are MakeHuman's assets, CC0 1.0 or
CC BY 4.0 per asset (`README.md`, "Sources and credit").

The clothes format (`.mhclo`) is read here from its published description
(MakeHuman's documentation, "File formats and extensions": a clothes
vertex is three body vertex numbers, three weights and an offset, the
offset scaled per axis by the distance between two named body vertices),
and checked against what MPFB itself fits (`build.py --check`).

Units are metres, Blender Z up, the character faces -Y (its left is +X).
MakeHuman's own files are in decimetres, Y up, facing +Z.
"""

import importlib
import math
import os

import addon_utils
import bpy
import numpy as np

_module = None


def enable():
    """Enables the installed MPFB add-on for this session (the saved
    preferences aren't touched) and returns its module name."""
    global _module
    if _module:
        return _module
    names = [m.__name__ for m in addon_utils.modules() if m.__name__.split(".")[-1] == "mpfb"]
    if not names:
        raise SystemExit("MPFB isn't installed in this Blender (Preferences > Get Extensions > MPFB). "
                         "Run without --factory-startup so Blender finds your extensions.")
    name = names[0]
    if not addon_utils.check(name)[1]:
        # MPFB reads its preferences while it registers, so it must be
        # listed before it is enabled.
        if name not in bpy.context.preferences.addons:
            bpy.context.preferences.addons.new().module = name
        addon_utils.enable(name, default_set=False)
    if not addon_utils.check(name)[1]:
        raise SystemExit("MPFB is installed but didn't enable; see Blender's output above.")
    _module = name
    return name


def service(module, name):
    return getattr(importlib.import_module(f"{enable()}.services.{module}"), name)


def version():
    for m in addon_utils.modules():
        if m.__name__ == enable():
            v = addon_utils.module_bl_info(m).get("version", ())
            return ".".join(str(x) for x in v)
    return "unknown"


# -- the human ---------------------------------------------------------------------

#: MakeHuman's macro controls at their middles. `gender` 1 is male.
NEUTRAL = {"gender": 1.0, "age": 0.5, "muscle": 0.5, "weight": 0.5, "proportions": 0.5, "height": 0.5,
           "cupsize": 0.5, "firmness": 0.5, "race": {"asian": 1 / 3, "caucasian": 1 / 3, "african": 1 / 3}}


def macro(**changes):
    out = {k: (dict(v) if isinstance(v, dict) else v) for k, v in NEUTRAL.items()}
    out.update(changes)
    return out


def human(settings):
    """A new MakeHuman body with these macro settings: the base mesh with
    its helper geometry, in metres, about MakeHuman's own origin (the
    hips): MPFB's rig is fitted in that frame, so the build stands the
    body on the floor itself."""
    create = service("humanservice", "HumanService").create_human
    return create(mask_helpers=False, detailed_helpers=False, extra_vertex_groups=True,
                  feet_on_ground=False, scale=0.1, macro_detail_dict=settings)


def positions(obj):
    """The object's corners where its shape keys' values put them."""
    n = len(obj.data.vertices)
    keys = obj.data.shape_keys
    out = np.empty(n * 3)
    if not keys:
        obj.data.vertices.foreach_get("co", out)
        return out.reshape(n, 3)
    basis = np.empty(n * 3)
    keys.reference_key.data.foreach_get("co", basis)
    out[:] = basis
    one = np.empty(n * 3)
    for key in keys.key_blocks:
        if key == keys.reference_key or key.mute or key.value == 0.0:
            continue
        key.data.foreach_get("co", one)
        out += key.value * (one - basis)
    return out.reshape(n, 3)


def remove(obj):
    """Removes an object and what MPFB hung on it (its rig, its clothes)."""
    family = [obj] + list(obj.children_recursive)
    if obj.parent:
        family += [obj.parent] + [c for c in obj.parent.children_recursive if c not in family]
    for o in family:
        data = o.data
        bpy.data.objects.remove(o)
        if data and data.users == 0:
            (bpy.data.meshes if isinstance(data, bpy.types.Mesh) else bpy.data.armatures).remove(data)


def sample(settings):
    """The body's corners at these macro settings (the object isn't kept)."""
    obj = human(settings)
    out = positions(obj)
    remove(obj)
    return out


def add_rig(base, name="default"):
    """Gives `base` one of MPFB's rigs with its weights; returns the rig.
    The body's shape is baked into its mesh first: MPFB fits a rig to the
    mesh as stored, so on a body still wearing its targets as shape keys
    the joints would be the unshaped base mesh's."""
    if base.data.shape_keys:
        service("targetservice", "TargetService").bake_targets(base)
    service("humanservice", "HumanService").add_builtin_rig(base, name, import_weights=True)
    return base.parent


def add_asset(base, path, kind):
    """Loads a clothes-type asset (`Clothes`, `Eyes`, `Teeth`, `Eyebrows`,
    `Eyelashes`, `Tongue`, `Hair`) fitted to `base`, with weights when the
    body has a rig; returns its object."""
    add = service("humanservice", "HumanService").add_mhclo_asset
    return add(path, base, asset_type=kind, subdiv_levels=0, material_type="MAKESKIN")


def rig_bones(rig):
    """[(name, parent or None)] of the rig's bones, parents first."""
    return [(b.name, b.parent.name if b.parent else None) for b in rig.data.bones]


def weights(obj, names):
    """The object's weights on the bones `names`, as (corners x bones)."""
    column = {n: k for k, n in enumerate(names)}
    of_group = {g.index: column[g.name] for g in obj.vertex_groups if g.name in column}
    out = np.zeros((len(obj.data.vertices), len(names)))
    for v in obj.data.vertices:
        for g in v.groups:
            k = of_group.get(g.group)
            if k is not None:
                out[v.index, k] = g.weight
    return out


def data_root():
    """The installed add-on's data folder (MakeHuman's base mesh and targets)."""
    return os.path.join(os.path.dirname(importlib.import_module(enable()).__file__), "data")


def read_target(folder, name, count):
    """A MakeHuman target's moves of the body's `count` corners (helpers
    included), in metres and Blender's axes. A target file is a line a
    corner it moves: its number and its move, in decimetres, Y up, +Z
    front (`<folder>/<name>.target.gz` under the add-on's `data/targets`)."""
    import gzip
    out = np.zeros((count, 3))
    path = os.path.join(data_root(), "targets", folder, name + ".target.gz")
    with gzip.open(path, "rt", encoding="utf-8", errors="replace") as f:
        for line in f:
            words = line.split()
            if len(words) == 4 and not line.startswith("#"):
                x, y, z = (float(w) for w in words[1:])
                out[int(words[0])] = (x * 0.1, -z * 0.1, y * 0.1)
    return out


def joint_groups():
    """{joint name: body corner numbers} of the base mesh's joint cubes:
    MakeHuman's base mesh (`base.obj`, CC0) carries a small cube at every
    joint, named `joint-<name>`, which every target moves with the body.
    Their middles are the joints (MPFB 2.0.18's own rig fit leaves half of
    its bones on the unshaped mesh under Blender 5.2, so it isn't used)."""
    groups, current = {}, None
    with open(os.path.join(data_root(), "3dobjs", "base.obj"), encoding="utf-8") as f:
        for line in f:
            if line.startswith("g "):
                current = line.split()[1]
            elif line.startswith("f ") and current and current.startswith("joint-"):
                groups.setdefault(current[len("joint-"):], set()).update(
                    int(t.split("/")[0]) - 1 for t in line.split()[1:])
    return {name: np.array(sorted(idx)) for name, idx in groups.items()}


def joint_centres(groups, body):
    return {name: body[idx].mean(axis=0) for name, idx in groups.items()}


# -- clothes files -----------------------------------------------------------------

class Clothes:
    """What a `.mhclo` file says: how each of its corners hangs on the
    body, and which body corners it hides."""

    def __init__(self, path):
        self.path = path
        self.folder = os.path.dirname(path)
        self.header = []
        self.fields = {}
        self.tags = []
        self.scale = {}
        rows, exact, self.delete = [], [], []
        mode = None
        with open(path, encoding="utf-8", errors="replace") as f:
            for raw in f:
                line = raw.strip()
                if not line:
                    continue
                if line.startswith("#"):
                    self.header.append(line.lstrip("# ").rstrip())
                    continue
                words = line.split()
                key = words[0]
                numeric = key.lstrip("-").replace(".", "", 1).isdigit()
                if not numeric:
                    # A field may come between `verts 0` and its rows (the
                    # shoes name their material there); only a list of
                    # hidden corners ends at the next word.
                    if mode == "delete":
                        mode = None
                    if key == "verts":
                        mode = "verts"
                    elif key == "delete_verts":
                        mode = "delete"
                    elif key in ("x_scale", "y_scale", "z_scale") and len(words) >= 4:
                        self.scale[key[0]] = (int(words[1]), int(words[2]), float(words[3]))
                    elif key == "tag":
                        self.tags.append(" ".join(words[1:]))
                    else:
                        self.fields[key] = " ".join(words[1:])
                    continue
                if mode == "verts":
                    if len(words) >= 9:
                        rows.append([float(w) for w in words[:9]])
                    else:
                        # One number: the corner sits exactly on that body corner.
                        rows.append([float(words[0])] * 3 + [1.0, 0.0, 0.0, 0.0, 0.0, 0.0])
                elif mode == "delete":
                    k = 0
                    while k < len(words):
                        if k + 2 < len(words) and words[k + 1] == "-":
                            self.delete += range(int(words[k]), int(words[k + 2]) + 1)
                            k += 3
                        else:
                            self.delete.append(int(words[k]))
                            k += 1
        table = np.array(rows) if rows else np.zeros((0, 9))
        self.index = table[:, :3].astype(np.int64)
        self.weight = table[:, 3:6]
        # MakeHuman's axes (Y up, front +Z) to Blender's (Z up, front -Y).
        self.offset = np.stack([table[:, 6], -table[:, 8], table[:, 7]], axis=1)
        self.name = self.fields.get("name", os.path.splitext(os.path.basename(path))[0])

    def licence(self):
        """What the file's own header says of its licence: `CC0`, `CC-BY`,
        `AGPL` (the old MakeClothes default line, never taken) or `unknown`."""
        text = " ".join(self.header).lower() + " " + self.fields.get("license", "").lower()
        if "agpl" in text:
            return "AGPL"
        if "cc0" in text or "cc 0" in text:
            return "CC0"
        if "cc-by" in text or "cc by" in text:
            return "CC-BY"
        return "unknown"

    def author(self):
        for line in self.header:
            low = line.lower()
            if low.startswith("author") or low.startswith("copyright"):
                return line.split(None, 1)[1].lstrip(":( ").strip() if " " in line else ""
        return self.fields.get("author", "")

    def place(self, body):
        """Its corners on a body whose corners (helpers included) are `body`."""
        base = np.einsum("vk,vkc->vc", self.weight, body[self.index])
        scale = np.ones(3)
        # x is across, y is height (Blender's z), z is depth (Blender's y).
        for axis, column in (("x", 0), ("y", 2), ("z", 1)):
            if axis in self.scale:
                a, b, distance = self.scale[axis]
                if distance > 0:
                    scale[column] = abs(body[a, column] - body[b, column]) / distance
        return base + self.offset * scale


# -- re-posing ---------------------------------------------------------------------

def rotation_between(a, b):
    """The smallest rotation (3x3) that turns direction `a` onto `b`."""
    a = a / np.linalg.norm(a)
    b = b / np.linalg.norm(b)
    v = np.cross(a, b)
    c = float(np.dot(a, b))
    s = np.linalg.norm(v)
    if s < 1e-9:
        return np.eye(3)
    k = np.array([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])
    return np.eye(3) + k + k @ k * ((1 - c) / (s * s))


class Repose:
    """Turns MakeHuman's rest pose (arms about 48 degrees down, elbows bent
    forward) into the engine's: arms straight, 45 degrees below horizontal.
    A corner goes where its bones take it (linear blend skinning with
    MakeHuman's own weights), and a shape's moves turn with it.

    `bones` is [(name, parent)] of MakeHuman's rig, parents first;
    `joints` the body's joint centres ([`joint_centres`])."""

    def __init__(self, bones, joints, arm_down=math.radians(45.0)):
        self.names = [b[0] for b in bones]
        parent = dict(bones)
        turn, pivot = {}, {}
        for s, side, cube in ((1, "L", "l"), (-1, "R", "r")):
            shoulder, elbow, wrist = (joints[f"{cube}-{n}"] for n in ("shoulder", "elbow", "hand"))
            target = np.array([s * math.cos(arm_down), 0.0, -math.sin(arm_down)])
            upper = rotation_between(elbow - shoulder, target)
            turn[f"upperarm01.{side}"], pivot[f"upperarm01.{side}"] = upper, shoulder
            # The forearm, in the frame its parent leaves it in.
            turn[f"lowerarm01.{side}"] = rotation_between(wrist - elbow, upper.T @ target)
            pivot[f"lowerarm01.{side}"] = elbow
        self.matrix = {None: np.eye(4)}
        for name in self.names:
            local = np.eye(4)
            if name in turn:
                local[:3, :3] = turn[name]
                local[:3, 3] = pivot[name] - turn[name] @ pivot[name]
            self.matrix[name] = self.matrix[parent[name]] @ local
        self.stack = np.stack([self.matrix[n] for n in self.names])

    def carry(self, bone, point):
        """Where `bone` (None: a bone the pose doesn't move) takes a point."""
        m = self.matrix[bone]
        return m[:3, :3] @ point + m[:3, 3]

    def blend(self, weights):
        """Each corner's own transform (corners x 4 x 4) from its weights."""
        total = weights.sum(axis=1, keepdims=True)
        w = np.where(total > 1e-9, weights / np.maximum(total, 1e-9), 0.0)
        m = np.einsum("vb,bij->vij", w, self.stack)
        loose = (total[:, 0] <= 1e-9)
        m[loose] = np.eye(4)
        return m

    @staticmethod
    def points(m, p):
        return np.einsum("vij,vj->vi", m[:, :3, :3], p) + m[:, :3, 3]

    @staticmethod
    def moves(m, d):
        return np.einsum("vij,vj->vi", m[:, :3, :3], d)
