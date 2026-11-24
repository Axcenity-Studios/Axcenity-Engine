//! A feature of the body on what's worn over it (ADR-0045): a garment
//! carries a feature's weights as the skin under it would, so a clothed
//! waist is lit, and picked, on the cloth.
//!
//! A [`WornMap`] holds, for points of a worn model at rest, the body's
//! corners near each and how much it takes from them. The points are the
//! model's corners and, on a triangle a body region's border may cross, a
//! lattice of points across it ([`Fine`]): a generated garment's
//! triangles run to 20 cm, and a feature's edge read from their corners
//! alone follows the triangles (a zig-zag, or streaks down a coat).
//! Sampled every centimetre or so, the edge runs where the skin's does,
//! whatever the garment's mesh is like. A point's weights depend only on
//! where it is, so the two sides of a garment's seam get the same.
//!
//! Which triangles: those whose corners lie over different body regions
//! (the features of the body are its regions, all or nothing: an edge
//! can only be where two meet), and, more coarsely, any triangle over
//! [`LARGE`], which another region could reach into. The rest, most of a
//! garment, keep their corners alone: a face feature's weights fall off
//! smoothly, and corners carry that well enough.

use axcenity_assets::Model;
use axcenity_render::glam::{IVec3, Vec3};

/// How far from the skin (metres of the file) a point of a garment still
/// takes the weights of the skin nearest it.
const REACH: f32 = 0.12;
/// A point takes its weights from the body's corners up to this much
/// farther (metres) than the nearest, the nearer the more: a body
/// region's weights are all or nothing, and blended they fall across its
/// edge over a centimetre or so...
const SPREAD: f32 = 0.012;
/// ...from at most this many of them.
const SHARED: usize = 8;
/// The side of the cells the body's corners are sorted into to find the
/// ones nearest a point (metres)...
const CELL: f32 = 0.03;
/// ...larger for a body more than this many cells across, so the cells
/// stay few.
const MOST_CELLS: f32 = 160.0;
/// A triangle is sampled at points no farther apart than this along its
/// longest side (metres)...
const STEP: f32 = 0.012;
/// ...in at most this many steps a side.
const MOST_STEPS: usize = 12;
/// A triangle with a side longer than this (metres) is sampled wherever
/// its corners lie, at points no farther apart than [`COARSE_STEP`] when
/// they all lie over one region: enough to find another that reaches
/// into it.
const LARGE: f32 = 0.04;
const COARSE_STEP: f32 = 0.03;
/// The region of a body corner that has none.
const NO_REGION: u8 = u8::MAX;

/// How a triangle of a worn model is sampled: `steps` steps along each
/// side (1: its corners only), its lattice's points from `start` on in
/// its primitive's points. Point (i, j), `i` steps toward the second
/// corner and `j` toward the third (`i + j` at most `steps`), is row `j`'s
/// `i`th; a row has one point fewer than the one before.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fine {
    start: u32,
    steps: u8,
}

/// A body's corners at rest (in its file's space), sorted into a grid of
/// cells to find the ones near a point.
struct Corners {
    /// The grid's first corner, its cells' side and how many there are
    /// each way.
    low: Vec3,
    cell: f32,
    size: IVec3,
    /// Where each cell's corners start in `corners` (one more than there
    /// are cells: the last one's end).
    starts: Vec<u32>,
    /// Each corner, as counted through all the body's primitives, and
    /// where it is; cell by cell.
    corners: Vec<(u32, Vec3)>,
}

impl Corners {
    fn new(points: &[Vec3]) -> Self {
        let low = points.iter().copied().fold(Vec3::INFINITY, Vec3::min);
        let high = points.iter().copied().fold(Vec3::NEG_INFINITY, Vec3::max);
        let (low, high) = if points.is_empty() {
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            (low, high)
        };
        let cell = CELL.max((high - low).max_element() / MOST_CELLS);
        let size = ((high - low) / cell).floor().as_ivec3() + IVec3::ONE;
        let mut grid = Self {
            low,
            cell,
            size,
            starts: vec![0; (size.x * size.y * size.z) as usize + 1],
            corners: vec![(0, Vec3::ZERO); points.len()],
        };
        // How many in each cell, then where each cell starts, then the
        // corners into their cells.
        let cells: Vec<usize> = points
            .iter()
            .map(|point| grid.index(grid.cell_of(*point)).unwrap_or(0))
            .collect();
        for cell in &cells {
            grid.starts[cell + 1] += 1;
        }
        for cell in 1..grid.starts.len() {
            grid.starts[cell] += grid.starts[cell - 1];
        }
        let mut next = grid.starts.clone();
        for (corner, (point, cell)) in points.iter().zip(&cells).enumerate() {
            grid.corners[next[*cell] as usize] = (corner as u32, *point);
            next[*cell] += 1;
        }
        grid
    }

