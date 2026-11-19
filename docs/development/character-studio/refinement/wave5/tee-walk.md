# CO9 part 3: hand-back (the walking fit test, refinement wave 5)

Branch `tee-walk`, from `refinement-wave5` (main, waves 1 to 4, PR #428, Paul's decisions of 2026-11-18). Task: Paul's decision 13, "the T-shirt must pass the walking animation fit test, not just the static character fit test", and arm movement with shoulder deformation; fix the fit, not one mesh.

**In one paragraph.** **The main goal is not met: no crew-neck tee passes the walk.** I found the cause and measured it. Most of what the gate called a failure in the walk was the gate's own measure, not the cloth. It took the nearest cloth as "the garment", and in a loose tee that is often another part of the shirt. At the wave-4 walk frame, 94% of the failing corners (2,660 of 2,822; 2,758 by the smooth normal alone, review fixes 2) were skin that the tee encloses: the way out along the skin's normal meets the cloth from inside, 17 to 35 mm out, at the same place and distance as at rest. Another part of the shirt had simply come nearer: a sleeve swung beside the chest, or the torso's panel beside the arm inside its sleeve. I corrected the measure (`enclosed`, red first; since review fixes 2 "from inside" also needs the cloth's posed triangle to face that way), and with the fit unchanged the walk-frame record fell from 54 bodies at 30.2 mm to 38 bodies at 10.2 mm. What is left is real, and visible in 4K captures: a slit about 1 cm long at the front of the armpit of the arm swung back, with skin showing on the masculine bodies and the black bra on the feminine heavyset body. Its cause is the bridge of cloth between the arm and the side: when the arm hangs it closes on the side, and the bridge can't fold into the crease. I added the walking fit test (the whole stride, every third frame, on all 54 gate bodies) and arms forward to the gate. Five changes to the fit itself didn't pass it, so none is in the commit. The test tee fails the stride on all 54 bodies in every third, up to 16.5 mm. The wave-4 realistic crew tee fails it on every body, up to 38.7 mm.

## Files

