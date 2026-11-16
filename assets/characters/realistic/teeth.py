"""The jaw of the realistic head: the bundle's teeth, gums and palate made
ready for a mouth (`jaw_mesh`), and the shading of the mouth's inside,
which the engine has no light model for (`paint`, `palettes`).

**Size.** The bundle's jaw is a sculpting base and large for its head: at
the body's scale its upper central incisors are 10.5 mm wide, the lower
6.3 mm, the first molars 12.7 and 12.9 mm, and the arch is 71 mm across
the second molars' outer faces and 56 mm deep. Measured adults (means of
dental-cast studies; the sources are in the CO6 notes): upper central
8.4 to 9.0 mm, lower central 5.4 to 5.7 mm, first molars 10.0 to 10.6 and
10.8 to 11.4 mm, 56 to 58 mm between the upper second molars' buccal
cusps and 44 mm from the incisors to the second molars' distal cusps.
`SIZE` brings the jaw to that.

**The smile's arc.** The bundle's upper canines hang 0.4 mm under the
central incisors and 1 mm under the laterals, which reads as fangs once
a smile bares them; `ARC` lifts the canines and premolars so the edges
of the upper row rise evenly from the middle to the back.

**What is kept.** Each tooth is a closed cage of 105 vertices, made to
be subdivided. The roots buried in the gums are removed, and the gum
that runs on inside each tooth. The six front teeth of each jaw are
subdivided once; the others are drawn as their cages. The lower gum,
which in the bundle is the front of a whole lower jaw and the floor of
the mouth, ends 9 mm under the teeth and is closed there
(`_close_lower_gum`). Only the gums away from the teeth, the hidden
shell over the palate and the lower front teeth are thinned: a collapse
across the whole jaw is what made the teeth faceted. `LEVELS` gives the
same for the two simpler copies, which the build makes itself: left to
the engine, the copy a picture of a whole face draws would be the
teeth's triangles halved wherever they fall.

**Shading.** A mouth is dark inside because the lips and cheeks shade
it, and the engine lights it like the face. So the shade is baked:
every vertex gets a place in a small palette image (its texture
coordinates), one per material, whose one axis is the material's colour
ramp and whose other runs from lit to nearly black, with an occlusion
image laid out alike. A vertex's shade is what would reach it: less
between neighbouring teeth, at the gum's edge, on the sides that face
into the mouth, on the lower jaw, and steadily less from the incisors
back to the throat. The mouth's lining, which is the body's, is shaded
the same way in the skin's maps (`skin._mouth_shade`).
"""

import os

import bmesh
import bpy
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

from common import array, import_obj, new_object, select_only, subdivide

