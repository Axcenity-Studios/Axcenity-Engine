"""A groom from guide curves by zone (Character Studio wave 5, decision 14),
for a style modelled from scratch against reference photographs.

The wave-4 groom (`groom.py`) combed every strand from one field and made
one card per strand. This one works as a groomer does in Blender's hair
tools:

1. **Guides by zone.** Guide roots are thrown over the scalp about every
   `guide_spacing`; each guide takes its zone's direction, length, height
   off the scalp and tip behaviour (`zones` in the style: the front, the
   top, the crown's whorl, the sides, over the ears, the back and the
   nape's taper), read from the references. The guides are the style's
   hand-editable data (`build_hair.py --guides-blend` writes them as
   Blender curves).
2. **Clumps.** Clump roots are thrown over the scalp at the layer's
   spacing; each takes the guides' parameters near it (inverse distance,
   the three nearest), varied a little (length, direction, height), and is
   grown over the scalp as its centre strand.
3. **Strands.** A clump's strands sit round its centre at the root,
   converge toward it along their length (the clump's tips come to a
   point, piecey), frizz a little, and end at staggered lengths. They are
   kept in the clump's own frame (across, off the scalp, along), so the
   card made from the clump and the atlas tile drawn from its strands
   agree (`clump_cards.py`).

Deterministic: every draw comes from a generator seeded by the style's
seed and a name.
"""

import math

import numpy as np

from groom import Groom, rotate, seeded, unit
from scalp import smoothstep, smooth_table


