"""The crew-neck T-shirt's pattern: FreeSewing's **Teagan** (a T-shirt),
drafted on FreeSewing's **Brian** block (`brian.base`, `brian.sleevecap`),
from a body's measurements. A Python port of
`designs/brian/src/base.mjs`, `sleevecap.mjs` and
`designs/teagan/src/front.mjs`, `back.mjs`, `sleeve.mjs` (FreeSewing 4.0.0,
https://codeberg.org/freesewing/freesewing), with Teagan's default
options. Used under the MIT licence; the notice is in `freesewing.py`.

Only the seam lines are drafted (no seam allowance, no annotations): the
garment pipeline sews along them. The pieces, in millimetres, y down,
the front and back from the centre line (cut on the fold) to the side:

- `front`, `back`: hem, side seam, armhole, shoulder, neckline;
- `sleeve`: hem, underarm seams, the cap from front to back;
- `neckband`: Teagan's neck opening length, for the rib band (Teagan
  finishes the neck with a band the pattern doesn't draw: 80% of the
  opening, a common rib ratio).
"""

import math

from freesewing import Path, Point, beam_intersects_x, beam_intersects_y, beams_intersect

# Teagan's options over Brian's (front.mjs, sleeve.mjs), as fractions.
OPTIONS = {
    # Brian, as Teagan sets them.
    "brianFitSleeve": True,
    "brianFitCollar": True,
    "collarFactor": 4.8,
    "bicepsEase": 0.05,
    "chestEase": 0.12,
    "collarEase": 0.0,
    "shoulderEase": 0.0,
    "lengthBonus": 0.15,
    "acrossBackFactor": 0.98,
    "armholeDepth": 0.02,
    "backNeckCutout": 0.08,
    "frontArmholeDeeper": 0.005,
    "shoulderSlopeReduction": 0.0,
    # Teagan.
    "sleeveLength": 0.30,
    "hipsEase": 0.18,
    "necklineDepth": 0.25,
    "necklineWidth": 0.30,
    "necklineBend": 0.30,
    "sleeveEase": 0.15,
    # Brian's sleevecap.
    "sleevecapEase": 0.0,
    "sleevecapTopFactorX": 0.50,
    "sleevecapTopFactorY": 0.45,
    "sleevecapBackFactorX": 0.60,
    "sleevecapBackFactorY": 0.33,
    "sleevecapFrontFactorX": 0.55,
    "sleevecapFrontFactorY": 0.33,
    "sleevecapQ1Offset": 0.017,
    "sleevecapQ2Offset": 0.035,
    "sleevecapQ3Offset": 0.025,
    "sleevecapQ4Offset": 0.01,
    "sleevecapQ1Spread1": 0.10,
    "sleevecapQ1Spread2": 0.15,
    "sleevecapQ2Spread1": 0.15,
    "sleevecapQ2Spread2": 0.10,
    "sleevecapQ3Spread1": 0.10,
    "sleevecapQ3Spread2": 0.08,
    "sleevecapQ4Spread1": 0.07,
    "sleevecapQ4Spread2": 0.063,
    "sleeveWidthGuarantee": 0.85,
}
# The neck band's length against the opening (rib is cut short so it
# lies flat).
NECKBAND_RATIO = 0.8

MEASUREMENTS = ("biceps", "chest", "hpsToBust", "hpsToWaistBack", "neck", "shoulderToShoulder",
                "shoulderSlope", "waistToArmpit", "waistToHips", "hips", "waist")


