"""The crew-neck T-shirt: Teagan's pieces (`teagan.py`) gridded, placed
round the realistic body, sewn and draped (`drape.py`).

Pieces: the front and the back (each cut on the fold, so drawn whole),
two sleeves. Seams: the side seams, the shoulders, the armholes (each
sleeve's cap: its front part to the front's armhole, its back part to the
back's), the sleeves' underarm seams. The neckline, hem and sleeve hems
are left open here and finished as a rib band and turned hems
(`finish.py`).

Blender's axes: Z up, the body facing -Y; a piece's x is the body's x
(+X the body's left), its y runs down from the high point of the
shoulder.
"""

import math

import numpy as np

import teagan
from panels import Panel, chain, mirror, path_points, polyline_length, resample


def pieces(m, spacing, opts=teagan.OPTIONS):
    """The four pieces gridded at about `spacing` mm, and the seams as
    pairs of (piece, segment, reversed) to sew corner for corner."""
    d = teagan.draft(m, opts)
    F = {k: path_points(v) for k, v in d["front"].items()}
    B = {k: path_points(v) for k, v in d["back"].items()}
    S = {k: path_points(v) for k, v in d["sleeve"].items()}
    L = polyline_length
    n = {
        "arm_f": max(2, round(L(F["armhole"]) / spacing)),
        "arm_b": max(2, round(L(B["armhole"]) / spacing)),
        "shoulder": max(1, round(L(F["shoulder"]) / spacing)),
        "neck_f": max(2, round(L(F["neckline"]) / spacing)),
        "neck_b": max(2, round(L(B["neckline"]) / spacing)),
        "rows": max(2, round((L(F["side"]) + L(F["centre"])) / 2 / spacing)),
        "sleeve_rows": max(2, round((L(S["front_underarm"]) + d["points"]["sleeve"]["hemLeft"].y
                                     - min(p[1] for p in S["cap"])) / 2 / spacing)),
    }
    out = {}
    for name, E, neck in (("front", F, "neck_f"), ("back", B, "neck_b")):
        a, sh, ne = n["arm_f" if name == "front" else "arm_b"], n["shoulder"], n[neck]
        top, marks = chain([
            ("L_arm", resample(mirror(E["armhole"]), a)),
            ("L_shoulder", resample(mirror(E["shoulder"]), sh)),
            ("L_neck", resample(mirror(E["neckline"]), ne)),
            ("R_neck", resample(E["neckline"][::-1], ne)),
            ("R_shoulder", resample(E["shoulder"][::-1], sh)),
            ("R_arm", resample(E["armhole"][::-1], a)),
        ])
        cols = len(top) - 1
        hem = E["hem"]
        bottom = resample(np.array([[-hem[-1][0], hem[-1][1]], [hem[-1][0], hem[-1][1]]]), cols)
        right = resample(E["side"], n["rows"])
        left = mirror(right)
        rows = n["rows"]
        segments = {k: [(rows, c) for c in range(s, e + 1)] for k, (s, e) in marks.items()}
        segments["left"] = [(r, 0) for r in range(rows + 1)]
        segments["right"] = [(r, cols) for r in range(rows + 1)]
        segments["hem"] = [(0, c) for c in range(cols + 1)]
        out[name] = Panel(name, (bottom, top, left, right), segments)
    # The sleeve: its cap from the back underarm over the top to the front;
    # split where the front part's length is the front armhole's.
    cap = S["cap"][::-1]
    cap_len = L(cap)
    back_len = cap_len - d["cap_front_length"]
    seg = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(cap, axis=0), axis=1))])
    k = int(np.searchsorted(seg, back_len))
    back_part = np.vstack([cap[:k], cap[k - 1] + (cap[k] - cap[k - 1]) * ((back_len - seg[k - 1]) / (seg[k] - seg[k - 1]))])
    front_part = np.vstack([back_part[-1:], cap[k:]])
    top, marks = chain([("cap_back", resample(back_part, n["arm_b"])),
                        ("cap_front", resample(front_part, n["arm_f"]))])
    cols = len(top) - 1
    rows = n["sleeve_rows"]
    bottom = resample(S["hem"], cols)
    left = resample(S["back_underarm"][::-1], rows)
    right = resample(S["front_underarm"], rows)
    segments = {k: [(rows, c) for c in range(s, e + 1)] for k, (s, e) in marks.items()}
    segments["left"] = [(r, 0) for r in range(rows + 1)]
    segments["right"] = [(r, cols) for r in range(rows + 1)]
    segments["hem"] = [(0, c) for c in range(cols + 1)]
    for side in ("sleeve_left", "sleeve_right"):
        out[side] = Panel(side, (bottom, top, left, right), segments)
    seams = [
        ("front", "right", "back", "right", False),
        ("front", "left", "back", "left", False),
        ("front", "R_shoulder", "back", "R_shoulder", False),
        ("front", "L_shoulder", "back", "L_shoulder", False),
        # The body's left arm (+X) wears `sleeve_left`.
        ("front", "R_arm", "sleeve_left", "cap_front", False),
        ("back", "R_arm", "sleeve_left", "cap_back", True),
        ("front", "L_arm", "sleeve_right", "cap_front", True),
        ("back", "L_arm", "sleeve_right", "cap_back", False),
        ("sleeve_left", "left", "sleeve_left", "right", False),
        ("sleeve_right", "left", "sleeve_right", "right", False),
    ]
    return out, seams, d


