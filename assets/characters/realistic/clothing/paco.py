"""The joggers' pattern: FreeSewing's **Paco** (trousers with an
elasticated waist and elasticated ankle cuffs), drafted on FreeSewing's
**Titan** trouser block, from a body's measurements. A Python port of
`designs/titan/src/back.mjs`, `front.mjs` and `designs/paco/src/back.mjs`,
`front.mjs`, `waistband.mjs`, `cuff.mjs` (FreeSewing 4.0.0,
https://codeberg.org/freesewing/freesewing), with Paco's default
options but two (`OPTIONS`). Used under the MIT licence; the notice is in
`freesewing.py`.

Paco is the jogger shape: its leg narrows from the seat to an ankle as
wide as the heel's girth (the foot must pass), gathered into an elastic
cuff; the waist is gathered into an elastic casing. Only seam lines are
drafted (no seam allowance). The front pocket's flap (Paco's slanted
pocket opening on the outseam) is left out of the outline and drawn by
the maps as a stitched opening (`build_joggers.py`).

The pieces, in mm, y down from the waist line, each for the body's left
leg (x grows toward the body's middle on the front, away from it on the
back, as FreeSewing draws them):

- `front`, `back`: waist (out to in), the rise (crotch seam, waist to
  the fork), the inseam (fork to the hem), the hem, the outseam;
- `waistband`: Paco's casing, its length and width;
- `cuff`: Paco's ankle cuff, its length and width.
"""

import math

from freesewing import Path, Point, beams_intersect

OPTIONS = {
    # Titan, as Paco sets them.
    "fitCrossSeam": True,
    "waistEase": 0.02,
    "seatEase": 0.05,
    "kneeEase": 0.06,
    # Paco's default is 5% (the waist on the hips); joggers sit higher.
    # Within Paco's range (0 to 100%).
    "waistHeight": 0.5,
    "lengthBonus": 0.0,
    "crotchDrop": 0.02,
    "legBalance": 0.575,
    "crossSeamCurveStart": 0.85,
    "crossSeamCurveBend": 0.65,
    "crossSeamCurveAngle": 12,
    "crotchSeamCurveStart": 0.80,
    "crotchSeamCurveBend": 0.80,
    "crotchSeamCurveAngle": 25,
    "waistBalance": 0.60,
    "grainlinePosition": 0.45,
    # Paco.
    "elasticatedCuff": True,
    # Shares of waistToFloor (Paco snaps them to elastic widths; these
    # are 32 and 54 mm on the realistic body). The cuff is Paco's
    # default; the waistband a little wider than its 3%, a jogger's.
    "waistbandWidth": 0.035,
    "ankleElastic": 0.05,
    "heelEase": 0.05,
}


