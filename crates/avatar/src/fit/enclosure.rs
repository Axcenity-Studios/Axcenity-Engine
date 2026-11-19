//! Hiding the body (and lower layers) under clothing (wave 5, stage 3):
//! the usual game approach. Skin and underwear a garment encloses aren't
//! drawn, so they can't come through it.
//!
//! **The rule** ([`hidden_under`]). A triangle of the body (or of a lower
//! layer) is hidden when every one of its corners
//!
//! 1. is **enclosed** by the garment at rest, on every body state: the
//!    way out along its normal meets the cloth from inside (the cloth's
//!    smooth normal and its triangle's face both point along the way out)
//!    within [`HideRule::enclose`];
//! 2. stays **near** the garment in every case (every pose on every
//!    state, and rest): the cloth within [`HideRule::near`];
//! 3. stays **clear of the openings** in every case: no point of the
//!    garment's open edges (neck, hems, sleeve ends, cuffs, waistband)
//!    within [`HideRule::opening`];
//! 4. **can't be seen** in any case ([`shows`]): from every view
//!    direction ([`Views`]: round the character, up and down, up and down
//!    each opening of the garment and the layers, and through each opening
//!    at the place itself), its place is behind cloth or drawn skin facing
//!    the viewer. Hiding is checked against what stays
//!    drawn, so a corner seen is drawn again, and drawing it only hides
//!    more of what is behind it: one pass is enough.
//!
//! The measures are [`HIDE_RULE`] (`docs/development/character-studio/
//! asset-quality.md` §3 says how they were set). Rule 4 is also the
//! check that proves no hole shows ([`holes_seen`]): the fit gate runs it
//! in the runtime's poses on every gate body, where the rule ran in the
//! fit's poses.
//!
//! **Masks** ([`write_mask`]). What is hidden is written as an image over
//! the mesh's texture coordinates (an item manifest's `hide_maps`), read
//! as the region map is (`axcenity_assets::RegionHiding::masks`): each
//! triangle's middle names its texel.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use axcenity_assets::BoneTransform;
use glam::Vec3;

use super::BodyMesh;
use super::pose::{FitPose, skin, skin_matrices};
use super::surface::{Hit, Surface};
use super::transfer::par_map;
use super::{FitError, write_png};

/// The rule's measures (metres).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HideRule {
    /// How far out along the skin's normal the cloth may be at rest and
    /// still enclose it.
    pub enclose: f32,
    /// The farthest the cloth may be from hidden skin in any case.
    pub near: f32,
    /// The nearest an opening of the garment may come to hidden skin in
    /// any case.
    pub opening: f32,
}

/// The rule as items are made with it (`refinement/wave5/body-hiding.md`
/// has the measurements). `enclose` is the coverage's reach
/// (`coverage::COVER_REACH`); `near` takes in a loose tee standing off
/// the small of the back; `opening` keeps the skin at a hem or sleeve end,
/// which moves over it between the fit's poses and the runtime's, drawn.
/// What a viewer could see is left to the views (rule 4), not to a wider
/// `opening`: a margin growing with the gap (5 times it, the steepest
/// graze counted) left 85 of the licensed tee's 289 hidden triangles and
/// still let the gate's walk see the belly up the hem, which only the
/// trim to the runtime's poses (`player` `hide_trim_tests`) caught.
pub const HIDE_RULE: HideRule = HideRule {
    enclose: 0.08,
    near: 0.06,
    opening: 0.04,
};

/// How far a hidden place is lifted towards the viewer before its ray is
/// cast, so it doesn't meet the triangles round it.
const LIFT: f32 = 0.0005;
/// How far a ray towards a viewer looks for something in the way.
const FAR: f32 = 3.0;
/// Cloth facing the viewer this close behind hidden skin is what is seen
/// where the skin was: the skin had come out through it.
pub const BEHIND: f32 = 0.03;
/// Hidden skin seen more than about 78 degrees from its normal (the
/// cosine below this) would be a sliver if it were drawn: its place isn't
/// counted as showing. (Without this, skin 10 cm up a sleeve counts as
/// seen at a graze through the gap between the arm and the cuff.)
pub const GRAZE: f32 = 0.2;
/// [`Views::fine`]'s graze.
pub const FINE_GRAZE: f32 = 0.05;
/// The spacing of the points the openings are kept as.
const OPENING_STEP: f32 = 0.005;
/// How far apart round an opening's rim the points a viewer looks
/// through it at are (the gate's; the trim's are half as far apart).
const APERTURE_STEP: f32 = 0.04;
/// How far from a hidden place an opening is looked through at it.
const APERTURE_REACH: f32 = 0.6;
/// An open edge loop shorter than this isn't an opening a viewer looks
/// into (a slit at a seam): it adds no view directions, but its points
/// still keep skin clear.
const OPENING_LEAST: f32 = 0.10;

/// A mesh as one case places it: its corners and their normals.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placed {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

/// One case the rule is checked in: the meshes under the garment (the
/// body first, then each lower layer) and the garment, all placed alike.
#[derive(Clone, Debug)]
pub struct HideCase {
    pub under: Vec<Placed>,
    pub garment: Placed,
    /// A rest case: the garment must enclose the hidden skin here.
    pub rest: bool,
}

