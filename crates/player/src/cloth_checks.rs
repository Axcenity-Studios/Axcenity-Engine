//! Cloth-only defects (CO9 part 4, wave 5 stage 3;
//! `docs/development/character-studio/asset-quality.md` §3 "Cloth in
//! motion"; `refinement/wave5/fit-drape.md`). The fit gate
//! (`fit_gate_tests`) counts skin through a garment; it can't see cloth
//! that tears, folds over itself or hangs off the body while no skin comes
//! out. This check measures each garment's triangles in a pose against the
//! same garment on the same body at rest, three ways:
//!
//! - **Stretched**: a triangle with an edge longer than [`STRETCH`] times
//!   its rest length. Garment fabric can't stretch that far in wear (the
//!   limit's sources are in asset-quality.md §3), so such a triangle is
//!   mesh separation: the strips under a sleeve, a spike, a flap pulled
//!   out.
//! - **Folded**: a triangle whose posed face turned past perpendicular
//!   from where the body carries it (its corners' skinned normals, which
//!   turn with the bones), while at rest it agreed with them: cloth folded
//!   over, seen from the wrong side.
//! - **Hanging**: a triangle farther outside the body than the item's
//!   declared ease ([`EASE`]), measured from the convex hull of the body
//!   part nearest it ([`Hulls`]): cloth draped across a hollow (under the
//!   bust, the small of the back, the cleft) is inside the hull and
//!   resting; a tab, a strip or a flap off the body is outside it.
//!
//! Each garment and pose keeps a record of what a draft still has: the
//! bodies with any, the triangles on all of them, and the worst. Records
//! are exact and only go down ([`check_cloth`]).

use std::collections::BTreeMap;
use std::path::PathBuf;

use axcenity_assets::Assets;
use axcenity_avatar::{Description, WornItem};
use axcenity_core::{ClassName, InstanceId, Value, World};
use axcenity_render::glam::Vec3;

use super::character_capture::{HeldPose, Stage, Stance, repository_library, stage_world};
use super::fit_gate_tests::{
    Posed, TEE as TEST_TEE, WALK, WALK_FRAMES, benchmarks, bodies, gate_library, is_footwear,
    pose_name, undressed, worn_in,
};

/// The most a garment's triangle edge may stretch in a pose: 1.5 times
/// its rest length (50%). Power-stretch fabric (elastane knits) takes 30
/// to 50% and comfort-stretch 20 to 30% (Guru, Varshney and Kumar 2022);
/// single jersey is a moderate-stretch knit and woven cloth far less. No
/// garment we make goes past 50% in wear: past it is a tear, not cloth.
pub(super) const STRETCH: f32 = 1.5;

/// A rest edge shorter than this is taken as this long (a turned hem's
/// fold is 1 to 5 mm; a zero-length edge pulled open is still a tear).
const SHORTEST: f32 = 1e-4;

/// How well a triangle's face must agree with its corners' normals at rest
/// (the cosine) for its facing to be known.
const ORIENTED: f32 = 0.5;

/// The cosine under which a posed face has turned over: past
/// perpendicular to where its corners' skinned normals point.
pub(super) const FOLDED: f32 = 0.0;

/// How many directions spread over the sphere [`Hulls`] measures a body
/// part's hull along, besides its own skin's normals (at most
/// [`SKIN_DIRECTIONS`] of them).
const DIRECTIONS: usize = 128;
const SKIN_DIRECTIONS: usize = 384;

/// The size of the cells [`Hulls`] keeps the body's corners in, to find
/// the nearest quickly.
const CELL: f32 = 0.02;

/// Each garment's declared ease for the hanging measure: how far outside
/// the body's hull its cloth may stand where it rests on the body
/// (metres; asset-quality.md §3). A regular-fit tee: FreeSewing Teagan's
/// hip ease, 18% of a 105 cm hip girth spread round it
/// (`clothing/teagan.py`), 30 mm. Joggers: Paco's seat ease, 5% of a 105
/// cm seat, 8 mm (`clothing/paco.py`). Cargo pants, a loose cut: 25 mm.
/// Underwear sits on the skin: 5 mm (its rims and the bra's band).
pub(super) const EASE: [(&str, f32); 10] = [
    ("TestTee", 0.030),
    ("Shirt_TeeCrew", 0.030),
    ("Shirt_CrewTee", 0.030),
    ("Shirt_Tee", 0.030),
    ("Pants_Joggers", 0.008),
    ("Pants_Cargo", 0.025),
    ("Underwear_Bra", 0.005),
    ("Underwear_BoxerBriefs", 0.005),
    ("Underwear_Briefs", 0.005),
    ("Underwear_Top", 0.005),
];

/// The declared ease of `garment` ([`EASE`]); every garment checked has
/// one.
fn ease_of(garment: &str) -> f32 {
    EASE.iter()
        .find(|(name, _)| *name == garment)
        .map(|(_, ease)| *ease)
        .unwrap_or_else(|| panic!("{garment} declares no ease: add it to `EASE`"))
}

/// One kind of cloth defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Kind {
    Stretched,
    Folded,
    Hanging,
}

impl Kind {
    /// How far under a record's worst a measure may fall before the record
    /// must come down (in the kind's unit: times rest, degrees, metres).
    fn slack(self) -> f32 {
        match self {
            Kind::Stretched => 0.01,
            Kind::Folded => 0.5,
            Kind::Hanging => 0.0005,
        }
    }

    /// The worst as printed.
    fn show(self, worst: f32) -> String {
        match self {
            Kind::Stretched => format!("{worst:.2} times rest"),
            Kind::Folded => format!("{worst:.1} degrees"),
            Kind::Hanging => format!("{:.1} mm", worst * 1000.0),
        }
    }
}

/// The face of `t` over `positions` (twice its area long).
fn face(positions: &[Vec3], t: [u32; 3]) -> Vec3 {
    let [a, b, c] = t.map(|v| positions[v as usize]);
    (b - a).cross(c - a)
}

/// The triangles of `posed` (the garment in a pose) with an edge longer
/// than [`STRETCH`] times the same edge in `rest` (the garment at rest on
/// the same body): (triangle, the most an edge of it stretched).
pub(super) fn stretched(rest: &Posed, posed: &Posed) -> Vec<(u32, f32)> {
    let mut out = Vec::new();
    for (k, t) in rest.triangles.iter().enumerate() {
        let mut most = 0.0f32;
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let (a, b) = (a as usize, b as usize);
            let was = rest.positions[a].distance(rest.positions[b]).max(SHORTEST);
            let now = posed.positions[a].distance(posed.positions[b]);
            most = most.max(now / was);
        }
        if most > STRETCH {
            out.push((k as u32, most));
        }
    }
    out
}

/// The triangles of `posed` folded over: at rest their face agreed with
/// their corners' normals ([`ORIENTED`]); now it has turned past
/// perpendicular to the corners' posed normals ([`FOLDED`]), which the
/// skinning turned with the bones. (triangle, degrees it turned from
/// them).
pub(super) fn folded(rest: &Posed, posed: &Posed) -> Vec<(u32, f32)> {
    // The triangle's face against the sum of its corners' normals, both
    // made unit long (none when either has no length).
    let agreement = |mesh: &Posed, t: [u32; 3]| {
        let f = face(&mesh.positions, t).normalize_or_zero();
        let n = t
            .iter()
            .map(|v| mesh.normals[*v as usize])
            .sum::<Vec3>()
            .normalize_or_zero();
        (f != Vec3::ZERO && n != Vec3::ZERO).then(|| f.dot(n))
    };
    let mut out = Vec::new();
    for (k, t) in rest.triangles.iter().enumerate() {
        if agreement(rest, *t).is_none_or(|a| a < ORIENTED) {
            continue;
        }
        if let Some(now) = agreement(posed, *t)
            && now < FOLDED
        {
            out.push((k as u32, now.clamp(-1.0, 1.0).acos().to_degrees()));
        }
    }
    out
}

/// The body's parts (torso, head, each upper arm, forearm, hand, thigh,
/// shin and foot) and each joint between two of them (the shoulder and
/// the hip, the elbow, the knee, the wrist, the ankle, the neck) as convex
/// hulls, to say how far cloth is outside the body where it can't be
/// resting on it ([`hanging`]): cloth across a hollow of a part, or across
/// a joint's crease (a gusset over the crotch, cloth over the armpit),
/// rests on the body; a flap or a strip off it doesn't.
///
/// A hull is kept as its support (how far its points reach) along many
/// directions: its skin's own normals, which are the hull's where the
/// hull touches the skin, and directions spread over the sphere. The
/// distance outside along the best of them is a lower bound of the true
/// distance outside the hull, close where the hull's face is near one of
/// them. The distance to the nearest body corner is an upper bound (every
/// corner is in its part's hull), and it is cheap: most cloth is within
/// its ease of the skin, so the hulls are made only when some isn't.
pub(super) struct Hulls {
    points: Vec<Vec3>,
    normals: Vec<Vec3>,
    part: Vec<u8>,
    hulls: std::sync::OnceLock<Vec<Hull>>,
    /// The body's corners by cell ([`CELL`]).
    cells: std::collections::HashMap<[i32; 3], Vec<Vec3>>,
}

struct Hull {
    lo: Vec3,
    hi: Vec3,
    directions: Vec<Vec3>,
    /// The farthest the part's points reach along each direction.
    support: Vec<f32>,
}

fn cell_of(p: Vec3) -> [i32; 3] {
    (p / CELL).floor().as_ivec3().to_array()
}

/// The pairs of parts ([`part_of_bone`]'s numbers) that meet at a joint.
fn joints() -> Vec<(u8, u8)> {
    let mut out = vec![(0, 1)];
    for side in 0..2u8 {
        let base = 2 + side * 6;
        let (hand, forearm, upper_arm) = (base, base + 1, base + 2);
        let (foot, shin, thigh) = (base + 3, base + 4, base + 5);
        out.extend([
            (0, upper_arm),
            (upper_arm, forearm),
            (forearm, hand),
            (0, thigh),
            (thigh, shin),
            (shin, foot),
        ]);
    }
    out
}

