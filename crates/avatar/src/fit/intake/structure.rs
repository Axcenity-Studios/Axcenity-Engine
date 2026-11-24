//! What a garment (or a body) is made of, read from its cross-sections:
//! a trunk, and the tubes that leave it (sleeves beside it, legs under
//! it), with the heights they part at (the underarm, the crotch).
//!
//! A ring is measured as a tape measure lies round it (its convex hull),
//! so a shirt open down the front, a skirt with a zip slit and a body
//! with a hollow down its spine all have a sensible length round.
//!
//! Nothing here knows the mesh's units: a garment is measured in its own
//! file's, a body in metres, and the two are compared as ratios.

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::slices::{Loop, Slice, hull_area, hull_centre, hull_holds, length_round};

/// A loop counts when it is at least this share of its plane's longest.
const LEAST_SHARE: f32 = 0.12;
/// The trunk ends where its ring takes in the loops beside it and widens
/// by this much, or changes width by [`JUMP`] with nothing beside it (a
/// neck above the shoulders).
const MERGE: f32 = 1.2;
const JUMP: f32 = 1.6;
/// The trunk is the lowest run of rings round the middle at least this
/// share of the mesh's height.
const TRUNK_LEAST: f32 = 0.04;
/// A limb is at least this share of the mesh's height.
const LIMB_LEAST: f32 = 0.03;
/// A limb's loop is at least this round: the area inside its tape over a
/// circle's of the tape's length.
const ROUND: f32 = 0.4;
/// A leg's middle line points at least this much downward (one unit long:
/// within about 30 degrees of straight down).
const HANGS: f32 = 0.85;
/// A ring round the axis reaches at least this share of the mesh's width
/// either side of it (two legs that touch are not one ring).
const REACH: f32 = 0.04;
/// Among the runs of rings the trunk is the one with the most cloth, a
/// tube's rings counting this many times a bent band's.
const TUBE_COUNTS: f32 = 3.0;

/// One loop of a cross-section, as a tape measure lies round it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ring {
    pub y: f32,
    /// Its middle, least and most corner in the plane (x, z).
    pub centre: Vec2,
    pub min: Vec2,
    pub max: Vec2,
    /// How long the tape is.
    pub length: f32,
    /// Whether the loop is a tube's (or a solid's) and not a band of
    /// cloth bent round (a shirt open down the front): its own area is
    /// most of the tape's.
    pub tube: bool,
    /// The tape's length round the hollow inside it (the body's place),
    /// when the cloth is hollow here.
    pub inside: Option<f32>,
}

impl Ring {
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }
}

/// A tube that leaves the trunk: a sleeve or a leg.
#[derive(Clone, Debug)]
pub(crate) struct Limb {
    /// Its middle line's ends: where it leaves the trunk, and its opening.
    pub start: Vec3,
    pub end: Vec3,
    /// How thick it is (twice its surface's usual distance from the
    /// middle line).
    pub width: f32,
}

impl Limb {
    pub fn length(&self) -> f32 {
        self.start.distance(self.end)
    }

    /// One unit along it, from the trunk outward.
    pub fn axis(&self) -> Vec3 {
        (self.end - self.start).normalize_or_zero()
    }

    /// How many times longer than thick it is.
    pub fn slenderness(&self) -> f32 {
        self.length() / self.width.max(1e-6)
    }
}

/// A mesh's trunk and limbs. Side 0 is the one at lower x (a character's
/// right), side 1 the one at higher x (its left).
#[derive(Clone, Debug, Default)]
pub(crate) struct Structure {
    pub bottom: f32,
    pub top: f32,
    /// The line straight up its middle (x, z).
    pub axis: Vec2,
    /// The trunk's rings, lowest first: one ring round the axis in each
    /// plane, from the hem or the crotch up to the underarm.
    pub trunk: Vec<Ring>,
    pub sleeves: [Option<Limb>; 2],
    pub legs: [Option<Limb>; 2],
    /// The mesh's points (welded) on the trunk's rings, and on each
    /// sleeve's and each leg's loops, both walls.
    pub trunk_points: Vec<u32>,
    pub sleeve_points: [Vec<u32>; 2],
    pub leg_points: [Vec<u32>; 2],
}