/// Why corners stayed drawn: how many corners each rule kept out
/// (counted at the first rule that did), over every mesh under the
/// garment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HideReport {
    pub cases: usize,
    /// Corners enclosed at rest in every rest case.
    pub enclosed: usize,
    /// Of those, kept drawn because the cloth went farther than `near`.
    pub far: usize,
    /// ... because an opening came within `opening`.
    pub opening: usize,
    /// ... because they could be seen in a case.
    pub seen: usize,
    /// Triangles hidden, per mesh under the garment.
    pub hidden: Vec<usize>,
}

/// The triangles of each mesh under a garment the rule hides (one list
/// per mesh, in `under`'s order), with why the rest stay drawn.
/// `under` and `garment` are the meshes' triangles; `case(i)` places
/// everything for case `i` of `cases`. The garment's openings are its open
/// edges (corners welded where they meet).
pub fn hidden_under(
    under: &[&[[u32; 3]]],
    garment: &[[u32; 3]],
    cases: usize,
    case: impl Fn(usize) -> HideCase + Sync,
    rule: HideRule,
) -> (Vec<Vec<bool>>, HideReport) {
    let indices: Vec<usize> = (0..cases).collect();
    let mut report = HideReport {
        cases,
        ..HideReport::default()
    };
    // Rules 1 to 3, case by case: for each mesh, each corner's verdict
    // (`None`: not near the garment at all, so not looked at further).
    let verdicts = par_map(&indices, |&i| {
        let case = case(i);
        let surface = garment_surface(&case.garment, garment);
        let openings = Openings::new(&case.garment.positions, garment);
        case.under
            .iter()
            .map(|placed| {
                placed
                    .positions
                    .iter()
                    .zip(&placed.normals)
                    .map(|(p, n)| corner_verdict(&surface, &openings, *p, *n, case.rest, rule))
                    .collect::<Vec<Verdict>>()
            })
            .collect::<Vec<_>>()
    });
    // Each corner's worst verdict over the cases.
    let mut worst: Vec<Vec<Verdict>> = under
        .iter()
        .enumerate()
        .map(|(m, _)| vec![Verdict::Hide; verdicts.first().map_or(0, |v| v[m].len())])
        .collect();
    for case in &verdicts {
        for (m, corners) in case.iter().enumerate() {
            for (k, verdict) in corners.iter().enumerate() {
                worst[m][k] = worst[m][k].max(*verdict);
            }
        }
    }
    drop(verdicts);
    for corners in &worst {
        for verdict in corners {
            match verdict {
                Verdict::Hide => report.enclosed += 1,
                Verdict::Far => {
                    report.enclosed += 1;
                    report.far += 1;
                }
                Verdict::Opening => {
                    report.enclosed += 1;
                    report.opening += 1;
                }
                Verdict::Open => {}
            }
        }
    }
    let candidates: Vec<Vec<bool>> = worst
        .iter()
        .map(|corners| corners.iter().map(|v| *v == Verdict::Hide).collect())
        .collect();
    let hidden_by = |corners: &[bool], triangles: &[[u32; 3]]| -> Vec<bool> {
        triangles
            .iter()
            .map(|t| t.iter().all(|v| corners[*v as usize]))
            .collect()
    };
    let first: Vec<Vec<bool>> = under
        .iter()
        .zip(&candidates)
        .map(|(triangles, corners)| hidden_by(corners, triangles))
        .collect();

    // Rule 4: every candidate corner, and the middle of every triangle
    // the first three rules hide, unseen in every case, with what they
    // hide left undrawn.
    let seen_by_case = par_map(&indices, |&i| {
        let case = case(i);
        let cloth = garment_surface(&case.garment, garment);
        // Every opening a viewer could look in by: the garment's, and
        // the lower layers' (a bra's lower edge seen up a tee's hem).
        let mut all = vec![Openings::new(&case.garment.positions, garment)];
        for (placed, triangles) in case.under.iter().zip(under).skip(1) {
            all.push(Openings::new(&placed.positions, triangles));
        }
        let views = Views::new(&all.iter().collect::<Vec<_>>());
        let drawn: Vec<Surface> = case
            .under
            .iter()
            .zip(under)
            .zip(&first)
            .map(|((placed, triangles), hidden)| drawn_surface(placed, triangles, hidden))
            .collect();
        // The cloth first: most ways out meet it within a few
        // centimetres, and the rest are cast through the skin.
        let mut occluders: Vec<&Surface> = vec![&cloth];
        occluders.extend(drawn.iter());
        let seen = |p: Vec3, n: Vec3| views.from(p).any(|d| shows(p, n, d, &occluders, &[&cloth]));
        case.under
            .iter()
            .zip(under)
            .zip(candidates.iter().zip(&first))
            .map(|((placed, triangles), (corners, hidden))| {
                let corners_seen: Vec<usize> = corners
                    .iter()
                    .enumerate()
                    .filter(|(k, c)| **c && seen(placed.positions[*k], placed.normals[*k]))
                    .map(|(k, _)| k)
                    .collect();
                let middles_seen: Vec<usize> = triangles
                    .iter()
                    .zip(hidden)
                    .enumerate()
                    .filter(|(_, (t, h))| {
                        **h && {
                            let (p, n) = middle(placed, t);
                            seen(p, n)
                        }
                    })
                    .map(|(t, _)| t)
                    .collect();
                (corners_seen, middles_seen)
            })
            .collect::<Vec<_>>()
    });
    let mut kept = candidates;
    let mut middles_seen: Vec<Vec<bool>> = first.iter().map(|h| vec![false; h.len()]).collect();
    for case in seen_by_case {
        for (m, (corners, middles)) in case.into_iter().enumerate() {
            for k in corners {
                if kept[m][k] {
                    kept[m][k] = false;
                    report.seen += 1;
                }
            }
            for t in middles {
                middles_seen[m][t] = true;
            }
        }
    }
    let hidden: Vec<Vec<bool>> = under
        .iter()
        .zip(&kept)
        .zip(&middles_seen)
        .map(|((triangles, corners), middles)| {
            hidden_by(corners, triangles)
                .into_iter()
                .zip(middles)
                .map(|(h, seen)| h && !seen)
                .collect()
        })
        .collect();
    report.hidden = hidden
        .iter()
        .map(|h| h.iter().filter(|x| **x).count())
        .collect();
    (hidden, report)
}