# The jaw's face sets: the gums, the palate, the floor with the tongue;
# every other set is a tooth.
FS_UPPER_GUM, FS_LOWER_GUM, FS_PALATE, FS_FLOOR = 93, 155, 157, 31
# The jaw against the bundle's (at the body's scale).
SIZE = 0.85
# How far each upper tooth is lifted, from the central incisor back to
# the second molar (metres at the bundle's size).
ARC = (0.0, 0.0, 0.0010, 0.0007, 0.0004, 0.0, 0.0)
# The gums further than `KEEP` from the crown of a tooth before rank
# `MOLAR`, and the palate, are thinned to `THIN` of their faces.
MOLAR = 5
KEEP = 0.004
THIN = 0.42
# Teeth up to rank `FINE` (the incisors and canines) are subdivided once.
FINE = 2
# The object and its two simpler copies, each: (the share of their faces
# the subdivided upper front teeth keep, or 0 for no subdividing: all,
# where a face is looked at, since thinning chips their edges; the share
# the subdivided lower front teeth keep; the share the other teeth's
# cages keep; the share the thinned gums keep (under `THIN`: all of the
# gums are thinned); whether the hidden shell over the palate is there,
# which only the first needs: the lips are kept off it). The first copy
# is what a picture of a whole face draws, so its upper teeth and the
# gums round the teeth are the object's own (its gums thinned whole, the
# gum's edge stepped across every tooth).
LEVELS = ((1.0, 0.45, 1.0, THIN, True), (1.0, 0.45, 0.8, THIN, False), (0.0, 0.0, 0.45, 0.15, False))
# A tooth's faces wholly this far past where its crown ends in the gum are
# its root: removed.
ROOT = 0.0015
# The lower gum is closed along a keel this far under the middle of the
# lower teeth (their roots' middle: some 9 mm under the gum's edge).
KEEL = 0.010
# Inside the row of lower teeth the gum is kept this far from the keel.
FLOOR = 0.008
# Where on the teeth's ramp a tooth's own colours begin: before it is a
# tooth in the mouth's shade, the dark red that light off the mouth's
# walls leaves it. The lower back teeth are this (`LOWER_IN_SHADE`: how
# much of it each lower tooth is, from the central incisor back to the
# second molar), lit `SHADE_LIT`. Shade alone could not do it: a vertex's shade
# is its colour's and its occlusion's at once, and with the occlusion
# low enough to darken a white tooth the engine drew the lower molars
# black, a row of black blocks in each corner of an open mouth against
# the dark red round them, whatever their colour.
IN_SHADE = 0.25
LOWER_IN_SHADE = (0.0, 0.0, 0.0, 0.3, 0.8, 1.0, 1.0)
SHADE_LIT = 0.40
# The colour at no light, and what the colours are multiplied by toward
# the dark end.
DARK = (0.050, 0.022, 0.024)
WARM = (1.0, 0.62, 0.56)
# material -> (file, the ramp's colours (sRGB) along u)
PALETTES = {
    # a tooth in the mouth's own shade (`IN_SHADE`), the neck by the gum,
    # the enamel, the thin edge
    "Teeth": ("RealisticTeeth.png", ((0.0, (0.70, 0.29, 0.29)), (IN_SHADE, (0.85, 0.76, 0.58)),
                                     (0.5 + 0.5 * IN_SHADE, (0.96, 0.94, 0.87)), (1.0, (0.90, 0.92, 0.93)))),
    # the gum's edge, the firm gum by the teeth, the redder lining beyond
    "Gums": ("RealisticGums.png", ((0.0, (0.58, 0.27, 0.29)), (0.35, (0.74, 0.42, 0.42)), (1.0, (0.55, 0.21, 0.25)))),
    # the groove down its middle, its back
    "Tongue": ("RealisticTongue.png", ((0.0, (0.48, 0.21, 0.23)), (1.0, (0.64, 0.31, 0.33)))),
}
# The same shade as occlusion (one image for the three materials): the
# engine takes the sky's light and its reflection off by it, which in the
# dark of a mouth is most of what a glossy tooth would otherwise show.
OCCLUSION = "RealisticMouthOcclusion.png"
IMAGES = (*(file for file, _ in PALETTES.values()), OCCLUSION)
PALETTE_SIZE = 64
# A palette's edge texels are left unused (a filtered lookup never reaches them).
INSET = 0.08
# How much light reaches the teeth by how far back they stand from the
# incisors' front (metres at the bundle's size): the incisors are lit,
# the canines a little less, the premolars are in the cheeks' shade and
# the molars nearly dark.
DEPTH = ((0.0, 1.0), (0.004, 0.97), (0.009, 0.84), (0.016, 0.60), (0.024, 0.38), (0.032, 0.21), (0.043, 0.10),
         (0.060, 0.04))
# (The lower teeth stand behind the lower lip and under the upper row:
# less reaches them, and less still at the sides.)
DEPTH_LOWER = ((0.0, 0.85), (0.004, 0.80), (0.009, 0.55), (0.016, 0.30), (0.024, 0.18), (0.032, 0.10), (0.043, 0.06),
               (0.060, 0.03))
# A shade is divided between the colour (to the power `IN_COLOUR`: all
# of a lamp's light is dimmed by it) and the occlusion (`IN_OCCLUSION`:
# the sky's light and its reflection are dimmed by both). The engine's
# own shadows and screen-space occlusion darken a mouth further.
IN_COLOUR = 0.5
IN_OCCLUSION = 0.8
# The gums and the palate by how far back they are: they are the mouth's
# dark red inside, which goes to near black only at the throat.
DEPTH_GUMS = ((0.0, 1.0), (0.009, 0.9), (0.016, 0.75), (0.024, 0.6), (0.032, 0.46), (0.043, 0.32), (0.060, 0.14))


