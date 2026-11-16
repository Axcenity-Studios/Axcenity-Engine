"""Step 1 of the MakeHuman transfer spike: gathers the two sides' data into
`WORK/realistic.npz` and `WORK/makehuman.npz`.

    python -I extract.py --bundle path/to/human_base_meshes_bundle.blend \
        --makehuman path/to/makehuman-checkout --work WORK

(`python` is one with the `bpy` module; `fetch_makehuman.py` makes the
checkout.) Nothing here is committed: both files are intermediates.

`realistic.npz`: the realistic male and female bodies of Blender Studio's
Human Base Meshes v1.4.1 (CC0), in the bundle's vertex order (10,582
vertices; vertex `i` of one is vertex `i` of the other) and the bundle's
pose (arms about 70 degrees below level), with the base vertices moved
onto the sculpt (multiresolution level 3) as the CO4 package vendors them.
`faces` (-1 pads the 20 triangles), `face_sets` (per face) are the male
body's.

`makehuman.npz`: MakeHuman's base mesh body (13,380 vertices, quads), its
joint centres (from the joint cubes in the base mesh), each gender's
neutral (default ethnicity mix, young), and the 16 "universal" corner
targets at the young age (gender x muscle x weight; the two average ones
are empty) with how each moves the joints, plus a few regional targets,
all as dense arrays in metres, Z up, facing -Y.
"""

import argparse
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import mh  # noqa: E402

BODIES = {"masculine": "GEO-body_male_realistic", "feminine": "GEO-body_female_realistic"}
SCULPT_LEVEL = 3

# Regional targets carried along to show the method isn't limited to the
# macro corners: {our name: (target, gender whose neutral it is shown on)}.
EXTRA = {
    "pregnant": "stomach/stomach-pregnant-incr",
    "stomach_tone": "stomach/stomach-tone-incr",
    "buttocks_volume": "buttocks/buttocks-volume-incr",
    "pectoral": "torso/torso-muscle-pectoral-incr",
    "breast_maxcup": "breast/female-young-averagemuscle-averageweight-maxcup-averagefirmness",
    "breast_mincup": "breast/female-young-averagemuscle-averageweight-mincup-averagefirmness",
}


def realistic(bundle, out):
    import bpy

    bpy.ops.wm.open_mainfile(filepath=bundle)
    data = {}
    faces_ref = None
    for frame, name in BODIES.items():
        obj = bpy.data.objects[name]
        obj.animation_data_clear()
        me = obj.data
        count = len(me.vertices)
        multires = obj.modifiers["Multires"]
        multires.levels = SCULPT_LEVEL
        bpy.context.view_layer.update()
        deps = bpy.context.evaluated_depsgraph_get()
        ev = obj.evaluated_get(deps).to_mesh()
        co = np.empty(len(ev.vertices) * 3)
        ev.vertices.foreach_get("co", co)
        sculpt = co.reshape(-1, 3)[:count].copy()
        cage = np.array([v.co[:] for v in me.vertices])
        # A subdivided mesh lists the base mesh's vertices first.
        assert np.linalg.norm(sculpt - cage, axis=1).max() < 0.02, name
        data[frame] = sculpt
        faces = {}
        for poly in me.polygons:
            vs = tuple(poly.vertices)
            k = vs.index(min(vs))
            faces[vs[k:] + vs[:k]] = poly.index
        if faces_ref is None:
            faces_ref = set(faces)
            sets = np.array([d.value for d in me.attributes[".sculpt_face_set"].data])
            data["faces"] = np.array([list(p.vertices) + [-1] * (4 - len(p.vertices))
                                      for p in me.polygons], dtype=np.int64)
            data["face_sets"] = sets
            uv = me.uv_layers.active.data
            data["corner_uv"] = np.array([d.uv[:] for d in uv], dtype=np.float32)
        else:
            assert set(faces) == faces_ref, "the two bodies don't share one topology"
        print(f"{frame}: {count} vertices, {len(me.polygons)} faces, "
              f"height {np.ptp(sculpt[:, 2]):.3f} m")
    np.savez_compressed(out, **data)


def makehuman(root, out):
    base = mh.load_base(root)
    n = mh.BODY_VERTS
    data = {"base": base["verts"][:n], "faces": base["faces"]}
    names = sorted(base["joints"])
    data["joint_names"] = np.array(names)
    data["joints"] = np.array([base["joints"][j] for j in names])
    # The joint cubes move with the targets too (they are vertices of the
    # base mesh), so each gender's neutral has its own joints.
    cubes = [base["groups"]["joint-" + j] for j in names]
    for gender in mh.GENDERS:
        off = mh.neutral_offset(root, gender)
        data[f"neutral_{gender}"] = base["verts"][:n] + off[:n]
        data[f"joints_{gender}"] = np.array([(base["verts"][c] + off[c]).mean(axis=0) for c in cubes])
        for muscle in mh.MUSCLES:
            for weight in mh.WEIGHTS:
                t = mh.load_target(root, mh.corner_name(gender, muscle, weight))
                data[f"corner_{gender}_{muscle}_{weight}"] = t[:n].astype(np.float32)
                # How the target moves each joint (its cube's vertices).
                data[f"jcorner_{gender}_{muscle}_{weight}"] = np.array(
                    [t[c].mean(axis=0) for c in cubes], dtype=np.float32)
    for name, target in EXTRA.items():
        data[f"extra_{name}"] = mh.load_target(root, target)[:n].astype(np.float32)
    np.savez_compressed(out, **data)
    print(f"makehuman: {n} body vertices, {len(base['faces'])} quads, {len(names)} joints, "
          f"{sum(k.startswith('corner_') for k in data)} corner targets, {len(EXTRA)} extras")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bundle", required=True)
    ap.add_argument("--makehuman", required=True)
    ap.add_argument("--work", required=True)
    a = ap.parse_args()
    os.makedirs(a.work, exist_ok=True)
    makehuman(a.makehuman, os.path.join(a.work, "makehuman.npz"))
    realistic(a.bundle, os.path.join(a.work, "realistic.npz"))


if __name__ == "__main__":
    main()