impl Hulls {
    /// The hulls of `points` (with their `normals`), each in the part
    /// `part` gives it.
    pub fn new(points: &[Vec3], normals: &[Vec3], part: &[u8]) -> Hulls {
        let mut cells: std::collections::HashMap<[i32; 3], Vec<Vec3>> =
            std::collections::HashMap::new();
        for p in points {
            cells.entry(cell_of(*p)).or_default().push(*p);
        }
        Hulls {
            points: points.to_vec(),
            normals: normals.to_vec(),
            part: part.to_vec(),
            hulls: std::sync::OnceLock::new(),
            cells,
        }
    }

    /// Each part's hull, and each joint's (its two parts'), made once. All
    /// share one set of directions: those spread over the sphere and the
    /// body's own skin normals (every so many corners, at most
    /// [`SKIN_DIRECTIONS`]), so a joint's support along each is the larger
    /// of its two parts'.
    fn hulls(&self) -> &[Hull] {
        self.hulls.get_or_init(|| {
            // Directions spread evenly over the sphere (a Fibonacci
            // lattice).
            let golden = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
            let mut directions: Vec<Vec3> = (0..DIRECTIONS)
                .map(|i| {
                    let y = 1.0 - 2.0 * (i as f32 + 0.5) / DIRECTIONS as f32;
                    let r = (1.0 - y * y).max(0.0).sqrt();
                    let a = golden * i as f32;
                    Vec3::new(r * a.cos(), y, r * a.sin())
                })
                .collect();
            let step = self.points.len().div_ceil(SKIN_DIRECTIONS).max(1);
            directions.extend(
                self.normals
                    .iter()
                    .step_by(step)
                    .map(|n| n.normalize_or_zero())
                    .filter(|n| *n != Vec3::ZERO),
            );
            let count = self
                .part
                .iter()
                .copied()
                .max()
                .map_or(0, |m| usize::from(m) + 1);
            let mut parts: Vec<Option<Hull>> = (0..count).map(|_| None).collect();
            for (p, k) in self.points.iter().zip(&self.part) {
                let hull = parts[usize::from(*k)].get_or_insert_with(|| Hull {
                    lo: Vec3::INFINITY,
                    hi: Vec3::NEG_INFINITY,
                    directions: directions.clone(),
                    support: vec![f32::NEG_INFINITY; directions.len()],
                });
                hull.lo = hull.lo.min(*p);
                hull.hi = hull.hi.max(*p);
                for (s, d) in hull.support.iter_mut().zip(&directions) {
                    *s = s.max(d.dot(*p));
                }
            }
            let mut out: Vec<Hull> = Vec::new();
            for (a, b) in joints() {
                let (Some(Some(ha)), Some(Some(hb))) =
                    (parts.get(usize::from(a)), parts.get(usize::from(b)))
                else {
                    continue;
                };
                out.push(Hull {
                    lo: ha.lo.min(hb.lo),
                    hi: ha.hi.max(hb.hi),
                    directions: directions.clone(),
                    support: ha
                        .support
                        .iter()
                        .zip(&hb.support)
                        .map(|(x, y)| x.max(*y))
                        .collect(),
                });
            }
            out.extend(parts.into_iter().flatten());
            out
        })
    }

    /// Whether a body corner is within `reach` of `p`.
    fn near(&self, p: Vec3, reach: f32) -> bool {
        let lo = cell_of(p - Vec3::splat(reach));
        let hi = cell_of(p + Vec3::splat(reach));
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                for z in lo[2]..=hi[2] {
                    if let Some(list) = self.cells.get(&[x, y, z])
                        && list.iter().any(|q| q.distance_squared(p) <= reach * reach)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// How far `p` is outside the nearest hull (negative inside one): a
    /// lower bound, measured along each hull's directions.
    pub fn outside(&self, p: Vec3) -> f32 {
        // Nearest box first: a hull is inside its box, so a hull whose box
        // is farther than the best found so far can't be nearer.
        let mut order: Vec<(f32, &Hull)> = self
            .hulls()
            .iter()
            .map(|hull| {
                let gap = (hull.lo - p).max(p - hull.hi).max(Vec3::ZERO);
                (gap.length(), hull)
            })
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut best = f32::INFINITY;
        for (gap, hull) in order {
            if gap >= best {
                break;
            }
            let out = hull
                .directions
                .iter()
                .zip(&hull.support)
                .map(|(d, s)| d.dot(p) - s)
                .fold(f32::NEG_INFINITY, f32::max);
            best = best.min(out);
        }
        best
    }

    /// How far `p` is outside the body's hulls when that is more than
    /// `ease`: `None` when a body corner is within `ease` of it, or the
    /// hulls' measure is no more than `ease`.
    pub fn hangs(&self, p: Vec3, ease: f32) -> Option<f32> {
        if self.near(p, ease) {
            return None;
        }
        let out = self.outside(p);
        (out > ease).then_some(out)
    }
}

/// The triangles of `posed` hanging off the body: more than `ease`
/// outside the body's hulls at their middle or a corner (a strip's middle,
/// a flap's end), or at a hem pulled into a point ([`hem_points`]):
/// (triangle, the farthest of those).
pub(super) fn hanging(rest: &Posed, posed: &Posed, hulls: &Hulls, ease: f32) -> Vec<(u32, f32)> {
    let mut points = vec![0.0f32; posed.positions.len()];
    for (corner, drop) in hem_points(rest, posed, ease) {
        points[corner as usize] = drop;
    }
    // Each corner measured once (0: within the ease).
    let mut corner: Vec<Option<f32>> = vec![None; posed.positions.len()];
    let mut out = Vec::new();
    for (k, t) in posed.triangles.iter().enumerate() {
        let middle = t.iter().map(|v| posed.positions[*v as usize]).sum::<Vec3>() / 3.0;
        let mut most = hulls.hangs(middle, ease).unwrap_or(0.0);
        for v in t {
            most = most.max(points[*v as usize]);
            let c = corner[*v as usize].get_or_insert_with(|| {
                hulls
                    .hangs(posed.positions[*v as usize], ease)
                    .unwrap_or(0.0)
            });
            most = most.max(*c);
        }
        if most > ease {
            out.push((k as u32, most));
        }
    }
    out
}

/// How far along a hem, each way, [`hem_points`] looks for the hem's line
/// (at rest, along the hem).
const HEM_SPAN: f32 = 0.10;

/// The farthest below a hem's edge the cloth under it may reach for it to
/// be a hem (its fold, or the hem drawn down into a point): from a farther
/// edge the cloth goes on down a panel, so it is a garment's top edge.
const HEM_REACH: f32 = 0.10;

/// How far to the side of a hem's corner (across its plane) the cloth
/// below it is looked for.
const HEM_ACROSS: f32 = 0.015;

/// How near level a hem's plane must be for [`hem_points`] (the cosine of
/// its normal's angle from up: 25 degrees; a brief's leg opening, rising
/// from the crotch to the hip, is steeper).
const LEVEL: f32 = 0.906;

/// How far above a garment's lowest level hem another may be and still be
/// one of its lowest (a pant's two cuffs, a tee's waist hem).
const LOWEST_HEMS: f32 = 0.05;

/// Corners of `posed`'s hems (its open edges) pulled down into a point:
/// each of its lowest hems that lies level (its plane, fitted to it in the
/// pose, within [`LEVEL`] of horizontal: a top's waist hem, a pant leg's
/// cuff) has its corners measured off that plane, and a corner
/// (the lowest cloth below it: a turned hem's fold) more than `ease` below the
/// line its hem makes [`HEM_SPAN`] away on
/// either side (the mean of the two) is a point: the hem drawn down into a
/// tab. A hem that curves evenly has none; a seam's open edge (an armhole)
/// or a raised sleeve's end stands upright and isn't measured. (corner,
/// how far below.)
pub(super) fn hem_points(rest: &Posed, posed: &Posed, ease: f32) -> Vec<(u32, f32)> {
    use std::collections::HashMap;
    // Corners at the same place are one.
    let mut first: HashMap<[i64; 3], u32> = HashMap::new();
    let weld: Vec<u32> = rest
        .positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            *first
                .entry((*p * 1e5).round().as_i64vec3().to_array())
                .or_insert(k as u32)
        })
        .collect();
    let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
    // Each welded corner's neighbours.
    let mut around: Vec<Vec<u32>> = vec![Vec::new(); rest.positions.len()];
    for t in &rest.triangles {
        let t = t.map(|v| weld[v as usize]);
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if a != b {
                *uses.entry((a.min(b), a.max(b))).or_default() += 1;
                around[a as usize].push(b);
                around[b as usize].push(a);
            }
        }
    }
    let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
    for ((a, b), n) in uses {
        if n == 1 {
            next.entry(a).or_default().push(b);
            next.entry(b).or_default().push(a);
        }
    }
    for list in next.values_mut() {
        list.sort_unstable();
    }
    // Walk each open edge loop.
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut starts: Vec<u32> = next.keys().copied().collect();
    starts.sort_unstable();
    // Each level hem: its corners in order, a loop or not, its plane.
    let mut hems = Vec::new();
    for start in starts {
        if seen.contains(&start) {
            continue;
        }
        let mut ring = vec![start];
        seen.insert(start);
        let mut at = start;
        while let Some(&step) = next[&at].iter().find(|v| !seen.contains(v)) {
            seen.insert(step);
            ring.push(step);
            at = step;
        }
        let closed = next[&at].contains(&start);
        if ring.len() < 6 {
            continue;
        }
        if let Some((middle, normal)) = level_plane(posed, &ring) {
            hems.push((ring, closed, middle, normal));
        }
    }
    // The garment's lowest hems (a top's waist hem, a pant's two cuffs):
    // a neckline, an armhole or a waistband isn't one.
    let lowest = hems
        .iter()
        .map(|(_, _, middle, _)| middle.y)
        .fold(f32::INFINITY, f32::min);
    let mut out = Vec::new();
    for (ring, closed, middle, normal) in &hems {
        if middle.y <= lowest + LOWEST_HEMS {
            out.extend(points_of_ring(
                rest, posed, &around, ring, *closed, *middle, *normal, ease,
            ));
        }
    }
    // Every corner at a point's place.
    let drops: HashMap<u32, f32> = out.into_iter().collect();
    (0..weld.len())
        .filter_map(|k| drops.get(&weld[k]).map(|d| (k as u32, *d)))
        .collect()
}