def base(m, o):
    """Brian's base block (`brian.base`): the points both Teagan's front
    and back start from."""
    p = {}
    p["cbHps"] = Point(0, 0)
    p["cbNeck"] = Point(0, o["backNeckCutout"] * m["neck"])
    p["cbChest"] = Point(0, m["hpsToBust"])
    p["cbWaist"] = Point(0, m["hpsToWaistBack"])
    p["cbHips"] = Point(0, p["cbWaist"].y + m["waistToHips"])
    shoulder_ease = (m["shoulderToShoulder"] * o["shoulderEase"]) / 2
    p["neck"] = Point((m["neck"] * (1 + o["collarEase"])) / o["collarFactor"], 0)
    p["hps"] = p["neck"].copy()
    p["shoulder"] = beams_intersect(
        p["hps"], p["hps"].shift(m["shoulderSlope"] * -1, 100),
        Point(m["shoulderToShoulder"] / 2 + shoulder_ease, -100),
        Point(m["shoulderToShoulder"] / 2 + shoulder_ease, 100))
    p["cbShoulder"] = Point(0, p["shoulder"].y)
    # The non-legacy armhole depth, from waistToArmpit.
    p["cbArmhole"] = Point(0, p["cbWaist"].y - m["waistToArmpit"]
                           * (1 - o["armholeDepth"] - o["bicepsEase"] / 2))
    p["shoulder"].y -= (p["shoulder"].y - p["cbHps"].y) * o["shoulderSlopeReduction"]
    if p["shoulder"].y < p["cbHps"].y:
        p["shoulder"] = Point(p["shoulder"].x, 0)
    p["cbHem"] = Point(0, p["cbHips"].y * (1 + o["lengthBonus"]))
    p["armhole"] = Point((m["chest"] * (1 + o["chestEase"])) / 4, p["cbArmhole"].y)
    p["waist"] = Point(p["armhole"].x, p["cbWaist"].y)
    p["hips"] = Point(p["armhole"].x, p["cbHips"].y)
    p["hem"] = Point(p["armhole"].x, p["cbHem"].y)
    p["armholePitch"] = Point(
        (m["shoulderToShoulder"] * o["acrossBackFactor"]) / 2 + shoulder_ease / 2,
        p["shoulder"].y + p["shoulder"].dy(p["armhole"]) / 2)
    tmp1 = Point(p["armholePitch"].x, p["armhole"].y)
    tmp2 = tmp1.shift(45, 10)
    tmp3 = beams_intersect(tmp1, tmp2, p["armhole"], p["armholePitch"])
    p["armholeHollow"] = tmp1.shift_fraction_towards(tmp3, 0.5)
    p["armholeCp2"] = p["armhole"].shift(180, tmp1.dx(p["armhole"]) / 4)
    p["armholeHollowCp1"] = p["armholeHollow"].shift(-45, p["armholeHollow"].dy(p["armhole"]) / 2)
    p["armholeHollowCp2"] = p["armholeHollow"].shift(135, p["armholePitch"].dx(p["armholeHollow"]))
    p["armholePitchCp1"] = p["armholePitch"].shift(-90, p["armholePitch"].dy(p["armholeHollow"]) / 2)
    p["armholePitchCp2"] = p["armholePitch"].shift(90, p["shoulder"].dy(p["armholePitch"]) / 2)
    p["shoulderCp1"] = p["shoulder"].shift_towards(
        p["neck"], p["shoulder"].dy(p["armholePitch"]) / 5).rotate(90, p["shoulder"])
    tmp4 = p["neck"].shift_towards(p["shoulder"], 10).rotate(-90, p["neck"])
    p["neckCp2"] = beam_intersects_y(p["neck"], tmp4, p["cbNeck"].y)
    # Brian fits a collar here (the cfNeck search); Teagan replaces the
    # front neckline, so it isn't needed.
    deeper = m["chest"] * o["frontArmholeDeeper"]
    for suffix in ("", "Cp1", "Cp2"):
        p["frontArmholePitch" + suffix] = p["armholePitch" + suffix].shift(180, deeper)
    return p


def _armhole(p, pitch="armholePitch"):
    return (Path().move(p["armhole"])
            .curve(p["armholeCp2"], p["armholeHollowCp1"], p["armholeHollow"])
            .curve(p["armholeHollowCp2"], p["shoulderCp1"], p["shoulder"]))


def front(m, o):
    """Teagan's front (`teagan.front`), without `fitWaist` (its default)."""
    p = base(m, o)
    width = (max(m["waist"], m["hips"]) * (1 + o["hipsEase"])) / 4
    p["hem"].x = width
    p["hips"].x = width
    p["waist"].x = width
    p["waistCp2"] = p["waist"].shift(90, p["armhole"].dy(p["waist"]) / 3)
    for key in ("Neck", "Shoulder", "Armhole", "Hips", "Hem"):
        p["cf" + key] = p["cb" + key].copy()
    p["cfNeck"] = Point(0, o["necklineDepth"] * m["hpsToWaistBack"])
    p["cfNeckCp1"] = p["cfNeck"].shift(0, p["neck"].x * o["necklineBend"] * 2)
    p["neck"] = p["hps"].shift_fraction_towards(p["shoulder"], o["necklineWidth"])
    p["neckCp2"] = p["neck"].shift_towards(
        p["shoulder"], p["neck"].dy(p["cfNeck"]) * (0.2 + o["necklineBend"])).rotate(-90, p["neck"])
    p["shoulderCp1"] = beam_intersects_y(p["shoulder"], p["shoulderCp1"], p["armholePitch"].y)
    p["armholeHollowCp2"] = beam_intersects_x(p["armholeHollow"], p["armholeHollowCp2"],
                                              p["armholePitch"].x)
    return p


