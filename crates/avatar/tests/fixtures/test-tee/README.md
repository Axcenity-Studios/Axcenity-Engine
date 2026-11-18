# Test tee (a fixture, not a wardrobe item)

`TestTee.obj` (with `TestTee.mtl`) is a short-sleeved T-shirt for the garment import tests (Character Studio CO9, `crates/avatar/tests/fitting.rs`). It is deliberately **not** made from the body's surface: `make_test_tee.py` lofts it from a girth every 2 cm (the convex outline of the body's cross-section there, made the same on both sides, 22 mm out; below the armpit only the trunk, out to the first 3 cm gap before the arm) and two sleeve tubes round the upper arm's slices (14 mm out), joins them with a voxel remesh, and opens the hem and the sleeve ends with plane cuts and the neck with two cylinders. A check stops the script if any girth comes out wider than the trunk at the armpit and the arm's root (an earlier version's girths caught the arm and grew flanges to ±0.31 m). It stands in for a third-party garment: an OBJ in the body's rest pose, roughly aligned.

Since CO9 review fixes 2:

- **The underarm is closed.** The sleeves run 90% of the way to the elbow, their hems leaning longer underneath, so the hem is 6.6 cm below the armpit underneath (the earlier sleeve's hem was above the armpit, which left the side open; raised arms showed it as a tear). The script checks it.
- **An ordinary crew neck** (wave 4, CO9 part 2). The realistic sports bra's straps now sit on the shoulders, so the neck's opening is 14 mm wider than the neck's column at the sides, with a round scoop in front down to the hollow at the neck's base. The shell's top is let down onto the shoulders, to 14 mm over the skin (`drape_shoulders`: the girths' ease is across, so it stood up to 2.4 cm off the shoulder's top), and the collar's edge onto the skin (`settle_collar`: 2 mm over it, fading out over 12 mm, then smoothed along its line), so there is no slit under the collar to see the far shoulder through. The script checks that the shirt hides the bra from the capture views (front, three-quarter, side, back; level and from 5 degrees above) on both frames: nothing shows but 3 corners of the side band up a sleeve from straight beside the body on the feminine frame (let off, as up any short sleeve). The collar's edge is still a little wavy (a few millimetres) in captures.
- 3,670 vertices and 7,034 triangles (over the 6k a top may have; it is a draft).
- **`TestTeeEvenEase.obj`** is the tee as review fixes 2 left it (a close crew neck, the same ease all round): the alignment test (`alignment_puts_a_garment_made_too_high_and_too_small_back_on_the_body`) uses it, because the import's fine alignment assumes a garment stands off the body by about the same ease everywhere, which a shirt lying on the shoulders doesn't (on `TestTee.obj` the true place is only 13% more even than 5% small and 3 cm high, under the 20% and 2 mm the fine fit needs).

It is a **draft** by `docs/development/character-studio/asset-quality.md` (one layer, no pattern construction, seams, hems or folds) and is never shipped in the character library.

Made by the Axcenity Engine contributors; dedicated to the public domain under **CC0 1.0**.

Remake it (Blender's Python, bpy 5.x):

```bash
python -I crates/avatar/tests/fixtures/test-tee/make_test_tee.py --body assets/characters/realistic/RealisticParts.glb --out crates/avatar/tests/fixtures/test-tee
```
