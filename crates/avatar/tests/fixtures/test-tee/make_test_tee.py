"""A test garment for the import flow (CO9): a short-sleeved T-shirt made
from simple solids, NOT from the body's surface.

The fitting tests import it as a third-party garment would be imported:
an OBJ in the realistic body's rest pose, roughly aligned. Only a
tailor's measurements are taken from the body (a girth every 2 cm, the
upper arm's girth along it), as a person modelling a shirt would take
them; the shirt itself is a loft of those girths with the ease (the
torso) and two tubes (the sleeves), joined by a voxel remesh, smoothed,
with the hem and sleeve ends cut by planes and a crew neck cut by a
cylinder round the neck. It is a **draft** by
`docs/development/character-studio/asset-quality.md` (one layer, no
construction, seams or folds): a fixture, not a wardrobe item.

Made by the Axcenity Engine contributors and dedicated to the public
domain (CC0 1.0).

Run with Blender's Python (bpy 5.x):

    python -I crates/avatar/tests/fixtures/test-tee/make_test_tee.py \
        --body assets/characters/realistic/RealisticParts.glb \
        --out crates/avatar/tests/fixtures/test-tee

Blender axes: Z up, the character faces -Y. The OBJ is written Y up, the
character facing +Z (glTF's axes).
"""

import argparse
import math
import os

import bpy
import numpy as np
from mathutils import Vector

# How far the shirt stands off the body (a loose fit), and at the neck
# (a crew neck's rib sits round the neck's base).
EASE = 0.022
NECK_EASE = 0.006
SLEEVE_EASE = 0.014
# How much wider than the neck the crew neck's opening is at the sides,
# where it sits on the base of the neck: an ordinary crew neck, which the
# neck's base doesn't rise into when the arms are raised. (Until wave 4
# it was 2 mm, close round the neck, to cover the sports bra's straps,
# which came up the side of the neck; they now sit on the shoulders.)
NECK_SIDE = 0.014
# How far down the neck (from its joint) the crew neck's cylinder starts.
NECK_DROP = 0.08
# The crew neck's front: a scoop this deep below the neck's sides, and
# this much narrower than the opening at the sides: a round U down to the
# hollow at the neck's base.
NECK_SCOOP = 0.07
NECK_SCOOP_IN = 0.004
# How far over the skin the shirt lies on the top of the shoulders, where
# it hangs (`drape_shoulders`): over the sports bra's straps (3 mm off the
# skin, more where a shape pushes them out), with a little room.
SHOULDER_EASE = 0.014
# The crew neck's edge lies on the skin (a collar's rib touches the neck's
# base all round): within `COLLAR_REACH` of the opening the shell comes in
# to `COLLAR_EASE` over the skin, fading out with the distance.
COLLAR_EASE = 0.002
COLLAR_REACH = 0.012
# Rounds of smoothing the settled edge along its line.
COLLAR_SMOOTHING = 12
# The voxel remesh's size: about 6k triangles for the shirt.
VOXEL = 0.019
ARM_PARTS = ("UpperArm", "LowerArm", "Hand", "Thumb", "Index", "Middle", "Ring", "Little")
# Below the armpit a slice of the body is the trunk in the middle and an
# arm either side, apart from it: a girth is the trunk's, out to the first
# gap this wide. (Skin of the arm weighted to the shoulder's helper bones,
# not the arm's, would otherwise widen a girth into a flange.)
GAP = 0.03
# How far past the trunk's side at the armpit a girth above it reaches
# (the arm's root, under the front and back of the shoulder).
ARM_ROOT = 0.05
# The sleeve's length, as a share of the upper arm: past the armpit by a
# hand's width underneath, as a T-shirt's underarm seam is, so a raised
# arm doesn't open the side.
SLEEVE = 0.9
# The sleeve's hem leans: longer underneath than on top by this share of
# the sleeve's width either side of its middle (a T-shirt's underarm seam
# is longer than the arm's top is from the shoulder's line to the hem).
HEM_TILT = 0.4
# How far below the armpit, at least, the sleeve's hem is underneath.
UNDERARM = 0.025
COLOUR = (0.30, 0.38, 0.52, 1.0)


