# CO6 review: eyes, lids, lashes and brows

What the review of the realistic head's upper face found, what was changed and what still falls short. The head itself is described in [README.md](README.md); the package is [assets/characters/realistic/](../../../../assets/characters/realistic/README.md). Code: `lids.py`, `lashes.py`, `eyes.py`, and in `face.py` the eye, brow, cheek and nose entries of `Face.expressions`, `Face.brow`, `Face.cheek` and `soften_lids`.

Every picture named here is a crop of a 3840 x 2160 capture from the engine's renderer (`capture_characters`, the straight-on `face-front` view) of the committed, baked build; masculine frame on the left, feminine on the right, a row per pose. They are in [engine/](engine/), and are the second round's: sections 1 to 7 are the first round as it was found and changed then, with what the second round changed noted in them; [Round 2](#round-2) is the second; [Still short](#still-short) is as things stand.

## How it was looked at

- The engine's captures at full size, in the studio light and the hard side light.
- The mesh, in numbers (millimetres, masculine frame). An eye is 24.5 mm from corner to corner; the lids' inner edges stand 11.8 mm apart at the middle and their outer edges (the lash lines) 10.4 mm; the eyeball's radius is 12.3 mm and its centre is 1.7 mm above the middle of the opening. The lid's inner edge is a ring of 20 vertices (8 on the upper lid), the outer edge one of 22, of which 7 run round the two corners outside the span of the inner edge.
- The skin's texture round the eye: from lash line to fold the upper lid is 10 to 12 texels of the 2048 map; the fold's own strip (loops 5 to 6) is half a millimetre at rest, one or two texels.

## 1. The squint

**Found.**

- *The ragged lid line was the lash line overshooting.* `Eye_Squint` asked the lower lid to go 3.4 times "the way to the closing line" (which lay a tenth of the opening above the lower rim). Each edge of a rim went that multiple of its own distance to the line, and near the corners the outer edge starts much further from it than the inner one: 3 mm from the inner corner the lash line rose 5.2 mm and at the outer corner 5.2 mm, against 2.6 mm at the middle. It stood over the inner edge in two peaks, with the lower lashes on it.
- *The step under the eye was the cheek.* The cheek's lift was cut to nothing on the lid's loops, so at the lid's border one loop rose 2.2 mm and the next 4.4 mm and 1.4 mm outward.
- *The lashes at the corners.* The lower lid's strip ran through the outer edge's corner vertices, sorted by position, which is not their order round the corner: zigzags at both ends.
- *Two shapes on one lid left the eyeball.* A shape moves a vertex along a straight line. `Eye_Squint` and `Cheek_Raise` each carried the lower lid along the chord of its own arc, and both chords point up and forward: with both at 1 the lid's edge was 16.3 mm in front of the eye's centre where the arc would have it at 13.6, 2.7 mm off the eyeball.

**Changed.**

- `lids.Eye.lid` takes where the closing line lies in the opening (`meet`). A squint's lids go 0.60 of the way to a line 0.56 of the way up the opening: the lower lid rises a third of the opening, the upper comes down a quarter, and both edges of each rim go the same share of the way to one line, so they keep their order everywhere.
- The lining and the inner edge go by the inner edge's turn, the outer edge and the skin by the outer edge's; the outer edge's corner vertices and the skin take their turn smoothed.
- `Eye_Squint` and `Cheek_Raise` have shares (0.60 and 0.40) of one straight path of the lower lid, to that line: together they bring it exactly there, on the eyeball.
- A lid that narrows the eye eases off toward the corners (over 2.5 mm; the lower lid over 5 mm from the inner corner, where it is held: at its full share 3 mm from that corner its edge stood as a ramp over the still skin beside the nose, which the engine lit as a pale patch, worst on the feminine frame).
- The cheek's lift runs on into the lower lid's skin loop by loop (`face.CHEEK_ON_LID`).
- Lashes grow between the corners only, the lower ones along the normal of the skin under the lid's edge, and every lash is kept 0.4 mm off the skin under every shape (`lashes.off_skin`).

**After round 1:** one unbroken lid line above and below at every strength; Laughing's opening 2.5 mm at the middle (3.75 mm before). The eye was still a straight-sided slit of bright white: round 2, below.

## 2. The closed eye

**Found** (4K, both lights, both frames, on the starting build).

- *The red speck and the dark blob were geometry.* Half way through a blink a straight path cuts under the eyeball's curve, so the closed lid is raised to clear it; it was raised vertex by vertex, and the inner edge, which travels furthest, ended 2.2 mm over the eyeball with the outer edge at 1.5 mm: the lining's side of the rim stood outside the skin. The outer edge and the first loop of skin ended on one spot (the lash line's strip had no area).
- *The white point was a hole.* Both corners are blunt (the rims are 2 to 4 mm apart within half a millimetre of them). The closing line followed the opening and dropped away from the inner corner faster than the two rims' vertices, which lie at different places along it, could follow alike: a three-cornered hole of 1.7 by 1.6 mm with the eyeball in it.
- *The pale patch is light, the streaks were texels.* The patch is the part of the closed lid the key light reaches past the nose's shadow; it is there in a build without skin maps. The streaks down it were the fold's strip, one or two texels drawn out to 3.6 mm (seven times), lying in the soft edge of the softened area where half the pores' detail was left.
- *The grey fringe was the card's shading.* A card shaded as a dielectric reflects 4% of the sky whatever its colour; on a card of albedo 0.02 that is most of its light. The texture's fringe (the lowest 16 to 30% of it, solid) made the band.

**Changed.**

- The closed lid is raised whole, by what its rim needs at each place along the eye (`lids.RAISED`), and the rim's outer edge ends outside its inner edge by the margin's width: the margin lies flat on the other lid's.
- The lids meet a fifth of the way up the opening (a tenth before): the upper lid travels less, so the closed lid needs less raising (its inner edge ends 1.3 mm off the eyeball at the middle; 2.2 mm before).
- The closing line runs straight to each corner from 2.2 mm inside it, and the rims end on one line (no seam).
- Each loop of the lid's skin trails the one before (`lids.SKIN`), so the skin is paid out evenly from lash line to fold.
- `face.soften_lids` softens out to two loops above the fold, takes the lid's colour from the skin round the eye only (the texels of the opening, which no face uses, are darker and were bleeding in), flattens the normal map there nearly all the way and makes the lid a little rougher.
- Lashes: a new texture (150 tapering lashes in tufts over a thin dark edge, 512 x 128), a rough metal's shading (it reflects only its own colour), and an upper lid's lashes straighten as it comes down (with their full curl a closed lid's lashes pointed their outer halves at the viewer and the fringe ended in a cut line).
- The occlusion film thins out toward where it stops short of the corners (`eyes.shell`).

**Now** ([engine/eyes-closed.jpg](engine/eyes-closed.jpg), and at twice the size [engine/eyes-closed-close.jpg](engine/eyes-closed-close.jpg)): at `Blink` 1 and `Eye_Closed` 1, in both lights on both frames, no speck, no blob and no white point; the lash line reads as dark lashes with separate tips; the lit part of the lid is smooth. The closed lid's inner edge stands 1.3 mm off the eyeball at the middle and its outer edge 3.4 mm. `face_checks.py`: no ray bare on either frame.

## 3. The brows' cards

**Found.** A card was 3 vertices across and 15 along, each on the nearest point of the skin and 0.5 mm over it, and under a shape all three across moved as the skin under the middle one did. The brow's shapes moved the skin under the card's lower edge 0.70 of what they moved the middle (3.2 mm less at `Brow_Raise`), so the card slid over a curved brow ridge: it cut into the skin on one side and stood off it on the other (the ragged top at `Brow_Lower` 0.15). Between its vertices the flat card cut under the round ridge at rest (the light hole). The texture ended square at the tail.

**Changed.**

- The brow's skin moves whole within 6.5 mm of the brow's line (`face.Face.brow`), easing off beyond; the fold over the upper lid takes less of it loop by loop.
- A card is 5 by 23 vertices; each vertex moves as its own point of the skin does, and no face of the card comes nearer the skin than 0.3 mm at rest or under any shape (`lashes.over_skin`).
- The texture thins to a point at the tail; the card is shaded as the lashes are (a raised brow, turned to the sky, went grey).

**Now** ([engine/brows-controls.jpg](engine/brows-controls.jpg), [engine/brows-controls-hard.jpg](engine/brows-controls-hard.jpg)): the cards are whole at rest, at `Brow_Lower` 0.15 and 1, `Brow_Raise` 1 (both and one side), `Brow_Inward` 1 and `Brow_Raise` with `Brow_Inward`, and in every expression of [engine/eyes-expressions.jpg](engine/eyes-expressions.jpg); no skin shows through. [engine/brows-arch.jpg](engine/brows-arch.jpg): the identity slider `brow_arch` at -1 and +1 flattens and peaks the brow without a hook. (The brow's identity shapes are made on the same weights, so they moved a little: `Face_Brow_Arch` 6.3 mm at its most, 6.1 before.)

## 4. Eyes wide

**Found.** The marks above the lids were the upper lashes: they turned all the way with the lid as it slid up under the fold and lay flat in the fold's skin. The lid's edge rose 3 mm, which left it on the iris.

**Changed.** An upper lid drawn up gives its lashes 0.3 of its turn, and they are kept off the skin. `Eye_Wide` raises the upper lid's edge 4.2 mm at the middle.

**Now** ([engine/eyes-wide.jpg](engine/eyes-wide.jpg)): white over the iris at `Eye_Wide` 1 and in Surprised on both frames, the lashes fanned up, nothing left on the skin above the lid.

## 5. The brow controls as muscles

| Control | Was | Is |
|---|---|---|
| `Brow_Lower` | The brow's skin 7.8 mm down at the inner end, 3.5 mm at the tail, 3.6 mm inward; the upper lid 14% of the way shut. | The same moves on the new weights, the skin between the brows 1.6 mm forward, the fold coming down on the lid and the lid 22% of the way shut (the opening 7.9 mm from 9.5; it was 8.35). |
| `Brow_Raise` | 10.5 mm up (the inner end 7.9 mm), the lid 10% wider. | The same, the fold drawn up with it, the lid 12% wider (the opening 10.5 mm). One side alone leaves the other brow where it was. |
| `Brow_Inward` | The inner end 8.2 mm up and 3.4 mm inward, the tail 2.6 mm down. | The same moves (the control is the inner brow raise: "draws the eyebrow's inner end in and up, as in worry", and ARKit's `browInnerUp` plays it), on the new weights, with the skin between the brows 1.5 mm forward. No vertex is drawn more than 0.4 of its way to the middle of the face. |

(Round 1 made `Brow_Inward` a level drawing-together, on a wrong reading of the control; it was put back before round 2, and Sad, Worried, Fearful and Crying have their raised inner ends again: [engine/brows-inner-raise.jpg](engine/brows-inner-raise.jpg), [engine/brows-inner-raise-hard.jpg](engine/brows-inner-raise-hard.jpg).)

What the pictures show ([engine/brows-controls.jpg](engine/brows-controls.jpg), [engine/brows-controls-hard.jpg](engine/brows-controls-hard.jpg)): `Brow_Lower` brings the inner ends down and toward each other and narrows the eye from above, with a small swelling between the brows; there is no furrow. `Brow_Raise` lifts the brow and shows more lid. `Brow_Inward` tilts the brow about its middle, inner end up, with a swelling between the inner ends. The mesh's edges there are 3 to 4 mm and the skin has no wrinkle maps, so none of them draws a line in the skin.

## 6. One eye alone

`Blink` now has 0.22 of the cheek's lift and 0.14 of the lowered brow's skin on its side (`Eye_Closed`, an eye at rest, has the lids alone). [engine/eyes-wink.jpg](engine/eyes-wink.jpg): the eye shuts, its cheek and brow move a little (1.6 mm and 1.1 mm at their most), the other eye is open and still.

## 7. The upper face in the expressions

[engine/eyes-expressions.jpg](engine/eyes-expressions.jpg) (ten at full strength, both frames, tiles at half size), [engine/brows-inner-raise.jpg](engine/brows-inner-raise.jpg) (Sad, Worried, Fearful, Crying, Frustrated at 0.8 of full size) and [engine/eyes-faces.jpg](engine/eyes-faces.jpg) (whole faces at a third: rest, blinks, Happy, Laughing, Surprised, Angry, Sad).

| Expression | From the nose up |
|---|---|
| Angry | Reads (brows down and in, eyes narrowed from above and below). |
| Surprised | Reads (brows up, white over the iris). |
| Fearful | Reads: wide eyes under brows raised with their inner ends highest. The inner ends go very high (0.7 of `Brow_Raise` and of `Brow_Inward` together lift them 11 mm). |
| Sad, Worried | Read: the brows slope up to their inner ends, Sad's over lowered lids, Worried's over slightly widened eyes. The slope is one straight line from tail to inner end. |
| Crying | Brows as Sad's at full strength, eyes all but shut. |
| Confused | Reads: one brow up, the other down with its eye narrowed. |
| Disgusted | Lowered brows and narrowed eyes; from the nose up it is a milder Angry (the nose and lip carry it). |
| Determined, Frustrated | Lowered brows, slightly narrowed eyes; both a mild Angry. |
| Sleepy | Heavy lids at 0.5; at 1 the eyes are 3 mm slits, shaded under the lid. |
| Happy, Laughing | The eyes narrow from below into arches; Laughing's are dark slits 2 mm tall (round 2). |

Recipes are not edited here. Numbers that would bring the brows nearer the reference faces, for whoever owns the recipes: Sad `Brow_Inward` 0.5 (0.7), Worried 0.6 (0.8), Fearful `Brow_Inward` 0.4 (0.7) with `Brow_Raise` left at 0.7, Crying 0.8 (1.0); Laughing, see round 2.

## Round 2

Found from the second review's captures and by taking things out of the build one at a time (a build without skin maps, without the lashes, without the brows, without all three parts, and with the film at one strength all over: none of those is committed).

### The laughing and squinting eye

**Found.**

- *The shape.* The squint's closing line followed the opening, which left it 1.3 mm higher at the middle than at the corners, and the upper lid came only 0.60 of the way down to it: a slit with a nearly straight floor and roof.
- *The white.* Three things kept it bright. The film's darkness was set by shares of the opening, so as the lids came together it shrank with them and the middle of the slit stayed clear. The film did not darken at all: the renderer's screen-space occlusion darkens the eyeball in its socket to about 0.58, and it is worked out on the nearest surface drawn, and a film across the opening, flush with the lids' edges, has none. Measured on one pixel of the white at rest (sRGB): no film 72, the round 1 film about 92, a film of alpha 0.2 all over 83, of 0.4 72, of 0.78 42. And the tear line ran along both lids a sixth of the opening wide (1.9 mm), white.
- On the feminine frame the new film first lay partly inside the eyeball (a pale patch beside the iris): its height over the eye was worked out on the masculine eyeball for the frame's shape too.

**Changed.**

- The squint's line is arched, 1.2 mm higher at the middle (`face.SQUINT_ARCH`), and the upper lid comes 0.80 of the way down onto it (`SQUINT_UPPER`). `Eye_Squint` and `Cheek_Raise` still share one straight path of the lower lid (0.60 and 0.40).
- The film is one sheet from corner to corner (`eyes.FILM_ROWS`): its rows keep their distance from the upper lid's edge (and two from the lower's) and stop at the other lid, so a narrowed eye has only the dark rows left in it, and the corners are dark at rest. It is black (a rough metal: as a dielectric it reflected 4% of the sky) and nowhere clearer than 0.42, the shade the renderer's occlusion gave the eyeball; under the upper lid it is 0.88 for 1.5 mm and falls to 0.42 by 5 mm. Under the frame's shape and the identity's it goes with the lids.
- The tear line is the lower lid's, half a millimetre wide.

**Now** ([engine/eyes-rest.jpg](engine/eyes-rest.jpg), [engine/eyes-squint.jpg](engine/eyes-squint.jpg), [engine/eyes-squint-hard.jpg](engine/eyes-squint-hard.jpg), and at twice the size [engine/eyes-laugh-close.jpg](engine/eyes-laugh-close.jpg)): at rest the whites are darker under the upper lid and into both corners, in both lights, the same on both frames. Laughing is a dark arch under the lashes, the white in it dark, on both frames in both lights; with both controls at 1 the eye is a dark line.

| `Eye_Squint`, `Cheek_Raise` | Opening at the middle between the lids' inner edges (mm) | Between the lash lines |
|---|---|---|
| rest | 11.4 | 10.2 |
| 1, 0 | 3.8 | 3.5 |
| 0, 1 | 8.3 | 7.2 |
| 0.85, 0.9 (Laughing) | 2.2 | 1.9 |
| 1, 1 | 0.7 | 0.6 |
| 0.85, 0.9 with `Eye_Closed` 0.1 | 1.0 | 0.8 |
| 0.85, 0.9 with `Eye_Closed` 0.2 | -0.1 (the lids overlap) | -0.25 |
| 1, 1 with `Eye_Closed` 0.1 | -0.4 | -0.5 |

`Eye_Closed` on top does not close a laugh cleanly: at 0.2 the two lids' edges pass each other by a quarter of a millimetre and show as a dashed line (bottom rows of the squint sheets). The clean ways to a nearly shut laugh are the two controls themselves: 1 and 1 leaves 0.7 mm, 0.95 and 0.95 about 1.2 mm.

**What a blink has left** (for the engine's rule that holds a blink back): with `Eye_Squint` at 1 a blink of 0.31 closes the eye (the control has used 0.69 of it), `Cheek_Raise` at 1 leaves 0.70 (0.30 used), `Brow_Lower` at 1 leaves 0.78 (0.22 used); both of the first two at 1 leave 0.05.

### `Brow_Lower` and Angry: the streaks on the upper lid

**Found.** They are in a build without skin maps, so not texels. The brow's shapes put the skin back on its resting surface from the seventh loop round the eye, which is the brow bone's overhang: lowered and slid on that surface it went over the edge of the bone into the hollow above the lid. On the feminine frame, at the outer half of the eye, the seventh loop's height over the eyeball fell from 5.0 to 2.6 mm; the fold was pressed flat, its three loops within 2 mm of each other, and their normals turned by 20 degrees and more (one from facing down to facing up): thin strips turned to the light. On the masculine frame the fold's loops also passed each other (12 pairs of crossing triangles round one eye, 2 at rest).

**Changed.** The brow's shapes leave the fold and the overhang off the resting surface (`face.BROW_SLIDES_FROM`: they slide from the eighth loop, fully from the eleventh) and hold the fold's two loops nearly alike (`BROW_ON_LID`).

**Now** ([engine/brows-controls.jpg](engine/brows-controls.jpg), [engine/brows-controls-hard.jpg](engine/brows-controls-hard.jpg), Angry in [engine/eyes-expressions.jpg](engine/eyes-expressions.jpg)): the lowered lid is one smooth band; the fold's normals turn 6 degrees at most at the place measured; crossing pairs under `Brow_Lower` on the masculine frame 0 and 1 (10 and 7). Under the hard light two thin bright lines still run along the masculine lid (its edge and the fold's).

### The lit blotch on a closed lid

**Found.** It is the same with the lashes left out, with the brows left out, and with lashes, brows and eye shell all left out: no card casts it. (A blended material is not a shadow caster in the renderer at all: `Blend` is drawn see-through, and only solid draws cast.) The shadow round it is the head's own, the brow ridge's and the nose's, and the blotch is the one part of the closed lid's dome turned to the lamp. Nothing was changed. What would lessen it: a closed lid that lies closer to the eyeball than a straight path lets it (an in-between shape in the engine; its outer edge stands 3.4 mm off the eyeball now), or a softer shadow edge in the renderer.

### Sleepy and the half blink: the dark shapes on the whites

**Found.** With the lashes left out they are gone; with the film left in alone there was only its own band. They were the upper lashes' card: on the straight path between curled up at rest and turned down on a closed lid, a half-shut eye's lashes point at the viewer, and a card seen nearly on edge, splayed toward its ends, is dark straight-edged pieces across the top of the opening. The solid root of the lash texture made them black.

**Changed.** The lashes' root line is thinner (5 to 10% of the lash) and there are fewer lashes (120); the upper lashes leave the lid a little higher and a closing lid turns them 1.3 times as far as itself; and the film now shades the half-shut eye from the lid down.

**Now** ([engine/eyes-closed.jpg](engine/eyes-closed.jpg), [engine/eyes-closed-close.jpg](engine/eyes-closed-close.jpg)): the half-shut eye is shaded from the upper lid down with a thin dark edge of lashes. Paler three-cornered areas remain low on the whites beside the iris (the film's rows run parallel to the upper lid's straight edges).

### `Eye_Wide`, feminine: the streaks above the lid

**Found.** As the upper lid went up, the lower of the fold's two loops (which took 0.55 of the turn) passed the upper one (0.28): the crease turned inside out and its strip faced the light. On the masculine frame they crossed (8 pairs round one eye).

**Changed.** An upper lid drawn up gathers its skin under the fold by weights of its own (`lids.SKIN_UP`): the fold's two loops go up together.

**Now** ([engine/eyes-wide.jpg](engine/eyes-wide.jpg)): crossing pairs under `Eye_Wide` on the masculine frame 2 and 2 (6 and 7). The lid's margin shows as a thin pale rim over the white. On the feminine frame a short faint streak is still there above the key-side lid, in `Eye_Wide` and in Surprised: less than it was, not gone.

### The lids' outline

Not started. It is 8 vertices along the upper lid's edge and 12 along the lower; a narrowed eye's arch is three or four straight pieces for that reason.

## Checks

On the committed build:

- `face_checks.py --check`: exit 0, 0 failures. `Blink` and `Eye_Closed`: no ray bare on either frame.
- Reported, not held: one lid vertex stands in the clear cornea under `Eye_Squint` and under `Cheek_Raise` (to 0.29 mm), two in Happy and Disgusted (to 0.26 mm): each control alone is part of the way along the straight path the two share, which cuts under the cornea's curve. None before.
- `cargo nextest run -p axcenity-anim -p axcenity-avatar --no-fail-fast`: 223 pass.

## Still short

1. **The laugh's arch is made of straight pieces** (the lids' vertices), its inner end stays a little open, and on the feminine frame a faint pale patch remains on the skin under the inner corner (round 1's: a shape's normals are stored as changes from the masculine frame's).
2. **No creases.** A laughing eye has no lines fanning from its outer corner, lowered brows no furrow between them and raised brows no lines across the forehead: geometry at this density cannot fold that finely and the skin has no wrinkle maps.
3. **The eye is shaded by a film, not by the renderer.** It is right for one head-on light; it does not know where the light is, and where it is drawn the renderer's own occlusion is lost (the film stands in for it). The eye's shading is CO7's.
4. **A half-shut eye** has paler three-cornered areas low on its whites.
5. **The closed lid's lit blotch** under a side light (above): not a card's shadow; not changed.
6. **`Brow_Lower` under the hard light**: two thin bright lines along the masculine lid.
7. **`Brow_Inward`** tilts the brow as one straight line and leaves a pale swelling between the inner ends, most in Fearful.
8. **`Eye_Wide`, feminine**: a short faint streak above the key-side lid remains.
9. **The lids' outline is angular** at 4K.
10. **Lashes and brows are cards.** Close up a brow is a dark mass with hairs at its edges; the lashes of the two lids do not interleave; a closed eye's lashes are longer than round 1's.
11. Under the hard light a patch of the closed feminine lid is stippled where it is lit (the renderer's shadow edge at a grazing light).
