//! Mending: the holes in a garment's cloth filled in.
//!
//! A garment that wasn't made on the body comes with holes (a generated
//! mesh has cracks of its own, taking off its inner wall leaves more, and
//! so do thinning it and cutting the strings and webs a hanging sleeve
//! was joined to its side by). Each shows the skin, the underwear or the
//! room behind the character, from everywhere.
//!
//! An open loop of the cloth is either an **opening**, which the body
//! comes through (a neck, a cuff, a hem, an open front), or a **hole**.
//! It is an opening when it is long ([`MEND_MOST`] round), when a bone
//! passes through it, or when its middle lies inside the body; anything
//! else is a hole and is closed with triangles between its own corners
//! (no corner is added, so skin weights, carried shapes and the binding
//! stay as they are; the patch takes its edge's texture coordinates). No
//! patch triangle is longer than [`SPAN`]: a longer one would join cloth
//! that was apart.
//!
//! The intake's own cuts are closed apart from that ([`close_cuts`]),
//! right after it makes them and knowing which cloth each rim is.

use std::collections::BTreeMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::surface::Surface;
use super::{BodyMesh, GarmentMesh};

/// An open loop longer round than this is an opening, never a hole
/// (metres): a wrist cuff is about 0.2 round, and so is a bad tear.
const MEND_MOST: f32 = 0.6;

/// A patch's triangle has no side longer than this (metres): a longer one
/// isn't over a hole but joins cloth that was apart (a sleeve to the side
/// it hangs beside).
const SPAN: f32 = 0.12;

/// The same for the cuts the intake makes itself ([`close_cuts`]): the
/// underside of a sleeve is as wide as the sleeve.
const CUT_SPAN: f32 = 0.2;

/// An ear along a slit's rim whose least angle is under this is three
/// corners in a line (radians).
const SLIVER: f32 = 0.035;

/// What a ring must be to be patched, and how.
#[derive(Clone, Copy)]
pub(super) struct Limits {
    /// A ring longer round than this is left alone (metres).
    pub round: f32,
    /// No side of a patch's triangle is longer than this (metres).
    pub span: f32,
    /// The ring is flat enough to say which of its corners point into
    /// it: ears that bulge the other way, or hold another corner, are cut
    /// last. Off for a long slit that winds round the garment, where
    /// only the shape of each ear counts.
    pub flat: bool,
}

impl Limits {
    /// A hole in the cloth.
    pub const HOLE: Limits = Limits {
        round: MEND_MOST,
        span: SPAN,
        flat: true,
    };
}

/// A loop whose middle lies deeper inside the body than this is an
/// opening the body comes through (metres).
const THROUGH: f32 = 0.01;

/// Bones longer than this aren't limbs (the root's line from the floor
/// to the hips), and don't make a loop an opening (metres).
const BONE_MOST: f32 = 0.6;

/// How far past the wrists, the ankles and the head a limb's line is
/// drawn on (metres), so a long cuff or a high collar is still gone
/// through.
const BEYOND: f32 = 0.25;

/// What mending did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mended {
    /// Holes filled, and the triangles that took.
    pub holes: usize,
    pub triangles: usize,
    /// Open loops left as openings.
    pub openings: usize,
    /// Holes left open, wholly or in part: their corners are in different
    /// pieces, or a patch would be a web (cloth a pose draws out).
    pub left: usize,
}

/// A hole and the patch that closes it: triangles over the garment's
/// corners ([`GarmentMesh::positions`]), all in one piece.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Hole {
    pub piece: usize,
    pub patch: Vec<[u32; 3]>,
}

/// Fills every hole of `garment`, worn on `body` at rest. `sound` says
/// whether a triangle between three of the garment's corners may be made
/// (the fit refuses webs); a hole whose patch it refuses in part is
/// closed as far as it allows.
pub(super) fn mend(
    garment: &mut GarmentMesh,
    body: &BodyMesh,
    sound: impl Fn(&[u32; 3]) -> bool,
) -> Mended {
    let (holes, mut mended) = holes(garment, body, sound);
    fill(garment, &holes);
    mended.holes = holes.len();
    mended.triangles = holes.iter().map(|hole| hole.patch.len()).sum();
    mended
}

