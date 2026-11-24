//! Level cross-sections of a mesh: where a stack of level planes cuts it,
//! as loops in each plane.
//!
//! A generated garment is a closed shell of cloth with thickness, so a
//! plane cuts it in rings of cloth: each ring's outline is the garment's
//! outer wall and the hole in it is the inner wall round the body's place.
//! Loops are therefore told apart by how many others are round them
//! ([`Loop::depth`]): an even count is cloth seen from outside, an odd one
//! the inside of a tube.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

/// A chain of a cross-section that doesn't come back to its start is
/// closed across the gap when the gap is at most this share of its length.
const GAP: f32 = 0.1;

/// A mesh with corners at the same place made one, so its triangles join
/// across texture seams.
#[derive(Clone, Debug, Default)]
pub(crate) struct Welded {
    pub points: Vec<Vec3>,
    /// Each corner's point.
    pub of_corner: Vec<u32>,
    /// Triangles over `points`, without the ones welding left flat.
    pub triangles: Vec<[u32; 3]>,
    /// Each of `triangles`' place in the list it was made from.
    pub source: Vec<u32>,
}

impl Welded {
    /// `positions` welded: corners closer than a millionth of the mesh's
    /// size are one point.
    pub fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Welded {
        let (lo, hi) = positions
            .iter()
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            });
        let size = (hi - lo).max_element().max(1e-6);
        let step = (size * 1e-6) as f64;
        let mut by_place: HashMap<[i64; 3], u32> = HashMap::new();
        let mut points = Vec::new();
        let of_corner: Vec<u32> = positions
            .iter()
            .map(|p| {
                let key = [
                    (f64::from(p.x) / step).round() as i64,
                    (f64::from(p.y) / step).round() as i64,
                    (f64::from(p.z) / step).round() as i64,
                ];
                *by_place.entry(key).or_insert_with(|| {
                    points.push(*p);
                    points.len() as u32 - 1
                })
            })
            .collect();
        let mut welded = Vec::new();
        let mut source = Vec::new();
        for (index, t) in triangles.iter().enumerate() {
            let w = t.map(|v| of_corner[v as usize]);
            if w[0] != w[1] && w[1] != w[2] && w[0] != w[2] {
                welded.push(w);
                source.push(index as u32);
            }
        }
        Welded {
            points,
            of_corner,
            triangles: welded,
            source,
        }
    }

    /// Its box: (least, most).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.points
            .iter()
            .fold((Vec3::INFINITY, Vec3::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(*p), hi.max(*p))
            })
    }

    /// Six times the volume its triangles enclose: below zero when they
    /// are wound inside out.
    pub fn signed_volume(&self) -> f32 {
        let (lo, hi) = self.bounds();
        let middle = (lo + hi) / 2.0;
        self.triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| self.points[v as usize] - middle);
                f64::from(a.dot(b.cross(c)))
            })
            .sum::<f64>() as f32
    }

    /// Each point's neighbours along an edge.
    pub fn neighbours(&self) -> Vec<Vec<u32>> {
        let mut out = vec![Vec::new(); self.points.len()];
        for t in &self.triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                out[a as usize].push(b);
                out[b as usize].push(a);
            }
        }
        for list in &mut out {
            list.sort_unstable();
            list.dedup();
        }
        out
    }
}

/// One loop a level plane cuts from a mesh.
#[derive(Clone, Debug)]
pub(crate) struct Loop {
    /// Its corners in the plane (x, z), in order.
    pub points: Vec<Vec2>,
    /// The mesh edge each corner is on (welded points, the lower index
    /// first).
    pub edges: Vec<(u32, u32)>,
    /// Whether it comes back to where it starts.
    pub closed: bool,
    /// How many of the plane's other loops are round it.
    pub depth: u32,
    /// The loop round it that is closest in size, if any.
    pub parent: Option<usize>,
    pub centre: Vec2,
    pub min: Vec2,
    pub max: Vec2,
    /// How long it is.
    pub length: f32,
    /// The area inside it (never negative).
    pub area: f32,
}

