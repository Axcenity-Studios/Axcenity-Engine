"""The realistic family's hairstyles as data (Character Studio R6, CO11),
and the hair colour presets every style carries.

A style is a groom's parameters: where hair grows (the hairline, its
ramps), how long it is over the head, which way it is combed, how far it
stands off the scalp, its wave, and the card layers it is built from.
`build_hair.py --style <id>` turns one into a model and an item manifest.

Paul signs off the first style of a kind before more are made
(`docs/development/character-studio/asset-quality.md` §1), so there is
one style here: a short textured crop, which the references cover well
and which reads on both frames.

Angles are degrees; lengths metres. Azimuth `phi` runs round the head's
vertical axis from the front (0) to the back (180), the same on both
sides; elevation `theta` is above (positive) or below the head's centre,
seen from that centre (`scalp.HeadFrame`).
"""

# -- Hair colour presets (brief §7.2) -----------------------------------------
#
# By melanin and redness, the Hair model's own pigment values
# (`docs/rendering/hair.md`: melanin moves the absorption x8 at 1 and /8 at
# -1 from the atlas's colour; redness moves pheomelanin's share). The
# atlas is painted a medium brown (eumelanin 0.55, pheomelanin 0.12:
# `atlas.BASE_EUMELANIN`), so black is about x8 of it and platinum about
# /8. Grey is pale ash with a dye of grey mixed in: grey hair is white
# strands among pigmented ones, which no single melanin gives.
# Darkest first; each is a variant of every realistic style.
COLOURS = [
    ("black", "Black", {"melanin": 0.95, "redness": -0.3}),
    ("dark-brown", "Dark brown", {"melanin": 0.5, "redness": 0.0}),
    ("brown", "Brown", {"melanin": 0.1, "redness": 0.15}),
    ("auburn", "Auburn", {"melanin": 0.0, "redness": 0.9}),
    ("ginger", "Ginger", {"melanin": -0.45, "redness": 1.0}),
    ("blonde", "Blonde", {"melanin": -0.85, "redness": 0.35}),
    ("platinum", "Platinum", {"melanin": -1.0, "redness": -0.5, "dye": "#e9e2d0", "dye_amount": 0.35}),
    ("grey", "Grey", {"melanin": -0.75, "redness": -1.0, "dye": "#bdbcb8", "dye_amount": 0.55}),
]

# Two colourings that show the style's roots and highlights (the atlas's
# root-to-tip and streak channels).
COLOURINGS = [
    ("blonde-dark-roots", "Blonde, dark roots",
     {"melanin": -0.85, "redness": 0.35, "root_melanin": 0.9, "root_length": 0.3}),
    ("brown-highlights", "Brown, highlights",
     {"melanin": 0.1, "redness": 0.15, "streaks": 1.0, "streak_melanin": -0.8}),
]

# The values every card style offers (all of the Hair model's: the atlas
# carries root-to-tip and the streak mask, so roots and highlights work).
CARD_VALUES = [
    "melanin", "redness", "dye", "dye_amount", "tint", "tip_lightening",
    "root_melanin", "root_length", "streaks", "streak_melanin",
    "highlight", "shift_scale", "roughness_scale",
]