class GuideGroom(Groom):
    """A style's groom from its zones: the scalp, hairline and crown come
    from `Groom`; the comb, length and height come from the zones."""

    def __init__(self, body, style):
        super().__init__(body, style)
        self.zones = style["zones"]
        self.whorl_axis = unit(self.crown - self.head.centre)

    # -- the zones ----------------------------------------------------------

    def zone_values(self, p):
        """Length and height off the scalp (metres) at scalp point `p`,
        blended over the head by azimuth (front, side, back) and elevation
        (each a table in the style)."""
        z = self.zones
        phi, theta, _ = self.head.angles(p[None])
        phi, theta = abs(float(phi[0])), float(theta[0])
        values = []
        for key in ("length", "height"):
            front = smooth_table(z[key + "_front"], theta)
            side = smooth_table(z[key + "_side"], theta)
            back = smooth_table(z[key + "_back"], theta)
            if phi <= 90:
                t = smoothstep(25.0, 90.0, phi)
                values.append(float(front * (1 - t) + side * t))
            else:
                t = smoothstep(95.0, 155.0, phi)
                values.append(float(side * (1 - t) + back * t))
        return values

    def comb(self, p, n):
        """The direction hair at `p` (normal `n`) is combed: out of the
        crown's whorl in a clockwise spiral, forward over the top (swept a
        little to one side at the front), down and back at the sides, down
        and a little toward the middle at the nape."""
        z = self.zones
        phi, theta, _ = self.head.angles(p[None])
        phi, theta = float(phi[0]), float(theta[0])
        away = p - self.crown
        away = away - n * (away @ n)
        dist = float(np.linalg.norm(away))
        if dist < 1e-6:
            away = np.array([1.0, 0.0, 0.0]) - n * n[0]
        away = unit(away)
        # The whorl: turned round the crown, most at its centre, none
        # past `whorl_reach` (clockwise seen from above: about -n).
        turn = z["whorl_turn"] * (1.0 - smoothstep(0.0, z["whorl_reach"], dist))
        away = rotate(away, n, -turn)
        # Down at the sides and back, a little backward over the ears and
        # a little in toward the middle at the nape.
        side = smoothstep(40.0, 70.0, abs(phi)) * (1.0 - smoothstep(125.0, 150.0, abs(phi)))
        nape = smoothstep(140.0, 170.0, abs(phi)) * (1.0 - smoothstep(-5.0, 15.0, theta))
        inward = -math.copysign(1.0, phi) * z["nape_in"] * nape
        down = np.array([inward, z["side_back"] * side, -1.0])
        down = unit(down - n * (down @ n))
        g = 1.0 - smoothstep(z["gravity_full"], z["gravity_from"], theta)
        d = unit((1.0 - g) * away + g * down)
        front = (1.0 - smoothstep(25.0, 80.0, abs(phi))) * smoothstep(10.0, 40.0, theta)
        if front > 0:
            d = rotate(d, n, z["sweep"] * front)
        return unit(d - n * (d @ n))

    def tip_lift(self, p):
        """How far a clump's tip lifts (+) or settles (-), as a share of its
        height: the top and the fringe lift (textured, piecey), the sides
        and back settle onto the head."""
        z = self.zones
        phi, theta, _ = self.head.angles(p[None])
        top = smoothstep(z["lift_from"], z["lift_from"] + 20.0, float(theta[0]))
        front = 1.0 - smoothstep(60.0, 110.0, abs(float(phi[0])))
        return float(z["tip_settle"] + top * (0.4 + 0.6 * front) * (z["tip_lift"] - z["tip_settle"]))

    # -- guides -------------------------------------------------------------

    def guides(self):
        """The guide curves: roots about every `guide_spacing` over the
        scalp, each grown with its zone's values (no variation). Returns
        [(points, normals, sides, values)] and the roots."""
        rng = seeded(self.style, "guides")
        roots, normals, density, f = self.roots(self.zones["guide_spacing"], "all", rng)
        out = []
        for p, n in zip(roots, normals):
            length, height = self.zone_values(p)
            pts, nrm, sides = self.grow(p, n, length, height, self.tip_lift(p), 0.0,
                                        overhang=self.zones["overhang_front"], rng=None)
            out.append((pts, nrm, sides, {"length": length, "height": height}))
        return out

    # -- clumps -------------------------------------------------------------

    def grow(self, root, normal, length, height, lift, turn, overhang, rng, segments=None):
        """A clump's centre strand from `root`: walks over the scalp along
        the comb (turned `turn` degrees) for `length`, standing off it by
        up to `height` (rising from the root, its tip lifting or settling by
        `lift`), and stops near an ear or more than `overhang` past the
        hairline. Returns its points, the scalp's normal under each and the
        direction across."""
        count = segments or int(np.clip(round(length / 0.0055), 4, 8))
        step = length / count
        d = rotate(self.comb(root, normal), normal, turn)
        rise = 0.28 if rng is None else rng.uniform(0.22, 0.34)
        points = [root - normal * 0.0003]
        normals = [normal]
        sides = [unit(np.cross(normal, d))]
        place = root
        for k in range(1, count + 1):
            t = k / count
            place_new, n_new = self.project(place + d * step)
            near_ear = np.linalg.norm(self.head.ear_points - place_new, axis=1).min() < self.zones["ear_keep"]
            f = self.hairline.signed(place_new[None])[0]
            if (near_ear or f < -overhang) and k > 2:
                break
            transported = unit(d - n_new * (d @ n_new))
            d = unit(0.7 * transported + 0.3 * self.comb(place_new, n_new))
            place = place_new
            h = height * math.sin(0.5 * math.pi * min(1.0, t / rise))
            if t > 0.5:
                h *= 1.0 + lift * ((t - 0.5) / 0.5) ** 2
            points.append(place + n_new * h)
            normals.append(n_new)
            sides.append(unit(np.cross(n_new, d)))
        return np.array(points), np.array(normals), np.array(sides)

    def clumps(self, layer, level=0):
        """The clumps of one layer (`style["layers"]`): their roots, their
        centre strands and their strand parameters."""
        name = layer["name"]
        rng = seeded(self.style, "clumps-" + name)
        spacing = layer["spacing"] * (layer.get("crowd_spacing", 1.9) if level == 2 else 1.0)
        roots, normals, density, f = self.roots(spacing, layer["where"], rng)
        if "within_crown" in layer:
            # A fill round the whorl, where the clumps fan apart.
            near = np.linalg.norm(roots - self.crown, axis=1) < layer["within_crown"]
            roots, normals, density, f = roots[near], normals[near], density[near], f[near]
        out = []
        for i in range(len(roots)):
            p, n = roots[i], normals[i]
            length, height = self.zone_values(p)
            # Scissor taper: shorter toward the hairline at the sides, back
            # and nape (not the front, which is pushed forward over it).
            phi_here = abs(float(self.head.angles(p[None])[0][0]))
            taper = self.zones["taper"] * smoothstep(40.0, 80.0, phi_here)
            length *= 1.0 - taper * (1.0 - smoothstep(0.0, self.zones["taper_reach"], float(f[i])))
            length *= layer["length"] * rng.uniform(*layer["length_spread"])
            # Lower toward the hairline all round (the front's too): the
            # hair there lies close, so the line shows its ramp.
            low = self.zones["height_at_line"]
            height *= low + (1.0 - low) * smoothstep(0.0, self.zones["height_reach"], float(f[i]))
            height *= layer["height"] * rng.uniform(0.85, 1.15)
            ramp = float(self.hairline.ramp(p[None])[0])
            if name == "hairline":
                # Short and low at the line, longer inside it.
                share = float(np.clip(f[i] / max(ramp, 1e-6), 0.0, 1.0))
                length = float(np.clip(length * (0.35 + 0.5 * share), 0.006, 0.026))
                height = max(0.0015, height * (0.4 + 0.6 * share))
            # Where the tips stop at the hairline: staggered, so no edge of
            # the style is one line.
            front = phi_here < 50.0
            # Behind the temples (over the ears and down the nape) hair
            # grows down toward the line: its fine hairs may hang a little
            # past it and the dense layers come nearer, so the taper is a
            # graded band of short hair, not bare skin under a jagged edge.
            behind = phi_here > self.zones["behind_from"]
            if name in ("hairline", "fly"):
                overhang = rng.uniform(*self.zones["hairline_stop_behind" if behind else "hairline_stop"])
            elif front:
                overhang = rng.uniform(*self.zones["fringe_stop"])
            else:
                # The sides, back and nape end inside the ramp, staggered,
                # so the hairline layer makes the last of it.
                lo, hi = self.zones["side_stop_behind" if behind else "side_stop"]
                overhang = -ramp * rng.uniform(lo, hi)
            turn = rng.normal(0.0, layer.get("jitter", 9.0))
            lift = self.tip_lift(p) * rng.uniform(0.6, 1.4)
            segments = 3 if level == 2 else (layer["segments"] if level == 0 else max(3, layer["segments"] - 2))
            # A clump keeps at least two steps, so one that would cross the
            # hairline in them is grown again shorter until it doesn't.
            for _ in range(6):
                pts, nrm, sides = self.grow(p, n, length, height, lift, turn, overhang, rng, segments=segments)
                if (self.hairline.signed(self.project(pts[-1])[0][None])[0] >= -overhang - 0.0005
                        or length < 0.004):
                    break
                length *= 0.7
            if len(pts) < 3:
                continue
            out.append({
                "root": p, "normal": n, "density": float(density[i]), "inside": float(f[i]),
                "points": pts, "normals": nrm, "sides": sides,
                "width": layer["width"] * rng.uniform(0.85, 1.15),
                "seed": int(rng.integers(1 << 30)),
            })
        return out