    /// The cell `point` is in (which may be outside the grid).
    fn cell_of(&self, point: Vec3) -> IVec3 {
        ((point - self.low) / self.cell).floor().as_ivec3()
    }

    /// Cell `at`'s place in `starts`, if it's in the grid.
    fn index(&self, at: IVec3) -> Option<usize> {
        (at.cmpge(IVec3::ZERO).all() && at.cmplt(self.size).all())
            .then(|| ((at.z * self.size.y + at.y) * self.size.x + at.x) as usize)
    }

    /// The corners in cell `at`.
    fn in_cell(&self, at: IVec3) -> &[(u32, Vec3)] {
        match self.index(at) {
            Some(index) => {
                &self.corners[self.starts[index] as usize..self.starts[index + 1] as usize]
            }
            None => &[],
        }
    }

    /// The corners that count for `point`, into `near` (each with how far
    /// it is, the nearest first): those within [`SPREAD`] of the nearest's
    /// distance, at most [`SHARED`]; none when the nearest is farther
    /// than [`REACH`]. How far the nearest is comes back.
    fn near(&self, point: Vec3, near: &mut Vec<(f32, u32)>) -> f32 {
        near.clear();
        let home = self.cell_of(point);
        // Outward a shell of cells at a time until one holds a corner:
        // nothing is nearer than that one by more than a shell.
        let shells = ((REACH + SPREAD) / self.cell).ceil() as i32 + 1;
        let mut least = f32::INFINITY;
        let mut found = None;
        for shell in 0..=shells {
            for dz in -shell..=shell {
                for dy in -shell..=shell {
                    let inside = dz.abs() < shell && dy.abs() < shell;
                    // Inside the shell's box only its two ends are new.
                    let step = if inside { (2 * shell).max(1) } else { 1 };
                    for dx in (-shell..=shell).step_by(step as usize) {
                        for (_, at) in self.in_cell(home + IVec3::new(dx, dy, dz)) {
                            least = least.min(at.distance(point));
                        }
                    }
                }
            }
            if least.is_finite() {
                found = Some(shell);
                break;
            }
        }
        let Some(shell) = found else {
            return least;
        };
        // A corner not looked at yet is at least the shell's inner side
        // away: when that and the one found are both out of reach, so is
        // every corner.
        if least > REACH && shell as f32 * self.cell > REACH {
            return least;
        }
        // Then every cell that could hold one that counts.
        let reach = ((least.min(REACH) + SPREAD) / self.cell).ceil() as i32;
        for dz in -reach..=reach {
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    for (corner, at) in self.in_cell(home + IVec3::new(dx, dy, dz)) {
                        let d = at.distance(point);
                        if d <= least + SPREAD {
                            near.push((d, *corner));
                            least = least.min(d);
                        }
                    }
                }
            }
        }
        if least > REACH {
            near.clear();
            return least;
        }
        near.retain(|(d, _)| *d <= least + SPREAD);
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        near.truncate(SHARED);
        least
    }
}

/// A worn model's points on the body.
pub(super) struct WornMap {
    /// For each of the worn model's primitives, each point's part of
    /// `shares` (where it starts, how many): its corners first, then its
    /// triangles' lattices.
    spans: Vec<Vec<(u32, u8)>>,
    /// For each primitive, how each triangle is sampled.
    fine: Vec<Vec<Fine>>,
    /// A body corner (counted through all its primitives) and the share
    /// taken from it; a point's shares add up to one.
    shares: Vec<(u32, f32)>,
    /// Where each of the body's primitives starts in that count, and how
    /// many corners the body has.
    starts: Vec<u32>,
    corners: u32,
}

