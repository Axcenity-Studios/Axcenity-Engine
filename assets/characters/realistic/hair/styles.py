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


# The style modelled from scratch against reference photographs (wave 5,
# decision 14; the references and the measured description are in
# docs/development/character-studio/references/hair/README.md, "crop-taper").
# A short textured crop: the top 40 to 46 mm, pushed forward with lifted,
# piecey ends and a fringe that stops at the hairline; the sides scissor-
# tapered from about 36 mm at the top of the side to 16 mm over the ears;
# the back 28 to 36 mm, tapering down the nape to about 10 mm over a soft
# 16 mm band; a clockwise whorl at the crown, a little left of the middle;
# no parting. Built by `guides.py` and `clump_cards.py` (guides by zone,
# clumps, cards from clumps and tiles drawn from their strands).
# What qa_hair.py measured at every head parameter's ends for the crop
# (refinement/wave5/hair-scratch/measurements.json).
ENDS_HELD = ("Measured over this range (draft): no hair vertex more than 1 mm inside the head "
             "at either end, on either frame (deepest 0.78 mm, at the roots).")

CROP_TAPER = {
    "id": "crop-taper",
    "method": "guides",
    "item": "axcenity/realistic-hair-crop-taper",
    "file": "CropTaper.glb",
    "object": "Hair_CropTaper",
    "label": "Textured crop, tapered",
    "about": ("A short textured crop: longer on top and pushed forward with lifted, "
              "piecey ends, scissor-tapered at the sides and down the nape."),
    "tags": ["short", "straight", "crop", "textured", "taper"],
    "seed": 5309,
    # The hairline (elevation against azimuth, as the first style): the
    # temples a little more recessed (the references' adult hairlines).
    "hairline": [
        (0, 39.0), (18, 38.0), (32, 33.0), (45, 25.0), (58, 14.0),
        (68, 2.0), (76, -10.0), (84, -14.0), (120, -15.0), (135, -25.0),
        (150, -37.0), (165, -44.0), (180, -46.0),
    ],
    # Density ramps (metres): front and temples, the sides, and the nape's
    # taper (a declared fade, references c16d1119 and 2eed5bf2).
    "ramp_front": 0.011,
    "ramp_side": 0.009,
    "ramp_back": 0.016,
    "ear_clear": 0.009,
    # The crown's whorl (azimuth, elevation; a little to the left).
    "crown": (172.0, 58.0),
    # The zones the guides are grown from: length and height off the
    # scalp (metres) by elevation, at the front, the side and the back.
    "zones": {
        "length_front": [(-20, 0.022), (10, 0.028), (30, 0.040), (50, 0.046), (90, 0.042)],
        "length_side": [(-30, 0.016), (-10, 0.017), (10, 0.021), (30, 0.027), (60, 0.036), (90, 0.040)],
        "length_back": [(-70, 0.010), (-50, 0.013), (-30, 0.018), (-10, 0.023), (20, 0.028),
                        (50, 0.033), (90, 0.036)],
        "height_front": [(-20, 0.010), (10, 0.012), (30, 0.015), (50, 0.017), (90, 0.016)],
        "height_side": [(-30, 0.0125), (-10, 0.0125), (10, 0.012), (30, 0.0125), (60, 0.015), (90, 0.016)],
        "height_back": [(-70, 0.0105), (-45, 0.012), (-20, 0.012), (10, 0.012), (40, 0.014), (90, 0.015)],
        # The whorl turns hair this many degrees at its centre, fading out
        # over `whorl_reach` (metres).
        "whorl_turn": 40.0,
        "whorl_reach": 0.035,
        # Sides combed a little back; the nape a little in.
        "side_back": 0.35,
        "nape_in": 0.25,
        "gravity_from": 35.0,
        "gravity_full": 5.0,
        "sweep": 10.0,
        # Tips lift on top (from `lift_from` degrees up) and settle on the
        # sides and back, as shares of the height.
        "lift_from": 20.0,
        "tip_lift": 0.55,
        "tip_settle": -0.15,
        "guide_spacing": 0.012,
        "ear_keep": 0.008,
        "overhang_front": 0.003,
        # Where the front's tips stop against the hairline (metres past it;
        # negative inside), drawn per clump.
        "fringe_stop": (-0.009, -0.001),
        # The sides, back and nape stop this far into the ramp (shares of
        # it), and taper shorter toward the hairline: by `taper` of their
        # length at the line, over `taper_reach` (metres) in from it.
        "side_stop": (0.4, 1.0),
        # The hairline's own fine clumps end this far past the line
        # (metres; negative inside).
        "hairline_stop": (-0.001, 0.0015),
        # Behind `behind_from` degrees of azimuth (over the ears, the
        # nape), where hair grows down toward the line: the stops there.
        "behind_from": 88.0,
        "hairline_stop_behind": (0.0, 0.006),
        "side_stop_behind": (0.1, 0.7),
        "taper": 0.45,
        "taper_reach": 0.03,
        # The height off the scalp falls to this share at the hairline,
        # over `height_reach` (metres) in from it.
        "height_at_line": 0.35,
        "height_reach": 0.014,
    },
    # The layers, deepest first. `tile` (pixels), `tiles` per layer and
    # `strands` per tile; `converge` how far a clump's strands meet at
    # its tip, `ends` where they end (shares of its length); `bow` the
    # card's bow across; `keep` the gameplay copy's share; `crowd` in the
    # crowd copy.
    "layers": [
        {"name": "under", "where": "all", "spacing": 0.007, "width": 0.013, "length": 0.85,
         "length_spread": (0.85, 1.05), "height": 0.6, "segments": 4, "tile": (240, 620), "tiles": 6,
         "strands": (330, 380), "converge": 0.45, "ends": (0.55, 1.0), "gain": 2.6, "hair": 1.3,
         "frizz": 0.015, "bow": 0.05, "card_taper": 0.2, "keep": 1.0, "crowd": True,
         "crowd_spacing": 2.1, "crowd_width": 1.8, "jitter": 6.0},
        {"name": "whorl", "where": "all", "within_crown": 0.03, "tiles_from": "under", "spacing": 0.0045,
         "width": 0.011, "length": 0.7, "length_spread": (0.8, 1.05), "height": 0.8, "segments": 4,
         "bow": 0.05, "card_taper": 0.2, "keep": 1.0, "jitter": 20.0},
        {"name": "outer", "where": "all", "spacing": 0.0068, "width": 0.0095, "length": 1.0,
         "length_spread": (0.8, 1.12), "height": 1.0, "segments": 6, "tile": (180, 760), "tiles": 10,
         "strands": (150, 190), "converge": 0.7, "ends": (0.62, 1.0), "gain": 1.8, "hair": 1.15,
         "frizz": 0.02, "strays": 0.05, "bow": 0.12, "card_taper": 0.2, "cross": 0.2, "keep": 0.6,
         "crowd": True, "crowd_spacing": 3.0, "crowd_width": 2.2, "jitter": 9.0},
        {"name": "hairline", "where": "hairline", "spacing": 0.003, "width": 0.005, "length": 1.0,
         "length_spread": (0.8, 1.1), "height": 0.6, "segments": 4, "tile": (90, 360), "tiles": 10,
         "strands": (14, 28), "converge": 0.35, "ends": (0.5, 1.0), "gain": 1.5, "hair": 0.95,
         "frizz": 0.03, "strays": 0.08, "root_stagger": 0.15, "taper": 0.5, "bow": 0.0,
         "card_taper": 0.1, "keep": 0.75, "jitter": 12.0},
        {"name": "fly", "where": "top", "spacing": 0.02, "width": 0.003, "length": 1.1,
         "length_spread": (0.95, 1.2), "height": 1.15, "segments": 6, "tile": (44, 420), "tiles": 12,
         "strands": (1, 3), "converge": 0.0, "ends": (0.85, 1.0), "gain": 1.3, "hair": 0.95,
         "frizz": 0.08, "strays": 0.5, "taper": 0.5, "bow": 0.0, "card_taper": 0.0, "keep": 0.0,
         "jitter": 18.0},
    ],
    # The cap: the scalp lifted `lift` (metres) past `inset` of the ramp,
    # mapped triangle by triangle into a whole, darker patch (`shade` of
    # the cards' colour) of `strands` short strands, at `px_per_cm`.
    "cap": {"tile": (256, 256), "strands": 9000, "inset": 0.85, "lift": 0.0012, "px_per_cm": 40.0,
            "shade": 0.55},
    # The style's declared minimum thickness off the scalp past the
    # hairline's ramps (asset-quality.md §4: 8 mm for a crop).
    "thickness_min": 0.008,
    "roughness": 0.45,
    "holds": {
        "frame": [0.0, 1.0],
        "head_size": [0.88, 1.12],
        "temple_width": [-1.0, 1.0],
        "forehead_height": [-1.0, 1.0],
        "forehead_slope": [-1.0, 1.0],
        "brow_ridge": [-1.0, 1.0],
    },
    # Rewritten from the measurements (refinement/wave5/hair-scratch/
    # measurements.json) whenever the style is re-measured.
    "measured": {
        "frame": ("Built for both frames; a draft on both. In Idle, Walk and Run no hair vertex goes "
                  "more than 1 mm inside the head (deepest 0.78 mm, at the roots). Front scalp "
                  "show-through away from the hair's edge 6.3% (masculine) and 1.9% (feminine) against "
                  "2% allowed."),
        "head_size": ENDS_HELD,
        "temple_width": ENDS_HELD,
        "forehead_height": ENDS_HELD,
        "forehead_slope": ENDS_HELD,
        "brow_ridge": ENDS_HELD,
    },
}

STYLES = {SHORT_CROP["id"]: SHORT_CROP, CROP_TAPER["id"]: CROP_TAPER}