def srgb_to_linear(c):
    c = np.asarray(c, dtype=np.float64)
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c):
    c = np.clip(np.asarray(c, dtype=np.float64), 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def step(a, b, x):
    t = np.clip((np.asarray(x, dtype=np.float64) - a) / (b - a), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def palettes(out_dir):
    """The palette images, {material: image}: across, the material's
    colours; up, from no light to full light (in linear light, so a
    vertex's shade multiplies its colour)."""
    from PIL import Image
    out = {}
    n = PALETTE_SIZE
    place = np.clip(((np.arange(n) + 0.5) / n - INSET) / (1.0 - 2.0 * INSET), 0.0, 1.0)
    dark = srgb_to_linear(DARK)
    for name, (file, stops) in PALETTES.items():
        at = [s[0] for s in stops]
        ramp = np.stack([np.interp(place, at, [float(srgb_to_linear(s[1][c])) for s in stops]) for c in range(3)], axis=1)
        lit = (place ** IN_COLOUR)[:, None, None]        # rows: 0 dark ... 1 lit (v up)
        # (What light there is deep in a mouth has come off its red walls.)
        warm = 1.0 - (1.0 - np.array(WARM))[None, None, :] * (1.0 - lit) ** 2
        rgb = dark[None, None, :] * (1.0 - lit) + ramp[None, :, :] * lit * warm
        path = os.path.join(out_dir, file)
        Image.fromarray((linear_to_srgb(rgb)[::-1] * 255 + 0.5).astype(np.uint8)).save(path, optimize=True)
        img = bpy.data.images.load(path)
        img.colorspace_settings.name = "sRGB"
        out[name] = img
    lit = np.repeat((place ** IN_OCCLUSION)[::-1, None], n, axis=1)
    path = os.path.join(out_dir, OCCLUSION)
    Image.fromarray(np.stack([(lit * 255 + 0.5).astype(np.uint8)] * 3, axis=2)).save(path, optimize=True)
    img = bpy.data.images.load(path)
    img.colorspace_settings.name = "Non-Color"
    out["occlusion"] = img
    return out


def place(ramp, shade):
    """Texture coordinates for a place on the ramp (0 to 1) at a shade
    (0 dark to 1 lit)."""
    span = 1.0 - 2.0 * INSET
    return np.stack([INSET + span * np.clip(ramp, 0.0, 1.0), INSET + span * np.clip(shade, 0.0, 1.0)], axis=1)


def depth_shade(y, ks, upper):
    """The light left `y` behind the incisors' front (jaw space), on the
    upper jaw or the lower."""
    y = np.asarray(y) / ks
    return np.where(upper, np.interp(y, [d[0] for d in DEPTH], [d[1] for d in DEPTH]),
                    np.interp(y, [d[0] for d in DEPTH_LOWER], [d[1] for d in DEPTH_LOWER]))


def jaw_mesh(ks, level=0):
    """The bundle's jaw about the incisors' front on the plane the teeth
    meet on (y back, z up), at `ks` of the bundle's size: (object, what
    each vertex is), at `level` of detail (`LEVELS`). Kept: the teeth,
    both gums and the palate above the teeth; the upper half is cut free
    of the lower. The object's materials are still the bundle's face
    sets (`parts._jaw_object` replaces them) and its texture
    coordinates are each vertex's shade (`paint`).

    What each vertex is: `upper` (on the head, not the jaw), `tooth`
    (its rank from the central incisor 0 to the second molar 6, or -1
    for the gums and palate), `piece` (one number per tooth, -1
    otherwise), `palate`."""
    round_keeps, lower_keeps, cage_keeps, thin_keeps, shell = LEVELS[level]
    (jaw,) = import_obj("teeth.obj")
    me = jaw.data
    ids = [int(m.name.split("_")[1].split(".")[0]) for m in me.materials]
    sets = [ids[p.material_index] for p in me.polygons]
    co = array(v.co for v in me.vertices)
    soft = {FS_UPPER_GUM, FS_LOWER_GUM, FS_PALATE, FS_FLOOR}
    teeth_sets = sorted(set(sets) - soft)
    centres = {fs: np.mean([co[i] for p, s in zip(me.polygons, sets) if s == fs for i in p.vertices], axis=0)
               for fs in teeth_sets}
    bite = float(np.median([c[2] for c in centres.values()]))
    upper_sets = {fs for fs, c in centres.items() if c[2] > bite} | {FS_UPPER_GUM, FS_PALATE}
    rank = {}
    for up in (True, False):
        for side in (1, -1):
            row = sorted((fs for fs in teeth_sets if (fs in upper_sets) == up and centres[fs][0] * side > 0),
                         key=lambda fs: abs(centres[fs][0]))
            rank.update({fs: r for r, fs in enumerate(row)})
    verts_of = {}
    for p, s in zip(me.polygons, sets):
        verts_of.setdefault(s, set()).update(p.vertices)
    tooth_verts = sorted(set().union(*(verts_of[fs] for fs in teeth_sets)))
    front = float(co[tooth_verts][:, 1].min())
    co = (co - np.array([0.0, front, bite])) * ks
    for fs in teeth_sets:
        if fs in upper_sets:
            co[sorted(verts_of[fs]), 2] += ARC[rank[fs]] * ks
    # The gums and the palate run on four centimetres behind the last
    # teeth (the throat, for sculpting): cut off there, they'd come
    # through the skin under the ear when the jaw opens. The bundle's
    # floor and the shell under the lower gum would have to stretch from
    # the head to the jaw: the body's own lining is the floor instead.
    back = float(co[tooth_verts][:, 1].max()) + 0.004 * ks
    bm = bmesh.new()
    bm.from_mesh(me)
    for v, p in zip(bm.verts, co):
        v.co = p
    bm.faces.ensure_lookup_table()
    centre = [f.calc_center_median() for f in bm.faces]
    gone = [f for f, c, s in zip(bm.faces, centre, sets)
            if c.y > back or s == FS_FLOOR or (s == FS_PALATE and (c.z < 0.002 * ks or not shell))]
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    index_of = {i: k for k, i in enumerate(ids)}
    fs_of = lambda f: ids[f.material_index]  # noqa: E731
    # The gum is one sheet that runs on through every tooth: what of it
    # lies inside a tooth (while the teeth are still closed cages) is
    # never seen, and is in the way of telling how far up its crown a
    # point of a tooth is. Removed once the lower gum is closed.
    bm.verts.ensure_lookup_table()
    bm.verts.index_update()
    at = [tuple(v.co) for v in bm.verts]
    cages = {}
    for f in bm.faces:
        if fs_of(f) not in soft:
            cages.setdefault(fs_of(f), []).append([v.index for v in f.verts])
    cages = {fs: (BVHTree.FromPolygons(at, faces), np.array([at[i] for face in faces for i in face]))
             for fs, faces in cages.items()}

    def in_tooth(p):
        # (Asked of the point's image on one side: the jaw is the same on
        # both, and a point on a cage's very surface must fall alike.)
        p = Vector((abs(p[0]), p[1], p[2]))
        for tree, pts in cages.values():
            if np.all(np.array(p) > pts.min(axis=0)) and np.all(np.array(p) < pts.max(axis=0)):
                votes = 0
                for way in (Vector((0.31, -0.93, 0.19)), Vector((-0.42, 0.37, 0.83)), Vector((0.77, 0.21, -0.60))):
                    way.normalize()
                    start, crossings = Vector(p), 0
                    for _ in range(16):
                        hit = tree.ray_cast(start, way, 1.0)
                        if hit[0] is None:
                            break
                        crossings += 1
                        start = hit[0] + way * 1e-6
                    votes += crossings % 2
                if votes >= 2:
                    return True
        return False

    buried = {}
    inside = [f for f in bm.faces if fs_of(f) in (FS_UPPER_GUM, FS_LOWER_GUM)
              and all(buried.setdefault(v.index, in_tooth(v.co)) for v in f.verts)]
    # The roots: a tooth's faces wholly inside the gum.
    bm.verts.ensure_lookup_table()
    bm.verts.index_update()
    gum = [f for f in bm.faces if fs_of(f) in (FS_UPPER_GUM, FS_LOWER_GUM)]
    tree = BVHTree.FromPolygons([tuple(v.co) for v in bm.verts], [[v.index for v in f.verts] for f in gum])

    def above_gum(p):
        p = Vector((abs(p[0]), p[1], p[2]))
        loc, normal, _, _ = tree.find_nearest(p, 0.03)
        return 1.0 if loc is None else (p - loc).dot(normal)

    # (By tooth: its crown ends at the furthest from its tip that still
    # stands out of the gum; "out of the gum" is only asked of its tip's
    # three quarters, as the lower gum is open underneath and a root's
    # end is in front of nothing.)
    roots = []
    by_tooth = {}
    for f in bm.faces:
        if fs_of(f) not in soft:
            by_tooth.setdefault(fs_of(f), []).append(f)
    for fs, faces in by_tooth.items():
        verts = {v for f in faces for v in f.verts}
        way = -1.0 if fs in upper_sets else 1.0            # toward the tip
        tip = max(v.co.z * way for v in verts)
        length = tip - min(v.co.z * way for v in verts)
        from_tip = {v: tip - v.co.z * way for v in verts}
        crown = max((d for v, d in from_tip.items() if d < 0.75 * length and above_gum(v.co) > 0.0), default=0.0)
        roots += [f for f in faces if all(from_tip[v] > crown + ROOT * ks for v in f.verts)]
    bmesh.ops.delete(bm, geom=roots, context="FACES")
    seam = [e for e in bm.edges if len({fs_of(f) in upper_sets for f in e.link_faces}) == 2]
    bmesh.ops.split_edges(bm, edges=seam)
    _close_lower_gum(bm, fs_of, {fs: (c - np.array([0.0, front, bite])) * ks
                                 for fs, c in centres.items() if fs not in upper_sets}, ks)
    bmesh.ops.delete(bm, geom=[f for f in inside if f.is_valid], context="FACES")
    # The front teeth's crowns are rounded: the bundle's teeth are cages
    # for subdividing, and a cage's corners show at a tooth's edge.
    fine = [f for f in bm.faces if fs_of(f) not in soft and rank[fs_of(f)] <= FINE]
    if fine and round_keeps:
        bm.faces.index_update()
        keep = {f.index for f in fine}
        part = bm.copy()
        bmesh.ops.delete(part, geom=[f for f in part.faces if f.index not in keep], context="FACES")
        bmesh.ops.delete(bm, geom=fine, context="FACES")
        mesh = bpy.data.meshes.new("FrontTeeth")
        part.to_mesh(mesh)
        part.free()
        for m in me.materials:
            mesh.materials.append(m)
        cage = new_object("FrontTeeth", mesh)
        subdivide(cage, 1)
        deps = bpy.context.evaluated_depsgraph_get()
        round_mesh = cage.evaluated_get(deps).to_mesh()
        bm.from_mesh(round_mesh)
        cage.evaluated_get(deps).to_mesh_clear()
        bpy.data.objects.remove(cage)
        bpy.data.meshes.remove(mesh)
    bm.to_mesh(me)
    bm.free()
    me.update()
    def what():
        """What each vertex of the mesh is, as it stands."""
        n = len(me.vertices)
        info = {"upper": np.zeros(n, dtype=bool), "tooth": np.full(n, -1, dtype=np.int64),
                "piece": np.full(n, -1, dtype=np.int64), "palate": np.zeros(n, dtype=bool)}
        for p in me.polygons:
            fs = ids[p.material_index]
            idx = list(p.vertices)
            info["upper"][idx] = fs in upper_sets
            if fs == FS_PALATE:
                info["palate"][idx] = True
            elif fs not in soft:
                info["tooth"][idx] = rank[fs]
                info["piece"][idx] = index_of[fs]
        info["is_tooth_face"] = [ids[p.material_index] not in soft for p in me.polygons]
        return info

    # The shade, while every tooth and the gums are whole: thinning keeps
    # the texture coordinates it finds.
    info = what()
    uv = paint(array(v.co for v in me.vertices), array(v.normal for v in me.vertices),
               [tuple(p.vertices) for p in me.polygons], info, ks)
    for extra in list(me.uv_layers):
        me.uv_layers.remove(extra)
    layer = me.uv_layers.new(name="UVMap")
    for loop in me.loops:
        layer.data[loop.index].uv = uv[loop.vertex_index]
    total = sum(len(p.vertices) - 2 for p in me.polygons)
    # Thinned: the rounded teeth (a subdivided cage has four times the
    # faces it needs), then the palate and the gums away from the front
    # teeth's crowns.
    now = [ids[p.material_index] for p in me.polygons]
    pts = [tuple(v.co) for v in me.vertices]
    crown_tree = BVHTree.FromPolygons(pts, [tuple(p.vertices) for p, fs in zip(me.polygons, now)
                                            if fs not in soft and rank[fs] < MOLAR])
    rounded, rounded_lower, cages, thin, firm = set(), set(), set(), set(), set()
    for p, fs in zip(me.polygons, now):
        if fs == FS_PALATE:
            thin.update(p.vertices)
        elif fs not in soft:
            if rank[fs] <= FINE and round_keeps:
                (rounded if fs in upper_sets else rounded_lower).update(p.vertices)
            else:
                cages.update(p.vertices)
    for p, fs in zip(me.polygons, now):
        if fs in (FS_UPPER_GUM, FS_LOWER_GUM):
            for v in p.vertices:
                if v not in thin and v not in firm:
                    if thin_keeps < THIN or crown_tree.find_nearest(Vector(pts[v]), KEEP * ks)[0] is None:
                        thin.add(v)
                    else:
                        firm.add(v)
    thin -= firm
    # (As groups: the first thinning renumbers the vertices.)
    todo = (("Rounded", rounded, round_keeps), ("RoundedLower", rounded_lower, lower_keeps),
            ("Cages", cages, cage_keeps), ("Thin", thin, thin_keeps))
    for name, members, _ in todo:
        jaw.vertex_groups.new(name=name).add(sorted(members), 1.0, "REPLACE")
    for name, _, keeps in todo:
        if keeps and keeps < 1.0:
            _thin(jaw, name, keeps)
    for name, _, _ in todo:
        jaw.vertex_groups.remove(jaw.vertex_groups[name])
    me = jaw.data
    info = what()
    info["triangles_from"] = total
    return jaw, info


def _thin(jaw, group, keeps):
    """`jaw` with the faces of the vertex group `group` collapsed to
    `keeps` of their number, the rest untouched, alike on both sides."""
    me = jaw.data
    index = jaw.vertex_groups[group].index
    inside = np.array([any(g.group == index and g.weight > 0.5 for g in v.groups) for v in me.vertices])
    among = sum(len(p.vertices) - 2 for p in me.polygons if all(inside[v] for v in p.vertices))
    total = sum(len(p.vertices) - 2 for p in me.polygons)
    if not among:
        return
    select_only([jaw])
    mod = jaw.modifiers.new("Decimate", "DECIMATE")
    mod.vertex_group = group
    mod.vertex_group_factor = 1000.0
    mod.use_symmetry = True
    mod.symmetry_axis = "X"
    mod.ratio = (total - among * (1.0 - keeps)) / total
    bpy.ops.object.modifier_apply(modifier="Decimate")


def _close_lower_gum(bm, fs_of, centres, ks):
    """The lower gum ended `KEEL` under the middle of the lower teeth and
    closed there, along a keel under the row. The bundle's lower gum is
    a wall 19 mm tall down the front of a whole lower jaw (a sculpting
    base), and inside the row it runs on as the floor of the mouth; the
    body's chin has no room for the wall (it came out through the skin
    under the lower lip), no mouth shows gum that far down, and the
    floor is the body's own lining. Closed, it is a solid the lips can
    be kept off: left open underneath, a point of the lining below its
    edge counts as behind its wall. The floor inside the row is cut
    away `FLOOR` from the keel; what is lower than the keel, and the
    open edges, are gathered onto it."""
    # The keel: under the middle of each lower tooth, from one end of the row to the other.
    row = sorted(centres.values(), key=lambda c: np.arctan2(c[0], 0.030 * ks - c[1]))
    line = np.array([row[0] + (row[0] - row[1]) * 1.5, *row, row[-1] + (row[-1] - row[-2]) * 1.5])
    line[:, 2] -= KEEL * ks
    middle = np.array([0.0, 0.045 * ks])

    def keel(q):
        """(how far `q` is from the keel seen from above, the keel's point there)."""
        best = None
        for a, b in zip(line[:-1], line[1:]):
            ab = (b - a)[:2]
            t = float(np.clip(((q[:2] - a[:2]) @ ab) / max(ab @ ab, 1e-12), 0.0, 1.0))
            at = a + (b - a) * t
            d = float(np.linalg.norm(at[:2] - q[:2]))
            if best is None or d < best[0]:
                best = (d, at)
        return best

    def node(q):
        """The keel's nearest stop to `q`: under a tooth's middle or half
        way between two. What is gathered onto the keel goes to these
        (and is joined there), so the two sides come out alike; joined
        by nearness alone, they differed by 2.4 mm."""
        stops = [a + (b - a) * t for a, b in zip(line[:-1], line[1:]) for t in (0.0, 0.5)] + [line[-1]]
        # (Chosen on one side and mirrored: a tie falls alike for both.)
        side = -1.0 if q[0] < -1e-7 else 1.0
        at = min(stops, key=lambda stop: float(np.linalg.norm(stop[:2] - np.array([abs(q[0]), q[1]]))))
        return np.array([side * at[0], at[1], at[2]])

    floor = []
    for f in bm.faces:
        if fs_of(f) == FS_LOWER_GUM:
            c = np.array(f.calc_center_median())
            d, at = keel(c)
            if d > FLOOR * ks and np.linalg.norm(c[:2] - middle) < np.linalg.norm(at[:2] - middle):
                floor.append(f)
    bmesh.ops.delete(bm, geom=floor, context="FACES")
    gum = {v for f in bm.faces if fs_of(f) == FS_LOWER_GUM for v in f.verts}
    back = max(v.co.y for v in gum) - 0.006 * ks
    sunk = set()
    for v in gum:
        q = np.array(v.co)
        _, at = keel(q)
        if q[2] < at[2] + 0.0015 * ks or (v.is_boundary and q[1] < back):
            v.co = Vector(node(q))
            sunk.add(v)
    bmesh.ops.remove_doubles(bm, verts=list(sunk), dist=1e-6)
    # (The faces that now close it are no longer flat: cut into triangles
    # here, on the diagonal that lies inside them, or the export's own
    # cut can turn one of a pair to face up into the gum.)
    closing = [f for f in bm.faces if fs_of(f) == FS_LOWER_GUM and any(v in sunk for v in f.verts)]
    bmesh.ops.triangulate(bm, faces=closing, quad_method="BEAUTY", ngon_method="BEAUTY")
    bmesh.ops.recalc_face_normals(bm, faces=[f for f in bm.faces if fs_of(f) == FS_LOWER_GUM])


def paint(points, normals, polys, info, ks):
    """Texture coordinates for the teeth and gums (jaw space, `ks` of the
    bundle's size): each vertex's place on its material's ramp and its
    shade (the module's notes)."""
    points, normals = np.asarray(points), np.asarray(normals)
    n = len(points)
    tooth, piece, upper = info["tooth"], info["piece"], info["upper"]
    is_tooth = tooth >= 0
    uv = np.zeros((n, 2))
    faces_of = {}
    for p, tooth_face in zip(polys, info["is_tooth_face"]):
        key = int(piece[p[0]]) if tooth_face else ("gum", bool(upper[p[0]]))
        faces_of.setdefault(key, []).append(p)
    pts = [tuple(p) for p in points]
    trees = {key: BVHTree.FromPolygons(pts, faces) for key, faces in faces_of.items()}

    def distance(tree, p, reach):
        hit = tree.find_nearest(Vector(p), reach)
        return reach if hit[0] is None else hit[3]

    # A tooth's neighbours along its row: the next rank either way on its
    # side, and for a central incisor the other one.
    mid = {key: np.mean([points[i] for f in faces for i in f], axis=0) for key, faces in faces_of.items()
           if not isinstance(key, tuple)}
    rank_of = {key: int(tooth[faces_of[key][0][0]]) for key in mid}
    up_of = {key: bool(upper[faces_of[key][0][0]]) for key in mid}
    beside = {}
    for a in mid:
        beside[a] = [b for b in mid if b != a and up_of[a] == up_of[b]
                     and ((abs(rank_of[a] - rank_of[b]) == 1 and mid[a][0] * mid[b][0] > 0)
                          or (rank_of[a] == 0 and rank_of[b] == 0))]
    centre = np.array([0.0, 0.030 * ks])
    out = points[:, :2] - centre
    out /= np.maximum(np.linalg.norm(out, axis=1, keepdims=True), 1e-9)
    facing = np.einsum("ij,ij->i", normals[:, :2], out)
    inward = 0.50 + 0.50 * step(-0.35, 0.25, facing)
    depth = depth_shade(points[:, 1], ks, upper)
    ramp, shade = np.zeros(n), np.zeros(n)
    for i in np.nonzero(is_tooth)[0]:
        key = int(piece[i])
        # (How far from the gum: what is left of a tooth is its crown.)
        h = distance(trees[("gum", bool(upper[i]))], points[i], 0.02 * ks) / ks
        near = min([distance(trees[b], points[i], 0.004 * ks) for b in beside[key]] or [0.004 * ks]) / ks
        ramp[i] = 0.5 * step(0.0002, 0.0032, h)
        shade[i] = ((0.45 + 0.55 * step(0.0002, 0.0020, h)) * (0.40 + 0.60 * step(0.0001, 0.0016, near))
                    * inward[i] * depth[i])
    # The thin edge of the front teeth (incisors and canines): cooler.
    for key in mid:
        if rank_of[key] > FINE:
            continue
        idx = np.unique([i for f in faces_of[key] for i in f])
        z = points[idx, 2]
        edge = z.min() if up_of[key] else z.max()
        ramp[idx] += 0.5 * (1.0 - step(0.0004 * ks, 0.0026 * ks, np.abs(z - edge))) * (ramp[idx] / 0.5)
    # The teeth's own colours begin at `IN_SHADE`; the lower back teeth
    # go over to the shade's colour, lit evenly.
    behind = np.zeros(n)
    for key in mid:
        if not up_of[key]:
            behind[np.unique([i for f in faces_of[key] for i in f])] = LOWER_IN_SHADE[min(rank_of[key], len(LOWER_IN_SHADE) - 1)]
    teeth = np.nonzero(is_tooth)[0]
    ramp[teeth] = (IN_SHADE + (1.0 - IN_SHADE) * ramp[teeth]) * (1.0 - behind[teeth])
    shade[teeth] += behind[teeth] * (SHADE_LIT * (0.7 + 0.3 * inward[teeth]) - shade[teeth])
    gums = np.nonzero(~is_tooth)[0]
    deep = np.interp(points[:, 1] / ks, [d[0] for d in DEPTH_GUMS], [d[1] for d in DEPTH_GUMS]) * np.where(upper, 1.0, 0.85)
    crowns = {up: BVHTree.FromPolygons(pts, [p for p, t in zip(polys, info["is_tooth_face"]) if t and bool(upper[p[0]]) == up])
              for up in (True, False)}
    for i in gums:
        d = distance(crowns[bool(upper[i])], points[i], 0.012 * ks) / ks
        ramp[i] = step(0.0, 0.0095, d)
        shade[i] = (0.55 + 0.45 * step(0.0, 0.0014, d)) * (0.7 + 0.3 * inward[i]) * deep[i]
    uv[:] = place(ramp, shade)
    return uv


def paint_tongue(along, around):
    """Texture coordinates for the tongue's points: `along` from the root
    0 to the tip 1; `around` is each point's place round its ring as
    (how far to the side 0 to 1, how far up -1 to 1). Darker toward the
    root and underneath, with a darker groove down the middle of its
    top."""
    along = np.asarray(along)
    side, up = np.asarray(around)[:, 0], np.asarray(around)[:, 1]
    groove = step(0.0, 0.45, side)
    ramp = np.where(up > 0.0, 0.25 + 0.75 * groove, 1.0)
    # (What shows of a tongue from the front of an open mouth is its back
    # third, as a ramp rising behind the lower lip: the shade falls
    # there, and is not near black at the root, or no tongue shows.)
    shade = (0.42 + 0.53 * step(0.0, 0.35, along)) * (0.55 + 0.45 * step(-0.4, 0.3, up))
    return place(ramp, shade)