impl WornMap {
    /// `worn`'s points on `body`, both at rest, compared in their files'
    /// space (each model's fitted box is its own). `None` unless both
    /// are skinned.
    pub(super) fn new(body: &Model, worn: &Model) -> Option<Self> {
        let body_file = body.rig.as_ref()?.fit.inverse();
        let worn_file = worn.rig.as_ref()?.fit.inverse();
        let mut starts = Vec::with_capacity(body.primitives.len());
        // Each body corner's place at rest and its region, counted
        // through all its primitives.
        let (mut places, mut regions) = (Vec::new(), Vec::new());
        for primitive in &body.primitives {
            starts.push(places.len() as u32);
            for (index, position) in primitive.positions.iter().enumerate() {
                places.push(body_file.transform_point3(Vec3::from(*position)));
                regions.push(primitive.regions.get(index).copied().unwrap_or(NO_REGION));
            }
        }
        let count = places.len() as u32;
        let corners = Corners::new(&places);
        let mut shares: Vec<(u32, f32)> = Vec::new();
        let mut near: Vec<(f32, u32)> = Vec::new();
        // The body's corners near `point`, into `shares`.
        let mut on_body = |point: Vec3, shares: &mut Vec<(u32, f32)>| -> (u32, u8) {
            let least = corners.near(point, &mut near);
            // The nearest counts fully, one at the spread's end hardly.
            let counts = |d: f32| (1.0 - (d - least) / SPREAD).clamp(0.0, 1.0) + 0.02;
            let total: f32 = near.iter().map(|(d, _)| counts(*d)).sum();
            let start = shares.len() as u32;
            shares.extend(near.iter().map(|(d, corner)| (*corner, counts(*d) / total)));
            (start, near.len() as u8)
        };
        let mut spans = Vec::with_capacity(worn.primitives.len());
        let mut fine = Vec::with_capacity(worn.primitives.len());
        for primitive in &worn.primitives {
            let at_rest: Vec<Vec3> = primitive
                .positions
                .iter()
                .map(|position| worn_file.transform_point3(Vec3::from(*position)))
                .collect();
            let mut points: Vec<(u32, u8)> = at_rest
                .iter()
                .map(|point| on_body(*point, &mut shares))
                .collect();
            // The body region a corner lies over (its nearest body
            // corner's); `None` over no skin at all.
            let over: Vec<Option<u8>> = points
                .iter()
                .map(|(start, count)| {
                    (*count > 0).then(|| regions[shares[*start as usize].0 as usize])
                })
                .collect();
            let mut triangles = Vec::with_capacity(primitive.indices.len() / 3);
            for triangle in primitive.indices.as_chunks::<3>().0 {
                let corners = Fine { start: 0, steps: 1 };
                let [Some(a), Some(b), Some(c)] =
                    triangle.map(|corner| at_rest.get(corner as usize).copied())
                else {
                    triangles.push(corners);
                    continue;
                };
                let longest = a.distance(b).max(b.distance(c)).max(c.distance(a));
                let [ra, rb, rc] = triangle.map(|corner| over[corner as usize]);
                // All over one region, or none over any skin: no region's
                // border crosses a small one.
                let one_region = ra == rb && rb == rc;
                let step = if one_region { COARSE_STEP } else { STEP };
                let steps = ((longest / step).ceil() as usize).clamp(1, MOST_STEPS);
                if steps == 1 || (one_region && longest <= LARGE) {
                    triangles.push(corners);
                    continue;
                }
                triangles.push(Fine {
                    start: points.len() as u32,
                    steps: steps as u8,
                });
                for j in 0..=steps {
                    for i in 0..=steps - j {
                        let point = a
                            + (b - a) * (i as f32 / steps as f32)
                            + (c - a) * (j as f32 / steps as f32);
                        points.push(on_body(point, &mut shares));
                    }
                }
            }
            spans.push(points);
            fine.push(triangles);
        }
        Some(Self {
            spans,
            fine,
            shares,
            starts,
            corners: count,
        })
    }

    /// A feature's weights on the body (each of its primitives' corners')
    /// on the worn model's points, primitive by primitive.
    pub(super) fn weights(&self, on_body: &[Vec<f32>]) -> Vec<Vec<f32>> {
        // The body's corners in one run, as `shares` counts them.
        let mut flat = vec![0.0; self.corners as usize];
        for (start, weights) in self.starts.iter().zip(on_body) {
            let start = *start as usize;
            if let Some(run) = flat.get_mut(start..) {
                let count = run.len().min(weights.len());
                run[..count].copy_from_slice(&weights[..count]);
            }
        }
        self.spans
            .iter()
            .map(|points| {
                points
                    .iter()
                    .map(|(start, count)| {
                        let start = *start as usize;
                        self.shares[start..start + usize::from(*count)]
                            .iter()
                            .map(|(corner, share)| flat[*corner as usize] * share)
                            .sum()
                    })
                    .collect()
            })
            .collect()
    }

    /// How each triangle of the worn model's primitive `primitive` is
    /// sampled.
    pub(super) fn fine(&self, primitive: usize) -> &[Fine] {
        self.fine.get(primitive).map_or(&[], Vec::as_slice)
    }
}