/// The plane `ring` (corners of `posed`) lies in, when it lies level
/// (within [`LEVEL`]): its middle and its normal, pointing up. The plane
/// is across the ring's least spread: the covariance's smallest axis, by
/// power iteration on the trace less the covariance.
fn level_plane(posed: &Posed, ring: &[u32]) -> Option<(Vec3, Vec3)> {
    let n = ring.len();
    let at = |k: usize| posed.positions[ring[k] as usize];
    let middle = (0..n).map(at).sum::<Vec3>() / n as f32;
    let mut c = [Vec3::ZERO; 3];
    for k in 0..n {
        let d = at(k) - middle;
        c[0] += d * d.x;
        c[1] += d * d.y;
        c[2] += d * d.z;
    }
    let cov = axcenity_render::glam::Mat3::from_cols(c[0], c[1], c[2]);
    let trace = c[0].x + c[1].y + c[2].z;
    let flipped = axcenity_render::glam::Mat3::from_diagonal(Vec3::splat(trace)) - cov;
    let mut normal = Vec3::new(0.3, 0.9, 0.2).normalize();
    for _ in 0..64 {
        normal = (flipped * normal).normalize_or_zero();
        if normal == Vec3::ZERO {
            return None;
        }
    }
    (normal.y.abs() >= LEVEL).then(|| (middle, normal * normal.y.signum()))
}

/// [`hem_points`] on one hem: `ring` its corners in order (a loop when
/// `closed`), in the plane through `middle` across `normal` (up).
#[allow(clippy::too_many_arguments)]
fn points_of_ring(
    rest: &Posed,
    posed: &Posed,
    around: &[Vec<u32>],
    ring: &[u32],
    closed: bool,
    middle: Vec3,
    normal: Vec3,
    ease: f32,
) -> Vec<(u32, f32)> {
    let n = ring.len();
    // How low the cloth goes at each corner of the hem: the lowest reached
    // from it by stepping only down, within [`HEM_ACROSS`] of straight
    // under it (a turned hem's open edge is the top of its turn-up, inside;
    // its fold is the lowest cloth below it, and the panel past the fold
    // goes up again).
    let height = |v: u32| (posed.positions[v as usize] - middle).dot(normal);
    let off = ring
        .iter()
        .map(|&start| {
            let mut lowest = height(start);
            let mut stack = vec![start];
            let mut seen = std::collections::HashSet::from([start]);
            while let Some(v) = stack.pop() {
                for &w in &around[v as usize] {
                    let d = posed.positions[w as usize] - posed.positions[start as usize];
                    let across = (d - normal * d.dot(normal)).length();
                    if height(w) < height(v) && across <= HEM_ACROSS && seen.insert(w) {
                        lowest = lowest.min(height(w));
                        stack.push(w);
                    }
                }
                // Down a panel, not a hem: this edge is a garment's top (a
                // neckline, a waistband).
                if height(start) - lowest > HEM_REACH {
                    return None;
                }
            }
            Some(lowest)
        })
        .collect::<Option<Vec<f32>>>();
    let Some(off) = off else {
        return Vec::new();
    };
    // Where along the hem each corner is, at rest.
    let mut along = vec![0.0f32; n];
    for k in 1..n {
        along[k] = along[k - 1]
            + rest.positions[ring[k] as usize].distance(rest.positions[ring[k - 1] as usize]);
    }
    let length = if closed {
        along[n - 1]
            + rest.positions[ring[0] as usize].distance(rest.positions[ring[n - 1] as usize])
    } else {
        along[n - 1]
    };
    if length < 4.0 * HEM_SPAN {
        return Vec::new();
    }
    // How far off the plane the hem is at `s` along it (between corners).
    let off_at = |s: f32| -> Option<f32> {
        let s = if closed { s.rem_euclid(length) } else { s };
        if !(0.0..=along[n - 1]).contains(&s) {
            if closed {
                // Between the last corner and the first.
                let gap = length - along[n - 1];
                let t = ((s - along[n - 1]) / gap.max(1e-9)).clamp(0.0, 1.0);
                return Some(off[n - 1] + (off[0] - off[n - 1]) * t);
            }
            return None;
        }
        let k = along.partition_point(|a| *a <= s).clamp(1, n - 1);
        let t = ((s - along[k - 1]) / (along[k] - along[k - 1]).max(1e-9)).clamp(0.0, 1.0);
        Some(off[k - 1] + (off[k] - off[k - 1]) * t)
    };
    let mut out = Vec::new();
    for k in 0..n {
        let (Some(before), Some(after)) =
            (off_at(along[k] - HEM_SPAN), off_at(along[k] + HEM_SPAN))
        else {
            continue;
        };
        let point = (before + after) / 2.0 - off[k];
        if point > ease {
            out.push((ring[k], point));
        }
    }
    out
}

/// Which part of the body a bone's skin is (`Hulls`): by the bone's name
/// (the rigs' humanoid names: `LeftUpperArm`, `RightLowerLeg`, fingers
/// `LeftIndexProximal`, ...). Everything else (hips, spine, chest,
/// shoulders, neck) is the torso.
fn part_of_bone(name: &str) -> u8 {
    let side = if name.starts_with("Left") {
        0
    } else if name.starts_with("Right") {
        1
    } else {
        2
    };
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    let limb = if has(&[
        "Hand", "Thumb", "Index", "Middle", "Ring", "Little", "Pinky",
    ]) {
        1
    } else if has(&["LowerArm", "Forearm", "ForeArm"]) {
        2
    } else if has(&["UpperArm"]) {
        3
    } else if has(&["Foot", "Toe"]) {
        4
    } else if has(&["LowerLeg"]) {
        5
    } else if has(&["UpperLeg"]) {
        6
    } else if has(&["Head", "Jaw", "Eye"]) {
        return 1;
    } else {
        return 0;
    };
    2 + side * 6 + (limb - 1)
}

/// Each corner of the body part `body`'s model (as `posed_at` orders them)
/// as a part of the body: its most-weighted bone's ([`part_of_bone`]).
fn body_parts(world: &World, assets: &Assets, body: InstanceId) -> Vec<u8> {
    let Some(Value::String(mesh_id)) = world.get(body).and_then(|p| p.property("MeshId")) else {
        panic!("the body has no MeshId");
    };
    let model = assets.model_blocking(mesh_id).expect("the body's model");
    let rig = model.rig.as_ref().expect("the body's rig");
    let mut out = Vec::new();
    for primitive in &model.primitives {
        for k in 0..primitive.positions.len() {
            let joints = primitive.joints.get(k).copied().unwrap_or_default();
            let weights = primitive.weights.get(k).copied().unwrap_or_default();
            let most = (0..4)
                .max_by(|a, b| weights[*a].total_cmp(&weights[*b]))
                .unwrap_or(0);
            let part = rig
                .joints
                .get(usize::from(joints[most]))
                .and_then(|joint| rig.bones.get(joint.bone))
                .map_or(0, |bone| part_of_bone(&bone.name));
            out.push(part);
        }
    }
    out
}

/// The body part of a character's world.
fn body_of(world: &World) -> InstanceId {
    world
        .all_below(world.workspace(), |class| class == ClassName::MeshPart)
        .into_iter()
        .find(|part| world.get(*part).is_some_and(|p| p.name() == "Body"))
        .expect("a body")
}

/// What one garment has of one kind on one body in one pose.
pub(super) struct Found {
    pub garment: String,
    pub pose: String,
    pub body: String,
    pub kind: Kind,
    pub triangles: usize,
    pub worst: f32,
}

/// Measures every garment in `posed` that `ours` takes against itself in
/// `rest` (both worn by the body `name`; `hulls` the bare body's in the
/// pose), adding what it finds to `found`.
fn measure_cloth(
    name: &str,
    pose: &str,
    rest: &[Posed],
    posed: &[Posed],
    hulls: &Hulls,
    ours: fn(&str) -> bool,
    found: &mut Vec<Found>,
) {
    for garment in posed {
        if is_footwear(garment) || !ours(&garment.name) {
            continue;
        }
        let at_rest = rest
            .iter()
            .find(|g| g.name == garment.name)
            .expect("the garment at rest");
        let kinds = [
            (Kind::Stretched, stretched(at_rest, garment)),
            (Kind::Folded, folded(at_rest, garment)),
            (
                Kind::Hanging,
                hanging(at_rest, garment, hulls, ease_of(&garment.name)),
            ),
        ];
        for (kind, triangles) in kinds {
            if triangles.is_empty() {
                continue;
            }
            debug_dump(name, pose, garment, kind, &triangles);
            found.push(Found {
                garment: garment.name.clone(),
                pose: pose.to_owned(),
                body: name.to_owned(),
                kind,
                triangles: triangles.len(),
                worst: triangles.iter().map(|(_, w)| *w).fold(0.0, f32::max),
            });
        }
    }
}

/// With `AXCENITY_CLOTH_DEBUG` set (to text a body's name must contain),
/// prints where each defect is: the triangle, its middle and its measure.
fn debug_dump(name: &str, pose: &str, garment: &Posed, kind: Kind, triangles: &[(u32, f32)]) {
    let Ok(only) = std::env::var("AXCENITY_CLOTH_DEBUG") else {
        return;
    };
    if !name.contains(&only) {
        return;
    }
    for (t, worst) in triangles {
        let [a, b, c] = garment.triangles[*t as usize].map(|v| garment.positions[v as usize]);
        let middle = (a + b + c) / 3.0;
        println!(
            "  cloth {name} {pose} {} {kind:?} triangle {t} at ({:.3}, {:.3}, {:.3}): {}",
            garment.name,
            middle.x,
            middle.y,
            middle.z,
            kind.show(*worst)
        );
    }
}

/// A recorded cloth defect of a draft: per garment, pose and kind, the
/// bodies with any, the triangles on all of them, and the worst. Exact:
/// a case with more fails, and one with fewer (or a worst more than the
/// kind's slack under) must lower the record.
#[derive(Clone, Copy, Debug)]
pub(super) struct ClothRecord {
    pub garment: &'static str,
    pub pose: &'static str,
    pub kind: Kind,
    pub bodies: usize,
    pub triangles: usize,
    pub worst: f32,
}