/// The middle of triangle `t` of `placed` and its normal (its corners'
/// mixed).
fn middle(placed: &Placed, t: &[u32; 3]) -> (Vec3, Vec3) {
    let p = t
        .iter()
        .map(|v| placed.positions[*v as usize])
        .sum::<Vec3>()
        / 3.0;
    let n = t
        .iter()
        .map(|v| placed.normals[*v as usize])
        .sum::<Vec3>()
        .normalize_or_zero();
    (p, n)
}

/// A corner's verdict in one case, worst last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Verdict {
    Hide,
    /// The cloth went farther than `near`.
    Far,
    /// An opening came within `opening`.
    Opening,
    /// Not enclosed at rest (or not near anything).
    Open,
}

fn corner_verdict(
    cloth: &Surface,
    openings: &Openings,
    p: Vec3,
    n: Vec3,
    rest: bool,
    rule: HideRule,
) -> Verdict {
    if rest && !encloses(cloth, p, n, rule.enclose) {
        return Verdict::Open;
    }
    if cloth.closest(p, rule.near).is_none() {
        return Verdict::Far;
    }
    if openings.within(p, rule.opening) {
        return Verdict::Opening;
    }
    Verdict::Hide
}

/// Whether `cloth` encloses the skin at `p` (normal `n`): the way out
/// along `n` meets, within `reach`, cloth whose triangle faces along the
/// way out, its smooth normal too (the garment's outside, seen from
/// outside). Cloth facing back towards the skin on the way (a lining, the
/// inner wall of cloth made with thickness, a fold flipped towards the
/// skin) is passed through: from outside it shows the garment's inside
/// (culled for a single-sided garment, the inside face for a double-sided
/// one), which encloses nothing.
pub fn encloses(cloth: &Surface, p: Vec3, n: Vec3, reach: f32) -> bool {
    cloth
        .cast_where(p, n, reach, |t| face_of(cloth, t).dot(n) > 0.0)
        .is_some_and(|hit| hit.normal.dot(n) > 0.0)
}

/// A triangle's face normal (not unit length).
fn face(surface: &Surface, hit: &Hit) -> Vec3 {
    face_of(surface, hit.triangle)
}

fn face_of(surface: &Surface, triangle: u32) -> Vec3 {
    let [a, b, c] = surface.triangles[triangle as usize].map(|v| surface.positions[v as usize]);
    (b - a).cross(c - a)
}

/// Whether the place of hidden skin at `p` (normal `n`) shows from the
/// direction `toward` (one unit long, pointing at the viewer), with
/// `drawn` the surfaces drawn (the garments, the skin and layers left
/// drawn) and `cloth` the garments.
///
/// Skin facing away from the viewer would be culled even drawn, so its
/// place shows nothing new; skin seen at a graze ([`GRAZE`]) is left out
/// too. Otherwise the place is covered when the way to the viewer meets
/// a drawn triangle facing the viewer. A triangle facing away doesn't
/// cover: a fold flipped towards the skin shows the garment's inside, a
/// hole by the brief's definition whether its back faces are drawn
/// (`doubleSided`, the crew tee and the joggers) or culled. Where it
/// doesn't, what is
/// seen is what lies behind the place:
///
/// - cloth facing the viewer within [`BEHIND`]: the skin had come out
///   through it, and the cloth is seen where the skin was;
/// - cloth facing away (the inside of a garment): a hole;
/// - nothing within [`BEHIND`]: covered only where the skin is out
///   through the cloth by no more than that, the cloth behind it as the
///   viewer sees it (the cloth's own outline is seen, where the skin
///   would have stood past it); otherwise a hole.
pub fn shows(p: Vec3, n: Vec3, toward: Vec3, drawn: &[&Surface], cloth: &[&Surface]) -> bool {
    shows_at(p, n, toward, drawn, cloth, GRAZE)
}

/// [`shows`], counting the place from `graze` (a cosine) instead of
/// [`GRAZE`].
pub fn shows_at(
    p: Vec3,
    n: Vec3,
    toward: Vec3,
    drawn: &[&Surface],
    cloth: &[&Surface],
    graze: f32,
) -> bool {
    if n.dot(toward) <= graze {
        return false;
    }
    let origin = p + toward * LIFT;
    if drawn.iter().any(|s| {
        s.cast_where(origin, toward, FAR, facing(s, toward))
            .is_some()
    }) {
        return false;
    }
    let behind = cloth
        .iter()
        .filter_map(|s| {
            s.cast_where(p, -toward, BEHIND, |_| true)
                .map(|hit| (hit.distance, face(s, &hit).dot(toward) > 0.0))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0));
    match behind {
        Some((_, facing_viewer)) => !facing_viewer,
        None => !cloth.iter().any(|s| {
            s.closest(p, BEHIND).is_some_and(|hit| {
                hit.signed(p) > 0.0 && hit.is_over(p) && (hit.point - p).dot(toward) <= 0.0
            })
        }),
    }
}