/// The holes of `garment`, worn on `body` at rest, each with its patch;
/// and the count of openings and of holes that can't be patched whole.
pub(super) fn holes(
    garment: &GarmentMesh,
    body: &BodyMesh,
    sound: impl Fn(&[u32; 3]) -> bool,
) -> (Vec<Hole>, Mended) {
    let surface = Surface::new(
        body.positions.clone(),
        body.normals.clone(),
        body.triangles.clone(),
    );
    let lines = limb_lines(body);
    let comes_through = |ring: &[Vec3]| {
        let middle = ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
        if surface
            .closest(middle, 0.3)
            .is_some_and(|hit| hit.signed(middle) < -THROUGH)
        {
            return true;
        }
        lines.iter().any(|(from, to)| {
            (0..ring.len())
                .any(|k| crosses(*from, *to, middle, ring[k], ring[(k + 1) % ring.len()]))
        })
    };
    holes_where(garment, comes_through, sound, Limits::HOLE)
}

/// Closes the cuts the intake made where a hanging sleeve was joined to
/// the garment's side: turned to the rest pose, the joins are taken out,
/// which leaves the sleeve open along its underside and the side open
/// where the sleeve was. Each open ring is closed from its ends inward
/// between corners `rim` gives a place (the cut's own rim: where each lies
/// between the trunk, all 0, and each limb, 1 in its place), by triangles
/// whose corners are no more than half a share apart on any limb, so the
/// sleeve is closed to itself and the side to itself, and they meet only
/// under the arm where the cloth passes from one to the other.
pub(super) fn close_cuts(
    garment: &mut GarmentMesh,
    rim: impl Fn(u32) -> Option<[f32; 4]>,
) -> Mended {
    let sound = |t: &[u32; 3]| {
        let Some([a, b, c]) = t
            .map(&rim)
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|places| <[[f32; 4]; 3]>::try_from(places).ok())
        else {
            return false;
        };
        (0..4).all(|limb| {
            let (low, high) = [a[limb], b[limb], c[limb]]
                .into_iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), share| {
                    (low.min(share), high.max(share))
                });
            high - low <= 0.5
        })
    };
    let limits = Limits {
        round: f32::INFINITY,
        span: CUT_SPAN,
        flat: false,
    };
    let (holes, mut mended) = holes_where(garment, |_| false, sound, limits);
    fill(garment, &holes);
    mended.holes = holes.len();
    mended.triangles = holes.iter().map(|hole| hole.patch.len()).sum();
    mended
}

