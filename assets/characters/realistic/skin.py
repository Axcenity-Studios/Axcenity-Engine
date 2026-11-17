"""The skin's texture set, baked from the vendored sculpts and from colour
and roughness fields over the body: albedo, a tangent-space normal map and
a packed occlusion-roughness map, at 2K by default (4K for the hero set).

- **Normal**: each frame's sculpt (the base mesh subdivided three times
  plus the vendored displacement, `source/sculpt_*.npz`) is baked onto
  its own low mesh, in the bundle's pose. One body wears both frames, so
  the two bakes are joined into a map that is right for both: where they
  agree (collarbones, knees, the navel, the spine, hands, feet, the face's
  folds) it is their mean; where they differ (the chest, the belly's
  muscles) it leans to the flatter of the two, so neither frame shows the
  other's anatomy. That map is the **defined** one (lean and
  muscular: abdominal lines, muscle separation). A **smooth** copy
  (what fat makes of it) flattens the sculpt's forms over the soft parts
  of the body (`soft_tissue`: the trunk, shoulders, arms and legs, but
  not the head, hands, feet, knees, elbows, nipples or navel (`navel`:
  the navel found and kept by name, as the occlusion doesn't keep its rim), nor
  any deep crease) and is the material's own normal map; the defined one
  rides beside it in the glTF material's `extras`
  (`axcenity_defined_normal`, `mark`), and the renderer's Skin model
  blends the two by the material values `definition` and `softness`,
  which the manifest drives from `muscle` and `fat` (R5, OPEN-07). Skin
  grain (cells about five texels across, and finer wrinkling; R5
  doubled the cells and halved the slope, so it reads as skin at a
  metre and as smooth at three) is added on top of both from a bump
  texture in the body's space, which has no seams.
- **Albedo**: a skin tone varied by region (a flush on the cheeks, nose,
  ears, chin, knuckles, elbows and knees; lips; paler palms and soles;
  nails), mottled at three scales, darkened a little in the
  creases, with the inside of the mouth and the eye sockets as flesh.
- **Occlusion, roughness**: red is the sculpts' ambient occlusion at
  short range (creases, not the pose's shadows), green the roughness
  (oilier on the forehead, nose and chin, drier on the elbows, knees and
  heels, smooth lips and nails), blue (metalness) 0. One image, as glTF's
  `metallicRoughnessTexture` and `occlusionTexture`.

The maps are ordinary glTF ones on the material `Skin`, which `mark`
marks `"axcenity_shading": "skin"` in its `extras`: the engine draws it
with its Skin shading model (diffusion, two specular lobes;
`docs/rendering/skin.md`), and any other viewer as standard PBR with the
smooth normal map.
"""

import os

import bpy
import numpy as np
from mathutils import Vector

import rig
from common import (
    FS_EAR, FS_FINGERS, FS_FOOT, FS_HEAD, FS_MUZZLE, FS_PALM, FS_SOCKET, FS_TOES, FS_TOE_BASE, FS_TORSO,
    HEAD_SETS,
    MOUTH_SETS, SIDES, SOURCE, array, duplicate, edge_neighbours, face_sets, fit_sphere, select_only,
    shade_smooth, subdivide,
)

SCULPT_LEVELS = 3
# The skin's tone (sRGB) and what each region does to it (linear factors).
TONE = (0.74, 0.53, 0.43)
FLESH = (0.42, 0.13, 0.14)          # inside the mouth, the eye sockets
LIPS = (0.57, 0.27, 0.29)
INNER_LIP = (0.50, 0.19, 0.21)      # the lips' inner sides at their edges
# What light is left on the mouth's lining by how far (metres) it is from
# the line the lips meet on: the cheeks' insides by the corners are lit,
# the back of the mouth is nearly black.
MOUTH_SHADE = ((0.006, 1.0), (0.020, 0.85), (0.040, 0.62), (0.060, 0.45))
# (The engine's own shadows and screen-space occlusion already make the
# back of an open mouth dark: with more than this baked on top it was a
# flat black hole.) Only the throat is near black: the lining further
# than the first of these from the lips' line, by how near it is to the
# middle of the mouth's back (to this share within the second distance,
# none of it beyond the third; metres).
THROAT = (0.042, 0.15, 0.005, 0.020)
# How much of the lining's shade goes into its occlusion (`teeth.IN_COLOUR`
# of it is in its colour).
LINING_OCCLUDES = 0.4
# The upper lip's underside lies on the lower lip while the mouth is
# closed, and is wet: seen through the hair's breadth between the lips it
# mirrored the studio's floor as a pale line along the mouth. So by how
# far in from the lips' edges it is (metres), it is occluded a little (a
# laugh bares it: much, and it was a black band over the teeth) and
# matt: (distance, occlusion, roughness; further in it is `WET`).
LIP_EDGE = ((0.0005, 0.25, 0.9), (0.005, 1.0, 0.9), (0.009, 1.0, 0.35))
WET = 0.35
# The least occlusion left where the lips touch (the sculpts are closed).
LIP_SEAM_OCCLUSION = 0.85
NAIL = (0.86, 0.70, 0.66)
FLUSH = (1.12, 0.82, 0.80)
PALE = (1.08, 1.01, 0.97)
ROUGHNESS = 0.50
# Nail sets: the fourth of each finger's, and the toes' smallest.
TOE_NAILS = (37, 42, 43, 44, 45, 47, 51, 55, 59, 63)
FILES = ("RealisticSkin_Albedo.jpg", "RealisticSkin_Normal.png", "RealisticSkin_ORM.jpg",
         "RealisticSkin_NormalDefined.png")
# Skin grain (R5): the cells' size and the finer wrinkling's, in texels,
# and the bump's height, in texels. Cells under four texels read as a
# speckle a pixel wide in a face seen from a metre (wave 1: 2.6 texels,
# 0.09 high); these are twice as wide and half as steep.
GRAIN_CELL = 5.0
GRAIN_FINE = 12.0
GRAIN_HEIGHT = 0.09
# How far the smooth normal map flattens the sculpt's forms over soft
# tissue (`soft_tissue`): what fat leaves of the definition.
SMOOTHED = 0.9