/// Which triangles of `surface` face a viewer in the direction `toward`.
fn facing(surface: &Surface, toward: Vec3) -> impl Fn(u32) -> bool + '_ {
    move |t| face_of(surface, t).dot(toward) > 0.0
}

/// A hidden place that shows ([`holes_seen`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeenHole {
    pub at: Vec3,
    /// The direction it shows from (towards the viewer).
    pub toward: Vec3,
    /// The place: a corner (its index over the mesh), or the middle of a
    /// triangle (its index in the `hidden` list checked).
    pub corner: Option<u32>,
    pub middle: Option<usize>,
}

/// The check that proves no hole shows: the corners and middles of the
/// `hidden` triangles (over `positions` and `normals`) that show
/// ([`shows`]) from any of `views`' directions, one entry per place (the
/// first direction it shows from).
pub fn holes_seen(
    hidden: &[[u32; 3]],
    positions: &[Vec3],
    normals: &[Vec3],
    drawn: &[&Surface],
    cloth: &[&Surface],
    views: &Views,
) -> Vec<SeenHole> {
    let mut corners: Vec<u32> = hidden.iter().flatten().copied().collect();
    corners.sort_unstable();
    corners.dedup();
    let mut places: Vec<(Vec3, Vec3, Option<u32>, Option<usize>)> = corners
        .iter()
        .map(|k| (positions[*k as usize], normals[*k as usize], Some(*k), None))
        .collect();
    places.extend(hidden.iter().enumerate().map(|(i, t)| {
        let p = t.iter().map(|v| positions[*v as usize]).sum::<Vec3>() / 3.0;
        let n = t
            .iter()
            .map(|v| normals[*v as usize])
            .sum::<Vec3>()
            .normalize_or_zero();
        (p, n, None, Some(i))
    }));
    par_map(&places, |(p, n, corner, middle)| {
        views
            .from(*p)
            .find(|d| shows_at(*p, *n, *d, drawn, cloth, views.graze))
            .map(|d| SeenHole {
                at: *p,
                toward: d,
                corner: *corner,
                middle: *middle,
            })
    })
    .into_iter()
    .flatten()
    .collect()
}

/// The directions a hidden place is looked at from (towards the viewer):
/// 64 spread evenly over the sphere (a Fibonacci spiral: round the
/// character, from above and from below), the capture matrix's front,
/// three-quarter, side and back views either way round (in the model
/// file's axes, the character facing +Z), and for each opening's axis
/// (`axes`), along it either way and six ways round it at 25 degrees.
/// The gate's hole check looks from these.
pub fn view_directions(axes: &[Vec3]) -> Vec<Vec3> {
    directions(axes, 64, &[25.0], 6)
}

/// Where hidden places are looked at from: [`view_directions`] of the
/// openings' axes, and through each opening at the place itself (towards
/// each of the openings' aperture points within [`APERTURE_REACH`]: the
/// lines of sight in by an opening, whatever its shape).
#[derive(Clone, Debug, Default)]
pub struct Views {
    pub directions: Vec<Vec3>,
    pub apertures: Vec<Vec3>,
    /// From what cosine to the place's normal it counts ([`GRAZE`]).
    pub graze: f32,
}

impl Views {
    /// The views of the openings `openings` (of every garment and layer
    /// in the scene), as the gate looks: [`view_directions`], and through
    /// each opening at points [`APERTURE_STEP`] apart round its rim, a
    /// quarter of the way in to its middle, and its middle.
    pub fn new(openings: &[&Openings]) -> Views {
        let axes: Vec<Vec3> = openings.iter().flat_map(|o| o.axes.clone()).collect();
        Views {
            directions: view_directions(&axes),
            apertures: apertures(openings, APERTURE_STEP, &[0.25]),
            graze: GRAZE,
        }
    }

    /// More views than [`new`](Self::new), for trimming masks to the
    /// runtime (`player` `hide_trim_tests`): those, 256 more directions
    /// over the sphere, and through each opening at points 2 cm apart round
    /// its rim and 2, 6, 12, 25, 40 and 60 hundredths of the way in (a view along the cloth's inside, just past a hem's edge,
    /// is one the gate's samples can pass between), counting places seen
    /// from 87 degrees off their normal (a cosine of 0.05: a close camera
    /// sees a hidden waistband's rim at a graze as a sliver).
    pub fn fine(openings: &[&Openings]) -> Views {
        let mut views = Views::new(openings);
        views.graze = FINE_GRAZE;
        views.directions.extend(directions(&[], 256, &[], 0));
        views.apertures.extend(apertures(
            openings,
            APERTURE_STEP / 2.0,
            &[0.02, 0.06, 0.12, 0.25, 0.4, 0.6],
        ));
        views
    }

    /// The directions (towards the viewer) a place at `p` is looked at
    /// from.
    pub fn from(&self, p: Vec3) -> impl Iterator<Item = Vec3> + '_ {
        let through = self.apertures.iter().filter_map(move |a| {
            let d = *a - p;
            let length = d.length();
            (length > 1e-4 && length <= APERTURE_REACH).then(|| d / length)
        });
        self.directions.iter().copied().chain(through)
    }
}