impl Structure {
    pub fn height(&self) -> f32 {
        self.top - self.bottom
    }

    /// The trunk's lowest and highest ring heights.
    pub fn trunk_low(&self) -> f32 {
        self.trunk.first().map_or(self.bottom, |r| r.y)
    }

    pub fn underarm(&self) -> f32 {
        self.trunk.last().map_or(self.top, |r| r.y)
    }

    /// Where the legs part, when it has two.
    pub fn crotch(&self) -> Option<f32> {
        match &self.legs {
            [Some(_), Some(_)] => Some(self.trunk_low()),
            _ => None,
        }
    }

    /// Whether it goes on above the trunk (shoulders, straps, a collar).
    pub fn has_upper(&self) -> bool {
        self.top - self.underarm() > 0.06 * self.height()
    }

    /// The trunk's ring nearest height `y`.
    pub fn trunk_at(&self, y: f32) -> Option<Ring> {
        self.trunk
            .iter()
            .min_by(|a, b| (a.y - y).abs().total_cmp(&(b.y - y).abs()))
            .copied()
    }

    /// The trunk's longest ring between heights `low` and `high`.
    pub fn widest(&self, low: f32, high: f32) -> Option<Ring> {
        self.trunk
            .iter()
            .filter(|r| r.y >= low && r.y <= high)
            .max_by(|a, b| a.length.total_cmp(&b.length))
            .copied()
    }

    /// The trunk's shortest ring between heights `low` and `high`.
    pub fn narrowest(&self, low: f32, high: f32) -> Option<Ring> {
        self.trunk
            .iter()
            .filter(|r| r.y >= low && r.y <= high)
            .min_by(|a, b| a.length.total_cmp(&b.length))
            .copied()
    }
}

/// A plane's loops that count, sorted out.
struct Plane<'a> {
    y: f32,
    /// The ring round the axis, if one loop is, and its loop.
    middle: Option<Ring>,
    middle_loop: Option<&'a Loop>,
    slice: &'a Slice,
    /// How many loops are round the axis (one inside another is one).
    middles: usize,
    /// The other loops.
    beside: Vec<&'a Loop>,
}

fn planes(slices: &[Slice], axis: Vec2, reach: f32) -> Vec<Plane<'_>> {
    slices
        .iter()
        .map(|slice| {
            let outer = || slice.loops.iter().filter(|l| l.closed && l.depth == 0);
            let longest = outer().map(|l| l.length).fold(0.0, f32::max);
            let mut plane = Plane {
                y: slice.y,
                middle: None,
                middle_loop: None,
                slice,
                middles: 0,
                beside: Vec::new(),
            };
            let mut round: Vec<(f32, Ring, &Loop)> = Vec::new();
            for item in outer().filter(|l| l.length >= LEAST_SHARE * longest) {
                let hull = item.hull();
                let reaches = item.min.x < axis.x - reach && item.max.x > axis.x + reach;
                if reaches && hull_holds(&hull, axis) {
                    let ring = Ring {
                        y: slice.y,
                        centre: hull_centre(&hull),
                        min: item.min,
                        max: item.max,
                        length: length_round(&hull),
                        tube: item.area >= 0.5 * hull_area(&hull),
                        inside: slice
                            .loops
                            .iter()
                            .filter(|l| l.closed && l.depth == item.depth + 1)
                            .filter(|l| l.area >= 0.5 * item.area && item.holds(l.points[0]))
                            .map(|l| length_round(&l.hull()))
                            .reduce(f32::max),
                    };
                    round.push((ring.length, ring, item));
                } else {
                    plane.beside.push(item);
                }
            }
            // Loops round the axis one inside another (both walls of a tube
            // with a slit down it) are one ring: the outermost.
            round.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut kept: Vec<(Ring, &Loop)> = Vec::new();
            for (_, ring, item) in round {
                let inside = kept.iter().any(|(k, _)| {
                    let slack = 0.05 * k.width();
                    ring.min.cmpge(k.min - slack).all() && ring.max.cmple(k.max + slack).all()
                });
                if !inside {
                    kept.push((ring, item));
                }
            }
            plane.middles = kept.len();
            if let [(ring, item)] = kept[..] {
                plane.middle = Some(ring);
                plane.middle_loop = Some(item);
            }
            plane
        })
        .collect()
}