/// [`holes`] with the body's part given as `comes_through`: whether the
/// body comes through a short open loop (its corners in order).
pub(super) fn holes_where(
    garment: &GarmentMesh,
    comes_through: impl Fn(&[Vec3]) -> bool,
    sound: impl Fn(&[u32; 3]) -> bool,
    limits: Limits,
) -> (Vec<Hole>, Mended) {
    let mut mended = Mended::default();
    let positions = garment.positions();
    let triangles = garment.triangles();
    // Corners at one place are one point.
    let mut places: BTreeMap<[i64; 3], u32> = BTreeMap::new();
    let point_of: Vec<u32> = positions
        .iter()
        .map(|p| {
            let key = (*p * 1e5).round().as_i64vec3().to_array();
            let next = places.len() as u32;
            *places.entry(key).or_insert(next)
        })
        .collect();
    let mut starts = Vec::with_capacity(garment.pieces.len());
    let mut total = 0u32;
    for piece in &garment.pieces {
        starts.push(total);
        total += piece.positions.len() as u32;
    }
    let piece_of = |corner: u32| starts.partition_point(|start| *start <= corner) - 1;

    // Open edges: the sides one triangle only has, each the way its
    // triangle winds.
    let side = |a: u32, b: u32| (a.min(b), a.max(b));
    let mut uses: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    let welded = |t: &[u32; 3]| t.map(|v| point_of[v as usize]);
    for t in &triangles {
        let w = welded(t);
        if w[0] == w[1] || w[1] == w[2] || w[0] == w[2] {
            continue;
        }
        for k in 0..3 {
            *uses.entry(side(w[k], w[(k + 1) % 3])).or_default() += 1;
        }
    }
    // Each open edge the way its triangle winds, with the triangle's
    // corners at its two ends.
    struct Open {
        ends: [u32; 2],
        corners: [u32; 2],
    }
    let mut open: Vec<Open> = Vec::new();
    let mut at_point: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for t in &triangles {
        let w = welded(t);
        if w[0] == w[1] || w[1] == w[2] || w[0] == w[2] {
            continue;
        }
        for k in 0..3 {
            let (a, b) = (w[k], w[(k + 1) % 3]);
            if uses[&side(a, b)] == 1 {
                at_point.entry(a).or_default().push(open.len());
                at_point.entry(b).or_default().push(open.len());
                open.push(Open {
                    ends: [a, b],
                    corners: [t[k], t[(k + 1) % 3]],
                });
            }
        }
    }
    // Walked into rings, whichever way each edge winds (a generated
    // mesh has triangles wound both ways side by side): a ring that
    // touches itself is cut at the touch, and each is turned the way most
    // of its edges wind.
    let mut used = vec![false; open.len()];
    let mut rings: Vec<Vec<u32>> = Vec::new();
    let begins: Vec<u32> = at_point.keys().copied().collect();
    for begin in begins {
        loop {
            // The point, the corner there, and whether the edge leaving it
            // winds the way it is walked.
            let mut path: Vec<(u32, u32, bool)> = Vec::new();
            let mut at = begin;
            let mut walked = false;
            while let Some(&edge) = at_point
                .get(&at)
                .and_then(|list| list.iter().find(|edge| !used[**edge]))
            {
                used[edge] = true;
                walked = true;
                let with = open[edge].ends[0] == at;
                let (here, there) = if with { (0, 1) } else { (1, 0) };
                path.push((at, open[edge].corners[here], with));
                at = open[edge].ends[there];
                if let Some(since) = path.iter().position(|(point, _, _)| *point == at) {
                    let mut ring = path.split_off(since);
                    let with = ring.iter().filter(|step| step.2).count();
                    if with * 2 < ring.len() {
                        ring.reverse();
                    }
                    rings.push(ring.into_iter().map(|step| step.1).collect());
                    if path.is_empty() {
                        break;
                    }
                }
            }
            if !walked {
                break;
            }
        }
    }

    let mut found = Vec::new();
    for ring in rings {
        if ring.len() < 3 {
            continue;
        }
        let at: Vec<Vec3> = ring.iter().map(|c| positions[*c as usize]).collect();
        let round: f32 = (0..at.len())
            .map(|k| at[k].distance(at[(k + 1) % at.len()]))
            .sum();
        if round > limits.round || comes_through(&at) {
            mended.openings += 1;
            continue;
        }
        let piece = piece_of(ring[0]);
        if ring.iter().any(|corner| piece_of(*corner) != piece) {
            mended.left += 1;
            continue;
        }
        // The patch winds against the ring, as the cloth round it does.
        let back: Vec<usize> = (0..ring.len()).rev().collect();
        let points: Vec<Vec3> = back.iter().map(|k| at[*k]).collect();
        let short = |t: &[usize; 3]| {
            let [a, b, c] = t.map(|k| points[k]);
            a.distance(b).max(b.distance(c)).max(c.distance(a)) <= limits.span
        };
        let (patch, whole) = ears(
            &points,
            |t| short(t) && sound(&t.map(|k| ring[back[k]])),
            limits.flat,
        );
        if !whole {
            mended.left += 1;
        }
        if !patch.is_empty() {
            let patch = patch
                .into_iter()
                .map(|t| t.map(|k| ring[back[k]]))
                .collect();
            found.push(Hole { piece, patch });
        }
    }
    (found, mended)
}

/// Adds each of `holes`' patches to `garment`.
pub(super) fn fill(garment: &mut GarmentMesh, holes: &[Hole]) {
    let mut starts = Vec::with_capacity(garment.pieces.len());
    let mut total = 0u32;
    for piece in &garment.pieces {
        starts.push(total);
        total += piece.positions.len() as u32;
    }
    for hole in holes {
        let Some(piece) = garment.pieces.get_mut(hole.piece) else {
            continue;
        };
        let start = starts[hole.piece];
        for t in &hole.patch {
            piece.indices.extend(t.map(|corner| corner - start));
        }
    }
}

/// The lines the body's limbs, trunk and neck lie along at rest: each
/// bone from its parent's head to its own, and the hands, feet and head
/// drawn on past their ends.
fn limb_lines(body: &BodyMesh) -> Vec<(Vec3, Vec3)> {
    let rig = &body.rig;
    let world = rig.world(&rig.rest_pose());
    let head = |bone: usize| world[bone].w_axis.truncate();
    let mut lines = Vec::new();
    for (index, bone) in rig.bones.iter().enumerate() {
        let Some(parent) = bone.parent else {
            continue;
        };
        let (from, to) = (head(parent), head(index));
        let length = from.distance(to);
        if !(1e-4..=BONE_MOST).contains(&length) {
            continue;
        }
        lines.push((from, to));
        let ends = ["Hand", "Foot", "Head"]
            .iter()
            .any(|end| bone.name.ends_with(end));
        if ends {
            lines.push((to, to + (to - from) / length * BEYOND));
        }
    }
    lines
}

