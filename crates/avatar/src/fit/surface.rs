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
        // (A point outside the grid has the cell it is clamped to as its
        // home; every cell is still within the grid's size of that.)
        let far = grid.counts.iter().copied().max().unwrap_or(0).max(1) + 1;
        // `within` may be huge or infinite: clamp it to the grid before
        // making it a ring count (a float past `i32::MAX` saturates, and
        // adding to it overflows).
        let rings = (within / grid.cell).ceil();
        let reach = if rings.is_finite() && rings < far as f32 {
            rings as i32 + 1
        } else {
            far
        };
        // Nothing farther than `within` is wanted: a cell farther than that,
        // or than the best so far, is passed over.
        let limit_sq = if within.is_finite() {
            within * within
        } else {
            f32::INFINITY
        };
        for ring in 0..=reach {
            let bound = best.map_or(limit_sq, |(distance_sq, _, _)| distance_sq.min(limit_sq));
            // Everything in ring `ring` is at least `(ring - 1)` cells away.
            let nearest = (ring - 1).max(0) as f32 * grid.cell;
            if nearest * nearest > bound {
                break;
            }
            grid.each_cell_in_ring(home, ring, |cell| {
                let bound = best.map_or(limit_sq, |(distance_sq, _, _)| distance_sq.min(limit_sq));
                if grid.distance_sq_to_cell(cell, query) > bound {
                    return;
                }
                for &triangle in grid.triangles_in(cell) {
                    let [a, b, c] = self.corners(triangle);
                    let (point, bary) = closest_on_triangle(query, a, b, c);
                    let distance_sq = point.distance_squared(query);
                    if best.is_none_or(|(best, _, _)| distance_sq < best) {
                        best = Some((distance_sq, triangle, bary));
                    }
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
        self.cast_with(origin, direction, reach, |_| true, 0.0)
    }

    /// As [`cast`](Self::cast), meeting only the triangles `keep` takes
    /// (by index): those facing a viewer, say, when back faces aren't
    /// drawn. A ray along an edge between two triangles meets one of
    /// them (an edge is widened by a millionth of the triangle, so a ray
    /// can't slip between two triangles by rounding). The walk stops
    /// where the ray leaves the grid.
    pub fn cast_where(
        &self,
        origin: Vec3,
        direction: Vec3,
        reach: f32,
        keep: impl Fn(u32) -> bool,
    ) -> Option<Hit> {
        self.cast_with(origin, direction, reach, keep, 1e-6)
    }

    fn cast_with(
        &self,
        origin: Vec3,
        direction: Vec3,
        reach: f32,
        keep: impl Fn(u32) -> bool,
        widen: f32,
    ) -> Option<Hit> {
        let grid = &self.grid;
        if grid.starts.is_empty() {
            return None;
        }
        let (enter, exit) = grid.span(origin, direction)?;
        if enter > reach {
            return None;
        }
        let reach = reach.min(exit);
        // The triangles already tried, in a set kept per thread (casting is
        // done millions of times when hiding under clothing is worked out:
        // `enclosure`), hashed cheaply. The set is borrowed for the whole
        // walk, so `keep` must not cast on this thread itself (a nested
        // cast would panic on the second borrow); no caller does.
        thread_local! {
            static SEEN: std::cell::RefCell<std::collections::HashSet<u32, Cheap>> =
                std::cell::RefCell::new(std::collections::HashSet::default());
        }
        SEEN.with(|seen| {
            let mut seen = seen.borrow_mut();
            seen.clear();
            self.cast_walk(origin, direction, (enter, reach), &keep, widen, &mut seen)
        })
    }

    fn cast_walk(
        &self,
        origin: Vec3,
        direction: Vec3,
        (enter, reach): (f32, f32),
        keep: &impl Fn(u32) -> bool,
        widen: f32,
        seen: &mut std::collections::HashSet<u32, Cheap>,
    ) -> Option<Hit> {
        let grid = &self.grid;
        let mut best: Option<(f32, u32, Vec3)> = None;
        let step = grid.cell * 0.5;
        // From where the ray enters the grid's box (nothing is nearer).
        let mut t = (enter - step).max(0.0);
        while t <= reach + step {
            let [cx, cy, cz] = grid.cell_of(origin + direction * t);
            for z in (cz - 1).max(0)..=(cz + 1).min(grid.counts[2] - 1) {
                for y in (cy - 1).max(0)..=(cy + 1).min(grid.counts[1] - 1) {
                    for x in (cx - 1).max(0)..=(cx + 1).min(grid.counts[0] - 1) {
                        let cell = grid.index([x, y, z]);
                        let (start, end) =
                            (grid.starts[cell] as usize, grid.starts[cell + 1] as usize);
                        for &triangle in &grid.items[start..end] {
                            if !seen.insert(triangle) || !keep(triangle) {
                                continue;
                            }
                            let [a, b, c] = self.corners(triangle);
                            if let Some((along, bary)) =
                                ray_triangle(origin, direction, a, b, c, widen)
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

/// A hasher for triangle numbers: a multiply, which spreads them well
/// enough for a set and is much cheaper than the standard one.
#[derive(Clone, Copy, Default)]
struct Cheap(u64);

impl std::hash::Hasher for Cheap {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn write_u32(&mut self, n: u32) {
        self.0 = u64::from(n).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
}

impl std::hash::BuildHasher for Cheap {
    type Hasher = Cheap;

    fn build_hasher(&self) -> Cheap {
        Cheap(0)
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
    /// Where along the ray from `origin` along `direction` it is inside
    /// the grid's box (a cell wider each way): from and to, or `None` when
    /// it never is. Nothing of the surface is outside that stretch.
    fn span(&self, origin: Vec3, direction: Vec3) -> Option<(f32, f32)> {
        let size = Vec3::new(
            self.counts[0] as f32,
            self.counts[1] as f32,
            self.counts[2] as f32,
        ) * self.cell;
        let (lo, hi) = (self.min - self.cell, self.min + size + self.cell);
        let (mut near, mut far) = (0.0f32, f32::INFINITY);
        for axis in 0..3 {
            let d = direction[axis];
            if d.abs() < 1e-12 {
                if origin[axis] < lo[axis] || origin[axis] > hi[axis] {
                    return None;
                }
                continue;
            }
            let (a, b) = ((lo[axis] - origin[axis]) / d, (hi[axis] - origin[axis]) / d);
            near = near.max(a.min(b));
            far = far.min(a.max(b));
        }
        (near <= far).then_some((near, far))
    }

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

    /// Calls `each` with every cell exactly `ring` cells (in the largest
    /// axis) from `home`.
    fn each_cell_in_ring(&self, home: [i32; 3], ring: i32, mut each: impl FnMut([i32; 3])) {
        let lo = home.map(|v| v - ring);
        let hi = home.map(|v| v + ring);
        for z in lo[2].max(0)..=hi[2].min(self.counts[2] - 1) {
            for y in lo[1].max(0)..=hi[1].min(self.counts[1] - 1) {
                for x in lo[0].max(0)..=hi[0].min(self.counts[0] - 1) {
                    let on_ring = (x - home[0]).abs() == ring
                        || (y - home[1]).abs() == ring
                        || (z - home[2]).abs() == ring;
                    if on_ring {
                        each([x, y, z]);
                    }
                }
            }
        }
    }

    /// The triangles in a cell (a triangle is in every cell its box
    /// touches).
    fn triangles_in(&self, cell: [i32; 3]) -> &[u32] {
        let index = self.index(cell);
        &self.items[self.starts[index] as usize..self.starts[index + 1] as usize]
    }

    /// How far `point` is from a cell's box, squared (0 inside it).
    fn distance_sq_to_cell(&self, cell: [i32; 3], point: Vec3) -> f32 {
        let lo = self.min + Vec3::new(cell[0] as f32, cell[1] as f32, cell[2] as f32) * self.cell;
        let hi = lo + Vec3::splat(self.cell);
        let outside = (lo - point).max(point - hi).max(Vec3::ZERO);
        outside.length_squared()
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
fn ray_triangle(
    origin: Vec3,
    direction: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    widen: f32,
) -> Option<(f32, Vec3)> {
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
    if !(-widen..=1.0 + widen).contains(&u) {
        return None;
    }
    let q = s.cross(ab);
    let v = direction.dot(q) * inv;
    if v < -widen || u + v > 1.0 + widen {
        return None;
    }
    let t = ac.dot(q) * inv;
    // Barycentric weights clamped back onto the triangle.
    let (u, v) = (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let (u, v) = if u + v > 1.0 {
        (u / (u + v), v / (u + v))
    } else {
        (u, v)
    };
    (t >= 0.0).then_some((t, Vec3::new(1.0 - u - v, u, v)))
}
