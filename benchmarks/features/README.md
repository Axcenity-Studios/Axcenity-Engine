# Feature benchmark settings

Graphics settings files for `axcenity-player --graphics <file>`: each holds a benchmark run to these settings (as `graphics.json` holds them; a setting left out keeps the preset's), including whether the GPU is opened with hardware ray tracing, without touching the machine's own `graphics.json`. `benchmarks/baselines.json`'s showcase runs use them to draw the same scene and camera through each feature in turn. What the runs are and how to read them: `docs/benchmarks.md`, "Feature benchmark (showcase)".

| File | What it adds to Ultra |
|---|---|
| `ultra-rt-device.json` | The GPU opened with hardware ray tracing, no ray traced effect: what opening it costs on its own |
| `ultra-rt-shadows.json` | Ray traced sun and lamp shadows (and the fog's shafts), High (4 rays a pixel where soft) |
| `ultra-rt-ao.json` | Ray traced ambient occlusion, High (8 rays a pixel) |
| `ultra-rt-gi.json` | Ray traced global illumination, High (512 probes a frame, 256 rays each) |
| `ultra-rt-all.json` | All three at High, Ultra reflections (ray traced water reflections), volumetric fog and sun rays |
| `ultra-rt-all-upscaled.json` | The same at 67% render scale with TAA upscaling |

The presets alone (Low to Ultra) are run with `--quality`, not a file.

The `owner-*` files are the owner's editor settings (Ultra, shadows Ultra to 300 studs, TAA, reflection quality Ultra, contact shadows, an 8 GB texture budget) in the four configurations his frame rate targets of 2026-11-25 name. `baselines.json`'s `homestead-*` runs draw Desert Homestead with them at 1996 × 998 (`docs/rendering/performance.md`, "Encoding on several threads, and one query a ray").

| File | Configuration | Target |
|---|---|---|
| `owner-ultra.json` | Standard Ultra: no ray tracing, no real-time GI | 120 frames a second |
| `owner-ultra-gi.json` | Advanced lighting: real-time GI, no rays | 90 |
| `owner-rt-shadows-ao.json` | Ray traced shadows and ambient occlusion at High | 90 |
| `owner-rt-all.json` | Ray traced reflections at High as well (what he runs) | 60 (100 for Desert Homestead) |