/// A record (shorter to write in a table).
pub(super) const fn rec(
    garment: &'static str,
    pose: &'static str,
    kind: Kind,
    bodies: usize,
    triangles: usize,
    worst: f32,
) -> ClothRecord {
    ClothRecord {
        garment,
        pose,
        kind,
        bodies,
        triangles,
        worst,
    }
}

/// Fails on any garment, pose and kind measured otherwise than its record
/// says (none, when there is no record), and prints the records that would
/// match.
pub(super) fn check_cloth(label: &str, found: &[Found], records: &[ClothRecord]) {
    let mut groups: BTreeMap<(&str, &str, Kind), (usize, usize, f32)> = BTreeMap::new();
    for f in found {
        let group = groups
            .entry((f.garment.as_str(), f.pose.as_str(), f.kind))
            .or_default();
        group.0 += 1;
        group.1 += f.triangles;
        group.2 = group.2.max(f.worst);
    }
    for ((garment, pose, kind), (bodies, triangles, worst)) in &groups {
        println!(
            "  {label} measured: rec({garment:?}, {pose:?}, Kind::{kind:?}, {bodies}, {triangles}, {worst:.4}), // {}",
            kind.show(*worst)
        );
    }
    let mut problems = Vec::new();
    for ((garment, pose, kind), (bodies, triangles, worst)) in &groups {
        let record = records
            .iter()
            .find(|r| r.garment == *garment && r.pose == *pose && r.kind == *kind);
        let (rb, rt, rw) = record.map_or((0, 0, 0.0), |r| (r.bodies, r.triangles, r.worst));
        if *bodies > rb || *triangles > rt || *worst > rw + kind.slack() * 0.5 {
            problems.push(format!(
                "{garment} {pose} {kind:?}: {bodies} bodies, {triangles} triangles, worst {} (recorded {rb}, {rt}, {})",
                kind.show(*worst),
                kind.show(rw)
            ));
        }
    }
    for r in records {
        match groups.get(&(r.garment, r.pose, r.kind)) {
            None => problems.push(format!(
                "{} {} {:?}: recorded {} bodies, {} triangles, now none: remove the record",
                r.garment, r.pose, r.kind, r.bodies, r.triangles
            )),
            Some((bodies, triangles, worst))
                if *bodies < r.bodies
                    || *triangles < r.triangles
                    || *worst < r.worst - r.kind.slack() =>
            {
                problems.push(format!(
                    "{} {} {:?}: recorded {} bodies, {} triangles, worst {}; now {bodies}, {triangles}, {}: lower the record",
                    r.garment,
                    r.pose,
                    r.kind,
                    r.bodies,
                    r.triangles,
                    r.kind.show(r.worst),
                    r.kind.show(*worst)
                ))
            }
            Some(_) => {}
        }
    }
    assert!(problems.is_empty(), "{label}: {problems:#?}");
}

/// A set of garments the check measures: worn over the default underwear
/// by the bodies of a family, from a library.
struct Set {
    name: &'static str,
    library: fn() -> PathBuf,
    /// The benchmark descriptions (`fit_gate_tests::bodies` or a family's
    /// cast).
    bodies: fn() -> Vec<Description>,
    items: &'static [&'static str],
    /// Which garments are this set's to record.
    ours: fn(&str) -> bool,
}

impl Set {
    fn dressed(&self, description: &Description) -> Description {
        let mut out = description.clone();
        for id in self.items {
            if !out.items.iter().any(|worn| worn.id == *id) {
                out.items.push(WornItem::new(*id));
            }
        }
        out
    }
}

/// The body's hulls and the garments, as `animators` last posed `world`
/// (and its bare twin in `bare`).
fn posed_now(
    assets: &Assets,
    dressed: (&World, &axcenity_anim::Animators),
    bare: (&World, &axcenity_anim::Animators),
    parts: &[u8],
) -> (Hulls, Vec<Posed>) {
    let (_, garments, _) = worn_in(dressed.0, assets, dressed.1);
    let (body, _, _) = worn_in(bare.0, assets, bare.1);
    (Hulls::new(&body.positions, &body.normals, parts), garments)
}

/// The garments of `set` at rest on `description`, and the body's parts.
fn at_rest(stage: &Stage, set: &Set, description: &Description) -> (Vec<Posed>, Vec<u8>) {
    let (world, animators) =
        stage_world(&stage.assets, &set.dressed(description), Stance::Rest, &[]).expect("rest");
    let parts = body_parts(&world, &stage.assets, body_of(&world));
    let (_, garments, _) = worn_in(&world, &stage.assets, &animators);
    (garments, parts)
}

/// Runs the check for `stance` on every body of `set`.
fn cloth_gate(set: &Set, stance: Stance, records: &[ClothRecord]) {
    let pose = pose_name(stance);
    let stage =
        Stage::new(&format!("cloth-{}-{pose}", set.name), &(set.library)()).expect("the stage");
    let mut found = Vec::new();
    let started = std::time::Instant::now();
    for description in (set.bodies)() {
        let (rest, parts) = at_rest(&stage, set, &description);
        let dressed =
            stage_world(&stage.assets, &set.dressed(&description), stance, &[]).expect("the pose");
        let bare = stage_world(&stage.assets, &undressed(&description), stance, &[])
            .expect("the bare pose");
        let (hulls, garments) = posed_now(
            &stage.assets,
            (&dressed.0, &dressed.1),
            (&bare.0, &bare.1),
            &parts,
        );
        measure_cloth(
            &description.name,
            &pose,
            &rest,
            &garments,
            &hulls,
            set.ours,
            &mut found,
        );
    }
    println!(
        "cloth {} {pose}: {} cases in {:.1} s",
        set.name,
        found.len(),
        started.elapsed().as_secs_f32()
    );
    check_cloth(&format!("cloth {} {pose}", set.name), &found, records);
}

/// Runs the check through the walk's `frames` on every cast body of `set`. Each
/// garment and kind is kept once per body, at the frame with the most
/// triangles (the worst its worst over the frames), under the pose
/// [`WALK`].
fn cloth_gate_walk(set: &Set, frames: &[u32], records: &[ClothRecord]) {
    let stage = Stage::new(
        &format!("cloth-{}-walk-{}", set.name, frames[0]),
        &(set.library)(),
    )
    .expect("the stage");
    let first = frames.iter().copied().min().unwrap_or(2).max(2);
    let last = frames.iter().copied().max().unwrap_or(first);
    let walk = Stance::Clip {
        name: "Walk",
        frames: first,
    };
    let mut found = Vec::new();
    // The cast only (not the body benchmarks' extremes, which the held
    // poses take): each frame poses the character twice, dressed and bare,
    // and the whole stride on every body is past the test time limit.
    let cast = (set.bodies)()
        .into_iter()
        .filter(|d| d.name.contains("-cast-"));
    for description in cast {
        let (rest, parts) = at_rest(&stage, set, &description);
        let (mut world, mut animators) =
            stage_world(&stage.assets, &set.dressed(&description), walk, &[]).expect("the walk");
        let (mut bare_world, mut bare_animators) =
            stage_world(&stage.assets, &undressed(&description), walk, &[]).expect("the walk");
        let mut kept: BTreeMap<(String, Kind), Found> = BTreeMap::new();
        for frame in first..=last {
            if frames.contains(&frame) {
                let (hulls, garments) = posed_now(
                    &stage.assets,
                    (&world, &animators),
                    (&bare_world, &bare_animators),
                    &parts,
                );
                let mut here = Vec::new();
                let pose = format!("Walk@{frame}");
                measure_cloth(
                    &description.name,
                    &pose,
                    &rest,
                    &garments,
                    &hulls,
                    set.ours,
                    &mut here,
                );
                for f in here {
                    let key = (f.garment.clone(), f.kind);
                    let worst = kept.get(&key).map_or(f.worst, |k| k.worst.max(f.worst));
                    let more = kept.get(&key).is_none_or(|k| f.triangles > k.triangles);
                    if more {
                        kept.insert(key.clone(), f);
                    }
                    if let Some(k) = kept.get_mut(&key) {
                        k.worst = worst;
                    }
                }
            }
            animators.update(&mut world, &stage.assets, 1.0 / 60.0);
            bare_animators.update(&mut bare_world, &stage.assets, 1.0 / 60.0);
        }
        found.extend(kept.into_values().map(|f| Found {
            body: format!("{} ({})", f.body, f.pose),
            pose: WALK.to_owned(),
            ..f
        }));
    }
    check_cloth(
        &format!("cloth {} {WALK} {frames:?}", set.name),
        &found,
        records,
    );
}

// The sets.

/// The fit gate's: the test tee (`avatar`'s fixture, imported by the
/// engine's fit when the tests run) and the realistic underwear, on every
/// realistic cast and body benchmark.
const GATE: Set = Set {
    name: "gate",
    library: || gate_library().to_owned(),
    bodies,
    items: &[TEST_TEE],
    ours: |_| true,
};

/// The first licensed tee (`tee_crew_tests`).
const LICENSED: Set = Set {
    name: "licensed-tee",
    library: repository_library,
    bodies,
    items: &[super::tee_crew_tests::TEE],
    ours: |name| name == super::tee_crew_tests::PART,
};

/// The first realistic wardrobe (`wardrobe_tests`): the wave-4 crew tee
/// and the joggers (the sneakers are the footwear check's).
const WARDROBE: Set = Set {
    name: "wardrobe",
    library: repository_library,
    bodies,
    items: &[
        super::wardrobe_tests::TEE,
        super::wardrobe_tests::JOGGERS,
        super::wardrobe_tests::SNEAKERS,
    ],
    ours: |name| name == "Shirt_CrewTee" || name == "Pants_Joggers",
};

/// The stylized wardrobe on the stylized cast (the extremes take it past
/// the test time limit, as in `fit_gate_tests`), with its underwear.
const STYLIZED: Set = Set {
    name: "stylized",
    library: repository_library,
    bodies: || {
        benchmarks("stylized-")
            .into_iter()
            .filter(|d| d.name.contains("-cast-"))
            .collect()
    },
    items: &["axcenity/tee", "axcenity/cargo-pants", "axcenity/sneakers"],
    ours: |_| true,
};

#[cfg(test)]
mod synthetic {
    use super::*;

