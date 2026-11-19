"""The intake's pose correctives (wave 5, stage 3): a fitted garment's
shapes for the shoulder, arm swing, elbow, hip and knee poses, baked from
Blender's cloth simulation and driven at run time by the joints, as the
body's own swing correctives are (`axcenity_avatar::fit::cloth`,
`docs/development/character-studio/refinement/wave5/pose-correctives.md`).

Four steps, each also runnable alone with `--only`:

| Step | What | Writes in `--work` |
|---|---|---|
| dump | The engine's poses (`cargo run -p axcenity-avatar --release --example pose_correctives -- dump`): the body and the garment, and per run the skin posed in each frame and the garment as linear skinning poses it today | `bake.json`, `*.f32`, `*.u32` |
| simulate | Blender's cloth simulation per run (bpy): the body a collider moving through the run's frames, the garment's cloth starting at rest and held towards its skinned place by a pin goal that lets go where skinning stretches or squeezes it (the bridge between the arm and the side); the cloth read at each of the run's reads | `<run>.sim.f32`, `simulate.json` |
| solve | The shapes (`... -- solve`): at each key pose the simulated cloth less the skinned one, taken back to rest, less what the simulation moved at rest; half way to each key pose the blend is checked against the simulation | `--out` (`<Object>.correctives.json`), `solve.json` |
| add | The shapes into the item's model and manifest (`... -- add`) | the item in `--library` |

    python -I correctives.py --library <library> --item <item ID> --work <folder> \\
        [--out <file>] [--only dump,simulate,solve,add] [--runs hang,swing-left]

Blender's Python (bpy 5.x with NumPy) for `simulate`; the other steps need
only NumPy and Cargo. A garment whose model already has cloth correctives
is refused by `add`: run it on the model as exported.
"""

import argparse
import json
import os
import subprocess
import sys
import time

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(HERE))))

STEPS = ["dump", "simulate", "solve", "add"]

# The cloth: cotton jersey (Blender's "Cotton" preset's tension, shear and
# air; a T-shirt's mass; bending raised from 0.5 to 2, so a coarse garment
# mesh folds in folds wider than a triangle), simulated at 24 frames a
# second.
CLOTH = {
    "mass": 0.15,
    "tension_stiffness": 15.0,
    "compression_stiffness": 15.0,
    "shear_stiffness": 5.0,
    "bending_stiffness": 2.0,
    "air_damping": 1.0,
    "quality": 10,
}
# How far the cloth is kept from the skin and from itself (metres): the
# fit's margin.
SKIN_DISTANCE = 0.002
SELF_DISTANCE = 0.0015
# No gravity: the bake corrects what a pose does to the cloth, not how the
# garment hangs at rest (the fit's: with gravity the cloth sagged up to
# 34 mm at rest and the freed bridge under the arm dropped away from the
# skin it covered, opening the armpit in the fit gate's T-pose).
GRAVITY = 0.0

# The pin goal: a corner whose edges skinning stretches or squeezes by
# `LET_GO` or more is held at `FREE`; one skinning keeps true at `HOLD`.
# The weights are spread over `SPREAD` rings, so the cloth lets go
# smoothly. Blender raises a weight to the fourth power: 0.95 holds at
# 0.81 of `PIN_STIFFNESS`, 0.7 at 0.24. (Measured on the test tee,
# pose-correctives.md: free corners at 0.3 and 0.5 let the bridge move 5
# to 10 cm and did worse in the fit gate.)
LET_GO = 0.12
HOLD = 0.95
FREE = 0.7
SPREAD = 4
PIN_STIFFNESS = 10.0


def cargo(*args):
    cmd = ["cargo", "run", "-q", "-p", "axcenity-avatar", "--release", "--example",
           "pose_correctives", "--", *args]
    out = subprocess.run(cmd, cwd=REPOSITORY, capture_output=True, text=True)
    if out.returncode:
        raise SystemExit(out.stdout + out.stderr)
    return out.stdout


def floats(path, width=3):
    return np.fromfile(path, dtype="<f4").reshape(-1, width).astype(np.float64)


def triangles(path):
    return np.fromfile(path, dtype="<u4").reshape(-1, 3).astype(np.int64)


def edges_of(tris):
    e = np.vstack([tris[:, [0, 1]], tris[:, [1, 2]], tris[:, [2, 0]]])
    e.sort(axis=1)
    return np.unique(e, axis=0)


def strain(rest, posed, edges):
    """Each corner's largest stretch or squeeze of an edge (0 is none)."""
    r = np.linalg.norm(rest[edges[:, 0]] - rest[edges[:, 1]], axis=1)
    p = np.linalg.norm(posed[edges[:, 0]] - posed[edges[:, 1]], axis=1)
    s = np.abs(p / np.maximum(r, 1e-9) - 1.0)
    out = np.zeros(len(rest))
    np.maximum.at(out, edges[:, 0], s)
    np.maximum.at(out, edges[:, 1], s)
    return out