/// Whether the line from `from` to `to` crosses the triangle `a b c`.
fn crosses(from: Vec3, to: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let along = to - from;
    let (ab, ac) = (b - a, c - a);
    let p = along.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return false;
    }
    let s = from - a;
    let u = s.dot(p) / det;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let q = s.cross(ab);
    let v = along.dot(q) / det;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let t = ac.dot(q) / det;
    (0.0..=1.0).contains(&t)
}

/// Triangles that fill the ring `points` (in order), winding as it does:
/// ears cut one at a time, each time the best-shaped one that bulges the
/// ring's way and has none of its other corners inside. An ear `sound`
/// refuses is never cut; when only those are left the rest of the ring
/// stays open, and the second value is false.
fn ears(
    points: &[Vec3],
    sound: impl Fn(&[usize; 3]) -> bool,
    flat: bool,
) -> (Vec<[usize; 3]>, bool) {
    let mut ring: Vec<usize> = (0..points.len()).collect();
    // Newell's normal: the way the ring winds, whatever its shape.
    let normal = (0..points.len())
        .map(|k| points[k].cross(points[(k + 1) % points.len()]))
        .sum::<Vec3>()
        .normalize_or_zero();
    let mut out = Vec::with_capacity(points.len().saturating_sub(2));
    while ring.len() > 3 {
        let count = ring.len();
        let mut best = (usize::MAX, f32::NEG_INFINITY);
        for k in 0..count {
            let (before, after) = ((k + count - 1) % count, (k + 1) % count);
            if !sound(&[ring[before], ring[k], ring[after]]) {
                continue;
            }
            let (a, b, c) = (points[ring[before]], points[ring[k]], points[ring[after]]);
            let mut score = least_angle(a, b, c);
            if !flat {
                // Only its shape counts, and three corners in a line
                // (along a straight rim) close nothing.
                if score < SLIVER {
                    continue;
                }
            } else if (b - a).cross(c - b).dot(normal) <= 0.0 {
                score -= 10.0;
            } else if (0..count)
                .filter(|other| *other != before && *other != k && *other != after)
                .any(|other| within(points[ring[other]], a, b, c, normal))
            {
                score -= 5.0;
            }
            if score > best.1 {
                best = (k, score);
            }
        }
        let k = best.0;
        if k == usize::MAX {
            return (out, false);
        }
        out.push([
            ring[(k + count - 1) % count],
            ring[k],
            ring[(k + 1) % count],
        ]);
        ring.remove(k);
    }
    if ring.len() == 3 {
        if !sound(&[ring[0], ring[1], ring[2]]) {
            return (out, false);
        }
        out.push([ring[0], ring[1], ring[2]]);
    }
    (out, true)
}

/// The smallest angle of triangle `a b c` (radians).
fn least_angle(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let angle = |at: Vec3, p: Vec3, q: Vec3| {
        let (u, v) = ((p - at).normalize_or_zero(), (q - at).normalize_or_zero());
        u.dot(v).clamp(-1.0, 1.0).acos()
    };
    angle(a, b, c).min(angle(b, c, a)).min(angle(c, a, b))
}

/// Whether `p`, seen along `normal`, is inside triangle `a b c` (which
/// winds round `normal`) or on one of its sides: cutting that ear would
/// leave `p` on a line with two of its neighbours, a triangle with no
/// area.
fn within(p: Vec3, a: Vec3, b: Vec3, c: Vec3, normal: Vec3) -> bool {
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .all(|(from, to)| (to - from).cross(p - from).dot(normal) >= -1e-10)
}

#[cfg(test)]
mod tests {
    use super::super::GarmentPiece;
    use super::*;