    /// A flat square of cloth `size` across, `cells` a side, at height `z`,
    /// facing up (+Z), its corners' normals +Z.
    fn sheet(size: f32, cells: u32, z: f32) -> Posed {
        let mut out = Posed {
            name: "Cloth".to_owned(),
            positions: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
        };
        for j in 0..=cells {
            for i in 0..=cells {
                let at = |k: u32| (k as f32 / cells as f32 - 0.5) * size;
                out.positions.push(Vec3::new(at(i), at(j), z));
                out.normals.push(Vec3::Z);
            }
        }
        let row = cells + 1;
        for j in 0..cells {
            for i in 0..cells {
                let a = j * row + i;
                out.triangles.push([a, a + 1, a + row + 1]);
                out.triangles.push([a, a + row + 1, a + row]);
            }
        }
        out
    }

    /// The triangles of a 10-cell sheet between rows `j` and `j + 1`.
    fn row(j: u32) -> Vec<u32> {
        (j * 20..(j + 1) * 20).collect()
    }

    /// A sheet stretched evenly within what knit takes isn't stretched; a
    /// band pulled to twice its length (a strip torn open under a sleeve)
    /// is, and only the band's triangles.
    #[test]
    fn a_sheet_stretched_past_the_fabric_is_found() {
        let rest = sheet(0.2, 10, 0.0);
        let mut even = rest.clone();
        for p in &mut even.positions {
            p.y *= 1.4;
        }
        assert!(stretched(&rest, &even).is_empty());
        // Every corner from row 6 up moved 2 cm on: the band between rows
        // 5 and 6 (2 cm tall) is pulled to 4 cm, twice its rest.
        let mut torn = rest.clone();
        for (k, p) in torn.positions.iter_mut().enumerate() {
            if k / 11 >= 6 {
                p.y += 0.02;
            }
        }
        let found = stretched(&rest, &torn);
        let mut triangles: Vec<u32> = found.iter().map(|(t, _)| *t).collect();
        triangles.sort_unstable();
        assert_eq!(triangles, row(5));
        let most = found.iter().map(|(_, s)| *s).fold(0.0, f32::max);
        // The band's diagonal grows least, its upright edge most: twice.
        assert!((most - 2.0).abs() < 1e-3, "{most}");
    }

    /// A sheet turned over whole by the bones (its normals turned with it)
    /// isn't folded; a row of corners pushed past the next row while their
    /// normals stay (cloth folded over itself) is, on the row turned over.
    #[test]
    fn a_fold_turned_over_is_found() {
        let rest = sheet(0.2, 10, 0.0);
        let mut turned = rest.clone();
        for p in &mut turned.positions {
            *p = Vec3::new(p.x, -p.y, -p.z);
        }
        for n in &mut turned.normals {
            *n = -*n;
        }
        assert!(folded(&rest, &turned).is_empty());
        // Row 5's corners pushed 3 cm on, past row 6 (2 cm on): the band
        // between rows 5 and 6 now faces down, its corners' normals up.
        let mut fold = rest.clone();
        for (k, p) in fold.positions.iter_mut().enumerate() {
            if k / 11 == 5 {
                p.y += 0.03;
            }
        }
        let found = folded(&rest, &fold);
        let mut triangles: Vec<u32> = found.iter().map(|(t, _)| *t).collect();
        triangles.sort_unstable();
        assert_eq!(triangles, row(5));
        for (_, degrees) in found {
            assert!((degrees - 180.0).abs() < 1.0, "{degrees}");
        }
    }

    /// Points on a cylinder (radius `r`, along Y from `y0` to `y1`), all of
    /// body part `part`, and with a hollow: the side facing +Z sunk in
    /// `hollow` deep between y = -2 and +2 cm (the small of a back).
    fn limb(r: f32, y0: f32, y1: f32, hollow: f32) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut out = Vec::new();
        let mut normals = Vec::new();
        for j in 0..=40 {
            let y = y0 + (y1 - y0) * j as f32 / 40.0;
            for i in 0..48 {
                let a = i as f32 / 48.0 * std::f32::consts::TAU;
                let (s, c) = a.sin_cos();
                let mut p = Vec3::new(r * s, y, r * c);
                if c > 0.7 && y.abs() < 0.02 {
                    p.z -= hollow;
                }
                out.push(p);
                normals.push(Vec3::new(s, 0.0, c));
            }
        }
        (out, normals)
    }

    /// Cloth resting on a body part, or draped across a hollow in it, isn't
    /// hanging; a tab pulled out past the ease is, and only its triangles.
    #[test]
    fn a_tab_hanging_off_the_body_is_found() {
        let (points, normals) = limb(0.1, -0.2, 0.2, 0.03);
        let hulls = Hulls::new(&points, &normals, &vec![0; points.len()]);
        // Inside the part's hull, on it and 2 cm out: the measure is the
        // distance outside, never more, and short of it by at most the
        // directions' error.
        let on = |p: Vec3, want: f32| {
            let got = hulls.outside(p);
            assert!(
                got <= want + 1e-4 && got > want - 0.004,
                "{p}: {got}, not {want}"
            );
        };
        assert!(hulls.outside(Vec3::ZERO) < -0.09);
        on(Vec3::new(0.0, 0.0, 0.1), 0.0);
        on(Vec3::new(0.0, 0.0, 0.12), 0.02);
        // A patch of cloth 5 mm off the +Z side, across the hollow (3 cm
        // deep there): resting, nothing hangs.
        let mut cloth = sheet(0.1, 10, 0.105);
        let flat = cloth.clone();
        assert!(hanging(&flat, &cloth, &hulls, 0.03).is_empty());
        // A tab: one corner (the middle of the bottom row) pulled 6 cm out
        // and down, the cloth's hem pulled into a point.
        cloth.positions[5] += Vec3::new(0.0, -0.03, 0.06);
        let found = hanging(&flat, &cloth, &hulls, 0.03);
        assert!(!found.is_empty());
        for (t, out) in &found {
            assert!(cloth.triangles[*t as usize].contains(&5), "triangle {t}");
            assert!(*out > 0.03 && *out < 0.07, "{out}");
        }
        // Two parts: cloth by one is measured from the nearest.
        let mut two = points.clone();
        let far: Vec<Vec3> = points.iter().map(|p| *p + Vec3::X).collect();
        two.extend(&far);
        let parts: Vec<u8> = (0..two.len())
            .map(|k| u8::from(k >= points.len()))
            .collect();
        let hulls = Hulls::new(&two, &[normals.clone(), normals.clone()].concat(), &parts);
        let got = hulls.outside(Vec3::new(1.0, 0.0, 0.12));
        assert!(got <= 0.0201 && got > 0.016, "{got}");
        // Two parts that meet at a joint (the torso, 0, and the left upper
        // arm, 4): cloth across the crease between them rests on the joint's
        // hull; the same cloth between parts that don't meet hangs.
        let arm: Vec<Vec3> = points
            .iter()
            .map(|p| *p + Vec3::new(0.3, 0.0, 0.0))
            .collect();
        let both = [points.clone(), arm].concat();
        let normals2 = [normals.clone(), normals.clone()].concat();
        let between = Vec3::new(0.15, 0.0, 0.09);
        let joint: Vec<u8> = (0..both.len())
            .map(|k| if k < points.len() { 0 } else { 4 })
            .collect();
        assert!(
            Hulls::new(&both, &normals2, &joint)
                .hangs(between, 0.03)
                .is_none()
        );
        let apart: Vec<u8> = (0..both.len())
            .map(|k| if k < points.len() { 0 } else { 3 })
            .collect();
        assert!(
            Hulls::new(&both, &normals2, &apart)
                .hangs(between, 0.03)
                .is_some()
        );
    }

    /// A hem: a ring of `n` corners (radius `r`, at height 0, level) and a
    /// band of cloth 15 cm above it, the ring's corners first (the band's
    /// top edge is a garment's top, not a hem: `HEM_REACH`).
    fn hem_ring(r: f32, n: u32) -> Posed {
        let mut out = Posed {
            name: "Cloth".to_owned(),
            positions: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
        };
        for y in [0.0, 0.15] {
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                out.positions.push(Vec3::new(r * a.sin(), y, r * a.cos()));
                out.normals.push(Vec3::new(a.sin(), 0.0, a.cos()));
            }
        }
        for i in 0..n {
            let j = (i + 1) % n;
            out.triangles.push([i, j, n + j]);
            out.triangles.push([i, n + j, n + i]);
        }
        out
    }

    /// A level hem, the same hem with a corner pulled up, or turned upright
    /// (an arm raised in its sleeve), has no point; one corner of it pulled 5 cm down (the tee's
    /// centre back drawn into the cleft) is a point, and the triangles at
    /// it hang.
    #[test]
    fn a_hem_pulled_into_a_point_hangs() {
        let rest = hem_ring(0.15, 48);
        assert!(hem_points(&rest, &rest, 0.03).is_empty());
        let mut raised = rest.clone();
        raised.positions[0].y += 0.05;
        assert!(hem_points(&rest, &raised, 0.03).is_empty());
        let mut upright = rest.clone();
        for p in &mut upright.positions {
            *p = Vec3::new(p.x, p.z, -p.y);
        }
        assert!(hem_points(&rest, &upright, 0.03).is_empty());
        let mut tab = rest.clone();
        tab.positions[0].y -= 0.05;
        tab.positions[1].y -= 0.025;
        tab.positions[47].y -= 0.025;
        let points = hem_points(&rest, &tab, 0.03);
        assert_eq!(points.len(), 1, "{points:?}");
        assert_eq!(points[0].0, 0);
        assert!(points[0].1 > 0.04 && points[0].1 < 0.055, "{points:?}");
        // A turned hem: the cloth turned under at the ring at y = 0 (now
        // its fold) and back up inside to an open edge at y = 2 cm (corners
        // 96 on). The fold's corner 0 pulled 5 cm down is a point at the
        // open edge's corner above it, found by walking down to the fold.
        let mut turned = rest.clone();
        for i in 0..48u32 {
            let a = i as f32 / 48.0 * std::f32::consts::TAU;
            turned
                .positions
                .push(Vec3::new(0.147 * a.sin(), 0.02, 0.147 * a.cos()));
            turned.normals.push(-Vec3::new(a.sin(), 0.0, a.cos()));
        }
        for i in 0..48u32 {
            let j = (i + 1) % 48;
            turned.triangles.push([i, 96 + j, j]);
            turned.triangles.push([i, 96 + i, 96 + j]);
        }
        let turned_rest = turned.clone();
        assert!(hem_points(&turned_rest, &turned_rest, 0.03).is_empty());
        turned.positions[0].y -= 0.05;
        let points = hem_points(&turned_rest, &turned, 0.03);
        assert!(points.iter().any(|(c, _)| *c == 96), "{points:?}");
        // A body just inside the band: only the point's triangles hang.
        let body: Vec<Vec3> = rest
            .positions
            .iter()
            .map(|p| *p * Vec3::new(0.97, 1.0, 0.97))
            .collect();
        let hulls = Hulls::new(&body, &rest.normals, &vec![0; body.len()]);
        assert!(hanging(&rest, &rest, &hulls, 0.03).is_empty());
        let found = hanging(&rest, &tab, &hulls, 0.03);
        assert!(!found.is_empty());
        for (t, _) in found {
            assert!(tab.triangles[t as usize].contains(&0), "triangle {t}");
        }
    }

    /// A body's bones as parts: the limbs by side, the torso and the head.
    #[test]
    fn the_bones_name_the_parts() {
        assert_eq!(part_of_bone("Hips"), 0);
        assert_eq!(part_of_bone("UpperChest"), 0);
        assert_eq!(part_of_bone("LeftShoulder"), 0);
        assert_eq!(part_of_bone("Neck"), 0);
        assert_eq!(part_of_bone("Head"), 1);
        assert_eq!(
            part_of_bone("LeftUpperArm"),
            part_of_bone("LeftUpperArmTwist")
        );
        assert_ne!(part_of_bone("LeftUpperArm"), part_of_bone("RightUpperArm"));
        assert_ne!(part_of_bone("LeftUpperArm"), part_of_bone("LeftLowerArm"));
        assert_eq!(part_of_bone("LeftIndexProximal"), part_of_bone("LeftHand"));
        assert_eq!(part_of_bone("RightToes"), part_of_bone("RightFoot"));
        assert_ne!(part_of_bone("RightUpperLeg"), part_of_bone("RightLowerLeg"));
    }

    /// The ratchet: a case at its record passes; more bodies, more
    /// triangles or a worse worst fail; fewer, or a worst well under, or
    /// none, must lower the record; one not recorded fails.
    #[test]
    fn the_cloth_check_holds_garments_to_their_records() {
        let found = |bodies: usize, triangles: usize, worst: f32| -> Vec<Found> {
            (0..bodies)
                .map(|k| Found {
                    garment: "Draft".to_owned(),
                    pose: "TPose".to_owned(),
                    body: format!("body {k}"),
                    kind: Kind::Stretched,
                    triangles: if k == 0 { triangles - (bodies - 1) } else { 1 },
                    worst,
                })
                .collect()
        };
        let records = [rec("Draft", "TPose", Kind::Stretched, 2, 10, 3.0)];
        check_cloth("at its record", &found(2, 10, 3.0), &records);
        for (label, cases, records) in [
            ("more bodies", found(3, 10, 3.0), &records[..]),
            ("more triangles", found(2, 11, 3.0), &records[..]),
            ("worse", found(2, 10, 3.1), &records[..]),
            ("fewer bodies", found(1, 10, 3.0), &records[..]),
            ("fewer triangles", found(2, 9, 3.0), &records[..]),
            ("well under", found(2, 10, 2.5), &records[..]),
            ("gone", Vec::new(), &records[..]),
            ("not recorded", found(1, 1, 2.0), &[][..]),
        ] {
            let caught = std::panic::catch_unwind(|| check_cloth(label, &cases, records));
            assert!(caught.is_err(), "{label}");
        }
    }
}

