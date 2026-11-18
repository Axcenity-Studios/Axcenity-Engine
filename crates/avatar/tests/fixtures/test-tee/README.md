# Test tee (a fixture, not a wardrobe item)

`TestTee.obj` (with `TestTee.mtl`) is a short-sleeved T-shirt for the garment import tests (Character Studio CO9, `crates/avatar/tests/fitting.rs`). It is deliberately **not** made from the body's surface: `make_test_tee.py` lofts it from a girth every 2 cm (the convex outline of the body's cross-section there, made the same on both sides, 22 mm out; below the armpit only the trunk, out to the first 3 cm gap before the arm) and two sleeve tubes round the upper arm's slices (14 mm out), joins them with a voxel remesh, and opens the hem and the sleeve ends with plane cuts and the neck with two cylinders. A check stops the script if any girth comes out wider than the trunk at the armpit and the arm's root (an earlier version's girths caught the arm and grew flanges to ±0.31 m). It stands in for a third-party garment: an OBJ in the body's rest pose, roughly aligned.

Since CO9 review fixes 2:

- **The underarm is closed.** The sleeves run 90% of the way to the elbow, their hems leaning longer underneath, so the hem is 6.6 cm below the armpit underneath (the earlier sleeve's hem was above the armpit, which left the side open; raised arms showed it as a tear). The script checks it.
- **A close crew neck.** An elliptic cylinder along the neck, 2 mm wider than its column, opens it at the neck's base, and a scoop brings the front down to the hollow. The realistic sports bra's straps come up beside the neck, so the script checks that the shirt hides the bra from the capture views (front, three-quarter, side, back; level and from 5 degrees above) on both frames. What shows: the straps' top at the collar's edge (20 corners on the masculine frame, 17 on the feminine, about 2 mm of strap beside the neck) and 2 or 3 corners of the side band up a sleeve from straight beside the body.
- 3,710 vertices and 7,124 triangles (over the 6k a top may have; it is a draft).

It is a **draft** by `docs/development/character-studio/asset-quality.md` (one layer, no pattern construction, seams, hems or folds) and is never shipped in the character library.

Made by the Axcenity Engine contributors; dedicated to the public domain under **CC0 1.0**.

Remake it (Blender's Python, bpy 5.x):

```bash
python -I crates/avatar/tests/fixtures/test-tee/make_test_tee.py --body assets/characters/realistic/RealisticParts.glb --out crates/avatar/tests/fixtures/test-tee
```