def body_data():
    """The body's rest positions (world), its arm weight per vertex, its
    left upper arm's weight (with the deltoid's helper), and the
    armature."""
    body = bpy.data.objects["Body"]
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    me = body.data
    n = len(me.vertices)
    co = np.zeros(n * 3)
    me.vertices.foreach_get("co", co)
    co = co.reshape(n, 3)
    world = np.array(body.matrix_world)
    P = co @ world[:3, :3].T + world[:3, 3]
    groups = {g.index: g.name for g in body.vertex_groups}
    arm_weight = np.zeros(n)
    upper_arm = np.zeros(n)
    for v in me.vertices:
        for g in v.groups:
            name = groups[g.group]
            if any(part in name for part in ARM_PARTS):
                arm_weight[v.index] += g.weight
            if "LeftUpperArm" in name or name == "LeftShoulderHelper":
                upper_arm[v.index] += g.weight
    return P, arm_weight, upper_arm, arm


def body_edges():
    me = bpy.data.objects["Body"].data
    edges = np.zeros(len(me.edges) * 2, dtype=np.int64)
    me.edges.foreach_get("vertices", edges)
    return edges.reshape(-1, 2)


def joint(arm, name):
    return np.array(arm.matrix_world @ arm.data.bones[name].head_local)


def section(P, edges, z, keep):
    """Where the body's edges cross the height `z` (among the edges whose
    both corners `keep` holds): its outline there."""
    a, b = P[edges[:, 0]], P[edges[:, 1]]
    crossing = ((a[:, 2] - z) * (b[:, 2] - z) <= 0) & (a[:, 2] != b[:, 2])
    crossing &= keep[edges[:, 0]] & keep[edges[:, 1]]
    a, b = a[crossing], b[crossing]
    t = (z - a[:, 2]) / (b[:, 2] - a[:, 2])
    return a + (b - a) * t[:, None]


def trunk(points):
    """The trunk's part of an outline: out from the middle to the first gap
    of `GAP` across (an arm, apart from it, is beyond)."""
    xs = np.sort(np.abs(points[:, 0]))
    reach = xs[0]
    for x in xs[1:]:
        if x - reach > GAP:
            break
        reach = x
    return points[np.abs(points[:, 0]) <= reach + 1e-9]


def hull(points):
    """The convex hull of 2D `points`, counter-clockwise (Andrew's monotone
    chain)."""
    pts = sorted(set(map(tuple, np.round(points, 6))))

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return np.array(lower[:-1] + upper[:-1])


def girth_ring(outline, z, ease, count=48):
    """A girth as a tape takes it: round the outline's convex hull (made
    the same on both sides), `ease` out from it, as `count` points round
    its middle."""
    xy = outline[:, :2]
    xy = np.vstack([xy, xy * np.array([-1.0, 1.0])])
    poly = hull(xy)
    centre = np.array([0.0, poly[:, 1].mean()])
    edges = [(poly[k], poly[(k + 1) % len(poly)]) for k in range(len(poly))]
    ring = []
    for t in (2 * math.pi * k / count for k in range(count)):
        d = np.array([math.cos(t), math.sin(t)])
        best = None
        for a, b in edges:
            e = b - a
            # centre + d * r = a + e * u
            m = np.array([[d[0], -e[0]], [d[1], -e[1]]])
            if abs(np.linalg.det(m)) < 1e-12:
                continue
            r, u = np.linalg.solve(m, a - centre)
            if r > 0 and -1e-9 <= u <= 1 + 1e-9 and (best is None or r < best[0]):
                normal = np.array([e[1], -e[0]]) / np.linalg.norm(e)
                best = (r, normal)
        r, normal = best
        # Out along `d` far enough to be `ease` from the hull's side.
        r += ease / max(abs(d @ normal), 0.3)
        ring.append((centre[0] + d[0] * r, centre[1] + d[1] * r, z))
    return ring