def srgb_to_linear(c):
    c = np.asarray(c, dtype=np.float64)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def _step(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def linear_to_srgb(c):
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


# -- the sculpts ---------------------------------------------------------------------

def sculpt(low, frame):
    """The frame's sculpt as a mesh object: `low` subdivided, plus the
    vendored displacement."""
    data = np.load(os.path.join(SOURCE, f"sculpt_{frame.lower()}.npz"))
    high = duplicate(low, f"Sculpt{frame}")
    for group in list(high.vertex_groups):
        high.vertex_groups.remove(group)
    subdivide(high, int(data["levels"]))
    deps = bpy.context.evaluated_depsgraph_get()
    mesh = bpy.data.meshes.new_from_object(high.evaluated_get(deps))
    high.modifiers.clear()
    high.data = mesh
    co = np.empty(len(mesh.vertices) * 3, dtype=np.float64)
    mesh.vertices.foreach_get("co", co)
    co = co.reshape(-1, 3)
    displacement = data["displacement"].astype(np.float64)
    if len(co) != len(displacement) or np.abs(
            co[data["check_index"]] - data["check_position"]).max() > 2e-4:
        raise RuntimeError(
            f"sculpt_{frame.lower()}.npz doesn't fit this Blender's subdivision "
            "(its vertex order or count differs): run source/extract.py again with this version")
    mesh.vertices.foreach_set("co", (co + displacement).ravel())
    mesh.update()
    shade_smooth(high)
    return high


# -- baking --------------------------------------------------------------------------

def _image(name, size, colour=(0.0, 0.0, 0.0, 1.0)):
    img = bpy.data.images.new(name, size, size, alpha=False, float_buffer=True)
    img.colorspace_settings.name = "Non-Color"
    img.generated_color = colour
    return img


def _pixels(img):
    size = img.size[0]
    data = np.empty(size * size * 4, dtype=np.float32)
    img.pixels.foreach_get(data)
    # Rows from the top, as image files have them.
    return data.reshape(size, size, 4)[::-1, :, :3].astype(np.float64)


def _target(obj, img, build=None):
    """Gives `obj` one material whose active image node is `img` (what a
    bake writes to); `build(node_tree, bsdf)` wires what's baked."""
    m = bpy.data.materials.new("Bake")
    m.use_nodes = True
    nt = m.node_tree
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = img
    nt.nodes.active = tex
    if build:
        build(nt, nt.nodes["Principled BSDF"])
    obj.data.materials.clear()
    obj.data.materials.append(m)
    for poly in obj.data.polygons:
        poly.material_index = 0
    return m


def _bake(kind, low, high=None, samples=1, size=2048):
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = False
    bake = scene.render.bake
    bake.margin = max(8, size // 128)
    bake.margin_type = "EXTEND"
    bake.use_clear = True
    bake.use_selected_to_active = high is not None
    if high is not None:
        bake.use_cage = False
        bake.cage_extrusion = 0.003
        bake.max_ray_distance = 0.007
    bake.normal_space = "TANGENT"
    bpy.ops.object.select_all(action="DESELECT")
    if high is not None:
        high.select_set(True)
    low.select_set(True)
    bpy.context.view_layer.objects.active = low
    bpy.ops.object.bake(type=kind)


def _emit(nt, colour_socket):
    out = nt.nodes["Material Output"]
    em = nt.nodes.new("ShaderNodeEmission")
    nt.links.new(colour_socket, em.inputs["Color"])
    nt.links.new(em.outputs["Emission"], out.inputs["Surface"])


def _attribute(obj, name, values):
    attr = obj.data.color_attributes.new(name=name, type="FLOAT_COLOR", domain="POINT")
    flat = np.concatenate([values, np.ones((len(values), 1))], axis=1).astype(np.float32)
    attr.data.foreach_set("color", flat.ravel())
    return attr


def _coverage(body, size, sets=None):
    """A mask of the texels the body's faces (or those of `sets`) cover."""
    from PIL import Image, ImageDraw
    img = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(img)
    layer = body.data.uv_layers[0].data
    of = face_sets(body) if sets is not None else None
    for poly in body.data.polygons:
        if sets is not None and of[poly.index] not in sets:
            continue
        pts = [(layer[i].uv[0] * size, (1 - layer[i].uv[1]) * size) for i in poly.loop_indices]
        draw.polygon(pts, fill=255, outline=255)
    return np.asarray(img) > 0


def _grow(mask, times):
    for _ in range(times):
        grown = mask.copy()
        grown[1:, :] |= mask[:-1, :]
        grown[:-1, :] |= mask[1:, :]
        grown[:, 1:] |= mask[:, :-1]
        grown[:, :-1] |= mask[:, 1:]
        mask = grown
    return mask


def _blur(a, radius):
    """A box blur of `a` ([h, w] or [h, w, c]), `radius` texels each way."""
    if radius < 1:
        return a
    out = a.astype(np.float64)
    for axis in (0, 1):
        c = np.cumsum(np.concatenate([np.zeros_like(out.take([0], axis=axis)), out], axis=axis), axis=axis)
        n = out.shape[axis]
        hi = np.clip(np.arange(n) + radius + 1, 0, n)
        lo = np.clip(np.arange(n) - radius, 0, n)
        out = (c.take(hi, axis=axis) - c.take(lo, axis=axis)) / (hi - lo).reshape(
            [-1 if k == axis else 1 for k in range(out.ndim)])
    return out


def _near_lips(body, P, lm, size, full, none, sets=None):
    """A soft mask over the texels of how near the line the lips meet on
    each face is: 1 within `full` metres of it, 0 beyond `none` (of the
    faces of `sets`, or all)."""
    from PIL import Image, ImageDraw
    X = array(P)
    line = np.array(sorted(lm.ring(FS_HEAD, MOUTH_SETS[0])), dtype=np.int64)
    line = X[line[np.argsort(X[line][:, 0])]]
    away = np.full(len(X), np.inf)
    head = np.array(sorted(set().union(*(lm.of_set[fs] for fs in HEAD_SETS))), dtype=np.int64)
    for a, b in zip(line[:-1], line[1:]):
        t = np.clip(((X[head] - a) @ (b - a)) / max(float((b - a) @ (b - a)), 1e-18), 0.0, 1.0)
        away[head] = np.minimum(away[head], np.linalg.norm(X[head] - (a + t[:, None] * (b - a)), axis=1))
    near = 1.0 - _step(full, none, away)
    img = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(img)
    layer = body.data.uv_layers[0].data
    of = face_sets(body) if sets is not None else None
    for poly in body.data.polygons:
        if sets is not None and of[poly.index] not in sets:
            continue
        value = float(np.mean([near[v] for v in poly.vertices]))
        if value <= 0.0:
            continue
        pts = [(layer[i].uv[0] * size, (1 - layer[i].uv[1]) * size) for i in poly.loop_indices]
        draw.polygon(pts, fill=int(round(255 * value)), outline=int(round(255 * value)))
    return _blur(np.asarray(img, dtype=np.float64) / 255.0, max(1, size // 700))


def _mouth_shade(body, P, lm, size):
    """What the mouth's lining is darkened by, over the texels: by how
    far each of its vertices is from the line the lips meet on
    (`MOUTH_SHADE`), smooth across its faces (they are a centimetre
    wide: one value a face showed as steps), 1 elsewhere; and its
    occlusion and how wet it is at the lips' edges (`LIP_EDGE`), alike."""
    X = array(P)
    line = np.array(sorted(lm.ring(FS_HEAD, MOUTH_SETS[0])), dtype=np.int64)
    line = X[line[np.argsort(X[line][:, 0])]]
    away = np.full(len(X), np.inf)
    bag = np.array(sorted(set().union(*(lm.of_set[fs] for fs in MOUTH_SETS))), dtype=np.int64)
    for a, b in zip(line[:-1], line[1:]):
        t = np.clip(((X[bag] - a) @ (b - a)) / max(float((b - a) @ (b - a)), 1e-18), 0.0, 1.0)
        away[bag] = np.minimum(away[bag], np.linalg.norm(X[bag] - (a + t[:, None] * (b - a)), axis=1))
    lit = np.ones(len(X))
    lit[bag] = np.interp(away[bag], [d for d, _ in MOUTH_SHADE], [v for _, v in MOUTH_SHADE])
    middle = line[len(line) // 2]
    off_axis = np.hypot(X[bag][:, 0] - middle[0], X[bag][:, 2] - (middle[2] - 0.004))
    throat = THROAT[1] + (1.0 - THROAT[1]) * _step(THROAT[2], THROAT[3], off_axis)
    lit[bag] *= np.where(away[bag] > THROAT[0], throat, 1.0)
    edge, wet = np.ones(len(X)), np.ones(len(X))
    edge[bag] = np.interp(away[bag], [d for d, _, _ in LIP_EDGE], [v for _, v, _ in LIP_EDGE])
    wet[bag] = np.interp(away[bag], [d for d, _, _ in LIP_EDGE], [(1.0 - r) / (1.0 - WET) for _, _, r in LIP_EDGE])
    return _over_faces(body, size, lit), _over_faces(body, size, edge), _over_faces(body, size, wet)


def _over_faces(body, size, lit):
    """Values of the mouth's lining's vertices (`lit`) over the texels,
    smooth across its faces; 1 elsewhere."""
    from PIL import Image, ImageDraw
    layer = body.data.uv_layers[0].data
    of = face_sets(body)
    # Each face's mean first, with its outline (the texels just outside
    # an island are sampled too), then the smooth values inside.
    flat = Image.new("L", (size, size), 255)
    draw = ImageDraw.Draw(flat)
    faces = [poly for poly in body.data.polygons if of[poly.index] in MOUTH_SETS]
    for poly in faces:
        value = int(round(255 * float(np.mean([lit[v] for v in poly.vertices]))))
        pts = [(layer[i].uv[0] * size, (1 - layer[i].uv[1]) * size) for i in poly.loop_indices]
        draw.polygon(pts, fill=value, outline=value)
    out = np.asarray(flat, dtype=np.float64) / 255.0
    out = np.where(_grow(out < 1.0, 2) & (out >= 1.0), _blur(np.where(out < 1.0, out, 0.0), 2)
                   / np.maximum(_blur((out < 1.0).astype(np.float64), 2), 1e-6), out)
    for poly in faces:
        loops = list(poly.loop_indices)
        uv = np.array([(layer[i].uv[0] * size, (1 - layer[i].uv[1]) * size) for i in loops])
        val = np.array([lit[v] for v in poly.vertices])
        for k in range(1, len(loops) - 1):
            tri, tv = uv[[0, k, k + 1]], val[[0, k, k + 1]]
            x0, y0 = np.floor(tri.min(axis=0)).astype(int)
            x1, y1 = np.ceil(tri.max(axis=0)).astype(int) + 1
            x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, size), min(y1, size)
            if x1 <= x0 or y1 <= y0:
                continue
            gx, gy = np.meshgrid(np.arange(x0, x1) + 0.5, np.arange(y0, y1) + 0.5)
            (ax, ay), (bx, by), (cx, cy) = tri
            det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
            if abs(det) < 1e-9:
                continue
            wa = ((by - cy) * (gx - cx) + (cx - bx) * (gy - cy)) / det
            wb = ((cy - ay) * (gx - cx) + (ax - cx) * (gy - cy)) / det
            wc = 1.0 - wa - wb
            # (A texel whose middle is just outside an edge is the
            # neighbouring face's, or the island's rim: filled too, or it
            # keeps the face's mean and draws the edge as a line.)
            inside = (wa >= -0.12) & (wb >= -0.12) & (wc >= -0.12)
            wa, wb, wc = np.clip(wa, 0.0, 1.0), np.clip(wb, 0.0, 1.0), np.clip(wc, 0.0, 1.0)
            patch = out[y0:y1, x0:x1]
            patch[inside] = ((wa * tv[0] + wb * tv[1] + wc * tv[2]) / np.maximum(wa + wb + wc, 1e-9))[inside]
    return out


# -- the fields over the body ---------------------------------------------------------

def fields(body, P, lm):
    """Per-vertex fields (0 to 1) the colour and roughness are made from:
    flush, lips, pale, nail, oily, dry."""
    X = array(P)
    n = len(X)
    me = body.data
    normals = np.array([v.normal[:] for v in me.vertices])
    of_set = {fs: np.array(sorted(idx), dtype=np.int64) for fs, idx in lm.of_set.items()}
    eyes = {s: np.array(fit_sphere(P[i] for i in lm.verts(FS_SOCKET[s]))[0]) for s in (1, -1)}
    joints = rig.fit_joints(P, lm, {s: Vector(c) for s, c in eyes.items()})
    J = {name: (np.array(h), np.array(t)) for name, (h, t, _) in joints.items()}

    def blob(centre, radius):
        d2 = ((X - np.asarray(centre)) ** 2).sum(axis=1)
        return np.exp(-d2 / (radius * radius))

    def on_skin(guess, among):
        """The vertex of `among` nearest `guess`."""
        return X[among[np.argmin(((X[among] - guess) ** 2).sum(axis=1))]]

    head = np.unique(np.concatenate([of_set[fs] for fs in HEAD_SETS]))
    front = head[X[head][:, 1] < J["Head"][0][1] - 0.05]
    flush = np.zeros(n)
    oily = np.zeros(n)
    dry = np.zeros(n)
    nose_tip = X[front[np.argmin(X[front][:, 1])]]
    flush += 0.8 * blob(nose_tip, 0.016)
    oily += blob(nose_tip, 0.022)
    muzzle = of_set[FS_MUZZLE]
    mid = muzzle[np.abs(X[muzzle][:, 0]) < 0.006]
    chin = X[mid[np.argmin(X[mid][:, 2])]]
    flush += 0.35 * blob(chin + np.array([0, 0, 0.012]), 0.02)
    oily += 0.6 * blob(chin + np.array([0, 0, 0.012]), 0.02)
    eye_z = eyes[1][2]
    brow = on_skin(np.array([0.0, nose_tip[1] + 0.02, eye_z + 0.035]), front)
    oily += 0.8 * blob(brow, 0.035)
    for side, s in SIDES:
        cheek = on_skin(np.array([s * 0.05, eyes[s][1] - 0.005, eye_z - 0.045]), front)
        flush += 0.9 * blob(cheek, 0.026)
        flush[of_set[FS_EAR[s]]] += 0.6
        for finger in ("Index", "Middle", "Ring", "Little", "Thumb"):
            for seg in ("Proximal", "Intermediate"):
                flush += 0.5 * blob(J[f"{side}{finger}{seg}"][0], 0.008)
                dry += 0.5 * blob(J[f"{side}{finger}{seg}"][0], 0.008)
        elbow = J[f"{side}LowerArm"][0]
        back = np.array([0.0, 0.035, 0.0])
        flush += 0.45 * blob(elbow + back, 0.035)
        dry += blob(elbow + back, 0.04)
        knee = J[f"{side}LowerLeg"][0] + np.array([0.0, -0.045, 0.02])
        flush += 0.4 * blob(knee, 0.045)
        dry += 0.8 * blob(knee, 0.05)
        heel = J[f"{side}Foot"][0] + np.array([0.0, 0.04, -0.05])
        dry += blob(heel, 0.04)
    # The lips: a lip's height at the middle and less toward the corners.
    # The lower lip is the muzzle set's skin below the line between the
    # mouth's corners. The upper lip is part of the face's set (the muzzle
    # set only reaches up to the corners): the face's skin above its own
    # lower edge, where the mouth's inside begins.
    corners = set(lm.ring(FS_MUZZLE, MOUTH_SETS[0]))
    edge = corners | set(lm.ring(FS_HEAD, MOUTH_SETS[0]))
    lips = np.zeros(n)
    height = float(X[:, 2].max() - X[:, 2].min())
    for part, line, upper in ((muzzle, corners, False), (of_set[FS_HEAD], edge, True)):
        if not line:
            continue
        line = np.array(sorted(line), dtype=np.int64)
        order = line[np.argsort(X[line][:, 0])]
        half = float(np.abs(X[line][:, 0]).max())
        at_x = X[part][:, 0]
        line_z = np.interp(at_x, X[order][:, 0], X[order][:, 2])
        line_y = np.interp(at_x, X[order][:, 0], X[order][:, 1])
        dz = X[part][:, 2] - line_z
        taper = np.clip(1.0 - (np.abs(at_x) / half) ** 2.2, 0.0, 1.0) ** 0.6
        if not upper:
            # (The lower lip's red thins sooner toward the corners: as
            # tall there as in the middle, it stood beside an open mouth
            # as a pink strip.)
            taper = np.clip(1.0 - (np.abs(at_x) / half) ** 1.9, 0.0, 1.0) ** 0.85
        tall = np.where(dz > 0, 0.0050, 0.0066) * height * taper + 0.0008
        on = 1.0 - _step(0.55, 1.0, np.abs(dz) / tall)
        on *= (np.abs(at_x) < half) & (X[part][:, 1] < line_y + 0.004)
        if upper:
            # Only the lip: above its edge, on the front of the face.
            on *= (dz > -0.0005) & (X[part][:, 1] > line_y - 0.02)
        lips[part] = np.maximum(lips[part], on)
    # Palms and soles: the skin facing the palm's side, and down under the foot.
    pale = np.zeros(n)
    nail = np.zeros(n)
    stylized = rig.stylized_rig()
    for side, s in SIDES:
        palm = np.array(stylized.palm_normal(joints, side, s))
        hand = np.unique(np.concatenate([of_set[FS_PALM[s]]] + [of_set[fs] for g in FS_FINGERS[s].values() for fs in g]))
        pale[hand] = np.clip((normals[hand] @ palm - 0.1) / 0.5, 0.0, 1.0)
        foot = np.unique(np.concatenate([of_set[FS_FOOT[s]], of_set[FS_TOE_BASE[s]]] + [of_set[fs] for fs in FS_TOES[s]]))
        pale[foot] = np.clip((-normals[foot][:, 2] - 0.3) / 0.4, 0.0, 1.0)
        dry[foot] += 0.5 * pale[foot]
        for g in FS_FINGERS[s].values():
            nail[of_set[g[3]]] = 1.0
    for fs in TOE_NAILS:
        nail[of_set[fs]] = 1.0
    clip = lambda a: np.clip(a, 0.0, 1.0)  # noqa: E731
    return {"flush": clip(flush), "lips": clip(lips), "pale": clip(pale), "nail": nail,
            "oily": clip(oily), "dry": clip(dry)}


def _joint_heads(P, lm):
    """The fitted skeleton's joints (their heads) on positions `P`."""
    eyes = {s: np.array(fit_sphere(P[i] for i in lm.verts(FS_SOCKET[s]))[0]) for s in (1, -1)}
    joints = rig.fit_joints(P, lm, {s: Vector(c) for s, c in eyes.items()})
    return {name: np.array(h) for name, (h, _, _) in joints.items()}


def navel(P, lm):
    """The navel's vertex: of the belly's midline vertices between the hips
    and the chest, the one lying furthest in behind the line through its
    midline neighbours above and below (the low mesh's own dent; the
    sculpt's is deeper). Front is -Y, up +Z."""
    X = array(P)
    J = _joint_heads(P, lm)
    mid = np.flatnonzero((np.abs(X[:, 0]) < 0.004) & (X[:, 1] < J["Spine"][1])
                         & (X[:, 2] > J["Hips"][2]) & (X[:, 2] < J["Chest"][2]))
    mid = mid[np.argsort(X[mid][:, 2])]
    z, y = X[mid][:, 2], X[mid][:, 1]
    behind = y[1:-1] - (y[:-2] + (y[2:] - y[:-2]) * (z[1:-1] - z[:-2]) / (z[2:] - z[:-2]))
    return int(mid[1 + int(np.argmax(behind))])


# How far round the navel (metres) the smooth map keeps the sculpt's forms
# whole (the navel and its rim), and where the flattening is full again.
NAVEL_KEPT = (0.02, 0.035)


def soft_tissue(P, lm):
    """Per vertex, 0 to 1: how much of the sculpt's forms there are soft
    tissue that fat covers (abdominal lines, the separation between
    muscles), for the smooth normal map. None on the head (down to the
    neck's middle), the hands and the feet, and less over the knees,
    elbows and nipples, whose forms stay on a heavy body (the navel is
    kept in the texels, by `bake`)."""
    X = array(P)
    of_set = {fs: np.array(sorted(idx), dtype=np.int64) for fs, idx in lm.of_set.items()}
    J = _joint_heads(P, lm)

    def blob(centre, radius):
        return np.exp(-((X - centre) ** 2).sum(axis=1) / (radius * radius))

    soft = np.ones(len(X))
    head = np.unique(np.concatenate([of_set[fs] for fs in HEAD_SETS]))
    soft[head] = _step(J["Head"][2], J["Neck"][2], X[head][:, 2])
    torso = of_set[FS_TORSO]
    for side, s in SIDES:
        ends = [of_set[FS_PALM[s]], of_set[FS_FOOT[s]], of_set[FS_TOE_BASE[s]]]
        ends += [of_set[fs] for g in FS_FINGERS[s].values() for fs in g]
        ends += [of_set[fs] for fs in FS_TOES[s]]
        soft[np.unique(np.concatenate(ends))] = 0.0
        soft *= 1.0 - blob(J[f"{side}LowerLeg"] + np.array([0.0, -0.04, 0.0]), 0.06)
        soft *= 1.0 - blob(J[f"{side}LowerArm"], 0.045)
        # The nipple: the chest's most forward point on this side, at the
        # height of the chest.
        on_side = torso[(s * X[torso][:, 0] > 0.05) & (s * X[torso][:, 0] < 0.16)
                        & (X[torso][:, 2] > J["Chest"][2]) & (X[torso][:, 2] < J["Neck"][2])]
        if len(on_side):
            soft *= 1.0 - blob(X[on_side[np.argmin(X[on_side][:, 1])]], 0.025)
    return np.clip(soft, 0.0, 1.0)


def _uv_of(obj, vertex):
    """`vertex`'s texture coordinates on `obj`, as glTF has them (v down),
    rounded to a texel of a 4K map."""
    layer = obj.data.uv_layers[0].data
    for loop in obj.data.loops:
        if loop.vertex_index == vertex:
            u, v = layer[loop.index].uv
            return [round(float(u), 4), round(1.0 - float(v), 4)]
    raise ValueError(f"vertex {vertex} has no face")


def _soft_texels(low, P, lm, size):
    """`soft_tissue` over the texels, blurred so it fades in."""
    soft = soft_tissue(P, lm)
    _attribute(low, "Soft", np.stack([soft, soft, soft], axis=1))
    img = _image("Soft", size)

    def emit(nt, bsdf):
        colour = nt.nodes.new("ShaderNodeVertexColor")
        colour.layer_name = "Soft"
        _emit(nt, colour.outputs["Color"])

    _target(low, img, emit)
    _bake("EMIT", low, size=size)
    return np.clip(_blur(_pixels(img)[:, :, 0], max(2, size // 256)), 0.0, 1.0)


# -- the bakes ---------------------------------------------------------------------

def bake(body, source, lm, size, out_dir, cage=None):
    """Bakes the texture set for `body` (which has its final texture
    coordinates) from the frames' vendored positions `source`. Returns
    {albedo, normal, orm}: image datablocks saved in `out_dir`. `cage`
    is the body on the bundle's own topology where `body` has more
    vertices than it (`rims.split`): the vendored sculpts are stored
    against that mesh subdivided, vertex by vertex."""
    hidden_sets = {*MOUTH_SETS, *FS_SOCKET.values()}
    hidden = _grow(_coverage(body, size, hidden_sets), 3)
    normals, occlusion = {}, {}
    lows = {}
    for frame in ("Masculine", "Feminine"):
        low = duplicate(body, f"Low{frame}")
        for vert, p in zip(low.data.vertices, source[frame]):
            vert.co = p
        low.data.update()
        shade_smooth(low)
        if cage is None:
            high = sculpt(low, frame)
        else:
            # (The bundle's vertices come first in `source`.)
            under = duplicate(cage, f"Cage{frame}")
            for vert, p in zip(under.data.vertices, source[frame]):
                vert.co = p
            under.data.update()
            shade_smooth(under)
            high = sculpt(under, frame)
            bpy.data.objects.remove(under)
        img = _image(f"Normal{frame}", size, (0.5, 0.5, 1.0, 1.0))
        _target(low, img)
        _target(high, _image("Unused", 8))
        _bake("NORMAL", low, high, samples=4, size=size)
        normals[frame] = _pixels(img) * 2.0 - 1.0
        world = bpy.context.scene.world or bpy.data.worlds.new("Bake")
        bpy.context.scene.world = world
        world.light_settings.distance = 0.02
        img = _image(f"Occlusion{frame}", size, (1.0, 1.0, 1.0, 1.0))
        # Baked on the sculpt itself, whose texture coordinates are the low
        # mesh's subdivided. Every other mesh lies against the sculpt and
        # would shade it, so they're hidden meanwhile.
        _target(high, img)
        others = [o for o in bpy.context.scene.objects if o is not high and not o.hide_render]
        for o in others:
            o.hide_render = True
        _bake("AO", high, samples=24, size=size)
        for o in others:
            o.hide_render = False
        occlusion[frame] = _pixels(img)[:, :, 0]
        bpy.data.objects.remove(high)
        lows[frame] = low
    low = lows["Masculine"]
    bpy.data.objects.remove(lows["Feminine"])

    # Where the two sculpts' normals part, by more than the bake's noise.
    a, b = normals["Masculine"], normals["Feminine"]
    flat = np.array([0.0, 0.0, 1.0])
    for one in (a, b):
        one /= np.maximum(np.linalg.norm(one, axis=2, keepdims=True), 1e-9)
        # In the deepest creases (the armpits, between the toes) a bake's
        # ray finds the far side of the fold and the normal comes out
        # lying on its side: nothing real leans past 60 degrees, so those
        # texels go back to the low mesh's own normal.
        trust = _step(0.35, 0.6, one[:, :, 2])[:, :, None]
        one *= trust
        one += flat * (1.0 - trust)
        one /= np.maximum(np.linalg.norm(one, axis=2, keepdims=True), 1e-9)
    apart = np.degrees(np.arccos(np.clip((a * b).sum(axis=2), -1.0, 1.0)))
    apart = _blur(apart, max(2, size // 400))
    lean = np.clip((apart - 3.0) / 7.0, 0.0, 1.0)
    lean = (lean * lean * (3 - 2 * lean))[:, :, None]
    mean = a + b
    mean /= np.maximum(np.linalg.norm(mean, axis=2, keepdims=True), 1e-9)
    flatter = np.where((a[:, :, 2] >= b[:, :, 2])[:, :, None], a, b)
    shared = mean * (1.0 - lean) + flatter * lean
    shared /= np.maximum(np.linalg.norm(shared, axis=2, keepdims=True), 1e-9)
    ao = np.maximum(occlusion["Masculine"], occlusion["Feminine"]) * lean[:, :, 0] + \
        (occlusion["Masculine"] + occlusion["Feminine"]) / 2 * (1.0 - lean[:, :, 0])
    ao = np.clip(ao, 0.0, 1.0)
    shared[hidden] = (0.0, 0.0, 1.0)
    ao[hidden] = 1.0
    # The lips were sculpted closed, and the bake shades where they touch
    # as a crease: on an open mouth that is a black line along the lips'
    # inner edges and a black notch in each corner. Most of it is taken
    # out (a closed mouth's line is drawn by the lips themselves).
    seam = _near_lips(body, source["Masculine"], lm, size, 0.004, 0.009)
    ao = ao + (np.maximum(ao, LIP_SEAM_OCCLUSION) - ao) * seam

    # Skin grain: a bump texture in the body's own space, baked to a normal map.
    grain = _image("Grain", size, (0.5, 0.5, 1.0, 1.0))
    P = source["Masculine"]
    f = fields(low, P, lm)
    _attribute(low, "FieldsA", np.stack([f["flush"], f["lips"], f["pale"]], axis=1))
    _attribute(low, "FieldsB", np.stack([f["nail"], f["oily"], f["dry"]], axis=1))
    texel = 1.0 / _texels_per_metre(low, size)   # metres a texel covers

    def bump(nt, bsdf):
        coords = nt.nodes.new("ShaderNodeTexCoord")
        cells = nt.nodes.new("ShaderNodeTexVoronoi")
        cells.feature = "DISTANCE_TO_EDGE"
        cells.inputs["Scale"].default_value = 1.0 / (GRAIN_CELL * texel)
        nt.links.new(coords.outputs["Object"], cells.inputs["Vector"])
        fine = nt.nodes.new("ShaderNodeTexNoise")
        fine.inputs["Scale"].default_value = 1.0 / (GRAIN_FINE * texel)
        fine.inputs["Detail"].default_value = 3.0
        nt.links.new(coords.outputs["Object"], fine.inputs["Vector"])
        # Grooves between the cells, on a gentler wrinkling.
        ramp = nt.nodes.new("ShaderNodeMapRange")
        ramp.inputs["From Min"].default_value = 0.0
        ramp.inputs["From Max"].default_value = 0.12
        nt.links.new(cells.outputs["Distance"], ramp.inputs["Value"])
        mix = nt.nodes.new("ShaderNodeMath")
        mix.operation = "MULTIPLY_ADD"
        mix.inputs[1].default_value = 0.6
        nt.links.new(fine.outputs["Fac"], mix.inputs[0])
        nt.links.new(ramp.outputs["Result"], mix.inputs[2])
        fields_a = nt.nodes.new("ShaderNodeVertexColor")
        fields_a.layer_name = "FieldsA"
        fields_b = nt.nodes.new("ShaderNodeVertexColor")
        fields_b.layer_name = "FieldsB"
        sep_a = nt.nodes.new("ShaderNodeSeparateColor")
        sep_b = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(fields_a.outputs["Color"], sep_a.inputs["Color"])
        nt.links.new(fields_b.outputs["Color"], sep_b.inputs["Color"])
        # Lips and nails are smooth.
        smooth = nt.nodes.new("ShaderNodeMath")
        smooth.operation = "MAXIMUM"
        nt.links.new(sep_a.outputs["Green"], smooth.inputs[0])
        nt.links.new(sep_b.outputs["Red"], smooth.inputs[1])
        strength = nt.nodes.new("ShaderNodeMapRange")
        strength.inputs["To Min"].default_value = 1.0
        strength.inputs["To Max"].default_value = 0.25
        nt.links.new(smooth.outputs["Value"], strength.inputs["Value"])
        node = nt.nodes.new("ShaderNodeBump")
        node.inputs["Distance"].default_value = GRAIN_HEIGHT * texel
        nt.links.new(strength.outputs["Result"], node.inputs["Strength"])
        nt.links.new(mix.outputs["Value"], node.inputs["Height"])
        nt.links.new(node.outputs["Normal"], bsdf.inputs["Normal"])

    _target(low, grain, bump)
    _bake("NORMAL", low, samples=4, size=size)
    g = _pixels(grain) * 2.0 - 1.0
    g[hidden] = (0.0, 0.0, 1.0)
    # The smooth map: the sculpt's forms flattened over soft tissue, but
    # not in deep creases (the fold under a breast, the buttocks'), which
    # fat deepens rather than hides.
    soft = _soft_texels(low, P, lm, size) * _step(0.80, 0.92, ao)
    # Nor round the navel, a crease the occlusion doesn't keep (only its
    # middle is deep, not its rim): measured in the texels, as the low
    # mesh's vertices round it are 2 to 4 cm apart.
    navel_uv = _uv_of(low, navel(P, lm))
    rows, cols = np.mgrid[0:size, 0:size]
    from_navel = np.hypot(cols + 0.5 - navel_uv[0] * size, rows + 0.5 - navel_uv[1] * size) * texel
    soft *= _step(*NAVEL_KEPT, from_navel)
    flat_share = (SMOOTHED * soft)[:, :, None]
    smooth = shared * (1.0 - flat_share) + flat * flat_share
    smooth /= np.maximum(np.linalg.norm(smooth, axis=2, keepdims=True), 1e-9)

    def grained(forms):
        """The grain over `forms` (both in tangent space)."""
        out = np.stack([forms[:, :, 0] + g[:, :, 0], forms[:, :, 1] + g[:, :, 1],
                        forms[:, :, 2] * np.maximum(g[:, :, 2], 0.2)], axis=2)
        return out / np.maximum(np.linalg.norm(out, axis=2, keepdims=True), 1e-9)

    normal = grained(smooth)
    defined = grained(shared)

    # Colour and roughness from the fields, mottled in the body's space.
    colour = _image("Colour", size)
    tone = srgb_to_linear(TONE)

    def paint(nt, bsdf, rough=False):
        coords = nt.nodes.new("ShaderNodeTexCoord")
        fields_a = nt.nodes.new("ShaderNodeVertexColor")
        fields_a.layer_name = "FieldsA"
        fields_b = nt.nodes.new("ShaderNodeVertexColor")
        fields_b.layer_name = "FieldsB"
        sep_a = nt.nodes.new("ShaderNodeSeparateColor")
        sep_b = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(fields_a.outputs["Color"], sep_a.inputs["Color"])
        nt.links.new(fields_b.outputs["Color"], sep_b.inputs["Color"])

        def noise(scale, detail=2.0):
            node = nt.nodes.new("ShaderNodeTexNoise")
            node.inputs["Scale"].default_value = scale
            node.inputs["Detail"].default_value = detail
            nt.links.new(coords.outputs["Object"], node.inputs["Vector"])
            return node.outputs["Fac"]

        def mix(a, b, factor, kind="MIX"):
            node = nt.nodes.new("ShaderNodeMix")
            node.data_type = "RGBA"
            node.blend_type = kind
            for socket, value in ((node.inputs[6], a), (node.inputs[7], b), (node.inputs[0], factor)):
                if isinstance(value, (tuple, list, np.ndarray)):
                    socket.default_value = (*[float(x) for x in value], 1.0)
                elif isinstance(value, float):
                    socket.default_value = value
                else:
                    nt.links.new(value, socket)
            return node.outputs[2]

        def scaled(value, lo, hi):
            node = nt.nodes.new("ShaderNodeMapRange")
            node.inputs["From Min"].default_value = 0.3
            node.inputs["From Max"].default_value = 0.7
            node.inputs["To Min"].default_value = lo
            node.inputs["To Max"].default_value = hi
            nt.links.new(value, node.inputs["Value"])
            return node.outputs["Result"]

        if rough:
            value = nt.nodes.new("ShaderNodeMath")
            value.operation = "MULTIPLY_ADD"       # oily lowers it
            value.inputs[1].default_value = -0.13
            value.inputs[2].default_value = ROUGHNESS
            nt.links.new(sep_b.outputs["Green"], value.inputs[0])
            dry = nt.nodes.new("ShaderNodeMath")
            dry.operation = "MULTIPLY_ADD"
            dry.inputs[1].default_value = 0.14
            nt.links.new(sep_b.outputs["Blue"], dry.inputs[0])
            nt.links.new(value.outputs["Value"], dry.inputs[2])
            vary = nt.nodes.new("ShaderNodeMath")
            vary.operation = "ADD"
            nt.links.new(dry.outputs["Value"], vary.inputs[0])
            nt.links.new(scaled(noise(1.0 / (6.0 * texel), 3.0), -0.05, 0.05), vary.inputs[1])
            grey = nt.nodes.new("ShaderNodeCombineColor")
            for k in range(3):
                nt.links.new(vary.outputs["Value"], grey.inputs[k])
            out = mix(grey.outputs["Color"], (0.30, 0.30, 0.30), sep_a.outputs["Green"])   # lips
            out = mix(out, (0.28, 0.28, 0.28), sep_b.outputs["Red"])                        # nails
            _emit(nt, out)
            return
        out = mix(tuple(tone), tuple(tone * np.array(FLUSH)), sep_a.outputs["Red"])
        out = mix(out, tuple(tone * np.array(PALE)), sep_a.outputs["Blue"])
        # Mottling: warm and cool patches a hand wide, blotches a finger wide, fine speckle.
        out = mix(out, (1.08, 0.93, 0.91), scaled(noise(9.0), 0.0, 1.0), "MULTIPLY")
        out = mix(out, (0.91, 0.90, 0.92), scaled(noise(70.0), 0.0, 1.0), "MULTIPLY")
        out = mix(out, (0.93, 0.92, 0.92), scaled(noise(1.0 / (5.0 * texel), 3.0), 0.0, 1.0), "MULTIPLY")
        out = mix(out, tuple(srgb_to_linear(LIPS)), sep_a.outputs["Green"])
        out = mix(out, tuple(srgb_to_linear(NAIL)), sep_b.outputs["Red"])
        _emit(nt, out)

    _target(low, colour, paint)
    _bake("EMIT", low, size=size)
    albedo = _pixels(colour)
    rough_img = _image("Roughness", size)
    _target(low, rough_img, lambda nt, bsdf: paint(nt, bsdf, rough=True))
    _bake("EMIT", low, size=size)
    roughness = _pixels(rough_img)[:, :, 0]

    # Creases a little darker and redder; the hidden insides are flesh.
    crease = (1.0 - ao)[:, :, None]
    albedo = albedo * (1.0 - crease * np.array([0.30, 0.42, 0.45]))
    albedo[hidden] = srgb_to_linear(FLESH)
    roughness[hidden] = WET
    # The lips' inner sides are the lips' colour at their edges and darken
    # into the mouth: flesh from the very edge drew a dark line round an
    # open mouth.
    inner = _near_lips(body, source["Masculine"], lm, size, 0.002, 0.013, set(MOUTH_SETS))[:, :, None]
    inner = np.where(hidden[:, :, None], inner, 0.0)
    albedo = albedo * (1.0 - inner) + srgb_to_linear(INNER_LIP) * inner
    # And the mouth is dark inside: the lips and the cheeks shade it, and
    # the engine lights it like the face. The shade is in the colour.
    # (Divided between the colour and the occlusion, which takes the
    # sky's light and its reflection off: `teeth.IN_COLOUR`.)
    from teeth import IN_COLOUR
    shade, edge, wet = _mouth_shade(body, source["Masculine"], lm, size)
    albedo = albedo * (shade ** IN_COLOUR)[:, :, None]
    ao = ao * shade ** LINING_OCCLUDES * edge
    roughness = np.where(hidden, 1.0 - (1.0 - WET) * wet, roughness)

    from PIL import Image
    paths = [os.path.join(out_dir, name) for name in FILES]
    Image.fromarray((linear_to_srgb(albedo) * 255 + 0.5).astype(np.uint8)).save(paths[0], quality=95, subsampling=0)
    Image.fromarray(((normal * 0.5 + 0.5) * 255 + 0.5).astype(np.uint8)).save(paths[1], optimize=True)
    Image.fromarray(((defined * 0.5 + 0.5) * 255 + 0.5).astype(np.uint8)).save(paths[3], optimize=True)
    orm = np.stack([ao, np.clip(roughness, 0.05, 1.0), np.zeros_like(ao)], axis=2)
    Image.fromarray((orm * 255 + 0.5).astype(np.uint8)).save(paths[2], quality=95, subsampling=0)
    bpy.data.objects.remove(low)
    images = {}
    for key, path, space in zip(("albedo", "normal", "orm"), paths[:3], ("sRGB", "Non-Color", "Non-Color")):
        img = bpy.data.images.load(path)
        img.colorspace_settings.name = space
        images[key] = img
    images["report"] = {
        "size": size, "texels_per_cm": round(1.0 / texel / 100.0, 2),
        "frames_differ_share": round(float((lean[:, :, 0] > 0.5).mean()), 3),
        "smoothed_share": round(float((soft > 0.5).mean()), 3),
        # Where the navel is (glTF texture coordinates), which the smooth
        # map keeps: the assets crate's test checks the two maps agree there.
        "navel_uv": navel_uv,
        "grain": {"cell_texels": GRAIN_CELL, "fine_texels": GRAIN_FINE, "height_texels": GRAIN_HEIGHT},
    }
    return images


def _texels_per_metre(body, size=1.0):
    """The body's median texel density per unit of texture size: texels
    along a metre of skin, for a texture one texel wide."""
    me = body.data
    layer = me.uv_layers[0].data
    scales, areas = [], []
    for poly in me.polygons:
        pts = [layer[i].uv for i in poly.loop_indices]
        area_uv = abs(sum(pts[i].cross(pts[(i + 1) % len(pts)]) for i in range(len(pts)))) / 2
        if poly.area > 1e-10 and area_uv > 1e-12:
            scales.append((area_uv / poly.area) ** 0.5)
            areas.append(poly.area)
    order = np.argsort(scales)
    cum = np.cumsum(np.array(areas)[order]) / sum(areas)
    return float(np.array(scales)[order][np.searchsorted(cum, 0.5)]) * size


# -- materials -----------------------------------------------------------------------

def _gltf_output():
    """The node group Blender's glTF exporter reads occlusion from."""
    name = "glTF Material Output"
    group = bpy.data.node_groups.get(name)
    if group is None:
        group = bpy.data.node_groups.new(name, "ShaderNodeTree")
        group.interface.new_socket("Occlusion", in_out="INPUT", socket_type="NodeSocketFloat")
        group.nodes.new("NodeGroupInput")
    return group


def material(name, maps, roughness=None, colour=None):
    """A glTF PBR material: the texture set (or, without one, `colour`),
    with a fixed `roughness` in place of the map's if given."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Metallic"].default_value = 0.0
    if maps is None:
        bsdf.inputs["Base Color"].default_value = (*srgb_to_linear(colour), 1.0)
        bsdf.inputs["Roughness"].default_value = roughness or ROUGHNESS
        return m
    base = nt.nodes.new("ShaderNodeTexImage")
    base.image = maps["albedo"]
    nt.links.new(base.outputs["Color"], bsdf.inputs["Base Color"])
    if roughness is not None:
        bsdf.inputs["Roughness"].default_value = roughness
        return m
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = maps["normal"]
    bump = nt.nodes.new("ShaderNodeNormalMap")
    nt.links.new(tex.outputs["Color"], bump.inputs["Color"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    orm = nt.nodes.new("ShaderNodeTexImage")
    orm.image = maps["orm"]
    split = nt.nodes.new("ShaderNodeSeparateColor")
    nt.links.new(orm.outputs["Color"], split.inputs["Color"])
    nt.links.new(split.outputs["Green"], bsdf.inputs["Roughness"])
    nt.links.new(split.outputs["Blue"], bsdf.inputs["Metallic"])
    out = nt.nodes.new("ShaderNodeGroup")
    out.node_tree = _gltf_output()
    nt.links.new(split.outputs["Red"], out.inputs["Occlusion"])
    return m


def materials(body, sets, maps):
    """`Skin` on the body, `Mouth` on the inside of the mouth and the eye
    sockets (the same albedo, wet, no detail maps). `sets` is each
    polygon's face set (the materials it replaces held them)."""
    hidden = {*MOUTH_SETS, *FS_SOCKET.values()}
    skin = material("Skin", maps, colour=TONE)
    # (The whole set of maps on the lining too: its texels are flat in
    # the normal map, and the mouth's shade is in the occlusion and its
    # wetness in the roughness: `bake`.)
    mouth = material("Mouth", maps, roughness=None if maps else WET, colour=FLESH)
    body.data.materials.clear()
    body.data.materials.append(skin)
    body.data.materials.append(mouth)
    for poly, fs in zip(body.data.polygons, sets):
        poly.material_index = 1 if fs in hidden else 0


def mark(glb, out_dir):
    """Marks the `Skin` material of the exported file `glb` as skin
    (`extras.axcenity_shading`) and adds the defined normal map beside its
    own, smooth one (`extras.axcenity_defined_normal`: a texture placed
    like the normal map; `bake` wrote it to `out_dir`). Blender's exporter
    writes only the images a material's nodes use, so it is added to the
    file here."""
    import json
    import struct
    with open(glb, "rb") as f:
        data = f.read()
    _, version, _ = struct.unpack_from("<III", data, 0)
    chunks, at = [], 12
    while at < len(data):
        length, kind = struct.unpack_from("<II", data, at)
        chunks.append([kind, data[at + 8:at + 8 + length]])
        at += 8 + length
    doc = json.loads(chunks[0][1])
    binary = bytearray(chunks[1][1]) if len(chunks) > 1 else bytearray()
    skin = next(m for m in doc["materials"] if m.get("name") == "Skin")
    extras = dict(skin.get("extras", {}), axcenity_shading="skin")
    path = os.path.join(out_dir, FILES[3])
    if "normalTexture" in skin and os.path.exists(path):
        with open(path, "rb") as f:
            png = f.read()
        binary.extend(b"\0" * (-len(binary) % 4))
        doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(png)})
        binary.extend(png)
        doc["images"].append({"bufferView": len(doc["bufferViews"]) - 1, "mimeType": "image/png",
                              "name": os.path.splitext(FILES[3])[0]})
        own = doc["textures"][skin["normalTexture"]["index"]]
        doc["textures"].append(dict(own, source=len(doc["images"]) - 1))
        extras["axcenity_defined_normal"] = {"index": len(doc["textures"]) - 1}
    skin["extras"] = extras
    binary.extend(b"\0" * (-len(binary) % 4))
    doc["buffers"][0]["byteLength"] = len(binary)
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    out = struct.pack("<II", len(text), 0x4E4F534A) + text + struct.pack("<II", len(binary), 0x004E4942) + binary
    with open(glb, "wb") as f:
        f.write(struct.pack("<III", 0x46546C67, version, 12 + len(out)) + out)


def clean_up(out_dir):
    """Removes the texture files the build left beside the model (they
    are inside the glTF file)."""
    for name in FILES:
        path = os.path.join(out_dir, name)
        if os.path.exists(path):
            os.remove(path)


del edge_neighbours, select_only