/// The trunk's planes: of the runs of planes with one ring round the
/// axis (a run ends where the ring takes in what was beside it, or jumps
/// in width), the one with the most cloth, never one that began by taking
/// in what was beside it (the shoulders, with the sleeves in them).
fn trunk_of(planes: &[Plane], least: usize) -> Vec<usize> {
    let mut best: (f32, Vec<usize>) = (0.0, Vec::new());
    let mut run: Vec<usize> = Vec::new();
    let mut from_merge = false;
    let mut close = |run: &mut Vec<usize>, from_merge: bool| {
        if run.len() >= least && !from_merge {
            let cloth: f32 = run
                .iter()
                .filter_map(|&k| planes[k].middle)
                .map(|ring| ring.length * if ring.tube { TUBE_COUNTS } else { 1.0 })
                .sum();
            if cloth > best.0 {
                best = (cloth, run.clone());
            }
        }
        run.clear();
    };
    for (k, plane) in planes.iter().enumerate() {
        let mut merged = false;
        let joins = match (plane.middle, run.last()) {
            (Some(ring), Some(&below)) => {
                let was = planes[below].middle.map_or(1e-9, |r| r.width());
                let ratio = ring.width() / was.max(1e-9);
                merged = !planes[below].beside.is_empty() && ratio > MERGE;
                !merged && ratio < JUMP && ratio > 1.0 / JUMP
            }
            _ => false,
        };
        if joins {
            run.push(k);
            continue;
        }
        close(&mut run, from_merge);
        from_merge = merged;
        if plane.middle.is_some() {
            run.push(k);
        }
    }
    close(&mut run, from_merge);
    best.1
}

