# CO6 review: eyes, lids, lashes and brows

What the review of the realistic head's upper face found, what was changed and what still falls short. The head itself is described in [README.md](README.md); the package is [assets/characters/realistic/](../../../../assets/characters/realistic/README.md). Code: `lids.py`, `lashes.py`, `eyes.py`, and in `face.py` the eye, brow, cheek and nose entries of `Face.expressions`, `Face.brow`, `Face.cheek` and `soften_lids`.

Every picture named here is a crop of a 3840 x 2160 capture from the engine's renderer (`capture_characters`, the straight-on `face-front` view) of the committed, baked build; masculine frame on the left, feminine on the right, a row per pose. They are in [engine/](engine/).

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

**Now** ([engine/eyes-squint.jpg](engine/eyes-squint.jpg), [engine/eyes-squint-hard.jpg](engine/eyes-squint-hard.jpg)): one unbroken lid line above and below at 0.5 and 1, alone and together, on both frames; the upper lashes stand as lashes; nothing comes through the lids. Laughing's opening is 2.5 mm at the middle (3.75 mm before). The head's triangles that cross each other fell from 49 to none on the masculine frame in Laughing and from 72 to 18 on the feminine (`face_checks.py`).

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

**Now** ([engine/eyes-closed.jpg](engine/eyes-closed.jpg), and at twice the size [engine/eyes-closed-close.jpg](engine/eyes-closed-close.jpg)): at `Blink` 1 and `Eye_Closed` 1, in both lights on both frames, no speck, no blob and no white point; the lash line reads as dark lashes with separate tips; the lit part of the lid is smooth. The closed lid's inner edge stands 1.3 mm off the eyeball at the middle and its outer edge 3.4 mm. `face_checks.py`: no ray bare on either frame; crossing triangle pairs in the closed eyes 175 (251) masculine, 182 (276) feminine.

## 3. The brows' cards

**Found.** A card was 3 vertices across and 15 along, each on the nearest point of the skin and 0.5 mm over it, and under a shape all three across moved as the skin under the middle one did. The brow's shapes moved the skin under the card's lower edge 0.70 of what they moved the middle (3.2 mm less at `Brow_Raise`), so the card slid over a curved brow ridge: it cut into the skin on one side and stood off it on the other (the ragged top at `Brow_Lower` 0.15). Between its vertices the flat card cut under the round ridge at rest (the light hole). The texture ended square at the tail.

**Changed.**

- The brow's skin moves whole within 6.5 mm of the brow's line (`face.Face.brow`), easing off beyond; the fold over the upper lid takes less of it loop by loop.
- A card is 5 by 23 vertices; each vertex moves as its own point of the skin does, and no face of the card comes nearer the skin than 0.3 mm at rest or under any shape (`lashes.over_skin`).
- The texture thins to a point at the tail; the card is shaded as the lashes are (a raised brow, turned to the sky, went grey).