def back(m, o):
    """Teagan's back (`teagan.back`): the front's points, with the back
    neck and armhole."""
    p = front(m, o)
    p["cbNeck"] = Point(0, p["neck"].y + o["backNeckCutout"] * m["neck"])
    p["cbNeckCp1"] = p["cbNeck"].shift(0, p["neck"].x / 2)
    p["neckCp2"] = beam_intersects_y(p["neck"], p["neckCp2"], p["cbNeck"].y)
    p["shoulderCp1"] = p["shoulderCp1"].shift_fraction_towards(p["shoulder"], 0.25)
    return p


def body_edges(p, centre_neck):
    """A front or back piece's seam lines, each from the centre line
    round to the centre line again: hem (centre to side), side seam (hem
    to armhole), armhole (side to shoulder), shoulder (to the neck),
    neckline (to the centre)."""
    return {
        "hem": Path().move(p["cfHem"]).line(p["hem"]),
        "side": Path().move(p["hem"]).curve_(p["waistCp2"], p["armhole"]),
        "armhole": _armhole(p),
        "shoulder": Path().move(p["shoulder"]).line(p["neck"]),
        "neckline": Path().move(p["neck"]).curve(p["neckCp2"], p[centre_neck + "Cp1"], p[centre_neck]),
        "centre": Path().move(p[centre_neck]).line(p["cfHem"]),
    }


def _sleevecap(m, o, factor, target):
    p = {}
    p["centerBiceps"] = Point(0, 0)
    biceps = m["biceps"] * (1 + o["bicepsEase"])
    # armholeDepthFactor's default (55%) is used for the cap's height.
    p["centerCap"] = p["centerBiceps"].shift(90, o["sleevecapTopFactorY"] * (biceps * 0.55 * factor))
    half = biceps / 2
    p["bicepsLeft"] = p["centerBiceps"].shift(
        180, half * o["sleeveWidthGuarantee"] + half * (1 - o["sleeveWidthGuarantee"]) * factor)
    p["bicepsRight"] = p["bicepsLeft"].flip_x(p["centerBiceps"])
    p["capLeft"] = Point(p["bicepsLeft"].x, p["centerCap"].y)
    p["capRight"] = p["capLeft"].flip_x()
    p["centerCap"] = p["capLeft"].shift_fraction_towards(p["capRight"], o["sleevecapTopFactorX"])
    width = p["bicepsRight"].x
    height = p["centerCap"].y
    p["backPitch"] = Point(-1 * width * o["sleevecapBackFactorX"], height * o["sleevecapBackFactorY"])
    p["frontPitch"] = Point(width * o["sleevecapFrontFactorX"], height * o["sleevecapFrontFactorY"])
    p["capQ1Base"] = p["frontPitch"].shift_fraction_towards(p["bicepsRight"], 0.5)
    p["capQ2Base"] = p["frontPitch"].shift_fraction_towards(p["centerCap"], 0.5)
    p["capQ3Base"] = p["backPitch"].shift_fraction_towards(p["centerCap"], 0.5)
    p["capQ4Base"] = p["backPitch"].shift_fraction_towards(p["bicepsLeft"], 0.5)
    b = biceps
    p["capQ1"] = p["capQ1Base"].shift(p["bicepsRight"].angle(p["frontPitch"]) + 90, b * o["sleevecapQ1Offset"])
    p["capQ2"] = p["capQ2Base"].shift(p["centerCap"].angle(p["frontPitch"]) + 90, b * o["sleevecapQ2Offset"])
    p["capQ3"] = p["capQ3Base"].shift(p["centerCap"].angle(p["backPitch"]) - 90, b * o["sleevecapQ3Offset"])
    p["capQ4"] = p["capQ4Base"].shift(p["bicepsLeft"].angle(p["backPitch"]) - 90, b * o["sleevecapQ4Offset"])
    p["capQ1Cp1"] = p["capQ1"].shift(p["frontPitch"].angle(p["bicepsRight"]), b * o["sleevecapQ1Spread1"])
    p["capQ1Cp2"] = p["capQ1"].shift(p["frontPitch"].angle(p["bicepsRight"]), b * o["sleevecapQ1Spread2"] * -1)
    p["capQ2Cp1"] = p["capQ2"].shift(p["centerCap"].angle(p["frontPitch"]), b * o["sleevecapQ2Spread1"])
    p["capQ2Cp2"] = p["capQ2"].shift(p["centerCap"].angle(p["frontPitch"]), b * o["sleevecapQ2Spread2"] * -1)
    p["capQ3Cp1"] = p["capQ3"].shift(p["backPitch"].angle(p["centerCap"]), b * o["sleevecapQ3Spread1"])
    p["capQ3Cp2"] = p["capQ3"].shift(p["backPitch"].angle(p["centerCap"]), b * o["sleevecapQ3Spread2"] * -1)
    p["capQ4Cp1"] = p["capQ4"].shift(p["bicepsLeft"].angle(p["backPitch"]), b * o["sleevecapQ4Spread1"])
    p["capQ4Cp2"] = p["capQ4"].shift(p["bicepsLeft"].angle(p["backPitch"]), b * o["sleevecapQ4Spread2"] * -1)
    cap = (Path().move(p["bicepsRight"])
           .curve(p["bicepsRight"], p["capQ1Cp1"], p["capQ1"])
           .curve(p["capQ1Cp2"], p["capQ2Cp1"], p["capQ2"])
           .curve(p["capQ2Cp2"], p["capQ3Cp1"], p["capQ3"])
           .curve(p["capQ3Cp2"], p["capQ4Cp1"], p["capQ4"])
           .curve(p["capQ4Cp2"], p["bicepsLeft"], p["bicepsLeft"]))
    return p, cap, cap.length() - target


