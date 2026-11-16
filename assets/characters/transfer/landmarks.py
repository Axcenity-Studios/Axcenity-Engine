"""Step 6 of the MakeHuman transfer spike: where fat goes and where it
stays off (bony landmarks), and regional layers, as prototypes.

    python -I landmarks.py --work WORK --out ../../../docs/development/character-studio/co5-spike

(`python` has the `bpy` module and Pillow; run `transfer.py` first.)
Writes `WORK/landmarks.npz` and the sheet `landmarks.png`, and prints the
numbers the README quotes.

**Fat depth, from the data.** Along MakeHuman's weight slider at average
muscle (min weight to max weight), its artists kept fat off the bones: the
skin over a shin or a kneecap moves outward by under a millimetre while a
thigh's side moves by two centimetres. Per vertex, `depth` is how far the
transferred skin moves along its normal between those two shapes, in
metres (MakeHuman's joints don't move with these targets, so this is depth
over the bone, not the limb shifting). `bony` is 1 where that is under
1.5 mm and 0 where it is over 6 mm, smooth between: the mask CO5 would
multiply any *made* fat by (regional exaggeration, fat beyond MakeHuman's
range), so it never covers a landmark the artists left bare.

MakeHuman's **extreme** corner (min muscle, max weight) is another matter:
there its own shins, knees and elbows thicken by two to three centimetres
(`depth_extreme`, printed beside the other). The last two tiles show that
corner as transferred and with the mask applied to it, for the lead to
choose between.

**The face sets, for comparison.** The bundle's face sets mark joints
(the rings between sets run round elbows, knees, wrists, ankles) but not
bone that lies under the skin along a limb (shin, clavicle, spine), so
they can't make this mask alone. `ring` is 1 on the rings and falls to 0
over 4 cm, for the comparison printed here.

**Regions.** `regions` holds smooth weights per vertex for CO5's regional
layers (chest, back, abdomen, hips, arms, thighs, calves, and the rest),
made from the face sets and adding up to 1, so the regional shapes add up
to the global one exactly.
"""

import argparse
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import geom  # noqa: E402
import skeleton  # noqa: E402

REGIONS = ("chest", "back", "abdomen", "hips", "arms", "thighs", "calves", "rest")
LOW, HIGH = 0.0015, 0.006


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0.0, 1.0)
    return t * t * (3 - 2 * t)


def fat_depth(shapes, frame, normals, muscle="average"):
    """How far the skin moves outward from min to max weight at one
    muscle setting, per vertex, in metres."""
    d = (shapes[f"posed/{frame}/{muscle}muscle_maxweight"].astype(np.float64)
         - shapes[f"posed/{frame}/{muscle}muscle_minweight"].astype(np.float64))
    return np.einsum("ij,ij->i", d, normals)


def ring_mask(P, sets, edges, reach=0.04):
    """1 on the rings round the limb joints, falling to 0 over `reach`
    metres along the surface (a few rounds of spreading over edges)."""
    S = skeleton
    rings = []
    for s in (1, -1):
        rings += [sets.ring(S.FS_UPPER_ARM[s], S.FS_FOREARM[s]), sets.ring(S.FS_FOREARM[s], S.FS_PALM[s]),
                  sets.ring(S.FS_THIGH[s], S.FS_SHIN[s]), sets.ring(S.FS_SHIN[s], S.FS_FOOT[s])]
    dist = np.full(len(P), np.inf)
    dist[np.concatenate(rings)] = 0.0
    length = np.linalg.norm(P[edges[:, 0]] - P[edges[:, 1]], axis=1)
    for _ in range(12):
        for a, b in ((0, 1), (1, 0)):
            np.minimum.at(dist, edges[:, a], dist[edges[:, b]] + length)
    return 1.0 - smoothstep(0.0, reach, dist)