def solid(rings):
    """A closed mesh: the rings joined side by side, capped at both ends."""
    count = len(rings[0])
    verts = [p for ring in rings for p in ring]
    faces = []
    for r in range(len(rings) - 1):
        for k in range(count):
            a, b = r * count + k, r * count + (k + 1) % count
            faces.append((a, b, b + count, a + count))
    faces.append(tuple(reversed(range(count))))
    last = (len(rings) - 1) * count
    faces.append(tuple(range(last, last + count)))
    me = bpy.data.meshes.new("part")
    me.from_pydata(verts, [], faces)
    me.update()
    me.validate()
    obj = bpy.data.objects.new("part", me)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def sleeve(centres, axis, radii, count=32):
    """A tube along `axis`: a ring of radius `radii[k]` round `centres[k]`."""
    side = np.cross(axis, [0.0, 0.0, 1.0])
    side /= np.linalg.norm(side)
    up = np.cross(side, axis)
    rings = []
    for centre, r in zip(centres, radii):
        rings.append([tuple(centre + r * (math.cos(a) * side + math.sin(a) * up))
                      for a in (2 * math.pi * k / count for k in range(count))])
    return solid(rings)


def mesh_points(obj, key=None):
    """`obj`'s vertices in the world, or its shape `key`'s."""
    me = obj.data
    co = np.zeros(len(me.vertices) * 3)
    (me.shape_keys.key_blocks[key].data if key else me.vertices).foreach_get("co", co)
    world = np.array(obj.matrix_world)
    return co.reshape(-1, 3) @ world[:3, :3].T + world[:3, 3]


def drape_shoulders(tee, P, shoulder):
    """Lets the shell's top down onto the shoulders, as a shirt hangs on
    them: the girths' ease is across, so over the top of the shoulder the
    shell stood up to 2 cm off the skin, and a crew neck's edge stood off
    the trapezius with a gap under it (the sports bra's straps showed
    through it from a three-quarter view). Each corner that faces up,
    between the neck and the shoulder joint, comes down to
    `SHOULDER_EASE` over the skin straight under it; the share fades out
    toward the side of the shoulder and where the shell faces sideways,
    and the moves are smoothed over the shell so it stays one surface.
    Returns the largest move (m)."""
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    body = bpy.data.objects["Body"]
    skin = BVHTree.FromPolygons(P.tolist(), [list(f.vertices) for f in body.data.polygons])
    me = tee.data
    co = mesh_points(tee)
    normals = np.array([tuple(v.normal) for v in me.vertices])
    reach_x = abs(shoulder[0])
    move = np.zeros(len(co))
    # How far each corner can come down before it is within its ease of
    # the skin straight under it.
    room = np.zeros(len(co))
    down = Vector((0.0, 0.0, -1.0))
    for k, (p, n) in enumerate(zip(co, normals)):
        if p[2] < shoulder[2] - 0.08:
            continue
        hit = skin.ray_cast(Vector(p), down, 0.08)[0]
        if hit is None:
            continue
        room[k] = max(p[2] - (hit.z + SHOULDER_EASE), 0.0)
        upward = np.clip((n[2] - 0.3) / 0.4, 0.0, 1.0)
        inward = np.clip((reach_x + 0.04 - abs(p[0])) / 0.04, 0.0, 1.0)
        move[k] = room[k] * upward * inward
    # Smoothed over the shell's edges, and never more than a corner has
    # room for.
    neighbours = [[] for _ in range(len(co))]
    for e in me.edges:
        a, b = e.vertices
        neighbours[a].append(b)
        neighbours[b].append(a)
    for _ in range(6):
        move = np.array([0.5 * move[k] + 0.5 * np.mean(move[nb]) if nb else move[k]
                         for k, nb in enumerate(neighbours)])
        move = np.minimum(move, room)
    for k, v in enumerate(me.vertices):
        v.co.z -= move[k]
    me.update()
    return float(move.max())


