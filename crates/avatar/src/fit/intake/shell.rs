//! A generated garment's two walls: which of its surface is the outside
//! of the cloth and which the inside, and the outside alone.
//!
//! Image- and text-to-3D tools make a garment as a closed shell several
//! millimetres thick: an outer wall, an inner wall facing the body, joined
//! at the openings. Fitting works on one layer of cloth, and the body
//! pushes on the inner wall while the picture is the outer one, so the
//! inner wall is taken off, except a band at each opening, which stays as
//! the turned edge (a hem with thickness).

use std::collections::VecDeque;

use glam::{Vec2, Vec3};

use super::slices::{Slice, Welded, hull_area, hull_centre};
use super::structure::Structure;

/// A loop with nothing inside it is a band of cloth when it is thinner
/// than this many thicknesses of cloth (twice its area over its length).
const THIN: f32 = 4.0;
/// A band's thickness is at most this share of its length round.
const SLENDER: f32 = 0.06;
/// A band is bent round something when the tape round it holds this many
/// times its own area.
const BENT: f32 = 3.0;
/// The point a band's walls are told apart by is the trunk's middle this
/// share of the garment's height below them.
const BELOW: f32 = 0.1;

/// A hole in a ring of cloth is the body's place when it is at least this
/// share of the ring's area; a smaller one is a gap the cloth happens to
/// close round (a belt loop, a knot).
const CAVITY_SHARE: f32 = 0.005;
/// A hollow is at least this round (its area over a circle's of its
/// length): a sliver of air under a pocket's flap or between two folds is
/// cut as a loop inside the cloth too, but a thin one.
const CAVITY_ROUND: f32 = 0.15;

/// Which of `mesh`'s points are on its inner wall.
///
/// Each cross-section votes: the points of a loop with an odd count of
/// loops round it (the inside of a tube) for the inner wall, the others
/// for the outer. A point no plane cuts near takes its neighbours' side,
/// and a point most of whose neighbours disagree with it joins them.
///
/// Cloth that is not a tube where a plane cuts it (a strap, the front of a
/// sleeveless top beside the armhole, an open jacket's front) is cut as a
/// thin band with nothing inside it: both its walls are one loop. There
/// each point votes by which way it faces: toward the middle of the trunk
/// a little below it is the inner wall.
pub(crate) fn inner_wall(
    mesh: &Welded,
    slices: &[Slice],
    normals: &[Vec3],
    structure: &Structure,
    thickness: f32,
) -> Vec<bool> {
    let mut votes = vec![0i32; mesh.points.len()];
    let (lo, hi) = mesh.bounds();
    let below = BELOW * (hi.y - lo.y);
    let core = |p: Vec3| {
        let middle = structure
            .trunk_at(p.y - below)
            .map_or(Vec2::new((lo.x + hi.x) / 2.0, (lo.z + hi.z) / 2.0), |r| {
                r.centre
            });
        Vec3::new(middle.x, p.y - below, middle.y)
    };
    for slice in slices {
        for item in &slice.loops {
            if !item.closed {
                continue;
            }
            // A band: as thin as cloth, and many times longer than thin
            // (a thick sleeve is as thin as its own padding, but round).
            let width = 2.0 * item.area / item.length.max(1e-9);
            if item.depth == 0 && width < THIN * thickness && width < SLENDER * item.length {
                // A band bent round something (a shirt open down the
                // front, a leg with a slit) faces its own middle; a flat
                // one (a strap, a panel) has none, and faces the trunk's.
                let hull = item.hull();
                let own = (hull_area(&hull) > BENT * item.area).then(|| hull_centre(&hull));
                for &(a, b) in &item.edges {
                    for v in [a as usize, b as usize] {
                        let p = mesh.points[v];
                        let to = match own {
                            Some(c) => Vec3::new(c.x, p.y - below, c.y),
                            None => core(p),
                        };
                        votes[v] += if normals[v].dot(to - p) > 0.0 { -1 } else { 1 };
                    }
                }
                continue;
            }
            let inner = item.depth % 2 == 1
                && 2.0 * std::f32::consts::TAU * item.area
                    >= CAVITY_ROUND * item.length * item.length
                && item
                    .parent
                    .is_some_and(|p| item.area >= CAVITY_SHARE * slice.loops[p].area);
            let vote = if inner { -1 } else { 1 };
            for &(a, b) in &item.edges {
                votes[a as usize] += vote;
                votes[b as usize] += vote;
            }
        }
    }
    let neighbours = mesh.neighbours();
    // Points without a vote take the side of the voted points round them,
    // ring by ring.
    let mut side: Vec<i8> = votes.iter().map(|v| v.signum() as i8).collect();
    for _ in 0..64 {
        let before = side.clone();
        let mut open = 0;
        for (point, list) in neighbours.iter().enumerate() {
            if before[point] != 0 {
                continue;
            }
            let sum: i32 = list.iter().map(|n| i32::from(before[*n as usize])).sum();
            side[point] = sum.signum() as i8;
            if side[point] == 0 {
                open += 1;
            }
        }
        if open == 0 {
            break;
        }
    }
    for _ in 0..2 {
        let before = side.clone();
        for (point, list) in neighbours.iter().enumerate() {
            if list.len() < 3 {
                continue;
            }
            let against = list
                .iter()
                .filter(|n| before[**n as usize] == -before[point])
                .count();
            if against * 3 > list.len() * 2 {
                side[point] = -before[point];
            }
        }
    }
    side.into_iter().map(|s| s < 0).collect()
}

/// How thick the cloth is: across the planes, the middle value of how far
/// a ring's inner loop is inside its outer one (their lengths' difference
/// over a full turn). `None` when no plane cuts a ring.
pub(crate) fn thickness(slices: &[Slice]) -> Option<f32> {
    let mut found = Vec::new();
    for slice in slices {
        for item in &slice.loops {
            let Some(parent) = item.parent else {
                continue;
            };
            let outer = &slice.loops[parent];
            if item.depth % 2 == 1 && item.closed && item.area > 0.5 * outer.area {
                found.push((outer.length - item.length) / std::f32::consts::TAU);
            }
        }
    }
    if found.is_empty() {
        return None;
    }
    found.sort_by(f32::total_cmp);
    Some(found[found.len() / 2].max(0.0))
}

/// The triangles of `mesh` (as places in [`Welded::triangles`]) that are
/// its outer wall and the turned edges: every triangle with a corner on
/// the outer wall, and inner-wall triangles within `rim` edges of it (the
/// turned edge at an opening; 0 keeps only triangles wholly on the outer
/// wall).
pub(crate) fn outer_triangles(mesh: &Welded, inner: &[bool], rim: u32) -> Vec<u32> {
    let neighbours = mesh.neighbours();
    // Each point's count of edges from the outer wall.
    let mut ring = vec![u32::MAX; mesh.points.len()];
    let mut queue = VecDeque::new();
    for (point, is_inner) in inner.iter().enumerate() {
        if !is_inner {
            ring[point] = 0;
            queue.push_back(point as u32);
        }
    }
    while let Some(point) = queue.pop_front() {
        let here = ring[point as usize];
        if here >= rim {
            continue;
        }
        for &n in &neighbours[point as usize] {
            if ring[n as usize] == u32::MAX {
                ring[n as usize] = here + 1;
                queue.push_back(n);
            }
        }
    }
    mesh.triangles
        .iter()
        .enumerate()
        .filter(|(_, t)| t.iter().all(|v| ring[*v as usize] <= rim))
        .map(|(index, _)| index as u32)
        .collect()
}