/// Points across `openings`: each loop's middle, and round its rim
/// every `step` at each of `inward` (shares of the way to the middle).
fn apertures(openings: &[&Openings], step: f32, inward: &[f32]) -> Vec<Vec3> {
    let mut out = Vec::new();
    for (centre, rim) in openings.iter().flat_map(|o| &o.loops) {
        out.push(*centre);
        let mut last: Option<Vec3> = None;
        for q in rim {
            if last.is_some_and(|l| l.distance(*q) < step) {
                continue;
            }
            last = Some(*q);
            out.extend(inward.iter().map(|share| q.lerp(*centre, *share)));
        }
    }
    out
}

fn directions(axes: &[Vec3], sphere: usize, tilts: &[f32], around: usize) -> Vec<Vec3> {
    let mut out = Vec::new();
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    for i in 0..sphere {
        let y = 1.0 - 2.0 * (i as f32 + 0.5) / sphere as f32;
        let r = (1.0 - y * y).max(0.0).sqrt();
        let a = golden * i as f32;
        out.push(Vec3::new(r * a.cos(), y, r * a.sin()));
    }
    let lift = 0.03;
    for (x, z) in [
        (0.0, 1.0),
        (0.0, -1.0),
        (1.0, 0.0),
        (-1.0, 0.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
        (-1.0, -1.0),
    ] {
        out.push(Vec3::new(x, lift, z).normalize());
    }
    for axis in axes {
        for sign in [1.0f32, -1.0] {
            let axis = (*axis * sign).normalize_or_zero();
            if axis == Vec3::ZERO {
                continue;
            }
            out.push(axis);
            let side = axis.any_orthonormal_vector();
            for degrees in tilts {
                let tilted = glam::Quat::from_axis_angle(side, degrees.to_radians()) * axis;
                for k in 0..around {
                    let turn = glam::Quat::from_axis_angle(
                        axis,
                        std::f32::consts::TAU * k as f32 / around as f32,
                    );
                    out.push(turn * tilted);
                }
            }
        }
    }
    out
}

/// A garment's openings in one case: points along its open edges (for
/// the distance rule) and the axis of each open loop long enough to look
/// into (for the view directions).
#[derive(Clone, Debug, Default)]
pub struct Openings {
    /// Points along the open edges, a few millimetres apart.
    points: Vec<Vec3>,
    /// Each opening's axis: the normal of its loop's best plane.
    pub axes: Vec<Vec3>,
    /// Each opening long enough to look into: its middle and its rim's
    /// corners (in the order its edges were joined).
    pub loops: Vec<(Vec3, Vec<Vec3>)>,
}

impl Openings {
    /// The openings of the mesh `triangles` over `positions` (corners at
    /// the same place, within a hundredth of a millimetre, are one).
    pub fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Openings {
        let weld = welded(positions);
        let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for t in triangles {
            let t = t.map(|v| weld[v as usize]);
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                if a != b {
                    *uses.entry((a.min(b), a.max(b))).or_default() += 1;
                }
            }
        }
        let edges: Vec<(u32, u32)> = uses
            .into_iter()
            .filter(|(_, n)| *n == 1)
            .map(|(e, _)| e)
            .collect();
        let mut openings = Openings::default();
        for &(a, b) in &edges {
            let (pa, pb) = (positions[a as usize], positions[b as usize]);
            let steps = (pa.distance(pb) / OPENING_STEP).ceil().max(1.0) as usize;
            for s in 0..=steps {
                openings.points.push(pa.lerp(pb, s as f32 / steps as f32));
            }
        }
        // The loops: the open edges joined at their corners.
        let mut next: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &(a, b) in &edges {
            next.entry(a).or_default().push(b);
            next.entry(b).or_default().push(a);
        }
        let mut done: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &start in next.keys() {
            if done.contains(&start) {
                continue;
            }
            let mut stack = vec![start];
            let mut members = Vec::new();
            while let Some(v) = stack.pop() {
                if !done.insert(v) {
                    continue;
                }
                members.push(v);
                stack.extend(next[&v].iter().copied().filter(|w| !done.contains(w)));
            }
            let length: f32 = edges
                .iter()
                .filter(|(a, _)| members.contains(a))
                .map(|(a, b)| positions[*a as usize].distance(positions[*b as usize]))
                .sum();
            if length < OPENING_LEAST || members.len() < 3 {
                continue;
            }
            openings.axes.push(loop_axis(positions, &members));
            let centre =
                members.iter().map(|v| positions[*v as usize]).sum::<Vec3>() / members.len() as f32;
            let rim = members.iter().map(|v| positions[*v as usize]).collect();
            openings.loops.push((centre, rim));
        }
        // Openings looked into the same way (a turned hem's inner edge
        // beside its fold) add nothing: one axis for those within 10
        // degrees of each other.
        let mut unique: Vec<Vec3> = Vec::new();
        for axis in std::mem::take(&mut openings.axes) {
            if unique
                .iter()
                .all(|u| u.dot(axis).abs() < 10f32.to_radians().cos())
            {
                unique.push(axis);
            }
        }
        openings.axes = unique;
        openings.axes.sort_by(|a, b| {
            a.to_array()
                .partial_cmp(&b.to_array())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        openings
    }

    /// Whether a point of an opening is within `reach` of `p`. (A
    /// garment's openings are a few thousand points at most: each is
    /// tried.)
    pub fn within(&self, p: Vec3, reach: f32) -> bool {
        let reach_sq = reach * reach;
        self.points
            .iter()
            .any(|q| q.distance_squared(p) <= reach_sq)
    }
}

/// The normal of the plane that fits the points `members` best (Newell's
/// method over their order round the centre).
fn loop_axis(positions: &[Vec3], members: &[u32]) -> Vec3 {
    let points: Vec<Vec3> = members.iter().map(|v| positions[*v as usize]).collect();
    let centre = points.iter().sum::<Vec3>() / points.len() as f32;
    // The loop's two widest directions span its plane: the farthest point
    // from the centre, and the farthest from the line through it.
    let far = points
        .iter()
        .copied()
        .max_by(|a, b| {
            a.distance_squared(centre)
                .total_cmp(&b.distance_squared(centre))
        })
        .unwrap_or(centre);
    let u = (far - centre).normalize_or_zero();
    let across = |p: &Vec3| {
        let d = *p - centre;
        (d - u * d.dot(u)).length_squared()
    };
    let other = points
        .iter()
        .copied()
        .max_by(|a, b| across(a).total_cmp(&across(b)))
        .unwrap_or(centre);
    let d = other - centre;
    let v = (d - u * d.dot(u)).normalize_or_zero();
    let axis = u.cross(v).normalize_or_zero();
    // Either way along it is the same opening: the way whose largest
    // part is positive, so the same loop always gives the same axis.
    let largest = axis.abs().max_element();
    let lead = [axis.x, axis.y, axis.z]
        .into_iter()
        .find(|c| c.abs() == largest)
        .unwrap_or(1.0);
    if lead < 0.0 { -axis } else { axis }
}

/// Each corner's first corner at the same place (within a hundredth of a
/// millimetre).
fn welded(positions: &[Vec3]) -> Vec<u32> {
    let mut first: HashMap<[i64; 3], u32> = HashMap::new();
    positions
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let key = (*p * 1e5).round().as_i64vec3().to_array();
            *first.entry(key).or_insert(k as u32)
        })
        .collect()
}