// The checks of every garment the gate fits, per set and pose, against
// their records (below).

#[test]
fn cloth_holds_at_rest_gate() {
    cloth_gate(&GATE, Stance::Rest, &records::GATE_REST);
}
#[test]
fn cloth_holds_in_idle_gate() {
    cloth_gate(&GATE, Stance::Idle, &records::GATE_IDLE);
}
#[test]
fn cloth_holds_with_the_arms_out_gate() {
    cloth_gate(&GATE, Stance::Held(HeldPose::TPose), &records::GATE_T_POSE);
}
#[test]
fn cloth_holds_with_the_arms_raised_gate() {
    cloth_gate(
        &GATE,
        Stance::Held(HeldPose::ArmsRaised),
        &records::GATE_ARMS_RAISED,
    );
}
#[test]
fn cloth_holds_with_the_arms_forward_gate() {
    cloth_gate(
        &GATE,
        Stance::Held(HeldPose::ArmsForward),
        &records::GATE_ARMS_FORWARD,
    );
}
#[test]
fn cloth_holds_with_the_elbows_bent_gate() {
    cloth_gate(
        &GATE,
        Stance::Held(HeldPose::ElbowsBent),
        &records::GATE_ELBOWS_BENT,
    );
}
#[test]
fn cloth_holds_with_the_knees_bent_gate() {
    cloth_gate(
        &GATE,
        Stance::Held(HeldPose::KneesBent),
        &records::GATE_KNEES_BENT,
    );
}
#[test]
fn cloth_holds_through_the_walk_first_third_gate() {
    cloth_gate_walk(&GATE, WALK_FRAMES[0], records::GATE_WALK[0]);
}
#[test]
fn cloth_holds_through_the_walk_second_third_gate() {
    cloth_gate_walk(&GATE, WALK_FRAMES[1], records::GATE_WALK[1]);
}
#[test]
fn cloth_holds_through_the_walk_last_third_gate() {
    cloth_gate_walk(&GATE, WALK_FRAMES[2], records::GATE_WALK[2]);
}

#[test]
fn cloth_holds_at_rest_licensed_tee() {
    cloth_gate(&LICENSED, Stance::Rest, &records::LICENSED_REST);
}
#[test]
fn cloth_holds_in_idle_licensed_tee() {
    cloth_gate(&LICENSED, Stance::Idle, &records::LICENSED_IDLE);
}
#[test]
fn cloth_holds_with_the_arms_out_licensed_tee() {
    cloth_gate(
        &LICENSED,
        Stance::Held(HeldPose::TPose),
        &records::LICENSED_T_POSE,
    );
}
#[test]
fn cloth_holds_with_the_arms_raised_licensed_tee() {
    cloth_gate(
        &LICENSED,
        Stance::Held(HeldPose::ArmsRaised),
        &records::LICENSED_ARMS_RAISED,
    );
}
#[test]
fn cloth_holds_with_the_arms_forward_licensed_tee() {
    cloth_gate(
        &LICENSED,
        Stance::Held(HeldPose::ArmsForward),
        &records::LICENSED_ARMS_FORWARD,
    );
}
#[test]
fn cloth_holds_with_the_elbows_bent_licensed_tee() {
    cloth_gate(
        &LICENSED,
        Stance::Held(HeldPose::ElbowsBent),
        &records::LICENSED_ELBOWS_BENT,
    );
}
#[test]
fn cloth_holds_with_the_knees_bent_licensed_tee() {
    cloth_gate(
        &LICENSED,
        Stance::Held(HeldPose::KneesBent),
        &records::LICENSED_KNEES_BENT,
    );
}
#[test]
fn cloth_holds_through_the_walk_first_third_licensed_tee() {
    cloth_gate_walk(&LICENSED, WALK_FRAMES[0], records::LICENSED_WALK[0]);
}
#[test]
fn cloth_holds_through_the_walk_second_third_licensed_tee() {
    cloth_gate_walk(&LICENSED, WALK_FRAMES[1], records::LICENSED_WALK[1]);
}
#[test]
fn cloth_holds_through_the_walk_last_third_licensed_tee() {
    cloth_gate_walk(&LICENSED, WALK_FRAMES[2], records::LICENSED_WALK[2]);
}

#[test]
fn cloth_holds_at_rest_wardrobe() {
    cloth_gate(&WARDROBE, Stance::Rest, &records::WARDROBE_REST);
}
#[test]
fn cloth_holds_in_idle_wardrobe() {
    cloth_gate(&WARDROBE, Stance::Idle, &records::WARDROBE_IDLE);
}
#[test]
fn cloth_holds_with_the_arms_out_wardrobe() {
    cloth_gate(
        &WARDROBE,
        Stance::Held(HeldPose::TPose),
        &records::WARDROBE_T_POSE,
    );
}
#[test]
fn cloth_holds_with_the_arms_raised_wardrobe() {
    cloth_gate(
        &WARDROBE,
        Stance::Held(HeldPose::ArmsRaised),
        &records::WARDROBE_ARMS_RAISED,
    );
}
#[test]
fn cloth_holds_with_the_arms_forward_wardrobe() {
    cloth_gate(
        &WARDROBE,
        Stance::Held(HeldPose::ArmsForward),
        &records::WARDROBE_ARMS_FORWARD,
    );
}
#[test]
fn cloth_holds_with_the_elbows_bent_wardrobe() {
    cloth_gate(
        &WARDROBE,
        Stance::Held(HeldPose::ElbowsBent),
        &records::WARDROBE_ELBOWS_BENT,
    );
}
#[test]
fn cloth_holds_with_the_knees_bent_wardrobe() {
    cloth_gate(
        &WARDROBE,
        Stance::Held(HeldPose::KneesBent),
        &records::WARDROBE_KNEES_BENT,
    );
}
#[test]
fn cloth_holds_through_the_walk_first_third_wardrobe() {
    cloth_gate_walk(&WARDROBE, WALK_FRAMES[0], records::WARDROBE_WALK[0]);
}
#[test]
fn cloth_holds_through_the_walk_second_third_wardrobe() {
    cloth_gate_walk(&WARDROBE, WALK_FRAMES[1], records::WARDROBE_WALK[1]);
}
#[test]
fn cloth_holds_through_the_walk_last_third_wardrobe() {
    cloth_gate_walk(&WARDROBE, WALK_FRAMES[2], records::WARDROBE_WALK[2]);
}