def settle_collar(tee, P, neck, neck_a):
    """Brings the crew neck's edge onto the skin (`COLLAR_EASE`), fading out
    over `COLLAR_REACH` from the opening: a shell standing off the skin at
    its neckline leaves a slit between the collar and the neck's base,
    through which the inside of the shirt (and the sports bra's straps on
    the far shoulder) shows from a three-quarter view. Returns how many
    corners moved."""
    from collections import Counter
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    body = bpy.data.objects["Body"]
    skin = BVHTree.FromPolygons(P.tolist(), [list(f.vertices) for f in body.data.polygons])
    me = tee.data
    co = mesh_points(tee)
    uses = Counter()
    for poly in me.polygons:
        vs = list(poly.vertices)
        for a, b in zip(vs, vs[1:] + vs[:1]):
            uses[(min(a, b), max(a, b))] += 1
    edge = sorted({v for e, n in uses.items() if n == 1 for v in e})
    # The neck's opening: open edges near the neck (not the hem, nor the
    # sleeves' ends).
    on_opening = {v for v in edge if co[v][2] > neck[2] - 0.12 and abs(co[v][0]) < neck_a + 0.03}
    opening = np.array([co[v] for v in sorted(on_opening)])
    moved = 0
    for k, v in enumerate(me.vertices):
        p = co[k]
        far = np.linalg.norm(opening - p, axis=1).min()
        if far >= COLLAR_REACH:
            continue
        loc, normal, _, _ = skin.find_nearest(Vector(p), 0.05)
        if loc is None:
            continue
        out = (Vector(p) - loc).dot(normal)
        if out <= COLLAR_EASE:
            continue
        t = 1.0 - far / COLLAR_REACH
        share = t * t * (3.0 - 2.0 * t)
        v.co -= Vector(normal) * (out - COLLAR_EASE) * share
        moved += 1
    # The edge itself smoothed along its own line (the skin under it is
    # uneven, and a settled edge took that up as a zigzag), kept its ease
    # over the skin.
    ring = {}
    for (a, b), n in uses.items():
        if n == 1 and a in on_opening and b in on_opening:
            ring.setdefault(a, []).append(b)
            ring.setdefault(b, []).append(a)
    for _ in range(COLLAR_SMOOTHING):
        now = {k: me.vertices[k].co.copy() for k in ring}
        for k, nb in ring.items():
            mean = sum((now[j] for j in nb), Vector()) / len(nb)
            co_k = now[k].lerp(mean, 0.5)
            loc, normal, _, _ = skin.find_nearest(co_k, 0.05)
            if loc is not None and (co_k - loc).dot(normal) < COLLAR_EASE:
                co_k = co_k + normal * (COLLAR_EASE - (co_k - loc).dot(normal))
            me.vertices[k].co = co_k
    me.update()
    return moved


def check_underarm(tee, armpit, side):
    """The sleeves' hems are below the armpit underneath (a short sleeve
    whose hem is above it leaves the side open when the arm is raised)."""
    from collections import Counter
    me = tee.data
    uses = Counter()
    for poly in me.polygons:
        vs = list(poly.vertices)
        for a, b in zip(vs, vs[1:] + vs[:1]):
            uses[(min(a, b), max(a, b))] += 1
    hem = {v for edge, n in uses.items() if n == 1 for v in edge}
    P = mesh_points(tee)
    sleeve_hem = [P[v] for v in hem if abs(P[v][0]) > side + EASE]
    lowest = min(p[2] for p in sleeve_hem)
    print(f"the sleeves' hems are {armpit - lowest:.3f} m below the armpit underneath")
    return [] if lowest <= armpit - UNDERARM else [f"the sleeves' hems are {armpit - lowest:.3f} m below the armpit"]