def titan_back(m, o, wb):
    p = {}
    p["waistX"] = Point(-1 * m["waistBackArc"] * (1 + o["waistEase"]), 0)
    p["upperLegY"] = Point(0, m["waistToUpperLeg"])
    p["seatX"] = Point(-1 * m["seatBackArc"] * (1 + o["seatEase"]), 0)
    p["seatY"] = Point(0, m["waistToSeat"])
    p["seatOut"] = p["seatY"].copy()
    p["cbSeat"] = Point(p["seatX"].x, p["seatY"].y)
    p["fork"] = Point(m["seatBackArc"] * (1 + o["seatEase"]) * -1.25, p["upperLegY"].y * (1 + o["crotchDrop"]))
    p["grainlineTop"] = p["upperLegY"].shift_fraction_towards(p["fork"], o["grainlinePosition"])
    p["knee"] = Point(p["grainlineTop"].x, m["waistToKnee"])
    p["floor"] = Point(p["grainlineTop"].x, m["waistToFloor"] * (1 + o["lengthBonus"]))
    knee_total = m["knee"] * (1 + o["kneeEase"])
    store = {"kneeBack": knee_total * o["legBalance"], "kneeFront": knee_total * (1 - o["legBalance"])}
    half = store["kneeBack"] / 2
    p["kneeOut"] = p["knee"].shift(0, half)
    p["kneeIn"] = p["kneeOut"].flip_x(p["knee"])
    p["floorOut"] = p["floor"].shift(0, half)
    p["floorIn"] = p["floorOut"].flip_x(p["floor"])
    p["kneeInCp1"] = p["kneeIn"].shift(90, p["fork"].dy(p["knee"]) / 3)
    p["kneeOutCp2"] = p["kneeOut"].shift(90, p["fork"].dy(p["knee"]) / 3)
    p["seatOutCp1"] = p["seatOut"].shift(-90, p["seatOut"].dy(p["knee"]) / 3)
    p["seatOutCp2"] = p["seatOut"].shift(90, p["seatOut"].y / 2)
    if p["cbSeat"].x < p["waistX"].x:
        delta = p["cbSeat"].dx(p["waistX"])
        p["waistIn"] = p["waistX"].shift(180, delta * (1 - o["waistBalance"]))
    else:
        p["waistIn"] = p["waistX"].copy()
    p["waistOut"] = p["waistIn"].shift(180, p["waistX"].x)

    def cross_seam():
        p["crossSeamCurveStart"] = p["waistIn"].shift_fraction_towards(p["cbSeat"], o["crossSeamCurveStart"])
        p["crossSeamCurveMax"] = beams_intersect(p["waistIn"], p["cbSeat"], p["fork"], p["fork"].shift(0, 666))
        p["crossSeamCurveCp1"] = p["crossSeamCurveStart"].shift_fraction_towards(p["crossSeamCurveMax"], o["crossSeamCurveBend"])
        p["crossSeamCurveCp2"] = p["fork"].shift_fraction_towards(p["crossSeamCurveMax"], o["crossSeamCurveBend"]).rotate(
            o["crossSeamCurveAngle"], p["fork"])
        p["forkCp2"] = p["crossSeamCurveCp2"].rotate(-90, p["fork"])

    def delta():
        return (Path().move(p["waistIn"]).line(p["crossSeamCurveStart"])
                .curve(p["crossSeamCurveCp1"], p["crossSeamCurveCp2"], p["fork"]).length() - m["crossSeamBack"])

    cross_seam()
    if o["fitCrossSeam"]:
        d = delta()
        for _ in range(15):
            prev = d
            saved = {k: p[k].copy() for k in ("waistIn", "waistOut", "fork")}
            for k in ("waistIn", "waistOut"):
                p[k] = p[k].rotate(d / 15, p["seatOut"])
            p["fork"] = p["fork"].shift(0, d / 5)
            cross_seam()
            d = delta()
            if abs(d) > abs(prev):
                p.update(saved)
                cross_seam()
                break
            if abs(d) <= 1:
                break

    def inseam():
        return Path().move(p["fork"]).curve(p["forkCp2"], p["kneeInCp1"], p["floorIn"])

    def outseam():
        return Path().move(p["floorOut"]).curve(p["kneeOutCp2"], p["seatOut"], p.get("styleWaistOut", p["waistOut"]))

    store["inseamBack"] = inseam().length()
    store["outseamBack"] = outseam().length()
    p["styleWaistOut"] = outseam().reverse().shift_along(m["waistToHips"] * (1 - o["waistHeight"]) + wb)
    p["styleWaistIn"] = beams_intersect(p["styleWaistOut"], p["styleWaistOut"].shift(p["waistOut"].angle(p["waistIn"]), 10),
                                        p["waistIn"], p["crossSeamCurveStart"])
    p["styleWaistInNoAngle"] = p["styleWaistIn"].copy()
    return p, store, inseam, outseam