    /// A flat sheet of `n` by `n` squares, 2 cm each, in the XY plane.
    fn sheet(n: u32) -> GarmentMesh {
        let mut positions = Vec::new();
        for y in 0..=n {
            for x in 0..=n {
                positions.push(Vec3::new(x as f32 * 0.02, y as f32 * 0.02, 0.0));
            }
        }
        let mut indices = Vec::new();
        let at = |x: u32, y: u32| y * (n + 1) + x;
        for y in 0..n {
            for x in 0..n {
                indices.extend([at(x, y), at(x + 1, y), at(x + 1, y + 1)]);
                indices.extend([at(x, y), at(x + 1, y + 1), at(x, y + 1)]);
            }
        }
        GarmentMesh {
            pieces: vec![GarmentPiece {
                material: "Cloth".to_owned(),
                colour: [1.0; 4],
                roughness: 0.8,
                normals: vec![Vec3::Z; positions.len()],
                uvs: Vec::new(),
                positions,
                indices,
            }],
        }
    }

    /// The triangles of `garment` that touch none of `corners`.
    fn without(garment: &GarmentMesh, corners: &[u32]) -> GarmentMesh {
        let mut out = garment.clone();
        out.pieces[0].indices = garment.pieces[0]
            .indices
            .chunks_exact(3)
            .filter(|t| !t.iter().any(|v| corners.contains(v)))
            .flatten()
            .copied()
            .collect();
        out
    }