def check_covers_the_bra(tee, armpit, side, neck_a):
    """The shirt hides the sports bra on both frames, from every way a
    capture looks at it: a ray from each of the bra's corners off the skin
    toward the camera meets the shirt or the body. On the feminine frame
    the shirt is moved as the skin under it moves. The straps go over the
    shoulders and the side band up under the arm: a neckline or an
    armhole that shows them fails here.

    One place is let off, by a few corners, and counted: the side band
    seen from straight beside the body up the sleeve, past the arm (as up
    any short sleeve). The straps' tops by the collar are counted apart and
    let off none (until wave 4 the straps came up the side of the neck,
    and their top 2 mm showed at a crew neck)."""
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    from mathutils.kdtree import KDTree
    body = bpy.data.objects["Body"]
    bra = bpy.data.objects["Underwear_Bra"]
    skin = {key: mesh_points(body, key) for key in (None, "Frame_Feminine")}
    tree = KDTree(len(skin[None]))
    for k, p in enumerate(skin[None]):
        tree.insert(Vector(p), k)
    tree.balance()
    tee_m = mesh_points(tee)
    nearest = np.array([tree.find(Vector(p))[1] for p in tee_m])
    polys = [list(p.vertices) for p in tee.data.polygons]
    body_polys = [list(p.vertices) for p in body.data.polygons]
    # The capture matrix's views (front, three-quarter, side, back; both
    # sides), level and from as far above as its three-quarter view is
    # (2 degrees), and a little more. Blender's -Y is the front.
    looks = [Vector((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))
             for a in np.radians([0.0, 90.0, 180.0, 225.0, 270.0, 315.0]) for e in np.radians([0.0, 5.0])]
    from collections import Counter
    uses = Counter()
    for poly in polys:
        for a, b in zip(poly, poly[1:] + poly[:1]):
            uses[(min(a, b), max(a, b))] += 1
    rim = sorted({v for e, n in uses.items() if n == 1 for v in e})
    problems = []
    for key in (None, "Frame_Feminine"):
        moved = tee_m + skin[key][nearest] - skin[None][nearest]
        # The shirt's open edges (hem, neck, sleeves), where the bra can
        # show past it.
        edge = KDTree(len(rim))
        for k, v in enumerate(rim):
            edge.insert(Vector(moved[v]), k)
        edge.balance()
        shirt = BVHTree.FromPolygons(moved.tolist(), polys)
        skin_tree = BVHTree.FromPolygons(skin[key].tolist(), body_polys)
        seen, collar, sleeve_ = 0, 0, 0
        under = mesh_points(bra, key)
        top = under[:, 2].max()
        for p in under:
            p = Vector(p)
            loc, normal, _, _ = skin_tree.find_nearest(p, 0.05)
            # The rim folded under the skin is not seen.
            if loc is None or (p - loc).dot(normal) < 0.001:
                continue
            # Where the bra stands out through the shirt (straight out from
            # the shirt's surface, not past an edge of it), the import's
            # layer push-out lifts the shirt over it: not this check's.
            loc, normal, _, _ = shirt.find_nearest(p, 0.05)
            if loc is not None and (p - loc).dot(normal) > 0 and edge.find(loc)[2] > 0.015:
                continue
            for d in looks:
                start = p + d * 0.001
                if shirt.ray_cast(start, d, 3.0)[0] is None and skin_tree.ray_cast(start, d, 3.0)[0] is None:
                    where = f"x {p.x:+.3f} y {p.y:+.3f} z {p.z:.3f} from {tuple(round(c, 2) for c in d)}"
                    if p.z > top - 0.01 and abs(p.x) < neck_a + 0.01:
                        collar += 1
                    elif abs(d.x) > 0.9 and abs(p.z - armpit) < 0.06 and abs(p.x) > side - 0.04:
                        sleeve_ += 1
                    else:
                        seen += 1
                        print(f"  seen at {where}")
                    break
        print(f"the sports bra on the {key or 'masculine'} frame: {seen} corners seen; "
              f"{collar} at the straps' tops by the collar, {sleeve_} up a sleeve from the side")
        if seen or collar or sleeve_ > 4:
            problems.append(f"{seen} of the sports bra's corners show on the {key or 'masculine'} frame")
    return problems


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--body", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--keep-going", action="store_true",
                        help="write the shirt even when a check fails, to look at it")
    args = parser.parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=os.path.abspath(args.body))
    P, arm_weight, upper_arm, arm = body_data()

    hips = joint(arm, "Hips")
    shoulder_l = joint(arm, "LeftUpperArm")
    elbow_l = joint(arm, "LeftLowerArm")
    neck = joint(arm, "Neck")
    hem = hips[2] - 0.10

    # The torso: a girth every 2 cm from the hem to the neck. Below the
    # armpit a girth is the trunk's (the arms apart from it left out);
    # above it, the body's outline no wider than the trunk at the armpit
    # and the arm's root (the shoulder past that is the sleeve's).
    edges = body_edges()
    not_arm = arm_weight < 0.5
    # The armpit: the highest the arm is still apart from the trunk
    # (from the elbow up, where the upper arm is beside it).
    armpit = elbow_l[2]
    while True:
        z = armpit + 0.005
        outline = section(P, edges, z, not_arm)
        if len(trunk(outline)) == len(outline):
            break
        armpit = z
    side = np.abs(trunk(section(P, edges, armpit, not_arm))[:, 0]).max()
    print(f"armpit at {armpit:.3f} m (the shoulder joint at {shoulder_l[2]:.3f} m), "
          f"the trunk {side:.3f} m wide there")
    # No girth is wider than this: one past it has caught an arm (and the
    # trunk at the armpit is no wider than the shoulder joint and a hand).
    widest = side + ARM_ROOT
    assert side < abs(shoulder_l[0]) + 0.05, f"the trunk at the armpit is {side:.3f} m wide"
    # Up the neck (the neck's cylinder cuts the opening).
    top = neck[2] + 0.04
    rings = []
    for z in np.arange(hem, top + 1e-6, 0.02):
        if z <= armpit:
            s = trunk(section(P, edges, z, not_arm))
        else:
            s = section(P, edges, z, not_arm)
            s = s[np.abs(s[:, 0]) <= side + ARM_ROOT]
        ease = NECK_EASE if z > top - 0.03 else EASE
        ring = girth_ring(s, z, ease)
        a = max(abs(p[0]) for p in ring)
        # A flange would be wider.
        assert a <= widest + EASE + 1e-6, f"the girth at {z:.3f} m is {a:.3f} m wide"
        rings.append(ring)
    parts = [solid(rings)]

    # The sleeves: round the upper arm from the shoulder joint to a little
    # past half way to the elbow, measured on the left arm (mirrored): at
    # each step the arm's girth there (the deltoid's near the shoulder)
    # and the ease.
    length = np.linalg.norm(elbow_l - shoulder_l)
    axis_l = (elbow_l - shoulder_l) / length
    rel = P[upper_arm > 0.5] - shoulder_l
    along = rel @ axis_l
    # The tube runs on past the hem (which leans further along
    # underneath), so the cut is through it, not through its rounded end.
    steps = np.linspace(0.0, SLEEVE * length + 0.05, 9)
    # Each ring round the arm's slice there (its middle, which is off the
    # bone), out to all but its farthest few corners and the ease.
    centres, radii = [], []
    for t in steps:
        # Past the upper arm's skin (by the elbow) as it is just before.
        m = min(t, 0.9 * length)
        part = rel[np.abs(along - m) < 0.015]
        middle = part.mean(axis=0)
        middle -= (middle @ axis_l - t) * axis_l
        # At the shoulder the slice takes in the deltoid's top: the ring
        # keeps to the bone there, and goes over to the slice's middle by
        # half way along.
        share = min(t / (0.5 * steps[-1]), 1.0)
        middle = axis_l * t + share * (middle - axis_l * t)
        centres.append(middle)
        off = part - middle
        off -= np.outer(off @ axis_l, axis_l)
        radii.append(np.percentile(np.linalg.norm(off, axis=1), 90) + SLEEVE_EASE)
    # No wider at the shoulder than a little over the sleeve's middle: a
    # wider start stands up off the shoulder's line.
    radii = [min(r, 1.05 * radii[len(radii) // 2]) for r in radii]
    radius = max(radii)
    print("sleeve radii " + " ".join(f"{r:.3f}" for r in radii))
    for sign in (1.0, -1.0):
        mirror = np.array([sign, 1.0, 1.0])
        axis = axis_l * mirror
        parts.append(sleeve([shoulder_l * mirror + c * mirror for c in centres], axis, radii))

    # One shell: the solids' union, smoothed.
    for obj in bpy.data.objects:
        obj.select_set(obj in parts)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    tee = parts[0]
    tee.name = "TestTee"
    tee.data.name = "TestTee"
    remesh = tee.modifiers.new("remesh", "REMESH")
    remesh.mode = "VOXEL"
    remesh.voxel_size = VOXEL
    smooth = tee.modifiers.new("smooth", "SMOOTH")
    smooth.factor = 0.5
    smooth.iterations = 4
    bpy.ops.object.modifier_apply(modifier="remesh")
    bpy.ops.object.modifier_apply(modifier="smooth")
    print(f"the shoulders draped: the shell came down {drape_shoulders(tee, P, shoulder_l):.3f} m at most")

    # The neck: a crew neck. An elliptic cylinder along the neck, close
    # round it (its column a little above the joint, the chin left out),
    # opens the shell's top: at the sides the opening sits on the neck's
    # base, above the sports bra's straps where they come up beside the
    # neck. In front a leaning plane (below) brings it down to the hollow
    # at the neck's base. The cylinder's faces (a pit into the closed
    # shell) are deleted after, which leaves the opening.
    column = section(P, edges, neck[2] + 0.03, arm_weight < 0.1)
    column = column[column[:, 1] > neck[1] - 0.09]
    neck_a = np.abs(column[:, 0]).max() + NECK_SIDE
    neck_cy = (column[:, 1].max() + column[:, 1].min()) / 2
    neck_b = (column[:, 1].max() - column[:, 1].min()) / 2 + NECK_SIDE
    print(f"the neck's opening: {2 * neck_a:.3f} m across, {2 * neck_b:.3f} m front to back")
    up = Vector(joint(arm, "Head") - neck).normalized()
    base = Vector((0.0, neck_cy, neck[2] + 0.03)) - up * (NECK_DROP + 0.03)
    bpy.ops.mesh.primitive_cylinder_add(vertices=96, radius=1.0, depth=1.0)
    cutter = bpy.context.active_object
    cutter.scale = (neck_a, neck_b, 0.5)
    cutter.rotation_mode = "QUATERNION"
    cutter.rotation_quaternion = up.to_track_quat("Z", "Y")
    cutter.location = base + up * 0.25
    # The front's scoop: an elliptic cylinder from the neck's middle
    # forward, as wide as the neck less `NECK_SCOOP_IN` either side and
    # reaching `NECK_SCOOP` below the sides of the opening: a U, high at
    # the sides, down to the hollow at the neck's base in front.
    bpy.ops.mesh.primitive_cylinder_add(vertices=96, radius=1.0, depth=1.0)
    scoop = bpy.context.active_object
    scoop.scale = (neck_a - NECK_SCOOP_IN, NECK_SCOOP, 0.15)
    scoop.rotation_mode = "QUATERNION"
    scoop.rotation_quaternion = Vector((0.0, -1.0, 0.0)).to_track_quat("Z", "Y")
    scoop.location = (0.0, neck_cy - 0.15, neck[2] + 0.03)
    shell = bpy.data.materials.new("Shell")
    hole = bpy.data.materials.new("Hole")
    tee.data.materials.append(shell)
    for tool in (cutter, scoop):
        tool.data.materials.append(hole)
        boolean = tee.modifiers.new("neck", "BOOLEAN")
        boolean.operation = "DIFFERENCE"
        boolean.solver = "EXACT"
        boolean.material_mode = "TRANSFER"
        boolean.object = tool
        bpy.context.view_layer.objects.active = tee
        bpy.ops.object.modifier_apply(modifier="neck")
        bpy.data.objects.remove(tool)
    hole_slot = [m.name for m in tee.data.materials].index(hole.name)

    # Open the hem and the sleeve ends with straight cuts: a
    # plane through the triangles, so each edge is a clean line rather
    # than the remesh's stair-steps.
    import bmesh
    me = tee.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.faces.ensure_lookup_table()
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if f.material_index == hole_slot], context="FACES")

    def cut(co, no, chosen=lambda c: True):
        """Cuts the faces `chosen` picks (by their centres) along the plane
        at `co` with normal `no`, and deletes what is on the normal's side."""
        faces = [f for f in bm.faces if chosen(np.array(f.calc_center_median()))]
        edges = list({e for f in faces for e in f.edges})
        verts = list({v for f in faces for v in f.verts})
        bmesh.ops.bisect_plane(bm, geom=verts + edges + faces, dist=1e-6,
                               plane_co=tuple(co), plane_no=tuple(no), clear_outer=True)

    # Each a little inside the solid's end, past where the remesh rounded
    # its cap.
    cut((0.0, 0.0, hem + 0.02), (0.0, 0.0, -1.0))
    for sign in (1.0, -1.0):
        s = shoulder_l * np.array([sign, 1.0, 1.0])
        axis = axis_l * np.array([sign, 1.0, 1.0])

        def on_sleeve(c, s=s, axis=axis):
            # The sleeve, past the torso's side (where they meet under the
            # arm the torso is left whole).
            along = (c - s) @ axis
            off_axis = np.linalg.norm((c - s) - along * axis)
            return along > 0.3 * length and off_axis < 1.6 * radius and abs(c[0]) > side + EASE

        # Leaning: the plane's normal turned from the arm's axis toward
        # its underside (toward the trunk and down), so the hem is
        # `HEM_TILT` of the sleeve's radius further along underneath.
        down = np.array([0.0, 0.0, -1.0])
        under = down - (down @ axis) * axis
        under /= np.linalg.norm(under)
        lean = axis - HEM_TILT * under
        cut(s + axis * (SLEEVE * length - 0.02), lean / np.linalg.norm(lean), on_sleeve)
    # Keep the largest piece (a cut can leave a stray ring).
    bm.faces.ensure_lookup_table()
    bm.faces.index_update()
    seen = set()
    pieces = []
    for f in bm.faces:
        if f.index in seen:
            continue
        stack, piece = [f], []
        seen.add(f.index)
        while stack:
            g = stack.pop()
            piece.append(g)
            for e in g.edges:
                for h in e.link_faces:
                    if h.index not in seen:
                        seen.add(h.index)
                        stack.append(h)
        pieces.append(piece)
    keep = max(pieces, key=len)
    bmesh.ops.delete(bm, geom=[f for piece in pieces if piece is not keep for f in piece], context="FACES")
    bm.to_mesh(me)
    bm.free()
    me.update()
    for poly in me.polygons:
        poly.use_smooth = True
    print(f"the collar's edge settled onto the skin: {settle_collar(tee, P, neck, neck_a)} corners")
    problems = check_underarm(tee, armpit, side) + check_covers_the_bra(tee, armpit, side, neck_a)
    assert args.keep_going or not problems, problems

    mat = bpy.data.materials.new("TeeCotton")
    mat.diffuse_color = COLOUR
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = COLOUR
    bsdf.inputs["Roughness"].default_value = 0.85
    me.materials.clear()
    me.materials.append(mat)

    # Texture coordinates, for completeness.
    for obj in bpy.data.objects:
        obj.select_set(obj == tee)
    bpy.context.view_layer.objects.active = tee
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(island_margin=0.01)
    bpy.ops.object.mode_set(mode="OBJECT")

    triangles = sum(len(p.vertices) - 2 for p in me.polygons)
    print(f"TestTee: {len(me.vertices)} vertices, {triangles} triangles")
    os.makedirs(args.out, exist_ok=True)
    bpy.ops.wm.obj_export(filepath=os.path.join(os.path.abspath(args.out), "TestTee.obj"),
                          export_selected_objects=True, forward_axis="NEGATIVE_Z", up_axis="Y",
                          export_materials=True, export_normals=True, export_uv=True,
                          apply_modifiers=True, path_mode="STRIP")
    # A shirt that fails a check is written only to be looked at
    # (`--keep-going`); the script still fails.
    assert not problems, problems


if __name__ == "__main__":
    main()