def paco_back(m, o, wb, ankle):
    p, store, _, _ = titan_back(m, o, wb)
    quarter = (m["heel"] * (1 + o["heelEase"]) * o["legBalance"]) / 2
    p["floorOut"] = p["floor"].shift(0, quarter)
    p["floorIn"] = p["floor"].shift(180, quarter)
    p["kneeOut"] = p["knee"].shift(0, quarter)
    p["kneeOutCp2"] = p["kneeOut"].copy()
    p["kneeIn"] = p["knee"].shift(180, quarter)
    p["kneeInCp1"] = p["kneeIn"].copy()
    for k in ("floor", "floorIn", "floorOut"):
        p[k] = p[k].shift(90, ankle)
    d = (m["seat"] * o["legBalance"]) / 2 - p["styleWaistIn"].dist(p["styleWaistOut"])
    angle = p["styleWaistInNoAngle"].angle(p["styleWaistOut"])
    p["styleWaistOut"] = p["styleWaistOut"].shift(angle, d)
    p["seatOut"] = p["seatOut"].shift(angle, d)

    def inseam():
        return Path().move(p["fork"]).curve(p["forkCp2"], p["kneeInCp1"], p["floorIn"])

    def outseam():
        return Path().move(p["floorOut"]).curve(p["kneeOutCp2"], p["seatOut"], p["styleWaistOut"])

    p["styleWaistOut"] = outseam().reverse().shift_along(wb)
    p["styleWaistIn"] = p["styleWaistIn"].shift_towards(p["crossSeamCurveStart"], wb)
    store["pacoInseamBack"] = inseam().length()
    store["pacoOutseamBack"] = outseam().length()
    store["backWaist"] = p["styleWaistIn"].dist(p["styleWaistOut"])
    store["backAnkle"] = p["floorIn"].dist(p["floorOut"])
    edges = {
        "waist": Path().move(p["styleWaistOut"]).line(p["styleWaistIn"]),
        "rise": (Path().move(p["styleWaistIn"]).line(p["crossSeamCurveStart"])
                 .curve(p["crossSeamCurveCp1"], p["crossSeamCurveCp2"], p["fork"])),
        "inseam": inseam(),
        "hem": Path().move(p["floorIn"]).line(p["floorOut"]),
        "outseam": outseam(),
    }
    return p, store, edges