impl Loop {
    fn new(points: Vec<Vec2>, edges: Vec<(u32, u32)>, closed: bool) -> Loop {
        let mut min = Vec2::INFINITY;
        let mut max = Vec2::NEG_INFINITY;
        let mut length = 0.0;
        let mut area = 0.0f64;
        let mut centre = Vec2::ZERO;
        let count = points.len();
        for (k, p) in points.iter().enumerate() {
            min = min.min(*p);
            max = max.max(*p);
            let q = points[(k + 1) % count];
            if closed || k + 1 < count {
                length += p.distance(q);
            }
            let cross = f64::from(p.x) * f64::from(q.y) - f64::from(q.x) * f64::from(p.y);
            area += cross;
            centre += (*p + q) * cross as f32;
        }
        let centre = if area.abs() > 1e-12 {
            centre / (3.0 * area as f32)
        } else {
            points.iter().sum::<Vec2>() / count.max(1) as f32
        };
        Loop {
            points,
            edges,
            closed,
            depth: 0,
            parent: None,
            centre,
            min,
            max,
            length,
            area: (area / 2.0).abs() as f32,
        }
    }

    /// Whether `p` is inside it (crossings of a line from `p`, odd inside).
    pub fn holds(&self, p: Vec2) -> bool {
        if p.cmplt(self.min).any() || p.cmpgt(self.max).any() {
            return false;
        }
        let mut inside = false;
        let count = self.points.len();
        for k in 0..count {
            let (a, b) = (self.points[k], self.points[(k + 1) % count]);
            if (a.y > p.y) != (b.y > p.y) {
                let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
                if x > p.x {
                    inside = !inside;
                }
            }
        }
        inside
    }

    /// The tightest loop without hollows round it (its convex hull), as a
    /// tape measure lies: counter-clockwise.
    pub fn hull(&self) -> Vec<Vec2> {
        let mut points = self.points.clone();
        points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        points.dedup();
        if points.len() < 3 {
            return points;
        }
        let cross = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
        let mut hull: Vec<Vec2> = Vec::with_capacity(points.len() + 1);
        for pass in 0..2 {
            let start = hull.len();
            let walk: Box<dyn Iterator<Item = &Vec2>> = if pass == 0 {
                Box::new(points.iter())
            } else {
                Box::new(points.iter().rev())
            };
            for p in walk {
                while hull.len() >= start + 2
                    && cross(hull[hull.len() - 2], hull[hull.len() - 1], *p) <= 0.0
                {
                    hull.pop();
                }
                hull.push(*p);
            }
            hull.pop();
        }
        hull
    }
}

/// How long the closed line through `points` is.
pub(crate) fn length_round(points: &[Vec2]) -> f32 {
    (0..points.len())
        .map(|k| points[k].distance(points[(k + 1) % points.len()]))
        .sum()
}

/// Whether `p` is inside the convex loop `hull` (counter-clockwise).
pub(crate) fn hull_holds(hull: &[Vec2], p: Vec2) -> bool {
    hull.len() >= 3
        && (0..hull.len()).all(|k| {
            let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
            (b - a).perp_dot(p - a) >= 0.0
        })
}

/// The area inside the convex loop `hull`.
pub(crate) fn hull_area(hull: &[Vec2]) -> f32 {
    let twice: f32 = (0..hull.len())
        .map(|k| hull[k].perp_dot(hull[(k + 1) % hull.len()]))
        .sum();
    (twice / 2.0).abs()
}

/// The middle of the area inside the convex loop `hull`.
pub(crate) fn hull_centre(hull: &[Vec2]) -> Vec2 {
    let mut area = 0.0;
    let mut centre = Vec2::ZERO;
    for k in 0..hull.len() {
        let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
        let cross = a.perp_dot(b);
        area += cross;
        centre += (a + b) * cross;
    }
    if area.abs() > 1e-12 {
        centre / (3.0 * area)
    } else {
        hull.iter().sum::<Vec2>() / hull.len().max(1) as f32
    }
}

/// What one level plane cuts.
#[derive(Clone, Debug)]
pub(crate) struct Slice {
    pub y: f32,
    pub loops: Vec<Loop>,
}