| File | What |
|---|---|
| `crates/player/src/fit_gate_tests.rs` | `poke_through` takes a corner the garment encloses as inside (`enclosed`, `POKE_REACH`); `AXCENITY_FIT_GATE_CLOSEST_ONLY` (the old measure, to compare); the debug print says where the way out meets the cloth. The walking fit test: `WALK_FRAMES`, `WALK`, `measure_walk`, `gate_walk`, three tests. The arms-forward test. The synthetic test `skin_a_loose_garment_encloses_is_inside_and_skin_through_it_is_out`, with `corner` and `joined`. Records: lowered or new (below). Review fixes 2: `from_inside` (the face condition), the flipped-fold case in the synthetic test, `check` printing every group it measured, the debug print's `outside, flipped`. |
| `crates/player/src/wardrobe_tests.rs` | `wardrobe_gate_walk`, the three walk-third tests and the arms-forward test, with records. Records the corrected measure brought down are lowered. The old `WALK` record list is renamed `WALK_18`. |
| `crates/player/src/character_capture.rs` | `HeldPose::ArmsForward`: the arms let down to hanging (the shortest turn), then raised forward to level. It is not in `HeldPose::ALL`, so the body suite keeps its rows. `AXCENITY_CAPTURE_POSE=arms-forward` takes it, and the held-pose test checks it (straight ahead within 1 degree). `bend`'s last step is now `turned_by`. |
| `docs/development/character-studio/asset-quality.md` | §1 (decision 13's row): how the walk is run. §3 "Poke-through": how it is measured. No limit changed. |
| `docs/development/character-studio/testing.md` | A wave-5 section. |
| `wiki/Engine-Systems/Characters.md`, `wiki/Roadmap/Known-Limitations.md` | The walking fit test, the corrected measure, the test tee's and crew tee's walk, and what was tried. |
| `refinement/wave5/tee-walk-red-run.txt`, `tee-walk/*.jpg` | The red run, and the sheets (below). |

**No change to the fit.** `crates/avatar` is as it was. The test tee's fitted GLB in the gate library is byte-identical to wave 4's (`cmp` on the two libraries). There are no generated files, no schema changes and no new dependencies.

## Interfaces (player, tests only)

- `fit_gate_tests::{WALK_FRAMES, WALK, measure_walk}`.
- `fit_gate_tests::enclosed` and `from_inside` (private).
- `HeldPose::ArmsForward`.
- `AXCENITY_FIT_GATE_CLOSEST_ONLY=1`: the old measure, for comparison only. Never use it for a record.

## The cause, measured

**1. The gate's measure (94% of the wave-4 cases).**
- **The old rule.** `poke_through` took the closest point of the whole garment. Where its normal faced the skin, the skin counted as out by its distance past that cloth.
- **What I measured.** At Walk@18 I cast each failing corner's ray out along its normal against the tee:
  - 2,758 of the 2,822 failing corners (body and bra) meet the tee **from inside**, within 10 cm;
  - 53 meet it from outside first;
  - 11 meet nothing.
- **Two examples** (masculine average, Python on the gate's own dumps):
  - **Chest corner 8781** (rest (0.098, 1.219, 0.124) m). Its ray meets triangle 3952 at 24.9 mm at rest and at 24.8 mm in the pose, from inside. The "closest cloth" was the left sleeve's inner wall, 22.9 mm to the side, its outside towards the chest.
  - **Upper-arm corner 9700** (rest (0.278, 1.283, 0.040) m). It is inside its sleeve (30.6 mm, then 29.9 mm). The torso's panel came 25.8 mm beside it.
- **Wrong in both directions.** The same rule counted the stylized cargo pants 27.2 mm through in the walk; enclosed, they are 1.0 mm. It counted the boxer briefs 4.1 mm through in frames the wave-4 gate never sampled; they are none.
- **The correction.** Where the closest point says out, the gate casts the corner's way out along its normal, within the same 5 cm it looks for the closest point. If that meets the garment from inside, the garment encloses the corner, and it is out only by how far it is past that cloth. This is the test the gate's holes already use for "shut in" (`avatar::fit::outside_cloth`, accepted in wave 4's review fixes 2), there within 15 cm.
- **From inside, since review fixes 2,** means both the hit's smooth normal (its corners' skinned rest normals, mixed) and its posed triangle's face normal point along the way out. Where a fold flips a triangle in a pose, the smooth normal still points out while the triangle faces the skin. The renderer culls that back face, so the skin under it shows. With the smooth normal alone the first commit called that skin enclosed.
- **Skin truly through the cloth is still out.** It is outside the cloth over it, so its way out meets nothing, or another fold from outside. The synthetic test holds four cases: enclosed 12 mm from a sleeve's wall; 5 mm through the panel; through the panel under a fold that faces back; and through the panel under a fold whose corner normals face out but whose triangles face the skin (review fixes 2).
- **No limit changed.** The rule is still asset-quality §3's: 0 corners more than 1 mm outside the garment. Only what "outside the garment" is measured against changed.

> **For the lead.** This is a change to the gate's rule, which wave 4 chose not to make with a looser variant (a ray rule that ignored the closest point). Here the closest point still decides wherever the way out doesn't meet the cloth from inside. I lowered every record it brought down, with the reason written beside each, and review fixes 2 set back up those the smooth normal alone had brought down too far. If you reject it, revert `enclosed` (one `if` in `poke_through`) and raise the records back to the closest-point values in the "Measurements" table. The fit's own failures, item 2, stand either way.

**2. The fit (what is left).** With the corrected measure, in the stride (every third frame, 54 bodies, the test tee). The region and kind tables below were made with the smooth normal alone (the first commit); review fixes 2's counts follow them.
- **Where.** Against the rest positions of the failing corners:

  | Region | Corners | Bodies | Worst |
  |---|---|---|---|
  | The upper arm's inner side and underside | 742 | 48 | 15.4 mm |
  | The torso's side under the arm | 312 | 49 | 18.8 mm |
  | The front of the armpit fold | 27 | 26 | 5.8 mm |

- **What kind.** Of the remaining corners:
  - 807 are **open**: no skin across them within 6 cm along the normal, and not inside the body;
  - 274 are shut in the crease (skin facing back within 4 cm);
  - 187 are inside the other limb (the hanging arm sinks into the side; parity of body crossings).
- **Review fixes 2, the face-consistent count.** 2,395 body corners fail in the stride (776, 825 and 794 a third) on all 54 bodies. Their way out meets: a fold flipped towards the skin, 1,220; the cloth from outside, 631; nothing, 544. Almost all are at armpit height: 1.10 to 1.35 m above the floor and 10 to 20 cm from the middle, the same arm's inner side and torso side as above. The 1,220 under a flipped fold are the corners the first commit called enclosed. I did not redo the region and kind split for them; that script was a one-off outside the repository.
- **When.** In every third of the stride, worst where an arm is fully forward (Walk@41: feminine average, torso-side corner 3229, 18.7 mm) or fully back.
- **Why.** Horizontal sections at the passing frame (masculine heavyset, y = 1.18, 1.21, 1.24 m) show it:
  - the hanging arm's section overlaps the torso's;
  - the tee runs round both, with small folds of cloth inside the overlap.

  At Walk@41 (an arm forward) the section through the side of the chest shows the cloth between the sleeve and the side panel stretched into zig-zags. Triangles fold over, and the side panel is pulled 3 cm off the skin it covered. In the 4K captures that is the slit at the front or back of the armpit.
- **The binding's own target fails worse.** I placed each tee corner at its bound point of the posed skin plus its offset (what the posed fit follows). That target fails more open corners than the skinned tee: 197 meeting nothing, up to 27.4 mm, and 86 from outside, up to 16.5 mm. Corners 40 to 66 mm from where skinning puts them. Following the binding is not the fix.

**3. The bra through the tee** is the same place and cause. With the corrected measure, at Walk@18 it fails on 6 bodies, 15.2 mm (21 and 17.4 with the closest point; 4 with the smooth normal alone). In the stride it fails on 20 bodies in every third, up to 17.0 mm (283 corners, 171 under a flipped fold). It is not fixed.

## What I tried on the fit, measured, and dropped

Each row is a full gate run (54 bodies, the corrected measure with the smooth normal alone, as in the first commit), with each change alone. Review fixes 2's face condition only adds failing corners, so no row would pass under it either; the experiments' code isn't committed, so they were not re-run. "Walk thirds" are the three walk tests' worst values.

| Change | Walk thirds (bodies, worst) | Arms forward | Notes |
|---|---|---|---|
| **None (the first commit)** | 54 / 13.5, 48 / 16.5, 53 / 14.7 mm | 39 / 9.4 mm | Face-consistent (review fixes 2): 54 / 13.5, 54 / 16.5, 54 / 14.7; arms forward 44 / 9.4 |
| **Cover push** (new `posed::cover`, the gate's rule in the fit poses, into the leaning correctives, largest or mean push, 6 to 10 rounds) | 54 / 38 to 42 mm | 54 / 20 mm | Didn't converge in the fit's own poses (795 to 833 cases each round), and the walk got worse. It also opened holes in the walk (7 bodies, 32 mm). |
| **Binding split** (sleeve corners drop the trunk's joints, side-panel corners the arm's, by the bound point's limb; then the usual smoothing) | 54 / 17.5, 52 / 20.4, 49 / 15.5 | 27 / 8.6 | |
| Split, without the posed follow | 53 / 13.2, 49 / 15.2, 49 / 16.5 | 21 / 6.5 | Better with the arms forward, no better in the walk |
| **Fit poses at the walk's own corrective weights** (Back reaches 0.6 in the clip, the fit's walk poses 0.99; a passing pose added) | 54 / 15.3, 53 / 18.7, 51 / 15.5 | 47 / 8.2 | |
| **Weight smoothing** 6 rounds (from 2) | 54 / 15.3, 53 / 18.9, 52 / 19.3 | 40 / 9.1 | |
| **No projected weights** (bound point only) | 50 / 13.3, 50 / 18.0, 54 / 14.8 | 21 / 6.9 | |
| **Arms forward added to the fit poses** | 52 / 15.3, 54 / 18.8, 49 / 15.4 | 47 / 8.5 | |

**What this means.** Leaving out the cover push, every change lands at 49 to 54 bodies and 13.2 to 20.4 mm in the stride. The first commit itself was 48 to 54 bodies, up to 16.5 mm (54 in every third with the face condition). The failure is structural. The underarm was made with the arm at 45 degrees, and in linear skinning plus per-pose correctives it has no place to go when the gap closes. That leaves wave 4's other route: a garment authored with the arms nearer hanging, or a real deformer (cloth proxies or deformer maps, ADR-0035). Either is a contract change for the lead, not a fitting constant. The experiments' code is not committed; it is kept outside the repository.

## Tests (red first: `tee-walk-red-run.txt`)

**Red.** The new tests, on the unchanged fit and measure: 16 run, 10 passed, 6 failed.
- The synthetic measure test: enclosed skin counted out by 12 mm.
- Arms forward: 54 bodies, 32.0 mm.
- The three walk thirds: 54 bodies each, up to 32.6 mm; the bra 21, up to 18.8 mm; the boxer briefs 20 bodies, 4.1 mm.
- The held-pose test. Its 1.17 degrees was the new pose's own error (let down about the forward axis only), fixed in the pose and not in the test.

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-player --no-fail-fast` (final) | **250 tests run: 250 passed** (8 slow), 22 skipped. No ABORT lines |
| `cargo nextest run -p axcenity-player -E 'test(/fit_gate_tests::/) \| test(/wardrobe_tests::the_wardrobe_covers/) \| test(/held_poses/)'` | 27 passed (51 s; each walk third about 20 s alone) |
| `cargo clippy -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 114 pages, 0 errors (57 staleness warnings on other pages) |

## Measurements

**The realistic gate, the same fit, three measures.**
- **Wave-4 record:** what the wave-4 tests held.
- **Closest point only:** the old measure, re-run on this fit (`AXCENITY_FIT_GATE_CLOSEST_ONLY=1`).
- **Corrected:** `enclosed` with the smooth normal alone, the first commit.
- **Face-consistent:** `enclosed` with the face condition (`from_inside`), review fixes 2. The records hold these.

| Case | Wave-4 record | Closest point only | Corrected (smooth normal) | Face-consistent (the records) |
|---|---|---|---|---|
| Test tee, Walk@18 | 54, 30.2 mm | 54, 30.2 mm | 16, 9.7 mm | 38, 10.2 mm |
| Bra through tee, Walk@18 | 21, 17.4 mm | 21, 17.4 mm | 4, 15.2 mm | 6, 15.2 mm |
| Test tee, arms raised | 2, 5.1 mm | 2, 5.1 mm | none (record removed) | 1, 2.9 mm (record restored) |
| Bra through tee, arms raised | 2, 3.6 mm | 2, 3.6 mm | none (record removed) | none |
| Test tee, the stride (thirds) | (new) | 54 / 54 / 54, up to 32.6 mm | 54 / 48 / 53, 13.5 / 16.5 / 14.7 mm | 54 / 54 / 54, 13.5 / 16.5 / 14.7 mm |
| Bra through tee, the stride | (new) | 21 each, up to 18.8 mm | 17 / 9 / 14, 10.8 / 17.0 / 13.1 mm | 20 / 20 / 20, 10.8 / 17.0 / 13.1 mm |
| Boxer briefs, the stride | (new) | 20, 4.1 mm | none | none |
| Test tee, arms forward | (new) | 54, 31.5 mm | 39, 9.4 mm | 44, 9.4 mm |
| Bra through tee, arms forward | (new) | 21, 9.8 mm | 4, 3.6 mm | 15, 3.6 mm |
| Rest, T-pose, elbows and knees bent | 0 | 0 | 0 | 0 |

**The stylized gate** (records lowered by the corrected measure; nothing refitted):
- the cargo pants in the walk: 15 bodies, 27.2 mm → 7 bodies, 5.3 mm (4 bodies, 1.0 mm with the smooth normal alone; 4 of the 8 corners are under a flipped fold);
- the cargo pants at rest and in the T-pose: 7 bodies, as at wave 4 (the smooth normal alone made it 6);
- the tee in the walk: 16 → 8 bodies (24.7 mm unchanged);
- the sports top alone in the walk: 1 body, 5.3 mm → none (record removed);
- the sports top through the tee in the walk: 12 bodies, 2.5 mm → 2 bodies, 1.6 mm.

**The wardrobe** (the wave-4 drafts; the realistic crew tee).
- **Lowered by the corrected measure** (and unchanged by review fixes 2's face condition: every wardrobe record was measured again and no count or worst moved):
  - the bra through the tee in Idle: 15.8 → 5.3 mm;
  - the boxer briefs through the joggers at Walk@18: 8.9 → 5.2 mm;
  - the joggers with the knees bent: 39.3 → 31.1 mm. That 31.1 mm is the value the wave-4 merge had raised from, a coincidence of the measure and not a refit: the joggers' case in every pose is the back of the ankle, 31.0 mm, and the 39.3 mm corner the merge recorded with the knees bent is skin the joggers enclose.
- **New records:**

  | Case | First third | Second third | Last third | Arms forward |
  |---|---|---|---|---|
  | Crew tee | 54, 38.5 mm | 54, 35.0 mm | 54, 38.7 mm | 54, 40.7 mm |
  | Joggers | 53, 28.3 mm | 54, 30.6 mm | 54, 28.0 mm | 53, 31.0 mm |
  | Joggers through the tee | 54, 23.3 mm | 54, 17.2 mm | 54, 24.3 mm | 54, 19.1 mm |
  | Boxer briefs through the joggers | 33, 7.6 mm | 33, 5.5 mm | 33, 7.0 mm | 33, 4.3 mm |
  | Bra through the tee | 21, 5.5 mm | 21, 5.5 mm | 21, 5.5 mm | 21, 6.0 mm |
  | Briefs through the joggers | 20, 9.6 mm | 11, 1.8 mm | 18, 9.6 mm | 8, 1.3 mm |

**Why the crew tee's mesh can't pass the walk.** It fails at rest already: 51 bodies, 28.2 mm, round the waist where its hem's turn-up lies over the joggers' waistband, and the stride keeps all of that. By where the failing corners are at rest, in the stride (every third frame, 54 bodies):

| Region | Corners | Worst |
|---|---|---|
| Round the waist and hem | 5,966 | 35.8 mm |
| Chest, back and neck | 1,383 | 38.7 mm |
| Arm and armpit | 2,957 | 26.9 mm |

The arm-and-armpit share is the same fit failure as the test tee's, larger. A walking fit can't pass while the rest fit fails at the waist. It stays a Draft for its look (decision 13), and its mesh would need the waist fixed before the walk is even meaningful.

## Visual evidence (`tee-walk/`)

In-engine captures (release, studio light, 2560 × 1440) of the imported test tee over the underwear on eight cast bodies: slim, average, heavyset and muscular, both frames. Views: front, side and back.
- **Poses:** rest; three walk frames (`Walk@0.7`, the right arm forward and the left back; `Walk@0.183`, passing; `Walk@0.367`, the right arm back and the left forward); arms raised; arms forward.
- **Sheets:**
  - `test-tee-<pose>.jpg`: the upper body of each, from chin to hips;
  - `armpit-zoom-<pose>-side.jpg`: 1.5× crops (the walk's are named by the near arm, the one the side view shows, since review fixes 2) of the side view at the armpit (360 px of the 2560 px capture, drawn at 540 px).
- **Before and after are the same pictures.** The fit didn't change, and the tee's fitted file is byte-identical to wave 4's, so one set stands for both.

**How to find a body in a sheet.**
- **`armpit-zoom-*.jpg`** (1080 × 2160): tiles of 540 px, two to a row, in this order: masculine slim, average (row 1); masculine heavyset, muscular (row 2); feminine slim, average (row 3); feminine heavyset, muscular (row 4). The order was checked by matching each tile against the raw captures.
- **`test-tee-*.jpg`** (1176 × 3328): one body a row, 416 px a row, in this order: feminine average, heavyset, muscular, slim, then masculine average, heavyset, muscular, slim. Columns: front, side, back, 392 px each.
- **Sizes.** Measured in zoom pixels and turned into millimetres at about 0.27 mm a zoom pixel (25 px of the 2560 px capture is about 1 cm, and a zoom pixel is 2/3 of a capture pixel). Treat them as ±30%.
- **Coordinates** are (x, y) in the sheet named.

**What I saw** (corrected in review fixes 1; every defect is listed, with its body and place):
- **Walk, one arm back** (`armpit-zoom-walk-near-arm-back-side.jpg`, `Walk@0.7`; the near arm is the one swung back).
  - **Skin through a slit at the front of the armpit, about 9 mm wide**, on all four masculine bodies: slim (465, 343), average (1005, 343), heavyset (460, 882), muscular (1005, 882).
  - **The black bra through it**, a patch about 7 mm long, on feminine heavyset (465, 1940); a smaller black mark, about 2 mm, on feminine muscular (1020, 1935).
  - **Clean at the armpit:** feminine slim and average. Each has a lit, rounded fold of cloth above the armpit, about (437, 1365) and (977, 1365). It is cloth, not a hole.
- **Walk, the other arm forward** (`armpit-zoom-walk-near-arm-forward-side.jpg`, `Walk@0.367`; the near arm is the one swung forward).
  - **Cloth spikes at the back of the armpit, on all eight bodies.** On masculine slim a dark spike about 10 mm long points up out of the fold, at (417, 268) to (435, 300); masculine average has a 3 mm dark tip at (977, 288); on masculine heavyset (440, 832) and muscular (980, 835) the fold rises to a sharp peak without breaking.
  - **A black triangle at the same place on every feminine body**, which is the bra or the inside of the shirt: slim about 3 mm at (372, 1407); average about 2 mm at (914, 1410); heavyset a wedge about 7.5 mm at (395, 1957); muscular about 5 mm at (917, 1952).
  - **Light flakes of cloth**, 2 to 4 mm, beside them: masculine slim (395, 357); feminine slim (420, 1427), average (952, 1430), heavyset (385, 1970), muscular (955, 1972) and (955, 1900). They look like the shirt's turned-over inside catching the light.
  - **The rear view shows the back of this armpit more clearly** (`test-tee-walk-arm-back.jpg`, back column, the body's left armpit, added in review fixes 2). Every body has a dark region in the gap between arm and side whose edge is a sawtooth of straight-edged dark triangles, not a soft shadow: folded-over triangles. Sizes at about 1.4 mm a sheet pixel (±30%). Masculine: slim, a thin spike about 20 mm long from (820, 3145) to (829, 3156) beside a wedge about 22 × 30 mm at (817 to 833, 3154 to 3175); average, a patch of two teeth about 20 × 25 mm at (818 to 833, 1902 to 1921); heavyset, about 18 × 23 mm at (818 to 832, 2317 to 2333); muscular, about 20 × 23 mm at (818 to 833, 2733 to 2749). Feminine: the dark region is taller, about 3 × 9 cm, with teeth of 7 to 14 mm along its edge: average (838 to 862, 223 to 290), heavyset (840 to 862, 643 to 716), muscular (833 to 857, 1052 to 1132), slim (838 to 862, 1468 to 1545). Light flakes: feminine muscular two, a streak about 17 mm long at (842 to 850, 1067 to 1079) and a flake about 15 × 10 mm at (842 to 853, 1114 to 1122); feminine slim one, about 7 mm, at (855 to 860, 1534 to 1539); feminine average a speck about 5 mm at (858, 288). So the side zoom's "3 mm" on masculine average is only the tip that shows from the side; from behind it is about 2 cm.
  - **A cloth tab at the back of the shoulder seam on seven of the eight bodies**, about 4 mm on the masculine bodies and 5 mm on the feminine ones: masculine slim (505, 80), average (1050, 78), muscular (1055, 615, small); feminine slim (472, 1162), average (1012, 1162), heavyset (475, 1702), muscular (1012, 1702). Masculine heavyset has none.
- **Passing** (`armpit-zoom-walk-passing-side.jpg`).
  - **No skin and no bra.**
  - **A tiny dark spike, about 1 to 2 mm, at the back of the armpit**: feminine slim (392, 1390) and muscular (947, 1937).
  - **The shoulder-seam tab, about 5 mm, beside the back of the neck, on all four feminine bodies:** slim (342, 1165), average (885, 1165), heavyset (347, 1705), muscular (882, 1705).
- **Arms forward** (`armpit-zoom-arms-forward-side.jpg`; the gate records 44 bodies, 9.4 mm).
  - **Skin specks at the front armpit fold on three masculine bodies:** slim, a streak about 5 mm long at (332, 282); average, about 4 mm at (877, 287); heavyset, about 3 mm at (337, 829). Masculine muscular shows no skin, but the fold's edge is notched at about (860, 810).
  - **A light sliver of folded-over cloth**, about 7 mm, on masculine slim at (345, 200), and a smaller flake at (325, 255).
  - **The black bra on feminine heavyset**, a sliver about 16 mm long and 2 mm wide at (363, 1887). Feminine muscular has a thin black mark, about 4 mm, at (888, 1860), and a light flake at (950, 1880).
  - **A thin dark line**, about 8 mm, along the side of the breast where it meets the armpit fold, on feminine slim (357, 1350) and average (900, 1350). It may be the fold's shaded edge rather than the bra; at this size I can't tell.
  - **A small cloth tab at the sleeve's front hem** on feminine slim (310, 1305) and average (850, 1305).
- **Arms raised** (`test-tee-arms-raised.jpg`).
  - **No skin shows.** The face-consistent gate (review fixes 2) finds 4 corners on feminine heavy-soft at the neck's sides under the collar, up to 2.9 mm; nothing shows there at the sheet's size.
  - Under the raised arm the cloth is crumpled into long vertical folds on every body.
  - **On all four feminine bodies it folds over itself into large dark triangular flaps**, the "dark underarm flap" wave 4 reported. That is visible mesh folding, a failure under §1's "no tearing or mesh separation", even though the gate passes the pose (no skin is out).
- **Rest** (`test-tee-rest.jpg`).
  - **No skin through.**
  - **A cloth tab sticks up at the back of the shoulder seam in the side view of every feminine body**, about 5 mm: average (670, 139), heavyset (673, 555), muscular (672, 971), slim (670, 1387). The masculine bodies show only a small step there, about (670, 1800), (672, 2216), (672, 2632) and (670, 3048).
  - **Looking into the loose sleeve from the side, the black bra shows on the feminine bodies.** That is not clipping, but a licensed tee should be judged for it.
  - **The wave-4 defects remain:** the crew neck's wavy edge, and the ridge across the chest below it.
- **The gate and the pictures agree on the armpit.** The visible slits and specks are where the corrected gate's remaining open cases are: the front and back of the armpit as the arm swings, and the front fold with the arms forward.
- **What the gate doesn't count:** the shoulder-seam tab, the cloth spikes and flakes, and the arms-raised flaps. Each is cloth folding or sticking out, with no skin out, so poke-through can't see it.

## Open issues

1. **No tee passes the walking fit test** (the main goal). The test tee fails on all 54 bodies in every third, up to 16.5 mm, with a visible slit at the armpit; the crew tee on every body, up to 38.7 mm. The cause is the underarm bridge, made at 45 degrees, that can't fold into the crease when the arm hangs; tuning within the current fit doesn't fix it (above). It needs one of these, each a decision for the lead:
   - a garment authored with the arms nearer hanging, with the fit binding it in that pose;
   - a deformer for loose cloth (cloth proxies or deformer maps, ADR-0035);
   - a licensed base tee with a gusseted underarm, tested on this gate before anything else.
2. **The measure change needs the lead's ruling** (above). Until then, the records the corrected measure lowered are ceilings set by it, with the face condition since review fixes 2.
3. **Visible cloth defects the gate doesn't count**, because no skin comes out:
   - the arms-raised flaps under the arm on the four feminine bodies, and the crumpled cloth on every body;
   - the cloth tab at the back of the shoulder seam: at rest and in the walk, on the feminine bodies and three masculine ones;
   - the cloth spikes and light flakes at the back of the armpit in the walk.

   A check for cloth folding over itself, or sticking out from the surface, would be a new gate rule. That is for the lead.
4. **The gate counts some skin no one sees.** By the first commit's split, 187 of the walk's remaining corners are inside the other limb, and 274 are shut in the crease. I didn't exclude them: the task forbids new exclusions, and the visible slits are among the open ones.
5. **The walk is a running stride** (wave 4's note on the starter Walk). The walking test plays the starter Walk as shipped.

## At merge

- **Nothing to regenerate.**
- The gate library cache (`target/<profile>/fit-gate/`) rebuilds itself.
- `wardrobe_tests.rs`'s Walk@18 record list is now `WALK_18`, because `WALK` is the gate's pose name. Another branch that edits those records must follow the rename.
- **Other branches' items** (the licensed tee from intake): run the walking fit test (`gate_walk` and `wardrobe_gate_walk` take any description), and record nothing for a Production item.
- **Your decision:** the corrected measure (`enclosed`, with the face condition), and a route for item 1.
- The two walk zoom sheets were renamed in review fixes 2 (`armpit-zoom-walk-near-arm-back-side.jpg`, `armpit-zoom-walk-near-arm-forward-side.jpg`).

## Review fixes 1

The review found one blocking problem: the visual report left out defects the committed pictures show. That was right. I looked at every zoom sheet again, tile by tile, enlarged two and three times. To be sure which tile is which body, I matched each tile against the raw captures. The fixes are docs only. Apart from one stale comment, no code, record or picture changed.

**The blocking item, fixed.**
- **"What I saw"** (above) now lists every defect with its body, its sheet and its coordinates, and a size in millimetres. It also says how to find a body in each sheet.
- **What the first report missed:**
  - **Arms forward, skin specks** at the front armpit fold: masculine slim, average and heavyset, 3 to 5 mm. The first report called this pose "clean except feminine heavyset".
  - **Arms forward, other marks:** a light fold-over sliver on masculine slim, a thin black mark on feminine muscular, and a thin dark line on feminine slim and average that I can't call bra or shadow at this size. The feminine heavyset bra sliver is about 16 mm long, not a "sliver" as first written.
  - **Walk, the arm swung forward:** cloth spikes at the back of the armpit on all eight bodies (dark on masculine slim and average; sharp, unbroken peaks on masculine heavyset and muscular), not "most bodies". There are black triangles on all four feminine bodies, and light flakes too.
  - **The shoulder-seam tab:** at rest in the side view of all four feminine bodies; in the walk with the arm swung forward on seven of eight (the review counted six; I count masculine muscular's small one too); in the passing frame on all four feminine bodies. The first report had none of this; its "tab on two" was a different thing, at the armpit.
  - **Passing:** a tiny 1 to 2 mm dark spike at the back of the armpit on feminine slim and muscular. It is not "nothing visible".
  - **The zoom scale** was given as 2×. It is 1.5× (360 px of the capture drawn at 540).
- **Known-Limitations.** "It fits ... with the arms raised" now reads "passes the gate with the arms raised but folds into visible flaps under the arm on the feminine bodies there, no skin through". The entry now gives the arms-forward skin and bra with the gate's record (39 bodies, 9.4 mm), the walk's armpit spikes, and the shoulder tab, and it says the gate counts none of the cloth-only defects.
- **Open issue 3** now lists the cloth defects the gate can't see.

**The non-blocking items.**
- **The stale comment.** `character_capture.rs`, `HeldPose::ArmsForward`, no longer says the pose is made as a fit pose. `fit_poses` has none; it was in a dropped experiment.
- **The motion check.** The test tee still fails it. The fit is byte-identical, so wave 4's records stand: the armpit jolts 4.1 mm a frame in the walk and 11.7 mm in the run (`garment_checks::WALK_RECORDS`, `RUN_RECORDS`). This is another reason no tee passes decision 13.
- **The number mismatch.** "What this means" now says: leaving out the cover push, every change lands at 49 to 54 bodies and 13.2 to 20.4 mm. The 48 is the commit's own second third. Known-Limitations' "49 to 54, 13 to 20 mm" already matched that.
- **The Walk@18 record.** A re-run measures 16 bodies, worst 9.72 mm (`realistic-masculine-body-slight`). The record holds 0.0098, 0.08 mm above and inside `SLACK`. The docs' 9.7 mm is the same value rounded. I left the record as it is.
- **The bra seen through the open sleeve at rest** (feminine side views) is now in "What I saw" as a note for judging the licensed tee. It is not clipping.
- **The smooth normal in `enclosed`.** This is for the lead's ruling on the measure. The inside-or-outside test uses the hit's interpolated normal. At the armpit's zig-zag fold-over triangles, where the remaining failures are, a flipped triangle could be read as "from inside". Also requiring the geometric face normal to agree would make the test stricter. I haven't changed it: that would change the measure again, and it is the lead's call along with the measure itself.
- **The authored rest pose nearer hanging was not tried.** The five experiments all kept the garment made with the arm at 45 degrees. So "structural, needs a contract change" is shown for the binding and weight routes, not for that one. It should be its own task (open issue 1).

**Tests re-run after these fixes**

| Run | Result |
|---|---|
| `cargo nextest run -p axcenity-player --no-fail-fast` | **250 tests run: 250 passed** (18 slow), 22 skipped. No ABORT lines |
| `cargo nextest run -p axcenity-player -E 'test(=fit_gate_tests::garments_cover_the_cast_walking)' --no-capture` | passed; 16 bodies fail the test tee at Walk@18, worst 9.72 mm (against its record) |
| `cargo clippy -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 114 pages, 0 errors (57 staleness warnings on other pages) |

## Review fixes 2

The review found one blocking problem: `enclosed` decided "from inside" by the hit's smooth normal alone. That normal mixes the corners' skinned rest normals. In the armpit's fold, where every remaining failure is, triangles flip in the walk: the smooth normal still points out while the posed triangle faces the skin. The renderer culls that back face, so the skin under it shows. The first commit counted such skin as enclosed, and lowered or removed records on that basis. The review measured it (1,633 such verdicts in the six walk, arms-forward and arms-raised tests). It was right, and the fix is code and records, not docs.

**The fix (red first, `tee-walk-red-run.txt`).**
- **The test.** `skin_a_loose_garment_encloses_is_inside_and_skin_through_it_is_out` has a fourth case: skin 5 mm through the panel, under a fold whose corner normals face out (+Z) but whose triangles are wound to face the skin. Red: the old rule called it 20 mm inside (count 0, expected 1).
- **The code.** `from_inside(garment, hit, n)`: the hit's smooth normal **and** its posed triangle's face normal, `(b - a) × (c - a)` from the garment's posed positions, both point along the way out. `enclosed` uses it, and so does the debug print, which now says `from outside, flipped` where the smooth normal alone would have said inside.
- **The records**, each set to what this measure finds (all `check` groups are now printed as `measured:` lines, so every record was compared). Setting these back up is not loosening anything: it undoes reductions the flawed test made, and each comment says so.

  | Record | First commit (smooth normal) | Now (face-consistent) | Wave 4 |
  |---|---|---|---|
  | Test tee, Walk@18 | 16, 9.8 mm | **38, 10.3 mm** (measured 10.21) | 54, 30.2 mm |
  | Bra through the test tee, Walk@18 | 4, 15.2 mm | **6, 15.2 mm** | 21, 17.4 mm |
  | Test tee, the stride's thirds | 54 / 48 / 53 | **54 / 54 / 54** (worsts unchanged: 13.6, 16.6, 14.7 mm) | (new) |
  | Bra through the test tee, the thirds | 17 / 9 / 14 | **20 / 20 / 20** (worsts unchanged: 10.8, 17.1, 13.1 mm) | (new) |
  | Test tee, arms forward | 39, 9.4 mm | **44, 9.4 mm** | (new) |
  | Bra through the test tee, arms forward | 4, 3.6 mm | **15, 3.6 mm** | (new) |
  | Test tee, arms raised | removed | **restored: 1 body, 2.9 mm**: feminine heavy-soft, 4 corners at the neck's sides under the collar, 1.8 to 2.9 mm, whose way out meets the shoulder's cloth flipped towards them (5 to 11 mm out). Too small to see in `test-tee-arms-raised.jpg`. | 2, 5.2 mm |
  | Bra through the test tee, arms raised | removed | none (measured) | 2, 3.6 mm |
  | Stylized cargo pants, rest and T-pose | 6 bodies | **7 bodies**, 2.4 mm | 7 bodies |
  | Stylized cargo pants, Walk@18 | 4, 1.1 mm | **7, 5.3 mm** | 15, 27.2 mm |

  The test tee at rest, in the T-pose and with the elbows or knees bent stays at 0. Every other stylized record and **every wardrobe record measured the same** under the face condition, count and worst, so none of them changed (the wardrobe's comment says so). These are the review's numbers, reproduced.
- **What the face-consistent gate finds in the stride** (test tee, 54 bodies): 2,395 failing body corners. Their way out meets a fold flipped towards the skin (1,220), the cloth from outside (631) or nothing (544). Almost all are at armpit height, 1.10 to 1.35 m up and 10 to 20 cm from the middle. The bra: 283 corners, 171 under a flipped fold. The region and kind split in "The cause, measured" is the first commit's, made before the face condition; I didn't redo it, because that script was a one-off.
- **Docs.** The summary paragraph, "The cause, measured", "Measurements" (a face-consistent column), the stylized list, the experiments table's baseline, and open issues 1, 2 and 4 in this file. Also `asset-quality.md` §3 "measured by" (names the face condition; no limit changed), `testing.md`, the wiki Characters page, and Known-Limitations. Known-Limitations no longer says the tee passes the gate with the arms raised (it fails on 1 body, 2.9 mm); it gives the walk frame as 38 bodies, 10.2 mm, the stride as all 54 bodies, the arms forward as 44, and the wave-4 share as 94% (2,660 of 2,822).

**The non-blocking items.**
- **The rear view of the walk** (`test-tee-walk-arm-back.jpg`, back column) is now in "What I saw", with each body's place and size. It shows the back-of-armpit defect better than the side zoom: a sawtooth of dark, straight-edged triangles on every body, about 2 cm across on the masculine bodies (the side zoom's "3 mm" on masculine average is only the tip that shows from the side), teeth of 7 to 14 mm on a dark region about 3 × 9 cm on the feminine bodies, and light flakes on feminine muscular (two), slim and average. I cropped each tile at 3× and 6× to measure them.
- **The zoom file names.** Renamed by the near arm, the one the side view shows: `armpit-zoom-walk-near-arm-back-side.jpg` (was `-arm-forward-`, `Walk@0.7`) and `armpit-zoom-walk-near-arm-forward-side.jpg` (was `-arm-back-`, `Walk@0.367`). The `test-tee-walk-*.jpg` sheets keep their names; they are named by the right arm, as "Visual evidence" says.
- **The record check now checks counts** (red first: `the_gate_holds_garments_to_their_records` has a "fewer bodies" case, which passed before the fix). `check` fails a record whose garment and pose fail on fewer bodies than recorded: lower the record. Every record on this branch, gate, stylized and wardrobe, already holds its exact count, so nothing else changed. **For other branches:** a record whose count is above what their item measures will now fail with "lower the record".
- **The knees-bent joggers record** (39.3 → 31.1 mm, the value from before the wave-4 merge): a coincidence of the measure, not a refit. The joggers' case in every pose is the back of the ankle, 31.0 mm, and the merge's 39.3 mm corner was skin the joggers enclose (it stays enclosed under the face condition). Said beside the record and in "Measurements".
- **The authored rest pose nearer hanging** is still untried, as open issue 1 says. The experiments table was measured with the smooth normal alone; the face condition only adds failing corners, so none of those changes would pass either. They weren't re-run, because their code isn't committed.

**Tests run for review fixes 2**

| Run | Result |
|---|---|
| Red: `cargo nextest run -p axcenity-player -E 'test(/skin_a_loose_garment/)'` | 1 run, 0 passed: the flipped fold counted 20 mm inside |
| Red: `cargo nextest run -p axcenity-player -E 'test(/the_gate_holds_garments/)'` | 1 run, 0 passed: "fewer bodies" not caught |
| `cargo nextest run -p axcenity-player -E 'test(/fit_gate_tests::/) \| test(/wardrobe_tests::/)' --no-fail-fast` (records set) | **31 tests run: 31 passed** |
| `AXCENITY_FIT_GATE_DEBUG=1 cargo nextest run -p axcenity-player -E 'test(/fit_gate_tests::garments_cover/) \| test(/stylized/)' --no-capture` | 14 passed; the corner counts above |
| `cargo nextest run -p axcenity-avatar -p axcenity-player --no-fail-fast` | **415 tests run: 415 passed** (16 slow), 22 skipped. No ABORT lines |
| `cargo clippy -p axcenity-player --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `python3 -I scripts/check-wiki.py` | 114 pages, 0 errors (57 staleness warnings on other pages) |