fn garment_surface(placed: &Placed, triangles: &[[u32; 3]]) -> Surface {
    Surface::new(
        placed.positions.clone(),
        placed.normals.clone(),
        triangles.to_vec(),
    )
}

fn drawn_surface(placed: &Placed, triangles: &[[u32; 3]], hidden: &[bool]) -> Surface {
    Surface::new(
        placed.positions.clone(),
        placed.normals.clone(),
        triangles
            .iter()
            .zip(hidden)
            .filter(|(_, h)| !**h)
            .map(|(t, _)| *t)
            .collect(),
    )
}

/// What [`hide_under`] works on: the base body, the garment and the
/// layers worn under it, as their models load
/// ([`BodyMesh::from_model`]), with the poses (on the body's rig) and
/// body states they are checked in.
pub struct HideInput<'a> {
    pub body: &'a BodyMesh,
    pub garment: &'a BodyMesh,
    pub layers: &'a [&'a BodyMesh],
    pub poses: &'a [FitPose],
    /// Shape weights of each body state ([`super::pose_states_of`]).
    pub states: &'a [Vec<(String, f32)>],
    /// Each layer's mask shapes ([`layer_masks`]): the body wears those of
    /// the layers worn, which sink the skin under them as the runtime
    /// does, opening a gap a viewer may see along.
    pub layer_shapes: &'a [Vec<(String, f32)>],
    /// Which layers are worn together (indices of `layers`: the
    /// feminine frame's bra and briefs, the masculine frame's boxer
    /// briefs). Each is checked on its own, and a mesh is hidden where
    /// every outfit wearing it hides it. Empty: all of them together.
    pub outfits: &'a [Vec<usize>],
    pub rule: HideRule,
}

/// The mask shapes (at full weight) of the regions `layers` (item IDs of
/// `library`) hide on `base`: what the body wears under them at run time
/// (`resolve`'s identity).
pub fn layer_masks(
    base: &crate::BaseManifest,
    library: &crate::Library,
    layers: &[String],
) -> Vec<(String, f32)> {
    let mut masks: Vec<(String, f32)> = layers
        .iter()
        .filter_map(|id| library.item(id))
        .flat_map(|item| item.hides.iter())
        .filter_map(|region| base.region(region)?.mask_shape.clone())
        .map(|shape| (shape, 1.0))
        .collect();
    masks.sort_by(|a, b| a.0.cmp(&b.0));
    masks.dedup();
    masks
}