/// `count` level planes through `mesh`, evenly spaced from its lowest
/// point to its highest, lowest first.
pub(crate) fn slices(mesh: &Welded, count: usize) -> Vec<Slice> {
    let (lo, hi) = mesh.bounds();
    let count = count.max(1);
    let step = (hi.y - lo.y) / count as f32;
    if step.is_nan() || step <= 0.0 {
        return Vec::new();
    }
    // Each plane's triangles: the ones with a corner either side.
    let mut crossing: Vec<Vec<u32>> = vec![Vec::new(); count];
    let level = |k: usize| lo.y + (k as f32 + 0.5) * step;
    for (index, t) in mesh.triangles.iter().enumerate() {
        let ys = t.map(|v| mesh.points[v as usize].y);
        let low = ys[0].min(ys[1]).min(ys[2]);
        let high = ys[0].max(ys[1]).max(ys[2]);
        let first = (((low - lo.y) / step - 0.5).ceil().max(0.0)) as usize;
        for (k, list) in crossing.iter_mut().enumerate().skip(first) {
            let y = level(k);
            // A corner on the plane counts as above it, so a plane through
            // a row of corners cuts the triangles under them.
            if y > high {
                break;
            }
            if y > low {
                list.push(index as u32);
            }
        }
    }
    crossing
        .into_iter()
        .enumerate()
        .map(|(k, triangles)| {
            let y = level(k);
            Slice {
                y,
                loops: cut(mesh, &triangles, y),
            }
        })
        .collect()
}

/// The loops the plane at height `y` cuts from `triangles` of `mesh`.
fn cut(mesh: &Welded, triangles: &[u32], y: f32) -> Vec<Loop> {
    // A triangle with a corner either side is cut along a line between two
    // of its edges; the lines join where triangles share an edge.
    let mut segments: Vec<[(u32, u32); 2]> = Vec::with_capacity(triangles.len());
    let mut at_edge: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for &index in triangles {
        let t = mesh.triangles[index as usize];
        let above = t.map(|v| mesh.points[v as usize].y >= y);
        let mut ends = Vec::with_capacity(2);
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            if above[a] != above[b] {
                ends.push((t[a].min(t[b]), t[a].max(t[b])));
            }
        }
        if ends.len() == 2 {
            let segment = segments.len() as u32;
            at_edge.entry(ends[0]).or_default().push(segment);
            at_edge.entry(ends[1]).or_default().push(segment);
            segments.push([ends[0], ends[1]]);
        }
    }
    let place = |edge: (u32, u32)| {
        let (a, b) = (mesh.points[edge.0 as usize], mesh.points[edge.1 as usize]);
        let t = if (b.y - a.y).abs() > 1e-12 {
            ((y - a.y) / (b.y - a.y)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let p = a + (b - a) * t;
        Vec2::new(p.x, p.z)
    };
    let mut used = vec![false; segments.len()];
    let next = |edge: (u32, u32), used: &[bool]| -> Option<u32> {
        at_edge
            .get(&edge)?
            .iter()
            .copied()
            .find(|s| !used[*s as usize])
    };
    let mut loops = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let [first, mut end] = segments[start];
        let mut edges = vec![first, end];
        let mut closed = false;
        // Forward from the line's far end.
        while let Some(s) = next(end, &used) {
            used[s as usize] = true;
            let [a, b] = segments[s as usize];
            end = if a == end { b } else { a };
            if end == first {
                closed = true;
                break;
            }
            edges.push(end);
        }
        if !closed {
            // An open chain (a hole in the mesh): backward from its start
            // too.
            let mut begin = first;
            while let Some(s) = next(begin, &used) {
                used[s as usize] = true;
                let [a, b] = segments[s as usize];
                begin = if a == begin { b } else { a };
                edges.insert(0, begin);
            }
        }
        if edges.len() < 3 {
            continue;
        }
        let points: Vec<Vec2> = edges.iter().map(|e| place(*e)).collect();
        if !closed {
            // A chain a small hole in the mesh left open is still a loop:
            // closed across the gap when that is a small part of it.
            let length: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
            let gap = points[0].distance(points[points.len() - 1]);
            closed = gap <= GAP * length;
        }
        loops.push(Loop::new(points, edges, closed));
    }
    // What is round what: a loop is inside another when its first corner
    // is.
    let count = loops.len();
    for k in 0..count {
        let p = loops[k].points[0];
        let mut depth = 0;
        let mut parent: Option<usize> = None;
        for j in 0..count {
            if j == k || !loops[j].closed || loops[j].area <= loops[k].area {
                continue;
            }
            if loops[j].holds(p) {
                depth += 1;
                if parent.is_none_or(|q| loops[j].area < loops[q].area) {
                    parent = Some(j);
                }
            }
        }
        loops[k].depth = depth;
        loops[k].parent = parent;
    }
    loops
}
