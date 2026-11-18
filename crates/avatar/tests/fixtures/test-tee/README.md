# Test tee (a fixture, not a wardrobe item)

`TestTee.obj` (with `TestTee.mtl`) is a short-sleeved T-shirt for the garment import tests (Character Studio CO9, `crates/avatar/tests/fitting.rs`). It is deliberately **not** made from the body's surface: `make_test_tee.py` builds it from a stack of ellipses and two cylinders sized by a tailor's measurements of the realistic body (girths, shoulder, neck, upper arm), joins them with a voxel remesh and cuts the hem, neck and sleeves open. It stands in for a third-party garment: an OBJ in the body's rest pose, roughly aligned.

It is a **draft** by `docs/development/character-studio/asset-quality.md` (one layer, no pattern construction, seams, hems or folds) and is never shipped in the character library.

Made by the Axcenity Engine contributors; dedicated to the public domain under **CC0 1.0**.

Remake it (Blender's Python, bpy 5.x):

```bash
python -I crates/avatar/tests/fixtures/test-tee/make_test_tee.py --body assets/characters/realistic/RealisticParts.glb --out crates/avatar/tests/fixtures/test-tee
```
