# CO8: compute posing, captures (`co8-compute-posing`)

In-engine captures of the benchmark cast with **hardware ray-traced sun shadows on** (`AXCENITY_CAPTURE_RT=shadows`, added for this: the High preset with `ray_traced_shadows: High`; every preset has them off), idle pose, 1280 × 720, TAA, on an RX 9070 XT (DX12). Each crop is three columns: **before** (the base branch `refinement-wave2`, `b39a9ed7`, RT shadows), **after** (this branch, RT shadows), and the same after-binary with **shadow maps only** (the reference: the shadow maps have always drawn the posed, morphed character). What changed: [../co8-compute-posing.md](../co8-compute-posing.md).

| Sheet | What it shows |
|---|---|
| `slim-hard-side.jpg`, `slim-hard-three-quarter.jpg`, `slim-outdoor-three-quarter.jpg` | `realistic-feminine-cast-slim`: before, the ray-traced copy was the unmorphed base body, larger than the slim one, so the slim body sat inside it and shadowed itself (back, arm, legs, scalp); after, the self-shadowing matches the shadow maps |
| `heavyset-hard-three-quarter.jpg`, `heavyset-outdoor-front.jpg` | `realistic-masculine-cast-heavyset`: the heavier body is mostly outside the base copy, so the change is small: in the hard three-quarter view the dark specks on the briefs (the base copy's surface poking through) are gone; from the front under outdoor light I can see no difference on the body, only in the floor shadow's outline |
| `happy-face-outdoor-side.jpg`, `happy-face-outdoor-three-quarter.jpg` | `realistic-masculine-cast-distinctive` with `Happy:1`: the jaw's shadow on the neck follows this face's jaw and the smile, not the base face's |
| `*-sheet.jpg` | The capture tool's whole sheets (rows: outdoor, hard; columns: the three views), before above after |

Measured on the full captures (pixels differing by more than 8 of 255, before against after; and, over those pixels, the mean distance of each RT picture from the shadow-map picture):

| Character, light, view | Pixels changed | Before → shadow maps | After → shadow maps |
|---|---|---|---|
| slim, hard, side | 1.81% | 69.6 | 0.7 |
| slim, hard, three-quarter | 1.77% | 57.3 | 2.4 |
| slim, outdoor, three-quarter | 2.56% | 37.7 | 2.0 |
| heavyset, hard, three-quarter | 0.22% | 48.1 | 9.5 |
| heavyset, outdoor, front | 0.25% | 18.5 | 5.7 |
| distinctive Happy, outdoor, face side | 0.26% | 19.0 | 4.2 |
| distinctive Happy, outdoor, face three-quarter | 0.21% | 20.1 | 9.0 |
| distinctive Happy, hard, face side | 0.00% | – | – |

The full captures and the comparison scripts stay outside the repository; the commands are in the hand-back.