/// [`hidden_under`] for a garment over the body and `layers`, at rest and
/// in every pose on every state: per mesh (the body, then each layer),
/// each triangle hidden or not, and the report.
pub fn hide_under(input: &HideInput) -> (Vec<Vec<bool>>, HideReport) {
    let all: Vec<usize> = (0..input.layers.len()).collect();
    let outfits: Vec<&[usize]> = if input.outfits.is_empty() {
        vec![&all]
    } else {
        input.outfits.iter().map(Vec::as_slice).collect()
    };
    // Per outfit, then combined: hidden only where every outfit that
    // wears the mesh hides it.
    let mut hidden: Vec<Option<Vec<bool>>> = vec![None; 1 + input.layers.len()];
    let mut report = HideReport::default();
    for outfit in outfits {
        let (one, part) = hide_under_outfit(input, outfit);
        let meshes = std::iter::once(0).chain(outfit.iter().map(|k| k + 1));
        for (mesh, triangles) in meshes.zip(one) {
            hidden[mesh] = Some(match hidden[mesh].take() {
                Some(before) => before
                    .iter()
                    .zip(&triangles)
                    .map(|(a, b)| *a && *b)
                    .collect(),
                None => triangles,
            });
        }
        report.cases += part.cases;
        report.enclosed += part.enclosed;
        report.far += part.far;
        report.opening += part.opening;
        report.seen += part.seen;
    }
    let meshes = std::iter::once(input.body).chain(input.layers.iter().copied());
    let hidden: Vec<Vec<bool>> = hidden
        .into_iter()
        .zip(meshes)
        .map(|(h, mesh)| h.unwrap_or_else(|| vec![false; mesh.triangles.len()]))
        .collect();
    report.hidden = hidden
        .iter()
        .map(|h| h.iter().filter(|x| **x).count())
        .collect();
    (hidden, report)
}

/// [`hidden_under`] for the garment over the body and the layers
/// `outfit` (indices of `input.layers`) worn together, the body wearing
/// their mask shapes: per mesh (the body, then each of them).
fn hide_under_outfit(input: &HideInput, outfit: &[usize]) -> (Vec<Vec<bool>>, HideReport) {
    let mut meshes: Vec<&BodyMesh> = vec![input.body];
    meshes.extend(outfit.iter().map(|k| input.layers[*k]));
    let triangles: Vec<&[[u32; 3]]> = meshes.iter().map(|m| m.triangles.as_slice()).collect();
    let body_shapes: Vec<(String, f32)> = outfit
        .iter()
        .filter_map(|k| input.layer_shapes.get(*k))
        .flatten()
        .cloned()
        .collect();
    let cases: Vec<(Option<usize>, usize)> = std::iter::once(None)
        .chain((0..input.poses.len()).map(Some))
        .flat_map(|p| (0..input.states.len().max(1)).map(move |s| (p, s)))
        .collect();
    let case = |i: usize| {
        let (pose, state) = cases[i];
        let pose = pose.map(|p| &input.poses[p]);
        let mut weights = input.states.get(state).cloned().unwrap_or_default();
        if let Some(pose) = pose {
            for (name, w) in &pose.correctives {
                match weights.iter_mut().find(|(n, _)| n == name) {
                    Some(entry) => entry.1 += *w,
                    None => weights.push((name.clone(), *w)),
                }
            }
        }
        let place = |mesh: &BodyMesh| placed(mesh, &input.body.rig, &weights, pose);
        let mut body_weights = weights.clone();
        body_weights.extend(body_shapes.iter().cloned());
        let body = placed(input.body, &input.body.rig, &body_weights, pose);
        HideCase {
            under: std::iter::once(body)
                .chain(meshes.iter().skip(1).map(|m| place(m)))
                .collect(),
            garment: place(input.garment),
            rest: pose.is_none(),
        }
    };
    hidden_under(
        &triangles,
        &input.garment.triangles,
        cases.len(),
        case,
        input.rule,
    )
}

/// `mesh` with the shape `weights`, posed by `pose` (bones of `rig`, the
/// body's, matched to the mesh's own rig by name) or at rest.
fn placed(
    mesh: &BodyMesh,
    rig: &axcenity_assets::Rig,
    weights: &[(String, f32)],
    pose: Option<&FitPose>,
) -> Placed {
    let (positions, normals) = mesh.shaped(weights);
    let Some(pose) = pose else {
        return Placed { positions, normals };
    };
    let bones: Vec<BoneTransform> = if mesh.rig.bones.len() == rig.bones.len()
        && mesh
            .rig
            .bones
            .iter()
            .zip(&rig.bones)
            .all(|(a, b)| a.name == b.name)
    {
        pose.bones.clone()
    } else {
        mesh.rig
            .bones
            .iter()
            .map(|bone| {
                rig.bone(&bone.name)
                    .and_then(|k| pose.bones.get(k).copied())
                    .unwrap_or(bone.rest)
            })
            .collect()
    };
    let matrices = skin_matrices(&mesh.rig, &bones);
    let (positions, normals, _) =
        skin(&matrices, &mesh.joints, &mesh.weights, &positions, &normals);
    Placed { positions, normals }
}