def titan_front(m, o, wb, store):
    p = {}
    p["waistX"] = Point(m["waistFrontArc"] * (1 + o["waistEase"]), 0)
    p["upperLegY"] = Point(0, m["waistToUpperLeg"])
    p["seatX"] = Point(m["seatFrontArc"] * (1 + o["seatEase"]), 0)
    p["seatY"] = Point(0, m["waistToSeat"])
    p["seatOut"] = p["seatY"].copy()
    p["cfSeat"] = Point(p["seatX"].x, p["seatY"].y)
    p["fork"] = Point(m["seatFrontArc"] * (1 + o["seatEase"]) * 1.25, p["upperLegY"].y * (1 + o["crotchDrop"]))
    p["grainlineTop"] = p["upperLegY"].shift_fraction_towards(p["fork"], o["grainlinePosition"])
    p["knee"] = Point(p["grainlineTop"].x, m["waistToKnee"])
    p["floor"] = Point(p["grainlineTop"].x, m["waistToFloor"] * (1 + o["lengthBonus"]))
    half = store["kneeFront"] / 2
    p["kneeOut"] = p["knee"].shift(180, half)
    p["kneeIn"] = p["kneeOut"].flip_x(p["knee"])
    p["floorOut"] = p["floor"].shift(180, half)
    p["floorIn"] = p["floorOut"].flip_x(p["floor"])
    p["kneeInCp2"] = p["kneeIn"].shift(90, p["fork"].dy(p["knee"]) / 3)
    p["kneeOutCp1"] = p["kneeOut"].shift(90, p["fork"].dy(p["knee"]) / 3)
    p["seatOutCp1"] = p["seatOut"].shift(90, m["waistToHips"] * o["waistHeight"] + wb)
    p["seatOutCp2"] = p["seatOut"].shift(-90, p["seatOut"].dy(p["knee"]) / 3)
    width = p["waistX"].x
    p["waistOut"] = Point(p["waistX"].dx(p["cfSeat"]) * o["waistBalance"], 0)
    p["waistIn"] = p["waistOut"].shift(0, width)
    p["cfWaist"] = p["waistIn"].copy()

    def crotch_seam():
        p["crotchSeamCurveStart"] = p["waistIn"].shift_fraction_towards(p["cfSeat"], o["crotchSeamCurveStart"])
        p["crotchSeamCurveMax"] = beams_intersect(p["waistIn"], p["cfSeat"], p["fork"], p["fork"].shift(0, 666))
        p["crotchSeamCurveCp1"] = p["fork"].shift_fraction_towards(p["crotchSeamCurveMax"], o["crotchSeamCurveBend"]).rotate(
            o["crotchSeamCurveAngle"] * -1, p["fork"])
        p["crotchSeamCurveCp2"] = p["crotchSeamCurveStart"].shift_fraction_towards(p["crotchSeamCurveMax"], o["crotchSeamCurveBend"])
        p["forkCp1"] = p["crotchSeamCurveCp1"].rotate(90, p["fork"])

    def crotch_delta():
        return (Path().move(p["waistIn"]).line(p["crotchSeamCurveStart"])
                .curve(p["crotchSeamCurveCp2"], p["crotchSeamCurveCp1"], p["fork"]).length() - m["crossSeamFront"])

    crotch_seam()
    if o["fitCrossSeam"]:
        d = crotch_delta()
        for _ in range(15):
            prev = d
            saved = {k: p[k].copy() for k in ("waistIn", "waistOut", "cfWaist", "fork")}
            for k in ("waistIn", "waistOut", "cfWaist"):
                p[k] = p[k].rotate(d / -15, p["seatOut"])
            p["fork"] = p["fork"].shift(180, d / 5)
            crotch_seam()
            d = crotch_delta()
            if abs(d) > abs(prev):
                p.update(saved)
                crotch_seam()
                break
            if abs(d) <= 1:
                break

    def inseam():
        return Path().move(p["floorIn"]).curve(p["kneeInCp2"], p["forkCp1"], p["fork"])

    def outseam():
        w = p.get("styleWaistOut", p["waistOut"])
        if p["waistOut"].x < p["seatOut"].x:
            return Path().move(w).curve(p["seatOut"], p["kneeOutCp1"], p["floorOut"])
        return Path().move(w)._curve(p["seatOutCp1"], p["seatOut"]).curve(p["seatOutCp2"], p["kneeOutCp1"], p["floorOut"])

    in_delta = lambda: inseam().length() - store["inseamBack"]
    out_delta = lambda: outseam().length() - store["outseamBack"]
    if (in_delta() < 0 and out_delta() < 0) or (in_delta() > 0 and out_delta() > 0):
        for _ in range(10):
            di, do = in_delta(), out_delta()
            d = do if abs(di) > abs(do) else di
            if abs(d) <= 1:
                break
            for k in ("kneeInCp2", "kneeOutCp1", "kneeIn", "kneeOut", "knee", "floorIn", "floorOut", "floor"):
                p[k] = p[k].shift(90, d)

    def adapt(out):
        keys = ["cfSeat", "crotchSeamCurveCp1", "crotchSeamCurveCp2", "crotchSeamCurveStart", "waistIn", "cfWaist",
                "waistOut", "seatOut" if out else "fork"]
        f = out_delta if out else in_delta
        centre = p["fork" if out else "seatOut"].copy()
        for _ in range(20):
            d = f()
            if abs(d) <= 1:
                break
            for k in keys:
                p[k] = p[k].rotate((d / 10) * (1 if out else -1), centre)
        p["forkCp1"] = p["crotchSeamCurveCp1"].rotate(90, p["fork"])

    adapt(True)
    adapt(False)
    adapt(True)
    adapt(False)
    p["styleWaistOut"] = outseam().shift_along(m["waistToHips"] * (1 - o["waistHeight"]) + wb)
    p["styleWaistIn"] = beams_intersect(p["styleWaistOut"], p["styleWaistOut"].shift(p["waistOut"].angle(p["waistIn"]), 10),
                                        p["waistIn"], p["crotchSeamCurveStart"])
    p["styleWaistInNoAngle"] = p["styleWaistIn"].copy()
    return p