def clump_strands(clump, layer, count, rng):
    """The strands of a clump in its own frame: each a polyline of
    (across, off the scalp, along) with `along` 0 at the root and 1 at
    the clump's full length, and a width (hairs). Roots spread over the
    clump's width, staggered a little along; strands converge toward the
    clump's centre toward their tips (`converge`), frizz, and end at
    staggered lengths (`ends`); a few strays leave the clump."""
    out = []
    converge = layer["converge"]
    lo, hi = layer["ends"]
    for _ in range(count):
        a0 = rng.uniform(-0.42, 0.42)
        b0 = rng.uniform(-0.5, 0.5)
        start = rng.uniform(0.0, layer.get("root_stagger", 0.08))
        end = rng.uniform(lo, hi)
        if end - start < 0.15:
            end = min(1.0, start + 0.15)
        stray = rng.random() < layer.get("strays", 0.04)
        t = np.linspace(start, end, 25)
        s = (t - start) / max(end - start, 1e-6)
        pull = converge * smoothstep(0.1, 1.0, s) * (0.6 + 0.4 * rng.random())
        frizz = layer.get("frizz", 0.02)
        phase = rng.uniform(0, 2 * math.pi)
        a = a0 * (1.0 - pull) + frizz * np.sin(2 * math.pi * rng.uniform(1.0, 2.5) * s + phase) * s
        if stray:
            a = a + rng.normal(0.0, 0.18) * s * s
        a = np.clip(a, -0.44, 0.44)
        b = b0 * (1.0 - pull)
        out.append({"across": a, "up": b, "along": t,
                    "width": layer.get("hair", 1.1) * rng.uniform(0.75, 1.25),
                    "taper": layer.get("taper", 0.4)})
    return out