def sleeve(m, o, armhole_length):
    """Brian's sleevecap fitted to the armhole (`brian.sleevecap`'s loop),
    then Teagan's short sleeve (`teagan.sleeve`)."""
    target = armhole_length * (1 + o["sleevecapEase"])
    factor = 1.0
    for _ in range(50):
        p, cap, delta = _sleevecap(m, o, factor, target)
        if abs(delta) <= 2:
            break
        doverl = delta / cap.length()
        if doverl > 0.1:
            factor *= 0.8
        if doverl > 0.02:
            factor *= 0.9
        elif doverl < -0.1:
            factor *= 1.3
        elif doverl < -0.02:
            factor *= 1.15
        elif delta > 0:
            factor *= 0.99
        else:
            factor *= 1.008
    height = p["bicepsRight"].x * o["sleeveLength"]
    width = m["biceps"] * (1 + o["bicepsEase"]) * (1 + o["sleeveEase"])
    width = min(width, p["bicepsRight"].x * 2)
    p["hemLeft"] = Point(width / -2, height)
    p["hemRight"] = Point(width / 2, height)
    return p, cap


def draft(m, o=OPTIONS):
    """The pieces' seam lines (see the module's notes) and their lengths."""
    f = front(m, o)
    b = back(m, o)
    front_armhole = _armhole(f).length()
    back_armhole = _armhole(b).length()
    s, cap = sleeve(m, o, front_armhole + back_armhole)
    # The cap runs from the front (bicepsRight) over the top to the back
    # (bicepsLeft); it is split where its length matches the front
    # armhole, which is where it meets the shoulder seam.
    sleeve_edges = {
        "hem": Path().move(s["hemLeft"]).line(s["hemRight"]),
        "front_underarm": Path().move(s["hemRight"]).line(s["bicepsRight"]),
        "cap": cap,
        "back_underarm": Path().move(s["bicepsLeft"]).line(s["hemLeft"]),
    }
    front_edges = body_edges(f, "cfNeck")
    back_edges = body_edges(b, "cbNeck")
    opening = 2 * (front_edges["neckline"].length() + back_edges["neckline"].length())
    return {
        "front": front_edges,
        "back": back_edges,
        "sleeve": sleeve_edges,
        "cap_front_length": front_armhole,
        "neck_opening": opening,
        "neckband": opening * NECKBAND_RATIO,
        "points": {"front": f, "back": b, "sleeve": s},
    }


if __name__ == "__main__":
    # A FreeSewing sample-sized body (mm, degrees), to see the draft runs.
    sample = {"biceps": 335, "chest": 1030, "hpsToBust": 290, "hpsToWaistBack": 470, "neck": 400,
              "shoulderToShoulder": 465, "shoulderSlope": 13, "waistToArmpit": 230, "waistToHips": 145,
              "hips": 980, "waist": 880}
    d = draft(sample)
    for piece in ("front", "back", "sleeve"):
        print(piece, {k: round(v.length(), 1) for k, v in d[piece].items()})
    print("cap at the front", round(d["cap_front_length"], 1), "neck opening", round(d["neck_opening"], 1))
