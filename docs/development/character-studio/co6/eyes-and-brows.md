# CO6 review: eyes, lids, lashes and brows

What the review of the realistic head's upper face found, what was changed and what still falls short. The head itself is described in [README.md](README.md); the package is [assets/characters/realistic/](../../../../assets/characters/realistic/README.md). Code: `rims.py`, `lids.py`, `lashes.py`, `eyes.py`, and in `face.py` the eye, brow, cheek and nose entries of `Face.expressions`, `Face.brow`, `Face.cheek` and `soften_lids`.

Every picture named here is a crop of a 3840 x 2160 capture from the engine's renderer (`capture_characters`, the straight-on `face-front` view) of the committed, baked build; masculine frame on the left, feminine on the right, a row per pose. They are in [engine/](engine/), and are the third round's (the mesh round the lids changed in it, so every picture was made again): sections 1 to 7 are the first round as it was found and changed then, with what the second round changed noted in them; [Round 2](#round-2) is the second and [Round 3](#round-3) the third; [Still short](#still-short) is as things stand.

## How it was looked at

- The engine's captures at full size, in the studio light and the hard side light.
- The mesh, in numbers (millimetres, masculine frame). An eye is 24.5 mm from corner to corner; the lids' inner edges stand 11.8 mm apart at the middle and their outer edges (the lash lines) 10.4 mm; the eyeball's radius is 12.3 mm and its centre is 1.7 mm above the middle of the opening. The lid's inner edge was a ring of 20 vertices (8 on the upper lid), the outer edge one of 22, of which 7 run round the two corners outside the span of the inner edge; since round 3 each has twice as many (40 and 42).
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

[engine/eyes-expressions.jpg](engine/eyes-expressions.jpg) (twelve at full strength, both frames, tiles at half size), [engine/brows-inner-raise.jpg](engine/brows-inner-raise.jpg) (Sad, Worried, Fearful, Crying, Frustrated at 0.8 of full size) and [engine/eyes-faces.jpg](engine/eyes-faces.jpg) (whole faces at a third: rest, half and whole blinks, Happy, Laughing, Surprised, Angry, Sad).

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

Not started in round 2. It was 8 vertices along the upper lid's edge and 12 along the lower; a narrowed eye's arch was three or four straight pieces for that reason: [round 3](#round-3).

## Round 3

Found from the third review's captures, from measures on the built file (the skin's height over its resting surface, the film's height over the eyeball's mesh, pairs of crossing triangles, the normals the engine adds up against the posed mesh's own) and by taking parts out of the build one at a time (the tear line, the lashes, the whole eye shell: none of those builds is committed).

### The lids' outline: twice the vertices

**Found.** At 4K the eye's opening was a polygon: 8 vertices along the upper lid's edge, 12 along the lower. What the mesh and the build had to say about cutting it finer:

- *The strips are clean far enough.* Past the lid's inner edge the loops counted by steps from the ring are not closed edge loops (a vertex of five edges sits at each corner of the eye), but the strips of quads that run out across them are well defined: from each of the ring's 20 edges, quad by quad through the edge across. All 20 run clean out to the fifth loop and 18 of them to the eleventh; the two under the inner corner turn along the cheek after the fifth. Inward, the lining is ten loops of 20 vertices closed by a cap of 5 by 5 quads.
- *What is kept by the bundle's vertex index.* Both frames' vendored bodies, the two sculpts (677,442 displacements each, stored against the bundle's mesh subdivided three times), the 16 vendored body shapes, and the mirror pairing the mouth is made even by. The eyeball's fit is found from the socket's vertices and from which skin vertices lie inside it.
- *Midpoints change nothing.* A new vertex at the middle of its edge leaves the polygon the polygon it was.

**Changed** (`rims.py`, called from `build.py` after the texture coordinates are laid out).

- Every quad of those strips is cut in two along the strip: over the lining back to its fifth loop and over the skin out to the seventh (the two strips under the inner corner to the fifth), so each of those loops has twice the vertices: 256 new vertices an eye. Where a strip stops, the quad beyond it takes the one new vertex on its edge as a fan of three triangles: behind the eyeball, under the brow and on the cheek, none on the lid.
- A new vertex is put on the cubic through the four vertices of its loop round it (spaced by the centripetal measure, which does not overshoot where the loop turns the corner of the eye). That is a fixed mix of four old vertices, so one rule places the new vertices on both frames, in the bundle's pose, in the stood pose and in the vendored shapes' offsets: the bundle's 10,582 vertices keep their indices and their places, and the new ones follow them. The mirror pairing of a new vertex is the vertex on the mirror image of its edge, exactly.
- The eyes are fitted and the texture coordinates laid out before the cut, on the bundle's topology: the eyeballs, every joint and the texture layout are the same as before to the last digit of `Realistic.build.json`. The sculpts are subdivided from the bundle's topology (`skin.bake`, `cage`) and baked onto the finer body.
- The lid shapes need nothing new: they find their loops by steps from the ring, and a new vertex is as many steps out as the two it lies between. Two things were made finer with them: the line the lids close on is worked out at 32 places (12), and a closed lid's raise takes the most its two neighbours either side need (one).
- The lashes get a row at every vertex of the lash line (twice the rows), the film a column at every vertex of the lids' edges.

| | Before | After |
|---|---|---|
| `Body` | 10,582 vertices, 21,160 triangles | 11,094 vertices, 22,184 triangles |
| `Body_LOD1`, `Body_LOD2` | 10,500 and 5,000 triangles | the same (they are made to a budget) |
| `EyeShell`, `Lashes` | 1,788 and 88 triangles | 2,612 and 188 |
| `RealisticParts.glb` | 31.04 MB | 31.61 MB |

**Now** ([engine/eyes-rest.jpg](engine/eyes-rest.jpg), [engine/eyes-wide.jpg](engine/eyes-wide.jpg), [engine/eyes-closed.jpg](engine/eyes-closed.jpg), and at twice the size [engine/eyes-outline-close.jpg](engine/eyes-outline-close.jpg)): at rest, wide and half shut the lids' edges are curves on both frames in both lights, at full size and at twice the size. The inner corner is still the bundle's: the lower lid leaves it in a short straight run before it curves.

### The narrowed eye: one share of the opening from corner to corner

**Found.** With the outline smooth, a squint and a laugh were still a slot with a tall end at each corner. Round 1 eased both lids' share to nothing over the last 2.5 to 5 mm before a corner, so neither lid moved there; the opening is 3 to 4 mm tall within a millimetre of a corner at rest, and a narrowed eye was 2 mm tall at the middle and taller than that at both ends, the white of the eye bright in them. The same easing made a blink on top of a squint close the middle before the ends.

**Changed.** The lower lid is still held toward the corners (over 6 mm from the inner corner and 4.5 mm from the outer; 5 and 2.5 before, which stood the lid's edge up as a hook at the outer corner). What it is held back by, the upper lid now comes down instead (`lids.Eye._turns`), in the squint and in the raised cheek alike, and the upper lid's own share is not eased. Each control that narrows the eye therefore closes one share of the opening from corner to corner: a narrowed eye has the outline of the open one, pressed flat.

**Now** ([engine/eyes-squint.jpg](engine/eyes-squint.jpg), [engine/eyes-squint-hard.jpg](engine/eyes-squint-hard.jpg), [engine/eyes-laugh-close.jpg](engine/eyes-laugh-close.jpg)): Laughing and the squint with the raised cheek are one dark arch from corner to corner, a curve at full size and at twice the size, on both frames in both lights, its ends coming down to the corners. The ends are no taller than the middle. A thin grey wedge of the white is left at the inner end of the squint and, thinner, of the laugh: the white beside the iris, where the film is at its lightest. Under the hard light a small lit point of skin stands at the outer corner of the lit eye. Reported by `face_checks.py`, not held: 13 pairs of triangles cross under `Eye_Squint` and 27 in Laughing (none before), all of them the two lids' lining behind the outer corner, 7.5 to 8.6 mm behind the opening, where a blink's lining crosses too (172 pairs).

**What a blink has left** (`Realistic.build.json`, `face.blink_shares`, which the build now measures: how much of a blink a control at 1 uses up, at the middle of the eye and the most anywhere between 3 mm inside each corner):

| Control | Round 2, at the middle | Round 2, the most | Now, at the middle | Now, the most |
|---|---|---|---|---|
| `Eye_Squint` | 0.670 | 0.692 | 0.673 | 0.705 |
| `Cheek_Raise` | 0.270 | 0.298 | 0.282 | 0.304 |
| `Brow_Lower` | 0.171 | 0.232 | 0.184 | 0.255 |

The most is 3 mm inside the outer corner in every case (0.88 of the way along the eye). The engine holds a blink back by 0.69, 0.30 and 0.22; by this measure it would need 0.71, 0.31 and 0.26 for the lids never to touch there (at 0.69, 0.30 and 0.22 they pass each other by a tenth or two of a millimetre at that one place, with a control at 1 and a full blink asked for). The engine's numbers are not changed here.

### Brows raised and drawn in together

**Found.** `Brow_Raise` with `Brow_Inward`, both at 1, left a lit ridge between the brows' inner ends, with the hair's inner ends splayed on it. Measured on the built file, the skin within 7 mm of the middle of the face stood up to 3.5 mm over its resting surface on the masculine frame and 2.6 mm on the feminine (1.1 and 1.9 mm with one control alone). It was neither the bunch alone nor the cards. Four shapes carry that skin up the forehead (both sides' raise and inward), each along the straight line to where it alone ends on the skull, and the forehead is curved: added together the four lines end off it. And the skin between the brows was held off the skull on purpose, for its bunch and because it is within a dozen loops of both eyes (like the folds over the lids).

**Changed** (`face.Face.expressions`).

- The skin within 7 mm of the middle of the face slides on the skull all the way (none of it past 13 mm), and its bunch is added afterwards, 0.8 mm (1.5).
- All four shapes together end on the skull: what their sum is off by is taken out of each by its share of the move. One alone then lies under the skull by that share.

**Now** ([engine/brows-controls.jpg](engine/brows-controls.jpg), [engine/brows-controls-hard.jpg](engine/brows-controls-hard.jpg), Fearful in [engine/brows-inner-raise.jpg](engine/brows-inner-raise.jpg)): no ridge and no lump between the inner ends on either frame, at full size and at twice the size; the hair's inner ends lie flat. What shows there now is the skin's own grain: it has been carried 13 to 16 mm up the root of the nose, and its pores are drawn out into short upright streaks where the light catches them, most with both controls at 1. The same measure:

| | Before, masculine | Before, feminine | Now, masculine | Now, feminine |
|---|---|---|---|---|
| `Brow_Raise` 1 with `Brow_Inward` 1 | -1.0 to +3.5 mm | -0.8 to +2.6 | 0.0 to +0.8 | -1.2 to +1.9 |
| both at 0.7 (Fearful) | -0.9 to +2.1 | -0.6 to +1.5 | -0.3 to +0.3 | -1.0 to +1.2 |
| `Brow_Raise` 1 | -1.0 to +1.1 | -0.6 to +0.6 | -0.6 to 0.0 | -1.1 to +0.8 |
| `Brow_Inward` 1 | -0.7 to +1.9 | -0.4 to +1.6 | -0.3 to +0.5 | -0.7 to +0.8 |

What is left on the feminine frame is the two frames' foreheads being different skulls under one set of shapes (the shapes are made on the masculine one): it is spread over the forehead, not a ridge. `Brow_Lower` is as it was.

### Looked at and not changed

- **`Eye_Wide`, feminine: the streak above the key-side lid.** It is the feminine lid's own skin between the lash line and the fold, which shows at rest too (the masculine frame's is under its fold): drawn up, it is a narrower band turned further up to the key light. With twice the vertices it is one even band, not a broken streak. Carrying the lid's skin up more nearly whole (the three loops above the lash line at 0.92, 0.75 and 0.50 of the lid's turn, for 0.85, 0.55 and 0.36) left the band as it was and doubled the crossing pairs under `Eye_Wide` on the masculine frame (8 to 16): withdrawn. Hiding it means tucking the lid's skin under the fold on one frame only, with one shape for both.
- **Pale shards low on the white beside the iris with the eye wide** (both frames; they were there before this round). They go when the eye shell is left out and stay when the tear line is, so they are the film's. The film stands 0.19 mm or more over the eyeball's and the cornea's mesh everywhere with the eye at rest and wide (measured by rays from the eye's centre at every vertex, edge middle and face middle of the film), so it is not under either. Two more rows in the film there made a new pale patch and were withdrawn. Not found.
- **The pale patch on the skin under the feminine inner corner in a laugh** (round 1's). It stays with the lashes, the tear line and the whole eye shell left out, one at a time: it is the skin. The skin's folds there are alike on the two frames (34 and 30 edges folded past 60 degrees). The normals the engine adds up from the frame's shape and the two controls' are off by up to 14 degrees there on the feminine frame against 6 on the masculine. Not changed. (Letting the film settle toward the eyeball as the lids close did nothing for it and was withdrawn.)

## Checks

On the committed build:

- `face_checks.py --check`: exit 0, 0 failures. `Blink` and `Eye_Closed`: no ray bare on either frame. No limit was changed.
- Reported, not held: lid vertices standing in the clear cornea, 28 over all poses on the masculine frame and 27 on the feminine (20 and 15 in round 2: there are twice the vertices along a lid), to 0.54 mm at the most as before (two under `Eye_Squint` and two under `Cheek_Raise`, to 0.27 and 0.30 mm; none in Sleepy, four before). Pairs of crossing triangles: 172 with a closed eye (96), 13 under `Eye_Squint` and 27 in Laughing (none), all between the two lids' lining behind the outer corner.
- `measure.py --package realistic --check`: the same lines as before this round (the two known shoulder lines, the sliders' section not run, four targets not met), none added.
- `cargo nextest run -p axcenity-anim -p axcenity-avatar --no-fail-fast`: 223 pass.
- New in `Realistic.build.json`: `face.blink_shares` (above).

## Still short

1. **A narrowed eye's inner end** keeps a thin grey wedge of the white, and on the feminine frame a faint pale patch remains on the skin under the inner corner in a laugh (round 3: it is the skin, not a card or the film).
2. **No creases.** A laughing eye has no lines fanning from its outer corner, lowered brows no furrow between them and raised brows no lines across the forehead: geometry at this density cannot fold that finely and the skin has no wrinkle maps.
3. **The eye is shaded by a film, not by the renderer.** It is right for one head-on light; it does not know where the light is, and where it is drawn the renderer's own occlusion is lost (the film stands in for it). The eye's shading is CO7's. With the eye wide the film leaves pale shards low on the white beside the iris (round 3: looked for, not found).
4. **A half-shut eye** has paler three-cornered areas low on its whites.
5. **The closed lid's lit blotch** under a side light (round 2): not a card's shadow; not changed.
6. **`Brow_Lower` under the hard light**: two thin bright lines along the masculine lid.
7. **Brows raised and drawn in** stretch the skin's grain between them into short upright streaks, and the feminine forehead's skin lies up to 1.2 mm under and 1.9 mm over its resting surface with both controls at 1 (the shapes are made on the masculine skull).
8. **`Eye_Wide`, feminine**: the lid's skin above the lash line is a lit band on the key side (round 3: what it is; not changed).
9. **The iris's outline** is 24 straight pieces (`eyes.SEGMENTS`), which shows at twice the size now that the lids round it are curves. The inner corner of the eye is the bundle's: the lower lid leaves it in a short straight run.
10. **Lashes and brows are cards.** Close up a brow is a dark mass with hairs at its edges; the lashes of the two lids do not interleave; a closed eye's lashes are longer than round 1's.
11. Under the hard light a patch of the closed feminine lid is stippled where it is lit (the renderer's shadow edge at a grazing light).
12. **`Eye_Closed` on top of a laugh** still passes the lids through each other from 0.2 (a dashed line, bottom rows of the squint sheets), and by the build's own measure the engine's three shares are a little low at the outer corner (round 3).