SHORT_CROP = {
    "id": "short-crop",
    "item": "axcenity/realistic-hair-short-crop",
    "file": "ShortCrop.glb",
    "object": "Hair_ShortCrop",
    "label": "Short textured crop",
    "about": ("Short and textured: longer on top and pushed forward with a "
              "loose, piecey fringe, tapered at the sides and the nape."),
    # Not "straight": its loose wave and flicked tips read as a short
    # wavy crop in the captures.
    "tags": ["short", "wavy", "crop", "textured"],
    "seed": 1931,
    # The hairline: elevation (degrees) against azimuth, between which it
    # is interpolated smoothly. The front sits about 6 cm above the brows
    # (the forehead's top curve), the temples are a little recessed, the
    # sideburn comes down in front of the ear to about its middle, the
    # line goes over the ear (kept `ear_clear` off it) and down the back
    # to the nape.
    "hairline": [
        (0, 39.0), (18, 38.0), (32, 34.0), (45, 27.0), (58, 15.0),
        (68, 2.0), (76, -10.0), (84, -14.0), (120, -14.0), (135, -22.0),
        (150, -31.0), (165, -36.0), (180, -37.0),
    ],
    # Density ramps over this distance (metres) inside the hairline: at
    # the front and temples (asset-quality.md §4: at least 8 mm), and
    # elsewhere.
    "ramp_front": 0.011,
    "ramp_side": 0.008,
    "ear_clear": 0.009,
    # Length over the head: (elevation, length) at the front, the crown
    # and the back, blended by azimuth. Top 55 to 65 mm, sides tapering
    # to about 20 mm over the ears, the nape to 14 mm.
    "length_front": [(-20, 0.024), (10, 0.030), (30, 0.052), (50, 0.062), (90, 0.060)],
    "length_side": [(-20, 0.020), (0, 0.022), (20, 0.030), (40, 0.046), (60, 0.056), (90, 0.058)],
    "length_back": [(-70, 0.014), (-45, 0.018), (-20, 0.026), (10, 0.036), (40, 0.048), (90, 0.054)],
    # How far past the hairline a layer's strands may reach (metres; less
    # than 0 stops them inside it): the fringe over the forehead, the rest
    # ending inside the ramp so the dense layers never make the line.
    "overhang_front": 0.004,
    "overhang": -0.002,
    "overhang_hairline": 0.0005,
    # The crown's whorl (azimuth, elevation): hair is combed away from it.
    "crown": (165.0, 58.0),
    # Toward the front, the top is swept a little to the character's
    # left (degrees of turn at the front).
    "sweep": 12.0,
    # Below this elevation the sides and back are combed down (gravity),
    # fully below `gravity_full`.
    "gravity_from": 35.0,
    "gravity_full": 5.0,
    # How steeply hair leaves the scalp (degrees) at the top and at the
    # sides and nape, and how far the layer stands off the scalp: at
    # least `thickness_min` (the style's declared volume, asset-quality §4)
    # and at most `thickness_max`.
    "lift_top": 32.0,
    "lift_side": 18.0,
    "thickness_min": 0.008,
    "thickness_max": 0.019,
    # Texture: a loose wave across the strand (amplitude, metres; waves
    # along a strand), and how many of the top's clumps flick up at the
    # tips, piecey.
    "wave": 0.0018,
    "waves": 1.2,
    "flick": 0.35,
    # Card layers: (name, spacing between roots in metres, card width,
    # height share of the layer's thickness, tile kind, where: "all", "top"
    # or "hairline"). Hero keeps every layer; the gameplay copy keeps
    # `keep` of each layer's cards; the crowd copy is the cap and the
    # coverage cards, coarser.
    "layers": [
        ("coverage", 0.0125, 0.018, 0.45, "coverage", "all"),
        ("mid", 0.0105, 0.015, 0.75, "clump", "all"),
        ("top", 0.0085, 0.011, 1.05, "detail", "top"),
        ("short", 0.0085, 0.010, 1.05, "short", "sides"),
        ("hairline", 0.0042, 0.007, 0.35, "hairline", "hairline"),
        ("flyaway", 0.020, 0.0024, 1.25, "flyaway", "top"),
    ],
    "keep": {"coverage": 1.0, "mid": 0.85, "top": 0.55, "short": 0.75, "hairline": 0.8, "flyaway": 0.0},
    # Roughness (the Hair model reads 0.3 to 0.45 as human hair).
    "roughness": 0.3,
    # Holds (measured by `qa_hair.py`): both frames and these ends of the
    # head's parameters (base parameter, [low, high]).
    "holds": {
        "frame": [0.0, 1.0],
        "head_size": [0.88, 1.12],
        "temple_width": [-1.0, 1.0],
        "forehead_height": [-1.0, 1.0],
        "forehead_slope": [-1.0, 1.0],
        "brow_ridge": [-1.0, 1.0],
    },
    # What `qa_hair.py` and the captures measured over each range, written
    # as the item's limitation notes (the browser shows them). They state
    # the measurement, never a pass the evidence doesn't show
    # (refinement/wave4/r6-hair-first/measurements.json): rewrite them
    # whenever the style is re-measured.
    "measured": {
        "frame": ("Built for both frames; a draft on both. Front scalp show-through "
                  "5.7% (masculine) and 1.7% (feminine) against 2% allowed; in Idle, "
                  "Walk and Run one card vertex goes up to 6.4 mm inside by the ear."),
        "head_size": ("Measured over this range (draft): no hair vertex more than 1 mm "
                      "inside the head at either end, on either frame."),
        "temple_width": ("Measured over this range (draft): at -1, 1 or 2 vertices 6 to "
                         "7 mm inside the head by the ear (masculine hero 6.1 mm, feminine "
                         "gameplay 7.2 mm); none over 1 mm at +1."),
        "forehead_height": ("Measured over this range (draft): no hair vertex more than 1 mm "
                            "inside the head at either end, on either frame."),
        "forehead_slope": ("Measured over this range (draft): no hair vertex more than 1 mm "
                           "inside the head at either end, on either frame."),
        "brow_ridge": ("Measured over this range (draft): no hair vertex more than 1 mm "
                       "inside the head at either end, on either frame."),
    },
}

STYLES = {SHORT_CROP["id"]: SHORT_CROP}
