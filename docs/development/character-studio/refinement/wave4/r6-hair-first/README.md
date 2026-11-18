# R6: the first realistic hairstyle (evidence)

The short textured crop, `axcenity/realistic-hair-short-crop` (`assets/characters/realistic/hair/`). The hand-back is [../r6-hair-first.md](../r6-hair-first.md); the numbers are in [measurements.json](measurements.json).

## Paul's sign-off (asset-quality.md §1: the first hairstyle)

| Item | Status | Look at | Decision |
|---|---|---|---|
| Short textured crop (`axcenity/realistic-hair-short-crop`) | **Draft.** Not ready for sign-off: it fails four measured checks of asset-quality §4 (hairline ramp, volume, front show-through, a vertex or two inside the head) and the review finds visible defects (below) | `close-masculine-studio-hair-hairline.jpg`, `close-masculine-studio-hair-temple.jpg`, `close-masculine-hard-hair-crown.jpg`, `close-masculine-studio-hair-nape.jpg` (the defects at 4K); `views-masculine-studio.jpg`, `views-feminine-studio.jpg`; `matrix-rest-masculine.jpg`, `matrix-walk-feminine.jpg`; `colours.jpg` | Pending. Wanted from Paul: whether this direction (cards from a scripted groom with a scalp cap) is worth taking to production, and the TAA stability limit (measured 1.9 to 2.6 of 255 rms; CO7's provisional limit is 4.5) |

## Files

- `matrix-<pose>-<frame>.jpg`: the capture matrix, one sheet per pose (rest, Idle, Walk at 0.3 s) and frame; five bodies a sheet (average, slim, athletic, muscular, heavyset, top to bottom), each a block of four rows (studio, hard, warm, outdoor light) and six columns (front, three-quarter, side, back, face, face-front). A third of the captures' size. The hair is drawn at its gameplay level in the full-body views and its hero level in the face views.
- `close-<frame>-<light>-<view>.jpg`: 1600 × 900 crops at full resolution of 3840 × 2160 captures (studio and hard light): the front hairline, the left temple and sideburn, the crown's whorl (the style has no parting), the nape (the ends).
- `views-<frame>-studio.jpg`: the six hair views of those captures, reduced (top, back, crown, hairline, temple, nape).
- `colours.jpg`: the ten colour variants on the masculine average character, face and back, studio light.
- `measurements.json`: every asset-quality §4 number, with how it was measured and whether it passes.

## What the pictures show

- It reads as short hair on both frames and all ten bodies, follows the head through Idle and Walk, and holds its shape; under warm and hard light it reads as brown hair.
- **Not natural yet.** Close up the hair reads as brush strokes, not strands; under the studio and outdoor light a broad pale grey sheen lies over the top, so brown hair looks grey from the front; the cap shows as flat pale patches between cards at the front hairline; the crown's whorl is a pinwheel with a bright fan under hard light; the nape ends bluntly in a jagged line; the temple's edge is a dark band rather than a soft ramp.
- The colours run black to dark brown to brown in the back views; auburn and ginger read as brown and drab, and blonde, platinum and grey as greys (redness alone only lightens; the sheen greys every light colour).