    fn area(garment: &GarmentMesh) -> f32 {
        let positions = garment.positions();
        garment
            .triangles()
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| positions[v as usize]);
                // Signed: a patch wound the wrong way would take area off.
                (b - a).cross(c - a).z / 2.0
            })
            .sum()
    }

    #[test]
    fn a_hole_is_filled_and_winds_with_the_cloth() {
        let whole = sheet(8);
        // The corner in the middle and its six triangles come out.
        let torn = without(&whole, &[4 * 9 + 4]);
        assert!(area(&torn) < area(&whole) - 1e-6);
        let (holes, mended) = holes_where(&torn, |ring| ring.len() > 12, |_| true, Limits::HOLE);
        assert_eq!((holes.len(), mended.openings, mended.left), (1, 1, 0));
        let mut filled = torn.clone();
        fill(&mut filled, &holes);
        assert!((area(&filled) - area(&whole)).abs() < 1e-6);
        // Nothing is open but the sheet's own edge.
        let (again, after) = holes_where(&filled, |ring| ring.len() > 12, |_| true, Limits::HOLE);
        assert!(again.is_empty());
        assert_eq!(after.openings, 1);
    }

    #[test]
    fn an_opening_the_body_comes_through_is_left_open() {
        let torn = without(&sheet(8), &[4 * 9 + 4]);
        let (holes, mended) = holes_where(&torn, |_| true, |_| true, Limits::HOLE);
        assert!(holes.is_empty());
        assert_eq!(mended.openings, 2);
    }

    #[test]
    fn two_holes_that_touch_at_a_corner_are_each_filled() {
        let whole = sheet(8);
        // Two squares that share only a corner.
        let mut torn = whole.clone();
        let at = |x: u32, y: u32| y * 9 + x;
        let gone = [
            [at(2, 2), at(3, 2), at(3, 3)],
            [at(2, 2), at(3, 3), at(2, 3)],
            [at(3, 3), at(4, 3), at(4, 4)],
            [at(3, 3), at(4, 4), at(3, 4)],
        ];
        torn.pieces[0].indices = whole.pieces[0]
            .indices
            .chunks_exact(3)
            .filter(|t| !gone.iter().any(|g| g == t))
            .flatten()
            .copied()
            .collect();
        let (holes, _) = holes_where(&torn, |ring| ring.len() > 12, |_| true, Limits::HOLE);
        assert_eq!(holes.len(), 2);
        let mut filled = torn.clone();
        fill(&mut filled, &holes);
        assert!((area(&filled) - area(&whole)).abs() < 1e-6);
    }

    #[test]
    fn a_dented_hole_is_filled_without_folding_over() {
        let whole = sheet(8);
        // An L of three squares: its ring has a corner that points in.
        let torn = without(&whole, &[]);
        let at = |x: u32, y: u32| y * 9 + x;
        let squares = [(3, 3), (4, 3), (3, 4)];
        let mut out = torn.clone();
        out.pieces[0].indices = whole.pieces[0]
            .indices
            .chunks_exact(3)
            .filter(|t| {
                !squares.iter().any(|&(x, y)| {
                    let own = [at(x, y), at(x + 1, y), at(x + 1, y + 1), at(x, y + 1)];
                    t.iter().all(|v| own.contains(v))
                })
            })
            .flatten()
            .copied()
            .collect();
        let (holes, _) = holes_where(&out, |ring| ring.len() > 12, |_| true, Limits::HOLE);
        assert_eq!(holes.len(), 1);
        let positions = out.positions();
        for t in &holes[0].patch {
            let [a, b, c] = t.map(|v| positions[v as usize]);
            assert!((b - a).cross(c - a).z > 0.0, "a patch triangle faces back");
        }
        fill(&mut out, &holes);
        assert!((area(&out) - area(&whole)).abs() < 1e-6);
    }

    #[test]
    fn a_patch_the_caller_refuses_in_part_closes_what_it_may() {
        let whole = sheet(8);
        let torn = without(&whole, &[4 * 9 + 4]);
        // No patch triangle may cross the hole's middle: not from its left
        // side to its right, nor from its bottom corner to its top one.
        let positions = torn.positions();
        let sound = |t: &[u32; 3]| {
            let at = t.map(|v| positions[v as usize]);
            let across = at.iter().any(|p| p.x < 0.07) && at.iter().any(|p| p.x > 0.09);
            let middle = |p: &Vec3| (p.x - 0.08).abs() < 0.001;
            let upright = at.iter().any(|p| middle(p) && p.y < 0.07)
                && at.iter().any(|p| middle(p) && p.y > 0.09);
            !across && !upright
        };
        let (holes, mended) = holes_where(&torn, |ring| ring.len() > 12, sound, Limits::HOLE);
        assert_eq!(mended.left, 1);
        let patched: usize = holes.iter().map(|hole| hole.patch.len()).sum();
        assert_eq!(patched, 2, "an ear at each side, and the middle left open");
        for t in holes.iter().flat_map(|hole| &hole.patch) {
            assert!(sound(t));
        }
    }

    #[test]
    fn a_cut_is_closed_between_the_cloth_of_one_part_only() {
        // A slit four squares long down the sheet's middle column, its
        // left rim the trunk's cloth and its right rim a sleeve's.
        let whole = sheet(8);
        let at = |x: u32, y: u32| y * 9 + x;
        let mut torn = whole.clone();
        torn.pieces[0].indices = whole.pieces[0]
            .indices
            .chunks_exact(3)
            .filter(|t| {
                !(2..6).any(|y| {
                    let own = [at(4, y), at(5, y), at(5, y + 1), at(4, y + 1)];
                    t.iter().all(|v| own.contains(v))
                })
            })
            .flatten()
            .copied()
            .collect();
        let positions = torn.positions();
        let sleeve = |corner: u32| positions[corner as usize].x > 0.09;
        // Every corner on the slit's rim has a place; no other has.
        let on_rim = |corner: u32| {
            let p = positions[corner as usize];
            (0.079..0.101).contains(&p.x) && (0.039..0.121).contains(&p.y)
        };
        let rim = |corner: u32| {
            on_rim(corner).then(|| [if sleeve(corner) { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0])
        };
        let mut closed = torn.clone();
        let mended = close_cuts(&mut closed, rim);
        // Nothing may join the two rims, so the slit stays open.
        assert_eq!(mended.triangles, 0, "{mended:?}");

        // With both rims the same cloth it is zipped shut, and the sheet's
        // own edge, which has no place on the cut, is left alone.
        let rim = |corner: u32| on_rim(corner).then_some([1.0, 0.0, 0.0, 0.0]);
        let mut closed = torn.clone();
        let mended = close_cuts(&mut closed, rim);
        assert_eq!(mended.left, 1, "the sheet's edge is no cut: {mended:?}");
        assert!((area(&closed) - area(&whole)).abs() < 1e-6);
    }

    #[test]
    fn a_line_through_a_triangle_crosses_it_and_one_beside_it_does_not() {
        let (a, b, c) = (Vec3::ZERO, Vec3::X, Vec3::Y);
        assert!(crosses(
            Vec3::new(0.2, 0.2, -1.0),
            Vec3::new(0.2, 0.2, 1.0),
            a,
            b,
            c
        ));
        assert!(!crosses(
            Vec3::new(0.9, 0.9, -1.0),
            Vec3::new(0.9, 0.9, 1.0),
            a,
            b,
            c
        ));
        // It stops short of the triangle.
        assert!(!crosses(
            Vec3::new(0.2, 0.2, -1.0),
            Vec3::new(0.2, 0.2, -0.1),
            a,
            b,
            c
        ));
    }
}