def spread(values, edges, rounds):
    """Each round, a corner takes the least of itself and its neighbours'
    (so a corner beside a let-go one is let go too), then the mean, so
    the goal changes smoothly."""
    v = values.copy()
    n = len(v)
    for _ in range(rounds):
        low = v.copy()
        np.minimum.at(low, edges[:, 0], v[edges[:, 1]])
        np.minimum.at(low, edges[:, 1], v[edges[:, 0]])
        total = low.copy()
        count = np.ones(n)
        np.add.at(total, edges[:, 0], low[edges[:, 1]])
        np.add.at(total, edges[:, 1], low[edges[:, 0]])
        np.add.at(count, edges[:, 0], 1)
        np.add.at(count, edges[:, 1], 1)
        v = total / count
    return v


def pin_weights(rest, goals, edges, frames):
    """The pin goal's weight per corner for a run: from the most skinning
    stretches or squeezes it in the frames read."""
    worst = np.zeros(len(rest))
    for f in frames:
        worst = np.maximum(worst, strain(rest, goals[f], edges))
    held = np.clip(1.0 - worst / LET_GO, 0.0, 1.0)
    w = FREE + (HOLD - FREE) * held
    return spread(w, edges, SPREAD)


def simulate(work, only_runs=None):
    import bpy  # noqa: F401  (Blender's Python)
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree

    bake = json.load(open(os.path.join(work, "bake.json")))
    body_rest = floats(os.path.join(work, "body.rest.f32"))
    body_tris = triangles(os.path.join(work, "body.triangles.u32"))
    # The garment's corners at one place (split for texture seams and
    # hard edges) are one corner of the cloth, so it doesn't come apart
    # at its seams; each takes its welded corner's result back.
    split_rest = floats(os.path.join(work, "garment.rest.f32"))
    rest, first, welded = np.unique(np.round(split_rest, 6), axis=0, return_index=True,
                                    return_inverse=True)
    welded = welded.ravel()
    rest = split_rest[first]
    tris = welded[triangles(os.path.join(work, "garment.triangles.u32"))]
    tris = tris[(tris[:, 0] != tris[:, 1]) & (tris[:, 1] != tris[:, 2]) & (tris[:, 2] != tris[:, 0])]
    edges = edges_of(tris)
    report = {"runs": []}

    def mesh_object(scene, name, P, faces):
        me = bpy.data.meshes.new(name)
        me.vertices.add(len(P))
        me.vertices.foreach_set("co", P.astype(np.float32).ravel())
        me.loops.add(len(faces) * 3)
        me.loops.foreach_set("vertex_index", faces.astype(np.int32).ravel())
        me.polygons.add(len(faces))
        me.polygons.foreach_set("loop_start", np.arange(0, len(faces) * 3, 3, dtype=np.int32))
        me.update()
        me.validate()
        ob = bpy.data.objects.new(name, me)
        scene.collection.objects.link(ob)
        return ob

    def keyed(ob, frames):
        """Absolute shape keys, one per frame, played one a frame."""
        ob.shape_key_add(name="Basis")
        for k in range(len(frames)):
            key = ob.shape_key_add(name=f"f{k}")
            key.data.foreach_set("co", frames[k].astype(np.float32).ravel())
        keys = ob.data.shape_keys
        keys.use_relative = False
        blocks = keys.key_blocks
        for k in range(len(frames)):
            keys.eval_time = blocks[k + 1].frame
            keys.keyframe_insert("eval_time", frame=k + 1)
        return keys

    for run in bake["runs"]:
        name = run["name"]
        if only_runs and name not in only_runs:
            continue
        started = time.time()
        F = run["frames"]
        body = floats(os.path.join(work, f"{name}.body.f32")).reshape(F, -1, 3)
        goals = floats(os.path.join(work, f"{name}.goal.f32")).reshape(F, -1, 3)[:, first]

        bpy.ops.wm.read_factory_settings(use_empty=True)
        scene = bpy.context.scene
        scene.frame_start, scene.frame_end = 1, F
        scene.render.fps = 24
        # The files' axes: Y is up.
        scene.gravity = (0.0, -GRAVITY, 0.0)

        # The skin's triangles face out (glTF's winding), as the
        # collision's one-sided test needs.
        collider = mesh_object(scene, "Body", body_rest, body_tris)
        keyed(collider, body)
        collider.modifiers.new("Collision", "COLLISION")
        collider.collision.thickness_outer = SKIN_DISTANCE
        collider.collision.thickness_inner = 0.004
        collider.collision.cloth_friction = 5.0
        collider.collision.damping = 0.5

        cloth_ob = mesh_object(scene, "Garment", rest, tris)
        keyed(cloth_ob, goals)
        reads = [r["frame"] for r in run["reads"]]
        solves = [r["frame"] for r in run["reads"] if r["kind"] == "solve"]
        pins = pin_weights(rest, goals, edges, solves)
        group = cloth_ob.vertex_groups.new(name="pin")
        for level in np.unique(np.round(pins, 3)):
            members = np.nonzero(np.round(pins, 3) == level)[0]
            group.add(members.tolist(), float(level), "REPLACE")
        cloth = cloth_ob.modifiers.new("Cloth", "CLOTH")
        s = cloth.settings
        s.mass = CLOTH["mass"]
        s.tension_stiffness = CLOTH["tension_stiffness"]
        s.compression_stiffness = CLOTH["compression_stiffness"]
        s.shear_stiffness = CLOTH["shear_stiffness"]
        s.bending_stiffness = CLOTH["bending_stiffness"]
        s.air_damping = CLOTH["air_damping"]
        s.quality = CLOTH["quality"]
        s.vertex_group_mass = "pin"
        s.pin_stiffness = PIN_STIFFNESS
        c = cloth.collision_settings
        c.collision_quality = 4
        c.distance_min = SKIN_DISTANCE
        c.use_self_collision = True
        c.self_distance_min = SELF_DISTANCE
        cloth.point_cache.frame_start = 1
        cloth.point_cache.frame_end = F

        out = np.zeros((len(reads), len(rest), 3), dtype=np.float32)
        for frame in range(1, F + 1):
            scene.frame_set(frame)
            if frame - 1 in reads:
                depsgraph = bpy.context.evaluated_depsgraph_get()
                evaluated = cloth_ob.evaluated_get(depsgraph)
                P = np.zeros(len(rest) * 3, dtype=np.float32)
                evaluated.data.vertices.foreach_get("co", P)
                for k, f in enumerate(reads):
                    if f == frame - 1:
                        out[k] = P.reshape(-1, 3)
        out[:, welded].astype("<f4").tofile(os.path.join(work, f"{name}.sim.f32"))
        # The cloth inside the skin at each read (more than the skin
        # distance in, along the skin's normal at the closest point).
        inside = []
        for k, f in enumerate(reads):
            tree = BVHTree.FromPolygons([tuple(p) for p in body[f]], body_tris.tolist())
            depth = 0.0
            count = 0
            for p in out[k]:
                hit = tree.find_nearest(Vector(p.tolist()), 0.05)
                if hit[0] is None:
                    continue
                d = (Vector(p.tolist()) - hit[0]).dot(hit[1])
                if d < -0.001:
                    count += 1
                    depth = max(depth, -d)
            inside.append({"corners": count, "deepest_mm": round(depth * 1000.0, 1)})
        # How far the cloth is from where skinning puts it at each read.
        moved = [float(np.linalg.norm(out[k] - goals[f], axis=1).max() * 1000.0)
                 for k, f in enumerate(reads)]
        report["runs"].append({
            "run": name,
            "frames": F,
            "seconds": round(time.time() - started, 1),
            "pinned_share": float((pins > 0.5).mean()),
            "free_share": float((pins < 0.1).mean()),
            "most_from_skinning_mm": moved,
            "inside_skin": inside,
        })
        print(f"simulated {name}: {F} frames in {report['runs'][-1]['seconds']} s; "
              f"most from skinning at the reads {[round(m, 1) for m in moved]} mm; "
              f"inside the skin {inside}", flush=True)
    json.dump(report, open(os.path.join(work, "simulate.json"), "w"), indent=1)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--library", required=True)
    parser.add_argument("--item", required=True)
    parser.add_argument("--work", required=True)
    parser.add_argument("--out", help="the shapes' file (default: <work>/correctives.json)")
    parser.add_argument("--only", help="some of " + ",".join(STEPS))
    parser.add_argument("--runs", help="simulate only these runs")
    args = parser.parse_args()
    steps = args.only.split(",") if args.only else STEPS
    os.makedirs(args.work, exist_ok=True)
    out = args.out or os.path.join(args.work, "correctives.json")
    library = os.path.abspath(args.library)
    work = os.path.abspath(args.work)
    if "dump" in steps:
        print(cargo("dump", library, args.item, work).strip())
    if "simulate" in steps:
        simulate(work, args.runs.split(",") if args.runs else None)
    if "solve" in steps:
        text = cargo("solve", library, args.item, work, os.path.abspath(out))
        open(os.path.join(work, "solve.json"), "w").write(text)
        print(text)
    if "add" in steps:
        print(cargo("add", library, args.item, os.path.abspath(out)).strip())


if __name__ == "__main__":
    main()