def region_weights(P, normals, sets, lap, lm):
    """(n, 8) smooth weights, rows adding to 1, in `REGIONS` order."""
    S = skeleton
    n = len(P)
    hard = np.full(n, REGIONS.index("rest"))
    front = normals[:, 1] < 0.15
    side = np.abs(P[:, 0]) > 0.6 * np.abs(P[sets.of[S.FS_PELVIS], 0]).max()
    torso, waist, pelvis = (np.zeros(n, dtype=bool) for _ in range(3))
    torso[sets.of[S.FS_TORSO]] = True
    waist[sets.of[S.FS_WAIST]] = True
    pelvis[sets.of[S.FS_PELVIS]] = True
    hard[torso & front] = REGIONS.index("chest")
    hard[torso & ~front] = REGIONS.index("back")
    hard[waist & front] = REGIONS.index("abdomen")
    hard[waist & ~front] = REGIONS.index("back")
    hard[pelvis] = REGIONS.index("hips")
    hard[pelvis & front & ~side & (P[:, 2] > lm["crotch"][2] + 0.06)] = REGIONS.index("abdomen")
    for s in (1, -1):
        hard[sets.verts(S.FS_UPPER_ARM[s], S.FS_FOREARM[s])] = REGIONS.index("arms")
        hard[sets.of[S.FS_THIGH[s]]] = REGIONS.index("thighs")
        hard[sets.of[S.FS_SHIN[s]]] = REGIONS.index("calves")
    W = np.eye(len(REGIONS))[hard]
    for _ in range(6):
        W = (W + lap.mean(W)) / 2
    return W / W.sum(axis=1, keepdims=True)


