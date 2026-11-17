# DIS-18: apply time of a parameter change (R1, measured 2026-11-17)

The refinement plan (11.4, DIS-18) asks for the apply time during a drag to be measured in a release build before the drag behaviour changes: over 33 ms means the drag should preview with live morph weights and bake on release. Nothing about the drag was changed here (the 120 ms throttle, `DRAG_APPLY_EVERY`, stays).

**Test:** `crates/editor/src/character_workspace/timing_tests.rs`, `drag_apply_time` (ignored by default):

```text
cargo test -p axcenity-editor --release -- --ignored drag_apply_time --nocapture
```

It opens the workspace on the realistic default character (Male, underwear only), warms up three frames, and for each parameter takes 20 slider steps across its range. For each step it times:

- **apply**: `gameplay::description::apply_description` alone (resolve, the identity, the body's bake, attributes): what runs when the preview is refreshed.
- **apply + step + draw list**: that, then one `Animators` step (an owner whose model changed rebinds its clips) and `build_draw_list_posed` (the worn parts' derived models are baked there on first use). The GPU upload is **not** included.

**Machine:** AMD Ryzen 9 9950X3D (16 cores), AMD Radeon RX 9070 XT, 62 GB, Windows 11. Other agents were compiling at the same time; runs 2 and 3 were pinned to the V-cache CCD (affinity 0xFFFF) by starting the test binary directly.

## Results (milliseconds, min / median / max of 20 steps)

| Parameter | What it drives | apply, run 1 (unpinned) | apply, run 2 (pinned) | apply, run 3 (pinned) | apply + step + draw list, run 2 (pinned) |
|---|---|---|---|---|---|
| `fat` | body shapes | 2.78 / 3.62 / 18.17 | 2.18 / 2.70 / 13.62 | 2.55 / 3.46 / 15.78 | 5.20 / 6.61 / 19.53 |
| `muscle` | body shapes | 2.53 / 3.60 / 15.07 | 1.90 / 2.81 / 13.08 | 2.55 / 3.28 / 13.96 | 4.58 / 6.09 / 16.28 |
| `height` | attribute only | 0.08 / 0.11 / 2.86 | 0.08 / 0.11 / 2.20 | 0.09 / 0.09 / 3.02 | 0.16 / 0.22 / 5.12 |
| `fat_abdomen` | regional shape | 2.62 / 3.13 / 13.45 | 1.72 / 2.56 / 10.37 | 2.04 / 2.61 / 12.89 | 4.45 / 6.47 / 14.36 |
| `nose_length` | face shape | 1.97 / 2.74 / 13.25 | 1.87 / 2.33 / 10.29 | 1.58 / 2.64 / 10.44 | 4.75 / 6.44 / 13.36 |
| `eye_size_left` | face shape (one side) | 2.07 / 2.79 / 13.93 | 1.83 / 2.17 / 10.77 | 1.63 / 2.41 / 11.56 | 4.89 / 6.21 / 16.85 |
| `jaw_width_left` | face shape (one side) | 2.48 / 2.82 / 12.09 | 1.91 / 2.18 / 11.39 | 1.63 / 2.11 / 9.98 | 4.75 / 5.36 / 15.83 |

The apply + step + draw list column of run 3 was within a millisecond of run 2's except one step of `fat` at 70.18 ms (a single outlier, not reproduced in the other runs).

## Reading

- On this machine a shape parameter applies in **2 to 3.6 ms (median)**, about 6 to 8 ms with the next frame's step and draw list, and at most about 20 ms in the worst step of each run except the one 70 ms outlier. That is under DIS-18's 33 ms, so by the plan's rule the drag needn't move to live morph weights yet. The 120 ms throttle is now the larger delay: a drag shows about eight updates a second where the work allows thirty or more. Lowering it is a separate decision (not taken here: "don't change the drag behaviour yet").
- An attribute-only parameter (height and the eight proportions) costs about 0.1 ms: no bake.
- The worst step of each parameter (10 to 18 ms) is usually the first: a new identity's first bake allocates; later steps reuse.
- **Not covered:** the GPU upload of the new buffers; a dressed character with hair (more parts to bake per step: the realistic default wears only underwear today); the minimum development GPU class (RX 6600 / RTX 3060) and its usual CPU. Measure again when R6 and R7 add hair and clothes, and on a slower machine, before deciding the throttle or the live-morph path.

## After R2a (live identity)

Refinement R2a made identity live weights (ADR-0041), and the same test now times a live and a baked pass per parameter: a live apply is 0.06 to 0.12 ms (median) and 0.14 to 0.25 ms with the next step and draw list, so the 120 ms throttle was removed. Numbers and method: [../r2/r2a-live-identity.md](../r2/r2a-live-identity.md).