#[test]
fn cloth_holds_at_rest_stylized() {
    cloth_gate(&STYLIZED, Stance::Rest, &records::STYLIZED_REST);
}
#[test]
fn cloth_holds_with_the_arms_out_stylized() {
    cloth_gate(
        &STYLIZED,
        Stance::Held(HeldPose::TPose),
        &records::STYLIZED_T_POSE,
    );
}
#[test]
fn cloth_holds_walking_stylized() {
    let walk = Stance::Clip {
        name: "Walk",
        frames: 18,
    };
    cloth_gate(&STYLIZED, walk, &records::STYLIZED_WALK);
}

/// What each draft garment still has (open defects, not accepted
/// behaviour; `refinement/wave5/fit-drape.md` says where each is). Exact:
/// lower them as items improve; never raise one; a production item has
/// none.
#[rustfmt::skip]
mod records {
    #[allow(unused_imports)]
    use super::{ClothRecord, Kind, rec};

    pub const GATE_REST: [ClothRecord; 4] = [
        rec("TestTee", "rest", Kind::Hanging, 10, 557, 0.0385),
        rec("Underwear_BoxerBriefs", "rest", Kind::Hanging, 31, 1619, 0.0155),
        rec("Underwear_Bra", "rest", Kind::Hanging, 21, 1826, 0.0086),
        rec("Underwear_Briefs", "rest", Kind::Hanging, 21, 3462, 0.0694),
    ];
    pub const GATE_IDLE: [ClothRecord; 7] = [
        rec("TestTee", "idle", Kind::Folded, 54, 5533, 179.3445),
        rec("TestTee", "idle", Kind::Hanging, 11, 658, 0.0387),
        rec("TestTee", "idle", Kind::Stretched, 54, 5718, 47.7391),
        rec("Underwear_BoxerBriefs", "idle", Kind::Hanging, 33, 2126, 0.0171),
        rec("Underwear_Bra", "idle", Kind::Folded, 13, 14, 123.7613),
        rec("Underwear_Bra", "idle", Kind::Hanging, 21, 2858, 0.01),
        rec("Underwear_Briefs", "idle", Kind::Hanging, 21, 3518, 0.0685),
    ];
    pub const GATE_T_POSE: [ClothRecord; 8] = [
        rec("TestTee", "TPose", Kind::Folded, 54, 330, 176.9525),
        rec("TestTee", "TPose", Kind::Hanging, 9, 349, 0.0396),
        rec("TestTee", "TPose", Kind::Stretched, 54, 16859, 43.8648),
        rec("Underwear_BoxerBriefs", "TPose", Kind::Hanging, 31, 1557, 0.0155),
        rec("Underwear_Bra", "TPose", Kind::Folded, 21, 21, 176.8709),
        rec("Underwear_Bra", "TPose", Kind::Hanging, 21, 1757, 0.0086),
        rec("Underwear_Bra", "TPose", Kind::Stretched, 6, 61, 1.7989),
        rec("Underwear_Briefs", "TPose", Kind::Hanging, 21, 3469, 0.0694),
    ];
    pub const GATE_ARMS_RAISED: [ClothRecord; 8] = [
        rec("TestTee", "ArmsRaised", Kind::Folded, 54, 2849, 176.9948),
        rec("TestTee", "ArmsRaised", Kind::Hanging, 8, 302, 0.0372),
        rec("TestTee", "ArmsRaised", Kind::Stretched, 54, 36992, 86.742),
        rec("Underwear_BoxerBriefs", "ArmsRaised", Kind::Hanging, 32, 1639, 0.0155),
        rec("Underwear_Bra", "ArmsRaised", Kind::Folded, 21, 85, 176.9729),
        rec("Underwear_Bra", "ArmsRaised", Kind::Hanging, 21, 1581, 0.0095),
        rec("Underwear_Bra", "ArmsRaised", Kind::Stretched, 21, 2751, 2.7257),
        rec("Underwear_Briefs", "ArmsRaised", Kind::Hanging, 21, 3543, 0.0694),
    ];
    pub const GATE_ARMS_FORWARD: [ClothRecord; 8] = [
        rec("TestTee", "ArmsForward", Kind::Folded, 54, 7836, 177.9618),
        rec("TestTee", "ArmsForward", Kind::Hanging, 46, 1873, 0.0406),
        rec("TestTee", "ArmsForward", Kind::Stretched, 54, 43040, 69.537),
        rec("Underwear_BoxerBriefs", "ArmsForward", Kind::Hanging, 33, 2276, 0.0155),
        rec("Underwear_Bra", "ArmsForward", Kind::Folded, 21, 376, 171.8169),
        rec("Underwear_Bra", "ArmsForward", Kind::Hanging, 20, 706, 0.0083),
        rec("Underwear_Bra", "ArmsForward", Kind::Stretched, 21, 1617, 3.6558),
        rec("Underwear_Briefs", "ArmsForward", Kind::Hanging, 21, 3727, 0.0694),
    ];
    pub const GATE_ELBOWS_BENT: [ClothRecord; 6] = [
        rec("TestTee", "ElbowsBent", Kind::Folded, 46, 148, 161.2895),
        rec("TestTee", "ElbowsBent", Kind::Hanging, 11, 572, 0.0385),
        rec("TestTee", "ElbowsBent", Kind::Stretched, 54, 583, 14.6994),
        rec("Underwear_BoxerBriefs", "ElbowsBent", Kind::Hanging, 31, 1560, 0.0155),
        rec("Underwear_Bra", "ElbowsBent", Kind::Hanging, 21, 1820, 0.0086),
        rec("Underwear_Briefs", "ElbowsBent", Kind::Hanging, 21, 3510, 0.0694),
    ];
    pub const GATE_KNEES_BENT: [ClothRecord; 4] = [
        rec("TestTee", "KneesBent", Kind::Hanging, 9, 518, 0.0385),
        rec("Underwear_BoxerBriefs", "KneesBent", Kind::Hanging, 30, 569, 0.0144),
        rec("Underwear_Bra", "KneesBent", Kind::Hanging, 21, 1841, 0.0086),
        rec("Underwear_Briefs", "KneesBent", Kind::Hanging, 19, 2601, 0.0694),
    ];
    pub const GATE_WALK: [&[ClothRecord]; 3] = [
        &[
            rec("TestTee", "Walk", Kind::Folded, 18, 1878, 179.3996),
            rec("TestTee", "Walk", Kind::Hanging, 18, 582, 0.0707),
            rec("TestTee", "Walk", Kind::Stretched, 18, 5264, 58.6948),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Folded, 9, 53, 169.037),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Hanging, 9, 561, 0.0115),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Stretched, 9, 1148, 10.5517),
            rec("Underwear_Bra", "Walk", Kind::Folded, 9, 37, 155.2484),
            rec("Underwear_Bra", "Walk", Kind::Hanging, 9, 717, 0.0079),
            rec("Underwear_Bra", "Walk", Kind::Stretched, 9, 20, 1.5656),
            rec("Underwear_Briefs", "Walk", Kind::Folded, 9, 32, 132.1338),
            rec("Underwear_Briefs", "Walk", Kind::Hanging, 9, 694, 0.0697),
            rec("Underwear_Briefs", "Walk", Kind::Stretched, 9, 902, 3.8735),
        ],
        &[
            rec("TestTee", "Walk", Kind::Folded, 18, 1676, 179.4489),
            rec("TestTee", "Walk", Kind::Hanging, 18, 505, 0.0707),
            rec("TestTee", "Walk", Kind::Stretched, 18, 5544, 65.5732),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Folded, 9, 95, 167.5686),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Hanging, 2, 20, 0.0062),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Stretched, 9, 1203, 10.4443),
            rec("Underwear_Bra", "Walk", Kind::Hanging, 9, 368, 0.0076),
            rec("Underwear_Bra", "Walk", Kind::Stretched, 9, 80, 1.7308),
            rec("Underwear_Briefs", "Walk", Kind::Folded, 9, 39, 148.8986),
            rec("Underwear_Briefs", "Walk", Kind::Hanging, 1, 204, 0.0429),
            rec("Underwear_Briefs", "Walk", Kind::Stretched, 9, 928, 3.5221),
        ],
        &[
            rec("TestTee", "Walk", Kind::Folded, 18, 1878, 179.1076),
            rec("TestTee", "Walk", Kind::Hanging, 18, 547, 0.0706),
            rec("TestTee", "Walk", Kind::Stretched, 18, 5371, 60.8803),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Folded, 9, 49, 168.8231),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Hanging, 9, 519, 0.0111),
            rec("Underwear_BoxerBriefs", "Walk", Kind::Stretched, 9, 1100, 10.5105),
            rec("Underwear_Bra", "Walk", Kind::Folded, 9, 35, 155.0492),
            rec("Underwear_Bra", "Walk", Kind::Hanging, 9, 799, 0.0079),
            rec("Underwear_Bra", "Walk", Kind::Stretched, 9, 26, 1.5838),
            rec("Underwear_Briefs", "Walk", Kind::Folded, 9, 32, 138.2174),
            rec("Underwear_Briefs", "Walk", Kind::Hanging, 9, 740, 0.0688),
            rec("Underwear_Briefs", "Walk", Kind::Stretched, 9, 899, 3.843),
        ],
    ];

    pub const LICENSED_REST: [ClothRecord; 1] = [
        rec("Shirt_TeeCrew", "rest", Kind::Hanging, 54, 3533, 0.0435),
    ];
    pub const LICENSED_IDLE: [ClothRecord; 3] = [
        rec("Shirt_TeeCrew", "idle", Kind::Folded, 54, 3044, 179.8587),
        rec("Shirt_TeeCrew", "idle", Kind::Hanging, 54, 3974, 0.0458),
        rec("Shirt_TeeCrew", "idle", Kind::Stretched, 54, 7354, 14.3378),
    ];
    pub const LICENSED_T_POSE: [ClothRecord; 3] = [
        rec("Shirt_TeeCrew", "TPose", Kind::Folded, 54, 2729, 178.6898),
        rec("Shirt_TeeCrew", "TPose", Kind::Hanging, 54, 3452, 0.0435),
        rec("Shirt_TeeCrew", "TPose", Kind::Stretched, 54, 8533, 13.56),
    ];
    pub const LICENSED_ARMS_RAISED: [ClothRecord; 3] = [
        rec("Shirt_TeeCrew", "ArmsRaised", Kind::Folded, 54, 5511, 179.8972),
        rec("Shirt_TeeCrew", "ArmsRaised", Kind::Hanging, 54, 4058, 0.0435),
        rec("Shirt_TeeCrew", "ArmsRaised", Kind::Stretched, 54, 25274, 25.7453),
    ];
    pub const LICENSED_ARMS_FORWARD: [ClothRecord; 3] = [
        rec("Shirt_TeeCrew", "ArmsForward", Kind::Folded, 54, 6541, 179.8022),
        rec("Shirt_TeeCrew", "ArmsForward", Kind::Hanging, 54, 4631, 0.0458),
        rec("Shirt_TeeCrew", "ArmsForward", Kind::Stretched, 54, 32879, 31.7672),
    ];
    pub const LICENSED_ELBOWS_BENT: [ClothRecord; 3] = [
        rec("Shirt_TeeCrew", "ElbowsBent", Kind::Folded, 2, 2, 93.1256),
        rec("Shirt_TeeCrew", "ElbowsBent", Kind::Hanging, 54, 3560, 0.0435),
        rec("Shirt_TeeCrew", "ElbowsBent", Kind::Stretched, 54, 1418, 5.8921),
    ];
    pub const LICENSED_KNEES_BENT: [ClothRecord; 1] = [
        rec("Shirt_TeeCrew", "KneesBent", Kind::Hanging, 1, 14, 0.0314),
    ];
    pub const LICENSED_WALK: [&[ClothRecord]; 3] = [
        &[
            rec("Shirt_TeeCrew", "Walk", Kind::Folded, 18, 1690, 179.2202),
            rec("Shirt_TeeCrew", "Walk", Kind::Hanging, 18, 937, 0.0638),
            rec("Shirt_TeeCrew", "Walk", Kind::Stretched, 18, 6257, 14.3494),
        ],
        &[
            rec("Shirt_TeeCrew", "Walk", Kind::Folded, 18, 1487, 179.6643),
            rec("Shirt_TeeCrew", "Walk", Kind::Hanging, 18, 440, 0.07),
            rec("Shirt_TeeCrew", "Walk", Kind::Stretched, 18, 6216, 16.2222),
        ],
        &[
            rec("Shirt_TeeCrew", "Walk", Kind::Folded, 18, 1725, 179.7368),
            rec("Shirt_TeeCrew", "Walk", Kind::Hanging, 18, 866, 0.0638),
            rec("Shirt_TeeCrew", "Walk", Kind::Stretched, 18, 6357, 14.6612),
        ],
    ];

    pub const WARDROBE_REST: [ClothRecord; 2] = [
        rec("Pants_Joggers", "rest", Kind::Hanging, 54, 21851, 0.0768),
        rec("Shirt_CrewTee", "rest", Kind::Hanging, 54, 3724, 0.0863),
    ];
    pub const WARDROBE_IDLE: [ClothRecord; 6] = [
        rec("Pants_Joggers", "idle", Kind::Folded, 54, 183, 173.3237),
        rec("Pants_Joggers", "idle", Kind::Hanging, 54, 23564, 0.0768),
        rec("Pants_Joggers", "idle", Kind::Stretched, 54, 1322, 4.4846),
        rec("Shirt_CrewTee", "idle", Kind::Folded, 54, 3449, 178.7182),
        rec("Shirt_CrewTee", "idle", Kind::Hanging, 54, 4458, 0.0821),
        rec("Shirt_CrewTee", "idle", Kind::Stretched, 54, 4454, 62.8966),
    ];
    pub const WARDROBE_T_POSE: [ClothRecord; 4] = [
        rec("Pants_Joggers", "TPose", Kind::Hanging, 54, 21571, 0.0768),
        rec("Shirt_CrewTee", "TPose", Kind::Folded, 54, 992, 178.0296),
        rec("Shirt_CrewTee", "TPose", Kind::Hanging, 54, 3743, 0.0863),
        rec("Shirt_CrewTee", "TPose", Kind::Stretched, 54, 9857, 96.2052),
    ];
    pub const WARDROBE_ARMS_RAISED: [ClothRecord; 4] = [
        rec("Pants_Joggers", "ArmsRaised", Kind::Hanging, 54, 22949, 0.0768),
        rec("Shirt_CrewTee", "ArmsRaised", Kind::Folded, 54, 1897, 178.0212),
        rec("Shirt_CrewTee", "ArmsRaised", Kind::Hanging, 54, 3963, 0.0863),
        rec("Shirt_CrewTee", "ArmsRaised", Kind::Stretched, 54, 34635, 178.9713),
    ];
    pub const WARDROBE_ARMS_FORWARD: [ClothRecord; 6] = [
        rec("Pants_Joggers", "ArmsForward", Kind::Folded, 54, 3718, 179.0447),
        rec("Pants_Joggers", "ArmsForward", Kind::Hanging, 54, 21179, 0.0768),
        rec("Pants_Joggers", "ArmsForward", Kind::Stretched, 54, 16037, 16.5881),
        rec("Shirt_CrewTee", "ArmsForward", Kind::Folded, 54, 6144, 179.2265),
        rec("Shirt_CrewTee", "ArmsForward", Kind::Hanging, 54, 5663, 0.0993),
        rec("Shirt_CrewTee", "ArmsForward", Kind::Stretched, 54, 30622, 95.4016),
    ];
    pub const WARDROBE_ELBOWS_BENT: [ClothRecord; 2] = [
        rec("Pants_Joggers", "ElbowsBent", Kind::Hanging, 54, 21598, 0.0768),
        rec("Shirt_CrewTee", "ElbowsBent", Kind::Hanging, 54, 3732, 0.0863),
    ];
    pub const WARDROBE_KNEES_BENT: [ClothRecord; 4] = [
        rec("Pants_Joggers", "KneesBent", Kind::Folded, 54, 1888, 179.1164),
        rec("Pants_Joggers", "KneesBent", Kind::Hanging, 54, 24671, 0.0376),
        rec("Pants_Joggers", "KneesBent", Kind::Stretched, 54, 9058, 3.1889),
        rec("Shirt_CrewTee", "KneesBent", Kind::Hanging, 15, 487, 0.0863),
    ];
    pub const WARDROBE_WALK: [&[ClothRecord]; 3] = [
        &[
            rec("Pants_Joggers", "Walk", Kind::Folded, 18, 1627, 179.0327),
            rec("Pants_Joggers", "Walk", Kind::Hanging, 18, 9358, 0.0761),
            rec("Pants_Joggers", "Walk", Kind::Stretched, 18, 8900, 65.902),
            rec("Shirt_CrewTee", "Walk", Kind::Folded, 18, 1467, 179.0952),
            rec("Shirt_CrewTee", "Walk", Kind::Hanging, 18, 976, 0.0793),
            rec("Shirt_CrewTee", "Walk", Kind::Stretched, 18, 5035, 92.405),
        ],
        &[
            rec("Pants_Joggers", "Walk", Kind::Folded, 18, 1540, 178.5659),
            rec("Pants_Joggers", "Walk", Kind::Hanging, 18, 8205, 0.0763),
            rec("Pants_Joggers", "Walk", Kind::Stretched, 18, 7949, 58.5563),
            rec("Shirt_CrewTee", "Walk", Kind::Folded, 18, 1053, 179.0081),
            rec("Shirt_CrewTee", "Walk", Kind::Hanging, 18, 710, 0.0724),
            rec("Shirt_CrewTee", "Walk", Kind::Stretched, 18, 5060, 74.8476),
        ],
        &[
            rec("Pants_Joggers", "Walk", Kind::Folded, 18, 1624, 179.0768),
            rec("Pants_Joggers", "Walk", Kind::Hanging, 18, 9441, 0.0445),
            rec("Pants_Joggers", "Walk", Kind::Stretched, 18, 8974, 65.8678),
            rec("Shirt_CrewTee", "Walk", Kind::Folded, 18, 1478, 179.1325),
            rec("Shirt_CrewTee", "Walk", Kind::Hanging, 18, 1141, 0.0792),
            rec("Shirt_CrewTee", "Walk", Kind::Stretched, 18, 4923, 96.431),
        ],
    ];

    pub const STYLIZED_REST: [ClothRecord; 3] = [
        rec("Pants_Cargo", "rest", Kind::Hanging, 16, 5759, 0.0367),
        rec("Underwear_Briefs", "rest", Kind::Hanging, 10, 396, 0.0183),
        rec("Underwear_Top", "rest", Kind::Hanging, 8, 76, 0.0095),
    ];
    pub const STYLIZED_T_POSE: [ClothRecord; 6] = [
        rec("Pants_Cargo", "TPose", Kind::Hanging, 16, 5678, 0.0367),
        rec("Shirt_Tee", "TPose", Kind::Stretched, 16, 820, 19.858),
        rec("Underwear_Briefs", "TPose", Kind::Hanging, 10, 396, 0.0183),
        rec("Underwear_Top", "TPose", Kind::Folded, 2, 2, 105.7183),
        rec("Underwear_Top", "TPose", Kind::Hanging, 8, 90, 0.0115),
        rec("Underwear_Top", "TPose", Kind::Stretched, 16, 200, 1.8042),
    ];
    pub const STYLIZED_WALK: [ClothRecord; 11] = [
        rec("Pants_Cargo", "Walk@18", Kind::Folded, 16, 2064, 178.5789),
        rec("Pants_Cargo", "Walk@18", Kind::Hanging, 16, 8500, 0.0388),
        rec("Pants_Cargo", "Walk@18", Kind::Stretched, 16, 7458, 46.9636),
        rec("Shirt_Tee", "Walk@18", Kind::Folded, 16, 263, 175.4417),
        rec("Shirt_Tee", "Walk@18", Kind::Stretched, 16, 442, 19.7719),
        rec("Underwear_Briefs", "Walk@18", Kind::Folded, 16, 106, 166.0419),
        rec("Underwear_Briefs", "Walk@18", Kind::Hanging, 16, 1334, 0.0305),
        rec("Underwear_Briefs", "Walk@18", Kind::Stretched, 16, 1004, 7.969),
        rec("Underwear_Top", "Walk@18", Kind::Folded, 15, 18, 134.92),
        rec("Underwear_Top", "Walk@18", Kind::Hanging, 10, 155, 0.0138),
        rec("Underwear_Top", "Walk@18", Kind::Stretched, 4, 12, 1.7694),
    ];
}
