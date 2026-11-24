"""The realistic head's finer face identity shapes (2026-11-24, Paul:
"certain areas should receive more customizability ... such as allowing
stitch outlining of the shape of the face"): the outline of the face, and
more of the nose, the eyes, the lips, the jaw, the chin and the cheeks.
`face.build` adds them after `Face.identity`'s, so every shape that was
there keeps its place; `faceparams.py` names their parameters.

Each is the +1 end of a parameter, made as `Face.identity`'s are: a smooth
field on the masculine frame's skin (metres, Blender axes: Z up, the face
looks along -Y, the character's left is +X), the same offsets on both
frames. What they hold to:

- **The eyes.** No shape here moves a lid's rims but `Face_Eye_Open_*`,
  so a blink still closes on every other one. `Face_Eye_Open_*` changes
  the opening's height, which a blink cannot follow (a shape adds: the
  closed lids part, or cross, by what the opening changed), so it is
  small (`EYE_OPENS`) and its parameter only narrows the eye
  (`faceparams.ONE_SIDED`): there the closed upper lid lies further over
  the lower one, where on a widened eye the lids stayed apart.
- **The mouth.** The outline's shapes keep off the lips and the skin round
  them as `face.feminine`'s narrowing does; `Face_Shape_Length` and
  `Face_Lip_Protrusion` move the mouth, its bag and the lips as one, and
  the teeth go with them (`MOVE_THE_MOUTH`, `face.build`'s `lifts`).
- **The mesh.** The face's quads are 4 to 5 mm wide: a chin's cleft is
  one column of vertices drawn in, an inner-corner fold three loops of
  the lid's skin.
"""

import numpy as np

from facetopo import SIDES, smoothstep

# How far each shape goes at +1 (metres, or a share where it scales).
FACE_WIDER = 0.075          # of each point's distance from the middle
FACE_LONGER = 0.0065        # the chin down
FACE_LONGER_TOP = 0.0030    # the forehead's top up
TAPER_TOP = 0.035           # the temples and cheekbones wider (heart)
TAPER_JAW = 0.110           # the jaw's sides narrower
TAPER_CHIN = 0.20           # and the chin
ROUND_SIDE = 0.0060
ROUND_CHEEK = 0.0020
ROUND_CORNER = 0.0035
NARROWER = 0.090
ROOT_FORWARD = 0.0045
HUMP_OUT = 0.0042
BRIDGE_WIDENS = 0.46
TIP_WIDENS = 0.50
TIP_ROUNDS = 0.0010
NOSTRIL_FLARES = 0.0038
NOSTRIL_RISES = 0.0036
COLUMELLA_HANGS = 0.0032
# The upper lid's rim goes this share of its way from the closing line
# (`lids.Eye.lid`, negative opens): 2.0 mm at its most and 1.8 mm at the
# eye's middle, of an opening of 11.5 mm. About the least that moves the
# lid 2 mm (what a parameter's end has to: `avatar`'s `realistic_parameters`).
EYE_OPENS = 0.21
EPICANTHUS = {3: 0.25, 4: 0.6, 5: 0.9, 6: 1.0, 7: 0.9, 8: 0.6, 9: 0.3}
EPICANTHUS_DOWN = 0.0036
EPICANTHUS_OVER = 0.0026
EPICANTHUS_FORWARD = 0.0018
LID_FULLER = 0.0031
UNDER_EYE_FULLER = 0.0034
BOW_LOOPS = {1: 0.45, 2: 1.0, 3: 0.4}
BOW_PEAKS = 0.0027
BOW_DIPS = 0.0015
CORNER_RISES = 0.0032
LIPS_FORWARD = 0.0030
JAWLINE_OUT = 0.0022
JAWLINE_UNDER = 0.0030
CHIN_CORNER = (0.0022, 0.0030)   # out, down
CHIN_MIDDLE_UP = 0.0010
CLEFT_DEEP = 0.0030
CHEEK_HOLLOWS = 0.0050
CHEEKBONE_RISES = 0.0036


# The shapes that work the lids: the eye's film and the lashes follow
# them as they follow a blink (`face.build`, `eyes.follow`).
ON_LIDS = {"Face_Eye_Open_Left", "Face_Eye_Open_Right"}
# The shapes that move the mouth whole: the teeth go with its middle.
MOVE_THE_MOUTH = ("Face_Shape_Length", "Face_Lip_Protrusion")


def gauss(distance, radius):
    return np.exp(-(np.asarray(distance) / radius) ** 2)