def paco_front(m, o, wb, ankle, store):
    p = titan_front(m, o, wb, store)
    quarter = (m["heel"] * (1 + o["heelEase"]) * (1 - o["legBalance"])) / 2
    p["floorOut"] = p["floor"].shift(180, quarter)
    p["floorIn"] = p["floor"].shift(0, quarter)
    p["kneeOut"] = p["knee"].shift(180, quarter)
    p["kneeOutCp1"] = p["kneeOut"].copy()
    p["kneeIn"] = p["knee"].shift(0, quarter)
    p["kneeInCp2"] = p["kneeIn"].copy()
    for k in ("floor", "floorIn", "floorOut"):
        p[k] = p[k].shift(90, ankle)
    d = (m["seat"] * (1 - o["legBalance"])) / 2 - p["styleWaistIn"].dist(p["styleWaistOut"])
    angle = p["styleWaistInNoAngle"].angle(p["styleWaistOut"])
    p["styleWaistOut"] = p["styleWaistOut"].shift(angle, d)
    p["seatOut"] = p["seatOut"].shift(angle, d)

    def inseam():
        return Path().move(p["floorIn"]).curve(p["kneeInCp2"], p["forkCp1"], p["fork"])

    def outseam():
        return Path().move(p["styleWaistOut"]).curve(p["seatOut"], p["kneeOutCp1"], p["floorOut"])

    p["styleWaistOut"] = outseam().shift_along(wb)
    p["styleWaistIn"] = p["styleWaistIn"].shift_towards(p["crotchSeamCurveStart"], wb)
    di = inseam().length() - store["pacoInseamBack"]
    for k in ("floor", "floorIn", "floorOut"):
        p[k] = p[k].shift(90, di)
    do = outseam().length() - store["pacoOutseamBack"]
    p["styleWaistOut"] = p["floorOut"].shift_outwards(p["styleWaistOut"], do * -1)
    store["frontWaist"] = p["styleWaistIn"].dist(p["styleWaistOut"])
    store["frontAnkle"] = p["floorIn"].dist(p["floorOut"])
    edges = {
        "waist": Path().move(p["styleWaistIn"]).line(p["styleWaistOut"]),
        "outseam": outseam(),
        "hem": Path().move(p["floorOut"]).line(p["floorIn"]),
        "inseam": inseam(),
        "rise": (Path().move(p["fork"]).curve(p["crotchSeamCurveCp1"], p["crotchSeamCurveCp2"], p["crotchSeamCurveStart"])
                 .line(p["styleWaistIn"])),
    }
    return p, edges


def draft(m, o=OPTIONS):
    wb = o["waistbandWidth"] * m["waistToFloor"]
    ankle = o["ankleElastic"] * m["waistToFloor"]
    bp, store, back = paco_back(m, o, wb, ankle)
    fp, front = paco_front(m, o, wb, ankle, store)
    return {
        "front": front,
        "back": back,
        "waistband": {"length": 2 * (store["frontWaist"] + store["backWaist"]), "width": wb},
        "cuff": {"length": store["frontAnkle"] + store["backAnkle"], "width": ankle * 1.05},
        "points": {"front": fp, "back": bp},
    }


if __name__ == "__main__":
    sample = {"waist": 841, "seat": 983, "waistFrontArc": 217, "waistBackArc": 203, "seatFrontArc": 232, "seatBackArc": 259,
              "waistToUpperLeg": 276, "waistToSeat": 247, "waistToKnee": 574, "waistToFloor": 1073, "waistToHips": 92,
              "crossSeamFront": 317, "crossSeamBack": 348, "knee": 365, "heel": 320}
    d = draft(sample)
    for piece in ("front", "back"):
        print(piece, {k: round(v.length(), 1) for k, v in d[piece].items()})
    print(d["waistband"], d["cuff"])