/// The texels of a `size` by `size` mask of `hidden` (one per triangle of
/// `triangles` over the texture coordinates `uvs`): 255 hides, 0 draws.
/// Every triangle's middle reads its own value, as the engine reads it
/// (`axcenity_assets::Model::without_triangles`); a texel two triangles'
/// middles share draws. Elsewhere hidden triangles are filled in and
/// drawn ones, one texel wider, are filled over them, so a simpler copy's
/// triangle (a level of detail) is hidden only well inside hidden skin.
pub fn mask_texels(
    size: u32,
    uvs: &[[f32; 2]],
    triangles: &[[u32; 3]],
    hidden: &[bool],
) -> Vec<u8> {
    let mut texels = vec![0u8; (size * size) as usize];
    if uvs.is_empty() {
        return texels;
    }
    let s = size as f32;
    let fill = |texels: &mut Vec<u8>, t: &[u32; 3], value: u8, grow: f32| {
        let [a, b, c] = t.map(|v| {
            let uv = uvs[v as usize];
            glam::Vec2::new(uv[0] * s, uv[1] * s)
        });
        let lo = a.min(b).min(c) - glam::Vec2::splat(grow + 1.0);
        let hi = a.max(b).max(c) + glam::Vec2::splat(grow + 1.0);
        let area = (b - a).perp_dot(c - a);
        if area.abs() < 1e-12 {
            return;
        }
        // A texel's centre in the triangle, or within `grow` texels of an
        // edge.
        let edge = |p: glam::Vec2, u: glam::Vec2, v: glam::Vec2| {
            let along = v - u;
            along.perp_dot(p - u) / along.length().max(1e-12) * area.signum()
        };
        for y in (lo.y.floor().max(0.0) as u32)..(hi.y.ceil().min(s) as u32) {
            for x in (lo.x.floor().max(0.0) as u32)..(hi.x.ceil().min(s) as u32) {
                let p = glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                if edge(p, a, b) >= -grow && edge(p, b, c) >= -grow && edge(p, c, a) >= -grow {
                    texels[(y * size + x) as usize] = value;
                }
            }
        }
    };
    for (t, h) in triangles.iter().zip(hidden) {
        if *h {
            fill(&mut texels, t, 255, 0.0);
        }
    }
    for (t, h) in triangles.iter().zip(hidden) {
        if !*h {
            fill(&mut texels, t, 0, 1.0);
        }
    }
    // The middles last, the drawn ones over the hidden.
    let texel = |t: &[u32; 3]| {
        let uv = t.iter().fold([0.0f32; 2], |sum, &v| {
            let uv = uvs[v as usize];
            [sum[0] + uv[0] / 3.0, sum[1] + uv[1] / 3.0]
        });
        let x = ((uv[0].rem_euclid(1.0)) * s) as u32;
        let y = ((uv[1].rem_euclid(1.0)) * s) as u32;
        (y.min(size - 1) * size + x.min(size - 1)) as usize
    };
    for draw in [false, true] {
        for (t, h) in triangles.iter().zip(hidden) {
            if *h != draw {
                texels[texel(t)] = if draw { 0 } else { 255 };
            }
        }
    }
    texels
}

/// The width of a mask by number ([`numbered_texels`]).
pub const NUMBERED_WIDTH: u32 = 256;

/// The texels of a mask by number, for a mesh without texture
/// coordinates: triangle `k` is texel `k` in reading order,
/// [`NUMBERED_WIDTH`] a row (`axcenity_assets::RegionHiding::masks`).
/// Returns (height, texels).
pub fn numbered_texels(hidden: &[bool]) -> (u32, Vec<u8>) {
    let height = (hidden.len() as u32).div_ceil(NUMBERED_WIDTH).max(1);
    let mut texels = vec![0u8; (NUMBERED_WIDTH * height) as usize];
    for (k, h) in hidden.iter().enumerate() {
        if *h {
            texels[k] = 255;
        }
    }
    (height, texels)
}

/// Writes the mask of `hidden` as a PNG at `path`: over the texture
/// coordinates `uvs`, `size` a side ([`mask_texels`]), or by number when
/// the mesh has none ([`numbered_texels`]).
pub fn write_mask(
    path: &Path,
    size: u32,
    uvs: &[[f32; 2]],
    triangles: &[[u32; 3]],
    hidden: &[bool],
) -> Result<(), FitError> {
    let (width, height, texels) = if uvs.is_empty() {
        let (height, texels) = numbered_texels(hidden);
        (NUMBERED_WIDTH, height, texels)
    } else {
        (size, size, mask_texels(size, uvs, triangles, hidden))
    };
    let rgba: Vec<u8> = texels.iter().flat_map(|v| [*v, *v, *v, 255]).collect();
    write_png(path, width, height, &rgba)
}

/// The size of a body's mask, and of a lower layer's (texels a side).
pub const BODY_MASK: u32 = 2048;
pub const LAYER_MASK: u32 = 1024;

/// Writes the masks of `hidden` (from [`hide_under`]: the body, then each
/// of `layers`, by item ID) into `folder` beside an item whose model is
/// `object`: `<object>.hide.png` for the body and
/// `<object>.hide.<layer>.png` for each layer (its ID's last word), each
/// only when it hides something. Returns the manifest's `hide_maps`.
pub fn write_hide_maps(
    folder: &Path,
    object: &str,
    body: &BodyMesh,
    layers: &[(&str, &BodyMesh)],
    hidden: &[Vec<bool>],
) -> Result<Vec<crate::HideMap>, FitError> {
    let mut maps = Vec::new();
    let meshes = std::iter::once((None, body, BODY_MASK)).chain(
        layers
            .iter()
            .map(|(id, mesh)| (Some(*id), *mesh, LAYER_MASK)),
    );
    for ((under, mesh, size), hidden) in meshes.zip(hidden) {
        if !hidden.iter().any(|h| *h) {
            continue;
        }
        let name = match under {
            None => format!("{object}.hide.png"),
            Some(id) => format!("{object}.hide.{}.png", id.rsplit('/').next().unwrap_or(id)),
        };
        write_mask(
            &folder.join(&name),
            size,
            &mesh.uvs,
            &mesh.triangles,
            hidden,
        )?;
        maps.push(crate::HideMap {
            under: under.map(str::to_owned),
            map: name,
        });
    }
    Ok(maps)
}

#[cfg(test)]
#[path = "enclosure_tests.rs"]
mod tests;