def to_line(P, a, b):
    """Each point's distance from the segment `a` to `b`."""
    ab = b - a
    t = np.clip((P - a) @ ab / float(ab @ ab), 0.0, 1.0)
    return np.linalg.norm(P - (a + t[:, None] * ab), axis=1)


def to_polyline(P, points):
    return np.min([to_line(P, a, b) for a, b in zip(points[:-1], points[1:])], axis=0)


def sellion(face):
    """The deepest point of the face's middle between the eyes: the
    nose's root."""
    P = face.P
    eye_z = 0.5 * (face.eye[1].c[2] + face.eye[-1].c[2])
    mid = np.where(face.skin & face.topo.is_head & (np.abs(P[:, 0]) < 1e-4)
                   & (P[:, 1] < face.nose_tip[1] + 0.04))[0]
    between = mid[(P[mid, 2] > face.nose_tip[2] + 0.010) & (P[mid, 2] < eye_z + 0.020)]
    return P[between[np.argmax(P[between, 1])]].copy()


def shapes(face):
    """{name: ({vertex: offset}, {})}: the shapes, in the order the body
    carries them."""
    P, N, topo = face.P, face.N, face.topo
    out = {}
    up, back = np.array([0.0, 0.0, 1.0]), np.array([0.0, 1.0, 0.0])
    vec = lambda w, v: np.asarray(w)[:, None] * np.asarray(v, dtype=np.float64)  # noqa: E731
    put = lambda name, field, **kw: out.__setitem__(name, (face._moved(field, **kw), {}))  # noqa: E731
    skin = face.skin.astype(np.float64)
    is_head = topo.is_head.astype(np.float64)
    x, y, z = P[:, 0], P[:, 1], P[:, 2]
    zeros = np.zeros(len(P))
    eye_z = face.eye[1].c[2]
    z_mouth, z_chin, tip = face.stomion[2], face.chin[2], face.nose_tip
    ear_y = 0.5 * (face.ear_at[1][1] + face.ear_at[-1][1])
    sn = face.subnasale()
    off_lips = 1.0 - face.lipness
    not_lid = (~face.lid).astype(np.float64)
    eyes_hold = np.ones(len(P))
    for _, s in SIDES:
        eyes_hold *= 1.0 - face.eye_region(s)
    sideways = lambda amount: np.stack([amount, zeros, zeros], axis=1)  # noqa: E731

    # -- the outline of the face -------------------------------------------
    # What the outline's width moves: the skin and the ears from the
    # forehead's sides down to the jaw, none into the neck, over the top of
    # the head or behind the ears, with the eyes and the mouth held
    # (`Face.feminine`'s narrowing, which this is 1.5 times at +1).
    # (Easing off over six rings of vertices to where the head's skin
    # ends on the neck: a shape stops there, and one that was still whole
    # left a step round the neck.)
    edge = [i for i in topo.head if any(not topo.is_head[j] for j in topo.nbr[i])]
    inside = smoothstep(0.0, 6.0, np.minimum(topo.steps(edge, lambda i: bool(topo.is_head[i]), most=8), 8))
    outline = (face.skin | face.ear).astype(np.float64) * is_head * inside
    neck = smoothstep(z_chin - 0.035, z_chin - 0.015, z)
    behind = smoothstep(ear_y + 0.050, ear_y + 0.012, y)
    top = 1.0 - smoothstep(eye_z + 0.035, eye_z + 0.085, z)
    mouth_hold = smoothstep(0.020, 0.050, face.d_line)
    sides = outline * neck * behind * eyes_hold
    put("Face_Shape_Width", sideways(x * sides * top * mouth_hold * FACE_WIDER), teeth=False)
    # Length: the face under the eyes drawn down, more the lower (the nose
    # a little longer, the upper lip, then the chin by all of it), in
    # front of the ears; the mouth, its bag and the lips go as one, by the
    # move of their middle, and the teeth with them. The forehead's top
    # rises a little (`Face_Forehead_Height`'s field).
    ramp = smoothstep(eye_z - 0.008, z_chin + 0.004, z)
    front = smoothstep(ear_y - 0.005, ear_y - 0.045, y)
    lower = ramp * neck * front * is_head * inside * (~face.ear)
    mouth_one = np.maximum(1.0 - smoothstep(0.014, 0.034, face.d_line), face.lining.astype(np.float64))
    at_mouth = float(smoothstep(eye_z - 0.008, z_chin + 0.004, z_mouth))
    down = lower * (1.0 - mouth_one) + at_mouth * mouth_one
    rise = smoothstep(eye_z + 0.025, face.crown - 0.01, z) * is_head
    put("Face_Shape_Length", vec(down, up * -FACE_LONGER) + vec(rise, up * FACE_LONGER_TOP), teeth=False)
    # Taper (+1 a heart): wider at the temples and cheekbones, narrower
    # down the jaw's sides, and the chin narrower (`Face_Chin_Width`'s
    # field). -1 is a square or pear: the jaw as wide as the cheekbones.
    chin = face.near(face.chin + np.array([0.0, 0.008, -0.004]), 0.026) * off_lips \
        * (face.skin + 0.15 * face.lining)
    height = smoothstep(z_chin - 0.005, eye_z + 0.015, z)
    by_height = TAPER_TOP * height - TAPER_JAW * (1.0 - height)
    put("Face_Shape_Taper", sideways(x * sides * top * mouth_hold * by_height - x * chin * TAPER_CHIN))
    # Roundness: the face's sides out between the cheekbone and the jaw,
    # the soft cheek a little fuller, the jaw's corner sloped. -1 is the
    # angular face: flat sides and a square corner.
    rounder = np.zeros((len(P), 3))
    for _, s in SIDES:
        side_at = face.on_skin(np.array([s * 0.070, ear_y - 0.030, z_mouth + 0.014]))
        cheek_at = face.on_skin(np.array([s * 0.050, face.corner[s][1] + 0.022, z_mouth + 0.012]))
        rounder += (face.near(side_at, 0.034) * skin * off_lips * not_lid * face.side(s) * ROUND_SIDE)[:, None] * N
        rounder += (face.near(cheek_at, 0.026) * skin * off_lips * ROUND_CHEEK)[:, None] * N
        rounder -= face.jaw_corner(s) * ROUND_CORNER
    put("Face_Shape_Round", rounder)
    # Narrow: the soft face between the cheekbones and the chin drawn in,
    # the skull, the ears and the jaw's corners held (a slim face, not a
    # narrow head).
    band = smoothstep(eye_z - 0.004, eye_z - 0.030, z) * smoothstep(z_chin - 0.012, z_chin + 0.014, z)
    before_ears = smoothstep(ear_y + 0.006, ear_y - 0.022, y)
    narrow = skin * is_head * inside * band * before_ears * mouth_hold * eyes_hold
    put("Face_Shape_Narrow", sideways(-x * narrow * NARROWER), teeth=False)

    # -- the nose ----------------------------------------------------------
    root_at = sellion(face)
    dorsum = tip - root_at
    dorsum[0] = 0.0
    dorsum /= np.linalg.norm(dorsum)
    out_of_dorsum = np.array([0.0, -abs(dorsum[2]), -abs(dorsum[1])])   # forward and a little down
    out_of_dorsum = out_of_dorsum - dorsum * (out_of_dorsum @ dorsum)
    out_of_dorsum /= np.linalg.norm(out_of_dorsum)
    if out_of_dorsum[1] > 0:
        out_of_dorsum = -out_of_dorsum
    # The root between the eyes: forward is a high bridge that runs
    # straight from the brow (a Greek nose), back a low one.
    root = face.near(root_at, 0.011) * skin * not_lid * (1.0 - smoothstep(0.006, 0.016, np.abs(x)))
    put("Face_Nose_Root", vec(root, back * -ROOT_FORWARD), teeth=False)
    # The hump: the dorsum out where bone meets cartilage, two fifths of
    # the way from the root to the tip (convex, aquiline); -1 scoops it.
    rhinion = face.on_skin(root_at + 0.42 * (tip - root_at))
    hump = face.near(rhinion, 0.011) * skin * (1.0 - smoothstep(0.005, 0.013, np.abs(x)))
    put("Face_Nose_Hump", vec(hump, out_of_dorsum * HUMP_OUT), teeth=False)
    # The bridge's width: the upper nose spread from the middle
    # (`Face.nose_width`'s field above the tip and the wings).
    wide = face.nose_width(sn)
    upper = smoothstep(tip[2] + 0.006, tip[2] + 0.018, z)
    put("Face_Nose_Bridge_Width", wide * (upper * BRIDGE_WIDENS)[:, None], teeth=False)
    # The tip's width: the lobule spread and a little rounder (bulbous),
    # or pinched; the lip under it holds.
    over_lip = smoothstep(sn[2] - 0.001, sn[2] + 0.003, z)
    lobule = face.near(tip, 0.0125) * skin * over_lip
    put("Face_Nose_Tip_Width", sideways(x * lobule * TIP_WIDENS) + (lobule * TIP_ROUNDS)[:, None] * N, teeth=False)
    # The columella: the column between the nostrils hangs lower (it shows
    # under the wings from the side) or is drawn up; subnasale holds.
    column_at = face.on_skin(0.5 * (sn + tip) + np.array([0.0, 0.002, -0.004]))
    column = face.near(column_at, 0.0075) * skin * (1.0 - smoothstep(0.003, 0.009, np.abs(x))) \
        * smoothstep(0.0015, 0.0065, np.linalg.norm(P - sn, axis=1)) * over_lip
    put("Face_Nose_Columella", vec(column, up * -COLUMELLA_HANGS + back * -0.0008), teeth=False)

    # -- the lips ----------------------------------------------------------
    # The cupid's bow: the upper lip's border up at its two peaks and down
    # between them; -1 is a flat border.
    # (By loop from the lips' line: the border is the second, and the red
    # under it and the skin over it go part of the way.)
    border = np.zeros(len(P))
    for k, share in BOW_LOOPS.items():
        border[face.mouth.upper[k]] = share
    peaks = gauss(np.abs(x) - 0.0055, 0.0040)
    dip = gauss(x, 0.0028)
    put("Face_Lip_Bow", vec(border * skin * (BOW_PEAKS * peaks - BOW_DIPS * dip), up))
    # The mouth forward or back: the lips, the skin round them and the
    # whole of the mouth's bag, with the teeth (as `Face_Mouth_Height`
    # moves them up and down; `face.build` gives the teeth the same move).
    # Moved without the teeth, the lips' edges parted at -1 and showed them.
    mouth_all = np.maximum(gauss(face.d_line, 0.026) * skin, face.lining.astype(np.float64))
    put("Face_Lip_Protrusion", vec(mouth_all, back * -LIPS_FORWARD), teeth=False)

    # -- the jaw and the chin ----------------------------------------------
    # The jawline's definition: the jaw's lower edge out and the skin under
    # it drawn in (sharp), or the edge lost in a full underside (soft).
    jawline = np.zeros((len(P), 3))
    for _, s in SIDES:
        corner, _, body = face.jaw_landmarks(s)
        fore = face.on_skin(np.array([s * 0.020, face.chin[1] + 0.016, z_chin - 0.012]))
        edge = to_polyline(P, [corner, body, fore])
        inward = np.array([-s * 0.010, 0.012, -0.012])
        under = to_polyline(P, [face.on_skin(corner + inward), face.on_skin(body + inward),
                                face.on_skin(fore + np.array([-s * 0.006, 0.016, -0.010]))])
        own = skin * off_lips * face.side(s)
        jawline += (gauss(edge, 0.009) * own * JAWLINE_OUT - gauss(under, 0.013) * own * JAWLINE_UNDER)[:, None] * N
    put("Face_Jaw_Definition", jawline)
    # The chin's shape: its two lower corners out and down and its middle
    # held up (square), or the corners drawn up and in to a point.
    squared = np.zeros((len(P), 3))
    for _, s in SIDES:
        corner_at = face.on_skin(face.chin + np.array([s * 0.017, 0.008, -0.008]))
        w = face.near(corner_at, 0.011) * skin * off_lips * face.side(s, 0.003)
        squared += vec(w, np.array([s * CHIN_CORNER[0], 0.0, -CHIN_CORNER[1]]))
    under_chin = face.on_skin(face.chin + np.array([0.0, 0.006, -0.011]))
    squared += vec(face.near(under_chin, 0.008) * skin * off_lips, up * CHIN_MIDDLE_UP)
    put("Face_Chin_Shape", squared)
    # The cleft: the chin's middle drawn in down its front.
    front_of_chin = face.on_skin(face.chin + np.array([0.0, 0.0, -0.003]))
    cleft = gauss(x, 0.0034) * gauss(z - front_of_chin[2], 0.0085) * face.near(front_of_chin, 0.016) * skin * off_lips
    put("Face_Chin_Cleft", -(cleft * CLEFT_DEEP)[:, None] * N)

    # -- by side: the eyes, the nostrils, the mouth's corners, the cheeks -----
    for side, s in SIDES:
        eye = face.eye[s]
        d = P - eye.c
        along, rise, ahead = d @ eye.axis, d @ eye.up, d @ eye.fwd
        span = eye.t_out - eye.t_in
        u = (along - eye.t_in) / span
        step = np.full(len(P), 99.0)
        upper_lid = np.zeros(len(P), dtype=bool)
        for i in eye.idx:
            step[i] = eye.step[i]
            upper_lid[i] = eye.upper[i]
        in_eye = step < 99.0
        own = skin * face.side(s, 0.004)
        # Openness: the upper lid's rim up (`lids.Eye.lid`, which keeps it
        # on the eyeball and takes the lid's skin with it).
        put(f"Face_Eye_Open_{side}", np.zeros((len(P), 3)) + _offsets(face._lids(s, upper=-EYE_OPENS), len(P)),
            teeth=False)
        # The inner-corner fold (epicanthic): the upper lid's skin at the
        # inner third comes down and across the corner, standing forward
        # of it, and the skin beside the nose is drawn over it. The rims
        # hold.
        by_loop = np.array([EPICANTHUS.get(int(k), 0.0) if k < 99 else 0.0 for k in step])
        inner = 1.0 - smoothstep(0.05, 0.50, u)
        fold = np.zeros((len(P), 3))
        above = in_eye & upper_lid
        beside = in_eye & (u < 0.10)
        fold[above] = (by_loop[above] * inner[above])[:, None] \
            * (-eye.up * EPICANTHUS_DOWN + eye.fwd * EPICANTHUS_FORWARD + eye.axis * 0.4 * EPICANTHUS_OVER)
        medial = by_loop * (1.0 - smoothstep(-0.02, 0.10, u)) * (1.0 - smoothstep(0.0, 0.010, rise - 0.004))
        fold[beside] += medial[beside][:, None] * 0.6 * (eye.axis * EPICANTHUS_OVER + eye.fwd * EPICANTHUS_FORWARD)
        still = [i for i in eye.idx if eye.step[i] <= 2]
        put(f"Face_Eye_Inner_Fold_{side}", topo.smooth(fold, 3, keep=still, among=eye.idx), teeth=False)
        # The upper lid's fullness: the skin between the lid's crease and
        # the brow out (a full, puffy lid) or in (a deep-set one).
        off_rims = smoothstep(4.0, 7.0, step)
        sulcus = gauss((u - 0.52) * span, 0.019) * gauss(rise - 0.0125, 0.0058) * gauss(ahead - eye.r * 0.95, 0.016)
        put(f"Face_Lid_Fullness_{side}", (sulcus * off_rims * own * LID_FULLER)[:, None] * (0.75 * N + 0.25 * eye.fwd),
            teeth=False)
        # Under the eye: the skin below the lower lid out (bags) or in (a
        # hollow down from the inner corner).
        below = gauss((u - 0.42) * span, 0.017) * gauss(rise + 0.0165, 0.0065)
        under_eye = below * smoothstep(4.0, 7.0, step) * own * (ahead > 0)
        put(f"Face_Under_Eye_{side}", (under_eye * UNDER_EYE_FULLER)[:, None] * (0.8 * N - 0.2 * eye.up), teeth=False)
        # The nostril: its wing rounded out from its base (flare), and the
        # wing's base up or down the face.
        lateral = np.array([float(s), 0.0, 0.0])
        wing = face.on_skin(tip + np.array([s * 0.019, 0.016, -0.004]))
        ala = face.on_skin(tip + np.array([s * 0.0185, 0.013, -0.0065]))
        flare = face.near(ala, 0.0078) * skin * face.side(s, 0.004) * over_lip
        put(f"Face_Nostril_Flare_{side}", (flare * NOSTRIL_FLARES)[:, None] * (0.7 * N + 0.3 * lateral), teeth=False)
        base = face.near(wing, 0.011) * skin * face.side(s, 0.004)
        put(f"Face_Nostril_Height_{side}", vec(base, up * NOSTRIL_RISES), teeth=False)
        # The mouth's corner up (or down), the lips following along their
        # length and the skin round the corner by distance.
        put(f"Face_Mouth_Corner_{side}",
            face.corner_pull(s, (0.0, 0.12, 1.0), CORNER_RISES, radius=0.015, start=0.15, power=1.5),
            slide=face.lip_slide())
        # The cheek's hollow under the cheekbone (buccal), and how high the
        # cheekbone sits.
        bone_at = face.cheek_point(s)
        cheek_at = face.on_skin(np.array([s * 0.050, face.corner[s][1] + 0.022, z_mouth + 0.012]))
        hollow_at = face.on_skin(0.5 * (bone_at + cheek_at) + np.array([s * 0.008, 0.010, 0.0]))
        put(f"Face_Cheek_Hollow_{side}",
            -(face.near(hollow_at, 0.0155) * skin * off_lips * not_lid * CHEEK_HOLLOWS)[:, None] * N)
        put(f"Face_Cheekbone_Height_{side}", vec(face.near(bone_at, 0.024) * skin * not_lid, up * CHEEKBONE_RISES),
            teeth=False)
    return out


def _offsets(shape, count):
    """A shape ({vertex: offset}) as offsets for every vertex."""
    out = np.zeros((count, 3))
    for i, d in shape.items():
        out[i] = d
    return out