def heat(value, top):
    """A colour per value: cold blue at 0 through pale to warm red at `top`."""
    t = np.clip(value / top, 0.0, 1.0)[:, None]
    cold, mid, warm = np.array([0.10, 0.25, 0.75]), np.array([0.85, 0.85, 0.80]), np.array([0.80, 0.15, 0.10])
    return np.where(t < 0.5, cold + (mid - cold) * (t * 2), mid + (warm - mid) * (t * 2 - 1))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--samples", type=int, default=24)
    a = ap.parse_args()
    M = np.load(os.path.join(a.work, "makehuman.npz"))
    R = np.load(os.path.join(a.work, "realistic.npz"))
    G = np.load(os.path.join(a.work, "registration.npz"))
    shapes = np.load(os.path.join(a.work, "shapes.npz"))
    faces = R["faces"]
    tris, edges = geom.triangles(faces), geom.edges(faces)
    sets = skeleton.Sets(faces, R["face_sets"])
    lap = geom.Laplacian(len(R["masculine"]), edges)
    lm_names = G["landmark_names"].tolist()
    data = {}
    for frame in ("masculine", "feminine"):
        P = G[f"posed_{frame}"]
        normals = geom.vertex_normals(P, tris)
        lm = dict(zip(lm_names, G[f"landmarks_{frame}"]))
        depth = fat_depth(shapes, frame, normals)
        extreme = fat_depth(shapes, frame, normals, "min")
        bony = 1.0 - smoothstep(LOW, HIGH, depth)
        data[f"depth_extreme_{frame}"] = extreme.astype(np.float32)
        ring = ring_mask(P, sets, edges)
        W = region_weights(P, normals, sets, lap, lm)
        data[f"depth_{frame}"], data[f"bony_{frame}"] = depth.astype(np.float32), bony.astype(np.float32)
        data[f"ring_{frame}"], data[f"regions_{frame}"] = ring.astype(np.float32), W.astype(np.float32)
        print(f"{frame}: fat depth (min to max weight at average muscle, along the normal) median {np.median(depth) * 1000:.1f} mm, "
              f"95% {np.percentile(depth, 95) * 1000:.1f} mm, largest {depth.max() * 1000:.1f} mm; "
              f"bony (under {LOW * 1000:.1f} mm): {(depth < LOW).mean() * 100:.0f}% of the vertices")
        S = skeleton
        spots = {
            "elbow ring": sets.ring(S.FS_UPPER_ARM[1], S.FS_FOREARM[1]),
            "wrist ring": sets.ring(S.FS_FOREARM[1], S.FS_PALM[1]),
            "knee ring": sets.ring(S.FS_THIGH[1], S.FS_SHIN[1]),
            "ankle ring": sets.ring(S.FS_SHIN[1], S.FS_FOOT[1]),
            "upper arm": sets.of[S.FS_UPPER_ARM[1]], "forearm": sets.of[S.FS_FOREARM[1]],
            "thigh": sets.of[S.FS_THIGH[1]], "shin": sets.of[S.FS_SHIN[1]],
            "hand": sets.of[S.FS_PALM[1]], "foot": sets.of[S.FS_FOOT[1]],
            "waist": sets.of[S.FS_WAIST], "pelvis": sets.of[S.FS_PELVIS], "chest and upper back": sets.of[S.FS_TORSO],
            "head and neck": sets.of[S.FS_HEAD],
        }
        shin = sets.of[S.FS_SHIN[1]]
        spots["shin, front third"] = shin[normals[shin, 1] < -0.6]
        spots["shin, back third"] = shin[normals[shin, 1] > 0.6]
        torso = sets.of[S.FS_TORSO]
        top = P[torso, 2] > lm["l-shoulder"][2] - 0.02
        spots["collarbones and shoulder tops"] = torso[top & (normals[torso, 1] < 0)]
        print("  | Place | Fat depth mm, median | Share bony | At the extreme corner mm |")
        print("  |---|---|---|---|")
        for k, v in spots.items():
            print(f"  | {k} | {np.median(depth[v]) * 1000:.1f} | {(depth[v] < LOW).mean() * 100:.0f}% "
                  f"| {np.median(extreme[v]) * 1000:.1f} |")
        limb = sets.verts(S.FS_UPPER_ARM[1], S.FS_FOREARM[1], S.FS_THIGH[1], S.FS_SHIN[1])
        on_ring = ring[limb] > 0.5
        print(f"  left limbs: {(bony[limb] > 0.5).mean() * 100:.0f}% of the vertices are bony by the data; "
              f"of those, {(on_ring & (bony[limb] > 0.5)).sum() / max((bony[limb] > 0.5).sum(), 1) * 100:.0f}% "
              f"lie within reach of a joint ring; of the ring vertices, "
              f"{(bony[limb][on_ring] > 0.5).mean() * 100:.0f}% are bony")
    np.savez_compressed(os.path.join(a.work, "landmarks.npz"), **data)

    import render
    from sheets import Sheet, paste
    sh = Sheet(render.Studio(330, 620, a.samples), os.path.join(a.work, "tiles"))
    os.makedirs(os.path.join(a.work, "tiles"), exist_ok=True)
    rows = []
    for frame in ("masculine", "feminine"):
        P = G[f"posed_{frame}"]
        row = []
        for label, colors in (("fat depth, 0 to 15 mm", heat(data[f"depth_{frame}"], 0.015)),
                              ("bony mask from the data", heat(1 - data[f"bony_{frame}"], 1.0)),
                              ("joint rings from face sets", heat(1 - data[f"ring_{frame}"], 1.0))):
            for view, azimuth in (("front", 0), ("back", 180)):
                row.append(sh.tile(P, faces, azimuth, f"{frame}, {view}: {label}", colors))
        rows.append(row)
    # Regional layers: the heaviest shape split by region.
    P = G["posed_masculine"]
    d = shapes["posed/masculine/minmuscle_maxweight"].astype(np.float64)
    W = data["regions_masculine"].astype(np.float64)
    hues = np.array([[0.85, 0.35, 0.3], [0.3, 0.45, 0.8], [0.9, 0.75, 0.3], [0.55, 0.35, 0.7],
                     [0.35, 0.7, 0.45], [0.85, 0.55, 0.25], [0.3, 0.7, 0.75], [0.7, 0.7, 0.7]])
    row = [sh.tile(P, faces, 0, "regions (front)", W @ hues), sh.tile(P, faces, 180, "regions (back)", W @ hues)]
    for label, picks in (("abdomen only", ("abdomen",)), ("hips + thighs only", ("hips", "thighs"))):
        w = sum(W[:, REGIONS.index(p)] for p in picks)
        row.append(sh.tile(P + d * w[:, None], faces, 35, f"heavy + soft, {label}"))
    row.append(sh.tile(P + d, faces, 0, "heavy + soft, as transferred"))
    keep = 1.0 - data["bony_masculine"].astype(np.float64)
    row.append(sh.tile(P + d * keep[:, None], faces, 0, "heavy + soft, bony mask applied"))
    rows.append(row)
    paste(rows, os.path.join(a.out, "landmarks.png"),
          "Where MakeHuman's data puts fat (blue: none, red: most), the bony mask it gives, the face sets' joint "
          "rings for comparison; bottom: regions, two regional layers, and the extreme corner with the mask")


if __name__ == "__main__":
    main()