/// The structure of the mesh `slices` were cut from.
pub(crate) fn structure(slices: &[Slice], points: &[Vec3]) -> Structure {
    let mut out = Structure::default();
    let Some((first, last)) = slices.first().zip(slices.last()) else {
        return out;
    };
    let step = if slices.len() > 1 {
        slices[1].y - slices[0].y
    } else {
        1.0
    };
    out.bottom = first.y - step / 2.0;
    out.top = last.y + step / 2.0;
    let (mut lo, mut hi) = (Vec2::INFINITY, Vec2::NEG_INFINITY);
    for item in slices.iter().flat_map(|s| &s.loops) {
        lo = lo.min(item.min);
        hi = hi.max(item.max);
    }
    let least = ((TRUNK_LEAST * out.height() / step).ceil() as usize).max(2);

    // The axis: the middle of the mesh's box first, then the middle of the
    // trunk that gives (a garment's sleeves or a train pull its box off
    // its trunk).
    let mut axis = (lo + hi) / 2.0;
    let reach = REACH * (hi.x - lo.x);
    let mut sorted = planes(slices, axis, reach);
    let mut trunk = trunk_of(&sorted, least);
    if !trunk.is_empty() {
        let mut xs: Vec<f32> = Vec::new();
        let mut zs: Vec<f32> = Vec::new();
        for &k in &trunk {
            if let Some(ring) = sorted[k].middle {
                xs.push(ring.centre.x);
                zs.push(ring.centre.y);
            }
        }
        xs.sort_by(f32::total_cmp);
        zs.sort_by(f32::total_cmp);
        axis = Vec2::new((lo.x + hi.x) / 2.0, zs[zs.len() / 2]);
        // Across, a garment is the same either side: the box's middle,
        // unless the trunk says it is far off it.
        if (xs[xs.len() / 2] - axis.x).abs() > 0.1 * (hi.x - lo.x) {
            axis.x = xs[xs.len() / 2];
        }
        sorted = planes(slices, axis, reach);
        trunk = trunk_of(&sorted, least);
    }
    out.axis = axis;
    out.trunk = trunk.iter().filter_map(|&k| sorted[k].middle).collect();
    let (Some(&low), Some(&high)) = (trunk.first(), trunk.last()) else {
        return out;
    };

    // Limbs: loops beside the trunk and farther out than it are sleeves;
    // loops under it and within its width are legs.
    let half = |k: usize| {
        let ring = &out.trunk[k.clamp(low, high) - low];
        (ring.max.x - axis.x).max(axis.x - ring.min.x)
    };
    let mut sleeves: [Vec<(f32, &Loop)>; 2] = [Vec::new(), Vec::new()];
    let mut legs: [Vec<(f32, &Loop)>; 2] = [Vec::new(), Vec::new()];
    for &k in &trunk {
        if let Some(item) = sorted[k].middle_loop {
            points_of(sorted[k].slice, item, &mut out.trunk_points);
        }
    }
    for (k, plane) in sorted.iter().enumerate().take(high + 1) {
        // Legs: under the trunk, a plane with nothing round the axis and
        // just two loops, one either side of it, side by side (a curved
        // hem's tails and an open shirt's fronts are more, or lie front
        // and back).
        if k < low
            && plane.middles == 0
            && let [a, b] = plane.beside[..]
        {
            let (a, b) = if a.centre.x <= b.centre.x {
                (a, b)
            } else {
                (b, a)
            };
            let apart = a.max.x <= axis.x + reach && b.min.x >= axis.x - reach;
            let overlap = a.max.y.min(b.max.y) - a.min.y.max(b.min.y);
            let deep = (a.max.y - a.min.y).max(b.max.y - b.min.y);
            let under = (a.centre.x - axis.x).abs() <= 0.9 * half(k)
                && (b.centre.x - axis.x).abs() <= 0.9 * half(k);
            if apart && under && overlap > 0.5 * deep {
                legs[0].push((plane.y, a));
                legs[1].push((plane.y, b));
                points_of(plane.slice, a, &mut out.leg_points[0]);
                points_of(plane.slice, b, &mut out.leg_points[1]);
                continue;
            }
        }
        for item in &plane.beside {
            let side = usize::from(item.centre.x > axis.x);
            if (item.centre.x - axis.x).abs() > 0.9 * half(k) {
                sleeves[side].push((plane.y, item));
                points_of(plane.slice, item, &mut out.sleeve_points[side]);
            }
        }
    }
    let least = LIMB_LEAST * out.height();
    for side in 0..2 {
        out.sleeves[side] = limb(&sleeves[side], least, points);
        out.legs[side] = limb(&legs[side], least, points);
    }
    // Legs come in pairs, and long ones hang down.
    let hangs = |limb: &Option<Limb>| {
        limb.as_ref()
            .is_some_and(|l| l.slenderness() < 1.5 || l.axis().y < -HANGS)
    };
    if !out.legs.iter().all(hangs) {
        out.legs = [None, None];
    }
    for side in 0..2 {
        if out.legs[side].is_none() {
            out.leg_points[side].clear();
        }
        if out.sleeves[side].is_none() {
            out.sleeve_points[side].clear();
        }
    }
    out
}

/// Adds to `into` the mesh points `item` of `slice` runs through, and the
/// ones of the loops inside it (its other wall).
fn points_of(slice: &Slice, item: &Loop, into: &mut Vec<u32>) {
    let mut add = |l: &Loop| into.extend(l.edges.iter().flat_map(|(a, b)| [*a, *b]));
    add(item);
    for other in &slice.loops {
        if other.depth > item.depth && item.holds(other.points[0]) {
            add(other);
        }
    }
}