def seam_pairs(panels, seams, offsets):
    """Corner pairs to sew, as indices into the joined cloth."""
    pairs = []
    for pa, sa, pb, sb, rev in seams:
        a = panels[pa].segment(sa)
        b = panels[pb].segment(sb)
        if rev:
            b = b[::-1]
        assert len(a) == len(b), (pa, sa, pb, sb, len(a), len(b))
        pairs.extend((offsets[pa] + i, offsets[pb] + j) for i, j in zip(a, b))
    return pairs


# How much farther out than its own girth the torso starts (m).
START_OUT = 0.025


class Placement:
    """Where a piece's pattern corner starts round the body: the front and
    back on an ellipse a little bigger than the chest (its girth the
    pieces' width), drawn in over the shoulders above the armhole; the
    sleeves rolled round each upper arm with the cap's top at the
    shoulder point."""

    def __init__(self, body, draft):
        self.body = body
        lm = body.landmarks
        J = body.joints
        pts = body.trunk(lm["chest_z"])
        a0 = np.abs(pts[:, 0]).max()
        y0, y1 = pts[:, 1].min(), pts[:, 1].max()
        b0 = (y1 - y0) / 2
        self.cy = (y0 + y1) / 2
        front = draft["points"]["front"]
        girth = 4 * front["armhole"].x / 1000
        # Ramanujan's ellipse perimeter, scaled to the girth.
        per = math.pi * (3 * (a0 + b0) - math.sqrt((3 * a0 + b0) * (a0 + 3 * b0)))
        k = girth / per
        # Started a little wider than its girth, so the pieces begin clear
        # of the body and the seams pull them in.
        self.a, self.b = a0 * k + START_OUT, b0 * k + START_OUT
        theta = np.linspace(0, math.pi, 2001)
        xy = np.stack([self.a * np.sin(theta), -self.b * np.cos(theta)], axis=1)
        self.theta = theta
        self.arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(xy, axis=0), axis=1))])
        self.hps_z = lm["hps"][2]
        self.ridge_y = lm["neck_y"]
        self.arm_depth = front["armhole"].y / 1000
        self.shoulder = np.array(lm["shoulder"])
        sl = draft["points"]["sleeve"]
        self.sleeve_top = min(p[1] for p in path_points(draft["sleeve"]["cap"]))
        self.biceps_w = (sl["bicepsRight"].x - sl["bicepsLeft"].x)
        self.hem_w = (sl["hemRight"].x - sl["hemLeft"].x)
        self.hem_y = sl["hemLeft"].y
        self.J = J

    def torso(self, uv, back):
        out = []
        for xp, yp in uv:
            s = abs(xp) / 1000
            th = np.interp(s, self.arc, self.theta)
            x = math.copysign(self.a * math.sin(th), xp)
            y = -self.b * math.cos(th)
            if back:
                y = -y
            y += self.cy
            # Wrapped round the ellipse along its length (no stretch: a
            # piece rolled is as long as it was flat, and the cloth takes
            # its starting shape's lengths as its rest), straight up over
            # the shoulders: the seams pull the tops together there.
            z = self.hps_z - yp / 1000
            out.append((x, y, z))
        return np.array(out)

    def sleeve(self, uv, side):
        s = 1 if side == "left" else -1
        prefix = "Left" if s > 0 else "Right"
        j_sh = self.J[f"{prefix}UpperArm"].copy()
        j_el = self.J[f"{prefix}LowerArm"].copy()
        axis = (j_el - j_sh) / np.linalg.norm(j_el - j_sh)
        up = np.array([0.0, 0.0, 1.0]) - axis[2] * axis
        up /= np.linalg.norm(up)
        front = np.cross(axis, up)
        if front[1] > 0:
            front = -front
        shoulder = self.shoulder * np.array([s, 1, 1])
        t0 = (shoulder - j_sh) @ axis
        # The sleeve rolled to its own width (no wider: the cloth would
        # take the rolled length as its rest and bunch when it relaxes).
        r = self.biceps_w / (2 * math.pi) / 1000
        out = []
        for xs, ys in uv:
            t = t0 + (ys - self.sleeve_top) / 1000
            # Rolled at one radius: across the piece is round the tube.
            alpha = (xs / 1000) / r
            out.append(j_sh + axis * t + r * (math.cos(alpha) * up + math.sin(alpha) * front))
        return np.array(out)

    def place(self, panel):
        uv = panel.corners()
        if panel.name in ("front", "back"):
            return self.torso(uv, panel.name == "back")
        return self.sleeve(uv, "left" if panel.name == "sleeve_left" else "right")
