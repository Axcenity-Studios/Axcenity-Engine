//! A triangle surface that answers "which point of me is closest to
//! this one?" quickly: the body (or a lower layer) a garment is bound to
//! and pushed out of.
//!
//! The triangles are put in a uniform grid of cells (each triangle in
//! every cell its box touches); a query looks at the cells round the point
//! ring by ring and stops once no farther ring can hold anything closer.

use glam::Vec3;

/// The closest point of a [`Surface`] to a query point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// The triangle it is on.
    pub triangle: u32,
    /// Where on the triangle, as weights of its three corners (adding up
    /// to 1, none below 0).
    pub bary: Vec3,
    pub point: Vec3,
    /// The surface's smooth normal there (the corners' normals mixed by
    /// `bary`), one unit long.
    pub normal: Vec3,
    /// How far the query point is from `point`.
    pub distance: f32,
}

impl Hit {
    /// How far `query` is outside the surface along its normal here:
    /// negative inside.
    pub fn signed(&self, query: Vec3) -> f32 {
        (query - self.point).dot(self.normal)
    }

    /// Whether the line from `point` to `query` is close to the normal
    /// (within about 40 degrees), so `query` is over this part of the
    /// surface rather than beside one of its edges.
    pub fn is_over(&self, query: Vec3) -> bool {
        let away = query - self.point;
        let along = away.dot(self.normal).abs();
        let across = (away - self.normal * away.dot(self.normal)).length();
        across <= 0.85 * along + 0.0005
    }
}

/// Triangles with corner normals, and a grid to find the closest point.
#[derive(Clone, Debug)]
pub struct Surface {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
    grid: Grid,
}

#[derive(Clone, Debug, Default)]
struct Grid {
    min: Vec3,
    cell: f32,
    counts: [i32; 3],
    /// Each cell's triangles: `starts[c]..starts[c + 1]` in `items`.
    starts: Vec<u32>,
    items: Vec<u32>,
}

