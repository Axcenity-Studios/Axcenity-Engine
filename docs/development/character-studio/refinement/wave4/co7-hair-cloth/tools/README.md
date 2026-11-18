# CO7 evidence tools

The scripts that made this folder's comparison sheets, kept so the comparison can be re-run on the first real hairstyle and garment.

1. Engine pictures: `AXCENITY_RENDER_EVIDENCE=<folder> cargo test -p axcenity-render --release -- --ignored hair_and_cloth_evidence --nocapture` writes `engine-*.ppm` (the scenes are `scenes()` in `crates/render/src/renderer/tests/character_models_evidence.rs`).
2. Blender references: `blender_references.py <folder> [scene ...]`, run with a Python that has Blender 5's `bpy` and numpy, renders the same scenes in Cycles (Principled Hair BSDF, Chiang model with direct colouring; Principled BSDF sheen and anisotropy) at the same camera and sun, as `blender-*.png`.
3. Sheets, with a Python that has Pillow and numpy:
   - `engine_vs_blender_sheet.py <folder> [out]`: `engine-vs-blender.jpg` and the per-scene measurements it prints;
   - `backlit_double_sided_sheet.py <engine folder> <out> <blender folder>`: `backlit-double-sided-hair-over-head-x3.png` (review fixes 2).