/// The limb made of `loops` (each with its plane's height), if its round
/// ones span at least `least` of height. It starts at its upper end.
/// `points` are the whole mesh's: where the limb's lower part touches the
/// trunk (a coat's sleeve against its skirt) no plane cuts it as a loop of
/// its own, and it is followed on down its middle line as long as the
/// mesh goes all the way round the line.
fn limb(loops: &[(f32, &Loop)], least: f32, points: &[Vec3]) -> Option<Limb> {
    // Only loops that are a tube's, measured by the tape (a leg with a
    // slit up its side is still one): a short sleeve's lower lip, hanging
    // under the underarm, is cut as a crescent.
    let mut rings: Vec<(Vec3, &Loop)> = loops
        .iter()
        .filter_map(|(y, item)| {
            let hull = item.hull();
            let round = length_round(&hull);
            let centre = hull_centre(&hull);
            (2.0 * TAU * hull_area(&hull) >= ROUND * round * round)
                .then(|| (Vec3::new(centre.x, *y, centre.y), *item))
        })
        .collect();
    rings.sort_by(|a, b| a.0.y.total_cmp(&b.0.y));
    let (lo, hi) = (rings.first()?.0.y, rings.last()?.0.y);
    if rings.len() < 4 || hi - lo < least {
        return None;
    }
    // A level plane cuts a tube in a loop whose middle is on the tube's
    // middle line, however the tube leans: the line runs from the middle
    // of its upper loops to the middle of its lower ones.
    let third = (rings.len() / 3).max(1);
    let middle =
        |part: &[(Vec3, &Loop)]| part.iter().map(|(c, _)| *c).sum::<Vec3>() / part.len() as f32;
    let upper = middle(&rings[rings.len() - third..]);
    let lower = middle(&rings[..third]);
    let axis = (lower - upper).try_normalize()?;
    let own: Vec<Vec3> = rings
        .iter()
        .flat_map(|(c, item)| item.points.iter().map(|p| Vec3::new(p.x, c.y, p.y)))
        .collect();
    let mean = (upper + lower) / 2.0;
    let mut along: Vec<f32> = own.iter().map(|p| (*p - mean).dot(axis)).collect();
    along.sort_by(f32::total_cmp);
    let pick = |share: f32| along[((along.len() - 1) as f32 * share) as usize];
    let spread = own
        .iter()
        .map(|p| {
            let from = *p - mean;
            (from - axis * from.dot(axis)).length_squared()
        })
        .sum::<f32>()
        / own.len() as f32;
    let width = 2.0 * spread.sqrt();
    let start = mean + axis * pick(0.02);
    let mut end = mean + axis * pick(0.98);

    // On down the line while the mesh is all the way round it.
    let step = 0.25 * width;
    let (across, other) = axis.any_orthonormal_pair();
    let mut sectors: Vec<u8> = Vec::new();
    for p in points {
        let from = *p - end;
        let along = from.dot(axis);
        if along < 0.0 || step <= 0.0 {
            continue;
        }
        let radial = from - axis * along;
        let distance = radial.length();
        if distance < 0.25 * width || distance > width {
            continue;
        }
        let bin = (along / step) as usize;
        if bin >= 400 {
            continue;
        }
        if sectors.len() <= bin {
            sectors.resize(bin + 1, 0);
        }
        let angle = radial.dot(other).atan2(radial.dot(across)) + std::f32::consts::PI;
        sectors[bin] |= 1 << ((angle / TAU * 8.0) as usize).min(7);
    }
    let on = sectors.iter().take_while(|s| s.count_ones() >= 7).count();
    end += axis * (on as f32 * step);
    Some(Limb { start, end, width })
}