impl Surface {
    /// A surface of `triangles` over `positions`, whose smooth normals are
    /// `normals` (one per position; made from the triangles when empty).
    pub fn new(positions: Vec<Vec3>, normals: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> Self {
        let normals = if normals.len() == positions.len() {
            normals
        } else {
            vertex_normals(&positions, &triangles)
        };
        let grid = Grid::new(&positions, &triangles);
        Surface {
            positions,
            normals,
            triangles,
            grid,
        }
    }

    /// The closest point to `query` no farther than `within`, if any.
    pub fn closest(&self, query: Vec3, within: f32) -> Option<Hit> {
        let grid = &self.grid;
        if grid.starts.is_empty() {
            return None;
        }
        let home = grid.cell_of(query);
        let mut best: Option<(f32, u32, Vec3)> = None;
        // Rings beyond the grid can hold nothing, so the search ends there.
        let far = grid.counts.iter().copied().max().unwrap_or(0) + 1;
        let reach = ((within / grid.cell).ceil() as i32 + 1).min(far.max(1));
        for ring in 0..=reach {
            if let Some((distance_sq, _, _)) = best {
                // Everything in ring `ring` is at least `(ring - 1)` cells
                // away.
                let nearest = (ring - 1).max(0) as f32 * grid.cell;
                if nearest * nearest > distance_sq {
                    break;
                }
            }
            grid.each_in_ring(home, ring, |triangle| {
                let [a, b, c] = self.corners(triangle);
                let (point, bary) = closest_on_triangle(query, a, b, c);
                let distance_sq = point.distance_squared(query);
                if best.is_none_or(|(best, _, _)| distance_sq < best) {
                    best = Some((distance_sq, triangle, bary));
                }
            });
        }
        let (distance_sq, triangle, bary) = best?;
        let distance = distance_sq.sqrt();
        if distance > within {
            return None;
        }
        Some(self.hit(triangle, bary, distance))
    }

    /// The first point the ray from `origin` along `direction` (one unit
    /// long) meets within `reach`, either side of a triangle. Walks the
    /// grid's cells along the ray in half-cell steps, with each step's
    /// neighbours, so no cell the ray crosses is missed.
    pub fn cast(&self, origin: Vec3, direction: Vec3, reach: f32) -> Option<Hit> {
        let grid = &self.grid;
        if grid.starts.is_empty() {
            return None;
        }
        let mut seen = std::collections::HashSet::new();
        let mut best: Option<(f32, u32, Vec3)> = None;
        let step = grid.cell * 0.5;
        let mut t = 0.0;
        while t <= reach + step {
            let [cx, cy, cz] = grid.cell_of(origin + direction * t);
            for z in (cz - 1).max(0)..=(cz + 1).min(grid.counts[2] - 1) {
                for y in (cy - 1).max(0)..=(cy + 1).min(grid.counts[1] - 1) {
                    for x in (cx - 1).max(0)..=(cx + 1).min(grid.counts[0] - 1) {
                        let cell = grid.index([x, y, z]);
                        let (start, end) =
                            (grid.starts[cell] as usize, grid.starts[cell + 1] as usize);
                        for &triangle in &grid.items[start..end] {
                            if !seen.insert(triangle) {
                                continue;
                            }
                            let [a, b, c] = self.corners(triangle);
                            if let Some((along, bary)) = ray_triangle(origin, direction, a, b, c)
                                && along <= reach
                                && best.is_none_or(|(best, _, _)| along < best)
                            {
                                best = Some((along, triangle, bary));
                            }
                        }
                    }
                }
            }
            // Nothing past the first hit can be nearer.
            if best.is_some_and(|(along, _, _)| along < t - step) {
                break;
            }
            t += step;
        }
        let (along, triangle, bary) = best?;
        Some(self.hit(triangle, bary, along))
    }

    /// The hit at `bary` on `triangle` (its point and smooth normal).
    pub fn hit(&self, triangle: u32, bary: Vec3, distance: f32) -> Hit {
        let [i, j, k] = self.triangles[triangle as usize].map(|v| v as usize);
        let point =
            self.positions[i] * bary.x + self.positions[j] * bary.y + self.positions[k] * bary.z;
        let mixed = self.normals[i] * bary.x + self.normals[j] * bary.y + self.normals[k] * bary.z;
        let normal = mixed.try_normalize().unwrap_or_else(|| {
            let [a, b, c] = self.corners(triangle);
            (b - a).cross(c - a).normalize_or_zero()
        });
        Hit {
            triangle,
            bary,
            point,
            normal,
            distance,
        }
    }

    fn corners(&self, triangle: u32) -> [Vec3; 3] {
        self.triangles[triangle as usize].map(|v| self.positions[v as usize])
    }
}

/// Area-weighted smooth normals of `positions` over `triangles`.
pub fn vertex_normals(positions: &[Vec3], triangles: &[[u32; 3]]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for &[a, b, c] in triangles {
        let (pa, pb, pc) = (
            positions[a as usize],
            positions[b as usize],
            positions[c as usize],
        );
        // The cross product's length is twice the area: bigger triangles
        // count for more.
        let face = (pb - pa).cross(pc - pa);
        for v in [a, b, c] {
            normals[v as usize] += face;
        }
    }
    normals.iter().map(|n| n.normalize_or_zero()).collect()
}

impl Grid {
    fn new(positions: &[Vec3], triangles: &[[u32; 3]]) -> Self {
        if triangles.is_empty() {
            return Grid::default();
        }
        let (mut min, mut max) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        let mut edges = 0.0;
        for &[a, b, c] in triangles {
            let (pa, pb, pc) = (
                positions[a as usize],
                positions[b as usize],
                positions[c as usize],
            );
            min = min.min(pa.min(pb).min(pc));
            max = max.max(pa.max(pb).max(pc));
            edges += pa.distance(pb) + pb.distance(pc) + pc.distance(pa);
        }
        // About two triangles across a cell, and never more than about two
        // million cells.
        let mean_edge = edges / (3.0 * triangles.len() as f32);
        let size = (max - min).max(Vec3::splat(1e-4));
        let mut cell = (2.0 * mean_edge).max(1e-4);
        while (size / cell).ceil().element_product() > 2.0e6 {
            cell *= 1.5;
        }
        let counts = (size / cell).ceil().as_ivec3().max(glam::IVec3::ONE);
        let grid = Grid {
            min,
            cell,
            counts: counts.to_array(),
            starts: Vec::new(),
            items: Vec::new(),
        };
        let total = counts.x as usize * counts.y as usize * counts.z as usize;
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); total];
        for (index, &[a, b, c]) in triangles.iter().enumerate() {
            let (pa, pb, pc) = (
                positions[a as usize],
                positions[b as usize],
                positions[c as usize],
            );
            let lo = grid.cell_of(pa.min(pb).min(pc));
            let hi = grid.cell_of(pa.max(pb).max(pc));
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    for x in lo[0]..=hi[0] {
                        lists[grid.index([x, y, z])].push(index as u32);
                    }
                }
            }
        }
        let mut starts = Vec::with_capacity(total + 1);
        let mut items = Vec::new();
        for list in lists {
            starts.push(items.len() as u32);
            items.extend(list);
        }
        starts.push(items.len() as u32);
        Grid {
            starts,
            items,
            ..grid
        }
    }

    /// The cell holding `point`, clamped into the grid.
    fn cell_of(&self, point: Vec3) -> [i32; 3] {
        let at = ((point - self.min) / self.cell).floor();
        [0, 1, 2].map(|axis| (at[axis] as i32).clamp(0, self.counts[axis] - 1))
    }

    fn index(&self, [x, y, z]: [i32; 3]) -> usize {
        (z as usize * self.counts[1] as usize + y as usize) * self.counts[0] as usize + x as usize
    }

    /// Calls `each` with every triangle of the cells exactly `ring` cells
    /// (in the largest axis) from `home`. A triangle in several of them
    /// may come more than once.
    fn each_in_ring(&self, home: [i32; 3], ring: i32, mut each: impl FnMut(u32)) {
        let lo = home.map(|v| v - ring);
        let hi = home.map(|v| v + ring);
        for z in lo[2].max(0)..=hi[2].min(self.counts[2] - 1) {
            for y in lo[1].max(0)..=hi[1].min(self.counts[1] - 1) {
                for x in lo[0].max(0)..=hi[0].min(self.counts[0] - 1) {
                    let on_ring = (x - home[0]).abs() == ring
                        || (y - home[1]).abs() == ring
                        || (z - home[2]).abs() == ring;
                    if !on_ring {
                        continue;
                    }
                    let cell = self.index([x, y, z]);
                    let (start, end) = (self.starts[cell] as usize, self.starts[cell + 1] as usize);
                    for &triangle in &self.items[start..end] {
                        each(triangle);
                    }
                }
            }
        }
    }
}

/// The closest point to `p` on triangle `a b c`, and its weights of the
/// three corners (Ericson, *Real-Time Collision Detection*, 5.1.5).
pub fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, Vec3) {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, Vec3::X);
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, Vec3::Y);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, Vec3::new(1.0 - v, v, 0.0));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, Vec3::Z);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, Vec3::new(1.0 - w, 0.0, w));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * w, Vec3::new(0.0, 1.0 - w, w));
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (a + ab * v + ac * w, Vec3::new(1.0 - v - w, v, w))
}

/// Where the ray from `origin` along `direction` meets triangle `a b c`
/// (either side): how far along, and the point's weights of the corners
/// (Möller and Trumbore).
fn ray_triangle(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, Vec3)> {
    let ab = b - a;
    let ac = c - a;
    let p = direction.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(ab);
    let v = direction.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = ac.dot(q) * inv;
    (t >= 0.0).then_some((t, Vec3::new(1.0 - u - v, u, v)))
}