**Now** ([engine/brows-controls.jpg](engine/brows-controls.jpg), [engine/brows-controls-hard.jpg](engine/brows-controls-hard.jpg)): the cards are whole at rest, at `Brow_Lower` 0.15 and 1, `Brow_Raise` 1 (both and one side), `Brow_Inward` 1 and `Brow_Lower` with `Brow_Inward`, and in every expression of [engine/eyes-expressions.jpg](engine/eyes-expressions.jpg); no skin shows through. [engine/brows-arch.jpg](engine/brows-arch.jpg): the identity slider `brow_arch` at -1 and +1 flattens and peaks the brow without a hook. (The brow's identity shapes are made on the same weights, so they moved a little: `Face_Brow_Arch` 6.3 mm at its most, 6.1 before.)

## 4. Eyes wide

**Found.** The marks above the lids were the upper lashes: they turned all the way with the lid as it slid up under the fold and lay flat in the fold's skin. The lid's edge rose 3 mm, which left it on the iris.

**Changed.** An upper lid drawn up gives its lashes 0.3 of its turn, and they are kept off the skin. `Eye_Wide` raises the upper lid's edge 4.2 mm at the middle.

**Now** ([engine/eyes-wide.jpg](engine/eyes-wide.jpg)): white over the iris at `Eye_Wide` 1 and in Surprised on both frames, the lashes fanned up, nothing left on the skin above the lid.

## 5. The brow controls as muscles

| Control | Was | Is |
|---|---|---|
| `Brow_Lower` | The brow's skin 7.8 mm down at the inner end, 3.5 mm at the tail, 3.6 mm inward; the upper lid 14% of the way shut. | The same moves on the new weights, the skin between the brows 1.6 mm forward, the fold coming down on the lid and the lid 22% of the way shut (the opening 7.9 mm from 9.5; it was 8.35). |
| `Brow_Raise` | 10.5 mm up (the inner end 7.9 mm), the lid 10% wider. | The same, the fold drawn up with it, the lid 12% wider (the opening 10.5 mm). One side alone leaves the other brow where it was. |
| `Brow_Inward` | The inner end 8.2 mm **up** and 3.4 mm inward, the tail 2.6 mm down: an inner brow raise under another name. | The inner half 4.8 mm toward the middle, level, the skin between 2.4 mm forward. No vertex is drawn more than 0.4 of its way to the middle of the face. |

`Brow_Inward` was changed because of what the profile uses it for: ARKit's `browDown` plays `Brow_Lower` with 0.3 of `Brow_Inward`, and Frustrated lowers the brows with 0.3 of it; an inner end that rises works against both.

What the pictures show ([engine/brows-controls.jpg](engine/brows-controls.jpg)): `Brow_Lower` brings the inner ends down and toward each other and narrows the eye from above, with a small swelling between the brows; there is no furrow. `Brow_Raise` lifts the brow and shows more lid. `Brow_Inward` brings the brows' heads together with a swelling between them that catches the light; there are no vertical lines. The mesh's edges there are 3 to 4 mm and the skin has no wrinkle maps.

**The inner brow raise.** [engine/brows-inner-raise.jpg](engine/brows-inner-raise.jpg): rest; `Brow_Raise` with `Brow_Inward`, both at 1; Sad, Worried and Fearful as their recipes ask; and Sad and Worried on the starting build, whose `Brow_Inward` raised the inner ends (and whose cards broke on the feminine frame). With the controls as they are named, `Brow_Raise` with `Brow_Inward` gives raised brows drawn together, their inner ends no higher than their tails: Sad (0.7 of `Brow_Inward`) and Worried (0.8 with 0.2 of `Brow_Raise`) do not read as sad or worried from the nose up, and Fearful reads as alarm by its eyes. An oblique brow needs the inner end raised alone, which no control among the 48 does.

## 6. One eye alone

`Blink` now has 0.22 of the cheek's lift and 0.14 of the lowered brow's skin on its side (`Eye_Closed`, an eye at rest, has the lids alone). [engine/eyes-wink.jpg](engine/eyes-wink.jpg): the eye shuts, its cheek and brow move a little (1.6 mm and 1.1 mm at their most), the other eye is open and still.

## 7. The upper face in the expressions

[engine/eyes-expressions.jpg](engine/eyes-expressions.jpg) (each at 0.5 and 1, both frames, tiles at half size) and [engine/eyes-faces.jpg](engine/eyes-faces.jpg) (whole faces at a third: rest, blinks, Happy, Laughing, Surprised, Angry, Sad).

| Expression | From the nose up |
|---|---|
| Angry | Reads at 1 (brows down and in, eyes narrowed from above and below); mild at 0.5. |
| Surprised | Reads at both (brows up, white over the iris). |
| Fearful | Wide eyes and raised brows; without oblique brows it is close to Surprised. |
| Confused | Reads: one brow up, the other down with its eye narrowed. |
| Disgusted | Lowered brows and narrowed eyes; from the nose up it is a milder Angry (the nose and lip carry it). |
| Determined, Frustrated | Lowered brows, slightly narrowed eyes; both a mild Angry, as before. |
| Sleepy | Heavy lids at 0.5; at 1 the eyes are 3.2 mm slits whose whites are broken by dark shapes (below). |
| Sad, Worried | Do not read (the inner brow raise, above). |

Recipes are not edited here. With the controls as they are, nothing set by hand through `AXCENITY_CAPTURE_FACE` makes Sad or Worried read better than `Brow_Raise` with `Brow_Inward` does in the sheet above.

## Checks

On the committed build:

- `face_checks.py --check`: exit 1 with 1 failure, the masculine `Mouth_Left`'s one tooth point (to 0.65 mm), as before. `Blink` and `Eye_Closed`: no ray bare on either frame. Half-way blinks: no lid vertex in the opaque eyeball; lid vertices in the clear cornea 3, 6 and 0 at a quarter, a half and three quarters (6, 8 and 0 before; to 0.56 mm); eyeball vertices in the lid 0, 1 and 0 on both frames (2, 0, 0 and 1, 0, 0 before).
- Reported, not held: crossing pairs of the head's triangles, less the resting face's, on the masculine frame: `Brow_Lower_Left` 10 and `_Right` 7 (none before), `Eye_Wide` 6 and 7 (2 and none), Disgusted 7 (none); none of these on the feminine frame. `Eye_Squint` 0 (17 and 20 before), Crying 0 (59).
- `cargo nextest run -p axcenity-anim -p axcenity-avatar`: 222 pass. The body's shape list is unchanged (48 live, 142 in all).
- Triangles: `Lashes` 88 (160), `Brows` 352 (112), `EyeShell` 208.

## Still short

1. **Sad and Worried** have no inner brow raise to use (section 5).
2. **No creases.** A laughing eye has no lines fanning from its outer corner, lowered brows no furrow between them and raised brows no lines across the forehead: geometry at this density cannot fold that finely and the skin has no wrinkle maps.
3. **The slit of a squint is pale.** The whites are lit as at rest (the eye's shading is CO7's), so a laughing eye is a pale slit under dark lashes where the photographs have a dark crescent; the lower lashes hardly show.
4. **The squint's inner corner** stays a little open (the lower lid is held there), and at `Eye_Squint` with `Cheek_Raise` both at 1 a faint pale mark remains beside it on the feminine frame. Its cause is not proven: it is in the skin's shading, not in any part (it stays with the eye shell, the lashes, the eyeballs and the simpler copies each taken out), and the likeliest is that a shape's normals are stored as changes from the masculine frame's where the two frames' skin differs.
5. **A half-shut eye** (`Blink` 0.5, Sleepy at 1): the whites are broken by dark, straight-edged shapes next to the iris, as they were before this work. The cause was not found (the occlusion film's coarse rings, the lash cards' shadows, or both).
6. **`Brow_Lower` on the feminine frame** shows the lowered lid as a pale band in the key light: the softened lid is smoother than the skin round it.
7. **`Brow_Lower` with `Brow_Inward`, both at 1** (no recipe asks for it): a short crease between the brows on the masculine frame.
8. **The lids' outline is angular** at 4K (8 vertices along the upper lid's edge), most with the eye wide.
9. **A blink during a squint.** Shapes add: `Blink` on top of Laughing carries the upper lid past the lower one. The engine scales a blink by what `Eye_Closed` leaves open, not by a squint.
10. **Lashes and brows are cards.** Close up a brow is a dark mass with hairs at its edges; the lashes of the two lids do not interleave.
11. Under the hard light a patch of the closed feminine lid is stippled where it is lit (the renderer's shadow edge at a grazing light; it is in the starting build's pictures too).
