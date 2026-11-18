# Test tee (a fixture, not a wardrobe item)

`TestTee.obj` (with `TestTee.mtl`) is a short-sleeved T-shirt for the garment import tests (Character Studio CO9, `crates/avatar/tests/fitting.rs`). It is deliberately **not** made from the body's surface: `make_test_tee.py` lofts it from a girth every 2 cm (the convex outline of the body's cross-section there, made the same on both sides, 22 mm out; below the armpit only the trunk, out to the first 3 cm gap before the arm) and two sleeve tubes sized by the upper arm's girth along it (14 mm out), joins them with a voxel remesh, and opens the hem and sleeve ends with straight plane cuts and a crew neck with an upright cylinder round the neck. A check stops the script if any girth comes out wider than the trunk at the armpit and the arm's root (the earlier version's girths caught the arm and grew flanges to ±0.31 m). It stands in for a third-party garment: an OBJ in the body's rest pose, roughly aligned. It stands in for a third-party garment: an OBJ in the body's rest pose, roughly aligned.

It is a **draft** by `docs/development/character-studio/asset-quality.md` (one layer, no pattern construction, seams, hems or folds) and is never shipped in the character library.

Made by the Axcenity Engine contributors; dedicated to the public domain under **CC0 1.0**.

Remake it (Blender's Python, bpy 5.x):

```bash
python -I crates/avatar/tests/fixtures/test-tee/make_test_tee.py --body assets/characters/realistic/RealisticParts.glb --out crates/avatar/tests/fixtures/test-tee
```
