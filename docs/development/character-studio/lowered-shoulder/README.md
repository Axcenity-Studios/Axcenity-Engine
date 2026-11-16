# The lowered shoulder (2026-11-16)

The realistic body's `Corrective_Shoulder_Lowered_*` shapes, made after Paul saw the shoulders deformed in Character Studio's idle preview. The account is in [progress.md](../progress.md).

| File | What |
|---|---|
| `idle-before-after-*.jpg` | The in-engine capture one second into Idle (`AXCENITY_CAPTURE_POSE=idle`), before (the previous `RealisticParts.glb`, through `AXCENITY_CAPTURE_LIBRARY`) on the left and after on the right: studio front, three-quarter and back, and hard light front, cropped to the upper body |
| `measurements.json`, `measure-check.txt` | `measure.py --package realistic --check …` on the rebuilt body: every limit holds, the new `shoulder_abd_10` lines included |

The arm hanging (`shoulder_abd_10`): local volume 1.125 / 1.093 before (masculine / feminine), 1.024 / 0.986 after; ring area 0.997 / 0.988 before, 0.907 / 0.962 after.