/// The weight at a point of a triangle: `weights` are its primitive's
/// points' (the corners', then the lattices'), `corners` the triangle's
/// and `shares` how much of each makes the point. With `fine` it's read
/// from the triangle's lattice (the three points round it); without, from
/// its corners.
pub(super) fn blend(
    weights: &[f32],
    corners: [u32; 3],
    [sa, sb, sc]: [f32; 3],
    fine: Option<Fine>,
) -> f32 {
    let of = |index: usize| weights.get(index).copied().unwrap_or(0.0);
    let Some(Fine { start, steps }) = fine.filter(|fine| fine.steps > 1) else {
        let [a, b, c] = corners.map(|corner| of(corner as usize));
        return sa * a + sb * b + sc * c;
    };
    let steps = usize::from(steps);
    let across = (sb * steps as f32).clamp(0.0, steps as f32);
    let up = (sc * steps as f32).clamp(0.0, steps as f32);
    // The lattice cell the point is in: its corner nearest the triangle's
    // first corner, and how far into it the point is.
    let j = (up.floor() as usize).min(steps - 1);
    let i = (across.floor() as usize).min(steps - 1 - j);
    let (fu, fv) = (across - i as f32, up - j as f32);
    let at =
        |i: usize, j: usize| of(start as usize + j * (steps + 1) - j * j.saturating_sub(1) / 2 + i);
    if fu + fv <= 1.0 {
        at(i, j) * (1.0 - fu - fv) + at(i + 1, j) * fu + at(i, j + 1) * fv
    } else if i + j + 2 <= steps {
        at(i + 1, j + 1) * (fu + fv - 1.0) + at(i, j + 1) * (1.0 - fu) + at(i + 1, j) * (1.0 - fv)
    } else {
        // Past the triangle's far side by a rounding: on it.
        let over = fu + fv;
        at(i + 1, j) * (fu / over) + at(i, j + 1) * (fv / over)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lattice of `steps` whose point (i, j) weighs `of(i, j)`, after
    /// three corners' weights.
    fn lattice(steps: usize, of: impl Fn(usize, usize) -> f32) -> (Vec<f32>, Fine) {
        let mut weights = vec![of(0, 0), of(steps, 0), of(0, steps)];
        for j in 0..=steps {
            for i in 0..=steps - j {
                weights.push(of(i, j));
            }
        }
        (
            weights,
            Fine {
                start: 3,
                steps: steps as u8,
            },
        )
    }

    #[test]
    fn a_lattice_gives_back_what_is_linear_across_the_triangle() {
        // Linear across the triangle: the lattice reads the same as the
        // corners do, anywhere in it and on its sides.
        for steps in [2, 3, 7, 12] {
            let (weights, fine) = lattice(steps, |i, j| {
                0.2 + 0.5 * i as f32 / steps as f32 + 0.3 * j as f32 / steps as f32
            });
            for shares in [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.5, 0.5, 0.0],
                [0.0, 0.5, 0.5],
                [0.2, 0.3, 0.5],
                [0.01, 0.495, 0.495],
                [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
            ] {
                let by_corners = blend(&weights, [0, 1, 2], shares, None);
                let by_lattice = blend(&weights, [0, 1, 2], shares, Some(fine));
                assert!(
                    (by_corners - by_lattice).abs() < 1e-4,
                    "{steps} steps at {shares:?}: {by_lattice} for {by_corners}"
                );
            }
        }
    }

    #[test]
    fn a_lattice_keeps_an_edge_its_corners_would_smear() {
        // A feature that ends a quarter of the way up a long triangle:
        // weight 1 below, 0 above. From the corners (1, 1 and 0) the
        // weight would fall evenly all the way up and cross a half at the
        // middle; the lattice keeps the edge at the quarter.
        let steps = 12;
        let (weights, fine) = lattice(steps, |_, j| if j <= 3 { 1.0 } else { 0.0 });
        let up = |sc: f32| {
            blend(
                &weights,
                [0, 1, 2],
                [(1.0 - sc) / 2.0, (1.0 - sc) / 2.0, sc],
                Some(fine),
            )
        };
        assert!(up(0.2) > 0.99, "inside below the edge");
        assert!(up(0.4) < 0.01, "outside above it");
        let by_corners = blend(&weights, [0, 1, 2], [0.3, 0.3, 0.4], None);
        assert!(by_corners > 0.5, "the corners alone would light it there");
    }
}
